// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Integration test: run every corpus case through the real validator and compare
//! against the frozen expected report twice: by normalised tuple set, and as a
//! whole report GRAPH under RDF isomorphism (RDFC-1.0), so everything the report
//! carries beyond the tuple — nested `sh:detail` results, complex-path
//! structure, types — is graded too. The one predicate the graph comparison
//! leaves out is `sh:resultMessage`: the frozen reports record the messages a
//! case is about and not every engine-generated one.
//!
//! Case discovery and the exact case count live in [`shacl_corpora`], the one
//! reader for this crate's two SHACL corpora, so this file is only the grading
//! half.

mod shacl_corpora;

use std::collections::BTreeSet;
use std::fs;

use purrdf_shapes::report::{conforms_from_ntriples, tuples_from_ntriples};

use shacl_corpora::{first_party_box_role_vocab, first_party_cases};

#[test]
fn conformance_corpus() {
    // The corpus relation, installed for every case. One case names it; no other case
    // can, because it sits in its own namespace. Installing it for all of them is what
    // keeps this harness and the product-equivalence harness grading the same corpus
    // under the same environment.
    let (corpus_relations, corpus_opens) = shacl_corpora::corpus_relations();
    let _relations = purrdf_shapes::sparql::enter_property_function_scope(corpus_relations);
    let cases = first_party_cases();

    let mut failures: Vec<String> = Vec::new();
    let mut passed = 0usize;

    for case in &cases {
        let case_failures_before = failures.len();
        let case_name = &case.name;

        let data_nt = fs::read_to_string(&case.data_path)
            .unwrap_or_else(|e| panic!("case {case_name}: cannot read data.nt: {e}"));
        let shapes_ttl = fs::read_to_string(&case.shapes_path)
            .unwrap_or_else(|e| panic!("case {case_name}: cannot read shapes.ttl: {e}"));
        let expected_nt = fs::read_to_string(&case.expected_report_path)
            .unwrap_or_else(|e| panic!("case {case_name}: cannot read expected-report.nt: {e}"));

        // Run the validator with the corpus box-role vocabulary configured.
        let report = match purrdf_shapes::engine::validate_graphs_with_config(
            &data_nt,
            &shapes_ttl,
            None,
            Some(first_party_box_role_vocab()),
            &purrdf_shapes::ShapesImports::new(),
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

        // Compare the report graphs, sh:resultMessage aside.
        let produced_graph = canonical_without_messages(&report.to_ntriples());
        let expected_graph = canonical_without_messages(&expected_nt);
        if produced_graph != expected_graph {
            failures.push(format!(
                "[{case_name}] report graph differs from the frozen report (RDFC-1.0, \
                 sh:resultMessage aside):\n  PRODUCED:\n{}\n  EXPECTED:\n{}",
                report.to_ntriples(),
                expected_nt
            ));
        }

        if failures.len() == case_failures_before {
            passed += 1;
        }
    }

    // Fixture-level scoreboard (surfaced under `--nocapture`) so the conformance
    // matrix scrapes a per-report count instead of the single test-function tally.
    println!("SHAPES-CORPUS: passed {passed} total {}", cases.len());

    assert!(
        failures.is_empty(),
        "conformance_corpus: {} case(s) failed:\n{}",
        failures.len(),
        failures.join("\n\n")
    );

    // The relation was actually REACHED. Without this the relation case would pass
    // over an environment that resolved nothing — the call lowered to an ordinary
    // triple pattern, the body matching nothing — which is indistinguishable from a
    // correctly-resolved relation over no matching rows, and is exactly the silent
    // outcome the case exists to rule out.
    assert!(
        corpus_opens.load(std::sync::atomic::Ordering::Relaxed) > 0,
        "no corpus case reached the corpus relation, so the relation case is grading \
         an empty environment rather than a resolved call",
    );
}

/// `sh:resultMessage`, the one predicate the graph comparison leaves out.
const RESULT_MESSAGE: &str = "<http://www.w3.org/ns/shacl#resultMessage>";

/// The RDFC-1.0 canonical N-Quads of a report's N-Triples without its
/// `sh:resultMessage` triples. N-Triples holds one triple per line, and every
/// report subject is a blank node or an IRI, so a line's predicate is its second
/// whitespace-separated token.
fn canonical_without_messages(nt: &str) -> String {
    let kept: String = nt
        .lines()
        .filter(|line| line.split_whitespace().nth(1) != Some(RESULT_MESSAGE))
        .map(|line| format!("{line}\n"))
        .collect();
    let dataset = purrdf::parse_dataset(kept.as_bytes(), "application/n-triples", None)
        .unwrap_or_else(|e| panic!("a report's N-Triples parse: {e}"));
    purrdf::canonicalize(dataset.as_ref()).nquads
}
