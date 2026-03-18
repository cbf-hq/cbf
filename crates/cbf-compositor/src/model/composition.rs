use crate::model::{geometry::Rect, ids::CompositionItemId, target::SurfaceTarget};

/// Background drawing policy for a scene item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundPolicy {
    /// The item should use a non-transparent background.
    Opaque,
    /// The item should clear its background to transparent.
    /// Currently, it is not working because it is not implemented.
    Transparent,
}

/// Declarative description of one scene item inside a compositor window.
#[derive(Debug, Clone, PartialEq)]
pub struct CompositionItemSpec {
    /// Stable identifier for this scene item.
    pub item_id: CompositionItemId,
    /// Browser-managed surface shown by this item.
    pub target: SurfaceTarget,
    /// Item bounds in compositor-window coordinates.
    pub bounds: Rect,
    /// Relative stacking order; higher values appear in front.
    pub z_index: i32,
    /// Whether the item should currently be visible.
    pub visible: bool,
    /// Whether the item should participate in hit-testing.
    pub interactive: bool,
    /// Background drawing policy for the item.
    pub background: BackgroundPolicy,
}

/// Full scene description for one compositor-managed window.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WindowCompositionSpec {
    /// Scene items to show in the window.
    pub items: Vec<CompositionItemSpec>,
}
