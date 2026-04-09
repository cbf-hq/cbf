#![allow(non_upper_case_globals)]

#[cfg(target_os = "macos")]
use std::{ffi::c_void, ptr::NonNull};

use cbf::data::dialog::DialogType;
use cbf::data::{
    drag::DragData,
    key::KeyEvent,
    mouse::{MouseEvent, MouseWheelEvent, PointerType},
};
use cbf_chrome_sys::{bridge::bridge, ffi::*};
use cursor_icon::CursorIcon;
use tracing::warn;

use super::{BridgeError, IpcEvent, utils::c_string_to_string};
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

pub(super) fn parse_event(event: CbfBridgeEvent) -> Result<IpcEvent, BridgeError> {
    match u32::from(event.kind) {
        CbfEventKind_kEventSurfaceHandleUpdated => {
            let handle = parse_surface_handle(event.surface_handle)?;

            Ok(IpcEvent::SurfaceHandleUpdated {
                profile_id: c_string_to_string(event.profile_id),
                browsing_context_id: TabId::new(event.tab_id),
                handle,
            })
        }
        CbfEventKind_kEventExtensionPopupOpened => Ok(IpcEvent::ExtensionPopupOpened {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            popup_id: event.extension_popup_id,
            extension_id: c_string_to_string(event.extension_id),
            title: c_string_to_string(event.title),
        }),
        CbfEventKind_kEventExtensionPopupSurfaceHandleUpdated => {
            let handle = parse_surface_handle(event.surface_handle)?;

            Ok(IpcEvent::ExtensionPopupSurfaceHandleUpdated {
                profile_id: c_string_to_string(event.profile_id),
                browsing_context_id: TabId::new(event.tab_id),
                popup_id: event.extension_popup_id,
                handle,
            })
        }
        CbfEventKind_kEventExtensionPopupPreferredSizeChanged => {
            Ok(IpcEvent::ExtensionPopupPreferredSizeChanged {
                profile_id: c_string_to_string(event.profile_id),
                browsing_context_id: TabId::new(event.tab_id),
                popup_id: event.extension_popup_id,
                width: event.width,
                height: event.height,
            })
        }
        CbfEventKind_kEventExtensionPopupContextMenuRequested => {
            Ok(IpcEvent::ExtensionPopupContextMenuRequested {
                profile_id: c_string_to_string(event.profile_id),
                browsing_context_id: TabId::new(event.tab_id),
                popup_id: PopupId::new(event.extension_popup_id),
                menu: parse_context_menu(event.context_menu),
            })
        }
        CbfEventKind_kEventExtensionPopupChoiceMenuRequested => {
            Ok(IpcEvent::ExtensionPopupChoiceMenuRequested {
                profile_id: c_string_to_string(event.profile_id),
                browsing_context_id: TabId::new(event.tab_id),
                popup_id: PopupId::new(event.extension_popup_id),
                request_id: event.request_id,
                menu: parse_choice_menu(event.choice_menu),
            })
        }
        CbfEventKind_kEventExtensionPopupCursorChanged => {
            Ok(IpcEvent::ExtensionPopupCursorChanged {
                profile_id: c_string_to_string(event.profile_id),
                browsing_context_id: TabId::new(event.tab_id),
                popup_id: PopupId::new(event.extension_popup_id),
                cursor_type: cursor_icon_from_ffi(event.cursor_type),
            })
        }
        CbfEventKind_kEventExtensionPopupTitleUpdated => Ok(IpcEvent::ExtensionPopupTitleUpdated {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            popup_id: PopupId::new(event.extension_popup_id),
            title: c_string_to_string(event.title),
        }),
        CbfEventKind_kEventExtensionPopupJavaScriptDialogRequested => {
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
        CbfEventKind_kEventExtensionPopupCloseRequested => {
            Ok(IpcEvent::ExtensionPopupCloseRequested {
                profile_id: c_string_to_string(event.profile_id),
                browsing_context_id: TabId::new(event.tab_id),
                popup_id: PopupId::new(event.extension_popup_id),
            })
        }
        CbfEventKind_kEventExtensionPopupRenderProcessGone => {
            Ok(IpcEvent::ExtensionPopupRenderProcessGone {
                profile_id: c_string_to_string(event.profile_id),
                browsing_context_id: TabId::new(event.tab_id),
                popup_id: PopupId::new(event.extension_popup_id),
                crashed: event.crashed,
            })
        }
        CbfEventKind_kEventExtensionPopupClosed => Ok(IpcEvent::ExtensionPopupClosed {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            popup_id: event.extension_popup_id,
        }),
        CbfEventKind_kEventTabCreated => Ok(IpcEvent::TabCreated {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            request_id: event.request_id,
        }),
        CbfEventKind_kEventDevToolsOpened => Ok(IpcEvent::DevToolsOpened {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            inspected_browsing_context_id: TabId::new(event.inspected_tab_id),
        }),
        CbfEventKind_kEventImeBoundsUpdated => Ok(IpcEvent::ImeBoundsUpdated {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            update: parse_ime_bounds(event.ime_bounds),
        }),
        CbfEventKind_kEventExtensionPopupImeBoundsUpdated => {
            Ok(IpcEvent::ExtensionPopupImeBoundsUpdated {
                profile_id: c_string_to_string(event.profile_id),
                browsing_context_id: TabId::new(event.tab_id),
                popup_id: PopupId::new(event.extension_popup_id),
                update: parse_ime_bounds(event.ime_bounds),
            })
        }
        CbfEventKind_kEventShutdownBlocked => Ok(IpcEvent::ShutdownBlocked {
            request_id: event.request_id,
            dirty_browsing_context_id: TabId::new(event.tab_id),
        }),
        CbfEventKind_kEventShutdownProceeding => Ok(IpcEvent::ShutdownProceeding {
            request_id: event.request_id,
        }),
        CbfEventKind_kEventShutdownCancelled => Ok(IpcEvent::ShutdownCancelled {
            request_id: event.request_id,
        }),
        CbfEventKind_kEventContextMenuRequested => Ok(IpcEvent::ContextMenuRequested {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            menu: parse_context_menu(event.context_menu),
        }),
        CbfEventKind_kEventChoiceMenuRequested => Ok(IpcEvent::ChoiceMenuRequested {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            request_id: event.request_id,
            menu: parse_choice_menu(event.choice_menu),
        }),
        CbfEventKind_kEventTabOpenRequested => Ok(IpcEvent::TabOpenRequested {
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
        CbfEventKind_kEventTabOpenResolved => Ok(IpcEvent::TabOpenResolved {
            profile_id: c_string_to_string(event.profile_id),
            request_id: event.request_id,
            result: tab_open_result_from_ffi(
                event.tab_open_result_kind,
                event.tab_open_has_target,
                event.tab_open_target_tab_id,
            ),
        }),
        CbfEventKind_kEventNavigationStateChanged => Ok(IpcEvent::NavigationStateChanged {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            url: c_string_to_string(event.url),
            can_go_back: event.can_go_back,
            can_go_forward: event.can_go_forward,
            is_loading: event.is_loading,
        }),
        CbfEventKind_kEventCursorChanged => Ok(IpcEvent::CursorChanged {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            cursor_type: cursor_icon_from_ffi(event.cursor_type),
        }),
        CbfEventKind_kEventTitleUpdated => Ok(IpcEvent::TitleUpdated {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            title: c_string_to_string(event.title),
        }),
        CbfEventKind_kEventFaviconUrlUpdated => Ok(IpcEvent::FaviconUrlUpdated {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            url: c_string_to_string(event.favicon_url),
        }),
        CbfEventKind_kEventBeforeUnloadDialogRequested => {
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
        CbfEventKind_kEventJavaScriptDialogRequested => Ok(IpcEvent::JavaScriptDialogRequested {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            request_id: event.request_id,
            r#type: javascript_dialog_type_from_ffi(event.javascript_dialog_type),
            message: c_string_to_string(event.message),
            default_prompt_text: optional_string_from_ffi(event.default_prompt_text),
            reason: beforeunload_reason_from_ffi(event.beforeunload_reason),
        }),
        CbfEventKind_kEventTabClosed => Ok(IpcEvent::TabClosed {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
        }),
        CbfEventKind_kEventTabResizeAcknowledged => Ok(IpcEvent::TabResizeAcknowledged {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
        }),
        CbfEventKind_kEventTabDomHtmlRead => Ok(IpcEvent::TabDomHtmlRead {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            request_id: event.request_id,
            html: c_string_to_string(event.dom_html),
        }),
        CbfEventKind_kEventFindReply => Ok(IpcEvent::FindReply {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            request_id: event.request_id,
            number_of_matches: event.find_number_of_matches,
            active_match_ordinal: event.find_active_match_ordinal,
            selection_rect: find_rect_from_ffi(event.find_selection_rect),
            final_update: event.find_final_update,
        }),
        CbfEventKind_kEventTabIpcMessageReceived => Ok(IpcEvent::TabIpcMessageReceived {
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
        CbfEventKind_kEventDragStartRequested => {
            let profile_id = c_string_to_string(event.profile_id);
            let request = parse_drag_start_request(event.drag_start_request);
            Ok(IpcEvent::DragStartRequested {
                browsing_context_id: request.browsing_context_id,
                profile_id,
                request,
            })
        }
        CbfEventKind_kEventExternalDragOperationChanged => {
            Ok(IpcEvent::ExternalDragOperationChanged {
                profile_id: c_string_to_string(event.profile_id),
                browsing_context_id: TabId::new(event.tab_id),
                operation: drag_operation_from_ffi(event.drag_operation),
            })
        }
        CbfEventKind_kEventExtensionsListed => Ok(IpcEvent::ExtensionsListed {
            profile_id: c_string_to_string(event.profile_id),
            extensions: parse_extension_list(event.extensions),
        }),
        CbfEventKind_kEventPromptUiRequested => Ok(IpcEvent::PromptUiOpenRequested {
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
        CbfEventKind_kEventPromptUiResolved => Ok(IpcEvent::PromptUiResolved {
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
                event.extension_install_prompt_result,
                event.extension_uninstall_prompt_result,
                event.extension_install_prompt_detail,
                event.prompt_ui_report_abuse,
                event.prompt_ui_repost_reason,
                event.prompt_ui_repost_target_url,
            ),
        }),
        CbfEventKind_kEventCustomSchemeRequestReceived => {
            Ok(IpcEvent::CustomSchemeRequestReceived {
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
            })
        }
        CbfEventKind_kEventExtensionRuntimeWarning => Ok(IpcEvent::ExtensionRuntimeWarning {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: TabId::new(event.tab_id),
            detail: c_string_to_string(event.extension_runtime_warning),
        }),
        CbfEventKind_kEventPromptUiOpened => Ok(IpcEvent::PromptUiOpened {
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
        CbfEventKind_kEventPromptUiClosed => Ok(IpcEvent::PromptUiClosed {
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
        CbfEventKind_kEventDownloadCreated => Ok(IpcEvent::DownloadCreated {
            profile_id: c_string_to_string(event.profile_id),
            download: parse_download_snapshot(event),
        }),
        CbfEventKind_kEventDownloadUpdated => Ok(IpcEvent::DownloadUpdated {
            profile_id: c_string_to_string(event.profile_id),
            download: parse_download_progress(event),
        }),
        CbfEventKind_kEventDownloadCompleted => Ok(IpcEvent::DownloadCompleted {
            profile_id: c_string_to_string(event.profile_id),
            download: parse_download_completion(event),
        }),
        _ => Err(BridgeError::InvalidEvent),
    }
}

fn tab_open_hint_from_ffi(value: u8) -> TabOpenHint {
    match u32::from(value) {
        CbfTabOpenHint_kCbfTabOpenHintCurrentContext => TabOpenHint::CurrentTab,
        CbfTabOpenHint_kCbfTabOpenHintNewForegroundContext => TabOpenHint::NewForegroundTab,
        CbfTabOpenHint_kCbfTabOpenHintNewBackgroundContext => TabOpenHint::NewBackgroundTab,
        CbfTabOpenHint_kCbfTabOpenHintNewWindow => TabOpenHint::NewWindow,
        CbfTabOpenHint_kCbfTabOpenHintPopup => TabOpenHint::Popup,
        _ => TabOpenHint::Unknown,
    }
}

fn tab_open_result_from_ffi(value: u8, has_target: bool, target_tab_id: u64) -> TabOpenResult {
    match u32::from(value) {
        CbfTabOpenResult_kCbfTabOpenResultOpenedNewContext => {
            if has_target {
                TabOpenResult::OpenedNewTab {
                    tab_id: TabId::new(target_tab_id),
                }
            } else {
                TabOpenResult::Aborted
            }
        }
        CbfTabOpenResult_kCbfTabOpenResultOpenedExistingContext => {
            if has_target {
                TabOpenResult::OpenedExistingTab {
                    tab_id: TabId::new(target_tab_id),
                }
            } else {
                TabOpenResult::Aborted
            }
        }
        CbfTabOpenResult_kCbfTabOpenResultDenied => TabOpenResult::Denied,
        CbfTabOpenResult_kCbfTabOpenResultAborted => TabOpenResult::Aborted,
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
    match u32::from(icon.kind) {
        CbfIconDataKind_kCbfIconDataKindNone => None,
        CbfIconDataKind_kCbfIconDataKindUrl => {
            let url = c_string_to_string(icon.url);
            if url.is_empty() {
                None
            } else {
                Some(ChromeIconData::Url(url))
            }
        }
        CbfIconDataKind_kCbfIconDataKindPng => {
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
        CbfIconDataKind_kCbfIconDataKindBinary => {
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
        item_type: choice_menu_item_type_from_ffi(item.type_),
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
        r#type: context_menu_item_type_from_ffi(item.type_),
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
    match u32::from(value) {
        CbfContextMenuItemType_kCbfMenuItemCommand => ChromeContextMenuItemType::Command,
        CbfContextMenuItemType_kCbfMenuItemCheck => ChromeContextMenuItemType::Check,
        CbfContextMenuItemType_kCbfMenuItemRadio => ChromeContextMenuItemType::Radio,
        CbfContextMenuItemType_kCbfMenuItemSeparator => ChromeContextMenuItemType::Separator,
        CbfContextMenuItemType_kCbfMenuItemButtonItem => ChromeContextMenuItemType::ButtonItem,
        CbfContextMenuItemType_kCbfMenuItemSubmenu => ChromeContextMenuItemType::Submenu,
        CbfContextMenuItemType_kCbfMenuItemActionableSubmenu => {
            ChromeContextMenuItemType::ActionableSubmenu
        }
        CbfContextMenuItemType_kCbfMenuItemHighlighted => ChromeContextMenuItemType::Highlighted,
        CbfContextMenuItemType_kCbfMenuItemTitle => ChromeContextMenuItemType::Title,
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
    match u32::from(value) {
        CbfBeforeUnloadReason_kCbfBeforeUnloadReasonCloseTab => {
            ChromeBeforeUnloadReason::CloseBrowsingContext
        }
        CbfBeforeUnloadReason_kCbfBeforeUnloadReasonNavigate => ChromeBeforeUnloadReason::Navigate,
        CbfBeforeUnloadReason_kCbfBeforeUnloadReasonReload => ChromeBeforeUnloadReason::Reload,
        CbfBeforeUnloadReason_kCbfBeforeUnloadReasonWindowClose => {
            ChromeBeforeUnloadReason::WindowClose
        }
        _ => ChromeBeforeUnloadReason::Unknown,
    }
}

fn javascript_dialog_type_from_ffi(value: u8) -> DialogType {
    match u32::from(value) {
        CbfJavaScriptDialogType_kCbfJavaScriptDialogAlert => DialogType::Alert,
        CbfJavaScriptDialogType_kCbfJavaScriptDialogConfirm => DialogType::Confirm,
        CbfJavaScriptDialogType_kCbfJavaScriptDialogPrompt => DialogType::Prompt,
        CbfJavaScriptDialogType_kCbfJavaScriptDialogBeforeUnload => DialogType::BeforeUnload,
        _ => DialogType::Alert,
    }
}

fn ipc_message_type_from_ffi(value: u8) -> TabIpcMessageType {
    match u32::from(value) {
        CbfIpcMessageType_kCbfIpcMessageRequest => TabIpcMessageType::Request,
        CbfIpcMessageType_kCbfIpcMessageResponse => TabIpcMessageType::Response,
        CbfIpcMessageType_kCbfIpcMessageEvent => TabIpcMessageType::Event,
        _ => TabIpcMessageType::Event,
    }
}

fn ipc_payload_from_ffi(
    kind: u8,
    text: *mut std::ffi::c_char,
    binary: *const u8,
    binary_len: u32,
) -> TabIpcPayload {
    match u32::from(kind) {
        CbfIpcPayloadKind_kCbfIpcPayloadBinary if !binary.is_null() && binary_len > 0 => {
            let bytes = unsafe { std::slice::from_raw_parts(binary, binary_len as usize) };
            TabIpcPayload::Binary(bytes.to_vec())
        }
        _ => TabIpcPayload::Text(c_string_to_string(text)),
    }
}

fn ipc_error_code_from_ffi(value: u8) -> Option<TabIpcErrorCode> {
    match u32::from(value) {
        CbfIpcErrorCode_kCbfIpcErrorNone => None,
        CbfIpcErrorCode_kCbfIpcErrorTimeout => Some(TabIpcErrorCode::Timeout),
        CbfIpcErrorCode_kCbfIpcErrorAborted => Some(TabIpcErrorCode::Aborted),
        CbfIpcErrorCode_kCbfIpcErrorDisconnected => Some(TabIpcErrorCode::Disconnected),
        CbfIpcErrorCode_kCbfIpcErrorIpcDisabled => Some(TabIpcErrorCode::IpcDisabled),
        CbfIpcErrorCode_kCbfIpcErrorContextClosed => Some(TabIpcErrorCode::ContextClosed),
        CbfIpcErrorCode_kCbfIpcErrorRemoteError => Some(TabIpcErrorCode::RemoteError),
        CbfIpcErrorCode_kCbfIpcErrorProtocolError => Some(TabIpcErrorCode::ProtocolError),
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
    match u32::from(value) {
        CbfExtensionInstallPromptResult_kCbfExtensionInstallPromptResultAccepted => {
            PromptUiExtensionInstallResult::Accepted
        }
        CbfExtensionInstallPromptResult_kCbfExtensionInstallPromptResultAcceptedWithWithheldPermissions => {
            PromptUiExtensionInstallResult::AcceptedWithWithheldPermissions
        }
        CbfExtensionInstallPromptResult_kCbfExtensionInstallPromptResultUserCanceled => {
            PromptUiExtensionInstallResult::UserCanceled
        }
        CbfExtensionInstallPromptResult_kCbfExtensionInstallPromptResultAborted => {
            PromptUiExtensionInstallResult::Aborted
        }
        _ => PromptUiExtensionInstallResult::Aborted,
    }
}

fn prompt_ui_extension_uninstall_result_from_ffi(value: u8) -> PromptUiExtensionUninstallResult {
    match u32::from(value) {
        CbfExtensionUninstallPromptResult_kCbfExtensionUninstallPromptResultAccepted => {
            PromptUiExtensionUninstallResult::Accepted
        }
        CbfExtensionUninstallPromptResult_kCbfExtensionUninstallPromptResultUserCanceled => {
            PromptUiExtensionUninstallResult::UserCanceled
        }
        CbfExtensionUninstallPromptResult_kCbfExtensionUninstallPromptResultAborted => {
            PromptUiExtensionUninstallResult::Aborted
        }
        CbfExtensionUninstallPromptResult_kCbfExtensionUninstallPromptResultFailed => {
            PromptUiExtensionUninstallResult::Failed
        }
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
    match u32::from(kind) {
        CbfPromptUiKind_kCbfPromptUiKindPermissionPrompt => PromptUiKind::PermissionPrompt {
            permission: prompt_ui_permission_from_ffi(permission),
            permission_key,
        },
        CbfPromptUiKind_kCbfPromptUiKindDownloadPrompt => PromptUiKind::DownloadPrompt {
            download_id: ChromeDownloadId::new(download_id),
            file_name: c_string_to_string(download_file_name),
            total_bytes: download_has_total_bytes.then_some(download_total_bytes),
            suggested_path: {
                let value = c_string_to_string(download_suggested_path);
                if value.is_empty() { None } else { Some(value) }
            },
            reason: download_prompt_reason_from_ffi(download_reason),
        },
        CbfPromptUiKind_kCbfPromptUiKindExtensionInstallPrompt => {
            PromptUiKind::ExtensionInstallPrompt {
                extension_id: c_string_to_string(extension_id),
                extension_name: c_string_to_string(extension_name),
                permission_names: parse_string_list(permission_names),
            }
        }
        CbfPromptUiKind_kCbfPromptUiKindExtensionUninstallPrompt => {
            PromptUiKind::ExtensionUninstallPrompt {
                extension_id: c_string_to_string(extension_id),
                extension_name: c_string_to_string(extension_name),
                triggering_extension_name: optional_string_from_ffi(triggering_extension_name),
                can_report_abuse,
            }
        }
        CbfPromptUiKind_kCbfPromptUiKindPrintPreviewDialog => PromptUiKind::PrintPreviewDialog,
        CbfPromptUiKind_kCbfPromptUiKindFormResubmissionPrompt => {
            PromptUiKind::FormResubmissionPrompt {
                reason: prompt_ui_form_resubmission_reason_from_ffi(repost_reason),
                target_url: optional_string_from_ffi(repost_target_url),
            }
        }
        _ => PromptUiKind::Unknown,
    }
}

#[allow(clippy::too_many_arguments)]
fn prompt_ui_dialog_result_from_ffi(value: u8) -> PromptUiDialogResult {
    match u32::from(value) {
        CbfPromptUiDialogResult_kCbfPromptUiDialogResultProceeded => {
            PromptUiDialogResult::Proceeded
        }
        CbfPromptUiDialogResult_kCbfPromptUiDialogResultCanceled => PromptUiDialogResult::Canceled,
        CbfPromptUiDialogResult_kCbfPromptUiDialogResultAborted => PromptUiDialogResult::Aborted,
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
    match u32::from(kind) {
        CbfPromptUiKind_kCbfPromptUiKindPermissionPrompt => PromptUiResolution::PermissionPrompt {
            permission: prompt_ui_permission_from_ffi(permission),
            permission_key,
            result: prompt_ui_resolution_result_from_ffi(permission_result),
        },
        CbfPromptUiKind_kCbfPromptUiKindDownloadPrompt => PromptUiResolution::DownloadPrompt {
            download_id: ChromeDownloadId::new(download_id),
            destination_path: {
                let value = c_string_to_string(download_destination_path);
                if value.is_empty() { None } else { Some(value) }
            },
            result: download_prompt_result_from_ffi(permission_result),
        },
        CbfPromptUiKind_kCbfPromptUiKindExtensionInstallPrompt => {
            PromptUiResolution::ExtensionInstallPrompt {
                extension_id: c_string_to_string(extension_id),
                result: prompt_ui_extension_install_result_from_ffi(extension_install_result),
                detail: {
                    let value = c_string_to_string(detail);
                    if value.is_empty() { None } else { Some(value) }
                },
            }
        }
        CbfPromptUiKind_kCbfPromptUiKindExtensionUninstallPrompt => {
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
        CbfPromptUiKind_kCbfPromptUiKindPrintPreviewDialog => {
            PromptUiResolution::PrintPreviewDialog {
                result: prompt_ui_dialog_result_from_ffi(permission_result),
            }
        }
        CbfPromptUiKind_kCbfPromptUiKindFormResubmissionPrompt => {
            PromptUiResolution::FormResubmissionPrompt {
                reason: prompt_ui_form_resubmission_reason_from_ffi(repost_reason),
                target_url: optional_string_from_ffi(repost_target_url),
                result: prompt_ui_resolution_result_from_ffi(permission_result),
            }
        }
        _ => PromptUiResolution::Unknown,
    }
}

#[allow(clippy::too_many_arguments)]
fn download_prompt_result_from_ffi(value: u8) -> ChromeDownloadPromptResult {
    match u32::from(value) {
        CbfDownloadPromptResult_kCbfDownloadPromptResultAllowed => {
            ChromeDownloadPromptResult::Allowed
        }
        CbfDownloadPromptResult_kCbfDownloadPromptResultDenied => {
            ChromeDownloadPromptResult::Denied
        }
        CbfDownloadPromptResult_kCbfDownloadPromptResultAborted => {
            ChromeDownloadPromptResult::Aborted
        }
        _ => ChromeDownloadPromptResult::Aborted,
    }
}

fn download_prompt_reason_from_ffi(value: u8) -> ChromeDownloadPromptReason {
    match u32::from(value) {
        CbfDownloadPromptReason_kCbfDownloadPromptReasonNone => ChromeDownloadPromptReason::None,
        CbfDownloadPromptReason_kCbfDownloadPromptReasonUnexpected => {
            ChromeDownloadPromptReason::Unexpected
        }
        CbfDownloadPromptReason_kCbfDownloadPromptReasonSaveAs => {
            ChromeDownloadPromptReason::SaveAs
        }
        CbfDownloadPromptReason_kCbfDownloadPromptReasonPreference => {
            ChromeDownloadPromptReason::Preference
        }
        CbfDownloadPromptReason_kCbfDownloadPromptReasonNameTooLong => {
            ChromeDownloadPromptReason::NameTooLong
        }
        CbfDownloadPromptReason_kCbfDownloadPromptReasonTargetConflict => {
            ChromeDownloadPromptReason::TargetConflict
        }
        CbfDownloadPromptReason_kCbfDownloadPromptReasonTargetPathNotWriteable => {
            ChromeDownloadPromptReason::TargetPathNotWriteable
        }
        CbfDownloadPromptReason_kCbfDownloadPromptReasonTargetNoSpace => {
            ChromeDownloadPromptReason::TargetNoSpace
        }
        CbfDownloadPromptReason_kCbfDownloadPromptReasonDlpBlocked => {
            ChromeDownloadPromptReason::DlpBlocked
        }
        _ => ChromeDownloadPromptReason::Unknown,
    }
}

fn download_state_from_ffi(value: u8) -> ChromeDownloadState {
    match u32::from(value) {
        CbfDownloadState_kCbfDownloadStateInProgress => ChromeDownloadState::InProgress,
        CbfDownloadState_kCbfDownloadStatePaused => ChromeDownloadState::Paused,
        CbfDownloadState_kCbfDownloadStateCompleted => ChromeDownloadState::Completed,
        CbfDownloadState_kCbfDownloadStateCancelled => ChromeDownloadState::Cancelled,
        CbfDownloadState_kCbfDownloadStateInterrupted => ChromeDownloadState::Interrupted,
        _ => ChromeDownloadState::Unknown,
    }
}

fn download_outcome_from_ffi(value: u8) -> ChromeDownloadOutcome {
    match u32::from(value) {
        CbfDownloadOutcome_kCbfDownloadOutcomeSucceeded => ChromeDownloadOutcome::Succeeded,
        CbfDownloadOutcome_kCbfDownloadOutcomeCancelled => ChromeDownloadOutcome::Cancelled,
        CbfDownloadOutcome_kCbfDownloadOutcomeInterrupted => ChromeDownloadOutcome::Interrupted,
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
    match u32::from(value) {
        CbfPromptUiPermissionType_kCbfPromptUiPermissionTypeGeolocation => {
            PromptUiPermissionType::Geolocation
        }
        CbfPromptUiPermissionType_kCbfPromptUiPermissionTypeNotifications => {
            PromptUiPermissionType::Notifications
        }
        CbfPromptUiPermissionType_kCbfPromptUiPermissionTypeAudioCapture => {
            PromptUiPermissionType::AudioCapture
        }
        CbfPromptUiPermissionType_kCbfPromptUiPermissionTypeVideoCapture => {
            PromptUiPermissionType::VideoCapture
        }
        _ => PromptUiPermissionType::Unknown,
    }
}

fn prompt_ui_form_resubmission_reason_from_ffi(value: u8) -> PromptUiFormResubmissionReason {
    match u32::from(value) {
        CbfFormResubmissionReason_kCbfFormResubmissionReasonReload => {
            PromptUiFormResubmissionReason::Reload
        }
        CbfFormResubmissionReason_kCbfFormResubmissionReasonBackForward => {
            PromptUiFormResubmissionReason::BackForward
        }
        CbfFormResubmissionReason_kCbfFormResubmissionReasonOther => {
            PromptUiFormResubmissionReason::Other
        }
        _ => PromptUiFormResubmissionReason::Unknown,
    }
}

fn prompt_ui_resolution_result_from_ffi(value: u8) -> PromptUiResolutionResult {
    match u32::from(value) {
        CbfPromptUiResolutionResult_kCbfPromptUiResolutionResultAllowed => {
            PromptUiResolutionResult::Allowed
        }
        CbfPromptUiResolutionResult_kCbfPromptUiResolutionResultDenied => {
            PromptUiResolutionResult::Denied
        }
        CbfPromptUiResolutionResult_kCbfPromptUiResolutionResultAborted => {
            PromptUiResolutionResult::Aborted
        }
        _ => PromptUiResolutionResult::Unknown,
    }
}

fn prompt_ui_close_reason_from_ffi(value: u8) -> PromptUiCloseReason {
    match u32::from(value) {
        CbfPromptUiCloseReason_kCbfPromptUiCloseReasonUserCanceled => {
            PromptUiCloseReason::UserCanceled
        }
        CbfPromptUiCloseReason_kCbfPromptUiCloseReasonHostForced => PromptUiCloseReason::HostForced,
        CbfPromptUiCloseReason_kCbfPromptUiCloseReasonSystemDismissed => {
            PromptUiCloseReason::SystemDismissed
        }
        _ => PromptUiCloseReason::Unknown,
    }
}

fn cursor_icon_from_ffi(value: u8) -> CursorIcon {
    match u32::from(value) {
        CbfCursorType_kCbfCursorCrosshair => CursorIcon::Crosshair,
        CbfCursorType_kCbfCursorPointer => CursorIcon::Pointer,
        CbfCursorType_kCbfCursorMove => CursorIcon::Move,
        CbfCursorType_kCbfCursorText => CursorIcon::Text,
        CbfCursorType_kCbfCursorWait => CursorIcon::Wait,
        CbfCursorType_kCbfCursorHelp => CursorIcon::Help,
        CbfCursorType_kCbfCursorProgress => CursorIcon::Progress,
        CbfCursorType_kCbfCursorNotAllowed => CursorIcon::NotAllowed,
        CbfCursorType_kCbfCursorContextMenu => CursorIcon::ContextMenu,
        CbfCursorType_kCbfCursorCell => CursorIcon::Cell,
        CbfCursorType_kCbfCursorVerticalText => CursorIcon::VerticalText,
        CbfCursorType_kCbfCursorAlias => CursorIcon::Alias,
        CbfCursorType_kCbfCursorCopy => CursorIcon::Copy,
        CbfCursorType_kCbfCursorNoDrop => CursorIcon::NoDrop,
        CbfCursorType_kCbfCursorGrab => CursorIcon::Grab,
        CbfCursorType_kCbfCursorGrabbing => CursorIcon::Grabbing,
        CbfCursorType_kCbfCursorAllScroll => CursorIcon::AllScroll,
        CbfCursorType_kCbfCursorZoomIn => CursorIcon::ZoomIn,
        CbfCursorType_kCbfCursorZoomOut => CursorIcon::ZoomOut,
        CbfCursorType_kCbfCursorEResize => CursorIcon::EResize,
        CbfCursorType_kCbfCursorNResize => CursorIcon::NResize,
        CbfCursorType_kCbfCursorNeResize => CursorIcon::NeResize,
        CbfCursorType_kCbfCursorNwResize => CursorIcon::NwResize,
        CbfCursorType_kCbfCursorSResize => CursorIcon::SResize,
        CbfCursorType_kCbfCursorSeResize => CursorIcon::SeResize,
        CbfCursorType_kCbfCursorSwResize => CursorIcon::SwResize,
        CbfCursorType_kCbfCursorWResize => CursorIcon::WResize,
        CbfCursorType_kCbfCursorEwResize => CursorIcon::EwResize,
        CbfCursorType_kCbfCursorNsResize => CursorIcon::NsResize,
        CbfCursorType_kCbfCursorNeswResize => CursorIcon::NeswResize,
        CbfCursorType_kCbfCursorNwseResize => CursorIcon::NwseResize,
        CbfCursorType_kCbfCursorColResize => CursorIcon::ColResize,
        CbfCursorType_kCbfCursorRowResize => CursorIcon::RowResize,
        _ => CursorIcon::Default,
    }
}

fn parse_surface_handle(handle: CbfSurfaceHandle) -> Result<SurfaceHandle, BridgeError> {
    match u32::from(handle.kind) {
        CbfSurfaceHandleKind_kSurfaceHandleMacCaContextId => {
            Ok(SurfaceHandle::MacCaContextId(handle.ca_context_id))
        }
        CbfSurfaceHandleKind_kSurfaceHandleWindowsHwnd => {
            unimplemented!("Windows HWND surface handle parsing not implemented yet")
        }
        _ => Err(BridgeError::InvalidEvent),
    }
}

pub(super) fn key_event_type_to_ffi(value: ChromeKeyEventType) -> u8 {
    (match value {
        ChromeKeyEventType::RawKeyDown => CbfKeyEventType_kCbfKeyEventRawKeyDown,
        ChromeKeyEventType::KeyDown => CbfKeyEventType_kCbfKeyEventKeyDown,
        ChromeKeyEventType::KeyUp => CbfKeyEventType_kCbfKeyEventKeyUp,
        ChromeKeyEventType::Char => CbfKeyEventType_kCbfKeyEventChar,
    }) as u8
}

pub(super) fn mouse_event_type_to_ffi(value: ChromeMouseEventType) -> u8 {
    (match value {
        ChromeMouseEventType::Down => CbfMouseEventType_kCbfMouseEventDown,
        ChromeMouseEventType::Up => CbfMouseEventType_kCbfMouseEventUp,
        ChromeMouseEventType::Move => CbfMouseEventType_kCbfMouseEventMove,
        ChromeMouseEventType::Enter => CbfMouseEventType_kCbfMouseEventEnter,
        ChromeMouseEventType::Leave => CbfMouseEventType_kCbfMouseEventLeave,
    }) as u8
}

fn mouse_event_type_from_ffi(value: u8) -> ChromeMouseEventType {
    match u32::from(value) {
        CbfMouseEventType_kCbfMouseEventDown => ChromeMouseEventType::Down,
        CbfMouseEventType_kCbfMouseEventUp => ChromeMouseEventType::Up,
        CbfMouseEventType_kCbfMouseEventMove => ChromeMouseEventType::Move,
        CbfMouseEventType_kCbfMouseEventEnter => ChromeMouseEventType::Enter,
        CbfMouseEventType_kCbfMouseEventLeave => ChromeMouseEventType::Leave,
        _ => ChromeMouseEventType::Move,
    }
}

pub(super) fn mouse_button_to_ffi(value: ChromeMouseButton) -> u8 {
    (match value {
        ChromeMouseButton::None => CbfMouseButton_kCbfMouseButtonNone,
        ChromeMouseButton::Left => CbfMouseButton_kCbfMouseButtonLeft,
        ChromeMouseButton::Middle => CbfMouseButton_kCbfMouseButtonMiddle,
        ChromeMouseButton::Right => CbfMouseButton_kCbfMouseButtonRight,
        ChromeMouseButton::Back => CbfMouseButton_kCbfMouseButtonBack,
        ChromeMouseButton::Forward => CbfMouseButton_kCbfMouseButtonForward,
    }) as u8
}

fn mouse_button_from_ffi(value: u8) -> ChromeMouseButton {
    match u32::from(value) {
        CbfMouseButton_kCbfMouseButtonLeft => ChromeMouseButton::Left,
        CbfMouseButton_kCbfMouseButtonMiddle => ChromeMouseButton::Middle,
        CbfMouseButton_kCbfMouseButtonRight => ChromeMouseButton::Right,
        CbfMouseButton_kCbfMouseButtonBack => ChromeMouseButton::Back,
        CbfMouseButton_kCbfMouseButtonForward => ChromeMouseButton::Forward,
        _ => ChromeMouseButton::None,
    }
}

pub(super) fn pointer_type_to_ffi(value: ChromePointerType) -> u8 {
    (match value {
        ChromePointerType::Unknown => CbfPointerType_kCbfPointerTypeUnknown,
        ChromePointerType::Mouse => CbfPointerType_kCbfPointerTypeMouse,
        ChromePointerType::Pen => CbfPointerType_kCbfPointerTypePen,
        ChromePointerType::Touch => CbfPointerType_kCbfPointerTypeTouch,
        ChromePointerType::Eraser => CbfPointerType_kCbfPointerTypeEraser,
    }) as u8
}

fn pointer_type_from_ffi(value: u8) -> ChromePointerType {
    match u32::from(value) {
        CbfPointerType_kCbfPointerTypeMouse => ChromePointerType::Mouse,
        CbfPointerType_kCbfPointerTypePen => ChromePointerType::Pen,
        CbfPointerType_kCbfPointerTypeTouch => ChromePointerType::Touch,
        CbfPointerType_kCbfPointerTypeEraser => ChromePointerType::Eraser,
        _ => ChromePointerType::Unknown,
    }
}

pub(super) fn scroll_granularity_to_ffi(value: ChromeScrollGranularity) -> u8 {
    (match value {
        ChromeScrollGranularity::PrecisePixel => CbfScrollGranularity_kCbfScrollByPrecisePixel,
        ChromeScrollGranularity::Pixel => CbfScrollGranularity_kCbfScrollByPixel,
        ChromeScrollGranularity::Line => CbfScrollGranularity_kCbfScrollByLine,
        ChromeScrollGranularity::Page => CbfScrollGranularity_kCbfScrollByPage,
        ChromeScrollGranularity::Document => CbfScrollGranularity_kCbfScrollByDocument,
    }) as u8
}

fn scroll_granularity_from_ffi(value: u8) -> ChromeScrollGranularity {
    match u32::from(value) {
        CbfScrollGranularity_kCbfScrollByPrecisePixel => ChromeScrollGranularity::PrecisePixel,
        CbfScrollGranularity_kCbfScrollByPixel => ChromeScrollGranularity::Pixel,
        CbfScrollGranularity_kCbfScrollByLine => ChromeScrollGranularity::Line,
        CbfScrollGranularity_kCbfScrollByPage => ChromeScrollGranularity::Page,
        CbfScrollGranularity_kCbfScrollByDocument => ChromeScrollGranularity::Document,
        _ => ChromeScrollGranularity::Pixel,
    }
}

fn ime_text_span_type_to_ffi(value: ChromeImeTextSpanType) -> u8 {
    (match value {
        ChromeImeTextSpanType::Composition => CbfImeTextSpanType_kCbfImeTextSpanTypeComposition,
        ChromeImeTextSpanType::Suggestion => CbfImeTextSpanType_kCbfImeTextSpanTypeSuggestion,
        ChromeImeTextSpanType::MisspellingSuggestion => {
            CbfImeTextSpanType_kCbfImeTextSpanTypeMisspellingSuggestion
        }
        ChromeImeTextSpanType::Autocorrect => CbfImeTextSpanType_kCbfImeTextSpanTypeAutocorrect,
        ChromeImeTextSpanType::GrammarSuggestion => {
            CbfImeTextSpanType_kCbfImeTextSpanTypeGrammarSuggestion
        }
    }) as u8
}

fn ime_text_span_thickness_to_ffi(value: ChromeImeTextSpanThickness) -> u8 {
    (match value {
        ChromeImeTextSpanThickness::None => CbfImeTextSpanThickness_kCbfImeTextSpanThicknessNone,
        ChromeImeTextSpanThickness::Thin => CbfImeTextSpanThickness_kCbfImeTextSpanThicknessThin,
        ChromeImeTextSpanThickness::Thick => CbfImeTextSpanThickness_kCbfImeTextSpanThicknessThick,
    }) as u8
}

fn ime_text_span_underline_style_to_ffi(value: ChromeImeTextSpanUnderlineStyle) -> u8 {
    (match value {
        ChromeImeTextSpanUnderlineStyle::None => {
            CbfImeTextSpanUnderlineStyle_kCbfImeTextSpanUnderlineStyleNone
        }
        ChromeImeTextSpanUnderlineStyle::Solid => {
            CbfImeTextSpanUnderlineStyle_kCbfImeTextSpanUnderlineStyleSolid
        }
        ChromeImeTextSpanUnderlineStyle::Dot => {
            CbfImeTextSpanUnderlineStyle_kCbfImeTextSpanUnderlineStyleDot
        }
        ChromeImeTextSpanUnderlineStyle::Dash => {
            CbfImeTextSpanUnderlineStyle_kCbfImeTextSpanUnderlineStyleDash
        }
        ChromeImeTextSpanUnderlineStyle::Squiggle => {
            CbfImeTextSpanUnderlineStyle_kCbfImeTextSpanUnderlineStyleSquiggle
        }
    }) as u8
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
                type_: ime_text_span_type_to_ffi(span.r#type),
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
    if let Err(error) = bridge().map(|bridge| unsafe {
        bridge.cbf_bridge_convert_nsevent(nsevent.as_ptr(), browsing_context_id, &mut ffi_event)
    }) {
        warn!(error = ?error, "failed to convert NSEvent to key event");
    }

    let event = ChromeKeyEvent {
        type_: match u32::from(ffi_event.type_) {
            CbfKeyEventType_kCbfKeyEventRawKeyDown => ChromeKeyEventType::RawKeyDown,
            CbfKeyEventType_kCbfKeyEventKeyDown => ChromeKeyEventType::KeyDown,
            CbfKeyEventType_kCbfKeyEventKeyUp => ChromeKeyEventType::KeyUp,
            CbfKeyEventType_kCbfKeyEventChar => ChromeKeyEventType::Char,
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

    if let Err(error) =
        bridge().map(|bridge| unsafe { bridge.cbf_bridge_free_converted_key_event(&mut ffi_event) })
    {
        warn!(error = ?error, "failed to free converted key event");
    }

    event
}

#[cfg(target_os = "macos")]
pub fn convert_nspasteboard_to_drag_data(nspasteboard: NonNull<c_void>) -> DragData {
    let mut ffi_data = CbfDragData::default();
    if let Err(error) = bridge().map(|bridge| unsafe {
        bridge.cbf_bridge_convert_nspasteboard_to_drag_data(nspasteboard.as_ptr(), &mut ffi_data)
    }) {
        warn!(error = ?error, "failed to convert NSPasteboard to drag data");
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

    if let Err(error) =
        bridge().map(|bridge| unsafe { bridge.cbf_bridge_free_converted_drag_data(&mut ffi_data) })
    {
        warn!(error = ?error, "failed to free converted drag data");
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
    if let Err(error) = bridge().map(|bridge| unsafe {
        bridge.cbf_bridge_convert_nsevent_to_mouse_event(
            nsevent.as_ptr(),
            nsview.as_ptr(),
            browsing_context_id,
            pointer_type_to_ffi(pointer_type),
            unaccelerated_movement,
            &mut ffi_event,
        )
    }) {
        warn!(error = ?error, "failed to convert NSEvent to mouse event");
    }

    ChromeMouseEvent {
        type_: mouse_event_type_from_ffi(ffi_event.type_),
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
    if let Err(error) = bridge().map(|bridge| unsafe {
        bridge.cbf_bridge_convert_nsevent_to_mouse_wheel_event(
            nsevent.as_ptr(),
            nsview.as_ptr(),
            browsing_context_id,
            &mut ffi_event,
        )
    }) {
        warn!(error = ?error, "failed to convert NSEvent to mouse wheel event");
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

    fn make_event(kind: u32) -> CbfBridgeEvent {
        CbfBridgeEvent {
            kind: kind as u8,
            ..Default::default()
        }
    }

    fn leaked_c_string(value: &str) -> *mut i8 {
        CString::new(value).unwrap().into_raw()
    }

    #[test]
    fn parse_event_tab_created_maps_tab_id() {
        let mut event = make_event(CbfEventKind_kEventTabCreated);
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
    fn parse_event_shutdown_blocked_maps_dirty_tab_id() {
        let mut event = make_event(CbfEventKind_kEventShutdownBlocked);
        event.request_id = 9;
        event.tab_id = 2;

        let parsed = parse_event(event).expect("shutdown blocked should parse");
        assert!(matches!(
            parsed,
            IpcEvent::ShutdownBlocked {
                request_id,
                dirty_browsing_context_id
            } if request_id == 9 && dirty_browsing_context_id == TabId::new(2)
        ));
    }

    #[test]
    fn parse_event_extension_popup_ime_bounds_maps_popup_id() {
        let mut event = make_event(CbfEventKind_kEventExtensionPopupImeBoundsUpdated);
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
        let mut event = make_event(CbfEventKind_kEventFindReply);
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
        let mut event = make_event(CbfEventKind_kEventTabOpenResolved);
        event.request_id = 55;
        event.tab_open_result_kind = CbfTabOpenResult_kCbfTabOpenResultOpenedNewContext as u8;
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
        let mut event = make_event(CbfEventKind_kEventPromptUiRequested);
        event.prompt_ui_has_source_tab_id = true;
        event.prompt_ui_source_tab_id = 21;
        event.request_id = 99;
        event.prompt_ui_kind = CbfPromptUiKind_kCbfPromptUiKindPermissionPrompt as u8;
        event.prompt_ui_permission =
            CbfPromptUiPermissionType_kCbfPromptUiPermissionTypeGeolocation as u8;
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
        let mut event = make_event(CbfEventKind_kEventPromptUiRequested);
        event.prompt_ui_has_source_tab_id = true;
        event.prompt_ui_source_tab_id = 31;
        event.request_id = 109;
        event.prompt_ui_kind = CbfPromptUiKind_kCbfPromptUiKindDownloadPrompt as u8;
        event.download_reason = CbfDownloadPromptReason_kCbfDownloadPromptReasonSaveAs as u8;
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
        let mut event = make_event(CbfEventKind_kEventPromptUiRequested);
        event.tab_id = 31;
        event.request_id = 110;
        event.prompt_ui_kind = CbfPromptUiKind_kCbfPromptUiKindDownloadPrompt as u8;
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
        let mut event = make_event(CbfEventKind_kEventPromptUiResolved);
        event.prompt_ui_has_source_tab_id = true;
        event.prompt_ui_source_tab_id = 18;
        event.request_id = 77;
        event.prompt_ui_kind = CbfPromptUiKind_kCbfPromptUiKindPermissionPrompt as u8;
        event.prompt_ui_permission =
            CbfPromptUiPermissionType_kCbfPromptUiPermissionTypeNotifications as u8;
        event.prompt_ui_result =
            CbfPromptUiResolutionResult_kCbfPromptUiResolutionResultDenied as u8;
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
        let mut event = make_event(CbfEventKind_kEventPromptUiOpened);
        event.prompt_ui_has_source_tab_id = true;
        event.prompt_ui_source_tab_id = 12;
        event.prompt_ui_id = 44;
        event.prompt_ui_kind = CbfPromptUiKind_kCbfPromptUiKindExtensionInstallPrompt as u8;
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
                kind: CbfIconDataKind_kCbfIconDataKindPng as u8,
                bytes: bytes.as_ptr(),
                len: bytes.len() as u32,
                ..Default::default()
            },
        }];
        let mut event = make_event(CbfEventKind_kEventExtensionsListed);
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
                kind: CbfIconDataKind_kCbfIconDataKindPng as u8,
                len: 0,
                bytes: std::ptr::null(),
                ..Default::default()
            },
        }];
        let mut event = make_event(CbfEventKind_kEventExtensionsListed);
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
                kind: CbfIconDataKind_kCbfIconDataKindBinary as u8,
                bytes: bytes.as_ptr(),
                len: bytes.len() as u32,
                media_type: leaked_c_string("image/webp"),
                ..Default::default()
            },
        }];
        let mut event = make_event(CbfEventKind_kEventExtensionsListed);
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
        let mut event = make_event(CbfEventKind_kEventPromptUiClosed);
        event.prompt_ui_has_source_tab_id = true;
        event.prompt_ui_source_tab_id = 8;
        event.prompt_ui_id = 19;
        event.prompt_ui_kind = CbfPromptUiKind_kCbfPromptUiKindPrintPreviewDialog as u8;
        event.prompt_ui_close_reason =
            CbfPromptUiCloseReason_kCbfPromptUiCloseReasonHostForced as u8;

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
        let mut event = make_event(CbfEventKind_kEventPromptUiResolved);
        event.prompt_ui_has_source_tab_id = true;
        event.prompt_ui_source_tab_id = 99;
        event.request_id = 101;
        event.prompt_ui_kind = CbfPromptUiKind_kCbfPromptUiKindExtensionInstallPrompt as u8;
        event.extension_id = CString::new("abc").unwrap().into_raw();
        event.extension_install_prompt_result =
            CbfExtensionInstallPromptResult_kCbfExtensionInstallPromptResultAcceptedWithWithheldPermissions as u8;
        event.extension_install_prompt_detail = CString::new("withheld").unwrap().into_raw();

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
        let mut event = make_event(CbfEventKind_kEventPromptUiResolved);
        event.prompt_ui_has_source_tab_id = true;
        event.prompt_ui_source_tab_id = 41;
        event.request_id = 51;
        event.prompt_ui_kind = CbfPromptUiKind_kCbfPromptUiKindPrintPreviewDialog as u8;
        event.prompt_ui_result = CbfPromptUiDialogResult_kCbfPromptUiDialogResultCanceled as u8;

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
        let mut event = make_event(CbfEventKind_kEventPromptUiRequested);
        event.prompt_ui_has_source_tab_id = true;
        event.prompt_ui_source_tab_id = 64;
        event.request_id = 808;
        event.prompt_ui_kind = CbfPromptUiKind_kCbfPromptUiKindExtensionInstallPrompt as u8;
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
