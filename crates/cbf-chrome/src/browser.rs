use cbf::{browser::BrowserHandle, data::ids::BrowsingContextId, error::Error};

use crate::{backend::ChromiumBackend, command::ChromeCommand, data::ids::TabId};

pub trait ChromiumBrowserHandleExt {
    fn activate_extension_action(
        &self,
        browsing_context_id: BrowsingContextId,
        extension_id: impl Into<String>,
    ) -> Result<(), Error>;
}

impl ChromiumBrowserHandleExt for BrowserHandle<ChromiumBackend> {
    fn activate_extension_action(
        &self,
        browsing_context_id: BrowsingContextId,
        extension_id: impl Into<String>,
    ) -> Result<(), Error> {
        self.send_raw(ChromeCommand::ActivateExtensionAction {
            browsing_context_id: TabId::from(browsing_context_id),
            extension_id: extension_id.into(),
        })
    }
}
