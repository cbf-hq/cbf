//! Chrome-facing stable identifiers; primarily `TabId` as the Chromium-layer counterpart of `BrowsingContextId`.

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

#[cfg(test)]
mod tests {
    use cbf::data::ids::BrowsingContextId;

    use super::TabId;

    #[test]
    fn tab_id_round_trip_preserves_raw_value() {
        let original = BrowsingContextId::new(4242);

        let tab_id = TabId::from(original);
        let round_trip = tab_id.to_browsing_context_id();

        assert_eq!(round_trip, BrowsingContextId::new(4242));
        assert_eq!(tab_id, TabId::new(4242));
    }
}
