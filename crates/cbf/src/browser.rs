use async_channel::{Receiver, Sender, TrySendError};

use crate::{
    error::Error,
    backend_delegate::BackendDelegate,
    command::BrowserCommand,
    data::{
        drag::{DragDrop, DragUpdate},
        ids::BrowsingContextId,
        ime::{ConfirmCompositionBehavior, ImeCommitText, ImeComposition},
        key::KeyEvent,
        mouse::{MouseEvent, MouseWheelEvent},
    },
    event::BrowserEvent,
};

/// Channel sender used to push `BrowserCommand` to a backend.
pub type CommandSender = Sender<BrowserCommand>;
/// Stream of `BrowserEvent` emitted by a backend.
pub type EventStream = Receiver<BrowserEvent>;

/// A backend implementation that can drive Chromium (or a Chromium fork).
///
/// `cbf` keeps this trait small on purpose: the high-level API surface is
/// expressed via commands and events, and transport details live behind this.
pub trait Backend: Send + 'static {
    /// Establish a command/event channel pair for this backend.
    fn connect<D: BackendDelegate>(
        self,
        delegate: D,
    ) -> Result<(CommandSender, EventStream), Error>;
}

/// A clonable handle used to send commands to the browser backend.
#[derive(Debug, Clone)]
pub struct BrowserHandle {
    command_tx: CommandSender,
}

impl BrowserHandle {
    pub(crate) fn new(command_tx: CommandSender) -> Self {
        Self { command_tx }
    }

    /// Send a raw browser command to the backend.
    pub fn send(&self, command: BrowserCommand) -> Result<(), Error> {
        match self.command_tx.try_send(command) {
            Ok(()) => Ok(()),
            Err(TrySendError::Closed(_)) => Err(Error::Disconnected),
            Err(TrySendError::Full(_)) => Err(Error::QueueFull),
        }
    }

    /// Create a new web page (tab) with an optional initial URL and profile.
    pub fn create_browsing_context(
        &self,
        request_id: u64,
        initial_url: Option<String>,
        profile_id: Option<String>,
    ) -> Result<(), Error> {
        self.send(BrowserCommand::CreateBrowsingContext {
            request_id,
            initial_url,
            profile_id,
        })
    }

    pub fn resize_browsing_context(
        &self,
        browsing_context_id: BrowsingContextId,
        width: u32,
        height: u32,
    ) -> Result<(), Error> {
        self.send(BrowserCommand::ResizeBrowsingContext {
            browsing_context_id,
            width,
            height,
        })
    }

    /// Request closing the given web page.
    pub fn request_close_browsing_context(&self, browsing_context_id: BrowsingContextId) -> Result<(), Error> {
        self.send(BrowserCommand::RequestCloseBrowsingContext { browsing_context_id })
    }

    /// Navigate the web page to the provided URL.
    pub fn navigate(&self, browsing_context_id: BrowsingContextId, url: String) -> Result<(), Error> {
        self.send(BrowserCommand::Navigate { browsing_context_id, url })
    }

    /// Navigate back in history for the given web page.
    pub fn go_back(&self, browsing_context_id: BrowsingContextId) -> Result<(), Error> {
        self.send(BrowserCommand::GoBack { browsing_context_id })
    }

    /// Navigate forward in history for the given web page.
    pub fn go_forward(&self, browsing_context_id: BrowsingContextId) -> Result<(), Error> {
        self.send(BrowserCommand::GoForward { browsing_context_id })
    }

    /// Reload the current page, optionally bypassing caches.
    pub fn reload(&self, browsing_context_id: BrowsingContextId, ignore_cache: bool) -> Result<(), Error> {
        self.send(BrowserCommand::Reload {
            browsing_context_id,
            ignore_cache,
        })
    }

    pub fn get_browsing_context_dom_html(
        &self,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
    ) -> Result<(), Error> {
        self.send(BrowserCommand::GetBrowsingContextDomHtml {
            browsing_context_id,
            request_id,
        })
    }

    /// Update whether the web page should receive text input focus.
    pub fn set_browsing_context_focus(&self, browsing_context_id: BrowsingContextId, focused: bool) -> Result<(), Error> {
        self.send(BrowserCommand::SetBrowsingContextFocus {
            browsing_context_id,
            focused,
        })
    }

    /// Request the list of available profiles from the backend.
    pub fn request_list_profiles(&self) -> Result<(), Error> {
        self.send(BrowserCommand::ListProfiles)
    }

    /// Send a keyboard input event to the web page.
    pub fn send_key_event(
        &self,
        browsing_context_id: BrowsingContextId,
        event: KeyEvent,
        commands: Vec<String>,
    ) -> Result<(), Error> {
        self.send(BrowserCommand::SendKeyEvent {
            browsing_context_id,
            event,
            commands,
        })
    }

    /// Send a mouse input event to the web page.
    pub fn send_mouse_event(&self, browsing_context_id: BrowsingContextId, event: MouseEvent) -> Result<(), Error> {
        self.send(BrowserCommand::SendMouseEvent { browsing_context_id, event })
    }

    /// Send a mouse wheel event to the web page.
    pub fn send_mouse_wheel_event(
        &self,
        browsing_context_id: BrowsingContextId,
        event: MouseWheelEvent,
    ) -> Result<(), Error> {
        self.send(BrowserCommand::SendMouseWheelEvent { browsing_context_id, event })
    }

    /// Send a drag update event for host-owned drag session.
    pub fn send_drag_update(&self, update: DragUpdate) -> Result<(), Error> {
        self.send(BrowserCommand::SendDragUpdate { update })
    }

    /// Send a drop event for host-owned drag session.
    pub fn send_drag_drop(&self, drop: DragDrop) -> Result<(), Error> {
        self.send(BrowserCommand::SendDragDrop { drop })
    }

    /// Cancel a host-owned drag session.
    pub fn send_drag_cancel(&self, session_id: u64, browsing_context_id: BrowsingContextId) -> Result<(), Error> {
        self.send(BrowserCommand::SendDragCancel {
            session_id,
            browsing_context_id,
        })
    }

    /// Update the current IME composition state.
    pub fn set_composition(&self, composition: ImeComposition) -> Result<(), Error> {
        self.send(BrowserCommand::SetComposition { composition })
    }

    /// Commit IME text input to the web page.
    pub fn commit_text(&self, commit: ImeCommitText) -> Result<(), Error> {
        self.send(BrowserCommand::CommitText { commit })
    }

    /// Finish IME composing with the specified selection behavior.
    pub fn finish_composing_text(
        &self,
        browsing_context_id: BrowsingContextId,
        behavior: ConfirmCompositionBehavior,
    ) -> Result<(), Error> {
        self.send(BrowserCommand::FinishComposingText {
            browsing_context_id,
            behavior,
        })
    }

    /// Execute a context menu command produced by the backend.
    pub fn execute_context_menu_command(
        &self,
        menu_id: u64,
        command_id: i32,
        event_flags: i32,
    ) -> Result<(), Error> {
        self.send(BrowserCommand::ExecuteContextMenuCommand {
            menu_id,
            command_id,
            event_flags,
        })
    }

    /// Dismiss an open context menu by menu id.
    pub fn dismiss_context_menu(&self, menu_id: u64) -> Result<(), Error> {
        self.send(BrowserCommand::DismissContextMenu { menu_id })
    }

    /// Request a graceful shutdown flow.
    pub fn request_shutdown(&self, request_id: u64) -> Result<(), Error> {
        self.send(BrowserCommand::Shutdown { request_id })
    }

    /// Respond to a shutdown confirmation request.
    pub fn confirm_shutdown(&self, request_id: u64, proceed: bool) -> Result<(), Error> {
        self.send(BrowserCommand::ConfirmShutdown {
            request_id,
            proceed,
        })
    }

    /// Respond to a beforeunload confirmation request for a page.
    pub fn confirm_beforeunload(
        &self,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        proceed: bool,
    ) -> Result<(), Error> {
        self.send(BrowserCommand::ConfirmBeforeUnload {
            browsing_context_id,
            request_id,
            proceed,
        })
    }

    /// Respond to a permission request for a page.
    pub fn confirm_permission(
        &self,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        allow: bool,
    ) -> Result<(), Error> {
        self.send(BrowserCommand::ConfirmPermission {
            browsing_context_id,
            request_id,
            allow,
        })
    }

    /// Force shutdown without waiting for confirmations.
    pub fn force_shutdown(&self) -> Result<(), Error> {
        self.send(BrowserCommand::ForceShutdown)
    }
}

/// A session that owns the initial command handle.
#[derive(Debug)]
pub struct BrowserSession {
    handle: BrowserHandle,
}

impl BrowserSession {
    pub(crate) fn new(command_tx: CommandSender) -> Self {
        Self {
            handle: BrowserHandle::new(command_tx),
        }
    }

    /// Get a cloneable handle for issuing browser commands.
    pub fn handle(&self) -> BrowserHandle {
        self.handle.clone()
    }
}

/// Connect to a backend and obtain a command session and an event stream.
///
/// This split form is the minimum core API: most applications want to drive
/// the backend from one place, while consuming events elsewhere.
pub fn connect<B: Backend, D: BackendDelegate>(
    backend: B,
    delegate: D,
) -> Result<(BrowserSession, EventStream), Error> {
    let (command_tx, events) = backend.connect(delegate)?;
    Ok((BrowserSession::new(command_tx), events))
}
