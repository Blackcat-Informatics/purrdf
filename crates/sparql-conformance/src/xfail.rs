// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! # How an entry is matched
//!
//! An entry names a **tail** of the case IRI, and the match is ANCHORED at an IRI
//! path-segment boundary: `case_iri` must end with [`Xfail::iri_tail`] AND the
//! character immediately before it must be `/` (or the tail must be the whole
//! IRI). A bare `ends_with` would let a short tail like `t01` swallow `subtest01`
//! in an unrelated group, so an entry could mark a passing test xfail or mask a
//! real failure without anything saying so.
//!
//! Two further rules keep the ledger from rotting, both HARD ERRORS rather than
//! silent no-ops:
//!
//! * an IRI matched by MORE THAN ONE entry is refused by [`lookup`] — two reasons
//!   for one case means the ledger cannot say which gap the case represents;
//! * an entry matching ZERO cases across the whole live suite is refused by the
//!   `every_xfail_entry_matches_exactly_one_case` test in
//!   `tests/xfail_ledger.rs` — a dead entry is a ceiling with nothing under it,
//!   and it keeps the budget in `scripts/conformance-baseline.json` propped up
//!   after the gap it named is gone.
//!
//! Case IRIs are globally unique by construction: every manifest is parsed
//! against its OWN base (see [`crate::manifest`]), so two group manifests that
//! each declare the relative `@prefix : <manifest#>` no longer mint the same IRI
//! for a shared local name, and [`crate::manifest::load`] refuses a closure in
//! which two manifests mint one IRI anyway.

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
    /// A frozen older specification's expectation conflicts with the current
    /// RDF/SPARQL contract. The entry cites the changed normative rule.
    HistoricalSemantics,
    /// A frozen result requires a different permitted numeric lexical mapping
    /// from the shipped canonical mapping. Independent native literal answers
    /// pin value, datatype and exact representation outside the ledger.
    RepresentationDifference,
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
            Self::HistoricalSemantics => "historical-semantics",
            Self::RepresentationDifference => "representation-difference",
            Self::Entailment => "entailment",
            Self::CustomFunction => "custom-function",
            Self::ResultFormat => "result-format",
            Self::UpdateSemantics => "update-semantics",
            Self::ParseUnsupported => "parse-unsupported",
            Self::ValueMismatch => "value-mismatch",
        }
    }
}

/// One registered expected failure: an anchored case-IRI tail plus its reason.
#[derive(Debug)]
pub struct Xfail {
    /// The tail of the case IRI this entry governs — conventionally
    /// `<group>/manifest#<local-name>`, which cannot cross-match between groups.
    ///
    /// Matched by [`matches()`]: the case IRI must END with this string and the
    /// match must START at an IRI path-segment boundary, so a tail can never
    /// capture the back half of a longer segment.
    pub iri_tail: &'static str,
    /// Why it is expected to fail.
    pub reason: XfailReason,
}

/// Whether `case_iri` is governed by the entry tail `iri_tail`.
///
/// The tail must be a suffix of the IRI AND begin at a `/` boundary (or be the
/// whole IRI). Anchoring is what stops a short tail from matching the back half
/// of an unrelated case's local name.
#[must_use]
pub fn matches(case_iri: &str, iri_tail: &str) -> bool {
    let Some(head) = case_iri.strip_suffix(iri_tail) else {
        return false;
    };
    head.is_empty() || head.ends_with('/')
}

/// The registry. Each entry is justified inline. Vendored W3C cases that the
/// native engine cannot yet pass are recorded here rather than skipped.
pub const XFAIL: &[Xfail] = &[
    // === W3C SPARQL 1.0 data-r2 ============================================
    // RDF 1.2 Concepts §3.4.1 makes a simple literal syntactic sugar for the
    // same xsd:string term: https://www.w3.org/TR/rdf12-concepts/#section-Graph-Literal
    // SPARQL 1.2 §15.1 therefore orders all eight string values together:
    // https://www.w3.org/TR/sparql12-query/#modOrderBy
    // The frozen extension instead places all four plain spellings before
    // all four typed spellings. The native independent eight-row oracle in
    // data_r2_harness.rs pins the current order without changing this fixture.
    Xfail {
        iri_tail: "w3c-sparql10/sort/#dawg-sort-11",
        reason: XfailReason::HistoricalSemantics,
    },
    // These four frozen cases require mf:KnownTypesDefault2Neq and mark
    // mf:IllFormedLiteral in their manifest. Their old rule treats the ill-typed
    // integer "xyz" as unequal to a known string/language value. SPARQL 1.2
    // §17.4.2.2 instead requires that literal comparison to raise an error:
    // https://www.w3.org/TR/sparql12-query/#func-sameValue
    // The independently specified native 8×8 truth tables in data_r2_harness.rs
    // prove the complete current pair sets (34/44/44/18 rows); the frozen sets
    // are 42/52/52/10. Known language inequality is repaired, not expected to
    // fail; unknown datatype errors, same-term equality and non-literal
    // inequality remain covered by the same native oracle and the exact ledger.
    Xfail {
        iri_tail: "data-r2/open-world/manifest#open-eq-08",
        reason: XfailReason::HistoricalSemantics,
    },
    Xfail {
        iri_tail: "data-r2/open-world/manifest#open-eq-10",
        reason: XfailReason::HistoricalSemantics,
    },
    Xfail {
        iri_tail: "data-r2/open-world/manifest#open-eq-11",
        reason: XfailReason::HistoricalSemantics,
    },
    Xfail {
        iri_tail: "data-r2/open-world/manifest#open-eq-12",
        reason: XfailReason::HistoricalSemantics,
    },
    // XSD 1.1 §3.3.4.2 / §3.3.5.2 permits more than one float/double
    // character mapping: https://www.w3.org/TR/xmlschema11-2/#float
    // The native computed mapping uses §E.1's exponential representation,
    // while these frozen results use bare whole numbers (e.g. "6" vs
    // "6.0E0"). Neither spelling is an invalid value. The independent native
    // literal oracle verifies all 64 binary type pairs and eight unary rows,
    // including promoted result types and verbatim echoed source bindings.
    // Keep exact literal comparison: do not normalize source or result terms
    // merely to make these six representation expectations match. The DAWG
    // test-suite contract grades result graphs as equivalent only when they
    // "have identical IRI and literal nodes", so value comparison is not a
    // permitted reading of these fixtures:
    // https://www.w3.org/2001/sw/DataAccess/tests/README.html
    Xfail {
        iri_tail: "data-r2/expr-ops/manifest#add-numbers-cast",
        reason: XfailReason::RepresentationDifference,
    },
    Xfail {
        iri_tail: "data-r2/expr-ops/manifest#subtract-numbers-cast",
        reason: XfailReason::RepresentationDifference,
    },
    Xfail {
        iri_tail: "data-r2/expr-ops/manifest#multiply-numbers-cast",
        reason: XfailReason::RepresentationDifference,
    },
    Xfail {
        iri_tail: "data-r2/expr-ops/manifest#divide-numbers-cast",
        reason: XfailReason::RepresentationDifference,
    },
    Xfail {
        iri_tail: "data-r2/expr-ops/manifest#unplus-2",
        reason: XfailReason::RepresentationDifference,
    },
    Xfail {
        iri_tail: "data-r2/expr-ops/manifest#unminus-2",
        reason: XfailReason::RepresentationDifference,
    },
    // === Full W3C sparql11 query-eval groups (commit 426c7df) ===============
    //
    // Every case below is a real gap the full vendored suite exposes; the
    // curated subset simply never exercised it. Grouped by root cause. Suffixes
    // are group-qualified (`<group>/manifest#<name>`) so they cannot cross-match.

    // No query-eval case is ledgered. The five that once were (`cast-decimal`,
    // `cast-double`, `cast-float`, `coalesce01`, `plus-1-corrected`) and the five
    // aggregates cases with the same cause failed only because the vendored
    // expected results spell a computed number inconsistently: "1050" beside
    // "2.5E0" for `xsd:double`, "2.0" beside "2" for an integer-valued
    // `xsd:decimal`. Their values and datatypes were always right. The result
    // comparer now compares two numeric literals of the same datatype by value
    // (`compare::comparison_lexical`), and all ten pass.

    // === Full W3C sparql11 UPDATE-eval groups (commit 426c7df) ===============
    //
    // The update groups (add/basic-update/clear/copy/delete*/drop/move/
    // update-silent) run through the UpdateEval harness path and all pass
    // natively (including per-operation blank-node scoping, compared by
    // RDFC-1.0 canonical N-Quads).

    // === W3C sparql11 entailment-regime group (commit 426c7df) ================
    //
    // The native reasoner (purrdf-entail) materializes RDF/RDFS + OWL-RL-shaped
    // closure for the forward-materializable regimes, answers the OWL-Direct
    // regime query-directed via `purrdf_entail::materialize_dl_reported` (a SHOIQ(D) tableau
    // over the query's class expressions), and forward-chains the RIF-Core Horn
    // rule sets via `purrdf_entail::materialize_rif` (the RIF-in-XML documents the
    // `qt:data` graphs reference, plus their RDF imports). So every
    // rdf*/rdfs*/lang/plainLit/bind* case, every `parent*`/`simple*`/`owlds*`, the
    // full `sparqldl-*` / `paper-sparqldl-Q*` OWL-DL query-answering set, and all
    // four `rif*` RIF-entailment cases now pass — this group has no residual
    // `Entailment` failures.
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
///
/// # Errors
///
/// Returns a message when MORE THAN ONE registry entry matches `case_iri`. An
/// ambiguous ledger entry is a bug, not a preference: whichever entry the lookup
/// happened to return would decide the case's typed reason (and therefore which
/// category the matrix reports the gap under) by registry order alone, and the
/// other entry would be silently inert. Refusing is the only reading that cannot
/// hide one of the two.
pub fn lookup(case_iri: &str) -> Result<Option<XfailReason>, String> {
    let mut hits = XFAIL.iter().filter(|x| matches(case_iri, x.iri_tail));
    let Some(first) = hits.next() else {
        return Ok(None);
    };
    let rest: Vec<&str> = hits.map(|x| x.iri_tail).collect();
    if rest.is_empty() {
        return Ok(Some(first.reason));
    }
    Err(format!(
        "case {case_iri} is matched by {} xfail entries ({}, {}). One case must have exactly one \
         typed reason; the extra entries would be silently inert and the reported gap category \
         would depend on registry order",
        rest.len() + 1,
        first.iri_tail,
        rest.join(", ")
    ))
}
