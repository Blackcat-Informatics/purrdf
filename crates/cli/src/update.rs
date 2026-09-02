// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `update` subcommand: COW-atomic SPARQL UPDATE followed by RDF serialization.

use purrdf_core::SparqlRequest;
use purrdf_rdf::JsonLdSerializeOptions;
use purrdf_sparql_eval::{GovernedUpdateOutcome, NativeSparqlEngine, QueryOptions};

use crate::cli::{CliRdfFormat, LedgerTarget};
use crate::error::{CliError, CliOutcome};
use crate::format;
use crate::governors::{self, GovernorFlags};
use crate::path_relation::{self, PathRelationSpec};
use crate::query::build_aggregate_registry;
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
    /// `--aggregate-namespace`: registers purrdf's first-party statistical aggregate
    /// set under this IRI namespace, reachable from a `DELETE`/`INSERT … WHERE` clause
    /// through a nested `SELECT … GROUP BY`. `None` leaves the set unregistered,
    /// exactly as before this flag existed.
    pub(crate) aggregate_namespace: Option<&'a str>,
    /// `--path-relation`, repeatable: the path-witness relations this request registers,
    /// reachable from a `DELETE`/`INSERT … WHERE` clause. Empty leaves
    /// `QueryOptions::property_functions` at its `EMPTY` value, exactly as before this
    /// flag existed.
    pub(crate) path_relations: &'a [PathRelationSpec],
}

/// Apply the request and emit the new dataset only after the whole request commits.
pub(crate) fn run(
    options: &UpdateOptions<'_>,
    ledger_target: &LedgerTarget,
) -> Result<CliOutcome, CliError> {
    let source_format = format::resolve(options.from, options.data)?;
    let target_format = format::resolve_target(options.to, options.output, "the --to target")?;
    let mut dataset = source::load_dataset(options.data, source_format, options.base)?;
    let engine = NativeSparqlEngine::new();
    let request = SparqlRequest {
        query: options.update,
        base_iri: options.base,
        substitutions: &[],
    };
    // `AggregateRegistry::register_statistical_aggregates` takes only a namespace
    // string; `None` here reproduces `QueryOptions::EMPTY` byte-for-byte, so an
    // omitted `--aggregate-namespace` changes nothing about existing behaviour.
    let aggregates = build_aggregate_registry(options.aggregate_namespace);
    // The path-witness relations `--path-relation` declares, snapshotted from the
    // PRE-update dataset — the same state the request's `WHERE` clause matches, and the
    // only state that exists before the mutation is computed. A duplicate IRI across
    // repeated flags is refused first: `PropertyFunctionRegistry::register` PANICS on one,
    // and a command line is a host misconfiguration rather than an abort.
    path_relation::refuse_duplicate_iris(options.path_relations)?;
    let relations = path_relation::build_registry(&*dataset, options.path_relations)?;
    // `QueryOptions::EMPTY` for every axis but `aggregates` and `property_functions`: the
    // CLI wires no SHACL-AF function table.
    // `static`, not a bare `&AggregateRegistry::EMPTY` temporary: `query_options` below
    // outlives this statement, and a `HashMap`-backed registry's drop glue blocks Rust's
    // rvalue static promotion for a reference that must live that long.
    static EMPTY_AGGREGATES: purrdf_sparql_eval::AggregateRegistry =
        purrdf_sparql_eval::AggregateRegistry::EMPTY;
    static EMPTY_RELATIONS: purrdf_sparql_eval::PropertyFunctionRegistry =
        purrdf_sparql_eval::PropertyFunctionRegistry::EMPTY;
    let query_options = QueryOptions {
        aggregates: aggregates.as_ref().unwrap_or(&EMPTY_AGGREGATES),
        property_functions: relations.as_ref().unwrap_or(&EMPTY_RELATIONS),
        ..QueryOptions::EMPTY
    };

    if options.governors.is_engaged() {
        let governors = options.governors.to_governors();
        match engine.update_governed(&mut dataset, request, query_options, &governors)? {
            GovernedUpdateOutcome::Applied { .. } => {}
            GovernedUpdateOutcome::BudgetExhausted {
                tripped, evidence, ..
            } => {
                eprint!("{}", governors::render_update_trip(tripped, &evidence));
                return Ok(CliOutcome::BudgetExhausted);
            }
        }
    } else {
        // `update_with_options(.., QueryOptions::EMPTY)` is the trait-level
        // `SparqlEngine::update` this call replaces, byte-for-byte (see
        // `NativeSparqlEngine`'s `SparqlEngine` impl) — switching to it unconditionally
        // does not change behaviour when `aggregates` is `None`.
        engine.update_with_options(&mut dataset, request, query_options)?;
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
