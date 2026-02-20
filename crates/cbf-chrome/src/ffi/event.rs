use cursor_icon::CursorIcon;

use cbf::data::{
    context_menu::ContextMenu,
    drag::DragStartRequest,
    ids::BrowsingContextId,
    ime::ImeBoundsUpdate,
};
use cbf::event::BeforeUnloadReason;

use crate::surface::SurfaceHandle;

/// Low-level IPC events emitted by the Chromium bridge.
#[derive(Debug, Clone, PartialEq)]
pub enum IpcEvent {
    /// The rendering surface handle for a page was updated.
    SurfaceHandleUpdated {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        handle: SurfaceHandle,
    },
    /// A new web page was created by the backend.
    WebContentsCreated {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
    },
    /// IME bounds information changed.
    ImeBoundsUpdated {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        update: ImeBoundsUpdate,
    },
    /// The backend requested a context menu.
    ContextMenuRequested {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        menu: ContextMenu,
    },
    /// The backend requested opening a new page.
    NewWebContentsRequested {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        target_url: String,
        is_popup: bool,
    },
    /// Navigation state changed for a page.
    NavigationStateChanged {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        url: String,
        can_go_back: bool,
        can_go_forward: bool,
        is_loading: bool,
    },
    /// Cursor appearance changed for a page.
    CursorChanged {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        cursor_type: CursorIcon,
    },
    /// The page title changed for a page.
    TitleUpdated {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        title: String,
    },
    /// The page favicon URL changed for a page.
    FaviconUrlUpdated {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        url: String,
    },
    /// A beforeunload dialog was requested.
    BeforeUnloadDialogRequested {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        reason: BeforeUnloadReason,
    },
    /// A web page closed event was observed.
    WebContentsClosed {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
    },
    /// A resize acknowledgement was received for a page.
    WebContentsResizeAcknowledged {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
    },
    /// The DOM HTML was read for a page.
    WebContentsDomHtmlRead {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        html: String,
    },
    /// Host-owned drag start request from renderer.
    DragStartRequested {
        profile_id: String,
        browsing_context_id: BrowsingContextId,
        request: DragStartRequest,
    },
    /// Shutdown is blocked by dirty pages.
    ShutdownBlocked {
        request_id: u64,
        dirty_browsing_context_ids: Vec<BrowsingContextId>,
    },
    /// Shutdown has started.
    ShutdownProceeding { request_id: u64 },
    /// Shutdown was cancelled.
    ShutdownCancelled { request_id: u64 },
}
