//! Chromium-specific safe API layer for CBF.
//!
//! This crate will host Chromium-focused command/event models and backend
//! integration while keeping `cbf` browser-generic.

pub mod backend;
pub mod process;
pub mod command;
pub mod context_menu;
pub mod event;
pub mod ffi;
pub mod input;
pub mod platform;
pub mod surface;

pub use cbf;
