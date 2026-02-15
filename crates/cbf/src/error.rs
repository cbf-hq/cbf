use crate::command::BrowserCommand;

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

/// Browser operation associated with an execution path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Shutdown,
    ConfirmShutdown,
    ForceShutdown,
    ConfirmBeforeUnload,
    CreateWebPage,
    ListProfiles,
    RequestCloseWebPage,
    ResizeWebPage,
    Navigate,
    GoBack,
    GoForward,
    Reload,
    GetWebPageDomHtml,
    SetWebPageFocus,
    SendKeyEvent,
    SendMouseEvent,
    SendMouseWheelEvent,
    SendDragUpdate,
    SendDragDrop,
    SendDragCancel,
    SetComposition,
    CommitText,
    FinishComposingText,
    ExecuteContextMenuCommand,
    DismissContextMenu,
}

impl Operation {
    pub fn from_command(command: &BrowserCommand) -> Self {
        match command {
            BrowserCommand::Shutdown { .. } => Self::Shutdown,
            BrowserCommand::ConfirmShutdown { .. } => Self::ConfirmShutdown,
            BrowserCommand::ForceShutdown => Self::ForceShutdown,
            BrowserCommand::ConfirmBeforeUnload { .. } => Self::ConfirmBeforeUnload,
            BrowserCommand::CreateWebPage { .. } => Self::CreateWebPage,
            BrowserCommand::ListProfiles => Self::ListProfiles,
            BrowserCommand::RequestCloseWebPage { .. } => Self::RequestCloseWebPage,
            BrowserCommand::ResizeWebPage { .. } => Self::ResizeWebPage,
            BrowserCommand::Navigate { .. } => Self::Navigate,
            BrowserCommand::GoBack { .. } => Self::GoBack,
            BrowserCommand::GoForward { .. } => Self::GoForward,
            BrowserCommand::Reload { .. } => Self::Reload,
            BrowserCommand::GetWebPageDomHtml { .. } => Self::GetWebPageDomHtml,
            BrowserCommand::SetWebPageFocus { .. } => Self::SetWebPageFocus,
            BrowserCommand::SendKeyEvent { .. } => Self::SendKeyEvent,
            BrowserCommand::SendMouseEvent { .. } => Self::SendMouseEvent,
            BrowserCommand::SendMouseWheelEvent { .. } => Self::SendMouseWheelEvent,
            BrowserCommand::SendDragUpdate { .. } => Self::SendDragUpdate,
            BrowserCommand::SendDragDrop { .. } => Self::SendDragDrop,
            BrowserCommand::SendDragCancel { .. } => Self::SendDragCancel,
            BrowserCommand::SetComposition { .. } => Self::SetComposition,
            BrowserCommand::CommitText { .. } => Self::CommitText,
            BrowserCommand::FinishComposingText { .. } => Self::FinishComposingText,
            BrowserCommand::ExecuteContextMenuCommand { .. } => Self::ExecuteContextMenuCommand,
            BrowserCommand::DismissContextMenu { .. } => Self::DismissContextMenu,
        }
    }
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let operation = match self {
            Self::Shutdown => "shutdown",
            Self::ConfirmShutdown => "confirm_shutdown",
            Self::ForceShutdown => "force_shutdown",
            Self::ConfirmBeforeUnload => "confirm_beforeunload",
            Self::CreateWebPage => "create_web_page",
            Self::ListProfiles => "list_profiles",
            Self::RequestCloseWebPage => "request_close_web_page",
            Self::ResizeWebPage => "resize_web_page",
            Self::Navigate => "navigate",
            Self::GoBack => "go_back",
            Self::GoForward => "go_forward",
            Self::Reload => "reload",
            Self::GetWebPageDomHtml => "get_web_page_dom_html",
            Self::SetWebPageFocus => "set_web_page_focus",
            Self::SendKeyEvent => "send_key_event",
            Self::SendMouseEvent => "send_mouse_event",
            Self::SendMouseWheelEvent => "send_mouse_wheel_event",
            Self::SendDragUpdate => "send_drag_update",
            Self::SendDragDrop => "send_drag_drop",
            Self::SendDragCancel => "send_drag_cancel",
            Self::SetComposition => "set_composition",
            Self::CommitText => "commit_text",
            Self::FinishComposingText => "finish_composing_text",
            Self::ExecuteContextMenuCommand => "execute_context_menu_command",
            Self::DismissContextMenu => "dismiss_context_menu",
        };

        f.write_str(operation)
    }
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
    pub operation: Option<Operation>,
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
    use crate::{command::BrowserCommand, data::ids::WebPageId};

    #[test]
    fn operation_from_command_maps_to_expected_variant() {
        let command = BrowserCommand::Navigate {
            web_page_id: WebPageId::new(1),
            url: "https://example.com".to_string(),
        };

        assert_eq!(Operation::from_command(&command), Operation::Navigate);
    }

    #[test]
    fn backend_error_info_display_contains_structured_fields() {
        let info = BackendErrorInfo {
            kind: ApiErrorKind::CommandDispatchFailed,
            operation: Some(Operation::SendMouseEvent),
            detail: Some("ConnectionFailed".to_string()),
        };

        let rendered = info.to_string();
        assert!(rendered.contains("kind=command_dispatch_failed"));
        assert!(rendered.contains("operation=send_mouse_event"));
        assert!(rendered.contains("detail=ConnectionFailed"));
    }

    #[test]
    fn operation_from_command_covers_profile_command() {
        let command = BrowserCommand::CreateWebPage {
            request_id: 42,
            initial_url: None,
            profile_id: Some("default".to_string()),
        };

        assert_eq!(Operation::from_command(&command), Operation::CreateWebPage);
    }
}
