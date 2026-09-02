// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The vendored **SEP-0009 SPARQL Composite Datatypes** corpus
//! (`vectors/sparql-cdt/`, upstream `awslabs/SPARQL-CDTs`).
//!
//! This is the independent oracle for `cdt:List`/`cdt:Map`, `FOLD` and `UNFOLD`.
//! Until it ran, every claim about this engine's CDT behavior rested on tests
//! written beside the implementation by the same hand — which grade the engine
//! against itself. The corpus grades it against the people who wrote the spec.
//!
//! # Why this target, and not `suite/`
//!
//! `sparql_conformance.rs`'s `datatest_stable::harness!` is rooted at `suite/`
//! and folds every manifest it finds into ONE scoreboard row. The CDT corpus is
//! vendored under `vectors/` (frozen and digest-pinned by
//! `scripts/check-corpus-frozen.py`, exactly like the GTS vectors), so it gets
//! its own row for the same reason the CONSTRUCT and DESCRIBE corpora do: a
//! consumer asking "does this engine do SEP-0009?" must be able to read the
//! answer off the scoreboard rather than out of a four-digit total.
//!
//! The scoreboard line `SPARQL-CDT-CORPUS: passed N xfail X total M` is what
//! `scripts/conformance-matrix.py` scrapes.
//!
//! # One entry point, six groups
//!
//! `manifest-all.ttl` is an `mf:include` aggregator over the six group
//! manifests. Running it (rather than the six directly) is deliberate: an
//! upstream re-sync that adds a seventh group is picked up here with no edit,
//! and `manifest::load` refuses an include closure in which two manifests mint
//! the same case IRI, so the aggregator cannot silently collapse two groups.
//! `tests/suite_inventory.rs::sparql_cdt_inventory` separately pins the
//! per-group counts, so a re-sync that DROPS a group is caught there.

use std::path::PathBuf;

/// The vendored corpus's `mf:include` aggregator manifest.
fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("vectors")
        .join("sparql-cdt")
        .join("manifest-all.ttl")
}

/// Run all 658 vendored SEP-0009 cases, honoring the shared xfail ledger.
#[test]
fn sparql_cdt_corpus() {
    let manifest = manifest();
    assert!(
        manifest.is_file(),
        "vendored SEP-0009 corpus manifest missing: {}",
        manifest.display()
    );

    let total = purrdf_sparql_conformance::manifest::load(&manifest)
        .unwrap_or_else(|e| panic!("load the SEP-0009 aggregator manifest: {e}"))
        .len();
    let summary = purrdf_sparql_conformance::run_manifest(&manifest)
        .unwrap_or_else(|e| panic!("run the SEP-0009 corpus: {e}"));

    println!(
        "SPARQL-CDT-CORPUS: passed {} xfail {} total {total}",
        summary.passed, summary.xfail
    );

    assert!(
        summary.is_ok(),
        "SEP-0009 CDT corpus failed:\n{}",
        summary.failure_report()
    );
    assert_eq!(
        summary.passed + summary.xfail,
        total,
        "every declared SEP-0009 case must be accounted for as a pass or a \
         ledgered gap; {} passed + {} xfail != {total} declared",
        summary.passed,
        summary.xfail
    );
}

/// Tripwire: the corpus must keep declaring the 658 cases the pinned upstream
/// commit carries, read through the SAME aggregator this target runs.
///
/// `suite_inventory.rs` pins the per-group counts by loading each group manifest
/// directly. This pins the count the aggregator resolves, which is a different
/// claim: an `mf:include` list that lost a member would still leave every group
/// manifest on disk and every per-group count intact, and only this number would
/// move — the corpus would quietly stop being run without a single group going
/// missing.
#[test]
fn the_aggregator_resolves_every_vendored_case() {
    let cases = purrdf_sparql_conformance::manifest::load(&manifest())
        .unwrap_or_else(|e| panic!("load the SEP-0009 aggregator manifest: {e}"));
    assert_eq!(
        cases.len(),
        658,
        "vectors/sparql-cdt/manifest-all.ttl must resolve exactly 658 cases through \
         its mf:include closure; got {} — a group left the include list",
        cases.len()
    );
}
