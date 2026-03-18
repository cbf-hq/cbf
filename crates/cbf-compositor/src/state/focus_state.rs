use crate::model::CompositionItemId;

#[derive(Debug, Default)]
pub(crate) struct FocusState {
    pub(crate) active_item_id: Option<CompositionItemId>,
    pub(crate) pointer_capture_item_id: Option<CompositionItemId>,
}

impl FocusState {
    pub(crate) fn clear_removed_items(&mut self, removed_item_ids: &[CompositionItemId]) {
        if let Some(active_item_id) = self.active_item_id
            && removed_item_ids.contains(&active_item_id)
        {
            self.active_item_id = None;
        }

        if let Some(pointer_capture_item_id) = self.pointer_capture_item_id
            && removed_item_ids.contains(&pointer_capture_item_id)
        {
            self.pointer_capture_item_id = None;
        }
    }
}
