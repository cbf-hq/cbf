#![allow(non_upper_case_globals)]

use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
    ptr,
    time::Duration,
};

use cbf::data::{edit::EditAction, window_open::WindowOpenResponse};
use cbf_chrome_sys::{
    bridge::{BridgeLibrary, BridgeLoadError, bridge},
    ffi::*,
};
use tracing::warn;

use super::map::{
    ime_range_to_ffi, key_event_type_to_ffi, mouse_button_to_ffi, mouse_event_type_to_ffi,
    parse_event, parse_extension_list, pointer_type_to_ffi, scroll_granularity_to_ffi,
    to_ffi_ime_text_spans,
};
use super::utils::{c_string_to_string, to_optional_cstring};
use super::{BridgeError, IpcEvent};
use crate::data::{
    background::ChromeBackgroundPolicy,
    browsing_context_open::ChromeBrowsingContextOpenResponse,
    custom_scheme::{ChromeCustomSchemeResponse, ChromeCustomSchemeResponseResult},
    download::ChromeDownloadId,
    drag::{
        ChromeDragData, ChromeDragDrop, ChromeDragUpdate, ChromeExternalDragDrop,
        ChromeExternalDragEnter, ChromeExternalDragUpdate,
    },
    extension::ChromeExtensionInfo,
    find::{ChromeFindInPageOptions, ChromeStopFindAction},
    ids::{PopupId, TabId},
    ime::{
        ChromeConfirmCompositionBehavior, ChromeImeCommitText, ChromeImeComposition,
        ChromeTransientImeCommitText, ChromeTransientImeComposition,
    },
    input::{ChromeKeyEvent, ChromeMouseWheelEvent},
    ipc::{TabIpcConfig, TabIpcErrorCode, TabIpcMessage, TabIpcMessageType, TabIpcPayload},
    mouse::ChromeMouseEvent,
    policy::{ChromeBrowsingContextPolicy, ChromeCapabilityPolicy, ChromeIpcPolicy},
    profile::ChromeProfileInfo,
    prompt_ui::{PromptUiId, PromptUiResponse},
    visibility::ChromeTabVisibility,
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

macro_rules! bridge_call {
    ($method:ident ( $($arg:expr),* $(,)? )) => {{
        let bridge = bridge_api()?;
        unsafe { bridge.$method($($arg),*) }
    }};
}

impl std::fmt::Debug for IpcClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IpcClient")
            .field("inner", &format!("{:p}", self.inner))
            .finish()
    }
}

impl IpcClient {
    /// Override the base bundle ID used for Mach rendezvous on macOS.
    ///
    /// Must be called before `prepare_channel`.
    pub fn set_base_bundle_id(bundle_id: &str) -> Result<(), BridgeError> {
        let bundle_id = CString::new(bundle_id).map_err(|_| BridgeError::InvalidInput)?;
        bridge_call!(cbf_bridge_set_base_bundle_id(bundle_id.as_ptr()));

        Ok(())
    }

    /// Prepare the Mojo channel before spawning the Chromium process.
    ///
    /// Returns `(remote_fd, switch_arg)` where:
    /// - `remote_fd` is the file descriptor of the remote channel endpoint that
    ///   must be inherited by the child process (Unix only; -1 on other platforms).
    /// - `switch_arg` is the command-line switch that Chromium needs to recover
    ///   the endpoint (e.g. `--cbf-ipc-handle=...`).
    pub fn prepare_channel() -> Result<(i32, String), BridgeError> {
        let mut buf = [0u8; 512];

        let fd = {
            bridge_call!(cbf_bridge_init());
            bridge_call!(cbf_bridge_prepare_channel(
                buf.as_mut_ptr() as *mut std::os::raw::c_char,
                buf.len() as i32,
            ))
        };

        let switch_arg = parse_channel_switch_arg(&buf)?;

        Ok((fd, switch_arg))
    }

    /// Notify the bridge of the spawned child's PID.
    ///
    /// Must be called after spawning the Chromium process and before
    /// `connect_inherited`. On macOS this registers the Mach port with the
    /// rendezvous server; on other platforms it completes channel bookkeeping.
    pub fn pass_child_pid(pid: u32) {
        if let Err(err) =
            bridge_api().map(|bridge| unsafe { bridge.cbf_bridge_pass_child_pid(pid as i64) })
        {
            warn!(error = ?err, "failed to pass child pid to bridge");
        }
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
    pub unsafe fn connect_inherited(
        inner: *mut CbfBridgeClientHandle,
    ) -> Result<Self, BridgeError> {
        if inner.is_null() {
            return Err(BridgeError::InvalidState);
        }

        let connected = bridge_call!(cbf_bridge_client_connect_inherited(inner));

        if !connected {
            warn!(
                result = "err",
                error = "ipc_connect_inherited_failed",
                "IPC inherited connect failed"
            );
            cleanup_bridge_call("destroy bridge client after connect failure", |bridge| {
                unsafe { bridge.cbf_bridge_client_destroy(inner) };
            });

            return Err(BridgeError::ConnectionFailed);
        }

        Ok(Self { inner })
    }

    /// Authenticate with the session token and set up the browser observer.
    ///
    /// Must be called once after `connect_inherited` and before any other method.
    pub fn authenticate(&self, token: &str) -> Result<(), BridgeError> {
        if self.inner.is_null() {
            return Err(BridgeError::InvalidState);
        }

        let token = CString::new(token).map_err(|_| BridgeError::InvalidInput)?;
        authentication_result(bridge_call!(cbf_bridge_client_authenticate(
            self.inner,
            token.as_ptr()
        )))
    }

    /// Wait until an event is available or the bridge closes.
    pub fn wait_for_event(
        &self,
        timeout: Option<Duration>,
    ) -> Result<EventWaitResult, BridgeError> {
        wait_for_event_inner(self.inner, timeout)
    }

    pub(crate) fn event_wait_handle(&self) -> IpcEventWaitHandle {
        IpcEventWaitHandle { inner: self.inner }
    }

    /// Poll the next IPC event, if any, from the backend.
    pub fn poll_event(&mut self) -> Option<Result<IpcEvent, BridgeError>> {
        if self.inner.is_null() {
            return None;
        }

        let mut event = CbfBridgeEvent::default();
        let polled = bridge_api()
            .map(|bridge| unsafe { bridge.cbf_bridge_client_poll_event(self.inner, &mut event) })
            .ok()?;

        if !polled {
            return None;
        }

        let parsed = parse_event(event);
        cleanup_bridge_call("free bridge event", |bridge| {
            unsafe { bridge.cbf_bridge_event_free(&mut event) };
        });

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
    pub fn list_profiles(&mut self) -> Result<Vec<ChromeProfileInfo>, BridgeError> {
        self.ensure_ready()?;

        let mut list = CbfProfileList::default();
        if !bridge_call!(cbf_bridge_client_get_profiles(self.inner, &mut list)) {
            return Err(BridgeError::OperationFailed {
                operation: "list_profiles",
            });
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

        cleanup_bridge_call("free profile list", |bridge| {
            unsafe { bridge.cbf_bridge_profile_list_free(&mut list) };
        });

        Ok(result)
    }

    /// Retrieve the list of extensions from the backend.
    pub fn list_extensions(
        &mut self,
        profile_id: &str,
    ) -> Result<Vec<ChromeExtensionInfo>, BridgeError> {
        self.ensure_ready()?;

        let profile = CString::new(profile_id).map_err(|_| BridgeError::InvalidInput)?;

        let mut list = CbfExtensionInfoList::default();
        if !bridge_call!(cbf_bridge_client_list_extensions(
            self.inner,
            profile.as_ptr(),
            &mut list
        )) {
            return Err(BridgeError::OperationFailed {
                operation: "list_extensions",
            });
        }

        let result = parse_extension_list(list);

        cleanup_bridge_call("free extension list", |bridge| {
            unsafe { bridge.cbf_bridge_extension_list_free(&mut list) };
        });
        Ok(result)
    }

    pub fn register_custom_scheme_handler(
        &mut self,
        scheme: &str,
        host: &str,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let scheme = CString::new(scheme).map_err(|_| BridgeError::InvalidInput)?;
        let host = CString::new(host).map_err(|_| BridgeError::InvalidInput)?;
        bridge_ok(
            "register_custom_scheme_handler",
            bridge_call!(cbf_bridge_client_register_custom_scheme_handler(
                self.inner,
                scheme.as_ptr(),
                host.as_ptr(),
            )),
        )
    }

    pub fn respond_custom_scheme_request(
        &mut self,
        response: &ChromeCustomSchemeResponse,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let mime_type =
            CString::new(response.mime_type.as_str()).map_err(|_| BridgeError::InvalidInput)?;
        let content_security_policy = to_optional_cstring(&response.content_security_policy)
            .map_err(|_| BridgeError::InvalidInput)?;
        let access_control_allow_origin =
            to_optional_cstring(&response.access_control_allow_origin)
                .map_err(|_| BridgeError::InvalidInput)?;

        let result = match response.result {
            ChromeCustomSchemeResponseResult::Ok => {
                CbfCustomSchemeResponseResult_kCbfCustomSchemeResponseResultOk
            }
            ChromeCustomSchemeResponseResult::NotFound => {
                CbfCustomSchemeResponseResult_kCbfCustomSchemeResponseResultNotFound
            }
            ChromeCustomSchemeResponseResult::Aborted => {
                CbfCustomSchemeResponseResult_kCbfCustomSchemeResponseResultAborted
            }
        } as u8;

        let body_ptr = if response.body.is_empty() {
            ptr::null()
        } else {
            response.body.as_ptr()
        };

        bridge_ok(
            "respond_custom_scheme_request",
            bridge_call!(cbf_bridge_client_respond_custom_scheme_request(
                self.inner,
                response.request_id,
                result,
                mime_type.as_ptr(),
                content_security_policy
                    .as_ref()
                    .map_or(ptr::null(), |value| value.as_ptr()),
                access_control_allow_origin
                    .as_ref()
                    .map_or(ptr::null(), |value| value.as_ptr()),
                body_ptr,
                response.body.len() as u32,
            )),
        )
    }

    pub fn activate_extension_action(
        &mut self,
        browsing_context_id: TabId,
        extension_id: &str,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let extension_id = CString::new(extension_id).map_err(|_| BridgeError::InvalidInput)?;
        bridge_ok(
            "activate_extension_action",
            bridge_call!(cbf_bridge_client_activate_extension_action(
                self.inner,
                browsing_context_id.get(),
                extension_id.as_ptr(),
            )),
        )
    }

    /// Create a tab via the IPC bridge.
    pub fn create_tab(
        &mut self,
        request_id: u64,
        initial_url: &str,
        profile_id: &str,
        policy: Option<&ChromeBrowsingContextPolicy>,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let url = CString::new(initial_url).map_err(|_| BridgeError::InvalidInput)?;
        let profile = CString::new(profile_id).map_err(|_| BridgeError::InvalidInput)?;
        let allowed_origins = match policy.map(|policy| &policy.ipc) {
            Some(ChromeIpcPolicy::Allow { allowed_origins }) => Some(
                allowed_origins
                    .iter()
                    .map(|origin| CString::new(origin.as_str()))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| BridgeError::InvalidInput)?,
            ),
            _ => None,
        };

        let allowed_origin_ptrs: Vec<*const c_char> = allowed_origins
            .as_ref()
            .map(|origins| origins.iter().map(|origin| origin.as_ptr()).collect())
            .unwrap_or_default();
        let allowed_origin_list = CbfCommandList {
            items: if allowed_origin_ptrs.is_empty() {
                ptr::null_mut()
            } else {
                allowed_origin_ptrs.as_ptr() as *mut *const c_char
            },
            len: allowed_origin_ptrs.len() as u32,
        };

        let (has_policy, ipc_policy_kind, extensions_policy) = match policy {
            Some(policy) => (
                true,
                match policy.ipc {
                    ChromeIpcPolicy::Deny => {
                        CbfBrowsingContextIpcPolicy_kCbfBrowsingContextIpcPolicyDeny as u8
                    }
                    ChromeIpcPolicy::Allow { .. } => {
                        CbfBrowsingContextIpcPolicy_kCbfBrowsingContextIpcPolicyAllow as u8
                    }
                },
                match policy.extensions {
                    ChromeCapabilityPolicy::Allow => {
                        CbfCapabilityPolicy_kCbfCapabilityPolicyAllow as u8
                    }
                    ChromeCapabilityPolicy::Deny => {
                        CbfCapabilityPolicy_kCbfCapabilityPolicyDeny as u8
                    }
                },
            ),
            None => (
                false,
                CbfBrowsingContextIpcPolicy_kCbfBrowsingContextIpcPolicyDeny as u8,
                CbfCapabilityPolicy_kCbfCapabilityPolicyAllow as u8,
            ),
        };

        bridge_ok(
            "create_tab",
            bridge_call!(cbf_bridge_client_create_tab(
                self.inner,
                request_id,
                url.as_ptr(),
                profile.as_ptr(),
                has_policy,
                ipc_policy_kind,
                &allowed_origin_list,
                extensions_policy
            )),
        )
    }

    /// Request closing the specified tab.
    pub fn request_close_tab(&mut self, browsing_context_id: TabId) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "request_close_tab",
            bridge_call!(cbf_bridge_client_request_close_tab(
                self.inner,
                browsing_context_id.get()
            )),
        )
    }

    /// Update the surface size of the specified tab.
    pub fn set_tab_size(
        &mut self,
        browsing_context_id: TabId,
        width: u32,
        height: u32,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "set_tab_size",
            bridge_call!(cbf_bridge_client_set_tab_size(
                self.inner,
                browsing_context_id.get(),
                width,
                height
            )),
        )
    }

    /// Update whether the specified tab should receive text input focus.
    pub fn set_tab_focus(
        &mut self,
        browsing_context_id: TabId,
        focused: bool,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "set_tab_focus",
            bridge_call!(cbf_bridge_client_set_tab_focus(
                self.inner,
                browsing_context_id.get(),
                focused
            )),
        )
    }

    /// Update whether the specified tab should be treated as visible.
    pub fn set_tab_visibility(
        &mut self,
        browsing_context_id: TabId,
        visibility: ChromeTabVisibility,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let visibility = match visibility {
            ChromeTabVisibility::Visible => CbfTabVisibility_kCbfTabVisibilityVisible,
            ChromeTabVisibility::Hidden => CbfTabVisibility_kCbfTabVisibilityHidden,
        } as u8;

        bridge_ok(
            "set_tab_visibility",
            bridge_call!(cbf_bridge_client_set_tab_visibility(
                self.inner,
                browsing_context_id.get(),
                visibility,
            )),
        )
    }

    /// Enable browsing context IPC with explicit origin allow list.
    pub fn enable_tab_ipc(
        &mut self,
        browsing_context_id: TabId,
        config: &TabIpcConfig,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let origin_cstrings: Result<Vec<CString>, _> = config
            .allowed_origins
            .iter()
            .map(|origin| CString::new(origin.as_str()))
            .collect();
        let origin_cstrings = origin_cstrings.map_err(|_| BridgeError::InvalidInput)?;
        let origin_ptrs: Vec<*const c_char> = origin_cstrings.iter().map(|s| s.as_ptr()).collect();

        let list = CbfCommandList {
            items: if origin_ptrs.is_empty() {
                ptr::null_mut()
            } else {
                origin_ptrs.as_ptr() as *mut *const c_char
            },
            len: origin_ptrs.len() as u32,
        };

        bridge_ok(
            "enable_tab_ipc",
            bridge_call!(cbf_bridge_client_enable_tab_ipc(
                self.inner,
                browsing_context_id.get(),
                &list
            )),
        )
    }

    /// Disable browsing context IPC.
    pub fn disable_tab_ipc(&mut self, browsing_context_id: TabId) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "disable_tab_ipc",
            bridge_call!(cbf_bridge_client_disable_tab_ipc(
                self.inner,
                browsing_context_id.get()
            )),
        )
    }

    /// Post host -> page IPC message.
    pub fn post_tab_ipc_message(
        &mut self,
        browsing_context_id: TabId,
        message: &TabIpcMessage,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let channel =
            CString::new(message.channel.as_str()).map_err(|_| BridgeError::InvalidInput)?;
        let content_type =
            to_optional_cstring(&message.content_type).map_err(|_| BridgeError::InvalidInput)?;
        let payload_kind = match message.payload {
            TabIpcPayload::Text(_) => CbfIpcPayloadKind_kCbfIpcPayloadText,
            TabIpcPayload::Binary(_) => CbfIpcPayloadKind_kCbfIpcPayloadBinary,
        } as u8;
        let message_type = ipc_message_type_to_ffi(message.message_type);
        let error_code = message
            .error_code
            .map(ipc_error_code_to_ffi)
            .unwrap_or(CbfIpcErrorCode_kCbfIpcErrorNone as u8);
        let (payload_text, payload_binary) = match &message.payload {
            TabIpcPayload::Text(text) => (
                Some(CString::new(text.as_str()).map_err(|_| BridgeError::InvalidInput)?),
                Vec::new(),
            ),
            TabIpcPayload::Binary(binary) => (None, binary.clone()),
        };

        bridge_ok(
            "post_tab_ipc_message",
            bridge_call!(cbf_bridge_client_post_tab_ipc_message(
                self.inner,
                browsing_context_id.get(),
                channel.as_ptr(),
                message_type,
                message.request_id,
                payload_kind,
                payload_text
                    .as_ref()
                    .map(|value| value.as_ptr())
                    .unwrap_or(ptr::null()),
                if payload_binary.is_empty() {
                    ptr::null()
                } else {
                    payload_binary.as_ptr()
                },
                payload_binary.len() as u32,
                content_type
                    .as_ref()
                    .map(|value| value.as_ptr())
                    .unwrap_or(ptr::null()),
                error_code,
            )),
        )
    }

    /// Update the page background policy of the specified tab.
    pub fn set_tab_background_policy(
        &mut self,
        browsing_context_id: TabId,
        policy: ChromeBackgroundPolicy,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let transparent = matches!(policy, ChromeBackgroundPolicy::Transparent);

        bridge_ok(
            "set_tab_background_policy",
            bridge_call!(cbf_bridge_client_set_tab_background_policy(
                self.inner,
                browsing_context_id.get(),
                transparent,
            )),
        )
    }

    /// Respond to a beforeunload confirmation request.
    pub fn confirm_beforeunload(
        &mut self,
        browsing_context_id: TabId,
        request_id: u64,
        proceed: bool,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "confirm_beforeunload",
            bridge_call!(cbf_bridge_client_confirm_beforeunload(
                self.inner,
                browsing_context_id.get(),
                request_id,
                proceed,
            )),
        )
    }

    /// Respond to a JavaScript dialog request for a tab.
    pub fn respond_javascript_dialog(
        &mut self,
        browsing_context_id: TabId,
        request_id: u64,
        accept: bool,
        prompt_text: Option<&str>,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let prompt_text = to_optional_cstring(&prompt_text.map(ToOwned::to_owned))
            .map_err(|_| BridgeError::InvalidInput)?;

        bridge_ok(
            "respond_javascript_dialog",
            bridge_call!(cbf_bridge_client_respond_javascript_dialog(
                self.inner,
                browsing_context_id.get(),
                request_id,
                accept,
                prompt_text.as_ref().map_or(ptr::null(), |v| v.as_ptr()),
            )),
        )
    }

    /// Respond to a JavaScript dialog request for an extension popup.
    pub fn respond_extension_popup_javascript_dialog(
        &mut self,
        popup_id: PopupId,
        request_id: u64,
        accept: bool,
        prompt_text: Option<&str>,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let prompt_text = to_optional_cstring(&prompt_text.map(ToOwned::to_owned))
            .map_err(|_| BridgeError::InvalidInput)?;

        bridge_ok(
            "respond_extension_popup_javascript_dialog",
            bridge_call!(cbf_bridge_client_respond_extension_popup_javascript_dialog(
                self.inner,
                popup_id.get(),
                request_id,
                accept,
                prompt_text.as_ref().map_or(ptr::null(), |v| v.as_ptr()),
            )),
        )
    }

    /// Navigate the page to the provided URL.
    pub fn navigate(&mut self, browsing_context_id: TabId, url: &str) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let url = CString::new(url).map_err(|_| BridgeError::InvalidInput)?;

        bridge_ok(
            "navigate",
            bridge_call!(cbf_bridge_client_navigate(
                self.inner,
                browsing_context_id.get(),
                url.as_ptr()
            )),
        )
    }

    /// Navigate back in history for the page.
    pub fn go_back(&mut self, browsing_context_id: TabId) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "go_back",
            bridge_call!(cbf_bridge_client_go_back(
                self.inner,
                browsing_context_id.get()
            )),
        )
    }

    /// Navigate forward in history for the page.
    pub fn go_forward(&mut self, browsing_context_id: TabId) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "go_forward",
            bridge_call!(cbf_bridge_client_go_forward(
                self.inner,
                browsing_context_id.get()
            )),
        )
    }

    /// Reload the page, optionally ignoring caches.
    pub fn reload(
        &mut self,
        browsing_context_id: TabId,
        ignore_cache: bool,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "reload",
            bridge_call!(cbf_bridge_client_reload(
                self.inner,
                browsing_context_id.get(),
                ignore_cache
            )),
        )
    }

    /// Open print preview for the page.
    pub fn print_preview(&mut self, browsing_context_id: TabId) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "print_preview",
            bridge_call!(cbf_bridge_client_print_preview(
                self.inner,
                browsing_context_id.get()
            )),
        )
    }

    /// Open DevTools for the specified page.
    pub fn open_dev_tools(&mut self, browsing_context_id: TabId) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "open_dev_tools",
            bridge_call!(cbf_bridge_client_open_dev_tools(
                self.inner,
                browsing_context_id.get()
            )),
        )
    }

    /// Open DevTools and inspect the element at the given coordinates.
    pub fn inspect_element(
        &mut self,
        browsing_context_id: TabId,
        x: i32,
        y: i32,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "inspect_element",
            bridge_call!(cbf_bridge_client_inspect_element(
                self.inner,
                browsing_context_id.get(),
                x,
                y
            )),
        )
    }

    /// Request the DOM HTML for the specified page.
    pub fn get_tab_dom_html(
        &mut self,
        browsing_context_id: TabId,
        request_id: u64,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "get_tab_dom_html",
            bridge_call!(cbf_bridge_client_get_tab_dom_html(
                self.inner,
                browsing_context_id.get(),
                request_id,
            )),
        )
    }

    pub fn find_in_page(
        &mut self,
        browsing_context_id: TabId,
        request_id: u64,
        options: &ChromeFindInPageOptions,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let query = CString::new(options.query.as_str()).map_err(|_| BridgeError::InvalidInput)?;
        bridge_ok(
            "find_in_page",
            bridge_call!(cbf_bridge_client_find_in_page(
                self.inner,
                browsing_context_id.get(),
                request_id,
                query.as_ptr(),
                options.forward,
                options.match_case,
                options.new_session,
                options.find_match,
            )),
        )
    }

    pub fn stop_finding(
        &mut self,
        browsing_context_id: TabId,
        action: ChromeStopFindAction,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "stop_finding",
            bridge_call!(cbf_bridge_client_stop_finding(
                self.inner,
                browsing_context_id.get(),
                action.to_ffi(),
            )),
        )
    }

    /// Open Chromium default PromptUi for pending request.
    pub fn open_default_prompt_ui(
        &mut self,
        profile_id: &str,
        request_id: u64,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;
        let profile_id = CString::new(profile_id).map_err(|_| BridgeError::InvalidInput)?;
        bridge_ok(
            "open_default_prompt_ui",
            bridge_call!(cbf_bridge_client_open_default_prompt_ui(
                self.inner,
                profile_id.as_ptr(),
                request_id,
            )),
        )
    }

    /// Respond to a pending chrome-specific PromptUi request.
    pub fn respond_prompt_ui(
        &mut self,
        profile_id: &str,
        request_id: u64,
        response: &PromptUiResponse,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;
        let profile_id = CString::new(profile_id).map_err(|_| BridgeError::InvalidInput)?;
        let (prompt_ui_kind, proceed, destination_path, report_abuse) = match response {
            PromptUiResponse::PermissionPrompt { allow } => (
                CbfPromptUiKind_kCbfPromptUiKindPermissionPrompt,
                *allow,
                None,
                false,
            ),
            PromptUiResponse::DownloadPrompt {
                allow,
                destination_path,
            } => (
                CbfPromptUiKind_kCbfPromptUiKindDownloadPrompt,
                *allow,
                to_optional_cstring(destination_path)?,
                false,
            ),
            PromptUiResponse::ExtensionInstallPrompt { proceed } => (
                CbfPromptUiKind_kCbfPromptUiKindExtensionInstallPrompt,
                *proceed,
                None,
                false,
            ),
            PromptUiResponse::ExtensionUninstallPrompt {
                proceed,
                report_abuse,
            } => (
                CbfPromptUiKind_kCbfPromptUiKindExtensionUninstallPrompt,
                *proceed,
                None,
                *report_abuse,
            ),
            PromptUiResponse::PrintPreviewDialog { proceed } => (
                CbfPromptUiKind_kCbfPromptUiKindPrintPreviewDialog,
                *proceed,
                None,
                false,
            ),
            PromptUiResponse::FormResubmissionPrompt { proceed } => (
                CbfPromptUiKind_kCbfPromptUiKindFormResubmissionPrompt,
                *proceed,
                None,
                false,
            ),
            PromptUiResponse::Unknown => {
                (CbfPromptUiKind_kCbfPromptUiKindUnknown, false, None, false)
            }
        };
        bridge_ok(
            "respond_prompt_ui",
            bridge_call!(cbf_bridge_client_respond_prompt_ui(
                self.inner,
                profile_id.as_ptr(),
                request_id,
                prompt_ui_kind as u8,
                proceed,
                report_abuse,
                destination_path
                    .as_ref()
                    .map_or(ptr::null(), |path| path.as_ptr()),
            )),
        )
    }

    /// Respond to a page-originated prompt by resolving profile on the bridge side.
    pub fn respond_prompt_ui_for_tab(
        &mut self,
        browsing_context_id: TabId,
        request_id: u64,
        response: &PromptUiResponse,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;
        let (prompt_ui_kind, proceed, destination_path, report_abuse) = match response {
            PromptUiResponse::PermissionPrompt { allow } => (
                CbfPromptUiKind_kCbfPromptUiKindPermissionPrompt,
                *allow,
                None,
                false,
            ),
            PromptUiResponse::DownloadPrompt {
                allow,
                destination_path,
            } => (
                CbfPromptUiKind_kCbfPromptUiKindDownloadPrompt,
                *allow,
                to_optional_cstring(destination_path)?,
                false,
            ),
            PromptUiResponse::ExtensionInstallPrompt { proceed } => (
                CbfPromptUiKind_kCbfPromptUiKindExtensionInstallPrompt,
                *proceed,
                None,
                false,
            ),
            PromptUiResponse::ExtensionUninstallPrompt {
                proceed,
                report_abuse,
            } => (
                CbfPromptUiKind_kCbfPromptUiKindExtensionUninstallPrompt,
                *proceed,
                None,
                *report_abuse,
            ),
            PromptUiResponse::PrintPreviewDialog { proceed } => (
                CbfPromptUiKind_kCbfPromptUiKindPrintPreviewDialog,
                *proceed,
                None,
                false,
            ),
            PromptUiResponse::FormResubmissionPrompt { proceed } => (
                CbfPromptUiKind_kCbfPromptUiKindFormResubmissionPrompt,
                *proceed,
                None,
                false,
            ),
            PromptUiResponse::Unknown => {
                (CbfPromptUiKind_kCbfPromptUiKindUnknown, false, None, false)
            }
        };
        bridge_ok(
            "respond_prompt_ui_for_tab",
            bridge_call!(cbf_bridge_client_respond_prompt_ui_for_tab(
                self.inner,
                browsing_context_id.get(),
                request_id,
                prompt_ui_kind as u8,
                proceed,
                report_abuse,
                destination_path
                    .as_ref()
                    .map_or(ptr::null(), |path| path.as_ptr()),
            )),
        )
    }

    /// Close a backend-managed PromptUi surface.
    pub fn close_prompt_ui(
        &mut self,
        profile_id: &str,
        prompt_ui_id: PromptUiId,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;
        let profile_id = CString::new(profile_id).map_err(|_| BridgeError::InvalidInput)?;
        bridge_ok(
            "close_prompt_ui",
            bridge_call!(cbf_bridge_client_close_prompt_ui(
                self.inner,
                profile_id.as_ptr(),
                prompt_ui_id.get(),
            )),
        )
    }

    /// Pause an in-progress download.
    pub fn pause_download(&mut self, download_id: ChromeDownloadId) -> Result<(), BridgeError> {
        self.ensure_ready()?;
        bridge_ok(
            "pause_download",
            bridge_call!(cbf_bridge_client_pause_download(
                self.inner,
                download_id.get()
            )),
        )
    }

    /// Resume a paused download.
    pub fn resume_download(&mut self, download_id: ChromeDownloadId) -> Result<(), BridgeError> {
        self.ensure_ready()?;
        bridge_ok(
            "resume_download",
            bridge_call!(cbf_bridge_client_resume_download(
                self.inner,
                download_id.get()
            )),
        )
    }

    /// Cancel an active download.
    pub fn cancel_download(&mut self, download_id: ChromeDownloadId) -> Result<(), BridgeError> {
        self.ensure_ready()?;
        bridge_ok(
            "cancel_download",
            bridge_call!(cbf_bridge_client_cancel_download(
                self.inner,
                download_id.get()
            )),
        )
    }

    /// Respond to host-mediated tab-open request.
    pub fn respond_tab_open(
        &mut self,
        request_id: u64,
        response: &ChromeBrowsingContextOpenResponse,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;
        let (response_kind, target_tab_id, activate) = match response {
            ChromeBrowsingContextOpenResponse::AllowNewContext { activate } => (
                CbfTabOpenResponseKind_kCbfTabOpenResponseAllowNewContext,
                0,
                *activate,
            ),
            ChromeBrowsingContextOpenResponse::AllowExistingContext { tab_id, activate } => (
                CbfTabOpenResponseKind_kCbfTabOpenResponseAllowExistingContext,
                tab_id.get(),
                *activate,
            ),
            ChromeBrowsingContextOpenResponse::Deny => {
                (CbfTabOpenResponseKind_kCbfTabOpenResponseDeny, 0, false)
            }
        };
        bridge_ok(
            "respond_tab_open",
            bridge_call!(cbf_bridge_client_respond_tab_open(
                self.inner,
                request_id,
                response_kind as u8,
                target_tab_id,
                activate,
            )),
        )
    }

    /// Respond to host-mediated window open request.
    ///
    /// Current bridge path reuses tab-open response semantics.
    pub fn respond_window_open(
        &mut self,
        request_id: u64,
        response: &WindowOpenResponse,
    ) -> Result<(), BridgeError> {
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
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let dom_code = to_optional_cstring(&event.dom_code)?;
        let dom_key = to_optional_cstring(&event.dom_key)?;
        let text = to_optional_cstring(&event.text)?;
        let unmodified_text = to_optional_cstring(&event.unmodified_text)?;

        let command_cstrings = commands
            .iter()
            .map(|command| CString::new(command.as_str()).map_err(|_| BridgeError::InvalidInput))
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
                ptr::null_mut()
            } else {
                command_ptrs.as_ptr() as *mut *const c_char
            },
            len: command_ptrs.len() as u32,
        };

        bridge_ok(
            "send_key_event",
            bridge_call!(cbf_bridge_client_send_key_event(
                self.inner,
                &ffi_event,
                &ffi_commands
            )),
        )
    }

    /// Send a Chromium-shaped keyboard event to an extension popup.
    pub fn send_extension_popup_key_event_raw(
        &mut self,
        popup_id: PopupId,
        event: &ChromeKeyEvent,
        commands: &[String],
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let dom_code = to_optional_cstring(&event.dom_code)?;
        let dom_key = to_optional_cstring(&event.dom_key)?;
        let text = to_optional_cstring(&event.text)?;
        let unmodified_text = to_optional_cstring(&event.unmodified_text)?;

        let command_cstrings = commands
            .iter()
            .map(|command| CString::new(command.as_str()).map_err(|_| BridgeError::InvalidInput))
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
                ptr::null_mut()
            } else {
                command_ptrs.as_ptr() as *mut *const c_char
            },
            len: command_ptrs.len() as u32,
        };

        bridge_ok(
            "send_extension_popup_key_event_raw",
            bridge_call!(cbf_bridge_client_send_extension_popup_key_event(
                self.inner,
                popup_id.get(),
                &ffi_event,
                &ffi_commands,
            )),
        )
    }

    /// Send a mouse event to the page.
    pub fn send_mouse_event(
        &mut self,
        browsing_context_id: TabId,
        event: &ChromeMouseEvent,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

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

        bridge_ok(
            "send_mouse_event",
            bridge_call!(cbf_bridge_client_send_mouse_event(self.inner, &ffi_event)),
        )
    }

    /// Send a mouse event to an extension popup.
    pub fn send_extension_popup_mouse_event(
        &mut self,
        popup_id: PopupId,
        event: &ChromeMouseEvent,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

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

        bridge_ok(
            "send_extension_popup_mouse_event",
            bridge_call!(cbf_bridge_client_send_extension_popup_mouse_event(
                self.inner,
                popup_id.get(),
                &ffi_event,
            )),
        )
    }

    /// Send a Chromium-shaped mouse wheel event to the page.
    pub fn send_mouse_wheel_event_raw(
        &mut self,
        browsing_context_id: TabId,
        event: &ChromeMouseWheelEvent,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

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

        bridge_ok(
            "send_mouse_wheel_event",
            bridge_call!(cbf_bridge_client_send_mouse_wheel_event(
                self.inner, &ffi_event
            )),
        )
    }

    /// Send a Chromium-shaped mouse wheel event to an extension popup.
    pub fn send_extension_popup_mouse_wheel_event_raw(
        &mut self,
        popup_id: PopupId,
        event: &ChromeMouseWheelEvent,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

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

        bridge_ok(
            "send_extension_popup_mouse_wheel_event_raw",
            bridge_call!(cbf_bridge_client_send_extension_popup_mouse_wheel_event(
                self.inner,
                popup_id.get(),
                &ffi_event,
            )),
        )
    }

    /// Send a drag update event for host-owned drag session.
    pub fn send_drag_update(&mut self, update: &ChromeDragUpdate) -> Result<(), BridgeError> {
        self.ensure_ready()?;

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

        bridge_ok(
            "send_drag_update",
            bridge_call!(cbf_bridge_client_send_drag_update(self.inner, &ffi_update)),
        )
    }

    /// Send a drag drop event for host-owned drag session.
    pub fn send_drag_drop(&mut self, drop: &ChromeDragDrop) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let ffi_drop = CbfDragDrop {
            session_id: drop.session_id,
            tab_id: drop.browsing_context_id.get(),
            modifiers: drop.modifiers,
            position_in_widget_x: drop.position_in_widget_x,
            position_in_widget_y: drop.position_in_widget_y,
            position_in_screen_x: drop.position_in_screen_x,
            position_in_screen_y: drop.position_in_screen_y,
        };

        bridge_ok(
            "send_drag_drop",
            bridge_call!(cbf_bridge_client_send_drag_drop(self.inner, &ffi_drop)),
        )
    }

    /// Cancel a host-owned drag session.
    pub fn send_drag_cancel(
        &mut self,
        session_id: u64,
        browsing_context_id: TabId,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "send_drag_cancel",
            bridge_call!(cbf_bridge_client_send_drag_cancel(
                self.inner,
                session_id,
                browsing_context_id.get(),
            )),
        )
    }

    /// Send an external drag enter event for a native drag destination.
    pub fn send_external_drag_enter(
        &mut self,
        event: &ChromeExternalDragEnter,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let mut owned_data = OwnedDragData::new(&event.data)?;
        let ffi_event = CbfExternalDragEnter {
            tab_id: event.browsing_context_id.get(),
            data: owned_data.as_ffi(),
            allowed_operations: event.allowed_operations.bits(),
            modifiers: event.modifiers,
            position_in_widget_x: event.position_in_widget_x,
            position_in_widget_y: event.position_in_widget_y,
            position_in_screen_x: event.position_in_screen_x,
            position_in_screen_y: event.position_in_screen_y,
        };

        bridge_ok(
            "send_external_drag_enter",
            bridge_call!(cbf_bridge_client_send_external_drag_enter(
                self.inner, &ffi_event
            )),
        )
    }

    /// Send an external drag update event for a native drag destination.
    pub fn send_external_drag_update(
        &mut self,
        event: &ChromeExternalDragUpdate,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let ffi_event = CbfExternalDragUpdate {
            tab_id: event.browsing_context_id.get(),
            allowed_operations: event.allowed_operations.bits(),
            modifiers: event.modifiers,
            position_in_widget_x: event.position_in_widget_x,
            position_in_widget_y: event.position_in_widget_y,
            position_in_screen_x: event.position_in_screen_x,
            position_in_screen_y: event.position_in_screen_y,
        };

        bridge_ok(
            "send_external_drag_update",
            bridge_call!(cbf_bridge_client_send_external_drag_update(
                self.inner, &ffi_event
            )),
        )
    }

    /// Notify the backend that the active external drag left the page.
    pub fn send_external_drag_leave(
        &mut self,
        browsing_context_id: TabId,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "send_external_drag_leave",
            bridge_call!(cbf_bridge_client_send_external_drag_leave(
                self.inner,
                browsing_context_id.get()
            )),
        )
    }

    /// Send an external drag drop event for a native drag destination.
    pub fn send_external_drag_drop(
        &mut self,
        event: &ChromeExternalDragDrop,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let ffi_event = CbfExternalDragDrop {
            tab_id: event.browsing_context_id.get(),
            modifiers: event.modifiers,
            position_in_widget_x: event.position_in_widget_x,
            position_in_widget_y: event.position_in_widget_y,
            position_in_screen_x: event.position_in_screen_x,
            position_in_screen_y: event.position_in_screen_y,
        };

        bridge_ok(
            "send_external_drag_drop",
            bridge_call!(cbf_bridge_client_send_external_drag_drop(
                self.inner, &ffi_event
            )),
        )
    }

    /// Update the IME composition state.
    pub fn set_composition(
        &mut self,
        composition: &ChromeImeComposition,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let text =
            CString::new(composition.text.as_str()).map_err(|_| BridgeError::InvalidInput)?;
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

        bridge_ok(
            "set_composition",
            bridge_call!(cbf_bridge_client_set_composition(
                self.inner,
                &ffi_composition
            )),
        )
    }

    /// Update the IME composition state for an extension popup.
    pub fn set_extension_popup_composition(
        &mut self,
        composition: &ChromeTransientImeComposition,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let text =
            CString::new(composition.text.as_str()).map_err(|_| BridgeError::InvalidInput)?;
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

        bridge_ok(
            "set_extension_popup_composition",
            bridge_call!(cbf_bridge_client_set_extension_popup_composition(
                self.inner,
                composition.popup_id.get(),
                &ffi_composition,
            )),
        )
    }

    /// Commit IME text input to the page.
    pub fn commit_text(&mut self, commit: &ChromeImeCommitText) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let text = CString::new(commit.text.as_str()).map_err(|_| BridgeError::InvalidInput)?;
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

        bridge_ok(
            "commit_text",
            bridge_call!(cbf_bridge_client_commit_text(self.inner, &ffi_commit)),
        )
    }

    /// Commit IME text input to an extension popup.
    pub fn commit_extension_popup_text(
        &mut self,
        commit: &ChromeTransientImeCommitText,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let text = CString::new(commit.text.as_str()).map_err(|_| BridgeError::InvalidInput)?;
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

        bridge_ok(
            "commit_extension_popup_text",
            bridge_call!(cbf_bridge_client_commit_extension_popup_text(
                self.inner,
                commit.popup_id.get(),
                &ffi_commit,
            )),
        )
    }

    /// Finish composing IME text with the specified behavior.
    pub fn finish_composing_text(
        &mut self,
        browsing_context_id: TabId,
        behavior: ChromeConfirmCompositionBehavior,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let behavior = match behavior {
            ChromeConfirmCompositionBehavior::DoNotKeepSelection => {
                CbfConfirmCompositionBehavior_kCbfConfirmCompositionDoNotKeepSelection
            }
            ChromeConfirmCompositionBehavior::KeepSelection => {
                CbfConfirmCompositionBehavior_kCbfConfirmCompositionKeepSelection
            }
        } as u8;

        bridge_ok(
            "finish_composing_text",
            bridge_call!(cbf_bridge_client_finish_composing_text(
                self.inner,
                browsing_context_id.get(),
                behavior,
            )),
        )
    }

    /// Finish composing IME text inside an extension popup.
    pub fn finish_extension_popup_composing_text(
        &mut self,
        popup_id: PopupId,
        behavior: ChromeConfirmCompositionBehavior,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let behavior = match behavior {
            ChromeConfirmCompositionBehavior::DoNotKeepSelection => {
                CbfConfirmCompositionBehavior_kCbfConfirmCompositionDoNotKeepSelection
            }
            ChromeConfirmCompositionBehavior::KeepSelection => {
                CbfConfirmCompositionBehavior_kCbfConfirmCompositionKeepSelection
            }
        } as u8;

        bridge_ok(
            "finish_extension_popup_composing_text",
            bridge_call!(cbf_bridge_client_finish_extension_popup_composing_text(
                self.inner,
                popup_id.get(),
                behavior,
            )),
        )
    }

    pub fn set_extension_popup_focus(
        &mut self,
        popup_id: PopupId,
        focused: bool,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "set_extension_popup_focus",
            bridge_call!(cbf_bridge_client_set_extension_popup_focus(
                self.inner,
                popup_id.get(),
                focused
            )),
        )
    }

    pub fn set_extension_popup_size(
        &mut self,
        popup_id: PopupId,
        width: u32,
        height: u32,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "set_extension_popup_size",
            bridge_call!(cbf_bridge_client_set_extension_popup_size(
                self.inner,
                popup_id.get(),
                width,
                height,
            )),
        )
    }

    /// Update the page background policy of the specified extension popup.
    pub fn set_extension_popup_background_policy(
        &mut self,
        popup_id: PopupId,
        policy: ChromeBackgroundPolicy,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let transparent = matches!(policy, ChromeBackgroundPolicy::Transparent);

        bridge_ok(
            "set_extension_popup_background_policy",
            bridge_call!(cbf_bridge_client_set_extension_popup_background_policy(
                self.inner,
                popup_id.get(),
                transparent,
            )),
        )
    }

    pub fn close_extension_popup(&mut self, popup_id: PopupId) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "close_extension_popup",
            bridge_call!(cbf_bridge_client_close_extension_popup(
                self.inner,
                popup_id.get()
            )),
        )
    }

    /// Execute a browser-generic edit action for the given page.
    pub fn execute_edit_action(
        &mut self,
        browsing_context_id: TabId,
        action: EditAction,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "execute_edit_action",
            bridge_call!(cbf_bridge_client_execute_edit_action(
                self.inner,
                browsing_context_id.get(),
                edit_action_to_ffi(action),
            )),
        )
    }

    /// Execute a browser-generic edit action for the given extension popup.
    pub fn execute_extension_popup_edit_action(
        &mut self,
        popup_id: PopupId,
        action: EditAction,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "execute_extension_popup_edit_action",
            bridge_call!(cbf_bridge_client_execute_extension_popup_edit_action(
                self.inner,
                popup_id.get(),
                edit_action_to_ffi(action),
            )),
        )
    }

    /// Execute a context menu command for the given menu.
    pub fn execute_context_menu_command(
        &mut self,
        menu_id: u64,
        command_id: i32,
        event_flags: i32,
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "execute_context_menu_command",
            bridge_call!(cbf_bridge_client_execute_context_menu_command(
                self.inner,
                menu_id,
                command_id,
                event_flags,
            )),
        )
    }

    /// Accept a host-owned choice menu selection.
    pub fn accept_choice_menu_selection(
        &mut self,
        request_id: u64,
        indices: &[i32],
    ) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        let ffi_indices = CbfChoiceMenuSelectedIndices {
            items: indices.as_ptr(),
            len: indices.len() as u32,
        };
        bridge_ok(
            "accept_choice_menu_selection",
            bridge_call!(cbf_bridge_client_accept_choice_menu_selection(
                self.inner,
                request_id,
                &ffi_indices
            )),
        )
    }

    /// Dismiss a host-owned choice menu without a selection.
    pub fn dismiss_choice_menu(&mut self, request_id: u64) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "dismiss_choice_menu",
            bridge_call!(cbf_bridge_client_dismiss_choice_menu(
                self.inner, request_id
            )),
        )
    }

    /// Dismiss the context menu with the given id.
    pub fn dismiss_context_menu(&mut self, menu_id: u64) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "dismiss_context_menu",
            bridge_call!(cbf_bridge_client_dismiss_context_menu(self.inner, menu_id)),
        )
    }

    /// Request a graceful shutdown from the backend.
    pub fn request_shutdown(&mut self, request_id: u64) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "request_shutdown",
            bridge_call!(cbf_bridge_client_request_shutdown(self.inner, request_id)),
        )
    }

    /// Respond to a shutdown confirmation request.
    pub fn confirm_shutdown(&mut self, request_id: u64, proceed: bool) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "confirm_shutdown",
            bridge_call!(cbf_bridge_client_confirm_shutdown(
                self.inner, request_id, proceed
            )),
        )
    }

    /// Force an immediate shutdown without confirmations.
    pub fn force_shutdown(&mut self) -> Result<(), BridgeError> {
        self.ensure_ready()?;

        bridge_ok(
            "force_shutdown",
            bridge_call!(cbf_bridge_client_force_shutdown(self.inner)),
        )
    }

    /// Tear down the IPC client and free native resources.
    pub fn shutdown(&mut self) {
        if !self.inner.is_null() {
            cleanup_bridge_call("shutdown bridge client", |bridge| {
                unsafe { bridge.cbf_bridge_client_shutdown(self.inner) };
            });
        }
    }
}

impl IpcEventWaitHandle {
    pub(crate) fn wait_for_event(
        &self,
        timeout: Option<Duration>,
    ) -> Result<EventWaitResult, BridgeError> {
        wait_for_event_inner(self.inner, timeout)
    }
}

fn wait_for_event_inner(
    inner: *mut CbfBridgeClientHandle,
    timeout: Option<Duration>,
) -> Result<EventWaitResult, BridgeError> {
    if inner.is_null() {
        return Ok(EventWaitResult::Closed);
    }

    let timeout_ms = timeout
        .map(|value| value.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(-1);
    let status = bridge_call!(cbf_bridge_client_wait_for_event(inner, timeout_ms));

    match status {
        CbfBridgeEventWaitStatus_kCbfBridgeEventWaitStatusEventAvailable => {
            Ok(EventWaitResult::EventAvailable)
        }
        CbfBridgeEventWaitStatus_kCbfBridgeEventWaitStatusTimedOut => Ok(EventWaitResult::TimedOut),
        CbfBridgeEventWaitStatus_kCbfBridgeEventWaitStatusDisconnected => {
            Ok(EventWaitResult::Disconnected)
        }
        CbfBridgeEventWaitStatus_kCbfBridgeEventWaitStatusClosed => Ok(EventWaitResult::Closed),
        _ => Err(BridgeError::InvalidEvent),
    }
}

fn edit_action_to_ffi(action: EditAction) -> u8 {
    (match action {
        EditAction::Undo => CbfEditAction_kCbfEditActionUndo,
        EditAction::Redo => CbfEditAction_kCbfEditActionRedo,
        EditAction::Cut => CbfEditAction_kCbfEditActionCut,
        EditAction::Copy => CbfEditAction_kCbfEditActionCopy,
        EditAction::Paste => CbfEditAction_kCbfEditActionPaste,
        EditAction::SelectAll => CbfEditAction_kCbfEditActionSelectAll,
    }) as u8
}

fn as_ffi_string_ptr(value: &CString) -> *mut c_char {
    value.as_ptr().cast_mut()
}

struct OwnedStringList {
    strings: Vec<CString>,
    ptrs: Vec<*mut c_char>,
}

impl OwnedStringList {
    fn new(values: &[String]) -> Result<Self, BridgeError> {
        let strings = values
            .iter()
            .map(|value| CString::new(value.as_str()).map_err(|_| BridgeError::InvalidInput))
            .collect::<Result<Vec<_>, _>>()?;
        let ptrs = strings.iter().map(as_ffi_string_ptr).collect::<Vec<_>>();
        Ok(Self { strings, ptrs })
    }

    fn as_ffi(&mut self) -> CbfStringList {
        let _ = &self.strings;
        CbfStringList {
            items: if self.ptrs.is_empty() {
                ptr::null_mut()
            } else {
                self.ptrs.as_mut_ptr()
            },
            len: self.ptrs.len() as u32,
        }
    }
}

struct OwnedDragUrlList {
    strings: Vec<(CString, CString)>,
    items: Vec<CbfDragUrlInfo>,
}

impl OwnedDragUrlList {
    fn new(values: &[crate::data::drag::ChromeDragUrlInfo]) -> Result<Self, BridgeError> {
        let strings = values
            .iter()
            .map(|value| {
                Ok((
                    CString::new(value.url.as_str()).map_err(|_| BridgeError::InvalidInput)?,
                    CString::new(value.title.as_str()).map_err(|_| BridgeError::InvalidInput)?,
                ))
            })
            .collect::<Result<Vec<_>, BridgeError>>()?;
        let items = strings
            .iter()
            .map(|(url, title)| CbfDragUrlInfo {
                url: as_ffi_string_ptr(url),
                title: as_ffi_string_ptr(title),
            })
            .collect();
        Ok(Self { strings, items })
    }

    fn as_ffi(&self) -> CbfDragUrlInfoList {
        let _ = &self.strings;
        CbfDragUrlInfoList {
            items: if self.items.is_empty() {
                ptr::null()
            } else {
                self.items.as_ptr()
            },
            len: self.items.len() as u32,
        }
    }
}

struct OwnedStringPairList {
    strings: Vec<(CString, CString)>,
    items: Vec<CbfStringPair>,
}

impl OwnedStringPairList {
    fn new(values: &std::collections::BTreeMap<String, String>) -> Result<Self, BridgeError> {
        let strings = values
            .iter()
            .map(|(key, value)| {
                Ok((
                    CString::new(key.as_str()).map_err(|_| BridgeError::InvalidInput)?,
                    CString::new(value.as_str()).map_err(|_| BridgeError::InvalidInput)?,
                ))
            })
            .collect::<Result<Vec<_>, BridgeError>>()?;
        let items = strings
            .iter()
            .map(|(key, value)| CbfStringPair {
                key: as_ffi_string_ptr(key),
                value: as_ffi_string_ptr(value),
            })
            .collect();
        Ok(Self { strings, items })
    }

    fn as_ffi(&mut self) -> CbfStringPairList {
        let _ = &self.strings;
        CbfStringPairList {
            items: if self.items.is_empty() {
                ptr::null_mut()
            } else {
                self.items.as_mut_ptr()
            },
            len: self.items.len() as u32,
        }
    }
}

struct OwnedDragData {
    text: CString,
    html: CString,
    html_base_url: CString,
    url_infos: OwnedDragUrlList,
    filenames: OwnedStringList,
    file_mime_types: OwnedStringList,
    custom_data: OwnedStringPairList,
}

impl OwnedDragData {
    fn new(data: &ChromeDragData) -> Result<Self, BridgeError> {
        Ok(Self {
            text: CString::new(data.text.as_str()).map_err(|_| BridgeError::InvalidInput)?,
            html: CString::new(data.html.as_str()).map_err(|_| BridgeError::InvalidInput)?,
            html_base_url: CString::new(data.html_base_url.as_str())
                .map_err(|_| BridgeError::InvalidInput)?,
            url_infos: OwnedDragUrlList::new(&data.url_infos)?,
            filenames: OwnedStringList::new(&data.filenames)?,
            file_mime_types: OwnedStringList::new(&data.file_mime_types)?,
            custom_data: OwnedStringPairList::new(&data.custom_data)?,
        })
    }

    fn as_ffi(&mut self) -> CbfDragData {
        CbfDragData {
            text: as_ffi_string_ptr(&self.text),
            html: as_ffi_string_ptr(&self.html),
            html_base_url: as_ffi_string_ptr(&self.html_base_url),
            url_infos: self.url_infos.as_ffi(),
            filenames: self.filenames.as_ffi(),
            file_mime_types: self.file_mime_types.as_ffi(),
            custom_data: self.custom_data.as_ffi(),
        }
    }
}

fn ipc_message_type_to_ffi(message_type: TabIpcMessageType) -> u8 {
    (match message_type {
        TabIpcMessageType::Request => CbfIpcMessageType_kCbfIpcMessageRequest,
        TabIpcMessageType::Response => CbfIpcMessageType_kCbfIpcMessageResponse,
        TabIpcMessageType::Event => CbfIpcMessageType_kCbfIpcMessageEvent,
    }) as u8
}

fn ipc_error_code_to_ffi(error_code: TabIpcErrorCode) -> u8 {
    (match error_code {
        TabIpcErrorCode::Timeout => CbfIpcErrorCode_kCbfIpcErrorTimeout,
        TabIpcErrorCode::Aborted => CbfIpcErrorCode_kCbfIpcErrorAborted,
        TabIpcErrorCode::Disconnected => CbfIpcErrorCode_kCbfIpcErrorDisconnected,
        TabIpcErrorCode::IpcDisabled => CbfIpcErrorCode_kCbfIpcErrorIpcDisabled,
        TabIpcErrorCode::ContextClosed => CbfIpcErrorCode_kCbfIpcErrorContextClosed,
        TabIpcErrorCode::RemoteError => CbfIpcErrorCode_kCbfIpcErrorRemoteError,
        TabIpcErrorCode::ProtocolError => CbfIpcErrorCode_kCbfIpcErrorProtocolError,
    }) as u8
}

impl Drop for IpcClient {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            cleanup_bridge_call("destroy bridge client on drop", |bridge| {
                unsafe { bridge.cbf_bridge_client_destroy(self.inner) };
            });
            self.inner = ptr::null_mut();
        }
    }
}

impl IpcClient {
    fn ensure_ready(&self) -> Result<(), BridgeError> {
        if self.inner.is_null() {
            Err(BridgeError::InvalidState)
        } else {
            Ok(())
        }
    }
}

fn bridge_api() -> Result<&'static BridgeLibrary, BridgeError> {
    bridge().map_err(map_bridge_load_error)
}

fn map_bridge_load_error(_: BridgeLoadError) -> BridgeError {
    BridgeError::BridgeLoadFailed
}

fn parse_channel_switch_arg(buf: &[u8]) -> Result<String, BridgeError> {
    let switch_arg = CStr::from_bytes_until_nul(buf)
        .map_err(|_| BridgeError::InvalidChannelArgument)?
        .to_str()
        .map_err(|_| BridgeError::InvalidChannelArgument)?
        .to_owned();
    if switch_arg.is_empty() {
        return Err(BridgeError::InvalidChannelArgument);
    }

    Ok(switch_arg)
}

fn authentication_result(success: bool) -> Result<(), BridgeError> {
    if success {
        Ok(())
    } else {
        Err(BridgeError::AuthenticationFailed)
    }
}

fn bridge_ok(operation: &'static str, success: bool) -> Result<(), BridgeError> {
    if success {
        Ok(())
    } else {
        Err(BridgeError::OperationFailed { operation })
    }
}

fn cleanup_bridge_call<F>(operation: &'static str, callback: F)
where
    F: FnOnce(&BridgeLibrary),
{
    if let Err(error) = bridge().map(callback) {
        warn!(operation, error = ?error, "bridge cleanup call failed");
    }
}

#[cfg(test)]
mod tests {
    use std::mem::MaybeUninit;

    use super::{
        BridgeError, IpcClient, authentication_result, bridge_ok, parse_channel_switch_arg,
    };

    fn null_ipc_client() -> IpcClient {
        // SAFETY: `IpcClient` is a raw pointer wrapper; a zeroed value is sufficient
        // for testing the null-handle guard.
        unsafe { MaybeUninit::zeroed().assume_init() }
    }

    #[test]
    fn parse_channel_switch_arg_rejects_missing_nul() {
        assert_eq!(
            parse_channel_switch_arg(b"--cbf-ipc-handle=abc"),
            Err(BridgeError::InvalidChannelArgument)
        );
    }

    #[test]
    fn parse_channel_switch_arg_rejects_invalid_utf8() {
        assert_eq!(
            parse_channel_switch_arg(b"--cbf-ipc-handle=\xFF\0"),
            Err(BridgeError::InvalidChannelArgument)
        );
    }

    #[test]
    fn parse_channel_switch_arg_rejects_empty_string() {
        assert_eq!(
            parse_channel_switch_arg(b"\0"),
            Err(BridgeError::InvalidChannelArgument)
        );
    }

    #[test]
    fn null_client_handle_reports_invalid_state() {
        let client = null_ipc_client();

        assert_eq!(client.ensure_ready(), Err(BridgeError::InvalidState));
    }

    #[test]
    fn authentication_result_reports_authentication_failure() {
        assert_eq!(
            authentication_result(false),
            Err(BridgeError::AuthenticationFailed)
        );
    }

    #[test]
    fn bridge_ok_reports_operation_name() {
        assert_eq!(
            bridge_ok("navigate", false),
            Err(BridgeError::OperationFailed {
                operation: "navigate",
            })
        );
    }
}
