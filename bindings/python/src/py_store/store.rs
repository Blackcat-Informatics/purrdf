// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The mutable quad-store surface for the `purrdf` Python extension: the
//! SPARQL-capable `Store`, the canonicalization-capable `Dataset`, and the
//! `QuadIter` snapshot iterator they share.
//!
//! # Native backing
//!
//! `Store` wraps a copy-on-write [`MutableDataset`] over the oxigraph-free
//! `purrdf-core` IR — never `oxigraph::store::Store`. Mutation (`add` / `remove`
//! / `load`) edits the COW delta; `query` freezes a snapshot and runs the native
//! `NativeSparqlEngine`; `update` runs the engine's COW-atomic UPDATE. The
//! `_store_capsule` hands `purrdf_shapes` / `purrdf_validate` a stable
//! `Arc<RdfDataset>` snapshot under the `c"purrdf-validation-dataset"` capsule name.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use purrdf_core::ir::{MutableDataset, QuadValues};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyCapsule, PyDict};

use super::canon::PyCanonicalizationAlgorithm;
use super::io::{
    PyRdfFormat, PySerializeLoss, dataset_from_quads_verbatim, dump_quads_with_loss, parse_quads,
    read_input,
};
use super::query::{
    EngineConfig, GovernorArgs, PyCancellationToken, PyEntailmentQueryOutcome, PyQueryOutcome,
    PyUpdateOutcome, build_aggregates, build_engine, build_relations, collect_relations,
    materialize_entailment_outcome, materialize_outcome, materialize_results,
    materialize_update_outcome, run_governed,
};
use super::term::{
    PyQuad, PyVariable, extract_graph_name, extract_term, rdf_term_to_value,
    rdf_term_to_value_scoped,
};
use crate::py_jsonld::{PyCompiledJsonLdContext, options_from_inputs};
use crate::{
    BlankScope, DatasetMut, GraphMatchValue, QueryEntailmentPlan, RdfDataset, RdfDatasetBuilder,
    RdfLiteral, RdfQuad, RdfTerm, RdfTriple, SerializeGraph, SparqlRequest, TermValue,
    query_with_entailment_governed, serialize_dataset, serialize_dataset_with_jsonld_options,
};

/// An in-memory RDF 1.2 quad store with SPARQL. Mirrors the oxigraph Python `Store`.
#[pyclass(name = "Store")]
#[derive(Debug)]
pub struct PyStore {
    inner: MutableDataset,
    /// Monotonic per-load counter that isolates blank-node label scopes across
    /// separate [`load`](PyStore::load) calls (see [`load`](PyStore::load) for why).
    next_load_scope: AtomicU64,
}

#[pymethods]
impl PyStore {
    #[new]
    fn new() -> PyResult<Self> {
        Ok(Self {
            inner: empty_mutable()?,
            next_load_scope: AtomicU64::new(1),
        })
    }

    /// Load RDF into the store. Either `input` (bytes/str data) or the keyword
    /// `path` (a file to read) must be given, together with `format`.
    #[pyo3(signature = (input=None, format=None, *, path=None, base=None))]
    fn load(
        &mut self,
        py: Python<'_>,
        input: Option<&Bound<'_, PyAny>>,
        format: Option<PyRdfFormat>,
        path: Option<String>,
        base: Option<String>,
    ) -> PyResult<()> {
        let format = format.ok_or_else(|| PyValueError::new_err("load: format is required"))?;
        let data = read_input(input, path)?;
        // Parse natively into the flat quad stream, then insert into the COW set.
        //
        // Blank-node labels in a serialized document are document-local: two distinct
        // documents may reuse the same label (`_:b0`) for *different* nodes, and the same
        // store loaded from many files must keep those distinct. oxigraph's prior per-load
        // blank scope gave each load call a fresh blank scope; the native codec preserves
        // labels verbatim, so we restore that isolation by tagging every parsed blank
        // node's label with a per-load-call-unique `BlankScope` before insertion.
        // `parse` / `parse_quads` keep labels verbatim — that path round-trips a single
        // document, where verbatim labels are correct and canonicalization needs them.
        let scope = BlankScope(self.next_load_scope() as u32);
        let inner = &mut self.inner;
        // Parse + insert run detached (GIL released); the closure only touches
        // plain Rust data.
        py.detach(move || {
            let base_ref = base.as_deref();
            for quad in parse_quads(&data, format.to_native(), base_ref)
                .map_err(|e| PyValueError::new_err(format!("load error: {e}")))?
            {
                inner.insert(rdf_quad_to_values_scoped(&quad, scope));
            }
            Ok(())
        })
    }

    /// Alias of [`load`] — oxigraph's bulk loader is a throughput optimization,
    /// not a different semantics, so the in-memory store path is identical.
    #[pyo3(signature = (input=None, format=None, *, path=None, base=None))]
    fn bulk_load(
        &mut self,
        py: Python<'_>,
        input: Option<&Bound<'_, PyAny>>,
        format: Option<PyRdfFormat>,
        path: Option<String>,
        base: Option<String>,
    ) -> PyResult<()> {
        self.load(py, input, format, path, base)
    }

    /// Add a single quad.
    fn add(&mut self, quad: &PyQuad) -> PyResult<()> {
        self.inner.insert(rdf_quad_to_values(&quad.inner));
        Ok(())
    }

    /// Remove a single quad. No-op if the quad is absent (matches the RDFLib
    /// `Graph.remove` contract, which silently ignores misses).
    fn remove(&mut self, quad: &PyQuad) -> PyResult<()> {
        self.inner.remove(&rdf_quad_to_values(&quad.inner));
        Ok(())
    }

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
    /// `aggregate_namespace` registers purrdf's first-party statistical aggregate set
    /// (`MEDIAN`, `PERCENTILE`, `STDDEV`, `STDDEV_POP`, `VARIANCE`, `VAR_POP`, `MODE`,
    /// `FIRST`, `LAST`, `TOPK`) under that IRI, so the query text can call
    /// `AGG(<{NAMESPACE}NAME>, args…)` (see
    /// [`build_aggregates`](super::query::build_aggregates)). Unset (the default)
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
            let aggregates = build_aggregates(aggregate_namespace);
            let engine = build_engine(config);
            engine
                .query_with_options_view(
                    &*dataset,
                    SparqlRequest {
                        query,
                        base_iri: None,
                        substitutions: &subs,
                    },
                    purrdf_sparql_eval::QueryOptions {
                        property_functions: registry
                            .as_ref()
                            .unwrap_or(&purrdf_sparql_eval::PropertyFunctionRegistry::EMPTY),
                        aggregates: aggregates
                            .as_ref()
                            .unwrap_or(&purrdf_sparql_eval::AggregateRegistry::EMPTY),
                        ..purrdf_sparql_eval::QueryOptions::EMPTY
                    },
                )
                .map_err(|e| PyValueError::new_err(format!("query evaluation error: {e}")))
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
    /// call runs.
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
        };
        let inner = &self.inner;
        // Snapshot + engine build + governed evaluation run detached (GIL released), so
        // the thread holding `cancel` keeps running while this one is in the engine.
        let outcome = run_governed(py, args, cancel, move |governors| {
            let dataset = inner
                .freeze()
                .map_err(|e| PyValueError::new_err(format!("store snapshot failed: {e}")))?;
            let registry = build_relations(specs, &dataset)?;
            let aggregates = build_aggregates(aggregate_namespace);
            let engine = build_engine(config);
            engine
                .query_governed(
                    &dataset,
                    SparqlRequest {
                        query,
                        base_iri: None,
                        substitutions: &subs,
                    },
                    purrdf_sparql_eval::QueryOptions {
                        property_functions: registry
                            .as_ref()
                            .unwrap_or(&purrdf_sparql_eval::PropertyFunctionRegistry::EMPTY),
                        aggregates: aggregates
                            .as_ref()
                            .unwrap_or(&purrdf_sparql_eval::AggregateRegistry::EMPTY),
                        ..purrdf_sparql_eval::QueryOptions::EMPTY
                    },
                    governors,
                )
                .map_err(|e| PyValueError::new_err(format!("query evaluation error: {e}")))
        })?;
        materialize_outcome(py, outcome)
    }

    /// Run a governed SPARQL query over a closure produced by the named entailment
    /// regime, carrying both the query outcome and the reasoning report.
    ///
    /// `aggregate_namespace` behaves exactly as on [`query_governed`](Self::query_governed):
    /// it registers purrdf's first-party statistical aggregate set under that IRI for the
    /// closure query's PARSE and its evaluation, so `AGG(<{NAMESPACE}NAME>, args…)` reaches
    /// the entailment-aware lane exactly as it reaches the ordinary one. Unset (the default)
    /// leaves every one of the ten names an ordinary unregistered custom-aggregate IRI.
    ///
    /// `property_fn_namespaces` / `relations` / `relations_from_graph` / `path_relations` behave exactly as on
    /// [`query_governed`](Self::query_governed): a registered relation is reachable from the
    /// closure query exactly as it is from an ordinary one, so registering an IRI here and
    /// omitting it there cannot silently change which rows the SAME predicate position
    /// yields. `relations_from_graph` reads its table — and `path_relations` snapshots its
    /// edges — from this store's PRE-entailment snapshot, the base the closure is
    /// materialized from, matching [`query`](Self::query)'s "this store's own default
    /// graph".
    #[pyo3(signature = (
        query,
        entailment,
        *,
        program="",
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
        cancel: Option<&PyCancellationToken>,
    ) -> PyResult<Py<PyEntailmentQueryOutcome>> {
        let subs = collect_substitutions(substitutions)?;
        let specs = collect_relations(relations, relations_from_graph, path_relations)?;
        let plan =
            QueryEntailmentPlan::parse(entailment, program).map_err(PyValueError::new_err)?;
        let args = GovernorArgs {
            fuel,
            deadline_ms,
            max_answers,
            max_intermediate_cells,
            max_scratch_bytes,
            max_remote_requests,
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
            let registry = build_relations(specs, &dataset)?;
            let engine = build_engine(config);
            let aggregates = build_aggregates(aggregate_namespace);
            query_with_entailment_governed(
                &engine,
                &dataset,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &subs,
                },
                plan.entailment(),
                purrdf_sparql_eval::QueryOptions {
                    property_functions: registry
                        .as_ref()
                        .unwrap_or(&purrdf_sparql_eval::PropertyFunctionRegistry::EMPTY),
                    aggregates: aggregates
                        .as_ref()
                        .unwrap_or(&purrdf_sparql_eval::AggregateRegistry::EMPTY),
                    ..purrdf_sparql_eval::QueryOptions::EMPTY
                },
                governors,
            )
            .map_err(|e| PyValueError::new_err(format!("entailment query failed: {e}")))
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
            let aggregates = build_aggregates(aggregate_namespace);
            let engine = build_engine(config);
            engine
                .update_with_options(
                    &mut dataset,
                    SparqlRequest {
                        query: update,
                        base_iri: None,
                        substitutions: &[],
                    },
                    purrdf_sparql_eval::QueryOptions {
                        property_functions: registry
                            .as_ref()
                            .unwrap_or(&purrdf_sparql_eval::PropertyFunctionRegistry::EMPTY),
                        aggregates: aggregates
                            .as_ref()
                            .unwrap_or(&purrdf_sparql_eval::AggregateRegistry::EMPTY),
                        ..purrdf_sparql_eval::QueryOptions::EMPTY
                    },
                )
                .map_err(|e| PyValueError::new_err(format!("update evaluation error: {e}")))?;
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
        };
        let inner = &self.inner;
        // Snapshot + governed evaluation run detached (GIL released).
        let (outcome, dataset) = run_governed(py, args, cancel, move |governors| {
            let mut dataset = inner
                .freeze()
                .map_err(|e| PyValueError::new_err(format!("store snapshot failed: {e}")))?;
            let registry = build_relations(specs, &dataset)?;
            let aggregates = build_aggregates(aggregate_namespace);
            let outcome = build_engine(config)
                .update_governed(
                    &mut dataset,
                    SparqlRequest {
                        query: update,
                        base_iri: None,
                        substitutions: &[],
                    },
                    purrdf_sparql_eval::QueryOptions {
                        property_functions: registry
                            .as_ref()
                            .unwrap_or(&purrdf_sparql_eval::PropertyFunctionRegistry::EMPTY),
                        aggregates: aggregates
                            .as_ref()
                            .unwrap_or(&purrdf_sparql_eval::AggregateRegistry::EMPTY),
                        ..purrdf_sparql_eval::QueryOptions::EMPTY
                    },
                    governors,
                )
                .map_err(|e| PyValueError::new_err(format!("update evaluation error: {e}")))?;
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

    /// Dump the whole store (or one graph, via `from_graph`) in `format`. Mirrors
    /// the oxigraph Python `Store.dump`: when `output` (a file-like with `.write`) is given
    /// the bytes are written to it and `None` is returned; otherwise the bytes are
    /// returned directly.
    #[pyo3(signature = (output=None, format=None, *, from_graph=None, jsonld_options=None, jsonld_context=None, yaml_schema_url=None))]
    #[allow(
        clippy::too_many_arguments,
        reason = "Python dump names graph selection and JSON-LD configuration explicitly"
    )]
    fn dump(
        &self,
        py: Python<'_>,
        output: Option<&Bound<'_, PyAny>>,
        format: Option<PyRdfFormat>,
        from_graph: Option<&Bound<'_, PyAny>>,
        jsonld_options: Option<&str>,
        jsonld_context: Option<&PyCompiledJsonLdContext>,
        yaml_schema_url: Option<&str>,
    ) -> PyResult<Option<Py<PyBytes>>> {
        let format = format.ok_or_else(|| PyValueError::new_err("dump: format is required"))?;
        let native = format.to_native();
        // Resolve the Python-side graph selection BEFORE releasing the GIL; the
        // quad materialization + native serialization run detached.
        let graph_projection: Option<Option<RdfTerm>> =
            if native.supports_datasets() && from_graph.is_none() {
                None
            } else {
                // `from_graph` selects one graph (a NamedNode/BlankNode → that graph; an
                // explicit DefaultGraph, or no `from_graph` on a non-dataset format → the
                // default graph). Project its triples into the default graph.
                Some(extract_graph_name(from_graph)?)
            };
        let configured =
            if jsonld_options.is_some() || jsonld_context.is_some() || yaml_schema_url.is_some() {
                Some(options_from_inputs(
                    jsonld_options,
                    jsonld_context,
                    yaml_schema_url,
                )?)
            } else {
                None
            };
        let buf: Vec<u8> = py.detach(|| {
            // Serialize natively: materialize the store's quads into the IR
            // verbatim (preserving literal lexical forms) and dispatch to the codec.
            let (quads, selection) = match &graph_projection {
                None => (self.collect_all_quads(), SerializeGraph::Dataset),
                Some(graph) => (
                    self.collect_graph_quads(graph.as_ref()),
                    SerializeGraph::DefaultGraph,
                ),
            };
            let dataset = dataset_from_quads_verbatim(&quads)
                .map_err(|e| PyValueError::new_err(format!("dump error: {e}")))?;
            if let Some(options) = &configured {
                serialize_dataset_with_jsonld_options(
                    &dataset,
                    native.media_type(),
                    selection,
                    options,
                )
                .map_err(|e| PyValueError::new_err(format!("dump error: {e}")))
            } else {
                serialize_dataset(&dataset, native.media_type(), selection)
                    .map_err(|e| PyValueError::new_err(format!("dump error: {e}")))
            }
        })?;
        match output {
            Some(output) => {
                output.call_method1("write", (PyBytes::new(py, &buf),))?;
                Ok(None)
            }
            None => Ok(Some(PyBytes::new(py, &buf).unbind())),
        }
    }

    /// Dump the WHOLE store in `format`, with the realized loss of doing so attached.
    ///
    /// The counting twin of [`dump`](Self::dump): same bytes, plus the three
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
    fn _store_capsule<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyCapsule>> {
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
}

impl PyStore {
    /// The next per-load blank scope ordinal (monotonic, wrapping past 1).
    fn next_load_scope(&self) -> u64 {
        self.next_load_scope.fetch_add(1, Ordering::Relaxed)
    }

    /// Every quad in the store, graph names intact (for the dataset-format dump path).
    fn collect_all_quads(&self) -> Vec<RdfQuad> {
        self.inner
            .quads_for_pattern(None, None, None, GraphMatchValue::Any)
            .iter()
            .map(values_to_rdf_quad)
            .collect()
    }

    /// The quads of ONE graph, re-homed to the default graph (so a single-graph dump
    /// serializes as triples). `graph` is the selected graph term, or `None` for the
    /// default graph — matching the oxigraph `Store.dump(from_graph=…)` projection.
    fn collect_graph_quads(&self, graph: Option<&RdfTerm>) -> Vec<RdfQuad> {
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

/// An in-memory quad set supporting RDFC-1.0 canonicalization. Mirrors
/// the oxigraph Python `Dataset`.
#[pyclass(name = "Dataset")]
#[derive(Debug)]
pub struct PyDataset {
    /// The accumulated quads, deduplicated by content (set semantics).
    quads: Vec<RdfQuad>,
}

#[pymethods]
impl PyDataset {
    /// Build a dataset, optionally seeding it from an iterable of `Quad`.
    #[new]
    #[pyo3(signature = (quads=None))]
    fn new(quads: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        let mut out = Self { quads: Vec::new() };
        if let Some(quads) = quads {
            for item in quads.try_iter()? {
                let item = item?;
                let quad = item
                    .cast::<PyQuad>()
                    .map_err(|_| PyTypeError::new_err("Dataset accepts an iterable of Quad"))?;
                out.insert(quad.get().inner.clone());
            }
        }
        Ok(out)
    }

    /// Add a single quad.
    fn add(&mut self, quad: &PyQuad) {
        self.insert(quad.inner.clone());
    }

    /// Canonicalize blank-node labels in place under `algorithm` (native RDFC-1.0).
    fn canonicalize(&mut self, py: Python<'_>, algorithm: PyCanonicalizationAlgorithm) {
        // RDFC-1.0 hashing is the heavy path — run it detached (GIL released).
        let quads = &self.quads;
        self.quads = py.detach(|| super::canon::canonicalize_quads(quads, algorithm));
    }

    fn __len__(&self) -> usize {
        self.quads.len()
    }

    fn __iter__(&self, py: Python<'_>) -> PyResult<Py<PyQuadIter>> {
        let quads = self.quads.clone();
        Py::new(py, PyQuadIter { quads, pos: 0 })
    }
}

impl PyDataset {
    /// Insert a quad with set semantics (no duplicate content).
    fn insert(&mut self, quad: RdfQuad) {
        if !self.quads.contains(&quad) {
            self.quads.push(quad);
        }
    }
}

/// Iterator over a [`PyDataset`]'s / [`PyStore`]'s quads (snapshot at iteration time).
#[pyclass(name = "QuadIter")]
#[derive(Debug)]
pub struct PyQuadIter {
    pub(crate) quads: Vec<RdfQuad>,
    pub(crate) pos: usize,
}

#[pymethods]
impl PyQuadIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>, py: Python<'_>) -> PyResult<Option<Py<PyQuad>>> {
        if slf.pos >= slf.quads.len() {
            return Ok(None);
        }
        let quad = slf.quads[slf.pos].clone();
        slf.pos += 1;
        Ok(Some(Py::new(py, PyQuad { inner: quad })?))
    }
}

// ── conversion helpers (native owned model ⇄ MutableDataset value model) ──────────

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

fn empty_mutable() -> PyResult<MutableDataset> {
    let base = RdfDatasetBuilder::new()
        .freeze()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(MutableDataset::new(base))
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

/// Convert a native owned [`RdfQuad`] into the `MutableDataset` [`QuadValues`] model
/// under the default blank scope.
fn rdf_quad_to_values(quad: &RdfQuad) -> QuadValues {
    rdf_quad_to_values_scoped(quad, BlankScope::DEFAULT)
}

/// Convert a native owned [`RdfQuad`] into [`QuadValues`], tagging every blank node
/// with `scope` (the per-load isolation scope).
fn rdf_quad_to_values_scoped(quad: &RdfQuad, scope: BlankScope) -> QuadValues {
    QuadValues {
        s: rdf_term_to_value_scoped(&quad.subject, scope),
        p: TermValue::Iri(quad.predicate.clone()),
        o: rdf_term_to_value_scoped(&quad.object, scope),
        g: quad
            .graph_name
            .as_ref()
            .map(|g| rdf_term_to_value_scoped(g, scope)),
    }
}

/// Convert a [`QuadValues`] back into the native owned [`RdfQuad`] model. Blank labels
/// are scope-qualified so a per-load scope is reflected in the surfaced label
/// (matching the prior oxigraph store's scoped blanks).
fn values_to_rdf_quad(values: &QuadValues) -> RdfQuad {
    let mut quad = RdfQuad::new(
        value_to_rdf_term(&values.s),
        predicate_iri(&values.p),
        value_to_rdf_term(&values.o),
    );
    quad.graph_name = values.g.as_ref().map(value_to_rdf_term);
    quad
}

fn predicate_iri(value: &TermValue) -> String {
    match value {
        TermValue::Iri(iri) => iri.clone(),
        other => value_to_rdf_term(other).to_string(),
    }
}

fn value_to_rdf_term(value: &TermValue) -> RdfTerm {
    match value {
        TermValue::Iri(iri) => RdfTerm::Iri(iri.clone()),
        TermValue::Blank { label, scope } => {
            RdfTerm::BlankNode(scope.qualify_label(label).into_owned())
        }
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => RdfTerm::Literal(RdfLiteral {
            datatype: collapse_synthetic_datatype(datatype, language.as_ref()),
            lexical_form: lexical_form.clone(),
            language: language.clone(),
            direction: *direction,
        }),
        TermValue::Triple { s, p, o } => RdfTerm::triple(RdfTriple::new(
            value_to_rdf_term(s),
            predicate_iri(p),
            value_to_rdf_term(o),
        )),
    }
}

/// Drop the synthetic `xsd:string` / `rdf:langString` datatype the value model always
/// carries, leaving the owned model's plain / lang literals datatype-less.
fn collapse_synthetic_datatype(datatype: &str, language: Option<&String>) -> Option<String> {
    if language.is_some() {
        return (datatype != RDF_LANG_STRING).then(|| datatype.to_owned());
    }
    (datatype != XSD_STRING).then(|| datatype.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iri(s: &str) -> RdfTerm {
        RdfTerm::iri(s)
    }

    #[test]
    fn scoping_keeps_iris_and_literals_verbatim() {
        let quad = RdfQuad::new(
            iri("https://e/s"),
            "https://e/p",
            RdfTerm::literal(RdfLiteral::simple("v")),
        );
        let values = rdf_quad_to_values_scoped(&quad, BlankScope(7));
        let back = values_to_rdf_quad(&values);
        assert_eq!(back, quad, "no blank node: the quad is unchanged");
    }

    #[test]
    fn same_label_different_scopes_yields_distinct_nodes() {
        // The regression guard: the SAME document-local blank label loaded
        // under two different scopes (two `Store::load` calls) MUST become two
        // distinct nodes once surfaced.
        let quad = RdfQuad::new(RdfTerm::blank_node("b0"), "https://e/p", iri("https://e/o"));
        let a = values_to_rdf_quad(&rdf_quad_to_values_scoped(&quad, BlankScope(1)));
        let b = values_to_rdf_quad(&rdf_quad_to_values_scoped(&quad, BlankScope(2)));
        assert_ne!(a.subject, b.subject);
        // …but the same label within one scope is the SAME node (intra-document joins).
        let a2 = values_to_rdf_quad(&rdf_quad_to_values_scoped(&quad, BlankScope(1)));
        assert_eq!(a.subject, a2.subject);
    }

    #[test]
    fn scoping_recurses_into_quoted_triple_terms() {
        let quad = RdfQuad::new(
            RdfTerm::blank_node("r"),
            "https://e/p",
            RdfTerm::triple(RdfTriple::new(
                RdfTerm::blank_node("s"),
                "https://e/q",
                RdfTerm::blank_node("o"),
            )),
        );
        let values = rdf_quad_to_values_scoped(&quad, BlankScope(5));
        let back = values_to_rdf_quad(&values);
        let RdfTerm::Triple(t) = &back.object else {
            panic!("object must stay a quoted triple");
        };
        // Both the reifier subject and the inner triple's blanks carry the scope.
        assert!(matches!(&back.subject, RdfTerm::BlankNode(l) if l.contains('5')));
        assert!(matches!(&t.subject, RdfTerm::BlankNode(l) if l.contains('5')));
        assert!(matches!(&t.object, RdfTerm::BlankNode(l) if l.contains('5')));
    }

    #[test]
    fn plain_literal_round_trips_without_synthetic_datatype() {
        let values = QuadValues {
            s: TermValue::Iri("https://e/s".to_owned()),
            p: TermValue::Iri("https://e/p".to_owned()),
            o: TermValue::Literal {
                lexical_form: "hi".to_owned(),
                datatype: XSD_STRING.to_owned(),
                language: None,
                direction: None,
            },
            g: None,
        };
        let quad = values_to_rdf_quad(&values);
        let RdfTerm::Literal(lit) = &quad.object else {
            panic!("expected a literal");
        };
        assert_eq!(lit.datatype, None, "plain literal stays datatype-less");
    }

    // ── capsule boundary ─────────────────────────────────────────────
    //
    // These tests pin the `_store_capsule` contract WITHOUT a Python interpreter:
    // they exercise the same snapshot → `Box<Arc<RdfDataset>>` → raw-address →
    // borrow lifecycle the capsule producer/consumer use across the FFI boundary.
    // The capsule's `#[pymethods]` are thin wrappers over exactly this logic.

    /// Build a [`MutableDataset`] seeded with `n` distinct default-graph triples.
    fn mutable_with(n: usize) -> MutableDataset {
        let mut m = empty_mutable().expect("empty");
        for i in 0..n {
            m.insert(rdf_quad_to_values(&RdfQuad::new(
                iri(&format!("https://e/s{i}")),
                "https://e/p",
                iri("https://e/o"),
            )));
        }
        m
    }

    /// Freeze the snapshot exactly as `_store_capsule` does (a frozen `Arc`), box it
    /// for a stable address, read the pointee back by raw address, and assert the
    /// boxed Arc round-trips. Dropping the box drops exactly ONE strong ref — no
    /// double-free (the destructor closure owns the single `keepalive` box).
    #[test]
    fn capsule_snapshot_round_trips_by_address_without_double_free() {
        let store = mutable_with(2);
        let snapshot: Arc<RdfDataset> = store.freeze().expect("freeze");
        assert_eq!(Arc::strong_count(&snapshot), 1);

        // Mirror `_store_capsule`: box the Arc so its address is stable, hand out the
        // address, then read the Arc back through the raw pointer (as the consumer
        // does after `pointer_checked`).
        let boxed: Box<Arc<RdfDataset>> = Box::new(snapshot);
        let addr = (&raw const *boxed) as usize;
        // SAFETY: `addr` is the live address of the Arc owned by `boxed` (test-local).
        let borrowed: &Arc<RdfDataset> = unsafe { &*(addr as *const Arc<RdfDataset>) };
        assert_eq!(borrowed.quad_count(), 2);
        // The consumer may clone the Arc to extend its lifetime; that is a second
        // strong ref over the SAME dataset, dropped before the box is.
        let consumer_clone = Arc::clone(borrowed);
        assert_eq!(Arc::strong_count(borrowed), 2);
        drop(consumer_clone);
        assert_eq!(Arc::strong_count(borrowed), 1);
        // The capsule destructor drops the box exactly once → one strong ref freed.
        drop(boxed);
    }

    /// Snapshot-vs-mutation aliasing: a consumer holding the frozen snapshot Arc must
    /// see a STABLE dataset after the producing store mutates (the capsule hands out
    /// an immutable frozen snapshot, not a live view).
    #[test]
    fn capsule_snapshot_is_unaffected_by_later_store_mutation() {
        let mut store = mutable_with(1);
        let snapshot: Arc<RdfDataset> = store.freeze().expect("freeze");
        assert_eq!(snapshot.quad_count(), 1);

        // The store mutates AFTER the snapshot was taken (a later `Store.add`).
        store.insert(rdf_quad_to_values(&RdfQuad::new(
            iri("https://e/s-new"),
            "https://e/p",
            iri("https://e/o"),
        )));
        let after = store.freeze().expect("freeze again");

        // The earlier snapshot the consumer holds is UNCHANGED…
        assert_eq!(snapshot.quad_count(), 1, "held snapshot must stay stable");
        // …while a fresh snapshot reflects the mutation.
        assert_eq!(after.quad_count(), 2, "fresh snapshot sees the new quad");
    }
}
