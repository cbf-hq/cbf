//! Data models for browser extension metadata and auxiliary prompt results.

use crate::data::download::{DownloadId, DownloadPromptResult};

/// Extension metadata exposed by backends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub enabled: bool,
    pub permission_names: Vec<String>,
}

/// Result for extension install prompt lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionInstallPromptResult {
    Accepted,
    AcceptedWithWithheldPermissions,
    UserCanceled,
    Aborted,
}

/// Browser-generic permission categories used by permission prompt UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionPromptType {
    Geolocation,
    Notifications,
    AudioCapture,
    VideoCapture,
    Other(String),
    Unknown,
}

/// Browser-generic result for permission prompt resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionPromptResult {
    Allowed,
    Denied,
    Aborted,
    Unknown,
}

/// Stable id for a backend-managed auxiliary dialog/window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AuxiliaryWindowId(u64);

impl AuxiliaryWindowId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Kind-specific payload for auxiliary window requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuxiliaryWindowKind {
    ExtensionInstallPrompt {
        extension_id: String,
        extension_name: String,
        permission_names: Vec<String>,
    },
    PermissionPrompt {
        permission: PermissionPromptType,
    },
    DownloadPrompt {
        download_id: DownloadId,
        file_name: String,
        total_bytes: Option<u64>,
        suggested_path: Option<String>,
    },
    PrintPreviewDialog,
    Unknown,
}

/// Host response for an auxiliary window request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuxiliaryWindowResponse {
    ExtensionInstallPrompt {
        proceed: bool,
    },
    PermissionPrompt {
        allow: bool,
    },
    DownloadPrompt {
        allow: bool,
        destination_path: Option<String>,
    },
    Unknown,
}

/// Kind-specific resolution payload for auxiliary window lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuxiliaryWindowResolution {
    ExtensionInstallPrompt {
        extension_id: String,
        result: ExtensionInstallPromptResult,
        detail: Option<String>,
    },
    PermissionPrompt {
        permission: PermissionPromptType,
        result: PermissionPromptResult,
    },
    DownloadPrompt {
        download_id: DownloadId,
        result: DownloadPromptResult,
        destination_path: Option<String>,
    },
    Unknown,
}

/// Close reason for backend-managed auxiliary dialog/window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuxiliaryWindowCloseReason {
    UserCanceled,
    HostForced,
    SystemDismissed,
    Unknown,
}
