//! Browser-generic background policy controls for rendered surfaces.

/// Background drawing policy for a browser-managed surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundPolicy {
    /// Use a non-transparent page background.
    Opaque,
    /// Clear the page background to transparent.
    Transparent,
}
