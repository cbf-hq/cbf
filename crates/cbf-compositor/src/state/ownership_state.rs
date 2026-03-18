use std::collections::HashMap;

use cbf::data::{
    ids::{BrowsingContextId, TransientBrowsingContextId},
    transient_browsing_context::TransientBrowsingContextKind,
};

use crate::model::TransientOwnership;

#[derive(Debug, Default)]
pub(crate) struct OwnershipState {
    transients: HashMap<TransientBrowsingContextId, TransientOwnership>,
}

impl OwnershipState {
    pub(crate) fn upsert(
        &mut self,
        transient_browsing_context_id: TransientBrowsingContextId,
        parent_browsing_context_id: BrowsingContextId,
        kind: TransientBrowsingContextKind,
    ) {
        self.transients.insert(
            transient_browsing_context_id,
            TransientOwnership {
                transient_browsing_context_id,
                parent_browsing_context_id,
                kind,
            },
        );
    }

    #[allow(dead_code)]
    pub(crate) fn get(
        &self,
        transient_browsing_context_id: TransientBrowsingContextId,
    ) -> Option<TransientOwnership> {
        self.transients.get(&transient_browsing_context_id).copied()
    }

    pub(crate) fn remove(
        &mut self,
        transient_browsing_context_id: TransientBrowsingContextId,
    ) -> Option<TransientOwnership> {
        self.transients.remove(&transient_browsing_context_id)
    }

    pub(crate) fn remove_by_parent(
        &mut self,
        parent_browsing_context_id: BrowsingContextId,
    ) -> Vec<TransientBrowsingContextId> {
        let transient_ids = self
            .transients
            .values()
            .filter_map(|ownership| {
                (ownership.parent_browsing_context_id == parent_browsing_context_id)
                    .then_some(ownership.transient_browsing_context_id)
            })
            .collect::<Vec<_>>();

        for transient_id in &transient_ids {
            self.transients.remove(transient_id);
        }

        transient_ids
    }
}

#[cfg(test)]
mod tests {
    use cbf::data::{
        ids::{BrowsingContextId, TransientBrowsingContextId},
        transient_browsing_context::TransientBrowsingContextKind,
    };

    use super::OwnershipState;

    #[test]
    fn remove_by_parent_only_removes_owned_transients() {
        let mut state = OwnershipState::default();
        state.upsert(
            TransientBrowsingContextId::new(1),
            BrowsingContextId::new(10),
            TransientBrowsingContextKind::Popup,
        );
        state.upsert(
            TransientBrowsingContextId::new(2),
            BrowsingContextId::new(11),
            TransientBrowsingContextKind::ToolWindow,
        );

        let removed = state.remove_by_parent(BrowsingContextId::new(10));

        assert_eq!(removed, vec![TransientBrowsingContextId::new(1)]);
        assert!(state.get(TransientBrowsingContextId::new(1)).is_none());
        assert!(state.get(TransientBrowsingContextId::new(2)).is_some());
    }
}
