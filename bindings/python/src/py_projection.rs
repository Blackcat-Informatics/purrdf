// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Thin Python wrappers over the unified projection archive surface.

use std::sync::Arc;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyString};

use crate::py_gts_dataset::PyRdfDataset;
use crate::py_store::PyRdfFormat;
use crate::{
    LiftProfile, LossEntry, LossLedger, ProjectionConfig, ProjectionProfile, RdfDataset,
    lift_archive, parse_dataset, project_archive,
};

/// One immutable, structured runtime loss record.
#[pyclass(name = "ProjectionLoss", frozen, skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct PyProjectionLoss {
    code: String,
    source: String,
    target: String,
    note: String,
    location: Option<String>,
}

#[pymethods]
impl PyProjectionLoss {
    /// Stable machine-readable loss code.
    #[getter]
    fn code(&self) -> &str {
        &self.code
    }

    /// Source representation name.
    #[getter]
    fn source(&self) -> &str {
        &self.source
    }

    /// Target representation name.
    #[getter]
    fn target(&self) -> &str {
        &self.target
    }

    /// Human-readable loss explanation.
    #[getter]
    fn note(&self) -> &str {
        &self.note
    }

    /// Stable rendered source location, when the engine located the loss.
    #[getter]
    fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }
}

impl From<&LossEntry> for PyProjectionLoss {
    fn from(entry: &LossEntry) -> Self {
        Self {
            code: entry.code.to_string(),
            source: entry.from.to_string(),
            target: entry.to.to_string(),
            note: entry.note.to_string(),
            location: entry.location.as_ref().map(|location| location.display()),
        }
    }
}

/// Immutable deterministic projection archive and structured loss records.
#[pyclass(name = "ProjectionPackage", frozen)]
#[derive(Debug)]
pub struct PyProjectionPackage {
    profile: String,
    archive: Vec<u8>,
    losses: Vec<PyProjectionLoss>,
}

#[pymethods]
impl PyProjectionPackage {
    /// Stable projection profile name.
    #[getter]
    fn profile(&self) -> &str {
        &self.profile
    }

    /// Canonical deterministic USTAR bytes.
    #[getter]
    fn archive<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.archive)
    }

    /// Fresh Python list of immutable structured loss records.
    #[getter]
    fn losses(&self, py: Python<'_>) -> PyResult<Vec<Py<PyProjectionLoss>>> {
        self.losses
            .iter()
            .cloned()
            .map(|loss| Py::new(py, loss))
            .collect()
    }
}

/// Immutable result of lifting a strict carrier into a frozen RDF dataset.
#[pyclass(name = "ProjectionLift", frozen)]
#[derive(Debug)]
pub struct PyProjectionLift {
    dataset: Arc<RdfDataset>,
    losses: Vec<PyProjectionLoss>,
}

#[pymethods]
impl PyProjectionLift {
    /// Frozen RDF 1.2 dataset handle.
    #[getter]
    fn dataset(&self, py: Python<'_>) -> PyResult<Py<PyRdfDataset>> {
        Py::new(py, PyRdfDataset::from_arc(Arc::clone(&self.dataset)))
    }

    /// Fresh Python list of immutable structured loss records.
    #[getter]
    fn losses(&self, py: Python<'_>) -> PyResult<Vec<Py<PyProjectionLoss>>> {
        self.losses
            .iter()
            .cloned()
            .map(|loss| Py::new(py, loss))
            .collect()
    }
}

/// Project RDF bytes into a deterministic graph/tabular USTAR package.
#[pyfunction(name = "project")]
#[pyo3(signature = (data, *, format, profile, config))]
fn project_py(
    py: Python<'_>,
    data: &Bound<'_, PyAny>,
    format: PyRdfFormat,
    profile: &str,
    config: &Bound<'_, PyAny>,
) -> PyResult<PyProjectionPackage> {
    let data = read_bytes(data, "data")?;
    let config_bytes = read_bytes(config, "config")?;
    let profile = profile
        .parse::<ProjectionProfile>()
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let config = ProjectionConfig::from_json(&config_bytes)
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let outcome = py
        .detach(move || {
            let dataset = parse_dataset(&data, format.to_native().media_type(), None)
                .map_err(|error| error.to_string())?;
            project_archive(dataset.as_ref(), profile, &config).map_err(|error| error.to_string())
        })
        .map_err(PyValueError::new_err)?;
    Ok(PyProjectionPackage {
        profile: outcome.profile.as_str().to_owned(),
        archive: outcome.archive,
        losses: losses(&outcome.loss_ledger),
    })
}

/// Lift a strict bidirectional USTAR package into a frozen RDF 1.2 dataset.
#[pyfunction(name = "lift")]
#[pyo3(signature = (archive, *, profile, config))]
fn lift_py(
    py: Python<'_>,
    archive: &[u8],
    profile: &str,
    config: &Bound<'_, PyAny>,
) -> PyResult<PyProjectionLift> {
    let archive = archive.to_vec();
    let config_bytes = read_bytes(config, "config")?;
    let profile = profile
        .parse::<LiftProfile>()
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let config = ProjectionConfig::from_json(&config_bytes)
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let outcome = py
        .detach(move || lift_archive(&archive, profile, &config).map_err(|error| error.to_string()))
        .map_err(PyValueError::new_err)?;
    Ok(PyProjectionLift {
        dataset: outcome.dataset,
        losses: losses(&outcome.loss_ledger),
    })
}

fn losses(ledger: &LossLedger) -> Vec<PyProjectionLoss> {
    ledger.entries().iter().map(Into::into).collect()
}

fn read_bytes(value: &Bound<'_, PyAny>, name: &str) -> PyResult<Vec<u8>> {
    if let Ok(bytes) = value.cast::<PyBytes>() {
        return Ok(bytes.as_bytes().to_vec());
    }
    if let Ok(text) = value.cast::<PyString>() {
        return Ok(text.to_str()?.as_bytes().to_vec());
    }
    Err(PyValueError::new_err(format!(
        "{name} must be bytes or str"
    )))
}

/// Register projection classes and functions on the root RDF module.
pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyProjectionLoss>()?;
    m.add_class::<PyProjectionPackage>()?;
    m.add_class::<PyProjectionLift>()?;
    m.add_function(wrap_pyfunction!(project_py, m)?)?;
    m.add_function(wrap_pyfunction!(lift_py, m)?)?;
    Ok(())
}
