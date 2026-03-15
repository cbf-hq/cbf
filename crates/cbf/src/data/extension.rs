//! Data models for browser extension metadata and install prompt results.

/// Browser-generic icon payload exposed by backends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconData {
    Url(String),
    Png(Vec<u8>),
    Binary {
        media_type: Option<String>,
        bytes: Vec<u8>,
    },
}

/// Extension metadata exposed by backends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub enabled: bool,
    pub permission_names: Vec<String>,
    pub icon: Option<IconData>,
}

/// Result for extension install prompt lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionInstallPromptResult {
    Accepted,
    AcceptedWithWithheldPermissions,
    UserCanceled,
    Aborted,
}

#[cfg(test)]
mod tests {
    use super::{ExtensionInfo, IconData};

    #[test]
    fn extension_info_preserves_png_icon() {
        let info = ExtensionInfo {
            id: "ext".to_string(),
            name: "Example".to_string(),
            version: "1.0.0".to_string(),
            enabled: true,
            permission_names: vec!["tabs".to_string()],
            icon: Some(IconData::Png(vec![1, 2, 3])),
        };

        assert_eq!(info.icon, Some(IconData::Png(vec![1, 2, 3])));
    }
}
