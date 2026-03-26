use std::{cell::RefCell, rc::Rc};

use cbf::{
    command::BrowserCommand,
    data::{
        context_menu::ContextMenu,
        drag::{DragOperation, DragStartRequest},
    },
};
#[cfg(feature = "chrome")]
use cbf_chrome::data::choice_menu::ChromeChoiceMenu;
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::NSView;
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use raw_window_handle::RawWindowHandle;

use crate::{
    error::CompositorError,
    model::{CompositionItemId, SurfaceTarget},
    platform::{
        host::{PlatformInputState, PlatformSceneItem, PlatformWindowHost},
        macos::compositor_view::{CommandCallback, CompositorViewMac, SharedInputState},
    },
    window::WindowHost,
};

pub(crate) struct MacPlatformWindowHost {
    view: Retained<CompositorViewMac>,
    #[allow(dead_code)]
    input_state: SharedInputState,
    _command_callback: CommandCallback,
}

impl PlatformWindowHost for MacPlatformWindowHost {
    fn sync_scene(&mut self, items: &[PlatformSceneItem]) -> Result<(), CompositorError> {
        self.view.replace_scene(items);
        Ok(())
    }

    fn set_active_item(
        &mut self,
        item_id: Option<CompositionItemId>,
    ) -> Result<(), CompositorError> {
        self.view.set_programmatic_active_item(item_id)
    }

    fn show_context_menu(
        &mut self,
        target: SurfaceTarget,
        menu: ContextMenu,
    ) -> Result<(), CompositorError> {
        self.view.show_context_menu(target, menu)
    }

    #[cfg(feature = "chrome")]
    fn show_choice_menu(
        &mut self,
        target: SurfaceTarget,
        menu: ChromeChoiceMenu,
    ) -> Result<(), CompositorError> {
        self.view.show_choice_menu(target, menu)
    }

    fn start_native_drag(
        &mut self,
        target: SurfaceTarget,
        request: DragStartRequest,
    ) -> Result<bool, CompositorError> {
        self.view.start_native_drag_session(target, &request)
    }

    fn set_external_drag_operation(
        &mut self,
        target: SurfaceTarget,
        operation: DragOperation,
    ) -> Result<(), CompositorError> {
        self.view.set_external_drag_operation(target, operation);
        Ok(())
    }

    fn input_state(&self) -> PlatformInputState {
        *self.input_state.borrow()
    }
}

pub(crate) fn attach_macos_window_host<W, E>(
    window: &W,
    emit: E,
) -> Result<Box<dyn PlatformWindowHost>, CompositorError>
where
    W: WindowHost,
    E: FnMut(BrowserCommand) + 'static,
{
    let raw = window
        .window_handle()
        .map_err(|_| CompositorError::PlatformUnsupported)?
        .as_raw();

    let content_view = match raw {
        RawWindowHandle::AppKit(handle) => unsafe { handle.ns_view.cast::<NSView>().as_ref() },
        _ => return Err(CompositorError::PlatformUnsupported),
    };

    let mtm = MainThreadMarker::new().ok_or(CompositorError::PlatformUnsupported)?;
    let input_state: SharedInputState = Rc::new(RefCell::new(PlatformInputState::default()));
    let command_callback: CommandCallback = Rc::new(RefCell::new(Box::new(emit)));

    let scale_factor = window.scale_factor().max(1.0);
    let (width, height) = window.inner_size();
    let frame = CGRect::new(
        CGPoint::ZERO,
        CGSize::new(width as f64 / scale_factor, height as f64 / scale_factor),
    );

    let view = CompositorViewMac::attach_to_host(
        mtm,
        content_view,
        frame,
        Rc::clone(&input_state),
        Rc::clone(&command_callback),
    );

    Ok(Box::new(MacPlatformWindowHost {
        view,
        input_state,
        _command_callback: command_callback,
    }))
}
