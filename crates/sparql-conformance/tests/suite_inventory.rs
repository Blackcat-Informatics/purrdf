// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! On-disk inventory tripwires for `suite/`.
//!
//! Every manifest under `suite/` is one datatest case in `sparql_conformance.rs`, so a
//! directory that vanishes — a re-sync that dropped a vendored group, a first-party
//! suite deleted in a refactor — simply stops appearing in the run. Nothing fails, and
//! the tally still reads GREEN for never having exercised the surface. These tests are
//! the "no silent skips" doctrine applied to corpus completeness (mirroring
//! `rdfc_w3c::w3c_inventory`): each asserts that the manifests the suite is supposed to
//! carry are on disk.
//!
//! # Why they live in their own target
//!
//! They cannot live beside the cases they guard. `sparql_conformance.rs` is a
//! `harness = false` target whose `datatest_stable::harness!` expands to the `fn main`
//! that runs the discovered manifests, and nothing else: a `#[test]` function in that
//! file is compiled and never called, so an inventory assertion written there would be
//! a tripwire that cannot trip. This target uses the ordinary libtest harness, so these
//! run.

/// Inventory tripwire: the full set of vendored W3C sparql11 groups must stay present.
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
        "entailment",
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
        // full verbatim syntax groups (commit 426c7df)
        "syntax-query",
        "syntax-update-1",
        "syntax-update-2",
        "syntax-fed",
    ];
    assert_manifests_present(
        "w3c-sparql11",
        EXPECTED_GROUPS,
        "vendored W3C sparql11 group",
    );
}

/// Inventory tripwire for the vendored SPARQL 1.2 (RDF 1.2) tree.
#[test]
fn w3c_sparql12_inventory() {
    const EXPECTED_GROUPS: &[&str] = &[
        "grouping",
        "codepoint-escapes",
        "syntax-triple-terms-negative",
        "syntax-triple-terms-positive",
        "eval-triple-terms",
        "expression",
        "version",
        "lang-basedir",
        "rdf11",
        "syntax",
    ];
    assert_manifests_present(
        "w3c-sparql12",
        EXPECTED_GROUPS,
        "vendored W3C sparql12 group",
    );
}

/// Inventory tripwire for the FIRST-PARTY suites, where the risk is sharpest.
///
/// A vendored group at least has an upstream to be re-synced against. A first-party
/// suite has nothing outside this repository that would notice its absence, and each
/// one below is the only oracle for a surface no W3C manifest reaches.
#[test]
fn first_party_suite_inventory() {
    /// `(directory, what its loss would stop measuring)`.
    const EXPECTED_SUITES: &[(&str, &str)] = &[
        (
            "purrdf-smoke",
            "the baseline query / ASK / aggregate / federated-SERVICE smoke cases",
        ),
        (
            "purrdf-extend",
            "the extension-function, standpoint, RDF 1.2 reifier and loss-aware CONSTRUCT cases",
        ),
        ("purrdf-list-functions", "the rdf:List function cases"),
        ("purrdf-update", "the first-party UPDATE evaluation cases"),
        (
            "purrdf-property-functions",
            "the property-function relation seam: the access-pattern lattice, the \
             multi-output and empty-argument-vector shapes, mode restriction and the \
             feasibility reorder, and the seam's two hard errors",
        ),
    ];
    let root = suite_root();
    for (suite, covers) in EXPECTED_SUITES {
        let manifest = root.join(suite).join("manifest.ttl");
        assert!(
            manifest.is_file(),
            "first-party suite '{suite}' lost its manifest ({}), so nothing now measures {covers}",
            manifest.display()
        );
    }
    // The relation tables the harness registers for every case are data, in a fixture
    // beside the manifest whose cases call them (`run::harness_relations`). Losing the
    // file is a loud panic rather than a silent pass, but it is named here too so the
    // inventory of what the relation suite needs is in one place.
    let tables = root.join("purrdf-property-functions").join("relations.ttl");
    assert!(
        tables.is_file(),
        "the harness relation tables ({}) are gone; every property-function case would \
         then be running against relations that do not exist",
        tables.display()
    );
}

/// The `suite/` directory of this crate.
fn suite_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("suite")
}

/// Assert every `<tree>/<group>/manifest.ttl` is on disk.
fn assert_manifests_present(tree: &str, groups: &[&str], label: &str) {
    let root = suite_root().join(tree);
    for group in groups {
        let manifest = root.join(group).join("manifest.ttl");
        assert!(
            manifest.is_file(),
            "{label} '{group}' lost its manifest: {}",
            manifest.display()
        );
    }
}
