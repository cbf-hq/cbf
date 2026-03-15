//! Browser-generic commands sent from the host application to a backend.
//!
//! This module defines request/operation inputs (`BrowserCommand`) used to
//! control browser lifecycle, navigation, and input handling.

use crate::data::{
    auxiliary_window::{AuxiliaryWindowId, AuxiliaryWindowResponse},
    browsing_context_open::BrowsingContextOpenResponse,
    dialog::DialogResponse,
    download::DownloadId,
    drag::{DragDrop, DragUpdate},
    ids::{BrowsingContextId, TransientBrowsingContextId},
    ime::{ConfirmCompositionBehavior, ImeCommitText, ImeComposition},
    key::KeyEvent,
    mouse::{MouseEvent, MouseWheelEvent},
    transient_browsing_context::{TransientImeCommitText, TransientImeComposition},
    window_open::WindowOpenResponse,
};

/// Commands issued by the UI process (or app) to the browser backend.
///
/// This is intentionally expressed in "browser generic" vocabulary so that
/// `cbf` can be reused outside of Atelier.
#[derive(Debug, Clone)]
pub enum BrowserCommand {
    /// Request the backend to shutdown with a request id.
    ///
    /// Backends may stop their event stream shortly after receiving this.
    Shutdown { request_id: u64 },
    /// Confirm whether shutdown should proceed.
    ConfirmShutdown { request_id: u64, proceed: bool },
    /// Force shutdown without unload confirmations.
    ForceShutdown,

    /// Confirm a beforeunload dialog request.
    ConfirmBeforeUnload {
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        proceed: bool,
    },
    /// Respond to a JavaScript dialog request for a page.
    RespondJavaScriptDialog {
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        response: DialogResponse,
    },
    /// Respond to a JavaScript dialog request for a transient browsing context.
    RespondJavaScriptDialogInTransientBrowsingContext {
        transient_browsing_context_id: TransientBrowsingContextId,
        request_id: u64,
        response: DialogResponse,
    },
    /// Confirm a permission request.
    ConfirmPermission {
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        allow: bool,
    },

    /// Create a new web page (tab).
    ///
    /// - `initial_url`: If `None`, the backend may create an empty page.
    /// - `profile_id`: A canonical backend-issued profile identifier from `ListProfiles`.
    CreateBrowsingContext {
        request_id: u64,
        initial_url: Option<String>,
        profile_id: String,
    },

    /// Fetch the list of available profiles from the backend.
    ListProfiles,
    /// Fetch the list of available extensions from the backend.
    ListExtensions { profile_id: String },

    /// Request to close a web page.
    RequestCloseBrowsingContext {
        browsing_context_id: BrowsingContextId,
    },
    /// Request to close a transient browsing context.
    CloseTransientBrowsingContext {
        transient_browsing_context_id: TransientBrowsingContextId,
    },

    /// Resize a web page surface.
    ResizeBrowsingContext {
        browsing_context_id: BrowsingContextId,
        width: u32,
        height: u32,
    },
    /// Resize a transient browsing context surface.
    ResizeTransientBrowsingContext {
        transient_browsing_context_id: TransientBrowsingContextId,
        width: u32,
        height: u32,
    },

    // --- Navigation ---
    /// Navigate a page to the provided URL.
    Navigate {
        browsing_context_id: BrowsingContextId,
        url: String,
    },
    /// Navigate back in history for the page.
    GoBack {
        browsing_context_id: BrowsingContextId,
    },
    /// Navigate forward in history for the page.
    GoForward {
        browsing_context_id: BrowsingContextId,
    },
    /// Reload the current page, optionally bypassing caches.
    Reload {
        browsing_context_id: BrowsingContextId,
        ignore_cache: bool,
    },
    /// Open print preview for the current page content.
    PrintPreview {
        browsing_context_id: BrowsingContextId,
    },

    /// Request the DOM HTML of a web page.
    GetBrowsingContextDomHtml {
        browsing_context_id: BrowsingContextId,
        request_id: u64,
    },

    // --- Focus / Lifecycle ---
    /// Update whether the given web page should be focused for text input.
    SetBrowsingContextFocus {
        browsing_context_id: BrowsingContextId,
        focused: bool,
    },
    /// Update whether the transient browsing context should be focused for text input.
    SetTransientBrowsingContextFocus {
        transient_browsing_context_id: TransientBrowsingContextId,
        focused: bool,
    },

    // --- Input ---
    /// Deliver a keyboard event to the page.
    SendKeyEvent {
        browsing_context_id: BrowsingContextId,
        event: KeyEvent,
        commands: Vec<String>,
    },
    /// Deliver a keyboard event to a transient browsing context.
    SendKeyEventToTransientBrowsingContext {
        transient_browsing_context_id: TransientBrowsingContextId,
        event: KeyEvent,
        commands: Vec<String>,
    },
    /// Deliver a mouse event to the page.
    SendMouseEvent {
        browsing_context_id: BrowsingContextId,
        event: MouseEvent,
    },
    /// Deliver a mouse event to a transient browsing context.
    SendMouseEventToTransientBrowsingContext {
        transient_browsing_context_id: TransientBrowsingContextId,
        event: MouseEvent,
    },
    /// Deliver a mouse wheel event to the page.
    SendMouseWheelEvent {
        browsing_context_id: BrowsingContextId,
        event: MouseWheelEvent,
    },
    /// Deliver a mouse wheel event to a transient browsing context.
    SendMouseWheelEventToTransientBrowsingContext {
        transient_browsing_context_id: TransientBrowsingContextId,
        event: MouseWheelEvent,
    },
    /// Deliver drag update for host-owned drag session.
    ///
    /// Uses browser-generic drag payload. Backend-native details should stay
    /// in backend-specific extension layers.
    SendDragUpdate { update: DragUpdate },
    /// Deliver drag drop for host-owned drag session.
    SendDragDrop { drop: DragDrop },
    /// Cancel host-owned drag session.
    SendDragCancel {
        session_id: u64,
        browsing_context_id: BrowsingContextId,
    },
    /// Update the current IME composition state.
    ///
    /// `ImeComposition` carries browser-generic span data and may include
    /// optional Chromium-specific style details per span.
    SetComposition { composition: ImeComposition },
    /// Commit IME text input to the focused element.
    ///
    /// `ImeCommitText` follows the same span boundary as `SetComposition`.
    CommitText { commit: ImeCommitText },
    /// Update the current IME composition state for a transient browsing context.
    SetTransientComposition {
        composition: TransientImeComposition,
    },
    /// Commit IME text input to the focused element in a transient browsing context.
    CommitTransientText { commit: TransientImeCommitText },
    /// Finish composing IME text with the given selection behavior.
    FinishComposingText {
        browsing_context_id: BrowsingContextId,
        behavior: ConfirmCompositionBehavior,
    },
    /// Finish composing IME text inside a transient browsing context.
    FinishComposingTextInTransientBrowsingContext {
        transient_browsing_context_id: TransientBrowsingContextId,
        behavior: ConfirmCompositionBehavior,
    },

    /// Accept a host-owned choice menu selection by request id.
    AcceptChoiceMenuSelection { request_id: u64, indices: Vec<i32> },

    /// Dismiss an open host-owned choice menu by request id.
    DismissChoiceMenu { request_id: u64 },

    /// Execute a context menu command by menu id.
    ExecuteContextMenuCommand {
        menu_id: u64,
        command_id: i32,
        event_flags: i32,
    },

    /// Dismiss an open context menu.
    DismissContextMenu { menu_id: u64 },

    /// Pause an in-progress download.
    PauseDownload { download_id: DownloadId },

    /// Resume a paused or resumable download.
    ResumeDownload { download_id: DownloadId },

    /// Cancel an active download.
    CancelDownload { download_id: DownloadId },

    /// Ask backend to open Chromium's default UI for a pending auxiliary request.
    OpenDefaultAuxiliaryWindow { profile_id: String, request_id: u64 },

    /// Respond to a pending auxiliary request with host-provided decision.
    RespondAuxiliaryWindow {
        profile_id: String,
        request_id: u64,
        response: AuxiliaryWindowResponse,
    },

    /// Request backend to close an auxiliary window/dialog.
    CloseAuxiliaryWindow {
        profile_id: String,
        window_id: AuxiliaryWindowId,
    },

    /// Respond to host-mediated browsing context open request.
    RespondBrowsingContextOpen {
        request_id: u64,
        response: BrowsingContextOpenResponse,
    },

    /// Respond to host-mediated window open request.
    RespondWindowOpen {
        request_id: u64,
        response: WindowOpenResponse,
    },
}

/// Browser operation associated with an execution path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserOperation {
    Shutdown,
    ConfirmShutdown,
    ForceShutdown,
    ConfirmBeforeUnload,
    RespondJavaScriptDialog,
    RespondJavaScriptDialogInTransientBrowsingContext,
    ConfirmPermission,
    CreateBrowsingContext,
    ListProfiles,
    ListExtensions,
    RequestCloseBrowsingContext,
    CloseTransientBrowsingContext,
    ResizeBrowsingContext,
    ResizeTransientBrowsingContext,
    Navigate,
    GoBack,
    GoForward,
    Reload,
    PrintPreview,
    GetBrowsingContextDomHtml,
    SetBrowsingContextFocus,
    SetTransientBrowsingContextFocus,
    SendKeyEvent,
    SendKeyEventToTransientBrowsingContext,
    SendMouseEvent,
    SendMouseEventToTransientBrowsingContext,
    SendMouseWheelEvent,
    SendMouseWheelEventToTransientBrowsingContext,
    SendDragUpdate,
    SendDragDrop,
    SendDragCancel,
    SetComposition,
    CommitText,
    SetTransientComposition,
    CommitTransientText,
    FinishComposingText,
    FinishComposingTextInTransientBrowsingContext,
    AcceptChoiceMenuSelection,
    DismissChoiceMenu,
    ExecuteContextMenuCommand,
    DismissContextMenu,
    PauseDownload,
    ResumeDownload,
    CancelDownload,
    OpenDefaultAuxiliaryWindow,
    RespondAuxiliaryWindow,
    CloseAuxiliaryWindow,
    RespondBrowsingContextOpen,
    RespondWindowOpen,
}

impl BrowserOperation {
    pub fn from_command(command: &BrowserCommand) -> Self {
        match command {
            BrowserCommand::Shutdown { .. } => Self::Shutdown,
            BrowserCommand::ConfirmShutdown { .. } => Self::ConfirmShutdown,
            BrowserCommand::ForceShutdown => Self::ForceShutdown,
            BrowserCommand::ConfirmBeforeUnload { .. } => Self::ConfirmBeforeUnload,
            BrowserCommand::RespondJavaScriptDialog { .. } => Self::RespondJavaScriptDialog,
            BrowserCommand::RespondJavaScriptDialogInTransientBrowsingContext { .. } => {
                Self::RespondJavaScriptDialogInTransientBrowsingContext
            }
            BrowserCommand::ConfirmPermission { .. } => Self::ConfirmPermission,
            BrowserCommand::CreateBrowsingContext { .. } => Self::CreateBrowsingContext,
            BrowserCommand::ListProfiles => Self::ListProfiles,
            BrowserCommand::ListExtensions { .. } => Self::ListExtensions,
            BrowserCommand::RequestCloseBrowsingContext { .. } => Self::RequestCloseBrowsingContext,
            BrowserCommand::CloseTransientBrowsingContext { .. } => {
                Self::CloseTransientBrowsingContext
            }
            BrowserCommand::ResizeBrowsingContext { .. } => Self::ResizeBrowsingContext,
            BrowserCommand::ResizeTransientBrowsingContext { .. } => {
                Self::ResizeTransientBrowsingContext
            }
            BrowserCommand::Navigate { .. } => Self::Navigate,
            BrowserCommand::GoBack { .. } => Self::GoBack,
            BrowserCommand::GoForward { .. } => Self::GoForward,
            BrowserCommand::Reload { .. } => Self::Reload,
            BrowserCommand::PrintPreview { .. } => Self::PrintPreview,
            BrowserCommand::GetBrowsingContextDomHtml { .. } => Self::GetBrowsingContextDomHtml,
            BrowserCommand::SetBrowsingContextFocus { .. } => Self::SetBrowsingContextFocus,
            BrowserCommand::SetTransientBrowsingContextFocus { .. } => {
                Self::SetTransientBrowsingContextFocus
            }
            BrowserCommand::SendKeyEvent { .. } => Self::SendKeyEvent,
            BrowserCommand::SendKeyEventToTransientBrowsingContext { .. } => {
                Self::SendKeyEventToTransientBrowsingContext
            }
            BrowserCommand::SendMouseEvent { .. } => Self::SendMouseEvent,
            BrowserCommand::SendMouseEventToTransientBrowsingContext { .. } => {
                Self::SendMouseEventToTransientBrowsingContext
            }
            BrowserCommand::SendMouseWheelEvent { .. } => Self::SendMouseWheelEvent,
            BrowserCommand::SendMouseWheelEventToTransientBrowsingContext { .. } => {
                Self::SendMouseWheelEventToTransientBrowsingContext
            }
            BrowserCommand::SendDragUpdate { .. } => Self::SendDragUpdate,
            BrowserCommand::SendDragDrop { .. } => Self::SendDragDrop,
            BrowserCommand::SendDragCancel { .. } => Self::SendDragCancel,
            BrowserCommand::SetComposition { .. } => Self::SetComposition,
            BrowserCommand::CommitText { .. } => Self::CommitText,
            BrowserCommand::SetTransientComposition { .. } => Self::SetTransientComposition,
            BrowserCommand::CommitTransientText { .. } => Self::CommitTransientText,
            BrowserCommand::FinishComposingText { .. } => Self::FinishComposingText,
            BrowserCommand::FinishComposingTextInTransientBrowsingContext { .. } => {
                Self::FinishComposingTextInTransientBrowsingContext
            }
            BrowserCommand::AcceptChoiceMenuSelection { .. } => Self::AcceptChoiceMenuSelection,
            BrowserCommand::DismissChoiceMenu { .. } => Self::DismissChoiceMenu,
            BrowserCommand::ExecuteContextMenuCommand { .. } => Self::ExecuteContextMenuCommand,
            BrowserCommand::DismissContextMenu { .. } => Self::DismissContextMenu,
            BrowserCommand::PauseDownload { .. } => Self::PauseDownload,
            BrowserCommand::ResumeDownload { .. } => Self::ResumeDownload,
            BrowserCommand::CancelDownload { .. } => Self::CancelDownload,
            BrowserCommand::OpenDefaultAuxiliaryWindow { .. } => Self::OpenDefaultAuxiliaryWindow,
            BrowserCommand::RespondAuxiliaryWindow { .. } => Self::RespondAuxiliaryWindow,
            BrowserCommand::CloseAuxiliaryWindow { .. } => Self::CloseAuxiliaryWindow,
            BrowserCommand::RespondBrowsingContextOpen { .. } => Self::RespondBrowsingContextOpen,
            BrowserCommand::RespondWindowOpen { .. } => Self::RespondWindowOpen,
        }
    }
}

impl std::fmt::Display for BrowserOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let operation = match self {
            Self::Shutdown => "shutdown",
            Self::ConfirmShutdown => "confirm_shutdown",
            Self::ForceShutdown => "force_shutdown",
            Self::ConfirmBeforeUnload => "confirm_beforeunload",
            Self::RespondJavaScriptDialog => "respond_javascript_dialog",
            Self::RespondJavaScriptDialogInTransientBrowsingContext => {
                "respond_javascript_dialog_in_transient_browsing_context"
            }
            Self::ConfirmPermission => "confirm_permission",
            Self::CreateBrowsingContext => "create_browsing_context",
            Self::ListProfiles => "list_profiles",
            Self::ListExtensions => "list_extensions",
            Self::RequestCloseBrowsingContext => "request_close_browsing_context",
            Self::CloseTransientBrowsingContext => "close_transient_browsing_context",
            Self::ResizeBrowsingContext => "resize_browsing_context",
            Self::ResizeTransientBrowsingContext => "resize_transient_browsing_context",
            Self::Navigate => "navigate",
            Self::GoBack => "go_back",
            Self::GoForward => "go_forward",
            Self::Reload => "reload",
            Self::PrintPreview => "print_preview",
            Self::GetBrowsingContextDomHtml => "get_browsing_context_dom_html",
            Self::SetBrowsingContextFocus => "set_browsing_context_focus",
            Self::SetTransientBrowsingContextFocus => "set_transient_browsing_context_focus",
            Self::SendKeyEvent => "send_key_event",
            Self::SendKeyEventToTransientBrowsingContext => {
                "send_key_event_to_transient_browsing_context"
            }
            Self::SendMouseEvent => "send_mouse_event",
            Self::SendMouseEventToTransientBrowsingContext => {
                "send_mouse_event_to_transient_browsing_context"
            }
            Self::SendMouseWheelEvent => "send_mouse_wheel_event",
            Self::SendMouseWheelEventToTransientBrowsingContext => {
                "send_mouse_wheel_event_to_transient_browsing_context"
            }
            Self::SendDragUpdate => "send_drag_update",
            Self::SendDragDrop => "send_drag_drop",
            Self::SendDragCancel => "send_drag_cancel",
            Self::SetComposition => "set_composition",
            Self::CommitText => "commit_text",
            Self::SetTransientComposition => "set_transient_composition",
            Self::CommitTransientText => "commit_transient_text",
            Self::FinishComposingText => "finish_composing_text",
            Self::FinishComposingTextInTransientBrowsingContext => {
                "finish_composing_text_in_transient_browsing_context"
            }
            Self::AcceptChoiceMenuSelection => "accept_choice_menu_selection",
            Self::DismissChoiceMenu => "dismiss_choice_menu",
            Self::ExecuteContextMenuCommand => "execute_context_menu_command",
            Self::DismissContextMenu => "dismiss_context_menu",
            Self::PauseDownload => "pause_download",
            Self::ResumeDownload => "resume_download",
            Self::CancelDownload => "cancel_download",
            Self::OpenDefaultAuxiliaryWindow => "open_default_auxiliary_window",
            Self::RespondAuxiliaryWindow => "respond_auxiliary_window",
            Self::CloseAuxiliaryWindow => "close_auxiliary_window",
            Self::RespondBrowsingContextOpen => "respond_browsing_context_open",
            Self::RespondWindowOpen => "respond_window_open",
        };

        f.write_str(operation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::ids::{BrowsingContextId, TransientBrowsingContextId};

    #[test]
    fn operation_from_command_maps_to_expected_variant() {
        let command = BrowserCommand::Navigate {
            browsing_context_id: BrowsingContextId::new(1),
            url: "https://example.com".to_string(),
        };

        assert_eq!(
            BrowserOperation::from_command(&command),
            BrowserOperation::Navigate
        );
    }
    #[test]
    fn operation_from_command_covers_profile_command() {
        let command = BrowserCommand::CreateBrowsingContext {
            request_id: 42,
            initial_url: None,
            profile_id: "profile-default".to_string(),
        };

        assert_eq!(
            BrowserOperation::from_command(&command),
            BrowserOperation::CreateBrowsingContext
        );
    }

    #[test]
    fn operation_from_command_covers_transient_command() {
        let command = BrowserCommand::CloseTransientBrowsingContext {
            transient_browsing_context_id: TransientBrowsingContextId::new(8),
        };

        assert_eq!(
            BrowserOperation::from_command(&command),
            BrowserOperation::CloseTransientBrowsingContext
        );
    }
}
