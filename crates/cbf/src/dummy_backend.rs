//! A dummy in-memory backend implementation for development and API shaping.

use std::{collections::HashMap, thread, time::Duration};

use async_channel::{Receiver, Sender, TryRecvError};

use crate::{
    browser::{Backend, CommandEnvelope, CommandSender, EventStream},
    command::BrowserCommand,
    data::ids::BrowsingContextId,
    delegate::{BackendDelegate, CommandDecision, DelegateDispatcher, EventDecision},
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
    type RawCommand = BrowserCommand;
    type RawEvent = BrowserEvent;
    type RawDelegate = ();

    fn to_raw_command(command: BrowserCommand) -> Self::RawCommand {
        command
    }

    fn to_generic_event(raw: &Self::RawEvent) -> Option<BrowserEvent> {
        Some(raw.clone())
    }

    fn connect<D: BackendDelegate>(
        self,
        delegate: D,
        _raw_delegate: Option<Self::RawDelegate>,
    ) -> Result<(CommandSender<Self>, EventStream<Self>), Error> {
        let (command_tx, command_rx) = async_channel::unbounded::<CommandEnvelope<Self>>();
        let (event_tx, event_rx) = async_channel::unbounded::<BrowserEvent>();

        let next_id = self.next_browsing_context_id;

        thread::spawn(move || Self::run_communication(command_rx, event_tx, next_id, delegate));

        Ok((
            CommandSender::from_raw_sender(command_tx),
            EventStream::from_raw_receiver(event_rx),
        ))
    }
}

impl DummyBackend {
    fn emit_event(event_tx: &Sender<BrowserEvent>, event: BrowserEvent) {
        _ = event_tx.send_blocking(event);
    }

    fn dispatch_generic_event(
        event: BrowserEvent,
        event_tx: &Sender<BrowserEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
    ) -> Option<BackendStopReason> {
        match dispatcher.dispatch_event(&event) {
            EventDecision::Forward => {
                Self::emit_event(event_tx, event);
                None
            }
            EventDecision::Stop(reason) => Some(reason),
        }
    }

    fn run_generic_command_with_delegate(
        command: BrowserCommand,
        event_tx: &Sender<BrowserEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
        pages: &mut HashMap<BrowsingContextId, String>,
        next_id: &mut u64,
    ) -> Option<BackendStopReason> {
        match dispatcher.dispatch_command(&command) {
            CommandDecision::Forward => {
                let (reason, events) = Self::execute_command(command, pages, next_id);
                for event in events {
                    if let Some(reason) = Self::dispatch_generic_event(event, event_tx, dispatcher)
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

    fn drain_delegate_queue(
        event_tx: &Sender<BrowserEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
        pages: &mut HashMap<BrowsingContextId, String>,
        next_id: &mut u64,
        mut pending_commands: Vec<BrowserCommand>,
    ) -> Option<BackendStopReason> {
        loop {
            for command in pending_commands {
                if let Some(reason) = Self::run_generic_command_with_delegate(
                    command, event_tx, dispatcher, pages, next_id,
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
        event_tx: &Sender<BrowserEvent>,
        dispatcher: &mut DelegateDispatcher<impl BackendDelegate>,
        pages: &mut HashMap<BrowsingContextId, String>,
        next_id: &mut u64,
    ) {
        let (mut final_reason, queued_commands) = dispatcher.stop(reason);
        if let Some(reason) =
            Self::drain_delegate_queue(event_tx, dispatcher, pages, next_id, queued_commands)
        {
            final_reason = reason;
        }
        Self::emit_event(
            event_tx,
            BrowserEvent::BackendStopped {
                reason: final_reason,
            },
        );
    }

    fn run_communication(
        command_rx: Receiver<CommandEnvelope<Self>>,
        event_tx: Sender<BrowserEvent>,
        mut next_id: u64,
        delegate: impl BackendDelegate,
    ) {
        let mut pages: HashMap<BrowsingContextId, String> = HashMap::new();
        let mut dispatcher = DelegateDispatcher::new(delegate);

        let ready_event = BrowserEvent::BackendReady;
        if let Some(reason) = Self::dispatch_generic_event(ready_event, &event_tx, &mut dispatcher)
        {
            Self::stop_backend(reason, &event_tx, &mut dispatcher, &mut pages, &mut next_id);
            return;
        }

        const POLL_INTERVAL: Duration = Duration::from_millis(16);

        loop {
            dispatcher.on_idle();
            let queued_commands = dispatcher.flush();
            if let Some(reason) = Self::drain_delegate_queue(
                &event_tx,
                &mut dispatcher,
                &mut pages,
                &mut next_id,
                queued_commands,
            ) {
                Self::stop_backend(reason, &event_tx, &mut dispatcher, &mut pages, &mut next_id);
                return;
            }

            let envelope = match command_rx.try_recv() {
                Ok(envelope) => envelope,
                Err(TryRecvError::Empty) => {
                    thread::sleep(POLL_INTERVAL);
                    continue;
                }
                Err(TryRecvError::Closed) => {
                    Self::stop_backend(
                        BackendStopReason::Disconnected,
                        &event_tx,
                        &mut dispatcher,
                        &mut pages,
                        &mut next_id,
                    );
                    return;
                }
            };

            match envelope {
                CommandEnvelope::Generic { command, .. } => {
                    if let Some(reason) = Self::run_generic_command_with_delegate(
                        command,
                        &event_tx,
                        &mut dispatcher,
                        &mut pages,
                        &mut next_id,
                    ) {
                        Self::stop_backend(
                            reason,
                            &event_tx,
                            &mut dispatcher,
                            &mut pages,
                            &mut next_id,
                        );
                        return;
                    }
                }
                CommandEnvelope::RawOnly { raw } => {
                    let (reason, events) = Self::execute_command(raw, &mut pages, &mut next_id);
                    for event in events {
                        if let Some(reason) =
                            Self::dispatch_generic_event(event, &event_tx, &mut dispatcher)
                        {
                            Self::stop_backend(
                                reason,
                                &event_tx,
                                &mut dispatcher,
                                &mut pages,
                                &mut next_id,
                            );
                            return;
                        }
                    }
                    if let Some(reason) = reason {
                        Self::stop_backend(
                            reason,
                            &event_tx,
                            &mut dispatcher,
                            &mut pages,
                            &mut next_id,
                        );
                        return;
                    }
                }
            }

            let queued_commands = dispatcher.flush();
            if let Some(reason) = Self::drain_delegate_queue(
                &event_tx,
                &mut dispatcher,
                &mut pages,
                &mut next_id,
                queued_commands,
            ) {
                Self::stop_backend(reason, &event_tx, &mut dispatcher, &mut pages, &mut next_id);
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
            BrowserCommand::ListExtensions { profile_id } => {
                events.push(BrowserEvent::ExtensionsListed {
                    profile_id: profile_id.unwrap_or_default(),
                    extensions: Vec::new(),
                });
                (None, events)
            }
            BrowserCommand::Navigate {
                browsing_context_id,
                url,
            } => {
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
            BrowserCommand::GoBack {
                browsing_context_id,
            }
            | BrowserCommand::GoForward {
                browsing_context_id,
            } => {
                push_navigation_state(&mut events, browsing_context_id, pages, true, false);
                (None, events)
            }
            BrowserCommand::SetBrowsingContextFocus { .. }
            | BrowserCommand::ResizeBrowsingContext { .. } => (None, events),
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
            | BrowserCommand::DismissContextMenu { .. }
            | BrowserCommand::OpenDefaultAuxiliaryWindow { .. }
            | BrowserCommand::RespondAuxiliaryWindow { .. }
            | BrowserCommand::CloseAuxiliaryWindow { .. }
            | BrowserCommand::RespondBrowsingContextOpen { .. }
            | BrowserCommand::RespondWindowOpen { .. } => (None, events),
            BrowserCommand::RequestCloseBrowsingContext {
                browsing_context_id,
            } => {
                pages.remove(&browsing_context_id);
                push_browsing_context_event(
                    &mut events,
                    browsing_context_id,
                    BrowsingContextEvent::CloseRequested,
                );
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
        event: Box::new(event),
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
