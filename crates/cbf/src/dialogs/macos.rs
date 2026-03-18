use std::sync::Mutex;

use block2::RcBlock;
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSAlertStyle, NSModalResponse, NSTextField, NSView,
};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::NSString;

use super::{DialogPresentationContext, DialogResponseFuture};
use crate::data::dialog::{DialogResponse, DialogType, JavaScriptDialogRequest};

/// Shows a native alert dialog and returns the matching JavaScript dialog response.
pub fn show_alert(message: &str) -> DialogResponse {
    show_blocking_alert(message)
}

/// Shows a native confirm dialog and returns the matching JavaScript dialog response.
pub fn show_confirm(message: &str) -> DialogResponse {
    show_blocking_confirm(message)
}

/// Shows a native prompt dialog and returns the matching JavaScript dialog response.
///
/// This helper is currently available on macOS only.
pub fn show_prompt(message: &str, default_prompt_text: Option<&str>) -> DialogResponse {
    show_blocking_prompt(message, default_prompt_text)
}

pub(super) fn present_javascript_dialog(
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
            _ = sender.send(response);
        }
    });
    alert.beginSheetModalForWindow_completionHandler(&parent_window, Some(&response_block));

    Box::pin(async move { receiver.await.unwrap_or(DialogResponse::Cancel) })
}

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

fn parent_window_from_context(
    context: DialogPresentationContext,
) -> Option<Retained<objc2_app_kit::NSWindow>> {
    let handle = context.parent_window_handle?;

    let raw_window_handle::RawWindowHandle::AppKit(handle) = handle else {
        return None;
    };
    let ns_view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };

    ns_view.window()
}
