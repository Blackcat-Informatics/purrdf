// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `mf:include` aggregation and per-manifest case-IRI identity.
//!
//! Two defects in `crate::manifest` are pinned here, both of the same family — a
//! manifest that reports success while measuring the wrong thing:
//!
//! * an aggregator manifest (zero `mf:entries`, one `mf:include` collection) used
//!   to load ZERO cases and return `Ok`, so a corpus wired through one would have
//!   scored GREEN having run nothing;
//! * every manifest was parsed against ONE constant sentinel base, so two group
//!   manifests that each declare the relative `@prefix : <manifest#>` minted
//!   byte-identical case IRIs for every local name they share — and the global,
//!   IRI-matched expected-failure ledger could not tell the two apart.
//!
//! The fixtures live under `tests/fixtures/include-manifests/`, never under
//! `suite/`, so the datatest harness never discovers them as live cases.

use std::path::{Path, PathBuf};

use purrdf_sparql_conformance::manifest;

/// `tests/fixtures/include-manifests/` of this crate.
fn fixture(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("include-manifests")
        .join(relative)
}

/// Load `relative` and return the error it must produce.
fn refusal(relative: &str) -> String {
    let path = fixture(relative);
    assert!(path.is_file(), "fixture missing: {}", path.display());
    match manifest::load(&path) {
        Ok(cases) => panic!(
            "{} must be refused, but it loaded {} case(s)",
            path.display(),
            cases.len()
        ),
        Err(e) => e,
    }
}

/// The positive control for the whole feature: a pure aggregator loads the UNION
/// of its children's cases.
///
/// Before `mf:include` was implemented this returned `Ok(vec![])` — the silent
/// green. The count is asserted exactly (2 groups x 2 cases), so an include that
/// silently resolves to nothing cannot pass this by returning an empty set.
#[test]
fn aggregator_loads_the_union_of_its_included_manifests() {
    let cases = manifest::load(&fixture("aggregator/manifest-all.ttl"))
        .expect("the aggregator fixture must load");

    let mut names: Vec<&str> = cases.iter().map(|c| c.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        ["alpha-only-01", "beta-only-01", "shared-01", "shared-01"],
        "the aggregator must contribute every case of every included manifest, \
         including the local name both groups share"
    );
}

/// The defect-2 regression: two included manifests that declare the SAME relative
/// prefix and the SAME local name must still mint DIFFERENT case IRIs.
///
/// Under the old single constant base both resolved to
/// `http://purrdf.test/manifest/manifest#shared-01`, and one entry in the global
/// `xfail` ledger — which matches on the case IRI — would then have governed two
/// different tests in two different groups, able to mark a passing test xfail or
/// mask a real failure with nothing saying so.
#[test]
fn included_manifests_sharing_a_relative_prefix_mint_distinct_case_iris() {
    let cases = manifest::load(&fixture("aggregator/manifest-all.ttl"))
        .expect("the aggregator fixture must load");

    let shared: Vec<&str> = cases
        .iter()
        .filter(|c| c.name == "shared-01")
        .map(|c| c.iri.as_str())
        .collect();
    assert_eq!(
        shared.len(),
        2,
        "both groups declare a case named shared-01; both must survive the merge"
    );
    assert_ne!(
        shared[0], shared[1],
        "two manifests declaring the relative @prefix : <manifest#> must not mint \
         one case IRI: the global xfail ledger matches on the IRI and could not \
         tell the two tests apart"
    );

    // Identity is derived from each manifest's workspace-relative directory, so
    // the IRIs are byte-identical in every checkout rather than carrying an
    // absolute path.
    for iri in &shared {
        assert!(
            iri.starts_with(
                "http://purrdf.test/manifest/crates/sparql-conformance/tests/fixtures/\
                 include-manifests/aggregator/"
            ),
            "case IRI must be anchored at the manifest's workspace-relative path, got {iri}"
        );
    }
}

/// A manifest that includes ITSELF must hard-fail, naming the cycle.
#[test]
fn a_self_including_manifest_is_refused_and_names_the_cycle() {
    let error = refusal("self-cycle/manifest-all.ttl");
    assert!(
        error.contains("mf:include cycle"),
        "the refusal must say what it caught, got: {error}"
    );
    assert!(
        error.contains("self-cycle/manifest-all.ttl -> "),
        "the refusal must name the cycle chain, got: {error}"
    );
}

/// A TRANSITIVE cycle (A includes B, B includes A) must hard-fail the same way,
/// with both manifests in the named chain.
#[test]
fn a_mutually_including_pair_is_refused_and_names_the_whole_chain() {
    let error = refusal("mutual-cycle/manifest-all.ttl");
    assert!(
        error.contains("mf:include cycle"),
        "the refusal must say what it caught, got: {error}"
    );
    assert!(
        error.contains("manifest-inner.ttl"),
        "the chain must name the intermediate manifest, got: {error}"
    );
    assert_eq!(
        error.matches("manifest-all.ttl").count(),
        2,
        "the chain must show the cycle closing back on its start, got: {error}"
    );
}

/// A DIAMOND is not a cycle — the walk terminates — but loading the shared
/// manifest through both arms would count its cases twice, so it is refused too.
#[test]
fn a_manifest_reached_twice_by_two_parents_is_refused() {
    let error = refusal("diamond/manifest-all.ttl");
    assert!(
        error.contains("reached twice"),
        "the refusal must say what it caught, got: {error}"
    );
    assert!(
        error.contains("diamond/shared/manifest.ttl"),
        "the refusal must name the doubly-reached manifest, got: {error}"
    );
    assert!(
        !error.contains("mf:include cycle"),
        "a diamond terminates and must not be diagnosed as a cycle, got: {error}"
    );
}

/// A manifest declaring NEITHER `mf:entries` NOR `mf:include` measures nothing and
/// must be refused rather than counted green.
#[test]
fn a_manifest_declaring_neither_entries_nor_include_is_refused() {
    let error = refusal("declares-nothing/manifest.ttl");
    assert!(
        error.contains("NEITHER mf:entries NOR mf:include"),
        "the refusal must say what it caught, got: {error}"
    );
}

/// An empty `mf:entries ()` is `rdf:nil` and walks to nothing exactly as an absent
/// `mf:entries` does, so it gets its own detection and its own diagnosis — and it
/// must surface through an aggregator, naming the CHILD that is empty.
#[test]
fn an_aggregator_over_an_empty_group_is_refused_and_names_the_child() {
    let error = refusal("empty-closure/manifest-all.ttl");
    assert!(
        error.contains("EMPTY mf:entries list"),
        "the refusal must distinguish an empty list from an absent one, got: {error}"
    );
    assert!(
        error.contains("empty-group/manifest.ttl"),
        "the refusal must name the offending CHILD, not the aggregator, got: {error}"
    );
}

/// An aggregator may not be named `manifest.ttl`: the datatest root glob discovers
/// every `*/manifest.ttl`, so such a file would be run alongside the
/// `manifest.ttl` files it includes and every included case would run twice.
#[test]
fn an_aggregator_named_manifest_ttl_is_refused() {
    let error = refusal("aggregator-named-manifest/manifest.ttl");
    assert!(
        error.contains("may not declare mf:include"),
        "the refusal must say what it caught, got: {error}"
    );
    assert!(
        error.contains("twice"),
        "the refusal must state the double-count consequence, got: {error}"
    );
}

/// The real corpus this whole chunk exists for: `vectors/sparql-cdt/manifest-all.ttl`
/// is a pure aggregator over the six SEP-0009 group manifests.
///
/// It used to load ZERO cases and report SUCCESS — a manifest that declares 658
/// tests and runs none, scoring green. It must now load all 658, and every case
/// IRI in the closure must be distinct (`manifest::load` refuses a closure in
/// which two manifests mint one IRI, so reaching this assertion at all already
/// proves it; the count is asserted anyway so a future dedup could not quietly
/// absorb a collision).
///
/// This is corpus-loading only: no case is evaluated here.
#[test]
fn the_vendored_cdt_aggregator_loads_every_group() {
    let aggregator = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("vectors/sparql-cdt/manifest-all.ttl");
    let cases =
        manifest::load(&aggregator).expect("the vendored CDT aggregator manifest must load");

    assert_eq!(
        cases.len(),
        658,
        "vectors/sparql-cdt/manifest-all.ttl must aggregate all 658 declared cases \
         across its six groups (42 unfold + 30 fold + 287 list-functions + 196 \
         map-functions + 27 orderby + 76 bnodes); got {}",
        cases.len()
    );

    let distinct: std::collections::BTreeSet<&str> = cases.iter().map(|c| c.iri.as_str()).collect();
    assert_eq!(
        distinct.len(),
        cases.len(),
        "every case IRI in the closure must be globally distinct; list-functions and \
         map-functions each declare get-01..04, size-01..05, get-null-01, \
         get-error-01 and sameterm-01..04 under the same relative prefix"
    );
}
