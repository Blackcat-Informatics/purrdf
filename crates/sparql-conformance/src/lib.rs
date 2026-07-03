// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0
#![forbid(unsafe_code)]

//! Native W3C SPARQL 1.1 conformance harness.
//!
//! Discovers `mf:` test manifests, runs each case against the native
//! [`purrdf_sparql_eval`] engine (zero oxigraph Store), and diffs the result
//! against the expected SPARQL Results (SRX/SRJ) or canonical N-Quads. The
//! datatest-stable test harness (`tests/sparql_conformance.rs`) emits one
//! nextest case per `manifest.ttl`; each loops its entries via [`run_manifest`].
//!
//! Expected failures are recorded in [`xfail`] — never skipped — and the
//! per-manifest [`Summary`] prints a tally (`passed / xfail / unexpected-pass /
//! failed`). An xfail that unexpectedly PASSES is a hard error so the registry
//! cannot rot.

pub mod compare;
pub mod manifest;
pub mod paths;
pub mod rs_resultset;
pub mod run;
pub mod service;
pub mod xfail;

use std::path::Path;

use manifest::SparqlTestCase;
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

    /// A detailed failure report for the datatest error message.
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

/// Run every case declared by `manifest_path`, honoring the [`xfail`] registry.
///
/// # Errors
///
/// Returns a message if the manifest itself cannot be loaded/parsed.
pub fn run_manifest(manifest_path: &Path) -> Result<Summary, String> {
    let cases = manifest::load(manifest_path)?;
    let mut summary = Summary::default();
    for case in &cases {
        match verdict_of(case) {
            Verdict::Unmodeled => summary.unmodeled.push(case.iri.clone()),
            Verdict::Pass => match xfail::lookup(&case.iri) {
                Some(reason) => summary.unexpected_pass.push(format!(
                    "{} (xfail: {})",
                    case.iri,
                    reason.label()
                )),
                None => summary.passed += 1,
            },
            Verdict::Fail(msg) => match xfail::lookup(&case.iri) {
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
    // Federated cases (`qt:serviceData`) resolve `SERVICE` through an in-memory
    // source mapping each endpoint IRI to its data file (offline, deterministic).
    let remote = match service::build(case) {
        Ok(source) => source,
        Err(msg) => return Verdict::Fail(msg),
    };
    let remote = remote
        .as_ref()
        .map(|s| s as &dyn purrdf_sparql_eval::RemoteQuerySource);
    match run::run(case, remote) {
        Ok(outcome) => match compare::compare(case, &outcome) {
            Ok(()) => Verdict::Pass,
            Err(msg) => Verdict::Fail(msg),
        },
        Err(msg) => Verdict::Fail(msg),
    }
}

/// Log an expected failure (with its reason) so xfails are visible, not silent.
fn log_xfail(iri: &str, reason: XfailReason, msg: &str) {
    eprintln!("[xfail: {}] {iri} — {msg}", reason.label());
}
