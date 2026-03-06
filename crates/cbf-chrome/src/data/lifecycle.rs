use cbf::{
    error::BackendErrorInfo,
    event::{BackendStopReason, BeforeUnloadReason},
};

pub type ChromeBackendErrorInfo = BackendErrorInfo;
pub type ChromeBackendStopReason = BackendStopReason;
pub type ChromeBeforeUnloadReason = BeforeUnloadReason;
