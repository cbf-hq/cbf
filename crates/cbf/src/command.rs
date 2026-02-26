//! Browser-generic commands sent from the host application to a backend.
//!
//! This module defines request/operation inputs (`BrowserCommand`) used to
//! control browser lifecycle, navigation, and input handling.

use crate::data::{
    browsing_context_open::BrowsingContextOpenResponse,
    drag::{DragDrop, DragUpdate},
    extension::{AuxiliaryWindowId, AuxiliaryWindowResponse},
    ids::BrowsingContextId,
    ime::{ConfirmCompositionBehavior, ImeCommitText, ImeComposition},
    key::KeyEvent,
    mouse::{MouseEvent, MouseWheelEvent},
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
    /// Confirm a permission request.
    ConfirmPermission {
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        allow: bool,
    },

    /// Create a new web page (tab).
    ///
    /// - `initial_url`: If `None`, the backend may create an empty page.
    /// - `profile_id`: An optional profile identifier (backend-specific).
    CreateBrowsingContext {
        request_id: u64,
        initial_url: Option<String>,
        profile_id: Option<String>,
    },

    /// Fetch the list of available profiles from the backend.
    ListProfiles,
    /// Fetch the list of available extensions from the backend.
    ListExtensions { profile_id: Option<String> },

    /// Request to close a web page.
    RequestCloseBrowsingContext {
        browsing_context_id: BrowsingContextId,
    },

    /// Resize a web page surface.
    ResizeBrowsingContext {
        browsing_context_id: BrowsingContextId,
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

    // --- Input ---
    /// Deliver a keyboard event to the page.
    SendKeyEvent {
        browsing_context_id: BrowsingContextId,
        event: KeyEvent,
        commands: Vec<String>,
    },
    /// Deliver a mouse event to the page.
    SendMouseEvent {
        browsing_context_id: BrowsingContextId,
        event: MouseEvent,
    },
    /// Deliver a mouse wheel event to the page.
    SendMouseWheelEvent {
        browsing_context_id: BrowsingContextId,
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
    /// Finish composing IME text with the given selection behavior.
    FinishComposingText {
        browsing_context_id: BrowsingContextId,
        behavior: ConfirmCompositionBehavior,
    },

    /// Execute a context menu command by menu id.
    ExecuteContextMenuCommand {
        menu_id: u64,
        command_id: i32,
        event_flags: i32,
    },

    /// Dismiss an open context menu.
    DismissContextMenu { menu_id: u64 },

    /// Ask backend to open Chromium's default UI for a pending auxiliary request.
    OpenDefaultAuxiliaryWindow {
        browsing_context_id: BrowsingContextId,
        request_id: u64,
    },

    /// Respond to a pending auxiliary request with host-provided decision.
    RespondAuxiliaryWindow {
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        response: AuxiliaryWindowResponse,
    },

    /// Request backend to close an auxiliary window/dialog.
    CloseAuxiliaryWindow {
        browsing_context_id: BrowsingContextId,
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
    ConfirmPermission,
    CreateBrowsingContext,
    ListProfiles,
    ListExtensions,
    RequestCloseBrowsingContext,
    ResizeBrowsingContext,
    Navigate,
    GoBack,
    GoForward,
    Reload,
    PrintPreview,
    GetBrowsingContextDomHtml,
    SetBrowsingContextFocus,
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
            BrowserCommand::ConfirmPermission { .. } => Self::ConfirmPermission,
            BrowserCommand::CreateBrowsingContext { .. } => Self::CreateBrowsingContext,
            BrowserCommand::ListProfiles => Self::ListProfiles,
            BrowserCommand::ListExtensions { .. } => Self::ListExtensions,
            BrowserCommand::RequestCloseBrowsingContext { .. } => Self::RequestCloseBrowsingContext,
            BrowserCommand::ResizeBrowsingContext { .. } => Self::ResizeBrowsingContext,
            BrowserCommand::Navigate { .. } => Self::Navigate,
            BrowserCommand::GoBack { .. } => Self::GoBack,
            BrowserCommand::GoForward { .. } => Self::GoForward,
            BrowserCommand::Reload { .. } => Self::Reload,
            BrowserCommand::PrintPreview { .. } => Self::PrintPreview,
            BrowserCommand::GetBrowsingContextDomHtml { .. } => Self::GetBrowsingContextDomHtml,
            BrowserCommand::SetBrowsingContextFocus { .. } => Self::SetBrowsingContextFocus,
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
            Self::ConfirmPermission => "confirm_permission",
            Self::CreateBrowsingContext => "create_browsing_context",
            Self::ListProfiles => "list_profiles",
            Self::ListExtensions => "list_extensions",
            Self::RequestCloseBrowsingContext => "request_close_browsing_context",
            Self::ResizeBrowsingContext => "resize_browsing_context",
            Self::Navigate => "navigate",
            Self::GoBack => "go_back",
            Self::GoForward => "go_forward",
            Self::Reload => "reload",
            Self::PrintPreview => "print_preview",
            Self::GetBrowsingContextDomHtml => "get_browsing_context_dom_html",
            Self::SetBrowsingContextFocus => "set_browsing_context_focus",
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
    use crate::data::ids::BrowsingContextId;

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
            profile_id: Some("default".to_string()),
        };

        assert_eq!(
            BrowserOperation::from_command(&command),
            BrowserOperation::CreateBrowsingContext
        );
    }
}
