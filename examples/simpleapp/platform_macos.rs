use std::sync::{Arc, Mutex};

use cbf::{
    browser::BrowserHandle,
    data::{
        drag::{DragDrop, DragUpdate},
        ids::BrowsingContextId,
        ime::{
            ConfirmCompositionBehavior, ImeCommitText, ImeComposition, ImeTextSpan, ImeTextSpanType,
        },
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
use tracing::warn;
use winit::{
    dpi::LogicalSize,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes, WindowId},
};

use crate::{
    app::PlatformApp,
    app::run_with_platform,
    core::{
        CoreAction, CoreState, SharedState, current_browsing_context_id, drag_allowed_operations,
        remove_drag_session, set_drag_allowed_operations,
    },
};

/// Delegate for the macOS browser view that handles input events, IME, drag-and-drop, etc.
struct SimpleBrowserViewDelegate {
    handle: BrowserHandle<ChromiumBackend>,
    shared: Arc<Mutex<SharedState>>,
}

impl SimpleBrowserViewDelegate {
    fn with_page_id<F>(&self, f: F)
    where
        F: FnOnce(BrowsingContextId),
    {
        if let Some(browsing_context_id) = current_browsing_context_id(&self.shared) {
            f(browsing_context_id);
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
        self.with_page_id(|browsing_context_id| {
            if let Err(err) = self
                .handle
                .send_key_event(browsing_context_id, event, commands)
            {
                warn!("failed to forward key event: {err}");
            }
        });
    }

    fn on_ime_event(&self, _view: &BrowserViewMac, event: BrowserViewMacImeEvent) {
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
    }

    fn on_char_event(&self, _view: &BrowserViewMac, text: String) {
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
    }

    fn on_mouse_event(&self, _view: &BrowserViewMac, event: cbf::data::mouse::MouseEvent) {
        self.with_page_id(|browsing_context_id| {
            if let Err(err) = self.handle.send_mouse_event(browsing_context_id, event) {
                warn!("failed to forward mouse event: {err}");
            }
        });
    }

    fn on_mouse_wheel_event(
        &self,
        _view: &BrowserViewMac,
        event: cbf::data::mouse::MouseWheelEvent,
    ) {
        self.with_page_id(|browsing_context_id| {
            if let Err(err) = self
                .handle
                .send_mouse_wheel_event(browsing_context_id, event)
            {
                warn!("failed to forward mouse wheel event: {err}");
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

    fn on_focus_changed(&self, _view: &BrowserViewMac, focused: bool) {
        self.with_page_id(|browsing_context_id| {
            if let Err(err) = self
                .handle
                .set_browsing_context_focus(browsing_context_id, focused)
            {
                warn!("failed to sync page focus: {err}");
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

/// macOS platform-specific application implementation.
struct SimpleAppMac {
    browser_handle: BrowserHandle<ChromiumBackend>,
    shared: Arc<Mutex<SharedState>>,
    window: Option<Window>,
    window_id: Option<WindowId>,
    browser_view: Option<Retained<BrowserViewMac>>,
}

impl SimpleAppMac {
    fn create_and_attach_browser_view(
        window: &Window,
        handle: BrowserHandle<ChromiumBackend>,
        shared: Arc<Mutex<SharedState>>,
    ) -> Result<Retained<BrowserViewMac>, String> {
        let frame = view_frame_for_window(window);

        let delegate = Box::new(SimpleBrowserViewDelegate { handle, shared });
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
        browser_view.set_layer_frame(frame);

        Ok(browser_view)
    }

    fn update_view_frame(&self, window: &Window) {
        let Some(browser_view) = self.browser_view.as_ref() else {
            return;
        };

        let frame = view_frame_for_window(window);
        browser_view.setFrame(frame);
        browser_view.set_layer_frame(frame);
    }

    fn sync_view_and_page_size(&self, core: &CoreState) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        self.update_view_frame(window);
        let (width, height) = page_size_for_window(window);
        core.sync_page_size(width, height);
    }
}

impl PlatformApp for SimpleAppMac {
    fn new(
        browser_handle: BrowserHandle<ChromiumBackend>,
        shared: Arc<Mutex<SharedState>>,
    ) -> Self {
        Self {
            browser_handle,
            shared,
            window: None,
            window_id: None,
            browser_view: None,
        }
    }

    fn window_id(&self) -> Option<WindowId> {
        self.window_id
    }

    fn ensure_window_and_view(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        if self.window.is_some() {
            return Ok(());
        }

        let attrs: WindowAttributes = Window::default_attributes()
            .with_title("CBF SimpleApp")
            .with_inner_size(LogicalSize::new(1280.0, 800.0));

        let window = event_loop
            .create_window(attrs)
            .map_err(|err| format!("failed to create window: {err}"))?;

        let browser_view = Self::create_and_attach_browser_view(
            &window,
            self.browser_handle.clone(),
            Arc::clone(&self.shared),
        )?;

        self.window_id = Some(window.id());
        self.browser_view = Some(browser_view);
        self.window = Some(window);

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
                CoreAction::SyncViewAndResize => self.sync_view_and_page_size(core),
                CoreAction::SyncViewResizeAndFocus => {
                    self.sync_view_and_page_size(core);
                    core.set_page_focus(true);
                }
                CoreAction::UpdateWindowTitle(title) => {
                    if let Some(window) = self.window.as_ref() {
                        window.set_title(&title);
                    }
                }
                CoreAction::UpdateCursor(cursor) => {
                    if let Some(window) = self.window.as_ref() {
                        window.set_cursor(cursor);
                    }
                }
                CoreAction::ApplySurfaceHandle(handle) => {
                    if let (Some(browser_view), SurfaceHandle::MacCaContextId(context_id)) =
                        (self.browser_view.as_ref(), handle)
                    {
                        browser_view.set_context_id(context_id);
                    }
                }
                CoreAction::ApplyImeBounds(update) => {
                    if let Some(browser_view) = self.browser_view.as_ref() {
                        browser_view.set_ime_bounds(update);
                    }
                }
                CoreAction::ShowContextMenu(menu) => {
                    if let Some(browser_view) = self.browser_view.as_ref() {
                        browser_view.show_context_menu(menu);
                    }
                }
                CoreAction::StartPlatformDrag(request) => {
                    if let Some(browser_view) = self.browser_view.as_ref()
                        && browser_view.start_native_drag_session(&request)
                    {
                        set_drag_allowed_operations(
                            &self.shared,
                            request.session_id,
                            request.allowed_operations,
                        );
                    }
                }
            }
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

fn page_size_for_window(window: &Window) -> (u32, u32) {
    let logical = window.inner_size().to_logical::<f64>(window.scale_factor());
    (
        logical.width.max(1.0).round() as u32,
        logical.height.max(1.0).round() as u32,
    )
}
