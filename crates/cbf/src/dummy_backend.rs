use std::{collections::HashMap, thread};

use async_channel::{Receiver, Sender};

use crate::{
    Backend, Error,
    command::BrowserCommand,
    data::ids::WebPageId,
    event::{BackendStopReason, BrowserEvent, WebPageEvent},
};

#[derive(Debug, Default, Clone)]
/// In-memory backend for development and API shaping.
pub struct DummyBackend {
    next_web_page_id: u64,
}

impl DummyBackend {
    /// Create a new dummy backend instance.
    pub fn new() -> Self {
        Self {
            next_web_page_id: 1,
        }
    }
}

impl Backend for DummyBackend {
    fn connect(self) -> Result<(Sender<BrowserCommand>, Receiver<BrowserEvent>), Error> {
        let (command_tx, command_rx) = async_channel::unbounded::<BrowserCommand>();
        let (event_tx, event_rx) = async_channel::unbounded::<BrowserEvent>();

        let next_id = self.next_web_page_id;

        thread::spawn(move || Self::run_communication(command_rx, event_tx, next_id));

        Ok((command_tx, event_rx))
    }
}

impl DummyBackend {
    fn run_communication(
        command_rx: Receiver<BrowserCommand>,
        event_tx: Sender<BrowserEvent>,
        mut next_id: u64,
    ) {
        let mut pages: HashMap<WebPageId, String> = HashMap::new();

        _ = event_tx.send_blocking(BrowserEvent::BackendReady {
            backend_name: "dummy".to_string(),
        });

        loop {
            let command = match command_rx.recv_blocking() {
                Ok(command) => command,
                Err(_) => {
                    _ = event_tx.send_blocking(BrowserEvent::BackendStopped {
                        reason: BackendStopReason::Disconnected,
                    });
                    return;
                }
            };

            if let Some(reason) = Self::handle_command(command, &mut pages, &mut next_id, &event_tx)
            {
                _ = event_tx.send_blocking(BrowserEvent::BackendStopped { reason });
                return;
            }
        }
    }

    fn handle_command(
        command: BrowserCommand,
        pages: &mut HashMap<WebPageId, String>,
        next_id: &mut u64,
        event_tx: &Sender<BrowserEvent>,
    ) -> Option<BackendStopReason> {
        match command {
            BrowserCommand::Shutdown { request_id } => {
                _ = event_tx.send_blocking(BrowserEvent::ShutdownProceeding { request_id });
                Some(BackendStopReason::ShutdownRequested)
            }
            BrowserCommand::ConfirmShutdown {
                request_id,
                proceed,
            } => {
                if proceed {
                    _ = event_tx.send_blocking(BrowserEvent::ShutdownProceeding { request_id });
                    Some(BackendStopReason::ShutdownRequested)
                } else {
                    _ = event_tx.send_blocking(BrowserEvent::ShutdownCancelled { request_id });
                    None
                }
            }
            BrowserCommand::ForceShutdown => Some(BackendStopReason::ShutdownRequested),
            BrowserCommand::ConfirmBeforeUnload { .. } => None,
            BrowserCommand::CreateWebPage {
                request_id,
                initial_url,
                profile_id: _,
            } => {
                let web_page_id = WebPageId::new(*next_id);
                *next_id += 1;

                let url = initial_url.unwrap_or_else(|| "about:blank".to_string());
                pages.insert(web_page_id, url.clone());

                send_web_page_event(event_tx, web_page_id, WebPageEvent::Created { request_id });
                send_navigation_state(event_tx, web_page_id, pages, false, false);
                send_web_page_event(
                    event_tx,
                    web_page_id,
                    WebPageEvent::TitleUpdated { title: url },
                );
                None
            }
            BrowserCommand::ListProfiles => {
                _ = event_tx.send_blocking(BrowserEvent::ProfilesListed {
                    profiles: Vec::new(),
                });
                None
            }
            BrowserCommand::Navigate { web_page_id, url } => {
                pages.insert(web_page_id, url.clone());

                send_navigation_state(event_tx, web_page_id, pages, false, true);
                send_web_page_event(
                    event_tx,
                    web_page_id,
                    WebPageEvent::TitleUpdated { title: url },
                );
                send_navigation_state(event_tx, web_page_id, pages, true, false);
                None
            }
            BrowserCommand::Reload {
                web_page_id,
                ignore_cache: _,
            } => {
                let title = pages
                    .get(&web_page_id)
                    .cloned()
                    .unwrap_or_else(|| "about:blank".to_string());

                send_navigation_state(event_tx, web_page_id, pages, false, true);
                send_web_page_event(event_tx, web_page_id, WebPageEvent::TitleUpdated { title });
                send_navigation_state(event_tx, web_page_id, pages, true, false);
                None
            }
            BrowserCommand::GetWebPageDomHtml {
                web_page_id,
                request_id,
            } => {
                let html = "<html><body>Dummy DOM</body></html>".to_string();
                send_web_page_event(
                    event_tx,
                    web_page_id,
                    WebPageEvent::DomHtmlRead { request_id, html },
                );
                None
            }
            BrowserCommand::GoBack { web_page_id } | BrowserCommand::GoForward { web_page_id } => {
                send_navigation_state(event_tx, web_page_id, pages, true, false);
                None
            }
            BrowserCommand::SetWebPageFocus { .. } | BrowserCommand::ResizeWebPage { .. } => None,
            BrowserCommand::SendKeyEvent { .. }
            | BrowserCommand::SendMouseEvent { .. }
            | BrowserCommand::SendMouseWheelEvent { .. }
            | BrowserCommand::SendDragUpdate { .. }
            | BrowserCommand::SendDragDrop { .. }
            | BrowserCommand::SendDragCancel { .. }
            | BrowserCommand::SetComposition { .. }
            | BrowserCommand::CommitText { .. }
            | BrowserCommand::FinishComposingText { .. }
            | BrowserCommand::ExecuteContextMenuCommand { .. }
            | BrowserCommand::DismissContextMenu { .. } => None,
            BrowserCommand::RequestCloseWebPage { web_page_id } => {
                pages.remove(&web_page_id);
                send_web_page_event(event_tx, web_page_id, WebPageEvent::CloseRequested);
                None
            }
        }
    }
}

fn send_web_page_event(
    event_tx: &Sender<BrowserEvent>,
    web_page_id: WebPageId,
    event: WebPageEvent,
) {
    _ = event_tx.send_blocking(BrowserEvent::WebPage {
        profile_id: String::new(),
        web_page_id,
        event,
    });
}

fn send_navigation_state(
    event_tx: &Sender<BrowserEvent>,
    web_page_id: WebPageId,
    pages: &HashMap<WebPageId, String>,
    can_go_back: bool,
    is_loading: bool,
) {
    let url = pages
        .get(&web_page_id)
        .cloned()
        .unwrap_or_else(|| "about:blank".to_string());
    send_web_page_event(
        event_tx,
        web_page_id,
        WebPageEvent::NavigationStateChanged {
            url,
            can_go_back,
            can_go_forward: false,
            is_loading,
        },
    );
}
