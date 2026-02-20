//! Tracing integration middleware for delegate pipelines.
//!
//! This module provides operational visibility for middleware stacks and is
//! typically composed with lifecycle and safety-oriented layers. Use it to
//! diagnose command/event flow and policy outcomes without changing business logic.

use tracing::{Level, debug, error, info, trace, warn};

use crate::{
    backend_delegate::{BackendDelegate, CommandDecision, DelegateContext, EventDecision},
    command::BrowserCommand,
    event::{BackendStopReason, BrowserEvent},
};

use super::DelegateLayer;

/// Observability layer that emits tracing logs for delegate activity.
///
/// This layer logs incoming commands/events, corresponding decisions, and
/// teardown reasons through the `tracing` crate.
#[derive(Debug, Clone, Copy)]
pub struct LoggingLayer {
    command_level: Level,
    event_level: Level,
    teardown_level: Level,
    target: &'static str,
}

impl LoggingLayer {
    /// Creates a logging layer with default log levels.
    ///
    /// Note: `tracing` requires callsite targets to be compile-time constants,
    /// so `target` is recorded as a structured field (`log_target`) rather than
    /// used as the tracing callsite target itself.
    ///
    /// Defaults:
    /// - `command_level`: `DEBUG`
    /// - `event_level`: `DEBUG`
    /// - `teardown_level`: `INFO`
    pub fn new(target: &'static str) -> Self {
        Self {
            command_level: Level::DEBUG,
            event_level: Level::DEBUG,
            teardown_level: Level::INFO,
            target,
        }
    }

    /// Sets the log level used for command reception and command decisions.
    pub fn command_level(mut self, level: Level) -> Self {
        self.command_level = level;
        self
    }

    /// Sets the log level used for event reception and event decisions.
    pub fn event_level(mut self, level: Level) -> Self {
        self.event_level = level;
        self
    }

    /// Sets the log level used for delegate teardown logging.
    pub fn teardown_level(mut self, level: Level) -> Self {
        self.teardown_level = level;
        self
    }
}

impl DelegateLayer for LoggingLayer {
    fn wrap(self: Box<Self>, inner: Box<dyn BackendDelegate>) -> Box<dyn BackendDelegate> {
        Box::new(Logging {
            inner,
            command_level: self.command_level,
            event_level: self.event_level,
            teardown_level: self.teardown_level,
            target: self.target,
        })
    }
}

struct Logging {
    inner: Box<dyn BackendDelegate>,
    command_level: Level,
    event_level: Level,
    teardown_level: Level,
    target: &'static str,
}

macro_rules! log_at_level {
    ($level:expr, $log_target:expr, $data:ident, $message:literal) => {
        match $level {
            Level::TRACE => {
                trace!(log_target = $log_target, ?$data, $message);
            }
            Level::DEBUG => {
                debug!(log_target = $log_target, ?$data, $message);
            }
            Level::INFO => {
                info!(log_target = $log_target, ?$data, $message);
            }
            Level::WARN => {
                warn!(log_target = $log_target, ?$data, $message);
            }
            Level::ERROR => {
                error!(log_target = $log_target, ?$data, $message);
            }
        }
    };
}

impl Logging {
    fn log_command_received(&self, command: &BrowserCommand) {
        log_at_level!(self.command_level, self.target, command, "received command");
    }

    fn log_command_decision(&self, decision: &CommandDecision) {
        log_at_level!(
            self.command_level,
            self.target,
            decision,
            "command decision"
        );
    }

    fn log_event_received(&self, event: &BrowserEvent) {
        log_at_level!(self.event_level, self.target, event, "received event");
    }

    fn log_event_decision(&self, decision: &EventDecision) {
        log_at_level!(self.event_level, self.target, decision, "event decision");
    }

    fn log_teardown(&self, reason: &BackendStopReason) {
        log_at_level!(
            self.teardown_level,
            self.target,
            reason,
            "delegate teardown"
        );
    }
}

impl BackendDelegate for Logging {
    fn on_command(
        &mut self,
        ctx: &mut DelegateContext,
        command: &BrowserCommand,
    ) -> CommandDecision {
        self.log_command_received(command);
        let decision = self.inner.on_command(ctx, command);
        self.log_command_decision(&decision);
        decision
    }

    fn on_event(&mut self, ctx: &mut DelegateContext, event: &BrowserEvent) -> EventDecision {
        self.log_event_received(event);
        let decision = self.inner.on_event(ctx, event);
        self.log_event_decision(&decision);
        decision
    }

    fn on_idle(&mut self, ctx: &mut DelegateContext) {
        self.inner.on_idle(ctx);
    }

    fn on_teardown(&mut self, ctx: &mut DelegateContext, reason: BackendStopReason) {
        self.log_teardown(&reason);
        self.inner.on_teardown(ctx, reason);
    }
}
