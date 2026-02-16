//! Fail-safe middleware for backend error bursts.
//!
//! Use this module when backend errors should remain observable first
//! (`BrowserEvent::BackendError`) and then be converted into stop decisions
//! by explicit policy.
//!
//! See [`ErrorGuardLayer`] for exact policy and configuration knobs.

use crate::{
    ApiErrorKind,
    backend_delegate::{BackendDelegate, CommandDecision, DelegateContext, EventDecision},
    command::BrowserCommand,
    event::{BackendStopReason, BrowserEvent},
};

use super::DelegateLayer;

/// Middleware layer that turns repeated backend error events into a stop decision.
///
/// This layer watches [`BrowserEvent::BackendError`] and returns
/// [`EventDecision::Stop`] when the configured policy deems the error stream unsafe.
///
/// Default policy:
/// - Immediate stop for `ConnectTimeout` and `ProtocolMismatch`
/// - Stop after 3 consecutive non-immediate backend errors
/// - Reset the consecutive error counter when a non-error event is observed
/// - Honor `terminal_hint` from the backend
#[derive(Debug, Clone)]
pub struct ErrorGuardLayer {
    consecutive_error_threshold: u32,
    immediate_kinds: Vec<ApiErrorKind>,
    honor_terminal_hint: bool,
}

impl ErrorGuardLayer {
    /// Creates an `ErrorGuardLayer` with a production-safe default policy.
    pub fn new() -> Self {
        Self {
            consecutive_error_threshold: 3,
            immediate_kinds: vec![ApiErrorKind::ConnectTimeout, ApiErrorKind::ProtocolMismatch],
            honor_terminal_hint: true,
        }
    }

    /// Sets the number of consecutive backend error events required to stop.
    ///
    /// Values smaller than `1` are clamped to `1`.
    pub fn consecutive_error_threshold(mut self, threshold: u32) -> Self {
        self.consecutive_error_threshold = threshold.max(1);
        self
    }

    /// Replaces the list of error kinds that should trigger immediate stop.
    ///
    /// Matching is exact against `BackendErrorInfo.kind`.
    pub fn immediate_kinds(mut self, kinds: Vec<ApiErrorKind>) -> Self {
        self.immediate_kinds = kinds;
        self
    }

    /// Controls whether `terminal_hint` should be treated as an immediate-stop signal.
    pub fn honor_terminal_hint(mut self, honor: bool) -> Self {
        self.honor_terminal_hint = honor;
        self
    }
}

impl Default for ErrorGuardLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl DelegateLayer for ErrorGuardLayer {
    fn wrap(self: Box<Self>, inner: Box<dyn BackendDelegate>) -> Box<dyn BackendDelegate> {
        Box::new(ErrorGuard {
            inner,
            consecutive_error_threshold: self.consecutive_error_threshold,
            immediate_kinds: self.immediate_kinds,
            honor_terminal_hint: self.honor_terminal_hint,
            consecutive_errors: 0,
        })
    }
}

struct ErrorGuard {
    inner: Box<dyn BackendDelegate>,
    consecutive_error_threshold: u32,
    immediate_kinds: Vec<ApiErrorKind>,
    honor_terminal_hint: bool,
    consecutive_errors: u32,
}

impl ErrorGuard {
    fn should_stop_on_error(&mut self, kind: ApiErrorKind, terminal_hint: bool) -> bool {
        self.consecutive_errors = self.consecutive_errors.saturating_add(1);

        (self.honor_terminal_hint && terminal_hint)
            || self.immediate_kinds.contains(&kind)
            || self.consecutive_errors >= self.consecutive_error_threshold
    }
}

impl BackendDelegate for ErrorGuard {
    fn on_command(
        &mut self,
        ctx: &mut DelegateContext,
        command: BrowserCommand,
    ) -> CommandDecision {
        self.inner.on_command(ctx, command)
    }

    fn on_event(&mut self, ctx: &mut DelegateContext, event: BrowserEvent) -> EventDecision {
        let stop_reason = match &event {
            BrowserEvent::BackendError {
                info,
                terminal_hint,
            } if self.should_stop_on_error(info.kind, *terminal_hint) => {
                Some(BackendStopReason::Error(info.clone()))
            }
            BrowserEvent::BackendError { .. } => None,
            _ => {
                self.consecutive_errors = 0;
                None
            }
        };

        match self.inner.on_event(ctx, event) {
            EventDecision::Stop(reason) => EventDecision::Stop(reason),
            decision => match stop_reason {
                Some(reason) => EventDecision::Stop(reason),
                None => decision,
            },
        }
    }

    fn on_idle(&mut self, ctx: &mut DelegateContext) {
        self.inner.on_idle(ctx)
    }

    fn on_teardown(&mut self, ctx: &mut DelegateContext, reason: BackendStopReason) {
        self.inner.on_teardown(ctx, reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BackendErrorInfo, backend_delegate::NoopDelegate};

    fn make_backend_error(kind: ApiErrorKind, terminal_hint: bool) -> BrowserEvent {
        BrowserEvent::BackendError {
            info: BackendErrorInfo {
                kind,
                operation: None,
                detail: None,
            },
            terminal_hint,
        }
    }

    #[test]
    fn immediate_kind_stops_without_waiting_for_threshold() {
        let mut guard = ErrorGuard {
            inner: Box::new(NoopDelegate),
            consecutive_error_threshold: 3,
            immediate_kinds: vec![ApiErrorKind::ProtocolMismatch],
            honor_terminal_hint: true,
            consecutive_errors: 0,
        };

        let decision = guard.on_event(
            &mut DelegateContext::default(),
            make_backend_error(ApiErrorKind::ProtocolMismatch, false),
        );

        assert!(matches!(decision, EventDecision::Stop(_)));
    }

    #[test]
    fn threshold_stops_on_third_consecutive_error() {
        let mut guard = ErrorGuard {
            inner: Box::new(NoopDelegate),
            consecutive_error_threshold: 3,
            immediate_kinds: vec![],
            honor_terminal_hint: false,
            consecutive_errors: 0,
        };

        let mut ctx = DelegateContext::default();
        let first = guard.on_event(
            &mut ctx,
            make_backend_error(ApiErrorKind::CommandDispatchFailed, false),
        );
        let second = guard.on_event(
            &mut ctx,
            make_backend_error(ApiErrorKind::CommandDispatchFailed, false),
        );
        let third = guard.on_event(
            &mut ctx,
            make_backend_error(ApiErrorKind::CommandDispatchFailed, false),
        );

        assert!(!matches!(first, EventDecision::Stop(_)));
        assert!(!matches!(second, EventDecision::Stop(_)));
        assert!(matches!(third, EventDecision::Stop(_)));
    }

    #[test]
    fn non_error_event_resets_consecutive_counter() {
        let mut guard = ErrorGuard {
            inner: Box::new(NoopDelegate),
            consecutive_error_threshold: 2,
            immediate_kinds: vec![],
            honor_terminal_hint: false,
            consecutive_errors: 0,
        };

        let mut ctx = DelegateContext::default();
        _ = guard.on_event(
            &mut ctx,
            make_backend_error(ApiErrorKind::EventProcessingFailed, false),
        );
        _ = guard.on_event(
            &mut ctx,
            BrowserEvent::BackendReady {
                backend_name: "chromium".to_string(),
            },
        );

        let decision = guard.on_event(
            &mut ctx,
            make_backend_error(ApiErrorKind::EventProcessingFailed, false),
        );

        assert!(!matches!(decision, EventDecision::Stop(_)));
    }
}
