use muda::{AboutMetadata, Menu, MenuEvent, PredefinedMenuItem, Submenu};

pub(crate) struct MacMenu {
    menu_bar: Menu,
    _app_menu: Submenu,
    _edit_menu: Submenu,
    window_menu: Submenu,
}

impl MacMenu {
    pub(crate) fn new() -> muda::Result<Self> {
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
        edit_menu.append_items(&[
            &PredefinedMenuItem::undo(None),
            &PredefinedMenuItem::redo(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::cut(None),
            &PredefinedMenuItem::copy(None),
            &PredefinedMenuItem::paste(None),
            &PredefinedMenuItem::select_all(None),
        ])?;

        let window_menu = Submenu::new("&Window", true);
        window_menu.append_items(&[
            &PredefinedMenuItem::minimize(None),
            &PredefinedMenuItem::maximize(None),
            &PredefinedMenuItem::fullscreen(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::bring_all_to_front(None),
        ])?;

        menu_bar.append_items(&[&edit_menu, &window_menu])?;

        Ok(Self {
            menu_bar,
            _app_menu: app_menu,
            _edit_menu: edit_menu,
            window_menu,
        })
    }

    pub(crate) fn setup(&self) {
        self.menu_bar.init_for_nsapp();
        self.window_menu.set_as_windows_menu_for_nsapp();
    }

    pub(crate) fn drain_pending_events(&self) {
        while let Ok(_event) = MenuEvent::receiver().try_recv() {}
    }
}
