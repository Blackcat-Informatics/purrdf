// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Python helpers for the native XSD value space.

use std::cmp::Ordering;

use pyo3::prelude::*;
use pyo3::types::PyBytes;

const XSD_NORMALIZED_STRING: &str = "http://www.w3.org/2001/XMLSchema#normalizedString";
const XSD_TOKEN: &str = "http://www.w3.org/2001/XMLSchema#token";

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

/// Decode an `xsd:hexBinary` or `xsd:base64Binary` lexical form to Python `bytes`.
///
/// Reuses the native zero-dependency codecs (`purrdf_xsd::parse_binary`, dispatching
/// to `parse_hex` / `parse_base64`). Returns `None` — so the Python caller falls back
/// to the lexical string, matching rdflib's `_castLexicalToPython` — when the datatype
/// is not one of the two binary types or the lexical form is malformed for it.
#[pyfunction]
pub(crate) fn xsd_decode_binary<'py>(
    py: Python<'py>,
    lexical: &str,
    datatype: &str,
) -> Option<Bound<'py, PyBytes>> {
    // `parse_binary` hard-errors on any non-binary datatype, so a non-binary IRI
    // (or a malformed lexical) yields `None` via the `?`/`ok()` chain.
    let dt = purrdf_xsd::XsdDatatype::from_iri(datatype)?;
    let bytes = purrdf_xsd::parse_binary(dt, lexical).ok()?;
    Some(PyBytes::new(py, &bytes))
}

/// Apply the XSD `whiteSpace` facet of `xsd:normalizedString` (`replace`) or
/// `xsd:token` (`collapse`) to `lexical`, returning the normalized string.
///
/// Delegates to the native facet functions in `purrdf_xsd`. Returns `None` for any
/// other datatype, so the Python caller keeps the lexical form verbatim for datatypes
/// that carry no whitespace facet.
#[pyfunction]
pub(crate) fn xsd_normalize_whitespace(lexical: &str, datatype: &str) -> Option<String> {
    match datatype {
        XSD_NORMALIZED_STRING => Some(purrdf_xsd::normalize_whitespace_replace(lexical)),
        XSD_TOKEN => Some(purrdf_xsd::normalize_whitespace_collapse(lexical)),
        _ => None,
    }
}
