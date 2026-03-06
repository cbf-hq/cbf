use cbf::data::extension::{AuxiliaryWindowResponse, ExtensionInfo};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChromeAuxiliaryWindowResponse {
    ExtensionInstallPrompt { proceed: bool },
    PermissionPrompt { allow: bool },
    Unknown,
}

impl From<AuxiliaryWindowResponse> for ChromeAuxiliaryWindowResponse {
    fn from(value: AuxiliaryWindowResponse) -> Self {
        match value {
            AuxiliaryWindowResponse::ExtensionInstallPrompt { proceed } => {
                Self::ExtensionInstallPrompt { proceed }
            }
            AuxiliaryWindowResponse::PermissionPrompt { allow } => Self::PermissionPrompt { allow },
            AuxiliaryWindowResponse::Unknown => Self::Unknown,
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
}

impl From<ChromeExtensionInfo> for ExtensionInfo {
    fn from(value: ChromeExtensionInfo) -> Self {
        Self {
            id: value.id,
            name: value.name,
            version: value.version,
            enabled: value.enabled,
            permission_names: value.permission_names,
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
        }
    }
}
