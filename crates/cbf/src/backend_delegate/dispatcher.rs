use async_channel::Sender;

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
/// - queued follow-up work emitted by delegate hooks
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

    /// Dispatches one command through the delegate pipeline.
    ///
    /// Returns a stop reason when either the delegate requests stop or the
    /// forward path reports terminal failure.
    pub fn dispatch_command<F>(
        &mut self,
        command: BrowserCommand,
        event_tx: &Sender<BrowserEvent>,
        forward: &mut F,
    ) -> Option<BackendStopReason>
    where
        F: FnMut(BrowserCommand) -> (Option<BackendStopReason>, Vec<BrowserEvent>),
    {
        match self.delegate.on_command(&mut self.ctx, command) {
            CommandDecision::Forward(command) => {
                let (reason, events) = forward(command);
                for event in events {
                    self.ctx.emit_event(event);
                }
                if let Some(reason) = reason {
                    return Some(reason);
                }
            }
            CommandDecision::Drop => {}
            CommandDecision::Stop(reason) => return Some(reason),
        }

        self.flush(event_tx, forward)
    }

    /// Dispatches one event through the delegate pipeline.
    pub fn dispatch_event(
        &mut self,
        event: BrowserEvent,
        event_tx: &Sender<BrowserEvent>,
    ) -> Option<BackendStopReason> {
        self.emit_event(event, event_tx)
    }

    /// Flushes queued commands/events produced via [`DelegateContext`].
    pub fn flush<F>(
        &mut self,
        event_tx: &Sender<BrowserEvent>,
        forward: &mut F,
    ) -> Option<BackendStopReason>
    where
        F: FnMut(BrowserCommand) -> (Option<BackendStopReason>, Vec<BrowserEvent>),
    {
        loop {
            while let Some(command) = self.ctx.pop_command() {
                if let Some(reason) = self.dispatch_command(command, event_tx, forward) {
                    return Some(reason);
                }
            }

            let mut emitted_event = false;
            while let Some(event) = self.ctx.pop_event() {
                emitted_event = true;
                if let Some(reason) = self.emit_event(event, event_tx) {
                    return Some(reason);
                }
            }

            if !emitted_event && !self.ctx.has_queued_commands() {
                break;
            }
        }

        None
    }

    /// Stops the dispatcher and emits a final `BackendStopped` event.
    pub fn stop<F>(
        &mut self,
        event_tx: &Sender<BrowserEvent>,
        reason: BackendStopReason,
        forward: &mut F,
    ) where
        F: FnMut(BrowserCommand) -> (Option<BackendStopReason>, Vec<BrowserEvent>),
    {
        self.delegate.on_teardown(&mut self.ctx, reason.clone());
        let reason = self.flush(event_tx, forward).unwrap_or(reason);
        _ = event_tx.send_blocking(BrowserEvent::BackendStopped { reason });
    }

    fn emit_event(
        &mut self,
        event: BrowserEvent,
        event_tx: &Sender<BrowserEvent>,
    ) -> Option<BackendStopReason> {
        match self.delegate.on_event(&mut self.ctx, event) {
            EventDecision::Forward(event) => {
                _ = event_tx.send_blocking(event);
                None
            }
            EventDecision::Drop => None,
            EventDecision::Stop(reason) => Some(reason),
        }
    }
}
