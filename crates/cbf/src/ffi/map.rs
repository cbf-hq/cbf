#[cfg(target_os = "macos")]
use std::{ffi::c_void, ptr::NonNull};

use cbf_sys::ffi::*;
use cursor_icon::CursorIcon;
use tracing::debug;

use crate::{
    data::{
        context_menu::{
            self, ContextMenu, ContextMenuAccelerator, ContextMenuIcon, ContextMenuItem,
            ContextMenuItemType,
        },
        drag::{DragData, DragImage, DragOperations, DragStartRequest, DragUrlInfo},
        ids::BrowsingContextId,
        ime::{
            ChromeImeTextSpanStyle, ChromeImeTextSpanThickness, ChromeImeTextSpanUnderlineStyle,
            ImeBoundsUpdate, ImeCompositionBounds, ImeRect, ImeTextRange, ImeTextSpan,
            ImeTextSpanType, TextSelectionBounds,
        },
        key::{KeyEvent, KeyEventType},
        mouse::{
            MouseButton, MouseEvent, MouseEventType, MouseWheelEvent, PointerType,
            ScrollGranularity,
        },
        surface::SurfaceHandle,
    },
    event::BeforeUnloadReason,
};

use super::{utils::c_string_to_string, Error, IpcEvent};

pub(super) fn parse_event(event: CbfBridgeEvent) -> Result<IpcEvent, Error> {
    match event.kind {
        CBF_EVENT_SURFACE_HANDLE_UPDATED => {
            let handle = parse_surface_handle(event.surface_handle)?;

            Ok(IpcEvent::SurfaceHandleUpdated {
                profile_id: c_string_to_string(event.profile_id),
                browsing_context_id: BrowsingContextId::new(event.web_page_id),
                handle,
            })
        }
        CBF_EVENT_WEB_PAGE_CREATED => Ok(IpcEvent::WebPageCreated {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: BrowsingContextId::new(event.web_page_id),
            request_id: event.request_id,
        }),
        CBF_EVENT_IME_BOUNDS_UPDATED => Ok(IpcEvent::ImeBoundsUpdated {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: BrowsingContextId::new(event.web_page_id),
            update: parse_ime_bounds(event.ime_bounds),
        }),
        CBF_EVENT_SHUTDOWN_BLOCKED => Ok(IpcEvent::ShutdownBlocked {
            request_id: event.request_id,
            dirty_browsing_context_ids: parse_browsing_context_ids(event.dirty_web_page_ids),
        }),
        CBF_EVENT_SHUTDOWN_PROCEEDING => Ok(IpcEvent::ShutdownProceeding {
            request_id: event.request_id,
        }),
        CBF_EVENT_SHUTDOWN_CANCELLED => Ok(IpcEvent::ShutdownCancelled {
            request_id: event.request_id,
        }),
        CBF_EVENT_CONTEXT_MENU_REQUESTED => Ok(IpcEvent::ContextMenuRequested {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: BrowsingContextId::new(event.web_page_id),
            menu: parse_context_menu(event.context_menu),
        }),
        CBF_EVENT_NEW_WEB_PAGE_REQUESTED => Ok(IpcEvent::NewBrowsingContextRequested {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: BrowsingContextId::new(event.web_page_id),
            target_url: c_string_to_string(event.target_url),
            is_popup: event.is_popup,
        }),
        CBF_EVENT_NAVIGATION_STATE_CHANGED => Ok(IpcEvent::NavigationStateChanged {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: BrowsingContextId::new(event.web_page_id),
            url: c_string_to_string(event.url),
            can_go_back: event.can_go_back,
            can_go_forward: event.can_go_forward,
            is_loading: event.is_loading,
        }),
        CBF_EVENT_CURSOR_CHANGED => Ok(IpcEvent::CursorChanged {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: BrowsingContextId::new(event.web_page_id),
            cursor_type: cursor_icon_from_ffi(event.cursor_type),
        }),
        CBF_EVENT_TITLE_UPDATED => Ok(IpcEvent::TitleUpdated {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: BrowsingContextId::new(event.web_page_id),
            title: c_string_to_string(event.title),
        }),
        CBF_EVENT_FAVICON_URL_UPDATED => Ok(IpcEvent::FaviconUrlUpdated {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: BrowsingContextId::new(event.web_page_id),
            url: c_string_to_string(event.favicon_url),
        }),
        CBF_EVENT_BEFOREUNLOAD_DIALOG_REQUESTED => {
            let profile_id = c_string_to_string(event.profile_id);
            let browsing_context_id = BrowsingContextId::new(event.web_page_id);
            let reason = beforeunload_reason_from_ffi(event.beforeunload_reason);
            debug!(
                ?profile_id,
                %browsing_context_id,
                request_id = event.request_id,
                ?reason,
                "CBF beforeunload event received"
            );
            Ok(IpcEvent::BeforeUnloadDialogRequested {
                profile_id,
                browsing_context_id,
                request_id: event.request_id,
                reason,
            })
        }
        CBF_EVENT_WEB_PAGE_CLOSED => Ok(IpcEvent::WebPageClosed {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: BrowsingContextId::new(event.web_page_id),
        }),
        CBF_EVENT_WEB_PAGE_RESIZE_ACKNOWLEDGED => Ok(IpcEvent::WebPageResizeAcknowledged {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: BrowsingContextId::new(event.web_page_id),
        }),
        CBF_EVENT_WEB_PAGE_DOM_HTML_READ => Ok(IpcEvent::WebPageDomHtmlRead {
            profile_id: c_string_to_string(event.profile_id),
            browsing_context_id: BrowsingContextId::new(event.web_page_id),
            request_id: event.request_id,
            html: c_string_to_string(event.dom_html),
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
        _ => Err(Error::InvalidEvent),
    }
}

fn parse_drag_start_request(request: CbfDragStartRequest) -> DragStartRequest {
    DragStartRequest {
        session_id: request.session_id,
        browsing_context_id: BrowsingContextId::new(request.web_page_id),
        allowed_operations: DragOperations::from_bits(request.allowed_operations),
        source_origin: c_string_to_string(request.source_origin),
        data: DragData {
            text: c_string_to_string(request.data.text),
            html: c_string_to_string(request.data.html),
            html_base_url: c_string_to_string(request.data.html_base_url),
            url_infos: parse_drag_url_infos(request.data.url_infos),
            filenames: Vec::new(),
            file_mime_types: Vec::new(),
            custom_data: Default::default(),
        },
        image: parse_drag_image(request.image),
    }
}

fn parse_drag_url_infos(list: CbfDragUrlInfoList) -> Vec<DragUrlInfo> {
    if list.len == 0 || list.items.is_null() {
        return Vec::new();
    }
    let infos = unsafe { std::slice::from_raw_parts(list.items, list.len as usize) };
    infos
        .iter()
        .map(|info| DragUrlInfo {
            url: c_string_to_string(info.url),
            title: c_string_to_string(info.title),
        })
        .collect()
}

fn parse_drag_image(image: CbfDragImage) -> Option<DragImage> {
    if image.png_bytes.is_null() || image.png_len == 0 {
        return None;
    }
    let bytes = unsafe { std::slice::from_raw_parts(image.png_bytes, image.png_len as usize) };
    Some(DragImage {
        png_bytes: bytes.to_vec(),
        pixel_width: image.pixel_width,
        pixel_height: image.pixel_height,
        scale: image.scale,
        cursor_offset_x: image.cursor_offset_x,
        cursor_offset_y: image.cursor_offset_y,
    })
}

fn parse_ime_bounds(update: CbfImeBoundsUpdate) -> ImeBoundsUpdate {
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
        Some(ImeCompositionBounds {
            range_start: update.composition.range_start,
            range_end: update.composition.range_end,
            character_bounds: rects,
        })
    } else {
        None
    };

    let selection = if update.has_selection {
        Some(TextSelectionBounds {
            range_start: update.selection.range_start,
            range_end: update.selection.range_end,
            caret_rect: rect_from_ffi(update.selection.caret_rect),
            first_selection_rect: rect_from_ffi(update.selection.first_selection_rect),
        })
    } else {
        None
    };

    ImeBoundsUpdate {
        composition,
        selection,
    }
}

fn parse_browsing_context_ids(list: CbfWebPageIdList) -> Vec<BrowsingContextId> {
    if list.len == 0 || list.items.is_null() {
        return Vec::new();
    }

    let ids = unsafe { std::slice::from_raw_parts(list.items, list.len as usize) };
    ids.iter().copied().map(BrowsingContextId::new).collect()
}

fn parse_context_menu(menu: CbfContextMenu) -> ContextMenu {
    let menu = ContextMenu {
        menu_id: menu.menu_id,
        x: menu.x,
        y: menu.y,
        source_type: menu.source_type,
        items: parse_context_menu_items(menu.items),
    };

    context_menu::filter_supported(menu)
}

fn parse_context_menu_items(list: CbfContextMenuItemList) -> Vec<ContextMenuItem> {
    if list.len == 0 || list.items.is_null() {
        return Vec::new();
    }

    let items = unsafe { std::slice::from_raw_parts(list.items, list.len as usize) };
    items.iter().map(parse_context_menu_item).collect()
}

fn parse_context_menu_item(item: &CbfContextMenuItem) -> ContextMenuItem {
    ContextMenuItem {
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

fn parse_context_menu_icon(icon: CbfContextMenuIcon) -> Option<ContextMenuIcon> {
    if icon.len == 0 || icon.png_bytes.is_null() {
        return None;
    }
    let bytes = unsafe { std::slice::from_raw_parts(icon.png_bytes, icon.len as usize) };
    Some(ContextMenuIcon {
        png_bytes: bytes.to_vec(),
        width: icon.width,
        height: icon.height,
    })
}

fn parse_context_menu_accelerator(item: &CbfContextMenuItem) -> Option<ContextMenuAccelerator> {
    if !item.has_accelerator {
        return None;
    }

    Some(ContextMenuAccelerator {
        key_equivalent: c_string_to_string(item.accelerator_key_equivalent),
        modifier_mask: item.accelerator_modifier_mask,
    })
}

fn context_menu_item_type_from_ffi(value: u8) -> ContextMenuItemType {
    match value {
        CBF_MENU_ITEM_COMMAND => ContextMenuItemType::Command,
        CBF_MENU_ITEM_CHECK => ContextMenuItemType::Check,
        CBF_MENU_ITEM_RADIO => ContextMenuItemType::Radio,
        CBF_MENU_ITEM_SEPARATOR => ContextMenuItemType::Separator,
        CBF_MENU_ITEM_BUTTON_ITEM => ContextMenuItemType::ButtonItem,
        CBF_MENU_ITEM_SUBMENU => ContextMenuItemType::Submenu,
        CBF_MENU_ITEM_ACTIONABLE_SUBMENU => ContextMenuItemType::ActionableSubmenu,
        CBF_MENU_ITEM_HIGHLIGHTED => ContextMenuItemType::Highlighted,
        CBF_MENU_ITEM_TITLE => ContextMenuItemType::Title,
        _ => ContextMenuItemType::Command,
    }
}

fn rect_from_ffi(rect: CbfRect) -> ImeRect {
    ImeRect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    }
}

fn beforeunload_reason_from_ffi(value: u8) -> BeforeUnloadReason {
    match value {
        CBF_BEFOREUNLOAD_REASON_CLOSE_WEB_PAGE => BeforeUnloadReason::CloseBrowsingContext,
        CBF_BEFOREUNLOAD_REASON_NAVIGATE => BeforeUnloadReason::Navigate,
        CBF_BEFOREUNLOAD_REASON_RELOAD => BeforeUnloadReason::Reload,
        CBF_BEFOREUNLOAD_REASON_WINDOW_CLOSE => BeforeUnloadReason::WindowClose,
        _ => BeforeUnloadReason::Unknown,
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

pub(super) fn key_event_type_to_ffi(value: KeyEventType) -> u8 {
    match value {
        KeyEventType::RawKeyDown => CBF_KEY_EVENT_RAW_KEY_DOWN,
        KeyEventType::KeyDown => CBF_KEY_EVENT_KEY_DOWN,
        KeyEventType::KeyUp => CBF_KEY_EVENT_KEY_UP,
        KeyEventType::Char => CBF_KEY_EVENT_CHAR,
    }
}

pub(super) fn mouse_event_type_to_ffi(value: MouseEventType) -> u8 {
    match value {
        MouseEventType::Down => CBF_MOUSE_EVENT_DOWN,
        MouseEventType::Up => CBF_MOUSE_EVENT_UP,
        MouseEventType::Move => CBF_MOUSE_EVENT_MOVE,
        MouseEventType::Enter => CBF_MOUSE_EVENT_ENTER,
        MouseEventType::Leave => CBF_MOUSE_EVENT_LEAVE,
    }
}

fn mouse_event_type_from_ffi(value: u8) -> MouseEventType {
    match value {
        CBF_MOUSE_EVENT_DOWN => MouseEventType::Down,
        CBF_MOUSE_EVENT_UP => MouseEventType::Up,
        CBF_MOUSE_EVENT_MOVE => MouseEventType::Move,
        CBF_MOUSE_EVENT_ENTER => MouseEventType::Enter,
        CBF_MOUSE_EVENT_LEAVE => MouseEventType::Leave,
        _ => MouseEventType::Move,
    }
}

pub(super) fn mouse_button_to_ffi(value: MouseButton) -> u8 {
    match value {
        MouseButton::None => CBF_MOUSE_BUTTON_NONE,
        MouseButton::Left => CBF_MOUSE_BUTTON_LEFT,
        MouseButton::Middle => CBF_MOUSE_BUTTON_MIDDLE,
        MouseButton::Right => CBF_MOUSE_BUTTON_RIGHT,
        MouseButton::Back => CBF_MOUSE_BUTTON_BACK,
        MouseButton::Forward => CBF_MOUSE_BUTTON_FORWARD,
    }
}

fn mouse_button_from_ffi(value: u8) -> MouseButton {
    match value {
        CBF_MOUSE_BUTTON_LEFT => MouseButton::Left,
        CBF_MOUSE_BUTTON_MIDDLE => MouseButton::Middle,
        CBF_MOUSE_BUTTON_RIGHT => MouseButton::Right,
        CBF_MOUSE_BUTTON_BACK => MouseButton::Back,
        CBF_MOUSE_BUTTON_FORWARD => MouseButton::Forward,
        _ => MouseButton::None,
    }
}

pub(super) fn pointer_type_to_ffi(value: PointerType) -> u8 {
    match value {
        PointerType::Unknown => CBF_POINTER_TYPE_UNKNOWN,
        PointerType::Mouse => CBF_POINTER_TYPE_MOUSE,
        PointerType::Pen => CBF_POINTER_TYPE_PEN,
        PointerType::Touch => CBF_POINTER_TYPE_TOUCH,
        PointerType::Eraser => CBF_POINTER_TYPE_ERASER,
    }
}

fn pointer_type_from_ffi(value: u8) -> PointerType {
    match value {
        CBF_POINTER_TYPE_MOUSE => PointerType::Mouse,
        CBF_POINTER_TYPE_PEN => PointerType::Pen,
        CBF_POINTER_TYPE_TOUCH => PointerType::Touch,
        CBF_POINTER_TYPE_ERASER => PointerType::Eraser,
        _ => PointerType::Unknown,
    }
}

pub(super) fn scroll_granularity_to_ffi(value: ScrollGranularity) -> u8 {
    match value {
        ScrollGranularity::PrecisePixel => CBF_SCROLL_BY_PRECISE_PIXEL,
        ScrollGranularity::Pixel => CBF_SCROLL_BY_PIXEL,
        ScrollGranularity::Line => CBF_SCROLL_BY_LINE,
        ScrollGranularity::Page => CBF_SCROLL_BY_PAGE,
        ScrollGranularity::Document => CBF_SCROLL_BY_DOCUMENT,
    }
}

fn scroll_granularity_from_ffi(value: u8) -> ScrollGranularity {
    match value {
        CBF_SCROLL_BY_PRECISE_PIXEL => ScrollGranularity::PrecisePixel,
        CBF_SCROLL_BY_PIXEL => ScrollGranularity::Pixel,
        CBF_SCROLL_BY_LINE => ScrollGranularity::Line,
        CBF_SCROLL_BY_PAGE => ScrollGranularity::Page,
        CBF_SCROLL_BY_DOCUMENT => ScrollGranularity::Document,
        _ => ScrollGranularity::Pixel,
    }
}

fn ime_text_span_type_to_ffi(value: ImeTextSpanType) -> u8 {
    match value {
        ImeTextSpanType::Composition => CBF_IME_TEXT_SPAN_TYPE_COMPOSITION,
        ImeTextSpanType::Suggestion => CBF_IME_TEXT_SPAN_TYPE_SUGGESTION,
        ImeTextSpanType::MisspellingSuggestion => CBF_IME_TEXT_SPAN_TYPE_MISSPELLING_SUGGESTION,
        ImeTextSpanType::Autocorrect => CBF_IME_TEXT_SPAN_TYPE_AUTOCORRECT,
        ImeTextSpanType::GrammarSuggestion => CBF_IME_TEXT_SPAN_TYPE_GRAMMAR_SUGGESTION,
    }
}

fn chrome_ime_text_span_thickness_to_ffi(value: ChromeImeTextSpanThickness) -> u8 {
    match value {
        ChromeImeTextSpanThickness::None => CBF_IME_TEXT_SPAN_THICKNESS_NONE,
        ChromeImeTextSpanThickness::Thin => CBF_IME_TEXT_SPAN_THICKNESS_THIN,
        ChromeImeTextSpanThickness::Thick => CBF_IME_TEXT_SPAN_THICKNESS_THICK,
    }
}

fn chrome_ime_text_span_underline_style_to_ffi(value: ChromeImeTextSpanUnderlineStyle) -> u8 {
    match value {
        ChromeImeTextSpanUnderlineStyle::None => CBF_IME_TEXT_SPAN_UNDERLINE_STYLE_NONE,
        ChromeImeTextSpanUnderlineStyle::Solid => CBF_IME_TEXT_SPAN_UNDERLINE_STYLE_SOLID,
        ChromeImeTextSpanUnderlineStyle::Dot => CBF_IME_TEXT_SPAN_UNDERLINE_STYLE_DOT,
        ChromeImeTextSpanUnderlineStyle::Dash => CBF_IME_TEXT_SPAN_UNDERLINE_STYLE_DASH,
        ChromeImeTextSpanUnderlineStyle::Squiggle => CBF_IME_TEXT_SPAN_UNDERLINE_STYLE_SQUIGGLE,
    }
}

fn chrome_ime_text_span_style_from_span(span: &ImeTextSpan) -> ChromeImeTextSpanStyle {
    span.chrome_style.clone().unwrap_or(ChromeImeTextSpanStyle {
        underline_color: span.underline_color,
        thickness: span.thickness,
        underline_style: span.underline_style,
        text_color: span.text_color,
        background_color: span.background_color,
        suggestion_highlight_color: span.suggestion_highlight_color,
        remove_on_finish_composing: span.remove_on_finish_composing,
        interim_char_selection: span.interim_char_selection,
        should_hide_suggestion_menu: span.should_hide_suggestion_menu,
    })
}

pub(super) fn ime_range_to_ffi(value: &Option<ImeTextRange>) -> (i32, i32) {
    match value {
        Some(range) => (range.start, range.end),
        // Sentinel for "no replacement range"; C++ side treats (-1, -1) as null.
        None => (-1, -1),
    }
}

pub(super) fn to_ffi_ime_text_spans(spans: &[ImeTextSpan]) -> Vec<CbfImeTextSpan> {
    spans
        .iter()
        .map(|span| {
            let chrome_style = chrome_ime_text_span_style_from_span(span);

            CbfImeTextSpan {
                type_: ime_text_span_type_to_ffi(span.r#type),
                start_offset: span.start_offset,
                end_offset: span.end_offset,
                underline_color: chrome_style.underline_color,
                thickness: chrome_ime_text_span_thickness_to_ffi(chrome_style.thickness),
                underline_style: chrome_ime_text_span_underline_style_to_ffi(
                    chrome_style.underline_style,
                ),
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
    let mut ffi_event = CbfKeyEvent::default();
    unsafe {
        cbf_bridge_convert_nsevent(nsevent.as_ptr(), browsing_context_id, &mut ffi_event);
    }

    let event = KeyEvent {
        type_: match ffi_event.type_ {
            CBF_KEY_EVENT_RAW_KEY_DOWN => KeyEventType::RawKeyDown,
            CBF_KEY_EVENT_KEY_DOWN => KeyEventType::KeyDown,
            CBF_KEY_EVENT_KEY_UP => KeyEventType::KeyUp,
            CBF_KEY_EVENT_CHAR => KeyEventType::Char,
            _ => KeyEventType::RawKeyDown,
        },
        modifiers: ffi_event.modifiers,
        key_code: ffi_event.windows_key_code,
        platform_key_code: ffi_event.native_key_code,
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
        url_infos: parse_drag_url_infos(ffi_data.url_infos),
        filenames: Vec::new(),
        file_mime_types: Vec::new(),
        custom_data: Default::default(),
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

    MouseEvent {
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
    let mut ffi_event = CbfMouseWheelEvent::default();
    unsafe {
        cbf_bridge_convert_nsevent_to_mouse_wheel_event(
            nsevent.as_ptr(),
            nsview.as_ptr(),
            browsing_context_id,
            &mut ffi_event,
        );
    }

    MouseWheelEvent {
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
        delta_units: scroll_granularity_from_ffi(ffi_event.delta_units),
    }
}
