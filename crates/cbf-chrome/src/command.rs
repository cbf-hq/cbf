use cbf::command::BrowserCommand;

use crate::data::{
    browsing_context_open::ChromeBrowsingContextOpenResponse,
    drag::{ChromeDragDrop, ChromeDragUpdate},
    extension::ChromeAuxiliaryWindowResponse,
    ids::TabId,
    ime::{ChromeConfirmCompositionBehavior, ChromeImeCommitText, ChromeImeComposition},
    input::{ChromeKeyEvent, ChromeMouseWheelEvent},
    mouse::ChromeMouseEvent,
    prompt_ui::{PromptUiId, PromptUiResponse},
    window_open::ChromeWindowOpenResponse,
};

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
        browsing_context_id: TabId,
        request_id: u64,
        proceed: bool,
    },
    ConfirmPermission {
        browsing_context_id: TabId,
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
        browsing_context_id: TabId,
    },
    SetWebContentsSize {
        browsing_context_id: TabId,
        width: u32,
        height: u32,
    },
    Navigate {
        browsing_context_id: TabId,
        url: String,
    },
    GoBack {
        browsing_context_id: TabId,
    },
    GoForward {
        browsing_context_id: TabId,
    },
    Reload {
        browsing_context_id: TabId,
        ignore_cache: bool,
    },
    PrintPreview {
        browsing_context_id: TabId,
    },
    OpenDevTools {
        browsing_context_id: TabId,
    },
    InspectElement {
        browsing_context_id: TabId,
        x: i32,
        y: i32,
    },
    GetWebContentsDomHtml {
        browsing_context_id: TabId,
        request_id: u64,
    },
    SetWebContentsFocus {
        browsing_context_id: TabId,
        focused: bool,
    },
    SendKeyEvent {
        browsing_context_id: TabId,
        event: ChromeKeyEvent,
        commands: Vec<String>,
    },
    SendMouseEvent {
        browsing_context_id: TabId,
        event: ChromeMouseEvent,
    },
    SendMouseWheelEvent {
        browsing_context_id: TabId,
        event: ChromeMouseWheelEvent,
    },
    SendDragUpdate {
        update: ChromeDragUpdate,
    },
    SendDragDrop {
        drop: ChromeDragDrop,
    },
    SendDragCancel {
        session_id: u64,
        browsing_context_id: TabId,
    },
    SetImeComposition {
        composition: ChromeImeComposition,
    },
    CommitImeText {
        commit: ChromeImeCommitText,
    },
    FinishComposingText {
        browsing_context_id: TabId,
        behavior: ChromeConfirmCompositionBehavior,
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
    OpenDefaultPromptUi {
        browsing_context_id: TabId,
        request_id: u64,
    },
    RespondPromptUi {
        browsing_context_id: TabId,
        request_id: u64,
        response: PromptUiResponse,
    },
    ClosePromptUi {
        browsing_context_id: TabId,
        prompt_ui_id: PromptUiId,
    },
    RespondTabOpen {
        request_id: u64,
        response: ChromeBrowsingContextOpenResponse,
    },
    RespondWindowOpen {
        request_id: u64,
        response: ChromeWindowOpenResponse,
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
                browsing_context_id: browsing_context_id.into(),
                request_id,
                proceed,
            },
            BrowserCommand::ConfirmPermission {
                browsing_context_id,
                request_id,
                allow,
            } => Self::RespondPromptUi {
                browsing_context_id: browsing_context_id.into(),
                request_id,
                response: PromptUiResponse::PermissionPrompt { allow },
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
                browsing_context_id: browsing_context_id.into(),
            },
            BrowserCommand::ResizeBrowsingContext {
                browsing_context_id,
                width,
                height,
            } => Self::SetWebContentsSize {
                browsing_context_id: browsing_context_id.into(),
                width,
                height,
            },
            BrowserCommand::Navigate {
                browsing_context_id,
                url,
            } => Self::Navigate {
                browsing_context_id: browsing_context_id.into(),
                url,
            },
            BrowserCommand::GoBack {
                browsing_context_id,
            } => Self::GoBack {
                browsing_context_id: browsing_context_id.into(),
            },
            BrowserCommand::GoForward {
                browsing_context_id,
            } => Self::GoForward {
                browsing_context_id: browsing_context_id.into(),
            },
            BrowserCommand::Reload {
                browsing_context_id,
                ignore_cache,
            } => Self::Reload {
                browsing_context_id: browsing_context_id.into(),
                ignore_cache,
            },
            BrowserCommand::PrintPreview {
                browsing_context_id,
            } => Self::PrintPreview {
                browsing_context_id: browsing_context_id.into(),
            },
            BrowserCommand::GetBrowsingContextDomHtml {
                browsing_context_id,
                request_id,
            } => Self::GetWebContentsDomHtml {
                browsing_context_id: browsing_context_id.into(),
                request_id,
            },
            BrowserCommand::SetBrowsingContextFocus {
                browsing_context_id,
                focused,
            } => Self::SetWebContentsFocus {
                browsing_context_id: browsing_context_id.into(),
                focused,
            },
            BrowserCommand::SendKeyEvent {
                browsing_context_id,
                event,
                commands,
            } => Self::SendKeyEvent {
                browsing_context_id: browsing_context_id.into(),
                event: event.into(),
                commands,
            },
            BrowserCommand::SendMouseEvent {
                browsing_context_id,
                event,
            } => Self::SendMouseEvent {
                browsing_context_id: browsing_context_id.into(),
                event,
            },
            BrowserCommand::SendMouseWheelEvent {
                browsing_context_id,
                event,
            } => Self::SendMouseWheelEvent {
                browsing_context_id: browsing_context_id.into(),
                event: event.into(),
            },
            BrowserCommand::SendDragUpdate { update } => Self::SendDragUpdate { update },
            BrowserCommand::SendDragDrop { drop } => Self::SendDragDrop { drop },
            BrowserCommand::SendDragCancel {
                session_id,
                browsing_context_id,
            } => Self::SendDragCancel {
                session_id,
                browsing_context_id: browsing_context_id.into(),
            },
            BrowserCommand::SetComposition { composition } => {
                Self::SetImeComposition { composition }
            }
            BrowserCommand::CommitText { commit } => Self::CommitImeText { commit },
            BrowserCommand::FinishComposingText {
                browsing_context_id,
                behavior,
            } => Self::FinishComposingText {
                browsing_context_id: browsing_context_id.into(),
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
            } => Self::OpenDefaultPromptUi {
                browsing_context_id: browsing_context_id.into(),
                request_id,
            },
            BrowserCommand::RespondAuxiliaryWindow {
                browsing_context_id,
                request_id,
                response,
            } => match response {
                ChromeAuxiliaryWindowResponse::PermissionPrompt { allow } => {
                    Self::RespondPromptUi {
                        browsing_context_id: browsing_context_id.into(),
                        request_id,
                        response: PromptUiResponse::PermissionPrompt { allow },
                    }
                }
                ChromeAuxiliaryWindowResponse::ExtensionInstallPrompt { proceed } => {
                    Self::RespondPromptUi {
                        browsing_context_id: browsing_context_id.into(),
                        request_id,
                        response: PromptUiResponse::ExtensionInstallPrompt { proceed },
                    }
                }
                ChromeAuxiliaryWindowResponse::Unknown => Self::RespondPromptUi {
                    browsing_context_id: browsing_context_id.into(),
                    request_id,
                    response: PromptUiResponse::Unknown,
                },
            },
            BrowserCommand::CloseAuxiliaryWindow {
                browsing_context_id,
                window_id,
            } => Self::ClosePromptUi {
                browsing_context_id: browsing_context_id.into(),
                prompt_ui_id: PromptUiId::new(window_id.get()),
            },
            BrowserCommand::RespondBrowsingContextOpen {
                request_id,
                response,
            } => Self::RespondTabOpen {
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

#[cfg(test)]
mod tests {
    use cbf::{
        command::BrowserCommand,
        data::{extension::AuxiliaryWindowResponse, ids::BrowsingContextId},
    };

    use super::ChromeCommand;
    use crate::data::{
        ids::TabId,
        prompt_ui::{PromptUiId, PromptUiResponse},
    };

    #[test]
    fn create_close_command_converts_browsing_context_id_into_tab_id() {
        let command = BrowserCommand::RequestCloseBrowsingContext {
            browsing_context_id: BrowsingContextId::new(42),
        };

        let raw: ChromeCommand = command.into();
        assert!(matches!(
            raw,
            ChromeCommand::RequestCloseWebContents {
                browsing_context_id
            } if browsing_context_id == TabId::new(42)
        ));
    }

    #[test]
    fn confirm_permission_maps_to_prompt_ui_response() {
        let command = BrowserCommand::ConfirmPermission {
            browsing_context_id: BrowsingContextId::new(9),
            request_id: 77,
            allow: true,
        };

        let raw: ChromeCommand = command.into();
        assert!(matches!(
            raw,
            ChromeCommand::RespondPromptUi {
                browsing_context_id,
                request_id,
                response: PromptUiResponse::PermissionPrompt { allow: true },
            } if browsing_context_id == TabId::new(9) && request_id == 77
        ));
    }

    #[test]
    fn permission_auxiliary_response_maps_to_prompt_ui_response() {
        let command = BrowserCommand::RespondAuxiliaryWindow {
            browsing_context_id: BrowsingContextId::new(13),
            request_id: 81,
            response: AuxiliaryWindowResponse::PermissionPrompt { allow: false },
        };

        let raw: ChromeCommand = command.into();
        assert!(matches!(
            raw,
            ChromeCommand::RespondPromptUi {
                browsing_context_id,
                request_id,
                response: PromptUiResponse::PermissionPrompt { allow: false },
            } if browsing_context_id == TabId::new(13) && request_id == 81
        ));
    }

    #[test]
    fn extension_auxiliary_response_maps_to_prompt_ui_response() {
        let command = BrowserCommand::RespondAuxiliaryWindow {
            browsing_context_id: BrowsingContextId::new(14),
            request_id: 82,
            response: AuxiliaryWindowResponse::ExtensionInstallPrompt { proceed: true },
        };

        let raw: ChromeCommand = command.into();
        assert!(matches!(
            raw,
            ChromeCommand::RespondPromptUi {
                browsing_context_id,
                request_id,
                response: PromptUiResponse::ExtensionInstallPrompt { proceed: true },
            } if browsing_context_id == TabId::new(14) && request_id == 82
        ));
    }

    #[test]
    fn close_auxiliary_window_maps_to_prompt_ui_close() {
        let command = BrowserCommand::CloseAuxiliaryWindow {
            browsing_context_id: BrowsingContextId::new(15),
            window_id: cbf::data::extension::AuxiliaryWindowId::new(33),
        };

        let raw: ChromeCommand = command.into();
        assert!(matches!(
            raw,
            ChromeCommand::ClosePromptUi {
                browsing_context_id,
                prompt_ui_id,
            } if browsing_context_id == TabId::new(15) && prompt_ui_id == PromptUiId::new(33)
        ));
    }
}
