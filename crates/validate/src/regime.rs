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

use purrdf_core::{RdfLiteral, RdfTerm, RdfTriple, TermValue, display_term};
use purrdf_entail::{
    ChaseProof, ClaimSubject, Completeness, DlAxiom, DlCertificate, DlCompleteness, EntailError,
    EntailmentCertificate, EntailmentMechanism, EntailmentOutcome, ImportMap, Justification,
    Materialization, ModuleExtraction, ModuleMethod, OwlProfile, ProfileCertificate, Question,
    Reasoner, ReasoningReport, Regime, RuleSet, Service, ServiceProof, VarKey, Verdict,
    explain_conclusion, extensions, extract_module, extract_module_with_proofs, implemented,
    justify, materialize, parse_rif_xml, profile, rules,
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
/// `4` because the grammar moved, which is the whole reason a banner is emitted at all.
/// Against `3`, ONE line is new and nothing was removed:
///
/// * `mechanism none | mechanism <name> <why>` — WHICH of the conclusion-directed
///   entailment service's seven mechanisms read an answer off this run, and the semantic
///   boundary of the rule table that mechanism crosses.
///   [`purrdf_entail::entails()`] reaches a conclusion six ways, and five of them exist
///   because the regime's rule table decides no conclusion of that shape — a
///   negative fact, a schema axiom, an anonymous class expression, a self-loop over a
///   construct outside the syntax, a containment between value spaces. Without this line
///   "the rule table decided this" and "the table has no head of this shape and a second
///   run over the premise's negation did" rendered as the same report, so the one fact a
///   reader most needs about a `yes` — how it was reached — was the one fact the report
///   did not carry. `none` is a materialization: it answered no conclusion-directed
///   question, and saying so is not the same as having no mechanism to name.
///
/// Against `2`, two lines were new: `extension <rule-id>` — the rules the run's calculus
/// states that NO specification table does, without which a caller reading `completeness
/// exact-within-boundaries` and `fired ext-eq-diff-sym 1` had to know from prose that one
/// of those ids is not in OWL 2 Profiles §4.3 and the other seventy-eight are — and
/// `termination …`, the weak-acyclicity certificate the restricted chase computed to admit
/// the program it then ran, which was computed on every `rdf` and `rdfs` run and read by
/// nothing.
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
pub const REPORT_FORMAT_BANNER: &str = "purrdf-reasoning-report 4";

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
    // No caller base: `program` crossed a STRING boundary, so it has no retrieval IRI
    // this layer could honestly supply (the same reason every other codec entry point
    // here refuses to invent one). An in-document `xml:base` still governs; without one,
    // a relative IRI reference in the rule document hard-fails rather than becoming a
    // silently-renamed predicate.
    let parsed = parse_rif_xml(program, None)
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
/// The empty string for a regime with no rule table of its own (`simple`, plus
/// `owl-direct`, which decides through the tableau, and `rif`, which entails under the
/// caller's rules — all three still MATERIALIZE). Lines are in specification table
/// order — the
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

/// The rules `regime`'s lane fires BEYOND its specification table, one name per line.
///
/// The empty string for a lane with nothing added to it. These names appear in neither
/// [`rules_string`] nor [`implemented_rules_string`]: the normative table is a statement
/// about the specification and does not move because this workspace fires a sound rule the
/// table happens not to list. A rendered report discloses the same names on its
/// `extension` line, but a caller should not have to materialize a dataset to find out
/// what a build adds — that is the question this answers.
///
/// # Errors
///
/// An unknown `regime` spelling; the message names the accepted set.
///
/// ```
/// use purrdf_validate::regime::{extension_rules_string, implemented_rules_string, rules_string};
///
/// let added = extension_rules_string("owl-rl").expect("known");
/// // Whatever the lane adds is disjoint from the table it is defined by, and from the
/// // subset of that table which fires. Both hold by construction, for every lane.
/// for rule in added.lines() {
///     assert!(!rules_string("owl-rl").expect("known").lines().any(|r| r == rule));
///     assert!(
///         !implemented_rules_string("owl-rl")
///             .expect("known")
///             .lines()
///             .any(|r| r == rule)
///     );
/// }
/// // Extending a lane is a decision taken per lane; RDFS has had none taken for it.
/// assert_eq!(extension_rules_string("rdfs").expect("known"), "");
/// ```
pub fn extension_rules_string(regime: &str) -> Result<String, String> {
    Ok(rule_lines(extensions(parse_regime(regime)?)))
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
/// purrdf-reasoning-report 4
/// regime <cli-spelling>
/// completeness exact | completeness exact-within-boundaries | completeness sound-incomplete <count>
/// missing <rule-id>                       (0..n, specification table order)
/// extension <rule-id>                     (0..n, declaration order)
/// fired <rule-id> <conclusions>           (0..n, specification table order)
/// boundary <construct> <reason>           (0..n, Construct declaration order)
/// mechanism none | mechanism <name> <why>
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
/// `mechanism` sits with the `boundary` lines above it because it answers the same question
/// one step further out: those say what the RUN could not fully handle, and this says which
/// of [`purrdf_entail::entails()`]'s seven mechanisms actually read the answer off it, together
/// with the semantic boundary of the rule table that mechanism crosses. `strict-table` is a
/// positive claim — the regime's own table was run once and the conclusion was matched into
/// (or proven absent from) its closure — and it is the only spelling a `not entailed` can
/// carry, because refuting needs the completeness half of a theorem and only the table has
/// one. The other five (`refutation`, `freeze`, `comprehension`, `reflexivity`,
/// `data-range`) each exist because the table DECIDES no conclusion of that shape — Theorem
/// PR1 claims completeness only for assertional conclusions over named individuals, and for
/// several of them Tables 4–9 state no head of the shape at all — and none of them adds a
/// rule: `missing`, `extension` and `fired` above are byte-identical
/// whichever answered. `composite` is two or more of those five folded over one conclusion —
/// a conclusion GRAPH is a conjunction, so it can need a lane per half — and it is spelled
/// that way rather than by any constituent's name, which would tell a reader that one
/// mechanism sufficed. `none` is a materialization, which asked no conclusion-directed
/// question at all. The name is the mechanism's own `as_str` spelling and never an enum
/// ordinal, so an eighth arm cannot silently renumber a consumer's reading of an old one.
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
/// document, an unsatisfiable tableau, a regime the conclusion-directed service is not
/// total over, an unresolved `owl:imports`, an exhausted match budget — and has no report to
/// carry, so it renders as its own diagnostic and nothing is implied about a closure that
/// was never assembled.
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
        | EntailError::UnsupportedRegime(_)
        | EntailError::UnresolvedImport(_)
        | EntailError::MatchBudget
        | EntailError::Unsatisfiable => head,
        // `EntailError` is `#[non_exhaustive]`, so a variant added in its own crate arrives
        // here without a compile error. Folding it in with the arm above is correct by
        // construction rather than by default: the split this match makes is "carries a
        // run's report" against "is the absence of a run", and only `Inconsistent` carries
        // one — an `InconsistentRun` is the single payload a report can be read out of. A
        // variant this match has never seen cannot be known to carry a report, so rendering
        // it as its own diagnostic states exactly what is known and implies nothing about a
        // closure that was never assembled.
        _ => head,
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
        // Beside the boundaries, and one step further out: they say what the run could not
        // fully handle, this says which mechanism read an answer off it and which semantic
        // boundary of the rule table that mechanism crosses.
        match report.mechanism() {
            None => writeln!(f, "mechanism none")?,
            Some(mechanism) => {
                writeln!(f, "mechanism {} {}", mechanism.as_str(), mechanism.reason())?;
            }
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
    /// The rendered [`ServiceProof`] of the run that produced it, ABSENT when nobody asked
    /// for one. See [`Self::proof`] for why absence is a value rather than an empty proof.
    proof: Option<String>,
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
    /// Never empty. Some services terminate their certificate with an explicit honesty
    /// gate literal — `checked` for a re-derived chase proof, `one-directional` for
    /// profile certification, `conservative` for a module extraction — so a consumer can
    /// read the gate without re-deriving it. Each of those reports something its own
    /// lines do not already contain. Where a would-be gate is a boolean function of
    /// lines already rendered it is ABSENT rather than restated: a justification ends on
    /// `minimal`, because `!(sufficient && minimal)` adds no bit, and the DL certificate
    /// (see [`render_dl_certificate`]) ends on its measurements, because `completeness`
    /// cannot read `decided` beside a non-empty `boundary` list by construction.
    #[must_use]
    pub fn certificate(&self) -> &str {
        &self.certificate
    }

    /// The rendered PROOF TERM of the run that produced [`Self::answer`], when one was
    /// recorded.
    ///
    /// `None` is not an empty proof, and the distinction is the whole reason this is an
    /// `Option`. Recording is OPT-IN — [`ReasonerSession::open`] records nothing and
    /// [`ReasonerSession::open_with_proofs`] records — so a service nobody asked to record
    /// answers `None` here, meaning NOTHING WAS MEASURED. A recorded proof with `runs 0` in
    /// it means something entirely different: that the service is syntactic, so there was no
    /// search to check, and the checker verified exactly that. Collapsing the two would let
    /// "never recorded" be presented as "verified, and there was nothing to verify", which is
    /// the one substitution this surface must never make.
    ///
    /// [`Self::proof_document`] is the same three-way fact for a host that carries strings
    /// and has no `Option`.
    #[must_use]
    pub fn proof(&self) -> Option<&str> {
        self.proof.as_deref()
    }

    /// [`Self::proof`] as a document that is never empty — the form every host boundary
    /// carries.
    ///
    /// An FFI surface has no `Option`, and an empty string would be a third spelling of
    /// "nothing" a consumer could read as either of the other two. So an unrecorded answer
    /// renders [`ABSENT_DL_PROOF`], which SAYS `availability not-recorded` in the same
    /// grammar a recorded proof uses — and [`check_dl_proof`] refuses that document by name
    /// rather than reporting a verification of it.
    #[must_use]
    pub fn proof_document(&self) -> &str {
        self.proof.as_deref().unwrap_or(ABSENT_DL_PROOF)
    }

    /// Consume the answer, yielding `(answer, certificate)`.
    ///
    /// Drops the proof, which is why it is not the way a host reaches one: see
    /// [`Self::into_proved_parts`].
    #[must_use]
    pub fn into_parts(self) -> (String, String) {
        (self.answer, self.certificate)
    }

    /// Consume the answer, yielding `(answer, certificate, proof_document)`.
    ///
    /// The three-string shape a host that asked for a proof carries. The third is never
    /// empty: see [`Self::proof_document`].
    #[must_use]
    pub fn into_proved_parts(self) -> (String, String, String) {
        let proof = self.proof.unwrap_or_else(|| ABSENT_DL_PROOF.to_owned());
        (self.answer, self.certificate, proof)
    }
}

// ── Term syntax at the boundary ─────────────────────────────────────────────

/// Render `term` in N-Triples term syntax (`<iri>`, `_:label`, `"lex"@en`,
/// `<<( s p o )>>`).
///
/// The escaping is [`purrdf_core::display_term`]'s, so a term rendered here and
/// the same term rendered by the native serializers escape identically. This is
/// report/diagnostic identity text (answer and certificate lines), not RDF
/// document egress, so blank-node label alphabets are deliberately not enforced
/// here and the function stays total. Triple terms recurse HERE rather than
/// through `display_term`'s owned model, because the owned model requires a
/// triple term's predicate to be an IRI and this function must be total over
/// [`TermValue`].
///
/// N-Triples terms are self-delimiting — `<…>` ends at the unescaped `>`, `_:…` at
/// whitespace, `"…"` at the unescaped closing quote — which is what makes a
/// two-term line like `subclass <C> <D>` unambiguous even though a literal's
/// lexical form may contain a space.
fn emit(term: &TermValue) -> String {
    match term {
        TermValue::Iri(iri) => display_term(&RdfTerm::iri(iri.clone())),
        TermValue::Blank { label, scope } => display_term(&RdfTerm::blank_node(
            scope.qualify_label(label).into_owned(),
        )),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => display_term(&RdfTerm::literal(RdfLiteral {
            lexical_form: lexical_form.clone(),
            datatype: Some(datatype.clone()),
            language: language.clone(),
            direction: *direction,
        })),
        TermValue::Triple { s, p, o } => match to_owned_term(term) {
            Some(owned) => display_term(&owned),
            // A triple term whose predicate is not an IRI — at THIS nesting level or
            // any level nested inside `s`/`o` — is not a well-formed RDF triple, so
            // the owned model cannot hold it anywhere along the chain. Rendering it
            // structurally is the honest option: the caller sees what the term
            // actually is, including the real offending predicate, rather than a
            // fabricated empty IRI standing in for it.
            None => format!("<<( {} {} {} )>>", emit(s), emit(p), emit(o)),
        },
    }
}

/// The owned-model twin of a [`TermValue`], for [`emit`].
///
/// `None` iff `term` — or any triple term nested inside it, at any depth — has a
/// predicate that is not an IRI. The owned model ([`RdfTriple`]) requires an IRI
/// predicate by construction, so there is no owned value to return for such a
/// term; callers fall back to [`emit`]'s structural `<<( … )>>` rendering, which
/// shows the real offending term instead of a fabricated placeholder.
fn to_owned_term(term: &TermValue) -> Option<RdfTerm> {
    match term {
        TermValue::Iri(iri) => Some(RdfTerm::iri(iri.clone())),
        TermValue::Blank { label, scope } => {
            Some(RdfTerm::blank_node(scope.qualify_label(label).into_owned()))
        }
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => Some(RdfTerm::literal(RdfLiteral {
            lexical_form: lexical_form.clone(),
            datatype: Some(datatype.clone()),
            language: language.clone(),
            direction: *direction,
        })),
        TermValue::Triple { s, p, o } => {
            let predicate = p.as_iri()?;
            Some(RdfTerm::triple(RdfTriple::new(
                to_owned_term(s)?,
                predicate.to_owned(),
                to_owned_term(o)?,
            )))
        }
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
/// work <n>
/// work-budget <n>
/// decisions <n>
/// peak-nodes <n>
/// disjunctions <n>
/// peak-depth <n>
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
/// `work` and `work-budget` are the SECOND budget, and they are what say which cap a
/// `budget-exhausted` run reached. A round is a pass rather than a unit of cost — its
/// price is the completion graph it runs over times the clauses matched against it — so
/// an ontology can make every round enormously more expensive without making the search
/// take more rounds, and a certificate reporting three percent of its `budget` while the
/// run grinds is reporting the only number it had. `work` counts the matcher join steps,
/// successor-subset enumerations, achiever closures, neighbour scans and branch-state
/// clones the run actually performed; it sums over decisions exactly as `steps` does, and
/// it is a count rather than a clock, so it too is identical on `wasm32`.
///
/// The last three lines say WHERE those rounds went, which `steps` alone cannot:
/// `peak-nodes` is the largest completion graph any decision built,
/// `disjunctions` is how many times the `⊔`-rule case split (summed, and often
/// zero — an ontology whose inclusions all absorb is decided without a split), and
/// `peak-depth` is the deepest that rule's branch stack got. The two peaks are
/// MAXIMA over the run's decisions and `disjunctions` is a SUM, because the first
/// two are sizes reached and the third is work done. All three are counts over a
/// deterministic search, so they move only when the search does.
///
/// There is deliberately no trailing `overclaims` line. [`DlCertificate`] stores no
/// completeness field beside its boundary list for one to contradict:
/// [`DlCertificate::completeness`] derives `decided` / `decided-within-boundaries` /
/// `budget-exhausted` from the boundary list on every call, so `decided` beside a
/// non-empty `boundary` line is not a value this function — or any other caller — has a
/// constructor for. A rendered constant is a disclosure, not a gate, so unlike the CHASE
/// report's former line (see [`ReasoningReport`]), this one was never added in the first
/// place.
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
    let _ = writeln!(out, "work {}", certificate.work());
    let _ = writeln!(out, "work-budget {}", certificate.work_budget());
    let _ = writeln!(out, "decisions {}", certificate.decisions());
    let _ = writeln!(out, "peak-nodes {}", certificate.peak_nodes());
    let _ = writeln!(out, "disjunctions {}", certificate.disjunctions());
    let _ = writeln!(out, "peak-depth {}", certificate.peak_depth());
    out
}

// ── Proof rendering: a text ENVELOPE, not a pretty-printer ──────────────────

/// The banner every rendered [`ServiceProof`] opens with.
pub const DL_PROOF_BANNER: &str = "purrdf-dl-proof 1";

/// The banner every [`check_dl_proof`] report opens with.
///
/// Deliberately not [`DL_PROOF_BANNER`]: a proof and a report ABOUT a proof are different
/// documents, and a consumer that parsed one as the other would read a summary of a check as
/// the thing checked.
pub const DL_PROOF_CHECK_BANNER: &str = "purrdf-dl-proof-check 1";

/// The document an answer produced WITHOUT recording carries in place of a proof.
///
/// Never the empty string, and never a proof term with nothing in it. An FFI surface has no
/// `Option`, so the absence has to be said out loud in the same grammar a present proof uses —
/// otherwise "nobody asked" arrives at a host as a blank, and a blank is what "verified,
/// nothing to check" also looks like. [`check_dl_proof`] refuses this document by name.
pub const ABSENT_DL_PROOF: &str = "purrdf-dl-proof 1\navailability not-recorded\n";

/// The services a [`ServiceProof`] can be about, in the spellings this boundary uses
/// everywhere else — the `service <name>` line of a rendered certificate, the CLI's own
/// words, and the `service` argument [`prove_to_string`] and [`check_dl_proof`] take.
pub const PROOF_SERVICE_NAMES: [&str; 7] = [
    "consistency",
    "class-satisfiability",
    "classify",
    "realize",
    "instances",
    "entails",
    "extract-module",
];

/// How many proof BYTES one `body` line carries, as `2 * n` lowercase hex digits.
const PROOF_BODY_BYTES_PER_LINE: usize = 32;

/// This boundary's name for `service`.
///
/// Total, and deliberately NOT [`Service::as_str`]: `purrdf-entail` names the classification
/// service `classification`, and every string this boundary has ever emitted for it — the
/// certificate's `service` line, the CLI subcommand, the WASM and Python method — says
/// `classify`. One vocabulary at the boundary is worth more than agreeing with an internal
/// one, so the map is here and [`parse_proof_service`] is its exact inverse.
const fn proof_service_name(service: Service) -> &'static str {
    match service {
        Service::Consistency => "consistency",
        Service::ClassSatisfiability => "class-satisfiability",
        Service::Classification => "classify",
        Service::Realization => "realize",
        Service::InstanceRetrieval => "instances",
        Service::AxiomEntailment => "entails",
        Service::ModuleExtraction => "extract-module",
        // `Service` is `#[non_exhaustive]`; a service added upstream without a name here is a
        // service this boundary cannot render, and saying so is better than inventing a name.
        _ => "unknown",
    }
}

/// Parse a service from its [`PROOF_SERVICE_NAMES`] spelling.
///
/// # Errors
///
/// A spelling outside the accepted set, which the message names in full so a caller three
/// language boundaries away can fix the call without reading this source.
fn parse_proof_service(name: &str) -> Result<Service, String> {
    match name {
        "consistency" => Ok(Service::Consistency),
        "class-satisfiability" => Ok(Service::ClassSatisfiability),
        "classify" => Ok(Service::Classification),
        "realize" => Ok(Service::Realization),
        "instances" => Ok(Service::InstanceRetrieval),
        "entails" => Ok(Service::AxiomEntailment),
        "extract-module" => Ok(Service::ModuleExtraction),
        other => Err(format!(
            "unknown proof service \"{other}\"; accepted: {}",
            PROOF_SERVICE_NAMES.join(", ")
        )),
    }
}

/// `bytes` as lowercase hex.
fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Parse lowercase hex back to bytes, refusing an odd length or a non-hex digit.
fn unhex(text: &str) -> Result<Vec<u8>, String> {
    if !text.len().is_multiple_of(2) {
        return Err("a proof body line has an odd number of hex digits".to_owned());
    }
    let mut out = Vec::with_capacity(text.len() / 2);
    for pair in text.as_bytes().as_chunks::<2>().0 {
        let digit = |byte: u8| match byte {
            b'0'..=b'9' => Ok(byte - b'0'),
            b'a'..=b'f' => Ok(byte - b'a' + 10),
            other => Err(format!(
                "a proof body carries {:?}, which is not a lowercase hex digit",
                char::from(other)
            )),
        };
        out.push(digit(pair[0])? << 4 | digit(pair[1])?);
    }
    Ok(out)
}

/// Render a [`ServiceProof`] to the boundary's byte-stable textual form.
///
/// The grammar, in emission order — one fact per line, `\n`-terminated:
///
/// ```text
/// purrdf-dl-proof 1
/// service <name>                  (one of PROOF_SERVICE_NAMES)
/// availability recorded
/// input <64 lowercase hex>        (the ontology's RDFC-1.0 identity)
/// digest <64 lowercase hex>       (BLAKE3 over the canonical bytes below)
/// trust-base <a,b,c>              (what a verification of this proof RESTS ON)
/// runs <n>                        (tableau runs the service made)
/// traced <n>                      (how many of them kept a replayable trace)
/// claims <n>
/// truncated true | false
/// receipt none | receipt <cause>  (caller-stop | work-cap | round-cap | nested-ceiling)
/// bytes <n>
/// body <hex>                      (ceil(n / 32) lines, 64 hex digits each but the last)
/// ```
///
/// # Why an envelope around canonical bytes, and not a rendering of the term
///
/// This was a deliberate choice between two shapes, and the alternative was rejected rather
/// than not considered.
///
/// A [`ServiceProof`] transitively contains one [`purrdf_entail::DlProof`] per tableau run,
/// and a `DlProof` is a completion graph: every node with its concept label, its nominals and
/// its distinctness set, every edge, every blocking pair, every branch point with all of its
/// grounded alternatives, every clash with the frame that grounded it. All of it in the
/// reasoner's INTERNED ids, which mean nothing without the reverse mapping. A line-oriented
/// rendering of that is not a summary — it is the same information in a second syntax, and it
/// would have to be exact, because [`ServiceProof::verify`] replays those very structures. Two
/// exact syntaxes for one term is two things to keep byte-identical across four hosts instead
/// of one, and the cost of getting the second one subtly wrong is a proof that checks against
/// a graph slightly unlike the one the search built.
///
/// The shape that is NOT acceptable is the third one: a readable digest of the term — "12
/// runs, 40 claims, refutation closed" — presented as a proof. A consumer holding that
/// cannot verify anything. It is a report about a proof wearing the word "proof", and the
/// issue this implements asks for the opposite.
///
/// So the text carries the canonical bytes verbatim, in a deterministic encoding, and every
/// header line above it is DERIVED from those same bytes. The header is a courtesy for a
/// reader; it is never evidence. [`decode_dl_proof`] re-renders the decoded term and refuses
/// the document unless it is byte-identical, so a header that disagrees with the body is a
/// rejection rather than a friendlier summary a forger got to write.
///
/// # Determinism
///
/// Every line is a fixed keyword and an integer, a fixed-order enum name, or hex over
/// [`ServiceProof::encode`] — which is itself byte-identical run to run and on `wasm32`. No
/// clock, no map iteration, no host paths, no floating point, and the hex is lowercase and
/// fixed-width, so a 32-bit host and a 64-bit host emit the same bytes.
#[must_use]
pub fn render_dl_proof(proof: &ServiceProof) -> String {
    let bytes = proof.encode();
    let mut out = String::with_capacity(bytes.len() * 2 + 512);
    out.push_str(DL_PROOF_BANNER);
    out.push('\n');
    let _ = writeln!(out, "service {}", proof_service_name(proof.service()));
    out.push_str("availability recorded\n");
    let _ = writeln!(out, "input {}", hex(&proof.input()));
    let _ = writeln!(out, "digest {}", proof.digest_hex());
    let _ = writeln!(
        out,
        "trust-base {}",
        proof
            .trust_base()
            .iter()
            .map(|entry| entry.as_str())
            .collect::<Vec<_>>()
            .join(",")
    );
    let _ = writeln!(out, "runs {}", proof.runs().len());
    let _ = writeln!(
        out,
        "traced {}",
        proof
            .runs()
            .iter()
            .filter(|run| run.proof().is_some())
            .count()
    );
    let _ = writeln!(out, "claims {}", proof.claims().len());
    let _ = writeln!(out, "truncated {}", proof.truncated());
    match proof.receipt() {
        None => out.push_str("receipt none\n"),
        Some(receipt) => {
            let _ = writeln!(out, "receipt {}", receipt.cause().as_str());
        }
    }
    let _ = writeln!(out, "bytes {}", bytes.len());
    for chunk in bytes.chunks(PROOF_BODY_BYTES_PER_LINE) {
        let _ = writeln!(out, "body {}", hex(chunk));
    }
    out
}

/// Read a `purrdf-dl-proof 1` document back into the term it renders.
///
/// The UNTRUSTED entrance for the TEXT layer, exactly as [`ServiceProof::decode`] is for the
/// byte layer, and it defers to that one for everything structural. What it adds is the
/// envelope's own discipline:
///
/// * the banner must be the first line, so a document in some other grammar is refused rather
///   than parsed hopefully;
/// * `availability not-recorded` is refused BY NAME — there is no proof in such a document,
///   and reporting a verification of it is the one substitution this whole surface exists to
///   prevent;
/// * the header is not believed. The decoded term is re-rendered and compared to the document
///   byte for byte, so every derived line — the digest, the run and claim counts, the trust
///   base, the stopping cause — is checked against the bytes rather than read off a line a
///   forger wrote. A document whose `runs 0` sits above a body carrying six runs is a
///   rejection.
///
/// # Errors
///
/// A document that is not this grammar, an unrecorded one, a body that is not lowercase hex,
/// bytes that are not a [`ServiceProof`], or a header that does not match its own body.
pub fn decode_dl_proof(document: &str) -> Result<ServiceProof, String> {
    let mut lines = document.lines();
    if lines.next() != Some(DL_PROOF_BANNER) {
        return Err(format!(
            "a proof document opens with {DL_PROOF_BANNER:?}, and this one does not"
        ));
    }
    if document.contains("\navailability not-recorded\n") {
        return Err(
            "this answer carries no proof: nothing was recorded, because nobody asked. That \
             is not the same as a proof with nothing in it, and it is not something a check \
             can report as verified — ask for the proof (open the session with proofs, or \
             call the `prove` entry point) and check that"
                .to_owned(),
        );
    }
    let mut bytes = Vec::new();
    for line in lines {
        if let Some(hex) = line.strip_prefix("body ") {
            bytes.extend_from_slice(&unhex(hex)?);
        }
    }
    if bytes.is_empty() {
        return Err("a recorded proof document carries no `body` line".to_owned());
    }
    let proof = ServiceProof::decode(&bytes)
        .map_err(|error| format!("the proof body is not a proof term: {error}"))?;
    let rendered = render_dl_proof(&proof);
    if rendered != document {
        return Err(
            "the proof document's header does not describe the proof its own body carries, \
             so at least one of the two was rewritten after the proof was issued"
                .to_owned(),
        );
    }
    Ok(proof)
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

/// A reasoning session over ONE ontology.
///
/// Every service below answers a question about a document, and each one's free-function
/// form takes that document as a `&str`. A caller asking three questions therefore parses
/// the document three times and reverse-maps it three times, even though the document is
/// the expensive part and does not change between questions. This type holds it: [`open`]
/// pays the parse once, the first knowledge-base service pays the reverse-mapping once,
/// and every later question reuses both.
///
/// The free functions are thin wrappers over this type rather than a second
/// implementation, so a session answer and a one-shot answer cannot drift apart.
///
/// # Laziness here is semantics, not an optimization
///
/// The knowledge base is built by the first service that needs one and NOT by [`open`].
/// Four services ([`Self::profile`], [`Self::extract_module`], [`Self::justify`],
/// [`Self::explain_conclusion`]) read the dataset and never reason, and `profile` is
/// documented to answer for *any* parseable document — including one whose `owl:hasKey`
/// axioms exhaust the tableau while [`Reasoner::new`] applies them. Building eagerly
/// would make those services start failing on documents they answer today.
///
/// [`open`]: Self::open
///
/// ```
/// use purrdf_validate::regime::ReasonerSession;
///
/// let data = "<http://example.org/tom> \
///     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Cat> .\n";
/// let mut session = ReasonerSession::open(data, 0, 0).expect("parses");
/// // Two questions, one parse and one reverse mapping.
/// assert_eq!(session.consistency().expect("decides").answer(), "consistency true\n");
/// assert!(session.classify().expect("decides").answer().contains("subclass"));
/// ```
pub struct ReasonerSession {
    /// The parsed document. Every service reads it; `Arc` because parsing hands one back.
    dataset: std::sync::Arc<purrdf_rdf::RdfDataset>,
    /// The requested per-decision ROUND cap, applied when the knowledge base is built.
    step_cap: u32,
    /// The requested per-decision WORK cap, applied when the knowledge base is built.
    work_cap: u32,
    /// The reverse-mapped knowledge base, built by the first service that needs it.
    reasoner: Option<Reasoner>,
    /// Whether this session RECORDS proof terms — set at [`ReasonerSession::open_with_proofs`]
    /// and read only when the knowledge base is built.
    ///
    /// A flag rather than two session types because it changes nothing a service DECIDES: it
    /// selects [`Reasoner::with_proofs`] over [`Reasoner::new`], which is an observation the
    /// decision core makes of itself and never a lever it reads.
    proofs: bool,
}

impl std::fmt::Debug for ReasonerSession {
    /// The session's SHAPE — how big the document is and whether it has reasoned yet.
    ///
    /// [`Reasoner`] elides its knowledge base for the reason given on its own `Debug`,
    /// and a dataset is likewise thousands of interned ids. What a reader wants from a
    /// debug line is the size of the problem and whether the reverse mapping has been
    /// paid for, which is what this prints.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReasonerSession")
            .field("quads", &self.dataset.quad_count())
            .field("step_cap", &self.step_cap)
            .field("work_cap", &self.work_cap)
            .field("reasoned", &self.reasoner.is_some())
            .field("proofs", &self.proofs)
            .finish_non_exhaustive()
    }
}

impl ReasonerSession {
    /// Parse `document` and open a session over it.
    ///
    /// `step_cap` and `work_cap` can only NARROW: [`Reasoner::with_step_cap`] and
    /// [`Reasoner::with_work_cap`] clamp to the knowledge base's own caps, which are pure
    /// functions of its size. `0` means "do not narrow" for BOTH rather than "a cap of
    /// zero", because a zero cap would exhaust every decision and make either parameter a
    /// footgun at three language boundaries.
    ///
    /// The two bound different quantities and neither implies the other: `step_cap` bounds
    /// derivation ROUNDS, `work_cap` bounds the matcher, scan, closure and clone WORK done
    /// inside them — which is what a rounds cap structurally cannot see.
    ///
    /// # Errors
    ///
    /// A malformed document (the native codec's own diagnostic). Nothing is reverse-mapped
    /// here, so an ontology whose knowledge base cannot be built still opens — and fails on
    /// the first service that needs one, with that service's own message.
    pub fn open(document: &str, step_cap: u32, work_cap: u32) -> Result<Self, String> {
        Self::opened(document, step_cap, work_cap, false)
    }

    /// Parse `document` and open a session that RECORDS a proof term for every service that
    /// has one.
    ///
    /// The opt-in. [`Self::open`] is unchanged and still records nothing, so a caller who
    /// does not ask pays exactly what they paid before: no clausification contract, no
    /// instrumented search, no kept traces. What this costs is real — the recorded
    /// completion graph of every tableau run — and it buys the one thing an unrecorded
    /// answer cannot have, which is a proof a consumer can check for themselves.
    ///
    /// It changes nothing a service DECIDES. Every verdict, every certificate counter and
    /// every rendered answer is identical to the same question asked through [`Self::open`];
    /// `a_proved_session_answers_exactly_what_an_unproved_one_answers` is what makes that
    /// falsifiable rather than asserted.
    ///
    /// `step_cap` and `work_cap` are [`Self::open`]'s.
    ///
    /// # Errors
    ///
    /// As [`Self::open`].
    pub fn open_with_proofs(document: &str, step_cap: u32, work_cap: u32) -> Result<Self, String> {
        Self::opened(document, step_cap, work_cap, true)
    }

    /// Parse `document` and open a session in the requested recording mode.
    fn opened(document: &str, step_cap: u32, work_cap: u32, proofs: bool) -> Result<Self, String> {
        let dataset = purrdf_rdf::parse_dataset(document.as_bytes(), INPUT_MEDIA_TYPE, None)
            .map_err(|diagnostic| diagnostic.to_string())?;
        Ok(Self {
            dataset,
            step_cap,
            work_cap,
            reasoner: None,
            proofs,
        })
    }

    /// Whether this session records proof terms.
    #[must_use]
    pub const fn records_proofs(&self) -> bool {
        self.proofs
    }

    /// The knowledge base, building it on first use.
    fn kb(&mut self) -> Result<&mut Reasoner, String> {
        if self.reasoner.is_none() {
            let reasoner = if self.proofs {
                Reasoner::with_proofs(&self.dataset)
            } else {
                Reasoner::new(&self.dataset)
            }
            .map_err(|error| format!("reasoner: {error}"))?;
            let reasoner = if self.step_cap == 0 {
                reasoner
            } else {
                reasoner.with_step_cap(u64::from(self.step_cap))
            };
            self.reasoner = Some(if self.work_cap == 0 {
                reasoner
            } else {
                reasoner.with_work_cap(u64::from(self.work_cap))
            });
        }
        Ok(self
            .reasoner
            .as_mut()
            .expect("the knowledge base was just built"))
    }
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

/// The services, as methods on a session.
///
/// Each one carries the doc comment of its free-function form rather than repeating it;
/// the answer and certificate grammars, and every error condition, are documented there.
/// The free function is a wrapper over the method, so the two cannot disagree.
///
/// Argument validation happens before the knowledge base is touched, preserving each
/// free function's error precedence: a `class` that is not one N-Triples term is reported
/// as such even for an ontology whose knowledge base could not have been built.
impl ReasonerSession {
    /// See [`consistency_to_string`].
    ///
    /// # Errors
    ///
    /// A knowledge base that cannot be built from the document.
    pub fn consistency(&mut self) -> Result<ReasoningAnswer, String> {
        let (verdict, certificate, proof) = self.kb()?.consistency().into_certified_parts();
        Ok(ReasoningAnswer {
            answer: format!("consistency {}\n", verdict_name(verdict)),
            certificate: render_dl_certificate("consistency", &certificate),
            proof: proof.as_ref().map(render_dl_proof),
        })
    }

    /// See [`classify_to_string`].
    ///
    /// # Errors
    ///
    /// A knowledge base that cannot be built, or an ontology with no model.
    pub fn classify(&mut self) -> Result<ReasoningAnswer, String> {
        let (hierarchy, certificate, proof) = self
            .kb()?
            .classify()
            .map_err(|error| service_error("classify", &error))?
            .into_certified_parts();
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
            proof: proof.as_ref().map(render_dl_proof),
        })
    }

    /// See [`realize_to_string`].
    ///
    /// # Errors
    ///
    /// A knowledge base that cannot be built, or an ontology with no model.
    pub fn realize(&mut self) -> Result<ReasoningAnswer, String> {
        let (realization, certificate, proof) = self
            .kb()?
            .realize()
            .map_err(|error| service_error("realize", &error))?
            .into_certified_parts();
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
            proof: proof.as_ref().map(render_dl_proof),
        })
    }

    /// See [`instances_to_string`].
    ///
    /// # Errors
    ///
    /// A `class` that is not one N-Triples term, a knowledge base that cannot be built,
    /// or an ontology with no model.
    pub fn instances(&mut self, class: &str) -> Result<ReasoningAnswer, String> {
        let term = parse_one_term(class)?;
        let (individuals, certificate, proof) = self
            .kb()?
            .instances(&term)
            .map_err(|error| service_error("instances", &error))?
            .into_certified_parts();
        let mut answer = String::new();
        for individual in &individuals {
            let _ = writeln!(answer, "instance {}", emit(individual));
        }
        Ok(ReasoningAnswer {
            answer,
            certificate: render_dl_certificate("instances", &certificate),
            proof: proof.as_ref().map(render_dl_proof),
        })
    }

    /// See [`entails_to_string`].
    ///
    /// # Errors
    ///
    /// An `axiom` that is not one triple, a knowledge base that cannot be built, or an
    /// ontology with no model.
    pub fn entails(&mut self, axiom: &str) -> Result<ReasoningAnswer, String> {
        let parsed = parse_axiom(axiom)?;
        let (verdict, certificate, proof) = self
            .kb()?
            .entails(&parsed)
            .map_err(|error| service_error("entails", &error))?
            .into_certified_parts();
        let mut answer = format!("entails {}\n", verdict_name(verdict));
        write_axiom(&parsed, &mut answer);
        Ok(ReasoningAnswer {
            answer,
            certificate: render_dl_certificate("entails", &certificate),
            proof: proof.as_ref().map(render_dl_proof),
        })
    }

    /// See [`profile_to_string`]. Purely syntactic — never builds a knowledge base.
    #[must_use]
    pub fn profile(&self) -> ReasoningAnswer {
        let certificate = profile(&self.dataset);
        ReasoningAnswer {
            answer: render_profile_answer(&certificate),
            certificate: render_profile_certificate(&certificate),
            proof: None,
        }
    }

    /// See [`extract_module_to_string`]. Never builds a knowledge base.
    ///
    /// # Errors
    ///
    /// An unknown `method`, a `signature` that is not N-Triples terms, or an extraction
    /// that fails.
    pub fn extract_module(&self, signature: &str, method: &str) -> Result<ReasoningAnswer, String> {
        let notion = parse_module_method(method)?;
        let seed = parse_signature(signature)?;
        // Two producers, one for each recording mode: `extract_module_with_proofs` records a
        // ZERO-RUN proof — a real measurement, saying this service is syntactic and had no
        // search to check — and `extract_module` records nothing at all. They are different
        // answers and must not be collapsed into one call with a discarded proof.
        let extraction = if self.proofs {
            extract_module_with_proofs(&self.dataset, &seed, notion)
        } else {
            extract_module(&self.dataset, &seed, notion)
        }
        .map_err(|error| format!("extract-module: {error}"))?;
        Ok(ReasoningAnswer {
            answer: purrdf_rdf::canonical_flat_nquads(extraction.module().as_ref())?,
            certificate: render_module_certificate(&extraction),
            proof: extraction.proof().map(render_dl_proof),
        })
    }

    /// See [`justify_to_string`]. Never builds a knowledge base.
    ///
    /// # Errors
    ///
    /// An `axiom` that is not one triple, or a justification that cannot be re-decided.
    pub fn justify(&self, axiom: &str) -> Result<ReasoningAnswer, String> {
        let parsed = parse_axiom(axiom)?;
        let justification =
            justify(&self.dataset, &parsed).map_err(|error| format!("justify: {error}"))?;
        Ok(ReasoningAnswer {
            answer: purrdf_rdf::canonical_flat_nquads(justification.ontology().as_ref())?,
            certificate: render_justification(&justification)?,
            proof: None,
        })
    }

    /// See [`explain_conclusion_to_string`]. Never builds a knowledge base.
    ///
    /// # Errors
    ///
    /// An unknown `regime`, a `conclusion` that is not one statement, or a conclusion the
    /// regime does not derive.
    pub fn explain_conclusion(
        &self,
        regime: &str,
        conclusion: &str,
    ) -> Result<ReasoningAnswer, String> {
        let parsed = parse_regime(regime)?;
        let (graph, subject, predicate, object) = parse_one_statement(conclusion)?;
        let proof = explain_conclusion(
            &self.dataset,
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
            proof: None,
        })
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
/// let decided = consistency_to_string(data, 0, 0).expect("reverse-maps");
/// assert_eq!(decided.answer(), "consistency true\n");
/// // The certificate is never optional and never claims more than it decided: `decided`
/// // is reported here only because the boundary list beside it is, in fact, empty.
/// assert!(decided.certificate().starts_with("purrdf-dl-certificate 1\n"));
/// assert!(decided.certificate().contains("\ncompleteness decided\n"));
/// assert!(!decided.certificate().contains("\nboundary "));
/// ```
pub fn consistency_to_string(
    document: &str,
    step_cap: u32,
    work_cap: u32,
) -> Result<ReasoningAnswer, String> {
    ReasonerSession::open(document, step_cap, work_cap)?.consistency()
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
pub fn classify_to_string(
    document: &str,
    step_cap: u32,
    work_cap: u32,
) -> Result<ReasoningAnswer, String> {
    ReasonerSession::open(document, step_cap, work_cap)?.classify()
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
pub fn realize_to_string(
    document: &str,
    step_cap: u32,
    work_cap: u32,
) -> Result<ReasoningAnswer, String> {
    ReasonerSession::open(document, step_cap, work_cap)?.realize()
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
    work_cap: u32,
) -> Result<ReasoningAnswer, String> {
    ReasonerSession::open(document, step_cap, work_cap)?.instances(class)
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
/// let decided = entails_to_string(data, asked, 0, 0).expect("decides");
/// assert!(decided.answer().starts_with("entails true\n"));
/// assert!(decided.answer().contains("\naxiom ClassAssertion\n"));
/// ```
pub fn entails_to_string(
    document: &str,
    axiom: &str,
    step_cap: u32,
    work_cap: u32,
) -> Result<ReasoningAnswer, String> {
    ReasonerSession::open(document, step_cap, work_cap)?.entails(axiom)
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
    Ok(ReasonerSession::open(document, 0, 0)?.profile())
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
    ReasonerSession::open(document, 0, 0)?.extract_module(signature, method)
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
/// ```
///
/// `sufficient` and `minimal` are **re-decided here**, over the justification alone
/// and over each of its one-axiom-smaller subsets. They do not consult the search
/// that found the justification and cannot be misled by it, which is what makes them
/// a check rather than a restatement. Both must hold for the answer to be a
/// justification rather than something weaker: a subset that does not entail, or an
/// axiom the entailment does not need, is reported by whichever of the two is `false`.
/// There is deliberately no trailing line combining them. `!(sufficient && minimal)`
/// is a function of the two lines immediately above it, so rendering it would restate
/// bits a reader already has under a name that reads like independent evidence — the
/// same reason `explain_conclusion` renders `checked` and nothing after it.
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
/// assert!(why.certificate().ends_with("minimal true\n"));
/// ```
pub fn justify_to_string(document: &str, axiom: &str) -> Result<ReasoningAnswer, String> {
    ReasonerSession::open(document, 0, 0)?.justify(axiom)
}

/// The certificate of [`ReasonerSession::extract_module`].
///
/// `conservative` is this certificate's honesty gate: an extraction is conservative when
/// the method kept every axiom whose removal it could not prove safe, and each such axiom
/// is named by its own `conservative-keep` line rather than only counted.
fn render_module_certificate(extraction: &ModuleExtraction) -> String {
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
    certificate
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
/// backward confirmed | abstained | skipped
/// derived-subject <surface>
/// derived-predicate <surface>
/// derived-object <surface>
/// digest <64 lowercase hex>
/// proof-term-bytes <n>
/// checked true | checked false
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
    ReasonerSession::open(document, 0, 0)?.explain_conclusion(regime, conclusion)
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
    // The independent backward re-derivation's verdict, so a corroborated conclusion is
    // distinguishable from one nothing cross-checked.
    let _ = writeln!(out, "backward {}", proof.backward().as_str());

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
    out
}

// ── The conclusion-directed entailment services ─────────────────────────────

/// The `mechanism` line every conclusion-directed answer opens its provenance with.
///
/// Rendered on the ANSWER as well as inside the certificate's report, and that repetition
/// is deliberate rather than redundant: the answer is what a caller branches on, and a
/// caller that must act only on a conclusion the normative rule table reached needs the
/// mechanism without parsing a certificate. It is the mechanism's own `as_str` spelling in
/// both places, never an enum ordinal, so the two cannot drift and neither can be renumbered
/// by a seventh mechanism arriving.
fn render_mechanism(mechanism: EntailmentMechanism) -> String {
    format!("mechanism {}\n", mechanism.as_str())
}

/// Render a [`VarKey`] in the syntax the question wrote it in.
///
/// A projected variable comes back as `?name` and a non-distinguished one as the N-Triples
/// blank node it was — which is what SPARQL says a query blank node is, and what keeps the
/// two distinguishable in a `binding` line without a third column saying which kind it is.
fn emit_var(key: &VarKey) -> String {
    match key {
        VarKey::Projected(name) => format!("?{name}"),
        VarKey::Blank { label, scope } => emit(&TermValue::Blank {
            label: label.clone(),
            scope: *scope,
        }),
    }
}

/// The IRI namespace a query VARIABLE is rewritten into before parsing.
///
/// See [`parse_bgp`] for why the rewrite happens at all and why the namespace is an IRI
/// rather than a blank node. It is extended with `q`s until it occurs nowhere in the
/// caller's own text, so a caller writing `<urn:purrdf-query-variable:purrdfQvar0>`
/// themselves cannot have it read as a projected variable.
///
/// This is NOT a vocabulary term and PurRDF does not mint one: nothing is ever asserted
/// about it, it denotes nothing, it is not written to any output, and [`parse_bgp`] maps
/// every occurrence back to a variable before the pattern leaves this function — so it
/// cannot reach a row, a binding, a warrant or a report. It is a `urn:` rather than an
/// `http:` name for exactly that reason: there is nothing to dereference.
const QUERY_VAR_IRI: &str = "urn:purrdf-query-variable:purrdfQvar";

/// Parse a basic graph pattern written as N-Triples with `?name` in any position.
///
/// # Why the variables are rewritten rather than tokenized
///
/// An RDF term's syntax is the parser's business, and this boundary has a parser: IRIs with
/// escapes, literals whose lexical form holds a `>` or a `?`, language tags, base directions
/// and RDF 1.2 triple terms are all things `purrdf_rdf::parse_dataset` already gets right. A
/// hand-rolled term scanner here would be a second, worse copy of that, and the first thing
/// it would get wrong is the `?` inside an IRI's query string.
///
/// So the ONLY thing scanned here is where a `?` is legal to start a variable: outside an
/// IRI, outside a literal and outside a comment. Each such variable is rewritten to a term
/// in the [`QUERY_VAR_IRI`] namespace, the whole document goes through the real parser, and
/// every such term is mapped back to [`QNode::Var`]. A blank node the caller wrote is left
/// alone and stays what SPARQL says it is — a non-distinguished variable, constrained by the
/// match and not projected.
///
/// # An IRI is the stand-in, because a blank node is not legal in every position
///
/// The stand-in used to be a blank node, and that made the promise above false in exactly
/// one position: RDF forbids a blank node as a PREDICATE, so `?s ?p ?o` — the most ordinary
/// basic graph pattern there is — was refused by the parser with a diagnostic about a
/// construct the caller had not written. An IRI is legal in all three positions, so the
/// stand-in is one, and the promise is now true rather than qualified.
///
/// # A variable INSIDE an RDF 1.2 triple term is THE SAME VARIABLE
///
/// [`QNode`] nests, so a variable inside a triple term is mapped back to [`QNode::Var`]
/// exactly like one at top level and is projected exactly like one. That is not a
/// convenience: one NAME is one VARIABLE wherever the caller wrote it, so
/// `?x <ex:p> <<( ?x <ex:q> <ex:r> )>>` is the join it reads as, and a premise that fails
/// it returns no row. A nested occurrence carried as a term of its own — a blank node, say
/// — would have been a SECOND variable joined to the first by nothing, so the pattern
/// would have matched premises that do not satisfy it. The nesting is in [`QNode`] for
/// that reason and no other.
///
/// # A variable in a literal's DATATYPE is refused, by name
///
/// `"5"^^?d` is the one position the stand-in's own legality opens up: a datatype slot holds
/// an IRI, so a blank-node stand-in was refused there by the parser and an IRI one is not.
/// [`QNode`] has no variable-datatype form and SPARQL admits no variable in a datatype slot
/// within a basic graph pattern, so the honest answer is the caller-visible refusal the
/// blank-node stand-in used to get — with a message naming the position, instead of a
/// parser diagnostic about a construct the caller did not write.
///
/// # The sweep reads the pattern the way the PARSER will read it
///
/// The stand-in namespace is extended with `q`s until it occurs nowhere in the caller's
/// text — and "occurs" has to mean what the PARSER will see, not what the bytes spell. An
/// N-Triples IRIREF admits `UCHAR` escapes, so `<urn:purrdf-query-variable:…>` and
/// `<urn:pur\u0072df-query-variable:…>` are ONE IRI written two ways: the second does not
/// contain the namespace as text, the parser hands back an IRI that does, and a sweep over
/// the raw bytes alone would then read the caller's own IRI back as a variable — one
/// spelling answering a different question from the other. So the sweep runs over the raw
/// text AND over [`uchar_expanded`], and either occurrence extends the namespace.
///
/// That is sound rather than approximate. The lexer's only transformation of an IRIREF
/// body is `\uXXXX`/`\UXXXXXXXX` decoding, and in a document that parses at all every `\`
/// inside an IRIREF begins a valid `UCHAR` — so the expansion visits the same escapes the
/// lexer does and yields a string containing every IRI value the parse can produce. A
/// document where that is not true is one the parser refuses, and a refusal is not an
/// answer. Expanding a `\u` the parser would NOT decode (in a comment, say) can only
/// extend the namespace further, which costs a `q` and no correctness.
///
/// # Errors
///
/// A `?` with no name after it, a variable in a literal's datatype, or a document the
/// N-Triples parser refuses.
fn parse_bgp(text: &str) -> Result<Vec<purrdf_entail::QTriple>, String> {
    // The one namespace the caller's own text does not contain — the IRI a variable is
    // rewritten INTO, so no IRI the caller wrote can be read back as a variable, in either
    // of the two ways N-Triples lets them write it (see the item docs).
    let expanded = uchar_expanded(text);
    let mut iri_prefix = QUERY_VAR_IRI.to_owned();
    while text.contains(&iri_prefix) || expanded.contains(&iri_prefix) {
        iri_prefix.push('q');
    }
    let mut names: Vec<String> = Vec::new();
    let mut rewritten = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            // An IRI runs to the first `>`; N-Triples forbids an unescaped one inside.
            //
            // A SECOND `<` is not an IRI at all — it opens an RDF 1.2 triple term, `<<( s p
            // o )>>`, whose own terms are scanned like any others. Reading it as an IRI
            // swallowed everything up to the first `>` of the term's first IRI, and a `?`
            // caught in that stretch reached the parser unrewritten, which then refused the
            // caller's pattern for a `?` the scanner had hidden from itself.
            '<' => {
                rewritten.push(c);
                if chars.peek() == Some(&'<') {
                    rewritten.push('<');
                    chars.next();
                    continue;
                }
                for inner in chars.by_ref() {
                    rewritten.push(inner);
                    if inner == '>' {
                        break;
                    }
                }
            }
            // A literal runs to the first UNESCAPED `"`.
            '"' => {
                rewritten.push(c);
                let mut escaped = false;
                for inner in chars.by_ref() {
                    rewritten.push(inner);
                    if escaped {
                        escaped = false;
                    } else if inner == '\\' {
                        escaped = true;
                    } else if inner == '"' {
                        break;
                    }
                }
            }
            // A comment runs to the end of the line, and a `?` in one is prose.
            '#' => {
                rewritten.push(c);
                for inner in chars.by_ref() {
                    rewritten.push(inner);
                    if inner == '\n' {
                        break;
                    }
                }
            }
            '?' | '$' => {
                let mut name = String::new();
                while let Some(&next) = chars.peek() {
                    if next.is_ascii_alphanumeric() || next == '_' {
                        name.push(next);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if name.is_empty() {
                    return Err(format!(
                        "a variable marker `{c}` with no name after it; a projected variable is \
                         written `{c}name`"
                    ));
                }
                let index = names
                    .iter()
                    .position(|known| *known == name)
                    .unwrap_or_else(|| {
                        names.push(name.clone());
                        names.len() - 1
                    });
                let _ = write!(rewritten, "<{iri_prefix}{index}>");
            }
            _ => rewritten.push(c),
        }
    }

    let dataset = purrdf_rdf::parse_dataset(rewritten.as_bytes(), INPUT_MEDIA_TYPE, None)
        .map_err(|diagnostic| format!("the basic graph pattern is not N-Triples: {diagnostic}"))?;
    // The variable index a stand-in IRI carries, or `None` for an IRI the caller wrote.
    let slot = |iri: &str| -> Option<usize> {
        iri.strip_prefix(iri_prefix.as_str())
            .and_then(|index| index.parse::<usize>().ok())
            .filter(|index| *index < names.len())
    };
    // One walk for all three positions and every depth below them: `restore_query_vars` is
    // total over the term, so no stand-in can survive into a returned pattern, and a nested
    // occurrence comes back as the SAME variable a top-level one does.
    let node = |term: TermValue| -> Result<purrdf_entail::QNode, String> {
        restore_query_vars(term, &slot, &names)
    };
    let mut bgp = Vec::new();
    // `quads()` yields interned ids in document order, so the pattern's own triple order —
    // and hence the column order `projected_vars` reads off it — is the caller's.
    for quad in dataset.quads() {
        if quad.g.is_some() {
            return Err(
                "a basic graph pattern is matched against the DEFAULT graph, so a pattern that \
                 names a graph is refused rather than having that graph silently dropped"
                    .to_owned(),
            );
        }
        bgp.push(purrdf_entail::QTriple {
            s: node(dataset.term_value(quad.s))?,
            p: node(dataset.term_value(quad.p))?,
            o: node(dataset.term_value(quad.o))?,
        });
    }
    Ok(bgp)
}

/// `term`, as the query node it stands for: every [`QUERY_VAR_IRI`] stand-in inside it back
/// to the variable the caller wrote — or the refusal of the one slot that has no such
/// reading.
///
/// TOTAL over the term's structure, and that is the point rather than a detail: the stand-in
/// is a name this boundary invented to get a variable past a parser, so an occurrence of it
/// that survived into a returned pattern would reach a row, a binding or a warrant as though
/// the caller had written it. This maps the three top-level positions AND everything below
/// them, so there is no position a stand-in can come out of.
///
/// A stand-in below the top level becomes the same [`QNode::Var`] one at the top level does,
/// carried by [`QNode::Triple`]. That is what makes one NAME one VARIABLE: the matcher keys
/// a binding by the name at every depth, so `?x <ex:p> <<( ?x <ex:q> <ex:r> )>>` is the join
/// it reads as. A nested occurrence demoted to a blank node instead — which is what this
/// used to do — was a second variable that the first constrained in no way, so a premise
/// failing the join still produced a row.
///
/// Totality is a claim about [`TermValue`]'s own definition, so here are its slots and what
/// each one does with a stand-in:
///
/// * [`TermValue::Iri`] — an IRI slot, and the only one a stand-in can be PARSED into.
///   Mapped back to the variable it stands for.
/// * [`TermValue::Blank`] — `label` is a bare blank-node label and `scope` a structural
///   ordinal. N-Triples' `BLANK_NODE_LABEL` admits no `:`, so no label can spell an IRI at
///   all, and a blank node the caller wrote is the non-distinguished variable SPARQL says
///   it is. Carried through unchanged.
/// * [`TermValue::Literal`] — four slots, one of which is an IRI. `lexical_form` is opaque
///   text the scanner never rewrites (a `?` inside a quoted literal is data); `language` is
///   a `BCP 47`-shaped tag, which admits no `:` and so cannot spell an IRI; `direction` is
///   an enum with no string at all; and `datatype` IS an IRI, which is why it is refused
///   here rather than walked — see [`parse_bgp`]'s own docs.
/// * [`TermValue::Triple`] — three nested term slots, each of which is one of the above.
///   Recursed into, so the audit holds at every depth. A triple term with a variable
///   anywhere inside it becomes [`QNode::Triple`]; a fully ground one stays a term.
///
/// A graph name is not a slot of a term: [`parse_bgp`] refuses a pattern that names a graph
/// before it reads one, so no fourth position exists for a stand-in to reach.
///
/// `slot` is [`parse_bgp`]'s own reader, so the two cannot disagree about which IRIs are
/// stand-ins, and `names` is its variable table, so a variable comes back under the name the
/// caller actually wrote and a refusal can say which one it means.
///
/// # Errors
///
/// A stand-in in a literal's `datatype`. [`QNode`] has no variable-datatype form, so the
/// alternatives are a refusal or a term of this boundary's own invented namespace reaching
/// the matcher as if the caller had written it.
fn restore_query_vars(
    term: TermValue,
    slot: &impl Fn(&str) -> Option<usize>,
    names: &[String],
) -> Result<purrdf_entail::QNode, String> {
    match term {
        TermValue::Iri(ref iri) => Ok(match slot(iri) {
            Some(index) => purrdf_entail::QNode::Var(names[index].clone()),
            None => purrdf_entail::QNode::Term(term),
        }),
        TermValue::Triple { s, p, o } => {
            let s = restore_query_vars(*s, slot, names)?;
            let p = restore_query_vars(*p, slot, names)?;
            let o = restore_query_vars(*o, slot, names)?;
            // A triple term with no variable in it is a TERM, not a three-node question:
            // the pattern layer reads a ground `QNode::Term` and a ground `QNode::Triple`
            // the same way, and keeping the term shape keeps an RDF 1.2 pattern's own terms
            // the ones the caller wrote.
            Ok(match (s, p, o) {
                (
                    purrdf_entail::QNode::Term(s),
                    purrdf_entail::QNode::Term(p),
                    purrdf_entail::QNode::Term(o),
                ) => purrdf_entail::QNode::Term(TermValue::Triple {
                    s: Box::new(s),
                    p: Box::new(p),
                    o: Box::new(o),
                }),
                (s, p, o) => purrdf_entail::QNode::Triple {
                    s: Box::new(s),
                    p: Box::new(p),
                    o: Box::new(o),
                },
            })
        }
        TermValue::Literal {
            ref lexical_form,
            ref datatype,
            ..
        } => match slot(datatype) {
            Some(index) => Err(format!(
                "a variable is not a datatype IRI: `?{}` stands in the datatype of the literal \
                 \"{lexical_form}\", and a basic graph pattern admits a variable only where a \
                 term can be bound",
                names[index]
            )),
            None => Ok(purrdf_entail::QNode::Term(term)),
        },
        TermValue::Blank { .. } => Ok(purrdf_entail::QNode::Term(term)),
    }
}

/// `text` with every N-Triples `UCHAR` escape — `\uXXXX` and `\UXXXXXXXX` — replaced by the
/// character it denotes.
///
/// The one string [`parse_bgp`]'s namespace sweep needs beside the raw text: an IRIREF's
/// only decoding is this one, so an IRI the parser will hand back containing the stand-in
/// namespace must spell that namespace here, whichever of the two ways the caller wrote it.
/// See [`parse_bgp`] for why sweeping the raw bytes alone is not enough and why expanding an
/// escape the parser would not decode costs nothing.
///
/// A `\` that does not begin a well-formed `UCHAR` is copied through as itself, exactly as
/// the lexer leaves it — one that reaches an IRIREF makes the document unparseable, and one
/// inside a literal is some other escape whose expansion no IRI can be read out of.
fn uchar_expanded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'\\'
            && let Some((width, decoded)) = read_uchar(bytes, at)
        {
            out.push(decoded);
            at += width;
            continue;
        }
        // `at` is a char boundary: every byte consumed above is ASCII, and so is every byte
        // of an escape, so the slice below always starts on one.
        let character = text[at..]
            .chars()
            .next()
            .expect("`at` is a char boundary inside the string");
        out.push(character);
        at += character.len_utf8();
    }
    out
}

/// The `UCHAR` escape starting at byte `at` (the `\`), as `(bytes consumed, character)`.
///
/// The escape is all-ASCII, so the byte count is the character count. `None` for anything
/// that is not a complete, in-range `\uXXXX` / `\UXXXXXXXX` — the same reading the lexer
/// this shadows applies, so the two agree about what an escape IS as well as about what it
/// denotes.
fn read_uchar(bytes: &[u8], at: usize) -> Option<(usize, char)> {
    let width = match *bytes.get(at + 1)? {
        b'u' => 4,
        b'U' => 8,
        _ => return None,
    };
    let mut value: u32 = 0;
    for offset in 0..width {
        // Sixteen times a 28-bit prefix plus a digit, `width` times: `\U`'s eight digits
        // reach `u32::MAX` exactly, so no step can overflow.
        value = value * 16 + char::from(*bytes.get(at + 2 + offset)?).to_digit(16)?;
    }
    Some((2 + width, char::from_u32(value)?))
}

/// The caller's `owl:imports` table, as an ORDERED list of `(ontology-iri, document)` pairs.
///
/// One entry declares that the ontology IRI an `owl:imports` names denotes that document —
/// source text in N-Quads (which accepts N-Triples), the same media type the premise itself is
/// read in, so
/// one parser serves both and a caller holding two documents does not have to hold them in two
/// syntaxes.
///
/// A LIST rather than a map, because a map's iteration order is the host's and this boundary's
/// output is a promise: the same input always produces the same run, and a table whose entries
/// arrive in an order the caller cannot see would make that promise depend on a hash seed.
/// Order is the caller's, and it is preserved.
pub type ImportList<'a> = [(&'a str, &'a str)];

/// The caller's [`ImportList`] as a resolved [`ImportMap`], or the reason it is not one.
///
/// **This library fetches nothing.** An `owl:imports` names an ontology DOCUMENT, and PurRDF has
/// no notion of what an ontology IRI dereferences to — inventing one would make an entailment
/// depend on the network. So the imports closure is caller-supplied configuration, exactly like
/// every other vocabulary this workspace reads, and an IRI the caller did not supply is a
/// caller-visible hard error (`purrdf_entail::EntailError::UnresolvedImport`, rendered by
/// [`render_entail_error`]) rather than a silently truncated premise.
///
/// The list is REQUIRED on every service that takes one, and the empty list is the ordinary
/// "imports nothing" case — not a default standing in for an argument the caller forgot.
///
/// # Errors
///
/// A document that is not N-Quads; an entry with an empty ontology IRI, which no
/// `owl:imports` object can ever equal and which would therefore be configuration that silently
/// never applies; or one ontology IRI declared twice, where keeping either document would be a
/// choice this boundary made on the caller's behalf.
fn build_import_map(imports: &ImportList<'_>) -> Result<ImportMap, String> {
    let mut map = ImportMap::new();
    for (iri, document) in imports {
        if iri.is_empty() {
            return Err(
                "an import entry names the empty ontology IRI, which no owl:imports object can \
                 equal, so the document it supplies could never be resolved"
                    .to_owned(),
            );
        }
        let parsed = purrdf_rdf::parse_dataset(document.as_bytes(), INPUT_MEDIA_TYPE, None)
            .map_err(|diagnostic| {
                format!("the import document for <{iri}> is not N-Quads: {diagnostic}")
            })?;
        if map.insert((*iri).to_owned(), parsed).is_some() {
            return Err(format!(
                "the import list declares <{iri}> twice; keeping either document would be a \
                 choice this boundary made for the caller"
            ));
        }
    }
    Ok(map)
}

/// Parse the CONFIGURATION of a reasoning service — the regime name and the import table —
/// which every service below does BEFORE it looks at a caller document.
///
/// The order is a contract, not an implementation detail. All three services can be handed
/// more than one bad input at once, and a caller who fixes what the first one names has to
/// be able to fix the same thing whichever service they called. Configuration comes first
/// because a run that names no regime this library implements, or an import table it cannot
/// read, has no question to ask yet: which of the caller's DOCUMENTS is also malformed is
/// not yet a meaningful thing to report. `the_three_services_agree_on_error_precedence`
/// drives all three with the same doubly-bad input and holds them to it.
fn configuration(regime: &str, imports: &ImportList<'_>) -> Result<(Regime, ImportMap), String> {
    let parsed = parse_regime(regime)?;
    let map = build_import_map(imports)?;
    Ok((parsed, map))
}

/// Parse a premise document as the N-Quads every service on this boundary takes.
///
/// The premise's diagnostic is rendered bare, unprefixed: it is the document the caller
/// named first and the one a service with only one document has, so a prefix would be
/// noise. A SECOND document — a conclusion, an import — says which one it is, because there
/// the ambiguity is real.
fn parse_premise(document: &str) -> Result<std::sync::Arc<purrdf_core::RdfDataset>, String> {
    purrdf_rdf::parse_dataset(document.as_bytes(), INPUT_MEDIA_TYPE, None)
        .map_err(|diagnostic| diagnostic.to_string())
}

/// The CERTAIN ANSWERS of a basic graph pattern over `document` under `regime`.
///
/// A row is a substitution the knowledge base ENTAILS the pattern under — true in every
/// model, not merely present in one closure — which is what SPARQL's entailment regimes
/// define the answers to a basic graph pattern to be.
///
/// `pattern` is N-Triples with `?name` (or `$name`) in any position — the PREDICATE
/// included, which is a position RDF reserves for an IRI and which this boundary reaches by
/// rewriting each variable to a term drawn from a namespace swept out of the caller's own
/// text, then mapping every occurrence back before the pattern is answered. Nothing of that
/// namespace reaches a row, a binding or a report.
///
/// A blank node in the pattern is a NON-DISTINGUISHED variable: constrained by the match,
/// not projected, and not a column — which is what SPARQL says a query blank node is.
///
/// A variable INSIDE an RDF 1.2 triple term is an ordinary variable: it binds, it is a
/// column, and one NAME is one VARIABLE wherever the caller wrote it — so
/// `?x <ex:p> <<( ?x <ex:q> <ex:r> )>>` is the join it reads as and a premise that fails it
/// yields no row.
///
/// A predicate variable is projected like any other, and under `owl-rl` it also renders a
/// `limit`: it ranges over the whole predicate vocabulary, so it ranges over the schema
/// predicates no head of Tables 4–9 concludes and over the constructs the mechanisms beyond
/// the table decide, neither of which the closure this service enumerates from holds.
///
/// The answer's grammar, in emission order:
///
/// ```text
/// mechanism <name>
/// var <name>              (0..n, in the order the pattern first mentions each)
/// row <term> …            (0..n, one per certain answer, positionally aligned to `var`)
/// limit <reason>          (0..n)
/// ```
///
/// A `limit` line is a reason the row set may not be EXHAUSTIVE. Every row is sound
/// unconditionally — the mechanism that found it is sound — and what needs a precondition is
/// the claim a caller makes about a row that is NOT there. So the absence of `limit` lines
/// is the claim that the row set is complete, and there is deliberately no `complete
/// true|false` line beside them: it would be a boolean function of lines already rendered,
/// which this boundary omits rather than restates.
///
/// # `mechanism` is a MEASUREMENT, and a pattern with no `?var` is an entailment question
///
/// A pattern with something to project reads `mechanism strict-table`, and that is a claim
/// rather than a placeholder: the five mechanisms beyond the rule table are not run for one,
/// each because a projected variable over what it decides would be a different question. That
/// they would have been NEEDED is not silence — it arrives as a `limit` line naming the lane
/// and the construct, so an empty row set never reads as exhaustive when nothing tested it.
///
/// A pattern with NO projected variable is a conclusion graph — every position is a term or a
/// blank node — so it is the question [`graph_entails_to_string`] asks, it is answered by the
/// same fold, and it renders whichever of the seven mechanisms reached it. Such an answer is
/// the relation with no columns: one bare `row` line for a `yes` (the empty substitution) and
/// none for a `no`, which is what SPARQL says an answer with nothing to project is. The two
/// entry points cannot disagree about such a question, because one call answers both.
///
/// The certificate is the run's [`ReasoningReport`], rendered by
/// [`render_reasoning_report`]. There is no answers-without-a-report entry point: an empty
/// row set is the answer a caller is most likely to act on and the one that says least on
/// its own.
///
/// # `imports` — the documents the premise says it is not all of
///
/// An ordered [`ImportList`] of `(ontology-iri, document)` pairs. A premise carrying an
/// `owl:imports` is an ontology stating that its axioms are its own PLUS those of the
/// documents it names, so answering over the premise alone would answer a different question;
/// this is where those documents arrive. The library fetches nothing, so an imported IRI this
/// list does not resolve is refused BY NAME rather than treated as an empty document. The
/// empty list is the ordinary "imports nothing" case, and the argument is required rather than
/// defaulted for the same reason `program` is.
///
/// # Errors
///
/// An unknown regime spelling; a regime this service is not total over (`owl-direct` and
/// `rif` are each defined by an input this signature does not carry); a malformed document,
/// pattern or import document; a duplicate or empty import IRI; a pattern that names a graph;
/// a pattern that writes a variable in a literal's DATATYPE, which is a slot RDF reserves for
/// an IRI and for which a basic graph pattern has no binding to project; an `owl:imports`
/// `imports` does not resolve; an inconsistent premise, whose refusal carries the full report;
/// or an exhausted match budget.
///
/// When several of those hold at once, the CONFIGURATION is reported first — the regime
/// spelling, then the import table — and only then the caller's documents. That precedence
/// is the same on all three services, so a caller who switches between them is told to fix
/// the same thing.
///
/// ```
/// use purrdf_validate::regime::certain_answers_to_string;
///
/// let data = "<http://example.org/Cat> \
///     <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/Animal> .\n\
///     <http://example.org/tom> \
///     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Cat> .\n";
/// let pattern = "<http://example.org/tom> \
///     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?c .\n";
/// let answers = certain_answers_to_string("owl-rl", data, pattern, &[]).expect("answers");
/// assert!(answers.answer().starts_with("mechanism strict-table\nvar c\n"));
/// // `?c` ranges over the ENTAILED types, not the asserted one.
/// assert!(answers.answer().contains("\nrow <http://example.org/Animal>\n"));
/// // Nothing beyond the rule table was needed, so the row set is exhaustive and says so by
/// // rendering no `limit` line at all.
/// assert!(!answers.answer().contains("\nlimit "));
/// // The certificate's own line carries the semantic boundary beside the name.
/// assert!(answers.certificate().contains("\nmechanism strict-table no boundary was crossed:"));
///
/// // The SAME call with nothing to project is an entailment question, and answers as one.
/// let ground = "<http://example.org/tom> \
///     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Animal> .\n";
/// let asked = certain_answers_to_string("owl-rl", data, ground, &[]).expect("answers");
/// assert_eq!(asked.answer(), "mechanism strict-table\nrow\n");
///
/// // A premise that IMPORTS its schema answers from the imports closure, with its
/// // `owl:imports` triple left exactly where the caller put it.
/// let importing = "<http://example.org/o> \
///     <http://www.w3.org/2002/07/owl#imports> <http://example.org/schema> .\n\
///     <http://example.org/tom> \
///     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Cat> .\n";
/// let schema = "<http://example.org/Cat> \
///     <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/Animal> .\n";
/// let closed = certain_answers_to_string(
///     "owl-rl", importing, pattern, &[("http://example.org/schema", schema)],
/// ).expect("answers");
/// assert!(closed.answer().contains("\nrow <http://example.org/Animal>\n"));
/// ```
pub fn certain_answers_to_string(
    regime: &str,
    document: &str,
    pattern: &str,
    imports: &ImportList<'_>,
) -> Result<ReasoningAnswer, String> {
    let (parsed, map) = configuration(regime, imports)?;
    let bgp = parse_bgp(pattern)?;
    let premise = parse_premise(document)?;
    let answers = purrdf_entail::certain_answers(&premise, &bgp, parsed, &map)
        .map_err(|error| render_entail_error(regime, &error))?;
    // The mechanism that ANSWERED, read off the answer set rather than asserted here. A
    // pattern with nothing to project routes through the same fold `graph_entails_to_string`
    // does, so it can be reached by any of the seven — and a hard-coded `strict-table` beside
    // such an answer told a consumer the rule table had decided a question it had not.
    let mut answer = render_mechanism(answers.mechanism());
    for var in answers.vars() {
        let _ = writeln!(answer, "var {var}");
    }
    for row in answers.rows() {
        answer.push_str("row");
        for term in row {
            let _ = write!(answer, " {}", emit(term));
        }
        answer.push('\n');
    }
    for limit in answers.limits() {
        let _ = writeln!(answer, "limit {limit}");
    }
    Ok(ReasoningAnswer {
        answer,
        certificate: render_reasoning_report(answers.report()),
        proof: None,
    })
}

/// Does `premise` entail `conclusion` under `regime`?
///
/// The zero-projected-variable case of [`certain_answers_to_string`]: a conclusion GRAPH is
/// a basic graph pattern with nothing to project, so its answer is a verdict rather than a
/// relation, and the binding is read as the WARRANT for a yes rather than as an answer.
///
/// # Not to be confused with [`entails_to_string`]
///
/// The collision is real and both names are right, so it is stated rather than resolved by
/// renaming. [`entails_to_string`] asks the OWL 2 Direct-Semantics TABLEAU whether an
/// ontology entails one AXIOM of the OWL 2 RDF mapping, and renders a `DlCertificate` whose
/// completeness counts hypertableau rounds. This asks the regime's RULE TABLE whether a
/// premise entails a conclusion GRAPH, and renders a `ReasoningReport` whose completeness is
/// the regime's own rule inventory. Different question, different calculus, different
/// certificate — and the two certificates carry different banners so neither can be parsed
/// as the other.
///
/// The answer's grammar, in emission order:
///
/// ```text
/// mechanism <name>
/// entailment entailed | entailment not-entailed | entailment undecided
/// constituent <name>                (0..n, a `composite` mechanism only)
/// binding <?var | _:label> <term>   (0..n, entailed only)
/// miss <summary>                    (not-entailed only)
/// undecided <reason>                (undecided only)
/// ```
///
/// THREE verdicts, never two. `not-entailed` is a PROOF — the procedure was complete for
/// this premise, so the absence of a mapping is the absence of an entailment — and
/// `undecided` is what an incomplete procedure is entitled to say instead. Collapsing the
/// second into the first would turn a limitation of this library into a false statement
/// about the caller's data.
///
/// A conclusion GRAPH is a conjunction, so it can need one mechanism per half; such an answer
/// reads `mechanism composite` and lists its `constituent` lines in the fixed cost order the
/// service folds them in. It is spelled that way rather than by any one constituent's name,
/// which would tell a consumer that one mechanism sufficed.
///
/// `imports` is [`certain_answers_to_string`]'s, and applies to the PREMISE: the conclusion is
/// a graph to match rather than an ontology to close, so an `owl:imports` in it names nothing
/// this service resolves.
///
/// # Errors
///
/// As [`certain_answers_to_string`].
///
/// ```
/// use purrdf_validate::regime::graph_entails_to_string;
///
/// let premise = "<http://example.org/p> \
///     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
///     <http://www.w3.org/2002/07/owl#TransitiveProperty> .\n\
///     <http://example.org/x> <http://example.org/p> <http://example.org/y> .\n\
///     <http://example.org/y> <http://example.org/p> <http://example.org/z> .\n";
/// let conclusion = "<http://example.org/x> <http://example.org/p> <http://example.org/z> .\n";
/// let decided = graph_entails_to_string("owl-rl", premise, conclusion, &[]).expect("decides");
/// // `prp-trp` derives it, so the rule table itself answers.
/// assert!(decided.answer().starts_with("mechanism strict-table\nentailment entailed\n"));
/// assert!(decided.certificate().contains("\nfired prp-trp "));
/// ```
pub fn graph_entails_to_string(
    regime: &str,
    premise: &str,
    conclusion: &str,
    imports: &ImportList<'_>,
) -> Result<ReasoningAnswer, String> {
    let (parsed, map) = configuration(regime, imports)?;
    let target = purrdf_rdf::parse_dataset(conclusion.as_bytes(), INPUT_MEDIA_TYPE, None)
        .map_err(|diagnostic| format!("the conclusion is not N-Quads: {diagnostic}"))?;
    let parsed_premise = parse_premise(premise)?;
    let certificate = purrdf_entail::entails(&parsed_premise, &target, parsed, &map)
        .map_err(|error| render_entail_error(regime, &error))?;
    Ok(ReasoningAnswer {
        answer: render_entailment_answer(&certificate),
        certificate: render_reasoning_report(certificate.report()),
        proof: None,
    })
}

/// The `mechanism`/`entailment`/evidence block shared by [`graph_entails_to_string`] and
/// [`verify_entailment_to_string`].
fn render_entailment_answer(certificate: &EntailmentCertificate) -> String {
    let mut answer = render_mechanism(certificate.mechanism());
    match certificate.outcome() {
        EntailmentOutcome::Entailed(warrant) => {
            answer.push_str("entailment entailed\n");
            // A COMPOSITE names its constituents, in the fixed cost order the fold tried them.
            // `mechanism composite` alone would be truthful and useless: the one thing a reader
            // of a folded answer needs is WHICH lanes did the work, and deriving it from the
            // `boundary` lines is not something a consumer should have to do.
            if let purrdf_entail::EntailmentWarrant::Composite(composite) = warrant {
                for mechanism in composite.mechanisms() {
                    let _ = writeln!(answer, "constituent {}", mechanism.as_str());
                }
            }
            for (key, term) in warrant.binding() {
                let _ = writeln!(answer, "binding {} {}", emit_var(key), emit(term));
            }
        }
        EntailmentOutcome::NotEntailed(miss) => {
            answer.push_str("entailment not-entailed\n");
            let _ = writeln!(answer, "miss {}", miss.summary());
        }
        EntailmentOutcome::Undecided(reason) => {
            answer.push_str("entailment undecided\n");
            let _ = writeln!(answer, "undecided {reason}");
        }
    }
    answer
}

/// Decide whether `premise` entails `conclusion`, then RE-DECIDE the warrant without running
/// a reasoner.
///
/// [`graph_entails_to_string`] with `purrdf_entail::verify` run over its own answer. The
/// re-check runs no reasoner and re-derives nothing — deliberately, because "the closure
/// follows from the premise" is the chase's claim and [`explain_conclusion_to_string`] is
/// its checker, while "the conclusion follows from the closure" is this one and is finite
/// and purely combinatorial. Folding them would cost what the original call cost and give a
/// caller no independent check at all.
///
/// The answer's grammar is [`graph_entails_to_string`]'s plus two lines:
///
/// ```text
/// warrant present | warrant absent
/// verified true | verified false | verified not-applicable
/// ```
///
/// `warrant absent` / `verified not-applicable` is a `not-entailed` or an `undecided`: there
/// is no evidence to re-decide, and a `false` there would read as a failed check rather than
/// as an absent one. A `verified false` beside `warrant present` is an engine defect, and it
/// is RENDERED rather than raised so a caller re-deciding sees what this boundary saw — the
/// same discipline the chase proof's `checked` line is under.
///
/// `imports` is [`certain_answers_to_string`]'s. The re-check runs against the premise AS
/// WRITTEN rather than against its imports closure, deliberately: a warrant this service can
/// re-decide from the caller's own document is a stronger check than one that could only be
/// re-decided against a graph the library assembled.
///
/// # Errors
///
/// As [`certain_answers_to_string`].
///
/// ```
/// use purrdf_validate::regime::verify_entailment_to_string;
///
/// let premise = "<http://example.org/A> \
///     <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/B> .\n\
///     <http://example.org/x> \
///     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .\n";
/// let conclusion = "<http://example.org/x> \
///     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/B> .\n";
/// let checked =
///     verify_entailment_to_string("owl-rl", premise, conclusion, &[]).expect("decides");
/// assert!(checked.answer().contains("\nwarrant present\n"));
/// assert!(checked.answer().ends_with("verified true\n"));
///
/// // A conclusion nothing derives has no warrant to re-decide, and says so rather than
/// // reporting a check that failed.
/// let never = "<http://example.org/x> \
///     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Never> .\n";
/// let missing = verify_entailment_to_string("owl-rl", premise, never, &[]).expect("decides");
/// assert!(missing.answer().contains("\nwarrant absent\n"));
/// assert!(missing.answer().ends_with("verified not-applicable\n"));
/// ```
pub fn verify_entailment_to_string(
    regime: &str,
    premise: &str,
    conclusion: &str,
    imports: &ImportList<'_>,
) -> Result<ReasoningAnswer, String> {
    let (parsed, map) = configuration(regime, imports)?;
    let target = purrdf_rdf::parse_dataset(conclusion.as_bytes(), INPUT_MEDIA_TYPE, None)
        .map_err(|diagnostic| format!("the conclusion is not N-Quads: {diagnostic}"))?;
    let parsed_premise = parse_premise(premise)?;
    let certificate = purrdf_entail::entails(&parsed_premise, &target, parsed, &map)
        .map_err(|error| render_entail_error(regime, &error))?;
    let mut answer = render_entailment_answer(&certificate);
    match certificate.warrant() {
        Some(warrant) => {
            answer.push_str("warrant present\n");
            let _ = writeln!(
                answer,
                "verified {}",
                purrdf_entail::verify(warrant, &parsed_premise, &target)
            );
        }
        None => {
            answer.push_str("warrant absent\n");
            answer.push_str("verified not-applicable\n");
        }
    }
    Ok(ReasoningAnswer {
        answer,
        certificate: render_reasoning_report(certificate.report()),
        proof: None,
    })
}

// ── Proving, and checking a proof ───────────────────────────────────────────

/// Refuse a non-empty `argument` for a service that takes none.
///
/// Never discarded silently: a caller who passed a class to `classify` believes it narrowed
/// the question, and the answer they would get back is about every named class.
fn refuse_argument(service: &str, argument: &str) -> Result<(), String> {
    if argument.trim().is_empty() {
        return Ok(());
    }
    Err(format!(
        "service \"{service}\" takes no argument, and \"{argument}\" was supplied; the \
         argument would have been discarded rather than narrowing the question"
    ))
}

/// Split `extract-module`'s argument into `(signature, method)`.
///
/// The grammar is one `method <bot|top|star>` line and then one N-Triples term per line —
/// the module extractor's question is a PAIR, and every other service's argument is a single
/// value, so the pair is spelled inside the one string every host already carries rather than
/// by giving one service a second parameter four hosts would have to grow.
fn parse_module_argument(argument: &str) -> Result<(String, String), String> {
    let (first, rest) = argument.split_once('\n').unwrap_or((argument, ""));
    let method = first.trim().strip_prefix("method ").ok_or_else(|| {
        format!(
            "an `extract-module` argument opens with a `method <{}>` line",
            MODULE_METHOD_NAMES.join("|")
        )
    })?;
    Ok((rest.to_owned(), method.trim().to_owned()))
}

/// The services, as a session that RECORDS.
impl ReasonerSession {
    /// Can `class` have an instance in some model?
    ///
    /// The seventh Direct-Semantics service, and the one this boundary had no entry point
    /// for. It is reachable through [`Self::prove`] rather than as a free function of its
    /// own, because what made it worth adding is that [`purrdf_entail::Service`] has seven
    /// members and a proof surface missing one of them is a surface with a hole in it.
    ///
    /// The answer's grammar:
    ///
    /// ```text
    /// class-satisfiability true | false | unknown
    /// term <C>
    /// ```
    ///
    /// `unknown` is never collapsed to `false`, and the class is echoed for the reason
    /// [`entails_to_string`] echoes its axiom.
    ///
    /// # Errors
    ///
    /// A `class` that is not one N-Triples term, a knowledge base that cannot be built, or an
    /// ontology with no model.
    pub fn class_satisfiability(&mut self, class: &str) -> Result<ReasoningAnswer, String> {
        let term = parse_one_term(class)?;
        let (verdict, certificate, proof) = self
            .kb()?
            .class_satisfiability(&term)
            .map_err(|error| service_error("class-satisfiability", &error))?
            .into_certified_parts();
        let mut answer = format!("class-satisfiability {}\n", verdict_name(verdict));
        let _ = writeln!(answer, "term {}", emit(&term));
        Ok(ReasoningAnswer {
            answer,
            certificate: render_dl_certificate("class-satisfiability", &certificate),
            proof: proof.as_ref().map(render_dl_proof),
        })
    }

    /// Answer `service` about `argument`, WITH the proof term of the run that answered.
    ///
    /// One entry point for all seven proof-bearing services rather than seven, because the
    /// thing a caller varies is the QUESTION and the thing they are asking for is the same:
    /// the answer, its certificate, and a proof they can hand to [`check_dl_proof`]. Seven
    /// parallel entry points would have to be grown at four hosts each.
    ///
    /// `argument` is the question's own input, in the grammar the service already uses:
    ///
    /// | `service` | `argument` |
    /// |---|---|
    /// | `consistency`, `classify`, `realize` | empty — a non-empty one is an ERROR |
    /// | `class-satisfiability`, `instances` | ONE N-Triples term |
    /// | `entails` | ONE triple of the OWL 2 RDF mapping |
    /// | `extract-module` | a `method <bot\|top\|star>` line, then one term per line |
    ///
    /// # Errors
    ///
    /// A session that is not recording (open it with [`Self::open_with_proofs`]), an unknown
    /// service spelling, an argument that is wrong for the service, or whatever that service
    /// itself refuses.
    pub fn prove(&mut self, service: &str, argument: &str) -> Result<ReasoningAnswer, String> {
        let asked = parse_proof_service(service)?;
        if !self.proofs {
            return Err(format!(
                "this session records nothing, so \"{service}\" has no proof to hand back. \
                 Recording is opt-in and costs the traces it keeps: open the session with \
                 proofs to ask for one"
            ));
        }
        match asked {
            Service::Consistency => {
                refuse_argument(service, argument)?;
                self.consistency()
            }
            Service::ClassSatisfiability => self.class_satisfiability(argument),
            Service::Classification => {
                refuse_argument(service, argument)?;
                self.classify()
            }
            Service::Realization => {
                refuse_argument(service, argument)?;
                self.realize()
            }
            Service::InstanceRetrieval => self.instances(argument),
            Service::AxiomEntailment => self.entails(argument),
            Service::ModuleExtraction => {
                let (signature, method) = parse_module_argument(argument)?;
                self.extract_module(&signature, &method)
            }
            // `Service` is `#[non_exhaustive]`: a member added upstream is a member this
            // boundary cannot ask about, and saying so beats answering a different question.
            other => Err(format!(
                "service \"{}\" has no entry point at this boundary",
                other.as_str()
            )),
        }
    }
}

/// Answer `service` about `argument` over `document`, WITH the proof term of the run.
///
/// [`ReasonerSession::prove`] over a session opened by [`ReasonerSession::open_with_proofs`].
/// The opt-in lives here rather than as a flag on the existing entry points: every one of
/// them is unchanged, so a caller who does not ask for a proof runs exactly the search they
/// ran before, keeps exactly the traces they kept before (none), and gets exactly the bytes
/// they got before.
///
/// The returned [`ReasoningAnswer`] carries all three strings. [`ReasoningAnswer::answer`]
/// and [`ReasoningAnswer::certificate`] are byte-identical to the same question asked without
/// proofs; [`ReasoningAnswer::proof_document`] is the `purrdf-dl-proof 1` block
/// [`check_dl_proof`] takes.
///
/// `step_cap` and `work_cap` are [`consistency_to_string`]'s.
///
/// # Errors
///
/// A malformed document, an unknown `service`, an `argument` that is wrong for the service,
/// or whatever that service itself refuses.
///
/// ```
/// use purrdf_validate::regime::{check_dl_proof, prove_to_string};
///
/// let data = "<http://example.org/Cat> \
///     <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/Animal> .\n";
/// let proved = prove_to_string(data, "consistency", "", 0, 0).expect("decides");
/// assert_eq!(proved.answer(), "consistency true\n");
/// assert!(proved.proof_document().starts_with("purrdf-dl-proof 1\n"));
/// assert!(proved.proof_document().contains("\navailability recorded\n"));
///
/// // …and a consumer holding the document, the answer and the proof can CHECK it.
/// let checked = check_dl_proof(
///     data,
///     "consistency",
///     "",
///     proved.answer(),
///     proved.certificate(),
///     proved.proof_document(),
/// )
/// .expect("a genuine proof checks");
/// assert!(checked.starts_with("purrdf-dl-proof-check 1\n"));
/// ```
pub fn prove_to_string(
    document: &str,
    service: &str,
    argument: &str,
    step_cap: u32,
    work_cap: u32,
) -> Result<ReasoningAnswer, String> {
    ReasonerSession::open_with_proofs(document, step_cap, work_cap)?.prove(service, argument)
}

/// Read a rendered `purrdf-dl-certificate 1` block back into the value it renders.
///
/// A CONSUMER's reading, for holding a proof's stopping receipt against — see
/// [`DlCertificate::stated`]. It is the certificate the consumer was HANDED beside the
/// answer, which is what makes the receipt check meaningful: the two halves of one answer
/// must agree, and a receipt free to state its own budget could widen the cap it claims to
/// have exhausted until the claim was unfalsifiable.
///
/// # The one reading the rendering does not carry
///
/// [`render_dl_certificate`] emits no cancellation flag — `completeness budget-exhausted`
/// covers both a cap reached and a caller's stop signal, and only [`DlCertificate::stopped`]
/// tells them apart. So a certificate read back from text states `stopped false`, and a proof
/// whose receipt claims a caller cancellation is REFUSED here rather than checked against a
/// flag the text never carried. That is the sound direction — a refusal, not an
/// acceptance — and it costs nothing at this boundary, which has no stop signal to fire: no
/// answer this crate produces can carry such a receipt.
///
/// # Errors
///
/// Text that is not this grammar, a missing or repeated line, a counter that is not a `u64`,
/// or a boundary naming no [`Construct`](purrdf_entail::Construct).
fn parse_dl_certificate(text: &str) -> Result<DlCertificate, String> {
    let mut lines = text.lines();
    if lines.next() != Some(DL_CERTIFICATE_BANNER) {
        return Err(format!(
            "a DL certificate opens with {DL_CERTIFICATE_BANNER:?}, and this one does not"
        ));
    }
    let mut exhausted = None;
    let mut boundaries = Vec::new();
    let mut counters: [Option<u64>; 8] = [None; 8];
    const COUNTERS: [&str; 8] = [
        "steps",
        "budget",
        "work",
        "work-budget",
        "decisions",
        "peak-nodes",
        "disjunctions",
        "peak-depth",
    ];
    for line in lines {
        if let Some(value) = line.strip_prefix("completeness ") {
            exhausted = Some(match value {
                "decided" | "decided-within-boundaries" => false,
                "budget-exhausted" => true,
                other => return Err(format!("unknown DL completeness \"{other}\"")),
            });
        } else if let Some(rest) = line.strip_prefix("boundary ") {
            let name = rest.split_once(' ').map_or(rest, |(name, _)| name);
            let construct = purrdf_entail::Construct::of_name(name)
                .ok_or_else(|| format!("\"{name}\" names no construct this build knows"))?;
            boundaries.push(purrdf_entail::Boundary::of(construct));
        } else if let Some((index, value)) = COUNTERS.iter().enumerate().find_map(|(at, name)| {
            line.strip_prefix(name)
                .and_then(|rest| rest.strip_prefix(' '))
                .map(|value| (at, value))
        }) {
            let parsed = value.parse::<u64>().map_err(|error| {
                format!("the certificate's `{}` line: {error}", COUNTERS[index])
            })?;
            if counters[index].replace(parsed).is_some() {
                return Err(format!(
                    "the certificate states `{}` twice",
                    COUNTERS[index]
                ));
            }
        }
    }
    let exhausted = exhausted.ok_or_else(|| "the certificate states no completeness".to_owned())?;
    let mut read = [0_u64; 8];
    for (at, value) in counters.iter().enumerate() {
        read[at] = value.ok_or_else(|| format!("the certificate states no `{}`", COUNTERS[at]))?;
    }
    Ok(DlCertificate::stated(
        exhausted, false, boundaries, read[0], read[1], read[2], read[3], read[4], read[5],
        read[6], read[7],
    ))
}

/// The two self-delimiting N-Triples terms a `<keyword> <s> <o>` answer line carries.
///
/// Safe to split on the first space after the first term because every term this boundary
/// puts on such a line is a NAME — an `<iri>` or a `_:label`, neither of which may carry an
/// unescaped space — which is the same property that makes the answer grammars readable.
fn two_terms(line: &str, keyword: &str) -> Result<(TermValue, TermValue), String> {
    let rest = line
        .strip_prefix(keyword)
        .and_then(|rest| rest.strip_prefix(' '))
        .ok_or_else(|| format!("a `{keyword}` line was expected, and this is {line:?}"))?;
    let (left, right) = rest
        .split_once(' ')
        .ok_or_else(|| format!("a `{keyword}` line carries two terms, and this is {line:?}"))?;
    Ok((parse_one_term(left)?, parse_one_term(right)?))
}

/// The three-valued verdict an answer's FIRST line reports about `keyword`.
fn answer_verdict(answer: &str, keyword: &str) -> Result<Verdict, String> {
    let first = answer.lines().next().unwrap_or_default();
    match first
        .strip_prefix(keyword)
        .and_then(|r| r.strip_prefix(' '))
    {
        Some("true") => Ok(Verdict::True),
        Some("false") => Ok(Verdict::False),
        Some("unknown") => Ok(Verdict::Unknown),
        _ => Err(format!(
            "a `{keyword}` answer opens `{keyword} true|false|unknown`, and this opens {first:?}"
        )),
    }
}

/// The claims `answer` REPORTS, read back out of the service's own answer grammar.
///
/// The other half of checking a proof, and the half [`ServiceProof::verify`] cannot do:
/// verification establishes that the runs happened and that the claims rest on them, and this
/// establishes that those claims are the ones the answer beside the proof actually states. A
/// genuine proof of some OTHER answer verifies perfectly and is caught here.
///
/// A three-valued `false` or `unknown` reports NO claim, which is the whole reason the DL
/// services answer three-valued: "not established" and "established false" are both the
/// absence of a claim.
fn answer_claims(
    service: Service,
    argument: &str,
    answer: &str,
) -> Result<Vec<ClaimSubject>, String> {
    Ok(match service {
        Service::Consistency => {
            if answer_verdict(answer, "consistency")?.is_true() {
                vec![ClaimSubject::Consistent]
            } else {
                Vec::new()
            }
        }
        Service::ClassSatisfiability => {
            if answer_verdict(answer, "class-satisfiability")?.is_true() {
                vec![ClaimSubject::ClassSatisfiable {
                    class: parse_one_term(argument)?,
                }]
            } else {
                Vec::new()
            }
        }
        Service::Classification => answer
            .lines()
            .filter(|line| line.starts_with("subclass "))
            .map(|line| {
                two_terms(line, "subclass").map(|(sub, sup)| ClaimSubject::Subsumption { sub, sup })
            })
            .collect::<Result<Vec<_>, String>>()?,
        Service::Realization => answer
            .lines()
            .filter(|line| line.starts_with("type "))
            .map(|line| {
                two_terms(line, "type")
                    .map(|(individual, class)| ClaimSubject::Type { individual, class })
            })
            .collect::<Result<Vec<_>, String>>()?,
        Service::InstanceRetrieval => {
            let class = parse_one_term(argument)?;
            answer
                .lines()
                .filter_map(|line| line.strip_prefix("instance "))
                .map(|term| {
                    parse_one_term(term).map(|individual| ClaimSubject::Type {
                        individual,
                        class: class.clone(),
                    })
                })
                .collect::<Result<Vec<_>, String>>()?
        }
        Service::AxiomEntailment => {
            if answer_verdict(answer, "entails")?.is_true() {
                vec![ClaimSubject::Axiom {
                    axiom: Box::new(parse_axiom(argument)?),
                }]
            } else {
                Vec::new()
            }
        }
        Service::ModuleExtraction => {
            // The answer IS the module, as canonical N-Quads, so its claim is that module's
            // own producer-independent identity — recomputed here from the bytes the consumer
            // holds rather than read off anything the producer said about them.
            let module = purrdf_rdf::parse_dataset(answer.as_bytes(), INPUT_MEDIA_TYPE, None)
                .map_err(|diagnostic| format!("the module answer is not N-Quads: {diagnostic}"))?;
            vec![ClaimSubject::Module {
                digest: purrdf_entail::ontology_identity(&module),
            }]
        }
        other => {
            return Err(format!(
                "service \"{}\" has no answer grammar at this boundary",
                other.as_str()
            ));
        }
    })
}

/// The question `service` and `argument` ask, RE-DERIVED from the consumer's own inputs.
///
/// Never read off the proof. The whole value of [`ServiceProof::verify`] is that it refuses a
/// proof whose question is not the one the consumer holds, and taking the question from the
/// proof would make that check compare a value with itself.
///
/// Two of the seven questions range over the ontology's own named terms rather than over an
/// argument, and those are re-derived from the consumer's `reasoner` — their own reverse
/// mapping of their own document — which is exactly the re-derivation the check needs.
fn proof_question(
    reasoner: &Reasoner,
    service: Service,
    argument: &str,
) -> Result<Question, String> {
    Ok(match service {
        Service::Consistency => Question::Consistency,
        Service::ClassSatisfiability => Question::ClassSatisfiability {
            class: parse_one_term(argument)?,
        },
        Service::Classification => reasoner.classification_question(),
        Service::Realization => reasoner.realization_question(),
        Service::InstanceRetrieval => Question::InstanceRetrieval {
            class: parse_one_term(argument)?,
        },
        Service::AxiomEntailment => Question::AxiomEntailment {
            axiom: Box::new(parse_axiom(argument)?),
        },
        Service::ModuleExtraction => {
            let (signature, method) = parse_module_argument(argument)?;
            Question::ModuleExtraction {
                signature: parse_signature(&signature)?,
                method: parse_module_method(&method)?,
            }
        }
        other => {
            return Err(format!(
                "service \"{}\" has no question at this boundary",
                other.as_str()
            ));
        }
    })
}

/// CHECK a rendered proof against the consumer's OWN ontology, question and answer.
///
/// The checker every host exposes, and the reason a proof is worth rendering at all. Nothing
/// about it trusts the producer: the ontology is parsed from `document`, the question is
/// re-derived from `service` and `argument`, the claims are read back out of `answer`'s own
/// grammar, and the checking context is built from a reverse mapping this function performs
/// itself. The proof supplies the runs and nothing else.
///
/// Five things are established, and each is a separate refusal:
///
/// 1. the document is a `purrdf-dl-proof 1` block whose header describes its own body — see
///    [`decode_dl_proof`], which re-renders and compares;
/// 2. the proof BINDS this ontology and this question: an `entails` proof for a different
///    axiom, or any proof for a different document, is refused before a run is read;
/// 3. every claim names a basis whose run exists and answered the way the basis requires, and
///    the stopping receipt — when there is one — is the certificate's, reading for reading;
/// 4. every run that kept a trace is REPLAYED against the consumer's own clause set;
/// 5. the proof's established claims are exactly the ones `answer` reports, so a genuine
///    proof of some other answer cannot travel beside this one.
///
/// `answer` and `certificate` may both be empty, and each empty one is a WEAKER check that
/// says so in the report rather than a check that quietly passed: with no answer the report
/// reads `answer not-checked`, and with no certificate a proof carrying a stopping receipt is
/// refused, because there is nothing for the receipt to be a receipt of.
///
/// The report's grammar:
///
/// ```text
/// purrdf-dl-proof-check 1
/// service <name>
/// availability recorded
/// digest <64 lowercase hex>
/// input <64 lowercase hex>
/// runs <n>
/// replayed <n>
/// claims <n>
/// attested <n>
/// trusted <n>
/// unattested <n>
/// rests-on <a,b,c>
/// answer checked <n> | answer not-checked
/// ```
///
/// There is deliberately no `verified true` line. A verification that FAILED is an `Err`
/// carrying the rejection, so a rendered `true` would be a constant — a disclosure dressed as
/// a gate, which is the thing [`ReasoningAnswer::certificate`] documents this boundary does
/// not do. What the report carries instead are the three counts, and they are load-bearing:
/// `unattested` above zero says some run was accounted for and NOT replayed, and `rests-on`
/// names the producer-shared components the whole check leans on. A proof is never "fully
/// attested" — reading a clause set is the producer's clausifier, and the report says so.
///
/// # Errors
///
/// A proof document that is absent (`availability not-recorded`), malformed, or for another
/// ontology, question or answer; a document that does not parse; an unknown service; a
/// certificate that is not a `purrdf-dl-certificate 1` block; or a run whose recorded trace
/// does not replay.
pub fn check_dl_proof(
    document: &str,
    service: &str,
    argument: &str,
    answer: &str,
    certificate: &str,
    proof: &str,
) -> Result<String, String> {
    let asked = parse_proof_service(service)?;
    let term = decode_dl_proof(proof)?;
    let dataset = purrdf_rdf::parse_dataset(document.as_bytes(), INPUT_MEDIA_TYPE, None)
        .map_err(|diagnostic| diagnostic.to_string())?;
    // The consumer's own reverse mapping, never the producer's: this is the step that rests on
    // the `refutation-encoding` and `reverse-mapping` entries the report names.
    let mut reasoner = Reasoner::with_proofs(&dataset).map_err(|e| format!("reasoner: {e}"))?;
    let question = proof_question(&reasoner, asked, argument)?;
    reasoner.prepare(&question);
    let context = reasoner
        .proof_context()
        .map_err(|error| format!("checking context: {error}"))?;
    let read = if asked == Service::ModuleExtraction {
        // Locality extraction opens no tableau, so it issues no DL certificate at all — its
        // `purrdf-module-extraction 1` block is a different document with no search counters
        // a stopping receipt could be held against. `ServiceProof::verify` takes `None` for
        // exactly this service, and a module proof arriving WITH a receipt is refused there
        // on the proof's own terms rather than admitted here.
        if !certificate.trim().is_empty() && !certificate.starts_with(MODULE_CERTIFICATE_BANNER) {
            return Err(format!(
                "`extract-module` issues a {MODULE_CERTIFICATE_BANNER:?} block and no DL \
                 certificate; this is some other document"
            ));
        }
        None
    } else if certificate.trim().is_empty() {
        None
    } else {
        Some(parse_dl_certificate(certificate)?)
    };
    let replay = term
        .verify(&dataset, &question, read.as_ref(), &context)
        .map_err(|error| format!("the proof does not check: {error}"))?;
    let bound = if answer.trim().is_empty() {
        None
    } else {
        let claims = answer_claims(asked, argument, answer)?;
        term.covers(&claims)
            .map_err(|error| format!("the proof does not cover the answer beside it: {error}"))?;
        Some(claims.len())
    };

    let mut out = String::new();
    out.push_str(DL_PROOF_CHECK_BANNER);
    out.push('\n');
    let _ = writeln!(out, "service {}", proof_service_name(term.service()));
    out.push_str("availability recorded\n");
    let _ = writeln!(out, "digest {}", term.digest_hex());
    let _ = writeln!(out, "input {}", hex(&term.input()));
    let _ = writeln!(out, "runs {}", replay.runs());
    let _ = writeln!(out, "replayed {}", replay.replayed());
    let _ = writeln!(out, "claims {}", replay.claims());
    let checks = replay.checks();
    let _ = writeln!(out, "attested {}", checks.attested());
    let _ = writeln!(out, "trusted {}", checks.trusted());
    let _ = writeln!(out, "unattested {}", checks.unattested());
    let _ = writeln!(
        out,
        "rests-on {}",
        checks
            .rests_on()
            .iter()
            .map(|entry| entry.as_str())
            .collect::<Vec<_>>()
            .join(",")
    );
    match bound {
        Some(count) => {
            let _ = writeln!(out, "answer checked {count}");
        }
        None => out.push_str("answer not-checked\n"),
    }
    Ok(out)
}

// ── The proof golden vector: one artifact, four hosts ───────────────────────

/// The committed golden vector for the proof surface, verbatim.
///
/// Compiled in with `include_str!` for the reason [`REGIME_GOLDEN_VECTORS`] is: the C ABI,
/// WASM and PyO3 crates consume the SAME bytes as the Rust test rather than each growing a
/// fixture that drifts from the other three. The artifact lives at
/// `crates/validate/tests/fixtures/dl-proof.vectors`.
pub const DL_PROOF_GOLDEN_VECTORS: &str = include_str!("../tests/fixtures/dl-proof.vectors");

/// One case of [`DL_PROOF_GOLDEN_VECTORS`]: a document and a question, and the three strings
/// this boundary must produce for them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DlProofVector {
    /// The case name, unique within the artifact.
    name: String,
    /// The service, in [`PROOF_SERVICE_NAMES`] spelling.
    service: String,
    /// The question's argument, in that service's own grammar.
    argument: String,
    /// The input document (N-Quads).
    input: String,
    /// The answer [`prove_to_string`] must return.
    answer: String,
    /// The `purrdf-dl-proof 1` document it must return beside it.
    proof: String,
    /// The `purrdf-dl-proof-check 1` report [`check_dl_proof`] must return for that proof.
    check: String,
}

impl DlProofVector {
    /// The case name, unique within the artifact.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The service, in [`PROOF_SERVICE_NAMES`] spelling.
    #[must_use]
    pub fn service(&self) -> &str {
        &self.service
    }

    /// The question's argument.
    #[must_use]
    pub fn argument(&self) -> &str {
        &self.argument
    }

    /// The input document, as N-Quads.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }

    /// The answer [`prove_to_string`] must return.
    #[must_use]
    pub fn answer(&self) -> &str {
        &self.answer
    }

    /// The proof document [`prove_to_string`] must return.
    #[must_use]
    pub fn proof(&self) -> &str {
        &self.proof
    }

    /// The check report [`check_dl_proof`] must return.
    #[must_use]
    pub fn check(&self) -> &str {
        &self.check
    }
}

/// Parse [`DL_PROOF_GOLDEN_VECTORS`] into its cases.
///
/// The format is [`REGIME_GOLDEN_VECTORS`]'s, with this artifact's own section names: a line
/// starting with `@` is a directive (`@case <name>`, `@service <name>`, `@argument`, `@input`,
/// `@answer`, `@proof`, `@check`, `@end`), every other line belongs to the last body a
/// directive opened, and outside a body only blank lines and `#` comments are allowed. No
/// body line may begin with `@`, and none does: N-Quads terms open with `<`, `_:` or `"`, and
/// every proof, answer and check line opens with a fixed keyword.
///
/// # Errors
///
/// A malformed artifact, with the 1-based line number.
pub fn dl_proof_golden_vectors() -> Result<Vec<DlProofVector>, String> {
    let mut cases: Vec<DlProofVector> = Vec::new();
    let mut open: Option<&'static str> = None;
    let mut name: Option<String> = None;
    let mut service: Option<String> = None;
    let mut bodies: std::collections::BTreeMap<&'static str, String> =
        std::collections::BTreeMap::new();

    for (index, raw) in DL_PROOF_GOLDEN_VECTORS.lines().enumerate() {
        let line = index + 1;
        let Some(directive) = raw.strip_prefix('@') else {
            match open {
                Some(section) => {
                    let body = bodies.entry(section).or_default();
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
            "case" if name.is_none() && !argument.is_empty() => {
                name = Some(argument.to_owned());
            }
            "service" if !argument.is_empty() => service = Some(argument.to_owned()),
            "argument" | "input" | "answer" | "proof" | "check" => {
                let section: &'static str = match keyword {
                    "argument" => "argument",
                    "input" => "input",
                    "answer" => "answer",
                    "proof" => "proof",
                    _ => "check",
                };
                bodies.entry(section).or_default();
                open = Some(section);
            }
            "end" => {
                let missing = |what: &str| format!("line {line}: @end before @{what}");
                cases.push(DlProofVector {
                    name: name.take().ok_or_else(|| missing("case"))?,
                    service: service.take().ok_or_else(|| missing("service"))?,
                    argument: bodies.remove("argument").unwrap_or_default(),
                    input: bodies.remove("input").ok_or_else(|| missing("input"))?,
                    answer: bodies.remove("answer").ok_or_else(|| missing("answer"))?,
                    proof: bodies.remove("proof").ok_or_else(|| missing("proof"))?,
                    check: bodies.remove("check").ok_or_else(|| missing("check"))?,
                });
                bodies.clear();
            }
            other => {
                return Err(format!(
                    "line {line}: unknown or misplaced directive @{other}"
                ));
            }
        }
    }
    if name.is_some() {
        return Err("the artifact ends inside an unclosed case (missing @end)".to_owned());
    }
    Ok(cases)
}

/// Run every case of [`DL_PROOF_GOLDEN_VECTORS`] through [`prove_to_string`] and
/// [`check_dl_proof`], and compare all three outputs byte for byte.
///
/// **This is the cross-host byte-stability assertion for the proof surface.** The Rust test
/// here, the C-ABI crate's test, the WASM crate's test and the PyO3 crate's test all call
/// THIS function over THESE bytes, and the WASM host also exposes it as
/// `entailCheckProofGoldenVectors` so the artifact can be run as real wasm — a different
/// pointer width, a different `usize`, a different allocator — and compared against what the
/// native build produced. A host that diverges fails here in the same words the others do.
///
/// The proof document is the load-bearing comparison: it carries
/// [`ServiceProof::encode`]'s canonical bytes, so a byte difference between hosts is a
/// difference in the PROOF TERM rather than in a rendering of one, and a consumer's pinned
/// digest would have moved under them.
///
/// It also requires the artifact to cover every service in [`PROOF_SERVICE_NAMES`], so a
/// truncated artifact fails loudly instead of passing vacuously.
///
/// # Errors
///
/// A malformed artifact, a case that fails to prove or to check, a byte difference in any of
/// the three outputs, or a service the artifact no longer covers.
pub fn check_dl_proof_golden_vectors() -> Result<(), String> {
    let cases = dl_proof_golden_vectors()?;
    if cases.is_empty() {
        return Err("the DL proof golden vector artifact holds no cases".to_owned());
    }
    for case in &cases {
        let produced = prove_to_string(
            case.input(),
            case.service(),
            case.argument().trim_end(),
            0,
            0,
        )
        .map_err(|error| format!("case \"{}\": {error}", case.name()))?;
        let differs = |what: &str, expected: &str, produced: &str| {
            format!(
                "case \"{}\": {what} mismatch\n--- expected ---\n{expected}--- produced ---\n{produced}",
                case.name()
            )
        };
        if produced.answer() != case.answer() {
            return Err(differs("answer", case.answer(), produced.answer()));
        }
        if produced.proof_document() != case.proof() {
            return Err(differs("proof", case.proof(), produced.proof_document()));
        }
        let checked = check_dl_proof(
            case.input(),
            case.service(),
            case.argument().trim_end(),
            produced.answer(),
            produced.certificate(),
            produced.proof_document(),
        )
        .map_err(|error| format!("case \"{}\": {error}", case.name()))?;
        if checked != case.check() {
            return Err(differs("check", case.check(), &checked));
        }
    }
    for service in PROOF_SERVICE_NAMES {
        if !cases.iter().any(|case| case.service() == service) {
            return Err(format!(
                "the DL proof golden vector artifact no longer covers service \"{service}\""
            ));
        }
    }
    Ok(())
}

/// Check that an answer nobody asked to record is NOT presentable as a verified one.
///
/// The twin of [`check_inconsistent_refusal`], shared for the same reason: the Rust test
/// here, the C-ABI test, the WASM test and the PyO3 test all call it, so the one host that
/// quietly started treating an absent proof as a checked one fails against the same
/// expectation as the other three.
///
/// Three facts, and each fails on its own:
///
/// 1. an answer from a session that records nothing carries [`ABSENT_DL_PROOF`], which SAYS
///    `availability not-recorded` rather than being blank;
/// 2. [`check_dl_proof`] REFUSES that document, and the refusal says nothing was recorded;
/// 3. the same question asked WITH proofs produces a document that says
///    `availability recorded` and does check — so (2) is the absence being refused rather
///    than the checker being broken.
///
/// # Errors
///
/// Any of the three failing.
pub fn check_absent_proof_is_not_verifiable() -> Result<(), String> {
    const DOCUMENT: &str = concat!(
        "<http://example.org/Cat> <http://www.w3.org/2000/01/rdf-schema#subClassOf> ",
        "<http://example.org/Animal> .\n"
    );
    let unrecorded = consistency_to_string(DOCUMENT, 0, 0)?;
    if unrecorded.proof().is_some() {
        return Err("a session that records nothing must carry no proof term".to_owned());
    }
    if unrecorded.proof_document() != ABSENT_DL_PROOF {
        return Err(format!(
            "an unrecorded answer must carry the absent-proof document, and it carries:\n{}",
            unrecorded.proof_document()
        ));
    }
    let refusal = check_dl_proof(
        DOCUMENT,
        "consistency",
        "",
        unrecorded.answer(),
        unrecorded.certificate(),
        unrecorded.proof_document(),
    )
    .err()
    .ok_or_else(|| {
        "checking an absent proof reported a result; an answer nobody recorded must never be \
         presented as a verified one"
            .to_owned()
    })?;
    if !refusal.contains("nothing was recorded") {
        return Err(format!(
            "the refusal must say that nothing was recorded, and it says: {refusal}"
        ));
    }
    let recorded = prove_to_string(DOCUMENT, "consistency", "", 0, 0)?;
    if !recorded
        .proof_document()
        .contains("\navailability recorded\n")
    {
        return Err("a recorded answer must say so".to_owned());
    }
    check_dl_proof(
        DOCUMENT,
        "consistency",
        "",
        recorded.answer(),
        recorded.certificate(),
        recorded.proof_document(),
    )
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    /// The backward re-derivation runs on the PRODUCTION surface and says so.
    ///
    /// `rdf` and `d` carry small, largely schema-specific rule tables, so SLG resolution
    /// reaches its fixpoint over them in microseconds and the cross-check reports
    /// `confirmed` — a conclusion derived twice, forward by the chase and backward by an
    /// engine sharing only the clause program. This drives the same entry point Python's
    /// `Reasoner`, the WASM `Reasoner` and the C ABI all call, so it fails the moment the
    /// resolver stops completing rather than passing quietly.
    #[test]
    fn the_backward_check_confirms_on_the_regimes_it_completes_over() {
        let data = "<http://example.org/x> \
<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .\n";
        let conclusion = data;
        for regime in ["rdf", "d"] {
            let answer = explain_conclusion_to_string(data, regime, conclusion)
                .unwrap_or_else(|e| panic!("{regime} explains an asserted triple: {e}"));
            assert!(
                answer.certificate().contains("\nbackward confirmed\n"),
                "{regime} completes its backward search, so the certificate must report \
                 the corroboration rather than leave it assumed:\n{}",
                answer.certificate()
            );
        }
    }

    /// The regimes whose search is skipped say `skipped`, not `confirmed`.
    ///
    /// The other half of the honesty gate. `rdfs` and `owl-rl` are skipped on COST, not
    /// inability: measured in release, RDFS reaches its fixpoint in seconds and OWL 2 RL is
    /// budget-cut, and both would report `Confirmed`. Neither is affordable per
    /// explanation, so the check does not run — and the certificate must say `skipped`
    /// rather than imply a corroboration that was never attempted.
    #[test]
    fn the_backward_check_reports_skipped_where_it_does_not_run() {
        let data = "<http://example.org/x> \
<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .\n";
        for regime in ["rdfs", "owl-rl"] {
            let answer = explain_conclusion_to_string(data, regime, data)
                .unwrap_or_else(|e| panic!("{regime} explains an asserted triple: {e}"));
            assert!(
                answer.certificate().contains("\nbackward skipped\n"),
                "{regime} must not claim a corroboration it never attempted:\n{}",
                answer.certificate()
            );
        }
    }

    /// A session's answers do not depend on what was asked before them.
    ///
    /// THE LOAD-BEARING CHECK for the session. `Reasoner::instances` and
    /// `Reasoner::entails` take `&mut self`, so a reused knowledge base is mutated by
    /// every question — and the whole point of the session is that the second question
    /// reuses it. If any of that mutation were observable, a service would answer one
    /// thing on a fresh knowledge base and another after a sibling ran, which is a wrong
    /// answer that no single-service test could see.
    ///
    /// Asks each service twice: once with a sibling run before it, once on its own.
    #[test]
    fn a_session_answers_the_same_as_a_fresh_one_whatever_ran_before() {
        let mut session = ReasonerSession::open(SCHEMA, 0, 0).expect("parses");
        // Deliberately interleaved: the two `&mut self` services run BETWEEN the three
        // that only read, so any state they leave behind would show up downstream.
        let consistency = session.consistency().expect("decides");
        let instances = session
            .instances("<http://example.org/C>")
            .expect("decides");
        let classify = session.classify().expect("decides");
        let entails = session
            .entails(
                "<http://example.org/A> \
<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .",
            )
            .expect("decides");
        let realize = session.realize().expect("decides");
        let profile = session.profile();

        // Each free function opens its own session, so these are the fresh answers.
        for (asked_in_sequence, fresh) in [
            (
                &consistency,
                consistency_to_string(SCHEMA, 0, 0).expect("decides"),
            ),
            (
                &instances,
                instances_to_string(SCHEMA, "<http://example.org/C>", 0, 0).expect("decides"),
            ),
            (
                &classify,
                classify_to_string(SCHEMA, 0, 0).expect("decides"),
            ),
            (
                &entails,
                entails_to_string(
                    SCHEMA,
                    "<http://example.org/A> \
<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .",
                    0,
                    0,
                )
                .expect("decides"),
            ),
            (&realize, realize_to_string(SCHEMA, 0, 0).expect("decides")),
            (&profile, profile_to_string(SCHEMA).expect("parses")),
        ] {
            assert_eq!(asked_in_sequence.answer(), fresh.answer());
            // The certificate too: `decisions` and `steps` are measured per call, and a
            // reused knowledge base that carried work forward would report fewer of them.
            assert_eq!(asked_in_sequence.certificate(), fresh.certificate());
        }
    }

    /// The knowledge base is built lazily, and that is a semantic requirement.
    ///
    /// `profile` is documented to answer for ANY parseable document. Building the
    /// knowledge base in `open` would make it inherit every way `Reasoner::new` can fail,
    /// so this asserts a session opens, and profiles, without ever reasoning.
    #[test]
    fn opening_a_session_does_not_build_a_knowledge_base() {
        let session = ReasonerSession::open(SCHEMA, 0, 0).expect("parses");
        assert!(
            session.reasoner.is_none(),
            "open must not reverse-map: `profile` answers for documents whose knowledge \
             base cannot be built, and eager construction would take that away"
        );
        let _ = session.profile();
        assert!(
            session.reasoner.is_none(),
            "`profile` is purely syntactic and must never build a knowledge base"
        );
    }

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
            ("consistency", consistency_to_string(document, 0, 0)),
            ("classify", classify_to_string(document, 0, 0)),
            ("realize", realize_to_string(document, 0, 0)),
            (
                "instances",
                instances_to_string(document, "<http://example.org/C>", 0, 0),
            ),
            ("entails", entails_to_string(document, CHAIN_AXIOM, 0, 0)),
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

    /// The nominal/inverse/counting corner is DECIDED, not bounded.
    ///
    /// Both decision cores now implement the nominal-introduction rule (Horrocks–Sattler `NN`
    /// in the concept-tree reference, Motik–Shearer–Horrocks Table 5 `NI` in the production
    /// hypertableau), so an at-most over an INVERSE role — `owl:InverseFunctionalProperty` is
    /// exactly `≤1 r⁻.⊤` — is decided outright, with no `counting-on-inverse` boundary and no
    /// `decided-within-boundaries` disclosure. The stale blanket boundary is gone; keeping it
    /// would be a false claim of incompleteness against a rule the differential and the vendored
    /// W3C `webont-description-logic-035` now prove is present.
    #[test]
    fn counting_on_inverse_is_decided_by_nominal_introduction() {
        let ifp = "<http://example.org/ssn> \
<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
<http://www.w3.org/2002/07/owl#InverseFunctionalProperty> .\n\
<http://example.org/a> <http://example.org/ssn> <http://example.org/n1> .\n";
        let answer = consistency_to_string(ifp, 0, 0).expect("decides");
        assert!(
            answer.answer().starts_with("consistency true"),
            "{answer:?}"
        );
        assert!(
            !answer.certificate().contains("counting-on-inverse"),
            "the NN/NI corner is now decided, so no boundary is raised: {}",
            answer.certificate()
        );
        assert!(
            !answer.certificate().contains("decided-within-boundaries"),
            "an inverse-functional property is decided outright, not within boundaries: {}",
            answer.certificate()
        );

        // THE SPELLING-INDEPENDENCE CASE. `q owl:inverseOf p` with `⊤ ⊑ ≤1 q.⊤` denotes exactly
        // what `owl:InverseFunctionalProperty p` denotes (the `S=Named(p)`-made-inverse form),
        // and it is likewise decided outright with no boundary.
        let named_inverse = "<http://example.org/q> \
<http://www.w3.org/2002/07/owl#inverseOf> <http://example.org/p> .\n\
<http://www.w3.org/2002/07/owl#Thing> \
<http://www.w3.org/2000/01/rdf-schema#subClassOf> _:c .\n\
_:c <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
<http://www.w3.org/2002/07/owl#Restriction> .\n\
_:c <http://www.w3.org/2002/07/owl#onProperty> <http://example.org/q> .\n\
_:c <http://www.w3.org/2002/07/owl#maxCardinality> \
\"1\"^^<http://www.w3.org/2001/XMLSchema#nonNegativeInteger> .\n\
<http://example.org/a> <http://example.org/p> <http://example.org/b> .\n";
        let answer = consistency_to_string(named_inverse, 0, 0).expect("decides");
        assert!(
            !answer.certificate().contains("counting-on-inverse"),
            "counting a NAMED role owl:inverseOf makes an inverse is decided, not bounded: {}",
            answer.certificate()
        );

        // A counted role with no inverse partner never was the corner, and still is not.
        let counted_only = "<http://www.w3.org/2002/07/owl#Thing> \
<http://www.w3.org/2000/01/rdf-schema#subClassOf> _:c .\n\
_:c <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
<http://www.w3.org/2002/07/owl#Restriction> .\n\
_:c <http://www.w3.org/2002/07/owl#onProperty> <http://example.org/p> .\n\
_:c <http://www.w3.org/2002/07/owl#maxCardinality> \
\"1\"^^<http://www.w3.org/2001/XMLSchema#nonNegativeInteger> .\n\
<http://example.org/a> <http://example.org/p> <http://example.org/b> .\n";
        let answer = consistency_to_string(counted_only, 0, 0).expect("decides");
        assert!(
            !answer.certificate().contains("counting-on-inverse"),
            "counting with no inverse partner anywhere is not the corner: {}",
            answer.certificate()
        );

        let plain = "<http://example.org/a> <http://example.org/p> <http://example.org/b> .\n";
        let answer = consistency_to_string(plain, 0, 0).expect("decides");
        assert!(
            !answer.certificate().contains("counting-on-inverse"),
            "no counting at all, no boundary: {}",
            answer.certificate()
        );
    }

    /// The counting boundaries answer HONESTLY: an unrepresentable cardinality is a
    /// named refusal, an unpayable one is `unknown`, and neither is ever a verdict.
    ///
    /// Both cliffs were live on the Python surface: `owl:maxCardinality` at `u32::MAX`
    /// wrapped the `n + 1` both calculi need — a trivially consistent ontology answered
    /// `false` under a certificate reading `decided` in release builds and PANICKED in
    /// debug builds — and `owl:minCardinality "30"` disappeared into an exponential
    /// clique search that the round cap could not see, because the work was inside one
    /// round. The fixes are a parse-time refusal, a branch-and-bound prune with a work
    /// budget on the clique search, and a witness-minting ceiling; all three degrade to
    /// named errors or `unknown`, never to a wrong answer or a hang.
    #[test]
    fn counting_boundaries_refuse_or_exhaust_but_never_answer_wrongly() {
        let restriction = |kind: &str, n: &str| {
            format!(
                "_:r <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
<http://www.w3.org/2002/07/owl#Restriction> .\n\
_:r <http://www.w3.org/2002/07/owl#onProperty> <http://example.org/r> .\n\
_:r <http://www.w3.org/2002/07/owl#{kind}> \
\"{n}\"^^<http://www.w3.org/2001/XMLSchema#nonNegativeInteger> .\n\
<http://example.org/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> _:r .\n"
            )
        };

        // u32::MAX cannot be incremented in this representation: refused BY NAME at
        // parse, for every counting construct, never decided.
        for kind in ["maxCardinality", "minCardinality", "cardinality"] {
            let error = consistency_to_string(&restriction(kind, "4294967295"), 0, 0)
                .expect_err("an unrepresentable cardinality is a refusal");
            assert!(error.contains("representable"), "{kind}: {error}");
        }
        // One below the bound is representable and trivially satisfiable.
        let answer = consistency_to_string(&restriction("maxCardinality", "4294967294"), 0, 0)
            .expect("representable");
        assert!(
            answer.answer().starts_with("consistency true"),
            "{answer:?}"
        );

        // The clique cliff: n=30 hung for over forty-five seconds before the
        // branch-and-bound prune; it must now decide immediately.
        let answer = consistency_to_string(&restriction("minCardinality", "30"), 0, 0)
            .expect("well inside every budget");
        assert!(
            answer.answer().starts_with("consistency true"),
            "{answer:?}"
        );

        // Past the witness-minting ceiling the decision is UNKNOWN — three-valued
        // honesty, the same shape as every other exhausted budget — not a hang and
        // not a verdict.
        let answer = consistency_to_string(&restriction("minCardinality", "100000"), 0, 0)
            .expect("exhaustion is an answer, not an error");
        assert!(
            answer.answer().starts_with("consistency unknown"),
            "{answer:?}"
        );
    }

    /// EVERY service carries a certificate, and every certificate names its own
    /// service and ends with its own honesty gate.
    ///
    /// The invariant this whole surface exists for: an answer without a statement
    /// of how completely it was decided is the defect, not the missing feature.
    ///
    /// The DL lane's own certificates (`consistency`, `classify`, `realize`,
    /// `instances`, `entails`) have no trailing gate LITERAL to match against — see
    /// [`ReasoningAnswer::certificate`] for why — so for those this test exercises
    /// the derivation itself: `completeness` may read `decided` only when `boundary`
    /// is absent, exactly what [`DlCertificate::completeness`] computes. Matching a
    /// literal here would have caught nothing, because [`render_dl_certificate`]
    /// used to render one that could only ever say `false`.
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
            assert!(certificate.ends_with('\n'), "{service}");
            if certificate.starts_with(DL_CERTIFICATE_BANNER) {
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
                // The gate is the LAST line, so a truncated certificate is visibly
                // truncated rather than silently gate-free.
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

    /// The DL certificate is NOT the chase report, and cannot be parsed as one.
    ///
    /// The two banners differ precisely so a consumer that reached for the wrong
    /// grammar fails at the first line rather than reading `decided` where it
    /// expected `exact`.
    #[test]
    fn the_two_lanes_render_different_certificates() {
        let chase = materialize_to_nquads_string("owl-rl", TAXONOMY, "").expect("owl-rl");
        assert!(chase.report().starts_with(REPORT_FORMAT_BANNER));
        let tableau = consistency_to_string(TAXONOMY, 0, 0).expect("consistency");
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
        let answer = classify_to_string(TAXONOMY, 0, 0)
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
        let answer = realize_to_string(TAXONOMY, 0, 0)
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
        let answer = instances_to_string(TAXONOMY, "<http://example.org/C>", 0, 0)
            .expect("instances")
            .into_parts()
            .0;
        assert_eq!(answer, "instance <http://example.org/x>\n");
        // A class no axiom constrains: a real question with a real, empty answer.
        let unknown = instances_to_string(TAXONOMY, "<http://example.org/Unmentioned>", 0, 0)
            .expect("an unconstrained name is a real name");
        assert_eq!(unknown.answer(), "");
        // An empty answer is still a DECIDED one: no boundary was met deciding it.
        assert!(unknown.certificate().contains("\ncompleteness decided\n"));
        assert!(!unknown.certificate().contains("\nboundary "));
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
            let answer = entails_to_string(TAXONOMY, &statement, 0, 0)
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
        let entailed = entails_to_string(TAXONOMY, CHAIN_AXIOM, 0, 0).expect("decides");
        assert!(entailed.answer().starts_with("entails true\n"));
        let reversed = "<http://example.org/C> \
<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/A> .\n";
        let refuted = entails_to_string(TAXONOMY, reversed, 0, 0).expect("decides");
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
        let starved = entails_to_string(TAXONOMY, CHAIN_AXIOM, 1, 0).expect("decides nothing");
        assert_eq!(starved.answer().lines().next(), Some("entails unknown"));
        assert!(
            starved
                .certificate()
                .contains("\ncompleteness budget-exhausted\n"),
            "{}",
            starved.certificate()
        );
        assert!(starved.certificate().contains("\nbudget 1\n"));
        // No boundary was met either — this is a plain RDFS taxonomy — and
        // `completeness` STILL reads `budget-exhausted` rather than collapsing to
        // `decided`: the exhausted flag takes precedence over an empty boundary list,
        // exactly as `DlCertificate::completeness` derives it.
        assert!(!starved.certificate().contains("\nboundary "));
        // …and the un-narrowed call decides the same question.
        assert!(
            entails_to_string(TAXONOMY, CHAIN_AXIOM, 0, 0)
                .expect("decides")
                .answer()
                .starts_with("entails true\n")
        );
    }

    /// An ontology with no model is REFUSED by every service but the one that
    /// detects it — and that one answers `false`.
    #[test]
    fn an_unsatisfiable_ontology_is_refused_rather_than_answered_vacuously() {
        let detected = consistency_to_string(UNSATISFIABLE, 0, 0).expect("consistency answers");
        assert_eq!(detected.answer(), "consistency false\n");
        for service in ["classify", "realize"] {
            let produced = match service {
                "classify" => classify_to_string(UNSATISFIABLE, 0, 0),
                _ => realize_to_string(UNSATISFIABLE, 0, 0),
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
        assert!(why.certificate().ends_with("minimal true\n"));
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
        // `checked` is the certificate's own last line — see `render_chase_proof_certificate`,
        // which no longer follows it with a redundant `overclaims !checked` restating the
        // same bit under a different name.
        assert!(why.certificate().ends_with("checked true\n"));
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

    /// The existential refusal is per CONCLUSION, not per regime — and an
    /// underivable conclusion is a hard error rather than an empty explanation.
    ///
    /// `rdfs` carries four existential rules beside fourteen Datalog ones, so a
    /// conclusion the Datalog subset derives EXPLAINS (with a proof the checker
    /// re-derives), while the same conclusion under `rdf` — whose three-rule
    /// table cannot reach it — refuses by name: one of the existential rules may
    /// be what derives it there, and "no derivation" would be a false answer.
    #[test]
    fn an_unexplainable_conclusion_is_refused_by_name() {
        let derived = "<http://example.org/x> \
<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C> .\n";
        let explained = explain_conclusion_to_string(TAXONOMY, "rdfs", derived)
            .expect("rdfs9 derives it, and rdfs9 is a Datalog rule");
        assert!(
            explained.certificate().contains("\nchecked true\n"),
            "the returned proof must re-derive"
        );
        let error = explain_conclusion_to_string(TAXONOMY, "rdf", derived)
            .expect_err("rdf's three-rule table cannot derive it");
        assert!(error.contains("existential"), "{error}");

        let absent = "<http://example.org/nobody> \
<http://example.org/nothing> <http://example.org/nowhere> .\n";
        let error =
            explain_conclusion_to_string(TAXONOMY, "owl-rl", absent).expect_err("not derived");
        assert!(error.contains("no derivation"), "{error}");
    }

    // ── The conclusion-directed service, at the string boundary ─────────────

    /// `Boy ⊓ Girl = ⊥`, `Stewie : Boy`, `Peter : Girl` — enough for the profile's own
    /// inconsistency calculus to refute `Stewie = Peter`, and for nothing else to reach it.
    const DISJOINT: &str = "<http://example.org/Boy> \
<http://www.w3.org/2002/07/owl#disjointWith> <http://example.org/Girl> .\n\
<http://example.org/Stewie> \
<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Boy> .\n\
<http://example.org/Peter> \
<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Girl> .\n";

    /// AN ANSWER NOTHING TESTED RENDERS A `limit` LINE, AND SAYS WHICH LANE.
    ///
    /// `?x owl:differentFrom <Peter>` asks which individuals are entailed different from
    /// `Peter`, which needs a refutation per candidate over the whole domain — a question this
    /// service declines. Before the `limit` line it rendered `mechanism strict-table` and
    /// `var x` and NOTHING ELSE, which a consumer reads as "no certain answers, exhaustively"
    /// — about a question no mechanism had tested, and one whose ground form
    /// [`graph_entails_to_string`] proves right below.
    #[test]
    fn a_declined_lane_renders_a_limit_line_naming_itself() {
        let pattern = "?x <http://www.w3.org/2002/07/owl#differentFrom> \
<http://example.org/Peter> .\n";
        let answers = certain_answers_to_string("owl-rl", DISJOINT, pattern, &[]).expect("answers");
        assert!(
            answers
                .answer()
                .starts_with("mechanism strict-table\nvar x\n"),
            "{}",
            answers.answer()
        );
        assert!(
            !answers.answer().contains("\nrow"),
            "this service does not search the domain for a witness: {}",
            answers.answer()
        );
        let limits: Vec<&str> = answers
            .answer()
            .lines()
            .filter_map(|line| line.strip_prefix("limit "))
            .collect();
        assert_eq!(limits.len(), 1, "{}", answers.answer());
        assert!(limits[0].starts_with("the refutation lane "), "{limits:?}");
    }

    /// …AND THE SAME QUESTION WITH NOTHING TO PROJECT AGREES WITH `graph_entails`, LINE
    /// FOR LINE.
    ///
    /// A pattern with no `?var` is a conclusion graph, so the two entry points ask one
    /// question and answer it through one fold: the mechanism is the mechanism that actually
    /// reached it — `refutation`, not `strict-table` — and the verdict is the one bare `row`
    /// line SPARQL says an answer over zero columns is. Before this they contradicted each
    /// other on the byte-identical input: `graph_entails` rendered `entailment entailed` while
    /// `certain_answers` rendered no row at all.
    #[test]
    fn nothing_to_project_answers_as_the_entailment_question_it_is() {
        let ground = "<http://example.org/Stewie> \
<http://www.w3.org/2002/07/owl#differentFrom> <http://example.org/Peter> .\n";
        let answers = certain_answers_to_string("owl-rl", DISJOINT, ground, &[]).expect("answers");
        let decided = graph_entails_to_string("owl-rl", DISJOINT, ground, &[]).expect("decides");
        assert_eq!(answers.answer(), "mechanism refutation\nrow\n");
        assert_eq!(
            decided.answer(),
            "mechanism refutation\nentailment entailed\n"
        );
        // Exhaustive, and it says so by rendering no `limit` line: over zero columns the
        // empty substitution is the only substitution there is, and it is present.
        assert!(!answers.answer().contains("\nlimit "));

        // A ground question the table REFUTES is the empty relation, and exhaustively so.
        let never = "<http://example.org/Stewie> \
<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Girl> .\n";
        let missing = certain_answers_to_string("owl-rl", DISJOINT, never, &[]).expect("answers");
        assert_eq!(missing.answer(), "mechanism strict-table\n");
        let refuted = graph_entails_to_string("owl-rl", DISJOINT, never, &[]).expect("decides");
        assert!(refuted.answer().contains("\nentailment not-entailed\n"));
    }

    // ── `?name` IN ANY POSITION, INCLUDING THE PREDICATE ────────────────────

    /// One triple, which under `simple` is the whole closure — so a row is a function of the
    /// pattern alone and every answer below can be asserted whole.
    const ONE_TRIPLE: &str =
        "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n";

    /// EVERY POSITION COMBINATION, PROJECTED AND ASSERTED WHOLE.
    ///
    /// Falsifiable against what this replaced: a variable in PREDICATE position was rewritten
    /// to a blank node, RDF forbids one there, and the parser refused four of these eight
    /// patterns — `?s ?p ?o` among them — with a diagnostic about the caller's N-Triples that
    /// named a construct the caller had not written.
    #[test]
    fn a_variable_is_projected_from_every_position() {
        for (pattern, expected) in [
            (
                "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n",
                "mechanism strict-table\nrow\n",
            ),
            (
                "?s <http://example.org/p> <http://example.org/o> .\n",
                "mechanism strict-table\nvar s\nrow <http://example.org/s>\n",
            ),
            (
                "<http://example.org/s> ?p <http://example.org/o> .\n",
                "mechanism strict-table\nvar p\nrow <http://example.org/p>\n",
            ),
            (
                "<http://example.org/s> <http://example.org/p> ?o .\n",
                "mechanism strict-table\nvar o\nrow <http://example.org/o>\n",
            ),
            (
                "?s ?p <http://example.org/o> .\n",
                "mechanism strict-table\nvar s\nvar p\n\
                 row <http://example.org/s> <http://example.org/p>\n",
            ),
            (
                "?s <http://example.org/p> ?o .\n",
                "mechanism strict-table\nvar s\nvar o\n\
                 row <http://example.org/s> <http://example.org/o>\n",
            ),
            (
                "<http://example.org/s> ?p ?o .\n",
                "mechanism strict-table\nvar p\nvar o\n\
                 row <http://example.org/p> <http://example.org/o>\n",
            ),
            (
                "?s ?p ?o .\n",
                "mechanism strict-table\nvar s\nvar p\nvar o\n\
                 row <http://example.org/s> <http://example.org/p> <http://example.org/o>\n",
            ),
            // `$name` is the same variable syntax, and it reaches the predicate too.
            (
                "$s $p $o .\n",
                "mechanism strict-table\nvar s\nvar p\nvar o\n\
                 row <http://example.org/s> <http://example.org/p> <http://example.org/o>\n",
            ),
        ] {
            let answers =
                certain_answers_to_string("simple", ONE_TRIPLE, pattern, &[]).expect(pattern);
            assert_eq!(answers.answer(), expected, "{pattern}");
        }
    }

    /// A PREDICATE VARIABLE RANGES OVER THE ENTAILED PREDICATES, NOT ONLY THE ASSERTED ONES.
    ///
    /// `rdfs9` puts `tom rdf:type Animal` in the closure and no triple of the premise states
    /// it, so the binding `?p ↦ rdf:type` for `<tom> ?p <Animal>` exists only because the
    /// chase widened the closure — which the same question under `simple` proves, by having
    /// no answer at all.
    #[test]
    fn a_predicate_variable_binds_over_the_widened_closure() {
        let premise = "<http://example.org/Cat> \
<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/Animal> .\n\
<http://example.org/tom> \
<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Cat> .\n";
        let pattern = "<http://example.org/tom> ?p <http://example.org/Animal> .\n";
        let entailed = certain_answers_to_string("rdfs", premise, pattern, &[]).expect("answers");
        let rows: Vec<&str> = entailed
            .answer()
            .lines()
            .filter(|line| line.starts_with("row "))
            .collect();
        assert_eq!(
            rows,
            ["row <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"],
            "{}",
            entailed.answer()
        );
        assert!(
            entailed
                .answer()
                .starts_with("mechanism strict-table\nvar p\n"),
            "{}",
            entailed.answer()
        );
        let asserted = certain_answers_to_string("simple", premise, pattern, &[]).expect("answers");
        assert_eq!(
            asserted.answer(),
            "mechanism strict-table\nvar p\n",
            "no triple of the premise states it, so the row is the chase's"
        );
    }

    /// A `?` INSIDE AN IRI, A LITERAL OR A COMMENT IS NOT A VARIABLE — IN THE PREDICATE TOO.
    ///
    /// The property the rewrite is arranged around: the ONLY `?` scanned is one outside an
    /// IRI, outside a literal and outside a comment. Each case below writes `?zzz` in one of
    /// those three places, so a scanner that read it would project a `zzz` column — and each
    /// asserts the whole answer, which has none.
    #[test]
    fn a_question_mark_that_is_not_a_variable_stays_where_it_was() {
        // In an IRI's query string, IN PREDICATE POSITION.
        let premise = "<http://example.org/s> <http://example.org/p?zzz=1> \
<http://example.org/o> .\n";
        let answers = certain_answers_to_string(
            "simple",
            premise,
            "<http://example.org/s> <http://example.org/p?zzz=1> ?o .\n",
            &[],
        )
        .expect("answers");
        assert_eq!(
            answers.answer(),
            "mechanism strict-table\nvar o\nrow <http://example.org/o>\n"
        );

        // In a literal's lexical form, beside a PREDICATE variable.
        let quoted = "<http://example.org/s> <http://example.org/p> \"is ?zzz a variable\" .\n";
        let answers = certain_answers_to_string(
            "simple",
            quoted,
            "<http://example.org/s> ?p \"is ?zzz a variable\" .\n",
            &[],
        )
        .expect("answers");
        assert_eq!(
            answers.answer(),
            "mechanism strict-table\nvar p\nrow <http://example.org/p>\n"
        );

        // In a comment, on the line above a PREDICATE variable.
        let answers = certain_answers_to_string(
            "simple",
            ONE_TRIPLE,
            "# is ?zzz a variable? it is prose.\n<http://example.org/s> ?p <http://example.org/o> .\n",
            &[],
        )
        .expect("answers");
        assert_eq!(
            answers.answer(),
            "mechanism strict-table\nvar p\nrow <http://example.org/p>\n"
        );
    }

    /// A VARIABLE INSIDE AN RDF 1.2 TRIPLE TERM IS A VARIABLE, AND A COLUMN.
    ///
    /// [`QNode`](purrdf_entail::QNode) nests, so a variable below the top level is the same
    /// kind of variable one above it is: it binds, it is projected, and it appears in the
    /// caller's own first-occurrence order beside the top-level ones. What is asserted here
    /// is the whole answer, so the stand-in IRI cannot have survived as a ground term and no
    /// position can have been dropped.
    #[test]
    fn a_variable_inside_a_triple_term_is_a_variable_of_the_pattern() {
        let premise = "<http://example.org/q> <http://example.org/r> \
<<( <http://example.org/s> <http://example.org/p> <http://example.org/o> )>> .\n";
        let answers =
            certain_answers_to_string("simple", premise, "?a ?b <<( ?s ?p ?o )>> .\n", &[])
                .expect("answers");
        assert_eq!(
            answers.answer(),
            "mechanism strict-table\nvar a\nvar b\nvar s\nvar p\nvar o\n\
             row <http://example.org/q> <http://example.org/r> <http://example.org/s> \
             <http://example.org/p> <http://example.org/o>\n",
            "every position the caller wrote a `?` in is a column"
        );
    }

    /// ONE NAME IS ONE VARIABLE ACROSS A TRIPLE-TERM BOUNDARY, SO AN UNSATISFIABLE JOIN
    /// RETURNS NO ROW.
    ///
    /// The defect this pins is a SILENT WRONG ANSWER rather than a missing column: a nested
    /// occurrence carried as its own term was a second variable the first constrained in no
    /// way, so `?x <p> <<( ?x … )>>` matched a premise whose quoted subject is somebody else
    /// — an EXTRA row, which no `limit` line can excuse, since a limit says the row set may
    /// not be exhaustive and never that a row in it is not an answer.
    ///
    /// Asserted SEMANTICALLY, over two premises that differ only in whether the join holds:
    /// the pattern is the same one both times, so a run that returns the same answer for
    /// both is not enforcing the join whatever it renders.
    #[test]
    fn a_variable_used_inside_and_outside_a_triple_term_is_one_variable() {
        let pattern = "?x <http://example.org/p> \
<<( ?x <http://example.org/q> <http://example.org/r> )>> .\n";
        // The quoted subject is somebody ELSE, so `?x` cannot be both at once.
        let unsatisfiable = "<http://example.org/a> <http://example.org/p> \
<<( <http://example.org/b> <http://example.org/q> <http://example.org/r> )>> .\n";
        let answers =
            certain_answers_to_string("simple", unsatisfiable, pattern, &[]).expect("answers");
        assert_eq!(
            answers.answer(),
            "mechanism strict-table\nvar x\n",
            "no substitution satisfies both occurrences of `?x`, so there is no certain answer"
        );

        // The same pattern over a premise that DOES satisfy the join.
        let satisfiable = "<http://example.org/a> <http://example.org/p> \
<<( <http://example.org/a> <http://example.org/q> <http://example.org/r> )>> .\n";
        let answers =
            certain_answers_to_string("simple", satisfiable, pattern, &[]).expect("answers");
        assert_eq!(
            answers.answer(),
            "mechanism strict-table\nvar x\nrow <http://example.org/a>\n",
            "the join holds here, so the row is a certain answer"
        );

        // ACROSS TWO TRIPLES of one pattern: nested in the first, top level in the second.
        let cross = "<http://example.org/a> <http://example.org/p> \
<<( <http://example.org/b> <http://example.org/q> <http://example.org/r> )>> .\n\
<http://example.org/c> <http://example.org/k> <http://example.org/v> .\n";
        let answers = certain_answers_to_string(
            "simple",
            cross,
            "<http://example.org/a> <http://example.org/p> \
<<( ?x <http://example.org/q> <http://example.org/r> )>> .\n\
?x <http://example.org/k> <http://example.org/v> .\n",
            &[],
        )
        .expect("answers");
        assert_eq!(
            answers.answer(),
            "mechanism strict-table\nvar x\n",
            "`?x` is <b> in the quoted triple and <c> in the second triple, which is no \
             substitution at all"
        );

        // And a PURELY NESTED name still answers: a variable that occurs only below the top
        // level is an ordinary variable, not a case this refuses.
        let answers = certain_answers_to_string(
            "simple",
            unsatisfiable,
            "<http://example.org/a> <http://example.org/p> \
<<( ?y <http://example.org/q> <http://example.org/r> )>> .\n",
            &[],
        )
        .expect("answers");
        assert_eq!(
            answers.answer(),
            "mechanism strict-table\nvar y\nrow <http://example.org/b>\n"
        );
    }

    /// ONE IRI IS ONE IRI, HOWEVER THE CALLER SPELLED IT — INCLUDING IN THE STAND-IN'S OWN
    /// NAMESPACE.
    ///
    /// [`QUERY_VAR_IRI`] is swept out of the caller's text so that no IRI the caller wrote
    /// can be read back as a variable. N-Triples lets an IRIREF spell any character as a
    /// `UCHAR`, so a sweep over the raw bytes alone missed a namespace one of whose letters
    /// was escaped: the parser reconstructed it and the reverse mapping read the caller's
    /// own IRI as a variable, which is a SILENT WRONG ANSWER — the escaped spelling
    /// answered a different question from the plain one, both with no `limit` line.
    ///
    /// Asserted SEMANTICALLY over four positions and both escape widths: the two spellings
    /// of one IRI must produce ONE answer. A rendered-output grep would not catch the
    /// datatype case at all, because a diagnostic elides a literal's datatype.
    #[test]
    fn one_iri_two_spellings_is_one_answer() {
        // The three spellings of the second `r` of `purrdf`: itself, and both `UCHAR`
        // widths. Each spells the stand-in namespace exactly — as the PARSER reads it, not
        // as the bytes read.
        let spellings = ["r", "\\u0072", "\\U00000072"];
        let stand_in = |letter: &str| format!("urn:pur{letter}df-query-variable:purrdfQvar0");

        // SUBJECT and OBJECT, over a premise whose one triple is `<a> <p> <a>`: read as a
        // variable, the stand-in would join with `?s`/`?o` and produce a row; read as the
        // IRI it is, the premise holds no such term and there is none.
        let reflexive = "<http://example.org/a> <http://example.org/p> <http://example.org/a> .\n";
        for (premise, pattern, expected) in [
            (
                reflexive,
                "?s <http://example.org/p> <{IRI}> .\n",
                "mechanism strict-table\nvar s\n",
            ),
            (
                reflexive,
                "<{IRI}> <http://example.org/p> ?o .\n",
                "mechanism strict-table\nvar o\n",
            ),
            // PREDICATE, over `<a> <p> <p>`: read as a variable the stand-in would bind to
            // `?o`'s own term.
            (
                "<http://example.org/a> <http://example.org/p> <http://example.org/p> .\n",
                "<http://example.org/a> <{IRI}> ?o .\n",
                "mechanism strict-table\nvar o\n",
            ),
            // DATATYPE, over a premise that USES the namespace as a datatype: read as a
            // variable the stand-in is refused by name, so the escaped spelling would refuse
            // a pattern the plain one answers.
            (
                "<http://example.org/a> <http://example.org/p> \
\"5\"^^<urn:purrdf-query-variable:purrdfQvar0> .\n",
                "?s <http://example.org/p> \"5\"^^<{IRI}> .\n",
                "mechanism strict-table\nvar s\nrow <http://example.org/a>\n",
            ),
        ] {
            for spelling in spellings.map(stand_in) {
                let written = pattern.replace("{IRI}", &spelling);
                let answers = certain_answers_to_string("simple", premise, &written, &[])
                    .unwrap_or_else(|refusal| panic!("{written}: {refusal}"));
                assert_eq!(answers.answer(), expected, "{written}");
            }
        }
    }

    /// A `?` WRITTEN AS A `UCHAR` IS STILL NOT A VARIABLE.
    ///
    /// The property the escape-aware sweep must not cost: the scanner reads a `?` only
    /// outside an IRI, outside a literal and outside a comment, and expanding escapes for
    /// the SWEEP must not turn an escaped `?` inside one of those into a variable. Each
    /// case writes the `?` as `?` and asserts the whole answer, which has no column for
    /// it.
    #[test]
    fn an_escaped_question_mark_is_not_a_variable_either() {
        // In an IRI's query string: the premise spells it raw, the pattern escaped, and the
        // two are the same IRI — so a row means the escape was NOT read as a variable.
        let premise = "<http://example.org/s> <http://example.org/p?zzz=1> \
<http://example.org/o> .\n";
        let answers = certain_answers_to_string(
            "simple",
            premise,
            "<http://example.org/s> <http://example.org/p\\u003Fzzz=1> ?o .\n",
            &[],
        )
        .expect("answers");
        assert_eq!(
            answers.answer(),
            "mechanism strict-table\nvar o\nrow <http://example.org/o>\n"
        );

        // In a literal's lexical form.
        let quoted = "<http://example.org/s> <http://example.org/p> \"is ?zzz a variable\" .\n";
        let answers = certain_answers_to_string(
            "simple",
            quoted,
            "<http://example.org/s> ?p \"is \\u003Fzzz a variable\" .\n",
            &[],
        )
        .expect("answers");
        assert_eq!(
            answers.answer(),
            "mechanism strict-table\nvar p\nrow <http://example.org/p>\n"
        );

        // In a comment, which no parser decodes at all.
        let answers = certain_answers_to_string(
            "simple",
            ONE_TRIPLE,
            "# is \\u003Fzzz a variable? it is prose.\n\
<http://example.org/s> ?p <http://example.org/o> .\n",
            &[],
        )
        .expect("answers");
        assert_eq!(
            answers.answer(),
            "mechanism strict-table\nvar p\nrow <http://example.org/p>\n"
        );
    }

    /// THE STAND-IN NAMESPACE REACHES NO ANSWER, NO CERTIFICATE AND NO REFUSAL.
    ///
    /// [`QUERY_VAR_IRI`] is a name this boundary invents to get a variable past a parser that
    /// requires an IRI in predicate position. PurRDF mints no vocabulary, so an occurrence of
    /// it in a row, a binding, a limit or a report would be this library's own scaffolding
    /// rendered to a caller as a term of their data. Every service, over patterns that put a
    /// variable in each position and inside an RDF 1.2 triple term.
    #[test]
    fn the_variable_stand_in_never_reaches_a_caller() {
        let triple_term = "<<( <http://example.org/s> <http://example.org/p> \
<http://example.org/o> )>> <http://example.org/q> <http://example.org/r> .\n";
        for pattern in [
            "?s ?p ?o .\n",
            "<http://example.org/s> ?p ?o .\n",
            "?s ?p <http://example.org/o> .\n",
            "?s ?p ?o .\n?o ?p2 ?s .\n",
            "<<( ?s <http://example.org/p> <http://example.org/o> )>> \
<http://example.org/q> ?r .\n",
            "<<( <http://example.org/s> ?p <http://example.org/o> )>> \
<http://example.org/q> ?r .\n",
        ] {
            for premise in [ONE_TRIPLE, triple_term, DISJOINT] {
                for regime in ["simple", "rdfs", "owl-rl"] {
                    let rendered = match certain_answers_to_string(regime, premise, pattern, &[]) {
                        Ok(answers) => {
                            format!("{}{}", answers.answer(), answers.certificate())
                        }
                        Err(refusal) => refusal,
                    };
                    assert!(
                        !rendered.contains(QUERY_VAR_IRI),
                        "{regime} / {pattern}: {rendered}"
                    );
                    assert!(
                        !rendered.contains("urn:purrdf"),
                        "{regime} / {pattern}: {rendered}"
                    );
                }
            }
        }
    }

    /// A VARIABLE IN A LITERAL'S DATATYPE IS REFUSED, AND THE STAND-IN MATCHES NOTHING THERE.
    ///
    /// The datatype slot is the one position the stand-in's own legality opens up: RDF forbids
    /// a blank node as a datatype, so the old stand-in was refused there by the parser, and an
    /// IRI is legal there, so the new one parses straight in. This asserts the SEMANTICS
    /// rather than the rendering, because
    /// [`show`](purrdf_entail) drops a literal's datatype when a diagnostic prints it — a
    /// stand-in that reached that slot would be invisible to
    /// [`the_variable_stand_in_never_reaches_a_caller`], and would silently answer a question
    /// about this boundary's own namespace instead of the caller's data.
    #[test]
    fn a_variable_in_a_literal_datatype_is_refused_rather_than_matched() {
        // A premise whose second half is reachable ONLY through the stand-in namespace: one
        // probe per slot index a pattern below could mint, so the assertion does not depend
        // on which variable of the pattern is rewritten first.
        let mut premise = String::from(
            "<http://example.org/caller> <http://example.org/p> \"5\"^^<http://example.org/dt> .\n",
        );
        for index in 0..3_usize {
            let _ = writeln!(
                premise,
                "<http://example.org/probe{index}> <http://example.org/p> \
                 \"5\"^^<{QUERY_VAR_IRI}{index}> ."
            );
        }
        // THE CONTROL: the probe half IS reachable by an ordinary pattern, so a leak into the
        // datatype slot would show up as a row — this is what makes the refusals below a
        // statement about matching rather than about parsing.
        let control = certain_answers_to_string(
            "simple",
            &premise,
            "?s <http://example.org/p> \"5\"^^<http://example.org/dt> .\n",
            &[],
        )
        .expect("answers");
        assert_eq!(
            control.answer(),
            "mechanism strict-table\nvar s\nrow <http://example.org/caller>\n",
            "the caller's own datatype matches the caller's own subject and nothing else"
        );
        let probed = certain_answers_to_string(
            "simple",
            &premise,
            &format!("?s <http://example.org/p> \"5\"^^<{QUERY_VAR_IRI}1> .\n"),
            &[],
        )
        .expect("answers");
        assert_eq!(
            probed.answer(),
            "mechanism strict-table\nvar s\nrow <http://example.org/probe1>\n",
            "and a caller who writes the stand-in namespace themselves reaches the probe half, \
             which is the row a leak would fabricate"
        );

        for pattern in [
            "?s <http://example.org/p> \"5\"^^?d .\n",
            "?d <http://example.org/p> \"5\"^^?d .\n",
            "?s <http://example.org/p> \"5\"^^?d .\n?s <http://example.org/p> ?o .\n",
            "?a <http://example.org/q> \
<<( <http://example.org/s> <http://example.org/p> \"5\"^^?d )>> .\n",
        ] {
            for regime in ["simple", "rdfs", "owl-rl"] {
                let refusal = match certain_answers_to_string(regime, &premise, pattern, &[]) {
                    Ok(answers) => panic!(
                        "{regime} / {pattern}: a variable has no datatype-IRI form, so this must \
                         be refused rather than answered: {}",
                        answers.answer()
                    ),
                    Err(refusal) => refusal,
                };
                assert!(
                    refusal.contains("a variable is not a datatype IRI"),
                    "the refusal names the position: {regime} / {pattern}: {refusal}"
                );
                assert!(
                    refusal.contains("`?d`"),
                    "and the variable the caller wrote: {regime} / {pattern}: {refusal}"
                );
                assert!(
                    !refusal.contains("urn:purrdf"),
                    "{regime} / {pattern}: {refusal}"
                );
                assert!(
                    !refusal.contains("probe"),
                    "{regime} / {pattern}: {refusal}"
                );
            }
        }
    }

    /// A CALLER WHO WRITES THE STAND-IN THEMSELVES STILL GETS AN IRI.
    ///
    /// The namespace is swept out of the caller's own text — extended with `q`s until it
    /// occurs nowhere in it — before a single variable is rewritten, so the mapping back is
    /// injective by construction. Without the sweep this pattern would project a THIRD column
    /// out of an IRI the caller wrote as a term.
    #[test]
    fn a_caller_writing_the_stand_in_is_not_read_as_a_variable() {
        let collision = "urn:purrdf-query-variable:purrdfQvar0";
        assert!(
            collision.starts_with(QUERY_VAR_IRI),
            "the collision must be inside the namespace this sweeps"
        );
        let premise = format!(
            "<{collision}> <http://example.org/p> <http://example.org/o> .\n\
             <http://example.org/s> <http://example.org/p> <http://example.org/o> .\n"
        );
        let answers =
            certain_answers_to_string("simple", &premise, &format!("<{collision}> ?p ?o .\n"), &[])
                .expect("answers");
        assert_eq!(
            answers.answer(),
            "mechanism strict-table\nvar p\nvar o\n\
             row <http://example.org/p> <http://example.org/o>\n",
            "the caller's own IRI is a ground term, and only `?p`/`?o` are columns"
        );
    }

    /// AN OPEN PREDICATE IS AN `Undecided` WITH A NAMED REASON, NEVER A SILENT EMPTY ANSWER.
    ///
    /// `p ∘ p ⊑ p` entails `p rdf:type owl:TransitiveProperty` and no rule of Tables 4–9 puts
    /// a schema triple in the closure, so `?s ?p ?o` over this premise misses a certain
    /// answer that `graph_entails_to_string` proves — and says so with a `limit` line naming
    /// the open position rather than rendering an exhaustive-looking row set.
    #[test]
    fn an_open_predicate_renders_a_limit_rather_than_an_exhaustive_answer() {
        let premise = "<http://example.org/p> \
<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
<http://www.w3.org/2002/07/owl#ObjectProperty> .\n\
<http://example.org/p> <http://www.w3.org/2002/07/owl#propertyChainAxiom> _:l1 .\n\
_:l1 <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> <http://example.org/p> .\n\
_:l1 <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> _:l2 .\n\
_:l2 <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> <http://example.org/p> .\n\
_:l2 <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> \
<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil> .\n";
        let transitive = "<http://example.org/p> \
<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
<http://www.w3.org/2002/07/owl#TransitiveProperty> .\n";
        let decided = graph_entails_to_string("owl-rl", premise, transitive, &[]).expect("decides");
        assert!(
            decided
                .answer()
                .starts_with("mechanism freeze\nentailment entailed\n"),
            "{}",
            decided.answer()
        );

        let answers =
            certain_answers_to_string("owl-rl", premise, "?s ?p ?o .\n", &[]).expect("answers");
        assert!(
            !answers.answer().contains("owl#TransitiveProperty"),
            "the closure does not hold it: {}",
            answers.answer()
        );
        let limits: Vec<&str> = answers
            .answer()
            .lines()
            .filter_map(|line| line.strip_prefix("limit "))
            .collect();
        assert_eq!(limits.len(), 1, "{}", answers.answer());
        assert!(
            limits[0].starts_with(
                "the question leaves the predicate open in 1 triple (first: ?s ?p ?o)"
            ),
            "{limits:?}"
        );

        // …and a GROUND predicate over the same premise keeps its exhaustive answer, so the
        // limit is a fact about the open position rather than a blanket disclaimer.
        let closed = certain_answers_to_string(
            "owl-rl",
            premise,
            "?s <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> ?o .\n",
            &[],
        )
        .expect("answers");
        assert!(!closed.answer().contains("\nlimit "), "{}", closed.answer());
    }

    // ── The caller's `owl:imports` table, at the string boundary ────────────

    /// A premise that IMPORTS its schema, with the `owl:imports` triple left where the
    /// caller wrote it.
    const IMPORTING_PREMISE: &str = "<http://example.org/o> \
<http://www.w3.org/2002/07/owl#imports> <http://example.org/schema> .\n\
<http://example.org/socrates> \
<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Man> .\n";

    /// The document `<http://example.org/schema>` denotes — supplied, never fetched.
    const IMPORTED_SCHEMA: &str = "<http://example.org/Man> \
<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/Mortal> .\n";

    /// The conclusion only the imports closure reaches.
    const IMPORTED_CONCLUSION: &str = "<http://example.org/socrates> \
<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Mortal> .\n";

    /// ALL THREE conclusion-directed services take the caller's import table, and answer
    /// from the imports closure over the premise AS WRITTEN.
    ///
    /// Falsifiable against what this replaced: every one of the three hard-coded an empty
    /// map, so a premise carrying any `owl:imports` was a permanent refusal on every host
    /// and the only escape was to hand the library a graph that was not the caller's.
    #[test]
    fn every_conclusion_directed_service_resolves_the_callers_imports() {
        let imports = [("http://example.org/schema", IMPORTED_SCHEMA)];

        let answers =
            certain_answers_to_string("owl-rl", IMPORTING_PREMISE, IMPORTED_CONCLUSION, &imports)
                .expect("answers");
        assert_eq!(answers.answer(), "mechanism strict-table\nrow\n");

        let decided =
            graph_entails_to_string("owl-rl", IMPORTING_PREMISE, IMPORTED_CONCLUSION, &imports)
                .expect("decides");
        assert_eq!(
            decided.answer(),
            "mechanism strict-table\nentailment entailed\n"
        );

        let checked =
            verify_entailment_to_string("owl-rl", IMPORTING_PREMISE, IMPORTED_CONCLUSION, &imports)
                .expect("decides");
        assert!(checked.answer().contains("\nentailment entailed\n"));
        assert!(
            checked
                .answer()
                .ends_with("warrant present\nverified true\n")
        );

        // …and the pattern-shaped question projects out of the same closure.
        let pattern = "<http://example.org/socrates> \
<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?c .\n";
        let projected = certain_answers_to_string("owl-rl", IMPORTING_PREMISE, pattern, &imports)
            .expect("answers");
        assert!(
            projected
                .answer()
                .contains("\nrow <http://example.org/Mortal>\n"),
            "{}",
            projected.answer()
        );
    }

    /// THE EMPTY LIST IS "IMPORTS NOTHING", NOT "IMPORT ANYTHING NAMED".
    ///
    /// The library fetches nothing, so an `owl:imports` the caller did not supply refuses BY
    /// NAME on all three services rather than being reasoned over as though the missing
    /// axioms said nothing.
    #[test]
    fn an_unsupplied_import_still_refuses_by_name() {
        for refusal in [
            certain_answers_to_string("owl-rl", IMPORTING_PREMISE, IMPORTED_CONCLUSION, &[])
                .expect_err("unresolved"),
            graph_entails_to_string("owl-rl", IMPORTING_PREMISE, IMPORTED_CONCLUSION, &[])
                .expect_err("unresolved"),
            verify_entailment_to_string("owl-rl", IMPORTING_PREMISE, IMPORTED_CONCLUSION, &[])
                .expect_err("unresolved"),
            // …and a list that resolves a DIFFERENT ontology is the same absence.
            graph_entails_to_string(
                "owl-rl",
                IMPORTING_PREMISE,
                IMPORTED_CONCLUSION,
                &[("http://example.org/other", IMPORTED_SCHEMA)],
            )
            .expect_err("unresolved"),
        ] {
            assert!(
                refusal.contains("<http://example.org/schema>"),
                "the refusal names the document it was not handed: {refusal}"
            );
        }
    }

    /// An import table that cannot be read is refused BEFORE any reasoning, and says which
    /// entry.
    #[test]
    fn a_malformed_import_table_is_refused_by_entry() {
        let not_nquads = graph_entails_to_string(
            "owl-rl",
            IMPORTING_PREMISE,
            IMPORTED_CONCLUSION,
            &[("http://example.org/schema", "this is not n-quads\n")],
        )
        .expect_err("a document that is not N-Quads");
        assert!(
            not_nquads.contains("the import document for <http://example.org/schema>"),
            "{not_nquads}"
        );

        let twice = graph_entails_to_string(
            "owl-rl",
            IMPORTING_PREMISE,
            IMPORTED_CONCLUSION,
            &[
                ("http://example.org/schema", IMPORTED_SCHEMA),
                ("http://example.org/schema", ""),
            ],
        )
        .expect_err("one ontology IRI declared twice");
        assert!(
            twice.contains("declares <http://example.org/schema> twice"),
            "{twice}"
        );

        let nameless = graph_entails_to_string(
            "owl-rl",
            IMPORTING_PREMISE,
            IMPORTED_CONCLUSION,
            &[("", IMPORTED_SCHEMA)],
        )
        .expect_err("the empty ontology IRI");
        assert!(nameless.contains("empty ontology IRI"), "{nameless}");
    }

    /// The three reasoning services report the SAME error when handed the same bad inputs.
    ///
    /// Not a test of one function's statement order: it is the invariant that the boundary
    /// has ONE error precedence. A caller who moves from `certain_answers_to_string` to
    /// `graph_entails_to_string` to `verify_entailment_to_string` — the same question, asked
    /// three ways — must be told to fix the same thing, and `verify_entailment_to_string`'s
    /// own `# Errors` section claims exactly that by saying "as `certain_answers_to_string`".
    /// Each case below is bad in TWO ways at once, so a service that checked in a different
    /// order would name the other fault and fail here.
    #[test]
    fn the_three_services_agree_on_error_precedence() {
        const BAD_DOCUMENT: &str = "this is not n-quads\n";
        const GOOD_GROUND: &str = "<http://example.org/x> \
<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .\n";
        let good_imports: &ImportList<'_> = &[];
        let bad_imports: &ImportList<'_> = &[("", GOOD_GROUND)];

        // (regime, premise, second document, imports, the fragment every service must name)
        let cases: &[(&str, &str, &str, &ImportList<'_>, &str)] = &[
            // An unknown regime AND an unparseable premise: the regime wins everywhere.
            (
                "not-a-regime",
                BAD_DOCUMENT,
                GOOD_GROUND,
                good_imports,
                "unknown entailment regime \"not-a-regime\"",
            ),
            // An unknown regime AND an unreadable import table: still the regime.
            (
                "not-a-regime",
                GOOD_GROUND,
                GOOD_GROUND,
                bad_imports,
                "unknown entailment regime \"not-a-regime\"",
            ),
            // A known regime, an unreadable import table AND an unparseable premise: the
            // import table wins, because it is configuration and the premise is data.
            (
                "owl-rl",
                BAD_DOCUMENT,
                GOOD_GROUND,
                bad_imports,
                "empty ontology IRI",
            ),
            // An unknown regime AND an unparseable SECOND document (the pattern for one
            // service, the conclusion for the other two): the regime still wins.
            (
                "not-a-regime",
                GOOD_GROUND,
                BAD_DOCUMENT,
                good_imports,
                "unknown entailment regime \"not-a-regime\"",
            ),
        ];

        for (regime, premise, second, imports, expected) in cases {
            let answers = certain_answers_to_string(regime, premise, second, imports)
                .expect_err("doubly-bad input");
            let entails = graph_entails_to_string(regime, premise, second, imports)
                .expect_err("doubly-bad input");
            let verified = verify_entailment_to_string(regime, premise, second, imports)
                .expect_err("doubly-bad input");
            for (service, refusal) in [
                ("certain_answers_to_string", &answers),
                ("graph_entails_to_string", &entails),
                ("verify_entailment_to_string", &verified),
            ] {
                assert!(
                    refusal.contains(expected),
                    "{service} must report {expected:?} for regime {regime:?}, not: {refusal}"
                );
            }
        }
    }

    /// The resolution is TRANSITIVE: a supplied document's own `owl:imports` is followed,
    /// and a stop at depth one would reason over a partial premise.
    #[test]
    fn the_import_resolution_reaches_a_fixpoint() {
        let first = "<http://example.org/schema> \
<http://www.w3.org/2002/07/owl#imports> <http://example.org/upper> .\n\
<http://example.org/Man> \
<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/Human> .\n";
        let upper = "<http://example.org/Human> \
<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/Mortal> .\n";
        let decided = graph_entails_to_string(
            "owl-rl",
            IMPORTING_PREMISE,
            IMPORTED_CONCLUSION,
            &[
                ("http://example.org/schema", first),
                ("http://example.org/upper", upper),
            ],
        )
        .expect("decides");
        assert!(decided.answer().contains("\nentailment entailed\n"));

        // Drop the second hop and the refusal NAMES it, rather than answering from half a
        // premise.
        let refusal = graph_entails_to_string(
            "owl-rl",
            IMPORTING_PREMISE,
            IMPORTED_CONCLUSION,
            &[("http://example.org/schema", first)],
        )
        .expect_err("the second hop is unresolved");
        assert!(refusal.contains("<http://example.org/upper>"), "{refusal}");
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

    /// A triple term nested INSIDE another triple term, with a non-IRI (literal or
    /// blank) predicate, must render structurally — `<<( s p o )>>`, with the real,
    /// offending predicate visible — rather than fabricating an empty IRI `<>` for
    /// the position the owned model cannot hold. This is the diagnostic rendering
    /// path every service line in this file goes through ([`emit`]), so a
    /// malformed term surfacing in ANY answer or certificate line is covered by
    /// this one check on the shared renderer.
    #[test]
    fn a_nested_malformed_predicate_renders_structurally_not_as_a_fabricated_empty_iri() {
        for bad_predicate in [
            TermValue::simple_literal("not a predicate"),
            TermValue::blank("b0"),
        ] {
            let nested = TermValue::Triple {
                s: Box::new(TermValue::iri("http://example.org/s")),
                p: Box::new(bad_predicate.clone()),
                o: Box::new(TermValue::iri("http://example.org/o")),
            };
            // Wrap it two deep: the outer triple term's OWN predicate is a well-formed
            // IRI, so only the recursive check on the NESTED term can catch this.
            let outer = TermValue::Triple {
                s: Box::new(TermValue::iri("http://example.org/subject")),
                p: Box::new(TermValue::iri("http://example.org/wraps")),
                o: Box::new(nested.clone()),
            };
            let rendered = emit(&outer);
            // The malformed nested triple renders structurally, carrying its real
            // subject and object, rather than as an owned model whose bad predicate
            // slot silently became "".
            assert!(
                rendered.contains("<<( <http://example.org/s>"),
                "{rendered}"
            );
            assert!(
                rendered.contains("<http://example.org/o> )>>"),
                "{rendered}"
            );
            // The real, offending predicate is visible in the rendering...
            assert!(rendered.contains(&emit(&bad_predicate)), "{rendered}");
            // ...and no empty IRI was fabricated to stand in for it.
            assert!(!rendered.contains("<>"), "{rendered}");
        }
    }

    /// A malformed term, axiom or document is an error, never a silent empty answer.
    #[test]
    fn malformed_boundary_input_is_an_error() {
        assert!(consistency_to_string("this is not n-quads\n", 0, 0).is_err());
        assert!(profile_to_string("this is not n-quads\n").is_err());
        assert!(instances_to_string(TAXONOMY, "not a term", 0, 0).is_err());
        // Two terms where one was asked for.
        assert!(
            instances_to_string(
                TAXONOMY,
                "<http://example.org/A> <http://example.org/B>",
                0,
                0
            )
            .is_err()
        );
        // Two statements where one axiom was asked for.
        let two = format!(
            "{CHAIN_AXIOM}<http://example.org/D> \
<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .\n"
        );
        assert!(entails_to_string(TAXONOMY, &two, 0, 0).is_err());
        // An axiom is one triple and is not graph-scoped.
        let scoped = "<http://example.org/A> \
<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> \
<http://example.org/g> .\n";
        let error = entails_to_string(TAXONOMY, scoped, 0, 0).expect_err("graph-scoped");
        assert!(error.contains("names a graph"), "{error}");
    }

    // ── A LARGE QUESTION IS A QUESTION, NOT A CRASH ─────────────────────────
    //
    // Three of this boundary's services search depth-first, and each one's depth is a
    // function of how big the caller's input is rather than of how complicated it is. Depth
    // held in CALL frames is not a refusal a caller can catch: the process aborts, nothing
    // unwinds, no `Result` is produced, and a host embedding this library — the CLI, Python,
    // the C ABI, WASM — dies with it. So each is checked here, on the surface those hosts
    // actually wrap, at a size that would have aborted.
    //
    // Every check below runs on a thread built with a 1 MiB stack: that is `wasm32`'s, the
    // SMALLEST target this library ships to and an eighth of a native thread's default. A
    // check that passes here passes on every target. It also means these tests fail LOUDLY
    // — by killing the harness — rather than by an assertion, which is the honest shape for
    // "the process must survive this".

    /// The stack these checks run on: `wasm32`'s, the smallest of any shipped target.
    const SMALLEST_TARGET_STACK: usize = 1 << 20;

    /// Run `question` on a thread with [`SMALLEST_TARGET_STACK`] and return its answer.
    fn on_the_smallest_stack<T: Send + 'static>(
        question: impl FnOnce() -> T + Send + 'static,
    ) -> T {
        std::thread::Builder::new()
            .stack_size(SMALLEST_TARGET_STACK)
            .spawn(question)
            .expect("a thread")
            .join()
            .expect("the question is answered rather than aborting the process")
    }

    /// `count` ground triples sharing a subject and an object, one predicate each.
    ///
    /// Distinct predicates are what make this a DEPTH test rather than a breadth test: the
    /// homomorphism search buckets the closure by predicate, so each bucket holds exactly
    /// one candidate and the match budget is spent one unit per level. The budget is five
    /// million and cannot stand in for a depth bound on this shape.
    fn wide_ground_graph(count: usize) -> String {
        use std::fmt::Write as _;
        let mut document = String::new();
        for index in 0..count {
            writeln!(
                document,
                "<http://example.org/s> <http://example.org/p{index}> <http://example.org/o> ."
            )
            .expect("a `String` is infallible to write to");
        }
        document
    }

    /// A conclusion of 25 000 triples is DECIDED — and decided correctly.
    ///
    /// One level of the homomorphism search is one conclusion triple, so this is a search
    /// 25 000 levels deep. Native call frames held roughly seven thousand of them on an
    /// 8 MiB stack and about eight times fewer on `wasm32`'s 1 MiB, so every one of the
    /// three questions below aborted the process before this check existed.
    #[test]
    fn a_large_conclusion_is_decided_rather_than_overflowing_the_stack() {
        let premise = wide_ground_graph(25_000);

        // Entailed: a graph entails itself, and saying so requires mapping all 25 000.
        let self_entailment = premise.clone();
        let answer = on_the_smallest_stack(move || {
            graph_entails_to_string("simple", &self_entailment.clone(), &self_entailment, &[])
                .map(|decided| decided.answer().to_owned())
        })
        .expect("decides");
        assert_eq!(answer, "mechanism strict-table\nentailment entailed\n");

        // Not entailed: one triple the premise lacks, at the far end of the question, is
        // diagnosed by name rather than lost — so the depth is walked, not truncated.
        let with_a_gap = format!(
            "{premise}<http://example.org/s> <http://example.org/absent> <http://example.org/o> .\n"
        );
        let (lhs, rhs) = (premise.clone(), with_a_gap);
        let answer = on_the_smallest_stack(move || {
            graph_entails_to_string("simple", &lhs, &rhs, &[])
                .map(|decided| decided.answer().to_owned())
        })
        .expect("decides");
        assert_eq!(
            answer,
            "mechanism strict-table\nentailment not-entailed\n\
             miss closure lacks <http://example.org/s> <http://example.org/absent> \
             <http://example.org/o>\n"
        );

        // Existential: ONE blank node has to carry all 25 000 edges, so the binding made at
        // the first level must still hold at the last. A level-local mapping would answer
        // `entailed` for the wrong reason; the reported binding is what rules that out.
        let existential = premise.replace(
            "<http://example.org/s> <http://example.org/p",
            "_:b <http://example.org/p",
        );
        let answer = on_the_smallest_stack(move || {
            graph_entails_to_string("simple", &premise, &existential, &[])
                .map(|decided| decided.answer().to_owned())
        })
        .expect("decides");
        assert_eq!(
            answer,
            "mechanism strict-table\nentailment entailed\n\
             binding _:b <http://example.org/s>\n"
        );
    }

    /// A disjunctive ABox with 3 000 individuals is DECIDED, not aborted.
    ///
    /// One level of the OWL-Direct hypertableau search is one `⊔`-rule application, so an
    /// ontology stating one disjunction and 3 000 individuals under it is a search 3 000
    /// levels deep. There is nothing pathological about that ontology, and the round cap is
    /// no defence: it bounds derivation ROUNDS, and a level costs one.
    #[test]
    fn a_disjunctive_abox_is_decided_rather_than_overflowing_the_stack() {
        let mut ontology = String::from(
            "<http://example.org/C> <http://www.w3.org/2002/07/owl#unionOf> _:l1 .\n\
_:l1 <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> <http://example.org/A> .\n\
_:l1 <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> _:l2 .\n\
_:l2 <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> <http://example.org/B> .\n\
_:l2 <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> \
<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil> .\n",
        );
        ontology.extend((0..3_000).map(|index| {
            format!(
                "<http://example.org/x{index}> \
<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C> .\n"
            )
        }));
        let answer = on_the_smallest_stack(move || {
            consistency_to_string(&ontology, 0, 0).map(|decided| decided.answer().to_owned())
        })
        .expect("decides");
        // Nothing in the ontology is contradictory: `A ⊔ B` is satisfied by choosing `A`
        // for every individual, so the verdict is `true` and not merely "no abort".
        assert_eq!(answer, "consistency true\n");
    }

    /// A class expression or data range that nests without end is a REFUSAL by name.
    ///
    /// Unlike the two searches above, nesting depth here is also the depth of the `Concept`
    /// (or `DataRange`) tree the parser builds, and that tree's `Drop`, `Clone` and
    /// negation-normalization each walk it recursively in turn. The bound therefore lives
    /// where the tree is built — no over-deep tree is ever constructed — and exceeding it is
    /// an error every host propagates rather than an abort no host survives.
    #[test]
    fn an_endlessly_nested_expression_is_refused_by_name() {
        use std::fmt::Write as _;
        // A chain of `owl:complementOf`, one level per triple.
        let mut chain = String::new();
        for index in 0..2_000 {
            writeln!(
                chain,
                "_:c{index} <http://www.w3.org/2002/07/owl#complementOf> _:c{} .",
                index + 1
            )
            .expect("a `String` is infallible to write to");
        }
        chain.push_str(
            "<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> _:c0 .\n",
        );
        let refused = on_the_smallest_stack(move || consistency_to_string(&chain, 0, 0))
            .expect_err("a 2 000-deep class expression is past the ceiling");
        assert!(refused.contains("nests deeper than 256"), "{refused}");

        // A CYCLIC data range is four lines long and used to abort the process outright:
        // the data-range decoder had no cycle guard at all.
        let cycle = "_:a <http://www.w3.org/2002/07/owl#datatypeComplementOf> _:b .\n\
_:b <http://www.w3.org/2002/07/owl#datatypeComplementOf> _:a .\n\
<http://example.org/p> <http://www.w3.org/2000/01/rdf-schema#range> _:a .\n\
<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C> .\n";
        let refused = on_the_smallest_stack(move || consistency_to_string(cycle, 0, 0))
            .expect_err("a data range cannot be its own complement");
        assert!(refused.contains("cyclic OWL data range"), "{refused}");
    }
}

#[cfg(test)]
mod proof_tests {
    use super::*;

    /// `Cat ⊑ Animal`, `Fish ⊑ Animal`, `tom : Cat` — a real taxonomy and a real realization.
    const TAXONOMY: &str = "\
<http://example.org/Cat> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/Animal> .
<http://example.org/Fish> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/Animal> .
<http://example.org/tom> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Cat> .
";

    /// A different consistent ontology, for the wrong-ontology negative.
    const OTHER: &str = "<http://example.org/a> <http://example.org/p> <http://example.org/c> .\n";

    /// `Cat ⊑ Animal`, asserted and therefore entailed.
    const AXIOM: &str = "<http://example.org/Cat> \
<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/Animal> .\n";

    /// Every proof-bearing service and the argument its question takes.
    fn every_question() -> Vec<(&'static str, &'static str)> {
        vec![
            ("consistency", ""),
            ("class-satisfiability", "<http://example.org/Cat>"),
            ("classify", ""),
            ("realize", ""),
            ("instances", "<http://example.org/Animal>"),
            ("entails", AXIOM),
            ("extract-module", "method bot\n<http://example.org/Cat>"),
        ]
    }

    /// **THE CROSS-HOST ASSERTION, made natively.** Every case of the committed artifact
    /// reproduces byte for byte.
    ///
    /// The wasm half is `entailCheckProofGoldenVectors`, and the C-ABI and PyO3 crates call
    /// this same function over these same bytes, so a host that diverges fails on one case
    /// rather than on four fixtures that quietly stopped agreeing.
    #[test]
    fn the_dl_proof_golden_vector_matches_natively() {
        check_dl_proof_golden_vectors().expect("the DL proof golden vector");
    }

    /// **THE AVAILABILITY ASSERTION, made natively.** An answer nobody asked to record is
    /// never presentable as a verified one.
    #[test]
    fn an_absent_proof_is_never_presented_as_a_verified_one_natively() {
        check_absent_proof_is_not_verifiable().expect("the absent-proof refusal");
    }

    /// EVERY proof-bearing service produces a proof this boundary can CHECK.
    ///
    /// The headline of the boundary half: seven services, one grammar, and the check runs
    /// against the consumer's own document, question and answer rather than against anything
    /// the producer said about them.
    #[test]
    fn every_proof_bearing_service_produces_a_checkable_proof() {
        let questions = every_question();
        assert_eq!(
            questions.len(),
            PROOF_SERVICE_NAMES.len(),
            "every service has a question here"
        );
        for (service, argument) in questions {
            let proved = prove_to_string(TAXONOMY, service, argument, 0, 0)
                .unwrap_or_else(|error| panic!("{service}: {error}"));
            let document = proved.proof_document();
            assert!(
                document.starts_with(&format!("{DL_PROOF_BANNER}\nservice {service}\n")),
                "{service}: {document}"
            );
            assert!(document.contains("\navailability recorded\n"), "{service}");
            let checked = check_dl_proof(
                TAXONOMY,
                service,
                argument,
                proved.answer(),
                proved.certificate(),
                document,
            )
            .unwrap_or_else(|error| panic!("{service}: {error}"));
            assert!(
                checked.starts_with(&format!("{DL_PROOF_CHECK_BANNER}\nservice {service}\n")),
                "{service}: {checked}"
            );
            assert!(
                checked.contains("\nanswer checked "),
                "{service}: the answer beside the proof is part of what was checked: {checked}"
            );
            // No proof is fully attested: reading a clause set is the PRODUCER's clausifier,
            // and the report names what the check rests on rather than claiming independence.
            let rests = checked
                .lines()
                .find_map(|line| line.strip_prefix("rests-on "))
                .unwrap_or_else(|| panic!("{service}: no rests-on line: {checked}"));
            assert!(!rests.is_empty(), "{service}: {checked}");
        }
    }

    /// **THE MODE-EQUIVALENCE TEST AT THE BOUNDARY.** A proved answer is the unproved answer,
    /// in the rendered answer AND in the rendered certificate.
    ///
    /// Recording is an observation the decision core makes of itself, never a lever it reads.
    /// This lifts `a_proofs_off_service_answer_is_identical_to_a_proofs_on_one` to the string
    /// surface every host sees: the certificate is compared as well as the answer, so a
    /// recording session that had carried extra work forward would move `steps` or `work`
    /// here even where the verdict did not.
    #[test]
    fn a_proved_session_answers_exactly_what_an_unproved_one_answers() {
        for (service, argument) in every_question() {
            let mut plain = ReasonerSession::open(TAXONOMY, 0, 0).expect("parses");
            assert!(!plain.records_proofs());
            let bare = match service {
                "consistency" => plain.consistency(),
                "class-satisfiability" => plain.class_satisfiability(argument),
                "classify" => plain.classify(),
                "realize" => plain.realize(),
                "instances" => plain.instances(argument),
                "entails" => plain.entails(argument),
                _ => {
                    let (signature, method) =
                        parse_module_argument(argument).expect("a method line");
                    plain.extract_module(&signature, &method)
                }
            }
            .unwrap_or_else(|error| panic!("{service}: {error}"));
            let proved = prove_to_string(TAXONOMY, service, argument, 0, 0)
                .unwrap_or_else(|error| panic!("{service}: {error}"));
            assert_eq!(bare.answer(), proved.answer(), "{service} answer");
            assert_eq!(
                bare.certificate(),
                proved.certificate(),
                "{service} certificate"
            );
            assert!(
                bare.proof().is_none(),
                "{service}: nobody asked, so nothing was measured"
            );
            assert!(proved.proof().is_some(), "{service}: somebody asked");
        }
    }

    /// **A REAL ZERO IS NOT AN ABSENCE, AT THE BOUNDARY.** The three availabilities are three
    /// different documents, and no two of them can be read as each other.
    ///
    /// The distinction the whole opt-in design turns on, carried all the way to the strings a
    /// host hands out. `extract-module` is a syntactic fixpoint, so its recorded proof states
    /// `runs 0` — a MEASUREMENT saying there was no search to check — and the check report
    /// says `runs 0` too, having verified exactly that. An unrecorded answer states
    /// `availability not-recorded` and is REFUSED. A host that collapsed the two would be
    /// presenting "never recorded" as "verified, and there was nothing to verify".
    #[test]
    fn a_recorded_zero_run_proof_is_a_different_document_from_an_absent_one() {
        let argument = "method bot\n<http://example.org/Cat>";
        let proved = prove_to_string(TAXONOMY, "extract-module", argument, 0, 0).expect("extracts");
        let document = proved.proof_document();
        assert!(document.contains("\navailability recorded\n"), "{document}");
        assert!(
            document.contains("\nruns 0\n"),
            "locality extraction opens no tableau: {document}"
        );
        let checked = check_dl_proof(
            TAXONOMY,
            "extract-module",
            argument,
            proved.answer(),
            proved.certificate(),
            document,
        )
        .expect("a genuine extraction's proof checks");
        assert!(
            checked.contains("\nruns 0\n") && checked.contains("\nreplayed 0\n"),
            "the REPLAY reports the real zero rather than claiming a search checked out: \
             {checked}"
        );

        // …and the third availability is neither of those, and is refused by name.
        let unrecorded = extract_module_to_string(TAXONOMY, "<http://example.org/Cat>\n", "bot")
            .expect("extracts");
        assert_eq!(unrecorded.proof_document(), ABSENT_DL_PROOF);
        assert_ne!(unrecorded.proof_document(), document);
        let refusal = check_dl_proof(
            TAXONOMY,
            "extract-module",
            argument,
            unrecorded.answer(),
            unrecorded.certificate(),
            unrecorded.proof_document(),
        )
        .expect_err("an absent proof is not a verified one");
        assert!(refusal.contains("nothing was recorded"), "{refusal}");
    }

    /// A session that records nothing refuses to prove, rather than handing back an answer
    /// with an empty proof beside it.
    #[test]
    fn a_session_that_records_nothing_refuses_to_prove() {
        let refusal = ReasonerSession::open(TAXONOMY, 0, 0)
            .expect("parses")
            .prove("consistency", "")
            .expect_err("this session records nothing");
        assert!(refusal.contains("records nothing"), "{refusal}");
    }

    /// A proof presented against ANOTHER ontology is refused.
    #[test]
    fn a_proof_for_another_ontology_is_refused_at_the_boundary() {
        let proved = prove_to_string(TAXONOMY, "consistency", "", 0, 0).expect("decides");
        let refusal = check_dl_proof(
            OTHER,
            "consistency",
            "",
            proved.answer(),
            proved.certificate(),
            proved.proof_document(),
        )
        .expect_err("a proof for another ontology");
        assert!(refusal.contains("does not check"), "{refusal}");
    }

    /// An `entails` proof for one axiom is refused against another — the equivocation the
    /// whole question binding exists to prevent, at the boundary.
    #[test]
    fn an_entails_proof_for_another_axiom_is_refused_at_the_boundary() {
        let proved = prove_to_string(TAXONOMY, "entails", AXIOM, 0, 0).expect("decides");
        let other = "<http://example.org/Fish> \
<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/Animal> .\n";
        let refusal = check_dl_proof(
            TAXONOMY,
            "entails",
            other,
            proved.answer(),
            proved.certificate(),
            proved.proof_document(),
        )
        .expect_err("a proof of a different axiom");
        assert!(refusal.contains("does not check"), "{refusal}");
    }

    /// A proof whose ESTABLISHED CLAIMS are not the answer's is refused, even though every
    /// run inside it is genuine.
    ///
    /// The forgery a two-string answer invites: ship a real proof of a real classification
    /// beside an answer that reports one subsumption more, or one fewer.
    #[test]
    fn a_proof_that_does_not_cover_the_answer_beside_it_is_refused() {
        let proved = prove_to_string(TAXONOMY, "classify", "", 0, 0).expect("classifies");
        let doctored: String = proved
            .answer()
            .lines()
            .filter(|line| !line.starts_with("subclass <http://example.org/Fish>"))
            .fold(String::new(), |mut out, line| {
                out.push_str(line);
                out.push('\n');
                out
            });
        assert_ne!(doctored, proved.answer(), "the fixture drops a subsumption");
        let refusal = check_dl_proof(
            TAXONOMY,
            "classify",
            "",
            &doctored,
            proved.certificate(),
            proved.proof_document(),
        )
        .expect_err("a proof of more than the answer says");
        assert!(refusal.contains("does not cover"), "{refusal}");
    }

    /// A rewritten HEADER line is refused: the header is derived from the body, so a forger
    /// cannot write a friendlier summary above bytes that say otherwise.
    #[test]
    fn a_rewritten_proof_header_is_refused() {
        let proved = prove_to_string(TAXONOMY, "realize", "", 0, 0).expect("realizes");
        let document = proved.proof_document();
        let runs = document
            .lines()
            .find(|line| line.starts_with("runs "))
            .expect("a runs line");
        let forged = document.replace(
            &format!("\n{runs}\n"),
            "\n runs 0\n".trim_start_matches(' '),
        );
        assert_ne!(forged, document, "the fixture rewrites the run count");
        let refusal = check_dl_proof(
            TAXONOMY,
            "realize",
            "",
            proved.answer(),
            proved.certificate(),
            &forged,
        )
        .expect_err("a header that does not describe its own body");
        assert!(refusal.contains("header does not describe"), "{refusal}");
    }

    /// A single edited hex digit in the BODY is refused — by the byte decoder, by the digest
    /// the header states, or by the replay, but never accepted.
    #[test]
    fn a_proof_body_edited_by_one_hex_digit_is_refused() {
        let proved = prove_to_string(TAXONOMY, "entails", AXIOM, 0, 0).expect("decides");
        let document = proved.proof_document();
        let at = document.rfind("\nbody ").expect("a body line") + "\nbody ".len();
        let mut forged: Vec<u8> = document.as_bytes().to_vec();
        forged[at] = if forged[at] == b'0' { b'1' } else { b'0' };
        let forged = String::from_utf8(forged).expect("hex is ASCII");
        assert_ne!(forged, document);
        assert!(
            check_dl_proof(
                TAXONOMY,
                "entails",
                AXIOM,
                proved.answer(),
                proved.certificate(),
                &forged,
            )
            .is_err(),
            "an edited proof body must never check"
        );
    }

    /// A body carrying anything but lowercase hex is refused rather than silently skipped.
    #[test]
    fn a_proof_body_that_is_not_lowercase_hex_is_refused() {
        let proved = prove_to_string(TAXONOMY, "consistency", "", 0, 0).expect("decides");
        for forged in [
            proved.proof_document().replacen("body 1a", "body 1A", 1),
            proved.proof_document().replacen("body 1a", "body 1", 1),
        ] {
            let refusal = check_dl_proof(
                TAXONOMY,
                "consistency",
                "",
                proved.answer(),
                proved.certificate(),
                &forged,
            )
            .expect_err("a body that is not lowercase hex");
            assert!(refusal.contains("hex"), "{refusal}");
        }
    }

    /// A BUDGET-EXHAUSTED proof carries a stopping receipt, checks against the certificate it
    /// arrived beside, and is REFUSED without one.
    ///
    /// The receipt is the one part of a proof whose counters live outside it, so this is the
    /// path where `parse_dl_certificate` is load-bearing: without a certificate there is
    /// nothing for the receipt to be a receipt of, and the check says so rather than passing.
    #[test]
    fn an_undecided_proof_needs_the_certificate_it_arrived_beside() {
        let starved = prove_to_string(TAXONOMY, "consistency", "", 1, 0).expect("decides nothing");
        assert_eq!(starved.answer(), "consistency unknown\n");
        assert!(
            starved
                .certificate()
                .contains("\ncompleteness budget-exhausted\n")
        );
        let document = starved.proof_document();
        assert!(
            document.contains("\nreceipt round-cap\n"),
            "the receipt names the cap that bit: {document}"
        );
        check_dl_proof(
            TAXONOMY,
            "consistency",
            "",
            starved.answer(),
            starved.certificate(),
            document,
        )
        .expect("an undecided proof checks against its own certificate");

        let refusal = check_dl_proof(TAXONOMY, "consistency", "", starved.answer(), "", document)
            .expect_err("a receipt with nothing to be a receipt of");
        assert!(refusal.contains("does not check"), "{refusal}");
    }

    /// A receipt checked against a DIFFERENT run's certificate is refused: the two halves of
    /// one answer must agree, and a widened cap is exactly the forgery the comparison exists
    /// to catch.
    #[test]
    fn a_receipt_checked_against_another_runs_certificate_is_refused() {
        let starved = prove_to_string(TAXONOMY, "consistency", "", 1, 0).expect("decides nothing");
        let decided = prove_to_string(TAXONOMY, "consistency", "", 0, 0).expect("decides");
        let refusal = check_dl_proof(
            TAXONOMY,
            "consistency",
            "",
            starved.answer(),
            decided.certificate(),
            starved.proof_document(),
        )
        .expect_err("a stopping receipt beside a decided certificate");
        assert!(refusal.contains("does not check"), "{refusal}");
    }

    /// A proof is byte-identical run to run, which is what makes the committed artifact a
    /// contract rather than a snapshot.
    #[test]
    fn a_rendered_proof_is_byte_identical_run_to_run() {
        for (service, argument) in every_question() {
            let first = prove_to_string(TAXONOMY, service, argument, 0, 0).expect("proves");
            let again = prove_to_string(TAXONOMY, service, argument, 0, 0).expect("proves");
            assert_eq!(
                first.proof_document(),
                again.proof_document(),
                "{service}: two runs, one proof"
            );
        }
    }

    /// A non-empty argument for a service that takes none is REFUSED rather than discarded.
    #[test]
    fn an_argument_a_service_does_not_take_is_refused() {
        for service in ["consistency", "classify", "realize"] {
            let refusal = prove_to_string(TAXONOMY, service, "<http://example.org/Cat>", 0, 0)
                .expect_err("this service takes no argument");
            assert!(refusal.contains("takes no argument"), "{refusal}");
        }
    }

    /// An unknown service spelling names the accepted set.
    #[test]
    fn an_unknown_proof_service_names_the_accepted_set() {
        let refusal =
            prove_to_string(TAXONOMY, "justify", "", 0, 0).expect_err("not a proof service");
        assert!(
            refusal.contains("consistency, class-satisfiability"),
            "{refusal}"
        );
    }

    /// A document in some OTHER grammar is refused rather than parsed hopefully.
    #[test]
    fn a_document_that_is_not_a_proof_is_refused() {
        for text in [
            "",
            "purrdf-dl-certificate 1\nservice consistency\n",
            "hello\n",
        ] {
            let refusal = check_dl_proof(TAXONOMY, "consistency", "", "", "", text)
                .expect_err("not a proof document");
            assert!(refusal.contains(DL_PROOF_BANNER), "{refusal}");
        }
    }

    /// The two proof grammars carry DIFFERENT banners, so neither can be parsed as the other
    /// — nor as any certificate this boundary already emits.
    #[test]
    fn the_proof_banners_are_all_distinct() {
        let banners = [
            REPORT_FORMAT_BANNER,
            DL_CERTIFICATE_BANNER,
            PROFILE_CERTIFICATE_BANNER,
            MODULE_CERTIFICATE_BANNER,
            JUSTIFICATION_CERTIFICATE_BANNER,
            CHASE_PROOF_CERTIFICATE_BANNER,
            DL_PROOF_BANNER,
            DL_PROOF_CHECK_BANNER,
        ];
        let mut sorted = banners.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), banners.len(), "every banner is distinct");
    }
}
