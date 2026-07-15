// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Operation-boundary tests for fallible paged reads.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use purrdf_core::{
    DatasetView, FallibleDatasetView, GraphMatch, InMemoryPageProvider, PageFault, PageGeneration,
    PageId, PageMaterialization, PageProvider, PagedDataset, PagedQueryError, PagedQueryEvidence,
    PagedQueryLimits, RdfDataset, RdfDatasetBuilder, TermValue, ViewOperationStatus,
};

fn page(subject: &str, object: &str) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let subject_iri = format!("http://example.org/{subject}");
    let object_iri = format!("http://example.org/{object}");
    let subject = builder.intern_iri(&subject_iri);
    let predicate = builder.intern_iri("http://example.org/p");
    let object = builder.intern_iri(&object_iri);
    builder.push_quad(subject, predicate, object, None);
    builder.freeze().expect("valid page")
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

fn failed_status(
    status: ViewOperationStatus<PagedQueryError, PagedQueryEvidence>,
) -> (PagedQueryError, PagedQueryEvidence) {
    match status {
        ViewOperationStatus::Ready { evidence } => {
            panic!("expected a failed operation, got ready evidence: {evidence:?}")
        }
        ViewOperationStatus::Failed { error, evidence } => (error, evidence),
    }
}

#[test]
fn inclusive_limits_and_cache_accounting_are_exact() {
    let generation = PageGeneration(5);
    let pages = [page("s0", "o0"), page("s1", "o1")];
    let paged = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::with_byte_lengths(
        vec![(pages[0].clone(), 10), (pages[1].clone(), 20)],
        generation,
    )))
    .expect("seal pages");

    let exact = paged.query_view(PagedQueryLimits::new(2, 30));
    assert_eq!(exact.quads().count(), 2, "equality with both limits admits");
    assert_eq!(exact.quads().count(), 2, "cached reread returns both pages");
    assert_eq!(
        ready_evidence(exact.operation_status()),
        PagedQueryEvidence {
            generation,
            requested_pages: vec![PageId(0), PageId(1)],
            consumed_pages: 2,
            consumed_bytes: 30,
        },
        "cached rereads are not charged twice"
    );

    let page_limited = paged.query_view(PagedQueryLimits::new(1, u64::MAX));
    assert_eq!(
        page_limited.quads().count(),
        1,
        "internal rows before a fault may exist but are not a complete result"
    );
    let (error, evidence) = failed_status(page_limited.operation_status());
    assert_eq!(
        error,
        PagedQueryError::PageBudgetExceeded {
            page: PageId(1),
            limit: 1,
            consumed: 1,
        }
    );
    assert_eq!(evidence.requested_pages, vec![PageId(0), PageId(1)]);
    assert_eq!(evidence.consumed_pages, 1);
    assert_eq!(evidence.consumed_bytes, 10);

    let byte_limited = paged.query_view(PagedQueryLimits::new(2, 29));
    assert_eq!(byte_limited.quads().count(), 1);
    let (error, evidence) = failed_status(byte_limited.operation_status());
    assert_eq!(
        error,
        PagedQueryError::ByteBudgetExceeded {
            page: PageId(1),
            limit: 29,
            consumed: 10,
            page_bytes: 20,
        }
    );
    assert_eq!(evidence.consumed_pages, 1);
    assert_eq!(evidence.consumed_bytes, 10);

    let zero = paged.query_view(PagedQueryLimits::new(0, 0));
    assert_eq!(zero.quads().count(), 0);
    let (error, evidence) = failed_status(zero.operation_status());
    assert!(matches!(
        error,
        PagedQueryError::PageBudgetExceeded {
            page: PageId(0),
            limit: 0,
            consumed: 0
        }
    ));
    assert_eq!(evidence.requested_pages, vec![PageId(0)]);
    assert_eq!(evidence.consumed_pages, 0);
    assert_eq!(evidence.consumed_bytes, 0);
}

struct FailAfterSealProvider {
    page: Arc<RdfDataset>,
    calls: AtomicUsize,
}

impl PageProvider for FailAfterSealProvider {
    fn page_count(&self) -> usize {
        1
    }

    fn generation(&self) -> PageGeneration {
        PageGeneration(7)
    }

    fn materialize(&self, page: PageId) -> Result<PageMaterialization, PageFault> {
        let call = self.calls.fetch_add(1, Ordering::Relaxed);
        if call == 0 {
            Ok(PageMaterialization::new(
                self.page.clone(),
                self.generation(),
                64,
            ))
        } else {
            Err(PageFault::cancelled(page, "cancel token set"))
        }
    }
}

#[test]
fn provider_failure_after_seal_is_sticky_and_never_panics() {
    let provider = Arc::new(FailAfterSealProvider {
        page: page("s", "o"),
        calls: AtomicUsize::new(0),
    });
    let paged = PagedDataset::from_provider(provider.clone()).expect("seal succeeds");
    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);

    assert_eq!(
        view.cardinality_estimate(None, None, None, GraphMatch::Any),
        1
    );
    assert_eq!(
        provider.calls.load(Ordering::Relaxed),
        1,
        "planning uses only seal-time metadata"
    );
    let planning_evidence = ready_evidence(view.operation_status());
    assert!(planning_evidence.requested_pages.is_empty());
    assert_eq!(planning_evidence.consumed_pages, 0);
    assert_eq!(planning_evidence.consumed_bytes, 0);

    assert_eq!(
        view.quads().count(),
        0,
        "query-time cancellation yields no row"
    );
    let first_status = view.operation_status();
    let (error, evidence) = failed_status(first_status.clone());
    assert_eq!(
        error,
        PagedQueryError::Cancelled {
            page: PageId(0),
            message: "cancel token set".to_owned(),
        }
    );
    assert_eq!(evidence.requested_pages, vec![PageId(0)]);
    assert_eq!(evidence.consumed_pages, 0);
    assert_eq!(evidence.consumed_bytes, 0);

    assert_eq!(
        view.quads().count(),
        0,
        "sticky failure stops every later read"
    );
    assert_eq!(
        view.operation_status(),
        first_status,
        "root cause stays stable"
    );
    assert_eq!(
        provider.calls.load(Ordering::Relaxed),
        2,
        "one seal call and one failed operation call; no retry after failure"
    );
}

struct MutableGenerationProvider {
    page: Arc<RdfDataset>,
    generation: AtomicU64,
    calls: AtomicUsize,
}

impl PageProvider for MutableGenerationProvider {
    fn page_count(&self) -> usize {
        1
    }

    fn generation(&self) -> PageGeneration {
        PageGeneration(self.generation.load(Ordering::Relaxed))
    }

    fn materialize(&self, _page: PageId) -> Result<PageMaterialization, PageFault> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(PageMaterialization::new(
            self.page.clone(),
            self.generation(),
            9,
        ))
    }
}

#[test]
fn generation_drift_is_refused_before_materialization() {
    let provider = Arc::new(MutableGenerationProvider {
        page: page("s", "o"),
        generation: AtomicU64::new(11),
        calls: AtomicUsize::new(0),
    });
    let paged = PagedDataset::from_provider(provider.clone()).expect("seal generation 11");
    provider.generation.store(12, Ordering::Relaxed);

    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    assert_eq!(view.quads().count(), 0);
    let (error, evidence) = failed_status(view.operation_status());
    assert_eq!(
        error,
        PagedQueryError::StaleGeneration {
            page: Some(PageId(0)),
            expected: PageGeneration(11),
            actual: PageGeneration(12),
        }
    );
    assert_eq!(evidence.requested_pages, vec![PageId(0)]);
    assert_eq!(
        provider.calls.load(Ordering::Relaxed),
        1,
        "only the seal materialized; stale operation was refused first"
    );
}

#[test]
fn status_checkpoint_detects_drift_without_a_page_read() {
    let provider = Arc::new(MutableGenerationProvider {
        page: page("s", "o"),
        generation: AtomicU64::new(14),
        calls: AtomicUsize::new(0),
    });
    let paged = PagedDataset::from_provider(provider.clone()).expect("seal generation 14");
    provider.generation.store(15, Ordering::Relaxed);

    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let (error, evidence) = failed_status(view.operation_status());
    assert_eq!(
        error,
        PagedQueryError::StaleGeneration {
            page: None,
            expected: PageGeneration(14),
            actual: PageGeneration(15),
        }
    );
    assert!(evidence.requested_pages.is_empty());
    assert_eq!(evidence.consumed_pages, 0);
    assert_eq!(evidence.consumed_bytes, 0);
    assert_eq!(
        provider.calls.load(Ordering::Relaxed),
        1,
        "checkpoint detects drift without another materialization"
    );
}

struct ChangingMetadataProvider {
    page: Arc<RdfDataset>,
    calls: AtomicUsize,
}

impl PageProvider for ChangingMetadataProvider {
    fn page_count(&self) -> usize {
        1
    }

    fn generation(&self) -> PageGeneration {
        PageGeneration(19)
    }

    fn materialize(&self, _page: PageId) -> Result<PageMaterialization, PageFault> {
        let call = self.calls.fetch_add(1, Ordering::Relaxed);
        let bytes = if call == 0 { 40 } else { 41 };
        Ok(PageMaterialization::new(
            self.page.clone(),
            self.generation(),
            bytes,
        ))
    }
}

#[test]
fn changed_materialization_metadata_is_invalid_data() {
    let provider = Arc::new(ChangingMetadataProvider {
        page: page("s", "o"),
        calls: AtomicUsize::new(0),
    });
    let paged = PagedDataset::from_provider(provider).expect("seal byte charge 40");
    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);

    assert_eq!(view.quads().count(), 0);
    let (error, evidence) = failed_status(view.operation_status());
    assert!(matches!(
        error,
        PagedQueryError::InvalidData {
            page: PageId(0),
            ref message
        } if message.contains("40") && message.contains("41")
    ));
    assert_eq!(evidence.requested_pages, vec![PageId(0)]);
    assert_eq!(evidence.consumed_pages, 0);
}

fn page_with_side_tables() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("http://example.org/s");
    let predicate = builder.intern_iri("http://example.org/p");
    let object = builder.intern_iri("http://example.org/o");
    builder.push_quad(subject, predicate, object, None);

    let quoted_subject = builder.intern_iri("http://example.org/a");
    let quoted_predicate = builder.intern_iri("http://example.org/b");
    let quoted_object = builder.intern_iri("http://example.org/c");
    let triple = builder.intern_triple(quoted_subject, quoted_predicate, quoted_object);
    let reifier = builder.intern_iri("http://example.org/r");
    builder.push_reifier(reifier, triple);
    let confidence = builder.intern_iri("http://example.org/confidence");
    let high = builder.intern_iri("http://example.org/high");
    builder.push_annotation(reifier, confidence, high);
    builder.freeze().expect("valid RDF 1.2 side tables")
}

#[test]
fn every_read_path_shares_one_operation_cache_and_evidence() {
    let generation = PageGeneration(29);
    let paged = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::with_byte_lengths(
        vec![(page_with_side_tables(), 77)],
        generation,
    )))
    .expect("seal page");
    let view = paged.query_view(PagedQueryLimits::new(1, 77));

    assert_eq!(
        view.cardinality_estimate(None, None, None, GraphMatch::Any),
        1
    );
    let planning_evidence = ready_evidence(view.operation_status());
    assert!(planning_evidence.requested_pages.is_empty());
    assert_eq!(planning_evidence.consumed_pages, 0);
    assert_eq!(planning_evidence.consumed_bytes, 0);
    assert_eq!(view.quads().count(), 1);
    assert_eq!(view.reifier_quads().count(), 1);
    assert_eq!(view.annotation_quads().count(), 1);
    let reifier = view
        .term_id_by_value(&TermValue::iri("http://example.org/r"))
        .expect("reifier global id");
    assert_eq!(view.annotations_of_with_graph(reifier).count(), 1);

    let evidence = ready_evidence(view.operation_status());
    assert_eq!(evidence.requested_pages, vec![PageId(0)]);
    assert_eq!(evidence.consumed_pages, 1);
    assert_eq!(evidence.consumed_bytes, 77);

    let repeat = paged.query_view(PagedQueryLimits::new(1, 77));
    assert_eq!(
        repeat.cardinality_estimate(None, None, None, GraphMatch::Any),
        1
    );
    assert_eq!(repeat.quads().count(), 1);
    assert_eq!(repeat.reifier_quads().count(), 1);
    assert_eq!(repeat.annotation_quads().count(), 1);
    let repeat_reifier = repeat
        .term_id_by_value(&TermValue::iri("http://example.org/r"))
        .expect("reifier global id");
    assert_eq!(repeat.annotations_of_with_graph(repeat_reifier).count(), 1);
    assert_eq!(
        repeat.operation_status(),
        view.operation_status(),
        "identical operation state produces identical evidence and status"
    );
}
