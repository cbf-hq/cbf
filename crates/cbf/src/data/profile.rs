#[derive(Debug, Clone, PartialEq, Eq)]
/// Metadata for a browser profile exposed by the backend.
pub struct ProfileInfo {
    pub profile_id: String,
    pub profile_path: String,
    pub display_name: String,
}
