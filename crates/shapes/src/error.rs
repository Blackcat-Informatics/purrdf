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
//! than on message text. A shapes graph that uses the SHACL JavaScript Extensions is the
//! other: SHACL-JS is a 2017 Working Group Note, not SHACL 1.2, and this engine has no
//! JavaScript engine, so the refusal is typed ([`ShapesError::ShaclJs`]) and names the
//! extension, the node and the term, rather than reading like a malformed graph.

use std::fmt;

use crate::imports::ShapesImportError;

/// Why a shapes-graph entry point refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShapesError {
    /// The shapes graph's `owl:imports` closure is not in hand, or the import table the
    /// caller supplied cannot be used. See [`crate::imports`].
    Imports(ShapesImportError),
    /// The shapes graph uses a term of the SHACL JavaScript Extensions (SHACL-JS) where
    /// the engine reads it. See [`ShaclJsRefusal`].
    ShaclJs(ShaclJsRefusal),
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
            Self::ShaclJs(_) | Self::Invalid(_) => None,
        }
    }

    /// The SHACL-JS refusal, when this is one.
    #[must_use]
    pub const fn as_shacl_js(&self) -> Option<&ShaclJsRefusal> {
        match self {
            Self::ShaclJs(refusal) => Some(refusal),
            Self::Imports(_) | Self::Invalid(_) => None,
        }
    }
}

impl fmt::Display for ShapesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Imports(error) => error.fmt(f),
            Self::ShaclJs(refusal) => refusal.fmt(f),
            Self::Invalid(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ShapesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Imports(error) => Some(error),
            Self::ShaclJs(refusal) => Some(refusal),
            Self::Invalid(_) => None,
        }
    }
}

/// A shapes graph refused because it uses the SHACL JavaScript Extensions.
///
/// SHACL-JS (`sh:JSConstraint`, `sh:js`, `sh:JSValidator`, `sh:JSRule`, …) is the 2017
/// Working Group Note "SHACL JavaScript Extensions", not part of SHACL 1.2, and this
/// engine has no JavaScript engine to evaluate it. A constraint it cannot evaluate is
/// refused rather than validated as if it were absent. The equivalent SHACL-SPARQL
/// constraint (`sh:sparql` with a `sh:select` query) loads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaclJsRefusal {
    node: String,
    term: String,
    message: String,
}

impl ShaclJsRefusal {
    pub(crate) const fn new(node: String, term: String, message: String) -> Self {
        Self {
            node,
            term,
            message,
        }
    }

    /// The node that uses the term, as the engine renders a term.
    #[must_use]
    pub fn node(&self) -> &str {
        &self.node
    }

    /// The SHACL-JS term's IRI.
    #[must_use]
    pub fn term(&self) -> &str {
        &self.term
    }

    /// The engine's full diagnostic, which names the SHACL JavaScript Extensions.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ShaclJsRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ShaclJsRefusal {}

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
