use cbf::{
    browser::EventStream,
    middleware::{
        MiddlewareBuilder, error_guard::ErrorGuardLayer, lifecycle::LifecycleLayer,
        logging::LoggingLayer,
    },
};
use cbf_chrome::{
    backend::ChromiumBackend,
    process::{ChromiumProcess, start_chromium},
};
use tracing::Level;

use crate::cli::{Cli, chromium_options_from_cli};

pub(crate) struct BrowserRuntime {
    pub(crate) session: cbf::browser::BrowserSession<ChromiumBackend>,
    pub(crate) events: EventStream<ChromiumBackend>,
    pub(crate) process: ChromiumProcess,
}

pub(crate) fn start_browser(cli: &Cli) -> Result<BrowserRuntime, String> {
    let options = chromium_options_from_cli(cli)?;
    let delegate = MiddlewareBuilder::new()
        .layer(LifecycleLayer::new())
        .layer(ErrorGuardLayer::new())
        .layer(
            LoggingLayer::new("simpleapp")
                .command_level(Level::DEBUG)
                .event_level(Level::DEBUG)
                .teardown_level(Level::INFO),
        )
        .build()
        .map_err(|err| format!("failed to build middleware delegate: {err}"))?;

    let (session, events, process) = start_chromium(options, delegate)
        .map_err(|err| format!("failed to start chromium backend: {err}"))?;

    Ok(BrowserRuntime {
        session,
        events,
        process,
    })
}
