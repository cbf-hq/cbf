use cbf::data::ime::{ImeBoundsUpdate, ImeCompositionBounds, ImeRect, TextSelectionBounds};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::NSRange;

pub(crate) fn candidate_rect_for_slot(
    range: NSRange,
    slot_bounds: CGRect,
    ime_bounds: Option<&ImeBoundsUpdate>,
) -> Option<CGRect> {
    let bounds = ime_bounds?;

    if let Some(composition) = bounds.composition.as_ref()
        && let Some(rect) = rect_for_composition_range(range, composition)
    {
        return Some(offset_rect(
            flip_rect_in_layer(rect, slot_bounds.size.height),
            slot_bounds.origin.x,
            slot_bounds.origin.y,
        ));
    }

    bounds.selection.as_ref().map(|selection| {
        offset_rect(
            flip_rect_in_layer(rect_from_selection(selection), slot_bounds.size.height),
            slot_bounds.origin.x,
            slot_bounds.origin.y,
        )
    })
}

fn rect_for_composition_range(
    range: NSRange,
    composition: &ImeCompositionBounds,
) -> Option<CGRect> {
    if composition.range_start < 0 || composition.range_end < composition.range_start {
        return None;
    }
    if composition.character_bounds.is_empty() {
        return None;
    }
    if range.location == usize::MAX {
        return None;
    }

    let start = range.location.min(i32::MAX as usize) as i32;
    let end = range.end().min(i32::MAX as usize).max(range.location) as i32;

    if start < composition.range_start || end > composition.range_end {
        return None;
    }

    let local_start = (start - composition.range_start) as usize;
    if local_start >= composition.character_bounds.len() {
        return None;
    }

    if range.length == 0 {
        return Some(rect_from_ime(&composition.character_bounds[local_start]));
    }

    let local_end = (end - composition.range_start) as usize;
    let clamped_end = local_end.min(composition.character_bounds.len());
    if clamped_end <= local_start {
        return Some(rect_from_ime(&composition.character_bounds[local_start]));
    }

    let mut rect = rect_from_ime(&composition.character_bounds[local_start]);
    for bounds in &composition.character_bounds[local_start + 1..clamped_end] {
        rect = union_rect(rect, rect_from_ime(bounds));
    }

    Some(rect)
}

fn rect_from_selection(selection: &TextSelectionBounds) -> CGRect {
    rect_from_ime(&selection.caret_rect)
}

fn rect_from_ime(rect: &ImeRect) -> CGRect {
    CGRect::new(
        CGPoint::new(rect.x as f64, rect.y as f64),
        CGSize::new(rect.width as f64, rect.height as f64),
    )
}

fn union_rect(a: CGRect, b: CGRect) -> CGRect {
    let min_x = a.origin.x.min(b.origin.x);
    let min_y = a.origin.y.min(b.origin.y);
    let max_x = (a.origin.x + a.size.width).max(b.origin.x + b.size.width);
    let max_y = (a.origin.y + a.size.height).max(b.origin.y + b.size.height);

    CGRect::new(
        CGPoint::new(min_x, min_y),
        CGSize::new((max_x - min_x).max(0.0), (max_y - min_y).max(0.0)),
    )
}

fn offset_rect(rect: CGRect, offset_x: f64, offset_y: f64) -> CGRect {
    CGRect::new(
        CGPoint::new(rect.origin.x + offset_x, rect.origin.y + offset_y),
        rect.size,
    )
}

fn flip_rect_in_layer(rect: CGRect, layer_height: f64) -> CGRect {
    let flipped_y = (layer_height - (rect.origin.y + rect.size.height)).max(0.0);
    CGRect::new(CGPoint::new(rect.origin.x, flipped_y), rect.size)
}

#[cfg(test)]
mod tests {
    use cbf::data::ime::{ImeBoundsUpdate, ImeCompositionBounds, ImeRect};
    use objc2_core_foundation::{CGPoint, CGRect, CGSize};
    use objc2_foundation::NSRange;

    use super::candidate_rect_for_slot;

    #[test]
    fn candidate_rect_offsets_into_slot_space() {
        let rect = candidate_rect_for_slot(
            NSRange::new(0, 1),
            CGRect::new(CGPoint::new(20.0, 30.0), CGSize::new(100.0, 50.0)),
            Some(&ImeBoundsUpdate {
                composition: Some(ImeCompositionBounds {
                    range_start: 0,
                    range_end: 1,
                    character_bounds: vec![ImeRect {
                        x: 1,
                        y: 2,
                        width: 3,
                        height: 4,
                    }],
                }),
                selection: None,
            }),
        )
        .unwrap();

        assert_eq!(rect.origin.x, 21.0);
        assert_eq!(rect.origin.y, 74.0);
        assert_eq!(rect.size.width, 3.0);
        assert_eq!(rect.size.height, 4.0);
    }
}
