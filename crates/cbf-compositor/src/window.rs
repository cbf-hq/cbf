//! Host window abstractions used to attach native windows to the compositor.

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::sync::Arc;

/// Narrow host-window abstraction required by the compositor.
pub trait WindowHost: HasWindowHandle + HasDisplayHandle {
    /// Return the current inner size in physical pixels.
    fn inner_size(&self) -> (u32, u32);

    /// Return the current scale factor for coordinate conversion.
    fn scale_factor(&self) -> f64 {
        1.0
    }
}

#[cfg(feature = "winit")]
impl WindowHost for winit::window::Window {
    fn inner_size(&self) -> (u32, u32) {
        let size = self.inner_size();
        (size.width, size.height)
    }

    fn scale_factor(&self) -> f64 {
        self.scale_factor()
    }
}

impl<W> WindowHost for Arc<W>
where
    W: WindowHost,
{
    fn inner_size(&self) -> (u32, u32) {
        (**self).inner_size()
    }

    fn scale_factor(&self) -> f64 {
        (**self).scale_factor()
    }
}
