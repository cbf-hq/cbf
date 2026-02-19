use super::ids::BrowsingContextId;

/// Classification of IME text spans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImeTextSpanType {
    Composition,
    Suggestion,
    MisspellingSuggestion,
    Autocorrect,
    GrammarSuggestion,
}

/// Thickness of IME underline decorations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImeTextSpanThickness {
    None,
    Thin,
    Thick,
}

/// Style of IME underline decorations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImeTextSpanUnderlineStyle {
    None,
    Solid,
    Dot,
    Dash,
    Squiggle,
}

/// Range in IME text, using character indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImeTextRange {
    pub start: i32,
    pub end: i32,
}

/// Metadata for a single IME text span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImeTextSpan {
    pub r#type: ImeTextSpanType,
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
    pub fn no_decoration(r#type: ImeTextSpanType, start_offset: u32, end_offset: u32) -> Self {
        Self {
            r#type,
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

/// Current IME composition state for a web page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImeComposition {
    pub browsing_context_id: BrowsingContextId,
    pub text: String,
    pub selection_start: i32,
    pub selection_end: i32,
    pub replacement_range: Option<ImeTextRange>,
    pub spans: Vec<ImeTextSpan>,
}

/// IME commit payload to finalize composed text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImeCommitText {
    pub browsing_context_id: BrowsingContextId,
    pub text: String,
    pub relative_caret_position: i32,
    pub replacement_range: Option<ImeTextRange>,
    pub spans: Vec<ImeTextSpan>,
}

/// Behavior used when finishing IME composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmCompositionBehavior {
    DoNotKeepSelection,
    KeepSelection,
}

/// Rectangle used by IME bounds and selection information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImeRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Bounds of the current IME composition range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImeCompositionBounds {
    pub range_start: i32,
    pub range_end: i32,
    pub character_bounds: Vec<ImeRect>,
}

/// Bounds information for the current text selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSelectionBounds {
    pub range_start: i32,
    pub range_end: i32,
    pub caret_rect: ImeRect,
    pub first_selection_rect: ImeRect,
}

/// IME bounds updates for composition and selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImeBoundsUpdate {
    pub composition: Option<ImeCompositionBounds>,
    pub selection: Option<TextSelectionBounds>,
}
