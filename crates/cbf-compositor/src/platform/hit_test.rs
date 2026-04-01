use crate::model::{HitTestCoordinateSpace, HitTestRegionMode, HitTestRegionSnapshot};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ItemLocalCssPoint {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

impl ItemLocalCssPoint {
    pub(crate) const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

pub(crate) fn snapshot_contains_point(
    snapshot: &HitTestRegionSnapshot,
    point: ItemLocalCssPoint,
) -> bool {
    let matched = match snapshot.coordinate_space {
        HitTestCoordinateSpace::ItemLocalCssPx => snapshot.regions.iter().any(|region| {
            point.x >= f64::from(region.x)
                && point.y >= f64::from(region.y)
                && point.x <= f64::from(region.x + region.width)
                && point.y <= f64::from(region.y + region.height)
        }),
    };

    match snapshot.mode {
        HitTestRegionMode::ConsumeListedRegions => matched,
        HitTestRegionMode::PassthroughListedRegions => !matched,
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{
        HitTestCoordinateSpace, HitTestRegion, HitTestRegionMode, HitTestRegionSnapshot,
    };

    use super::{ItemLocalCssPoint, snapshot_contains_point};

    fn snapshot(mode: HitTestRegionMode) -> HitTestRegionSnapshot {
        HitTestRegionSnapshot {
            snapshot_id: 1,
            coordinate_space: HitTestCoordinateSpace::ItemLocalCssPx,
            mode,
            regions: vec![HitTestRegion::new(10.0, 15.0, 20.0, 25.0)],
        }
    }

    #[test]
    fn snapshot_contains_point_uses_top_left_item_local_coordinates() {
        let snapshot = snapshot(HitTestRegionMode::ConsumeListedRegions);

        assert!(snapshot_contains_point(
            &snapshot,
            ItemLocalCssPoint::new(15.0, 20.0)
        ));
        assert!(!snapshot_contains_point(
            &snapshot,
            ItemLocalCssPoint::new(15.0, 90.0)
        ));
    }

    #[test]
    fn snapshot_contains_point_supports_passthrough_listed_regions() {
        let snapshot = snapshot(HitTestRegionMode::PassthroughListedRegions);

        assert!(!snapshot_contains_point(
            &snapshot,
            ItemLocalCssPoint::new(15.0, 20.0)
        ));
        assert!(snapshot_contains_point(
            &snapshot,
            ItemLocalCssPoint::new(15.0, 90.0)
        ));
    }
}
