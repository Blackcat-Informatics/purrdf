// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One case per discovered group manifest under `suite/`, named
//! `run_manifest_case::<path below suite/>`, and one per index manifest, named
//! `index_manifest::<path below suite/>`. Each runs all of its manifest's
//! cases (honoring the xfail registry) and prints a tally; a non-xfail failure
//! or a stale-xfail unexpected-pass fails the case.
//!
//! The target is `harness = false`: `main` discovers the manifests with
//! [`purrdf_sparql_conformance::discover`] and hands them to the
//! libtest-compatible [`purrdf_testkit::harness`] runner, which implements the
//! libtest command line (filters, `--exact`, `--skip`, `--list`, `--nocapture`,
//! `--test-threads`, …). Nothing else runs here, so a `#[test]` function written
//! in this file would be compiled and never called. The on-disk inventory
//! tripwires that guard against a suite directory disappearing are therefore in
//! `suite_inventory.rs`, which uses the ordinary libtest harness.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use purrdf_sparql_conformance::paths::SUITE_MANIFEST_NAME;
use purrdf_testkit::harness::{self, ERROR_EXIT_CODE, Failed, Trial};

/// The case-name prefix every discovered group manifest is run under.
const CASE_PREFIX: &str = "run_manifest_case::";

/// The case-name prefix every discovered index manifest is reported under.
const INDEX_PREFIX: &str = "index_manifest::";

/// Cursor parity is independent of the expected-failure registry.
const CURSOR_PREFIX: &str = "cursor_manifest_case::";

/// Ordinary claims with explicitly complete, closed input for native status parity.
struct ConformanceClaims;

impl purrdf_sparql_eval::EvidenceSource<purrdf_core::TermId> for ConformanceClaims {
    fn claim(
        &self,
        _: &purrdf_core::QuadIds<purrdf_core::TermId>,
    ) -> Result<Option<&purrdf_core::evidence::EvidenceRecord>, purrdf_sparql_eval::EvalError> {
        Ok(None)
    }

    fn scope(
        &self,
        _: purrdf_core::GraphMatch<purrdf_core::TermId>,
    ) -> Result<purrdf_sparql_eval::evidence_exec::EvidenceScope, purrdf_sparql_eval::EvalError>
    {
        Ok(purrdf_sparql_eval::evidence_exec::EvidenceScope {
            completeness: purrdf_core::evidence::SearchCompleteness::Complete,
            closure_declared: true,
            missing_reason: None,
        })
    }

    fn reason(
        &self,
        _: purrdf_core::evidence::ReasonId,
    ) -> Result<Option<purrdf_core::evidence::ReasonDescription<'_>>, purrdf_sparql_eval::EvalError>
    {
        Ok(None)
    }
}

type NativeStatus = (
    purrdf_core::evidence::Information,
    purrdf_core::evidence::SearchCompleteness,
    u64,
    u64,
    Vec<purrdf_core::evidence::ReasonCount>,
    Option<purrdf_core::evidence::WitnessVerdict>,
);

fn native_status(status: purrdf_core::evidence::AnswerStatus<'_>) -> NativeStatus {
    (
        status.information,
        status.completeness,
        status.established,
        status.undecided,
        status.reasons.to_vec(),
        status.verdict,
    )
}

#[derive(Debug, PartialEq)]
struct CursorRows {
    cells: Vec<Vec<Option<purrdf_core::TermValue>>>,
    undecided: Vec<bool>,
}

fn read_cursor_rows<R>(
    cursor: &mut purrdf_sparql_eval::engine::PreparedSelectCursor<
        '_,
        '_,
        purrdf_core::RdfDataset,
        R,
    >,
    variables: &[String],
    iri: &str,
) -> Result<CursorRows, Failed> {
    let header_matches = |schema: &purrdf_sparql_eval::solution::VarSchema| {
        schema
            .vars()
            .iter()
            .map(purrdf_sparql_algebra::ast::Variable::as_str)
            .eq(variables.iter().map(String::as_str))
    };
    if !header_matches(cursor.schema()) {
        return Err(Failed::from(format!(
            "{iri} cursor header column order differs from completed evaluation"
        )));
    }
    let mut actual = CursorRows {
        cells: Vec::new(),
        undecided: Vec::new(),
    };
    while let Some(row) = cursor
        .next_row()
        .map_err(|error| Failed::from(format!("{iri} cursor read: {error}")))?
    {
        if !header_matches(row.schema()) {
            return Err(Failed::from(format!(
                "{iri} cursor column order differs from completed evaluation"
            )));
        }
        actual.cells.push(
            row.cells()
                .iter()
                .map(|cell| {
                    cell.map(|term| {
                        row.context()
                            .scratch
                            .try_value_of(row.context().dataset, term)
                    })
                    .transpose()
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| Failed::from(format!("{iri} cursor cell: {error}")))?,
        );
        actual.undecided.push(row.evidence().is_undecided());
    }
    Ok(actual)
}

#[derive(Default)]
struct CapturedCoordinates(std::sync::Mutex<Option<purrdf_sparql_eval::QueryCoordinates>>);

impl purrdf_sparql_eval::ExecutionCoordinateObserver for CapturedCoordinates {
    fn observe(
        &self,
        coordinates: &purrdf_sparql_eval::QueryCoordinates,
    ) -> Result<(), purrdf_sparql_eval::EvalError> {
        let mut saved = self.0.lock().map_err(|_| {
            purrdf_sparql_eval::EvalError::config("conformance coordinate capture was poisoned")
        })?;
        if saved.is_none() {
            *saved = Some(coordinates.clone());
        }
        drop(saved);
        Ok(())
    }
}

fn run_manifest_cursor_case(manifest: &Path, label: &str) -> Result<(), Failed> {
    use purrdf_core::{SparqlRequest, SparqlResult};
    use purrdf_sparql_conformance::{manifest, run, service};
    let cases = manifest::load(manifest).map_err(Failed::from)?;
    let mut verified = 0;
    let mut unavailable = 0;
    let mut native_verified = 0;
    let mut native_unavailable = 0;
    for case in cases
        .iter()
        .filter(|case| case.kind == manifest::TestKind::QueryEval)
    {
        let text = std::fs::read_to_string(&case.query)
            .map_err(|error| Failed::from(format!("read {}: {error}", case.query.display())))?;
        let dataset = run::query_eval_dataset(case, &text).map_err(Failed::from)?;
        let engine = run::query_eval_engine();
        let env = run::query_eval_env(case).map_err(Failed::from)?;
        let remote = service::build(case).map_err(Failed::from)?;
        let coordinates = CapturedCoordinates::default();
        let options = run::query_eval_options(case, &env)
            .with_remote(Some(&remote))
            .with_coordinate_observer(&coordinates);
        let original = engine.query_with_options_view(
            dataset.as_ref(),
            SparqlRequest {
                query: &text,
                base_iri: Some(&case.base),
                substitutions: &[],
            },
            &options,
        );
        let result = match original {
            Ok(result) => result,
            Err(error) => {
                unavailable += 1;
                eprintln!(
                    "[{label}] completed evaluation unavailable for {}: {error}",
                    case.iri
                );
                continue;
            }
        };
        let SparqlResult::Solutions {
            variables,
            rows: expected,
            ..
        } = result
        else {
            continue;
        };
        let coordinates = coordinates
            .0
            .lock()
            .map_err(|_| Failed::from("conformance coordinate capture was poisoned"))?
            .clone()
            .ok_or_else(|| Failed::from(format!("{} completed without coordinates", case.iri)))?;
        let purrdf_sparql_eval::QueryCoordinates::Started(coordinates) = coordinates else {
            return Err(Failed::from(format!(
                "{} completed without a query context",
                case.iri
            )));
        };
        let replay = options
            .with_now(coordinates.effective_now())
            .with_entropy_seed(coordinates.effective_entropy_seed());
        let prepared = engine
            .prepare_query_with_options(&text, Some(&case.base), &replay)
            .map_err(|error| Failed::from(format!("{} cursor preparation: {error}", case.iri)))?;
        let mut cursor = engine
            .open_select_cursor(dataset.as_ref(), &prepared, &[], &replay)
            .map_err(|error| Failed::from(format!("{} cursor open: {error}", case.iri)))?;
        let actual = read_cursor_rows(&mut cursor, &variables, &case.iri)?.cells;
        if actual != expected {
            let difference = actual
                .iter()
                .zip(&expected)
                .position(|(actual, expected)| actual != expected);
            return Err(Failed::from(format!(
                "{} cursor rows differ: first differing position {difference:?}, cursor {} rows, completed {} rows; first cursor row {:?}; first completed row {:?}",
                case.iri,
                actual.len(),
                expected.len(),
                actual.first(),
                expected.first()
            )));
        }
        let settled = cursor
            .settle()
            .map_err(|error| Failed::from(format!("{} cursor settlement: {error}", case.iri)))?;
        if !matches!(
            settled.completion,
            purrdf_sparql_eval::eval::SelectCompletion::Complete
        ) || settled.delivered != expected.len()
        {
            return Err(Failed::from(format!(
                "{} cursor settlement differs from completed evaluation: {settled:?}",
                case.iri
            )));
        }
        verified += 1;

        let source = ConformanceClaims;
        let answer = purrdf_core::evidence::AnswerRequest {
            tier: purrdf_core::evidence::AnswerTier::Summary,
            question: None,
            standpoint: purrdf_core::evidence::StandpointKey::Unscoped,
            input_completeness: purrdf_core::evidence::SearchCompleteness::Complete,
            closure_declared: true,
        };
        let original = engine.query_evidence_status_view(
            dataset.as_ref(),
            purrdf_sparql_eval::InternedRequest {
                query: &text,
                base_iri: Some(&case.base),
                substitutions: &[],
            },
            &replay,
            &source,
            answer,
            |outcome, status| {
                let purrdf_sparql_eval::InternedOutcome::Solutions(rows) = outcome else {
                    return Err(Failed::from(
                        "an evident SELECT returned another query form",
                    ));
                };
                let expected = CursorRows {
                    cells: rows
                        .rows()
                        .iter()
                        .map(|row| {
                            row.iter()
                                .map(|cell| cell.map(|term| rows.value_of(term)))
                                .collect()
                        })
                        .collect(),
                    undecided: rows
                        .rows()
                        .iter()
                        .map(|row| row.evidence().is_undecided())
                        .collect(),
                };
                Ok((
                    rows.variables()
                        .iter()
                        .map(|variable| variable.as_str().to_owned())
                        .collect::<Vec<_>>(),
                    expected,
                    native_status(status),
                ))
            },
        );
        let (names, expected, status) = match original {
            Ok(result) => result?,
            Err(error) => {
                native_unavailable += 1;
                eprintln!(
                    "[{label}] completed native evidence unavailable for {}: {error}",
                    case.iri
                );
                continue;
            }
        };
        let mut cursor = engine
            .open_select_cursor_evidence(dataset.as_ref(), &prepared, &[], &replay, &source, answer)
            .map_err(|error| Failed::from(format!("{} evident cursor open: {error}", case.iri)))?;
        let actual = read_cursor_rows(&mut cursor, &names, &case.iri)?;
        if actual != expected {
            return Err(Failed::from(format!(
                "{} evident cursor rows or partition differ from completed evaluation",
                case.iri
            )));
        }
        let settled = cursor.settle().map_err(|error| {
            Failed::from(format!("{} evident cursor settlement: {error}", case.iri))
        })?;
        let purrdf_sparql_eval::eval::CursorAnswerStatus::Settled(actual) =
            cursor.answer_status().map_err(|error| {
                Failed::from(format!("{} evident cursor status: {error}", case.iri))
            })?
        else {
            return Err(Failed::from(format!(
                "{} evident cursor lost its completed native status",
                case.iri
            )));
        };
        if native_status(actual) != status
            || !matches!(
                settled.completion,
                purrdf_sparql_eval::eval::SelectCompletion::Complete
            )
            || settled.delivered != expected.cells.len()
            || u64::try_from(settled.established).ok() != Some(status.2)
            || u64::try_from(settled.undecided).ok() != Some(status.3)
        {
            return Err(Failed::from(format!(
                "{} evident cursor settled status or partition differs from completed evaluation",
                case.iri
            )));
        }
        native_verified += 1;
    }
    eprintln!(
        "[{label}] cursor parity: {verified} completed SELECT cases verified; {unavailable} original evaluation failures reported; {native_verified} native status and partition cases verified; {native_unavailable} original native evidence failures reported"
    );
    Ok(())
}

/// Run one manifest; `label` names it in the tally line and failure message.
fn run_manifest_case(
    manifest: &Path,
    label: &str,
    control: &mut purrdf_core::CanonicalizationControl<'_>,
) -> Result<(), Failed> {
    let summary = purrdf_sparql_conformance::run_manifest(manifest, control)
        .map_err(|e| Failed::from(format!("{label}: {e}")))?;
    eprintln!("[{label}] {}", summary.tally_line());
    if summary.is_ok() {
        Ok(())
    } else {
        Err(Failed::from(format!(
            "{label} failed:\n{}",
            summary.failure_report()
        )))
    }
}

fn main() -> ExitCode {
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("suite");
    let discovery = match purrdf_sparql_conformance::discover(&root) {
        Ok(discovery) => discovery,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(ERROR_EXIT_CODE);
        }
    };
    if discovery.groups.is_empty() {
        eprintln!(
            "error: no {SUITE_MANIFEST_NAME} group was found under {}; an \
             empty run would report success without exercising anything",
            root.display()
        );
        return ExitCode::from(ERROR_EXIT_CODE);
    }
    let groups = discovery.groups.into_iter().flat_map(|manifest| {
        let label = format!("suite/{}", manifest.relative);
        let cursor_label = label.clone();
        let cursor_path = manifest.path.clone();
        let cursor_name = format!("{CURSOR_PREFIX}{}", manifest.relative);
        [
            Trial::test(format!("{CASE_PREFIX}{}", manifest.relative), move || {
                let mut control = purrdf_core::CanonicalizationControl::new(
                    purrdf_core::RDFC_V2_CORPUS_POLICY,
                    None,
                );
                run_manifest_case(&manifest.path, &label, &mut control)
            }),
            Trial::test(cursor_name, move || {
                run_manifest_cursor_case(&cursor_path, &cursor_label)
            }),
        ]
    });
    // An index's coverage is checked by `discover` itself; its case reports the
    // membership so the run log says where those cases went.
    let indexes = discovery.indexes.into_iter().map(|(manifest, members)| {
        let label = format!("suite/{}", manifest.relative);
        Trial::test(format!("{INDEX_PREFIX}{}", manifest.relative), move || {
            eprintln!(
                "[{label}] index of {} manifests, each run as its own case",
                members.len()
            );
            Ok(())
        })
    });
    harness::main(groups.chain(indexes))
}
