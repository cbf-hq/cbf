#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DragUrlInfo {
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DragData {
    pub text: String,
    pub html: String,
    pub html_base_url: String,
    pub url_infos: Vec<DragUrlInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DragImage {
    pub png_bytes: Vec<u8>,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub scale: f32,
    pub cursor_offset_x: i32,
    pub cursor_offset_y: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DragStartRequest {
    pub session_id: u64,
    pub web_page_id: crate::data::ids::WebPageId,
    pub allowed_operations: u32,
    pub source_origin: String,
    pub data: DragData,
    pub image: Option<DragImage>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DragUpdate {
    pub session_id: u64,
    pub web_page_id: crate::data::ids::WebPageId,
    pub allowed_operations: u32,
    pub modifiers: u32,
    pub position_in_widget_x: f32,
    pub position_in_widget_y: f32,
    pub position_in_screen_x: f32,
    pub position_in_screen_y: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DragDrop {
    pub session_id: u64,
    pub web_page_id: crate::data::ids::WebPageId,
    pub modifiers: u32,
    pub position_in_widget_x: f32,
    pub position_in_widget_y: f32,
    pub position_in_screen_x: f32,
    pub position_in_screen_y: f32,
}
