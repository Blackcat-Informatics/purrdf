// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The error every shapes-graph entry point returns.
//!
//! Most of what can go wrong while turning a shapes document into a verdict is a message for
//! a human — a syntax error, an unsupported construct, a SPARQL body that does not parse —
//! and is carried as one ([`ShapesError::Invalid`]). An incomplete `owl:imports` closure is
//! different: it is the one refusal a caller can ACT on programmatically, by supplying the
//! documents it names, and it must read the same on every host. So it is its own variant,
//! carrying the typed [`ShapesImportError`], and a caller branches on the variant rather
//! than on message text.

use std::fmt;

use crate::imports::ShapesImportError;

/// Why a shapes-graph entry point refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShapesError {
    /// The shapes graph's `owl:imports` closure is not in hand, or the import table the
    /// caller supplied cannot be used. See [`crate::imports`].
    Imports(ShapesImportError),
    /// Anything else: a document that does not parse, an unsupported or malformed SHACL
    /// construct, a failure during evaluation. The engine's own diagnostic.
    Invalid(String),
}

impl ShapesError {
    /// The import refusal, when this is one.
    #[must_use]
    pub const fn as_imports(&self) -> Option<&ShapesImportError> {
        match self {
            Self::Imports(error) => Some(error),
            Self::Invalid(_) => None,
        }
    }
}

impl fmt::Display for ShapesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Imports(error) => error.fmt(f),
            Self::Invalid(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ShapesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Imports(error) => Some(error),
            Self::Invalid(_) => None,
        }
    }
}

impl From<ShapesImportError> for ShapesError {
    fn from(error: ShapesImportError) -> Self {
        Self::Imports(error)
    }
}

impl From<String> for ShapesError {
    fn from(message: String) -> Self {
        Self::Invalid(message)
    }
}

impl From<&str> for ShapesError {
    fn from(message: &str) -> Self {
        Self::Invalid(message.to_owned())
    }
}

/// The rendered message, for a caller whose own error type is a string.
impl From<ShapesError> for String {
    fn from(error: ShapesError) -> Self {
        error.to_string()
    }
}
