//! Core compositor orchestration and public command types.
//!
//! The core module owns scene state, ownership relationships, and window
//! attachment while platform and backend adapters remain internal details.

mod commands;
mod compositor;

pub use commands::CompositionCommand;
pub use compositor::{AttachWindowOptions, Compositor};
