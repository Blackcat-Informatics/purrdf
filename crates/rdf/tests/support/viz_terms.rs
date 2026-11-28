// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The example-namespace terms the visualization tests, bench and samples draw.

// The module is included into more than one target, and no single target uses every
// helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use purrdf_rdf::TermValue;

/// The test-fixture namespace every visualized term sits under.
pub const EX: &str = "https://example.org/";

/// The IRI `EX` + `local`.
pub fn iri(local: &str) -> TermValue {
    TermValue::Iri(format!("{EX}{local}"))
}
