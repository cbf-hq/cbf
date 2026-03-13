//! Debounce middleware for high-frequency page resize commands.
//!
//! This layer debounces [`BrowserCommand::ResizeBrowsingContext`] per page and
//! forwards only the latest size when the debounce window elapses.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use crate::{
    command::BrowserCommand,
    data::ids::BrowsingContextId,
    delegate::{BackendDelegate, CommandDecision, DelegateContext, EventDecision},
    event::{BackendStopReason, BrowserEvent, BrowsingContextEvent},
};

use super::DelegateLayer;

/// Debounces `ResizeBrowsingContext` commands per browsing context.
///
/// Default debounce window is 16ms.
#[derive(Debug, Clone)]
pub struct ResizeDebounceLayer {
    window: Duration,
}

impl ResizeDebounceLayer {
    /// Creates a resize debounce layer with the default 16ms window.
    pub fn new() -> Self {
        Self {
            window: Duration::from_millis(16),
        }
    }

    /// Sets the debounce window.
    pub fn window(mut self, window: Duration) -> Self {
        self.window = window;
        self
    }
}

impl Default for ResizeDebounceLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl DelegateLayer for ResizeDebounceLayer {
    fn wrap(self: Box<Self>, inner: Box<dyn BackendDelegate>) -> Box<dyn BackendDelegate> {
        Box::new(ResizeDebounce {
            inner,
            window: self.window,
            pending: HashMap::new(),
        })
    }
}

struct ResizeDebounce {
    inner: Box<dyn BackendDelegate>,
    window: Duration,
    pending: HashMap<BrowsingContextId, PendingResize>,
}

#[derive(Debug, Clone, Copy)]
struct PendingResize {
    width: u32,
    height: u32,
    deadline: Instant,
}

impl ResizeDebounce {
    fn request_next_deadline(&self, ctx: &mut DelegateContext) {
        if let Some(deadline) = self.pending.values().map(|pending| pending.deadline).min() {
            ctx.request_wake_at(deadline);
        }
    }

    fn flush_expired(&mut self, ctx: &mut DelegateContext) {
        let now = Instant::now();
        let expired: Vec<(BrowsingContextId, PendingResize)> = self
            .pending
            .iter()
            .filter_map(|(id, pending)| (pending.deadline <= now).then_some((*id, *pending)))
            .collect();

        for (browsing_context_id, pending) in expired {
            self.pending.remove(&browsing_context_id);
            ctx.enqueue_command(BrowserCommand::ResizeBrowsingContext {
                browsing_context_id,
                width: pending.width,
                height: pending.height,
            });
        }

        self.request_next_deadline(ctx);
    }
}

impl BackendDelegate for ResizeDebounce {
    fn on_command(
        &mut self,
        ctx: &mut DelegateContext,
        command: &BrowserCommand,
    ) -> CommandDecision {
        match command {
            BrowserCommand::ResizeBrowsingContext {
                browsing_context_id,
                width,
                height,
            } => {
                let deadline = Instant::now() + self.window;
                self.pending.insert(
                    *browsing_context_id,
                    PendingResize {
                        width: *width,
                        height: *height,
                        deadline,
                    },
                );
                ctx.request_wake_at(deadline);
                CommandDecision::Drop
            }
            _ => {
                self.flush_expired(ctx);
                self.inner.on_command(ctx, command)
            }
        }
    }

    fn on_event(&mut self, ctx: &mut DelegateContext, event: &BrowserEvent) -> EventDecision {
        if let BrowserEvent::BrowsingContext {
            browsing_context_id,
            event,
            ..
        } = event
            && matches!(event.as_ref(), BrowsingContextEvent::Closed)
        {
            self.pending.remove(browsing_context_id);
        }

        self.inner.on_event(ctx, event)
    }

    fn on_wake(&mut self, ctx: &mut DelegateContext) {
        self.flush_expired(ctx);
        self.inner.on_wake(ctx);
    }

    fn on_teardown(&mut self, ctx: &mut DelegateContext, reason: BackendStopReason) {
        self.pending.clear();
        self.inner.on_teardown(ctx, reason);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delegate::NoopDelegate;

    fn resize(id: u64, width: u32, height: u32) -> BrowserCommand {
        BrowserCommand::ResizeBrowsingContext {
            browsing_context_id: BrowsingContextId::new(id),
            width,
            height,
        }
    }

    #[test]
    fn resize_command_requests_deadline_and_emits_on_wake() {
        let layer = ResizeDebounceLayer::new().window(Duration::ZERO);
        let mut delegate = Box::new(layer).wrap(Box::new(NoopDelegate));
        let mut ctx = DelegateContext::default();

        let decision = delegate.on_command(&mut ctx, &resize(1, 800, 600));
        assert!(matches!(decision, CommandDecision::Drop));
        assert!(ctx.requested_wake_deadline().is_some());
        assert!(ctx.pop_command().is_none());

        delegate.on_wake(&mut ctx);
        let queued = ctx.pop_command();
        assert!(matches!(
            queued,
            Some(BrowserCommand::ResizeBrowsingContext {
                browsing_context_id,
                width: 800,
                height: 600
            }) if browsing_context_id == BrowsingContextId::new(1)
        ));
        assert!(ctx.pop_command().is_none());
    }

    #[test]
    fn latest_resize_wins_for_same_context() {
        let layer = ResizeDebounceLayer::new().window(Duration::ZERO);
        let mut delegate = Box::new(layer).wrap(Box::new(NoopDelegate));
        let mut ctx = DelegateContext::default();

        let first = delegate.on_command(&mut ctx, &resize(7, 800, 600));
        let second = delegate.on_command(&mut ctx, &resize(7, 1024, 768));
        assert!(matches!(first, CommandDecision::Drop));
        assert!(matches!(second, CommandDecision::Drop));
        assert!(ctx.requested_wake_deadline().is_some());

        delegate.on_wake(&mut ctx);
        let queued = ctx.pop_command();
        assert!(matches!(
            queued,
            Some(BrowserCommand::ResizeBrowsingContext {
                browsing_context_id,
                width: 1024,
                height: 768
            }) if browsing_context_id == BrowsingContextId::new(7)
        ));
        assert!(ctx.pop_command().is_none());
    }

    #[test]
    fn wake_before_deadline_re_registers_without_emitting_resize() {
        let pending = PendingResize {
            width: 800,
            height: 600,
            deadline: Instant::now() + Duration::from_secs(60),
        };
        let mut delegate: Box<dyn BackendDelegate> = Box::new(ResizeDebounce {
            inner: Box::new(NoopDelegate),
            window: Duration::from_millis(16),
            pending: HashMap::from([(BrowsingContextId::new(9), pending)]),
        });
        let mut ctx = DelegateContext::default();

        delegate.on_wake(&mut ctx);

        assert!(ctx.pop_command().is_none());
        assert_eq!(ctx.requested_wake_deadline(), Some(pending.deadline));
    }

    #[test]
    fn pending_resize_is_dropped_when_page_closes() {
        let layer = ResizeDebounceLayer::new().window(Duration::ZERO);
        let mut delegate = Box::new(layer).wrap(Box::new(NoopDelegate));
        let mut ctx = DelegateContext::default();

        _ = delegate.on_command(&mut ctx, &resize(9, 640, 480));
        _ = delegate.on_event(
            &mut ctx,
            &BrowserEvent::BrowsingContext {
                profile_id: "default".to_string(),
                browsing_context_id: BrowsingContextId::new(9),
                event: Box::new(BrowsingContextEvent::Closed),
            },
        );

        delegate.on_wake(&mut ctx);
        assert!(ctx.pop_command().is_none());
    }
}
