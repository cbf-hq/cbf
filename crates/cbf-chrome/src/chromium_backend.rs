use std::{
    collections::{HashMap, HashSet},
    thread,
    time::{Duration, Instant},
};

use async_channel::{Receiver, Sender, TryRecvError};

use cbf::{
    backend_delegate::{BackendDelegate, DelegateDispatcher},
    browser::Backend,
    command::BrowserCommand,
    data::ids::BrowsingContextId,
    error::{ApiErrorKind, BackendErrorInfo, Error, Operation},
    event::{BackendStopReason, BrowserEvent, DialogType, BrowsingContextEvent},
};

use crate::ffi::{Error as IpcError, IpcClient, IpcEvent};

/// Backend implementation that speaks the Chromium IPC protocol.
#[derive(Debug, Clone)]
pub struct ChromiumBackend {
    options: ChromiumBackendOptions,
}

/// Options for establishing an IPC connection to Chromium.
#[derive(Debug, Clone)]
pub struct ChromiumBackendOptions {
    /// The name of the IPC channel to connect to.
    pub channel_name: String,
    /// Timeout for establishing the initial IPC connection.
    ///
    /// `Some(duration)` fails startup if the timeout is exceeded.
    /// `None` waits indefinitely until a connection is established.
    pub connect_timeout: Option<Duration>,
    /// Retry interval between IPC connect attempts.
    pub retry_interval: Duration,
}

impl ChromiumBackendOptions {
    /// Create options with default connect behavior for the given channel.
    pub fn new(channel_name: impl Into<String>) -> Self {
        Self {
            channel_name: channel_name.into(),
            connect_timeout: Some(Duration::from_secs(60)),
            retry_interval: Duration::from_millis(100),
        }
    }
}

struct CommunicationState {
    resizes_in_flight: HashSet<BrowsingContextId>,
    pending_resizes: HashMap<BrowsingContextId, (u32, u32)>,
}

#[derive(Debug)]
enum CommandExecutionError {
    IpcCall {
        operation: Operation,
        source: IpcError,
    },
}

impl CommandExecutionError {
    fn into_backend_error_info(self) -> BackendErrorInfo {
        match self {
            Self::IpcCall { operation, source } => BackendErrorInfo {
                kind: match source {
                    IpcError::ConnectionFailed => ApiErrorKind::CommandDispatchFailed,
                    IpcError::InvalidInput => ApiErrorKind::InvalidInput,
                    IpcError::InvalidEvent => ApiErrorKind::ProtocolMismatch,
                },
                operation: Some(operation),
                detail: Some(format!("{source:?}")),
            },
        }
    }
}

fn backend_error_connect_timeout(source: IpcError) -> BackendErrorInfo {
    BackendErrorInfo {
        kind: ApiErrorKind::ConnectTimeout,
        operation: None,
        detail: Some(format!("{source:?}")),
    }
}

fn backend_error_event(source: IpcError) -> BackendErrorInfo {
    let kind = match source {
        IpcError::InvalidEvent => ApiErrorKind::ProtocolMismatch,
        IpcError::InvalidInput => ApiErrorKind::InvalidInput,
        IpcError::ConnectionFailed => ApiErrorKind::EventProcessingFailed,
    };

    BackendErrorInfo {
        kind,
        operation: None,
        detail: Some(format!("{source:?}")),
    }
}

fn backend_error_terminal_hint(kind: ApiErrorKind) -> bool {
    matches!(
        kind,
        ApiErrorKind::ConnectTimeout | ApiErrorKind::ProtocolMismatch
    )
}

impl Backend for ChromiumBackend {
    fn connect<D: BackendDelegate>(
        self,
        delegate: D,
    ) -> Result<(Sender<BrowserCommand>, Receiver<BrowserEvent>), Error> {
        let (command_tx, command_rx) = async_channel::unbounded::<BrowserCommand>();
        let (event_tx, event_rx) = async_channel::unbounded::<BrowserEvent>();
        let options = self.options;

        thread::spawn(move || {
            Self::run_communication(options, command_rx, event_tx, delegate)
        });

        Ok((command_tx, event_rx))
    }
}

impl ChromiumBackend {
    /// Create a backend from Chromium IPC connection options.
    pub fn new(options: ChromiumBackendOptions) -> Self {
        Self { options }
    }

    fn start_connection(
        event_tx: &Sender<BrowserEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
        options: &ChromiumBackendOptions,
    ) -> Option<IpcClient> {
        let start_time = Instant::now();

        let mut client = loop {
            match IpcClient::connect(&options.channel_name) {
                Ok(client) => break client,
                Err(err) => {
                    if options
                        .connect_timeout
                        .is_some_and(|timeout| start_time.elapsed() > timeout)
                    {
                        let info = backend_error_connect_timeout(err);
                        let stop_reason = dispatcher
                            .dispatch_event(
                                BrowserEvent::BackendError {
                                    terminal_hint: true,
                                    info: info.clone(),
                                },
                                event_tx,
                            )
                            .unwrap_or(BackendStopReason::Error(info));
                        let mut no_forward = |_| (None, Vec::new());
                        dispatcher.stop(event_tx, stop_reason, &mut no_forward);

                        return None;
                    }

                    thread::sleep(options.retry_interval);
                }
            }
        };

        // Notify that the backend is ready after establishing the connection.
        if let Some(stop_reason) = dispatcher.dispatch_event(BrowserEvent::BackendReady, event_tx) {
            let mut forward = |command| Self::execute_command(command, &mut client, event_tx);
            dispatcher.stop(event_tx, stop_reason, &mut forward);
            return None;
        }

        Some(client)
    }

    fn poll_event(
        command_rx: &Receiver<BrowserCommand>,
        client: &mut IpcClient,
        event_tx: &Sender<BrowserEvent>,
        state: &mut CommunicationState,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
    ) -> bool {
        dispatcher.on_idle();

        macro_rules! make_forward {
            () => {
                |command| Self::execute_command(command, client, event_tx)
            };
        }

        let mut forward = make_forward!();
        if let Some(stop_reason) = dispatcher.flush(event_tx, &mut forward) {
            dispatcher.stop(event_tx, stop_reason, &mut forward);
            return false;
        }

        if let Some(stop_reason) =
            Self::process_command_queue(command_rx, client, event_tx, state, dispatcher)
        {
            let mut forward = make_forward!();
            dispatcher.stop(event_tx, stop_reason, &mut forward);

            return false;
        };

        if let Some(stop_reason) = Self::process_event_queue(client, event_tx, state, dispatcher) {
            let mut forward = make_forward!();
            dispatcher.stop(event_tx, stop_reason, &mut forward);

            return false;
        };

        let mut forward = make_forward!();
        if let Some(stop_reason) = dispatcher.flush(event_tx, &mut forward) {
            dispatcher.stop(event_tx, stop_reason, &mut forward);
            return false;
        }

        true
    }

    fn run_communication(
        options: ChromiumBackendOptions,
        command_rx: Receiver<BrowserCommand>,
        event_tx: Sender<BrowserEvent>,
        delegate: impl BackendDelegate,
    ) {
        let mut dispatcher = DelegateDispatcher::new(delegate);

        // Start the connection and get the IPC client.
        let Some(mut client) = Self::start_connection(&event_tx, &mut dispatcher, &options)
        else {
            return;
        };

        // Initialize communication state and enter the event loop.
        let mut state = CommunicationState {
            resizes_in_flight: HashSet::new(),
            pending_resizes: HashMap::new(),
        };
        const POLL_INTERVAL: Duration = Duration::from_millis(16);

        while Self::poll_event(
            &command_rx,
            &mut client,
            &event_tx,
            &mut state,
            &mut dispatcher,
        ) {
            thread::sleep(POLL_INTERVAL);
        }
    }

    fn process_event_queue(
        client: &mut IpcClient,
        event_tx: &Sender<BrowserEvent>,
        state: &mut CommunicationState,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
    ) -> Option<BackendStopReason> {
        while let Some(event) = client.poll_event() {
            match event {
                Ok(event) => {
                    if let Some(reason) =
                        Self::handle_ipc_event(event, event_tx, client, state, dispatcher)
                    {
                        return Some(reason);
                    }
                }
                Err(err) => {
                    let info = backend_error_event(err);
                    let terminal_hint = backend_error_terminal_hint(info.kind);
                    if let Some(reason) = dispatcher.dispatch_event(
                        BrowserEvent::BackendError {
                            info,
                            terminal_hint,
                        },
                        event_tx,
                    ) {
                        return Some(reason);
                    }
                }
            }
        }

        None
    }

    fn handle_ipc_event(
        event: IpcEvent,
        event_tx: &Sender<BrowserEvent>,
        client: &mut IpcClient,
        state: &mut CommunicationState,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
    ) -> Option<BackendStopReason> {
        let mut emit = |event| dispatcher.dispatch_event(event, event_tx);

        match event {
            IpcEvent::WebContentsCreated {
                profile_id,
                browsing_context_id,
                request_id,
            } => {
                if let Some(reason) = emit(BrowserEvent::BrowsingContext {
                    profile_id,
                    browsing_context_id,
                    event: BrowsingContextEvent::Created { request_id },
                }) {
                    return Some(reason);
                }
            }
            IpcEvent::SurfaceHandleUpdated { .. } => {}
            IpcEvent::WebContentsResizeAcknowledged { browsing_context_id, .. } => {
                state.resizes_in_flight.remove(&browsing_context_id);
                if let Some((width, height)) = state.pending_resizes.remove(&browsing_context_id) {
                    state.resizes_in_flight.insert(browsing_context_id);
                    if let Err(err) = client.set_web_contents_size(browsing_context_id, width, height) {
                        tracing::warn!(
                            result = "err",
                            error = "resize_failed",
                            err = ?err,
                            %browsing_context_id,
                            "Failed to send pending resize"
                        );
                        state.resizes_in_flight.remove(&browsing_context_id);
                    }
                }
            }
            IpcEvent::WebContentsDomHtmlRead {
                profile_id,
                browsing_context_id,
                request_id,
                html,
            } => {
                if let Some(reason) = emit(BrowserEvent::BrowsingContext {
                    profile_id,
                    browsing_context_id,
                    event: BrowsingContextEvent::DomHtmlRead { request_id, html },
                }) {
                    return Some(reason);
                }
            }
            IpcEvent::DragStartRequested {
                profile_id,
                browsing_context_id,
                request,
            } => {
                if let Some(reason) = emit(BrowserEvent::BrowsingContext {
                    profile_id,
                    browsing_context_id,
                    event: BrowsingContextEvent::DragStartRequested { request },
                }) {
                    return Some(reason);
                }
            }
            IpcEvent::ImeBoundsUpdated {
                profile_id,
                browsing_context_id,
                update,
            } => {
                if let Some(reason) = emit(BrowserEvent::BrowsingContext {
                    profile_id,
                    browsing_context_id,
                    event: BrowsingContextEvent::ImeBoundsUpdated { update },
                }) {
                    return Some(reason);
                }
            }
            IpcEvent::ContextMenuRequested {
                profile_id,
                browsing_context_id,
                menu,
            } => {
                if let Some(reason) = emit(BrowserEvent::BrowsingContext {
                    profile_id,
                    browsing_context_id,
                    event: BrowsingContextEvent::ContextMenuRequested { menu },
                }) {
                    return Some(reason);
                }
            }
            IpcEvent::NewWebContentsRequested {
                profile_id,
                browsing_context_id,
                target_url,
                is_popup,
            } => {
                if let Some(reason) = emit(BrowserEvent::BrowsingContext {
                    profile_id,
                    browsing_context_id,
                    event: BrowsingContextEvent::NewBrowsingContextRequested {
                        target_url,
                        is_popup,
                    },
                }) {
                    return Some(reason);
                }
            }
            IpcEvent::NavigationStateChanged {
                profile_id,
                browsing_context_id,
                url,
                can_go_back,
                can_go_forward,
                is_loading,
            } => {
                if let Some(reason) = emit(BrowserEvent::BrowsingContext {
                    profile_id,
                    browsing_context_id,
                    event: BrowsingContextEvent::NavigationStateChanged {
                        url,
                        can_go_back,
                        can_go_forward,
                        is_loading,
                    },
                }) {
                    return Some(reason);
                }
            }
            IpcEvent::CursorChanged {
                profile_id,
                browsing_context_id,
                cursor_type,
            } => {
                if let Some(reason) = emit(BrowserEvent::BrowsingContext {
                    profile_id,
                    browsing_context_id,
                    event: BrowsingContextEvent::CursorChanged { cursor_type },
                }) {
                    return Some(reason);
                }
            }
            IpcEvent::TitleUpdated {
                profile_id,
                browsing_context_id,
                title,
            } => {
                if let Some(reason) = emit(BrowserEvent::BrowsingContext {
                    profile_id,
                    browsing_context_id,
                    event: BrowsingContextEvent::TitleUpdated { title },
                }) {
                    return Some(reason);
                }
            }
            IpcEvent::FaviconUrlUpdated {
                profile_id,
                browsing_context_id,
                url,
            } => {
                if let Some(reason) = emit(BrowserEvent::BrowsingContext {
                    profile_id,
                    browsing_context_id,
                    event: BrowsingContextEvent::FaviconUrlUpdated { url },
                }) {
                    return Some(reason);
                }
            }
            IpcEvent::BeforeUnloadDialogRequested {
                profile_id,
                browsing_context_id,
                request_id,
                reason,
            } => {
                if let Some(reason) = emit(BrowserEvent::BrowsingContext {
                    profile_id,
                    browsing_context_id,
                    event: BrowsingContextEvent::JavaScriptDialogRequested {
                        request_id,
                        message: String::new(),
                        default_prompt_text: None,
                        r#type: DialogType::BeforeUnload,
                        beforeunload_reason: Some(reason),
                    },
                }) {
                    return Some(reason);
                }
            }
            IpcEvent::WebContentsClosed {
                profile_id,
                browsing_context_id,
            } => {
                state.resizes_in_flight.remove(&browsing_context_id);
                state.pending_resizes.remove(&browsing_context_id);
                if let Some(reason) = emit(BrowserEvent::BrowsingContext {
                    profile_id,
                    browsing_context_id,
                    event: BrowsingContextEvent::Closed,
                }) {
                    return Some(reason);
                }
            }
            IpcEvent::ShutdownBlocked {
                request_id,
                dirty_browsing_context_ids,
            } => {
                if let Some(reason) = emit(BrowserEvent::ShutdownBlocked {
                    request_id,
                    dirty_browsing_context_ids,
                }) {
                    return Some(reason);
                }
            }
            IpcEvent::ShutdownProceeding { request_id } => {
                if let Some(reason) = emit(BrowserEvent::ShutdownProceeding { request_id }) {
                    return Some(reason);
                }
            }
            IpcEvent::ShutdownCancelled { request_id } => {
                if let Some(reason) = emit(BrowserEvent::ShutdownCancelled { request_id }) {
                    return Some(reason);
                }
            }
        }

        None
    }

    fn process_command_queue(
        command_rx: &Receiver<BrowserCommand>,
        client: &mut IpcClient,
        event_tx: &Sender<BrowserEvent>,
        state: &mut CommunicationState,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
    ) -> Option<BackendStopReason> {
        let mut pending_command: Option<BrowserCommand> = None;

        loop {
            let command = match pending_command.take() {
                Some(command) => command,
                None => match command_rx.try_recv() {
                    Ok(command) => command,
                    Err(TryRecvError::Empty) => break None,
                    Err(TryRecvError::Closed) => break Some(BackendStopReason::Disconnected),
                },
            };

            if let BrowserCommand::ResizeBrowsingContext {
                browsing_context_id,
                width,
                height,
            } = command
            {
                let mut latest_resizes: HashMap<BrowsingContextId, (u32, u32)> = HashMap::new();
                latest_resizes.insert(browsing_context_id, (width, height));

                loop {
                    match command_rx.try_recv() {
                        Ok(BrowserCommand::ResizeBrowsingContext {
                            browsing_context_id,
                            width,
                            height,
                        }) => {
                            latest_resizes.insert(browsing_context_id, (width, height));
                        }
                        Ok(other) => {
                            pending_command = Some(other);
                            break;
                        }
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Closed) => return Some(BackendStopReason::Disconnected),
                    }
                }

                for (browsing_context_id, (width, height)) in latest_resizes {
                    if state.resizes_in_flight.contains(&browsing_context_id) {
                        state.pending_resizes.insert(browsing_context_id, (width, height));
                        continue;
                    }

                    state.resizes_in_flight.insert(browsing_context_id);
                    let mut forward = |command| Self::execute_command(command, client, event_tx);
                    if let Some(reason) = dispatcher.dispatch_command(
                        BrowserCommand::ResizeBrowsingContext {
                            browsing_context_id,
                            width,
                            height,
                        },
                        event_tx,
                        &mut forward,
                    ) {
                        return Some(reason);
                    }
                }

                continue;
            }

            let mut forward = |command| Self::execute_command(command, client, event_tx);
            if let Some(reason) = dispatcher.dispatch_command(command, event_tx, &mut forward) {
                return Some(reason);
            }
        }
    }

    fn execute_command(
        command: BrowserCommand,
        client: &mut IpcClient,
        event_tx: &Sender<BrowserEvent>,
    ) -> (Option<BackendStopReason>, Vec<BrowserEvent>) {
        match Self::handle_command(command, client, event_tx) {
            Ok(Some(reason)) => (Some(reason), Vec::new()),
            Ok(None) => (None, Vec::new()),
            Err(err) => {
                let info = err.into_backend_error_info();
                let terminal_hint = backend_error_terminal_hint(info.kind);
                (
                    None,
                    vec![BrowserEvent::BackendError {
                        info,
                        terminal_hint,
                    }],
                )
            }
        }
    }

    fn handle_command(
        command: BrowserCommand,
        client: &mut IpcClient,
        event_tx: &Sender<BrowserEvent>,
    ) -> Result<Option<BackendStopReason>, CommandExecutionError> {
        match command {
            BrowserCommand::Shutdown { request_id } => client
                .request_shutdown(request_id)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::Shutdown,
                    source,
                }),
            BrowserCommand::ConfirmShutdown {
                request_id,
                proceed,
            } => client
                .confirm_shutdown(request_id, proceed)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::ConfirmShutdown,
                    source,
                }),
            BrowserCommand::ForceShutdown => {
                client.force_shutdown().map(|_| None).map_err(|source| {
                    CommandExecutionError::IpcCall {
                        operation: Operation::ForceShutdown,
                        source,
                    }
                })
            }
            BrowserCommand::ConfirmBeforeUnload {
                browsing_context_id,
                request_id,
                proceed,
            } => client
                .confirm_beforeunload(browsing_context_id, request_id, proceed)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::ConfirmBeforeUnload,
                    source,
                }),
            BrowserCommand::ConfirmPermission { .. } => Ok(None),
            BrowserCommand::CreateBrowsingContext {
                request_id,
                initial_url,
                profile_id,
            } => {
                let url = initial_url.unwrap_or_else(|| "about:blank".to_string());
                let profile = profile_id.unwrap_or_default();

                client
                    .create_web_contents(request_id, &url, &profile)
                    .map(|_| None)
                    .map_err(|source| CommandExecutionError::IpcCall {
                        operation: Operation::CreateBrowsingContext,
                        source,
                    })
            }
            BrowserCommand::ResizeBrowsingContext {
                browsing_context_id,
                width,
                height,
            } => client
                .set_web_contents_size(browsing_context_id, width, height)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::ResizeBrowsingContext,
                    source,
                }),
            BrowserCommand::ListProfiles => match client.list_profiles() {
                Ok(profiles) => {
                    _ = event_tx.send_blocking(BrowserEvent::ProfilesListed { profiles });

                    Ok(None)
                }
                Err(source) => Err(CommandExecutionError::IpcCall {
                    operation: Operation::ListProfiles,
                    source,
                }),
            },
            BrowserCommand::SendKeyEvent {
                browsing_context_id,
                event,
                commands,
            } => client
                .send_key_event(browsing_context_id, &event, &commands)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::SendKeyEvent,
                    source,
                }),
            BrowserCommand::SendMouseEvent { browsing_context_id, event } => client
                .send_mouse_event(browsing_context_id, &event)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::SendMouseEvent,
                    source,
                }),
            BrowserCommand::SendMouseWheelEvent { browsing_context_id, event } => client
                .send_mouse_wheel_event(browsing_context_id, &event)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::SendMouseWheelEvent,
                    source,
                }),
            BrowserCommand::SendDragUpdate { update } => client
                .send_drag_update(&update)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::SendDragUpdate,
                    source,
                }),
            BrowserCommand::SendDragDrop { drop } => client
                .send_drag_drop(&drop)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::SendDragDrop,
                    source,
                }),
            BrowserCommand::SendDragCancel {
                session_id,
                browsing_context_id,
            } => client
                .send_drag_cancel(session_id, browsing_context_id)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::SendDragCancel,
                    source,
                }),
            BrowserCommand::SetComposition { composition } => client
                .set_composition(&composition)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::SetComposition,
                    source,
                }),
            BrowserCommand::CommitText { commit } => client
                .commit_text(&commit)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::CommitText,
                    source,
                }),
            BrowserCommand::FinishComposingText {
                browsing_context_id,
                behavior,
            } => client
                .finish_composing_text(browsing_context_id, behavior)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::FinishComposingText,
                    source,
                }),
            BrowserCommand::ExecuteContextMenuCommand {
                menu_id,
                command_id,
                event_flags,
            } => client
                .execute_context_menu_command(menu_id, command_id, event_flags)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::ExecuteContextMenuCommand,
                    source,
                }),
            BrowserCommand::DismissContextMenu { menu_id } => client
                .dismiss_context_menu(menu_id)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::DismissContextMenu,
                    source,
                }),
            BrowserCommand::RequestCloseBrowsingContext { browsing_context_id } => client
                .request_close_web_contents(browsing_context_id)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::RequestCloseBrowsingContext,
                    source,
                }),
            BrowserCommand::Navigate { browsing_context_id, url } => client
                .navigate(browsing_context_id, &url)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::Navigate,
                    source,
                }),
            BrowserCommand::GoBack { browsing_context_id } => client
                .go_back(browsing_context_id)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::GoBack,
                    source,
                }),
            BrowserCommand::GoForward { browsing_context_id } => client
                .go_forward(browsing_context_id)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::GoForward,
                    source,
                }),
            BrowserCommand::Reload {
                browsing_context_id,
                ignore_cache,
            } => client
                .reload(browsing_context_id, ignore_cache)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::Reload,
                    source,
                }),
            BrowserCommand::GetBrowsingContextDomHtml {
                browsing_context_id,
                request_id,
            } => client
                .get_web_contents_dom_html(browsing_context_id, request_id)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::GetBrowsingContextDomHtml,
                    source,
                }),
            BrowserCommand::SetBrowsingContextFocus {
                browsing_context_id,
                focused,
            } => client
                .set_web_contents_focus(browsing_context_id, focused)
                .map(|_| None)
                .map_err(|source| CommandExecutionError::IpcCall {
                    operation: Operation::SetBrowsingContextFocus,
                    source,
                }),
        }
    }
}
