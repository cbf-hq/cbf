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
    /// ObjC wrapper for CALayerHost used to host the Chromium surface.
    #[unsafe(super(CALayer))]
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
        /// Get the current CALayerHost context id.
        #[unsafe(method(contextId))]
        pub fn contextId(&self) -> ContextId;

        /// Set the CALayerHost context id.
        #[unsafe(method(setContextId:))]
        pub unsafe fn setContextId(&self, contextId: ContextId);
    );
}
