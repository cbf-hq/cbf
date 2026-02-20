//! Lifecycle coordination middleware.
//!
//! This module provides the safety baseline for dialog-related lifecycle edges.
//! It is intended to be present in standard stacks and acts as the anchor layer
//! that keeps teardown/page-close flows consistent before optional policies are applied.

use std::collections::HashSet;

use crate::{
    command::BrowserCommand,
    data::ids::BrowsingContextId,
    delegate::{BackendDelegate, CommandDecision, DelegateContext, EventDecision},
    event::{BackendStopReason, BrowserEvent, BrowsingContextEvent, DialogType},
};

use super::DelegateLayer;

/// Lifecycle safety layer for dialog-related cleanup.
///
/// This layer tracks pending `beforeunload` dialogs and ensures they are
/// resolved with `proceed: false` when:
/// - the related page is closed, or
/// - the backend is tearing down.
///
/// Include this layer in normal middleware stacks. `MiddlewareBuilder` requires
/// it by default unless unsafe mode is explicitly enabled.
#[derive(Debug, Default, Clone, Copy)]
pub struct LifecycleLayer;

impl LifecycleLayer {
    /// Creates a lifecycle layer.
    pub fn new() -> Self {
        Self
    }
}

impl DelegateLayer for LifecycleLayer {
    fn wrap(self: Box<Self>, inner: Box<dyn BackendDelegate>) -> Box<dyn BackendDelegate> {
        Box::new(Lifecycle {
            inner,
            pending_beforeunload: HashSet::new(),
        })
    }

    fn is_lifecycle(&self) -> bool {
        true
    }
}

struct Lifecycle {
    inner: Box<dyn BackendDelegate>,
    pending_beforeunload: HashSet<(BrowsingContextId, u64)>,
}

impl Lifecycle {
    fn resolve_all_pending(&mut self, ctx: &mut DelegateContext) {
        for (browsing_context_id, request_id) in self.pending_beforeunload.drain() {
            ctx.enqueue_command(BrowserCommand::ConfirmBeforeUnload {
                browsing_context_id,
                request_id,
                proceed: false,
            });
        }
    }

    fn resolve_pending_for_page(
        &mut self,
        ctx: &mut DelegateContext,
        browsing_context_id: BrowsingContextId,
    ) {
        let request_ids: Vec<u64> = self
            .pending_beforeunload
            .iter()
            .filter_map(|(id, request_id)| (*id == browsing_context_id).then_some(*request_id))
            .collect();

        for request_id in request_ids {
            self.pending_beforeunload
                .remove(&(browsing_context_id, request_id));
            ctx.enqueue_command(BrowserCommand::ConfirmBeforeUnload {
                browsing_context_id,
                request_id,
                proceed: false,
            });
        }
    }
}

impl BackendDelegate for Lifecycle {
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
            self.pending_beforeunload
                .remove(&(*browsing_context_id, *request_id));
        }

        self.inner.on_command(ctx, command)
    }

    fn on_event(&mut self, ctx: &mut DelegateContext, event: &BrowserEvent) -> EventDecision {
        if let BrowserEvent::BrowsingContext {
            browsing_context_id,
            event,
            ..
        } = event
            && let BrowsingContextEvent::JavaScriptDialogRequested {
                request_id, r#type, ..
            } = event.as_ref()
            && *r#type == DialogType::BeforeUnload
        {
            self.pending_beforeunload
                .insert((*browsing_context_id, *request_id));
        }

        if let BrowserEvent::BrowsingContext {
            browsing_context_id,
            event,
            ..
        } = event
            && matches!(event.as_ref(), BrowsingContextEvent::Closed)
        {
            self.resolve_pending_for_page(ctx, *browsing_context_id);
        }

        self.inner.on_event(ctx, event)
    }

    fn on_teardown(&mut self, ctx: &mut DelegateContext, reason: BackendStopReason) {
        self.resolve_all_pending(ctx);
        self.inner.on_teardown(ctx, reason);
    }
}
