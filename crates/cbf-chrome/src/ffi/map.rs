#[cfg(target_os = "macos")]
use std::{ffi::c_void, ptr::NonNull};

use cbf::data::dialog::DialogType;
use cbf::data::{
    drag::DragData,
    key::KeyEvent,
    mouse::{MouseEvent, MouseWheelEvent, PointerType},
};
use cbf_chrome_sys::ffi::*;
use cursor_icon::CursorIcon;

use super::{Error, IpcEvent, utils::c_string_to_string};
use crate::data::{
    choice_menu::{
        ChromeChoiceMenu, ChromeChoiceMenuItem, ChromeChoiceMenuSelectionMode,
        choice_menu_item_type_from_ffi, choice_menu_text_direction_from_ffi,
    },
    context_menu::{
        ChromeContextMenu, ChromeContextMenuAccelerator, ChromeContextMenuIcon,
        ChromeContextMenuItem, ChromeContextMenuItemType,
    },
    custom_scheme::{ChromeCustomSchemeRequest, ChromeCustomSchemeRequestMethod},
    download::{
        ChromeDownloadCompletion, ChromeDownloadId, ChromeDownloadOutcome, ChromeDownloadProgress,
        ChromeDownloadPromptReason, ChromeDownloadPromptResult, ChromeDownloadSnapshot,
        ChromeDownloadState,
    },
    drag::{
        ChromeDragData, ChromeDragImage, ChromeDragOperations, ChromeDragStartRequest,
        ChromeDragUrlInfo,
    },
    extension::{ChromeExtensionInfo, ChromeIconData},
    find::ChromeFindRect,
    ids::{PopupId, TabId},
    ime::{
        ChromeImeBoundsUpdate, ChromeImeCompositionBounds, ChromeImeRect, ChromeImeTextRange,
        ChromeImeTextSpan, ChromeImeTextSpanStyle, ChromeImeTextSpanThickness,
        ChromeImeTextSpanType, ChromeImeTextSpanUnderlineStyle, ChromeTextSelectionBounds,
    },
    input::{ChromeKeyEvent, ChromeKeyEventType, ChromeMouseWheelEvent, ChromeScrollGranularity},
    ipc::{TabIpcErrorCode, TabIpcMessageType, TabIpcPayload},
    lifecycle::ChromeBeforeUnloadReason,
    mouse::{ChromeMouseButton, ChromeMouseEvent, ChromeMouseEventType, ChromePointerType},
    prompt_ui::{
        PromptUiCloseReason, PromptUiDialogResult, PromptUiExtensionInstallResult,
        PromptUiExtensionUninstallResult, PromptUiFormResubmissionReason, PromptUiId, PromptUiKind,
        PromptUiPermissionType, PromptUiResolution, PromptUiResolutionResult,
    },
    surface::SurfaceHandle,
    tab_open::{TabOpenHint, TabOpenResult},
};

pub(super) fn parse_event(event: CbfBridgeEvent) -> Result<IpcEvent, Error> {
    match event.kind {
        CBF_EVENT_SURFACE_HANDLE_UPDATED => {
            let handle = parse_surface_handle(event.surface_handle)?;

            Ok(IpcEvent::SurfaceHandleUpdated {
                profile_id: c_string_to_string(event.profile_id),
                browsing_context_id: TabId::new(event.tab_id),
                handle,
            })
        }
        CBF_EVENT_EXTENSION_POPUP_OPENED => Ok(IpcEvent::ExtensionPopupOpened {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            popup_id: event.extension_popup_id,
            extension_id: c_string_to_string(event.extension_id),
            title: c_string_to_string(event.title),
        }),
        CBF_EVENT_EXTENSION_POPUP_SURFACE_HANDLE_UPDATED => {
            let handle = parse_surface_handle(event.surface_handle)?;

            Ok(IpcEvent::ExtensionPopupSurfaceHandleUpdated {
                profile_id: c_string_to_string(event.profile_id),
                browsing_context_id: TabId::new(event.tab_id),
                popup_id: event.extension_popup_id,
                handle,
            })
        }
        CBF_EVENT_EXTENSION_POPUP_PREFERRED_SIZE_CHANGED => {
            Ok(IpcEvent::ExtensionPopupPreferredSizeChanged {
                profile_id: c_string_to_string(event.profile_id),
                browsing_context_id: TabId::new(event.tab_id),
                popup_id: event.extension_popup_id,
                width: event.width,
                height: event.height,
            })
        }
        CBF_EVENT_EXTENSION_POPUP_CONTEXT_MENU_REQUESTED => {
            Ok(IpcEvent::ExtensionPopupContextMenuRequested {
                profile_id: c_string_to_string(event.profile_id),
                browsing_context_id: TabId::new(event.tab_id),
                popup_id: PopupId::new(event.extension_popup_id),
                menu: parse_context_menu(event.context_menu),
            })
        }
        CBF_EVENT_EXTENSION_POPUP_CHOICE_MENU_REQUESTED => {
            Ok(IpcEvent::ExtensionPopupChoiceMenuRequested {
                profile_id: c_string_to_string(event.profile_id),
                browsing_context_id: TabId::new(event.tab_id),
                popup_id: PopupId::new(event.extension_popup_id),
                request_id: event.request_id,
                menu: parse_choice_menu(event.choice_menu),
            })
        }
        CBF_EVENT_EXTENSION_POPUP_CURSOR_CHANGED => Ok(IpcEvent::ExtensionPopupCursorChanged {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            popup_id: PopupId::new(event.extension_popup_id),
            cursor_type: cursor_icon_from_ffi(event.cursor_type),
        }),
        CBF_EVENT_EXTENSION_POPUP_TITLE_UPDATED => Ok(IpcEvent::ExtensionPopupTitleUpdated {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            popup_id: PopupId::new(event.extension_popup_id),
            title: c_string_to_string(event.title),
        }),
        CBF_EVENT_EXTENSION_POPUP_JAVASCRIPT_DIALOG_REQUESTED => {
            Ok(IpcEvent::ExtensionPopupJavaScriptDialogRequested {
                profile_id: c_string_to_string(event.profile_id),
                browsing_context_id: TabId::new(event.tab_id),
                popup_id: PopupId::new(event.extension_popup_id),
                request_id: event.request_id,
                r#type: javascript_dialog_type_from_ffi(event.javascript_dialog_type),
                message: c_string_to_string(event.message),
                default_prompt_text: optional_string_from_ffi(event.default_prompt_text),
                reason: beforeunload_reason_from_ffi(event.beforeunload_reason),
            })
        }
        CBF_EVENT_EXTENSION_POPUP_CLOSE_REQUESTED => Ok(IpcEvent::ExtensionPopupCloseRequested {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            popup_id: PopupId::new(event.extension_popup_id),
        }),
        CBF_EVENT_EXTENSION_POPUP_RENDER_PROCESS_GONE => {
            Ok(IpcEvent::ExtensionPopupRenderProcessGone {
                profile_id: c_string_to_string(event.profile_id),
                browsing_context_id: TabId::new(event.tab_id),
                popup_id: PopupId::new(event.extension_popup_id),
                crashed: event.crashed,
            })
        }
        CBF_EVENT_EXTENSION_POPUP_CLOSED => Ok(IpcEvent::ExtensionPopupClosed {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            popup_id: event.extension_popup_id,
        }),
        CBF_EVENT_TAB_CREATED => Ok(IpcEvent::TabCreated {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            request_id: event.request_id,
        }),
        CBF_EVENT_DEVTOOLS_OPENED => Ok(IpcEvent::DevToolsOpened {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            inspected_browsing_context_id: TabId::new(event.inspected_tab_id),
        }),
        CBF_EVENT_IME_BOUNDS_UPDATED => Ok(IpcEvent::ImeBoundsUpdated {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            update: parse_ime_bounds(event.ime_bounds),
        }),
        CBF_EVENT_EXTENSION_POPUP_IME_BOUNDS_UPDATED => {
            Ok(IpcEvent::ExtensionPopupImeBoundsUpdated {
                profile_id: c_string_to_string(event.profile_id),
                browsing_context_id: TabId::new(event.tab_id),
                popup_id: PopupId::new(event.extension_popup_id),
                update: parse_ime_bounds(event.ime_bounds),
            })
        }
        CBF_EVENT_SHUTDOWN_BLOCKED => Ok(IpcEvent::ShutdownBlocked {
            request_id: event.request_id,
            dirty_browsing_context_ids: parse_browsing_context_ids(event.dirty_tab_ids),
        }),
        CBF_EVENT_SHUTDOWN_PROCEEDING => Ok(IpcEvent::ShutdownProceeding {
            request_id: event.request_id,
        }),
        CBF_EVENT_SHUTDOWN_CANCELLED => Ok(IpcEvent::ShutdownCancelled {
            request_id: event.request_id,
        }),
        CBF_EVENT_CONTEXT_MENU_REQUESTED => Ok(IpcEvent::ContextMenuRequested {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            menu: parse_context_menu(event.context_menu),
        }),
        CBF_EVENT_CHOICE_MENU_REQUESTED => Ok(IpcEvent::ChoiceMenuRequested {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            request_id: event.request_id,
            menu: parse_choice_menu(event.choice_menu),
        }),
        CBF_EVENT_TAB_OPEN_REQUESTED => Ok(IpcEvent::TabOpenRequested {
            profile_id: c_string_to_string(event.profile_id),
            request_id: event.request_id,
            source_tab_id: if event.tab_open_has_source {
                Some(TabId::new(event.tab_open_source_tab_id))
            } else {
                None
            },
            target_url: c_string_to_string(event.target_url),
            open_hint: tab_open_hint_from_ffi(event.tab_open_hint),
            user_gesture: event.tab_open_user_gesture,
        }),
        CBF_EVENT_TAB_OPEN_RESOLVED => Ok(IpcEvent::TabOpenResolved {
            profile_id: c_string_to_string(event.profile_id),
            request_id: event.request_id,
            result: tab_open_result_from_ffi(
                event.tab_open_result_kind,
                event.tab_open_has_target,
                event.tab_open_target_tab_id,
            ),
        }),
        CBF_EVENT_NAVIGATION_STATE_CHANGED => Ok(IpcEvent::NavigationStateChanged {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            url: c_string_to_string(event.url),
            can_go_back: event.can_go_back,
            can_go_forward: event.can_go_forward,
            is_loading: event.is_loading,
        }),
        CBF_EVENT_CURSOR_CHANGED => Ok(IpcEvent::CursorChanged {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            cursor_type: cursor_icon_from_ffi(event.cursor_type),
        }),
        CBF_EVENT_TITLE_UPDATED => Ok(IpcEvent::TitleUpdated {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            title: c_string_to_string(event.title),
        }),
        CBF_EVENT_FAVICON_URL_UPDATED => Ok(IpcEvent::FaviconUrlUpdated {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            url: c_string_to_string(event.favicon_url),
        }),
        CBF_EVENT_BEFOREUNLOAD_DIALOG_REQUESTED => {
            let profile_id = c_string_to_string(event.profile_id);
            let browsing_context_id = TabId::new(event.tab_id);
            let reason = beforeunload_reason_from_ffi(event.beforeunload_reason);
            Ok(IpcEvent::BeforeUnloadDialogRequested {
                profile_id,
                browsing_context_id,
                request_id: event.request_id,
                reason,
            })
        }
        CBF_EVENT_JAVASCRIPT_DIALOG_REQUESTED => Ok(IpcEvent::JavaScriptDialogRequested {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            request_id: event.request_id,
            r#type: javascript_dialog_type_from_ffi(event.javascript_dialog_type),
            message: c_string_to_string(event.message),
            default_prompt_text: optional_string_from_ffi(event.default_prompt_text),
            reason: beforeunload_reason_from_ffi(event.beforeunload_reason),
        }),
        CBF_EVENT_TAB_CLOSED => Ok(IpcEvent::TabClosed {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
        }),
        CBF_EVENT_TAB_RESIZE_ACKNOWLEDGED => Ok(IpcEvent::TabResizeAcknowledged {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
        }),
        CBF_EVENT_TAB_DOM_HTML_READ => Ok(IpcEvent::TabDomHtmlRead {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            request_id: event.request_id,
            html: c_string_to_string(event.dom_html),
        }),
        CBF_EVENT_FIND_REPLY => Ok(IpcEvent::FindReply {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            request_id: event.request_id,
            number_of_matches: event.find_number_of_matches,
            active_match_ordinal: event.find_active_match_ordinal,
            selection_rect: find_rect_from_ffi(event.find_selection_rect),
            final_update: event.find_final_update,
        }),
        CBF_EVENT_TAB_IPC_MESSAGE_RECEIVED => Ok(IpcEvent::TabIpcMessageReceived {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            channel: c_string_to_string(event.ipc_channel),
            message_type: ipc_message_type_from_ffi(event.ipc_message_type),
            request_id: event.request_id,
            payload: ipc_payload_from_ffi(
                event.ipc_payload_kind,
                event.ipc_payload_text,
                event.ipc_payload_binary,
                event.ipc_payload_binary_len,
            ),
            content_type: optional_string_from_ffi(event.ipc_content_type),
            error_code: ipc_error_code_from_ffi(event.ipc_error_code),
        }),
        CBF_EVENT_DRAG_START_REQUESTED => {
            let profile_id = c_string_to_string(event.profile_id);
            let request = parse_drag_start_request(event.drag_start_request);
            Ok(IpcEvent::DragStartRequested {
                browsing_context_id: request.browsing_context_id,
                profile_id,
                request,
            })
        }
        CBF_EVENT_EXTERNAL_DRAG_OPERATION_CHANGED => Ok(IpcEvent::ExternalDragOperationChanged {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            operation: drag_operation_from_ffi(event.drag_operation),
        }),
        CBF_EVENT_EXTENSIONS_LISTED => Ok(IpcEvent::ExtensionsListed {
            profile_id: c_string_to_string(event.profile_id),
            extensions: parse_extension_list(event.extensions),
        }),
        CBF_EVENT_PROMPT_UI_REQUESTED => Ok(IpcEvent::PromptUiOpenRequested {
            profile_id: c_string_to_string(event.profile_id),
            source_tab_id: event
                .prompt_ui_has_source_tab_id
                .then(|| TabId::new(event.prompt_ui_source_tab_id)),
            request_id: event.request_id,
            kind: prompt_ui_kind_from_ffi(
                event.prompt_ui_kind,
                event.prompt_ui_permission,
                event.prompt_ui_permission_key,
                event.download_reason,
                event.download_id,
                event.download_file_name,
                event.download_total_bytes,
                event.download_has_total_bytes,
                event.download_suggested_path,
                event.extension_id,
                event.extension_name,
                event.triggering_extension_name,
                event.prompt_ui_can_report_abuse,
                event.permission_names,
                event.prompt_ui_repost_reason,
                event.prompt_ui_repost_target_url,
            ),
        }),
        CBF_EVENT_PROMPT_UI_RESOLVED => Ok(IpcEvent::PromptUiResolved {
            profile_id: c_string_to_string(event.profile_id),
            source_tab_id: event
                .prompt_ui_has_source_tab_id
                .then(|| TabId::new(event.prompt_ui_source_tab_id)),
            request_id: event.request_id,
            resolution: prompt_ui_resolution_from_ffi(
                event.prompt_ui_kind,
                event.prompt_ui_permission,
                event.prompt_ui_permission_key,
                event.prompt_ui_result,
                event.download_id,
                event.download_destination_path,
                event.extension_id,
                event.prompt_ui_extension_install_result,
                event.prompt_ui_extension_uninstall_result,
                event.prompt_ui_extension_install_detail,
                event.prompt_ui_report_abuse,
                event.prompt_ui_repost_reason,
                event.prompt_ui_repost_target_url,
            ),
        }),
        CBF_EVENT_CUSTOM_SCHEME_REQUEST_RECEIVED => Ok(IpcEvent::CustomSchemeRequestReceived {
            request: ChromeCustomSchemeRequest {
                request_id: event.request_id,
                profile_id: c_string_to_string(event.profile_id),
                url: c_string_to_string(event.url),
                scheme: c_string_to_string(event.custom_scheme_scheme),
                host: c_string_to_string(event.custom_scheme_host),
                path: c_string_to_string(event.custom_scheme_path),
                query: optional_string_from_ffi(event.custom_scheme_query),
                method: custom_scheme_request_method_from_ffi(event.custom_scheme_method),
            },
        }),
        CBF_EVENT_EXTENSION_RUNTIME_WARNING => Ok(IpcEvent::ExtensionRuntimeWarning {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            detail: c_string_to_string(event.extension_runtime_warning),
        }),
        CBF_EVENT_PROMPT_UI_OPENED => Ok(IpcEvent::PromptUiOpened {
            profile_id: c_string_to_string(event.profile_id),
            source_tab_id: event
                .prompt_ui_has_source_tab_id
                .then(|| TabId::new(event.prompt_ui_source_tab_id)),
            prompt_ui_id: PromptUiId::new(event.prompt_ui_id),
            kind: prompt_ui_kind_from_ffi(
                event.prompt_ui_kind,
                event.prompt_ui_permission,
                event.prompt_ui_permission_key,
                event.download_reason,
                event.download_id,
                event.download_file_name,
                event.download_total_bytes,
                event.download_has_total_bytes,
                event.download_suggested_path,
                event.extension_id,
                event.extension_name,
                event.triggering_extension_name,
                event.prompt_ui_can_report_abuse,
                event.permission_names,
                event.prompt_ui_repost_reason,
                event.prompt_ui_repost_target_url,
            ),
            title: {
                let value = c_string_to_string(event.prompt_ui_title);
                if value.is_empty() { None } else { Some(value) }
            },
            modal: event.prompt_ui_modal,
        }),
        CBF_EVENT_PROMPT_UI_CLOSED => Ok(IpcEvent::PromptUiClosed {
            profile_id: c_string_to_string(event.profile_id),
            source_tab_id: event
                .prompt_ui_has_source_tab_id
                .then(|| TabId::new(event.prompt_ui_source_tab_id)),
            prompt_ui_id: PromptUiId::new(event.prompt_ui_id),
            kind: prompt_ui_kind_from_ffi(
                event.prompt_ui_kind,
                event.prompt_ui_permission,
                event.prompt_ui_permission_key,
                event.download_reason,
                event.download_id,
                event.download_file_name,
                event.download_total_bytes,
                event.download_has_total_bytes,
                event.download_suggested_path,
                event.extension_id,
                event.extension_name,
                event.triggering_extension_name,
                event.prompt_ui_can_report_abuse,
                event.permission_names,
                event.prompt_ui_repost_reason,
                event.prompt_ui_repost_target_url,
            ),
            reason: prompt_ui_close_reason_from_ffi(event.prompt_ui_close_reason),
        }),
        CBF_EVENT_DOWNLOAD_CREATED => Ok(IpcEvent::DownloadCreated {
            profile_id: c_string_to_string(event.profile_id),
            download: parse_download_snapshot(event),
        }),
        CBF_EVENT_DOWNLOAD_UPDATED => Ok(IpcEvent::DownloadUpdated {
            profile_id: c_string_to_string(event.profile_id),
            download: parse_download_progress(event),
        }),
        CBF_EVENT_DOWNLOAD_COMPLETED => Ok(IpcEvent::DownloadCompleted {
            profile_id: c_string_to_string(event.profile_id),
            download: parse_download_completion(event),
        }),
        _ => Err(Error::InvalidEvent),
    }
}

fn tab_open_hint_from_ffi(value: u8) -> TabOpenHint {
    match value {
        CBF_TAB_OPEN_HINT_CURRENT_CONTEXT => TabOpenHint::CurrentTab,
        CBF_TAB_OPEN_HINT_NEW_FOREGROUND_CONTEXT => TabOpenHint::NewForegroundTab,
        CBF_TAB_OPEN_HINT_NEW_BACKGROUND_CONTEXT => TabOpenHint::NewBackgroundTab,
        CBF_TAB_OPEN_HINT_NEW_WINDOW => TabOpenHint::NewWindow,
        CBF_TAB_OPEN_HINT_POPUP => TabOpenHint::Popup,
        _ => TabOpenHint::Unknown,
    }
}

fn tab_open_result_from_ffi(value: u8, has_target: bool, target_tab_id: u64) -> TabOpenResult {
    match value {
        CBF_TAB_OPEN_RESULT_OPENED_NEW_CONTEXT => {
            if has_target {
                TabOpenResult::OpenedNewTab {
                    tab_id: TabId::new(target_tab_id),
                }
            } else {
                TabOpenResult::Aborted
            }
        }
        CBF_TAB_OPEN_RESULT_OPENED_EXISTING_CONTEXT => {
            if has_target {
                TabOpenResult::OpenedExistingTab {
                    tab_id: TabId::new(target_tab_id),
                }
            } else {
                TabOpenResult::Aborted
            }
        }
        CBF_TAB_OPEN_RESULT_DENIED => TabOpenResult::Denied,
        CBF_TAB_OPEN_RESULT_ABORTED => TabOpenResult::Aborted,
        _ => TabOpenResult::Aborted,
    }
}

fn parse_drag_start_request(request: CbfDragStartRequest) -> ChromeDragStartRequest {
    ChromeDragStartRequest {
        session_id: request.session_id,
        browsing_context_id: TabId::new(request.tab_id),
        allowed_operations: ChromeDragOperations::from_bits(request.allowed_operations),
        source_origin: c_string_to_string(request.source_origin),
        data: ChromeDragData {
            text: c_string_to_string(request.data.text),
            html: c_string_to_string(request.data.html),
            html_base_url: c_string_to_string(request.data.html_base_url),
            url_infos: parse_drag_url_infos(request.data.url_infos),
            filenames: parse_string_list(request.data.filenames),
            file_mime_types: parse_string_list(request.data.file_mime_types),
            custom_data: parse_string_pair_list(request.data.custom_data),
        },
        image: parse_drag_image(request.image),
    }
}

fn drag_operation_from_ffi(operation: u32) -> cbf::data::drag::DragOperation {
    match operation {
        1 => cbf::data::drag::DragOperation::Copy,
        2 => cbf::data::drag::DragOperation::Link,
        16 => cbf::data::drag::DragOperation::Move,
        _ => cbf::data::drag::DragOperation::None,
    }
}

fn parse_drag_url_infos(list: CbfDragUrlInfoList) -> Vec<ChromeDragUrlInfo> {
    if list.len == 0 || list.items.is_null() {
        return Vec::new();
    }
    let infos = unsafe { std::slice::from_raw_parts(list.items, list.len as usize) };
    infos
        .iter()
        .map(|info| ChromeDragUrlInfo {
            url: c_string_to_string(info.url),
            title: c_string_to_string(info.title),
        })
        .collect()
}

fn parse_string_list(list: CbfStringList) -> Vec<String> {
    if list.len == 0 || list.items.is_null() {
        return Vec::new();
    }
    let values = unsafe { std::slice::from_raw_parts(list.items, list.len as usize) };
    values
        .iter()
        .map(|value| c_string_to_string(*value))
        .collect()
}

pub(crate) fn parse_extension_list(list: CbfExtensionInfoList) -> Vec<ChromeExtensionInfo> {
    if list.len == 0 || list.items.is_null() {
        return Vec::new();
    }
    let values = unsafe { std::slice::from_raw_parts(list.items, list.len as usize) };
    values
        .iter()
        .map(|value| ChromeExtensionInfo {
            id: c_string_to_string(value.id),
            name: c_string_to_string(value.name),
            version: c_string_to_string(value.version),
            enabled: value.enabled,
            permission_names: parse_string_list(value.permission_names),
            icon: parse_icon_data(value.icon),
        })
        .collect()
}

pub(crate) fn parse_icon_data(icon: CbfIconData) -> Option<ChromeIconData> {
    match icon.kind {
        CBF_ICON_DATA_KIND_NONE => None,
        CBF_ICON_DATA_KIND_URL => {
            let url = c_string_to_string(icon.url);
            if url.is_empty() {
                None
            } else {
                Some(ChromeIconData::Url(url))
            }
        }
        CBF_ICON_DATA_KIND_PNG => {
            if icon.bytes.is_null() || icon.len == 0 {
                return None;
            }
            let bytes = unsafe { std::slice::from_raw_parts(icon.bytes, icon.len as usize) };
            if bytes.is_empty() {
                None
            } else {
                Some(ChromeIconData::Png(bytes.to_vec()))
            }
        }
        CBF_ICON_DATA_KIND_BINARY => {
            if icon.bytes.is_null() || icon.len == 0 {
                return None;
            }
            let bytes = unsafe { std::slice::from_raw_parts(icon.bytes, icon.len as usize) };
            if bytes.is_empty() {
                return None;
            }
            let media_type = {
                let value = c_string_to_string(icon.media_type);
                if value.is_empty() { None } else { Some(value) }
            };
            Some(ChromeIconData::Binary {
                media_type,
                bytes: bytes.to_vec(),
            })
        }
        _ => None,
    }
}

fn parse_string_pair_list(list: CbfStringPairList) -> std::collections::BTreeMap<String, String> {
    if list.len == 0 || list.items.is_null() {
        return Default::default();
    }
    let pairs = unsafe { std::slice::from_raw_parts(list.items, list.len as usize) };
    pairs
        .iter()
        .map(|pair| (c_string_to_string(pair.key), c_string_to_string(pair.value)))
        .collect()
}

fn parse_drag_image(image: CbfDragImage) -> Option<ChromeDragImage> {
    if image.png_bytes.is_null() || image.png_len == 0 {
        return None;
    }
    let bytes = unsafe { std::slice::from_raw_parts(image.png_bytes, image.png_len as usize) };
    Some(ChromeDragImage {
        png_bytes: bytes.to_vec(),
        pixel_width: image.pixel_width,
        pixel_height: image.pixel_height,
        scale: image.scale,
        cursor_offset_x: image.cursor_offset_x,
        cursor_offset_y: image.cursor_offset_y,
    })
}

fn parse_ime_bounds(update: CbfImeBoundsUpdate) -> ChromeImeBoundsUpdate {
    let composition = if update.has_composition {
        let list = update.composition.character_bounds;
        let rects = if list.len == 0 || list.items.is_null() {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(list.items, list.len as usize) }
                .iter()
                .map(|rect| rect_from_ffi(*rect))
                .collect()
        };
        Some(ChromeImeCompositionBounds {
            range_start: update.composition.range_start,
            range_end: update.composition.range_end,
            character_bounds: rects,
        })
    } else {
        None
    };

    let selection = if update.has_selection {
        Some(ChromeTextSelectionBounds {
            range_start: update.selection.range_start,
            range_end: update.selection.range_end,
            caret_rect: rect_from_ffi(update.selection.caret_rect),
            first_selection_rect: rect_from_ffi(update.selection.first_selection_rect),
        })
    } else {
        None
    };

    ChromeImeBoundsUpdate {
        composition,
        selection,
    }
}

fn parse_browsing_context_ids(list: CbfTabIdList) -> Vec<TabId> {
    if list.len == 0 || list.items.is_null() {
        return Vec::new();
    }

    let ids = unsafe { std::slice::from_raw_parts(list.items, list.len as usize) };
    ids.iter().copied().map(TabId::new).collect()
}

fn parse_context_menu(menu: CbfContextMenu) -> ChromeContextMenu {
    let menu = ChromeContextMenu {
        menu_id: menu.menu_id,
        x: menu.x,
        y: menu.y,
        source_type: menu.source_type,
        items: parse_context_menu_items(menu.items),
    };

    crate::data::context_menu::filter_supported(menu)
}

fn parse_choice_menu(menu: CbfChoiceMenu) -> ChromeChoiceMenu {
    ChromeChoiceMenu {
        request_id: menu.request_id,
        x: menu.x,
        y: menu.y,
        width: menu.width,
        height: menu.height,
        item_font_size: menu.item_font_size,
        selected_item: menu.selected_item,
        right_aligned: menu.right_aligned,
        selection_mode: if menu.allow_multiple_selection {
            ChromeChoiceMenuSelectionMode::Multiple
        } else {
            ChromeChoiceMenuSelectionMode::Single
        },
        items: parse_choice_menu_items(menu.items),
    }
}

fn parse_choice_menu_items(list: CbfChoiceMenuItemList) -> Vec<ChromeChoiceMenuItem> {
    if list.len == 0 || list.items.is_null() {
        return Vec::new();
    }

    let items = unsafe { std::slice::from_raw_parts(list.items, list.len as usize) };
    items.iter().map(parse_choice_menu_item).collect()
}

fn parse_choice_menu_item(item: &CbfChoiceMenuItem) -> ChromeChoiceMenuItem {
    ChromeChoiceMenuItem {
        item_type: choice_menu_item_type_from_ffi(item.r#type),
        label: optional_string_from_ffi(item.label),
        tool_tip: optional_string_from_ffi(item.tool_tip),
        enabled: item.enabled,
        checked: item.checked,
        text_direction: choice_menu_text_direction_from_ffi(item.text_direction),
        has_text_direction_override: item.has_text_direction_override,
        action: item.action,
        children: parse_choice_menu_items(item.children),
    }
}

fn parse_context_menu_items(list: CbfContextMenuItemList) -> Vec<ChromeContextMenuItem> {
    if list.len == 0 || list.items.is_null() {
        return Vec::new();
    }

    let items = unsafe { std::slice::from_raw_parts(list.items, list.len as usize) };
    items.iter().map(parse_context_menu_item).collect()
}

fn parse_context_menu_item(item: &CbfContextMenuItem) -> ChromeContextMenuItem {
    ChromeContextMenuItem {
        r#type: context_menu_item_type_from_ffi(item.r#type),
        command_id: item.command_id,
        label: c_string_to_string(item.label),
        secondary_label: c_string_to_string(item.secondary_label),
        minor_text: c_string_to_string(item.minor_text),
        accessible_name: c_string_to_string(item.accessible_name),
        enabled: item.enabled,
        visible: item.visible,
        checked: item.checked,
        group_id: item.group_id,
        is_new_feature: item.is_new_feature,
        is_alerted: item.is_alerted,
        may_have_mnemonics: item.may_have_mnemonics,
        accelerator: parse_context_menu_accelerator(item),
        icon: parse_context_menu_icon(item.icon),
        minor_icon: parse_context_menu_icon(item.minor_icon),
        submenu: parse_context_menu_items(item.submenu),
    }
}

fn parse_context_menu_icon(icon: CbfContextMenuIcon) -> Option<ChromeContextMenuIcon> {
    if icon.len == 0 || icon.png_bytes.is_null() {
        return None;
    }
    let bytes = unsafe { std::slice::from_raw_parts(icon.png_bytes, icon.len as usize) };
    Some(ChromeContextMenuIcon {
        png_bytes: bytes.to_vec(),
        width: icon.width,
        height: icon.height,
    })
}

fn parse_context_menu_accelerator(
    item: &CbfContextMenuItem,
) -> Option<ChromeContextMenuAccelerator> {
    if !item.has_accelerator {
        return None;
    }

    Some(ChromeContextMenuAccelerator {
        key_equivalent: c_string_to_string(item.accelerator_key_equivalent),
        modifier_mask: item.accelerator_modifier_mask,
    })
}

fn context_menu_item_type_from_ffi(value: u8) -> ChromeContextMenuItemType {
    match value {
        CBF_MENU_ITEM_COMMAND => ChromeContextMenuItemType::Command,
        CBF_MENU_ITEM_CHECK => ChromeContextMenuItemType::Check,
        CBF_MENU_ITEM_RADIO => ChromeContextMenuItemType::Radio,
        CBF_MENU_ITEM_SEPARATOR => ChromeContextMenuItemType::Separator,
        CBF_MENU_ITEM_BUTTON_ITEM => ChromeContextMenuItemType::ButtonItem,
        CBF_MENU_ITEM_SUBMENU => ChromeContextMenuItemType::Submenu,
        CBF_MENU_ITEM_ACTIONABLE_SUBMENU => ChromeContextMenuItemType::ActionableSubmenu,
        CBF_MENU_ITEM_HIGHLIGHTED => ChromeContextMenuItemType::Highlighted,
        CBF_MENU_ITEM_TITLE => ChromeContextMenuItemType::Title,
        _ => ChromeContextMenuItemType::Command,
    }
}

fn rect_from_ffi(rect: CbfRect) -> ChromeImeRect {
    ChromeImeRect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    }
}

fn find_rect_from_ffi(rect: CbfRect) -> ChromeFindRect {
    ChromeFindRect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    }
}

fn beforeunload_reason_from_ffi(value: u8) -> ChromeBeforeUnloadReason {
    match value {
        CBF_BEFOREUNLOAD_REASON_CLOSE_TAB => ChromeBeforeUnloadReason::CloseBrowsingContext,
        CBF_BEFOREUNLOAD_REASON_NAVIGATE => ChromeBeforeUnloadReason::Navigate,
        CBF_BEFOREUNLOAD_REASON_RELOAD => ChromeBeforeUnloadReason::Reload,
        CBF_BEFOREUNLOAD_REASON_WINDOW_CLOSE => ChromeBeforeUnloadReason::WindowClose,
        _ => ChromeBeforeUnloadReason::Unknown,
    }
}

fn javascript_dialog_type_from_ffi(value: u8) -> DialogType {
    match value {
        CBF_JAVASCRIPT_DIALOG_ALERT => DialogType::Alert,
        CBF_JAVASCRIPT_DIALOG_CONFIRM => DialogType::Confirm,
        CBF_JAVASCRIPT_DIALOG_PROMPT => DialogType::Prompt,
        CBF_JAVASCRIPT_DIALOG_BEFOREUNLOAD => DialogType::BeforeUnload,
        _ => DialogType::Alert,
    }
}

fn ipc_message_type_from_ffi(value: u8) -> TabIpcMessageType {
    match value {
        CBF_IPC_MESSAGE_REQUEST => TabIpcMessageType::Request,
        CBF_IPC_MESSAGE_RESPONSE => TabIpcMessageType::Response,
        CBF_IPC_MESSAGE_EVENT => TabIpcMessageType::Event,
        _ => TabIpcMessageType::Event,
    }
}

fn ipc_payload_from_ffi(
    kind: u8,
    text: *mut std::ffi::c_char,
    binary: *const u8,
    binary_len: u32,
) -> TabIpcPayload {
    match kind {
        CBF_IPC_PAYLOAD_BINARY if !binary.is_null() && binary_len > 0 => {
            let bytes = unsafe { std::slice::from_raw_parts(binary, binary_len as usize) };
            TabIpcPayload::Binary(bytes.to_vec())
        }
        _ => TabIpcPayload::Text(c_string_to_string(text)),
    }
}

fn ipc_error_code_from_ffi(value: u8) -> Option<TabIpcErrorCode> {
    match value {
        CBF_IPC_ERROR_NONE => None,
        CBF_IPC_ERROR_TIMEOUT => Some(TabIpcErrorCode::Timeout),
        CBF_IPC_ERROR_ABORTED => Some(TabIpcErrorCode::Aborted),
        CBF_IPC_ERROR_DISCONNECTED => Some(TabIpcErrorCode::Disconnected),
        CBF_IPC_ERROR_IPC_DISABLED => Some(TabIpcErrorCode::IpcDisabled),
        CBF_IPC_ERROR_CONTEXT_CLOSED => Some(TabIpcErrorCode::ContextClosed),
        CBF_IPC_ERROR_REMOTE_ERROR => Some(TabIpcErrorCode::RemoteError),
        CBF_IPC_ERROR_PROTOCOL_ERROR => Some(TabIpcErrorCode::ProtocolError),
        _ => Some(TabIpcErrorCode::ProtocolError),
    }
}

fn custom_scheme_request_method_from_ffi(
    value: *mut std::ffi::c_char,
) -> ChromeCustomSchemeRequestMethod {
    match c_string_to_string(value).as_str() {
        "GET" => ChromeCustomSchemeRequestMethod::Get,
        other => ChromeCustomSchemeRequestMethod::Other(other.to_string()),
    }
}

fn optional_string_from_ffi(value: *mut std::ffi::c_char) -> Option<String> {
    let value = c_string_to_string(value);
    if value.is_empty() { None } else { Some(value) }
}

fn prompt_ui_extension_install_result_from_ffi(value: u8) -> PromptUiExtensionInstallResult {
    match value {
        CBF_PROMPT_UI_EXTENSION_INSTALL_RESULT_ACCEPTED => PromptUiExtensionInstallResult::Accepted,
        CBF_PROMPT_UI_EXTENSION_INSTALL_RESULT_ACCEPTED_WITH_WITHHELD_PERMISSIONS => {
            PromptUiExtensionInstallResult::AcceptedWithWithheldPermissions
        }
        CBF_PROMPT_UI_EXTENSION_INSTALL_RESULT_USER_CANCELED => {
            PromptUiExtensionInstallResult::UserCanceled
        }
        CBF_PROMPT_UI_EXTENSION_INSTALL_RESULT_ABORTED => PromptUiExtensionInstallResult::Aborted,
        _ => PromptUiExtensionInstallResult::Aborted,
    }
}

fn prompt_ui_extension_uninstall_result_from_ffi(value: u8) -> PromptUiExtensionUninstallResult {
    match value {
        CBF_PROMPT_UI_EXTENSION_UNINSTALL_RESULT_ACCEPTED => {
            PromptUiExtensionUninstallResult::Accepted
        }
        CBF_PROMPT_UI_EXTENSION_UNINSTALL_RESULT_USER_CANCELED => {
            PromptUiExtensionUninstallResult::UserCanceled
        }
        CBF_PROMPT_UI_EXTENSION_UNINSTALL_RESULT_ABORTED => {
            PromptUiExtensionUninstallResult::Aborted
        }
        CBF_PROMPT_UI_EXTENSION_UNINSTALL_RESULT_FAILED => PromptUiExtensionUninstallResult::Failed,
        _ => PromptUiExtensionUninstallResult::Aborted,
    }
}

#[allow(clippy::too_many_arguments)]
fn prompt_ui_kind_from_ffi(
    kind: u8,
    permission: u8,
    permission_key: *mut std::ffi::c_char,
    download_reason: u8,
    download_id: u64,
    download_file_name: *mut std::ffi::c_char,
    download_total_bytes: u64,
    download_has_total_bytes: bool,
    download_suggested_path: *mut std::ffi::c_char,
    extension_id: *mut std::ffi::c_char,
    extension_name: *mut std::ffi::c_char,
    triggering_extension_name: *mut std::ffi::c_char,
    can_report_abuse: bool,
    permission_names: CbfStringList,
    repost_reason: u8,
    repost_target_url: *mut std::ffi::c_char,
) -> PromptUiKind {
    let permission_key = {
        let value = c_string_to_string(permission_key);
        if value.is_empty() { None } else { Some(value) }
    };
    match kind {
        CBF_PROMPT_UI_KIND_PERMISSION_PROMPT => PromptUiKind::PermissionPrompt {
            permission: prompt_ui_permission_from_ffi(permission),
            permission_key,
        },
        CBF_PROMPT_UI_KIND_DOWNLOAD_PROMPT => PromptUiKind::DownloadPrompt {
            download_id: ChromeDownloadId::new(download_id),
            file_name: c_string_to_string(download_file_name),
            total_bytes: download_has_total_bytes.then_some(download_total_bytes),
            suggested_path: {
                let value = c_string_to_string(download_suggested_path);
                if value.is_empty() { None } else { Some(value) }
            },
            reason: download_prompt_reason_from_ffi(download_reason),
        },
        CBF_PROMPT_UI_KIND_EXTENSION_INSTALL_PROMPT => PromptUiKind::ExtensionInstallPrompt {
            extension_id: c_string_to_string(extension_id),
            extension_name: c_string_to_string(extension_name),
            permission_names: parse_string_list(permission_names),
        },
        CBF_PROMPT_UI_KIND_EXTENSION_UNINSTALL_PROMPT => PromptUiKind::ExtensionUninstallPrompt {
            extension_id: c_string_to_string(extension_id),
            extension_name: c_string_to_string(extension_name),
            triggering_extension_name: optional_string_from_ffi(triggering_extension_name),
            can_report_abuse,
        },
        CBF_PROMPT_UI_KIND_PRINT_PREVIEW_DIALOG => PromptUiKind::PrintPreviewDialog,
        CBF_PROMPT_UI_KIND_FORM_RESUBMISSION_PROMPT => PromptUiKind::FormResubmissionPrompt {
            reason: prompt_ui_form_resubmission_reason_from_ffi(repost_reason),
            target_url: optional_string_from_ffi(repost_target_url),
        },
        _ => PromptUiKind::Unknown,
    }
}

#[allow(clippy::too_many_arguments)]
fn prompt_ui_dialog_result_from_ffi(value: u8) -> PromptUiDialogResult {
    match value {
        CBF_PROMPT_UI_DIALOG_RESULT_PROCEEDED => PromptUiDialogResult::Proceeded,
        CBF_PROMPT_UI_DIALOG_RESULT_CANCELED => PromptUiDialogResult::Canceled,
        CBF_PROMPT_UI_DIALOG_RESULT_ABORTED => PromptUiDialogResult::Aborted,
        _ => PromptUiDialogResult::Unknown,
    }
}

#[allow(clippy::too_many_arguments)]
fn prompt_ui_resolution_from_ffi(
    kind: u8,
    permission: u8,
    permission_key: *mut std::ffi::c_char,
    permission_result: u8,
    download_id: u64,
    download_destination_path: *mut std::ffi::c_char,
    extension_id: *mut std::ffi::c_char,
    extension_install_result: u8,
    extension_uninstall_result: u8,
    detail: *mut std::ffi::c_char,
    report_abuse: bool,
    repost_reason: u8,
    repost_target_url: *mut std::ffi::c_char,
) -> PromptUiResolution {
    let permission_key = {
        let value = c_string_to_string(permission_key);
        if value.is_empty() { None } else { Some(value) }
    };
    match kind {
        CBF_PROMPT_UI_KIND_PERMISSION_PROMPT => PromptUiResolution::PermissionPrompt {
            permission: prompt_ui_permission_from_ffi(permission),
            permission_key,
            result: prompt_ui_resolution_result_from_ffi(permission_result),
        },
        CBF_PROMPT_UI_KIND_DOWNLOAD_PROMPT => PromptUiResolution::DownloadPrompt {
            download_id: ChromeDownloadId::new(download_id),
            destination_path: {
                let value = c_string_to_string(download_destination_path);
                if value.is_empty() { None } else { Some(value) }
            },
            result: download_prompt_result_from_ffi(permission_result),
        },
        CBF_PROMPT_UI_KIND_EXTENSION_INSTALL_PROMPT => PromptUiResolution::ExtensionInstallPrompt {
            extension_id: c_string_to_string(extension_id),
            result: prompt_ui_extension_install_result_from_ffi(extension_install_result),
            detail: {
                let value = c_string_to_string(detail);
                if value.is_empty() { None } else { Some(value) }
            },
        },
        CBF_PROMPT_UI_KIND_EXTENSION_UNINSTALL_PROMPT => {
            PromptUiResolution::ExtensionUninstallPrompt {
                extension_id: c_string_to_string(extension_id),
                result: prompt_ui_extension_uninstall_result_from_ffi(extension_uninstall_result),
                detail: {
                    let value = c_string_to_string(detail);
                    if value.is_empty() { None } else { Some(value) }
                },
                report_abuse,
            }
        }
        CBF_PROMPT_UI_KIND_PRINT_PREVIEW_DIALOG => PromptUiResolution::PrintPreviewDialog {
            result: prompt_ui_dialog_result_from_ffi(permission_result),
        },
        CBF_PROMPT_UI_KIND_FORM_RESUBMISSION_PROMPT => PromptUiResolution::FormResubmissionPrompt {
            reason: prompt_ui_form_resubmission_reason_from_ffi(repost_reason),
            target_url: optional_string_from_ffi(repost_target_url),
            result: prompt_ui_resolution_result_from_ffi(permission_result),
        },
        _ => PromptUiResolution::Unknown,
    }
}

#[allow(clippy::too_many_arguments)]
fn download_prompt_result_from_ffi(value: u8) -> ChromeDownloadPromptResult {
    match value {
        CBF_DOWNLOAD_PROMPT_RESULT_ALLOWED => ChromeDownloadPromptResult::Allowed,
        CBF_DOWNLOAD_PROMPT_RESULT_DENIED => ChromeDownloadPromptResult::Denied,
        CBF_DOWNLOAD_PROMPT_RESULT_ABORTED => ChromeDownloadPromptResult::Aborted,
        _ => ChromeDownloadPromptResult::Aborted,
    }
}

fn download_prompt_reason_from_ffi(value: u8) -> ChromeDownloadPromptReason {
    match value {
        CBF_DOWNLOAD_PROMPT_REASON_NONE => ChromeDownloadPromptReason::None,
        CBF_DOWNLOAD_PROMPT_REASON_UNEXPECTED => ChromeDownloadPromptReason::Unexpected,
        CBF_DOWNLOAD_PROMPT_REASON_SAVE_AS => ChromeDownloadPromptReason::SaveAs,
        CBF_DOWNLOAD_PROMPT_REASON_PREFERENCE => ChromeDownloadPromptReason::Preference,
        CBF_DOWNLOAD_PROMPT_REASON_NAME_TOO_LONG => ChromeDownloadPromptReason::NameTooLong,
        CBF_DOWNLOAD_PROMPT_REASON_TARGET_CONFLICT => ChromeDownloadPromptReason::TargetConflict,
        CBF_DOWNLOAD_PROMPT_REASON_TARGET_PATH_NOT_WRITEABLE => {
            ChromeDownloadPromptReason::TargetPathNotWriteable
        }
        CBF_DOWNLOAD_PROMPT_REASON_TARGET_NO_SPACE => ChromeDownloadPromptReason::TargetNoSpace,
        CBF_DOWNLOAD_PROMPT_REASON_DLP_BLOCKED => ChromeDownloadPromptReason::DlpBlocked,
        _ => ChromeDownloadPromptReason::Unknown,
    }
}

fn download_state_from_ffi(value: u8) -> ChromeDownloadState {
    match value {
        CBF_DOWNLOAD_STATE_IN_PROGRESS => ChromeDownloadState::InProgress,
        CBF_DOWNLOAD_STATE_PAUSED => ChromeDownloadState::Paused,
        CBF_DOWNLOAD_STATE_COMPLETED => ChromeDownloadState::Completed,
        CBF_DOWNLOAD_STATE_CANCELLED => ChromeDownloadState::Cancelled,
        CBF_DOWNLOAD_STATE_INTERRUPTED => ChromeDownloadState::Interrupted,
        _ => ChromeDownloadState::Unknown,
    }
}

fn download_outcome_from_ffi(value: u8) -> ChromeDownloadOutcome {
    match value {
        CBF_DOWNLOAD_OUTCOME_SUCCEEDED => ChromeDownloadOutcome::Succeeded,
        CBF_DOWNLOAD_OUTCOME_CANCELLED => ChromeDownloadOutcome::Cancelled,
        CBF_DOWNLOAD_OUTCOME_INTERRUPTED => ChromeDownloadOutcome::Interrupted,
        _ => ChromeDownloadOutcome::Unknown,
    }
}

fn parse_download_snapshot(event: CbfBridgeEvent) -> ChromeDownloadSnapshot {
    ChromeDownloadSnapshot {
        download_id: ChromeDownloadId::new(event.download_id),
        source_tab_id: event
            .download_has_source_tab_id
            .then_some(TabId::new(event.download_source_tab_id)),
        file_name: c_string_to_string(event.download_file_name),
        total_bytes: event
            .download_has_total_bytes
            .then_some(event.download_total_bytes),
        target_path: {
            let value = c_string_to_string(event.download_target_path);
            if value.is_empty() { None } else { Some(value) }
        },
    }
}

fn parse_download_progress(event: CbfBridgeEvent) -> ChromeDownloadProgress {
    ChromeDownloadProgress {
        download_id: ChromeDownloadId::new(event.download_id),
        source_tab_id: event
            .download_has_source_tab_id
            .then_some(TabId::new(event.download_source_tab_id)),
        state: download_state_from_ffi(event.download_state),
        file_name: c_string_to_string(event.download_file_name),
        received_bytes: event.download_received_bytes,
        total_bytes: event
            .download_has_total_bytes
            .then_some(event.download_total_bytes),
        target_path: {
            let value = c_string_to_string(event.download_target_path);
            if value.is_empty() { None } else { Some(value) }
        },
        can_resume: event.download_can_resume,
        is_paused: event.download_is_paused,
    }
}

fn parse_download_completion(event: CbfBridgeEvent) -> ChromeDownloadCompletion {
    ChromeDownloadCompletion {
        download_id: ChromeDownloadId::new(event.download_id),
        source_tab_id: event
            .download_has_source_tab_id
            .then_some(TabId::new(event.download_source_tab_id)),
        outcome: download_outcome_from_ffi(event.download_outcome),
        file_name: c_string_to_string(event.download_file_name),
        received_bytes: event.download_received_bytes,
        total_bytes: event
            .download_has_total_bytes
            .then_some(event.download_total_bytes),
        target_path: {
            let value = c_string_to_string(event.download_target_path);
            if value.is_empty() { None } else { Some(value) }
        },
    }
}

fn prompt_ui_permission_from_ffi(value: u8) -> PromptUiPermissionType {
    match value {
        CBF_PROMPT_UI_PERMISSION_TYPE_GEOLOCATION => PromptUiPermissionType::Geolocation,
        CBF_PROMPT_UI_PERMISSION_TYPE_NOTIFICATIONS => PromptUiPermissionType::Notifications,
        CBF_PROMPT_UI_PERMISSION_TYPE_AUDIO_CAPTURE => PromptUiPermissionType::AudioCapture,
        CBF_PROMPT_UI_PERMISSION_TYPE_VIDEO_CAPTURE => PromptUiPermissionType::VideoCapture,
        _ => PromptUiPermissionType::Unknown,
    }
}

fn prompt_ui_form_resubmission_reason_from_ffi(value: u8) -> PromptUiFormResubmissionReason {
    match value {
        CBF_FORM_RESUBMISSION_REASON_RELOAD => PromptUiFormResubmissionReason::Reload,
        CBF_FORM_RESUBMISSION_REASON_BACK_FORWARD => PromptUiFormResubmissionReason::BackForward,
        CBF_FORM_RESUBMISSION_REASON_OTHER => PromptUiFormResubmissionReason::Other,
        _ => PromptUiFormResubmissionReason::Unknown,
    }
}

fn prompt_ui_resolution_result_from_ffi(value: u8) -> PromptUiResolutionResult {
    match value {
        CBF_PROMPT_UI_RESOLUTION_RESULT_ALLOWED => PromptUiResolutionResult::Allowed,
        CBF_PROMPT_UI_RESOLUTION_RESULT_DENIED => PromptUiResolutionResult::Denied,
        CBF_PROMPT_UI_RESOLUTION_RESULT_ABORTED => PromptUiResolutionResult::Aborted,
        _ => PromptUiResolutionResult::Unknown,
    }
}

fn prompt_ui_close_reason_from_ffi(value: u8) -> PromptUiCloseReason {
    match value {
        CBF_PROMPT_UI_CLOSE_REASON_USER_CANCELED => PromptUiCloseReason::UserCanceled,
        CBF_PROMPT_UI_CLOSE_REASON_HOST_FORCED => PromptUiCloseReason::HostForced,
        CBF_PROMPT_UI_CLOSE_REASON_SYSTEM_DISMISSED => PromptUiCloseReason::SystemDismissed,
        _ => PromptUiCloseReason::Unknown,
    }
}

fn cursor_icon_from_ffi(value: u8) -> CursorIcon {
    match value {
        CBF_CURSOR_CROSSHAIR => CursorIcon::Crosshair,
        CBF_CURSOR_POINTER => CursorIcon::Pointer,
        CBF_CURSOR_MOVE => CursorIcon::Move,
        CBF_CURSOR_TEXT => CursorIcon::Text,
        CBF_CURSOR_WAIT => CursorIcon::Wait,
        CBF_CURSOR_HELP => CursorIcon::Help,
        CBF_CURSOR_PROGRESS => CursorIcon::Progress,
        CBF_CURSOR_NOT_ALLOWED => CursorIcon::NotAllowed,
        CBF_CURSOR_CONTEXT_MENU => CursorIcon::ContextMenu,
        CBF_CURSOR_CELL => CursorIcon::Cell,
        CBF_CURSOR_VERTICAL_TEXT => CursorIcon::VerticalText,
        CBF_CURSOR_ALIAS => CursorIcon::Alias,
        CBF_CURSOR_COPY => CursorIcon::Copy,
        CBF_CURSOR_NO_DROP => CursorIcon::NoDrop,
        CBF_CURSOR_GRAB => CursorIcon::Grab,
        CBF_CURSOR_GRABBING => CursorIcon::Grabbing,
        CBF_CURSOR_ALL_SCROLL => CursorIcon::AllScroll,
        CBF_CURSOR_ZOOM_IN => CursorIcon::ZoomIn,
        CBF_CURSOR_ZOOM_OUT => CursorIcon::ZoomOut,
        CBF_CURSOR_E_RESIZE => CursorIcon::EResize,
        CBF_CURSOR_N_RESIZE => CursorIcon::NResize,
        CBF_CURSOR_NE_RESIZE => CursorIcon::NeResize,
        CBF_CURSOR_NW_RESIZE => CursorIcon::NwResize,
        CBF_CURSOR_S_RESIZE => CursorIcon::SResize,
        CBF_CURSOR_SE_RESIZE => CursorIcon::SeResize,
        CBF_CURSOR_SW_RESIZE => CursorIcon::SwResize,
        CBF_CURSOR_W_RESIZE => CursorIcon::WResize,
        CBF_CURSOR_EW_RESIZE => CursorIcon::EwResize,
        CBF_CURSOR_NS_RESIZE => CursorIcon::NsResize,
        CBF_CURSOR_NESW_RESIZE => CursorIcon::NeswResize,
        CBF_CURSOR_NWSE_RESIZE => CursorIcon::NwseResize,
        CBF_CURSOR_COL_RESIZE => CursorIcon::ColResize,
        CBF_CURSOR_ROW_RESIZE => CursorIcon::RowResize,
        _ => CursorIcon::Default,
    }
}

fn parse_surface_handle(handle: CbfSurfaceHandle) -> Result<SurfaceHandle, Error> {
    match handle.kind {
        CBF_SURFACE_HANDLE_MAC_CA_CONTEXT_ID => {
            Ok(SurfaceHandle::MacCaContextId(handle.ca_context_id))
        }
        CBF_SURFACE_HANDLE_WINDOWS_HWND => {
            unimplemented!("Windows HWND surface handle parsing not implemented yet")
        }
        _ => Err(Error::InvalidEvent),
    }
}

pub(super) fn key_event_type_to_ffi(value: ChromeKeyEventType) -> u8 {
    match value {
        ChromeKeyEventType::RawKeyDown => CBF_KEY_EVENT_RAW_KEY_DOWN,
        ChromeKeyEventType::KeyDown => CBF_KEY_EVENT_KEY_DOWN,
        ChromeKeyEventType::KeyUp => CBF_KEY_EVENT_KEY_UP,
        ChromeKeyEventType::Char => CBF_KEY_EVENT_CHAR,
    }
}

pub(super) fn mouse_event_type_to_ffi(value: ChromeMouseEventType) -> u8 {
    match value {
        ChromeMouseEventType::Down => CBF_MOUSE_EVENT_DOWN,
        ChromeMouseEventType::Up => CBF_MOUSE_EVENT_UP,
        ChromeMouseEventType::Move => CBF_MOUSE_EVENT_MOVE,
        ChromeMouseEventType::Enter => CBF_MOUSE_EVENT_ENTER,
        ChromeMouseEventType::Leave => CBF_MOUSE_EVENT_LEAVE,
    }
}

fn mouse_event_type_from_ffi(value: u8) -> ChromeMouseEventType {
    match value {
        CBF_MOUSE_EVENT_DOWN => ChromeMouseEventType::Down,
        CBF_MOUSE_EVENT_UP => ChromeMouseEventType::Up,
        CBF_MOUSE_EVENT_MOVE => ChromeMouseEventType::Move,
        CBF_MOUSE_EVENT_ENTER => ChromeMouseEventType::Enter,
        CBF_MOUSE_EVENT_LEAVE => ChromeMouseEventType::Leave,
        _ => ChromeMouseEventType::Move,
    }
}

pub(super) fn mouse_button_to_ffi(value: ChromeMouseButton) -> u8 {
    match value {
        ChromeMouseButton::None => CBF_MOUSE_BUTTON_NONE,
        ChromeMouseButton::Left => CBF_MOUSE_BUTTON_LEFT,
        ChromeMouseButton::Middle => CBF_MOUSE_BUTTON_MIDDLE,
        ChromeMouseButton::Right => CBF_MOUSE_BUTTON_RIGHT,
        ChromeMouseButton::Back => CBF_MOUSE_BUTTON_BACK,
        ChromeMouseButton::Forward => CBF_MOUSE_BUTTON_FORWARD,
    }
}

fn mouse_button_from_ffi(value: u8) -> ChromeMouseButton {
    match value {
        CBF_MOUSE_BUTTON_LEFT => ChromeMouseButton::Left,
        CBF_MOUSE_BUTTON_MIDDLE => ChromeMouseButton::Middle,
        CBF_MOUSE_BUTTON_RIGHT => ChromeMouseButton::Right,
        CBF_MOUSE_BUTTON_BACK => ChromeMouseButton::Back,
        CBF_MOUSE_BUTTON_FORWARD => ChromeMouseButton::Forward,
        _ => ChromeMouseButton::None,
    }
}

pub(super) fn pointer_type_to_ffi(value: ChromePointerType) -> u8 {
    match value {
        ChromePointerType::Unknown => CBF_POINTER_TYPE_UNKNOWN,
        ChromePointerType::Mouse => CBF_POINTER_TYPE_MOUSE,
        ChromePointerType::Pen => CBF_POINTER_TYPE_PEN,
        ChromePointerType::Touch => CBF_POINTER_TYPE_TOUCH,
        ChromePointerType::Eraser => CBF_POINTER_TYPE_ERASER,
    }
}

fn pointer_type_from_ffi(value: u8) -> ChromePointerType {
    match value {
        CBF_POINTER_TYPE_MOUSE => ChromePointerType::Mouse,
        CBF_POINTER_TYPE_PEN => ChromePointerType::Pen,
        CBF_POINTER_TYPE_TOUCH => ChromePointerType::Touch,
        CBF_POINTER_TYPE_ERASER => ChromePointerType::Eraser,
        _ => ChromePointerType::Unknown,
    }
}

pub(super) fn scroll_granularity_to_ffi(value: ChromeScrollGranularity) -> u8 {
    match value {
        ChromeScrollGranularity::PrecisePixel => CBF_SCROLL_BY_PRECISE_PIXEL,
        ChromeScrollGranularity::Pixel => CBF_SCROLL_BY_PIXEL,
        ChromeScrollGranularity::Line => CBF_SCROLL_BY_LINE,
        ChromeScrollGranularity::Page => CBF_SCROLL_BY_PAGE,
        ChromeScrollGranularity::Document => CBF_SCROLL_BY_DOCUMENT,
    }
}

fn scroll_granularity_from_ffi(value: u8) -> ChromeScrollGranularity {
    match value {
        CBF_SCROLL_BY_PRECISE_PIXEL => ChromeScrollGranularity::PrecisePixel,
        CBF_SCROLL_BY_PIXEL => ChromeScrollGranularity::Pixel,
        CBF_SCROLL_BY_LINE => ChromeScrollGranularity::Line,
        CBF_SCROLL_BY_PAGE => ChromeScrollGranularity::Page,
        CBF_SCROLL_BY_DOCUMENT => ChromeScrollGranularity::Document,
        _ => ChromeScrollGranularity::Pixel,
    }
}

fn ime_text_span_type_to_ffi(value: ChromeImeTextSpanType) -> u8 {
    match value {
        ChromeImeTextSpanType::Composition => CBF_IME_TEXT_SPAN_TYPE_COMPOSITION,
        ChromeImeTextSpanType::Suggestion => CBF_IME_TEXT_SPAN_TYPE_SUGGESTION,
        ChromeImeTextSpanType::MisspellingSuggestion => {
            CBF_IME_TEXT_SPAN_TYPE_MISSPELLING_SUGGESTION
        }
        ChromeImeTextSpanType::Autocorrect => CBF_IME_TEXT_SPAN_TYPE_AUTOCORRECT,
        ChromeImeTextSpanType::GrammarSuggestion => CBF_IME_TEXT_SPAN_TYPE_GRAMMAR_SUGGESTION,
    }
}

fn ime_text_span_thickness_to_ffi(value: ChromeImeTextSpanThickness) -> u8 {
    match value {
        ChromeImeTextSpanThickness::None => CBF_IME_TEXT_SPAN_THICKNESS_NONE,
        ChromeImeTextSpanThickness::Thin => CBF_IME_TEXT_SPAN_THICKNESS_THIN,
        ChromeImeTextSpanThickness::Thick => CBF_IME_TEXT_SPAN_THICKNESS_THICK,
    }
}

fn ime_text_span_underline_style_to_ffi(value: ChromeImeTextSpanUnderlineStyle) -> u8 {
    match value {
        ChromeImeTextSpanUnderlineStyle::None => CBF_IME_TEXT_SPAN_UNDERLINE_STYLE_NONE,
        ChromeImeTextSpanUnderlineStyle::Solid => CBF_IME_TEXT_SPAN_UNDERLINE_STYLE_SOLID,
        ChromeImeTextSpanUnderlineStyle::Dot => CBF_IME_TEXT_SPAN_UNDERLINE_STYLE_DOT,
        ChromeImeTextSpanUnderlineStyle::Dash => CBF_IME_TEXT_SPAN_UNDERLINE_STYLE_DASH,
        ChromeImeTextSpanUnderlineStyle::Squiggle => CBF_IME_TEXT_SPAN_UNDERLINE_STYLE_SQUIGGLE,
    }
}

fn chrome_ime_text_span_style_from_span(span: &ChromeImeTextSpan) -> ChromeImeTextSpanStyle {
    span.chrome_style.clone().unwrap_or_default()
}

pub(super) fn ime_range_to_ffi(value: &Option<ChromeImeTextRange>) -> (i32, i32) {
    match value {
        Some(range) => (range.start, range.end),
        // Sentinel for "no replacement range"; C++ side treats (-1, -1) as null.
        None => (-1, -1),
    }
}

pub(super) fn to_ffi_ime_text_spans(spans: &[ChromeImeTextSpan]) -> Vec<CbfImeTextSpan> {
    spans
        .iter()
        .map(|span| {
            let chrome_style = chrome_ime_text_span_style_from_span(span);
            CbfImeTextSpan {
                r#type: ime_text_span_type_to_ffi(span.r#type),
                start_offset: span.start_offset,
                end_offset: span.end_offset,
                underline_color: chrome_style.underline_color,
                thickness: ime_text_span_thickness_to_ffi(chrome_style.thickness),
                underline_style: ime_text_span_underline_style_to_ffi(chrome_style.underline_style),
                text_color: chrome_style.text_color,
                background_color: chrome_style.background_color,
                suggestion_highlight_color: chrome_style.suggestion_highlight_color,
                remove_on_finish_composing: chrome_style.remove_on_finish_composing,
                interim_char_selection: chrome_style.interim_char_selection,
                should_hide_suggestion_menu: chrome_style.should_hide_suggestion_menu,
            }
        })
        .collect()
}

#[cfg(target_os = "macos")]
pub fn convert_nsevent_to_key_event(
    browsing_context_id: u64,
    nsevent: NonNull<c_void>,
) -> KeyEvent {
    KeyEvent::from(convert_nsevent_to_chrome_key_event(
        browsing_context_id,
        nsevent,
    ))
}

#[cfg(target_os = "macos")]
pub fn convert_nsevent_to_chrome_key_event(
    browsing_context_id: u64,
    nsevent: NonNull<c_void>,
) -> ChromeKeyEvent {
    let mut ffi_event = CbfKeyEvent::default();
    unsafe {
        cbf_bridge_convert_nsevent(nsevent.as_ptr(), browsing_context_id, &mut ffi_event);
    }

    let event = ChromeKeyEvent {
        type_: match ffi_event.r#type {
            CBF_KEY_EVENT_RAW_KEY_DOWN => ChromeKeyEventType::RawKeyDown,
            CBF_KEY_EVENT_KEY_DOWN => ChromeKeyEventType::KeyDown,
            CBF_KEY_EVENT_KEY_UP => ChromeKeyEventType::KeyUp,
            CBF_KEY_EVENT_CHAR => ChromeKeyEventType::Char,
            _ => ChromeKeyEventType::RawKeyDown,
        },
        modifiers: ffi_event.modifiers,
        windows_key_code: ffi_event.windows_key_code,
        native_key_code: ffi_event.native_key_code,
        dom_code: if ffi_event.dom_code.is_null() {
            None
        } else {
            Some(c_string_to_string(ffi_event.dom_code as *mut _))
        },
        dom_key: if ffi_event.dom_key.is_null() {
            None
        } else {
            Some(c_string_to_string(ffi_event.dom_key as *mut _))
        },
        text: if ffi_event.text.is_null() {
            None
        } else {
            Some(c_string_to_string(ffi_event.text as *mut _))
        },
        unmodified_text: if ffi_event.unmodified_text.is_null() {
            None
        } else {
            Some(c_string_to_string(ffi_event.unmodified_text as *mut _))
        },
        auto_repeat: ffi_event.auto_repeat,
        is_keypad: ffi_event.is_keypad,
        is_system_key: ffi_event.is_system_key,
        location: ffi_event.location,
    };

    unsafe {
        cbf_bridge_free_converted_key_event(&mut ffi_event);
    }

    event
}

#[cfg(target_os = "macos")]
pub fn convert_nspasteboard_to_drag_data(nspasteboard: NonNull<c_void>) -> DragData {
    let mut ffi_data = CbfDragData::default();
    unsafe {
        cbf_bridge_convert_nspasteboard_to_drag_data(nspasteboard.as_ptr(), &mut ffi_data);
    }

    let drag_data = DragData {
        text: c_string_to_string(ffi_data.text),
        html: c_string_to_string(ffi_data.html),
        html_base_url: c_string_to_string(ffi_data.html_base_url),
        url_infos: parse_drag_url_infos(ffi_data.url_infos)
            .into_iter()
            .map(Into::into)
            .collect(),
        filenames: parse_string_list(ffi_data.filenames),
        file_mime_types: parse_string_list(ffi_data.file_mime_types),
        custom_data: parse_string_pair_list(ffi_data.custom_data),
    };

    unsafe {
        cbf_bridge_free_converted_drag_data(&mut ffi_data);
    }

    drag_data
}

#[cfg(target_os = "macos")]
pub fn convert_nsevent_to_mouse_event(
    browsing_context_id: u64,
    nsevent: NonNull<c_void>,
    nsview: NonNull<c_void>,
    pointer_type: PointerType,
    unaccelerated_movement: bool,
) -> MouseEvent {
    convert_nsevent_to_chrome_mouse_event(
        browsing_context_id,
        nsevent,
        nsview,
        pointer_type.into(),
        unaccelerated_movement,
    )
    .into()
}

#[cfg(target_os = "macos")]
pub fn convert_nsevent_to_chrome_mouse_event(
    browsing_context_id: u64,
    nsevent: NonNull<c_void>,
    nsview: NonNull<c_void>,
    pointer_type: ChromePointerType,
    unaccelerated_movement: bool,
) -> ChromeMouseEvent {
    let mut ffi_event = CbfMouseEvent::default();
    unsafe {
        cbf_bridge_convert_nsevent_to_mouse_event(
            nsevent.as_ptr(),
            nsview.as_ptr(),
            browsing_context_id,
            pointer_type_to_ffi(pointer_type),
            unaccelerated_movement,
            &mut ffi_event,
        );
    }

    ChromeMouseEvent {
        type_: mouse_event_type_from_ffi(ffi_event.r#type),
        modifiers: ffi_event.modifiers,
        button: mouse_button_from_ffi(ffi_event.button),
        click_count: ffi_event.click_count,
        position_in_widget_x: ffi_event.position_in_widget_x,
        position_in_widget_y: ffi_event.position_in_widget_y,
        position_in_screen_x: ffi_event.position_in_screen_x,
        position_in_screen_y: ffi_event.position_in_screen_y,
        movement_x: ffi_event.movement_x,
        movement_y: ffi_event.movement_y,
        is_raw_movement_event: ffi_event.is_raw_movement_event,
        pointer_type: pointer_type_from_ffi(ffi_event.pointer_type),
    }
}

#[cfg(target_os = "macos")]
pub fn convert_nsevent_to_mouse_wheel_event(
    browsing_context_id: u64,
    nsevent: NonNull<c_void>,
    nsview: NonNull<c_void>,
) -> MouseWheelEvent {
    MouseWheelEvent::from(convert_nsevent_to_chrome_mouse_wheel_event(
        browsing_context_id,
        nsevent,
        nsview,
    ))
}

#[cfg(target_os = "macos")]
pub fn convert_nsevent_to_chrome_mouse_wheel_event(
    browsing_context_id: u64,
    nsevent: NonNull<c_void>,
    nsview: NonNull<c_void>,
) -> ChromeMouseWheelEvent {
    let mut ffi_event = CbfMouseWheelEvent::default();
    unsafe {
        cbf_bridge_convert_nsevent_to_mouse_wheel_event(
            nsevent.as_ptr(),
            nsview.as_ptr(),
            browsing_context_id,
            &mut ffi_event,
        );
    }

    ChromeMouseWheelEvent {
        modifiers: ffi_event.modifiers,
        position_in_widget_x: ffi_event.position_in_widget_x,
        position_in_widget_y: ffi_event.position_in_widget_y,
        position_in_screen_x: ffi_event.position_in_screen_x,
        position_in_screen_y: ffi_event.position_in_screen_y,
        movement_x: ffi_event.movement_x,
        movement_y: ffi_event.movement_y,
        is_raw_movement_event: ffi_event.is_raw_movement_event,
        delta_x: ffi_event.delta_x,
        delta_y: ffi_event.delta_y,
        wheel_ticks_x: ffi_event.wheel_ticks_x,
        wheel_ticks_y: ffi_event.wheel_ticks_y,
        phase: ffi_event.phase,
        momentum_phase: ffi_event.momentum_phase,
        delta_units: scroll_granularity_from_ffi(ffi_event.delta_units),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use cbf_chrome_sys::ffi::*;

    use crate::data::{
        extension::ChromeIconData,
        ids::{PopupId, TabId},
        prompt_ui::{
            PromptUiCloseReason, PromptUiDialogResult, PromptUiExtensionInstallResult, PromptUiId,
            PromptUiKind, PromptUiPermissionType, PromptUiResolution, PromptUiResolutionResult,
        },
        tab_open::TabOpenResult,
    };

    use super::{IpcEvent, parse_event};

    fn make_event(kind: u8) -> CbfBridgeEvent {
        CbfBridgeEvent {
            kind,
            ..Default::default()
        }
    }

    fn leaked_c_string(value: &str) -> *mut i8 {
        CString::new(value).unwrap().into_raw()
    }

    #[test]
    fn parse_event_tab_created_maps_tab_id() {
        let mut event = make_event(CBF_EVENT_TAB_CREATED);
        event.tab_id = 7;
        event.request_id = 11;

        let parsed = parse_event(event).expect("tab created should parse");
        assert!(matches!(
            parsed,
            IpcEvent::TabCreated {
                browsing_context_id,
                request_id,
                ..
            } if browsing_context_id == TabId::new(7) && request_id == 11
        ));
    }

    #[test]
    fn parse_event_shutdown_blocked_maps_dirty_tab_ids() {
        let dirty_ids = [2_u64, 3_u64];
        let mut event = make_event(CBF_EVENT_SHUTDOWN_BLOCKED);
        event.request_id = 9;
        event.dirty_tab_ids = CbfTabIdList {
            items: dirty_ids.as_ptr(),
            len: dirty_ids.len() as u32,
        };

        let parsed = parse_event(event).expect("shutdown blocked should parse");
        assert!(matches!(
            parsed,
            IpcEvent::ShutdownBlocked {
                request_id,
                dirty_browsing_context_ids
            } if request_id == 9
                && dirty_browsing_context_ids == vec![TabId::new(2), TabId::new(3)]
        ));
    }

    #[test]
    fn parse_event_extension_popup_ime_bounds_maps_popup_id() {
        let mut event = make_event(CBF_EVENT_EXTENSION_POPUP_IME_BOUNDS_UPDATED);
        event.tab_id = 44;
        event.extension_popup_id = 88;
        event.ime_bounds.has_selection = true;
        event.ime_bounds.selection.range_start = 1;
        event.ime_bounds.selection.range_end = 1;
        event.ime_bounds.selection.caret_rect = CbfRect {
            x: 10,
            y: 20,
            width: 2,
            height: 16,
        };
        event.ime_bounds.selection.first_selection_rect = CbfRect {
            x: 10,
            y: 20,
            width: 2,
            height: 16,
        };

        let parsed = parse_event(event).expect("popup ime bounds should parse");
        assert!(matches!(
            parsed,
            IpcEvent::ExtensionPopupImeBoundsUpdated {
                browsing_context_id,
                popup_id,
                update,
                ..
            } if browsing_context_id == TabId::new(44)
                && popup_id == PopupId::new(88)
                && update.selection.as_ref().is_some_and(|selection| selection.range_start == 1)
        ));
    }

    #[test]
    fn parse_event_find_reply_maps_selection_rect() {
        let mut event = make_event(CBF_EVENT_FIND_REPLY);
        event.tab_id = 52;
        event.request_id = 7;
        event.find_number_of_matches = 9;
        event.find_active_match_ordinal = 3;
        event.find_selection_rect = CbfRect {
            x: 10,
            y: 11,
            width: 12,
            height: 13,
        };
        event.find_final_update = true;

        let parsed = parse_event(event).expect("find reply should parse");
        assert!(matches!(
            parsed,
            IpcEvent::FindReply {
                browsing_context_id,
                request_id,
                number_of_matches,
                active_match_ordinal,
                selection_rect,
                final_update,
                ..
            } if browsing_context_id == TabId::new(52)
                && request_id == 7
                && number_of_matches == 9
                && active_match_ordinal == 3
                && selection_rect.x == 10
                && selection_rect.height == 13
                && final_update
        ));
    }

    #[test]
    fn parse_event_tab_open_resolved_maps_target_tab_id() {
        let mut event = make_event(CBF_EVENT_TAB_OPEN_RESOLVED);
        event.request_id = 55;
        event.tab_open_result_kind = CBF_TAB_OPEN_RESULT_OPENED_NEW_CONTEXT;
        event.tab_open_has_target = true;
        event.tab_open_target_tab_id = 123;

        let parsed = parse_event(event).expect("tab open resolved should parse");
        assert!(matches!(
            parsed,
            IpcEvent::TabOpenResolved {
                request_id,
                result: TabOpenResult::OpenedNewTab { tab_id },
                ..
            } if request_id == 55 && tab_id.get() == 123
        ));
    }

    #[test]
    fn parse_event_prompt_ui_open_requested_maps_permission_kind() {
        let mut event = make_event(CBF_EVENT_PROMPT_UI_REQUESTED);
        event.prompt_ui_has_source_tab_id = true;
        event.prompt_ui_source_tab_id = 21;
        event.request_id = 99;
        event.prompt_ui_kind = CBF_PROMPT_UI_KIND_PERMISSION_PROMPT;
        event.prompt_ui_permission = CBF_PROMPT_UI_PERMISSION_TYPE_GEOLOCATION;
        let permission_key = CString::new("geolocation").unwrap();
        event.prompt_ui_permission_key = permission_key.as_ptr() as *mut _;

        let parsed = parse_event(event).expect("prompt ui requested should parse");
        assert!(matches!(
            parsed,
            IpcEvent::PromptUiOpenRequested {
                source_tab_id: Some(source_tab_id),
                request_id,
                kind: PromptUiKind::PermissionPrompt {
                    permission: PromptUiPermissionType::Geolocation,
                    permission_key: Some(ref permission_key),
                },
                ..
            } if source_tab_id == TabId::new(21)
                && request_id == 99
                && permission_key == "geolocation"
        ));
    }

    #[test]
    fn parse_event_prompt_ui_open_requested_maps_download_kind() {
        let mut event = make_event(CBF_EVENT_PROMPT_UI_REQUESTED);
        event.prompt_ui_has_source_tab_id = true;
        event.prompt_ui_source_tab_id = 31;
        event.request_id = 109;
        event.prompt_ui_kind = CBF_PROMPT_UI_KIND_DOWNLOAD_PROMPT;
        event.download_reason = CBF_DOWNLOAD_PROMPT_REASON_SAVE_AS;
        event.download_id = 55;
        let file_name = CString::new("sample.zip").unwrap();
        let suggested_path = CString::new("/tmp/sample.zip").unwrap();
        event.download_file_name = file_name.as_ptr() as *mut _;
        event.download_suggested_path = suggested_path.as_ptr() as *mut _;
        event.download_has_total_bytes = true;
        event.download_total_bytes = 1234;

        let parsed = parse_event(event).expect("download prompt requested should parse");
        assert!(matches!(
            parsed,
            IpcEvent::PromptUiOpenRequested {
                source_tab_id: Some(source_tab_id),
                request_id,
                kind: PromptUiKind::DownloadPrompt {
                    download_id,
                    file_name,
                    total_bytes: Some(1234),
                    suggested_path: Some(ref suggested_path),
                    reason: crate::data::download::ChromeDownloadPromptReason::SaveAs,
                },
                ..
            } if source_tab_id == TabId::new(31)
                && request_id == 109
                && download_id == crate::data::download::ChromeDownloadId::new(55)
                && file_name == "sample.zip"
                && suggested_path == "/tmp/sample.zip"
        ));
    }

    #[test]
    fn parse_event_prompt_ui_open_requested_maps_unknown_download_reason() {
        let mut event = make_event(CBF_EVENT_PROMPT_UI_REQUESTED);
        event.tab_id = 31;
        event.request_id = 110;
        event.prompt_ui_kind = CBF_PROMPT_UI_KIND_DOWNLOAD_PROMPT;
        event.download_reason = 250;
        event.download_id = 56;
        let file_name = CString::new("sample-2.zip").unwrap();
        event.download_file_name = file_name.as_ptr() as *mut _;

        let parsed = parse_event(event).expect("download prompt requested should parse");
        assert!(matches!(
            parsed,
            IpcEvent::PromptUiOpenRequested {
                kind: PromptUiKind::DownloadPrompt {
                    reason: crate::data::download::ChromeDownloadPromptReason::Unknown,
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn parse_event_prompt_ui_resolved_maps_result() {
        let mut event = make_event(CBF_EVENT_PROMPT_UI_RESOLVED);
        event.prompt_ui_has_source_tab_id = true;
        event.prompt_ui_source_tab_id = 18;
        event.request_id = 77;
        event.prompt_ui_kind = CBF_PROMPT_UI_KIND_PERMISSION_PROMPT;
        event.prompt_ui_permission = CBF_PROMPT_UI_PERMISSION_TYPE_NOTIFICATIONS;
        event.prompt_ui_result = CBF_PROMPT_UI_RESOLUTION_RESULT_DENIED;
        let permission_key = CString::new("notifications").unwrap();
        event.prompt_ui_permission_key = permission_key.as_ptr() as *mut _;

        let parsed = parse_event(event).expect("prompt ui resolved should parse");
        assert!(matches!(
            parsed,
            IpcEvent::PromptUiResolved {
                source_tab_id: Some(source_tab_id),
                request_id,
                resolution: PromptUiResolution::PermissionPrompt {
                    permission: PromptUiPermissionType::Notifications,
                    permission_key: Some(ref permission_key),
                    result: PromptUiResolutionResult::Denied
                },
                ..
            } if source_tab_id == TabId::new(18)
                && request_id == 77
                && permission_key == "notifications"
        ));
    }

    #[test]
    fn parse_event_prompt_ui_opened_maps_extension_kind_and_metadata() {
        let mut event = make_event(CBF_EVENT_PROMPT_UI_OPENED);
        event.prompt_ui_has_source_tab_id = true;
        event.prompt_ui_source_tab_id = 12;
        event.prompt_ui_id = 44;
        event.prompt_ui_kind = CBF_PROMPT_UI_KIND_EXTENSION_INSTALL_PROMPT;
        event.prompt_ui_title = CString::new("Install extension").unwrap().into_raw();
        event.prompt_ui_modal = true;
        event.extension_id = CString::new("ext-id").unwrap().into_raw();
        event.extension_name = CString::new("Ext").unwrap().into_raw();

        let permission_names = [
            CString::new("tabs").unwrap(),
            CString::new("storage").unwrap(),
        ];
        let permission_ptrs: Vec<*mut i8> = permission_names
            .iter()
            .map(|s| s.as_ptr() as *mut i8)
            .collect();
        event.permission_names = CbfStringList {
            items: permission_ptrs.as_ptr() as *mut _,
            len: permission_ptrs.len() as u32,
        };

        let parsed = parse_event(event).expect("prompt ui opened should parse");
        assert!(matches!(
            parsed,
            IpcEvent::PromptUiOpened {
                source_tab_id: Some(source_tab_id),
                prompt_ui_id,
                kind: PromptUiKind::ExtensionInstallPrompt { extension_id, extension_name, permission_names },
                title: Some(ref title),
                modal,
                ..
            } if source_tab_id == TabId::new(12)
                && prompt_ui_id == PromptUiId::new(44)
                && extension_id == "ext-id"
                && extension_name == "Ext"
                && permission_names == vec!["tabs".to_string(), "storage".to_string()]
                && title == "Install extension"
                && modal
        ));
    }

    #[test]
    fn parse_event_extensions_listed_maps_png_icon() {
        let bytes = [1_u8, 2, 3];
        let extensions = [CbfExtensionInfo {
            id: leaked_c_string("ext"),
            name: leaked_c_string("Example"),
            version: leaked_c_string("1.0.0"),
            enabled: true,
            permission_names: CbfStringList::default(),
            icon: CbfIconData {
                kind: CBF_ICON_DATA_KIND_PNG,
                bytes: bytes.as_ptr(),
                len: bytes.len() as u32,
                ..Default::default()
            },
        }];
        let mut event = make_event(CBF_EVENT_EXTENSIONS_LISTED);
        event.extensions = CbfExtensionInfoList {
            items: extensions.as_ptr() as *mut _,
            len: 1,
        };

        let parsed = parse_event(event).expect("extensions listed should parse");
        assert!(matches!(
            parsed,
            IpcEvent::ExtensionsListed { extensions, .. }
                if matches!(
                    extensions.as_slice(),
                    [crate::data::extension::ChromeExtensionInfo {
                        icon: Some(ChromeIconData::Png(icon)),
                        ..
                    }] if icon == &vec![1, 2, 3]
                )
        ));
    }

    #[test]
    fn parse_event_extensions_listed_maps_missing_icon_to_none() {
        let extensions = [CbfExtensionInfo {
            id: leaked_c_string("ext"),
            name: leaked_c_string("Example"),
            version: leaked_c_string("1.0.0"),
            enabled: false,
            permission_names: CbfStringList::default(),
            icon: CbfIconData {
                kind: CBF_ICON_DATA_KIND_PNG,
                len: 0,
                bytes: std::ptr::null(),
                ..Default::default()
            },
        }];
        let mut event = make_event(CBF_EVENT_EXTENSIONS_LISTED);
        event.extensions = CbfExtensionInfoList {
            items: extensions.as_ptr() as *mut _,
            len: 1,
        };

        let parsed = parse_event(event).expect("extensions listed should parse");
        assert!(matches!(
            parsed,
            IpcEvent::ExtensionsListed { extensions, .. }
                if matches!(
                    extensions.as_slice(),
                    [crate::data::extension::ChromeExtensionInfo { icon: None, .. }]
                )
        ));
    }

    #[test]
    fn parse_event_extensions_listed_maps_binary_icon() {
        let bytes = [4_u8, 5, 6];
        let extensions = [CbfExtensionInfo {
            id: leaked_c_string("ext"),
            name: leaked_c_string("Example"),
            version: leaked_c_string("1.0.0"),
            enabled: true,
            permission_names: CbfStringList::default(),
            icon: CbfIconData {
                kind: CBF_ICON_DATA_KIND_BINARY,
                bytes: bytes.as_ptr(),
                len: bytes.len() as u32,
                media_type: leaked_c_string("image/webp"),
                ..Default::default()
            },
        }];
        let mut event = make_event(CBF_EVENT_EXTENSIONS_LISTED);
        event.extensions = CbfExtensionInfoList {
            items: extensions.as_ptr() as *mut _,
            len: 1,
        };

        let parsed = parse_event(event).expect("extensions listed should parse");
        assert!(matches!(
            parsed,
            IpcEvent::ExtensionsListed { extensions, .. }
                if matches!(
                    extensions.as_slice(),
                    [crate::data::extension::ChromeExtensionInfo {
                        icon: Some(ChromeIconData::Binary { media_type: Some(media_type), bytes: icon_bytes }),
                        ..
                    }] if media_type == "image/webp" && icon_bytes == &vec![4, 5, 6]
                )
        ));
    }

    #[test]
    fn parse_event_prompt_ui_closed_maps_reason() {
        let mut event = make_event(CBF_EVENT_PROMPT_UI_CLOSED);
        event.prompt_ui_has_source_tab_id = true;
        event.prompt_ui_source_tab_id = 8;
        event.prompt_ui_id = 19;
        event.prompt_ui_kind = CBF_PROMPT_UI_KIND_PRINT_PREVIEW_DIALOG;
        event.prompt_ui_close_reason = CBF_PROMPT_UI_CLOSE_REASON_HOST_FORCED;

        let parsed = parse_event(event).expect("prompt ui closed should parse");
        assert!(matches!(
            parsed,
            IpcEvent::PromptUiClosed {
                source_tab_id: Some(source_tab_id),
                prompt_ui_id,
                kind: PromptUiKind::PrintPreviewDialog,
                reason: PromptUiCloseReason::HostForced,
                ..
            } if source_tab_id == TabId::new(8)
                && prompt_ui_id == PromptUiId::new(19)
        ));
    }

    #[test]
    fn parse_event_prompt_ui_resolved_maps_extension_install_result() {
        let mut event = make_event(CBF_EVENT_PROMPT_UI_RESOLVED);
        event.prompt_ui_has_source_tab_id = true;
        event.prompt_ui_source_tab_id = 99;
        event.request_id = 101;
        event.prompt_ui_kind = CBF_PROMPT_UI_KIND_EXTENSION_INSTALL_PROMPT;
        event.extension_id = CString::new("abc").unwrap().into_raw();
        event.prompt_ui_extension_install_result =
            CBF_PROMPT_UI_EXTENSION_INSTALL_RESULT_ACCEPTED_WITH_WITHHELD_PERMISSIONS;
        event.prompt_ui_extension_install_detail = CString::new("withheld").unwrap().into_raw();

        let parsed = parse_event(event).expect("prompt ui resolved should parse");
        assert!(matches!(
            parsed,
            IpcEvent::PromptUiResolved {
                source_tab_id: Some(source_tab_id),
                request_id,
                resolution: PromptUiResolution::ExtensionInstallPrompt {
                    extension_id,
                    result: PromptUiExtensionInstallResult::AcceptedWithWithheldPermissions,
                    detail: Some(ref detail),
                },
                ..
            } if source_tab_id == TabId::new(99)
                && request_id == 101
                && extension_id == "abc"
                && detail == "withheld"
        ));
    }

    #[test]
    fn parse_event_prompt_ui_resolved_maps_print_preview_result() {
        let mut event = make_event(CBF_EVENT_PROMPT_UI_RESOLVED);
        event.prompt_ui_has_source_tab_id = true;
        event.prompt_ui_source_tab_id = 41;
        event.request_id = 51;
        event.prompt_ui_kind = CBF_PROMPT_UI_KIND_PRINT_PREVIEW_DIALOG;
        event.prompt_ui_result = CBF_PROMPT_UI_DIALOG_RESULT_CANCELED;

        let parsed = parse_event(event).expect("prompt ui resolved should parse");
        assert!(matches!(
            parsed,
            IpcEvent::PromptUiResolved {
                source_tab_id: Some(source_tab_id),
                request_id,
                resolution: PromptUiResolution::PrintPreviewDialog {
                    result: PromptUiDialogResult::Canceled,
                },
                ..
            } if source_tab_id == TabId::new(41)
                && request_id == 51
        ));
    }

    #[test]
    fn parse_event_prompt_ui_requested_maps_extension_kind() {
        let mut event = make_event(CBF_EVENT_PROMPT_UI_REQUESTED);
        event.prompt_ui_has_source_tab_id = true;
        event.prompt_ui_source_tab_id = 64;
        event.request_id = 808;
        event.prompt_ui_kind = CBF_PROMPT_UI_KIND_EXTENSION_INSTALL_PROMPT;
        event.extension_id = CString::new("ext-aux").unwrap().into_raw();
        event.extension_name = CString::new("AuxExt").unwrap().into_raw();

        let parsed = parse_event(event).expect("prompt ui requested should parse");
        assert!(matches!(
            parsed,
            IpcEvent::PromptUiOpenRequested {
                source_tab_id: Some(source_tab_id),
                request_id,
                kind: PromptUiKind::ExtensionInstallPrompt { extension_id, extension_name, .. },
                ..
            } if source_tab_id == TabId::new(64)
                && request_id == 808
                && extension_id == "ext-aux"
                && extension_name == "AuxExt"
        ));
    }
}
