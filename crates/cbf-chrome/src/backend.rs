//! Chromium IPC backend integration for `cbf-chrome`.
//!
//! This module implements [`ChromiumBackend`], the `cbf` backend adapter that
//! drives command dispatch and event processing over the Chromium bridge. It
//! translates generic `cbf` backend flow into Chrome-aware transport behavior
//! while keeping browser-generic API vocabulary above this layer.

use std::{
    collections::VecDeque,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use async_channel::{Receiver, Sender};
use cbf::{
    backend_event_loop::{BackendEventLoop, BackendWake},
    browser::{Backend, CommandEnvelope, CommandSender, EventStream},
    command::{BrowserCommand, BrowserOperation},
    data::dialog::DialogResponse,
    delegate::{BackendDelegate, CommandDecision, DelegateDispatcher, EventDecision},
    error::{ApiErrorKind, BackendErrorInfo, Error},
    event::{BackendStopReason, BrowserEvent},
};

use crate::{
    bridge::{BridgeError as IpcError, EventWaitResult, IpcClient, IpcEvent, IpcEventWaitHandle},
    command::ChromeCommand,
    data::{custom_scheme::ChromeCustomSchemeRegistration, prompt_ui::PromptUiResponse},
    event::{ChromeEvent, to_generic_event},
};

/// Backend implementation that speaks the Chromium IPC protocol.
#[derive(Debug)]
pub struct ChromiumBackend {
    options: ChromiumBackendOptions,
    client: IpcClient,
}

/// Options for controlling the Chromium backend.
#[derive(Debug, Default, Clone)]
pub struct ChromiumBackendOptions {
    pub custom_scheme_registrations: Vec<ChromeCustomSchemeRegistration>,
}

impl ChromiumBackendOptions {
    /// Create default backend options.
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug)]
enum CommandExecutionError {
    IpcCall {
        operation: Option<BrowserOperation>,
        source: IpcError,
    },
    Unsupported {
        operation: BrowserOperation,
        detail: &'static str,
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
                    IpcError::BridgeLoadFailed => ApiErrorKind::CommandDispatchFailed,
                    IpcError::ConnectionFailed => ApiErrorKind::CommandDispatchFailed,
                    IpcError::InvalidInput => ApiErrorKind::InvalidInput,
                    IpcError::InvalidEvent => ApiErrorKind::ProtocolMismatch,
                },
                operation,
                detail: Some(format!("{source:?}")),
            },
            Self::Unsupported { operation, detail } => BackendErrorInfo {
                kind: ApiErrorKind::Unsupported,
                operation: Some(operation),
                detail: Some(detail.to_string()),
            },
        }
    }
}

fn backend_error_event(source: IpcError) -> BackendErrorInfo {
    let kind = match source {
        IpcError::BridgeLoadFailed => ApiErrorKind::EventProcessingFailed,
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
    matches!(kind, ApiErrorKind::ProtocolMismatch)
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

trait BackendInputWaiter: Send + 'static {
    fn wait_for_input(&self, timeout: Option<Duration>) -> Result<EventWaitResult, IpcError>;
}

impl BackendInputWaiter for IpcEventWaitHandle {
    fn wait_for_input(&self, timeout: Option<Duration>) -> Result<EventWaitResult, IpcError> {
        self.wait_for_event(timeout)
    }
}

#[derive(Default)]
struct WakeStateInner {
    pending_commands: VecDeque<CommandEnvelope<ChromiumBackend>>,
    command_channel_closed: bool,
    backend_input_ready: bool,
    backend_terminal: Option<EventWaitResult>,
    wait_error: Option<IpcError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeadlineStatus {
    None,
    Pending,
    Reached,
}

fn classify_deadline(now: Instant, deadline: Option<Instant>) -> DeadlineStatus {
    match deadline {
        None => DeadlineStatus::None,
        Some(deadline) if now >= deadline => DeadlineStatus::Reached,
        Some(_) => DeadlineStatus::Pending,
    }
}

fn classify_ready_wake(inner: &WakeStateInner) -> Option<BackendWake> {
    if !inner.pending_commands.is_empty() {
        return Some(BackendWake::CommandReady);
    }
    if inner.backend_input_ready || inner.wait_error.is_some() {
        return Some(BackendWake::BackendInputReady);
    }
    if inner.command_channel_closed || inner.backend_terminal.is_some() {
        return Some(BackendWake::Stopped);
    }

    None
}

fn classify_timeout_wake(inner: &WakeStateInner) -> BackendWake {
    classify_ready_wake(inner).unwrap_or(BackendWake::DeadlineReached)
}

fn stop_reason_from_wake_state(inner: &WakeStateInner) -> Option<BackendStopReason> {
    if !inner.pending_commands.is_empty() || inner.backend_input_ready || inner.wait_error.is_some()
    {
        return None;
    }

    if inner.command_channel_closed {
        return Some(BackendStopReason::Disconnected);
    }

    match inner.backend_terminal {
        Some(EventWaitResult::Disconnected | EventWaitResult::Closed) => {
            Some(BackendStopReason::Disconnected)
        }
        Some(EventWaitResult::EventAvailable | EventWaitResult::TimedOut) | None => None,
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum ShutdownState {
    #[default]
    Idle,
    Proceeding {
        request_id: u64,
    },
}

fn update_shutdown_state(shutdown_state: &mut ShutdownState, event: &IpcEvent) {
    match event {
        IpcEvent::ShutdownProceeding { request_id } => {
            *shutdown_state = ShutdownState::Proceeding {
                request_id: *request_id,
            };
        }
        IpcEvent::ShutdownCancelled { .. } => {
            *shutdown_state = ShutdownState::Idle;
        }
        _ => {}
    }
}

#[derive(Default)]
struct WakeState {
    inner: Mutex<WakeStateInner>,
    cv: Condvar,
    stop_requested: AtomicBool,
}

impl WakeState {
    fn push_command(&self, envelope: CommandEnvelope<ChromiumBackend>) {
        let mut inner = self.inner.lock().unwrap();
        inner.pending_commands.push_back(envelope);
        self.cv.notify_all();
    }

    fn mark_command_channel_closed(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.command_channel_closed = true;
        self.cv.notify_all();
    }

    fn mark_backend_input_ready(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.backend_input_ready = true;
        self.cv.notify_all();
    }

    fn mark_backend_terminal(&self, wait_result: EventWaitResult) {
        let mut inner = self.inner.lock().unwrap();
        inner.backend_terminal = Some(wait_result);
        self.cv.notify_all();
    }

    fn mark_wait_error(&self, err: IpcError) {
        let mut inner = self.inner.lock().unwrap();
        inner.wait_error = Some(err);
        inner.backend_input_ready = true;
        self.cv.notify_all();
    }

    fn wait_for_backend_input_release(&self) {
        let mut inner = self.inner.lock().unwrap();
        while !self.stop_requested.load(Ordering::Acquire)
            && (inner.backend_input_ready || inner.wait_error.is_some())
        {
            inner = self.cv.wait(inner).unwrap();
        }
    }

    fn take_pending_commands(&self) -> Vec<CommandEnvelope<ChromiumBackend>> {
        let mut inner = self.inner.lock().unwrap();
        inner.pending_commands.drain(..).collect()
    }

    fn take_wait_error(&self) -> Option<IpcError> {
        let mut inner = self.inner.lock().unwrap();
        let err = inner.wait_error.take();
        if inner.wait_error.is_none() && !inner.backend_input_ready {
            self.cv.notify_all();
        }
        err
    }

    fn acknowledge_backend_input(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.backend_input_ready = false;
        self.cv.notify_all();
    }

    fn stop_reason(&self) -> Option<BackendStopReason> {
        let inner = self.inner.lock().unwrap();
        stop_reason_from_wake_state(&inner)
    }
}

struct ChromiumBackendEventLoop<W: BackendInputWaiter = IpcEventWaitHandle> {
    wake_state: Arc<WakeState>,
    command_rx: Receiver<CommandEnvelope<ChromiumBackend>>,
    command_thread: Option<JoinHandle<()>>,
    ipc_thread: Option<JoinHandle<()>>,
    _backend_input_waiter: std::marker::PhantomData<W>,
}

impl<W: BackendInputWaiter> ChromiumBackendEventLoop<W> {
    const IPC_WATCH_STOP_POLL_INTERVAL: Duration = Duration::from_millis(50);

    fn new(
        command_rx: Receiver<CommandEnvelope<ChromiumBackend>>,
        backend_input_waiter: W,
    ) -> Self {
        let wake_state = Arc::new(WakeState::default());

        let command_thread = {
            let wake_state = Arc::clone(&wake_state);
            let command_rx = command_rx.clone();
            thread::spawn(move || {
                loop {
                    match command_rx.recv_blocking() {
                        Ok(envelope) => wake_state.push_command(envelope),
                        Err(_) => {
                            wake_state.mark_command_channel_closed();
                            break;
                        }
                    }
                }
            })
        };

        let ipc_thread = {
            let wake_state = Arc::clone(&wake_state);
            thread::spawn(move || {
                while !wake_state.stop_requested.load(Ordering::Acquire) {
                    match backend_input_waiter
                        .wait_for_input(Some(Self::IPC_WATCH_STOP_POLL_INTERVAL))
                    {
                        Ok(EventWaitResult::EventAvailable) => {
                            wake_state.mark_backend_input_ready();
                            wake_state.wait_for_backend_input_release();
                        }
                        Ok(EventWaitResult::TimedOut) => {}
                        Ok(wait_result @ EventWaitResult::Disconnected)
                        | Ok(wait_result @ EventWaitResult::Closed) => {
                            wake_state.mark_backend_terminal(wait_result);
                            break;
                        }
                        Err(err) => {
                            wake_state.mark_wait_error(err);
                            wake_state.wait_for_backend_input_release();
                        }
                    }
                }
            })
        };

        Self {
            wake_state,
            command_rx,
            command_thread: Some(command_thread),
            ipc_thread: Some(ipc_thread),
            _backend_input_waiter: std::marker::PhantomData,
        }
    }

    fn take_pending_commands(&self) -> Vec<CommandEnvelope<ChromiumBackend>> {
        self.wake_state.take_pending_commands()
    }

    fn take_wait_error(&self) -> Option<IpcError> {
        self.wake_state.take_wait_error()
    }

    fn acknowledge_backend_input(&self) {
        self.wake_state.acknowledge_backend_input();
    }

    fn stop_reason(&self) -> Option<BackendStopReason> {
        self.wake_state.stop_reason()
    }
}

impl<W: BackendInputWaiter> BackendEventLoop for ChromiumBackendEventLoop<W> {
    fn wait_until(&self, deadline: Option<Instant>) -> BackendWake {
        let mut inner = self.wake_state.inner.lock().unwrap();

        loop {
            if let Some(wake) = classify_ready_wake(&inner) {
                return wake;
            }

            match classify_deadline(Instant::now(), deadline) {
                DeadlineStatus::None => {
                    inner = self.wake_state.cv.wait(inner).unwrap();
                }
                DeadlineStatus::Reached => return BackendWake::DeadlineReached,
                DeadlineStatus::Pending => {
                    let deadline = deadline.expect("pending deadline must exist");
                    let timeout = deadline.saturating_duration_since(Instant::now());
                    let (next_inner, timeout_result) =
                        self.wake_state.cv.wait_timeout(inner, timeout).unwrap();
                    inner = next_inner;

                    if timeout_result.timed_out() {
                        return classify_timeout_wake(&inner);
                    }
                }
            }
        }
    }
}

impl<W: BackendInputWaiter> Drop for ChromiumBackendEventLoop<W> {
    fn drop(&mut self) {
        self.wake_state
            .stop_requested
            .store(true, Ordering::Release);
        _ = self.command_rx.close();
        self.wake_state.cv.notify_all();

        if let Some(handle) = self.command_thread.take() {
            handle.join().ok();
        }
        if let Some(handle) = self.ipc_thread.take() {
            handle.join().ok();
        }
    }
}

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
        let ChromiumBackend { options, client } = self;
        let raw_delegate = raw_delegate.unwrap_or_else(|| Box::<NoopRawDelegate>::default());

        thread::spawn(move || {
            Self::run_communication(
                options,
                client,
                command_rx,
                event_tx,
                delegate,
                raw_delegate,
            )
        });

        Ok((
            CommandSender::from_raw_sender(command_tx),
            EventStream::from_raw_receiver(event_rx),
        ))
    }
}

impl ChromiumBackend {
    /// Create a backend from a pre-connected IPC client.
    pub fn new(options: ChromiumBackendOptions, client: IpcClient) -> Self {
        Self { options, client }
    }

    fn run_communication(
        options: ChromiumBackendOptions,
        client: IpcClient,
        command_rx: Receiver<CommandEnvelope<Self>>,
        event_tx: Sender<ChromeEvent>,
        delegate: impl BackendDelegate,
        mut raw_delegate: Box<dyn ChromeRawDelegate>,
    ) {
        let mut dispatcher = DelegateDispatcher::new(delegate);

        // Client is already connected; emit BackendReady and start the event loop.
        let Some(mut client) = Self::start_connection(options, client, &event_tx, &mut dispatcher)
        else {
            return;
        };
        let event_loop = ChromiumBackendEventLoop::new(command_rx, client.event_wait_handle());
        let mut shutdown_state = ShutdownState::default();

        while Self::run_iteration(
            &event_loop,
            &mut client,
            &event_tx,
            &mut dispatcher,
            raw_delegate.as_mut(),
            &mut shutdown_state,
        ) {}
    }

    fn start_connection(
        options: ChromiumBackendOptions,
        mut client: IpcClient,
        event_tx: &Sender<ChromeEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
    ) -> Option<IpcClient> {
        for registration in &options.custom_scheme_registrations {
            if let Err(err) =
                client.register_custom_scheme_handler(&registration.scheme, &registration.host)
            {
                let info = backend_error_event(err);
                let terminal_hint = backend_error_terminal_hint(info.kind);
                if let Some(stop_reason) = Self::handle_raw_event_with_delegate_gate(
                    dispatcher,
                    event_tx,
                    ChromeEvent::BackendError {
                        info,
                        terminal_hint,
                    },
                ) {
                    Self::stop_backend(stop_reason, dispatcher, Some(&mut client), event_tx);
                    return None;
                }
            }
        }

        // Notify that the backend is ready. The connection is already established.
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

    fn run_iteration(
        event_loop: &ChromiumBackendEventLoop,
        client: &mut IpcClient,
        event_tx: &Sender<ChromeEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
        raw_delegate: &mut dyn ChromeRawDelegate,
        shutdown_state: &mut ShutdownState,
    ) -> bool {
        if let Some(stop_reason) = Self::drain_ready_sources(
            event_loop,
            client,
            event_tx,
            dispatcher,
            raw_delegate,
            shutdown_state,
        ) {
            Self::stop_backend(stop_reason, dispatcher, Some(client), event_tx);
            return false;
        }

        let wake = event_loop.wait_until(dispatcher.next_wake_deadline());
        if matches!(
            wake,
            BackendWake::CommandReady
                | BackendWake::BackendInputReady
                | BackendWake::DeadlineReached
        ) {
            dispatcher.on_wake();
        }

        if let Some(stop_reason) = Self::drain_ready_sources(
            event_loop,
            client,
            event_tx,
            dispatcher,
            raw_delegate,
            shutdown_state,
        ) {
            Self::stop_backend(stop_reason, dispatcher, Some(client), event_tx);
            return false;
        }

        if matches!(wake, BackendWake::Stopped)
            && let Some(stop_reason) = event_loop.stop_reason()
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
        shutdown_state: &mut ShutdownState,
    ) -> Option<BackendStopReason> {
        while let Some(event) = client.poll_event() {
            match event {
                Ok(event) => {
                    if let Some(reason) =
                        Self::handle_ipc_event(event, event_tx, dispatcher, shutdown_state)
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
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
        shutdown_state: &mut ShutdownState,
    ) -> Option<BackendStopReason> {
        update_shutdown_state(shutdown_state, &event);
        Self::handle_raw_event_with_delegate_gate(
            dispatcher,
            event_tx,
            ChromeEvent::Ipc(Box::new(event)),
        )
    }

    fn drain_pending_command_queue(
        event_loop: &ChromiumBackendEventLoop,
        client: &mut IpcClient,
        event_tx: &Sender<ChromeEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
        raw_delegate: &mut dyn ChromeRawDelegate,
    ) -> Option<BackendStopReason> {
        for envelope in event_loop.take_pending_commands() {
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

        None
    }

    fn handle_wait_error(
        event_loop: &ChromiumBackendEventLoop,
        event_tx: &Sender<ChromeEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
    ) -> Option<BackendStopReason> {
        if let Some(err) = event_loop.take_wait_error() {
            let info = backend_error_event(err);
            let terminal_hint = backend_error_terminal_hint(info.kind);
            return Self::handle_raw_event_with_delegate_gate(
                dispatcher,
                event_tx,
                ChromeEvent::BackendError {
                    info,
                    terminal_hint,
                },
            );
        }

        None
    }

    fn drain_ready_sources(
        event_loop: &ChromiumBackendEventLoop,
        client: &mut IpcClient,
        event_tx: &Sender<ChromeEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
        raw_delegate: &mut dyn ChromeRawDelegate,
        shutdown_state: &mut ShutdownState,
    ) -> Option<BackendStopReason> {
        let queued_commands = dispatcher.flush();
        if let Some(stop_reason) =
            Self::drain_delegate_queue(dispatcher, client, event_tx, queued_commands)
        {
            return Some(stop_reason);
        }

        if let Some(stop_reason) = Self::drain_pending_command_queue(
            event_loop,
            client,
            event_tx,
            dispatcher,
            raw_delegate,
        ) {
            return Some(stop_reason);
        }

        if let Some(stop_reason) = Self::handle_wait_error(event_loop, event_tx, dispatcher) {
            event_loop.acknowledge_backend_input();
            return Some(stop_reason);
        }

        if let Some(stop_reason) =
            Self::process_event_queue(client, event_tx, dispatcher, shutdown_state)
        {
            event_loop.acknowledge_backend_input();
            return Some(stop_reason);
        }
        event_loop.acknowledge_backend_input();

        let queued_commands = dispatcher.flush();
        if let Some(stop_reason) =
            Self::drain_delegate_queue(dispatcher, client, event_tx, queued_commands)
        {
            return Some(stop_reason);
        }

        event_loop.stop_reason()
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
            ChromeCommand::RespondJavaScriptDialog {
                browsing_context_id,
                request_id,
                response,
            } => {
                let (accept, prompt_text) = dialog_response_parts(response);
                client
                    .respond_javascript_dialog(
                        *browsing_context_id,
                        *request_id,
                        accept,
                        prompt_text.as_deref(),
                    )
                    .map(|_| (None, Vec::new()))
            }
            ChromeCommand::RespondExtensionPopupJavaScriptDialog {
                popup_id,
                request_id,
                response,
            } => {
                let (accept, prompt_text) = dialog_response_parts(response);
                client
                    .respond_extension_popup_javascript_dialog(
                        *popup_id,
                        *request_id,
                        accept,
                        prompt_text.as_deref(),
                    )
                    .map(|_| (None, Vec::new()))
            }
            ChromeCommand::ConfirmPermission {
                browsing_context_id,
                request_id,
                allow,
            } => client
                .respond_prompt_ui_for_tab(
                    *browsing_context_id,
                    *request_id,
                    &PromptUiResponse::PermissionPrompt { allow: *allow },
                )
                .map(|_| (None, Vec::new())),
            ChromeCommand::CreateTab {
                request_id,
                initial_url,
                profile_id,
            } => {
                let url = initial_url
                    .clone()
                    .unwrap_or_else(|| "about:blank".to_string());

                client
                    .create_tab(*request_id, &url, profile_id)
                    .map(|_| (None, Vec::new()))
            }
            ChromeCommand::SetTabSize {
                browsing_context_id,
                width,
                height,
            } => client
                .set_tab_size(*browsing_context_id, *width, *height)
                .map(|_| (None, Vec::new())),
            ChromeCommand::SetTabBackgroundPolicy {
                browsing_context_id,
                policy,
            } => client
                .set_tab_background_policy(*browsing_context_id, *policy)
                .map(|_| (None, Vec::new())),
            ChromeCommand::ListProfiles => client
                .list_profiles()
                .map(|profiles| (None, vec![ChromeEvent::ProfilesListed { profiles }])),
            ChromeCommand::ListExtensions { profile_id } => {
                client.list_extensions(profile_id).map(|extensions| {
                    (
                        None,
                        vec![ChromeEvent::Ipc(Box::new(IpcEvent::ExtensionsListed {
                            profile_id: profile_id.clone(),
                            extensions,
                        }))],
                    )
                })
            }
            ChromeCommand::RespondCustomSchemeRequest { response } => client
                .respond_custom_scheme_request(response)
                .map(|_| (None, Vec::new())),
            ChromeCommand::ActivateExtensionAction {
                browsing_context_id,
                extension_id,
            } => client
                .activate_extension_action(*browsing_context_id, extension_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::CloseExtensionPopup { popup_id } => client
                .close_extension_popup(*popup_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::SetExtensionPopupSize {
                popup_id,
                width,
                height,
            } => client
                .set_extension_popup_size(*popup_id, *width, *height)
                .map(|_| (None, Vec::new())),
            ChromeCommand::SetExtensionPopupBackgroundPolicy { popup_id, policy } => client
                .set_extension_popup_background_policy(*popup_id, *policy)
                .map(|_| (None, Vec::new())),
            ChromeCommand::SetExtensionPopupFocus { popup_id, focused } => client
                .set_extension_popup_focus(*popup_id, *focused)
                .map(|_| (None, Vec::new())),
            ChromeCommand::SendExtensionPopupKeyEvent {
                popup_id,
                event,
                commands,
            } => client
                .send_extension_popup_key_event_raw(*popup_id, event, commands)
                .map(|_| (None, Vec::new())),
            ChromeCommand::ExecuteExtensionPopupEditAction { popup_id, action } => client
                .execute_extension_popup_edit_action(*popup_id, *action)
                .map(|_| (None, Vec::new())),
            ChromeCommand::SendExtensionPopupMouseEvent { popup_id, event } => client
                .send_extension_popup_mouse_event(*popup_id, event)
                .map(|_| (None, Vec::new())),
            ChromeCommand::SendExtensionPopupMouseWheelEvent { popup_id, event } => client
                .send_extension_popup_mouse_wheel_event_raw(*popup_id, event)
                .map(|_| (None, Vec::new())),
            ChromeCommand::SendKeyEvent {
                browsing_context_id,
                event,
                commands,
            } => client
                .send_key_event_raw(*browsing_context_id, event, commands)
                .map(|_| (None, Vec::new())),
            ChromeCommand::ExecuteEditAction {
                browsing_context_id,
                action,
            } => client
                .execute_edit_action(*browsing_context_id, *action)
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
                .send_mouse_wheel_event_raw(*browsing_context_id, event)
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
            ChromeCommand::SendExternalDragEnter { event } => client
                .send_external_drag_enter(event)
                .map(|_| (None, Vec::new())),
            ChromeCommand::SendExternalDragUpdate { event } => client
                .send_external_drag_update(event)
                .map(|_| (None, Vec::new())),
            ChromeCommand::SendExternalDragLeave {
                browsing_context_id,
            } => client
                .send_external_drag_leave(*browsing_context_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::SendExternalDragDrop { event } => client
                .send_external_drag_drop(event)
                .map(|_| (None, Vec::new())),
            ChromeCommand::SetImeComposition { composition } => client
                .set_composition(composition)
                .map(|_| (None, Vec::new())),
            ChromeCommand::SetExtensionPopupComposition { composition } => client
                .set_extension_popup_composition(composition)
                .map(|_| (None, Vec::new())),
            ChromeCommand::CommitImeText { commit } => {
                client.commit_text(commit).map(|_| (None, Vec::new()))
            }
            ChromeCommand::CommitExtensionPopupText { commit } => client
                .commit_extension_popup_text(commit)
                .map(|_| (None, Vec::new())),
            ChromeCommand::FinishComposingText {
                browsing_context_id,
                behavior,
            } => client
                .finish_composing_text(*browsing_context_id, *behavior)
                .map(|_| (None, Vec::new())),
            ChromeCommand::FinishExtensionPopupComposingText { popup_id, behavior } => client
                .finish_extension_popup_composing_text(*popup_id, *behavior)
                .map(|_| (None, Vec::new())),
            ChromeCommand::ExecuteContextMenuCommand {
                menu_id,
                command_id,
                event_flags,
            } => client
                .execute_context_menu_command(*menu_id, *command_id, *event_flags)
                .map(|_| (None, Vec::new())),
            ChromeCommand::AcceptChoiceMenuSelection {
                request_id,
                indices,
            } => client
                .accept_choice_menu_selection(*request_id, indices)
                .map(|_| (None, Vec::new())),
            ChromeCommand::DismissChoiceMenu { request_id } => client
                .dismiss_choice_menu(*request_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::DismissContextMenu { menu_id } => client
                .dismiss_context_menu(*menu_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::PauseDownload { download_id } => client
                .pause_download(*download_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::ResumeDownload { download_id } => client
                .resume_download(*download_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::CancelDownload { download_id } => client
                .cancel_download(*download_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::RequestCloseTab {
                browsing_context_id,
            } => client
                .request_close_tab(*browsing_context_id)
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
            ChromeCommand::GetTabDomHtml {
                browsing_context_id,
                request_id,
            } => client
                .get_tab_dom_html(*browsing_context_id, *request_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::FindInPage {
                browsing_context_id,
                request_id,
                options,
            } => client
                .find_in_page(*browsing_context_id, *request_id, options)
                .map(|_| (None, Vec::new())),
            ChromeCommand::StopFinding {
                browsing_context_id,
                action,
            } => client
                .stop_finding(*browsing_context_id, *action)
                .map(|_| (None, Vec::new())),
            ChromeCommand::SetTabFocus {
                browsing_context_id,
                focused,
            } => client
                .set_tab_focus(*browsing_context_id, *focused)
                .map(|_| (None, Vec::new())),
            ChromeCommand::SetTabVisibility {
                browsing_context_id,
                visibility,
            } => client
                .set_tab_visibility(*browsing_context_id, *visibility)
                .map(|_| (None, Vec::new())),
            ChromeCommand::EnableTabIpc {
                browsing_context_id,
                config,
            } => client
                .enable_tab_ipc(*browsing_context_id, config)
                .map(|_| (None, Vec::new())),
            ChromeCommand::DisableTabIpc {
                browsing_context_id,
            } => client
                .disable_tab_ipc(*browsing_context_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::PostTabIpcMessage {
                browsing_context_id,
                message,
            } => client
                .post_tab_ipc_message(*browsing_context_id, message)
                .map(|_| (None, Vec::new())),
            ChromeCommand::OpenDefaultPromptUi {
                profile_id,
                request_id,
            } => client
                .open_default_prompt_ui(profile_id, *request_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::RespondPromptUi {
                profile_id,
                request_id,
                response,
            } => client
                .respond_prompt_ui(profile_id, *request_id, response)
                .map(|_| (None, Vec::new())),
            ChromeCommand::ClosePromptUi {
                profile_id,
                prompt_ui_id,
            } => client
                .close_prompt_ui(profile_id, *prompt_ui_id)
                .map(|_| (None, Vec::new())),
            ChromeCommand::RespondTabOpen {
                request_id,
                response,
            } => client
                .respond_tab_open(*request_id, response)
                .map(|_| (None, Vec::new())),
            ChromeCommand::RespondWindowOpen {
                request_id,
                response,
            } => client
                .respond_window_open(*request_id, response)
                .map(|_| (None, Vec::new())),
            ChromeCommand::UnsupportedGenericCommand { operation } => {
                return Err(CommandExecutionError::Unsupported {
                    operation: *operation,
                    detail: "transient browsing context commands are not yet implemented in the Chromium transport",
                });
            }
        };

        result.map_err(|source| CommandExecutionError::from_ipc_call(operation, source))
    }
}

fn dialog_response_parts(response: &DialogResponse) -> (bool, Option<String>) {
    match response {
        DialogResponse::Success { input } => (true, input.clone()),
        DialogResponse::Cancel => (false, None),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        mem::MaybeUninit,
        sync::{Arc, Mutex, mpsc},
        thread,
        time::{Duration, Instant},
    };

    use async_channel::unbounded;
    use cbf::{
        backend_event_loop::{BackendEventLoop, BackendWake},
        browser::{Backend, EventStream, RawOpaqueEventExt},
        delegate::{BackendDelegate, DelegateContext, NoopDelegate},
        event::BackendStopReason,
    };

    use super::{
        BackendInputWaiter, ChromeCommand, ChromeEvent, ChromiumBackend, ChromiumBackendEventLoop,
        ChromiumBackendOptions, DeadlineStatus, EventWaitResult, IpcClient, ShutdownState,
        WakeStateInner, classify_deadline, classify_ready_wake, classify_timeout_wake,
        stop_reason_from_wake_state, update_shutdown_state,
    };
    use crate::bridge::IpcEvent;

    struct StubWaiter {
        rx: std::sync::mpsc::Receiver<Result<EventWaitResult, super::IpcError>>,
    }

    impl BackendInputWaiter for StubWaiter {
        fn wait_for_input(
            &self,
            timeout: Option<Duration>,
        ) -> Result<EventWaitResult, super::IpcError> {
            match timeout {
                Some(timeout) => self
                    .rx
                    .recv_timeout(timeout)
                    .unwrap_or(Ok(EventWaitResult::TimedOut)),
                None => self.rx.recv().unwrap_or(Ok(EventWaitResult::Closed)),
            }
        }
    }

    fn null_ipc_client() -> IpcClient {
        // SAFETY: `IpcClient` is a raw pointer wrapper. A null pointer is a valid
        // inert state for this test path because `poll_event`/`drop` both handle null.
        unsafe { MaybeUninit::zeroed().assume_init() }
    }

    fn sample_pending_command() -> cbf::browser::CommandEnvelope<ChromiumBackend> {
        cbf::browser::CommandEnvelope::RawOnly {
            raw: ChromeCommand::ForceShutdown,
        }
    }

    fn recv_raw_event_with_timeout(
        events: &EventStream<ChromiumBackend>,
        timeout: Duration,
    ) -> ChromeEvent {
        let events = events.clone();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let event = events.recv_blocking().map(|opaque| opaque.as_raw().clone());
            tx.send(event).ok();
        });

        rx.recv_timeout(timeout)
            .expect("timed out waiting for backend event")
            .expect("event stream closed unexpectedly")
    }

    #[test]
    fn dropping_all_command_senders_emits_disconnected_stop_event() {
        let backend = ChromiumBackend::new(ChromiumBackendOptions::new(), null_ipc_client());
        let (command_tx, events) = backend.connect(NoopDelegate, None).unwrap();
        drop(command_tx);

        let ready = recv_raw_event_with_timeout(&events, Duration::from_secs(1));
        assert!(matches!(ready, ChromeEvent::BackendReady));

        let stopped = recv_raw_event_with_timeout(&events, Duration::from_secs(1));
        assert!(matches!(
            stopped,
            ChromeEvent::BackendStopped {
                reason: BackendStopReason::Disconnected
            }
        ));
    }

    #[test]
    fn command_wake_beats_long_deadline() {
        let (command_tx, command_rx) =
            unbounded::<cbf::browser::CommandEnvelope<ChromiumBackend>>();
        let (_wait_tx, wait_rx) = std::sync::mpsc::channel();
        let event_loop = ChromiumBackendEventLoop::new(command_rx, StubWaiter { rx: wait_rx });
        let (wake_tx, wake_rx) = mpsc::channel();
        let waiter_thread = thread::spawn(move || {
            let wake = event_loop.wait_until(Some(Instant::now() + Duration::from_secs(1)));
            wake_tx.send(wake).unwrap();
        });

        thread::sleep(Duration::from_millis(20));
        command_tx
            .send_blocking(cbf::browser::CommandEnvelope::RawOnly {
                raw: ChromeCommand::ForceShutdown,
            })
            .unwrap();

        let wake = wake_rx.recv_timeout(Duration::from_millis(250)).unwrap();
        assert_eq!(wake, BackendWake::CommandReady);
        waiter_thread.join().unwrap();
    }

    #[test]
    fn backend_input_and_terminal_wait_results_map_to_wakes() {
        let (_command_tx, command_rx) =
            unbounded::<cbf::browser::CommandEnvelope<ChromiumBackend>>();
        let (wait_tx, wait_rx) = std::sync::mpsc::channel();
        let event_loop = ChromiumBackendEventLoop::new(command_rx, StubWaiter { rx: wait_rx });

        wait_tx.send(Ok(EventWaitResult::EventAvailable)).unwrap();
        assert_eq!(
            event_loop.wait_until(Some(Instant::now() + Duration::from_secs(1))),
            BackendWake::BackendInputReady
        );
        event_loop.acknowledge_backend_input();

        wait_tx.send(Ok(EventWaitResult::Disconnected)).unwrap();
        assert_eq!(
            event_loop.wait_until(Some(Instant::now() + Duration::from_secs(1))),
            BackendWake::Stopped
        );
        assert_eq!(
            event_loop.stop_reason(),
            Some(BackendStopReason::Disconnected)
        );
    }

    #[test]
    fn ready_wake_classification_prefers_pending_command() {
        let mut inner = WakeStateInner {
            command_channel_closed: true,
            backend_input_ready: true,
            backend_terminal: Some(EventWaitResult::Disconnected),
            wait_error: Some(super::IpcError::ConnectionFailed),
            ..WakeStateInner::default()
        };
        inner.pending_commands.push_back(sample_pending_command());

        assert_eq!(classify_ready_wake(&inner), Some(BackendWake::CommandReady));
    }

    #[test]
    fn ready_wake_classification_prefers_backend_input_over_stop() {
        let inner = WakeStateInner {
            command_channel_closed: true,
            backend_input_ready: true,
            backend_terminal: Some(EventWaitResult::Closed),
            ..WakeStateInner::default()
        };

        assert_eq!(
            classify_ready_wake(&inner),
            Some(BackendWake::BackendInputReady)
        );
    }

    #[test]
    fn ready_wake_classification_returns_stopped_only_after_work_is_drained() {
        let stopped = WakeStateInner {
            command_channel_closed: true,
            ..WakeStateInner::default()
        };
        let idle = WakeStateInner::default();

        assert_eq!(classify_ready_wake(&stopped), Some(BackendWake::Stopped));
        assert_eq!(classify_ready_wake(&idle), None);
    }

    #[test]
    fn deadline_classification_handles_none_due_and_future() {
        let now = Instant::now();

        assert_eq!(classify_deadline(now, None), DeadlineStatus::None);
        assert_eq!(
            classify_deadline(now, Some(now - Duration::from_millis(1))),
            DeadlineStatus::Reached
        );
        assert_eq!(
            classify_deadline(now, Some(now + Duration::from_secs(1))),
            DeadlineStatus::Pending
        );
    }

    #[test]
    fn timeout_wake_classification_falls_back_to_deadline_when_idle() {
        let idle = WakeStateInner::default();
        let mut command_ready = WakeStateInner::default();
        command_ready
            .pending_commands
            .push_back(sample_pending_command());

        assert_eq!(classify_timeout_wake(&idle), BackendWake::DeadlineReached);
        assert_eq!(
            classify_timeout_wake(&command_ready),
            BackendWake::CommandReady
        );
    }

    #[test]
    fn stop_reason_classification_requires_pending_work_to_be_drained() {
        let mut pending_command = WakeStateInner {
            command_channel_closed: true,
            ..WakeStateInner::default()
        };
        pending_command
            .pending_commands
            .push_back(sample_pending_command());
        let backend_input = WakeStateInner {
            command_channel_closed: true,
            backend_input_ready: true,
            ..WakeStateInner::default()
        };
        let wait_error = WakeStateInner {
            command_channel_closed: true,
            wait_error: Some(super::IpcError::ConnectionFailed),
            ..WakeStateInner::default()
        };

        assert_eq!(stop_reason_from_wake_state(&pending_command), None);
        assert_eq!(stop_reason_from_wake_state(&backend_input), None);
        assert_eq!(stop_reason_from_wake_state(&wait_error), None);
    }

    #[test]
    fn stop_reason_classification_maps_only_terminal_states() {
        let command_closed = WakeStateInner {
            command_channel_closed: true,
            ..WakeStateInner::default()
        };
        let disconnected = WakeStateInner {
            backend_terminal: Some(EventWaitResult::Disconnected),
            ..WakeStateInner::default()
        };
        let closed = WakeStateInner {
            backend_terminal: Some(EventWaitResult::Closed),
            ..WakeStateInner::default()
        };
        let event_available = WakeStateInner {
            backend_terminal: Some(EventWaitResult::EventAvailable),
            ..WakeStateInner::default()
        };
        let timed_out = WakeStateInner {
            backend_terminal: Some(EventWaitResult::TimedOut),
            ..WakeStateInner::default()
        };

        assert_eq!(
            stop_reason_from_wake_state(&command_closed),
            Some(BackendStopReason::Disconnected)
        );
        assert_eq!(
            stop_reason_from_wake_state(&disconnected),
            Some(BackendStopReason::Disconnected)
        );
        assert_eq!(
            stop_reason_from_wake_state(&closed),
            Some(BackendStopReason::Disconnected)
        );
        assert_eq!(stop_reason_from_wake_state(&event_available), None);
        assert_eq!(stop_reason_from_wake_state(&timed_out), None);
    }

    #[test]
    fn update_shutdown_state_tracks_proceeding_and_cancelled_events() {
        let mut shutdown_state = ShutdownState::Idle;

        update_shutdown_state(
            &mut shutdown_state,
            &IpcEvent::ShutdownProceeding { request_id: 11 },
        );
        assert_eq!(shutdown_state, ShutdownState::Proceeding { request_id: 11 });

        update_shutdown_state(
            &mut shutdown_state,
            &IpcEvent::ShutdownCancelled { request_id: 11 },
        );
        assert_eq!(shutdown_state, ShutdownState::Idle);
    }

    #[test]
    fn update_shutdown_state_ignores_shutdown_blocked() {
        let mut shutdown_state = ShutdownState::Idle;

        update_shutdown_state(
            &mut shutdown_state,
            &IpcEvent::ShutdownBlocked {
                request_id: 5,
                dirty_browsing_context_ids: Vec::new(),
            },
        );

        assert_eq!(shutdown_state, ShutdownState::Idle);
    }

    #[derive(Clone)]
    struct RecordTeardownDelegate {
        reasons: Arc<Mutex<Vec<BackendStopReason>>>,
    }

    impl BackendDelegate for RecordTeardownDelegate {
        fn on_teardown(&mut self, _ctx: &mut DelegateContext, reason: BackendStopReason) {
            self.reasons.lock().unwrap().push(reason);
        }
    }

    #[test]
    fn stop_backend_passes_disconnected_reason_to_teardown() {
        let reasons = Arc::new(Mutex::new(Vec::new()));
        let delegate = RecordTeardownDelegate {
            reasons: Arc::clone(&reasons),
        };
        let mut dispatcher = cbf::delegate::DelegateDispatcher::new(delegate);
        let (event_tx, _event_rx) = async_channel::unbounded();

        ChromiumBackend::stop_backend(
            BackendStopReason::Disconnected,
            &mut dispatcher,
            None,
            &event_tx,
        );

        let recorded = reasons.lock().unwrap().clone();
        assert_eq!(recorded, vec![BackendStopReason::Disconnected]);
    }
}
