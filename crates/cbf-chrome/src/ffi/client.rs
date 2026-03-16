use std::{
    ffi::{CStr, CString},
    ptr,
    time::Duration,
};

use cbf::data::{edit::EditAction, window_open::WindowOpenResponse};
use cbf_chrome_sys::ffi::*;
use tracing::warn;

use super::map::{
    ime_range_to_ffi, key_event_type_to_ffi, mouse_button_to_ffi, mouse_event_type_to_ffi,
    parse_event, parse_extension_list, pointer_type_to_ffi, scroll_granularity_to_ffi,
    to_ffi_ime_text_spans,
};
use super::utils::{c_string_to_string, to_optional_cstring};
use super::{Error, IpcEvent};
use crate::data::{
    browsing_context_open::ChromeBrowsingContextOpenResponse,
    download::ChromeDownloadId,
    drag::{ChromeDragDrop, ChromeDragUpdate},
    extension::ChromeExtensionInfo,
    ids::{PopupId, TabId},
    ime::{
        ChromeConfirmCompositionBehavior, ChromeImeCommitText, ChromeImeComposition,
        ChromeTransientImeCommitText, ChromeTransientImeComposition,
    },
    input::{ChromeKeyEvent, ChromeMouseWheelEvent},
    mouse::ChromeMouseEvent,
    profile::ChromeProfileInfo,
    prompt_ui::{PromptUiId, PromptUiResponse},
};

/// Client wrapper for the CBF IPC bridge.
pub struct IpcClient {
    inner: *mut CbfBridgeClientHandle,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct IpcEventWaitHandle {
    inner: *mut CbfBridgeClientHandle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventWaitResult {
    EventAvailable,
    TimedOut,
    Disconnected,
    Closed,
}

// SAFETY: IpcClient owns the bridge client handle and its methods
// serialize access through the Mojo thread internally. The handle
// is not shared, only moved across the process::start_chromium →
// backend thread boundary exactly once.
unsafe impl Send for IpcClient {}
// SAFETY: `cbf_bridge_client_wait_for_event` synchronizes through the bridge's
// internal event wait state. This handle is non-owning and only used while the
// owning `IpcClient` remains alive.
unsafe impl Send for IpcEventWaitHandle {}
// SAFETY: The bridge wait path is internally synchronized; this wrapper only
// exposes `wait_for_event` and does not own the underlying handle.
unsafe impl Sync for IpcEventWaitHandle {}

impl std::fmt::Debug for IpcClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IpcClient")
            .field("inner", &format!("{:p}", self.inner))
            .finish()
    }
}

impl IpcClient {
    /// Prepare the Mojo channel before spawning the Chromium process.
    ///
    /// Returns `(remote_fd, switch_arg)` where:
    /// - `remote_fd` is the file descriptor of the remote channel endpoint that
    ///   must be inherited by the child process (Unix only; -1 on other platforms).
    /// - `switch_arg` is the command-line switch that Chromium needs to recover
    ///   the endpoint (e.g. `--cbf-ipc-handle=...`).
    pub fn prepare_channel() -> Result<(i32, String), Error> {
        let mut buf = [0u8; 512];
        let fd = unsafe {
            cbf_bridge_init();
            cbf_bridge_prepare_channel(
                buf.as_mut_ptr() as *mut std::os::raw::c_char,
                buf.len() as i32,
            )
        };
        let switch_arg = CStr::from_bytes_until_nul(&buf)
            .map_err(|_| Error::ConnectionFailed)?
            .to_str()
            .map_err(|_| Error::ConnectionFailed)?
            .to_owned();
        Ok((fd, switch_arg))
    }

    /// Notify the bridge of the spawned child's PID.
    ///
    /// Must be called after spawning the Chromium process and before
    /// `connect_inherited`. On macOS this registers the Mach port with the
    /// rendezvous server; on other platforms it completes channel bookkeeping.
    pub fn pass_child_pid(pid: u32) {
        unsafe { cbf_bridge_pass_child_pid(pid as i64) }
    }

    /// Wrap a pre-created bridge client handle and complete the Mojo connection.
    ///
    /// `inner` must have been created by `cbf_bridge_client_create()` and the
    /// channel must have been prepared with `prepare_channel()` before calling
    /// this function (after the child process has been spawned).
    ///
    /// # Safety
    ///
    /// `inner` must be a valid, live `CbfBridgeClientHandle` allocated by
    /// `cbf_bridge_client_create()`. Ownership is transferred to the returned
    /// `IpcClient` on success and consumed by this function on failure.
    pub unsafe fn connect_inherited(inner: *mut CbfBridgeClientHandle) -> Result<Self, Error> {
        if inner.is_null() {
            return Err(Error::ConnectionFailed);
        }
        let connected = unsafe { cbf_bridge_client_connect_inherited(inner) };
        if !connected {
            warn!(
                result = "err",
                error = "ipc_connect_inherited_failed",
                "IPC inherited connect failed"
            );
            unsafe { cbf_bridge_client_destroy(inner) };
            return Err(Error::ConnectionFailed);
        }
        Ok(Self { inner })
    }

    /// Authenticate with the session token and set up the browser observer.
    ///
    /// Must be called once after `connect_inherited` and before any other method.
    pub fn authenticate(&self, token: &str) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }
        let token = CString::new(token).map_err(|_| Error::InvalidInput)?;
        if unsafe { cbf_bridge_client_authenticate(self.inner, token.as_ptr()) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Wait until an event is available or the bridge closes.
    pub fn wait_for_event(&self, timeout: Option<Duration>) -> Result<EventWaitResult, Error> {
        wait_for_event_inner(self.inner, timeout)
    }

    pub(crate) fn event_wait_handle(&self) -> IpcEventWaitHandle {
        IpcEventWaitHandle { inner: self.inner }
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
    pub fn list_profiles(&mut self) -> Result<Vec<ChromeProfileInfo>, Error> {
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
            result.push(ChromeProfileInfo {
                profile_id: c_string_to_string(profile.profile_id),
                profile_path: c_string_to_string(profile.profile_path),
                display_name: c_string_to_string(profile.display_name),
                is_default: profile.is_default,
            });
        }

        unsafe { cbf_bridge_profile_list_free(&mut list) };

        Ok(result)
    }

    /// Retrieve the list of extensions from the backend.
    pub fn list_extensions(&mut self, profile_id: &str) -> Result<Vec<ChromeExtensionInfo>, Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let profile = CString::new(profile_id).map_err(|_| Error::InvalidInput)?;

        let mut list = CbfExtensionInfoList::default();
        if !unsafe { cbf_bridge_client_list_extensions(self.inner, profile.as_ptr(), &mut list) } {
            return Err(Error::ConnectionFailed);
        }

        let result = parse_extension_list(list);

        unsafe { cbf_bridge_extension_list_free(&mut list) };
        Ok(result)
    }

    pub fn activate_extension_action(
        &mut self,
        browsing_context_id: TabId,
        extension_id: &str,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let extension_id = CString::new(extension_id).map_err(|_| Error::InvalidInput)?;
        if unsafe {
            cbf_bridge_client_activate_extension_action(
                self.inner,
                browsing_context_id.get(),
                extension_id.as_ptr(),
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Create a tab via the IPC bridge.
    pub fn create_tab(
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
            cbf_bridge_client_create_tab(self.inner, request_id, url.as_ptr(), profile.as_ptr())
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Request closing the specified tab.
    pub fn request_close_tab(&mut self, browsing_context_id: TabId) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe { cbf_bridge_client_request_close_tab(self.inner, browsing_context_id.get()) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Update the surface size of the specified tab.
    pub fn set_tab_size(
        &mut self,
        browsing_context_id: TabId,
        width: u32,
        height: u32,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe {
            cbf_bridge_client_set_tab_size(self.inner, browsing_context_id.get(), width, height)
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Update whether the specified tab should receive text input focus.
    pub fn set_tab_focus(
        &mut self,
        browsing_context_id: TabId,
        focused: bool,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe {
            cbf_bridge_client_set_tab_focus(self.inner, browsing_context_id.get(), focused)
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Respond to a beforeunload confirmation request.
    pub fn confirm_beforeunload(
        &mut self,
        browsing_context_id: TabId,
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

    /// Respond to a JavaScript dialog request for a tab.
    pub fn respond_javascript_dialog(
        &mut self,
        browsing_context_id: TabId,
        request_id: u64,
        accept: bool,
        prompt_text: Option<&str>,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let prompt_text = to_optional_cstring(&prompt_text.map(ToOwned::to_owned))
            .map_err(|_| Error::InvalidInput)?;

        if unsafe {
            cbf_bridge_client_respond_javascript_dialog(
                self.inner,
                browsing_context_id.get(),
                request_id,
                accept,
                prompt_text.as_ref().map_or(ptr::null(), |v| v.as_ptr()),
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Respond to a JavaScript dialog request for an extension popup.
    pub fn respond_extension_popup_javascript_dialog(
        &mut self,
        popup_id: PopupId,
        request_id: u64,
        accept: bool,
        prompt_text: Option<&str>,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let prompt_text = to_optional_cstring(&prompt_text.map(ToOwned::to_owned))
            .map_err(|_| Error::InvalidInput)?;

        if unsafe {
            cbf_bridge_client_respond_extension_popup_javascript_dialog(
                self.inner,
                popup_id.get(),
                request_id,
                accept,
                prompt_text.as_ref().map_or(ptr::null(), |v| v.as_ptr()),
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Navigate the page to the provided URL.
    pub fn navigate(&mut self, browsing_context_id: TabId, url: &str) -> Result<(), Error> {
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
    pub fn go_back(&mut self, browsing_context_id: TabId) -> Result<(), Error> {
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
    pub fn go_forward(&mut self, browsing_context_id: TabId) -> Result<(), Error> {
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
    pub fn reload(&mut self, browsing_context_id: TabId, ignore_cache: bool) -> Result<(), Error> {
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
    pub fn print_preview(&mut self, browsing_context_id: TabId) -> Result<(), Error> {
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
    pub fn open_dev_tools(&mut self, browsing_context_id: TabId) -> Result<(), Error> {
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
        browsing_context_id: TabId,
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
    pub fn get_tab_dom_html(
        &mut self,
        browsing_context_id: TabId,
        request_id: u64,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe {
            cbf_bridge_client_get_tab_dom_html(self.inner, browsing_context_id.get(), request_id)
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Open Chromium default PromptUi for pending request.
    pub fn open_default_prompt_ui(
        &mut self,
        profile_id: &str,
        request_id: u64,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }
        let profile_id = CString::new(profile_id).map_err(|_| Error::InvalidInput)?;
        if unsafe {
            cbf_bridge_client_open_default_prompt_ui(self.inner, profile_id.as_ptr(), request_id)
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Respond to a pending chrome-specific PromptUi request.
    pub fn respond_prompt_ui(
        &mut self,
        profile_id: &str,
        request_id: u64,
        response: &PromptUiResponse,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }
        let profile_id = CString::new(profile_id).map_err(|_| Error::InvalidInput)?;
        let (prompt_ui_kind, proceed, destination_path, report_abuse) = match response {
            PromptUiResponse::PermissionPrompt { allow } => {
                (CBF_PROMPT_UI_KIND_PERMISSION_PROMPT, *allow, None, false)
            }
            PromptUiResponse::DownloadPrompt {
                allow,
                destination_path,
            } => (
                CBF_PROMPT_UI_KIND_DOWNLOAD_PROMPT,
                *allow,
                to_optional_cstring(destination_path)?,
                false,
            ),
            PromptUiResponse::ExtensionInstallPrompt { proceed } => (
                CBF_PROMPT_UI_KIND_EXTENSION_INSTALL_PROMPT,
                *proceed,
                None,
                false,
            ),
            PromptUiResponse::ExtensionUninstallPrompt {
                proceed,
                report_abuse,
            } => (
                CBF_PROMPT_UI_KIND_EXTENSION_UNINSTALL_PROMPT,
                *proceed,
                None,
                *report_abuse,
            ),
            PromptUiResponse::PrintPreviewDialog { proceed } => (
                CBF_PROMPT_UI_KIND_PRINT_PREVIEW_DIALOG,
                *proceed,
                None,
                false,
            ),
            PromptUiResponse::Unknown => (CBF_PROMPT_UI_KIND_UNKNOWN, false, None, false),
        };
        if unsafe {
            cbf_bridge_client_respond_prompt_ui(
                self.inner,
                profile_id.as_ptr(),
                request_id,
                prompt_ui_kind,
                proceed,
                report_abuse,
                destination_path
                    .as_ref()
                    .map_or(ptr::null(), |path| path.as_ptr()),
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Respond to a page-originated prompt by resolving profile on the bridge side.
    pub fn respond_prompt_ui_for_tab(
        &mut self,
        browsing_context_id: TabId,
        request_id: u64,
        response: &PromptUiResponse,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }
        let (prompt_ui_kind, proceed, destination_path, report_abuse) = match response {
            PromptUiResponse::PermissionPrompt { allow } => {
                (CBF_PROMPT_UI_KIND_PERMISSION_PROMPT, *allow, None, false)
            }
            PromptUiResponse::DownloadPrompt {
                allow,
                destination_path,
            } => (
                CBF_PROMPT_UI_KIND_DOWNLOAD_PROMPT,
                *allow,
                to_optional_cstring(destination_path)?,
                false,
            ),
            PromptUiResponse::ExtensionInstallPrompt { proceed } => (
                CBF_PROMPT_UI_KIND_EXTENSION_INSTALL_PROMPT,
                *proceed,
                None,
                false,
            ),
            PromptUiResponse::ExtensionUninstallPrompt {
                proceed,
                report_abuse,
            } => (
                CBF_PROMPT_UI_KIND_EXTENSION_UNINSTALL_PROMPT,
                *proceed,
                None,
                *report_abuse,
            ),
            PromptUiResponse::PrintPreviewDialog { proceed } => (
                CBF_PROMPT_UI_KIND_PRINT_PREVIEW_DIALOG,
                *proceed,
                None,
                false,
            ),
            PromptUiResponse::Unknown => (CBF_PROMPT_UI_KIND_UNKNOWN, false, None, false),
        };
        if unsafe {
            cbf_bridge_client_respond_prompt_ui_for_tab(
                self.inner,
                browsing_context_id.get(),
                request_id,
                prompt_ui_kind,
                proceed,
                report_abuse,
                destination_path
                    .as_ref()
                    .map_or(ptr::null(), |path| path.as_ptr()),
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Close a backend-managed PromptUi surface.
    pub fn close_prompt_ui(
        &mut self,
        profile_id: &str,
        prompt_ui_id: PromptUiId,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }
        let profile_id = CString::new(profile_id).map_err(|_| Error::InvalidInput)?;
        if unsafe {
            cbf_bridge_client_close_prompt_ui(self.inner, profile_id.as_ptr(), prompt_ui_id.get())
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Pause an in-progress download.
    pub fn pause_download(&mut self, download_id: ChromeDownloadId) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }
        if unsafe { cbf_bridge_client_pause_download(self.inner, download_id.get()) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Resume a paused download.
    pub fn resume_download(&mut self, download_id: ChromeDownloadId) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }
        if unsafe { cbf_bridge_client_resume_download(self.inner, download_id.get()) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Cancel an active download.
    pub fn cancel_download(&mut self, download_id: ChromeDownloadId) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }
        if unsafe { cbf_bridge_client_cancel_download(self.inner, download_id.get()) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Respond to host-mediated tab-open request.
    pub fn respond_tab_open(
        &mut self,
        request_id: u64,
        response: &ChromeBrowsingContextOpenResponse,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }
        let (response_kind, target_tab_id, activate) = match response {
            ChromeBrowsingContextOpenResponse::AllowNewContext { activate } => {
                (CBF_TAB_OPEN_RESPONSE_ALLOW_NEW_CONTEXT, 0, *activate)
            }
            ChromeBrowsingContextOpenResponse::AllowExistingContext { tab_id, activate } => (
                CBF_TAB_OPEN_RESPONSE_ALLOW_EXISTING_CONTEXT,
                tab_id.get(),
                *activate,
            ),
            ChromeBrowsingContextOpenResponse::Deny => (CBF_TAB_OPEN_RESPONSE_DENY, 0, false),
        };
        if unsafe {
            cbf_bridge_client_respond_tab_open(
                self.inner,
                request_id,
                response_kind,
                target_tab_id,
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
    /// Current bridge path reuses tab-open response semantics.
    pub fn respond_window_open(
        &mut self,
        request_id: u64,
        response: &WindowOpenResponse,
    ) -> Result<(), Error> {
        let tab_open_response = match response {
            WindowOpenResponse::AllowExistingWindow { .. }
            | WindowOpenResponse::AllowNewWindow { .. } => {
                ChromeBrowsingContextOpenResponse::AllowNewContext { activate: true }
            }
            WindowOpenResponse::Deny => ChromeBrowsingContextOpenResponse::Deny,
        };
        self.respond_tab_open(request_id, &tab_open_response)
    }

    /// Send a Chromium-shaped keyboard event to the page.
    pub fn send_key_event_raw(
        &mut self,
        browsing_context_id: TabId,
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
            tab_id: browsing_context_id.get(),
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

    /// Send a Chromium-shaped keyboard event to an extension popup.
    pub fn send_extension_popup_key_event_raw(
        &mut self,
        popup_id: PopupId,
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
            tab_id: 0,
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

        if unsafe {
            cbf_bridge_client_send_extension_popup_key_event(
                self.inner,
                popup_id.get(),
                &ffi_event,
                &ffi_commands,
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Send a mouse event to the page.
    pub fn send_mouse_event(
        &mut self,
        browsing_context_id: TabId,
        event: &ChromeMouseEvent,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let ffi_event = CbfMouseEvent {
            tab_id: browsing_context_id.get(),
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

    /// Send a mouse event to an extension popup.
    pub fn send_extension_popup_mouse_event(
        &mut self,
        popup_id: PopupId,
        event: &ChromeMouseEvent,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let ffi_event = CbfMouseEvent {
            tab_id: 0,
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

        if unsafe {
            cbf_bridge_client_send_extension_popup_mouse_event(
                self.inner,
                popup_id.get(),
                &ffi_event,
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Send a Chromium-shaped mouse wheel event to the page.
    pub fn send_mouse_wheel_event_raw(
        &mut self,
        browsing_context_id: TabId,
        event: &ChromeMouseWheelEvent,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let ffi_event = CbfMouseWheelEvent {
            tab_id: browsing_context_id.get(),
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

    /// Send a Chromium-shaped mouse wheel event to an extension popup.
    pub fn send_extension_popup_mouse_wheel_event_raw(
        &mut self,
        popup_id: PopupId,
        event: &ChromeMouseWheelEvent,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let ffi_event = CbfMouseWheelEvent {
            tab_id: 0,
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

        if unsafe {
            cbf_bridge_client_send_extension_popup_mouse_wheel_event(
                self.inner,
                popup_id.get(),
                &ffi_event,
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Send a drag update event for host-owned drag session.
    pub fn send_drag_update(&mut self, update: &ChromeDragUpdate) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let ffi_update = CbfDragUpdate {
            session_id: update.session_id,
            tab_id: update.browsing_context_id.get(),
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
    pub fn send_drag_drop(&mut self, drop: &ChromeDragDrop) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let ffi_drop = CbfDragDrop {
            session_id: drop.session_id,
            tab_id: drop.browsing_context_id.get(),
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
        browsing_context_id: TabId,
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
    pub fn set_composition(&mut self, composition: &ChromeImeComposition) -> Result<(), Error> {
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
            tab_id: composition.browsing_context_id.get(),
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

    /// Update the IME composition state for an extension popup.
    pub fn set_extension_popup_composition(
        &mut self,
        composition: &ChromeTransientImeComposition,
    ) -> Result<(), Error> {
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
            tab_id: 0,
            text: text.as_ptr(),
            selection_start: composition.selection_start,
            selection_end: composition.selection_end,
            replacement_range_start: replacement_start,
            replacement_range_end: replacement_end,
            spans: span_list,
        };

        if unsafe {
            cbf_bridge_client_set_extension_popup_composition(
                self.inner,
                composition.popup_id.get(),
                &ffi_composition,
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Commit IME text input to the page.
    pub fn commit_text(&mut self, commit: &ChromeImeCommitText) -> Result<(), Error> {
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
            tab_id: commit.browsing_context_id.get(),
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

    /// Commit IME text input to an extension popup.
    pub fn commit_extension_popup_text(
        &mut self,
        commit: &ChromeTransientImeCommitText,
    ) -> Result<(), Error> {
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
            tab_id: 0,
            text: text.as_ptr(),
            relative_caret_position: commit.relative_caret_position,
            replacement_range_start: replacement_start,
            replacement_range_end: replacement_end,
            spans: span_list,
        };

        if unsafe {
            cbf_bridge_client_commit_extension_popup_text(
                self.inner,
                commit.popup_id.get(),
                &ffi_commit,
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Finish composing IME text with the specified behavior.
    pub fn finish_composing_text(
        &mut self,
        browsing_context_id: TabId,
        behavior: ChromeConfirmCompositionBehavior,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let behavior = match behavior {
            ChromeConfirmCompositionBehavior::DoNotKeepSelection => {
                CBF_IME_CONFIRM_DO_NOT_KEEP_SELECTION
            }
            ChromeConfirmCompositionBehavior::KeepSelection => CBF_IME_CONFIRM_KEEP_SELECTION,
        };

        if unsafe {
            cbf_bridge_client_finish_composing_text(self.inner, browsing_context_id.get(), behavior)
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Finish composing IME text inside an extension popup.
    pub fn finish_extension_popup_composing_text(
        &mut self,
        popup_id: PopupId,
        behavior: ChromeConfirmCompositionBehavior,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let behavior = match behavior {
            ChromeConfirmCompositionBehavior::DoNotKeepSelection => {
                CBF_IME_CONFIRM_DO_NOT_KEEP_SELECTION
            }
            ChromeConfirmCompositionBehavior::KeepSelection => CBF_IME_CONFIRM_KEEP_SELECTION,
        };

        if unsafe {
            cbf_bridge_client_finish_extension_popup_composing_text(
                self.inner,
                popup_id.get(),
                behavior,
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    pub fn set_extension_popup_focus(
        &mut self,
        popup_id: PopupId,
        focused: bool,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe {
            cbf_bridge_client_set_extension_popup_focus(self.inner, popup_id.get(), focused)
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    pub fn set_extension_popup_size(
        &mut self,
        popup_id: PopupId,
        width: u32,
        height: u32,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe {
            cbf_bridge_client_set_extension_popup_size(self.inner, popup_id.get(), width, height)
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    pub fn close_extension_popup(&mut self, popup_id: PopupId) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe { cbf_bridge_client_close_extension_popup(self.inner, popup_id.get()) } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Execute a browser-generic edit action for the given page.
    pub fn execute_edit_action(
        &mut self,
        browsing_context_id: TabId,
        action: EditAction,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe {
            cbf_bridge_client_execute_edit_action(
                self.inner,
                browsing_context_id.get(),
                edit_action_to_ffi(action),
            )
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Execute a browser-generic edit action for the given extension popup.
    pub fn execute_extension_popup_edit_action(
        &mut self,
        popup_id: PopupId,
        action: EditAction,
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe {
            cbf_bridge_client_execute_extension_popup_edit_action(
                self.inner,
                popup_id.get(),
                edit_action_to_ffi(action),
            )
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

    /// Accept a host-owned choice menu selection.
    pub fn accept_choice_menu_selection(
        &mut self,
        request_id: u64,
        indices: &[i32],
    ) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        let ffi_indices = CbfChoiceMenuSelectedIndices {
            items: indices.as_ptr(),
            len: indices.len() as u32,
        };
        if unsafe {
            cbf_bridge_client_accept_choice_menu_selection(self.inner, request_id, &ffi_indices)
        } {
            Ok(())
        } else {
            Err(Error::ConnectionFailed)
        }
    }

    /// Dismiss a host-owned choice menu without a selection.
    pub fn dismiss_choice_menu(&mut self, request_id: u64) -> Result<(), Error> {
        if self.inner.is_null() {
            return Err(Error::ConnectionFailed);
        }

        if unsafe { cbf_bridge_client_dismiss_choice_menu(self.inner, request_id) } {
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

impl IpcEventWaitHandle {
    pub(crate) fn wait_for_event(
        &self,
        timeout: Option<Duration>,
    ) -> Result<EventWaitResult, Error> {
        wait_for_event_inner(self.inner, timeout)
    }
}

fn wait_for_event_inner(
    inner: *mut CbfBridgeClientHandle,
    timeout: Option<Duration>,
) -> Result<EventWaitResult, Error> {
    if inner.is_null() {
        return Ok(EventWaitResult::Closed);
    }

    let timeout_ms = timeout
        .map(|value| value.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(-1);
    let status = unsafe { cbf_bridge_client_wait_for_event(inner, timeout_ms) };

    match status {
        CBF_BRIDGE_EVENT_WAIT_STATUS_EVENT_AVAILABLE => Ok(EventWaitResult::EventAvailable),
        CBF_BRIDGE_EVENT_WAIT_STATUS_TIMED_OUT => Ok(EventWaitResult::TimedOut),
        CBF_BRIDGE_EVENT_WAIT_STATUS_DISCONNECTED => Ok(EventWaitResult::Disconnected),
        CBF_BRIDGE_EVENT_WAIT_STATUS_CLOSED => Ok(EventWaitResult::Closed),
        _ => Err(Error::InvalidEvent),
    }
}

fn edit_action_to_ffi(action: EditAction) -> u8 {
    match action {
        EditAction::Undo => CBF_EDIT_ACTION_UNDO,
        EditAction::Redo => CBF_EDIT_ACTION_REDO,
        EditAction::Cut => CBF_EDIT_ACTION_CUT,
        EditAction::Copy => CBF_EDIT_ACTION_COPY,
        EditAction::Paste => CBF_EDIT_ACTION_PASTE,
        EditAction::SelectAll => CBF_EDIT_ACTION_SELECT_ALL,
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
