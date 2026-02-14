//! Bindings for macOS CALayerHost.

#![allow(non_snake_case)]

use objc2::{
    extern_class, extern_methods,
    rc::{Allocated, Retained},
};
use objc2_quartz_core::CALayer;

/// CALayerHost context identifier type.
pub type ContextId = std::ffi::c_uint;

extern_class!(
    #[unsafe(super(CALayer))]
    /// ObjC wrapper for CALayerHost used to host the Chromium surface.
    pub struct CALayerHost;
);

impl CALayerHost {
    extern_methods!(
        #[unsafe(method(init))]
        #[unsafe(method_family = init)]
        pub fn init(this: Allocated<Self>) -> Retained<Self>;
    );
}

impl CALayerHost {
    extern_methods!(
        #[unsafe(method(contextId))]
        /// Get the current CALayerHost context id.
        pub fn contextId(&self) -> ContextId;

        #[unsafe(method(setContextId:))]
        /// Set the CALayerHost context id.
        pub unsafe fn setContextId(&self, contextId: ContextId);
    );
}
