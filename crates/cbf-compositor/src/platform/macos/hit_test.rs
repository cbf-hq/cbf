use std::collections::HashMap;

use objc2_core_foundation::CGPoint;

use crate::model::CompositionItemId;

use super::surface_slot::SurfaceSlot;

pub(crate) fn topmost_item_at_point(
    order: &[CompositionItemId],
    slots: &HashMap<CompositionItemId, SurfaceSlot>,
    point: CGPoint,
) -> Option<CompositionItemId> {
    order.iter().copied().find(|item_id| {
        let Some(slot) = slots.get(item_id) else {
            return false;
        };

        slot.visible
            && slot.interactive
            && point.x >= slot.bounds.origin.x
            && point.y >= slot.bounds.origin.y
            && point.x <= slot.bounds.origin.x + slot.bounds.size.width
            && point.y <= slot.bounds.origin.y + slot.bounds.size.height
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use cbf_chrome::platform::macos::bindings::CALayerHost;
    use objc2::AnyThread;
    use objc2_core_foundation::{CGPoint, CGRect, CGSize};

    use super::topmost_item_at_point;
    use crate::{
        model::{CompositionItemId, SurfaceTarget},
        platform::{host::PlatformSurfaceHandle, macos::surface_slot::SurfaceSlot},
    };

    fn slot(item_id: u64, target: SurfaceTarget, visible: bool) -> SurfaceSlot {
        SurfaceSlot {
            target,
            layer: CALayerHost::init(CALayerHost::alloc()),
            bounds: CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(100.0, 100.0)),
            visible,
            interactive: true,
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

        let topmost = topmost_item_at_point(&[first, second], &slots, CGPoint::new(10.0, 10.0));
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

        let topmost = topmost_item_at_point(&[first, second], &slots, CGPoint::new(10.0, 10.0));
        assert_eq!(topmost, Some(second));
    }
}
