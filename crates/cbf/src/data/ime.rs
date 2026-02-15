use super::ids::WebPageId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Classification of IME text spans.
pub enum ImeTextSpanType {
    Composition,
    Suggestion,
    MisspellingSuggestion,
    Autocorrect,
    GrammarSuggestion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Thickness of IME underline decorations.
pub enum ImeTextSpanThickness {
    None,
    Thin,
    Thick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Style of IME underline decorations.
pub enum ImeTextSpanUnderlineStyle {
    None,
    Solid,
    Dot,
    Dash,
    Squiggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Range in IME text, using character indices.
pub struct ImeTextRange {
    pub start: i32,
    pub end: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Metadata for a single IME text span.
pub struct ImeTextSpan {
    pub type_: ImeTextSpanType,
    pub start_offset: u32,
    pub end_offset: u32,
    pub underline_color: u32,
    pub thickness: ImeTextSpanThickness,
    pub underline_style: ImeTextSpanUnderlineStyle,
    pub text_color: u32,
    pub background_color: u32,
    pub suggestion_highlight_color: u32,
    pub remove_on_finish_composing: bool,
    pub interim_char_selection: bool,
    pub should_hide_suggestion_menu: bool,
}

impl ImeTextSpan {
    /// Create a span with no visual decorations to avoid default IME highlights.
    pub fn no_decoration(type_: ImeTextSpanType, start_offset: u32, end_offset: u32) -> Self {
        Self {
            type_,
            start_offset,
            end_offset,
            underline_color: 0,
            thickness: ImeTextSpanThickness::None,
            underline_style: ImeTextSpanUnderlineStyle::None,
            text_color: 0,
            background_color: 0,
            suggestion_highlight_color: 0,
            remove_on_finish_composing: false,
            interim_char_selection: false,
            should_hide_suggestion_menu: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Current IME composition state for a web page.
pub struct ImeComposition {
    pub web_page_id: WebPageId,
    pub text: String,
    pub selection_start: i32,
    pub selection_end: i32,
    pub replacement_range: Option<ImeTextRange>,
    pub spans: Vec<ImeTextSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// IME commit payload to finalize composed text.
pub struct ImeCommitText {
    pub web_page_id: WebPageId,
    pub text: String,
    pub relative_caret_position: i32,
    pub replacement_range: Option<ImeTextRange>,
    pub spans: Vec<ImeTextSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Behavior used when finishing IME composition.
pub enum ConfirmCompositionBehavior {
    DoNotKeepSelection,
    KeepSelection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Rectangle used by IME bounds and selection information.
pub struct ImeRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Bounds of the current IME composition range.
pub struct ImeCompositionBounds {
    pub range_start: i32,
    pub range_end: i32,
    pub character_bounds: Vec<ImeRect>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Bounds information for the current text selection.
pub struct TextSelectionBounds {
    pub range_start: i32,
    pub range_end: i32,
    pub caret_rect: ImeRect,
    pub first_selection_rect: ImeRect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// IME bounds updates for composition and selection.
pub struct ImeBoundsUpdate {
    pub composition: Option<ImeCompositionBounds>,
    pub selection: Option<TextSelectionBounds>,
}
