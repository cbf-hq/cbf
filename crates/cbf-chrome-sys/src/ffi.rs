//! Chromium-specific low-level FFI bindings and bridge-facing primitives.
//!
//! This module keeps the handwritten wrapper layer minimal so a future
//! code-generation step can own most ABI declarations directly.

#[path = "ffi_bridge_generated.rs"]
mod ffi_bridge_generated;
#[path = "ffi_data_generated.rs"]
mod ffi_data_generated;

pub use ffi_bridge_generated::cbf_bridge as CbfBridge;
pub use ffi_data_generated::*;
