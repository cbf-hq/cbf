//! Chrome transport IPC data models and conversions.

/// Chromium-facing IPC config.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TabIpcConfig {
    /// Allowed top-level origins for page -> host invoke.
    ///
    /// The match is exact string equality on origin. If empty, invoke is denied.
    pub allowed_origins: Vec<String>,
}

impl From<cbf::data::ipc::IpcConfig> for TabIpcConfig {
    fn from(value: cbf::data::ipc::IpcConfig) -> Self {
        Self {
            allowed_origins: value.allowed_origins,
        }
    }
}

impl From<TabIpcConfig> for cbf::data::ipc::IpcConfig {
    fn from(value: TabIpcConfig) -> Self {
        Self {
            allowed_origins: value.allowed_origins,
        }
    }
}

/// Chromium-facing IPC payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabIpcPayload {
    Text(String),
    Binary(Vec<u8>),
}

impl From<cbf::data::ipc::IpcPayload> for TabIpcPayload {
    fn from(value: cbf::data::ipc::IpcPayload) -> Self {
        match value {
            cbf::data::ipc::IpcPayload::Text(text) => Self::Text(text),
            cbf::data::ipc::IpcPayload::Binary(binary) => Self::Binary(binary),
        }
    }
}

impl From<TabIpcPayload> for cbf::data::ipc::IpcPayload {
    fn from(value: TabIpcPayload) -> Self {
        match value {
            TabIpcPayload::Text(text) => Self::Text(text),
            TabIpcPayload::Binary(binary) => Self::Binary(binary),
        }
    }
}

/// Chromium-facing IPC message type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabIpcMessageType {
    Request,
    Response,
    Event,
}

impl From<cbf::data::ipc::IpcMessageType> for TabIpcMessageType {
    fn from(value: cbf::data::ipc::IpcMessageType) -> Self {
        match value {
            cbf::data::ipc::IpcMessageType::Request => Self::Request,
            cbf::data::ipc::IpcMessageType::Response => Self::Response,
            cbf::data::ipc::IpcMessageType::Event => Self::Event,
        }
    }
}

impl From<TabIpcMessageType> for cbf::data::ipc::IpcMessageType {
    fn from(value: TabIpcMessageType) -> Self {
        match value {
            TabIpcMessageType::Request => Self::Request,
            TabIpcMessageType::Response => Self::Response,
            TabIpcMessageType::Event => Self::Event,
        }
    }
}

/// Chromium-facing IPC error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabIpcErrorCode {
    Timeout,
    Aborted,
    Disconnected,
    IpcDisabled,
    ContextClosed,
    RemoteError,
    ProtocolError,
}

impl From<cbf::data::ipc::IpcErrorCode> for TabIpcErrorCode {
    fn from(value: cbf::data::ipc::IpcErrorCode) -> Self {
        match value {
            cbf::data::ipc::IpcErrorCode::Timeout => Self::Timeout,
            cbf::data::ipc::IpcErrorCode::Aborted => Self::Aborted,
            cbf::data::ipc::IpcErrorCode::Disconnected => Self::Disconnected,
            cbf::data::ipc::IpcErrorCode::IpcDisabled => Self::IpcDisabled,
            cbf::data::ipc::IpcErrorCode::ContextClosed => Self::ContextClosed,
            cbf::data::ipc::IpcErrorCode::RemoteError => Self::RemoteError,
            cbf::data::ipc::IpcErrorCode::ProtocolError => Self::ProtocolError,
        }
    }
}

impl From<TabIpcErrorCode> for cbf::data::ipc::IpcErrorCode {
    fn from(value: TabIpcErrorCode) -> Self {
        match value {
            TabIpcErrorCode::Timeout => Self::Timeout,
            TabIpcErrorCode::Aborted => Self::Aborted,
            TabIpcErrorCode::Disconnected => Self::Disconnected,
            TabIpcErrorCode::IpcDisabled => Self::IpcDisabled,
            TabIpcErrorCode::ContextClosed => Self::ContextClosed,
            TabIpcErrorCode::RemoteError => Self::RemoteError,
            TabIpcErrorCode::ProtocolError => Self::ProtocolError,
        }
    }
}

/// Chromium-facing tab IPC message envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabIpcMessage {
    pub channel: String,
    pub message_type: TabIpcMessageType,
    pub request_id: u64,
    pub payload: TabIpcPayload,
    pub content_type: Option<String>,
    pub error_code: Option<TabIpcErrorCode>,
}

impl From<cbf::data::ipc::BrowsingContextIpcMessage> for TabIpcMessage {
    fn from(value: cbf::data::ipc::BrowsingContextIpcMessage) -> Self {
        Self {
            channel: value.channel,
            message_type: value.message_type.into(),
            request_id: value.request_id,
            payload: value.payload.into(),
            content_type: value.content_type,
            error_code: value.error_code.map(Into::into),
        }
    }
}

impl From<TabIpcMessage> for cbf::data::ipc::BrowsingContextIpcMessage {
    fn from(value: TabIpcMessage) -> Self {
        Self {
            channel: value.channel,
            message_type: value.message_type.into(),
            request_id: value.request_id,
            payload: value.payload.into(),
            content_type: value.content_type,
            error_code: value.error_code.map(Into::into),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TabIpcConfig, TabIpcErrorCode, TabIpcMessage, TabIpcMessageType, TabIpcPayload};

    #[test]
    fn tab_ipc_config_round_trip_with_generic() {
        let config = TabIpcConfig {
            allowed_origins: vec!["https://example.com".to_string()],
        };

        let generic: cbf::data::ipc::IpcConfig = config.clone().into();
        let round_trip = TabIpcConfig::from(generic);

        assert_eq!(round_trip, config);
    }

    #[test]
    fn tab_ipc_message_round_trip_with_generic() {
        let message = TabIpcMessage {
            channel: "app.rpc".to_string(),
            message_type: TabIpcMessageType::Response,
            request_id: 77,
            payload: TabIpcPayload::Binary(vec![1, 2, 3]),
            content_type: Some("application/octet-stream".to_string()),
            error_code: Some(TabIpcErrorCode::RemoteError),
        };

        let generic: cbf::data::ipc::BrowsingContextIpcMessage = message.clone().into();
        let round_trip = TabIpcMessage::from(generic);

        assert_eq!(round_trip, message);
    }
}
