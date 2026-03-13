//! Backend event-loop abstractions for backend implementers.
//!
//! These types describe wake reasons for backend runtime loops without mixing
//! them into browser-domain command or event payloads.

use std::time::Instant;

/// Wake reason returned by a backend event loop wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendWake {
    /// A command source is ready to be drained.
    CommandReady,
    /// Backend-owned input is ready to be drained.
    BackendInputReady,
    /// The requested deadline elapsed.
    DeadlineReached,
    /// The event loop should stop.
    Stopped,
}

/// Wait interface used by backend implementations to multiplex wake sources.
pub trait BackendEventLoop {
    /// Wait until a command source, backend input, stop signal, or deadline fires.
    fn wait_until(&self, deadline: Option<Instant>) -> BackendWake;
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{BackendEventLoop, BackendWake};

    struct StubEventLoop {
        wake: BackendWake,
    }

    impl BackendEventLoop for StubEventLoop {
        fn wait_until(&self, _deadline: Option<Instant>) -> BackendWake {
            self.wake
        }
    }

    #[test]
    fn wait_returns_wake_reason_without_payload() {
        let event_loop = StubEventLoop {
            wake: BackendWake::Stopped,
        };

        assert_eq!(
            event_loop.wait_until(Some(Instant::now() + Duration::from_secs(1))),
            BackendWake::Stopped
        );
    }

    #[test]
    fn wake_variants_are_distinct_control_reasons() {
        let wakes = [
            BackendWake::CommandReady,
            BackendWake::BackendInputReady,
            BackendWake::DeadlineReached,
            BackendWake::Stopped,
        ];

        assert_eq!(wakes.len(), 4);
        assert_ne!(wakes[0], wakes[1]);
        assert_ne!(wakes[1], wakes[2]);
        assert_ne!(wakes[2], wakes[3]);
    }

    #[test]
    fn wait_contract_allows_absent_deadline_without_changing_wake_reason() {
        let event_loop = StubEventLoop {
            wake: BackendWake::Stopped,
        };

        assert_eq!(event_loop.wait_until(None), BackendWake::Stopped);
    }
}
