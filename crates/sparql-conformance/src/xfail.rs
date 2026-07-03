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
    /// Syntax the parser does not yet accept — e.g. RDF-1.2 triple-term/reifier
    /// grammar. A genuine unimplemented feature (real work to land), tracked here
    /// until the parser implements it; the ledger shrinks as each lands.
    ParseUnsupported,
    /// The engine evaluates the case but yields a different solution value or
    /// lexical form than the spec expects (e.g. a numeric-function result whose
    /// datatype/canonical form diverges). A real correctness gap to close, not a
    /// missing feature — recorded so the divergence stays visible and typed.
    ValueMismatch,
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
            Self::ParseUnsupported => "parse-unsupported",
            Self::ValueMismatch => "value-mismatch",
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
    // === Full W3C sparql11 query-eval groups (commit 426c7df) ===============
    //
    // Every case below is a real gap the full vendored suite exposes; the
    // curated subset simply never exercised it. Grouped by root cause. Suffixes
    // are group-qualified (`<group>/manifest#<name>`) so they cannot cross-match.

    // --- XSD cast (`cast-decimal`/`cast-double`/`cast-float`): the fixture's
    //     expected `xsd:decimal`/`xsd:double`/`xsd:float` lexicals are NOT the XSD
    //     canonical mapping and are internally inconsistent about it (e.g. an
    //     `xsd:integer` `0` cast to `xsd:decimal` expects the bare, non-canonical
    //     "0" while the same cast of `1` correctly expects canonical "1.0"; a
    //     `double`/`float` constructor cast of `xsd:boolean true` correctly expects
    //     the canonical "1.0E0" while casting the string "1" expects the
    //     non-canonical, non-exponential "1"). The native engine emits the true
    //     XSD canonical literal mapping (mandatory exponential notation for
    //     double/float, a mandatory fractional digit for decimal) for every case
    //     uniformly, which cannot also reproduce these fixtures' inconsistent
    //     per-row non-canonical shortcuts — this is the vendored fixture's
    //     erratum, consistent with these three cases never having been promoted
    //     past `dawgt:approval dawgt:Proposed` in the manifest. `cast-bool`,
    //     `cast-int`, and `cast-string` (the numeric→boolean/integer casts and the
    //     XPath F&O §19 numeric/boolean→`xsd:string` casting rule) are spec-clean
    //     and pass natively; they are not ledgered here.
    Xfail {
        iri_suffix: "cast/manifest#cast-decimal",
        reason: XfailReason::UpstreamErratum,
    },
    Xfail {
        iri_suffix: "cast/manifest#cast-double",
        reason: XfailReason::UpstreamErratum,
    },
    Xfail {
        iri_suffix: "cast/manifest#cast-float",
        reason: XfailReason::UpstreamErratum,
    },
    // --- Whole-valued `xsd:decimal` lexical form: the vendored W3C SPARQL 1.1
    //     suite is INTERNALLY INCONSISTENT, at the SAME `dawgt:Approved`
    //     resolution, about how a COMPUTED integer-valued `xsd:decimal` result is
    //     serialized. `functions#coalesce01` (Approved) expects `?div = 0/2` as
    //     "0.0" and `4/2` as "2.0" — the XSD-1.0-legacy form WITH a mandatory
    //     decimal point — while `functions#ceil01`/`floor01`/`round01`/`seconds`
    //     (also Approved) expect "3"/"2"/"1"/"0" — the XSD 1.1 canonical form with
    //     NO decimal point for an integer-valued decimal. The Proposed
    //     `plus-1-corrected` (whose `?sum = ?x + ?y` COMPUTES a whole decimal)
    //     follows coalesce01's "1.0"/"3.0" legacy form. No single deterministic
    //     serializer can satisfy both sets, so one side is an unavoidable
    //     vendored-fixture erratum. (`plus-2-corrected` is deliberately NOT
    //     ledgered: its "1.0" is an ECHOED source decimal from `data-builtin-3.ttl`
    //     preserved verbatim by the round-trip codec — it never flows through the
    //     canonical serializer, so it is unaffected and still passes.)
    //
    //     PurRDF targets SPARQL 1.1, which normatively references XSD 1.1, so the
    //     engine emits the XSD 1.1 canonical decimal (§3.3.3.2: no decimal point
    //     for an integer-valued decimal) uniformly. The ceil/floor/round/seconds
    //     fixtures therefore PASS natively; the fixtures below carry the legacy
    //     "X.0" expectation and are the ledgered erratum. Their value and datatype
    //     are computed correctly — only the divergent legacy lexical differs.
    Xfail {
        iri_suffix: "functions/manifest#coalesce01",
        reason: XfailReason::UpstreamErratum,
    },
    Xfail {
        iri_suffix: "functions/manifest#plus-1-corrected",
        reason: XfailReason::UpstreamErratum,
    },
    // === Full W3C sparql11 UPDATE-eval groups (commit 426c7df) ===============
    //
    // The update groups (add/basic-update/clear/copy/delete*/drop/move/
    // update-silent) run through the UpdateEval harness path and all pass
    // natively (including per-operation blank-node scoping, compared by
    // RDFC-1.0 canonical N-Quads).

    // === W3C sparql11 entailment-regime group (commit 426c7df) ================
    //
    // The native reasoner (purrdf-entail) materializes RDF/RDFS + OWL-RL-shaped
    // closure, so all rdf*/rdfs*/lang/plainLit/bind* cases and many OWL cases
    // pass. The residual failures require regimes a forward-materialization
    // reasoner cannot reach — spec-inherent boundaries, each ledgered `Entailment`:
    //   - OWL-Direct-only tests (`parent*`, `simple*`) and OWL-DL query answering
    //     (`sparqldl-*`, `paper-sparqldl-Q*`, `owlds02`) — full DL is not a
    //     materialize-and-match affair;
    //   - RIF-rule entailment (`rif*`) — the RIF regime is out of scope;
    //   - RDF axiomatic-triple entailment under the bare RDF regime (`rdf01`).
    Xfail {
        iri_suffix: "entailment/manifest#parent3",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#parent4",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#parent5",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#parent6",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#parent7",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#parent8",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#parent9",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#parent10",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#simple1",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#simple2",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#simple3",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#simple4",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#simple5",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#simple6",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#simple7",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#simple8",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#sparqldl-02",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#sparqldl-03",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#sparqldl-10",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#sparqldl-11",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#sparqldl-12",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#sparqldl-13",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#paper-sparqldl-Q1",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#paper-sparqldl-Q2",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#paper-sparqldl-Q3",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#owlds02",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#rif01",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#rif03",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#rif04",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#rif06",
        reason: XfailReason::Entailment,
    },
    Xfail {
        iri_suffix: "entailment/manifest#rdf01",
        reason: XfailReason::Entailment,
    },
    // === W3C SPARQL 1.2 / RDF-1.2 group (commit 426c7df) ====================
    //
    // SPARQL 1.2 (RDF-star: triple terms, reifiers, base-direction) is a complete
    // first-class spec here (see suite/w3c-sparql12/PROVENANCE.md). The engine now
    // passes the full triple-term/reifier/annotation surface — including the
    // graph-scoped `eval-triple-terms` cases (`graphs-1`, `graphs-2`, `expr-1`): the
    // RDF 1.2 reifier/annotation side-tables carry a graph dimension end-to-end
    // (parse fold, IR storage, RDFC-1.0 canonicalization, the GTS reader/writer, the
    // N-Quads/TriG serializer, and the BGP virtual-candidate match), so a reifier
    // declared inside `GRAPH g { << s p o >> … }` binds `?g` under `GRAPH ?g`.
];

/// The registered [`XfailReason`] for `case_iri`, if any.
#[must_use]
pub fn lookup(case_iri: &str) -> Option<XfailReason> {
    XFAIL
        .iter()
        .find(|x| case_iri.ends_with(x.iri_suffix))
        .map(|x| x.reason)
}
