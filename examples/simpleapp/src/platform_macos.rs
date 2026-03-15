use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

mod menu;

use cbf::{
    browser::BrowserHandle,
    data::{
        drag::{DragDrop, DragUpdate},
        ids::{BrowsingContextId, TransientBrowsingContextId, WindowId as HostWindowId},
        ime::{
            ConfirmCompositionBehavior, ImeCommitText, ImeComposition, ImeTextSpan, ImeTextSpanType,
        },
        transient_browsing_context::{TransientImeCommitText, TransientImeComposition},
        window_open::WindowDescriptor,
    },
};
use cbf_chrome::{
    backend::ChromiumBackend,
    data::surface::SurfaceHandle,
    platform::macos::browser_view::{
        BrowserViewMac, BrowserViewMacConfig, BrowserViewMacDelegate, BrowserViewMacImeEvent,
        BrowserViewMacNativeDragDrop, BrowserViewMacNativeDragUpdate,
    },
};
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::NSView;
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tracing::{debug, warn};
use winit::{
    dpi::LogicalSize,
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::{Window, WindowAttributes, WindowId as WinitWindowId},
};

use crate::{
    app::{PlatformApp, UserEvent, run_with_platform},
    core::{
        CoreAction, CoreState, PRIMARY_HOST_WINDOW_ID, SharedState, ViewTarget,
        bind_transient_to_window, browsing_context_id_for_target, browsing_context_id_for_window,
        drag_allowed_operations, remove_drag_session, set_drag_allowed_operations,
        set_primary_host_window_id, transient_browsing_context_id_for_window,
        window_id_for_transient_browsing_context,
    },
};

#[derive(Debug, Clone, Copy)]
enum BrowserViewBinding {
    Primary,
    DevTools,
    HostWindow(HostWindowId),
    Transient(TransientBrowsingContextId),
}

/// Delegate for the macOS browser view that handles input events, IME, drag-and-drop, etc.
struct SimpleBrowserViewDelegate {
    handle: BrowserHandle<ChromiumBackend>,
    shared: Arc<Mutex<SharedState>>,
    binding: BrowserViewBinding,
}

impl SimpleBrowserViewDelegate {
    fn with_page_id<F>(&self, f: F)
    where
        F: FnOnce(BrowsingContextId),
    {
        let browsing_context_id = match self.binding {
            BrowserViewBinding::Primary => {
                browsing_context_id_for_target(&self.shared, ViewTarget::Primary)
            }
            BrowserViewBinding::DevTools => {
                browsing_context_id_for_target(&self.shared, ViewTarget::DevTools)
            }
            BrowserViewBinding::HostWindow(window_id) => {
                browsing_context_id_for_window(&self.shared, window_id)
            }
            BrowserViewBinding::Transient(_) => None,
        };
        if let Some(browsing_context_id) = browsing_context_id {
            f(browsing_context_id);
        }
    }

    fn with_transient_id<F>(&self, f: F)
    where
        F: FnOnce(TransientBrowsingContextId),
    {
        let transient_browsing_context_id = match self.binding {
            BrowserViewBinding::Transient(transient_browsing_context_id) => {
                Some(transient_browsing_context_id)
            }
            BrowserViewBinding::Primary
            | BrowserViewBinding::DevTools
            | BrowserViewBinding::HostWindow(_) => None,
        };
        if let Some(transient_browsing_context_id) = transient_browsing_context_id {
            f(transient_browsing_context_id);
        }
    }
}

impl BrowserViewMacDelegate for SimpleBrowserViewDelegate {
    fn on_key_event(
        &self,
        _view: &BrowserViewMac,
        event: cbf::data::key::KeyEvent,
        commands: Vec<String>,
    ) {
        let transient_event = event.clone();
        let transient_commands = commands.clone();
        self.with_page_id(|browsing_context_id| {
            if let Err(err) = self
                .handle
                .send_key_event(browsing_context_id, event, commands)
            {
                warn!("failed to forward key event: {err}");
            }
        });
        self.with_transient_id(|transient_browsing_context_id| {
            if let Err(err) = self.handle.send_key_event_to_transient_browsing_context(
                transient_browsing_context_id,
                transient_event,
                transient_commands,
            ) {
                warn!("failed to forward transient key event: {err}");
            }
        });
    }

    fn on_ime_event(&self, _view: &BrowserViewMac, event: BrowserViewMacImeEvent) {
        let transient_event = event.clone();
        self.with_page_id(|browsing_context_id| match event {
            BrowserViewMacImeEvent::SetComposition {
                text,
                selection,
                replacement,
            } => {
                let utf16_len = text.encode_utf16().count();
                let (selection_start, selection_end) = selection
                    .map(|range| (range.start, range.end))
                    .unwrap_or_else(|| {
                        let len = text.chars().count() as i32;
                        (len, len)
                    });

                let composition = ImeComposition {
                    browsing_context_id,
                    text,
                    selection_start,
                    selection_end,
                    replacement_range: replacement,
                    spans: vec![ImeTextSpan::no_decoration(
                        ImeTextSpanType::Composition,
                        0,
                        utf16_len as u32,
                    )],
                };

                if let Err(err) = self.handle.set_composition(composition) {
                    warn!("failed to send ime composition: {err}");
                }
            }
            BrowserViewMacImeEvent::CommitText {
                text,
                replacement,
                relative_caret_position,
            } => {
                let commit = ImeCommitText {
                    browsing_context_id,
                    text,
                    relative_caret_position,
                    replacement_range: replacement,
                    spans: Vec::new(),
                };
                if let Err(err) = self.handle.commit_text(commit) {
                    warn!("failed to commit ime text: {err}");
                }
            }
            BrowserViewMacImeEvent::FinishComposingText { keep_selection } => {
                let behavior = if keep_selection {
                    ConfirmCompositionBehavior::KeepSelection
                } else {
                    ConfirmCompositionBehavior::DoNotKeepSelection
                };
                if let Err(err) = self
                    .handle
                    .finish_composing_text(browsing_context_id, behavior)
                {
                    warn!("failed to finish ime composition: {err}");
                }
            }
        });
        self.with_transient_id(|transient_browsing_context_id| match transient_event {
            BrowserViewMacImeEvent::SetComposition {
                text,
                selection,
                replacement,
            } => {
                let utf16_len = text.encode_utf16().count();
                let (selection_start, selection_end) = selection
                    .map(|range| (range.start, range.end))
                    .unwrap_or_else(|| {
                        let len = text.chars().count() as i32;
                        (len, len)
                    });

                let composition = TransientImeComposition {
                    transient_browsing_context_id,
                    text,
                    selection_start,
                    selection_end,
                    replacement_range: replacement,
                    spans: vec![ImeTextSpan::no_decoration(
                        ImeTextSpanType::Composition,
                        0,
                        utf16_len as u32,
                    )],
                };

                if let Err(err) = self.handle.set_transient_composition(composition) {
                    warn!("failed to send transient ime composition: {err}");
                }
            }
            BrowserViewMacImeEvent::CommitText {
                text,
                replacement,
                relative_caret_position,
            } => {
                let commit = TransientImeCommitText {
                    transient_browsing_context_id,
                    text,
                    relative_caret_position,
                    replacement_range: replacement,
                    spans: Vec::new(),
                };
                if let Err(err) = self.handle.commit_transient_text(commit) {
                    warn!("failed to commit transient ime text: {err}");
                }
            }
            BrowserViewMacImeEvent::FinishComposingText { keep_selection } => {
                let behavior = if keep_selection {
                    ConfirmCompositionBehavior::KeepSelection
                } else {
                    ConfirmCompositionBehavior::DoNotKeepSelection
                };
                if let Err(err) = self
                    .handle
                    .finish_composing_text_in_transient_browsing_context(
                        transient_browsing_context_id,
                        behavior,
                    )
                {
                    warn!("failed to finish transient ime composition: {err}");
                }
            }
        });
    }

    fn on_char_event(&self, _view: &BrowserViewMac, text: String) {
        let transient_text = text.clone();
        self.with_page_id(|browsing_context_id| {
            let event = cbf::data::key::KeyEvent {
                type_: cbf::data::key::KeyEventType::Char,
                modifiers: 0,
                key_code: 0,
                platform_key_code: 0,
                dom_code: None,
                dom_key: None,
                text: Some(text.clone()),
                unmodified_text: Some(text),
                auto_repeat: false,
                is_keypad: false,
                is_system_key: false,
                location: 0,
            };

            if let Err(err) = self
                .handle
                .send_key_event(browsing_context_id, event, Vec::new())
            {
                warn!("failed to send char input: {err}");
            }
        });
        self.with_transient_id(|transient_browsing_context_id| {
            let event = cbf::data::key::KeyEvent {
                type_: cbf::data::key::KeyEventType::Char,
                modifiers: 0,
                key_code: 0,
                platform_key_code: 0,
                dom_code: None,
                dom_key: None,
                text: Some(transient_text.clone()),
                unmodified_text: Some(transient_text),
                auto_repeat: false,
                is_keypad: false,
                is_system_key: false,
                location: 0,
            };

            if let Err(err) = self.handle.send_key_event_to_transient_browsing_context(
                transient_browsing_context_id,
                event,
                Vec::new(),
            ) {
                warn!("failed to send transient char input: {err}");
            }
        });
    }

    fn on_mouse_event(&self, _view: &BrowserViewMac, event: cbf::data::mouse::MouseEvent) {
        let transient_event = event.clone();
        self.with_page_id(|browsing_context_id| {
            if let Err(err) = self.handle.send_mouse_event(browsing_context_id, event) {
                warn!("failed to forward mouse event: {err}");
            }
        });
        self.with_transient_id(|transient_browsing_context_id| {
            if let Err(err) = self.handle.send_mouse_event_to_transient_browsing_context(
                transient_browsing_context_id,
                transient_event,
            ) {
                warn!("failed to forward transient mouse event: {err}");
            }
        });
    }

    fn on_mouse_wheel_event(
        &self,
        _view: &BrowserViewMac,
        event: cbf::data::mouse::MouseWheelEvent,
    ) {
        let transient_event = event.clone();
        self.with_page_id(|browsing_context_id| {
            if let Err(err) = self
                .handle
                .send_mouse_wheel_event(browsing_context_id, event)
            {
                warn!("failed to forward mouse wheel event: {err}");
            }
        });
        self.with_transient_id(|transient_browsing_context_id| {
            if let Err(err) = self
                .handle
                .send_mouse_wheel_event_to_transient_browsing_context(
                    transient_browsing_context_id,
                    transient_event,
                )
            {
                warn!("failed to forward transient mouse wheel event: {err}");
            }
        });
    }

    fn on_context_menu_command(&self, _view: &BrowserViewMac, menu_id: u64, command_id: i32) {
        if let Err(err) = self
            .handle
            .execute_context_menu_command(menu_id, command_id, 0)
        {
            warn!("failed to execute context menu command: {err}");
        }
    }

    fn on_context_menu_dismissed(&self, _view: &BrowserViewMac, menu_id: u64) {
        if let Err(err) = self.handle.dismiss_context_menu(menu_id) {
            warn!("failed to dismiss context menu: {err}");
        }
    }

    fn on_choice_menu_selected(&self, _view: &BrowserViewMac, request_id: u64, indices: Vec<i32>) {
        if let Err(err) = self
            .handle
            .accept_choice_menu_selection(request_id, indices)
        {
            warn!("failed to accept choice menu selection: {err}");
        }
    }

    fn on_choice_menu_dismissed(&self, _view: &BrowserViewMac, request_id: u64) {
        if let Err(err) = self.handle.dismiss_choice_menu(request_id) {
            warn!("failed to dismiss choice menu: {err}");
        }
    }

    fn on_focus_changed(&self, _view: &BrowserViewMac, focused: bool) {
        self.with_page_id(|browsing_context_id| {
            if let Err(err) = self
                .handle
                .set_browsing_context_focus(browsing_context_id, focused)
            {
                warn!("failed to sync page focus: {err}");
            }
        });
        self.with_transient_id(|transient_browsing_context_id| {
            if let Err(err) = self
                .handle
                .set_transient_browsing_context_focus(transient_browsing_context_id, focused)
            {
                warn!("failed to sync transient focus: {err}");
            }
        });
    }

    fn on_native_drag_update(&self, _view: &BrowserViewMac, event: BrowserViewMacNativeDragUpdate) {
        self.with_page_id(|browsing_context_id| {
            let update = DragUpdate {
                session_id: event.session_id,
                browsing_context_id,
                allowed_operations: drag_allowed_operations(&self.shared, event.session_id),
                modifiers: event.modifiers,
                position_in_widget_x: event.position_in_widget_x,
                position_in_widget_y: event.position_in_widget_y,
                position_in_screen_x: event.position_in_screen_x,
                position_in_screen_y: event.position_in_screen_y,
            };

            if let Err(err) = self.handle.send_drag_update(update) {
                warn!("failed to forward native drag update: {err}");
            }
        });
    }

    fn on_native_drag_drop(&self, _view: &BrowserViewMac, event: BrowserViewMacNativeDragDrop) {
        self.with_page_id(|browsing_context_id| {
            let drop = DragDrop {
                session_id: event.session_id,
                browsing_context_id,
                modifiers: event.modifiers,
                position_in_widget_x: event.position_in_widget_x,
                position_in_widget_y: event.position_in_widget_y,
                position_in_screen_x: event.position_in_screen_x,
                position_in_screen_y: event.position_in_screen_y,
            };

            if let Err(err) = self.handle.send_drag_drop(drop) {
                warn!("failed to forward native drag drop: {err}");
            }

            remove_drag_session(&self.shared, event.session_id);
        });
    }

    fn on_native_drag_cancel(&self, _view: &BrowserViewMac, session_id: u64) {
        self.with_page_id(|browsing_context_id| {
            if let Err(err) = self
                .handle
                .send_drag_cancel(session_id, browsing_context_id)
            {
                warn!("failed to forward native drag cancel: {err}");
            }
            remove_drag_session(&self.shared, session_id);
        });
    }
}

struct WindowEntry {
    host_window_id: HostWindowId,
    window: Window,
    primary_browser_view: Retained<BrowserViewMac>,
    devtools_browser_view: Option<Retained<BrowserViewMac>>,
}

/// macOS platform-specific application implementation.
struct SimpleAppMac {
    browser_handle: BrowserHandle<ChromiumBackend>,
    shared: Arc<Mutex<SharedState>>,
    menu: Option<menu::MacMenu>,
    windows: HashMap<WinitWindowId, WindowEntry>,
    winit_id_by_host_window: HashMap<HostWindowId, WinitWindowId>,
}

impl SimpleAppMac {
    fn transient_host_window_id(
        transient_browsing_context_id: TransientBrowsingContextId,
    ) -> HostWindowId {
        HostWindowId::new((1_u64 << 63) | transient_browsing_context_id.get())
    }

    fn create_and_attach_browser_view(
        window: &Window,
        frame: CGRect,
        handle: BrowserHandle<ChromiumBackend>,
        shared: Arc<Mutex<SharedState>>,
        binding: BrowserViewBinding,
    ) -> Result<Retained<BrowserViewMac>, String> {
        let delegate = Box::new(SimpleBrowserViewDelegate {
            handle,
            shared,
            binding,
        });
        let mtm = MainThreadMarker::new()
            .ok_or_else(|| "BrowserViewMac must be created on main thread".to_owned())?;
        let browser_view = BrowserViewMac::new(mtm, BrowserViewMacConfig { frame, delegate });

        let raw = window
            .window_handle()
            .map_err(|err| format!("window handle acquisition failed: {err}"))?
            .as_raw();

        let content_view_ptr = match raw {
            RawWindowHandle::AppKit(handle) => handle.ns_view.cast::<NSView>(),
            _ => return Err("non-AppKit window handle on macOS".to_owned()),
        };
        let content_view = unsafe { content_view_ptr.as_ref() };

        content_view.addSubview(&browser_view);

        browser_view.setFrame(frame);
        browser_view.set_layer_frame(layer_frame_for_view_frame(frame));

        Ok(browser_view)
    }

    fn create_window_for_host(
        &mut self,
        event_loop: &ActiveEventLoop,
        host_window_id: HostWindowId,
        title: &str,
        size: LogicalSize<f64>,
        resizable: bool,
        binding: BrowserViewBinding,
    ) -> Result<(), String> {
        if self.winit_id_by_host_window.contains_key(&host_window_id) {
            return Ok(());
        }

        let attrs: WindowAttributes = Window::default_attributes()
            .with_title(title)
            .with_inner_size(size)
            .with_resizable(resizable);

        let window = event_loop
            .create_window(attrs)
            .map_err(|err| format!("failed to create window: {err}"))?;

        let frame = view_frame_for_window(&window);
        let browser_view = Self::create_and_attach_browser_view(
            &window,
            frame,
            self.browser_handle.clone(),
            Arc::clone(&self.shared),
            binding,
        )?;

        let winit_window_id = window.id();
        self.windows.insert(
            winit_window_id,
            WindowEntry {
                host_window_id,
                window,
                primary_browser_view: browser_view,
                devtools_browser_view: None,
            },
        );
        self.winit_id_by_host_window
            .insert(host_window_id, winit_window_id);

        Ok(())
    }

    fn close_window_for_host(&mut self, host_window_id: HostWindowId) {
        let Some(winit_id) = self.winit_id_by_host_window.remove(&host_window_id) else {
            return;
        };
        self.windows.remove(&winit_id);
    }

    fn sync_window_and_page_size(&self, core: &CoreState, host_window_id: HostWindowId) {
        let Some(winit_id) = self.winit_id_by_host_window.get(&host_window_id).copied() else {
            return;
        };
        let Some(entry) = self.windows.get(&winit_id) else {
            return;
        };

        let has_devtools =
            host_window_id == PRIMARY_HOST_WINDOW_ID && entry.devtools_browser_view.is_some();
        let (primary_frame, devtools_frame) = split_frames_for_window(&entry.window, has_devtools);
        entry.primary_browser_view.setFrame(primary_frame);
        entry
            .primary_browser_view
            .set_layer_frame(layer_frame_for_view_frame(primary_frame));

        if let (Some(devtools_view), Some(frame)) =
            (entry.devtools_browser_view.as_ref(), devtools_frame)
        {
            devtools_view.setFrame(frame);
            devtools_view.set_layer_frame(layer_frame_for_view_frame(frame));
        }

        let (primary_size, devtools_size) =
            split_page_sizes_for_window(&entry.window, has_devtools);

        if host_window_id == PRIMARY_HOST_WINDOW_ID {
            if let Some(id) = core.browsing_context_id_for_target(ViewTarget::Primary) {
                core.sync_page_size(id, primary_size.0, primary_size.1);
            }
            if let (Some(id), Some((width, height))) = (
                core.browsing_context_id_for_target(ViewTarget::DevTools),
                devtools_size,
            ) {
                core.sync_page_size(id, width, height);
            }
            return;
        }

        if transient_browsing_context_id_for_window(&self.shared, host_window_id).is_some() {
            // Chromium IPC for transient context resize is sent exclusively from the
            // ResizeTransientBrowsingContext action handler (driven by preferred size changes).
            // Sending it here too would re-send stale sizes from delayed winit Resized events,
            // causing a feedback loop.
            return;
        }

        if let Some(id) = browsing_context_id_for_window(&self.shared, host_window_id) {
            core.sync_page_size(id, primary_size.0, primary_size.1);
        }
    }

    fn ensure_devtools_view(&mut self) {
        let Some(winit_id) = self
            .winit_id_by_host_window
            .get(&PRIMARY_HOST_WINDOW_ID)
            .copied()
        else {
            return;
        };
        let Some(entry) = self.windows.get_mut(&winit_id) else {
            return;
        };
        if entry.devtools_browser_view.is_some() {
            return;
        }

        let (_, devtools_frame) = split_frames_for_window(&entry.window, true);
        let Some(devtools_frame) = devtools_frame else {
            return;
        };

        let Ok(devtools_view) = Self::create_and_attach_browser_view(
            &entry.window,
            devtools_frame,
            self.browser_handle.clone(),
            Arc::clone(&self.shared),
            BrowserViewBinding::DevTools,
        ) else {
            return;
        };

        entry.devtools_browser_view = Some(devtools_view);
    }

    fn view_for_context_id(
        &self,
        core: &CoreState,
        id: BrowsingContextId,
    ) -> Option<&BrowserViewMac> {
        if core.browsing_context_id_for_target(ViewTarget::Primary) == Some(id)
            && let Some(winit_id) = self.winit_id_by_host_window.get(&PRIMARY_HOST_WINDOW_ID)
            && let Some(entry) = self.windows.get(winit_id)
        {
            return Some(&entry.primary_browser_view);
        }
        if core.browsing_context_id_for_target(ViewTarget::DevTools) == Some(id)
            && let Some(winit_id) = self.winit_id_by_host_window.get(&PRIMARY_HOST_WINDOW_ID)
            && let Some(entry) = self.windows.get(winit_id)
        {
            return entry.devtools_browser_view.as_deref();
        }

        let host_window_id = {
            let guard = self.shared.lock().expect("shared state lock poisoned");
            guard.browsing_context_to_window.get(&id).copied()
        };
        let host_window_id = host_window_id?;
        let winit_id = self.winit_id_by_host_window.get(&host_window_id)?;
        self.windows
            .get(winit_id)
            .map(|entry| entry.primary_browser_view.as_ref())
    }

    fn view_for_transient_id(
        &self,
        transient_browsing_context_id: TransientBrowsingContextId,
    ) -> Option<&BrowserViewMac> {
        let host_window_id =
            window_id_for_transient_browsing_context(&self.shared, transient_browsing_context_id)?;
        let winit_id = self.winit_id_by_host_window.get(&host_window_id)?;
        self.windows
            .get(winit_id)
            .map(|entry| entry.primary_browser_view.as_ref())
    }

    fn create_transient_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        transient_browsing_context_id: TransientBrowsingContextId,
        title: &str,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        let host_window_id = Self::transient_host_window_id(transient_browsing_context_id);
        self.create_window_for_host(
            event_loop,
            host_window_id,
            title,
            LogicalSize::new(f64::from(width.max(25)), f64::from(height.max(25))),
            false,
            BrowserViewBinding::Transient(transient_browsing_context_id),
        )?;
        bind_transient_to_window(&self.shared, transient_browsing_context_id, host_window_id);
        Ok(())
    }

    fn ensure_host_window_for_descriptor(
        &mut self,
        event_loop: &ActiveEventLoop,
        window: WindowDescriptor,
    ) {
        if self.winit_id_by_host_window.contains_key(&window.window_id) {
            return;
        }

        let size = LogicalSize::new(
            f64::from(window.bounds.width.max(1)),
            f64::from(window.bounds.height.max(1)),
        );

        if let Err(err) = self.create_window_for_host(
            event_loop,
            window.window_id,
            "CBF SimpleApp",
            size,
            true,
            BrowserViewBinding::HostWindow(window.window_id),
        ) {
            warn!("failed to create host window {}: {err}", window.window_id);
        }
    }

    fn window_for_host_id(&self, host_window_id: HostWindowId) -> Option<&Window> {
        let winit_id = self.winit_id_by_host_window.get(&host_window_id)?;
        self.windows.get(winit_id).map(|entry| &entry.window)
    }
}

impl PlatformApp for SimpleAppMac {
    fn new(
        browser_handle: BrowserHandle<ChromiumBackend>,
        shared: Arc<Mutex<SharedState>>,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Self {
        Self {
            browser_handle,
            shared,
            menu: Some(menu::MacMenu::new(proxy).expect("failed to create macOS menu")),
            windows: HashMap::new(),
            winit_id_by_host_window: HashMap::new(),
        }
    }

    fn host_window_id_for_winit_window(&self, window_id: WinitWindowId) -> Option<HostWindowId> {
        self.windows
            .get(&window_id)
            .map(|entry| entry.host_window_id)
    }

    fn ensure_window_and_view(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        if let Some(menu) = self.menu.as_ref() {
            menu.setup();
        }

        if self
            .winit_id_by_host_window
            .contains_key(&PRIMARY_HOST_WINDOW_ID)
        {
            return Ok(());
        }

        self.create_window_for_host(
            event_loop,
            PRIMARY_HOST_WINDOW_ID,
            "CBF SimpleApp",
            LogicalSize::new(1280.0, 800.0),
            true,
            BrowserViewBinding::Primary,
        )?;
        set_primary_host_window_id(&self.shared, PRIMARY_HOST_WINDOW_ID);
        Ok(())
    }

    fn apply_core_actions(
        &mut self,
        event_loop: &ActiveEventLoop,
        core: &mut CoreState,
        actions: Vec<CoreAction>,
    ) {
        for action in actions {
            match action {
                CoreAction::ExitEventLoop => event_loop.exit(),
                CoreAction::EnsureHostWindow { window } => {
                    self.ensure_host_window_for_descriptor(event_loop, window)
                }
                CoreAction::CloseHostWindow { window_id } => self.close_window_for_host(window_id),
                CoreAction::SyncWindowAndResize { window_id } => {
                    self.sync_window_and_page_size(core, window_id)
                }
                CoreAction::SyncWindowResizeAndFocus { window_id } => {
                    self.sync_window_and_page_size(core, window_id);
                    if let Some(id) = browsing_context_id_for_window(&self.shared, window_id) {
                        core.set_page_focus(id, true);
                    }
                }
                CoreAction::UpdateWindowTitle { window_id, title } => {
                    if let Some(window) = self.window_for_host_id(window_id) {
                        window.set_title(&title);
                    }
                }
                CoreAction::UpdateCursor { window_id, cursor } => {
                    if let Some(window) = self.window_for_host_id(window_id) {
                        window.set_cursor(cursor);
                    }
                }
                CoreAction::ApplySurfaceHandle {
                    browsing_context_id,
                    handle,
                } => {
                    if let (Some(browser_view), SurfaceHandle::MacCaContextId(context_id)) =
                        (self.view_for_context_id(core, browsing_context_id), handle)
                    {
                        browser_view.set_context_id(context_id);
                    }
                }
                CoreAction::EnsureTransientHostWindow {
                    transient_browsing_context_id,
                    title,
                    width,
                    height,
                } => {
                    if let Err(err) = self.create_transient_window(
                        event_loop,
                        transient_browsing_context_id,
                        &title,
                        width,
                        height,
                    ) {
                        warn!(
                            "failed to create transient popup window {}: {err}",
                            transient_browsing_context_id
                        );
                    }
                }
                CoreAction::ResizeTransientBrowsingContext {
                    transient_browsing_context_id,
                    width,
                    height,
                } => {
                    debug!(
                        transient_browsing_context_id = %transient_browsing_context_id,
                        width,
                        height,
                        "requesting transient popup host window resize"
                    );
                    if let Some(window) = self.window_for_host_id(Self::transient_host_window_id(
                        transient_browsing_context_id,
                    )) {
                        let _ = window.request_inner_size(LogicalSize::new(
                            f64::from(width.max(25)),
                            f64::from(height.max(25)),
                        ));
                    }
                }
                CoreAction::CloseTransientHostWindow {
                    transient_browsing_context_id,
                } => {
                    self.close_window_for_host(Self::transient_host_window_id(
                        transient_browsing_context_id,
                    ));
                }
                CoreAction::ApplyTransientSurfaceHandle {
                    transient_browsing_context_id,
                    handle,
                } => {
                    if let (Some(browser_view), SurfaceHandle::MacCaContextId(context_id)) = (
                        self.view_for_transient_id(transient_browsing_context_id),
                        handle,
                    ) {
                        browser_view.set_context_id(context_id);
                    }
                }
                CoreAction::ApplyImeBounds {
                    browsing_context_id,
                    update,
                } => {
                    if let Some(browser_view) = self.view_for_context_id(core, browsing_context_id)
                    {
                        browser_view.set_ime_bounds(update);
                    }
                }
                CoreAction::ApplyTransientImeBounds {
                    transient_browsing_context_id,
                    update,
                } => {
                    if let Some(browser_view) =
                        self.view_for_transient_id(transient_browsing_context_id)
                    {
                        browser_view.set_ime_bounds(update);
                    }
                }
                CoreAction::ShowContextMenu {
                    browsing_context_id,
                    menu,
                } => {
                    if let Some(browser_view) = self.view_for_context_id(core, browsing_context_id)
                    {
                        browser_view.show_context_menu(menu);
                    }
                }
                CoreAction::ShowContextMenuInTransientBrowsingContext {
                    transient_browsing_context_id,
                    menu,
                } => {
                    if let Some(browser_view) =
                        self.view_for_transient_id(transient_browsing_context_id)
                    {
                        browser_view.show_context_menu(menu);
                    }
                }
                CoreAction::ShowChoiceMenu {
                    browsing_context_id,
                    menu,
                } => {
                    if let Some(browser_view) = self.view_for_context_id(core, browsing_context_id)
                    {
                        browser_view.show_choice_menu(menu);
                    }
                }
                CoreAction::ShowChoiceMenuInTransientBrowsingContext {
                    transient_browsing_context_id,
                    menu,
                } => {
                    if let Some(browser_view) =
                        self.view_for_transient_id(transient_browsing_context_id)
                    {
                        browser_view.show_choice_menu(menu);
                    }
                }
                CoreAction::StartPlatformDrag(request) => {
                    if let Some(browser_view) =
                        self.view_for_context_id(core, request.browsing_context_id)
                        && browser_view.start_native_drag_session(&request)
                    {
                        set_drag_allowed_operations(
                            &self.shared,
                            request.session_id,
                            request.allowed_operations,
                        );
                    }
                }
                CoreAction::SetExtensionsMenuLoading => {
                    if let Some(menu) = self.menu.as_ref() {
                        menu.show_extensions_loading();
                    }
                }
                CoreAction::ReplaceExtensionsMenu { extensions } => {
                    if let Some(menu) = self.menu.as_ref() {
                        menu.replace_extensions(&extensions);
                    }
                }
            }
        }

        if core
            .browsing_context_id_for_target(ViewTarget::DevTools)
            .is_some()
        {
            self.ensure_devtools_view();
            self.sync_window_and_page_size(core, PRIMARY_HOST_WINDOW_ID);
        }
    }
}

/// Entry point for the macOS platform.
pub fn run() {
    run_with_platform::<SimpleAppMac>();
}

fn view_frame_for_window(window: &Window) -> CGRect {
    let logical = window.inner_size().to_logical::<f64>(window.scale_factor());
    CGRect::new(CGPoint::ZERO, CGSize::new(logical.width, logical.height))
}

fn split_frames_for_window(window: &Window, with_devtools: bool) -> (CGRect, Option<CGRect>) {
    let frame = view_frame_for_window(window);
    if !with_devtools {
        return (frame, None);
    }
    let width = frame.size.width;
    let height = frame.size.height;
    let primary_width = (width * 0.5).floor();
    let devtools_width = (width - primary_width).max(1.0);

    let primary_frame = CGRect::new(
        CGPoint::new(frame.origin.x, frame.origin.y),
        CGSize::new(primary_width.max(1.0), height),
    );
    let devtools_frame = CGRect::new(
        CGPoint::new(frame.origin.x + primary_width, frame.origin.y),
        CGSize::new(devtools_width, height),
    );
    (primary_frame, Some(devtools_frame))
}

fn split_page_sizes_for_window(
    window: &Window,
    with_devtools: bool,
) -> ((u32, u32), Option<(u32, u32)>) {
    let logical = window.inner_size().to_logical::<f64>(window.scale_factor());
    if !with_devtools {
        return (
            (
                logical.width.max(1.0).round() as u32,
                logical.height.max(1.0).round() as u32,
            ),
            None,
        );
    }

    let primary_width = (logical.width * 0.5).floor().max(1.0);
    let devtools_width = (logical.width - primary_width).max(1.0);
    let height = logical.height.max(1.0).round() as u32;

    (
        (primary_width.round() as u32, height),
        Some((devtools_width.round() as u32, height)),
    )
}

fn layer_frame_for_view_frame(view_frame: CGRect) -> CGRect {
    CGRect::new(CGPoint::ZERO, view_frame.size)
}
