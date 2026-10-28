// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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

mod support;
use purrdf_core::term_fixture::{Triple, build_page, iri};
use std::sync::Arc;
use support::solutions;

use purrdf_core::{
    CountingDemandProvider, DatasetView, GraphMatch, InMemoryPageProvider, PageId, PagedDataset,
    PagedQueryLimits, RdfDataset, TermValue,
};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};

// ── shared fixture helpers ─────────────────────────────────────────────────────

/// An `xsd:integer` typed literal (numeric so `FILTER(?age >= 18)` compares by value).
fn xsd_int(n: &str) -> TermValue {
    TermValue::typed_literal(n, "http://www.w3.org/2001/XMLSchema#integer")
}

/// Wrap eagerly-built pages in an in-memory provider and seal them into a
/// `PagedDataset`.
fn paged_over(pages: Vec<Arc<RdfDataset>>) -> PagedDataset {
    let provider = Arc::new(InMemoryPageProvider::new(pages));
    PagedDataset::from_provider(provider).expect("seal pages")
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
    let single = build_page(&corpus).expect("authored fixture");

    // (2) the paged view over the SAME triples, split so each of `knows`/`name`/`age`
    // lives on its OWN page — a join binding for `?x` therefore straddles all three.
    let paged = paged_over(vec![
        build_page(&parity_page_knows()).expect("authored fixture"),
        build_page(&parity_page_age()).expect("authored fixture"),
        build_page(&parity_page_name()).expect("authored fixture"),
    ]);
    assert_eq!(paged.page_count(), 3);

    // ONE prepared query, evaluated via the concrete `query_prepared` (single) and the
    // generic `query_prepared_view` (paged) public engine entry points.
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(PARITY_QUERY, None).expect("prepare");

    let (single_vars, single_rows) = solutions(
        engine
            .query_prepared(&single, &prepared, &[], QueryOptions::EMPTY)
            .expect("single query"),
    );
    let (paged_vars, paged_rows) = solutions(
        engine
            .query_prepared_view(&paged, &prepared, &[], QueryOptions::EMPTY)
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
    ])
    .expect("authored fixture");
    let page1 = build_page(&[
        (iri("h2"), heavy.clone(), iri("v2")),
        (iri("h3"), heavy.clone(), iri("v3")),
    ])
    .expect("authored fixture");
    let page2 = build_page(&[
        (iri("h4"), heavy.clone(), iri("v4")),
        (iri("h5"), heavy, iri("v5")),
    ])
    .expect("authored fixture");
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
    let card = |ds: &PagedDataset, pred: &str| -> u64 {
        let p = ds
            .term_id_by_value(&iri(pred))
            .expect("resident dictionary read")
            .expect("predicate interned");
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
    let explained1 = engine
        .explain_query_view(&fixture1, SKEW_QUERY, None)
        .expect("explain fixture 1");
    let order1 = explained1.join_orders().to_vec();
    assert_eq!(
        order1,
        vec![PATTERN_A, PATTERN_B],
        "selective pa probed first"
    );

    let explained2 = engine
        .explain_query_view(&fixture2, SKEW_QUERY, None)
        .expect("explain fixture 2");
    let order2 = explained2.join_orders().to_vec();
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

#[test]
/// Inverse store cardinalities reverse join order without loading pages the join cannot use.
fn operational_explain_uses_store_cardinalities_without_materializing_unneeded_pages() {
    let engine = NativeSparqlEngine::new();
    let first = skew_fixture("pa", "pb");
    let second = skew_fixture("pb", "pa");
    for _ in 0..3 {
        for (dataset, expected) in [
            (&first, [PATTERN_A, PATTERN_B]),
            (&second, [PATTERN_B, PATTERN_A]),
        ] {
            let view = dataset.query_view(PagedQueryLimits::UNBOUNDED);
            let (explanation, evidence) = engine
                .explain_query_fallible_view(&view, SKEW_QUERY, None)
                .expect("explain the operational store directly");
            assert_eq!(explanation.join_orders(), expected);
            // Each light predicate lives on page zero and its subject occurs in no
            // heavy-predicate page. The measured join is genuinely empty; planning
            // consults sealed cardinalities while execution demands only page zero.
            // A resident copy would have read all three pages before explaining.
            assert_eq!(evidence.requested_pages, vec![PageId(0)]);
            assert_eq!(evidence.consumed_pages, 1);
            assert!(
                explanation
                    .ledger()
                    .iter()
                    .any(|node| node.estimate.is_some())
            );
        }
    }
}

// ── Test C — lazy hook fires only for needed pages (query-time) ─────────────────

const BOUND_SUBJECT_QUERY: &str =
    "SELECT ?o WHERE { <http://example.org/alice> <http://example.org/knows> ?o }";

#[test]
fn query_materializes_only_the_pages_the_plan_needs() {
    // `:alice` lives on page 0 ONLY; pages 1 and 2 carry unrelated `knows` edges. A
    // query bound to `:alice` can only match page 0.
    let provider = Arc::new(CountingDemandProvider::new(vec![
        Box::new(|| {
            build_page(&[(iri("alice"), iri("knows"), iri("bob"))]).expect("authored fixture")
        }),
        Box::new(|| {
            build_page(&[(iri("carol"), iri("knows"), iri("dave"))]).expect("authored fixture")
        }),
        Box::new(|| {
            build_page(&[(iri("eve"), iri("knows"), iri("frank"))]).expect("authored fixture")
        }),
    ]));
    let paged = PagedDataset::from_provider(provider.clone() as Arc<dyn purrdf_core::PageProvider>)
        .expect("seal pages");

    // The seal pass materialized each page exactly once.
    let hits_after_seal = provider.hits();
    assert_eq!(
        u64::try_from(hits_after_seal).expect("fixture hits fit u64"),
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
            .query_prepared_view(&paged, &prepared, &[], QueryOptions::EMPTY)
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
        .query_prepared_view(&paged, &prepared, &[], QueryOptions::EMPTY)
        .expect("paged query (rerun)");
    assert_eq!(
        provider.hits() - hits_after_seal,
        1,
        "the cached page is not re-materialized on a repeat query"
    );
}

// ── Test D — harder SPARQL feature classes: paged == single (byte-identical) ─────
//
// The BGP parity above proves basic joins cross pages. These guards extend that proof
// to the SPARQL feature classes whose evaluation is NOT a plain triple scan — property
// paths, aggregates, and subqueries — over a multi-page `PagedDataset`. Each fixture is
// built so the feature genuinely spans page boundaries, and each asserts the paged
// result is byte-identical (same `variables`, same ordered `rows`) to the single-dataset
// evaluation of the SAME prepared query through the public engine entry points.

/// Evaluate `query` over a single frozen dataset of `corpus` AND over `pages` sealed
/// into a `PagedDataset`, and assert the two `Solutions` are identical. `min_rows`
/// guards against a vacuous pass (empty == empty). The pages MUST be exactly the corpus
/// re-partitioned, so any divergence is the paged evaluation, not a different dataset.
fn assert_paged_matches_single(
    corpus: &[Triple],
    pages: Vec<Arc<RdfDataset>>,
    query: &str,
    min_rows: usize,
) {
    let single = build_page(corpus).expect("authored fixture");
    let paged = paged_over(pages);
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(query, None).expect("prepare");

    let (single_vars, single_rows) = solutions(
        engine
            .query_prepared(&single, &prepared, &[], QueryOptions::EMPTY)
            .expect("single query"),
    );
    let (paged_vars, paged_rows) = solutions(
        engine
            .query_prepared_view(&paged, &prepared, &[], QueryOptions::EMPTY)
            .expect("paged query"),
    );

    assert!(
        single_rows.len() >= min_rows,
        "non-vacuous: expected at least {min_rows} rows, got {}",
        single_rows.len()
    );
    assert_eq!(single_vars, paged_vars, "same projected variables");
    assert_eq!(
        single_rows, paged_rows,
        "byte-identical rows: single == paged"
    );
}

const PATH_QUERY: &str = "\
PREFIX ex: <http://example.org/>
SELECT ?y WHERE { ex:alice ex:knows+ ?y } ORDER BY ?y";

#[test]
fn paged_property_path_parity_across_pages() {
    // A knows-chain alice→bob→carol→dave→erin with EACH edge on its own page, so the
    // transitive `ex:knows+` closure can only be computed by walking across all pages.
    let p0 = vec![(iri("alice"), iri("knows"), iri("bob"))];
    let p1 = vec![(iri("bob"), iri("knows"), iri("carol"))];
    let p2 = vec![(iri("carol"), iri("knows"), iri("dave"))];
    let p3 = vec![(iri("dave"), iri("knows"), iri("erin"))];
    let corpus: Vec<Triple> = p0
        .iter()
        .chain(&p1)
        .chain(&p2)
        .chain(&p3)
        .cloned()
        .collect();
    let pages = vec![
        build_page(&p0).expect("authored fixture"),
        build_page(&p1).expect("authored fixture"),
        build_page(&p2).expect("authored fixture"),
        build_page(&p3).expect("authored fixture"),
    ];
    // The closure of alice under knows+ is {bob, carol, dave, erin} — 4 rows spanning
    // all four pages.
    assert_paged_matches_single(&corpus, pages, PATH_QUERY, 4);
}

const AGGREGATE_QUERY: &str = "\
PREFIX ex: <http://example.org/>
SELECT ?g (COUNT(?p) AS ?c) WHERE { ?p ex:inGroup ?g } GROUP BY ?g ORDER BY ?g";

#[test]
fn paged_aggregate_group_by_parity_across_pages() {
    // Members of two groups spread across three pages, so each group's COUNT must sum
    // contributions materialized from multiple pages.
    let p0 = vec![
        (iri("p1"), iri("inGroup"), iri("g_a")),
        (iri("p2"), iri("inGroup"), iri("g_b")),
    ];
    let p1 = vec![
        (iri("p3"), iri("inGroup"), iri("g_a")),
        (iri("p4"), iri("inGroup"), iri("g_a")),
    ];
    let p2 = vec![(iri("p5"), iri("inGroup"), iri("g_b"))];
    let corpus: Vec<Triple> = p0.iter().chain(&p1).chain(&p2).cloned().collect();
    let pages = vec![
        build_page(&p0).expect("authored fixture"),
        build_page(&p1).expect("authored fixture"),
        build_page(&p2).expect("authored fixture"),
    ];
    // Two groups (g_a with 3 members, g_b with 2), each counted across pages → 2 rows.
    assert_paged_matches_single(&corpus, pages, AGGREGATE_QUERY, 2);
}

const SUBQUERY_QUERY: &str = "\
PREFIX ex: <http://example.org/>
SELECT ?x ?c WHERE {
  { SELECT ?x (COUNT(?y) AS ?c) WHERE { ?x ex:knows ?y } GROUP BY ?x }
} ORDER BY ?x ?c";

#[test]
fn paged_subquery_parity_across_pages() {
    // Each subject's knows-edges are spread across pages, so the inner aggregating
    // subquery must count them cross-page before the outer query projects the result.
    let p0 = vec![
        (iri("alice"), iri("knows"), iri("bob")),
        (iri("alice"), iri("knows"), iri("carol")),
    ];
    let p1 = vec![
        (iri("alice"), iri("knows"), iri("dave")), // alice's third edge, on another page
        (iri("bob"), iri("knows"), iri("carol")),
    ];
    let corpus: Vec<Triple> = p0.iter().chain(&p1).cloned().collect();
    let pages = vec![
        build_page(&p0).expect("authored fixture"),
        build_page(&p1).expect("authored fixture"),
    ];
    // alice (3 edges) and bob (1 edge) → 2 rows; alice's count only equals 3 if the
    // subquery aggregated across both pages.
    assert_paged_matches_single(&corpus, pages, SUBQUERY_QUERY, 2);
}

/// Each page has a dense numeric interval plus one distant outlier. Every row
/// belongs to a distinct reified claim; no fixture pages or claims are repeated.
fn annotation_range_pages(page_count: usize, claims_per_page: usize) -> PagedDataset {
    use purrdf_core::{RdfDatasetBuilder, TermFactory as _};
    let mut pages = Vec::with_capacity(page_count + 1);
    for page in 0..page_count {
        let mut builder = RdfDatasetBuilder::new();
        let predicate = builder.intern_iri("urn:range:value").unwrap();
        let relation = builder.intern_iri("urn:range:claim").unwrap();
        let object = builder.intern_iri("urn:range:object").unwrap();
        for offset in 0..claims_per_page {
            let ordinal = page * claims_per_page + offset;
            let claim = builder
                .intern_iri(&format!("urn:range:{ordinal:010}"))
                .unwrap();
            let target = builder.intern_triple(claim, relation, object).unwrap();
            builder.push_reifier(claim, target).unwrap();
            let value = if offset + 1 == claims_per_page {
                1_000_000_000_000
            } else {
                // Keep the scale fixture's dictionary proportional to claim IDs,
                // not duplicated numeric spellings; the first 32 rows span the
                // dense interval and later claims share its minimum value.
                (page * claims_per_page + if offset < 32 { offset } else { 0 }) as u64
            };
            let value = builder.intern_value(&xsd_int(&value.to_string())).unwrap();
            builder.push_annotation(claim, predicate, value).unwrap();
        }
        pages.push(builder.freeze().unwrap());
    }
    paged_over(pages)
}

fn compare_annotation_range_reads(page_count: usize, claims_per_page: usize) {
    use purrdf_core::{FallibleDatasetView as _, SparqlRequest, ViewOperationStatus};
    let paged = annotation_range_pages(page_count, claims_per_page);
    assert_eq!(
        paged.source_statement_count().unwrap(),
        Some((page_count * claims_per_page * 2) as u128),
        "each unique reified claim has a declaration and one annotation"
    );
    let low = (page_count / 2) * claims_per_page + 10;
    let high = low + 10;
    let engine = NativeSparqlEngine::new();
    let run = |value: &str| {
        let query = format!(
            "SELECT ?claim ?value WHERE {{ ?claim <urn:range:value> ?value FILTER({value} >= {low} && {value} < {high}) }} ORDER BY ?claim"
        );
        let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
        let began = std::time::Instant::now();
        let result = engine
            .query_fallible_view(
                &view,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
            )
            .unwrap();
        let elapsed = began.elapsed();
        let ViewOperationStatus::Ready { evidence } = view.operation_status() else {
            panic!("complete source")
        };
        (solutions(result.result), evidence, elapsed)
    };
    // Arithmetic is outside the extracted comparison grammar. Adding integer
    // zero preserves this fixture's values and executes the real residual FILTER.
    let (unpruned, full, full_time) = run("(?value + 0)");
    let (pruned, selected, selected_time) = run("?value");
    assert_eq!(pruned, unpruned);
    assert_eq!(pruned.1.len(), 10);
    assert_eq!(full.consumed_pages, page_count as u64);
    assert_eq!(
        selected.consumed_pages, 1,
        "outliers cannot widen the dense interior"
    );
    assert!(selected.consumed_bytes < full.consumed_bytes);
    eprintln!(
        "G8 claims={} pages={} full_pages={} pruned_pages={} full_bytes={} pruned_bytes={} full_elapsed={full_time:?} pruned_elapsed={selected_time:?}",
        page_count * claims_per_page,
        page_count,
        full.consumed_pages,
        selected.consumed_pages,
        full.consumed_bytes,
        selected.consumed_bytes
    );
}

#[test]
fn annotation_ranges_skip_outlier_pages_without_changing_exact_filter_results() {
    compare_annotation_range_reads(24, 64);
}

#[test]
#[ignore = "explicit ten-million-claim G8 scale acceptance"]
fn ten_million_reified_claims_range_g8() {
    compare_annotation_range_reads(10_000, 1_000);
}

#[test]
fn annotation_ranges_keep_unknown_values_and_exact_exclusive_bounds() {
    use purrdf_core::{RdfDatasetBuilder, SparqlRequest, TermFactory as _};
    let mut pages = Vec::new();
    for (index, values) in [
        vec![xsd_int("10"), xsd_int("20"), xsd_int("30")],
        vec![
            xsd_int("not-an-integer"),
            TermValue::typed_literal("20", "urn:unknown:type"),
        ],
        vec![
            TermValue::typed_literal("NaN", purrdf_xsd::datatype::XSD_DOUBLE),
            TermValue::lang_literal("20", "en"),
        ],
    ]
    .into_iter()
    .enumerate()
    {
        let mut builder = RdfDatasetBuilder::new();
        let predicate = builder.intern_iri("urn:range:value").unwrap();
        for (offset, value) in values.iter().enumerate() {
            let claim = builder
                .intern_iri(&format!("urn:range:{index}:{offset}"))
                .unwrap();
            let value = builder.intern_value(value).unwrap();
            builder.push_annotation(claim, predicate, value).unwrap();
        }
        pages.push(builder.freeze().unwrap());
    }
    let paged = paged_over(pages);
    let engine = NativeSparqlEngine::new();
    let run = |value: &str| {
        let query = format!(
            "SELECT ?v WHERE {{ ?s <urn:range:value> ?v FILTER({value} > 10 && {value} < 30) }}"
        );
        let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
        solutions(
            engine
                .query_fallible_view(
                    &view,
                    SparqlRequest {
                        query: &query,
                        base_iri: None,
                        substitutions: &[],
                    },
                    QueryOptions::EMPTY,
                )
                .unwrap()
                .result,
        )
    };
    let actual = run("?v");
    assert_eq!(actual, run("(?v + 0)"));
    assert_eq!(actual.1, vec![vec![Some(xsd_int("20"))]]);
}

#[test]
fn annotation_validity_ranges_use_supplied_now_and_independent_variables() {
    use purrdf_core::{RdfDatasetBuilder, SparqlRequest, TermFactory as _};
    use purrdf_xsd::{XsdDatatype, XsdValue};
    let mut pages = Vec::new();
    for (name, from, until) in [
        ("expired", "2020-01-01T00:00:00Z", "2021-01-01T00:00:00Z"),
        ("current", "2021-01-01T00:00:00Z", "2023-01-01T00:00:00Z"),
        ("future", "2023-01-01T00:00:00Z", "2024-01-01T00:00:00Z"),
    ] {
        let mut builder = RdfDatasetBuilder::new();
        let claim = builder.intern_iri(&format!("urn:range:{name}")).unwrap();
        for (predicate, value) in [("urn:range:from", from), ("urn:range:until", until)] {
            let predicate = builder.intern_iri(predicate).unwrap();
            let value = builder
                .intern_value(&TermValue::typed_literal(
                    value,
                    XsdDatatype::DateTime.iri(),
                ))
                .unwrap();
            builder.push_annotation(claim, predicate, value).unwrap();
        }
        pages.push(builder.freeze().unwrap());
    }
    let paged = paged_over(pages);
    let XsdValue::DateTime(now) =
        purrdf_xsd::parse("2022-01-01T00:00:00Z", XsdDatatype::DateTime).unwrap()
    else {
        panic!("dateTime")
    };
    let engine = NativeSparqlEngine::new();
    let result = {
        let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
        engine.query_fallible_view(&view, SparqlRequest {
            query: "SELECT ?claim WHERE { ?claim <urn:range:from> ?from ; <urn:range:until> ?until FILTER(?from <= NOW() && ?until > NOW()) }",
            base_iri: None, substitutions: &[],
        }, QueryOptions::EMPTY.with_now(&now).with_entropy_seed(0)).unwrap().result
    };
    assert_eq!(
        solutions(result).1,
        vec![vec![Some(TermValue::iri("urn:range:current"))]]
    );
}

#[test]
fn annotation_range_endpoint_parsing_uses_the_original_scratch_ceiling() {
    use purrdf_core::{
        RdfDatasetBuilder, ResourceDimension, SparqlRequest, TermFactory as _, TrippedGovernor,
    };
    use purrdf_sparql_eval::{GovernedOutcome, QueryGovernors};
    let fixture = |lexical: &str| {
        let mut builder = RdfDatasetBuilder::new();
        let claim = builder.intern_iri("urn:range:claim").unwrap();
        let predicate = builder.intern_iri("urn:range:value").unwrap();
        let value = builder.intern_value(&xsd_int(lexical)).unwrap();
        builder.push_annotation(claim, predicate, value).unwrap();
        paged_over(vec![builder.freeze().unwrap()])
    };
    let tiny = fixture("1");
    let wide = fixture(&"7".repeat(16_384));
    let engine = NativeSparqlEngine::new();
    let request = SparqlRequest {
        query: "SELECT ?v WHERE { ?s <urn:range:value> ?v FILTER(?v < 0) }",
        base_iri: None,
        substitutions: &[],
    };
    let metered = engine
        .query_governed_view(
            &tiny,
            request,
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .unwrap();
    assert!(matches!(metered, GovernedOutcome::Complete { .. }));
    let limit = metered
        .evidence()
        .consumed_in(ResourceDimension::ScratchBytes);
    let governors = QueryGovernors::UNBOUNDED.with_max_scratch_bytes(limit);
    assert!(matches!(
        engine
            .query_governed_view(&tiny, request, QueryOptions::EMPTY, &governors)
            .unwrap(),
        GovernedOutcome::Complete { .. }
    ));
    let stopped = engine
        .query_governed_view(&wide, request, QueryOptions::EMPTY, &governors)
        .unwrap();
    let GovernedOutcome::BudgetExhausted(exhausted) = stopped else {
        panic!("wide range endpoint must refuse its actual scratch allocation")
    };
    assert!(matches!(
        exhausted.tripped,
        TrippedGovernor::Budget {
            dimension: ResourceDimension::ScratchBytes,
            ..
        }
    ));
}
