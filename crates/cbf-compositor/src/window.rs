use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub trait WindowHost: HasWindowHandle + HasDisplayHandle {
    fn inner_size(&self) -> (u32, u32);

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
