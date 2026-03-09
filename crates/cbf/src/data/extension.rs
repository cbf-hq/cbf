//! Data models for browser extension metadata and auxiliary prompt results.

use crate::data::download::{DownloadId, DownloadPromptActionHint, DownloadPromptResult};

/// Browser-generic icon payload exposed by backends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconData {
    Url(String),
    Png(Vec<u8>),
    Binary {
        media_type: Option<String>,
        bytes: Vec<u8>,
    },
}

/// Extension metadata exposed by backends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub enabled: bool,
    pub permission_names: Vec<String>,
    pub icon: Option<IconData>,
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
        /// Browser-generic action hint for host behavior.
        ///
        /// Detailed backend-specific reasons are intentionally not exposed in
        /// `cbf`; inspect backend raw events (for example from `cbf-chrome`)
        /// if your application needs exact reason codes.
        action_hint: DownloadPromptActionHint,
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

#[cfg(test)]
mod tests {
    use super::{ExtensionInfo, IconData};

    #[test]
    fn extension_info_preserves_png_icon() {
        let info = ExtensionInfo {
            id: "ext".to_string(),
            name: "Example".to_string(),
            version: "1.0.0".to_string(),
            enabled: true,
            permission_names: vec!["tabs".to_string()],
            icon: Some(IconData::Png(vec![1, 2, 3])),
        };

        assert_eq!(info.icon, Some(IconData::Png(vec![1, 2, 3])));
    }
}
