use crate::data::ids::BrowsingContextId;
use std::collections::BTreeMap;

/// Browser-generic drag operation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DragOperation {
    None,
    Copy,
    Link,
    Move,
}

/// Bitmask of browser-generic drag operations.
///
/// Backends may map this to native/raw protocol flags internally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DragOperations(u32);

impl DragOperations {
    pub const NONE: Self = Self(0);
    pub const COPY: Self = Self(1 << 0);
    pub const LINK: Self = Self(1 << 1);
    pub const MOVE: Self = Self(1 << 2);

    pub const fn empty() -> Self {
        Self::NONE
    }

    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn contains(self, operation: DragOperation) -> bool {
        let mask = match operation {
            DragOperation::None => 0,
            DragOperation::Copy => Self::COPY.0,
            DragOperation::Link => Self::LINK.0,
            DragOperation::Move => Self::MOVE.0,
        };
        (self.0 & mask) == mask
    }
}

impl Default for DragOperations {
    fn default() -> Self {
        Self::NONE
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DragUrlInfo {
    pub url: String,
    pub title: String,
}

/// Browser-generic drag payload.
///
/// Fields with Chromium-internal semantics (filesystem routing, privilege
/// markers, renderer-origin flags, etc.) are intentionally excluded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DragData {
    pub text: String,
    pub html: String,
    pub html_base_url: String,
    pub url_infos: Vec<DragUrlInfo>,
    pub filenames: Vec<String>,
    pub file_mime_types: Vec<String>,
    pub custom_data: BTreeMap<String, String>,
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
    pub browsing_context_id: BrowsingContextId,
    pub allowed_operations: DragOperations,
    pub source_origin: String,
    pub data: DragData,
    pub image: Option<DragImage>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DragUpdate {
    pub session_id: u64,
    pub browsing_context_id: BrowsingContextId,
    pub allowed_operations: DragOperations,
    pub modifiers: u32,
    pub position_in_widget_x: f32,
    pub position_in_widget_y: f32,
    pub position_in_screen_x: f32,
    pub position_in_screen_y: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DragDrop {
    pub session_id: u64,
    pub browsing_context_id: BrowsingContextId,
    pub modifiers: u32,
    pub position_in_widget_x: f32,
    pub position_in_widget_y: f32,
    pub position_in_screen_x: f32,
    pub position_in_screen_y: f32,
}
