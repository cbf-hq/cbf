/// Identifier for a compositor-managed native window attachment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CompositorWindowId(u64);

impl CompositorWindowId {
    /// Construct an identifier from its raw numeric value.
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Return the raw numeric value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for CompositorWindowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get())
    }
}

/// Identifier for a scene item inside a compositor-managed window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CompositionItemId(u64);

impl CompositionItemId {
    /// Construct an identifier from its raw numeric value.
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Return the raw numeric value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for CompositionItemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get())
    }
}
