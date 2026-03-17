#![allow(dead_code)]

use std::os::raw::c_char;

#[repr(C)]
pub struct CbfBridgeClientHandle {
    _private: [u8; 0],
}

pub const CBF_EVENT_NONE: u8 = 0;
pub const CBF_EVENT_SURFACE_HANDLE_UPDATED: u8 = 1;
pub const CBF_EVENT_WEB_PAGE_CREATED: u8 = 2;
pub const CBF_EVENT_IME_BOUNDS_UPDATED: u8 = 3;
pub const CBF_EVENT_SHUTDOWN_BLOCKED: u8 = 4;
pub const CBF_EVENT_SHUTDOWN_PROCEEDING: u8 = 5;
pub const CBF_EVENT_SHUTDOWN_CANCELLED: u8 = 6;
pub const CBF_EVENT_CONTEXT_MENU_REQUESTED: u8 = 7;
pub const CBF_EVENT_NEW_WEB_PAGE_REQUESTED: u8 = 8;
pub const CBF_EVENT_BEFOREUNLOAD_DIALOG_REQUESTED: u8 = 9;
pub const CBF_EVENT_WEB_PAGE_CLOSED: u8 = 10;
pub const CBF_EVENT_NAVIGATION_STATE_CHANGED: u8 = 11;
pub const CBF_EVENT_WEB_PAGE_RESIZE_ACKNOWLEDGED: u8 = 12;
pub const CBF_EVENT_CURSOR_CHANGED: u8 = 13;
pub const CBF_EVENT_WEB_PAGE_DOM_HTML_READ: u8 = 14;
pub const CBF_EVENT_DRAG_START_REQUESTED: u8 = 15;
pub const CBF_EVENT_TITLE_UPDATED: u8 = 16;
pub const CBF_EVENT_FAVICON_URL_UPDATED: u8 = 17;
pub const CBF_EVENT_DEVTOOLS_OPENED: u8 = 18;
pub const CBF_EVENT_EXTENSIONS_LISTED: u8 = 19;
pub const CBF_EVENT_AUXILIARY_WINDOW_OPEN_REQUESTED: u8 = 20;
pub const CBF_EVENT_AUXILIARY_WINDOW_RESOLVED: u8 = 21;
pub const CBF_EVENT_EXTENSION_RUNTIME_WARNING: u8 = 22;
pub const CBF_EVENT_AUXILIARY_WINDOW_OPENED: u8 = 23;
pub const CBF_EVENT_AUXILIARY_WINDOW_CLOSED: u8 = 24;
pub const CBF_EVENT_TAB_OPEN_REQUESTED: u8 = 25;
pub const CBF_EVENT_TAB_OPEN_RESOLVED: u8 = 26;
pub const CBF_EVENT_PROMPT_UI_REQUESTED: u8 = 27;
pub const CBF_EVENT_PROMPT_UI_RESOLVED: u8 = 28;
pub const CBF_EVENT_DOWNLOAD_CREATED: u8 = 29;
pub const CBF_EVENT_DOWNLOAD_UPDATED: u8 = 30;
pub const CBF_EVENT_DOWNLOAD_COMPLETED: u8 = 31;
pub const CBF_EVENT_EXTENSION_POPUP_OPENED: u8 = 32;
pub const CBF_EVENT_EXTENSION_POPUP_SURFACE_HANDLE_UPDATED: u8 = 33;
pub const CBF_EVENT_EXTENSION_POPUP_PREFERRED_SIZE_CHANGED: u8 = 34;
pub const CBF_EVENT_EXTENSION_POPUP_CLOSED: u8 = 35;
pub const CBF_EVENT_EXTENSION_POPUP_IME_BOUNDS_UPDATED: u8 = 36;
pub const CBF_EVENT_EXTENSION_POPUP_CONTEXT_MENU_REQUESTED: u8 = 37;
pub const CBF_EVENT_EXTENSION_POPUP_CURSOR_CHANGED: u8 = 38;
pub const CBF_EVENT_EXTENSION_POPUP_TITLE_UPDATED: u8 = 39;
pub const CBF_EVENT_EXTENSION_POPUP_JAVASCRIPT_DIALOG_REQUESTED: u8 = 40;
pub const CBF_EVENT_EXTENSION_POPUP_CLOSE_REQUESTED: u8 = 41;
pub const CBF_EVENT_EXTENSION_POPUP_RENDER_PROCESS_GONE: u8 = 42;
pub const CBF_EVENT_JAVASCRIPT_DIALOG_REQUESTED: u8 = 43;
pub const CBF_EVENT_CHOICE_MENU_REQUESTED: u8 = 44;
pub const CBF_EVENT_EXTENSION_POPUP_CHOICE_MENU_REQUESTED: u8 = 45;

pub const CBF_BRIDGE_EVENT_WAIT_STATUS_EVENT_AVAILABLE: i32 = 0;
pub const CBF_BRIDGE_EVENT_WAIT_STATUS_TIMED_OUT: i32 = 1;
pub const CBF_BRIDGE_EVENT_WAIT_STATUS_DISCONNECTED: i32 = 2;
pub const CBF_BRIDGE_EVENT_WAIT_STATUS_CLOSED: i32 = 3;

pub const CBF_EVENT_PROMPT_UI_OPEN_REQUESTED: u8 = CBF_EVENT_PROMPT_UI_REQUESTED;
pub const CBF_EVENT_PROMPT_UI_OPENED: u8 = CBF_EVENT_AUXILIARY_WINDOW_OPENED;
pub const CBF_EVENT_PROMPT_UI_CLOSED: u8 = CBF_EVENT_AUXILIARY_WINDOW_CLOSED;
pub const CBF_EVENT_TAB_CREATED: u8 = CBF_EVENT_WEB_PAGE_CREATED;
pub const CBF_EVENT_NEW_TAB_REQUESTED: u8 = CBF_EVENT_NEW_WEB_PAGE_REQUESTED;
pub const CBF_EVENT_TAB_CLOSED: u8 = CBF_EVENT_WEB_PAGE_CLOSED;
pub const CBF_EVENT_TAB_RESIZE_ACKNOWLEDGED: u8 = CBF_EVENT_WEB_PAGE_RESIZE_ACKNOWLEDGED;
pub const CBF_EVENT_TAB_DOM_HTML_READ: u8 = CBF_EVENT_WEB_PAGE_DOM_HTML_READ;

pub const CBF_PROMPT_UI_REQUESTED: u8 = CBF_EVENT_PROMPT_UI_REQUESTED;
pub const CBF_PROMPT_UI_RESOLVED: u8 = CBF_EVENT_PROMPT_UI_RESOLVED;

pub const CBF_PROMPT_UI_EXTENSION_INSTALL_RESULT_ACCEPTED: u8 = 0;
pub const CBF_PROMPT_UI_EXTENSION_INSTALL_RESULT_ACCEPTED_WITH_WITHHELD_PERMISSIONS: u8 = 1;
pub const CBF_PROMPT_UI_EXTENSION_INSTALL_RESULT_USER_CANCELED: u8 = 2;
pub const CBF_PROMPT_UI_EXTENSION_INSTALL_RESULT_ABORTED: u8 = 3;
pub const CBF_PROMPT_UI_EXTENSION_UNINSTALL_RESULT_ACCEPTED: u8 = 0;
pub const CBF_PROMPT_UI_EXTENSION_UNINSTALL_RESULT_USER_CANCELED: u8 = 1;
pub const CBF_PROMPT_UI_EXTENSION_UNINSTALL_RESULT_ABORTED: u8 = 2;
pub const CBF_PROMPT_UI_EXTENSION_UNINSTALL_RESULT_FAILED: u8 = 3;
pub const CBF_EXTENSION_INSTALL_PROMPT_RESULT_ACCEPTED: u8 =
    CBF_PROMPT_UI_EXTENSION_INSTALL_RESULT_ACCEPTED;
pub const CBF_EXTENSION_INSTALL_PROMPT_RESULT_ACCEPTED_WITH_WITHHELD_PERMISSIONS: u8 =
    CBF_PROMPT_UI_EXTENSION_INSTALL_RESULT_ACCEPTED_WITH_WITHHELD_PERMISSIONS;
pub const CBF_EXTENSION_INSTALL_PROMPT_RESULT_USER_CANCELED: u8 =
    CBF_PROMPT_UI_EXTENSION_INSTALL_RESULT_USER_CANCELED;
pub const CBF_EXTENSION_INSTALL_PROMPT_RESULT_ABORTED: u8 =
    CBF_PROMPT_UI_EXTENSION_INSTALL_RESULT_ABORTED;
pub const CBF_EXTENSION_UNINSTALL_PROMPT_RESULT_ACCEPTED: u8 =
    CBF_PROMPT_UI_EXTENSION_UNINSTALL_RESULT_ACCEPTED;
pub const CBF_EXTENSION_UNINSTALL_PROMPT_RESULT_USER_CANCELED: u8 =
    CBF_PROMPT_UI_EXTENSION_UNINSTALL_RESULT_USER_CANCELED;
pub const CBF_EXTENSION_UNINSTALL_PROMPT_RESULT_ABORTED: u8 =
    CBF_PROMPT_UI_EXTENSION_UNINSTALL_RESULT_ABORTED;
pub const CBF_EXTENSION_UNINSTALL_PROMPT_RESULT_FAILED: u8 =
    CBF_PROMPT_UI_EXTENSION_UNINSTALL_RESULT_FAILED;

pub const CBF_AUXILIARY_WINDOW_KIND_UNKNOWN: u8 = 0;
pub const CBF_AUXILIARY_WINDOW_KIND_EXTENSION_INSTALL_PROMPT: u8 = 1;
pub const CBF_AUXILIARY_WINDOW_KIND_EXTENSION_UNINSTALL_PROMPT: u8 = 2;
pub const CBF_AUXILIARY_WINDOW_KIND_PRINT_PREVIEW_DIALOG: u8 = 3;

pub const CBF_PROMPT_UI_KIND_UNKNOWN: u8 = 0;
pub const CBF_PROMPT_UI_KIND_PERMISSION_PROMPT: u8 = 1;
pub const CBF_PROMPT_UI_KIND_DOWNLOAD_PROMPT: u8 = 2;
// Legacy prompt-ui kind values kept for compatibility with older bridge events.
pub const CBF_PROMPT_UI_KIND_EXTENSION_INSTALL_PROMPT: u8 = 3;
pub const CBF_PROMPT_UI_KIND_EXTENSION_UNINSTALL_PROMPT: u8 = 4;
pub const CBF_PROMPT_UI_KIND_PRINT_PREVIEW_DIALOG: u8 = 5;

pub const CBF_PROMPT_UI_CLOSE_REASON_UNKNOWN: u8 = 0;
pub const CBF_PROMPT_UI_CLOSE_REASON_USER_CANCELED: u8 = 1;
pub const CBF_PROMPT_UI_CLOSE_REASON_HOST_FORCED: u8 = 2;
pub const CBF_PROMPT_UI_CLOSE_REASON_SYSTEM_DISMISSED: u8 = 3;
pub const CBF_AUXILIARY_WINDOW_CLOSE_REASON_UNKNOWN: u8 = CBF_PROMPT_UI_CLOSE_REASON_UNKNOWN;
pub const CBF_AUXILIARY_WINDOW_CLOSE_REASON_USER_CANCELED: u8 =
    CBF_PROMPT_UI_CLOSE_REASON_USER_CANCELED;
pub const CBF_AUXILIARY_WINDOW_CLOSE_REASON_HOST_FORCED: u8 =
    CBF_PROMPT_UI_CLOSE_REASON_HOST_FORCED;
pub const CBF_AUXILIARY_WINDOW_CLOSE_REASON_SYSTEM_DISMISSED: u8 =
    CBF_PROMPT_UI_CLOSE_REASON_SYSTEM_DISMISSED;

pub const CBF_PROMPT_UI_PERMISSION_TYPE_UNKNOWN: u8 = 0;
pub const CBF_PROMPT_UI_PERMISSION_TYPE_GEOLOCATION: u8 = 1;
pub const CBF_PROMPT_UI_PERMISSION_TYPE_NOTIFICATIONS: u8 = 2;
pub const CBF_PROMPT_UI_PERMISSION_TYPE_AUDIO_CAPTURE: u8 = 3;
pub const CBF_PROMPT_UI_PERMISSION_TYPE_VIDEO_CAPTURE: u8 = 4;

pub const CBF_PROMPT_UI_RESOLUTION_RESULT_UNKNOWN: u8 = 0;
pub const CBF_PROMPT_UI_RESOLUTION_RESULT_ALLOWED: u8 = 1;
pub const CBF_PROMPT_UI_RESOLUTION_RESULT_DENIED: u8 = 2;
pub const CBF_PROMPT_UI_RESOLUTION_RESULT_ABORTED: u8 = 3;

pub const CBF_PROMPT_UI_DIALOG_RESULT_UNKNOWN: u8 = 0;
pub const CBF_PROMPT_UI_DIALOG_RESULT_PROCEEDED: u8 = 1;
pub const CBF_PROMPT_UI_DIALOG_RESULT_CANCELED: u8 = 2;
pub const CBF_PROMPT_UI_DIALOG_RESULT_ABORTED: u8 = 3;

pub const CBF_DOWNLOAD_PROMPT_RESULT_ALLOWED: u8 = 1;
pub const CBF_DOWNLOAD_PROMPT_RESULT_DENIED: u8 = 2;
pub const CBF_DOWNLOAD_PROMPT_RESULT_ABORTED: u8 = 3;

pub const CBF_DOWNLOAD_PROMPT_REASON_NONE: u8 = 0;
pub const CBF_DOWNLOAD_PROMPT_REASON_UNEXPECTED: u8 = 1;
pub const CBF_DOWNLOAD_PROMPT_REASON_SAVE_AS: u8 = 2;
pub const CBF_DOWNLOAD_PROMPT_REASON_PREFERENCE: u8 = 3;
pub const CBF_DOWNLOAD_PROMPT_REASON_NAME_TOO_LONG: u8 = 4;
pub const CBF_DOWNLOAD_PROMPT_REASON_TARGET_CONFLICT: u8 = 5;
pub const CBF_DOWNLOAD_PROMPT_REASON_TARGET_PATH_NOT_WRITEABLE: u8 = 6;
pub const CBF_DOWNLOAD_PROMPT_REASON_TARGET_NO_SPACE: u8 = 7;
pub const CBF_DOWNLOAD_PROMPT_REASON_DLP_BLOCKED: u8 = 8;
pub const CBF_DOWNLOAD_PROMPT_REASON_UNKNOWN: u8 = 255;

pub const CBF_ICON_DATA_KIND_NONE: u8 = 0;
pub const CBF_ICON_DATA_KIND_URL: u8 = 1;
pub const CBF_ICON_DATA_KIND_PNG: u8 = 2;
pub const CBF_ICON_DATA_KIND_BINARY: u8 = 3;

pub const CBF_DOWNLOAD_STATE_UNKNOWN: u8 = 0;
pub const CBF_DOWNLOAD_STATE_IN_PROGRESS: u8 = 1;
pub const CBF_DOWNLOAD_STATE_PAUSED: u8 = 2;
pub const CBF_DOWNLOAD_STATE_COMPLETED: u8 = 3;
pub const CBF_DOWNLOAD_STATE_CANCELLED: u8 = 4;
pub const CBF_DOWNLOAD_STATE_INTERRUPTED: u8 = 5;

pub const CBF_DOWNLOAD_OUTCOME_UNKNOWN: u8 = 0;
pub const CBF_DOWNLOAD_OUTCOME_SUCCEEDED: u8 = 1;
pub const CBF_DOWNLOAD_OUTCOME_CANCELLED: u8 = 2;
pub const CBF_DOWNLOAD_OUTCOME_INTERRUPTED: u8 = 3;

pub const CBF_TAB_OPEN_HINT_UNKNOWN: u8 = 0;
pub const CBF_TAB_OPEN_HINT_CURRENT_CONTEXT: u8 = 1;
pub const CBF_TAB_OPEN_HINT_NEW_FOREGROUND_CONTEXT: u8 = 2;
pub const CBF_TAB_OPEN_HINT_NEW_BACKGROUND_CONTEXT: u8 = 3;
pub const CBF_TAB_OPEN_HINT_NEW_WINDOW: u8 = 4;
pub const CBF_TAB_OPEN_HINT_POPUP: u8 = 5;

pub const CBF_TAB_OPEN_RESPONSE_ALLOW_NEW_CONTEXT: u8 = 0;
pub const CBF_TAB_OPEN_RESPONSE_ALLOW_EXISTING_CONTEXT: u8 = 1;
pub const CBF_TAB_OPEN_RESPONSE_DENY: u8 = 2;

pub const CBF_TAB_OPEN_RESULT_OPENED_NEW_CONTEXT: u8 = 0;
pub const CBF_TAB_OPEN_RESULT_OPENED_EXISTING_CONTEXT: u8 = 1;
pub const CBF_TAB_OPEN_RESULT_DENIED: u8 = 2;
pub const CBF_TAB_OPEN_RESULT_ABORTED: u8 = 3;

pub const CBF_TAB_VISIBILITY_VISIBLE: u8 = 0;
pub const CBF_TAB_VISIBILITY_HIDDEN: u8 = 1;

pub const CBF_SURFACE_HANDLE_NONE: u8 = 0;
pub const CBF_SURFACE_HANDLE_MAC_CA_CONTEXT_ID: u8 = 1;
pub const CBF_SURFACE_HANDLE_WINDOWS_HWND: u8 = 2;

pub const CBF_KEY_EVENT_RAW_KEY_DOWN: u8 = 0;
pub const CBF_KEY_EVENT_KEY_DOWN: u8 = 1;
pub const CBF_KEY_EVENT_KEY_UP: u8 = 2;
pub const CBF_KEY_EVENT_CHAR: u8 = 3;

pub const CBF_MOUSE_EVENT_DOWN: u8 = 0;
pub const CBF_MOUSE_EVENT_UP: u8 = 1;
pub const CBF_MOUSE_EVENT_MOVE: u8 = 2;
pub const CBF_MOUSE_EVENT_ENTER: u8 = 3;
pub const CBF_MOUSE_EVENT_LEAVE: u8 = 4;

pub const CBF_MOUSE_BUTTON_NONE: u8 = 0;
pub const CBF_MOUSE_BUTTON_LEFT: u8 = 1;
pub const CBF_MOUSE_BUTTON_MIDDLE: u8 = 2;
pub const CBF_MOUSE_BUTTON_RIGHT: u8 = 3;
pub const CBF_MOUSE_BUTTON_BACK: u8 = 4;
pub const CBF_MOUSE_BUTTON_FORWARD: u8 = 5;

pub const CBF_POINTER_TYPE_UNKNOWN: u8 = 0;
pub const CBF_POINTER_TYPE_MOUSE: u8 = 1;
pub const CBF_POINTER_TYPE_PEN: u8 = 2;
pub const CBF_POINTER_TYPE_TOUCH: u8 = 3;
pub const CBF_POINTER_TYPE_ERASER: u8 = 4;

pub const CBF_SCROLL_BY_PRECISE_PIXEL: u8 = 0;
pub const CBF_SCROLL_BY_PIXEL: u8 = 1;
pub const CBF_SCROLL_BY_LINE: u8 = 2;
pub const CBF_SCROLL_BY_PAGE: u8 = 3;
pub const CBF_SCROLL_BY_DOCUMENT: u8 = 4;

pub const CBF_IME_TEXT_SPAN_TYPE_COMPOSITION: u8 = 0;
pub const CBF_IME_TEXT_SPAN_TYPE_SUGGESTION: u8 = 1;
pub const CBF_IME_TEXT_SPAN_TYPE_MISSPELLING_SUGGESTION: u8 = 2;
pub const CBF_IME_TEXT_SPAN_TYPE_AUTOCORRECT: u8 = 3;
pub const CBF_IME_TEXT_SPAN_TYPE_GRAMMAR_SUGGESTION: u8 = 4;

pub const CBF_IME_TEXT_SPAN_THICKNESS_NONE: u8 = 0;
pub const CBF_IME_TEXT_SPAN_THICKNESS_THIN: u8 = 1;
pub const CBF_IME_TEXT_SPAN_THICKNESS_THICK: u8 = 2;

pub const CBF_IME_TEXT_SPAN_UNDERLINE_STYLE_NONE: u8 = 0;
pub const CBF_IME_TEXT_SPAN_UNDERLINE_STYLE_SOLID: u8 = 1;
pub const CBF_IME_TEXT_SPAN_UNDERLINE_STYLE_DOT: u8 = 2;
pub const CBF_IME_TEXT_SPAN_UNDERLINE_STYLE_DASH: u8 = 3;
pub const CBF_IME_TEXT_SPAN_UNDERLINE_STYLE_SQUIGGLE: u8 = 4;

pub const CBF_IME_CONFIRM_DO_NOT_KEEP_SELECTION: u8 = 0;
pub const CBF_IME_CONFIRM_KEEP_SELECTION: u8 = 1;
pub const CBF_EDIT_ACTION_UNDO: u8 = 0;
pub const CBF_EDIT_ACTION_REDO: u8 = 1;
pub const CBF_EDIT_ACTION_CUT: u8 = 2;
pub const CBF_EDIT_ACTION_COPY: u8 = 3;
pub const CBF_EDIT_ACTION_PASTE: u8 = 4;
pub const CBF_EDIT_ACTION_SELECT_ALL: u8 = 5;

pub const CBF_BEFOREUNLOAD_REASON_UNKNOWN: u8 = 0;
pub const CBF_BEFOREUNLOAD_REASON_CLOSE_WEB_PAGE: u8 = 1;
pub const CBF_BEFOREUNLOAD_REASON_NAVIGATE: u8 = 2;
pub const CBF_BEFOREUNLOAD_REASON_RELOAD: u8 = 3;
pub const CBF_BEFOREUNLOAD_REASON_WINDOW_CLOSE: u8 = 4;

pub const CBF_JAVASCRIPT_DIALOG_ALERT: u8 = 0;
pub const CBF_JAVASCRIPT_DIALOG_CONFIRM: u8 = 1;
pub const CBF_JAVASCRIPT_DIALOG_PROMPT: u8 = 2;
pub const CBF_JAVASCRIPT_DIALOG_BEFOREUNLOAD: u8 = 3;
pub const CBF_BEFOREUNLOAD_REASON_CLOSE_TAB: u8 = CBF_BEFOREUNLOAD_REASON_CLOSE_WEB_PAGE;

pub const CBF_CURSOR_DEFAULT: u8 = 0;
pub const CBF_CURSOR_CROSSHAIR: u8 = 1;
pub const CBF_CURSOR_POINTER: u8 = 2;
pub const CBF_CURSOR_MOVE: u8 = 3;
pub const CBF_CURSOR_TEXT: u8 = 4;
pub const CBF_CURSOR_WAIT: u8 = 5;
pub const CBF_CURSOR_HELP: u8 = 6;
pub const CBF_CURSOR_PROGRESS: u8 = 7;
pub const CBF_CURSOR_NOT_ALLOWED: u8 = 8;
pub const CBF_CURSOR_CONTEXT_MENU: u8 = 9;
pub const CBF_CURSOR_CELL: u8 = 10;
pub const CBF_CURSOR_VERTICAL_TEXT: u8 = 11;
pub const CBF_CURSOR_ALIAS: u8 = 12;
pub const CBF_CURSOR_COPY: u8 = 13;
pub const CBF_CURSOR_NO_DROP: u8 = 14;
pub const CBF_CURSOR_GRAB: u8 = 15;
pub const CBF_CURSOR_GRABBING: u8 = 16;
pub const CBF_CURSOR_ALL_SCROLL: u8 = 17;
pub const CBF_CURSOR_ZOOM_IN: u8 = 18;
pub const CBF_CURSOR_ZOOM_OUT: u8 = 19;
pub const CBF_CURSOR_E_RESIZE: u8 = 20;
pub const CBF_CURSOR_N_RESIZE: u8 = 21;
pub const CBF_CURSOR_NE_RESIZE: u8 = 22;
pub const CBF_CURSOR_NW_RESIZE: u8 = 23;
pub const CBF_CURSOR_S_RESIZE: u8 = 24;
pub const CBF_CURSOR_SE_RESIZE: u8 = 25;
pub const CBF_CURSOR_SW_RESIZE: u8 = 26;
pub const CBF_CURSOR_W_RESIZE: u8 = 27;
pub const CBF_CURSOR_EW_RESIZE: u8 = 28;
pub const CBF_CURSOR_NS_RESIZE: u8 = 29;
pub const CBF_CURSOR_NESW_RESIZE: u8 = 30;
pub const CBF_CURSOR_NWSE_RESIZE: u8 = 31;
pub const CBF_CURSOR_COL_RESIZE: u8 = 32;
pub const CBF_CURSOR_ROW_RESIZE: u8 = 33;

pub const CBF_MENU_ITEM_COMMAND: u8 = 0;
pub const CBF_MENU_ITEM_CHECK: u8 = 1;
pub const CBF_MENU_ITEM_RADIO: u8 = 2;
pub const CBF_MENU_ITEM_SEPARATOR: u8 = 3;
pub const CBF_MENU_ITEM_BUTTON_ITEM: u8 = 4;
pub const CBF_MENU_ITEM_SUBMENU: u8 = 5;
pub const CBF_MENU_ITEM_ACTIONABLE_SUBMENU: u8 = 6;
pub const CBF_MENU_ITEM_HIGHLIGHTED: u8 = 7;
pub const CBF_MENU_ITEM_TITLE: u8 = 8;
pub const CBF_CHOICE_MENU_ITEM_OPTION: u8 = 0;
pub const CBF_CHOICE_MENU_ITEM_CHECKABLE_OPTION: u8 = 1;
pub const CBF_CHOICE_MENU_ITEM_GROUP: u8 = 2;
pub const CBF_CHOICE_MENU_ITEM_SEPARATOR: u8 = 3;
pub const CBF_CHOICE_MENU_ITEM_SUB_MENU: u8 = 4;
pub const CBF_CHOICE_MENU_TEXT_DIRECTION_LEFT_TO_RIGHT: u8 = 0;
pub const CBF_CHOICE_MENU_TEXT_DIRECTION_RIGHT_TO_LEFT: u8 = 1;

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfSurfaceHandle {
    pub kind: u8,
    pub ca_context_id: u32,
    pub win_hwnd: u64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfBridgeEvent {
    pub kind: u8,
    pub tab_id: u64,
    pub inspected_tab_id: u64,
    pub request_id: u64,
    pub extension_popup_id: u64,
    pub width: u32,
    pub height: u32,
    pub beforeunload_reason: u8,
    pub javascript_dialog_type: u8,
    pub cursor_type: u8,
    pub profile_id: *mut c_char,
    pub surface_handle: CbfSurfaceHandle,
    pub ime_bounds: CbfImeBoundsUpdate,
    pub dirty_tab_ids: CbfTabIdList,
    pub context_menu: CbfContextMenu,
    pub choice_menu: CbfChoiceMenu,
    pub target_url: *mut c_char,
    pub url: *mut c_char,
    pub is_popup: bool,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub is_loading: bool,
    pub dom_html: *mut c_char,
    pub message: *mut c_char,
    pub default_prompt_text: *mut c_char,
    pub title: *mut c_char,
    pub favicon_url: *mut c_char,
    pub crashed: bool,
    pub drag_start_request: CbfDragStartRequest,
    pub extensions: CbfExtensionInfoList,
    pub extension_id: *mut c_char,
    pub extension_name: *mut c_char,
    pub triggering_extension_name: *mut c_char,
    pub permission_names: CbfStringList,
    pub prompt_ui_extension_install_result: u8,
    pub prompt_ui_extension_install_detail: *mut c_char,
    pub prompt_ui_extension_uninstall_result: u8,
    pub prompt_ui_extension_uninstall_detail: *mut c_char,
    pub extension_runtime_warning: *mut c_char,
    pub auxiliary_window_id: u64,
    pub auxiliary_window_kind: u8,
    pub auxiliary_window_close_reason: u8,
    pub auxiliary_window_title: *mut c_char,
    pub auxiliary_window_modal: bool,
    pub prompt_ui_kind: u8,
    pub prompt_ui_has_source_tab_id: bool,
    pub prompt_ui_source_tab_id: u64,
    pub prompt_ui_can_report_abuse: bool,
    pub prompt_ui_report_abuse: bool,
    pub prompt_ui_permission: u8,
    pub prompt_ui_result: u8,
    pub prompt_ui_permission_key: *mut c_char,
    pub download_reason: u8,
    pub download_id: u64,
    pub download_has_source_tab_id: bool,
    pub download_source_tab_id: u64,
    pub download_file_name: *mut c_char,
    pub download_target_path: *mut c_char,
    pub download_suggested_path: *mut c_char,
    pub download_destination_path: *mut c_char,
    pub download_total_bytes: u64,
    pub download_has_total_bytes: bool,
    pub download_received_bytes: u64,
    pub download_can_resume: bool,
    pub download_is_paused: bool,
    pub download_state: u8,
    pub download_outcome: u8,
    pub tab_open_hint: u8,
    pub tab_open_user_gesture: bool,
    pub tab_open_has_source: bool,
    pub tab_open_source_tab_id: u64,
    pub tab_open_result_kind: u8,
    pub tab_open_has_target: bool,
    pub tab_open_target_tab_id: u64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfChoiceMenuItemList {
    pub items: *const CbfChoiceMenuItem,
    pub len: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfChoiceMenuItem {
    pub type_: u8,
    pub label: *mut c_char,
    pub tool_tip: *mut c_char,
    pub action: u32,
    pub text_direction: u8,
    pub has_text_direction_override: bool,
    pub enabled: bool,
    pub checked: bool,
    pub children: CbfChoiceMenuItemList,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfChoiceMenu {
    pub request_id: u64,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub item_font_size: f64,
    pub selected_item: i32,
    pub right_aligned: bool,
    pub allow_multiple_selection: bool,
    pub items: CbfChoiceMenuItemList,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfChoiceMenuSelectedIndices {
    pub items: *const i32,
    pub len: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CbfProfileInfo {
    pub profile_id: *mut c_char,
    pub profile_path: *mut c_char,
    pub display_name: *mut c_char,
    pub is_default: bool,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfProfileList {
    pub profiles: *mut CbfProfileInfo,
    pub len: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfCommandList {
    pub items: *const *const c_char,
    pub len: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfKeyEvent {
    pub tab_id: u64,
    pub type_: u8,
    pub modifiers: u32,
    pub windows_key_code: i32,
    pub native_key_code: i32,
    pub dom_code: *const c_char,
    pub dom_key: *const c_char,
    pub text: *const c_char,
    pub unmodified_text: *const c_char,
    pub auto_repeat: bool,
    pub is_keypad: bool,
    pub is_system_key: bool,
    pub location: i32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfMouseEvent {
    pub tab_id: u64,
    pub type_: u8,
    pub modifiers: u32,
    pub button: u8,
    pub click_count: i32,
    pub position_in_widget_x: f32,
    pub position_in_widget_y: f32,
    pub position_in_screen_x: f32,
    pub position_in_screen_y: f32,
    pub movement_x: f32,
    pub movement_y: f32,
    pub is_raw_movement_event: bool,
    pub pointer_type: u8,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfMouseWheelEvent {
    pub tab_id: u64,
    pub modifiers: u32,
    pub position_in_widget_x: f32,
    pub position_in_widget_y: f32,
    pub position_in_screen_x: f32,
    pub position_in_screen_y: f32,
    pub movement_x: f32,
    pub movement_y: f32,
    pub is_raw_movement_event: bool,
    pub delta_x: f32,
    pub delta_y: f32,
    pub wheel_ticks_x: f32,
    pub wheel_ticks_y: f32,
    pub phase: u32,
    pub momentum_phase: u32,
    pub delta_units: u8,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfDragUrlInfo {
    pub url: *mut c_char,
    pub title: *mut c_char,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfDragUrlInfoList {
    pub items: *const CbfDragUrlInfo,
    pub len: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfStringList {
    pub items: *mut *mut c_char,
    pub len: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfIconData {
    pub kind: u8,
    pub url: *mut c_char,
    pub bytes: *const u8,
    pub len: u32,
    pub media_type: *mut c_char,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfExtensionInfo {
    pub id: *mut c_char,
    pub name: *mut c_char,
    pub version: *mut c_char,
    pub enabled: bool,
    pub permission_names: CbfStringList,
    pub icon: CbfIconData,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfExtensionInfoList {
    pub items: *mut CbfExtensionInfo,
    pub len: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfStringPair {
    pub key: *mut c_char,
    pub value: *mut c_char,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfStringPairList {
    pub items: *mut CbfStringPair,
    pub len: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfDragData {
    pub text: *mut c_char,
    pub html: *mut c_char,
    pub html_base_url: *mut c_char,
    pub url_infos: CbfDragUrlInfoList,
    pub filenames: CbfStringList,
    pub file_mime_types: CbfStringList,
    pub custom_data: CbfStringPairList,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfDragImage {
    pub png_bytes: *const u8,
    pub png_len: u32,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub scale: f32,
    pub cursor_offset_x: i32,
    pub cursor_offset_y: i32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfDragStartRequest {
    pub session_id: u64,
    pub tab_id: u64,
    pub allowed_operations: u32,
    pub source_origin: *mut c_char,
    pub data: CbfDragData,
    pub image: CbfDragImage,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfDragUpdate {
    pub session_id: u64,
    pub tab_id: u64,
    pub allowed_operations: u32,
    pub modifiers: u32,
    pub position_in_widget_x: f32,
    pub position_in_widget_y: f32,
    pub position_in_screen_x: f32,
    pub position_in_screen_y: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfDragDrop {
    pub session_id: u64,
    pub tab_id: u64,
    pub modifiers: u32,
    pub position_in_widget_x: f32,
    pub position_in_widget_y: f32,
    pub position_in_screen_x: f32,
    pub position_in_screen_y: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfRectList {
    pub items: *const CbfRect,
    pub len: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfImeCompositionBounds {
    pub range_start: i32,
    pub range_end: i32,
    pub character_bounds: CbfRectList,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfTextSelectionBounds {
    pub range_start: i32,
    pub range_end: i32,
    pub caret_rect: CbfRect,
    pub first_selection_rect: CbfRect,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfImeBoundsUpdate {
    pub has_composition: bool,
    pub composition: CbfImeCompositionBounds,
    pub has_selection: bool,
    pub selection: CbfTextSelectionBounds,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfTabIdList {
    pub items: *const u64,
    pub len: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfImeTextSpan {
    pub type_: u8,
    pub start_offset: u32,
    pub end_offset: u32,
    pub underline_color: u32,
    pub thickness: u8,
    pub underline_style: u8,
    pub text_color: u32,
    pub background_color: u32,
    pub suggestion_highlight_color: u32,
    pub remove_on_finish_composing: bool,
    pub interim_char_selection: bool,
    pub should_hide_suggestion_menu: bool,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfImeTextSpanList {
    pub items: *const CbfImeTextSpan,
    pub len: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfImeComposition {
    pub tab_id: u64,
    pub text: *const c_char,
    pub selection_start: i32,
    pub selection_end: i32,
    pub replacement_range_start: i32,
    pub replacement_range_end: i32,
    pub spans: CbfImeTextSpanList,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfImeCommitText {
    pub tab_id: u64,
    pub text: *const c_char,
    pub relative_caret_position: i32,
    pub replacement_range_start: i32,
    pub replacement_range_end: i32,
    pub spans: CbfImeTextSpanList,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfContextMenuIcon {
    pub png_bytes: *const u8,
    pub len: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfContextMenuItemList {
    pub items: *const CbfContextMenuItem,
    pub len: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfContextMenuItem {
    pub type_: u8,
    pub command_id: i32,
    pub label: *mut c_char,
    pub secondary_label: *mut c_char,
    pub minor_text: *mut c_char,
    pub accessible_name: *mut c_char,
    pub enabled: bool,
    pub visible: bool,
    pub checked: bool,
    pub group_id: i32,
    pub is_new_feature: bool,
    pub is_alerted: bool,
    pub may_have_mnemonics: bool,
    pub has_accelerator: bool,
    pub accelerator_key_equivalent: *mut c_char,
    pub accelerator_modifier_mask: u32,
    pub icon: CbfContextMenuIcon,
    pub minor_icon: CbfContextMenuIcon,
    pub submenu: CbfContextMenuItemList,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CbfContextMenu {
    pub menu_id: u64,
    pub x: i32,
    pub y: i32,
    pub source_type: u32,
    pub items: CbfContextMenuItemList,
}

unsafe extern "C" {
    pub fn cbf_bridge_client_create() -> *mut CbfBridgeClientHandle;
    pub fn cbf_bridge_client_destroy(client: *mut CbfBridgeClientHandle);
    pub fn cbf_bridge_init();
    pub fn cbf_bridge_prepare_channel(out_switch_arg: *mut c_char, out_arg_len: i32) -> i32;
    pub fn cbf_bridge_pass_child_pid(child_pid: i64);
    pub fn cbf_bridge_client_connect_inherited(client: *mut CbfBridgeClientHandle) -> bool;
    pub fn cbf_bridge_client_authenticate(
        client: *mut CbfBridgeClientHandle,
        token: *const c_char,
    ) -> bool;
    pub fn cbf_bridge_client_wait_for_event(
        client: *mut CbfBridgeClientHandle,
        timeout_ms: i64,
    ) -> i32;
    pub fn cbf_bridge_client_poll_event(
        client: *mut CbfBridgeClientHandle,
        out_event: *mut CbfBridgeEvent,
    ) -> bool;
    pub fn cbf_bridge_event_free(event: *mut CbfBridgeEvent);
    pub fn cbf_bridge_client_get_profiles(
        client: *mut CbfBridgeClientHandle,
        out_list: *mut CbfProfileList,
    ) -> bool;
    pub fn cbf_bridge_profile_list_free(list: *mut CbfProfileList);
    pub fn cbf_bridge_client_list_extensions(
        client: *mut CbfBridgeClientHandle,
        profile_id: *const c_char,
        out_list: *mut CbfExtensionInfoList,
    ) -> bool;
    pub fn cbf_bridge_extension_list_free(list: *mut CbfExtensionInfoList);
    pub fn cbf_bridge_client_create_web_page(
        client: *mut CbfBridgeClientHandle,
        request_id: u64,
        initial_url: *const c_char,
        profile_id: *const c_char,
    ) -> bool;
    pub fn cbf_bridge_client_create_tab(
        client: *mut CbfBridgeClientHandle,
        request_id: u64,
        initial_url: *const c_char,
        profile_id: *const c_char,
    ) -> bool;
    pub fn cbf_bridge_client_request_close_web_page(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
    ) -> bool;
    pub fn cbf_bridge_client_request_close_tab(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
    ) -> bool;
    pub fn cbf_bridge_client_set_web_page_size(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
        width: u32,
        height: u32,
    ) -> bool;
    pub fn cbf_bridge_client_set_tab_size(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
        width: u32,
        height: u32,
    ) -> bool;
    pub fn cbf_bridge_client_set_extension_popup_size(
        client: *mut CbfBridgeClientHandle,
        popup_id: u64,
        width: u32,
        height: u32,
    ) -> bool;
    pub fn cbf_bridge_client_set_web_page_focus(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
        focused: bool,
    ) -> bool;
    pub fn cbf_bridge_client_set_tab_focus(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
        focused: bool,
    ) -> bool;
    pub fn cbf_bridge_client_set_tab_visibility(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
        visibility: u8,
    ) -> bool;
    pub fn cbf_bridge_client_set_extension_popup_focus(
        client: *mut CbfBridgeClientHandle,
        popup_id: u64,
        focused: bool,
    ) -> bool;
    pub fn cbf_bridge_client_send_key_event(
        client: *mut CbfBridgeClientHandle,
        event: *const CbfKeyEvent,
        commands: *const CbfCommandList,
    ) -> bool;
    pub fn cbf_bridge_client_send_extension_popup_key_event(
        client: *mut CbfBridgeClientHandle,
        popup_id: u64,
        event: *const CbfKeyEvent,
        commands: *const CbfCommandList,
    ) -> bool;
    pub fn cbf_bridge_client_send_mouse_event(
        client: *mut CbfBridgeClientHandle,
        event: *const CbfMouseEvent,
    ) -> bool;
    pub fn cbf_bridge_client_send_extension_popup_mouse_event(
        client: *mut CbfBridgeClientHandle,
        popup_id: u64,
        event: *const CbfMouseEvent,
    ) -> bool;
    pub fn cbf_bridge_client_send_mouse_wheel_event(
        client: *mut CbfBridgeClientHandle,
        event: *const CbfMouseWheelEvent,
    ) -> bool;
    pub fn cbf_bridge_client_send_extension_popup_mouse_wheel_event(
        client: *mut CbfBridgeClientHandle,
        popup_id: u64,
        event: *const CbfMouseWheelEvent,
    ) -> bool;
    pub fn cbf_bridge_client_send_drag_update(
        client: *mut CbfBridgeClientHandle,
        update: *const CbfDragUpdate,
    ) -> bool;
    pub fn cbf_bridge_client_send_drag_drop(
        client: *mut CbfBridgeClientHandle,
        drop: *const CbfDragDrop,
    ) -> bool;
    pub fn cbf_bridge_client_send_drag_cancel(
        client: *mut CbfBridgeClientHandle,
        session_id: u64,
        tab_id: u64,
    ) -> bool;
    pub fn cbf_bridge_convert_nsevent(
        nsevent: *mut std::ffi::c_void,
        tab_id: u64,
        out_event: *mut CbfKeyEvent,
    );
    pub fn cbf_bridge_free_converted_key_event(event: *mut CbfKeyEvent);
    pub fn cbf_bridge_convert_nsevent_to_mouse_event(
        nsevent: *mut std::ffi::c_void,
        nsview: *mut std::ffi::c_void,
        tab_id: u64,
        pointer_type: u8,
        unaccelerated_movement: bool,
        out_event: *mut CbfMouseEvent,
    );
    pub fn cbf_bridge_convert_nsevent_to_mouse_wheel_event(
        nsevent: *mut std::ffi::c_void,
        nsview: *mut std::ffi::c_void,
        tab_id: u64,
        out_event: *mut CbfMouseWheelEvent,
    );
    pub fn cbf_bridge_convert_nspasteboard_to_drag_data(
        nspasteboard: *mut std::ffi::c_void,
        out_data: *mut CbfDragData,
    );
    pub fn cbf_bridge_free_converted_drag_data(data: *mut CbfDragData);
    pub fn cbf_bridge_client_set_composition(
        client: *mut CbfBridgeClientHandle,
        composition: *const CbfImeComposition,
    ) -> bool;
    pub fn cbf_bridge_client_set_extension_popup_composition(
        client: *mut CbfBridgeClientHandle,
        popup_id: u64,
        composition: *const CbfImeComposition,
    ) -> bool;
    pub fn cbf_bridge_client_commit_text(
        client: *mut CbfBridgeClientHandle,
        commit: *const CbfImeCommitText,
    ) -> bool;
    pub fn cbf_bridge_client_commit_extension_popup_text(
        client: *mut CbfBridgeClientHandle,
        popup_id: u64,
        commit: *const CbfImeCommitText,
    ) -> bool;
    pub fn cbf_bridge_client_finish_composing_text(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
        behavior: u8,
    ) -> bool;
    pub fn cbf_bridge_client_finish_extension_popup_composing_text(
        client: *mut CbfBridgeClientHandle,
        popup_id: u64,
        behavior: u8,
    ) -> bool;
    pub fn cbf_bridge_client_execute_edit_action(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
        action: u8,
    ) -> bool;
    pub fn cbf_bridge_client_execute_extension_popup_edit_action(
        client: *mut CbfBridgeClientHandle,
        popup_id: u64,
        action: u8,
    ) -> bool;
    pub fn cbf_bridge_client_execute_context_menu_command(
        client: *mut CbfBridgeClientHandle,
        menu_id: u64,
        command_id: i32,
        event_flags: i32,
    ) -> bool;
    pub fn cbf_bridge_client_dismiss_context_menu(
        client: *mut CbfBridgeClientHandle,
        menu_id: u64,
    ) -> bool;
    pub fn cbf_bridge_client_accept_choice_menu_selection(
        client: *mut CbfBridgeClientHandle,
        request_id: u64,
        indices: *const CbfChoiceMenuSelectedIndices,
    ) -> bool;
    pub fn cbf_bridge_client_dismiss_choice_menu(
        client: *mut CbfBridgeClientHandle,
        request_id: u64,
    ) -> bool;
    pub fn cbf_bridge_client_confirm_beforeunload(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
        request_id: u64,
        proceed: bool,
    ) -> bool;
    pub fn cbf_bridge_client_respond_javascript_dialog(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
        request_id: u64,
        accept: bool,
        prompt_text: *const c_char,
    ) -> bool;
    pub fn cbf_bridge_client_respond_extension_popup_javascript_dialog(
        client: *mut CbfBridgeClientHandle,
        popup_id: u64,
        request_id: u64,
        accept: bool,
        prompt_text: *const c_char,
    ) -> bool;
    pub fn cbf_bridge_client_navigate(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
        url: *const c_char,
    ) -> bool;
    pub fn cbf_bridge_client_go_back(client: *mut CbfBridgeClientHandle, tab_id: u64) -> bool;
    pub fn cbf_bridge_client_go_forward(client: *mut CbfBridgeClientHandle, tab_id: u64) -> bool;
    pub fn cbf_bridge_client_reload(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
        ignore_cache: bool,
    ) -> bool;
    pub fn cbf_bridge_client_print_preview(client: *mut CbfBridgeClientHandle, tab_id: u64)
    -> bool;
    pub fn cbf_bridge_client_activate_extension_action(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
        extension_id: *const c_char,
    ) -> bool;
    pub fn cbf_bridge_client_close_extension_popup(
        client: *mut CbfBridgeClientHandle,
        popup_id: u64,
    ) -> bool;
    pub fn cbf_bridge_client_open_dev_tools(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
    ) -> bool;
    pub fn cbf_bridge_client_inspect_element(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
        x: i32,
        y: i32,
    ) -> bool;
    pub fn cbf_bridge_client_get_web_page_dom_html(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
        request_id: u64,
    ) -> bool;
    pub fn cbf_bridge_client_get_tab_dom_html(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
        request_id: u64,
    ) -> bool;
    pub fn cbf_bridge_client_open_default_prompt_ui(
        client: *mut CbfBridgeClientHandle,
        profile_id: *const c_char,
        request_id: u64,
    ) -> bool;
    pub fn cbf_bridge_client_respond_prompt_ui(
        client: *mut CbfBridgeClientHandle,
        profile_id: *const c_char,
        request_id: u64,
        prompt_ui_kind: u8,
        proceed: bool,
        report_abuse: bool,
        destination_path: *const c_char,
    ) -> bool;
    pub fn cbf_bridge_client_respond_prompt_ui_for_tab(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
        request_id: u64,
        prompt_ui_kind: u8,
        proceed: bool,
        report_abuse: bool,
        destination_path: *const c_char,
    ) -> bool;
    pub fn cbf_bridge_client_pause_download(
        client: *mut CbfBridgeClientHandle,
        download_id: u64,
    ) -> bool;
    pub fn cbf_bridge_client_resume_download(
        client: *mut CbfBridgeClientHandle,
        download_id: u64,
    ) -> bool;
    pub fn cbf_bridge_client_cancel_download(
        client: *mut CbfBridgeClientHandle,
        download_id: u64,
    ) -> bool;
    pub fn cbf_bridge_client_close_prompt_ui(
        client: *mut CbfBridgeClientHandle,
        profile_id: *const c_char,
        prompt_ui_id: u64,
    ) -> bool;
    pub fn cbf_bridge_client_respond_tab_open(
        client: *mut CbfBridgeClientHandle,
        request_id: u64,
        response_kind: u8,
        target_tab_id: u64,
        activate: bool,
    ) -> bool;
    pub fn cbf_bridge_client_shutdown(client: *mut CbfBridgeClientHandle);
    pub fn cbf_bridge_client_request_shutdown(
        client: *mut CbfBridgeClientHandle,
        request_id: u64,
    ) -> bool;
    pub fn cbf_bridge_client_confirm_shutdown(
        client: *mut CbfBridgeClientHandle,
        request_id: u64,
        proceed: bool,
    ) -> bool;
    pub fn cbf_bridge_client_force_shutdown(client: *mut CbfBridgeClientHandle) -> bool;
}
