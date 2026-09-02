// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Public completeness-boundary tests for fallible SPARQL execution.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use purrdf_core::{
    InMemoryPageProvider, PackBuilder, PackView, PageFault, PageGeneration, PageId,
    PageMaterialization, PageProvider, PagedDataset, PagedQueryError, PagedQueryEvidence,
    PagedQueryLimits, RdfDataset, RdfDatasetBuilder, SparqlRequest, SparqlResult, StopCause,
    TermValue,
};
use purrdf_sparql_eval::{
    FallibleSparqlError, MemoryRelation, NativeSparqlEngine, PropertyFunctionRegistry, QueryOptions,
};

type CompleteSolutions = (Vec<String>, Vec<Vec<Option<TermValue>>>, PagedQueryEvidence);

fn page() -> Arc<RdfDataset> {
    build_page(&[("s", "p", "o")])
}

fn build_page(triples: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for &(subject, predicate, object) in triples {
        let subject = builder.intern_iri(&format!("http://example.org/{subject}"));
        let predicate = builder.intern_iri(&format!("http://example.org/{predicate}"));
        let object = builder.intern_iri(&format!("http://example.org/{object}"));
        builder.push_quad(subject, predicate, object, None);
    }
    builder.freeze().expect("valid page")
}

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

#[test]
fn successful_and_empty_results_are_explicitly_complete() {
    let generation = PageGeneration(3);
    let paged = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::with_byte_lengths(
        vec![(page(), 25)],
        generation,
    )))
    .expect("seal page");
    let engine = NativeSparqlEngine::new();

    let view = paged.query_view(PagedQueryLimits::new(1, 25));
    let complete = engine
        .query_fallible_view(
            &view,
            request("SELECT ?s WHERE { ?s <http://example.org/p> <http://example.org/o> }"),
            QueryOptions::EMPTY,
        )
        .expect("complete SELECT");
    match complete.result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => {
            assert_eq!(variables, vec!["s"]);
            assert_eq!(rows.len(), 1);
        }
        other => panic!("expected SELECT solutions, got: {other:?}"),
    }
    assert_eq!(complete.evidence.requested_pages, vec![PageId(0)]);
    assert_eq!(complete.evidence.consumed_pages, 1);
    assert_eq!(complete.evidence.consumed_bytes, 25);

    let empty_view = paged.query_view(PagedQueryLimits::new(1, 25));
    let empty = engine
        .query_fallible_view(
            &empty_view,
            request("SELECT ?s WHERE { ?s <http://example.org/missing> ?o }"),
            QueryOptions::EMPTY,
        )
        .expect("a genuinely empty answer is complete");
    match empty.result {
        SparqlResult::Solutions { rows, .. } => assert_eq!(rows, [] as [Vec<Option<TermValue>>; 0]),
        other => panic!("expected empty SELECT solutions, got: {other:?}"),
    }
    assert!(
        empty.evidence.requested_pages.is_empty(),
        "an absent constant is proven by the complete global dictionary"
    );
}

#[test]
fn prepared_entry_uses_the_same_completeness_boundary() {
    let paged = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::with_byte_lengths(
        vec![(page(), 17)],
        PageGeneration(4),
    )))
    .expect("seal page");
    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_query("ASK { ?s ?p ?o }", None)
        .expect("prepare query");
    let view = paged.query_view(PagedQueryLimits::new(1, 17));
    let complete = engine
        .query_prepared_fallible_view(&view, &prepared, &[], QueryOptions::EMPTY)
        .expect("complete prepared ASK");
    assert!(matches!(complete.result, SparqlResult::Boolean(true)));
    assert_eq!(complete.evidence.requested_pages, vec![PageId(0)]);
    assert_eq!(complete.evidence.consumed_bytes, 17);
}

struct CancelAfterSealProvider {
    page: Arc<RdfDataset>,
    calls: AtomicUsize,
}

impl PageProvider for CancelAfterSealProvider {
    fn page_count(&self) -> usize {
        1
    }

    fn generation(&self) -> PageGeneration {
        PageGeneration(8)
    }

    fn materialize(&self, page: PageId) -> Result<PageMaterialization, PageFault> {
        if self.calls.fetch_add(1, Ordering::Relaxed) == 0 {
            Ok(PageMaterialization::new(
                self.page.clone(),
                self.generation(),
                33,
            ))
        } else {
            Err(PageFault::cancelled(page, "cancelled by host"))
        }
    }
}

fn cancelled_paged() -> PagedDataset {
    PagedDataset::from_provider(Arc::new(CancelAfterSealProvider {
        page: page(),
        calls: AtomicUsize::new(0),
    }))
    .expect("provider succeeds during seal")
}

#[test]
fn query_time_failure_cannot_masquerade_as_an_empty_result() {
    let paged = cancelled_paged();
    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let error = NativeSparqlEngine::new()
        .query_fallible_view(
            &view,
            request("SELECT ?s WHERE { ?s ?p ?o }"),
            QueryOptions::EMPTY,
        )
        .expect_err("cancelled materialization cannot return empty solutions");
    match error {
        FallibleSparqlError::Operational { error, evidence } => {
            assert_eq!(
                error,
                PagedQueryError::Stopped {
                    page: PageId(0),
                    cause: StopCause::Cancelled,
                    message: "cancelled by host".to_owned(),
                }
            );
            assert_eq!(evidence.requested_pages, vec![PageId(0)]);
            assert_eq!(evidence.consumed_pages, 0);
            assert_eq!(evidence.consumed_bytes, 0);
        }
        FallibleSparqlError::Query { diagnostic, .. } => {
            panic!("expected operational cancellation, got: {diagnostic}")
        }
        // `FallibleSparqlError` is `#[non_exhaustive]`, so a variant added in its own crate
        // reaches this test without a compile error. It fails loudly rather than passing:
        // this case asserts a CANCELLATION is reported as operational, and a new failure
        // shape silently satisfying that is the assertion going quiet.
        other => panic!("expected operational cancellation, got: {other:?}"),
    }
}

#[test]
fn operational_root_cause_wins_over_a_derived_evaluator_error() {
    let paged = cancelled_paged();
    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    // The left UNION branch requests the failed page. The right branch is a
    // non-SILENT SERVICE with no source and therefore independently yields an
    // evaluator diagnostic. The final checkpoint must preserve cancellation as the
    // root cause and discard both partial branch state and the derived diagnostic.
    let query = "SELECT * WHERE { \
                 { ?s ?p ?o } UNION \
                 { SERVICE <http://example.org/service> { ?a ?b ?c } } \
                 }";
    let error = NativeSparqlEngine::new()
        .query_fallible_view(&view, request(query), QueryOptions::EMPTY)
        .expect_err("operational failure has precedence");
    assert!(matches!(
        error,
        FallibleSparqlError::Operational {
            error: PagedQueryError::Stopped {
                page: PageId(0),
                cause: StopCause::Cancelled,
                ..
            },
            ..
        }
    ));
}

#[test]
fn parse_failure_remains_an_ordinary_query_error_with_evidence() {
    let paged = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(vec![page()])))
        .expect("seal page");
    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let error = NativeSparqlEngine::new()
        .query_fallible_view(&view, request("SELECT WHERE {"), QueryOptions::EMPTY)
        .expect_err("invalid SPARQL must fail parsing");
    match error {
        FallibleSparqlError::Query {
            diagnostic,
            evidence,
        } => {
            assert_eq!(diagnostic.code, "native-sparql-query-parse");
            assert_eq!(evidence.requested_pages, [] as [_; 0]);
            assert_eq!(evidence.consumed_pages, 0);
        }
        FallibleSparqlError::Operational { error, .. } => {
            panic!("view remained ready; expected parse error, got: {error}")
        }
        // As above: a variant added later must fail this test rather than satisfy it. The
        // claim here is that a PARSE failure leaves the view ready, and only the `Query`
        // arm can witness that.
        other => panic!("view remained ready; expected parse error, got: {other:?}"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScriptedFault {
    Provider,
    Cancelled,
    Deadline,
    InvalidData,
    StaleMaterialization,
    CorruptByteCharge,
}

struct FaultingProvider {
    pages: Box<[Arc<RdfDataset>]>,
    byte_lengths: Box<[u64]>,
    generation: PageGeneration,
    fail_page: PageId,
    fault: ScriptedFault,
    calls: AtomicUsize,
}

impl FaultingProvider {
    fn new(
        pages: Vec<Arc<RdfDataset>>,
        byte_lengths: Vec<u64>,
        fail_page: PageId,
        fault: ScriptedFault,
    ) -> Self {
        assert_eq!(pages.len(), byte_lengths.len());
        Self {
            pages: pages.into_boxed_slice(),
            byte_lengths: byte_lengths.into_boxed_slice(),
            generation: PageGeneration(31),
            fail_page,
            fault,
            calls: AtomicUsize::new(0),
        }
    }
}

impl PageProvider for FaultingProvider {
    fn page_count(&self) -> usize {
        self.pages.len()
    }

    fn generation(&self) -> PageGeneration {
        self.generation
    }

    fn materialize(&self, page: PageId) -> Result<PageMaterialization, PageFault> {
        let index = usize::try_from(page.0).expect("page id fits usize");
        let Some(dataset) = self.pages.get(index) else {
            return Err(PageFault::provider(page, "page out of range"));
        };
        let call = self.calls.fetch_add(1, Ordering::Relaxed);
        let query_phase = call >= self.pages.len();
        if query_phase && page == self.fail_page {
            return match self.fault {
                ScriptedFault::Provider => Err(PageFault::provider(page, "object read failed")),
                ScriptedFault::Cancelled => Err(PageFault::cancelled(page, "cancel token set")),
                ScriptedFault::Deadline => {
                    Err(PageFault::deadline_exceeded(page, "host deadline elapsed"))
                }
                ScriptedFault::InvalidData => {
                    Err(PageFault::invalid_data(page, "page checksum mismatch"))
                }
                ScriptedFault::StaleMaterialization => Ok(PageMaterialization::new(
                    dataset.clone(),
                    PageGeneration(self.generation.0 + 1),
                    self.byte_lengths[index],
                )),
                ScriptedFault::CorruptByteCharge => Ok(PageMaterialization::new(
                    dataset.clone(),
                    self.generation,
                    self.byte_lengths[index] + 1,
                )),
            };
        }
        Ok(PageMaterialization::new(
            dataset.clone(),
            self.generation,
            self.byte_lengths[index],
        ))
    }
}

fn two_page_faulting_dataset(fault: ScriptedFault) -> PagedDataset {
    let pages = vec![
        build_page(&[("a", "p", "b")]),
        build_page(&[("b", "q", "c")]),
    ];
    PagedDataset::from_provider(Arc::new(FaultingProvider::new(
        pages,
        vec![10, 20],
        PageId(1),
        fault,
    )))
    .expect("fault is armed only after the successful seal")
}

fn one_page_faulting_dataset(fault: ScriptedFault) -> PagedDataset {
    PagedDataset::from_provider(Arc::new(FaultingProvider::new(
        vec![page()],
        vec![13],
        PageId(0),
        fault,
    )))
    .expect("fault is armed only after the successful seal")
}

fn solution_parts(result: SparqlResult) -> (Vec<String>, Vec<Vec<Option<TermValue>>>) {
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => (variables, rows),
        other => panic!("expected solutions, got: {other:?}"),
    }
}

#[test]
fn every_required_query_form_and_operator_propagates_page_failure() {
    let cases = [
        (
            "cross-page join",
            "SELECT ?x WHERE { \
             <http://example.org/a> <http://example.org/p> ?x . \
             ?x <http://example.org/q> <http://example.org/c> \
             }",
        ),
        (
            "property path",
            "ASK { <http://example.org/a> \
             (<http://example.org/p>/<http://example.org/q>) \
             <http://example.org/c> }",
        ),
        (
            "filter",
            "SELECT ?s WHERE { ?s ?p ?o FILTER(?o = <http://example.org/c>) }",
        ),
        (
            "aggregate",
            "SELECT (COUNT(*) AS ?count) WHERE { ?s ?p ?o }",
        ),
        (
            "ASK",
            "ASK { <http://example.org/b> <http://example.org/q> <http://example.org/c> }",
        ),
        ("SELECT", "SELECT * WHERE { ?s ?p ?o }"),
        (
            "CONSTRUCT",
            "CONSTRUCT { ?s <http://example.org/copy> ?o } WHERE { ?s ?p ?o }",
        ),
    ];
    let engine = NativeSparqlEngine::new();

    for (label, query) in cases {
        let paged = two_page_faulting_dataset(ScriptedFault::Provider);
        let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
        let error = engine
            .query_fallible_view(&view, request(query), QueryOptions::EMPTY)
            .expect_err("query must propagate the page failure");
        assert!(
            matches!(
                error,
                FallibleSparqlError::Operational {
                    error: PagedQueryError::Provider {
                        page: PageId(1),
                        ..
                    },
                    ..
                }
            ),
            "wrong failure for {label}: {error}"
        );
    }
}

#[test]
fn production_query_budget_boundaries_are_exact_and_distinct() {
    let pages = [
        build_page(&[("a", "p", "b")]),
        build_page(&[("b", "q", "c")]),
    ];
    let paged = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::with_byte_lengths(
        vec![(pages[0].clone(), 10), (pages[1].clone(), 20)],
        PageGeneration(37),
    )))
    .expect("seal pages");
    let engine = NativeSparqlEngine::new();
    let query = request("SELECT * WHERE { ?s ?p ?o } ORDER BY ?s ?p ?o");

    let exact_view = paged.query_view(PagedQueryLimits::new(2, 30));
    let exact = engine
        .query_fallible_view(&exact_view, query, QueryOptions::EMPTY)
        .expect("equality with both ceilings is admitted");
    assert_eq!(solution_parts(exact.result).1.len(), 2);
    assert_eq!(exact.evidence.requested_pages, vec![PageId(0), PageId(1)]);
    assert_eq!(exact.evidence.consumed_pages, 2);
    assert_eq!(exact.evidence.consumed_bytes, 30);

    let page_view = paged.query_view(PagedQueryLimits::new(1, u64::MAX));
    let page_error = engine
        .query_fallible_view(&page_view, query, QueryOptions::EMPTY)
        .expect_err("second page exceeds page ceiling");
    assert!(matches!(
        page_error,
        FallibleSparqlError::Operational {
            error: PagedQueryError::PageBudgetExceeded {
                page: PageId(1),
                limit: 1,
                consumed: 1
            },
            ..
        }
    ));

    let byte_view = paged.query_view(PagedQueryLimits::new(2, 29));
    let byte_error = engine
        .query_fallible_view(&byte_view, query, QueryOptions::EMPTY)
        .expect_err("second page exceeds byte ceiling");
    assert!(matches!(
        byte_error,
        FallibleSparqlError::Operational {
            error: PagedQueryError::ByteBudgetExceeded {
                page: PageId(1),
                limit: 29,
                consumed: 10,
                page_bytes: 20
            },
            ..
        }
    ));

    let zero_pages = paged.query_view(PagedQueryLimits::new(0, u64::MAX));
    assert!(matches!(
        engine
            .query_fallible_view(&zero_pages, query, QueryOptions::EMPTY)
            .expect_err("zero page limit"),
        FallibleSparqlError::Operational {
            error: PagedQueryError::PageBudgetExceeded {
                page: PageId(0),
                limit: 0,
                consumed: 0
            },
            ..
        }
    ));

    let zero_bytes = paged.query_view(PagedQueryLimits::new(u64::MAX, 0));
    assert!(matches!(
        engine
            .query_fallible_view(&zero_bytes, query, QueryOptions::EMPTY)
            .expect_err("zero byte limit"),
        FallibleSparqlError::Operational {
            error: PagedQueryError::ByteBudgetExceeded {
                page: PageId(0),
                limit: 0,
                consumed: 0,
                page_bytes: 10
            },
            ..
        }
    ));
}

#[test]
fn operational_failure_taxonomy_is_not_an_empty_answer() {
    let cases = [
        ScriptedFault::Provider,
        ScriptedFault::Cancelled,
        ScriptedFault::Deadline,
        ScriptedFault::InvalidData,
        ScriptedFault::StaleMaterialization,
        ScriptedFault::CorruptByteCharge,
    ];
    let engine = NativeSparqlEngine::new();
    for fault in cases {
        let paged = one_page_faulting_dataset(fault);
        let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
        let error = engine
            .query_fallible_view(
                &view,
                request("SELECT * WHERE { ?s ?p ?o }"),
                QueryOptions::EMPTY,
            )
            .expect_err("scripted operational fault cannot return a result");
        let FallibleSparqlError::Operational { error, evidence } = error else {
            panic!("{fault:?} became an ordinary query diagnostic");
        };
        let category_matches = matches!(
            (fault, &error),
            (
                ScriptedFault::Provider,
                PagedQueryError::Provider {
                    page: PageId(0),
                    ..
                }
            ) | (
                ScriptedFault::Cancelled,
                PagedQueryError::Stopped {
                    page: PageId(0),
                    cause: StopCause::Cancelled,
                    ..
                }
            ) | (
                ScriptedFault::Deadline,
                PagedQueryError::Stopped {
                    page: PageId(0),
                    cause: StopCause::Deadline,
                    ..
                }
            ) | (
                ScriptedFault::InvalidData | ScriptedFault::CorruptByteCharge,
                PagedQueryError::InvalidData {
                    page: PageId(0),
                    ..
                }
            ) | (
                ScriptedFault::StaleMaterialization,
                PagedQueryError::StaleGeneration {
                    page: Some(PageId(0)),
                    ..
                }
            )
        );
        assert!(category_matches, "wrong category for {fault:?}: {error}");
        assert_eq!(evidence.requested_pages, vec![PageId(0)]);
        assert_eq!(evidence.consumed_pages, 0);
        assert_eq!(evidence.consumed_bytes, 0);
    }

    let normal = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(vec![page()])))
        .expect("seal normal page");
    let normal_view = normal.query_view(PagedQueryLimits::UNBOUNDED);
    let empty = engine
        .query_fallible_view(
            &normal_view,
            request("SELECT * WHERE { ?s <http://example.org/absent> ?o }"),
            QueryOptions::EMPTY,
        )
        .expect("genuinely empty query is complete");
    assert_eq!(
        solution_parts(empty.result).1,
        [] as [Vec<Option<TermValue>>; 0]
    );
}

#[test]
fn identical_executions_have_identical_results_status_and_evidence() {
    let pages = [
        build_page(&[("a", "p", "b")]),
        build_page(&[("b", "q", "c")]),
    ];
    let paged = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::with_byte_lengths(
        vec![(pages[0].clone(), 10), (pages[1].clone(), 20)],
        PageGeneration(41),
    )))
    .expect("seal pages");
    let engine = NativeSparqlEngine::new();
    let query = request(
        "SELECT ?x WHERE { \
         <http://example.org/a> <http://example.org/p> ?x . \
         ?x <http://example.org/q> <http://example.org/c> \
         } ORDER BY ?x",
    );
    let mut expected_success: Option<CompleteSolutions> = None;
    for _ in 0..4 {
        let view = paged.query_view(PagedQueryLimits::new(2, 30));
        let complete = engine
            .query_fallible_view(&view, query, QueryOptions::EMPTY)
            .expect("identical complete execution");
        let (variables, rows) = solution_parts(complete.result);
        let current = (variables, rows, complete.evidence);
        if let Some(expected) = &expected_success {
            assert_eq!(&current, expected);
        } else {
            expected_success = Some(current);
        }
    }

    let mut expected_failure: Option<(PagedQueryError, PagedQueryEvidence)> = None;
    for _ in 0..4 {
        let failing = two_page_faulting_dataset(ScriptedFault::Provider);
        let view = failing.query_view(PagedQueryLimits::UNBOUNDED);
        let failure = engine
            .query_fallible_view(
                &view,
                request("SELECT * WHERE { ?s ?p ?o }"),
                QueryOptions::EMPTY,
            )
            .expect_err("identical failed execution");
        // The error type carries a materialized `SparqlResult` on its budget-exhausted
        // arm and is therefore not comparable as a whole. What an identical execution has
        // to reproduce is the discriminant, the root cause, and the evidence — each
        // comparable on its own, and together a strictly more precise claim than
        // whole-value equality, because the arm is now asserted by name.
        let FallibleSparqlError::Operational { error, evidence } = failure else {
            panic!("a provider fault is an operational failure: {failure:?}");
        };
        let current = (error, evidence);
        if let Some(expected) = &expected_failure {
            assert_eq!(&current, expected);
        } else {
            expected_failure = Some(current);
        }
    }
}

#[test]
fn cold_and_warm_bgp_planning_have_identical_demand_paging_evidence() {
    let pages = [
        build_page(&[("a", "p", "x"), ("y", "q", "b")]),
        build_page(&[
            ("c0", "r", "d0"),
            ("c1", "r", "d1"),
            ("c2", "r", "d2"),
            ("c3", "r", "d3"),
            ("c4", "r", "d4"),
        ]),
    ];
    let paged = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::with_byte_lengths(
        vec![(pages[0].clone(), 20), (pages[1].clone(), 50)],
        PageGeneration(43),
    )))
    .expect("seal pages");
    let engine = NativeSparqlEngine::new();
    let query = request(
        "SELECT * WHERE { \
         <http://example.org/a> <http://example.org/q> <http://example.org/b> . \
         ?s <http://example.org/r> ?o \
         }",
    );

    let mut expected = None;
    for _ in 0..2 {
        let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
        let complete = engine
            .query_fallible_view(&view, query, QueryOptions::EMPTY)
            .expect("complete empty execution");
        let (_, rows) = solution_parts(complete.result);
        assert_eq!(rows, [] as [Vec<Option<TermValue>>; 0]);
        assert_eq!(complete.evidence.requested_pages, vec![PageId(0)]);
        assert_eq!(complete.evidence.consumed_pages, 1);
        assert_eq!(complete.evidence.consumed_bytes, 20);
        if let Some(expected) = &expected {
            assert_eq!(
                &complete.evidence, expected,
                "warming the BGP-order cache must not change provider demand"
            );
        } else {
            expected = Some(complete.evidence);
        }
    }
}

#[test]
fn resident_and_pack_views_keep_the_ordinary_byte_identical_result_path() {
    let resident = build_page(&[("a", "p", "b"), ("b", "q", "c")]);
    let pack_bytes = PackBuilder::build_bytes(&resident).expect("build pack");
    let pack = PackView::from_bytes(&pack_bytes).expect("open pack");
    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_query("SELECT * WHERE { ?s ?p ?o } ORDER BY ?s ?p ?o", None)
        .expect("prepare query");

    let resident_result = engine
        .query_prepared(&resident, &prepared, &[], QueryOptions::EMPTY)
        .expect("resident infallible query");
    let pack_result = engine
        .query_prepared_view(&pack, &prepared, &[], QueryOptions::EMPTY)
        .expect("pack infallible query");
    assert_eq!(
        solution_parts(resident_result),
        solution_parts(pack_result),
        "ordinary resident and immutable-pack results remain exactly identical"
    );
}

// ---------------------------------------------------------------------------
// The fallible-view lane's registry gap, closed the same way the ordinary and
// governed lanes' were: `query_fallible_view`/`query_prepared_fallible_view` used to
// take no `QueryOptions` at all, so a registered relation could never be reached
// through a lazy/paged view — the predicate always stayed an ordinary triple
// pattern and the query answered whatever the page held (or nothing).
// ---------------------------------------------------------------------------

const RELATION_QUERY: &str =
    "PREFIX rel: <https://example.org/rel/> SELECT ?a ?b WHERE { ?a rel:pair ?b }";

fn pair_relation() -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        "https://example.org/rel/pair".to_owned(),
        Arc::new(
            MemoryRelation::new(
                1,
                1,
                vec![vec![
                    TermValue::iri("https://example.org/d/left"),
                    TermValue::iri("https://example.org/d/right"),
                ]],
            )
            .expect("one row, two values wide"),
        ),
    );
    registry
}

/// A registered relation answers through `query_fallible_view` once `options` carries
/// it — the fallible entry's registry-aware parse and options application, exercised
/// end to end. The page holds no `rel:pair` triple, so a non-empty answer is only
/// reachable through the relation.
#[test]
fn query_fallible_view_dispatches_a_registered_relation_with_options() {
    let paged = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(vec![page()])))
        .expect("seal page");
    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let engine = NativeSparqlEngine::new();
    let registry = pair_relation();
    let options = QueryOptions {
        property_functions: &registry,
        ..QueryOptions::EMPTY
    };

    let complete = engine
        .query_fallible_view(&view, request(RELATION_QUERY), options)
        .expect("a registered relation's call evaluates through the fallible entry");
    let (variables, rows) = solution_parts(complete.result);
    assert_eq!(variables, vec!["a", "b"]);
    assert_eq!(
        rows.len(),
        1,
        "the relation's one row, not the page's unrelated triple: {rows:?}"
    );
}

/// The prepared-plan fallible pair's residue, closed the same way the ordinary and
/// governed prepared entries' is: a plan parsed WITHOUT the registry is refused
/// rather than silently evaluated with it, and the SAME text prepared WITH the
/// registry answers.
#[test]
fn query_prepared_fallible_view_with_a_mismatched_registry_is_refused_and_the_matched_registry_answers()
 {
    let paged = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(vec![page()])))
        .expect("seal page");
    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let engine = NativeSparqlEngine::new();
    let registry = pair_relation();
    let options = QueryOptions {
        property_functions: &registry,
        ..QueryOptions::EMPTY
    };

    let stale = engine
        .prepare_query(RELATION_QUERY, None)
        .expect("the text parses as ordinary data with no registry in scope");
    let refused = engine.query_prepared_fallible_view(&view, &stale, &[], options);
    match refused.expect_err("a plan/registry disagreement must be a diagnostic") {
        FallibleSparqlError::Query { diagnostic, .. } => {
            assert_eq!(diagnostic.code, "native-sparql-property-function");
        }
        other => panic!("view remained ready; expected a query diagnostic, got: {other:?}"),
    }

    // Prepared under the SAME options, the very same text runs and answers from the
    // relation — never from the page, which holds no `rel:pair` triple to have
    // matched instead.
    let matched = engine
        .prepare_query_with_options(RELATION_QUERY, None, options)
        .expect("the registry-aware parse lowers the predicate to a call");
    let complete = engine
        .query_prepared_fallible_view(&view, &matched, &[], options)
        .expect("a plan and options that agree on the registry evaluate");
    assert_eq!(solution_parts(complete.result).1.len(), 1);
}
