//! Chrome-specific profile information, with conversions to/from `cbf::data::profile::ProfileInfo`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeProfileInfo {
    pub profile_id: String,
    pub profile_path: String,
    pub display_name: String,
    pub is_default: bool,
}

impl From<ChromeProfileInfo> for cbf::data::profile::ProfileInfo {
    fn from(value: ChromeProfileInfo) -> Self {
        Self {
            profile_id: value.profile_id,
            profile_path: value.profile_path,
            display_name: value.display_name,
            is_default: value.is_default,
        }
    }
}

impl From<cbf::data::profile::ProfileInfo> for ChromeProfileInfo {
    fn from(value: cbf::data::profile::ProfileInfo) -> Self {
        Self {
            profile_id: value.profile_id,
            profile_path: value.profile_path,
            display_name: value.display_name,
            is_default: value.is_default,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ChromeProfileInfo;

    #[test]
    fn round_trip_preserves_is_default() {
        let chrome = ChromeProfileInfo {
            profile_id: "profile-a".to_string(),
            profile_path: "/tmp/profile-a".to_string(),
            display_name: "Profile A".to_string(),
            is_default: true,
        };

        let generic: cbf::data::profile::ProfileInfo = chrome.clone().into();
        let round_tripped: ChromeProfileInfo = generic.into();

        assert_eq!(round_tripped, chrome);
    }
}
