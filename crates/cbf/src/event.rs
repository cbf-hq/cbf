//! Browser-generic events emitted from backend to the host application.
//!
//! This module defines top-level backend events (`BrowserEvent`) and
//! web page-scoped events (`BrowsingContextEvent`) for state synchronization.

use cursor_icon::CursorIcon;

use crate::data::{
    auxiliary_window::{
        AuxiliaryWindowCloseReason, AuxiliaryWindowId, AuxiliaryWindowKind,
        AuxiliaryWindowResolution,
    },
    browsing_context_open::{BrowsingContextOpenHint, BrowsingContextOpenResult},
    context_menu::ContextMenu,
    dialog::{BeforeUnloadReason, DialogType},
    download::{DownloadId, DownloadOutcome, DownloadState},
    drag::{DragOperation, DragStartRequest},
    extension::ExtensionInfo,
    ids::{BrowsingContextId, TransientBrowsingContextId},
    ime::ImeBoundsUpdate,
    ipc::BrowsingContextIpcMessage,
    permission::PermissionType,
    profile::ProfileInfo,
    transient_browsing_context::{
        TransientBrowsingContextCloseReason, TransientBrowsingContextKind,
    },
    window_open::{WindowDescriptor, WindowOpenRequest, WindowOpenResult},
};
use crate::error::BackendErrorInfo;

/// Events emitted by the browser backend as a whole.
#[derive(Debug, Clone)]
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

    /// An event scoped to a specific browsing context.
    BrowsingContext {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        event: Box<BrowsingContextEvent>,
    },

    /// An event scoped to a transient browsing context.
    TransientBrowsingContext {
        profile_id: String,
        transient_browsing_context_id: TransientBrowsingContextId,
        parent_browsing_context_id: BrowsingContextId,
        event: Box<TransientBrowsingContextEvent>,
    },

    /// Host-mediated open request for a new browsing context.
    BrowsingContextOpenRequested {
        profile_id: String,
        request_id: u64,
        source_browsing_context_id: Option<BrowsingContextId>,
        target_url: String,
        open_hint: BrowsingContextOpenHint,
        user_gesture: bool,
    },

    /// Result of applying host response to browsing context open request.
    BrowsingContextOpenResolved {
        profile_id: String,
        request_id: u64,
        result: BrowsingContextOpenResult,
    },

    /// Host-mediated request for opening/selecting a window.
    WindowOpenRequested {
        profile_id: String,
        request: WindowOpenRequest,
    },

    /// Result of applying host response to a window open request.
    WindowOpenResolved {
        profile_id: String,
        request_id: u64,
        result: WindowOpenResult,
    },

    /// A host-managed window descriptor became visible/active.
    WindowOpened {
        profile_id: String,
        window: WindowDescriptor,
    },

    /// A host-managed window descriptor was closed.
    WindowClosed {
        profile_id: String,
        window_id: crate::data::ids::WindowId,
    },

    /// Result of listing available profiles.
    ProfilesListed { profiles: Vec<ProfileInfo> },

    /// Result of listing available extensions for a profile.
    ExtensionsListed {
        profile_id: String,
        extensions: Vec<ExtensionInfo>,
    },

    /// Auxiliary window open was requested and host must choose flow.
    AuxiliaryWindowOpenRequested {
        profile_id: String,
        request_id: u64,
        source_browsing_context_id: Option<BrowsingContextId>,
        kind: AuxiliaryWindowKind,
    },

    /// Auxiliary window request was resolved.
    AuxiliaryWindowResolved {
        profile_id: String,
        request_id: u64,
        source_browsing_context_id: Option<BrowsingContextId>,
        resolution: AuxiliaryWindowResolution,
    },

    /// Backend-managed auxiliary window/dialog was opened.
    AuxiliaryWindowOpened {
        profile_id: String,
        source_browsing_context_id: Option<BrowsingContextId>,
        window_id: AuxiliaryWindowId,
        kind: AuxiliaryWindowKind,
        title: Option<String>,
        modal: bool,
    },

    /// Backend-managed auxiliary window/dialog was closed.
    AuxiliaryWindowClosed {
        profile_id: String,
        source_browsing_context_id: Option<BrowsingContextId>,
        window_id: AuxiliaryWindowId,
        kind: AuxiliaryWindowKind,
        reason: AuxiliaryWindowCloseReason,
    },

    /// A download became visible to the host lifecycle.
    DownloadCreated {
        profile_id: String,
        download_id: DownloadId,
        source_browsing_context_id: Option<BrowsingContextId>,
        file_name: String,
        total_bytes: Option<u64>,
        target_path: Option<String>,
    },

    /// Download state was synchronized from the backend.
    DownloadUpdated {
        profile_id: String,
        download_id: DownloadId,
        source_browsing_context_id: Option<BrowsingContextId>,
        state: DownloadState,
        file_name: String,
        received_bytes: u64,
        total_bytes: Option<u64>,
        target_path: Option<String>,
        can_resume: bool,
        is_paused: bool,
    },

    /// Download reached a terminal state.
    DownloadCompleted {
        profile_id: String,
        download_id: DownloadId,
        source_browsing_context_id: Option<BrowsingContextId>,
        outcome: DownloadOutcome,
        file_name: String,
        received_bytes: u64,
        total_bytes: Option<u64>,
        target_path: Option<String>,
    },

    /// Shutdown was blocked by a browsing context that requested confirmation.
    ShutdownBlocked {
        request_id: u64,
        dirty_browsing_context_id: BrowsingContextId,
    },

    /// Shutdown has started and is proceeding.
    ShutdownProceeding { request_id: u64 },

    /// Shutdown has been cancelled.
    ShutdownCancelled { request_id: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendStopReason {
    /// Stopped because the command channel was closed or disconnected.
    Disconnected,
    /// Stopped due to a crash or fatal termination.
    Crashed,
    /// Stopped due to an internal backend error.
    Error(BackendErrorInfo),
}

/// Events emitted from a specific browsing context.
/// The host application consumes these events to update UI and state.
#[derive(Debug, Clone)]
pub enum BrowsingContextEvent {
    /// The browsing context was created.
    Created { request_id: u64 },

    // --- Navigation & History ---
    /// Navigation state changed (URL, history capabilities, and loading state).
    ///
    /// Emitted when a browsing context's visible URL or history capabilities
    /// (`can_go_back`, `can_go_forward`) change, including same-document history
    /// updates such as `pushState`, `replaceState`, and same-document
    /// back-forward traversal.
    ///
    /// `is_loading` reflects document-navigation loading state only. Backend
    /// implementations should not toggle it for page-internal data fetches
    /// (for example, XHR/fetch without a document navigation).
    NavigationStateChanged {
        /// Current page URL.
        url: String,
        can_go_back: bool,
        can_go_forward: bool,
        is_loading: bool,
    },

    /// The browsing context title was updated.
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

    /// A browsing context close was requested (e.g., window.close).
    CloseRequested,

    /// The browsing context was closed.
    Closed,

    /// IME bounds information was updated.
    ///
    /// This payload is browser-generic. Backend-specific IME visual details
    /// should not be carried through this event.
    ImeBoundsUpdated { update: ImeBoundsUpdate },

    /// A host-owned choice menu display was requested.
    ///
    /// Backend-specific payload should be retrieved from the backend-native raw
    /// event stream using this request id.
    ChoiceMenuRequested { request_id: u64 },

    /// A context menu display was requested.
    ContextMenuRequested { menu: ContextMenu },

    /// A browsing context IPC message was received.
    ///
    /// `message.channel` follows the same contract as command payloads and is
    /// always non-empty.
    IpcMessageReceived { message: BrowsingContextIpcMessage },

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

    /// The negotiated operation for an active external drag changed.
    ExternalDragOperationChanged { operation: DragOperation },

    /// Non-fatal extension runtime warning.
    ExtensionRuntimeWarning { detail: String },

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

/// Events emitted from a transient browsing context.
#[derive(Debug, Clone)]
pub enum TransientBrowsingContextEvent {
    /// The transient browsing context was opened and is ready for host lifecycle.
    Opened {
        kind: TransientBrowsingContextKind,
        title: Option<String>,
    },

    /// Focus moved to this transient browsing context.
    Focused,

    /// Focus left this transient browsing context.
    Blurred,

    /// The transient browsing context surface size changed.
    Resized { width: u32, height: u32 },

    /// IME bounds information was updated for this transient browsing context.
    ///
    /// This payload is browser-generic. Backend-specific IME visual details
    /// should not be carried through this event.
    ImeBoundsUpdated { update: ImeBoundsUpdate },

    /// The cursor shape should be updated for this transient browsing context.
    CursorChanged { cursor_type: CursorIcon },

    /// A host-owned choice menu display was requested for this transient
    /// browsing context.
    ///
    /// Backend-specific payload should be retrieved from the backend-native raw
    /// event stream using this request id.
    ChoiceMenuRequested { request_id: u64 },

    /// A context menu display was requested for this transient browsing context.
    ContextMenuRequested { menu: ContextMenu },

    /// A JavaScript dialog (alert/confirm/prompt/beforeunload) was requested.
    JavaScriptDialogRequested {
        request_id: u64,
        message: String,
        default_prompt_text: Option<String>,
        r#type: DialogType,
        beforeunload_reason: Option<BeforeUnloadReason>,
    },

    /// The transient browsing context title was updated.
    TitleUpdated { title: String },

    /// The transient browsing context requested to close itself.
    CloseRequested,

    /// The renderer process for the transient browsing context exited or crashed.
    RenderProcessGone { crashed: bool },

    /// The transient browsing context closed.
    Closed {
        reason: TransientBrowsingContextCloseReason,
    },
}
