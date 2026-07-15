// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `reason` subcommand: `Source → materialize → Sink`.
//!
//! Map the requested regime to its library [`Regime`], reject the regimes the CLI
//! cannot materialize (they need inputs it has no way to supply), load the source
//! as a concrete dataset, compute the entailment closure, and write it through the
//! [`sink`] to the output. Both `--from`/`--to` are resolved up front (mirroring
//! `convert`): an explicit choice always wins, otherwise the format is inferred
//! from the input/output path's extension; `-` (stdin/stdout) has no extension and
//! REQUIRES the explicit override. Resolving both before `load_dataset`/
//! `materialize` runs means an unresolvable output format fails fast, not after
//! the source has already been loaded and closed over. The resulting loss ledger
//! is surfaced under `--loss-ledger`.

use purrdf_entail::{Regime, materialize};

use crate::cli::{CliRdfFormat, CliRegime, LedgerTarget};
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
    // Each boundary regime is unsupported for a DIFFERENT spec-inherent reason; name it.
    let reason = match regime {
        Regime::OwlDirect => Some(
            "it needs the query's class expressions, which materialization alone cannot supply",
        ),
        Regime::Rif => Some("it needs a parsed RIF rule set, which the CLI has no way to supply"),
        Regime::D => Some("datatype (D) entailment is a spec-inherent materialization boundary"),
        Regime::Simple | Regime::Rdf | Regime::Rdfs | Regime::OwlRl => None,
    };
    if let Some(reason) = reason {
        return Err(CliError::UnsupportedRegime(format!(
            "entailment regime `{regime:?}` cannot be materialized by the CLI: {reason}"
        )));
    }
    Ok(regime)
}

/// Run the `reason` subcommand.
pub(crate) fn run(
    regime: CliRegime,
    from: Option<CliRdfFormat>,
    to: Option<CliRdfFormat>,
    base: Option<&str>,
    input: &str,
    output: &str,
    ledger_target: &LedgerTarget,
) -> Result<(), CliError> {
    let regime = resolve_materializable_regime(regime)?;

    // Resolve BOTH formats up front (before touching the source) so an
    // unresolvable OUT fails fast rather than after the (potentially
    // expensive) load + materialize work has already run.
    let source_format = format::resolve(from, input)?;
    let target_format = format::resolve(to, output)?;

    let dataset = source::load_dataset(input, source_format, base)?;

    let closure = materialize(&dataset, regime)?;

    let src_codec = source_format.loss_codec_name();
    let ledger = sink::write_rdf(&*closure, output, target_format, base, src_codec)?;
    ledger::surface(ledger_target, &ledger)
}
