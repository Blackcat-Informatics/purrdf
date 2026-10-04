// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Immutable, fully identified analysis choices.
use crate::{TextError, segment::Dictionary, unicode::AccentScript};
use purrdf_hash::{Domain, blake3, frame::frame_le};
use std::sync::Arc;
const ORDERED_LAW: Domain =
    Domain::new(b"references-once-emoji-protect-caseless-scoped-accents-v3");
const LEXICAL_LAW: Domain = Domain::new(b"lexical-transparent-controls-letter-dot-colon-v3");
const BOUNDING_LAW: Domain = Domain::new(b"stem-then-whole-egc-prefix-first-oversized-intact-v1");
const BASELINE_LAW: Domain = Domain::new(b"five-full-baselines-minimum-collision-cost-v1");
const SUBSTRING_LAW: Domain =
    Domain::new(b"positional-grams-1-2-3-relative-offsets-emoji-endpoints-rarest-anchor-v1");

const PROFILE_DOMAIN: Domain = Domain::new(b"purrdf-text-analysis-profile-v2\0");
/// Largest admitted nominal token bound.
pub const MAX_TOKEN_SCALARS: usize = 1024;
/// Any subset of the five supported accent-fold scripts.
///
/// Construction accepts typed script members, never numeric flags. Repetition
/// and input order do not change the set; the empty set preserves every mark.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct AccentScripts(u8);
impl AccentScripts {
    const MEMBERS: [AccentScript; 5] = [
        AccentScript::Latin,
        AccentScript::Greek,
        AccentScript::Cyrillic,
        AccentScript::Arabic,
        AccentScript::Hebrew,
    ];

    /// Collect script members into an order-independent set.
    pub fn from_scripts(scripts: impl IntoIterator<Item = AccentScript>) -> Self {
        scripts.into_iter().fold(Self::default(), Self::with)
    }
    /// Select another typed script; selecting an existing member changes nothing.
    #[must_use]
    pub const fn with(mut self, script: AccentScript) -> Self {
        self.0 |= 1 << script as u8;
        self
    }
    /// Whether this set selects a script.
    pub const fn contains(self, script: AccentScript) -> bool {
        self.0 & (1 << script as u8) != 0
    }
    /// Whether accent marks are preserved in every script.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
    /// Members in stable Latin, Greek, Cyrillic, Arabic, Hebrew order.
    pub fn iter(self) -> impl Iterator<Item = AccentScript> {
        Self::MEMBERS
            .into_iter()
            .filter(move |&script| self.contains(script))
    }
}
/// Script-scoped removal of nonspacing accent marks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AccentFold {
    /// Retain every mark.
    Preserve,
    /// Fold marks attached to Latin, Greek or Cyrillic letters.
    #[default]
    LatinGreekCyrillic,
    /// Also fold marks attached to Arabic and Hebrew letters.
    LatinGreekCyrillicArabicHebrew,
    /// Independently select any subset; an empty set preserves every mark.
    Selected(AccentScripts),
}
impl AccentFold {
    /// The semantic script set, independent of its preset or explicit spelling.
    pub const fn scripts(self) -> AccentScripts {
        match self {
            Self::Preserve => AccentScripts(0),
            Self::LatinGreekCyrillic => AccentScripts(7),
            Self::LatinGreekCyrillicArabicHebrew => AccentScripts(31),
            Self::Selected(scripts) => scripts,
        }
    }
    const fn canonical(self) -> Self {
        match self.scripts().0 {
            0 => Self::Preserve,
            7 => Self::LatinGreekCyrillic,
            31 => Self::LatinGreekCyrillicArabicHebrew,
            _ => self,
        }
    }
    const fn fingerprint_code(self) -> u8 {
        // Preserve the existing preset preimages exactly. Other sets occupy a
        // disjoint byte range; aliases, repetitions and order never enter it.
        match self.scripts().0 {
            0 => 0,
            7 => 1,
            31 => 2,
            mask => 32 | mask,
        }
    }
}
/// Explicit stemming language.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Stemming {
    /// Preserve lexical spelling.
    #[default]
    None,
    /// English Snowball, with the pinned Latin scalar domain.
    English,
}
/// Dictionary selection; baseline artifacts are always caller-supplied bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Segmentation {
    /// Resolve all five pinned full baseline artifacts.
    #[default]
    Baseline,
    /// Explicit grapheme fallback for unspaced scripts; no hidden dictionary.
    EmptyLexicon,
    /// A caller's immutable costed lexicon.
    Dictionary(Arc<Dictionary>),
}
/// Literal interpretation before Unicode analysis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InputMode {
    /// Plain Unicode text.
    #[default]
    Plain,
    /// Strict HTML text-context character references, decoded once.
    HtmlText,
    /// Strict HTML attribute-context character references, decoded once.
    HtmlAttribute,
}
/// A validated analysis law, independent of artifact storage representation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AnalyzerProfile {
    max_token_scalars: usize,
    accent: AccentFold,
    stemming: Stemming,
    segmentation: Segmentation,
    input_mode: InputMode,
    code_length: usize,
    edit_distance: usize,
    substring_limits: crate::surface::SubstringLimits,
}
impl AnalyzerProfile {
    /// Standard law: 128 scalars, scoped accents, five full baseline lexicons.
    pub const fn standard() -> Self {
        Self {
            max_token_scalars: 128,
            accent: AccentFold::LatinGreekCyrillic,
            stemming: Stemming::None,
            segmentation: Segmentation::Baseline,
            input_mode: InputMode::Plain,
            code_length: 4,
            edit_distance: 2,
            substring_limits: crate::surface::SubstringLimits::STANDARD,
        }
    }
    /// Explicit profile without dictionary artifacts.
    pub const fn empty_lexicon() -> Self {
        Self {
            max_token_scalars: 128,
            accent: AccentFold::LatinGreekCyrillic,
            stemming: Stemming::None,
            segmentation: Segmentation::EmptyLexicon,
            input_mode: InputMode::Plain,
            code_length: 4,
            edit_distance: 2,
            substring_limits: crate::surface::SubstringLimits::STANDARD,
        }
    }
    /// Set the nominal final projection bound.
    /// # Errors
    /// Refuses bounds below the longest recognized emoji or above 1024.
    pub fn new(max_token_scalars: usize) -> Result<Self, TextError> {
        if !(crate::unicode::MAX_EMOJI_SCALARS..=MAX_TOKEN_SCALARS).contains(&max_token_scalars) {
            return Err(TextError::config(
                "token bound must admit the longest recognized emoji and be at most 1024",
            ));
        }
        Ok(Self {
            max_token_scalars,
            ..Self::standard()
        })
    }
    /// Select accent folding.
    #[must_use]
    pub const fn with_accent_fold(mut self, accent: AccentFold) -> Self {
        self.accent = accent.canonical();
        self
    }
    /// Select stemming.
    #[must_use]
    pub const fn with_stemming(mut self, stemming: Stemming) -> Self {
        self.stemming = stemming;
        self
    }
    /// Select dictionary resolution.
    #[must_use]
    pub fn with_segmentation(mut self, segmentation: Segmentation) -> Self {
        self.segmentation = segmentation;
        self
    }
    /// Select character-reference interpretation.
    #[must_use]
    pub const fn with_input_mode(mut self, input_mode: InputMode) -> Self {
        self.input_mode = input_mode;
        self
    }
    /// Select bounded phonetic coding and refinement.
    /// # Errors
    /// Refuses code lengths outside 1..=64 or distances above 64.
    pub fn with_phonetics(
        mut self,
        code_length: usize,
        edit_distance: usize,
    ) -> Result<Self, TextError> {
        crate::phonetic::DoubleMetaphone::new(code_length)?;
        if edit_distance > crate::phonetic::MAX_EDIT_DISTANCE {
            return Err(TextError::config("phonetic edit distance exceeds 64"));
        }
        self.code_length = code_length;
        self.edit_distance = edit_distance;
        Ok(self)
    }
    /// Nominal scalar bound, applied to whole grapheme prefixes.
    pub const fn max_token_scalars(&self) -> usize {
        self.max_token_scalars
    }
    /// Accent law.
    pub const fn accent_fold(&self) -> AccentFold {
        self.accent
    }
    /// Stemming law.
    pub const fn stemming(&self) -> Stemming {
        self.stemming
    }
    /// Dictionary selection.
    pub const fn segmentation(&self) -> &Segmentation {
        &self.segmentation
    }
    /// Input interpretation.
    pub const fn input_mode(&self) -> InputMode {
        self.input_mode
    }
    /// Maximum phonetic code length.
    pub const fn code_length(&self) -> usize {
        self.code_length
    }
    /// Maximum phonetic edit distance.
    pub const fn edit_distance(&self) -> usize {
        self.edit_distance
    }
    /// Logical work limits for indexed substring probes.
    pub const fn substring_limits(&self) -> crate::surface::SubstringLimits {
        self.substring_limits
    }
    /// Select explicit complete-or-refuse substring work limits.
    #[must_use]
    pub const fn with_substring_limits(mut self, limits: crate::surface::SubstringLimits) -> Self {
        self.substring_limits = limits;
        self
    }
    /// Identity of all parameters, ordered laws and pinned semantic data.
    pub fn fingerprint(&self) -> [u8; 32] {
        let mut bytes = Vec::new();
        for law in [
            PROFILE_DOMAIN.as_bytes(),
            crate::ANALYZER_PROFILE_ID.as_bytes(),
            ORDERED_LAW.as_bytes(),
            LEXICAL_LAW.as_bytes(),
            BOUNDING_LAW.as_bytes(),
            crate::phonetic::PROFILE_ID.as_bytes(),
            crate::segment::PROFILE_ID.as_bytes(),
        ] {
            frame_le(&mut bytes, law);
        }
        frame_le(&mut bytes, &crate::unicode::TEXT_DATA_DIGEST);
        bytes.extend_from_slice(&(self.max_token_scalars as u64).to_le_bytes());
        bytes.extend_from_slice(&[
            self.accent.fingerprint_code(),
            self.stemming as u8,
            self.input_mode as u8,
        ]);
        if self.stemming == Stemming::English {
            frame_le(&mut bytes, crate::stem::PROFILE_ID.as_bytes());
        }
        match &self.segmentation {
            Segmentation::Baseline => {
                frame_le(&mut bytes, BASELINE_LAW.as_bytes());
                for id in baseline_ids() {
                    frame_le(&mut bytes, id.as_bytes());
                }
            }
            Segmentation::EmptyLexicon => frame_le(&mut bytes, b"explicit-empty-lexicon"),
            Segmentation::Dictionary(dictionary) => frame_le(&mut bytes, &dictionary.fingerprint()),
        }
        if self.input_mode != InputMode::Plain {
            for id in [
                purrdf_lex::html::REFERENCE_LAW,
                purrdf_lex::html::SPEC_SNAPSHOT,
            ] {
                frame_le(&mut bytes, id.as_bytes());
            }
            frame_le(&mut bytes, &purrdf_lex::html::NAMED_TABLE_BLAKE3);
            frame_le(&mut bytes, &purrdf_lex::html::NUMERIC_TABLE_BLAKE3);
        }
        bytes.extend_from_slice(&(self.code_length as u64).to_le_bytes());
        bytes.extend_from_slice(&(self.edit_distance as u64).to_le_bytes());
        for value in [
            self.substring_limits.posting_operations,
            self.substring_limits.candidate_spans,
            self.substring_limits.verification_bytes,
        ] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        frame_le(&mut bytes, SUBSTRING_LAW.as_bytes());
        *blake3::hash(&bytes).as_bytes()
    }
}
pub(crate) const fn baseline_ids() -> [&'static str; 5] {
    use crate::segment::baseline;
    [
        baseline::CJDICT_PHYSICAL,
        baseline::THAIDICT_PHYSICAL,
        baseline::LAODICT_PHYSICAL,
        baseline::KHMERDICT_PHYSICAL,
        baseline::BURMESEDICT_PHYSICAL,
    ]
}
