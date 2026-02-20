use crate::{
    command::BrowserCommand,
    event::{BackendStopReason, BrowserEvent},
};

use super::{BackendDelegate, CommandDecision, DelegateContext, EventDecision};

/// Drives a [`BackendDelegate`] and applies its decisions to runtime flow.
///
/// The dispatcher owns a delegate and a [`DelegateContext`], then coordinates:
/// - command dispatch and optional forwarding to backend transport
/// - event dispatch to the application-facing stream
/// - queued follow-up commands emitted by delegate hooks
pub struct DelegateDispatcher<D: BackendDelegate> {
    delegate: D,
    ctx: DelegateContext,
}

impl<D: BackendDelegate> DelegateDispatcher<D> {
    /// Creates a dispatcher for a delegate.
    pub fn new(delegate: D) -> Self {
        Self {
            delegate,
            ctx: DelegateContext::default(),
        }
    }

    /// Runs the delegate idle hook once.
    pub fn on_idle(&mut self) {
        self.delegate.on_idle(&mut self.ctx);
    }

    /// Dispatches one command through the delegate pipeline and returns the decision.
    pub fn dispatch_command(&mut self, command: &BrowserCommand) -> CommandDecision {
        self.delegate.on_command(&mut self.ctx, command)
    }

    /// Dispatches one event through the delegate pipeline and returns the decision.
    pub fn dispatch_event(&mut self, event: &BrowserEvent) -> EventDecision {
        self.delegate.on_event(&mut self.ctx, event)
    }

    /// Flushes queued commands produced via [`DelegateContext`].
    pub fn flush(&mut self) -> Vec<BrowserCommand> {
        let mut queued = Vec::new();
        while let Some(command) = self.ctx.pop_command() {
            queued.push(command);
        }

        debug_assert!(
            !self.ctx.has_queued_commands(),
            "DelegateContext command queue should be empty after flush"
        );
        queued
    }

    /// Runs teardown hook and returns stop reason with queued follow-up commands.
    pub fn stop(&mut self, reason: BackendStopReason) -> (BackendStopReason, Vec<BrowserCommand>) {
        self.delegate.on_teardown(&mut self.ctx, reason.clone());
        let queued_commands = self.flush();
        (reason, queued_commands)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        command::BrowserCommand,
        error::{ApiErrorKind, BackendErrorInfo},
        event::{BackendStopReason, BrowserEvent},
    };

    use super::{
        BackendDelegate, CommandDecision, DelegateContext, DelegateDispatcher, EventDecision,
    };

    struct EnqueueListProfilesDelegate;

    impl BackendDelegate for EnqueueListProfilesDelegate {
        fn on_command(
            &mut self,
            ctx: &mut DelegateContext,
            command: &BrowserCommand,
        ) -> CommandDecision {
            if matches!(command, BrowserCommand::ForceShutdown) {
                ctx.enqueue_command(BrowserCommand::ListProfiles);
            }
            CommandDecision::Forward
        }
    }

    struct StopOnReadyDelegate;

    impl BackendDelegate for StopOnReadyDelegate {
        fn on_command(
            &mut self,
            ctx: &mut DelegateContext,
            command: &BrowserCommand,
        ) -> CommandDecision {
            if matches!(command, BrowserCommand::ForceShutdown) {
                ctx.enqueue_command(BrowserCommand::ListProfiles);
            }
            CommandDecision::Forward
        }

        fn on_event(&mut self, _ctx: &mut DelegateContext, event: &BrowserEvent) -> EventDecision {
            if matches!(event, BrowserEvent::BackendReady) {
                return EventDecision::Stop(BackendStopReason::Error(BackendErrorInfo {
                    kind: ApiErrorKind::EventProcessingFailed,
                    operation: None,
                    detail: Some("stop on ready".to_string()),
                }));
            }
            EventDecision::Forward
        }
    }

    #[test]
    fn dispatch_methods_return_decisions_and_flush_returns_queued_commands() {
        let mut dispatcher = DelegateDispatcher::new(EnqueueListProfilesDelegate);

        let decision = dispatcher.dispatch_command(&BrowserCommand::ForceShutdown);
        assert!(matches!(decision, CommandDecision::Forward));

        let queued = dispatcher.flush();
        assert_eq!(queued.len(), 1);
        assert!(matches!(queued[0], BrowserCommand::ListProfiles));
    }

    #[test]
    fn stop_returns_reason_and_queued_commands() {
        let mut dispatcher = DelegateDispatcher::new(StopOnReadyDelegate);

        let decision = dispatcher.dispatch_command(&BrowserCommand::ForceShutdown);
        assert!(matches!(decision, CommandDecision::Forward));

        let (reason, queued) = dispatcher.stop(BackendStopReason::Disconnected);
        assert!(matches!(reason, BackendStopReason::Disconnected));
        assert_eq!(queued.len(), 1);
        assert!(matches!(queued[0], BrowserCommand::ListProfiles));
    }
}
