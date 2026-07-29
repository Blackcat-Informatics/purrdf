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
    ReasoningAnswer as BoundaryAnswer, RegimeClosure as BoundaryClosure,
    check_inconsistent_refusal, check_regime_golden_vectors, classify_to_string,
    consistency_to_string, entails_to_string, explain_conclusion_to_string, extension_rules_string,
    extract_module_to_string, implemented_rules_string, instances_to_string, justify_to_string,
    materialize_to_nquads_string, profile_to_string, realize_to_string, rules_string,
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
pub(crate) fn materialize_impl(
    document: &str,
    regime: &str,
    program: &str,
) -> Result<RegimeClosure, String> {
    materialize_to_nquads_string(regime, document, program).map(RegimeClosure::from)
}

/// `entailMaterialize(document, regime, program)` → a `RegimeClosure` carrying the
/// canonical N-Quads closure and the rendered reasoning report.
///
/// `document` is parsed as N-Quads, which accepts an N-Triples document
/// unchanged, so a document that names a graph keeps naming it. `regime` is one
/// of `simple`, `rdf`, `rdfs`, `owl-rl`, `owl-direct`, `rif`, `d` — the same
/// spellings the CLI, the C ABI and the Python surface accept — and ALL SEVEN
/// materialize.
///
/// `program` is the regime's own rule document. `rif` entails under the CALLER's
/// rules, so for that spelling `program` is a normative RIF-in-XML document; every
/// other regime's rule table is the specification's, so its `program` is `""`, and a
/// non-empty one throws rather than being silently discarded.
///
/// Throws if `document` fails to parse, if `regime` is not one of those spellings
/// (the message names the accepted set), or if `program` is wrong for the regime.
#[wasm_bindgen(js_name = entailMaterialize)]
pub fn entail_materialize(
    document: &str,
    regime: &str,
    program: &str,
) -> Result<RegimeClosure, JsError> {
    materialize_impl(document, regime, program).map_err(|e| JsError::new(&e))
}

/// The rule table `regime` is *defined by*. See [`entail_rules`].
pub(crate) fn rules_impl(regime: &str) -> Result<Vec<String>, String> {
    Ok(rules_string(regime)?.lines().map(str::to_owned).collect())
}

/// `entailRules(regime)` → the rule table the specification *defines* the regime
/// by, one canonical rule name per array entry, in specification table order.
///
/// `[]` for a regime with no rule table of its own (`simple`, plus `owl-direct`, which
/// decides through the tableau, and `rif`, which entails under the caller's rules —
/// all three still MATERIALIZE). `owl-rl` returns all 78 rules of OWL 2 Profiles §4.3
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

/// The rules this build fires beyond the specification table. See [`entail_extensions`].
pub(crate) fn extensions_impl(regime: &str) -> Result<Vec<String>, String> {
    Ok(extension_rules_string(regime)?
        .lines()
        .map(str::to_owned)
        .collect())
}

/// `entailExtensions(regime)` → the rules this build fires BEYOND `regime`'s
/// specification table, one canonical rule name per array entry.
///
/// Disjoint from both `entailRules(regime)` and `entailImplementedRules(regime)`:
/// the normative table is a statement about the specification and does not move
/// because this build fires a sound rule the table happens not to list. A rendered
/// report names the same rules on its `extension` line — this answers the question
/// without materializing a dataset first.
///
/// `[]` for a lane with nothing added to it, which is every lane but `owl-rl`.
///
/// Throws for an unknown regime spelling, naming the accepted set.
#[wasm_bindgen(js_name = entailExtensions)]
pub fn entail_extensions(regime: &str) -> Result<Vec<String>, JsError> {
    extensions_impl(regime).map_err(|e| JsError::new(&e))
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

/// `entailCheckInconsistentRefusal()` — prove that an INCONSISTENT input is refused
/// with its certificate and its witness triples, and throw if it is not.
///
/// The companion to [`entail_check_golden_vectors`], and the path that artifact cannot
/// cover: an inconsistent knowledge base has no closure, so there is nothing to pair an
/// input with, and the only channel the evidence has is the thrown error's message. That
/// makes it exactly the path on which a host quietly drops the report — as every host but
/// the command line once did, leaving `inconsistency` a constant `none`.
///
/// The C-ABI crate's Rust test and the `purrdf-validate` Rust test call the SAME checker.
///
/// Throws naming the fragment the refusal failed to carry, with the refusal verbatim.
#[wasm_bindgen(js_name = entailCheckInconsistentRefusal)]
pub fn entail_check_inconsistent_refusal() -> Result<(), JsError> {
    check_inconsistent_refusal().map_err(|e| JsError::new(&e))
}

// ── The Description-Logic reasoning services ────────────────────────────────

/// One reasoning service's answer and the certificate of the run that produced it.
///
/// A class with two named getters rather than a positional pair, exactly as
/// [`RegimeClosure`] is and for the same reason: both strings cross the host
/// boundary and a two-string tuple is the wrong thing for a JS caller to get
/// backwards. There is no certificate-free getter — a caller that ignores the
/// evidence must at least have been handed it.
#[wasm_bindgen]
#[derive(Debug)]
pub struct ReasoningAnswer {
    /// The service's answer, in the boundary's line-oriented rendering.
    answer: String,
    /// The certificate of the run that produced it.
    certificate: String,
}

#[wasm_bindgen]
impl ReasoningAnswer {
    /// The service's answer.
    ///
    /// Line-oriented and byte-stable. A dataset-valued answer
    /// (`entailExtractModule`, `entailJustify`) is canonical (RDFC-1.0) N-Quads
    /// instead.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn answer(&self) -> String {
        self.answer.clone()
    }

    /// How completely the service decided: the `purrdf-dl-certificate 1` block for
    /// a tableau service, or that service's own certificate grammar for the three
    /// that run no tableau (`entailProfile`, `entailExtractModule`,
    /// `entailExplainConclusion`).
    ///
    /// The DL lane's completeness is `decided` / `decided-within-boundaries` /
    /// `budget-exhausted` — NOT the chase's `exact` / `sound-incomplete`, which is
    /// a difference of two rule tables and would report "exact" for a tableau that
    /// ran out of budget.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn certificate(&self) -> String {
        self.certificate.clone()
    }
}

impl From<BoundaryAnswer> for ReasoningAnswer {
    fn from(produced: BoundaryAnswer) -> Self {
        let (answer, certificate) = produced.into_parts();
        Self {
            answer,
            certificate,
        }
    }
}

/// Decide whether `document`'s ontology has a model at all. See
/// [`entail_consistency`].
pub(crate) fn consistency_impl(document: &str, step_cap: u32) -> Result<ReasoningAnswer, String> {
    consistency_to_string(document, step_cap).map(ReasoningAnswer::from)
}

/// `entailConsistency(document, stepCap)` → is the knowledge base consistent?
///
/// `stepCap` narrows the per-decision tableau step cap; **0 means the knowledge
/// base's own cap**, not a cap of zero steps. It can only NARROW, so it cannot be
/// used to make a hard instance answerable — only to make the `budget-exhausted`
/// certificate reachable from a test.
///
/// The one DL service that answers for an unsatisfiable ontology, because it is the
/// one that detects one; every other throws rather than returning the vacuous
/// answer an ontology with no model gives.
///
/// Throws if `document` fails to parse or the reverse mapping fails.
#[wasm_bindgen(js_name = entailConsistency)]
pub fn entail_consistency(document: &str, step_cap: u32) -> Result<ReasoningAnswer, JsError> {
    consistency_impl(document, step_cap).map_err(|e| JsError::new(&e))
}

/// The subsumption hierarchy over the named classes. See [`entail_classify`].
pub(crate) fn classify_impl(document: &str, step_cap: u32) -> Result<ReasoningAnswer, String> {
    classify_to_string(document, step_cap).map(ReasoningAnswer::from)
}

/// `entailClassify(document, stepCap)` → the entailed subsumption hierarchy over
/// the ontology's named classes.
///
/// The answer carries `equivalent`, `subclass` (the full transitively-closed
/// relation), `direct` (its transitive reduction) and `unsatisfiable` lines, in
/// that block order. Both subsumption blocks are emitted because they are different
/// facts: `direct` is "direct as far as this run decided", which weakens under a
/// `budget-exhausted` certificate while every listed pair stays a genuine
/// subsumption.
///
/// Throws on a malformed document, or on an ontology with no model — every class
/// then subsumes every other and the hierarchy would be a complete graph.
#[wasm_bindgen(js_name = entailClassify)]
pub fn entail_classify(document: &str, step_cap: u32) -> Result<ReasoningAnswer, JsError> {
    classify_impl(document, step_cap).map_err(|e| JsError::new(&e))
}

/// The entailed types of the named individuals. See [`entail_realize`].
pub(crate) fn realize_impl(document: &str, step_cap: u32) -> Result<ReasoningAnswer, String> {
    realize_to_string(document, step_cap).map(ReasoningAnswer::from)
}

/// `entailRealize(document, stepCap)` → the entailed types of the ontology's named
/// individuals, and the most specific of them (`type` then `direct-type` lines).
///
/// Throws on a malformed document or an ontology with no model.
#[wasm_bindgen(js_name = entailRealize)]
pub fn entail_realize(document: &str, step_cap: u32) -> Result<ReasoningAnswer, JsError> {
    realize_impl(document, step_cap).map_err(|e| JsError::new(&e))
}

/// Instance retrieval for one named class. See [`entail_instances`].
pub(crate) fn instances_impl(
    document: &str,
    class: &str,
    step_cap: u32,
) -> Result<ReasoningAnswer, String> {
    instances_to_string(document, class, step_cap).map(ReasoningAnswer::from)
}

/// `entailInstances(document, class, stepCap)` → the named individuals entailed to
/// be instances of `class`, as `instance <term>` lines.
///
/// `class` is ONE N-Triples term — `"<http://example.org/Cat>"`, angle brackets
/// included. A class the ontology never mentions is not an error: nothing
/// constrains it, so the empty answer for it is a real answer.
///
/// Throws on a malformed document, a `class` that is not one N-Triples term, or an
/// ontology with no model.
#[wasm_bindgen(js_name = entailInstances)]
pub fn entail_instances(
    document: &str,
    class: &str,
    step_cap: u32,
) -> Result<ReasoningAnswer, JsError> {
    instances_impl(document, class, step_cap).map_err(|e| JsError::new(&e))
}

/// Axiom entailment by refutation. See [`entail_entails`].
pub(crate) fn entails_impl(
    document: &str,
    axiom: &str,
    step_cap: u32,
) -> Result<ReasoningAnswer, String> {
    entails_to_string(document, axiom, step_cap).map(ReasoningAnswer::from)
}

/// `entailEntails(document, axiom, stepCap)` → does the ontology entail `axiom`?
///
/// `axiom` is ONE triple of the OWL 2 RDF mapping, in N-Triples syntax. Seven
/// reserved predicates select the seven named axiom kinds — `rdfs:subClassOf`,
/// `owl:equivalentClass`, `owl:disjointWith`, `rdf:type`, `owl:sameAs`,
/// `owl:differentFrom`, `rdfs:subPropertyOf` — and any other predicate is an
/// object-property assertion. No encoding is invented: this is the mapping the
/// reasoner's own reverse mapping reads.
///
/// The answer is `entails true|false|unknown` followed by the axiom as it was READ,
/// so a caller can see which axiom its predicate selected. `unknown` is never
/// collapsed to `false`: it means the search reached its step cap, which the
/// certificate says in its own words.
///
/// Throws on a malformed document, an `axiom` that is not one triple, or an
/// ontology with no model.
#[wasm_bindgen(js_name = entailEntails)]
pub fn entail_entails(
    document: &str,
    axiom: &str,
    step_cap: u32,
) -> Result<ReasoningAnswer, JsError> {
    entails_impl(document, axiom, step_cap).map_err(|e| JsError::new(&e))
}

/// OWL 2 profile certification. See [`entail_profile`].
pub(crate) fn profile_impl(document: &str) -> Result<ReasoningAnswer, String> {
    profile_to_string(document).map(ReasoningAnswer::from)
}

/// `entailProfile(document)` → which OWL 2 profiles the ontology is provably in,
/// and what blocked the others.
///
/// The answer is `certified <profile>` lines, most restrictive first (EL, QL, RL,
/// DL, Full). Purely syntactic — no tableau, no closure, no budget — so the
/// certificate is a `purrdf-owl-profile-certificate 1` block rather than a DL one:
/// there is no search whose completeness could be reported, and rendering a
/// fabricated `decided` would be exactly the overclaim the certificates prevent.
///
/// The certificate ends `one-directional true`: a certification PROVES membership,
/// a violation does NOT prove non-membership.
///
/// Throws on a malformed document.
#[wasm_bindgen(js_name = entailProfile)]
pub fn entail_profile(document: &str) -> Result<ReasoningAnswer, JsError> {
    profile_impl(document).map_err(|e| JsError::new(&e))
}

/// Locality-based module extraction. See [`entail_extract_module`].
pub(crate) fn extract_module_impl(
    document: &str,
    signature: &str,
    method: &str,
) -> Result<ReasoningAnswer, String> {
    extract_module_to_string(document, signature, method).map(ReasoningAnswer::from)
}

/// `entailExtractModule(document, signature, method)` → the locality module of the
/// ontology for a seed signature.
///
/// `signature` is one N-Triples term per line (blank lines ignored). `method` is
/// `bot`, `top` or `star`; an unknown spelling throws with the accepted set named.
///
/// The answer is the module as canonical (RDFC-1.0) N-Quads — the same serializer
/// `entailMaterialize` uses. The certificate's `conservative` line says whether the
/// module is the minimal one or a sound SUPERSET, which is what a caller sizing a
/// module needs to know.
///
/// Throws on a malformed document, a signature line that is not one N-Triples term,
/// an unknown method, or a module that cannot be frozen.
#[wasm_bindgen(js_name = entailExtractModule)]
pub fn entail_extract_module(
    document: &str,
    signature: &str,
    method: &str,
) -> Result<ReasoningAnswer, JsError> {
    extract_module_impl(document, signature, method).map_err(|e| JsError::new(&e))
}

/// A minimal entailing subset for a DL axiom. See [`entail_justify`].
pub(crate) fn justify_impl(document: &str, axiom: &str) -> Result<ReasoningAnswer, String> {
    justify_to_string(document, axiom).map(ReasoningAnswer::from)
}

/// `entailJustify(document, axiom)` → WHY a Description-Logic axiom is entailed: a
/// minimal subset of the ontology that still entails it.
///
/// A tableau performs no derivation steps, so this is a JUSTIFICATION and
/// deliberately not called a proof; `entailExplainConclusion` is the chase lane's
/// genuinely derivational explanation, and the two are different kinds of thing
/// rather than two spellings of one.
///
/// The answer is the justification's axioms as canonical N-Quads. The certificate's
/// `sufficient` and `minimal` lines are **re-decided** over the justification alone
/// and over each of its one-axiom-smaller subsets, so they check the answer rather
/// than restate it.
///
/// Throws if the ontology does not entail the axiom — the empty set reads as
/// "nothing is needed" and means the opposite — or if the tableau could not decide
/// it, leaving no answer to shrink against.
#[wasm_bindgen(js_name = entailJustify)]
pub fn entail_justify(document: &str, axiom: &str) -> Result<ReasoningAnswer, JsError> {
    justify_impl(document, axiom).map_err(|e| JsError::new(&e))
}

/// A checked derivation for one chase conclusion. See [`entail_explain_conclusion`].
pub(crate) fn explain_conclusion_impl(
    document: &str,
    regime: &str,
    conclusion: &str,
) -> Result<ReasoningAnswer, String> {
    explain_conclusion_to_string(document, regime, conclusion).map(ReasoningAnswer::from)
}

/// `entailExplainConclusion(document, regime, conclusion)` → WHY one triple of a
/// chase closure holds: which rules, from which premises.
///
/// `conclusion` is ONE N-Quads statement; its graph, if it names one, selects the
/// closure to explain.
///
/// The certificate's `derived-*` lines are what the CHECKER re-derived from the
/// proof term and the clause program — not what the proof claims — so a proof whose
/// stated conclusion its own premises do not license shows up as differing lines
/// rather than a silent `checked true`.
///
/// Throws for `rdf` and `rdfs`, four of whose rules conclude about a FRESH blank
/// node: an existential head has no Datalog semantics, so a "proof" of such a step
/// could only be believed. Also throws for a conclusion that is neither asserted nor
/// derived — a hard error, because there is nothing to explain and an empty answer
/// would read as though there were.
#[wasm_bindgen(js_name = entailExplainConclusion)]
pub fn entail_explain_conclusion(
    document: &str,
    regime: &str,
    conclusion: &str,
) -> Result<ReasoningAnswer, JsError> {
    explain_conclusion_impl(document, regime, conclusion).map_err(|e| JsError::new(&e))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `A ⊑ B` and one typed instance — enough for `rdfs9` to re-type it.
    const SCHEMA: &str = "\
<http://example.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/B> .
<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .
";

    /// A normative RIF-in-XML rule document: `?x a ex:A` ⟹ `?x a ex:B`.
    ///
    /// `rif` is the one regime whose calculus is the CALLER's, so it is the one
    /// spelling whose `program` argument is a document rather than the empty string.
    const RIF_PROGRAM: &str = "<Document xmlns=\"http://www.w3.org/2007/rif#\"><payload><Group><sentence><Forall><declare><Var>x</Var></declare><formula><Implies><if><Frame><object><Var>x</Var></object><slot><Const type=\"http://www.w3.org/2007/rif#iri\">http://www.w3.org/1999/02/22-rdf-syntax-ns#type</Const><Const type=\"http://www.w3.org/2007/rif#iri\">http://example.org/A</Const></slot></Frame></if><then><Frame><object><Var>x</Var></object><slot><Const type=\"http://www.w3.org/2007/rif#iri\">http://www.w3.org/1999/02/22-rdf-syntax-ns#type</Const><Const type=\"http://www.w3.org/2007/rif#iri\">http://example.org/B</Const></slot></Frame></then></Implies></formula></Forall></sentence></Group></payload></Document>";

    /// The native half of the tri-host assertion, made from THIS crate.
    ///
    /// The wasm half is `entailCheckGoldenVectors`, called as real wasm by
    /// `js/tests/entail.test.mjs`; both run the same checker over the same
    /// committed artifact, so a native/wasm divergence is one failing case.
    #[test]
    fn the_golden_vector_matches_natively() {
        check_regime_golden_vectors().expect("the regime golden vector");
    }

    /// The other tri-host assertion, made from THIS crate: an inconsistent input is
    /// refused WITH its certificate and its witness triples.
    ///
    /// The wasm half is `entailCheckInconsistentRefusal`. A refusal has no closure, so it
    /// cannot be a golden-vector case — and it is exactly the path on which a host is most
    /// likely to drop the evidence, because the only channel it has is the error string.
    #[test]
    fn an_inconsistent_input_is_refused_with_its_report_natively() {
        check_inconsistent_refusal().expect("the inconsistent refusal");
    }

    #[test]
    fn materialize_infers_and_reports() {
        let closed = materialize_impl(SCHEMA, "rdfs", "").expect("rdfs closure");
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
        assert!(closed.report().starts_with("purrdf-reasoning-report 3\n"));
        assert!(closed.report().contains("\nregime rdfs\n"));
        assert!(closed.report().contains("\ncompleteness "));
        assert!(closed.report().contains("\nboundary "));
        // The four existential rules' only observable, and it reaches WASM now.
        assert!(closed.report().contains("\nwithheld-surrogates "));
        assert!(closed.report().ends_with("inconsistency none\n"));
    }

    #[test]
    fn an_unknown_regime_names_the_accepted_set() {
        for error in [
            materialize_impl(SCHEMA, "RDFS", "").expect_err("case-sensitive"),
            rules_impl("rdfs-plus").expect_err("unknown"),
            implemented_rules_impl("rdfs-plus").expect_err("unknown"),
        ] {
            assert!(error.contains("accepted: simple, rdf, rdfs"), "{error}");
        }
    }

    /// EVERY accepted spelling materializes on this host too. None is refused.
    ///
    /// Falsifiable against the old behavior: `owl-direct` and `rif` were refused here
    /// with a message naming the five that were not.
    #[test]
    fn every_regime_spelling_materializes() {
        for (regime, program) in [
            ("simple", ""),
            ("rdf", ""),
            ("rdfs", ""),
            ("owl-rl", ""),
            ("owl-direct", ""),
            ("rif", RIF_PROGRAM),
            ("d", ""),
        ] {
            let closed = materialize_impl(SCHEMA, regime, program)
                .unwrap_or_else(|error| panic!("{regime}: {error}"));
            assert!(
                closed.report().contains(&format!("\nregime {regime}\n")),
                "{}",
                closed.report()
            );
        }
        // A rule document belongs to exactly one regime; passing one anywhere else is
        // refused rather than discarded.
        let error = materialize_impl(SCHEMA, "rdfs", RIF_PROGRAM)
            .expect_err("a rule document for a rule-table regime");
        assert!(error.contains("takes no rule document"), "{error}");
    }

    #[test]
    fn a_malformed_document_is_an_error() {
        assert!(materialize_impl("this is not n-quads\n", "rdfs", "").is_err());
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

    // ── The Description-Logic reasoning services ────────────────────────────

    /// `A ⊑ B ⊑ C`, `D ⊑ C`, and one instance of `A`.
    const TAXONOMY: &str = "\
<http://example.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/B> .
<http://example.org/B> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .
<http://example.org/D> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .
<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .
";

    /// `A ⊑ C` — entailed by the chain, asserted nowhere.
    const CHAIN_AXIOM: &str = "<http://example.org/A> \
<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .\n";

    /// Every DL service reaches this host, and every one carries a certificate.
    ///
    /// The load-bearing assertion for this crate's half of the surface: a service
    /// exported without its certificate is the defect, and this test would pass
    /// vacuously only if the service were missing entirely — which the list makes
    /// a compile error rather than an omission.
    ///
    /// The tableau services (`consistency`, `classify`, `realize`, `instances`,
    /// `entails`) have no trailing gate LITERAL: their `purrdf-dl-certificate 1`
    /// block derives `completeness` from `boundary` on every render, so this test
    /// exercises that derivation directly rather than matching a constant that
    /// could only ever read `false`.
    #[test]
    fn every_dl_service_reaches_this_host_with_its_certificate() {
        let services = [
            ("consistency", consistency_impl(TAXONOMY, 0)),
            ("classify", classify_impl(TAXONOMY, 0)),
            ("realize", realize_impl(TAXONOMY, 0)),
            (
                "instances",
                instances_impl(TAXONOMY, "<http://example.org/C>", 0),
            ),
            ("entails", entails_impl(TAXONOMY, CHAIN_AXIOM, 0)),
            ("profile", profile_impl(TAXONOMY)),
            (
                "extract-module",
                extract_module_impl(TAXONOMY, "<http://example.org/A>\n", "star"),
            ),
            ("justify", justify_impl(TAXONOMY, CHAIN_AXIOM)),
            (
                "explain-conclusion",
                explain_conclusion_impl(
                    TAXONOMY,
                    "owl-rl",
                    "<http://example.org/x> \
                     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
                     <http://example.org/C> .\n",
                ),
            ),
        ];
        assert_eq!(services.len(), 9);
        for (service, produced) in services {
            let produced = produced.unwrap_or_else(|error| panic!("{service}: {error}"));
            let certificate = produced.certificate();
            assert!(
                certificate.contains(&format!("\nservice {service}\n")),
                "{service}: {certificate}"
            );
            if certificate.starts_with("purrdf-dl-certificate 1\n") {
                let completeness = certificate
                    .lines()
                    .find_map(|line| line.strip_prefix("completeness "))
                    .unwrap_or_else(|| panic!("{service}: no completeness line: {certificate}"));
                let has_boundaries = certificate
                    .lines()
                    .any(|line| line.starts_with("boundary "));
                match completeness {
                    "decided" => assert!(!has_boundaries, "{service}: {certificate}"),
                    "decided-within-boundaries" => {
                        assert!(has_boundaries, "{service}: {certificate}");
                    }
                    "budget-exhausted" => {}
                    other => panic!("{service}: unknown completeness {other}"),
                }
            } else {
                let gate = certificate.lines().last().unwrap_or_default();
                assert!(
                    matches!(
                        gate,
                        "minimal true"
                            | "minimal false"
                            | "one-directional true"
                            | "conservative false"
                            | "conservative true"
                            | "checked true"
                            | "checked false"
                    ),
                    "{service}: {gate}"
                );
            }
        }
    }

    /// The DL certificate's completeness vocabulary is the TABLEAU's, not the
    /// chase's — the distinction the whole certificate exists to make.
    #[test]
    fn the_dl_certificate_is_not_the_chase_report() {
        let tableau = consistency_impl(TAXONOMY, 0).expect("consistency");
        assert_eq!(tableau.answer(), "consistency true\n");
        assert!(
            tableau
                .certificate()
                .starts_with("purrdf-dl-certificate 1\n")
        );
        assert!(tableau.certificate().contains("\ncompleteness decided\n"));
        let chase = materialize_impl(TAXONOMY, "owl-rl", "").expect("owl-rl");
        assert!(chase.report().starts_with("purrdf-reasoning-report 3\n"));
        assert!(!chase.report().contains("completeness decided"));
    }

    /// A starved search answers `unknown`, never `false`, and says so.
    #[test]
    fn an_exhausted_budget_crosses_this_host_intact() {
        let starved = entails_impl(TAXONOMY, CHAIN_AXIOM, 1).expect("decides nothing");
        assert_eq!(starved.answer().lines().next(), Some("entails unknown"));
        assert!(
            starved
                .certificate()
                .contains("\ncompleteness budget-exhausted\n")
        );
        assert!(
            entails_impl(TAXONOMY, CHAIN_AXIOM, 0)
                .expect("decides")
                .answer()
                .starts_with("entails true\n")
        );
    }

    /// Refusals cross this host as messages a JS caller can act on.
    #[test]
    fn dl_refusals_name_what_went_wrong() {
        assert!(consistency_impl("this is not n-quads\n", 0).is_err());
        assert!(instances_impl(TAXONOMY, "not a term", 0).is_err());
        let error = extract_module_impl(TAXONOMY, "", "nested").expect_err("unknown method");
        assert!(error.contains("bot, top, star"), "{error}");
        let error =
            explain_conclusion_impl(TAXONOMY, "rdfs", CHAIN_AXIOM).expect_err("existential");
        assert!(error.contains("existential"), "{error}");
    }
}
