use crate::command::BrowserOperation;

/// Errors returned by the `cbf` public API.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The backend disconnected or the command channel was closed.
    #[error("cbf backend disconnected")]
    Disconnected,
    /// The command queue is full and cannot accept new commands.
    #[error("cbf command queue full")]
    QueueFull,

    /// Failed to spawn a backend process.
    #[error("cbf process spawn error: {0}")]
    ProcessSpawnError(#[from] std::io::Error),

    /// Invalid high-level API configuration.
    #[error("cbf invalid configuration: {0}")]
    InvalidConfiguration(InvalidConfiguration),

    /// A backend failure surfaced to the public API.
    #[error("cbf backend failure: {0}")]
    BackendFailure(BackendErrorInfo),
}

/// Stable categories for backend failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiErrorKind {
    ConnectTimeout,
    CommandDispatchFailed,
    EventProcessingFailed,
    ProtocolMismatch,
    InvalidInput,
    Unsupported,
}

impl std::fmt::Display for ApiErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self {
            Self::ConnectTimeout => "connect_timeout",
            Self::CommandDispatchFailed => "command_dispatch_failed",
            Self::EventProcessingFailed => "event_processing_failed",
            Self::ProtocolMismatch => "protocol_mismatch",
            Self::InvalidInput => "invalid_input",
            Self::Unsupported => "unsupported",
        };

        f.write_str(kind)
    }
}

/// Invalid public API configuration values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidConfiguration {
    #[error("missing required LifecycleLayer")]
    MissingLifecycleLayer,
}

/// Structured backend error information for stable programmatic handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendErrorInfo {
    pub kind: ApiErrorKind,
    pub operation: Option<BrowserOperation>,
    pub detail: Option<String>,
}

impl std::fmt::Display for BackendErrorInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.operation, self.detail.as_deref()) {
            (Some(operation), Some(detail)) => {
                write!(
                    f,
                    "kind={}, operation={}, detail={detail}",
                    self.kind, operation
                )
            }
            (Some(operation), None) => write!(f, "kind={}, operation={operation}", self.kind),
            (None, Some(detail)) => write!(f, "kind={}, detail={detail}", self.kind),
            (None, None) => write!(f, "kind={}", self.kind),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_error_info_display_contains_structured_fields() {
        let info = BackendErrorInfo {
            kind: ApiErrorKind::CommandDispatchFailed,
            operation: Some(BrowserOperation::SendMouseEvent),
            detail: Some("ConnectionFailed".to_string()),
        };

        let rendered = info.to_string();
        assert!(rendered.contains("kind=command_dispatch_failed"));
        assert!(rendered.contains("operation=send_mouse_event"));
        assert!(rendered.contains("detail=ConnectionFailed"));
    }
}
