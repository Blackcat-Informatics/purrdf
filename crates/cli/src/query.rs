// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `query` subcommand: evaluate a SPARQL query over a data source.
//!
//! The data source is opened as a view (a pack is queried zero-copy) and the
//! prepared query is evaluated over it with [`NativeSparqlEngine`]. Results are
//! emitted to stdout:
//!
//! * SELECT solutions and ASK booleans go through the W3C SPARQL-results
//!   serializer in the requested `--results-format`; a shape the chosen format
//!   cannot carry (e.g. an ASK boolean under CSV) surfaces the serializer's own
//!   error as a runtime failure.
//! * a CONSTRUCT/DESCRIBE graph is serialized as **N-Triples** — the core RDF
//!   projection for a graph result (the full RDF-format dispatch is later work).

use purrdf_core::{DatasetView, SparqlResult};
use purrdf_rdf::{NativeRdfFormat, serialize_dataset_to_format};
use purrdf_sparql_eval::{NativeSparqlEngine, PreparedQuery};
use purrdf_sparql_results::{ResultProvenance, serialize};

use crate::cli::{LedgerTarget, ResultsFormat};
use crate::error::CliError;
use crate::format;
use crate::ledger;
use crate::sink;
use crate::source::{self, ViewOp};

/// The generic query operation: evaluate the prepared query over whichever concrete
/// view the data source resolved to, then emit the result.
struct QueryOp<'a> {
    engine: &'a NativeSparqlEngine,
    prepared: &'a PreparedQuery,
    results_format: ResultsFormat,
}

impl ViewOp for QueryOp<'_> {
    type Output = ();

    fn run<D: DatasetView + Sync>(self, view: &D) -> Result<(), CliError> {
        let result = self.engine.query_prepared_view(view, self.prepared, &[])?;
        emit_result(&result, self.results_format)
    }
}

/// Emit a SPARQL result to stdout in the chosen results format.
fn emit_result(result: &SparqlResult, results_format: ResultsFormat) -> Result<(), CliError> {
    match result {
        SparqlResult::Solutions { .. } | SparqlResult::Boolean(_) => {
            let outcome = serialize(
                result,
                results_format.to_native(),
                &ResultProvenance::default(),
            )?;
            sink::write_out("-", &outcome.bytes)
        }
        SparqlResult::Graph(graph) => {
            // Core behavior: a CONSTRUCT/DESCRIBE graph is projected to N-Triples.
            let outcome = serialize_dataset_to_format(&**graph, NativeRdfFormat::NTriples, None)?;
            sink::write_out("-", &outcome.bytes)
        }
    }
}

/// Run the `query` subcommand.
pub(crate) fn run(
    data: &str,
    results_format: ResultsFormat,
    query: &str,
    ledger_target: &LedgerTarget,
) -> Result<(), CliError> {
    let data_format = format::resolve(None, data)?;

    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(query, None)?;

    source::run_over_input(
        data,
        data_format,
        None,
        QueryOp {
            engine: &engine,
            // `prepared` is an `Arc<PreparedQuery>`; reborrow it as `&PreparedQuery`.
            prepared: &prepared,
            results_format,
        },
    )?;

    // Querying performs no lossy transcode, but honor the flag uniformly.
    ledger::surface(ledger_target, &purrdf_core::LossLedger::new())
}
