// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Differential SPARQL Results writer/reader harness.
//!
//! `purrdf-sparql-conformance`'s manifest-driven suite ([`sparql_conformance`])
//! reads every vendored `.srj`/`.srx` fixture with [`purrdf_sparql_results`]'s
//! readers and compares the DECODED model against the engine's own result — a
//! model-level comparison that can never see a writer bug, because the writer is
//! never exercised on that path at all.
//!
//! This harness closes that gap directly: for every vendored `.srj`/`.srx` file
//! under `suite/`, it
//!
//! 1. reads the fixture with the crate's own reader (`from_json`/`from_json_boolean`
//!    or `from_xml`/`from_xml_boolean`, whichever the file's category needs),
//! 2. re-serializes the decoded result with the crate's own writer
//!    (`to_json`/`to_xml`, no provenance — the fixtures carry none),
//! 3. reads the freshly written bytes back with the same reader, and
//! 4. asserts the twice-decoded result is model-equal to the once-decoded one,
//!    via [`purrdf_sparql_conformance::compare::compare_results`] — the SAME
//!    comparison machinery the manifest-driven suite already trusts, so this
//!    harness introduces no second oracle.
//!
//! A structural mismatch here — a key spelled differently, a namespace declared
//! in the wrong place, an attribute order the reader is accidentally sensitive
//! to — proves the writer diverges from the reader's own expectations, something
//! the model-level manifest comparison structurally cannot observe (both sides
//! of that comparison are already-decoded models; the writer never runs).
//!
//! # Coverage statement (skip-nothing)
//!
//! Every `.srj`/`.srx` file under `suite/` is visited. Each is categorized by
//! what it actually decodes as:
//!
//! * **SELECT** (`from_json`/`from_xml` succeeds) — round-tripped and compared
//!   as an ORDERED sequence (`compare_results(..., ordered = true)`), which is
//!   strictly stronger than the manifest suite's usual multiset comparison: a
//!   writer/reader pair that silently reordered rows would still be caught here.
//! * **ASK** (`from_json_boolean`/`from_xml_boolean` succeeds after the SELECT
//!   parse is refused) — round-tripped and compared as a boolean.
//!
//! No third category exists in `suite/`'s `.srj`/`.srx` corpus: CONSTRUCT/DESCRIBE
//! results are vendored as N-Triples/N-Quads/RDF-XML/Turtle files, never as SRJ/SRX
//! (SPARQL Results XML/JSON have no CONSTRUCT representation at all — SRX
//! hard-fails on a `Graph` result in the writer itself). A file that decodes as
//! NEITHER SELECT nor ASK is a hard test failure (reported by name), not a silent
//! skip — this harness's whole point is to leave nothing uncovered.
//!
//! The final assertion prints the category tally so the coverage is visible in
//! every green run, not just inferred from the absence of a failure.

use std::path::{Path, PathBuf};

use purrdf_core::{RdfDatasetBuilder, SparqlResult};
use purrdf_sparql_conformance::compare::compare_results;
use purrdf_sparql_results::{
    ResultProvenance, from_json, from_json_boolean, from_xml, from_xml_boolean, to_json, to_xml,
};

/// The `suite/` directory of this crate.
fn suite_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("suite")
}

/// Every `.srj`/`.srx` file under `root`, in path order. Sidecar `*.srj.license`/
/// `*.srx.license` files carry the `license` extension, not `srj`/`srx`, so they
/// are excluded by construction (no special-casing needed).
fn results_fixtures(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path
                .extension()
                .is_some_and(|ext| ext == "srj" || ext == "srx")
            {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out.sort();
    out
}

/// A tally of what the harness actually exercised, printed at the end so
/// coverage is visible in every green run.
#[derive(Debug, Default)]
struct Tally {
    select_json: usize,
    select_xml: usize,
    ask_json: usize,
    ask_xml: usize,
}

impl Tally {
    fn total(&self) -> usize {
        self.select_json + self.select_xml + self.ask_json + self.ask_xml
    }

    fn summary(&self) -> String {
        format!(
            "{} fixtures round-tripped (SELECT-JSON {}, SELECT-XML {}, ASK-JSON {}, ASK-XML {})",
            self.total(),
            self.select_json,
            self.select_xml,
            self.ask_json,
            self.ask_xml
        )
    }
}

/// Wrap a decoded `SELECT` solution set as the model-level [`SparqlResult`]
/// [`compare_results`] compares. The `aux` dataset is always empty here: SRJ/SRX
/// carry no auxiliary graph, and `compare_results`'s `Solutions` arm never
/// inspects it.
fn as_solutions(parsed: purrdf_sparql_results::ParsedSolutions) -> SparqlResult {
    SparqlResult::Solutions {
        variables: parsed.variables,
        rows: parsed.rows,
        aux: RdfDatasetBuilder::new()
            .freeze()
            .expect("an empty dataset always freezes"),
    }
}

/// Round-trip one `.srj` fixture: decode → re-encode → re-decode → compare.
///
/// # Errors
///
/// Returns a message describing the mismatch, or the reason the fixture could
/// not be categorized as SELECT or ASK JSON at all.
fn roundtrip_srj(path: &Path, tally: &mut Tally) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    match from_json(&bytes) {
        Ok(first) => {
            tally.select_json += 1;
            let first_result = as_solutions(first);
            let written = to_json(&first_result, &ResultProvenance::default(), None)
                .map_err(|e| format!("re-serialize as JSON: {e}"))?;
            let second = from_json(&written.bytes)
                .map_err(|e| format!("re-parse our own JSON output: {e}"))?;
            let second_result = as_solutions(second);
            compare_results(&first_result, &second_result, true)
                .map_err(|e| format!("SELECT round-trip mismatch: {e}"))
        }
        Err(select_err) => match from_json_boolean(&bytes) {
            Ok(value) => {
                tally.ask_json += 1;
                let first_result = SparqlResult::Boolean(value);
                let written = to_json(&first_result, &ResultProvenance::default(), None)
                    .map_err(|e| format!("re-serialize as JSON: {e}"))?;
                let second_value = from_json_boolean(&written.bytes)
                    .map_err(|e| format!("re-parse our own JSON output: {e}"))?;
                compare_results(&first_result, &SparqlResult::Boolean(second_value), false)
                    .map_err(|e| format!("ASK round-trip mismatch: {e}"))
            }
            Err(boolean_err) => Err(format!(
                "fixture decodes as NEITHER SELECT nor ASK JSON — coverage gap, not a skip \
                 (SELECT error: {select_err}; ASK error: {boolean_err})"
            )),
        },
    }
}

/// Round-trip one `.srx` fixture: decode → re-encode → re-decode → compare.
///
/// # Errors
///
/// Returns a message describing the mismatch, or the reason the fixture could
/// not be categorized as SELECT or ASK XML at all.
fn roundtrip_srx(path: &Path, tally: &mut Tally) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    match from_xml(&bytes) {
        Ok(first) => {
            tally.select_xml += 1;
            let first_result = as_solutions(first);
            let written = to_xml(&first_result, &ResultProvenance::default(), None)
                .map_err(|e| format!("re-serialize as XML: {e}"))?;
            let second = from_xml(&written.bytes)
                .map_err(|e| format!("re-parse our own XML output: {e}"))?;
            let second_result = as_solutions(second);
            compare_results(&first_result, &second_result, true)
                .map_err(|e| format!("SELECT round-trip mismatch: {e}"))
        }
        Err(select_err) => match from_xml_boolean(&bytes) {
            Ok(value) => {
                tally.ask_xml += 1;
                let first_result = SparqlResult::Boolean(value);
                let written = to_xml(&first_result, &ResultProvenance::default(), None)
                    .map_err(|e| format!("re-serialize as XML: {e}"))?;
                let second_value = from_xml_boolean(&written.bytes)
                    .map_err(|e| format!("re-parse our own XML output: {e}"))?;
                compare_results(&first_result, &SparqlResult::Boolean(second_value), false)
                    .map_err(|e| format!("ASK round-trip mismatch: {e}"))
            }
            Err(boolean_err) => Err(format!(
                "fixture decodes as NEITHER SELECT nor ASK XML — coverage gap, not a skip \
                 (SELECT error: {select_err}; ASK error: {boolean_err})"
            )),
        },
    }
}

/// Every vendored `.srj`/`.srx` fixture round-trips through our own
/// writer/reader pair with no model-level change. See the module docs for the
/// exact procedure and the coverage statement.
#[test]
fn every_vendored_results_fixture_round_trips() {
    let root = suite_root();
    let fixtures = results_fixtures(&root);
    assert!(
        !fixtures.is_empty(),
        "no .srj/.srx fixtures found under {} — the harness would pass vacuously",
        root.display()
    );

    let mut tally = Tally::default();
    let mut failures: Vec<(PathBuf, String)> = Vec::new();
    for path in &fixtures {
        let outcome = match path.extension().and_then(|e| e.to_str()) {
            Some("srj") => roundtrip_srj(path, &mut tally),
            Some("srx") => roundtrip_srx(path, &mut tally),
            other => unreachable!("results_fixtures only collects srj/srx, got {other:?}"),
        };
        if let Err(msg) = outcome {
            failures.push((path.clone(), msg));
        }
    }

    eprintln!("[results_roundtrip] {}", tally.summary());

    assert!(
        failures.is_empty(),
        "{} of {} vendored SPARQL Results fixtures failed to round-trip:\n{}",
        failures.len(),
        fixtures.len(),
        failures
            .iter()
            .map(|(path, msg)| format!("  • {}: {msg}", path.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );

    // Coverage sanity: every visited fixture landed in exactly one category
    // (no file is silently dropped between the walk and the tally).
    assert_eq!(
        tally.total(),
        fixtures.len(),
        "tally does not account for every visited fixture — a category is being dropped"
    );
}
