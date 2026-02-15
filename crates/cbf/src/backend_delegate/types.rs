use std::collections::VecDeque;

use crate::{
    command::BrowserCommand,
    event::{BackendStopReason, BrowserEvent},
};

#[derive(Debug)]
pub enum CommandDecision {
    Forward(BrowserCommand),
    Drop,
    Stop(BackendStopReason),
}

#[derive(Debug)]
pub enum EventDecision {
    Forward(BrowserEvent),
    Drop,
    Stop(BackendStopReason),
}

#[derive(Debug, Default)]
pub struct DelegateContext {
    queued_commands: VecDeque<BrowserCommand>,
    queued_events: VecDeque<BrowserEvent>,
}

impl DelegateContext {
    pub fn enqueue_command(&mut self, command: BrowserCommand) {
        self.queued_commands.push_back(command);
    }

    pub fn emit_event(&mut self, event: BrowserEvent) {
        self.queued_events.push_back(event);
    }

    pub(crate) fn pop_command(&mut self) -> Option<BrowserCommand> {
        self.queued_commands.pop_front()
    }

    pub(crate) fn pop_event(&mut self) -> Option<BrowserEvent> {
        self.queued_events.pop_front()
    }

    pub(crate) fn has_queued_commands(&self) -> bool {
        !self.queued_commands.is_empty()
    }
}

pub trait BackendDelegate: Send + 'static {
    fn on_command(
        &mut self,
        _ctx: &mut DelegateContext,
        command: BrowserCommand,
    ) -> CommandDecision {
        CommandDecision::Forward(command)
    }

    fn on_event(&mut self, _ctx: &mut DelegateContext, event: BrowserEvent) -> EventDecision {
        EventDecision::Forward(event)
    }

    fn on_idle(&mut self, _ctx: &mut DelegateContext) {}

    fn on_teardown(&mut self, _ctx: &mut DelegateContext, _reason: BackendStopReason) {}
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NoopDelegate;

impl BackendDelegate for NoopDelegate {}
