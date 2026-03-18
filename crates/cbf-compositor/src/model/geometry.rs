/// Rectangle in compositor-window coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Left edge of the rectangle.
    pub x: f64,
    /// Top edge of the rectangle.
    pub y: f64,
    /// Rectangle width.
    pub width: f64,
    /// Rectangle height.
    pub height: f64,
}

impl Rect {
    /// Create a new rectangle from origin and size values.
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}
