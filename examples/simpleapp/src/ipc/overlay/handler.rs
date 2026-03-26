use cbf::data::ipc::BrowsingContextIpcMessage;

use super::protocol::{OverlayRequest, ParseError, parse_request};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HandleError {
    InvalidMessageType,
    InvalidRequestId,
    Parse(ParseError),
}

pub(crate) fn decode_request(
    message: &BrowsingContextIpcMessage,
) -> Result<(u64, OverlayRequest), HandleError> {
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
