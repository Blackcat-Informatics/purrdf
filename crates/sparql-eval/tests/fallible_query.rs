// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Public completeness-boundary tests for fallible SPARQL execution.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use purrdf_core::{
    InMemoryPageProvider, PackBuilder, PackView, PageFault, PageGeneration, PageId,
    PageMaterialization, PageProvider, PagedDataset, PagedQueryError, PagedQueryEvidence,
    PagedQueryLimits, RdfDataset, RdfDatasetBuilder, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{FallibleSparqlError, NativeSparqlEngine};

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
        )
        .expect("a genuinely empty answer is complete");
    match empty.result {
        SparqlResult::Solutions { rows, .. } => assert!(rows.is_empty()),
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
        .query_prepared_fallible_view(&view, &prepared, &[])
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
        .query_fallible_view(&view, request("SELECT ?s WHERE { ?s ?p ?o }"))
        .expect_err("cancelled materialization cannot return empty solutions");
    match error {
        FallibleSparqlError::Operational { error, evidence } => {
            assert_eq!(
                error,
                PagedQueryError::Cancelled {
                    page: PageId(0),
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
        .query_fallible_view(&view, request(query))
        .expect_err("operational failure has precedence");
    assert!(matches!(
        error,
        FallibleSparqlError::Operational {
            error: PagedQueryError::Cancelled {
                page: PageId(0),
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
        .query_fallible_view(&view, request("SELECT WHERE {"))
        .expect_err("invalid SPARQL must fail parsing");
    match error {
        FallibleSparqlError::Query {
            diagnostic,
            evidence,
        } => {
            assert_eq!(diagnostic.code, "native-sparql-query-parse");
            assert!(evidence.requested_pages.is_empty());
            assert_eq!(evidence.consumed_pages, 0);
        }
        FallibleSparqlError::Operational { error, .. } => {
            panic!("view remained ready; expected parse error, got: {error}")
        }
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
            .query_fallible_view(&view, request(query))
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
        .query_fallible_view(&exact_view, query)
        .expect("equality with both ceilings is admitted");
    assert_eq!(solution_parts(exact.result).1.len(), 2);
    assert_eq!(exact.evidence.requested_pages, vec![PageId(0), PageId(1)]);
    assert_eq!(exact.evidence.consumed_pages, 2);
    assert_eq!(exact.evidence.consumed_bytes, 30);

    let page_view = paged.query_view(PagedQueryLimits::new(1, u64::MAX));
    let page_error = engine
        .query_fallible_view(&page_view, query)
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
        .query_fallible_view(&byte_view, query)
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
            .query_fallible_view(&zero_pages, query)
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
            .query_fallible_view(&zero_bytes, query)
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
            .query_fallible_view(&view, request("SELECT * WHERE { ?s ?p ?o }"))
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
                PagedQueryError::Cancelled {
                    page: PageId(0),
                    ..
                }
            ) | (
                ScriptedFault::Deadline,
                PagedQueryError::DeadlineExceeded {
                    page: PageId(0),
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
        )
        .expect("genuinely empty query is complete");
    assert!(solution_parts(empty.result).1.is_empty());
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
            .query_fallible_view(&view, query)
            .expect("identical complete execution");
        let (variables, rows) = solution_parts(complete.result);
        let current = (variables, rows, complete.evidence);
        if let Some(expected) = &expected_success {
            assert_eq!(&current, expected);
        } else {
            expected_success = Some(current);
        }
    }

    let mut expected_failure = None;
    for _ in 0..4 {
        let failing = two_page_faulting_dataset(ScriptedFault::Provider);
        let view = failing.query_view(PagedQueryLimits::UNBOUNDED);
        let current = engine
            .query_fallible_view(&view, request("SELECT * WHERE { ?s ?p ?o }"))
            .expect_err("identical failed execution");
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
            .query_fallible_view(&view, query)
            .expect("complete empty execution");
        let (_, rows) = solution_parts(complete.result);
        assert!(rows.is_empty());
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
        .query_prepared(&resident, &prepared, &[])
        .expect("resident infallible query");
    let pack_result = engine
        .query_prepared_view(&pack, &prepared, &[])
        .expect("pack infallible query");
    assert_eq!(
        solution_parts(resident_result),
        solution_parts(pack_result),
        "ordinary resident and immutable-pack results remain exactly identical"
    );
}
