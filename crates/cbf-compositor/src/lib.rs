//! GUI composition integration layer for desktop applications built on CBF.
//!
//! This crate manages host window registration and declarative frame
//! composition while leaving browser connection setup, command dispatch, and
//! event loop ownership to the application.
//!
//! Frame embedding is not yet implemented in CBF. The current scaffold tracks
//! composition state and emits `CreateBrowsingContext` / close requests.
//! Chrome-specific surface synchronization is available behind the `chrome`
//! feature and is intentionally a no-op for now.

pub mod data;
pub mod error;
pub mod window;

use std::collections::{HashMap, HashSet};

use cbf::{
    command::BrowserCommand,
    data::ids::BrowsingContextId,
    event::{BrowserEvent, BrowsingContextEvent},
};
#[cfg(feature = "chrome")]
use cbf_chrome::event::ChromeEvent;

pub use data::{
    AttachWindowOptions, CompositionCommand, CompositorWindowId, DefaultRequestIdAllocator,
    FrameBounds, FrameComposition, FrameId, FrameKind, FrameSpec, IpcPolicy, Rect, RequestId,
    RequestIdAllocator, TransparencyPolicy,
};
pub use error::CompositorError;
pub use window::WindowHost;

pub struct Compositor<A = DefaultRequestIdAllocator> {
    request_ids: A,
    next_window_id: u64,
    windows: HashMap<CompositorWindowId, AttachedWindow>,
    frames: HashMap<FrameId, FrameState>,
    pending_creates: HashMap<RequestId, FrameId>,
}

struct AttachedWindow {
    _host: Box<dyn WindowHost>,
    _options: AttachWindowOptions,
}

struct FrameState {
    window_id: CompositorWindowId,
    spec: FrameSpec,
    visible: bool,
    browsing_context_id: Option<BrowsingContextId>,
}

impl Default for Compositor<DefaultRequestIdAllocator> {
    fn default() -> Self {
        Self::with_request_id_allocator(DefaultRequestIdAllocator::default())
    }
}

impl<A> Compositor<A>
where
    A: RequestIdAllocator,
{
    pub fn with_request_id_allocator(request_ids: A) -> Self {
        Self {
            request_ids,
            next_window_id: 1,
            windows: HashMap::new(),
            frames: HashMap::new(),
            pending_creates: HashMap::new(),
        }
    }

    pub fn attach_window<W>(
        &mut self,
        window: W,
        options: AttachWindowOptions,
        _emit: impl FnMut(BrowserCommand),
    ) -> Result<CompositorWindowId, CompositorError>
    where
        W: WindowHost + 'static,
    {
        let window_id = CompositorWindowId::new(self.next_window_id);
        self.next_window_id = self.next_window_id.saturating_add(1);
        self.windows.insert(
            window_id,
            AttachedWindow {
                _host: Box::new(window),
                _options: options,
            },
        );
        Ok(window_id)
    }

    pub fn detach_window(
        &mut self,
        window_id: CompositorWindowId,
        mut emit: impl FnMut(BrowserCommand),
    ) -> Result<(), CompositorError> {
        if self.windows.remove(&window_id).is_none() {
            return Err(CompositorError::UnknownWindow);
        }

        let frame_ids: Vec<_> = self
            .frames
            .iter()
            .filter_map(|(frame_id, state)| (state.window_id == window_id).then_some(*frame_id))
            .collect();

        for frame_id in frame_ids {
            self.remove_frame(frame_id, &mut emit)?;
        }

        Ok(())
    }

    pub fn apply(
        &mut self,
        command: CompositionCommand,
        mut emit: impl FnMut(BrowserCommand),
    ) -> Result<(), CompositorError> {
        match command {
            CompositionCommand::SetComposition {
                window_id,
                composition,
            } => {
                if !self.windows.contains_key(&window_id) {
                    return Err(CompositorError::UnknownWindow);
                }

                let desired_ids: HashSet<_> =
                    composition.frames.iter().map(|frame| frame.id).collect();
                let stale_frame_ids: Vec<_> = self
                    .frames
                    .iter()
                    .filter_map(|(frame_id, state)| {
                        (state.window_id == window_id && !desired_ids.contains(frame_id))
                            .then_some(*frame_id)
                    })
                    .collect();

                for frame_id in stale_frame_ids {
                    self.remove_frame(frame_id, &mut emit)?;
                }

                for spec in composition.frames {
                    let frame_id = spec.id;

                    if let Some(state) = self.frames.get_mut(&frame_id) {
                        if state.window_id != window_id {
                            return Err(CompositorError::FrameOwnedByAnotherWindow);
                        }

                        state.spec = spec;
                        state.visible = true;
                    } else {
                        self.frames.insert(
                            frame_id,
                            FrameState {
                                window_id,
                                spec,
                                visible: true,
                                browsing_context_id: None,
                            },
                        );
                    }

                    if !self.has_pending_create(frame_id) {
                        let needs_create = self
                            .frames
                            .get(&frame_id)
                            .is_some_and(|state| state.browsing_context_id.is_none());
                        if needs_create {
                            self.emit_create_for_frame(frame_id, &mut emit)?;
                        }
                    }
                }

                Ok(())
            }
            CompositionCommand::MoveFrame {
                window_id,
                frame_id,
                bounds,
            } => {
                let state = self.frame_mut_for_window(window_id, frame_id)?;
                state.spec.bounds = bounds;
                Ok(())
            }
            CompositionCommand::ShowFrame {
                window_id,
                frame_id,
            } => {
                let state = self.frame_mut_for_window(window_id, frame_id)?;
                state.visible = true;
                Ok(())
            }
            CompositionCommand::HideFrame {
                window_id,
                frame_id,
            } => {
                let state = self.frame_mut_for_window(window_id, frame_id)?;
                state.visible = false;
                Ok(())
            }
            CompositionCommand::RemoveFrame {
                window_id,
                frame_id,
            } => {
                self.frame_for_window(window_id, frame_id)?;
                self.remove_frame(frame_id, &mut emit)
            }
        }
    }

    pub fn update_browser_event(
        &mut self,
        event: &BrowserEvent,
        _emit: impl FnMut(BrowserCommand),
    ) -> Result<(), CompositorError> {
        if let BrowserEvent::BrowsingContext {
            browsing_context_id,
            event,
            ..
        } = event
        {
            match event.as_ref() {
                BrowsingContextEvent::Created { request_id } => {
                    if let Some(frame_id) = self.pending_creates.remove(request_id)
                        && let Some(frame) = self.frames.get_mut(&frame_id)
                    {
                        frame.browsing_context_id = Some(*browsing_context_id);
                    }
                }
                BrowsingContextEvent::Closed => {
                    if let Some(frame) = self
                        .frames
                        .values_mut()
                        .find(|frame| frame.browsing_context_id == Some(*browsing_context_id))
                    {
                        frame.browsing_context_id = None;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    #[cfg(feature = "chrome")]
    pub fn update_chrome_event(&mut self, _event: &ChromeEvent) -> Result<(), CompositorError> {
        // Surface-handle synchronization lands once embedded frame export exists.
        Ok(())
    }

    pub fn browsing_context_id_for_frame(&self, frame_id: FrameId) -> Option<BrowsingContextId> {
        self.frames
            .get(&frame_id)
            .and_then(|frame| frame.browsing_context_id)
    }

    fn frame_for_window(
        &self,
        window_id: CompositorWindowId,
        frame_id: FrameId,
    ) -> Result<&FrameState, CompositorError> {
        let state = self
            .frames
            .get(&frame_id)
            .ok_or(CompositorError::UnknownFrame)?;
        if state.window_id != window_id {
            return Err(CompositorError::FrameOwnedByAnotherWindow);
        }
        Ok(state)
    }

    fn frame_mut_for_window(
        &mut self,
        window_id: CompositorWindowId,
        frame_id: FrameId,
    ) -> Result<&mut FrameState, CompositorError> {
        let state = self
            .frames
            .get_mut(&frame_id)
            .ok_or(CompositorError::UnknownFrame)?;
        if state.window_id != window_id {
            return Err(CompositorError::FrameOwnedByAnotherWindow);
        }
        Ok(state)
    }

    fn remove_frame(
        &mut self,
        frame_id: FrameId,
        mut emit: impl FnMut(BrowserCommand),
    ) -> Result<(), CompositorError> {
        let state = self
            .frames
            .remove(&frame_id)
            .ok_or(CompositorError::UnknownFrame)?;
        self.pending_creates
            .retain(|_, pending_frame_id| *pending_frame_id != frame_id);

        if let Some(browsing_context_id) = state.browsing_context_id {
            emit(BrowserCommand::RequestCloseBrowsingContext {
                browsing_context_id,
            });
        }

        Ok(())
    }

    fn has_pending_create(&self, frame_id: FrameId) -> bool {
        self.pending_creates
            .values()
            .any(|pending_frame_id| *pending_frame_id == frame_id)
    }

    fn emit_create_for_frame(
        &mut self,
        frame_id: FrameId,
        mut emit: impl FnMut(BrowserCommand),
    ) -> Result<(), CompositorError> {
        let initial_url = self
            .frames
            .get(&frame_id)
            .map(|state| state.spec.url.clone())
            .ok_or(CompositorError::UnknownFrame)?;
        let request_id = self.request_ids.next_request_id();
        self.pending_creates.insert(request_id, frame_id);
        emit(BrowserCommand::CreateBrowsingContext {
            request_id,
            initial_url: Some(initial_url),
            profile_id: None,
        });
        Ok(())
    }
}
