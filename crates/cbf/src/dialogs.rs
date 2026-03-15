//! JavaScript dialog presentation abstractions and optional native helpers.
//!
//! `DialogPresenter` lets applications decouple dialog UI from event handling,
//! so hosts can spawn dialog work on their own runtime and route the resulting
//! [`DialogResponse`](crate::data::dialog::DialogResponse) back through
//! `BrowserHandle`.
//!
//! Minimal manual wiring example:
//!
//! ```no_run
//! use cbf::{
//!     data::dialog::{DialogResponse, DialogType, JavaScriptDialogRequest},
//!     dialogs::{DialogPresentationContext, DialogPresenter, DialogResponseFuture},
//! };
//!
//! #[derive(Clone, Default)]
//! struct Browser;
//!
//! impl Browser {
//!     fn respond_javascript_dialog(&self, _request_id: u64, _response: DialogResponse) {}
//! }
//!
//! struct ImmediatePresenter;
//!
//! impl DialogPresenter for ImmediatePresenter {
//!     fn present_javascript_dialog(
//!         &self,
//!         _request: JavaScriptDialogRequest,
//!         _context: DialogPresentationContext,
//!     ) -> DialogResponseFuture {
//!         Box::pin(async { DialogResponse::Success { input: None } })
//!     }
//! }
//!
//! fn handle_dialog_event(
//!     presenter: ImmediatePresenter,
//!     browser: Browser,
//!     request_id: u64,
//!     request: JavaScriptDialogRequest,
//! ) {
//!     std::thread::spawn(move || {
//!         let response = futures_lite::future::block_on(
//!             presenter.present_javascript_dialog(request, DialogPresentationContext::default()),
//!         );
//!         browser.respond_javascript_dialog(request_id, response);
//!     });
//! }
//!
//! handle_dialog_event(
//!     ImmediatePresenter,
//!     Browser,
//!     7,
//!     JavaScriptDialogRequest::new(DialogType::Alert, "Hello", None),
//! );
//! ```

use std::{future::Future, pin::Pin};

#[cfg(all(feature = "native-dialogs", target_os = "macos"))]
use std::sync::Mutex;

use crate::data::dialog::{DialogResponse, JavaScriptDialogRequest};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

#[cfg(all(feature = "native-dialogs", target_os = "macos"))]
use crate::data::dialog::DialogType;

#[cfg(all(feature = "native-dialogs", target_os = "macos"))]
use block2::RcBlock;

#[cfg(all(feature = "native-dialogs", target_os = "macos"))]
use objc2::{MainThreadMarker, rc::Retained};

#[cfg(all(feature = "native-dialogs", target_os = "macos"))]
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSAlertStyle, NSModalResponse, NSTextField, NSView,
};

#[cfg(all(feature = "native-dialogs", target_os = "macos"))]
use objc2_core_foundation::{CGPoint, CGRect, CGSize};

#[cfg(all(feature = "native-dialogs", target_os = "macos"))]
use objc2_foundation::NSString;

/// Boxed future returned by [`DialogPresenter`].
pub type DialogResponseFuture = Pin<Box<dyn Future<Output = DialogResponse> + Send + 'static>>;

/// Presentation-only context for native dialog placement.
#[derive(Debug, Clone, Copy, Default)]
pub struct DialogPresentationContext {
    pub parent_window_handle: Option<RawWindowHandle>,
    pub parent_display_handle: Option<RawDisplayHandle>,
}

impl DialogPresentationContext {
    /// Creates an empty context with no parent window or display handle.
    pub const fn new() -> Self {
        Self {
            parent_window_handle: None,
            parent_display_handle: None,
        }
    }

    /// Returns a copy of this context with the given parent window handle.
    pub const fn with_parent_window_handle(mut self, handle: RawWindowHandle) -> Self {
        self.parent_window_handle = Some(handle);
        self
    }

    /// Returns a copy of this context with the given parent display handle.
    pub const fn with_parent_display_handle(mut self, handle: RawDisplayHandle) -> Self {
        self.parent_display_handle = Some(handle);
        self
    }
}

/// Application-provided presenter for JavaScript dialogs.
pub trait DialogPresenter: Send + Sync + 'static {
    /// Presents a JavaScript dialog request and resolves to the chosen response.
    fn present_javascript_dialog(
        &self,
        request: JavaScriptDialogRequest,
        context: DialogPresentationContext,
    ) -> DialogResponseFuture;
}

/// Native dialog presenter backed by platform dialog toolkits.
#[cfg(feature = "native-dialogs")]
#[derive(Debug, Default, Clone, Copy)]
pub struct NativeDialogPresenter;

#[cfg(feature = "native-dialogs")]
impl DialogPresenter for NativeDialogPresenter {
    fn present_javascript_dialog(
        &self,
        request: JavaScriptDialogRequest,
        context: DialogPresentationContext,
    ) -> DialogResponseFuture {
        native_present_javascript_dialog(request, context)
    }
}

/// Shows a native alert dialog and returns the matching JavaScript dialog response.
#[cfg(all(feature = "native-dialogs", target_os = "macos"))]
pub fn show_alert(message: &str) -> DialogResponse {
    show_blocking_alert(message)
}

/// Shows a native confirm dialog and returns the matching JavaScript dialog response.
#[cfg(all(feature = "native-dialogs", target_os = "macos"))]
pub fn show_confirm(message: &str) -> DialogResponse {
    show_blocking_confirm(message)
}

/// Shows a native prompt dialog and returns the matching JavaScript dialog response.
///
/// This helper is currently available on macOS only.
#[cfg(all(feature = "native-dialogs", target_os = "macos"))]
pub fn show_prompt(message: &str, default_prompt_text: Option<&str>) -> DialogResponse {
    show_blocking_prompt(message, default_prompt_text)
}

#[cfg(all(feature = "native-dialogs", not(target_os = "macos")))]
fn native_present_javascript_dialog(
    _request: JavaScriptDialogRequest,
    _context: DialogPresentationContext,
) -> DialogResponseFuture {
    Box::pin(async { DialogResponse::Cancel })
}

#[cfg(all(feature = "native-dialogs", target_os = "macos"))]
fn native_present_javascript_dialog(
    request: JavaScriptDialogRequest,
    context: DialogPresentationContext,
) -> DialogResponseFuture {
    let Some(parent_window) = parent_window_from_context(context) else {
        return Box::pin(async { DialogResponse::Cancel });
    };

    let Some(mtm) = MainThreadMarker::new() else {
        return Box::pin(async { DialogResponse::Cancel });
    };

    let (sender, receiver) = oneshot::channel();
    let sender = Mutex::new(Some(sender));
    let alert = configured_alert(mtm, &request);
    let prompt_input = if request.r#type == DialogType::Prompt {
        Some(configure_prompt_accessory(
            &alert,
            mtm,
            request.default_prompt_text.as_deref(),
        ))
    } else {
        None
    };

    let completion_alert = Retained::clone(&alert);
    let completion_input = prompt_input.clone();
    let response_block = RcBlock::new(move |response_code: NSModalResponse| {
        let response = response_from_modal_response(
            completion_alert.as_ref(),
            response_code,
            completion_input.as_ref().map(|input| input.as_ref()),
        );
        if let Some(sender) = sender.lock().expect("dialog sender lock poisoned").take() {
            let _ = sender.send(response);
        }
    });

    alert.beginSheetModalForWindow_completionHandler(&parent_window, Some(&response_block));

    Box::pin(async move { receiver.await.unwrap_or(DialogResponse::Cancel) })
}

#[cfg(all(feature = "native-dialogs", target_os = "macos"))]
fn show_blocking_alert(message: &str) -> DialogResponse {
    let Some(mtm) = MainThreadMarker::new() else {
        return DialogResponse::Cancel;
    };
    let alert = configured_alert(
        mtm,
        &JavaScriptDialogRequest::new(DialogType::Alert, message, None),
    );
    let response = alert.runModal();
    response_from_modal_response(alert.as_ref(), response, None)
}

#[cfg(all(feature = "native-dialogs", target_os = "macos"))]
fn show_blocking_confirm(message: &str) -> DialogResponse {
    let Some(mtm) = MainThreadMarker::new() else {
        return DialogResponse::Cancel;
    };
    let alert = configured_alert(
        mtm,
        &JavaScriptDialogRequest::new(DialogType::Confirm, message, None),
    );
    let response = alert.runModal();
    response_from_modal_response(alert.as_ref(), response, None)
}

#[cfg(all(feature = "native-dialogs", target_os = "macos"))]
fn show_blocking_prompt(message: &str, default_prompt_text: Option<&str>) -> DialogResponse {
    let Some(mtm) = MainThreadMarker::new() else {
        return DialogResponse::Cancel;
    };
    let alert = configured_alert(
        mtm,
        &JavaScriptDialogRequest::new(
            DialogType::Prompt,
            message,
            default_prompt_text.map(ToOwned::to_owned),
        ),
    );
    let input = configure_prompt_accessory(&alert, mtm, default_prompt_text);
    let response = alert.runModal();
    response_from_modal_response(alert.as_ref(), response, Some(&input))
}

#[cfg(all(feature = "native-dialogs", target_os = "macos"))]
fn configured_alert(mtm: MainThreadMarker, request: &JavaScriptDialogRequest) -> Retained<NSAlert> {
    let alert = NSAlert::new(mtm);
    let title = match request.r#type {
        DialogType::Alert => "JavaScript Alert",
        DialogType::Confirm => "JavaScript Confirm",
        DialogType::Prompt => "JavaScript Prompt",
        DialogType::BeforeUnload => "Dialog",
    };
    let ok = NSString::from_str("OK");
    let cancel = NSString::from_str("Cancel");
    alert.setMessageText(&NSString::from_str(title));
    alert.setInformativeText(&NSString::from_str(&request.message));
    alert.setAlertStyle(NSAlertStyle::Informational);
    alert.addButtonWithTitle(&ok);
    if request.r#type != DialogType::Alert {
        alert.addButtonWithTitle(&cancel);
    }
    alert
}

#[cfg(all(feature = "native-dialogs", target_os = "macos"))]
fn configure_prompt_accessory(
    alert: &NSAlert,
    mtm: MainThreadMarker,
    default_prompt_text: Option<&str>,
) -> Retained<NSTextField> {
    let initial = NSString::from_str(default_prompt_text.unwrap_or_default());
    let input = NSTextField::textFieldWithString(&initial, mtm);
    alert.layout();
    input.setFrame(CGRect::new(
        CGPoint::new(0.0, 0.0),
        CGSize::new(320.0, 24.0),
    ));
    alert.setAccessoryView(Some(input.as_ref()));
    input
}

#[cfg(all(feature = "native-dialogs", target_os = "macos"))]
fn response_from_modal_response(
    _alert: &NSAlert,
    response_code: NSModalResponse,
    prompt_input: Option<&NSTextField>,
) -> DialogResponse {
    if response_code == NSAlertFirstButtonReturn {
        DialogResponse::Success {
            input: prompt_input.map(|input| input.stringValue().to_string()),
        }
    } else {
        DialogResponse::Cancel
    }
}

#[cfg(all(feature = "native-dialogs", target_os = "macos"))]
fn parent_window_from_context(
    context: DialogPresentationContext,
) -> Option<Retained<objc2_app_kit::NSWindow>> {
    let handle = context.parent_window_handle?;
    let RawWindowHandle::AppKit(handle) = handle else {
        return None;
    };
    let ns_view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
    ns_view.window()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::dialog::DialogType;

    struct ImmediatePresenter;

    impl DialogPresenter for ImmediatePresenter {
        fn present_javascript_dialog(
            &self,
            request: JavaScriptDialogRequest,
            _context: DialogPresentationContext,
        ) -> DialogResponseFuture {
            Box::pin(async move {
                DialogResponse::Success {
                    input: request.default_prompt_text,
                }
            })
        }
    }

    #[test]
    fn custom_presenter_resolves_boxed_future() {
        let request =
            JavaScriptDialogRequest::new(DialogType::Prompt, "hello", Some("value".to_string()));

        let response = futures_lite::future::block_on(
            ImmediatePresenter
                .present_javascript_dialog(request, DialogPresentationContext::default()),
        );

        assert_eq!(
            response,
            DialogResponse::Success {
                input: Some("value".to_string())
            }
        );
    }

    #[cfg(feature = "native-dialogs")]
    #[test]
    fn presentation_context_accepts_missing_parent_handle() {
        let context = DialogPresentationContext::default();

        assert!(context.parent_window_handle.is_none());
        assert!(context.parent_display_handle.is_none());
    }

    #[cfg(feature = "native-dialogs")]
    #[test]
    fn native_presenter_creates_future_for_request() {
        let presenter = NativeDialogPresenter;
        let request = JavaScriptDialogRequest::new(DialogType::Alert, "hello", None);

        let future =
            presenter.present_javascript_dialog(request, DialogPresentationContext::default());

        drop(future);
    }
}
