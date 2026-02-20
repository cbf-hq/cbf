use cbf::{
    command::BrowserCommand,
    data::{
        drag::{DragDrop, DragUpdate},
        ids::BrowsingContextId,
        ime::{ConfirmCompositionBehavior, ImeCommitText, ImeComposition},
        key::KeyEvent,
        mouse::MouseEvent,
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
}

impl ChromeCommand {
    /// Converts a Chromium raw command back to browser-generic command when possible.
    pub fn to_browser_command(&self) -> Option<BrowserCommand> {
        match self {
            Self::RequestShutdown { request_id } => Some(BrowserCommand::Shutdown {
                request_id: *request_id,
            }),
            Self::ConfirmShutdown {
                request_id,
                proceed,
            } => Some(BrowserCommand::ConfirmShutdown {
                request_id: *request_id,
                proceed: *proceed,
            }),
            Self::ForceShutdown => Some(BrowserCommand::ForceShutdown),
            Self::ConfirmBeforeUnload {
                browsing_context_id,
                request_id,
                proceed,
            } => Some(BrowserCommand::ConfirmBeforeUnload {
                browsing_context_id: *browsing_context_id,
                request_id: *request_id,
                proceed: *proceed,
            }),
            Self::ConfirmPermission {
                browsing_context_id,
                request_id,
                allow,
            } => Some(BrowserCommand::ConfirmPermission {
                browsing_context_id: *browsing_context_id,
                request_id: *request_id,
                allow: *allow,
            }),
            Self::CreateWebContents {
                request_id,
                initial_url,
                profile_id,
            } => Some(BrowserCommand::CreateBrowsingContext {
                request_id: *request_id,
                initial_url: initial_url.clone(),
                profile_id: profile_id.clone(),
            }),
            Self::ListProfiles => Some(BrowserCommand::ListProfiles),
            Self::RequestCloseWebContents {
                browsing_context_id,
            } => Some(BrowserCommand::RequestCloseBrowsingContext {
                browsing_context_id: *browsing_context_id,
            }),
            Self::SetWebContentsSize {
                browsing_context_id,
                width,
                height,
            } => Some(BrowserCommand::ResizeBrowsingContext {
                browsing_context_id: *browsing_context_id,
                width: *width,
                height: *height,
            }),
            Self::Navigate {
                browsing_context_id,
                url,
            } => Some(BrowserCommand::Navigate {
                browsing_context_id: *browsing_context_id,
                url: url.clone(),
            }),
            Self::GoBack {
                browsing_context_id,
            } => Some(BrowserCommand::GoBack {
                browsing_context_id: *browsing_context_id,
            }),
            Self::GoForward {
                browsing_context_id,
            } => Some(BrowserCommand::GoForward {
                browsing_context_id: *browsing_context_id,
            }),
            Self::Reload {
                browsing_context_id,
                ignore_cache,
            } => Some(BrowserCommand::Reload {
                browsing_context_id: *browsing_context_id,
                ignore_cache: *ignore_cache,
            }),
            Self::GetWebContentsDomHtml {
                browsing_context_id,
                request_id,
            } => Some(BrowserCommand::GetBrowsingContextDomHtml {
                browsing_context_id: *browsing_context_id,
                request_id: *request_id,
            }),
            Self::SetWebContentsFocus {
                browsing_context_id,
                focused,
            } => Some(BrowserCommand::SetBrowsingContextFocus {
                browsing_context_id: *browsing_context_id,
                focused: *focused,
            }),
            Self::SendKeyEvent {
                browsing_context_id,
                event,
                commands,
            } => Some(BrowserCommand::SendKeyEvent {
                browsing_context_id: *browsing_context_id,
                event: KeyEvent::from(event.clone()),
                commands: commands.clone(),
            }),
            Self::SendMouseEvent {
                browsing_context_id,
                event,
            } => Some(BrowserCommand::SendMouseEvent {
                browsing_context_id: *browsing_context_id,
                event: event.clone(),
            }),
            Self::SendMouseWheelEvent {
                browsing_context_id,
                event,
            } => Some(BrowserCommand::SendMouseWheelEvent {
                browsing_context_id: *browsing_context_id,
                event: event.clone().into(),
            }),
            Self::SendDragUpdate { update } => Some(BrowserCommand::SendDragUpdate {
                update: update.clone(),
            }),
            Self::SendDragDrop { drop } => {
                Some(BrowserCommand::SendDragDrop { drop: drop.clone() })
            }
            Self::SendDragCancel {
                session_id,
                browsing_context_id,
            } => Some(BrowserCommand::SendDragCancel {
                session_id: *session_id,
                browsing_context_id: *browsing_context_id,
            }),
            Self::SetImeComposition { composition } => Some(BrowserCommand::SetComposition {
                composition: composition.clone(),
            }),
            Self::CommitImeText { commit } => Some(BrowserCommand::CommitText {
                commit: commit.clone(),
            }),
            Self::FinishComposingText {
                browsing_context_id,
                behavior,
            } => Some(BrowserCommand::FinishComposingText {
                browsing_context_id: *browsing_context_id,
                behavior: *behavior,
            }),
            Self::ExecuteContextMenuCommand {
                menu_id,
                command_id,
                event_flags,
            } => Some(BrowserCommand::ExecuteContextMenuCommand {
                menu_id: *menu_id,
                command_id: *command_id,
                event_flags: *event_flags,
            }),
            Self::DismissContextMenu { menu_id } => {
                Some(BrowserCommand::DismissContextMenu { menu_id: *menu_id })
            }
        }
    }
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
        }
    }
}
