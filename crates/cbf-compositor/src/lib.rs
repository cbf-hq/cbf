//! Scene-based browser surface compositor for CBF desktop applications.
//!
//! This crate separates backend ownership relationships from host composition
//! relationships. A transient browsing context remains owned by a parent page,
//! but may still be rendered in any compositor-managed window.

mod backend;
pub mod core;
mod error;
pub mod model;
pub(crate) mod platform;
pub(crate) mod state;
mod window;

pub use error::CompositorError;
pub use window::WindowHost;
