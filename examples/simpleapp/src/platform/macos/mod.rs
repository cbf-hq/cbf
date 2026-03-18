mod menu;
mod window_registry;
mod window_visibility;

use std::sync::{Arc, Mutex};

use async_executor::LocalExecutor;
use cbf::dialogs::{DialogPresenter, NativeDialogPresenter};
use tracing::error;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::WindowId,
};

use crate::{
    app::{
        controller::{AppController, respond_javascript_dialog_for_target},
        events::UserEvent,
        state::{CoreAction, SharedState, SharedStateHandle},
    },
    browser::{forwarder::spawn_browser_event_forwarder, startup::start_browser},
    cli::parse_cli,
};

struct AppRunner {
    controller: AppController,
    process: cbf_chrome::process::ChromiumProcess,
    registry: window_registry::WindowRegistry,
    menu: Option<menu::MacMenu>,
    // JavaScript dialogs are driven by AppKit sheet callbacks, so user
    // interaction naturally produces another winit turn. Polling this local
    // executor from `about_to_wait` is therefore enough to resume and finish
    // dialog futures without blocking the main thread.
    executor: LocalExecutor<'static>,
}

impl ApplicationHandler<UserEvent> for AppRunner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(menu) = &self.menu {
            menu.setup();
        }
        if let Err(err) = self
            .registry
            .ensure_main_window(event_loop, &mut self.controller)
        {
            error!("{err}");
            event_loop.exit();
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        let actions = match event {
            UserEvent::Browser(event) => self.controller.handle_browser_event(event),
            UserEvent::Chrome(event) => self.controller.handle_chrome_event(event),
            UserEvent::Menu(command) => self.controller.handle_menu_command(command),
        };
        self.apply_core_actions(event_loop, actions);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(host_window_id) = self.registry.host_window_id_for_winit_window(window_id) else {
            return;
        };
        let actions = self.controller.handle_window_event(host_window_id, &event);
        self.apply_core_actions(event_loop, actions);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.pump_executor();
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.controller.request_shutdown_once();
        _ = self.process.kill();
    }
}

impl AppRunner {
    fn apply_core_actions(&mut self, event_loop: &ActiveEventLoop, actions: Vec<CoreAction>) {
        for action in actions {
            match action {
                CoreAction::ExitEventLoop => event_loop.exit(),
                CoreAction::EnsureMainWindow => {
                    if let Err(err) = self
                        .registry
                        .ensure_main_window(event_loop, &mut self.controller)
                    {
                        error!("{err}");
                        event_loop.exit();
                    }
                }
                CoreAction::EnsureHostWindow { window } => {
                    if let Err(err) =
                        self.registry
                            .ensure_host_window(event_loop, &mut self.controller, window)
                    {
                        error!("{err}");
                    }
                }
                CoreAction::EnsureDevToolsWindow => {
                    if let Err(err) = self
                        .registry
                        .ensure_devtools_window(event_loop, &mut self.controller)
                    {
                        error!("{err}");
                    }
                }
                CoreAction::EnsureTransientHostWindow {
                    transient_browsing_context_id,
                    title,
                    width,
                    height,
                } => {
                    if let Err(err) = self.registry.ensure_popup_window(
                        event_loop,
                        &mut self.controller,
                        transient_browsing_context_id,
                        &title,
                        width,
                        height,
                    ) {
                        error!("{err}");
                    }
                }
                CoreAction::CloseHostWindow { window_id } => {
                    self.registry
                        .close_host_window(&mut self.controller, window_id);
                }
                CoreAction::ResizeHostWindow {
                    window_id,
                    width,
                    height,
                } => {
                    self.registry.resize_window(window_id, width, height);
                    self.registry
                        .sync_window_scene(&mut self.controller, window_id);
                }
                CoreAction::SyncWindowScene { window_id } => {
                    self.registry
                        .sync_window_scene(&mut self.controller, window_id);
                }
                CoreAction::UpdateWindowTitle { window_id, title } => {
                    self.registry.update_title(window_id, &title);
                }
                CoreAction::UpdateCursor { window_id, cursor } => {
                    self.registry.update_cursor(window_id, cursor);
                }
                CoreAction::SetExtensionsMenuLoading => {
                    if let Some(menu) = &self.menu {
                        menu.show_extensions_loading();
                    }
                }
                CoreAction::ReplaceExtensionsMenu { extensions } => {
                    if let Some(menu) = &self.menu {
                        menu.replace_extensions(&extensions);
                    }
                }
                CoreAction::PresentJavaScriptDialog {
                    target,
                    request_id,
                    request,
                } => {
                    let context = self
                        .controller
                        .host_window_id_for_dialog_target(target)
                        .map(|window_id| self.registry.dialog_context_for_host_window(window_id))
                        .unwrap_or_default();
                    let browser = self.controller.browser_handle();

                    self.executor
                        .spawn(async move {
                            let response = NativeDialogPresenter
                                .present_javascript_dialog(request, context)
                                .await;
                            respond_javascript_dialog_for_target(
                                browser, target, request_id, response,
                            );
                        })
                        .detach();
                }
            }
        }

        self.pump_executor();
    }

    fn pump_executor(&mut self) {
        while self.executor.try_tick() {}
    }
}

pub(crate) fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "simpleapp=info,cbf=info".into()),
        )
        .init();

    let cli = parse_cli();
    let runtime = match start_browser(&cli) {
        Ok(runtime) => runtime,
        Err(err) => {
            error!("{err}");
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
    let proxy = event_loop.create_proxy();
    let menu = menu::MacMenu::new(proxy.clone()).ok();
    spawn_browser_event_forwarder(runtime.events, proxy.clone());

    let shared: SharedStateHandle = Arc::new(Mutex::new(SharedState::default()));
    let controller = AppController::new(cli, runtime.session.handle(), Arc::clone(&shared));
    let registry = window_registry::WindowRegistry::new(runtime.session.handle(), shared);

    let mut runner = AppRunner {
        controller,
        process: runtime.process,
        registry,
        menu,
        executor: LocalExecutor::new(),
    };

    if let Err(err) = event_loop.run_app(&mut runner) {
        error!("event loop error: {err}");
    }
}
