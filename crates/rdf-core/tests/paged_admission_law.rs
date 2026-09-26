// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A differential property test proving the page-admission law is a SOUND
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
//! (`property_indexed_pattern_matches_linear_scan`) compares via `BTreeSet` for
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

/// One `(subject predicate object)` triple in named graph `graph`, as its own page.
fn build_graph_page(subject: &str, predicate: &str, object: &str, graph: &str) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&format!("http://example.org/{subject}"));
    let p = b.intern_iri(&format!("http://example.org/{predicate}"));
    let o = b.intern_iri(&format!("http://example.org/{object}"));
    let g = b.intern_iri(&format!("http://example.org/{graph}"));
    b.push_quad(s, p, o, Some(g));
    b.freeze().expect("page freeze")
}

// ── the differential property test ──────────────────────────────────────────────

mod fuzz {
    use super::{
        Arc, BTreeSet, DatasetView, GlobalTermId, GraphMatch, InMemoryPageProvider, PageId,
        PageProvider, PagedDataset, PagedQueryLimits, QuadIds, RdfDataset, RdfDatasetBuilder,
        TermValue,
    };
    use purrdf_testkit::prop::prelude::*;

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

    /// The page a row's derived REIFIER binding is assigned to.
    ///
    /// Quad-disjointness (G3) is enforced over the reifier side table too, so the same
    /// rule the base quads obey applies here: a row's page must be a pure function of
    /// that row's OWN identity. A reifier binding is `(reifier, triple-term, graph)`,
    /// and the triple term is the same value on every page, so its identity is just
    /// `(s, g)` — strictly less than the base quad's `(s, p, o, g)`. Assigning it to
    /// the base quad's page would therefore place the SAME binding on two pages
    /// whenever two generated rows share `(s, g)` but differ on `p` or `o`, which
    /// `from_provider` correctly refuses. Keying the page on `(s, g)` alone restores
    /// the invariant — and lands the binding on a page the base row need not be on,
    /// which is the more interesting layout anyway.
    fn reifier_page(s: u8, g: Option<u8>, page_count: usize) -> usize {
        row_page(s, 0, 0, g, page_count)
    }

    /// Build `page_count` quad-disjoint pages from `rows`, each page interning the
    /// FULL shared term pool (so every pool/graph/triple term is resolvable by value
    /// regardless of whether a row on that particular page used it), plus a
    /// deliberately declared-empty graph, a reifier row in a graph with NO base
    /// quad (a side-table-only graph), and at least one quoted-triple object.
    ///
    /// # The RDF 1.2 side tables are generated, not decorative
    ///
    /// Every generated base row ALSO contributes a reifier row and an annotation row
    /// on the same page, carrying that row's own graph slot — so the reifier and
    /// annotation streams are as randomized, and as spread across graphs and pages,
    /// as the base-quad stream is. Two properties of the shape matter:
    ///
    /// * The reifier of both side rows is the base row's SUBJECT term. A pool term
    ///   that the generator only ever placed on the object axis therefore appears in
    ///   a page's term table while owning NO side-table row at all — the exact
    ///   negative case the per-term summary check (`reifier_rows`/`annotation_rows`)
    ///   exists to catch, and the one a `to_local`-only check would get wrong.
    /// * `gSide` holds a reifier row and no annotation row; `gAnno` holds an
    ///   annotation row and no reifier row. Neither graph is named by any base quad.
    ///   A page-narrowing that consulted the base-quad postings, or that shared one
    ///   posting list across the two side streams, admits the wrong pages for these
    ///   two graphs and nothing else in the corpus would notice.
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
        let mut g_anno_ids = Vec::with_capacity(page_count);
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
            let g_anno = b.intern_iri("http://example.org/gAnno");
            let r_side = b.intern_iri("http://example.org/rSide");
            pools.push(pool);
            graphs.push(gs);
            triples.push(triple);
            g_empty_ids.push(g_empty);
            g_side_ids.push(g_side);
            g_anno_ids.push(g_anno);
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
            // The RDF 1.2 twins of this row, in this row's own graph: a reifier
            // binding for the quoted triple and an annotation carrying the row's
            // predicate and object. Both are keyed on the row's SUBJECT, so a term
            // used only on the object axis owns no side-table row anywhere. Both
            // builder pushes deduplicate, so a repeated generated row collapses to one
            // side-table row exactly as it collapses to one quad.
            //
            // The annotation's identity is the whole row, so it belongs on the row's
            // own page; the reifier binding's is only `(s, g)`, so it goes to the page
            // `reifier_page` assigns — see there for why the distinction is what keeps
            // the pages disjoint.
            let r_page = reifier_page(s, g, page_count);
            let r_graph = g.map(|gi| graphs[r_page][gi as usize]);
            builders[r_page].push_reifier_in_graph(
                pools[r_page][s as usize],
                triples[r_page],
                r_graph,
            );
            builders[page].push_annotation_in_graph(s_id, p_id, o_id, g_id);
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

        // Its annotation-stream mirror: an annotation row in `gAnno`, a graph named
        // by NO base quad and by NO reifier row either. `gSide` and `gAnno` are
        // therefore single-stream graphs pointing in opposite directions, which is
        // what forces the reifier and annotation page-postings to be read
        // separately rather than through one shared list.
        builders[side_graph_page].push_annotation_in_graph(
            r_side_ids[side_graph_page],
            pools[side_graph_page][0],
            pools[side_graph_page][1],
            Some(g_anno_ids[side_graph_page]),
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
    /// `property_indexed_pattern_matches_linear_scan` in `ir/dataset.rs`)
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
    /// behavior (the repo's own `property_indexed_pattern_matches_linear_scan`
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

    /// Every term the generator ever interns, as `variant`'s `GlobalTermId`s: the
    /// probe key set for the reifier-keyed side-table walks.
    ///
    /// It deliberately includes terms that own no side-table row at all — a pool term
    /// the generator only ever placed on the object axis, the graph terms, `gEmpty` —
    /// because a keyed walk has two ways to be wrong and only one of them looks like a
    /// bug. Under-answering (skipping a page that does own rows for the key) is the
    /// silent drop; answering a non-empty stream for a key that owns nothing, or
    /// failing outright on it, is the over-refusal's mirror. Both are caught here,
    /// because the oracle is the unkeyed stream filtered by the same predicate and it
    /// is equally authoritative about emptiness.
    fn probe_terms(variant: &PagedDataset) -> Vec<GlobalTermId> {
        let mut values: Vec<TermValue> = (0..POOL_LEN).map(pool_value).collect();
        values.extend((0..GRAPH_LEN).map(graph_value));
        values.push(triple_value());
        for name in ["gEmpty", "gSide", "gAnno", "rSide"] {
            values.push(TermValue::iri(format!("http://example.org/{name}")));
        }
        values
            .iter()
            .map(|value| {
                variant
                    .term_id_by_value(value)
                    .expect("every generator term is interned on every page")
            })
            .collect()
    }

    /// Assert that every NARROWED side-table walk on BOTH paged surfaces equals the
    /// corresponding UNKEYED stream filtered the same way — same rows, same multiset,
    /// same ORDER.
    ///
    /// # Why this is the right oracle
    ///
    /// `reifier_quads_in_graph`, `annotation_quads_in_graph`, `reifier_quads_of` and
    /// `annotations_of_with_graph` are each documented as an OPTIMIZATION SEAM over an
    /// unkeyed stream: the `DatasetView` trait default for every one of them literally
    /// IS the filter, so "the narrowed walk equals the unkeyed walk filtered" is the
    /// published contract, not an implementation detail. That makes the contract the
    /// only safe thing to assert — a paged override may legitimately change WHICH
    /// pages it visits (and it does: it consults per-stream graph postings and
    /// per-term row counts to skip pages before materializing them, and may narrow
    /// further still), but it may never change the row stream those visits produce.
    /// A test written against a particular page-visiting strategy would have to be
    /// rewritten every time the strategy tightens; this one holds across all of them.
    ///
    /// The graph arms sweep `Any`, `Default` and `Named(g)` for EVERY graph the
    /// variant knows — which, because `named_graphs()` folds the declared, base-quad,
    /// reifier and annotation graph sets together, includes the declared-empty graph
    /// and the two single-stream graphs (`gSide`, `gAnno`) no base quad ever names.
    ///
    /// The last block composes the two narrowings: a keyed walk filtered to a graph,
    /// and a graph-narrowed walk filtered to a key, must both equal the unkeyed stream
    /// filtered by BOTH predicates. Narrowing on one axis may not disturb the other.
    fn assert_side_table_walks_match_unkeyed_streams(variant: &PagedDataset) {
        let view = variant.query_view(PagedQueryLimits::UNBOUNDED);

        // The unkeyed streams: the reference every narrowed walk below is measured
        // against. Both surfaces must already agree here, or nothing downstream means
        // anything.
        let reifiers: Vec<QuadIds<GlobalTermId>> = variant.reifier_quads().collect();
        let annotations: Vec<QuadIds<GlobalTermId>> = variant.annotation_quads().collect();
        assert_eq!(
            view.reifier_quads().collect::<Vec<_>>(),
            reifiers,
            "PagedQueryView::reifier_quads diverged from PagedDataset::reifier_quads"
        );
        assert_eq!(
            view.annotation_quads().collect::<Vec<_>>(),
            annotations,
            "PagedQueryView::annotation_quads diverged from PagedDataset::annotation_quads"
        );

        let mut arms: Vec<GraphMatch<GlobalTermId>> = vec![GraphMatch::Any, GraphMatch::Default];
        arms.extend(variant.named_graphs().map(GraphMatch::Named));

        for &g in &arms {
            let expected: Vec<QuadIds<GlobalTermId>> = reifiers
                .iter()
                .copied()
                .filter(|q| g.matches(q.g))
                .collect();
            assert_eq!(
                variant.reifier_quads_in_graph(g).collect::<Vec<_>>(),
                expected,
                "PagedDataset::reifier_quads_in_graph({g:?}) diverged from reifier_quads() \
                 filtered to that graph"
            );
            assert_eq!(
                view.reifier_quads_in_graph(g).collect::<Vec<_>>(),
                expected,
                "PagedQueryView::reifier_quads_in_graph({g:?}) diverged from reifier_quads() \
                 filtered to that graph"
            );

            let expected: Vec<QuadIds<GlobalTermId>> = annotations
                .iter()
                .copied()
                .filter(|q| g.matches(q.g))
                .collect();
            assert_eq!(
                variant.annotation_quads_in_graph(g).collect::<Vec<_>>(),
                expected,
                "PagedDataset::annotation_quads_in_graph({g:?}) diverged from annotation_quads() \
                 filtered to that graph"
            );
            assert_eq!(
                view.annotation_quads_in_graph(g).collect::<Vec<_>>(),
                expected,
                "PagedQueryView::annotation_quads_in_graph({g:?}) diverged from \
                 annotation_quads() filtered to that graph"
            );
        }

        for reifier in probe_terms(variant) {
            let expected: Vec<QuadIds<GlobalTermId>> = reifiers
                .iter()
                .copied()
                .filter(|q| q.s == reifier)
                .collect();
            assert_eq!(
                variant.reifier_quads_of(reifier).collect::<Vec<_>>(),
                expected,
                "PagedDataset::reifier_quads_of({reifier:?}) diverged from reifier_quads() \
                 filtered to that reifier"
            );
            assert_eq!(
                view.reifier_quads_of(reifier).collect::<Vec<_>>(),
                expected,
                "PagedQueryView::reifier_quads_of({reifier:?}) diverged from reifier_quads() \
                 filtered to that reifier"
            );

            let expected: Vec<(GlobalTermId, GlobalTermId, Option<GlobalTermId>)> = annotations
                .iter()
                .filter(|q| q.s == reifier)
                .map(|q| (q.p, q.o, q.g))
                .collect();
            assert_eq!(
                variant
                    .annotations_of_with_graph(reifier)
                    .collect::<Vec<_>>(),
                expected,
                "PagedDataset::annotations_of_with_graph({reifier:?}) diverged from \
                 annotation_quads() filtered to that reifier"
            );
            assert_eq!(
                view.annotations_of_with_graph(reifier).collect::<Vec<_>>(),
                expected,
                "PagedQueryView::annotations_of_with_graph({reifier:?}) diverged from \
                 annotation_quads() filtered to that reifier"
            );
        }

        // The two narrowings, composed. The key set is every reifier that owns a row
        // in either stream, plus `gEmpty` — a term that owns none — so the composition
        // is checked on both a populated and an empty answer.
        let mut keys: BTreeSet<GlobalTermId> = reifiers.iter().map(|q| q.s).collect();
        keys.extend(annotations.iter().map(|q| q.s));
        keys.insert(
            variant
                .term_id_by_value(&TermValue::iri("http://example.org/gEmpty"))
                .expect("gEmpty is interned on every page"),
        );
        for reifier in keys {
            for &g in &arms {
                let expected: Vec<QuadIds<GlobalTermId>> = reifiers
                    .iter()
                    .copied()
                    .filter(|q| q.s == reifier && g.matches(q.g))
                    .collect();
                assert_eq!(
                    variant
                        .reifier_quads_of(reifier)
                        .filter(|q| g.matches(q.g))
                        .collect::<Vec<_>>(),
                    expected,
                    "reifier_quads_of({reifier:?}) then graph {g:?} is not the doubly-filtered \
                     reifier stream"
                );
                assert_eq!(
                    variant
                        .reifier_quads_in_graph(g)
                        .filter(|q| q.s == reifier)
                        .collect::<Vec<_>>(),
                    expected,
                    "reifier_quads_in_graph({g:?}) then reifier {reifier:?} is not the \
                     doubly-filtered reifier stream"
                );
                assert_eq!(
                    view.reifier_quads_of(reifier)
                        .filter(|q| g.matches(q.g))
                        .collect::<Vec<_>>(),
                    expected,
                    "view reifier_quads_of({reifier:?}) then graph {g:?} is not the \
                     doubly-filtered reifier stream"
                );

                let expected: Vec<(GlobalTermId, GlobalTermId, Option<GlobalTermId>)> = annotations
                    .iter()
                    .filter(|q| q.s == reifier && g.matches(q.g))
                    .map(|q| (q.p, q.o, q.g))
                    .collect();
                assert_eq!(
                    variant
                        .annotations_of_with_graph(reifier)
                        .filter(|&(_, _, graph)| g.matches(graph))
                        .collect::<Vec<_>>(),
                    expected,
                    "annotations_of_with_graph({reifier:?}) then graph {g:?} is not the \
                     doubly-filtered annotation stream"
                );
                assert_eq!(
                    variant
                        .annotation_quads_in_graph(g)
                        .filter(|q| q.s == reifier)
                        .map(|q| (q.p, q.o, q.g))
                        .collect::<Vec<_>>(),
                    expected,
                    "annotation_quads_in_graph({g:?}) then reifier {reifier:?} is not the \
                     doubly-filtered annotation stream"
                );
                assert_eq!(
                    view.annotations_of_with_graph(reifier)
                        .filter(|&(_, _, graph)| g.matches(graph))
                        .collect::<Vec<_>>(),
                    expected,
                    "view annotations_of_with_graph({reifier:?}) then graph {g:?} is not the \
                     doubly-filtered annotation stream"
                );
            }
        }
    }

    /// One generated row: subject, predicate and object selectors, and the named
    /// graph (`None` is the default graph).
    type Row = (u8, u8, u8, Option<u8>);

    /// Everything one law draws: the rows, the `(s, p, o, g)` selectors, the page
    /// count, and the raw pages the empty-graph and side-table rows land on.
    type LawInputs = (
        Vec<Row>,
        Option<u8>,
        Option<u8>,
        Option<u8>,
        u8,
        usize,
        usize,
        usize,
    );

    fn law_inputs() -> impl Strategy<Value = LawInputs> {
        (
            prop::collection::vec(
                (
                    0u8..POOL_LEN,
                    0u8..POOL_LEN,
                    0u8..(POOL_LEN + 1),
                    prop::option::of(0u8..GRAPH_LEN),
                ),
                0..40,
            ),
            prop::option::of(0u8..POOL_LEN),
            prop::option::of(0u8..POOL_LEN),
            prop::option::of(0u8..(POOL_LEN + 1)),
            // 0 = Any, 1 = Default, 2..(2+GRAPH_LEN) = Named(graphs[g - 2]).
            0u8..(2 + GRAPH_LEN),
            2usize..=4,
            0usize..4,
            0usize..4,
        )
    }

    /// The sound-filter claim, checked differentially: for randomly generated
    /// multi-page datasets and randomly generated `(s, p, o, g)` patterns, both
    /// paged `quads_for_pattern` overrides equal the per-page reference (see
    /// `assert_pattern_matches_reference`) — on the freshly sealed dataset, after
    /// `compact()`, after `with_pages`/`drop_page`, and after a `to_parts()` →
    /// `from_parts()` warm restart.
    ///
    /// The same five variants also carry the RDF 1.2 side-table claim: every
    /// graph-narrowed and reifier-keyed reifier/annotation walk, on both paged
    /// surfaces, equals its unkeyed stream filtered the same way (see
    /// `assert_side_table_walks_match_unkeyed_streams`). The base-quad path and the
    /// side-table paths compose the same page-admission machinery, so they are
    /// worth exactly as much evidence, and a side-table walk that visits the wrong
    /// pages drops rows just as silently.
    // Eight parameters because the law is stated over exactly the eight inputs
    // `law_inputs` draws, named as the property draws them.
    #[allow(clippy::too_many_arguments)]
    fn paged_quads_for_pattern_matches_linear_scan_over_quads(
        rows: &[Row],
        s_sel: Option<u8>,
        p_sel: Option<u8>,
        o_sel: Option<u8>,
        g_sel: u8,
        page_count: usize,
        empty_graph_page_raw: usize,
        side_graph_page_raw: usize,
    ) -> Result<(), TestCaseError> {
        let empty_graph_page = empty_graph_page_raw % page_count;
        let side_graph_page = side_graph_page_raw % page_count;

        let pages = build_pages(rows, page_count, empty_graph_page, side_graph_page);
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
            &paged,
            &raw_pages,
            &identity_map,
            s_sel,
            p_sel,
            o_sel,
            g_sel,
        );
        assert_side_table_walks_match_unkeyed_streams(&paged);

        // Variant 2: after compact() (renumbers GlobalTermIds; page SLOTS are
        // untouched, so the identity map still applies).
        let compacted = paged.compact();
        assert_pattern_matches_reference(
            &compacted,
            &raw_pages,
            &identity_map,
            s_sel,
            p_sel,
            o_sel,
            g_sel,
        );
        assert_side_table_walks_match_unkeyed_streams(&compacted);

        // Variant 3: after with_pages() reordering every page (no page dropped):
        // reordered's PageId(i) == raw_pages[page_count - 1 - i].
        let reversed: Vec<PageId> = (0..page_count)
            .rev()
            .map(|i| PageId(u32::try_from(i).expect("page count fits u32")))
            .collect();
        let reordered = paged.with_pages(&reversed);
        let reordered_map: Vec<usize> = (0..page_count).rev().collect();
        assert_pattern_matches_reference(
            &reordered,
            &raw_pages,
            &reordered_map,
            s_sel,
            p_sel,
            o_sel,
            g_sel,
        );
        assert_side_table_walks_match_unkeyed_streams(&reordered);

        // Variant 4: after drop_page(0) (page_count is always >= 2, so a page
        // always survives): dropped's PageId(i) == raw_pages[i + 1].
        let dropped = paged.drop_page(PageId(0));
        let dropped_map: Vec<usize> = (1..page_count).collect();
        assert_pattern_matches_reference(
            &dropped,
            &raw_pages,
            &dropped_map,
            s_sel,
            p_sel,
            o_sel,
            g_sel,
        );
        assert_side_table_walks_match_unkeyed_streams(&dropped);

        // Variant 5: a to_parts() -> from_parts() warm restart. Page slots are
        // preserved, so the identity map still applies.
        let (dictionary, generation, parts) = paged.to_parts();
        let warm = PagedDataset::from_parts(dictionary, provider, generation, parts)
            .expect("warm restart from matching parts");
        assert_pattern_matches_reference(
            &warm,
            &raw_pages,
            &identity_map,
            s_sel,
            p_sel,
            o_sel,
            g_sel,
        );
        assert_side_table_walks_match_unkeyed_streams(&warm);
        Ok(())
    }

    /// `cardinality_estimate` is a sound UPPER BOUND on the true match count on
    /// BOTH paged surfaces, and is RESIDENCY-INDEPENDENT.
    ///
    /// The paged estimate is structurally weaker than the one
    /// `property_cardinality_estimate_upper_bounds_count` pins for a single
    /// `RdfDataset` (`ir/dataset.rs`), and this is the property that says the
    /// weakening is still sound. Where the old paged path materialized an admitted
    /// page and asked it for its own estimate, the estimate is now summed across
    /// admitted pages from each page's SEALED summary alone: a per-page `min` over
    /// independently computed per-axis row counts. That `min` is exact when exactly
    /// one axis is bound and merely an upper bound when several are (the axes'
    /// counts are each exact for their own axis, but nothing in the summary records
    /// how the axes CO-OCCUR), so the claim that survives is the one the consumer
    /// actually relies on — join-order ranking in `sparql-eval`'s BGP planner —
    /// which needs an upper bound, not an exact count.
    ///
    /// Residency-independence is a stated guarantee, not an optimization: the
    /// estimate is a pure function of `(snapshot, pattern)`. If it could shift as
    /// pages warm, plan choice would shift with it, and the `requested_pages`
    /// evidence sequence a G-clause treats as proof of what a query touched would
    /// depend on incidental cache state rather than on the query. Both surfaces are
    /// therefore measured cold and again after a full scan has forced every page
    /// resident.
    ///
    /// There is no over-refusal risk in the estimate itself — nothing refuses a
    /// query on it — so the properties asserted are soundness and stability, and
    /// the upper bound is also checked against the whole-dataset row count so a
    /// trivially enormous "estimate" cannot pass.
    // Eight parameters because the law is stated over exactly the eight inputs
    // `law_inputs` draws, named as the property draws them.
    #[allow(clippy::too_many_arguments)]
    fn paged_cardinality_estimate_upper_bounds_count(
        rows: &[Row],
        s_sel: Option<u8>,
        p_sel: Option<u8>,
        o_sel: Option<u8>,
        g_sel: u8,
        page_count: usize,
        empty_graph_page_raw: usize,
        side_graph_page_raw: usize,
    ) -> Result<(), TestCaseError> {
        let empty_graph_page = empty_graph_page_raw % page_count;
        let side_graph_page = side_graph_page_raw % page_count;

        let pages = build_pages(rows, page_count, empty_graph_page, side_graph_page);
        let provider: Arc<dyn PageProvider> = Arc::new(InMemoryPageProvider::new(pages));
        let paged = PagedDataset::from_provider(provider)
            .expect("randomly generated pages are quad-disjoint by construction");
        let (s, p, o, g) = resolve_pattern(&paged, s_sel, p_sel, o_sel, g_sel);

        // Surface 1: `PagedDataset`, read BEFORE anything has walked a row through
        // it, so its per-page cache is as cold as it will ever be.
        let cold_dataset = paged.cardinality_estimate(s, p, o, g);
        let count = paged.quads_for_pattern(s, p, o, g).count();
        prop_assert!(
            cold_dataset >= count,
            "PagedDataset estimate {} must upper-bound count {}",
            cold_dataset,
            count
        );
        let total = paged.quads().count();
        prop_assert!(
            cold_dataset <= total,
            "PagedDataset estimate {} must not exceed the whole-dataset row count {}",
            cold_dataset,
            total
        );
        // The scans above forced every page resident; the estimate may not move.
        let warm_dataset = paged.cardinality_estimate(s, p, o, g);
        prop_assert_eq!(
            cold_dataset,
            warm_dataset,
            "PagedDataset::cardinality_estimate must not depend on page residency"
        );

        // Surface 2: `PagedQueryView`, whose page cache is per-OPERATION and so
        // starts cold again on a freshly constructed view.
        let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
        let cold_view = view.cardinality_estimate(s, p, o, g);
        let view_count = view.quads_for_pattern(s, p, o, g).count();
        prop_assert!(
            cold_view >= view_count,
            "PagedQueryView estimate {} must upper-bound count {}",
            cold_view,
            view_count
        );
        let warm_view = view.cardinality_estimate(s, p, o, g);
        prop_assert_eq!(
            cold_view,
            warm_view,
            "PagedQueryView::cardinality_estimate must not depend on \
             operation-cache residency"
        );
        prop_assert_eq!(
            cold_dataset,
            cold_view,
            "both paged surfaces apply the identical estimation rule"
        );
        prop_assert_eq!(
            count,
            view_count,
            "both paged surfaces answer the identical pattern identically"
        );
        Ok(())
    }

    prop_test! {
        #[test]
        fn property_paged_quads_for_pattern_matches_linear_scan_over_quads(
            (rows, s_sel, p_sel, o_sel, g_sel, page_count, empty_graph_page_raw, side_graph_page_raw)
                in law_inputs(),
        ) {
            paged_quads_for_pattern_matches_linear_scan_over_quads(
                &rows,
                s_sel,
                p_sel,
                o_sel,
                g_sel,
                page_count,
                empty_graph_page_raw,
                side_graph_page_raw,
            )?;
        }

        #[test]
        fn property_paged_cardinality_estimate_upper_bounds_count(
            (rows, s_sel, p_sel, o_sel, g_sel, page_count, empty_graph_page_raw, side_graph_page_raw)
                in law_inputs(),
        ) {
            paged_cardinality_estimate_upper_bounds_count(
                &rows,
                s_sel,
                p_sel,
                o_sel,
                g_sel,
                page_count,
                empty_graph_page_raw,
                side_graph_page_raw,
            )?;
        }
    }

    /// Counterexamples found by an earlier property run, kept as ordinary tests.
    ///
    /// Each is the choice sequence (hex, as a failing property prints it) that makes
    /// [`law_inputs`] generate the inputs, and the inputs themselves: the test proves
    /// the sequence still decodes to exactly those inputs, then holds both paged laws
    /// over them. `SHRUNK` is the shrunk counterexample; `SEEDED` is the input its
    /// seed generated before shrinking.
    #[test]
    fn paged_law_regressions_decode_to_their_inputs_and_hold_both_laws() {
        const SHRUNK: &str = "010201020001000000000100000000010002000001010201010101010305000100020001010101000000010202050001020201000101010500000001010000000000";
        const SEEDED: &str = "010002010102010302010001000005010201010204000101000201010100000201000100010501020102010001010103000500010104010100010202040001020003000104040000010401040001020303010201010303010001010303000101000000010203020001030203010101040200010201010102000101030501000100040001010103000000010202050001020403010201010103000101010500000001010000000203";
        let shrunk: LawInputs = (
            vec![
                (2, 1, 2, None),
                (0, 0, 0, None),
                (0, 0, 0, None),
                (0, 2, 0, None),
                (1, 2, 1, Some(1)),
                (1, 3, 5, None),
                (0, 2, 0, Some(1)),
                (1, 0, 0, None),
                (2, 2, 5, None),
                (2, 2, 1, None),
                (1, 1, 5, None),
            ],
            None,
            Some(1),
            None,
            0,
            2,
            0,
            0,
        );
        let seeded: LawInputs = (
            vec![
                (0, 2, 1, Some(2)),
                (3, 2, 1, None),
                (0, 0, 5, Some(2)),
                (1, 2, 4, None),
                (1, 0, 2, Some(1)),
                (0, 0, 2, Some(0)),
                (0, 1, 5, Some(2)),
                (2, 1, 0, Some(1)),
                (3, 0, 5, None),
                (1, 4, 1, Some(0)),
                (2, 2, 4, None),
                (2, 0, 3, None),
                (4, 4, 0, None),
                (4, 1, 4, None),
                (2, 3, 3, Some(2)),
                (1, 3, 3, Some(0)),
                (1, 3, 3, None),
                (1, 0, 0, None),
                (2, 3, 2, None),
                (3, 2, 3, Some(1)),
                (4, 2, 0, Some(2)),
                (1, 1, 2, None),
                (1, 3, 5, Some(0)),
                (0, 4, 0, Some(1)),
                (3, 0, 0, None),
                (2, 2, 5, None),
                (2, 4, 3, Some(2)),
                (1, 1, 3, None),
                (1, 1, 5, None),
            ],
            None,
            Some(1),
            None,
            0,
            2,
            2,
            3,
        );
        for (label, hex, expected) in [("SHRUNK", SHRUNK, shrunk), ("SEEDED", SEEDED, seeded)] {
            let inputs = prop::replay(&law_inputs(), hex);
            assert_eq!(inputs, expected, "{label} decodes to its inputs");
            let (rows, s_sel, p_sel, o_sel, g_sel, page_count, empty, side) = inputs;
            let laws = [
                (
                    "quads_for_pattern",
                    paged_quads_for_pattern_matches_linear_scan_over_quads(
                        &rows, s_sel, p_sel, o_sel, g_sel, page_count, empty, side,
                    ),
                ),
                (
                    "cardinality_estimate",
                    paged_cardinality_estimate_upper_bounds_count(
                        &rows, s_sel, p_sel, o_sel, g_sel, page_count, empty, side,
                    ),
                ),
            ];
            for (law, outcome) in laws {
                if let Err(error) = outcome {
                    panic!("{label}: the {law} law fails: {error}");
                }
            }
        }
    }
}

// ── parity and determinism ──────────────────────────────────────────────────────

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

// ── whole-dataset behaviour, pinned as literals against a recorded baseline ─────
//
// Every pinned test below states TWO literals side by side: the `requested_pages`
// sequence the PRE-CHANGE page-admission rule produced, and the one the current rule
// produces. A single post-change literal is not evidence — it pins whatever the code
// happens to do, and passes just as happily on code that never changed. Stating the
// pair, and asserting the delta between them, is what makes these tests fail if the
// change were reverted.
//
// The pre-change rule is not remembered, it is REPLAYED: `baseline_admitted_pages`
// below reimplements it from the public sealed-metadata surface and derives each
// baseline literal from the same fixture the post-change assertion runs on.

/// Replay the PRE-CHANGE page-admission rule over `paged`'s sealed page translations,
/// returning the pages it would have admitted for `(s, p, o, g)`, ascending.
///
/// The pre-change rule translated the whole pattern per page and admitted the page
/// unless some BOUND axis' term was absent from that page's term table — a
/// role-agnostic presence test over the page's terms, and nothing more. Two things it
/// did not do are exactly what the current rule added, and both are visible in the
/// literals below:
///
/// * It never consulted the per-graph page postings, so `Named(g)` admitted any page
///   that merely interns `g` in any role, and every `named_graphs()` enumeration fell
///   through to a full `quads()` scan that materialized the whole dataset.
/// * It never consulted the per-term, per-AXIS row counts, so `(alice, knows, ?)`
///   admitted a page whose only mention of `alice` is on the OBJECT axis.
fn baseline_admitted_pages(
    paged: &PagedDataset,
    s: Option<GlobalTermId>,
    p: Option<GlobalTermId>,
    o: Option<GlobalTermId>,
    g: GraphMatch<GlobalTermId>,
) -> Vec<PageId> {
    let mut admitted = Vec::new();
    for i in 0..paged.page_count() {
        let id = PageId(u32::try_from(i).expect("page count fits u32"));
        let translation = paged.translation(id).expect("page id in range");
        let present =
            |axis: Option<GlobalTermId>| axis.is_none_or(|g| translation.to_local(g).is_some());
        let graph_present = match g {
            GraphMatch::Any | GraphMatch::Default => true,
            GraphMatch::Named(graph) => translation.to_local(graph).is_some(),
        };
        if present(s) && present(p) && present(o) && graph_present {
            admitted.push(id);
        }
    }
    admitted
}

/// The `requested_pages` sequence one operation records for a SEQUENCE of admissions:
/// first occurrence wins, later repeats are silent.
///
/// A page is pushed onto the evidence only on its first materialization within an
/// operation (`PagedQueryView::materialize_page` runs inside a `get_or_init`), so a
/// second step that re-admits an already-resident page adds nothing. This is why a
/// pre-change sequence can be SHORTER than the concatenation of its steps, and why
/// removing a wide first step can make a later step's own requests visible for the
/// first time.
fn first_occurrences(steps: &[Vec<PageId>]) -> Vec<PageId> {
    let mut seen = BTreeSet::new();
    let mut order = Vec::new();
    for step in steps {
        for &id in step {
            if seen.insert(id) {
                order.push(id);
            }
        }
    }
    order
}

/// A full scan (`GraphMatch::Any`) must admit exactly the pages it admits today: all
/// of them, in ascending order. The expectation is written as a LITERAL array, not
/// computed via `pages_for_pattern` or any other code path under test.
///
/// # The two literals, and why this shape cannot distinguish the change
///
/// * Pre-change:  `[PageId(0), PageId(1), PageId(2)]`
/// * Post-change: `[PageId(0), PageId(1), PageId(2)]`
///
/// They are identical, and that is the correct answer rather than an oversight: a
/// fully unbound pattern under `GraphMatch::Any` gives NEITHER rule an axis to narrow
/// on, every page is a candidate under both, and every page really does own a
/// matching row. So this test would pass unchanged on the pre-change code, and it is
/// not evidence that the change happened — it is an OVER-REFUSAL guard, pinning that
/// the new narrowing did not quietly shrink a scan that has to stay wide. The
/// whole-dataset shapes that do distinguish the change are
/// `pinned_graph_enumeration_admits_no_page_where_the_pre_change_rule_scanned_them_all`
/// and `pinned_cross_page_join_never_admits_a_page_that_only_mentions_the_subject_as_an_object`.
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
        "post-change literal: a full scan admits every page, ascending"
    );

    let baseline = baseline_admitted_pages(&paged, None, None, None, GraphMatch::Any);
    assert_eq!(
        baseline,
        vec![PageId(0), PageId(1), PageId(2)],
        "pre-change literal: the old rule also admitted every page for an unbound scan"
    );
    assert_eq!(
        evidence.requested_pages, baseline,
        "the two literals coincide BY DESIGN for this shape — an unbound scan under \
         GraphMatch::Any offers neither rule anything to narrow on, so this test is an \
         over-refusal guard, not evidence that the admission rule changed"
    );
}

/// A cross-page join must admit exactly the pages it admits today: page 0 (the
/// first hop, `alice knows ?`) then page 1 (the second hop, `bob knows ?`) — never
/// page 2, which the admission law proves cannot contribute to either hop. The
/// expectation is written as a LITERAL array.
///
/// # The two literals, and why this shape cannot distinguish the change
///
/// * Pre-change:  `[PageId(0), PageId(1)]`
/// * Post-change: `[PageId(0), PageId(1)]`
///
/// Identical again, and again for a reason worth stating rather than hiding. Page 2
/// (`dave knows eve`) interns neither `alice` nor `bob`, so the old presence-only rule
/// already excluded it on both hops; the new per-axis rule excludes it for a stronger
/// reason but reaches the same page set. The one page this fixture could have
/// distinguished on — page 0 on the SECOND hop, which interns `bob` only as an object
/// and so was admitted by the old rule and is refused by the new one — was already
/// resident from the first hop, and a re-admission of a resident page records no
/// evidence. So this test, too, passes on the pre-change code.
///
/// `pinned_cross_page_join_never_admits_a_page_that_only_mentions_the_subject_as_an_object`
/// is the same join over a fixture that moves the object-only mention onto a page the
/// first hop has NOT already made resident, which is what makes the difference
/// observable in the evidence.
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
        "post-change literal: the join touches exactly page 0 (alice knows ?) then page 1 \
         (bob knows ?) — page 2 is never requested"
    );

    let baseline = first_occurrences(&[
        baseline_admitted_pages(&paged, Some(alice), Some(knows), None, GraphMatch::Any),
        baseline_admitted_pages(&paged, Some(x), Some(knows), None, GraphMatch::Any),
    ]);
    assert_eq!(
        baseline,
        vec![PageId(0), PageId(1)],
        "pre-change literal: the old rule re-admitted page 0 on the second hop (it interns \
         `bob` as an object), but page 0 was already resident, so the evidence is the same"
    );
    assert_eq!(
        evidence.requested_pages, baseline,
        "the two literals coincide for this fixture — see this test's doc comment, and the \
         object-only-mention fixture that separates them"
    );
}

/// A graph enumeration must charge NOTHING: `named_graphs()` is answered from sealed
/// per-page metadata, so no page is requested — and a graph-selective read that
/// follows it then requests exactly the one page that can hold a row in that graph.
///
/// # The two literals, and the delta between them
///
/// * Pre-change:  `[PageId(0), PageId(1), PageId(2)]`
/// * Post-change: `[]` for the enumeration, then `[PageId(1)]` once the selective read
///   runs.
///
/// The delta is exactly the removal of the LEADING `named_graphs()` block. The
/// pre-change surface did not override `named_graphs` at all, so it fell through to
/// the `DatasetView` default — a `quads()` scan collecting each row's graph slot —
/// which materializes every page in ascending order before answering. That block is
/// the whole of the pre-change sequence here, and removing it does two things at once:
/// the enumeration's own three requests vanish, and the selective read's single
/// request becomes VISIBLE, because it is no longer masked by a page the enumeration
/// had already made resident.
///
/// This test fails if the change is reverted: restoring the default `named_graphs`
/// puts `[PageId(0), PageId(1), PageId(2)]` back where `[]` is asserted.
#[test]
fn pinned_graph_enumeration_admits_no_page_where_the_pre_change_rule_scanned_them_all() {
    let pages = vec![
        build_graph_page("s0", "p", "o0", "g0"),
        build_graph_page("s1", "p", "o1", "g1"),
        build_graph_page("s2", "p", "o2", "g2"),
    ];
    let provider = Arc::new(InMemoryPageProvider::new(pages));
    let paged = PagedDataset::from_provider(provider).expect("seal pages");
    let g1 = paged.term_id_by_value(&iri("g1")).expect("g1 interned");

    // Post-change: enumerate first, and assert the evidence BEFORE the selective read,
    // so the enumeration's own charge is measured on its own.
    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let enumerated: Vec<_> = view.named_graphs().collect();
    assert_eq!(
        enumerated.len(),
        3,
        "sanity: three named graphs, one per page"
    );
    assert_eq!(
        ready_evidence(view.operation_status()).requested_pages,
        Vec::<PageId>::new(),
        "post-change literal: enumerating named graphs requests no page at all"
    );
    let row_count = view
        .quads_for_pattern(None, None, None, GraphMatch::Named(g1))
        .count();
    assert_eq!(row_count, 1, "sanity: g1 owns exactly one row");
    let post_change = ready_evidence(view.operation_status()).requested_pages;
    assert_eq!(
        post_change,
        vec![PageId(1)],
        "post-change literal: enumeration charges nothing, so the selective read's own \
         single request is the entire sequence"
    );

    // Pre-change: replay the old `named_graphs` on a FRESH view. The old surface had no
    // override, so the trait default ran verbatim — a `quads()` scan collecting graph
    // slots — and `PagedQueryView::quads` materializes every page in ascending order.
    let baseline_view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let baseline_enumerated: BTreeSet<GlobalTermId> =
        baseline_view.quads().filter_map(|q| q.g).collect();
    assert_eq!(
        baseline_enumerated.len(),
        enumerated.len(),
        "both rules enumerate the same graphs for this fixture (every graph owns a row), \
         so the delta below is purely about which pages were charged for saying so"
    );
    // The same selective read, on the same view. Its admission rule is irrelevant to
    // the evidence here: the scan above already made every page resident, and a
    // re-admission of a resident page records nothing.
    let baseline_rows = baseline_view
        .quads_for_pattern(None, None, None, GraphMatch::Named(g1))
        .count();
    assert_eq!(baseline_rows, 1, "sanity: the same single row");
    let pre_change = ready_evidence(baseline_view.operation_status()).requested_pages;
    assert_eq!(
        pre_change,
        vec![PageId(0), PageId(1), PageId(2)],
        "pre-change literal: the enumeration alone materialized the whole dataset"
    );

    assert_ne!(
        pre_change, post_change,
        "this shape MUST separate the two rules, or it is not evidence of anything"
    );
    let removed: Vec<PageId> = pre_change
        .iter()
        .copied()
        .filter(|id| !post_change.contains(id))
        .collect();
    assert_eq!(
        removed,
        vec![PageId(0), PageId(2)],
        "the delta is exactly the leading named_graphs() block: the two pages that hold \
         no row in g1 and were requested only to enumerate graph names"
    );
}

/// A cross-page join must never admit a page whose only mention of the join key is on
/// the OBJECT axis — the per-axis row counts refuse it, where a role-agnostic
/// term-presence test admits it.
///
/// # The two literals, and the delta between them
///
/// * Pre-change:  `[PageId(0), PageId(3), PageId(1)]`
/// * Post-change: `[PageId(0), PageId(1)]`
///
/// The fixture adds `carol knows alice` as page 3. The first hop, `alice knows ?`,
/// finds `alice` in page 3's term table — as the OBJECT of that page's only row — so
/// the pre-change presence test admitted and materialized page 3, out of ascending
/// order relative to the second hop's page 1. The current rule reads the page's sealed
/// count of rows with `alice` on the SUBJECT axis, finds zero, and skips the page
/// without materializing it. The delta is exactly `PageId(3)`, removed from the middle
/// of the sequence — so this test fails if the change is reverted.
#[test]
fn pinned_cross_page_join_never_admits_a_page_that_only_mentions_the_subject_as_an_object() {
    let pages = vec![
        build_simple_page("alice", "knows", "bob"),
        build_simple_page("bob", "knows", "carol"),
        build_simple_page("dave", "knows", "eve"),
        // `alice` appears here ONLY as an object.
        build_simple_page("carol", "knows", "alice"),
    ];
    let provider = Arc::new(InMemoryPageProvider::new(pages));
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
    assert_eq!(
        first.len(),
        1,
        "sanity: alice is the SUBJECT of exactly one row, on page 0"
    );
    let x = first[0].o;
    let second_count = view
        .quads_for_pattern(Some(x), Some(knows), None, GraphMatch::Any)
        .count();
    assert_eq!(second_count, 1, "sanity: bob knows exactly one thing");
    let post_change = ready_evidence(view.operation_status()).requested_pages;
    assert_eq!(
        post_change,
        vec![PageId(0), PageId(1)],
        "post-change literal: page 3 is never requested — its only `alice` is an object"
    );

    let pre_change = first_occurrences(&[
        baseline_admitted_pages(&paged, Some(alice), Some(knows), None, GraphMatch::Any),
        baseline_admitted_pages(&paged, Some(x), Some(knows), None, GraphMatch::Any),
    ]);
    assert_eq!(
        pre_change,
        vec![PageId(0), PageId(3), PageId(1)],
        "pre-change literal: the first hop also materialized page 3, which interns `alice` \
         only as an object"
    );

    assert_ne!(
        pre_change, post_change,
        "this shape MUST separate the two rules, or it is not evidence of anything"
    );
    let removed: Vec<PageId> = pre_change
        .iter()
        .copied()
        .filter(|id| !post_change.contains(id))
        .collect();
    assert_eq!(
        removed,
        vec![PageId(3)],
        "the delta is exactly the object-only page, and the surviving order is unchanged"
    );

    // Over-refusal guard: refusing page 3 for `(alice, knows, ?)` must NOT mean the
    // page has become unreachable. The neighbouring pattern that genuinely matches it
    // still does, and still returns its row.
    let carol = paged
        .term_id_by_value(&iri("carol"))
        .expect("carol interned");
    let reachable = paged
        .quads_for_pattern(Some(carol), Some(knows), Some(alice), GraphMatch::Any)
        .count();
    assert_eq!(
        reachable, 1,
        "the refused page is refused only for the pattern it cannot match: `carol knows \
         alice` is still answered from it"
    );
}
