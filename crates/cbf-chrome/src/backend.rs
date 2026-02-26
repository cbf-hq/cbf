use std::{
    thread,
    time::{Duration, Instant},
};

use async_channel::{Receiver, Sender, TryRecvError};
use cbf::{
    browser::{Backend, CommandEnvelope, CommandSender, EventStream},
    command::{BrowserCommand, BrowserOperation},
    delegate::{BackendDelegate, CommandDecision, DelegateDispatcher, EventDecision},
    error::{ApiErrorKind, BackendErrorInfo, Error},
    event::{BackendStopReason, BrowserEvent},
};

use crate::{
    command::ChromeCommand,
    event::{ChromeEvent, to_generic_event},
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

#[derive(Debug)]
enum CommandExecutionError {
    IpcCall {
        operation: Option<BrowserOperation>,
        source: IpcError,
    },
}

impl CommandExecutionError {
    fn from_ipc_call(operation: Option<BrowserOperation>, source: IpcError) -> Self {
        Self::IpcCall { operation, source }
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

/// Decision returned from [`ChromeRawDelegate::on_raw_command`].
#[derive(Debug)]
pub enum RawCommandDecision {
    /// Forward the raw command to Chromium transport.
    Forward,
    /// Drop the raw command and continue processing.
    Drop,
    /// Stop backend processing with the given reason.
    Stop(BackendStopReason),
}

/// Hook-based interface for mediating Chromium raw command flow.
pub trait ChromeRawDelegate: Send + 'static {
    /// Called for each raw command sent through `send_raw`.
    fn on_raw_command(&mut self, _command: &ChromeCommand) -> RawCommandDecision {
        RawCommandDecision::Forward
    }
}

#[derive(Debug, Default)]
struct NoopRawDelegate;

impl ChromeRawDelegate for NoopRawDelegate {}

impl Backend for ChromiumBackend {
    type RawCommand = ChromeCommand;
    type RawEvent = ChromeEvent;
    type RawDelegate = Box<dyn ChromeRawDelegate>;

    fn to_raw_command(command: BrowserCommand) -> Self::RawCommand {
        command.into()
    }

    fn to_generic_event(raw: &Self::RawEvent) -> Option<BrowserEvent> {
        to_generic_event(raw)
    }

    fn connect<D: BackendDelegate>(
        self,
        delegate: D,
        raw_delegate: Option<Self::RawDelegate>,
    ) -> Result<(CommandSender<Self>, EventStream<Self>), Error> {
        let (command_tx, command_rx) = async_channel::unbounded::<CommandEnvelope<Self>>();
        let (event_tx, event_rx) = async_channel::unbounded::<ChromeEvent>();
        let options = self.options;
        let raw_delegate = raw_delegate.unwrap_or_else(|| Box::<NoopRawDelegate>::default());

        thread::spawn(move || {
            Self::run_communication(options, command_rx, event_tx, delegate, raw_delegate)
        });

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

    fn run_communication(
        options: ChromiumBackendOptions,
        command_rx: Receiver<CommandEnvelope<Self>>,
        event_tx: Sender<ChromeEvent>,
        delegate: impl BackendDelegate,
        mut raw_delegate: Box<dyn ChromeRawDelegate>,
    ) {
        let mut dispatcher = DelegateDispatcher::new(delegate);

        // Start the connection and get the IPC client.
        let Some(mut client) = Self::start_connection(&event_tx, &mut dispatcher, &options) else {
            return;
        };

        const POLL_INTERVAL: Duration = Duration::from_millis(16);

        while Self::poll_event(
            &command_rx,
            &mut client,
            &event_tx,
            &mut dispatcher,
            raw_delegate.as_mut(),
        ) {
            thread::sleep(POLL_INTERVAL);
        }
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

    fn poll_event(
        command_rx: &Receiver<CommandEnvelope<Self>>,
        client: &mut IpcClient,
        event_tx: &Sender<ChromeEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
        raw_delegate: &mut dyn ChromeRawDelegate,
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
            Self::process_command_queue(command_rx, client, event_tx, dispatcher, raw_delegate)
        {
            Self::stop_backend(stop_reason, dispatcher, Some(client), event_tx);
            return false;
        };

        if let Some(stop_reason) = Self::process_event_queue(client, event_tx, dispatcher) {
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
        raw_command: ChromeCommand,
        client: &mut IpcClient,
        event_tx: &Sender<ChromeEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
    ) -> Option<BackendStopReason> {
        match dispatcher.dispatch_command(&command) {
            CommandDecision::Forward => {
                let operation = Some(BrowserOperation::from_command(&command));
                let (reason, events) = Self::execute_raw_command(raw_command, operation, client);
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
        operation: Option<BrowserOperation>,
        client: &mut IpcClient,
        event_tx: &Sender<ChromeEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
    ) -> Option<BackendStopReason> {
        let (reason, events) = Self::execute_raw_command(command, operation, client);
        for event in events {
            if let Some(reason) =
                Self::handle_raw_event_with_delegate_gate(dispatcher, event_tx, event)
            {
                return Some(reason);
            }
        }
        reason
    }

    fn run_raw_command_with_raw_delegate(
        command: ChromeCommand,
        client: &mut IpcClient,
        event_tx: &Sender<ChromeEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
        raw_delegate: &mut dyn ChromeRawDelegate,
    ) -> Option<BackendStopReason> {
        match raw_delegate.on_raw_command(&command) {
            RawCommandDecision::Forward => {
                Self::run_raw_command(command, None, client, event_tx, dispatcher)
            }
            RawCommandDecision::Drop => None,
            RawCommandDecision::Stop(reason) => Some(reason),
        }
    }

    fn dispatch_command_envelope(
        envelope: CommandEnvelope<Self>,
        client: &mut IpcClient,
        event_tx: &Sender<ChromeEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
        raw_delegate: &mut dyn ChromeRawDelegate,
    ) -> Option<BackendStopReason> {
        match envelope {
            CommandEnvelope::Generic { command, raw } => {
                Self::run_generic_command_with_delegate(command, raw, client, event_tx, dispatcher)
            }
            CommandEnvelope::RawOnly { raw } => Self::run_raw_command_with_raw_delegate(
                raw,
                client,
                event_tx,
                dispatcher,
                raw_delegate,
            ),
        }
    }

    fn drain_delegate_queue(
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
        client: &mut IpcClient,
        event_tx: &Sender<ChromeEvent>,
        mut pending_commands: Vec<BrowserCommand>,
    ) -> Option<BackendStopReason> {
        loop {
            for command in pending_commands {
                let raw_command = Self::to_raw_command(command.clone());
                if let Some(reason) = Self::run_generic_command_with_delegate(
                    command,
                    raw_command,
                    client,
                    event_tx,
                    dispatcher,
                ) {
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
        if let Some(client) = client
            && let Some(reason) =
                Self::drain_delegate_queue(dispatcher, client, event_tx, queued_commands)
        {
            final_reason = reason;
        }
        Self::emit_raw_event(
            event_tx,
            ChromeEvent::BackendStopped {
                reason: final_reason,
            },
        );
    }

    fn process_event_queue(
        client: &mut IpcClient,
        event_tx: &Sender<ChromeEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
    ) -> Option<BackendStopReason> {
        while let Some(event) = client.poll_event() {
            match event {
                Ok(event) => {
                    if let Some(reason) = Self::handle_ipc_event(event, event_tx, dispatcher) {
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
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
    ) -> Option<BackendStopReason> {
        Self::handle_raw_event_with_delegate_gate(
            dispatcher,
            event_tx,
            ChromeEvent::Ipc(Box::new(event)),
        )
    }

    fn process_command_queue(
        command_rx: &Receiver<CommandEnvelope<Self>>,
        client: &mut IpcClient,
        event_tx: &Sender<ChromeEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
        raw_delegate: &mut dyn ChromeRawDelegate,
    ) -> Option<BackendStopReason> {
        loop {
            let envelope = match command_rx.try_recv() {
                Ok(envelope) => envelope,
                Err(TryRecvError::Empty) => break None,
                Err(TryRecvError::Closed) => break Some(BackendStopReason::Disconnected),
            };

            if let Some(reason) = Self::dispatch_command_envelope(
                envelope,
                client,
                event_tx,
                dispatcher,
                raw_delegate,
            ) {
                return Some(reason);
            }
        }
    }

    fn execute_raw_command(
        command: ChromeCommand,
        operation: Option<BrowserOperation>,
        client: &mut IpcClient,
    ) -> (Option<BackendStopReason>, Vec<ChromeEvent>) {
        match Self::handle_command(command, operation, client) {
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
        operation: Option<BrowserOperation>,
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
            ChromeCommand::ListExtensions { profile_id } => {
                client.list_extensions(profile_id).map(|extensions| {
                    (
                        None,
                        vec![ChromeEvent::Ipc(Box::new(IpcEvent::ExtensionsListed {
                            profile_id: profile_id.clone().unwrap_or_default(),
                            extensions,
                        }))],
                    )
                })
            }
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
            ChromeCommand::PrintPreview {
                browsing_context_id,
            } => client
                .print_preview(*browsing_context_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::OpenDevTools {
                browsing_context_id,
            } => client
                .open_dev_tools(*browsing_context_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::InspectElement {
                browsing_context_id,
                x,
                y,
            } => client
                .inspect_element(*browsing_context_id, *x, *y)
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
            ChromeCommand::OpenDefaultAuxiliaryWindow {
                browsing_context_id,
                request_id,
            } => client
                .open_default_auxiliary_window(*browsing_context_id, *request_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::RespondAuxiliaryWindow {
                browsing_context_id,
                request_id,
                response,
            } => client
                .respond_auxiliary_window(*browsing_context_id, *request_id, response)
                .map(|_| (None, Vec::new())),
            ChromeCommand::CloseAuxiliaryWindow {
                browsing_context_id,
                window_id,
            } => client
                .close_auxiliary_window(*browsing_context_id, *window_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::RespondBrowsingContextOpen {
                request_id,
                response,
            } => client
                .respond_browsing_context_open(*request_id, response)
                .map(|_| (None, Vec::new())),
            ChromeCommand::RespondWindowOpen {
                request_id,
                response,
            } => client
                .respond_window_open(*request_id, response)
                .map(|_| (None, Vec::new())),
        };

        result.map_err(|source| CommandExecutionError::from_ipc_call(operation, source))
    }
}
