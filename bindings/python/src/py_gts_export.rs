// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit admission policy and owned reports for GTS columnar exports.

use purrdf::{GtsBlobExportReport, GtsBlobLimits, GtsBlobOmission, GtsBlobUsage};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use std::sync::Arc;

/// Caller-owned byte policy. Every field is explicit; None means unbounded.
#[pyclass(name = "GtsBlobLimits", frozen)]
#[derive(Debug, Clone, Copy)]
pub struct PyGtsBlobLimits {
    inner: GtsBlobLimits,
}

purrdf_lex::constructors! {
    impl PyGtsBlobLimits { pub(crate) fn from_native(inner: GtsBlobLimits) -> Self { .. }; }
}

const fn visible_limit(value: usize) -> Option<usize> {
    if value == usize::MAX {
        None
    } else {
        Some(value)
    }
}

#[pymethods]
impl PyGtsBlobLimits {
    #[new]
    #[pyo3(signature = (*, max_encoded_bytes, max_decoded_bytes, max_total_decoded_bytes, max_metadata_bytes, max_frame_decoded_bytes))]
    fn new(
        max_encoded_bytes: Option<usize>,
        max_decoded_bytes: Option<usize>,
        max_total_decoded_bytes: Option<usize>,
        max_metadata_bytes: Option<usize>,
        max_frame_decoded_bytes: Option<usize>,
    ) -> Self {
        let mut inner = GtsBlobLimits::UNBOUNDED;
        inner.max_encoded_bytes = max_encoded_bytes.unwrap_or(usize::MAX);
        inner.max_decoded_bytes = max_decoded_bytes.unwrap_or(usize::MAX);
        inner.max_total_decoded_bytes = max_total_decoded_bytes.unwrap_or(usize::MAX);
        inner.max_metadata_bytes = max_metadata_bytes.unwrap_or(usize::MAX);
        inner.max_frame_decoded_bytes = max_frame_decoded_bytes.unwrap_or(usize::MAX);
        Self { inner }
    }
    /// No chosen byte ceiling; storage and address-space failures remain errors.
    #[staticmethod]
    fn unbounded() -> Self {
        Self::from_native(GtsBlobLimits::UNBOUNDED)
    }
    #[getter]
    const fn max_encoded_bytes(&self) -> Option<usize> {
        visible_limit(self.inner.max_encoded_bytes)
    }
    #[getter]
    const fn max_decoded_bytes(&self) -> Option<usize> {
        visible_limit(self.inner.max_decoded_bytes)
    }
    #[getter]
    const fn max_total_decoded_bytes(&self) -> Option<usize> {
        visible_limit(self.inner.max_total_decoded_bytes)
    }
    #[getter]
    const fn max_metadata_bytes(&self) -> Option<usize> {
        visible_limit(self.inner.max_metadata_bytes)
    }
    #[getter]
    const fn max_frame_decoded_bytes(&self) -> Option<usize> {
        visible_limit(self.inner.max_frame_decoded_bytes)
    }
}

/// Exact logical admission charges, excluding dataset/frame/index owners.
#[pyclass(name = "GtsBlobUsage", frozen)]
#[derive(Debug, Clone, Copy)]
pub struct PyGtsBlobUsage {
    inner: GtsBlobUsage,
}

purrdf_lex::constructors! {
    impl PyGtsBlobUsage { pub(crate) fn from_native(inner: GtsBlobUsage) -> Self { .. }; }
}

#[pymethods]
impl PyGtsBlobUsage {
    #[getter]
    const fn retained_payload_bytes(&self) -> usize {
        self.inner.retained_payload_bytes
    }
    #[getter]
    const fn retained_refusal_metadata_bytes(&self) -> usize {
        self.inner.retained_refusal_metadata_bytes
    }
    #[getter]
    const fn peak_payload_bytes(&self) -> usize {
        self.inner.peak_payload_bytes
    }
    #[getter]
    const fn peak_retained_bytes(&self) -> usize {
        self.inner.peak_retained_bytes
    }
    #[getter]
    const fn peak_selected_digests(&self) -> usize {
        self.inner.peak_selected_digests
    }
}

/// Immutable original omission evidence, keeping its native report alive.
#[pyclass(name = "GtsBlobOmission", frozen)]
#[derive(Debug)]
pub struct PyGtsBlobOmission {
    owner: Arc<GtsBlobExportReport>,
    index: usize,
}

impl PyGtsBlobOmission {
    fn inner(&self) -> &GtsBlobOmission {
        // Only the report's checked enumerate constructs this private index.
        &self.owner.omissions()[self.index]
    }
}

#[pymethods]
impl PyGtsBlobOmission {
    #[getter]
    fn reason(&self) -> &'static str {
        self.inner().kind().code()
    }
    #[getter]
    fn digest(&self) -> Option<&str> {
        self.inner().digest()
    }
    #[getter]
    fn digest_computed(&self) -> bool {
        self.inner().digest_computed()
    }
    #[getter]
    fn encoded_bytes(&self) -> usize {
        self.inner().encoded_bytes()
    }
    #[getter]
    fn segment_index(&self) -> u64 {
        self.inner().source().segment_index
    }
    #[getter]
    fn frame_index(&self) -> u64 {
        self.inner().source().frame_index
    }
    #[getter]
    fn frame_range(&self) -> (u64, u64) {
        let range = &self.inner().source().frame_range;
        (range.start, range.end)
    }
    #[getter]
    fn frame_id<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner().source().frame_id)
    }
    #[getter]
    fn segment_head<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner().source().segment_head)
    }
    #[getter]
    fn limits(&self) -> PyGtsBlobLimits {
        PyGtsBlobLimits::from_native(self.inner().limits())
    }
    #[getter]
    fn usage(&self) -> PyGtsBlobUsage {
        PyGtsBlobUsage::from_native(self.inner().usage())
    }
    /// Fresh structured snapshot of the original cause, never its rendered text.
    #[getter]
    fn native_cause<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyDict>>> {
        self.inner()
            .native_cause()
            .map(|cause| crate::py_native_fields::inspect(py, cause))
            .transpose()
    }
}

/// Every export returns this report, including lossless exports.
#[pyclass(name = "GtsBlobExportReport", frozen)]
#[derive(Debug)]
pub struct PyGtsBlobExportReport {
    inner: Arc<GtsBlobExportReport>,
}

purrdf_lex::constructors! {
    impl PyGtsBlobExportReport { pub(crate) fn from_native(inner: impl Into<Arc<GtsBlobExportReport>>) -> Self { .. }; }
}

#[pymethods]
impl PyGtsBlobExportReport {
    #[getter]
    fn losses(&self, py: Python<'_>) -> PyResult<Vec<Py<crate::py_projection::PyProjectionLoss>>> {
        crate::py_projection::ledger_losses(py, self.inner.losses())
    }
    #[getter]
    fn omissions(&self) -> Vec<PyGtsBlobOmission> {
        self.inner
            .omissions()
            .iter()
            .enumerate()
            .map(|(index, _)| PyGtsBlobOmission {
                owner: Arc::clone(&self.inner),
                index,
            })
            .collect()
    }
    #[getter]
    fn usage(&self) -> PyGtsBlobUsage {
        PyGtsBlobUsage::from_native(self.inner.usage())
    }
}

fn limits_from(value: &Bound<'_, PyAny>) -> PyResult<GtsBlobLimits> {
    if let Ok(limits) = value.extract::<PyRef<'_, PyGtsBlobLimits>>() {
        return Ok(limits.inner);
    }
    if value
        .extract::<&str>()
        .is_ok_and(|value| value == "unbounded")
    {
        return Ok(GtsBlobLimits::UNBOUNDED);
    }
    Err(PyTypeError::new_err(
        "blob_limits must be GtsBlobLimits or the literal 'unbounded'",
    ))
}

/// Canonical rows and their complete native admission report.
#[pyfunction]
#[pyo3(signature = (data, *, blob_limits))]
fn gts_columnar_rows_from_bytes<'py>(
    py: Python<'py>,
    data: &[u8],
    blob_limits: &Bound<'_, PyAny>,
) -> PyResult<(Bound<'py, PyDict>, PyGtsBlobExportReport)> {
    let limits = limits_from(blob_limits)?;
    let (projection, report) = py
        .detach(|| purrdf::prepare_gts_columnar(data, limits)?.project())
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok((
        crate::py_gts_view::columnar_rows_dict(py, &projection)?,
        PyGtsBlobExportReport::from_native(report),
    ))
}

/// Native file bytes and their complete native admission/writer loss report.
#[pyfunction]
#[pyo3(signature = (data, *, blob_limits))]
fn gts_columnar_parquet_from_bytes<'py>(
    py: Python<'py>,
    data: &[u8],
    blob_limits: &Bound<'_, PyAny>,
) -> PyResult<(Bound<'py, PyDict>, PyGtsBlobExportReport)> {
    let limits = limits_from(blob_limits)?;
    let (files, report) = py
        .detach(|| {
            purrdf::prepare_gts_columnar(data, limits)?
                .write(purrdf_columnar::Compression::Uncompressed)
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok((
        crate::py_gts_view::columnar_parquet_dict(py, &files)?,
        PyGtsBlobExportReport::from_native(report),
    ))
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyGtsBlobLimits>()?;
    m.add_class::<PyGtsBlobUsage>()?;
    m.add_class::<PyGtsBlobOmission>()?;
    m.add_class::<PyGtsBlobExportReport>()?;
    m.add_function(wrap_pyfunction!(gts_columnar_rows_from_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(gts_columnar_parquet_from_bytes, m)?)?;
    Ok(())
}
