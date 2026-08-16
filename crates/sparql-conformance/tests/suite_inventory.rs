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

/// Case-count and kind-breakdown tripwire for `purrdf-extend/manifest.ttl`.
///
/// `manifest::load` groups rows through a SPARQL `SELECT` whose `?type`/`?name`/
/// `?act` are all MANDATORY (not `OPTIONAL`), so an `mf:entries` member missing any
/// one of `rdf:type`/`mf:name`/`mf:action` produces no row and would otherwise
/// vanish from the loaded case set with no trace whatsoever — the manifest would
/// keep advertising the cases it declares while the harness quietly ran fewer of
/// them, and the scoreboard would stay green. `manifest::load` now asserts this
/// itself (a declared-vs-loaded count check that fails the load), but this test
/// pins the SPECIFIC count and kind breakdown this suite is supposed to carry, so
/// a change to the loader, the manifest, or a fixture that silently drops a case —
/// while still leaving `load` itself succeeding — still turns this test red rather
/// than just quietly reporting fewer cases through the datatest tally line.
///
/// This suite mixes `mf:action` shapes on purpose (a blank node carrying
/// `qt:query`/`qt:data` for most `mf:QueryEvaluationTest` cases, and a bare IRI
/// action for the `mf:PositiveSyntaxTest`/`mf:NegativeSyntaxTest` cases), which is
/// exactly the shape H13 flagged as a silent-skip risk if the loader ever started
/// requiring one shape unconditionally.
#[test]
fn purrdf_extend_case_count_and_kinds() {
    use purrdf_sparql_conformance::manifest::TestKind;

    let manifest = suite_root().join("purrdf-extend").join("manifest.ttl");
    let cases = purrdf_sparql_conformance::manifest::load(&manifest)
        .unwrap_or_else(|e| panic!("purrdf-extend/manifest.ttl failed to load: {e}"));

    assert_eq!(
        cases.len(),
        31,
        "purrdf-extend/manifest.ttl must load exactly 31 cases (its mf:entries list \
         count); got {} — a case silently stopped loading",
        cases.len()
    );

    let query_eval = cases
        .iter()
        .filter(|c| c.kind == TestKind::QueryEval)
        .count();
    let positive_syntax = cases
        .iter()
        .filter(|c| c.kind == TestKind::PositiveSyntax)
        .count();
    let negative_syntax = cases
        .iter()
        .filter(|c| c.kind == TestKind::NegativeSyntax)
        .count();
    let unmodeled = cases.iter().filter(|c| c.kind == TestKind::Unknown).count();

    assert_eq!(
        unmodeled, 0,
        "purrdf-extend must carry no unmodeled (TestKind::Unknown) cases"
    );
    assert_eq!(
        query_eval, 28,
        "purrdf-extend's mf:QueryEvaluationTest count drifted (blank-node \
         qt:query/qt:data actions) — expected 28, got {query_eval}"
    );
    assert_eq!(
        positive_syntax, 1,
        "purrdf-extend's mf:PositiveSyntaxTest count drifted (bare-IRI action) — \
         expected 1, got {positive_syntax}"
    );
    assert_eq!(
        negative_syntax, 2,
        "purrdf-extend's mf:NegativeSyntaxTest count drifted (bare-IRI action) — \
         expected 2, got {negative_syntax}"
    );
    assert_eq!(
        query_eval + positive_syntax + negative_syntax + unmodeled,
        cases.len(),
        "kind breakdown must account for every loaded case"
    );
}

/// The `suite/` directory of this crate.
fn suite_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("suite")
}

/// The `tests/fixtures/` directory of this crate — deliberately NOT under `suite/`,
/// so nothing here is ever discovered as a live conformance case by
/// `sparql_conformance.rs`'s `datatest_stable::harness!` (rooted at `suite/` only).
fn fixtures_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// F9: prove the manifest loader's silent-skip completeness guard actually FIRES,
/// rather than merely existing unexercised (it was previously verified only by
/// hand-breaking a manifest locally — never by a committed regression).
///
/// `tests/fixtures/broken-manifests/undescribed-entry/manifest.ttl` declares one
/// `mf:entries` member with no `rdf:type`/`mf:name`/`mf:action` triples at all — the
/// loader's row-grouping SELECT requires all three to bind, so that entry produces
/// no row and would otherwise vanish from the loaded case set with no trace. The
/// completeness check in `crate::manifest::load` (see its doc comment, around the
/// `list_entry_iris`/`missing` logic) must catch exactly this and name the offending
/// entry in its error, rather than returning a manifest that silently advertises
/// fewer cases than it declares.
#[test]
fn broken_manifest_with_an_undescribed_entry_is_rejected() {
    let manifest = fixtures_root()
        .join("broken-manifests")
        .join("undescribed-entry")
        .join("manifest.ttl");
    assert!(
        manifest.is_file(),
        "negative fixture missing: {}",
        manifest.display()
    );

    let error = purrdf_sparql_conformance::manifest::load(&manifest)
        .expect_err("a manifest with a declared-but-undescribed mf:entries member must be refused");

    assert!(
        error.contains("silent-skip"),
        "the guard's error must name what it caught, got: {error}"
    );
    assert!(
        error.contains("#undescribedEntry"),
        "the guard's error must name the OFFENDING entry, got: {error}"
    );
    assert!(
        error.contains("1 of 1"),
        "the guard's error must state the declared-vs-loaded count, got: {error}"
    );
}

/// Guards the PLACEMENT invariant `broken_manifest_with_an_undescribed_entry_is_rejected`
/// (and its positive control immediately below) both rely on: the negative fixture
/// directory lives under `tests/fixtures/`, never under `suite/`, so it is never
/// discovered as a real (and permanently failing) conformance case by
/// `sparql_conformance.rs`'s `datatest_stable::harness!`, which is rooted at
/// `suite/` only.
///
/// This is a directory-existence check, not the positive control the module doc
/// for the test above once claimed to be here — see
/// `broken_manifest_with_a_described_entry_loads_cleanly` for that.
#[test]
fn broken_manifest_fixture_directory_does_not_leak_into_the_live_suite() {
    let leaked = suite_root().join("broken-manifests");
    assert!(
        !leaked.exists(),
        "the negative fixture must live under tests/fixtures/, not suite/, or the \
         datatest harness would pick it up as a real (and permanently failing) case"
    );
}

/// The ACTUAL positive control for `broken_manifest_with_an_undescribed_entry_is_rejected`:
/// `tests/fixtures/broken-manifests/described-entry/manifest.ttl` is the exact sibling
/// shape of `undescribed-entry/manifest.ttl` — same `mf:Manifest`/single-entry
/// `mf:entries` list, same bare-IRI `mf:action` style — except its sole entry DOES
/// carry `rdf:type`/`mf:name`/`mf:action`, the three triples the loader's
/// row-grouping SELECT requires to bind a row at all.
///
/// If this manifest failed to load, or loaded with a different case count than 1,
/// the negative test's rejection could not be attributed specifically to the missing
/// description — it could equally be some other structural defect the two fixtures
/// happen to share (a malformed `mf:entries` list, an unresolvable base IRI, a Turtle
/// parse error). This test rules that out: the only difference between the two
/// fixtures is the presence of the description triples, and only the one missing
/// them is rejected.
#[test]
fn broken_manifest_with_a_described_entry_loads_cleanly() {
    let manifest = fixtures_root()
        .join("broken-manifests")
        .join("described-entry")
        .join("manifest.ttl");
    assert!(
        manifest.is_file(),
        "positive-control fixture missing: {}",
        manifest.display()
    );

    let cases = purrdf_sparql_conformance::manifest::load(&manifest).unwrap_or_else(|e| {
        panic!("a manifest whose sole entry IS fully described must load cleanly, got: {e}")
    });

    assert_eq!(
        cases.len(),
        1,
        "the manifest declares exactly one mf:entries member; it must load exactly \
         one case, got {}",
        cases.len()
    );
    assert_eq!(
        cases[0].iri, "http://purrdf.test/manifest/#describedEntry",
        "the loaded case must be the described entry, not some other IRI"
    );
    assert_eq!(
        cases[0].kind,
        purrdf_sparql_conformance::manifest::TestKind::PositiveSyntax,
        "the loaded case's kind must reflect its declared rdf:type"
    );
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
