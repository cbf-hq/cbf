//! Chromium-specific safe API layer for CBF.
//!
//! This crate will host Chromium-focused command/event models and backend
//! integration while keeping `cbf` browser-generic.

pub mod backend;
mod browser;
pub mod command;
pub mod data;
pub mod event;
pub mod ffi;
pub mod platform;
pub mod process;

pub use browser::ChromiumBrowserHandleExt;
