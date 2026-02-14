use crate::data::{
    drag::{DragDrop, DragUpdate},
    ids::WebPageId,
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
        web_page_id: WebPageId,
        request_id: u64,
        proceed: bool,
    },

    /// Create a new web page (tab).
    ///
    /// - `initial_url`: If `None`, the backend may create an empty page.
    /// - `profile_id`: An optional profile identifier (backend-specific).
    CreateWebPage {
        request_id: u64,
        initial_url: Option<String>,
        profile_id: Option<String>,
    },

    /// Fetch the list of available profiles from the backend.
    ListProfiles,

    /// Request to close a web page.
    RequestCloseWebPage { web_page_id: WebPageId },

    /// Resize a web page surface.
    ResizeWebPage {
        web_page_id: WebPageId,
        width: u32,
        height: u32,
    },

    // --- Navigation ---
    /// Navigate a page to the provided URL.
    Navigate { web_page_id: WebPageId, url: String },
    /// Navigate back in history for the page.
    GoBack { web_page_id: WebPageId },
    /// Navigate forward in history for the page.
    GoForward { web_page_id: WebPageId },
    /// Reload the current page, optionally bypassing caches.
    Reload {
        web_page_id: WebPageId,
        ignore_cache: bool,
    },

    /// Request the DOM HTML of a web page.
    GetWebPageDomHtml {
        web_page_id: WebPageId,
        request_id: u64,
    },

    // --- Focus / Lifecycle ---
    /// Update whether the given web page should be focused for text input.
    SetWebPageFocus {
        web_page_id: WebPageId,
        focused: bool,
    },

    // --- Input ---
    /// Deliver a keyboard event to the page.
    SendKeyEvent {
        web_page_id: WebPageId,
        event: KeyEvent,
        commands: Vec<String>,
    },
    /// Deliver a mouse event to the page.
    SendMouseEvent {
        web_page_id: WebPageId,
        event: MouseEvent,
    },
    /// Deliver a mouse wheel event to the page.
    SendMouseWheelEvent {
        web_page_id: WebPageId,
        event: MouseWheelEvent,
    },
    /// Deliver drag update for host-owned drag session.
    SendDragUpdate { update: DragUpdate },
    /// Deliver drag drop for host-owned drag session.
    SendDragDrop { drop: DragDrop },
    /// Cancel host-owned drag session.
    SendDragCancel {
        session_id: u64,
        web_page_id: WebPageId,
    },
    /// Update the current IME composition state.
    SetComposition { composition: ImeComposition },
    /// Commit IME text input to the focused element.
    CommitText { commit: ImeCommitText },
    /// Finish composing IME text with the given selection behavior.
    FinishComposingText {
        web_page_id: WebPageId,
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
