//! Permission-related data models for browser permission requests.

/// Permission categories that may be requested by a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionType {
    VideoCapture,
    AudioCapture,
    Notifications,
    Geolocation,
    // Extend as needed.
}
