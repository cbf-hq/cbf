//! Platform-specific rendering surface handle exposed by Chromium (e.g. `CAContext` ID on macOS).

/// Platform-specific handle to the rendering surface exposed by Chromium.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceHandle {
    MacCaContextId(u32),
}
