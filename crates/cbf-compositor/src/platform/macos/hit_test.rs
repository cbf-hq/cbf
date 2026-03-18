use std::collections::HashMap;

use objc2_core_foundation::CGPoint;

use crate::model::CompositionItemId;

use super::surface_slot::SurfaceSlot;

pub(crate) fn topmost_item_at_point(
    order: &[CompositionItemId],
    slots: &HashMap<CompositionItemId, SurfaceSlot>,
    point: CGPoint,
) -> Option<CompositionItemId> {
    order.iter().rev().copied().find(|item_id| {
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
