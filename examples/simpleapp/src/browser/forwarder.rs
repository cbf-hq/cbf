use std::thread;

use cbf::browser::{EventStream, RawOpaqueEventExt};
use cbf_chrome::{
    backend::ChromiumBackend,
    process::{ChromiumRuntimeShutdownState, ChromiumRuntimeShutdownStateReader},
};
use tracing::{debug, warn};
use winit::event_loop::EventLoopProxy;

use crate::app::events::UserEvent;

pub(crate) fn spawn_browser_event_forwarder(
    events: EventStream<ChromiumBackend>,
    shutdown_state: ChromiumRuntimeShutdownStateReader,
    proxy: EventLoopProxy<UserEvent>,
) {
    thread::spawn(move || {
        loop {
            match events.recv_blocking() {
                Ok(event) => {
                    if let Some(generic_event) = event.as_generic().cloned()
                        && proxy.send_event(UserEvent::Browser(generic_event)).is_err()
                    {
                        return;
                    }

                    if proxy
                        .send_event(UserEvent::Chrome(event.as_raw().clone()))
                        .is_err()
                    {
                        return;
                    }
                }
                Err(err) => {
                    if shutdown_state.shutdown_state() != ChromiumRuntimeShutdownState::Idle {
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
