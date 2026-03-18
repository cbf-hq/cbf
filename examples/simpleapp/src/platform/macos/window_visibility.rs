use std::sync::{Arc, Mutex};

use block2::RcBlock;
use cbf::{
    browser::BrowserHandle,
    data::{ids::WindowId as HostWindowId, visibility::BrowsingContextVisibility},
};
use cbf_chrome::backend::ChromiumBackend;
use objc2::{
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
};
use objc2_app_kit::{
    NSView, NSWindow, NSWindowDidDeminiaturizeNotification, NSWindowDidMiniaturizeNotification,
};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSObjectProtocol};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tracing::warn;
use winit::window::Window;

use crate::app::state::{SharedStateHandle, browsing_context_ids_for_window};

type NotificationBlock = RcBlock<dyn Fn(std::ptr::NonNull<NSNotification>) + 'static>;
type ObserverToken = Retained<ProtocolObject<dyn NSObjectProtocol>>;

pub(super) struct WindowVisibilityObserver {
    center: Retained<NSNotificationCenter>,
    did_miniaturize_token: ObserverToken,
    did_deminiaturize_token: ObserverToken,
    _did_miniaturize_block: NotificationBlock,
    _did_deminiaturize_block: NotificationBlock,
}

impl WindowVisibilityObserver {
    pub(super) fn install(
        window: &Window,
        browser_handle: BrowserHandle<ChromiumBackend>,
        shared: SharedStateHandle,
        host_window_id: HostWindowId,
    ) -> Result<Self, String> {
        let ns_window = ns_window_for_winit_window(window)?;
        let center = NSNotificationCenter::defaultCenter();
        let window_object: &AnyObject = ns_window.as_ref();

        let did_miniaturize_block = RcBlock::new({
            let browser_handle = browser_handle.clone();
            let shared = Arc::clone(&shared);
            move |_notification: std::ptr::NonNull<NSNotification>| {
                sync_host_window_visibility(
                    &browser_handle,
                    &shared,
                    host_window_id,
                    BrowsingContextVisibility::Hidden,
                );
            }
        });
        let did_deminiaturize_block =
            RcBlock::new(move |_notification: std::ptr::NonNull<NSNotification>| {
                sync_host_window_visibility(
                    &browser_handle,
                    &shared,
                    host_window_id,
                    BrowsingContextVisibility::Visible,
                );
            });

        let did_miniaturize_token = unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(NSWindowDidMiniaturizeNotification),
                Some(window_object),
                None,
                &did_miniaturize_block,
            )
        };
        let did_deminiaturize_token = unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(NSWindowDidDeminiaturizeNotification),
                Some(window_object),
                None,
                &did_deminiaturize_block,
            )
        };

        Ok(Self {
            center,
            did_miniaturize_token,
            did_deminiaturize_token,
            _did_miniaturize_block: did_miniaturize_block,
            _did_deminiaturize_block: did_deminiaturize_block,
        })
    }
}

impl Drop for WindowVisibilityObserver {
    fn drop(&mut self) {
        unsafe {
            self.center
                .removeObserver(self.did_miniaturize_token.as_ref());
            self.center
                .removeObserver(self.did_deminiaturize_token.as_ref());
        }
    }
}

fn ns_window_for_winit_window(window: &Window) -> Result<Retained<NSWindow>, String> {
    let raw = window
        .window_handle()
        .map_err(|err| format!("window handle acquisition failed: {err}"))?
        .as_raw();

    let content_view = match raw {
        RawWindowHandle::AppKit(handle) => unsafe { handle.ns_view.cast::<NSView>().as_ref() },
        _ => return Err("non-AppKit window handle on macOS".to_owned()),
    };

    content_view
        .window()
        .ok_or_else(|| "AppKit content view is not attached to an NSWindow".to_owned())
}

fn sync_host_window_visibility(
    browser_handle: &BrowserHandle<ChromiumBackend>,
    shared: &Arc<Mutex<crate::app::state::SharedState>>,
    host_window_id: HostWindowId,
    visibility: BrowsingContextVisibility,
) {
    for browsing_context_id in browsing_context_ids_for_window(shared, host_window_id) {
        if let Err(err) =
            browser_handle.set_browsing_context_visibility(browsing_context_id, visibility)
        {
            warn!(
                host_window_id = %host_window_id,
                browsing_context_id = %browsing_context_id,
                ?visibility,
                "failed to sync browsing context visibility: {err}"
            );
        }
    }
}
