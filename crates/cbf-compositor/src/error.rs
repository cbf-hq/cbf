//! Error types returned by `cbf-compositor` public operations.

use thiserror::Error;

/// Errors produced by compositor state management and platform attachment.
#[derive(Debug, Error)]
pub enum CompositorError {
    /// The requested compositor window is not attached.
    #[error("unknown window")]
    UnknownWindow,
    /// The requested browser surface target is not known to the compositor.
    #[error("unknown surface target")]
    UnknownTarget,
    /// The requested scene item is not present in the composition state.
    #[error("unknown composition item")]
    UnknownItem,
    /// The requested scene item cannot accept focus.
    #[error("composition item is not interactive")]
    ItemNotInteractive,
    /// A scene item cannot belong to two compositor windows at once.
    #[error("composition item is already attached to another window")]
    ItemOwnedByAnotherWindow,
    /// A browser surface target cannot appear more than once in the live composition.
    #[error("surface target is already attached in the live composition")]
    DuplicateSurfaceTarget,
    /// The current target/platform combination does not support native hosting.
    #[error("platform-specific compositor support is unavailable")]
    PlatformUnsupported,
}
