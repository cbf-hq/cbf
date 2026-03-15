//! Chrome/Blink-shaped choice menu payload for host-owned `<select>` popups.

use cbf_chrome_sys::ffi::{
    CBF_CHOICE_MENU_ITEM_CHECKABLE_OPTION, CBF_CHOICE_MENU_ITEM_GROUP, CBF_CHOICE_MENU_ITEM_OPTION,
    CBF_CHOICE_MENU_ITEM_SEPARATOR, CBF_CHOICE_MENU_ITEM_SUB_MENU,
    CBF_CHOICE_MENU_TEXT_DIRECTION_LEFT_TO_RIGHT, CBF_CHOICE_MENU_TEXT_DIRECTION_RIGHT_TO_LEFT,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeChoiceMenuItemType {
    Option,
    CheckableOption,
    Group,
    Separator,
    SubMenu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeChoiceMenuTextDirection {
    LeftToRight,
    RightToLeft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeChoiceMenuSelectionMode {
    Single,
    Multiple,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeChoiceMenuItem {
    pub item_type: ChromeChoiceMenuItemType,
    pub label: Option<String>,
    pub tool_tip: Option<String>,
    pub action: u32,
    pub text_direction: ChromeChoiceMenuTextDirection,
    pub has_text_direction_override: bool,
    pub enabled: bool,
    pub checked: bool,
    pub children: Vec<ChromeChoiceMenuItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChromeChoiceMenu {
    pub request_id: u64,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub item_font_size: f64,
    pub selected_item: i32,
    pub right_aligned: bool,
    pub selection_mode: ChromeChoiceMenuSelectionMode,
    pub items: Vec<ChromeChoiceMenuItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeChoiceMenuResponse {
    pub request_id: u64,
    pub indices: Vec<i32>,
}

pub(crate) fn choice_menu_item_type_from_ffi(value: u8) -> ChromeChoiceMenuItemType {
    match value {
        CBF_CHOICE_MENU_ITEM_OPTION => ChromeChoiceMenuItemType::Option,
        CBF_CHOICE_MENU_ITEM_CHECKABLE_OPTION => ChromeChoiceMenuItemType::CheckableOption,
        CBF_CHOICE_MENU_ITEM_GROUP => ChromeChoiceMenuItemType::Group,
        CBF_CHOICE_MENU_ITEM_SEPARATOR => ChromeChoiceMenuItemType::Separator,
        CBF_CHOICE_MENU_ITEM_SUB_MENU => ChromeChoiceMenuItemType::SubMenu,
        _ => ChromeChoiceMenuItemType::Option,
    }
}

pub(crate) fn choice_menu_text_direction_from_ffi(value: u8) -> ChromeChoiceMenuTextDirection {
    match value {
        CBF_CHOICE_MENU_TEXT_DIRECTION_RIGHT_TO_LEFT => ChromeChoiceMenuTextDirection::RightToLeft,
        CBF_CHOICE_MENU_TEXT_DIRECTION_LEFT_TO_RIGHT => ChromeChoiceMenuTextDirection::LeftToRight,
        _ => ChromeChoiceMenuTextDirection::LeftToRight,
    }
}
