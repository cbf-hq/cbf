use cbf::data::ime::ImeBoundsUpdate;
use cbf_chrome::platform::macos::bindings::CALayerHost;
use objc2::rc::Retained;
use objc2_core_foundation::CGRect;

use crate::{
    model::{HitTestPolicy, HitTestRegionSnapshot, SurfaceTarget},
    platform::host::PlatformSurfaceHandle,
};

#[derive(Clone)]
pub(crate) struct SurfaceSlot {
    pub(crate) target: SurfaceTarget,
    pub(crate) layer: Retained<CALayerHost>,
    pub(crate) bounds: CGRect,
    pub(crate) visible: bool,
    pub(crate) hit_test: HitTestPolicy,
    pub(crate) hit_test_snapshot: Option<HitTestRegionSnapshot>,
    pub(crate) surface: Option<PlatformSurfaceHandle>,
    pub(crate) ime_bounds: Option<ImeBoundsUpdate>,
}
