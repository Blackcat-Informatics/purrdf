// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `reason` subcommand: `Source → materialize → Sink`.
//!
//! Map the requested regime to its library [`Regime`], reject the regimes the CLI
//! cannot materialize (they need inputs it has no way to supply), load the source
//! as a concrete dataset, compute the entailment closure, and write it through the
//! [`sink`] to the output (whose format is inferred from its
//! extension). The resulting loss ledger is surfaced under `--loss-ledger`.

use purrdf_entail::{Regime, materialize};

use crate::cli::{CliRegime, LedgerTarget};
use crate::error::CliError;
use crate::format;
use crate::ledger;
use crate::sink;
use crate::source;

/// Resolve a [`CliRegime`] to its library [`Regime`], rejecting the regimes the CLI
/// cannot materialize.
///
/// OWL-Direct needs the query's class expressions, RIF needs a parsed rule set, and
/// D (datatype) entailment is a spec-inherent materialization boundary — the CLI has
/// no way to supply those inputs, so they map to the exit-3
/// [`CliError::UnsupportedRegime`] path. Shared by `reason` and `convert
/// --entailment` so both reject identically.
pub(crate) fn resolve_materializable_regime(regime: CliRegime) -> Result<Regime, CliError> {
    let regime = regime.to_native();
    if matches!(regime, Regime::OwlDirect | Regime::Rif | Regime::D) {
        return Err(CliError::UnsupportedRegime(format!(
            "entailment regime `{regime:?}` cannot be materialized by the CLI: it needs inputs \
             (query class expressions or a rule set) the CLI has no way to supply"
        )));
    }
    Ok(regime)
}

/// Run the `reason` subcommand.
pub(crate) fn run(
    regime: CliRegime,
    input: &str,
    output: &str,
    ledger_target: &LedgerTarget,
) -> Result<(), CliError> {
    let regime = resolve_materializable_regime(regime)?;

    let source_format = format::resolve(None, input)?;
    let dataset = source::load_dataset(input, source_format, None)?;

    let closure = materialize(&dataset, regime)?;

    let target_format = format::resolve(None, output)?;
    let src_codec = source_format.loss_codec_name();
    let ledger = sink::write_rdf(&*closure, output, target_format, None, src_codec)?;
    ledger::surface(ledger_target, &ledger)
}
