use std::{
    collections::{HashMap, HashSet},
    thread,
    time::{Duration, Instant},
};

use async_channel::{Receiver, Sender, TryRecvError};

use crate::{
    Backend, Error,
    backend_delegate::{BackendDelegate, DelegateDispatcher},
    command::BrowserCommand,
    data::ids::WebPageId,
    event::{BackendStopReason, BrowserEvent, DialogType, WebPageEvent},
    ffi::{IpcClient, IpcEvent},
};

/// Backend implementation that speaks the Chromium IPC protocol.
#[derive(Debug, Clone)]
pub struct ChromiumBackend {
    channel_name: String,
}

struct CommunicationState {
    resizes_in_flight: HashSet<WebPageId>,
    pending_resizes: HashMap<WebPageId, (u32, u32)>,
}

impl Backend for ChromiumBackend {
    fn connect<D: BackendDelegate>(
        self,
        delegate: D,
    ) -> Result<(Sender<BrowserCommand>, Receiver<BrowserEvent>), Error> {
        let (command_tx, command_rx) = async_channel::unbounded::<BrowserCommand>();
        let (event_tx, event_rx) = async_channel::unbounded::<BrowserEvent>();
        let channel_name = self.channel_name;

        thread::spawn(move || {
            Self::run_communication(channel_name, command_rx, event_tx, delegate)
        });

        Ok((command_tx, event_rx))
    }
}

impl ChromiumBackend {
    /// Create a backend that connects to the given IPC channel name.
    pub fn new(channel_name: impl Into<String>) -> Self {
        Self {
            channel_name: channel_name.into(),
        }
    }

    fn start_connection(
        event_tx: &Sender<BrowserEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
        channel_name: &str,
    ) -> Option<IpcClient> {
        let timeout = Duration::from_secs(60);
        let start_time = Instant::now();
        const RETRY_INTERVAL: Duration = Duration::from_millis(100);

        let mut client = loop {
            match IpcClient::connect(channel_name) {
                Ok(client) => break client,
                Err(err) => {
                    if start_time.elapsed() > timeout {
                        let mut no_forward = |_| (None, Vec::new());
                        dispatcher.stop(
                            event_tx,
                            BackendStopReason::Error {
                                message: format!("IPC connect timed out: {err:?}"),
                            },
                            &mut no_forward,
                        );

                        return None;
                    }

                    thread::sleep(RETRY_INTERVAL);
                }
            }
        };

        // Notify that the backend is ready after establishing the connection.
        if let Some(stop_reason) = dispatcher.dispatch_event(
            BrowserEvent::BackendReady {
                backend_name: "chromium".to_string(),
            },
            event_tx,
        ) {
            let mut forward = |command| {
                (
                    Self::execute_command(command, &mut client, event_tx),
                    Vec::new(),
                )
            };
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
                |command| (Self::execute_command(command, client, event_tx), Vec::new())
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
        channel_name: String,
        command_rx: Receiver<BrowserCommand>,
        event_tx: Sender<BrowserEvent>,
        delegate: impl BackendDelegate,
    ) {
        let mut dispatcher = DelegateDispatcher::new(delegate);

        // Start the connection and get the IPC client.
        let Some(mut client) = Self::start_connection(&event_tx, &mut dispatcher, &channel_name)
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
                        handle_ipc_event(event, event_tx, client, state, dispatcher)
                    {
                        return Some(reason);
                    }
                }
                Err(err) => {
                    return Some(BackendStopReason::Error {
                        message: format!("IPC event error: {err:?}"),
                    });
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

            if let BrowserCommand::ResizeWebPage {
                web_page_id,
                width,
                height,
            } = command
            {
                let mut latest_resizes: HashMap<WebPageId, (u32, u32)> = HashMap::new();
                latest_resizes.insert(web_page_id, (width, height));

                loop {
                    match command_rx.try_recv() {
                        Ok(BrowserCommand::ResizeWebPage {
                            web_page_id,
                            width,
                            height,
                        }) => {
                            latest_resizes.insert(web_page_id, (width, height));
                        }
                        Ok(other) => {
                            pending_command = Some(other);
                            break;
                        }
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Closed) => return Some(BackendStopReason::Disconnected),
                    }
                }

                for (web_page_id, (width, height)) in latest_resizes {
                    if state.resizes_in_flight.contains(&web_page_id) {
                        state.pending_resizes.insert(web_page_id, (width, height));
                        continue;
                    }

                    state.resizes_in_flight.insert(web_page_id);
                    let mut forward =
                        |command| (Self::execute_command(command, client, event_tx), Vec::new());
                    if let Some(reason) = dispatcher.dispatch_command(
                        BrowserCommand::ResizeWebPage {
                            web_page_id,
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

            let mut forward =
                |command| (Self::execute_command(command, client, event_tx), Vec::new());
            if let Some(reason) = dispatcher.dispatch_command(command, event_tx, &mut forward) {
                return Some(reason);
            }
        }
    }

    fn execute_command(
        command: BrowserCommand,
        client: &mut IpcClient,
        event_tx: &Sender<BrowserEvent>,
    ) -> Option<BackendStopReason> {
        match Self::handle_command(command, client, event_tx) {
            Ok(Some(reason)) => Some(reason),
            Ok(None) => None,
            Err(message) => Some(BackendStopReason::Error { message }),
        }
    }

    fn handle_command(
        command: BrowserCommand,
        client: &mut IpcClient,
        event_tx: &Sender<BrowserEvent>,
    ) -> Result<Option<BackendStopReason>, String> {
        match command {
            BrowserCommand::Shutdown { request_id } => client
                .request_shutdown(request_id)
                .map(|_| None)
                .map_err(|err| format!("RequestShutdown failed: {err:?}")),
            BrowserCommand::ConfirmShutdown {
                request_id,
                proceed,
            } => client
                .confirm_shutdown(request_id, proceed)
                .map(|_| None)
                .map_err(|err| format!("ConfirmShutdown failed: {err:?}")),
            BrowserCommand::ForceShutdown => client
                .force_shutdown()
                .map(|_| None)
                .map_err(|err| format!("ForceShutdown failed: {err:?}")),
            BrowserCommand::ConfirmBeforeUnload {
                web_page_id,
                request_id,
                proceed,
            } => client
                .confirm_beforeunload(web_page_id, request_id, proceed)
                .map(|_| None)
                .map_err(|err| format!("ConfirmBeforeUnload failed: {err:?}")),
            BrowserCommand::CreateWebPage {
                request_id,
                initial_url,
                profile_id,
            } => {
                let url = initial_url.unwrap_or_else(|| "about:blank".to_string());
                let profile = profile_id.unwrap_or_default();

                client
                    .create_web_page(request_id, &url, &profile)
                    .map(|_| None)
                    .map_err(|err| format!("CreateWebPage failed: {err:?}"))
            }
            BrowserCommand::ResizeWebPage {
                web_page_id,
                width,
                height,
            } => client
                .set_web_page_size(web_page_id, width, height)
                .map(|_| None)
                .map_err(|err| format!("SetWebPageSize failed: {err:?}")),
            BrowserCommand::ListProfiles => match client.list_profiles() {
                Ok(profiles) => {
                    _ = event_tx.send_blocking(BrowserEvent::ProfilesListed { profiles });

                    Ok(None)
                }
                Err(err) => Err(format!("ListProfiles failed: {err:?}")),
            },
            BrowserCommand::SendKeyEvent {
                web_page_id,
                event,
                commands,
            } => client
                .send_key_event(web_page_id, &event, &commands)
                .map(|_| None)
                .map_err(|err| format!("SendKeyEvent failed: {err:?}")),
            BrowserCommand::SendMouseEvent { web_page_id, event } => client
                .send_mouse_event(web_page_id, &event)
                .map(|_| None)
                .map_err(|err| format!("SendMouseEvent failed: {err:?}")),
            BrowserCommand::SendMouseWheelEvent { web_page_id, event } => client
                .send_mouse_wheel_event(web_page_id, &event)
                .map(|_| None)
                .map_err(|err| format!("SendMouseWheelEvent failed: {err:?}")),
            BrowserCommand::SendDragUpdate { update } => client
                .send_drag_update(&update)
                .map(|_| None)
                .map_err(|err| format!("SendDragUpdate failed: {err:?}")),
            BrowserCommand::SendDragDrop { drop } => client
                .send_drag_drop(&drop)
                .map(|_| None)
                .map_err(|err| format!("SendDragDrop failed: {err:?}")),
            BrowserCommand::SendDragCancel {
                session_id,
                web_page_id,
            } => client
                .send_drag_cancel(session_id, web_page_id)
                .map(|_| None)
                .map_err(|err| format!("SendDragCancel failed: {err:?}")),
            BrowserCommand::SetComposition { composition } => client
                .set_composition(&composition)
                .map(|_| None)
                .map_err(|err| format!("SetComposition failed: {err:?}")),
            BrowserCommand::CommitText { commit } => client
                .commit_text(&commit)
                .map(|_| None)
                .map_err(|err| format!("CommitText failed: {err:?}")),
            BrowserCommand::FinishComposingText {
                web_page_id,
                behavior,
            } => client
                .finish_composing_text(web_page_id, behavior)
                .map(|_| None)
                .map_err(|err| format!("FinishComposingText failed: {err:?}")),
            BrowserCommand::ExecuteContextMenuCommand {
                menu_id,
                command_id,
                event_flags,
            } => client
                .execute_context_menu_command(menu_id, command_id, event_flags)
                .map(|_| None)
                .map_err(|err| format!("ExecuteContextMenuCommand failed: {err:?}")),
            BrowserCommand::DismissContextMenu { menu_id } => client
                .dismiss_context_menu(menu_id)
                .map(|_| None)
                .map_err(|err| format!("DismissContextMenu failed: {err:?}")),
            BrowserCommand::RequestCloseWebPage { web_page_id } => client
                .request_close_web_page(web_page_id)
                .map(|_| None)
                .map_err(|err| format!("RequestCloseWebPage failed: {err:?}")),
            BrowserCommand::Navigate { web_page_id, url } => client
                .navigate(web_page_id, &url)
                .map(|_| None)
                .map_err(|err| format!("Navigate failed: {err:?}")),
            BrowserCommand::GoBack { web_page_id } => client
                .go_back(web_page_id)
                .map(|_| None)
                .map_err(|err| format!("GoBack failed: {err:?}")),
            BrowserCommand::GoForward { web_page_id } => client
                .go_forward(web_page_id)
                .map(|_| None)
                .map_err(|err| format!("GoForward failed: {err:?}")),
            BrowserCommand::Reload {
                web_page_id,
                ignore_cache,
            } => client
                .reload(web_page_id, ignore_cache)
                .map(|_| None)
                .map_err(|err| format!("Reload failed: {err:?}")),
            BrowserCommand::GetWebPageDomHtml {
                web_page_id,
                request_id,
            } => client
                .get_web_page_dom_html(web_page_id, request_id)
                .map(|_| None)
                .map_err(|err| format!("GetWebPageDomHtml failed: {err:?}")),
            BrowserCommand::SetWebPageFocus {
                web_page_id,
                focused,
            } => client
                .set_web_page_focus(web_page_id, focused)
                .map(|_| None)
                .map_err(|err| format!("SetWebPageFocus failed: {err:?}")),
        }
    }
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
        IpcEvent::WebPageCreated {
            profile_id,
            web_page_id,
            request_id,
        } => {
            if let Some(reason) = emit(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::Created { request_id },
            }) {
                return Some(reason);
            }
        }
        IpcEvent::SurfaceHandleUpdated {
            profile_id,
            web_page_id,
            handle,
        } => {
            if let Some(reason) = emit(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::SurfaceHandleUpdated { handle },
            }) {
                return Some(reason);
            }
        }
        IpcEvent::WebPageResizeAcknowledged { web_page_id, .. } => {
            state.resizes_in_flight.remove(&web_page_id);
            if let Some((width, height)) = state.pending_resizes.remove(&web_page_id) {
                state.resizes_in_flight.insert(web_page_id);
                if let Err(err) = client.set_web_page_size(web_page_id, width, height) {
                    tracing::warn!(
                        result = "err",
                        error = "resize_failed",
                        err = ?err,
                        %web_page_id,
                        "Failed to send pending resize"
                    );
                    state.resizes_in_flight.remove(&web_page_id);
                }
            }
        }
        IpcEvent::WebPageDomHtmlRead {
            profile_id,
            web_page_id,
            request_id,
            html,
        } => {
            if let Some(reason) = emit(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::DomHtmlRead { request_id, html },
            }) {
                return Some(reason);
            }
        }
        IpcEvent::DragStartRequested {
            profile_id,
            web_page_id,
            request,
        } => {
            if let Some(reason) = emit(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::DragStartRequested { request },
            }) {
                return Some(reason);
            }
        }
        IpcEvent::ImeBoundsUpdated {
            profile_id,
            web_page_id,
            update,
        } => {
            if let Some(reason) = emit(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::ImeBoundsUpdated { update },
            }) {
                return Some(reason);
            }
        }
        IpcEvent::ContextMenuRequested {
            profile_id,
            web_page_id,
            menu,
        } => {
            if let Some(reason) = emit(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::ContextMenuRequested { menu },
            }) {
                return Some(reason);
            }
        }
        IpcEvent::NewWebPageRequested {
            profile_id,
            web_page_id,
            target_url,
            is_popup,
        } => {
            if let Some(reason) = emit(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::NewWebPageRequested {
                    target_url,
                    is_popup,
                },
            }) {
                return Some(reason);
            }
        }
        IpcEvent::NavigationStateChanged {
            profile_id,
            web_page_id,
            url,
            can_go_back,
            can_go_forward,
            is_loading,
        } => {
            if let Some(reason) = emit(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::NavigationStateChanged {
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
            web_page_id,
            cursor_type,
        } => {
            if let Some(reason) = emit(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::CursorChanged { cursor_type },
            }) {
                return Some(reason);
            }
        }
        IpcEvent::TitleUpdated {
            profile_id,
            web_page_id,
            title,
        } => {
            if let Some(reason) = emit(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::TitleUpdated { title },
            }) {
                return Some(reason);
            }
        }
        IpcEvent::FaviconUrlUpdated {
            profile_id,
            web_page_id,
            url,
        } => {
            if let Some(reason) = emit(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::FaviconUrlUpdated { url },
            }) {
                return Some(reason);
            }
        }
        IpcEvent::BeforeUnloadDialogRequested {
            profile_id,
            web_page_id,
            request_id,
            reason,
        } => {
            if let Some(reason) = emit(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::JavaScriptDialogRequested {
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
        IpcEvent::WebPageClosed {
            profile_id,
            web_page_id,
        } => {
            state.resizes_in_flight.remove(&web_page_id);
            state.pending_resizes.remove(&web_page_id);
            if let Some(reason) = emit(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::Closed,
            }) {
                return Some(reason);
            }
        }
        IpcEvent::ShutdownBlocked {
            request_id,
            dirty_web_page_ids,
        } => {
            if let Some(reason) = emit(BrowserEvent::ShutdownBlocked {
                request_id,
                dirty_web_page_ids,
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
