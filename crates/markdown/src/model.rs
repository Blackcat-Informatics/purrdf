// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The typed stand-off model: the document's own bytes, untouched, plus
//! a layer of typed annotations over ranges of them.
//!
//! This is what [`analyze`](crate::analyze) returns and what
//! [`render`](crate::render) projects. The order matters: the model is
//! the finding, and the claims are *one* projection of it. A consumer
//! that wants the graph renders; a consumer that wants to ask which
//! unit covers byte 4,821, or what a unit's exact quote and surrounding
//! context are, or which concordance rows lifted nothing here, asks the
//! model and never parses a triple to do it.
//!
//! Nothing here is Markdown. A [`Section`] is a span with a level, a
//! [`Unit`] is a span with a verse number that may be absent — the
//! words are the dialect's, the structure is not. See
//! [`crate::dialect`] for the seam.
//!
//! # The containment lattice
//!
//! Spans are ordered by containment: `document ⊃ section ⊃ unit ⊃
//! split piece`. It is a **partial** order, not a tree of everything —
//! two sibling sections contain neither the other, and two pieces of
//! one oversize unit deliberately *overlap* rather than nest, because
//! the continuation reaches back. [`Span::relation`] answers with the
//! full thirteen-way Allen interval relation, so a consumer never has
//! to reconstruct "contains" from four comparisons and get one of them
//! wrong at the endpoints.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::sync::Arc;

use purrdf_core::ContentDigest;
use purrdf_core::embedding::{ChunkingContractId, TargetId, TextChunkTarget};

use crate::dialect::Reading;
use crate::error::MarkdownError;
use crate::profile::Profile;
use crate::split::{floor_boundary, split_spans};

/// How many bytes of context a unit's [content anchor](Unit::anchor)
/// reaches on each side, snapped to a scalar boundary so the context is
/// always text.
///
/// The bound exists because the anchor is for **re-anchoring**, not for
/// reading: enough surrounding text to find the quote again in a
/// revised document, and not so much that the anchor becomes a copy of
/// the document. Thirty-two bytes is a line's worth of ordinary prose
/// and a handful of scalars of dense script; the snap can only make it
/// shorter, never longer.
pub const CONTEXT_BYTES: usize = 32;

/// A byte range of the source: `start` inclusive, `end` exclusive.
///
/// Every span in the model is a range of the document's **own** bytes,
/// as handed to [`analyze`](crate::analyze) — no trimming, no
/// normalization, nothing prepended — so `&source[span]` is always
/// exactly what the span annotates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    /// First byte of the range.
    pub start: u64,
    /// One past the last byte of the range.
    pub end: u64,
}

impl Span {
    /// A span from its two offsets.
    #[must_use]
    pub const fn new(start: u64, end: u64) -> Self {
        Self { start, end }
    }

    /// How many bytes the span covers.
    #[must_use]
    pub const fn len(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    /// Whether the span covers no byte at all.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.end <= self.start
    }

    /// Whether a byte offset falls inside the span.
    #[must_use]
    pub const fn contains(&self, byte: u64) -> bool {
        self.start <= byte && byte < self.end
    }

    /// Whether another span lies wholly inside this one. This is the
    /// containment order of the lattice: reflexive, transitive, and
    /// antisymmetric, and a partial order because two spans that merely
    /// overlap are unrelated under it.
    #[must_use]
    pub const fn contains_span(&self, other: Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    /// Whether the two spans share at least one byte.
    ///
    /// An empty span shares none with anything, itself included — it
    /// holds no byte to share. That is why the emptiness test is here
    /// and not left to the endpoint comparisons: `[3,3)` sits *inside*
    /// `[0,8)` by every comparison and still touches nothing of it.
    #[must_use]
    pub const fn intersects(&self, other: Self) -> bool {
        if self.is_empty() || other.is_empty() {
            return false;
        }
        self.start < other.end && other.start < self.end
    }

    /// The Allen interval relation of this span to another. See
    /// [`span_relation`].
    #[must_use]
    pub fn relation(&self, other: Self) -> SpanRelation {
        span_relation(*self, other)
    }
}

/// The thirteen ways two byte spans can lie against each other: Allen's
/// interval relations, with the six inverses named as their own
/// variants.
///
/// The relation is total and exclusive — any two spans stand in exactly
/// one of these — which is what makes it usable as a match arm rather
/// than a pile of boolean tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpanRelation {
    /// `a` ends before `b` starts, with a gap between them.
    Before,
    /// `a` ends exactly where `b` starts.
    Meets,
    /// `a` starts first, and they share bytes, and `b` outlasts `a`.
    Overlaps,
    /// `a` and `b` start together and `a` ends first.
    Starts,
    /// `a` lies strictly inside `b`, sharing neither endpoint.
    During,
    /// `a` and `b` end together and `a` starts later.
    Finishes,
    /// `a` and `b` are the same span.
    Equals,
    /// The inverse of [`Self::Finishes`]: they end together and `a`
    /// starts first.
    FinishedBy,
    /// The inverse of [`Self::During`]: `b` lies strictly inside `a`.
    Contains,
    /// The inverse of [`Self::Starts`]: they start together and `b`
    /// ends first.
    StartedBy,
    /// The inverse of [`Self::Overlaps`]: `b` starts first, and they
    /// share bytes, and `a` outlasts `b`.
    OverlappedBy,
    /// The inverse of [`Self::Meets`]: `b` ends exactly where `a`
    /// starts.
    MetBy,
    /// The inverse of [`Self::Before`]: `b` ends before `a` starts,
    /// with a gap between them.
    After,
}

impl SpanRelation {
    /// The relation seen from the other span: `a.relation(b).inverse()`
    /// is `b.relation(a)`.
    #[must_use]
    pub const fn inverse(self) -> Self {
        match self {
            Self::Before => Self::After,
            Self::Meets => Self::MetBy,
            Self::Overlaps => Self::OverlappedBy,
            Self::Starts => Self::StartedBy,
            Self::During => Self::Contains,
            Self::Finishes => Self::FinishedBy,
            Self::Equals => Self::Equals,
            Self::FinishedBy => Self::Finishes,
            Self::Contains => Self::During,
            Self::StartedBy => Self::Starts,
            Self::OverlappedBy => Self::Overlaps,
            Self::MetBy => Self::Meets,
            Self::After => Self::Before,
        }
    }

    /// Whether the relation says the first span holds the second:
    /// [`Contains`](Self::Contains), [`StartedBy`](Self::StartedBy),
    /// [`FinishedBy`](Self::FinishedBy), or [`Equals`](Self::Equals) —
    /// the four relations the containment order of the lattice is made
    /// of, and exactly the ones for which
    /// [`Span::contains_span`] holds.
    #[must_use]
    pub const fn is_containment(self) -> bool {
        matches!(
            self,
            Self::Contains | Self::StartedBy | Self::FinishedBy | Self::Equals
        )
    }
}

/// The Allen interval relation of `a` to `b`, over half-open byte
/// spans.
///
/// Touching is not overlapping: a span ending exactly where the next
/// begins [`Meets`](SpanRelation::Meets) it, and the two share no byte.
/// That is the endpoint case a hand-written comparison gets wrong, and
/// it is the common one here — a section ends exactly where the next
/// section's heading line begins.
///
/// Every span the model states is non-empty, so the degenerate case is
/// a caller's own span: an empty one is answered as a point, and the
/// gap tests are asked first, so `[3,3)` against `[3,8)` is
/// [`Meets`](SpanRelation::Meets) rather than
/// [`Starts`](SpanRelation::Starts).
#[must_use]
pub fn span_relation(a: Span, b: Span) -> SpanRelation {
    if a.end < b.start {
        return SpanRelation::Before;
    }
    if a.end == b.start {
        return SpanRelation::Meets;
    }
    if b.end < a.start {
        return SpanRelation::After;
    }
    if b.end == a.start {
        return SpanRelation::MetBy;
    }
    match (a.start.cmp(&b.start), a.end.cmp(&b.end)) {
        (Ordering::Less, Ordering::Less) => SpanRelation::Overlaps,
        (Ordering::Less, Ordering::Equal) => SpanRelation::FinishedBy,
        (Ordering::Less, Ordering::Greater) => SpanRelation::Contains,
        (Ordering::Equal, Ordering::Less) => SpanRelation::Starts,
        (Ordering::Equal, Ordering::Equal) => SpanRelation::Equals,
        (Ordering::Equal, Ordering::Greater) => SpanRelation::StartedBy,
        (Ordering::Greater, Ordering::Less) => SpanRelation::During,
        (Ordering::Greater, Ordering::Equal) => SpanRelation::Finishes,
        (Ordering::Greater, Ordering::Greater) => SpanRelation::OverlappedBy,
    }
}

/// A section of the document: a heading, or a movement marker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    level: u32,
    ordinal: u64,
    parent: Option<usize>,
    children: Vec<usize>,
    heading: String,
    span: Span,
    heading_span: Span,
    movement: bool,
}

impl Section {
    /// Depth in the heading tree, from 1. A movement marker sits one
    /// level under the nearest heading, so consecutive movements are
    /// siblings rather than a descending chain.
    #[must_use]
    pub const fn level(&self) -> u32 {
        self.level
    }

    /// Document order among sections, counting from zero. It is also
    /// this section's index in [`Document::sections`].
    #[must_use]
    pub const fn ordinal(&self) -> u64 {
        self.ordinal
    }

    /// The enclosing section's index, or `None` for a section at the
    /// document's own level.
    #[must_use]
    pub const fn parent(&self) -> Option<usize> {
        self.parent
    }

    /// The indices of the sections directly under this one, in document
    /// order. Only the direct children: a grandchild is reached through
    /// its own parent, or by containment with
    /// [`Span::contains_span`].
    #[must_use]
    pub fn children(&self) -> &[usize] {
        &self.children
    }

    /// The heading text as the dialect read it, with its markers and
    /// surrounding whitespace removed. It may be empty — a heading of
    /// nothing but hashes is a heading with an empty title, not the
    /// absence of one.
    #[must_use]
    pub fn heading(&self) -> &str {
        &self.heading
    }

    /// The section's whole span: from the first byte of its opening
    /// line to the byte before the next section at its level or above,
    /// or to the end of the document. Nested subsections and every unit
    /// under them are inside it.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    /// The span of the opening line alone, without its newline: the
    /// bytes the section's identity is minted over.
    #[must_use]
    pub const fn heading_span(&self) -> Span {
        self.heading_span
    }

    /// Whether the section was opened by a movement marker rather than
    /// by a heading. It decides the class the projection states, and
    /// nothing else: a movement is a section in every other respect.
    #[must_use]
    pub const fn is_movement(&self) -> bool {
        self.movement
    }
}

/// A unit: a numbered verse, a paragraph, or one piece of either after
/// the oversize split.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unit<'a> {
    source: &'a str,
    span: Span,
    scalars: Span,
    ordinal: u64,
    verse: Option<u64>,
    section: Option<usize>,
    /// The heading stack in force at the unit's start. Shared, not
    /// copied: every unit of a section has the *same* lineage, and
    /// every piece of a split unit has the lineage of the unit it came
    /// from, so one heading stack per section is built and each unit
    /// and piece holds a handle to it. The strings are immutable and
    /// the handle is behind [`Unit::lineage`], which still answers with
    /// a plain slice, so nothing outside this crate can tell.
    lineage: Arc<[String]>,
    continues: Option<usize>,
    digest: ContentDigest,
}

impl<'a> Unit<'a> {
    /// The unit's byte span in the source.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    /// The same range counted in **Unicode scalars** rather than bytes:
    /// how many scalars of the document precede the unit, and how many
    /// precede its end.
    ///
    /// Byte offsets are the identity and the ground truth; scalar
    /// offsets are what a consumer needs when it hands a range to
    /// something that counts characters — a UTF-16 host, a diff, an
    /// annotation format whose offsets are scalar-based. Deriving one
    /// from the other needs the whole document, which is exactly what
    /// the model has and the consumer of a single claim does not, so it
    /// is counted once here.
    ///
    /// The count is over the document's own scalars from byte zero, so
    /// a leading byte order mark is one scalar, counted, just as it is
    /// one to three bytes, counted.
    #[must_use]
    pub const fn scalar_span(&self) -> Span {
        self.scalars
    }

    /// Document order among units, counting from zero. It is also this
    /// unit's index in [`Document::units`], and a split piece takes its
    /// own place in it.
    #[must_use]
    pub const fn ordinal(&self) -> u64 {
        self.ordinal
    }

    /// The verse number the unit opened with, if it opened with one. A
    /// split piece keeps the verse number of the unit it is part of.
    #[must_use]
    pub const fn verse(&self) -> Option<u64> {
        self.verse
    }

    /// The index of the innermost section in force at the unit's start,
    /// which is *membership*, not containment: containment asks which
    /// sections hold the unit's span, and every ancestor does.
    #[must_use]
    pub const fn section(&self) -> Option<usize> {
        self.section
    }

    /// The heading stack in force at the unit's start, outermost first.
    #[must_use]
    pub fn lineage(&self) -> &[String] {
        &self.lineage
    }

    /// The index of the piece this one continues, for a unit the split
    /// law cut. The first piece continues nothing, and a unit that
    /// never split is a first piece.
    #[must_use]
    pub const fn continues(&self) -> Option<usize> {
        self.continues
    }

    /// The SHA-256 of the unit's own bytes — the digest inside its
    /// identity, taken once here rather than again at every consumer.
    /// A reader holding the source proves a hit is bound to its text by
    /// re-taking this over the span.
    #[must_use]
    pub const fn digest(&self) -> ContentDigest {
        self.digest
    }

    /// The unit's exact bytes: `&source[span]`, verbatim.
    #[must_use]
    pub fn quote(&self) -> &'a str {
        &self.source[self.span.start as usize..self.span.end as usize]
    }

    /// Up to [`CONTEXT_BYTES`] of the document immediately before the
    /// unit, snapped forward to a scalar boundary. Empty at the start
    /// of the document, and short wherever the snap or the document's
    /// edge makes it short.
    #[must_use]
    pub fn prefix(&self) -> &'a str {
        let start = self.span.start as usize;
        let mut from = start.saturating_sub(CONTEXT_BYTES);
        while from < start && !self.source.is_char_boundary(from) {
            from += 1;
        }
        &self.source[from..start]
    }

    /// Up to [`CONTEXT_BYTES`] of the document immediately after the
    /// unit, snapped backward to a scalar boundary. Empty at the end of
    /// the document.
    #[must_use]
    pub fn suffix(&self) -> &'a str {
        let end = self.span.end as usize;
        let to = floor_boundary(
            self.source,
            self.source.len().min(end.saturating_add(CONTEXT_BYTES)),
        );
        &self.source[end..to]
    }

    /// The unit's content anchor: its exact text between a bounded
    /// prefix and suffix. Three strings that identify the unit's text
    /// in a *revised* document, where every byte offset has moved —
    /// which is the one thing a span cannot do.
    #[must_use]
    pub fn anchor(&self) -> ContentAnchor<'a> {
        ContentAnchor {
            prefix: self.prefix(),
            exact: self.quote(),
            suffix: self.suffix(),
        }
    }

    /// The unit as the kernel's own chunk subject: the
    /// [`TextChunkTarget`] a `.purremb` pack addresses this unit by.
    ///
    /// Every field is the model's, handed over unchanged — the byte span
    /// of [`Self::span`], the scalar span of [`Self::scalar_span`], and
    /// the digest of [`Self::digest`] — so nothing is re-derived here
    /// and the two views of the unit cannot drift apart.
    ///
    /// `document_id` is the target the unit's *document* was minted as
    /// (a [`DocumentTarget`](purrdf_core::embedding::DocumentTarget)
    /// over the same bytes), and `chunking_id` is the family's chunking
    /// id — the one derived from
    /// [`Profile::purremb_chunking_stage`](crate::Profile::purremb_chunking_stage),
    /// never the profile's law id. Both are the caller's to supply
    /// because both are facts about the pack, not about the document.
    ///
    /// # The verification law
    ///
    /// [`TextChunkTarget::verify_document`] is the kernel's **reference
    /// verification law** for these targets: given the document's exact
    /// bytes it re-derives the span's digest and both scalar
    /// coordinates and refuses anything that disagrees, an out-of-bounds
    /// span and a cut scalar among it. [`verify_unit`] is this crate's
    /// mirror of that law and delegates to it, so there is one law and
    /// not two.
    #[must_use]
    pub const fn text_chunk_target(
        &self,
        document_id: TargetId,
        chunking_id: ChunkingContractId,
    ) -> TextChunkTarget {
        TextChunkTarget {
            document_id,
            chunking_id,
            content_digest: self.digest,
            byte_start: self.span.start,
            byte_end: self.span.end,
            scalar_start: self.scalars.start,
            scalar_end: self.scalars.end,
        }
    }
}

/// Proves a unit against the bytes it claims to annotate: the span is
/// inside them, it falls on scalar boundaries at both ends, and the
/// bytes at it still digest to the digest the unit carries.
///
/// This is the one question a [`Document`] cannot answer at slicing
/// time. A unit is minted over bytes that were what they were *then*;
/// this asks whether the bytes in hand *now* are still those. A consumer
/// that stored spans and later re-read the file, or received a claim and
/// a document from two places, asks here before trusting that the two
/// belong together.
///
/// The law applied is the kernel's own — the target of
/// [`Unit::text_chunk_target`] handed to
/// [`TextChunkTarget::verify_document`] — so this crate and a PURREMB
/// consumer refuse the same bytes for the same reason. `document_id` and
/// `chunking_id` travel through unchanged and cannot affect the answer;
/// they are asked for so that what is proved is the very target a
/// consumer would ship, rather than a stand-in for it.
///
/// # Errors
///
/// [`MarkdownError::TamperedUnit`], naming the unit's span and carrying
/// the kernel's own finding.
pub fn verify_unit(
    bytes: &[u8],
    unit: &Unit<'_>,
    document_id: TargetId,
    chunking_id: ChunkingContractId,
) -> Result<(), MarkdownError> {
    unit.text_chunk_target(document_id, chunking_id)
        .verify_document(bytes)
        .map_err(|cause| MarkdownError::TamperedUnit {
            span: (unit.span.start, unit.span.end),
            cause,
        })
}

/// A unit's text with bounded context on each side: the anchor a
/// consumer re-finds the unit by after the document has been edited.
///
/// It is deterministic in the bytes — the same document yields the same
/// three strings on every target — and it is deliberately *not* an
/// identity: two identical paragraphs in one document have identical
/// exact text and differ only in their context, which is what the
/// context is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentAnchor<'a> {
    prefix: &'a str,
    exact: &'a str,
    suffix: &'a str,
}

impl<'a> ContentAnchor<'a> {
    /// The bounded text before the quote.
    #[must_use]
    pub const fn prefix(&self) -> &'a str {
        self.prefix
    }

    /// The quote itself: the unit's exact bytes.
    #[must_use]
    pub const fn exact(&self) -> &'a str {
        self.exact
    }

    /// The bounded text after the quote.
    #[must_use]
    pub const fn suffix(&self) -> &'a str {
        self.suffix
    }
}

/// One concordance row, unflattened: the verse range it covers, its
/// canon sources, its anchors, and what it lifted **in this document**.
///
/// The projection carries most of this now: each lift onto a unit mints
/// a citation node that reifies the row's `cites` edge and holds the
/// row's own sources, so *this row named these anchors beside those
/// sources* is a fact the graph states. What the graph still cannot
/// state is a row that names a verse this document does not carry —
/// there is no unit for it to be an edge of. The model states that too,
/// run by run ([`Self::unmatched`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Citation {
    first: u64,
    last: u64,
    sources: Vec<String>,
    anchors: Vec<String>,
    lifted: Vec<(u64, usize)>,
    /// The verses of the range this document does not carry, as the
    /// maximal runs they form. See [`Self::unmatched`] for why runs.
    unmatched: Vec<(u64, u64)>,
    span: Span,
}

impl Citation {
    /// The first and last verse of the row's range, inclusive. A row
    /// naming a single verse states it as both.
    #[must_use]
    pub const fn verses(&self) -> (u64, u64) {
        (self.first, self.last)
    }

    /// The backticked canon source paths of the row, in the order the
    /// row wrote them.
    #[must_use]
    pub fn sources(&self) -> &[String] {
        &self.sources
    }

    /// The backticked anchors of the row, in the order the row wrote
    /// them.
    #[must_use]
    pub fn anchors(&self) -> &[String] {
        &self.anchors
    }

    /// What the row lifted here: each verse of its range this document
    /// carries, paired with the index of the unit it lifted onto — the
    /// first piece of that verse.
    #[must_use]
    pub fn lifted(&self) -> &[(u64, usize)] {
        &self.lifted
    }

    /// The verses of the row's range this document does not carry, as
    /// the maximal runs they form: each an inclusive `(first, last)`
    /// pair, in ascending order, none of them touching. A row that
    /// lifted nothing at all states its whole range as one run, and a
    /// row that lifted every verse it named states none.
    ///
    /// **Not an error**: see [`Document::unmatched_citations`].
    ///
    /// # Why runs, and not the verses
    ///
    /// A row's range is a pair of `u64`s the *document* wrote, and the
    /// document is not trusted. `| 1–18446744073709551615 |` is a
    /// perfectly readable row naming more verses than there are bytes
    /// in every disk ever made; listing them one at a time is a hang,
    /// not an answer, and a slicer that is a pure function of its bytes
    /// cannot have one input shape that never returns.
    ///
    /// The runs are the same fact, and they are **bounded by what this
    /// document carries** rather than by what a row claims: there is at
    /// most one run before each verse the row lifted and one after the
    /// last, so a row can never state more runs than it lifted verses,
    /// plus one. Reading a run back verse by verse is `first..=last`
    /// where a caller wants that, and it is the caller — who knows how
    /// many it is willing to walk — that decides to.
    #[must_use]
    pub fn unmatched(&self) -> &[(u64, u64)] {
        &self.unmatched
    }

    /// Whether the row lifted nothing at all here.
    #[must_use]
    pub fn is_unmatched(&self) -> bool {
        self.lifted.is_empty()
    }

    /// The span of the row's own line, so a caller reporting on a row
    /// can point at it.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

/// A row of a concordance table that could not be read as a citation at
/// all, and why.
///
/// A malformed row is *not* a refusal — nothing is minted from it, so
/// there is nothing unlawful to refuse — but it is not nothing either:
/// a row written to cite verses that cites none is a defect in the
/// document, and a slicer that drops it in silence is the reason the
/// author never finds out. The model carries it out as data so a
/// consumer can report it, count it, or fail its own build on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MalformedRow<'a> {
    source: &'a str,
    span: Span,
    defect: RowDefect,
}

impl<'a> MalformedRow<'a> {
    /// The span of the row's line, without its newline.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    /// The row's line, verbatim.
    #[must_use]
    pub fn line(&self) -> &'a str {
        &self.source[self.span.start as usize..self.span.end as usize]
    }

    /// What could not be made of it.
    #[must_use]
    pub const fn defect(&self) -> RowDefect {
        self.defect
    }
}

/// What a concordance row failed at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RowDefect {
    /// The row states fewer than the three cells a concordance row is:
    /// the verse range, the canon sources, the anchors.
    TooFewCells {
        /// How many cells the row did state.
        found: usize,
    },
    /// The row states three cells or more, but its first cell is not a
    /// verse range (`4`, `2-5`, or `2–5` with an en dash, first at or
    /// under last, each a `u64`), so there is no verse to lift onto.
    UnreadableVerseRange,
}

/// A document as a typed layer of annotations over untouched source
/// bytes: the stand-off model [`analyze`](crate::analyze) returns.
///
/// It holds no copy of the text — it borrows the bytes it was analyzed
/// over — and it holds nothing the source cannot answer for: every
/// section, unit, and citation names a byte span of that source, and
/// the strings it does own are the ones the dialect *read* (a heading's
/// text, an anchor's name) rather than the ones it could quote.
///
/// It also holds **the profile it was admitted under**, which is the
/// one thing here that is not read off the source. That is what makes
/// [`render`](crate::render) infallible rather than merely documented
/// as such: the law the projection applies is the law
/// [`analyze`](crate::analyze) answered for — the same vocabulary, the
/// same constants, the same canon base every anchor of this document
/// was walked against — and there is no seam at which a second law
/// could be handed in. A document admitted with no canon base, whose
/// anchors were therefore never asked to mint an IRI, cannot later be
/// projected under a base that would mint them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document<'a> {
    id: &'a str,
    source: &'a str,
    profile: Profile,
    title: Option<String>,
    sections: Vec<Section>,
    units: Vec<Unit<'a>>,
    citations: Vec<Citation>,
    malformed_rows: Vec<MalformedRow<'a>>,
}

impl<'a> Document<'a> {
    /// The document's IRI, as declared to [`analyze`](crate::analyze).
    #[must_use]
    pub const fn id(&self) -> &'a str {
        self.id
    }

    /// The document's bytes, as text. Every span of the model indexes
    /// this.
    #[must_use]
    pub const fn source(&self) -> &'a str {
        self.source
    }

    /// The profile this document was admitted under: the law
    /// [`analyze`](crate::analyze) answered for and the law
    /// [`render`](crate::render) applies.
    ///
    /// It is offered because a consumer that keeps the model regularly
    /// needs it — the contract id inside every node identity
    /// ([`Profile::contract_id`]), the `sliceProfile` literal
    /// ([`Profile::label`]), the chunking stage a `.purremb` family
    /// carries ([`Profile::purremb_chunking_stage`]), the vocabulary an
    /// index reads the emitted triples back through — and reaching for
    /// the caller's own copy risks reaching for a *different* one.
    /// There is one law per document, and this is it.
    ///
    /// The document owns its profile rather than borrowing one, so
    /// analysis outlives the caller's binding and no consumer is put to
    /// lifetime work to keep a model around.
    #[must_use]
    pub const fn profile(&self) -> &Profile {
        &self.profile
    }

    /// How many bytes the document is.
    #[must_use]
    pub fn byte_length(&self) -> u64 {
        self.source.len() as u64
    }

    /// The whole document as a span: the top of the containment
    /// lattice.
    #[must_use]
    pub fn span(&self) -> Span {
        Span::new(0, self.byte_length())
    }

    /// How many Unicode scalars of the document precede a byte offset.
    ///
    /// `None` when the offset is past the document's end, and `None`
    /// when it falls **inside** a scalar. A byte offset that is not a
    /// boundary names no position in the text at all, and answering with
    /// a rounded one is exactly how a consumer moves a span without
    /// learning that it did. The document's own end *is* a boundary, and
    /// answers with the document's whole scalar count.
    ///
    /// Byte offsets are this crate's ground truth and its identities;
    /// scalar offsets are what every JS, Python, and wasm consumer
    /// indexes text by. The conversion is offered here so it is written
    /// once, against the document that has the bytes, rather than
    /// reimplemented in each host — [`Unit::scalar_span`] is this same
    /// count, taken once for every unit at analysis.
    ///
    /// It scans: O(n) in the bytes before the offset.
    #[must_use]
    pub fn byte_to_scalar(&self, byte_offset: u64) -> Option<u64> {
        let offset = usize::try_from(byte_offset).ok()?;
        // `is_char_boundary` is false past the end and false inside a
        // scalar, which are the two refusals, and true at the end.
        if !self.source.is_char_boundary(offset) {
            return None;
        }
        Some(self.source[..offset].chars().count() as u64)
    }

    /// The byte offset a scalar offset names: the inverse of
    /// [`Self::byte_to_scalar`], and its exact inverse wherever either
    /// answers.
    ///
    /// Total on `0..=` the document's scalar count — the count itself
    /// answers with the document's byte length, so a half-open scalar
    /// range ending at the document's end converts — and `None` past
    /// it.
    ///
    /// It scans: O(n) in the scalars before the offset.
    #[must_use]
    pub fn scalar_to_byte(&self, scalar_offset: u64) -> Option<u64> {
        let wanted = usize::try_from(scalar_offset).ok()?;
        // Every scalar's start, then the end: the boundaries in order,
        // which is precisely what a scalar offset indexes.
        self.source
            .char_indices()
            .map(|(byte, _)| byte)
            .chain(std::iter::once(self.source.len()))
            .nth(wanted)
            .map(|byte| byte as u64)
    }

    /// The document's title: the first heading that is not a movement
    /// marker, if the document has one. An empty title is a title.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Every section, in document order.
    #[must_use]
    pub fn sections(&self) -> &[Section] {
        &self.sections
    }

    /// Every unit, in document order, including each piece of a unit
    /// the split law cut.
    #[must_use]
    pub fn units(&self) -> &[Unit<'a>] {
        &self.units
    }

    /// Every concordance row that could be read, in document order,
    /// whether or not it lifted anything here.
    #[must_use]
    pub fn citations(&self) -> &[Citation] {
        &self.citations
    }

    /// Every concordance row that could not be read at all, in document
    /// order. See [`MalformedRow`].
    #[must_use]
    pub fn malformed_rows(&self) -> &[MalformedRow<'a>] {
        &self.malformed_rows
    }

    /// The rows that lifted nothing in this document.
    ///
    /// **A concordance may cover a wider canon than the document that
    /// carries it.** One table shared across the volumes of a canon
    /// names verses that live in sibling documents; a row whose verses
    /// are all elsewhere lifts nothing *here* and is not an error, and
    /// this crate refuses nothing on account of it. It is reported
    /// because the same shape has a second cause — a typo in a verse
    /// number, a row left behind by an edit — and only the caller knows
    /// which of the two its corpus is. A single-document canon in which
    /// this iterator is non-empty has a defect; a multi-document canon
    /// in which it is non-empty is working exactly as intended.
    pub fn unmatched_citations(&self) -> impl Iterator<Item = &Citation> {
        self.citations.iter().filter(|c| c.is_unmatched())
    }

    /// A section by index, as [`Section::parent`] and
    /// [`Unit::section`] state it.
    #[must_use]
    pub fn section(&self, index: usize) -> Option<&Section> {
        self.sections.get(index)
    }

    /// A unit by index, as [`Unit::continues`] and
    /// [`Citation::lifted`] state it.
    #[must_use]
    pub fn unit(&self, index: usize) -> Option<&Unit<'a>> {
        self.units.get(index)
    }

    /// The sections directly under a section, in document order.
    pub fn child_sections(&self, section: &Section) -> impl Iterator<Item = &Section> {
        section.children().iter().filter_map(|&i| self.section(i))
    }

    /// The unit covering a byte, if one covers it.
    ///
    /// Not every byte has one: a heading line belongs to a section and
    /// to no unit, a blank line separates units, and the newline a
    /// split cut at is carried by no piece. Where the overlap of a
    /// split makes two pieces cover one byte, this answers with the
    /// earlier piece — the first in document order — and
    /// [`Self::covering`] answers with both.
    #[must_use]
    pub fn unit_at(&self, byte: u64) -> Option<&Unit<'a>> {
        self.units.iter().find(|u| u.span().contains(byte))
    }

    /// Every unit sharing a byte with a span, in document order.
    ///
    /// Intersection, not containment: a unit that begins inside the
    /// span and runs past its end is answered, because a reader asking
    /// what text a range touches wants it. An empty span touches
    /// nothing.
    pub fn covering(&self, span: Span) -> impl Iterator<Item = &Unit<'a>> {
        self.units.iter().filter(move |u| u.span().intersects(span))
    }

    /// Every unit whose span lies inside a section's span, in document
    /// order — the containment order of the lattice, so the units of a
    /// nested subsection are under its ancestors too.
    ///
    /// This is the transitive reading. For the units whose *innermost*
    /// section is this one, filter on [`Unit::section`] instead; the
    /// two answers differ exactly by the nested sections, and both are
    /// wanted often enough to be worth telling apart by name.
    pub fn units_under(&self, section: &Section) -> impl Iterator<Item = &Unit<'a>> {
        let span = section.span();
        self.units
            .iter()
            .filter(move |u| span.contains_span(u.span()))
    }

    /// Assembles the model from what a dialect reader read: the
    /// containment closure over the sections, the split law over the
    /// units, the scalar count over the whole text, and the match of
    /// every concordance row against the verses this document carries.
    ///
    /// The profile is kept, not merely consulted. This constructor is
    /// `pub(crate)` and reached only from [`analyze`](crate::analyze),
    /// which has already validated the profile handed here, so the
    /// profile a document carries is always one the crate has answered
    /// for.
    pub(crate) fn assemble(
        id: &'a str,
        source: &'a str,
        reading: &Reading,
        profile: &Profile,
    ) -> Self {
        let sections = assemble_sections(reading, source.len());
        let units = assemble_units(source, reading, profile);
        let citations = assemble_citations(reading, &units);
        let malformed_rows = reading
            .defective_rows
            .iter()
            .map(|row| MalformedRow {
                source,
                span: Span::new(row.start as u64, row.end as u64),
                defect: row.defect,
            })
            .collect();
        Self {
            id,
            source,
            profile: profile.clone(),
            title: reading.title.clone(),
            sections,
            units,
            citations,
            malformed_rows,
        }
    }
}

/// A section runs to the next section at its level or above, and its
/// parent is the nearest section before it at a shallower level. Both
/// are containment facts, read off the levels alone — which is why the
/// dialect reader states a level and says nothing about either.
///
/// One pass over the sections answers both, off one stack of the
/// sections still open, whose levels strictly increase. The section on
/// top is by construction the nearest preceding shallower one, so it is
/// the parent; and every section the arriving one closes is closed *by*
/// it, because a section between the two at that level or above would
/// have closed it already. So each section is pushed once and popped
/// once and nothing is re-scanned — where searching forward for the
/// close re-walked, for every section, every descendant it holds.
fn assemble_sections(reading: &Reading, len: usize) -> Vec<Section> {
    let raw = &reading.sections;
    let mut sections: Vec<Section> = Vec::with_capacity(raw.len());
    let mut stack: Vec<usize> = Vec::new();
    for (index, s) in raw.iter().enumerate() {
        while let Some(&open) = stack.last() {
            if raw[open].level < s.level {
                break;
            }
            stack.pop();
            sections[open].span.end = s.start as u64;
        }
        sections.push(Section {
            level: s.level,
            ordinal: index as u64,
            parent: stack.last().copied(),
            children: Vec::new(),
            heading: s.heading.clone(),
            // Open to the document's end; a later section that closes
            // this one overwrites it, and one that never comes leaves
            // the section running to the end, which is the law.
            span: Span::new(s.start as u64, len as u64),
            heading_span: Span::new(s.start as u64, s.line_end as u64),
            movement: s.movement,
        });
        stack.push(index);
    }
    for index in 0..sections.len() {
        if let Some(parent) = sections[index].parent {
            sections[parent].children.push(index);
        }
    }
    sections
}

/// Each unit the reader stated, through the split law, with its scalar
/// span and its content digest taken once.
fn assemble_units<'a>(source: &'a str, reading: &Reading, profile: &Profile) -> Vec<Unit<'a>> {
    let scalars = scalar_starts(source);
    let scalar_at = |byte: usize| scalars.partition_point(|&s| s < byte) as u64;
    let mut units = Vec::with_capacity(reading.units.len());
    for raw in &reading.units {
        let mut previous = None;
        for (start, end) in split_spans(source, raw.start, raw.end, profile) {
            let index = units.len();
            units.push(Unit {
                source,
                span: Span::new(start as u64, end as u64),
                scalars: Span::new(scalar_at(start), scalar_at(end)),
                ordinal: index as u64,
                verse: raw.verse,
                section: raw.section,
                lineage: Arc::clone(&raw.lineage),
                continues: previous,
                digest: ContentDigest::of(&source.as_bytes()[start..end]),
            });
            previous = Some(index);
        }
    }
    units
}

/// The byte offset of every Unicode scalar of the text, so a byte
/// offset becomes a scalar offset by one binary search rather than by
/// re-walking the document.
fn scalar_starts(source: &str) -> Vec<usize> {
    source.char_indices().map(|(i, _)| i).collect()
}

/// Every row against the verses this document carries. A row lifts onto
/// the **first piece** of a verse: an oversize verse's continuations
/// carry the same number, and a citation states one fact about the
/// verse, not one about each of its pieces.
///
/// The row's range is *never* walked. A range is two `u64`s a document
/// wrote and a document is not trusted: `1–18446744073709551615` is a
/// readable row, and walking it would be a hang rather than an answer —
/// the one input shape from which this crate's pure function never
/// returns. So the walk goes the other way, over the verses the
/// document actually carries that fall in the range
/// ([`BTreeMap::range`]), which is bounded by the document; what the
/// row named and the document lacks is the gaps between them, stated as
/// runs. The cost of a row is therefore what it lifted, not what it
/// claimed.
fn assemble_citations(reading: &Reading, units: &[Unit<'_>]) -> Vec<Citation> {
    let first_pieces = first_pieces(units);
    reading
        .rows
        .iter()
        .map(|row| {
            let mut lifted = Vec::new();
            let mut unmatched = Vec::new();
            // The first verse of the range not yet accounted for, or
            // `None` once the accounting has run off the end of `u64`.
            let mut open = Some(row.first);
            for (&verse, &unit) in first_pieces.range(row.first..=row.last) {
                if let Some(gap) = open
                    && gap < verse
                {
                    unmatched.push((gap, verse - 1));
                }
                lifted.push((verse, unit));
                open = verse.checked_add(1);
            }
            if let Some(gap) = open
                && gap <= row.last
            {
                unmatched.push((gap, row.last));
            }
            Citation {
                first: row.first,
                last: row.last,
                sources: row.sources.clone(),
                anchors: row.anchors.clone(),
                lifted,
                unmatched,
                span: Span::new(row.start as u64, row.end as u64),
            }
        })
        .collect()
}

/// The index of the first piece of every numbered verse the document
/// carries, taken in one pass over the units.
///
/// Document order is what makes the map the *first* piece: a verse
/// already in the map keeps the index it was entered with. A
/// continuation carries the verse number of the unit it continues and
/// is skipped here for the same reason it was skipped before — a
/// citation states one fact about the verse, not one about each of its
/// pieces. Taking the map once is the difference between a concordance
/// that costs one walk of the units and one that costs a walk of them
/// per verse named.
fn first_pieces(units: &[Unit<'_>]) -> BTreeMap<u64, usize> {
    let mut out = BTreeMap::new();
    for (index, unit) in units.iter().enumerate() {
        if let Some(verse) = unit.verse
            && unit.continues.is_none()
        {
            out.entry(verse).or_insert(index);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_allen_relation_is_its_inverse_seen_from_the_other_span() {
        let spans = [
            Span::new(0, 4),
            Span::new(4, 8),
            Span::new(2, 6),
            Span::new(0, 8),
            Span::new(9, 12),
        ];
        for a in spans {
            for b in spans {
                assert_eq!(
                    a.relation(b).inverse(),
                    b.relation(a),
                    "{a:?} against {b:?}"
                );
                assert_eq!(
                    a.relation(b).is_containment(),
                    a.contains_span(b),
                    "{a:?} against {b:?}"
                );
            }
        }
    }

    #[test]
    fn a_scalar_offset_counts_the_scalars_before_a_byte() {
        let text = "a\u{e9}\u{4e2d}\u{1f41a}z";
        let starts = scalar_starts(text);
        assert_eq!(starts, vec![0, 1, 3, 6, 10]);
        let at = |byte: usize| starts.partition_point(|&s| s < byte);
        assert_eq!(at(0), 0);
        assert_eq!(at(1), 1);
        assert_eq!(at(6), 3);
        assert_eq!(at(text.len()), 5);
    }
}
