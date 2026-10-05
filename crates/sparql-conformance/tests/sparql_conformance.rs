// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One case per discovered leaf manifest under `suite/`, named
//! `run_manifest_case::<path below suite/>`. Each runs all of its manifest's
//! cases (honoring the xfail registry) and prints a tally; a non-xfail failure
//! or a stale-xfail unexpected-pass fails the case.
//!
//! The target is `harness = false`: `main` discovers the manifests with
//! [`purrdf_sparql_conformance::paths::suite_manifests`] and hands them to the
//! libtest-compatible [`purrdf_testkit::harness`] runner, which implements the
//! libtest command line (filters, `--exact`, `--skip`, `--list`, `--nocapture`,
//! `--test-threads`, …). Nothing else runs here, so a `#[test]` function written
//! in this file would be compiled and never called. The on-disk inventory
//! tripwires that guard against a suite directory disappearing are therefore in
//! `suite_inventory.rs`, which uses the ordinary libtest harness.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use purrdf_sparql_conformance::paths::{self, SUITE_MANIFEST_NAME};
use purrdf_testkit::harness::{self, ERROR_EXIT_CODE, Failed, Trial};

/// The case-name prefix every discovered manifest is run under.
const CASE_PREFIX: &str = "run_manifest_case::";

/// Run one manifest; `label` names it in the tally line and failure message.
fn run_manifest_case(manifest: &Path, label: &str) -> Result<(), Failed> {
    let summary = purrdf_sparql_conformance::run_manifest(manifest)
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
    let manifests = match paths::suite_manifests(&root) {
        Ok(manifests) => manifests,
        Err(e) => {
            eprintln!("error: discovering manifests under {}: {e}", root.display());
            return ExitCode::from(ERROR_EXIT_CODE);
        }
    };
    if manifests.is_empty() {
        eprintln!(
            "error: no {SUITE_MANIFEST_NAME} or extended-manifest.ttl was found under {}; an empty run would report \
             success without exercising anything",
            root.display()
        );
        return ExitCode::from(ERROR_EXIT_CODE);
    }
    let trials = manifests.into_iter().map(|manifest| {
        let label = format!("suite/{}", manifest.relative);
        Trial::test(format!("{CASE_PREFIX}{}", manifest.relative), move || {
            run_manifest_case(&manifest.path, &label)
        })
    });
    harness::main(trials)
}
