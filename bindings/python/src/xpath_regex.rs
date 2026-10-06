// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The dated XPath regular-expression law a Python caller selects by name.
//!
//! Every entry point that evaluates a SPARQL `REGEX`/`REPLACE`, a SHACL
//! `sh:pattern` or a ShEx pattern facet takes the same `xpath_regex` keyword. Its
//! value is a stable law name read by
//! [`Profile::from_name`](purrdf_core::xsd_regex::xpath::Profile::from_name), which
//! matches exactly: no case folding, abbreviation or undated alias names a law, so
//! an unknown name raises `ValueError` listing the accepted names rather than
//! falling back to a guess. `None` keeps the compatibility pattern behaviour
//! unchanged. A selected law runs under the finite production bounds of
//! [`Limits::new`].

use purrdf_core::xsd_regex::xpath::{Limits, Profile};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// The Python name of the module constant that lists the accepted law names.
const PROFILES_ATTRIBUTE: &str = "XPATH_REGEX_PROFILES";

/// A selected dated law and the finite bounds it runs under.
pub(crate) type Selection = (Profile, Limits);

/// Read the caller's `xpath_regex` keyword: `None` keeps compatibility behaviour,
/// a stable law name selects that law under [`Limits::new`].
///
/// # Errors
///
/// `ValueError` naming the refused value and every accepted name, for a value
/// that is not exactly one of them.
pub(crate) fn selection(name: Option<&str>) -> PyResult<Option<Selection>> {
    name.map(|name| {
        Profile::from_name(name)
            .map(|profile| (profile, Limits::new()))
            .ok_or_else(|| {
                let accepted: Vec<String> = Profile::ALL
                    .into_iter()
                    .map(|profile| format!("{:?}", profile.name()))
                    .collect();
                PyValueError::new_err(format!(
                    "xpath_regex: unknown XPath regex profile {name:?} (expected one of {})",
                    accepted.join(", ")
                ))
            })
    })
    .transpose()
}

/// Expose the accepted law names, oldest first, as the module's
/// `XPATH_REGEX_PROFILES` tuple.
pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let names: Vec<&'static str> = Profile::ALL.into_iter().map(Profile::name).collect();
    m.add(
        PROFILES_ATTRIBUTE,
        pyo3::types::PyTuple::new(m.py(), names)?,
    )
}
