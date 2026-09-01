// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integrity of the global expected-failure ledger.
//!
//! `purrdf_sparql_conformance::xfail::XFAIL` is matched against a case IRI, and
//! the run itself only ever asks the ledger about cases it actually reached. That
//! leaves two failure modes invisible to the run:
//!
//! * an entry matching NO case — dead weight that keeps the suite's budget in
//!   `scripts/conformance-baseline.json` propped up after the gap it named is
//!   gone, so the equality ratchet stops noticing the fix;
//! * an entry matching MORE THAN ONE case — one typed reason silently governing
//!   two different tests, which can mark a passing test xfail or mask a real
//!   failure.
//!
//! `xfail::lookup` refuses the second at run time, but only for a case it is
//! asked about. This target closes both over the WHOLE live suite by loading
//! every `suite/**/manifest.ttl` and matching the ledger against the complete set
//! of case IRIs. It uses the ordinary libtest harness, because
//! `tests/sparql_conformance.rs` is `harness = false` and its
//! `datatest_stable::harness!` expands to a `fn main` that would never call a
//! `#[test]` written beside it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use purrdf_sparql_conformance::xfail::{self, XFAIL};

/// Every case IRI the live `suite/` tree declares.
fn all_live_case_iris() -> BTreeSet<String> {
    let mut iris = BTreeSet::new();
    for manifest in live_manifests() {
        let cases = purrdf_sparql_conformance::manifest::load(&manifest)
            .unwrap_or_else(|e| panic!("{}: failed to load: {e}", manifest.display()));
        for case in cases {
            assert!(
                iris.insert(case.iri.clone()),
                "case IRI {} is declared by more than one manifest in suite/; the ledger \
                 matches on the IRI and could not tell the two tests apart",
                case.iri
            );
        }
    }
    assert!(
        !iris.is_empty(),
        "no case IRIs were collected from suite/ — the walk found no manifest, so every \
         assertion below would pass vacuously"
    );
    iris
}

/// Every `manifest.ttl` under `suite/`, i.e. exactly what the datatest root glob
/// `.*/manifest\.ttl$` discovers.
fn live_manifests() -> Vec<PathBuf> {
    let mut found = Vec::new();
    collect(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("suite"),
        &mut found,
    );
    found.sort();
    found
}

/// Recursively collect `manifest.ttl` files under `dir`.
fn collect(dir: &Path, found: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
            .path();
        if path.is_dir() {
            collect(&path, found);
        } else if path.file_name().and_then(|n| n.to_str()) == Some("manifest.ttl") {
            found.push(path);
        }
    }
}

/// Every ledger entry must govern EXACTLY ONE live case.
///
/// Zero is a dead entry; more than one is an ambiguous one. Both are bugs in the
/// ledger rather than in the engine, and neither can be seen from a run's tally —
/// a dead entry simply never fires, and an ambiguous one fires for whichever case
/// reaches it.
#[test]
fn every_xfail_entry_matches_exactly_one_live_case() {
    let iris = all_live_case_iris();
    for entry in XFAIL {
        let hits: Vec<&str> = iris
            .iter()
            .filter(|iri| xfail::matches(iri, entry.iri_tail))
            .map(String::as_str)
            .collect();
        assert_eq!(
            hits.len(),
            1,
            "xfail entry {:?} ({}) matches {} live case(s) [{}]. An entry matching none is \
             dead weight that keeps this suite's budget in \
             scripts/conformance-baseline.json propped up after the gap it named is gone; \
             an entry matching several silently governs more than one test",
            entry.iri_tail,
            entry.reason.label(),
            hits.len(),
            hits.join(", "),
        );
    }
}

/// No live case may be governed by two ledger entries.
///
/// `xfail::lookup` refuses this at run time, but only for a case it is asked
/// about — and it is asked only about cases that were reached. This checks the
/// whole declared set.
#[test]
fn no_live_case_is_matched_by_two_xfail_entries() {
    for iri in all_live_case_iris() {
        let hits: Vec<&str> = XFAIL
            .iter()
            .filter(|entry| xfail::matches(&iri, entry.iri_tail))
            .map(|entry| entry.iri_tail)
            .collect();
        assert!(
            hits.len() < 2,
            "case {iri} is matched by {} xfail entries ({}); one case must carry exactly \
             one typed reason",
            hits.len(),
            hits.join(", ")
        );
        // The same fact through the production path, so the two cannot drift.
        assert!(
            xfail::lookup(&iri).is_ok(),
            "xfail::lookup must not refuse {iri}"
        );
    }
}

/// Matching is ANCHORED at an IRI path-segment boundary, not a bare `ends_with`.
///
/// A bare suffix test lets a short tail capture the back half of a longer local
/// name in an unrelated group — the exact way a ledger entry silently acquires a
/// second, unintended test.
#[test]
fn entry_matching_is_anchored_at_a_path_segment_boundary() {
    assert!(xfail::matches(
        "http://example.org/tests/cast/manifest#cast-decimal",
        "cast/manifest#cast-decimal"
    ));
    assert!(
        xfail::matches("cast/manifest#cast-decimal", "cast/manifest#cast-decimal"),
        "a tail equal to the whole IRI matches"
    );
    assert!(
        !xfail::matches(
            "http://example.org/tests/upcast/manifest#cast-decimal",
            "cast/manifest#cast-decimal"
        ),
        "a tail must not capture the back half of a longer path segment ('upcast')"
    );
    assert!(
        !xfail::matches(
            "http://example.org/tests/cast/manifest#cast-decimal",
            "manifest#cast-decima"
        ),
        "a non-suffix does not match"
    );
}
