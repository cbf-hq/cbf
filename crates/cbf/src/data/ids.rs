/// A stable identifier for a web page (tab) managed by the browser backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WebPageId(pub u64);

impl std::fmt::Display for WebPageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get())
    }
}

impl WebPageId {
    /// Create a new identifier from a raw numeric value.
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Get the raw numeric value of this identifier.
    pub const fn get(self) -> u64 {
        self.0
    }
}
