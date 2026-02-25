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
    Unknown,
}

/// Host response for an auxiliary window request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuxiliaryWindowResponse {
    ExtensionInstallPrompt { proceed: bool },
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
