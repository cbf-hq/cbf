//! Chromium-specific low-level FFI bindings and bridge-facing primitives.
//!
//! This module keeps the handwritten wrapper layer minimal so a future
//! code-generation step can own most ABI declarations directly.

#[repr(C)]
pub struct CbfBridgeClientHandle {
    _private: [u8; 0],
}

#[path = "ffi_generated.rs"]
mod ffi_generated;

pub use ffi_generated::*;
