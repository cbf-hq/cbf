use cbf::data::ids::BrowsingContextId;

/// Chrome-facing stable identifier for a tab managed by Chromium runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TabId(pub u64);

impl std::fmt::Display for TabId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get())
    }
}

impl TabId {
    /// Create a new identifier from a raw numeric value.
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Get the raw numeric value of this identifier.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Convert from generic-layer browsing context ID.
    pub const fn from_browsing_context_id(id: BrowsingContextId) -> Self {
        Self(id.get())
    }

    /// Convert into generic-layer browsing context ID.
    pub const fn to_browsing_context_id(self) -> BrowsingContextId {
        BrowsingContextId::new(self.get())
    }
}

impl From<BrowsingContextId> for TabId {
    fn from(value: BrowsingContextId) -> Self {
        Self::from_browsing_context_id(value)
    }
}

impl From<TabId> for BrowsingContextId {
    fn from(value: TabId) -> Self {
        value.to_browsing_context_id()
    }
}
