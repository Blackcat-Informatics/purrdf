// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use purrdf_lex::allocation::{StandardCause, retain_standard_cause};
use std::fmt;
use std::sync::OnceLock;

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
#[derive(Debug, Clone)]
pub struct ProjectionError {
    kind: ProjectionErrorKind,
    message: OnceLock<String>,
    path: Option<String>,
    cause: Option<ProjectionCause>,
}

impl PartialEq for ProjectionError {
    fn eq(&self, other: &Self) -> bool {
        if self.kind != other.kind || self.path != other.path {
            return false;
        }
        if self.kind == ProjectionErrorKind::SourceRead {
            if let (Some(a), Some(b)) = (self.standard_source(), other.standard_source())
                && a.shares_owner(b)
            {
                return true;
            }
            // Source errors historically compared their presentation, category
            // and path. Explicit comparison may request that lazy presentation;
            // capture, cloning and structured inspection never do.
            return self.message() == other.message();
        }
        self.cause == other.cause && self.message() == other.message()
    }
}
impl Eq for ProjectionError {}

/// The original native refusal, kept independently of its human-readable context.
#[derive(Debug, Clone)]
enum ProjectionCause {
    Storage(purrdf_lex::allocation::StorageError),
    Xsd(Box<purrdf_xsd::XsdError>),
    Ustar(crate::ustar::UstarError),
    Loss(Box<purrdf_core::LossSoundnessError>),
    Source(StandardCause),
}

impl PartialEq for ProjectionCause {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Storage(a), Self::Storage(b)) => a == b,
            (Self::Xsd(a), Self::Xsd(b)) => a == b,
            (Self::Ustar(a), Self::Ustar(b)) => a == b,
            (Self::Loss(a), Self::Loss(b)) => a == b,
            (Self::Source(a), Self::Source(b)) => a.shares_owner(b),
            _ => false,
        }
    }
}
impl Eq for ProjectionCause {}

impl ProjectionError {
    pub(crate) fn source_read(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        match retain_standard_cause(error) {
            Ok(source) => {
                let mut result = Self::new(ProjectionErrorKind::SourceRead, String::new());
                result.cause = Some(ProjectionCause::Source(source));
                result
            }
            Err(cause) => cause.into(),
        }
    }

    pub(crate) fn new(kind: ProjectionErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: OnceLock::from(message.into()),
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
            Some(ProjectionCause::Ustar(error)) => Some(error),
            Some(ProjectionCause::Loss(error)) => Some(error.as_ref()),
            Some(ProjectionCause::Source(_)) => None,
            None => None,
        }
    }

    fn standard_source(&self) -> Option<&StandardCause> {
        match &self.cause {
            Some(ProjectionCause::Source(source)) => Some(source),
            _ => None,
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
        if let Some(source) = self.standard_source() {
            return source.presentation();
        }
        if self.message.get().is_none()
            && let Some(ProjectionCause::Storage(cause)) = &self.cause
        {
            return self.message.get_or_init(|| cause.to_string());
        }
        // Ordinary constructors initialize text immediately; the only lazy
        // cases are the original standard and storage causes handled above.
        self.message
            .get()
            .expect("ordinary projection error text is initialized")
    }

    /// Inspection reports context without rendering an opaque standard source.
    fn inspection_message(&self) -> &str {
        if self.standard_source().is_some() {
            "dataset source read failed"
        } else if matches!(self.cause, Some(ProjectionCause::Storage(_))) {
            "native projection storage refused"
        } else {
            // Source and storage laziness were handled above; every ordinary
            // constructor publishes initialized text before this inspection.
            self.message
                .get()
                .expect("ordinary projection error text is initialized")
        }
    }

    /// Artifact or logical path associated with the failure, when known.
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// The original logical capacity or physical storage failure, if present.
    pub const fn storage_error(&self) -> Option<purrdf_lex::allocation::StorageError> {
        match &self.cause {
            Some(ProjectionCause::Storage(error)) => Some(*error),
            Some(
                ProjectionCause::Xsd(_)
                | ProjectionCause::Ustar(_)
                | ProjectionCause::Loss(_)
                | ProjectionCause::Source(_),
            )
            | None => None,
        }
    }
}

impl purrdf_hash::error::StructuredError for ProjectionError {
    purrdf_hash::error_visit! {
        @record self { kind, message, path, cause } => "rdf-projection" {
            "kind" => Code(self.kind.as_str()),
            "message" => Text(self.inspection_message()),
            "path" => Value(self.path.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)),
            "cause" => Value(self.native_cause().map_or(ErrorValue::Absent, |cause| ErrorValue::Cause(cause))),
            "source_type" => Value(self.standard_source().map_or(ErrorValue::Absent, |source| ErrorValue::Text(source.type_name()))),
        }
    }
}

impl fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "projection {} error", self.kind.as_str())?;
        if let Some(path) = &self.path {
            write!(f, " at `{path}`")?;
        }
        write!(f, ": {}", self.message())
    }
}

impl From<purrdf_lex::allocation::StorageError> for ProjectionError {
    fn from(error: purrdf_lex::allocation::StorageError) -> Self {
        // Capturing an original allocation refusal must not allocate a fallback
        // message. The existing infallible accessor formats it on first request.
        let mut result = Self::new(ProjectionErrorKind::Storage, String::new());
        result.message = OnceLock::new();
        result.cause = Some(ProjectionCause::Storage(error));
        result
    }
}

impl From<crate::ustar::UstarError> for ProjectionError {
    fn from(error: crate::ustar::UstarError) -> Self {
        let mut result = Self::package(error.to_string());
        result.cause = Some(ProjectionCause::Ustar(error));
        result
    }
}

impl From<purrdf_core::LossSoundnessError<&str>> for ProjectionError {
    fn from(error: purrdf_core::LossSoundnessError<&str>) -> Self {
        let mut result = Self::integrity(error.to_string());
        result.cause = Some(ProjectionCause::Loss(Box::new(error.into_owned())));
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

impl std::error::Error for ProjectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Some(source) = self.standard_source() {
            Some(source.error())
        } else {
            self.native_cause()
                .map(|cause| cause as &dyn std::error::Error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_causes_survive_without_capture_rendering_and_share_presentation() {
        use std::sync::Arc;
        use std::sync::Mutex;
        use std::sync::atomic::{AtomicUsize, Ordering};

        // A valid DatasetView read error need not implement StructuredError.
        #[derive(Debug, Clone)]
        struct BackendRefusal {
            marker: u64,
            renders: Arc<AtomicUsize>,
            text: Arc<Mutex<String>>,
        }
        impl fmt::Display for BackendRefusal {
            fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.renders.fetch_add(1, Ordering::SeqCst);
                out.write_str(&self.text.lock().expect("fixture text is not poisoned"))
            }
        }
        impl std::error::Error for BackendRefusal {}

        let renders = Arc::new(AtomicUsize::new(0));
        let text = Arc::new(Mutex::new("source refused".to_owned()));
        let refusal = |marker| BackendRefusal {
            marker,
            renders: Arc::clone(&renders),
            text: Arc::clone(&text),
        };
        let original = ProjectionError::source_read(refusal(7)).at_path("rows.csv");
        let copy = original.clone();
        assert_eq!(original, copy, "shared source equality needs no rendering");
        let fields = purrdf_testkit::error_capture::Capture::read(&copy);
        assert!(matches!(
            fields.field("kind"),
            ErrorValue::Code("source-read")
        ));
        assert!(
            matches!(fields.field("source_type"), ErrorValue::Text(name) if name == std::any::type_name::<BackendRefusal>())
        );
        let source = std::error::Error::source(&copy).expect("original standard error");
        assert_eq!(
            source
                .downcast_ref::<BackendRefusal>()
                .expect("concrete backend error")
                .marker,
            7
        );
        assert_eq!(
            renders.load(Ordering::SeqCst),
            0,
            "capture, clone, equality fast path and inspection do not render"
        );

        let independent = ProjectionError::source_read(refusal(99)).at_path("rows.csv");
        assert_eq!(
            original, independent,
            "legacy equality compares source text, category and path"
        );
        assert_eq!(renders.load(Ordering::SeqCst), 2);
        "changed later".clone_into(&mut text.lock().expect("fixture text is not poisoned"));
        assert_eq!(
            copy.message(),
            "source refused",
            "clones share the first presentation snapshot"
        );
        assert_eq!(original.message(), copy.message());
        assert_eq!(renders.load(Ordering::SeqCst), 2);
        let later = ProjectionError::source_read(refusal(101)).at_path("rows.csv");
        assert_ne!(copy, later);
        assert_eq!(renders.load(Ordering::SeqCst), 3);
        drop(original);
        assert_eq!(
            std::error::Error::source(&copy)
                .expect("retained source")
                .downcast_ref::<BackendRefusal>()
                .expect("concrete source survives producer")
                .marker,
            7
        );
        assert_eq!(
            copy.to_string(),
            "projection source-read error at `rows.csv`: source refused"
        );
        assert_eq!(renders.load(Ordering::SeqCst), 3);
    }

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
            assert!(matches!(
                inspected.field("message"),
                ErrorValue::Text("native projection storage refused")
            ));
            assert_eq!(direct.message(), cause.to_string());
            let presented = purrdf_testkit::error_capture::Capture::read(&direct);
            assert!(
                matches!(
                    presented.field("message"),
                    ErrorValue::Text("native projection storage refused")
                ),
                "inspection is independent of presentation cache state"
            );
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
