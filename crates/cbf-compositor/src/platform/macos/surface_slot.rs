use cbf::data::ime::ImeBoundsUpdate;
use cbf_chrome::platform::macos::bindings::CALayerHost;
use objc2::rc::Retained;
use objc2_core_foundation::CGRect;

use crate::{model::SurfaceTarget, platform::host::PlatformSurfaceHandle};

#[derive(Clone)]
pub(crate) struct SurfaceSlot {
    pub(crate) target: SurfaceTarget,
    pub(crate) layer: Retained<CALayerHost>,
    pub(crate) bounds: CGRect,
    pub(crate) z_index: i32,
    pub(crate) visible: bool,
    pub(crate) interactive: bool,
    pub(crate) surface: Option<PlatformSurfaceHandle>,
    pub(crate) ime_bounds: Option<ImeBoundsUpdate>,
}
