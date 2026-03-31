use std::collections::HashMap;

use objc2_core_foundation::CGPoint;

use crate::model::{
    CompositionItemId, HitTestPolicy,
};

use super::surface_slot::SurfaceSlot;
use crate::platform::hit_test::{snapshot_contains_point, ItemLocalCssPoint};

pub(crate) fn topmost_item_at_point(
    order: &[CompositionItemId],
    slots: &HashMap<CompositionItemId, SurfaceSlot>,
    point: CGPoint,
) -> Option<CompositionItemId> {
    order.iter().copied().find(|item_id| {
        let Some(slot) = slots.get(item_id) else {
            return false;
        };

        slot.visible && slot_hit_test_contains_point(slot, point)
    })
}

pub(crate) fn slot_hit_test_contains_point(slot: &SurfaceSlot, point: CGPoint) -> bool {
    if !bounds_contains_point(slot, point) {
        return false;
    }

    match slot.hit_test {
        HitTestPolicy::Passthrough => false,
        HitTestPolicy::Bounds => true,
        HitTestPolicy::RegionSnapshot => slot.hit_test_snapshot.as_ref().is_some_and(|snapshot| {
            // Native points must be normalized before snapshot comparison.
            // Hit-test snapshots are always interpreted in item-local CSS px
            // with a top-left origin and positive y downward.
            snapshot_contains_point(snapshot, native_point_to_item_local_css_point(slot, point))
        }),
    }
}

pub(crate) fn native_point_to_item_local_css_point(
    slot: &SurfaceSlot,
    point: CGPoint,
) -> ItemLocalCssPoint {
    let local_x = point.x - slot.bounds.origin.x;
    let local_y_from_bottom = point.y - slot.bounds.origin.y;
    let local_y = slot.bounds.size.height - local_y_from_bottom;
    ItemLocalCssPoint::new(local_x, local_y)
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

    use super::{
        native_point_to_item_local_css_point, slot_hit_test_contains_point, topmost_item_at_point,
    };
    use crate::{
        model::{
            CompositionItemId, HitTestCoordinateSpace, HitTestPolicy, HitTestRegion,
            HitTestRegionMode, HitTestRegionSnapshot, SurfaceTarget,
        },
        platform::{
            hit_test::ItemLocalCssPoint, host::PlatformSurfaceHandle,
            macos::surface_slot::SurfaceSlot,
        },
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
            topmost_item_at_point(&[first, second], &slots, CGPoint::new(10.0, 10.0));
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
            topmost_item_at_point(&[first, second], &slots, CGPoint::new(10.0, 10.0));
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
            topmost_item_at_point(&[first, second], &slots, CGPoint::new(10.0, 10.0));
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
            mode: HitTestRegionMode::ConsumeListedRegions,
            regions: vec![HitTestRegion::new(10.0, 10.0, 20.0, 20.0)],
        });
        slots.insert(first, region_slot);

        let hit = topmost_item_at_point(&[first], &slots, CGPoint::new(15.0, 85.0));
        let miss = topmost_item_at_point(&[first], &slots, CGPoint::new(5.0, 95.0));

        assert_eq!(hit, Some(first));
        assert_eq!(miss, None);
    }

    #[test]
    fn topmost_item_at_point_uses_passthrough_regions_inside_bounds() {
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
            mode: HitTestRegionMode::PassthroughListedRegions,
            regions: vec![HitTestRegion::new(10.0, 10.0, 20.0, 20.0)],
        });
        slots.insert(first, region_slot);

        let passthrough = topmost_item_at_point(&[first], &slots, CGPoint::new(15.0, 85.0));
        let consume = topmost_item_at_point(&[first], &slots, CGPoint::new(5.0, 95.0));

        assert_eq!(passthrough, None);
        assert_eq!(consume, Some(first));
    }

    #[test]
    fn native_point_to_item_local_css_point_flips_y_to_top_left_origin() {
        let mut region_slot = slot(
            1,
            SurfaceTarget::BrowsingContext(cbf::data::ids::BrowsingContextId::new(10)),
            true,
        );
        region_slot.bounds = CGRect::new(CGPoint::new(40.0, 30.0), CGSize::new(100.0, 120.0));
        let point = native_point_to_item_local_css_point(&region_slot, CGPoint::new(55.0, 130.0));

        assert_eq!(point, ItemLocalCssPoint::new(15.0, 20.0));
    }

    #[test]
    fn slot_hit_test_contains_point_uses_normalized_item_local_point() {
        let mut region_slot = slot(
            1,
            SurfaceTarget::BrowsingContext(cbf::data::ids::BrowsingContextId::new(10)),
            true,
        );
        region_slot.bounds = CGRect::new(CGPoint::new(40.0, 30.0), CGSize::new(100.0, 120.0));
        region_slot.hit_test = HitTestPolicy::RegionSnapshot;
        region_slot.hit_test_snapshot = Some(HitTestRegionSnapshot {
            snapshot_id: 1,
            coordinate_space: HitTestCoordinateSpace::ItemLocalCssPx,
            mode: HitTestRegionMode::ConsumeListedRegions,
            regions: vec![HitTestRegion::new(10.0, 15.0, 20.0, 25.0)],
        });

        assert!(slot_hit_test_contains_point(
            &region_slot,
            CGPoint::new(55.0, 130.0)
        ));
        assert!(!slot_hit_test_contains_point(
            &region_slot,
            CGPoint::new(55.0, 60.0)
        ));
    }
}
