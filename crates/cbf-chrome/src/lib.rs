//! Chromium-specific safe API layer for CBF.
//!
//! This crate will host Chromium-focused command/event models and backend
//! integration while keeping `cbf` browser-generic.

pub mod chromium_backend;
pub mod chromium_process;
pub mod ffi;

pub use cbf;
