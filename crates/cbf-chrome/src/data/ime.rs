//! Chrome-specific IME (Input Method Editor) text span types and composition state, with conversions to/from `cbf` equivalents.

use super::ids::TabId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeImeTextSpanType {
    Composition,
    Suggestion,
    MisspellingSuggestion,
    Autocorrect,
    GrammarSuggestion,
}

impl From<ChromeImeTextSpanType> for cbf::data::ime::ImeTextSpanType {
    fn from(value: ChromeImeTextSpanType) -> Self {
        match value {
            ChromeImeTextSpanType::Composition => Self::Composition,
            ChromeImeTextSpanType::Suggestion => Self::Suggestion,
            ChromeImeTextSpanType::MisspellingSuggestion => Self::MisspellingSuggestion,
            ChromeImeTextSpanType::Autocorrect => Self::Autocorrect,
            ChromeImeTextSpanType::GrammarSuggestion => Self::GrammarSuggestion,
        }
    }
}

impl From<cbf::data::ime::ImeTextSpanType> for ChromeImeTextSpanType {
    fn from(value: cbf::data::ime::ImeTextSpanType) -> Self {
        match value {
            cbf::data::ime::ImeTextSpanType::Composition => Self::Composition,
            cbf::data::ime::ImeTextSpanType::Suggestion => Self::Suggestion,
            cbf::data::ime::ImeTextSpanType::MisspellingSuggestion => Self::MisspellingSuggestion,
            cbf::data::ime::ImeTextSpanType::Autocorrect => Self::Autocorrect,
            cbf::data::ime::ImeTextSpanType::GrammarSuggestion => Self::GrammarSuggestion,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChromeImeTextSpanThickness {
    #[default]
    None,
    Thin,
    Thick,
}

impl From<ChromeImeTextSpanThickness> for cbf::data::ime::ImeTextSpanThickness {
    fn from(value: ChromeImeTextSpanThickness) -> Self {
        match value {
            ChromeImeTextSpanThickness::None => Self::None,
            ChromeImeTextSpanThickness::Thin => Self::Thin,
            ChromeImeTextSpanThickness::Thick => Self::Thick,
        }
    }
}

impl From<cbf::data::ime::ImeTextSpanThickness> for ChromeImeTextSpanThickness {
    fn from(value: cbf::data::ime::ImeTextSpanThickness) -> Self {
        match value {
            cbf::data::ime::ImeTextSpanThickness::None => Self::None,
            cbf::data::ime::ImeTextSpanThickness::Thin => Self::Thin,
            cbf::data::ime::ImeTextSpanThickness::Thick => Self::Thick,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChromeImeTextSpanUnderlineStyle {
    #[default]
    None,
    Solid,
    Dot,
    Dash,
    Squiggle,
}

impl From<ChromeImeTextSpanUnderlineStyle> for cbf::data::ime::ImeTextSpanUnderlineStyle {
    fn from(value: ChromeImeTextSpanUnderlineStyle) -> Self {
        match value {
            ChromeImeTextSpanUnderlineStyle::None => Self::None,
            ChromeImeTextSpanUnderlineStyle::Solid => Self::Solid,
            ChromeImeTextSpanUnderlineStyle::Dot => Self::Dot,
            ChromeImeTextSpanUnderlineStyle::Dash => Self::Dash,
            ChromeImeTextSpanUnderlineStyle::Squiggle => Self::Squiggle,
        }
    }
}

impl From<cbf::data::ime::ImeTextSpanUnderlineStyle> for ChromeImeTextSpanUnderlineStyle {
    fn from(value: cbf::data::ime::ImeTextSpanUnderlineStyle) -> Self {
        match value {
            cbf::data::ime::ImeTextSpanUnderlineStyle::None => Self::None,
            cbf::data::ime::ImeTextSpanUnderlineStyle::Solid => Self::Solid,
            cbf::data::ime::ImeTextSpanUnderlineStyle::Dot => Self::Dot,
            cbf::data::ime::ImeTextSpanUnderlineStyle::Dash => Self::Dash,
            cbf::data::ime::ImeTextSpanUnderlineStyle::Squiggle => Self::Squiggle,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChromeImeTextRange {
    pub start: i32,
    pub end: i32,
}

impl From<ChromeImeTextRange> for cbf::data::ime::ImeTextRange {
    fn from(value: ChromeImeTextRange) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

impl From<cbf::data::ime::ImeTextRange> for ChromeImeTextRange {
    fn from(value: cbf::data::ime::ImeTextRange) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChromeImeTextSpanStyle {
    pub underline_color: u32,
    pub thickness: ChromeImeTextSpanThickness,
    pub underline_style: ChromeImeTextSpanUnderlineStyle,
    pub text_color: u32,
    pub background_color: u32,
    pub suggestion_highlight_color: u32,
    pub remove_on_finish_composing: bool,
    pub interim_char_selection: bool,
    pub should_hide_suggestion_menu: bool,
}

impl From<ChromeImeTextSpanStyle> for cbf::data::ime::ImeTextSpanStyle {
    fn from(value: ChromeImeTextSpanStyle) -> Self {
        Self {
            underline_color: value.underline_color,
            thickness: value.thickness.into(),
            underline_style: value.underline_style.into(),
            text_color: value.text_color,
            background_color: value.background_color,
            suggestion_highlight_color: value.suggestion_highlight_color,
            remove_on_finish_composing: value.remove_on_finish_composing,
            interim_char_selection: value.interim_char_selection,
            should_hide_suggestion_menu: value.should_hide_suggestion_menu,
        }
    }
}

impl From<cbf::data::ime::ImeTextSpanStyle> for ChromeImeTextSpanStyle {
    fn from(value: cbf::data::ime::ImeTextSpanStyle) -> Self {
        Self {
            underline_color: value.underline_color,
            thickness: value.thickness.into(),
            underline_style: value.underline_style.into(),
            text_color: value.text_color,
            background_color: value.background_color,
            suggestion_highlight_color: value.suggestion_highlight_color,
            remove_on_finish_composing: value.remove_on_finish_composing,
            interim_char_selection: value.interim_char_selection,
            should_hide_suggestion_menu: value.should_hide_suggestion_menu,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeImeTextSpan {
    pub r#type: ChromeImeTextSpanType,
    pub start_offset: u32,
    pub end_offset: u32,
    pub chrome_style: Option<ChromeImeTextSpanStyle>,
}

impl ChromeImeTextSpan {
    pub fn new(r#type: ChromeImeTextSpanType, start_offset: u32, end_offset: u32) -> Self {
        Self {
            r#type,
            start_offset,
            end_offset,
            chrome_style: None,
        }
    }

    pub fn with_chrome_style(mut self, chrome_style: ChromeImeTextSpanStyle) -> Self {
        self.chrome_style = Some(chrome_style);
        self
    }

    pub fn no_decoration(
        r#type: ChromeImeTextSpanType,
        start_offset: u32,
        end_offset: u32,
    ) -> Self {
        Self {
            r#type,
            start_offset,
            end_offset,
            chrome_style: Some(ChromeImeTextSpanStyle::default()),
        }
    }
}

impl From<ChromeImeTextSpan> for cbf::data::ime::ImeTextSpan {
    fn from(value: ChromeImeTextSpan) -> Self {
        Self {
            r#type: value.r#type.into(),
            start_offset: value.start_offset,
            end_offset: value.end_offset,
            style: value.chrome_style.map(Into::into),
        }
    }
}

impl From<cbf::data::ime::ImeTextSpan> for ChromeImeTextSpan {
    fn from(value: cbf::data::ime::ImeTextSpan) -> Self {
        Self {
            r#type: value.r#type.into(),
            start_offset: value.start_offset,
            end_offset: value.end_offset,
            chrome_style: value.style.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeImeComposition {
    pub browsing_context_id: TabId,
    pub text: String,
    pub selection_start: i32,
    pub selection_end: i32,
    pub replacement_range: Option<ChromeImeTextRange>,
    pub spans: Vec<ChromeImeTextSpan>,
}

impl From<ChromeImeComposition> for cbf::data::ime::ImeComposition {
    fn from(value: ChromeImeComposition) -> Self {
        Self {
            browsing_context_id: value.browsing_context_id.into(),
            text: value.text,
            selection_start: value.selection_start,
            selection_end: value.selection_end,
            replacement_range: value.replacement_range.map(Into::into),
            spans: value.spans.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<cbf::data::ime::ImeComposition> for ChromeImeComposition {
    fn from(value: cbf::data::ime::ImeComposition) -> Self {
        Self {
            browsing_context_id: value.browsing_context_id.into(),
            text: value.text,
            selection_start: value.selection_start,
            selection_end: value.selection_end,
            replacement_range: value.replacement_range.map(Into::into),
            spans: value.spans.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeImeCommitText {
    pub browsing_context_id: TabId,
    pub text: String,
    pub relative_caret_position: i32,
    pub replacement_range: Option<ChromeImeTextRange>,
    pub spans: Vec<ChromeImeTextSpan>,
}

impl From<ChromeImeCommitText> for cbf::data::ime::ImeCommitText {
    fn from(value: ChromeImeCommitText) -> Self {
        Self {
            browsing_context_id: value.browsing_context_id.into(),
            text: value.text,
            relative_caret_position: value.relative_caret_position,
            replacement_range: value.replacement_range.map(Into::into),
            spans: value.spans.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<cbf::data::ime::ImeCommitText> for ChromeImeCommitText {
    fn from(value: cbf::data::ime::ImeCommitText) -> Self {
        Self {
            browsing_context_id: value.browsing_context_id.into(),
            text: value.text,
            relative_caret_position: value.relative_caret_position,
            replacement_range: value.replacement_range.map(Into::into),
            spans: value.spans.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeConfirmCompositionBehavior {
    DoNotKeepSelection,
    KeepSelection,
}

impl From<ChromeConfirmCompositionBehavior> for cbf::data::ime::ConfirmCompositionBehavior {
    fn from(value: ChromeConfirmCompositionBehavior) -> Self {
        match value {
            ChromeConfirmCompositionBehavior::DoNotKeepSelection => Self::DoNotKeepSelection,
            ChromeConfirmCompositionBehavior::KeepSelection => Self::KeepSelection,
        }
    }
}

impl From<cbf::data::ime::ConfirmCompositionBehavior> for ChromeConfirmCompositionBehavior {
    fn from(value: cbf::data::ime::ConfirmCompositionBehavior) -> Self {
        match value {
            cbf::data::ime::ConfirmCompositionBehavior::DoNotKeepSelection => {
                Self::DoNotKeepSelection
            }
            cbf::data::ime::ConfirmCompositionBehavior::KeepSelection => Self::KeepSelection,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChromeImeRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl From<ChromeImeRect> for cbf::data::ime::ImeRect {
    fn from(value: ChromeImeRect) -> Self {
        Self {
            x: value.x,
            y: value.y,
            width: value.width,
            height: value.height,
        }
    }
}

impl From<cbf::data::ime::ImeRect> for ChromeImeRect {
    fn from(value: cbf::data::ime::ImeRect) -> Self {
        Self {
            x: value.x,
            y: value.y,
            width: value.width,
            height: value.height,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeImeCompositionBounds {
    pub range_start: i32,
    pub range_end: i32,
    pub character_bounds: Vec<ChromeImeRect>,
}

impl From<ChromeImeCompositionBounds> for cbf::data::ime::ImeCompositionBounds {
    fn from(value: ChromeImeCompositionBounds) -> Self {
        Self {
            range_start: value.range_start,
            range_end: value.range_end,
            character_bounds: value.character_bounds.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<cbf::data::ime::ImeCompositionBounds> for ChromeImeCompositionBounds {
    fn from(value: cbf::data::ime::ImeCompositionBounds) -> Self {
        Self {
            range_start: value.range_start,
            range_end: value.range_end,
            character_bounds: value.character_bounds.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeTextSelectionBounds {
    pub range_start: i32,
    pub range_end: i32,
    pub caret_rect: ChromeImeRect,
    pub first_selection_rect: ChromeImeRect,
}

impl From<ChromeTextSelectionBounds> for cbf::data::ime::TextSelectionBounds {
    fn from(value: ChromeTextSelectionBounds) -> Self {
        Self {
            range_start: value.range_start,
            range_end: value.range_end,
            caret_rect: value.caret_rect.into(),
            first_selection_rect: value.first_selection_rect.into(),
        }
    }
}

impl From<cbf::data::ime::TextSelectionBounds> for ChromeTextSelectionBounds {
    fn from(value: cbf::data::ime::TextSelectionBounds) -> Self {
        Self {
            range_start: value.range_start,
            range_end: value.range_end,
            caret_rect: value.caret_rect.into(),
            first_selection_rect: value.first_selection_rect.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeImeBoundsUpdate {
    pub composition: Option<ChromeImeCompositionBounds>,
    pub selection: Option<ChromeTextSelectionBounds>,
}

impl From<ChromeImeBoundsUpdate> for cbf::data::ime::ImeBoundsUpdate {
    fn from(value: ChromeImeBoundsUpdate) -> Self {
        Self {
            composition: value.composition.map(Into::into),
            selection: value.selection.map(Into::into),
        }
    }
}

impl From<cbf::data::ime::ImeBoundsUpdate> for ChromeImeBoundsUpdate {
    fn from(value: cbf::data::ime::ImeBoundsUpdate) -> Self {
        Self {
            composition: value.composition.map(Into::into),
            selection: value.selection.map(Into::into),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ChromeImeBoundsUpdate, ChromeImeCommitText, ChromeImeComposition,
        ChromeImeCompositionBounds, ChromeImeRect, ChromeImeTextRange, ChromeImeTextSpan,
        ChromeImeTextSpanStyle, ChromeImeTextSpanThickness, ChromeImeTextSpanType,
        ChromeImeTextSpanUnderlineStyle, ChromeTextSelectionBounds,
    };
    use crate::data::ids::TabId;

    #[test]
    fn ime_text_span_round_trip_preserves_chrome_style() {
        let raw = ChromeImeTextSpan {
            r#type: ChromeImeTextSpanType::GrammarSuggestion,
            start_offset: 2,
            end_offset: 7,
            chrome_style: Some(ChromeImeTextSpanStyle {
                underline_color: 0x00112233,
                thickness: ChromeImeTextSpanThickness::Thick,
                underline_style: ChromeImeTextSpanUnderlineStyle::Squiggle,
                text_color: 0x00445566,
                background_color: 0x00778899,
                suggestion_highlight_color: 0x00AABBCC,
                remove_on_finish_composing: true,
                interim_char_selection: false,
                should_hide_suggestion_menu: true,
            }),
        };

        let generic: cbf::data::ime::ImeTextSpan = raw.clone().into();
        let round_trip = ChromeImeTextSpan::from(generic);

        assert_eq!(round_trip, raw);
    }

    #[test]
    fn ime_composition_and_commit_round_trip() {
        let composition = ChromeImeComposition {
            browsing_context_id: TabId::new(42),
            text: "あいう".to_string(),
            selection_start: 1,
            selection_end: 3,
            replacement_range: Some(ChromeImeTextRange { start: 0, end: 2 }),
            spans: vec![ChromeImeTextSpan::new(
                ChromeImeTextSpanType::Composition,
                0,
                3,
            )],
        };
        let commit = ChromeImeCommitText {
            browsing_context_id: TabId::new(42),
            text: "確定".to_string(),
            relative_caret_position: -1,
            replacement_range: Some(ChromeImeTextRange { start: 0, end: 3 }),
            spans: vec![ChromeImeTextSpan::no_decoration(
                ChromeImeTextSpanType::Suggestion,
                0,
                2,
            )],
        };

        let composition_generic: cbf::data::ime::ImeComposition = composition.clone().into();
        let commit_generic: cbf::data::ime::ImeCommitText = commit.clone().into();

        assert_eq!(ChromeImeComposition::from(composition_generic), composition);
        assert_eq!(ChromeImeCommitText::from(commit_generic), commit);
    }

    #[test]
    fn ime_bounds_update_round_trip() {
        let raw = ChromeImeBoundsUpdate {
            composition: Some(ChromeImeCompositionBounds {
                range_start: 0,
                range_end: 2,
                character_bounds: vec![
                    ChromeImeRect {
                        x: 10,
                        y: 20,
                        width: 30,
                        height: 40,
                    },
                    ChromeImeRect {
                        x: 50,
                        y: 60,
                        width: 70,
                        height: 80,
                    },
                ],
            }),
            selection: Some(ChromeTextSelectionBounds {
                range_start: 1,
                range_end: 2,
                caret_rect: ChromeImeRect {
                    x: 100,
                    y: 200,
                    width: 10,
                    height: 20,
                },
                first_selection_rect: ChromeImeRect {
                    x: 110,
                    y: 210,
                    width: 15,
                    height: 25,
                },
            }),
        };

        let generic: cbf::data::ime::ImeBoundsUpdate = raw.clone().into();
        let round_trip = ChromeImeBoundsUpdate::from(generic);

        assert_eq!(round_trip, raw);
    }
}
