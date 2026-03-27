//! Backend-specific event adapters for `cbf-compositor`.
//!
//! This module translates backend-native events into compositor state updates
//! without leaking backend details into the public scene model.

/// Chrome-specific adapters for surface handles and popup lifecycle events.
#[cfg(feature = "chrome")]
pub mod chrome;
