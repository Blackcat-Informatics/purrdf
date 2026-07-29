// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Entailment-**regime** materialization → canonical N-Quads **and a rendered
//! report**, in one call — the shared string boundary every language binding
//! routes through; plus the OWL 2 Direct-Semantics reasoning services, each with
//! **its own** certificate rendered beside its answer.
//!
//! # Two lanes, two completeness notions, never interchanged
//!
//! The first half of this module is the **chase**: [`materialize_to_nquads_string`]
//! closes a document under a regime's rule table and renders a
//! [`ReasoningReport`] whose `completeness` line is `rules(r) ∖ implemented(r)`.
//!
//! The second half is the **tableau**: [`consistency_to_string`],
//! [`classify_to_string`], [`realize_to_string`], [`instances_to_string`],
//! [`entails_to_string`], [`profile_to_string`], [`extract_module_to_string`],
//! [`justify_to_string`] and [`explain_conclusion_to_string`]. Those services
//! render a [`DlCertificate`], whose completeness is `decided` /
//! `decided-within-boundaries` / `budget-exhausted` — a different measurement
//! entirely, because the DL lane has no rule table to subtract and reusing the
//! chase's notion would report "exact" for a search that ran out of budget. The
//! two renderings therefore carry DIFFERENT banners, so a consumer cannot mistake
//! one for the other by parsing the wrong grammar.
//!
//! # Not to be confused with [`crate::entail`]
//!
//! The name collision is real and worth stating in full, because the two modules
//! sit beside each other and both say "entailment":
//!
//! * [`crate::entail`] is **SHACL-AF `sh:rule` entailment**. It takes a *shapes*
//!   graph, applies every active `sh:rule` to a fixpoint, and is a feature of the
//!   SHACL Advanced Features specification. Nothing about it is a SPARQL
//!   entailment regime.
//! * **This module** is **SPARQL entailment-regime materialization**
//!   (`sparql:entailmentRegime` — `Simple`, `RDF`, `RDFS`, `OWL-RL`, …). It takes
//!   no shapes at all: it closes an RDF document under the regime's own rule
//!   table via [`purrdf_entail::materialize`].
//!
//! A caller that wants "the inferences my shapes declare" wants
//! [`crate::entail`]; a caller that wants "the inferences the RDFS semantics
//! license" wants this module. They compose, but neither is the other.
//!
//! # Why this lives in `purrdf-validate`
//!
//! The crate is named for validation and regime entailment is not validation —
//! that tension is acknowledged rather than papered over. It lives here because
//! this crate is already *the* string boundary the three bindings share: the
//! C-ABI (`purrdf-rdf-capi`), WASM (`purrdf-wasm`) and the PyO3 caller all reach
//! [`crate::shacl::validate_to_sarif_string`] and
//! [`crate::entail::entail_to_ntriples_string`] here, and a fourth surface split
//! into its own crate would give the bindings two boundary crates to keep in
//! step instead of one. If a dedicated crate is ever wanted, that split is made
//! once, from here.
//!
//! # Determinism of the rendered report
//!
//! The report string is the value the C ABI and WASM will carry across their host
//! boundaries, so a byte-unstable rendering would make tri-host comparison
//! meaningless. [`render_reasoning_report`] is byte-stable by construction:
//!
//! * **Fixed field order.** The lines are emitted in one hard-coded order, never
//!   by iterating a map.
//! * **Fixed sequence order, inherited not invented.** Missing rules and fired
//!   rules arrive from `purrdf-entail` in specification table order, and
//!   boundaries in [`Construct`](purrdf_entail::Construct) declaration order;
//!   this module re-orders nothing.
//! * **No ambient input.** No clock, no host paths, no RNG, no floating point.
//!   The `contract-hash` line is a digest of the *calculus*, not of a run.
//! * **Width-independent numbers.** Counts render as plain decimal, so a 32-bit
//!   host (wasm32) and a 64-bit host produce the same bytes.
//! * **Fixed line discipline.** `\n` endings, one fact per line, always a
//!   trailing newline, and a leading [`REPORT_FORMAT_BANNER`] line so a later
//!   change to the format is visible instead of silent.
//!
//! # Portability
//!
//! Pure in-memory string work over the wasm-clean native codecs and the wasm-clean
//! `purrdf-entail` chase — no threads of its own, no filesystem, no clock, no RNG,
//! and no dependency beyond `purrdf-entail` over what the crate already had.

use core::fmt;
use core::fmt::Write as _;

use purrdf_core::{RdfLiteral, RdfTerm, RdfTriple, TermValue, emit_term};
use purrdf_entail::{
    ChaseProof, Completeness, DlAxiom, DlCertificate, DlCompleteness, EntailError, Justification,
    Materialization, ModuleMethod, OwlProfile, ProfileCertificate, Reasoner, ReasoningReport,
    Regime, RuleSet, Verdict, explain_conclusion, extract_module, implemented, justify,
    materialize, parse_rif_xml, profile, rules,
};

/// The accepted regime spellings, in the order an error message lists them.
///
/// These are exactly the value names of the CLI's `--regime` / `--entailment`
/// flag, so one spelling works at the command line, through the C ABI, through
/// WASM and from Python.
pub const REGIME_NAMES: [&str; 7] = ["simple", "rdf", "rdfs", "owl-rl", "owl-direct", "rif", "d"];

/// The regimes whose `program` argument to [`materialize_to_nquads_string`] is a
/// document rather than the empty string.
///
/// This constant replaces the refusal set this module used to publish. That list
/// named the two regimes the boundary would not close — and it no longer refuses
/// any, because the input those two were missing is now a PARAMETER rather than an
/// absence: `rif` entails under the caller's rule document, which is what this
/// names.
///
/// `owl-direct` is deliberately NOT here, and that is a statement rather than an
/// omission. Its extra input is the QUERY's class expressions, and a
/// document-in/document-out boundary has no query at all — so there is nothing
/// withheld. What it closes is the query-independent OWL Direct-Semantics
/// augmentation (the classification, the realization, the entailed role assertions
/// and the `owl:sameAs` identifications the tableau decides about the ontology's
/// own named terms), which is the whole answer when there is no query to direct.
/// A caller who HAS a query reaches the query-directed lane through
/// `purrdf::query_with_entailment`, which has one.
///
/// Every other regime is defined by a rule table this workspace states, so its
/// `program` is empty — and a non-empty one is an ERROR rather than a silently
/// discarded argument.
pub const PROGRAM_REGIME_NAMES: [&str; 1] = ["rif"];

/// The version banner every rendered report opens with.
///
/// `3` because the grammar moved, which is the whole reason a banner is emitted at all.
/// Against `2`, two lines are new and nothing was removed:
///
/// * `extension <rule-id>` — the rules the run's calculus states that NO specification
///   table does. Without it a caller reading `completeness exact-within-boundaries` and
///   `fired ext-eq-diff-sym 1` had to know, from prose, that one of those ids is not in
///   OWL 2 Profiles §4.3 and the other seventy-eight are.
/// * `termination …` — the weak-acyclicity certificate the restricted chase computed to
///   admit the program it then ran. It was computed on every `rdf` and `rdfs` run and read
///   by nothing, so a proof the workspace had already paid for reached no caller.
///
/// Against `1`: the `withheld-surrogates` count is rendered (it was reachable only
/// from the CLI's private renderer, so the four existential rules were unobservable from
/// Python, WASM and the C ABI); an inconsistency renders its GRAPH and its premise
/// TRIPLES rather than a bare count; and the trailing `overclaims` line is gone, because
/// `ReasoningReport` no longer carries a completeness field that could contradict its
/// boundary list and a rendered constant is not a gate.
///
/// It is also the marker that lets a REFUSAL carry a report: an inconsistent run has no
/// closure, so its certificate travels in the error message, beginning at the first line
/// equal to this banner. See [`render_entail_error`].
pub const REPORT_FORMAT_BANNER: &str = "purrdf-reasoning-report 3";

/// The media type this boundary parses its input document as.
///
/// N-Quads, because N-Triples is a syntactic subset of it: one entry point
/// accepts both, and a document that names a graph keeps naming it.
const INPUT_MEDIA_TYPE: &str = "application/n-quads";

/// The CLI spelling of `regime` — the left inverse of [`parse_regime`].
#[must_use]
pub const fn regime_name(regime: Regime) -> &'static str {
    match regime {
        Regime::Simple => "simple",
        Regime::Rdf => "rdf",
        Regime::Rdfs => "rdfs",
        Regime::OwlRl => "owl-rl",
        Regime::OwlDirect => "owl-direct",
        Regime::Rif => "rif",
        Regime::D => "d",
    }
}

/// Parse a regime from its CLI spelling (`simple`, `rdf`, `rdfs`, `owl-rl`,
/// `owl-direct`, `rif`, `d`).
///
/// Matching is exact and case-sensitive, as the CLI writes the names.
///
/// # Errors
///
/// Returns a message naming the offending spelling *and the whole accepted set*,
/// so a caller three language boundaries away can fix the call without reading
/// this source.
///
/// ```
/// use purrdf_validate::regime::{parse_regime, regime_name};
///
/// assert_eq!(regime_name(parse_regime("owl-rl").expect("known")), "owl-rl");
/// let error = parse_regime("OWL-RL").expect_err("case-sensitive");
/// assert!(error.contains("owl-rl"), "{error}");
/// ```
pub fn parse_regime(name: &str) -> Result<Regime, String> {
    match name {
        "simple" => Ok(Regime::Simple),
        "rdf" => Ok(Regime::Rdf),
        "rdfs" => Ok(Regime::Rdfs),
        "owl-rl" => Ok(Regime::OwlRl),
        "owl-direct" => Ok(Regime::OwlDirect),
        "rif" => Ok(Regime::Rif),
        "d" => Ok(Regime::D),
        other => Err(format!(
            "unknown entailment regime \"{other}\"; accepted: {}",
            REGIME_NAMES.join(", ")
        )),
    }
}

/// One closure of one document under one regime: the canonical N-Quads and the
/// rendered report, both as strings.
///
/// Two strings rather than a tuple because both cross an FFI boundary and a
/// positional pair is the wrong thing to get backwards there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegimeClosure {
    /// The materialized dataset as canonical (RDFC-1.0) N-Quads.
    nquads: String,
    /// The run's [`ReasoningReport`], rendered by [`render_reasoning_report`].
    report: String,
}

impl RegimeClosure {
    /// The materialized dataset — every input quad plus every inferred triple —
    /// as canonical (RDFC-1.0) N-Quads.
    #[must_use]
    pub fn nquads(&self) -> &str {
        &self.nquads
    }

    /// What the run did, rendered by [`render_reasoning_report`].
    #[must_use]
    pub fn report(&self) -> &str {
        &self.report
    }

    /// Consume the closure, yielding `(nquads, report)`.
    #[must_use]
    pub fn into_parts(self) -> (String, String) {
        (self.nquads, self.report)
    }
}

/// Materialize `document` under the regime spelled `regime`, returning canonical
/// N-Quads **and** the rendered reasoning report.
///
/// `document` is parsed as N-Quads, which accepts an N-Triples document
/// unchanged. The closure is the input dataset plus every triple the regime's
/// implemented rules infer, serialized through the native RDFC-1.0 flat
/// serializer (deterministic, blank-node-canonical).
///
/// # `program` — the regime's own input, not an option
///
/// Five of the seven regimes are defined by a rule table this workspace states and
/// `owl-direct` is directed by a query this boundary does not have, so for six of
/// them the whole input is the document and `program` must be EMPTY; a non-empty
/// one is refused rather than ignored, because an argument silently discarded is
/// how a caller comes to believe their rules ran.
///
/// `rif` is the exception named by [`PROGRAM_REGIME_NAMES`]: RIF entails under the
/// CALLER's rules, this workspace declares none of its own, and so `program` is
/// that rule set as a normative RIF-in-XML document (parsed by
/// [`purrdf_entail::parse_rif_xml`]). An `Import` directive is refused: resolving
/// one is I/O, and this boundary performs none — a caller who needs imports
/// resolves them itself through [`purrdf_entail::resolve_rif_imports`] and reaches
/// [`purrdf_entail::materialize`] directly.
///
/// This is what makes the boundary TOTAL: every one of [`REGIME_NAMES`] closes, and
/// none of them is refused for being the regime it is.
///
/// The report is never optional and never separately requested — the same
/// discipline [`purrdf_entail::materialize`] enforces in Rust, carried across the
/// string boundary. A binding that renders "RDFS entailment" without saying which
/// of the eighteen RDFS patterns did not fire — or, once they all do, which
/// CONSTRUCTS the run still could not fully handle — is the overclaim the report
/// exists to prevent. A complete rule table is not a complete closure, which is
/// why the report carries `boundary` lines beside `missing` ones.
///
/// # Errors
///
/// * An unknown `regime` spelling — the message names the accepted set.
/// * A non-empty `program` for a regime that takes none — the message names
///   [`PROGRAM_REGIME_NAMES`].
/// * A malformed RIF-in-XML `program`, or one carrying an `Import`.
/// * A malformed input document (the native codec's own diagnostic).
/// * An exhausted evaluation ceiling or a dataset that cannot be frozen (the
///   `purrdf-entail` error).
///
/// # Examples
///
/// ```
/// use purrdf_validate::regime::materialize_to_nquads_string;
///
/// let data = "<http://example.org/A> \
///     <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/B> .\n\
///     <http://example.org/x> \
///     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .\n";
///
/// let closed = materialize_to_nquads_string("rdfs", data, "").expect("rdfs closure");
/// // rdfs9 re-types the instance.
/// assert!(closed.nquads().contains(
///     "<http://example.org/x> \
///      <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/B> ."
/// ));
/// // …and the report says so, and says what it could not do. The `boundary`
/// // lines are the load-bearing ones: they survive a rule table going complete,
/// // which a `sound-incomplete <n>` assertion would not.
/// assert!(closed.report().contains("\nfired rdfs9 "));
/// assert!(closed.report().contains("\nboundary "));
/// // …including the count of the conclusions it WITHHELD. `rdfD1`, `rdfD1a`, `rdfs14`
/// // and `rdfs14a` all fire and none of them can ever appear in a `fired` line, so this
/// // number is the only observable they have — and it reaches every host, not just the
/// // command line.
/// assert!(closed.report().contains("\nwithheld-surrogates "));
/// // The knowledge base was consistent, and the report says so as a checked fact.
/// assert!(closed.report().ends_with("inconsistency none\n"));
/// ```
pub fn materialize_to_nquads_string(
    regime: &str,
    document: &str,
    program: &str,
) -> Result<RegimeClosure, String> {
    let parsed = parse_regime(regime)?;
    let dataset = purrdf_rdf::parse_dataset(document.as_bytes(), INPUT_MEDIA_TYPE, None)
        .map_err(|diagnostic| diagnostic.to_string())?;
    // Bound here, outside the call below, because the plan BORROWS it: a rule set
    // built inline would not outlive the value that names it.
    let rules = regime_rule_set(parsed, regime, program)?;
    let (closure, report) = materialize(&dataset, regime_plan(parsed, &rules))
        .map_err(|error| render_entail_error(regime, &error))?;
    Ok(RegimeClosure {
        nquads: purrdf_rdf::canonical_flat_nquads(closure.as_ref())?,
        report: render_reasoning_report(&report),
    })
}

/// The plan that closes `regime` under `rules` — the ONE map from a regime spelling
/// to a [`Materialization`], shared by every host.
///
/// It lives here rather than in each binding for the reason this whole module does: the
/// C ABI, WASM and PyO3 hosts must not each decide what `owl-direct` means at a document
/// boundary. `rules` is the value [`regime_rule_set`] returned for the same regime, and is
/// ignored by the six regimes whose rule table is the specification's.
///
/// `owl-direct` gets the EMPTY basic graph pattern: a document boundary has no query, so
/// what runs is the query-independent augmentation — see [`PROGRAM_REGIME_NAMES`].
#[must_use]
pub fn regime_plan(regime: Regime, rules: &RuleSet) -> Materialization<'_> {
    match regime {
        Regime::Simple => Materialization::Simple,
        Regime::Rdf => Materialization::Rdf,
        Regime::Rdfs => Materialization::Rdfs,
        Regime::OwlRl => Materialization::OwlRl,
        Regime::D => Materialization::D,
        Regime::OwlDirect => Materialization::OwlDirect(&[]),
        Regime::Rif => Materialization::Rif(rules),
    }
}

/// The rule set `program` carries for `regime`, or the empty one for a regime that
/// takes no program.
///
/// A non-empty `program` for a regime with a specification rule table is an error
/// and not a discarded argument: a caller who passed rules to `rdfs` believes they
/// ran, and returning a closure that ignored them is the failure this whole module
/// exists to prevent.
///
/// `spelling` is the regime's CLI name, carried separately so every host's message
/// names the regime the CALLER wrote rather than a `Debug` rendering of the enum.
///
/// # Errors
///
/// A non-empty `program` for a regime that takes none; a malformed RIF-in-XML
/// document; or a rule document carrying an `Import` (resolving one is I/O this
/// boundary does not perform).
pub fn regime_rule_set(regime: Regime, spelling: &str, program: &str) -> Result<RuleSet, String> {
    if regime != Regime::Rif {
        return if program.trim().is_empty() {
            Ok(RuleSet::new())
        } else {
            Err(format!(
                "entailment regime \"{spelling}\" takes no rule document, and one was \
                 supplied; its rule table is the specification's. Regimes that take one: {}",
                PROGRAM_REGIME_NAMES.join(", ")
            ))
        };
    }
    let parsed = parse_rif_xml(program)
        .map_err(|error| format!("entailment regime \"{spelling}\": rule document: {error}"))?;
    if let Some(import) = parsed.imports.first() {
        return Err(format!(
            "entailment regime \"{spelling}\": the rule document imports \
             \"{}\", and resolving an import is I/O this boundary does not perform; \
             resolve it in the caller and use purrdf_entail::materialize",
            import.location
        ));
    }
    Ok(parsed.ruleset)
}

/// The rule table `regime` is *defined by*, one specification rule name per line.
///
/// The empty string for a regime with no rule table (`simple`, and the two that
/// are not forward-materializable). Lines are in specification table order — the
/// order `purrdf-entail` returns them in — and the string always ends with a
/// newline when it is non-empty.
///
/// # Errors
///
/// An unknown `regime` spelling; the message names the accepted set.
///
/// ```
/// use purrdf_validate::regime::rules_string;
///
/// // OWL 2 RL is defined by 78 rules.
/// assert_eq!(rules_string("owl-rl").expect("known").lines().count(), 78);
/// // `simple` is the identity closure: no rule table at all.
/// assert_eq!(rules_string("simple").expect("known"), "");
/// ```
pub fn rules_string(regime: &str) -> Result<String, String> {
    Ok(rule_lines(rules(parse_regime(regime)?)))
}

/// The subset of [`rules_string`] this workspace's chase actually fires today,
/// one specification rule name per line.
///
/// `rules_string(r)` minus `implemented_rules_string(r)` is the regime's
/// measurable gap — the same gap the rendered report's `missing` lines name.
///
/// # Errors
///
/// An unknown `regime` spelling; the message names the accepted set.
///
/// The gap is a MEASUREMENT, so this example measures it rather than asserting a
/// number that goes stale the day a rule lands:
///
/// ```
/// use purrdf_validate::regime::{implemented_rules_string, rules_string};
///
/// let defined = rules_string("rdfs").expect("known");
/// let fired = implemented_rules_string("rdfs").expect("known");
/// let missing: Vec<&str> = defined
///     .lines()
///     .filter(|rule| !fired.lines().any(|f| f == *rule))
///     .collect();
/// // `fired` is always a subsequence of `defined`, so the two agree — and the
/// // gap is legitimately empty for a regime whose table is fully implemented.
/// assert_eq!(missing.len(), defined.lines().count() - fired.lines().count());
/// ```
pub fn implemented_rules_string(regime: &str) -> Result<String, String> {
    Ok(rule_lines(implemented(parse_regime(regime)?)))
}

/// `rules`, one canonical specification name per line, newline-terminated.
fn rule_lines(rules: &[purrdf_entail::RuleId]) -> String {
    let mut out = String::new();
    for rule in rules {
        out.push_str(rule.as_str());
        out.push('\n');
    }
    out
}

/// Render `report` to the byte-stable textual form documented on this module.
///
/// The grammar, in emission order — one fact per line, `\n`-terminated:
///
/// ```text
/// purrdf-reasoning-report 3
/// regime <cli-spelling>
/// completeness exact | completeness exact-within-boundaries | completeness sound-incomplete <count>
/// missing <rule-id>                       (0..n, specification table order)
/// extension <rule-id>                     (0..n, declaration order)
/// fired <rule-id> <conclusions>           (0..n, specification table order)
/// boundary <construct> <reason>           (0..n, Construct declaration order)
/// budget join-steps <n>
/// budget stored-facts <n>
/// budget term-arena-bytes <n>
/// contract-hash <64 lowercase hex>
/// withheld-surrogates <n>
/// termination none | termination weakly-acyclic positions <n> existential-edges <n>
/// inconsistency none | inconsistency <rule-id> premises <n>
/// inconsistency-graph default | inconsistency-graph <term>   (only after a witness)
/// inconsistency-premise <s> <p> <o>       (n of them, the rule's own premise order)
/// ```
///
/// `extension` is the mirror image of `missing`, and the two sit together because they
/// answer the same question from opposite sides. A `missing` id is a rule the
/// specification defines and this workspace does not fire, so the closure may be SMALLER
/// than the regime requires. An `extension` id is a rule this workspace fires that no
/// specification table states, so the closure may be LARGER — still sound, but larger.
/// `owl-rl` renders exactly one (`ext-eq-diff-sym`, symmetry of `owl:differentFrom`) and
/// every other regime renders none, which is what lets a caller that must act only on
/// normative conclusions decide from the report rather than from prose. Note what it does
/// NOT say: it names the rules the run's calculus STATES beyond the table, whether or not
/// they fired, because a caller choosing whether to trust a closure needs to know which
/// rule set produced it. Whether an extension actually contributed is the `fired` line.
///
/// `termination` is the chase's own proof that the run had to stop. `rdf` and `rdfs` state
/// existentially quantified rules and are evaluated by `purrdf-datalog`'s restricted
/// chase, which INVENTS terms — so their programs are certified weakly acyclic before a
/// round runs, and an uncertified one is refused outright rather than run under a budget.
/// The two numbers are the size of that proof and are a function of the CLAUSE SET, so
/// they differ between those two lanes and do not vary with the data. Every other regime
/// renders `none`, which says its rules invent no term and so owe no proof — not that
/// termination is unknown.
///
/// `completeness` has three forms and the middle one is the interesting one:
/// `exact-within-boundaries` says the rule TABLE was complete and the run still
/// met a construct it could not fully handle, which is what an `owl-rl` closure
/// is. It is DERIVED from the `boundary` lines below it —
/// [`ReasoningReport::completeness`] computes it rather than reading a field — so a
/// consumer never has to reconcile the two, and a self-contradicting certificate has no
/// constructor to come from. That is why there is no trailing `overclaims` line any more:
/// a rendered constant is a disclosure, not a gate.
///
/// `withheld-surrogates` is the only observable trace of `rdfD1`, `rdfD1a`, `rdfs14` and
/// `rdfs14a`. All four fire, every conclusion they reach mentions a blank node a SPARQL
/// entailment regime may not answer with, so none of them can ever appear in a `fired`
/// line — and without this number a caller could not tell an RDFS run that evaluated them
/// from one that did not. It reached only the CLI before, which left the four rules
/// invisible from exactly the hosts the report exists for.
///
/// The `inconsistency` block names the rule that detected a clash, the graph whose closure
/// refused, and the asserted triples that satisfied the rule, in that rule's own premise
/// order — so a reader can line them up against the specification's rule-table entry. It
/// is `none` for every closure this boundary produces, and that is a CHECKED fact rather
/// than a vacuous one: the seventeen OWL 2 RL rules that conclude `false` all run, and a
/// run that witnesses one is REFUSED. The refusal still carries this rendering; see
/// [`render_entail_error`].
///
/// Premise terms are in N-Triples term syntax, which is self-delimiting, so a three-term
/// line stays unambiguous even when a literal's lexical form holds a space.
#[must_use]
pub fn render_reasoning_report(report: &ReasoningReport) -> String {
    RenderedReport(report).to_string()
}

/// Render the boundary's refusal for `error`, under the regime spelling `regime`.
///
/// The one map from a [`purrdf_entail::EntailError`] to the string every host's error
/// channel carries — the C ABI's message buffer, WASM's `JsError`, Python's `ValueError`.
///
/// # An inconsistent run is refused WITH its certificate
///
/// [`purrdf_entail::EntailError::Inconsistent`] carries an
/// [`purrdf_entail::InconsistentRun`]: the witness AND the [`ReasoningReport`] for
/// everything the run had done when it stopped. This boundary used to render that through
/// `Display` alone — a one-line summary that read only the premise COUNT — so the caller
/// whose data was bad was the one caller who got no report at all, on every host, and the
/// witness's triples had no reader outside this workspace's own tests.
///
/// They travel in the message because a string-in/string-out boundary has exactly one
/// error channel and it is a string. The message is the `Display` one-liner, then the full
/// [`render_reasoning_report`] rendering on the following lines — whose
/// `inconsistency-premise` lines are the witness triples and whose first line is
/// [`REPORT_FORMAT_BANNER`], so a consumer that wants the certificate splits there rather
/// than parsing prose.
///
/// Every other variant is the ABSENCE of a run — an exhausted ceiling, a malformed
/// document, an unsatisfiable tableau — and has no report to carry, so it renders as its
/// own diagnostic and nothing is implied about a closure that was never assembled.
#[must_use]
pub fn render_entail_error(regime: &str, error: &EntailError) -> String {
    let head = format!("entailment regime \"{regime}\": {error}");
    match error {
        EntailError::Inconsistent(run) => {
            format!("{head}\n{}", render_reasoning_report(run.report()))
        }
        EntailError::Build(_)
        | EntailError::Parse(_)
        | EntailError::Evaluate(_)
        | EntailError::Chase(_)
        | EntailError::MalformedList(_)
        | EntailError::Unsatisfiable => head,
    }
}

/// The [`fmt::Display`] carrier for [`render_reasoning_report`].
///
/// A `Display` impl rather than a `String`-building function so every line goes
/// through one `writeln!` with `?`, which is what keeps the field order a single
/// readable block.
#[derive(Debug, Clone, Copy)]
struct RenderedReport<'a>(&'a ReasoningReport);

impl fmt::Display for RenderedReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let report = self.0;
        writeln!(f, "{REPORT_FORMAT_BANNER}")?;
        writeln!(f, "regime {}", regime_name(report.regime()))?;
        // Bound once: the completeness is DERIVED from the boundary list on every call, so
        // asking twice would recompute it — and the `missing` lines below borrow from it.
        let completeness = report.completeness();
        match &completeness {
            Completeness::Exact => writeln!(f, "completeness exact")?,
            Completeness::ExactWithinBoundaries => {
                writeln!(f, "completeness exact-within-boundaries")?;
            }
            Completeness::SoundIncomplete { missing } => {
                writeln!(f, "completeness sound-incomplete {}", missing.len())?;
            }
        }
        for rule in completeness.missing() {
            writeln!(f, "missing {}", rule.as_str())?;
        }
        // Beside `missing`, and for the same reason: both say how the calculus that ran
        // differs from the table the regime is named after, and a reader who sees only one
        // of them learns half of that.
        for rule in report.extensions() {
            writeln!(f, "extension {}", rule.as_str())?;
        }
        for &(rule, count) in report.rules_fired() {
            writeln!(f, "fired {} {count}", rule.as_str())?;
        }
        for boundary in report.boundaries() {
            writeln!(
                f,
                "boundary {} {}",
                boundary.construct().as_str(),
                boundary.reason()
            )?;
        }
        let budget = report.budget();
        writeln!(f, "budget join-steps {}", budget.join_steps())?;
        writeln!(f, "budget stored-facts {}", budget.stored_facts())?;
        writeln!(f, "budget term-arena-bytes {}", budget.term_arena_bytes())?;
        writeln!(f, "contract-hash {}", report.contract_hash().to_hex())?;
        writeln!(f, "withheld-surrogates {}", report.withheld_surrogates())?;
        match report.termination() {
            None => writeln!(f, "termination none")?,
            Some(certificate) => writeln!(
                f,
                "termination weakly-acyclic positions {} existential-edges {}",
                certificate.positions(),
                certificate.existential_edges()
            )?,
        }
        let Some(witness) = report.inconsistency() else {
            return writeln!(f, "inconsistency none");
        };
        writeln!(
            f,
            "inconsistency {} premises {}",
            witness.rule().as_str(),
            witness.premises().len()
        )?;
        // The graph whose CLOSURE refused, which is not the same claim as "every premise
        // is asserted here": a named graph is closed against the union of itself and the
        // default graph, so a premise may be asserted in either.
        match witness.graph() {
            None => writeln!(f, "inconsistency-graph default")?,
            Some(graph) => writeln!(f, "inconsistency-graph {}", emit(graph))?,
        }
        for premise in witness.premises() {
            writeln!(
                f,
                "inconsistency-premise {} {} {}",
                emit(premise.subject()),
                emit(premise.predicate()),
                emit(premise.object())
            )?;
        }
        Ok(())
    }
}

// ── The golden vector: one artifact, three hosts ────────────────────────────

/// The committed golden vector for this boundary, verbatim.
///
/// Compiled in with `include_str!`, so the C ABI and WASM crates consume the
/// SAME bytes as the Rust test rather than each growing a fixture that drifts
/// from the other two. It is a `const`, so a host that never mentions it (the
/// release WASM cdylib, say) pays nothing for it: `const` string data is only
/// emitted at a use site.
///
/// The artifact itself lives at `crates/validate/tests/fixtures/regime-boundary.vectors`.
pub const REGIME_GOLDEN_VECTORS: &str = include_str!("../tests/fixtures/regime-boundary.vectors");

/// One case of [`REGIME_GOLDEN_VECTORS`]: an input document, a regime, and the
/// two strings this boundary must produce for them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegimeVector {
    /// The case name, unique within the artifact.
    name: String,
    /// The regime's CLI spelling.
    regime: String,
    /// The input document (N-Quads).
    input: String,
    /// The regime's own rule document, empty for every regime that takes none.
    program: String,
    /// The canonical N-Quads the closure must serialize to.
    closure: String,
    /// The report rendering the run must produce.
    report: String,
}

impl RegimeVector {
    /// The case name, unique within the artifact.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The regime's CLI spelling.
    #[must_use]
    pub fn regime(&self) -> &str {
        &self.regime
    }

    /// The input document, as N-Quads.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }

    /// The `program` argument the case is materialized with — a RIF-in-XML rule
    /// document for `rif`, and the empty string for every other regime.
    #[must_use]
    pub fn program(&self) -> &str {
        &self.program
    }

    /// The canonical N-Quads [`materialize_to_nquads_string`] must return.
    #[must_use]
    pub fn closure(&self) -> &str {
        &self.closure
    }

    /// The rendered report [`materialize_to_nquads_string`] must return.
    #[must_use]
    pub fn report(&self) -> &str {
        &self.report
    }
}

/// Parse [`REGIME_GOLDEN_VECTORS`] into its cases.
///
/// # Errors
///
/// A malformed artifact — an unknown directive, a case missing a section, a
/// duplicate case name, or stray text outside a section — with the 1-based line
/// number.
pub fn regime_golden_vectors() -> Result<Vec<RegimeVector>, String> {
    parse_regime_vectors(REGIME_GOLDEN_VECTORS)
}

/// Run every case of [`REGIME_GOLDEN_VECTORS`] through
/// [`materialize_to_nquads_string`] and compare both outputs byte for byte.
///
/// This is the *one* assertion each host makes: the Rust test here, the C-ABI
/// crate's test, and the WASM crate's test all call this, so a divergence between
/// hosts is one failing artifact rather than three fixtures that quietly stopped
/// agreeing.
///
/// It also checks that the artifact still covers every regime in
/// [`REGIME_NAMES`] — all seven, since the boundary refuses none — so a truncated
/// artifact fails loudly instead of passing vacuously.
///
/// # Errors
///
/// A malformed artifact, a case that fails to materialize, a byte difference in
/// either output, or a regime the artifact no longer covers.
pub fn check_regime_golden_vectors() -> Result<(), String> {
    let cases = regime_golden_vectors()?;
    if cases.is_empty() {
        return Err("the regime golden vector artifact holds no cases".to_owned());
    }
    for case in &cases {
        let produced = materialize_to_nquads_string(case.regime(), case.input(), case.program())
            .map_err(|error| format!("case \"{}\": {error}", case.name()))?;
        if produced.nquads() != case.closure() {
            return Err(format!(
                "case \"{}\": closure mismatch\n--- expected ---\n{}--- produced ---\n{}",
                case.name(),
                case.closure(),
                produced.nquads()
            ));
        }
        if produced.report() != case.report() {
            return Err(format!(
                "case \"{}\": report mismatch\n--- expected ---\n{}--- produced ---\n{}",
                case.name(),
                case.report(),
                produced.report()
            ));
        }
    }
    for regime in REGIME_NAMES {
        if !cases.iter().any(|case| case.regime() == regime) {
            return Err(format!(
                "the regime golden vector artifact no longer covers regime \"{regime}\""
            ));
        }
    }
    Ok(())
}

/// The document [`check_inconsistent_refusal`] refuses.
///
/// Two disjoint classes and one instance of both — OWL 2 RL's `cax-dw`, whose three
/// premises are exactly these three triples, in this order.
pub const INCONSISTENT_DOCUMENT: &str = concat!(
    "<http://example.org/A> <http://www.w3.org/2002/07/owl#disjointWith> ",
    "<http://example.org/B> .\n",
    "<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ",
    "<http://example.org/A> .\n",
    "<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ",
    "<http://example.org/B> .\n",
);

/// Check that an INCONSISTENT input is refused WITH its certificate and its witness
/// triples.
///
/// The twin of [`check_regime_golden_vectors`], and made shared for the same reason: the
/// Rust test here, the C-ABI crate's test and the WASM crate's test all call it, so the
/// one host that quietly stopped carrying the evidence fails against the same expectation
/// as the other two.
///
/// It is a separate entry point rather than a tenth golden case because the golden artifact
/// pairs an input with the two strings a SUCCESSFUL closure produces, and an inconsistent
/// input has no closure. What it produces is a refusal, and the property under test is that
/// the refusal is not evidence-free: before this, every host mapped
/// [`purrdf_entail::EntailError::Inconsistent`] through `Display`, which reads only the
/// premise COUNT — so `inconsistency` was the constant `none` on every host, and
/// `WitnessTriple`'s three accessors had no caller outside this workspace's tests.
///
/// # Errors
///
/// A closure where a refusal was required, or a refusal that does not carry the report
/// banner, the witness rule, the graph whose closure refused, or all three premise triples.
pub fn check_inconsistent_refusal() -> Result<(), String> {
    let Err(refusal) = materialize_to_nquads_string("owl-rl", INCONSISTENT_DOCUMENT, "") else {
        return Err(
            "two disjoint classes with a shared instance must be refused by cax-dw".to_owned(),
        );
    };
    let required = [
        "knowledge base is inconsistent: cax-dw was satisfied by 3 asserted triples",
        REPORT_FORMAT_BANNER,
        "\nregime owl-rl\n",
        "\ninconsistency cax-dw premises 3\n",
        "\ninconsistency-graph default\n",
        "\ninconsistency-premise <http://example.org/A> \
         <http://www.w3.org/2002/07/owl#disjointWith> <http://example.org/B>\n",
        "\ninconsistency-premise <http://example.org/x> \
         <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A>\n",
        "\ninconsistency-premise <http://example.org/x> \
         <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/B>\n",
        // The run is described, not merely refused: it cost a budget and named a calculus.
        "\ncontract-hash ",
        "\nwithheld-surrogates 0\n",
    ];
    for fragment in required {
        if !refusal.contains(fragment) {
            return Err(format!(
                "the refusal must carry {fragment:?}\n--- refusal ---\n{refusal}"
            ));
        }
    }
    Ok(())
}

/// Which body a `@`-directive opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    /// The input document.
    Input,
    /// The regime's rule document.
    Program,
    /// The expected canonical N-Quads.
    Closure,
    /// The expected rendered report.
    Report,
}

/// The case under construction while [`parse_regime_vectors`] walks the artifact.
#[derive(Debug, Default)]
struct CaseBuilder {
    /// `@case`.
    name: Option<String>,
    /// `@regime`.
    regime: Option<String>,
    /// `@input`.
    input: Option<String>,
    /// `@program`; absent is the empty program, which is what six regimes take.
    program: Option<String>,
    /// `@closure`.
    closure: Option<String>,
    /// `@report`.
    report: Option<String>,
}

impl CaseBuilder {
    /// Finish the case, or say which section is missing.
    fn build(self, line: usize) -> Result<RegimeVector, String> {
        let missing = |what: &str| format!("line {line}: @end before @{what}");
        Ok(RegimeVector {
            name: self.name.ok_or_else(|| missing("case"))?,
            regime: self.regime.ok_or_else(|| missing("regime"))?,
            input: self.input.ok_or_else(|| missing("input"))?,
            program: self.program.unwrap_or_default(),
            closure: self.closure.ok_or_else(|| missing("closure"))?,
            report: self.report.ok_or_else(|| missing("report"))?,
        })
    }

    /// The body slot `section` fills.
    fn slot(&mut self, section: Section) -> &mut Option<String> {
        match section {
            Section::Input => &mut self.input,
            Section::Program => &mut self.program,
            Section::Closure => &mut self.closure,
            Section::Report => &mut self.report,
        }
    }
}

/// Parse the golden-vector artifact.
///
/// The format is line-oriented and deliberately dependency-free, so all three
/// hosts can read it with this one parser:
///
/// * A line starting with `@` is a directive: `@case <name>`, `@regime <name>`,
///   `@input`, `@program`, `@closure`, `@report`, `@end`.
/// * `@program` is the only optional one: omitting it is the empty program, which
///   is what every regime but `rif` takes.
/// * Every other line belongs to the body the last body-directive opened.
/// * Outside a body only blank lines and `#` comments are allowed, which is where
///   the artifact's SPDX header and its prose live.
///
/// A body line may therefore never begin with `@`. Neither N-Quads (whose terms
/// open with `<`, `_:` or `"`) nor a rendered report (whose lines open with a
/// fixed keyword) ever does.
fn parse_regime_vectors(text: &str) -> Result<Vec<RegimeVector>, String> {
    let mut cases: Vec<RegimeVector> = Vec::new();
    let mut builder = CaseBuilder::default();
    let mut open: Option<Section> = None;

    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let Some(directive) = raw.strip_prefix('@') else {
            match open {
                Some(section) => {
                    let body = builder.slot(section).get_or_insert_with(String::new);
                    body.push_str(raw);
                    body.push('\n');
                }
                None if raw.trim().is_empty() || raw.starts_with('#') => {}
                None => return Err(format!("line {line}: text outside any section: {raw}")),
            }
            continue;
        };
        let (keyword, argument) = directive.split_once(' ').unwrap_or((directive, ""));
        let argument = argument.trim();
        open = None;
        match keyword {
            "case" => {
                if argument.is_empty() {
                    return Err(format!("line {line}: @case needs a name"));
                }
                if builder.name.is_some() {
                    return Err(format!("line {line}: @case inside an unclosed case"));
                }
                if cases.iter().any(|case| case.name == argument) {
                    return Err(format!("line {line}: duplicate case name \"{argument}\""));
                }
                builder.name = Some(argument.to_owned());
            }
            "regime" => {
                if argument.is_empty() {
                    return Err(format!("line {line}: @regime needs a name"));
                }
                builder.regime = Some(argument.to_owned());
            }
            "input" | "program" | "closure" | "report" => {
                let section = match keyword {
                    "input" => Section::Input,
                    "program" => Section::Program,
                    "closure" => Section::Closure,
                    _ => Section::Report,
                };
                // Opened but empty is a legal body (an empty closure, say), so the
                // slot is seeded here rather than by the first body line.
                *builder.slot(section) = Some(String::new());
                open = Some(section);
            }
            "end" => {
                cases.push(core::mem::take(&mut builder).build(line)?);
            }
            other => return Err(format!("line {line}: unknown directive @{other}")),
        }
    }

    if builder.name.is_some() {
        return Err("the artifact ends inside an unclosed case (missing @end)".to_owned());
    }
    Ok(cases)
}

// ── The Description-Logic reasoning services ────────────────────────────────

/// The banner every [`DlCertificate`] rendering opens with.
///
/// Deliberately NOT [`REPORT_FORMAT_BANNER`]: a tableau certificate and a chase
/// report answer different questions, and a consumer that parsed one as the other
/// would read `decided` where it expected `exact`.
pub const DL_CERTIFICATE_BANNER: &str = "purrdf-dl-certificate 1";

/// The banner an OWL 2 profile certificate rendering opens with.
pub const PROFILE_CERTIFICATE_BANNER: &str = "purrdf-owl-profile-certificate 1";

/// The banner a module-extraction certificate rendering opens with.
pub const MODULE_CERTIFICATE_BANNER: &str = "purrdf-module-extraction 1";

/// The banner a justification certificate rendering opens with.
pub const JUSTIFICATION_CERTIFICATE_BANNER: &str = "purrdf-justification 1";

/// The banner a chase-proof certificate rendering opens with.
pub const CHASE_PROOF_CERTIFICATE_BANNER: &str = "purrdf-chase-proof 1";

/// The accepted locality-module method spellings, in the order an error lists them.
pub const MODULE_METHOD_NAMES: [&str; 3] = ["bot", "top", "star"];

/// The axiom kinds [`entails_to_string`] and [`justify_to_string`] can decide, in
/// the order the `axiom` line of a rendering spells them.
///
/// These are the [`DlAxiom`] variant names verbatim, so the rendering names the
/// same thing the Rust API does.
pub const AXIOM_KINDS: [&str; 8] = [
    "SubClassOf",
    "EquivalentClasses",
    "DisjointClasses",
    "ClassAssertion",
    "ObjectPropertyAssertion",
    "SameIndividual",
    "DifferentIndividuals",
    "SubObjectPropertyOf",
];

/// `rdf:type` — the OWL 2 RDF mapping's class-assertion predicate, and the
/// scaffold predicate [`parse_one_term`] parses a bare term through.
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
/// `rdfs:subClassOf` — the mapping's sub-class predicate.
const RDFS_SUBCLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
/// `rdfs:subPropertyOf` — the mapping's sub-property predicate.
const RDFS_SUBPROPERTY_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
/// `owl:equivalentClass` — the mapping's class-equivalence predicate.
const OWL_EQUIVALENT_CLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
/// `owl:disjointWith` — the mapping's class-disjointness predicate.
const OWL_DISJOINT_WITH: &str = "http://www.w3.org/2002/07/owl#disjointWith";
/// `owl:sameAs` — the mapping's individual-identity predicate.
const OWL_SAME_AS: &str = "http://www.w3.org/2002/07/owl#sameAs";
/// `owl:differentFrom` — the mapping's individual-difference predicate.
const OWL_DIFFERENT_FROM: &str = "http://www.w3.org/2002/07/owl#differentFrom";

/// One reasoning service's answer and the certificate of the run that produced it.
///
/// The DL twin of [`RegimeClosure`], and for the same reason: both strings cross an
/// FFI boundary and a positional pair is the wrong thing to get backwards there.
/// There is no certificate-free constructor and no second entry point that drops
/// the certificate — a caller that does not care must still bind it, because the
/// alternative is how "the reasoner says no" comes to mean "the reasoner ran out
/// of steps".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningAnswer {
    /// The service's answer, in the service's own line-oriented rendering.
    answer: String,
    /// The certificate of the run that produced it.
    certificate: String,
}

impl ReasoningAnswer {
    /// The service's answer.
    ///
    /// Line-oriented and byte-stable, in the grammar the service's entry point
    /// documents. A dataset-valued answer ([`extract_module_to_string`],
    /// [`justify_to_string`]) is canonical (RDFC-1.0) N-Quads instead.
    #[must_use]
    pub fn answer(&self) -> &str {
        &self.answer
    }

    /// The certificate of the run that produced [`Self::answer`].
    ///
    /// Never empty, and always terminated by the service's own honesty gate — the
    /// `overclaims` line for a tableau service, `conservative` for a module
    /// extraction — so a consumer can apply the gate without re-deriving it.
    #[must_use]
    pub fn certificate(&self) -> &str {
        &self.certificate
    }

    /// Consume the answer, yielding `(answer, certificate)`.
    #[must_use]
    pub fn into_parts(self) -> (String, String) {
        (self.answer, self.certificate)
    }
}

// ── Term syntax at the boundary ─────────────────────────────────────────────

/// Render `term` in N-Triples term syntax (`<iri>`, `_:label`, `"lex"@en`,
/// `<<( s p o )>>`).
///
/// The escaping is [`purrdf_core::emit_term`]'s, so a term rendered here and the
/// same term rendered by the native serializers escape identically. Triple terms
/// recurse HERE rather than through `emit_term`'s owned model, because the owned
/// model requires a triple term's predicate to be an IRI and this function must be
/// total over [`TermValue`].
///
/// N-Triples terms are self-delimiting — `<…>` ends at the unescaped `>`, `_:…` at
/// whitespace, `"…"` at the unescaped closing quote — which is what makes a
/// two-term line like `subclass <C> <D>` unambiguous even though a literal's
/// lexical form may contain a space.
fn emit(term: &TermValue) -> String {
    match term {
        TermValue::Iri(iri) => emit_term(&RdfTerm::iri(iri.clone())),
        TermValue::Blank { label, scope } => emit_term(&RdfTerm::blank_node(
            scope.qualify_label(label).into_owned(),
        )),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => emit_term(&RdfTerm::literal(RdfLiteral {
            lexical_form: lexical_form.clone(),
            datatype: Some(datatype.clone()),
            language: language.clone(),
            direction: *direction,
        })),
        TermValue::Triple { s, p, o } => match p.as_iri() {
            Some(predicate) => emit_term(&RdfTerm::triple(RdfTriple::new(
                to_owned_term(s),
                predicate.to_owned(),
                to_owned_term(o),
            ))),
            // A triple term whose predicate is not an IRI is not a well-formed RDF
            // triple, so the owned model cannot hold it. Rendering it structurally
            // is the honest option: the caller sees what the term actually is
            // rather than a silently dropped component.
            None => format!("<<( {} {} {} )>>", emit(s), emit(p), emit(o)),
        },
    }
}

/// The owned-model twin of a non-recursive [`TermValue`], for [`emit`].
///
/// A triple term nests through [`emit`]'s own recursion, so this only has to be
/// correct for the three flat kinds; a nested triple term is rebuilt from its
/// rendering rather than from this, which is why the fallback is an IRI-shaped
/// term carrying the rendering instead of a panic.
fn to_owned_term(term: &TermValue) -> RdfTerm {
    match term {
        TermValue::Iri(iri) => RdfTerm::iri(iri.clone()),
        TermValue::Blank { label, scope } => {
            RdfTerm::blank_node(scope.qualify_label(label).into_owned())
        }
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => RdfTerm::literal(RdfLiteral {
            lexical_form: lexical_form.clone(),
            datatype: Some(datatype.clone()),
            language: language.clone(),
            direction: *direction,
        }),
        TermValue::Triple { s, p, o } => RdfTerm::triple(RdfTriple::new(
            to_owned_term(s),
            p.as_iri().unwrap_or_default().to_owned(),
            to_owned_term(o),
        )),
    }
}

/// Parse ONE N-Triples term — an IRI or a blank node — from `text`.
///
/// Implemented by parsing `text` in SUBJECT position of a scaffold triple rather
/// than by a hand-rolled term parser, so the boundary's term syntax is exactly the
/// native codec's and cannot drift from it. The scaffold's predicate and object are
/// `rdf:type`, an RDF specification IRI: it is read and discarded, never emitted,
/// and PurRDF mints nothing here.
///
/// A literal is refused by the codec itself, which is the right answer: a class, an
/// individual and a property are all NAMES, and a literal is not a name.
fn parse_one_term(text: &str) -> Result<TermValue, String> {
    let scaffold = format!("{} <{RDF_TYPE}> <{RDF_TYPE}> .\n", text.trim());
    let dataset = purrdf_rdf::parse_dataset(scaffold.as_bytes(), INPUT_MEDIA_TYPE, None)
        .map_err(|diagnostic| format!("\"{text}\" is not one N-Triples term: {diagnostic}"))?;
    let mut quads = dataset.quads();
    let Some(quad) = quads.next() else {
        return Err(format!("\"{text}\" is not one N-Triples term: it is empty"));
    };
    if quads.next().is_some() {
        return Err(format!(
            "\"{text}\" is not ONE N-Triples term: it parses as more than one"
        ));
    }
    // The scaffold's own predicate, object and (absent) graph must come back
    // unchanged. They do not when `text` was more than one term: `<A> <B>` makes
    // the scaffold a four-term N-Quads statement whose graph is the trailing
    // `rdf:type`, which would otherwise be read as the single term `<A>`.
    let intact = quad.g.is_none()
        && dataset.term_value(quad.p).as_iri() == Some(RDF_TYPE)
        && dataset.term_value(quad.o).as_iri() == Some(RDF_TYPE);
    if !intact {
        return Err(format!(
            "\"{text}\" is not ONE N-Triples term: it parses as more than one"
        ));
    }
    Ok(dataset.term_value(quad.s))
}

/// Parse a newline-separated signature: one N-Triples term per non-blank line.
fn parse_signature(text: &str) -> Result<Vec<TermValue>, String> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_one_term)
        .collect()
}

/// Parse ONE N-Quads statement into `(graph, subject, predicate, object)`.
fn parse_one_statement(
    text: &str,
) -> Result<(Option<TermValue>, TermValue, TermValue, TermValue), String> {
    let dataset = purrdf_rdf::parse_dataset(text.as_bytes(), INPUT_MEDIA_TYPE, None)
        .map_err(|diagnostic| format!("\"{text}\" is not one N-Quads statement: {diagnostic}"))?;
    let mut quads = dataset.quads();
    let Some(quad) = quads.next() else {
        return Err(format!(
            "\"{text}\" is not one N-Quads statement: it is empty"
        ));
    };
    if quads.next().is_some() {
        return Err(format!(
            "\"{text}\" is not ONE N-Quads statement: it parses as more than one"
        ));
    }
    Ok((
        quad.g.map(|g| dataset.term_value(g)),
        dataset.term_value(quad.s),
        dataset.term_value(quad.p),
        dataset.term_value(quad.o),
    ))
}

/// Read an axiom written as ONE triple of the **OWL 2 RDF mapping**.
///
/// No mini-language is invented here: every [`DlAxiom`] variant already HAS an RDF
/// spelling, and it is the one the reasoner's own reverse mapping reads. Seven
/// reserved predicates select the seven named variants and every other predicate is
/// an object-property assertion — exactly the OWL 2 mapping's own dispatch:
///
/// | triple | axiom |
/// |---|---|
/// | `C rdfs:subClassOf D` | `SubClassOf` |
/// | `C owl:equivalentClass D` | `EquivalentClasses` |
/// | `C owl:disjointWith D` | `DisjointClasses` |
/// | `a rdf:type C` | `ClassAssertion` |
/// | `a owl:sameAs b` | `SameIndividual` |
/// | `a owl:differentFrom b` | `DifferentIndividuals` |
/// | `p rdfs:subPropertyOf q` | `SubObjectPropertyOf` |
/// | `a p b` (anything else) | `ObjectPropertyAssertion` |
///
/// An axiom names no graph, so a statement that does is refused rather than having
/// its graph silently dropped.
fn parse_axiom(text: &str) -> Result<DlAxiom, String> {
    let (graph, subject, predicate, object) = parse_one_statement(text)?;
    if graph.is_some() {
        return Err(format!(
            "\"{text}\" names a graph; an axiom is one triple and is not graph-scoped"
        ));
    }
    Ok(match predicate.as_iri() {
        Some(RDFS_SUBCLASS_OF) => DlAxiom::SubClassOf {
            sub: subject,
            sup: object,
        },
        Some(OWL_EQUIVALENT_CLASS) => DlAxiom::EquivalentClasses {
            left: subject,
            right: object,
        },
        Some(OWL_DISJOINT_WITH) => DlAxiom::DisjointClasses {
            left: subject,
            right: object,
        },
        Some(RDF_TYPE) => DlAxiom::ClassAssertion {
            individual: subject,
            class: object,
        },
        Some(OWL_SAME_AS) => DlAxiom::SameIndividual {
            left: subject,
            right: object,
        },
        Some(OWL_DIFFERENT_FROM) => DlAxiom::DifferentIndividuals {
            left: subject,
            right: object,
        },
        Some(RDFS_SUBPROPERTY_OF) => DlAxiom::SubObjectPropertyOf {
            sub: subject,
            sup: object,
        },
        _ => DlAxiom::ObjectPropertyAssertion {
            subject,
            property: predicate,
            object,
        },
    })
}

/// The [`AXIOM_KINDS`] name of `axiom`, and its terms in declaration order.
fn axiom_parts(axiom: &DlAxiom) -> (&'static str, Vec<&TermValue>) {
    match axiom {
        DlAxiom::SubClassOf { sub, sup } => ("SubClassOf", vec![sub, sup]),
        DlAxiom::EquivalentClasses { left, right } => ("EquivalentClasses", vec![left, right]),
        DlAxiom::DisjointClasses { left, right } => ("DisjointClasses", vec![left, right]),
        DlAxiom::ClassAssertion { individual, class } => {
            ("ClassAssertion", vec![individual, class])
        }
        DlAxiom::ObjectPropertyAssertion {
            subject,
            property,
            object,
        } => ("ObjectPropertyAssertion", vec![subject, property, object]),
        DlAxiom::SameIndividual { left, right } => ("SameIndividual", vec![left, right]),
        DlAxiom::DifferentIndividuals { left, right } => {
            ("DifferentIndividuals", vec![left, right])
        }
        DlAxiom::SubObjectPropertyOf { sub, sup } => ("SubObjectPropertyOf", vec![sub, sup]),
    }
}

/// Write `axiom` as an `axiom <kind>` line followed by one `term <t>` line each.
fn write_axiom(axiom: &DlAxiom, out: &mut String) {
    let (kind, terms) = axiom_parts(axiom);
    out.push_str("axiom ");
    out.push_str(kind);
    out.push('\n');
    for term in terms {
        out.push_str("term ");
        out.push_str(&emit(term));
        out.push('\n');
    }
}

// ── Certificate rendering ───────────────────────────────────────────────────

/// Render a [`DlCertificate`] to the boundary's byte-stable textual form.
///
/// The grammar, in emission order — one fact per line, `\n`-terminated:
///
/// ```text
/// purrdf-dl-certificate 1
/// service <name>
/// completeness decided | completeness decided-within-boundaries | completeness budget-exhausted
/// boundary <construct> <reason>           (0..n, Construct declaration order)
/// steps <n>
/// budget <n>
/// decisions <n>
/// overclaims false | overclaims true
/// ```
///
/// `completeness` is the DL lane's own notion and NOT the chase's: the middle value
/// says the search finished and the reverse mapping still could not turn every
/// axiom of the supplied ontology into a DL clause, so a `False` answer under it
/// means only "not entailed by what was read". `budget-exhausted` says at least one
/// tableau run reached its step cap, which is why a boolean service answers
/// `unknown` rather than `false` — reporting a resource limit as an entailment is
/// the defect this line exists to prevent.
///
/// `steps` is a count of saturation rounds and `budget` the per-DECISION cap, so
/// `steps` may legitimately exceed `budget` when `decisions` is greater than one.
/// Neither is a clock reading, so the rendering is identical on `wasm32`.
///
/// `overclaims` is derivable from `completeness` and the `boundary` lines and is emitted
/// anyway, so a consumer on the far side of an FFI boundary does not have to re-derive it.
/// Note that the CHASE report has no such line: [`ReasoningReport`] stores no completeness
/// field to contradict its boundary list, so there is nothing there to gate. A
/// [`DlCertificate`] does store its verdict beside its boundaries, which is why this one
/// stays.
#[must_use]
pub fn render_dl_certificate(service: &str, certificate: &DlCertificate) -> String {
    let mut out = String::new();
    out.push_str(DL_CERTIFICATE_BANNER);
    out.push('\n');
    let _ = writeln!(out, "service {service}");
    let completeness = match certificate.completeness() {
        DlCompleteness::Decided => "decided",
        DlCompleteness::DecidedWithinBoundaries => "decided-within-boundaries",
        DlCompleteness::BudgetExhausted => "budget-exhausted",
    };
    let _ = writeln!(out, "completeness {completeness}");
    for boundary in certificate.boundaries() {
        let _ = writeln!(
            out,
            "boundary {} {}",
            boundary.construct().as_str(),
            boundary.reason()
        );
    }
    let _ = writeln!(out, "steps {}", certificate.steps());
    let _ = writeln!(out, "budget {}", certificate.budget());
    let _ = writeln!(out, "decisions {}", certificate.decisions());
    let _ = writeln!(out, "overclaims {}", certificate.overclaims());
    out
}

/// The rendering of a three-valued [`Verdict`]: `true`, `false` or `unknown`.
///
/// `unknown` is never collapsed to `false`. That is the whole point of the third
/// value, and it is the one substitution a string boundary could make silently.
fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::True => "true",
        Verdict::False => "false",
        Verdict::Unknown => "unknown",
    }
}

/// Open a [`Reasoner`] over `document`, narrowed to `step_cap` when that is non-zero.
///
/// `step_cap` can only NARROW: [`Reasoner::with_step_cap`] clamps to the knowledge
/// base's own cap, which is a pure function of its size. `0` means "do not narrow"
/// rather than "a cap of zero steps", because a zero cap would exhaust every
/// decision and make the parameter a footgun at three language boundaries.
fn open_reasoner(document: &str, step_cap: u32) -> Result<Reasoner, String> {
    let dataset = purrdf_rdf::parse_dataset(document.as_bytes(), INPUT_MEDIA_TYPE, None)
        .map_err(|diagnostic| diagnostic.to_string())?;
    let reasoner = Reasoner::new(&dataset).map_err(|error| format!("reasoner: {error}"))?;
    Ok(if step_cap == 0 {
        reasoner
    } else {
        reasoner.with_step_cap(u64::from(step_cap))
    })
}

/// The message a service that cannot reason over an unsatisfiable ontology is
/// refused with.
///
/// Named rather than answered: every class subsumes every other in an ontology with
/// no model, so an answer would be a complete graph carrying no information. The
/// message points at the one service that DOES answer for such an ontology.
fn service_error(service: &str, error: &EntailError) -> String {
    match error {
        EntailError::Unsatisfiable => format!(
            "{service}: the ontology has no model, so every answer would be vacuous; \
             consistency_to_string reports this as `consistency false`"
        ),
        other => format!("{service}: {other}"),
    }
}

// ── The services ────────────────────────────────────────────────────────────

/// Is the ontology consistent — does it have a model at all?
///
/// The one DL service that does not fail on an unsatisfiable ontology, because it
/// is the service that DETECTS one.
///
/// The answer is one line, `consistency true | false | unknown`; `unknown` means
/// the tableau reached its step cap, which the certificate's
/// `completeness budget-exhausted` line says in its own words.
///
/// # Errors
///
/// A malformed document (the native codec's own diagnostic), or a reverse mapping
/// that fails on a malformed OWL class-expression graph.
///
/// ```
/// use purrdf_validate::regime::consistency_to_string;
///
/// let data = "<http://example.org/x> \
///     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .\n";
/// let decided = consistency_to_string(data, 0).expect("reverse-maps");
/// assert_eq!(decided.answer(), "consistency true\n");
/// // The certificate is never optional and never claims more than it decided.
/// assert!(decided.certificate().starts_with("purrdf-dl-certificate 1\n"));
/// assert!(decided.certificate().ends_with("overclaims false\n"));
/// ```
pub fn consistency_to_string(document: &str, step_cap: u32) -> Result<ReasoningAnswer, String> {
    let reasoner = open_reasoner(document, step_cap)?;
    let (verdict, certificate) = reasoner.consistency().into_parts();
    Ok(ReasoningAnswer {
        answer: format!("consistency {}\n", verdict_name(verdict)),
        certificate: render_dl_certificate("consistency", &certificate),
    })
}

/// The subsumption hierarchy over the ontology's named classes.
///
/// The answer's grammar, in emission order — each block in the classifier's own
/// sorted, dataset-independent term order:
///
/// ```text
/// equivalent <C> <D>      (0..n, each unordered pair once)
/// subclass <C> <D>        (0..n, the FULL transitively-closed relation)
/// direct <C> <D>          (0..n, the transitive reduction)
/// unsatisfiable <C>       (0..n, the classes established equivalent to owl:Nothing)
/// ```
///
/// `subclass` and `direct` are both emitted because they are different facts:
/// `direct` is "direct as far as this run decided", which weakens under a
/// `budget-exhausted` certificate while every listed pair stays a genuine
/// subsumption. Dropping either would make the other's meaning ambiguous.
///
/// # Errors
///
/// A malformed document, or an ontology with no model — every class then subsumes
/// every other and the hierarchy would be a complete graph.
pub fn classify_to_string(document: &str, step_cap: u32) -> Result<ReasoningAnswer, String> {
    let reasoner = open_reasoner(document, step_cap)?;
    let (hierarchy, certificate) = reasoner
        .classify()
        .map_err(|error| service_error("classify", &error))?
        .into_parts();
    let mut answer = String::new();
    for (left, right) in hierarchy.equivalences() {
        let _ = writeln!(answer, "equivalent {} {}", emit(left), emit(right));
    }
    for (sub, sup) in hierarchy.subsumptions() {
        let _ = writeln!(answer, "subclass {} {}", emit(sub), emit(sup));
    }
    for (sub, sup) in hierarchy.direct_subsumptions() {
        let _ = writeln!(answer, "direct {} {}", emit(sub), emit(sup));
    }
    for class in hierarchy.unsatisfiable() {
        let _ = writeln!(answer, "unsatisfiable {}", emit(class));
    }
    Ok(ReasoningAnswer {
        answer,
        certificate: render_dl_certificate("classify", &certificate),
    })
}

/// The entailed types of the ontology's named individuals, and the most specific
/// of them.
///
/// The answer's grammar, in emission order:
///
/// ```text
/// type <a> <C>            (0..n, every established a : C)
/// direct-type <a> <C>     (0..n, the most specific of them)
/// ```
///
/// Both blocks are emitted for the reason [`classify_to_string`] emits both of its
/// subsumption blocks: `direct-type` is a statement about the hierarchy as this run
/// decided it, and `type` is the answer set.
///
/// # Errors
///
/// A malformed document, or an ontology with no model.
pub fn realize_to_string(document: &str, step_cap: u32) -> Result<ReasoningAnswer, String> {
    let reasoner = open_reasoner(document, step_cap)?;
    let (realization, certificate) = reasoner
        .realize()
        .map_err(|error| service_error("realize", &error))?
        .into_parts();
    let mut answer = String::new();
    for (individual, class) in realization.types() {
        let _ = writeln!(answer, "type {} {}", emit(individual), emit(class));
    }
    for (individual, class) in realization.direct_types() {
        let _ = writeln!(answer, "direct-type {} {}", emit(individual), emit(class));
    }
    Ok(ReasoningAnswer {
        answer,
        certificate: render_dl_certificate("realize", &certificate),
    })
}

/// The named individuals entailed to be instances of `class`.
///
/// `class` is ONE N-Triples term — `<iri>` or `_:label`. A class the ontology never
/// mentions is not an error: it is an atomic name no axiom constrains, which is what
/// the Direct Semantics says it is, and the (empty) answer for it is a real answer.
///
/// The answer's grammar: `instance <a>`, zero or more, sorted.
///
/// # Errors
///
/// A malformed document, a `class` that is not one N-Triples term, or an ontology
/// with no model — every individual would then be an instance of every class.
pub fn instances_to_string(
    document: &str,
    class: &str,
    step_cap: u32,
) -> Result<ReasoningAnswer, String> {
    let term = parse_one_term(class)?;
    let mut reasoner = open_reasoner(document, step_cap)?;
    let (individuals, certificate) = reasoner
        .instances(&term)
        .map_err(|error| service_error("instances", &error))?
        .into_parts();
    let mut answer = String::new();
    for individual in &individuals {
        let _ = writeln!(answer, "instance {}", emit(individual));
    }
    Ok(ReasoningAnswer {
        answer,
        certificate: render_dl_certificate("instances", &certificate),
    })
}

/// Does the ontology entail `axiom`?
///
/// `axiom` is ONE triple of the OWL 2 RDF mapping — see this module's `parse_axiom` for the
/// seven reserved predicates and the object-property-assertion default.
///
/// The answer's grammar, in emission order:
///
/// ```text
/// entails true | entails false | entails unknown
/// axiom <kind>            (one of AXIOM_KINDS)
/// term <t>                (2..3, in the axiom's own declaration order)
/// ```
///
/// The axiom is echoed because the predicate DISPATCHES: a caller that meant an
/// object-property assertion and wrote `rdfs:subClassOf` can see which axiom was
/// actually decided rather than inferring it from a verdict.
///
/// # Errors
///
/// A malformed document, an `axiom` that is not one triple, an axiom statement that
/// names a graph, or an ontology with no model — in which case every axiom is
/// entailed and the answer would be worthless.
///
/// ```
/// use purrdf_validate::regime::entails_to_string;
///
/// let data = "<http://example.org/Cat> \
///     <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/Animal> .\n\
///     <http://example.org/tom> \
///     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Cat> .\n";
/// // Not asserted, and entailed.
/// let asked = "<http://example.org/tom> \
///     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Animal> .\n";
/// let decided = entails_to_string(data, asked, 0).expect("decides");
/// assert!(decided.answer().starts_with("entails true\n"));
/// assert!(decided.answer().contains("\naxiom ClassAssertion\n"));
/// ```
pub fn entails_to_string(
    document: &str,
    axiom: &str,
    step_cap: u32,
) -> Result<ReasoningAnswer, String> {
    let parsed = parse_axiom(axiom)?;
    let mut reasoner = open_reasoner(document, step_cap)?;
    let (verdict, certificate) = reasoner
        .entails(&parsed)
        .map_err(|error| service_error("entails", &error))?
        .into_parts();
    let mut answer = format!("entails {}\n", verdict_name(verdict));
    write_axiom(&parsed, &mut answer);
    Ok(ReasoningAnswer {
        answer,
        certificate: render_dl_certificate("entails", &certificate),
    })
}

/// Which OWL 2 profiles the ontology is provably in, and what blocked the others.
///
/// Purely syntactic — no tableau, no closure, no budget — which is why this is the
/// one service whose certificate is NOT a [`DlCertificate`]: there is no search to
/// report the completeness of, and rendering a fabricated `decided` beside a
/// certification that never ran a tableau would be a lie of the exact kind the DL
/// certificate exists to prevent.
///
/// The answer's grammar: `certified <profile>`, most restrictive first
/// (`EL`, `QL`, `RL`, `DL`, `Full`). `Full` is always certified.
///
/// The certificate's grammar:
///
/// ```text
/// purrdf-owl-profile-certificate 1
/// service profile
/// violation <profile> <term> <subject> <reason…>   (0..n, sorted)
/// certifies-el true|false
/// certifies-ql true|false
/// certifies-rl true|false
/// certifies-dl true|false
/// certifies-full true|false
/// one-directional true
/// ```
///
/// The first three fields of a `violation` line are self-delimiting N-Triples terms
/// and a profile name; the rest of the line is the reason, exactly as
/// [`render_reasoning_report`]'s `boundary` lines are shaped.
///
/// `one-directional` is this certificate's honesty gate and is a constant `true`:
/// a certification PROVES membership, and a violation does not prove
/// non-membership — it says only that the syntactic analysis could not place this
/// occurrence somewhere legal. A consumer must not read a violation as a proof of
/// exclusion, and the line says so on every certificate rather than in prose the
/// consumer may never read.
///
/// # Errors
///
/// A malformed document (the native codec's own diagnostic).
///
/// ```
/// use purrdf_validate::regime::profile_to_string;
///
/// let data = "<http://example.org/Cat> \
///     <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/Animal> .\n";
/// let certified = profile_to_string(data).expect("parses");
/// // A bare sub-class axiom is in every profile, most restrictive first.
/// assert_eq!(certified.answer().lines().next(), Some("certified EL"));
/// assert!(certified.certificate().ends_with("one-directional true\n"));
/// ```
pub fn profile_to_string(document: &str) -> Result<ReasoningAnswer, String> {
    let dataset = purrdf_rdf::parse_dataset(document.as_bytes(), INPUT_MEDIA_TYPE, None)
        .map_err(|diagnostic| diagnostic.to_string())?;
    let certificate = profile(&dataset);
    Ok(ReasoningAnswer {
        answer: render_profile_answer(&certificate),
        certificate: render_profile_certificate(&certificate),
    })
}

/// The `certified <profile>` block of [`profile_to_string`]'s answer.
fn render_profile_answer(certificate: &ProfileCertificate) -> String {
    let mut answer = String::new();
    for profile in certificate.certified() {
        let _ = writeln!(answer, "certified {}", profile.as_str());
    }
    answer
}

/// The violation block and the per-profile gate of [`profile_to_string`].
fn render_profile_certificate(certificate: &ProfileCertificate) -> String {
    let mut out = String::new();
    out.push_str(PROFILE_CERTIFICATE_BANNER);
    out.push('\n');
    out.push_str("service profile\n");
    for violation in certificate.violations() {
        let _ = writeln!(
            out,
            "violation {} {} {} {}",
            violation.profile().as_str(),
            emit(violation.term()),
            emit(violation.subject()),
            violation.reason()
        );
    }
    // A dense per-profile answer beside the sparse violation list, so a consumer
    // asking "is this RL?" reads one line rather than searching for the ABSENCE of
    // a violation — which is the reading a sparse list makes easy to get wrong.
    for profile in OwlProfile::ALL {
        let _ = writeln!(
            out,
            "certifies-{} {}",
            profile.as_str().to_ascii_lowercase(),
            certificate.certifies(profile)
        );
    }
    out.push_str("one-directional true\n");
    out
}

/// The locality module of the ontology for a seed signature.
///
/// `signature` is one N-Triples term per non-blank line. `method` is `bot`, `top`
/// or `star` — see [`MODULE_METHOD_NAMES`].
///
/// The answer is the extracted module as canonical (RDFC-1.0) N-Quads, exactly as
/// [`materialize_to_nquads_string`]'s closure is, so the two dataset-valued
/// services serialize through one path.
///
/// The certificate's grammar:
///
/// ```text
/// purrdf-module-extraction 1
/// service extract-module
/// method BOT | TOP | STAR
/// axioms <n>
/// signature <t>                    (0..n, the signature the fixpoint CLOSED to)
/// conservative-keep <s> <p>        (0..n)
/// conservative false | conservative true
/// ```
///
/// `conservative` is this certificate's honesty gate: `true` says at least one
/// triple was kept because the extractor could not decide its locality exactly, so
/// the module is a SUPERSET rather than the minimal one. That is the sound
/// direction of the doctrine, made visible instead of silently inflating a module a
/// caller is sizing.
///
/// # Errors
///
/// A malformed document, a signature line that is not one N-Triples term, an
/// unknown `method` spelling (the message names the accepted set), or a module that
/// cannot be frozen into a dataset.
pub fn extract_module_to_string(
    document: &str,
    signature: &str,
    method: &str,
) -> Result<ReasoningAnswer, String> {
    let notion = parse_module_method(method)?;
    let seed = parse_signature(signature)?;
    let dataset = purrdf_rdf::parse_dataset(document.as_bytes(), INPUT_MEDIA_TYPE, None)
        .map_err(|diagnostic| diagnostic.to_string())?;
    let extraction = extract_module(&dataset, &seed, notion)
        .map_err(|error| format!("extract-module: {error}"))?;

    let mut certificate = String::new();
    certificate.push_str(MODULE_CERTIFICATE_BANNER);
    certificate.push('\n');
    certificate.push_str("service extract-module\n");
    let _ = writeln!(certificate, "method {}", extraction.method().as_str());
    let _ = writeln!(certificate, "axioms {}", extraction.axioms());
    for term in extraction.signature() {
        let _ = writeln!(certificate, "signature {}", emit(term));
    }
    for keep in extraction.conservative_keeps() {
        let _ = writeln!(
            certificate,
            "conservative-keep {} {}",
            emit(keep.subject()),
            emit(keep.predicate())
        );
    }
    let _ = writeln!(
        certificate,
        "conservative {}",
        !extraction.conservative_keeps().is_empty()
    );

    Ok(ReasoningAnswer {
        answer: purrdf_rdf::canonical_flat_nquads(extraction.module().as_ref())?,
        certificate,
    })
}

/// Parse a locality-module method from its CLI spelling.
fn parse_module_method(name: &str) -> Result<ModuleMethod, String> {
    match name {
        "bot" => Ok(ModuleMethod::Bot),
        "top" => Ok(ModuleMethod::Top),
        "star" => Ok(ModuleMethod::Star),
        other => Err(format!(
            "unknown locality-module method \"{other}\"; accepted: {}",
            MODULE_METHOD_NAMES.join(", ")
        )),
    }
}

/// WHY a Description-Logic axiom is entailed: a minimal subset of the ontology that
/// still entails it.
///
/// A tableau performs no derivation steps, so this is a JUSTIFICATION and
/// deliberately not called a proof — see [`explain_conclusion_to_string`] for the
/// chase lane's genuinely derivational explanation and for why the two are
/// different kinds of thing rather than two spellings of one.
///
/// The answer is the justification's axioms as canonical (RDFC-1.0) N-Quads: a
/// justification introduces no term at all, so it is emitted as an ordinary RDF 1.2
/// dataset holding exactly the axioms already present in the input.
///
/// The certificate's grammar:
///
/// ```text
/// purrdf-justification 1
/// service justify
/// axiom <kind>
/// term <t>                   (2..3)
/// axioms <n>
/// decisions <n>
/// digest <64 lowercase hex>
/// sufficient true | false
/// minimal true | false
/// overclaims false | overclaims true
/// ```
///
/// `sufficient` and `minimal` are **re-decided here**, over the justification alone
/// and over each of its one-axiom-smaller subsets. They do not consult the search
/// that found the justification and cannot be misled by it, which is what makes them
/// a check rather than a restatement. `overclaims` is `true` unless both hold: a
/// subset that does not entail, or that carries an axiom the entailment does not
/// need, is a weaker answer than "a justification" and says so.
///
/// `digest` is BLAKE3 over the canonical N-Quads of the justification — a CONTENT
/// digest, never an IRI, because PurRDF mints no vocabulary.
///
/// # Errors
///
/// A malformed document; an `axiom` that is not one triple; an ontology that does
/// not entail the axiom (a subset of a non-entailing ontology does not entail
/// either, so the search would return the empty set, which reads as "nothing is
/// needed" and means the opposite); or a tableau that ran out of step budget
/// deciding the axiom, leaving no answer to shrink against.
///
/// ```
/// use purrdf_validate::regime::justify_to_string;
///
/// let sub = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
/// let data = format!(
///     "<http://example.org/Cat> <{sub}> <http://example.org/Mammal> .\n\
///      <http://example.org/Mammal> <{sub}> <http://example.org/Animal> .\n\
///      <http://example.org/Fish> <{sub}> <http://example.org/Animal> .\n"
/// );
/// let asked = format!("<http://example.org/Cat> <{sub}> <http://example.org/Animal> .\n");
/// let why = justify_to_string(&data, &asked).expect("entailed");
/// // The chain, and NOT the sibling.
/// assert_eq!(why.answer().lines().count(), 2);
/// assert!(why.certificate().contains("\nsufficient true\n"));
/// assert!(why.certificate().contains("\nminimal true\n"));
/// assert!(why.certificate().ends_with("overclaims false\n"));
/// ```
pub fn justify_to_string(document: &str, axiom: &str) -> Result<ReasoningAnswer, String> {
    let parsed = parse_axiom(axiom)?;
    let dataset = purrdf_rdf::parse_dataset(document.as_bytes(), INPUT_MEDIA_TYPE, None)
        .map_err(|diagnostic| diagnostic.to_string())?;
    let justification = justify(&dataset, &parsed).map_err(|error| format!("justify: {error}"))?;
    Ok(ReasoningAnswer {
        answer: purrdf_rdf::canonical_flat_nquads(justification.ontology().as_ref())?,
        certificate: render_justification(&justification)?,
    })
}

/// Render a [`Justification`], re-deciding both halves of its claim.
fn render_justification(justification: &Justification) -> Result<String, String> {
    let sufficient = justification
        .is_sufficient()
        .map_err(|error| format!("justify: re-deciding sufficiency: {error}"))?;
    let minimal = justification
        .is_minimal()
        .map_err(|error| format!("justify: re-deciding minimality: {error}"))?;
    let mut out = String::new();
    out.push_str(JUSTIFICATION_CERTIFICATE_BANNER);
    out.push('\n');
    out.push_str("service justify\n");
    write_axiom(justification.axiom(), &mut out);
    let _ = writeln!(out, "axioms {}", justification.axioms());
    let _ = writeln!(out, "decisions {}", justification.decisions());
    let _ = writeln!(out, "digest {}", justification.digest_hex());
    let _ = writeln!(out, "sufficient {sufficient}");
    let _ = writeln!(out, "minimal {minimal}");
    let _ = writeln!(out, "overclaims {}", !(sufficient && minimal));
    Ok(out)
}

/// WHY a chase conclusion holds: the derivation, re-derived from the clause program.
///
/// The chase's explanation is a DERIVATION — which rule, from which premises — and
/// is a different kind of thing from [`justify_to_string`]'s justification. Giving
/// them one entry point would let a caller write code that treats a tableau answer
/// as though a rule had fired.
///
/// `conclusion` is ONE N-Quads statement. Its graph, if it names one, selects the
/// closure to explain under the dataset semantics
/// [`materialize_to_nquads_string`] documents: no graph is the default graph closed
/// against itself, and a named graph is closed against the union of itself and the
/// default graph. A conclusion drawn in one graph therefore has an explanation in
/// that graph's run and, in general, in no other.
///
/// The answer's grammar, in emission order:
///
/// ```text
/// asserted true | asserted false
/// steps <n>
/// rule <rule-id>            (0..n, specification table order, deduplicated)
/// ```
///
/// `asserted true` is a real explanation and a checkable one: a given triple is
/// explained by the fact that it is given, and the check runs against the SEEDED
/// store, so a derived fact cannot be passed off as a given.
///
/// The certificate's grammar:
///
/// ```text
/// purrdf-chase-proof 1
/// service explain-conclusion
/// regime <cli-spelling>
/// graph default | graph <t>
/// conclusion-subject <t>
/// conclusion-predicate <t>
/// conclusion-object <t>
/// derived-subject <surface>
/// derived-predicate <surface>
/// derived-object <surface>
/// digest <64 lowercase hex>
/// proof-term-bytes <n>
/// checked true | checked false
/// overclaims false | overclaims true
/// ```
///
/// The `derived-*` lines are what the CHECKER re-derived from the proof term and
/// the clause program — not what the proof claims. They are emitted beside the
/// `conclusion-*` lines precisely so the two can be compared: a proof whose stated
/// conclusion is not the one its own premises license shows up as three differing
/// lines rather than as a silent `checked true`.
///
/// `proof-term-bytes` is the length of the proof term's canonical encoding. The
/// encoding itself does NOT cross this boundary — see the crate's own README-level
/// note on the typed surface that would be needed to carry the derivation DAG — so
/// `digest` is what a consumer compares across hosts.
///
/// # Errors
///
/// A malformed document; a `conclusion` that is not one statement; an unknown
/// regime spelling; `rdf` or `rdfs`, whose four blank-node-minting rules have
/// existential heads with no Datalog semantics and therefore no checkable proof
/// term; a conclusion that is neither asserted nor derived; or a proof that does
/// not re-derive, which is an engine defect made visible rather than shipped.
pub fn explain_conclusion_to_string(
    document: &str,
    regime: &str,
    conclusion: &str,
) -> Result<ReasoningAnswer, String> {
    let parsed = parse_regime(regime)?;
    let (graph, subject, predicate, object) = parse_one_statement(conclusion)?;
    let dataset = purrdf_rdf::parse_dataset(document.as_bytes(), INPUT_MEDIA_TYPE, None)
        .map_err(|diagnostic| diagnostic.to_string())?;
    let proof = explain_conclusion(
        &dataset,
        parsed,
        graph.as_ref(),
        &subject,
        &predicate,
        &object,
    )
    .map_err(|error| format!("explain-conclusion: {error}"))?;
    Ok(ReasoningAnswer {
        answer: render_chase_proof_answer(&proof),
        certificate: render_chase_proof_certificate(&proof),
    })
}

/// The `asserted`/`steps`/`rule` block of [`explain_conclusion_to_string`].
fn render_chase_proof_answer(proof: &ChaseProof) -> String {
    let mut answer = String::new();
    let _ = writeln!(answer, "asserted {}", proof.is_asserted());
    let _ = writeln!(answer, "steps {}", proof.steps());
    for rule in proof.rules() {
        let _ = writeln!(answer, "rule {}", rule.as_str());
    }
    answer
}

/// The certificate of [`explain_conclusion_to_string`], with the proof RE-CHECKED.
fn render_chase_proof_certificate(proof: &ChaseProof) -> String {
    let (subject, predicate, object) = proof.conclusion();
    let mut out = String::new();
    out.push_str(CHASE_PROOF_CERTIFICATE_BANNER);
    out.push('\n');
    out.push_str("service explain-conclusion\n");
    let _ = writeln!(out, "regime {}", regime_name(proof.regime()));
    match proof.graph() {
        None => out.push_str("graph default\n"),
        Some(graph) => {
            let _ = writeln!(out, "graph {}", emit(graph));
        }
    }
    let _ = writeln!(out, "conclusion-subject {}", emit(subject));
    let _ = writeln!(out, "conclusion-predicate {}", emit(predicate));
    let _ = writeln!(out, "conclusion-object {}", emit(object));
    // RE-DERIVED, not re-read: `check` walks the premises to the facts they
    // establish, matches the cited clause's body against them, and instantiates the
    // head. The proof's stated conclusion is not an input to that computation.
    let checked = proof.check();
    match &checked {
        Ok(fact) => {
            let _ = writeln!(out, "derived-subject {}", fact.subject);
            let _ = writeln!(out, "derived-predicate {}", fact.predicate);
            let _ = writeln!(out, "derived-object {}", fact.object);
        }
        Err(error) => {
            let _ = writeln!(out, "derived-subject unchecked");
            let _ = writeln!(out, "derived-predicate unchecked");
            let _ = writeln!(out, "derived-object {error}");
        }
    }
    let _ = writeln!(out, "digest {}", proof.digest_hex());
    let _ = writeln!(out, "proof-term-bytes {}", proof.encode().len());
    let _ = writeln!(out, "checked {}", checked.is_ok());
    let _ = writeln!(out, "overclaims {}", checked.is_err());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    /// `A ⊑ B ⊑ C`, `p` with a domain and a range, and one typed instance.
    const SCHEMA: &str = "\
<http://example.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/B> .
<http://example.org/B> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .
<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .
";

    /// A minimal normative RIF-in-XML rule document: `?x a ex:A` ⟹ `?x a ex:B`.
    ///
    /// Written against [`SCHEMA`]'s own vocabulary so the conclusion is observable, and
    /// deliberately a conclusion RDFS ALSO draws from `ex:A rdfs:subClassOf ex:B` — which
    /// is what lets the `rif` test show the rule fired rather than the rule table.
    const RIF_PROGRAM: &str = concat!(
        "<Document xmlns=\"http://www.w3.org/2007/rif#\"><payload><Group><sentence><Forall>",
        "<declare><Var>x</Var></declare><formula><Implies>",
        "<if><Frame><object><Var>x</Var></object><slot>",
        "<Const type=\"http://www.w3.org/2007/rif#iri\">",
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#type</Const>",
        "<Const type=\"http://www.w3.org/2007/rif#iri\">http://example.org/A</Const>",
        "</slot></Frame></if>",
        "<then><Frame><object><Var>x</Var></object><slot>",
        "<Const type=\"http://www.w3.org/2007/rif#iri\">",
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#type</Const>",
        "<Const type=\"http://www.w3.org/2007/rif#iri\">http://example.org/B</Const>",
        "</slot></Frame></then>",
        "</Implies></formula></Forall></sentence></Group></payload></Document>"
    );

    /// Every accepted regime spelling with the `program` its boundary call takes.
    ///
    /// THE POINT OF THIS TABLE is that it has seven rows. The constant it replaces named
    /// the five regimes this boundary would close and left two to a refusal test; every
    /// cross-cutting invariant below therefore ranged over five sevenths of the surface.
    /// They range over all of it now, which is what makes "this boundary refuses no
    /// regime" a property the whole test module checks rather than one test's claim.
    const REGIME_CALLS: [(&str, &str); 7] = [
        ("simple", ""),
        ("rdf", ""),
        ("rdfs", ""),
        ("owl-rl", ""),
        ("owl-direct", ""),
        ("rif", RIF_PROGRAM),
        ("d", ""),
    ];

    // ── The golden vector ───────────────────────────────────────────────────

    /// The committed artifact still describes what this boundary produces.
    ///
    /// The C-ABI and WASM crates make this exact call against this exact
    /// artifact, so a host that diverges fails here in the same words.
    #[test]
    fn the_golden_vector_matches() {
        check_regime_golden_vectors().expect("the regime golden vector");
    }

    /// The artifact parses into the cases it claims, and every case is usable.
    #[test]
    fn the_golden_vector_is_well_formed() {
        let cases = regime_golden_vectors().expect("the artifact parses");
        assert!(!cases.is_empty());
        for case in &cases {
            parse_regime(case.regime()).expect("a case names a real regime");
            assert!(!case.input().is_empty(), "{}", case.name());
            assert!(
                case.report().starts_with(REPORT_FORMAT_BANNER),
                "{}",
                case.name()
            );
            // The format constraint the parser relies on.
            for line in case.input().lines().chain(case.report().lines()) {
                assert!(!line.starts_with('@'), "{}: {line}", case.name());
            }
        }
    }

    /// A malformed artifact is rejected, with the line number, rather than
    /// silently parsing into fewer cases than it looks like it has.
    #[test]
    fn a_malformed_artifact_is_rejected() {
        for (bad, needle) in [
            ("@case a\n@regime rdfs\n@input\n@closure\n@report\n", "@end"),
            ("@case a\n@bogus\n", "@bogus"),
            ("@case\n", "@case needs a name"),
            ("stray text\n", "outside any section"),
            (
                "@case a\n@regime rdfs\n@input\n@closure\n@report\n@end\n@case a\n@regime rdfs\n\
                 @input\n@closure\n@report\n@end\n",
                "duplicate case name",
            ),
            ("@case a\n@input\n@closure\n@report\n@end\n", "@regime"),
        ] {
            let error = parse_regime_vectors(bad).expect_err("malformed");
            assert!(error.contains(needle), "{error} (wanted {needle})");
        }
    }

    // ── Regime spellings ────────────────────────────────────────────────────

    /// Every accepted spelling round-trips through the parse/name pair.
    #[test]
    fn every_regime_spelling_round_trips() {
        for name in REGIME_NAMES {
            let regime = parse_regime(name).expect("an accepted spelling");
            assert_eq!(regime_name(regime), name);
        }
        // …and the CLI's own enum has no eighth value this set is missing.
        assert_eq!(REGIME_NAMES.len(), 7);
        for name in PROGRAM_REGIME_NAMES {
            assert!(REGIME_NAMES.contains(&name), "{name}");
        }
        // The call table below covers the whole accepted set, in its order — so a regime
        // added to `REGIME_NAMES` without a boundary call to exercise it fails here.
        assert_eq!(
            REGIME_CALLS.map(|(name, _)| name).to_vec(),
            REGIME_NAMES.to_vec()
        );
    }

    /// THE REFUSAL CARRIES THE CERTIFICATE, and the witness TRIPLES with it.
    ///
    /// The C-ABI and WASM hosts make this same call against this same checker.
    #[test]
    fn an_inconsistent_input_is_refused_with_its_report() {
        check_inconsistent_refusal().expect("the inconsistent refusal");
    }

    /// The withheld-surrogate count is a MEASUREMENT of the run, not a constant.
    ///
    /// The four existential rules are unobservable any other way — they can never appear
    /// in a `fired` line — so a renderer that omits this number leaves a caller with a
    /// `boundary surrogate` paragraph saying conclusions are "counted here" beside no
    /// count. It is asserted as a comparison between lanes rather than as a literal: the
    /// RDFS lane states all four rules and withholds conclusions, `owl-rl` states none of
    /// them and withholds nothing, and both numbers reach the same shared renderer.
    #[test]
    fn the_withheld_surrogate_count_moves_with_the_lane() {
        let withheld = |regime: &str| -> u64 {
            let report = materialize_to_nquads_string(regime, SCHEMA, "")
                .expect("a regime")
                .into_parts()
                .1;
            report
                .lines()
                .find_map(|line| line.strip_prefix("withheld-surrogates "))
                .expect("every report states the count")
                .parse()
                .expect("a decimal count")
        };
        assert!(
            withheld("rdfs") > 0,
            "rdfs states rdfD1, rdfD1a, rdfs14 and rdfs14a, and withholds what they conclude"
        );
        assert_eq!(
            withheld("owl-rl"),
            0,
            "OWL 2 RL states none of the four, so there is nothing to withhold"
        );
        assert_eq!(
            withheld("simple"),
            0,
            "the identity closure evaluates nothing"
        );
    }

    /// An unknown spelling is refused with the whole accepted set named.
    #[test]
    fn an_unknown_regime_names_the_accepted_set() {
        let error = parse_regime("rdfs-plus").expect_err("unknown");
        assert!(error.contains("rdfs-plus"), "{error}");
        for name in REGIME_NAMES {
            assert!(error.contains(name), "{error} omits {name}");
        }
        // The same message reaches the two string entry points.
        for error in [
            materialize_to_nquads_string("rdfs-plus", SCHEMA, "").expect_err("unknown"),
            rules_string("rdfs-plus").expect_err("unknown"),
            implemented_rules_string("rdfs-plus").expect_err("unknown"),
        ] {
            assert!(error.contains("accepted: simple, rdf, rdfs"), "{error}");
        }
    }

    /// EVERY accepted spelling closes. This boundary refuses no regime.
    ///
    /// Falsifiable against the behaviour this replaced: `owl-direct` and `rif` were
    /// refused here by name, against this same `SCHEMA`, with a message listing the five
    /// that were not. The list is gone because the refusal is, and what replaced it is
    /// this: seven spellings in, seven closures out, each report naming the regime asked
    /// for.
    #[test]
    fn every_regime_spelling_materializes() {
        for (name, program) in REGIME_CALLS {
            let closed = materialize_to_nquads_string(name, SCHEMA, program)
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            assert!(
                closed.report().contains(&format!("\nregime {name}\n")),
                "{name}: {}",
                closed.report()
            );
            assert!(closed.report().contains("\nwithheld-surrogates "), "{name}");
            assert!(closed.report().ends_with("inconsistency none\n"), "{name}");
            // The asserted data is carried through by every lane, so none of them is an
            // empty answer dressed as a closure.
            assert!(
                closed.nquads().contains("<http://example.org/A>"),
                "{name}: {}",
                closed.nquads()
            );
        }
    }

    /// A rule document is an INPUT of exactly one regime, and passing one to any other
    /// is an error rather than a silently discarded argument.
    #[test]
    fn a_rule_document_belongs_to_rif_and_is_refused_elsewhere() {
        for (name, _) in REGIME_CALLS {
            if name == "rif" {
                continue;
            }
            let error = materialize_to_nquads_string(name, SCHEMA, RIF_PROGRAM)
                .expect_err("a rule document for a rule-table regime");
            assert!(error.contains(name), "{error}");
            assert!(error.contains("takes no rule document"), "{error}");
            assert!(error.contains("Regimes that take one: rif"), "{error}");
        }
        // …and `rif` without one is an error too: an empty string is not a rule document,
        // so it fails as a malformed one rather than closing over no rules at all.
        let error = materialize_to_nquads_string("rif", SCHEMA, "").expect_err("no rule document");
        assert!(error.contains("rif"), "{error}");
    }

    /// An `Import` is I/O, and this boundary performs none — so it says so by name.
    #[test]
    fn a_rif_import_is_refused_with_its_location() {
        let importing = RIF_PROGRAM.replace(
            "<payload>",
            "<directive><Import><location>http://example.org/facts.nt</location>\
             </Import></directive><payload>",
        );
        let error = materialize_to_nquads_string("rif", SCHEMA, &importing).expect_err("an import");
        assert!(error.contains("http://example.org/facts.nt"), "{error}");
        assert!(error.contains("resolve it in the caller"), "{error}");
    }

    /// The `rif` lane really runs the caller's rules, and only the caller's.
    #[test]
    fn the_rif_lane_entails_under_the_supplied_rules() {
        let closed = materialize_to_nquads_string("rif", SCHEMA, RIF_PROGRAM).expect("rif");
        // The rule concludes `?x a ex:B` from `?x a ex:A`; the fixture asserts the premise.
        assert!(
            closed.nquads().contains(
                "<http://example.org/x> \
                 <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/B> ."
            ),
            "{}",
            closed.nquads()
        );
        // …and nothing the RDFS table would have added: the rule set is the whole calculus.
        assert!(
            !closed
                .nquads()
                .contains("<http://www.w3.org/2000/01/rdf-schema#Resource>"),
            "{}",
            closed.nquads()
        );
    }

    // ── Materialization ─────────────────────────────────────────────────────

    /// The closure really closes, and `simple` really does not.
    #[test]
    fn the_closure_infers_under_rdfs_and_not_under_simple() {
        let typed = "<http://example.org/x> \
                     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C> .";
        let rdfs = materialize_to_nquads_string("rdfs", SCHEMA, "").expect("rdfs");
        assert!(rdfs.nquads().contains(typed), "{}", rdfs.nquads());
        let simple = materialize_to_nquads_string("simple", SCHEMA, "").expect("simple");
        assert!(!simple.nquads().contains(typed), "{}", simple.nquads());
        // `simple` is the identity closure, so its canonical form is the input's.
        assert_eq!(
            simple.nquads(),
            materialize_to_nquads_string("simple", simple.nquads(), "")
                .expect("simple")
                .nquads()
        );
    }

    /// An N-Quads input keeps its named graph through the boundary, which is why
    /// the entry point parses N-Quads rather than N-Triples.
    #[test]
    fn a_named_graph_survives_the_boundary() {
        let quads = "<http://example.org/x> <http://example.org/p> <http://example.org/y> \
                     <http://example.org/g> .\n";
        let closed = materialize_to_nquads_string("simple", quads, "").expect("simple");
        assert!(closed.nquads().contains("<http://example.org/g>"));
        // …and the RDFS lane reports it as a boundary rather than reasoning over it.
        let closed = materialize_to_nquads_string("rdfs", quads, "").expect("rdfs");
        assert!(closed.report().contains("\nboundary named-graph "));
    }

    /// A malformed document is an error, not an empty closure.
    #[test]
    fn a_malformed_document_is_an_error() {
        assert!(materialize_to_nquads_string("rdfs", "this is not n-quads\n", "").is_err());
    }

    // ── The rendering ───────────────────────────────────────────────────────

    /// The rendering is byte-stable across repeated calls, for every regime.
    ///
    /// Each `materialize` seeds a freshly-hashed fact store, so a rendering that
    /// leaked any hash order — or any clock, path or address — would diverge here.
    #[test]
    fn the_rendering_is_byte_stable_across_calls() {
        for (regime, program) in REGIME_CALLS {
            let first = materialize_to_nquads_string(regime, SCHEMA, program).expect("a regime");
            let second = materialize_to_nquads_string(regime, SCHEMA, program).expect("a regime");
            assert_eq!(first, second, "{regime}");
            // Ten more, so a one-in-two divergence cannot pass by luck.
            for _ in 0..10 {
                let again =
                    materialize_to_nquads_string(regime, SCHEMA, program).expect("a regime");
                assert_eq!(again.report(), first.report(), "{regime}");
                assert_eq!(again.nquads(), first.nquads(), "{regime}");
            }
        }
    }

    /// The rendering's shape: banner first, newline-terminated, fixed field order, the
    /// withheld-surrogate count present on every host, and a checked `inconsistency none`
    /// last.
    #[test]
    fn the_rendering_has_the_documented_shape() {
        for (regime, program) in REGIME_CALLS {
            let report = materialize_to_nquads_string(regime, SCHEMA, program)
                .expect("a regime")
                .into_parts()
                .1;
            let lines: Vec<&str> = report.lines().collect();
            assert_eq!(lines[0], REPORT_FORMAT_BANNER, "{regime}");
            assert_eq!(lines[1], format!("regime {regime}"), "{regime}");
            assert!(lines[2].starts_with("completeness "), "{regime}");
            assert!(report.ends_with("inconsistency none\n"), "{regime}");
            assert_eq!(
                report.matches("\nwithheld-surrogates ").count(),
                1,
                "{regime}: the withheld-surrogate count reaches every host"
            );
            assert_eq!(
                report.matches("\ncontract-hash ").count(),
                1,
                "{regime}: exactly one calculus identity"
            );
            // Field order: the optional blocks appear between `completeness` and
            // `budget`, never after it.
            let budget = report.find("\nbudget join-steps ").expect("a budget");
            for keyword in ["\nmissing ", "\nfired ", "\nboundary "] {
                if let Some(at) = report.find(keyword) {
                    assert!(at < budget, "{regime}: {keyword} after the budget");
                }
            }
        }
    }

    /// The `missing` lines are exactly the inventory difference the two
    /// rule-string entry points expose — the report and the inventory cannot
    /// drift apart.
    #[test]
    fn the_missing_lines_are_the_inventory_difference() {
        for (regime, program) in REGIME_CALLS {
            let defined_set = rules_string(regime).expect("known");
            let defined: Vec<&str> = defined_set.lines().collect();
            let fired_set = implemented_rules_string(regime).expect("known");
            let fired: Vec<&str> = fired_set.lines().collect();
            let expected: Vec<&str> = defined
                .iter()
                .copied()
                .filter(|rule| !fired.contains(rule))
                .collect();

            let report = materialize_to_nquads_string(regime, SCHEMA, program)
                .expect("a regime")
                .into_parts()
                .1;
            let missing: Vec<&str> = report
                .lines()
                .filter_map(|line| line.strip_prefix("missing "))
                .collect();
            assert_eq!(missing, expected, "{regime}");
            let expected_head = if expected.is_empty() {
                "completeness exact".to_owned()
            } else {
                format!("completeness sound-incomplete {}", expected.len())
            };
            assert!(report.contains(&expected_head), "{regime}");
        }
    }

    /// REGENERATE [`REGIME_GOLDEN_VECTORS`] from the current engine.
    ///
    /// `#[ignore]`d, so an ordinary `cargo test` can only ever COMPARE — the same
    /// discipline `purrdf-entail`'s oracle corpus keeps, and for the same reason: a
    /// golden that regenerates itself on every run is not a golden. The `@case`
    /// names, their `@regime`s and their `@input` documents are the artifact's own
    /// and are carried through unchanged; only the `@closure` and `@report` bodies
    /// are rewritten, so a regeneration can never invent a case or quietly change
    /// what one is about.
    ///
    /// ```text
    /// cargo test -p purrdf-validate --lib -- --ignored --exact \
    ///     regime::tests::regenerate_regime_golden_vectors
    /// ```
    #[test]
    #[ignore = "writes the committed golden vector; run deliberately"]
    fn regenerate_regime_golden_vectors() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/regime-boundary.vectors");
        let source = std::fs::read_to_string(&path).expect("the committed vector");
        let mut out = String::new();
        let mut lines = source.lines().peekable();
        while let Some(line) = lines.next() {
            if let Some(rest) = line.strip_prefix("@closure") {
                assert!(rest.is_empty(), "@closure takes no argument");
                // Skip the committed bodies; they are replaced wholesale below.
                while lines.peek().is_some_and(|next| !next.starts_with("@end")) {
                    let _ = lines.next();
                }
                continue;
            }
            out.push_str(line);
            out.push('\n');
        }
        // Re-run every case and splice its two bodies back in. `@input` and `@program`
        // are both carried through verbatim AND accumulated, because the second is an
        // argument to the call that produces the two bodies being rewritten.
        let mut rendered = String::new();
        let mut case_regime: Option<&str> = None;
        let mut input = String::new();
        let mut program = String::new();
        let mut open: Option<Section> = None;
        for line in out.lines() {
            if let Some(name) = line.strip_prefix("@regime ") {
                case_regime = Some(Box::leak(name.to_owned().into_boxed_str()));
            }
            if line == "@input" || line == "@program" {
                let section = if line == "@input" {
                    input.clear();
                    Section::Input
                } else {
                    program.clear();
                    Section::Program
                };
                open = Some(section);
                rendered.push_str(line);
                rendered.push('\n');
                continue;
            }
            if line == "@end" {
                let regime = case_regime.expect("a case names its regime");
                let closed = materialize_to_nquads_string(regime, &input, &program)
                    .expect("a golden case runs");
                rendered.push_str("@closure\n");
                rendered.push_str(closed.nquads());
                rendered.push_str("@report\n");
                rendered.push_str(closed.report());
                rendered.push_str("@end\n");
                open = None;
                program.clear();
                continue;
            }
            if open.is_some() && !line.starts_with('@') {
                let body = if open == Some(Section::Input) {
                    &mut input
                } else {
                    &mut program
                };
                body.push_str(line);
                body.push('\n');
                rendered.push_str(line);
                rendered.push('\n');
                continue;
            }
            if line.starts_with("@report") {
                open = None;
                continue;
            }
            open = None;
            rendered.push_str(line);
            rendered.push('\n');
        }
        std::fs::write(&path, &rendered).expect("write the golden vector");
    }

    /// The inventory strings are the specification tables, in table order, and
    /// `implemented` is a subsequence of `rules`.
    #[test]
    fn the_inventory_strings_are_the_specification_tables() {
        assert_eq!(rules_string("owl-rl").expect("known").lines().count(), 78);
        assert_eq!(rules_string("rdfs").expect("known").lines().count(), 18);
        assert_eq!(rules_string("rdf").expect("known").lines().count(), 3);
        // `d` is OWL 2 Profiles §4.3 Table 8 — five rules, all of them fired.
        assert_eq!(rules_string("d").expect("known").lines().count(), 5);
        assert_eq!(
            implemented_rules_string("d")
                .expect("known")
                .lines()
                .count(),
            5
        );
        for regime in ["simple", "owl-direct", "rif"] {
            assert_eq!(rules_string(regime).expect("known"), "", "{regime}");
            assert_eq!(implemented_rules_string(regime).expect("known"), "");
        }
        for regime in REGIME_NAMES {
            let defined = rules_string(regime).expect("known");
            let implemented = implemented_rules_string(regime).expect("known");
            // A subsequence: same relative order, no additions.
            let mut defined_lines = defined.lines();
            for rule in implemented.lines() {
                assert!(
                    defined_lines.any(|candidate| candidate == rule),
                    "{regime}: {rule} is implemented but not defined"
                );
            }
            // Non-empty inventories end with a newline; empty ones are empty.
            assert_eq!(defined.is_empty(), defined.lines().count() == 0);
            if !defined.is_empty() {
                assert!(defined.ends_with('\n'), "{regime}");
            }
        }
    }

    /// The `boundary` lines carry the construct AND its technical reason, so a
    /// consumer three languages away is not left to re-derive the mapping.
    #[test]
    fn boundary_lines_carry_the_reason() {
        let report = materialize_to_nquads_string("rdfs", SCHEMA, "")
            .expect("rdfs")
            .into_parts()
            .1;
        let boundaries: Vec<&str> = report
            .lines()
            .filter_map(|line| line.strip_prefix("boundary "))
            .collect();
        assert!(!boundaries.is_empty());
        for boundary in boundaries {
            let (construct, reason) = boundary.split_once(' ').expect("a construct and a reason");
            assert!(!construct.is_empty());
            assert!(reason.len() > construct.len(), "{boundary}");
        }
        // `simple` copies faithfully, so it names none — and is therefore the one
        // regime whose `exact` claim carries no contradiction.
        let simple = materialize_to_nquads_string("simple", SCHEMA, "")
            .expect("simple")
            .into_parts()
            .1;
        assert!(!simple.contains("\nboundary "));
        assert!(simple.contains("\ncompleteness exact\n"));
    }

    /// Two different regimes are two different calculi, and the rendered
    /// contract hash says which one a closure was minted under.
    #[test]
    fn the_contract_hash_distinguishes_the_calculi() {
        let hash_of = |regime: &str| -> String {
            materialize_to_nquads_string(regime, SCHEMA, "")
                .expect("materializable")
                .into_parts()
                .1
                .lines()
                .find_map(|line| line.strip_prefix("contract-hash ").map(str::to_owned))
                .expect("a contract hash")
        };
        let rdfs = hash_of("rdfs");
        assert_eq!(rdfs.len(), 64);
        assert!(
            rdfs.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
        );
        assert_ne!(rdfs, hash_of("owl-rl"));
        assert_ne!(rdfs, hash_of("rdf"));
        // …and it is a property of the calculus, not of the data.
        assert_eq!(rdfs, {
            let other = "<http://example.org/u> <http://example.org/q> <http://example.org/v> .\n";
            materialize_to_nquads_string("rdfs", other, "")
                .expect("rdfs")
                .into_parts()
                .1
                .lines()
                .find_map(|line| line.strip_prefix("contract-hash ").map(str::to_owned))
                .expect("a contract hash")
        });
    }

    // ── The Description-Logic reasoning services ────────────────────────────

    /// `A ⊑ B ⊑ C`, `D ⊑ C`, and one instance of `A`.
    const TAXONOMY: &str = "\
<http://example.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/B> .
<http://example.org/B> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .
<http://example.org/D> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .
<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .
";

    /// `A` and `B` are disjoint and `x` is in both: an ontology with NO model.
    const UNSATISFIABLE: &str = "\
<http://example.org/A> <http://www.w3.org/2002/07/owl#disjointWith> <http://example.org/B> .
<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .
<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/B> .
";

    /// `A ⊑ C` — asserted nowhere, entailed by the chain.
    const CHAIN_AXIOM: &str = "<http://example.org/A> \
<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .\n";

    /// Every service, as `(name, produced)`, over `document`.
    ///
    /// One list so a service added without a certificate assertion is a compile
    /// error at this call site rather than an omission nobody notices.
    fn every_service(document: &str) -> Vec<(&'static str, Result<ReasoningAnswer, String>)> {
        vec![
            ("consistency", consistency_to_string(document, 0)),
            ("classify", classify_to_string(document, 0)),
            ("realize", realize_to_string(document, 0)),
            (
                "instances",
                instances_to_string(document, "<http://example.org/C>", 0),
            ),
            ("entails", entails_to_string(document, CHAIN_AXIOM, 0)),
            ("profile", profile_to_string(document)),
            (
                "extract-module",
                extract_module_to_string(document, "<http://example.org/A>\n", "star"),
            ),
            ("justify", justify_to_string(document, CHAIN_AXIOM)),
            (
                "explain-conclusion",
                explain_conclusion_to_string(
                    document,
                    "owl-rl",
                    "<http://example.org/x> \
                     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
                     <http://example.org/C> .\n",
                ),
            ),
        ]
    }

    /// EVERY service carries a certificate, and every certificate names its own
    /// service and ends with its own honesty gate.
    ///
    /// The invariant this whole surface exists for: an answer without a statement
    /// of how completely it was decided is the defect, not the missing feature.
    #[test]
    fn every_service_carries_its_certificate() {
        for (service, produced) in every_service(TAXONOMY) {
            let produced = produced.unwrap_or_else(|error| panic!("{service}: {error}"));
            let certificate = produced.certificate();
            assert!(!certificate.is_empty(), "{service}");
            assert!(
                certificate.contains(&format!("\nservice {service}\n")),
                "{service}: {certificate}"
            );
            // The gate is the LAST line, so a truncated certificate is visibly
            // truncated rather than silently gate-free.
            let gate = certificate.lines().last().unwrap_or_default();
            assert!(
                matches!(
                    gate,
                    "overclaims false" | "one-directional true" | "conservative false"
                ),
                "{service}: {gate}"
            );
            assert!(certificate.ends_with('\n'), "{service}");
        }
    }

    /// The DL certificate is NOT the chase report, and cannot be parsed as one.
    ///
    /// The two banners differ precisely so a consumer that reached for the wrong
    /// grammar fails at the first line rather than reading `decided` where it
    /// expected `exact`.
    #[test]
    fn the_two_lanes_render_different_certificates() {
        let chase = materialize_to_nquads_string("owl-rl", TAXONOMY, "").expect("owl-rl");
        assert!(chase.report().starts_with(REPORT_FORMAT_BANNER));
        let tableau = consistency_to_string(TAXONOMY, 0).expect("consistency");
        assert!(tableau.certificate().starts_with(DL_CERTIFICATE_BANNER));
        assert_ne!(REPORT_FORMAT_BANNER, DL_CERTIFICATE_BANNER);
        // …and neither completeness vocabulary appears in the other's rendering.
        assert!(!chase.report().contains("completeness decided"));
        assert!(!tableau.certificate().contains("completeness exact"));
    }

    /// Every service is byte-stable across repeated calls.
    ///
    /// Each call reverse-maps a freshly-interned knowledge base, so a rendering
    /// that leaked interner order, a clock or an address would diverge here.
    #[test]
    fn the_dl_renderings_are_byte_stable_across_calls() {
        let first: Vec<_> = every_service(TAXONOMY)
            .into_iter()
            .map(|(service, produced)| (service, produced.expect("a service runs")))
            .collect();
        for _ in 0..8 {
            for ((service, expected), (_, again)) in first.iter().zip(
                every_service(TAXONOMY)
                    .into_iter()
                    .map(|(service, produced)| (service, produced.expect("a service runs"))),
            ) {
                assert_eq!(expected, &again, "{service}");
            }
        }
    }

    /// Classification derives the transitive closure AND its reduction, and both
    /// are emitted because they are different facts.
    #[test]
    fn classify_emits_the_closure_and_its_reduction() {
        let answer = classify_to_string(TAXONOMY, 0)
            .expect("classify")
            .into_parts()
            .0;
        // A ⊑ C is entailed but not asserted, and it is NOT a direct subsumption:
        // B sits between them.
        assert!(answer.contains("subclass <http://example.org/A> <http://example.org/C>\n"));
        assert!(!answer.contains("direct <http://example.org/A> <http://example.org/C>\n"));
        assert!(answer.contains("direct <http://example.org/A> <http://example.org/B>\n"));
        // `owl:Nothing` is read as ⊥, so it is unsatisfiable and subsumed by every
        // named class — the answers the semantics gives, not an opaque-class reading.
        assert!(
            answer.contains("unsatisfiable <http://www.w3.org/2002/07/owl#Nothing>\n"),
            "{answer}"
        );
        // The blocks appear in the documented order.
        let subclass = answer
            .find("\nsubclass ")
            .or_else(|| answer.find("subclass "));
        let direct = answer.find("direct <");
        assert!(subclass < direct, "{answer}");
    }

    /// Realization lists every entailed type and marks the most specific one.
    #[test]
    fn realize_marks_the_most_specific_type() {
        let answer = realize_to_string(TAXONOMY, 0)
            .expect("realize")
            .into_parts()
            .0;
        for class in ["A", "B", "C"] {
            assert!(
                answer.contains(&format!(
                    "type <http://example.org/x> <http://example.org/{class}>\n"
                )),
                "{answer}"
            );
        }
        // `owl:Thing` is a type of every individual and is listed: an entailed
        // answer omitted for being obvious is an answer set that is not one.
        assert!(
            answer.contains("type <http://example.org/x> <http://www.w3.org/2002/07/owl#Thing>")
        );
        // Exactly one direct type — the most specific of the three.
        let direct: Vec<&str> = answer
            .lines()
            .filter(|line| line.starts_with("direct-type "))
            .collect();
        assert_eq!(
            direct,
            vec!["direct-type <http://example.org/x> <http://example.org/A>"]
        );
    }

    /// Instance retrieval reaches THROUGH the hierarchy, and an unmentioned class
    /// is an empty answer rather than an error.
    #[test]
    fn instances_retrieves_through_the_hierarchy() {
        let answer = instances_to_string(TAXONOMY, "<http://example.org/C>", 0)
            .expect("instances")
            .into_parts()
            .0;
        assert_eq!(answer, "instance <http://example.org/x>\n");
        // A class no axiom constrains: a real question with a real, empty answer.
        let unknown = instances_to_string(TAXONOMY, "<http://example.org/Unmentioned>", 0)
            .expect("an unconstrained name is a real name");
        assert_eq!(unknown.answer(), "");
        assert!(unknown.certificate().ends_with("overclaims false\n"));
    }

    /// Every OWL 2 RDF-mapping predicate dispatches to the axiom kind it spells,
    /// and any other predicate is an object-property assertion.
    #[test]
    fn the_axiom_encoding_is_the_owl_2_rdf_mapping() {
        let cases = [
            (RDFS_SUBCLASS_OF, "SubClassOf"),
            (OWL_EQUIVALENT_CLASS, "EquivalentClasses"),
            (OWL_DISJOINT_WITH, "DisjointClasses"),
            (RDF_TYPE, "ClassAssertion"),
            (OWL_SAME_AS, "SameIndividual"),
            (OWL_DIFFERENT_FROM, "DifferentIndividuals"),
            (RDFS_SUBPROPERTY_OF, "SubObjectPropertyOf"),
            ("http://example.org/p", "ObjectPropertyAssertion"),
        ];
        for (predicate, kind) in cases {
            let statement =
                format!("<http://example.org/s> <{predicate}> <http://example.org/o> .\n");
            let answer = entails_to_string(TAXONOMY, &statement, 0)
                .unwrap_or_else(|error| panic!("{kind}: {error}"))
                .into_parts()
                .0;
            assert!(answer.contains(&format!("\naxiom {kind}\n")), "{answer}");
            assert!(AXIOM_KINDS.contains(&kind));
        }
        // …and all eight kinds are covered, so a ninth variant fails here.
        assert_eq!(cases.len(), AXIOM_KINDS.len());
    }

    /// An entailed axiom is `true`, an unentailed one is `false`, and neither is
    /// invented: `A ⊑ C` follows from the chain, `C ⊑ A` does not.
    #[test]
    fn entails_decides_both_directions() {
        let entailed = entails_to_string(TAXONOMY, CHAIN_AXIOM, 0).expect("decides");
        assert!(entailed.answer().starts_with("entails true\n"));
        let reversed = "<http://example.org/C> \
<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/A> .\n";
        let refuted = entails_to_string(TAXONOMY, reversed, 0).expect("decides");
        assert!(refuted.answer().starts_with("entails false\n"));
    }

    /// A search that runs out of budget answers `unknown`, NEVER `false`, and the
    /// certificate says why.
    ///
    /// This is the third completeness state made reachable from the boundary: the
    /// `step_cap` argument can only NARROW, so it cannot make a hard instance
    /// answerable — it can only make this branch executable.
    #[test]
    fn an_exhausted_budget_is_unknown_and_never_false() {
        let starved = entails_to_string(TAXONOMY, CHAIN_AXIOM, 1).expect("decides nothing");
        assert_eq!(starved.answer().lines().next(), Some("entails unknown"));
        assert!(
            starved
                .certificate()
                .contains("\ncompleteness budget-exhausted\n"),
            "{}",
            starved.certificate()
        );
        assert!(starved.certificate().contains("\nbudget 1\n"));
        // The gate still holds: an exhausted run claims nothing it cannot support.
        assert!(starved.certificate().ends_with("overclaims false\n"));
        // …and the un-narrowed call decides the same question.
        assert!(
            entails_to_string(TAXONOMY, CHAIN_AXIOM, 0)
                .expect("decides")
                .answer()
                .starts_with("entails true\n")
        );
    }

    /// An ontology with no model is REFUSED by every service but the one that
    /// detects it — and that one answers `false`.
    #[test]
    fn an_unsatisfiable_ontology_is_refused_rather_than_answered_vacuously() {
        let detected = consistency_to_string(UNSATISFIABLE, 0).expect("consistency answers");
        assert_eq!(detected.answer(), "consistency false\n");
        for service in ["classify", "realize"] {
            let produced = match service {
                "classify" => classify_to_string(UNSATISFIABLE, 0),
                _ => realize_to_string(UNSATISFIABLE, 0),
            };
            let error = produced.expect_err("no model");
            assert!(error.starts_with(service), "{error}");
            assert!(error.contains("consistency_to_string"), "{error}");
        }
    }

    /// Profile certification lists the profiles most restrictive first and says,
    /// densely, which it certifies — the reading a sparse violation list makes easy
    /// to get wrong.
    #[test]
    fn profile_certifies_most_restrictive_first() {
        let certified = profile_to_string(TAXONOMY).expect("parses");
        assert_eq!(
            certified.answer(),
            "certified EL\ncertified QL\ncertified RL\ncertified DL\ncertified Full\n"
        );
        for profile in ["el", "ql", "rl", "dl", "full"] {
            assert!(
                certified
                    .certificate()
                    .contains(&format!("\ncertifies-{profile} true\n")),
                "{}",
                certified.certificate()
            );
        }
        // The one-directional doctrine is stated on the certificate itself, not
        // only in prose a consumer may never read.
        assert!(certified.certificate().ends_with("one-directional true\n"));
    }

    /// A construct outside a profile blocks it BY NAME, and `Full` is certified
    /// whatever the ontology says.
    #[test]
    fn a_profile_violation_names_its_term_and_reason() {
        // `owl:complementOf` is not in the EL grammar.
        let complement = "<http://example.org/NotA> \
<http://www.w3.org/2002/07/owl#complementOf> <http://example.org/A> .\n";
        let certified = profile_to_string(complement).expect("parses");
        let violations: Vec<&str> = certified
            .certificate()
            .lines()
            .filter_map(|line| line.strip_prefix("violation "))
            .collect();
        assert!(!violations.is_empty(), "{}", certified.certificate());
        for violation in violations {
            // profile, term, subject, then the rest of the line is the reason.
            let mut fields = violation.splitn(4, ' ');
            let profile = fields.next().expect("a profile");
            assert!(
                OwlProfile::ALL
                    .iter()
                    .any(|known| known.as_str() == profile),
                "{violation}"
            );
            assert!(fields.next().is_some_and(|term| term.starts_with('<')));
            assert!(fields.next().is_some_and(|subject| !subject.is_empty()));
            assert!(fields.next().is_some_and(|reason| reason.len() > 4));
        }
        // Full is every RDF graph, so it is always certified.
        assert!(certified.answer().ends_with("certified Full\n"));
        assert!(certified.certificate().contains("\ncertifies-full true\n"));
    }

    /// Module extraction keeps the chain above the seed and leaves the sibling
    /// behind — and says how it decided every keep.
    #[test]
    fn extract_module_is_smaller_than_the_ontology() {
        let extracted =
            extract_module_to_string(TAXONOMY, "<http://example.org/A>\n", "bot").expect("extract");
        assert!(extracted.answer().contains("<http://example.org/A>"));
        // The ⊥-module for {A} follows the chain UP; the sibling D is not on it.
        assert!(
            !extracted.answer().contains("<http://example.org/D>"),
            "{}",
            extracted.answer()
        );
        assert!(extracted.certificate().contains("\nmethod BOT\n"));
        // Fewer axioms than the ontology has: a module that kept everything would
        // be sound and useless, so the count is the load-bearing measurement.
        let axioms: usize = extracted
            .certificate()
            .lines()
            .find_map(|line| line.strip_prefix("axioms "))
            .and_then(|count| count.parse().ok())
            .expect("an axiom count");
        assert!(
            (1..TAXONOMY.lines().count()).contains(&axioms),
            "{}",
            extracted.certificate()
        );
        // Every keep was decided by the locality rules, which is the strongest
        // thing an extraction can say.
        assert!(extracted.certificate().ends_with("conservative false\n"));
        // …and the three methods are all reachable and distinct in the rendering.
        for method in MODULE_METHOD_NAMES {
            let produced = extract_module_to_string(TAXONOMY, "<http://example.org/A>\n", method)
                .unwrap_or_else(|error| panic!("{method}: {error}"));
            assert!(
                produced
                    .certificate()
                    .contains(&format!("\nmethod {}\n", method.to_ascii_uppercase())),
                "{method}"
            );
        }
    }

    /// An unknown module method is refused with the accepted set named.
    #[test]
    fn an_unknown_module_method_names_the_accepted_set() {
        let error = extract_module_to_string(TAXONOMY, "", "nested").expect_err("unknown");
        assert!(error.contains("nested"), "{error}");
        for method in MODULE_METHOD_NAMES {
            assert!(error.contains(method), "{error} omits {method}");
        }
    }

    /// A justification is minimal AND sufficient, and both halves are RE-DECIDED
    /// here rather than restated from the search that found it.
    #[test]
    fn justify_re_decides_both_halves_of_its_claim() {
        let why = justify_to_string(TAXONOMY, CHAIN_AXIOM).expect("entailed");
        // The chain, and NOT the sibling: two axioms of the four.
        assert_eq!(why.answer().lines().count(), 2, "{}", why.answer());
        assert!(!why.answer().contains("<http://example.org/D>"));
        assert!(why.certificate().contains("\nsufficient true\n"));
        assert!(why.certificate().contains("\nminimal true\n"));
        assert!(why.certificate().ends_with("overclaims false\n"));
        // The identity is a CONTENT digest, never an IRI.
        let digest = why
            .certificate()
            .lines()
            .find_map(|line| line.strip_prefix("digest "))
            .expect("a digest");
        assert_eq!(digest.len(), 64);
        assert!(
            digest
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
        );
        // …and the axiom it justifies is echoed, so the answer is self-describing.
        assert!(why.certificate().contains("\naxiom SubClassOf\n"));
    }

    /// An unentailed axiom has NO justification, and that is a refusal rather than
    /// an empty set — which would read as "nothing is needed" and mean the opposite.
    #[test]
    fn an_unentailed_axiom_has_no_justification() {
        let reversed = "<http://example.org/C> \
<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/A> .\n";
        let error = justify_to_string(TAXONOMY, reversed).expect_err("not entailed");
        assert!(error.starts_with("justify: "), "{error}");
        assert!(error.contains("does not entail"), "{error}");
    }

    /// A chase proof RE-DERIVES its conclusion; the certificate reports what the
    /// checker computed beside what the proof claims.
    #[test]
    fn explain_conclusion_re_derives_rather_than_re_reads() {
        let derived = "<http://example.org/x> \
<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C> .\n";
        let why = explain_conclusion_to_string(TAXONOMY, "owl-rl", derived).expect("derived");
        assert!(why.answer().starts_with("asserted false\n"));
        assert!(
            why.answer().contains("\nrule cax-sco\n"),
            "{}",
            why.answer()
        );
        assert!(why.certificate().contains("\nchecked true\n"));
        assert!(why.certificate().ends_with("overclaims false\n"));
        // The re-derived fact and the stated conclusion agree, line for line.
        let field = |key: &str| {
            why.certificate()
                .lines()
                .find_map(|line| line.strip_prefix(key).map(str::to_owned))
                .unwrap_or_else(|| panic!("{key}"))
        };
        assert_eq!(field("conclusion-subject "), field("derived-subject "));
        assert_eq!(field("conclusion-predicate "), field("derived-predicate "));
        assert_eq!(field("conclusion-object "), field("derived-object "));
        // An ASSERTED triple is explained by the fact that it is asserted, which is
        // a real explanation and a checkable one.
        let asserted = "<http://example.org/x> \
<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .\n";
        let given = explain_conclusion_to_string(TAXONOMY, "owl-rl", asserted).expect("asserted");
        assert!(given.answer().starts_with("asserted true\n"));
        assert!(given.certificate().contains("\nchecked true\n"));
    }

    /// The two lanes whose rules mint a fresh blank node are refused BY NAME, and
    /// an underivable conclusion is a hard error rather than an empty explanation.
    #[test]
    fn an_unexplainable_conclusion_is_refused_by_name() {
        let derived = "<http://example.org/x> \
<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C> .\n";
        for regime in ["rdf", "rdfs"] {
            let error = explain_conclusion_to_string(TAXONOMY, regime, derived)
                .expect_err("existential heads");
            assert!(error.contains("existential"), "{error}");
        }
        let absent = "<http://example.org/nobody> \
<http://example.org/nothing> <http://example.org/nowhere> .\n";
        let error =
            explain_conclusion_to_string(TAXONOMY, "owl-rl", absent).expect_err("not derived");
        assert!(error.contains("no derivation"), "{error}");
    }

    // ── The boundary's term syntax ──────────────────────────────────────────

    /// A term round-trips through the boundary's syntax, for every kind a DL
    /// answer can carry.
    #[test]
    fn terms_round_trip_through_the_boundary_syntax() {
        for (term, rendered) in [
            (
                TermValue::iri("http://example.org/A"),
                "<http://example.org/A>",
            ),
            (TermValue::blank("b0"), "_:b0"),
        ] {
            assert_eq!(emit(&term), rendered);
            assert_eq!(parse_one_term(rendered).expect("a term"), term);
        }
        // A literal renders (an answer may carry one) but is NOT a name, so it is
        // refused in the positions that take a class, an individual or a property.
        assert_eq!(
            emit(&TermValue::lang_literal("hello world", "EN")),
            "\"hello world\"@en"
        );
        assert!(parse_one_term("\"hello\"").is_err());
    }

    /// A malformed term, axiom or document is an error, never a silent empty answer.
    #[test]
    fn malformed_boundary_input_is_an_error() {
        assert!(consistency_to_string("this is not n-quads\n", 0).is_err());
        assert!(profile_to_string("this is not n-quads\n").is_err());
        assert!(instances_to_string(TAXONOMY, "not a term", 0).is_err());
        // Two terms where one was asked for.
        assert!(
            instances_to_string(TAXONOMY, "<http://example.org/A> <http://example.org/B>", 0)
                .is_err()
        );
        // Two statements where one axiom was asked for.
        let two = format!(
            "{CHAIN_AXIOM}<http://example.org/D> \
<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .\n"
        );
        assert!(entails_to_string(TAXONOMY, &two, 0).is_err());
        // An axiom is one triple and is not graph-scoped.
        let scoped = "<http://example.org/A> \
<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> \
<http://example.org/g> .\n";
        let error = entails_to_string(TAXONOMY, scoped, 0).expect_err("graph-scoped");
        assert!(error.contains("names a graph"), "{error}");
    }
}
