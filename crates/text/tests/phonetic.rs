// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native execution of the shared algorithm conformance cases.

#[path = "support/phonetic_cases.rs"]
mod cases;

#[test]
fn every_independent_reference_vector_matches_at_every_declared_code_bound() {
    cases::every_independent_reference_vector_matches_at_every_declared_code_bound();
}

#[test]
fn phonetic_domain_and_every_resource_boundary_are_explicit() {
    cases::phonetic_domain_and_every_resource_boundary_are_explicit();
}

#[test]
fn canonical_spelling_is_shared_by_code_and_distance() {
    cases::canonical_spelling_is_shared_by_code_and_distance();
}

#[test]
fn scalar_distance_examples_cover_thresholds_and_unicode() {
    cases::scalar_distance_examples_cover_thresholds_and_unicode();
}

#[test]
fn banded_distance_matches_full_matrix_for_all_short_mixed_scalar_strings() {
    cases::banded_distance_matches_full_matrix_for_all_short_mixed_scalar_strings();
}

#[test]
fn scalar_distance_refuses_invalid_bounds_and_handles_maximum_inputs() {
    cases::scalar_distance_refuses_invalid_bounds_and_handles_maximum_inputs();
}

#[test]
fn both_engines_cross_every_block_boundary_and_reuse_query_scratch() {
    cases::both_engines_cross_every_block_boundary_and_reuse_query_scratch();
}

#[test]
fn long_graphemes_remain_scalar_sequences_for_refinement() {
    cases::long_graphemes_remain_scalar_sequences_for_refinement();
}

#[test]
fn arbitrary_unicode_edits_match_the_independent_global_matrix() {
    cases::arbitrary_unicode_edits_match_the_independent_global_matrix();
}
