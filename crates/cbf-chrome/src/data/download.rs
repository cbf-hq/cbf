//! Chrome-specific download lifecycle types.

use cbf::data::download::{DownloadId, DownloadOutcome, DownloadPromptResult, DownloadState};

use crate::data::ids::TabId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChromeDownloadId(u64);

impl ChromeDownloadId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub const fn to_generic(self) -> DownloadId {
        DownloadId::new(self.get())
    }
}

impl From<DownloadId> for ChromeDownloadId {
    fn from(value: DownloadId) -> Self {
        Self::new(value.get())
    }
}

impl From<ChromeDownloadId> for DownloadId {
    fn from(value: ChromeDownloadId) -> Self {
        value.to_generic()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeDownloadPromptResult {
    Allowed,
    Denied,
    Aborted,
}

impl From<ChromeDownloadPromptResult> for DownloadPromptResult {
    fn from(value: ChromeDownloadPromptResult) -> Self {
        match value {
            ChromeDownloadPromptResult::Allowed => Self::Allowed,
            ChromeDownloadPromptResult::Denied => Self::Denied,
            ChromeDownloadPromptResult::Aborted => Self::Aborted,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeDownloadState {
    InProgress,
    Paused,
    Completed,
    Cancelled,
    Interrupted,
    Unknown,
}

impl From<ChromeDownloadState> for DownloadState {
    fn from(value: ChromeDownloadState) -> Self {
        match value {
            ChromeDownloadState::InProgress => Self::InProgress,
            ChromeDownloadState::Paused => Self::Paused,
            ChromeDownloadState::Completed => Self::Completed,
            ChromeDownloadState::Cancelled => Self::Cancelled,
            ChromeDownloadState::Interrupted => Self::Interrupted,
            ChromeDownloadState::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeDownloadOutcome {
    Succeeded,
    Cancelled,
    Interrupted,
    Unknown,
}

impl From<ChromeDownloadOutcome> for DownloadOutcome {
    fn from(value: ChromeDownloadOutcome) -> Self {
        match value {
            ChromeDownloadOutcome::Succeeded => Self::Succeeded,
            ChromeDownloadOutcome::Cancelled => Self::Cancelled,
            ChromeDownloadOutcome::Interrupted => Self::Interrupted,
            ChromeDownloadOutcome::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeDownloadSnapshot {
    pub download_id: ChromeDownloadId,
    pub source_tab_id: Option<TabId>,
    pub file_name: String,
    pub total_bytes: Option<u64>,
    pub target_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeDownloadProgress {
    pub download_id: ChromeDownloadId,
    pub source_tab_id: Option<TabId>,
    pub state: ChromeDownloadState,
    pub file_name: String,
    pub received_bytes: u64,
    pub total_bytes: Option<u64>,
    pub target_path: Option<String>,
    pub can_resume: bool,
    pub is_paused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeDownloadCompletion {
    pub download_id: ChromeDownloadId,
    pub source_tab_id: Option<TabId>,
    pub outcome: ChromeDownloadOutcome,
    pub file_name: String,
    pub received_bytes: u64,
    pub total_bytes: Option<u64>,
    pub target_path: Option<String>,
}
