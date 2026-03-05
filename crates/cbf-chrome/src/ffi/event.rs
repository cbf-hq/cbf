use cursor_icon::CursorIcon;

use cbf::data::{
    context_menu::ContextMenu, drag::DragStartRequest, extension::ExtensionInfo,
    ime::ImeBoundsUpdate,
};
use cbf::event::BeforeUnloadReason;

use crate::data::{
    ids::TabId,
    prompt_ui::{PromptUiCloseReason, PromptUiId, PromptUiKind, PromptUiResolution},
    surface::SurfaceHandle,
    tab_open::{TabOpenHint, TabOpenResult},
};

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
        browsing_context_id: TabId,
        handle: SurfaceHandle,
    },
    /// A new web page was created by the backend.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::Created`.
    WebContentsCreated {
        profile_id: String,
        browsing_context_id: TabId,
        request_id: u64,
    },
    /// DevTools was opened for a page.
    ///
    /// **Note**: This event does not map to `BrowserEvent` because DevTools is
    /// currently exposed via the Chrome-specific raw event stream only.
    DevToolsOpened {
        profile_id: String,
        browsing_context_id: TabId,
        inspected_browsing_context_id: TabId,
    },
    /// IME bounds information changed.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::ImeBoundsUpdated`.
    ImeBoundsUpdated {
        profile_id: String,
        browsing_context_id: TabId,
        update: ImeBoundsUpdate,
    },
    /// The backend requested a context menu.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::ContextMenuRequested`.
    ContextMenuRequested {
        profile_id: String,
        browsing_context_id: TabId,
        menu: ContextMenu,
    },
    /// Host-mediated open request for tab.
    ///
    /// Maps to `BrowserEvent::BrowsingContextOpenRequested`.
    TabOpenRequested {
        profile_id: String,
        request_id: u64,
        source_tab_id: Option<TabId>,
        target_url: String,
        open_hint: TabOpenHint,
        user_gesture: bool,
    },
    /// Result for host-mediated open request.
    ///
    /// Maps to `BrowserEvent::BrowsingContextOpenResolved`.
    TabOpenResolved {
        profile_id: String,
        request_id: u64,
        result: TabOpenResult,
    },
    /// Navigation state changed for a page.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::NavigationStateChanged`.
    NavigationStateChanged {
        profile_id: String,
        browsing_context_id: TabId,
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
        browsing_context_id: TabId,
        cursor_type: CursorIcon,
    },
    /// The page title changed for a page.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::TitleUpdated`.
    TitleUpdated {
        profile_id: String,
        browsing_context_id: TabId,
        title: String,
    },
    /// The page favicon URL changed for a page.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::FaviconUrlUpdated`.
    FaviconUrlUpdated {
        profile_id: String,
        browsing_context_id: TabId,
        url: String,
    },
    /// A beforeunload dialog was requested.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::JavaScriptDialogRequested`
    /// with `DialogType::BeforeUnload`.
    BeforeUnloadDialogRequested {
        profile_id: String,
        browsing_context_id: TabId,
        request_id: u64,
        reason: BeforeUnloadReason,
    },
    /// A web page closed event was observed.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::Closed`.
    WebContentsClosed {
        profile_id: String,
        browsing_context_id: TabId,
    },
    /// A resize acknowledgement was received for a page.
    ///
    /// **Note**: This event does not map to `BrowserEvent` because it is an internal
    /// acknowledgement with no semantic value for browser-generic consumers.
    WebContentsResizeAcknowledged {
        profile_id: String,
        browsing_context_id: TabId,
    },
    /// The DOM HTML was read for a page.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::DomHtmlRead`.
    WebContentsDomHtmlRead {
        profile_id: String,
        browsing_context_id: TabId,
        request_id: u64,
        html: String,
    },
    /// Host-owned drag start request from renderer.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::DragStartRequested`.
    DragStartRequested {
        profile_id: String,
        browsing_context_id: TabId,
        request: DragStartRequest,
    },
    /// Shutdown is blocked by dirty pages.
    ///
    /// Maps to `BrowserEvent::ShutdownBlocked`.
    ShutdownBlocked {
        request_id: u64,
        dirty_browsing_context_ids: Vec<TabId>,
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
    /// Prompt UI open was requested and host must choose flow.
    PromptUiOpenRequested {
        profile_id: String,
        browsing_context_id: TabId,
        request_id: u64,
        kind: PromptUiKind,
    },
    /// Prompt UI request was resolved.
    PromptUiResolved {
        profile_id: String,
        browsing_context_id: TabId,
        request_id: u64,
        resolution: PromptUiResolution,
    },
    /// Non-fatal extension runtime warning.
    ///
    /// Maps to `BrowsingContextEvent::ExtensionRuntimeWarning`.
    ExtensionRuntimeWarning {
        profile_id: String,
        browsing_context_id: TabId,
        detail: String,
    },
    /// Backend-managed prompt UI surface was opened.
    PromptUiOpened {
        profile_id: String,
        browsing_context_id: TabId,
        prompt_ui_id: PromptUiId,
        kind: PromptUiKind,
        title: Option<String>,
        modal: bool,
    },
    /// Backend-managed prompt UI surface was closed.
    PromptUiClosed {
        profile_id: String,
        browsing_context_id: TabId,
        prompt_ui_id: PromptUiId,
        kind: PromptUiKind,
        reason: PromptUiCloseReason,
    },
}
