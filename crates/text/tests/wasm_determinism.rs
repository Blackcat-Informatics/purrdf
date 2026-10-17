// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native fixed-point ranking expectations: score lexicals, the integer
//! logarithm, independent fielded-scoring references and pinned sentence bytes. The full target runs
//! under `cargo test --workspace`; generic arithmetic remains native Rust.
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

mod support {
    pub(crate) mod scoped_candidates;
    pub(crate) mod sentence_cases;
}

use support::scoped_candidates::{
    scoped_text_bound_candidates_retain_unknown_membership,
    scoped_text_depth_preserves_original_unknown_rows_and_scores,
    scoped_text_preserves_original_constraint_failure,
};

use std::sync::Arc;
use support::sentence_cases::the_sentence_boundaries_are_reproduced_on_this_target;

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
    let dataset = corpus_dataset(&CORPUS);
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

fn corpus_dataset(corpus: &[(&str, &str)]) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let note = builder.intern_iri(NOTE).unwrap();
    for &(local, text) in corpus {
        let subject = builder
            .intern_iri(&format!("https://example.org/{local}"))
            .unwrap();
        let literal = builder.intern_literal(RdfLiteral::simple(text)).unwrap();
        builder.push_quad(subject, note, literal, None).unwrap();
    }
    builder.freeze().expect("the fixture must validate")
}

fn delta_updates_reproduce_fresh_ranked_and_surface_answers_on_this_target() {
    let original = corpus_index();
    let old_fingerprint = original.fingerprint();
    let snapshot = original.clone();
    let additions = [("d0", "quick brown quick quick")];
    let removed = corpus_dataset(&CORPUS[..1]);
    let added = corpus_dataset(&additions);
    let next_rows: Vec<_> = CORPUS[1..].iter().chain(&additions).copied().collect();
    let next = corpus_dataset(&next_rows);
    let fresh = TextIndex::from_dataset(&*next, original.config()).unwrap();
    let updated = original.with_delta(&*next, &*added, &*removed).unwrap();
    assert_eq!(updated.fingerprint(), fresh.fingerprint());
    assert_eq!(updated.source_fingerprint(), fresh.source_fingerprint());
    let needle = Analyzer::empty_lexicon().terms("quick brown").unwrap();
    assert_eq!(
        select(
            &updated,
            &needle,
            &PartitionFilter::unconstrained(),
            None,
            None
        )
        .unwrap(),
        select(
            &fresh,
            &needle,
            &PartitionFilter::unconstrained(),
            None,
            None
        )
        .unwrap()
    );
    for needle in ["qu", "brown", "river"] {
        assert_eq!(
            format!(
                "{:?}",
                updated.surface_index().unwrap().substring_report(needle)
            ),
            format!(
                "{:?}",
                fresh.surface_index().unwrap().substring_report(needle)
            )
        );
    }
    assert_eq!(snapshot.fingerprint(), old_fingerprint);
    let empty = corpus_dataset(&[]);
    let emptied = updated.with_delta(&*empty, &*empty, &*next).unwrap();
    assert_eq!(
        emptied.fingerprint(),
        TextIndex::from_dataset(&*empty, emptied.config())
            .unwrap()
            .fingerprint()
    );
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("urn:subject").unwrap();
    let predicate = builder.intern_iri(NOTE).unwrap();
    let object = builder.intern_iri("urn:object").unwrap();
    builder.push_quad(subject, predicate, object, None).unwrap();
    let nonliteral = builder.freeze().unwrap();
    let generation = emptied.fingerprint();
    let covered = emptied
        .with_delta(&*nonliteral, &*nonliteral, &*empty)
        .unwrap();
    assert_eq!(covered.fingerprint(), generation);
    assert!(covered.source_coverage().dataset_holds_statements());
    assert_eq!(covered.source_coverage().unrepresented_predicates(), []);
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

#[path = "support/unrestricted_ranking_cases.rs"]
mod unrestricted_ranking;
use unrestricted_ranking::{
    declared_large_corpora_are_exact, five_thousand_term_index, promoted_field_arithmetic_is_exact,
    thirty_two_index_fields_are_exact,
};

/// All fielded scoring vectors, including maximum bounds, agree on this target.
fn the_independent_fielded_reference_is_reproduced_on_this_target() {
    bm25f_reference::verify_reference_corpus();
}

purrdf_testkit::harness_main!(
    delta_updates_reproduce_fresh_ranked_and_surface_answers_on_this_target,
    scoped_text_bound_candidates_retain_unknown_membership,
    scoped_text_depth_preserves_original_unknown_rows_and_scores,
    scoped_text_preserves_original_constraint_failure,
    the_independent_fielded_reference_is_reproduced_on_this_target,
    the_integer_logarithm_agrees_with_its_hand_values_on_this_target,
    the_pinned_ranking_is_reproduced_on_this_target,
    the_sentence_boundaries_are_reproduced_on_this_target,
    declared_large_corpora_are_exact,
    five_thousand_term_index,
    promoted_field_arithmetic_is_exact,
    thirty_two_index_fields_are_exact,
);
