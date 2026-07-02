// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The explicit expected-failure registry.
//!
//! Per the project's "no silent skips" doctrine, every conformance case the
//! native engine cannot yet pass is recorded HERE with a reason. The harness:
//!
//! * runs every discovered case (nothing is skipped at discovery time);
//! * for an `XFAIL` case, treats a real failure as the *expected* outcome but a
//!   surprise PASS as a HARD ERROR (so a stale xfail is caught and removed);
//! * prints an end-of-run tally (`N passed, M xfail, K unexpected-pass, …`).
//!
//! Entries are matched on the test-case IRI's local name (the fragment after the
//! manifest base), which is stable across vendored manifests.

/// Why a conformance case is expected to fail today.
///
/// Each variant is a *typed, justified* reason — never a catch-all. The full
/// W3C 1.1/1.2 corpus surfaces distinct failure classes, and bucketing them here
/// (rather than skipping) keeps the ledger doubling as a precise roadmap: the
/// matrix can report per-category counts, and a category emptying out is visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XfailReason {
    /// Uses a construct the native engine deliberately does not support yet.
    UnsupportedConstruct,
    /// A federated `SERVICE` shape the harness cannot resolve offline (e.g. a
    /// variable endpoint, which needs the lateral seam).
    PendingService,
    /// The result is format-/order-/blank-node-nondeterministic in a way this
    /// harness does not normalize.
    NonDeterministic,
    /// Known upstream erratum in the vendored fixture.
    UpstreamErratum,
    /// Requires an entailment regime (RDF/RDFS/D/OWL) whose closure the native
    /// reasoner does not (yet, or by spec-inherent boundary) materialize.
    Entailment,
    /// Invokes an extension / spec function or aggregate the engine has not
    /// implemented.
    CustomFunction,
    /// A result-format / result-shape (CSV/TSV/SRJ ordering) the comparer does
    /// not model.
    ResultFormat,
    /// An UPDATE operation whose post-state the engine computes differently
    /// (e.g. graph-existence edge cases where `CREATE`/`CLEAR` are no-ops).
    UpdateSemantics,
    /// A property-path form (negated property sets, nested `{n,m}`, etc.) the
    /// path evaluator does not yet handle.
    PropertyPath,
    /// Syntax the parser rejects — typically unstable SPARQL 1.2 / RDF-1.2
    /// draft grammar. (Stable-but-unimplemented syntax is in-scope work, NOT a
    /// ledger row.)
    ParseUnsupported,
}

impl XfailReason {
    /// A short human-readable label for the tally / logs.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::UnsupportedConstruct => "unsupported-construct",
            Self::PendingService => "pending-service",
            Self::NonDeterministic => "non-deterministic",
            Self::UpstreamErratum => "upstream-erratum",
            Self::Entailment => "entailment",
            Self::CustomFunction => "custom-function",
            Self::ResultFormat => "result-format",
            Self::UpdateSemantics => "update-semantics",
            Self::PropertyPath => "property-path",
            Self::ParseUnsupported => "parse-unsupported",
        }
    }
}

/// One registered expected failure: a case-IRI local-name suffix plus its reason.
#[derive(Debug)]
pub struct Xfail {
    /// Match when the case IRI ends with this string (usually its local name).
    pub iri_suffix: &'static str,
    /// Why it is expected to fail.
    pub reason: XfailReason,
}

/// The registry. Each entry is justified inline. Vendored W3C cases that the
/// native engine cannot yet pass are recorded here rather than skipped.
pub const XFAIL: &[Xfail] = &[
    // A `SERVICE` clause nested inside another `SERVICE`'s pattern is not yet
    // evaluated: the inner endpoint is never resolved against its source.
    Xfail {
        iri_suffix: "service3",
        reason: XfailReason::UnsupportedConstruct,
    },
    // A trailing top-level `VALUES` clause after the `WHERE` block is not yet
    // accepted by the parser (only inline `VALUES` inside a group is).
    Xfail {
        iri_suffix: "service4a",
        reason: XfailReason::UnsupportedConstruct,
    },
    // Variable-endpoint `SERVICE ?var` needs the lateral binding seam to bind
    // the endpoint from the surrounding solution before federating.
    Xfail {
        iri_suffix: "service5",
        reason: XfailReason::PendingService,
    },
];

/// The registered [`XfailReason`] for `case_iri`, if any.
#[must_use]
pub fn lookup(case_iri: &str) -> Option<XfailReason> {
    XFAIL
        .iter()
        .find(|x| case_iri.ends_with(x.iri_suffix))
        .map(|x| x.reason)
}
