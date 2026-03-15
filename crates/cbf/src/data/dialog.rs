//! Dialog-related data models for JavaScript dialogs and beforeunload flows.

/// Types of JavaScript dialogs or beforeunload confirmations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogType {
    Alert,
    Confirm,
    Prompt,
    BeforeUnload,
}

/// Reasons for triggering a beforeunload confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeforeUnloadReason {
    Unknown,
    CloseBrowsingContext,
    Navigate,
    Reload,
    WindowClose,
}

/// Browser-generic request payload for JavaScript dialogs.
///
/// This request model is intended for `alert`, `confirm`, and `prompt`.
/// `DialogType::BeforeUnload` remains handled via the existing dedicated flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaScriptDialogRequest {
    pub r#type: DialogType,
    pub message: String,
    pub default_prompt_text: Option<String>,
}

impl JavaScriptDialogRequest {
    /// Creates a browser-generic JavaScript dialog request payload.
    pub fn new(
        r#type: DialogType,
        message: impl Into<String>,
        default_prompt_text: Option<String>,
    ) -> Self {
        Self {
            r#type,
            message: message.into(),
            default_prompt_text,
        }
    }
}

/// Response payload for a JavaScript dialog request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogResponse {
    Success {
        input: Option<String>, // Input text for prompt dialogs.
    },
    Cancel,
}
