use super::ids::{BrowsingContextId, WindowId};

/// Host-level window type requested by backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowKind {
    Unknown,
    Normal,
    Popup,
}

/// Host-level window state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Unknown,
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
}

/// Window bounds in screen coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowBounds {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
}

/// Detailed window metadata exchanged between backend and host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowDescriptor {
    pub window_id: WindowId,
    pub kind: WindowKind,
    pub state: WindowState,
    pub focused: bool,
    pub incognito: bool,
    pub always_on_top: bool,
    pub bounds: WindowBounds,
}

/// Why a window open was requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowOpenReason {
    Unknown,
    Navigation,
    ScriptOpen,
    ExtensionApi,
}

/// Host-mediated window open request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowOpenRequest {
    pub request_id: u64,
    pub reason: WindowOpenReason,
    pub opener_window_id: Option<WindowId>,
    pub opener_browsing_context_id: Option<BrowsingContextId>,
    pub target_url: Option<String>,
    pub requested_kind: WindowKind,
    pub user_gesture: bool,
}

/// Host decision for an open window request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowOpenResponse {
    AllowExistingWindow { window: WindowDescriptor },
    AllowNewWindow { window: WindowDescriptor },
    Deny,
}

/// Result emitted after backend applies host window-open response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowOpenResult {
    OpenedExistingWindow { window: WindowDescriptor },
    OpenedNewWindow { window: WindowDescriptor },
    Denied,
    Aborted,
}
