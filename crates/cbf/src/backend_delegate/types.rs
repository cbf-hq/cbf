use std::collections::VecDeque;

use crate::{
    command::BrowserCommand,
    event::{BackendStopReason, BrowserEvent},
};

/// Decision returned from [`BackendDelegate::on_command`].
#[derive(Debug)]
pub enum CommandDecision {
    /// Forward the command to the backend transport.
    Forward,
    /// Drop the command and continue processing.
    Drop,
    /// Stop backend processing with the given reason.
    Stop(BackendStopReason),
}

/// Decision returned from [`BackendDelegate::on_event`].
#[derive(Debug)]
pub enum EventDecision {
    /// Forward the event to the consumer.
    Forward,
    /// Stop backend processing with the given reason.
    Stop(BackendStopReason),
}

/// Per-dispatch mutable context shared with a delegate.
///
/// Delegates can enqueue follow-up commands.
#[derive(Debug, Default)]
pub struct DelegateContext {
    queued_commands: VecDeque<BrowserCommand>,
}

impl DelegateContext {
    /// Queues a command to be processed after the current callback returns.
    pub fn enqueue_command(&mut self, command: BrowserCommand) {
        self.queued_commands.push_back(command);
    }

    pub(crate) fn pop_command(&mut self) -> Option<BrowserCommand> {
        self.queued_commands.pop_front()
    }

    pub(crate) fn has_queued_commands(&self) -> bool {
        !self.queued_commands.is_empty()
    }
}

/// Hook-based interface for mediating backend commands and events.
///
/// Implement this trait to inject policy, logging, filtering, or fail-safe
/// behavior into browser command/event flow without mutating payloads.
pub trait BackendDelegate: Send + 'static {
    /// Called for each command before backend transport handling.
    fn on_command(
        &mut self,
        _ctx: &mut DelegateContext,
        _command: &BrowserCommand,
    ) -> CommandDecision {
        CommandDecision::Forward
    }

    /// Called for each backend event before it is delivered to consumers.
    fn on_event(&mut self, _ctx: &mut DelegateContext, _event: &BrowserEvent) -> EventDecision {
        EventDecision::Forward
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
