// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native execution of the shared algorithm conformance cases.

#[path = "support/stemming_cases.rs"]
mod cases;

#[test]
fn every_official_snowball_english_vector() {
    cases::every_official_snowball_english_vector();
}

#[test]
fn version_and_new_snowball_exceptions_are_pinned() {
    cases::version_and_new_snowball_exceptions_are_pinned();
}

#[test]
fn non_latin_words_and_short_inputs_are_preserved() {
    cases::non_latin_words_and_short_inputs_are_preserved();
}

#[test]
fn latin_domain_uses_scalar_regions_and_preserves_non_ascii_consonants() {
    cases::latin_domain_uses_scalar_regions_and_preserves_non_ascii_consonants();
}

#[test]
fn y_is_marked_as_a_consonant_only_in_the_defined_positions() {
    cases::y_is_marked_as_a_consonant_only_in_the_defined_positions();
}

#[test]
fn arbitrarily_long_admitted_tokens_do_not_recurse_or_grow() {
    cases::arbitrarily_long_admitted_tokens_do_not_recurse_or_grow();
}
