// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native variable refusals shared by RIF parsing and rule admission.

/// An original variable used where the RIF operation has no binding for it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RifVariableError {
    /// An asserted ground fact contains this variable.
    GroundFact(String),
    /// This rule-head variable has no occurrence in the body.
    UnboundHead(String),
}

impl purrdf_hash::error::StructuredError for RifVariableError {
    purrdf_hash::error_visit! {
        GroundFact(name) => "rif-ground-fact-variable" { "name" => Text(name) },
        UnboundHead(name) => "rif-unbound-head-variable" { "name" => Text(name) },
    }
}

impl core::fmt::Display for RifVariableError {
    fn fmt(&self, output: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::GroundFact(name) => write!(output, "variable ?{name} in a ground fact"),
            Self::UnboundHead(name) => write!(
                output,
                "RIF rule head variable ?{name} is not range-restricted \
                 (not bound by the rule body)",
            ),
        }
    }
}

impl std::error::Error for RifVariableError {}
