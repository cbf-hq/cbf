//! Shared data models for transient browsing contexts.

use super::{
    ids::TransientBrowsingContextId,
    ime::{ImeTextRange, ImeTextSpan},
};

/// Browser-generic role of a transient browsing context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransientBrowsingContextKind {
    Popup,
    ToolWindow,
    Unknown,
}

/// Browser-generic reason for a transient browsing context closing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransientBrowsingContextCloseReason {
    UserDismissed,
    ParentClosed,
    Programmatic,
    RendererClosed,
    Unknown,
}

/// IME composition payload scoped to a transient browsing context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransientImeComposition {
    pub transient_browsing_context_id: TransientBrowsingContextId,
    pub text: String,
    pub selection_start: i32,
    pub selection_end: i32,
    pub replacement_range: Option<ImeTextRange>,
    pub spans: Vec<ImeTextSpan>,
}

/// IME commit payload scoped to a transient browsing context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransientImeCommitText {
    pub transient_browsing_context_id: TransientBrowsingContextId,
    pub text: String,
    pub relative_caret_position: i32,
    pub replacement_range: Option<ImeTextRange>,
    pub spans: Vec<ImeTextSpan>,
}
