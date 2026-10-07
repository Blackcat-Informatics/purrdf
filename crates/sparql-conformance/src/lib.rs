// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
#![forbid(unsafe_code)]

//! Native W3C SPARQL 1.0/1.1/1.2 conformance harness.
//!
//! Discovers `mf:` test manifests, runs each case against the native
//! [`purrdf_sparql_eval`] engine, and diffs the result
//! against the expected SPARQL Results, Turtle/RDFXML result sets or canonical
//! N-Quads. The
//! `harness = false` test target `tests/sparql_conformance.rs` runs one case per
//! group manifest that [`discover`] finds under `suite/`; each loops its entries
//! via [`run_manifest`]. An index manifest (one that only includes others) is
//! checked for coverage and never run through its members.
//!
//! Expected failures are recorded in [`xfail`] — never skipped — and the
//! per-manifest [`Summary`] prints a tally (`passed / xfail / unexpected-pass /
//! failed`). An xfail that unexpectedly PASSES is a hard error so the registry
//! cannot rot.
//!
//! The crate also hosts two further, unrelated corpora, both cut from the same
//! upstream W3C OWL 2 manifest and each grading a different reasoning lane:
//!
//! * [`owl2`] grades `entailment-suite/w3c-owl2/` — 261 *consistency*-shaped
//!   cases — against PurRDF's `OWL-Direct` SHOIQ(D) tableau. It validates the DL
//!   lane's satisfiability verdicts and says nothing about the OWL 2 RL rule
//!   table.
//! * [`owl2_rl`] grades `entailment-suite/w3c-owl2-rl/` — 27 positive and 23
//!   negative W3C *entailment* tests — against the OWL 2 RL forward chase. This
//!   is the rule table's first third-party oracle.
//!
//! Both feed the conformance matrix's own `Entailment` row rather than folding
//! into the SPARQL row, and both report what they leave out: `owl2_rl`'s
//! `census.tsv` accounts for all 489 upstream cases, and the harnesses print the
//! exclusion tallies next to their scoreboards. They live here because this is
//! the workspace's `publish = false` conformance crate, so their corpora never
//! enter a published `.crate`.

pub mod community;
pub mod compare;
pub mod ledger;
pub mod manifest;
pub mod mode_restricted;
pub mod owl2;
pub mod owl2_rl;
pub mod paths;
pub mod rs_resultset;
pub mod run;
pub mod service;
pub mod xfail;

use std::path::{Path, PathBuf};

use manifest::SparqlTestCase;
use paths::SuiteManifest;
use xfail::XfailReason;

/// Per-manifest run summary.
#[derive(Debug, Default)]
pub struct Summary {
    /// Cases that passed (and were not registered as xfail).
    pub passed: usize,
    /// Cases that failed as their xfail entry expected.
    pub xfail: usize,
    /// Registered-xfail cases that unexpectedly PASSED (a hard error: the entry
    /// is stale and must be removed). Carries the case IRI + reason label.
    pub unexpected_pass: Vec<String>,
    /// Cases that failed without an xfail entry: `(case IRI, message)`.
    pub failed: Vec<(String, String)>,
    /// Cases whose `rdf:type` the harness does not model. A HARD ERROR: an
    /// unmodeled type is a silent-skip hole, so it fails the manifest until the
    /// harness models the type (or a modeled no-op is added with a reason).
    pub unmodeled: Vec<String>,
}

impl Summary {
    /// True when the manifest passed: no unexpected passes, no unexplained
    /// failures, and no unmodeled test types (xfails are allowed).
    ///
    /// `unmodeled` is fatal by design (the "no silent skips" doctrine): a test
    /// whose `rdf:type` the harness does not recognize would otherwise pass
    /// green without ever running, so it must fail loudly until modeled.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.unexpected_pass.is_empty() && self.failed.is_empty() && self.unmodeled.is_empty()
    }

    /// A one-line tally for the run log.
    #[must_use]
    pub fn tally_line(&self) -> String {
        format!(
            "{} passed, {} xfail, {} unexpected-pass, {} failed, {} unmodeled",
            self.passed,
            self.xfail,
            self.unexpected_pass.len(),
            self.failed.len(),
            self.unmodeled.len(),
        )
    }

    /// A detailed failure report for the failing manifest case's message.
    #[must_use]
    pub fn failure_report(&self) -> String {
        let mut lines = Vec::new();
        for iri in &self.unexpected_pass {
            lines.push(format!("  • UNEXPECTED PASS (remove xfail): {iri}"));
        }
        for (iri, msg) in &self.failed {
            lines.push(format!("  • FAIL {iri}: {msg}"));
        }
        for iri in &self.unmodeled {
            lines.push(format!(
                "  • UNMODELED (harness models no TestKind for this rdf:type — model it or add a reasoned no-op): {iri}"
            ));
        }
        lines.join("\n")
    }
}

/// The verdict for a single case before xfail accounting.
enum Verdict {
    Pass,
    Fail(String),
    Unmodeled,
}

/// The manifests [`paths::suite_manifests`] finds under a root, sorted by role.
#[derive(Debug)]
pub struct Discovery {
    /// Every group manifest: each runs as its own case.
    pub groups: Vec<SuiteManifest>,
    /// Every index (see [`manifest::load`]) with its members. An index is never
    /// run through its members, which are discovered and run on their own.
    pub indexes: Vec<(SuiteManifest, Vec<PathBuf>)>,
}

/// Discover the manifests under `root` and sort them into groups and indexes.
///
/// # Errors
///
/// Returns a message when discovery fails, a manifest cannot be read, or an
/// index names a member that is not itself among the discovered manifests: the
/// runner never runs an index's members through it, so such a member would be
/// dropped without trace.
pub fn discover(root: &Path) -> Result<Discovery, String> {
    let found = paths::suite_manifests(root)
        .map_err(|e| format!("discovering manifests under {}: {e}", root.display()))?;
    let mut discovered = std::collections::BTreeSet::new();
    let mut groups = Vec::new();
    let mut indexes = Vec::new();
    for manifest in found {
        discovered.insert(
            manifest
                .path
                .canonicalize()
                .map_err(|e| format!("resolve {}: {e}", manifest.path.display()))?,
        );
        match manifest::index_members(&manifest.path)? {
            Some(members) => indexes.push((manifest, members)),
            None => groups.push(manifest),
        }
    }
    for (index, members) in &indexes {
        for member in members {
            let canonical = member
                .canonicalize()
                .map_err(|e| format!("resolve {}: {e}", member.display()))?;
            if !discovered.contains(&canonical) {
                return Err(format!(
                    "{}: the index includes {}, which discovery under {} did not find",
                    index.path.display(),
                    member.display(),
                    root.display()
                ));
            }
        }
    }
    Ok(Discovery { groups, indexes })
}

/// Run every case declared by `manifest_path`, honoring the [`xfail`] registry.
///
/// # Errors
///
/// Returns a message if the manifest itself cannot be loaded/parsed, if it is an
/// index (whose members run as their own cases, so running it would run them
/// twice), or if a case IRI is matched by more than one [`xfail`] registry entry.
pub fn run_manifest(manifest_path: &Path) -> Result<Summary, String> {
    if manifest::index_members(manifest_path)?.is_some() {
        return Err(format!(
            "{} is an index: its members run as their own cases, and running it would run \
             every one of them twice",
            manifest_path.display()
        ));
    }
    let cases = manifest::load(manifest_path)?;
    let mut summary = Summary::default();
    for case in &cases {
        match verdict_of(case) {
            Verdict::Unmodeled => summary.unmodeled.push(case.iri.clone()),
            // An ambiguous ledger entry is a hard error, not a preference — see
            // `xfail::lookup` — so it aborts the whole manifest rather than letting
            // one of the two matching entries win by registry order.
            Verdict::Pass => match xfail::lookup(&case.iri)? {
                Some(reason) => summary.unexpected_pass.push(format!(
                    "{} (xfail: {})",
                    case.iri,
                    reason.label()
                )),
                None => summary.passed += 1,
            },
            Verdict::Fail(msg) => match xfail::lookup(&case.iri)? {
                Some(reason) => {
                    log_xfail(&case.iri, reason, &msg);
                    summary.xfail += 1;
                }
                None => summary.failed.push((case.iri.clone(), msg)),
            },
        }
    }
    Ok(summary)
}

/// Run + compare a single case into a [`Verdict`].
fn verdict_of(case: &SparqlTestCase) -> Verdict {
    if matches!(case.kind, manifest::TestKind::Unknown) {
        return Verdict::Unmodeled;
    }
    // Every case resolves `SERVICE` through an in-memory source mapping each
    // `qt:serviceData` endpoint IRI to its data file and failing every other endpoint
    // as unreachable (offline, deterministic).
    let remote = match service::build(case) {
        Ok(source) => source,
        Err(msg) => return Verdict::Fail(msg),
    };
    match run::run(case, Some(&remote)) {
        Ok(outcome) => match compare::compare(case, &outcome) {
            Ok(()) => Verdict::Pass,
            Err(msg) => Verdict::Fail(msg),
        },
        // A run failure is graded, not assumed: a case whose `mf:result` is a `.err`
        // file EXPECTED to be refused and passes when the diagnostic says what the
        // file says. Every other case gets its diagnostic back unchanged.
        Err(msg) => match compare::compare_failure(case, &msg) {
            Ok(()) => Verdict::Pass,
            Err(reported) => Verdict::Fail(reported),
        },
    }
}

/// Log an expected failure (with its reason) so xfails are visible, not silent.
fn log_xfail(iri: &str, reason: XfailReason, msg: &str) {
    eprintln!("[xfail: {}] {iri} — {msg}", reason.label());
}
