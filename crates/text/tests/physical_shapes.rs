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
);
