use cbf::data::ime::ImeBoundsUpdate;
use cbf::data::{context_menu::ContextMenu, drag::DragStartRequest};
#[cfg(feature = "chrome")]
use cbf_chrome::data::choice_menu::ChromeChoiceMenu;

use crate::{
    error::CompositorError,
    model::{CompositionItemId, Rect, SurfaceTarget},
    window::WindowHost,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlatformSurfaceHandle {
    MacCaContextId(u32),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PlatformInputState {
    pub(crate) active_item_id: Option<CompositionItemId>,
    pub(crate) pointer_capture_item_id: Option<CompositionItemId>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlatformSceneItem {
    pub(crate) item_id: CompositionItemId,
    pub(crate) target: SurfaceTarget,
    pub(crate) bounds: Rect,
    pub(crate) visible: bool,
    pub(crate) interactive: bool,
    pub(crate) surface: Option<PlatformSurfaceHandle>,
    pub(crate) ime_bounds: Option<ImeBoundsUpdate>,
}

pub(crate) trait PlatformWindowHost {
    fn sync_scene(&mut self, items: &[PlatformSceneItem]) -> Result<(), CompositorError>;

    fn show_context_menu(
        &mut self,
        target: SurfaceTarget,
        menu: ContextMenu,
    ) -> Result<(), CompositorError>;

    #[cfg(feature = "chrome")]
    fn show_choice_menu(
        &mut self,
        target: SurfaceTarget,
        menu: ChromeChoiceMenu,
    ) -> Result<(), CompositorError>;

    fn start_native_drag(
        &mut self,
        target: SurfaceTarget,
        request: DragStartRequest,
    ) -> Result<bool, CompositorError>;

    #[allow(dead_code)]
    fn input_state(&self) -> PlatformInputState {
        PlatformInputState::default()
    }
}

pub(crate) fn attach_window_host<W, E>(
    window: &W,
    emit: E,
) -> Result<Box<dyn PlatformWindowHost>, CompositorError>
where
    W: WindowHost,
    E: FnMut(cbf::command::BrowserCommand) + 'static,
{
    #[cfg(all(target_os = "macos", feature = "chrome"))]
    {
        crate::platform::macos::attach_macos_window_host(window, emit)
    }

    #[cfg(not(all(target_os = "macos", feature = "chrome")))]
    {
        _ = window;
        _ = emit;
        crate::platform::unsupported::attach_unsupported_window_host()
    }
}
