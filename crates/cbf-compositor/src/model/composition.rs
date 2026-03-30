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

impl From<BackgroundPolicy> for cbf::data::background::BackgroundPolicy {
    fn from(value: BackgroundPolicy) -> Self {
        match value {
            BackgroundPolicy::Opaque => Self::Opaque,
            BackgroundPolicy::Transparent => Self::Transparent,
        }
    }
}

/// Hit-test behavior for one scene item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTestPolicy {
    /// The item never receives pointer hit-tests.
    Passthrough,
    /// The item's full bounds participate in hit-testing.
    Bounds,
    /// Only the latest pushed hit-test snapshot participates in hit-testing.
    RegionSnapshot,
}

/// How a hit-test snapshot interprets its listed regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTestRegionMode {
    /// Listed regions consume pointer input and all other bounds pass through.
    ConsumeListedRegions,
    /// Listed regions pass pointer input through and all other bounds consume it.
    PassthroughListedRegions,
}

/// Coordinate space used by pushed hit-test regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTestCoordinateSpace {
    /// Item-local CSS pixel coordinates with an origin at the item's top-left.
    ItemLocalCssPx,
}

/// Axis-aligned rectangle used by hit-test snapshots.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitTestRegion {
    /// Left edge in the chosen coordinate space.
    pub x: f32,
    /// Top edge in the chosen coordinate space.
    pub y: f32,
    /// Rectangle width.
    pub width: f32,
    /// Rectangle height.
    pub height: f32,
}

impl HitTestRegion {
    /// Create a new hit-test region rectangle.
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Cached hit-test snapshot for one scene item.
#[derive(Debug, Clone, PartialEq)]
pub struct HitTestRegionSnapshot {
    /// Monotonic identifier used to discard stale async updates.
    pub snapshot_id: u64,
    /// Coordinate space for every region in this snapshot.
    pub coordinate_space: HitTestCoordinateSpace,
    /// Interpretation mode for the listed regions.
    pub mode: HitTestRegionMode,
    /// Regions interpreted according to [`Self::mode`].
    pub regions: Vec<HitTestRegion>,
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
    /// Whether the item should currently be visible.
    pub visible: bool,
    /// Hit-test behavior for the item.
    pub hit_test: HitTestPolicy,
    /// Background drawing policy for the item.
    pub background: BackgroundPolicy,
}

/// Full scene description for one compositor-managed window.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WindowCompositionSpec {
    /// Scene items to show in the window, ordered from front to back.
    pub items: Vec<CompositionItemSpec>,
}
