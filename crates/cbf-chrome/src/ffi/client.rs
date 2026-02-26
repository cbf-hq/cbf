use std::{ffi::CString, ptr};

use cbf::data::{
    browsing_context_open::BrowsingContextOpenResponse,
    drag::{DragDrop, DragUpdate},
    extension::{AuxiliaryWindowId, AuxiliaryWindowResponse, ExtensionInfo},
    ids::BrowsingContextId,
    ime::{ConfirmCompositionBehavior, ImeCommitText, ImeComposition},
    key::KeyEvent,
    mouse::{MouseEvent, MouseWheelEvent},
    profile::ProfileInfo,
    window_open::WindowOpenResponse,
};
use cbf_chrome_sys::ffi::*;
use tracing::{debug, warn};

use super::map::{
    ime_range_to_ffi, key_event_type_to_ffi, mouse_button_to_ffi, mouse_event_type_to_ffi,
    parse_event, pointer_type_to_ffi, scroll_granularity_to_ffi, to_ffi_ime_text_spans,
};
use super::utils::{c_string_to_string, to_optional_cstring};
use super::{Error, IpcEvent};
use crate::data::input::{ChromeKeyEvent, ChromeMouseWheelEvent};

/// Client wrapper for the CBF IPC bridge.
pub struct IpcClient {
    inner: *mut CbfBridgeClientHandle,
}

impl IpcClient {
    /// Connect to the IPC channel and create a new client.
    pub fn connect(channel_name: &str) -> Result<Self, Error> {
        let channel = CString::new(channel_name).map_err(|_| Error::InvalidInput)?;

        let inner = unsafe {
            cbf_bridge_init();
            cbf_bridge_client_create()
        };

        let connected = if inner.is_null() {
            false
        } else {
            unsafe { cbf_bridge_client_connect(inner, channel.as_ptr()) }
        };

        if !connected {
            warn!(
                result = "err",
                error = "ipc_connect_failed",
                channel = %channel_name,
                "IPC client connect failed"
            );
            if !inner.is_null() {
                unsafe { cbf_bridge_client_destroy(inner) };
            }

            return Err(Error::ConnectionFailed);
        }

        debug!(channel = %channel_name, "IPC client connected");
        Ok(Self { inner })
    }

    /// Poll the next IPC event, if any, from the backend.
    pub fn poll_event(&mut self) -> Option<Result<IpcEvent, Error>> {
        if self.inner.is_null() {
            return None;
        }

        let mut event = CbfBridgeEvent::default();
        if !unsafe { cbf_bridge_client_poll_event(self.inner, &mut event) } {
            return None;
        }

        let parsed = parse_event(event);
        unsafe { cbf_bridge_event_free(&mut event) };

        if let Err(err) = &parsed {
            warn!(
                result = "err",
                error = "ipc_event_parse_failed",
                err = ?err,
                "IPC event parse failed"
            );
        }

        Some(parsed)
    }

    /// Retrieve the list of browser profiles from the backend.
    pub fn list_profiles(&mut self) -> Result<Vec<ProfileInfo>, Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let mut list = CbfProfileList::default();
        if !unsafe { cbf_bridge_client_get_profiles(self.inner, &mut list) } {
            return Err(Error::ConnectionFailed);
        }

        let profiles = if list.len == 0 || list.profiles.is_null() {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(list.profiles, list.len as usize) }
        };
        let mut result = Vec::with_capacity(profiles.len());

        for profile in profiles {
            result.push(ProfileInfo {
                profile_id: c_string_to_string(profile.profile_id),
                profile_path: c_string_to_string(profile.profile_path),
                display_name: c_string_to_string(profile.display_name),
            });
        }

        unsafe { cbf_bridge_profile_list_free(&mut list) };

        Ok(result)
    }

    /// Retrieve the list of extensions from the backend.
    pub fn list_extensions(
        &mut self,
        profile_id: &Option<String>,
    ) -> Result<Vec<ExtensionInfo>, Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let profile = to_optional_cstring(profile_id).map_err(|_| Error::InvalidInput)?;
        let profile_ptr = profile.as_ref().map_or(ptr::null(), |v| v.as_ptr());

        let mut list = CbfExtensionInfoList::default();
        if !unsafe { cbf_bridge_client_list_extensions(self.inner, profile_ptr, &mut list) } {
            return Err(Error::ConnectionFailed);
        }

        let values = if list.len == 0 || list.items.is_null() {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(list.items, list.len as usize) }
        };
        let mut result = Vec::with_capacity(values.len());
        for value in values {
            let permission_names =
                if value.permission_names.len == 0 || value.permission_names.items.is_null() {
                    Vec::new()
                } else {
                    let permission_items = unsafe {
                        std::slice::from_raw_parts(
                            value.permission_names.items,
                            value.permission_names.len as usize,
                        )
                    };
                    permission_items
                        .iter()
                        .map(|entry| c_string_to_string(*entry))
                        .collect()
                };
            result.push(ExtensionInfo {
                id: c_string_to_string(value.id),
                name: c_string_to_string(value.name),
                version: c_string_to_string(value.version),
                enabled: value.enabled,
                permission_names,
            });
        }

        unsafe { cbf_bridge_extension_list_free(&mut list) };
        Ok(result)
    }

    /// Create a web page (tab) via the IPC bridge.
    pub fn create_web_contents(
        &mut self,
        request_id: u64,
        initial_url: &str,
        profile_id: &str,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let url = CString::new(initial_url).map_err(|_| Error::InvalidInput)?;
        let profile = CString::new(profile_id).map_err(|_| Error::InvalidInput)?;

        if unsafe {
            cbf_bridge_client_create_web_page(
                self.inner,
                request_id,
                url.as_ptr(),
                profile.as_ptr(),
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Request closing the specified web page.
    pub fn request_close_web_contents(
        &mut self,
        browsing_context_id: BrowsingContextId,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe {
            cbf_bridge_client_request_close_web_page(self.inner, browsing_context_id.get())
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Update the surface size of the specified web page.
    pub fn set_web_contents_size(
        &mut self,
        browsing_context_id: BrowsingContextId,
        width: u32,
        height: u32,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe {
            cbf_bridge_client_set_web_page_size(
                self.inner,
                browsing_context_id.get(),
                width,
                height,
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Update whether the specified web page should receive text input focus.
    pub fn set_web_contents_focus(
        &mut self,
        browsing_context_id: BrowsingContextId,
        focused: bool,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe {
            cbf_bridge_client_set_web_page_focus(self.inner, browsing_context_id.get(), focused)
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Respond to a beforeunload confirmation request.
    pub fn confirm_beforeunload(
        &mut self,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        proceed: bool,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe {
            cbf_bridge_client_confirm_beforeunload(
                self.inner,
                browsing_context_id.get(),
                request_id,
                proceed,
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Navigate the page to the provided URL.
    pub fn navigate(
        &mut self,
        browsing_context_id: BrowsingContextId,
        url: &str,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let url = CString::new(url).map_err(|_| Error::InvalidInput)?;

        if unsafe {
            cbf_bridge_client_navigate(self.inner, browsing_context_id.get(), url.as_ptr())
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Navigate back in history for the page.
    pub fn go_back(&mut self, browsing_context_id: BrowsingContextId) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe { cbf_bridge_client_go_back(self.inner, browsing_context_id.get()) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Navigate forward in history for the page.
    pub fn go_forward(&mut self, browsing_context_id: BrowsingContextId) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe { cbf_bridge_client_go_forward(self.inner, browsing_context_id.get()) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Reload the page, optionally ignoring caches.
    pub fn reload(
        &mut self,
        browsing_context_id: BrowsingContextId,
        ignore_cache: bool,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe { cbf_bridge_client_reload(self.inner, browsing_context_id.get(), ignore_cache) }
        {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Open print preview for the page.
    pub fn print_preview(&mut self, browsing_context_id: BrowsingContextId) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe { cbf_bridge_client_print_preview(self.inner, browsing_context_id.get()) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Open DevTools for the specified page.
    pub fn open_dev_tools(&mut self, browsing_context_id: BrowsingContextId) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe { cbf_bridge_client_open_dev_tools(self.inner, browsing_context_id.get()) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Open DevTools and inspect the element at the given coordinates.
    pub fn inspect_element(
        &mut self,
        browsing_context_id: BrowsingContextId,
        x: i32,
        y: i32,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe { cbf_bridge_client_inspect_element(self.inner, browsing_context_id.get(), x, y) }
        {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Request the DOM HTML for the specified page.
    pub fn get_web_contents_dom_html(
        &mut self,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe {
            cbf_bridge_client_get_web_page_dom_html(
                self.inner,
                browsing_context_id.get(),
                request_id,
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Open Chromium default auxiliary window UI for pending request.
    pub fn open_default_auxiliary_window(
        &mut self,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }
        debug!(
            %browsing_context_id,
            request_id,
            "ffi open_default_auxiliary_window"
        );
        if unsafe {
            cbf_bridge_client_open_default_auxiliary_window(
                self.inner,
                browsing_context_id.get(),
                request_id,
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Respond to a pending auxiliary request.
    pub fn respond_auxiliary_window(
        &mut self,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        response: &AuxiliaryWindowResponse,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }
        let proceed = match response {
            AuxiliaryWindowResponse::ExtensionInstallPrompt { proceed } => *proceed,
            AuxiliaryWindowResponse::Unknown => false,
        };
        debug!(
            %browsing_context_id,
            request_id,
            proceed,
            ?response,
            "ffi respond_auxiliary_window"
        );
        if unsafe {
            cbf_bridge_client_respond_auxiliary_window(
                self.inner,
                browsing_context_id.get(),
                request_id,
                proceed,
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Close a backend-managed auxiliary window/dialog.
    pub fn close_auxiliary_window(
        &mut self,
        browsing_context_id: BrowsingContextId,
        window_id: AuxiliaryWindowId,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }
        debug!(
            %browsing_context_id,
            ?window_id,
            "ffi close_auxiliary_window"
        );
        if unsafe {
            cbf_bridge_client_close_auxiliary_window(
                self.inner,
                browsing_context_id.get(),
                window_id.get(),
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Respond to host-mediated browsing context open request.
    pub fn respond_browsing_context_open(
        &mut self,
        request_id: u64,
        response: &BrowsingContextOpenResponse,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }
        let (response_kind, target_web_page_id, activate) = match response {
            BrowsingContextOpenResponse::AllowNewContext { activate } => (
                CBF_BROWSING_CONTEXT_OPEN_RESPONSE_ALLOW_NEW_CONTEXT,
                0,
                *activate,
            ),
            BrowsingContextOpenResponse::AllowExistingContext {
                browsing_context_id,
                activate,
            } => (
                CBF_BROWSING_CONTEXT_OPEN_RESPONSE_ALLOW_EXISTING_CONTEXT,
                browsing_context_id.get(),
                *activate,
            ),
            BrowsingContextOpenResponse::Deny => {
                (CBF_BROWSING_CONTEXT_OPEN_RESPONSE_DENY, 0, false)
            }
        };
        debug!(
            request_id,
            response_kind,
            target_web_page_id,
            activate,
            ?response,
            "ffi respond_browsing_context_open"
        );
        if unsafe {
            cbf_bridge_client_respond_browsing_context_open(
                self.inner,
                request_id,
                response_kind,
                target_web_page_id,
                activate,
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Respond to host-mediated window open request.
    ///
    /// Current bridge path reuses browsing-context-open response semantics.
    pub fn respond_window_open(
        &mut self,
        request_id: u64,
        response: &WindowOpenResponse,
    ) -> Result<(), Error> {
        let browsing_context_response = match response {
            WindowOpenResponse::AllowExistingWindow { .. }
            | WindowOpenResponse::AllowNewWindow { .. } => {
                BrowsingContextOpenResponse::AllowNewContext { activate: true }
            }
            WindowOpenResponse::Deny => BrowsingContextOpenResponse::Deny,
        };
        self.respond_browsing_context_open(request_id, &browsing_context_response)
    }

    /// Send a keyboard event to the page.
    pub fn send_key_event(
        &mut self,
        browsing_context_id: BrowsingContextId,
        event: &KeyEvent,
        commands: &[String],
    ) -> Result<(), Error> {
        let chrome_event = ChromeKeyEvent::from(event.clone());
        self.send_key_event_raw(browsing_context_id, &chrome_event, commands)
    }

    /// Send a Chromium-shaped keyboard event to the page.
    pub fn send_key_event_raw(
        &mut self,
        browsing_context_id: BrowsingContextId,
        event: &ChromeKeyEvent,
        commands: &[String],
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let dom_code = to_optional_cstring(&event.dom_code)?;
        let dom_key = to_optional_cstring(&event.dom_key)?;
        let text = to_optional_cstring(&event.text)?;
        let unmodified_text = to_optional_cstring(&event.unmodified_text)?;

        let command_cstrings = commands
            .iter()
            .map(|command| CString::new(command.as_str()).map_err(|_| Error::InvalidInput))
            .collect::<Result<Vec<_>, _>>()?;
        let command_ptrs: Vec<*const std::os::raw::c_char> =
            command_cstrings.iter().map(|cstr| cstr.as_ptr()).collect();

        let ffi_event = CbfKeyEvent {
            web_page_id: browsing_context_id.get(),
            type_: key_event_type_to_ffi(event.type_),
            modifiers: event.modifiers,
            windows_key_code: event.windows_key_code,
            native_key_code: event.native_key_code,
            dom_code: dom_code.as_ref().map_or(ptr::null(), |v| v.as_ptr()),
            dom_key: dom_key.as_ref().map_or(ptr::null(), |v| v.as_ptr()),
            text: text.as_ref().map_or(ptr::null(), |v| v.as_ptr()),
            unmodified_text: unmodified_text.as_ref().map_or(ptr::null(), |v| v.as_ptr()),
            auto_repeat: event.auto_repeat,
            is_keypad: event.is_keypad,
            is_system_key: event.is_system_key,
            location: event.location,
        };

        let ffi_commands = CbfCommandList {
            items: if command_ptrs.is_empty() {
                ptr::null()
            } else {
                command_ptrs.as_ptr()
            },
            len: command_ptrs.len() as u32,
        };

        if unsafe { cbf_bridge_client_send_key_event(self.inner, &ffi_event, &ffi_commands) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Send a mouse event to the page.
    pub fn send_mouse_event(
        &mut self,
        browsing_context_id: BrowsingContextId,
        event: &MouseEvent,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let ffi_event = CbfMouseEvent {
            web_page_id: browsing_context_id.get(),
            type_: mouse_event_type_to_ffi(event.type_),
            modifiers: event.modifiers,
            button: mouse_button_to_ffi(event.button),
            click_count: event.click_count,
            position_in_widget_x: event.position_in_widget_x,
            position_in_widget_y: event.position_in_widget_y,
            position_in_screen_x: event.position_in_screen_x,
            position_in_screen_y: event.position_in_screen_y,
            movement_x: event.movement_x,
            movement_y: event.movement_y,
            is_raw_movement_event: event.is_raw_movement_event,
            pointer_type: pointer_type_to_ffi(event.pointer_type),
        };

        if unsafe { cbf_bridge_client_send_mouse_event(self.inner, &ffi_event) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Send a mouse wheel event to the page.
    pub fn send_mouse_wheel_event(
        &mut self,
        browsing_context_id: BrowsingContextId,
        event: &MouseWheelEvent,
    ) -> Result<(), Error> {
        let chrome_event = ChromeMouseWheelEvent::from(event.clone());
        self.send_mouse_wheel_event_raw(browsing_context_id, &chrome_event)
    }

    /// Send a Chromium-shaped mouse wheel event to the page.
    pub fn send_mouse_wheel_event_raw(
        &mut self,
        browsing_context_id: BrowsingContextId,
        event: &ChromeMouseWheelEvent,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let ffi_event = CbfMouseWheelEvent {
            web_page_id: browsing_context_id.get(),
            modifiers: event.modifiers,
            position_in_widget_x: event.position_in_widget_x,
            position_in_widget_y: event.position_in_widget_y,
            position_in_screen_x: event.position_in_screen_x,
            position_in_screen_y: event.position_in_screen_y,
            movement_x: event.movement_x,
            movement_y: event.movement_y,
            is_raw_movement_event: event.is_raw_movement_event,
            delta_x: event.delta_x,
            delta_y: event.delta_y,
            wheel_ticks_x: event.wheel_ticks_x,
            wheel_ticks_y: event.wheel_ticks_y,
            phase: event.phase,
            momentum_phase: event.momentum_phase,
            delta_units: scroll_granularity_to_ffi(event.delta_units),
        };

        if unsafe { cbf_bridge_client_send_mouse_wheel_event(self.inner, &ffi_event) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Send a drag update event for host-owned drag session.
    pub fn send_drag_update(&mut self, update: &DragUpdate) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let ffi_update = CbfDragUpdate {
            session_id: update.session_id,
            web_page_id: update.browsing_context_id.get(),
            allowed_operations: update.allowed_operations.bits(),
            modifiers: update.modifiers,
            position_in_widget_x: update.position_in_widget_x,
            position_in_widget_y: update.position_in_widget_y,
            position_in_screen_x: update.position_in_screen_x,
            position_in_screen_y: update.position_in_screen_y,
        };

        if unsafe { cbf_bridge_client_send_drag_update(self.inner, &ffi_update) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Send a drag drop event for host-owned drag session.
    pub fn send_drag_drop(&mut self, drop: &DragDrop) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let ffi_drop = CbfDragDrop {
            session_id: drop.session_id,
            web_page_id: drop.browsing_context_id.get(),
            modifiers: drop.modifiers,
            position_in_widget_x: drop.position_in_widget_x,
            position_in_widget_y: drop.position_in_widget_y,
            position_in_screen_x: drop.position_in_screen_x,
            position_in_screen_y: drop.position_in_screen_y,
        };

        if unsafe { cbf_bridge_client_send_drag_drop(self.inner, &ffi_drop) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Cancel a host-owned drag session.
    pub fn send_drag_cancel(
        &mut self,
        session_id: u64,
        browsing_context_id: BrowsingContextId,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe {
            cbf_bridge_client_send_drag_cancel(self.inner, session_id, browsing_context_id.get())
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Update the IME composition state.
    pub fn set_composition(&mut self, composition: &ImeComposition) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let text = CString::new(composition.text.as_str()).map_err(|_| Error::InvalidInput)?;
        let spans = to_ffi_ime_text_spans(&composition.spans);
        let span_list = CbfImeTextSpanList {
            items: if spans.is_empty() {
                ptr::null()
            } else {
                spans.as_ptr()
            },
            len: spans.len() as u32,
        };
        let (replacement_start, replacement_end) = ime_range_to_ffi(&composition.replacement_range);

        let ffi_composition = CbfImeComposition {
            web_page_id: composition.browsing_context_id.get(),
            text: text.as_ptr(),
            selection_start: composition.selection_start,
            selection_end: composition.selection_end,
            replacement_range_start: replacement_start,
            replacement_range_end: replacement_end,
            spans: span_list,
        };

        if unsafe { cbf_bridge_client_set_composition(self.inner, &ffi_composition) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Commit IME text input to the page.
    pub fn commit_text(&mut self, commit: &ImeCommitText) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let text = CString::new(commit.text.as_str()).map_err(|_| Error::InvalidInput)?;
        let spans = to_ffi_ime_text_spans(&commit.spans);
        let span_list = CbfImeTextSpanList {
            items: if spans.is_empty() {
                ptr::null()
            } else {
                spans.as_ptr()
            },
            len: spans.len() as u32,
        };
        let (replacement_start, replacement_end) = ime_range_to_ffi(&commit.replacement_range);

        let ffi_commit = CbfImeCommitText {
            web_page_id: commit.browsing_context_id.get(),
            text: text.as_ptr(),
            relative_caret_position: commit.relative_caret_position,
            replacement_range_start: replacement_start,
            replacement_range_end: replacement_end,
            spans: span_list,
        };

        if unsafe { cbf_bridge_client_commit_text(self.inner, &ffi_commit) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Finish composing IME text with the specified behavior.
    pub fn finish_composing_text(
        &mut self,
        browsing_context_id: BrowsingContextId,
        behavior: ConfirmCompositionBehavior,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let behavior = match behavior {
            ConfirmCompositionBehavior::DoNotKeepSelection => CBF_IME_CONFIRM_DO_NOT_KEEP_SELECTION,
            ConfirmCompositionBehavior::KeepSelection => CBF_IME_CONFIRM_KEEP_SELECTION,
        };

        if unsafe {
            cbf_bridge_client_finish_composing_text(self.inner, browsing_context_id.get(), behavior)
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Execute a context menu command for the given menu.
    pub fn execute_context_menu_command(
        &mut self,
        menu_id: u64,
        command_id: i32,
        event_flags: i32,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe {
            cbf_bridge_client_execute_context_menu_command(
                self.inner,
                menu_id,
                command_id,
                event_flags,
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Dismiss the context menu with the given id.
    pub fn dismiss_context_menu(&mut self, menu_id: u64) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe { cbf_bridge_client_dismiss_context_menu(self.inner, menu_id) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Request a graceful shutdown from the backend.
    pub fn request_shutdown(&mut self, request_id: u64) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe { cbf_bridge_client_request_shutdown(self.inner, request_id) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Respond to a shutdown confirmation request.
    pub fn confirm_shutdown(&mut self, request_id: u64, proceed: bool) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe { cbf_bridge_client_confirm_shutdown(self.inner, request_id, proceed) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Force an immediate shutdown without confirmations.
    pub fn force_shutdown(&mut self) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe { cbf_bridge_client_force_shutdown(self.inner) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Tear down the IPC client and free native resources.
    pub fn shutdown(&mut self) {
        if !self.inner.is_null() {
            unsafe { cbf_bridge_client_shutdown(self.inner) };
        }
    }
}

impl Drop for IpcClient {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { cbf_bridge_client_destroy(self.inner) };
            self.inner = ptr::null_mut();
        }
    }
}
