// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Public completeness-boundary tests for fallible SPARQL execution.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use purrdf_core::{
    InMemoryPageProvider, PageFault, PageGeneration, PageId, PageMaterialization, PageProvider,
    PagedDataset, PagedQueryError, PagedQueryLimits, RdfDataset, RdfDatasetBuilder, SparqlRequest,
    SparqlResult,
};
use purrdf_sparql_eval::{FallibleSparqlError, NativeSparqlEngine};

fn page() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("http://example.org/s");
    let predicate = builder.intern_iri("http://example.org/p");
    let object = builder.intern_iri("http://example.org/o");
    builder.push_quad(subject, predicate, object, None);
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
