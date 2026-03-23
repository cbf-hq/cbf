use cbf_chrome::data::custom_scheme::{
    ChromeCustomSchemeRequest, ChromeCustomSchemeRequestMethod, ChromeCustomSchemeResponse,
    ChromeCustomSchemeResponseResult,
};
use rust_embed::RustEmbed;

pub(crate) const APP_SCHEME: &str = "app";
pub(crate) const APP_HOST: &str = "simpleapp";
pub(crate) const APP_ORIGIN: &str = "app://simpleapp";

const DEFAULT_CONTENT_SECURITY_POLICY: &str = concat!(
    "default-src 'self'; ",
    "script-src 'self' 'unsafe-inline'; ",
    "style-src 'self' 'unsafe-inline'; ",
    "img-src 'self' data:; ",
    "connect-src 'self'; ",
    "object-src 'none'; ",
    "base-uri 'none'; ",
    "frame-ancestors 'none'"
);

#[derive(RustEmbed)]
#[folder = "src/"]
#[include = "ui.html"]
#[include = "overlay.html"]
struct EmbeddedAssets;

pub(crate) fn respond_to_request(
    request: &ChromeCustomSchemeRequest,
) -> ChromeCustomSchemeResponse {
    if request.scheme != APP_SCHEME || request.host != APP_HOST {
        return aborted_response(request.request_id);
    }

    if !matches!(request.method, ChromeCustomSchemeRequestMethod::Get) {
        return aborted_response(request.request_id);
    }

    let Some(asset_path) = asset_path_from_request_path(&request.path) else {
        return not_found_response(request.request_id, request.path.as_str());
    };

    let Some(asset) = EmbeddedAssets::get(asset_path) else {
        return not_found_response(request.request_id, asset_path);
    };

    ChromeCustomSchemeResponse {
        request_id: request.request_id,
        result: ChromeCustomSchemeResponseResult::Ok,
        body: asset.data.into_owned(),
        mime_type: mime_type_for_path(asset_path).to_string(),
        content_security_policy: Some(DEFAULT_CONTENT_SECURITY_POLICY.to_string()),
        access_control_allow_origin: Some(APP_ORIGIN.to_string()),
    }
}

fn aborted_response(request_id: u64) -> ChromeCustomSchemeResponse {
    ChromeCustomSchemeResponse {
        request_id,
        result: ChromeCustomSchemeResponseResult::Aborted,
        body: Vec::new(),
        mime_type: "text/plain; charset=utf-8".to_string(),
        content_security_policy: Some(DEFAULT_CONTENT_SECURITY_POLICY.to_string()),
        access_control_allow_origin: Some(APP_ORIGIN.to_string()),
    }
}

fn not_found_response(request_id: u64, path: &str) -> ChromeCustomSchemeResponse {
    ChromeCustomSchemeResponse {
        request_id,
        result: ChromeCustomSchemeResponseResult::NotFound,
        body: format!("Not found: {path}").into_bytes(),
        mime_type: "text/plain; charset=utf-8".to_string(),
        content_security_policy: Some(DEFAULT_CONTENT_SECURITY_POLICY.to_string()),
        access_control_allow_origin: Some(APP_ORIGIN.to_string()),
    }
}

fn asset_path_from_request_path(path: &str) -> Option<&str> {
    match path.trim_start_matches('/') {
        "" => Some("ui.html"),
        asset_path if !asset_path.contains('\\') => Some(asset_path),
        _ => None,
    }
}

fn mime_type_for_path(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, extension)| extension) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        APP_HOST, APP_ORIGIN, APP_SCHEME, DEFAULT_CONTENT_SECURITY_POLICY, respond_to_request,
    };
    use cbf_chrome::data::custom_scheme::{
        ChromeCustomSchemeRequest, ChromeCustomSchemeRequestMethod,
        ChromeCustomSchemeResponseResult,
    };

    fn request(path: &str) -> ChromeCustomSchemeRequest {
        ChromeCustomSchemeRequest {
            request_id: 7,
            profile_id: "profile".to_string(),
            url: format!("{APP_ORIGIN}{path}"),
            scheme: APP_SCHEME.to_string(),
            host: APP_HOST.to_string(),
            path: path.to_string(),
            query: None,
            method: ChromeCustomSchemeRequestMethod::Get,
        }
    }

    #[test]
    fn embedded_ui_request_returns_html_response() {
        let response = respond_to_request(&request("/ui.html"));

        assert_eq!(response.result, ChromeCustomSchemeResponseResult::Ok);
        assert_eq!(response.mime_type, "text/html; charset=utf-8");
        assert_eq!(
            response.content_security_policy.as_deref(),
            Some(DEFAULT_CONTENT_SECURITY_POLICY)
        );
        assert_eq!(
            response.access_control_allow_origin.as_deref(),
            Some(APP_ORIGIN)
        );
        assert!(!response.body.is_empty());
    }

    #[test]
    fn missing_asset_returns_not_found_response() {
        let response = respond_to_request(&request("/missing.txt"));

        assert_eq!(response.result, ChromeCustomSchemeResponseResult::NotFound);
        assert_eq!(response.mime_type, "text/plain; charset=utf-8");
    }

    #[test]
    fn non_get_requests_are_aborted() {
        let mut request = request("/ui.html");
        request.method = ChromeCustomSchemeRequestMethod::Other("POST".to_string());

        let response = respond_to_request(&request);

        assert_eq!(response.result, ChromeCustomSchemeResponseResult::Aborted);
    }
}
