//! Low-level Chromium bridge bindings for CBF.
//!
//! This crate owns the unsafe C ABI boundary to `cbf_bridge` and the runtime
//! loading path for the bridge library used by higher Chrome-specific layers.
//! Generated ABI mirrors live in [`ffi`], while [`bridge`] provides CBF-specific
//! library discovery and process-wide bridge access.
//!
//! Higher crates should keep Chromium details contained here instead of
//! re-declaring bridge symbols or loading the bridge library themselves.

pub mod bridge;
pub mod ffi;
