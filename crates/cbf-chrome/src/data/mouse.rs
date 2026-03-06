#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeMouseEventType {
    Down,
    Up,
    Move,
    Enter,
    Leave,
}

impl From<ChromeMouseEventType> for cbf::data::mouse::MouseEventType {
    fn from(value: ChromeMouseEventType) -> Self {
        match value {
            ChromeMouseEventType::Down => Self::Down,
            ChromeMouseEventType::Up => Self::Up,
            ChromeMouseEventType::Move => Self::Move,
            ChromeMouseEventType::Enter => Self::Enter,
            ChromeMouseEventType::Leave => Self::Leave,
        }
    }
}

impl From<cbf::data::mouse::MouseEventType> for ChromeMouseEventType {
    fn from(value: cbf::data::mouse::MouseEventType) -> Self {
        match value {
            cbf::data::mouse::MouseEventType::Down => Self::Down,
            cbf::data::mouse::MouseEventType::Up => Self::Up,
            cbf::data::mouse::MouseEventType::Move => Self::Move,
            cbf::data::mouse::MouseEventType::Enter => Self::Enter,
            cbf::data::mouse::MouseEventType::Leave => Self::Leave,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeMouseButton {
    None,
    Left,
    Middle,
    Right,
    Back,
    Forward,
}

impl From<ChromeMouseButton> for cbf::data::mouse::MouseButton {
    fn from(value: ChromeMouseButton) -> Self {
        match value {
            ChromeMouseButton::None => Self::None,
            ChromeMouseButton::Left => Self::Left,
            ChromeMouseButton::Middle => Self::Middle,
            ChromeMouseButton::Right => Self::Right,
            ChromeMouseButton::Back => Self::Back,
            ChromeMouseButton::Forward => Self::Forward,
        }
    }
}

impl From<cbf::data::mouse::MouseButton> for ChromeMouseButton {
    fn from(value: cbf::data::mouse::MouseButton) -> Self {
        match value {
            cbf::data::mouse::MouseButton::None => Self::None,
            cbf::data::mouse::MouseButton::Left => Self::Left,
            cbf::data::mouse::MouseButton::Middle => Self::Middle,
            cbf::data::mouse::MouseButton::Right => Self::Right,
            cbf::data::mouse::MouseButton::Back => Self::Back,
            cbf::data::mouse::MouseButton::Forward => Self::Forward,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromePointerType {
    Unknown,
    Mouse,
    Pen,
    Touch,
    Eraser,
}

impl From<ChromePointerType> for cbf::data::mouse::PointerType {
    fn from(value: ChromePointerType) -> Self {
        match value {
            ChromePointerType::Unknown => Self::Unknown,
            ChromePointerType::Mouse => Self::Mouse,
            ChromePointerType::Pen => Self::Pen,
            ChromePointerType::Touch => Self::Touch,
            ChromePointerType::Eraser => Self::Eraser,
        }
    }
}

impl From<cbf::data::mouse::PointerType> for ChromePointerType {
    fn from(value: cbf::data::mouse::PointerType) -> Self {
        match value {
            cbf::data::mouse::PointerType::Unknown => Self::Unknown,
            cbf::data::mouse::PointerType::Mouse => Self::Mouse,
            cbf::data::mouse::PointerType::Pen => Self::Pen,
            cbf::data::mouse::PointerType::Touch => Self::Touch,
            cbf::data::mouse::PointerType::Eraser => Self::Eraser,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChromeMouseEvent {
    pub type_: ChromeMouseEventType,
    pub modifiers: u32,
    pub button: ChromeMouseButton,
    pub click_count: i32,
    pub position_in_widget_x: f32,
    pub position_in_widget_y: f32,
    pub position_in_screen_x: f32,
    pub position_in_screen_y: f32,
    pub movement_x: f32,
    pub movement_y: f32,
    pub is_raw_movement_event: bool,
    pub pointer_type: ChromePointerType,
}

impl From<ChromeMouseEvent> for cbf::data::mouse::MouseEvent {
    fn from(value: ChromeMouseEvent) -> Self {
        Self {
            type_: value.type_.into(),
            modifiers: value.modifiers,
            button: value.button.into(),
            click_count: value.click_count,
            position_in_widget_x: value.position_in_widget_x,
            position_in_widget_y: value.position_in_widget_y,
            position_in_screen_x: value.position_in_screen_x,
            position_in_screen_y: value.position_in_screen_y,
            movement_x: value.movement_x,
            movement_y: value.movement_y,
            is_raw_movement_event: value.is_raw_movement_event,
            pointer_type: value.pointer_type.into(),
        }
    }
}

impl From<cbf::data::mouse::MouseEvent> for ChromeMouseEvent {
    fn from(value: cbf::data::mouse::MouseEvent) -> Self {
        Self {
            type_: value.type_.into(),
            modifiers: value.modifiers,
            button: value.button.into(),
            click_count: value.click_count,
            position_in_widget_x: value.position_in_widget_x,
            position_in_widget_y: value.position_in_widget_y,
            position_in_screen_x: value.position_in_screen_x,
            position_in_screen_y: value.position_in_screen_y,
            movement_x: value.movement_x,
            movement_y: value.movement_y,
            is_raw_movement_event: value.is_raw_movement_event,
            pointer_type: value.pointer_type.into(),
        }
    }
}
