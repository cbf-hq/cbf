use std::{collections::VecDeque, time::Instant};

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
    requested_wake_deadline: Option<Instant>,
}

impl DelegateContext {
    /// Queues a command to be processed after the current callback returns.
    pub fn enqueue_command(&mut self, command: BrowserCommand) {
        self.queued_commands.push_back(command);
    }

    /// Requests that the runtime wake the delegate loop no later than `deadline`.
    pub fn request_wake_at(&mut self, deadline: Instant) {
        self.requested_wake_deadline =
            choose_earlier_deadline(self.requested_wake_deadline, Some(deadline));
    }

    pub(crate) fn pop_command(&mut self) -> Option<BrowserCommand> {
        self.queued_commands.pop_front()
    }

    pub(crate) fn has_queued_commands(&self) -> bool {
        !self.queued_commands.is_empty()
    }

    pub(crate) fn requested_wake_deadline(&self) -> Option<Instant> {
        self.requested_wake_deadline
    }

    pub(crate) fn clear_requested_wake_deadline(&mut self) {
        self.requested_wake_deadline = None;
    }
}

pub(crate) fn choose_earlier_deadline(
    current: Option<Instant>,
    requested: Option<Instant>,
) -> Option<Instant> {
    match (current, requested) {
        (Some(current), Some(requested)) => Some(current.min(requested)),
        (Some(current), None) => Some(current),
        (None, Some(requested)) => Some(requested),
        (None, None) => None,
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

    /// Called when the backend loop wakes and the delegate gets a chance to do work.
    fn on_wake(&mut self, ctx: &mut DelegateContext) {
        self.on_idle(ctx);
    }

    /// Compatibility hook for legacy poll-driven middleware.
    ///
    /// New code should prefer [`BackendDelegate::on_wake`].
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

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::choose_earlier_deadline;

    #[test]
    fn choose_earlier_deadline_prefers_the_shortest_requested_deadline() {
        let now = Instant::now();
        let later = now + Duration::from_millis(30);
        let earlier = now + Duration::from_millis(10);

        assert_eq!(
            choose_earlier_deadline(Some(later), Some(earlier)),
            Some(earlier)
        );
        assert_eq!(
            choose_earlier_deadline(Some(earlier), Some(later)),
            Some(earlier)
        );
    }

    #[test]
    fn choose_earlier_deadline_keeps_existing_when_no_new_deadline_is_requested() {
        let now = Instant::now();
        let deadline = now + Duration::from_millis(10);

        assert_eq!(
            choose_earlier_deadline(Some(deadline), None),
            Some(deadline)
        );
        assert_eq!(
            choose_earlier_deadline(None, Some(deadline)),
            Some(deadline)
        );
        assert_eq!(choose_earlier_deadline(None, None), None);
    }
}
