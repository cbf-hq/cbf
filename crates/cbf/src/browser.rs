use async_channel::{Receiver, Sender, TrySendError};

use crate::{
    command::BrowserCommand,
    data::{
        drag::{DragDrop, DragUpdate},
        ids::BrowsingContextId,
        ime::{ConfirmCompositionBehavior, ImeCommitText, ImeComposition},
        key::KeyEvent,
        mouse::{MouseEvent, MouseWheelEvent},
    },
    delegate::BackendDelegate,
    error::Error,
    event::BrowserEvent,
};

pub use crate::browser_session::BrowserSession;

/// A backend implementation that can drive a browser process.
///
/// The `cbf` layer stays browser-generic. Backend-specific command/event
/// contracts are represented as raw associated types and converted through
/// `to_raw_command` / `to_generic_event`.
pub trait Backend: Send + 'static {
    type RawCommand: Send + 'static;
    type RawEvent: Send + 'static;
    type RawDelegate: Send + 'static;

    /// Converts a browser-generic command into backend-native raw command.
    fn to_raw_command(command: BrowserCommand) -> Self::RawCommand;

    /// Converts a backend-native raw event into browser-generic event if possible.
    fn to_generic_event(raw: &Self::RawEvent) -> Option<BrowserEvent>;

    /// Establish a command/event channel pair for this backend.
    fn connect<D: BackendDelegate>(
        self,
        delegate: D,
        raw_delegate: Option<Self::RawDelegate>,
    ) -> Result<(CommandSender<Self>, EventStream<Self>), Error>
    where
        Self: Sized;
}

/// Channel sender used to push backend raw commands.
pub struct CommandSender<B: Backend> {
    tx: Sender<B::RawCommand>,
}

impl<B: Backend> Clone for CommandSender<B> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<B: Backend> std::fmt::Debug for CommandSender<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandSender").finish_non_exhaustive()
    }
}

impl<B: Backend> CommandSender<B> {
    #[doc(hidden)]
    pub fn from_raw_sender(tx: Sender<B::RawCommand>) -> Self {
        Self { tx }
    }

    /// Send a browser-generic command to the backend.
    pub fn send(&self, command: BrowserCommand) -> Result<(), Error> {
        let raw = B::to_raw_command(command);
        self.try_send_raw(raw)
    }

    fn try_send_raw(&self, raw: B::RawCommand) -> Result<(), Error> {
        match self.tx.try_send(raw) {
            Ok(()) => Ok(()),
            Err(TrySendError::Closed(_)) => Err(Error::Disconnected),
            Err(TrySendError::Full(_)) => Err(Error::QueueFull),
        }
    }
}

/// Explicit raw command escape hatch.
pub trait RawCommandSenderExt<B: Backend> {
    fn send_raw(&self, raw: B::RawCommand) -> Result<(), Error>;
}

impl<B: Backend> RawCommandSenderExt<B> for CommandSender<B> {
    fn send_raw(&self, raw: B::RawCommand) -> Result<(), Error> {
        self.try_send_raw(raw)
    }
}

/// Event payload that carries both raw and browser-generic interpretation.
pub struct OpaqueEvent<B: Backend> {
    raw: B::RawEvent,
    generic: Option<BrowserEvent>,
}

impl<B: Backend> OpaqueEvent<B> {
    fn new(raw: B::RawEvent) -> Self {
        let generic = B::to_generic_event(&raw);
        Self { raw, generic }
    }

    pub fn as_generic(&self) -> Option<&BrowserEvent> {
        self.generic.as_ref()
    }
}

impl<B: Backend> std::fmt::Debug for OpaqueEvent<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpaqueEvent").finish_non_exhaustive()
    }
}

/// Explicit raw event escape hatch.
pub trait RawOpaqueEventExt<B: Backend> {
    fn as_raw(&self) -> &B::RawEvent;
}

impl<B: Backend> RawOpaqueEventExt<B> for OpaqueEvent<B> {
    fn as_raw(&self) -> &B::RawEvent {
        &self.raw
    }
}

/// Stream of backend raw events.
pub struct EventStream<B: Backend> {
    rx: Receiver<B::RawEvent>,
}

impl<B: Backend> Clone for EventStream<B> {
    fn clone(&self) -> Self {
        Self {
            rx: self.rx.clone(),
        }
    }
}

impl<B: Backend> std::fmt::Debug for EventStream<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventStream").finish_non_exhaustive()
    }
}

impl<B: Backend> EventStream<B> {
    #[doc(hidden)]
    pub fn from_raw_receiver(rx: Receiver<B::RawEvent>) -> Self {
        Self { rx }
    }

    pub async fn recv(&self) -> Result<OpaqueEvent<B>, Error> {
        let raw = self.rx.recv().await.map_err(|_| Error::Disconnected)?;
        Ok(OpaqueEvent::new(raw))
    }

    pub fn recv_blocking(&self) -> Result<OpaqueEvent<B>, Error> {
        let raw = self.rx.recv_blocking().map_err(|_| Error::Disconnected)?;
        Ok(OpaqueEvent::new(raw))
    }
}

/// A clonable handle used to send commands to the browser backend.
pub struct BrowserHandle<B: Backend> {
    command_tx: CommandSender<B>,
}

impl<B: Backend> Clone for BrowserHandle<B> {
    fn clone(&self) -> Self {
        Self {
            command_tx: self.command_tx.clone(),
        }
    }
}

impl<B: Backend> std::fmt::Debug for BrowserHandle<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserHandle").finish_non_exhaustive()
    }
}

impl<B: Backend> BrowserHandle<B> {
    pub(crate) fn new(command_tx: CommandSender<B>) -> Self {
        Self { command_tx }
    }

    /// Send a browser-generic command to the backend.
    pub fn send(&self, command: BrowserCommand) -> Result<(), Error> {
        self.command_tx.send(command)
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
    pub fn request_close_browsing_context(
        &self,
        browsing_context_id: BrowsingContextId,
    ) -> Result<(), Error> {
        self.send(BrowserCommand::RequestCloseBrowsingContext {
            browsing_context_id,
        })
    }

    /// Navigate the web page to the provided URL.
    pub fn navigate(
        &self,
        browsing_context_id: BrowsingContextId,
        url: String,
    ) -> Result<(), Error> {
        self.send(BrowserCommand::Navigate {
            browsing_context_id,
            url,
        })
    }

    /// Navigate back in history for the given web page.
    pub fn go_back(&self, browsing_context_id: BrowsingContextId) -> Result<(), Error> {
        self.send(BrowserCommand::GoBack {
            browsing_context_id,
        })
    }

    /// Navigate forward in history for the given web page.
    pub fn go_forward(&self, browsing_context_id: BrowsingContextId) -> Result<(), Error> {
        self.send(BrowserCommand::GoForward {
            browsing_context_id,
        })
    }

    /// Reload the current page, optionally bypassing caches.
    pub fn reload(
        &self,
        browsing_context_id: BrowsingContextId,
        ignore_cache: bool,
    ) -> Result<(), Error> {
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
    pub fn set_browsing_context_focus(
        &self,
        browsing_context_id: BrowsingContextId,
        focused: bool,
    ) -> Result<(), Error> {
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
    pub fn send_mouse_event(
        &self,
        browsing_context_id: BrowsingContextId,
        event: MouseEvent,
    ) -> Result<(), Error> {
        self.send(BrowserCommand::SendMouseEvent {
            browsing_context_id,
            event,
        })
    }

    /// Send a mouse wheel event to the web page.
    pub fn send_mouse_wheel_event(
        &self,
        browsing_context_id: BrowsingContextId,
        event: MouseWheelEvent,
    ) -> Result<(), Error> {
        self.send(BrowserCommand::SendMouseWheelEvent {
            browsing_context_id,
            event,
        })
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
    pub fn send_drag_cancel(
        &self,
        session_id: u64,
        browsing_context_id: BrowsingContextId,
    ) -> Result<(), Error> {
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

impl<B: Backend> RawCommandSenderExt<B> for BrowserHandle<B> {
    fn send_raw(&self, raw: B::RawCommand) -> Result<(), Error> {
        self.command_tx.send_raw(raw)
    }
}

/// Connect to a backend and obtain a command session and an event stream.
///
/// This function remains as a compatibility wrapper.
#[deprecated(note = "Use BrowserSession::connect instead.")]
pub fn connect<B: Backend, D: BackendDelegate>(
    backend: B,
    delegate: D,
    raw_delegate: Option<B::RawDelegate>,
) -> Result<(BrowserSession<B>, EventStream<B>), Error> {
    BrowserSession::connect(backend, delegate, raw_delegate)
}
