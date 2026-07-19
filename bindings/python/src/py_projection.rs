// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Thin Python wrappers over the unified graph, tabular, and research-object archive surface.

use std::sync::Arc;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyString};

use crate::py_gts_dataset::PyRdfDataset;
use crate::py_store::PyRdfFormat;
use crate::{
    LiftProfile, LossEntry, LossLedger, LpgProgress, LpgProgressObserver, LpgProjectionReport,
    ProjectionArtifactSink, ProjectionConfig, ProjectionError, ProjectionProfile, RdfDataset,
    lift_archive, parse_dataset, project_archive, project_lpg_artifacts_to_sink,
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

/// Immutable progress snapshot from one scoped LPG projection.
#[pyclass(name = "ProjectionProgress", frozen, skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct PyProjectionProgress {
    phase: String,
    input_records: usize,
    model_records: usize,
    nodes: usize,
    edges: usize,
    artifacts: usize,
    bytes: usize,
    path: Option<String>,
}

#[pymethods]
impl PyProjectionProgress {
    /// Stable operation phase.
    #[getter]
    fn phase(&self) -> &str {
        &self.phase
    }

    /// RDF input records scanned so far.
    #[getter]
    const fn input_records(&self) -> usize {
        self.input_records
    }

    /// Records retained in the canonical LPG model so far.
    #[getter]
    const fn model_records(&self) -> usize {
        self.model_records
    }

    /// Canonical LPG nodes built so far.
    #[getter]
    const fn nodes(&self) -> usize {
        self.nodes
    }

    /// Canonical LPG edges built so far.
    #[getter]
    const fn edges(&self) -> usize {
        self.edges
    }

    /// Fully finished output artifacts.
    #[getter]
    const fn artifacts(&self) -> usize {
        self.artifacts
    }

    /// Artifact body bytes accepted by the sink so far.
    #[getter]
    const fn bytes(&self) -> usize {
        self.bytes
    }

    /// Active or most recently finished artifact path.
    #[getter]
    fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }
}

impl From<&LpgProgress> for PyProjectionProgress {
    fn from(progress: &LpgProgress) -> Self {
        Self {
            phase: progress.phase.as_str().to_owned(),
            input_records: progress.report.input_records,
            model_records: progress.report.model_records,
            nodes: progress.report.nodes,
            edges: progress.report.edges,
            artifacts: progress.artifacts,
            bytes: progress.bytes,
            path: progress.path.clone(),
        }
    }
}

/// Immutable summary from direct LPG projection into a Python artifact sink.
#[pyclass(name = "ProjectionStream", frozen)]
#[derive(Debug)]
pub struct PyProjectionStream {
    profile: String,
    losses: Vec<PyProjectionLoss>,
    report: LpgProjectionReport,
}

#[pymethods]
impl PyProjectionStream {
    /// Stable projection profile name.
    #[getter]
    fn profile(&self) -> &str {
        &self.profile
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

    /// Exact number of RDF input records scanned.
    #[getter]
    const fn input_records(&self) -> usize {
        self.report.input_records
    }

    /// Exact number of records retained in the canonical LPG model.
    #[getter]
    const fn model_records(&self) -> usize {
        self.report.model_records
    }

    /// Exact number of canonical LPG nodes.
    #[getter]
    const fn nodes(&self) -> usize {
        self.report.nodes
    }

    /// Exact number of canonical LPG edges.
    #[getter]
    const fn edges(&self) -> usize {
        self.report.edges
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

/// Project RDF bytes into a deterministic graph, tabular, or research-object USTAR package.
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

/// Project one LPG profile into lifecycle-delimited artifact chunks.
///
/// The artifact callback receives `(event, path, chunk)`. Events are
/// `begin-package`, `begin-artifact`, `chunk`, `finish-artifact`,
/// `commit-package`, and `abort-package`. The optional progress callback receives
/// immutable [`ProjectionProgress`](PyProjectionProgress) snapshots.
#[pyfunction(name = "project_artifacts")]
#[pyo3(signature = (data, *, format, profile, config, artifact_callback, progress_callback=None))]
fn project_artifacts_py(
    py: Python<'_>,
    data: &Bound<'_, PyAny>,
    format: PyRdfFormat,
    profile: &str,
    config: &Bound<'_, PyAny>,
    artifact_callback: &Bound<'_, PyAny>,
    progress_callback: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyProjectionStream> {
    let data = read_bytes(data, "data")?;
    let config_bytes = read_bytes(config, "config")?;
    let profile = profile
        .parse::<ProjectionProfile>()
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let config = ProjectionConfig::from_json(&config_bytes)
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let dataset = py
        .detach(move || {
            parse_dataset(&data, format.to_native().media_type(), None)
                .map_err(|error| error.to_string())
        })
        .map_err(PyValueError::new_err)?;

    let mut sink = PythonArtifactSink::new(artifact_callback.clone());
    let mut observer = PythonProgressObserver::new(progress_callback.cloned());
    let outcome =
        project_lpg_artifacts_to_sink(dataset.as_ref(), profile, &config, &mut sink, &mut observer);
    match outcome {
        Ok(outcome) => Ok(PyProjectionStream {
            profile: profile.as_str().to_owned(),
            losses: losses(&outcome.loss_ledger),
            report: outcome.report,
        }),
        Err(error) => {
            if let Some(error) = observer.take_callback_error() {
                return Err(error);
            }
            if let Some(error) = sink.take_callback_error() {
                return Err(error);
            }
            Err(PyValueError::new_err(error.to_string()))
        }
    }
}

struct PythonArtifactSink<'py> {
    callback: Bound<'py, PyAny>,
    current_path: Option<String>,
    callback_error: Option<PyErr>,
}

impl<'py> PythonArtifactSink<'py> {
    fn new(callback: Bound<'py, PyAny>) -> Self {
        Self {
            callback,
            current_path: None,
            callback_error: None,
        }
    }

    fn notify(
        &mut self,
        event: &str,
        path: Option<&str>,
        chunk: &[u8],
    ) -> Result<(), ProjectionError> {
        let bytes = PyBytes::new(self.callback.py(), chunk);
        if let Err(error) = self.callback.call1((event, path, bytes)) {
            if self.callback_error.is_none() {
                self.callback_error = Some(error);
            }
            return Err(ProjectionError::integrity(format!(
                "Python artifact callback failed during `{event}`"
            )));
        }
        Ok(())
    }

    fn take_callback_error(&mut self) -> Option<PyErr> {
        self.callback_error.take()
    }
}

impl ProjectionArtifactSink for PythonArtifactSink<'_> {
    fn begin_package(&mut self) -> Result<(), ProjectionError> {
        self.notify("begin-package", None, &[])
    }

    fn begin_artifact(&mut self, path: &str) -> Result<(), ProjectionError> {
        self.current_path = Some(path.to_owned());
        self.notify("begin-artifact", Some(path), &[])
    }

    fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), ProjectionError> {
        let path = self.current_path.clone();
        self.notify("chunk", path.as_deref(), chunk)
    }

    fn finish_artifact(&mut self) -> Result<(), ProjectionError> {
        let path = self.current_path.clone();
        self.notify("finish-artifact", path.as_deref(), &[])?;
        self.current_path = None;
        Ok(())
    }

    fn commit_package(&mut self) -> Result<(), ProjectionError> {
        self.notify("commit-package", None, &[])
    }

    fn abort_package(&mut self) {
        self.current_path = None;
        let bytes = PyBytes::new(self.callback.py(), &[]);
        let _ = self
            .callback
            .call1(("abort-package", Option::<&str>::None, bytes));
    }
}

struct PythonProgressObserver<'py> {
    callback: Option<Bound<'py, PyAny>>,
    callback_error: Option<PyErr>,
}

impl<'py> PythonProgressObserver<'py> {
    const fn new(callback: Option<Bound<'py, PyAny>>) -> Self {
        Self {
            callback,
            callback_error: None,
        }
    }

    fn take_callback_error(&mut self) -> Option<PyErr> {
        self.callback_error.take()
    }
}

impl LpgProgressObserver for PythonProgressObserver<'_> {
    fn observe(&mut self, progress: &LpgProgress) -> Result<(), ProjectionError> {
        let result = self.callback.as_ref().map_or(Ok(()), |callback| {
            let snapshot = Py::new(callback.py(), PyProjectionProgress::from(progress))?;
            callback.call1((snapshot,)).map(|_| ())
        });
        if let Err(error) = result {
            if self.callback_error.is_none() {
                self.callback_error = Some(error);
            }
            return Err(ProjectionError::integrity(
                "Python projection progress callback failed",
            ));
        }
        Ok(())
    }
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
    m.add_class::<PyProjectionProgress>()?;
    m.add_class::<PyProjectionStream>()?;
    m.add_class::<PyProjectionLift>()?;
    m.add_function(wrap_pyfunction!(project_py, m)?)?;
    m.add_function(wrap_pyfunction!(project_artifacts_py, m)?)?;
    m.add_function(wrap_pyfunction!(lift_py, m)?)?;
    Ok(())
}
