use cbf::data::ipc::IpcPayload;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) const CHANNEL_NAV_OPEN: &str = "simpleapp.nav.open";
pub(crate) const CHANNEL_NAV_BACK: &str = "simpleapp.nav.back";
pub(crate) const CHANNEL_NAV_FORWARD: &str = "simpleapp.nav.forward";
pub(crate) const CHANNEL_NAV_RELOAD: &str = "simpleapp.nav.reload";
pub(crate) const CHANNEL_STATE_REQUEST: &str = "simpleapp.state.request";
pub(crate) const CHANNEL_STATE_NAVIGATION: &str = "simpleapp.state.navigation";
pub(crate) const CONTENT_TYPE_JSON: &str = "application/json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolbarRequest {
    Open { url: String },
    Back,
    Forward,
    Reload { ignore_cache: bool },
    StateRequest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParseError {
    UnsupportedPayload,
    UnknownChannel,
    InvalidJson,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct NavigationState {
    pub(crate) url: String,
    pub(crate) can_go_back: bool,
    pub(crate) can_go_forward: bool,
    pub(crate) is_loading: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenRequest {
    url: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ReloadRequest {
    #[serde(default)]
    ignore_cache: bool,
}

pub(crate) fn parse_request(
    channel: &str,
    payload: &IpcPayload,
) -> Result<ToolbarRequest, ParseError> {
    let payload_text = match payload {
        IpcPayload::Text(text) => text,
        IpcPayload::Binary(_) => return Err(ParseError::UnsupportedPayload),
    };

    match channel {
        CHANNEL_NAV_OPEN => {
            let open: OpenRequest =
                serde_json::from_str(payload_text).map_err(|_| ParseError::InvalidJson)?;
            Ok(ToolbarRequest::Open { url: open.url })
        }
        CHANNEL_NAV_BACK => {
            parse_empty_object(payload_text)?;
            Ok(ToolbarRequest::Back)
        }
        CHANNEL_NAV_FORWARD => {
            parse_empty_object(payload_text)?;
            Ok(ToolbarRequest::Forward)
        }
        CHANNEL_NAV_RELOAD => {
            let reload: ReloadRequest =
                serde_json::from_str(payload_text).map_err(|_| ParseError::InvalidJson)?;
            Ok(ToolbarRequest::Reload {
                ignore_cache: reload.ignore_cache,
            })
        }
        CHANNEL_STATE_REQUEST => {
            parse_empty_object(payload_text)?;
            Ok(ToolbarRequest::StateRequest)
        }
        _ => Err(ParseError::UnknownChannel),
    }
}

pub(crate) fn to_success_json(value: Value) -> Result<String, serde_json::Error> {
    serde_json::to_string(&serde_json::json!({
        "ok": true,
        "value": value,
    }))
}

pub(crate) fn to_error_json(code: &str, message: &str) -> Result<String, serde_json::Error> {
    serde_json::to_string(&serde_json::json!({
        "ok": false,
        "code": code,
        "message": message,
    }))
}

fn parse_empty_object(payload_text: &str) -> Result<(), ParseError> {
    let value: Value = serde_json::from_str(payload_text).map_err(|_| ParseError::InvalidJson)?;
    match value {
        Value::Object(map) if map.is_empty() => Ok(()),
        _ => Err(ParseError::InvalidJson),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_open_request() {
        let request = parse_request(
            CHANNEL_NAV_OPEN,
            &IpcPayload::Text("{\"url\":\"https://example.com\"}".to_string()),
        )
        .expect("open request should parse");
        assert_eq!(
            request,
            ToolbarRequest::Open {
                url: "https://example.com".to_string()
            }
        );
    }

    #[test]
    fn parse_unknown_channel_is_error() {
        let err = parse_request("simpleapp.unknown", &IpcPayload::Text("{}".to_string()))
            .expect_err("unknown channel should fail");
        assert_eq!(err, ParseError::UnknownChannel);
    }

    #[test]
    fn parse_invalid_json_is_error() {
        let err = parse_request(CHANNEL_NAV_OPEN, &IpcPayload::Text("{".to_string()))
            .expect_err("invalid json should fail");
        assert_eq!(err, ParseError::InvalidJson);
    }
}
