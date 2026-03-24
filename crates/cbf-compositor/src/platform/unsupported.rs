use cbf::data::{
    context_menu::ContextMenu,
    drag::{DragOperation, DragStartRequest},
};
#[cfg(feature = "chrome")]
use cbf_chrome::data::choice_menu::ChromeChoiceMenu;

use crate::{
    error::CompositorError,
    model::SurfaceTarget,
    platform::host::{PlatformInputState, PlatformSceneItem, PlatformWindowHost},
};

#[allow(dead_code)]
#[derive(Debug, Default)]
pub(crate) struct UnsupportedPlatformWindowHost;

impl PlatformWindowHost for UnsupportedPlatformWindowHost {
    fn sync_scene(&mut self, _items: &[PlatformSceneItem]) -> Result<(), CompositorError> {
        Ok(())
    }

    fn show_context_menu(
        &mut self,
        _target: SurfaceTarget,
        _menu: ContextMenu,
    ) -> Result<(), CompositorError> {
        Err(CompositorError::PlatformUnsupported)
    }

    #[cfg(feature = "chrome")]
    fn show_choice_menu(
        &mut self,
        _target: SurfaceTarget,
        _menu: ChromeChoiceMenu,
    ) -> Result<(), CompositorError> {
        Err(CompositorError::PlatformUnsupported)
    }

    fn start_native_drag(
        &mut self,
        _target: SurfaceTarget,
        _request: DragStartRequest,
    ) -> Result<bool, CompositorError> {
        Err(CompositorError::PlatformUnsupported)
    }

    fn set_external_drag_operation(
        &mut self,
        _target: SurfaceTarget,
        _operation: DragOperation,
    ) -> Result<(), CompositorError> {
        Ok(())
    }

    fn input_state(&self) -> PlatformInputState {
        PlatformInputState::default()
    }
}

#[allow(dead_code)]
pub(crate) fn attach_unsupported_window_host()
-> Result<Box<dyn PlatformWindowHost>, CompositorError> {
    Err(CompositorError::PlatformUnsupported)
}
