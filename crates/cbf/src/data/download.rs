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
