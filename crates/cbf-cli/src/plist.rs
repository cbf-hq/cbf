use serde::Serialize;

#[derive(Debug, Serialize)]
#[allow(non_snake_case)]
pub struct InfoPlist {
    pub CFBundleDisplayName: String,
    pub CFBundleExecutable: String,
    pub CFBundleIdentifier: String,
    pub CFBundleName: String,
    pub CFBundlePackageType: String,
    pub CFBundleVersion: String,
    pub CFBundleShortVersionString: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub CFBundleIconFile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub LSApplicationCategoryType: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub LSMinimumSystemVersion: Option<String>,
}
