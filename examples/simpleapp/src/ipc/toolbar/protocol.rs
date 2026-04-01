use cbf::data::ipc::IpcPayload;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) const CHANNEL_NAV_OPEN: &str = "simpleapp.nav.open";
pub(crate) const CHANNEL_NAV_BACK: &str = "simpleapp.nav.back";
pub(crate) const CHANNEL_NAV_FORWARD: &str = "simpleapp.nav.forward";
pub(crate) const CHANNEL_NAV_RELOAD: &str = "simpleapp.nav.reload";
pub(crate) const CHANNEL_STATE_REQUEST: &str = "simpleapp.state.request";
pub(crate) const CHANNEL_STATE_NAVIGATION: &str = "simpleapp.state.navigation";
pub(crate) const CHANNEL_FIND_SET_QUERY: &str = "simpleapp.find.set_query";
pub(crate) const CHANNEL_FIND_NEXT: &str = "simpleapp.find.next";
pub(crate) const CHANNEL_FIND_PREVIOUS: &str = "simpleapp.find.previous";
pub(crate) const CHANNEL_FIND_CLOSE: &str = "simpleapp.find.close";
pub(crate) const CHANNEL_FIND_STATE: &str = "simpleapp.find.state";
pub(crate) const CONTENT_TYPE_JSON: &str = "application/json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolbarRequest {
    Open { url: String },
    Back,
    Forward,
    Reload { ignore_cache: bool },
    StateRequest,
    FindSetQuery { query: String },
    FindNext,
    FindPrevious,
    FindClose,
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
    pub(crate) favicon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub(crate) struct FindStateSnapshot {
    pub(crate) visible: bool,
    pub(crate) query: String,
    pub(crate) number_of_matches: u32,
    pub(crate) active_match_ordinal: i32,
    pub(crate) pending: bool,
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

#[derive(Debug, Clone, Deserialize)]
struct FindSetQueryRequest {
    #[serde(default)]
    query: String,
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
        CHANNEL_FIND_SET_QUERY => {
            let request: FindSetQueryRequest =
                serde_json::from_str(payload_text).map_err(|_| ParseError::InvalidJson)?;
            Ok(ToolbarRequest::FindSetQuery {
                query: request.query,
            })
        }
        CHANNEL_FIND_NEXT => {
            parse_empty_object(payload_text)?;
            Ok(ToolbarRequest::FindNext)
        }
        CHANNEL_FIND_PREVIOUS => {
            parse_empty_object(payload_text)?;
            Ok(ToolbarRequest::FindPrevious)
        }
        CHANNEL_FIND_CLOSE => {
            parse_empty_object(payload_text)?;
            Ok(ToolbarRequest::FindClose)
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

    #[test]
    fn parse_find_set_query_request() {
        let request = parse_request(
            CHANNEL_FIND_SET_QUERY,
            &IpcPayload::Text("{\"query\":\"needle\"}".to_string()),
        )
        .expect("find request should parse");
        assert_eq!(
            request,
            ToolbarRequest::FindSetQuery {
                query: "needle".to_string()
            }
        );
    }

    #[test]
    fn parse_find_next_request() {
        let request = parse_request(CHANNEL_FIND_NEXT, &IpcPayload::Text("{}".to_string()))
            .expect("find next should parse");
        assert_eq!(request, ToolbarRequest::FindNext);
    }

    #[test]
    fn navigation_state_round_trips_with_favicon_url() {
        let state = NavigationState {
            url: "https://example.com".to_string(),
            can_go_back: true,
            can_go_forward: false,
            is_loading: false,
            favicon_url: Some("https://example.com/favicon.ico".to_string()),
        };

        let json = serde_json::to_string(&state).expect("navigation state should serialize");
        let parsed: NavigationState =
            serde_json::from_str(&json).expect("navigation state should deserialize");
        assert_eq!(parsed, state);
    }
}
