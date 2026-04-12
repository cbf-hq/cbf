use cbf_compositor::model::Rect;

const TOOLBAR_HEIGHT: f64 = 56.0;
const TEST_POPUP_WIDTH: f64 = 360.0;
const TEST_POPUP_HEIGHT: f64 = 240.0;

pub(crate) fn main_toolbar_rect(width: u32, height: u32) -> Rect {
    let width = f64::from(width);
    let height = f64::from(height);

    Rect::new(
        0.0,
        (height - TOOLBAR_HEIGHT).max(0.0),
        width,
        TOOLBAR_HEIGHT,
    )
}

pub(crate) fn main_page_rect(width: u32, height: u32) -> Rect {
    let width = f64::from(width);
    let height = f64::from(height);

    Rect::new(0.0, 0.0, width, (height - TOOLBAR_HEIGHT).max(0.0))
}

pub(crate) fn full_window_rect(width: u32, height: u32) -> Rect {
    Rect::new(0.0, 0.0, f64::from(width), f64::from(height))
}

pub(crate) fn test_popup_rect(width: u32, height: u32) -> Rect {
    let width = f64::from(width);
    let height = f64::from(height);
    let popup_width = TEST_POPUP_WIDTH.min(width.max(1.0));
    let popup_height = TEST_POPUP_HEIGHT.min(height.max(1.0));

    Rect::new(
        ((width - popup_width) / 2.0).max(0.0),
        ((height - popup_height) / 2.0).max(0.0),
        popup_width,
        popup_height,
    )
}
