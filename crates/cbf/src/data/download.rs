//! Browser-generic download lifecycle types.

/// Stable identifier for a backend-managed download.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DownloadId(u64);

impl DownloadId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Result for the pre-start download prompt lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadPromptResult {
    Allowed,
    Denied,
    Aborted,
}

/// Browser-generic action hint for host handling of a download prompt.
///
/// This hint is intended to represent common behavior seen in major browsers,
/// but it is not guaranteed to exactly match backend-specific behavior.
/// Backends may still deviate based on platform or policy details.
///
/// If your application needs detailed backend-specific reason data, use the
/// backend's raw events (for example, `cbf-chrome` raw events) instead of this
/// browser-generic hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadPromptActionHint {
    /// Proceed without showing a destination selection dialog.
    AutoSave,
    /// Show a destination selection dialog before proceeding.
    SelectDestination,
    /// Deny the download prompt request.
    Deny,
    /// Backend could not provide a stable hint.
    Unknown,
}

/// Current state of a download.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadState {
    InProgress,
    Paused,
    Completed,
    Cancelled,
    Interrupted,
    Unknown,
}

/// Terminal outcome of a download.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadOutcome {
    Succeeded,
    Cancelled,
    Interrupted,
    Unknown,
}
