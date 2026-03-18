//! Backend-specific event adapters for `cbf-compositor`.
//!
//! This module translates backend-native events into compositor state updates
//! without leaking backend details into the public scene model.

#[cfg(feature = "chrome")]
/// Chrome-specific adapters for surface handles and popup lifecycle events.
pub mod chrome;
