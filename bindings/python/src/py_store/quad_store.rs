// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The copy-on-write quad store `Store` and `MutableDataset` both are.
//!
//! Both Python classes wrap one [`MutableDataset`] and answer SPARQL query, UPDATE,
//! the governed and entailment-aware variants, iteration, length, and the
//! `_store_capsule` validation protocol identically: each is a function of the COW
//! dataset alone. So those methods live once, on the [`PyQuadStore`] base class
//! both extend, and each subclass carries only what is genuinely its own (the
//! blank-scope policy of its `load`, and its own mutation surface).

use super::env::extension_env;
use super::presentation;
use std::sync::Arc;

use purrdf_core::ir::MutableDataset;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyCapsule, PyDict};

use super::io::{PyRdfFormat, PySerializeLoss, dump_quads_with_loss};
use super::query::{
    EngineConfig, GovernorArgs, PyCancellationToken, PyEntailmentQueryOutcome, PyQueryOutcome,
    PyUpdateOutcome, build_engine, build_relations, collect_relations, engine_parser_options,
    materialize_entailment_outcome, materialize_outcome, materialize_results,
    materialize_update_outcome, registry_over, run_governed,
};
use super::store::PyQuadIter;
use super::term::{PyVariable, extract_term, rdf_term_to_value, values_to_rdf_quad};
use crate::{
    ClosureRelations, DatasetMut, EntailmentClosure, GraphMatchValue, QueryEntailmentPlan,
    RdfDataset, RdfDatasetBuilder, RdfQuad, RdfTerm, SparqlRequest, TermValue,
    query_with_entailment_closure_governed,
};

/// The copy-on-write RDF 1.2 quad store `Store` and `MutableDataset` extend: the
/// one home of the query, UPDATE, iteration and validation-capsule surface they
/// share.
#[pyclass(name = "QuadStore", subclass)]
#[derive(Debug)]
pub struct PyQuadStore {
    pub(super) inner: MutableDataset,
}

#[pymethods]
impl PyQuadStore {
    /// Run a SPARQL query. Returns `QuerySolutions` (SELECT), `QueryTriples`
    /// (CONSTRUCT/DESCRIBE), or `QueryBoolean` (ASK). Optional `substitutions`
    /// is a `{Variable: term}` mapping applied natively (never string-spliced).
    ///
    /// Engine configuration (unset = engine defaults, see
    /// [`build_engine`](super::query::build_engine)): `extension_namespaces`
    /// enables the closed extension-function set under the caller's namespaces
    /// (OFF by default); `property_fn_namespaces` does the same for property-function
    /// PREFIX recognition; `standpoint_predicates` is the `(according_to,
    /// sharpens)` predicate table `heldIn` requires.
    ///
    /// `relations` / `relations_from_graph` / `path_relations` register host relations
    /// for THIS call (see [`collect_relations`](super::query::collect_relations)):
    /// `{iri: (subject_arity, object_arity, rows)}` supplies the table as Python
    /// data, `{iri: (head, subject_arity, object_arity)}` reads it out of this
    /// store's own default graph as an `rdf:List` of `rdf:List`s, and
    /// `{iri: (steps, min_hops, max_hops, max_paths_per_seed,
    /// max_expansions_per_invocation, mode)}` registers a PATH-WITNESS traversal over
    /// this store's own edges — callable as
    /// `?start <iri> ( ?end ?pathId ?len ?step ?node ?edge )`, one row per hop, with
    /// `?edge` the traversed statement as a first-class RDF 1.2 term. A registered IRI
    /// is recognized in predicate position EXACTLY, so no namespace declaration is
    /// needed to reach one.
    ///
    /// `aggregate_namespace` registers PurRDF's first-party statistical aggregate set
    /// (`MEDIAN`, `PERCENTILE`, `STDDEV`, `STDDEV_POP`, `VARIANCE`, `VAR_POP`, `MODE`,
    /// `FIRST`, `LAST`, `TOPK`) under that IRI, so the query text can call
    /// `AGG(<{NAMESPACE}NAME>, args…)` (see
    /// [`statistical_aggregates`](purrdf_validate::query::statistical_aggregates)). Unset (the default)
    /// leaves every one of the ten names an ordinary unregistered custom-aggregate IRI.
    #[pyo3(signature = (
        query,
        *,
        substitutions=None,
        extension_namespaces=None,
        property_fn_namespaces=None,
        standpoint_predicates=None,
        relations=None,
        relations_from_graph=None,
        path_relations=None,
        aggregate_namespace=None,
    ))]
    #[allow(
        clippy::too_many_arguments,
        reason = "each engine-configuration axis is named explicitly at the call site"
    )]
    fn query(
        &self,
        py: Python<'_>,
        query: &str,
        substitutions: Option<&Bound<'_, PyDict>>,
        extension_namespaces: Option<Vec<String>>,
        property_fn_namespaces: Option<Vec<String>>,
        standpoint_predicates: Option<(String, String)>,
        relations: Option<&Bound<'_, PyDict>>,
        relations_from_graph: Option<&Bound<'_, PyDict>>,
        path_relations: Option<&Bound<'_, PyDict>>,
        aggregate_namespace: Option<String>,
    ) -> PyResult<Py<PyAny>> {
        let subs = collect_substitutions(substitutions)?;
        // Python data is converted to owned `TermValue`s HERE, while the GIL is
        // held; nothing below re-enters the interpreter.
        let specs = collect_relations(relations, relations_from_graph, path_relations)?;
        let config = EngineConfig {
            extension_namespaces,
            property_fn_namespaces,
            standpoint_predicates,
        };
        let inner = &self.inner;
        // Snapshot + engine build + evaluation run detached (GIL released);
        // results are materialized into Python objects after reacquiring.
        let result = py.detach(move || {
            let dataset = inner
                .freeze()
                .map_err(|e| PyValueError::new_err(format!("store snapshot failed: {e}")))?;
            let registry = build_relations(specs, &dataset)?;
            let aggregates =
                purrdf_validate::query::statistical_aggregates(aggregate_namespace.as_deref());
            let parser_options = engine_parser_options(&config);
            let engine = build_engine(config);
            engine
                .query_with_options_view(
                    &*dataset,
                    SparqlRequest {
                        query,
                        base_iri: None,
                        substitutions: &subs,
                    },
                    purrdf_sparql_eval::QueryOptions::new().with_env(&extension_env(
                        parser_options,
                        registry.as_ref(),
                        aggregates.as_ref(),
                    )?),
                )
                .map_err(|e| presentation::value_error(format!("query evaluation error: {e}"), &e))
        })?;
        materialize_results(py, result)
    }

    /// Run a SPARQL query under caller-supplied execution governors, returning a
    /// `QueryOutcome` rather than the results directly.
    ///
    /// Every governor keyword is optional. An omitted dimension remains metered at an
    /// effectively unreachable ceiling; an explicit value replaces that ceiling. `fuel`
    /// bounds abstract execution steps, `deadline_ms` a wall-clock budget in milliseconds,
    /// `max_answers` the answer sequence (solution rows for SELECT, output statements for
    /// CONSTRUCT/DESCRIBE, nothing for ASK),
    /// `max_intermediate_cells` the largest intermediate bag in `rows * columns`,
    /// `max_scratch_bytes` the per-query scratch arena, and `max_remote_requests`
    /// federated requests. Every ceiling is **inclusive**: consumption equal to it is
    /// admitted, and zero is a valid ceiling that trips on the first charged unit of
    /// work. `cancel` takes a `CancellationToken` another thread can flip while this
    /// call runs. `no_ceiling=True` declines all ceilings and accounting, retaining
    /// deadline and cancellation signals; combining it with any resource cap raises
    /// `ValueError`, including a cap of zero.
    ///
    /// A tripped governor is an **outcome, not an exception** — see
    /// [`materialize_outcome`](super::query::materialize_outcome). The one stop cause
    /// that does raise is a `KeyboardInterrupt`: this call polls the interpreter's
    /// pending-signal flag while the GIL is released, so Ctrl-C stops the query instead
    /// of being noticed only once it has finished.
    ///
    /// `substitutions` / `extension_namespaces` / `property_fn_namespaces` /
    /// `standpoint_predicates` / `relations` / `relations_from_graph` / `path_relations` behave exactly
    /// as on [`query`](Self::query). A relation's rows are charged through the same
    /// governors as every other row source, so a ceiling bounds a call as it bounds a
    /// scan.
    #[pyo3(signature = (
        query,
        *,
        substitutions=None,
        extension_namespaces=None,
        property_fn_namespaces=None,
        standpoint_predicates=None,
        relations=None,
        relations_from_graph=None,
        path_relations=None,
        aggregate_namespace=None,
        fuel=None,
        deadline_ms=None,
        max_answers=None,
        max_intermediate_cells=None,
        max_scratch_bytes=None,
        max_remote_requests=None,
        no_ceiling=false,
        cancel=None,
    ))]
    #[allow(
        clippy::too_many_arguments,
        reason = "each governed dimension is named explicitly at the call site; a bag \
                  argument would make an unset ceiling and a misspelt one look alike"
    )]
    fn query_governed(
        &self,
        py: Python<'_>,
        query: &str,
        substitutions: Option<&Bound<'_, PyDict>>,
        extension_namespaces: Option<Vec<String>>,
        property_fn_namespaces: Option<Vec<String>>,
        standpoint_predicates: Option<(String, String)>,
        relations: Option<&Bound<'_, PyDict>>,
        relations_from_graph: Option<&Bound<'_, PyDict>>,
        path_relations: Option<&Bound<'_, PyDict>>,
        aggregate_namespace: Option<String>,
        fuel: Option<u64>,
        deadline_ms: Option<u64>,
        max_answers: Option<u64>,
        max_intermediate_cells: Option<u64>,
        max_scratch_bytes: Option<u64>,
        max_remote_requests: Option<u64>,
        no_ceiling: bool,
        cancel: Option<&PyCancellationToken>,
    ) -> PyResult<Py<PyQueryOutcome>> {
        let subs = collect_substitutions(substitutions)?;
        let specs = collect_relations(relations, relations_from_graph, path_relations)?;
        let config = EngineConfig {
            extension_namespaces,
            property_fn_namespaces,
            standpoint_predicates,
        };
        let args = GovernorArgs {
            fuel,
            deadline_ms,
            max_answers,
            max_intermediate_cells,
            max_scratch_bytes,
            max_remote_requests,
            no_ceiling,
        };
        let inner = &self.inner;
        // Snapshot + engine build + governed evaluation run detached (GIL released), so
        // the thread holding `cancel` keeps running while this one is in the engine.
        let outcome = run_governed(py, args, cancel, move |governors| {
            let dataset = inner
                .freeze()
                .map_err(|e| PyValueError::new_err(format!("store snapshot failed: {e}")))?;
            let registry = build_relations(specs, &dataset)?;
            let aggregates =
                purrdf_validate::query::statistical_aggregates(aggregate_namespace.as_deref());
            let parser_options = engine_parser_options(&config);
            let engine = build_engine(config);
            engine
                .query_governed(
                    &dataset,
                    SparqlRequest {
                        query,
                        base_iri: None,
                        substitutions: &subs,
                    },
                    purrdf_sparql_eval::QueryOptions::new().with_env(&extension_env(
                        parser_options,
                        registry.as_ref(),
                        aggregates.as_ref(),
                    )?),
                    governors,
                )
                .map_err(|e| presentation::value_error(format!("query evaluation error: {e}"), &e))
        })?;
        materialize_outcome(py, outcome)
    }

    /// Run a governed SPARQL query over a closure produced by the named entailment
    /// regime, carrying both the query outcome and the reasoning report.
    ///
    /// `aggregate_namespace` behaves exactly as on [`query_governed`](Self::query_governed):
    /// it registers PurRDF's first-party statistical aggregate set under that IRI for the
    /// closure query's PARSE and its evaluation, so `AGG(<{NAMESPACE}NAME>, args…)` reaches
    /// the entailment-aware lane exactly as it reaches the ordinary one. Unset (the default)
    /// leaves every one of the ten names an ordinary unregistered custom-aggregate IRI.
    ///
    /// `property_fn_namespaces` / `relations` / `relations_from_graph` / `path_relations` behave exactly as on
    /// [`query_governed`](Self::query_governed): a registered relation is reachable from the
    /// closure query exactly as it is from an ordinary one, so registering an IRI here and
    /// omitting it there cannot silently change which rows the SAME predicate position
    /// yields. `relations_from_graph` reads its table — and `path_relations` snapshots its
    /// edges — from the CLOSURE the regime materializes, which is the dataset the query is
    /// answered over, still scoped to the default graph exactly as [`query`](Self::query)
    /// is. A regime that DERIVES a quad under a step's predicate therefore widens the walk
    /// exactly as it widens a `p+` in the same query; reading the pre-entailment store
    /// instead is what used to make the two halves of one query disagree, silently and at
    /// success. See [`purrdf::ClosureRelations`] for the order and for the one
    /// `owl-direct` pairing it refuses.
    ///
    /// `imports` and `premise_iris` are the store's `owl:imports` table, spelled as
    /// [`purrdf.entail.certain_answers`](crate::py_entail) spells them: a sequence of
    /// `(ontology_iri, nquads_document)` pairs and the IRIs the store's data was read from.
    /// OWL 2 defines an ontology's imports closure to BE the ontology, so the closure the
    /// query runs over is materialized over the store merged with every document the table
    /// supplies. An `owl:imports` the table does not resolve, and the store does not already
    /// hold, raises `ValueError` naming it — never a closure of a smaller premise — and so
    /// does an entry the closure never reaches. Omitted, both are empty.
    ///
    /// `max_stored_facts` and `max_join_steps` are the closure's evaluation limits, exactly as
    /// `purrdf.entail.materialize` takes them, for the `rdf`, `rdfs`, `owl-rl` and `d`
    /// regimes; `None` keeps the default. A closure past one raises `ValueError` naming the
    /// limit, the numbers and this method's keyword; the governors price the evaluation over
    /// the closure and never become a limit on it.
    #[pyo3(signature = (
        query,
        entailment,
        *,
        program="",
        imports=None,
        premise_iris=None,
        max_stored_facts=None,
        max_join_steps=None,
        substitutions=None,
        extension_namespaces=None,
        property_fn_namespaces=None,
        standpoint_predicates=None,
        relations=None,
        relations_from_graph=None,
        path_relations=None,
        aggregate_namespace=None,
        fuel=None,
        deadline_ms=None,
        max_answers=None,
        max_intermediate_cells=None,
        max_scratch_bytes=None,
        max_remote_requests=None,
        no_ceiling=false,
        cancel=None,
    ))]
    #[allow(
        clippy::too_many_arguments,
        reason = "the regime plus each governed dimension is named explicitly at the host boundary"
    )]
    fn query_entailment_governed(
        &self,
        py: Python<'_>,
        query: &str,
        entailment: &str,
        program: &str,
        imports: Option<Vec<(String, String)>>,
        premise_iris: Option<Vec<String>>,
        max_stored_facts: Option<u64>,
        max_join_steps: Option<u64>,
        substitutions: Option<&Bound<'_, PyDict>>,
        extension_namespaces: Option<Vec<String>>,
        property_fn_namespaces: Option<Vec<String>>,
        standpoint_predicates: Option<(String, String)>,
        relations: Option<&Bound<'_, PyDict>>,
        relations_from_graph: Option<&Bound<'_, PyDict>>,
        path_relations: Option<&Bound<'_, PyDict>>,
        aggregate_namespace: Option<String>,
        fuel: Option<u64>,
        deadline_ms: Option<u64>,
        max_answers: Option<u64>,
        max_intermediate_cells: Option<u64>,
        max_scratch_bytes: Option<u64>,
        max_remote_requests: Option<u64>,
        no_ceiling: bool,
        cancel: Option<&PyCancellationToken>,
    ) -> PyResult<Py<PyEntailmentQueryOutcome>> {
        let subs = collect_substitutions(substitutions)?;
        let specs = collect_relations(relations, relations_from_graph, path_relations)?;
        let plan =
            QueryEntailmentPlan::parse(entailment, program).map_err(PyValueError::new_err)?;
        // The store's `owl:imports` table, parsed by the shared boundary before any closure
        // work. Absent is EMPTY — "imports nothing" — so a store that does import a document
        // is refused by name rather than closed without it.
        let imports = crate::py_entail::entailment_import_map(imports, premise_iris)?;
        let limits = purrdf_validate::MaterializeLimits {
            max_stored_facts,
            max_join_steps,
            host: purrdf_validate::RegimeHost::Python,
        };
        let args = GovernorArgs {
            fuel,
            deadline_ms,
            max_answers,
            max_intermediate_cells,
            max_scratch_bytes,
            max_remote_requests,
            no_ceiling,
        };
        let inner = &self.inner;
        let config = EngineConfig {
            extension_namespaces,
            property_fn_namespaces,
            standpoint_predicates,
        };
        let outcome = run_governed(py, args, cancel, move |governors| {
            let dataset = inner
                .freeze()
                .map_err(|e| PyValueError::new_err(format!("store snapshot failed: {e}")))?;
            // Two registries, and only the second one answers. This one is what the query
            // is PARSED against — a registered predicate becomes a call node only if its
            // registry was in scope when the query was read — and it is built over the
            // store's own snapshot. The one below is built over the CLOSURE the regime
            // materializes, which is the dataset the query is evaluated over, so a
            // `path_relations` traversal and a `relations_from_graph` table both read the
            // same data every other pattern in the query does. Before this pairing existed
            // they read the pre-closure store and returned a SHORT bag with no diagnostic.
            let registry = build_relations(specs.clone(), &dataset)?;
            let rebuild = |closure: &RdfDataset| {
                registry_over(specs.clone(), closure).map_err(purrdf_sparql_eval::EvalError::data)
            };
            let relations = if specs.is_empty() {
                ClosureRelations::NONE
            } else {
                ClosureRelations::rebuilt_by(&rebuild)
            };
            let parser_options = engine_parser_options(&config);
            let engine = build_engine(config);
            let aggregates =
                purrdf_validate::query::statistical_aggregates(aggregate_namespace.as_deref());
            query_with_entailment_closure_governed(
                &engine,
                &dataset,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &subs,
                },
                &EntailmentClosure::new(plan.entailment(), &imports)
                    .with_limits(limits.eval_options()),
                purrdf_sparql_eval::QueryOptions::new().with_env(&extension_env(
                    parser_options,
                    registry.as_ref(),
                    aggregates.as_ref(),
                )?),
                &relations,
                governors,
            )
            .map_err(|failure| match &failure {
                // Rendered by the shared boundary, so a passed evaluation limit names this
                // method's keyword rather than a Rust type a Python caller cannot reach.
                crate::ReasoningError::Entailment(error) => PyValueError::new_err(format!(
                    "entailment query failed: {}",
                    purrdf_validate::render_entail_error_in(
                        entailment,
                        error,
                        purrdf_validate::RegimeHost::Python,
                        purrdf_validate::RegimeService::Query,
                    )
                )),
                crate::ReasoningError::Query(diagnostic) => presentation::value_error(
                    format!("entailment query failed: {failure}"),
                    diagnostic,
                ),
                _ => PyValueError::new_err(format!("entailment query failed: {failure}")),
            })
        })?;
        materialize_entailment_outcome(py, outcome)
    }

    /// Run a SPARQL UPDATE against the store (COW-atomic: a failed update leaves the
    /// store unchanged). `extension_namespaces` / `property_fn_namespaces` /
    /// `standpoint_predicates` / `relations` / `relations_from_graph` / `path_relations` configure the
    /// engine exactly as on [`query`](Self::query); a registered relation is reachable
    /// from a `DELETE`/`INSERT … WHERE` clause, which is a triple-pattern context
    /// exactly as a query's is. A `relations_from_graph` table is read — and a
    /// `path_relations` traversal is snapshotted — from the PRE-update state, which is
    /// the same state the `WHERE` clause matches.
    #[pyo3(signature = (
        update,
        *,
        extension_namespaces=None,
        property_fn_namespaces=None,
        standpoint_predicates=None,
        relations=None,
        relations_from_graph=None,
        path_relations=None,
        aggregate_namespace=None,
    ))]
    #[allow(
        clippy::too_many_arguments,
        reason = "each engine-configuration axis is named explicitly at the call site"
    )]
    fn update(
        &mut self,
        py: Python<'_>,
        update: &str,
        extension_namespaces: Option<Vec<String>>,
        property_fn_namespaces: Option<Vec<String>>,
        standpoint_predicates: Option<(String, String)>,
        relations: Option<&Bound<'_, PyDict>>,
        relations_from_graph: Option<&Bound<'_, PyDict>>,
        path_relations: Option<&Bound<'_, PyDict>>,
        aggregate_namespace: Option<String>,
    ) -> PyResult<()> {
        let specs = collect_relations(relations, relations_from_graph, path_relations)?;
        let config = EngineConfig {
            extension_namespaces,
            property_fn_namespaces,
            standpoint_predicates,
        };
        // Snapshot + evaluation run detached (GIL released); the fresh frozen
        // base is adopted after reacquiring.
        let inner = &self.inner;
        let dataset = py.detach(move || {
            let mut dataset = inner
                .freeze()
                .map_err(|e| PyValueError::new_err(format!("store snapshot failed: {e}")))?;
            let registry = build_relations(specs, &dataset)?;
            let aggregates =
                purrdf_validate::query::statistical_aggregates(aggregate_namespace.as_deref());
            let parser_options = engine_parser_options(&config);
            let engine = build_engine(config);
            engine
                .update_with_options(
                    &mut dataset,
                    SparqlRequest {
                        query: update,
                        base_iri: None,
                        substitutions: &[],
                    },
                    purrdf_sparql_eval::QueryOptions::new().with_env(&extension_env(
                        parser_options,
                        registry.as_ref(),
                        aggregates.as_ref(),
                    )?),
                )
                .map_err(|e| {
                    presentation::value_error(format!("update evaluation error: {e}"), &e)
                })?;
            Ok::<_, PyErr>(dataset)
        })?;
        // The UPDATE produced a fresh frozen base; adopt it as the new COW base.
        self.inner = MutableDataset::new(dataset);
        Ok(())
    }

    /// Run a SPARQL UPDATE under caller-supplied execution governors, returning an
    /// `UpdateOutcome` rather than `None`.
    ///
    /// The governor keywords are those of
    /// [`query_governed`](Self::query_governed) minus `max_answers`, which bounds an
    /// answer sequence an UPDATE does not have. A request's size is bounded by the
    /// ceilings on the work that computes it. The engine-configuration keywords —
    /// including `relations` / `relations_from_graph` / `path_relations` — are those of
    /// [`update`](Self::update).
    ///
    /// **A tripped request applies nothing.** Not "not all of it": the store is left
    /// exactly as it was found, whichever operation the governor stopped and however much
    /// work the earlier operations of the same request had already done. As on the query
    /// path the trip is an outcome rather than an exception, and a `KeyboardInterrupt`
    /// raises.
    #[pyo3(signature = (
        update,
        *,
        extension_namespaces=None,
        property_fn_namespaces=None,
        standpoint_predicates=None,
        relations=None,
        relations_from_graph=None,
        path_relations=None,
        aggregate_namespace=None,
        fuel=None,
        deadline_ms=None,
        max_intermediate_cells=None,
        max_scratch_bytes=None,
        max_remote_requests=None,
        no_ceiling=false,
        cancel=None,
    ))]
    #[allow(
        clippy::too_many_arguments,
        reason = "each governed dimension is named explicitly at the call site; a bag \
                  argument would make an unset ceiling and a misspelt one look alike"
    )]
    fn update_governed(
        &mut self,
        py: Python<'_>,
        update: &str,
        extension_namespaces: Option<Vec<String>>,
        property_fn_namespaces: Option<Vec<String>>,
        standpoint_predicates: Option<(String, String)>,
        relations: Option<&Bound<'_, PyDict>>,
        relations_from_graph: Option<&Bound<'_, PyDict>>,
        path_relations: Option<&Bound<'_, PyDict>>,
        aggregate_namespace: Option<String>,
        fuel: Option<u64>,
        deadline_ms: Option<u64>,
        max_intermediate_cells: Option<u64>,
        max_scratch_bytes: Option<u64>,
        max_remote_requests: Option<u64>,
        no_ceiling: bool,
        cancel: Option<&PyCancellationToken>,
    ) -> PyResult<Py<PyUpdateOutcome>> {
        let specs = collect_relations(relations, relations_from_graph, path_relations)?;
        let config = EngineConfig {
            extension_namespaces,
            property_fn_namespaces,
            standpoint_predicates,
        };
        let args = GovernorArgs {
            fuel,
            deadline_ms,
            max_answers: None,
            max_intermediate_cells,
            max_scratch_bytes,
            max_remote_requests,
            no_ceiling,
        };
        let inner = &self.inner;
        // Snapshot + governed evaluation run detached (GIL released).
        let (outcome, dataset) = run_governed(py, args, cancel, move |governors| {
            let mut dataset = inner
                .freeze()
                .map_err(|e| PyValueError::new_err(format!("store snapshot failed: {e}")))?;
            let registry = build_relations(specs, &dataset)?;
            let aggregates =
                purrdf_validate::query::statistical_aggregates(aggregate_namespace.as_deref());
            let parser_options = engine_parser_options(&config);
            let outcome = build_engine(config)
                .update_governed(
                    &mut dataset,
                    SparqlRequest {
                        query: update,
                        base_iri: None,
                        substitutions: &[],
                    },
                    purrdf_sparql_eval::QueryOptions::new().with_env(&extension_env(
                        parser_options,
                        registry.as_ref(),
                        aggregates.as_ref(),
                    )?),
                    governors,
                )
                .map_err(|e| {
                    presentation::value_error(format!("update evaluation error: {e}"), &e)
                })?;
            Ok((outcome, dataset))
        })?;
        // The engine publishes into its own `Arc` only on the applied path, so adopting
        // the returned base on a trip would adopt a base nothing was written to. Adopt it
        // only when the request applied, and the tripped path leaves this store's COW
        // base untouched.
        if outcome.is_applied() {
            self.inner = MutableDataset::new(dataset);
        }
        materialize_update_outcome(py, &outcome)
    }

    fn __len__(&self) -> usize {
        self.inner
            .quads_for_pattern(None, None, None, GraphMatchValue::Any)
            .len()
    }

    /// Iterate the store's quads (a snapshot taken at iteration time).
    fn __iter__(&self, py: Python<'_>) -> PyResult<Py<PyQuadIter>> {
        let quads = self
            .inner
            .quads_for_pattern(None, None, None, GraphMatchValue::Any)
            .iter()
            .map(values_to_rdf_quad)
            .collect();
        Py::new(py, PyQuadIter { quads, pos: 0 })
    }

    /// Internal protocol: a capsule exposing a frozen `Arc<RdfDataset>` snapshot of
    /// this store by address, consumed by `purrdf_shapes.Shapes.validate_store` so the
    /// SHACL engine validates this store natively with no N-Triples round-trip. Do
    /// not call from Python directly. The capsule name and pointee type match exactly
    /// what `purrdf_shapes` consumes from `purrdf_validate.ValidationStore`.
    ///
    /// The capsule's destructor owns the `Arc<RdfDataset>`, so the dataset is kept
    /// alive for the capsule's entire lifetime. Because the snapshot is an immutable
    /// frozen `Arc` taken now, a later `add`/`remove`/`update` on this `Store` leaves
    /// the snapshot a consumer already holds untouched (snapshot-vs-mutation aliasing
    /// safety).
    /// The leading underscore belongs to the PYTHON name, not the Rust one: it is
    /// the cross-package protocol `purrdf_shapes` calls by string
    /// (`data.call_method0("_store_capsule")`), and Python spells "internal" with
    /// an underscore. Naming the Rust item `_store_capsule` too would make it an
    /// underscore-prefixed item that the crate then uses — the convention for
    /// "deliberately unused" — so the Python spelling is carried by
    /// `#[pyo3(name = ...)]` and the Rust item keeps an ordinary name.
    #[pyo3(name = "_store_capsule")]
    fn store_capsule<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyCapsule>> {
        let py = slf.py();
        let guard = slf.borrow();
        let inner = &guard.inner;
        // The COW freeze can be a real copy on a mutated store — run it detached.
        let snapshot: Arc<RdfDataset> = py.detach(|| {
            inner
                .freeze()
                .map_err(|e| PyValueError::new_err(format!("store snapshot failed: {e}")))
        })?;
        drop(guard);
        // Heap-box the Arc so its address is stable; the destructor reclaims the box
        // (dropping the held Arc) when the capsule is collected.
        let boxed: Box<Arc<RdfDataset>> = Box::new(snapshot);
        let addr = (&raw const *boxed) as usize;
        let keepalive = boxed;
        // SAFETY: `addr` is the address of the `Arc<RdfDataset>` owned by `keepalive`,
        // moved into the destructor closure; it stays live and at a stable address for
        // the capsule's entire lifetime. The consumer reads the `Arc<RdfDataset>` at
        // that address (cloning it to extend the lifetime as needed).
        PyCapsule::new_with_value_and_destructor(
            py,
            addr,
            c"purrdf-validation-dataset",
            move |_addr, _ctx| drop(keepalive),
        )
    }

    /// Dump the WHOLE store in `format`, with the realized loss of doing so attached.
    ///
    /// The counting twin of `dump` (on `Store` and `MutableDataset`): same bytes, plus the three
    /// independent loss counts a `SerializeLoss` carries. `dump(format=RdfFormat.TURTLE)`
    /// on a store holding named graphs returns a well-formed document with every
    /// graph-scoped statement missing and no signal at all; this is the entry point that
    /// says how many. Mirrors the C ABI's `purrdf_serialize` count out-params and the
    /// wasm `Dataset.serializeWithLoss`, so one serialization reports the same three
    /// numbers on every host.
    ///
    /// There is deliberately no `from_graph` and no JSON-LD configuration here: a graph
    /// SELECTION would make the named-graph count meaningless (the caller would already
    /// have chosen what to keep), and the JSON-LD family is dataset-capable and
    /// star-capable, so its loss is zero by construction. Use `dump` for either.
    #[pyo3(signature = (format))]
    fn dump_with_loss(&self, py: Python<'_>, format: PyRdfFormat) -> PyResult<PySerializeLoss> {
        let native = format.to_native();
        py.detach(|| {
            dump_quads_with_loss(&self.collect_all_quads(), native)
                .map_err(|e| PyValueError::new_err(format!("dump error: {e}")))
        })
    }
}

impl PyQuadStore {
    /// An empty store: an empty frozen base under an empty copy-on-write delta.
    ///
    /// # Errors
    ///
    /// `ValueError` if the empty base cannot be frozen.
    pub(super) fn empty() -> PyResult<Self> {
        let base = RdfDatasetBuilder::new()
            .freeze()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: MutableDataset::new(base),
        })
    }

    /// Freeze this store's CURRENT contents into an immutable `Arc<RdfDataset>`
    /// snapshot.
    ///
    /// Called fresh by [`PyPreparedQuery::run`](super::prepared::PyPreparedQuery::run)
    /// on every run, rather than once at prepare time, so a prepared query's answer
    /// reflects the store as it stands right now — the same freeze `query` and
    /// `prepare` already do, exposed here so a held `Py<PyStore>` can repeat it. The
    /// freeze itself (a real copy on a mutated store) runs with the GIL released;
    /// only the borrow that reaches `self.inner` needs it held.
    ///
    /// # Errors
    ///
    /// `ValueError` if the store cannot be frozen.
    pub(crate) fn freeze_snapshot(&self, py: Python<'_>) -> PyResult<Arc<RdfDataset>> {
        let inner = &self.inner;
        py.detach(|| {
            inner
                .freeze()
                .map_err(|e| PyValueError::new_err(format!("store snapshot failed: {e}")))
        })
    }

    /// An immutable snapshot of this store's copy-on-write DELTA — the base, the
    /// rows added on top of it, and the rows suppressed from it, read through one
    /// view rather than copied.
    ///
    /// The change-path counterpart of [`store_capsule`](Self::store_capsule), and
    /// deliberately not a capsule: that protocol hands out a frozen
    /// `Arc<RdfDataset>` with the change already flattened away, which is the one
    /// thing an incremental validation needs. This is Rust-side and `pub(crate)`
    /// because the consumer (`crate::shacl`) is in this crate, and because a
    /// `DeltaDatasetView` names this store's own interners — there is no honest way
    /// to hand one across a language boundary.
    ///
    /// # Errors
    ///
    /// `ValueError` when the snapshot exceeds the view's retention limits.
    pub(crate) fn change_snapshot(&self) -> PyResult<purrdf_core::ir::DeltaDatasetView> {
        self.inner
            .snapshot_view()
            .map_err(|e| PyValueError::new_err(format!("store change snapshot failed: {e}")))
    }

    /// Every quad in the store, graph names intact (for the dataset-format dump path).
    pub(super) fn collect_all_quads(&self) -> Vec<RdfQuad> {
        self.inner
            .quads_for_pattern(None, None, None, GraphMatchValue::Any)
            .iter()
            .map(values_to_rdf_quad)
            .collect()
    }

    /// The quads of ONE graph, re-homed to the default graph (so a single-graph dump
    /// serializes as triples). `graph` is the selected graph term, or `None` for the
    /// default graph — the `Store.dump(from_graph=…)` projection.
    pub(super) fn collect_graph_quads(&self, graph: Option<&RdfTerm>) -> Vec<RdfQuad> {
        self.inner
            .quads_for_pattern(None, None, None, GraphMatchValue::Any)
            .iter()
            .filter_map(|values| {
                let quad = values_to_rdf_quad(values);
                (quad.graph_name.as_ref() == graph).then(|| {
                    let mut projected = quad;
                    projected.graph_name = None;
                    projected
                })
            })
            .collect()
    }
}

/// Collect the `{Variable: term}` substitution dict into the native
/// `(name, TermValue)` pre-binding slice the SPARQL request carries.
fn collect_substitutions(
    substitutions: Option<&Bound<'_, PyDict>>,
) -> PyResult<Vec<(String, TermValue)>> {
    let Some(subs) = substitutions else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(subs.len());
    for (key, value) in subs.iter() {
        let name = key
            .cast::<PyVariable>()
            .map_err(|_| PyTypeError::new_err("substitution keys must be Variable"))?
            .get()
            .inner
            .clone();
        out.push((name, rdf_term_to_value(&extract_term(&value)?)));
    }
    Ok(out)
}
