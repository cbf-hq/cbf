use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use cbf::{
    browser::{BrowserHandle, BrowserSession},
    data::{
        auxiliary_window::{AuxiliaryWindowKind, AuxiliaryWindowResponse, PermissionPromptType},
        browsing_context_open::BrowsingContextOpenHint,
        browsing_context_open::BrowsingContextOpenResponse,
        context_menu::ContextMenu,
        dialog::{BeforeUnloadReason, DialogResponse, DialogType},
        download::{DownloadId, DownloadOutcome, DownloadPromptActionHint, DownloadState},
        drag::{DragOperations, DragStartRequest},
        extension::ExtensionInfo,
        ids::{BrowsingContextId, TransientBrowsingContextId, WindowId},
        ime::ImeBoundsUpdate,
        profile::ProfileInfo,
        transient_browsing_context::TransientBrowsingContextKind,
        window_open::{
            WindowBounds, WindowDescriptor, WindowKind, WindowOpenResponse, WindowOpenResult,
            WindowState,
        },
    },
    event::{BackendStopReason, BrowserEvent, BrowsingContextEvent, TransientBrowsingContextEvent},
};
use cbf_chrome::{
    backend::ChromiumBackend,
    browser::ChromiumBrowserHandleExt,
    data::{choice_menu::ChromeChoiceMenu, surface::SurfaceHandle},
};
use cursor_icon::CursorIcon;
use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use tracing::{debug, error, info, warn};
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
    /// Maps transient browsing contexts to host window IDs.
    pub(crate) transient_to_window: HashMap<TransientBrowsingContextId, WindowId>,
    /// Maps host window IDs to transient browsing contexts.
    pub(crate) window_to_transient: HashMap<WindowId, TransientBrowsingContextId>,
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
    !guard.window_to_browsing_context.is_empty() || !guard.window_to_transient.is_empty()
}

pub(crate) fn bind_transient_to_window(
    shared: &Arc<Mutex<SharedState>>,
    transient_browsing_context_id: TransientBrowsingContextId,
    window_id: WindowId,
) {
    let mut guard = shared.lock().expect("shared state lock poisoned");
    guard
        .transient_to_window
        .insert(transient_browsing_context_id, window_id);
    guard
        .window_to_transient
        .insert(window_id, transient_browsing_context_id);
}

pub(crate) fn unbind_transient_browsing_context(
    shared: &Arc<Mutex<SharedState>>,
    transient_browsing_context_id: TransientBrowsingContextId,
) -> Option<WindowId> {
    let mut guard = shared.lock().expect("shared state lock poisoned");
    let window_id = guard
        .transient_to_window
        .remove(&transient_browsing_context_id);
    if let Some(window_id) = window_id {
        guard.window_to_transient.remove(&window_id);
    }
    window_id
}

pub(crate) fn transient_browsing_context_id_for_window(
    shared: &Arc<Mutex<SharedState>>,
    window_id: WindowId,
) -> Option<TransientBrowsingContextId> {
    let guard = shared.lock().expect("shared state lock poisoned");
    guard.window_to_transient.get(&window_id).copied()
}

pub(crate) fn window_id_for_transient_browsing_context(
    shared: &Arc<Mutex<SharedState>>,
    transient_browsing_context_id: TransientBrowsingContextId,
) -> Option<WindowId> {
    let guard = shared.lock().expect("shared state lock poisoned");
    guard
        .transient_to_window
        .get(&transient_browsing_context_id)
        .copied()
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
    /// Tracks transient popup metadata until a surface is available.
    transient_popups: HashMap<TransientBrowsingContextId, TransientPopupState>,
    /// Holds popup sizes that arrive before the generic opened event.
    pending_transient_popup_sizes: HashMap<TransientBrowsingContextId, (u32, u32)>,
    /// Popup blur-close activates only after the popup was focused once.
    blur_close_armed_transients: HashSet<TransientBrowsingContextId>,
    /// Canonical profile id selected from `ProfilesListed`.
    resolved_profile_id: Option<String>,
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

#[derive(Debug, Clone)]
struct TransientPopupState {
    parent_browsing_context_id: BrowsingContextId,
    title: String,
    /// The last preferred size that was actually applied for the popup host.
    size: Option<(u32, u32)>,
    /// The size applied one step before `size`, used to detect A→B→A oscillation.
    prev_sent_size: Option<(u32, u32)>,
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
    EnsureTransientHostWindow {
        transient_browsing_context_id: TransientBrowsingContextId,
        title: String,
        width: u32,
        height: u32,
    },
    ResizeTransientBrowsingContext {
        transient_browsing_context_id: TransientBrowsingContextId,
        width: u32,
        height: u32,
    },
    CloseTransientHostWindow {
        transient_browsing_context_id: TransientBrowsingContextId,
    },
    ApplyTransientSurfaceHandle {
        transient_browsing_context_id: TransientBrowsingContextId,
        handle: SurfaceHandle,
    },
    ApplyImeBounds {
        browsing_context_id: BrowsingContextId,
        update: ImeBoundsUpdate,
    },
    ApplyTransientImeBounds {
        transient_browsing_context_id: TransientBrowsingContextId,
        update: ImeBoundsUpdate,
    },
    ShowContextMenu {
        browsing_context_id: BrowsingContextId,
        menu: ContextMenu,
    },
    ShowContextMenuInTransientBrowsingContext {
        transient_browsing_context_id: TransientBrowsingContextId,
        menu: ContextMenu,
    },
    ShowChoiceMenu {
        browsing_context_id: BrowsingContextId,
        menu: ChromeChoiceMenu,
    },
    ShowChoiceMenuInTransientBrowsingContext {
        transient_browsing_context_id: TransientBrowsingContextId,
        menu: ChromeChoiceMenu,
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
            transient_popups: HashMap::new(),
            pending_transient_popup_sizes: HashMap::new(),
            blur_close_armed_transients: HashSet::new(),
            resolved_profile_id: None,
        }
    }

    pub(crate) fn browser_handle(&self) -> BrowserHandle<ChromiumBackend> {
        self.session.handle()
    }

    pub(crate) fn handle_menu_command(&mut self, command: MenuCommand) -> Vec<CoreAction> {
        match command {
            MenuCommand::ReloadExtensions => {
                let Some(profile_id) = self.resolved_profile_id.as_deref() else {
                    warn!("ignoring extension reload before a canonical profile is resolved");
                    return Vec::new();
                };
                if let Err(err) = self.browser_handle().request_list_extensions(profile_id) {
                    warn!("failed to request extension list: {err}");
                    return Vec::new();
                }
                vec![CoreAction::SetExtensionsMenuLoading]
            }
            MenuCommand::ActivateExtension { extension_id } => {
                info!("extension menu item activated: {extension_id}");
                let Some(browsing_context_id) =
                    browsing_context_id_for_target(&self.shared, ViewTarget::Primary)
                else {
                    warn!("ignoring extension activation without a primary browsing context");
                    return Vec::new();
                };
                if let Err(err) = self
                    .browser_handle()
                    .activate_extension_action(browsing_context_id, extension_id)
                {
                    warn!("failed to activate extension action: {err}");
                }
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

    fn resolve_startup_profile_id(profiles: &[ProfileInfo]) -> Option<String> {
        profiles
            .iter()
            .find(|profile| profile.is_default)
            .or_else(|| profiles.first())
            .map(|profile| profile.profile_id.clone())
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

    pub(crate) fn handle_extension_popup_surface_update(
        &mut self,
        transient_browsing_context_id: TransientBrowsingContextId,
        parent_browsing_context_id: BrowsingContextId,
        handle: SurfaceHandle,
    ) -> Vec<CoreAction> {
        let Some(state) = self
            .transient_popups
            .get(&transient_browsing_context_id)
            .cloned()
        else {
            warn!(
                "ignoring popup surface update for unknown transient browsing context {}",
                transient_browsing_context_id
            );
            return Vec::new();
        };

        if state.parent_browsing_context_id != parent_browsing_context_id {
            warn!(
                "popup parent mismatch for transient {}: expected {}, got {}",
                transient_browsing_context_id,
                state.parent_browsing_context_id,
                parent_browsing_context_id
            );
        }

        let mut actions = Vec::new();
        if window_id_for_transient_browsing_context(&self.shared, transient_browsing_context_id)
            .is_none()
        {
            let (width, height) = state.size.unwrap_or((420, 600));
            actions.push(CoreAction::EnsureTransientHostWindow {
                transient_browsing_context_id,
                title: state.title.clone(),
                width,
                height,
            });
        }
        actions.push(CoreAction::ApplyTransientSurfaceHandle {
            transient_browsing_context_id,
            handle,
        });
        actions
    }

    pub(crate) fn handle_extension_popup_preferred_size_update(
        &mut self,
        transient_browsing_context_id: TransientBrowsingContextId,
        parent_browsing_context_id: BrowsingContextId,
        width: u32,
        height: u32,
    ) -> Vec<CoreAction> {
        let Some(state) = self
            .transient_popups
            .get_mut(&transient_browsing_context_id)
        else {
            self.pending_transient_popup_sizes
                .insert(transient_browsing_context_id, (width, height));
            return Vec::new();
        };

        if state.parent_browsing_context_id != parent_browsing_context_id {
            warn!(
                "popup parent mismatch for transient {}: expected {}, got {}",
                transient_browsing_context_id,
                state.parent_browsing_context_id,
                parent_browsing_context_id
            );
        }

        let new_size = (width, height);
        let size_changed = state.size != Some(new_size);
        // Detect A→B→A oscillation: the incoming size equals the size sent two steps ago.
        // When this happens, suppress the resize and leave state.size unchanged so that
        // repeated oscillating preferred-size events keep being suppressed.
        let is_oscillating = size_changed && state.prev_sent_size == Some(new_size);
        let is_bound =
            window_id_for_transient_browsing_context(&self.shared, transient_browsing_context_id)
                .is_some();
        debug!(
            transient_browsing_context_id = %transient_browsing_context_id,
            parent_browsing_context_id = %parent_browsing_context_id,
            width,
            height,
            is_bound,
            size_changed,
            is_oscillating,
            "processing extension popup preferred size update"
        );

        if is_oscillating {
            return Vec::new();
        }

        let prev_size = state.size;
        state.size = Some(new_size);

        if is_bound && size_changed {
            state.prev_sent_size = prev_size;
            vec![CoreAction::ResizeTransientBrowsingContext {
                transient_browsing_context_id,
                width,
                height,
            }]
        } else {
            Vec::new()
        }
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
                if let Err(err) = self.browser_handle().request_list_profiles() {
                    error!("failed to request profile list on startup: {err}");
                    return vec![CoreAction::ExitEventLoop];
                }
                vec![CoreAction::SetExtensionsMenuLoading]
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
            BrowserEvent::TransientBrowsingContext {
                transient_browsing_context_id,
                parent_browsing_context_id,
                event,
                ..
            } => self.handle_transient_browsing_context_event(
                transient_browsing_context_id,
                parent_browsing_context_id,
                *event,
            ),
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
            BrowserEvent::AuxiliaryWindowOpenRequested {
                profile_id,
                request_id,
                source_browsing_context_id,
                kind,
            } => {
                info!(
                    "auxiliary open requested: profile_id={profile_id}, request_id={request_id}, source={source_browsing_context_id:?}, kind={kind:?}"
                );

                if let AuxiliaryWindowKind::PermissionPrompt { permission } = &kind {
                    let allow = show_permission_prompt_dialog(permission);
                    if let Err(err) = self.browser_handle().respond_auxiliary_window(
                        profile_id,
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
                        profile_id,
                        request_id,
                        response,
                    ) {
                        warn!(
                            "failed to respond download prompt request_id={request_id}, download_id={download_id:?}: {err}"
                        );
                    }
                    return Vec::new();
                }

                if let AuxiliaryWindowKind::ExtensionInstallPrompt {
                    extension_name,
                    permission_names,
                    ..
                } = &kind
                {
                    let proceed =
                        show_extension_install_prompt_dialog(extension_name, permission_names);
                    if let Err(err) = self.browser_handle().respond_auxiliary_window(
                        profile_id,
                        request_id,
                        AuxiliaryWindowResponse::ExtensionInstallPrompt { proceed },
                    ) {
                        warn!(
                            "failed to respond extension install prompt request_id={request_id}: {err}"
                        );
                    }
                    return Vec::new();
                }

                if let AuxiliaryWindowKind::ExtensionUninstallPrompt {
                    extension_name,
                    triggering_extension_name,
                    can_report_abuse,
                    ..
                } = &kind
                {
                    let response = show_extension_uninstall_prompt_dialog(
                        extension_name,
                        triggering_extension_name.as_deref(),
                        *can_report_abuse,
                    );
                    if let Err(err) = self.browser_handle().respond_auxiliary_window(
                        profile_id,
                        request_id,
                        response,
                    ) {
                        warn!(
                            "failed to respond extension uninstall prompt request_id={request_id}: {err}"
                        );
                    }
                    return Vec::new();
                }

                Vec::new()
            }
            BrowserEvent::AuxiliaryWindowResolved {
                profile_id,
                request_id,
                source_browsing_context_id,
                resolution,
            } => {
                info!(
                    "auxiliary resolved: profile_id={profile_id}, request_id={request_id}, source={source_browsing_context_id:?}, resolution={resolution:?}"
                );
                Vec::new()
            }
            BrowserEvent::AuxiliaryWindowOpened {
                profile_id,
                source_browsing_context_id,
                window_id,
                kind,
                title,
                modal,
            } => {
                info!(
                    "auxiliary opened: profile_id={profile_id}, source={source_browsing_context_id:?}, window_id={window_id:?}, kind={kind:?}, title={title:?}, modal={modal}"
                );
                Vec::new()
            }
            BrowserEvent::AuxiliaryWindowClosed {
                profile_id,
                source_browsing_context_id,
                window_id,
                kind,
                reason,
            } => {
                info!(
                    "auxiliary closed: profile_id={profile_id}, source={source_browsing_context_id:?}, window_id={window_id:?}, kind={kind:?}, reason={reason:?}"
                );
                Vec::new()
            }
            BrowserEvent::ProfilesListed { profiles } => {
                let Some(profile_id) = Self::resolve_startup_profile_id(&profiles) else {
                    error!("backend returned no profiles; exiting without issuing profile-scoped commands");
                    return vec![CoreAction::ExitEventLoop];
                };

                if let Some(default_profile) = profiles.iter().find(|profile| profile.is_default) {
                    info!(
                        profile_id = default_profile.profile_id,
                        display_name = default_profile.display_name,
                        "resolved default startup profile"
                    );
                } else if let Some(first_profile) = profiles.first() {
                    warn!(
                        profile_id = first_profile.profile_id,
                        display_name = first_profile.display_name,
                        "backend did not mark a default profile; falling back to the first listed profile"
                    );
                }

                self.resolved_profile_id = Some(profile_id.clone());

                if !self.page_create_requested {
                    self.page_create_requested = true;
                    if let Err(err) = self.browser_handle().create_browsing_context(
                        1,
                        Some(self.cli.url.clone()),
                        profile_id.clone(),
                    ) {
                        error!("failed to create browsing context: {err}");
                        return vec![CoreAction::ExitEventLoop];
                    }
                }

                if let Err(err) = self.browser_handle().request_list_extensions(profile_id) {
                    warn!("failed to request extension list on startup: {err}");
                    return Vec::new();
                }

                Vec::new()
            }
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
                if let Some(transient_browsing_context_id) =
                    transient_browsing_context_id_for_window(&self.shared, window_id)
                {
                    if let Err(err) = self
                        .browser_handle()
                        .close_transient_browsing_context(transient_browsing_context_id)
                    {
                        warn!("failed to close transient browsing context: {err}");
                        return vec![CoreAction::CloseTransientHostWindow {
                            transient_browsing_context_id,
                        }];
                    }
                    return Vec::new();
                }
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
                debug!(window_id = %window_id, event = ?event, "received host window resize event");
                if transient_browsing_context_id_for_window(&self.shared, window_id).is_some() {
                    return vec![CoreAction::SyncWindowAndResize { window_id }];
                }
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
                if let Some(transient_browsing_context_id) =
                    transient_browsing_context_id_for_window(&self.shared, window_id)
                {
                    if *focused {
                        self.blur_close_armed_transients
                            .insert(transient_browsing_context_id);
                        if let Err(err) =
                            self.browser_handle().set_transient_browsing_context_focus(
                                transient_browsing_context_id,
                                true,
                            )
                        {
                            warn!("failed to focus transient browsing context: {err}");
                        }
                    } else if self
                        .blur_close_armed_transients
                        .contains(&transient_browsing_context_id)
                        && let Err(err) = self
                            .browser_handle()
                            .close_transient_browsing_context(transient_browsing_context_id)
                    {
                        warn!("failed to close blurred transient browsing context: {err}");
                        return vec![CoreAction::CloseTransientHostWindow {
                            transient_browsing_context_id,
                        }];
                    }
                    return Vec::new();
                }
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
            BrowsingContextEvent::ChoiceMenuRequested { .. } => Vec::new(),
            BrowsingContextEvent::DragStartRequested { request } => {
                vec![CoreAction::StartPlatformDrag(request)]
            }
            BrowsingContextEvent::JavaScriptDialogRequested {
                request_id,
                message,
                default_prompt_text,
                r#type,
                beforeunload_reason,
            } => {
                let response = show_javascript_dialog(
                    r#type,
                    &message,
                    default_prompt_text.as_deref(),
                    beforeunload_reason.as_ref(),
                );
                if let Err(err) = respond_javascript_dialog_for_browsing_context(
                    self.browser_handle(),
                    browsing_context_id,
                    request_id,
                    r#type,
                    response,
                ) {
                    warn!("failed to respond javascript dialog: {err}");
                }
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

    fn handle_transient_browsing_context_event(
        &mut self,
        transient_browsing_context_id: TransientBrowsingContextId,
        parent_browsing_context_id: BrowsingContextId,
        event: TransientBrowsingContextEvent,
    ) -> Vec<CoreAction> {
        match event {
            TransientBrowsingContextEvent::Opened { kind, title } => {
                if kind != TransientBrowsingContextKind::Popup {
                    return Vec::new();
                }
                self.transient_popups.insert(
                    transient_browsing_context_id,
                    TransientPopupState {
                        parent_browsing_context_id,
                        title: title.unwrap_or_else(|| "Extension Popup".to_string()),
                        size: self
                            .pending_transient_popup_sizes
                            .remove(&transient_browsing_context_id),
                        prev_sent_size: None,
                    },
                );
                Vec::new()
            }
            TransientBrowsingContextEvent::Closed { .. } => {
                self.transient_popups.remove(&transient_browsing_context_id);
                self.pending_transient_popup_sizes
                    .remove(&transient_browsing_context_id);
                self.blur_close_armed_transients
                    .remove(&transient_browsing_context_id);
                unbind_transient_browsing_context(&self.shared, transient_browsing_context_id)
                    .map(|_| CoreAction::CloseTransientHostWindow {
                        transient_browsing_context_id,
                    })
                    .into_iter()
                    .collect()
            }
            TransientBrowsingContextEvent::Focused => Vec::new(),
            TransientBrowsingContextEvent::Blurred => Vec::new(),
            TransientBrowsingContextEvent::Resized { width, height } => self
                .handle_extension_popup_preferred_size_update(
                    transient_browsing_context_id,
                    parent_browsing_context_id,
                    width,
                    height,
                ),
            TransientBrowsingContextEvent::ImeBoundsUpdated { update } => {
                vec![CoreAction::ApplyTransientImeBounds {
                    transient_browsing_context_id,
                    update,
                }]
            }
            TransientBrowsingContextEvent::CursorChanged { cursor_type } => {
                if let Some(window_id) = window_id_for_transient_browsing_context(
                    &self.shared,
                    transient_browsing_context_id,
                ) {
                    vec![CoreAction::UpdateCursor {
                        window_id,
                        cursor: cursor_type,
                    }]
                } else {
                    Vec::new()
                }
            }
            TransientBrowsingContextEvent::ContextMenuRequested { menu } => {
                vec![CoreAction::ShowContextMenuInTransientBrowsingContext {
                    transient_browsing_context_id,
                    menu,
                }]
            }
            TransientBrowsingContextEvent::ChoiceMenuRequested { .. } => Vec::new(),
            TransientBrowsingContextEvent::JavaScriptDialogRequested {
                request_id,
                message,
                default_prompt_text,
                r#type,
                beforeunload_reason,
            } => {
                let response = show_javascript_dialog(
                    r#type,
                    &message,
                    default_prompt_text.as_deref(),
                    beforeunload_reason.as_ref(),
                );
                if let Err(err) = self
                    .browser_handle()
                    .respond_javascript_dialog_in_transient_browsing_context(
                        transient_browsing_context_id,
                        request_id,
                        response,
                    )
                {
                    warn!("failed to respond transient javascript dialog: {err}");
                }
                Vec::new()
            }
            TransientBrowsingContextEvent::TitleUpdated { title } => {
                if let Some(state) = self
                    .transient_popups
                    .get_mut(&transient_browsing_context_id)
                {
                    state.title = title.clone();
                }
                window_id_for_transient_browsing_context(
                    &self.shared,
                    transient_browsing_context_id,
                )
                .map(|window_id| CoreAction::UpdateWindowTitle { window_id, title })
                .into_iter()
                .collect()
            }
            TransientBrowsingContextEvent::CloseRequested => {
                if let Err(err) = self
                    .browser_handle()
                    .close_transient_browsing_context(transient_browsing_context_id)
                {
                    warn!(
                        "failed to close transient browsing context {}: {err}",
                        transient_browsing_context_id
                    );
                    return vec![CoreAction::CloseTransientHostWindow {
                        transient_browsing_context_id,
                    }];
                }
                Vec::new()
            }
            TransientBrowsingContextEvent::RenderProcessGone { crashed } => {
                warn!(
                    transient_browsing_context_id = %transient_browsing_context_id,
                    crashed,
                    "transient renderer exited"
                );
                vec![CoreAction::CloseTransientHostWindow {
                    transient_browsing_context_id,
                }]
            }
        }
    }
}

fn respond_javascript_dialog_for_browsing_context(
    browser: BrowserHandle<ChromiumBackend>,
    browsing_context_id: BrowsingContextId,
    request_id: u64,
    dialog_type: DialogType,
    response: DialogResponse,
) -> Result<(), cbf::error::Error> {
    if dialog_type == DialogType::BeforeUnload {
        return browser.confirm_beforeunload(
            browsing_context_id,
            request_id,
            matches!(response, DialogResponse::Success { .. }),
        );
    }

    browser.respond_javascript_dialog(browsing_context_id, request_id, response)
}

fn show_javascript_dialog(
    dialog_type: DialogType,
    message: &str,
    default_prompt_text: Option<&str>,
    beforeunload_reason: Option<&BeforeUnloadReason>,
) -> DialogResponse {
    match dialog_type {
        DialogType::Alert => {
            let _ = MessageDialog::new()
                .set_level(MessageLevel::Info)
                .set_title("JavaScript Alert")
                .set_description(message)
                .set_buttons(MessageButtons::Ok)
                .show();
            DialogResponse::Success { input: None }
        }
        DialogType::Confirm => {
            let confirmed = MessageDialog::new()
                .set_level(MessageLevel::Info)
                .set_title("JavaScript Confirm")
                .set_description(message)
                .set_buttons(MessageButtons::YesNo)
                .show();
            if confirmed == MessageDialogResult::Yes {
                DialogResponse::Success { input: None }
            } else {
                DialogResponse::Cancel
            }
        }
        DialogType::Prompt => show_prompt_dialog(message, default_prompt_text),
        DialogType::BeforeUnload => {
            let reason_suffix = beforeunload_reason
                .map(beforeunload_reason_description)
                .unwrap_or("The page requested confirmation before closing.");
            let description = format!("{message}\n\n{reason_suffix}");
            let confirmed = MessageDialog::new()
                .set_level(MessageLevel::Warning)
                .set_title("Leave Page?")
                .set_description(&description)
                .set_buttons(MessageButtons::YesNo)
                .show();
            if confirmed == MessageDialogResult::Yes {
                DialogResponse::Success { input: None }
            } else {
                DialogResponse::Cancel
            }
        }
    }
}

fn beforeunload_reason_description(reason: &BeforeUnloadReason) -> &'static str {
    match reason {
        BeforeUnloadReason::CloseBrowsingContext => {
            "Closing this page may discard unsaved changes."
        }
        BeforeUnloadReason::Navigate => "Navigating away may discard unsaved changes.",
        BeforeUnloadReason::Reload => "Reloading may discard unsaved changes.",
        BeforeUnloadReason::WindowClose => "Closing the window may discard unsaved changes.",
        BeforeUnloadReason::Unknown => "The page requested confirmation before closing.",
    }
}

#[cfg(target_os = "macos")]
fn show_prompt_dialog(message: &str, default_prompt_text: Option<&str>) -> DialogResponse {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSAlert, NSAlertFirstButtonReturn, NSAlertStyle, NSTextField};
    use objc2_core_foundation::{CGPoint, CGRect, CGSize};
    use objc2_foundation::NSString;

    let Some(mtm) = MainThreadMarker::new() else {
        return DialogResponse::Cancel;
    };

    let alert = NSAlert::new(mtm);
    let message_text = NSString::from_str("JavaScript Prompt");
    let informative_text = NSString::from_str(message);
    let ok = NSString::from_str("OK");
    let cancel = NSString::from_str("Cancel");
    let initial = NSString::from_str(default_prompt_text.unwrap_or_default());

    alert.setMessageText(&message_text);
    alert.setInformativeText(&informative_text);
    alert.setAlertStyle(NSAlertStyle::Informational);
    alert.addButtonWithTitle(&ok);
    alert.addButtonWithTitle(&cancel);

    let input = NSTextField::textFieldWithString(&initial, mtm);
    input.setFrame(CGRect::new(
        CGPoint::new(0.0, 0.0),
        CGSize::new(320.0, 24.0),
    ));
    alert.setAccessoryView(Some(&input));

    if alert.runModal() == NSAlertFirstButtonReturn {
        DialogResponse::Success {
            input: Some(input.stringValue().to_string()),
        }
    } else {
        DialogResponse::Cancel
    }
}

#[cfg(not(target_os = "macos"))]
fn show_prompt_dialog(message: &str, default_prompt_text: Option<&str>) -> DialogResponse {
    let confirmed = MessageDialog::new()
        .set_level(MessageLevel::Info)
        .set_title("JavaScript Prompt")
        .set_description(message)
        .set_buttons(MessageButtons::YesNo)
        .show();
    if confirmed == MessageDialogResult::Yes {
        DialogResponse::Success {
            input: default_prompt_text.map(ToOwned::to_owned),
        }
    } else {
        DialogResponse::Cancel
    }
}

fn show_permission_prompt_dialog(permission: &PermissionPromptType) -> bool {
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

fn build_extension_install_prompt_message(
    extension_name: &str,
    permission_names: &[String],
) -> String {
    let trimmed_name = extension_name.trim();
    let display_name = if trimmed_name.is_empty() {
        "This extension".to_string()
    } else {
        format!("\"{trimmed_name}\"")
    };

    if permission_names.is_empty() {
        return format!("{display_name} wants to be installed.\n\nAllow this extension?");
    }

    let permissions = permission_names.join("\n- ");
    format!(
        "{display_name} wants to be installed.\n\nRequested permissions:\n- {permissions}\n\nAllow this extension?"
    )
}

fn show_extension_install_prompt_dialog(
    extension_name: &str,
    permission_names: &[String],
) -> bool {
    let message = build_extension_install_prompt_message(extension_name, permission_names);

    let result = MessageDialog::new()
        .set_level(MessageLevel::Info)
        .set_title("Extension Install Request")
        .set_description(&message)
        .set_buttons(MessageButtons::YesNo)
        .show();

    matches!(result, MessageDialogResult::Yes)
}

fn build_extension_uninstall_prompt_message(
    extension_name: &str,
    triggering_extension_name: Option<&str>,
) -> String {
    let trimmed_name = extension_name.trim();
    let display_name = if trimmed_name.is_empty() {
        "This extension".to_string()
    } else {
        format!("\"{trimmed_name}\"")
    };

    match triggering_extension_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(trigger) => format!(
            "{display_name} wants to be uninstalled.\n\nRequested by \"{trigger}\".\n\nAllow this extension to be removed?"
        ),
        None => format!(
            "{display_name} wants to be uninstalled.\n\nAllow this extension to be removed?"
        ),
    }
}

fn show_extension_uninstall_prompt_dialog(
    extension_name: &str,
    triggering_extension_name: Option<&str>,
    can_report_abuse: bool,
) -> AuxiliaryWindowResponse {
    let message =
        build_extension_uninstall_prompt_message(extension_name, triggering_extension_name);

    let proceed = matches!(
        MessageDialog::new()
            .set_level(MessageLevel::Warning)
            .set_title("Extension Uninstall Request")
            .set_description(&message)
            .set_buttons(MessageButtons::YesNo)
            .show(),
        MessageDialogResult::Yes
    );
    let report_abuse = proceed
        && can_report_abuse
        && matches!(
            MessageDialog::new()
                .set_level(MessageLevel::Info)
                .set_title("Report Abuse")
                .set_description(
                    "Do you also want to open the report abuse page after uninstalling this extension?"
                )
                .set_buttons(MessageButtons::YesNo)
                .show(),
            MessageDialogResult::Yes
        );

    AuxiliaryWindowResponse::ExtensionUninstallPrompt {
        proceed,
        report_abuse,
    }
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

fn permission_prompt_description(permission: &PermissionPromptType) -> String {
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
    fn build_extension_install_prompt_message_includes_extension_name() {
        let message = build_extension_install_prompt_message("Example Extension", &[]);

        assert!(message.contains("\"Example Extension\" wants to be installed."));
        assert!(message.contains("Allow this extension?"));
    }

    #[test]
    fn build_extension_install_prompt_message_lists_permissions() {
        let message = build_extension_install_prompt_message(
            "Example Extension",
            &["tabs".to_string(), "storage".to_string()],
        );

        assert!(message.contains("Requested permissions:"));
        assert!(message.contains("- tabs"));
        assert!(message.contains("- storage"));
    }

    #[test]
    fn build_extension_uninstall_prompt_message_includes_trigger() {
        let message = build_extension_uninstall_prompt_message(
            "Example Extension",
            Some("Trigger Extension"),
        );

        assert!(message.contains("\"Example Extension\" wants to be uninstalled."));
        assert!(message.contains("Requested by \"Trigger Extension\"."));
    }

    #[test]
    fn build_extension_uninstall_prompt_message_handles_missing_trigger() {
        let message = build_extension_uninstall_prompt_message("Example Extension", None);

        assert!(message.contains("\"Example Extension\" wants to be uninstalled."));
        assert!(!message.contains("Requested by"));
    }

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
