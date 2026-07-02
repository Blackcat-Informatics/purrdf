// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Datatest-stable entry: one nextest case per `manifest.ttl` under `suite/`.
//! Each runs all of its manifest's cases (honoring the xfail registry) and prints
//! a tally; a non-xfail failure or a stale-xfail unexpected-pass fails the case.

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

/// Inventory tripwire: the full set of vendored W3C sparql11 groups must stay
/// present. Each group is one datatest case, so a group directory silently
/// dropped on a re-sync would simply vanish from the run with no failure — this
/// asserts every expected `manifest.ttl` is on disk (the "no silent skips"
/// doctrine applied to corpus completeness, mirroring `rdfc_w3c::w3c_inventory`).
#[test]
fn w3c_sparql11_inventory() {
    const EXPECTED_GROUPS: &[&str] = &[
        // curated subset
        "aggregates",
        "subquery",
        "service",
        // full verbatim query-eval groups (commit 426c7df)
        "bind",
        "bindings",
        "cast",
        "construct",
        "exists",
        "functions",
        "grouping",
        "negation",
        "project-expression",
        "property-path",
        // full verbatim update-eval groups (commit 426c7df)
        "add",
        "basic-update",
        "clear",
        "copy",
        "delete",
        "delete-data",
        "delete-insert",
        "delete-where",
        "drop",
        "move",
        "update-silent",
    ];
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("suite/w3c-sparql11");
    for group in EXPECTED_GROUPS {
        let manifest = root.join(group).join("manifest.ttl");
        assert!(
            manifest.is_file(),
            "vendored W3C sparql11 group '{group}' lost its manifest: {}",
            manifest.display()
        );
    }
}

datatest_stable::harness! {
    { test = run_manifest_case, root = "suite", pattern = r".*/manifest\.ttl$" },
}
