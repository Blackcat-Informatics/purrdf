// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One fallible, source-aligned analysis law for documents and queries.
use crate::segment::{Dictionary, SegmentationScratch};
use crate::{AccentFold, AnalyzerProfile, InputMode, Segmentation, Stemming, TextError, unicode};
use purrdf_hash::{Domain, frame::frame_le};
const ANALYZER_DOMAIN: Domain = Domain::new(b"purrdf-text/resolved-analyzer/v4\0");
use purrdf_lex::unicode::{TaggedScalar, compose_tagged, decompose_tagged};
use std::{borrow::Cow, collections::BTreeMap, fmt, ops::Range, sync::Arc};

/// A lexical token and its consecutive ordinal.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Token<'a> {
    /// Final, possibly stemmed and bounded spelling.
    pub text: Cow<'a, str>,
    /// Lexical position; auxiliary projections have independent positions.
    pub position: u32,
}
/// Unicode table version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnicodeVersion {
    /// Major number.
    pub major: u64,
    /// Minor number.
    pub minor: u64,
    /// Patch number.
    pub patch: u64,
}
impl From<(u8, u8, u8)> for UnicodeVersion {
    fn from((major, minor, patch): (u8, u8, u8)) -> Self {
        Self {
            major: u64::from(major),
            minor: u64::from(minor),
            patch: u64::from(patch),
        }
    }
}
impl From<(u64, u64, u64)> for UnicodeVersion {
    fn from((major, minor, patch): (u64, u64, u64)) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}
impl fmt::Display for UnicodeVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
/// Pinned versions of each Unicode stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UnicodeVersions {
    /// Character properties.
    pub core: UnicodeVersion,
    /// Normalization.
    pub normalization: UnicodeVersion,
    /// Full case folding.
    pub case_folding: UnicodeVersion,
    /// Word and grapheme boundaries.
    pub segmentation: UnicodeVersion,
}
/// Every table is generated from the same versioned Unicode data.
pub fn unicode_versions() -> UnicodeVersions {
    let version = unicode::UNICODE_VERSION.into();
    UnicodeVersions {
        core: version,
        normalization: version,
        case_folding: version,
        segmentation: version,
    }
}
/// Normalized bytes and their actual original UTF-8 contributors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlignedText {
    /// Normalized projection buffer, including preserved emoji atoms.
    pub text: String,
    scalar_offsets: Vec<usize>,
    scalar_sources: Vec<usize>,
    records: Vec<Range<usize>>,
    sources: Vec<Range<usize>>,
}
impl AlignedText {
    fn identity(text: String) -> Self {
        Self {
            text,
            scalar_offsets: Vec::new(),
            scalar_sources: Vec::new(),
            records: Vec::new(),
            sources: Vec::new(),
        }
    }
    /// An unchanged normalized projection with exact contributor evidence.
    pub fn projection(&self, range: Range<usize>) -> Projection {
        project(self, self.text[range.clone()].to_owned(), range, false)
    }
    /// Sorted merged original contributors to a normalized byte range.
    pub fn contributors(&self, range: Range<usize>) -> Vec<Range<usize>> {
        if self.scalar_sources.is_empty() {
            return if range.is_empty() {
                Vec::new()
            } else {
                vec![range]
            };
        }
        let start = self.scalar_offsets.partition_point(|&at| at < range.start);
        let end = self.scalar_offsets.partition_point(|&at| at < range.end);
        let mut sources = Vec::new();
        for &record in &self.scalar_sources[start..end] {
            sources.extend_from_slice(&self.sources[self.records[record].clone()]);
        }
        merge_ranges(&mut sources);
        sources
    }
}
/// One independently bounded projection with inspectable source evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Projection {
    /// Final spelling.
    pub text: String,
    /// Originating normalized byte range; stems may differ from these bytes.
    pub range: Range<usize>,
    /// Original UTF-8 contributor ranges, distinct from projected offsets.
    pub sources: Vec<Range<usize>>,
    /// Enclosing original highlight; may include removed bytes between contributors.
    pub highlight: Range<usize>,
    /// True when a stem carries whole-word evidence rather than character alignment.
    pub coarse: bool,
}
/// All views produced atomically by one analysis pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Analysis {
    /// Shared normalized source-aligned text.
    pub normalized: AlignedText,
    /// Lexical terms, after optional stemming and final bounding.
    pub lexical: Vec<Projection>,
    /// Pre-stem words with word-internal controls retained.
    pub surface: Vec<Projection>,
    /// Punctuation-bearing whitespace spans.
    pub spans: Vec<Projection>,
}
/// Reusable lexical normalization, segmentation and word storage.
#[derive(Debug, Default)]
pub struct AnalyzerScratch {
    normalization: NormalizationScratch<()>,
    ranges: Vec<Range<usize>>,
    segmentation: SegmentationScratch,
    word: String,
}
/// Immutable resolved analysis law. No filesystem, network or hidden data access.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Analyzer {
    profile: AnalyzerProfile,
    dictionary: Option<Arc<Dictionary>>,
}
impl Analyzer {
    /// Explicit no-artifact grapheme fallback with the standard linguistic choices.
    pub const fn empty_lexicon() -> Self {
        Self {
            profile: AnalyzerProfile::empty_lexicon(),
            dictionary: None,
        }
    }
    /// Resolve an explicit empty or caller-dictionary profile.
    /// # Errors
    /// Refuses a baseline profile without its five artifact byte strings.
    pub fn with_profile(profile: AnalyzerProfile) -> Result<Self, TextError> {
        Self::resolve(profile, &[])
    }
    /// Resolve baseline physical identities and bind effective profile-normalized costs.
    /// # Errors
    /// Refuses missing, duplicate, unexpected or corrupt artifacts and conflicting caller costs.
    pub fn resolve(profile: AnalyzerProfile, artifacts: &[&[u8]]) -> Result<Self, TextError> {
        let mut normalization = NormalizationScratch::default();
        let dictionary = match profile.segmentation() {
            Segmentation::EmptyLexicon => {
                if !artifacts.is_empty() {
                    return Err(TextError::config(
                        "empty lexicon profile refuses artifact bytes",
                    ));
                }
                None
            }
            Segmentation::Dictionary(dictionary) => {
                if !artifacts.is_empty() {
                    return Err(TextError::config(
                        "caller dictionary profile refuses baseline artifacts",
                    ));
                }
                let entries = dictionary
                    .weighted_entries()
                    .map(|(word, cost)| {
                        dictionary_key(word, &profile, &mut normalization)
                            .map(|word| (word.to_owned(), cost))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Some(Arc::new(Dictionary::with_costs(entries)?))
            }
            Segmentation::Baseline => {
                let expected = crate::profile::baseline_ids();
                let mut seen = [false; 5];
                let mut entries: BTreeMap<String, u32> = BTreeMap::new();
                for bytes in artifacts {
                    let hash = *purrdf_hash::blake3::hash(bytes).as_bytes();
                    let identity = purrdf_hash::hex::encode(&hash);
                    let Some(at) = expected.iter().position(|&id| id == identity) else {
                        return Err(TextError::config("unexpected baseline artifact identity"));
                    };
                    if std::mem::replace(&mut seen[at], true) {
                        return Err(TextError::config("duplicate baseline artifact"));
                    }
                    let dictionary = Dictionary::from_artifact(hash, bytes)?;
                    for (word, cost) in dictionary.weighted_entries() {
                        let word = dictionary_key(word, &profile, &mut normalization)?;
                        if let Some(prior) = entries.get_mut(word) {
                            *prior = (*prior).min(cost);
                        } else {
                            entries.insert(word.to_owned(), cost);
                        }
                    }
                }
                if !seen.into_iter().all(|present| present) {
                    return Err(TextError::config(
                        "standard analysis requires all five full baseline artifacts",
                    ));
                }
                Some(Arc::new(Dictionary::with_costs(entries)?))
            }
        };
        Ok(Self {
            profile,
            dictionary,
        })
    }
    /// Validated profile.
    pub const fn profile(&self) -> &AnalyzerProfile {
        &self.profile
    }
    /// Identity of semantic data, resolved costed keys and all ordered choices.
    pub fn fingerprint(&self) -> [u8; 32] {
        let mut bytes = Vec::new();
        frame_le(&mut bytes, ANALYZER_DOMAIN.as_bytes());
        frame_le(&mut bytes, &self.profile.fingerprint());
        if let Some(dictionary) = &self.dictionary {
            frame_le(&mut bytes, &dictionary.fingerprint());
        }
        *purrdf_hash::blake3::hash(&bytes).as_bytes()
    }
    /// Analyze atomically; no partial output survives a refusal.
    /// # Errors
    /// Propagates strict reference errors and position-space exhaustion.
    pub fn analyze<'a>(&self, input: &'a str, out: &mut Vec<Token<'a>>) -> Result<(), TextError> {
        out.clear();
        let analysis = self.projections(input)?;
        for (position, term) in analysis.lexical.into_iter().enumerate() {
            out.push(Token {
                text: if !term.coarse
                    && input.get(term.highlight.clone()) == Some(term.text.as_str())
                {
                    Cow::Borrowed(&input[term.highlight])
                } else {
                    Cow::Owned(term.text)
                },
                position: u32::try_from(position)
                    .map_err(|_| TextError::data("token positions exceed u32"))?,
            });
        }
        Ok(())
    }
    /// Collect lexical terms through the same law used by indexing.
    /// # Errors
    /// Propagates analysis refusal.
    pub fn terms(&self, input: &str) -> Result<Vec<String>, TextError> {
        let mut terms = Vec::new();
        self.analyze_each(input, &mut String::new(), |token| {
            terms.push(token.text.into_owned());
        })?;
        Ok(terms)
    }
    /// Consume lexical tokens after a complete successful analysis.
    /// # Errors
    /// Propagates analysis refusal before invoking the sink.
    pub fn analyze_each(
        &self,
        input: &str,
        scratch: &mut String,
        mut sink: impl FnMut(Token<'_>),
    ) -> Result<(), TextError> {
        let mut buffers = AnalyzerScratch::default();
        std::mem::swap(scratch, &mut buffers.normalization.output);
        let result = self.analyze_into(input, &mut buffers, &mut sink);
        std::mem::swap(scratch, &mut buffers.normalization.output);
        result
    }
    /// Stream lexical terms using reusable normalization, lattice and word storage.
    /// # Errors
    /// Strict input errors and position exhaustion are refused before the sink runs.
    pub fn analyze_each_with_scratch(
        &self,
        input: &str,
        scratch: &mut AnalyzerScratch,
        mut sink: impl FnMut(Token<'_>),
    ) -> Result<(), TextError> {
        self.analyze_into(input, scratch, &mut sink)
    }
    fn analyze_into(
        &self,
        input: &str,
        scratch: &mut AnalyzerScratch,
        sink: &mut impl FnMut(Token<'_>),
    ) -> Result<(), TextError> {
        normalize_into(
            input,
            self.profile.input_mode(),
            self.profile.accent_fold(),
            &mut scratch.normalization,
            &mut Unaligned,
        )?;
        let normalized = &scratch.normalization.output;
        self.fill_word_ranges(
            normalized,
            true,
            &mut scratch.ranges,
            &mut scratch.segmentation,
        );
        if scratch.ranges.len() > u32::MAX as usize {
            return Err(TextError::data("token positions exceed u32"));
        }
        for (position, range) in scratch.ranges.iter().cloned().enumerate() {
            let word = &normalized[range];
            let emoji = unicode::is_emoji_grapheme(word);
            let changed = self.profile.stemming() == Stemming::English && !emoji
                || !word.is_ascii()
                    && unicode::emoji_scalars(word).any(|(_, c, protected)| {
                        !protected && unicode::is_word_internal_control(c)
                    });
            let text = if changed {
                scratch.word.clear();
                scratch.word.extend(
                    unicode::emoji_scalars(word)
                        .filter(|&(_, c, protected)| {
                            protected || !unicode::is_word_internal_control(c)
                        })
                        .map(|(_, c, _)| c),
                );
                if self.profile.stemming() == Stemming::English && !emoji {
                    crate::stem::english_in_place(&mut scratch.word);
                }
                self.bounded(&scratch.word)
            } else {
                self.bounded(word)
            };
            sink(Token {
                text: Cow::Borrowed(text),
                position: position as u32,
            });
        }
        Ok(())
    }

    /// Folded normalized text, preserving meaningful controls and emoji.
    /// # Errors
    /// Propagates strict HTML reference errors.
    pub fn analysis_form(&self, input: &str) -> Result<String, TextError> {
        let mut scratch = NormalizationScratch::default();
        normalize_into(
            input,
            self.profile.input_mode(),
            self.profile.accent_fold(),
            &mut scratch,
            &mut Unaligned,
        )?;
        Ok(scratch.output)
    }
    /// Pre-stem bounded surface words.
    /// # Errors
    /// Propagates analysis refusal.
    pub fn surface_terms(&self, input: &str) -> Result<Vec<String>, TextError> {
        Ok(self
            .projections(input)?
            .surface
            .into_iter()
            .map(|term| term.text)
            .collect())
    }
    /// Bounded punctuation-bearing spans.
    /// # Errors
    /// Propagates analysis refusal.
    pub fn substring_terms(&self, input: &str) -> Result<Vec<String>, TextError> {
        Ok(self
            .projections(input)?
            .spans
            .into_iter()
            .map(|term| term.text)
            .collect())
    }
    /// Produce all independent views from one source-aligned normalization.
    /// # Errors
    /// Propagates strict reference errors and position-space exhaustion.
    pub fn projections(&self, input: &str) -> Result<Analysis, TextError> {
        let normalized = self.normalize(input)?;
        let mut surface_ranges = Vec::new();
        let mut lexical_ranges = Vec::new();
        let mut segmentation = SegmentationScratch::default();
        self.fill_word_ranges(
            &normalized.text,
            false,
            &mut surface_ranges,
            &mut segmentation,
        );
        self.fill_word_ranges(
            &normalized.text,
            true,
            &mut lexical_ranges,
            &mut segmentation,
        );
        let mut surface = Vec::new();
        let mut lexical = Vec::new();
        let mut spans = Vec::new();
        for range in surface_ranges {
            let origin = range;
            let text = self.bounded(&normalized.text[origin.clone()]);
            let range = origin.start..origin.start + text.len();
            surface.push(project(&normalized, text.to_owned(), range, false));
        }
        for range in lexical_ranges {
            let original = &normalized.text[range.clone()];
            let emoji = unicode::is_emoji_grapheme(original);
            let mut text = if original.is_ascii() {
                original.to_owned()
            } else {
                unicode::emoji_scalars(original)
                    .filter(|&(_, c, protected)| protected || !unicode::is_word_internal_control(c))
                    .map(|(_, c, _)| c)
                    .collect::<String>()
            };
            let coarse = if self.profile.stemming() == Stemming::English && !emoji {
                let before = text.clone();
                crate::stem::english_in_place(&mut text);
                text != before
            } else {
                false
            };
            text.truncate(self.bounded(&text).len());
            let mut projection = project(&normalized, text, range.clone(), coarse);
            if !coarse {
                projection.sources.clear();
                let mut bytes = 0;
                let mut end = range.start;
                for (at, c, protected) in unicode::emoji_scalars(original) {
                    if !protected && unicode::is_word_internal_control(c) {
                        continue;
                    }
                    if bytes == projection.text.len() {
                        break;
                    }
                    let origin = range.start + at..range.start + at + c.len_utf8();
                    end = origin.end;
                    bytes += c.len_utf8();
                    projection.sources.extend(normalized.contributors(origin));
                }
                projection.range.end = end;
                merge_ranges(&mut projection.sources);
                projection.highlight = projection.sources.first().map_or(0, |range| range.start)
                    ..projection.sources.last().map_or(0, |range| range.end);
            }
            lexical.push(projection);
        }
        let mut start = None;
        for (at, c) in normalized
            .text
            .char_indices()
            .chain(std::iter::once((normalized.text.len(), ' ')))
        {
            if unicode::is_whitespace_separator(c) {
                if let Some(begin) = start.take() {
                    let text = self.bounded(&normalized.text[begin..at]);
                    spans.push(project(
                        &normalized,
                        text.to_owned(),
                        begin..begin + text.len(),
                        false,
                    ));
                }
            } else {
                start.get_or_insert(at);
            }
        }
        if lexical.len() > u32::MAX as usize {
            return Err(TextError::data("token positions exceed u32"));
        }
        Ok(Analysis {
            normalized,
            lexical,
            surface,
            spans,
        })
    }
    pub(crate) fn bounded<'a>(&self, text: &'a str) -> &'a str {
        let mut total = 0;
        let mut end = 0;
        for (at, cluster) in unicode::grapheme_bounds(text) {
            let count = cluster.chars().count();
            if total + count > self.profile.max_token_scalars() {
                if at == 0 {
                    end = cluster.len();
                }
                break;
            }
            total += count;
            end = at + cluster.len();
        }
        &text[..end]
    }
    fn fill_word_ranges(
        &self,
        text: &str,
        split_punctuation: bool,
        ranges: &mut Vec<Range<usize>>,
        segmentation: &mut SegmentationScratch,
    ) {
        ranges.clear();
        let mut run_start = 0;
        for (at, cluster) in unicode::grapheme_bounds(text) {
            if unicode::is_emoji_grapheme(cluster) {
                self.segment_run(text, run_start..at, split_punctuation, ranges, segmentation);
                ranges.push(at..at + cluster.len());
                run_start = at + cluster.len();
            }
        }
        self.segment_run(
            text,
            run_start..text.len(),
            split_punctuation,
            ranges,
            segmentation,
        );
    }
    fn segment_run(
        &self,
        text: &str,
        range: Range<usize>,
        split_punctuation: bool,
        out: &mut Vec<Range<usize>>,
        segmentation: &mut SegmentationScratch,
    ) {
        let mut begin = range.start;
        for (relative, c) in text[range.clone()].char_indices() {
            let at = range.start + relative;
            if split_punctuation && matches!(c, '.' | ':') && letter_sides(text, at, c.len_utf8()) {
                self.segment_chunk(text, begin..at, split_punctuation, out, segmentation);
                begin = at + c.len_utf8();
            }
        }
        self.segment_chunk(text, begin..range.end, split_punctuation, out, segmentation);
    }
    fn segment_chunk(
        &self,
        text: &str,
        range: Range<usize>,
        tailor: bool,
        out: &mut Vec<Range<usize>>,
        segmentation: &mut SegmentationScratch,
    ) {
        let part = &text[range.clone()];
        if let Some(dictionary) = &self.dictionary {
            let mut sink = |word: &str| {
                let at = word.as_ptr() as usize - part.as_ptr() as usize + range.start;
                out.push(at..at + word.len());
            };
            if tailor {
                dictionary.segment_each_with_scratch(part, segmentation, &mut sink);
            } else {
                dictionary.segment_surface_each_with_scratch(part, segmentation, &mut sink);
            }
        } else {
            for (at, word) in unicode::word_indices(part) {
                if word.chars().any(unicode::is_alphanumeric) {
                    if word.chars().any(unspaced) {
                        for (offset, cluster) in unicode::grapheme_bounds(word) {
                            out.push(
                                range.start + at + offset
                                    ..range.start + at + offset + cluster.len(),
                            );
                        }
                    } else {
                        out.push(range.start + at..range.start + at + word.len());
                    }
                }
            }
        }
    }
    fn normalize(&self, input: &str) -> Result<AlignedText, TextError> {
        let mut scratch = NormalizationScratch::default();
        let mut store = SourceStore::default();
        normalize_into(
            input,
            self.profile.input_mode(),
            self.profile.accent_fold(),
            &mut scratch,
            &mut store,
        )?;
        let text = scratch.output;
        if text == input {
            return Ok(AlignedText::identity(text));
        }
        Ok(AlignedText {
            text,
            scalar_offsets: store.scalar_offsets,
            scalar_sources: store.scalar_sources,
            records: store.records,
            sources: store.sources,
        })
    }
}

/// One reusable pipeline, specialized only by the metadata it carries.
#[derive(Debug, Default)]
struct NormalizationScratch<M> {
    output: String,
    cleaned: String,
    scalars: Vec<TaggedScalar<M>>,
    pending: Vec<TaggedScalar<M>>,
    working: Vec<TaggedScalar<M>>,
}

/// Source tracking is orthogonal to every normalization decision. Streaming
/// instantiates the same pipeline with unit metadata and no source allocations.
trait Alignment {
    type Metadata: Copy;
    fn source(&mut self, source: Range<usize>) -> Self::Metadata;
    fn merge(&mut self, left: &mut Self::Metadata, right: Self::Metadata);
    fn reserve_output(&mut self, additional: usize);
    fn emit(&mut self, offset: usize, metadata: Self::Metadata);
}

struct Unaligned;
impl Alignment for Unaligned {
    type Metadata = ();
    fn source(&mut self, _: Range<usize>) {}
    fn merge(&mut self, (): &mut (), (): ()) {}
    fn reserve_output(&mut self, _: usize) {}
    fn emit(&mut self, _: usize, (): ()) {}
}

fn normalize_into<A: Alignment>(
    input: &str,
    mode: InputMode,
    accent: AccentFold,
    scratch: &mut NormalizationScratch<A::Metadata>,
    alignment: &mut A,
) -> Result<(), TextError> {
    admit_source_size(input.len(), "analysis input")?;
    scratch.output.clear();
    let mode = match mode {
        InputMode::Plain => purrdf_lex::html::Mode::Plain,
        InputMode::HtmlText => purrdf_lex::html::Mode::Text,
        InputMode::HtmlAttribute => purrdf_lex::html::Mode::Attribute,
    };
    let decoded = purrdf_lex::html::resolve_strict(input, mode).map_err(TextError::Html)?;
    admit_source_size(decoded.text.len(), "decoded input")?;
    let mut cleanup = CleanupCursor::new(&decoded.text);
    let no_cleanup = if decoded.text.is_ascii() {
        !decoded
            .text
            .char_indices()
            .any(|(at, c)| !cleanup.survives(at, c, false))
    } else {
        !unicode::grapheme_bounds(&decoded.text).any(|(at, cluster)| {
            unicode::emoji_scalars(cluster)
                .any(|(relative, c, protected)| !cleanup.survives(at + relative, c, protected))
        })
    };
    if decoded.sources.is_empty() && no_cleanup {
        // These paths preserve byte positions exactly, so neither projection
        // needs per-scalar source records. Both still apply the same admission.
        if decoded.text.is_ascii() {
            scratch.output.extend(
                decoded
                    .text
                    .bytes()
                    .map(|byte| char::from(byte.to_ascii_lowercase())),
            );
            return Ok(());
        }
        let scripts = accent.scripts();
        let accent_safe = scripts.is_empty()
            || !decoded.text.chars().any(|c| {
                unicode::is_nonspacing_mark(c)
                    || !c.is_ascii() && scripts.iter().any(|script| script.contains(c))
            });
        if accent_safe {
            let mut compare = unicode::Compare::new(&decoded.text);
            unicode::analysis_form(&decoded.text, &mut compare);
            if compare.finish() {
                scratch.output.push_str(&decoded.text);
                return Ok(());
            }
        }
    }
    scratch.scalars.clear();
    scratch.cleaned.clear();
    if no_cleanup {
        scratch.cleaned.reserve(decoded.text.len());
    }
    let mut ordinal = 0;
    let mut cleanup = CleanupCursor::new(&decoded.text);
    for (at, cluster) in unicode::grapheme_bounds(&decoded.text) {
        for (relative, c, protected) in unicode::emoji_scalars(cluster) {
            let offset = at + relative;
            let source = decoded
                .sources
                .get(ordinal)
                .cloned()
                .unwrap_or_else(|| offset..offset + c.len_utf8());
            ordinal += 1;
            if !cleanup.survives(offset, c, protected) {
                continue;
            }
            let metadata = alignment.source(source);
            scratch.scalars.push(TaggedScalar { value: c, metadata });
            scratch.cleaned.push(c);
        }
    }
    scratch.output.reserve(scratch.cleaned.len());
    // Re-segment after cleanup: deleted controls can reveal new emoji joins or
    // permit canonical composition across their former positions.
    scratch.pending.clear();
    let mut offset = 0;
    for (_, cluster) in unicode::grapheme_bounds(&scratch.cleaned) {
        let count = cluster.chars().count();
        let slice = &scratch.scalars[offset..offset + count];
        offset += count;
        if unicode::is_emoji_grapheme(cluster) {
            normalize_run(
                &mut scratch.pending,
                &mut scratch.working,
                accent,
                |left, right| alignment.merge(left, right),
            );
            emit_scalars(&mut scratch.output, scratch.pending.drain(..), alignment);
            emit_scalars(&mut scratch.output, slice.iter().cloned(), alignment);
        } else {
            scratch.pending.extend_from_slice(slice);
        }
    }
    normalize_run(
        &mut scratch.pending,
        &mut scratch.working,
        accent,
        |left, right| alignment.merge(left, right),
    );
    emit_scalars(&mut scratch.output, scratch.pending.drain(..), alignment);
    Ok(())
}

fn emit_scalars<A: Alignment>(
    output: &mut String,
    scalars: impl IntoIterator<Item = TaggedScalar<A::Metadata>>,
    alignment: &mut A,
) {
    let scalars = scalars.into_iter();
    alignment.reserve_output(scalars.size_hint().0);
    for scalar in scalars {
        alignment.emit(output.len(), scalar.metadata);
        output.push(scalar.value);
    }
}

#[derive(Default)]
struct SourceStore {
    scalar_offsets: Vec<usize>,
    scalar_sources: Vec<usize>,
    records: Vec<Range<usize>>,
    sources: Vec<Range<usize>>,
}
impl Alignment for SourceStore {
    type Metadata = usize;
    fn source(&mut self, source: Range<usize>) -> usize {
        let at = self.sources.len();
        self.sources.push(source);
        let id = self.records.len();
        self.records.push(at..at + 1);
        id
    }
    fn merge(&mut self, left: &mut usize, right: usize) {
        if *left == right {
            return;
        }
        let mut sources = self.sources[self.records[*left].clone()].to_vec();
        sources.extend_from_slice(&self.sources[self.records[right].clone()]);
        merge_ranges(&mut sources);
        let at = self.sources.len();
        self.sources.extend(sources);
        let id = self.records.len();
        self.records.push(at..self.sources.len());
        *left = id;
    }
    fn reserve_output(&mut self, additional: usize) {
        // Emoji can emit one-scalar blocks. Amortized growth keeps repeated
        // protected blocks linear instead of reallocating both arrays per atom.
        self.scalar_offsets.reserve(additional);
        self.scalar_sources.reserve(additional);
    }
    fn emit(&mut self, offset: usize, metadata: usize) {
        self.scalar_offsets.push(offset);
        self.scalar_sources.push(metadata);
    }
}

pub(crate) fn merge_ranges(ranges: &mut Vec<Range<usize>>) {
    ranges.sort_unstable_by_key(|range| (range.start, range.end));
    let mut count = 0;
    for at in 0..ranges.len() {
        if count != 0 && ranges[at].start <= ranges[count - 1].end {
            ranges[count - 1].end = ranges[count - 1].end.max(ranges[at].end);
        } else {
            ranges[count] = ranges[at].clone();
            count += 1;
        }
    }
    ranges.truncate(count);
}
fn normalize_run<M: Copy>(
    scalars: &mut Vec<TaggedScalar<M>>,
    scratch: &mut Vec<TaggedScalar<M>>,
    accent: AccentFold,
    merge: impl FnMut(&mut M, M),
) {
    decompose_tagged::<false, _>(scalars, scratch);
    fold_tagged(scalars, scratch);
    decompose_tagged::<true, _>(scalars, scratch);
    fold_tagged(scalars, scratch);
    decompose_tagged::<true, _>(scalars, scratch);
    let scripts = accent.scripts();
    if !scripts.is_empty() {
        let mut base = None;
        scalars.retain(|scalar| {
            let c = scalar.value;
            if unicode::is_nonspacing_mark(c) && !unicode::is_word_internal_control(c) {
                !base.is_some_and(|script: unicode::AccentScript| script.admits_mark(c))
            } else {
                if !unicode::is_combining_mark(c) && !unicode::is_word_internal_control(c) {
                    base = scripts.iter().find(|script| script.contains(c));
                }
                true
            }
        });
    }
    compose_tagged(scalars, scratch, merge);
}
fn fold_tagged<M: Copy>(scalars: &mut Vec<TaggedScalar<M>>, scratch: &mut Vec<TaggedScalar<M>>) {
    scratch.clear();
    for scalar in scalars.drain(..) {
        unicode::fold_scalar(scalar.value, |value| {
            scratch.push(TaggedScalar {
                value,
                metadata: scalar.metadata,
            });
        });
    }
    std::mem::swap(scalars, scratch);
}
fn admit_source_size(bytes: usize, kind: &str) -> Result<(), TextError> {
    if bytes > u32::MAX as usize {
        return Err(TextError::data(format!(
            "{kind} exceeds source record space"
        )));
    }
    Ok(())
}
/// Two monotonic scalar walks supply nearest significant neighbors. Each
/// scalar is visited at most once for context, even across long control runs.
struct CleanupCursor<'a> {
    remaining: std::str::CharIndices<'a>,
    left: Option<char>,
    right: Option<(usize, char)>,
}
impl<'a> CleanupCursor<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            remaining: text.char_indices(),
            left: None,
            right: None,
        }
    }
    fn survives(&mut self, at: usize, c: char, protected: bool) -> bool {
        protected
            || !(unicode::is_removed_control(c) || c == '\u{200d}' && !self.meaningful_joiner(at))
    }
    fn meaningful_joiner(&mut self, at: usize) -> bool {
        while self.right.is_none_or(|(offset, _)| offset < at) {
            if let Some((_, c)) = self.right.take() {
                self.left = Some(c);
            }
            self.right = self.remaining.find(|&(_, c)| {
                !unicode::is_combining_mark(c)
                    && !unicode::is_removed_control(c)
                    && !unicode::is_word_internal_control(c)
            });
            if self.right.is_none() {
                break;
            }
        }
        self.left
            .zip(self.right.map(|(_, c)| c))
            .is_some_and(|(left, right)| {
                unicode::is_joining_character(left) && unicode::is_joining_character(right)
                    || unicode::is_conjunct_consonant(left) && unicode::is_conjunct_consonant(right)
            })
    }
}
fn unspaced(c: char) -> bool {
    use unicode::SegmentationScript::{Han, Hiragana, Katakana, Khmer, Lao, Myanmar, Thai};
    [Han, Hiragana, Katakana, Thai, Lao, Khmer, Myanmar]
        .into_iter()
        .any(|script| unicode::has_segmentation_script(c, script))
}
fn letter_sides(text: &str, at: usize, len: usize) -> bool {
    text[..at]
        .chars()
        .rev()
        .find(|&c| !unicode::is_combining_mark(c) && !unicode::is_word_internal_control(c))
        .is_some_and(unicode::is_letter)
        && text[at + len..]
            .chars()
            .find(|&c| !unicode::is_combining_mark(c) && !unicode::is_word_internal_control(c))
            .is_some_and(unicode::is_letter)
}
fn project(
    normalized: &AlignedText,
    text: String,
    range: Range<usize>,
    coarse: bool,
) -> Projection {
    let sources = normalized.contributors(range.clone());
    let highlight =
        sources.first().map_or(0, |range| range.start)..sources.last().map_or(0, |range| range.end);
    Projection {
        text,
        range,
        sources,
        highlight,
        coarse,
    }
}
fn dictionary_key<'a>(
    word: &str,
    profile: &AnalyzerProfile,
    scratch: &'a mut NormalizationScratch<()>,
) -> Result<&'a str, TextError> {
    normalize_into(
        word,
        InputMode::Plain,
        profile.accent_fold(),
        scratch,
        &mut Unaligned,
    )?;
    scratch
        .output
        .retain(|c| !unicode::is_word_internal_control(c));
    Ok(&scratch.output)
}
