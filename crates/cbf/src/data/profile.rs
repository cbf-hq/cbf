//! Data models for browser profile metadata.

/// Metadata for a browser profile exposed by the backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileInfo {
    pub profile_id: String,
    pub profile_path: String,
    pub display_name: String,
    pub is_default: bool,
}

#[cfg(test)]
mod tests {
    use super::ProfileInfo;

    #[test]
    fn profile_info_equality_covers_is_default() {
        let default_profile = ProfileInfo {
            profile_id: "profile-a".to_string(),
            profile_path: "/tmp/profile-a".to_string(),
            display_name: "Profile A".to_string(),
            is_default: true,
        };
        let non_default_profile = ProfileInfo {
            is_default: false,
            ..default_profile.clone()
        };

        assert_ne!(default_profile, non_default_profile);
    }
}
