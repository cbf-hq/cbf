use cbf::{
    data::{
        extension::{
            AuxiliaryWindowKind, AuxiliaryWindowResolution, PermissionPromptResult,
            PermissionPromptType,
        },
        ids::WindowId,
        profile::ProfileInfo,
        window_open::{
            WindowBounds, WindowDescriptor, WindowKind, WindowOpenReason, WindowOpenRequest,
            WindowOpenResult, WindowState,
        },
    },
    event::{BackendStopReason, BeforeUnloadReason},
};
use cbf::{
    error::BackendErrorInfo,
    event::{BrowserEvent, BrowsingContextEvent, DialogType},
};

use crate::data::{
    prompt_ui::{
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
    BackendStopped { reason: BackendStopReason },
    /// Backend error surfaced from command/event processing.
    BackendError {
        info: BackendErrorInfo,
        terminal_hint: bool,
    },
    /// Profile list obtained through backend-side request/response path.
    ProfilesListed { profiles: Vec<ProfileInfo> },
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
            profiles: profiles.clone(),
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
        IpcEvent::WebContentsCreated {
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
                update: update.clone(),
            }),
        }),
        IpcEvent::ContextMenuRequested {
            profile_id,
            browsing_context_id,
            menu,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::ContextMenuRequested { menu: menu.clone() }),
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
                result: (*result).into(),
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
                r#type: DialogType::BeforeUnload,
                beforeunload_reason: Some(match reason {
                    BeforeUnloadReason::Unknown => BeforeUnloadReason::Unknown,
                    BeforeUnloadReason::CloseBrowsingContext => {
                        BeforeUnloadReason::CloseBrowsingContext
                    }
                    BeforeUnloadReason::Navigate => BeforeUnloadReason::Navigate,
                    BeforeUnloadReason::Reload => BeforeUnloadReason::Reload,
                    BeforeUnloadReason::WindowClose => BeforeUnloadReason::WindowClose,
                }),
            }),
        }),
        IpcEvent::WebContentsClosed {
            profile_id,
            browsing_context_id,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::Closed),
        }),
        IpcEvent::WebContentsResizeAcknowledged { .. } => None,
        IpcEvent::WebContentsDomHtmlRead {
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
                request: request.clone(),
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
            extensions: extensions.clone(),
        }),
        IpcEvent::AuxiliaryWindowOpenRequested {
            profile_id,
            browsing_context_id,
            request_id,
            kind,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::AuxiliaryWindowOpenRequested {
                request_id: *request_id,
                kind: kind.clone(),
            }),
        }),
        IpcEvent::AuxiliaryWindowResolved {
            profile_id,
            browsing_context_id,
            request_id,
            resolution,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::AuxiliaryWindowResolved {
                request_id: *request_id,
                resolution: resolution.clone(),
            }),
        }),
        IpcEvent::PromptUiRequested {
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
        IpcEvent::AuxiliaryWindowOpened {
            profile_id,
            browsing_context_id,
            window_id,
            kind,
            title,
            modal,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::AuxiliaryWindowOpened {
                window_id: *window_id,
                kind: kind.clone(),
                title: title.clone(),
                modal: *modal,
            }),
        }),
        IpcEvent::AuxiliaryWindowClosed {
            profile_id,
            browsing_context_id,
            window_id,
            kind,
            reason,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: browsing_context_id.to_browsing_context_id(),
            event: Box::new(BrowsingContextEvent::AuxiliaryWindowClosed {
                window_id: *window_id,
                kind: kind.clone(),
                reason: *reason,
            }),
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
        PromptUiResolution::Unknown => AuxiliaryWindowResolution::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use cbf::{
        data::{
            extension::{
                AuxiliaryWindowKind, AuxiliaryWindowResolution, PermissionPromptResult,
                PermissionPromptType,
            },
            ids::BrowsingContextId,
        },
        event::{BrowserEvent, BrowsingContextEvent},
    };

    use super::map_ipc_event_to_generic;
    use crate::{
        data::{ids::TabId, prompt_ui::PromptUiKind},
        ffi::IpcEvent,
    };

    #[test]
    fn web_contents_created_maps_tab_id_into_browsing_context_id() {
        let event = IpcEvent::WebContentsCreated {
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
    fn prompt_ui_requested_maps_into_permission_auxiliary_window() {
        let event = IpcEvent::PromptUiRequested {
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
        let event = IpcEvent::PromptUiRequested {
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
}
