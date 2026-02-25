use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use cbf::{
    browser::{BrowserHandle, BrowserSession},
    data::{
        browsing_context_open::BrowsingContextOpenHint,
        browsing_context_open::BrowsingContextOpenResponse,
        context_menu::ContextMenu,
        drag::{DragOperations, DragStartRequest},
        extension::{AuxiliaryWindowKind, AuxiliaryWindowResponse},
        ids::{BrowsingContextId, WindowId},
        ime::ImeBoundsUpdate,
        window_open::{
            WindowBounds, WindowDescriptor, WindowKind, WindowOpenResponse, WindowOpenResult,
            WindowState,
        },
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
    /// The initial host window used for app startup browsing contexts.
    pub(crate) primary_host_window_id: Option<WindowId>,
    /// Maps browsing contexts to host window IDs.
    pub(crate) browsing_context_to_window: HashMap<BrowsingContextId, WindowId>,
    /// Maps host window IDs to browsing contexts.
    pub(crate) window_to_browsing_context: HashMap<WindowId, BrowsingContextId>,
    /// Tracks pending backend window-open request IDs to the host window ID selected by app.
    pub(crate) pending_window_open_requests: HashMap<u64, WindowId>,
    /// Maps drag session IDs to their allowed operations.
    pub(crate) drag_allowed_operations: HashMap<u64, DragOperations>,
}

pub(crate) const PRIMARY_HOST_WINDOW_ID: WindowId = WindowId::new(u64::MAX - 1);

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

pub(crate) fn set_primary_host_window_id(shared: &Arc<Mutex<SharedState>>, window_id: WindowId) {
    let mut guard = shared.lock().expect("shared state lock poisoned");
    guard.primary_host_window_id = Some(window_id);
}

pub(crate) fn primary_host_window_id(shared: &Arc<Mutex<SharedState>>) -> Option<WindowId> {
    let guard = shared.lock().expect("shared state lock poisoned");
    guard.primary_host_window_id
}

pub(crate) fn register_pending_window_open_request(
    shared: &Arc<Mutex<SharedState>>,
    request_id: u64,
    window_id: WindowId,
) {
    let mut guard = shared.lock().expect("shared state lock poisoned");
    guard
        .pending_window_open_requests
        .insert(request_id, window_id);
}

pub(crate) fn take_pending_window_open_request(
    shared: &Arc<Mutex<SharedState>>,
    request_id: u64,
) -> Option<WindowId> {
    let mut guard = shared.lock().expect("shared state lock poisoned");
    guard.pending_window_open_requests.remove(&request_id)
}

pub(crate) fn bind_browsing_context_to_window(
    shared: &Arc<Mutex<SharedState>>,
    browsing_context_id: BrowsingContextId,
    window_id: WindowId,
) {
    let mut guard = shared.lock().expect("shared state lock poisoned");
    guard
        .browsing_context_to_window
        .insert(browsing_context_id, window_id);
    guard
        .window_to_browsing_context
        .insert(window_id, browsing_context_id);
}

pub(crate) fn unbind_browsing_context(
    shared: &Arc<Mutex<SharedState>>,
    browsing_context_id: BrowsingContextId,
) -> Option<WindowId> {
    let mut guard = shared.lock().expect("shared state lock poisoned");
    let window_id = guard
        .browsing_context_to_window
        .remove(&browsing_context_id);
    if let Some(window_id) = window_id {
        guard.window_to_browsing_context.remove(&window_id);
    }
    window_id
}

pub(crate) fn window_id_for_browsing_context(
    shared: &Arc<Mutex<SharedState>>,
    browsing_context_id: BrowsingContextId,
) -> Option<WindowId> {
    let guard = shared.lock().expect("shared state lock poisoned");
    guard
        .browsing_context_to_window
        .get(&browsing_context_id)
        .copied()
}

pub(crate) fn browsing_context_id_for_window(
    shared: &Arc<Mutex<SharedState>>,
    window_id: WindowId,
) -> Option<BrowsingContextId> {
    let guard = shared.lock().expect("shared state lock poisoned");
    guard.window_to_browsing_context.get(&window_id).copied()
}

pub(crate) fn has_bound_windows(shared: &Arc<Mutex<SharedState>>) -> bool {
    let guard = shared.lock().expect("shared state lock poisoned");
    !guard.window_to_browsing_context.is_empty()
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
    EnsureHostWindow {
        window: WindowDescriptor,
    },
    CloseHostWindow {
        window_id: WindowId,
    },
    SyncWindowAndResize {
        window_id: WindowId,
    },
    SyncWindowResizeAndFocus {
        window_id: WindowId,
    },
    UpdateWindowTitle {
        window_id: WindowId,
        title: String,
    },
    UpdateCursor {
        window_id: WindowId,
        cursor: CursorIcon,
    },
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
            inspected_browsing_context_id, browsing_context_id
        );
        set_devtools_browsing_context_id(&self.shared, Some(browsing_context_id));
        Vec::new()
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
            BrowserEvent::BrowsingContextOpenRequested {
                request_id,
                source_browsing_context_id,
                target_url,
                open_hint,
                user_gesture,
                ..
            } => {
                info!(
                    "browsing context open requested: request_id={request_id}, source={source_browsing_context_id:?}, target_url={target_url}, hint={open_hint:?}, user_gesture={user_gesture}"
                );
                let response = match open_hint {
                    BrowsingContextOpenHint::NewWindow | BrowsingContextOpenHint::Popup => {
                        BrowsingContextOpenResponse::AllowNewContext { activate: true }
                    }
                    BrowsingContextOpenHint::Unknown
                    | BrowsingContextOpenHint::CurrentContext
                    | BrowsingContextOpenHint::NewForegroundContext
                    | BrowsingContextOpenHint::NewBackgroundContext => {
                        let target_browsing_context_id = source_browsing_context_id.or_else(|| {
                            browsing_context_id_for_target(&self.shared, ViewTarget::Primary)
                        });

                        if let Some(browsing_context_id) = target_browsing_context_id {
                            BrowsingContextOpenResponse::AllowExistingContext {
                                browsing_context_id,
                                activate: true,
                            }
                        } else {
                            warn!(
                                "no reusable browsing context available for request_id={request_id}; denying open request"
                            );
                            BrowsingContextOpenResponse::Deny
                        }
                    }
                };
                if let Err(err) = self
                    .browser_handle()
                    .respond_browsing_context_open(request_id, response)
                {
                    warn!("failed to respond browsing context open request: {err}");
                }
                Vec::new()
            }
            BrowserEvent::BrowsingContextOpenResolved {
                request_id, result, ..
            } => {
                info!("browsing context open resolved: request_id={request_id}, result={result:?}");
                Vec::new()
            }
            BrowserEvent::WindowOpenRequested { request, .. } => {
                info!("window open requested: request={request:?}");

                let kind = match request.requested_kind {
                    WindowKind::Popup => WindowKind::Popup,
                    _ => WindowKind::Normal,
                };
                let window = WindowDescriptor {
                    window_id: WindowId::new(request.request_id),
                    kind,
                    state: WindowState::Normal,
                    focused: true,
                    incognito: false,
                    always_on_top: false,
                    bounds: WindowBounds {
                        left: 80,
                        top: 80,
                        width: 1280,
                        height: 900,
                    },
                };

                register_pending_window_open_request(
                    &self.shared,
                    request.request_id,
                    window.window_id,
                );

                if let Err(err) = self.browser_handle().respond_window_open(
                    request.request_id,
                    WindowOpenResponse::AllowNewWindow { window },
                ) {
                    warn!("failed to respond window open request: {err}");
                }

                vec![CoreAction::EnsureHostWindow { window }]
            }
            BrowserEvent::WindowOpenResolved {
                request_id, result, ..
            } => {
                info!("window open resolved: request_id={request_id}, result={result:?}");

                if matches!(result, WindowOpenResult::Denied | WindowOpenResult::Aborted)
                    && let Some(window_id) =
                        take_pending_window_open_request(&self.shared, request_id)
                {
                    return vec![CoreAction::CloseHostWindow { window_id }];
                }

                Vec::new()
            }
            BrowserEvent::WindowOpened { window, .. } => {
                info!("window opened: {window:?}");
                Vec::new()
            }
            BrowserEvent::WindowClosed { window_id, .. } => {
                info!("window closed: {window_id}");
                Vec::new()
            }
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
            BrowserEvent::ExtensionsListed {
                profile_id,
                extensions,
            } => {
                println!("extensions: {profile_id} {extensions:?}");
                Vec::new()
            }
        }
    }

    /// Handles incoming window events from the platform layer.
    pub(crate) fn handle_window_event(
        &mut self,
        window_id: WindowId,
        event: &WindowEvent,
    ) -> Vec<CoreAction> {
        match event {
            WindowEvent::CloseRequested => {
                if let Some(browsing_context_id) =
                    browsing_context_id_for_window(&self.shared, window_id)
                {
                    if let Err(err) = self
                        .browser_handle()
                        .request_close_browsing_context(browsing_context_id)
                    {
                        warn!("failed to request context close on window close: {err}");
                    }
                    Vec::new()
                } else {
                    self.request_shutdown_once();
                    vec![CoreAction::ExitEventLoop]
                }
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                vec![CoreAction::SyncWindowAndResize { window_id }]
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
                if let Some(id) = browsing_context_id_for_window(&self.shared, window_id) {
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
            BrowsingContextEvent::Created { request_id } => {
                if self
                    .browsing_context_id_for_target(ViewTarget::Primary)
                    .is_none()
                {
                    set_primary_browsing_context_id(&self.shared, Some(browsing_context_id));
                }

                let host_window_id = take_pending_window_open_request(&self.shared, request_id)
                    .or_else(|| primary_host_window_id(&self.shared));
                if let Some(host_window_id) = host_window_id {
                    bind_browsing_context_to_window(
                        &self.shared,
                        browsing_context_id,
                        host_window_id,
                    );
                    vec![CoreAction::SyncWindowResizeAndFocus {
                        window_id: host_window_id,
                    }]
                } else {
                    warn!(
                        "created browsing context has no host window mapping: context={}, request_id={request_id}",
                        browsing_context_id
                    );
                    Vec::new()
                }
            }
            BrowsingContextEvent::TitleUpdated { title } => {
                if let Some(window_id) =
                    window_id_for_browsing_context(&self.shared, browsing_context_id)
                {
                    vec![CoreAction::UpdateWindowTitle { window_id, title }]
                } else {
                    Vec::new()
                }
            }
            BrowsingContextEvent::CursorChanged { cursor_type } => {
                if let Some(window_id) =
                    window_id_for_browsing_context(&self.shared, browsing_context_id)
                {
                    vec![CoreAction::UpdateCursor {
                        window_id,
                        cursor: cursor_type,
                    }]
                } else {
                    Vec::new()
                }
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
            BrowsingContextEvent::CloseRequested => {
                if let Err(err) = self
                    .browser_handle()
                    .request_close_browsing_context(browsing_context_id)
                {
                    warn!(
                        "failed to request close for context {}: {err}",
                        browsing_context_id
                    );
                }
                Vec::new()
            }
            BrowsingContextEvent::Closed => {
                let close_action = unbind_browsing_context(&self.shared, browsing_context_id)
                    .map(|window_id| CoreAction::CloseHostWindow { window_id });

                if self.browsing_context_id_for_target(ViewTarget::Primary)
                    == Some(browsing_context_id)
                {
                    set_primary_browsing_context_id(&self.shared, None);
                }
                if self.browsing_context_id_for_target(ViewTarget::DevTools)
                    == Some(browsing_context_id)
                {
                    set_devtools_browsing_context_id(&self.shared, None);
                }

                let mut actions = Vec::new();
                if let Some(action) = close_action {
                    actions.push(action);
                }
                if !has_bound_windows(&self.shared) {
                    self.request_shutdown_once();
                    actions.push(CoreAction::ExitEventLoop);
                }
                actions
            }
            BrowsingContextEvent::AuxiliaryWindowOpenRequested { request_id, kind } => {
                info!("auxiliary open requested: request_id={request_id}, kind={kind:?}");

                // Extension install prompts
                if matches!(kind, AuxiliaryWindowKind::ExtensionInstallPrompt { .. })
                    && self
                        .browser_handle()
                        .open_default_auxiliary_window(browsing_context_id, request_id)
                        .is_err()
                {
                    // Best-effort
                    warn!("failed to open default auxiliary window for request_id={request_id}");

                    self.browser_handle()
                        .respond_auxiliary_window(
                            browsing_context_id,
                            request_id,
                            AuxiliaryWindowResponse::ExtensionInstallPrompt { proceed: false },
                        )
                        .ok();
                };

                Vec::new()
            }
            BrowsingContextEvent::SelectionChanged { .. }
            | BrowsingContextEvent::ScrollPositionChanged { .. }
            | BrowsingContextEvent::NavigationStateChanged { .. }
            | BrowsingContextEvent::FaviconUrlUpdated { .. }
            | BrowsingContextEvent::UpdateTargetUrl { .. }
            | BrowsingContextEvent::FullscreenToggled { .. }
            | BrowsingContextEvent::RenderProcessGone { .. }
            | BrowsingContextEvent::AudioStateChanged { .. }
            | BrowsingContextEvent::DomHtmlRead { .. }
            | BrowsingContextEvent::AuxiliaryWindowResolved { .. }
            | BrowsingContextEvent::ExtensionRuntimeWarning { .. }
            | BrowsingContextEvent::AuxiliaryWindowOpened { .. }
            | BrowsingContextEvent::AuxiliaryWindowClosed { .. } => Vec::new(),
        }
    }
}
