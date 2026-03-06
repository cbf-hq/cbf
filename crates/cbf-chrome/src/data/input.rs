//! Chrome-specific keyboard and mouse-wheel input event types, with conversions to/from `cbf` equivalents.

use cbf::data::{
    key::{KeyEvent, KeyEventType},
    mouse::{MouseWheelEvent, ScrollGranularity},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeKeyEventType {
    RawKeyDown,
    KeyDown,
    KeyUp,
    Char,
}

impl From<KeyEventType> for ChromeKeyEventType {
    fn from(value: KeyEventType) -> Self {
        match value {
            KeyEventType::RawKeyDown => Self::RawKeyDown,
            KeyEventType::KeyDown => Self::KeyDown,
            KeyEventType::KeyUp => Self::KeyUp,
            KeyEventType::Char => Self::Char,
        }
    }
}

impl From<ChromeKeyEventType> for KeyEventType {
    fn from(value: ChromeKeyEventType) -> Self {
        match value {
            ChromeKeyEventType::RawKeyDown => Self::RawKeyDown,
            ChromeKeyEventType::KeyDown => Self::KeyDown,
            ChromeKeyEventType::KeyUp => Self::KeyUp,
            ChromeKeyEventType::Char => Self::Char,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeScrollGranularity {
    PrecisePixel,
    Pixel,
    Line,
    Page,
    Document,
}

impl From<ScrollGranularity> for ChromeScrollGranularity {
    fn from(value: ScrollGranularity) -> Self {
        match value {
            ScrollGranularity::PrecisePixel => Self::PrecisePixel,
            ScrollGranularity::Pixel => Self::Pixel,
            ScrollGranularity::Line => Self::Line,
            ScrollGranularity::Page => Self::Page,
            ScrollGranularity::Document => Self::Document,
        }
    }
}

impl From<ChromeScrollGranularity> for ScrollGranularity {
    fn from(value: ChromeScrollGranularity) -> Self {
        match value {
            ChromeScrollGranularity::PrecisePixel => Self::PrecisePixel,
            ChromeScrollGranularity::Pixel => Self::Pixel,
            ChromeScrollGranularity::Line => Self::Line,
            ChromeScrollGranularity::Page => Self::Page,
            ChromeScrollGranularity::Document => Self::Document,
        }
    }
}

/// Chromium-specific keyboard input payload.
///
/// Field names intentionally match Chromium/bridge vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeKeyEvent {
    pub type_: ChromeKeyEventType,
    pub modifiers: u32,
    pub windows_key_code: i32,
    pub native_key_code: i32,
    pub dom_code: Option<String>,
    pub dom_key: Option<String>,
    pub text: Option<String>,
    pub unmodified_text: Option<String>,
    pub auto_repeat: bool,
    pub is_keypad: bool,
    pub is_system_key: bool,
    pub location: i32,
}

impl From<KeyEvent> for ChromeKeyEvent {
    fn from(value: KeyEvent) -> Self {
        Self {
            type_: value.type_.into(),
            modifiers: value.modifiers,
            windows_key_code: value.key_code,
            native_key_code: value.platform_key_code,
            dom_code: value.dom_code,
            dom_key: value.dom_key,
            text: value.text,
            unmodified_text: value.unmodified_text,
            auto_repeat: value.auto_repeat,
            is_keypad: value.is_keypad,
            is_system_key: value.is_system_key,
            location: value.location,
        }
    }
}

impl From<ChromeKeyEvent> for KeyEvent {
    fn from(value: ChromeKeyEvent) -> Self {
        Self {
            type_: value.type_.into(),
            modifiers: value.modifiers,
            key_code: value.windows_key_code,
            platform_key_code: value.native_key_code,
            dom_code: value.dom_code,
            dom_key: value.dom_key,
            text: value.text,
            unmodified_text: value.unmodified_text,
            auto_repeat: value.auto_repeat,
            is_keypad: value.is_keypad,
            is_system_key: value.is_system_key,
            location: value.location,
        }
    }
}

/// Chromium-specific mouse wheel payload.
///
/// Includes wheel phase fields that are omitted from generic `cbf` input.
#[derive(Debug, Clone, PartialEq)]
pub struct ChromeMouseWheelEvent {
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
    pub delta_units: ChromeScrollGranularity,
}

impl From<MouseWheelEvent> for ChromeMouseWheelEvent {
    fn from(value: MouseWheelEvent) -> Self {
        Self {
            modifiers: value.modifiers,
            position_in_widget_x: value.position_in_widget_x,
            position_in_widget_y: value.position_in_widget_y,
            position_in_screen_x: value.position_in_screen_x,
            position_in_screen_y: value.position_in_screen_y,
            movement_x: value.movement_x,
            movement_y: value.movement_y,
            is_raw_movement_event: value.is_raw_movement_event,
            delta_x: value.delta_x,
            delta_y: value.delta_y,
            wheel_ticks_x: value.wheel_ticks_x,
            wheel_ticks_y: value.wheel_ticks_y,
            phase: 0,
            momentum_phase: 0,
            delta_units: value.delta_units.into(),
        }
    }
}

impl From<ChromeMouseWheelEvent> for MouseWheelEvent {
    fn from(value: ChromeMouseWheelEvent) -> Self {
        Self {
            modifiers: value.modifiers,
            position_in_widget_x: value.position_in_widget_x,
            position_in_widget_y: value.position_in_widget_y,
            position_in_screen_x: value.position_in_screen_x,
            position_in_screen_y: value.position_in_screen_y,
            movement_x: value.movement_x,
            movement_y: value.movement_y,
            is_raw_movement_event: value.is_raw_movement_event,
            delta_x: value.delta_x,
            delta_y: value.delta_y,
            wheel_ticks_x: value.wheel_ticks_x,
            wheel_ticks_y: value.wheel_ticks_y,
            delta_units: value.delta_units.into(),
        }
    }
}
