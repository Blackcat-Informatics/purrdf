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

use purrdf_core::TermValue;
use purrdf_sparql_eval::{
    AggregateRegistry, ExtensionEnv, InternedOutcome, NativeSparqlEngine, ParserOptions,
    PreparedExecution, PropertyFunctionRegistry, StandpointPredicates,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use super::PyStore;
use super::env::extension_env;
use super::query::materialize_results;
use super::term::{extract_term, rdf_term_to_value};

/// A SPARQL query parsed and admitted once, run many times with different bindings.
///
/// Built by `Store.prepare`.
#[pyclass(name = "PreparedQuery", module = "purrdf")]
pub(crate) struct PyPreparedQuery {
    /// The admitted plan and its parameter slots.
    execution: PreparedExecution,
    /// The store this query was prepared against.
    ///
    /// [`run`](Self::run) re-reads and re-freezes this store's CURRENT contents on
    /// every call rather than holding a snapshot taken once at prepare time — what
    /// is prepared is the PLAN, not the data. `Store.prepare`'s own module doc names
    /// "SHACL validation, a rule fixpoint" as the motivating use case, and a
    /// fixpoint mutates its store every round by definition: refusing a run whose
    /// store has advanced since `prepare` would make that use case impossible,
    /// which would be satisfying the feature by refusing it.
    store: Py<PyStore>,
    /// The extension environment `execution`'s plan was admitted under: the parse
    /// configuration `Store.prepare`'s `extension_namespaces` /
    /// `property_fn_namespaces` declared, the relation registry its `relations` /
    /// `relations_from_graph` / `path_relations` built, and the custom-aggregate
    /// registry its `aggregate_namespace` built.
    ///
    /// **Load-bearing, not incidental.** `execution` was parsed and admitted against
    /// this exact environment: a plan admitted with no relation registry in scope
    /// already lowered a registered relation's predicate to an ORDINARY triple
    /// pattern, so [`run`](Self::run) must hand the engine this SAME environment back
    /// rather than, say, `ExtensionEnv::empty()` — otherwise the engine's own
    /// `check_prepared_registries_unchanged` guard (which compares content-derived
    /// fingerprints, not object identity) refuses the run rather than silently
    /// answering short.
    ///
    /// It is the ENVIRONMENT that is held, not the registries it was assembled from,
    /// and that is what makes "prepare and run cannot disagree" a fact about this
    /// field rather than a resemblance between two call sites: there is one value,
    /// derived once by [`extension_env`], and both the admission and every run read
    /// it. A second derivation at run time would be a second answer to "what does
    /// this query text mean", which is the shape of the defect the environment type
    /// exists to remove.
    env: ExtensionEnv,
    /// The `(according_to, sharpens)` predicate table `Store.prepare` was given.
    ///
    /// Unlike the environment above, this is not *admission* configuration — it is
    /// read at EVALUATION time, off the engine, by `heldIn` and loss-aware
    /// `CONSTRUCT` (see [`purrdf_sparql_eval::NativeSparqlEngine::with_standpoint_predicates`]).
    /// [`run`](Self::run) builds a fresh engine per call (see its own doc comment for
    /// why one cannot be held on this object), so this is what lets that fresh engine
    /// answer `heldIn` the same way the engine `prepare` admitted the plan under
    /// would have.
    standpoint_predicates: Option<(String, String)>,
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
    ///
    /// A keyword-argument call reads as TOTAL: every slot is reset to unbound
    /// before this call's keywords are applied, so `run(this=X)` means "these are
    /// the bindings", never "these, plus whatever an earlier call left behind". A
    /// bare `run()` after a prior `run(this=X)` is refused exactly as it would be
    /// on a handle nothing had ever bound — it does not silently re-answer for `X`.
    #[pyo3(signature = (**bindings))]
    fn run(&mut self, py: Python<'_>, bindings: Option<&Bound<'_, PyDict>>) -> PyResult<Py<PyAny>> {
        // Every slot goes back to unbound before this call's own keywords are
        // applied, so a parameter an EARLIER call bound but this call does not
        // (re)mention is `None` again here — not a stale value left over from that
        // earlier call. That is what makes a keyword-argument call read as TOTAL:
        // `run(this=X)` means "these are the bindings", never "these, plus whatever
        // a prior call left behind". See `PreparedExecution::unbind_all`'s own doc
        // for why this is the engine's job rather than a second bookkeeping layer
        // here.
        self.execution.unbind_all();
        if let Some(bindings) = bindings {
            for (name, value) in bindings {
                let name: String = name.extract()?;
                let value: TermValue = rdf_term_to_value(&extract_term(&value)?);
                self.execution
                    .bind_named(&name, value)
                    .map_err(|e| PyValueError::new_err(e.to_string()))?;
            }
        }
        // No unbound check here: a slot this call left unmentioned is `None` after
        // `unbind_all` above, and `NativeSparqlEngine::execute` below already
        // refuses any still-`None` slot with its own diagnostic — the one place
        // that decides what "still unbound" means and what it says.

        // Re-read the owning store FRESH for this run (see `PyPreparedQuery::store`)
        // rather than a snapshot taken once at prepare time, so a fixpoint's mutation
        // between rounds is visible to the next round's run. Frozen while the GIL is
        // held (a cheap borrow), then the heavy freeze work itself runs detached
        // inside `freeze_snapshot`.
        let dataset = {
            let store = self.store.bind(py).borrow();
            store.freeze_snapshot(py)?
        };

        // Built from a direct field access (not through a `&self` method) so this
        // borrow of `env` stays disjoint from the `&mut self.execution` borrow just
        // below — both are held live across the `py.detach` call.
        //
        // The environment is the one `prepare` admitted this plan under, read back
        // rather than re-derived: see [`PyPreparedQuery::env`].
        let options = purrdf_sparql_eval::QueryOptions {
            env: &self.env,
            ..purrdf_sparql_eval::QueryOptions::EMPTY
        };
        let standpoint_predicates = self.standpoint_predicates.clone();
        let execution = &mut self.execution;
        // The engine is built HERE rather than held, because it is deliberately
        // `!Sync` — its plan cache is a `RefCell` — so it cannot live in a Python
        // object at all. Nothing is lost that this class exists to keep: the plan is
        // already parsed and admitted, and the execution holds it by `Arc`, so a run
        // does no parse, no admission and no cache probe. What a fresh engine gives
        // up is the join-order memo between runs, which is a plan-shaped hint rather
        // than the plan.
        //
        // `options` carries the SAME extension environment `execution`'s plan was
        // admitted under (see `PyPreparedQuery::env`); running under a different one
        // is what the engine's own `check_prepared_registries_unchanged` guard
        // inside `execute` refuses. `standpoint_predicates` is reapplied to
        // this fresh engine for the same reason — it is read at evaluation time, off
        // the engine, not admitted into the plan.
        let result = py.detach(move || {
            let mut engine = NativeSparqlEngine::new();
            if let Some((according_to, sharpens)) = standpoint_predicates {
                engine = engine
                    .with_standpoint_predicates(StandpointPredicates::new(according_to, sharpens));
            }
            engine
                .execute(execution, &*dataset, options, |outcome| {
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

/// Build a prepared query admitted against the environment `parser_options`,
/// `property_functions` and `aggregates` describe — the SAME environment
/// [`PyPreparedQuery::run`] must later evaluate under, which is why it is stored on
/// the returned object rather than dropped once admission succeeds. `store` is the
/// owning `Store`, held so `run` can re-read its CURRENT contents on every call
/// rather than a one-time snapshot (see [`PyPreparedQuery::store`]).
///
/// The environment is assembled by [`extension_env`] — the one place on this seam
/// where a caller's three optional configuration axes become the value a query text
/// is interpreted relative to, shared with `Store.query`, `MutableDataset`'s query
/// and update entries, and the governed doors. Admission and every run read the ONE
/// value this builds, so there is no second assembly for them to drift apart on.
///
/// # Errors
///
/// A `ValueError` if the environment cannot be derived, or if the query does not
/// parse or admit under it.
#[allow(
    clippy::too_many_arguments,
    reason = "every configuration axis `Store.prepare` accepts is named explicitly"
)]
pub(super) fn prepare(
    store: Py<PyStore>,
    engine: &NativeSparqlEngine,
    query: &str,
    parameters: &[String],
    parser_options: ParserOptions,
    property_functions: Option<&PropertyFunctionRegistry>,
    aggregates: Option<&AggregateRegistry>,
    standpoint_predicates: Option<(String, String)>,
) -> PyResult<PyPreparedQuery> {
    let borrowed: Vec<&str> = parameters.iter().map(String::as_str).collect();
    let env = extension_env(parser_options, property_functions, aggregates)?;
    let execution = engine
        .prepare_execution(
            query,
            None,
            &borrowed,
            purrdf_sparql_eval::QueryOptions {
                env: &env,
                ..purrdf_sparql_eval::QueryOptions::EMPTY
            },
        )
        .map_err(|e| PyValueError::new_err(format!("query preparation failed: {e}")))?;
    Ok(PyPreparedQuery {
        execution,
        store,
        env,
        standpoint_predicates,
    })
}
