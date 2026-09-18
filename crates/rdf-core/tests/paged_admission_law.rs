// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A differential proptest proving the page-admission law is a SOUND
//! filter — `PagedDataset::quads_for_pattern` and `PagedQueryView::quads_for_pattern`
//! must equal a per-page reference, row-for-row and order-for-order, not merely as
//! sets — plus a parity/determinism suite and a pinned-literal whole-dataset
//! acceptance test.
//!
//! # Why row order, not just the row set
//!
//! A set-only comparison can hide an over-refusal that happens to preserve the SET
//! while reordering it (for example, admitting the right pages but in the wrong
//! order relative to a caller depending on evaluation order for evidence). Egress
//! order is a contract here — [`PagedQueryEvidence::requested_pages`] is
//! evaluation-order evidence a G-clause treats as proof of what a query actually
//! touched — so the comparisons in `mod fuzz` below are `Vec` equality, not
//! `BTreeSet` equality.
//!
//! # Why the reference is per-page, not `paged.quads().filter(...)`
//!
//! The naive trait-default reference (`self.quads().filter(...)`) is NOT a valid
//! order oracle here: `RdfDataset::quads_for_pattern` (which every page's own
//! indexed lookup is) is ITSELF an override of that default, dispatching to one of
//! six sort-key permutations depending on which axes are bound, and returning rows
//! in THAT permutation's order rather than `quads()`'s SPO-sorted order whenever the
//! candidate run is selective — a confirmed, pre-existing property of a single
//! non-paged `RdfDataset`, unrelated to paging (see `mod fuzz`'s
//! `assert_pattern_matches_reference` doc for the reproduction and the mechanism in
//! `RdfDataset::candidate_access`). The repo's own `ir/dataset.rs` template
//! (`proptest_indexed_pattern_matches_linear_scan`) compares via `BTreeSet` for
//! exactly this reason. The reference used below is instead built by visiting each
//! page in the SAME order `PagedDataset` does and calling that page's OWN
//! `quads_for_pattern` — isolating what THIS task is actually about (did the
//! admission law pick the right pages, in the right order, with correct id
//! translation) from `RdfDataset`'s own index-permutation choice, which is already
//! covered elsewhere and is not a page-admission concern.

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf_core::{
    DatasetView, FallibleDatasetView, GlobalTermId, GraphMatch, InMemoryPageProvider, PageId,
    PageProvider, PagedDataset, PagedQueryError, PagedQueryEvidence, PagedQueryLimits, QuadIds,
    RdfDataset, RdfDatasetBuilder, TermRef, TermValue, ViewOperationStatus,
};

/// An `example.org` IRI value.
fn iri(name: &str) -> TermValue {
    TermValue::iri(format!("http://example.org/{name}"))
}

/// Resolve a view id to its dataset-INDEPENDENT `TermValue`, recursing through the
/// literal datatype and triple components.
fn to_value<V: DatasetView>(v: &V, id: V::Id) -> TermValue {
    match v.resolve(id) {
        TermRef::Iri(s) => TermValue::iri(s),
        TermRef::Blank { label, scope } => TermValue::Blank {
            label: label.to_owned(),
            scope,
        },
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            let datatype = match v.resolve(datatype) {
                TermRef::Iri(s) => s.to_owned(),
                other => panic!("literal datatype must resolve to an IRI, got {other:?}"),
            };
            TermValue::Literal {
                lexical_form: lexical.to_owned(),
                datatype,
                language: language.map(str::to_owned),
                direction,
            }
        }
        TermRef::Triple { s, p, o } => TermValue::Triple {
            s: Box::new(to_value(v, s)),
            p: Box::new(to_value(v, p)),
            o: Box::new(to_value(v, o)),
        },
    }
}

fn ready_evidence(
    status: ViewOperationStatus<PagedQueryError, PagedQueryEvidence>,
) -> PagedQueryEvidence {
    match status {
        ViewOperationStatus::Ready { evidence } => evidence,
        ViewOperationStatus::Failed { error, .. } => {
            panic!("expected a ready operation, got: {error}")
        }
    }
}

/// One `(subject predicate object)` triple in the default graph, as its own page.
fn build_simple_page(subject: &str, predicate: &str, object: &str) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&format!("http://example.org/{subject}"));
    let p = b.intern_iri(&format!("http://example.org/{predicate}"));
    let o = b.intern_iri(&format!("http://example.org/{object}"));
    b.push_quad(s, p, o, None);
    b.freeze().expect("page freeze")
}

// ── 9b.1: the differential proptest ─────────────────────────────────────────────

mod fuzz {
    use super::{
        Arc, DatasetView, GlobalTermId, GraphMatch, InMemoryPageProvider, PageId, PageProvider,
        PagedDataset, PagedQueryLimits, QuadIds, RdfDataset, RdfDatasetBuilder, TermValue,
    };
    use proptest::prelude::*;

    const POOL_LEN: u8 = 5;
    const GRAPH_LEN: u8 = 3;
    /// The sentinel `o` selector meaning "use the quoted triple `<<(n0 n1 n2)>>`"
    /// rather than an ordinary pool term.
    const TRIPLE_SENTINEL: u8 = POOL_LEN;

    fn pool_value(i: u8) -> TermValue {
        TermValue::iri(format!("http://example.org/n{i}"))
    }

    fn graph_value(i: u8) -> TermValue {
        TermValue::iri(format!("http://example.org/g{i}"))
    }

    fn triple_value() -> TermValue {
        TermValue::Triple {
            s: Box::new(pool_value(0)),
            p: Box::new(pool_value(1)),
            o: Box::new(pool_value(2)),
        }
    }

    /// The deterministic page a `(s, p, o_choice, g)` row is assigned to: a pure
    /// function of the row's own components, so the SAME row always lands on the
    /// SAME page regardless of how many times it is generated — which is what makes
    /// the resulting pages quad-disjoint (G3) by construction, with no need to
    /// deduplicate rows across pages by hand.
    fn row_page(s: u8, p: u8, o_choice: u8, g: Option<u8>, page_count: usize) -> usize {
        let g_component = g.map_or(97usize, |gi| usize::from(gi) + 1);
        (usize::from(s) + usize::from(p) + usize::from(o_choice) + g_component) % page_count
    }

    /// Build `page_count` quad-disjoint pages from `rows`, each page interning the
    /// FULL shared term pool (so every pool/graph/triple term is resolvable by value
    /// regardless of whether a row on that particular page used it), plus a
    /// deliberately declared-empty graph, a reifier row in a graph with NO base
    /// quad (a side-table-only graph), and at least one quoted-triple object.
    fn build_pages(
        rows: &[(u8, u8, u8, Option<u8>)],
        page_count: usize,
        empty_graph_page: usize,
        side_graph_page: usize,
    ) -> Vec<Arc<RdfDataset>> {
        let mut builders: Vec<RdfDatasetBuilder> =
            (0..page_count).map(|_| RdfDatasetBuilder::new()).collect();

        // Intern the shared pool/graphs/triple/side terms on EVERY page, in the same
        // order, so they are all resolvable by value on every page — including a
        // page that never uses one of them in a row.
        let mut pools = Vec::with_capacity(page_count);
        let mut graphs = Vec::with_capacity(page_count);
        let mut triples = Vec::with_capacity(page_count);
        let mut g_empty_ids = Vec::with_capacity(page_count);
        let mut g_side_ids = Vec::with_capacity(page_count);
        let mut r_side_ids = Vec::with_capacity(page_count);
        for b in &mut builders {
            let pool: Vec<_> = (0..POOL_LEN)
                .map(|i| b.intern_iri(&format!("http://example.org/n{i}")))
                .collect();
            let gs: Vec<_> = (0..GRAPH_LEN)
                .map(|i| b.intern_iri(&format!("http://example.org/g{i}")))
                .collect();
            let triple = b.intern_triple(pool[0], pool[1], pool[2]);
            let g_empty = b.intern_iri("http://example.org/gEmpty");
            let g_side = b.intern_iri("http://example.org/gSide");
            let r_side = b.intern_iri("http://example.org/rSide");
            pools.push(pool);
            graphs.push(gs);
            triples.push(triple);
            g_empty_ids.push(g_empty);
            g_side_ids.push(g_side);
            r_side_ids.push(r_side);
        }

        for &(s, p, o_choice, g) in rows {
            let page = row_page(s, p, o_choice, g, page_count);
            let pool = &pools[page];
            let gs = &graphs[page];
            let s_id = pool[s as usize];
            let p_id = pool[p as usize];
            let o_id = if o_choice == TRIPLE_SENTINEL {
                triples[page]
            } else {
                pool[o_choice as usize]
            };
            let g_id = g.map(|gi| gs[gi as usize]);
            builders[page].push_quad(s_id, p_id, o_id, g_id);
        }

        // Guaranteed declared-empty graph: this page knows `gEmpty` exists but owns
        // no row in it.
        builders[empty_graph_page].declare_named_graph(g_empty_ids[empty_graph_page]);

        // Guaranteed side-table-only graph: a reifier row in `gSide`, with NO base
        // quad ever naming that graph.
        builders[side_graph_page].push_reifier_in_graph(
            r_side_ids[side_graph_page],
            triples[side_graph_page],
            Some(g_side_ids[side_graph_page]),
        );

        // Guaranteed quoted-triple object: a base quad whose object is the triple
        // term, on the page the SAME deterministic row-page function assigns it to
        // (so it can never collide across pages with an identically-keyed random
        // row).
        let guaranteed_page = row_page(3, 4, TRIPLE_SENTINEL, None, page_count);
        {
            let pool = &pools[guaranteed_page];
            let triple = triples[guaranteed_page];
            builders[guaranteed_page].push_quad(pool[3], pool[4], triple, None);
        }

        builders
            .into_iter()
            .map(|b| b.freeze().expect("random disjoint page must freeze"))
            .collect()
    }

    /// Resolve the `(s_sel, p_sel, o_sel, g_sel)` selectors against `paged` AFRESH —
    /// never reused across dataset variants, because `compact()` legitimately
    /// renumbers `GlobalTermId`s (the pool/graph/triple/side term VALUES survive
    /// every variant below; their ids do not).
    fn resolve_pattern(
        paged: &PagedDataset,
        s_sel: Option<u8>,
        p_sel: Option<u8>,
        o_sel: Option<u8>,
        g_sel: u8,
    ) -> (
        Option<GlobalTermId>,
        Option<GlobalTermId>,
        Option<GlobalTermId>,
        GraphMatch<GlobalTermId>,
    ) {
        let s = s_sel.map(|i| {
            paged
                .term_id_by_value(&pool_value(i))
                .expect("pool term always interned")
        });
        let p = p_sel.map(|i| {
            paged
                .term_id_by_value(&pool_value(i))
                .expect("pool term always interned")
        });
        let o = o_sel.map(|i| {
            let value = if i == TRIPLE_SENTINEL {
                triple_value()
            } else {
                pool_value(i)
            };
            paged
                .term_id_by_value(&value)
                .expect("pool/triple term always interned")
        });
        let g = match g_sel {
            0 => GraphMatch::Any,
            1 => GraphMatch::Default,
            n => GraphMatch::Named(
                paged
                    .term_id_by_value(&graph_value(n - 2))
                    .expect("graph term always interned"),
            ),
        };
        (s, p, o, g)
    }

    /// Assert that BOTH `PagedDataset::quads_for_pattern` and
    /// `PagedQueryView::quads_for_pattern` equal a PER-PAGE reference built by
    /// visiting `variant`'s pages in ascending order and, for each, calling that
    /// page's OWN (already independently, exhaustively tested elsewhere — see
    /// `proptest_indexed_pattern_matches_linear_scan` in `ir/dataset.rs`)
    /// `DatasetView::quads_for_pattern` on `raw_pages[page_map[i]]`, the ORIGINAL
    /// frozen page `variant`'s `PageId(i)` addresses, translating the GLOBAL pattern
    /// down to that page's LOCAL id space first (a page absent a bound term
    /// contributes nothing, exactly mirroring [`admit_pattern`]'s soundness claim).
    ///
    /// # Why not `variant.quads().filter(...)` (the naive trait-default reference)
    ///
    /// `RdfDataset::quads_for_pattern` is ITSELF an override of the `DatasetView`
    /// trait default (`self.quads().filter(...)`): it dispatches to whichever of six
    /// sort-key permutations (SPOG/POS/OSP/GSPO/GPOS/GOSP) has the longest bound-axis
    /// prefix, and returns rows in THAT permutation's order whenever the candidate
    /// run is selective enough to use it (see `RdfDataset::candidate_access` in
    /// `ir/dataset.rs`) — which does NOT, in general, match the SPO-sorted order
    /// `quads()` itself yields, even for a single non-paged dataset with no paging
    /// involved at all. This is confirmed, pre-existing, unrelated-to-paging
    /// behavior (the repo's own `proptest_indexed_pattern_matches_linear_scan`
    /// template compares by `BTreeSet`, not `Vec`, for exactly this reason). Because
    /// each admitted page's row order is ultimately produced by ITS OWN
    /// `quads_for_pattern`, the only reference that can honestly claim row-for-row
    /// AND order-for-order equality is one built from that same per-page mechanism —
    /// which is what this function does, isolating what this task IS about (did the
    /// admission law pick the right pages, in the right order, with correct id
    /// translation) from what it is NOT about (`RdfDataset`'s own index-permutation
    /// choice, already covered elsewhere).
    fn assert_pattern_matches_reference(
        variant: &PagedDataset,
        raw_pages: &[Arc<RdfDataset>],
        page_map: &[usize],
        s_sel: Option<u8>,
        p_sel: Option<u8>,
        o_sel: Option<u8>,
        g_sel: u8,
    ) {
        let (s, p, o, g) = resolve_pattern(variant, s_sel, p_sel, o_sel, g_sel);

        let mut expected: Vec<QuadIds<GlobalTermId>> = Vec::new();
        for i in 0..variant.page_count() {
            let page_id = PageId(u32::try_from(i).expect("page count fits u32"));
            let translation = variant.translation(page_id).expect("page id in range");

            let local_s = match s.map(|id| translation.to_local(id)) {
                None => None,
                Some(Some(local)) => Some(local),
                Some(None) => continue, // s absent from this page: contributes nothing
            };
            let local_p = match p.map(|id| translation.to_local(id)) {
                None => None,
                Some(Some(local)) => Some(local),
                Some(None) => continue,
            };
            let local_o = match o.map(|id| translation.to_local(id)) {
                None => None,
                Some(Some(local)) => Some(local),
                Some(None) => continue,
            };
            let local_g = match g {
                GraphMatch::Any => GraphMatch::Any,
                GraphMatch::Default => GraphMatch::Default,
                GraphMatch::Named(id) => match translation.to_local(id) {
                    Some(local) => GraphMatch::Named(local),
                    None => continue, // named graph absent from this page
                },
            };

            let raw = &raw_pages[page_map[i]];
            for q in raw.quads_for_pattern(local_s, local_p, local_o, local_g) {
                expected.push(QuadIds {
                    s: translation.to_global(q.s),
                    p: translation.to_global(q.p),
                    o: translation.to_global(q.o),
                    g: q.g.map(|x| translation.to_global(x)),
                });
            }
        }

        let dataset_actual: Vec<QuadIds<GlobalTermId>> =
            variant.quads_for_pattern(s, p, o, g).collect();
        assert_eq!(
            dataset_actual, expected,
            "PagedDataset::quads_for_pattern diverged from the per-page reference for \
             s_sel={s_sel:?} p_sel={p_sel:?} o_sel={o_sel:?} g_sel={g_sel}"
        );

        let view = variant.query_view(PagedQueryLimits::UNBOUNDED);
        let view_actual: Vec<QuadIds<GlobalTermId>> = view.quads_for_pattern(s, p, o, g).collect();
        assert_eq!(
            view_actual, expected,
            "PagedQueryView::quads_for_pattern diverged from the per-page reference for \
             s_sel={s_sel:?} p_sel={p_sel:?} o_sel={o_sel:?} g_sel={g_sel}"
        );
    }

    proptest! {
        /// The sound-filter claim, checked differentially: for randomly generated
        /// multi-page datasets and randomly generated `(s, p, o, g)` patterns, both
        /// paged `quads_for_pattern` overrides equal the per-page reference (see
        /// `assert_pattern_matches_reference`) — on the freshly sealed dataset, after
        /// `compact()`, after `with_pages`/`drop_page`, and after a `to_parts()` →
        /// `from_parts()` warm restart.
        #[test]
        fn proptest_paged_quads_for_pattern_matches_linear_scan_over_quads(
            rows in prop::collection::vec(
                (0u8..POOL_LEN, 0u8..POOL_LEN, 0u8..(POOL_LEN + 1), prop::option::of(0u8..GRAPH_LEN)),
                0..40,
            ),
            s_sel in prop::option::of(0u8..POOL_LEN),
            p_sel in prop::option::of(0u8..POOL_LEN),
            o_sel in prop::option::of(0u8..(POOL_LEN + 1)),
            // 0 = Any, 1 = Default, 2..(2+GRAPH_LEN) = Named(graphs[g - 2]).
            g_sel in 0u8..(2 + GRAPH_LEN),
            page_count in 2usize..=4,
            empty_graph_page_raw in 0usize..4,
            side_graph_page_raw in 0usize..4,
        ) {
            let empty_graph_page = empty_graph_page_raw % page_count;
            let side_graph_page = side_graph_page_raw % page_count;

            let pages = build_pages(&rows, page_count, empty_graph_page, side_graph_page);
            // Kept as the ground truth for the per-page reference: the underlying
            // per-page `RdfDataset` CONTENT never changes across the variants below
            // (`compact`/`with_pages`/`drop_page`/`from_parts` only touch page
            // numbering, global ids, or laziness — never a page's own local rows).
            let raw_pages = pages.clone();
            let provider: Arc<dyn PageProvider> = Arc::new(InMemoryPageProvider::new(pages));
            let paged = PagedDataset::from_provider(provider.clone())
                .expect("randomly generated pages are quad-disjoint by construction");

            // Variant 1: the freshly sealed dataset itself. PageId(i) == raw_pages[i].
            let identity_map: Vec<usize> = (0..page_count).collect();
            assert_pattern_matches_reference(
                &paged, &raw_pages, &identity_map, s_sel, p_sel, o_sel, g_sel,
            );

            // Variant 2: after compact() (renumbers GlobalTermIds; page SLOTS are
            // untouched, so the identity map still applies).
            let compacted = paged.compact();
            assert_pattern_matches_reference(
                &compacted, &raw_pages, &identity_map, s_sel, p_sel, o_sel, g_sel,
            );

            // Variant 3: after with_pages() reordering every page (no page dropped):
            // reordered's PageId(i) == raw_pages[page_count - 1 - i].
            let reversed: Vec<PageId> = (0..page_count)
                .rev()
                .map(|i| PageId(u32::try_from(i).expect("page count fits u32")))
                .collect();
            let reordered = paged.with_pages(&reversed);
            let reordered_map: Vec<usize> = (0..page_count).rev().collect();
            assert_pattern_matches_reference(
                &reordered, &raw_pages, &reordered_map, s_sel, p_sel, o_sel, g_sel,
            );

            // Variant 4: after drop_page(0) (page_count is always >= 2, so a page
            // always survives): dropped's PageId(i) == raw_pages[i + 1].
            let dropped = paged.drop_page(PageId(0));
            let dropped_map: Vec<usize> = (1..page_count).collect();
            assert_pattern_matches_reference(
                &dropped, &raw_pages, &dropped_map, s_sel, p_sel, o_sel, g_sel,
            );

            // Variant 5: a to_parts() -> from_parts() warm restart. Page slots are
            // preserved, so the identity map still applies.
            let (dictionary, generation, parts) = paged.to_parts();
            let warm = PagedDataset::from_parts(dictionary, provider, generation, parts)
                .expect("warm restart from matching parts");
            assert_pattern_matches_reference(
                &warm, &raw_pages, &identity_map, s_sel, p_sel, o_sel, g_sel,
            );
        }
    }
}

// ── 9b.2: parity and determinism ────────────────────────────────────────────────

/// Warm restart: `to_parts` -> `from_parts` preserves the dictionary, so a fresh
/// query on the reconstituted dataset must yield byte-identical rows AND an
/// identical `requested_pages` evidence sequence to the same query on the eagerly
/// sealed original.
#[test]
fn warm_restart_preserves_dictionary_rows_and_requested_pages_evidence() {
    let pages = vec![
        build_simple_page("s0", "p", "o0"),
        build_simple_page("s1", "p", "o1"),
        build_simple_page("s2", "p", "o2"),
    ];
    let provider: Arc<dyn PageProvider> = Arc::new(InMemoryPageProvider::new(pages));
    let eager = PagedDataset::from_provider(provider.clone()).expect("seal pages");
    let (dictionary, generation, parts) = eager.to_parts();
    let warm = PagedDataset::from_parts(dictionary, provider, generation, parts)
        .expect("warm restart from matching parts");

    // Byte-identity of rows: `to_parts` clones the dictionary verbatim, so these are
    // the literal same `GlobalTermId` numbers, not merely equal by resolved value.
    let eager_rows: Vec<_> = eager.quads().collect();
    let warm_rows: Vec<_> = warm.quads().collect();
    assert_eq!(
        eager_rows, warm_rows,
        "warm restart preserves every row byte-for-byte"
    );

    let eager_view = eager.query_view(PagedQueryLimits::UNBOUNDED);
    let warm_view = warm.query_view(PagedQueryLimits::UNBOUNDED);
    let eager_count = eager_view
        .quads_for_pattern(None, None, None, GraphMatch::Any)
        .count();
    let warm_count = warm_view
        .quads_for_pattern(None, None, None, GraphMatch::Any)
        .count();
    assert_eq!(eager_count, warm_count);
    let eager_evidence = ready_evidence(eager_view.operation_status());
    let warm_evidence = ready_evidence(warm_view.operation_status());
    assert_eq!(
        eager_evidence.requested_pages, warm_evidence.requested_pages,
        "an eagerly-sealed dataset and its warm-restarted twin request identical pages, \
         in identical order, for the identical query"
    );
}

/// Compaction: `compact()` re-interns live terms in canonical value order, so
/// `GlobalTermId` assignment CHANGES but the `named_graphs()` SET (resolved by
/// value) must be identical before and after, and AFTER compaction the order must
/// be ascending canonical `TermValue` order.
#[test]
fn compaction_preserves_the_named_graphs_value_set_in_canonical_order() {
    // Interned in an order that is deliberately NOT canonical value order (page
    // arrival order is z, a, m), so a pre-compaction order check would be a false
    // positive if it happened to already look sorted.
    let mut pages = Vec::new();
    for name in ["zzz", "aaa", "mmm"] {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s");
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");
        let g = b.intern_iri(&format!("http://example.org/{name}"));
        b.push_quad(s, p, o, Some(g));
        pages.push(b.freeze().expect("page freeze"));
    }
    let provider = Arc::new(InMemoryPageProvider::new(pages));
    let paged = PagedDataset::from_provider(provider).expect("seal pages");

    let pre_values: BTreeSet<TermValue> = paged
        .named_graphs()
        .map(|id| to_value(&paged, id))
        .collect();

    let compacted = paged.compact();
    let post_order: Vec<TermValue> = compacted
        .named_graphs()
        .map(|id| to_value(&compacted, id))
        .collect();
    let post_values: BTreeSet<TermValue> = post_order.iter().cloned().collect();

    assert_eq!(
        pre_values, post_values,
        "compaction preserves the named_graphs() VALUE set"
    );
    assert!(
        post_order.is_sorted(),
        "after compaction, named_graphs() is ascending canonical TermValue order: {post_order:?}"
    );
}

/// Determinism: the same query run twice on two FRESH views over the same snapshot
/// produces identical `requested_pages` sequences.
#[test]
fn identical_query_on_two_fresh_views_produces_identical_requested_pages() {
    let pages = vec![
        build_simple_page("s0", "p", "o0"),
        build_simple_page("s1", "p", "o1"),
        build_simple_page("s2", "p", "o2"),
    ];
    let provider = Arc::new(InMemoryPageProvider::new(pages));
    let paged = PagedDataset::from_provider(provider).expect("seal pages");

    let view_a = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let view_b = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let count_a = view_a
        .quads_for_pattern(None, None, None, GraphMatch::Any)
        .count();
    let count_b = view_b
        .quads_for_pattern(None, None, None, GraphMatch::Any)
        .count();
    assert_eq!(count_a, count_b);
    let evidence_a = ready_evidence(view_a.operation_status());
    let evidence_b = ready_evidence(view_b.operation_status());
    assert_eq!(
        evidence_a.requested_pages, evidence_b.requested_pages,
        "identical query, identical snapshot, two fresh views: identical requested_pages"
    );
}

/// Emptiness vs incompleteness: a genuinely empty graph-selective answer returns a
/// COMPLETE result with final Ready evidence, never an operational error. An empty
/// row iterator alone is ambiguous between "no matching data" and "the operation
/// broke partway through"; only the terminal status disambiguates.
#[test]
fn a_genuinely_empty_graph_selective_answer_is_a_complete_ready_result_not_an_error() {
    let mut b = RdfDatasetBuilder::new();
    let g_empty = b.intern_iri("http://example.org/gEmpty");
    b.declare_named_graph(g_empty);
    let page = b.freeze().expect("page freeze");
    let provider = Arc::new(InMemoryPageProvider::new(vec![page]));
    let paged = PagedDataset::from_provider(provider).expect("seal page");
    let g_id = paged
        .term_id_by_value(&iri("gEmpty"))
        .expect("gEmpty interned");

    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let row_count = view
        .quads_for_pattern(None, None, None, GraphMatch::Named(g_id))
        .count();
    assert_eq!(row_count, 0, "gEmpty genuinely owns no rows");
    match view.operation_status() {
        ViewOperationStatus::Ready { .. } => {}
        ViewOperationStatus::Failed { error, .. } => {
            panic!(
                "an empty graph-selective answer must be a complete Ready result, not an \
                 operational error: {error}"
            )
        }
    }
}

// ── 9b.3: acceptance criterion 3 — whole-dataset behaviour, pinned as literals ──

/// A full scan (`GraphMatch::Any`) must admit exactly the pages it admits today: all
/// of them, in ascending order. The expectation is written as a LITERAL array, not
/// computed via `pages_for_pattern` or any other code path under test.
#[test]
fn pinned_full_scan_admits_exactly_the_literal_page_set_in_order() {
    let pages = vec![
        build_simple_page("a0", "p", "o0"),
        build_simple_page("a1", "p", "o1"),
        build_simple_page("a2", "p", "o2"),
    ];
    let provider = Arc::new(InMemoryPageProvider::new(pages));
    let paged = PagedDataset::from_provider(provider).expect("seal pages");

    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let row_count = view
        .quads_for_pattern(None, None, None, GraphMatch::Any)
        .count();
    assert_eq!(row_count, 3, "sanity: one row per page");
    let evidence = ready_evidence(view.operation_status());
    assert_eq!(
        evidence.requested_pages,
        vec![PageId(0), PageId(1), PageId(2)],
        "a full scan admits every page, ascending — pinned as a literal"
    );
}

/// A cross-page join must admit exactly the pages it admits today: page 0 (the
/// first hop, `alice knows ?`) then page 1 (the second hop, `bob knows ?`) — never
/// page 2, which the admission law proves cannot contribute to either hop. The
/// expectation is written as a LITERAL array.
#[test]
fn pinned_cross_page_join_admits_exactly_the_literal_page_sequence() {
    let page0 = build_simple_page("alice", "knows", "bob");
    let page1 = build_simple_page("bob", "knows", "carol");
    let page2 = build_simple_page("dave", "knows", "eve");
    let provider = Arc::new(InMemoryPageProvider::new(vec![page0, page1, page2]));
    let paged = PagedDataset::from_provider(provider).expect("seal pages");

    let alice = paged
        .term_id_by_value(&iri("alice"))
        .expect("alice interned");
    let knows = paged
        .term_id_by_value(&iri("knows"))
        .expect("knows interned");

    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let first: Vec<_> = view
        .quads_for_pattern(Some(alice), Some(knows), None, GraphMatch::Any)
        .collect();
    assert_eq!(first.len(), 1, "sanity: alice knows exactly one thing");
    let x = first[0].o;
    let second_count = view
        .quads_for_pattern(Some(x), Some(knows), None, GraphMatch::Any)
        .count();
    assert_eq!(second_count, 1, "sanity: bob knows exactly one thing");

    let evidence = ready_evidence(view.operation_status());
    assert_eq!(
        evidence.requested_pages,
        vec![PageId(0), PageId(1)],
        "the join touches exactly page 0 (alice knows ?) then page 1 (bob knows ?), \
         pinned as a literal sequence — page 2 is never requested"
    );
}
