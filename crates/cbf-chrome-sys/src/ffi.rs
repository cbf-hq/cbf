//! Chromium-specific low-level FFI bindings and bridge-facing primitives.
//!
//! This module keeps the handwritten wrapper layer minimal so a future
//! code-generation step can own most ABI declarations directly.

/// Opaque handle to a bridge-owned IPC client instance.
#[repr(C)]
pub struct CbfBridgeClientHandle {
    _private: [u8; 0],
}

/// Bindgen's dynamic-loading output expects the exported wait status typedef by name.
pub type CbfBridgeEventWaitStatus = i32;

#[path = "bridge_api_generated.rs"]
mod bridge_api_generated;
#[path = "ffi_generated.rs"]
mod ffi_generated;

pub use bridge_api_generated::cbf_bridge as CbfBridge;
pub use ffi_generated::*;
