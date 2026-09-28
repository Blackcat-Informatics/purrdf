// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Every W3C SPARQL 1.2 RL syntax, well-formedness and stratification entry, and every
//! evaluation entry's rule set, run through the BUILT `purrdf rules --srl FILE --check`
//! binary — the production check-only entry point, not the library harness's internals.
//!
//! Discovery is the library harness's own corpus reader
//! (`crates/shapes/tests/shacl_corpora`, included here by path); the per-type counts are
//! pinned before anything runs, so this sweep cannot pass by checking fewer entries.
//!
//! A positive entry must exit 0 at the level its type asks about — a positive syntax test
//! at `--check=syntax` ("regardless of well-formedness and stratification"), a positive
//! well-formedness test at `--check=well-formed`, a positive stratification test and every
//! evaluation entry's rule set at bare `--check` — writing the one-line summary for that
//! level to stdout. A negative entry must exit 1 under bare `--check`, refused by the
//! stage its type names, and write nothing to stdout.
//!
//! The level is observable, not decoration: fourteen positive syntax entries are not
//! stratifiable and one is not well formed, so bare `--check` refuses exactly those
//! fifteen — by a later stage, never the grammar — while `--check=syntax` accepts them.

#[path = "../../shapes/tests/shacl_corpora/mod.rs"]
mod shacl_corpora;

use std::process::{Command, Output};

use shacl_corpora::file_iri;
use shacl_corpora::shacl12::{Body, SrlCase, SrlKind, shacl12_cases};

/// The discovered entries of each kind, restated from the corpus so the sweep cannot
/// pass by running fewer.
const EXPECTED: [(SrlKind, usize); 7] = [
    (SrlKind::PositiveSyntax, 109),
    (SrlKind::NegativeSyntax, 30),
    (SrlKind::PositiveWellFormedness, 4),
    (SrlKind::NegativeWellFormedness, 4),
    (SrlKind::PositiveStratification, 5),
    (SrlKind::NegativeStratification, 5),
    (SrlKind::Eval, 46),
];

/// The positive syntax entries bare `--check` refuses: not stratifiable (fourteen) or not
/// well formed (one).
const SYNTAX_ONLY: usize = 15;

fn check(case: &SrlCase, level: Option<&str>) -> Result<Output, String> {
    let file = case.ruleset.to_str().ok_or("a non-UTF-8 corpus path")?;
    let base = file_iri(&case.ruleset);
    let flag = level.map_or_else(|| "--check".to_owned(), |level| format!("--check={level}"));
    Command::new(env!("CARGO_BIN_EXE_purrdf"))
        .args(["rules", "--srl", file, "--srl-base", &base, &flag])
        .output()
        .map_err(|e| format!("spawn purrdf: {e}"))
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// A positive entry: exit 0, and exactly the one summary line for `level`.
fn accepted(case: &SrlCase, level: Option<&str>, verdict: &str) -> Result<(), String> {
    let out = check(case, level)?;
    if out.status.code() != Some(0) {
        return Err(format!(
            "exit {:?}: {}",
            out.status.code(),
            text(&out.stderr)
        ));
    }
    let stdout = text(&out.stdout);
    let prefix = format!(
        "--srl {}: SPARQL 1.2 RL rule set {verdict}: ",
        case.ruleset.display()
    );
    if !stdout.starts_with(&prefix) || stdout.lines().count() != 1 {
        return Err(format!(
            "expected one line starting {prefix:?}, got {stdout:?}"
        ));
    }
    Ok(())
}

/// A negative entry: exit 1 under bare `--check`, the refusal naming `stage`, no stdout.
fn refused(case: &SrlCase, stage: &str) -> Result<(), String> {
    let out = check(case, None)?;
    let stderr = text(&out.stderr);
    if out.status.code() != Some(1) {
        return Err(format!(
            "expected exit 1, got {:?}: {stderr}",
            out.status.code()
        ));
    }
    if !stderr.contains(stage) {
        return Err(format!(
            "expected a refusal naming {stage:?}, got: {stderr}"
        ));
    }
    if !out.stdout.is_empty() {
        return Err(format!("a refusal wrote to stdout: {}", text(&out.stdout)));
    }
    Ok(())
}

const SYNTAX: &str = "SPARQL 1.2 RL syntax error";
const ILL_FORMED: &str = "is not well formed";
const UNSTRATIFIABLE: &str = "is not stratifiable";

#[test]
fn every_w3c_srl_entry_is_graded_through_rules_check() {
    let cases = shacl12_cases();
    let srl: Vec<(&str, &SrlCase)> = cases
        .iter()
        .filter_map(|case| match &case.body {
            Body::Srl(tc) => Some((case.id.as_str(), tc)),
            _ => None,
        })
        .collect();
    for (kind, expected) in EXPECTED {
        let found = srl.iter().filter(|(_, tc)| tc.kind == kind).count();
        assert_eq!(found, expected, "discovered {kind:?} entries");
    }

    let mut errors: Vec<String> = Vec::new();
    let mut syntax_only = 0usize;
    for (id, tc) in &srl {
        let graded = match tc.kind {
            SrlKind::PositiveSyntax => {
                accepted(tc, Some("syntax"), "is syntactically valid (level syntax)").and_then(
                    |()| {
                        // The full check: accepted, or refused by a LATER stage.
                        let out = check(tc, None)?;
                        match out.status.code() {
                            Some(0) => Ok(()),
                            Some(1) => {
                                let stderr = text(&out.stderr);
                                if stderr.contains(SYNTAX) {
                                    return Err(format!(
                                        "bare --check refused a positive syntax test by the \
                                         grammar: {stderr}"
                                    ));
                                }
                                if !stderr.contains(ILL_FORMED) && !stderr.contains(UNSTRATIFIABLE)
                                {
                                    return Err(format!("an unexpected refusal: {stderr}"));
                                }
                                syntax_only += 1;
                                Ok(())
                            }
                            other => Err(format!("bare --check exited {other:?}")),
                        }
                    },
                )
            }
            SrlKind::PositiveWellFormedness => accepted(
                tc,
                Some("well-formed"),
                "is well formed (level well-formed)",
            ),
            SrlKind::PositiveStratification | SrlKind::Eval => {
                accepted(tc, None, "is well formed and stratified (level stratified)")
            }
            SrlKind::NegativeSyntax => refused(tc, SYNTAX),
            SrlKind::NegativeWellFormedness => refused(tc, ILL_FORMED),
            SrlKind::NegativeStratification => refused(tc, UNSTRATIFIABLE),
        };
        if let Err(error) = graded {
            errors.push(format!("FAIL [{id}] ({:?}): {error}", tc.kind));
        }
    }
    assert!(
        errors.is_empty(),
        "{} entr(ies) failed through `rules --check`:\n{}",
        errors.len(),
        errors.join("\n\n")
    );
    assert_eq!(
        syntax_only, SYNTAX_ONLY,
        "the positive syntax entries bare --check refuses by a later stage"
    );
}
