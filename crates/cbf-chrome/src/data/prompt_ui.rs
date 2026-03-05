/// Chrome-specific permission categories exposed through PromptUi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptUiPermissionType {
    Geolocation,
    Notifications,
    AudioCapture,
    VideoCapture,
    Unknown,
}

/// Chrome-specific prompt UI request kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptUiKind {
    PermissionPrompt {
        permission: PromptUiPermissionType,
        permission_key: Option<String>,
    },
    Unknown,
}

/// Chrome-specific prompt UI response payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptUiResponse {
    PermissionPrompt { allow: bool },
    Unknown,
}

/// Chrome-specific prompt UI resolution result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptUiResolutionResult {
    Allowed,
    Denied,
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
    Unknown,
}
