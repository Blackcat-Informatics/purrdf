// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Immutable dictionaries and integer weighted lattice segmentation.
//!
//! The path minimizes unknown orthographic units, total dictionary cost, emitted
//! count, then prefers the longest earliest edge. Unknown unspaced-script units
//! are extended grapheme clusters; other scripts retain Unicode word boundaries.
//! Word-internal controls are transparent to lookup, while returned tokens borrow
//! their full original coverage. Lexical letter-period/colon-letter splits cannot
//! be crossed. The analyzer isolates protected emoji before calling this module.
//!
//! Keys occupy one UTF-8 arena. Semantic entry/cost identity is independent of
//! lookup storage. Artifact identity hashes canonical CBOR bytes; loading and
//! analysis perform no filesystem or network access.

use crate::TextError;
use crate::unicode::{self, SegmentationScript};
use purrdf_hash::Domain;
use purrdf_hash::blake3::RecordHasher;
use purrdf_hash::frame::frame_le_into;

mod artifact;
/// Content identities only; no baseline word data is embedded.
pub mod baseline;
mod storage;
use storage::Storage;
pub use storage::{DictionaryRepresentation, DictionaryStorageStats};

/// Normalization, lattice ordering and artifact edition.
pub const PROFILE_ID: &str =
    "unicode17-caseless-transparent-egc-lattice-unknown-cost-count-longest/v2";
/// Maximum admitted source or canonical vocabulary entries.
pub const MAX_ENTRIES: usize = 1_000_000;
/// Maximum UTF-8 bytes in the word arena.
pub const MAX_ARENA_BYTES: usize = 16 * 1024 * 1024;
/// Maximum scalars in one dictionary key.
pub const MAX_WORD_SCALARS: usize = 1024;
/// Maximum artifact bytes accepted before decoding.
pub const MAX_ARTIFACT_BYTES: usize = 32 * 1024 * 1024;
const DICTIONARY_DOMAIN: Domain = Domain::new(b"purrdf-text/dictionary-lattice/v2");

/// Flat immutable vocabulary with compact lookup storage.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Dictionary {
    arena: String,
    offsets: Vec<u32>,
    costs: Vec<u32>,
    storage: Storage,
    fingerprint: [u8; 32],
}

impl Dictionary {
    /// Construct a vocabulary with unit costs.
    ///
    /// # Errors
    /// Invalid entries, conflicting canonical costs, or resource bounds.
    pub fn new(entries: impl IntoIterator<Item = String>) -> Result<Self, TextError> {
        Self::with_costs(entries.into_iter().map(|word| (word, 1)))
    }

    /// Construct from nonnegative integer costs, including zero. Equal canonical
    /// entries collapse; conflicting costs fail independently of insertion order.
    ///
    /// # Errors
    /// Invalid entries, conflicting canonical costs, or resource bounds.
    pub fn with_costs(entries: impl IntoIterator<Item = (String, u32)>) -> Result<Self, TextError> {
        let mut canonical = Vec::new();
        let mut size = 0usize;
        for (input, cost) in entries {
            if canonical.len() >= MAX_ENTRIES {
                return Err(invalid("dictionary entry count exceeds its bound"));
            }
            let word = Self::canonical_key(&input)?;
            size = size
                .checked_add(word.len())
                .ok_or_else(|| invalid("dictionary size overflow"))?;
            if size > MAX_ARENA_BYTES {
                return Err(invalid("dictionary arena exceeds its byte bound"));
            }
            canonical.push((word, cost));
        }
        canonical.sort_unstable();
        canonical.dedup();
        if canonical.windows(2).any(|pair| pair[0].0 == pair[1].0) {
            return Err(invalid("canonical dictionary key has conflicting costs"));
        }
        let mut arena = String::with_capacity(size);
        let mut offsets = Vec::with_capacity(canonical.len() + 1);
        let mut costs = Vec::with_capacity(canonical.len());
        offsets.push(0);
        for (word, cost) in canonical {
            arena.push_str(&word);
            offsets.push(u32::try_from(arena.len()).expect("bounded arena"));
            costs.push(cost);
        }
        arena.shrink_to_fit();
        Ok(Self::assemble(arena, offsets, costs))
    }

    /// Canonicalize a Unicode entry without HTML decoding. Word-internal controls
    /// disappear from keys, not from returned token coverage. Script punctuation
    /// and iteration marks are legal; separators, protected emoji and other
    /// controls are refused. Emoji are independent analyzer atoms.
    ///
    /// # Errors
    /// Nonlexical/empty entries, forbidden controls, separators or length bounds.
    pub fn canonical_key(input: &str) -> Result<String, TextError> {
        if input.len() > MAX_WORD_SCALARS * 4 {
            return Err(invalid("dictionary input exceeds its byte bound"));
        }
        if unicode::grapheme_bounds(input).any(|(_, text)| unicode::is_emoji_grapheme(text)) {
            return Err(invalid("dictionary entries cannot contain protected emoji"));
        }
        let mut word = String::with_capacity(input.len());
        unicode::analysis_form(input, &mut word);
        word.retain(|c| !unicode::is_word_internal_control(c));
        if !lexical(&word)
            || word.chars().count() > MAX_WORD_SCALARS
            || word
                .chars()
                .any(|c| unicode::is_lexical_separator(c) || unicode::is_default_ignorable(c))
        {
            return Err(invalid(
                "dictionary key must be lexical, bounded and contain no separators or non-internal controls",
            ));
        }
        Ok(word)
    }

    /// Explicit empty-vocabulary orthographic fallback.
    #[must_use]
    pub fn empty() -> Self {
        Self::assemble(String::new(), vec![0], Vec::new())
    }
    /// Distinct canonical entry count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.costs.len()
    }
    /// Whether the vocabulary is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.costs.is_empty()
    }
    /// Canonical words in UTF-8 lexical order, borrowed from one arena.
    pub fn words(&self) -> impl ExactSizeIterator<Item = &str> {
        self.offsets
            .windows(2)
            .map(|r| &self.arena[r[0] as usize..r[1] as usize])
    }
    /// Canonical keys and costs in lexical order.
    pub fn weighted_entries(&self) -> impl ExactSizeIterator<Item = (&str, u32)> {
        self.words().zip(self.costs.iter().copied())
    }
    /// Semantic identity, independent of physical lookup representation.
    #[must_use]
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    /// Change lookup representation without changing semantics or artifact bytes.
    #[must_use]
    pub fn with_representation(mut self, representation: DictionaryRepresentation) -> Self {
        if self.storage.representation() != representation {
            self.storage = Storage::build(&self.arena, &self.offsets, representation);
        }
        self
    }
    /// Storage counts and retained allocation bytes, excluding struct headers.
    #[must_use]
    pub fn storage_stats(&self) -> DictionaryStorageStats {
        self.storage
            .stats(self.arena.capacity() + self.offsets.capacity() * 4 + self.costs.capacity() * 4)
    }
    /// Look up an already canonical key, without additional normalization.
    #[must_use]
    pub fn cost(&self, canonical: &str) -> Option<u32> {
        self.storage
            .exact(canonical.as_bytes(), &self.arena)
            .map(|rank| self.costs[rank])
    }
    /// Segment normalized input, borrowing complete token spellings.
    pub fn segment_each<'a>(&self, normalized: &'a str, sink: impl FnMut(&'a str)) {
        self.segment_each_with_scratch(normalized, &mut SegmentationScratch::default(), sink);
    }
    /// Segment through reusable heap storage. The analyzer isolates protected emoji
    /// before this call; every dictionary endpoint still respects EGC boundaries.
    pub fn segment_each_with_scratch<'a>(
        &self,
        normalized: &'a str,
        scratch: &mut SegmentationScratch,
        sink: impl FnMut(&'a str),
    ) {
        self.segment_with_tailoring(normalized, scratch, true, sink);
    }

    /// Segment pre-stem surface words without lexical period/colon tailoring.
    /// Dictionary and fallback endpoints remain grapheme-safe, and transparent
    /// controls retain their original coverage as in lexical segmentation.
    pub fn segment_surface_each_with_scratch<'a>(
        &self,
        normalized: &'a str,
        scratch: &mut SegmentationScratch,
        sink: impl FnMut(&'a str),
    ) {
        self.segment_with_tailoring(normalized, scratch, false, sink);
    }

    fn segment_with_tailoring<'a>(
        &self,
        normalized: &'a str,
        scratch: &mut SegmentationScratch,
        tailor: bool,
        mut sink: impl FnMut(&'a str),
    ) {
        for chunk in normalized
            .split(|c| unicode::is_lexical_separator(c) && !unicode::is_word_internal_control(c))
        {
            if !chunk.is_empty() {
                self.segment_chunk(chunk, scratch, tailor, &mut sink);
            }
        }
    }
    /// Collect borrowed segments.
    #[must_use]
    pub fn segments<'a>(&self, normalized: &'a str) -> Vec<&'a str> {
        let mut words = Vec::new();
        self.segment_each(normalized, |word| words.push(word));
        words
    }
    fn assemble(arena: String, offsets: Vec<u32>, costs: Vec<u32>) -> Self {
        let mut digest = RecordHasher::new();
        frame_le_into(&mut digest, DICTIONARY_DOMAIN.as_bytes());
        frame_le_into(&mut digest, PROFILE_ID.as_bytes());
        frame_le_into(&mut digest, &<[u8; 3]>::from(unicode::UNICODE_VERSION));
        digest.update(&(costs.len() as u64).to_le_bytes());
        for (range, cost) in offsets.windows(2).zip(&costs) {
            frame_le_into(
                &mut digest,
                &arena.as_bytes()[range[0] as usize..range[1] as usize],
            );
            digest.update(&cost.to_le_bytes());
        }
        let storage = Storage::build(&arena, &offsets, DictionaryRepresentation::Radix);
        Self {
            arena,
            offsets,
            costs,
            storage,
            fingerprint: *digest.finalize().as_bytes(),
        }
    }
    fn segment_chunk<'a>(
        &self,
        chunk: &'a str,
        scratch: &mut SegmentationScratch,
        tailor: bool,
        sink: &mut impl FnMut(&'a str),
    ) {
        scratch.prepare(chunk, tailor);
        let units = scratch.boundaries.len() - 1;
        for at in (0..units).rev() {
            let fallback = scratch.fallbacks[at];
            let mut best = Path::prepend(
                scratch.paths[fallback.next],
                fallback.next,
                fallback.emit,
                0,
                fallback.emit,
            );
            let start = scratch.clean_offsets[at];
            let limit = scratch.barriers[at];
            self.storage.prefixes(
                &scratch.clean.as_bytes()[start..limit],
                &self.arena,
                |end, rank| {
                    let target = start + end;
                    let next = scratch
                        .clean_offsets
                        .partition_point(|&offset| offset <= target)
                        .saturating_sub(1);
                    if next > at && scratch.clean_offsets[next] == target {
                        let candidate =
                            Path::prepend(scratch.paths[next], next, false, self.costs[rank], true);
                        if candidate.key() < best.key() {
                            best = candidate;
                        }
                    }
                },
            );
            scratch.paths[at] = best;
        }
        let mut at = 0;
        while at < units {
            let path = scratch.paths[at];
            if path.emit {
                sink(&chunk[scratch.boundaries[at]..scratch.boundaries[path.next]]);
            }
            at = path.next;
        }
    }
}
purrdf_hash::default_from_new!(Dictionary => empty);

/// Reusable lattice workspace; no input references or per-token allocations.
#[derive(Debug, Default)]
pub struct SegmentationScratch {
    boundaries: Vec<usize>,
    clean_offsets: Vec<usize>,
    clean: String,
    fallbacks: Vec<Fallback>,
    barriers: Vec<usize>,
    paths: Vec<Path>,
}
#[derive(Clone, Copy, Debug, Default)]
struct Fallback {
    next: usize,
    emit: bool,
}
#[derive(Clone, Copy, Debug, Default)]
struct Path {
    unknown: usize,
    cost: u128,
    tokens: usize,
    next: usize,
    emit: bool,
}
impl Path {
    const fn prepend(tail: Self, next: usize, unknown: bool, cost: u32, emit: bool) -> Self {
        Self {
            unknown: tail.unknown + unknown as usize,
            cost: tail.cost + cost as u128,
            tokens: tail.tokens + emit as usize,
            next,
            emit,
        }
    }
    const fn key(self) -> (usize, u128, usize, core::cmp::Reverse<usize>) {
        (
            self.unknown,
            self.cost,
            self.tokens,
            core::cmp::Reverse(self.next),
        )
    }
}
impl SegmentationScratch {
    fn prepare(&mut self, chunk: &str, tailor: bool) {
        self.boundaries.clear();
        self.clean_offsets.clear();
        self.clean.clear();
        for (offset, grapheme) in unicode::grapheme_bounds(chunk) {
            self.boundaries.push(offset);
            self.clean_offsets.push(self.clean.len());
            self.clean.extend(
                grapheme
                    .chars()
                    .filter(|&c| !unicode::is_word_internal_control(c)),
            );
        }
        self.boundaries.push(chunk.len());
        self.clean_offsets.push(self.clean.len());
        let units = self.boundaries.len() - 1;
        self.fallbacks.clear();
        self.barriers.clear();
        self.paths.clear();
        self.paths.resize(units + 1, Path::default());
        self.fallbacks.extend((0..units).map(|at| Fallback {
            next: at + 1,
            emit: lexical(&self.clean[self.clean_offsets[at]..self.clean_offsets[at + 1]]),
        }));
        self.barriers.resize(units, self.clean.len());
        let mut limit = self.clean.len();
        for at in (0..units).rev() {
            let piece = &self.clean[self.clean_offsets[at]..self.clean_offsets[at + 1]];
            if tailor
                && piece.chars().next().is_some_and(|c| matches!(c, '.' | ':'))
                && neighbor_letter(&self.clean[..self.clean_offsets[at]], false)
                && neighbor_letter(&self.clean[self.clean_offsets[at + 1]..], true)
            {
                limit = self.clean_offsets[at];
            }
            self.barriers[at] = limit.max(self.clean_offsets[at]);
        }
        let mut at = 0;
        while at < units {
            if unspaced(&self.clean[self.clean_offsets[at]..self.clean_offsets[at + 1]]) {
                at += 1;
                continue;
            }
            let start = at;
            at += 1;
            while at < units
                && !unspaced(&self.clean[self.clean_offsets[at]..self.clean_offsets[at + 1]])
            {
                at += 1;
            }
            let offset = self.clean_offsets[start];
            for (word_at, word) in
                unicode::word_indices(&self.clean[offset..self.clean_offsets[at]])
            {
                let begin = offset + word_at;
                let end = begin + word.len();
                let from = self.clean_offsets.partition_point(|&n| n < begin);
                let through = self
                    .clean_offsets
                    .partition_point(|&n| n <= end)
                    .saturating_sub(1);
                if self.clean_offsets.get(from) != Some(&begin)
                    || self.clean_offsets[through] != end
                {
                    continue;
                }
                for unit in from..through {
                    let cap = self.barriers[unit].min(end);
                    let next = self
                        .clean_offsets
                        .partition_point(|&n| n <= cap)
                        .saturating_sub(1);
                    if next > unit
                        && lexical(&self.clean[self.clean_offsets[unit]..self.clean_offsets[next]])
                    {
                        self.fallbacks[unit] = Fallback { next, emit: true };
                    }
                }
            }
        }
    }
}
fn lexical(text: &str) -> bool {
    text.chars()
        .any(|c| unicode::is_alphanumeric(c) || unicode::segmentation_script(c).is_some())
}
fn unspaced(text: &str) -> bool {
    text.chars().any(|c| {
        unicode::segmentation_script(c).is_some()
            || unicode::has_segmentation_script(c, SegmentationScript::Hiragana)
            || unicode::has_segmentation_script(c, SegmentationScript::Katakana)
    })
}
fn neighbor_letter(text: &str, forward: bool) -> bool {
    let significant =
        |c: &char| !unicode::is_combining_mark(*c) && !unicode::is_word_internal_control(*c);
    if forward {
        text.chars().find(significant)
    } else {
        text.chars().rev().find(significant)
    }
    .is_some_and(unicode::is_letter)
}
fn invalid(message: &str) -> TextError {
    TextError::config(message)
}
