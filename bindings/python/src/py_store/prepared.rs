// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The Python surface of a **prepared, parameterized query**.
//!
//! `Store.query(text)` parses and admits `text` on every call. A caller running one
//! query per row — which is what SHACL validation, a rule fixpoint, and any
//! row-driven traversal all are — therefore pays a parse-shaped cost per row to be
//! handed back the same plan, and a caller who instead splices the row's value into
//! the query text pays a real re-parse, because a spliced query is a different query.
//!
//! [`PyPreparedQuery`] is the alternative the engine already wanted callers to have:
//! prepare once, then bind and run.
//!
//! ```python
//! q = store.prepare("SELECT ?o WHERE { ?this <http://example.org/p> ?o }",
//!                   parameters=["this"])
//! for node in nodes:
//!     for row in q.run(this=node):
//!         ...
//! ```
//!
//! # It is not thread-safe, and that is the design rather than an omission
//!
//! A prepared query is bound by `&mut` for the duration of a run, because a query
//! body can re-enter the evaluator and a handle reachable a second time while it is
//! in flight is a handle two evaluations can disagree about. Python sees that as a
//! plain object that must not be shared between threads; `run` takes `&mut self`, so
//! two concurrent calls on one object raise rather than interleave.

use std::sync::Arc;

use purrdf_core::{RdfDataset, TermValue};
use purrdf_sparql_eval::{InternedOutcome, NativeSparqlEngine, PreparedExecution, QueryOptions};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use super::query::materialize_results;
use super::term::{extract_term, rdf_term_to_value};

/// A SPARQL query parsed and admitted once, run many times with different bindings.
///
/// Built by `Store.prepare`.
#[pyclass(name = "PreparedQuery", module = "purrdf")]
pub(crate) struct PyPreparedQuery {
    /// The admitted plan and its parameter slots.
    execution: PreparedExecution,
    /// The snapshot this query runs against.
    ///
    /// Taken once, when the query is prepared. A prepared query therefore answers
    /// over the store **as it was at prepare time**, which is stated on `prepare` —
    /// re-reading the store per run would make two runs of one prepared query
    /// silently disagree for a reason the caller never asked about.
    dataset: Arc<RdfDataset>,
}

#[pymethods]
impl PyPreparedQuery {
    /// The declared parameter names, in declaration order.
    #[getter]
    fn parameters(&self) -> Vec<String> {
        self.execution
            .parameters()
            .iter()
            .map(|parameter| parameter.as_str().to_owned())
            .collect()
    }

    /// Bind every declared parameter and run, returning the results exactly as
    /// `Store.query` does.
    ///
    /// Each keyword names a declared parameter. An unknown keyword raises, and so
    /// does a parameter left unbound: an unbound focus would answer over every
    /// subject, which is a silently wider answer rather than a visible mistake.
    #[pyo3(signature = (**bindings))]
    fn run(&mut self, py: Python<'_>, bindings: Option<&Bound<'_, PyDict>>) -> PyResult<Py<PyAny>> {
        if let Some(bindings) = bindings {
            for (name, value) in bindings {
                let name: String = name.extract()?;
                let value: TermValue = rdf_term_to_value(&extract_term(&value)?);
                self.execution
                    .bind_named(&name, value)
                    .map_err(|e| PyValueError::new_err(e.to_string()))?;
            }
        }
        let execution = &mut self.execution;
        let dataset = &self.dataset;
        // The engine is built HERE rather than held, because it is deliberately
        // `!Sync` — its plan cache is a `RefCell` — so it cannot live in a Python
        // object at all. Nothing is lost that this class exists to keep: the plan is
        // already parsed and admitted, and the execution holds it by `Arc`, so a run
        // does no parse, no admission and no cache probe. What a fresh engine gives
        // up is the join-order memo between runs, which is a plan-shaped hint rather
        // than the plan.
        let result = py.detach(move || {
            let engine = NativeSparqlEngine::new();
            engine
                .execute(execution, &**dataset, QueryOptions::EMPTY, |outcome| {
                    materialize_interned(&outcome)
                })
                .map_err(|e| PyValueError::new_err(format!("query evaluation error: {e}")))
        })?;
        materialize_results(py, result)
    }

    fn __repr__(&self) -> String {
        format!("PreparedQuery(parameters={:?})", self.parameters())
    }
}

/// Copy an interned outcome into the owned form `materialize_results` takes.
///
/// The interned rows do not outlive the evaluation that produced them, so they are
/// read here, inside `visit`, rather than returned.
fn materialize_interned<D: purrdf_core::DatasetView + Sync>(
    outcome: &InternedOutcome<'_, '_, D>,
) -> purrdf_core::SparqlResult {
    match outcome {
        InternedOutcome::Boolean(value) => purrdf_core::SparqlResult::Boolean(*value),
        InternedOutcome::Graph(graph) => purrdf_core::SparqlResult::Graph(Arc::clone(graph)),
        InternedOutcome::Solutions(solutions) => {
            let variables = solutions
                .variables()
                .iter()
                .map(|variable| variable.as_str().to_owned())
                .collect();
            let rows = solutions
                .rows()
                .iter()
                .map(|row| {
                    (0..solutions.variables().len())
                        .map(|column| solutions.cell(row, column))
                        .collect()
                })
                .collect();
            purrdf_core::SparqlResult::Solutions {
                variables,
                rows,
                aux: solutions.constructed_dataset(),
            }
        }
    }
}

/// Build a prepared query over `dataset`.
pub(super) fn prepare(
    dataset: Arc<RdfDataset>,
    engine: &NativeSparqlEngine,
    query: &str,
    parameters: &[String],
) -> PyResult<PyPreparedQuery> {
    let borrowed: Vec<&str> = parameters.iter().map(String::as_str).collect();
    let execution = engine
        .prepare_execution(query, None, &borrowed, QueryOptions::EMPTY)
        .map_err(|e| PyValueError::new_err(format!("query preparation failed: {e}")))?;
    Ok(PyPreparedQuery { execution, dataset })
}
