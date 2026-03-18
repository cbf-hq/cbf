//! Chrome transport background policy conversions.

/// Background drawing policy transported to the Chrome backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeBackgroundPolicy {
    Opaque,
    Transparent,
}

impl From<cbf::data::background::BackgroundPolicy> for ChromeBackgroundPolicy {
    fn from(value: cbf::data::background::BackgroundPolicy) -> Self {
        match value {
            cbf::data::background::BackgroundPolicy::Opaque => Self::Opaque,
            cbf::data::background::BackgroundPolicy::Transparent => Self::Transparent,
        }
    }
}
