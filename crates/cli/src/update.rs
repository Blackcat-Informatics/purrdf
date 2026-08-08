// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `update` subcommand: COW-atomic SPARQL UPDATE followed by RDF serialization.

use purrdf_core::{SparqlEngine, SparqlRequest};
use purrdf_rdf::JsonLdSerializeOptions;
use purrdf_sparql_eval::{GovernedUpdateOutcome, NativeSparqlEngine, QueryOptions};

use crate::cli::{CliRdfFormat, LedgerTarget};
use crate::error::{CliError, CliOutcome};
use crate::format;
use crate::governors::{self, GovernorFlags};
use crate::{ledger, sink, source};

/// The resolved `update` flags.
pub(crate) struct UpdateOptions<'a> {
    pub(crate) data: &'a str,
    pub(crate) from: Option<CliRdfFormat>,
    pub(crate) output: &'a str,
    pub(crate) to: Option<CliRdfFormat>,
    pub(crate) base: Option<&'a str>,
    pub(crate) update: &'a str,
    pub(crate) governors: GovernorFlags,
    pub(crate) jsonld_options: Option<&'a JsonLdSerializeOptions>,
}

/// Apply the request and emit the new dataset only after the whole request commits.
pub(crate) fn run(
    options: &UpdateOptions<'_>,
    ledger_target: &LedgerTarget,
) -> Result<CliOutcome, CliError> {
    let source_format = format::resolve(options.from, options.data)?;
    let target_format = format::resolve(options.to, options.output)?;
    let mut dataset = source::load_dataset(options.data, source_format, options.base)?;
    let engine = NativeSparqlEngine::new();
    let request = SparqlRequest {
        query: options.update,
        base_iri: options.base,
        substitutions: &[],
    };

    if options.governors.is_engaged() {
        let governors = options.governors.to_governors();
        // `QueryOptions::EMPTY`: the CLI wires no SHACL-AF function table and no
        // property-function registry.
        match engine.update_governed(&mut dataset, request, QueryOptions::EMPTY, &governors)? {
            GovernedUpdateOutcome::Applied { .. } => {}
            GovernedUpdateOutcome::BudgetExhausted {
                tripped, evidence, ..
            } => {
                eprint!("{}", governors::render_update_trip(tripped, &evidence));
                return Ok(CliOutcome::BudgetExhausted);
            }
        }
    } else {
        engine.update(&mut dataset, request)?;
    }

    let loss = sink::write_rdf(
        &*dataset,
        options.output,
        target_format,
        options.base,
        source_format.loss_codec_name(),
        options.jsonld_options,
    )?;
    ledger::surface(ledger_target, &loss)?;
    Ok(CliOutcome::Complete)
}
