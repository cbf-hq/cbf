//! Chrome transport visibility state for tabs.

use cbf::data::visibility::BrowsingContextVisibility;

/// Represents the visibility state of a Chromium tab.
///
/// This enum indicates whether a tab is currently visible to the user or hidden.
/// The `Hidden` variant is used when a tab is explicitly not visible, such as when
/// the user switches to a different tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeTabVisibility {
    Visible,
    Hidden,
}

impl From<BrowsingContextVisibility> for ChromeTabVisibility {
    fn from(value: BrowsingContextVisibility) -> Self {
        match value {
            BrowsingContextVisibility::Visible => Self::Visible,
            BrowsingContextVisibility::Hidden => Self::Hidden,
        }
    }
}
