use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use cbf::{
    browser::{BrowserHandle, BrowserSession},
    data::{
        context_menu::ContextMenu,
        drag::{DragOperations, DragStartRequest},
        ids::BrowsingContextId,
        ime::ImeBoundsUpdate,
    },
    event::{BackendStopReason, BrowserEvent, BrowsingContextEvent},
};
use cbf_chrome::{backend::ChromiumBackend, data::surface::SurfaceHandle};
use cursor_icon::CursorIcon;
use tracing::{error, info, warn};
use winit::event::WindowEvent;

use crate::cli::Cli;

/// Shared state between the core logic and platform-specific code.
/// This struct is protected by a mutex and can be accessed from multiple parts of the application.
#[derive(Default)]
pub(crate) struct SharedState {
    /// The primary (inspected page) browsing context ID.
    pub(crate) primary_browsing_context_id: Option<BrowsingContextId>,
    /// The DevTools browsing context ID.
    pub(crate) devtools_browsing_context_id: Option<BrowsingContextId>,
    /// Maps drag session IDs to their allowed operations.
    pub(crate) drag_allowed_operations: HashMap<u64, DragOperations>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViewTarget {
    Primary,
    DevTools,
}

/// Gets the browsing context ID for the requested target.
pub(crate) fn browsing_context_id_for_target(
    shared: &Arc<Mutex<SharedState>>,
    target: ViewTarget,
) -> Option<BrowsingContextId> {
    let guard = shared.lock().expect("shared state lock poisoned");
    match target {
        ViewTarget::Primary => guard.primary_browsing_context_id,
        ViewTarget::DevTools => guard.devtools_browsing_context_id,
    }
}

pub(crate) fn set_primary_browsing_context_id(
    shared: &Arc<Mutex<SharedState>>,
    browsing_context_id: Option<BrowsingContextId>,
) {
    let mut guard = shared.lock().expect("shared state lock poisoned");
    guard.primary_browsing_context_id = browsing_context_id;
}

pub(crate) fn set_devtools_browsing_context_id(
    shared: &Arc<Mutex<SharedState>>,
    browsing_context_id: Option<BrowsingContextId>,
) {
    let mut guard = shared.lock().expect("shared state lock poisoned");
    guard.devtools_browsing_context_id = browsing_context_id;
}

/// Gets the allowed drag operations for a given drag session.
/// Returns no allowed operations if the session is not found.
pub(crate) fn drag_allowed_operations(
    shared: &Arc<Mutex<SharedState>>,
    session_id: u64,
) -> DragOperations {
    let guard = shared.lock().expect("shared state lock poisoned");
    guard
        .drag_allowed_operations
        .get(&session_id)
        .copied()
        .unwrap_or_default()
}

/// Sets the allowed drag operations for a given drag session.
pub(crate) fn set_drag_allowed_operations(
    shared: &Arc<Mutex<SharedState>>,
    session_id: u64,
    allowed_operations: DragOperations,
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
    session: BrowserSession<ChromiumBackend>,
    shared: Arc<Mutex<SharedState>>,
    /// Whether we've already requested browsing context creation (to avoid duplicates).
    page_create_requested: bool,
    /// Whether we've initiated shutdown sequence.
    shutdown_requested: bool,
}

/// Actions that the core state wants the platform layer to execute.
/// These are returned from event handlers and applied by the platform-specific code.
pub(crate) enum CoreAction {
    ExitEventLoop,
    SyncViewAndResize,
    SyncViewResizeAndFocus(ViewTarget),
    UpdateWindowTitle(String),
    UpdateCursor(CursorIcon),
    ApplySurfaceHandle {
        browsing_context_id: BrowsingContextId,
        handle: SurfaceHandle,
    },
    ApplyImeBounds {
        browsing_context_id: BrowsingContextId,
        update: ImeBoundsUpdate,
    },
    ShowContextMenu {
        browsing_context_id: BrowsingContextId,
        menu: ContextMenu,
    },
    StartPlatformDrag(DragStartRequest),
}

impl CoreState {
    pub(crate) fn new(
        cli: Cli,
        session: BrowserSession<ChromiumBackend>,
        shared: Arc<Mutex<SharedState>>,
    ) -> Self {
        Self {
            cli,
            session,
            shared,
            page_create_requested: false,
            shutdown_requested: false,
        }
    }

    pub(crate) fn browser_handle(&self) -> BrowserHandle<ChromiumBackend> {
        self.session.handle()
    }

    pub(crate) fn browsing_context_id_for_target(
        &self,
        target: ViewTarget,
    ) -> Option<BrowsingContextId> {
        browsing_context_id_for_target(&self.shared, target)
    }

    pub(crate) fn sync_page_size(
        &self,
        browsing_context_id: BrowsingContextId,
        width: u32,
        height: u32,
    ) {
        if let Err(err) =
            self.browser_handle()
                .resize_browsing_context(browsing_context_id, width, height)
        {
            warn!("failed to resize page: {err}");
        }
    }

    pub(crate) fn set_page_focus(&self, browsing_context_id: BrowsingContextId, focused: bool) {
        if let Err(err) = self
            .browser_handle()
            .set_browsing_context_focus(browsing_context_id, focused)
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

    pub(crate) fn handle_surface_update(
        &self,
        browsing_context_id: BrowsingContextId,
        handle: SurfaceHandle,
    ) -> Vec<CoreAction> {
        vec![CoreAction::ApplySurfaceHandle {
            browsing_context_id,
            handle,
        }]
    }

    pub(crate) fn handle_devtools_opened(
        &mut self,
        browsing_context_id: BrowsingContextId,
        inspected_browsing_context_id: BrowsingContextId,
    ) -> Vec<CoreAction> {
        info!(
            "devtools opened: inspected={}, devtools={}",
            inspected_browsing_context_id,
            browsing_context_id
        );
        set_devtools_browsing_context_id(&self.shared, Some(browsing_context_id));
        vec![CoreAction::SyncViewResizeAndFocus(ViewTarget::DevTools)]
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
                    if let Err(err) = self.browser_handle().create_browsing_context(
                        1,
                        Some(self.cli.url.clone()),
                        None,
                    ) {
                        error!("failed to create browsing context: {err}");
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
            BrowserEvent::BrowsingContext {
                browsing_context_id,
                event,
                ..
            } => self.handle_browsing_context_event(browsing_context_id, *event),
            BrowserEvent::ProfilesListed { .. } => Vec::new(),
            BrowserEvent::ShutdownBlocked {
                request_id,
                dirty_browsing_context_ids,
            } => {
                warn!(
                    "shutdown blocked request_id={request_id}, dirty_pages={}",
                    dirty_browsing_context_ids.len()
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
                if let Some(id) = self.browsing_context_id_for_target(ViewTarget::Primary) {
                    self.set_page_focus(id, *focused);
                }
                if let Some(id) = self.browsing_context_id_for_target(ViewTarget::DevTools) {
                    self.set_page_focus(id, *focused);
                }
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    fn handle_browsing_context_event(
        &mut self,
        browsing_context_id: BrowsingContextId,
        event: BrowsingContextEvent,
    ) -> Vec<CoreAction> {
        match event {
            BrowsingContextEvent::Created { .. } => {
                set_primary_browsing_context_id(&self.shared, Some(browsing_context_id));
                vec![CoreAction::SyncViewResizeAndFocus(ViewTarget::Primary)]
            }
            BrowsingContextEvent::TitleUpdated { title } => {
                vec![CoreAction::UpdateWindowTitle(title)]
            }
            BrowsingContextEvent::CursorChanged { cursor_type } => {
                vec![CoreAction::UpdateCursor(cursor_type)]
            }
            BrowsingContextEvent::ImeBoundsUpdated { update } => {
                vec![CoreAction::ApplyImeBounds {
                    browsing_context_id,
                    update,
                }]
            }
            BrowsingContextEvent::ContextMenuRequested { menu } => {
                vec![CoreAction::ShowContextMenu {
                    browsing_context_id,
                    menu,
                }]
            }
            BrowsingContextEvent::DragStartRequested { request } => {
                vec![CoreAction::StartPlatformDrag(request)]
            }
            BrowsingContextEvent::JavaScriptDialogRequested { request_id, .. } => {
                _ = self.browser_handle().confirm_beforeunload(
                    browsing_context_id,
                    request_id,
                    false,
                );
                Vec::new()
            }
            BrowsingContextEvent::PermissionRequested { request_id, .. } => {
                _ = self.browser_handle().confirm_permission(
                    browsing_context_id,
                    request_id,
                    false,
                );
                Vec::new()
            }
            BrowsingContextEvent::CloseRequested | BrowsingContextEvent::Closed => {
                self.request_shutdown_once();
                vec![CoreAction::ExitEventLoop]
            }
            BrowsingContextEvent::SelectionChanged { .. }
            | BrowsingContextEvent::ScrollPositionChanged { .. }
            | BrowsingContextEvent::NavigationStateChanged { .. }
            | BrowsingContextEvent::FaviconUrlUpdated { .. }
            | BrowsingContextEvent::UpdateTargetUrl { .. }
            | BrowsingContextEvent::FullscreenToggled { .. }
            | BrowsingContextEvent::NewBrowsingContextRequested { .. }
            | BrowsingContextEvent::RenderProcessGone { .. }
            | BrowsingContextEvent::AudioStateChanged { .. }
            | BrowsingContextEvent::DomHtmlRead { .. } => Vec::new(),
        }
    }
}
