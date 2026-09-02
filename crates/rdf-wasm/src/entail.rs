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
//!   entailment-regime** materialization over all seven regimes (`simple` /
//!   `rdf` / `rdfs` / `owl-rl` / `owl-direct` / `rif` / `d`). It takes no
//!   shapes at all: it closes a document under the regime's own rule table (or,
//!   for `rif`, the caller's).
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
    ReasonerSession, ReasoningAnswer as BoundaryAnswer, RegimeClosure as BoundaryClosure,
    certain_answers_to_string, check_absent_proof_is_not_verifiable, check_dl_proof,
    check_dl_proof_golden_vectors, check_inconsistent_refusal, check_regime_golden_vectors,
    classify_to_string, consistency_to_string, entails_to_string, explain_conclusion_to_string,
    extension_rules_string, extract_module_to_string, graph_entails_to_string,
    implemented_rules_string, instances_to_string, justify_to_string, materialize_to_nquads_string,
    profile_to_string, prove_to_string, realize_to_string, rules_string,
    verify_entailment_to_string,
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
    /// The proof term of the run that produced it, or the `availability not-recorded`
    /// document when nobody asked for one. NEVER empty — see [`Self::proof`].
    proof: String,
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

    /// How completely the service decided, in one of THREE certificate grammars —
    /// which one is a property of the service, so a caller reads it off the call
    /// rather than sniffing the string:
    ///
    /// - `purrdf-dl-certificate 1` — the tableau services (`entailConsistency`,
    ///   `entailClassify`, `entailRealize`, `entailInstances`, `entailEntails`).
    /// - `purrdf-reasoning-report 4` — the entailment-regime services
    ///   (`entailCertainAnswers`, `entailGraphEntails`, `entailVerifyEntailment`),
    ///   which decide by the regime's rule table rather than by a tableau.
    /// - that service's own grammar — the three that run neither (`entailProfile`,
    ///   `entailExtractModule`, `entailExplainConclusion`).
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

    /// The PROOF TERM of the run that produced this answer, as a `purrdf-dl-proof 1`
    /// document — the thing `entailCheckProof` takes.
    ///
    /// Never the empty string, and that is the point. Recording is OPT-IN: an answer from an
    /// `entail*` function or from a `new Reasoner(...)` session records nothing, and this
    /// getter then reads
    ///
    /// ```text
    /// purrdf-dl-proof 1
    /// availability not-recorded
    /// ```
    ///
    /// which says NOTHING WAS MEASURED. A recorded proof says `availability recorded`, and a
    /// recorded proof with `runs 0` in it says something different again: that the service is
    /// syntactic (`extractModule` is), so there was no search to check and the checker
    /// verified exactly that. Three states, three documents, and `entailCheckProof` refuses
    /// the first by name rather than reporting a verification of it.
    ///
    /// Ask for a proof with `entailProve(...)` or `Reasoner.withProofs(...)`.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn proof(&self) -> String {
        self.proof.clone()
    }
}

impl From<BoundaryAnswer> for ReasoningAnswer {
    fn from(produced: BoundaryAnswer) -> Self {
        let (answer, certificate, proof) = produced.into_proved_parts();
        Self {
            answer,
            certificate,
            proof,
        }
    }
}

/// Decide whether `document`'s ontology has a model at all. See
/// [`entail_consistency`].
pub(crate) fn consistency_impl(
    document: &str,
    step_cap: u32,
    work_cap: u32,
) -> Result<ReasoningAnswer, String> {
    consistency_to_string(document, step_cap, work_cap).map(ReasoningAnswer::from)
}

/// `entailConsistency(document, stepCap, workCap)` → is the knowledge base consistent?
///
/// `stepCap` narrows the per-decision tableau step cap; **0 means the knowledge
/// base's own cap**, not a cap of zero steps. It can only NARROW, so it cannot be
/// used to make a hard instance answerable — only to make the `budget-exhausted`
/// certificate reachable from a test.
///
/// `workCap` narrows the per-decision WORK cap on the same `0`-means-the-knowledge-base's-own-cap
/// rule and narrows only, and it bounds what `stepCap` cannot: a round is a PASS over the
/// completion graph rather than a unit of cost, so an ontology can make each round enormously
/// more expensive without taking more rounds. A run that reaches it answers `unknown` with
/// `work` equal to `work-budget` in its certificate.
///
/// The one DL service that answers for an unsatisfiable ontology, because it is the
/// one that detects one; every other throws rather than returning the vacuous
/// answer an ontology with no model gives.
///
/// Throws if `document` fails to parse or the reverse mapping fails.
#[wasm_bindgen(js_name = entailConsistency)]
pub fn entail_consistency(
    document: &str,
    step_cap: u32,
    work_cap: u32,
) -> Result<ReasoningAnswer, JsError> {
    consistency_impl(document, step_cap, work_cap).map_err(|e| JsError::new(&e))
}

/// The subsumption hierarchy over the named classes. See [`entail_classify`].
pub(crate) fn classify_impl(
    document: &str,
    step_cap: u32,
    work_cap: u32,
) -> Result<ReasoningAnswer, String> {
    classify_to_string(document, step_cap, work_cap).map(ReasoningAnswer::from)
}

/// `entailClassify(document, stepCap, workCap)` → the entailed subsumption hierarchy over
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
pub fn entail_classify(
    document: &str,
    step_cap: u32,
    work_cap: u32,
) -> Result<ReasoningAnswer, JsError> {
    classify_impl(document, step_cap, work_cap).map_err(|e| JsError::new(&e))
}

/// The entailed types of the named individuals. See [`entail_realize`].
pub(crate) fn realize_impl(
    document: &str,
    step_cap: u32,
    work_cap: u32,
) -> Result<ReasoningAnswer, String> {
    realize_to_string(document, step_cap, work_cap).map(ReasoningAnswer::from)
}

/// `entailRealize(document, stepCap, workCap)` → the entailed types of the ontology's named
/// individuals, and the most specific of them (`type` then `direct-type` lines).
///
/// Throws on a malformed document or an ontology with no model.
#[wasm_bindgen(js_name = entailRealize)]
pub fn entail_realize(
    document: &str,
    step_cap: u32,
    work_cap: u32,
) -> Result<ReasoningAnswer, JsError> {
    realize_impl(document, step_cap, work_cap).map_err(|e| JsError::new(&e))
}

/// Instance retrieval for one named class. See [`entail_instances`].
pub(crate) fn instances_impl(
    document: &str,
    class: &str,
    step_cap: u32,
    work_cap: u32,
) -> Result<ReasoningAnswer, String> {
    instances_to_string(document, class, step_cap, work_cap).map(ReasoningAnswer::from)
}

/// `entailInstances(document, class, stepCap, workCap)` → the named individuals entailed to
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
    work_cap: u32,
) -> Result<ReasoningAnswer, JsError> {
    instances_impl(document, class, step_cap, work_cap).map_err(|e| JsError::new(&e))
}

/// Axiom entailment by refutation. See [`entail_entails`].
pub(crate) fn entails_impl(
    document: &str,
    axiom: &str,
    step_cap: u32,
    work_cap: u32,
) -> Result<ReasoningAnswer, String> {
    entails_to_string(document, axiom, step_cap, work_cap).map(ReasoningAnswer::from)
}

/// `entailEntails(document, axiom, stepCap, workCap)` → does the ontology entail `axiom`?
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
/// collapsed to `false`: it means the search reached its step cap or its work cap, which
/// the certificate says in its own words.
///
/// Throws on a malformed document, an `axiom` that is not one triple, or an
/// ontology with no model.
#[wasm_bindgen(js_name = entailEntails)]
pub fn entail_entails(
    document: &str,
    axiom: &str,
    step_cap: u32,
    work_cap: u32,
) -> Result<ReasoningAnswer, JsError> {
    entails_impl(document, axiom, step_cap, work_cap).map_err(|e| JsError::new(&e))
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

// ── The conclusion-directed entailment services ─────────────────────────────────

/// Zip the caller's two import arrays into the boundary's ordered `(iri, document)` table.
///
/// # Why TWO arrays and not one array of `[iri, document]` pairs
///
/// wasm-bindgen has no ABI for a nested string array: `Vec<Vec<String>>` does not
/// implement `VectorFromWasmAbi`, because `ErasableGeneric` bottoms out at `&str` rather
/// than at `JsValue`. Reading a JS `Array` of `Array`s therefore needs `js-sys`, which this
/// crate does not depend on, and the alternative — receiving the pairs in `js/index.mjs`
/// and flattening them there — is structurally refused by
/// `scripts/check-wasm-js-exports.py`, which requires every `#[wasm_bindgen]` free
/// function to be re-exported from the package root under its OWN name, leaving no room
/// for a renaming wrapper.
///
/// So this host reuses the C ABI's convention — parallel arrays plus a length agreement —
/// rather than inventing a second one. Order is the caller's and is preserved: the
/// boundary's table is a list rather than a map precisely so the same input always
/// produces the same run.
///
/// Two arrays of different lengths are a caller error and are REFUSED, never truncated to
/// the shorter one: a silently dropped tail is an import the caller believes was supplied.
fn import_pairs<'a>(
    iris: &'a [String],
    documents: &'a [String],
) -> Result<Vec<(&'a str, &'a str)>, String> {
    if iris.len() != documents.len() {
        return Err(format!(
            "the import table has {} ontology IRI(s) and {} document(s); an entry is a PAIR, \
             so truncating to the shorter array would drop an import the caller supplied",
            iris.len(),
            documents.len()
        ));
    }
    Ok(iris
        .iter()
        .zip(documents)
        .map(|(iri, document)| (iri.as_str(), document.as_str()))
        .collect())
}

/// The certain answers of a basic graph pattern. See [`entail_certain_answers`].
pub(crate) fn certain_answers_impl(
    regime: &str,
    document: &str,
    pattern: &str,
    import_iris: &[String],
    import_documents: &[String],
) -> Result<ReasoningAnswer, String> {
    let imports = import_pairs(import_iris, import_documents)?;
    certain_answers_to_string(regime, document, pattern, &imports).map(ReasoningAnswer::from)
}

/// `entailCertainAnswers(regime, document, pattern)` → the substitutions the knowledge
/// base ENTAILS the pattern under, as `var` and `row` lines.
///
/// A certain answer is true in every model, not merely present in one closure, which is
/// what SPARQL's entailment regimes define the answers to a basic graph pattern to be.
///
/// `pattern` is N-Triples with `?name` in any position, the PREDICATE included. A blank
/// node in it is a NON-DISTINGUISHED variable — constrained by the match, not projected, and
/// not a column — which is what SPARQL says a query blank node is. A variable inside an RDF
/// 1.2 triple term is an ordinary variable: it binds, it is a column, and one NAME is one
/// VARIABLE wherever it was written, so a pattern using it above and below the triple-term
/// boundary is joined rather than split into two. A predicate variable is
/// projected like any other, and under `owl-rl` it also renders a `limit`: it ranges over the
/// whole predicate vocabulary, including the schema predicates and the constructs the
/// mechanisms beyond the rule table decide, and the closure holds neither.
///
/// A `limit` line says the row set may not be EXHAUSTIVE. Every row is sound
/// unconditionally; what needs a precondition is the claim about a row that is NOT there,
/// so no `limit` lines is the claim that the row set is complete.
///
/// The answer opens `mechanism <name>`. A pattern with a projected variable is
/// `strict-table`: the five mechanisms beyond the rule table are not run for one, because
/// a projected variable over what any of them decides is a different question — and that
/// one of them WOULD have been needed arrives as a `limit` line naming the lane, never as
/// an exhaustive empty answer. A pattern with NO projected variable is a conclusion graph,
/// is answered by the same fold `entailGraphEntails` runs, and names whichever of the seven
/// reached it; such an answer is the relation with no columns, so a `yes` is one bare `row`
/// line and a `no` is none.
///
/// `importIris` and `importDocuments` are the caller's `owl:imports` table, as two PARALLEL
/// arrays of the same length: entry `i` declares that the ontology IRI `importIris[i]`
/// denotes the N-Quads document `importDocuments[i]`. A premise carrying an `owl:imports`
/// states that its axioms are its own PLUS those of the documents it names, so this is where
/// those documents arrive and the `owl:imports` triple stays exactly where the caller wrote
/// it. **PurRDF fetches nothing**: an ontology IRI the table does not resolve throws by name,
/// never a network access and never a silently empty import. Two empty arrays are the
/// ordinary "imports nothing" case, and both are required rather than defaulted.
///
/// Throws on an unknown regime, on `owl-direct` or `rif` (each defined by an input this
/// signature does not carry), on a malformed document, pattern or import document, on import
/// arrays of different lengths, on a duplicate or empty import IRI, on a pattern that names
/// a graph, on a pattern that writes a variable in a literal's DATATYPE — a slot RDF reserves
/// for an IRI, and one a basic graph pattern has no binding to project — on an `owl:imports`
/// the table does not resolve, and on an inconsistent premise — whose refusal carries the
/// full report.
#[wasm_bindgen(js_name = entailCertainAnswers)]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
pub fn entail_certain_answers(
    regime: &str,
    document: &str,
    pattern: &str,
    import_iris: Vec<String>,
    import_documents: Vec<String>,
) -> Result<ReasoningAnswer, JsError> {
    certain_answers_impl(regime, document, pattern, &import_iris, &import_documents)
        .map_err(|e| JsError::new(&e))
}

/// Conclusion-directed entailment under a regime. See [`entail_graph_entails`].
pub(crate) fn graph_entails_impl(
    regime: &str,
    premise: &str,
    conclusion: &str,
    import_iris: &[String],
    import_documents: &[String],
) -> Result<ReasoningAnswer, String> {
    let imports = import_pairs(import_iris, import_documents)?;
    graph_entails_to_string(regime, premise, conclusion, &imports).map(ReasoningAnswer::from)
}

/// `entailGraphEntails(regime, premise, conclusion)` → does the premise entail the
/// conclusion GRAPH under the regime's rule table?
///
/// NOT [`entail_entails`], which asks the OWL 2 Direct-Semantics TABLEAU about one AXIOM
/// and renders a `purrdf-dl-certificate 1`. This asks the regime's RULE TABLE about a
/// conclusion GRAPH and renders a `purrdf-reasoning-report 4`. Different question,
/// different calculus, different certificate — and the banners differ so neither can be
/// parsed as the other.
///
/// The answer opens `mechanism <name>`: WHICH of the seven mechanisms reached the verdict.
/// `strict-table` is the regime's own rule table, run once; five more exist because no head
/// in that table has the conclusion's shape at all; and `composite` is two or more of those
/// five folded over one conclusion.
///
/// THREE verdicts, never two. `not-entailed` is a PROOF — the procedure was complete for
/// this premise — and `undecided` is what an incomplete procedure is entitled to say
/// instead. Collapsing the second into the first would turn a limitation of this library
/// into a false statement about the caller's data.
///
/// `importIris`/`importDocuments` are [`entail_certain_answers`]'s, and apply to the
/// PREMISE: the conclusion is a graph to match rather than an ontology to close, so an
/// `owl:imports` in it names nothing this service resolves.
///
/// Throws as [`entail_certain_answers`].
#[wasm_bindgen(js_name = entailGraphEntails)]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
pub fn entail_graph_entails(
    regime: &str,
    premise: &str,
    conclusion: &str,
    import_iris: Vec<String>,
    import_documents: Vec<String>,
) -> Result<ReasoningAnswer, JsError> {
    graph_entails_impl(regime, premise, conclusion, &import_iris, &import_documents)
        .map_err(|e| JsError::new(&e))
}

/// Entailment with its warrant RE-DECIDED. See [`entail_verify_entailment`].
pub(crate) fn verify_entailment_impl(
    regime: &str,
    premise: &str,
    conclusion: &str,
    import_iris: &[String],
    import_documents: &[String],
) -> Result<ReasoningAnswer, String> {
    let imports = import_pairs(import_iris, import_documents)?;
    verify_entailment_to_string(regime, premise, conclusion, &imports).map(ReasoningAnswer::from)
}

/// `entailVerifyEntailment(regime, premise, conclusion)` → [`entail_graph_entails`] with
/// the warrant re-decided, without running a reasoner.
///
/// The re-check re-derives nothing: "the closure follows from the premise" is the chase's
/// claim and `entailExplainConclusion` is its checker, while "the conclusion follows from
/// the closure" is this one and is finite and purely combinatorial.
///
/// `warrant absent` / `verified not-applicable` is a `not-entailed` or an `undecided`:
/// there is no evidence to re-decide, and a `false` there would read as a failed check
/// rather than as an absent one.
///
/// `importIris`/`importDocuments` are [`entail_certain_answers`]'s. The re-check runs
/// against the premise AS WRITTEN rather than against its imports closure: a warrant
/// re-decidable from the caller's own document is a stronger check than one only
/// re-decidable against a graph the library assembled.
///
/// Throws as [`entail_certain_answers`].
#[wasm_bindgen(js_name = entailVerifyEntailment)]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
pub fn entail_verify_entailment(
    regime: &str,
    premise: &str,
    conclusion: &str,
    import_iris: Vec<String>,
    import_documents: Vec<String>,
) -> Result<ReasoningAnswer, JsError> {
    verify_entailment_impl(regime, premise, conclusion, &import_iris, &import_documents)
        .map_err(|e| JsError::new(&e))
}

// ── Proofs: opt-in to produce, and a checker to consume ─────────────────────────

/// Answer one service WITH its proof term. See [`entail_prove`].
pub(crate) fn prove_impl(
    document: &str,
    service: &str,
    argument: &str,
    step_cap: u32,
    work_cap: u32,
) -> Result<ReasoningAnswer, String> {
    prove_to_string(document, service, argument, step_cap, work_cap).map(ReasoningAnswer::from)
}

/// `entailProve(document, service, argument, stepCap, workCap)` → a `ReasoningAnswer` whose
/// `.proof` is the run's `purrdf-dl-proof 1` document.
///
/// THE OPT-IN. Every other `entail*` function is unchanged and records nothing, so a caller
/// who does not want a proof runs exactly the search they ran before and pays exactly what
/// they paid before. This one records — which costs the completion graph of every tableau run
/// it keeps — and hands back a document `entailCheckProof` can verify.
///
/// `service` is one of `consistency`, `class-satisfiability`, `classify`, `realize`,
/// `instances`, `entails`, `extract-module`; an unknown spelling throws with the accepted set
/// named. `argument` is the question's own input in that service's grammar: empty for
/// `consistency`/`classify`/`realize` (a non-empty one throws rather than being discarded),
/// ONE N-Triples term for `class-satisfiability`/`instances`, ONE triple for `entails`, and a
/// `method <bot|top|star>` line followed by one term per line for `extract-module`.
///
/// `.answer` and `.certificate` are byte-identical to the same question asked without a
/// proof: recording is an observation the reasoner makes of itself, never a lever it reads.
///
/// `stepCap` and `workCap` are `entailConsistency`'s.
///
/// Throws on a malformed document, an unknown service, an argument wrong for the service, or
/// whatever that service itself refuses.
#[wasm_bindgen(js_name = entailProve)]
pub fn entail_prove(
    document: &str,
    service: &str,
    argument: &str,
    step_cap: u32,
    work_cap: u32,
) -> Result<ReasoningAnswer, JsError> {
    prove_impl(document, service, argument, step_cap, work_cap).map_err(|e| JsError::new(&e))
}

/// Check a proof against the caller's own ontology, question and answer. See
/// [`entail_check_proof`].
pub(crate) fn check_proof_impl(
    document: &str,
    service: &str,
    argument: &str,
    answer: &str,
    certificate: &str,
    proof: &str,
) -> Result<String, String> {
    check_dl_proof(document, service, argument, answer, certificate, proof)
}

/// `entailCheckProof(document, service, argument, answer, certificate, proof)` → the
/// `purrdf-dl-proof-check 1` report, or a throw naming the rejection.
///
/// THE CHECKER, and nothing in it trusts the producer: the ontology is parsed from
/// `document`, the question is re-derived from `service` and `argument`, the claims are read
/// back out of `answer`'s own grammar, and the checking context comes from a reverse mapping
/// this call performs itself. The proof supplies the runs and nothing else. An `entails`
/// proof for a different axiom, a proof for a different document, or a genuine proof of some
/// OTHER answer are each refused.
///
/// `answer` and `certificate` may be empty, and each empty one is a WEAKER check that says so
/// rather than one that quietly passed: with no answer the report reads `answer not-checked`,
/// and with no certificate a proof carrying a stopping receipt is refused, because there is
/// nothing for the receipt to be a receipt of.
///
/// Throws for a proof document that says `availability not-recorded` — an answer nobody asked
/// to record is never presented as a verified one — and for every other rejection.
#[wasm_bindgen(js_name = entailCheckProof)]
pub fn entail_check_proof(
    document: &str,
    service: &str,
    argument: &str,
    answer: &str,
    certificate: &str,
    proof: &str,
) -> Result<String, JsError> {
    check_proof_impl(document, service, argument, answer, certificate, proof)
        .map_err(|e| JsError::new(&e))
}

/// `entailCheckProofGoldenVectors()` — run the committed proof golden-vector artifact through
/// this build and throw on the first byte that differs.
///
/// The wasm half of the CROSS-HOST byte-stability assertion for the proof surface. The
/// artifact (`crates/validate/tests/fixtures/dl-proof.vectors`) is compiled into the module by
/// `purrdf-validate`, and the C-ABI crate's Rust test, the PyO3 crate's Rust test, the
/// `purrdf-validate` Rust test and this entry point all call the SAME checker over the SAME
/// bytes. Running it here produces and verifies real proof terms on `wasm32` — different
/// pointer width, different `usize` — and compares the rendered proof byte for byte against
/// what the native build produced. Since the rendered proof carries `ServiceProof::encode`'s
/// canonical bytes, a divergence here is a divergence in the PROOF TERM, not in a rendering.
///
/// Throws with the case name and a diff of the two strings on any mismatch.
#[wasm_bindgen(js_name = entailCheckProofGoldenVectors)]
pub fn entail_check_proof_golden_vectors() -> Result<(), JsError> {
    check_dl_proof_golden_vectors().map_err(|e| JsError::new(&e))
}

/// `entailCheckAbsentProof()` — prove that an answer nobody asked to record is NOT
/// presentable as a verified one, and throw if it is.
///
/// The companion to [`entail_check_proof_golden_vectors`], covering the path an artifact
/// cannot: there is no proof to pair an input with, and the property under test is that the
/// ABSENCE says so. An unrecorded answer must carry `availability not-recorded`,
/// `entailCheckProof` must refuse that document by name, and the same question asked WITH a
/// proof must check — so the refusal is the absence rather than a broken checker.
///
/// Throws naming which of the three failed.
#[wasm_bindgen(js_name = entailCheckAbsentProof)]
pub fn entail_check_absent_proof() -> Result<(), JsError> {
    check_absent_proof_is_not_verifiable().map_err(|e| JsError::new(&e))
}

// ── The session ─────────────────────────────────────────────────────────────────

/// A reasoning session over one ontology — `new Reasoner(document, stepCap)`.
///
/// Every `entail*` function above takes the document as a string and rebuilds
/// everything it needs, so asking three questions parses and reverse-maps the ontology
/// three times. This class holds the parsed document instead: constructing it parses
/// once, the first question needing a knowledge base reverse-maps once, and later
/// questions reuse both. That matters more here than on any other host — the browser
/// pays this cost on the main thread.
///
/// The methods answer exactly what the same-named functions answer: they are the
/// `purrdf_validate::regime` session those functions now wrap.
///
/// ```js
/// const r = new Reasoner(document, 0);
/// const consistent = r.consistency();
/// const hierarchy = r.classify();   // no second parse
/// r.free();                         // wasm objects are not garbage-collected for you
/// ```
#[wasm_bindgen]
pub struct Reasoner {
    /// The shared boundary's session. Every method is a thin call onto it.
    session: ReasonerSession,
}

impl std::fmt::Debug for Reasoner {
    /// Delegates to the session, which prints the SHAPE of the problem rather than a
    /// dump of thousands of interned ids.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Reasoner")
            .field("session", &self.session)
            .finish()
    }
}

#[wasm_bindgen]
impl Reasoner {
    /// Parse `document` and open a session over it.
    ///
    /// `stepCap` narrows the per-decision tableau step cap for every question asked
    /// through this session; **0 means the knowledge base's own cap**, not a cap of zero
    /// steps, and it can only NARROW. `workCap` narrows the per-decision WORK cap on the
    /// same rule — the cap on the matcher, scan, closure and clone work done INSIDE a
    /// round, which a round cap cannot see.
    ///
    /// Nothing is reverse-mapped here, so an ontology whose knowledge base cannot be
    /// built still constructs — and throws on the first question that needs one. That is
    /// deliberate: `profile`, `extractModule`, `justify` and `explainConclusion` never
    /// reason, and `profile` answers for any parseable document.
    ///
    /// Throws if `document` fails to parse.
    #[wasm_bindgen(constructor)]
    pub fn new(document: &str, step_cap: u32, work_cap: u32) -> Result<Self, JsError> {
        ReasonerSession::open(document, step_cap, work_cap)
            .map(|session| Self { session })
            .map_err(|e| JsError::new(&e))
    }

    /// `Reasoner.withProofs(document, stepCap, workCap)` → a session that RECORDS a proof
    /// term for every service that has one.
    ///
    /// The session-level opt-in. `new Reasoner(...)` is unchanged and records nothing; this
    /// records, so every answer it hands back carries a real `.proof` document instead of the
    /// `availability not-recorded` one, and `prove` becomes callable.
    ///
    /// It changes nothing a service DECIDES. Every `.answer` and `.certificate` is
    /// byte-identical to the same question asked through `new Reasoner(...)`.
    ///
    /// Throws if `document` fails to parse.
    #[wasm_bindgen(js_name = withProofs)]
    pub fn with_proofs(document: &str, step_cap: u32, work_cap: u32) -> Result<Self, JsError> {
        ReasonerSession::open_with_proofs(document, step_cap, work_cap)
            .map(|session| Self { session })
            .map_err(|e| JsError::new(&e))
    }

    /// Whether this session records proof terms — `true` only for `Reasoner.withProofs`.
    #[wasm_bindgen(js_name = recordsProofs)]
    #[must_use]
    pub fn records_proofs(&self) -> bool {
        self.session.records_proofs()
    }

    /// Answer `service` about `argument`, with its proof. See `entailProve`.
    ///
    /// Throws on a session that records nothing: open it with `Reasoner.withProofs`.
    pub fn prove(&mut self, service: &str, argument: &str) -> Result<ReasoningAnswer, JsError> {
        self.session
            .prove(service, argument)
            .map(ReasoningAnswer::from)
            .map_err(|e| JsError::new(&e))
    }

    /// Is the knowledge base consistent? See `entailConsistency`.
    pub fn consistency(&mut self) -> Result<ReasoningAnswer, JsError> {
        self.session
            .consistency()
            .map(ReasoningAnswer::from)
            .map_err(|e| JsError::new(&e))
    }

    /// The entailed subsumption hierarchy. See `entailClassify`.
    pub fn classify(&mut self) -> Result<ReasoningAnswer, JsError> {
        self.session
            .classify()
            .map(ReasoningAnswer::from)
            .map_err(|e| JsError::new(&e))
    }

    /// The entailed types of the named individuals. See `entailRealize`.
    pub fn realize(&mut self) -> Result<ReasoningAnswer, JsError> {
        self.session
            .realize()
            .map(ReasoningAnswer::from)
            .map_err(|e| JsError::new(&e))
    }

    /// The individuals entailed to be instances of `class`. See `entailInstances`.
    pub fn instances(&mut self, class: &str) -> Result<ReasoningAnswer, JsError> {
        self.session
            .instances(class)
            .map(ReasoningAnswer::from)
            .map_err(|e| JsError::new(&e))
    }

    /// Does the ontology entail `axiom`? See `entailEntails`.
    pub fn entails(&mut self, axiom: &str) -> Result<ReasoningAnswer, JsError> {
        self.session
            .entails(axiom)
            .map(ReasoningAnswer::from)
            .map_err(|e| JsError::new(&e))
    }

    /// Which OWL 2 profiles the ontology is provably in. See `entailProfile`.
    ///
    /// Purely syntactic: never builds a knowledge base, so it answers even for an
    /// ontology whose other services would throw.
    #[must_use]
    pub fn profile(&self) -> ReasoningAnswer {
        ReasoningAnswer::from(self.session.profile())
    }

    /// A module for `signature`. See `entailExtractModule`.
    #[wasm_bindgen(js_name = extractModule)]
    pub fn extract_module(
        &self,
        signature: &str,
        method: &str,
    ) -> Result<ReasoningAnswer, JsError> {
        self.session
            .extract_module(signature, method)
            .map(ReasoningAnswer::from)
            .map_err(|e| JsError::new(&e))
    }

    /// A justification for `axiom`. See `entailJustify`.
    pub fn justify(&self, axiom: &str) -> Result<ReasoningAnswer, JsError> {
        self.session
            .justify(axiom)
            .map(ReasoningAnswer::from)
            .map_err(|e| JsError::new(&e))
    }

    /// Why `conclusion` holds under `regime`. See `entailExplainConclusion`.
    #[wasm_bindgen(js_name = explainConclusion)]
    pub fn explain_conclusion(
        &self,
        regime: &str,
        conclusion: &str,
    ) -> Result<ReasoningAnswer, JsError> {
        self.session
            .explain_conclusion(regime, conclusion)
            .map(ReasoningAnswer::from)
            .map_err(|e| JsError::new(&e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The session answers exactly what the one-shot functions answer.
    ///
    /// `Reasoner` exists so a browser pays one parse for N questions, and the ONE thing
    /// that must not change is the result. Both halves of the pair are compared: an
    /// answer that matched while its certificate reported different `steps` would mean
    /// the session had carried work forward between questions.
    ///
    /// The `_impl` forms are compared rather than the `#[wasm_bindgen]` ones because
    /// constructing a `JsError` calls a wasm-only import that panics off wasm — the same
    /// reason `codec::resolve_media_type` returns a `String` error.
    #[test]
    fn the_session_answers_what_the_one_shot_functions_answer() {
        let mut session = ReasonerSession::open(SCHEMA, 0, 0).expect("parses");
        let class = "<http://example.org/B>";
        for (service, from_session, one_shot) in [
            (
                "consistency",
                session.consistency().expect("decides"),
                consistency_impl(SCHEMA, 0, 0).expect("decides"),
            ),
            (
                "classify",
                session.classify().expect("decides"),
                classify_impl(SCHEMA, 0, 0).expect("decides"),
            ),
            (
                "realize",
                session.realize().expect("decides"),
                realize_impl(SCHEMA, 0, 0).expect("decides"),
            ),
            (
                "instances",
                session.instances(class).expect("decides"),
                instances_impl(SCHEMA, class, 0, 0).expect("decides"),
            ),
        ] {
            let from_session = ReasoningAnswer::from(from_session);
            assert_eq!(from_session.answer(), one_shot.answer(), "{service} answer");
            assert_eq!(
                from_session.certificate(),
                one_shot.certificate(),
                "{service} certificate"
            );
        }
    }

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

    /// EVERY conclusion-directed service reaches THIS host, with its report.
    ///
    /// The sibling of `every_dl_service_reaches_this_host_with_its_certificate`, and it
    /// exists for the same reason: nine tableau services were once compiled into this
    /// artifact, budgeted for, and unreachable from the npm package root. The structural
    /// half of that — is the symbol re-exported? — is
    /// `scripts/check-entailment-surface.py`. This is the behavioural half: the service
    /// runs on this host and its answer says what the boundary says it says.
    #[test]
    fn every_conclusion_directed_service_reaches_this_host() {
        let conclusion = "<http://example.org/x> \
                          <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
                          <http://example.org/B> .\n";
        let pattern = "<http://example.org/x> \
                       <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?c .\n";

        let answers = certain_answers_impl("owl-rl", SCHEMA, pattern, &[], &[]).expect("answers");
        assert!(
            answers
                .answer()
                .starts_with("mechanism strict-table\nvar c\n")
        );
        // `?c` ranges over the ENTAILED types, so `B` is a row and it is not asserted.
        assert!(
            answers.answer().contains("\nrow <http://example.org/B>\n"),
            "{}",
            answers.answer()
        );

        let decided = graph_entails_impl("owl-rl", SCHEMA, conclusion, &[], &[]).expect("decides");
        assert_eq!(
            decided.answer(),
            "mechanism strict-table\nentailment entailed\n"
        );

        let checked =
            verify_entailment_impl("owl-rl", SCHEMA, conclusion, &[], &[]).expect("decides");
        assert!(
            checked
                .answer()
                .ends_with("warrant present\nverified true\n")
        );

        // …and all three carry the run, on the SAME banner the materialization lane uses,
        // naming the mechanism that answered.
        for (service, produced) in [
            ("certain-answers", &answers),
            ("graph-entails", &decided),
            ("verify-entailment", &checked),
        ] {
            let certificate = produced.certificate();
            assert!(
                certificate.starts_with("purrdf-reasoning-report 4\n"),
                "{service}: {certificate}"
            );
            assert!(
                certificate.contains("\nmechanism strict-table "),
                "{service}: {certificate}"
            );
        }
    }

    /// A conclusion nothing derives has NO warrant, and the answer says so rather than
    /// reporting a check that failed.
    #[test]
    fn an_unreached_conclusion_reports_an_absent_warrant_rather_than_a_failed_check() {
        let never = "<http://example.org/x> \
                     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
                     <http://example.org/Never> .\n";
        let checked = verify_entailment_impl("owl-rl", SCHEMA, never, &[], &[]).expect("decides");
        assert!(checked.answer().starts_with("mechanism strict-table\n"));
        assert!(checked.answer().contains("\nentailment not-entailed\n"));
        assert!(
            checked
                .answer()
                .ends_with("warrant absent\nverified not-applicable\n")
        );
    }

    /// The two regimes this service is not total over are REFUSED by name.
    #[test]
    fn the_regimes_defined_by_a_missing_input_are_refused_by_name() {
        let conclusion = "<http://example.org/x> \
                          <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
                          <http://example.org/B> .\n";
        for regime in ["owl-direct", "rif"] {
            let refused = graph_entails_impl(regime, SCHEMA, conclusion, &[], &[])
                .expect_err("defined by an input this signature does not carry");
            assert!(refused.contains(regime), "{refused}");
        }
    }

    /// The proof surface's CROSS-HOST assertion, made from THIS crate.
    ///
    /// The wasm half is `entailCheckProofGoldenVectors`, called as real wasm by
    /// `js/tests/entail.test.mjs`; both run the same checker over the same committed
    /// artifact, so a native/wasm divergence in a PROOF TERM is one failing case.
    #[test]
    fn the_dl_proof_golden_vector_matches_natively() {
        check_dl_proof_golden_vectors().expect("the DL proof golden vector");
    }

    /// An answer nobody asked to record is never presented as a verified one, on this host.
    #[test]
    fn an_absent_proof_is_never_presented_as_a_verified_one() {
        check_absent_proof_is_not_verifiable().expect("the absent-proof refusal");
    }

    /// EVERY proof-bearing service reaches this host, with a proof this host can CHECK.
    ///
    /// The sibling of `every_dl_service_reaches_this_host_with_its_certificate`, for the
    /// surface this stage adds: a proof compiled in and unreachable is the defect, and a
    /// proof reachable but uncheckable is the worse one.
    #[test]
    fn every_proof_bearing_service_reaches_this_host_with_a_checkable_proof() {
        let questions = [
            ("consistency", ""),
            ("class-satisfiability", "<http://example.org/A>"),
            ("classify", ""),
            ("realize", ""),
            ("instances", "<http://example.org/C>"),
            ("entails", CHAIN_AXIOM),
            ("extract-module", "method bot\n<http://example.org/A>"),
        ];
        assert_eq!(questions.len(), 7, "seven services carry a proof term");
        for (service, argument) in questions {
            let proved = prove_impl(TAXONOMY, service, argument, 0, 0)
                .unwrap_or_else(|error| panic!("{service}: {error}"));
            assert!(
                proved.proof().contains("\navailability recorded\n"),
                "{service}: {}",
                proved.proof()
            );
            let report = check_proof_impl(
                TAXONOMY,
                service,
                argument,
                &proved.answer(),
                &proved.certificate(),
                &proved.proof(),
            )
            .unwrap_or_else(|error| panic!("{service}: {error}"));
            assert!(
                report.starts_with("purrdf-dl-proof-check 1\n"),
                "{service}: {report}"
            );
        }
    }

    /// The default cost is unchanged: an ordinary `entail*` answer records NOTHING, and its
    /// proof getter says so rather than handing back a blank a caller could read as a
    /// verified emptiness.
    #[test]
    fn an_unasked_answer_carries_the_absent_proof_document_on_this_host() {
        let plain = consistency_impl(TAXONOMY, 0, 0).expect("decides");
        assert_eq!(
            plain.proof(),
            "purrdf-dl-proof 1\navailability not-recorded\n"
        );
        assert!(
            check_proof_impl(
                TAXONOMY,
                "consistency",
                "",
                &plain.answer(),
                &plain.certificate(),
                &plain.proof(),
            )
            .is_err(),
            "an absent proof must never check"
        );
        // …and the recording session says the opposite, with the same answer bytes.
        let mut recording = ReasonerSession::open_with_proofs(TAXONOMY, 0, 0).expect("parses");
        assert!(recording.records_proofs());
        let proved = ReasoningAnswer::from(recording.consistency().expect("decides"));
        assert_eq!(proved.answer(), plain.answer());
        assert_eq!(proved.certificate(), plain.certificate());
        assert!(proved.proof().contains("\navailability recorded\n"));
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
        assert!(closed.report().starts_with("purrdf-reasoning-report 4\n"));
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
        assert_eq!(rules_impl("simple").expect("known"), [] as [String; 0]);
        assert_eq!(
            implemented_rules_impl("simple").expect("known"),
            [] as [String; 0]
        );
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
            ("consistency", consistency_impl(TAXONOMY, 0, 0)),
            ("classify", classify_impl(TAXONOMY, 0, 0)),
            ("realize", realize_impl(TAXONOMY, 0, 0)),
            (
                "instances",
                instances_impl(TAXONOMY, "<http://example.org/C>", 0, 0),
            ),
            ("entails", entails_impl(TAXONOMY, CHAIN_AXIOM, 0, 0)),
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
        let tableau = consistency_impl(TAXONOMY, 0, 0).expect("consistency");
        assert_eq!(tableau.answer(), "consistency true\n");
        assert!(
            tableau
                .certificate()
                .starts_with("purrdf-dl-certificate 1\n")
        );
        assert!(tableau.certificate().contains("\ncompleteness decided\n"));
        let chase = materialize_impl(TAXONOMY, "owl-rl", "").expect("owl-rl");
        assert!(chase.report().starts_with("purrdf-reasoning-report 4\n"));
        assert!(!chase.report().contains("completeness decided"));
    }

    /// A starved search answers `unknown`, never `false`, and says so.
    #[test]
    fn an_exhausted_budget_crosses_this_host_intact() {
        let starved = entails_impl(TAXONOMY, CHAIN_AXIOM, 1, 0).expect("decides nothing");
        assert_eq!(starved.answer().lines().next(), Some("entails unknown"));
        assert!(
            starved
                .certificate()
                .contains("\ncompleteness budget-exhausted\n")
        );
        // The SECOND cap, starved on its own: a round is a pass rather than a unit of
        // cost, so this one bounds the work done inside a round — and it reaches the
        // same honest three-valued answer through this host.
        let overworked = entails_impl(TAXONOMY, CHAIN_AXIOM, 0, 1).expect("decides nothing");
        assert_eq!(overworked.answer().lines().next(), Some("entails unknown"));
        assert!(
            overworked
                .certificate()
                .contains("\ncompleteness budget-exhausted\n")
        );
        assert!(overworked.certificate().contains("\nwork-budget 1\n"));
        assert!(
            entails_impl(TAXONOMY, CHAIN_AXIOM, 0, 0)
                .expect("decides")
                .answer()
                .starts_with("entails true\n")
        );
    }

    /// Refusals cross this host as messages a JS caller can act on.
    #[test]
    fn dl_refusals_name_what_went_wrong() {
        assert!(consistency_impl("this is not n-quads\n", 0, 0).is_err());
        assert!(instances_impl(TAXONOMY, "not a term", 0, 0).is_err());
        let error = extract_module_impl(TAXONOMY, "", "nested").expect_err("unknown method");
        assert!(error.contains("bot, top, star"), "{error}");
        // The existential refusal is per CONCLUSION, not per regime: `rdfs`
        // derives the chain axiom through plain Datalog rules and explains it,
        // while `rdf` — whose three-rule table cannot reach it, beside two
        // existential rules that might — refuses by name.
        let proof = explain_conclusion_impl(TAXONOMY, "rdfs", CHAIN_AXIOM)
            .expect("rdfs derives the chain axiom by Datalog rules");
        assert!(proof.certificate().contains("\nchecked true\n"));
        let error = explain_conclusion_impl(TAXONOMY, "rdf", CHAIN_AXIOM).expect_err("existential");
        assert!(error.contains("existential"), "{error}");
    }
}
