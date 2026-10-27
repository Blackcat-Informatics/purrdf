// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::fmt;

use purrdf_hash::error::{ErrorValue, NativeError};

/// Stable category for a projection failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ProjectionErrorKind {
    /// A logical capacity or physical storage admission refused reconstruction.
    Storage,
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
            Self::Storage => "storage",
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
    cause: Option<ProjectionCause>,
}

/// The original native refusal, kept independently of its human-readable context.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ProjectionCause {
    Storage(purrdf_lex::allocation::StorageError),
    Xsd(Box<purrdf_xsd::XsdError>),
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
            cause: None,
        }
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

    /// Preserve a metadata facet's native value-space failure and existing context.
    pub(crate) fn csvw_facet(error: purrdf_xsd::XsdError) -> Self {
        let mut result = Self::integrity(format!("invalid CSVW datatype facet: {error}"));
        result.cause = Some(ProjectionCause::Xsd(Box::new(error)));
        result
    }

    fn native_cause(&self) -> Option<&(dyn NativeError + 'static)> {
        match &self.cause {
            Some(ProjectionCause::Storage(error)) => Some(error),
            Some(ProjectionCause::Xsd(error)) => Some(error.as_ref()),
            None => None,
        }
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

    /// The original logical capacity or physical storage failure, if present.
    pub const fn storage_error(&self) -> Option<purrdf_lex::allocation::StorageError> {
        match &self.cause {
            Some(ProjectionCause::Storage(error)) => Some(*error),
            Some(ProjectionCause::Xsd(_)) | None => None,
        }
    }
}

impl purrdf_hash::error::StructuredError for ProjectionError {
    purrdf_hash::error_visit! {
        @record self { kind, message, path, cause } => "rdf-projection" {
            "kind" => Code(self.kind.as_str()),
            "message" => Text(&self.message),
            "path" => Value(self.path.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)),
            "cause" => Value(self.native_cause().map_or(ErrorValue::Absent, |cause| ErrorValue::Cause(cause))),
        }
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

impl From<purrdf_lex::allocation::StorageError> for ProjectionError {
    fn from(error: purrdf_lex::allocation::StorageError) -> Self {
        let mut result = Self::new(ProjectionErrorKind::Storage, error.to_string());
        result.cause = Some(ProjectionCause::Storage(error));
        result
    }
}

impl From<purrdf_core::RdfDiagnostic> for ProjectionError {
    fn from(error: purrdf_core::RdfDiagnostic) -> Self {
        error
            .storage_error()
            .map_or_else(|| Self::integrity(error.to_string()), Self::from)
    }
}

purrdf_hash::error_sources!(ProjectionError, native native_cause);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_causes_survive_projection_and_rdf_diagnostic_boundaries() {
        use purrdf_lex::allocation::{CapacityError, CapacitySpace, StorageError};
        for cause in [
            StorageError::Capacity(CapacityError::new(
                CapacitySpace::LocalTerms,
                u128::from(u32::MAX) + 1,
                u128::from(u32::MAX),
            )),
            StorageError::AllocationFailed,
        ] {
            let direct = ProjectionError::from(cause).at_path("rows.csv");
            let diagnostic = ProjectionError::from(purrdf_core::RdfDiagnostic::from(cause));
            assert_eq!(direct.kind(), ProjectionErrorKind::Storage);
            assert_eq!(direct.storage_error(), Some(cause));
            assert_eq!(diagnostic.storage_error(), Some(cause));
            assert_eq!(direct.path(), Some("rows.csv"));
            assert_eq!(
                std::error::Error::source(&direct)
                    .unwrap()
                    .downcast_ref::<StorageError>(),
                Some(&cause)
            );
            let inspected = purrdf_testkit::error_capture::Capture::read(&direct);
            assert!(matches!(
                inspected.field("kind"),
                ErrorValue::Code("storage")
            ));
            let expected = purrdf_testkit::error_capture::Capture::read(&cause);
            assert_eq!(inspected.child("cause").code, expected.code);
        }
    }

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
