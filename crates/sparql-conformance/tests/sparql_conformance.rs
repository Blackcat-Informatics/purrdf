// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One case per `manifest.ttl` under `suite/`, run by the `purrdf_testkit::harness`
//! runner. Each runs all of its manifest's cases (honoring the xfail registry) and
//! prints a tally; a non-xfail failure or a stale-xfail unexpected-pass fails the case.
//!
//! Cases are discovered by `purrdf_sparql_conformance::paths::suite_manifests`, named
//! `run_manifest_case::<path below suite/>`, and handed to the runner in that
//! function's byte order of the relative path, which is the order they list and run in.
//!
//! Nothing but the harness lives here. This target is `harness = false` and its `main`
//! runs the discovered manifests and nothing else, so a `#[test]` function written in
//! this file would be compiled and never called. The on-disk inventory tripwires that
//! guard against a suite directory disappearing are therefore in `suite_inventory.rs`,
//! which uses the ordinary libtest harness.

use std::path::Path;
use std::process::ExitCode;

use purrdf_sparql_conformance::paths::suite_manifests;
use purrdf_testkit::harness::{self, Failed, Trial};

/// The suite root, relative to the package directory cargo runs tests in.
const SUITE_ROOT: &str = "suite";

/// The prefix of every case name: `run_manifest_case::<path below suite/>`.
const CASE_PREFIX: &str = "run_manifest_case";

fn run_manifest_case(manifest: &Path) -> Result<(), Failed> {
    let summary = purrdf_sparql_conformance::run_manifest(manifest)?;
    eprintln!("[{}] {}", manifest.display(), summary.tally_line());
    if summary.is_ok() {
        Ok(())
    } else {
        Err(Failed::from(format!(
            "{} failed:\n{}",
            manifest.display(),
            summary.failure_report()
        )))
    }
}

fn main() -> ExitCode {
    let manifests = match suite_manifests(Path::new(SUITE_ROOT)) {
        Ok(manifests) if manifests.is_empty() => {
            eprintln!(
                "error: no {} found below {SUITE_ROOT}/; a run of zero manifests would \
                 report green for measuring nothing",
                purrdf_sparql_conformance::paths::SUITE_MANIFEST_NAME
            );
            return ExitCode::from(harness::ERROR_EXIT_CODE);
        }
        Ok(manifests) => manifests,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(harness::ERROR_EXIT_CODE);
        }
    };
    harness::main(manifests.into_iter().map(|manifest| {
        let path = manifest.path;
        Trial::test(format!("{CASE_PREFIX}::{}", manifest.relative), move || {
            run_manifest_case(&path)
        })
    }))
}
