// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `PyRdfDataset` — a Python handle to a frozen, immutable [`crate::RdfDataset`]
//! (C7 foundation).
//!
//! This pyclass wraps an `Arc<RdfDataset>` so a parsed RDF artifact can cross the
//! FFI boundary ONCE (as bytes), be frozen into the validated IR, and then be
//! consumed natively (count, GTS emission) WITHOUT re-serializing back to text.
//! A later commit (C7) migrates the text-exchange call sites onto this
//! handle so the producer/consumer seam stops round-tripping through N-Quads.
//!
//! Construction parses Turtle/N-Quads/TriG through the PyO3-free
//! [`crate::dataset_from_bytes`] helper so Python and Rust ingress share one
//! concrete-IR path.

use std::sync::Arc;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyString};

use crate::py_store::PyRdfFormat;
use crate::{NativeRdfFormat, RdfDataset, RdfLookaside, dataset_from_bytes, gts_write};

/// A Python handle to a frozen [`RdfDataset`].
#[pyclass(name = "RdfDataset", frozen)]
#[derive(Debug)]
pub struct PyRdfDataset {
    inner: Arc<RdfDataset>,
}

#[pymethods]
impl PyRdfDataset {
    /// Build a frozen dataset by parsing RDF `data` (bytes or str) in `format`.
    #[new]
    #[pyo3(signature = (data, format))]
    fn new(py: Python<'_>, data: &Bound<'_, PyAny>, format: PyRdfFormat) -> PyResult<Self> {
        let bytes = read_bytes(data)?;
        let inner = py
            .detach(|| dataset_from_bytes(&bytes, rdf_format(format)))
            .map_err(PyValueError::new_err)?;
        Ok(Self { inner })
    }

    /// The number of deduplicated quads.
    fn quad_count(&self) -> usize {
        self.inner.quad_count()
    }

    /// The number of distinct interned terms.
    fn term_count(&self) -> usize {
        self.inner.term_count()
    }

    fn __len__(&self) -> usize {
        self.inner.quad_count()
    }

    /// Emit a GTS byte stream for this dataset under `profile`. Uses the
    /// [`gts_write`] encoder (separate `terms`/`quads`/`reifies`/`annot` frames via
    /// `Writer::deterministic`); the folded graph is semantically identical to the
    /// snapshot-frame producer (the SEMANTIC-FOLD gate, STEP 1).
    #[pyo3(signature = (profile="dist"))]
    fn to_gts(&self, py: Python<'_>, profile: &str) -> PyResult<Py<PyBytes>> {
        // A bare dataset carries no out-of-band envelope, so the lookaside is empty
        // (matching the prior compat-bridge behavior, which yielded an empty lookaside).
        let dataset = &self.inner;
        let bytes = py
            .detach(|| gts_write::to_gts(dataset.as_ref(), &RdfLookaside::default(), profile))
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(PyBytes::new(py, &bytes).unbind())
    }
}

fn read_bytes(data: &Bound<'_, PyAny>) -> PyResult<Vec<u8>> {
    if let Ok(bytes) = data.cast::<PyBytes>() {
        return Ok(bytes.as_bytes().to_vec());
    }
    if let Ok(text) = data.cast::<PyString>() {
        return Ok(text.to_str()?.as_bytes().to_vec());
    }
    Err(PyValueError::new_err("data must be bytes or str"))
}

fn rdf_format(format: PyRdfFormat) -> NativeRdfFormat {
    format.to_native()
}

// PyRdfDataset is registered via `py_gts::register`; no standalone `register` here
// beyond the class add, which `register` performs.
pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyRdfDataset>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dataset_from_bytes_counts_quads() {
        let nt = "<https://e/s> <https://e/p> <https://e/o> .\n\
                  <https://e/s> <https://e/p2> \"lit\" .\n";
        let ds = dataset_from_bytes(nt.as_bytes(), NativeRdfFormat::NTriples).expect("build");
        assert_eq!(ds.quad_count(), 2);
        assert!(ds.term_count() >= 4);
    }

    #[test]
    fn dataset_from_bytes_classifies_rdf12_statement_layer() {
        // A reifier's reifies binding + an annotation: the base quad table is
        // empty, the reifier binding and annotation land in their own tables.
        let nt = concat!(
            "<https://e/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ",
            "<<( <https://e/s> <https://e/p> <https://e/o> )>> .\n",
            "<https://e/r> <https://e/confidence> \"0.9\" .\n",
        );
        let ds = dataset_from_bytes(nt.as_bytes(), NativeRdfFormat::NTriples).expect("build");
        assert_eq!(ds.quad_count(), 0, "reifier rows are not base quads");
        assert_eq!(ds.reifiers().count(), 1);
        assert_eq!(ds.annotations().count(), 1);
    }

    #[test]
    fn dataset_to_gts_folds_back() {
        let nt = "<https://e/s> <https://e/p> <https://e/o> .\n";
        let ds = dataset_from_bytes(nt.as_bytes(), NativeRdfFormat::NTriples).expect("build");
        let bytes =
            gts_write::to_gts(ds.as_ref(), &RdfLookaside::default(), "dist").expect("to_gts");
        let graph = purrdf_gts::reader::read(&bytes, false, None);
        assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
        assert_eq!(graph.quads.len(), 1);
    }
}
