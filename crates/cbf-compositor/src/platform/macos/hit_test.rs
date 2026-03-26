use std::collections::HashMap;

use objc2_core_foundation::CGPoint;

use crate::model::{CompositionItemId, HitTestCoordinateSpace, HitTestPolicy};

use super::surface_slot::SurfaceSlot;

pub(crate) fn topmost_item_at_point(
    order: &[CompositionItemId],
    slots: &HashMap<CompositionItemId, SurfaceSlot>,
    point: CGPoint,
    view_height: f64,
) -> Option<CompositionItemId> {
    order.iter().copied().find(|item_id| {
        let Some(slot) = slots.get(item_id) else {
            return false;
        };

        slot.visible && slot_hit_test_contains_point(slot, point, view_height)
    })
}

pub(crate) fn slot_hit_test_contains_point(
    slot: &SurfaceSlot,
    point: CGPoint,
    view_height: f64,
) -> bool {
    if !bounds_contains_point(slot, point) {
        return false;
    }

    match slot.hit_test {
        HitTestPolicy::Passthrough => false,
        HitTestPolicy::Bounds => true,
        HitTestPolicy::RegionSnapshot => slot
            .hit_test_snapshot
            .as_ref()
            .is_some_and(|snapshot| match snapshot.coordinate_space {
                HitTestCoordinateSpace::ItemLocalCssPx => {
                    let local_x = point.x - slot.bounds.origin.x;
                    let local_y =
                        point.y - (view_height - (slot.bounds.origin.y + slot.bounds.size.height));
                    snapshot.regions.iter().any(|region| {
                        local_x >= f64::from(region.x)
                            && local_y >= f64::from(region.y)
                            && local_x <= f64::from(region.x + region.width)
                            && local_y <= f64::from(region.y + region.height)
                    })
                }
            }),
    }
}

fn bounds_contains_point(slot: &SurfaceSlot, point: CGPoint) -> bool {
    point.x >= slot.bounds.origin.x
        && point.y >= slot.bounds.origin.y
        && point.x <= slot.bounds.origin.x + slot.bounds.size.width
        && point.y <= slot.bounds.origin.y + slot.bounds.size.height
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use cbf_chrome::platform::macos::bindings::CALayerHost;
    use objc2::AnyThread;
    use objc2_core_foundation::{CGPoint, CGRect, CGSize};

    use super::topmost_item_at_point;
    use crate::{
        model::{
            CompositionItemId, HitTestCoordinateSpace, HitTestPolicy, HitTestRegion,
            HitTestRegionSnapshot, SurfaceTarget,
        },
        platform::{host::PlatformSurfaceHandle, macos::surface_slot::SurfaceSlot},
    };

    fn slot(item_id: u64, target: SurfaceTarget, visible: bool) -> SurfaceSlot {
        SurfaceSlot {
            target,
            layer: CALayerHost::init(CALayerHost::alloc()),
            bounds: CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(100.0, 100.0)),
            visible,
            hit_test: HitTestPolicy::Bounds,
            hit_test_snapshot: None,
            surface: Some(PlatformSurfaceHandle::MacCaContextId(item_id as u32)),
            ime_bounds: None,
        }
    }

    #[test]
    fn topmost_item_at_point_prefers_frontmost_item_in_order() {
        let first = CompositionItemId::new(1);
        let second = CompositionItemId::new(2);
        let mut slots = HashMap::new();
        slots.insert(
            first,
            slot(
                1,
                SurfaceTarget::BrowsingContext(cbf::data::ids::BrowsingContextId::new(10)),
                true,
            ),
        );
        slots.insert(
            second,
            slot(
                2,
                SurfaceTarget::BrowsingContext(cbf::data::ids::BrowsingContextId::new(20)),
                true,
            ),
        );

        let topmost =
            topmost_item_at_point(&[first, second], &slots, CGPoint::new(10.0, 10.0), 100.0);
        assert_eq!(topmost, Some(first));
    }

    #[test]
    fn topmost_item_at_point_skips_non_visible_frontmost_item() {
        let first = CompositionItemId::new(1);
        let second = CompositionItemId::new(2);
        let mut slots = HashMap::new();
        slots.insert(
            first,
            slot(
                1,
                SurfaceTarget::BrowsingContext(cbf::data::ids::BrowsingContextId::new(10)),
                false,
            ),
        );
        slots.insert(
            second,
            slot(
                2,
                SurfaceTarget::BrowsingContext(cbf::data::ids::BrowsingContextId::new(20)),
                true,
            ),
        );

        let topmost =
            topmost_item_at_point(&[first, second], &slots, CGPoint::new(10.0, 10.0), 100.0);
        assert_eq!(topmost, Some(second));
    }

    #[test]
    fn topmost_item_at_point_skips_passthrough_item() {
        let first = CompositionItemId::new(1);
        let second = CompositionItemId::new(2);
        let mut slots = HashMap::new();
        let mut front = slot(
            1,
            SurfaceTarget::BrowsingContext(cbf::data::ids::BrowsingContextId::new(10)),
            true,
        );
        front.hit_test = HitTestPolicy::Passthrough;
        slots.insert(first, front);
        slots.insert(
            second,
            slot(
                2,
                SurfaceTarget::BrowsingContext(cbf::data::ids::BrowsingContextId::new(20)),
                true,
            ),
        );

        let topmost =
            topmost_item_at_point(&[first, second], &slots, CGPoint::new(10.0, 10.0), 100.0);
        assert_eq!(topmost, Some(second));
    }

    #[test]
    fn topmost_item_at_point_uses_region_snapshot() {
        let first = CompositionItemId::new(1);
        let mut slots = HashMap::new();
        let mut region_slot = slot(
            1,
            SurfaceTarget::BrowsingContext(cbf::data::ids::BrowsingContextId::new(10)),
            true,
        );
        region_slot.hit_test = HitTestPolicy::RegionSnapshot;
        region_slot.hit_test_snapshot = Some(HitTestRegionSnapshot {
            snapshot_id: 1,
            coordinate_space: HitTestCoordinateSpace::ItemLocalCssPx,
            regions: vec![HitTestRegion::new(10.0, 10.0, 20.0, 20.0)],
        });
        slots.insert(first, region_slot);

        let hit = topmost_item_at_point(&[first], &slots, CGPoint::new(15.0, 15.0), 100.0);
        let miss = topmost_item_at_point(&[first], &slots, CGPoint::new(5.0, 5.0), 100.0);

        assert_eq!(hit, Some(first));
        assert_eq!(miss, None);
    }
}
