use cbf_compositor::model::Rect;

const TOOLBAR_HEIGHT: f64 = 56.0;

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
