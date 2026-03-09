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
        download::{DownloadId, DownloadOutcome, DownloadPromptActionHint, DownloadState},
        drag::{DragOperations, DragStartRequest},
        extension::{AuxiliaryWindowKind, AuxiliaryWindowResponse, ExtensionInfo},
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
use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use tracing::{error, info, warn};
use winit::event::WindowEvent;

use crate::{app::MenuCommand, cli::Cli};

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
    /// Tracks the most recent page title per host window before download decoration.
    window_base_titles: HashMap<WindowId, String>,
    /// Tracks active and terminal download snapshots for title aggregation and logging.
    downloads: HashMap<DownloadId, DownloadStatus>,
}

#[derive(Debug, Clone)]
struct DownloadStatus {
    source_browsing_context_id: Option<BrowsingContextId>,
    file_name: String,
    received_bytes: u64,
    total_bytes: Option<u64>,
    state: DownloadState,
    is_paused: bool,
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
    SetExtensionsMenuLoading,
    ReplaceExtensionsMenu {
        extensions: Vec<ExtensionInfo>,
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
            window_base_titles: HashMap::new(),
            downloads: HashMap::new(),
        }
    }

    pub(crate) fn browser_handle(&self) -> BrowserHandle<ChromiumBackend> {
        self.session.handle()
    }

    pub(crate) fn handle_menu_command(&mut self, command: MenuCommand) -> Vec<CoreAction> {
        match command {
            MenuCommand::ReloadExtensions => {
                if let Err(err) = self.browser_handle().request_list_extensions(None) {
                    warn!("failed to request extension list: {err}");
                    return Vec::new();
                }
                vec![CoreAction::SetExtensionsMenuLoading]
            }
            MenuCommand::ActivateExtension { extension_id } => {
                info!("extension menu item activated: {extension_id}");
                Vec::new()
            }
        }
    }

    fn base_title_for_window(&self, window_id: WindowId) -> String {
        self.window_base_titles
            .get(&window_id)
            .cloned()
            .unwrap_or_else(|| "CBF SimpleApp".to_string())
    }

    fn is_active_download(state: DownloadState) -> bool {
        matches!(state, DownloadState::InProgress | DownloadState::Paused)
    }

    fn format_progress(received_bytes: u64, total_bytes: Option<u64>) -> Option<String> {
        total_bytes.and_then(|total| {
            if total == 0 {
                None
            } else {
                Some(format!("{}%", received_bytes.saturating_mul(100) / total))
            }
        })
    }

    fn download_title_suffix(&self) -> Option<String> {
        let active: Vec<_> = self
            .downloads
            .values()
            .filter(|download| Self::is_active_download(download.state))
            .collect();
        if active.is_empty() {
            return None;
        }

        if active.len() == 1 {
            let download = active[0];
            let verb = if download.is_paused {
                "Paused"
            } else {
                "Downloading"
            };
            return Some(
                match Self::format_progress(download.received_bytes, download.total_bytes) {
                    Some(progress) => format!("{verb} {progress} - {}", download.file_name),
                    None => format!("{verb} - {}", download.file_name),
                },
            );
        }

        let paused = active.iter().filter(|download| download.is_paused).count();
        Some(if paused > 0 {
            format!("{} downloads active, {} paused", active.len(), paused)
        } else {
            format!("{} downloads active", active.len())
        })
    }

    fn decorated_window_title(&self, window_id: WindowId) -> String {
        let base_title = self.base_title_for_window(window_id);
        match self.download_title_suffix() {
            Some(suffix) => format!("{base_title} - {suffix}"),
            None => base_title,
        }
    }

    fn refresh_primary_window_title(&self) -> Vec<CoreAction> {
        primary_host_window_id(&self.shared)
            .map(|window_id| CoreAction::UpdateWindowTitle {
                window_id,
                title: self.decorated_window_title(window_id),
            })
            .into_iter()
            .collect()
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
                let mut actions = vec![CoreAction::SetExtensionsMenuLoading];

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

                if let Err(err) = self.browser_handle().request_list_extensions(None) {
                    warn!("failed to request extension list on startup: {err}");
                    actions.clear();
                }

                actions
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
                profile_id: _,
                extensions,
            } => {
                vec![CoreAction::ReplaceExtensionsMenu { extensions }]
            }
            BrowserEvent::DownloadCreated {
                download_id,
                source_browsing_context_id,
                file_name,
                total_bytes,
                target_path,
                ..
            } => {
                info!(
                    "download created: id={download_id:?}, source={source_browsing_context_id:?}, file={file_name}, total_bytes={total_bytes:?}, target_path={target_path:?}"
                );
                self.downloads.insert(
                    download_id,
                    DownloadStatus {
                        source_browsing_context_id,
                        file_name,
                        received_bytes: 0,
                        total_bytes,
                        state: DownloadState::InProgress,
                        is_paused: false,
                    },
                );
                self.refresh_primary_window_title()
            }
            BrowserEvent::DownloadUpdated {
                download_id,
                source_browsing_context_id,
                state,
                file_name,
                received_bytes,
                total_bytes,
                target_path: _,
                can_resume,
                is_paused,
                ..
            } => {
                info!(
                    "download updated: id={download_id:?}, state={state:?}, file={file_name}, received={received_bytes}, total={total_bytes:?}, paused={is_paused}, resumable={can_resume}"
                );
                self.downloads.insert(
                    download_id,
                    DownloadStatus {
                        source_browsing_context_id,
                        file_name,
                        received_bytes,
                        total_bytes,
                        state,
                        is_paused,
                    },
                );
                self.refresh_primary_window_title()
            }
            BrowserEvent::DownloadCompleted {
                download_id,
                source_browsing_context_id,
                outcome,
                file_name,
                received_bytes,
                total_bytes,
                target_path,
                ..
            } => {
                info!(
                    "download completed: id={download_id:?}, outcome={outcome:?}, file={file_name}, received={received_bytes}, total={total_bytes:?}, target_path={target_path:?}"
                );
                self.downloads.insert(
                    download_id,
                    DownloadStatus {
                        source_browsing_context_id,
                        file_name,
                        received_bytes,
                        total_bytes,
                        state: match outcome {
                            DownloadOutcome::Succeeded => DownloadState::Completed,
                            DownloadOutcome::Cancelled => DownloadState::Cancelled,
                            DownloadOutcome::Interrupted => DownloadState::Interrupted,
                            DownloadOutcome::Unknown => DownloadState::Unknown,
                        },
                        is_paused: false,
                    },
                );
                self.refresh_primary_window_title()
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
                    self.window_base_titles.insert(window_id, title);
                    vec![CoreAction::UpdateWindowTitle {
                        window_id,
                        title: self.decorated_window_title(window_id),
                    }]
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
                self.downloads.retain(|_, download| {
                    download.source_browsing_context_id != Some(browsing_context_id)
                        || !Self::is_active_download(download.state)
                });

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
                if let Some(CoreAction::CloseHostWindow { window_id }) = close_action {
                    self.window_base_titles.remove(&window_id);
                    actions.push(CoreAction::CloseHostWindow { window_id });
                }
                actions.extend(self.refresh_primary_window_title());
                if !has_bound_windows(&self.shared) {
                    self.request_shutdown_once();
                    actions.push(CoreAction::ExitEventLoop);
                }
                actions
            }
            BrowsingContextEvent::AuxiliaryWindowOpenRequested { request_id, kind } => {
                info!("auxiliary open requested: request_id={request_id}, kind={kind:?}");

                if let AuxiliaryWindowKind::PermissionPrompt { permission } = &kind {
                    let allow = show_permission_prompt_dialog(permission);
                    if let Err(err) = self.browser_handle().respond_auxiliary_window(
                        browsing_context_id,
                        request_id,
                        AuxiliaryWindowResponse::PermissionPrompt { allow },
                    ) {
                        warn!("failed to respond permission prompt request_id={request_id}: {err}");
                    }
                    return Vec::new();
                }

                if let AuxiliaryWindowKind::DownloadPrompt {
                    download_id,
                    file_name,
                    total_bytes: _,
                    suggested_path,
                    action_hint,
                } = &kind
                {
                    let response = download_prompt_response_for_simpleapp(
                        *action_hint,
                        file_name,
                        suggested_path,
                        self.cli.download_dir.as_deref(),
                    );
                    if let Err(err) = self.browser_handle().respond_auxiliary_window(
                        browsing_context_id,
                        request_id,
                        response,
                    ) {
                        warn!(
                            "failed to respond download prompt request_id={request_id}, download_id={download_id:?}: {err}"
                        );
                    }
                    return Vec::new();
                }

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
            BrowsingContextEvent::AuxiliaryWindowResolved {
                request_id,
                resolution,
            } => {
                info!("auxiliary resolved: request_id={request_id}, resolution={resolution:?}");
                Vec::new()
            }
            BrowsingContextEvent::AuxiliaryWindowOpened {
                window_id,
                kind,
                title,
                modal,
            } => {
                info!(
                    "auxiliary opened: window_id={window_id:?}, kind={kind:?}, title={title:?}, modal={modal}"
                );
                Vec::new()
            }
            BrowsingContextEvent::AuxiliaryWindowClosed {
                window_id,
                kind,
                reason,
            } => {
                info!(
                    "auxiliary closed: window_id={window_id:?}, kind={kind:?}, reason={reason:?}"
                );
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
            | BrowsingContextEvent::ExtensionRuntimeWarning { .. } => Vec::new(),
        }
    }
}

fn show_permission_prompt_dialog(permission: &cbf::data::extension::PermissionPromptType) -> bool {
    let message = format!(
        "{}\n\nAllow this request?",
        permission_prompt_description(permission)
    );

    let result = MessageDialog::new()
        .set_level(MessageLevel::Info)
        .set_title("Permission Request")
        .set_description(&message)
        .set_buttons(MessageButtons::YesNo)
        .show();

    matches!(result, MessageDialogResult::Yes)
}

fn show_download_save_as_dialog(
    file_name: &str,
    suggested_path: &Option<String>,
) -> AuxiliaryWindowResponse {
    let mut dialog = FileDialog::new().set_file_name(file_name);
    if let Some(suggested_path) = suggested_path {
        let suggested = std::path::Path::new(suggested_path);
        if let Some(parent) = suggested.parent() {
            dialog = dialog.set_directory(parent);
        }
        if let Some(name) = suggested.file_name().and_then(|name| name.to_str()) {
            dialog = dialog.set_file_name(name);
        }
    }

    match dialog.save_file() {
        Some(path) => AuxiliaryWindowResponse::DownloadPrompt {
            allow: true,
            destination_path: Some(path.to_string_lossy().into_owned()),
        },
        None => AuxiliaryWindowResponse::DownloadPrompt {
            allow: false,
            destination_path: None,
        },
    }
}

fn show_download_blocked_dialog() {
    let _ = MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title("Download Blocked")
        .set_description("This download is blocked by policy and cannot be saved.")
        .set_buttons(MessageButtons::Ok)
        .show();
}

fn build_default_download_destination_path(
    download_dir: Option<&std::path::Path>,
    file_name: &str,
) -> Option<String> {
    let file_name = file_name.trim();
    if file_name.is_empty() {
        return None;
    }
    download_dir.map(|dir| dir.join(file_name).to_string_lossy().into_owned())
}

fn download_prompt_response_for_simpleapp(
    action_hint: DownloadPromptActionHint,
    file_name: &str,
    suggested_path: &Option<String>,
    default_download_dir: Option<&std::path::Path>,
) -> AuxiliaryWindowResponse {
    match download_prompt_handling(action_hint) {
        DownloadPromptHandling::ImmediateAllow => {
            if prompt_dialog() {
                AuxiliaryWindowResponse::DownloadPrompt {
                    allow: true,
                    destination_path: build_default_download_destination_path(
                        default_download_dir,
                        file_name,
                    ),
                }
            } else {
                AuxiliaryWindowResponse::DownloadPrompt {
                    allow: false,
                    destination_path: None,
                }
            }
        }
        DownloadPromptHandling::ShowSaveDialog => {
            show_download_save_as_dialog(file_name, suggested_path)
        }
        DownloadPromptHandling::DenyBlocked => {
            show_download_blocked_dialog();
            AuxiliaryWindowResponse::DownloadPrompt {
                allow: false,
                destination_path: None,
            }
        }
    }
}

fn prompt_dialog() -> bool {
    let result = MessageDialog::new()
        .set_level(MessageLevel::Info)
        .set_title("Download Request")
        .set_description("This site wants to download a file. Allow this request?")
        .set_buttons(MessageButtons::YesNo)
        .show();

    result == MessageDialogResult::Yes
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DownloadPromptHandling {
    ImmediateAllow,
    ShowSaveDialog,
    DenyBlocked,
}

fn download_prompt_handling(action_hint: DownloadPromptActionHint) -> DownloadPromptHandling {
    match action_hint {
        DownloadPromptActionHint::AutoSave => DownloadPromptHandling::ImmediateAllow,
        DownloadPromptActionHint::SelectDestination | DownloadPromptActionHint::Unknown => {
            DownloadPromptHandling::ShowSaveDialog
        }
        DownloadPromptActionHint::Deny => DownloadPromptHandling::DenyBlocked,
    }
}

fn permission_prompt_description(
    permission: &cbf::data::extension::PermissionPromptType,
) -> String {
    use cbf::data::extension::PermissionPromptType;

    match permission {
        PermissionPromptType::Geolocation => "This site wants to access your location.".to_string(),
        PermissionPromptType::Notifications => "This site wants to show notifications.".to_string(),
        PermissionPromptType::AudioCapture => "This site wants to use your microphone.".to_string(),
        PermissionPromptType::VideoCapture => "This site wants to use your camera.".to_string(),
        PermissionPromptType::Other(name) => format!("This site requests permission: {name}."),
        PermissionPromptType::Unknown => {
            "This site requests a permission that could not be identified.".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_default_download_destination_path_joins_file_name() {
        let dir = std::path::Path::new("/tmp/downloads");
        let path =
            build_default_download_destination_path(Some(dir), "file.bin").expect("path expected");

        assert_eq!(path, "/tmp/downloads/file.bin");
    }

    #[test]
    fn build_default_download_destination_path_returns_none_without_dir() {
        assert_eq!(
            build_default_download_destination_path(None, "file.bin"),
            None
        );
    }

    #[test]
    fn download_prompt_handling_none_is_immediate_allow() {
        assert_eq!(
            download_prompt_handling(DownloadPromptActionHint::AutoSave),
            DownloadPromptHandling::ImmediateAllow
        );
    }

    #[test]
    fn download_prompt_handling_dlp_is_denied() {
        assert_eq!(
            download_prompt_handling(DownloadPromptActionHint::Deny),
            DownloadPromptHandling::DenyBlocked
        );
    }

    #[test]
    fn download_prompt_handling_save_as_shows_dialog() {
        assert_eq!(
            download_prompt_handling(DownloadPromptActionHint::SelectDestination),
            DownloadPromptHandling::ShowSaveDialog
        );
    }

    #[test]
    fn download_prompt_handling_unknown_shows_dialog() {
        assert_eq!(
            download_prompt_handling(DownloadPromptActionHint::Unknown),
            DownloadPromptHandling::ShowSaveDialog
        );
    }
}
