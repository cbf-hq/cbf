use std::{collections::HashMap, thread, time::Duration};

use async_channel::{Receiver, Sender, TryRecvError};

use crate::{
    backend_delegate::{BackendDelegate, DelegateDispatcher},
    browser::Backend,
    command::BrowserCommand,
    data::ids::BrowsingContextId,
    error::Error,
    event::{BackendStopReason, BrowserEvent, BrowsingContextEvent},
};

/// In-memory backend for development and API shaping.
#[derive(Debug, Default, Clone)]
pub struct DummyBackend {
    next_browsing_context_id: u64,
}

impl DummyBackend {
    /// Create a new dummy backend instance.
    pub fn new() -> Self {
        Self {
            next_browsing_context_id: 1,
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

        let next_id = self.next_browsing_context_id;

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
        let mut pages: HashMap<BrowsingContextId, String> = HashMap::new();
        let mut dispatcher = DelegateDispatcher::new(delegate);

        if let Some(reason) = dispatcher.dispatch_event(BrowserEvent::BackendReady, &event_tx) {
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
        pages: &mut HashMap<BrowsingContextId, String>,
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
            BrowserCommand::ConfirmPermission { .. } => (None, events),
            BrowserCommand::CreateBrowsingContext {
                request_id,
                initial_url,
                profile_id: _,
            } => {
                let browsing_context_id = BrowsingContextId::new(*next_id);
                *next_id += 1;

                let url = initial_url.unwrap_or_else(|| "about:blank".to_string());
                pages.insert(browsing_context_id, url.clone());

                push_browsing_context_event(
                    &mut events,
                    browsing_context_id,
                    BrowsingContextEvent::Created { request_id },
                );
                push_navigation_state(&mut events, browsing_context_id, pages, false, false);
                push_browsing_context_event(
                    &mut events,
                    browsing_context_id,
                    BrowsingContextEvent::TitleUpdated { title: url },
                );
                (None, events)
            }
            BrowserCommand::ListProfiles => {
                events.push(BrowserEvent::ProfilesListed {
                    profiles: Vec::new(),
                });
                (None, events)
            }
            BrowserCommand::Navigate { browsing_context_id, url } => {
                pages.insert(browsing_context_id, url.clone());

                push_navigation_state(&mut events, browsing_context_id, pages, false, true);
                push_browsing_context_event(
                    &mut events,
                    browsing_context_id,
                    BrowsingContextEvent::TitleUpdated { title: url },
                );
                push_navigation_state(&mut events, browsing_context_id, pages, true, false);
                (None, events)
            }
            BrowserCommand::Reload {
                browsing_context_id,
                ignore_cache: _,
            } => {
                let title = pages
                    .get(&browsing_context_id)
                    .cloned()
                    .unwrap_or_else(|| "about:blank".to_string());

                push_navigation_state(&mut events, browsing_context_id, pages, false, true);
                push_browsing_context_event(
                    &mut events,
                    browsing_context_id,
                    BrowsingContextEvent::TitleUpdated { title },
                );
                push_navigation_state(&mut events, browsing_context_id, pages, true, false);
                (None, events)
            }
            BrowserCommand::GetBrowsingContextDomHtml {
                browsing_context_id,
                request_id,
            } => {
                let html = "<html><body>Dummy DOM</body></html>".to_string();
                push_browsing_context_event(
                    &mut events,
                    browsing_context_id,
                    BrowsingContextEvent::DomHtmlRead { request_id, html },
                );
                (None, events)
            }
            BrowserCommand::GoBack { browsing_context_id } | BrowserCommand::GoForward { browsing_context_id } => {
                push_navigation_state(&mut events, browsing_context_id, pages, true, false);
                (None, events)
            }
            BrowserCommand::SetBrowsingContextFocus { .. } | BrowserCommand::ResizeBrowsingContext { .. } => {
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
            BrowserCommand::RequestCloseBrowsingContext { browsing_context_id } => {
                pages.remove(&browsing_context_id);
                push_browsing_context_event(&mut events, browsing_context_id, BrowsingContextEvent::CloseRequested);
                (None, events)
            }
        }
    }
}

fn push_browsing_context_event(
    events: &mut Vec<BrowserEvent>,
    browsing_context_id: BrowsingContextId,
    event: BrowsingContextEvent,
) {
    events.push(BrowserEvent::BrowsingContext {
        profile_id: String::new(),
        browsing_context_id,
        event,
    });
}

fn push_navigation_state(
    events: &mut Vec<BrowserEvent>,
    browsing_context_id: BrowsingContextId,
    pages: &HashMap<BrowsingContextId, String>,
    can_go_back: bool,
    is_loading: bool,
) {
    let url = pages
        .get(&browsing_context_id)
        .cloned()
        .unwrap_or_else(|| "about:blank".to_string());
    push_browsing_context_event(
        events,
        browsing_context_id,
        BrowsingContextEvent::NavigationStateChanged {
            url,
            can_go_back,
            can_go_forward: false,
            is_loading,
        },
    );
}
