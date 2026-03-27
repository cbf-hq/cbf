//! Chrome-specific `BrowserHandle` extension methods.
//!
//! This module adds convenience methods on top of
//! [`cbf::browser::BrowserHandle`] for Chrome-only operations that are not part
//! of the browser-generic `cbf` API surface.

use cbf::{browser::BrowserHandle, data::ids::BrowsingContextId, error::Error};

use crate::{
    backend::ChromiumBackend,
    command::ChromeCommand,
    data::{
        find::{ChromeFindInPageOptions, ChromeStopFindAction},
        ids::TabId,
    },
};

/// Extension trait that adds Chrome-specific commands to
/// [`BrowserHandle<ChromiumBackend>`].
///
/// These helpers wrap raw [`crate::command::ChromeCommand`] dispatch while
/// accepting browser-generic [`BrowsingContextId`] values at the API boundary.
pub trait ChromiumBrowserHandleExt {
    /// Activates an extension action for the given browsing context.
    fn activate_extension_action(
        &self,
        browsing_context_id: BrowsingContextId,
        extension_id: impl Into<String>,
    ) -> Result<(), Error>;

    /// Starts or updates a Chrome find-in-page request with explicit options.
    fn find_in_page(
        &self,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        options: ChromeFindInPageOptions,
    ) -> Result<(), Error>;

    /// Advances an existing find session to the next match for `query`.
    fn find_next(
        &self,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        query: impl Into<String>,
        match_case: bool,
    ) -> Result<(), Error>;

    /// Advances an existing find session to the previous match for `query`.
    fn find_previous(
        &self,
        browsing_context_id: BrowsingContextId,
        request_id: u64,
        query: impl Into<String>,
        match_case: bool,
    ) -> Result<(), Error>;

    /// Stops the active find-in-page session using the requested stop action.
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
