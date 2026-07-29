// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Entailment-**regime** materialization for the wasm/JS surface.
//!
//! # Not to be confused with [`crate::shacl`]
//!
//! The two modules sit beside each other and both say "entail":
//!
//! * [`crate::shacl`]'s `shaclEntail(shapesTtl, dataNt)` is **SHACL-AF
//!   `sh:rule`** entailment. It needs a *shapes* graph and applies the rules that
//!   graph declares.
//! * This module's `entailMaterialize(document, regime)` is **SPARQL
//!   entailment-regime** materialization (`Simple` / `RDF` / `RDFS` / `OWL-RL`).
//!   It takes no shapes at all: it closes a document under the regime's own
//!   specification rule table.
//!
//! # One boundary, three hosts
//!
//! Nothing here reimplements the parse → close → serialize sequence. Every entry
//! point routes through [`purrdf_validate::regime`], the same string boundary the
//! C-ABI and PyO3 hosts call, so a byte difference between the three hosts is one
//! shared golden vector failing rather than three surfaces that quietly stopped
//! agreeing. This module converts JS values to plain Rust `&str`, calls that
//! boundary, and maps its `Result<_, String>` onto a thrown `JsError`. That is
//! its whole job.
//!
//! # Why the report is not optional
//!
//! `materialize_impl` returns the closure **and** the rendered reasoning report,
//! never one without the other. All seventy-eight OWL 2 RL rules now run, so under
//! `owl-rl` the report's load-bearing lines are the `boundary` ones: a binding that
//! answered "OWL-RL entailment" without saying which CONSTRUCTS the run could not
//! fully handle would be making exactly the overclaim the report exists to prevent,
//! because a complete rule table is not a complete closure.
//! [`purrdf_validate::regime`] documents the discipline in full, and the rendering
//! is byte-stable by construction so the string survives this host boundary
//! unchanged.

use purrdf_validate::regime::{
    RegimeClosure as BoundaryClosure, check_regime_golden_vectors, implemented_rules_string,
    materialize_to_nquads_string, rules_string,
};
use wasm_bindgen::prelude::*;

/// One closure of one document under one regime: the canonical N-Quads and the
/// rendered reasoning report.
///
/// A class with two named getters rather than a positional pair, because a
/// two-string tuple is the wrong thing for a JS caller to get backwards.
#[wasm_bindgen]
#[derive(Debug)]
pub struct RegimeClosure {
    /// The materialized dataset as canonical (RDFC-1.0) N-Quads.
    nquads: String,
    /// The run's reasoning report, in the boundary's byte-stable rendering.
    report: String,
}

#[wasm_bindgen]
impl RegimeClosure {
    /// The materialized dataset — every input quad plus every inferred triple —
    /// as canonical (RDFC-1.0) N-Quads.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn nquads(&self) -> String {
        self.nquads.clone()
    }

    /// What the run actually did: which rules fired and how often, which
    /// specification rules did NOT fire, which constructs were left at a
    /// boundary, the evaluation budget, and the calculus's contract hash.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn report(&self) -> String {
        self.report.clone()
    }
}

impl From<BoundaryClosure> for RegimeClosure {
    fn from(closure: BoundaryClosure) -> Self {
        let (nquads, report) = closure.into_parts();
        Self { nquads, report }
    }
}

/// Close `document` under the regime spelled `regime`.
///
/// Returns a plain `String` error (NOT a `JsError`) so it is unit-testable on the
/// native build — constructing a `JsError` calls a wasm-only import that panics
/// off wasm. The `#[wasm_bindgen]` wrapper maps the `String` to a `JsError`.
pub(crate) fn materialize_impl(document: &str, regime: &str) -> Result<RegimeClosure, String> {
    materialize_to_nquads_string(regime, document).map(RegimeClosure::from)
}

/// `entailMaterialize(document, regime)` → a `RegimeClosure` carrying the
/// canonical N-Quads closure and the rendered reasoning report.
///
/// `document` is parsed as N-Quads, which accepts an N-Triples document
/// unchanged, so a document that names a graph keeps naming it. `regime` is one
/// of `simple`, `rdf`, `rdfs`, `owl-rl`, `owl-direct`, `rif`, `d` — the same
/// spellings the CLI, the C ABI and the Python surface accept.
///
/// Throws if `document` fails to parse, if `regime` is not one of those
/// spellings, or if `regime` is one of the two that cannot be forward
/// materialized (`owl-direct`, `rif`); the message names the accepted set in
/// every case. `d` is not one of them — datatype entailment materializes, as the
/// five `dt-*` rules of OWL 2 Profiles §4.3 Table 8.
#[wasm_bindgen(js_name = entailMaterialize)]
pub fn entail_materialize(document: &str, regime: &str) -> Result<RegimeClosure, JsError> {
    materialize_impl(document, regime).map_err(|e| JsError::new(&e))
}

/// The rule table `regime` is *defined by*. See [`entail_rules`].
pub(crate) fn rules_impl(regime: &str) -> Result<Vec<String>, String> {
    Ok(rules_string(regime)?.lines().map(str::to_owned).collect())
}

/// `entailRules(regime)` → the rule table the specification *defines* the regime
/// by, one canonical rule name per array entry, in specification table order.
///
/// `[]` for a regime with no rule table (`simple`, and the two that are not
/// forward-materializable). `owl-rl` returns all 78 rules of OWL 2 Profiles §4.3
/// Tables 4–9 whether or not this build fires them — that is the point: compare
/// it with [`entail_implemented_rules`] to measure the gap.
///
/// Throws for an unknown regime spelling, naming the accepted set.
#[wasm_bindgen(js_name = entailRules)]
pub fn entail_rules(regime: &str) -> Result<Vec<String>, JsError> {
    rules_impl(regime).map_err(|e| JsError::new(&e))
}

/// The subset of the rule table this build fires. See [`entail_implemented_rules`].
pub(crate) fn implemented_rules_impl(regime: &str) -> Result<Vec<String>, String> {
    Ok(implemented_rules_string(regime)?
        .lines()
        .map(str::to_owned)
        .collect())
}

/// `entailImplementedRules(regime)` → the subset of `entailRules(regime)` this
/// build's chase actually fires today.
///
/// `entailRules(r)` minus `entailImplementedRules(r)` is the regime's measurable
/// gap — the same set the rendered report's `missing` lines name.
///
/// Throws for an unknown regime spelling, naming the accepted set.
#[wasm_bindgen(js_name = entailImplementedRules)]
pub fn entail_implemented_rules(regime: &str) -> Result<Vec<String>, JsError> {
    implemented_rules_impl(regime).map_err(|e| JsError::new(&e))
}

/// `entailCheckGoldenVectors()` — run the committed tri-host golden vector
/// artifact through this build and throw on the first byte that differs.
///
/// This is the wasm half of the load-bearing assertion. The artifact
/// (`crates/validate/tests/fixtures/regime-boundary.vectors`) is compiled into
/// the module by `purrdf-validate`, and the C-ABI crate's Rust test, the
/// `purrdf-validate` Rust test and this entry point all call the SAME checker
/// over the SAME bytes. Running it here executes the entailment chase, the
/// RDFC-1.0 canonical serializer and the report renderer on `wasm32` — different
/// pointer width, different `usize`, different map iteration — and compares the
/// result byte for byte against what the native build produced. A host that
/// diverges fails here in the same words it fails natively.
///
/// It ships in the released module deliberately: a consumer can prove the wasm
/// they actually loaded agrees with the reference implementation, without
/// trusting this repository's CI.
///
/// Throws with the case name and a diff of the two strings on any mismatch.
#[wasm_bindgen(js_name = entailCheckGoldenVectors)]
pub fn entail_check_golden_vectors() -> Result<(), JsError> {
    check_regime_golden_vectors().map_err(|e| JsError::new(&e))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `A ⊑ B` and one typed instance — enough for `rdfs9` to re-type it.
    const SCHEMA: &str = "\
<http://example.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/B> .
<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .
";

    /// The native half of the tri-host assertion, made from THIS crate.
    ///
    /// The wasm half is `entailCheckGoldenVectors`, called as real wasm by
    /// `js/tests/entail.test.mjs`; both run the same checker over the same
    /// committed artifact, so a native/wasm divergence is one failing case.
    #[test]
    fn the_golden_vector_matches_natively() {
        check_regime_golden_vectors().expect("the regime golden vector");
    }

    #[test]
    fn materialize_infers_and_reports() {
        let closed = materialize_impl(SCHEMA, "rdfs").expect("rdfs closure");
        assert!(closed.nquads().contains(
            "<http://example.org/x> \
             <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/B> ."
        ));
        // The base fact survives into the materialized dataset.
        assert!(closed.nquads().contains(
            "<http://example.org/x> \
             <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> ."
        ));
        // …and the report is never optional, and says what the run could NOT do.
        // Asserted as the invariant rather than as a `sound-incomplete <n>`
        // literal: the count moves every time a rule lands, the honesty gate does
        // not, and a `boundary` line outlives a rule table going complete.
        assert!(closed.report().starts_with("purrdf-reasoning-report 1\n"));
        assert!(closed.report().contains("\nregime rdfs\n"));
        assert!(closed.report().contains("\ncompleteness "));
        assert!(closed.report().contains("\nboundary "));
        assert!(closed.report().ends_with("overclaims false\n"));
    }

    #[test]
    fn an_unknown_regime_names_the_accepted_set() {
        for error in [
            materialize_impl(SCHEMA, "RDFS").expect_err("case-sensitive"),
            rules_impl("rdfs-plus").expect_err("unknown"),
            implemented_rules_impl("rdfs-plus").expect_err("unknown"),
        ] {
            assert!(error.contains("accepted: simple, rdf, rdfs"), "{error}");
        }
    }

    #[test]
    fn a_non_materializable_regime_is_refused_by_name() {
        for regime in ["owl-direct", "rif"] {
            let error = materialize_impl(SCHEMA, regime).expect_err("unsupported");
            assert!(
                error.contains("materializable regimes: simple, rdf, rdfs, owl-rl, d"),
                "{error}"
            );
        }
        // `d` is on the other side of that line: it materializes here as it does
        // on the Rust, C-ABI and Python hosts.
        assert!(materialize_impl(SCHEMA, "d").is_ok());
    }

    #[test]
    fn a_malformed_document_is_an_error() {
        assert!(materialize_impl("this is not n-quads\n", "rdfs").is_err());
    }

    #[test]
    fn the_inventories_are_the_specification_tables() {
        // The SPECIFICATION's counts; these do not move.
        assert_eq!(rules_impl("owl-rl").expect("known").len(), 78);
        let defined = rules_impl("rdfs").expect("known");
        assert_eq!(defined.len(), 18);
        // The implemented set is a subsequence of it, and the gap is MEASURED —
        // never a literal, which would go stale the day a rule lands, and which
        // may legitimately be zero once the table is complete.
        let implemented = implemented_rules_impl("rdfs").expect("known");
        let missing = defined
            .iter()
            .filter(|rule| !implemented.contains(rule))
            .count();
        assert_eq!(missing, defined.len() - implemented.len());
        // An empty table is an empty array, not a one-element array of "".
        assert!(rules_impl("simple").expect("known").is_empty());
        assert!(implemented_rules_impl("simple").expect("known").is_empty());
    }
}
