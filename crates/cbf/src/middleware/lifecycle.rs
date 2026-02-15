use std::collections::HashSet;

use crate::{
    backend_delegate::{BackendDelegate, CommandDecision, DelegateContext, EventDecision},
    command::BrowserCommand,
    data::ids::WebPageId,
    event::{BackendStopReason, BrowserEvent, DialogType, WebPageEvent},
};

use super::DelegateLayer;

#[derive(Debug, Default, Clone, Copy)]
pub struct LifecycleLayer;

impl LifecycleLayer {
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
    pending_beforeunload: HashSet<(WebPageId, u64)>,
}

impl Lifecycle {
    fn resolve_all_pending(&mut self, ctx: &mut DelegateContext) {
        for (web_page_id, request_id) in self.pending_beforeunload.drain() {
            ctx.enqueue_command(BrowserCommand::ConfirmBeforeUnload {
                web_page_id,
                request_id,
                proceed: false,
            });
        }
    }

    fn resolve_pending_for_page(&mut self, ctx: &mut DelegateContext, web_page_id: WebPageId) {
        let request_ids: Vec<u64> = self
            .pending_beforeunload
            .iter()
            .filter_map(|(id, request_id)| (*id == web_page_id).then_some(*request_id))
            .collect();

        for request_id in request_ids {
            self.pending_beforeunload.remove(&(web_page_id, request_id));
            ctx.enqueue_command(BrowserCommand::ConfirmBeforeUnload {
                web_page_id,
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
        command: BrowserCommand,
    ) -> CommandDecision {
        if let BrowserCommand::ConfirmBeforeUnload {
            web_page_id,
            request_id,
            ..
        } = &command
        {
            self.pending_beforeunload
                .remove(&(*web_page_id, *request_id));
        }

        self.inner.on_command(ctx, command)
    }

    fn on_event(&mut self, ctx: &mut DelegateContext, event: BrowserEvent) -> EventDecision {
        if let BrowserEvent::WebPage {
            web_page_id,
            event:
                WebPageEvent::JavaScriptDialogRequested {
                    request_id, r#type, ..
                },
            ..
        } = &event
            && *r#type == DialogType::BeforeUnload
        {
            self.pending_beforeunload
                .insert((*web_page_id, *request_id));
        }

        if let BrowserEvent::WebPage {
            web_page_id,
            event: WebPageEvent::Closed,
            ..
        } = &event
        {
            self.resolve_pending_for_page(ctx, *web_page_id);
        }

        self.inner.on_event(ctx, event)
    }

    fn on_teardown(&mut self, ctx: &mut DelegateContext, reason: BackendStopReason) {
        self.resolve_all_pending(ctx);
        self.inner.on_teardown(ctx, reason);
    }
}
