//! Browser-generic events emitted from backend to the host application.
//!
//! This module defines top-level backend events (`BrowserEvent`) and
//! web page-scoped events (`BrowsingContextEvent`) for state synchronization.

use cursor_icon::CursorIcon;

use crate::data::{
    context_menu::ContextMenu, drag::DragStartRequest, ids::BrowsingContextId,
    ime::ImeBoundsUpdate, profile::ProfileInfo, surface::SurfaceHandle,
};
use crate::error::BackendErrorInfo;

/// Events emitted by the browser backend as a whole.
#[derive(Debug)]
pub enum BrowserEvent {
    /// The backend is connected and ready to accept commands.
    BackendReady,

    /// The backend stopped due to shutdown, disconnect, or crash.
    BackendStopped { reason: BackendStopReason },

    /// A backend error was observed and surfaced as an event.
    BackendError {
        info: BackendErrorInfo,
        terminal_hint: bool,
    },

    /// An event scoped to a specific web page (tab).
    BrowsingContext {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        event: BrowsingContextEvent,
    },

    /// Result of listing available profiles.
    ProfilesListed { profiles: Vec<ProfileInfo> },

    /// Shutdown is blocked by dirty pages that require confirmation.
    ShutdownBlocked {
        request_id: u64,
        dirty_browsing_context_ids: Vec<BrowsingContextId>,
    },

    /// Shutdown has started and is proceeding.
    ShutdownProceeding { request_id: u64 },

    /// Shutdown has been cancelled.
    ShutdownCancelled { request_id: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendStopReason {
    /// Stopped because an upstream shutdown was requested.
    ShutdownRequested,
    /// Stopped because the command channel was closed or disconnected.
    Disconnected,
    /// Stopped due to a crash or fatal termination.
    Crashed,
    /// Stopped due to an internal backend error.
    Error(BackendErrorInfo),
}

/// Events emitted from a specific web page (tab).
/// The host application consumes these events to update UI and state.
#[derive(Debug)]
pub enum BrowsingContextEvent {
    /// The web page was created.
    Created { request_id: u64 },

    // --- Navigation & History ---
    /// Navigation state changed (back/forward availability and loading state).
    NavigationStateChanged {
        /// Current page URL.
        url: String,
        can_go_back: bool,
        can_go_forward: bool,
        is_loading: bool,
    },

    /// The page title was updated.
    TitleUpdated { title: String },

    /// The favicon URL was updated.
    FaviconUrlUpdated {
        url: String, // TODO: We may eventually need raw image bytes instead.
    },

    // --- UI & Interaction ---
    /// The target URL display should be updated (e.g., hover on link).
    /// `None` means the host should clear the target URL display.
    UpdateTargetUrl { url: Option<String> },

    /// The cursor shape should be updated.
    CursorChanged { cursor_type: CursorIcon },

    /// Fullscreen state toggled.
    FullscreenToggled { is_fullscreen: bool },

    // --- Window & Tab Lifecycle ---
    /// A new web page was requested (e.g., window.open, target="_blank").
    NewBrowsingContextRequested {
        target_url: String,
        // TODO: Add WindowOpenDisposition (Popup, NewTab, etc.).
        is_popup: bool,
    },

    /// A tab close was requested (e.g., window.close).
    CloseRequested,

    /// The web page was closed.
    Closed,

    /// The rendering surface handle was updated.
    SurfaceHandleUpdated { handle: SurfaceHandle },

    /// IME bounds information was updated.
    ///
    /// This payload is browser-generic. Backend-specific IME visual details
    /// should not be carried through this event.
    ImeBoundsUpdated { update: ImeBoundsUpdate },

    /// A context menu display was requested.
    ContextMenuRequested { menu: ContextMenu },

    // --- Dialogs & Permissions (Response Required) ---
    /// A JavaScript dialog (alert/confirm/prompt) was requested.
    /// The host should present a dialog and respond with the matching command.
    JavaScriptDialogRequested {
        request_id: u64,
        message: String,
        default_prompt_text: Option<String>,
        r#type: DialogType,
        beforeunload_reason: Option<BeforeUnloadReason>,
    },

    /// A permission request (camera, microphone, etc.).
    PermissionRequested {
        permission: PermissionType,
        request_id: u64,
    },

    // --- Process Lifecycle ---
    /// The renderer process exited or crashed.
    RenderProcessGone { crashed: bool },

    // --- Audio ---
    /// The audio playback state changed.
    AudioStateChanged { is_audible: bool },

    /// The DOM HTML was read for the page.
    DomHtmlRead { request_id: u64, html: String },

    /// Renderer requested host-owned drag start.
    ///
    /// Carries browser-generic drag payload only.
    DragStartRequested { request: DragStartRequest },

    // --- Additional Signals ---
    /// The text selection range changed.
    SelectionChanged { text: String },

    /// The scroll position changed.
    ScrollPositionChanged {
        // TODO: Define a dedicated coordinate type.
        x: f64,
        y: f64,
    },
}

/// Types of JavaScript dialogs or beforeunload confirmations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogType {
    Alert,
    Confirm,
    Prompt,
    BeforeUnload,
}

/// Reasons for triggering a beforeunload confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeforeUnloadReason {
    Unknown,
    CloseBrowsingContext,
    Navigate,
    Reload,
    WindowClose,
}

/// Response payload for a JavaScript dialog request.
#[derive(Debug)]
pub enum DialogResponse {
    Success {
        input: Option<String>, // Input text for prompt dialogs.
    },
    Cancel,
}

/// Permission categories that may be requested by a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionType {
    VideoCapture,
    AudioCapture,
    Notifications,
    Geolocation,
    // Extend as needed.
}
