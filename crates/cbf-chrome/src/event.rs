use cbf::{
    data::profile::ProfileInfo,
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
            event: Box::new(BrowsingContextEvent::ContextMenuRequested {
                menu: menu.clone(),
            }),
        }),
        IpcEvent::NewWebContentsRequested {
            profile_id,
            browsing_context_id,
            target_url,
            is_popup,
        } => Some(BrowserEvent::BrowsingContext {
            profile_id: profile_id.clone(),
            browsing_context_id: *browsing_context_id,
            event: Box::new(BrowsingContextEvent::NewBrowsingContextRequested {
                target_url: target_url.clone(),
                is_popup: *is_popup,
            }),
        }),
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
            event: Box::new(BrowsingContextEvent::FaviconUrlUpdated {
                url: url.clone(),
            }),
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
    }
}
