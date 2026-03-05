use cbf::data::browsing_context_open::{BrowsingContextOpenHint, BrowsingContextOpenResult};

use super::ids::TabId;

/// Chromium-facing hint describing how opener requested a tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabOpenHint {
    Unknown,
    CurrentTab,
    NewForegroundTab,
    NewBackgroundTab,
    NewWindow,
    Popup,
}

impl TabOpenHint {
    /// Convert into generic-layer hint only for non-window-open cases.
    pub const fn to_browsing_context_open_hint(self) -> Option<BrowsingContextOpenHint> {
        match self {
            TabOpenHint::Unknown => Some(BrowsingContextOpenHint::Unknown),
            TabOpenHint::CurrentTab => Some(BrowsingContextOpenHint::CurrentContext),
            TabOpenHint::NewForegroundTab => Some(BrowsingContextOpenHint::NewForegroundContext),
            TabOpenHint::NewBackgroundTab => Some(BrowsingContextOpenHint::NewBackgroundContext),
            TabOpenHint::NewWindow | TabOpenHint::Popup => None,
        }
    }
}

impl From<BrowsingContextOpenHint> for TabOpenHint {
    fn from(value: BrowsingContextOpenHint) -> Self {
        match value {
            BrowsingContextOpenHint::Unknown => Self::Unknown,
            BrowsingContextOpenHint::CurrentContext => Self::CurrentTab,
            BrowsingContextOpenHint::NewForegroundContext => Self::NewForegroundTab,
            BrowsingContextOpenHint::NewBackgroundContext => Self::NewBackgroundTab,
        }
    }
}

/// Chromium-facing result emitted after backend applies tab-open response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabOpenResult {
    OpenedNewTab { tab_id: TabId },
    OpenedExistingTab { tab_id: TabId },
    Denied,
    Aborted,
}

impl From<TabOpenResult> for BrowsingContextOpenResult {
    fn from(value: TabOpenResult) -> Self {
        match value {
            TabOpenResult::OpenedNewTab { tab_id } => Self::OpenedNewContext {
                browsing_context_id: tab_id.into(),
            },
            TabOpenResult::OpenedExistingTab { tab_id } => Self::OpenedExistingContext {
                browsing_context_id: tab_id.into(),
            },
            TabOpenResult::Denied => Self::Denied,
            TabOpenResult::Aborted => Self::Aborted,
        }
    }
}

impl From<BrowsingContextOpenResult> for TabOpenResult {
    fn from(value: BrowsingContextOpenResult) -> Self {
        match value {
            BrowsingContextOpenResult::OpenedNewContext {
                browsing_context_id,
            } => Self::OpenedNewTab {
                tab_id: browsing_context_id.into(),
            },
            BrowsingContextOpenResult::OpenedExistingContext {
                browsing_context_id,
            } => Self::OpenedExistingTab {
                tab_id: browsing_context_id.into(),
            },
            BrowsingContextOpenResult::Denied => Self::Denied,
            BrowsingContextOpenResult::Aborted => Self::Aborted,
        }
    }
}

#[cfg(test)]
mod tests {
    use cbf::data::browsing_context_open::{BrowsingContextOpenHint, BrowsingContextOpenResult};

    use crate::data::ids::TabId;

    use super::{TabOpenHint, TabOpenResult};

    #[test]
    fn tab_open_hint_round_trip() {
        assert_eq!(
            TabOpenHint::from(BrowsingContextOpenHint::CurrentContext),
            TabOpenHint::CurrentTab
        );
        assert_eq!(
            TabOpenHint::NewBackgroundTab.to_browsing_context_open_hint(),
            Some(BrowsingContextOpenHint::NewBackgroundContext)
        );
        assert_eq!(TabOpenHint::Popup.to_browsing_context_open_hint(), None);
    }

    #[test]
    fn tab_open_result_round_trip() {
        let raw = TabOpenResult::OpenedExistingTab {
            tab_id: TabId::new(42),
        };
        let generic = BrowsingContextOpenResult::from(raw);
        let round_trip = TabOpenResult::from(generic);

        assert_eq!(round_trip, raw);
    }
}
