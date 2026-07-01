// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Python-facing copy-on-write mutable dataset for the RDFLib compat shim.
//!
//! The canonical mutation semantics live in `purrdf-core::MutableDataset`.
//! This adapter keeps Python on that COW surface; query / update run on the native
//! `NativeSparqlEngine` over a frozen snapshot (EPIC #906 — no oxigraph).

use std::sync::Arc;

use purrdf_core::ir::{MutableDataset, QuadValues};
use purrdf_sparql_eval::NativeSparqlEngine;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};

use super::io::{dataset_from_quads_verbatim, parse_quads, read_input, PyRdfFormat};
use super::query::materialize_results;
use super::store::PyQuadIter;
use super::term::{extract_graph_name, extract_term, PyQuad, PyVariable};
use crate::{
    serialize_dataset, BlankScope, DatasetMut, GraphMatchValue, RdfDataset, RdfDatasetBuilder,
    RdfLiteral, RdfQuad, RdfTerm, RdfTriple, SerializeGraph, SparqlEngine, SparqlRequest,
    TermValue,
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
    #[pyo3(signature = (input=None, format=None, *, path=None))]
    fn load(
        &mut self,
        input: Option<&Bound<'_, PyAny>>,
        format: Option<PyRdfFormat>,
        path: Option<String>,
    ) -> PyResult<()> {
        let format = format.ok_or_else(|| PyValueError::new_err("load: format is required"))?;
        let data = read_input(input, path)?;
        let blank_scope = self.allocate_blank_scope();
        for quad in parse_quads(&data, format.to_native())
            .map_err(|e| PyValueError::new_err(format!("load parse error: {e}")))?
        {
            self.inner
                .insert(rdf_quad_to_values_scoped(&quad, blank_scope));
        }
        Ok(())
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
        let graph_match = if any_graph {
            GraphMatchValue::Any
        } else {
            match g_value.as_ref() {
                Some(g) => GraphMatchValue::Named(g),
                None => GraphMatchValue::Default,
            }
        };
        self.inner
            .quads_for_pattern(s.as_ref(), p.as_ref(), o.as_ref(), graph_match)
            .iter()
            .map(|q| {
                Py::new(
                    py,
                    PyQuad {
                        inner: values_to_rdf_quad(q),
                    },
                )
            })
            .collect()
    }

    /// Dump the effective dataset (or one graph) in `format`.
    #[pyo3(signature = (output=None, format=None, *, from_graph=None))]
    fn dump(
        &self,
        py: Python<'_>,
        output: Option<&Bound<'_, PyAny>>,
        format: Option<PyRdfFormat>,
        from_graph: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Option<Py<PyBytes>>> {
        let format = format.ok_or_else(|| PyValueError::new_err("dump: format is required"))?;
        let native = format.to_native();
        // Materialize the effective set into the IR verbatim, then serialize through
        // the native codec (#909) — literal lexical forms are preserved.
        let dataset = self.materialize_dataset()?;
        let graph_filter = match from_graph {
            Some(graph) => optional_graph_value(Some(graph))?,
            None => None,
        };
        let selection = match (&graph_filter, from_graph.is_some()) {
            (Some(name), _) => SerializeGraph::Named(name),
            // An explicit default-graph (`from_graph=DefaultGraph`) selection.
            (None, true) => SerializeGraph::DefaultGraph,
            (None, false) if native.supports_datasets() => SerializeGraph::Dataset,
            (None, false) => SerializeGraph::DefaultGraph,
        };
        let buf = serialize_dataset(&dataset, native.media_type(), selection)
            .map_err(|e| PyValueError::new_err(format!("dump error: {e}")))?;
        match output {
            Some(output) => {
                output.call_method1("write", (PyBytes::new(py, &buf),))?;
                Ok(None)
            }
            None => Ok(Some(PyBytes::new(py, &buf).unbind())),
        }
    }

    /// Run a SPARQL query over the effective dataset.
    #[pyo3(signature = (query, *, substitutions=None))]
    fn query(
        &self,
        py: Python<'_>,
        query: &str,
        substitutions: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Py<PyAny>> {
        let dataset = self.snapshot()?;
        let subs = collect_substitutions(substitutions)?;
        let engine = NativeSparqlEngine::new();
        let result = engine
            .query(
                &dataset,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &subs,
                },
            )
            .map_err(|e| PyValueError::new_err(format!("query evaluation error: {e}")))?;
        materialize_results(py, result)
    }

    /// Run a SPARQL UPDATE (COW-atomic: a failed update leaves the set unchanged).
    fn update(&mut self, update: &str) -> PyResult<()> {
        let mut dataset = self.snapshot()?;
        let engine = NativeSparqlEngine::new();
        engine
            .update(
                &mut dataset,
                SparqlRequest {
                    query: update,
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .map_err(|e| PyValueError::new_err(format!("update evaluation error: {e}")))?;
        self.inner = MutableDataset::new(dataset);
        Ok(())
    }

    /// Compact the effective set into a fresh frozen base.
    fn compact(&mut self) -> PyResult<()> {
        let frozen = self
            .inner
            .freeze()
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

    /// Freeze the effective COW quad set into an immutable `Arc<RdfDataset>` snapshot.
    fn snapshot(&self) -> PyResult<Arc<RdfDataset>> {
        self.inner
            .freeze()
            .map_err(|e| PyValueError::new_err(format!("snapshot failed: {e}")))
    }

    /// Freeze the effective quad set into the IR verbatim (RDF 1.2 triple-term
    /// objects preserved; no statement-layer fold), for native serialization (#909).
    fn materialize_dataset(&self) -> PyResult<Arc<RdfDataset>> {
        let quads: Vec<RdfQuad> = self
            .inner
            .quads_for_pattern(None, None, None, GraphMatchValue::Any)
            .iter()
            .map(values_to_rdf_quad)
            .collect();
        dataset_from_quads_verbatim(&quads).map_err(PyValueError::new_err)
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

fn rdf_term_to_value(term: &RdfTerm) -> TermValue {
    rdf_term_to_value_scoped(term, BlankScope::DEFAULT)
}

fn rdf_term_to_value_scoped(term: &RdfTerm, scope: BlankScope) -> TermValue {
    match term {
        RdfTerm::Iri(iri) => TermValue::Iri(iri.clone()),
        RdfTerm::BlankNode(label) => blank_value_scoped(label, scope),
        RdfTerm::Literal(lit) => TermValue::Literal {
            lexical_form: lit.lexical_form.clone(),
            datatype: match (&lit.datatype, &lit.language) {
                (Some(dt), _) => dt.clone(),
                (None, Some(_)) => RDF_LANG_STRING.to_owned(),
                (None, None) => XSD_STRING.to_owned(),
            },
            language: lit.language.clone(),
            direction: lit.direction,
        },
        RdfTerm::Triple(t) => TermValue::Triple {
            s: Box::new(rdf_term_to_value_scoped(&t.subject, scope)),
            p: Box::new(TermValue::Iri(t.predicate.clone())),
            o: Box::new(rdf_term_to_value_scoped(&t.object, scope)),
        },
    }
}

/// Build the `TermValue::Blank` for a surfaced blank-node `label`.
///
/// Under a non-default `scope` (the per-load isolation path), the bare label is
/// tagged with that scope verbatim. Under the DEFAULT scope (a blank node arriving
/// FROM Python), the label may already carry the `.s{n}` scope suffix
/// [`BlankScope::qualify_label`] emitted on the way OUT; decode it back to its
/// `(label, scope)` so a round-tripped blank matches the stored node.
fn blank_value_scoped(label: &str, scope: BlankScope) -> TermValue {
    if scope == BlankScope::DEFAULT {
        blank_value_from_external_label(label)
    } else {
        TermValue::Blank {
            label: label.to_owned(),
            scope,
        }
    }
}

/// Decode a surfaced blank label, reversing [`BlankScope::qualify_label`]: a label of
/// the form `"{inner}.s{n}"` (non-empty `inner`, `n > 0`) decodes to
/// `Blank{inner, scope: n}`; any other label is a DEFAULT-scope blank verbatim.
fn blank_value_from_external_label(label: &str) -> TermValue {
    if let Some((inner, raw_scope)) = label.rsplit_once(".s") {
        if !inner.is_empty() {
            if let Ok(scope) = raw_scope.parse::<u32>() {
                if scope > 0 {
                    return TermValue::Blank {
                        label: inner.to_owned(),
                        scope: BlankScope(scope),
                    };
                }
            }
        }
    }
    TermValue::Blank {
        label: label.to_owned(),
        scope: BlankScope::DEFAULT,
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
