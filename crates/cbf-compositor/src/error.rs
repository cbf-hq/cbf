use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompositorError {
    #[error("unknown window")]
    UnknownWindow,
    #[error("unknown frame")]
    UnknownFrame,
    #[error("frame is already attached to another window")]
    FrameOwnedByAnotherWindow,
}
