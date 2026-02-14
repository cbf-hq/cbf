#[derive(Debug, Clone, PartialEq, Eq)]
/// Platform-specific handle to the rendering surface.
pub enum SurfaceHandle {
    MacCaContextId(u32),
    WindowsHwnd(u64),
}
