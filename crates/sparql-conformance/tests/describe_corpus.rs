// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The first-party **DESCRIBE** conformance corpus
//! (`crates/sparql-conformance/corpus/describe/`).
//!
//! SPARQL 1.1 §16.4 leaves the description implementation-defined, so no W3C
//! manifest can grade a `DESCRIBE`. That is precisely why a first-party corpus
//! is the deliverable: it states, case by case, what THIS engine returns — the
//! Symmetric Concise Bounded Description — so a downstream consumer can read
//! the behavior off a scoreboard row and a golden file instead of off a claim.
//!
//! The scoreboard line `DESCRIBE-CORPUS: passed N total M` is what
//! `scripts/conformance-matrix.py` scrapes. See `construct_corpus.rs` for why
//! these corpora live under `corpus/` rather than `suite/`.

use std::path::PathBuf;

use purrdf_sparql_conformance::manifest::TestKind;

/// The corpus manifest.
fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("corpus")
        .join("describe")
        .join("manifest.ttl")
}

/// Every case in the corpus must pass — no xfail ledger (see `construct_corpus.rs`).
#[test]
fn describe_corpus_is_green() {
    let manifest = manifest();
    assert!(manifest.is_file(), "corpus manifest missing: {manifest:?}");

    let total = purrdf_sparql_conformance::manifest::load(&manifest)
        .unwrap_or_else(|e| panic!("load the DESCRIBE corpus manifest: {e}"))
        .len();
    let summary = purrdf_sparql_conformance::run_manifest(&manifest)
        .unwrap_or_else(|e| panic!("run the DESCRIBE corpus: {e}"));

    println!("DESCRIBE-CORPUS: passed {} total {total}", summary.passed);
    assert!(
        summary.is_ok(),
        "DESCRIBE corpus failed:\n{}",
        summary.failure_report()
    );
    assert_eq!(
        summary.xfail, 0,
        "the first-party DESCRIBE corpus carries no xfail ledger"
    );
    assert_eq!(
        summary.passed, total,
        "every declared DESCRIBE case must run and pass"
    );
}

/// Count-and-kind tripwire, for the same reason the CONSTRUCT corpus has one.
#[test]
fn describe_corpus_case_count_and_kinds() {
    let cases = purrdf_sparql_conformance::manifest::load(&manifest())
        .unwrap_or_else(|e| panic!("load the DESCRIBE corpus manifest: {e}"));

    assert_eq!(cases.len(), 13, "the corpus declares 13 cases");
    assert_eq!(
        cases
            .iter()
            .filter(|c| c.kind == TestKind::QueryEval)
            .count(),
        11
    );
    assert_eq!(
        cases
            .iter()
            .filter(|c| c.kind == TestKind::PositiveSyntax)
            .count(),
        1
    );
    assert_eq!(
        cases
            .iter()
            .filter(|c| c.kind == TestKind::NegativeSyntax)
            .count(),
        1
    );
    assert_eq!(
        cases.iter().filter(|c| c.kind == TestKind::Unknown).count(),
        0,
        "an unmodeled case would be a silent skip"
    );
}

/// The corpus's anti-tautology check: at least one case must pin a NON-EMPTY
/// description, and at least one must pin an EMPTY one. A corpus whose every
/// expectation was empty would be green against an engine that described
/// nothing at all.
#[test]
fn the_corpus_pins_both_non_empty_and_empty_descriptions() {
    let cases = purrdf_sparql_conformance::manifest::load(&manifest())
        .unwrap_or_else(|e| panic!("load the DESCRIBE corpus manifest: {e}"));

    let mut non_empty = 0_usize;
    let mut empty = 0_usize;
    for case in &cases {
        if case.kind != TestKind::QueryEval {
            continue;
        }
        let purrdf_sparql_conformance::manifest::ExpectedResult::Graph(result) = &case.expected
        else {
            panic!("{} must expect a graph result", case.iri);
        };
        let statements = std::fs::read_to_string(result)
            .expect("read the expected result")
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("@prefix"))
            .count();
        if statements == 0 {
            empty += 1;
        } else {
            non_empty += 1;
        }
    }
    assert!(
        non_empty >= 5,
        "the corpus must pin real descriptions, saw {non_empty}"
    );
    assert!(
        empty >= 2,
        "the corpus must pin the two empty-description rules, saw {empty}"
    );
}

/// Clause 4 of the symmetric CBD — the RDF 1.2 statement layer — must actually be
/// measured, on BOTH sides of its "reified subject *or* object" disjunction.
///
/// Without this, a corpus could satisfy every check above while containing no
/// RDF 1.2 syntax at all, which is exactly how a whole clause of the definition
/// the corpus exists to pin can go ungraded.
#[test]
fn the_corpus_measures_the_rdf_12_statement_layer() {
    let cases = purrdf_sparql_conformance::manifest::load(&manifest())
        .unwrap_or_else(|e| panic!("load the DESCRIBE corpus manifest: {e}"));

    let mut reifying_expectations = 0_usize;
    let mut annotated_expectations = 0_usize;
    for case in &cases {
        let purrdf_sparql_conformance::manifest::ExpectedResult::Graph(result) = &case.expected
        else {
            continue;
        };
        let expected = std::fs::read_to_string(result).expect("read the expected result");
        // A reifier is declared either by the `~` reifier syntax or by an explicit
        // `rdf:reifies` edge onto a `<<( … )>>` triple term.
        if expected.contains("rdf:reifies") || expected.contains(" ~") {
            reifying_expectations += 1;
        }
        if expected.contains("{|") || expected.contains("rdf:reifies") {
            annotated_expectations += 1;
        }
    }
    assert!(
        reifying_expectations >= 3,
        "clause 4 must be pinned by real reifier expectations, saw {reifying_expectations}"
    );
    assert!(
        annotated_expectations >= 3,
        "clause 4's annotations must be pinned too, saw {annotated_expectations}"
    );

    // The object half of the disjunction is the one a subject-only describer would
    // silently pass, so it is named rather than merely counted: its fixture reifies
    // an UNASSERTED triple, so every statement in the expectation had to be selected
    // through the reified triple's object.
    let object_half = cases
        .iter()
        .find(|c| c.iri.ends_with("statementLayerObject"))
        .expect("the object half of clause 4 must be a declared case");
    let purrdf_sparql_conformance::manifest::ExpectedResult::Graph(result) = &object_half.expected
    else {
        panic!("the object-half case must expect a graph result");
    };
    let expected = std::fs::read_to_string(result).expect("read the expected result");
    assert!(
        expected.contains("rdf:reifies"),
        "the object-half expectation must carry the reifier it selected"
    );
    assert!(
        !expected.contains(":mentions :q ."),
        "the object-half fixture's reified triple must stay unasserted, or the case \
         no longer isolates the object side of the disjunction"
    );
}
