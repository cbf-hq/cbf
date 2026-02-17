/// Platform-specific handle to the rendering surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceHandle {
    MacCaContextId(u32),
}
