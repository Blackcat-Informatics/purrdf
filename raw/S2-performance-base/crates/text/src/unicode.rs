// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The analyzer's Unicode layer: full case folding, the four normalization
//! forms, and word boundaries, over tables generated from the vendored Unicode
//! Character Database.
//!
//! * [`case_fold`] — full default case folding, the `C` and `F` mappings of
//!   `CaseFolding.txt` (Unicode core specification §3.13).
//! * [`nfd`], [`nfc`], [`nfkd`], [`nfkc`] — the normalization forms of
//!   `UAX #15`, re-exported from [`purrdf_lex::unicode`], the workspace's one
//!   normalization pipeline; the analysis form composes its stages with this
//!   module's case fold.
//! * [`analysis_form`] — `NFD → fold → NFKD → fold → NFKD → NFC`, the
//!   compatibility caseless form (§3.13 D146) completed with NFC.
//! * [`word_bounds`], [`word_indices`] — the default word boundaries of
//!   `UAX #29` (rules WB1 to WB999); [`word_indices`] keeps only segments that
//!   hold an [`is_alphanumeric`] character.
//!
//! Every table is a pure function of the vendored database, generated with
//! every other Unicode table in the workspace at one version, so the answers
//! do not depend on the toolchain the crate is built with.
//!
//! # The ASCII fast path
//!
//! An ASCII run's full case fold is its ASCII lowercase. The pipeline is driven
//! by [`purrdf_lex::unicode::drive`], which hands ASCII runs to every stage
//! whole (see its module documentation for why that is exact); the `Fold`
//! stage lowercases such a run in bulk instead of folding it per character.

use purrdf_lex::unicode::{
    Collect, Compose, Decompose, Out, Stage, collect, drive, lookup_two_stage,
};

use crate::unicode_tables as tables;

pub use purrdf_lex::unicode::{Compare, Sink, nfc, nfd, nfkc, nfkd};

mod grapheme;
pub(crate) use grapheme::emoji_scalars;
pub use grapheme::{
    EmojiStatus, GraphemeBounds, emoji_status, grapheme_bounds, is_conjunct_consonant,
    is_emoji_grapheme,
};

/// Largest recognized emoji atom, calculated from the pinned Unicode data.
pub const MAX_EMOJI_SCALARS: usize = tables::MAX_EMOJI_SCALARS;
/// Identity of all semantic Unicode analysis inputs, independent of compiler.
pub const TEXT_DATA_DIGEST: [u8; 32] = tables::TEXT_DATA_DIGEST;

/// The Unicode version of the normalization, word-break and alphanumeric
/// tables: the workspace's one, [`purrdf_lex::unicode::UNICODE_VERSION`].
pub const UNICODE_VERSION: (u8, u8, u8) = purrdf_lex::unicode::UNICODE_VERSION;

/// The Unicode version of the `CaseFolding.txt` the fold tables come from.
pub const FOLD_UNICODE_VERSION: (u8, u8, u8) = purrdf_lex::unicode::UNICODE_VERSION;

/// The segmentation byte: `Word_Break` value and flags.
#[inline]
fn segmentation(c: char) -> u8 {
    lookup_two_stage(&tables::SEGMENTATION_INDEX, &tables::SEGMENTATION_BLOCKS, c)
}

/// The generated analysis-property word: marks, controls and script routing.
#[inline]
fn analysis_properties(c: char) -> u32 {
    lookup_two_stage(&tables::ANALYSIS_INDEX, &tables::ANALYSIS_BLOCKS, c)
}

/// Whether `c` has General_Category `Mn` in the pinned Unicode database.
///
/// This is not a combining-class test: a nonspacing mark may have class zero.
#[must_use]
pub fn is_nonspacing_mark(c: char) -> bool {
    analysis_properties(c) & tables::ANALYSIS_NONSPACING != 0
}

/// Whether `c` has General_Category `Mn`, `Mc` or `Me` in the pinned database.
#[must_use]
pub fn is_combining_mark(c: char) -> bool {
    analysis_properties(c) & tables::ANALYSIS_MARK != 0
}

/// Unicode `Default_Ignorable_Code_Point`, including reserved default ignorables.
#[must_use]
pub fn is_default_ignorable(c: char) -> bool {
    analysis_properties(c) & tables::ANALYSIS_IGNORABLE != 0
}

/// Unicode `Bidi_Control`, as declared in the pinned `PropList.txt`.
#[must_use]
pub fn is_bidi_control(c: char) -> bool {
    analysis_properties(c) & tables::ANALYSIS_BIDI != 0
}

/// Unicode `White_Space` or General_Category `Cc`/`Cf`.
///
/// These characters separate dictionary lookup chunks and are refused inside
/// dictionary entries. Values come from the pinned database, not the compiler.
#[must_use]
pub fn is_lexical_separator(c: char) -> bool {
    c == '\u{200b}'
        || (analysis_properties(c) & tables::ANALYSIS_SEPARATOR != 0
            && !is_word_internal_control(c)
            && (is_whitespace_separator(c) || is_removed_control(c)))
}

/// Exact General_Category L, from the pinned data rather than compiler tables.
#[must_use]
pub fn is_letter(c: char) -> bool {
    analysis_properties(c) & tables::ANALYSIS_LETTER != 0
}

/// Whether a letter belongs to Latin Script or Script_Extensions.
#[must_use]
pub fn is_latin_letter(c: char) -> bool {
    is_letter(c) && has_analysis_script(c, tables::SCRIPT_LATIN)
}

/// Exact White_Space plus ZWSP, which the search law declares a separator.
#[must_use]
pub fn is_whitespace_separator(c: char) -> bool {
    c == '\u{200b}' || analysis_properties(c) & tables::ANALYSIS_WHITESPACE != 0
}

/// Controls transparent to dictionary keys and removed only from lexical words.
/// ZWJ reaches this stage only after contextual preservation or stray removal.
#[must_use]
pub const fn is_word_internal_control(c: char) -> bool {
    matches!(c, '\u{ad}' | '\u{200c}' | '\u{200d}' | '\u{180b}'..='\u{180d}' | '\u{180f}'
        | '\u{fe00}'..='\u{fe0f}' | '\u{e0100}'..='\u{e01ef}')
}

/// Explicit removed control set; whitespace has separator behavior instead.
#[must_use]
pub fn is_removed_control(c: char) -> bool {
    matches!(c, '\u{61c}' | '\u{200e}'..='\u{200f}' | '\u{202a}'..='\u{202e}'
        | '\u{2066}'..='\u{2069}' | '\u{2060}' | '\u{feff}' | '\u{e0001}' | '\u{e0020}'..='\u{e007f}')
        || (matches!(c, '\0'..='\u{1f}' | '\u{7f}'..='\u{9f}') && !is_whitespace_separator(c))
}

/// Controls removed before Latin phonetic coding; separators remain inadmissible.
#[must_use]
pub fn is_phonetic_control(c: char) -> bool {
    is_word_internal_control(c) || is_removed_control(c)
}

/// A joining-script letter or character, used for orthographic ZWJ context.
#[must_use]
pub fn is_joining_character(c: char) -> bool {
    analysis_properties(c) & tables::ANALYSIS_JOINING != 0
}

#[inline]
fn has_analysis_script(c: char, value: u32) -> bool {
    let properties = analysis_properties(c);
    let own = (properties >> tables::ANALYSIS_SCRIPT_SHIFT) & 0xF;
    let extensions = (properties >> tables::ANALYSIS_EXTENSIONS_SHIFT) & 0x1FFF;
    own == value || extensions & (1 << (value - 1)) != 0
}

/// Script assignments that the declared accent fold may select.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum AccentScript {
    /// Latin letters and diacritics.
    Latin = 0,
    /// Greek letters and diacritics.
    Greek = 1,
    /// Cyrillic letters and diacritics.
    Cyrillic = 2,
    /// Hebrew points, explicitly selected.
    Hebrew = 3,
    /// Arabic harakat, explicitly selected.
    Arabic = 4,
}

impl AccentScript {
    const fn value(self) -> u32 {
        match self {
            Self::Latin => tables::SCRIPT_LATIN,
            Self::Greek => tables::SCRIPT_GREEK,
            Self::Cyrillic => tables::SCRIPT_CYRILLIC,
            Self::Hebrew => tables::SCRIPT_HEBREW,
            Self::Arabic => tables::SCRIPT_ARABIC,
        }
    }

    /// Script membership of a base letter.
    pub fn contains(self, c: char) -> bool {
        is_letter(c) && has_analysis_script(c, self.value())
    }

    /// Whether an Mn mark can modify a base of this script. Explicit extensions
    /// to unsupported scripts do not become a wildcard merely by being absent
    /// from the small search-script enumeration.
    pub fn admits_mark(self, c: char) -> bool {
        if !is_nonspacing_mark(c) {
            return false;
        }
        let properties = analysis_properties(c);
        has_analysis_script(c, self.value())
            || (properties & tables::ANALYSIS_NEUTRAL_SCRIPT != 0
                && properties & tables::ANALYSIS_EXPLICIT_EXTENSIONS == 0)
    }
}

/// Unicode `Extended_Pictographic`, used to recognize emoji joiner context.
#[must_use]
pub fn is_extended_pictographic(c: char) -> bool {
    segmentation(c) & tables::EXTENDED_PICTOGRAPHIC != 0
}

/// Whether `c` has `Word_Break=Extend`, including emoji modifiers.
#[must_use]
pub fn is_word_extend(c: char) -> bool {
    segmentation(c) & tables::WB_MASK == tables::WB_EXTEND
}

/// A Unicode `Script` routed through the dictionary segmenter.
///
/// These are exact `Scripts.txt` assignments, not approximations by block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SegmentationScript {
    /// Han ideographs, including the supplementary extensions.
    Han,
    /// Hiragana.
    Hiragana,
    /// Katakana.
    Katakana,
    /// Hangul, including conjoining jamo.
    Hangul,
    /// Thai.
    Thai,
    /// Lao.
    Lao,
    /// Khmer.
    Khmer,
    /// Myanmar.
    Myanmar,
}

/// The exact supported `Script` of `c`, or `None` for every other assignment.
///
/// Inherited marks and Common punctuation have no script here. The segmenter
/// attaches combining marks to a preceding base and keeps punctuation as a
/// boundary, rather than guessing a script for either.
#[must_use]
pub fn segmentation_script(c: char) -> Option<SegmentationScript> {
    match (analysis_properties(c) >> tables::ANALYSIS_SCRIPT_SHIFT) & 0xF {
        tables::SCRIPT_HAN => Some(SegmentationScript::Han),
        tables::SCRIPT_HIRAGANA => Some(SegmentationScript::Hiragana),
        tables::SCRIPT_KATAKANA => Some(SegmentationScript::Katakana),
        tables::SCRIPT_HANGUL => Some(SegmentationScript::Hangul),
        tables::SCRIPT_THAI => Some(SegmentationScript::Thai),
        tables::SCRIPT_LAO => Some(SegmentationScript::Lao),
        tables::SCRIPT_KHMER => Some(SegmentationScript::Khmer),
        tables::SCRIPT_MYANMAR => Some(SegmentationScript::Myanmar),
        _ => None,
    }
}

/// Whether `c` belongs to a supported Script or its Script_Extensions.
///
/// This admits shared letters such as the Japanese prolonged sound mark, whose
/// Script is Common and whose Script_Extensions are Hiragana and Katakana.
/// Punctuation can also have script membership; tokenizers still decide lexical
/// membership separately with [`is_alphanumeric`] and [`is_combining_mark`].
#[must_use]
pub fn has_segmentation_script(c: char, script: SegmentationScript) -> bool {
    let value = match script {
        SegmentationScript::Han => tables::SCRIPT_HAN,
        SegmentationScript::Hiragana => tables::SCRIPT_HIRAGANA,
        SegmentationScript::Katakana => tables::SCRIPT_KATAKANA,
        SegmentationScript::Hangul => tables::SCRIPT_HANGUL,
        SegmentationScript::Thai => tables::SCRIPT_THAI,
        SegmentationScript::Lao => tables::SCRIPT_LAO,
        SegmentationScript::Khmer => tables::SCRIPT_KHMER,
        SegmentationScript::Myanmar => tables::SCRIPT_MYANMAR,
    };
    let properties = analysis_properties(c);
    let own = (properties >> tables::ANALYSIS_SCRIPT_SHIFT) & 0xF;
    let extensions = (properties >> tables::ANALYSIS_EXTENSIONS_SHIFT) & 0x1FFF;
    own == value || extensions & (1 << (value - 1)) != 0
}

/// Whether `c` is `Alphabetic`, or of General_Category `Nd`, `Nl` or `No`:
/// the predicate the word filter keeps a segment by.
#[must_use]
pub fn is_alphanumeric(c: char) -> bool {
    if c.is_ascii() {
        c.is_ascii_alphanumeric()
    } else {
        segmentation(c) & tables::ALPHANUMERIC != 0
    }
}

/// Full case folding: `CaseFolding.txt` statuses `C` and `F`, as a stage of a
/// [`purrdf_lex::unicode`] pipeline.
struct Fold<N> {
    next: N,
}

/// Full C/F case mapping shared by streaming and source-tagged normalization.
pub(crate) fn fold_scalar(c: char, mut emit: impl FnMut(char)) {
    if c.is_ascii() {
        emit(c.to_ascii_lowercase());
        return;
    }
    let record = lookup_two_stage(&tables::FOLD_INDEX, &tables::FOLD_BLOCKS, c);
    if record == 0 {
        emit(c);
        return;
    }
    let (start, len) = tables::FOLD_RECORDS[usize::from(record)];
    let start = usize::from(start);
    for &part in &tables::FOLD_CHARS[start..start + usize::from(len)] {
        emit(part);
    }
}

impl<N: Stage> Stage for Fold<N> {
    #[inline]
    fn push(&mut self, c: char) {
        fold_scalar(c, |part| self.next.push(part));
    }

    fn push_ascii(&mut self, ascii: &str, _lowercase: bool) {
        self.next.push_ascii(ascii, true);
    }

    fn flush(&mut self) {
        self.next.flush();
    }
}

/// The full case fold of `text`: every character replaced by its
/// `CaseFolding.txt` mapping of status `C` or `F` (core specification §3.13).
#[must_use]
pub fn case_fold(text: &str) -> String {
    collect(text, |next: Collect| Fold { next }, |stage| stage.next)
}

/// Append the analysis form of `input` to `out`:
/// `NFC(NFKD(fold(NFKD(fold(NFD(input))))))`.
///
/// The inner five steps are the compatibility caseless form of the core
/// specification §3.13 (D146); two strings have the same analysis form exactly
/// when they are compatibility caseless matches.
pub fn analysis_form<O: ?Sized + Sink>(input: &str, out: &mut O) {
    let mut pipeline = Decompose::<false, _>::new(Fold {
        next: Decompose::<true, _>::new(Fold {
            next: Decompose::<true, _>::new(Compose::new(Out(out))),
        }),
    });
    drive(input, &mut pipeline);
}

// ---------------------------------------------------------------------------
// Word boundaries, UAX 29.

/// Before the first character (`sot`), and after the last (`eot`).
const SOT: u8 = 0xFE;
const EOT: u8 = 0xFF;

/// `AHLetter`.
#[inline]
const fn is_ah_letter(value: u8) -> bool {
    value == tables::WB_ALETTER || value == tables::WB_HEBREW_LETTER
}

/// `MidLetter | MidNumLetQ`.
#[inline]
const fn is_mid_letter_q(value: u8) -> bool {
    value == tables::WB_MIDLETTER
        || value == tables::WB_MIDNUMLET
        || value == tables::WB_SINGLE_QUOTE
}

/// `MidNum | MidNumLetQ`.
#[inline]
const fn is_mid_num_q(value: u8) -> bool {
    value == tables::WB_MIDNUM || value == tables::WB_MIDNUMLET || value == tables::WB_SINGLE_QUOTE
}

/// `Extend | Format | ZWJ`, the characters rule WB4 attaches to what precedes
/// them.
#[inline]
const fn is_ignorable(value: u8) -> bool {
    value == tables::WB_EXTEND || value == tables::WB_FORMAT || value == tables::WB_ZWJ
}

/// `Newline | CR | LF`.
#[inline]
const fn is_newline(value: u8) -> bool {
    value == tables::WB_NEWLINE || value == tables::WB_CR || value == tables::WB_LF
}

/// The `Word_Break` value of the first character of `text` that rule WB4 does
/// not attach to its predecessor, or [`EOT`].
fn next_effective(text: &str) -> u8 {
    text.chars()
        .map(|c| segmentation(c) & tables::WB_MASK)
        .find(|&value| !is_ignorable(value))
        .unwrap_or(EOT)
}

/// A forward pass over the default word boundaries.
#[derive(Clone, Debug)]
struct Boundaries<'a> {
    text: &'a str,
    /// The byte offset where the next segment starts.
    start: usize,
    /// The `Word_Break` value of the character before `start`, as written.
    raw: u8,
    /// The value of the last character WB4 did not attach to its predecessor.
    last: u8,
    /// The value of the such character before `last`.
    before_last: u8,
    /// How many `Regional_Indicator`s end at `last`, counting only those WB4
    /// left in place.
    regional: usize,
}

impl<'a> Boundaries<'a> {
    const fn new(text: &'a str) -> Self {
        Self {
            text,
            start: 0,
            raw: SOT,
            last: SOT,
            before_last: SOT,
            regional: 0,
        }
    }

    /// Take `value` into the left context.
    #[inline]
    fn accept(&mut self, value: u8) {
        // WB4: X (Extend | Format | ZWJ)* → X, except after sot, CR, LF and
        // Newline.
        let attached = is_ignorable(value) && self.raw != SOT && !is_newline(self.raw);
        self.raw = value;
        if !attached {
            self.regional = if value == tables::WB_REGIONAL_INDICATOR {
                if self.last == tables::WB_REGIONAL_INDICATOR {
                    self.regional + 1
                } else {
                    1
                }
            } else {
                0
            };
            self.before_last = self.last;
            self.last = value;
        }
    }

    /// Whether there is a boundary before a character with segmentation byte
    /// `byte`, followed by `rest`. The rules in their numbered order.
    fn breaks_before(&self, byte: u8, rest: &str) -> bool {
        let value = byte & tables::WB_MASK;
        let (raw, last, before_last) = (self.raw, self.last, self.before_last);
        // WB3: CR × LF.
        if raw == tables::WB_CR && value == tables::WB_LF {
            return false;
        }
        // WB3a, WB3b: break after and before Newline | CR | LF.
        if is_newline(raw) || is_newline(value) {
            return true;
        }
        // WB3c: ZWJ × \p{Extended_Pictographic}.
        if raw == tables::WB_ZWJ && byte & tables::EXTENDED_PICTOGRAPHIC != 0 {
            return false;
        }
        // WB3d: WSegSpace × WSegSpace.
        if raw == tables::WB_WSEGSPACE && value == tables::WB_WSEGSPACE {
            return false;
        }
        // WB4: no break before Extend | Format | ZWJ (sot, CR, LF and Newline
        // on the left are decided above).
        if is_ignorable(value) {
            return false;
        }
        let numeric = tables::WB_NUMERIC;
        let katakana = tables::WB_KATAKANA;
        let extend_num_let = tables::WB_EXTENDNUMLET;
        let hebrew = tables::WB_HEBREW_LETTER;
        let joins = (is_ah_letter(last) && is_ah_letter(value)) // WB5
            || (is_ah_letter(last) && is_mid_letter_q(value) && is_ah_letter(next_effective(rest))) // WB6
            || (is_ah_letter(before_last) && is_mid_letter_q(last) && is_ah_letter(value)) // WB7
            || (last == hebrew && value == tables::WB_SINGLE_QUOTE) // WB7a
            || (last == hebrew && value == tables::WB_DOUBLE_QUOTE && next_effective(rest) == hebrew) // WB7b
            || (before_last == hebrew && last == tables::WB_DOUBLE_QUOTE && value == hebrew) // WB7c
            || (last == numeric && value == numeric) // WB8
            || (is_ah_letter(last) && value == numeric) // WB9
            || (last == numeric && is_ah_letter(value)) // WB10
            || (before_last == numeric && is_mid_num_q(last) && value == numeric) // WB11
            || (last == numeric && is_mid_num_q(value) && next_effective(rest) == numeric) // WB12
            || (last == katakana && value == katakana) // WB13
            || ((is_ah_letter(last) || last == numeric || last == katakana || last == extend_num_let)
                && value == extend_num_let) // WB13a
            || (last == extend_num_let
                && (is_ah_letter(value) || value == numeric || value == katakana)) // WB13b
            || (last == tables::WB_REGIONAL_INDICATOR
                && value == tables::WB_REGIONAL_INDICATOR
                && self.regional % 2 == 1); // WB15, WB16
        // WB999: otherwise, break everywhere.
        !joins
    }

    /// The next segment and its byte offset.
    fn next_segment(&mut self) -> Option<(usize, &'a str)> {
        let text = self.text;
        let start = self.start;
        let rest = &text[start..];
        let mut chars = rest.char_indices();
        // WB1: the boundary before the first character, and the boundary
        // before this segment, are already decided.
        let (_, first) = chars.next()?;
        self.accept(segmentation(first) & tables::WB_MASK);
        let mut end = rest.len();
        for (offset, c) in chars {
            let byte = segmentation(c);
            if self.breaks_before(byte, &rest[offset + c.len_utf8()..]) {
                end = offset;
                break;
            }
            self.accept(byte & tables::WB_MASK);
        }
        // WB2: the end of text is a boundary.
        self.start = start + end;
        Some((start, &rest[..end]))
    }
}

/// The segments of `text` between default word boundaries (`UAX #29`), every
/// one of them, in order. Concatenated, they are `text`.
#[must_use]
pub fn word_bounds(text: &str) -> WordBounds<'_> {
    WordBounds(Boundaries::new(text))
}

/// The word segments of `text` with their byte offsets: the segments of
/// [`word_bounds`] that hold at least one [`is_alphanumeric`] character.
#[must_use]
pub fn word_indices(text: &str) -> WordIndices<'_> {
    WordIndices(Boundaries::new(text))
}

/// The iterator [`word_bounds`] returns.
#[derive(Clone, Debug)]
pub struct WordBounds<'a>(Boundaries<'a>);

impl<'a> Iterator for WordBounds<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        self.0.next_segment().map(|(_, segment)| segment)
    }
}

/// The iterator [`word_indices`] returns.
#[derive(Clone, Debug)]
pub struct WordIndices<'a>(Boundaries<'a>);

impl<'a> Iterator for WordIndices<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<(usize, &'a str)> {
        loop {
            let (start, segment) = self.0.next_segment()?;
            if segment.chars().any(is_alphanumeric) {
                return Some((start, segment));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use purrdf_lex::unicode::{Compose, Decompose, Out, Stage, nfd};

    use super::{Fold, analysis_form};

    /// The analysis form with no ASCII bypass: every character through the
    /// pipeline.
    fn analysis_form_without_bypass(input: &str) -> String {
        let mut out = String::new();
        let mut pipeline = Decompose::<false, _>::new(Fold {
            next: Decompose::<true, _>::new(Fold {
                next: Decompose::<true, _>::new(Compose::new(Out(&mut out))),
            }),
        });
        input.chars().for_each(|c| pipeline.push(c));
        pipeline.flush();
        out
    }

    /// The ASCII bypass is exact through the fold: every ASCII character,
    /// alone and before and after every precomposed character outside the
    /// Hangul syllables (and its decomposition), and every character whose
    /// fold or composition is special, analyzes the same with and without it.
    #[test]
    fn the_ascii_bypass_agrees_with_the_pipeline() {
        let mut interesting: Vec<char> = (0x80..0x3_0000_u32)
            .filter(|point| !(0xAC00..=0xD7A3).contains(point))
            .filter_map(char::from_u32)
            .filter(|&c| {
                let decomposed = nfd(&c.to_string());
                decomposed.chars().count() == 2 && decomposed != c.to_string()
            })
            .flat_map(|c| {
                let mut chars: Vec<char> = nfd(&c.to_string()).chars().collect();
                chars.push(c);
                chars
            })
            .collect();
        interesting.extend([
            '\u{345}', '\u{212A}', '\u{FB01}', '\u{1E9E}', '\u{130}', '\u{AC00}', '\u{1161}',
            '\u{11A8}',
        ]);
        interesting.sort_unstable();
        interesting.dedup();
        for ascii in (0u8..0x80).map(char::from) {
            for &other in &interesting {
                for input in [
                    format!("{ascii}{other}"),
                    format!("{other}{ascii}"),
                    format!("ABCDEFGH{ascii}{other}{ascii}xyzXYZ12"),
                    format!("{other}{other}{ascii}{ascii}{other}"),
                ] {
                    let mut bypassed = String::new();
                    analysis_form(&input, &mut bypassed);
                    assert_eq!(bypassed, analysis_form_without_bypass(&input), "{input:?}");
                }
            }
        }
    }
}
