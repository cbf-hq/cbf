mod bindings;
mod browser_view;

use crate::data::{
    key::KeyEvent,
    mouse::{MouseEvent, MouseWheelEvent, PointerType},
};
use std::ptr::NonNull;

pub use self::{
    bindings::{CALayerHost, ContextId},
    browser_view::{
        BrowserViewMac, BrowserViewMacConfig, BrowserViewMacDelegate, BrowserViewMacImeEvent,
        BrowserViewMacNativeDragDrop, BrowserViewMacNativeDragUpdate,
    },
};

/// Convert an NSEvent into a CBF key event.
pub fn convert_nsevent_to_key_event(
    web_page_id: u64,
    nsevent_ptr: NonNull<std::ffi::c_void>,
) -> KeyEvent {
    crate::ffi::convert_nsevent_to_key_event(web_page_id, nsevent_ptr)
}

/// Convert an NSEvent into a CBF mouse event.
pub fn convert_nsevent_to_mouse_event(
    web_page_id: u64,
    nsevent_ptr: NonNull<std::ffi::c_void>,
    nsview_ptr: NonNull<std::ffi::c_void>,
    pointer_type: PointerType,
    unaccelerated_movement: bool,
) -> MouseEvent {
    crate::ffi::convert_nsevent_to_mouse_event(
        web_page_id,
        nsevent_ptr,
        nsview_ptr,
        pointer_type,
        unaccelerated_movement,
    )
}

/// Convert an NSEvent into a CBF mouse wheel event.
pub fn convert_nsevent_to_mouse_wheel_event(
    web_page_id: u64,
    nsevent_ptr: NonNull<std::ffi::c_void>,
    nsview_ptr: NonNull<std::ffi::c_void>,
) -> MouseWheelEvent {
    crate::ffi::convert_nsevent_to_mouse_wheel_event(web_page_id, nsevent_ptr, nsview_ptr)
}
