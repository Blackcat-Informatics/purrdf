// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The example-namespace terms the visualization tests, bench and samples draw.

use purrdf_rdf::TermValue;

/// The test-fixture namespace every visualized term sits under.
pub(crate) const EX: &str = "https://example.org/";

/// The IRI `EX` + `local`.
pub(crate) fn iri(local: &str) -> TermValue {
    TermValue::Iri(format!("{EX}{local}"))
}
