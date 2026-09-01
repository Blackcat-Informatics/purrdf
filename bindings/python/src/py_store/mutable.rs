// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Python-facing copy-on-write mutable dataset for the RDFLib compat shim.
//!
//! The canonical mutation semantics live in `purrdf-core::MutableDataset`.
//! This adapter keeps Python on that COW surface; query / update run on the native
//! `NativeSparqlEngine` over a frozen snapshot ( — no oxigraph).

use purrdf_core::ir::{MutableDataset, QuadValues};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};

use super::io::{PyRdfFormat, dataset_from_quads_verbatim, parse_quads, read_input};
use super::query::{
    EngineConfig, GovernorArgs, PyCancellationToken, PyEntailmentQueryOutcome, PyQueryOutcome,
    PyUpdateOutcome, build_aggregates, build_engine, build_relations, collect_relations,
    materialize_entailment_outcome, materialize_outcome, materialize_results,
    materialize_update_outcome, run_governed,
};
use super::store::PyQuadIter;
use super::term::{
    PyQuad, PyVariable, extract_graph_name, extract_term, rdf_term_to_value,
    rdf_term_to_value_scoped,
};
use crate::py_jsonld::{PyCompiledJsonLdContext, options_from_inputs};
use crate::{
    BlankScope, DatasetMut, GraphMatchValue, QueryEntailmentPlan, RdfDatasetBuilder, RdfLiteral,
    RdfQuad, RdfTerm, RdfTriple, SerializeGraph, SparqlRequest, TermValue,
    query_with_entailment_governed, serialize_dataset, serialize_dataset_with_jsonld_options,
};

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// A COW mutable RDF dataset over the native `purrdf-core` IR.
#[pyclass(name = "MutableDataset")]
#[derive(Debug)]
pub struct PyMutableDataset {
    inner: MutableDataset,
    next_blank_scope: u32,
}

#[pymethods]
impl PyMutableDataset {
    #[new]
    fn new() -> PyResult<Self> {
        Ok(Self {
            inner: empty_mutable()?,
            next_blank_scope: 1,
        })
    }

    /// Load RDF into the mutable dataset.
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
        let blank_scope = self.allocate_blank_scope();
        let inner = &mut self.inner;
        // Parse + insert run detached (GIL released); only plain Rust data is touched.
        py.detach(move || {
            let base_ref = base.as_deref();
            for quad in parse_quads(&data, format.to_native(), base_ref)
                .map_err(|e| PyValueError::new_err(format!("load parse error: {e}")))?
            {
                inner.insert(rdf_quad_to_values_scoped(&quad, blank_scope));
            }
            Ok(())
        })
    }

    /// Add a single quad. Returns whether the effective set changed.
    fn add(&mut self, quad: &PyQuad) -> PyResult<bool> {
        Ok(self.inner.insert(rdf_quad_to_values(&quad.inner)))
    }

    /// Remove a single quad. Returns whether the effective set changed.
    fn remove(&mut self, quad: &PyQuad) -> PyResult<bool> {
        Ok(self.inner.remove(&rdf_quad_to_values(&quad.inner)))
    }

    /// Return whether the exact quad is effective.
    fn contains(&self, quad: &PyQuad) -> PyResult<bool> {
        Ok(self.inner.contains(&rdf_quad_to_values(&quad.inner)))
    }

    /// Effective quads matching a value pattern.
    #[pyo3(signature = (subject=None, predicate=None, object=None, graph_name=None, *, any_graph=false))]
    fn quads_for_pattern(
        &self,
        py: Python<'_>,
        subject: Option<&Bound<'_, PyAny>>,
        predicate: Option<&Bound<'_, PyAny>>,
        object: Option<&Bound<'_, PyAny>>,
        graph_name: Option<&Bound<'_, PyAny>>,
        any_graph: bool,
    ) -> PyResult<Vec<Py<PyQuad>>> {
        let s = optional_term(subject)?;
        let p = optional_term(predicate)?;
        let o = optional_term(object)?;
        let g_value = optional_graph_value(graph_name)?;
        let inner = &self.inner;
        // The pattern scan over the effective set runs detached (GIL released);
        // the matched quads are wrapped into Python objects after reacquiring.
        let quads: Vec<RdfQuad> = py.detach(|| {
            let graph_match = if any_graph {
                GraphMatchValue::Any
            } else {
                match g_value.as_ref() {
                    Some(g) => GraphMatchValue::Named(g),
                    None => GraphMatchValue::Default,
                }
            };
            inner
                .quads_for_pattern(s.as_ref(), p.as_ref(), o.as_ref(), graph_match)
                .iter()
                .map(values_to_rdf_quad)
                .collect()
        });
        quads
            .into_iter()
            .map(|inner| Py::new(py, PyQuad { inner }))
            .collect()
    }

    /// Dump the effective dataset (or one graph) in `format`.
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
        // Resolve the Python-side graph selection BEFORE releasing the GIL.
        let graph_filter = match from_graph {
            Some(graph) => optional_graph_value(Some(graph))?,
            None => None,
        };
        let explicit_from_graph = from_graph.is_some();
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
        let inner = &self.inner;
        // Materialize the effective set into the IR verbatim, then serialize through
        // the native codec — literal lexical forms are preserved. Both steps
        // run detached (GIL released).
        let buf: Vec<u8> = py.detach(|| {
            let quads: Vec<RdfQuad> = inner
                .quads_for_pattern(None, None, None, GraphMatchValue::Any)
                .iter()
                .map(values_to_rdf_quad)
                .collect();
            let dataset = dataset_from_quads_verbatim(&quads).map_err(PyValueError::new_err)?;
            let selection = match (&graph_filter, explicit_from_graph) {
                (Some(name), _) => SerializeGraph::Named(name),
                // An explicit default-graph (`from_graph=DefaultGraph`) selection.
                (None, true) => SerializeGraph::DefaultGraph,
                (None, false) if native.supports_datasets() => SerializeGraph::Dataset,
                (None, false) => SerializeGraph::DefaultGraph,
            };
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

    /// Run a SPARQL query over the effective dataset.
    ///
    /// The engine-configuration and relation keywords are exactly those of
    /// `Store.query`: `extension_namespaces` / `property_fn_namespaces` declare
    /// prefix recognition, `standpoint_predicates` is the `(according_to, sharpens)`
    /// table `heldIn` requires, and `relations` / `relations_from_graph` / `path_relations` register
    /// host relations for this call.
    ///
    /// `aggregate_namespace` behaves exactly as on `Store.query` (see
    /// [`build_aggregates`](super::query::build_aggregates)).
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
                .map_err(|e| PyValueError::new_err(format!("snapshot failed: {e}")))?;
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
    /// `QueryOutcome`. The keywords, the outcome, and the Ctrl-C interaction are exactly
    /// those of `Store.query_governed`.
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
                .map_err(|e| PyValueError::new_err(format!("snapshot failed: {e}")))?;
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

    /// Governed entailment-aware query, with the same two-phase carrier as `Store`.
    ///
    /// `aggregate_namespace` behaves exactly as on `Store.query_entailment_governed`.
    ///
    /// `property_fn_namespaces` / `relations` / `relations_from_graph` / `path_relations` behave exactly as on
    /// `Store.query_entailment_governed`: a registered relation is reachable from the closure
    /// query exactly as it is from an ordinary one. `relations_from_graph` reads its table —
    /// and `path_relations` snapshots its edges — from this dataset's PRE-entailment
    /// snapshot, the base the closure is materialized from.
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
                .map_err(|e| PyValueError::new_err(format!("snapshot failed: {e}")))?;
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

    /// Run a SPARQL UPDATE under caller-supplied execution governors, returning an
    /// `UpdateOutcome`. The keywords — governors, engine configuration, and the
    /// `relations` / `relations_from_graph` / `path_relations` tables — and the all-or-nothing guarantee
    /// are exactly those of `Store.update_governed`.
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
                .map_err(|e| PyValueError::new_err(format!("snapshot failed: {e}")))?;
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
        // Adopted only on the applied path: a tripped request published nothing, so the
        // COW base it started from is still the one this dataset must keep.
        if outcome.is_applied() {
            self.inner = MutableDataset::new(dataset);
        }
        materialize_update_outcome(py, &outcome)
    }

    /// Run a SPARQL UPDATE (COW-atomic: a failed update leaves the set unchanged).
    /// The engine-configuration and relation keywords configure the engine exactly
    /// as on [`query`](Self::query); a `relations_from_graph` table is read — and a
    /// `path_relations` traversal is snapshotted — from the PRE-update state, the state
    /// the `WHERE` clause matches.
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
                .map_err(|e| PyValueError::new_err(format!("snapshot failed: {e}")))?;
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
        self.inner = MutableDataset::new(dataset);
        Ok(())
    }

    /// Compact the effective set into a fresh frozen base.
    fn compact(&mut self, py: Python<'_>) -> PyResult<()> {
        // The COW freeze can be a real copy — run it detached (GIL released).
        let inner = &self.inner;
        let frozen = py
            .detach(|| inner.freeze())
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        self.inner = MutableDataset::new(frozen);
        Ok(())
    }

    fn __len__(&self) -> usize {
        self.inner
            .quads_for_pattern(None, None, None, GraphMatchValue::Any)
            .len()
    }

    fn __iter__(&self, py: Python<'_>) -> PyResult<Py<PyQuadIter>> {
        let quads = self
            .inner
            .quads_for_pattern(None, None, None, GraphMatchValue::Any)
            .iter()
            .map(values_to_rdf_quad)
            .collect();
        Py::new(py, PyQuadIter { quads, pos: 0 })
    }
}

impl PyMutableDataset {
    fn allocate_blank_scope(&mut self) -> BlankScope {
        let scope = BlankScope(self.next_blank_scope);
        self.next_blank_scope = self.next_blank_scope.checked_add(1).unwrap_or(1);
        scope
    }
}

fn empty_mutable() -> PyResult<MutableDataset> {
    let base = RdfDatasetBuilder::new()
        .freeze()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(MutableDataset::new(base))
}

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

fn optional_term(obj: Option<&Bound<'_, PyAny>>) -> PyResult<Option<TermValue>> {
    let Some(obj) = obj else {
        return Ok(None);
    };
    if obj.is_none() {
        return Ok(None);
    }
    Ok(Some(rdf_term_to_value(&extract_term(obj)?)))
}

fn optional_graph_value(obj: Option<&Bound<'_, PyAny>>) -> PyResult<Option<TermValue>> {
    let Some(obj) = obj else {
        return Ok(None);
    };
    if obj.is_none() {
        return Ok(None);
    }
    Ok(extract_graph_name(Some(obj))?
        .as_ref()
        .map(rdf_term_to_value))
}

// ── native owned model ⇄ MutableDataset value model ───────────────────────────────

fn rdf_quad_to_values(quad: &RdfQuad) -> QuadValues {
    rdf_quad_to_values_scoped(quad, BlankScope::DEFAULT)
}

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

fn collapse_synthetic_datatype(datatype: &str, language: Option<&String>) -> Option<String> {
    if language.is_some() {
        return (datatype != RDF_LANG_STRING).then(|| datatype.to_owned());
    }
    (datatype != XSD_STRING).then(|| datatype.to_owned())
}
