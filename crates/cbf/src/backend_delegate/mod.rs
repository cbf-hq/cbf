//! Delegate interfaces for command/event mediation in CBF.
//!
//! [`BackendDelegate`] is the extension point that lets applications observe,
//! transform, or stop command and event flow between `BrowserCommand` and
//! `BrowserEvent`.
//!
//! For layered composition, see `crate::middleware` and
//! `crate::middleware::MiddlewareDelegate`, which wraps multiple delegates into
//! a single delegate instance.

mod dispatcher;
mod types;

pub use dispatcher::*;
pub use types::*;
