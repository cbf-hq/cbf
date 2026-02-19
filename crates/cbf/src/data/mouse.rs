/// Mouse event kind delivered to the backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventType {
    Down,
    Up,
    Move,
    Enter,
    Leave,
}

/// Mouse button identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    None,
    Left,
    Middle,
    Right,
    Back,
    Forward,
}

/// Pointer device type for mouse-like events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerType {
    Unknown,
    Mouse,
    Pen,
    Touch,
    Eraser,
}

/// Scroll granularity reported by the input device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollGranularity {
    PrecisePixel,
    Pixel,
    Line,
    Page,
    Document,
}

/// Mouse event payload sent to the backend.
#[derive(Debug, Clone, PartialEq)]
pub struct MouseEvent {
    pub type_: MouseEventType,
    pub modifiers: u32,
    pub button: MouseButton,
    pub click_count: i32,
    pub position_in_widget_x: f32,
    pub position_in_widget_y: f32,
    pub position_in_screen_x: f32,
    pub position_in_screen_y: f32,
    pub movement_x: f32,
    pub movement_y: f32,
    pub is_raw_movement_event: bool,
    pub pointer_type: PointerType,
}

/// Mouse wheel event payload sent to the backend.
#[derive(Debug, Clone, PartialEq)]
pub struct MouseWheelEvent {
    pub modifiers: u32,
    pub position_in_widget_x: f32,
    pub position_in_widget_y: f32,
    pub position_in_screen_x: f32,
    pub position_in_screen_y: f32,
    pub movement_x: f32,
    pub movement_y: f32,
    pub is_raw_movement_event: bool,
    pub delta_x: f32,
    pub delta_y: f32,
    pub wheel_ticks_x: f32,
    pub wheel_ticks_y: f32,
    pub delta_units: ScrollGranularity,
}
