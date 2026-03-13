//! Chrome-specific event types and conversion utilities.
//!
//! This module defines [`ChromeEvent`], an event type that carries
//! Chrome-specific vocabulary and serves as the primary event stream payload
//! of the Chrome backend. The underlying FFI bindings live in `cbf-chrome-sys`;
//! the types here are safe Rust abstractions on top of them.
//!
//! Two conversion functions translate Chrome-specific events into
//! browser-generic ones where a mapping exists:
//!
//! - [`to_generic_event`] — converts a [`ChromeEvent`] into a
//!   [`cbf::event::BrowserEvent`].
//! - [`map_ipc_event_to_generic`] — converts a [`crate::ffi::IpcEvent`]
//!   received over the IPC bridge into a [`cbf::event::BrowserEvent`].
//!
//! Not every Chrome-specific event has a generic counterpart; those return
//! `None` and are intended to be consumed only by Chrome-aware code.

use cbf::data::{
    dialog::DialogType,
    download::DownloadPromptResult,
    extension::{
        AuxiliaryWindowCloseReason, AuxiliaryWindowId, AuxiliaryWindowKind,
        AuxiliaryWindowResolution, ExtensionInstallPromptResult, PermissionPromptResult,
        PermissionPromptType,
    },
    ids::WindowId,
    transient_browsing_context::{
        TransientBrowsingContextCloseReason, TransientBrowsingContextKind,
    },
    window_open::{
        WindowBounds, WindowDescriptor, WindowKind, WindowOpenReason, WindowOpenRequest,
        WindowOpenResult, WindowState,
    },
};
use cbf::event::{BrowserEvent, BrowsingContextEvent, TransientBrowsingContextEvent};

use crate::data::{
    browsing_context_open::ChromeBrowsingContextOpenResult,
    download::ChromeDownloadPromptResult,
    extension::ChromeExtensionInfo,
    ids::PopupId,
    lifecycle::{ChromeBackendErrorInfo, ChromeBackendStopReason},
    profile::ChromeProfileInfo,
    prompt_ui::{
        PromptUiCloseReason, PromptUiDialogResult, PromptUiExtensionInstallResult, PromptUiId,
        PromptUiKind, PromptUiPermissionType, PromptUiResolution, PromptUiResolutionResult,
    },
    tab_open::{TabOpenHint, TabOpenResult},
};
use crate::ffi::IpcEvent;

/// Chromium-specific raw event stream payload.
#[derive(Debug, Clone)]
pub enum ChromeEvent {
    /// Raw IPC event from the bridge.
    Ipc(Box<IpcEvent>),
    /// Backend connected and ready.
    BackendReady,
    /// Backend stopped with a reason.
    BackendStopped { reason: ChromeBackendStopReason },
    /// Backend error surfaced from command/event processing.
    BackendError {
        info: ChromeBackendErrorInfo,
        terminal_hint: bool,
    },
    /// Profile list obtained through backend-side request/response path.
    ProfilesListed { profiles: Vec<ChromeProfileInfo> },
}

/// Maps Chromium raw events into browser-generic events when possible.
///
/// Some Chrome-specific events are implementation details that do not map to
/// generic browser events. See each event variant's documentation for conversion
/// behavior.
pub fn to_generic_event(event: &ChromeEvent) -> Option<BrowserEvent> {
    match event {
        ChromeEvent::Ipc(raw) => map_ipc_event_to_generic(raw),
        ChromeEvent::BackendReady => Some(BrowserEvent::BackendReady),
        ChromeEvent::BackendStopped { reason } => Some(BrowserEvent::BackendStopped {
            reason: reason.clone(),
        }),
        ChromeEvent::BackendError {
            info,
            terminal_hint,
        } => Some(BrowserEvent::BackendError {
            info: info.clone(),
            terminal_hint: *terminal_hint,
        }),
        ChromeEvent::ProfilesListed { profiles } => Some(BrowserEvent::ProfilesListed {
            profiles: profiles.iter().cloned().map(Into::into).collect(),
        }),
    }
}

/// Maps IPC events into browser-generic events when possible.
///
/// Some IPC events are Chrome-specific implementation details and do not map
/// to generic browser events. See each [`IpcEvent`] variant's documentation for
/// conversion behavior.
pub fn map_ipc_event_to_generic(event: &IpcEvent) -> Option<BrowserEvent> {
    match event {
        IpcEvent::SurfaceHandleUpdated { .. } => None,
        IpcEvent::ExtensionPopupOpened {
            profile_id,
            browsing_context_id,
            popup_id,
            title,
            ..
        } => Some(BrowserEvent::TransientBrowsingContext {
            profile_id: profile_id.clone(),
            transient_browsing_context_id: PopupId::new(*popup_id)
                .to_transient_browsing_context_id(),
            parent_browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(TransientBrowsingContextEvent::Opened {
                kind: TransientBrowsingContextKind::Popup,
                title: Some(title.clone()),
            }),
        }),
        IpcEvent::ExtensionPopupSurfaceHandleUpdated { .. } => None,
        IpcEvent::ExtensionPopupPreferredSizeChanged {
            profile_id,
            browsing_context_id,
            popup_id,
            width,
            height,
        } => Some(BrowserEvent::TransientBrowsingContext {
            profile_id: profile_id.clone(),
            transient_browsing_context_id: PopupId::new(*popup_id)
                .to_transient_browsing_context_id(),
            parent_browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(TransientBrowsingContextEvent::Resized {
                width: *width,
                height: *height,
            }),
        }),
        IpcEvent::ExtensionPopupContextMenuRequested {
            profile_id,
            browsing_context_id,
            popup_id,
            menu,
        } => Some(BrowserEvent::TransientBrowsingContext {
            profile_id: profile_id.clone(),
            transient_browsing_context_id: popup_id.to_transient_browsing_context_id(),
            parent_browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(TransientBrowsingContextEvent::ContextMenuRequested {
                menu: menu.clone().into(),
            }),
        }),
        IpcEvent::ExtensionPopupCursorChanged {
            profile_id,
            browsing_context_id,
            popup_id,
            cursor_type,
        } => Some(BrowserEvent::TransientBrowsingContext {
            profile_id: profile_id.clone(),
            transient_browsing_context_id: popup_id.to_transient_browsing_context_id(),
            parent_browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(TransientBrowsingContextEvent::CursorChanged {
                cursor_type: *cursor_type,
            }),
        }),
        IpcEvent::ExtensionPopupTitleUpdated {
            profile_id,
            browsing_context_id,
            popup_id,
            title,
        } => Some(BrowserEvent::TransientBrowsingContext {
            profile_id: profile_id.clone(),
            transient_browsing_context_id: popup_id.to_transient_browsing_context_id(),
            parent_browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(TransientBrowsingContextEvent::TitleUpdated {
                title: title.clone(),
            }),
        }),
        IpcEvent::ExtensionPopupJavaScriptDialogRequested {
            profile_id,
            browsing_context_id,
            popup_id,
            request_id,
            r#type,
            message,
            default_prompt_text,
            reason,
        } => Some(BrowserEvent::TransientBrowsingContext {
            profile_id: profile_id.clone(),
            transient_browsing_context_id: popup_id.to_transient_browsing_context_id(),
            parent_browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(TransientBrowsingContextEvent::JavaScriptDialogRequested {
                request_id: *request_id,
                message: message.clone(),
                default_prompt_text: default_prompt_text.clone(),
                r#type: *r#type,
                beforeunload_reason: if *r#type == DialogType::BeforeUnload {
                    Some((*reason).into())
                } else {
                    None
                },
            }),
        }),
        IpcEvent::ExtensionPopupCloseRequested {
            profile_id,
            browsing_context_id,
            popup_id,
        } => Some(BrowserEvent::TransientBrowsingContext {
            profile_id: profile_id.clone(),
            transient_browsing_context_id: popup_id.to_transient_browsing_context_id(),
            parent_browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(TransientBrowsingContextEvent::CloseRequested),
        }),
        IpcEvent::ExtensionPopupRenderProcessGone {
            profile_id,
            browsing_context_id,
            popup_id,
            crashed,
        } => Some(BrowserEvent::TransientBrowsingContext {
            profile_id: profile_id.clone(),
            transient_browsing_context_id: popup_id.to_transient_browsing_context_id(),
            parent_browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(TransientBrowsingContextEvent::RenderProcessGone { crashed: *crashed }),
        }),
        IpcEvent::ExtensionPopupClosed {
            profile_id,
            browsing_context_id,
            popup_id,
        } => Some(BrowserEvent::TransientBrowsingContext {
            profile_id: profile_id.clone(),
            transient_browsing_context_id: PopupId::new(*popup_id)
                .to_transient_browsing_context_id(),
            parent_browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(TransientBrowsingContextEvent::Closed {
                reason: TransientBrowsingContextCloseReason::Unknown,
            }),
        }),
        IpcEvent::TabCreated {
            profile_id,
            browsing_context_id,
            request_id,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::Created {
                request_id: *request_id,
            }),
        }),
        IpcEvent::DevToolsOpened { .. } => None,
        IpcEvent::ImeBoundsUpdated {
            profile_id,
            browsing_context_id,
            update,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::ImeBoundsUpdated {
                update: update.clone().into(),
            }),
        }),
        IpcEvent::ExtensionPopupImeBoundsUpdated {
            profile_id,
            browsing_context_id,
            popup_id,
            update,
        } => Some(BrowserEvent::TransientBrowsingContext {
            profile_id: profile_id.clone(),
            transient_browsing_context_id: popup_id.to_transient_browsing_context_id(),
            parent_browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(TransientBrowsingContextEvent::ImeBoundsUpdated {
                update: update.clone().into(),
            }),
        }),
        IpcEvent::ContextMenuRequested {
            profile_id,
            browsing_context_id,
            menu,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::ContextMenuRequested {
                menu: menu.clone().into(),
            }),
        }),
        IpcEvent::TabOpenRequested {
            profile_id,
            request_id,
            source_tab_id,
            target_url,
            open_hint,
            user_gesture,
        } => match open_hint {
            TabOpenHint::NewWindow | TabOpenHint::Popup => {
                Some(BrowserEvent::WindowOpenRequested {
                    profile_id: profile_id.clone(),
                    request: WindowOpenRequest {
                        request_id: *request_id,
                        reason: WindowOpenReason::Navigation,
                        opener_window_id: None,
                        opener_browsing_context_id: source_tab_id
                            .map(|id| id.to_browsing_context_id()),
                        target_url: Some(target_url.clone()),
                        requested_kind: if matches!(open_hint, TabOpenHint::Popup) {
                            WindowKind::Popup
                        } else {
                            WindowKind::Normal
                        },
                        user_gesture: *user_gesture,
                    },
                })
            }
            _ => Some(BrowserEvent::BrowsingContextOpenRequested {
                profile_id: profile_id.clone(),
                request_id: *request_id,
                source_browsing_context_id: source_tab_id.map(|id| id.to_browsing_context_id()),
                target_url: target_url.clone(),
                open_hint: open_hint
                    .to_browsing_context_open_hint()
                    .map(Into::into)
                    .unwrap_or_else(|| {
                        unreachable!(
                            "window-oriented tab-open hints are mapped to WindowOpenRequested"
                        )
                    }),
                user_gesture: *user_gesture,
            }),
        },
        IpcEvent::TabOpenResolved {
            profile_id,
            request_id,
            result,
        } => match result {
            TabOpenResult::OpenedNewTab { tab_id } => Some(BrowserEvent::WindowOpenResolved {
                profile_id: profile_id.clone(),
                request_id: *request_id,
                result: WindowOpenResult::OpenedNewWindow {
                    window: synthetic_window_descriptor(
                        WindowId::new(tab_id.get()),
                        WindowKind::Normal,
                        true,
                    ),
                },
            }),
            _ => Some(BrowserEvent::BrowsingContextOpenResolved {
                profile_id: profile_id.clone(),
                request_id: *request_id,
                result: ChromeBrowsingContextOpenResult::from(*result).into(),
            }),
        },
        IpcEvent::NavigationStateChanged {
            profile_id,
            browsing_context_id,
            url,
            can_go_back,
            can_go_forward,
            is_loading,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::NavigationStateChanged {
                url: url.clone(),
                can_go_back: *can_go_back,
                can_go_forward: *can_go_forward,
                is_loading: *is_loading,
            }),
        }),
        IpcEvent::CursorChanged {
            profile_id,
            browsing_context_id,
            cursor_type,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::CursorChanged {
                cursor_type: *cursor_type,
            }),
        }),
        IpcEvent::TitleUpdated {
            profile_id,
            browsing_context_id,
            title,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::TitleUpdated {
                title: title.clone(),
            }),
        }),
        IpcEvent::FaviconUrlUpdated {
            profile_id,
            browsing_context_id,
            url,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::FaviconUrlUpdated { url: url.clone() }),
        }),
        IpcEvent::BeforeUnloadDialogRequested {
            profile_id,
            browsing_context_id,
            request_id,
            reason,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::JavaScriptDialogRequested {
                request_id: *request_id,
                message: String::new(),
                default_prompt_text: None,
                r#type: cbf::data::dialog::DialogType::BeforeUnload,
                beforeunload_reason: Some((*reason).into()),
            }),
        }),
        IpcEvent::JavaScriptDialogRequested {
            profile_id,
            browsing_context_id,
            request_id,
            r#type,
            message,
            default_prompt_text,
            reason,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::JavaScriptDialogRequested {
                request_id: *request_id,
                message: message.clone(),
                default_prompt_text: default_prompt_text.clone(),
                r#type: *r#type,
                beforeunload_reason: if *r#type == DialogType::BeforeUnload {
                    Some((*reason).into())
                } else {
                    None
                },
            }),
        }),
        IpcEvent::TabClosed {
            profile_id,
            browsing_context_id,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::Closed),
        }),
        IpcEvent::TabResizeAcknowledged { .. } => None,
        IpcEvent::TabDomHtmlRead {
            profile_id,
            browsing_context_id,
            request_id,
            html,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::DomHtmlRead {
                request_id: *request_id,
                html: html.clone(),
            }),
        }),
        IpcEvent::DragStartRequested {
            profile_id,
            browsing_context_id,
            request,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::DragStartRequested {
                request: request.clone().into(),
            }),
        }),
        IpcEvent::ShutdownBlocked {
            request_id,
            dirty_browsing_context_ids,
        } => Some(BrowserEvent::ShutdownBlocked {
            request_id: *request_id,
            dirty_browsing_context_ids: dirty_browsing_context_ids
                .iter()
                .copied()
                .map(|id| id.to_browsing_context_id())
                .collect(),
        }),
        IpcEvent::ShutdownProceeding { request_id } => Some(BrowserEvent::ShutdownProceeding {
            request_id: *request_id,
        }),
        IpcEvent::ShutdownCancelled { request_id } => Some(BrowserEvent::ShutdownCancelled {
            request_id: *request_id,
        }),
        IpcEvent::ExtensionsListed {
            profile_id,
            extensions,
        } => Some(BrowserEvent::ExtensionsListed {
            profile_id: profile_id.clone(),
            extensions: extensions
                .iter()
                .cloned()
                .map(ChromeExtensionInfo::into)
                .collect(),
        }),
        IpcEvent::PromptUiOpenRequested {
            profile_id,
            browsing_context_id,
            request_id,
            kind,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::AuxiliaryWindowOpenRequested {
                request_id: *request_id,
                kind: prompt_ui_kind_to_auxiliary_window_kind(kind),
            }),
        }),
        IpcEvent::PromptUiResolved {
            profile_id,
            browsing_context_id,
            request_id,
            resolution,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::AuxiliaryWindowResolved {
                request_id: *request_id,
                resolution: prompt_ui_resolution_to_auxiliary_window_resolution(resolution),
            }),
        }),
        IpcEvent::ExtensionRuntimeWarning {
            profile_id,
            browsing_context_id,
            detail,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::ExtensionRuntimeWarning {
                detail: detail.clone(),
            }),
        }),
        IpcEvent::PromptUiOpened {
            profile_id,
            browsing_context_id,
            prompt_ui_id,
            kind,
            title,
            modal,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::AuxiliaryWindowOpened {
                window_id: prompt_ui_id_to_auxiliary_window_id(*prompt_ui_id),
                kind: prompt_ui_kind_to_auxiliary_window_kind(kind),
                title: title.clone(),
                modal: *modal,
            }),
        }),
        IpcEvent::PromptUiClosed {
            profile_id,
            browsing_context_id,
            prompt_ui_id,
            kind,
            reason,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::AuxiliaryWindowClosed {
                window_id: prompt_ui_id_to_auxiliary_window_id(*prompt_ui_id),
                kind: prompt_ui_kind_to_auxiliary_window_kind(kind),
                reason: prompt_ui_close_reason_to_auxiliary_window_close_reason(reason),
            }),
        }),
        IpcEvent::DownloadCreated {
            profile_id,
            download,
        } => Some(BrowserEvent::DownloadCreated {
            profile_id: profile_id.clone(),
            download_id: download.download_id.into(),
            source_browsing_context_id: download
                .source_tab_id
                .map(|tab_id| tab_id.to_browsing_context_id()),
            file_name: download.file_name.clone(),
            total_bytes: download.total_bytes,
            target_path: download.target_path.clone(),
        }),
        IpcEvent::DownloadUpdated {
            profile_id,
            download,
        } => Some(BrowserEvent::DownloadUpdated {
            profile_id: profile_id.clone(),
            download_id: download.download_id.into(),
            source_browsing_context_id: download
                .source_tab_id
                .map(|tab_id| tab_id.to_browsing_context_id()),
            state: download.state.into(),
            file_name: download.file_name.clone(),
            received_bytes: download.received_bytes,
            total_bytes: download.total_bytes,
            target_path: download.target_path.clone(),
            can_resume: download.can_resume,
            is_paused: download.is_paused,
        }),
        IpcEvent::DownloadCompleted {
            profile_id,
            download,
        } => Some(BrowserEvent::DownloadCompleted {
            profile_id: profile_id.clone(),
            download_id: download.download_id.into(),
            source_browsing_context_id: download
                .source_tab_id
                .map(|tab_id| tab_id.to_browsing_context_id()),
            outcome: download.outcome.into(),
            file_name: download.file_name.clone(),
            received_bytes: download.received_bytes,
            total_bytes: download.total_bytes,
            target_path: download.target_path.clone(),
        }),
    }
}

fn synthetic_window_descriptor(
    window_id: WindowId,
    kind: WindowKind,
    focused: bool,
) -> WindowDescriptor {
    WindowDescriptor {
        window_id,
        kind,
        state: WindowState::Normal,
        focused,
        incognito: false,
        always_on_top: false,
        bounds: WindowBounds {
            left: 0,
            top: 0,
            width: 1280,
            height: 720,
        },
    }
}

fn prompt_ui_permission_to_permission_prompt_type(
    permission: &PromptUiPermissionType,
    permission_key: Option<&str>,
) -> PermissionPromptType {
    match permission {
        PromptUiPermissionType::Geolocation => PermissionPromptType::Geolocation,
        PromptUiPermissionType::Notifications => PermissionPromptType::Notifications,
        PromptUiPermissionType::AudioCapture => PermissionPromptType::AudioCapture,
        PromptUiPermissionType::VideoCapture => PermissionPromptType::VideoCapture,
        PromptUiPermissionType::Unknown => permission_key
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| PermissionPromptType::Other(value.to_string()))
            .unwrap_or(PermissionPromptType::Unknown),
    }
}

fn prompt_ui_kind_to_auxiliary_window_kind(kind: &PromptUiKind) -> AuxiliaryWindowKind {
    match kind {
        PromptUiKind::PermissionPrompt {
            permission,
            permission_key,
        } => AuxiliaryWindowKind::PermissionPrompt {
            permission: prompt_ui_permission_to_permission_prompt_type(
                permission,
                permission_key.as_deref(),
            ),
        },
        PromptUiKind::DownloadPrompt {
            download_id,
            file_name,
            total_bytes,
            suggested_path,
            reason,
        } => AuxiliaryWindowKind::DownloadPrompt {
            download_id: (*download_id).into(),
            file_name: file_name.clone(),
            total_bytes: *total_bytes,
            suggested_path: suggested_path.clone(),
            action_hint: (*reason).into(),
        },
        PromptUiKind::ExtensionInstallPrompt {
            extension_id,
            extension_name,
            permission_names,
        } => AuxiliaryWindowKind::ExtensionInstallPrompt {
            extension_id: extension_id.clone(),
            extension_name: extension_name.clone(),
            permission_names: permission_names.clone(),
        },
        PromptUiKind::PrintPreviewDialog => AuxiliaryWindowKind::PrintPreviewDialog,
        PromptUiKind::Unknown => AuxiliaryWindowKind::Unknown,
    }
}

fn prompt_ui_resolution_to_auxiliary_window_resolution(
    resolution: &PromptUiResolution,
) -> AuxiliaryWindowResolution {
    match resolution {
        PromptUiResolution::PermissionPrompt {
            permission,
            permission_key,
            result,
        } => AuxiliaryWindowResolution::PermissionPrompt {
            permission: prompt_ui_permission_to_permission_prompt_type(
                permission,
                permission_key.as_deref(),
            ),
            result: match result {
                PromptUiResolutionResult::Allowed => PermissionPromptResult::Allowed,
                PromptUiResolutionResult::Denied => PermissionPromptResult::Denied,
                PromptUiResolutionResult::Aborted => PermissionPromptResult::Aborted,
                PromptUiResolutionResult::Unknown => PermissionPromptResult::Unknown,
            },
        },
        PromptUiResolution::DownloadPrompt {
            download_id,
            destination_path,
            result,
        } => AuxiliaryWindowResolution::DownloadPrompt {
            download_id: (*download_id).into(),
            destination_path: destination_path.clone(),
            result: match result {
                ChromeDownloadPromptResult::Allowed => DownloadPromptResult::Allowed,
                ChromeDownloadPromptResult::Denied => DownloadPromptResult::Denied,
                ChromeDownloadPromptResult::Aborted => DownloadPromptResult::Aborted,
            },
        },
        PromptUiResolution::ExtensionInstallPrompt {
            extension_id,
            result,
            detail,
        } => AuxiliaryWindowResolution::ExtensionInstallPrompt {
            extension_id: extension_id.clone(),
            result: match result {
                PromptUiExtensionInstallResult::Accepted => ExtensionInstallPromptResult::Accepted,
                PromptUiExtensionInstallResult::AcceptedWithWithheldPermissions => {
                    ExtensionInstallPromptResult::AcceptedWithWithheldPermissions
                }
                PromptUiExtensionInstallResult::UserCanceled => {
                    ExtensionInstallPromptResult::UserCanceled
                }
                PromptUiExtensionInstallResult::Aborted => ExtensionInstallPromptResult::Aborted,
            },
            detail: detail.clone(),
        },
        PromptUiResolution::PrintPreviewDialog { result } => match result {
            PromptUiDialogResult::Proceeded => AuxiliaryWindowResolution::Unknown,
            PromptUiDialogResult::Canceled => AuxiliaryWindowResolution::Unknown,
            PromptUiDialogResult::Aborted => AuxiliaryWindowResolution::Unknown,
            PromptUiDialogResult::Unknown => AuxiliaryWindowResolution::Unknown,
        },
        PromptUiResolution::Unknown => AuxiliaryWindowResolution::Unknown,
    }
}

fn prompt_ui_id_to_auxiliary_window_id(value: PromptUiId) -> AuxiliaryWindowId {
    AuxiliaryWindowId::new(value.get())
}

fn prompt_ui_close_reason_to_auxiliary_window_close_reason(
    value: &PromptUiCloseReason,
) -> AuxiliaryWindowCloseReason {
    match value {
        PromptUiCloseReason::UserCanceled => AuxiliaryWindowCloseReason::UserCanceled,
        PromptUiCloseReason::HostForced => AuxiliaryWindowCloseReason::HostForced,
        PromptUiCloseReason::SystemDismissed => AuxiliaryWindowCloseReason::SystemDismissed,
        PromptUiCloseReason::Unknown => AuxiliaryWindowCloseReason::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use cbf::{
        data::{
            download::DownloadPromptActionHint,
            extension::{
                AuxiliaryWindowCloseReason, AuxiliaryWindowId, AuxiliaryWindowKind,
                AuxiliaryWindowResolution, ExtensionInstallPromptResult, PermissionPromptResult,
                PermissionPromptType,
            },
            ids::{BrowsingContextId, TransientBrowsingContextId},
            transient_browsing_context::{
                TransientBrowsingContextCloseReason, TransientBrowsingContextKind,
            },
        },
        event::{BrowserEvent, BrowsingContextEvent, TransientBrowsingContextEvent},
    };

    use super::map_ipc_event_to_generic;
    use crate::{
        data::{
            download::ChromeDownloadPromptReason,
            ids::{PopupId, TabId},
            prompt_ui::{
                PromptUiCloseReason, PromptUiExtensionInstallResult, PromptUiId, PromptUiKind,
                PromptUiResolution,
            },
        },
        ffi::IpcEvent,
    };

    #[test]
    fn tab_created_maps_tab_id_into_browsing_context_id() {
        let event = IpcEvent::TabCreated {
            profile_id: "default".to_string(),
            browsing_context_id: TabId::new(7),
            request_id: 1,
        };

        let mapped = map_ipc_event_to_generic(&event).unwrap();
        assert!(matches!(
            mapped,
            BrowserEvent::BrowsingContext {
                browsing_context_id,
                event,
                ..
            } if browsing_context_id == BrowsingContextId::new(7)
                && matches!(*event, BrowsingContextEvent::Created { request_id: 1 })
        ));
    }

    #[test]
    fn shutdown_blocked_maps_dirty_tab_ids_into_browsing_context_ids() {
        let event = IpcEvent::ShutdownBlocked {
            request_id: 9,
            dirty_browsing_context_ids: vec![TabId::new(2), TabId::new(3)],
        };

        let mapped = map_ipc_event_to_generic(&event).unwrap();
        assert!(matches!(
            mapped,
            BrowserEvent::ShutdownBlocked {
                request_id: 9,
                dirty_browsing_context_ids
            } if dirty_browsing_context_ids
                == vec![BrowsingContextId::new(2), BrowsingContextId::new(3)]
        ));
    }

    #[test]
    fn extension_popup_opened_maps_into_transient_browsing_context() {
        let event = IpcEvent::ExtensionPopupOpened {
            profile_id: "default".to_string(),
            browsing_context_id: TabId::new(44),
            popup_id: 88,
            extension_id: "ext".to_string(),
            title: "Popup".to_string(),
        };

        let mapped = map_ipc_event_to_generic(&event).unwrap();
        assert!(matches!(
            mapped,
            BrowserEvent::TransientBrowsingContext {
                transient_browsing_context_id,
                parent_browsing_context_id,
                event,
                ..
            } if transient_browsing_context_id == TransientBrowsingContextId::new(88)
                && parent_browsing_context_id == BrowsingContextId::new(44)
                && matches!(
                    *event,
                    TransientBrowsingContextEvent::Opened {
                        kind: TransientBrowsingContextKind::Popup,
                        title: Some(ref title),
                    } if title == "Popup"
                )
        ));
    }

    #[test]
    fn extension_popup_closed_maps_into_transient_browsing_context() {
        let event = IpcEvent::ExtensionPopupClosed {
            profile_id: "default".to_string(),
            browsing_context_id: TabId::new(44),
            popup_id: 88,
        };

        let mapped = map_ipc_event_to_generic(&event).unwrap();
        assert!(matches!(
            mapped,
            BrowserEvent::TransientBrowsingContext {
                transient_browsing_context_id,
                parent_browsing_context_id,
                event,
                ..
            } if transient_browsing_context_id == TransientBrowsingContextId::new(88)
                && parent_browsing_context_id == BrowsingContextId::new(44)
                && matches!(
                    *event,
                    TransientBrowsingContextEvent::Closed {
                        reason: TransientBrowsingContextCloseReason::Unknown
                    }
                )
        ));
    }

    #[test]
    fn extension_popup_ime_bounds_maps_into_transient_browsing_context() {
        let event = IpcEvent::ExtensionPopupImeBoundsUpdated {
            profile_id: "default".to_string(),
            browsing_context_id: TabId::new(44),
            popup_id: PopupId::new(88),
            update: crate::data::ime::ChromeImeBoundsUpdate {
                composition: None,
                selection: Some(crate::data::ime::ChromeTextSelectionBounds {
                    range_start: 1,
                    range_end: 1,
                    caret_rect: crate::data::ime::ChromeImeRect {
                        x: 10,
                        y: 20,
                        width: 2,
                        height: 16,
                    },
                    first_selection_rect: crate::data::ime::ChromeImeRect {
                        x: 10,
                        y: 20,
                        width: 2,
                        height: 16,
                    },
                }),
            },
        };

        let mapped = map_ipc_event_to_generic(&event).unwrap();
        assert!(matches!(
            mapped,
            BrowserEvent::TransientBrowsingContext {
                transient_browsing_context_id,
                parent_browsing_context_id,
                event,
                ..
            } if transient_browsing_context_id == TransientBrowsingContextId::new(88)
                && parent_browsing_context_id == BrowsingContextId::new(44)
                && matches!(
                    *event,
                    TransientBrowsingContextEvent::ImeBoundsUpdated { ref update }
                        if update.selection.as_ref().is_some_and(|selection| selection.range_start == 1)
                )
        ));
    }

    #[test]
    fn prompt_ui_requested_maps_into_permission_auxiliary_window() {
        let event = IpcEvent::PromptUiOpenRequested {
            profile_id: "default".to_string(),
            browsing_context_id: TabId::new(44),
            request_id: 12,
            kind: PromptUiKind::PermissionPrompt {
                permission: crate::data::prompt_ui::PromptUiPermissionType::Geolocation,
                permission_key: None,
            },
        };

        let mapped = map_ipc_event_to_generic(&event).unwrap();
        assert!(matches!(
            mapped,
            BrowserEvent::BrowsingContext {
                browsing_context_id,
                event,
                ..
            } if browsing_context_id == BrowsingContextId::new(44)
                && matches!(
                    *event,
                    BrowsingContextEvent::AuxiliaryWindowOpenRequested {
                        request_id: 12,
                        kind: AuxiliaryWindowKind::PermissionPrompt {
                            permission: PermissionPromptType::Geolocation
                        }
                    }
                )
        ));
    }

    #[test]
    fn prompt_ui_resolved_maps_into_permission_auxiliary_resolution() {
        let event = IpcEvent::PromptUiResolved {
            profile_id: "default".to_string(),
            browsing_context_id: TabId::new(44),
            request_id: 12,
            resolution: crate::data::prompt_ui::PromptUiResolution::PermissionPrompt {
                permission: crate::data::prompt_ui::PromptUiPermissionType::Geolocation,
                permission_key: None,
                result: crate::data::prompt_ui::PromptUiResolutionResult::Denied,
            },
        };

        let mapped = map_ipc_event_to_generic(&event).unwrap();
        assert!(matches!(
            mapped,
            BrowserEvent::BrowsingContext {
                browsing_context_id,
                event,
                ..
            } if browsing_context_id == BrowsingContextId::new(44)
                && matches!(
                    *event,
                    BrowsingContextEvent::AuxiliaryWindowResolved {
                        request_id: 12,
                        resolution: AuxiliaryWindowResolution::PermissionPrompt {
                            permission: PermissionPromptType::Geolocation,
                            result: PermissionPromptResult::Denied
                        }
                    }
                )
        ));
    }

    #[test]
    fn prompt_ui_requested_maps_unknown_with_key_to_other_permission_prompt() {
        let event = IpcEvent::PromptUiOpenRequested {
            profile_id: "default".to_string(),
            browsing_context_id: TabId::new(44),
            request_id: 12,
            kind: PromptUiKind::PermissionPrompt {
                permission: crate::data::prompt_ui::PromptUiPermissionType::Unknown,
                permission_key: Some("window_management".to_string()),
            },
        };

        let mapped = map_ipc_event_to_generic(&event).unwrap();
        assert!(matches!(
            mapped,
            BrowserEvent::BrowsingContext {
                browsing_context_id,
                event,
                ..
            } if browsing_context_id == BrowsingContextId::new(44)
                && matches!(
                    *event,
                    BrowsingContextEvent::AuxiliaryWindowOpenRequested {
                        request_id: 12,
                        kind: AuxiliaryWindowKind::PermissionPrompt {
                            permission: PermissionPromptType::Other(ref key)
                        }
                    } if key == "window_management"
                )
        ));
    }

    #[test]
    fn prompt_ui_requested_maps_download_reason_into_auxiliary_window() {
        let event = IpcEvent::PromptUiOpenRequested {
            profile_id: "default".to_string(),
            browsing_context_id: TabId::new(12),
            request_id: 71,
            kind: PromptUiKind::DownloadPrompt {
                download_id: crate::data::download::ChromeDownloadId::new(55),
                file_name: "file.bin".to_string(),
                total_bytes: Some(42),
                suggested_path: Some("/tmp/file.bin".to_string()),
                reason: ChromeDownloadPromptReason::SaveAs,
            },
        };

        let mapped = map_ipc_event_to_generic(&event).unwrap();
        assert!(matches!(
            mapped,
            BrowserEvent::BrowsingContext {
                browsing_context_id,
                event,
                ..
            } if browsing_context_id == BrowsingContextId::new(12)
                && matches!(
                    *event,
                    BrowsingContextEvent::AuxiliaryWindowOpenRequested {
                        request_id: 71,
                        kind: AuxiliaryWindowKind::DownloadPrompt {
                            action_hint: DownloadPromptActionHint::SelectDestination,
                            ..
                        }
                    }
                )
        ));
    }

    #[test]
    fn prompt_ui_requested_maps_dlp_reason_into_deny_action_hint() {
        let event = IpcEvent::PromptUiOpenRequested {
            profile_id: "default".to_string(),
            browsing_context_id: TabId::new(12),
            request_id: 72,
            kind: PromptUiKind::DownloadPrompt {
                download_id: crate::data::download::ChromeDownloadId::new(56),
                file_name: "file.bin".to_string(),
                total_bytes: Some(42),
                suggested_path: Some("/tmp/file.bin".to_string()),
                reason: ChromeDownloadPromptReason::DlpBlocked,
            },
        };

        let mapped = map_ipc_event_to_generic(&event).unwrap();
        assert!(matches!(
            mapped,
            BrowserEvent::BrowsingContext {
                browsing_context_id,
                event,
                ..
            } if browsing_context_id == BrowsingContextId::new(12)
                && matches!(
                    *event,
                    BrowsingContextEvent::AuxiliaryWindowOpenRequested {
                        request_id: 72,
                        kind: AuxiliaryWindowKind::DownloadPrompt {
                            action_hint: DownloadPromptActionHint::Deny,
                            ..
                        }
                    }
                )
        ));
    }

    #[test]
    fn prompt_ui_open_requested_maps_extension_install_kind() {
        let event = IpcEvent::PromptUiOpenRequested {
            profile_id: "default".to_string(),
            browsing_context_id: TabId::new(9),
            request_id: 2,
            kind: PromptUiKind::ExtensionInstallPrompt {
                extension_id: "ext".to_string(),
                extension_name: "ExtName".to_string(),
                permission_names: vec!["tabs".to_string()],
            },
        };

        let mapped = map_ipc_event_to_generic(&event).unwrap();
        assert!(matches!(
            mapped,
            BrowserEvent::BrowsingContext {
                browsing_context_id,
                event,
                ..
            } if browsing_context_id == BrowsingContextId::new(9)
                && matches!(
                    *event,
                    BrowsingContextEvent::AuxiliaryWindowOpenRequested {
                        request_id: 2,
                        kind: AuxiliaryWindowKind::ExtensionInstallPrompt {
                            ref extension_id,
                            ref extension_name,
                            ref permission_names,
                        }
                    } if extension_id == "ext"
                        && extension_name == "ExtName"
                        && permission_names == &vec!["tabs".to_string()]
                )
        ));
    }

    #[test]
    fn prompt_ui_resolved_maps_extension_install_resolution() {
        let event = IpcEvent::PromptUiResolved {
            profile_id: "default".to_string(),
            browsing_context_id: TabId::new(9),
            request_id: 5,
            resolution: PromptUiResolution::ExtensionInstallPrompt {
                extension_id: "ext".to_string(),
                result: PromptUiExtensionInstallResult::UserCanceled,
                detail: Some("user dismissed".to_string()),
            },
        };

        let mapped = map_ipc_event_to_generic(&event).unwrap();
        assert!(matches!(
            mapped,
            BrowserEvent::BrowsingContext {
                browsing_context_id,
                event,
                ..
            } if browsing_context_id == BrowsingContextId::new(9)
                && matches!(
                    *event,
                    BrowsingContextEvent::AuxiliaryWindowResolved {
                        request_id: 5,
                        resolution: AuxiliaryWindowResolution::ExtensionInstallPrompt {
                            ref extension_id,
                            result: ExtensionInstallPromptResult::UserCanceled,
                            detail: Some(ref detail),
                        }
                    } if extension_id == "ext" && detail == "user dismissed"
                )
        ));
    }

    #[test]
    fn prompt_ui_opened_maps_into_auxiliary_window_opened() {
        let event = IpcEvent::PromptUiOpened {
            profile_id: "default".to_string(),
            browsing_context_id: TabId::new(11),
            prompt_ui_id: PromptUiId::new(88),
            kind: PromptUiKind::PrintPreviewDialog,
            title: Some("Print".to_string()),
            modal: true,
        };

        let mapped = map_ipc_event_to_generic(&event).unwrap();
        assert!(matches!(
            mapped,
            BrowserEvent::BrowsingContext {
                browsing_context_id,
                event,
                ..
            } if browsing_context_id == BrowsingContextId::new(11)
                && matches!(
                    *event,
                    BrowsingContextEvent::AuxiliaryWindowOpened {
                        window_id,
                        kind: AuxiliaryWindowKind::PrintPreviewDialog,
                        title: Some(ref title),
                        modal: true,
                    } if window_id == AuxiliaryWindowId::new(88) && title == "Print"
                )
        ));
    }

    #[test]
    fn prompt_ui_closed_maps_into_auxiliary_window_closed() {
        let event = IpcEvent::PromptUiClosed {
            profile_id: "default".to_string(),
            browsing_context_id: TabId::new(11),
            prompt_ui_id: PromptUiId::new(88),
            kind: PromptUiKind::PrintPreviewDialog,
            reason: PromptUiCloseReason::SystemDismissed,
        };

        let mapped = map_ipc_event_to_generic(&event).unwrap();
        assert!(matches!(
            mapped,
            BrowserEvent::BrowsingContext {
                browsing_context_id,
                event,
                ..
            } if browsing_context_id == BrowsingContextId::new(11)
                && matches!(
                    *event,
                    BrowsingContextEvent::AuxiliaryWindowClosed {
                        window_id,
                        kind: AuxiliaryWindowKind::PrintPreviewDialog,
                        reason: AuxiliaryWindowCloseReason::SystemDismissed,
                    } if window_id == AuxiliaryWindowId::new(88)
                )
        ));
    }
}
