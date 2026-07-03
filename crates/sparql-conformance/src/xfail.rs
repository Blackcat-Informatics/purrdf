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
            Self::PropertyPath => "property-path",
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

    // Expected result is a Turtle-encoded `rs:ResultSet` (not a CONSTRUCT graph);
    // the harness models `.ttl` results as graphs, so the SELECT solutions cannot
    // be compared against the result-set encoding yet.
    Xfail {
        iri_suffix: "bindings/manifest#graph",
        reason: XfailReason::ResultFormat,
    },
    // --- XSD cast: the engine evaluates but the cast result's datatype/lexical
    //     form diverges from the spec's expected solution. -----------------------
    Xfail {
        iri_suffix: "cast/manifest#cast-bool",
        reason: XfailReason::ValueMismatch,
    },
    Xfail {
        iri_suffix: "cast/manifest#cast-decimal",
        reason: XfailReason::ValueMismatch,
    },
    Xfail {
        iri_suffix: "cast/manifest#cast-double",
        reason: XfailReason::ValueMismatch,
    },
    Xfail {
        iri_suffix: "cast/manifest#cast-float",
        reason: XfailReason::ValueMismatch,
    },
    Xfail {
        iri_suffix: "cast/manifest#cast-int",
        reason: XfailReason::ValueMismatch,
    },
    Xfail {
        iri_suffix: "cast/manifest#cast-string",
        reason: XfailReason::ValueMismatch,
    },
    // --- CONSTRUCT: the `()` collection template and the `CONSTRUCT WHERE {}`
    //     shorthand (incl. a trailing FROM) are not yet parsed. ------------------
    Xfail {
        iri_suffix: "construct/manifest#constructlist",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "construct/manifest#constructwhere01",
        reason: XfailReason::UnsupportedConstruct,
    },
    Xfail {
        iri_suffix: "construct/manifest#constructwhere02",
        reason: XfailReason::UnsupportedConstruct,
    },
    Xfail {
        iri_suffix: "construct/manifest#constructwhere03",
        reason: XfailReason::UnsupportedConstruct,
    },
    Xfail {
        iri_suffix: "construct/manifest#constructwhere04",
        reason: XfailReason::UnsupportedConstruct,
    },
    // --- EXISTS whose inner pattern references the enclosing GRAPH variable
    //     yields the wrong solution set. ------------------------------------------
    Xfail {
        iri_suffix: "exists/manifest#exists-graph-variable",
        reason: XfailReason::UnsupportedConstruct,
    },
    // --- Built-in functions: the engine evaluates but the produced value/lexical
    //     form diverges (numeric CEIL/FLOOR/ROUND datatype, unary plus, SECONDS,
    //     STRAFTER/STRBEFORE, STRDT/STRLANG, IRI). `bnode0*` differ only by blank-
    //     node label (the harness does not do bnode isomorphism). -----------------
    Xfail {
        iri_suffix: "functions/manifest#bnode01",
        reason: XfailReason::NonDeterministic,
    },
    Xfail {
        iri_suffix: "functions/manifest#bnode02",
        reason: XfailReason::NonDeterministic,
    },
    Xfail {
        iri_suffix: "functions/manifest#ceil01",
        reason: XfailReason::ValueMismatch,
    },
    Xfail {
        iri_suffix: "functions/manifest#floor01",
        reason: XfailReason::ValueMismatch,
    },
    Xfail {
        iri_suffix: "functions/manifest#round01",
        reason: XfailReason::ValueMismatch,
    },
    Xfail {
        iri_suffix: "functions/manifest#iri01",
        reason: XfailReason::ValueMismatch,
    },
    Xfail {
        iri_suffix: "functions/manifest#plus-1-corrected",
        reason: XfailReason::ValueMismatch,
    },
    Xfail {
        iri_suffix: "functions/manifest#plus-2-corrected",
        reason: XfailReason::ValueMismatch,
    },
    Xfail {
        iri_suffix: "functions/manifest#seconds",
        reason: XfailReason::ValueMismatch,
    },
    Xfail {
        iri_suffix: "functions/manifest#strafter02",
        reason: XfailReason::ValueMismatch,
    },
    Xfail {
        iri_suffix: "functions/manifest#strbefore02",
        reason: XfailReason::ValueMismatch,
    },
    Xfail {
        iri_suffix: "functions/manifest#strdt01",
        reason: XfailReason::ValueMismatch,
    },
    Xfail {
        iri_suffix: "functions/manifest#strdt03-rdf11",
        reason: XfailReason::ValueMismatch,
    },
    Xfail {
        iri_suffix: "functions/manifest#strlang01",
        reason: XfailReason::ValueMismatch,
    },
    Xfail {
        iri_suffix: "functions/manifest#strlang02",
        reason: XfailReason::ValueMismatch,
    },
    Xfail {
        iri_suffix: "functions/manifest#strlang03-rdf11",
        reason: XfailReason::ValueMismatch,
    },
    // --- Grouping: projecting a non-grouped variable must be a query error;
    //     the parser/algebra does not yet reject it (negative-syntax tests). -----
    Xfail {
        iri_suffix: "grouping/manifest#group06",
        reason: XfailReason::UnsupportedConstruct,
    },
    Xfail {
        iri_suffix: "grouping/manifest#group07",
        reason: XfailReason::UnsupportedConstruct,
    },
    // --- Property paths: inverse (`^`) inside a negated property set, and
    //     zero-or-more / zero-or-one over a property set at a bound endpoint,
    //     are not yet evaluated (pp11/pp31 drop a result). -----------------------
    Xfail {
        iri_suffix: "property-path/manifest#nps_a_inverse",
        reason: XfailReason::PropertyPath,
    },
    Xfail {
        iri_suffix: "property-path/manifest#nps_direct_and_inverse",
        reason: XfailReason::PropertyPath,
    },
    Xfail {
        iri_suffix: "property-path/manifest#nps_inverse",
        reason: XfailReason::PropertyPath,
    },
    Xfail {
        iri_suffix: "property-path/manifest#pp11",
        reason: XfailReason::PropertyPath,
    },
    Xfail {
        iri_suffix: "property-path/manifest#pp31",
        reason: XfailReason::PropertyPath,
    },
    Xfail {
        iri_suffix: "property-path/manifest#zero_or_more_set_end",
        reason: XfailReason::PropertyPath,
    },
    Xfail {
        iri_suffix: "property-path/manifest#zero_or_more_set_start",
        reason: XfailReason::PropertyPath,
    },
    Xfail {
        iri_suffix: "property-path/manifest#zero_or_one_set_end",
        reason: XfailReason::PropertyPath,
    },
    Xfail {
        iri_suffix: "property-path/manifest#zero_or_one_set_start",
        reason: XfailReason::PropertyPath,
    },
    // === Full W3C sparql11 UPDATE-eval groups (commit 426c7df) ===============
    //
    // The update groups (add/basic-update/clear/copy/delete*/drop/move/
    // update-silent) run through the UpdateEval harness path; the vast majority
    // pass. The residual gaps are post-state semantics differences (compared by
    // RDFC-1.0 canonical N-Quads, so these are genuine structural divergences,
    // not blank-node relabelling).

    // COPY/ADD graph edge cases (e.g. copying a graph onto itself, or ADD
    // involving graph existence) leave a different post-state.
    Xfail {
        iri_suffix: "copy/manifest#copy03",
        reason: XfailReason::UpdateSemantics,
    },
    Xfail {
        iri_suffix: "copy/manifest#copy04",
        reason: XfailReason::UpdateSemantics,
    },
    Xfail {
        iri_suffix: "add/manifest#add04",
        reason: XfailReason::UpdateSemantics,
    },
    // Blank-node scoping across separate INSERT operations in one request: a
    // `_:b` label in one operation must denote a fresh bnode distinct from the
    // same label in another operation; the engine reuses it, so the post-state
    // triple count diverges.
    Xfail {
        iri_suffix: "basic-update/manifest#insert-where-same-bnode",
        reason: XfailReason::UpdateSemantics,
    },
    Xfail {
        iri_suffix: "basic-update/manifest#insert-where-same-bnode2",
        reason: XfailReason::UpdateSemantics,
    },
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
    // first-class spec here (see suite/w3c-sparql12/PROVENANCE.md). The surface
    // the engine already satisfies passes (codepoint-escapes, grouping, rdf11, and
    // much of the triple-term syntax); the residuals are genuine unimplemented
    // features — `parse-unsupported` for triple-term/reifier grammar the parser
    // does not yet accept, `unsupported-construct` for triple-term eval semantics
    // — real work to land, not provisional-spec placeholders.
    Xfail {
        iri_suffix: "eval-triple-terms/manifest#basic-7",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "eval-triple-terms/manifest#basic-9",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "eval-triple-terms/manifest#construct-1",
        reason: XfailReason::UnsupportedConstruct,
    },
    Xfail {
        iri_suffix: "eval-triple-terms/manifest#construct-2",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "eval-triple-terms/manifest#construct-3",
        reason: XfailReason::UnsupportedConstruct,
    },
    Xfail {
        iri_suffix: "eval-triple-terms/manifest#construct-4",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "eval-triple-terms/manifest#construct-5",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "eval-triple-terms/manifest#expr-1",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "eval-triple-terms/manifest#graphs-1",
        reason: XfailReason::UnsupportedConstruct,
    },
    Xfail {
        iri_suffix: "eval-triple-terms/manifest#graphs-2",
        reason: XfailReason::UnsupportedConstruct,
    },
    Xfail {
        iri_suffix: "eval-triple-terms/manifest#op-1",
        reason: XfailReason::UnsupportedConstruct,
    },
    Xfail {
        iri_suffix: "eval-triple-terms/manifest#op-2",
        reason: XfailReason::UnsupportedConstruct,
    },
    Xfail {
        iri_suffix: "eval-triple-terms/manifest#order-1",
        reason: XfailReason::UnsupportedConstruct,
    },
    Xfail {
        iri_suffix: "eval-triple-terms/manifest#order-2",
        reason: XfailReason::UnsupportedConstruct,
    },
    Xfail {
        iri_suffix: "eval-triple-terms/manifest#pattern-8",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "eval-triple-terms/manifest#results-reifiedtriples-1j",
        reason: XfailReason::UnsupportedConstruct,
    },
    Xfail {
        iri_suffix: "eval-triple-terms/manifest#results-reifiedtriples-1x",
        reason: XfailReason::UnsupportedConstruct,
    },
    Xfail {
        iri_suffix: "eval-triple-terms/manifest#update-1",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "eval-triple-terms/manifest#update-2",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "eval-triple-terms/manifest#update-3",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "expression/manifest#not-not",
        reason: XfailReason::UnsupportedConstruct,
    },
    Xfail {
        iri_suffix: "expression/manifest#triple-on-literals",
        reason: XfailReason::UnsupportedConstruct,
    },
    Xfail {
        iri_suffix: "expression/manifest#triple-on-str-literals",
        reason: XfailReason::UnsupportedConstruct,
    },
    Xfail {
        iri_suffix: "expression/manifest#triple-on-triple-terms",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "lang-basedir/manifest#concat",
        reason: XfailReason::UnsupportedConstruct,
    },
    Xfail {
        iri_suffix: "lang-basedir/manifest#contains",
        reason: XfailReason::UnsupportedConstruct,
    },
    Xfail {
        iri_suffix: "lang-basedir/manifest#langdir-literal",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "lang-basedir/manifest#strlang",
        reason: XfailReason::UnsupportedConstruct,
    },
    Xfail {
        iri_suffix: "lang-basedir/manifest#strlangdir",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-negative/manifest#tripleterm-subject-01",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-negative/manifest#tripleterm-subject-02",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-negative/manifest#tripleterm-subject-04",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-negative/manifest#tripleterm-subject-05",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#annotation-anonreifier-09",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#annotation-anonreifier-multiple-03",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#annotation-anonreifier-multiple-04",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#annotation-reifier-03",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#annotation-reifier-09",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#annotation-reifier-multiple-06",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#annotation-reifier-multiple-07",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#annotation-reifier-multiple-08",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#annotation-reifier-multiple-09",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#annotation-reifier-multiple-10",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#basic-anonreifier-07",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#basic-anonreifier-08",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#basic-anonreifier-09",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#basic-anonreifier-10",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#basic-anonreifier-11",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#basic-anonreifier-12",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#basic-anonreifier-13",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#basic-reifier-03",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#basic-reifier-06",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#basic-reifier-07",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#basic-reifier-08",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#basic-reifier-09",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#basic-reifier-10",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#basic-reifier-11",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#basic-reifier-12",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#basic-reifier-13",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#basic-tripleterm-07",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#bnode-reifier-02",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#compound-all",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#compound-reifier",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#expr-tripleterm-01",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#expr-tripleterm-06",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#inside-reifier-02",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#nested-reifier-01",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#nested-reifier-02",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#subject-tripleterm",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#update-reifier-01",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#update-reifier-03",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#update-reifier-04",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#update-reifier-07",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax-triple-terms-positive/manifest#update-reifier-08",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax/manifest#duplicated-values-variable",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax/manifest#group-by-scope-bad-1",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax/manifest#group-by-scope-bad-2",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "syntax/manifest#group-by-scope-bad-3",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "version/manifest#version-01",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "version/manifest#version-02",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "version/manifest#version-03",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "version/manifest#version-04",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "version/manifest#version-05",
        reason: XfailReason::ParseUnsupported,
    },
    Xfail {
        iri_suffix: "version/manifest#version-06",
        reason: XfailReason::ParseUnsupported,
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
