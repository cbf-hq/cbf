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

impl From<TabOpenHint> for BrowsingContextOpenHint {
    fn from(value: TabOpenHint) -> Self {
        match value {
            TabOpenHint::Unknown => Self::Unknown,
            TabOpenHint::CurrentTab => Self::CurrentContext,
            TabOpenHint::NewForegroundTab => Self::NewForegroundContext,
            TabOpenHint::NewBackgroundTab => Self::NewBackgroundContext,
            TabOpenHint::NewWindow => Self::NewWindow,
            TabOpenHint::Popup => Self::Popup,
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
            BrowsingContextOpenHint::NewWindow => Self::NewWindow,
            BrowsingContextOpenHint::Popup => Self::Popup,
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
            BrowsingContextOpenHint::from(TabOpenHint::NewBackgroundTab),
            BrowsingContextOpenHint::NewBackgroundContext
        );
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
