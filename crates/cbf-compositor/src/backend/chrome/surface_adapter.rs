use cbf_chrome::{
    bridge::IpcEvent,
    data::{ids::PopupId, surface::SurfaceHandle},
};

use crate::{
    core::Compositor, error::CompositorError, model::SurfaceTarget,
    platform::host::PlatformSurfaceHandle,
};

pub(crate) fn apply_surface_event(
    compositor: &mut Compositor,
    event: &IpcEvent,
) -> Result<(), CompositorError> {
    match event {
        IpcEvent::SurfaceHandleUpdated {
            browsing_context_id,
            handle,
            ..
        } => compositor.set_surface_handle_for_target(
            SurfaceTarget::BrowsingContext(browsing_context_id.to_browsing_context_id()),
            map_surface_handle(handle),
        ),
        IpcEvent::ExtensionPopupSurfaceHandleUpdated {
            popup_id, handle, ..
        } => compositor.set_surface_handle_for_target(
            SurfaceTarget::TransientBrowsingContext(
                PopupId::new(*popup_id).to_transient_browsing_context_id(),
            ),
            map_surface_handle(handle),
        ),
        _ => Ok(()),
    }
}

fn map_surface_handle(handle: &SurfaceHandle) -> PlatformSurfaceHandle {
    match handle {
        SurfaceHandle::MacCaContextId(context_id) => {
            PlatformSurfaceHandle::MacCaContextId(*context_id)
        }
    }
}
