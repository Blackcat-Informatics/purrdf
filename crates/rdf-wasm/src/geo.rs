// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The shared strict geographic session, with no JavaScript mathematics.

use wasm_bindgen::prelude::*;

use crate::operation::{OPTIONS_CODE, coded_error};

/// Immutable geographic profile and scalar/batch request session.
#[wasm_bindgen(js_name = GeoSession)]
#[derive(Debug, Default)]
pub struct GeoSession {
    inner: purrdf_validate::geo::GeoSession,
}

#[wasm_bindgen(js_class = GeoSession)]
impl GeoSession {
    /// Prepare an explicit version-one profile, or the standard CRS84 profile.
    ///
    /// # Errors
    ///
    /// Refuses malformed records, conflicting references and invalid limits.
    #[wasm_bindgen(constructor)]
    pub fn new(profile: Option<String>) -> Result<Self, JsValue> {
        let inner = profile
            .map(|text| purrdf_validate::geo::GeoSession::from_profile_str(&text))
            .transpose()
            .map_err(|error| coded_error(&error.to_string(), OPTIONS_CODE))?
            .unwrap_or_default();
        Ok(Self { inner })
    }

    /// Prepare immutable native ordered point buckets from a strict request.
    ///
    /// # Errors
    /// Refuses invalid source records, reference mismatch and incomplete admission.
    #[wasm_bindgen(js_name = pointIndex)]
    pub fn point_index(&self, request: &str) -> Result<GeoPointIndex, JsValue> {
        Ok(GeoPointIndex {
            inner: self
                .inner
                .point_index(request)
                .map_err(|error| coded_error(&error.to_string(), OPTIONS_CODE))?,
        })
    }

    /// Return the canonical strict profile record.
    ///
    /// # Errors
    ///
    /// Refuses a native custom parameter without an exact decimal representation.
    pub fn profile(&self) -> Result<String, JsValue> {
        purrdf_validate::geo::profile_to_string(self.inner.profile())
            .map_err(|error| coded_error(&error.to_string(), OPTIONS_CODE))
    }

    /// Execute a strict version-one scalar/batch request and return its response.
    /// Typed engine refusals remain JSON records alongside the same identities.
    pub fn call(&self, request: &str) -> String {
        self.inner.call_string(request)
    }
}

impl GeoSession {
    pub(crate) const fn native(&self) -> &purrdf_validate::geo::GeoSession {
        &self.inner
    }
}

/// Reusable immutable native point-cell index with strict search requests.
#[wasm_bindgen(js_name = GeoPointIndex)]
#[derive(Debug)]
pub struct GeoPointIndex {
    inner: purrdf_validate::geo::GeoPointIndex,
}

#[wasm_bindgen(js_class = GeoPointIndex)]
impl GeoPointIndex {
    /// Search stored buckets using the exact selected public comparison law.
    pub fn call(&self, request: &str) -> String {
        self.inner.call_string(request)
    }
}
