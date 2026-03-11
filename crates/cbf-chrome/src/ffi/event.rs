use cursor_icon::CursorIcon;

use cbf::data::dialog::DialogType;

use crate::data::{
    context_menu::ChromeContextMenu,
    download::{ChromeDownloadCompletion, ChromeDownloadProgress, ChromeDownloadSnapshot},
    drag::ChromeDragStartRequest,
    extension::ChromeExtensionInfo,
    ids::{PopupId, TabId},
    ime::ChromeImeBoundsUpdate,
    lifecycle::ChromeBeforeUnloadReason,
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
    /// An extension action popup lifecycle started.
    ExtensionPopupOpened {
        profile_id: String,
        browsing_context_id: TabId,
        popup_id: u64,
        extension_id: String,
        title: String,
    },
    /// The rendering surface handle for an extension popup was updated.
    ExtensionPopupSurfaceHandleUpdated {
        profile_id: String,
        browsing_context_id: TabId,
        popup_id: u64,
        handle: SurfaceHandle,
    },
    /// The effective popup size changed after Chromium-side clamping.
    ExtensionPopupPreferredSizeChanged {
        profile_id: String,
        browsing_context_id: TabId,
        popup_id: u64,
        width: u32,
        height: u32,
    },
    /// A context menu was requested for an extension popup.
    ExtensionPopupContextMenuRequested {
        profile_id: String,
        browsing_context_id: TabId,
        popup_id: PopupId,
        menu: ChromeContextMenu,
    },
    /// The cursor appearance changed for an extension popup.
    ExtensionPopupCursorChanged {
        profile_id: String,
        browsing_context_id: TabId,
        popup_id: PopupId,
        cursor_type: CursorIcon,
    },
    /// The title changed for an extension popup.
    ExtensionPopupTitleUpdated {
        profile_id: String,
        browsing_context_id: TabId,
        popup_id: PopupId,
        title: String,
    },
    /// A JavaScript dialog was requested for an extension popup.
    ExtensionPopupJavaScriptDialogRequested {
        profile_id: String,
        browsing_context_id: TabId,
        popup_id: PopupId,
        request_id: u64,
        r#type: DialogType,
        message: String,
        default_prompt_text: Option<String>,
        reason: ChromeBeforeUnloadReason,
    },
    /// The extension popup requested to close.
    ExtensionPopupCloseRequested {
        profile_id: String,
        browsing_context_id: TabId,
        popup_id: PopupId,
    },
    /// The extension popup renderer exited or crashed.
    ExtensionPopupRenderProcessGone {
        profile_id: String,
        browsing_context_id: TabId,
        popup_id: PopupId,
        crashed: bool,
    },
    /// An extension action popup closed.
    ExtensionPopupClosed {
        profile_id: String,
        browsing_context_id: TabId,
        popup_id: u64,
    },
    /// A new tab was created by the backend.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::Created`.
    TabCreated {
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
        update: ChromeImeBoundsUpdate,
    },
    /// IME bounds information changed for an extension popup.
    ///
    /// Maps to `BrowserEvent::TransientBrowsingContext` with
    /// `TransientBrowsingContextEvent::ImeBoundsUpdated`.
    ExtensionPopupImeBoundsUpdated {
        profile_id: String,
        browsing_context_id: TabId,
        popup_id: PopupId,
        update: ChromeImeBoundsUpdate,
    },
    /// The backend requested a context menu.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::ContextMenuRequested`.
    ContextMenuRequested {
        profile_id: String,
        browsing_context_id: TabId,
        menu: ChromeContextMenu,
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
        reason: ChromeBeforeUnloadReason,
    },
    /// A JavaScript dialog was requested for a tab.
    JavaScriptDialogRequested {
        profile_id: String,
        browsing_context_id: TabId,
        request_id: u64,
        r#type: DialogType,
        message: String,
        default_prompt_text: Option<String>,
        reason: ChromeBeforeUnloadReason,
    },
    /// A tab closed event was observed.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::Closed`.
    TabClosed {
        profile_id: String,
        browsing_context_id: TabId,
    },
    /// A resize acknowledgement was received for a page.
    ///
    /// **Note**: This event does not map to `BrowserEvent` because it is an internal
    /// acknowledgement with no semantic value for browser-generic consumers.
    TabResizeAcknowledged {
        profile_id: String,
        browsing_context_id: TabId,
    },
    /// The DOM HTML was read for a page.
    ///
    /// Maps to `BrowserEvent::BrowsingContext` with `BrowsingContextEvent::DomHtmlRead`.
    TabDomHtmlRead {
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
        request: ChromeDragStartRequest,
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
        extensions: Vec<ChromeExtensionInfo>,
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
    /// Download lifecycle became visible to the host.
    DownloadCreated {
        profile_id: String,
        download: ChromeDownloadSnapshot,
    },
    /// Download state changed.
    DownloadUpdated {
        profile_id: String,
        download: ChromeDownloadProgress,
    },
    /// Download reached a terminal state.
    DownloadCompleted {
        profile_id: String,
        download: ChromeDownloadCompletion,
    },
}
