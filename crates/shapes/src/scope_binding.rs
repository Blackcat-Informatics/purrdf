// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The shared CLI, C and WASM spelling of an explicit scope binding.

/// A scope binding whose original spelling contains no equals sign.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeBindingError {
    binding: String,
}

impl ScopeBindingError {
    /// The complete original supplied binding.
    #[must_use]
    pub fn binding(&self) -> &str {
        &self.binding
    }
}

impl core::fmt::Display for ScopeBindingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "scope binding `{}` is not NAME=TERM: it has no `=`",
            self.binding
        )
    }
}

impl core::error::Error for ScopeBindingError {}

impl purrdf_hash::error::StructuredError for ScopeBindingError {
    purrdf_hash::error_visit! {
        @record self { binding } => "scope-binding-missing-equals" { "binding" => Text(&self.binding) }
    }
}

/// Split a scope binding at its first equals sign, borrowing both original parts.
/// Empty names and values retain their existing downstream interpretation.
///
/// # Errors
/// Returns the complete owned spelling when no equals sign occurs.
pub fn parse_scope_binding(binding: &str) -> Result<(&str, &str), ScopeBindingError> {
    binding.split_once('=').ok_or_else(|| ScopeBindingError {
        binding: binding.to_owned(),
    })
}
