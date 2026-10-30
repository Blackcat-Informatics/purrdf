// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//! Shared terms and coordinates of the native embedding query regressions.
use purrdf_core::TermValue;
/// The fixture data namespace.
pub(crate) const EX: &str = "https://example.org/d/";
/// An IRI in the fixture namespace.
pub(crate) fn iri(local: &str) -> TermValue {
    TermValue::iri(format!("{EX}{local}"))
}
/// The three points every test below searches: `a` at the origin, `b` at distance 25,
/// `c` at distance 2500.
pub(crate) fn points() -> Vec<(&'static str, Vec<f64>)> {
    vec![
        ("a", vec![0.0, 0.0]),
        ("b", vec![3.0, 4.0]),
        ("c", vec![30.0, 40.0]),
    ]
}
