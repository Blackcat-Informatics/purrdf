// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete native fold findings retained at the RDF boundary.

#[derive(Debug)]
pub(super) struct FoldFailure {
    pub(super) diagnostics: Vec<purrdf_gts::model::Diagnostic>,
}

impl core::fmt::Display for FoldFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "GTS fold reported {} diagnostic(s)",
            self.diagnostics.len()
        )
    }
}

impl std::error::Error for FoldFailure {}

impl purrdf_hash::error::StructuredError for FoldFailure {
    purrdf_hash::error_visit! {
        @record self { diagnostics } => "rdf-gts-fold" {
            "diagnostics" => List(self.diagnostics.iter().map(|diagnostic| purrdf_hash::error::ErrorValue::Cause(diagnostic)))
        }
    }
}
