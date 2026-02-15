use std::{
    sync::{Arc, Mutex},
    thread,
};

use cbf::{
    BrowserHandle, EventStream,
    chromium_process::{ChromiumProcess, start_chromium},
    event::BrowserEvent,
    middleware::{MiddlewareBuilder, lifecycle::LifecycleLayer, logging::LoggingLayerBuilder},
};
use tracing::{Level, error, warn};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::WindowId,
};

use crate::{
    cli::{chromium_options_from_cli, parse_cli},
    core::{CoreAction, CoreState, SharedState},
};

#[derive(Debug)]
pub(crate) enum UserEvent {
    Browser(BrowserEvent),
}

pub(crate) fn spawn_browser_event_forwarder(events: EventStream, proxy: EventLoopProxy<UserEvent>) {
    thread::spawn(move || {
        loop {
            match events.recv_blocking() {
                Ok(event) => {
                    if proxy.send_event(UserEvent::Browser(event)).is_err() {
                        return;
                    }
                }
                Err(err) => {
                    warn!("browser event stream closed: {err}");
                    return;
                }
            }
        }
    });
}

pub(crate) trait PlatformApp {
    fn new(browser_handle: BrowserHandle, shared: Arc<Mutex<SharedState>>) -> Self;
    fn window_id(&self) -> Option<WindowId>;
    fn ensure_window_and_view(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String>;
    fn apply_core_actions(
        &mut self,
        event_loop: &ActiveEventLoop,
        core: &mut CoreState,
        actions: Vec<CoreAction>,
    );
}

struct AppRunner<P: PlatformApp> {
    core: CoreState,
    process: ChromiumProcess,
    platform: P,
}

impl<P: PlatformApp> ApplicationHandler<UserEvent> for AppRunner<P> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(message) = self.platform.ensure_window_and_view(event_loop) {
            error!("{message}");
            event_loop.exit();
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        let UserEvent::Browser(event) = event;
        let actions = self.core.handle_browser_event(event);
        self.platform
            .apply_core_actions(event_loop, &mut self.core, actions);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if Some(window_id) != self.platform.window_id() {
            return;
        }

        let actions = self.core.handle_window_event(&event);
        self.platform
            .apply_core_actions(event_loop, &mut self.core, actions);
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.core.request_shutdown_once();
        if let Err(err) = self.process.kill() {
            warn!("failed to kill chromium process on exit: {err}");
        }
    }
}

pub(crate) fn run_with_platform<P: PlatformApp + 'static>() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "simpleapp=info,cbf=info".into()),
        )
        .init();

    let cli = parse_cli();
    let options = match chromium_options_from_cli(&cli) {
        Ok(options) => options,
        Err(message) => {
            error!("{message}");
            return;
        }
    };

    let delegate = match MiddlewareBuilder::new()
        .layer(LifecycleLayer::new())
        .layer(
            LoggingLayerBuilder::new("simpleapp")
                .command_level(Level::DEBUG)
                .event_level(Level::DEBUG)
                .teardown_level(Level::INFO)
                .build(),
        )
        .build()
    {
        Ok(delegate) => delegate,
        Err(err) => {
            error!("failed to build middleware delegate: {err}");
            return;
        }
    };

    let (session, events, process) = match start_chromium(options, delegate) {
        Ok(values) => values,
        Err(err) => {
            error!("failed to start chromium backend: {err}");
            return;
        }
    };

    let event_loop = match EventLoop::<UserEvent>::with_user_event().build() {
        Ok(event_loop) => event_loop,
        Err(err) => {
            error!("failed to build winit event loop: {err}");
            return;
        }
    };

    let shared = Arc::new(Mutex::new(SharedState::default()));
    let core = CoreState::new(cli, session, Arc::clone(&shared));
    let platform = P::new(core.browser_handle(), shared);

    let proxy = event_loop.create_proxy();
    spawn_browser_event_forwarder(events, proxy);

    let mut runner = AppRunner {
        core,
        process,
        platform,
    };

    if let Err(err) = event_loop.run_app(&mut runner) {
        error!("event loop error: {err}");
    }
}
