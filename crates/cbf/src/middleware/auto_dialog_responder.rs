use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use crate::{
    backend_delegate::{BackendDelegate, CommandDecision, DelegateContext, EventDecision},
    command::BrowserCommand,
    data::ids::WebPageId,
    event::{BrowserEvent, DialogType, WebPageEvent},
};

use super::DelegateLayer;

#[derive(Debug, Clone)]
pub struct AutoDialogResponderLayer {
    timeout: Option<Duration>,
    proceed: bool,
}

impl AutoDialogResponderLayer {
    pub fn new() -> Self {
        Self {
            timeout: None,
            proceed: false,
        }
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

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
    pending: HashMap<(WebPageId, u64), Instant>,
}

impl AutoDialogResponder {
    fn clear_web_page(&mut self, web_page_id: WebPageId) {
        self.pending.retain(|(id, _), _| *id != web_page_id);
    }
}

impl BackendDelegate for AutoDialogResponder {
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
            self.pending.remove(&(*web_page_id, *request_id));
        }

        self.inner.on_command(ctx, command)
    }

    fn on_event(&mut self, ctx: &mut DelegateContext, event: BrowserEvent) -> EventDecision {
        match &event {
            BrowserEvent::WebPage {
                web_page_id,
                event:
                    WebPageEvent::JavaScriptDialogRequested {
                        request_id, r#type, ..
                    },
                ..
            } if *r#type == DialogType::BeforeUnload => {
                if let Some(timeout) = self.timeout {
                    self.pending
                        .insert((*web_page_id, *request_id), Instant::now() + timeout);
                }
            }
            BrowserEvent::WebPage {
                web_page_id,
                event: WebPageEvent::Closed,
                ..
            } => self.clear_web_page(*web_page_id),
            _ => {}
        }

        self.inner.on_event(ctx, event)
    }

    fn on_idle(&mut self, ctx: &mut DelegateContext) {
        if self.timeout.is_some() {
            let now = Instant::now();
            let expired: Vec<(WebPageId, u64)> = self
                .pending
                .iter()
                .filter_map(|(key, deadline)| (*deadline <= now).then_some(*key))
                .collect();

            for (web_page_id, request_id) in expired {
                self.pending.remove(&(web_page_id, request_id));
                ctx.enqueue_command(BrowserCommand::ConfirmBeforeUnload {
                    web_page_id,
                    request_id,
                    proceed: self.proceed,
                });
            }
        }

        self.inner.on_idle(ctx)
    }
}
