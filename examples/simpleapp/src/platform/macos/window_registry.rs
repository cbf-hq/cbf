use std::{collections::HashMap, sync::Arc};

use cbf::dialogs::DialogPresentationContext;
use cbf::{browser::BrowserHandle, data::ids::WindowId as HostWindowId};
use cbf_chrome::backend::ChromiumBackend;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use tracing::warn;
use winit::{
    dpi::LogicalSize,
    event_loop::ActiveEventLoop,
    window::{CursorIcon, Window, WindowAttributes, WindowId as WinitWindowId},
};

use crate::{
    app::{
        controller::AppController,
        state::{
            DEVTOOLS_HOST_WINDOW_ID, PRIMARY_HOST_WINDOW_ID, SharedStateHandle,
            bind_transient_to_window, set_compositor_window_id_for_host_window,
            set_primary_host_window_id,
        },
    },
    platform::macos::window_visibility::WindowVisibilityObserver,
};

#[derive(Clone, Copy)]
pub(crate) enum WindowRole {
    Main,
    DevTools,
    HostPage,
    Popup,
}

struct CreateWindowOptions<'a> {
    host_window_id: HostWindowId,
    role: WindowRole,
    title: &'a str,
    size: LogicalSize<f64>,
    resizable: bool,
}

#[allow(dead_code)]
pub(crate) struct WindowEntry {
    pub(crate) host_window_id: HostWindowId,
    pub(crate) role: WindowRole,
    pub(crate) window: Arc<Window>,
    pub(crate) compositor_window_id: cbf_compositor::model::CompositorWindowId,
    _visibility_observer: Option<WindowVisibilityObserver>,
}

pub(crate) struct WindowRegistry {
    browser_handle: BrowserHandle<ChromiumBackend>,
    shared: SharedStateHandle,
    windows: HashMap<WinitWindowId, WindowEntry>,
    winit_id_by_host_window: HashMap<HostWindowId, WinitWindowId>,
}

impl WindowRegistry {
    pub(crate) fn new(
        browser_handle: BrowserHandle<ChromiumBackend>,
        shared: SharedStateHandle,
    ) -> Self {
        Self {
            browser_handle,
            shared,
            windows: HashMap::new(),
            winit_id_by_host_window: HashMap::new(),
        }
    }

    pub(crate) fn host_window_id_for_winit_window(
        &self,
        window_id: WinitWindowId,
    ) -> Option<HostWindowId> {
        self.windows
            .get(&window_id)
            .map(|entry| entry.host_window_id)
    }

    pub(crate) fn ensure_main_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        controller: &mut AppController,
    ) -> Result<(), String> {
        if self
            .winit_id_by_host_window
            .contains_key(&PRIMARY_HOST_WINDOW_ID)
        {
            return Ok(());
        }
        self.create_window(
            event_loop,
            controller,
            CreateWindowOptions {
                host_window_id: PRIMARY_HOST_WINDOW_ID,
                role: WindowRole::Main,
                title: "CBF SimpleApp",
                size: LogicalSize::new(1280.0, 900.0),
                resizable: true,
            },
        )?;
        set_primary_host_window_id(&self.shared, PRIMARY_HOST_WINDOW_ID);
        Ok(())
    }

    pub(crate) fn ensure_devtools_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        controller: &mut AppController,
    ) -> Result<(), String> {
        if self
            .winit_id_by_host_window
            .contains_key(&DEVTOOLS_HOST_WINDOW_ID)
        {
            return Ok(());
        }
        self.create_window(
            event_loop,
            controller,
            CreateWindowOptions {
                host_window_id: DEVTOOLS_HOST_WINDOW_ID,
                role: WindowRole::DevTools,
                title: "DevTools",
                size: LogicalSize::new(960.0, 720.0),
                resizable: true,
            },
        )
    }

    pub(crate) fn ensure_host_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        controller: &mut AppController,
        descriptor: cbf::data::window_open::WindowDescriptor,
    ) -> Result<(), String> {
        if self
            .winit_id_by_host_window
            .contains_key(&descriptor.window_id)
        {
            return Ok(());
        }
        self.create_window(
            event_loop,
            controller,
            CreateWindowOptions {
                host_window_id: descriptor.window_id,
                role: WindowRole::HostPage,
                title: "CBF SimpleApp",
                size: LogicalSize::new(
                    f64::from(descriptor.bounds.width.max(1)),
                    f64::from(descriptor.bounds.height.max(1)),
                ),
                resizable: true,
            },
        )
    }

    pub(crate) fn ensure_popup_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        controller: &mut AppController,
        transient_browsing_context_id: cbf::data::ids::TransientBrowsingContextId,
        title: &str,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        let host_window_id = HostWindowId::new((1_u64 << 63) | transient_browsing_context_id.get());
        if self.winit_id_by_host_window.contains_key(&host_window_id) {
            return Ok(());
        }
        self.create_window(
            event_loop,
            controller,
            CreateWindowOptions {
                host_window_id,
                role: WindowRole::Popup,
                title,
                size: LogicalSize::new(f64::from(width.max(25)), f64::from(height.max(25))),
                resizable: false,
            },
        )?;
        bind_transient_to_window(&self.shared, transient_browsing_context_id, host_window_id);
        Ok(())
    }

    pub(crate) fn close_host_window(
        &mut self,
        controller: &mut AppController,
        host_window_id: HostWindowId,
    ) {
        let Some(winit_id) = self.winit_id_by_host_window.remove(&host_window_id) else {
            return;
        };
        set_compositor_window_id_for_host_window(&self.shared, host_window_id, None);
        if let Some(entry) = self.windows.remove(&winit_id) {
            _ = controller.detach_window(entry.compositor_window_id);
        }
    }

    pub(crate) fn update_title(&self, host_window_id: HostWindowId, title: &str) {
        if let Some(window) = self.window_for_host_id(host_window_id) {
            window.set_title(title);
        }
    }

    pub(crate) fn update_cursor(&self, host_window_id: HostWindowId, cursor: CursorIcon) {
        if let Some(window) = self.window_for_host_id(host_window_id) {
            window.set_cursor(cursor);
        }
    }

    pub(crate) fn resize_window(&self, host_window_id: HostWindowId, width: u32, height: u32) {
        if let Some(window) = self.window_for_host_id(host_window_id) {
            _ = window.request_inner_size(LogicalSize::new(f64::from(width), f64::from(height)));
        }
    }

    pub(crate) fn dialog_context_for_host_window(
        &self,
        host_window_id: HostWindowId,
    ) -> DialogPresentationContext {
        let Some(window) = self.window_for_host_id(host_window_id) else {
            return DialogPresentationContext::default();
        };

        let mut context = DialogPresentationContext::default();
        if let Ok(handle) = window.window_handle() {
            context = context.with_parent_window_handle(handle.as_raw());
        }
        if let Ok(handle) = window.display_handle() {
            context = context.with_parent_display_handle(handle.as_raw());
        }
        context
    }

    pub(crate) fn sync_window_scene(
        &self,
        controller: &mut AppController,
        host_window_id: HostWindowId,
    ) {
        let Some(entry) = self.entry_for_host_id(host_window_id) else {
            return;
        };
        let (width, height) = logical_inner_size(&entry.window);
        controller.sync_window_scene(host_window_id, entry.compositor_window_id, width, height);
    }

    fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        controller: &mut AppController,
        options: CreateWindowOptions<'_>,
    ) -> Result<(), String> {
        let CreateWindowOptions {
            host_window_id,
            role,
            title,
            size,
            resizable,
        } = options;
        let attrs: WindowAttributes = Window::default_attributes()
            .with_title(title)
            .with_inner_size(size)
            .with_resizable(resizable);
        let window = event_loop
            .create_window(attrs)
            .map_err(|err| format!("failed to create window: {err}"))?;
        let window = Arc::new(window);
        let compositor_window_id = controller.attach_window(Arc::clone(&window))?;
        let visibility_observer = if matches!(role, WindowRole::Popup) {
            None
        } else {
            match WindowVisibilityObserver::install(
                &window,
                self.browser_handle.clone(),
                Arc::clone(&self.shared),
                host_window_id,
            ) {
                Ok(observer) => Some(observer),
                Err(err) => {
                    warn!(host_window_id = %host_window_id, "failed to install visibility observer: {err}");
                    None
                }
            }
        };

        let winit_window_id = window.id();
        self.windows.insert(
            winit_window_id,
            WindowEntry {
                host_window_id,
                role,
                window: Arc::clone(&window),
                compositor_window_id,
                _visibility_observer: visibility_observer,
            },
        );
        self.winit_id_by_host_window
            .insert(host_window_id, winit_window_id);
        set_compositor_window_id_for_host_window(
            &self.shared,
            host_window_id,
            Some(compositor_window_id),
        );
        let (width, height) = logical_inner_size(&window);
        controller.sync_window_scene(host_window_id, compositor_window_id, width, height);
        Ok(())
    }

    fn entry_for_host_id(&self, host_window_id: HostWindowId) -> Option<&WindowEntry> {
        let winit_id = self.winit_id_by_host_window.get(&host_window_id)?;
        self.windows.get(winit_id)
    }

    fn window_for_host_id(&self, host_window_id: HostWindowId) -> Option<&Arc<Window>> {
        self.entry_for_host_id(host_window_id)
            .map(|entry| &entry.window)
    }
}

fn logical_inner_size(window: &Window) -> (u32, u32) {
    let logical = window.inner_size().to_logical::<f64>(window.scale_factor());
    (
        logical.width.max(1.0).round() as u32,
        logical.height.max(1.0).round() as u32,
    )
}
