// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact source bytes and existing dataset handles over the native YAML carrier.

use purrdf_core::DatasetHandle;
use purrdf_lex::allocation::{Memory, Resident, StorageError};
use purrdf_validate::yaml_source;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use crate::py_gts_dataset::PyRdfDataset;

/// Native source root and its immutable RDF carrier dataset.
#[pyclass(name = "YamlSourceExport", frozen)]
#[derive(Debug)]
pub(crate) struct PyYamlSourceExport {
    root: String,
    dataset: Py<PyRdfDataset>,
}

#[pymethods]
impl PyYamlSourceExport {
    #[getter]
    fn root(&self) -> &str {
        &self.root
    }

    #[getter]
    fn dataset(&self, py: Python<'_>) -> Py<PyRdfDataset> {
        self.dataset.clone_ref(py)
    }
}

fn source_error(py: Python<'_>, error: yaml_source::Error) -> PyErr {
    crate::py_store::presentation::native_error(py, &error.into_diagnostic())
}

/// Export immutable original UTF-8 bytes under an explicit source and profile.
#[pyfunction]
#[pyo3(signature = (source, *, source_iri, profile_json))]
fn yaml_source_export(
    py: Python<'_>,
    source: &Bound<'_, PyBytes>,
    source_iri: &str,
    profile_json: &Bound<'_, PyBytes>,
) -> PyResult<PyYamlSourceExport> {
    // Bound immutable bytes keep their Python owners alive while the GIL is
    // released. No source copy precedes native profile and input admission.
    let bytes = source.as_bytes();
    let config = profile_json.as_bytes();
    let (root, dataset) = py
        .detach(|| {
            let profile = yaml_source::profile(config)?;
            let exported = yaml_source::export(source_iri, bytes, &profile)?;
            let mut resident = Resident;
            Memory::new(&mut resident)
                .add_bytes(DatasetHandle::allocation_layout::<Resident>().size())
                .map_err(yaml_source::YamlError::from)?;
            let dataset = DatasetHandle::try_from_admitted(exported.dataset, resident)
                .map_err(|_| yaml_source::YamlError::from(StorageError::AllocationFailed))?;
            Ok::<_, yaml_source::Error>((exported.root, dataset))
        })
        .map_err(|error| source_error(py, error))?;
    Ok(PyYamlSourceExport {
        root,
        dataset: Py::new(py, PyRdfDataset::from_frozen(dataset))?,
    })
}

/// Reconstruct exact verified source bytes from an existing dataset handle.
#[pyfunction]
#[pyo3(signature = (dataset, *, root, profile_json))]
fn yaml_source_reconstruct<'py>(
    py: Python<'py>,
    dataset: &PyRdfDataset,
    root: &str,
    profile_json: &Bound<'_, PyBytes>,
) -> PyResult<Bound<'py, PyBytes>> {
    let dataset = dataset.dataset();
    let config = profile_json.as_bytes();
    let text = py
        .detach(|| {
            let profile = yaml_source::profile(config)?;
            yaml_source::reconstruct(dataset.as_ref(), root, &profile)
        })
        .map_err(|error| source_error(py, error))?;
    Ok(PyBytes::new(py, text.as_bytes()))
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyYamlSourceExport>()?;
    module.add_function(wrap_pyfunction!(yaml_source_export, module)?)?;
    module.add_function(wrap_pyfunction!(yaml_source_reconstruct, module)?)?;
    Ok(())
}
