use crate::model::{CompositionItemId, CompositorWindowId, Rect, WindowCompositionSpec};

/// Declarative scene updates applied to a compositor-managed window.
#[derive(Debug, Clone)]
pub enum CompositionCommand {
    /// Replace the entire scene contents for a window.
    SetWindowComposition {
        /// Window to update.
        window_id: CompositorWindowId,
        /// New composition snapshot for the window.
        composition: WindowCompositionSpec,
    },
    /// Update only the bounds of one existing scene item.
    UpdateItemBounds {
        /// Window that owns the item.
        window_id: CompositorWindowId,
        /// Item to move or resize.
        item_id: CompositionItemId,
        /// New bounds in window coordinates.
        bounds: Rect,
    },
    /// Update only the visibility of one existing scene item.
    SetItemVisibility {
        /// Window that owns the item.
        window_id: CompositorWindowId,
        /// Item to show or hide.
        item_id: CompositionItemId,
        /// New visibility state.
        visible: bool,
    },
    /// Remove one scene item from a window.
    RemoveItem {
        /// Window that owns the item.
        window_id: CompositorWindowId,
        /// Item to remove.
        item_id: CompositionItemId,
    },
}
