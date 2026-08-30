// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The first-party **CONSTRUCT** conformance corpus
//! (`crates/sparql-conformance/corpus/construct/`).
//!
//! The corpus covers both `CONSTRUCT` forms, case for case: the SPARQL 1.1
//! §16.2 triple-producing form (expected results in Turtle, so every statement
//! must land in the DEFAULT graph) and the quad-producing
//! `CONSTRUCT GRAPH <iri>` form (expected results in N-Quads, carrying the
//! target graph on every line). A downstream consumer asking "is this query
//! form covered by conformance evidence" gets a scoreboard row, not a promise.
//!
//! # Why its own target, beside `suite/`
//!
//! `sparql_conformance.rs`'s `datatest_stable::harness!` is rooted at `suite/`
//! and folds every manifest it finds into ONE conformance-matrix row. This
//! corpus lives under `corpus/` instead so it reports its own row (and carries
//! its own ratchet budget in `scripts/conformance-baseline.json`) rather than
//! disappearing into the full-corpus tally, where a regression in it would move
//! a four-digit number by one.
//!
//! The scoreboard line `CONSTRUCT-CORPUS: passed N total M` is what
//! `scripts/conformance-matrix.py` scrapes.

use std::path::PathBuf;

use purrdf_sparql_conformance::manifest::TestKind;

/// The corpus manifest.
fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("corpus")
        .join("construct")
        .join("manifest.ttl")
}

/// Every case in the corpus must pass — there is no xfail ledger here, because a
/// first-party corpus that ledgers its own cases is a corpus that grades itself
/// against what the engine happens to do.
#[test]
fn construct_corpus_is_green() {
    let manifest = manifest();
    assert!(manifest.is_file(), "corpus manifest missing: {manifest:?}");

    let total = purrdf_sparql_conformance::manifest::load(&manifest)
        .unwrap_or_else(|e| panic!("load the CONSTRUCT corpus manifest: {e}"))
        .len();
    let summary = purrdf_sparql_conformance::run_manifest(&manifest)
        .unwrap_or_else(|e| panic!("run the CONSTRUCT corpus: {e}"));

    // The scoreboard line the conformance matrix scrapes. Printed BEFORE the
    // assertions so a red run still reports its tally.
    println!("CONSTRUCT-CORPUS: passed {} total {total}", summary.passed);
    assert!(
        summary.is_ok(),
        "CONSTRUCT corpus failed:\n{}",
        summary.failure_report()
    );
    assert_eq!(
        summary.xfail, 0,
        "the first-party CONSTRUCT corpus carries no xfail ledger"
    );
    assert_eq!(
        summary.passed, total,
        "every declared CONSTRUCT case must run and pass"
    );
}

/// Count-and-kind tripwire: the corpus is the deliverable, so its shape is
/// pinned rather than merely reported. A case silently dropped from
/// `mf:entries` — or a quad-form case quietly converted into a triple-form one —
/// changes these numbers.
#[test]
fn construct_corpus_case_count_and_kinds() {
    let cases = purrdf_sparql_conformance::manifest::load(&manifest())
        .unwrap_or_else(|e| panic!("load the CONSTRUCT corpus manifest: {e}"));

    assert_eq!(cases.len(), 14, "the corpus declares 14 cases");
    assert_eq!(
        cases
            .iter()
            .filter(|c| c.kind == TestKind::QueryEval)
            .count(),
        12
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

    // Both halves must be present. The quad-form cases are exactly those whose
    // query text names a target graph; if that half ever emptied out, the corpus
    // would still be "green" while measuring only the pre-existing form.
    let quad_form = cases
        .iter()
        .filter(|c| {
            std::fs::read_to_string(&c.query)
                .unwrap_or_default()
                .contains("CONSTRUCT GRAPH")
        })
        .count();
    assert_eq!(
        quad_form, 7,
        "seven cases must exercise the quad-producing CONSTRUCT GRAPH form"
    );
}

/// The quad-form evaluation cases must expect **N-Quads** results carrying the
/// target graph, and the triple-form ones must expect graph results that carry
/// none. This is the corpus's own anti-tautology check: an expected file that
/// silently lost its graph term would let a triple-emitting regression pass.
#[test]
fn quad_form_expectations_actually_name_a_graph() {
    let cases = purrdf_sparql_conformance::manifest::load(&manifest())
        .unwrap_or_else(|e| panic!("load the CONSTRUCT corpus manifest: {e}"));

    const TARGET_GRAPH: &str = "<http://example.org/out>";
    let mut graph_bearing_lines = 0_usize;
    for case in &cases {
        if case.kind != TestKind::QueryEval {
            continue;
        }
        let query = std::fs::read_to_string(&case.query).expect("read the case query");
        let purrdf_sparql_conformance::manifest::ExpectedResult::Graph(result) = &case.expected
        else {
            panic!("{} must expect a graph result", case.iri);
        };
        let expected = std::fs::read_to_string(result).expect("read the expected result");
        let statements: Vec<&str> = expected
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();

        if query.contains("CONSTRUCT GRAPH") {
            for line in &statements {
                assert!(
                    line.contains(TARGET_GRAPH),
                    "{}: a quad-form expectation must carry the target graph, got `{line}`",
                    case.iri
                );
                graph_bearing_lines += 1;
            }
        } else {
            for line in &statements {
                assert!(
                    !line.contains(TARGET_GRAPH),
                    "{}: a triple-form expectation must carry no graph term, got `{line}`",
                    case.iri
                );
            }
        }
    }
    assert!(
        graph_bearing_lines >= 6,
        "the quad-form expectations must pin real statements, saw {graph_bearing_lines}"
    );
}
