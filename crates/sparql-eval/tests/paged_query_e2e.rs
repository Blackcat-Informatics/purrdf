// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Headline acceptance: a SPARQL query served DIRECTLY by the native
//! evaluator over a multi-page [`PagedDataset`] (the external paged backend), through
//! the engine's public generic entry points ([`NativeSparqlEngine::query_prepared_view`]
//! / [`NativeSparqlEngine::explain_query_view`]).
//!
//! Three falsifiable guards, each exercising the production query surface:
//!
//! * **Test A — result parity (byte-identical).** The SAME prepared query (a
//!   three-pattern BGP + `FILTER` + `ORDER BY`) evaluated over a single frozen
//!   `RdfDataset` and over a `PagedDataset` split across three pages — where the
//!   shared join variable `?x` binds a term (`:bob`, `:frank`, `:heidi`) whose
//!   `knows`/`name`/`age` triples live on DIFFERENT pages — yields the exact same
//!   materialized `SparqlResult::Solutions` (`variables` and `rows` equal). This is
//!   the proof the paged backend is served directly and unifies terms across pages.
//! * **Test B — cross-page join order is cost-driven.** Two paged fixtures with
//!   DELIBERATELY INVERTED cross-page cardinality skew for a two-pattern BGP. The
//!   planner's explained probe order (via `explain_query_view`) puts the low-Σ-per-page
//!   pattern FIRST, and the order FLIPS when the skew inverts. Result-equality alone is
//!   invariant under `ORDER BY`, so it cannot detect a broken cross-page estimator; only
//!   the order FLIP can, and it can only happen if the Σ-per-page cost model is consulted.
//! * **Test C — lazy hook fires only for needed pages.** Over a
//!   `CountingDemandProvider`, a query whose BGP is bound to a subject present on exactly
//!   one page re-materializes exactly that one page (not all pages), and a second
//!   identical run adds no hits (the per-page `OnceLock` cache).

use std::sync::Arc;

use purrdf_core::{
    CountingDemandProvider, DatasetView, GraphMatch, InMemoryPageProvider, PagedDataset,
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlResult, TermId, TermValue,
};
use purrdf_sparql_eval::NativeSparqlEngine;

// ── shared fixture helpers ─────────────────────────────────────────────────────

/// An `example.org` IRI value.
fn iri(name: &str) -> TermValue {
    TermValue::iri(format!("http://example.org/{name}"))
}

/// An `xsd:integer` typed literal (numeric so `FILTER(?age >= 18)` compares by value).
fn xsd_int(n: &str) -> TermValue {
    TermValue::typed_literal(n, "http://www.w3.org/2001/XMLSchema#integer")
}

/// Intern one dataset-independent value into a builder, recursing for triple terms.
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

/// Freeze one page (or the single reference dataset) from `(s, p, o)` triples in the
/// default graph.
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

/// Wrap eagerly-built pages in an in-memory provider and seal them into a
/// `PagedDataset`.
fn paged_over(pages: Vec<Arc<RdfDataset>>) -> PagedDataset {
    let provider = Arc::new(InMemoryPageProvider::new(pages));
    PagedDataset::from_provider(provider).expect("seal pages")
}

/// Destructure a `SparqlResult` into `(variables, rows)`, panicking on any other shape.
fn solutions(result: SparqlResult) -> (Vec<String>, Vec<Vec<Option<TermValue>>>) {
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => (variables, rows),
        other => panic!("expected solutions, got {other:?}"),
    }
}

// ── Test A — direct evaluation, byte-identical result parity ────────────────────

/// Page 0 — the `knows` edges. `:judy` is known but has no `name`/`age`; `:dave` has
/// `name`/`age` but no `knows` edge — both must be excluded by the join.
fn parity_page_knows() -> Vec<Triple> {
    vec![
        (iri("alice"), iri("knows"), iri("bob")),
        (iri("eve"), iri("knows"), iri("frank")),
        (iri("grace"), iri("knows"), iri("heidi")),
        (iri("ivan"), iri("knows"), iri("judy")),
        (iri("alice"), iri("knows"), iri("carol")),
    ]
}

/// Page 1 — the `age` edges (a DIFFERENT page from `knows` and `name`, so a single
/// join binding for `?x` must be assembled across three pages).
fn parity_page_age() -> Vec<Triple> {
    vec![
        (iri("bob"), iri("age"), xsd_int("30")),
        (iri("carol"), iri("age"), xsd_int("17")),
        (iri("frank"), iri("age"), xsd_int("40")),
        (iri("heidi"), iri("age"), xsd_int("19")),
        (iri("dave"), iri("age"), xsd_int("22")),
    ]
}

/// Page 2 — the `name` edges.
fn parity_page_name() -> Vec<Triple> {
    vec![
        (iri("bob"), iri("name"), TermValue::simple_literal("Bob")),
        (
            iri("carol"),
            iri("name"),
            TermValue::simple_literal("Carol"),
        ),
        (
            iri("frank"),
            iri("name"),
            TermValue::simple_literal("Frank"),
        ),
        (
            iri("heidi"),
            iri("name"),
            TermValue::simple_literal("Heidi"),
        ),
        (iri("dave"), iri("name"), TermValue::simple_literal("Dave")),
    ]
}

const PARITY_QUERY: &str = "\
PREFIX ex: <http://example.org/>
SELECT ?x ?name ?age WHERE {
  ?p ex:knows ?x .
  ?x ex:name ?name .
  ?x ex:age ?age .
  FILTER(?age >= 18)
} ORDER BY ?name";

#[test]
fn paged_query_parity_is_byte_identical_to_single_dataset() {
    let corpus: Vec<Triple> = parity_page_knows()
        .into_iter()
        .chain(parity_page_age())
        .chain(parity_page_name())
        .collect();
    assert_eq!(corpus.len(), 15, "the fixture corpus is 15 triples");

    // (1) the single-dataset reference: all 15 triples in one frozen dataset.
    let single = build_page(&corpus);

    // (2) the paged view over the SAME triples, split so each of `knows`/`name`/`age`
    // lives on its OWN page — a join binding for `?x` therefore straddles all three.
    let paged = paged_over(vec![
        build_page(&parity_page_knows()),
        build_page(&parity_page_age()),
        build_page(&parity_page_name()),
    ]);
    assert_eq!(paged.page_count(), 3);

    // ONE prepared query, evaluated via the concrete `query_prepared` (single) and the
    // generic `query_prepared_view` (paged) public engine entry points.
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(PARITY_QUERY, None).expect("prepare");

    let (single_vars, single_rows) = solutions(
        engine
            .query_prepared(&single, &prepared, &[])
            .expect("single query"),
    );
    let (paged_vars, paged_rows) = solutions(
        engine
            .query_prepared_view(&paged, &prepared, &[])
            .expect("paged query"),
    );

    // The join actually crossed page boundaries: bob(30)/frank(40)/heidi(19) survive the
    // FILTER, carol(17) is dropped, judy(no name/age) and dave(no knows edge) never bind.
    // ORDER BY ?name makes the row order total and deterministic.
    assert_eq!(single_vars, vec!["x", "name", "age"], "projected variables");
    assert_eq!(
        single_rows.len(),
        3,
        "Bob, Frank, Heidi survive the join+filter"
    );
    let subjects: Vec<&TermValue> = single_rows.iter().map(|r| r[0].as_ref().unwrap()).collect();
    assert_eq!(subjects, vec![&iri("bob"), &iri("frank"), &iri("heidi")]);

    // The headline assertion: the paged and single result sequences are IDENTICAL —
    // same variables AND same ordered rows of dataset-independent `TermValue`s. This is
    // the proof a SPARQL query is served directly over the paged backend, unifying `?x`
    // across pages, with a byte-identical result to the single-dataset evaluation.
    assert_eq!(single_vars, paged_vars, "same projected variables");
    assert_eq!(
        single_rows, paged_rows,
        "byte-identical rows: single == paged"
    );
}

// ── Test B — cross-page join order is cost-driven (F1 guard) ─────────────────────

/// A two-pattern BGP joined on `?x`. Full IRIs so the explained pattern strings are
/// stable and directly assertable.
const SKEW_QUERY: &str = "\
SELECT ?x ?y ?z WHERE {
  ?x <http://example.org/pa> ?y .
  ?x <http://example.org/pb> ?z .
}";

/// `triple_pattern_to_string` renders `?x <predicate> ?var .` — the exact form the
/// explain API emits, so probe order can be asserted position-by-position.
const PATTERN_A: &str = "?x <http://example.org/pa> ?y .";
const PATTERN_B: &str = "?x <http://example.org/pb> ?z .";

/// Build a paged fixture over three pages where predicate `heavy` outnumbers `light`.
/// The heavy/light triples are spread across the pages so the estimator's Σ-per-page
/// summation (not a single page) is what makes the two patterns' cardinalities differ.
fn skew_fixture(light: &str, heavy: &str) -> PagedDataset {
    let light = iri(light);
    let heavy = iri(heavy);
    // light: exactly ONE triple, on page 0. heavy: SIX triples, spread 2/2/2.
    let page0 = build_page(&[
        (iri("s_light"), light, iri("o_light")),
        (iri("h0"), heavy.clone(), iri("v0")),
        (iri("h1"), heavy.clone(), iri("v1")),
    ]);
    let page1 = build_page(&[
        (iri("h2"), heavy.clone(), iri("v2")),
        (iri("h3"), heavy.clone(), iri("v3")),
    ]);
    let page2 = build_page(&[
        (iri("h4"), heavy.clone(), iri("v4")),
        (iri("h5"), heavy, iri("v5")),
    ]);
    paged_over(vec![page0, page1, page2])
}

#[test]
fn cross_page_join_order_is_cost_driven_and_flips_with_skew() {
    let engine = NativeSparqlEngine::new();

    // Fixture 1: `pa` selective (Σ = 1), `pb` broad (Σ = 6). The planner must probe the
    // low-cardinality pattern A first.
    let fixture1 = skew_fixture("pa", "pb");
    // Fixture 2: the skew INVERTED — `pb` selective, `pa` broad — so the order flips.
    let fixture2 = skew_fixture("pb", "pa");

    // The estimator's Σ-per-page cross-page cardinality is what drives the order. These
    // direct estimates (using the same GraphMatch::Default scope the planner uses) show
    // the skew is real and inverted between the two fixtures.
    let card = |ds: &PagedDataset, pred: &str| -> usize {
        let p = ds.term_id_by_value(&iri(pred)).expect("predicate interned");
        ds.cardinality_estimate(None, Some(p), None, GraphMatch::Default)
    };
    assert_eq!(card(&fixture1, "pa"), 1, "pa is selective in fixture 1");
    assert_eq!(card(&fixture1, "pb"), 6, "pb is broad in fixture 1");
    assert!(card(&fixture1, "pa") < card(&fixture1, "pb"));
    assert_eq!(card(&fixture2, "pa"), 6, "pa is broad in fixture 2");
    assert_eq!(card(&fixture2, "pb"), 1, "pb is selective in fixture 2");
    assert!(card(&fixture2, "pb") < card(&fixture2, "pa"));

    // The real planner path: `explain_query_view` returns the cost-based probe order as
    // triple-pattern strings. The low-Σ-per-page pattern is probed FIRST.
    let order1 = engine
        .explain_query_view(&fixture1, SKEW_QUERY, None)
        .expect("explain fixture 1");
    assert_eq!(
        order1,
        vec![PATTERN_A, PATTERN_B],
        "selective pa probed first"
    );

    let order2 = engine
        .explain_query_view(&fixture2, SKEW_QUERY, None)
        .expect("explain fixture 2");
    // The FLIP: inverting the cross-page skew inverts the probe order. This can only
    // happen if the Σ-per-page cost model is actually consulted — an estimator that
    // ignored the paged cardinalities would return the same (source) order both times.
    assert_eq!(
        order2,
        vec![PATTERN_B, PATTERN_A],
        "selective pb probed first"
    );
    assert_ne!(order1, order2, "the probe order flips with the skew");
}

// ── Test C — lazy hook fires only for needed pages (query-time) ─────────────────

const BOUND_SUBJECT_QUERY: &str =
    "SELECT ?o WHERE { <http://example.org/alice> <http://example.org/knows> ?o }";

#[test]
fn query_materializes_only_the_pages_the_plan_needs() {
    // `:alice` lives on page 0 ONLY; pages 1 and 2 carry unrelated `knows` edges. A
    // query bound to `:alice` can only match page 0.
    let provider = Arc::new(CountingDemandProvider::new(vec![
        Box::new(|| build_page(&[(iri("alice"), iri("knows"), iri("bob"))])),
        Box::new(|| build_page(&[(iri("carol"), iri("knows"), iri("dave"))])),
        Box::new(|| build_page(&[(iri("eve"), iri("knows"), iri("frank"))])),
    ]));
    let paged = PagedDataset::from_provider(provider.clone() as Arc<dyn purrdf_core::PageProvider>)
        .expect("seal pages");

    // The seal pass materialized each page exactly once.
    let hits_after_seal = provider.hits();
    assert_eq!(
        hits_after_seal,
        paged.page_count(),
        "seal pass pulls each of the 3 pages once"
    );

    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_query(BOUND_SUBJECT_QUERY, None)
        .expect("prepare");

    // Run the bound-subject query through the PUBLIC generic entry point.
    let (_vars, rows) = solutions(
        engine
            .query_prepared_view(&paged, &prepared, &[])
            .expect("paged query"),
    );
    // Correctness: the query is genuinely served — `:alice :knows :bob`.
    assert_eq!(rows.len(), 1, "alice knows exactly one thing");
    assert_eq!(rows[0][0].as_ref().unwrap(), &iri("bob"));

    // The lazy hook fired for EXACTLY the one page that could match (page 0); pages 1
    // and 2 — whose translations lack `:alice` — were never re-materialized.
    let delta = provider.hits() - hits_after_seal;
    assert_eq!(
        delta, 1,
        "only page 0 (the page containing :alice) materialized"
    );

    // A second identical run hits the per-page OnceLock cache — no further pulls.
    let _ = engine
        .query_prepared_view(&paged, &prepared, &[])
        .expect("paged query (rerun)");
    assert_eq!(
        provider.hits() - hits_after_seal,
        1,
        "the cached page is not re-materialized on a repeat query"
    );
}
