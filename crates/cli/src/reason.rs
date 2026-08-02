// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `reason` subcommand: `Source → materialize → Sink`.
//!
//! Map the requested regime and its `--rules` document to an [`EntailmentPlan`], load the
//! source as a concrete dataset, compute the entailment closure, and write it through the
//! [`sink`] to the output. Both `--from`/`--to` are resolved up front (mirroring
//! `convert`): an explicit choice always wins, otherwise the format is inferred
//! from the input/output path's extension; `-` (stdin/stdout) has no extension and
//! REQUIRES the explicit override. Resolving both before `load_dataset`/
//! `materialize` runs means an unresolvable output format fails fast, not after
//! the source has already been loaded and closed over. The resulting loss ledger
//! is surfaced under `--loss-ledger`, and the run's reasoning report under
//! `--report` — on the refusal path as well as the success path, because an
//! inconsistent knowledge base has no closure and still had a run.
//!
//! # Every regime materializes here
//!
//! There is no regime this subcommand refuses. `purrdf_entail::materialize` takes a
//! [`Materialization`], which carries each regime's own input, and [`EntailmentPlan`] is
//! where the CLI supplies it: `rif` reads its rule set from `--rules`, and `owl-direct`
//! runs the query-independent tableau augmentation, because `reason` transforms a document
//! and has no query for a query-directed one to be directed by.

use std::path::Path;

use purrdf::QueryEntailment;
use purrdf_entail::{Materialization, Regime, RuleSet, parse_rif_xml};
use purrdf_rdf::JsonLdSerializeOptions;

use crate::cli::{CliRdfFormat, CliRegime, LedgerTarget, ReportTarget};
use crate::error::CliError;
use crate::format;
use crate::ledger;
use crate::report;
use crate::sink;
use crate::source;

/// A resolved entailment plan: the regime, plus the input that regime is defined by.
///
/// It exists because the input is OWNED and the plan BORROWS it —
/// [`Materialization::Rif`] holds a `&RuleSet`, so the rule set needs a place to live that
/// outlives the call. Shared by `reason`, `convert --entailment` and `query --entailment`
/// so all three resolve identically.
#[derive(Debug)]
pub(crate) struct EntailmentPlan {
    /// The regime the plan runs.
    regime: Regime,
    /// The rule set `--rules` supplied; empty for every regime but `rif`, which is
    /// the only one whose calculus is the caller's rather than a specification's.
    rules: RuleSet,
}

impl EntailmentPlan {
    /// Resolve `--regime`/`--entailment` together with `--rules`.
    ///
    /// The two arguments are one input, so they are validated as one: `rif` REQUIRES a
    /// rule document and every other regime FORBIDS one. Both failures are usage errors
    /// (exit 2) — they describe an incomplete or contradictory command line, not a regime
    /// this pipeline will not run.
    ///
    /// `owl-direct` needs no flag: the tableau lane is directed by a query's class
    /// expressions, and `reason`/`convert` transform a document rather than answering a
    /// query, so what runs is the query-independent augmentation — the classification, the
    /// realization, the entailed role assertions and the `owl:sameAs` identifications the
    /// tableau decides about the ontology's own named terms.
    pub(crate) fn resolve(regime: CliRegime, rules: Option<&Path>) -> Result<Self, CliError> {
        let regime = regime.to_native();
        let rules = match (regime, rules) {
            (Regime::Rif, Some(path)) => read_rule_set(path)?,
            (Regime::Rif, None) => {
                return Err(CliError::Usage(
                    "entailment regime `rif` entails under a rule set this workspace does not \
                     declare, so it requires `--rules <FILE>` naming a RIF-in-XML rule document"
                        .to_owned(),
                ));
            }
            (_, Some(path)) => {
                return Err(CliError::Usage(format!(
                    "`--rules {}` was supplied for entailment regime `{regime:?}`, whose rule \
                     table is the specification's; only `rif` takes a rule document",
                    path.display()
                )));
            }
            (_, None) => RuleSet::new(),
        };
        Ok(Self { regime, rules })
    }

    /// The plan to hand [`purrdf_entail::materialize`].
    pub(crate) fn materialization(&self) -> Materialization<'_> {
        match self.regime {
            Regime::Simple => Materialization::Simple,
            Regime::Rdf => Materialization::Rdf,
            Regime::Rdfs => Materialization::Rdfs,
            Regime::OwlRl => Materialization::OwlRl,
            Regime::D => Materialization::D,
            Regime::OwlDirect => Materialization::OwlDirect(&[]),
            Regime::Rif => Materialization::Rif(&self.rules),
        }
    }

    /// The same resolved plan as a [`QueryEntailment`], for the lane that has a QUERY.
    ///
    /// [`Self::materialization`] is the document-transforming plan: `reason` and `convert`
    /// have no query, so `owl-direct` there is the query-INDEPENDENT augmentation over an
    /// empty basic graph pattern. `query --entailment` does have one, and handing that same
    /// empty pattern to `materialize` was a capability the CLI threw away — the combined
    /// approach is directed BY the query, so a lane that passes it no query cannot run it and
    /// answers a query whose certain answer the library computes with an empty result set.
    /// This is the plan `purrdf::query_with_entailment` takes, and the reason `query` no
    /// longer open-codes "materialize, then evaluate".
    ///
    /// The mapping is total over the seven regimes, exactly as [`Self::materialization`] is:
    /// there is no regime the query lane serves that the document lane does not, or the
    /// reverse.
    pub(crate) fn query_entailment(&self) -> QueryEntailment<'_> {
        match self.regime {
            Regime::Simple => QueryEntailment::Simple,
            Regime::Rdf => QueryEntailment::Rdf,
            Regime::Rdfs => QueryEntailment::Rdfs,
            Regime::OwlRl => QueryEntailment::OwlRl,
            Regime::D => QueryEntailment::D,
            Regime::OwlDirect => QueryEntailment::OwlDirect,
            Regime::Rif => QueryEntailment::Rif(&self.rules),
        }
    }
}

/// Read and parse a RIF-in-XML rule document.
///
/// An `Import` is refused by name: resolving one means fetching whatever its location
/// points at, and this pipeline reads the files the operator named and nothing else. A
/// caller who needs imports resolves them itself through
/// `purrdf_entail::resolve_rif_imports`.
fn read_rule_set(path: &Path) -> Result<RuleSet, CliError> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| CliError::Runtime(format!("--rules {}: {error}", path.display())))?;
    let parsed = parse_rif_xml(&text)
        .map_err(|error| CliError::Runtime(format!("--rules {}: {error}", path.display())))?;
    if let Some(import) = parsed.imports.first() {
        return Err(CliError::Runtime(format!(
            "--rules {}: the rule document imports \"{}\", and this pipeline fetches nothing \
             the operator did not name",
            path.display(),
            import.location
        )));
    }
    Ok(parsed.ruleset)
}

/// Run the `reason` subcommand.
#[allow(
    clippy::too_many_arguments,
    reason = "the CLI dispatcher passes the command fields and shared sink configuration explicitly"
)]
pub(crate) fn run(
    regime: CliRegime,
    rules: Option<&Path>,
    from: Option<CliRdfFormat>,
    to: Option<CliRdfFormat>,
    base: Option<&str>,
    input: &str,
    output: &str,
    jsonld_options: Option<&JsonLdSerializeOptions>,
    ledger_target: &LedgerTarget,
    report_target: &ReportTarget,
) -> Result<(), CliError> {
    let plan = EntailmentPlan::resolve(regime, rules)?;

    // Resolve BOTH formats up front (before touching the source) so an
    // unresolvable OUT fails fast rather than after the (potentially
    // expensive) load + materialize work has already run.
    let source_format = format::resolve(from, input)?;
    let target_format = format::resolve(to, output)?;
    sink::validate_jsonld_options(target_format, jsonld_options)?;

    let dataset = source::load_dataset(input, source_format, base)?;

    // The closure goes to the sink and the report goes to `--report`: `reason` writes RDF,
    // and the evidence of what produced it is a second output rather than a discarded one.
    // An INCONSISTENT input has no closure and still gets its report written — see
    // `report::materialize_reported`.
    let closure = report::materialize_reported(&dataset, plan.materialization(), report_target)?;

    let src_codec = source_format.loss_codec_name();
    let ledger = sink::write_rdf(
        &*closure,
        output,
        target_format,
        base,
        src_codec,
        jsonld_options,
    )?;
    ledger::surface(ledger_target, &ledger)
}
