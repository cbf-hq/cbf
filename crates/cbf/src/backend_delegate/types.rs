use std::collections::VecDeque;

use crate::{
    command::BrowserCommand,
    event::{BackendStopReason, BrowserEvent},
};

/// Decision returned from [`BackendDelegate::on_command`].
#[derive(Debug)]
pub enum CommandDecision {
    /// Forward the (possibly transformed) command to the backend transport.
    Forward(BrowserCommand),
    /// Drop the command and continue processing.
    Drop,
    /// Stop backend processing with the given reason.
    Stop(BackendStopReason),
}

/// Decision returned from [`BackendDelegate::on_event`].
#[derive(Debug)]
pub enum EventDecision {
    /// Forward the (possibly transformed) event to the consumer.
    Forward(BrowserEvent),
    /// Drop the event and continue processing.
    Drop,
    /// Stop backend processing with the given reason.
    Stop(BackendStopReason),
}

/// Per-dispatch mutable context shared with a delegate.
///
/// Delegates can enqueue follow-up commands and emit additional events.
#[derive(Debug, Default)]
pub struct DelegateContext {
    queued_commands: VecDeque<BrowserCommand>,
    queued_events: VecDeque<BrowserEvent>,
}

impl DelegateContext {
    /// Queues a command to be processed after the current callback returns.
    pub fn enqueue_command(&mut self, command: BrowserCommand) {
        self.queued_commands.push_back(command);
    }

    /// Emits an event to be dispatched after the current callback returns.
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

/// Hook-based interface for mediating backend commands and events.
///
/// Implement this trait to inject policy, logging, filtering, or fail-safe
/// behavior into browser command/event flow.
pub trait BackendDelegate: Send + 'static {
    /// Called for each command before backend transport handling.
    fn on_command(
        &mut self,
        _ctx: &mut DelegateContext,
        command: BrowserCommand,
    ) -> CommandDecision {
        CommandDecision::Forward(command)
    }

    /// Called for each backend event before it is delivered to consumers.
    fn on_event(&mut self, _ctx: &mut DelegateContext, event: BrowserEvent) -> EventDecision {
        EventDecision::Forward(event)
    }

    /// Called periodically while the backend loop is idle.
    fn on_idle(&mut self, _ctx: &mut DelegateContext) {}

    /// Called when backend teardown is initiated.
    fn on_teardown(&mut self, _ctx: &mut DelegateContext, _reason: BackendStopReason) {}
}

/// Delegate that forwards everything unchanged.
///
/// Useful as a baseline or placeholder in tests and composition.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopDelegate;

impl BackendDelegate for NoopDelegate {}
