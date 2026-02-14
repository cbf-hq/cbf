#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Mouse event kind delivered to the backend.
pub enum MouseEventType {
    Down,
    Up,
    Move,
    Enter,
    Leave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Mouse button identifier.
pub enum MouseButton {
    None,
    Left,
    Middle,
    Right,
    Back,
    Forward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Pointer device type for mouse-like events.
pub enum PointerType {
    Unknown,
    Mouse,
    Pen,
    Touch,
    Eraser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Scroll granularity reported by the input device.
pub enum ScrollGranularity {
    PrecisePixel,
    Pixel,
    Line,
    Page,
    Document,
}

#[derive(Debug, Clone, PartialEq)]
/// Mouse event payload sent to the backend.
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

#[derive(Debug, Clone, PartialEq)]
/// Mouse wheel event payload sent to the backend.
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
    pub phase: u32,
    pub momentum_phase: u32,
    pub delta_units: ScrollGranularity,
}
