use cbf::{data::dialog::BeforeUnloadReason, error::BackendErrorInfo, event::BackendStopReason};

pub type ChromeBackendErrorInfo = BackendErrorInfo;
pub type ChromeBackendStopReason = BackendStopReason;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeBeforeUnloadReason {
    Unknown,
    CloseBrowsingContext,
    Navigate,
    Reload,
    WindowClose,
}

impl From<ChromeBeforeUnloadReason> for BeforeUnloadReason {
    fn from(value: ChromeBeforeUnloadReason) -> Self {
        match value {
            ChromeBeforeUnloadReason::Unknown => Self::Unknown,
            ChromeBeforeUnloadReason::CloseBrowsingContext => Self::CloseBrowsingContext,
            ChromeBeforeUnloadReason::Navigate => Self::Navigate,
            ChromeBeforeUnloadReason::Reload => Self::Reload,
            ChromeBeforeUnloadReason::WindowClose => Self::WindowClose,
        }
    }
}

impl From<BeforeUnloadReason> for ChromeBeforeUnloadReason {
    fn from(value: BeforeUnloadReason) -> Self {
        match value {
            BeforeUnloadReason::Unknown => Self::Unknown,
            BeforeUnloadReason::CloseBrowsingContext => Self::CloseBrowsingContext,
            BeforeUnloadReason::Navigate => Self::Navigate,
            BeforeUnloadReason::Reload => Self::Reload,
            BeforeUnloadReason::WindowClose => Self::WindowClose,
        }
    }
}
