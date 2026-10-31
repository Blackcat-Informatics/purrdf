// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One explicit canonical-search boundary and immutable evidence envelope.

use purrdf_core::CanonicalizationControl;
use purrdf_validate::canonical::{execute, native_canonical_cause, parse_policy, receipts_json};
use pyo3::IntoPyObjectExt;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::py_store::PyCancellationToken;

/// The original native cause, retaining its immutable payload and admission owner.
#[pyclass(name = "CanonicalCause", frozen)]
#[derive(Debug)]
pub(crate) struct PyCanonicalCause {
    cause: purrdf_core::CanonError,
}

#[pymethods]
impl PyCanonicalCause {
    /// Exact closed native fields, read from the retained original cause.
    #[getter]
    fn json(&self) -> PyResult<String> {
        purrdf_validate::native_error::to_json(&self.cause)
            .map(|value| value.to_string())
            .map_err(|error| pyo3::exceptions::PyRuntimeError::new_err(error.to_string()))
    }
}

/// An original host value and this operation's immutable canonicalization receipt JSON.
#[pyclass(name = "CanonicalResult", frozen)]
#[derive(Debug)]
pub(crate) struct PyCanonicalResult {
    value: Py<PyAny>,
    canonicalization_json: String,
}

#[pymethods]
impl PyCanonicalResult {
    /// The unchanged value produced by the selected operation.
    #[getter]
    fn value(&self, py: Python<'_>) -> Py<PyAny> {
        self.value.clone_ref(py)
    }

    /// Ordered per-run receipts. Decimal strings preserve all integer bits.
    #[getter]
    fn canonicalization_json(&self) -> &str {
        &self.canonicalization_json
    }
}

pub(crate) fn finish<'py, T>(
    py: Python<'py>,
    value: T,
    canonicalization_json: String,
) -> PyResult<PyCanonicalResult>
where
    T: IntoPyObject<'py>,
    T::Error: Into<PyErr>,
{
    Ok(PyCanonicalResult {
        value: value.into_py_any(py)?,
        canonicalization_json,
    })
}

pub(crate) fn run<T, E>(
    py: Python<'_>,
    canonical_search: &str,
    cancel: Option<&PyCancellationToken>,
    operation: impl FnOnce(&mut CanonicalizationControl<'_>) -> Result<T, E> + Send,
) -> PyResult<(T, String)>
where
    T: Send,
    E: std::error::Error + Send + 'static,
{
    run_with(py, canonical_search, cancel, operation, |_py, error| {
        PyValueError::new_err(error.to_string())
    })
}

/// Map the original typed native refusal after the GIL is reacquired, then
/// attach the same finalized receipts and retained canonical cause as run.
pub(crate) fn run_with<'py, T, E>(
    py: Python<'py>,
    canonical_search: &str,
    cancel: Option<&PyCancellationToken>,
    operation: impl FnOnce(&mut CanonicalizationControl<'_>) -> Result<T, E> + Send,
    map_error: impl FnOnce(Python<'py>, E) -> PyErr,
) -> PyResult<(T, String)>
where
    T: Send,
    E: std::error::Error + Send + 'static,
{
    let policy =
        parse_policy(canonical_search).map_err(|error| error_to_python(py, &error, "[]"))?;
    let executed = py.detach(|| {
        execute(
            policy,
            cancel.map(|signal| signal as &dyn purrdf_core::StopSignal),
            operation,
        )
    });
    let receipts = receipts_json(&executed.receipts).to_string();
    match executed.result {
        Ok(value) => Ok((value, receipts)),
        Err(error) => {
            // Retain the original immutable cause owner before the host mapper
            // consumes its typed error. No rendered string becomes a native cause.
            let cause = native_canonical_cause(&error).cloned();
            let raised = map_error(py, error);
            Err(attach_error(py, raised, cause, &receipts))
        }
    }
}

pub(crate) fn error_to_python(
    py: Python<'_>,
    error: &(dyn std::error::Error + 'static),
    receipts: &str,
) -> PyErr {
    attach_error(
        py,
        PyValueError::new_err(error.to_string()),
        native_canonical_cause(error).cloned(),
        receipts,
    )
}

fn attach_error(
    py: Python<'_>,
    refused: PyErr,
    cause: Option<purrdf_core::CanonError>,
    receipts: &str,
) -> PyErr {
    let attach = || -> PyResult<()> {
        refused
            .value(py)
            .setattr("canonicalization_json", receipts)?;
        if let Some(cause) = cause {
            let fields = purrdf_validate::native_error::to_json(&cause).map_err(|failure| {
                pyo3::exceptions::PyRuntimeError::new_err(failure.to_string())
            })?;
            refused
                .value(py)
                .setattr("native_cause_json", fields.to_string())?;
            refused
                .value(py)
                .setattr("native_cause", Py::new(py, PyCanonicalCause { cause })?)?;
        }
        Ok(())
    };
    match attach() {
        Ok(()) => refused,
        Err(failure) => failure,
    }
}
