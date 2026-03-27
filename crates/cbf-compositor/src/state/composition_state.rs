use std::collections::{HashMap, HashSet};

use crate::{
    error::CompositorError,
    model::{
        CompositionItemId, CompositionItemSpec, CompositorWindowId, HitTestCoordinateSpace,
        HitTestPolicy, HitTestRegion, HitTestRegionSnapshot, Rect, SurfaceTarget,
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
    hit_test_snapshot: Option<HitTestRegionSnapshot>,
}

#[derive(Debug, Clone)]
pub(crate) struct WindowSceneItemState {
    pub(crate) spec: CompositionItemSpec,
    pub(crate) hit_test_snapshot: Option<HitTestRegionSnapshot>,
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

        let mut desired_targets = HashSet::new();
        for spec in &composition.items {
            if !desired_targets.insert(spec.target) {
                return Err(CompositorError::DuplicateSurfaceTarget);
            }
            if self
                .items
                .values()
                .any(|state| state.window_id != window_id && state.spec.target == spec.target)
            {
                return Err(CompositorError::DuplicateSurfaceTarget);
            }
        }

        let ordered_item_ids = composition
            .items
            .iter()
            .map(|item| item.item_id)
            .collect::<Vec<_>>();

        for spec in composition.items {
            let hit_test_snapshot = self.items.get(&spec.item_id).and_then(|state| {
                matches!(spec.hit_test, HitTestPolicy::RegionSnapshot)
                    .then(|| state.hit_test_snapshot.clone())
                    .flatten()
            });
            self.items.insert(
                spec.item_id,
                CompositionItemState {
                    window_id,
                    spec,
                    hit_test_snapshot,
                },
            );
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

    pub(crate) fn set_item_hit_test_regions(
        &mut self,
        window_id: CompositorWindowId,
        item_id: CompositionItemId,
        snapshot_id: u64,
        coordinate_space: HitTestCoordinateSpace,
        regions: Vec<HitTestRegion>,
    ) -> Result<bool, CompositorError> {
        let item = self.item_state_mut(window_id, item_id)?;
        if !matches!(item.spec.hit_test, HitTestPolicy::RegionSnapshot) {
            return Err(CompositorError::ItemDoesNotUseRegionHitTesting);
        }

        if item
            .hit_test_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.snapshot_id > snapshot_id)
        {
            return Ok(false);
        }

        item.hit_test_snapshot = Some(HitTestRegionSnapshot {
            snapshot_id,
            coordinate_space,
            regions,
        });
        Ok(true)
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

    pub(crate) fn window_scene_items(
        &self,
        window_id: CompositorWindowId,
    ) -> Option<Vec<WindowSceneItemState>> {
        self.windows.get(&window_id).map(|item_ids| {
            item_ids
                .iter()
                .filter_map(|item_id| {
                    self.items.get(item_id).map(|state| WindowSceneItemState {
                        spec: state.spec.clone(),
                        hit_test_snapshot: state.hit_test_snapshot.clone(),
                    })
                })
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

    pub(crate) fn item_spec(&self, item_id: CompositionItemId) -> Option<&CompositionItemSpec> {
        self.items.get(&item_id).map(|item| &item.spec)
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
    use crate::CompositorError;
    use crate::model::{
        BackgroundPolicy, CompositionItemId, CompositionItemSpec, CompositorWindowId,
        HitTestCoordinateSpace, HitTestPolicy, HitTestRegion, Rect, SurfaceTarget,
        WindowCompositionSpec,
    };

    fn item(item_id: u64, target: SurfaceTarget) -> CompositionItemSpec {
        CompositionItemSpec {
            item_id: CompositionItemId::new(item_id),
            target,
            bounds: Rect::new(1.0, 2.0, 3.0, 4.0),
            visible: true,
            hit_test: HitTestPolicy::Bounds,
            background: BackgroundPolicy::Opaque,
        }
    }

    fn region_item(item_id: u64, target: SurfaceTarget) -> CompositionItemSpec {
        CompositionItemSpec {
            hit_test: HitTestPolicy::RegionSnapshot,
            ..item(item_id, target)
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

    #[test]
    fn set_window_composition_rejects_duplicate_target_within_one_window() {
        let mut state = CompositionState::default();
        let window_id = CompositorWindowId::new(1);
        state.ensure_window(window_id);
        let target = SurfaceTarget::BrowsingContext(BrowsingContextId::new(10));

        let error = state
            .set_window_composition(
                window_id,
                WindowCompositionSpec {
                    items: vec![item(1, target), item(2, target)],
                },
            )
            .unwrap_err();

        assert!(matches!(error, CompositorError::DuplicateSurfaceTarget));
    }

    #[test]
    fn set_window_composition_rejects_duplicate_target_across_windows() {
        let mut state = CompositionState::default();
        let first_window = CompositorWindowId::new(1);
        let second_window = CompositorWindowId::new(2);
        let target = SurfaceTarget::BrowsingContext(BrowsingContextId::new(10));
        state.ensure_window(first_window);
        state.ensure_window(second_window);

        state
            .set_window_composition(
                first_window,
                WindowCompositionSpec {
                    items: vec![item(1, target)],
                },
            )
            .unwrap();

        let error = state
            .set_window_composition(
                second_window,
                WindowCompositionSpec {
                    items: vec![item(2, target)],
                },
            )
            .unwrap_err();

        assert!(matches!(error, CompositorError::DuplicateSurfaceTarget));
    }

    #[test]
    fn set_item_hit_test_regions_rejects_non_region_item() {
        let mut state = CompositionState::default();
        let window_id = CompositorWindowId::new(1);
        let item_id = CompositionItemId::new(1);
        state.ensure_window(window_id);
        state
            .set_window_composition(
                window_id,
                WindowCompositionSpec {
                    items: vec![item(
                        item_id.get(),
                        SurfaceTarget::BrowsingContext(BrowsingContextId::new(10)),
                    )],
                },
            )
            .unwrap();

        let err = state
            .set_item_hit_test_regions(
                window_id,
                item_id,
                1,
                HitTestCoordinateSpace::ItemLocalCssPx,
                vec![HitTestRegion::new(0.0, 0.0, 10.0, 10.0)],
            )
            .unwrap_err();

        assert!(matches!(
            err,
            CompositorError::ItemDoesNotUseRegionHitTesting
        ));
    }

    #[test]
    fn set_item_hit_test_regions_ignores_stale_snapshot() {
        let mut state = CompositionState::default();
        let window_id = CompositorWindowId::new(1);
        let item_id = CompositionItemId::new(1);
        state.ensure_window(window_id);
        state
            .set_window_composition(
                window_id,
                WindowCompositionSpec {
                    items: vec![region_item(
                        item_id.get(),
                        SurfaceTarget::BrowsingContext(BrowsingContextId::new(10)),
                    )],
                },
            )
            .unwrap();

        assert!(
            state
                .set_item_hit_test_regions(
                    window_id,
                    item_id,
                    2,
                    HitTestCoordinateSpace::ItemLocalCssPx,
                    vec![HitTestRegion::new(1.0, 2.0, 3.0, 4.0)],
                )
                .unwrap()
        );
        assert!(
            !state
                .set_item_hit_test_regions(
                    window_id,
                    item_id,
                    1,
                    HitTestCoordinateSpace::ItemLocalCssPx,
                    vec![HitTestRegion::new(5.0, 6.0, 7.0, 8.0)],
                )
                .unwrap()
        );

        let scene = state.window_scene_items(window_id).unwrap();
        let snapshot = scene
            .into_iter()
            .next()
            .and_then(|item| item.hit_test_snapshot)
            .expect("snapshot should exist");
        assert_eq!(snapshot.snapshot_id, 2);
        assert_eq!(
            snapshot.regions,
            vec![HitTestRegion::new(1.0, 2.0, 3.0, 4.0)]
        );
    }
}
