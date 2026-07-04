// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! PyO3 Python bindings for `purrdf-shapes`.
//!
//! # Platform note
//!
//! This module belongs to the separate Python binding crate because pyo3 exposes
//! CPython C-API symbols and those are intentionally unavailable in the main
//! wasm-clean Rust crates. There are zero degraded fallbacks and zero feature
//! flags controlling this.
//!
//! # Engine core separation
//!
//! Only this file imports pyo3. All engine modules (`engine`, `shapes`,
//! `constraints`, `path`, `report`, `model`) are PyO3-free so the rlib links
//! into the future Rust compiler without any Python dependency.

use std::sync::Arc;

use ::purrdf::RdfDataset;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyCapsule, PyCapsuleMethods, PyDict, PyList};

use purrdf_shapes::engine;
use purrdf_shapes::report::ValidationReport;

/// Validate a data graph (N-Triples) against a shapes graph (Turtle).
///
/// Returns a dict with keys:
/// - `"conforms"` — bool
/// - `"results"` — list of dicts, each with keys:
///   `"focus"`, `"path"`, `"value"`, `"severity"`, `"component"`,
///   `"source_shape"`, `"message"`.
#[pyfunction]
fn validate(py: Python<'_>, shapes_ttl: &str, data_nt: &str) -> PyResult<Py<PyAny>> {
    // Parse + validation run detached (GIL released); the result dicts are
    // built after the GIL is reacquired.
    let report = py
        .detach(|| engine::validate_graphs(data_nt, shapes_ttl))
        .map_err(pyo3::exceptions::PyValueError::new_err)?;

    let out = PyDict::new(py);
    out.set_item("conforms", report.conforms)?;

    let results = PyList::empty(py);
    for r in &report.results {
        let d = PyDict::new(py);
        d.set_item("focus", r.focus_node.to_string())?;
        d.set_item("path", r.result_path.as_ref().map(ToString::to_string))?;
        d.set_item("value", r.value.as_ref().map(ToString::to_string))?;
        d.set_item("severity", r.severity.iri())?;
        d.set_item("component", r.source_constraint_component.as_str())?;
        d.set_item("source_shape", r.source_shape.to_string())?;
        d.set_item("message", r.message.clone())?;
        if !r.source_box_roles.is_empty() {
            let roles: Vec<&str> = r
                .source_box_roles
                .iter()
                .map(purrdf_shapes::term::NamedNode::as_str)
                .collect();
            d.set_item("source_box_roles", roles)?;
        }
        if !r.path_box_roles.is_empty() {
            let roles: Vec<&str> = r
                .path_box_roles
                .iter()
                .map(purrdf_shapes::term::NamedNode::as_str)
                .collect();
            d.set_item("path_box_roles", roles)?;
        }
        if !r.result_box_roles.is_empty() {
            let roles: Vec<&str> = r
                .result_box_roles
                .iter()
                .map(purrdf_shapes::term::NamedNode::as_str)
                .collect();
            d.set_item("result_box_roles", roles)?;
        }
        results.append(d)?;
    }
    out.set_item("results", results)?;

    Ok(out.into_any().unbind())
}

/// Parsed SHACL shapes that can be reused to validate multiple data graphs.
///
/// Construct from a Turtle shapes graph with `PyShapes(shapes_ttl)`, then call
/// `validate_nt(data_nt)` for each data graph. The Rust orchestration path in
/// `purrdf-validate` borrows the parsed shapes via [`Self::validate_against_dataset`].
#[pyclass(name = "Shapes")]
#[derive(Debug)]
pub struct PyShapes {
    inner: purrdf_shapes::shapes::Shapes,
}

impl PyShapes {
    /// Validate a borrowed native [`RdfDataset`] against these parsed shapes.
    ///
    /// This is the Rust-side primitive used by `purrdf-validate::PyValidationStore`
    /// so the data store does not have to be re-serialized to N-Triples.
    pub fn validate_against_dataset(&self, data: &RdfDataset) -> ValidationReport {
        engine::validate_dataset(data, &self.inner)
            .expect("validation over a frozen dataset is infallible")
    }
}

#[pymethods]
impl PyShapes {
    #[new]
    fn new(py: Python<'_>, shapes_ttl: &str) -> PyResult<Self> {
        // Shapes-graph parsing runs detached (GIL released).
        let inner = py
            .detach(|| engine::parse_shapes(shapes_ttl))
            .map_err(pyo3::exceptions::PyValueError::new_err)?;
        Ok(Self { inner })
    }

    /// Validate an N-Triples data graph against these parsed shapes.
    fn validate_nt(&self, py: Python<'_>, data_nt: &str) -> PyResult<PyValidationReport> {
        // Native codec ingest: lenient on private-use language tags, every
        // malformed line reported in one pass. The engine runs over the frozen IR.
        // Both the ingest and the validation run detached (GIL released).
        let shapes = &self.inner;
        let report = py.detach(|| {
            let data = purrdf_shapes::text_ingest::parse_ntriples_to_dataset(data_nt)
                .map_err(|errors| pyo3::exceptions::PyValueError::new_err(errors.join("\n")))?;
            engine::validate_dataset(data.as_ref(), shapes)
                .map_err(pyo3::exceptions::PyValueError::new_err)
        })?;
        Ok(PyValidationReport::new(report))
    }

    /// Validate a borrowed native dataset against these parsed shapes.
    ///
    /// `data` must be an object (typically `purrdf_validate.ValidationStore`) that
    /// exposes an internal `_store_capsule()` method returning a capsule borrowing a
    /// frozen `Arc<RdfDataset>` snapshot. This avoids serialising the store to
    /// N-Triples for each validation phase.
    ///
    /// # Errors
    ///
    /// Returns `AttributeError` if `data` has no `_store_capsule` method, and
    /// `ValueError` if the capsule cannot be read.
    fn validate_store(&self, data: &Bound<'_, PyAny>) -> PyResult<PyValidationReport> {
        let capsule = data.call_method0("_store_capsule")?;
        let capsule = capsule.cast::<PyCapsule>()?;
        let ptr = capsule
            .pointer_checked(Some(c"purrdf-validation-dataset"))
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let addr = unsafe { *ptr.cast::<usize>().as_ptr() };
        // SAFETY: the capsule's value is the address of an `Arc<RdfDataset>` the
        // producer keeps alive (and at a stable address) for the capsule's lifetime.
        // We clone the `Arc` (extending the dataset's lifetime past the capsule
        // borrow) so validation can run detached (GIL released) without touching
        // any py-bound value.
        let dataset = Arc::clone(unsafe { &*(addr as *const Arc<RdfDataset>) });
        let report = data
            .py()
            .detach(|| self.validate_against_dataset(dataset.as_ref()));
        Ok(PyValidationReport::new(report))
    }
}

/// A SHACL validation report.
///
/// Wraps the Rust [`crate::report::ValidationReport`] and exposes `conforms`,
/// the list of result dicts, and a canonical N-Triples serialization.
#[pyclass(name = "ValidationReport")]
#[derive(Debug)]
pub struct PyValidationReport {
    inner: ValidationReport,
}

impl PyValidationReport {
    /// Construct from a Rust [`crate::report::ValidationReport`].
    pub fn new(inner: ValidationReport) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyValidationReport {
    #[getter]
    fn conforms(&self) -> bool {
        self.inner.conforms
    }

    #[getter]
    fn results(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        for r in &self.inner.results {
            let d = PyDict::new(py);
            d.set_item("focus", r.focus_node.to_string())?;
            d.set_item("path", r.result_path.as_ref().map(ToString::to_string))?;
            d.set_item("value", r.value.as_ref().map(ToString::to_string))?;
            d.set_item("severity", r.severity.iri())?;
            d.set_item("component", r.source_constraint_component.as_str())?;
            d.set_item("source_shape", r.source_shape.to_string())?;
            d.set_item("message", r.message.clone())?;
            if !r.source_box_roles.is_empty() {
                let roles: Vec<&str> = r
                    .source_box_roles
                    .iter()
                    .map(purrdf_shapes::term::NamedNode::as_str)
                    .collect();
                d.set_item("source_box_roles", roles)?;
            }
            if !r.path_box_roles.is_empty() {
                let roles: Vec<&str> = r
                    .path_box_roles
                    .iter()
                    .map(purrdf_shapes::term::NamedNode::as_str)
                    .collect();
                d.set_item("path_box_roles", roles)?;
            }
            if !r.result_box_roles.is_empty() {
                let roles: Vec<&str> = r
                    .result_box_roles
                    .iter()
                    .map(purrdf_shapes::term::NamedNode::as_str)
                    .collect();
                d.set_item("result_box_roles", roles)?;
            }
            list.append(d)?;
        }
        Ok(list.into_any().unbind())
    }

    /// Serialize the report to canonical N-Triples (runs detached — GIL released).
    fn to_ntriples(&self, py: Python<'_>) -> String {
        let report = &self.inner;
        py.detach(|| report.to_ntriples())
    }

    /// Serialize the report to a SARIF 2.1.0 JSON string (runs detached — GIL
    /// released). The sibling of [`to_ntriples`](Self::to_ntriples): the RDF
    /// report graph stays canonical N-Triples, while SARIF is the source-traced,
    /// actionable surface for editors and CI. Logical locations are emitted here
    /// (focus node, result path, constraint component, source shape); physical
    /// source spans require a span-tracked parse.
    fn to_sarif(&self, py: Python<'_>) -> String {
        let report = &self.inner;
        py.detach(|| {
            purrdf_validate::report_to_sarif_string(report, &purrdf_validate::SarifOptions::default())
        })
    }
}

/// Register the `purrdf-shapes` surface on a Python module.
///
/// Exposes the legacy `validate(shapes_ttl, data_nt)` function and the reusable
/// `Shapes` / `ValidationReport` wrappers used by the Rust-native orchestration
/// in `purrdf-validate`. Called by the unified `purrdf_native` cdylib to
/// populate the `purrdf_native.shacl` submodule.
pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(validate, m)?)?;
    m.add_class::<PyShapes>()?;
    m.add_class::<PyValidationReport>()?;
    Ok(())
}
