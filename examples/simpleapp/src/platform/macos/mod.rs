mod menu;
mod window_registry;
mod window_visibility;

use async_executor::LocalExecutor;
use cbf::dialogs::{DialogPresenter, NativeDialogPresenter};
use tracing::error;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::WindowId,
};

pub(crate) use self::window_registry::WindowRegistry;

use crate::{
    app::{controller::respond_javascript_dialog_for_target, events::UserEvent, state::CoreAction},
    browser::startup::{RunningApp, launch_backend},
    cli::{Cli, parse_cli},
};

enum AppRunnerState {
    Launching,
    Running(Box<RunningApp>),
    Failed,
}

struct AppRunner {
    pending_cli: Option<Cli>,
    proxy: EventLoopProxy<UserEvent>,
    state: AppRunnerState,
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

        if matches!(self.state, AppRunnerState::Launching) {
            let Some(cli) = self.pending_cli.take() else {
                error!("missing startup CLI while launching backend");
                self.state = AppRunnerState::Failed;
                event_loop.exit();
                return;
            };

            match launch_backend(cli, self.proxy.clone()) {
                Ok(running) => self.state = AppRunnerState::Running(Box::new(running)),
                Err(err) => {
                    error!("{err}");
                    self.state = AppRunnerState::Failed;
                    event_loop.exit();
                    return;
                }
            }
        }

        if let Err(err) = self.ensure_main_window(event_loop) {
            error!("{err}");
            self.state = AppRunnerState::Failed;
            event_loop.exit();
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        let Some(running) = self.running_mut() else {
            return;
        };

        let actions = match event {
            UserEvent::Browser(event) => running.controller.handle_browser_event(event),
            UserEvent::Chrome(event) => running.controller.handle_chrome_event(event),
            UserEvent::Menu(command) => running.controller.handle_menu_command(command),
        };

        self.apply_core_actions(event_loop, actions);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(running) = self.running_mut() else {
            return;
        };

        let Some(host_window_id) = running.registry.host_window_id_for_winit_window(window_id)
        else {
            return;
        };
        let actions = running
            .controller
            .handle_window_event(host_window_id, &event);

        self.apply_core_actions(event_loop, actions);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.pump_executor();
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        let Some(running) = self.running_mut() else {
            return;
        };

        if running.browser.shutdown_state()
            == cbf_chrome::process::ChromiumRuntimeShutdownState::Idle
        {
            running.controller.request_shutdown_once();
            _ = running
                .browser
                .shutdown(cbf_chrome::process::ShutdownMode::Force);
        }
    }
}

impl AppRunner {
    fn running_mut(&mut self) -> Option<&mut RunningApp> {
        match &mut self.state {
            AppRunnerState::Running(running) => Some(running),
            AppRunnerState::Launching | AppRunnerState::Failed => None,
        }
    }

    fn ensure_main_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let Some(running) = self.running_mut() else {
            return Ok(());
        };

        running
            .registry
            .ensure_main_window(event_loop, &mut running.controller)
    }

    fn apply_core_actions(&mut self, event_loop: &ActiveEventLoop, actions: Vec<CoreAction>) {
        for action in actions {
            match action {
                CoreAction::ExitEventLoop => event_loop.exit(),
                CoreAction::EnsureMainWindow => {
                    let Some(running) = self.running_mut() else {
                        continue;
                    };

                    if let Err(err) = running
                        .registry
                        .ensure_main_window(event_loop, &mut running.controller)
                    {
                        error!("{err}");
                        event_loop.exit();
                    }
                }
                CoreAction::EnsureHostWindow { window } => {
                    let Some(running) = self.running_mut() else {
                        continue;
                    };

                    if let Err(err) = running.registry.ensure_host_window(
                        event_loop,
                        &mut running.controller,
                        window,
                    ) {
                        error!("{err}");
                    }
                }
                CoreAction::EnsureDevToolsWindow => {
                    let Some(running) = self.running_mut() else {
                        continue;
                    };

                    if let Err(err) = running
                        .registry
                        .ensure_devtools_window(event_loop, &mut running.controller)
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
                    let Some(running) = self.running_mut() else {
                        continue;
                    };

                    if let Err(err) = running.registry.ensure_popup_window(
                        event_loop,
                        &mut running.controller,
                        transient_browsing_context_id,
                        &title,
                        width,
                        height,
                    ) {
                        error!("{err}");
                    }
                }
                CoreAction::CloseHostWindow { window_id } => {
                    let Some(running) = self.running_mut() else {
                        continue;
                    };

                    running
                        .registry
                        .close_host_window(&mut running.controller, window_id);
                }
                CoreAction::ResizeHostWindow {
                    window_id,
                    width,
                    height,
                } => {
                    let Some(running) = self.running_mut() else {
                        continue;
                    };

                    running.registry.resize_window(window_id, width, height);
                    running
                        .registry
                        .sync_window_scene(&mut running.controller, window_id);
                }
                CoreAction::SyncWindowScene { window_id } => {
                    let Some(running) = self.running_mut() else {
                        continue;
                    };

                    running
                        .registry
                        .sync_window_scene(&mut running.controller, window_id);
                }
                CoreAction::UpdateWindowTitle { window_id, title } => {
                    let Some(running) = self.running_mut() else {
                        continue;
                    };

                    running.registry.update_title(window_id, &title);
                }
                CoreAction::UpdateCursor { window_id, cursor } => {
                    let Some(running) = self.running_mut() else {
                        continue;
                    };

                    running.registry.update_cursor(window_id, cursor);
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
                    let (context, browser) = {
                        let Some(running) = self.running_mut() else {
                            continue;
                        };
                        let context = running
                            .controller
                            .host_window_id_for_dialog_target(target)
                            .map(|window_id| {
                                running.registry.dialog_context_for_host_window(window_id)
                            })
                            .unwrap_or_default();
                        let browser = running.controller.browser_handle();
                        (context, browser)
                    };

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

    let event_loop = match EventLoop::<UserEvent>::with_user_event().build() {
        Ok(event_loop) => event_loop,
        Err(err) => {
            error!("failed to build winit event loop: {err}");
            return;
        }
    };
    let proxy = event_loop.create_proxy();
    let menu = menu::MacMenu::new(proxy.clone()).ok();

    let mut runner = AppRunner {
        pending_cli: Some(cli),
        proxy,
        state: AppRunnerState::Launching,
        menu,
        executor: LocalExecutor::new(),
    };

    if let Err(err) = event_loop.run_app(&mut runner) {
        error!("event loop error: {err}");
    }
}
