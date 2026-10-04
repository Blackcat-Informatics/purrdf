// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Independent integration checks across fast paths, controls and source evidence.

#[path = "support/analysis_review_cases.rs"]
mod cases;

#[test]
fn every_lexical_entry_point_agrees_across_reused_scratch() {
    cases::every_lexical_entry_point_agrees_across_reused_scratch();
}

#[test]
fn fast_and_expanding_normalization_keep_original_utf8_contributors() {
    cases::fast_and_expanding_normalization_keep_original_utf8_contributors();
}

#[test]
fn refusals_never_call_a_sink_and_scratch_remains_reusable() {
    cases::refusals_never_call_a_sink_and_scratch_remains_reusable();
}

#[test]
fn removed_controls_do_not_destroy_meaningful_orthographic_joiners() {
    cases::removed_controls_do_not_destroy_meaningful_orthographic_joiners();
}

#[test]
fn html_streaming_agrees_after_decode_cleanup_and_joining() {
    cases::html_streaming_agrees_after_decode_cleanup_and_joining();
}

#[test]
fn long_control_runs_preserve_joining_context_without_rescanning() {
    cases::long_control_runs_preserve_joining_context_without_rescanning();
}
