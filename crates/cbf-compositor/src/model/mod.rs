//! Public scene-model types used by `cbf-compositor`.
//!
//! These types describe compositor windows, scene items, layout geometry, and
//! ownership relationships without exposing backend-specific details.

mod composition;
mod geometry;
mod ids;
mod ownership;
mod target;

pub use composition::{
    BackgroundPolicy, CompositionItemSpec, HitTestCoordinateSpace, HitTestPolicy, HitTestRegion,
    HitTestRegionMode, HitTestRegionSnapshot, WindowCompositionSpec,
};
pub use geometry::Rect;
pub use ids::{CompositionItemId, CompositorWindowId};
pub use ownership::TransientOwnership;
pub use target::SurfaceTarget;
