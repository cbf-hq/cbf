//! Stable logical identifiers used across the browser API boundary.

/// A stable identifier for a browsing context managed by the browser backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BrowsingContextId(pub u64);

impl std::fmt::Display for BrowsingContextId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get())
    }
}

/// A stable identifier for a host-managed window abstraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(pub u64);

/// A stable identifier for a transient browsing context managed by the backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TransientBrowsingContextId(pub u64);

impl std::fmt::Display for WindowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get())
    }
}

impl WindowId {
    /// Create a new identifier from a raw numeric value.
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Get the raw numeric value of this identifier.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl TransientBrowsingContextId {
    /// Create a new identifier from a raw numeric value.
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Get the raw numeric value of this identifier.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl BrowsingContextId {
    /// Create a new identifier from a raw numeric value.
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Get the raw numeric value of this identifier.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for TransientBrowsingContextId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get())
    }
}
