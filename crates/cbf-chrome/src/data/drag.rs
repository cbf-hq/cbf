//! Chrome-specific drag-and-drop operation flags and data, with conversions to/from `cbf` equivalents.

use std::collections::BTreeMap;

use super::ids::TabId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChromeDragOperations(u32);

impl ChromeDragOperations {
    pub const NONE: Self = Self(0);
    pub const COPY: Self = Self(1 << 0);
    pub const LINK: Self = Self(1 << 1);
    pub const MOVE: Self = Self(16);

    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn contains(self, operation: ChromeDragOperation) -> bool {
        let mask = match operation {
            ChromeDragOperation::None => 0,
            ChromeDragOperation::Copy => Self::COPY.0,
            ChromeDragOperation::Link => Self::LINK.0,
            ChromeDragOperation::Move => Self::MOVE.0,
        };
        (self.0 & mask) == mask
    }
}

impl From<ChromeDragOperations> for cbf::data::drag::DragOperations {
    fn from(value: ChromeDragOperations) -> Self {
        let mut bits = Self::NONE.bits();
        if value.contains(ChromeDragOperation::Copy) {
            bits |= Self::COPY.bits();
        }
        if value.contains(ChromeDragOperation::Link) {
            bits |= Self::LINK.bits();
        }
        if value.contains(ChromeDragOperation::Move) {
            bits |= Self::MOVE.bits();
        }
        Self::from_bits(bits)
    }
}

impl From<cbf::data::drag::DragOperations> for ChromeDragOperations {
    fn from(value: cbf::data::drag::DragOperations) -> Self {
        let mut bits = Self::NONE.bits();
        if value.contains(cbf::data::drag::DragOperation::Copy) {
            bits |= Self::COPY.bits();
        }
        if value.contains(cbf::data::drag::DragOperation::Link) {
            bits |= Self::LINK.bits();
        }
        if value.contains(cbf::data::drag::DragOperation::Move) {
            bits |= Self::MOVE.bits();
        }
        Self::from_bits(bits)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChromeDragOperation {
    None,
    Copy,
    Link,
    Move,
}

impl From<ChromeDragOperation> for cbf::data::drag::DragOperation {
    fn from(value: ChromeDragOperation) -> Self {
        match value {
            ChromeDragOperation::None => Self::None,
            ChromeDragOperation::Copy => Self::Copy,
            ChromeDragOperation::Link => Self::Link,
            ChromeDragOperation::Move => Self::Move,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeDragUrlInfo {
    pub url: String,
    pub title: String,
}

impl From<ChromeDragUrlInfo> for cbf::data::drag::DragUrlInfo {
    fn from(value: ChromeDragUrlInfo) -> Self {
        Self {
            url: value.url,
            title: value.title,
        }
    }
}

impl From<cbf::data::drag::DragUrlInfo> for ChromeDragUrlInfo {
    fn from(value: cbf::data::drag::DragUrlInfo) -> Self {
        Self {
            url: value.url,
            title: value.title,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeDragData {
    pub text: String,
    pub html: String,
    pub html_base_url: String,
    pub url_infos: Vec<ChromeDragUrlInfo>,
    pub filenames: Vec<String>,
    pub file_mime_types: Vec<String>,
    pub custom_data: BTreeMap<String, String>,
}

impl From<ChromeDragData> for cbf::data::drag::DragData {
    fn from(value: ChromeDragData) -> Self {
        Self {
            text: value.text,
            html: value.html,
            html_base_url: value.html_base_url,
            url_infos: value.url_infos.into_iter().map(Into::into).collect(),
            filenames: value.filenames,
            file_mime_types: value.file_mime_types,
            custom_data: value.custom_data,
        }
    }
}

impl From<cbf::data::drag::DragData> for ChromeDragData {
    fn from(value: cbf::data::drag::DragData) -> Self {
        Self {
            text: value.text,
            html: value.html,
            html_base_url: value.html_base_url,
            url_infos: value.url_infos.into_iter().map(Into::into).collect(),
            filenames: value.filenames,
            file_mime_types: value.file_mime_types,
            custom_data: value.custom_data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ChromeDragOperation, ChromeDragOperations};

    #[test]
    fn chrome_drag_operations_map_move_to_chromium_bits() {
        let generic = cbf::data::drag::DragOperations::from_bits(
            cbf::data::drag::DragOperations::COPY.bits()
                | cbf::data::drag::DragOperations::MOVE.bits(),
        );

        let chrome = ChromeDragOperations::from(generic);

        assert!(chrome.contains(ChromeDragOperation::Copy));
        assert!(chrome.contains(ChromeDragOperation::Move));
        assert!(!chrome.contains(ChromeDragOperation::Link));
        assert_eq!(
            chrome.bits(),
            ChromeDragOperations::COPY.bits() | ChromeDragOperations::MOVE.bits()
        );
    }

    #[test]
    fn chrome_drag_operations_map_move_back_to_generic_bits() {
        let chrome = ChromeDragOperations::from_bits(
            ChromeDragOperations::LINK.bits() | ChromeDragOperations::MOVE.bits(),
        );

        let generic = cbf::data::drag::DragOperations::from(chrome);

        assert!(generic.contains(cbf::data::drag::DragOperation::Link));
        assert!(generic.contains(cbf::data::drag::DragOperation::Move));
        assert!(!generic.contains(cbf::data::drag::DragOperation::Copy));
        assert_eq!(
            generic.bits(),
            cbf::data::drag::DragOperations::LINK.bits()
                | cbf::data::drag::DragOperations::MOVE.bits()
        );
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChromeDragImage {
    pub png_bytes: Vec<u8>,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub scale: f32,
    pub cursor_offset_x: i32,
    pub cursor_offset_y: i32,
}

impl From<ChromeDragImage> for cbf::data::drag::DragImage {
    fn from(value: ChromeDragImage) -> Self {
        Self {
            png_bytes: value.png_bytes,
            pixel_width: value.pixel_width,
            pixel_height: value.pixel_height,
            scale: value.scale,
            cursor_offset_x: value.cursor_offset_x,
            cursor_offset_y: value.cursor_offset_y,
        }
    }
}

impl From<cbf::data::drag::DragImage> for ChromeDragImage {
    fn from(value: cbf::data::drag::DragImage) -> Self {
        Self {
            png_bytes: value.png_bytes,
            pixel_width: value.pixel_width,
            pixel_height: value.pixel_height,
            scale: value.scale,
            cursor_offset_x: value.cursor_offset_x,
            cursor_offset_y: value.cursor_offset_y,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChromeDragStartRequest {
    pub session_id: u64,
    pub browsing_context_id: TabId,
    pub allowed_operations: ChromeDragOperations,
    pub source_origin: String,
    pub data: ChromeDragData,
    pub image: Option<ChromeDragImage>,
}

impl From<ChromeDragStartRequest> for cbf::data::drag::DragStartRequest {
    fn from(value: ChromeDragStartRequest) -> Self {
        Self {
            session_id: value.session_id,
            browsing_context_id: value.browsing_context_id.into(),
            allowed_operations: value.allowed_operations.into(),
            source_origin: value.source_origin,
            data: value.data.into(),
            image: value.image.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChromeDragUpdate {
    pub session_id: u64,
    pub browsing_context_id: TabId,
    pub allowed_operations: ChromeDragOperations,
    pub modifiers: u32,
    pub position_in_widget_x: f32,
    pub position_in_widget_y: f32,
    pub position_in_screen_x: f32,
    pub position_in_screen_y: f32,
}

impl From<ChromeDragUpdate> for cbf::data::drag::DragUpdate {
    fn from(value: ChromeDragUpdate) -> Self {
        Self {
            session_id: value.session_id,
            browsing_context_id: value.browsing_context_id.into(),
            allowed_operations: value.allowed_operations.into(),
            modifiers: value.modifiers,
            position_in_widget_x: value.position_in_widget_x,
            position_in_widget_y: value.position_in_widget_y,
            position_in_screen_x: value.position_in_screen_x,
            position_in_screen_y: value.position_in_screen_y,
        }
    }
}

impl From<cbf::data::drag::DragUpdate> for ChromeDragUpdate {
    fn from(value: cbf::data::drag::DragUpdate) -> Self {
        Self {
            session_id: value.session_id,
            browsing_context_id: value.browsing_context_id.into(),
            allowed_operations: value.allowed_operations.into(),
            modifiers: value.modifiers,
            position_in_widget_x: value.position_in_widget_x,
            position_in_widget_y: value.position_in_widget_y,
            position_in_screen_x: value.position_in_screen_x,
            position_in_screen_y: value.position_in_screen_y,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChromeExternalDragEnter {
    pub browsing_context_id: TabId,
    pub data: ChromeDragData,
    pub allowed_operations: ChromeDragOperations,
    pub modifiers: u32,
    pub position_in_widget_x: f32,
    pub position_in_widget_y: f32,
    pub position_in_screen_x: f32,
    pub position_in_screen_y: f32,
}

impl From<ChromeExternalDragEnter> for cbf::data::drag::ExternalDragEnter {
    fn from(value: ChromeExternalDragEnter) -> Self {
        Self {
            browsing_context_id: value.browsing_context_id.into(),
            data: value.data.into(),
            allowed_operations: value.allowed_operations.into(),
            modifiers: value.modifiers,
            position_in_widget_x: value.position_in_widget_x,
            position_in_widget_y: value.position_in_widget_y,
            position_in_screen_x: value.position_in_screen_x,
            position_in_screen_y: value.position_in_screen_y,
        }
    }
}

impl From<cbf::data::drag::ExternalDragEnter> for ChromeExternalDragEnter {
    fn from(value: cbf::data::drag::ExternalDragEnter) -> Self {
        Self {
            browsing_context_id: value.browsing_context_id.into(),
            data: value.data.into(),
            allowed_operations: value.allowed_operations.into(),
            modifiers: value.modifiers,
            position_in_widget_x: value.position_in_widget_x,
            position_in_widget_y: value.position_in_widget_y,
            position_in_screen_x: value.position_in_screen_x,
            position_in_screen_y: value.position_in_screen_y,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChromeExternalDragUpdate {
    pub browsing_context_id: TabId,
    pub allowed_operations: ChromeDragOperations,
    pub modifiers: u32,
    pub position_in_widget_x: f32,
    pub position_in_widget_y: f32,
    pub position_in_screen_x: f32,
    pub position_in_screen_y: f32,
}

impl From<ChromeExternalDragUpdate> for cbf::data::drag::ExternalDragUpdate {
    fn from(value: ChromeExternalDragUpdate) -> Self {
        Self {
            browsing_context_id: value.browsing_context_id.into(),
            allowed_operations: value.allowed_operations.into(),
            modifiers: value.modifiers,
            position_in_widget_x: value.position_in_widget_x,
            position_in_widget_y: value.position_in_widget_y,
            position_in_screen_x: value.position_in_screen_x,
            position_in_screen_y: value.position_in_screen_y,
        }
    }
}

impl From<cbf::data::drag::ExternalDragUpdate> for ChromeExternalDragUpdate {
    fn from(value: cbf::data::drag::ExternalDragUpdate) -> Self {
        Self {
            browsing_context_id: value.browsing_context_id.into(),
            allowed_operations: value.allowed_operations.into(),
            modifiers: value.modifiers,
            position_in_widget_x: value.position_in_widget_x,
            position_in_widget_y: value.position_in_widget_y,
            position_in_screen_x: value.position_in_screen_x,
            position_in_screen_y: value.position_in_screen_y,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChromeExternalDragDrop {
    pub browsing_context_id: TabId,
    pub modifiers: u32,
    pub position_in_widget_x: f32,
    pub position_in_widget_y: f32,
    pub position_in_screen_x: f32,
    pub position_in_screen_y: f32,
}

impl From<ChromeExternalDragDrop> for cbf::data::drag::ExternalDragDrop {
    fn from(value: ChromeExternalDragDrop) -> Self {
        Self {
            browsing_context_id: value.browsing_context_id.into(),
            modifiers: value.modifiers,
            position_in_widget_x: value.position_in_widget_x,
            position_in_widget_y: value.position_in_widget_y,
            position_in_screen_x: value.position_in_screen_x,
            position_in_screen_y: value.position_in_screen_y,
        }
    }
}

impl From<cbf::data::drag::ExternalDragDrop> for ChromeExternalDragDrop {
    fn from(value: cbf::data::drag::ExternalDragDrop) -> Self {
        Self {
            browsing_context_id: value.browsing_context_id.into(),
            modifiers: value.modifiers,
            position_in_widget_x: value.position_in_widget_x,
            position_in_widget_y: value.position_in_widget_y,
            position_in_screen_x: value.position_in_screen_x,
            position_in_screen_y: value.position_in_screen_y,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChromeDragDrop {
    pub session_id: u64,
    pub browsing_context_id: TabId,
    pub modifiers: u32,
    pub position_in_widget_x: f32,
    pub position_in_widget_y: f32,
    pub position_in_screen_x: f32,
    pub position_in_screen_y: f32,
}

impl From<ChromeDragDrop> for cbf::data::drag::DragDrop {
    fn from(value: ChromeDragDrop) -> Self {
        Self {
            session_id: value.session_id,
            browsing_context_id: value.browsing_context_id.into(),
            modifiers: value.modifiers,
            position_in_widget_x: value.position_in_widget_x,
            position_in_widget_y: value.position_in_widget_y,
            position_in_screen_x: value.position_in_screen_x,
            position_in_screen_y: value.position_in_screen_y,
        }
    }
}

impl From<cbf::data::drag::DragDrop> for ChromeDragDrop {
    fn from(value: cbf::data::drag::DragDrop) -> Self {
        Self {
            session_id: value.session_id,
            browsing_context_id: value.browsing_context_id.into(),
            modifiers: value.modifiers,
            position_in_widget_x: value.position_in_widget_x,
            position_in_widget_y: value.position_in_widget_y,
            position_in_screen_x: value.position_in_screen_x,
            position_in_screen_y: value.position_in_screen_y,
        }
    }
}
