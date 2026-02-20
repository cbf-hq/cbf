/// Platform-specific handle to the rendering surface exposed by Chromium.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceHandle {
    MacCaContextId(u32),
}
