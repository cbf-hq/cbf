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

/// Response payload for a JavaScript dialog request.
#[derive(Debug, Clone)]
pub enum DialogResponse {
    Success {
        input: Option<String>, // Input text for prompt dialogs.
    },
    Cancel,
}
