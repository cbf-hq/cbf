//! Chrome backend adapters for `cbf-compositor`.
//!
//! These adapters consume Chrome-specific IPC and event details, then update
//! the compositor using browser-generic scene targets and state.

mod event_adapter;
mod surface_adapter;
mod transient_adapter;

pub(crate) use event_adapter::apply_chrome_event;
