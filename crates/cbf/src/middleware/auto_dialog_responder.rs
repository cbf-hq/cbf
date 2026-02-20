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
    data::ids::BrowsingContextId,
    delegate::{BackendDelegate, CommandDecision, DelegateContext, EventDecision},
    event::{BrowserEvent, BrowsingContextEvent, DialogType},
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
                self.pending.insert(
                    (*browsing_context_id, *request_id),
                    Instant::now() + timeout,
                );
            }
            if matches!(event.as_ref(), BrowsingContextEvent::Closed) {
                self.clear_browsing_context(*browsing_context_id);
            }
        }

        self.inner.on_event(ctx, event)
    }

    fn on_idle(&mut self, ctx: &mut DelegateContext) {
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

        self.inner.on_idle(ctx)
    }
}
