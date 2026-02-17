//! Low-level FFI bindings and bridge-facing primitives for CBF.
//!
//! This crate is an implementation layer for Chromium integration.
//! Most applications should depend on the higher-level `cbf` crate instead of
//! using `cbf-sys` directly.

pub mod ffi;
pub mod modifiers;
