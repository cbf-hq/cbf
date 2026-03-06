//! Data models for browsing context open requests, host responses, and open results.

use super::ids::BrowsingContextId;

/// Advisory hint describing how opener requested a new browsing context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowsingContextOpenHint {
    Unknown,
    CurrentContext,
    NewForegroundContext,
    NewBackgroundContext,
}

/// Host decision for an open request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowsingContextOpenResponse {
    AllowNewContext {
        activate: bool,
    },
    AllowExistingContext {
        browsing_context_id: BrowsingContextId,
        activate: bool,
    },
    Deny,
}

/// Result emitted after backend applies host open response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowsingContextOpenResult {
    OpenedNewContext {
        browsing_context_id: BrowsingContextId,
    },
    OpenedExistingContext {
        browsing_context_id: BrowsingContextId,
    },
    Denied,
    Aborted,
}
