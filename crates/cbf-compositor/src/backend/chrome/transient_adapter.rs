use cbf_chrome::{data::ids::PopupId, bridge::IpcEvent};

use crate::{core::Compositor, error::CompositorError};

pub(crate) fn apply_transient_event(
    compositor: &mut Compositor,
    event: &IpcEvent,
) -> Result<(), CompositorError> {
    match event {
        IpcEvent::ExtensionPopupPreferredSizeChanged {
            popup_id,
            width,
            height,
            ..
        } => {
            compositor.set_transient_preferred_size(
                PopupId::new(*popup_id).to_transient_browsing_context_id(),
                (*width, *height),
            );
            Ok(())
        }
        _ => Ok(()),
    }
}
