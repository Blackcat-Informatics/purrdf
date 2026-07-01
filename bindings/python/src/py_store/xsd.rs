// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Python helpers for the native XSD value space.

use std::cmp::Ordering;

use pyo3::prelude::*;

/// Compare two XSD lexical values by value space.
///
/// Returns `-1`, `0`, or `1` when both datatypes are supported and comparable.
/// Returns `None` for unsupported datatypes, malformed lexicals, or spec-level
/// incomparability such as `NaN`.
#[pyfunction]
pub(crate) fn xsd_value_compare(
    left_lexical: &str,
    left_datatype: &str,
    right_lexical: &str,
    right_datatype: &str,
) -> Option<i8> {
    let left = purrdf_xsd::parse_by_iri(left_lexical, left_datatype)
        .ok()
        .flatten()?;
    let right = purrdf_xsd::parse_by_iri(right_lexical, right_datatype)
        .ok()
        .flatten()?;
    match purrdf_xsd::value_cmp(&left, &right)? {
        Ordering::Less => Some(-1),
        Ordering::Equal => Some(0),
        Ordering::Greater => Some(1),
    }
}

/// Return the canonical lexical form of a supported XSD lexical value.
///
/// Returns `None` for unsupported datatypes or malformed lexicals.
#[pyfunction]
pub(crate) fn xsd_canonical_lexical(lexical: &str, datatype: &str) -> Option<String> {
    purrdf_xsd::parse_by_iri(lexical, datatype)
        .ok()
        .flatten()
        .map(|value| value.canonical_lexical())
}
