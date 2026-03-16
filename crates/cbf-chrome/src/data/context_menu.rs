//! Chrome-specific context menu item structures, including types, icons, accelerators, and submenus.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChromeContextMenuItemType {
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
pub struct ChromeContextMenuIcon {
    pub png_bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeContextMenuAccelerator {
    pub key_equivalent: String,
    pub modifier_mask: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeContextMenuItem {
    pub r#type: ChromeContextMenuItemType,
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
    pub accelerator: Option<ChromeContextMenuAccelerator>,
    pub icon: Option<ChromeContextMenuIcon>,
    pub minor_icon: Option<ChromeContextMenuIcon>,
    pub submenu: Vec<ChromeContextMenuItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeContextMenu {
    pub menu_id: u64,
    pub x: i32,
    pub y: i32,
    pub source_type: u32,
    pub items: Vec<ChromeContextMenuItem>,
}

impl From<ChromeContextMenuItemType> for cbf::data::context_menu::ContextMenuItemType {
    fn from(value: ChromeContextMenuItemType) -> Self {
        match value {
            ChromeContextMenuItemType::Command => Self::Command,
            ChromeContextMenuItemType::Check => Self::Check,
            ChromeContextMenuItemType::Radio => Self::Radio,
            ChromeContextMenuItemType::Separator => Self::Separator,
            ChromeContextMenuItemType::ButtonItem => Self::ButtonItem,
            ChromeContextMenuItemType::Submenu => Self::Submenu,
            ChromeContextMenuItemType::ActionableSubmenu => Self::ActionableSubmenu,
            ChromeContextMenuItemType::Highlighted => Self::Highlighted,
            ChromeContextMenuItemType::Title => Self::Title,
        }
    }
}

impl From<ChromeContextMenuIcon> for cbf::data::context_menu::ContextMenuIcon {
    fn from(value: ChromeContextMenuIcon) -> Self {
        Self {
            png_bytes: value.png_bytes,
            width: value.width,
            height: value.height,
        }
    }
}

impl From<ChromeContextMenuAccelerator> for cbf::data::context_menu::ContextMenuAccelerator {
    fn from(value: ChromeContextMenuAccelerator) -> Self {
        Self {
            key_equivalent: value.key_equivalent,
            modifier_mask: value.modifier_mask,
        }
    }
}

impl From<ChromeContextMenuItem> for cbf::data::context_menu::ContextMenuItem {
    fn from(value: ChromeContextMenuItem) -> Self {
        Self {
            r#type: value.r#type.into(),
            command_id: value.command_id,
            label: value.label,
            secondary_label: value.secondary_label,
            minor_text: value.minor_text,
            accessible_name: value.accessible_name,
            enabled: value.enabled,
            visible: value.visible,
            checked: value.checked,
            group_id: value.group_id,
            is_new_feature: value.is_new_feature,
            is_alerted: value.is_alerted,
            may_have_mnemonics: value.may_have_mnemonics,
            accelerator: value.accelerator.map(Into::into),
            icon: value.icon.map(Into::into),
            minor_icon: value.minor_icon.map(Into::into),
            submenu: value.submenu.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ChromeContextMenu> for cbf::data::context_menu::ContextMenu {
    fn from(value: ChromeContextMenu) -> Self {
        Self {
            menu_id: value.menu_id,
            x: value.x,
            y: value.y,
            source_type: value.source_type,
            items: value.items.into_iter().map(Into::into).collect(),
        }
    }
}

/// Chromium-derived command ids mirrored from `chrome/app/chrome_command_ids.h`.
/// Keep these values aligned with the Chromium revision used by CBF because the
/// backend reports and accepts the same command id space.
///
/// Current upstream reference:
/// - `IDC_BACK`, `IDC_FORWARD`, `IDC_RELOAD`
/// - `IDC_PRINT`
/// - `IDC_CONTENT_CONTEXT_*`
///   in `chromium/src/chrome/app/chrome_command_ids.h`
///
/// Command id for navigating back.
pub const CMD_BACK: i32 = 33000;
/// Command id for navigating forward.
pub const CMD_FORWARD: i32 = 33001;
/// Command id for reloading the page.
pub const CMD_RELOAD: i32 = 33002;
/// Command id for printing the page.
pub const CMD_PRINT: i32 = 35003;
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
/// Command id for saving an image as a file.
pub const CMD_CONTENT_SAVE_IMAGE_AS: i32 = 50120;
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
/// Command id for inspecting element via DevTools.
pub const CMD_CONTENT_INSPECT_ELEMENT: i32 = 50162;

const CONTEXT_MENU_ALLOWLIST: &[i32] = &[
    CMD_BACK,
    CMD_FORWARD,
    CMD_RELOAD,
    CMD_PRINT,
    CMD_CUT,
    CMD_COPY,
    CMD_PASTE,
    CMD_CONTENT_OPEN_LINK_NEW_TAB,
    CMD_CONTENT_OPEN_LINK_NEW_WINDOW,
    CMD_CONTENT_COPY_LINK_LOCATION,
    CMD_CONTENT_SAVE_IMAGE_AS,
    CMD_CONTENT_COPY_IMAGE_LOCATION,
    CMD_CONTENT_COPY_IMAGE,
    CMD_CONTENT_COPY,
    CMD_CONTENT_CUT,
    CMD_CONTENT_PASTE,
    CMD_CONTENT_UNDO,
    CMD_CONTENT_REDO,
    CMD_CONTENT_SELECT_ALL,
    CMD_CONTENT_PASTE_AND_MATCH_STYLE,
    CMD_CONTENT_INSPECT_ELEMENT,
];

pub fn filter_supported(menu: ChromeContextMenu) -> ChromeContextMenu {
    let items = filter_items(menu.items);
    ChromeContextMenu { items, ..menu }
}

/// Check whether a command id represents "open link in new tab".
pub fn is_open_link_new_tab(command_id: i32) -> bool {
    command_id == CMD_CONTENT_OPEN_LINK_NEW_TAB
}

/// Check whether a command id represents "open link in new window".
pub fn is_open_link_new_window(command_id: i32) -> bool {
    command_id == CMD_CONTENT_OPEN_LINK_NEW_WINDOW
}

fn filter_items(items: Vec<ChromeContextMenuItem>) -> Vec<ChromeContextMenuItem> {
    let mut filtered = Vec::new();

    for mut item in items {
        match item.r#type {
            ChromeContextMenuItemType::Separator => filtered.push(item),
            ChromeContextMenuItemType::Submenu | ChromeContextMenuItemType::ActionableSubmenu => {
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

fn trim_menu_separators(items: Vec<ChromeContextMenuItem>) -> Vec<ChromeContextMenuItem> {
    let mut trimmed = Vec::new();
    let mut last_was_separator = true;

    for item in items {
        if item.r#type == ChromeContextMenuItemType::Separator {
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

    if matches!(trimmed.last(), Some(item) if item.r#type == ChromeContextMenuItemType::Separator) {
        trimmed.pop();
    }

    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_supported_keeps_save_image_as_item() {
        let menu = ChromeContextMenu {
            menu_id: 1,
            x: 0,
            y: 0,
            source_type: 0,
            items: vec![ChromeContextMenuItem {
                r#type: ChromeContextMenuItemType::Command,
                command_id: CMD_CONTENT_SAVE_IMAGE_AS,
                label: String::new(),
                secondary_label: String::new(),
                minor_text: String::new(),
                accessible_name: String::new(),
                enabled: true,
                visible: true,
                checked: false,
                group_id: 0,
                is_new_feature: false,
                is_alerted: false,
                may_have_mnemonics: false,
                accelerator: None,
                icon: None,
                minor_icon: None,
                submenu: Vec::new(),
            }],
        };

        let filtered = filter_supported(menu);

        assert_eq!(filtered.items.len(), 1);
        assert_eq!(filtered.items[0].command_id, CMD_CONTENT_SAVE_IMAGE_AS);
    }
}
