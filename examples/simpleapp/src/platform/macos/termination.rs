// Winit owns and depends on its own NSApplicationDelegate on macOS, and
// replacing that delegate is not viable here. This module therefore injects
// applicationShouldTerminate: into the existing delegate class at runtime and
// bridges the terminate decision back into SimpleApp.

use std::{
    ffi::CString,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};

use objc2::{
    encode::{Encode, RefEncode},
    ffi,
    runtime::{AnyClass, AnyObject, Imp, ProtocolObject, Sel},
    sel,
};
use objc2_app_kit::{NSApplication, NSApplicationTerminateReply};
use objc2_foundation::MainThreadMarker;
use tracing::{debug, warn};
use winit::event_loop::EventLoopProxy;

use crate::app::events::UserEvent;

static TERMINATION_STATE: OnceLock<Mutex<TerminationState>> = OnceLock::new();
static NEXT_TERMINATION_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Default)]
struct TerminationState {
    proxy: Option<EventLoopProxy<UserEvent>>,
    pending_sequence: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct AppTerminationBridge;

impl AppTerminationBridge {
    pub(crate) fn install(proxy: EventLoopProxy<UserEvent>) -> Result<Self, String> {
        let state = TERMINATION_STATE.get_or_init(|| Mutex::new(TerminationState::default()));
        {
            let mut guard = state.lock().expect("termination state lock poisoned");
            guard.proxy = Some(proxy);
        }

        let mtm = MainThreadMarker::new().ok_or_else(|| {
            "app termination bridge must be installed on the main thread".to_owned()
        })?;
        let app = NSApplication::sharedApplication(mtm);
        let delegate = app
            .delegate()
            .ok_or_else(|| "winit did not configure an NSApplication delegate".to_owned())?;
        let delegate_obj: &ProtocolObject<dyn objc2_app_kit::NSApplicationDelegate> =
            delegate.as_ref();
        let delegate_class = AsRef::<AnyObject>::as_ref(delegate_obj).class();
        let selector = sel!(applicationShouldTerminate:);

        if delegate_class.instance_method(selector).is_none() {
            let types = method_type_encoding(
                &NSApplicationTerminateReply::ENCODING,
                &[<NSApplication as RefEncode>::ENCODING_REF],
            );
            let imp = unsafe {
                std::mem::transmute::<
                    unsafe extern "C-unwind" fn(
                        &AnyObject,
                        Sel,
                        &NSApplication,
                    ) -> NSApplicationTerminateReply,
                    Imp,
                >(termination_hook)
            };
            let success = unsafe {
                ffi::class_addMethod(
                    delegate_class as *const AnyClass as *mut AnyClass,
                    selector,
                    imp,
                    types.as_ptr(),
                )
            };
            if !success.as_bool() {
                return Err(format!(
                    "failed to add applicationShouldTerminate: to {}",
                    delegate_class.name().to_string_lossy()
                ));
            }
        }

        Ok(Self)
    }

    pub(crate) fn reply(&self, sequence: u64, should_terminate: bool) {
        let state = TERMINATION_STATE.get_or_init(|| Mutex::new(TerminationState::default()));
        let mut guard = state.lock().expect("termination state lock poisoned");
        if guard.pending_sequence != Some(sequence) {
            debug!(
                pending_sequence = ?guard.pending_sequence,
                sequence,
                "ignored stale application termination reply",
            );
            return;
        }

        guard.pending_sequence = None;
        drop(guard);

        let Some(mtm) = MainThreadMarker::new() else {
            warn!(
                "failed to reply to application termination because main thread marker was unavailable"
            );
            return;
        };
        let app = NSApplication::sharedApplication(mtm);
        app.replyToApplicationShouldTerminate(should_terminate);
    }
}

unsafe extern "C-unwind" fn termination_hook(
    _this: &AnyObject,
    _cmd: Sel,
    _sender: &NSApplication,
) -> NSApplicationTerminateReply {
    let state = TERMINATION_STATE.get_or_init(|| Mutex::new(TerminationState::default()));
    let mut guard = state.lock().expect("termination state lock poisoned");

    if guard.pending_sequence.is_some() {
        return NSApplicationTerminateReply::TerminateLater;
    }

    let sequence = NEXT_TERMINATION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let Some(proxy) = guard.proxy.clone() else {
        return NSApplicationTerminateReply::TerminateCancel;
    };

    guard.pending_sequence = Some(sequence);
    drop(guard);

    if proxy
        .send_event(UserEvent::AppTerminationRequested { sequence })
        .is_err()
    {
        let mut guard = state.lock().expect("termination state lock poisoned");
        if guard.pending_sequence == Some(sequence) {
            guard.pending_sequence = None;
        }
        return NSApplicationTerminateReply::TerminateCancel;
    }

    NSApplicationTerminateReply::TerminateLater
}

fn method_type_encoding(
    ret: &objc2::encode::Encoding,
    args: &[objc2::encode::Encoding],
) -> CString {
    let mut types = format!("{ret}{}{}", <*mut AnyObject>::ENCODING, Sel::ENCODING);
    for enc in args {
        use std::fmt::Write;
        write!(&mut types, "{enc}").expect("writing method type encoding");
    }
    CString::new(types).expect("objective-c method type encoding should not contain interior NULs")
}
