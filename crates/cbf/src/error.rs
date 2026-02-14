/// Errors returned by the `cbf` public API.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The backend disconnected or the command channel was closed.
    #[error("cbf backend disconnected")]
    Disconnected,
    /// The command queue is full and cannot accept new commands.
    #[error("cbf command queue full")]
    QueueFull,

    /// Failed to spawn a backend process.
    #[error("cbf process spawn error: {0}")]
    ProcessSpawnError(#[from] std::io::Error),

    /// A backend-specific error with a message.
    #[error("cbf backend error: {message}")]
    Backend { message: String },
}
