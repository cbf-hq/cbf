//! Shared data models used across commands, events, and backend integration.
//!
//! These types represent browser-facing concepts such as input events, IDs,
//! profiles, and context menu payloads.

pub mod auxiliary_window;
pub mod background;
pub mod browsing_context_open;
pub mod context_menu;
pub mod dialog;
pub mod download;
pub mod drag;
pub mod edit;
pub mod extension;
pub mod ids;
pub mod ime;
pub mod key;
pub mod mouse;
pub mod permission;
pub mod profile;
pub mod transient_browsing_context;
pub mod visibility;
pub mod window_open;
