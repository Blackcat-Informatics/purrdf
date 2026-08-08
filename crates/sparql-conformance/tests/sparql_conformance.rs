// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Datatest-stable entry: one nextest case per `manifest.ttl` under `suite/`.
//! Each runs all of its manifest's cases (honoring the xfail registry) and prints
//! a tally; a non-xfail failure or a stale-xfail unexpected-pass fails the case.
//!
//! Nothing but the harness lives here. `datatest_stable::harness!` expands to this
//! target's whole `fn main`, which runs the discovered manifests and nothing else, so
//! a `#[test]` function written in this file would be compiled and never called. The
//! on-disk inventory tripwires that guard against a suite directory disappearing are
//! therefore in `suite_inventory.rs`, which uses the ordinary libtest harness.

use camino::Utf8Path;

fn run_manifest_case(manifest: &Utf8Path) -> datatest_stable::Result<()> {
    let summary = purrdf_sparql_conformance::run_manifest(manifest.as_std_path())?;
    eprintln!("[{manifest}] {}", summary.tally_line());
    if summary.is_ok() {
        Ok(())
    } else {
        Err(format!("{} failed:\n{}", manifest, summary.failure_report()).into())
    }
}

datatest_stable::harness! {
    { test = run_manifest_case, root = "suite", pattern = r".*/manifest\.ttl$" },
}
