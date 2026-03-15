//! Chrome-specific extension info and auxiliary window response types.

use cbf::data::{
    auxiliary_window::AuxiliaryWindowResponse,
    extension::{ExtensionInfo, IconData},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChromeAuxiliaryWindowResponse {
    ExtensionInstallPrompt {
        proceed: bool,
    },
    ExtensionUninstallPrompt {
        proceed: bool,
        report_abuse: bool,
    },
    PermissionPrompt {
        allow: bool,
    },
    DownloadPrompt {
        allow: bool,
        destination_path: Option<String>,
    },
    Unknown,
}

impl From<AuxiliaryWindowResponse> for ChromeAuxiliaryWindowResponse {
    fn from(value: AuxiliaryWindowResponse) -> Self {
        match value {
            AuxiliaryWindowResponse::ExtensionInstallPrompt { proceed } => {
                Self::ExtensionInstallPrompt { proceed }
            }
            AuxiliaryWindowResponse::ExtensionUninstallPrompt {
                proceed,
                report_abuse,
            } => Self::ExtensionUninstallPrompt {
                proceed,
                report_abuse,
            },
            AuxiliaryWindowResponse::PermissionPrompt { allow } => Self::PermissionPrompt { allow },
            AuxiliaryWindowResponse::DownloadPrompt {
                allow,
                destination_path,
            } => Self::DownloadPrompt {
                allow,
                destination_path,
            },
            AuxiliaryWindowResponse::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChromeIconData {
    Url(String),
    Png(Vec<u8>),
    Binary {
        media_type: Option<String>,
        bytes: Vec<u8>,
    },
}

impl From<ChromeIconData> for IconData {
    fn from(value: ChromeIconData) -> Self {
        match value {
            ChromeIconData::Url(url) => Self::Url(url),
            ChromeIconData::Png(bytes) => Self::Png(bytes),
            ChromeIconData::Binary { media_type, bytes } => Self::Binary { media_type, bytes },
        }
    }
}

impl From<IconData> for ChromeIconData {
    fn from(value: IconData) -> Self {
        match value {
            IconData::Url(url) => Self::Url(url),
            IconData::Png(bytes) => Self::Png(bytes),
            IconData::Binary { media_type, bytes } => Self::Binary { media_type, bytes },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeExtensionInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub enabled: bool,
    pub permission_names: Vec<String>,
    pub icon: Option<ChromeIconData>,
}

impl From<ChromeExtensionInfo> for ExtensionInfo {
    fn from(value: ChromeExtensionInfo) -> Self {
        Self {
            id: value.id,
            name: value.name,
            version: value.version,
            enabled: value.enabled,
            permission_names: value.permission_names,
            icon: value.icon.map(Into::into),
        }
    }
}

impl From<ExtensionInfo> for ChromeExtensionInfo {
    fn from(value: ExtensionInfo) -> Self {
        Self {
            id: value.id,
            name: value.name,
            version: value.version,
            enabled: value.enabled,
            permission_names: value.permission_names,
            icon: value.icon.map(Into::into),
        }
    }
}

#[cfg(test)]
mod tests {
    use cbf::data::extension::IconData;

    use super::{ChromeExtensionInfo, ChromeIconData};

    #[test]
    fn converts_png_icon_to_generic() {
        let chrome = ChromeExtensionInfo {
            id: "ext".to_string(),
            name: "Example".to_string(),
            version: "1.0.0".to_string(),
            enabled: true,
            permission_names: vec!["tabs".to_string()],
            icon: Some(ChromeIconData::Png(vec![1, 2, 3])),
        };

        let generic: cbf::data::extension::ExtensionInfo = chrome.into();
        assert_eq!(generic.icon, Some(IconData::Png(vec![1, 2, 3])));
    }

    #[test]
    fn converts_binary_icon_from_generic() {
        let generic = cbf::data::extension::ExtensionInfo {
            id: "ext".to_string(),
            name: "Example".to_string(),
            version: "1.0.0".to_string(),
            enabled: false,
            permission_names: vec![],
            icon: Some(IconData::Binary {
                media_type: Some("image/webp".to_string()),
                bytes: vec![9, 8, 7],
            }),
        };

        let chrome: ChromeExtensionInfo = generic.into();
        assert_eq!(
            chrome.icon,
            Some(ChromeIconData::Binary {
                media_type: Some("image/webp".to_string()),
                bytes: vec![9, 8, 7],
            })
        );
    }
}
