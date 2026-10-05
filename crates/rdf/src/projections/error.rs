// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::fmt;

/// Stable category for a projection failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ProjectionErrorKind {
    /// The source dictionary or row stream failed operationally.
    SourceRead,
    /// Mandatory configuration is absent, malformed, or contradictory.
    Configuration,
    /// A caller-supplied resource bound was exceeded.
    ResourceLimit,
    /// An artifact package or deterministic archive is invalid.
    Package,
    /// An RDF term is malformed or invalid in its structural position.
    Term,
    /// A carrier document is syntactically invalid.
    Syntax,
    /// Cross-record or semantic integrity validation failed.
    Integrity,
}

impl ProjectionErrorKind {
    /// Stable lowercase machine label for this category.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SourceRead => "source-read",
            Self::Configuration => "configuration",
            Self::ResourceLimit => "resource-limit",
            Self::Package => "package",
            Self::Term => "term",
            Self::Syntax => "syntax",
            Self::Integrity => "integrity",
        }
    }
}

/// Typed hard failure from a graph or tabular projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionError {
    kind: ProjectionErrorKind,
    message: String,
    path: Option<String>,
    /// The typed condition behind the failure, when one is known: an XSD value
    /// failure's `xsd-*` presentation, with its XPath F&O `code`.
    presentation: Option<Box<purrdf_core::DiagnosticPresentation>>,
}

impl ProjectionError {
    pub(crate) fn source_read(error: impl fmt::Display) -> Self {
        Self::new(ProjectionErrorKind::SourceRead, error.to_string())
    }

    pub(crate) fn new(kind: ProjectionErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            path: None,
            presentation: None,
        }
    }

    /// Attach the typed condition behind this failure.
    #[must_use]
    pub(crate) fn with_presentation(
        mut self,
        presentation: purrdf_core::DiagnosticPresentation,
    ) -> Self {
        self.presentation = Some(Box::new(presentation));
        self
    }

    pub(crate) fn configuration(message: impl Into<String>) -> Self {
        Self::new(ProjectionErrorKind::Configuration, message)
    }

    pub(crate) fn limit(message: impl Into<String>) -> Self {
        Self::new(ProjectionErrorKind::ResourceLimit, message)
    }

    pub(crate) fn package(message: impl Into<String>) -> Self {
        Self::new(ProjectionErrorKind::Package, message)
    }

    pub(crate) fn term(message: impl Into<String>) -> Self {
        Self::new(ProjectionErrorKind::Term, message)
    }

    pub(crate) fn syntax(message: impl Into<String>) -> Self {
        Self::new(ProjectionErrorKind::Syntax, message)
    }

    /// Construct a semantic/cross-record integrity failure.
    pub fn integrity(message: impl Into<String>) -> Self {
        Self::new(ProjectionErrorKind::Integrity, message)
    }

    #[must_use]
    pub(crate) fn at_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Failure category.
    pub const fn kind(&self) -> ProjectionErrorKind {
        self.kind
    }

    /// Human-readable detail without the category/path decoration.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Artifact or logical path associated with the failure, when known.
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// The typed condition behind the failure, when one is known: for a value that
    /// `purrdf_xsd` refuses, its `xsd-*` presentation, whose `code` argument is the
    /// XPath F&O error code (`err:FORG0001`, `err:FOCA0003`, …).
    pub fn presentation(&self) -> Option<&purrdf_core::DiagnosticPresentation> {
        self.presentation.as_deref()
    }
}

impl fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "projection {} error", self.kind.as_str())?;
        if let Some(path) = &self.path {
            write!(f, " at `{path}`")?;
        }
        write!(f, ": {}", self.message)
    }
}

impl std::error::Error for ProjectionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_includes_stable_category_and_path() {
        let error = ProjectionError::syntax("bad row").at_path("nodes.csv:2");
        assert_eq!(error.kind(), ProjectionErrorKind::Syntax);
        assert_eq!(error.message(), "bad row");
        assert_eq!(error.path(), Some("nodes.csv:2"));
        assert_eq!(
            error.to_string(),
            "projection syntax error at `nodes.csv:2`: bad row"
        );
    }

    #[test]
    fn integrity_constructor_is_typed() {
        assert_eq!(
            ProjectionError::integrity("dangling edge").kind(),
            ProjectionErrorKind::Integrity
        );
    }
}
