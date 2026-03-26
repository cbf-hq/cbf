use cbf::data::ipc::{BrowsingContextIpcMessage, IpcErrorCode, IpcMessageType, IpcPayload};

pub(crate) fn response_success_message(
    channel: &str,
    request_id: u64,
) -> BrowsingContextIpcMessage {
    BrowsingContextIpcMessage {
        channel: channel.to_string(),
        message_type: IpcMessageType::Response,
        request_id,
        payload: IpcPayload::Text("{\"ok\":true}".to_string()),
        content_type: Some("application/json".to_string()),
        error_code: None,
    }
}

pub(crate) fn response_error_message(
    channel: &str,
    request_id: u64,
    code: IpcErrorCode,
    message: &str,
) -> BrowsingContextIpcMessage {
    BrowsingContextIpcMessage {
        channel: channel.to_string(),
        message_type: IpcMessageType::Response,
        request_id,
        payload: IpcPayload::Text(
            serde_json::json!({
                "ok": false,
                "message": message,
            })
            .to_string(),
        ),
        content_type: Some("application/json".to_string()),
        error_code: Some(code),
    }
}
