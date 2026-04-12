use cbf::data::ids::{BrowsingContextId, TransientBrowsingContextId};
use cbf_compositor::model::{
    BackgroundPolicy, CompositionItemId, CompositionItemSpec, HitTestPolicy, SurfaceTarget,
    WindowCompositionSpec,
};

use crate::scene::layout::{full_window_rect, main_page_rect, main_toolbar_rect, test_popup_rect};

const PAGE_ITEM_NAMESPACE: u64 = 1_000_000_000;
const TRANSIENT_ITEM_NAMESPACE: u64 = 2_000_000_000;
const TOOLBAR_ITEM_NAMESPACE: u64 = 3_000_000_000;
const OVERLAY_ITEM_NAMESPACE: u64 = 4_000_000_000;
const DEVTOOLS_ITEM_NAMESPACE: u64 = 5_000_000_000;
const TEST_POPUP_ITEM_NAMESPACE: u64 = 6_000_000_000;

pub(crate) fn main_window_composition(
    overlay_id: Option<BrowsingContextId>,
    toolbar_id: Option<BrowsingContextId>,
    page_id: Option<BrowsingContextId>,
    test_popup_id: Option<BrowsingContextId>,
    width: u32,
    height: u32,
) -> WindowCompositionSpec {
    let mut items = Vec::new();

    if let Some(test_popup_id) = test_popup_id {
        items.push(CompositionItemSpec {
            item_id: test_popup_item_id(test_popup_id),
            target: SurfaceTarget::BrowsingContext(test_popup_id),
            bounds: test_popup_rect(width, height),
            visible: true,
            hit_test: HitTestPolicy::Bounds,
            background: BackgroundPolicy::Transparent,
        });
    }

    if let Some(overlay_id) = overlay_id {
        items.push(CompositionItemSpec {
            item_id: overlay_item_id(overlay_id),
            target: SurfaceTarget::BrowsingContext(overlay_id),
            bounds: full_window_rect(width, height),
            visible: true,
            hit_test: HitTestPolicy::RegionSnapshot,
            background: BackgroundPolicy::Transparent,
        });
    }

    if let Some(toolbar_id) = toolbar_id {
        items.push(CompositionItemSpec {
            item_id: toolbar_item_id(toolbar_id),
            target: SurfaceTarget::BrowsingContext(toolbar_id),
            bounds: main_toolbar_rect(width, height),
            visible: true,
            hit_test: HitTestPolicy::Bounds,
            background: BackgroundPolicy::Opaque,
        });
    }

    if let Some(page_id) = page_id {
        items.push(CompositionItemSpec {
            item_id: page_item_id(page_id),
            target: SurfaceTarget::BrowsingContext(page_id),
            bounds: main_page_rect(width, height),
            visible: true,
            hit_test: HitTestPolicy::Bounds,
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
            item_id: devtools_item_id(browsing_context_id),
            target: SurfaceTarget::BrowsingContext(browsing_context_id),
            bounds: full_window_rect(width, height),
            visible: true,
            hit_test: HitTestPolicy::Bounds,
            background: BackgroundPolicy::Opaque,
        }],
    }
}

pub(crate) fn host_window_composition(
    browsing_context_id: BrowsingContextId,
    width: u32,
    height: u32,
) -> WindowCompositionSpec {
    WindowCompositionSpec {
        items: vec![CompositionItemSpec {
            item_id: page_item_id(browsing_context_id),
            target: SurfaceTarget::BrowsingContext(browsing_context_id),
            bounds: full_window_rect(width, height),
            visible: true,
            hit_test: HitTestPolicy::Bounds,
            background: BackgroundPolicy::Opaque,
        }],
    }
}

pub(crate) fn transient_window_composition(
    transient_browsing_context_id: TransientBrowsingContextId,
    width: u32,
    height: u32,
) -> WindowCompositionSpec {
    WindowCompositionSpec {
        items: vec![CompositionItemSpec {
            item_id: transient_item_id(transient_browsing_context_id),
            target: SurfaceTarget::TransientBrowsingContext(transient_browsing_context_id),
            bounds: full_window_rect(width, height),
            visible: true,
            hit_test: HitTestPolicy::Bounds,
            background: BackgroundPolicy::Opaque,
        }],
    }
}

pub(crate) const fn page_item_id(browsing_context_id: BrowsingContextId) -> CompositionItemId {
    CompositionItemId::new(PAGE_ITEM_NAMESPACE + browsing_context_id.get())
}

const fn transient_item_id(
    transient_browsing_context_id: TransientBrowsingContextId,
) -> CompositionItemId {
    CompositionItemId::new(TRANSIENT_ITEM_NAMESPACE + transient_browsing_context_id.get())
}

pub(crate) const fn toolbar_item_id(browsing_context_id: BrowsingContextId) -> CompositionItemId {
    CompositionItemId::new(TOOLBAR_ITEM_NAMESPACE + browsing_context_id.get())
}

pub(crate) const fn overlay_item_id(browsing_context_id: BrowsingContextId) -> CompositionItemId {
    CompositionItemId::new(OVERLAY_ITEM_NAMESPACE + browsing_context_id.get())
}

const fn devtools_item_id(browsing_context_id: BrowsingContextId) -> CompositionItemId {
    CompositionItemId::new(DEVTOOLS_ITEM_NAMESPACE + browsing_context_id.get())
}

pub(crate) const fn test_popup_item_id(
    browsing_context_id: BrowsingContextId,
) -> CompositionItemId {
    CompositionItemId::new(TEST_POPUP_ITEM_NAMESPACE + browsing_context_id.get())
}
