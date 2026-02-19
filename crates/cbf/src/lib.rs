//! CBF (Chromium Browser Framework) is a reusable Rust browser backend framework.
//!
//! This crate provides a browser-generic high-level API for controlling browser
//! backends and handling browser events. Chromium-specific integration and FFI
//! details are kept in lower layers.
//!
//! For setup, architecture, and implementation details, see the repository
//! documentation under `docs/`.

pub mod backend_delegate;
pub mod browser;
pub mod command;
pub mod data;
pub mod error;
pub mod event;
pub mod middleware;
pub mod platform;

#[cfg(feature = "dummy-backend")]
pub mod dummy_backend;

pub use cbf_sys as sys;

#[allow(dead_code, unused_imports)]
mod ffi;
