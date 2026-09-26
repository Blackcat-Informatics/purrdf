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
//! The expected report is compared on the tuple
//! `(focusNode, resultPath, value, sourceConstraintComponent, severity,
//! sourceShape)` as a MULTISET, plus `sh:conforms`; see [`super::norm`] for the blank-node
//! normalization and the harness module docs for what is deliberately not
//! compared. `sht:Failure` means the input must be REJECTED: any `Err` from
//! loading or validation passes, a successful validation fails.
//!
//! Two parts of the expected report are more than a comparison:
//!
//! * its `sh:conformanceDisallows` values are the VALIDATION PARAMETER the case
//!   runs under ("the test framework needs to use the values of
//!   sh:conformanceDisallows from the mf:result", W3C
//!   `core/validation-reports/conformance-disallows-001`), so the case is
//!   validated with exactly that set, and the report must echo it back; and
//! * every `sh:resultMessage` it mentions must be carried by a produced result
//!   with the same tuple ("the test harness needs to preserve all
//!   sh:resultMessage triples that are mentioned in the 'expected' results
//!   graph", W3C `core/misc/message-001`). The comparison is EXACT: the produced
//!   result's message set — each literal with its language tag, direction and
//!   datatype — must equal the expected set. A result the expected report states
//!   no message for is not graded on messages; and
//! * every SHACL-SPARQL result annotation it mentions — a predicate of an
//!   expected result outside `rdf:type` and the SHACL namespaces — must be
//!   carried the same way: the produced result with the same tuple has EXACTLY
//!   the expected `(property, value)` set (SHACL 1.2 SPARQL Extensions,
//!   "Annotation Properties").
//!
//! Every function here returns a verdict; none asserts one. The harness decides
//! what a verdict means against its ledger.

use std::fs;

use purrdf_shapes::engine::ValidationOptions;
use purrdf_shapes::report::{ConformanceDisallows, ValidationReport};

use std::collections::BTreeSet;

use purrdf_shapes::term::Term;

use super::{Expected, Multiset, Tuple, W3cCase, file_iri, norm};

/// Load graphs, run the engine. `Err` carries the parse/validation error.
fn validate_case(tc: &W3cCase) -> Result<ValidationReport, String> {
    let shapes_text = fs::read_to_string(&tc.shapes_path)
        .map_err(|e| format!("cannot read shapes {}: {e}", tc.shapes_path.display()))?;
    let purrdf_shapes::text_ingest::TurtleDocument {
        dataset: shapes_dataset,
        prefixes: doc_prefixes,
        ..
    } = purrdf_shapes::text_ingest::parse_turtle_document(
        &shapes_text,
        Some(&file_iri(&tc.shapes_path)),
    )
    .map_err(|errors| format!("shapes graph parse error: {}", errors.join("; ")))?;
    let shapes_graph_iri = tc.shapes_graph_iri.as_deref();
    let shapes = purrdf_shapes::shapes::from_dataset_with_base(
        &shapes_dataset,
        None,
        &doc_prefixes,
        None,
        shapes_graph_iri.map(ToOwned::to_owned),
        &super::w3c_case_imports(&shapes_dataset),
    )
    .map_err(|e| format!("shapes parse error: {e}"))?;

    let data_dataset = if tc.data_path == tc.shapes_path {
        shapes_dataset
    } else {
        super::parse_turtle_file(&tc.data_path)
            .map_err(|e| format!("data graph parse error: {e}"))?
    };

    let mut shapes = shapes;
    if !tc.conformance_disallows.is_empty() {
        let disallows = ConformanceDisallows::from_iris(&tc.conformance_disallows)
            .map_err(|e| format!("the expected report's sh:conformanceDisallows: {e}"))?;
        shapes.set_validation_options(
            ValidationOptions::default().with_conformance_disallows(disallows),
        );
    }

    purrdf_shapes::engine::validate_dataset_with_shapes_graph(
        data_dataset.as_ref(),
        &shapes,
        shapes_graph_iri,
    )
    .map_err(|e| format!("validation error: {e}"))
}

/// The two report checks beyond the tuple multiset (see the module docs): the
/// requested conformance-disallow set is echoed, and every expected message is
/// carried.
fn grade_report_details(tc: &W3cCase, report: &ValidationReport) -> Result<(), String> {
    if !tc.conformance_disallows.is_empty() {
        let echoed =
            purrdf_shapes::report::conformance_disallows_from_dataset(&report.to_dataset())
                .map_err(|e| format!("the report's own sh:conformanceDisallows: {e}"))?;
        let requested = ConformanceDisallows::from_iris(&tc.conformance_disallows)
            .map_err(|e| format!("the expected report's sh:conformanceDisallows: {e}"))?;
        if echoed != requested {
            return Err(format!(
                "the report echoes sh:conformanceDisallows {:?}, not the requested {:?}",
                echoed.iris(),
                requested.iris()
            ));
        }
    }
    // Each expected result that states messages claims one produced result with
    // the same tuple whose message set is EXACTLY the expected set; a claimed
    // result is not claimed twice, so two expected results need two produced ones.
    let mut unclaimed: Vec<(Tuple, BTreeSet<String>)> = report
        .results
        .iter()
        .map(|r| {
            (
                result_tuple(r),
                r.messages
                    .iter()
                    .map(|m| super::message_key(&Term::Literal(m.clone())))
                    .collect(),
            )
        })
        .collect();
    for (tuple, messages) in &tc.expected_messages {
        let Some(position) = unclaimed
            .iter()
            .position(|(produced, carried)| produced == tuple && carried == messages)
        else {
            let produced: Vec<&BTreeSet<String>> = unclaimed
                .iter()
                .filter(|(produced, _)| produced == tuple)
                .map(|(_, carried)| carried)
                .collect();
            return Err(format!(
                "no produced result {tuple:?} carries exactly the expected sh:resultMessage \
                 set {messages:?}; the results with that tuple carry {produced:?}"
            ));
        };
        unclaimed.swap_remove(position);
    }
    // The same claim for result annotations.
    let mut unclaimed: Vec<(Tuple, BTreeSet<(String, String)>)> = report
        .results
        .iter()
        .map(|r| {
            (
                result_tuple(r),
                r.annotations
                    .iter()
                    .map(|(property, value)| (format!("<{}>", property.as_str()), norm(value)))
                    .collect(),
            )
        })
        .collect();
    for (tuple, annotations) in &tc.expected_annotations {
        let Some(position) = unclaimed
            .iter()
            .position(|(produced, carried)| produced == tuple && carried == annotations)
        else {
            return Err(format!(
                "no produced result {tuple:?} carries exactly the expected result annotations \
                 {annotations:?}"
            ));
        };
        unclaimed.swap_remove(position);
    }
    Ok(())
}

/// The comparison tuple of one produced result.
fn result_tuple(r: &purrdf_shapes::report::ValidationResult) -> Tuple {
    (
        norm(&r.focus_node),
        r.result_path.as_ref().map(norm),
        r.value.as_ref().map(norm),
        format!("<{}>", r.source_constraint_component.as_str()),
        format!("<{}>", r.severity.iri()),
        norm(&r.source_shape),
    )
}

/// Multiset of comparison tuples the engine produced.
fn produced_multiset(report: &ValidationReport) -> Multiset {
    let mut multiset = Multiset::new();
    for r in &report.results {
        *multiset.entry(result_tuple(r)).or_insert(0) += 1;
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
    grade_against(tc, &tc.expected)
}

/// Grade one `sht:Validate` case against `expected` — the approved expectation,
/// or an amended one — with the [`grade`] comparison and the report details the
/// case itself asks for.
pub(crate) fn grade_against(tc: &W3cCase, expected: &Expected) -> Result<(), String> {
    let report = no_panic(|| validate_case(tc));
    grade(
        expected,
        report
            .as_ref()
            .map(|report| (report.conforms, produced_multiset(report)))
            .map_err(Clone::clone),
    )?;
    match report {
        Ok(report) => grade_report_details(tc, &report),
        Err(_) => Ok(()),
    }
}

/// Run the engine over one case and reduce its report to what [`grade`]
/// compares: `sh:conforms` and the result multiset. `Err` is a load or
/// validation error (or an engine panic).
pub(crate) fn produce(tc: &W3cCase) -> Result<(bool, Multiset), String> {
    no_panic(|| validate_case(tc)).map(|report| (report.conforms, produced_multiset(&report)))
}

/// Grade an engine outcome against an expectation — the one comparison both
/// harnesses use, split from [`produce`] so a harness that must grade against an
/// AMENDED expectation compares with exactly the same rule.
pub(crate) fn grade(
    expected: &Expected,
    outcome: Result<(bool, Multiset), String>,
) -> Result<(), String> {
    match (expected, outcome) {
        (Expected::Failure, Err(_)) => Ok(()),
        (Expected::Failure, Ok(_)) => {
            Err("suite expects sht:Failure but the engine validated successfully".to_owned())
        }
        (Expected::Report { .. }, Err(e)) => Err(e),
        (Expected::Report { conforms, results }, Ok((produced_conforms, produced))) => {
            if produced_conforms != *conforms {
                return Err(format!(
                    "conforms mismatch: produced={produced_conforms}, expected={conforms}"
                ));
            }
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
