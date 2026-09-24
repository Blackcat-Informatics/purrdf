// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **The one `sht:Validate` grader** shared by the W3C SHACL 1.0 and SHACL 1.2
//! harnesses.
//!
//! Both suites spell a validation case the same way — an `mf:action` naming a
//! shapes graph and a data graph, and an `mf:result` that is either an expected
//! `sh:ValidationReport` or `sht:Failure` — so both are graded by this one
//! function. Two harnesses carrying two copies of the comparison would be free
//! to drift into two different meanings of "agrees with the manifest".
//!
//! ## Comparison contract
//!
//! The expected report is compared on the tuple subset
//! `(focusNode, resultPath, value, sourceConstraintComponent, severity)` as a
//! MULTISET, plus `sh:conforms`; see [`super::norm`] for the blank-node
//! normalization and the harness module docs for what is deliberately not
//! compared. `sht:Failure` means the input must be REJECTED: any `Err` from
//! loading or validation passes, a successful validation fails.
//!
//! Every function here returns a verdict; none asserts one. The harness decides
//! what a verdict means against its ledger.

use std::fs;

use super::{Expected, Multiset, W3cCase, file_iri, norm};

/// Load graphs, run the engine. `Err` carries the parse/validation error.
fn validate_case(tc: &W3cCase) -> Result<purrdf_shapes::report::ValidationReport, String> {
    let shapes_text = fs::read_to_string(&tc.shapes_path)
        .map_err(|e| format!("cannot read shapes {}: {e}", tc.shapes_path.display()))?;
    let shapes_dataset = purrdf::parse_dataset(
        shapes_text.as_bytes(),
        "text/turtle",
        Some(&file_iri(&tc.shapes_path)),
    )
    .map_err(|e| format!("shapes graph parse error: {e}"))?;
    let doc_prefixes = purrdf_shapes::text_ingest::extract_prefixes(&shapes_text);
    let shapes_graph_iri = tc.shapes_graph_iri.as_deref();
    let shapes = purrdf_shapes::shapes::from_dataset_with_config_and_graph(
        &shapes_dataset,
        &doc_prefixes,
        None,
        shapes_graph_iri.map(ToOwned::to_owned),
    )
    .map_err(|e| format!("shapes parse error: {e}"))?;

    let data_dataset = if tc.data_path == tc.shapes_path {
        shapes_dataset
    } else {
        super::parse_turtle_file(&tc.data_path)
            .map_err(|e| format!("data graph parse error: {e}"))?
    };

    purrdf_shapes::engine::validate_dataset_with_shapes_graph(
        data_dataset.as_ref(),
        &shapes,
        shapes_graph_iri,
    )
    .map_err(|e| format!("validation error: {e}"))
}

/// Multiset of comparison tuples the engine produced.
fn produced_multiset(report: &purrdf_shapes::report::ValidationReport) -> Multiset {
    let mut multiset = Multiset::new();
    for r in &report.results {
        let focus = norm(&r.focus_node);
        let path = r.result_path.as_ref().map(norm);
        let value = r.value.as_ref().map(norm);
        let component = format!("<{}>", r.source_constraint_component.as_str());
        let severity = format!("<{}>", r.severity.iri());
        *multiset
            .entry((focus, path, value, component, severity))
            .or_insert(0) += 1;
    }
    multiset
}

/// Run `f`, reporting an engine panic as an `Err` verdict rather than aborting
/// the whole harness.
///
/// No known case panics; the guard stays so a regression reads as a FAIL with a
/// message instead of a harness abort that hides every later verdict.
pub(crate) fn no_panic<T>(f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_else(|payload| {
        let msg = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("<non-string panic payload>");
        Err(format!("engine panicked: {msg}"))
    })
}

/// Grade one `sht:Validate` case to a pass (`Ok`) / fail-with-reason (`Err`).
pub(crate) fn run_validate_case(tc: &W3cCase) -> Result<(), String> {
    let outcome = no_panic(|| validate_case(tc));
    match (&tc.expected, outcome) {
        (Expected::Failure, Err(_)) => Ok(()),
        (Expected::Failure, Ok(_)) => {
            Err("suite expects sht:Failure but the engine validated successfully".to_owned())
        }
        (Expected::Report { .. }, Err(e)) => Err(e),
        (Expected::Report { conforms, results }, Ok(report)) => {
            if report.conforms != *conforms {
                return Err(format!(
                    "conforms mismatch: produced={}, expected={conforms}",
                    report.conforms
                ));
            }
            let produced = produced_multiset(&report);
            if &produced != results {
                return Err(multiset_diff(results, &produced));
            }
            Ok(())
        }
    }
}

/// Human-readable multiset diff for the failure message.
fn multiset_diff(expected: &Multiset, produced: &Multiset) -> String {
    let mut lines = vec!["result multiset mismatch:".to_owned()];
    for (tuple, n) in expected {
        let have = produced.get(tuple).copied().unwrap_or(0);
        if have != *n {
            lines.push(format!("  expected x{n}, produced x{have}: {tuple:?}"));
        }
    }
    for (tuple, n) in produced {
        if !expected.contains_key(tuple) {
            lines.push(format!("  expected x0, produced x{n}: {tuple:?}"));
        }
    }
    lines.join("\n")
}
