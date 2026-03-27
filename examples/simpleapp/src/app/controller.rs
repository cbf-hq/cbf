use std::collections::{HashMap, HashSet};

use cbf::{
    browser::BrowserHandle,
    command::BrowserCommand,
    data::{
        auxiliary_window::{
            AuxiliaryWindowKind, AuxiliaryWindowResponse, FormResubmissionPromptReason,
            PermissionPromptType,
        },
        browsing_context_open::{BrowsingContextOpenHint, BrowsingContextOpenResponse},
        dialog::{BeforeUnloadReason, DialogResponse, DialogType, JavaScriptDialogRequest},
        download::{DownloadId, DownloadOutcome, DownloadPromptActionHint, DownloadState},
        ids::{BrowsingContextId, TransientBrowsingContextId, WindowId as HostWindowId},
        ipc::{IpcConfig, IpcErrorCode},
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
    command::ChromeCommand,
    data::find::{ChromeFindInPageOptions, ChromeStopFindAction},
    event::ChromeEvent,
    ffi::IpcEvent,
    process::{ChromiumRuntimeShutdownState, ChromiumRuntimeShutdownStateReader},
};
use cbf_compositor::{
    WindowHost,
    core::{AttachWindowOptions, CompositionCommand, Compositor},
    model::{CompositorWindowId, SurfaceTarget},
};
use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use tracing::{debug, error, info, warn};
use winit::event::WindowEvent;

use crate::{
    app::{
        events::MenuCommand,
        state::{
            CoreAction, DEVTOOLS_HOST_WINDOW_ID, DownloadStatus, JavaScriptDialogTarget,
            MAIN_PAGE_CREATE_REQUEST_ID, OVERLAY_CREATE_REQUEST_ID, PRIMARY_HOST_WINDOW_ID,
            PendingWindowBrowsingContextCreate, PendingWindowBrowsingContextRole,
            SharedStateHandle, TOOLBAR_CREATE_REQUEST_ID, TransientPopupState, allocate_request_id,
            bind_browsing_context_to_window, browsing_context_ids_for_window,
            compositor_window_id_for_host_window, devtools_browsing_context_id, has_bound_windows,
            overlay_browsing_context_id, page_browsing_context_id_for_toolbar_browsing_context,
            page_browsing_context_id_for_window, primary_browsing_context_id,
            primary_host_window_id, register_pending_window_browsing_context_create,
            set_devtools_browsing_context_id, set_overlay_browsing_context_id,
            set_primary_browsing_context_id, set_toolbar_browsing_context_id,
            set_window_page_browsing_context, set_window_toolbar_browsing_context,
            take_pending_window_browsing_context_create, take_pending_window_open_request,
            toolbar_browsing_context_id, toolbar_browsing_context_id_for_window,
            transient_browsing_context_id_for_window, unbind_browsing_context,
            unbind_transient_browsing_context, window_id_for_browsing_context,
            window_id_for_page_browsing_context, window_id_for_toolbar_browsing_context,
            window_id_for_transient_browsing_context,
        },
    },
    cli::Cli,
    ipc::overlay::{
        handler as overlay_handler, protocol as overlay_protocol, publisher as overlay_publisher,
    },
    ipc::toolbar::{
        handler as toolbar_handler, protocol as toolbar_protocol, publisher as toolbar_publisher,
    },
    scene::composition,
    scene::embedded_assets::{APP_ORIGIN, respond_to_request},
    scene::ui_url::{overlay_test_ui_url, toolbar_ui_url},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolbarUiMode {
    Navigation,
    Find,
}

#[derive(Debug, Clone)]
struct FindSessionState {
    ui_mode: ToolbarUiMode,
    query: String,
    number_of_matches: u32,
    active_match_ordinal: i32,
    pending: bool,
    last_request_id: u64,
}

impl Default for FindSessionState {
    fn default() -> Self {
        Self {
            ui_mode: ToolbarUiMode::Navigation,
            query: String::new(),
            number_of_matches: 0,
            active_match_ordinal: 0,
            pending: false,
            last_request_id: 0,
        }
    }
}

impl FindSessionState {
    fn snapshot(&self) -> toolbar_protocol::FindStateSnapshot {
        toolbar_protocol::FindStateSnapshot {
            visible: matches!(self.ui_mode, ToolbarUiMode::Find),
            query: self.query.clone(),
            number_of_matches: self.number_of_matches,
            active_match_ordinal: self.active_match_ordinal,
            pending: self.pending,
        }
    }

    fn open(&mut self) {
        self.ui_mode = ToolbarUiMode::Find;
    }

    fn reset_search_results(&mut self) {
        self.query.clear();
        self.number_of_matches = 0;
        self.active_match_ordinal = 0;
        self.pending = false;
        self.last_request_id = 0;
    }

    fn close(&mut self) {
        self.ui_mode = ToolbarUiMode::Navigation;
        self.reset_search_results();
    }
}

pub(crate) struct AppController {
    cli: Cli,
    browser_handle: BrowserHandle<ChromiumBackend>,
    shutdown_state: ChromiumRuntimeShutdownStateReader,
    compositor: Compositor,
    shared: SharedStateHandle,
    startup_requested: bool,
    shutdown_requested: bool,
    window_base_titles: HashMap<HostWindowId, String>,
    downloads: HashMap<DownloadId, DownloadStatus>,
    transient_popups: HashMap<TransientBrowsingContextId, TransientPopupState>,
    pending_transient_popup_sizes: HashMap<TransientBrowsingContextId, (u32, u32)>,
    blur_close_armed_transients: HashSet<TransientBrowsingContextId>,
    resolved_profile_id: Option<String>,
    extensions_loading: bool,
    navigation_state_by_page: HashMap<BrowsingContextId, toolbar_protocol::NavigationState>,
    find_state_by_page: HashMap<BrowsingContextId, FindSessionState>,
    focused_host_window_id: Option<HostWindowId>,
}

impl AppController {
    pub(crate) fn new(
        cli: Cli,
        browser_handle: BrowserHandle<ChromiumBackend>,
        shutdown_state: ChromiumRuntimeShutdownStateReader,
        shared: SharedStateHandle,
    ) -> Self {
        Self {
            cli,
            browser_handle,
            shutdown_state,
            compositor: Compositor::new(),
            shared,
            startup_requested: false,
            shutdown_requested: false,
            window_base_titles: HashMap::new(),
            downloads: HashMap::new(),
            transient_popups: HashMap::new(),
            pending_transient_popup_sizes: HashMap::new(),
            blur_close_armed_transients: HashSet::new(),
            resolved_profile_id: None,
            extensions_loading: false,
            navigation_state_by_page: HashMap::new(),
            find_state_by_page: HashMap::new(),
            focused_host_window_id: None,
        }
    }

    pub(crate) fn browser_handle(&self) -> BrowserHandle<ChromiumBackend> {
        self.browser_handle.clone()
    }

    pub(crate) fn host_window_id_for_dialog_target(
        &self,
        target: JavaScriptDialogTarget,
    ) -> Option<HostWindowId> {
        match target {
            JavaScriptDialogTarget::BrowsingContext(browsing_context_id) => {
                window_id_for_browsing_context(&self.shared, browsing_context_id)
            }
            JavaScriptDialogTarget::TransientBrowsingContext(transient_browsing_context_id) => {
                window_id_for_transient_browsing_context(
                    &self.shared,
                    transient_browsing_context_id,
                )
            }
        }
    }

    pub(crate) fn attach_window<W>(&mut self, window: W) -> Result<CompositorWindowId, String>
    where
        W: WindowHost + 'static,
    {
        let handle = self.browser_handle.clone();
        self.compositor
            .attach_window(window, AttachWindowOptions, move |command| {
                if let Err(err) = handle.send(command) {
                    warn!("failed to forward compositor command: {err}");
                }
            })
            .map_err(|err| format!("failed to attach compositor window: {err}"))
    }

    pub(crate) fn detach_window(
        &mut self,
        compositor_window_id: CompositorWindowId,
    ) -> Result<(), String> {
        self.compositor
            .detach_window(compositor_window_id, |_command| {})
            .map_err(|err| format!("failed to detach compositor window: {err}"))
    }

    pub(crate) fn handle_menu_command(&mut self, command: MenuCommand) -> Vec<CoreAction> {
        match command {
            MenuCommand::ReloadExtensions => {
                let Some(profile_id) = self.resolved_profile_id.as_deref() else {
                    warn!("ignoring extension reload before a canonical profile is resolved");
                    return Vec::new();
                };
                if let Err(err) = self.browser_handle.request_list_extensions(profile_id) {
                    warn!("failed to request extension list: {err}");
                    return Vec::new();
                }
                self.extensions_loading = true;
                vec![CoreAction::SetExtensionsMenuLoading]
            }
            MenuCommand::OpenExtensionsPage => {
                let Some(browsing_context_id) = primary_browsing_context_id(&self.shared) else {
                    warn!("ignoring extensions page open without a primary browsing context");
                    return Vec::new();
                };
                if let Err(err) = self
                    .browser_handle
                    .navigate(browsing_context_id, "chrome://extensions".to_string())
                {
                    warn!("failed to open extensions page: {err}");
                }
                Vec::new()
            }
            MenuCommand::OpenFind => {
                self.open_find_ui_for_target();
                Vec::new()
            }
            MenuCommand::ActivateExtension { extension_id } => {
                let Some(browsing_context_id) = primary_browsing_context_id(&self.shared) else {
                    warn!("ignoring extension activation without a primary browsing context");
                    return Vec::new();
                };
                if let Err(err) = self
                    .browser_handle
                    .activate_extension_action(browsing_context_id, extension_id)
                {
                    warn!("failed to activate extension action: {err}");
                }
                Vec::new()
            }
        }
    }

    pub(crate) fn handle_browser_event(&mut self, event: BrowserEvent) -> Vec<CoreAction> {
        let browser_handle = self.browser_handle.clone();
        if let Err(err) = self.compositor.update_browser_event(&event, |command| {
            forward_compositor_command(&browser_handle, command);
        }) {
            warn!("failed to update compositor browser state: {err}");
        }

        match event {
            BrowserEvent::BackendReady => {
                if let Err(err) = self.browser_handle.request_list_profiles() {
                    error!("failed to request profile list on startup: {err}");
                    return vec![CoreAction::ExitEventLoop];
                }
                self.extensions_loading = true;
                vec![
                    CoreAction::EnsureMainWindow,
                    CoreAction::SetExtensionsMenuLoading,
                ]
            }
            BrowserEvent::BackendStopped { reason } => {
                match reason {
                    BackendStopReason::Disconnected => {
                        if self.shutdown_state.shutdown_state()
                            != ChromiumRuntimeShutdownState::Idle
                        {
                            info!("backend stopped during shutdown: disconnected");
                        } else {
                            warn!("backend stopped: disconnected");
                        }
                    }
                    BackendStopReason::Crashed => error!("backend stopped: crashed"),
                    BackendStopReason::Error(info) => error!("backend stopped with error: {info}"),
                }
                vec![CoreAction::ExitEventLoop]
            }
            BrowserEvent::BackendError {
                info,
                terminal_hint,
            } => {
                warn!("backend error event: {info}, terminal_hint={terminal_hint}");
                Vec::new()
            }
            BrowserEvent::ProfilesListed { profiles } => self.handle_profiles_listed(profiles),
            BrowserEvent::ExtensionsListed { extensions, .. } => {
                self.extensions_loading = false;
                vec![CoreAction::ReplaceExtensionsMenu { extensions }]
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
                        let target_browsing_context_id = source_browsing_context_id
                            .or_else(|| primary_browsing_context_id(&self.shared));
                        if let Some(browsing_context_id) = target_browsing_context_id {
                            BrowsingContextOpenResponse::AllowExistingContext {
                                browsing_context_id,
                                activate: true,
                            }
                        } else {
                            BrowsingContextOpenResponse::Deny
                        }
                    }
                };
                if let Err(err) = self
                    .browser_handle
                    .respond_browsing_context_open(request_id, response)
                {
                    warn!("failed to respond browsing context open request: {err}");
                }
                Vec::new()
            }
            BrowserEvent::WindowOpenRequested {
                profile_id,
                request,
            } => {
                let kind = match request.requested_kind {
                    WindowKind::Popup => WindowKind::Popup,
                    _ => WindowKind::Normal,
                };
                let window = WindowDescriptor {
                    window_id: HostWindowId::new(request.request_id),
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

                if let Err(err) = self
                    .browser_handle
                    .respond_window_open(request.request_id, WindowOpenResponse::Deny)
                {
                    warn!("failed to respond window open request: {err}");
                }

                let page_request_id = allocate_request_id(&self.shared);
                register_pending_window_browsing_context_create(
                    &self.shared,
                    page_request_id,
                    PendingWindowBrowsingContextCreate {
                        window_id: window.window_id,
                        role: PendingWindowBrowsingContextRole::Page,
                    },
                );
                if let Err(err) = self.browser_handle.create_browsing_context(
                    page_request_id,
                    Some(
                        request
                            .target_url
                            .clone()
                            .unwrap_or_else(|| "about:blank".to_string()),
                    ),
                    profile_id.clone(),
                ) {
                    warn!("failed to create page browsing context for new window: {err}");
                }

                if kind == WindowKind::Normal {
                    let toolbar_request_id = allocate_request_id(&self.shared);
                    register_pending_window_browsing_context_create(
                        &self.shared,
                        toolbar_request_id,
                        PendingWindowBrowsingContextCreate {
                            window_id: window.window_id,
                            role: PendingWindowBrowsingContextRole::Toolbar,
                        },
                    );
                    if let Err(err) = self.browser_handle.create_browsing_context(
                        toolbar_request_id,
                        Some(toolbar_ui_url().unwrap_or_else(|_| "about:blank".to_string())),
                        profile_id,
                    ) {
                        warn!("failed to create toolbar browsing context for new window: {err}");
                    }
                }

                vec![CoreAction::EnsureHostWindow { window }]
            }
            BrowserEvent::WindowOpenResolved {
                request_id, result, ..
            } => {
                if matches!(result, WindowOpenResult::Denied | WindowOpenResult::Aborted)
                    && let Some(window_id) =
                        take_pending_window_open_request(&self.shared, request_id)
                {
                    return vec![CoreAction::CloseHostWindow { window_id }];
                }
                Vec::new()
            }
            BrowserEvent::AuxiliaryWindowOpenRequested {
                profile_id,
                request_id,
                kind,
                ..
            } => {
                self.handle_auxiliary_window_request(profile_id, request_id, kind);
                Vec::new()
            }
            BrowserEvent::DownloadCreated {
                download_id,
                source_browsing_context_id,
                file_name,
                total_bytes,
                ..
            } => {
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
                is_paused,
                ..
            } => {
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
                ..
            } => {
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
            BrowserEvent::ShutdownBlocked { request_id, .. } => {
                if let Err(err) = self.browser_handle.confirm_shutdown(request_id, true) {
                    warn!("failed to confirm shutdown: {err}");
                }
                Vec::new()
            }
            BrowserEvent::ShutdownProceeding { .. }
            | BrowserEvent::ShutdownCancelled { .. }
            | BrowserEvent::BrowsingContextOpenResolved { .. }
            | BrowserEvent::WindowOpened { .. }
            | BrowserEvent::WindowClosed { .. }
            | BrowserEvent::AuxiliaryWindowResolved { .. }
            | BrowserEvent::AuxiliaryWindowOpened { .. }
            | BrowserEvent::AuxiliaryWindowClosed { .. } => Vec::new(),
        }
    }

    pub(crate) fn handle_chrome_event(&mut self, event: ChromeEvent) -> Vec<CoreAction> {
        if let Err(err) = self.compositor.update_chrome_event(&event) {
            warn!("failed to update compositor chrome state: {err}");
        }

        match event {
            ChromeEvent::Ipc(ipc_event) => match *ipc_event {
                IpcEvent::DevToolsOpened {
                    browsing_context_id,
                    inspected_browsing_context_id,
                    ..
                } => {
                    let browsing_context_id = browsing_context_id.into();
                    let inspected_browsing_context_id: BrowsingContextId =
                        inspected_browsing_context_id.into();
                    bind_browsing_context_to_window(
                        &self.shared,
                        browsing_context_id,
                        DEVTOOLS_HOST_WINDOW_ID,
                    );
                    set_devtools_browsing_context_id(&self.shared, Some(browsing_context_id));
                    info!(
                        "devtools opened: inspected={}, devtools={}",
                        inspected_browsing_context_id, browsing_context_id
                    );
                    vec![
                        CoreAction::EnsureDevToolsWindow,
                        CoreAction::SyncWindowScene {
                            window_id: DEVTOOLS_HOST_WINDOW_ID,
                        },
                    ]
                }
                IpcEvent::ChoiceMenuRequested {
                    browsing_context_id,
                    menu,
                    ..
                } => {
                    if let Err(err) = self.compositor.show_choice_menu(
                        SurfaceTarget::BrowsingContext(browsing_context_id.into()),
                        menu,
                    ) {
                        warn!("failed to show choice menu: {err}");
                    }
                    Vec::new()
                }
                IpcEvent::ExtensionPopupChoiceMenuRequested { popup_id, menu, .. } => {
                    if let Err(err) = self.compositor.show_choice_menu(
                        SurfaceTarget::TransientBrowsingContext(TransientBrowsingContextId::new(
                            popup_id.get(),
                        )),
                        menu,
                    ) {
                        warn!("failed to show popup choice menu: {err}");
                    }
                    Vec::new()
                }
                IpcEvent::CustomSchemeRequestReceived { request } => {
                    let response = respond_to_request(&request);
                    if let Err(err) = self
                        .browser_handle
                        .send_raw(ChromeCommand::RespondCustomSchemeRequest { response })
                    {
                        warn!("failed to respond to custom scheme request: {err}");
                    }
                    Vec::new()
                }
                IpcEvent::FindReply {
                    browsing_context_id,
                    request_id,
                    number_of_matches,
                    active_match_ordinal,
                    final_update,
                    ..
                } => {
                    let page_browsing_context_id: BrowsingContextId = browsing_context_id.into();
                    let Some(state) = self.find_state_by_page.get_mut(&page_browsing_context_id)
                    else {
                        return Vec::new();
                    };
                    if !matches!(state.ui_mode, ToolbarUiMode::Find) {
                        return Vec::new();
                    }

                    state.number_of_matches = number_of_matches;
                    state.active_match_ordinal = active_match_ordinal;
                    if request_id == state.last_request_id && final_update {
                        state.pending = false;
                    }
                    self.publish_find_state_for_page(page_browsing_context_id, 0);
                    Vec::new()
                }
                _ => Vec::new(),
            },
            _ => Vec::new(),
        }
    }

    pub(crate) fn handle_window_event(
        &mut self,
        window_id: HostWindowId,
        event: &WindowEvent,
    ) -> Vec<CoreAction> {
        match event {
            WindowEvent::Resized(_) => vec![CoreAction::SyncWindowScene { window_id }],
            WindowEvent::CloseRequested => self.handle_window_close_requested(window_id),
            WindowEvent::Focused(focused) => {
                if *focused {
                    self.focused_host_window_id = Some(window_id);
                } else if self.focused_host_window_id == Some(window_id) {
                    self.focused_host_window_id = None;
                }
                if let Some(transient_id) =
                    transient_browsing_context_id_for_window(&self.shared, window_id)
                {
                    if *focused {
                        self.blur_close_armed_transients.insert(transient_id);
                    } else if self.blur_close_armed_transients.contains(&transient_id)
                        && let Err(err) = self
                            .browser_handle
                            .close_transient_browsing_context(transient_id)
                    {
                        warn!("failed to close transient on blur: {err}");
                    }
                }
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    pub(crate) fn sync_window_scene(
        &mut self,
        host_window_id: HostWindowId,
        compositor_window_id: CompositorWindowId,
        width: u32,
        height: u32,
    ) {
        let browser_handle = self.browser_handle.clone();
        let result = if host_window_id == PRIMARY_HOST_WINDOW_ID {
            let overlay_id = overlay_browsing_context_id(&self.shared);
            let toolbar_id = toolbar_browsing_context_id_for_window(&self.shared, host_window_id)
                .or_else(|| toolbar_browsing_context_id(&self.shared));
            let page_id = page_browsing_context_id_for_window(&self.shared, host_window_id)
                .or_else(|| primary_browsing_context_id(&self.shared));
            if let Some(overlay_id) = overlay_id {
                self.resize_browsing_context(overlay_id, width, height);
            }
            if let Some(toolbar_id) = toolbar_id {
                self.resize_browsing_context(toolbar_id, width, 56);
            }
            if let Some(page_id) = page_id {
                self.resize_browsing_context(page_id, width, height.saturating_sub(56));
            }
            self.compositor.apply(
                CompositionCommand::SetWindowComposition {
                    window_id: compositor_window_id,
                    composition: composition::main_window_composition(
                        overlay_id, toolbar_id, page_id, width, height,
                    ),
                },
                |command| forward_compositor_command(&browser_handle, command),
            )
        } else if host_window_id == DEVTOOLS_HOST_WINDOW_ID {
            if let Some(devtools_id) = devtools_browsing_context_id(&self.shared) {
                self.resize_browsing_context(devtools_id, width, height);
                let browser_handle = browser_handle.clone();
                self.compositor.apply(
                    CompositionCommand::SetWindowComposition {
                        window_id: compositor_window_id,
                        composition: composition::devtools_window_composition(
                            devtools_id,
                            width,
                            height,
                        ),
                    },
                    |command| forward_compositor_command(&browser_handle, command),
                )
            } else {
                let browser_handle = browser_handle.clone();
                self.compositor.apply(
                    CompositionCommand::SetWindowComposition {
                        window_id: compositor_window_id,
                        composition: Default::default(),
                    },
                    |command| forward_compositor_command(&browser_handle, command),
                )
            }
        } else if let Some(transient_id) =
            transient_browsing_context_id_for_window(&self.shared, host_window_id)
        {
            if let Err(err) =
                self.browser_handle
                    .resize_transient_browsing_context(transient_id, width, height)
            {
                warn!("failed to resize transient browsing context: {err}");
            }
            let browser_handle = browser_handle.clone();
            self.compositor.apply(
                CompositionCommand::SetWindowComposition {
                    window_id: compositor_window_id,
                    composition: composition::transient_window_composition(
                        transient_id,
                        width,
                        height,
                    ),
                },
                |command| forward_compositor_command(&browser_handle, command),
            )
        } else {
            let toolbar_id = toolbar_browsing_context_id_for_window(&self.shared, host_window_id);
            let page_id = page_browsing_context_id_for_window(&self.shared, host_window_id)
                .or_else(|| {
                    browsing_context_ids_for_window(&self.shared, host_window_id)
                        .into_iter()
                        .find(|candidate| {
                            Some(*candidate) != toolbar_id
                                && Some(*candidate) != toolbar_browsing_context_id(&self.shared)
                                && Some(*candidate) != devtools_browsing_context_id(&self.shared)
                        })
                });

            if let Some(toolbar_id) = toolbar_id {
                self.resize_browsing_context(toolbar_id, width, 56);
            }

            if let Some(page_id) = page_id {
                let has_toolbar = toolbar_id.is_some();
                self.resize_browsing_context(
                    page_id,
                    width,
                    if has_toolbar {
                        height.saturating_sub(56)
                    } else {
                        height
                    },
                );
            }

            let browser_handle = browser_handle.clone();
            if toolbar_id.is_some() {
                self.compositor.apply(
                    CompositionCommand::SetWindowComposition {
                        window_id: compositor_window_id,
                        composition: composition::main_window_composition(
                            None, toolbar_id, page_id, width, height,
                        ),
                    },
                    |command| forward_compositor_command(&browser_handle, command),
                )
            } else if let Some(page_id) = page_id {
                self.compositor.apply(
                    CompositionCommand::SetWindowComposition {
                        window_id: compositor_window_id,
                        composition: composition::host_window_composition(page_id, width, height),
                    },
                    |command| forward_compositor_command(&browser_handle, command),
                )
            } else {
                self.compositor.apply(
                    CompositionCommand::SetWindowComposition {
                        window_id: compositor_window_id,
                        composition: Default::default(),
                    },
                    |command| forward_compositor_command(&browser_handle, command),
                )
            }
        };

        if let Err(err) = result {
            warn!("failed to sync compositor scene: {err}");
        }
    }

    pub(crate) fn focus_page_surface(&mut self, host_window_id: HostWindowId) {
        let Some(page_browsing_context_id) =
            page_browsing_context_id_for_window(&self.shared, host_window_id)
        else {
            return;
        };

        if let Err(err) = self
            .compositor
            .set_active_item(composition::page_item_id(page_browsing_context_id))
        {
            warn!("failed to focus page surface for window {host_window_id}: {err}");
        }
    }

    pub(crate) fn request_shutdown_once(&mut self) {
        if self.shutdown_requested {
            return;
        }
        self.shutdown_requested = true;
        if let Err(err) = self.browser_handle.request_shutdown(1) {
            if matches!(err, cbf::error::Error::Disconnected)
                && self.shutdown_state.shutdown_state() != ChromiumRuntimeShutdownState::Idle
            {
                debug!("shutdown request skipped because backend is already disconnecting: {err}");
            } else {
                warn!("failed to request shutdown: {err}");
            }
        }
    }

    fn try_enable_toolbar_ipc(&self, toolbar_browsing_context_id: BrowsingContextId) {
        let Some(allowed_origin) = toolbar_allowed_origin() else {
            warn!("failed to derive toolbar allowed origin; skipping IPC enable");
            return;
        };
        let command = BrowserCommand::EnableIpc {
            browsing_context_id: toolbar_browsing_context_id,
            config: IpcConfig {
                allowed_origins: vec![allowed_origin],
            },
        };
        if let Err(err) = self.browser_handle.send(command) {
            warn!("failed to enable toolbar ipc: {err}");
        }
    }

    fn try_enable_overlay_ipc(&self, overlay_browsing_context_id: BrowsingContextId) {
        let command = BrowserCommand::EnableIpc {
            browsing_context_id: overlay_browsing_context_id,
            config: IpcConfig {
                allowed_origins: vec![APP_ORIGIN.to_string()],
            },
        };
        if let Err(err) = self.browser_handle.send(command) {
            warn!("failed to enable overlay ipc: {err}");
        }
    }

    fn post_toolbar_ipc_message(
        &self,
        toolbar_browsing_context_id: BrowsingContextId,
        message: cbf::data::ipc::BrowsingContextIpcMessage,
    ) {
        if let Err(err) = self
            .browser_handle
            .send(BrowserCommand::PostBrowsingContextIpcMessage {
                browsing_context_id: toolbar_browsing_context_id,
                message,
            })
        {
            warn!("failed to post toolbar ipc message: {err}");
        }
    }

    fn post_overlay_ipc_message(
        &self,
        overlay_browsing_context_id: BrowsingContextId,
        message: cbf::data::ipc::BrowsingContextIpcMessage,
    ) {
        if let Err(err) = self
            .browser_handle
            .send(BrowserCommand::PostBrowsingContextIpcMessage {
                browsing_context_id: overlay_browsing_context_id,
                message,
            })
        {
            warn!("failed to post overlay ipc message: {err}");
        }
    }

    fn publish_navigation_state_to_toolbar(
        &self,
        toolbar_browsing_context_id: BrowsingContextId,
        page_browsing_context_id: BrowsingContextId,
    ) {
        let Some(state) = self.navigation_state_by_page.get(&page_browsing_context_id) else {
            return;
        };
        let message = toolbar_publisher::navigation_state_event_message(state);
        self.post_toolbar_ipc_message(toolbar_browsing_context_id, message);
    }

    fn publish_find_state_to_toolbar(
        &self,
        toolbar_browsing_context_id: BrowsingContextId,
        page_browsing_context_id: BrowsingContextId,
        event_request_id: u64,
    ) {
        let Some(state) = self.find_state_by_page.get(&page_browsing_context_id) else {
            return;
        };
        let message =
            toolbar_publisher::find_state_event_message(&state.snapshot(), event_request_id);
        self.post_toolbar_ipc_message(toolbar_browsing_context_id, message);
    }

    fn publish_find_state_for_page(
        &self,
        page_browsing_context_id: BrowsingContextId,
        event_request_id: u64,
    ) {
        let Some(window_id) =
            window_id_for_page_browsing_context(&self.shared, page_browsing_context_id)
        else {
            return;
        };
        let Some(toolbar_browsing_context_id) =
            toolbar_browsing_context_id_for_window(&self.shared, window_id)
        else {
            return;
        };
        self.publish_find_state_to_toolbar(
            toolbar_browsing_context_id,
            page_browsing_context_id,
            event_request_id,
        );
    }

    fn ensure_find_state(
        &mut self,
        page_browsing_context_id: BrowsingContextId,
    ) -> &mut FindSessionState {
        self.find_state_by_page
            .entry(page_browsing_context_id)
            .or_default()
    }

    fn resolve_find_target(&self) -> Option<(HostWindowId, BrowsingContextId)> {
        self.focused_host_window_id
            .and_then(|window_id| {
                page_browsing_context_id_for_window(&self.shared, window_id)
                    .map(|page_browsing_context_id| (window_id, page_browsing_context_id))
            })
            .or_else(|| {
                let window_id =
                    primary_host_window_id(&self.shared).unwrap_or(PRIMARY_HOST_WINDOW_ID);
                page_browsing_context_id_for_window(&self.shared, window_id)
                    .or_else(|| primary_browsing_context_id(&self.shared))
                    .map(|page_browsing_context_id| (window_id, page_browsing_context_id))
            })
    }

    fn open_find_ui_for_target(&mut self) {
        let Some((window_id, page_browsing_context_id)) = self.resolve_find_target() else {
            warn!("ignoring find open without a target page browsing context");
            return;
        };
        self.ensure_find_state(page_browsing_context_id).open();

        let Some(toolbar_browsing_context_id) =
            toolbar_browsing_context_id_for_window(&self.shared, window_id)
        else {
            return;
        };
        if let Err(err) = self
            .compositor
            .set_active_item(composition::toolbar_item_id(toolbar_browsing_context_id))
        {
            warn!("failed to focus toolbar find surface: {err}");
        }
        self.publish_find_state_to_toolbar(
            toolbar_browsing_context_id,
            page_browsing_context_id,
            allocate_request_id(&self.shared),
        );
    }

    fn handle_find_set_query(
        &mut self,
        page_browsing_context_id: BrowsingContextId,
        request_id: u64,
        query: String,
    ) -> cbf::data::ipc::BrowsingContextIpcMessage {
        if query.is_empty() {
            if let Err(err) = self.browser_handle.stop_finding(
                page_browsing_context_id,
                ChromeStopFindAction::ClearSelection,
            ) {
                return toolbar_publisher::response_error_message(
                    toolbar_protocol::CHANNEL_FIND_SET_QUERY,
                    request_id,
                    IpcErrorCode::ContextClosed,
                    "CONTEXT_CLOSED",
                    &format!("failed to clear find state: {err}"),
                );
            }
            let state = self.ensure_find_state(page_browsing_context_id);
            state.open();
            state.reset_search_results();
            self.publish_find_state_for_page(page_browsing_context_id, 0);
            return toolbar_publisher::response_success_message(
                toolbar_protocol::CHANNEL_FIND_SET_QUERY,
                request_id,
                serde_json::json!({}),
            );
        }

        let chrome_request_id = allocate_request_id(&self.shared);
        let mut options = ChromeFindInPageOptions::new(query.clone());
        options.new_session = true;
        options.find_match = true;
        match self
            .browser_handle
            .find_in_page(page_browsing_context_id, chrome_request_id, options)
        {
            Ok(()) => {
                let state = self.ensure_find_state(page_browsing_context_id);
                state.open();
                state.query = query;
                state.number_of_matches = 0;
                state.active_match_ordinal = 0;
                state.pending = true;
                state.last_request_id = chrome_request_id;
                self.publish_find_state_for_page(page_browsing_context_id, 0);
                toolbar_publisher::response_success_message(
                    toolbar_protocol::CHANNEL_FIND_SET_QUERY,
                    request_id,
                    serde_json::json!({}),
                )
            }
            Err(err) => toolbar_publisher::response_error_message(
                toolbar_protocol::CHANNEL_FIND_SET_QUERY,
                request_id,
                IpcErrorCode::ContextClosed,
                "CONTEXT_CLOSED",
                &format!("failed to search page: {err}"),
            ),
        }
    }

    fn handle_find_step(
        &mut self,
        page_browsing_context_id: BrowsingContextId,
        request_id: u64,
        forward: bool,
    ) -> cbf::data::ipc::BrowsingContextIpcMessage {
        let Some(state) = self.find_state_by_page.get(&page_browsing_context_id) else {
            return toolbar_publisher::response_success_message(
                if forward {
                    toolbar_protocol::CHANNEL_FIND_NEXT
                } else {
                    toolbar_protocol::CHANNEL_FIND_PREVIOUS
                },
                request_id,
                serde_json::json!({}),
            );
        };
        if !matches!(state.ui_mode, ToolbarUiMode::Find) || state.query.is_empty() {
            return toolbar_publisher::response_success_message(
                if forward {
                    toolbar_protocol::CHANNEL_FIND_NEXT
                } else {
                    toolbar_protocol::CHANNEL_FIND_PREVIOUS
                },
                request_id,
                serde_json::json!({}),
            );
        }

        let query = state.query.clone();
        let chrome_request_id = allocate_request_id(&self.shared);
        let result = if forward {
            self.browser_handle
                .find_next(page_browsing_context_id, chrome_request_id, query, false)
        } else {
            self.browser_handle.find_previous(
                page_browsing_context_id,
                chrome_request_id,
                query,
                false,
            )
        };

        match result {
            Ok(()) => {
                if let Some(state) = self.find_state_by_page.get_mut(&page_browsing_context_id) {
                    state.pending = true;
                    state.last_request_id = chrome_request_id;
                }
                self.publish_find_state_for_page(page_browsing_context_id, 0);
                toolbar_publisher::response_success_message(
                    if forward {
                        toolbar_protocol::CHANNEL_FIND_NEXT
                    } else {
                        toolbar_protocol::CHANNEL_FIND_PREVIOUS
                    },
                    request_id,
                    serde_json::json!({}),
                )
            }
            Err(err) => toolbar_publisher::response_error_message(
                if forward {
                    toolbar_protocol::CHANNEL_FIND_NEXT
                } else {
                    toolbar_protocol::CHANNEL_FIND_PREVIOUS
                },
                request_id,
                IpcErrorCode::ContextClosed,
                "CONTEXT_CLOSED",
                &format!("failed to move find selection: {err}"),
            ),
        }
    }

    fn handle_find_close(
        &mut self,
        page_browsing_context_id: BrowsingContextId,
        request_id: u64,
    ) -> cbf::data::ipc::BrowsingContextIpcMessage {
        if let Err(err) = self.browser_handle.stop_finding(
            page_browsing_context_id,
            ChromeStopFindAction::KeepSelection,
        ) {
            return toolbar_publisher::response_error_message(
                toolbar_protocol::CHANNEL_FIND_CLOSE,
                request_id,
                IpcErrorCode::ContextClosed,
                "CONTEXT_CLOSED",
                &format!("failed to close find UI: {err}"),
            );
        }

        self.ensure_find_state(page_browsing_context_id).close();
        if let Err(err) = self
            .compositor
            .set_active_item(composition::page_item_id(page_browsing_context_id))
        {
            warn!("failed to restore page focus surface: {err}");
        }
        self.publish_find_state_for_page(page_browsing_context_id, 0);
        toolbar_publisher::response_success_message(
            toolbar_protocol::CHANNEL_FIND_CLOSE,
            request_id,
            serde_json::json!({}),
        )
    }

    fn handle_toolbar_ipc_request(
        &mut self,
        toolbar_browsing_context_id: BrowsingContextId,
        message: cbf::data::ipc::BrowsingContextIpcMessage,
    ) {
        let response_channel = message.channel.clone();
        let (request_id, request) = match toolbar_handler::decode_request(&message) {
            Ok(decoded) => decoded,
            Err(err) => {
                let response = toolbar_publisher::response_error_message(
                    &response_channel,
                    message.request_id,
                    IpcErrorCode::ProtocolError,
                    "PROTOCOL_ERROR",
                    &format!("invalid request: {err:?}"),
                );
                self.post_toolbar_ipc_message(toolbar_browsing_context_id, response);
                return;
            }
        };

        let Some(page_browsing_context_id) = page_browsing_context_id_for_toolbar_browsing_context(
            &self.shared,
            toolbar_browsing_context_id,
        ) else {
            let response = toolbar_publisher::response_error_message(
                &response_channel,
                request_id,
                IpcErrorCode::ContextClosed,
                "CONTEXT_CLOSED",
                "target page context not found",
            );
            self.post_toolbar_ipc_message(toolbar_browsing_context_id, response);
            return;
        };

        let response = match request {
            toolbar_protocol::ToolbarRequest::Open { url } => {
                let normalized_url = toolbar_handler::normalize_url(&url);
                match self
                    .browser_handle
                    .navigate(page_browsing_context_id, normalized_url.clone())
                {
                    Ok(()) => toolbar_publisher::response_success_message(
                        &response_channel,
                        request_id,
                        serde_json::json!({ "url": normalized_url }),
                    ),
                    Err(err) => toolbar_publisher::response_error_message(
                        &response_channel,
                        request_id,
                        IpcErrorCode::ContextClosed,
                        "CONTEXT_CLOSED",
                        &format!("failed to navigate: {err}"),
                    ),
                }
            }
            toolbar_protocol::ToolbarRequest::Back => {
                match self.browser_handle.go_back(page_browsing_context_id) {
                    Ok(()) => toolbar_publisher::response_success_message(
                        &response_channel,
                        request_id,
                        serde_json::json!({}),
                    ),
                    Err(err) => toolbar_publisher::response_error_message(
                        &response_channel,
                        request_id,
                        IpcErrorCode::ContextClosed,
                        "CONTEXT_CLOSED",
                        &format!("failed to go back: {err}"),
                    ),
                }
            }
            toolbar_protocol::ToolbarRequest::Forward => {
                match self.browser_handle.go_forward(page_browsing_context_id) {
                    Ok(()) => toolbar_publisher::response_success_message(
                        &response_channel,
                        request_id,
                        serde_json::json!({}),
                    ),
                    Err(err) => toolbar_publisher::response_error_message(
                        &response_channel,
                        request_id,
                        IpcErrorCode::ContextClosed,
                        "CONTEXT_CLOSED",
                        &format!("failed to go forward: {err}"),
                    ),
                }
            }
            toolbar_protocol::ToolbarRequest::Reload { ignore_cache } => {
                match self
                    .browser_handle
                    .reload(page_browsing_context_id, ignore_cache)
                {
                    Ok(()) => toolbar_publisher::response_success_message(
                        &response_channel,
                        request_id,
                        serde_json::json!({}),
                    ),
                    Err(err) => toolbar_publisher::response_error_message(
                        &response_channel,
                        request_id,
                        IpcErrorCode::ContextClosed,
                        "CONTEXT_CLOSED",
                        &format!("failed to reload: {err}"),
                    ),
                }
            }
            toolbar_protocol::ToolbarRequest::FindSetQuery { query } => {
                self.handle_find_set_query(page_browsing_context_id, request_id, query)
            }
            toolbar_protocol::ToolbarRequest::FindNext => {
                self.handle_find_step(page_browsing_context_id, request_id, true)
            }
            toolbar_protocol::ToolbarRequest::FindPrevious => {
                self.handle_find_step(page_browsing_context_id, request_id, false)
            }
            toolbar_protocol::ToolbarRequest::FindClose => {
                self.handle_find_close(page_browsing_context_id, request_id)
            }
            toolbar_protocol::ToolbarRequest::StateRequest => {
                if let Some(state) = self.navigation_state_by_page.get(&page_browsing_context_id) {
                    toolbar_publisher::response_success_message(
                        &response_channel,
                        request_id,
                        serde_json::to_value(state).unwrap_or_else(|_| serde_json::json!({})),
                    )
                } else {
                    toolbar_publisher::response_success_message(
                        &response_channel,
                        request_id,
                        serde_json::json!({
                            "url": "",
                            "can_go_back": false,
                            "can_go_forward": false,
                            "is_loading": false
                        }),
                    )
                }
            }
        };

        self.post_toolbar_ipc_message(toolbar_browsing_context_id, response);
    }

    fn handle_overlay_ipc_request(
        &mut self,
        overlay_browsing_context_id: BrowsingContextId,
        message: cbf::data::ipc::BrowsingContextIpcMessage,
    ) {
        let response_channel = message.channel.clone();
        let (request_id, request) = match overlay_handler::decode_request(&message) {
            Ok(decoded) => decoded,
            Err(err) => {
                let response = overlay_publisher::response_error_message(
                    &response_channel,
                    message.request_id,
                    IpcErrorCode::ProtocolError,
                    &format!("invalid request: {err:?}"),
                );
                self.post_overlay_ipc_message(overlay_browsing_context_id, response);
                return;
            }
        };

        let Some(window_id) =
            window_id_for_browsing_context(&self.shared, overlay_browsing_context_id)
        else {
            let response = overlay_publisher::response_error_message(
                &response_channel,
                request_id,
                IpcErrorCode::ContextClosed,
                "overlay host window not found",
            );
            self.post_overlay_ipc_message(overlay_browsing_context_id, response);
            return;
        };

        let Some(compositor_window_id) =
            compositor_window_id_for_host_window(&self.shared, window_id)
        else {
            let response = overlay_publisher::response_error_message(
                &response_channel,
                request_id,
                IpcErrorCode::ContextClosed,
                "overlay compositor window not found",
            );
            self.post_overlay_ipc_message(overlay_browsing_context_id, response);
            return;
        };

        let response = match request {
            overlay_protocol::OverlayRequest::UpdateHitTest { snapshot } => {
                match self.compositor.apply(
                    CompositionCommand::SetItemHitTestRegions {
                        window_id: compositor_window_id,
                        item_id: composition::overlay_item_id(overlay_browsing_context_id),
                        snapshot_id: snapshot.snapshot_id,
                        coordinate_space: snapshot.coordinate_space,
                        regions: snapshot.regions,
                    },
                    |_| {},
                ) {
                    Ok(()) => {
                        overlay_publisher::response_success_message(&response_channel, request_id)
                    }
                    Err(err) => overlay_publisher::response_error_message(
                        &response_channel,
                        request_id,
                        IpcErrorCode::ProtocolError,
                        &format!("failed to update overlay hit test regions: {err}"),
                    ),
                }
            }
        };

        self.post_overlay_ipc_message(overlay_browsing_context_id, response);
    }

    fn handle_profiles_listed(&mut self, profiles: Vec<ProfileInfo>) -> Vec<CoreAction> {
        let Some(profile_id) = profiles
            .iter()
            .find(|profile| profile.is_default)
            .or_else(|| profiles.first())
            .map(|profile| profile.profile_id.clone())
        else {
            warn!("no profile available for startup");
            return vec![CoreAction::ExitEventLoop];
        };

        self.resolved_profile_id = Some(profile_id.clone());
        if let Err(err) = self.browser_handle.request_list_extensions(&profile_id) {
            warn!("failed to request extension list: {err}");
        }

        if !self.startup_requested {
            self.startup_requested = true;

            if let Err(err) = self.browser_handle.create_browsing_context(
                TOOLBAR_CREATE_REQUEST_ID,
                Some(toolbar_ui_url().unwrap_or_else(|_| "about:blank".to_string())),
                profile_id.clone(),
            ) {
                warn!("failed to create toolbar browsing context: {err}");
            }

            if self.cli.test_overlay_surface
                && let Err(err) = self.browser_handle.create_browsing_context(
                    OVERLAY_CREATE_REQUEST_ID,
                    Some(overlay_test_ui_url().unwrap_or_else(|_| "about:blank".to_string())),
                    profile_id.clone(),
                )
            {
                warn!("failed to create overlay browsing context: {err}");
            }

            if let Err(err) = self.browser_handle.create_browsing_context(
                MAIN_PAGE_CREATE_REQUEST_ID,
                Some(self.cli.url.clone()),
                profile_id,
            ) {
                warn!("failed to create primary browsing context: {err}");
            }
        }

        vec![CoreAction::SetExtensionsMenuLoading]
    }

    fn handle_browsing_context_event(
        &mut self,
        browsing_context_id: BrowsingContextId,
        event: BrowsingContextEvent,
    ) -> Vec<CoreAction> {
        match event {
            BrowsingContextEvent::Created { request_id } => {
                let mut created_toolbar = false;
                let mut should_focus_page_surface = false;
                let host_window_id = if request_id == TOOLBAR_CREATE_REQUEST_ID {
                    set_toolbar_browsing_context_id(&self.shared, Some(browsing_context_id));
                    set_window_toolbar_browsing_context(
                        &self.shared,
                        PRIMARY_HOST_WINDOW_ID,
                        Some(browsing_context_id),
                    );
                    created_toolbar = true;
                    Some(PRIMARY_HOST_WINDOW_ID)
                } else if request_id == OVERLAY_CREATE_REQUEST_ID {
                    set_overlay_browsing_context_id(&self.shared, Some(browsing_context_id));
                    self.try_enable_overlay_ipc(browsing_context_id);
                    Some(PRIMARY_HOST_WINDOW_ID)
                } else if request_id == MAIN_PAGE_CREATE_REQUEST_ID {
                    set_primary_browsing_context_id(&self.shared, Some(browsing_context_id));
                    set_window_page_browsing_context(
                        &self.shared,
                        PRIMARY_HOST_WINDOW_ID,
                        Some(browsing_context_id),
                    );
                    should_focus_page_surface = true;
                    Some(PRIMARY_HOST_WINDOW_ID)
                } else if let Some(pending) =
                    take_pending_window_browsing_context_create(&self.shared, request_id)
                {
                    match pending.role {
                        PendingWindowBrowsingContextRole::Page => {
                            should_focus_page_surface = true;
                            set_window_page_browsing_context(
                                &self.shared,
                                pending.window_id,
                                Some(browsing_context_id),
                            )
                        }
                        PendingWindowBrowsingContextRole::Toolbar => {
                            created_toolbar = true;
                            set_window_toolbar_browsing_context(
                                &self.shared,
                                pending.window_id,
                                Some(browsing_context_id),
                            )
                        }
                    }
                    Some(pending.window_id)
                } else {
                    take_pending_window_open_request(&self.shared, request_id)
                };

                if let Some(host_window_id) = host_window_id {
                    bind_browsing_context_to_window(
                        &self.shared,
                        browsing_context_id,
                        host_window_id,
                    );
                    if created_toolbar {
                        self.try_enable_toolbar_ipc(browsing_context_id);
                        if let Some(page_browsing_context_id) =
                            page_browsing_context_id_for_window(&self.shared, host_window_id)
                        {
                            self.ensure_find_state(page_browsing_context_id);
                            self.publish_navigation_state_to_toolbar(
                                browsing_context_id,
                                page_browsing_context_id,
                            );
                            self.publish_find_state_to_toolbar(
                                browsing_context_id,
                                page_browsing_context_id,
                                0,
                            );
                        }
                    }
                    let mut actions = vec![CoreAction::SyncWindowScene {
                        window_id: host_window_id,
                    }];
                    if should_focus_page_surface {
                        actions.push(CoreAction::FocusPageSurface {
                            window_id: host_window_id,
                        });
                    }
                    actions
                } else {
                    Vec::new()
                }
            }
            BrowsingContextEvent::TitleUpdated { title } => {
                if Some(browsing_context_id) == toolbar_browsing_context_id(&self.shared)
                    || window_id_for_toolbar_browsing_context(&self.shared, browsing_context_id)
                        .is_some()
                    || Some(browsing_context_id) == overlay_browsing_context_id(&self.shared)
                {
                    return Vec::new();
                }
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
                window_id_for_browsing_context(&self.shared, browsing_context_id)
                    .map(|window_id| CoreAction::UpdateCursor {
                        window_id,
                        cursor: cursor_type,
                    })
                    .into_iter()
                    .collect()
            }
            BrowsingContextEvent::ContextMenuRequested { menu } => {
                if let Err(err) = self
                    .compositor
                    .show_context_menu(SurfaceTarget::BrowsingContext(browsing_context_id), menu)
                {
                    warn!("failed to show context menu: {err}");
                }
                Vec::new()
            }
            BrowsingContextEvent::DragStartRequested { request } => {
                if let Err(err) = self.compositor.start_native_drag(request) {
                    warn!("failed to start native drag: {err}");
                }
                Vec::new()
            }
            BrowsingContextEvent::JavaScriptDialogRequested {
                request_id,
                message,
                default_prompt_text,
                r#type,
                beforeunload_reason,
            } => {
                if r#type == DialogType::BeforeUnload {
                    let response = show_beforeunload_dialog(&message, beforeunload_reason.as_ref());
                    if let Err(err) = respond_javascript_dialog_for_browsing_context(
                        self.browser_handle.clone(),
                        browsing_context_id,
                        request_id,
                        r#type,
                        response,
                    ) {
                        warn!("failed to respond javascript dialog: {err}");
                    }
                    return Vec::new();
                }
                vec![CoreAction::PresentJavaScriptDialog {
                    target: JavaScriptDialogTarget::BrowsingContext(browsing_context_id),
                    request_id,
                    request: JavaScriptDialogRequest::new(r#type, message, default_prompt_text),
                }]
            }
            BrowsingContextEvent::PermissionRequested { request_id, .. } => {
                _ = self
                    .browser_handle
                    .confirm_permission(browsing_context_id, request_id, false);
                Vec::new()
            }
            BrowsingContextEvent::CloseRequested => {
                if let Err(err) = self
                    .browser_handle
                    .request_close_browsing_context(browsing_context_id)
                {
                    warn!("failed to request close: {err}");
                }
                Vec::new()
            }
            BrowsingContextEvent::Closed => {
                self.navigation_state_by_page.remove(&browsing_context_id);
                self.find_state_by_page.remove(&browsing_context_id);
                self.handle_closed_browsing_context(browsing_context_id)
            }
            BrowsingContextEvent::NavigationStateChanged {
                url,
                can_go_back,
                can_go_forward,
                is_loading,
            } => {
                if window_id_for_toolbar_browsing_context(&self.shared, browsing_context_id)
                    .is_some()
                    || Some(browsing_context_id) == toolbar_browsing_context_id(&self.shared)
                    || Some(browsing_context_id) == overlay_browsing_context_id(&self.shared)
                {
                    return Vec::new();
                }

                debug!(
                    browsing_context_id = ?browsing_context_id,
                    url = %url,
                    can_go_back,
                    can_go_forward,
                    is_loading,
                    "navigation_state_changed"
                );

                self.navigation_state_by_page.insert(
                    browsing_context_id,
                    toolbar_protocol::NavigationState {
                        url,
                        can_go_back,
                        can_go_forward,
                        is_loading,
                    },
                );

                if let Some(window_id) =
                    window_id_for_page_browsing_context(&self.shared, browsing_context_id)
                    && let Some(toolbar_browsing_context_id) =
                        toolbar_browsing_context_id_for_window(&self.shared, window_id)
                {
                    self.ensure_find_state(browsing_context_id);
                    self.publish_navigation_state_to_toolbar(
                        toolbar_browsing_context_id,
                        browsing_context_id,
                    );
                    self.publish_find_state_to_toolbar(
                        toolbar_browsing_context_id,
                        browsing_context_id,
                        0,
                    );
                }
                Vec::new()
            }
            BrowsingContextEvent::IpcMessageReceived { message } => {
                if window_id_for_toolbar_browsing_context(&self.shared, browsing_context_id)
                    .is_some()
                    || Some(browsing_context_id) == toolbar_browsing_context_id(&self.shared)
                {
                    self.handle_toolbar_ipc_request(browsing_context_id, message);
                } else if Some(browsing_context_id) == overlay_browsing_context_id(&self.shared) {
                    self.handle_overlay_ipc_request(browsing_context_id, message);
                }
                Vec::new()
            }
            BrowsingContextEvent::FaviconUrlUpdated { .. }
            | BrowsingContextEvent::UpdateTargetUrl { .. }
            | BrowsingContextEvent::FullscreenToggled { .. }
            | BrowsingContextEvent::ImeBoundsUpdated { .. }
            | BrowsingContextEvent::ChoiceMenuRequested { .. }
            | BrowsingContextEvent::RenderProcessGone { .. }
            | BrowsingContextEvent::AudioStateChanged { .. }
            | BrowsingContextEvent::DomHtmlRead { .. }
            | BrowsingContextEvent::ExtensionRuntimeWarning { .. }
            | BrowsingContextEvent::SelectionChanged { .. }
            | BrowsingContextEvent::ScrollPositionChanged { .. }
            | BrowsingContextEvent::ExternalDragOperationChanged { .. } => Vec::new(),
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
                let size = self
                    .pending_transient_popup_sizes
                    .remove(&transient_browsing_context_id);
                let state = TransientPopupState {
                    parent_browsing_context_id,
                    title: title.unwrap_or_else(|| "Extension Popup".to_string()),
                    size,
                    prev_sent_size: None,
                };
                let (width, height) = state.size.unwrap_or((420, 600));
                self.transient_popups
                    .insert(transient_browsing_context_id, state.clone());
                vec![CoreAction::EnsureTransientHostWindow {
                    transient_browsing_context_id,
                    title: state.title,
                    width,
                    height,
                }]
            }
            TransientBrowsingContextEvent::Resized { width, height } => {
                self.handle_transient_resize(transient_browsing_context_id, width, height)
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
            TransientBrowsingContextEvent::CursorChanged { cursor_type } => {
                window_id_for_transient_browsing_context(
                    &self.shared,
                    transient_browsing_context_id,
                )
                .map(|window_id| CoreAction::UpdateCursor {
                    window_id,
                    cursor: cursor_type,
                })
                .into_iter()
                .collect()
            }
            TransientBrowsingContextEvent::ContextMenuRequested { menu } => {
                if let Err(err) = self.compositor.show_context_menu(
                    SurfaceTarget::TransientBrowsingContext(transient_browsing_context_id),
                    menu,
                ) {
                    warn!("failed to show popup context menu: {err}");
                }
                Vec::new()
            }
            TransientBrowsingContextEvent::JavaScriptDialogRequested {
                request_id,
                message,
                default_prompt_text,
                r#type,
                beforeunload_reason,
            } => {
                if r#type == DialogType::BeforeUnload {
                    let response = show_beforeunload_dialog(&message, beforeunload_reason.as_ref());

                    if let Err(err) = self
                        .browser_handle
                        .respond_javascript_dialog_in_transient_browsing_context(
                            transient_browsing_context_id,
                            request_id,
                            response,
                        )
                    {
                        warn!("failed to respond transient javascript dialog: {err}");
                    }
                    return Vec::new();
                }

                vec![CoreAction::PresentJavaScriptDialog {
                    target: JavaScriptDialogTarget::TransientBrowsingContext(
                        transient_browsing_context_id,
                    ),
                    request_id,
                    request: JavaScriptDialogRequest::new(r#type, message, default_prompt_text),
                }]
            }
            TransientBrowsingContextEvent::CloseRequested => {
                if let Err(err) = self
                    .browser_handle
                    .close_transient_browsing_context(transient_browsing_context_id)
                {
                    warn!("failed to close transient: {err}");
                }
                Vec::new()
            }
            TransientBrowsingContextEvent::Closed { .. }
            | TransientBrowsingContextEvent::RenderProcessGone { .. } => {
                self.transient_popups.remove(&transient_browsing_context_id);
                self.pending_transient_popup_sizes
                    .remove(&transient_browsing_context_id);
                self.blur_close_armed_transients
                    .remove(&transient_browsing_context_id);

                unbind_transient_browsing_context(&self.shared, transient_browsing_context_id)
                    .map(|window_id| CoreAction::CloseHostWindow { window_id })
                    .into_iter()
                    .collect()
            }
            TransientBrowsingContextEvent::Focused
            | TransientBrowsingContextEvent::Blurred
            | TransientBrowsingContextEvent::ImeBoundsUpdated { .. }
            | TransientBrowsingContextEvent::ChoiceMenuRequested { .. } => Vec::new(),
        }
    }

    fn handle_closed_browsing_context(
        &mut self,
        browsing_context_id: BrowsingContextId,
    ) -> Vec<CoreAction> {
        let is_toolbar = Some(browsing_context_id) == toolbar_browsing_context_id(&self.shared);
        let is_overlay = Some(browsing_context_id) == overlay_browsing_context_id(&self.shared);
        let is_primary = Some(browsing_context_id) == primary_browsing_context_id(&self.shared);
        let is_devtools = Some(browsing_context_id) == devtools_browsing_context_id(&self.shared);
        let secondary_toolbar_window_id =
            window_id_for_toolbar_browsing_context(&self.shared, browsing_context_id)
                .filter(|window_id| *window_id != PRIMARY_HOST_WINDOW_ID);
        let secondary_page_window_id =
            window_id_for_page_browsing_context(&self.shared, browsing_context_id)
                .filter(|window_id| *window_id != PRIMARY_HOST_WINDOW_ID);

        let window_id = unbind_browsing_context(&self.shared, browsing_context_id);

        if is_toolbar {
            set_toolbar_browsing_context_id(&self.shared, None);
            set_window_toolbar_browsing_context(&self.shared, PRIMARY_HOST_WINDOW_ID, None);
        }
        if is_overlay {
            set_overlay_browsing_context_id(&self.shared, None);
        }
        if is_primary {
            set_primary_browsing_context_id(&self.shared, None);
            set_window_page_browsing_context(&self.shared, PRIMARY_HOST_WINDOW_ID, None);

            if let Some(toolbar_id) = toolbar_browsing_context_id(&self.shared) {
                _ = self
                    .browser_handle
                    .request_close_browsing_context(toolbar_id);
            }
        }
        if let Some(window_id) = secondary_toolbar_window_id {
            set_window_toolbar_browsing_context(&self.shared, window_id, None);
            if let Some(page_id) = page_browsing_context_id_for_window(&self.shared, window_id) {
                _ = self.browser_handle.request_close_browsing_context(page_id);
            }
        }
        if let Some(window_id) = secondary_page_window_id {
            set_window_page_browsing_context(&self.shared, window_id, None);
            if let Some(toolbar_id) =
                toolbar_browsing_context_id_for_window(&self.shared, window_id)
            {
                _ = self
                    .browser_handle
                    .request_close_browsing_context(toolbar_id);
            }
        }
        if is_devtools {
            set_devtools_browsing_context_id(&self.shared, None);
        }

        let mut actions = Vec::new();
        if let Some(window_id) = window_id {
            let remaining = browsing_context_ids_for_window(&self.shared, window_id);

            if remaining.is_empty()
                || is_primary
                || is_devtools
                || secondary_toolbar_window_id.is_some()
                || secondary_page_window_id.is_some()
            {
                self.window_base_titles.remove(&window_id);
                actions.push(CoreAction::CloseHostWindow { window_id });
            } else {
                actions.push(CoreAction::SyncWindowScene { window_id });
            }
        }
        actions.extend(self.refresh_primary_window_title());

        if !has_bound_windows(&self.shared) {
            self.request_shutdown_once();
            actions.push(CoreAction::ExitEventLoop);
        }
        actions
    }

    fn handle_transient_resize(
        &mut self,
        transient_browsing_context_id: TransientBrowsingContextId,
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

        let new_size = (width, height);
        let size_changed = state.size != Some(new_size);
        let is_oscillating = size_changed && state.prev_sent_size == Some(new_size);
        if is_oscillating {
            return Vec::new();
        }

        let previous = state.size;
        state.size = Some(new_size);
        if let Some(window_id) =
            window_id_for_transient_browsing_context(&self.shared, transient_browsing_context_id)
        {
            if size_changed {
                state.prev_sent_size = previous;
                return vec![CoreAction::ResizeHostWindow {
                    window_id,
                    width,
                    height,
                }];
            }
            Vec::new()
        } else {
            vec![CoreAction::EnsureTransientHostWindow {
                transient_browsing_context_id,
                title: state.title.clone(),
                width,
                height,
            }]
        }
    }

    fn handle_window_close_requested(&mut self, window_id: HostWindowId) -> Vec<CoreAction> {
        if let Some(transient_id) =
            transient_browsing_context_id_for_window(&self.shared, window_id)
        {
            if let Err(err) = self
                .browser_handle
                .close_transient_browsing_context(transient_id)
            {
                warn!("failed to close transient window: {err}");
                return vec![CoreAction::CloseHostWindow { window_id }];
            }
            return Vec::new();
        }

        let browsing_context_ids = browsing_context_ids_for_window(&self.shared, window_id);
        for browsing_context_id in browsing_context_ids {
            if let Err(err) = self
                .browser_handle
                .request_close_browsing_context(browsing_context_id)
            {
                warn!("failed to request close for browsing context: {err}");
            }
        }
        Vec::new()
    }

    fn handle_auxiliary_window_request(
        &mut self,
        profile_id: String,
        request_id: u64,
        kind: AuxiliaryWindowKind,
    ) {
        if let AuxiliaryWindowKind::PermissionPrompt { permission } = &kind {
            let allow = show_permission_prompt_dialog(permission);
            _ = self.browser_handle.respond_auxiliary_window(
                profile_id,
                request_id,
                AuxiliaryWindowResponse::PermissionPrompt { allow },
            );
            return;
        }

        if let AuxiliaryWindowKind::ExtensionInstallPrompt {
            extension_name,
            permission_names,
            ..
        } = &kind
        {
            let proceed = show_extension_install_prompt_dialog(extension_name, permission_names);
            _ = self.browser_handle.respond_auxiliary_window(
                profile_id,
                request_id,
                AuxiliaryWindowResponse::ExtensionInstallPrompt { proceed },
            );
            return;
        }

        if let AuxiliaryWindowKind::ExtensionUninstallPrompt {
            extension_name,
            triggering_extension_name,
            ..
        } = &kind
        {
            let proceed = show_extension_uninstall_prompt_dialog(
                extension_name,
                triggering_extension_name.as_deref(),
            );
            _ = self.browser_handle.respond_auxiliary_window(
                profile_id,
                request_id,
                AuxiliaryWindowResponse::ExtensionUninstallPrompt {
                    proceed,
                    report_abuse: false,
                },
            );
            return;
        }

        if let AuxiliaryWindowKind::DownloadPrompt {
            file_name,
            suggested_path,
            action_hint,
            ..
        } = &kind
        {
            let response = download_prompt_response_for_simpleapp(
                *action_hint,
                file_name,
                suggested_path,
                self.cli.download_dir.as_deref(),
            );
            _ = self
                .browser_handle
                .respond_auxiliary_window(profile_id, request_id, response);
            return;
        }

        if let AuxiliaryWindowKind::FormResubmissionPrompt { reason, target_url } = &kind {
            let proceed = show_form_resubmission_prompt_dialog(*reason, target_url.as_deref());
            _ = self.browser_handle.respond_auxiliary_window(
                profile_id,
                request_id,
                AuxiliaryWindowResponse::FormResubmissionPrompt { proceed },
            );
            return;
        }

        if let Err(err) = self
            .browser_handle
            .open_default_auxiliary_window(profile_id, request_id)
        {
            warn!("failed to open default auxiliary window: {err}");
        }
    }

    fn resize_browsing_context(
        &self,
        browsing_context_id: BrowsingContextId,
        width: u32,
        height: u32,
    ) {
        if let Err(err) =
            self.browser_handle
                .resize_browsing_context(browsing_context_id, width, height)
        {
            warn!("failed to resize browsing context: {err}");
        }
    }

    fn decorated_window_title(&self, window_id: HostWindowId) -> String {
        let base_title = self
            .window_base_titles
            .get(&window_id)
            .cloned()
            .unwrap_or_else(|| "CBF SimpleApp".to_string());

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

    fn download_title_suffix(&self) -> Option<String> {
        let active: Vec<_> = self
            .downloads
            .values()
            .filter(|download| {
                matches!(
                    download.state,
                    DownloadState::InProgress | DownloadState::Paused
                )
            })
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
                match format_progress(download.received_bytes, download.total_bytes) {
                    Some(progress) => format!("{verb} {progress} - {}", download.file_name),
                    None => format!("{verb} - {}", download.file_name),
                },
            );
        }
        Some(format!("{} downloads active", active.len()))
    }
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

pub(crate) fn respond_javascript_dialog_for_target(
    browser: BrowserHandle<ChromiumBackend>,
    target: JavaScriptDialogTarget,
    request_id: u64,
    response: DialogResponse,
) {
    let result = match target {
        JavaScriptDialogTarget::BrowsingContext(browsing_context_id) => {
            browser.respond_javascript_dialog(browsing_context_id, request_id, response)
        }
        JavaScriptDialogTarget::TransientBrowsingContext(transient_browsing_context_id) => browser
            .respond_javascript_dialog_in_transient_browsing_context(
                transient_browsing_context_id,
                request_id,
                response,
            ),
    };
    if let Err(err) = result {
        warn!("failed to respond javascript dialog: {err}");
    }
}

fn show_beforeunload_dialog(
    message: &str,
    beforeunload_reason: Option<&BeforeUnloadReason>,
) -> DialogResponse {
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

fn permission_prompt_description(permission: &PermissionPromptType) -> &'static str {
    match permission {
        PermissionPromptType::VideoCapture => "This page wants to use the camera.",
        PermissionPromptType::AudioCapture => "This page wants to use the microphone.",
        PermissionPromptType::Notifications => "This page wants to show notifications.",
        PermissionPromptType::Geolocation => "This page wants to access your location.",
        PermissionPromptType::Other(_) => "This page wants a browser permission.",
        PermissionPromptType::Unknown => "This page wants a browser permission.",
    }
}

fn show_extension_install_prompt_dialog(extension_name: &str, permission_names: &[String]) -> bool {
    let permissions = if permission_names.is_empty() {
        "No additional permissions were listed.".to_string()
    } else {
        permission_names
            .iter()
            .map(|permission| format!("• {permission}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let message = format!(
        "Install the extension “{extension_name}”?\n\nRequested permissions:\n{permissions}"
    );

    let result = MessageDialog::new()
        .set_level(MessageLevel::Info)
        .set_title("Install Extension")
        .set_description(&message)
        .set_buttons(MessageButtons::YesNo)
        .show();

    matches!(result, MessageDialogResult::Yes)
}

fn show_extension_uninstall_prompt_dialog(
    extension_name: &str,
    triggering_extension_name: Option<&str>,
) -> bool {
    let mut message = format!("Remove the extension “{extension_name}”?");
    if let Some(trigger) = triggering_extension_name {
        message.push_str(&format!(
            "\n\nThis request was initiated by the extension “{trigger}”."
        ));
    }

    let result = MessageDialog::new()
        .set_level(MessageLevel::Warning)
        .set_title("Remove Extension")
        .set_description(&message)
        .set_buttons(MessageButtons::YesNo)
        .show();

    matches!(result, MessageDialogResult::Yes)
}

fn show_form_resubmission_prompt_dialog(
    reason: FormResubmissionPromptReason,
    target_url: Option<&str>,
) -> bool {
    let reason_message = match reason {
        FormResubmissionPromptReason::Reload => {
            "Reloading this page will resend the previous form data."
        }
        FormResubmissionPromptReason::BackForward => {
            "Moving back or forward will resend the previous form data."
        }
        FormResubmissionPromptReason::Other => "Continuing will resend the previous form data.",
        FormResubmissionPromptReason::Unknown => {
            "The page needs to resend previously submitted form data."
        }
    };

    let mut message = reason_message.to_string();
    if let Some(url) = target_url.filter(|value| !value.is_empty()) {
        message.push_str(&format!("\n\nTarget:\n{url}"));
    }
    message.push_str("\n\nResend form data?");

    let result = MessageDialog::new()
        .set_level(MessageLevel::Warning)
        .set_title("Confirm Form Resubmission")
        .set_description(&message)
        .set_buttons(MessageButtons::YesNo)
        .show();

    matches!(result, MessageDialogResult::Yes)
}

fn download_prompt_response_for_simpleapp(
    action_hint: DownloadPromptActionHint,
    file_name: &str,
    suggested_path: &Option<String>,
    download_dir: Option<&std::path::Path>,
) -> AuxiliaryWindowResponse {
    match action_hint {
        DownloadPromptActionHint::AutoSave => AuxiliaryWindowResponse::DownloadPrompt {
            allow: true,
            destination_path: download_dir
                .map(|dir| dir.join(file_name))
                .or_else(|| suggested_path.as_ref().map(std::path::PathBuf::from))
                .map(|path| path.to_string_lossy().to_string()),
        },
        DownloadPromptActionHint::Deny => AuxiliaryWindowResponse::DownloadPrompt {
            allow: false,
            destination_path: None,
        },
        DownloadPromptActionHint::SelectDestination | DownloadPromptActionHint::Unknown => {
            let path = FileDialog::new()
                .set_file_name(file_name)
                .save_file()
                .map(|path| path.to_string_lossy().to_string());
            AuxiliaryWindowResponse::DownloadPrompt {
                allow: path.is_some(),
                destination_path: path,
            }
        }
    }
}

fn toolbar_allowed_origin() -> Option<String> {
    let _ = toolbar_ui_url().ok()?;
    Some(APP_ORIGIN.to_string())
}

fn forward_compositor_command(
    browser_handle: &BrowserHandle<ChromiumBackend>,
    command: cbf::command::BrowserCommand,
) {
    if let Err(err) = browser_handle.send(command) {
        warn!("failed to forward compositor command: {err}");
    }
}
