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
use super::query::{RelationSpec, build_aggregates, build_relations, materialize_results};
use super::term::{extract_term, rdf_term_to_value};
use crate::RdfDataset;
use crate::attestation::Attestation;

/// The relation SOURCES a handle must re-derive its registry from on every run,
/// because at least one of them reads the store's own graph.
///
/// # Why the configuration is kept and the built table is not
///
/// `Store.prepare`'s `relations_from_graph` and `path_relations` are read out of the
/// dataset they are built against — `MemoryRelation::from_graph` walks an `rdf:List`
/// of `rdf:List`s, `PathGraph::from_dataset` walks the edges — so a table built at
/// PREPARE time describes the store as it was then. The surrounding query does not:
/// [`PyPreparedQuery::run`] re-reads and re-freezes the owning store on every call.
/// A handle that kept the built table would therefore assemble one answer out of two
/// different points in time, silently, and a fixpoint — the very use case
/// `Store.prepare` exists for — mutates its store every round, so it would hit that
/// on every round after the first.
///
/// Refusing such a run was the other candidate and is not available: it would make
/// the fixpoint plus `relations_from_graph` combination unusable, which is satisfying
/// the feature by declining it. So the derived tables are rebuilt from the RUN's
/// dataset, through the same [`build_relations`] every other door on this surface
/// uses — there is no second mechanism for reading a relation out of a graph.
///
/// # Why the plan is re-admitted and not merely re-pointed
///
/// The rebuilt registry is a DIFFERENT registry to the engine, and correctly so — so
/// swapping it in under the plan the old one admitted is not available, for two
/// independent reasons.
///
/// The first is instance identity. A registry's identity, as
/// `NativeSparqlEngine::execute`'s `check_prepared_registries_unchanged` guard
/// compares it, leads with
/// `purrdf_sparql_eval::PropertyFunctionRegistry::instance_id`, and building a
/// registry mints a fresh one. `Clone` inherits that id rather than re-minting it —
/// which is what makes the environment's stored registry the same registry for
/// identity purposes as the one it was built from — but a clone carries the OLD
/// relation, which is the whole thing that has to change. A rebuild is a new
/// registry, so the guard refuses it even when the store has not moved at all.
///
/// The second is that the identity's content half moves with the DATA. It digests
/// every relation's declared `rows_per_invocation`, and for both graph-derived kinds
/// that bound is a function of the dataset: a `MemoryRelation` declares its own row
/// count, and a path relation declares a bound over the traversal graph's node and
/// edge counts. The same declared bound is also what the engine's feasibility
/// ordering breaks ties on, so a mutation can legitimately reorder the plan — the
/// admitted plan is not even guaranteed to be the plan the new declarations order to.
///
/// The honest answer to both is to re-admit: `prepare_execution` parses and admits
/// the same text, with the same parameters, against the rebuilt environment, and the
/// run then evaluates under an environment its plan really was admitted under. That
/// costs a parse on the rounds that rebuild — next to the graph walk the rebuild
/// itself performs, and paid only by a handle that configured a graph-derived
/// relation at all.
pub(super) struct GraphDerivedRelations {
    /// The query text, re-admitted verbatim: the same text under a new environment,
    /// never a re-rendered or re-spliced one.
    query: String,
    /// The declared parameter names, in declaration order — the same list
    /// `Store.prepare` was given, so a re-admitted execution has the same slots under
    /// the same names and `PreparedQuery.parameters` cannot shift under a caller.
    parameters: Vec<String>,
    /// Every relation `Store.prepare` was given, in registration order — the
    /// graph-derived ones AND the caller's inline tables.
    ///
    /// All of them, because a registry is one value: the plan is admitted against the
    /// whole table of registered IRIs, so rebuilding a subset would produce a registry
    /// missing the rest. An inline [`RelationSpec::Rows`] table is nonetheless a
    /// caller-supplied CONSTANT — re-registering it re-reads the caller's own rows and
    /// cannot change what it answers, which is what makes including it a rebuild of
    /// the registry rather than a re-derivation of the constant.
    specs: Vec<(String, RelationSpec, Attestation)>,
    /// The caller's `extension_namespaces` / `property_fn_namespaces`, as parse
    /// configuration. A constant; carried so the rebuilt environment declares exactly
    /// what the prepare-time one did.
    parser_options: ParserOptions,
    /// The caller's `aggregate_namespace`. A constant, and the custom-aggregate
    /// registry it builds is a pure function of it, so it is rebuilt from the string
    /// rather than held as a registry.
    aggregate_namespace: Option<String>,
}

impl GraphDerivedRelations {
    /// The sources for a `Store.prepare` call, or `None` when nothing it registered
    /// reads the store's graph and the prepare-time environment therefore stays valid
    /// for every run.
    ///
    /// `None` is the common case and the cheap one: a handle with no relations at all,
    /// or with only inline tables, rebuilds nothing and re-admits nothing.
    pub(super) fn for_specs(
        specs: &[(String, RelationSpec, Attestation)],
        query: &str,
        parameters: &[String],
        parser_options: &ParserOptions,
        aggregate_namespace: Option<&String>,
    ) -> Option<Self> {
        if !specs
            .iter()
            .any(|(_, spec, _)| spec.reads_the_store_graph())
        {
            return None;
        }
        Some(Self {
            query: query.to_owned(),
            parameters: parameters.to_vec(),
            specs: specs.to_vec(),
            parser_options: parser_options.clone(),
            aggregate_namespace: aggregate_namespace.cloned(),
        })
    }

    /// Rebuild the registry against `dataset` and re-admit the plan under it,
    /// returning the pair a run then evaluates with.
    ///
    /// `engine` is the run's own engine, standpoint predicates already applied, so the
    /// admission here reads the query text exactly as the engine about to evaluate it
    /// does.
    ///
    /// # Errors
    ///
    /// A `ValueError` if a relation can no longer be built from the current dataset —
    /// a table head the store no longer names, a list a mutation tore — or if the
    /// re-admission fails. Both are raised rather than answered around: a relation the
    /// store cannot supply is not an empty relation.
    fn readmit(
        &self,
        engine: &NativeSparqlEngine,
        dataset: &RdfDataset,
    ) -> PyResult<(PreparedExecution, ExtensionEnv)> {
        let registry = build_relations(self.specs.clone(), dataset)?;
        let aggregates = build_aggregates(self.aggregate_namespace.clone());
        let env = extension_env(
            self.parser_options.clone(),
            registry.as_ref(),
            aggregates.as_ref(),
        )?;
        let borrowed: Vec<&str> = self.parameters.iter().map(String::as_str).collect();
        let execution = engine
            .prepare_execution(
                &self.query,
                None,
                &borrowed,
                purrdf_sparql_eval::QueryOptions::new().with_env(&env),
            )
            .map_err(|e| PyValueError::new_err(format!("query preparation failed: {e}")))?;
        Ok((execution, env))
    }
}

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
    /// and both the admission and the run that evaluates under it read THAT value.
    /// Two independent derivations would be two answers to "what does this query text
    /// mean", which is the shape of the defect the environment type exists to remove.
    ///
    /// When [`Self::graph_derived`] is `Some`, this field and [`Self::execution`] are
    /// replaced TOGETHER, by one call to
    /// [`GraphDerivedRelations::readmit`](GraphDerivedRelations::readmit), at the top
    /// of the run that needs them — so the pairing above is what is maintained across
    /// the rebuild, rather than something only the prepare-time pair enjoyed.
    env: ExtensionEnv,
    /// The relation sources to re-derive from the RUN's dataset, when at least one of
    /// them reads the store's own graph — `None` otherwise, which is the common case.
    ///
    /// See [`GraphDerivedRelations`] for why a graph-derived relation cannot be built
    /// once and kept, and why rebuilding it means re-admitting the plan rather than
    /// swapping a table underneath one.
    graph_derived: Option<GraphDerivedRelations>,
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
    ///
    /// A relation registered from the store's own graph (`relations_from_graph`,
    /// `path_relations`) is REBUILT here, from this run's dataset, and the plan is
    /// re-admitted under the rebuilt registry — see [`GraphDerivedRelations`]. Without
    /// that, the relation would answer about the store as it was at `prepare` time
    /// while the rest of the query answered about the store as it is now.
    #[pyo3(signature = (**bindings))]
    fn run(&mut self, py: Python<'_>, bindings: Option<&Bound<'_, PyDict>>) -> PyResult<Py<PyAny>> {
        // The keyword values are Python objects, so they are read HERE, under the GIL.
        // They are read into owned terms BEFORE anything else this call does, because
        // a rebuild below may replace the execution the slots live in — a binding
        // written into an execution that is then discarded would silently not be a
        // binding at all.
        let mut bound: Vec<(String, TermValue)> = Vec::new();
        if let Some(bindings) = bindings {
            for (name, value) in bindings {
                let name: String = name.extract()?;
                let value: TermValue = rdf_term_to_value(&extract_term(&value)?);
                bound.push((name, value));
            }
        }

        // Re-read the owning store FRESH for this run (see `PyPreparedQuery::store`)
        // rather than a snapshot taken once at prepare time, so a fixpoint's mutation
        // between rounds is visible to the next round's run. Frozen while the GIL is
        // held (a cheap borrow), then the heavy freeze work itself runs detached
        // inside `freeze_snapshot`.
        let dataset = {
            let store = self.store.bind(py).borrow();
            store.freeze_snapshot(py)?
        };

        // Rebuild, bind and evaluate run detached (GIL released): the rebuild walks
        // the dataset and re-admits the plan, which is Rust work over owned data
        // exactly as the evaluation is, and a relation on this surface is data rather
        // than a Python callable precisely so nothing here can re-enter the
        // interpreter.
        let result = py.detach(move || {
            // The engine is built HERE rather than held, because it is deliberately
            // `!Sync` — its plan cache is a `RefCell` — so it cannot live in a Python
            // object at all. What a fresh engine gives up is the join-order memo
            // between runs, which is a plan-shaped hint rather than the plan.
            //
            // `standpoint_predicates` is reapplied to it because that axis is read at
            // evaluation time, off the engine, not admitted into the plan — and the
            // re-admission below is handed this same engine, so a re-admitted plan is
            // read under the configuration the evaluation runs under.
            let mut engine = NativeSparqlEngine::new();
            if let Some((according_to, sharpens)) = self.standpoint_predicates.clone() {
                engine = engine
                    .with_standpoint_predicates(StandpointPredicates::new(according_to, sharpens));
            }
            // A relation read out of the store's graph is rebuilt against THIS run's
            // dataset and the plan re-admitted under it, so the whole answer comes
            // from one point in time. Both fields are replaced together, and only on
            // success: a rebuild that fails leaves the handle exactly as it was.
            let readmitted = match self.graph_derived.as_ref() {
                Some(sources) => Some(sources.readmit(&engine, &dataset)?),
                None => None,
            };
            if let Some((execution, env)) = readmitted {
                self.execution = execution;
                self.env = env;
            }

            // Every slot goes back to unbound before this call's own keywords are
            // applied, so a parameter an EARLIER call bound but this call does not
            // (re)mention is `None` again here — not a stale value left over from that
            // earlier call. That is what makes a keyword-argument call read as TOTAL:
            // `run(this=X)` means "these are the bindings", never "these, plus whatever
            // a prior call left behind". See `PreparedExecution::unbind_all`'s own doc
            // for why this is the engine's job rather than a second bookkeeping layer
            // here. A re-admitted execution starts unbound anyway; this is what makes
            // the two paths reach evaluation in the same state.
            self.execution.unbind_all();
            for (name, value) in bound {
                self.execution
                    .bind_named(&name, value)
                    .map_err(|e| PyValueError::new_err(e.to_string()))?;
            }
            // No unbound check here: a slot this call left unmentioned is `None` after
            // `unbind_all` above, and `NativeSparqlEngine::execute` below already
            // refuses any still-`None` slot with its own diagnostic — the one place
            // that decides what "still unbound" means and what it says.

            // The environment is the one THIS execution's plan was admitted under —
            // the prepare-time value, or the value the rebuild just replaced it with
            // alongside the execution itself. Running under a different one is what
            // the engine's own `check_prepared_registries_unchanged` guard inside
            // `execute` refuses, and the pairing is what keeps that refusal a guard
            // against a caller's mistake rather than a trap this surface walks into.
            let options = purrdf_sparql_eval::QueryOptions::new().with_env(&self.env);
            engine
                .execute(&mut self.execution, &*dataset, options, |outcome| {
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
/// and update entries, and the governed doors. Admission and the runs that evaluate
/// under it read the ONE value this builds, so there is no second assembly for them
/// to drift apart on — and where `graph_derived` is `Some`, the rebuild that replaces
/// that value replaces the admitted execution with it, so the pairing survives.
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
    graph_derived: Option<GraphDerivedRelations>,
) -> PyResult<PyPreparedQuery> {
    let borrowed: Vec<&str> = parameters.iter().map(String::as_str).collect();
    let env = extension_env(parser_options, property_functions, aggregates)?;
    let execution = engine
        .prepare_execution(
            query,
            None,
            &borrowed,
            purrdf_sparql_eval::QueryOptions::new().with_env(&env),
        )
        .map_err(|e| PyValueError::new_err(format!("query preparation failed: {e}")))?;
    Ok(PyPreparedQuery {
        execution,
        store,
        env,
        graph_derived,
        standpoint_predicates,
    })
}
