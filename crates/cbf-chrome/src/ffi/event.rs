use cursor_icon::CursorIcon;

use cbf::data::{
    browsing_context_open::{BrowsingContextOpenHint, BrowsingContextOpenResult},
    context_menu::ContextMenu,
    drag::DragStartRequest,
    extension::{
        AuxiliaryWindowCloseReason, AuxiliaryWindowId, AuxiliaryWindowKind,
        AuxiliaryWindowResolution, ExtensionInfo,
    },
    ids::BrowsingContextId,
    ime::ImeBoundsUpdate,
};
use cbf::event::BeforeUnloadReason;

use crate::data::surface::SurfaceHandle;

/// Low-level IPC events emitted by the Chromium bridge.
#[derive(Debug, Clone, PartialEq)]
pub enum IpcEvent {
    /// The rendering surface handle for a page was updated.
    ///
    /// **Note**: This event does not map to `BrowserEvent` because surface handles
    /// are a Chrome-specific rendering implementation detail. Applications needing
    /// this information should subscribe to the raw `ChromeEvent` stream.
    SurfaceHandleUpdated {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        handle: SurfaceHandle,
    },
    /// A new web page was created by the backend.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::Created`.
    WebContentsCreated {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
    },
    /// DevTools was opened for a page.
    ///
    /// **Note**: This event does not map to `BrowserEvent` because DevTools is
    /// currently exposed via the Chrome-specific raw event stream only.
    DevToolsOpened {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        inspected_browsing_context_id: BrowsingContextId,
    },
    /// IME bounds information changed.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::ImeBoundsUpdated`.
    ImeBoundsUpdated {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        update: ImeBoundsUpdate,
    },
    /// The backend requested a context menu.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::ContextMenuRequested`.
    ContextMenuRequested {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        menu: ContextMenu,
    },
    /// Host-mediated open request for browsing context.
    ///
    /// Maps to `BrowserEvent::BrowsingContextOpenRequested`.
    BrowsingContextOpenRequested {
        profile_id: String,
        request_id: u64,
        source_browsing_context_id: Option<BrowsingContextId>,
        target_url: String,
        open_hint: BrowsingContextOpenHint,
        user_gesture: bool,
    },
    /// Result for host-mediated open request.
    ///
    /// Maps to `BrowserEvent::BrowsingContextOpenResolved`.
    BrowsingContextOpenResolved {
        profile_id: String,
        request_id: u64,
        result: BrowsingContextOpenResult,
    },
    /// Navigation state changed for a page.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::NavigationStateChanged`.
    NavigationStateChanged {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        url: String,
        can_go_back: bool,
        can_go_forward: bool,
        is_loading: bool,
    },
    /// Cursor appearance changed for a page.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::CursorChanged`.
    CursorChanged {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        cursor_type: CursorIcon,
    },
    /// The page title changed for a page.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::TitleUpdated`.
    TitleUpdated {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        title: String,
    },
    /// The page favicon URL changed for a page.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::FaviconUrlUpdated`.
    FaviconUrlUpdated {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        url: String,
    },
    /// A beforeunload dialog was requested.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::JavaScriptDialogRequested`
    /// with `DialogType::BeforeUnload`.
    BeforeUnloadDialogRequested {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        reason: BeforeUnloadReason,
    },
    /// A web page closed event was observed.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::Closed`.
    WebContentsClosed {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
    },
    /// A resize acknowledgement was received for a page.
    ///
    /// **Note**: This event does not map to `BrowserEvent` because it is an internal
    /// acknowledgement with no semantic value for browser-generic consumers.
    WebContentsResizeAcknowledged {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
    },
    /// The DOM HTML was read for a page.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::DomHtmlRead`.
    WebContentsDomHtmlRead {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        html: String,
    },
    /// Host-owned drag start request from renderer.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::DragStartRequested`.
    DragStartRequested {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        request: DragStartRequest,
    },
    /// Shutdown is blocked by dirty pages.
    ///
    /// Maps to `BrowserEvent::ShutdownBlocked`.
    ShutdownBlocked {
        request_id: u64,
        dirty_browsing_context_ids: Vec<BrowsingContextId>,
    },
    /// Shutdown has started.
    ///
    /// Maps to `BrowserEvent::ShutdownProceeding`.
    ShutdownProceeding { request_id: u64 },
    /// Shutdown was cancelled.
    ///
    /// Maps to `BrowserEvent::ShutdownCancelled`.
    ShutdownCancelled { request_id: u64 },
    /// Installed extensions were listed for a profile.
    ///
    /// Maps to `BrowserEvent::ExtensionsListed`.
    ExtensionsListed {
        profile_id: String,
        extensions: Vec<ExtensionInfo>,
    },
    /// Auxiliary window open was requested and host must choose flow.
    AuxiliaryWindowOpenRequested {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        kind: AuxiliaryWindowKind,
    },
    /// Auxiliary request was resolved.
    AuxiliaryWindowResolved {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        resolution: AuxiliaryWindowResolution,
    },
    /// Non-fatal extension runtime warning.
    ///
    /// Maps to `BrowsingContextEvent::ExtensionRuntimeWarning`.
    ExtensionRuntimeWarning {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        detail: String,
    },
    /// Backend-managed auxiliary window/dialog was opened.
    AuxiliaryWindowOpened {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        window_id: AuxiliaryWindowId,
        kind: AuxiliaryWindowKind,
        title: Option<String>,
        modal: bool,
    },
    /// Backend-managed auxiliary window/dialog was closed.
    AuxiliaryWindowClosed {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        window_id: AuxiliaryWindowId,
        kind: AuxiliaryWindowKind,
        reason: AuxiliaryWindowCloseReason,
    },
}
