//! Browser-generic IPC data models for browsing context message exchange.

/// Configuration for browsing context IPC.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IpcConfig {
    /// Allowed top-level origins for page -> host invoke.
    ///
    /// The match is exact string equality on origin. If empty, invoke is denied.
    pub allowed_origins: Vec<String>,
}

/// Payload transported through browsing context IPC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcPayload {
    Text(String),
    Binary(Vec<u8>),
}

/// Logical IPC message type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcMessageType {
    Request,
    Response,
    Event,
}

/// Structured IPC error code for invoke failure paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcErrorCode {
    Timeout,
    Aborted,
    Disconnected,
    IpcDisabled,
    ContextClosed,
    RemoteError,
    ProtocolError,
}

/// Browser-generic IPC message envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowsingContextIpcMessage {
    /// Logical channel name used to route IPC messages.
    ///
    /// This must be a non-empty string. Empty channel names are invalid input
    /// and are outside the CBF IPC contract.
    pub channel: String,
    pub message_type: IpcMessageType,
    pub request_id: u64,
    pub payload: IpcPayload,
    pub content_type: Option<String>,
    pub error_code: Option<IpcErrorCode>,
}
