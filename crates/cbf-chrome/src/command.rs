use cbf::{
    command::BrowserCommand,
    data::{
        browsing_context_open::BrowsingContextOpenResponse,
        drag::{DragDrop, DragUpdate},
        extension::{AuxiliaryWindowId, AuxiliaryWindowResponse},
        ids::BrowsingContextId,
        ime::{ConfirmCompositionBehavior, ImeCommitText, ImeComposition},
        mouse::MouseEvent,
        window_open::WindowOpenResponse,
    },
};

use crate::data::input::{ChromeKeyEvent, ChromeMouseWheelEvent};

/// Chromium-specific transport command vocabulary.
#[derive(Debug, Clone, PartialEq)]
pub enum ChromeCommand {
    RequestShutdown {
        request_id: u64,
    },
    ConfirmShutdown {
        request_id: u64,
        proceed: bool,
    },
    ForceShutdown,
    ConfirmBeforeUnload {
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        proceed: bool,
    },
    ConfirmPermission {
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        allow: bool,
    },
    CreateWebContents {
        request_id: u64,
        initial_url: Option<String>,
        profile_id: Option<String>,
    },
    ListProfiles,
    RequestCloseWebContents {
        browsing_context_id: BrowsingContextId,
    },
    SetWebContentsSize {
        browsing_context_id: BrowsingContextId,
        width: u32,
        height: u32,
    },
    Navigate {
        browsing_context_id: BrowsingContextId,
        url: String,
    },
    GoBack {
        browsing_context_id: BrowsingContextId,
    },
    GoForward {
        browsing_context_id: BrowsingContextId,
    },
    Reload {
        browsing_context_id: BrowsingContextId,
        ignore_cache: bool,
    },
    PrintPreview {
        browsing_context_id: BrowsingContextId,
    },
    OpenDevTools {
        browsing_context_id: BrowsingContextId,
    },
    InspectElement {
        browsing_context_id: BrowsingContextId,
        x: i32,
        y: i32,
    },
    GetWebContentsDomHtml {
        browsing_context_id: BrowsingContextId,
        request_id: u64,
    },
    SetWebContentsFocus {
        browsing_context_id: BrowsingContextId,
        focused: bool,
    },
    SendKeyEvent {
        browsing_context_id: BrowsingContextId,
        event: ChromeKeyEvent,
        commands: Vec<String>,
    },
    SendMouseEvent {
        browsing_context_id: BrowsingContextId,
        event: MouseEvent,
    },
    SendMouseWheelEvent {
        browsing_context_id: BrowsingContextId,
        event: ChromeMouseWheelEvent,
    },
    SendDragUpdate {
        update: DragUpdate,
    },
    SendDragDrop {
        drop: DragDrop,
    },
    SendDragCancel {
        session_id: u64,
        browsing_context_id: BrowsingContextId,
    },
    SetImeComposition {
        composition: ImeComposition,
    },
    CommitImeText {
        commit: ImeCommitText,
    },
    FinishComposingText {
        browsing_context_id: BrowsingContextId,
        behavior: ConfirmCompositionBehavior,
    },
    ExecuteContextMenuCommand {
        menu_id: u64,
        command_id: i32,
        event_flags: i32,
    },
    DismissContextMenu {
        menu_id: u64,
    },
    ListExtensions {
        profile_id: Option<String>,
    },
    OpenDefaultAuxiliaryWindow {
        browsing_context_id: BrowsingContextId,
        request_id: u64,
    },
    RespondAuxiliaryWindow {
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        response: AuxiliaryWindowResponse,
    },
    CloseAuxiliaryWindow {
        browsing_context_id: BrowsingContextId,
        window_id: AuxiliaryWindowId,
    },
    RespondBrowsingContextOpen {
        request_id: u64,
        response: BrowsingContextOpenResponse,
    },
    RespondWindowOpen {
        request_id: u64,
        response: WindowOpenResponse,
    },
}

impl From<BrowserCommand> for ChromeCommand {
    fn from(value: BrowserCommand) -> Self {
        match value {
            BrowserCommand::Shutdown { request_id } => Self::RequestShutdown { request_id },
            BrowserCommand::ConfirmShutdown {
                request_id,
                proceed,
            } => Self::ConfirmShutdown {
                request_id,
                proceed,
            },
            BrowserCommand::ForceShutdown => Self::ForceShutdown,
            BrowserCommand::ConfirmBeforeUnload {
                browsing_context_id,
                request_id,
                proceed,
            } => Self::ConfirmBeforeUnload {
                browsing_context_id,
                request_id,
                proceed,
            },
            BrowserCommand::ConfirmPermission {
                browsing_context_id,
                request_id,
                allow,
            } => Self::ConfirmPermission {
                browsing_context_id,
                request_id,
                allow,
            },
            BrowserCommand::CreateBrowsingContext {
                request_id,
                initial_url,
                profile_id,
            } => Self::CreateWebContents {
                request_id,
                initial_url,
                profile_id,
            },
            BrowserCommand::ListProfiles => Self::ListProfiles,
            BrowserCommand::RequestCloseBrowsingContext {
                browsing_context_id,
            } => Self::RequestCloseWebContents {
                browsing_context_id,
            },
            BrowserCommand::ResizeBrowsingContext {
                browsing_context_id,
                width,
                height,
            } => Self::SetWebContentsSize {
                browsing_context_id,
                width,
                height,
            },
            BrowserCommand::Navigate {
                browsing_context_id,
                url,
            } => Self::Navigate {
                browsing_context_id,
                url,
            },
            BrowserCommand::GoBack {
                browsing_context_id,
            } => Self::GoBack {
                browsing_context_id,
            },
            BrowserCommand::GoForward {
                browsing_context_id,
            } => Self::GoForward {
                browsing_context_id,
            },
            BrowserCommand::Reload {
                browsing_context_id,
                ignore_cache,
            } => Self::Reload {
                browsing_context_id,
                ignore_cache,
            },
            BrowserCommand::PrintPreview {
                browsing_context_id,
            } => Self::PrintPreview {
                browsing_context_id,
            },
            BrowserCommand::GetBrowsingContextDomHtml {
                browsing_context_id,
                request_id,
            } => Self::GetWebContentsDomHtml {
                browsing_context_id,
                request_id,
            },
            BrowserCommand::SetBrowsingContextFocus {
                browsing_context_id,
                focused,
            } => Self::SetWebContentsFocus {
                browsing_context_id,
                focused,
            },
            BrowserCommand::SendKeyEvent {
                browsing_context_id,
                event,
                commands,
            } => Self::SendKeyEvent {
                browsing_context_id,
                event: event.into(),
                commands,
            },
            BrowserCommand::SendMouseEvent {
                browsing_context_id,
                event,
            } => Self::SendMouseEvent {
                browsing_context_id,
                event,
            },
            BrowserCommand::SendMouseWheelEvent {
                browsing_context_id,
                event,
            } => Self::SendMouseWheelEvent {
                browsing_context_id,
                event: event.into(),
            },
            BrowserCommand::SendDragUpdate { update } => Self::SendDragUpdate { update },
            BrowserCommand::SendDragDrop { drop } => Self::SendDragDrop { drop },
            BrowserCommand::SendDragCancel {
                session_id,
                browsing_context_id,
            } => Self::SendDragCancel {
                session_id,
                browsing_context_id,
            },
            BrowserCommand::SetComposition { composition } => {
                Self::SetImeComposition { composition }
            }
            BrowserCommand::CommitText { commit } => Self::CommitImeText { commit },
            BrowserCommand::FinishComposingText {
                browsing_context_id,
                behavior,
            } => Self::FinishComposingText {
                browsing_context_id,
                behavior,
            },
            BrowserCommand::ExecuteContextMenuCommand {
                menu_id,
                command_id,
                event_flags,
            } => Self::ExecuteContextMenuCommand {
                menu_id,
                command_id,
                event_flags,
            },
            BrowserCommand::DismissContextMenu { menu_id } => Self::DismissContextMenu { menu_id },
            BrowserCommand::ListExtensions { profile_id } => Self::ListExtensions { profile_id },
            BrowserCommand::OpenDefaultAuxiliaryWindow {
                browsing_context_id,
                request_id,
            } => Self::OpenDefaultAuxiliaryWindow {
                browsing_context_id,
                request_id,
            },
            BrowserCommand::RespondAuxiliaryWindow {
                browsing_context_id,
                request_id,
                response,
            } => Self::RespondAuxiliaryWindow {
                browsing_context_id,
                request_id,
                response,
            },
            BrowserCommand::CloseAuxiliaryWindow {
                browsing_context_id,
                window_id,
            } => Self::CloseAuxiliaryWindow {
                browsing_context_id,
                window_id,
            },
            BrowserCommand::RespondBrowsingContextOpen {
                request_id,
                response,
            } => Self::RespondBrowsingContextOpen {
                request_id,
                response,
            },
            BrowserCommand::RespondWindowOpen {
                request_id,
                response,
            } => Self::RespondWindowOpen {
                request_id,
                response,
            },
        }
    }
}
