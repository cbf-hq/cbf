use cbf::data::ipc::BrowsingContextIpcMessage;

use super::protocol::{ParseError, ToolbarRequest, parse_request};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HandleError {
    InvalidMessageType,
    InvalidRequestId,
    Parse(ParseError),
}

pub(crate) fn decode_request(
    message: &BrowsingContextIpcMessage,
) -> Result<(u64, ToolbarRequest), HandleError> {
    if !matches!(
        message.message_type,
        cbf::data::ipc::IpcMessageType::Request
    ) {
        return Err(HandleError::InvalidMessageType);
    }
    if message.request_id == 0 {
        return Err(HandleError::InvalidRequestId);
    }
    let request = parse_request(&message.channel, &message.payload).map_err(HandleError::Parse)?;
    Ok((message.request_id, request))
}

pub(crate) fn normalize_url(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "about:blank".to_string();
    }
    if trimmed.starts_with("about:")
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("file://")
    {
        return trimmed.to_string();
    }
    format!("https://{trimmed}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use cbf::data::ipc::{BrowsingContextIpcMessage, IpcMessageType, IpcPayload};

    #[test]
    fn decode_request_rejects_non_request() {
        let message = BrowsingContextIpcMessage {
            channel: "simpleapp.nav.back".to_string(),
            message_type: IpcMessageType::Event,
            request_id: 1,
            payload: IpcPayload::Text("{}".to_string()),
            content_type: None,
            error_code: None,
        };
        let err = decode_request(&message).expect_err("non-request should fail");
        assert_eq!(err, HandleError::InvalidMessageType);
    }

    #[test]
    fn normalize_url_adds_https() {
        assert_eq!(normalize_url("example.com"), "https://example.com");
    }
}
