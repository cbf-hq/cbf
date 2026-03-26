use std::cell::{Cell, RefCell};

use cbf::data::extension::{ExtensionInfo, IconData};
use image::{ImageFormat, imageops::FilterType};
use muda::{
    AboutMetadata, Icon, IconMenuItem, IconMenuItemBuilder, Menu, MenuEvent, MenuId, MenuItem,
    PredefinedMenuItem, Submenu,
    accelerator::{Accelerator, CMD_OR_CTRL, Code},
};
use tracing::warn;
use winit::event_loop::EventLoopProxy;

use crate::app::events::{MenuCommand, UserEvent};

const MENU_ID_RELOAD_EXTENSIONS: &str = "simpleapp.extensions.reload";
const MENU_ID_EXTENSION_PREFIX: &str = "simpleapp.extensions.item.";
const MENU_ID_EXTENSIONS_STATUS: &str = "simpleapp.extensions.status";
const MENU_ID_FIND: &str = "simpleapp.edit.find";
const MENU_ICON_BITMAP_SIZE: u32 = 32;

struct ExtensionMenuEntry {
    item: IconMenuItem,
}

pub(crate) struct MacMenu {
    menu_bar: Menu,
    _app_menu: Submenu,
    _edit_menu: Submenu,
    extensions_menu: Submenu,
    window_menu: Submenu,
    setup_done: Cell<bool>,
    extensions_status_item: RefCell<Option<MenuItem>>,
    extension_items: RefCell<Vec<ExtensionMenuEntry>>,
}

impl MacMenu {
    pub(crate) fn new(proxy: EventLoopProxy<UserEvent>) -> muda::Result<Self> {
        MenuEvent::set_event_handler(Some(move |event| {
            if let Some(command) = menu_command_for_event(&event) {
                _ = proxy.send_event(UserEvent::Menu(command));
            }
        }));

        let menu_bar = Menu::new();
        let app_menu = Submenu::new("SimpleApp", true);
        app_menu.append_items(&[
            &PredefinedMenuItem::about(
                None,
                Some(AboutMetadata {
                    name: Some("CBF SimpleApp".to_owned()),
                    ..Default::default()
                }),
            ),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::services(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::hide(None),
            &PredefinedMenuItem::hide_others(None),
            &PredefinedMenuItem::show_all(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::quit(None),
        ])?;
        menu_bar.append(&app_menu)?;

        let edit_menu = Submenu::new("&Edit", true);
        let find_item = MenuItem::with_id(
            MENU_ID_FIND,
            "Find",
            true,
            Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyF)),
        );
        edit_menu.append_items(&[
            &PredefinedMenuItem::undo(None),
            &PredefinedMenuItem::redo(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::cut(None),
            &PredefinedMenuItem::copy(None),
            &PredefinedMenuItem::paste(None),
            &PredefinedMenuItem::select_all(None),
            &PredefinedMenuItem::separator(),
            &find_item,
        ])?;

        let reload_extensions_item =
            MenuItem::with_id(MENU_ID_RELOAD_EXTENSIONS, "Reload Extensions", true, None);
        let extensions_status_item = MenuItem::with_id(
            MENU_ID_EXTENSIONS_STATUS,
            "No extensions loaded",
            false,
            None,
        );
        let extensions_menu = Submenu::new("&Extensions", true);
        extensions_menu.append_items(&[
            &reload_extensions_item,
            &PredefinedMenuItem::separator(),
            &extensions_status_item,
        ])?;

        let window_menu = Submenu::new("&Window", true);
        window_menu.append_items(&[
            &PredefinedMenuItem::minimize(None),
            &PredefinedMenuItem::maximize(None),
            &PredefinedMenuItem::fullscreen(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::bring_all_to_front(None),
        ])?;

        menu_bar.append_items(&[&edit_menu, &extensions_menu, &window_menu])?;

        Ok(Self {
            menu_bar,
            _app_menu: app_menu,
            _edit_menu: edit_menu,
            extensions_menu,
            window_menu,
            setup_done: Cell::new(false),
            extensions_status_item: RefCell::new(Some(extensions_status_item)),
            extension_items: RefCell::new(Vec::new()),
        })
    }

    pub(crate) fn setup(&self) {
        if self.setup_done.replace(true) {
            return;
        }
        self.menu_bar.init_for_nsapp();
        self.window_menu.set_as_windows_menu_for_nsapp();
    }

    pub(crate) fn show_extensions_loading(&self) {
        self.clear_extension_items();
        self.replace_extensions_status("Loading extensions...");
    }

    pub(crate) fn replace_extensions(&self, extensions: &[ExtensionInfo]) {
        self.clear_extension_items();
        if extensions.is_empty() {
            self.replace_extensions_status("No extensions installed");
            return;
        }

        self.clear_extensions_status();
        let mut items = Vec::with_capacity(extensions.len());
        for extension in extensions {
            let menu_item = IconMenuItemBuilder::new()
                .id(MenuId::new(extension_menu_id(&extension.id)))
                .text(extension_label(extension))
                .enabled(true)
                .icon(icon_for_extension(extension))
                .build();
            if let Err(err) = self.extensions_menu.append(&menu_item) {
                warn!(
                    "failed to append extension menu item {}: {err}",
                    extension.id
                );
                continue;
            }
            items.push(ExtensionMenuEntry { item: menu_item });
        }

        if items.is_empty() {
            self.replace_extensions_status("Extensions could not be rendered");
            return;
        }

        *self.extension_items.borrow_mut() = items;
    }

    fn replace_extensions_status(&self, text: &str) {
        self.clear_extensions_status();
        let item = MenuItem::with_id(MENU_ID_EXTENSIONS_STATUS, text, false, None);
        if let Err(err) = self.extensions_menu.append(&item) {
            warn!("failed to append extensions status item: {err}");
            return;
        }
        *self.extensions_status_item.borrow_mut() = Some(item);
    }

    fn clear_extensions_status(&self) {
        if let Some(item) = self.extensions_status_item.borrow_mut().take()
            && let Err(err) = self.extensions_menu.remove(&item)
        {
            warn!("failed to remove extensions status item: {err}");
        }
    }

    fn clear_extension_items(&self) {
        let mut items = self.extension_items.borrow_mut();
        for item in items.drain(..) {
            if let Err(err) = self.extensions_menu.remove(&item.item) {
                warn!("failed to remove extension menu item: {err}");
            }
        }
    }
}

fn extension_label(extension: &ExtensionInfo) -> String {
    format!("{} ({})", extension.name, extension.version)
}

fn extension_menu_id(extension_id: &str) -> String {
    format!("{MENU_ID_EXTENSION_PREFIX}{extension_id}")
}

fn menu_command_for_event(event: &MenuEvent) -> Option<MenuCommand> {
    if event.id == MENU_ID_RELOAD_EXTENSIONS {
        return Some(MenuCommand::ReloadExtensions);
    }
    if event.id == MENU_ID_FIND {
        return Some(MenuCommand::OpenFind);
    }
    let extension_id = event.id.as_ref().strip_prefix(MENU_ID_EXTENSION_PREFIX)?;
    Some(MenuCommand::ActivateExtension {
        extension_id: extension_id.to_owned(),
    })
}

fn icon_for_extension(extension: &ExtensionInfo) -> Option<Icon> {
    match extension.icon.as_ref()? {
        IconData::Url(_) => None,
        IconData::Png(bytes) => decode_menu_icon(bytes, Some(ImageFormat::Png)),
        IconData::Binary { bytes, .. } => decode_menu_icon(bytes, None),
    }
}

fn decode_menu_icon(bytes: &[u8], format: Option<ImageFormat>) -> Option<Icon> {
    let image = match format {
        Some(format) => image::load_from_memory_with_format(bytes, format).ok()?,
        None => image::load_from_memory(bytes).ok()?,
    };
    let image = image.resize_exact(
        MENU_ICON_BITMAP_SIZE,
        MENU_ICON_BITMAP_SIZE,
        FilterType::Triangle,
    );
    let rgba = image.to_rgba8();
    Icon::from_rgba(rgba.into_raw(), image.width(), image.height()).ok()
}
