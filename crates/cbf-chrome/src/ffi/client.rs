use std::{ffi::CString, ptr};

use cbf::data::{
    drag::{DragDrop, DragUpdate},
    ids::BrowsingContextId,
    ime::{ConfirmCompositionBehavior, ImeCommitText, ImeComposition},
    key::KeyEvent,
    mouse::{MouseEvent, MouseWheelEvent},
    profile::ProfileInfo,
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
