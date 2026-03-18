use std::thread;

use cbf::{
    browser::{EventStream, RawOpaqueEventExt},
    event::{BackendStopReason, BrowserEvent},
};
use cbf_chrome::backend::ChromiumBackend;
use tracing::{debug, warn};
use winit::event_loop::EventLoopProxy;

use crate::app::events::UserEvent;

pub(crate) fn spawn_browser_event_forwarder(
    events: EventStream<ChromiumBackend>,
    proxy: EventLoopProxy<UserEvent>,
) {
    thread::spawn(move || {
        let mut shutdown_observed = false;
        loop {
            match events.recv_blocking() {
                Ok(event) => {
                    if let Some(generic_event) = event.as_generic().cloned() {
                        if matches!(
                            generic_event,
                            BrowserEvent::ShutdownProceeding { .. }
                                | BrowserEvent::BackendStopped {
                                    reason: BackendStopReason::ShutdownRequested,
                                }
                        ) {
                            shutdown_observed = true;
                        }

                        if proxy.send_event(UserEvent::Browser(generic_event)).is_err() {
                            return;
                        }
                    }

                    if proxy
                        .send_event(UserEvent::Chrome(event.as_raw().clone()))
                        .is_err()
                    {
                        return;
                    }
                }
                Err(err) => {
                    if shutdown_observed {
                        debug!("browser event stream closed during shutdown: {err}");
                    } else {
                        warn!("browser event stream closed: {err}");
                    }
                    return;
                }
            }
        }
    });
}
