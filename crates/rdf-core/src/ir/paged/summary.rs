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
    /// `page.annotation_quads()`, plus `page.named_graphs()`. Every count is
    /// self-reconciled against an independently-computed total below (a mismatch is
    /// a hard `assert_eq!` failure at the producer, never a silently wrong summary
    /// surfacing later).
    ///
    /// # Panics
    ///
    /// Panics (via `assert_eq!`) if the accumulated per-term or per-graph counts do
    /// not reconcile against `page`'s own totals — a broken-invariant bug in this
    /// function, unreachable for a validly frozen `page`.
    #[must_use]
    pub(crate) fn seal(page: &RdfDataset) -> Self {
        let term_count = page.term_count();
        let graphs: Box<[TermId]> = page.named_graphs().collect();

        let mut subject_scratch = vec![0u64; term_count];
        let mut predicate_scratch = vec![0u64; term_count];
        let mut object_scratch = vec![0u64; term_count];
        let mut reifier_scratch = vec![0u64; term_count];
        let mut annotation_scratch = vec![0u64; term_count];

        let mut graph_base_scratch = vec![0u64; graphs.len()];
        let mut graph_reifier_scratch = vec![0u64; graphs.len()];
        let mut graph_annotation_scratch = vec![0u64; graphs.len()];

        let mut default_base_rows = 0u64;
        let mut default_reifier_rows = 0u64;
        let mut default_annotation_rows = 0u64;

        let graph_index = |g: TermId| -> usize {
            graphs.binary_search(&g).unwrap_or_else(|_| {
                panic!(
                    "page_summary: quad graph {g:?} is not in the page's own \
                     RdfDataset::named_graphs() list — a frozen dataset's graph slots \
                     must all be known graphs"
                )
            })
        };

        let mut reifier_row_total = 0u64;
        let mut annotation_row_total = 0u64;

        for q in page.quads() {
            subject_scratch[q.s.index()] += 1;
            predicate_scratch[q.p.index()] += 1;
            object_scratch[q.o.index()] += 1;
            match q.g {
                None => default_base_rows += 1,
                Some(g) => graph_base_scratch[graph_index(g)] += 1,
            }
        }

        for q in page.reifier_quads() {
            reifier_row_total += 1;
            reifier_scratch[q.s.index()] += 1;
            match q.g {
                None => default_reifier_rows += 1,
                Some(g) => graph_reifier_scratch[graph_index(g)] += 1,
            }
        }

        for q in page.annotation_quads() {
            annotation_row_total += 1;
            annotation_scratch[q.s.index()] += 1;
            match q.g {
                None => default_annotation_rows += 1,
                Some(g) => graph_annotation_scratch[graph_index(g)] += 1,
            }
        }

        // Self-reconciliation: sum of subject/predicate/object counts must each equal
        // the page's own quad count, and the default+per-graph row split must sum to
        // the same total, for all three streams. A mismatch is a broken accumulation
        // bug, made a hard failure here rather than a wrong answer surfacing later.
        let quad_count = page.quad_count() as u64;
        let subject_total: u64 = subject_scratch.iter().sum();
        let predicate_total: u64 = predicate_scratch.iter().sum();
        let object_total: u64 = object_scratch.iter().sum();
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
        let graph_base_total: u64 = graph_base_scratch.iter().sum();
        assert_eq!(
            default_base_rows + graph_base_total,
            quad_count,
            "page_summary: default_base_rows ({default_base_rows}) + graph_base_rows sum \
             ({graph_base_total}) != page quad count {quad_count}"
        );

        let reifier_total: u64 = reifier_scratch.iter().sum();
        assert_eq!(
            reifier_total, reifier_row_total,
            "page_summary: reifier occurrence total {reifier_total} != reifier row total \
             {reifier_row_total}"
        );
        let graph_reifier_total: u64 = graph_reifier_scratch.iter().sum();
        assert_eq!(
            default_reifier_rows + graph_reifier_total,
            reifier_row_total,
            "page_summary: default_reifier_rows ({default_reifier_rows}) + graph_reifier_rows \
             sum ({graph_reifier_total}) != reifier row total {reifier_row_total}"
        );

        let annotation_total: u64 = annotation_scratch.iter().sum();
        assert_eq!(
            annotation_total, annotation_row_total,
            "page_summary: annotation occurrence total {annotation_total} != annotation row \
             total {annotation_row_total}"
        );
        let graph_annotation_total: u64 = graph_annotation_scratch.iter().sum();
        assert_eq!(
            default_annotation_rows + graph_annotation_total,
            annotation_row_total,
            "page_summary: default_annotation_rows ({default_annotation_rows}) + \
             graph_annotation_rows sum ({graph_annotation_total}) != annotation row total \
             {annotation_row_total}"
        );

        Self {
            graphs,
            subject: build_int_vector(&subject_scratch),
            predicate: build_int_vector(&predicate_scratch),
            object: build_int_vector(&object_scratch),
            reifier: build_int_vector(&reifier_scratch),
            annotation: build_int_vector(&annotation_scratch),
            graph_base_rows: build_int_vector(&graph_base_scratch),
            graph_reifier_rows: build_int_vector(&graph_reifier_scratch),
            graph_annotation_rows: build_int_vector(&graph_annotation_scratch),
            default_base_rows,
            default_reifier_rows,
            default_annotation_rows,
        }
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
    use super::{PageStream, PageSummary};
    use crate::ir::{RdfDatasetBuilder, TermId};

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
        let summary = PageSummary::seal(&page);

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
}
