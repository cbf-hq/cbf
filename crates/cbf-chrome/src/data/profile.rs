#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeProfileInfo {
    pub profile_id: String,
    pub profile_path: String,
    pub display_name: String,
}

impl From<ChromeProfileInfo> for cbf::data::profile::ProfileInfo {
    fn from(value: ChromeProfileInfo) -> Self {
        Self {
            profile_id: value.profile_id,
            profile_path: value.profile_path,
            display_name: value.display_name,
        }
    }
}

impl From<cbf::data::profile::ProfileInfo> for ChromeProfileInfo {
    fn from(value: cbf::data::profile::ProfileInfo) -> Self {
        Self {
            profile_id: value.profile_id,
            profile_path: value.profile_path,
            display_name: value.display_name,
        }
    }
}
