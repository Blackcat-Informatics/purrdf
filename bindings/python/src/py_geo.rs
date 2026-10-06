// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Geographic profiles and sessions carry the shared Rust codec verbatim.

use pyo3::{exceptions::PyValueError, prelude::*};
use std::sync::Arc;

#[pyclass(name = "GeoProfile", module = "purrdf.geo", frozen)]
pub(crate) struct PyGeoProfile {
    inner: Arc<purrdf_validate::geo::GeoProfile>,
}

#[pymethods]
impl PyGeoProfile {
    #[new]
    #[pyo3(signature = (profile=None))]
    fn new(profile: Option<&str>) -> PyResult<Self> {
        Ok(Self {
            inner: profile
                .map(purrdf_validate::geo::GeoSession::from_profile_str)
                .transpose()
                .map_err(|error| PyValueError::new_err(error.to_string()))?
                .unwrap_or_default()
                .profile_snapshot(),
        })
    }

    /// Canonical strict version-one profile, with explicit limits.
    fn to_json(&self) -> PyResult<String> {
        purrdf_validate::geo::profile_to_string(&self.inner)
            .map_err(|error| PyValueError::new_err(error.to_string()))
    }
}

#[pyclass(name = "GeoSession", module = "purrdf.geo", frozen)]
pub(crate) struct PyGeoSession {
    inner: purrdf_validate::geo::GeoSession,
}

#[pymethods]
impl PyGeoSession {
    #[new]
    #[pyo3(signature = (geo=None))]
    fn new(geo: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        Ok(Self {
            inner: prepare_session(geo)?,
        })
    }

    /// Run the scalar or batch operation named by a strict version-one request.
    /// A typed refusal is returned in its canonical JSON response record.
    fn call(&self, py: Python<'_>, request: &str) -> String {
        py.detach(|| self.inner.call_string(request))
    }

    /// Prepare immutable native ordered point buckets from exact input coordinates.
    fn point_index(&self, py: Python<'_>, request: &str) -> PyResult<PyGeoPointIndex> {
        py.detach(|| self.inner.point_index(request))
            .map(|inner| PyGeoPointIndex { inner })
            .map_err(|error| PyValueError::new_err(error.to_string()))
    }

    fn profile(&self) -> PyGeoProfile {
        PyGeoProfile {
            inner: self.inner.profile_snapshot(),
        }
    }
}

#[pyclass(name = "GeoPointIndex", module = "purrdf.geo", frozen)]
pub(crate) struct PyGeoPointIndex {
    inner: purrdf_validate::geo::GeoPointIndex,
}

#[pymethods]
impl PyGeoPointIndex {
    /// Refine reusable ordered buckets using a strict physical/reported request.
    fn call(&self, py: Python<'_>, request: &str) -> String {
        py.detach(|| self.inner.call_string(request))
    }
}

fn prepare_session(geo: Option<&Bound<'_, PyAny>>) -> PyResult<purrdf_validate::geo::GeoSession> {
    let Some(geo) = geo else {
        return Ok(purrdf_validate::geo::GeoSession::default());
    };
    if let Ok(profile) = geo.extract::<PyRef<'_, PyGeoProfile>>() {
        return purrdf_validate::geo::GeoSession::try_from_profile(
            &profile.inner,
            purrdf::geo::ExecutionPolicy::geometry(),
        )
        .map_err(|error| PyValueError::new_err(error.to_string()));
    }
    if let Ok(session) = geo.extract::<PyRef<'_, PyGeoSession>>() {
        return Ok(session.inner.clone());
    }
    let text: &str = geo.extract().map_err(|_| {
        PyValueError::new_err("geo must be a GeoProfile, GeoSession or strict profile JSON string")
    })?;
    purrdf_validate::geo::GeoSession::from_profile_str(text)
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

fn decode_text(text: Option<&str>) -> PyResult<purrdf_validate::geo::GeoProfile> {
    text.map(purrdf_validate::geo::profile_from_str)
        .transpose()
        .map_err(|error| PyValueError::new_err(error.to_string()))
        .map(Option::unwrap_or_default)
}

/// Convert Python configuration under the GIL, before Rust computation detaches.
pub(crate) fn decode(geo: Option<&Bound<'_, PyAny>>) -> PyResult<purrdf_validate::geo::GeoProfile> {
    let Some(geo) = geo else {
        return Ok(purrdf_validate::geo::GeoProfile::standard());
    };
    if let Ok(profile) = geo.extract::<PyRef<'_, PyGeoProfile>>() {
        return copy_profile(&profile.inner);
    }
    if let Ok(session) = geo.extract::<PyRef<'_, PyGeoSession>>() {
        return copy_profile(session.inner.profile());
    }
    let text: &str = geo.extract().map_err(|_| {
        PyValueError::new_err("geo must be a GeoProfile, GeoSession or strict profile JSON string")
    })?;
    decode_text(Some(text))
}

fn copy_profile(
    profile: &purrdf_validate::geo::GeoProfile,
) -> PyResult<purrdf_validate::geo::GeoProfile> {
    profile
        .clone_in_budget(&mut purrdf::geo::PreparationBudget::new(
            purrdf::geo::ExecutionPolicy::geometry(),
        ))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyGeoProfile>()?;
    module.add_class::<PyGeoSession>()?;
    module.add_class::<PyGeoPointIndex>()?;
    Ok(())
}
