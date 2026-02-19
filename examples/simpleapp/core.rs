use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use cbf::{
    browser::{BrowserHandle, BrowserSession},
    data::{
        context_menu::ContextMenu, drag::DragStartRequest, ids::WebPageId, ime::ImeBoundsUpdate,
        surface::SurfaceHandle,
    },
    event::{BackendStopReason, BrowserEvent, WebPageEvent},
};
use cursor_icon::CursorIcon;
use tracing::{error, info, warn};
use winit::event::WindowEvent;

use crate::cli::Cli;

/// Shared state between the core logic and platform-specific code.
/// This struct is protected by a mutex and can be accessed from multiple parts of the application.
#[derive(Default)]
pub(crate) struct SharedState {
    /// The currently active web page ID, if any.
    pub(crate) web_page_id: Option<WebPageId>,
    /// Maps drag session IDs to their allowed operations bitmask.
    pub(crate) drag_allowed_operations: HashMap<u64, u32>,
}

/// Gets the currently active web page ID from shared state.
pub(crate) fn current_web_page_id(shared: &Arc<Mutex<SharedState>>) -> Option<WebPageId> {
    let guard = shared.lock().expect("shared state lock poisoned");
    guard.web_page_id
}

/// Sets the currently active web page ID in shared state.
pub(crate) fn set_web_page_id(shared: &Arc<Mutex<SharedState>>, web_page_id: Option<WebPageId>) {
    let mut guard = shared.lock().expect("shared state lock poisoned");
    guard.web_page_id = web_page_id;
}

/// Gets the allowed drag operations for a given drag session.
/// Returns 0 if the session is not found.
pub(crate) fn drag_allowed_operations(shared: &Arc<Mutex<SharedState>>, session_id: u64) -> u32 {
    let guard = shared.lock().expect("shared state lock poisoned");
    guard
        .drag_allowed_operations
        .get(&session_id)
        .copied()
        .unwrap_or(0)
}

/// Sets the allowed drag operations for a given drag session.
pub(crate) fn set_drag_allowed_operations(
    shared: &Arc<Mutex<SharedState>>,
    session_id: u64,
    allowed_operations: u32,
) {
    let mut guard = shared.lock().expect("shared state lock poisoned");
    guard
        .drag_allowed_operations
        .insert(session_id, allowed_operations);
}

/// Removes a drag session from shared state when it completes or is cancelled.
pub(crate) fn remove_drag_session(shared: &Arc<Mutex<SharedState>>, session_id: u64) {
    let mut guard = shared.lock().expect("shared state lock poisoned");
    guard.drag_allowed_operations.remove(&session_id);
}

/// Core application state that manages the browser session and handles events.
/// This is the central piece of business logic that connects the CBF browser backend
/// to the platform-specific UI layer.
pub(crate) struct CoreState {
    cli: Cli,
    session: BrowserSession,
    shared: Arc<Mutex<SharedState>>,
    /// Whether we've already requested web page creation (to avoid duplicates).
    page_create_requested: bool,
    /// Whether we've initiated shutdown sequence.
    shutdown_requested: bool,
}

/// Actions that the core state wants the platform layer to execute.
/// These are returned from event handlers and applied by the platform-specific code.
pub(crate) enum CoreAction {
    ExitEventLoop,
    SyncViewAndResize,
    SyncViewResizeAndFocus,
    UpdateWindowTitle(String),
    UpdateCursor(CursorIcon),
    ApplySurfaceHandle(SurfaceHandle),
    ApplyImeBounds(ImeBoundsUpdate),
    ShowContextMenu(ContextMenu),
    StartPlatformDrag(DragStartRequest),
}

impl CoreState {
    pub(crate) fn new(cli: Cli, session: BrowserSession, shared: Arc<Mutex<SharedState>>) -> Self {
        Self {
            cli,
            session,
            shared,
            page_create_requested: false,
            shutdown_requested: false,
        }
    }

    pub(crate) fn browser_handle(&self) -> BrowserHandle {
        self.session.handle()
    }

    pub(crate) fn current_web_page_id(&self) -> Option<WebPageId> {
        current_web_page_id(&self.shared)
    }

    pub(crate) fn sync_page_size(&self, width: u32, height: u32) {
        let Some(web_page_id) = self.current_web_page_id() else {
            return;
        };

        if let Err(err) = self
            .browser_handle()
            .resize_web_page(web_page_id, width, height)
        {
            warn!("failed to resize page: {err}");
        }
    }

    pub(crate) fn set_page_focus(&self, focused: bool) {
        let Some(web_page_id) = self.current_web_page_id() else {
            return;
        };

        if let Err(err) = self
            .browser_handle()
            .set_web_page_focus(web_page_id, focused)
        {
            warn!("failed to sync page focus: {err}");
        }
    }

    pub(crate) fn request_shutdown_once(&mut self) {
        if self.shutdown_requested {
            return;
        }
        self.shutdown_requested = true;

        if let Err(err) = self.browser_handle().request_shutdown(1) {
            warn!("failed to request shutdown: {err}");
        }
    }

    /// Handles incoming browser events and returns platform actions to execute.
    ///
    /// This is the main event processing loop for browser-originated events like
    /// page creation, backend readiness, shutdown notifications, etc.
    pub(crate) fn handle_browser_event(&mut self, event: BrowserEvent) -> Vec<CoreAction> {
        match event {
            BrowserEvent::BackendReady => {
                info!("backend ready");
                if !self.page_create_requested {
                    self.page_create_requested = true;
                    if let Err(err) =
                        self.browser_handle()
                            .create_web_page(1, Some(self.cli.url.clone()), None)
                    {
                        error!("failed to create web page: {err}");
                        return vec![CoreAction::ExitEventLoop];
                    }
                }
                Vec::new()
            }
            BrowserEvent::BackendStopped { reason } => {
                match reason {
                    BackendStopReason::ShutdownRequested => {
                        info!("backend stopped: shutdown requested");
                    }
                    BackendStopReason::Disconnected => {
                        warn!("backend stopped: disconnected");
                    }
                    BackendStopReason::Crashed => {
                        error!("backend stopped: crashed");
                    }
                    BackendStopReason::Error(info) => {
                        error!("backend stopped with error: {info}");
                    }
                }
                vec![CoreAction::ExitEventLoop]
            }
            BrowserEvent::BackendError {
                info,
                terminal_hint,
            } => {
                warn!(
                    "backend error event: {}, terminal_hint={terminal_hint}",
                    info
                );
                Vec::new()
            }
            BrowserEvent::WebPage {
                web_page_id, event, ..
            } => self.handle_web_page_event(web_page_id, event),
            BrowserEvent::ProfilesListed { .. } => Vec::new(),
            BrowserEvent::ShutdownBlocked {
                request_id,
                dirty_web_page_ids,
            } => {
                warn!(
                    "shutdown blocked request_id={request_id}, dirty_pages={}",
                    dirty_web_page_ids.len()
                );
                if let Err(err) = self.browser_handle().confirm_shutdown(request_id, true) {
                    warn!("failed to confirm shutdown: {err}");
                }
                Vec::new()
            }
            BrowserEvent::ShutdownProceeding { request_id } => {
                info!("shutdown proceeding: request_id={request_id}");
                Vec::new()
            }
            BrowserEvent::ShutdownCancelled { request_id } => {
                warn!("shutdown cancelled: request_id={request_id}");
                self.shutdown_requested = false;
                Vec::new()
            }
        }
    }

    /// Handles incoming window events from the platform layer.
    pub(crate) fn handle_window_event(&mut self, event: &WindowEvent) -> Vec<CoreAction> {
        match event {
            WindowEvent::CloseRequested => {
                self.request_shutdown_once();
                vec![CoreAction::ExitEventLoop]
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                vec![CoreAction::SyncViewAndResize]
            }
            WindowEvent::DroppedFile(path) => {
                info!("os dropped file received: {}", path.display());
                Vec::new()
            }
            WindowEvent::HoveredFile(path) => {
                info!("os hovered file: {}", path.display());
                Vec::new()
            }
            WindowEvent::HoveredFileCancelled => {
                info!("os hovered file cancelled");
                Vec::new()
            }
            WindowEvent::Focused(focused) => {
                self.set_page_focus(*focused);
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    fn handle_web_page_event(
        &mut self,
        web_page_id: WebPageId,
        event: WebPageEvent,
    ) -> Vec<CoreAction> {
        match event {
            WebPageEvent::Created { .. } => {
                set_web_page_id(&self.shared, Some(web_page_id));
                vec![CoreAction::SyncViewResizeAndFocus]
            }
            WebPageEvent::SurfaceHandleUpdated { handle } => {
                vec![CoreAction::ApplySurfaceHandle(handle)]
            }
            WebPageEvent::TitleUpdated { title } => vec![CoreAction::UpdateWindowTitle(title)],
            WebPageEvent::CursorChanged { cursor_type } => {
                vec![CoreAction::UpdateCursor(cursor_type)]
            }
            WebPageEvent::ImeBoundsUpdated { update } => vec![CoreAction::ApplyImeBounds(update)],
            WebPageEvent::ContextMenuRequested { menu } => vec![CoreAction::ShowContextMenu(menu)],
            WebPageEvent::DragStartRequested { request } => {
                vec![CoreAction::StartPlatformDrag(request)]
            }
            WebPageEvent::JavaScriptDialogRequested { request_id, .. } => {
                _ = self
                    .browser_handle()
                    .confirm_beforeunload(web_page_id, request_id, false);
                Vec::new()
            }
            WebPageEvent::PermissionRequested { request_id, .. } => {
                _ = self
                    .browser_handle()
                    .confirm_permission(web_page_id, request_id, false);
                Vec::new()
            }
            WebPageEvent::CloseRequested | WebPageEvent::Closed => {
                self.request_shutdown_once();
                vec![CoreAction::ExitEventLoop]
            }
            WebPageEvent::SelectionChanged { .. }
            | WebPageEvent::ScrollPositionChanged { .. }
            | WebPageEvent::NavigationStateChanged { .. }
            | WebPageEvent::FaviconUrlUpdated { .. }
            | WebPageEvent::UpdateTargetUrl { .. }
            | WebPageEvent::FullscreenToggled { .. }
            | WebPageEvent::NewWebPageRequested { .. }
            | WebPageEvent::RenderProcessGone { .. }
            | WebPageEvent::AudioStateChanged { .. }
            | WebPageEvent::DomHtmlRead { .. } => Vec::new(),
        }
    }
}
