use cbf::data::ipc::IpcPayload;
use cbf_compositor::model::{HitTestCoordinateSpace, HitTestRegion};
use serde::Deserialize;

pub(crate) const CHANNEL_HIT_TEST_UPDATE: &str = "simpleapp.overlay.hit_test.update";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OverlayHitTestSnapshot {
    pub(crate) snapshot_id: u64,
    pub(crate) coordinate_space: HitTestCoordinateSpace,
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
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotRequest {
    snapshot_id: u64,
    coordinate_space: String,
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
            Ok(OverlayRequest::UpdateHitTest {
                snapshot: OverlayHitTestSnapshot {
                    snapshot_id: snapshot.snapshot_id,
                    coordinate_space,
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
                "{\"snapshotId\":2,\"coordinateSpace\":\"item-local-css-px\",\"regions\":[{\"x\":1,\"y\":2,\"width\":3,\"height\":4}]}"
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
                    regions: vec![HitTestRegion::new(1.0, 2.0, 3.0, 4.0)],
                }
            }
        );
    }
}
