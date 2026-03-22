//! Auxiliary window data models for backend-managed dialogs and prompt flows.

use crate::data::download::{DownloadId, DownloadPromptActionHint, DownloadPromptResult};
use crate::data::extension::{ExtensionInstallPromptResult, ExtensionUninstallPromptResult};

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

/// Browser-generic reason for form resubmission prompts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormResubmissionPromptReason {
    Reload,
    BackForward,
    Other,
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
    ExtensionUninstallPrompt {
        extension_id: String,
        extension_name: String,
        triggering_extension_name: Option<String>,
        can_report_abuse: bool,
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
    FormResubmissionPrompt {
        reason: FormResubmissionPromptReason,
        target_url: Option<String>,
    },
    Unknown,
}

/// Host response for an auxiliary window request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuxiliaryWindowResponse {
    ExtensionInstallPrompt {
        proceed: bool,
    },
    ExtensionUninstallPrompt {
        proceed: bool,
        report_abuse: bool,
    },
    PermissionPrompt {
        allow: bool,
    },
    DownloadPrompt {
        allow: bool,
        destination_path: Option<String>,
    },
    FormResubmissionPrompt {
        proceed: bool,
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
    ExtensionUninstallPrompt {
        extension_id: String,
        result: ExtensionUninstallPromptResult,
        detail: Option<String>,
        report_abuse: bool,
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
    FormResubmissionPrompt {
        reason: FormResubmissionPromptReason,
        target_url: Option<String>,
        result: PermissionPromptResult,
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
