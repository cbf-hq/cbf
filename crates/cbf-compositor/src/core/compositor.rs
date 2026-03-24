use std::collections::{HashMap, HashSet};

use cbf::{
    command::BrowserCommand,
    data::{
        background::BackgroundPolicy as GenericBackgroundPolicy, context_menu::ContextMenu,
        drag::DragStartRequest, ids::BrowsingContextId, ime::ImeBoundsUpdate,
    },
    event::{BrowserEvent, BrowsingContextEvent, TransientBrowsingContextEvent},
};
#[cfg(feature = "chrome")]
use cbf_chrome::{data::choice_menu::ChromeChoiceMenu, event::ChromeEvent};

use crate::{
    core::CompositionCommand,
    error::CompositorError,
    model::{
        BackgroundPolicy, CompositionItemId, CompositionItemSpec, CompositorWindowId, SurfaceTarget,
    },
    platform::host::{
        PlatformSceneItem, PlatformSurfaceHandle, PlatformWindowHost, attach_window_host,
    },
    state::{
        composition_state::CompositionState, focus_state::FocusState,
        ownership_state::OwnershipState, surface_state::SurfaceState,
    },
    window::WindowHost,
};

/// Scene compositor that attaches to native host windows and routes browser
/// surfaces into a declarative composition tree.
pub struct Compositor {
    next_window_id: u64,
    windows: HashMap<CompositorWindowId, AttachedWindow>,
    ownership_state: OwnershipState,
    composition_state: CompositionState,
    focus_state: FocusState,
    surface_state: SurfaceState,
}

/// Options for attaching a host window to the compositor.
#[derive(Debug, Clone, Default)]
pub struct AttachWindowOptions;

struct AttachedWindow {
    _host: Box<dyn WindowHost>,
    _options: AttachWindowOptions,
    platform_host: Box<dyn PlatformWindowHost>,
}

impl Default for Compositor {
    fn default() -> Self {
        Self::new()
    }
}

impl Compositor {
    /// Create an empty compositor with no attached windows.
    pub fn new() -> Self {
        Self {
            next_window_id: 1,
            windows: HashMap::new(),
            ownership_state: OwnershipState::default(),
            composition_state: CompositionState::default(),
            focus_state: FocusState::default(),
            surface_state: SurfaceState::default(),
        }
    }

    /// Attach a host-native window and return its compositor window id.
    pub fn attach_window<W, E>(
        &mut self,
        window: W,
        options: AttachWindowOptions,
        emit: E,
    ) -> Result<CompositorWindowId, CompositorError>
    where
        W: WindowHost + 'static,
        E: FnMut(BrowserCommand) + 'static,
    {
        let window_id = CompositorWindowId::new(self.next_window_id);
        self.next_window_id = self.next_window_id.saturating_add(1);
        let platform_host = attach_window_host(&window, emit)?;

        self.composition_state.ensure_window(window_id);
        self.windows.insert(
            window_id,
            AttachedWindow {
                _host: Box::new(window),
                _options: options,
                platform_host,
            },
        );

        Ok(window_id)
    }

    /// Detach a previously attached compositor window.
    pub fn detach_window(
        &mut self,
        window_id: CompositorWindowId,
        mut emit: impl FnMut(BrowserCommand),
    ) -> Result<(), CompositorError> {
        _ = &mut emit;
        if self.windows.remove(&window_id).is_none() {
            return Err(CompositorError::UnknownWindow);
        }

        let removed_item_ids = self.composition_state.remove_window(window_id);
        self.focus_state.clear_removed_items(&removed_item_ids);

        Ok(())
    }

    /// Apply a declarative scene update to one attached window.
    pub fn apply(
        &mut self,
        command: CompositionCommand,
        mut emit: impl FnMut(BrowserCommand),
    ) -> Result<(), CompositorError> {
        match command {
            CompositionCommand::SetWindowComposition {
                window_id,
                composition,
            } => {
                self.ensure_window(window_id)?;
                let previous_items = self
                    .composition_state
                    .items_for_window(window_id)
                    .unwrap_or_default();
                let next_items = composition.items.clone();
                let removed = self
                    .composition_state
                    .set_window_composition(window_id, composition)?;
                self.focus_state.clear_removed_items(&removed);
                self.emit_background_policy_updates(&previous_items, &next_items, &mut emit);
                self.sync_window_scene(window_id)
            }
            CompositionCommand::UpdateItemBounds {
                window_id,
                item_id,
                bounds,
            } => {
                self.ensure_window(window_id)?;
                self.composition_state
                    .update_item_bounds(window_id, item_id, bounds)?;
                self.sync_window_scene(window_id)
            }
            CompositionCommand::SetItemVisibility {
                window_id,
                item_id,
                visible,
            } => {
                self.ensure_window(window_id)?;
                self.composition_state
                    .set_item_visibility(window_id, item_id, visible)?;
                self.sync_window_scene(window_id)
            }
            CompositionCommand::RemoveItem { window_id, item_id } => {
                self.ensure_window(window_id)?;
                self.composition_state.remove_item(window_id, item_id)?;
                self.focus_state.clear_removed_items(&[item_id]);
                self.sync_window_scene(window_id)
            }
        }
    }

    /// Feed browser-generic backend events into the compositor state machine.
    pub fn update_browser_event(
        &mut self,
        event: &BrowserEvent,
        mut emit: impl FnMut(BrowserCommand),
    ) -> Result<(), CompositorError> {
        _ = &mut emit;

        match event {
            BrowserEvent::BrowsingContext {
                browsing_context_id,
                event,
                ..
            } => match event.as_ref() {
                BrowsingContextEvent::Closed => {
                    self.remove_target_and_owned_transients(
                        SurfaceTarget::BrowsingContext(*browsing_context_id),
                        *browsing_context_id,
                    )?;
                }
                BrowsingContextEvent::RenderProcessGone { .. } => {
                    self.remove_owned_transients(*browsing_context_id)?;
                }
                BrowsingContextEvent::ImeBoundsUpdated { update } => {
                    self.set_ime_bounds_for_target(
                        SurfaceTarget::BrowsingContext(*browsing_context_id),
                        update.clone(),
                    )?;
                }
                BrowsingContextEvent::ExternalDragOperationChanged { operation } => {
                    let target = SurfaceTarget::BrowsingContext(*browsing_context_id);
                    if let Some(window_id) = self.window_id_for_target(target)
                        && let Some(window) = self.windows.get_mut(&window_id)
                    {
                        window
                            .platform_host
                            .set_external_drag_operation(target, *operation)?;
                    }
                }
                _ => {}
            },
            BrowserEvent::TransientBrowsingContext {
                transient_browsing_context_id,
                parent_browsing_context_id,
                event,
                ..
            } => match event.as_ref() {
                TransientBrowsingContextEvent::Opened { kind, .. } => {
                    self.ownership_state.upsert(
                        *transient_browsing_context_id,
                        *parent_browsing_context_id,
                        *kind,
                    );
                }
                TransientBrowsingContextEvent::Resized { width, height } => {
                    self.set_transient_preferred_size(
                        *transient_browsing_context_id,
                        (*width, *height),
                    );
                }
                TransientBrowsingContextEvent::ImeBoundsUpdated { update } => {
                    self.set_ime_bounds_for_target(
                        SurfaceTarget::TransientBrowsingContext(*transient_browsing_context_id),
                        update.clone(),
                    )?;
                }
                TransientBrowsingContextEvent::Closed { .. }
                | TransientBrowsingContextEvent::RenderProcessGone { .. } => {
                    self.remove_transient(*transient_browsing_context_id)?;
                }
                _ => {}
            },
            _ => {}
        }

        Ok(())
    }

    #[cfg(feature = "chrome")]
    /// Feed Chrome-specific events into the compositor.
    pub fn update_chrome_event(&mut self, event: &ChromeEvent) -> Result<(), CompositorError> {
        crate::backend::chrome::apply_chrome_event(self, event)
    }

    #[cfg(not(feature = "chrome"))]
    /// No-op Chrome event hook when the Chrome backend feature is disabled.
    pub fn update_chrome_event(&mut self, _event: &()) -> Result<(), CompositorError> {
        Ok(())
    }

    /// Return the scene target currently displayed by an item.
    pub fn surface_target_for_item(&self, item_id: CompositionItemId) -> Option<SurfaceTarget> {
        self.composition_state.surface_target_for_item(item_id)
    }

    /// Return every item currently showing the given target.
    pub fn item_ids_for_target(&self, target: SurfaceTarget) -> Vec<CompositionItemId> {
        self.composition_state.item_ids_for_target(target)
    }

    /// Return the compositor window that owns the given item.
    pub fn window_id_for_item(&self, item_id: CompositionItemId) -> Option<CompositorWindowId> {
        self.composition_state.window_id_for_item(item_id)
    }

    /// Return the last preferred size hint reported for a transient popup.
    pub fn transient_preferred_size(
        &self,
        transient_browsing_context_id: cbf::data::ids::TransientBrowsingContextId,
    ) -> Option<(u32, u32)> {
        self.surface_state
            .get(SurfaceTarget::TransientBrowsingContext(
                transient_browsing_context_id,
            ))
            .and_then(|state| state.transient_preferred_size)
    }

    /// Present a host-owned context menu for the given surface target.
    pub fn show_context_menu(
        &mut self,
        target: SurfaceTarget,
        menu: ContextMenu,
    ) -> Result<(), CompositorError> {
        let window_id = self
            .window_id_for_target(target)
            .ok_or(CompositorError::UnknownTarget)?;
        let window = self
            .windows
            .get_mut(&window_id)
            .ok_or(CompositorError::UnknownWindow)?;
        window.platform_host.show_context_menu(target, menu)
    }

    #[cfg(feature = "chrome")]
    /// Present a host-owned choice menu for the given surface target.
    pub fn show_choice_menu(
        &mut self,
        target: SurfaceTarget,
        menu: ChromeChoiceMenu,
    ) -> Result<(), CompositorError> {
        let window_id = self
            .window_id_for_target(target)
            .ok_or(CompositorError::UnknownTarget)?;
        let window = self
            .windows
            .get_mut(&window_id)
            .ok_or(CompositorError::UnknownWindow)?;
        window.platform_host.show_choice_menu(target, menu)
    }

    /// Start a host-owned native drag session from a browsing-context target.
    pub fn start_native_drag(
        &mut self,
        request: DragStartRequest,
    ) -> Result<bool, CompositorError> {
        let target = SurfaceTarget::BrowsingContext(request.browsing_context_id);
        let window_id = self
            .window_id_for_target(target)
            .ok_or(CompositorError::UnknownTarget)?;
        let window = self
            .windows
            .get_mut(&window_id)
            .ok_or(CompositorError::UnknownWindow)?;
        window.platform_host.start_native_drag(target, request)
    }

    pub(crate) fn set_surface_handle_for_target(
        &mut self,
        target: SurfaceTarget,
        handle: PlatformSurfaceHandle,
    ) -> Result<(), CompositorError> {
        self.surface_state.set_surface(target, handle);
        for window_id in self.composition_state.window_ids_for_target(target) {
            self.sync_window_scene(window_id)?;
        }
        Ok(())
    }

    pub(crate) fn set_transient_preferred_size(
        &mut self,
        transient_browsing_context_id: cbf::data::ids::TransientBrowsingContextId,
        size: (u32, u32),
    ) {
        self.surface_state.set_transient_preferred_size(
            SurfaceTarget::TransientBrowsingContext(transient_browsing_context_id),
            size,
        );
    }

    fn emit_background_policy_updates(
        &self,
        previous_items: &[CompositionItemSpec],
        next_items: &[CompositionItemSpec],
        emit: &mut impl FnMut(BrowserCommand),
    ) {
        let previous = previous_items
            .iter()
            .map(|item| (item.target, item.background))
            .collect::<HashMap<_, _>>();
        let next = next_items
            .iter()
            .map(|item| (item.target, item.background))
            .collect::<HashMap<_, _>>();

        let mut targets = previous.keys().copied().collect::<HashSet<_>>();
        targets.extend(next.keys().copied());

        for target in targets {
            let Some(next_policy) = next.get(&target).copied() else {
                continue;
            };
            if previous.get(&target).copied() == Some(next_policy) {
                continue;
            }
            self.emit_background_policy_command(target, next_policy, emit);
        }
    }

    fn emit_background_policy_command(
        &self,
        target: SurfaceTarget,
        policy: BackgroundPolicy,
        emit: &mut impl FnMut(BrowserCommand),
    ) {
        let policy: GenericBackgroundPolicy = policy.into();
        match target {
            SurfaceTarget::BrowsingContext(browsing_context_id) => {
                emit(BrowserCommand::SetBrowsingContextBackgroundPolicy {
                    browsing_context_id,
                    policy,
                });
            }
            SurfaceTarget::TransientBrowsingContext(transient_browsing_context_id) => {
                emit(
                    BrowserCommand::SetTransientBrowsingContextBackgroundPolicy {
                        transient_browsing_context_id,
                        policy,
                    },
                );
            }
        }
    }

    fn ensure_window(&self, window_id: CompositorWindowId) -> Result<(), CompositorError> {
        if self.windows.contains_key(&window_id) {
            Ok(())
        } else {
            Err(CompositorError::UnknownWindow)
        }
    }

    fn window_id_for_target(&self, target: SurfaceTarget) -> Option<CompositorWindowId> {
        self.focus_state
            .active_item_id
            .and_then(|item_id| {
                (self.composition_state.surface_target_for_item(item_id) == Some(target))
                    .then_some(item_id)
            })
            .and_then(|item_id| self.composition_state.window_id_for_item(item_id))
            .or_else(|| {
                self.composition_state
                    .window_ids_for_target(target)
                    .into_iter()
                    .next()
            })
    }

    fn sync_window_scene(&mut self, window_id: CompositorWindowId) -> Result<(), CompositorError> {
        let scene = self
            .composition_state
            .items_for_window(window_id)
            .ok_or(CompositorError::UnknownWindow)?
            .into_iter()
            .map(|spec| {
                let runtime_state = self.surface_state.get(spec.target);
                PlatformSceneItem {
                    item_id: spec.item_id,
                    target: spec.target,
                    bounds: spec.bounds,
                    visible: spec.visible,
                    interactive: spec.interactive,
                    surface: runtime_state.and_then(|state| state.surface.clone()),
                    ime_bounds: runtime_state.and_then(|state| state.ime_bounds.clone()),
                }
            })
            .collect::<Vec<_>>();

        self.windows
            .get_mut(&window_id)
            .ok_or(CompositorError::UnknownWindow)?
            .platform_host
            .sync_scene(&scene)
    }

    fn remove_target_and_owned_transients(
        &mut self,
        target: SurfaceTarget,
        parent_browsing_context_id: BrowsingContextId,
    ) -> Result<(), CompositorError> {
        let removed = self.composition_state.remove_target(target);
        self.focus_state
            .clear_removed_items(&removed.removed_item_ids);
        self.surface_state.remove(&target);

        for transient_id in self
            .ownership_state
            .remove_by_parent(parent_browsing_context_id)
        {
            let transient_target = SurfaceTarget::TransientBrowsingContext(transient_id);
            let removed = self.composition_state.remove_target(transient_target);
            self.focus_state
                .clear_removed_items(&removed.removed_item_ids);
            self.surface_state.remove(&transient_target);

            for window_id in removed.affected_windows {
                self.sync_window_scene(window_id)?;
            }
        }

        for window_id in removed.affected_windows {
            self.sync_window_scene(window_id)?;
        }

        Ok(())
    }

    fn remove_owned_transients(
        &mut self,
        parent_browsing_context_id: BrowsingContextId,
    ) -> Result<(), CompositorError> {
        for transient_id in self
            .ownership_state
            .remove_by_parent(parent_browsing_context_id)
        {
            let transient_target = SurfaceTarget::TransientBrowsingContext(transient_id);
            let removed = self.composition_state.remove_target(transient_target);
            self.focus_state
                .clear_removed_items(&removed.removed_item_ids);
            self.surface_state.remove(&transient_target);

            for window_id in removed.affected_windows {
                self.sync_window_scene(window_id)?;
            }
        }
        Ok(())
    }

    fn remove_transient(
        &mut self,
        transient_browsing_context_id: cbf::data::ids::TransientBrowsingContextId,
    ) -> Result<(), CompositorError> {
        self.ownership_state.remove(transient_browsing_context_id);
        let target = SurfaceTarget::TransientBrowsingContext(transient_browsing_context_id);
        let removed = self.composition_state.remove_target(target);
        self.focus_state
            .clear_removed_items(&removed.removed_item_ids);
        self.surface_state.remove(&target);
        for window_id in removed.affected_windows {
            self.sync_window_scene(window_id)?;
        }
        Ok(())
    }

    fn set_ime_bounds_for_target(
        &mut self,
        target: SurfaceTarget,
        update: ImeBoundsUpdate,
    ) -> Result<(), CompositorError> {
        self.surface_state.set_ime_bounds(target, update);
        for window_id in self.composition_state.window_ids_for_target(target) {
            self.sync_window_scene(window_id)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn attach_test_window(
        &mut self,
        window_id: CompositorWindowId,
        platform_host: Box<dyn PlatformWindowHost>,
    ) {
        self.composition_state.ensure_window(window_id);
        self.windows.insert(
            window_id,
            AttachedWindow {
                _host: Box::new(crate::core::compositor::tests::TestWindowHost),
                _options: AttachWindowOptions,
                platform_host,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use cbf::data::background::BackgroundPolicy as GenericBackgroundPolicy;
    use raw_window_handle::{
        AppKitDisplayHandle, AppKitWindowHandle, DisplayHandle, HandleError, HasDisplayHandle,
        HasWindowHandle, WindowHandle,
    };

    use super::*;
    use crate::{
        model::{
            BackgroundPolicy, CompositionItemId, CompositionItemSpec, Rect, WindowCompositionSpec,
        },
        platform::host::{PlatformInputState, PlatformSceneItem},
    };

    #[derive(Default)]
    pub(crate) struct TestWindowHost;

    impl HasWindowHandle for TestWindowHost {
        fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
            let raw = AppKitWindowHandle::new(core::ptr::NonNull::dangling());
            Ok(unsafe { WindowHandle::borrow_raw(raw.into()) })
        }
    }

    impl HasDisplayHandle for TestWindowHost {
        fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
            Ok(unsafe { DisplayHandle::borrow_raw(AppKitDisplayHandle::new().into()) })
        }
    }

    impl WindowHost for TestWindowHost {
        fn inner_size(&self) -> (u32, u32) {
            (800, 600)
        }
    }

    struct TestPlatformHost {
        last_scene: Rc<RefCell<Vec<PlatformSceneItem>>>,
    }

    impl Default for TestPlatformHost {
        fn default() -> Self {
            Self {
                last_scene: Rc::new(RefCell::new(Vec::new())),
            }
        }
    }

    impl PlatformWindowHost for TestPlatformHost {
        fn sync_scene(&mut self, items: &[PlatformSceneItem]) -> Result<(), CompositorError> {
            self.last_scene.replace(items.to_vec());
            Ok(())
        }

        fn show_context_menu(
            &mut self,
            _target: SurfaceTarget,
            _menu: cbf::data::context_menu::ContextMenu,
        ) -> Result<(), CompositorError> {
            Ok(())
        }

        #[cfg(feature = "chrome")]
        fn show_choice_menu(
            &mut self,
            _target: SurfaceTarget,
            _menu: cbf_chrome::data::choice_menu::ChromeChoiceMenu,
        ) -> Result<(), CompositorError> {
            Ok(())
        }

        fn start_native_drag(
            &mut self,
            _target: SurfaceTarget,
            _request: cbf::data::drag::DragStartRequest,
        ) -> Result<bool, CompositorError> {
            Ok(false)
        }

        fn input_state(&self) -> PlatformInputState {
            PlatformInputState::default()
        }
    }

    fn item(item_id: u64, target: SurfaceTarget) -> CompositionItemSpec {
        CompositionItemSpec {
            item_id: CompositionItemId::new(item_id),
            target,
            bounds: Rect::new(0.0, 0.0, 100.0, 100.0),
            visible: true,
            interactive: true,
            background: BackgroundPolicy::Opaque,
        }
    }

    fn transparent_item(item_id: u64, target: SurfaceTarget) -> CompositionItemSpec {
        CompositionItemSpec {
            background: BackgroundPolicy::Transparent,
            ..item(item_id, target)
        }
    }

    #[test]
    fn parent_close_removes_transient_items_across_windows() {
        let mut compositor = Compositor::new();
        let first_window = CompositorWindowId::new(1);
        let second_window = CompositorWindowId::new(2);
        compositor.attach_test_window(first_window, Box::<TestPlatformHost>::default());
        compositor.attach_test_window(second_window, Box::<TestPlatformHost>::default());

        let parent_id = BrowsingContextId::new(10);
        let transient_id = cbf::data::ids::TransientBrowsingContextId::new(20);
        compositor.ownership_state.upsert(
            transient_id,
            parent_id,
            cbf::data::transient_browsing_context::TransientBrowsingContextKind::Popup,
        );
        compositor
            .composition_state
            .set_window_composition(
                first_window,
                WindowCompositionSpec {
                    items: vec![item(1, SurfaceTarget::BrowsingContext(parent_id))],
                },
            )
            .unwrap();
        compositor
            .composition_state
            .set_window_composition(
                second_window,
                WindowCompositionSpec {
                    items: vec![item(
                        2,
                        SurfaceTarget::TransientBrowsingContext(transient_id),
                    )],
                },
            )
            .unwrap();

        compositor
            .update_browser_event(
                &BrowserEvent::BrowsingContext {
                    profile_id: "p".into(),
                    browsing_context_id: parent_id,
                    event: Box::new(BrowsingContextEvent::Closed),
                },
                |_| {},
            )
            .unwrap();

        assert!(
            compositor
                .item_ids_for_target(SurfaceTarget::BrowsingContext(parent_id))
                .is_empty()
        );
        assert!(
            compositor
                .item_ids_for_target(SurfaceTarget::TransientBrowsingContext(transient_id))
                .is_empty()
        );
        assert!(compositor.ownership_state.get(transient_id).is_none());
    }

    #[test]
    fn sync_window_scene_preserves_front_to_back_item_order() {
        let mut compositor = Compositor::new();
        let window_id = CompositorWindowId::new(1);
        let host = TestPlatformHost::default();
        let scene_log = Rc::clone(&host.last_scene);
        compositor.attach_test_window(window_id, Box::new(host));

        compositor
            .composition_state
            .set_window_composition(
                window_id,
                WindowCompositionSpec {
                    items: vec![
                        item(
                            3,
                            SurfaceTarget::BrowsingContext(BrowsingContextId::new(30)),
                        ),
                        item(
                            1,
                            SurfaceTarget::BrowsingContext(BrowsingContextId::new(10)),
                        ),
                        item(
                            2,
                            SurfaceTarget::BrowsingContext(BrowsingContextId::new(20)),
                        ),
                    ],
                },
            )
            .unwrap();

        compositor.sync_window_scene(window_id).unwrap();

        let scene = scene_log.borrow();
        let ordered_ids = scene.iter().map(|item| item.item_id).collect::<Vec<_>>();
        assert_eq!(
            ordered_ids,
            vec![
                CompositionItemId::new(3),
                CompositionItemId::new(1),
                CompositionItemId::new(2),
            ]
        );
    }

    #[test]
    fn set_window_composition_rejects_duplicate_target_across_windows() {
        let mut compositor = Compositor::new();
        let first_window = CompositorWindowId::new(1);
        let second_window = CompositorWindowId::new(2);
        let target = SurfaceTarget::BrowsingContext(BrowsingContextId::new(10));
        compositor.attach_test_window(first_window, Box::<TestPlatformHost>::default());
        compositor.attach_test_window(second_window, Box::<TestPlatformHost>::default());

        compositor
            .apply(
                CompositionCommand::SetWindowComposition {
                    window_id: first_window,
                    composition: WindowCompositionSpec {
                        items: vec![item(1, target)],
                    },
                },
                |_| {},
            )
            .unwrap();

        let error = compositor
            .apply(
                CompositionCommand::SetWindowComposition {
                    window_id: second_window,
                    composition: WindowCompositionSpec {
                        items: vec![item(2, target)],
                    },
                },
                |_| {},
            )
            .unwrap_err();

        assert!(matches!(error, CompositorError::DuplicateSurfaceTarget));
    }

    #[test]
    fn set_window_composition_emits_background_policy_commands_only_for_changes() {
        let mut compositor = Compositor::new();
        let window_id = CompositorWindowId::new(1);
        let target = SurfaceTarget::BrowsingContext(BrowsingContextId::new(10));
        compositor.attach_test_window(window_id, Box::<TestPlatformHost>::default());

        let emitted = Rc::new(RefCell::new(Vec::new()));
        compositor
            .apply(
                CompositionCommand::SetWindowComposition {
                    window_id,
                    composition: WindowCompositionSpec {
                        items: vec![transparent_item(1, target)],
                    },
                },
                {
                    let emitted = Rc::clone(&emitted);
                    move |command| emitted.borrow_mut().push(command)
                },
            )
            .unwrap();

        compositor
            .apply(
                CompositionCommand::SetWindowComposition {
                    window_id,
                    composition: WindowCompositionSpec {
                        items: vec![transparent_item(1, target)],
                    },
                },
                {
                    let emitted = Rc::clone(&emitted);
                    move |command| emitted.borrow_mut().push(command)
                },
            )
            .unwrap();

        compositor
            .apply(
                CompositionCommand::SetWindowComposition {
                    window_id,
                    composition: WindowCompositionSpec {
                        items: vec![item(1, target)],
                    },
                },
                {
                    let emitted = Rc::clone(&emitted);
                    move |command| emitted.borrow_mut().push(command)
                },
            )
            .unwrap();

        let emitted = emitted.take();
        assert_eq!(emitted.len(), 2);
        assert!(matches!(
            emitted.first(),
            Some(BrowserCommand::SetBrowsingContextBackgroundPolicy {
                browsing_context_id,
                policy: GenericBackgroundPolicy::Transparent,
            }) if *browsing_context_id == BrowsingContextId::new(10)
        ));
        assert!(matches!(
            emitted.get(1),
            Some(BrowserCommand::SetBrowsingContextBackgroundPolicy {
                browsing_context_id,
                policy: GenericBackgroundPolicy::Opaque,
            }) if *browsing_context_id == BrowsingContextId::new(10)
        ));
    }

    #[test]
    fn set_window_composition_emits_transient_background_policy_command() {
        let mut compositor = Compositor::new();
        let window_id = CompositorWindowId::new(1);
        let target = SurfaceTarget::TransientBrowsingContext(
            cbf::data::ids::TransientBrowsingContextId::new(20),
        );
        compositor.attach_test_window(window_id, Box::<TestPlatformHost>::default());

        let emitted = Rc::new(RefCell::new(Vec::new()));
        compositor
            .apply(
                CompositionCommand::SetWindowComposition {
                    window_id,
                    composition: WindowCompositionSpec {
                        items: vec![transparent_item(1, target)],
                    },
                },
                {
                    let emitted = Rc::clone(&emitted);
                    move |command| emitted.borrow_mut().push(command)
                },
            )
            .unwrap();

        let emitted = emitted.take();
        assert_eq!(emitted.len(), 1);
        assert!(matches!(
            emitted.first(),
            Some(BrowserCommand::SetTransientBrowsingContextBackgroundPolicy {
                transient_browsing_context_id,
                policy: GenericBackgroundPolicy::Transparent,
            }) if *transient_browsing_context_id
                == cbf::data::ids::TransientBrowsingContextId::new(20)
        ));
    }
}
