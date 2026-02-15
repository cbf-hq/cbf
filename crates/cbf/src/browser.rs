use async_channel::{Receiver, Sender, TrySendError};

use crate::{
    backend_delegate::BackendDelegate, Error,
    command::BrowserCommand,
    data::{
        drag::{DragDrop, DragUpdate},
        ids::WebPageId,
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
    pub fn create_web_page(
        &self,
        request_id: u64,
        initial_url: Option<String>,
        profile_id: Option<String>,
    ) -> Result<(), Error> {
        self.send(BrowserCommand::CreateWebPage {
            request_id,
            initial_url,
            profile_id,
        })
    }

    pub fn resize_web_page(
        &self,
        web_page_id: WebPageId,
        width: u32,
        height: u32,
    ) -> Result<(), Error> {
        self.send(BrowserCommand::ResizeWebPage {
            web_page_id,
            width,
            height,
        })
    }

    /// Request closing the given web page.
    pub fn request_close_web_page(&self, web_page_id: WebPageId) -> Result<(), Error> {
        self.send(BrowserCommand::RequestCloseWebPage { web_page_id })
    }

    /// Navigate the web page to the provided URL.
    pub fn navigate(&self, web_page_id: WebPageId, url: String) -> Result<(), Error> {
        self.send(BrowserCommand::Navigate { web_page_id, url })
    }

    /// Navigate back in history for the given web page.
    pub fn go_back(&self, web_page_id: WebPageId) -> Result<(), Error> {
        self.send(BrowserCommand::GoBack { web_page_id })
    }

    /// Navigate forward in history for the given web page.
    pub fn go_forward(&self, web_page_id: WebPageId) -> Result<(), Error> {
        self.send(BrowserCommand::GoForward { web_page_id })
    }

    /// Reload the current page, optionally bypassing caches.
    pub fn reload(&self, web_page_id: WebPageId, ignore_cache: bool) -> Result<(), Error> {
        self.send(BrowserCommand::Reload {
            web_page_id,
            ignore_cache,
        })
    }

    pub fn get_web_page_dom_html(
        &self,
        web_page_id: WebPageId,
        request_id: u64,
    ) -> Result<(), Error> {
        self.send(BrowserCommand::GetWebPageDomHtml {
            web_page_id,
            request_id,
        })
    }

    /// Update whether the web page should receive text input focus.
    pub fn set_web_page_focus(&self, web_page_id: WebPageId, focused: bool) -> Result<(), Error> {
        self.send(BrowserCommand::SetWebPageFocus {
            web_page_id,
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
        web_page_id: WebPageId,
        event: KeyEvent,
        commands: Vec<String>,
    ) -> Result<(), Error> {
        self.send(BrowserCommand::SendKeyEvent {
            web_page_id,
            event,
            commands,
        })
    }

    /// Send a mouse input event to the web page.
    pub fn send_mouse_event(&self, web_page_id: WebPageId, event: MouseEvent) -> Result<(), Error> {
        self.send(BrowserCommand::SendMouseEvent { web_page_id, event })
    }

    /// Send a mouse wheel event to the web page.
    pub fn send_mouse_wheel_event(
        &self,
        web_page_id: WebPageId,
        event: MouseWheelEvent,
    ) -> Result<(), Error> {
        self.send(BrowserCommand::SendMouseWheelEvent { web_page_id, event })
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
    pub fn send_drag_cancel(&self, session_id: u64, web_page_id: WebPageId) -> Result<(), Error> {
        self.send(BrowserCommand::SendDragCancel {
            session_id,
            web_page_id,
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
        web_page_id: WebPageId,
        behavior: ConfirmCompositionBehavior,
    ) -> Result<(), Error> {
        self.send(BrowserCommand::FinishComposingText {
            web_page_id,
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
        web_page_id: WebPageId,
        request_id: u64,
        proceed: bool,
    ) -> Result<(), Error> {
        self.send(BrowserCommand::ConfirmBeforeUnload {
            web_page_id,
            request_id,
            proceed,
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
