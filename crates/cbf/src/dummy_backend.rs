use std::{collections::HashMap, thread, time::Duration};

use async_channel::{Receiver, Sender, TryRecvError};

use crate::{
    Backend, Error,
    backend_delegate::{BackendDelegate, DelegateDispatcher},
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
    fn connect<D: BackendDelegate>(
        self,
        delegate: D,
    ) -> Result<(Sender<BrowserCommand>, Receiver<BrowserEvent>), Error> {
        let (command_tx, command_rx) = async_channel::unbounded::<BrowserCommand>();
        let (event_tx, event_rx) = async_channel::unbounded::<BrowserEvent>();

        let next_id = self.next_web_page_id;

        thread::spawn(move || Self::run_communication(command_rx, event_tx, next_id, delegate));

        Ok((command_tx, event_rx))
    }
}

impl DummyBackend {
    fn run_communication(
        command_rx: Receiver<BrowserCommand>,
        event_tx: Sender<BrowserEvent>,
        mut next_id: u64,
        delegate: impl BackendDelegate,
    ) {
        let mut pages: HashMap<WebPageId, String> = HashMap::new();
        let mut dispatcher = DelegateDispatcher::new(delegate);

        if let Some(reason) = dispatcher.dispatch_event(
            BrowserEvent::BackendReady {
                backend_name: "dummy".to_string(),
            },
            &event_tx,
        ) {
            let mut forward = |_| (None, Vec::new());
            dispatcher.stop(&event_tx, reason, &mut forward);
            return;
        }

        const POLL_INTERVAL: Duration = Duration::from_millis(16);

        loop {
            dispatcher.on_idle();
            let mut forward = |command| Self::execute_command(command, &mut pages, &mut next_id);
            if let Some(reason) = dispatcher.flush(&event_tx, &mut forward) {
                dispatcher.stop(&event_tx, reason, &mut forward);
                return;
            }

            let command = match command_rx.try_recv() {
                Ok(command) => command,
                Err(TryRecvError::Empty) => {
                    thread::sleep(POLL_INTERVAL);
                    continue;
                }
                Err(TryRecvError::Closed) => {
                    dispatcher.stop(&event_tx, BackendStopReason::Disconnected, &mut forward);
                    return;
                }
            };

            if let Some(reason) = dispatcher.dispatch_command(command, &event_tx, &mut forward) {
                dispatcher.stop(&event_tx, reason, &mut forward);
                return;
            }

            if let Some(reason) = dispatcher.flush(&event_tx, &mut forward) {
                dispatcher.stop(&event_tx, reason, &mut forward);
                return;
            }
        }
    }

    fn execute_command(
        command: BrowserCommand,
        pages: &mut HashMap<WebPageId, String>,
        next_id: &mut u64,
    ) -> (Option<BackendStopReason>, Vec<BrowserEvent>) {
        let mut events = Vec::new();
        match command {
            BrowserCommand::Shutdown { request_id } => {
                events.push(BrowserEvent::ShutdownProceeding { request_id });
                (Some(BackendStopReason::ShutdownRequested), events)
            }
            BrowserCommand::ConfirmShutdown {
                request_id,
                proceed,
            } => {
                if proceed {
                    events.push(BrowserEvent::ShutdownProceeding { request_id });
                    (Some(BackendStopReason::ShutdownRequested), events)
                } else {
                    events.push(BrowserEvent::ShutdownCancelled { request_id });
                    (None, events)
                }
            }
            BrowserCommand::ForceShutdown => (Some(BackendStopReason::ShutdownRequested), events),
            BrowserCommand::ConfirmBeforeUnload { .. } => (None, events),
            BrowserCommand::CreateWebPage {
                request_id,
                initial_url,
                profile_id: _,
            } => {
                let web_page_id = WebPageId::new(*next_id);
                *next_id += 1;

                let url = initial_url.unwrap_or_else(|| "about:blank".to_string());
                pages.insert(web_page_id, url.clone());

                push_web_page_event(
                    &mut events,
                    web_page_id,
                    WebPageEvent::Created { request_id },
                );
                push_navigation_state(&mut events, web_page_id, pages, false, false);
                push_web_page_event(
                    &mut events,
                    web_page_id,
                    WebPageEvent::TitleUpdated { title: url },
                );
                (None, events)
            }
            BrowserCommand::ListProfiles => {
                events.push(BrowserEvent::ProfilesListed {
                    profiles: Vec::new(),
                });
                (None, events)
            }
            BrowserCommand::Navigate { web_page_id, url } => {
                pages.insert(web_page_id, url.clone());

                push_navigation_state(&mut events, web_page_id, pages, false, true);
                push_web_page_event(
                    &mut events,
                    web_page_id,
                    WebPageEvent::TitleUpdated { title: url },
                );
                push_navigation_state(&mut events, web_page_id, pages, true, false);
                (None, events)
            }
            BrowserCommand::Reload {
                web_page_id,
                ignore_cache: _,
            } => {
                let title = pages
                    .get(&web_page_id)
                    .cloned()
                    .unwrap_or_else(|| "about:blank".to_string());

                push_navigation_state(&mut events, web_page_id, pages, false, true);
                push_web_page_event(
                    &mut events,
                    web_page_id,
                    WebPageEvent::TitleUpdated { title },
                );
                push_navigation_state(&mut events, web_page_id, pages, true, false);
                (None, events)
            }
            BrowserCommand::GetWebPageDomHtml {
                web_page_id,
                request_id,
            } => {
                let html = "<html><body>Dummy DOM</body></html>".to_string();
                push_web_page_event(
                    &mut events,
                    web_page_id,
                    WebPageEvent::DomHtmlRead { request_id, html },
                );
                (None, events)
            }
            BrowserCommand::GoBack { web_page_id } | BrowserCommand::GoForward { web_page_id } => {
                push_navigation_state(&mut events, web_page_id, pages, true, false);
                (None, events)
            }
            BrowserCommand::SetWebPageFocus { .. } | BrowserCommand::ResizeWebPage { .. } => {
                (None, events)
            }
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
            | BrowserCommand::DismissContextMenu { .. } => (None, events),
            BrowserCommand::RequestCloseWebPage { web_page_id } => {
                pages.remove(&web_page_id);
                push_web_page_event(&mut events, web_page_id, WebPageEvent::CloseRequested);
                (None, events)
            }
        }
    }
}

fn push_web_page_event(
    events: &mut Vec<BrowserEvent>,
    web_page_id: WebPageId,
    event: WebPageEvent,
) {
    events.push(BrowserEvent::WebPage {
        profile_id: String::new(),
        web_page_id,
        event,
    });
}

fn push_navigation_state(
    events: &mut Vec<BrowserEvent>,
    web_page_id: WebPageId,
    pages: &HashMap<WebPageId, String>,
    can_go_back: bool,
    is_loading: bool,
) {
    let url = pages
        .get(&web_page_id)
        .cloned()
        .unwrap_or_else(|| "about:blank".to_string());
    push_web_page_event(
        events,
        web_page_id,
        WebPageEvent::NavigationStateChanged {
            url,
            can_go_back,
            can_go_forward: false,
            is_loading,
        },
    );
}
