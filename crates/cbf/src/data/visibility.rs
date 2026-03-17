//! Browser-generic visibility state for a browsing context.

/// Visibility state applied to a browsing context by the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowsingContextVisibility {
    Visible,
    Hidden,
}
