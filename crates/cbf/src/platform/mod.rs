//! Platform-specific integrations used by `cbf`.

/// macOS-specific platform bindings and view helpers.
#[cfg(target_os = "macos")]
pub mod macos;
