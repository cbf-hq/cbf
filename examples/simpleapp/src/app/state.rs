use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use cbf::data::{
    download::DownloadState,
    extension::ExtensionInfo,
    ids::{BrowsingContextId, TransientBrowsingContextId, WindowId as HostWindowId},
};
use cursor_icon::CursorIcon;

pub(crate) type SharedStateHandle = Arc<Mutex<SharedState>>;

pub(crate) const PRIMARY_HOST_WINDOW_ID: HostWindowId = HostWindowId::new(u64::MAX - 1);
pub(crate) const DEVTOOLS_HOST_WINDOW_ID: HostWindowId = HostWindowId::new(u64::MAX - 2);
pub(crate) const MAIN_PAGE_CREATE_REQUEST_ID: u64 = 1;
pub(crate) const TOOLBAR_CREATE_REQUEST_ID: u64 = 2;
pub(crate) const OVERLAY_CREATE_REQUEST_ID: u64 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PendingWindowBrowsingContextRole {
    Page,
    Toolbar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingWindowBrowsingContextCreate {
    pub(crate) window_id: HostWindowId,
    pub(crate) role: PendingWindowBrowsingContextRole,
}

pub(crate) struct SharedState {
    pub(crate) primary_browsing_context_id: Option<BrowsingContextId>,
    pub(crate) toolbar_browsing_context_id: Option<BrowsingContextId>,
    pub(crate) overlay_browsing_context_id: Option<BrowsingContextId>,
    pub(crate) devtools_browsing_context_id: Option<BrowsingContextId>,
    pub(crate) primary_host_window_id: Option<HostWindowId>,
    pub(crate) browsing_context_to_window: HashMap<BrowsingContextId, HostWindowId>,
    pub(crate) window_to_browsing_contexts: HashMap<HostWindowId, HashSet<BrowsingContextId>>,
    pub(crate) pending_window_open_requests: HashMap<u64, HostWindowId>,
    pub(crate) pending_window_browsing_context_creates:
        HashMap<u64, PendingWindowBrowsingContextCreate>,
    pub(crate) window_to_page_browsing_context: HashMap<HostWindowId, BrowsingContextId>,
    pub(crate) window_to_toolbar_browsing_context: HashMap<HostWindowId, BrowsingContextId>,
    pub(crate) next_browsing_context_request_id: u64,
    pub(crate) transient_to_window: HashMap<TransientBrowsingContextId, HostWindowId>,
    pub(crate) window_to_transient: HashMap<HostWindowId, TransientBrowsingContextId>,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            primary_browsing_context_id: None,
            toolbar_browsing_context_id: None,
            overlay_browsing_context_id: None,
            devtools_browsing_context_id: None,
            primary_host_window_id: None,
            browsing_context_to_window: HashMap::new(),
            window_to_browsing_contexts: HashMap::new(),
            pending_window_open_requests: HashMap::new(),
            pending_window_browsing_context_creates: HashMap::new(),
            window_to_page_browsing_context: HashMap::new(),
            window_to_toolbar_browsing_context: HashMap::new(),
            next_browsing_context_request_id: 10_000,
            transient_to_window: HashMap::new(),
            window_to_transient: HashMap::new(),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct DownloadStatus {
    pub(crate) source_browsing_context_id: Option<BrowsingContextId>,
    pub(crate) file_name: String,
    pub(crate) received_bytes: u64,
    pub(crate) total_bytes: Option<u64>,
    pub(crate) state: DownloadState,
    pub(crate) is_paused: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct TransientPopupState {
    pub(crate) parent_browsing_context_id: BrowsingContextId,
    pub(crate) title: String,
    pub(crate) size: Option<(u32, u32)>,
    pub(crate) prev_sent_size: Option<(u32, u32)>,
}

#[derive(Debug)]
pub(crate) enum CoreAction {
    ExitEventLoop,
    EnsureMainWindow,
    EnsureHostWindow {
        window: cbf::data::window_open::WindowDescriptor,
    },
    EnsureDevToolsWindow,
    EnsureTransientHostWindow {
        transient_browsing_context_id: TransientBrowsingContextId,
        title: String,
        width: u32,
        height: u32,
    },
    CloseHostWindow {
        window_id: HostWindowId,
    },
    ResizeHostWindow {
        window_id: HostWindowId,
        width: u32,
        height: u32,
    },
    SyncWindowScene {
        window_id: HostWindowId,
    },
    UpdateWindowTitle {
        window_id: HostWindowId,
        title: String,
    },
    UpdateCursor {
        window_id: HostWindowId,
        cursor: CursorIcon,
    },
    SetExtensionsMenuLoading,
    ReplaceExtensionsMenu {
        extensions: Vec<ExtensionInfo>,
    },
    PresentJavaScriptDialog {
        target: JavaScriptDialogTarget,
        request_id: u64,
        request: cbf::data::dialog::JavaScriptDialogRequest,
    },
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum JavaScriptDialogTarget {
    BrowsingContext(BrowsingContextId),
    TransientBrowsingContext(TransientBrowsingContextId),
}

pub(crate) fn set_primary_host_window_id(shared: &SharedStateHandle, window_id: HostWindowId) {
    let mut guard = shared.lock().expect("shared state lock poisoned");
    guard.primary_host_window_id = Some(window_id);
}

pub(crate) fn primary_host_window_id(shared: &SharedStateHandle) -> Option<HostWindowId> {
    shared
        .lock()
        .expect("shared state lock poisoned")
        .primary_host_window_id
}

pub(crate) fn bind_browsing_context_to_window(
    shared: &SharedStateHandle,
    browsing_context_id: BrowsingContextId,
    window_id: HostWindowId,
) {
    let mut guard = shared.lock().expect("shared state lock poisoned");
    guard
        .browsing_context_to_window
        .insert(browsing_context_id, window_id);
    guard
        .window_to_browsing_contexts
        .entry(window_id)
        .or_default()
        .insert(browsing_context_id);
}

pub(crate) fn unbind_browsing_context(
    shared: &SharedStateHandle,
    browsing_context_id: BrowsingContextId,
) -> Option<HostWindowId> {
    let mut guard = shared.lock().expect("shared state lock poisoned");
    let window_id = guard
        .browsing_context_to_window
        .remove(&browsing_context_id)?;
    if let Some(set) = guard.window_to_browsing_contexts.get_mut(&window_id) {
        set.remove(&browsing_context_id);
        if set.is_empty() {
            guard.window_to_browsing_contexts.remove(&window_id);
        }
    }
    Some(window_id)
}

pub(crate) fn window_id_for_browsing_context(
    shared: &SharedStateHandle,
    browsing_context_id: BrowsingContextId,
) -> Option<HostWindowId> {
    shared
        .lock()
        .expect("shared state lock poisoned")
        .browsing_context_to_window
        .get(&browsing_context_id)
        .copied()
}

pub(crate) fn browsing_context_ids_for_window(
    shared: &SharedStateHandle,
    window_id: HostWindowId,
) -> Vec<BrowsingContextId> {
    shared
        .lock()
        .expect("shared state lock poisoned")
        .window_to_browsing_contexts
        .get(&window_id)
        .map(|set| set.iter().copied().collect())
        .unwrap_or_default()
}

pub(crate) fn has_bound_windows(shared: &SharedStateHandle) -> bool {
    let guard = shared.lock().expect("shared state lock poisoned");
    !guard.window_to_browsing_contexts.is_empty() || !guard.window_to_transient.is_empty()
}

pub(crate) fn bind_transient_to_window(
    shared: &SharedStateHandle,
    transient_browsing_context_id: TransientBrowsingContextId,
    window_id: HostWindowId,
) {
    let mut guard = shared.lock().expect("shared state lock poisoned");
    guard
        .transient_to_window
        .insert(transient_browsing_context_id, window_id);
    guard
        .window_to_transient
        .insert(window_id, transient_browsing_context_id);
}

pub(crate) fn unbind_transient_browsing_context(
    shared: &SharedStateHandle,
    transient_browsing_context_id: TransientBrowsingContextId,
) -> Option<HostWindowId> {
    let mut guard = shared.lock().expect("shared state lock poisoned");
    let window_id = guard
        .transient_to_window
        .remove(&transient_browsing_context_id)?;
    guard.window_to_transient.remove(&window_id);
    Some(window_id)
}

pub(crate) fn window_id_for_transient_browsing_context(
    shared: &SharedStateHandle,
    transient_browsing_context_id: TransientBrowsingContextId,
) -> Option<HostWindowId> {
    shared
        .lock()
        .expect("shared state lock poisoned")
        .transient_to_window
        .get(&transient_browsing_context_id)
        .copied()
}

pub(crate) fn transient_browsing_context_id_for_window(
    shared: &SharedStateHandle,
    window_id: HostWindowId,
) -> Option<TransientBrowsingContextId> {
    shared
        .lock()
        .expect("shared state lock poisoned")
        .window_to_transient
        .get(&window_id)
        .copied()
}

pub(crate) fn set_primary_browsing_context_id(
    shared: &SharedStateHandle,
    browsing_context_id: Option<BrowsingContextId>,
) {
    shared
        .lock()
        .expect("shared state lock poisoned")
        .primary_browsing_context_id = browsing_context_id;
}

pub(crate) fn primary_browsing_context_id(shared: &SharedStateHandle) -> Option<BrowsingContextId> {
    shared
        .lock()
        .expect("shared state lock poisoned")
        .primary_browsing_context_id
}

pub(crate) fn set_toolbar_browsing_context_id(
    shared: &SharedStateHandle,
    browsing_context_id: Option<BrowsingContextId>,
) {
    shared
        .lock()
        .expect("shared state lock poisoned")
        .toolbar_browsing_context_id = browsing_context_id;
}

pub(crate) fn toolbar_browsing_context_id(shared: &SharedStateHandle) -> Option<BrowsingContextId> {
    shared
        .lock()
        .expect("shared state lock poisoned")
        .toolbar_browsing_context_id
}

pub(crate) fn set_overlay_browsing_context_id(
    shared: &SharedStateHandle,
    browsing_context_id: Option<BrowsingContextId>,
) {
    shared
        .lock()
        .expect("shared state lock poisoned")
        .overlay_browsing_context_id = browsing_context_id;
}

pub(crate) fn overlay_browsing_context_id(shared: &SharedStateHandle) -> Option<BrowsingContextId> {
    shared
        .lock()
        .expect("shared state lock poisoned")
        .overlay_browsing_context_id
}

pub(crate) fn set_devtools_browsing_context_id(
    shared: &SharedStateHandle,
    browsing_context_id: Option<BrowsingContextId>,
) {
    shared
        .lock()
        .expect("shared state lock poisoned")
        .devtools_browsing_context_id = browsing_context_id;
}

pub(crate) fn devtools_browsing_context_id(
    shared: &SharedStateHandle,
) -> Option<BrowsingContextId> {
    shared
        .lock()
        .expect("shared state lock poisoned")
        .devtools_browsing_context_id
}

pub(crate) fn allocate_browsing_context_request_id(shared: &SharedStateHandle) -> u64 {
    let mut guard = shared.lock().expect("shared state lock poisoned");
    let request_id = guard.next_browsing_context_request_id.max(10_000);
    guard.next_browsing_context_request_id = request_id.saturating_add(1);
    request_id
}

pub(crate) fn take_pending_window_open_request(
    shared: &SharedStateHandle,
    request_id: u64,
) -> Option<HostWindowId> {
    shared
        .lock()
        .expect("shared state lock poisoned")
        .pending_window_open_requests
        .remove(&request_id)
}

pub(crate) fn register_pending_window_browsing_context_create(
    shared: &SharedStateHandle,
    request_id: u64,
    pending: PendingWindowBrowsingContextCreate,
) {
    shared
        .lock()
        .expect("shared state lock poisoned")
        .pending_window_browsing_context_creates
        .insert(request_id, pending);
}

pub(crate) fn take_pending_window_browsing_context_create(
    shared: &SharedStateHandle,
    request_id: u64,
) -> Option<PendingWindowBrowsingContextCreate> {
    shared
        .lock()
        .expect("shared state lock poisoned")
        .pending_window_browsing_context_creates
        .remove(&request_id)
}

pub(crate) fn set_window_page_browsing_context(
    shared: &SharedStateHandle,
    window_id: HostWindowId,
    browsing_context_id: Option<BrowsingContextId>,
) {
    let mut guard = shared.lock().expect("shared state lock poisoned");
    match browsing_context_id {
        Some(browsing_context_id) => {
            guard
                .window_to_page_browsing_context
                .insert(window_id, browsing_context_id);
        }
        None => {
            guard.window_to_page_browsing_context.remove(&window_id);
        }
    }
}

pub(crate) fn page_browsing_context_id_for_window(
    shared: &SharedStateHandle,
    window_id: HostWindowId,
) -> Option<BrowsingContextId> {
    shared
        .lock()
        .expect("shared state lock poisoned")
        .window_to_page_browsing_context
        .get(&window_id)
        .copied()
}

pub(crate) fn window_id_for_page_browsing_context(
    shared: &SharedStateHandle,
    browsing_context_id: BrowsingContextId,
) -> Option<HostWindowId> {
    shared
        .lock()
        .expect("shared state lock poisoned")
        .window_to_page_browsing_context
        .iter()
        .find_map(|(window_id, candidate)| {
            (*candidate == browsing_context_id).then_some(*window_id)
        })
}

pub(crate) fn set_window_toolbar_browsing_context(
    shared: &SharedStateHandle,
    window_id: HostWindowId,
    browsing_context_id: Option<BrowsingContextId>,
) {
    let mut guard = shared.lock().expect("shared state lock poisoned");
    match browsing_context_id {
        Some(browsing_context_id) => {
            guard
                .window_to_toolbar_browsing_context
                .insert(window_id, browsing_context_id);
        }
        None => {
            guard.window_to_toolbar_browsing_context.remove(&window_id);
        }
    }
}

pub(crate) fn toolbar_browsing_context_id_for_window(
    shared: &SharedStateHandle,
    window_id: HostWindowId,
) -> Option<BrowsingContextId> {
    shared
        .lock()
        .expect("shared state lock poisoned")
        .window_to_toolbar_browsing_context
        .get(&window_id)
        .copied()
}

pub(crate) fn window_id_for_toolbar_browsing_context(
    shared: &SharedStateHandle,
    browsing_context_id: BrowsingContextId,
) -> Option<HostWindowId> {
    shared
        .lock()
        .expect("shared state lock poisoned")
        .window_to_toolbar_browsing_context
        .iter()
        .find_map(|(window_id, candidate)| {
            (*candidate == browsing_context_id).then_some(*window_id)
        })
}

pub(crate) fn page_browsing_context_id_for_toolbar_browsing_context(
    shared: &SharedStateHandle,
    toolbar_browsing_context_id: BrowsingContextId,
) -> Option<BrowsingContextId> {
    let guard = shared.lock().expect("shared state lock poisoned");
    let window_id =
        guard
            .window_to_toolbar_browsing_context
            .iter()
            .find_map(|(window_id, candidate)| {
                (*candidate == toolbar_browsing_context_id).then_some(*window_id)
            })?;
    guard
        .window_to_page_browsing_context
        .get(&window_id)
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_shared() -> SharedStateHandle {
        Arc::new(Mutex::new(SharedState::default()))
    }

    #[test]
    fn resolve_page_for_toolbar_in_single_window() {
        let shared = make_shared();
        let window_id = HostWindowId::new(10);
        let toolbar = BrowsingContextId::new(11);
        let page = BrowsingContextId::new(12);

        set_window_toolbar_browsing_context(&shared, window_id, Some(toolbar));
        set_window_page_browsing_context(&shared, window_id, Some(page));

        let resolved = page_browsing_context_id_for_toolbar_browsing_context(&shared, toolbar);
        assert_eq!(resolved, Some(page));
    }

    #[test]
    fn resolve_page_for_toolbar_in_multi_window() {
        let shared = make_shared();
        let window_a = HostWindowId::new(20);
        let window_b = HostWindowId::new(30);
        let toolbar_a = BrowsingContextId::new(21);
        let toolbar_b = BrowsingContextId::new(31);
        let page_a = BrowsingContextId::new(22);
        let page_b = BrowsingContextId::new(32);

        set_window_toolbar_browsing_context(&shared, window_a, Some(toolbar_a));
        set_window_page_browsing_context(&shared, window_a, Some(page_a));
        set_window_toolbar_browsing_context(&shared, window_b, Some(toolbar_b));
        set_window_page_browsing_context(&shared, window_b, Some(page_b));

        let resolved = page_browsing_context_id_for_toolbar_browsing_context(&shared, toolbar_b);
        assert_eq!(resolved, Some(page_b));
    }

    #[test]
    fn resolve_page_for_toolbar_returns_none_when_page_missing() {
        let shared = make_shared();
        let window_id = HostWindowId::new(40);
        let toolbar = BrowsingContextId::new(41);
        set_window_toolbar_browsing_context(&shared, window_id, Some(toolbar));

        let resolved = page_browsing_context_id_for_toolbar_browsing_context(&shared, toolbar);
        assert_eq!(resolved, None);
    }
}
