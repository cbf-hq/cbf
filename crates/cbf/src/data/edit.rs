//! Data models for browser-generic edit actions.

/// Browser-generic edit command executed against the focused element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditAction {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
}
