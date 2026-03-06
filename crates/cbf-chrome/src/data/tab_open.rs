use super::{
    browsing_context_open::{ChromeBrowsingContextOpenHint, ChromeBrowsingContextOpenResult},
    ids::TabId,
};

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
    pub const fn to_browsing_context_open_hint(self) -> Option<ChromeBrowsingContextOpenHint> {
        match self {
            TabOpenHint::Unknown => Some(ChromeBrowsingContextOpenHint::Unknown),
            TabOpenHint::CurrentTab => Some(ChromeBrowsingContextOpenHint::CurrentContext),
            TabOpenHint::NewForegroundTab => {
                Some(ChromeBrowsingContextOpenHint::NewForegroundContext)
            }
            TabOpenHint::NewBackgroundTab => {
                Some(ChromeBrowsingContextOpenHint::NewBackgroundContext)
            }
            TabOpenHint::NewWindow | TabOpenHint::Popup => None,
        }
    }
}

impl From<ChromeBrowsingContextOpenHint> for TabOpenHint {
    fn from(value: ChromeBrowsingContextOpenHint) -> Self {
        match value {
            ChromeBrowsingContextOpenHint::Unknown => Self::Unknown,
            ChromeBrowsingContextOpenHint::CurrentContext => Self::CurrentTab,
            ChromeBrowsingContextOpenHint::NewForegroundContext => Self::NewForegroundTab,
            ChromeBrowsingContextOpenHint::NewBackgroundContext => Self::NewBackgroundTab,
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

impl From<TabOpenResult> for ChromeBrowsingContextOpenResult {
    fn from(value: TabOpenResult) -> Self {
        match value {
            TabOpenResult::OpenedNewTab { tab_id } => Self::OpenedNewContext { tab_id },
            TabOpenResult::OpenedExistingTab { tab_id } => Self::OpenedExistingContext { tab_id },
            TabOpenResult::Denied => Self::Denied,
            TabOpenResult::Aborted => Self::Aborted,
        }
    }
}

impl From<ChromeBrowsingContextOpenResult> for TabOpenResult {
    fn from(value: ChromeBrowsingContextOpenResult) -> Self {
        match value {
            ChromeBrowsingContextOpenResult::OpenedNewContext { tab_id } => {
                Self::OpenedNewTab { tab_id }
            }
            ChromeBrowsingContextOpenResult::OpenedExistingContext { tab_id } => {
                Self::OpenedExistingTab { tab_id }
            }
            ChromeBrowsingContextOpenResult::Denied => Self::Denied,
            ChromeBrowsingContextOpenResult::Aborted => Self::Aborted,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::data::{
        browsing_context_open::{ChromeBrowsingContextOpenHint, ChromeBrowsingContextOpenResult},
        ids::TabId,
    };

    use super::{TabOpenHint, TabOpenResult};

    #[test]
    fn tab_open_hint_round_trip() {
        assert_eq!(
            TabOpenHint::from(ChromeBrowsingContextOpenHint::CurrentContext),
            TabOpenHint::CurrentTab
        );
        assert_eq!(
            TabOpenHint::NewBackgroundTab.to_browsing_context_open_hint(),
            Some(ChromeBrowsingContextOpenHint::NewBackgroundContext)
        );
        assert_eq!(TabOpenHint::Popup.to_browsing_context_open_hint(), None);
    }

    #[test]
    fn tab_open_result_round_trip() {
        let raw = TabOpenResult::OpenedExistingTab {
            tab_id: TabId::new(42),
        };
        let chrome_result = ChromeBrowsingContextOpenResult::from(raw);
        let round_trip = TabOpenResult::from(chrome_result);

        assert_eq!(round_trip, raw);
    }
}
