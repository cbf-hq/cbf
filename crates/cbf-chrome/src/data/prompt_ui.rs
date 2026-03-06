//! Chrome-specific prompt UI types for permission prompts, extension install dialogs, and print preview.

/// Chrome-specific permission categories exposed through PromptUi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptUiPermissionType {
    Geolocation,
    Notifications,
    AudioCapture,
    VideoCapture,
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
    ExtensionInstallPrompt {
        extension_id: String,
        extension_name: String,
        permission_names: Vec<String>,
    },
    PrintPreviewDialog,
    Unknown,
}

/// Chrome-specific prompt UI response payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptUiResponse {
    PermissionPrompt { allow: bool },
    ExtensionInstallPrompt { proceed: bool },
    PrintPreviewDialog { proceed: bool },
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
    ExtensionInstallPrompt {
        extension_id: String,
        result: PromptUiExtensionInstallResult,
        detail: Option<String>,
    },
    PrintPreviewDialog {
        result: PromptUiDialogResult,
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
