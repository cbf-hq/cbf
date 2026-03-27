//! Bindgen-generated ABI mirrors for the `cbf_bridge` C interface.
//!
//! This module re-exports the generated bridge and data bindings consumed by
//! `cbf-chrome` and the runtime bridge loader. It is the raw FFI layer: ABI
//! names, layouts, and constants should stay aligned with Chromium bridge
//! headers rather than being hand-written here.
//!
//! Prefer using [`crate::bridge`] when you need loaded bridge symbols at
//! runtime; this module exists to expose the generated low-level bindings.

#[path = "ffi_bridge_generated.rs"]
mod ffi_bridge_generated;
#[path = "ffi_data_generated.rs"]
mod ffi_data_generated;

pub use ffi_bridge_generated::cbf_bridge as CbfBridge;
pub use ffi_data_generated::*;
