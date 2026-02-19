//! Browser-generic commands sent from the host application to a backend.
//!
//! This module defines request/operation inputs (`BrowserCommand`) used to
//! control browser lifecycle, navigation, and input handling.

use crate::data::{
    drag::{DragDrop, DragUpdate},
    ids::BrowsingContextId,
    ime::{ConfirmCompositionBehavior, ImeCommitText, ImeComposition},
    key::KeyEvent,
    mouse::{MouseEvent, MouseWheelEvent},
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

    /// Request to close a web page.
    RequestCloseBrowsingContext { browsing_context_id: BrowsingContextId },

    /// Resize a web page surface.
    ResizeBrowsingContext {
        browsing_context_id: BrowsingContextId,
        width: u32,
        height: u32,
    },

    // --- Navigation ---
    /// Navigate a page to the provided URL.
    Navigate { browsing_context_id: BrowsingContextId, url: String },
    /// Navigate back in history for the page.
    GoBack { browsing_context_id: BrowsingContextId },
    /// Navigate forward in history for the page.
    GoForward { browsing_context_id: BrowsingContextId },
    /// Reload the current page, optionally bypassing caches.
    Reload {
        browsing_context_id: BrowsingContextId,
        ignore_cache: bool,
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
    SendDragUpdate { update: DragUpdate },
    /// Deliver drag drop for host-owned drag session.
    SendDragDrop { drop: DragDrop },
    /// Cancel host-owned drag session.
    SendDragCancel {
        session_id: u64,
        browsing_context_id: BrowsingContextId,
    },
    /// Update the current IME composition state.
    SetComposition { composition: ImeComposition },
    /// Commit IME text input to the focused element.
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
}
