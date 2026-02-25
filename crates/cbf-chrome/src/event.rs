use cbf::{
    data::{
        browsing_context_open::{BrowsingContextOpenHint, BrowsingContextOpenResult},
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
            browsing_context_id: *browsing_context_id,
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
            browsing_context_id: *browsing_context_id,
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
            browsing_context_id: *browsing_context_id,
            event: Box::new(BrowsingContextEvent::ContextMenuRequested { menu: menu.clone() }),
        }),
        IpcEvent::BrowsingContextOpenRequested {
            profile_id,
            request_id,
            source_browsing_context_id,
            target_url,
            open_hint,
            user_gesture,
        } => match open_hint {
            BrowsingContextOpenHint::NewWindow | BrowsingContextOpenHint::Popup => {
                Some(BrowserEvent::WindowOpenRequested {
                    profile_id: profile_id.clone(),
                    request: WindowOpenRequest {
                        request_id: *request_id,
                        reason: WindowOpenReason::Navigation,
                        opener_window_id: None,
                        opener_browsing_context_id: *source_browsing_context_id,
                        target_url: Some(target_url.clone()),
                        requested_kind: if matches!(open_hint, BrowsingContextOpenHint::Popup) {
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
                source_browsing_context_id: *source_browsing_context_id,
                target_url: target_url.clone(),
                open_hint: *open_hint,
                user_gesture: *user_gesture,
            }),
        },
        IpcEvent::BrowsingContextOpenResolved {
            profile_id,
            request_id,
            result,
        } => match result {
            BrowsingContextOpenResult::OpenedNewContext { browsing_context_id } => {
                Some(BrowserEvent::WindowOpenResolved {
                    profile_id: profile_id.clone(),
                    request_id: *request_id,
                    result: WindowOpenResult::OpenedNewWindow {
                        window: synthetic_window_descriptor(
                            WindowId::new(browsing_context_id.get()),
                            WindowKind::Normal,
                            true,
                        ),
                    },
                })
            }
            _ => Some(BrowserEvent::BrowsingContextOpenResolved {
                profile_id: profile_id.clone(),
                request_id: *request_id,
                result: *result,
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
            browsing_context_id: *browsing_context_id,
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
            browsing_context_id: *browsing_context_id,
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
            browsing_context_id: *browsing_context_id,
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
            browsing_context_id: *browsing_context_id,
            event: Box::new(BrowsingContextEvent::FaviconUrlUpdated { url: url.clone() }),
        }),
        IpcEvent::BeforeUnloadDialogRequested {
            profile_id,
            browsing_context_id,
            request_id,
            reason,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: *browsing_context_id,
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
            browsing_context_id: *browsing_context_id,
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
            browsing_context_id: *browsing_context_id,
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
            browsing_context_id: *browsing_context_id,
            event: Box::new(BrowsingContextEvent::DragStartRequested {
                request: request.clone(),
            }),
        }),
        IpcEvent::ShutdownBlocked {
            request_id,
            dirty_browsing_context_ids,
        } => Some(BrowserEvent::ShutdownBlocked {
            request_id: *request_id,
            dirty_browsing_context_ids: dirty_browsing_context_ids.clone(),
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
            browsing_context_id: *browsing_context_id,
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
            browsing_context_id: *browsing_context_id,
            event: Box::new(BrowsingContextEvent::AuxiliaryWindowResolved {
                request_id: *request_id,
                resolution: resolution.clone(),
            }),
        }),
        IpcEvent::ExtensionRuntimeWarning {
            profile_id,
            browsing_context_id,
            detail,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: *browsing_context_id,
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
            browsing_context_id: *browsing_context_id,
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
            browsing_context_id: *browsing_context_id,
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
