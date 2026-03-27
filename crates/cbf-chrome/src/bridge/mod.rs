//! FFI bridge adapters between Rust-side models and `cbf-chrome-sys` C ABI types.
//!
//! This module handles low-level IPC event decoding and conversion utilities
//! that remain internal to the crate boundary.

mod client;
mod event;
mod map;
mod utils;

pub(crate) use client::IpcEventWaitHandle;
pub use client::{EventWaitResult, IpcClient};
pub use event::IpcEvent;

/// Convert native NSEvent input to CBF input events on macOS.
#[cfg(target_os = "macos")]
pub use map::{
    convert_nsevent_to_chrome_key_event, convert_nsevent_to_chrome_mouse_event,
    convert_nsevent_to_chrome_mouse_wheel_event, convert_nsevent_to_key_event,
    convert_nsevent_to_mouse_event, convert_nsevent_to_mouse_wheel_event,
    convert_nspasteboard_to_drag_data,
};

/// Errors that can occur in the `cbf-chrome-sys` (`cbf_bridge`) layer.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BridgeError {
    /// Failed to load the runtime bridge library or one of its required symbols.
    #[error("failed to load the runtime bridge library")]
    BridgeLoadFailed,
    /// Failed to connect to the IPC channel.
    #[error("failed to connect to the IPC channel")]
    ConnectionFailed,
    /// Input data was invalid for the FFI layer.
    #[error("invalid input for the FFI layer")]
    InvalidInput,
    /// An IPC event could not be parsed.
    #[error("failed to parse IPC event")]
    InvalidEvent,
}
