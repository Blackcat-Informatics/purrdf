// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Falsifiable tests for the reference demand-paged dataset (`PagedDataset`): the
//! id-agnostic `DatasetView` read path over many frozen `RdfDataset` pages composed
//! through the shared `GlobalTermId` space.
//!
//! Three guards:
//! 1. **Cross-page parity** — the trait surface over a multi-page `PagedDataset`
//!    yields exactly the rows of a single `RdfDataset` of the same triples (whole-
//!    dataset scan, a two-pattern join whose `?x` unifies across a page boundary, and
//!    `DatasetView::members` walking an `rdf:List` whose cons cells straddle pages).
//! 2. **Lazy hook fires on demand** — a `CountingDemandProvider` shows the seal pass
//!    pulling each page once, value lookups pulling nothing, and a bound-id query
//!    re-materializing exactly the one page that can match (cached by the `OnceLock`).
//! 3. **Cross-page cost model (F1)** — `cardinality_estimate` on a skewed page
//!    distribution equals the independently-computed per-page sum.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use purrdf_core::{
    CountingDemandProvider, DatasetView, FallibleDatasetView, GraphMatch, InMemoryPageProvider,
    PageFault, PageFaultKind, PageGeneration, PageId, PageMaterialization, PageProvider,
    PagedDataset, PagedFreezeError, PagedQuadTable, PagedQueryLimits, RdfDataset,
    RdfDatasetBuilder, RdfLiteral, StopCause, TermId, TermRef, TermValue, ViewOperationStatus,
    render_canonical_turtle,
};

// The standard RDF Collection vocabulary (crate-internal constants are not public;
// these are the well-known IRIs).
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";

/// An `example.org` IRI value.
fn iri(name: &str) -> TermValue {
    TermValue::iri(format!("http://example.org/{name}"))
}

/// Intern one dataset-independent value into a builder, recursing for triple terms
/// (this is the by-value inverse the fixtures need).
fn intern_value(b: &mut RdfDatasetBuilder, v: &TermValue) -> TermId {
    match v {
        TermValue::Iri(s) => b.intern_iri(s),
        TermValue::Blank { label, scope } => b.intern_blank(label, *scope),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => b.intern_literal(RdfLiteral {
            lexical_form: lexical_form.clone(),
            datatype: Some(datatype.clone()),
            language: language.clone(),
            direction: *direction,
        }),
        TermValue::Triple { s, p, o } => {
            let s = intern_value(b, s);
            let p = intern_value(b, p);
            let o = intern_value(b, o);
            b.intern_triple(s, p, o)
        }
    }
}

type Triple = (TermValue, TermValue, TermValue);

/// Freeze one page from a list of `(s, p, o)` triples in the default graph.
fn build_page(triples: &[Triple]) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    for (s, p, o) in triples {
        let s = intern_value(&mut b, s);
        let p = intern_value(&mut b, p);
        let o = intern_value(&mut b, o);
        b.push_quad(s, p, o, None);
    }
    b.freeze().expect("page freeze")
}

/// Resolve a view id to its dataset-INDEPENDENT `TermValue`, recursing through the
/// literal datatype and triple components. Generic over any `DatasetView`, so the
/// same routine reads a single `RdfDataset` and a multi-page `PagedDataset` and lets
/// their rows be compared by value.
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
                _ => panic!("literal datatype must resolve to an IRI"),
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

/// A deterministic sort key for a value row (`TermValue` is not `Ord`; its `Debug`
/// form is total and dataset-independent).
fn row_key(row: &[TermValue]) -> String {
    format!("{row:?}")
}

/// Collect every quad of the view as sorted `[s, p, o, g?]` value rows, via the
/// generic trait surface only. `g` is rendered as an extra element (a sentinel for
/// the default graph) so named-graph quads stay distinguishable.
fn collect_rows<V: DatasetView>(v: &V) -> Vec<Vec<TermValue>> {
    let mut rows: Vec<Vec<TermValue>> = v
        .quads_for_pattern(None, None, None, GraphMatch::Any)
        .map(|q| {
            let mut row = vec![to_value(v, q.s), to_value(v, q.p), to_value(v, q.o)];
            row.push(q.g.map_or_else(|| TermValue::iri("urn:default-graph"), |g| to_value(v, g)));
            row
        })
        .collect();
    rows.sort_by_key(|r| row_key(r));
    rows
}

/// A generic two-pattern join `(a_s a_p ?x)(?x b_p ?y)` returning the sorted
/// `(?x, ?y)` value pairs. `?x` is threaded as a NATIVE view id from the first
/// pattern into the second — the id-space unification a `PagedDataset` must provide
/// across a page boundary.
fn join_rows<V: DatasetView>(
    v: &V,
    a_s: &TermValue,
    a_p: &TermValue,
    b_p: &TermValue,
) -> Vec<(TermValue, TermValue)> {
    let a_s_id = v.term_id_by_value(a_s);
    let a_p_id = v.term_id_by_value(a_p);
    let b_p_id = v.term_id_by_value(b_p);
    let mut out: Vec<(TermValue, TermValue)> = Vec::new();
    for q1 in v.quads_for_pattern(a_s_id, a_p_id, None, GraphMatch::Any) {
        let x = q1.o;
        for q2 in v.quads_for_pattern(Some(x), b_p_id, None, GraphMatch::Any) {
            out.push((to_value(v, x), to_value(v, q2.o)));
        }
    }
    out.sort_by_key(|(x, y)| format!("{x:?}{y:?}"));
    out
}

/// Generic list walk: resolve `head` by value, then `DatasetView::members`, mapping
/// each member id back to its value.
fn walk_members<V: DatasetView>(v: &V, head: &TermValue) -> Vec<TermValue> {
    let head_id = v.term_id_by_value(head).expect("head interned");
    v.members(head_id, GraphMatch::Any)
        .expect("well-formed list")
        .into_iter()
        .map(|id| to_value(v, id))
        .collect()
}

/// The full triple corpus used by the parity test. Cons cells `c1 -> c2 -> c3 -> nil`
/// form an `rdf:List`; the `knows` edges form the join chain.
fn parity_corpus() -> Vec<Triple> {
    vec![
        // A join chain: alice knows bob; bob knows carol (unifies on ?x = bob).
        (iri("alice"), iri("knows"), iri("bob")),
        (iri("bob"), iri("knows"), iri("carol")),
        // A typed literal object, to exercise datatype interning across pages.
        (
            iri("alice"),
            iri("age"),
            TermValue::typed_literal("42", "http://www.w3.org/2001/XMLSchema#integer"),
        ),
        // An rdf:List (item1, item2, item3) whose cons cells straddle pages.
        (iri("c1"), TermValue::iri(RDF_FIRST), iri("item1")),
        (iri("c1"), TermValue::iri(RDF_REST), iri("c2")),
        (iri("c2"), TermValue::iri(RDF_FIRST), iri("item2")),
        (iri("c2"), TermValue::iri(RDF_REST), iri("c3")),
        (iri("c3"), TermValue::iri(RDF_FIRST), iri("item3")),
        (iri("c3"), TermValue::iri(RDF_REST), TermValue::iri(RDF_NIL)),
    ]
}

/// Split a corpus round-robin across `page_count` quad-disjoint pages.
fn split_pages(triples: &[Triple], page_count: usize) -> Vec<Arc<RdfDataset>> {
    let mut buckets: Vec<Vec<Triple>> = vec![Vec::new(); page_count];
    for (i, t) in triples.iter().enumerate() {
        buckets[i % page_count].push(t.clone());
    }
    buckets.iter().map(|b| build_page(b)).collect()
}

#[test]
fn cross_page_parity_via_trait_surface() {
    let corpus = parity_corpus();

    // The single-dataset reference.
    let single = build_page(&corpus);

    // The multi-page paged view over the SAME triples, split across 3 pages so the
    // join chain and the list cons cells straddle page boundaries.
    let pages = split_pages(&corpus, 3);
    let provider = Arc::new(InMemoryPageProvider::new(pages));
    let paged = PagedDataset::from_provider(provider).expect("seal pages");

    assert_eq!(paged.page_count(), 3);

    // (1) Whole-dataset scan parity.
    assert_eq!(
        collect_rows(&*single),
        collect_rows(&paged),
        "multi-page scan must equal the single-dataset scan"
    );

    // (2) Two-pattern join parity: ?x unifies bob across pages.
    let single_join = join_rows(&*single, &iri("alice"), &iri("knows"), &iri("knows"));
    let paged_join = join_rows(&paged, &iri("alice"), &iri("knows"), &iri("knows"));
    assert_eq!(
        single_join,
        vec![(iri("bob"), iri("carol"))],
        "join must find alice->bob->carol"
    );
    assert_eq!(single_join, paged_join, "join parity across pages");

    // (3) rdf:List membership parity with cons cells on different pages.
    let single_members = walk_members(&*single, &iri("c1"));
    let paged_members = walk_members(&paged, &iri("c1"));
    assert_eq!(
        single_members,
        vec![iri("item1"), iri("item2"), iri("item3")],
        "list members in order"
    );
    assert_eq!(single_members, paged_members, "member parity across pages");
}

/// Page 0: `o_shared` interned FIRST (local index 0). `s0` present only here.
fn lazy_page0() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let o = b.intern_iri("http://example.org/o_shared");
    let s = b.intern_iri("http://example.org/s0");
    let p = b.intern_iri("http://example.org/p");
    b.push_quad(s, p, o, None);
    b.freeze().expect("page0")
}

/// Page 1: `o_shared` interned LAST (local index 2). `s1` present only here — so its
/// distinct local index vs page 0 proves the id spaces are independent.
fn lazy_page1() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s1");
    let p = b.intern_iri("http://example.org/p");
    let o = b.intern_iri("http://example.org/o_shared");
    b.push_quad(s, p, o, None);
    b.freeze().expect("page1")
}

/// Page 2: `s2` present only here.
fn lazy_page2() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s2");
    let p = b.intern_iri("http://example.org/p");
    let o = b.intern_iri("http://example.org/o_shared");
    b.push_quad(s, p, o, None);
    b.freeze().expect("page2")
}

#[test]
fn lazy_hook_fires_on_demand() {
    let provider = Arc::new(CountingDemandProvider::new(vec![
        Box::new(lazy_page0),
        Box::new(lazy_page1),
        Box::new(lazy_page2),
    ]));
    let paged =
        PagedDataset::from_provider(provider.clone() as Arc<dyn PageProvider>).expect("seal pages");

    // The seal pass materializes each page exactly once.
    let hits_after_construction = provider.hits();
    assert_eq!(hits_after_construction, 3, "seal pass pulls each page once");

    // A value lookup is answered by the shared dictionary — no page is pulled.
    let s1 = paged
        .term_id_by_value(&iri("s1"))
        .expect("s1 interned at seal");
    assert_eq!(
        provider.hits(),
        hits_after_construction,
        "term_id_by_value must not materialize a page"
    );

    // A query bound to a subject present only on page 1 re-materializes exactly that
    // one page (pages 0 and 2 are skipped before any materialization).
    let row_count = paged
        .quads_for_pattern(Some(s1), None, None, GraphMatch::Any)
        .count();
    assert_eq!(row_count, 1, "s1 has exactly one quad");
    assert_eq!(
        provider.hits(),
        hits_after_construction + 1,
        "only page 1 re-materialized"
    );

    // Re-running the same query hits the OnceLock cache — no further pull.
    let _ = paged
        .quads_for_pattern(Some(s1), None, None, GraphMatch::Any)
        .count();
    assert_eq!(
        provider.hits(),
        hits_after_construction + 1,
        "cached page is not re-materialized"
    );

    // The shared object lives on all three pages: ONE GlobalTermId, but DISTINCT
    // local TermIds via the per-page translations.
    let o_global = paged
        .term_id_by_value(&iri("o_shared"))
        .expect("o_shared interned");
    let local0 = paged
        .translation(PageId(0))
        .expect("page 0")
        .to_local(o_global)
        .expect("o_shared on page 0");
    let local1 = paged
        .translation(PageId(1))
        .expect("page 1")
        .to_local(o_global)
        .expect("o_shared on page 1");
    assert_ne!(
        local0, local1,
        "same global id maps to distinct local ids on different pages"
    );
    assert_eq!(
        local0,
        TermId::from_index(0),
        "o_shared is index 0 on page 0"
    );
    assert_eq!(
        local1,
        TermId::from_index(2),
        "o_shared is index 2 on page 1"
    );
}

#[test]
fn cross_page_cost_model_is_per_page_sum() {
    // A skewed distribution: predicate `dense` is heavy on page 0 and sparse
    // elsewhere. Every page also carries `dense`, so no page is skipped and the sum
    // spans all pages.
    let dense = iri("dense");
    let page0 = build_page(&[
        (iri("a0"), dense.clone(), iri("x0")),
        (iri("a1"), dense.clone(), iri("x1")),
        (iri("a2"), dense.clone(), iri("x2")),
        (iri("a3"), dense.clone(), iri("x3")),
        (iri("a4"), dense.clone(), iri("x4")),
    ]);
    let page1 = build_page(&[(iri("b0"), dense.clone(), iri("y0"))]);
    let page2 = build_page(&[(iri("c0"), dense.clone(), iri("z0"))]);

    let raw_pages = vec![page0, page1, page2];
    let provider = Arc::new(InMemoryPageProvider::new(raw_pages.clone()));
    let paged = PagedDataset::from_provider(provider).expect("seal pages");

    let dense_g = paged
        .term_id_by_value(&dense)
        .expect("dense predicate interned");

    // The paged estimate for (?, dense, ?, Any).
    let paged_estimate = paged.cardinality_estimate(None, Some(dense_g), None, GraphMatch::Any);

    // Independently: translate the pattern per page and sum each page's own estimate.
    let expected: usize = raw_pages
        .iter()
        .map(|page| {
            page.term_id_by_value(&dense).map_or(0, |local_p| {
                page.cardinality_estimate(None, Some(local_p), None, GraphMatch::Any)
            })
        })
        .sum();

    assert_eq!(
        paged_estimate, expected,
        "paged cardinality must equal the per-page sum (Merge-scope summation)"
    );
    // Falsifiability: the sum spans multiple pages (not a single-page constant).
    assert!(expected >= 7, "skewed corpus totals at least 5 + 1 + 1");
}

#[test]
fn cardinality_estimate_materializes_no_page() {
    // Two pages, both carrying predicate `p`: the estimate must read only sealed
    // `PageSummary` metadata, on BOTH `cardinality_estimate` surfaces — `PagedDataset`
    // (planning before any query view exists) and `PagedQueryView` (planning inside
    // one fallible operation).
    let p = iri("p");
    let provider = Arc::new(CountingDemandProvider::new(vec![
        Box::new(|| build_page(&[(iri("a0"), iri("p"), iri("o0"))])),
        Box::new(|| build_page(&[(iri("a1"), iri("p"), iri("o1"))])),
    ]));
    let paged =
        PagedDataset::from_provider(provider.clone() as Arc<dyn PageProvider>).expect("seal pages");
    let p_id = paged.term_id_by_value(&p).expect("p interned");

    // The seal pass materialized each page exactly once; that is the only charge the
    // whole test should ever record.
    let hits_after_seal = provider.hits();
    assert_eq!(
        hits_after_seal,
        paged.page_count(),
        "seal pass pulls each page once"
    );

    // Surface 1: `PagedDataset::cardinality_estimate` — used by planning before any
    // query view is constructed.
    let dataset_estimate = paged.cardinality_estimate(None, Some(p_id), None, GraphMatch::Any);
    assert_eq!(dataset_estimate, 2, "one matching row per page");
    assert_eq!(
        provider.hits(),
        hits_after_seal,
        "PagedDataset::cardinality_estimate must materialize no page"
    );

    // Surface 2: `PagedQueryView::cardinality_estimate` — used by planning inside a
    // fallible operation, before any pattern has been read through that operation.
    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let view_estimate = view.cardinality_estimate(None, Some(p_id), None, GraphMatch::Any);
    assert_eq!(view_estimate, 2, "one matching row per page");
    assert_eq!(
        provider.hits(),
        hits_after_seal,
        "PagedQueryView::cardinality_estimate must materialize no page"
    );
    assert!(
        matches!(view.operation_status(), ViewOperationStatus::Ready { .. }),
        "estimating must not touch operational state"
    );
}

#[test]
fn cardinality_estimate_is_residency_independent() {
    // Page 0 carries two `p` rows, page 1 carries one — both pages are candidates for
    // (?, p, ?, Any).
    let p = iri("p");
    let page0 = build_page(&[
        (iri("a0"), p.clone(), iri("o0")),
        (iri("a1"), p.clone(), iri("o1")),
    ]);
    let page1 = build_page(&[(iri("b0"), p.clone(), iri("o2"))]);
    let provider = Arc::new(InMemoryPageProvider::new(vec![page0, page1]));
    let paged = PagedDataset::from_provider(provider).expect("seal pages");
    let p_id = paged.term_id_by_value(&p).expect("p interned");

    // Surface 1: `PagedDataset` — its own per-page `OnceLock` cache starts cold.
    let cold_dataset_estimate = paged.cardinality_estimate(None, Some(p_id), None, GraphMatch::Any);
    // Force both pages resident through a real pattern read.
    let row_count = paged
        .quads_for_pattern(None, Some(p_id), None, GraphMatch::Any)
        .count();
    assert_eq!(row_count, 3, "sanity: three rows across both pages");
    let warm_dataset_estimate = paged.cardinality_estimate(None, Some(p_id), None, GraphMatch::Any);
    assert_eq!(
        cold_dataset_estimate, warm_dataset_estimate,
        "PagedDataset::cardinality_estimate must not depend on page residency"
    );

    // Surface 2: `PagedQueryView` — its per-OPERATION cache starts cold.
    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let cold_view_estimate = view.cardinality_estimate(None, Some(p_id), None, GraphMatch::Any);
    let view_row_count = view
        .quads_for_pattern(None, Some(p_id), None, GraphMatch::Any)
        .count();
    assert_eq!(view_row_count, 3, "sanity: three rows across both pages");
    let warm_view_estimate = view.cardinality_estimate(None, Some(p_id), None, GraphMatch::Any);
    assert_eq!(
        cold_view_estimate, warm_view_estimate,
        "PagedQueryView::cardinality_estimate must not depend on operation-cache residency"
    );
    assert_eq!(
        cold_dataset_estimate, cold_view_estimate,
        "both surfaces apply the identical rule"
    );
}

#[test]
fn pages_for_pattern_predicts_actual_consumption() {
    // Four pages. Only pages 0 and 3 can possibly match `(?, p, ?, Named(g))`:
    // - page 0: `p` in graph `g` — admits on both axes.
    // - page 1: `q` (not `p`) in graph `g` — the graph axis owns a row, but the
    //   predicate axis proves it cannot match.
    // - page 2: `p` in a DIFFERENT graph `h` — the predicate axis owns a row, but the
    //   graph axis proves it cannot match (and the graph-index posting list for `g`
    //   never lists it as a candidate at all).
    // - page 3: `p` in graph `g` — admits on both axes.
    let g = "http://example.org/g";
    let h = "http://example.org/h";
    let build_named = |subject: &str, predicate: &str, object: &str, graph: &str| {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(&format!("http://example.org/{subject}"));
        let p = b.intern_iri(&format!("http://example.org/{predicate}"));
        let o = b.intern_iri(&format!("http://example.org/{object}"));
        let graph = b.intern_iri(graph);
        b.push_quad(s, p, o, Some(graph));
        b.freeze().expect("page freeze")
    };
    let pages = vec![
        build_named("a0", "p", "o0", g),
        build_named("b0", "q", "o1", g),
        build_named("c0", "p", "o2", h),
        build_named("d0", "p", "o3", g),
    ];

    let provider = Arc::new(InMemoryPageProvider::new(pages));
    let paged = PagedDataset::from_provider(provider).expect("seal pages");
    let p_id = paged.term_id_by_value(&iri("p")).expect("p interned");
    let g_id = paged.term_id_by_value(&iri("g")).expect("g interned");

    let predicted = paged.pages_for_pattern(None, Some(p_id), None, GraphMatch::Named(g_id));
    assert_eq!(
        predicted,
        vec![PageId(0), PageId(3)],
        "only the pages genuinely admitting both axes are predicted"
    );

    // The actual consumption: the same pattern, run through a fresh fallible
    // operation under an UNBOUNDED budget, read to exhaustion.
    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let row_count = view
        .quads_for_pattern(None, Some(p_id), None, GraphMatch::Named(g_id))
        .count();
    assert_eq!(
        row_count, 2,
        "sanity: pages 0 and 3 each contribute one row"
    );
    let evidence = match view.operation_status() {
        ViewOperationStatus::Ready { evidence } => evidence,
        ViewOperationStatus::Failed { error, .. } => {
            panic!("expected a ready operation, got: {error}")
        }
    };
    assert_eq!(
        evidence.requested_pages, predicted,
        "pages_for_pattern predicts exactly the pages the query actually consumed"
    );
}

#[test]
fn reifier_and_annotation_views_compose_across_pages() {
    // Page A: a base triple, its reifier binding `r rdf:reifies <<(s p o)>>`, and one
    // annotation on `r`. This page surfaces quoted_triples + reifiers + annotations.
    let page_a = {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s");
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");
        b.push_quad(s, p, o, None);
        let triple = b.intern_triple(s, p, o);
        let r = b.intern_iri("http://example.org/r");
        b.push_reifier(r, triple);
        let conf = b.intern_iri("http://example.org/confidence");
        let high = b.intern_iri("http://example.org/high");
        b.push_annotation(r, conf, high);
        b.freeze().expect("page a")
    };
    // Page B: a SECOND annotation on the same reifier resource `r` (by value). The
    // reifier need not bind a triple here — an annotation only requires an asserted
    // subject — so this exercises cross-page annotation aggregation.
    let page_b = {
        let mut b = RdfDatasetBuilder::new();
        let r = b.intern_iri("http://example.org/r");
        let source = b.intern_iri("http://example.org/source");
        let doc = b.intern_iri("http://example.org/doc");
        b.push_annotation(r, source, doc);
        b.freeze().expect("page b")
    };

    let provider = Arc::new(InMemoryPageProvider::new(vec![page_a, page_b]));
    let paged = PagedDataset::from_provider(provider).expect("seal pages");

    // Capabilities are the honest OR of the pages, and the reifier/annotation methods
    // actually surface what those capabilities advertise.
    let caps = paged.capabilities();
    assert!(caps.reifiers, "page A has a reifier binding");
    assert!(caps.annotations, "both pages carry annotations");
    assert!(caps.quoted_triples, "the reifier binds a triple term");

    // The single reifier binding surfaces as one virtual `rdf:reifies` quad.
    let reifier_quads: Vec<_> = paged.reifier_quads().collect();
    assert_eq!(reifier_quads.len(), 1, "exactly one reifier binding");
    // Its object is the triple term <<(s p o)>>.
    assert_eq!(
        to_value(&paged, reifier_quads[0].o),
        TermValue::Triple {
            s: Box::new(iri("s")),
            p: Box::new(iri("p")),
            o: Box::new(iri("o")),
        },
    );

    // Both annotations aggregate across the two pages for the shared reifier `r`.
    let r_global = paged
        .term_id_by_value(&iri("r"))
        .expect("reifier r interned");
    let mut annos: Vec<(TermValue, TermValue)> = paged
        .annotations_of_with_graph(r_global)
        .map(|(p, o, _g)| (to_value(&paged, p), to_value(&paged, o)))
        .collect();
    annos.sort_by_key(|(p, o)| format!("{p:?}{o:?}"));
    assert_eq!(
        annos,
        vec![
            (iri("confidence"), iri("high")),
            (iri("source"), iri("doc")),
        ],
        "annotations from both pages compose for the one reifier"
    );
    // `annotation_quads` sees the same two annotations as virtual quads.
    assert_eq!(paged.annotation_quads().count(), 2);
}

// ── Freeze / disjointness refusal (G3) ─────────────────────────────────────────

#[test]
fn freeze_refuses_non_quad_disjoint_pages() {
    // Two pages that carry the SAME triple via the SAME term values → after mapping
    // to the shared GlobalTermId space they are the SAME global quad, so the seal MUST
    // refuse (never silently dedup).
    let shared = (iri("s"), iri("p"), iri("o"));
    let page0 = build_page(std::slice::from_ref(&shared));
    let page1 = build_page(std::slice::from_ref(&shared));
    let provider = Arc::new(InMemoryPageProvider::new(vec![page0, page1]));
    let err = PagedDataset::from_provider(provider).expect_err("overlapping pages must refuse");
    match err {
        PagedFreezeError::QuadOverlap(o) => {
            assert_eq!(o.first_page, PageId(0), "the earlier page is named");
            assert_eq!(o.second_page, PageId(1), "the later page is named");
            assert_eq!(o.table, PagedQuadTable::Primary, "a base-quad overlap");
            assert_eq!(o.subject, iri("s"));
            assert_eq!(o.predicate, iri("p"));
            assert_eq!(o.object, iri("o"));
            assert_eq!(o.graph, None, "default-graph quad");
        }
        PagedFreezeError::Page(fault) => panic!("expected QuadOverlap, got page fault: {fault}"),
        other => panic!("expected QuadOverlap, got: {other}"),
    }

    // The disjoint construction (distinct global quads) is the normal path and Oks.
    let ok_pages = vec![
        build_page(&[(iri("s"), iri("p"), iri("o"))]),
        build_page(&[(iri("s2"), iri("p"), iri("o"))]),
    ];
    let ok_provider = Arc::new(InMemoryPageProvider::new(ok_pages));
    let paged = PagedDataset::from_provider(ok_provider).expect("disjoint pages seal");
    assert_eq!(paged.page_count(), 2);
    assert_eq!(paged.quads().count(), 2, "both disjoint quads survive");
}

#[test]
fn freeze_refuses_non_disjoint_side_tables() {
    // The reifier and annotation side tables are composed across pages the SAME way as
    // primary quads — concatenated with no cross-page dedup — so the seal must enforce
    // disjointness on them too, not just the primary quads. Both pages below carry
    // DISJOINT primary quads (so the primary ledger stays clean) but a DUPLICATE
    // side-table entry, which must be refused and attributed to the right stream.

    // (1) Annotation overlap: the shared `(r, confidence, high)` annotation appears on
    // both pages; the composed annotation stream would emit it twice.
    let anno_page = |s: &str, o: &str| {
        let mut b = RdfDatasetBuilder::new();
        let subj = b.intern_iri(&format!("http://example.org/{s}"));
        let p = b.intern_iri("http://example.org/p");
        let obj = b.intern_iri(&format!("http://example.org/{o}"));
        b.push_quad(subj, p, obj, None); // a page-unique primary quad
        let r = b.intern_iri("http://example.org/r");
        let conf = b.intern_iri("http://example.org/confidence");
        let high = b.intern_iri("http://example.org/high");
        b.push_annotation(r, conf, high); // the SHARED annotation
        b.freeze().expect("annotation page")
    };
    let provider = Arc::new(InMemoryPageProvider::new(vec![
        anno_page("s0", "o0"),
        anno_page("s1", "o1"),
    ]));
    match PagedDataset::from_provider(provider)
        .expect_err("duplicate annotation across pages must refuse")
    {
        PagedFreezeError::QuadOverlap(o) => {
            assert_eq!(
                o.table,
                PagedQuadTable::Annotation,
                "annotation-table overlap"
            );
            assert_eq!(o.first_page, PageId(0));
            assert_eq!(o.second_page, PageId(1));
            assert_eq!(o.subject, iri("r"));
            assert_eq!(o.predicate, iri("confidence"));
            assert_eq!(o.object, iri("high"));
        }
        PagedFreezeError::Page(fault) => panic!("expected QuadOverlap, got page fault: {fault}"),
        other => panic!("expected QuadOverlap, got: {other}"),
    }

    // (2) Reifier overlap: the shared reifier binding `r rdf:reifies <<(a b c)>>` (over
    // an unasserted triple term) appears on both pages; the composed reifier stream
    // would emit it twice.
    let reifier_page = |s: &str, o: &str| {
        let mut b = RdfDatasetBuilder::new();
        let subj = b.intern_iri(&format!("http://example.org/{s}"));
        let p = b.intern_iri("http://example.org/p");
        let obj = b.intern_iri(&format!("http://example.org/{o}"));
        b.push_quad(subj, p, obj, None); // a page-unique primary quad
        let a = b.intern_iri("http://example.org/a");
        let bb = b.intern_iri("http://example.org/b");
        let c = b.intern_iri("http://example.org/c");
        let triple = b.intern_triple(a, bb, c);
        let r = b.intern_iri("http://example.org/r");
        b.push_reifier(r, triple); // the SHARED reifier binding
        b.freeze().expect("reifier page")
    };
    let provider = Arc::new(InMemoryPageProvider::new(vec![
        reifier_page("s0", "o0"),
        reifier_page("s1", "o1"),
    ]));
    match PagedDataset::from_provider(provider)
        .expect_err("duplicate reifier binding across pages must refuse")
    {
        PagedFreezeError::QuadOverlap(o) => {
            assert_eq!(o.table, PagedQuadTable::Reifier, "reifier-table overlap");
            assert_eq!(o.first_page, PageId(0));
            assert_eq!(o.second_page, PageId(1));
            assert_eq!(o.subject, iri("r"));
            assert_eq!(
                o.object,
                TermValue::Triple {
                    s: Box::new(iri("a")),
                    p: Box::new(iri("b")),
                    o: Box::new(iri("c")),
                },
                "the reified triple term is the shared object"
            );
        }
        PagedFreezeError::Page(fault) => panic!("expected QuadOverlap, got page fault: {fault}"),
        other => panic!("expected QuadOverlap, got: {other}"),
    }
}

// ── Compaction: dead-id reclaim + determinism ──────────────────────────────────

/// A three-page corpus where page 2's terms (`dave`, `likes`, `eve`) are UNIQUE to it,
/// so dropping page 2 makes exactly those three ids dead.
fn reclaim_pages() -> Vec<Arc<RdfDataset>> {
    vec![
        build_page(&[(iri("alice"), iri("knows"), iri("bob"))]),
        build_page(&[(iri("bob"), iri("knows"), iri("carol"))]),
        build_page(&[(iri("dave"), iri("likes"), iri("eve"))]),
    ]
}

#[test]
fn compact_reclaims_dead_ids_deterministically() {
    let provider = Arc::new(InMemoryPageProvider::new(reclaim_pages()));
    let full = PagedDataset::from_provider(provider).expect("seal pages");

    // Seven distinct IRIs across the three pages.
    let full_len = full.dictionary().len();
    assert_eq!(full_len, 7, "alice knows bob carol dave likes eve");

    // Evict page 2. Its three unique terms are now DEAD but the dictionary still
    // carries them (len unchanged) — reclaim only happens at compaction.
    let dropped = full.drop_page(PageId(2));
    assert_eq!(dropped.page_count(), 2);
    assert_eq!(
        dropped.dictionary().len(),
        full_len,
        "dropping a page does NOT reclaim ids"
    );
    // The dead terms are still resolvable by value in the oversized dictionary.
    for dead in [iri("dave"), iri("likes"), iri("eve")] {
        assert!(
            dropped.term_id_by_value(&dead).is_some(),
            "dead term {dead:?} is retained before compaction"
        );
    }

    // Compact: the three dead ids are reclaimed, so len shrinks by EXACTLY three.
    let compacted = dropped.compact();
    assert_eq!(
        compacted.dictionary().len(),
        full_len - 3,
        "compaction reclaims exactly the 3 terms unique to the dropped page"
    );
    for reclaimed in [iri("dave"), iri("likes"), iri("eve")] {
        assert!(
            compacted.term_id_by_value(&reclaimed).is_none(),
            "reclaimed term {reclaimed:?} is gone after compaction"
        );
    }

    // Meaning is preserved: every surviving quad resolves to identical TermValues.
    assert_eq!(
        collect_rows(&compacted),
        vec![
            vec![
                iri("alice"),
                iri("knows"),
                iri("bob"),
                TermValue::iri("urn:default-graph")
            ],
            vec![
                iri("bob"),
                iri("knows"),
                iri("carol"),
                TermValue::iri("urn:default-graph")
            ],
        ],
        "compaction preserves the surviving quads by value"
    );

    // Determinism: compacting the SAME live set twice assigns IDENTICAL GlobalTermIds
    // to every survivor (a pure function of the live term-value set). Compare the id
    // of every live value under two independent compactions.
    let compacted_again = dropped.compact();
    assert_eq!(
        compacted.dictionary().len(),
        compacted_again.dictionary().len()
    );
    for value in [iri("alice"), iri("knows"), iri("bob"), iri("carol")] {
        assert_eq!(
            compacted.term_id_by_value(&value),
            compacted_again.term_id_by_value(&value),
            "value {value:?} must get the same GlobalTermId across compactions"
        );
    }
    // And the whole id→value mapping is identical index-for-index.
    for i in 0..compacted.dictionary().len() {
        let id = purrdf_core::GlobalTermId::from_index(u64::try_from(i).expect("fits u64"));
        assert_eq!(
            to_value(&compacted, id),
            to_value(&compacted_again, id),
            "id {i} resolves to the same value across compactions"
        );
    }
}

// ── Serialization equivalence (determinism vs a single dataset) ─────────────────

/// Materialize a paged dataset's quads (by value) into a fresh single `RdfDataset`.
fn materialize_to_dataset(paged: &PagedDataset) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    for q in paged.quads() {
        let s = intern_value(&mut b, &to_value(paged, q.s));
        let p = intern_value(&mut b, &to_value(paged, q.p));
        let o = intern_value(&mut b, &to_value(paged, q.o));
        let g = q.g.map(|g| intern_value(&mut b, &to_value(paged, g)));
        b.push_quad(s, p, o, g);
    }
    b.freeze().expect("materialized freeze")
}

#[test]
fn compacted_paged_serializes_byte_identical_to_single() {
    let corpus = parity_corpus();

    // The single-dataset reference and its canonical Turtle.
    let single = build_page(&corpus);
    let single_ttl = render_canonical_turtle(&single, &[]);

    // The paged view over the SAME triples, split across 3 pages, then COMPACTED
    // (a canonical renumber). Materialize its quads back into one RdfDataset and
    // serialize — the honest determinism check (there is no standalone paged
    // serializer; proving the materialized-equivalent is byte-identical is the point).
    let pages = split_pages(&corpus, 3);
    let provider = Arc::new(InMemoryPageProvider::new(pages));
    let paged = PagedDataset::from_provider(provider).expect("seal pages");
    let compacted = paged.compact();
    let materialized = materialize_to_dataset(&compacted);
    let paged_ttl = render_canonical_turtle(&materialized, &[]);

    assert_eq!(
        single_ttl, paged_ttl,
        "compacted-then-materialized paged dataset must serialize byte-identically"
    );
}

// ── from_parts: warm restart without the eager re-scan ──────────────────────────

#[test]
fn from_parts_reconstitutes_without_materializing_pages() {
    // Seal a 3-page dataset the EAGER way (from_provider materializes every page once),
    // then decompose it into its persisted parts.
    let corpus = parity_corpus();
    let raw = split_pages(&corpus, 3);
    let eager = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(raw.clone())))
        .expect("seal pages");
    let (dictionary, generation, parts) = eager.to_parts();
    assert_eq!(parts.len(), 3, "one part per page");

    // Rebuild from those parts over a COUNTING provider serving the SAME page contents.
    // The warm-restart path must materialize NO page at construction — that is the whole
    // point (an already-indexed store reloads without re-scanning).
    let p0 = raw[0].clone();
    let p1 = raw[1].clone();
    let p2 = raw[2].clone();
    let counting = Arc::new(CountingDemandProvider::new(vec![
        Box::new(move || p0.clone()),
        Box::new(move || p1.clone()),
        Box::new(move || p2.clone()),
    ]));
    let warm = PagedDataset::from_parts(
        dictionary,
        counting.clone() as Arc<dyn PageProvider>,
        generation,
        parts,
    )
    .expect("matching warm snapshot");
    assert_eq!(
        counting.hits(),
        0,
        "from_parts must not materialize any page (unlike the eager from_provider seal)"
    );
    assert_eq!(warm.page_count(), 3);

    // The reconstituted dataset is byte-identical to the eagerly-sealed one, and it
    // genuinely serves reads — which DO now pull pages lazily through the provider.
    assert_eq!(
        collect_rows(&eager),
        collect_rows(&warm),
        "from_parts yields the same rows as from_provider"
    );
    assert!(
        counting.hits() > 0,
        "reads materialize pages lazily after construction"
    );
}

// ── verify_parts: certifying a warm-restart index against summary drift ─────────

/// A single-page fixture with two variants sharing everything the CHEAP admission
/// checks compare — term count, quad count, the deterministic reference byte
/// charge, and capabilities — but disagreeing on which named graph one quad lands
/// in:
///
/// - `honest`: `alice knows bob` AND `bob knows carol` both in graph `g1`;
///   `g2` is declared but carries zero rows.
/// - `drifted`: `alice knows bob` stays in `g1`; `bob knows carol` moves to `g2`.
///
/// Both variants intern the identical six terms (`alice`, `bob`, `carol`, `knows`,
/// `g1`, `g2`) and carry two base quads, so `term_count`, `quad_count`, the
/// logical byte charge (a pure function of the term table and total row counts),
/// and `capabilities` (`named_graphs: true` either way) are all identical between
/// them — the existing O(1)/O(term_count) admission checks cannot tell them apart.
/// Only the PER-GRAPH row split differs: `g1`/`g2` base rows are `(2, 0)` honest
/// versus `(1, 1)` drifted, which is exactly the shape `PageSummary` tracks and the
/// graph-pruning index is built from.
fn graph_drift_fixture() -> (Arc<RdfDataset>, Arc<RdfDataset>, TermValue, TermValue) {
    let g1 = iri("g1");
    let g2 = iri("g2");

    let honest = {
        let mut b = RdfDatasetBuilder::new();
        let alice = b.intern_iri("http://example.org/alice");
        let bob = b.intern_iri("http://example.org/bob");
        let carol = b.intern_iri("http://example.org/carol");
        let knows = b.intern_iri("http://example.org/knows");
        let g1_id = intern_value(&mut b, &g1);
        let g2_id = intern_value(&mut b, &g2);
        b.push_quad(alice, knows, bob, Some(g1_id));
        b.push_quad(bob, knows, carol, Some(g1_id));
        b.declare_named_graph(g2_id);
        b.freeze().expect("honest page freeze")
    };

    let drifted = {
        let mut b = RdfDatasetBuilder::new();
        let alice = b.intern_iri("http://example.org/alice");
        let bob = b.intern_iri("http://example.org/bob");
        let carol = b.intern_iri("http://example.org/carol");
        let knows = b.intern_iri("http://example.org/knows");
        let g1_id = intern_value(&mut b, &g1);
        let g2_id = intern_value(&mut b, &g2);
        b.push_quad(alice, knows, bob, Some(g1_id));
        // Moved from g1 (honest) to g2: same terms, same quad, different graph.
        b.push_quad(bob, knows, carol, Some(g2_id));
        b.freeze().expect("drifted page freeze")
    };

    (honest, drifted, g1, g2)
}

/// Materializes the HONEST fixture content on the first call and the DRIFTED
/// content on every later call — modeling a persisted backend whose stored bytes
/// were correct when a warm-restart index was originally sealed, but have since
/// drifted underneath it. Used to certify [`PagedDataset::verify_parts`], which is
/// the only thing that re-materializes a page regardless of what pruning would
/// have decided.
struct GraphDriftProvider {
    honest: Arc<RdfDataset>,
    drifted: Arc<RdfDataset>,
    generation: PageGeneration,
    calls: AtomicUsize,
}

impl PageProvider for GraphDriftProvider {
    fn page_count(&self) -> usize {
        1
    }

    fn generation(&self) -> PageGeneration {
        self.generation
    }

    fn materialize(&self, page: PageId) -> Result<PageMaterialization, PageFault> {
        if page != PageId(0) {
            return Err(PageFault::provider(page, "page out of range"));
        }
        let call = self.calls.fetch_add(1, Ordering::Relaxed);
        let dataset = if call == 0 {
            self.honest.clone()
        } else {
            self.drifted.clone()
        };
        // The deterministic reference charge is a pure function of the term table
        // and total row counts, both identical between the two variants, so this
        // never itself trips the cheap byte-charge check.
        Ok(PageMaterialization::in_memory(dataset, self.generation))
    }
}

/// `to_parts` → `from_parts` → `verify_parts()` over an HONEST provider (content
/// never drifts) must certify cleanly: this is the ordinary warm-restart case
/// `verify_parts` exists to let a caller prove, once, out of band.
#[test]
fn verify_parts_accepts_an_honest_warm_restart() {
    let (honest, _drifted, _g1, _g2) = graph_drift_fixture();
    let generation = PageGeneration(3);
    let provider = Arc::new(InMemoryPageProvider::with_generation(
        vec![honest],
        generation,
    ));
    let eager = PagedDataset::from_provider(provider.clone() as Arc<dyn PageProvider>)
        .expect("seal honest page");
    let (dictionary, sealed_generation, parts) = eager.to_parts();

    let warm = PagedDataset::from_parts(
        dictionary,
        provider as Arc<dyn PageProvider>,
        sealed_generation,
        parts,
    )
    .expect("matching warm snapshot");

    warm.verify_parts()
        .expect("an honest warm restart must certify cleanly");
}

/// `to_parts` → `from_parts` → `verify_parts()` over the drifting provider must
/// name the exact page whose materialized content no longer matches the summary
/// it was sealed with. This is the failure mode no admission-time check can ever
/// observe: `PagedDataset::from_provider`'s own seal pass reads the page ONCE (the
/// honest content, call 0) to build the summary the warm-restart metadata then
/// carries forward unchanged, and ordinary reads only re-materialize pages the
/// pruning law actually admits — `verify_parts` is the only path that reads every
/// page unconditionally and can therefore witness the drift.
#[test]
fn verify_parts_rejects_a_summary_that_disagrees_with_page_content() {
    let (honest, drifted, _g1, _g2) = graph_drift_fixture();
    let generation = PageGeneration(3);
    let provider = Arc::new(GraphDriftProvider {
        honest,
        drifted,
        generation,
        calls: AtomicUsize::new(0),
    });
    let eager = PagedDataset::from_provider(provider.clone() as Arc<dyn PageProvider>)
        .expect("seal honest page (call 0)");
    let (dictionary, sealed_generation, parts) = eager.to_parts();

    let warm = PagedDataset::from_parts(
        dictionary,
        provider as Arc<dyn PageProvider>,
        sealed_generation,
        parts,
    )
    .expect("matching warm snapshot");

    let error = warm
        .verify_parts()
        .expect_err("drifted content must not certify");
    match error {
        PagedFreezeError::SummaryDrift {
            page: PageId(0), ..
        } => {}
        other => panic!("expected a typed summary-drift error naming page 0, got: {other}"),
    }
}

/// The neighbouring valid case the repo's over-refusal rule requires: the SAME
/// fixture, served HONESTLY (content never drifts), must still complete ordinary
/// reads and return exactly the right rows — the debug-only full-summary
/// re-derive added to the hot admission path (`PagedDataset::page`) must never
/// refuse, nor alter the result of, a legitimate page.
#[test]
fn graph_drift_fixture_with_an_honest_provider_still_returns_the_right_rows() {
    let (honest, _drifted, g1, g2) = graph_drift_fixture();
    let provider = Arc::new(InMemoryPageProvider::new(vec![honest]));
    let paged = PagedDataset::from_provider(provider).expect("seal honest page");

    let g1_id = paged.term_id_by_value(&g1).expect("g1 interned");
    let g2_id = paged.term_id_by_value(&g2).expect("g2 interned");

    let mut g1_rows: Vec<_> = paged
        .quads_for_pattern(None, None, None, GraphMatch::Named(g1_id))
        .map(|q| (to_value(&paged, q.s), to_value(&paged, q.o)))
        .collect();
    g1_rows.sort_by_key(row_key_pair);
    assert_eq!(
        g1_rows,
        vec![(iri("alice"), iri("bob")), (iri("bob"), iri("carol")),],
        "g1 genuinely owns both base rows"
    );

    assert!(
        paged
            .quads_for_pattern(None, None, None, GraphMatch::Named(g2_id))
            .next()
            .is_none(),
        "g2 is declared but owns no base rows"
    );

    paged
        .verify_parts()
        .expect("an honest, undecomposed dataset must also certify cleanly");
}

/// A deterministic sort key for a `(TermValue, TermValue)` pair (mirrors `row_key`
/// for the single-value case).
fn row_key_pair(pair: &(TermValue, TermValue)) -> String {
    format!("{pair:?}")
}

struct MismatchedGenerationProvider {
    page: Arc<RdfDataset>,
    current: PageGeneration,
    materialized: PageGeneration,
}

impl PageProvider for MismatchedGenerationProvider {
    fn page_count(&self) -> usize {
        1
    }

    fn generation(&self) -> PageGeneration {
        self.current
    }

    fn materialize(&self, page: PageId) -> Result<PageMaterialization, PageFault> {
        if page != PageId(0) {
            return Err(PageFault::provider(page, "page out of range"));
        }
        Ok(PageMaterialization::new(
            self.page.clone(),
            self.materialized,
            17,
        ))
    }
}

#[test]
fn seal_rejects_a_typed_generation_mismatch() {
    let provider = Arc::new(MismatchedGenerationProvider {
        page: build_page(&[(iri("s"), iri("p"), iri("o"))]),
        current: PageGeneration(7),
        materialized: PageGeneration(8),
    });
    let error = PagedDataset::from_provider(provider).expect_err("stale page must not seal");
    match error {
        PagedFreezeError::Page(PageFault {
            page: PageId(0),
            kind:
                PageFaultKind::StaleGeneration {
                    expected: PageGeneration(7),
                    actual: PageGeneration(8),
                },
            ..
        }) => {}
        other => panic!("expected a typed stale-generation page fault, got: {other}"),
    }
}

#[test]
fn warm_restart_rejects_generation_and_page_count_mismatches() {
    let pages = vec![
        build_page(&[(iri("s0"), iri("p"), iri("o0"))]),
        build_page(&[(iri("s1"), iri("p"), iri("o1"))]),
    ];
    let generation = PageGeneration(12);
    let eager = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::with_byte_lengths(
        vec![(pages[0].clone(), 31), (pages[1].clone(), 47)],
        generation,
    )))
    .expect("seal certified pages");
    let (dictionary, certified_generation, parts) = eager.to_parts();

    let stale_provider = Arc::new(InMemoryPageProvider::with_generation(
        pages.clone(),
        PageGeneration(13),
    ));
    let stale = PagedDataset::from_parts(
        dictionary.clone(),
        stale_provider,
        certified_generation,
        parts.clone(),
    )
    .expect_err("warm metadata from another generation must be rejected");
    assert!(matches!(
        stale,
        PagedFreezeError::GenerationMismatch {
            expected: PageGeneration(12),
            actual: PageGeneration(13)
        }
    ));

    let short_provider = Arc::new(InMemoryPageProvider::with_generation(
        vec![pages[0].clone()],
        generation,
    ));
    let short = PagedDataset::from_parts(dictionary, short_provider, certified_generation, parts)
        .expect_err("warm metadata with a different page count must be rejected");
    assert!(matches!(
        short,
        PagedFreezeError::PageCountMismatch {
            metadata: 2,
            provider: 1
        }
    ));
}

#[test]
fn snapshot_and_byte_metadata_survive_selection_and_compaction() {
    let generation = PageGeneration(23);
    let pages = reclaim_pages();
    let paged = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::with_byte_lengths(
        vec![
            (pages[0].clone(), 101),
            (pages[1].clone(), 202),
            (pages[2].clone(), 303),
        ],
        generation,
    )))
    .expect("seal metadata-bearing pages");
    let (_, sealed_generation, sealed_parts) = paged.to_parts();
    assert_eq!(sealed_generation, generation);
    assert_eq!(
        sealed_parts
            .iter()
            .map(|part| part.byte_len)
            .collect::<Vec<_>>(),
        vec![101, 202, 303]
    );

    let selected = paged.with_pages(&[PageId(2), PageId(0)]);
    let (_, selected_generation, selected_parts) = selected.to_parts();
    assert_eq!(selected_generation, generation);
    assert_eq!(
        selected_parts
            .iter()
            .map(|part| part.byte_len)
            .collect::<Vec<_>>(),
        vec![303, 101]
    );

    let compacted = selected.compact();
    let (_, compacted_generation, compacted_parts) = compacted.to_parts();
    assert_eq!(compacted_generation, generation);
    assert_eq!(
        compacted_parts
            .iter()
            .map(|part| part.byte_len)
            .collect::<Vec<_>>(),
        vec![303, 101]
    );
    assert_eq!(compacted.quads().count(), 2, "metadata remains usable");
}

#[test]
fn provider_fault_categories_remain_distinct() {
    let page = PageId(4);
    let expected = PageGeneration(1);
    let actual = PageGeneration(2);
    assert_eq!(
        PageFault::provider(page, "I/O").kind,
        PageFaultKind::Provider
    );
    assert_eq!(
        PageFault::stale_generation(page, expected, actual).kind,
        PageFaultKind::StaleGeneration { expected, actual }
    );
    assert_eq!(
        PageFault::cancelled(page, "cancelled by host").kind,
        PageFaultKind::Stopped(StopCause::Cancelled)
    );
    assert_eq!(
        PageFault::deadline_exceeded(page, "host deadline elapsed").kind,
        PageFaultKind::Stopped(StopCause::Deadline)
    );
    assert_eq!(
        PageFault::invalid_data(page, "corrupt payload").kind,
        PageFaultKind::InvalidData
    );
}

/// The paged dataset and its provider must be thread-shareable.
#[test]
fn paged_dataset_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PagedDataset>();
    assert_send_sync::<Arc<dyn PageProvider>>();
    assert_send_sync::<CountingDemandProvider>();
    assert_send_sync::<InMemoryPageProvider>();
}

/// Build a 3-page `PagedDataset` whose named-graph membership can ONLY be recovered
/// completely by consulting each page's declared-graph list and side tables, not just
/// its base quads:
/// * page 0 — a base quad in named graph `gA`.
/// * page 1 — graph `gEmpty` declared but carrying no rows at all (a page-local
///   `RdfDatasetBuilder::declare_named_graph` with no quad/reifier/annotation in it).
/// * page 2 — graph `gReifierOnly` named ONLY by a reifier side-table row's graph slot
///   (no base quad ever names it).
fn named_graphs_fixture() -> (Vec<Arc<RdfDataset>>, TermValue, TermValue, TermValue) {
    let ga = iri("gA");
    let g_empty = iri("gEmpty");
    let g_reifier_only = iri("gReifierOnly");

    let page0 = {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s0");
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o0");
        let ga_id = intern_value(&mut b, &ga);
        b.push_quad(s, p, o, Some(ga_id));
        b.freeze().expect("page0 freeze")
    };
    let page1 = {
        let mut b = RdfDatasetBuilder::new();
        let g_empty_id = intern_value(&mut b, &g_empty);
        b.declare_named_graph(g_empty_id);
        b.freeze().expect("page1 freeze")
    };
    let page2 = {
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri("http://example.org/a");
        let bb = b.intern_iri("http://example.org/b");
        let c = b.intern_iri("http://example.org/c");
        let triple = b.intern_triple(a, bb, c);
        let r = b.intern_iri("http://example.org/r");
        let g_reifier_only_id = intern_value(&mut b, &g_reifier_only);
        b.push_reifier_in_graph(r, triple, Some(g_reifier_only_id));
        b.freeze().expect("page2 freeze")
    };

    (vec![page0, page1, page2], ga, g_empty, g_reifier_only)
}

/// `DatasetView::named_graphs` on a `PagedQueryView` charges ZERO pages: the answer
/// comes entirely from `GraphPageIndex::keys`, which is folded from each page's
/// already-sealed `PageSummary`, never from a materialized page. The set it returns
/// must also be COMPLETE — it must include a declared-empty graph and a graph named
/// only by a reifier row, not just graphs with base quads.
#[test]
fn named_graphs_costs_no_pages() {
    let (pages, ga, g_empty, g_reifier_only) = named_graphs_fixture();
    let provider = Arc::new(InMemoryPageProvider::new(pages));
    let paged = PagedDataset::from_provider(provider).expect("seal pages");

    // A zero/zero budget: any real page materialization would fail immediately.
    let view = paged.query_view(PagedQueryLimits::new(0, 0));

    let ids: Vec<_> = DatasetView::named_graphs(&view).collect();
    assert!(
        ids.is_sorted(),
        "named_graphs must yield ascending GlobalTermId order"
    );
    let values: std::collections::BTreeSet<String> = ids
        .iter()
        .map(|&id| format!("{:?}", to_value(&view, id)))
        .collect();
    assert_eq!(
        values,
        std::collections::BTreeSet::from([
            format!("{ga:?}"),
            format!("{g_empty:?}"),
            format!("{g_reifier_only:?}"),
        ]),
        "named_graphs must include the declared-empty graph and the reifier-only graph"
    );

    match view.operation_status() {
        ViewOperationStatus::Ready { evidence } => {
            assert_eq!(
                evidence.requested_pages,
                Vec::new(),
                "reading graph metadata must request no page"
            );
            assert_eq!(
                evidence.consumed_pages, 0,
                "reading graph metadata must consume no page"
            );
        }
        ViewOperationStatus::Failed { error, .. } => {
            panic!("named_graphs must not fail a zero-budget view: {error}")
        }
    }
}

/// The paged `named_graphs()` set, resolved to `TermValue`s, must equal the
/// `named_graphs()` set of a single merged `RdfDataset` built from the SAME content
/// (declared-empty graph and reifier-only graph included) — the parity this override
/// exists to restore between the paged surfaces and `RdfDataset`.
#[test]
fn paged_named_graphs_match_a_single_merged_dataset() {
    let (pages, ga, g_empty, g_reifier_only) = named_graphs_fixture();
    let provider = Arc::new(InMemoryPageProvider::new(pages));
    let paged = PagedDataset::from_provider(provider).expect("seal pages");

    let paged_values: std::collections::BTreeSet<String> = paged
        .named_graphs()
        .map(|id| format!("{:?}", to_value(&paged, id)))
        .collect();

    // The single merged reference dataset: same quad, same declared-empty graph, same
    // reifier-only graph, built directly (not derived from the pages).
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s0");
    let p = b.intern_iri("http://example.org/p");
    let o = b.intern_iri("http://example.org/o0");
    let ga_id = intern_value(&mut b, &ga);
    b.push_quad(s, p, o, Some(ga_id));
    let g_empty_id = intern_value(&mut b, &g_empty);
    b.declare_named_graph(g_empty_id);
    let a = b.intern_iri("http://example.org/a");
    let bb = b.intern_iri("http://example.org/b");
    let c = b.intern_iri("http://example.org/c");
    let triple = b.intern_triple(a, bb, c);
    let r = b.intern_iri("http://example.org/r");
    let g_reifier_only_id = intern_value(&mut b, &g_reifier_only);
    b.push_reifier_in_graph(r, triple, Some(g_reifier_only_id));
    let single = b.freeze().expect("single dataset freeze");

    let single_values: std::collections::BTreeSet<String> = single
        .named_graphs()
        .map(|id| format!("{:?}", to_value(&*single, id)))
        .collect();

    assert_eq!(
        paged_values, single_values,
        "paged named_graphs() must match a single merged RdfDataset's named_graphs()"
    );
    assert_eq!(
        paged_values,
        std::collections::BTreeSet::from([
            format!("{ga:?}"),
            format!("{g_empty:?}"),
            format!("{g_reifier_only:?}"),
        ]),
        "sanity: the expected three graphs are present"
    );
}

// ── `reifier_quads_in_graph` / `annotation_quads_in_graph` narrowing (Task 6b) ──

/// Populate `b` with page A's content: a reifier row + an annotation row in a named
/// graph `g_owns` that genuinely owns rows, a SECOND reifier row + annotation row in
/// the DEFAULT graph, and a named graph `g_none` declared but carrying nothing.
fn populate_graph_narrow_page_a(b: &mut RdfDatasetBuilder, g_owns: &TermValue, g_none: &TermValue) {
    let s = b.intern_iri("http://example.org/s");
    let p = b.intern_iri("http://example.org/p");
    let o = b.intern_iri("http://example.org/o");
    b.push_quad(s, p, o, None);

    let triple1 = b.intern_triple(s, p, o);
    let r1 = b.intern_iri("http://example.org/r1");
    let g_owns_id = intern_value(b, g_owns);
    b.push_reifier_in_graph(r1, triple1, Some(g_owns_id));
    let conf = b.intern_iri("http://example.org/confidence");
    let high = b.intern_iri("http://example.org/high");
    b.push_annotation_in_graph(r1, conf, high, Some(g_owns_id));

    let triple2 = b.intern_triple(o, p, s);
    let r2 = b.intern_iri("http://example.org/r2");
    b.push_reifier_in_graph(r2, triple2, None);
    let source = b.intern_iri("http://example.org/source");
    let doc = b.intern_iri("http://example.org/doc");
    b.push_annotation_in_graph(r2, source, doc, None);

    let g_none_id = intern_value(b, g_none);
    b.declare_named_graph(g_none_id);
}

/// Populate `b` with page B's content: a reifier row + an annotation row in a
/// DIFFERENT named graph `g_other`, so `g_owns`'s postings must not pick this page up,
/// while `GraphMatch::Any` still must.
fn populate_graph_narrow_page_b(b: &mut RdfDatasetBuilder, g_other: &TermValue) {
    let a = b.intern_iri("http://example.org/a");
    let bb = b.intern_iri("http://example.org/b");
    let c = b.intern_iri("http://example.org/c");
    let triple3 = b.intern_triple(a, bb, c);
    let r3 = b.intern_iri("http://example.org/r3");
    let g_other_id = intern_value(b, g_other);
    b.push_reifier_in_graph(r3, triple3, Some(g_other_id));
    let p3 = b.intern_iri("http://example.org/p3");
    let o3 = b.intern_iri("http://example.org/o3");
    b.push_annotation_in_graph(r3, p3, o3, Some(g_other_id));
}

/// Assert, for every graph constraint in `graphs`, that
/// `reifier_quads_in_graph(g)`/`annotation_quads_in_graph(g)` yield EXACTLY
/// `reifier_quads()`/`annotation_quads()` filtered by `g.matches(q.g)`, in the same
/// order — the equivalence [`DatasetView::reifier_quads_in_graph`] and
/// [`DatasetView::annotation_quads_in_graph`] document as their contract.
fn assert_graph_narrow_matches_filter<V: DatasetView>(view: &V, graphs: &[GraphMatch<V::Id>]) {
    for &g in graphs {
        let expected_reifier: Vec<_> = view.reifier_quads().filter(|q| g.matches(q.g)).collect();
        let actual_reifier: Vec<_> = view.reifier_quads_in_graph(g).collect();
        assert_eq!(
            actual_reifier, expected_reifier,
            "reifier_quads_in_graph({g:?}) must equal reifier_quads().filter(|q| g.matches(q.g))"
        );

        let expected_annotation: Vec<_> =
            view.annotation_quads().filter(|q| g.matches(q.g)).collect();
        let actual_annotation: Vec<_> = view.annotation_quads_in_graph(g).collect();
        assert_eq!(
            actual_annotation, expected_annotation,
            "annotation_quads_in_graph({g:?}) must equal \
             annotation_quads().filter(|q| g.matches(q.g))"
        );
    }
}

/// Differential test: on a `PagedDataset`, a `PagedQueryView` over the SAME dataset,
/// and a plain `RdfDataset` holding the same content merged into one builder,
/// `reifier_quads_in_graph`/`annotation_quads_in_graph` must equal the corresponding
/// whole-table stream filtered by `GraphMatch::matches`, for `Any`, `Default`, a graph
/// that owns rows, and a graph that owns none — row order included, not just the set.
#[test]
fn reifier_and_annotation_quads_in_graph_match_the_filtered_whole_table_on_every_backend() {
    let g_owns = iri("gOwns");
    let g_none = iri("gNone");
    let g_other = iri("gOther");

    let page_a = {
        let mut b = RdfDatasetBuilder::new();
        populate_graph_narrow_page_a(&mut b, &g_owns, &g_none);
        b.freeze().expect("page a freeze")
    };
    let page_b = {
        let mut b = RdfDatasetBuilder::new();
        populate_graph_narrow_page_b(&mut b, &g_other);
        b.freeze().expect("page b freeze")
    };

    let provider = Arc::new(InMemoryPageProvider::new(vec![page_a, page_b]));
    let paged = PagedDataset::from_provider(provider).expect("seal pages");
    let query_view = paged.query_view(PagedQueryLimits::UNBOUNDED);

    let g_owns_paged = paged.term_id_by_value(&g_owns).expect("gOwns interned");
    let g_none_paged = paged.term_id_by_value(&g_none).expect("gNone interned");
    let paged_graphs = [
        GraphMatch::Any,
        GraphMatch::Default,
        GraphMatch::Named(g_owns_paged),
        GraphMatch::Named(g_none_paged),
    ];
    assert_graph_narrow_matches_filter(&paged, &paged_graphs);
    assert_graph_narrow_matches_filter(&query_view, &paged_graphs);

    let single = {
        let mut b = RdfDatasetBuilder::new();
        populate_graph_narrow_page_a(&mut b, &g_owns, &g_none);
        populate_graph_narrow_page_b(&mut b, &g_other);
        b.freeze().expect("single freeze")
    };
    let g_owns_single = single.term_id_by_value(&g_owns).expect("gOwns interned");
    let g_none_single = single.term_id_by_value(&g_none).expect("gNone interned");
    let single_graphs = [
        GraphMatch::Any,
        GraphMatch::Default,
        GraphMatch::Named(g_owns_single),
        GraphMatch::Named(g_none_single),
    ];
    assert_graph_narrow_matches_filter(&*single, &single_graphs);

    // Sanity: the fixture is not degenerate — `g_owns` genuinely owns rows and
    // `g_none` genuinely owns none, on every backend.
    assert_eq!(
        paged
            .reifier_quads_in_graph(GraphMatch::Named(g_owns_paged))
            .count(),
        1
    );
    assert_eq!(
        paged
            .reifier_quads_in_graph(GraphMatch::Named(g_none_paged))
            .count(),
        0
    );
    assert_eq!(
        paged
            .annotation_quads_in_graph(GraphMatch::Named(g_owns_paged))
            .count(),
        1
    );
    assert_eq!(
        paged
            .annotation_quads_in_graph(GraphMatch::Named(g_none_paged))
            .count(),
        0
    );
}

/// Part A (Task 6): a page that mentions a reifier term ONLY in its base-quad table
/// (role-agnostic term-table presence via `PageTranslation::to_local` alone would pass
/// it) must be skipped by `reifier_quads_of` — never materialized — while a page that
/// genuinely owns a reifier row for the same term is admitted and still yields it.
#[test]
fn reifier_quads_of_skips_a_page_that_only_mentions_the_term_and_admits_the_owning_page() {
    fn mentions_only_page() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let r = b.intern_iri("http://example.org/r");
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");
        // `r` occurs here ONLY as a base-quad subject — no reifier row names it.
        b.push_quad(r, p, o, None);
        b.freeze().expect("mentions-only page freeze")
    }
    fn owning_page() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri("http://example.org/a");
        let bb = b.intern_iri("http://example.org/b");
        let c = b.intern_iri("http://example.org/c");
        let triple = b.intern_triple(a, bb, c);
        let r = b.intern_iri("http://example.org/r");
        b.push_reifier(r, triple);
        b.freeze().expect("owning page freeze")
    }

    let provider = Arc::new(CountingDemandProvider::new(vec![
        Box::new(mentions_only_page),
        Box::new(owning_page),
    ]));
    let paged =
        PagedDataset::from_provider(provider.clone() as Arc<dyn PageProvider>).expect("seal pages");
    let hits_after_construction = provider.hits();

    let r_global = paged.term_id_by_value(&iri("r")).expect("r interned");
    let rows: Vec<_> = paged.reifier_quads_of(r_global).collect();
    assert_eq!(
        rows.len(),
        1,
        "only the owning page's genuine reifier row is yielded"
    );
    assert_eq!(
        to_value(&paged, rows[0].o),
        TermValue::Triple {
            s: Box::new(iri("a")),
            p: Box::new(iri("b")),
            o: Box::new(iri("c")),
        },
        "the yielded row is the owning page's reifier binding"
    );
    assert_eq!(
        provider.hits(),
        hits_after_construction + 1,
        "the mentions-only page (term-table presence, no reifier row) must never be \
         materialized; only the owning page is"
    );
}
