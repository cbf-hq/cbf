use cbf::data::ipc::{BrowsingContextIpcMessage, IpcErrorCode, IpcMessageType, IpcPayload};

use super::protocol::{
    CHANNEL_FIND_STATE, CHANNEL_STATE_NAVIGATION, CONTENT_TYPE_JSON, FindStateSnapshot,
    NavigationState, to_error_json, to_success_json,
};

pub(crate) fn response_success_message(
    channel: &str,
    request_id: u64,
    payload: serde_json::Value,
) -> BrowsingContextIpcMessage {
    let payload_text =
        to_success_json(payload).unwrap_or_else(|_| "{\"ok\":true,\"value\":null}".to_string());
    BrowsingContextIpcMessage {
        channel: channel.to_string(),
        message_type: IpcMessageType::Response,
        request_id,
        payload: IpcPayload::Text(payload_text),
        content_type: Some(CONTENT_TYPE_JSON.to_string()),
        error_code: None,
    }
}

pub(crate) fn response_error_message(
    channel: &str,
    request_id: u64,
    error_code: IpcErrorCode,
    code: &str,
    message: &str,
) -> BrowsingContextIpcMessage {
    let payload_text = to_error_json(code, message).unwrap_or_else(|_| {
        format!(
            "{{\"ok\":false,\"code\":\"{}\",\"message\":\"{}\"}}",
            code, message
        )
    });
    BrowsingContextIpcMessage {
        channel: channel.to_string(),
        message_type: IpcMessageType::Response,
        request_id,
        payload: IpcPayload::Text(payload_text),
        content_type: Some(CONTENT_TYPE_JSON.to_string()),
        error_code: Some(error_code),
    }
}

pub(crate) fn navigation_state_event_message(state: &NavigationState) -> BrowsingContextIpcMessage {
    let payload_text = serde_json::to_string(state).unwrap_or_else(|_| {
        "{\"url\":\"\",\"can_go_back\":false,\"can_go_forward\":false,\"is_loading\":false,\"favicon_url\":null}".to_string()
    });

    BrowsingContextIpcMessage {
        channel: CHANNEL_STATE_NAVIGATION.to_string(),
        message_type: IpcMessageType::Event,
        request_id: 0,
        payload: IpcPayload::Text(payload_text),
        content_type: Some(CONTENT_TYPE_JSON.to_string()),
        error_code: None,
    }
}

pub(crate) fn find_state_event_message(
    state: &FindStateSnapshot,
    request_id: u64,
) -> BrowsingContextIpcMessage {
    let payload_text = serde_json::to_string(state).unwrap_or_else(|_| {
        "{\"visible\":false,\"query\":\"\",\"number_of_matches\":0,\"active_match_ordinal\":0,\"pending\":false}"
            .to_string()
    });

    BrowsingContextIpcMessage {
        channel: CHANNEL_FIND_STATE.to_string(),
        message_type: IpcMessageType::Event,
        request_id,
        payload: IpcPayload::Text(payload_text),
        content_type: Some(CONTENT_TYPE_JSON.to_string()),
        error_code: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_event_message_has_expected_shape() {
        let state = NavigationState {
            url: "https://example.com".to_string(),
            can_go_back: true,
            can_go_forward: false,
            is_loading: false,
            favicon_url: Some("https://example.com/favicon.ico".to_string()),
        };

        let message = navigation_state_event_message(&state);
        assert_eq!(message.channel, CHANNEL_STATE_NAVIGATION);
        assert!(matches!(message.message_type, IpcMessageType::Event));
        match message.payload {
            IpcPayload::Text(text) => {
                let parsed: NavigationState =
                    serde_json::from_str(&text).expect("payload should parse");
                assert_eq!(parsed.url, "https://example.com");
                assert!(parsed.can_go_back);
                assert_eq!(
                    parsed.favicon_url.as_deref(),
                    Some("https://example.com/favicon.ico")
                );
            }
            IpcPayload::Binary(_) => panic!("expected text payload"),
        }
    }

    #[test]
    fn response_message_keeps_channel() {
        let ok = response_success_message("simpleapp.nav.open", 1, serde_json::json!({}));
        assert_eq!(ok.channel, "simpleapp.nav.open");

        let err = response_error_message(
            "simpleapp.nav.open",
            1,
            IpcErrorCode::ProtocolError,
            "PROTOCOL_ERROR",
            "bad",
        );
        assert_eq!(err.channel, "simpleapp.nav.open");
    }

    #[test]
    fn find_state_event_message_has_expected_shape() {
        let state = FindStateSnapshot {
            visible: true,
            query: "needle".to_string(),
            number_of_matches: 3,
            active_match_ordinal: 2,
            pending: false,
        };

        let message = find_state_event_message(&state, 42);
        assert_eq!(message.channel, CHANNEL_FIND_STATE);
        assert_eq!(message.request_id, 42);
        match message.payload {
            IpcPayload::Text(text) => {
                let parsed: FindStateSnapshot =
                    serde_json::from_str(&text).expect("payload should parse");
                assert_eq!(parsed, state);
            }
            IpcPayload::Binary(_) => panic!("expected text payload"),
        }
    }
}
