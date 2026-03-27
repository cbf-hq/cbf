//! Chrome-specific find-in-page data models.
#![allow(non_upper_case_globals)]

use cbf_chrome_sys::ffi::{
    CbfStopFindAction_kCbfStopFindActionActivateSelection,
    CbfStopFindAction_kCbfStopFindActionClearSelection,
    CbfStopFindAction_kCbfStopFindActionKeepSelection,
};

/// Chromium stop-find action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeStopFindAction {
    ClearSelection,
    KeepSelection,
    ActivateSelection,
}

impl ChromeStopFindAction {
    pub(crate) fn to_ffi(self) -> u8 {
        (match self {
            Self::ClearSelection => CbfStopFindAction_kCbfStopFindActionClearSelection,
            Self::KeepSelection => CbfStopFindAction_kCbfStopFindActionKeepSelection,
            Self::ActivateSelection => CbfStopFindAction_kCbfStopFindActionActivateSelection,
        }) as u8
    }
}

/// Options for a Chromium find-in-page request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeFindInPageOptions {
    pub query: String,
    pub forward: bool,
    pub match_case: bool,
    pub new_session: bool,
    pub find_match: bool,
}

impl ChromeFindInPageOptions {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            forward: true,
            match_case: false,
            new_session: true,
            find_match: true,
        }
    }
}

/// Screen-space selection rect reported by Chromium find replies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChromeFindRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
