use cbf::data::ipc::IpcPayload;
use cbf_compositor::model::{HitTestCoordinateSpace, HitTestRegion, HitTestRegionMode};
use serde::Deserialize;

pub(crate) const CHANNEL_HIT_TEST_UPDATE: &str = "simpleapp.overlay.hit_test.update";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OverlayHitTestSnapshot {
    pub(crate) snapshot_id: u64,
    pub(crate) coordinate_space: HitTestCoordinateSpace,
    pub(crate) mode: HitTestRegionMode,
    pub(crate) regions: Vec<HitTestRegion>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum OverlayRequest {
    UpdateHitTest { snapshot: OverlayHitTestSnapshot },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParseError {
    UnsupportedPayload,
    UnknownChannel,
    InvalidJson,
    UnsupportedCoordinateSpace,
    UnsupportedMode,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotRequest {
    snapshot_id: u64,
    coordinate_space: String,
    mode: String,
    #[serde(default)]
    regions: Vec<RegionPayload>,
}

#[derive(Debug, Clone, Deserialize)]
struct RegionPayload {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

pub(crate) fn parse_request(
    channel: &str,
    payload: &IpcPayload,
) -> Result<OverlayRequest, ParseError> {
    let payload_text = match payload {
        IpcPayload::Text(text) => text,
        IpcPayload::Binary(_) => return Err(ParseError::UnsupportedPayload),
    };

    match channel {
        CHANNEL_HIT_TEST_UPDATE => {
            let snapshot: SnapshotRequest =
                serde_json::from_str(payload_text).map_err(|_| ParseError::InvalidJson)?;
            let coordinate_space = match snapshot.coordinate_space.as_str() {
                "item-local-css-px" => HitTestCoordinateSpace::ItemLocalCssPx,
                _ => return Err(ParseError::UnsupportedCoordinateSpace),
            };
            let mode = match snapshot.mode.as_str() {
                "consume-listed-regions" => HitTestRegionMode::ConsumeListedRegions,
                "passthrough-listed-regions" => HitTestRegionMode::PassthroughListedRegions,
                _ => return Err(ParseError::UnsupportedMode),
            };
            Ok(OverlayRequest::UpdateHitTest {
                snapshot: OverlayHitTestSnapshot {
                    snapshot_id: snapshot.snapshot_id,
                    coordinate_space,
                    mode,
                    regions: snapshot
                        .regions
                        .into_iter()
                        .map(|region| {
                            HitTestRegion::new(region.x, region.y, region.width, region.height)
                        })
                        .collect(),
                },
            })
        }
        _ => Err(ParseError::UnknownChannel),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_update_hit_test_request() {
        let request = parse_request(
            CHANNEL_HIT_TEST_UPDATE,
            &IpcPayload::Text(
                "{\"snapshotId\":2,\"coordinateSpace\":\"item-local-css-px\",\"mode\":\"consume-listed-regions\",\"regions\":[{\"x\":1,\"y\":2,\"width\":3,\"height\":4}]}"
                    .to_string(),
            ),
        )
        .expect("request should parse");

        assert_eq!(
            request,
            OverlayRequest::UpdateHitTest {
                snapshot: OverlayHitTestSnapshot {
                    snapshot_id: 2,
                    coordinate_space: HitTestCoordinateSpace::ItemLocalCssPx,
                    mode: HitTestRegionMode::ConsumeListedRegions,
                    regions: vec![HitTestRegion::new(1.0, 2.0, 3.0, 4.0)],
                }
            }
        );
    }
}
