use cbf::data::dialog::DialogResponse;
use cbf::{
    command::{BrowserCommand, BrowserOperation},
    data::edit::EditAction,
};

use crate::data::{
    background::ChromeBackgroundPolicy,
    browsing_context_open::ChromeBrowsingContextOpenResponse,
    download::ChromeDownloadId,
    drag::{ChromeDragDrop, ChromeDragUpdate},
    extension::ChromeAuxiliaryWindowResponse,
    ids::{PopupId, TabId},
    ime::{
        ChromeConfirmCompositionBehavior, ChromeImeCommitText, ChromeImeComposition,
        ChromeTransientImeCommitText, ChromeTransientImeComposition,
    },
    input::{ChromeKeyEvent, ChromeMouseWheelEvent},
    mouse::ChromeMouseEvent,
    prompt_ui::{PromptUiId, PromptUiResponse},
    visibility::ChromeTabVisibility,
    window_open::ChromeWindowOpenResponse,
};

/// Chromium-specific transport command vocabulary.
///
/// Transient browsing context operations are currently transported through
/// Chromium's extension popup plumbing. The public `cbf` API remains generic,
/// and this layer performs the boundary translation into the current Chrome
/// implementation model.
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
    RespondJavaScriptDialog {
        browsing_context_id: TabId,
        request_id: u64,
        response: DialogResponse,
    },
    RespondExtensionPopupJavaScriptDialog {
        popup_id: PopupId,
        request_id: u64,
        response: DialogResponse,
    },
    ConfirmPermission {
        browsing_context_id: TabId,
        request_id: u64,
        allow: bool,
    },
    CreateTab {
        request_id: u64,
        initial_url: Option<String>,
        profile_id: String,
    },
    ListProfiles,
    RequestCloseTab {
        browsing_context_id: TabId,
    },
    SetTabSize {
        browsing_context_id: TabId,
        width: u32,
        height: u32,
    },
    SetTabBackgroundPolicy {
        browsing_context_id: TabId,
        policy: ChromeBackgroundPolicy,
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
    GetTabDomHtml {
        browsing_context_id: TabId,
        request_id: u64,
    },
    SetTabFocus {
        browsing_context_id: TabId,
        focused: bool,
    },
    SetTabVisibility {
        browsing_context_id: TabId,
        visibility: ChromeTabVisibility,
    },
    SendKeyEvent {
        browsing_context_id: TabId,
        event: ChromeKeyEvent,
        commands: Vec<String>,
    },
    ExecuteEditAction {
        browsing_context_id: TabId,
        action: EditAction,
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
    AcceptChoiceMenuSelection {
        request_id: u64,
        indices: Vec<i32>,
    },
    DismissChoiceMenu {
        request_id: u64,
    },
    DismissContextMenu {
        menu_id: u64,
    },
    PauseDownload {
        download_id: ChromeDownloadId,
    },
    ResumeDownload {
        download_id: ChromeDownloadId,
    },
    CancelDownload {
        download_id: ChromeDownloadId,
    },
    ListExtensions {
        profile_id: String,
    },
    ActivateExtensionAction {
        browsing_context_id: TabId,
        extension_id: String,
    },
    CloseExtensionPopup {
        popup_id: PopupId,
    },
    SetExtensionPopupSize {
        popup_id: PopupId,
        width: u32,
        height: u32,
    },
    SetExtensionPopupBackgroundPolicy {
        popup_id: PopupId,
        policy: ChromeBackgroundPolicy,
    },
    SetExtensionPopupFocus {
        popup_id: PopupId,
        focused: bool,
    },
    SendExtensionPopupKeyEvent {
        popup_id: PopupId,
        event: ChromeKeyEvent,
        commands: Vec<String>,
    },
    ExecuteExtensionPopupEditAction {
        popup_id: PopupId,
        action: EditAction,
    },
    SendExtensionPopupMouseEvent {
        popup_id: PopupId,
        event: ChromeMouseEvent,
    },
    SendExtensionPopupMouseWheelEvent {
        popup_id: PopupId,
        event: ChromeMouseWheelEvent,
    },
    SetExtensionPopupComposition {
        composition: ChromeTransientImeComposition,
    },
    CommitExtensionPopupText {
        commit: ChromeTransientImeCommitText,
    },
    FinishExtensionPopupComposingText {
        popup_id: PopupId,
        behavior: ChromeConfirmCompositionBehavior,
    },
    OpenDefaultPromptUi {
        profile_id: String,
        request_id: u64,
    },
    RespondPromptUi {
        profile_id: String,
        request_id: u64,
        response: PromptUiResponse,
    },
    ClosePromptUi {
        profile_id: String,
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
    UnsupportedGenericCommand {
        operation: BrowserOperation,
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
            BrowserCommand::RespondJavaScriptDialog {
                browsing_context_id,
                request_id,
                response,
            } => Self::RespondJavaScriptDialog {
                browsing_context_id: browsing_context_id.into(),
                request_id,
                response,
            },
            BrowserCommand::RespondJavaScriptDialogInTransientBrowsingContext {
                transient_browsing_context_id,
                request_id,
                response,
            } => Self::RespondExtensionPopupJavaScriptDialog {
                popup_id: transient_browsing_context_id.into(),
                request_id,
                response,
            },
            BrowserCommand::ConfirmPermission {
                browsing_context_id,
                request_id,
                allow,
            } => Self::ConfirmPermission {
                browsing_context_id: browsing_context_id.into(),
                request_id,
                allow,
            },
            BrowserCommand::CreateBrowsingContext {
                request_id,
                initial_url,
                profile_id,
            } => Self::CreateTab {
                request_id,
                initial_url,
                profile_id,
            },
            BrowserCommand::ListProfiles => Self::ListProfiles,
            BrowserCommand::RequestCloseBrowsingContext {
                browsing_context_id,
            } => Self::RequestCloseTab {
                browsing_context_id: browsing_context_id.into(),
            },
            BrowserCommand::CloseTransientBrowsingContext {
                transient_browsing_context_id,
            } => Self::CloseExtensionPopup {
                popup_id: transient_browsing_context_id.into(),
            },
            BrowserCommand::ResizeBrowsingContext {
                browsing_context_id,
                width,
                height,
            } => Self::SetTabSize {
                browsing_context_id: browsing_context_id.into(),
                width,
                height,
            },
            BrowserCommand::ResizeTransientBrowsingContext {
                transient_browsing_context_id,
                width,
                height,
            } => Self::SetExtensionPopupSize {
                popup_id: transient_browsing_context_id.into(),
                width,
                height,
            },
            BrowserCommand::SetBrowsingContextBackgroundPolicy {
                browsing_context_id,
                policy,
            } => Self::SetTabBackgroundPolicy {
                browsing_context_id: browsing_context_id.into(),
                policy: policy.into(),
            },
            BrowserCommand::SetTransientBrowsingContextBackgroundPolicy {
                transient_browsing_context_id,
                policy,
            } => Self::SetExtensionPopupBackgroundPolicy {
                popup_id: transient_browsing_context_id.into(),
                policy: policy.into(),
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
            } => Self::GetTabDomHtml {
                browsing_context_id: browsing_context_id.into(),
                request_id,
            },
            BrowserCommand::SetBrowsingContextFocus {
                browsing_context_id,
                focused,
            } => Self::SetTabFocus {
                browsing_context_id: browsing_context_id.into(),
                focused,
            },
            BrowserCommand::SetTransientBrowsingContextFocus {
                transient_browsing_context_id,
                focused,
            } => Self::SetExtensionPopupFocus {
                popup_id: transient_browsing_context_id.into(),
                focused,
            },
            BrowserCommand::SetBrowsingContextVisibility {
                browsing_context_id,
                visibility,
            } => Self::SetTabVisibility {
                browsing_context_id: browsing_context_id.into(),
                visibility: visibility.into(),
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
            BrowserCommand::ExecuteEditAction {
                browsing_context_id,
                action,
            } => Self::ExecuteEditAction {
                browsing_context_id: browsing_context_id.into(),
                action,
            },
            BrowserCommand::SendKeyEventToTransientBrowsingContext {
                transient_browsing_context_id,
                event,
                commands,
            } => Self::SendExtensionPopupKeyEvent {
                popup_id: transient_browsing_context_id.into(),
                event: event.into(),
                commands,
            },
            BrowserCommand::ExecuteEditActionInTransientBrowsingContext {
                transient_browsing_context_id,
                action,
            } => Self::ExecuteExtensionPopupEditAction {
                popup_id: transient_browsing_context_id.into(),
                action,
            },
            BrowserCommand::SendMouseEvent {
                browsing_context_id,
                event,
            } => Self::SendMouseEvent {
                browsing_context_id: browsing_context_id.into(),
                event: event.into(),
            },
            BrowserCommand::SendMouseEventToTransientBrowsingContext {
                transient_browsing_context_id,
                event,
            } => Self::SendExtensionPopupMouseEvent {
                popup_id: transient_browsing_context_id.into(),
                event: event.into(),
            },
            BrowserCommand::SendMouseWheelEvent {
                browsing_context_id,
                event,
            } => Self::SendMouseWheelEvent {
                browsing_context_id: browsing_context_id.into(),
                event: event.into(),
            },
            BrowserCommand::SendMouseWheelEventToTransientBrowsingContext {
                transient_browsing_context_id,
                event,
            } => Self::SendExtensionPopupMouseWheelEvent {
                popup_id: transient_browsing_context_id.into(),
                event: event.into(),
            },
            BrowserCommand::SendDragUpdate { update } => Self::SendDragUpdate {
                update: update.into(),
            },
            BrowserCommand::SendDragDrop { drop } => Self::SendDragDrop { drop: drop.into() },
            BrowserCommand::SendDragCancel {
                session_id,
                browsing_context_id,
            } => Self::SendDragCancel {
                session_id,
                browsing_context_id: browsing_context_id.into(),
            },
            BrowserCommand::SetComposition { composition } => Self::SetImeComposition {
                composition: composition.into(),
            },
            BrowserCommand::CommitText { commit } => Self::CommitImeText {
                commit: commit.into(),
            },
            BrowserCommand::SetTransientComposition { composition } => {
                Self::SetExtensionPopupComposition {
                    composition: composition.into(),
                }
            }
            BrowserCommand::CommitTransientText { commit } => Self::CommitExtensionPopupText {
                commit: commit.into(),
            },
            BrowserCommand::FinishComposingText {
                browsing_context_id,
                behavior,
            } => Self::FinishComposingText {
                browsing_context_id: browsing_context_id.into(),
                behavior: behavior.into(),
            },
            BrowserCommand::FinishComposingTextInTransientBrowsingContext {
                transient_browsing_context_id,
                behavior,
            } => Self::FinishExtensionPopupComposingText {
                popup_id: transient_browsing_context_id.into(),
                behavior: behavior.into(),
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
            BrowserCommand::AcceptChoiceMenuSelection {
                request_id,
                indices,
            } => Self::AcceptChoiceMenuSelection {
                request_id,
                indices,
            },
            BrowserCommand::DismissChoiceMenu { request_id } => {
                Self::DismissChoiceMenu { request_id }
            }
            BrowserCommand::DismissContextMenu { menu_id } => Self::DismissContextMenu { menu_id },
            BrowserCommand::PauseDownload { download_id } => Self::PauseDownload {
                download_id: download_id.into(),
            },
            BrowserCommand::ResumeDownload { download_id } => Self::ResumeDownload {
                download_id: download_id.into(),
            },
            BrowserCommand::CancelDownload { download_id } => Self::CancelDownload {
                download_id: download_id.into(),
            },
            BrowserCommand::ListExtensions { profile_id } => Self::ListExtensions { profile_id },
            BrowserCommand::OpenDefaultAuxiliaryWindow {
                profile_id,
                request_id,
            } => Self::OpenDefaultPromptUi {
                profile_id,
                request_id,
            },
            BrowserCommand::RespondAuxiliaryWindow {
                profile_id,
                request_id,
                response,
            } => match ChromeAuxiliaryWindowResponse::from(response) {
                ChromeAuxiliaryWindowResponse::PermissionPrompt { allow } => {
                    Self::RespondPromptUi {
                        profile_id,
                        request_id,
                        response: PromptUiResponse::PermissionPrompt { allow },
                    }
                }
                ChromeAuxiliaryWindowResponse::DownloadPrompt {
                    allow,
                    destination_path,
                } => Self::RespondPromptUi {
                    profile_id,
                    request_id,
                    response: PromptUiResponse::DownloadPrompt {
                        allow,
                        destination_path,
                    },
                },
                ChromeAuxiliaryWindowResponse::ExtensionInstallPrompt { proceed } => {
                    Self::RespondPromptUi {
                        profile_id,
                        request_id,
                        response: PromptUiResponse::ExtensionInstallPrompt { proceed },
                    }
                }
                ChromeAuxiliaryWindowResponse::ExtensionUninstallPrompt {
                    proceed,
                    report_abuse,
                } => Self::RespondPromptUi {
                    profile_id,
                    request_id,
                    response: PromptUiResponse::ExtensionUninstallPrompt {
                        proceed,
                        report_abuse,
                    },
                },
                ChromeAuxiliaryWindowResponse::Unknown => Self::RespondPromptUi {
                    profile_id,
                    request_id,
                    response: PromptUiResponse::Unknown,
                },
            },
            BrowserCommand::CloseAuxiliaryWindow {
                profile_id,
                window_id,
            } => Self::ClosePromptUi {
                profile_id,
                prompt_ui_id: PromptUiId::new(window_id.get()),
            },
            BrowserCommand::RespondBrowsingContextOpen {
                request_id,
                response,
            } => Self::RespondTabOpen {
                request_id,
                response: response.into(),
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
        data::{
            auxiliary_window::{AuxiliaryWindowId, AuxiliaryWindowResponse},
            background::BackgroundPolicy,
            edit::EditAction,
            ids::{BrowsingContextId, TransientBrowsingContextId},
            visibility::BrowsingContextVisibility,
        },
    };

    use super::ChromeCommand;
    use crate::data::{
        background::ChromeBackgroundPolicy,
        ids::{PopupId, TabId},
        prompt_ui::{PromptUiId, PromptUiResponse},
        visibility::ChromeTabVisibility,
    };

    #[test]
    fn create_close_command_converts_browsing_context_id_into_tab_id() {
        let command = BrowserCommand::RequestCloseBrowsingContext {
            browsing_context_id: BrowsingContextId::new(42),
        };

        let raw: ChromeCommand = command.into();
        assert!(matches!(
            raw,
            ChromeCommand::RequestCloseTab {
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
            ChromeCommand::ConfirmPermission {
                browsing_context_id,
                request_id,
                allow: true,
            } if browsing_context_id == TabId::new(9) && request_id == 77
        ));
    }

    #[test]
    fn permission_auxiliary_response_maps_to_prompt_ui_response() {
        let command = BrowserCommand::RespondAuxiliaryWindow {
            profile_id: "profile-a".to_string(),
            request_id: 81,
            response: AuxiliaryWindowResponse::PermissionPrompt { allow: false },
        };

        let raw: ChromeCommand = command.into();
        assert!(matches!(
            raw,
            ChromeCommand::RespondPromptUi {
                profile_id,
                request_id,
                response: PromptUiResponse::PermissionPrompt { allow: false },
            } if profile_id == "profile-a" && request_id == 81
        ));
    }

    #[test]
    fn extension_auxiliary_response_maps_to_prompt_ui_response() {
        let command = BrowserCommand::RespondAuxiliaryWindow {
            profile_id: "profile-b".to_string(),
            request_id: 82,
            response: AuxiliaryWindowResponse::ExtensionInstallPrompt { proceed: true },
        };

        let raw: ChromeCommand = command.into();
        assert!(matches!(
            raw,
            ChromeCommand::RespondPromptUi {
                profile_id,
                request_id,
                response: PromptUiResponse::ExtensionInstallPrompt { proceed: true },
            } if profile_id == "profile-b" && request_id == 82
        ));
    }

    #[test]
    fn uninstall_auxiliary_response_maps_to_prompt_ui_response() {
        let command = BrowserCommand::RespondAuxiliaryWindow {
            profile_id: "profile-b".to_string(),
            request_id: 83,
            response: AuxiliaryWindowResponse::ExtensionUninstallPrompt {
                proceed: true,
                report_abuse: true,
            },
        };

        let raw: ChromeCommand = command.into();
        assert!(matches!(
            raw,
            ChromeCommand::RespondPromptUi {
                profile_id,
                request_id,
                response: PromptUiResponse::ExtensionUninstallPrompt {
                    proceed: true,
                    report_abuse: true,
                },
            } if profile_id == "profile-b" && request_id == 83
        ));
    }

    #[test]
    fn close_auxiliary_window_maps_to_prompt_ui_close() {
        let command = BrowserCommand::CloseAuxiliaryWindow {
            profile_id: "profile-c".to_string(),
            window_id: AuxiliaryWindowId::new(33),
        };

        let raw: ChromeCommand = command.into();
        assert!(matches!(
            raw,
            ChromeCommand::ClosePromptUi {
                profile_id,
                prompt_ui_id,
            } if profile_id == "profile-c" && prompt_ui_id == PromptUiId::new(33)
        ));
    }

    #[test]
    fn transient_close_command_maps_to_extension_popup_close() {
        let command = BrowserCommand::CloseTransientBrowsingContext {
            transient_browsing_context_id: TransientBrowsingContextId::new(99),
        };

        let raw: ChromeCommand = command.into();
        assert!(matches!(
            raw,
            ChromeCommand::CloseExtensionPopup { popup_id }
                if popup_id == PopupId::new(99)
        ));
    }

    #[test]
    fn edit_action_command_maps_to_chrome_edit_action() {
        let command = BrowserCommand::ExecuteEditAction {
            browsing_context_id: BrowsingContextId::new(11),
            action: EditAction::Paste,
        };

        let raw: ChromeCommand = command.into();
        assert!(matches!(
            raw,
            ChromeCommand::ExecuteEditAction {
                browsing_context_id,
                action: EditAction::Paste,
            } if browsing_context_id == TabId::new(11)
        ));
    }

    #[test]
    fn transient_edit_action_command_maps_to_extension_popup_edit_action() {
        let command = BrowserCommand::ExecuteEditActionInTransientBrowsingContext {
            transient_browsing_context_id: TransientBrowsingContextId::new(12),
            action: EditAction::SelectAll,
        };

        let raw: ChromeCommand = command.into();
        assert!(matches!(
            raw,
            ChromeCommand::ExecuteExtensionPopupEditAction {
                popup_id,
                action: EditAction::SelectAll,
            } if popup_id == PopupId::new(12)
        ));
    }

    #[test]
    fn set_visibility_command_converts_browsing_context_id_into_tab_id() {
        let command = BrowserCommand::SetBrowsingContextVisibility {
            browsing_context_id: BrowsingContextId::new(24),
            visibility: BrowsingContextVisibility::Hidden,
        };

        let raw: ChromeCommand = command.into();
        assert!(matches!(
            raw,
            ChromeCommand::SetTabVisibility {
                browsing_context_id,
                visibility: ChromeTabVisibility::Hidden,
            } if browsing_context_id == TabId::new(24)
        ));
    }

    #[test]
    fn set_background_policy_command_converts_browsing_context_id_into_tab_id() {
        let command = BrowserCommand::SetBrowsingContextBackgroundPolicy {
            browsing_context_id: BrowsingContextId::new(25),
            policy: BackgroundPolicy::Transparent,
        };

        let raw: ChromeCommand = command.into();
        assert!(matches!(
            raw,
            ChromeCommand::SetTabBackgroundPolicy {
                browsing_context_id,
                policy: ChromeBackgroundPolicy::Transparent,
            } if browsing_context_id == TabId::new(25)
        ));
    }

    #[test]
    fn transient_background_policy_command_maps_to_extension_popup_policy() {
        let command = BrowserCommand::SetTransientBrowsingContextBackgroundPolicy {
            transient_browsing_context_id: TransientBrowsingContextId::new(26),
            policy: BackgroundPolicy::Opaque,
        };

        let raw: ChromeCommand = command.into();
        assert!(matches!(
            raw,
            ChromeCommand::SetExtensionPopupBackgroundPolicy {
                popup_id,
                policy: ChromeBackgroundPolicy::Opaque,
            } if popup_id == PopupId::new(26)
        ));
    }
}
