use std::{
    sync::{Arc, Mutex},
    thread,
};

use cbf::{
    browser::RawOpaqueEventExt,
    browser::{BrowserHandle, EventStream},
    event::BrowserEvent,
    middleware::{
        MiddlewareBuilder, error_guard::ErrorGuardLayer, lifecycle::LifecycleLayer,
        logging::LoggingLayer,
    },
};
use cbf_chrome::{
    chromium_backend::ChromiumBackend,
    chromium_process::{ChromiumProcess, start_chromium},
    event::ChromeEvent,
    ffi::IpcEvent,
    surface::SurfaceHandle,
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

/// Custom event type for the winit event loop.
/// This wraps browser events so they can be delivered through the event loop.
#[derive(Debug)]
pub(crate) enum UserEvent {
    Browser(BrowserEvent),
    SurfaceHandleUpdated {
        browsing_context_id: cbf::data::ids::BrowsingContextId,
        handle: SurfaceHandle,
    },
}

/// Spawns a background thread that forwards browser events to the winit event loop.
///
/// This thread continuously reads from the CBF event stream and sends events
/// to the event loop proxy. It terminates when either the event stream closes
/// or the event loop proxy becomes invalid.
pub(crate) fn spawn_browser_event_forwarder(
    events: EventStream<ChromiumBackend>,
    proxy: EventLoopProxy<UserEvent>,
) {
    thread::spawn(move || {
        loop {
            match events.recv_blocking() {
                Ok(event) => {
                    if let ChromeEvent::Ipc(ipc_event) = event.as_raw().clone()
                        && let IpcEvent::SurfaceHandleUpdated {
                            browsing_context_id,
                            handle,
                            ..
                        } = *ipc_event
                        && proxy
                            .send_event(UserEvent::SurfaceHandleUpdated {
                                browsing_context_id,
                                handle,
                            })
                            .is_err()
                    {
                        return;
                    }

                    if let Some(event) = event.as_generic().cloned()
                        && proxy.send_event(UserEvent::Browser(event)).is_err()
                    {
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

/// Platform-specific application trait that must be implemented for each OS.
///
/// This trait separates platform-agnostic core logic from platform-specific
/// windowing and view management. Each platform (macOS, Windows, Linux) implements
/// this trait to provide its own native window and browser view handling.
pub(crate) trait PlatformApp {
    /// Creates a new platform application instance.
    fn new(browser_handle: BrowserHandle<ChromiumBackend>, shared: Arc<Mutex<SharedState>>)
    -> Self;

    /// Returns the winit window ID, if a window has been created.
    fn window_id(&self) -> Option<WindowId>;

    /// Ensures that a window and browser view are created and ready.
    /// Called when the event loop is resumed.
    fn ensure_window_and_view(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String>;

    /// Applies a list of core actions to the platform layer.
    /// This is where platform-specific implementations execute requested actions
    /// like updating the window title, resizing views, showing menus, etc.
    fn apply_core_actions(
        &mut self,
        event_loop: &ActiveEventLoop,
        core: &mut CoreState,
        actions: Vec<CoreAction>,
    );
}

/// Main application runner that ties together core logic, browser process, and platform layer.
///
/// This struct implements the winit [`ApplicationHandler`] trait and orchestrates
/// the flow of events between the windowing system, CBF browser backend, and core logic.
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
        let actions = match event {
            UserEvent::Browser(event) => self.core.handle_browser_event(event),
            UserEvent::SurfaceHandleUpdated {
                browsing_context_id,
                handle,
            } => self.core.handle_surface_update(browsing_context_id, handle),
        };
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

/// Main entry point that initializes and runs the application with a platform-specific implementation.
pub(crate) fn run_with_platform<P: PlatformApp + 'static>() {
    // Initialize logging with default level of info for simpleapp and cbf.
    // Can be overridden with RUST_LOG environment variable.
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

    // Build the middleware stack for the browser session.
    // - LifecycleLayer: Manages browser lifecycle and reconnection
    // - ErrorGuardLayer: Stops backend on severe/repeated backend errors
    // - LoggingLayer: Logs commands, events, and teardown for debugging
    let delegate = match MiddlewareBuilder::new()
        .layer(LifecycleLayer::new())
        .layer(ErrorGuardLayer::new())
        .layer(
            LoggingLayer::new("simpleapp")
                .command_level(Level::DEBUG)
                .event_level(Level::DEBUG)
                .teardown_level(Level::INFO),
        )
        .build()
    {
        Ok(delegate) => delegate,
        Err(err) => {
            error!("failed to build middleware delegate: {err}");
            return;
        }
    };

    // Start the Chromium browser process and establish IPC connection.
    // Returns: browser session, event stream, and process handle.
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

    // Create shared state, core logic, and platform implementation.
    let shared = Arc::new(Mutex::new(SharedState::default()));
    let core = CoreState::new(cli, session, Arc::clone(&shared));
    let platform = P::new(core.browser_handle(), shared);

    // Spawn background thread to forward browser events to the event loop.
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
