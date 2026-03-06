use cbf::data::browsing_context_open::{
    BrowsingContextOpenHint, BrowsingContextOpenResponse, BrowsingContextOpenResult,
};

use super::ids::TabId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeBrowsingContextOpenHint {
    Unknown,
    CurrentContext,
    NewForegroundContext,
    NewBackgroundContext,
}

impl From<ChromeBrowsingContextOpenHint> for BrowsingContextOpenHint {
    fn from(value: ChromeBrowsingContextOpenHint) -> Self {
        match value {
            ChromeBrowsingContextOpenHint::Unknown => Self::Unknown,
            ChromeBrowsingContextOpenHint::CurrentContext => Self::CurrentContext,
            ChromeBrowsingContextOpenHint::NewForegroundContext => Self::NewForegroundContext,
            ChromeBrowsingContextOpenHint::NewBackgroundContext => Self::NewBackgroundContext,
        }
    }
}

impl From<BrowsingContextOpenHint> for ChromeBrowsingContextOpenHint {
    fn from(value: BrowsingContextOpenHint) -> Self {
        match value {
            BrowsingContextOpenHint::Unknown => Self::Unknown,
            BrowsingContextOpenHint::CurrentContext => Self::CurrentContext,
            BrowsingContextOpenHint::NewForegroundContext => Self::NewForegroundContext,
            BrowsingContextOpenHint::NewBackgroundContext => Self::NewBackgroundContext,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeBrowsingContextOpenResponse {
    AllowNewContext { activate: bool },
    AllowExistingContext { tab_id: TabId, activate: bool },
    Deny,
}

impl From<ChromeBrowsingContextOpenResponse> for BrowsingContextOpenResponse {
    fn from(value: ChromeBrowsingContextOpenResponse) -> Self {
        match value {
            ChromeBrowsingContextOpenResponse::AllowNewContext { activate } => {
                Self::AllowNewContext { activate }
            }
            ChromeBrowsingContextOpenResponse::AllowExistingContext { tab_id, activate } => {
                Self::AllowExistingContext {
                    browsing_context_id: tab_id.into(),
                    activate,
                }
            }
            ChromeBrowsingContextOpenResponse::Deny => Self::Deny,
        }
    }
}

impl From<BrowsingContextOpenResponse> for ChromeBrowsingContextOpenResponse {
    fn from(value: BrowsingContextOpenResponse) -> Self {
        match value {
            BrowsingContextOpenResponse::AllowNewContext { activate } => {
                Self::AllowNewContext { activate }
            }
            BrowsingContextOpenResponse::AllowExistingContext {
                browsing_context_id,
                activate,
            } => Self::AllowExistingContext {
                tab_id: browsing_context_id.into(),
                activate,
            },
            BrowsingContextOpenResponse::Deny => Self::Deny,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeBrowsingContextOpenResult {
    OpenedNewContext { tab_id: TabId },
    OpenedExistingContext { tab_id: TabId },
    Denied,
    Aborted,
}

impl From<ChromeBrowsingContextOpenResult> for BrowsingContextOpenResult {
    fn from(value: ChromeBrowsingContextOpenResult) -> Self {
        match value {
            ChromeBrowsingContextOpenResult::OpenedNewContext { tab_id } => {
                Self::OpenedNewContext {
                    browsing_context_id: tab_id.into(),
                }
            }
            ChromeBrowsingContextOpenResult::OpenedExistingContext { tab_id } => {
                Self::OpenedExistingContext {
                    browsing_context_id: tab_id.into(),
                }
            }
            ChromeBrowsingContextOpenResult::Denied => Self::Denied,
            ChromeBrowsingContextOpenResult::Aborted => Self::Aborted,
        }
    }
}

impl From<BrowsingContextOpenResult> for ChromeBrowsingContextOpenResult {
    fn from(value: BrowsingContextOpenResult) -> Self {
        match value {
            BrowsingContextOpenResult::OpenedNewContext {
                browsing_context_id,
            } => Self::OpenedNewContext {
                tab_id: browsing_context_id.into(),
            },
            BrowsingContextOpenResult::OpenedExistingContext {
                browsing_context_id,
            } => Self::OpenedExistingContext {
                tab_id: browsing_context_id.into(),
            },
            BrowsingContextOpenResult::Denied => Self::Denied,
            BrowsingContextOpenResult::Aborted => Self::Aborted,
        }
    }
}
