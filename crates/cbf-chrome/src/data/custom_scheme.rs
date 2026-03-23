//! Chrome-specific custom scheme request/response models.

/// Immutable startup registration for a Chrome-backed custom scheme handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeCustomSchemeRegistration {
    pub scheme: String,
    pub host: String,
}

/// Chromium-observed request method for a custom scheme load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChromeCustomSchemeRequestMethod {
    Get,
    Other(String),
}

impl ChromeCustomSchemeRequestMethod {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Get => "GET",
            Self::Other(value) => value.as_str(),
        }
    }
}

/// A host-routable custom scheme request emitted by the Chrome backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeCustomSchemeRequest {
    pub request_id: u64,
    pub profile_id: String,
    pub url: String,
    pub scheme: String,
    pub host: String,
    pub path: String,
    pub query: Option<String>,
    pub method: ChromeCustomSchemeRequestMethod,
}

/// Host-visible completion result for a custom scheme request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeCustomSchemeResponseResult {
    Ok,
    NotFound,
    Aborted,
}

/// Response body and headers for a custom scheme request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeCustomSchemeResponse {
    pub request_id: u64,
    pub result: ChromeCustomSchemeResponseResult,
    pub body: Vec<u8>,
    pub mime_type: String,
    pub content_security_policy: Option<String>,
    pub access_control_allow_origin: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{
        ChromeCustomSchemeRegistration, ChromeCustomSchemeRequest, ChromeCustomSchemeRequestMethod,
        ChromeCustomSchemeResponse, ChromeCustomSchemeResponseResult,
    };

    #[test]
    fn request_method_as_str_returns_expected_value() {
        assert_eq!(ChromeCustomSchemeRequestMethod::Get.as_str(), "GET");
        assert_eq!(
            ChromeCustomSchemeRequestMethod::Other("HEAD".to_string()).as_str(),
            "HEAD"
        );
    }

    #[test]
    fn custom_scheme_models_are_cloneable() {
        let registration = ChromeCustomSchemeRegistration {
            scheme: "app".to_string(),
            host: "simpleapp".to_string(),
        };
        let request = ChromeCustomSchemeRequest {
            request_id: 1,
            profile_id: "profile".to_string(),
            url: "app://simpleapp/ui.html".to_string(),
            scheme: "app".to_string(),
            host: "simpleapp".to_string(),
            path: "/ui.html".to_string(),
            query: None,
            method: ChromeCustomSchemeRequestMethod::Get,
        };
        let response = ChromeCustomSchemeResponse {
            request_id: 1,
            result: ChromeCustomSchemeResponseResult::Ok,
            body: b"ok".to_vec(),
            mime_type: "text/plain".to_string(),
            content_security_policy: Some("default-src 'self'".to_string()),
            access_control_allow_origin: Some("app://simpleapp".to_string()),
        };

        assert_eq!(registration.clone(), registration);
        assert_eq!(request.clone(), request);
        assert_eq!(response.clone(), response);
    }
}
