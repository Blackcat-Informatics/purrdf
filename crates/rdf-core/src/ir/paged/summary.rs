// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! [`PageSummary`] — a frozen page's exact per-term and per-graph row counts, in the
//! page's own LOCAL [`TermId`] space.
//!
//! A `PageSummary` is produced ONLY by [`PageSummary::seal`], which walks the page's
//! base quads, its reifier side-table, its annotation side-table, and its named-graph
//! list exactly once each, and self-reconciles every count against an independent
//! total before returning. Making `seal` the sole producer means a wrong summary is
//! *unrepresentable* rather than merely validated after the fact: there is no public
//! constructor a caller could hand hand-built (and possibly wrong) counts to.
//!
//! Every count is an exact `u64` — never a `u32` or a saturating/truncating counter. A
//! truncating counter that wrapped to zero would silently authorize skipping a page
//! that in fact holds rows, which is the silent-data-loss failure mode this type
//! exists to make impossible.
//!
//! # The sealed digest
//!
//! A summary also carries a [`digest`](PageSummary::digest): one `u64` mixed, at seal
//! time, over EVERY count the summary holds (see [`PageCounts::digest`]). Comparing
//! two digests is `O(1)` and comparing two whole summaries is `O(page size)`, which is
//! what lets the admission path certify a freshly materialized page in every build
//! profile instead of only under `debug_assertions`: a page whose content has drifted
//! since its summary was sealed — in EITHER direction, including the under-reporting
//! one that authorizes skipping rows the page actually holds — mixes to a different
//! digest and is refused with a typed invalid-data fault.
//!
//! The mixing function is written inline from fixed constants precisely so it is
//! byte-identical across runs, platforms and `wasm32-unknown-unknown`. It is NOT a
//! `std::collections::hash_map::DefaultHasher`/`RandomState` hash, which is seeded
//! per-process and would make the digest a different number on every run; and it
//! reads no address, clock, or thread identity.

use std::fmt;

use crate::ir::pack::bits::{IntVector, bits_for};

use crate::ir::{RdfDataset, TermId};

/// Which of a page's three quad-shaped record streams a [`PageSummary`] query
/// concerns: the base (asserted) quads, the RDF 1.2 reifier side-table, or the RDF
/// 1.2 annotation side-table. Mirrors [`PagedQuadTable`](super::PagedQuadTable) one
/// level down, at the single-page granularity `PageSummary` works in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PageStream {
    /// The base (asserted) quads.
    Base,
    /// The reifier side-table's `(reifier, triple-term, graph)` bindings.
    Reifier,
    /// The annotation side-table's `(reifier, predicate, object, graph)` rows.
    Annotation,
}

/// Why a page could not be summarized: it named something its own frozen tables do
/// not contain, so no honest set of counts exists for it.
///
/// Sealing runs on provider-supplied content — including, in
/// [`PagedDataset::verify_parts`](super::PagedDataset::verify_parts), content whose
/// whole purpose is to be certified BEFORE it is trusted — so this is a typed
/// refusal, never a panic. Every caller maps it onto the typed error of its own
/// surface (`PagedFreezeError::SummaryDrift`, `PageFault::invalid_data`, or
/// `PagedQueryError::InvalidData`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SummaryDefect {
    /// The diagnostic sentence, already naming the offending row.
    message: String,
}

impl fmt::Display for SummaryDefect {
    /// Writes the diagnostic sentence [`SummaryDefect::message`] already carries,
    /// verbatim — the type never needs formatting beyond that one prepared string.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// The 64-bit mixing constant seeding [`Digest`] and re-added after every word. Two
/// of the three constants below are the SplitMix64 finalizer's; this one is the
/// golden-ratio odd constant. All three are fixed literals — the digest must be the
/// same number on every run, every platform, and `wasm32-unknown-unknown`.
const DIGEST_SEED: u64 = 0x9e37_79b9_7f4a_7c15;

/// The SplitMix64 finalizer: a BIJECTION on `u64` built from fixed constants.
///
/// Bijectivity is the property the digest's guarantee rests on. Each step of
/// [`Digest::write`] is `state -> mix(state ^ word) + SEED`, a composition of three
/// bijections, so for a fixed word the state map is injective and for a fixed state
/// the word map is injective. Two count sequences that differ in exactly ONE position
/// and agree everywhere else therefore ALWAYS finish at different digests — they
/// diverge at that word and then evolve under identical injective steps. Sequences
/// differing in several positions collide only at the `2^-64` rate any 64-bit digest
/// carries.
const fn mix(mut z: u64) -> u64 {
    z ^= z >> 30;
    z = z.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    z
}

/// A running, order-SENSITIVE digest over a sequence of `u64` words.
#[derive(Debug)]
struct Digest(u64);

impl Digest {
    /// Start from the fixed seed (never a randomized one).
    const fn new() -> Self {
        Self(DIGEST_SEED)
    }

    /// Absorb one word. See [`mix`] for why this step is injective in both arguments.
    #[inline]
    fn write(&mut self, word: u64) {
        self.0 = mix(self.0 ^ word).wrapping_add(DIGEST_SEED);
    }

    /// Absorb a whole count vector: its field tag, then its LENGTH, then every value
    /// in index order. Feeding the length is what keeps `[1, 2]` and `[1, 2, 0]`
    /// distinct — a page that drops trailing terms or trailing graphs is a drift, and
    /// without the length the two would absorb the same words after the zeros are
    /// mixed in.
    fn write_field(&mut self, tag: u64, values: &[u64]) {
        self.write(tag);
        self.write(values.len() as u64);
        for &value in values {
            self.write(value);
        }
    }

    /// Finalize with one more mixing round, so the last word absorbed is diffused
    /// across all 64 output bits rather than sitting in the state's low end.
    const fn finish(self) -> u64 {
        mix(self.0)
    }
}

/// Field tags, absorbed ahead of each count vector so that two DIFFERENT fields
/// holding the same values cannot cancel out against each other (for example, rows
/// moving wholesale from the reifier column to the annotation column).
mod tag {
    /// The page's named-graph list.
    pub(super) const GRAPHS: u64 = 0x01;
    /// Base-quad subject-position occurrence counts.
    pub(super) const SUBJECT: u64 = 0x02;
    /// Base-quad predicate-position occurrence counts.
    pub(super) const PREDICATE: u64 = 0x03;
    /// Base-quad object-position occurrence counts.
    pub(super) const OBJECT: u64 = 0x04;
    /// Reifier-table reifier-column occurrence counts.
    pub(super) const REIFIER: u64 = 0x05;
    /// Annotation-table reifier-column occurrence counts.
    pub(super) const ANNOTATION: u64 = 0x06;
    /// Per-graph base-quad row counts.
    pub(super) const GRAPH_BASE: u64 = 0x07;
    /// Per-graph reifier-table row counts.
    pub(super) const GRAPH_REIFIER: u64 = 0x08;
    /// Per-graph annotation-table row counts.
    pub(super) const GRAPH_ANNOTATION: u64 = 0x09;
    /// The three default-graph row scalars.
    pub(super) const DEFAULT_ROWS: u64 = 0x0a;
}

/// The raw counts one pass over a page produces, before they are bit-packed into a
/// [`PageSummary`]'s [`IntVector`]s.
///
/// This is the shared accumulation core of BOTH certification paths:
/// [`PageSummary::seal`] packs these counts and keeps them, while
/// [`PageSummary::digest_of`] takes only their [`digest`](Self::digest) and drops
/// them. Sharing the accumulator is what makes "the digest of a fresh page" and "the
/// digest sealed into that page's summary" the same number by construction rather
/// than by two implementations agreeing.
#[derive(Debug)]
struct PageCounts {
    /// The page's named graphs, ascending — exactly `RdfDataset::named_graphs()`.
    graphs: Box<[TermId]>,
    /// Base-quad subject-position counts, indexed by local term index.
    subject: Vec<u64>,
    /// Base-quad predicate-position counts, indexed by local term index.
    predicate: Vec<u64>,
    /// Base-quad object-position counts, indexed by local term index.
    object: Vec<u64>,
    /// Reifier-column counts, indexed by local term index.
    reifier: Vec<u64>,
    /// Annotation reifier-column counts, indexed by local term index.
    annotation: Vec<u64>,
    /// Per-graph base-quad row counts, parallel to `graphs`.
    graph_base: Vec<u64>,
    /// Per-graph reifier-table row counts, parallel to `graphs`.
    graph_reifier: Vec<u64>,
    /// Per-graph annotation-table row counts, parallel to `graphs`.
    graph_annotation: Vec<u64>,
    /// Base-quad rows whose `g` is `None`.
    default_base_rows: u64,
    /// Reifier-table rows whose `g` is `None`.
    default_reifier_rows: u64,
    /// Annotation-table rows whose `g` is `None`.
    default_annotation_rows: u64,
}

impl PageCounts {
    /// One pass each over `page.quads()`, `page.reifier_quads()`,
    /// `page.annotation_quads()`, plus `page.named_graphs()`.
    ///
    /// Every lookup into a scratch array is CHECKED and refuses with a
    /// [`SummaryDefect`] rather than panicking: `page` arrives from a
    /// [`PageProvider`](super::PageProvider), and the one entry point whose whole
    /// purpose is certifying content it does not trust must return a typed refusal.
    fn accumulate(page: &RdfDataset) -> Result<Self, SummaryDefect> {
        let term_count = page.term_count();
        let graphs: Box<[TermId]> = page.named_graphs().collect();

        let mut counts = Self {
            subject: vec![0u64; term_count],
            predicate: vec![0u64; term_count],
            object: vec![0u64; term_count],
            reifier: vec![0u64; term_count],
            annotation: vec![0u64; term_count],
            graph_base: vec![0u64; graphs.len()],
            graph_reifier: vec![0u64; graphs.len()],
            graph_annotation: vec![0u64; graphs.len()],
            graphs,
            default_base_rows: 0,
            default_reifier_rows: 0,
            default_annotation_rows: 0,
        };

        let mut reifier_row_total = 0u64;
        let mut annotation_row_total = 0u64;

        for q in page.quads() {
            bump_term(&mut counts.subject, q.s, "base-quad subject")?;
            bump_term(&mut counts.predicate, q.p, "base-quad predicate")?;
            bump_term(&mut counts.object, q.o, "base-quad object")?;
            match q.g {
                None => counts.default_base_rows += 1,
                Some(g) => {
                    let index = graph_index(&counts.graphs, g)?;
                    counts.graph_base[index] += 1;
                }
            }
        }

        for q in page.reifier_quads() {
            reifier_row_total += 1;
            bump_term(&mut counts.reifier, q.s, "reifier-table reifier")?;
            match q.g {
                None => counts.default_reifier_rows += 1,
                Some(g) => {
                    let index = graph_index(&counts.graphs, g)?;
                    counts.graph_reifier[index] += 1;
                }
            }
        }

        for q in page.annotation_quads() {
            annotation_row_total += 1;
            bump_term(&mut counts.annotation, q.s, "annotation-table reifier")?;
            match q.g {
                None => counts.default_annotation_rows += 1,
                Some(g) => {
                    let index = graph_index(&counts.graphs, g)?;
                    counts.graph_annotation[index] += 1;
                }
            }
        }

        // Self-reconciliation: sum of subject/predicate/object counts must each equal
        // the page's own quad count, and the default+per-graph row split must sum to
        // the same total, for all three streams. A mismatch is a broken accumulation
        // bug in the loops directly above — not a statement about the provider's
        // content, which the checked bumps have already screened — so it is a hard
        // failure here rather than a wrong answer surfacing later.
        let quad_count = page.quad_count() as u64;
        let subject_total: u64 = counts.subject.iter().sum();
        let predicate_total: u64 = counts.predicate.iter().sum();
        let object_total: u64 = counts.object.iter().sum();
        assert_eq!(
            subject_total, quad_count,
            "page_summary: subject occurrence total {subject_total} != page quad count {quad_count}"
        );
        assert_eq!(
            predicate_total, quad_count,
            "page_summary: predicate occurrence total {predicate_total} != page quad count \
             {quad_count}"
        );
        assert_eq!(
            object_total, quad_count,
            "page_summary: object occurrence total {object_total} != page quad count {quad_count}"
        );
        let graph_base_total: u64 = counts.graph_base.iter().sum();
        assert_eq!(
            counts.default_base_rows + graph_base_total,
            quad_count,
            "page_summary: default_base_rows ({}) + graph_base_rows sum ({graph_base_total}) != \
             page quad count {quad_count}",
            counts.default_base_rows
        );

        let reifier_total: u64 = counts.reifier.iter().sum();
        assert_eq!(
            reifier_total, reifier_row_total,
            "page_summary: reifier occurrence total {reifier_total} != reifier row total \
             {reifier_row_total}"
        );
        let graph_reifier_total: u64 = counts.graph_reifier.iter().sum();
        assert_eq!(
            counts.default_reifier_rows + graph_reifier_total,
            reifier_row_total,
            "page_summary: default_reifier_rows ({}) + graph_reifier_rows sum \
             ({graph_reifier_total}) != reifier row total {reifier_row_total}",
            counts.default_reifier_rows
        );

        let annotation_total: u64 = counts.annotation.iter().sum();
        assert_eq!(
            annotation_total, annotation_row_total,
            "page_summary: annotation occurrence total {annotation_total} != annotation row \
             total {annotation_row_total}"
        );
        let graph_annotation_total: u64 = counts.graph_annotation.iter().sum();
        assert_eq!(
            counts.default_annotation_rows + graph_annotation_total,
            annotation_row_total,
            "page_summary: default_annotation_rows ({}) + graph_annotation_rows sum \
             ({graph_annotation_total}) != annotation row total {annotation_row_total}",
            counts.default_annotation_rows
        );

        Ok(counts)
    }

    /// Mix EVERY count into one `u64`, in a fixed field order, each field preceded by
    /// its tag and its length.
    ///
    /// Nothing the summary seals is left out: the graph list, all five per-term
    /// vectors, all three per-graph vectors, and the three default-graph scalars. An
    /// under-count anywhere — one fewer row attributed to a term, to a named graph, or
    /// to the default graph — changes the word at that position, and every step of the
    /// mixer is injective, so the finished digest changes with it. That is what makes
    /// the `O(1)` digest comparison on the admission path a real certification of the
    /// direction (under-reporting) that authorizes skipping rows.
    fn digest(&self) -> u64 {
        let mut digest = Digest::new();
        digest.write(tag::GRAPHS);
        digest.write(self.graphs.len() as u64);
        for graph in &self.graphs {
            digest.write(graph.index() as u64);
        }
        digest.write_field(tag::SUBJECT, &self.subject);
        digest.write_field(tag::PREDICATE, &self.predicate);
        digest.write_field(tag::OBJECT, &self.object);
        digest.write_field(tag::REIFIER, &self.reifier);
        digest.write_field(tag::ANNOTATION, &self.annotation);
        digest.write_field(tag::GRAPH_BASE, &self.graph_base);
        digest.write_field(tag::GRAPH_REIFIER, &self.graph_reifier);
        digest.write_field(tag::GRAPH_ANNOTATION, &self.graph_annotation);
        digest.write_field(
            tag::DEFAULT_ROWS,
            &[
                self.default_base_rows,
                self.default_reifier_rows,
                self.default_annotation_rows,
            ],
        );
        digest.finish()
    }
}

/// Increment `scratch[term.index()]`, refusing with a [`SummaryDefect`] when the row
/// names a term outside the page's own dense term table. Structurally unreachable for
/// a builder-frozen [`RdfDataset`]; typed rather than panicking because the input is
/// provider-supplied.
#[inline]
fn bump_term(scratch: &mut [u64], term: TermId, position: &str) -> Result<(), SummaryDefect> {
    let index = term.index();
    let Some(slot) = scratch.get_mut(index) else {
        return Err(SummaryDefect {
            message: format!(
                "page_summary: a {position} names local term index {index}, which is outside the \
                 page's own term table of {} terms",
                scratch.len()
            ),
        });
    };
    *slot += 1;
    Ok(())
}

/// Locate `g` in the page's own ascending named-graph list, refusing with a
/// [`SummaryDefect`] when it is absent.
///
/// Structurally unreachable for a builder-frozen [`RdfDataset`] — a frozen dataset's
/// graph slots are all declared graphs — but this runs on provider-supplied content
/// inside [`PagedDataset::verify_parts`](super::PagedDataset::verify_parts), the one
/// entry point whose purpose is certifying input it does not trust, so it refuses with
/// a typed error instead of aborting the process.
#[inline]
fn graph_index(graphs: &[TermId], g: TermId) -> Result<usize, SummaryDefect> {
    graphs.binary_search(&g).map_err(|_| SummaryDefect {
        message: format!(
            "page_summary: quad graph {g:?} is not in the page's own \
             RdfDataset::named_graphs() list — a frozen dataset's graph slots must all be known \
             graphs"
        ),
    })
}

/// A frozen page's exact base-quad, reifier-row, and annotation-row occurrence
/// counts, keyed by the page's own LOCAL [`TermId`] space — never the shared
/// [`GlobalTermId`](crate::ir::GlobalTermId) space.
///
/// # Enumeration and pruning are different questions
///
/// This type deliberately does NOT expose its `graphs` array directly. A caller
/// asking "does this page carry rows for named graph G" (pruning, via
/// [`graph_rows`](Self::graph_rows)/[`declared_graphs`](Self::declared_graphs)'s
/// binary search) must not be handed the same array to answer "what named graphs
/// does this page know about" in a way that lets it silently treat "zero rows" as
/// "graph absent" — a page's `graphs` list includes graphs the caller explicitly
/// declared empty (see [`RdfDataset::named_graphs`]), and collapsing that
/// distinction would drop every declared-empty graph from an enumeration built on
/// top of the pruning accessor. Keeping the two questions behind distinct,
/// narrowly-typed accessors makes that collapse a compile error instead of a
/// silent bug.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PageSummary {
    /// The page's named graphs, ascending — exactly `RdfDataset::named_graphs()`
    /// (already sorted and deduplicated; includes declared-empty graphs and graphs
    /// mentioned only by a side table). Parallel-indexed by `graph_base_rows`,
    /// `graph_reifier_rows`, and `graph_annotation_rows`.
    graphs: Box<[TermId]>,
    /// Exact base-quad occurrence count per local term id, subject position.
    /// Indexed by [`TermId::index`], length == the page's `term_count()`.
    subject: IntVector,
    /// Exact base-quad occurrence count per local term id, predicate position.
    predicate: IntVector,
    /// Exact base-quad occurrence count per local term id, object position.
    object: IntVector,
    /// Exact occurrence count per local term id in the reifier table's reifier
    /// column.
    reifier: IntVector,
    /// Exact occurrence count per local term id in the annotation table's reifier
    /// column.
    annotation: IntVector,
    /// Per graph (parallel to `graphs`): base-quad row count in that graph.
    graph_base_rows: IntVector,
    /// Per graph (parallel to `graphs`): reifier-table row count in that graph.
    graph_reifier_rows: IntVector,
    /// Per graph (parallel to `graphs`): annotation-table row count in that graph.
    graph_annotation_rows: IntVector,
    /// Base-quad row count whose `g` is `None` (the default graph).
    default_base_rows: u64,
    /// Reifier-table row count whose `g` is `None`.
    default_reifier_rows: u64,
    /// Annotation-table row count whose `g` is `None`.
    default_annotation_rows: u64,
    /// The `O(1)` certification handle: one deterministic `u64` mixed over EVERY
    /// field above at seal time (see [`PageCounts::digest`] and the
    /// [module docs](self)). Derived, so it never disagrees with the fields it
    /// summarizes; carried so the admission path can certify a freshly materialized
    /// page without re-deriving and re-comparing the whole summary.
    digest: u64,
}

/// Read `vector.get(term.index())`, or `0` if `term`'s index falls outside the
/// vector — an out-of-range `TermId` is a term from another page and must never
/// panic in release (see the [module docs](self)).
#[inline]
fn count_at(vector: &IntVector, term: TermId) -> u64 {
    let index = term.index();
    if index < vector.len() {
        vector.get(index)
    } else {
        0
    }
}

/// Build a value-indexed [`IntVector`] from a scratch counter array: the width is
/// chosen once from the scratch array's own maximum (never a fixed/guessed width),
/// then every value is pushed in index order (`IntVector::push` is append-only).
fn build_int_vector(scratch: &[u64]) -> IntVector {
    let max = scratch.iter().copied().max().unwrap_or(0);
    let mut vector = IntVector::with_width(bits_for(max));
    for &value in scratch {
        vector.push(value);
    }
    vector
}

impl PageSummary {
    /// Seal `page`'s exact per-term and per-graph row counts — the ONLY producer of
    /// a `PageSummary`.
    ///
    /// One pass each over `page.quads()`, `page.reifier_quads()`,
    /// `page.annotation_quads()`, plus `page.named_graphs()`, then the `O(1)`
    /// [`digest`](Self::digest) over everything accumulated. Every count is
    /// self-reconciled against an independently-computed total (see
    /// [`PageCounts::accumulate`]).
    ///
    /// # Errors
    ///
    /// [`SummaryDefect`] if a row names a term or a named graph the page's own frozen
    /// tables do not contain.
    ///
    /// # Panics
    ///
    /// Panics (via `assert_eq!`) if the accumulated per-term or per-graph counts do
    /// not reconcile against `page`'s own totals — a broken-invariant bug in this
    /// crate's accumulation loop, unreachable for a validly frozen `page`.
    pub(crate) fn seal(page: &RdfDataset) -> Result<Self, SummaryDefect> {
        let counts = PageCounts::accumulate(page)?;
        let digest = counts.digest();
        Ok(Self {
            subject: build_int_vector(&counts.subject),
            predicate: build_int_vector(&counts.predicate),
            object: build_int_vector(&counts.object),
            reifier: build_int_vector(&counts.reifier),
            annotation: build_int_vector(&counts.annotation),
            graph_base_rows: build_int_vector(&counts.graph_base),
            graph_reifier_rows: build_int_vector(&counts.graph_reifier),
            graph_annotation_rows: build_int_vector(&counts.graph_annotation),
            graphs: counts.graphs,
            default_base_rows: counts.default_base_rows,
            default_reifier_rows: counts.default_reifier_rows,
            default_annotation_rows: counts.default_annotation_rows,
            digest,
        })
    }

    /// The digest `page`'s content seals to, WITHOUT building (or allocating) the
    /// bit-packed summary itself — the admission path's certification primitive.
    ///
    /// Computed by the same accumulator [`seal`](Self::seal) uses, so
    /// `PageSummary::digest_of(p)` and `PageSummary::seal(p).digest()` are the same
    /// number by construction. Costs the one counting pass a fresh page's counts
    /// inescapably require (there is no way to learn that a page holds MORE rows in a
    /// graph than its summary claims without counting that graph's rows), and nothing
    /// more: no `IntVector` packing, no summary allocation, no field-by-field compare.
    ///
    /// # Errors
    ///
    /// [`SummaryDefect`], exactly as [`seal`](Self::seal).
    pub(crate) fn digest_of(page: &RdfDataset) -> Result<u64, SummaryDefect> {
        Ok(PageCounts::accumulate(page)?.digest())
    }

    /// This summary's sealed digest. Comparing two of these is `O(1)` and is a
    /// complete check in BOTH drift directions — see the [module docs](self).
    #[must_use]
    #[inline]
    pub(crate) const fn digest(&self) -> u64 {
        self.digest
    }

    /// The name of the first field on which this summary disagrees with `other`, in
    /// the summary's own declaration order, or `None` when every field agrees.
    ///
    /// This is the diagnostic half of certification: the digest says THAT a page
    /// drifted, and this says WHERE. Used by the explicitly-paid
    /// [`PagedDataset::verify_parts`](super::PagedDataset::verify_parts) pass, which
    /// has already re-derived the whole summary and can afford the field-by-field
    /// walk.
    #[must_use]
    pub(crate) fn first_disagreeing_field(&self, other: &Self) -> Option<&'static str> {
        let checks: [(&'static str, bool); 12] = [
            ("graphs", self.graphs == other.graphs),
            ("subject", self.subject == other.subject),
            ("predicate", self.predicate == other.predicate),
            ("object", self.object == other.object),
            ("reifier", self.reifier == other.reifier),
            ("annotation", self.annotation == other.annotation),
            (
                "graph_base_rows",
                self.graph_base_rows == other.graph_base_rows,
            ),
            (
                "graph_reifier_rows",
                self.graph_reifier_rows == other.graph_reifier_rows,
            ),
            (
                "graph_annotation_rows",
                self.graph_annotation_rows == other.graph_annotation_rows,
            ),
            (
                "default_base_rows",
                self.default_base_rows == other.default_base_rows,
            ),
            (
                "default_reifier_rows",
                self.default_reifier_rows == other.default_reifier_rows,
            ),
            (
                "default_annotation_rows",
                self.default_annotation_rows == other.default_annotation_rows,
            ),
        ];
        checks
            .into_iter()
            .find_map(|(field, agrees)| (!agrees).then_some(field))
    }

    /// Exact base-quad occurrence count for `term` in the subject position. `0` if
    /// `term`'s index is out of range for this page (a term from another page).
    #[must_use]
    #[inline]
    pub(crate) fn base_rows_as_subject(&self, term: TermId) -> u64 {
        count_at(&self.subject, term)
    }

    /// Exact base-quad occurrence count for `term` in the predicate position. `0` if
    /// `term`'s index is out of range for this page.
    #[must_use]
    #[inline]
    pub(crate) fn base_rows_as_predicate(&self, term: TermId) -> u64 {
        count_at(&self.predicate, term)
    }

    /// Exact base-quad occurrence count for `term` in the object position. `0` if
    /// `term`'s index is out of range for this page.
    #[must_use]
    #[inline]
    pub(crate) fn base_rows_as_object(&self, term: TermId) -> u64 {
        count_at(&self.object, term)
    }

    /// Exact occurrence count for `term` in the reifier side-table's reifier
    /// column. `0` if `term`'s index is out of range for this page.
    #[must_use]
    #[inline]
    pub(crate) fn reifier_rows(&self, term: TermId) -> u64 {
        count_at(&self.reifier, term)
    }

    /// Exact occurrence count for `term` in the annotation side-table's reifier
    /// column. `0` if `term`'s index is out of range for this page.
    #[must_use]
    #[inline]
    pub(crate) fn annotation_rows(&self, term: TermId) -> u64 {
        count_at(&self.annotation, term)
    }

    /// The row count of `stream` in named graph `graph` on this page (`O(log n)`
    /// binary search of the page's own graph list). `0` if `graph` is not one of
    /// this page's named graphs.
    #[must_use]
    #[inline]
    pub(crate) fn graph_rows(&self, graph: TermId, stream: PageStream) -> u64 {
        let Ok(index) = self.graphs.binary_search(&graph) else {
            return 0;
        };
        let vector = match stream {
            PageStream::Base => &self.graph_base_rows,
            PageStream::Reifier => &self.graph_reifier_rows,
            PageStream::Annotation => &self.graph_annotation_rows,
        };
        vector.get(index)
    }

    /// The row count of `stream` whose `g` is `None` (the default graph) on this
    /// page.
    #[must_use]
    #[inline]
    pub(crate) fn default_rows(&self, stream: PageStream) -> u64 {
        match stream {
            PageStream::Base => self.default_base_rows,
            PageStream::Reifier => self.default_reifier_rows,
            PageStream::Annotation => self.default_annotation_rows,
        }
    }

    /// Every named graph this page knows about, ascending — exactly
    /// `RdfDataset::named_graphs()` at seal time (declared-empty graphs included).
    /// See the [type docs](Self) for why this is the ONLY enumeration surface: it
    /// is deliberately separate from [`graph_rows`](Self::graph_rows), which
    /// answers a pruning question over the same array.
    #[inline]
    pub(crate) fn declared_graphs(&self) -> impl Iterator<Item = TermId> + '_ {
        self.graphs.iter().copied()
    }
}

/// `IntVector` derives `Debug`/`Clone` but not `PartialEq`/`Eq` (it is a bit-packed
/// builder, not a value type with a canonical equality). `PageSummary` needs a real
/// equality for its own test, so this compares each `IntVector` field through its
/// deterministic `to_bytes()` encoding — two vectors seal to the same bytes iff they
/// hold the same values at the same width, which is exactly the equality a caller of
/// `seal` expects.
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{PageCounts, PageStream, PageSummary};
    use crate::ir::{RdfDataset, RdfDatasetBuilder, TermId};

    /// Build ONE fixture exercising every shape `PageSummary::seal` must handle, then
    /// assert every accessor against an INDEPENDENTLY hand-computed expected value
    /// (never recomputed via the same accumulation loop the implementation uses).
    ///
    /// Fixture (all in `http://example.org/` unless noted):
    /// - `:alice :knows :bob` — default graph. `:alice` subject-only, `:bob`
    ///   object-only (in this quad), `:knows` predicate-only.
    /// - `:bob :knows :carol` — named graph `:g1`. `:bob` now also a subject (two
    ///   positions across the fixture), `:carol` object-only, `:knows` predicate again.
    /// - `:g2` declared named but empty (`declare_named_graph`, no rows at all).
    /// - A quoted triple `<<( :alice :knows :bob )>>`, used as:
    ///   - the OBJECT of `:dave :claims <<(...)>>` (default graph) — the ordinary,
    ///     structural object-position use every base quad allows; and
    ///   - the reification TARGET of `:r1 rdf:reifies <<(...)>>` (named graph `:g1`)
    ///     — the RDF 1.2 sense in which a quoted triple is "the subject of an
    ///     annotation": semantically the statement being annotated, even though
    ///     `RdfDataset`'s frozen IR (see `validate::require_asserted_subject`) never
    ///     permits a triple TERM in a literal subject slot anywhere (quad subject,
    ///     reifier, or annotation reifier must be an IRI or blank node) — that
    ///     restriction is RDF 1.2 spec-correct, not an over-refusal, so this fixture
    ///     does not attempt it.
    /// - A reifier row: `:r1 rdf:reifies <<( :alice :knows :bob )>>` in `:g1`.
    /// - An annotation row: `:r1 :certainty "0.9"` in `:g1`.
    #[test]
    fn page_summary_matches_an_independent_recount() {
        let mut b = RdfDatasetBuilder::new();
        let alice = b.intern_iri("http://example.org/alice");
        let bob = b.intern_iri("http://example.org/bob");
        let carol = b.intern_iri("http://example.org/carol");
        let dave = b.intern_iri("http://example.org/dave");
        let knows = b.intern_iri("http://example.org/knows");
        let claims = b.intern_iri("http://example.org/claims");
        let certainty = b.intern_iri("http://example.org/certainty");
        let r1 = b.intern_iri("http://example.org/r1");
        let g1 = b.intern_iri("http://example.org/g1");
        let g2 = b.intern_iri("http://example.org/g2");
        let point_nine = b.intern_literal(crate::RdfLiteral {
            lexical_form: "0.9".to_owned(),
            datatype: None,
            language: None,
            direction: None,
        });

        // A term appearing in subject-only, predicate-only, and object-only positions,
        // plus a term (`bob`) in two different positions.
        b.push_quad(alice, knows, bob, None); // default graph
        b.push_quad(bob, knows, carol, Some(g1)); // named graph g1

        // Declared-empty named graph.
        b.declare_named_graph(g2);

        // A quoted triple used as the object of a base quad.
        let alice_knows_bob = b.intern_triple(alice, knows, bob);
        b.push_quad(dave, claims, alice_knows_bob, None); // default graph

        // A reifier row (the triple's reification target) and an annotation row,
        // both in g1.
        b.push_reifier_in_graph(r1, alice_knows_bob, Some(g1));
        b.push_annotation_in_graph(r1, certainty, point_nine, Some(g1));

        let page = b.freeze().expect("fixture freezes");
        let summary = PageSummary::seal(&page).expect("fixture seals");

        // -- base-quad per-term occurrence counts -------------------------------
        // Base quads: (alice knows bob . ), (bob knows carol g1), (dave claims <<a k b>> . )
        assert_eq!(summary.base_rows_as_subject(alice), 1); // alice knows bob
        assert_eq!(summary.base_rows_as_subject(bob), 1); // bob knows carol
        assert_eq!(summary.base_rows_as_subject(dave), 1); // dave claims <<..>>
        assert_eq!(summary.base_rows_as_subject(alice_knows_bob), 0); // never a subject
        assert_eq!(summary.base_rows_as_subject(carol), 0);
        assert_eq!(summary.base_rows_as_subject(knows), 0);

        assert_eq!(summary.base_rows_as_predicate(knows), 2); // alice-knows-bob, bob-knows-carol
        assert_eq!(summary.base_rows_as_predicate(claims), 1);
        assert_eq!(summary.base_rows_as_predicate(alice), 0);

        assert_eq!(summary.base_rows_as_object(bob), 1); // alice knows bob
        assert_eq!(summary.base_rows_as_object(carol), 1); // bob knows carol
        assert_eq!(summary.base_rows_as_object(alice_knows_bob), 1); // dave claims <<..>>
        assert_eq!(summary.base_rows_as_object(alice), 0);

        // -- reifier / annotation per-term occurrence counts --------------------
        assert_eq!(summary.reifier_rows(r1), 1);
        assert_eq!(summary.reifier_rows(alice), 0);
        assert_eq!(summary.annotation_rows(r1), 1);
        assert_eq!(summary.annotation_rows(bob), 0);

        // -- declared graphs (sorted TermId order: whichever of g1/g2 interned first) --
        let mut expected_graphs = [g1, g2];
        expected_graphs.sort();
        let declared: Vec<TermId> = summary.declared_graphs().collect();
        assert_eq!(declared, expected_graphs);

        // -- per-graph and default row counts, all three streams ----------------
        // Base: 1 row in g1 (bob-knows-carol), 2 in default (alice-knows-bob,
        // dave-claims-<<..>>), 0 in g2.
        assert_eq!(summary.graph_rows(g1, PageStream::Base), 1);
        assert_eq!(summary.graph_rows(g2, PageStream::Base), 0);
        assert_eq!(summary.default_rows(PageStream::Base), 2);

        // Reifier: 1 row in g1, 0 in default, 0 in g2.
        assert_eq!(summary.graph_rows(g1, PageStream::Reifier), 1);
        assert_eq!(summary.graph_rows(g2, PageStream::Reifier), 0);
        assert_eq!(summary.default_rows(PageStream::Reifier), 0);

        // Annotation: 1 row in g1, 0 in default, 0 in g2.
        assert_eq!(summary.graph_rows(g1, PageStream::Annotation), 1);
        assert_eq!(summary.graph_rows(g2, PageStream::Annotation), 0);
        assert_eq!(summary.default_rows(PageStream::Annotation), 0);

        // -- out-of-range TermId never panics, always reads 0 --------------------
        let bogus = TermId::from_index(u32::try_from(page.term_count()).expect("fits u32") + 1000);
        assert_eq!(summary.base_rows_as_subject(bogus), 0);
        assert_eq!(summary.base_rows_as_predicate(bogus), 0);
        assert_eq!(summary.base_rows_as_object(bogus), 0);
        assert_eq!(summary.reifier_rows(bogus), 0);
        assert_eq!(summary.annotation_rows(bogus), 0);
        assert_eq!(summary.graph_rows(bogus, PageStream::Base), 0);
    }

    /// `alice knows bob` and `bob knows carol`, with the SECOND quad landing in `g1`
    /// or in `g2` depending on `second_in_g2`. Both variants intern the same terms and
    /// carry the same two quads; only the per-graph row split differs — `(2, 0)`
    /// versus `(1, 1)` — which is exactly the drift shape a page's sealed summary
    /// authorizes skipping on.
    fn graph_split_page(second_in_g2: bool) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let alice = b.intern_iri("http://example.org/alice");
        let bob = b.intern_iri("http://example.org/bob");
        let carol = b.intern_iri("http://example.org/carol");
        let knows = b.intern_iri("http://example.org/knows");
        let g1 = b.intern_iri("http://example.org/g1");
        let g2 = b.intern_iri("http://example.org/g2");
        b.push_quad(alice, knows, bob, Some(g1));
        b.push_quad(bob, knows, carol, Some(if second_in_g2 { g2 } else { g1 }));
        b.declare_named_graph(g2);
        b.freeze().expect("fixture freezes")
    }

    /// The digest must be a pure function of the page's content: sealing the SAME page
    /// twice yields the same number, and `digest_of` — the admission path's cheaper
    /// primitive, which never builds a summary — yields that same number too. If these
    /// ever disagreed, every admission would refuse every honest page.
    #[test]
    fn sealing_the_same_page_twice_yields_the_same_digest() {
        let page = graph_split_page(false);

        let first = PageSummary::seal(&page).expect("fixture seals");
        let second = PageSummary::seal(&page).expect("fixture seals again");
        assert_eq!(
            first.digest(),
            second.digest(),
            "the digest is a pure function of page content"
        );
        assert_eq!(first, second, "so is the whole summary");

        assert_eq!(
            PageSummary::digest_of(&page).expect("fixture digests"),
            first.digest(),
            "digest_of and seal().digest() are the same number by construction"
        );
    }

    /// The drift the graph-pruning index is built from: moving one row from `g1` to
    /// `g2` leaves term count, quad count and capabilities identical — every cheap
    /// admission check passes — and must still change the digest, because the
    /// per-graph split is one of the fields mixed in.
    #[test]
    fn moving_one_row_between_graphs_changes_the_digest() {
        let honest = PageSummary::seal(&graph_split_page(false)).expect("honest seals");
        let drifted = PageSummary::seal(&graph_split_page(true)).expect("drifted seals");
        assert_ne!(
            honest.digest(),
            drifted.digest(),
            "a per-graph row split of (2, 0) must not digest the same as (1, 1)"
        );
        assert_eq!(
            honest.first_disagreeing_field(&drifted),
            Some("graph_base_rows"),
            "and the paid pass names the field that moved"
        );
    }

    /// Every UNDER-count the summary can express — one fewer row attributed to a term
    /// in any of the five per-term columns, to a named graph in any of the three
    /// per-graph columns, or to the default graph in any of the three scalars — must
    /// change the digest. This is the direction that authorizes skipping a page which
    /// actually holds matching rows, so it is checked field by field rather than
    /// sampled.
    #[test]
    fn every_under_count_changes_the_digest() {
        let mut b = RdfDatasetBuilder::new();
        let alice = b.intern_iri("http://example.org/alice");
        let bob = b.intern_iri("http://example.org/bob");
        let knows = b.intern_iri("http://example.org/knows");
        let certainty = b.intern_iri("http://example.org/certainty");
        let r1 = b.intern_iri("http://example.org/r1");
        let g1 = b.intern_iri("http://example.org/g1");
        let nine = b.intern_literal(crate::RdfLiteral {
            lexical_form: "0.9".to_owned(),
            datatype: None,
            language: None,
            direction: None,
        });
        let alice_knows_bob = b.intern_triple(alice, knows, bob);
        b.push_quad(alice, knows, bob, None);
        b.push_quad(alice, knows, bob, Some(g1));
        b.push_reifier_in_graph(r1, alice_knows_bob, None);
        b.push_reifier_in_graph(r1, alice_knows_bob, Some(g1));
        b.push_annotation_in_graph(r1, certainty, nine, None);
        b.push_annotation_in_graph(r1, certainty, nine, Some(g1));
        let page = b.freeze().expect("fixture freezes");

        let fresh = || PageCounts::accumulate(&page).expect("fixture accumulates");
        let sealed = fresh().digest();

        // The fixture really does hold a count of two everywhere a mutation below
        // decrements, so none of them silently wraps a zero into `u64::MAX` and
        // "changes the digest" for the wrong reason.
        let counts = fresh();
        assert_eq!(counts.subject[alice.index()], 2);
        assert_eq!(counts.predicate[knows.index()], 2);
        assert_eq!(counts.object[bob.index()], 2);
        assert_eq!(counts.reifier[r1.index()], 2);
        assert_eq!(counts.annotation[r1.index()], 2);
        assert_eq!(counts.graph_base, vec![1u64]);
        assert_eq!(counts.graph_reifier, vec![1u64]);
        assert_eq!(counts.graph_annotation, vec![1u64]);
        assert_eq!(counts.default_base_rows, 1);
        assert_eq!(counts.default_reifier_rows, 1);
        assert_eq!(counts.default_annotation_rows, 1);

        // Each mutation drops exactly ONE row from exactly one field, leaving every
        // other word identical — the single-word case the mixer's bijectivity makes a
        // guarantee rather than a probability.
        let check = |field: &str, mutate: &dyn Fn(&mut PageCounts)| {
            let mut counts = fresh();
            mutate(&mut counts);
            assert_ne!(
                counts.digest(),
                sealed,
                "under-reporting {field} must change the digest"
            );
        };
        check("subject", &|c| c.subject[alice.index()] -= 1);
        check("predicate", &|c| c.predicate[knows.index()] -= 1);
        check("object", &|c| c.object[bob.index()] -= 1);
        check("reifier", &|c| c.reifier[r1.index()] -= 1);
        check("annotation", &|c| c.annotation[r1.index()] -= 1);
        check("graph_base", &|c| c.graph_base[0] -= 1);
        check("graph_reifier", &|c| c.graph_reifier[0] -= 1);
        check("graph_annotation", &|c| c.graph_annotation[0] -= 1);
        check("default_base_rows", &|c| c.default_base_rows -= 1);
        check("default_reifier_rows", &|c| c.default_reifier_rows -= 1);
        check("default_annotation_rows", &|c| {
            c.default_annotation_rows -= 1;
        });
        // A dropped named graph: the graph list shortens and the three per-graph
        // vectors shorten with it, which the length words catch.
        check("graphs", &|c| {
            c.graphs = Box::default();
            c.graph_base = Vec::new();
            c.graph_reifier = Vec::new();
            c.graph_annotation = Vec::new();
        });
    }
}
