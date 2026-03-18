//! Scene-based browser surface compositor for CBF desktop applications.
//!
//! This crate separates backend ownership relationships from host composition
//! relationships. A transient browsing context remains owned by a parent page,
//! but may still be rendered in any compositor-managed window.

pub mod backend;
pub mod core;
pub mod error;
pub mod model;
pub(crate) mod platform;
pub(crate) mod state;
pub mod window;

pub use core::{AttachWindowOptions, CompositionCommand, Compositor};
pub use error::CompositorError;
pub use model::{
    BackgroundPolicy, CompositionItemId, CompositionItemSpec, CompositorWindowId, Rect,
    SurfaceTarget, TransientOwnership, WindowCompositionSpec,
};
pub use window::WindowHost;
