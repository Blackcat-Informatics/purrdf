// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native result invariants and their original operands.

/// A result cannot be serialized without losing or fabricating information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MalformedResult {
    /// A triple term contains a non-IRI predicate.
    Predicate {
        /// Original term at the refusing conversion boundary.
        term: purrdf_core::TermValue,
        /// Original kernel classification.
        cause: purrdf_core::NonIriPredicate,
    },
    /// The row has more cells than the projected variables can name.
    RowWidth {
        /// Original cell count.
        bindings: usize,
        /// Original projected variable count.
        variables: usize,
    },
    /// A bound cell has no corresponding variable header.
    MissingHeader {
        /// Original cell column.
        column: usize,
        /// Original header count.
        variables: usize,
    },
}

impl purrdf_hash::error::StructuredError for MalformedResult {
    purrdf_hash::error_visit! {
        Predicate { term, cause } => "results-predicate" { "term" => Term(term), "cause" => Cause(cause) },
        RowWidth { bindings, variables } => "results-row-width" { "bindings" => Unsigned(*bindings as u128), "variables" => Unsigned(*variables as u128) },
        MissingHeader { column, variables } => "results-missing-header" { "column" => Unsigned(*column as u128), "variables" => Unsigned(*variables as u128) },
    }
}

impl core::fmt::Display for MalformedResult {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Predicate { .. } => f.write_str("triple-term predicate is not an IRI"),
            Self::RowWidth {
                bindings,
                variables,
            } => write!(
                f,
                "solution row has {bindings} bindings but only {variables} variables are projected"
            ),
            Self::MissingHeader { column, variables } => write!(
                f,
                "binding column {column} has no variable header (row has {variables} vars)"
            ),
        }
    }
}

impl std::error::Error for MalformedResult {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::Predicate { cause, .. } = self {
            Some(cause)
        } else {
            None
        }
    }
}
