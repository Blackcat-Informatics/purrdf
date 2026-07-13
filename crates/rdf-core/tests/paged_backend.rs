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

use purrdf_core::{
    CountingDemandProvider, DatasetView, GraphMatch, InMemoryPageProvider, PageId, PagedDataset,
    PagedFreezeError, PagedQuadTable, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermId, TermRef,
    TermValue, render_canonical_turtle,
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
    let paged = PagedDataset::from_provider(provider.clone() as Arc<dyn purrdf_core::PageProvider>)
        .expect("seal pages");

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
    let (dictionary, parts) = eager.to_parts();
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
        counting.clone() as Arc<dyn purrdf_core::PageProvider>,
        parts,
    );
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

/// The paged dataset and its provider must be thread-shareable.
#[test]
fn paged_dataset_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PagedDataset>();
    assert_send_sync::<Arc<dyn purrdf_core::PageProvider>>();
    assert_send_sync::<CountingDemandProvider>();
    assert_send_sync::<InMemoryPageProvider>();
}
