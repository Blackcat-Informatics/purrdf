// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Python-facing copy-on-write mutable dataset for the RDFLib compat shim.
//!
//! The canonical mutation semantics live in `purrdf-core::MutableDataset`.
//! This adapter keeps Python on that COW surface; query / update run on the native
//! `NativeSparqlEngine` over a frozen snapshot.

use purrdf_core::ir::MutableDataset;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use super::io::{
    PyRdfFormat, dataset_from_quads_verbatim, declare_loaded_graphs, parse_quads_and_prefixes,
    read_input,
};
use super::quad_store::PyQuadStore;
use super::term::{
    PyQuad, extract_graph_name, extract_term, rdf_quad_to_values, rdf_quad_to_values_scoped,
    rdf_term_to_value, values_to_rdf_quad,
};
use crate::py_jsonld::{PyCompiledJsonLdContext, options_from_inputs};
use crate::py_store::iri_value_error;
use crate::{
    BlankScope, DatasetMut, GraphMatchValue, RdfQuad, SerializeGraph, SerializeOptions,
    StatementLayer, TermValue, serialize_dataset_with,
};

/// A COW mutable RDF dataset over the native `purrdf-core` IR.
///
/// The query, UPDATE, iteration and validation-capsule surface is the
/// [`PyQuadStore`] base class's, shared with `Store`.
/// The keyword-only `remember_empty_graphs=True` selects native remembered
/// graph slots; the 3.x default is false. Compaction and UPDATE retain the choice.
#[pyclass(name = "MutableDataset", extends = PyQuadStore)]
#[derive(Debug)]
pub struct PyMutableDataset {
    next_blank_scope: u32,
}

#[pymethods]
impl PyMutableDataset {
    #[new]
    #[pyo3(signature = (*, remember_empty_graphs=false))]
    fn new(remember_empty_graphs: bool) -> PyResult<PyClassInitializer<Self>> {
        Ok(
            PyClassInitializer::from(PyQuadStore::empty(remember_empty_graphs)?).add_subclass(
                Self {
                    next_blank_scope: 1,
                },
            ),
        )
    }

    /// Load RDF into the mutable dataset.
    ///
    /// Returns the document's prefix map exactly as `Store.load` does.
    #[pyo3(signature = (input=None, format=None, *, path=None, base=None))]
    fn load(
        mut slf: PyRefMut<'_, Self>,
        py: Python<'_>,
        input: Option<&Bound<'_, PyAny>>,
        format: Option<PyRdfFormat>,
        path: Option<String>,
        base: Option<String>,
    ) -> PyResult<Vec<(String, String)>> {
        let format = format.ok_or_else(|| PyValueError::new_err("load: format is required"))?;
        let data = read_input(input, path)?;
        let blank_scope = slf.allocate_blank_scope();
        let inner = &mut slf.as_super().inner;
        // Parse + insert run detached (GIL released); only plain Rust data is touched.
        py.detach(move || {
            let base_ref = base.as_deref();
            let (quads, prefixes, declared) =
                parse_quads_and_prefixes(&data, format.to_native(), base_ref)
                    .map_err(|e| PyValueError::new_err(format!("load parse error: {e}")))?;
            for quad in quads {
                inner
                    .insert(rdf_quad_to_values_scoped(&quad, blank_scope))
                    .map_err(|e| iri_value_error(&e))?;
            }
            declare_loaded_graphs(inner, declared, blank_scope)?;
            Ok(prefixes)
        })
    }

    /// Add a single quad. Returns whether the effective set changed.
    ///
    /// Raises `ValueError` if the quad carries a relative IRI in any position. This
    /// surface is handed terms, never a document, so no base is in scope to resolve
    /// one against and none is invented; the message leads with the shared
    /// `iri-relative-no-base` diagnostic code.
    fn add(mut slf: PyRefMut<'_, Self>, quad: &PyQuad) -> PyResult<bool> {
        slf.as_super()
            .inner
            .insert(rdf_quad_to_values(&quad.inner))
            .map_err(|e| iri_value_error(&e))
    }

    /// Remove a single quad. Returns whether the effective set changed.
    fn remove(mut slf: PyRefMut<'_, Self>, quad: &PyQuad) -> PyResult<bool> {
        Ok(slf
            .as_super()
            .inner
            .remove(&rdf_quad_to_values(&quad.inner)))
    }

    /// Return whether the exact quad is effective.
    fn contains(slf: &Bound<'_, Self>, quad: &PyQuad) -> PyResult<bool> {
        Ok(slf
            .as_super()
            .borrow()
            .inner
            .contains(&rdf_quad_to_values(&quad.inner)))
    }

    /// Effective quads matching a value pattern.
    #[pyo3(signature = (subject=None, predicate=None, object=None, graph_name=None, *, any_graph=false))]
    fn quads_for_pattern(
        slf: &Bound<'_, Self>,
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
        let guard = slf.as_super().borrow();
        let inner = &guard.inner;
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
    ///
    /// `base` is the document base the output is written under — the egress MIRROR of
    /// [`load`](Self::load)'s. Honored exactly as everywhere else: written and
    /// relativized against by the syntaxes that can express one, absolute IRIs from the
    /// ones that cannot, and a hard failure if it is not an absolute IRI.
    ///
    /// The statement layer is [`StatementLayer::Emit`] — the fidelity answer this dump
    /// already applied. It is preserved even under a `Named` graph selection, where the
    /// core reports the rows as dropped because the selection (not the format) excluded
    /// them; that accounting is exactly why `Emit` must be named here rather than
    /// derived.
    #[pyo3(signature = (output=None, format=None, *, from_graph=None, jsonld_options=None, jsonld_context=None, yaml_schema_url=None, base=None))]
    #[allow(
        clippy::too_many_arguments,
        reason = "Python dump names graph selection, the document base, and JSON-LD configuration explicitly"
    )]
    fn dump(
        slf: &Bound<'_, Self>,
        py: Python<'_>,
        output: Option<&Bound<'_, PyAny>>,
        format: Option<PyRdfFormat>,
        from_graph: Option<&Bound<'_, PyAny>>,
        jsonld_options: Option<&str>,
        jsonld_context: Option<&PyCompiledJsonLdContext>,
        yaml_schema_url: Option<&str>,
        base: Option<String>,
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
        let guard = slf.as_super().borrow();
        let store: &PyQuadStore = &guard;
        // Materialize the effective set into the IR verbatim, then serialize through
        // the native codec — literal lexical forms are preserved. Both steps
        // run detached (GIL released).
        let buf: Vec<u8> = py.detach(move || {
            let quads = store.collect_all_quads();
            let dataset = dataset_from_quads_verbatim(&quads, &store.declared_graphs())
                .map_err(PyValueError::new_err)?;
            let selection = match (&graph_filter, explicit_from_graph) {
                (Some(name), _) => SerializeGraph::Named(name),
                // An explicit default-graph (`from_graph=DefaultGraph`) selection.
                (None, true) => SerializeGraph::DefaultGraph,
                (None, false) if native.supports_datasets() => SerializeGraph::Dataset,
                (None, false) => SerializeGraph::DefaultGraph,
            };
            // ONE serialization call: the base, the graph selection, the statement layer
            // and the JSON-LD options are all axes of the same options value, so no
            // configured/unconfigured split can drop one of them.
            serialize_dataset_with(
                &dataset,
                native,
                base.as_deref(),
                &SerializeOptions {
                    selection,
                    statement_layer: StatementLayer::Emit,
                    jsonld_options: configured.as_ref(),
                },
            )
            .map(|outcome| outcome.bytes)
            .map_err(|e| PyValueError::new_err(format!("dump error: {e}")))
        })?;
        match output {
            Some(output) => {
                output.call_method1("write", (PyBytes::new(py, &buf),))?;
                Ok(None)
            }
            None => Ok(Some(PyBytes::new(py, &buf).unbind())),
        }
    }

    /// Compact the effective set into a fresh frozen base.
    fn compact(mut slf: PyRefMut<'_, Self>, py: Python<'_>) -> PyResult<()> {
        // The COW freeze can be a real copy — run it detached (GIL released).
        let store: &mut PyQuadStore = slf.as_super();
        let frozen = py
            .detach(|| store.inner.freeze())
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        store.inner =
            MutableDataset::new_with_graph_existence(frozen, store.inner.graph_existence());
        Ok(())
    }
}

impl PyMutableDataset {
    fn allocate_blank_scope(&mut self) -> BlankScope {
        let scope = BlankScope(self.next_blank_scope);
        self.next_blank_scope = self.next_blank_scope.checked_add(1).unwrap_or(1);
        scope
    }
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
