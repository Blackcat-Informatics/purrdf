// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **The same analysis and ranking laws executed natively and on
//! `wasm32-unknown-unknown`.**
//!
//! Shared test bodies exercise the full frozen English stemming and phonetic
//! vectors, independent scalar edit-distance oracle, official Unicode grapheme
//! and emoji corpora, normalization and source alignment, and all five pinned
//! dictionary artifacts. Ranking answers below use hand-computed decimals.
//!
//! This crate's headline claim is that a host and a browser scoring the same
//! corpus with the same needle return the same rows in the same order with the
//! same score lexicals. Every other test in the suite proves something weaker:
//! that the ranker is a pure function *of one target*. Run it fifty times on
//! this machine and it agrees with itself — which cannot distinguish a ranker
//! that is target-independent from one that merely happens to be self-consistent
//! wherever it was last compiled.
//!
//! `make wasm` does not close that gap either. It proves the release crates
//! **build** for wasm32; it cannot prove they **answer** the same way there.
//!
//! # What is actually at risk
//!
//! BM25 needs a natural logarithm. A libm `ln` may differ by a unit in the last
//! place between implementations, and that is enough to reverse the order of two
//! near-tied documents — an answer divergence, not a rounding detail, and one
//! nothing downstream could detect. [`Fixed::ln`] is therefore an integer series
//! at a **fixed** iteration count rather than a convergence test, over `i128`
//! fixed point with no floating-point value anywhere in the crate.
//!
//! That is an argument. This file is where it becomes an executed test.
//!
//! # How it runs on both
//!
//! One test body per case, one runner on both targets. The target is
//! `harness = false`, and its `main` hands the named cases to
//! `purrdf_testkit::harness`: natively they run under `cargo test --workspace`,
//! and on `wasm32-unknown-unknown` the same named cases run in Node through
//! `scripts/wasm-test-runner.sh`, the cargo runner `make wasm-test` (and CI's
//! wasm job) sets:
//!
//! ```text
//! CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=scripts/wasm-test-runner.sh \
//!     cargo test -p purrdf-text --target wasm32-unknown-unknown --test wasm_determinism
//! ```
//!
//! # Why the expectations are what they are
//!
//! They are **hand computed, not recorded** — a test that fills its expectation
//! from the kernel it is testing agrees with any kernel at all, including a
//! wrong one. The fixture is this crate's scoring golden: four documents of four
//! tokens each, so `avgdl` is exactly four and every document's length
//! normalization is exactly one; `df` is two for both needle terms, so the
//! inverse document frequency is exactly `ln 2`. The saturation at `tf = 2` is
//! exactly `1.375`. The two scores are therefore
//! `ln 2 × 1.375 + ln 2` and `ln 2 × 2`, and `ln 2` truncated to this crate's
//! twelve fractional digits is `0.693147180559`.
//!
//! The comparison is on the **decimal lexical**, not on the raw `i128`, because
//! the lexical is what a consumer receives and what a serializer writes.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_text::{
    Analyzer, Fixed, GraphSelector, PartitionFilter, TextIndex, TextIndexConfig, select,
};

/// The one predicate the fixture indexes.
const NOTE: &str = "https://example.org/note";

/// The scoring golden's corpus: four documents of four tokens each.
const CORPUS: [(&str, &str); 4] = [
    ("d1", "quick quick brown fox"),
    ("d2", "quick brown fox jumps"),
    ("d3", "lazy dog sleeps late"),
    ("d4", "river stone bridge path"),
];

/// `ln 2`, truncated to [`purrdf_text::SCALE_DIGITS`] fractional digits.
///
/// Hand value: `ln 2 = 0.6931471805599453...`, so twelve digits truncate to
/// this. Every expectation below is built from it.
const LN_2: &str = "0.693147180559";

/// The two documents the needle `"quick brown"` retrieves, and their exact
/// scores.
///
/// * `d1` holds `quick` twice and `brown` once: `ln 2 × 1.375 + ln 2 × 1`.
/// * `d2` holds each once: `ln 2 × 1 + ln 2 × 1`.
///
/// `d3` and `d4` share no term with the needle and are absent rather than
/// present with a zero score.
const EXPECTED: [(&str, &str); 2] = [("d1", "1.646224553827"), ("d2", "1.386294361118")];

/// An index over [`CORPUS`].
fn corpus_index() -> TextIndex {
    let mut builder = RdfDatasetBuilder::new();
    let note = builder.intern_iri(NOTE);
    for (local, text) in CORPUS {
        let subject = builder.intern_iri(&format!("https://example.org/{local}"));
        let literal = builder.intern_literal(RdfLiteral::simple(text));
        builder.push_quad(subject, note, literal, None);
    }
    let dataset: Arc<RdfDataset> = builder.freeze().expect("the fixture must validate");
    TextIndex::from_dataset(
        &*dataset,
        &TextIndexConfig::new(
            vec![TermValue::iri(NOTE)],
            GraphSelector::Any,
            Analyzer::empty_lexicon(),
        )
        .expect("the fixture configuration is well formed"),
    )
    .expect("the fixture index must build")
}

/// The ranking, as `(document local name, score lexical)` in emission order.
fn ranked() -> Vec<(String, String)> {
    let index = corpus_index();
    select(
        &index,
        &Analyzer::empty_lexicon()
            .terms("quick brown")
            .expect("valid text analysis"),
        &PartitionFilter::unconstrained(),
        None,
        None,
    )
    .expect("the fixture ranks")
    .into_iter()
    .map(|row| {
        let document = index
            .document(row.document)
            .expect("an emitted id resolves");
        let TermValue::Iri(subject) = document.subject() else {
            panic!("the fixture's subjects are IRIs")
        };
        (
            subject
                .strip_prefix("https://example.org/")
                .expect("fixture IRI")
                .to_owned(),
            row.score.to_decimal_lexical(),
        )
    })
    .collect()
}

/// The pinned ranking is reproduced on whichever target is executing this.
fn the_pinned_ranking_is_reproduced_on_this_target() {
    let rows = ranked();

    // The short-bag guard first: two of the four documents share a term with the
    // needle. `>=` would be satisfied by returning all four and `<=` by none.
    assert_eq!(
        rows.len(),
        EXPECTED.len(),
        "two of the four documents hold a needle term, got {rows:?}"
    );

    for (at, (got, want)) in rows.iter().zip(EXPECTED.iter()).enumerate() {
        assert_eq!(
            (got.0.as_str(), got.1.as_str()),
            (want.0, want.1),
            "rank {at} differs from the pinned cross-target answer; a divergence here is \
             the native/wasm ln hazard the fixed-point arithmetic exists to exclude, not \
             a rounding detail"
        );
    }

    // And the whole answer as one string, so a reordering that preserved every
    // pair individually would still be caught.
    assert_eq!(
        rows.iter()
            .map(|(document, score)| format!("{document}={score}"))
            .collect::<Vec<_>>()
            .join("|"),
        EXPECTED
            .iter()
            .map(|(document, score)| format!("{document}={score}"))
            .collect::<Vec<_>>()
            .join("|")
    );
}

/// The integer logarithm itself, digit for digit, on whichever target is
/// executing this.
///
/// The ranking above would still agree across targets if `ln` were wrong in the
/// same way everywhere. This pins the series against values computed outside
/// this crate, so the cross-target claim and the correctness claim are separate
/// assertions rather than one assertion doing double duty.
fn the_integer_logarithm_agrees_with_its_hand_values_on_this_target() {
    // ln 1 = 0 exactly, and it is the one input whose answer needs no series.
    assert_eq!(
        Fixed::ONE.ln().expect("ln 1").to_decimal_lexical(),
        "0.000000000000"
    );

    // ln 2 = 0.6931471805599453..., truncated to twelve fractional digits.
    let two = Fixed::from_integer(2).expect("2 is representable");
    assert_eq!(two.ln().expect("ln 2").to_decimal_lexical(), LN_2);

    // ln 4 = 2 ln 2 = 1.3862943611198906..., truncated to twelve digits. Pinned
    // as its own value rather than derived, so a series that drifted with
    // magnitude is visible here rather than cancelling out.
    let four = Fixed::from_integer(4).expect("4 is representable");
    assert_eq!(
        four.ln().expect("ln 4").to_decimal_lexical(),
        "1.386294361119"
    );

    // ln 10 = 2.302585092994046..., the largest of the three, where a
    // fixed-iteration series has the least headroom.
    let ten = Fixed::from_integer(10).expect("10 is representable");
    assert_eq!(
        ten.ln().expect("ln 10").to_decimal_lexical(),
        "2.302585092994"
    );
}

#[path = "support/bm25f_reference.rs"]
mod bm25f_reference;

/// All fielded scoring vectors, including maximum bounds, agree on this target.
fn the_independent_fielded_reference_is_reproduced_on_this_target() {
    bm25f_reference::verify_reference_corpus();
}

#[path = "support/baseline_cases.rs"]
mod baseline_cases;

fn baseline_artifact_resolution() {
    baseline_cases::baseline_artifact_resolution();
}

#[path = "support/analysis_cases.rs"]
mod analysis_cases;

fn official_grapheme_and_emoji_cases() {
    analysis_cases::official_grapheme_and_emoji_cases();
}

fn ordered_analysis_and_alignment_cases() {
    analysis_cases::ordered_analysis_and_alignment_cases();
}

#[path = "support/stemming_cases.rs"]
mod stemming_cases;

fn the_stemming_algorithms_agree_on_this_target() {
    stemming_cases::every_official_snowball_english_vector();
    stemming_cases::version_and_new_snowball_exceptions_are_pinned();
    stemming_cases::non_latin_words_and_short_inputs_are_preserved();
    stemming_cases::latin_domain_uses_scalar_regions_and_preserves_non_ascii_consonants();
    stemming_cases::y_is_marked_as_a_consonant_only_in_the_defined_positions();
    stemming_cases::arbitrarily_long_admitted_tokens_do_not_recurse_or_grow();
}

#[path = "support/phonetic_cases.rs"]
mod phonetic_cases;

fn the_phonetic_algorithms_agree_on_this_target() {
    phonetic_cases::every_independent_reference_vector_matches_at_every_declared_code_bound();
    phonetic_cases::phonetic_domain_and_every_resource_boundary_are_explicit();
    phonetic_cases::canonical_spelling_is_shared_by_code_and_distance();
    phonetic_cases::scalar_distance_examples_cover_thresholds_and_unicode();
    phonetic_cases::banded_distance_matches_full_matrix_for_all_short_mixed_scalar_strings();
    phonetic_cases::scalar_distance_refuses_invalid_bounds_and_handles_maximum_inputs();
    phonetic_cases::both_engines_cross_every_block_boundary_and_reuse_query_scratch();
    phonetic_cases::long_graphemes_remain_scalar_sequences_for_refinement();
    phonetic_cases::arbitrary_unicode_edits_match_the_independent_global_matrix();
}

#[path = "support/analysis_review_cases.rs"]
mod analysis_review_cases;

fn adversarial_analysis_agrees_on_this_target() {
    analysis_review_cases::every_lexical_entry_point_agrees_across_reused_scratch();
    analysis_review_cases::fast_and_expanding_normalization_keep_original_utf8_contributors();
    analysis_review_cases::refusals_never_call_a_sink_and_scratch_remains_reusable();
    analysis_review_cases::removed_controls_do_not_destroy_meaningful_orthographic_joiners();
    analysis_review_cases::html_streaming_agrees_after_decode_cleanup_and_joining();
    analysis_review_cases::long_control_runs_preserve_joining_context_without_rescanning();
}

#[path = "support/character_cases.rs"]
mod character_cases;

fn han_prepend_graphemes_agree_on_this_target() {
    character_cases::prepend_scalars_do_not_hide_han_bases();
}

#[path = "support/emoji_predicate_cases.rs"]
mod emoji_predicate_cases;

fn emoji_recognition_agrees_on_this_target() {
    emoji_predicate_cases::every_emoji_spelling_and_protected_scalar_matches_the_input_data();
}

#[path = "support/accent_cases.rs"]
mod accent_cases;

fn every_accent_subset_agrees_on_this_target() {
    accent_cases::every_accent_subset_is_independent_and_identified();
}

purrdf_testkit::harness_main!(
    every_accent_subset_agrees_on_this_target,
    emoji_recognition_agrees_on_this_target,
    han_prepend_graphemes_agree_on_this_target,
    adversarial_analysis_agrees_on_this_target,
    baseline_artifact_resolution,
    official_grapheme_and_emoji_cases,
    ordered_analysis_and_alignment_cases,
    the_stemming_algorithms_agree_on_this_target,
    the_phonetic_algorithms_agree_on_this_target,
    the_independent_fielded_reference_is_reproduced_on_this_target,
    the_integer_logarithm_agrees_with_its_hand_values_on_this_target,
    the_pinned_ranking_is_reproduced_on_this_target,
);
