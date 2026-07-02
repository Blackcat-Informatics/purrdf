// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration test: run every corpus case through the real validator and compare
//! against the frozen expected report by normalised tuple set.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use purrdf_shapes::model::BoxRoleVocab;
use purrdf_shapes::report::{conforms_from_ntriples, tuples_from_ntriples};

const CORPUS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/corpus");

/// The corpus fixtures' caller-supplied box-role vocabulary: the reifier-shape
/// cases (38–42) annotate shapes with `meta:graphBoxRole` terms under
/// `https://example.org/meta/`. PurRDF mints no vocabulary of its own, so the
/// harness configures the vocab explicitly, exactly as a consumer would.
fn corpus_box_role_vocab() -> BoxRoleVocab {
    BoxRoleVocab::for_namespace("https://example.org/meta/")
}

#[test]
fn conformance_corpus() {
    let corpus_path = Path::new(CORPUS_DIR);
    assert!(
        corpus_path.exists(),
        "corpus directory not found at {CORPUS_DIR}"
    );

    let mut cases: Vec<_> = fs::read_dir(corpus_path)
        .expect("failed to read corpus dir")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_dir() {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    cases.sort();

    // Assert the EXACT case count (not merely non-empty) so a removed or renamed
    // corpus directory fails fast instead of silently reducing coverage. Bump this
    // when adding a case.
    assert_eq!(
        cases.len(),
        48,
        "unexpected corpus case count — update this when adding/removing a corpus case"
    );

    let mut failures: Vec<String> = Vec::new();

    for case_path in &cases {
        let case_name = case_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        let data_nt = fs::read_to_string(case_path.join("data.nt"))
            .unwrap_or_else(|e| panic!("case {case_name}: cannot read data.nt: {e}"));
        let shapes_ttl = fs::read_to_string(case_path.join("shapes.ttl"))
            .unwrap_or_else(|e| panic!("case {case_name}: cannot read shapes.ttl: {e}"));
        let expected_nt = fs::read_to_string(case_path.join("expected-report.nt"))
            .unwrap_or_else(|e| panic!("case {case_name}: cannot read expected-report.nt: {e}"));

        // Run the validator with the corpus box-role vocabulary configured.
        let report = match purrdf_shapes::engine::validate_graphs_with_config(
            &data_nt,
            &shapes_ttl,
            Some(corpus_box_role_vocab()),
        ) {
            Ok(r) => r,
            Err(e) => {
                failures.push(format!("[{case_name}] validate_graphs failed: {e}"));
                continue;
            }
        };

        // Compare conforms booleans.
        let expected_conforms = match conforms_from_ntriples(&expected_nt) {
            Ok(b) => b,
            Err(e) => {
                failures.push(format!("[{case_name}] conforms_from_ntriples failed: {e}"));
                continue;
            }
        };
        if report.conforms != expected_conforms {
            failures.push(format!(
                "[{case_name}] conforms mismatch: produced={}, expected={expected_conforms}",
                report.conforms
            ));
        }

        // Compare result tuple sets.
        let produced_tuples: BTreeSet<_> = report.result_tuples();
        let expected_tuples = match tuples_from_ntriples(&expected_nt) {
            Ok(t) => t,
            Err(e) => {
                failures.push(format!("[{case_name}] tuples_from_ntriples failed: {e}"));
                continue;
            }
        };

        if produced_tuples != expected_tuples {
            let only_expected: Vec<_> = expected_tuples.difference(&produced_tuples).collect();
            let only_produced: Vec<_> = produced_tuples.difference(&expected_tuples).collect();
            failures.push(format!(
                "[{case_name}] tuple set mismatch:\n  EXPECTED-ONLY: {only_expected:#?}\n  PRODUCED-ONLY: {only_produced:#?}"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "conformance_corpus: {} case(s) failed:\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
