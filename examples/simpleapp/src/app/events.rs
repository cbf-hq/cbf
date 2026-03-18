use cbf::event::BrowserEvent;
use cbf_chrome::event::ChromeEvent;

#[derive(Debug, Clone)]
pub(crate) enum MenuCommand {
    ReloadExtensions,
    ActivateExtension { extension_id: String },
}

#[derive(Debug)]
pub(crate) enum UserEvent {
    Browser(BrowserEvent),
    Chrome(ChromeEvent),
    Menu(MenuCommand),
}
