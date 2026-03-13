//! Timeout policy middleware for unresolved unload dialogs.
//!
//! This module is an optional resilience layer that complements lifecycle cleanup.
//! It converts prolonged user-code silence into deterministic command responses,
//! which helps avoid indefinitely pending unload flows in long-running sessions.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use crate::{
    command::BrowserCommand,
    data::dialog::DialogType,
    data::ids::BrowsingContextId,
    delegate::{BackendDelegate, CommandDecision, DelegateContext, EventDecision},
    event::{BrowserEvent, BrowsingContextEvent},
};

use super::DelegateLayer;

/// Timeout-based responder for `beforeunload` dialogs.
///
/// This layer watches `beforeunload` dialog requests and auto-enqueues
/// `ConfirmBeforeUnload` when the configured timeout expires.
///
/// Use this as a safeguard when application logic may delay or miss
/// manual dialog responses.
#[derive(Debug, Clone)]
pub struct AutoDialogResponderLayer {
    timeout: Option<Duration>,
    proceed: bool,
}

impl AutoDialogResponderLayer {
    /// Creates a layer with no timeout behavior.
    ///
    /// Defaults:
    /// - `timeout`: disabled (`None`)
    /// - `proceed_on_timeout`: `false` (cancel dialog)
    pub fn new() -> Self {
        Self {
            timeout: None,
            proceed: false,
        }
    }

    /// Enables timeout handling for pending `beforeunload` dialogs.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets whether auto-response should proceed (`true`) or cancel (`false`).
    pub fn proceed_on_timeout(mut self, proceed: bool) -> Self {
        self.proceed = proceed;
        self
    }
}

impl Default for AutoDialogResponderLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl DelegateLayer for AutoDialogResponderLayer {
    fn wrap(self: Box<Self>, inner: Box<dyn BackendDelegate>) -> Box<dyn BackendDelegate> {
        Box::new(AutoDialogResponder {
            inner,
            timeout: self.timeout,
            proceed: self.proceed,
            pending: HashMap::new(),
        })
    }
}

struct AutoDialogResponder {
    inner: Box<dyn BackendDelegate>,
    timeout: Option<Duration>,
    proceed: bool,
    pending: HashMap<(BrowsingContextId, u64), Instant>,
}

impl AutoDialogResponder {
    fn clear_browsing_context(&mut self, browsing_context_id: BrowsingContextId) {
        self.pending.retain(|(id, _), _| *id != browsing_context_id);
    }

    fn request_next_deadline(&self, ctx: &mut DelegateContext) {
        if let Some(deadline) = self.pending.values().copied().min() {
            ctx.request_wake_at(deadline);
        }
    }
}

impl BackendDelegate for AutoDialogResponder {
    fn on_command(
        &mut self,
        ctx: &mut DelegateContext,
        command: &BrowserCommand,
    ) -> CommandDecision {
        if let BrowserCommand::ConfirmBeforeUnload {
            browsing_context_id,
            request_id,
            ..
        } = command
        {
            self.pending.remove(&(*browsing_context_id, *request_id));
        }

        self.inner.on_command(ctx, command)
    }

    fn on_event(&mut self, ctx: &mut DelegateContext, event: &BrowserEvent) -> EventDecision {
        if let BrowserEvent::BrowsingContext {
            browsing_context_id,
            event,
            ..
        } = event
        {
            if let BrowsingContextEvent::JavaScriptDialogRequested {
                request_id, r#type, ..
            } = event.as_ref()
                && *r#type == DialogType::BeforeUnload
                && let Some(timeout) = self.timeout
            {
                let deadline = Instant::now() + timeout;
                self.pending
                    .insert((*browsing_context_id, *request_id), deadline);
                ctx.request_wake_at(deadline);
            }
            if matches!(event.as_ref(), BrowsingContextEvent::Closed) {
                self.clear_browsing_context(*browsing_context_id);
            }
        }

        self.inner.on_event(ctx, event)
    }

    fn on_wake(&mut self, ctx: &mut DelegateContext) {
        if self.timeout.is_some() {
            let now = Instant::now();
            let expired: Vec<(BrowsingContextId, u64)> = self
                .pending
                .iter()
                .filter_map(|(key, deadline)| (*deadline <= now).then_some(*key))
                .collect();

            for (browsing_context_id, request_id) in expired {
                self.pending.remove(&(browsing_context_id, request_id));
                ctx.enqueue_command(BrowserCommand::ConfirmBeforeUnload {
                    browsing_context_id,
                    request_id,
                    proceed: self.proceed,
                });
            }
        }

        self.request_next_deadline(ctx);
        self.inner.on_wake(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delegate::NoopDelegate;

    fn beforeunload_requested(browsing_context_id: u64, request_id: u64) -> BrowserEvent {
        BrowserEvent::BrowsingContext {
            profile_id: "default".to_string(),
            browsing_context_id: BrowsingContextId::new(browsing_context_id),
            event: Box::new(BrowsingContextEvent::JavaScriptDialogRequested {
                request_id,
                r#type: DialogType::BeforeUnload,
                message: "leave?".to_string(),
                default_prompt_text: None,
                beforeunload_reason: None,
            }),
        }
    }

    #[test]
    fn timeout_event_requests_wake_deadline() {
        let layer = AutoDialogResponderLayer::new()
            .timeout(Duration::from_secs(1))
            .proceed_on_timeout(true);
        let mut delegate = Box::new(layer).wrap(Box::new(NoopDelegate));
        let mut ctx = DelegateContext::default();

        let decision = delegate.on_event(&mut ctx, &beforeunload_requested(1, 7));

        assert!(matches!(decision, EventDecision::Forward));
        assert!(ctx.requested_wake_deadline().is_some());
        assert!(ctx.pop_command().is_none());
    }

    #[test]
    fn wake_before_timeout_re_registers_without_enqueuing_response() {
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut delegate: Box<dyn BackendDelegate> = Box::new(AutoDialogResponder {
            inner: Box::new(NoopDelegate),
            timeout: Some(Duration::from_secs(1)),
            proceed: true,
            pending: HashMap::from([((BrowsingContextId::new(1), 7), deadline)]),
        });
        let mut ctx = DelegateContext::default();

        delegate.on_wake(&mut ctx);

        assert!(ctx.pop_command().is_none());
        assert_eq!(ctx.requested_wake_deadline(), Some(deadline));
    }

    #[test]
    fn wake_after_timeout_enqueues_confirm_beforeunload() {
        let mut delegate: Box<dyn BackendDelegate> = Box::new(AutoDialogResponder {
            inner: Box::new(NoopDelegate),
            timeout: Some(Duration::from_secs(1)),
            proceed: false,
            pending: HashMap::from([(
                (BrowsingContextId::new(3), 11),
                Instant::now() - Duration::from_millis(1),
            )]),
        });
        let mut ctx = DelegateContext::default();

        delegate.on_wake(&mut ctx);

        assert!(matches!(
            ctx.pop_command(),
            Some(BrowserCommand::ConfirmBeforeUnload {
                browsing_context_id,
                request_id: 11,
                proceed: false,
            }) if browsing_context_id == BrowsingContextId::new(3)
        ));
        assert!(ctx.requested_wake_deadline().is_none());
    }
}
