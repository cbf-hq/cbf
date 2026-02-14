#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Keyboard event type as understood by the backend.
pub enum KeyEventType {
    RawKeyDown,
    KeyDown,
    KeyUp,
    Char,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Keyboard input payload sent to the backend.
pub struct KeyEvent {
    pub type_: KeyEventType,
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

impl KeyEvent {
    /// Build a raw key down event.
    pub fn raw_key_down(native_key_code: i32, windows_key_code: i32, modifiers: u32) -> Self {
        Self::new(
            KeyEventType::RawKeyDown,
            native_key_code,
            windows_key_code,
            modifiers,
        )
    }

    /// Build a key down event.
    pub fn key_down(native_key_code: i32, windows_key_code: i32, modifiers: u32) -> Self {
        Self::new(
            KeyEventType::KeyDown,
            native_key_code,
            windows_key_code,
            modifiers,
        )
    }

    /// Build a key up event.
    pub fn key_up(native_key_code: i32, windows_key_code: i32, modifiers: u32) -> Self {
        Self::new(
            KeyEventType::KeyUp,
            native_key_code,
            windows_key_code,
            modifiers,
        )
    }

    /// Build a character input event.
    pub fn char_input(
        native_key_code: i32,
        windows_key_code: i32,
        modifiers: u32,
        text: impl Into<String>,
        unmodified_text: impl Into<String>,
    ) -> Self {
        Self {
            text: Some(text.into()),
            unmodified_text: Some(unmodified_text.into()),
            ..Self::new(
                KeyEventType::Char,
                native_key_code,
                windows_key_code,
                modifiers,
            )
        }
    }

    fn new(
        type_: KeyEventType,
        native_key_code: i32,
        windows_key_code: i32,
        modifiers: u32,
    ) -> Self {
        Self {
            type_,
            modifiers,
            windows_key_code,
            native_key_code,
            dom_code: None,
            dom_key: None,
            text: None,
            unmodified_text: None,
            auto_repeat: false,
            is_keypad: false,
            is_system_key: false,
            location: 0,
        }
    }
}
