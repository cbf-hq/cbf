use std::{
    collections::{HashMap, HashSet},
    thread,
    time::{Duration, Instant},
};

use async_channel::{Receiver, Sender, TryRecvError};

use cbf::{
    backend_delegate::{BackendDelegate, CommandDecision, DelegateDispatcher, EventDecision},
    browser::{Backend, CommandSender, EventStream},
    command::BrowserCommand,
    data::ids::BrowsingContextId,
    error::{ApiErrorKind, BackendErrorInfo, Error, Operation},
    event::{BackendStopReason, BrowserEvent},
};

use crate::{
    command::ChromeCommand,
    event::{to_generic_event, ChromeEvent},
    ffi::{Error as IpcError, IpcClient, IpcEvent},
};

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
        operation: Option<Operation>,
        source: IpcError,
    },
}

impl CommandExecutionError {
    fn from_ipc_call(command: &ChromeCommand, source: IpcError) -> Self {
        Self::IpcCall {
            operation: operation_from_raw_command(command),
            source,
        }
    }

    fn into_backend_error_info(self) -> BackendErrorInfo {
        match self {
            Self::IpcCall { operation, source } => BackendErrorInfo {
                kind: match source {
                    IpcError::ConnectionFailed => ApiErrorKind::CommandDispatchFailed,
                    IpcError::InvalidInput => ApiErrorKind::InvalidInput,
                    IpcError::InvalidEvent => ApiErrorKind::ProtocolMismatch,
                },
                operation,
                detail: Some(format!("{source:?}")),
            },
        }
    }
}

fn operation_from_raw_command(command: &ChromeCommand) -> Option<Operation> {
    command
        .to_browser_command()
        .as_ref()
        .map(Operation::from_command)
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
    type RawCommand = ChromeCommand;
    type RawEvent = ChromeEvent;
    type RawDelegate = ();

    fn to_raw_command(command: BrowserCommand) -> Self::RawCommand {
        command.into()
    }

    fn to_generic_event(raw: &Self::RawEvent) -> Option<BrowserEvent> {
        to_generic_event(raw)
    }

    fn connect<D: BackendDelegate>(
        self,
        delegate: D,
        _raw_delegate: Option<Self::RawDelegate>,
    ) -> Result<(CommandSender<Self>, EventStream<Self>), Error> {
        let (command_tx, command_rx) = async_channel::unbounded::<ChromeCommand>();
        let (event_tx, event_rx) = async_channel::unbounded::<ChromeEvent>();
        let options = self.options;

        thread::spawn(move || Self::run_communication(options, command_rx, event_tx, delegate));

        Ok((
            CommandSender::from_raw_sender(command_tx),
            EventStream::from_raw_receiver(event_rx),
        ))
    }
}

impl ChromiumBackend {
    /// Create a backend from Chromium IPC connection options.
    pub fn new(options: ChromiumBackendOptions) -> Self {
        Self { options }
    }

    fn emit_raw_event(event_tx: &Sender<ChromeEvent>, event: ChromeEvent) {
        _ = event_tx.send_blocking(event);
    }

    fn handle_raw_event_with_delegate_gate(
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
        event_tx: &Sender<ChromeEvent>,
        event: ChromeEvent,
    ) -> Option<BackendStopReason> {
        if let Some(generic_event) = Self::to_generic_event(&event) {
            match dispatcher.dispatch_event(&generic_event) {
                EventDecision::Forward => {
                    Self::emit_raw_event(event_tx, event);
                    None
                }
                EventDecision::Stop(reason) => Some(reason),
            }
        } else {
            Self::emit_raw_event(event_tx, event);
            None
        }
    }

    fn run_generic_command_with_delegate(
        command: BrowserCommand,
        client: &mut IpcClient,
        event_tx: &Sender<ChromeEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
    ) -> Option<BackendStopReason> {
        match dispatcher.dispatch_command(&command) {
            CommandDecision::Forward => {
                let raw_command = Self::to_raw_command(command);
                let (reason, events) = Self::execute_raw_command(raw_command, client);
                for event in events {
                    if let Some(reason) =
                        Self::handle_raw_event_with_delegate_gate(dispatcher, event_tx, event)
                    {
                        return Some(reason);
                    }
                }
                reason
            }
            CommandDecision::Drop => None,
            CommandDecision::Stop(reason) => Some(reason),
        }
    }

    fn run_raw_command(
        command: ChromeCommand,
        client: &mut IpcClient,
        event_tx: &Sender<ChromeEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
    ) -> Option<BackendStopReason> {
        let (reason, events) = Self::execute_raw_command(command, client);
        for event in events {
            if let Some(reason) =
                Self::handle_raw_event_with_delegate_gate(dispatcher, event_tx, event)
            {
                return Some(reason);
            }
        }
        reason
    }

    fn drain_delegate_queue(
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
        client: &mut IpcClient,
        event_tx: &Sender<ChromeEvent>,
        mut pending_commands: Vec<BrowserCommand>,
    ) -> Option<BackendStopReason> {
        loop {
            for command in pending_commands {
                if let Some(reason) =
                    Self::run_generic_command_with_delegate(command, client, event_tx, dispatcher)
                {
                    return Some(reason);
                }
            }

            pending_commands = dispatcher.flush();
            if pending_commands.is_empty() {
                return None;
            }
        }
    }

    fn stop_backend(
        reason: BackendStopReason,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
        client: Option<&mut IpcClient>,
        event_tx: &Sender<ChromeEvent>,
    ) {
        let (mut final_reason, queued_commands) = dispatcher.stop(reason);
        if let Some(client) = client {
            if let Some(reason) =
                Self::drain_delegate_queue(dispatcher, client, event_tx, queued_commands)
            {
                final_reason = reason;
            }
        }
        Self::emit_raw_event(
            event_tx,
            ChromeEvent::BackendStopped {
                reason: final_reason,
            },
        );
    }

    fn start_connection(
        event_tx: &Sender<ChromeEvent>,
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
                        let stop_reason = Self::handle_raw_event_with_delegate_gate(
                            dispatcher,
                            event_tx,
                            ChromeEvent::BackendError {
                                terminal_hint: true,
                                info: info.clone(),
                            },
                        )
                        .unwrap_or(BackendStopReason::Error(info));
                        Self::stop_backend(stop_reason, dispatcher, None, event_tx);
                        return None;
                    }

                    thread::sleep(options.retry_interval);
                }
            }
        };

        // Notify that the backend is ready after establishing the connection.
        if let Some(stop_reason) = Self::handle_raw_event_with_delegate_gate(
            dispatcher,
            event_tx,
            ChromeEvent::BackendReady,
        ) {
            Self::stop_backend(stop_reason, dispatcher, Some(&mut client), event_tx);
            return None;
        }

        Some(client)
    }

    fn dispatch_raw_command(
        command: ChromeCommand,
        client: &mut IpcClient,
        event_tx: &Sender<ChromeEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
    ) -> Option<BackendStopReason> {
        if let Some(command) = command.to_browser_command() {
            return Self::run_generic_command_with_delegate(command, client, event_tx, dispatcher);
        }

        Self::run_raw_command(command, client, event_tx, dispatcher)
    }

    fn poll_event(
        command_rx: &Receiver<ChromeCommand>,
        client: &mut IpcClient,
        event_tx: &Sender<ChromeEvent>,
        state: &mut CommunicationState,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
    ) -> bool {
        dispatcher.on_idle();

        let queued_commands = dispatcher.flush();
        if let Some(stop_reason) =
            Self::drain_delegate_queue(dispatcher, client, event_tx, queued_commands)
        {
            Self::stop_backend(stop_reason, dispatcher, Some(client), event_tx);
            return false;
        }

        if let Some(stop_reason) =
            Self::process_command_queue(command_rx, client, event_tx, state, dispatcher)
        {
            Self::stop_backend(stop_reason, dispatcher, Some(client), event_tx);
            return false;
        };

        if let Some(stop_reason) = Self::process_event_queue(client, event_tx, state, dispatcher) {
            Self::stop_backend(stop_reason, dispatcher, Some(client), event_tx);
            return false;
        };

        let queued_commands = dispatcher.flush();
        if let Some(stop_reason) =
            Self::drain_delegate_queue(dispatcher, client, event_tx, queued_commands)
        {
            Self::stop_backend(stop_reason, dispatcher, Some(client), event_tx);
            return false;
        }

        true
    }

    fn run_communication(
        options: ChromiumBackendOptions,
        command_rx: Receiver<ChromeCommand>,
        event_tx: Sender<ChromeEvent>,
        delegate: impl BackendDelegate,
    ) {
        let mut dispatcher = DelegateDispatcher::new(delegate);

        // Start the connection and get the IPC client.
        let Some(mut client) = Self::start_connection(&event_tx, &mut dispatcher, &options) else {
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
        event_tx: &Sender<ChromeEvent>,
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
                    if let Some(reason) = Self::handle_raw_event_with_delegate_gate(
                        dispatcher,
                        event_tx,
                        ChromeEvent::BackendError {
                            info,
                            terminal_hint,
                        },
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
        event_tx: &Sender<ChromeEvent>,
        client: &mut IpcClient,
        state: &mut CommunicationState,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
    ) -> Option<BackendStopReason> {
        match &event {
            IpcEvent::WebContentsResizeAcknowledged {
                browsing_context_id,
                ..
            } => {
                state.resizes_in_flight.remove(browsing_context_id);
                if let Some((width, height)) = state.pending_resizes.remove(browsing_context_id) {
                    state.resizes_in_flight.insert(*browsing_context_id);
                    if let Some(reason) = Self::dispatch_raw_command(
                        ChromeCommand::SetWebContentsSize {
                            browsing_context_id: *browsing_context_id,
                            width,
                            height,
                        },
                        client,
                        event_tx,
                        dispatcher,
                    ) {
                        return Some(reason);
                    }
                }
            }
            IpcEvent::WebContentsClosed {
                browsing_context_id,
                ..
            } => {
                state.resizes_in_flight.remove(browsing_context_id);
                state.pending_resizes.remove(browsing_context_id);
            }
            _ => {}
        }

        Self::handle_raw_event_with_delegate_gate(dispatcher, event_tx, ChromeEvent::Ipc(event))
    }

    fn process_command_queue(
        command_rx: &Receiver<ChromeCommand>,
        client: &mut IpcClient,
        event_tx: &Sender<ChromeEvent>,
        state: &mut CommunicationState,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
    ) -> Option<BackendStopReason> {
        let mut pending_command: Option<ChromeCommand> = None;

        loop {
            let command = match pending_command.take() {
                Some(command) => command,
                None => match command_rx.try_recv() {
                    Ok(command) => command,
                    Err(TryRecvError::Empty) => break None,
                    Err(TryRecvError::Closed) => break Some(BackendStopReason::Disconnected),
                },
            };

            if let ChromeCommand::SetWebContentsSize {
                browsing_context_id,
                width,
                height,
            } = command
            {
                let mut latest_resizes: HashMap<BrowsingContextId, (u32, u32)> = HashMap::new();
                latest_resizes.insert(browsing_context_id, (width, height));

                loop {
                    match command_rx.try_recv() {
                        Ok(ChromeCommand::SetWebContentsSize {
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
                        state
                            .pending_resizes
                            .insert(browsing_context_id, (width, height));
                        continue;
                    }

                    state.resizes_in_flight.insert(browsing_context_id);
                    if let Some(reason) = Self::dispatch_raw_command(
                        ChromeCommand::SetWebContentsSize {
                            browsing_context_id,
                            width,
                            height,
                        },
                        client,
                        event_tx,
                        dispatcher,
                    ) {
                        return Some(reason);
                    }
                }

                continue;
            }

            if let Some(reason) = Self::dispatch_raw_command(command, client, event_tx, dispatcher)
            {
                return Some(reason);
            }
        }
    }

    fn execute_raw_command(
        command: ChromeCommand,
        client: &mut IpcClient,
    ) -> (Option<BackendStopReason>, Vec<ChromeEvent>) {
        match Self::handle_command(command, client) {
            Ok((reason, events)) => (reason, events),
            Err(err) => {
                let info = err.into_backend_error_info();
                let terminal_hint = backend_error_terminal_hint(info.kind);
                (
                    None,
                    vec![ChromeEvent::BackendError {
                        info,
                        terminal_hint,
                    }],
                )
            }
        }
    }

    fn handle_command(
        command: ChromeCommand,
        client: &mut IpcClient,
    ) -> Result<(Option<BackendStopReason>, Vec<ChromeEvent>), CommandExecutionError> {
        let result = match &command {
            ChromeCommand::RequestShutdown { request_id } => client
                .request_shutdown(*request_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::ConfirmShutdown {
                request_id,
                proceed,
            } => client
                .confirm_shutdown(*request_id, *proceed)
                .map(|_| (None, Vec::new())),
            ChromeCommand::ForceShutdown => client.force_shutdown().map(|_| (None, Vec::new())),
            ChromeCommand::ConfirmBeforeUnload {
                browsing_context_id,
                request_id,
                proceed,
            } => client
                .confirm_beforeunload(*browsing_context_id, *request_id, *proceed)
                .map(|_| (None, Vec::new())),
            ChromeCommand::ConfirmPermission { .. } => Ok((None, Vec::new())),
            ChromeCommand::CreateWebContents {
                request_id,
                initial_url,
                profile_id,
            } => {
                let url = initial_url
                    .clone()
                    .unwrap_or_else(|| "about:blank".to_string());
                let profile = profile_id.clone().unwrap_or_default();

                client
                    .create_web_contents(*request_id, &url, &profile)
                    .map(|_| (None, Vec::new()))
            }
            ChromeCommand::SetWebContentsSize {
                browsing_context_id,
                width,
                height,
            } => client
                .set_web_contents_size(*browsing_context_id, *width, *height)
                .map(|_| (None, Vec::new())),
            ChromeCommand::ListProfiles => client
                .list_profiles()
                .map(|profiles| (None, vec![ChromeEvent::ProfilesListed { profiles }])),
            ChromeCommand::SendKeyEvent {
                browsing_context_id,
                event,
                commands,
            } => client
                .send_key_event(*browsing_context_id, &event.clone().into(), commands)
                .map(|_| (None, Vec::new())),
            ChromeCommand::SendMouseEvent {
                browsing_context_id,
                event,
            } => client
                .send_mouse_event(*browsing_context_id, event)
                .map(|_| (None, Vec::new())),
            ChromeCommand::SendMouseWheelEvent {
                browsing_context_id,
                event,
            } => client
                .send_mouse_wheel_event(*browsing_context_id, &event.clone().into())
                .map(|_| (None, Vec::new())),
            ChromeCommand::SendDragUpdate { update } => {
                client.send_drag_update(update).map(|_| (None, Vec::new()))
            }
            ChromeCommand::SendDragDrop { drop } => {
                client.send_drag_drop(drop).map(|_| (None, Vec::new()))
            }
            ChromeCommand::SendDragCancel {
                session_id,
                browsing_context_id,
            } => client
                .send_drag_cancel(*session_id, *browsing_context_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::SetImeComposition { composition } => client
                .set_composition(composition)
                .map(|_| (None, Vec::new())),
            ChromeCommand::CommitImeText { commit } => {
                client.commit_text(commit).map(|_| (None, Vec::new()))
            }
            ChromeCommand::FinishComposingText {
                browsing_context_id,
                behavior,
            } => client
                .finish_composing_text(*browsing_context_id, *behavior)
                .map(|_| (None, Vec::new())),
            ChromeCommand::ExecuteContextMenuCommand {
                menu_id,
                command_id,
                event_flags,
            } => client
                .execute_context_menu_command(*menu_id, *command_id, *event_flags)
                .map(|_| (None, Vec::new())),
            ChromeCommand::DismissContextMenu { menu_id } => client
                .dismiss_context_menu(*menu_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::RequestCloseWebContents {
                browsing_context_id,
            } => client
                .request_close_web_contents(*browsing_context_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::Navigate {
                browsing_context_id,
                url,
            } => client
                .navigate(*browsing_context_id, url)
                .map(|_| (None, Vec::new())),
            ChromeCommand::GoBack {
                browsing_context_id,
            } => client
                .go_back(*browsing_context_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::GoForward {
                browsing_context_id,
            } => client
                .go_forward(*browsing_context_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::Reload {
                browsing_context_id,
                ignore_cache,
            } => client
                .reload(*browsing_context_id, *ignore_cache)
                .map(|_| (None, Vec::new())),
            ChromeCommand::GetWebContentsDomHtml {
                browsing_context_id,
                request_id,
            } => client
                .get_web_contents_dom_html(*browsing_context_id, *request_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::SetWebContentsFocus {
                browsing_context_id,
                focused,
            } => client
                .set_web_contents_focus(*browsing_context_id, *focused)
                .map(|_| (None, Vec::new())),
        };

        result.map_err(|source| CommandExecutionError::from_ipc_call(&command, source))
    }
}
