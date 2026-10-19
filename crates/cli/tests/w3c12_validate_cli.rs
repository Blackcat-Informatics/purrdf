// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Every approved W3C SHACL 1.2 `sht:Validate` entry, run through the BUILT `purrdf validate
//! --shacl-profile shacl-ed-2026-10-16` binary — the production entry point, not the library
//! harness — and graded against its approved expectation.
//!
//! Discovery is the library harness's own corpus reader (`purrdf_shapes::shacl_corpora`)
//! with its drift guards. Grading is the one `sht:Validate` grader
//! (`shacl_corpora::report_grading::grade`) applied to the report graph the binary wrote:
//! `shacl_corpora::report_graph_outcome` reduces that N-Triples report to `sh:conforms` and
//! the multiset of `(focusNode, resultPath, value, sourceConstraintComponent, severity,
//! sourceShape)` tuples, exactly as the manifest reader reduces the approved `mf:result`.
//! An entry whose approved result is `sht:Failure` must be refused by the binary at run time
//! (exit 1). The entry's `sh:conformanceDisallows` is passed as `--conformance-disallows`,
//! the way the suite asks a framework to use it, and its `sht:shapesGraph` as
//! `--shapes-graph`. The shapes and data graphs are read under their own `file://` IRIs.
//!
//! This proves the 1.2 `sht:Validate` result on a shipped surface: the same law the
//! library harness and the EARL report grade under is the one a user selects on the CLI.

use std::process::Command;

use purrdf_shapes::shacl_corpora::report_grading::grade;
use purrdf_shapes::shacl_corpora::shacl12::{Body, shacl12_cases};
use purrdf_shapes::shacl_corpora::{Expected, Multiset, W3cCase, file_iri, report_graph_outcome};

/// The law the 1.2 suite is graded under.
const ED: &str = "shacl-ed-2026-10-16";

/// The discovered `sht:Validate` entries, restated so this lane cannot pass by running fewer.
const VALIDATE_CASES: usize = 213;

/// Run one entry through the binary and reduce the report it wrote to what the grader compares.
fn produce(tc: &W3cCase) -> Result<(bool, Multiset), String> {
    let shapes = tc.shapes_path.to_str().ok_or("a non-UTF-8 corpus path")?;
    let data = tc.data_path.to_str().ok_or("a non-UTF-8 corpus path")?;
    let mut command = Command::new(env!("CARGO_BIN_EXE_purrdf"));
    command.args([
        "validate",
        "--shacl-profile",
        ED,
        "--shapes",
        shapes,
        "--shapes-base",
        &file_iri(&tc.shapes_path),
        "--base",
        &file_iri(&tc.data_path),
    ]);
    if let Some(iri) = &tc.shapes_graph_iri {
        command.args(["--shapes-graph", iri]);
    }
    for iri in &tc.conformance_disallows {
        command.args(["--conformance-disallows", iri]);
    }
    command.args([data, "-"]);
    let out = command.output().map_err(|e| format!("spawn purrdf: {e}"))?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    match out.status.code() {
        Some(0) => {
            let report = purrdf_rdf::parse_dataset(&out.stdout, "application/n-triples", None)
                .map_err(|e| format!("the report is not N-Triples: {e}"))?;
            report_graph_outcome(&report)
        }
        // A refusal the binary decided: a runtime failure, never a usage error or a crash.
        Some(1) => Err(format!("refused (exit 1): {stderr}")),
        other => Err(format!("exit {other:?}: {stderr}")),
    }
}

/// Run one entry through the binary and grade the report it wrote.
fn run_case(tc: &W3cCase) -> Result<(), String> {
    grade(&tc.expected, produce(tc))
}

#[test]
fn every_approved_w3c_validate_entry_passes_through_the_cli_under_the_editors_draft() {
    let cases = shacl12_cases();
    let validate: Vec<(&str, &W3cCase, bool)> = cases
        .iter()
        .filter_map(|case| match &case.body {
            Body::Validate(tc) => Some((case.id.as_str(), tc, case.is_approved_shacl_root())),
            _ => None,
        })
        .collect();
    assert_eq!(
        validate.len(),
        VALIDATE_CASES,
        "discovered sht:Validate entries"
    );
    assert!(
        validate.iter().all(|(_, _, approved)| *approved),
        "every sht:Validate entry is an approved one an upstream manifest lists"
    );

    let mut passed = 0usize;
    let mut refused_as_declared = 0usize;
    let mut errors: Vec<String> = Vec::new();
    for (id, tc, _) in &validate {
        match run_case(tc) {
            Ok(()) => {
                passed += 1;
                refused_as_declared += usize::from(matches!(tc.expected, Expected::Failure));
            }
            Err(e) => errors.push(format!("FAIL [{id}]: {e}")),
        }
    }
    println!(
        "W3C SHACL 1.2 sht:Validate through `purrdf validate --shacl-profile {ED}`: passed \
         {passed} ({refused_as_declared} refused as sht:Failure), failed {}",
        errors.len()
    );
    assert!(
        errors.is_empty(),
        "{} entr(ies) failed through the CLI:\n{}",
        errors.len(),
        errors.join("\n\n")
    );
    assert_eq!(passed, VALIDATE_CASES, "every entry is a pass");
}

/// The lane's grader is not vacuous: the produced report of an entry that has results is
/// graded against its own approved expectation (a pass), and then against a flipped verdict,
/// a dropped result and `sht:Failure` (each a mismatch).
#[test]
fn the_lane_grades_the_report_it_reads_and_does_not_pass_a_wrong_expectation() {
    let cases = shacl12_cases();
    let tc = cases
        .iter()
        .find_map(|case| match &case.body {
            Body::Validate(tc)
                if matches!(&tc.expected, Expected::Report { results, .. } if !results.is_empty()) =>
            {
                Some(tc)
            }
            _ => None,
        })
        .expect("an entry that expects results");
    let Expected::Report { conforms, results } = &tc.expected else {
        unreachable!("selected for its expected report");
    };
    let produced = produce(tc).expect("the entry validates through the CLI");
    assert!(grade(&tc.expected, Ok(produced.clone())).is_ok());
    let flipped = Expected::Report {
        conforms: !conforms,
        results: results.clone(),
    };
    assert!(grade(&flipped, Ok(produced.clone())).is_err());
    let mut fewer = results.clone();
    let first = fewer.keys().next().expect("a result").clone();
    fewer.remove(&first);
    let dropped = Expected::Report {
        conforms: *conforms,
        results: fewer,
    };
    assert!(grade(&dropped, Ok(produced.clone())).is_err());
    assert!(grade(&Expected::Failure, Ok(produced)).is_err());
}
