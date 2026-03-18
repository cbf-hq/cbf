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

use crate::data::dialog::{DialogResponse, JavaScriptDialogRequest};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

#[cfg(all(feature = "native-dialogs", target_os = "macos"))]
mod macos;

#[cfg(all(feature = "native-dialogs", target_os = "macos"))]
pub use macos::{show_alert, show_confirm, show_prompt};

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
    macos::present_javascript_dialog(request, context)
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
