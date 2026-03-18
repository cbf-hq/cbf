use cbf::data::{
    ids::{BrowsingContextId, TransientBrowsingContextId},
    transient_browsing_context::TransientBrowsingContextKind,
};

/// Ownership relationship for a transient browsing context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransientOwnership {
    /// Transient browsing context tracked by the compositor.
    pub transient_browsing_context_id: TransientBrowsingContextId,
    /// Parent browsing context that owns the transient context.
    pub parent_browsing_context_id: BrowsingContextId,
    /// Backend-reported transient kind.
    pub kind: TransientBrowsingContextKind,
}
