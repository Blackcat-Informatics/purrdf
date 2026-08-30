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

    assert_eq!(cases.len(), 10, "the corpus declares 10 cases");
    assert_eq!(
        cases
            .iter()
            .filter(|c| c.kind == TestKind::QueryEval)
            .count(),
        8
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
