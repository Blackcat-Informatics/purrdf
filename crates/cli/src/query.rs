// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `query` subcommand: evaluate a SPARQL query over a data source.
//!
//! The data source is opened as a view (a pack is queried zero-copy) and the
//! prepared query is evaluated over it with [`NativeSparqlEngine`]. Both the input
//! parse and the query itself resolve relative IRIs against `--base`.
//!
//! ## `--entailment`: query the closure, not the raw view
//!
//! Without `--entailment` the query runs over the raw view (text `RdfDataset` or a
//! zero-copy `PackView`). With `--entailment REGIME` the pipeline reconstructs an
//! owned `Arc<RdfDataset>` up front (a pack is rebuilt via [`source::load_dataset`]),
//! materializes the regime's closure IN MEMORY (rejecting the non-materializable
//! regimes on the same exit-3 path as `reason`), and queries the closure.
//!
//! ## The result-shape × format-kind dispatch
//!
//! `--results-format` is a superset [`QueryFormat`] of the four SPARQL-results
//! serializations and the nine RDF syntaxes; the result SHAPE selects which half is
//! legal:
//!
//! * SELECT [`Solutions`](SparqlResult::Solutions) / ASK
//!   [`Boolean`](SparqlResult::Boolean) + a SPARQL-results format → the W3C results
//!   serializer to stdout (a format that cannot carry the shape — CSV/TSV vs a
//!   boolean — surfaces the serializer's own error);
//! * a CONSTRUCT/DESCRIBE [`Graph`](SparqlResult::Graph) + an RDF syntax → the SAME
//!   [`sink::write_rdf`] the `convert`/`reason` lanes use, so a star-incapable target
//!   (e.g. RDF/XML) projects the RDF-1.2 statement layer and the loss ledger records
//!   the drop (the universal-sink invariant), surfaced under `--loss-ledger`;
//! * a shape/format-kind MISMATCH (solutions/boolean + an RDF syntax, or a graph +
//!   a SPARQL-results format) is a hard runtime error (exit 1).

use purrdf_core::{DatasetView, LossLedger, SparqlResult};
use purrdf_entail::materialize;
use purrdf_sparql_eval::{NativeSparqlEngine, PreparedQuery};
use purrdf_sparql_results::{ResultProvenance, serialize};

use crate::cli::{CliRegime, LedgerTarget, QueryFormat};
use crate::error::CliError;
use crate::format::{self, CliFormat};
use crate::ledger;
use crate::reason;
use crate::sink;
use crate::source::{self, ViewOp};

/// The generic query operation: evaluate the prepared query over whichever concrete
/// view the data source resolved to, returning the (fully owned) [`SparqlResult`].
///
/// A [`SparqlResult`] borrows nothing from the view — `Graph` is an `Arc<RdfDataset>`
/// and every solution cell is an owned `TermValue` — so the result outlives the
/// borrowed view and is emitted after `run_over_input` returns.
struct QueryOp<'a> {
    engine: &'a NativeSparqlEngine,
    prepared: &'a PreparedQuery,
}

impl ViewOp for QueryOp<'_> {
    type Output = SparqlResult;

    fn run<D: DatasetView + Sync>(self, view: &D) -> Result<SparqlResult, CliError> {
        Ok(self.engine.query_prepared_view(view, self.prepared, &[])?)
    }
}

/// Emit a SPARQL result to stdout, dispatching on the result shape × format kind, and
/// surface the loss ledger the emission produced.
fn emit_result(
    result: &SparqlResult,
    results_format: QueryFormat,
    base: Option<&str>,
    ledger_target: &LedgerTarget,
) -> Result<(), CliError> {
    match result {
        SparqlResult::Solutions { .. } | SparqlResult::Boolean(_) => {
            let Some(fmt) = results_format.to_results_format() else {
                let kind = match result {
                    SparqlResult::Boolean(_) => "an ASK boolean",
                    _ => "SELECT solutions",
                };
                return Err(CliError::Runtime(format!(
                    "{kind} result cannot be serialized to the RDF syntax `{}`: a SELECT/ASK \
                     result needs a SPARQL-results format (json/xml/csv/tsv)",
                    results_format.token()
                )));
            };
            // The results serializer itself rejects the shapes its format cannot carry
            // (CSV/TSV reject a boolean); its `Err` maps cleanly to a runtime failure.
            let outcome = serialize(result, fmt, &ResultProvenance::default())?;
            sink::write_out("-", &outcome.bytes)?;
            // A tabular/boolean result performs no lossy transcode; honor the flag
            // uniformly with an empty ledger.
            ledger::surface(ledger_target, &LossLedger::new())
        }
        SparqlResult::Graph(graph) => {
            let Some(fmt) = results_format.to_rdf_format() else {
                return Err(CliError::Runtime(format!(
                    "a CONSTRUCT/DESCRIBE graph result cannot be serialized to the SPARQL-results \
                     format `{}`: a graph needs an RDF syntax \
                     (turtle/trig/ntriples/nquads/rdfxml/trix/hextuples/jsonld/yamlld)",
                    results_format.token()
                )));
            };
            // The universal sink: a star-incapable target (RDF/XML, TriX, HexTuples)
            // projects the RDF-1.2 statement layer and records the drop in the ledger.
            // The graph is freshly constructed, so there is no source codec to seed the
            // contract-loss half (`None`); only the realized dropped-row counts appear.
            let ledger = sink::write_rdf(&**graph, "-", CliFormat::Rdf(fmt), base, None)?;
            ledger::surface(ledger_target, &ledger)
        }
    }
}

/// Run the `query` subcommand.
pub(crate) fn run(
    data: &str,
    base: Option<&str>,
    entailment: Option<CliRegime>,
    results_format: QueryFormat,
    query: &str,
    ledger_target: &LedgerTarget,
) -> Result<(), CliError> {
    let data_format = format::resolve(None, data)?;

    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(query, base)?;

    let result = match entailment {
        Some(regime) => {
            // The `--entailment` lane: reconstruct an owned dataset (a pack is rebuilt),
            // materialize the closure in memory, and query THAT.
            let regime = reason::resolve_materializable_regime(regime)?;
            let dataset = source::load_dataset(data, data_format, base)?;
            let closure = materialize(&dataset, regime)?;
            engine.query_prepared(&closure, &prepared, &[])?
        }
        None => source::run_over_input(
            data,
            data_format,
            base,
            QueryOp {
                engine: &engine,
                // `prepared` is an `Arc<PreparedQuery>`; reborrow it as `&PreparedQuery`.
                prepared: &prepared,
            },
        )?,
    };

    emit_result(&result, results_format, base, ledger_target)
}
