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
/// DevTools command id for storing a node as a global variable.
pub const CMD_DEVTOOLS_STORE_AS_GLOBAL_VARIABLE: i32 = 47000;
/// DevTools command id for adding an attribute to a node.
pub const CMD_DEVTOOLS_ADD_ATTRIBUTE: i32 = 47001;
/// DevTools command id for editing an attribute.
pub const CMD_DEVTOOLS_EDIT_ATTRIBUTE: i32 = 47002;
/// DevTools command id for editing a node as HTML.
pub const CMD_DEVTOOLS_EDIT_AS_HTML: i32 = 47003;
/// DevTools command id for cutting a node.
pub const CMD_DEVTOOLS_CUT: i32 = 47004;
/// DevTools command id for copying a node outer HTML.
pub const CMD_DEVTOOLS_COPY_OUTER_HTML: i32 = 47005;
/// DevTools command id for copying a node selector.
pub const CMD_DEVTOOLS_COPY_SELECTOR: i32 = 47006;
/// DevTools command id for copying a node JS path.
pub const CMD_DEVTOOLS_COPY_JS_PATH: i32 = 47007;
/// DevTools command id for copying node styles.
pub const CMD_DEVTOOLS_COPY_STYLES: i32 = 47008;
/// DevTools command id for copying a node XPath.
pub const CMD_DEVTOOLS_COPY_XPATH: i32 = 47009;
/// DevTools command id for copying a node full XPath.
pub const CMD_DEVTOOLS_COPY_FULL_XPATH: i32 = 47010;
/// DevTools command id for copying an element.
pub const CMD_DEVTOOLS_COPY_ELEMENT: i32 = 47011;
/// DevTools command id for duplicating an element.
pub const CMD_DEVTOOLS_DUPLICATE_ELEMENT: i32 = 47012;
/// DevTools command id for pasting into a node.
pub const CMD_DEVTOOLS_PASTE: i32 = 47013;
/// DevTools command id for hiding an element.
pub const CMD_DEVTOOLS_HIDE_ELEMENT: i32 = 47014;
/// DevTools command id for deleting an element.
pub const CMD_DEVTOOLS_DELETE_ELEMENT: i32 = 47015;
/// DevTools command id for expanding a node recursively.
pub const CMD_DEVTOOLS_EXPAND_RECURSIVELY: i32 = 47016;
/// DevTools command id for collapsing a node's children.
pub const CMD_DEVTOOLS_COLLAPSE_CHILDREN: i32 = 47017;
/// DevTools command id for forcing :active state.
pub const CMD_DEVTOOLS_FORCE_STATE_ACTIVE: i32 = 47019;
/// DevTools command id for forcing :hover state.
pub const CMD_DEVTOOLS_FORCE_STATE_HOVER: i32 = 47020;
/// DevTools command id for forcing :focus state.
pub const CMD_DEVTOOLS_FORCE_STATE_FOCUS: i32 = 47021;
/// DevTools command id for forcing :visited state.
pub const CMD_DEVTOOLS_FORCE_STATE_VISITED: i32 = 47022;
/// DevTools command id for forcing :focus-within state.
pub const CMD_DEVTOOLS_FORCE_STATE_FOCUS_WITHIN: i32 = 47023;
/// DevTools command id for forcing :focus-visible state.
pub const CMD_DEVTOOLS_FORCE_STATE_FOCUS_VISIBLE: i32 = 47024;
/// DevTools command id for scrolling a node into view.
pub const CMD_DEVTOOLS_SCROLL_INTO_VIEW: i32 = 47025;
/// DevTools command id for focusing a node.
pub const CMD_DEVTOOLS_FOCUS: i32 = 47026;
/// DevTools command id for toggling the ad badge.
pub const CMD_DEVTOOLS_BADGE_AD: i32 = 47027;
/// DevTools command id for toggling the container badge.
pub const CMD_DEVTOOLS_BADGE_CONTAINER: i32 = 47028;
/// DevTools command id for toggling the flex badge.
pub const CMD_DEVTOOLS_BADGE_FLEX: i32 = 47029;
/// DevTools command id for toggling the grid badge.
pub const CMD_DEVTOOLS_BADGE_GRID: i32 = 47030;
/// DevTools command id for toggling the grid-lanes badge.
pub const CMD_DEVTOOLS_BADGE_GRID_LANES: i32 = 47031;
/// DevTools command id for toggling the media badge.
pub const CMD_DEVTOOLS_BADGE_MEDIA: i32 = 47032;
/// DevTools command id for toggling the popover badge.
pub const CMD_DEVTOOLS_BADGE_POPOVER: i32 = 47033;
/// DevTools command id for toggling the reveal badge.
pub const CMD_DEVTOOLS_BADGE_REVEAL: i32 = 47034;
/// DevTools command id for toggling the scroll badge.
pub const CMD_DEVTOOLS_BADGE_SCROLL: i32 = 47035;
/// DevTools command id for toggling the scroll-snap badge.
pub const CMD_DEVTOOLS_BADGE_SCROLL_SNAP: i32 = 47036;
/// DevTools command id for toggling the slot badge.
pub const CMD_DEVTOOLS_BADGE_SLOT: i32 = 47037;
/// DevTools command id for toggling the view-source badge.
pub const CMD_DEVTOOLS_BADGE_VIEW_SOURCE: i32 = 47038;
/// DevTools command id for toggling the starting-style badge.
pub const CMD_DEVTOOLS_BADGE_STARTING_STYLE: i32 = 47039;
/// DevTools command id for toggling the subgrid badge.
pub const CMD_DEVTOOLS_BADGE_SUBGRID: i32 = 47040;
/// DevTools command id for toggling the top-layer badge.
pub const CMD_DEVTOOLS_BADGE_TOP_LAYER: i32 = 47041;
/// DevTools command id for breaking on subtree modifications.
pub const CMD_DEVTOOLS_BREAK_ON_SUBTREE_MODIFICATIONS: i32 = 47042;
/// DevTools command id for breaking on attribute modifications.
pub const CMD_DEVTOOLS_BREAK_ON_ATTRIBUTE_MODIFICATIONS: i32 = 47043;
/// DevTools command id for breaking on node removal.
pub const CMD_DEVTOOLS_BREAK_ON_NODE_REMOVAL: i32 = 47044;

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
    CMD_DEVTOOLS_STORE_AS_GLOBAL_VARIABLE,
    CMD_DEVTOOLS_ADD_ATTRIBUTE,
    CMD_DEVTOOLS_EDIT_ATTRIBUTE,
    CMD_DEVTOOLS_EDIT_AS_HTML,
    CMD_DEVTOOLS_CUT,
    CMD_DEVTOOLS_COPY_OUTER_HTML,
    CMD_DEVTOOLS_COPY_SELECTOR,
    CMD_DEVTOOLS_COPY_JS_PATH,
    CMD_DEVTOOLS_COPY_STYLES,
    CMD_DEVTOOLS_COPY_XPATH,
    CMD_DEVTOOLS_COPY_FULL_XPATH,
    CMD_DEVTOOLS_COPY_ELEMENT,
    CMD_DEVTOOLS_DUPLICATE_ELEMENT,
    CMD_DEVTOOLS_PASTE,
    CMD_DEVTOOLS_HIDE_ELEMENT,
    CMD_DEVTOOLS_DELETE_ELEMENT,
    CMD_DEVTOOLS_EXPAND_RECURSIVELY,
    CMD_DEVTOOLS_COLLAPSE_CHILDREN,
    CMD_DEVTOOLS_FORCE_STATE_ACTIVE,
    CMD_DEVTOOLS_FORCE_STATE_HOVER,
    CMD_DEVTOOLS_FORCE_STATE_FOCUS,
    CMD_DEVTOOLS_FORCE_STATE_VISITED,
    CMD_DEVTOOLS_FORCE_STATE_FOCUS_WITHIN,
    CMD_DEVTOOLS_FORCE_STATE_FOCUS_VISIBLE,
    CMD_DEVTOOLS_SCROLL_INTO_VIEW,
    CMD_DEVTOOLS_FOCUS,
    CMD_DEVTOOLS_BADGE_AD,
    CMD_DEVTOOLS_BADGE_CONTAINER,
    CMD_DEVTOOLS_BADGE_FLEX,
    CMD_DEVTOOLS_BADGE_GRID,
    CMD_DEVTOOLS_BADGE_GRID_LANES,
    CMD_DEVTOOLS_BADGE_MEDIA,
    CMD_DEVTOOLS_BADGE_POPOVER,
    CMD_DEVTOOLS_BADGE_REVEAL,
    CMD_DEVTOOLS_BADGE_SCROLL,
    CMD_DEVTOOLS_BADGE_SCROLL_SNAP,
    CMD_DEVTOOLS_BADGE_SLOT,
    CMD_DEVTOOLS_BADGE_VIEW_SOURCE,
    CMD_DEVTOOLS_BADGE_STARTING_STYLE,
    CMD_DEVTOOLS_BADGE_SUBGRID,
    CMD_DEVTOOLS_BADGE_TOP_LAYER,
    CMD_DEVTOOLS_BREAK_ON_SUBTREE_MODIFICATIONS,
    CMD_DEVTOOLS_BREAK_ON_ATTRIBUTE_MODIFICATIONS,
    CMD_DEVTOOLS_BREAK_ON_NODE_REMOVAL,
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
