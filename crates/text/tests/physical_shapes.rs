// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Public physical execution against literal ordered bags on both targets.
#[path = "support/shape_cases.rs"]
mod shape_cases;
use shape_cases::{
    bounded_shapes_keep_original_owners, bucket_custom_and_parallel_order,
    cache_refusal_keeps_previous_owner, dictionary_analyzer_identity, dictionary_cache_and_size,
    dictionary_long_phrase_and_determinism, dictionary_projection_and_aliases, dictionary_property,
    expanded_dictionary_and_union, false_expression_gaps_keep_original_traversal,
    graph_free_corpus_keys_ranking_and_query, graph_free_rdf12_partitions_and_named_refusal,
    host_refusal_and_governor_boundary, longest_first_reference,
    original_evidence_and_empty_barriers, prepared_and_query_forms, registered_host_and_decline,
    scoped_streams_and_update_where, typed_series_property, window_barrier_property,
    window_property,
};

#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

purrdf_testkit::harness_main!(
    dictionary_property,
    expanded_dictionary_and_union,
    dictionary_cache_and_size,
    dictionary_projection_and_aliases,
    longest_first_reference,
    window_property,
    typed_series_property,
    registered_host_and_decline,
    prepared_and_query_forms,
    window_barrier_property,
    dictionary_analyzer_identity,
    bucket_custom_and_parallel_order,
    host_refusal_and_governor_boundary,
    dictionary_long_phrase_and_determinism,
    bounded_shapes_keep_original_owners,
    original_evidence_and_empty_barriers,
    scoped_streams_and_update_where,
    cache_refusal_keeps_previous_owner,
    false_expression_gaps_keep_original_traversal,
    graph_free_corpus_keys_ranking_and_query,
    graph_free_rdf12_partitions_and_named_refusal,
    source_read_providers_keep_native_fields_after_disposal,
);

fn source_read_providers_keep_native_fields_after_disposal() {
    use purrdf_core::{
        InMemoryPageProvider, PagedDataset, PagedQueryError, PagedQueryLimits, SegmentedError,
        SegmentedReadLimits,
    };
    use purrdf_hash::error::ErrorValue;
    use purrdf_sparql_eval::EvalError;
    use purrdf_testkit::error_capture::Capture;
    use purrdf_text::{TextError, TextIndex};
    use std::sync::Arc;

    let fixture = shape_cases::Fixture::text(&[(
        "https://example.org/document".into(),
        "river".into(),
        None,
        false,
    )]);
    let config = fixture.index.config();
    let image = shape_cases::segmented(&fixture.dataset);
    let source = image
        .open_session(SegmentedReadLimits::new(
            32_000_000, 2, 16_384, 20_000_000, 8,
        ))
        .unwrap();
    let control = source.evidence().peak_bytes();
    drop(source);
    let source = image
        .open_session(SegmentedReadLimits::new(control, 2, 16_384, 20_000_000, 8))
        .unwrap();
    let segmented = TextIndex::from_dataset(&source, config).unwrap_err();
    drop(source);
    drop(image);

    let paged = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(vec![
        fixture.dataset.clone(),
    ])))
    .unwrap();
    let source = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let complete = TextIndex::from_dataset(&source, config).unwrap();
    assert_eq!(
        complete.source_fingerprint(),
        fixture.index.source_fingerprint()
    );
    drop(source);
    let source = paged.query_view(PagedQueryLimits::new(0, u64::MAX));
    let paged_error = TextIndex::from_dataset(&source, config).unwrap_err();
    drop(source);
    drop(paged);
    drop(fixture);

    for (error, segmented_source) in [(segmented, true), (paged_error, false)] {
        let copy = error.clone();
        let converted: EvalError = error.into();
        assert!(converted.is_native_storage_failure());
        let TextError::SourceRead(original) = copy else {
            panic!("provider refusal must remain a source failure");
        };
        let EvalError::RetainedNativeFailure(retained) = converted else {
            panic!("evaluator must keep the original owner");
        };
        assert!(core::ptr::eq(original.cause(), retained.cause()));
        let buffer = Vec::with_capacity(4);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let fields = Capture::read_with_buffer(retained.cause(), buffer);
        let measured = window.close();
        assert_eq!(measured.allocations, 0);
        if segmented_source {
            let native = std::error::Error::source(&retained)
                .unwrap()
                .downcast_ref::<SegmentedError>()
                .unwrap();
            assert!(
                matches!(native, SegmentedError::Residency { limit, .. } if *limit == control),
                "original source refusal: {native:?}"
            );
            assert_eq!(fields.code, "segmented-residency");
            assert!(
                matches!(fields.field("limit"), ErrorValue::ByteCount(value) if value == u128::from(control))
            );
        } else {
            let native = std::error::Error::source(&retained)
                .unwrap()
                .downcast_ref::<PagedQueryError>()
                .unwrap();
            assert!(matches!(
                native,
                PagedQueryError::PageBudgetExceeded {
                    limit: 0,
                    consumed: 0,
                    ..
                }
            ));
            assert_eq!(fields.code, "paged-query-page-budget-exceeded");
            assert!(matches!(fields.field("page"), ErrorValue::Unsigned(0)));
            assert!(matches!(fields.field("limit"), ErrorValue::Unsigned(0)));
        }
    }
}
