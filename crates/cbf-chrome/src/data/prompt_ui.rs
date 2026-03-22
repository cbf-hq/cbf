//! Chrome-specific prompt UI types for permission prompts, extension install dialogs, print preview, and download prompts.

use crate::data::download::{
    ChromeDownloadId, ChromeDownloadPromptReason, ChromeDownloadPromptResult,
};

/// Chrome-specific permission categories exposed through PromptUi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptUiPermissionType {
    Geolocation,
    Notifications,
    AudioCapture,
    VideoCapture,
    Unknown,
}

/// Chrome-specific reason for form resubmission prompts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptUiFormResubmissionReason {
    Reload,
    BackForward,
    Other,
    Unknown,
}

/// Stable id for a backend-managed prompt UI surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PromptUiId(u64);

impl PromptUiId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Chrome-specific prompt UI request kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptUiKind {
    PermissionPrompt {
        permission: PromptUiPermissionType,
        permission_key: Option<String>,
    },
    DownloadPrompt {
        download_id: ChromeDownloadId,
        file_name: String,
        total_bytes: Option<u64>,
        suggested_path: Option<String>,
        reason: ChromeDownloadPromptReason,
    },
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
    PrintPreviewDialog,
    FormResubmissionPrompt {
        reason: PromptUiFormResubmissionReason,
        target_url: Option<String>,
    },
    Unknown,
}

/// Chrome-specific prompt UI response payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptUiResponse {
    PermissionPrompt {
        allow: bool,
    },
    DownloadPrompt {
        allow: bool,
        destination_path: Option<String>,
    },
    ExtensionInstallPrompt {
        proceed: bool,
    },
    ExtensionUninstallPrompt {
        proceed: bool,
        report_abuse: bool,
    },
    PrintPreviewDialog {
        proceed: bool,
    },
    FormResubmissionPrompt {
        proceed: bool,
    },
    Unknown,
}

/// Chrome-specific result for extension install prompt resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptUiExtensionInstallResult {
    Accepted,
    AcceptedWithWithheldPermissions,
    UserCanceled,
    Aborted,
}

/// Chrome-specific result for extension uninstall prompt resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptUiExtensionUninstallResult {
    Accepted,
    UserCanceled,
    Aborted,
    Failed,
}

/// Chrome-specific prompt UI resolution result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptUiResolutionResult {
    Allowed,
    Denied,
    Aborted,
    Unknown,
}

/// Chrome-specific resolution result for non-permission dialogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptUiDialogResult {
    Proceeded,
    Canceled,
    Aborted,
    Unknown,
}

/// Chrome-specific prompt UI resolution payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptUiResolution {
    PermissionPrompt {
        permission: PromptUiPermissionType,
        permission_key: Option<String>,
        result: PromptUiResolutionResult,
    },
    DownloadPrompt {
        download_id: ChromeDownloadId,
        destination_path: Option<String>,
        result: ChromeDownloadPromptResult,
    },
    ExtensionInstallPrompt {
        extension_id: String,
        result: PromptUiExtensionInstallResult,
        detail: Option<String>,
    },
    ExtensionUninstallPrompt {
        extension_id: String,
        result: PromptUiExtensionUninstallResult,
        detail: Option<String>,
        report_abuse: bool,
    },
    PrintPreviewDialog {
        result: PromptUiDialogResult,
    },
    FormResubmissionPrompt {
        reason: PromptUiFormResubmissionReason,
        target_url: Option<String>,
        result: PromptUiResolutionResult,
    },
    Unknown,
}

/// Close reason for backend-managed prompt UI surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptUiCloseReason {
    UserCanceled,
    HostForced,
    SystemDismissed,
    Unknown,
}
