use cbf_chrome::event::{ChromeEvent, map_ipc_event_to_generic, to_generic_event};

use crate::{core::Compositor, error::CompositorError};

use super::{surface_adapter, transient_adapter};

pub(crate) fn apply_chrome_event(
    compositor: &mut Compositor,
    event: &ChromeEvent,
) -> Result<(), CompositorError> {
    match event {
        ChromeEvent::Ipc(raw) => {
            if let Some(generic_event) = map_ipc_event_to_generic(raw) {
                compositor.update_browser_event(&generic_event, |_| {})?;
            }
            surface_adapter::apply_surface_event(compositor, raw.as_ref())?;
            transient_adapter::apply_transient_event(compositor, raw.as_ref())
        }
        _ => {
            if let Some(generic_event) = to_generic_event(event) {
                compositor.update_browser_event(&generic_event, |_| {})?;
            }
            Ok(())
        }
    }
}
