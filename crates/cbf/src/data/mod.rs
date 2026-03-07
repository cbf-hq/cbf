//! Shared data models used across commands, events, and backend integration.
//!
//! These types represent browser-facing concepts such as input events, IDs,
//! profiles, and context menu payloads.

pub mod browsing_context_open;
pub mod context_menu;
pub mod dialog;
pub mod download;
pub mod drag;
pub mod extension;
pub mod ids;
pub mod ime;
pub mod key;
pub mod mouse;
pub mod permission;
pub mod profile;
pub mod window_open;
