//! Chromium-specific low-level FFI bindings and bridge-facing primitives.
//!
//! This crate owns the unsafe C ABI contract with `cbf_bridge`.

pub mod ffi;
pub mod modifiers;
