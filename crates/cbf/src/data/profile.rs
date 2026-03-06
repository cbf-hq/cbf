//! Data models for browser profile metadata.

/// Metadata for a browser profile exposed by the backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileInfo {
    pub profile_id: String,
    pub profile_path: String,
    pub display_name: String,
}
