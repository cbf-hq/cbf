use cbf::data::ids::{BrowsingContextId, TransientBrowsingContextId};

/// Browser-managed surface target that can appear in the compositor scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceTarget {
    /// A regular browsing context surface.
    BrowsingContext(BrowsingContextId),
    /// A transient browsing context surface such as an extension popup.
    TransientBrowsingContext(TransientBrowsingContextId),
}
