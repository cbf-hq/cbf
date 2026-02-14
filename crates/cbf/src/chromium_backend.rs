use std::{
    collections::{HashMap, HashSet},
    thread,
    time::{Duration, Instant},
};

use crate::ffi::{IpcClient, IpcEvent};
use async_channel::{Receiver, Sender, TryRecvError};
use tracing::{debug, info, info_span, warn};

use crate::{
    Backend, Error,
    command::BrowserCommand,
    data::ids::WebPageId,
    event::{BackendStopReason, BrowserEvent, DialogResponse, DialogType, WebPageEvent},
};
use oneshot;

#[derive(Debug, Clone)]
/// Backend implementation that speaks the Chromium IPC protocol.
pub struct ChromiumBackend {
    channel_name: String,
}

struct CommunicationState {
    resizes_in_flight: HashSet<WebPageId>,
    pending_resizes: HashMap<WebPageId, (u32, u32)>,
}

impl Backend for ChromiumBackend {
    fn connect(self) -> Result<(Sender<BrowserCommand>, Receiver<BrowserEvent>), Error> {
        let (command_tx, command_rx) = async_channel::unbounded::<BrowserCommand>();
        let (event_tx, event_rx) = async_channel::unbounded::<BrowserEvent>();
        let channel_name = self.channel_name;

        thread::spawn({
            let command_tx = command_tx.clone();

            move || Self::run_communication(channel_name, command_rx, event_tx, command_tx)
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

    fn run_communication(
        channel_name: String,
        command_rx: Receiver<BrowserCommand>,
        event_tx: Sender<BrowserEvent>,
        command_tx: Sender<BrowserCommand>,
    ) {
        let timeout = Duration::from_secs(5);
        let start_time = Instant::now();
        let retry_interval = Duration::from_millis(100);

        let mut client = loop {
            match IpcClient::connect(&channel_name) {
                Ok(client) => break client,
                Err(err) => {
                    if start_time.elapsed() > timeout {
                        warn!(
                            result = "err",
                            error = "ipc_connect_timeout",
                            err = ?err,
                            "IPC connect timed out"
                        );
                        _ = event_tx.send_blocking(BrowserEvent::BackendStopped {
                            reason: BackendStopReason::Error {
                                message: format!("IPC connect timed out: {err:?}"),
                            },
                        });
                        return;
                    }
                    thread::sleep(retry_interval);
                }
            }
        };

        info!(channel = %channel_name, "CBF backend ready");
        _ = event_tx.send_blocking(BrowserEvent::BackendReady {
            backend_name: "chromium".to_string(),
        });

        let mut state = CommunicationState {
            resizes_in_flight: HashSet::new(),
            pending_resizes: HashMap::new(),
        };

        const POLL_INTERVAL: Duration = Duration::from_millis(16);

        loop {
            if let Some(stop_reason) =
                Self::process_command_queue(&command_rx, &mut client, &event_tx, &mut state)
            {
                warn!(
                    result = "err",
                    error = "backend_stopped",
                    reason = ?stop_reason,
                    "CBF backend stopped"
                );
                _ = event_tx.send_blocking(BrowserEvent::BackendStopped {
                    reason: stop_reason,
                });
                return;
            };

            if let Some(stop_reason) =
                Self::process_event_queue(&mut client, &event_tx, &command_tx, &mut state)
            {
                warn!(
                    result = "err",
                    error = "backend_stopped",
                    reason = ?stop_reason,
                    "CBF backend stopped"
                );
                _ = event_tx.send_blocking(BrowserEvent::BackendStopped {
                    reason: stop_reason,
                });
                return;
            };

            thread::sleep(POLL_INTERVAL);
        }
    }

    fn process_event_queue(
        client: &mut IpcClient,
        event_tx: &Sender<BrowserEvent>,
        command_tx: &Sender<BrowserCommand>,
        state: &mut CommunicationState,
    ) -> Option<BackendStopReason> {
        while let Some(event) = client.poll_event() {
            match event {
                Ok(event) => handle_ipc_event(event, event_tx, command_tx, client, state),
                Err(err) => {
                    warn!(
                        result = "err",
                        error = "ipc_event_error",
                        err = ?err,
                        "IPC event error"
                    );
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

            let mut handle_one = |command| match Self::handle_command(command, client, event_tx) {
                Ok(Some(reason)) => Some(reason),
                Ok(None) => None,
                Err(message) => {
                    warn!(
                        result = "err",
                        error = "command_failed",
                        message = %message,
                        "CBF command failed"
                    );
                    Some(BackendStopReason::Error { message })
                }
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
                    if let Some(reason) = handle_one(BrowserCommand::ResizeWebPage {
                        web_page_id,
                        width,
                        height,
                    }) {
                        return Some(reason);
                    }
                }

                continue;
            }

            if let Some(reason) = handle_one(command) {
                return Some(reason);
            }
        }
    }

    fn handle_command(
        command: BrowserCommand,
        client: &mut IpcClient,
        event_tx: &Sender<BrowserEvent>,
    ) -> Result<Option<BackendStopReason>, String> {
        let span = info_span!("cbf.command", command = ?command);
        let _span = span.enter();
        debug!("Handle CBF command");

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
    command_tx: &Sender<BrowserCommand>,
    client: &mut IpcClient,
    state: &mut CommunicationState,
) {
    match event {
        IpcEvent::WebPageCreated {
            profile_id,
            web_page_id,
            request_id,
        } => {
            _ = event_tx.send_blocking(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::Created { request_id },
            });
        }
        IpcEvent::SurfaceHandleUpdated {
            profile_id,
            web_page_id,
            handle,
        } => {
            _ = event_tx.send_blocking(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::SurfaceHandleUpdated { handle },
            });
        }
        IpcEvent::WebPageResizeAcknowledged { web_page_id, .. } => {
            state.resizes_in_flight.remove(&web_page_id);
            if let Some((width, height)) = state.pending_resizes.remove(&web_page_id) {
                state.resizes_in_flight.insert(web_page_id);
                if let Err(err) = client.set_web_page_size(web_page_id, width, height) {
                    warn!(
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
            _ = event_tx.send_blocking(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::DomHtmlRead { request_id, html },
            });
        }
        IpcEvent::DragStartRequested {
            profile_id,
            web_page_id,
            request,
        } => {
            _ = event_tx.send_blocking(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::DragStartRequested { request },
            });
        }
        IpcEvent::ImeBoundsUpdated {
            profile_id,
            web_page_id,
            update,
        } => {
            _ = event_tx.send_blocking(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::ImeBoundsUpdated { update },
            });
        }
        IpcEvent::ContextMenuRequested {
            profile_id,
            web_page_id,
            menu,
        } => {
            _ = event_tx.send_blocking(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::ContextMenuRequested { menu },
            });
        }
        IpcEvent::NewWebPageRequested {
            profile_id,
            web_page_id,
            target_url,
            is_popup,
        } => {
            _ = event_tx.send_blocking(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::NewWebPageRequested {
                    target_url,
                    is_popup,
                },
            });
        }
        IpcEvent::NavigationStateChanged {
            profile_id,
            web_page_id,
            url,
            can_go_back,
            can_go_forward,
            is_loading,
        } => {
            _ = event_tx.send_blocking(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::NavigationStateChanged {
                    url,
                    can_go_back,
                    can_go_forward,
                    is_loading,
                },
            });
        }
        IpcEvent::CursorChanged {
            profile_id,
            web_page_id,
            cursor_type,
        } => {
            _ = event_tx.send_blocking(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::CursorChanged { cursor_type },
            });
        }
        IpcEvent::TitleUpdated {
            profile_id,
            web_page_id,
            title,
        } => {
            _ = event_tx.send_blocking(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::TitleUpdated { title },
            });
        }
        IpcEvent::FaviconUrlUpdated {
            profile_id,
            web_page_id,
            url,
        } => {
            _ = event_tx.send_blocking(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::FaviconUrlUpdated { url },
            });
        }
        IpcEvent::BeforeUnloadDialogRequested {
            profile_id,
            web_page_id,
            request_id,
            reason,
        } => {
            debug!(
                ?profile_id,
                %web_page_id,
                request_id,
                ?reason,
                "BeforeUnloadDialogRequested received"
            );
            let (response_tx, response_rx) = oneshot::channel();
            _ = event_tx.send_blocking(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::JavaScriptDialogRequested {
                    message: String::new(),
                    default_prompt_text: None,
                    r#type: DialogType::BeforeUnload,
                    beforeunload_reason: Some(reason),
                    response_channel: response_tx,
                },
            });

            let command_tx = command_tx.clone();
            thread::spawn(move || {
                if let Ok(response) = response_rx.recv() {
                    let proceed = matches!(response, DialogResponse::Success { .. });
                    command_tx
                        .send_blocking(BrowserCommand::ConfirmBeforeUnload {
                            web_page_id,
                            request_id,
                            proceed,
                        })
                        .ok();
                }
            });
        }
        IpcEvent::WebPageClosed {
            profile_id,
            web_page_id,
        } => {
            state.resizes_in_flight.remove(&web_page_id);
            state.pending_resizes.remove(&web_page_id);
            _ = event_tx.send_blocking(BrowserEvent::WebPage {
                profile_id,
                web_page_id,
                event: WebPageEvent::Closed,
            });
        }
        IpcEvent::ShutdownBlocked {
            request_id,
            dirty_web_page_ids,
        } => {
            _ = event_tx.send_blocking(BrowserEvent::ShutdownBlocked {
                request_id,
                dirty_web_page_ids,
            });
        }
        IpcEvent::ShutdownProceeding { request_id } => {
            _ = event_tx.send_blocking(BrowserEvent::ShutdownProceeding { request_id });
        }
        IpcEvent::ShutdownCancelled { request_id } => {
            _ = event_tx.send_blocking(BrowserEvent::ShutdownCancelled { request_id });
        }
    }
}
