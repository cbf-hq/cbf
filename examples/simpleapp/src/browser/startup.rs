use cbf::middleware::{
    MiddlewareBuilder, error_guard::ErrorGuardLayer, lifecycle::LifecycleLayer,
    logging::LoggingLayer,
};
use cbf_chrome::process::{ChromiumRuntime, start_chromium};
use std::sync::{Arc, Mutex};
use tracing::Level;
use winit::event_loop::EventLoopProxy;

use crate::{
    app::{
        controller::AppController,
        events::UserEvent,
        state::{SharedState, SharedStateHandle},
    },
    browser::forwarder::spawn_browser_event_forwarder,
    cli::{Cli, chromium_options_from_cli},
    platform::macos::WindowRegistry,
};

pub(crate) struct RunningApp {
    pub(crate) controller: AppController,
    pub(crate) browser: ChromiumRuntime,
    pub(crate) registry: WindowRegistry,
}

pub(crate) fn launch_backend(
    cli: Cli,
    proxy: EventLoopProxy<UserEvent>,
) -> Result<RunningApp, String> {
    let options = chromium_options_from_cli(&cli)?;
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

    let browser = ChromiumRuntime::new(session, events, process);
    let shutdown_state = browser.shutdown_state_reader();
    browser.install_signal_handlers().ok();
    spawn_browser_event_forwarder(browser.events(), shutdown_state.clone(), proxy);

    let shared: SharedStateHandle = Arc::new(Mutex::new(SharedState::default()));
    let controller = AppController::new(
        cli,
        browser.session().handle(),
        shutdown_state,
        Arc::clone(&shared),
    );
    let registry = WindowRegistry::new(browser.session().handle(), shared);

    Ok(RunningApp {
        controller,
        browser,
        registry,
    })
}
