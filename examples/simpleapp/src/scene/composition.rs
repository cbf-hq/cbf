use cbf::data::ids::{BrowsingContextId, TransientBrowsingContextId};
use cbf_compositor::model::{
    BackgroundPolicy, CompositionItemId, CompositionItemSpec, SurfaceTarget, WindowCompositionSpec,
};

use crate::scene::layout::{full_window_rect, main_page_rect, main_toolbar_rect};

pub(crate) const MAIN_TOOLBAR_ITEM_ID: CompositionItemId = CompositionItemId::new(1);
pub(crate) const MAIN_PAGE_ITEM_ID: CompositionItemId = CompositionItemId::new(2);
pub(crate) const DEVTOOLS_ITEM_ID: CompositionItemId = CompositionItemId::new(3);

pub(crate) fn main_window_composition(
    toolbar_id: Option<BrowsingContextId>,
    page_id: Option<BrowsingContextId>,
    width: u32,
    height: u32,
) -> WindowCompositionSpec {
    let mut items = Vec::new();

    if let Some(toolbar_id) = toolbar_id {
        items.push(CompositionItemSpec {
            item_id: MAIN_TOOLBAR_ITEM_ID,
            target: SurfaceTarget::BrowsingContext(toolbar_id),
            bounds: main_toolbar_rect(width, height),
            visible: true,
            interactive: true,
            background: BackgroundPolicy::Opaque,
        });
    }

    if let Some(page_id) = page_id {
        items.push(CompositionItemSpec {
            item_id: MAIN_PAGE_ITEM_ID,
            target: SurfaceTarget::BrowsingContext(page_id),
            bounds: main_page_rect(width, height),
            visible: true,
            interactive: true,
            background: BackgroundPolicy::Opaque,
        });
    }

    WindowCompositionSpec { items }
}

pub(crate) fn devtools_window_composition(
    browsing_context_id: BrowsingContextId,
    width: u32,
    height: u32,
) -> WindowCompositionSpec {
    WindowCompositionSpec {
        items: vec![CompositionItemSpec {
            item_id: DEVTOOLS_ITEM_ID,
            target: SurfaceTarget::BrowsingContext(browsing_context_id),
            bounds: full_window_rect(width, height),
            visible: true,
            interactive: true,
            background: BackgroundPolicy::Opaque,
        }],
    }
}

pub(crate) fn host_window_composition(
    browsing_context_id: BrowsingContextId,
    width: u32,
    height: u32,
) -> WindowCompositionSpec {
    let item_id = CompositionItemId::new(1_000_000_000 + browsing_context_id.get());
    WindowCompositionSpec {
        items: vec![CompositionItemSpec {
            item_id,
            target: SurfaceTarget::BrowsingContext(browsing_context_id),
            bounds: full_window_rect(width, height),
            visible: true,
            interactive: true,
            background: BackgroundPolicy::Opaque,
        }],
    }
}

pub(crate) fn transient_window_composition(
    transient_browsing_context_id: TransientBrowsingContextId,
    width: u32,
    height: u32,
) -> WindowCompositionSpec {
    let item_id = CompositionItemId::new(2_000_000_000 + transient_browsing_context_id.get());
    WindowCompositionSpec {
        items: vec![CompositionItemSpec {
            item_id,
            target: SurfaceTarget::TransientBrowsingContext(transient_browsing_context_id),
            bounds: full_window_rect(width, height),
            visible: true,
            interactive: true,
            background: BackgroundPolicy::Opaque,
        }],
    }
}
