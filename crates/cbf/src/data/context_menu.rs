//! Data models for context menu items, icons, and keyboard accelerators.

/// Context menu item kind coming from the backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextMenuItemType {
    Command,
    Check,
    Radio,
    Separator,
    ButtonItem,
    Submenu,
    ActionableSubmenu,
    Highlighted,
    Title,
}

/// Icon payload for a context menu item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextMenuIcon {
    pub png_bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Keyboard accelerator information for a context menu item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextMenuAccelerator {
    pub key_equivalent: String,
    pub modifier_mask: u32,
}

/// A single context menu item tree node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextMenuItem {
    pub r#type: ContextMenuItemType,
    pub command_id: i32,
    pub label: String,
    pub secondary_label: String,
    pub minor_text: String,
    pub accessible_name: String,
    pub enabled: bool,
    pub visible: bool,
    pub checked: bool,
    pub group_id: i32,
    pub is_new_feature: bool,
    pub is_alerted: bool,
    pub may_have_mnemonics: bool,
    pub accelerator: Option<ContextMenuAccelerator>,
    pub icon: Option<ContextMenuIcon>,
    pub minor_icon: Option<ContextMenuIcon>,
    pub submenu: Vec<ContextMenuItem>,
}

/// The context menu tree requested by the backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextMenu {
    pub menu_id: u64,
    pub x: i32,
    pub y: i32,
    pub source_type: u32,
    pub items: Vec<ContextMenuItem>,
}
