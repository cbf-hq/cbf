use std::collections::HashMap;

use cbf::data::ime::ImeBoundsUpdate;

use crate::{model::SurfaceTarget, platform::host::PlatformSurfaceHandle};

#[derive(Debug, Default, Clone)]
pub(crate) struct TargetRuntimeState {
    pub(crate) surface: Option<PlatformSurfaceHandle>,
    pub(crate) ime_bounds: Option<ImeBoundsUpdate>,
    pub(crate) transient_preferred_size: Option<(u32, u32)>,
}

#[derive(Debug, Default)]
pub(crate) struct SurfaceState {
    states: HashMap<SurfaceTarget, TargetRuntimeState>,
}

impl SurfaceState {
    pub(crate) fn set_surface(&mut self, target: SurfaceTarget, handle: PlatformSurfaceHandle) {
        self.states.entry(target).or_default().surface = Some(handle);
    }

    pub(crate) fn set_ime_bounds(&mut self, target: SurfaceTarget, ime_bounds: ImeBoundsUpdate) {
        self.states.entry(target).or_default().ime_bounds = Some(ime_bounds);
    }

    pub(crate) fn set_transient_preferred_size(&mut self, target: SurfaceTarget, size: (u32, u32)) {
        self.states
            .entry(target)
            .or_default()
            .transient_preferred_size = Some(size);
    }

    pub(crate) fn get(&self, target: SurfaceTarget) -> Option<&TargetRuntimeState> {
        self.states.get(&target)
    }

    pub(crate) fn remove(&mut self, target: &SurfaceTarget) -> Option<TargetRuntimeState> {
        self.states.remove(target)
    }
}
