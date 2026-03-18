use std::collections::{HashMap, HashSet};

use crate::{
    error::CompositorError,
    model::{
        CompositionItemId, CompositionItemSpec, CompositorWindowId, Rect, SurfaceTarget,
        WindowCompositionSpec,
    },
};

#[derive(Debug, Default)]
pub(crate) struct CompositionState {
    windows: HashMap<CompositorWindowId, Vec<CompositionItemId>>,
    items: HashMap<CompositionItemId, CompositionItemState>,
}

#[derive(Debug, Clone)]
struct CompositionItemState {
    window_id: CompositorWindowId,
    spec: CompositionItemSpec,
}

#[derive(Debug, Default)]
pub(crate) struct RemovedItems {
    pub(crate) removed_item_ids: Vec<CompositionItemId>,
    pub(crate) affected_windows: Vec<CompositorWindowId>,
}

impl CompositionState {
    pub(crate) fn ensure_window(&mut self, window_id: CompositorWindowId) {
        self.windows.entry(window_id).or_default();
    }

    pub(crate) fn remove_window(
        &mut self,
        window_id: CompositorWindowId,
    ) -> Vec<CompositionItemId> {
        let item_ids = self.windows.remove(&window_id).unwrap_or_default();
        for item_id in &item_ids {
            self.items.remove(item_id);
        }
        item_ids
    }

    pub(crate) fn set_window_composition(
        &mut self,
        window_id: CompositorWindowId,
        composition: WindowCompositionSpec,
    ) -> Result<Vec<CompositionItemId>, CompositorError> {
        self.ensure_window(window_id);

        let desired_item_ids = composition
            .items
            .iter()
            .map(|item| item.item_id)
            .collect::<HashSet<_>>();
        let current_item_ids = self.windows.get(&window_id).cloned().unwrap_or_default();
        let removed_item_ids = current_item_ids
            .iter()
            .copied()
            .filter(|item_id| !desired_item_ids.contains(item_id))
            .collect::<Vec<_>>();

        for item_id in &removed_item_ids {
            self.items.remove(item_id);
        }

        for spec in &composition.items {
            if let Some(existing) = self.items.get(&spec.item_id)
                && existing.window_id != window_id
            {
                return Err(CompositorError::ItemOwnedByAnotherWindow);
            }
        }

        let ordered_item_ids = composition
            .items
            .iter()
            .map(|item| item.item_id)
            .collect::<Vec<_>>();

        for spec in composition.items {
            self.items
                .insert(spec.item_id, CompositionItemState { window_id, spec });
        }

        self.windows.insert(window_id, ordered_item_ids);
        Ok(removed_item_ids)
    }

    pub(crate) fn update_item_bounds(
        &mut self,
        window_id: CompositorWindowId,
        item_id: CompositionItemId,
        bounds: Rect,
    ) -> Result<(), CompositorError> {
        let item = self.item_state_mut(window_id, item_id)?;
        item.spec.bounds = bounds;
        Ok(())
    }

    pub(crate) fn set_item_visibility(
        &mut self,
        window_id: CompositorWindowId,
        item_id: CompositionItemId,
        visible: bool,
    ) -> Result<(), CompositorError> {
        let item = self.item_state_mut(window_id, item_id)?;
        item.spec.visible = visible;
        Ok(())
    }

    pub(crate) fn remove_item(
        &mut self,
        window_id: CompositorWindowId,
        item_id: CompositionItemId,
    ) -> Result<(), CompositorError> {
        self.item_state(window_id, item_id)?;
        self.items.remove(&item_id);
        if let Some(item_ids) = self.windows.get_mut(&window_id) {
            item_ids.retain(|candidate| *candidate != item_id);
        }
        Ok(())
    }

    pub(crate) fn remove_target(&mut self, target: SurfaceTarget) -> RemovedItems {
        let matching_item_ids = self
            .items
            .iter()
            .filter_map(|(item_id, state)| (state.spec.target == target).then_some(*item_id))
            .collect::<Vec<_>>();

        let mut affected_windows = Vec::new();
        for item_id in &matching_item_ids {
            if let Some(state) = self.items.remove(item_id) {
                if let Some(item_ids) = self.windows.get_mut(&state.window_id) {
                    item_ids.retain(|candidate| candidate != item_id);
                }
                if !affected_windows.contains(&state.window_id) {
                    affected_windows.push(state.window_id);
                }
            }
        }

        RemovedItems {
            removed_item_ids: matching_item_ids,
            affected_windows,
        }
    }

    pub(crate) fn items_for_window(
        &self,
        window_id: CompositorWindowId,
    ) -> Option<Vec<CompositionItemSpec>> {
        self.windows.get(&window_id).map(|item_ids| {
            item_ids
                .iter()
                .filter_map(|item_id| self.items.get(item_id).map(|state| state.spec.clone()))
                .collect()
        })
    }

    pub(crate) fn surface_target_for_item(
        &self,
        item_id: CompositionItemId,
    ) -> Option<SurfaceTarget> {
        self.items.get(&item_id).map(|item| item.spec.target)
    }

    pub(crate) fn item_ids_for_target(&self, target: SurfaceTarget) -> Vec<CompositionItemId> {
        self.items
            .iter()
            .filter_map(|(item_id, state)| (state.spec.target == target).then_some(*item_id))
            .collect()
    }

    pub(crate) fn window_id_for_item(
        &self,
        item_id: CompositionItemId,
    ) -> Option<CompositorWindowId> {
        self.items.get(&item_id).map(|item| item.window_id)
    }

    pub(crate) fn window_ids_for_target(&self, target: SurfaceTarget) -> Vec<CompositorWindowId> {
        let mut window_ids = Vec::new();
        for state in self.items.values() {
            if state.spec.target == target && !window_ids.contains(&state.window_id) {
                window_ids.push(state.window_id);
            }
        }
        window_ids
    }

    fn item_state(
        &self,
        window_id: CompositorWindowId,
        item_id: CompositionItemId,
    ) -> Result<&CompositionItemState, CompositorError> {
        let item = self
            .items
            .get(&item_id)
            .ok_or(CompositorError::UnknownItem)?;
        if item.window_id == window_id {
            Ok(item)
        } else {
            Err(CompositorError::UnknownItem)
        }
    }

    fn item_state_mut(
        &mut self,
        window_id: CompositorWindowId,
        item_id: CompositionItemId,
    ) -> Result<&mut CompositionItemState, CompositorError> {
        let item = self
            .items
            .get_mut(&item_id)
            .ok_or(CompositorError::UnknownItem)?;
        if item.window_id == window_id {
            Ok(item)
        } else {
            Err(CompositorError::UnknownItem)
        }
    }
}

#[cfg(test)]
mod tests {
    use cbf::data::ids::{BrowsingContextId, TransientBrowsingContextId};

    use super::CompositionState;
    use crate::model::{
        BackgroundPolicy, CompositionItemId, CompositionItemSpec, CompositorWindowId, Rect,
        SurfaceTarget, WindowCompositionSpec,
    };

    fn item(item_id: u64, target: SurfaceTarget) -> CompositionItemSpec {
        CompositionItemSpec {
            item_id: CompositionItemId::new(item_id),
            target,
            bounds: Rect::new(1.0, 2.0, 3.0, 4.0),
            visible: true,
            interactive: true,
            background: BackgroundPolicy::Opaque,
        }
    }

    #[test]
    fn composition_and_target_identity_are_independent() {
        let mut state = CompositionState::default();
        let window_id = CompositorWindowId::new(1);
        state.ensure_window(window_id);
        state
            .set_window_composition(
                window_id,
                WindowCompositionSpec {
                    items: vec![
                        item(
                            1,
                            SurfaceTarget::BrowsingContext(BrowsingContextId::new(10)),
                        ),
                        item(
                            2,
                            SurfaceTarget::TransientBrowsingContext(
                                TransientBrowsingContextId::new(20),
                            ),
                        ),
                    ],
                },
            )
            .unwrap();

        assert_eq!(
            state.surface_target_for_item(CompositionItemId::new(1)),
            Some(SurfaceTarget::BrowsingContext(BrowsingContextId::new(10)))
        );
        assert_eq!(
            state.surface_target_for_item(CompositionItemId::new(2)),
            Some(SurfaceTarget::TransientBrowsingContext(
                TransientBrowsingContextId::new(20)
            ))
        );
    }

    #[test]
    fn set_window_composition_preserves_front_to_back_input_order() {
        let mut state = CompositionState::default();
        let window_id = CompositorWindowId::new(1);
        state.ensure_window(window_id);
        state
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

        let ordered_ids = state
            .items_for_window(window_id)
            .unwrap()
            .into_iter()
            .map(|item| item.item_id)
            .collect::<Vec<_>>();
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
    fn set_window_composition_replaces_order_with_latest_input_order() {
        let mut state = CompositionState::default();
        let window_id = CompositorWindowId::new(1);
        state.ensure_window(window_id);
        state
            .set_window_composition(
                window_id,
                WindowCompositionSpec {
                    items: vec![
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

        state
            .set_window_composition(
                window_id,
                WindowCompositionSpec {
                    items: vec![
                        item(
                            2,
                            SurfaceTarget::BrowsingContext(BrowsingContextId::new(20)),
                        ),
                        item(
                            1,
                            SurfaceTarget::BrowsingContext(BrowsingContextId::new(10)),
                        ),
                    ],
                },
            )
            .unwrap();

        let ordered_ids = state
            .items_for_window(window_id)
            .unwrap()
            .into_iter()
            .map(|item| item.item_id)
            .collect::<Vec<_>>();
        assert_eq!(
            ordered_ids,
            vec![CompositionItemId::new(2), CompositionItemId::new(1)]
        );
    }
}
