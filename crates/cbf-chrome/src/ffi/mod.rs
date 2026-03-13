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

/// Errors that can occur in the IPC bridge layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Failed to connect to the IPC channel.
    ConnectionFailed,
    /// Input data was invalid for the FFI layer.
    InvalidInput,
    /// An IPC event could not be parsed.
    InvalidEvent,
}
