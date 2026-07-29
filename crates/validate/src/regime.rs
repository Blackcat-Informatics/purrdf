// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Entailment-**regime** materialization → canonical N-Quads **and a rendered
//! report**, in one call — the shared string boundary every language binding
//! routes through.
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
//!   trailing newline, and a leading `purrdf-reasoning-report 1` banner so a
//!   later change to the format is visible instead of silent.
//!
//! # Portability
//!
//! Pure in-memory string work over the wasm-clean native codecs and the wasm-clean
//! `purrdf-entail` chase — no threads of its own, no filesystem, no clock, no RNG,
//! and no dependency beyond `purrdf-entail` over what the crate already had.

use core::fmt;

use purrdf_entail::{EntailError, ReasoningReport, Regime, implemented, materialize, rules};

/// The accepted regime spellings, in the order an error message lists them.
///
/// These are exactly the value names of the CLI's `--regime` / `--entailment`
/// flag, so one spelling works at the command line, through the C ABI, through
/// WASM and from Python.
pub const REGIME_NAMES: [&str; 7] = ["simple", "rdf", "rdfs", "owl-rl", "owl-direct", "rif", "d"];

/// The subset of [`REGIME_NAMES`] that [`materialize_to_nquads_string`] can close.
///
/// The other three are refused with a message that names these: `owl-direct`
/// needs the query's class expressions, `rif` needs a parsed rule set, and `d`
/// is a spec-inherent boundary for forward materialization.
pub const MATERIALIZABLE_REGIME_NAMES: [&str; 4] = ["simple", "rdf", "rdfs", "owl-rl"];

/// The version banner every rendered report opens with.
pub const REPORT_FORMAT_BANNER: &str = "purrdf-reasoning-report 1";

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
/// The report is never optional and never separately requested — the same
/// discipline [`purrdf_entail::materialize`] enforces in Rust, carried across the
/// string boundary. A binding that renders "RDFS entailment" without saying which
/// nine of the eighteen RDFS patterns did not fire is the overclaim the report
/// exists to prevent.
///
/// # Errors
///
/// * An unknown `regime` spelling — the message names the accepted set.
/// * A regime that cannot be forward-materialized (`owl-direct`, `rif`, `d`) —
///   the message names [`MATERIALIZABLE_REGIME_NAMES`].
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
/// let closed = materialize_to_nquads_string("rdfs", data).expect("rdfs closure");
/// // rdfs9 re-types the instance.
/// assert!(closed.nquads().contains(
///     "<http://example.org/x> \
///      <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/B> ."
/// ));
/// // …and the report says so, and says what it could not do.
/// assert!(closed.report().contains("\nfired rdfs9 "));
/// assert!(closed.report().contains("\ncompleteness sound-incomplete 4\n"));
/// ```
pub fn materialize_to_nquads_string(regime: &str, document: &str) -> Result<RegimeClosure, String> {
    let parsed = parse_regime(regime)?;
    let dataset = purrdf_rdf::parse_dataset(document.as_bytes(), INPUT_MEDIA_TYPE, None)
        .map_err(|diagnostic| diagnostic.to_string())?;
    let (closure, report) = materialize(&dataset, parsed).map_err(|error| match error {
        EntailError::Unsupported(_) => format!(
            "entailment regime \"{regime}\" cannot be forward-materialized \
             (owl-direct needs the query's class expressions, rif needs a parsed rule set, \
             and d is a spec-inherent boundary); materializable regimes: {}",
            MATERIALIZABLE_REGIME_NAMES.join(", ")
        ),
        other => format!("entailment regime \"{regime}\": {other}"),
    })?;
    Ok(RegimeClosure {
        nquads: purrdf_rdf::canonical_flat_nquads(closure.as_ref())?,
        report: render_reasoning_report(&report),
    })
}

/// The rule table `regime` is *defined by*, one specification rule name per line.
///
/// The empty string for a regime with no rule table (`simple`, and the three that
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
/// ```
/// use purrdf_validate::regime::{implemented_rules_string, rules_string};
///
/// let defined = rules_string("rdfs").expect("known");
/// let fired = implemented_rules_string("rdfs").expect("known");
/// assert_eq!(defined.lines().count() - fired.lines().count(), 4);
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
/// purrdf-reasoning-report 1
/// regime <cli-spelling>
/// completeness exact | completeness sound-incomplete <count>
/// missing <rule-id>                       (0..n, specification table order)
/// fired <rule-id> <conclusions>           (0..n, specification table order)
/// boundary <construct> <reason>           (0..n, Construct declaration order)
/// budget join-steps <n>
/// budget stored-facts <n>
/// budget term-arena-bytes <n>
/// contract-hash <64 lowercase hex>
/// inconsistency none | inconsistency <rule-id> premises <n>
/// overclaims false | overclaims true
/// ```
///
/// `overclaims` is derivable from `completeness` and the `boundary` lines, and is
/// emitted anyway: it is the invariant no report may ever violate, and a consumer
/// on the far side of an FFI boundary should not have to re-derive the gate to
/// check it.
///
/// The `inconsistency` line names the rule that detected a clash. It is `none`
/// for every closure this boundary can currently produce — not because the case
/// is unhandled, but because, as
/// [`InconsistencyWitness`](purrdf_entail::InconsistencyWitness) documents, none
/// of the seventeen OWL 2 RL rules that conclude `false` is implemented, so no
/// chase path can reach one. The witness's premise *triples* are not rendered
/// here; a Rust caller that needs them reads them from
/// [`purrdf_entail::materialize`] directly, where they are terms rather than text.
#[must_use]
pub fn render_reasoning_report(report: &ReasoningReport) -> String {
    RenderedReport(report).to_string()
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
        let completeness = report.completeness();
        if completeness.is_exact() {
            writeln!(f, "completeness exact")?;
        } else {
            writeln!(
                f,
                "completeness sound-incomplete {}",
                completeness.missing().len()
            )?;
        }
        for rule in completeness.missing() {
            writeln!(f, "missing {}", rule.as_str())?;
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
        match report.inconsistency() {
            None => writeln!(f, "inconsistency none")?,
            Some(witness) => writeln!(
                f,
                "inconsistency {} premises {}",
                witness.rule().as_str(),
                witness.premises().len()
            )?,
        }
        writeln!(f, "overclaims {}", report.overclaims())
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
/// [`MATERIALIZABLE_REGIME_NAMES`], so a truncated artifact fails loudly instead
/// of passing vacuously.
///
/// # Errors
///
/// A malformed artifact, a case that fails to materialize, a byte difference in
/// either output, or a materializable regime the artifact no longer covers.
pub fn check_regime_golden_vectors() -> Result<(), String> {
    let cases = regime_golden_vectors()?;
    if cases.is_empty() {
        return Err("the regime golden vector artifact holds no cases".to_owned());
    }
    for case in &cases {
        let produced = materialize_to_nquads_string(case.regime(), case.input())
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
    for regime in MATERIALIZABLE_REGIME_NAMES {
        if !cases.iter().any(|case| case.regime() == regime) {
            return Err(format!(
                "the regime golden vector artifact no longer covers regime \"{regime}\""
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
            closure: self.closure.ok_or_else(|| missing("closure"))?,
            report: self.report.ok_or_else(|| missing("report"))?,
        })
    }

    /// The body slot `section` fills.
    fn slot(&mut self, section: Section) -> &mut Option<String> {
        match section {
            Section::Input => &mut self.input,
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
///   `@input`, `@closure`, `@report`, `@end`.
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
            "input" | "closure" | "report" => {
                let section = match keyword {
                    "input" => Section::Input,
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
        for name in MATERIALIZABLE_REGIME_NAMES {
            assert!(REGIME_NAMES.contains(&name), "{name}");
        }
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
            materialize_to_nquads_string("rdfs-plus", SCHEMA).expect_err("unknown"),
            rules_string("rdfs-plus").expect_err("unknown"),
            implemented_rules_string("rdfs-plus").expect_err("unknown"),
        ] {
            assert!(error.contains("accepted: simple, rdf, rdfs"), "{error}");
        }
    }

    /// The three regimes that need inputs this façade does not have are refused
    /// by name, and the refusal says which regimes *do* materialize.
    #[test]
    fn a_non_materializable_regime_is_refused_by_name() {
        for name in ["owl-direct", "rif", "d"] {
            let error = materialize_to_nquads_string(name, SCHEMA).expect_err("unsupported");
            assert!(error.contains(name), "{error}");
            assert!(
                error.contains("materializable regimes: simple, rdf, rdfs, owl-rl"),
                "{error}"
            );
        }
    }

    // ── Materialization ─────────────────────────────────────────────────────

    /// The closure really closes, and `simple` really does not.
    #[test]
    fn the_closure_infers_under_rdfs_and_not_under_simple() {
        let typed = "<http://example.org/x> \
                     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C> .";
        let rdfs = materialize_to_nquads_string("rdfs", SCHEMA).expect("rdfs");
        assert!(rdfs.nquads().contains(typed), "{}", rdfs.nquads());
        let simple = materialize_to_nquads_string("simple", SCHEMA).expect("simple");
        assert!(!simple.nquads().contains(typed), "{}", simple.nquads());
        // `simple` is the identity closure, so its canonical form is the input's.
        assert_eq!(
            simple.nquads(),
            materialize_to_nquads_string("simple", simple.nquads())
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
        let closed = materialize_to_nquads_string("simple", quads).expect("simple");
        assert!(closed.nquads().contains("<http://example.org/g>"));
        // …and the RDFS lane reports it as a boundary rather than reasoning over it.
        let closed = materialize_to_nquads_string("rdfs", quads).expect("rdfs");
        assert!(closed.report().contains("\nboundary named-graph "));
    }

    /// A malformed document is an error, not an empty closure.
    #[test]
    fn a_malformed_document_is_an_error() {
        assert!(materialize_to_nquads_string("rdfs", "this is not n-quads\n").is_err());
    }

    // ── The rendering ───────────────────────────────────────────────────────

    /// The rendering is byte-stable across repeated calls, for every
    /// materializable regime.
    ///
    /// Each `materialize` seeds a freshly-hashed fact store, so a rendering that
    /// leaked any hash order — or any clock, path or address — would diverge here.
    #[test]
    fn the_rendering_is_byte_stable_across_calls() {
        for regime in MATERIALIZABLE_REGIME_NAMES {
            let first = materialize_to_nquads_string(regime, SCHEMA).expect("materializable");
            let second = materialize_to_nquads_string(regime, SCHEMA).expect("materializable");
            assert_eq!(first, second, "{regime}");
            // Ten more, so a one-in-two divergence cannot pass by luck.
            for _ in 0..10 {
                let again = materialize_to_nquads_string(regime, SCHEMA).expect("materializable");
                assert_eq!(again.report(), first.report(), "{regime}");
                assert_eq!(again.nquads(), first.nquads(), "{regime}");
            }
        }
    }

    /// The rendering's shape: banner first, newline-terminated, fixed field
    /// order, and the derived `overclaims` gate never true.
    #[test]
    fn the_rendering_has_the_documented_shape() {
        for regime in MATERIALIZABLE_REGIME_NAMES {
            let report = materialize_to_nquads_string(regime, SCHEMA)
                .expect("materializable")
                .into_parts()
                .1;
            let lines: Vec<&str> = report.lines().collect();
            assert_eq!(lines[0], REPORT_FORMAT_BANNER, "{regime}");
            assert_eq!(lines[1], format!("regime {regime}"), "{regime}");
            assert!(lines[2].starts_with("completeness "), "{regime}");
            assert!(report.ends_with("overclaims false\n"), "{regime}");
            assert!(report.contains("\ninconsistency none\n"), "{regime}");
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
        for regime in MATERIALIZABLE_REGIME_NAMES {
            let defined_set = rules_string(regime).expect("known");
            let defined: Vec<&str> = defined_set.lines().collect();
            let fired_set = implemented_rules_string(regime).expect("known");
            let fired: Vec<&str> = fired_set.lines().collect();
            let expected: Vec<&str> = defined
                .iter()
                .copied()
                .filter(|rule| !fired.contains(rule))
                .collect();

            let report = materialize_to_nquads_string(regime, SCHEMA)
                .expect("materializable")
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

    /// The inventory strings are the specification tables, in table order, and
    /// `implemented` is a subsequence of `rules`.
    #[test]
    fn the_inventory_strings_are_the_specification_tables() {
        assert_eq!(rules_string("owl-rl").expect("known").lines().count(), 78);
        assert_eq!(rules_string("rdfs").expect("known").lines().count(), 18);
        assert_eq!(rules_string("rdf").expect("known").lines().count(), 3);
        for regime in ["simple", "owl-direct", "rif", "d"] {
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
        let report = materialize_to_nquads_string("rdfs", SCHEMA)
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
        let simple = materialize_to_nquads_string("simple", SCHEMA)
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
            materialize_to_nquads_string(regime, SCHEMA)
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
            materialize_to_nquads_string("rdfs", other)
                .expect("rdfs")
                .into_parts()
                .1
                .lines()
                .find_map(|line| line.strip_prefix("contract-hash ").map(str::to_owned))
                .expect("a contract hash")
        });
    }
}
