#[derive(Debug, Clone, PartialEq, Eq)]
/// Context menu item kind coming from the backend.
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

#[derive(Debug, Clone, PartialEq, Eq)]
/// Icon payload for a context menu item.
pub struct ContextMenuIcon {
    pub png_bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Keyboard accelerator information for a context menu item.
pub struct ContextMenuAccelerator {
    pub key_equivalent: String,
    pub modifier_mask: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A single context menu item tree node.
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

#[derive(Debug, Clone, PartialEq, Eq)]
/// The context menu tree requested by the backend.
pub struct ContextMenu {
    pub menu_id: u64,
    pub x: i32,
    pub y: i32,
    pub source_type: u32,
    pub items: Vec<ContextMenuItem>,
}

/// Command id for navigating back.
pub const CMD_BACK: i32 = 33000;
/// Command id for navigating forward.
pub const CMD_FORWARD: i32 = 33001;
/// Command id for reloading the page.
pub const CMD_RELOAD: i32 = 33002;
/// Command id for cutting selection.
pub const CMD_CUT: i32 = 36000;
/// Command id for copying selection.
pub const CMD_COPY: i32 = 36001;
/// Command id for pasting clipboard contents.
pub const CMD_PASTE: i32 = 36003;
/// Command id for opening a link in a new tab.
pub const CMD_CONTENT_OPEN_LINK_NEW_TAB: i32 = 50100;
/// Command id for opening a link in a new window.
pub const CMD_CONTENT_OPEN_LINK_NEW_WINDOW: i32 = 50101;
/// Command id for copying a link location.
pub const CMD_CONTENT_COPY_LINK_LOCATION: i32 = 50104;
/// Command id for copying an image location.
pub const CMD_CONTENT_COPY_IMAGE_LOCATION: i32 = 50121;
/// Command id for copying an image.
pub const CMD_CONTENT_COPY_IMAGE: i32 = 50122;
/// Command id for copying selected content.
pub const CMD_CONTENT_COPY: i32 = 50150;
/// Command id for cutting selected content.
pub const CMD_CONTENT_CUT: i32 = 50151;
/// Command id for pasting into content.
pub const CMD_CONTENT_PASTE: i32 = 50152;
/// Command id for undo in editable content.
pub const CMD_CONTENT_UNDO: i32 = 50154;
/// Command id for redo in editable content.
pub const CMD_CONTENT_REDO: i32 = 50155;
/// Command id for selecting all content.
pub const CMD_CONTENT_SELECT_ALL: i32 = 50156;
/// Command id for pasting while matching style.
pub const CMD_CONTENT_PASTE_AND_MATCH_STYLE: i32 = 50157;

const CONTEXT_MENU_ALLOWLIST: &[i32] = &[
    CMD_BACK,
    CMD_FORWARD,
    CMD_RELOAD,
    CMD_CUT,
    CMD_COPY,
    CMD_PASTE,
    CMD_CONTENT_OPEN_LINK_NEW_TAB,
    CMD_CONTENT_OPEN_LINK_NEW_WINDOW,
    CMD_CONTENT_COPY_LINK_LOCATION,
    CMD_CONTENT_COPY_IMAGE_LOCATION,
    CMD_CONTENT_COPY_IMAGE,
    CMD_CONTENT_COPY,
    CMD_CONTENT_CUT,
    CMD_CONTENT_PASTE,
    CMD_CONTENT_UNDO,
    CMD_CONTENT_REDO,
    CMD_CONTENT_SELECT_ALL,
    CMD_CONTENT_PASTE_AND_MATCH_STYLE,
];

pub fn filter_supported(menu: ContextMenu) -> ContextMenu {
    let items = filter_items(menu.items);
    ContextMenu { items, ..menu }
}

/// Check whether a command id represents "open link in new tab".
pub fn is_open_link_new_tab(command_id: i32) -> bool {
    command_id == CMD_CONTENT_OPEN_LINK_NEW_TAB
}

/// Check whether a command id represents "open link in new window".
pub fn is_open_link_new_window(command_id: i32) -> bool {
    command_id == CMD_CONTENT_OPEN_LINK_NEW_WINDOW
}

fn filter_items(items: Vec<ContextMenuItem>) -> Vec<ContextMenuItem> {
    let mut filtered = Vec::new();

    for mut item in items {
        match item.r#type {
            ContextMenuItemType::Separator => filtered.push(item),
            ContextMenuItemType::Submenu | ContextMenuItemType::ActionableSubmenu => {
                let submenu = filter_items(item.submenu);
                if submenu.is_empty() {
                    continue;
                }
                item.submenu = submenu;
                filtered.push(item);
            }
            _ => {
                if CONTEXT_MENU_ALLOWLIST.contains(&item.command_id) {
                    filtered.push(item);
                }
            }
        }
    }

    trim_menu_separators(filtered)
}

fn trim_menu_separators(items: Vec<ContextMenuItem>) -> Vec<ContextMenuItem> {
    let mut trimmed = Vec::new();
    let mut last_was_separator = true;

    for item in items {
        if item.r#type == ContextMenuItemType::Separator {
            if last_was_separator {
                continue;
            }
            last_was_separator = true;
            trimmed.push(item);
        } else {
            last_was_separator = false;
            trimmed.push(item);
        }
    }

    if matches!(trimmed.last(), Some(item) if item.r#type == ContextMenuItemType::Separator) {
        trimmed.pop();
    }

    trimmed
}
