use cbf::{browser::BrowserHandle, data::ids::BrowsingContextId, error::Error};

use crate::{
    backend::ChromiumBackend,
    command::ChromeCommand,
    data::{
        find::{ChromeFindInPageOptions, ChromeStopFindAction},
        ids::TabId,
    },
};

pub trait ChromiumBrowserHandleExt {
    fn activate_extension_action(
        &self,
        browsing_context_id: BrowsingContextId,
        extension_id: impl Into<String>,
    ) -> Result<(), Error>;

    fn find_in_page(
        &self,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        options: ChromeFindInPageOptions,
    ) -> Result<(), Error>;

    fn find_next(
        &self,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        query: impl Into<String>,
        match_case: bool,
    ) -> Result<(), Error>;

    fn find_previous(
        &self,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        query: impl Into<String>,
        match_case: bool,
    ) -> Result<(), Error>;

    fn stop_finding(
        &self,
        browsing_context_id: BrowsingContextId,
        action: ChromeStopFindAction,
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

    fn find_in_page(
        &self,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        options: ChromeFindInPageOptions,
    ) -> Result<(), Error> {
        self.send_raw(ChromeCommand::FindInPage {
            browsing_context_id: TabId::from(browsing_context_id),
            request_id,
            options,
        })
    }

    fn find_next(
        &self,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        query: impl Into<String>,
        match_case: bool,
    ) -> Result<(), Error> {
        self.find_in_page(
            browsing_context_id,
            request_id,
            ChromeFindInPageOptions {
                query: query.into(),
                forward: true,
                match_case,
                new_session: false,
                find_match: true,
            },
        )
    }

    fn find_previous(
        &self,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        query: impl Into<String>,
        match_case: bool,
    ) -> Result<(), Error> {
        self.find_in_page(
            browsing_context_id,
            request_id,
            ChromeFindInPageOptions {
                query: query.into(),
                forward: false,
                match_case,
                new_session: false,
                find_match: true,
            },
        )
    }

    fn stop_finding(
        &self,
        browsing_context_id: BrowsingContextId,
        action: ChromeStopFindAction,
    ) -> Result<(), Error> {
        self.send_raw(ChromeCommand::StopFinding {
            browsing_context_id: TabId::from(browsing_context_id),
            action,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_channel::unbounded;
    use cbf::browser::{CommandEnvelope, CommandSender};

    fn build_sender() -> CommandSender<ChromiumBackend> {
        let (tx, _rx) = unbounded::<CommandEnvelope<ChromiumBackend>>();
        CommandSender::from_raw_sender(tx)
    }

    #[test]
    fn find_options_default_to_forward_new_session() {
        let options = ChromeFindInPageOptions::new("needle");
        assert_eq!(options.query, "needle");
        assert!(options.forward);
        assert!(!options.match_case);
        assert!(options.new_session);
        assert!(options.find_match);
    }

    #[test]
    fn stop_find_action_maps_to_expected_ffi_value() {
        assert_eq!(
            ChromeStopFindAction::ClearSelection.to_ffi(),
            cbf_chrome_sys::ffi::CbfStopFindAction_kCbfStopFindActionClearSelection as u8
        );
        assert_eq!(
            ChromeStopFindAction::KeepSelection.to_ffi(),
            cbf_chrome_sys::ffi::CbfStopFindAction_kCbfStopFindActionKeepSelection as u8
        );
        assert_eq!(
            ChromeStopFindAction::ActivateSelection.to_ffi(),
            cbf_chrome_sys::ffi::CbfStopFindAction_kCbfStopFindActionActivateSelection as u8
        );
    }

    #[test]
    fn command_sender_can_be_constructed_for_find_helpers() {
        let _sender = build_sender();
    }
}
