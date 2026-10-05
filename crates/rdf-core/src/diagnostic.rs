// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::fmt;
use std::fmt::Write as _;

pub use purrdf_lex::diagnostic::{
    DiagnosticParameter, DiagnosticPresentation, DiagnosticPresentationError, DiagnosticValue,
};

/// Severity for RDF ingestion, conversion, and adapter diagnostics.
///
/// Deliberately exhaustive (NOT `#[non_exhaustive]`): these are the four standard
/// diagnostic levels (mirroring LSP `DiagnosticSeverity`), a closed set — so
/// consumers SHOULD match them exhaustively rather than fall back on a lossy `_`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RdfSeverity {
    /// A hard failure: the operation's result is incorrect or unusable.
    Error,
    /// A likely problem that did not prevent the operation from completing.
    Warning,
    /// A secondary remark attached to another diagnostic or finding.
    Note,
    /// Purely informational output.
    Info,
}

impl RdfSeverity {
    /// The lowercase severity label (`"error"`, `"warning"`, `"note"`, `"info"`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
            Self::Info => "info",
        }
    }
}

impl fmt::Display for RdfSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Concrete or logical location attached to an RDF diagnostic.
///
/// Every field is `Ord` (plain `Option<String>`/`Option<u32>`/`Option<u64>`),
/// so the struct derives a total order directly — the loss ledger relies on
/// this to sort runtime entries deterministically without a separate
/// `display()`-string comparison.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash, PartialOrd, Ord)]
pub struct RdfLocation {
    /// The source file path (physical location anchor).
    pub path: Option<String>,
    /// The 1-based source line within [`path`](Self::path).
    pub line: Option<u64>,
    /// The 1-based source column within [`line`](Self::line).
    pub column: Option<u32>,
    /// A logical (non-file) location label, e.g. an adapter or stage name.
    pub logical: Option<String>,
    /// The subject the diagnostic concerns: a shape/term IRI, a blank-node id,
    /// or a JSON pointer into a compiled artifact. Distinct from
    /// [`logical`](Self::logical) (an adapter/stage label): this identifies
    /// *what* was affected, not *where in the pipeline* it happened.
    pub subject: Option<String>,
    /// The GTS term id the diagnostic refers to.
    pub gts_term_id: Option<u64>,
    /// The GTS quad index the diagnostic refers to.
    pub gts_quad_index: Option<u64>,
    /// The GTS reifier id the diagnostic refers to.
    pub gts_reifier_id: Option<u64>,
    /// The GTS frame index the diagnostic refers to.
    pub gts_frame_index: Option<u64>,
    /// The GTS segment index the diagnostic refers to.
    pub gts_segment_index: Option<u64>,
}

impl RdfLocation {
    /// A purely logical location (no file path), from its label.
    pub fn logical(logical: impl Into<String>) -> Self {
        Self {
            logical: Some(logical.into()),
            ..Self::default()
        }
    }

    /// A source-file (physical) location, by repo-relative path. Pair with
    /// [`with_line`](Self::with_line)/[`with_column`](Self::with_column) for a
    /// sub-file position. This is the file-level anchor that threads into a SARIF
    /// `physicalLocation`; [`logical`](Self::logical) sets the label field instead.
    pub fn file(path: impl Into<String>) -> Self {
        Self {
            path: Some(path.into()),
            ..Self::default()
        }
    }

    /// Attaches a 1-based source line.
    #[must_use]
    pub fn with_line(mut self, line: u64) -> Self {
        self.line = Some(line);
        self
    }

    /// Attaches a 1-based source column.
    #[must_use]
    pub fn with_column(mut self, column: u32) -> Self {
        self.column = Some(column);
        self
    }

    /// Attaches a subject identifier: a shape/term IRI, a blank-node id, or a
    /// JSON pointer into a compiled artifact.
    #[must_use]
    pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Attaches the GTS term id the diagnostic refers to.
    #[must_use]
    pub fn with_gts_term(mut self, term_id: u64) -> Self {
        self.gts_term_id = Some(term_id);
        self
    }

    /// Attaches the GTS quad index the diagnostic refers to.
    #[must_use]
    pub fn with_gts_quad(mut self, quad_index: u64) -> Self {
        self.gts_quad_index = Some(quad_index);
        self
    }

    /// Attaches the GTS reifier id the diagnostic refers to.
    #[must_use]
    pub fn with_gts_reifier(mut self, reifier_id: u64) -> Self {
        self.gts_reifier_id = Some(reifier_id);
        self
    }

    /// Attaches the GTS frame index the diagnostic refers to.
    #[must_use]
    pub fn with_gts_frame(mut self, frame_index: u64) -> Self {
        self.gts_frame_index = Some(frame_index);
        self
    }

    /// Attaches the GTS segment index the diagnostic refers to.
    #[must_use]
    pub fn with_gts_segment(mut self, segment_index: u64) -> Self {
        self.gts_segment_index = Some(segment_index);
        self
    }

    /// Whether every slot is unset (an empty location carries no information).
    pub fn is_empty(&self) -> bool {
        self.path.is_none()
            && self.line.is_none()
            && self.column.is_none()
            && self.logical.is_none()
            && self.subject.is_none()
            && self.gts_term_id.is_none()
            && self.gts_quad_index.is_none()
            && self.gts_reifier_id.is_none()
            && self.gts_frame_index.is_none()
            && self.gts_segment_index.is_none()
    }

    /// Renders a human-readable one-line form: `path:line:column` (falling back
    /// to the logical label or `<unknown>`) followed by any GTS anchors, e.g.
    /// `term#3 quad#7`.
    pub fn display(&self) -> String {
        let mut out = self
            .path
            .as_deref()
            .or(self.logical.as_deref())
            .unwrap_or("<unknown>")
            .to_owned();
        if let Some(line) = self.line {
            out.push(':');
            out.push_str(&line.to_string());
            if let Some(column) = self.column {
                out.push(':');
                out.push_str(&column.to_string());
            }
        }
        if let Some(term_id) = self.gts_term_id {
            let _ = write!(out, " term#{term_id}");
        }
        if let Some(quad_index) = self.gts_quad_index {
            let _ = write!(out, " quad#{quad_index}");
        }
        if let Some(reifier_id) = self.gts_reifier_id {
            let _ = write!(out, " reifier#{reifier_id}");
        }
        if let Some(frame_index) = self.gts_frame_index {
            let _ = write!(out, " frame#{frame_index}");
        }
        if let Some(segment_index) = self.gts_segment_index {
            let _ = write!(out, " segment#{segment_index}");
        }
        if let Some(subject) = &self.subject {
            let _ = write!(out, " subject={subject}");
        }
        out
    }
}

/// Structured RDF diagnostic. Callers translate this to their reporting layer.
/// Construct through [`Self::new`] or [`Self::error`], retaining the readable
/// compatibility fields while optional presentation data remains extensible.
///
/// ```compile_fail
/// use purrdf_core::{RdfDiagnostic, RdfSeverity};
/// let diagnostic = RdfDiagnostic {
///     severity: RdfSeverity::Error, code: "example".to_owned(),
///     message: "failure".to_owned(), detail: None, location: None,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct RdfDiagnostic {
    /// The diagnostic severity level.
    pub severity: RdfSeverity,
    /// The stable machine-readable diagnostic code.
    pub code: String,
    /// The primary human-readable message.
    pub message: String,
    /// Optional secondary detail elaborating on the message.
    pub detail: Option<String>,
    /// Where the diagnostic applies, when known.
    pub location: Option<Box<RdfLocation>>,
    presentation: Option<Box<DiagnosticPresentation>>,
}

impl RdfDiagnostic {
    /// A diagnostic from its severity, code, and message, with no detail or
    /// location.
    pub fn new(severity: RdfSeverity, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
            detail: None,
            location: None,
            presentation: None,
        }
    }

    /// An [`RdfSeverity::Error`]-level diagnostic from its code and message.
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(RdfSeverity::Error, code, message)
    }

    /// Preserve an IRI failure's typed fields and exact existing English wording.
    /// Machine consumers never need to recover arguments from that wording.
    pub fn from_iri(error: &purrdf_iri::IriError) -> Self {
        Self::error(error.diagnostic_code(), "").with_presentation(error.presentation())
    }

    /// Attach validated presentation data and render its compatibility English.
    #[must_use]
    pub fn with_presentation(mut self, presentation: DiagnosticPresentation) -> Self {
        presentation.english().clone_into(&mut self.message);
        self.detail = presentation
            .detail()
            .map(|detail| detail.english().to_owned());
        self.presentation = Some(Box::new(presentation));
        self
    }

    /// The stable presentation identity and typed arguments, when supplied.
    pub fn presentation(&self) -> Option<&DiagnosticPresentation> {
        self.presentation.as_deref()
    }

    /// A machine record preserving codes, presentation and exact logical anchors.
    /// Logical 64-bit positions are decimal strings; local columns remain numbers.
    pub fn to_json(&self) -> purrdf_lex::json::Value {
        use purrdf_lex::json::{Object, Value};
        let mut object = Object::new();
        object.insert("schema".to_owned(), Value::from("purrdf-diagnostic-v1"));
        object.insert("severity".to_owned(), Value::from(self.severity.as_str()));
        object.insert("code".to_owned(), Value::from(self.code.clone()));
        object.insert("message".to_owned(), Value::from(self.message.clone()));
        if let Some(detail) = &self.detail {
            object.insert("detail".to_owned(), Value::from(detail.clone()));
        }
        if let Some(presentation) = self.presentation() {
            object.insert("presentation".to_owned(), presentation.to_json());
        }
        if let Some(location) = &self.location {
            let mut anchor = Object::new();
            for (name, value) in [
                ("path", &location.path),
                ("logical", &location.logical),
                ("subject", &location.subject),
            ] {
                if let Some(value) = value {
                    anchor.insert(name.to_owned(), Value::from(value.clone()));
                }
            }
            for (name, value) in [
                ("line", location.line),
                ("gtsTermId", location.gts_term_id),
                ("gtsQuadIndex", location.gts_quad_index),
                ("gtsReifierId", location.gts_reifier_id),
                ("gtsFrameIndex", location.gts_frame_index),
                ("gtsSegmentIndex", location.gts_segment_index),
            ] {
                if let Some(value) = value {
                    anchor.insert(name.to_owned(), Value::from(value.to_string()));
                }
            }
            if let Some(column) = location.column {
                anchor.insert("column".to_owned(), Value::from(column));
            }
            object.insert("location".to_owned(), Value::Object(anchor));
        }
        Value::Object(object)
    }

    /// Attaches secondary detail text.
    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Attaches a location; an empty location is dropped rather than stored.
    #[must_use]
    pub fn with_location(mut self, location: RdfLocation) -> Self {
        if !location.is_empty() {
            self.location = Some(Box::new(location));
        }
        self
    }
}

impl fmt::Display for RdfDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}: {}", self.severity, self.code, self.message)?;
        if let Some(location) = &self.location {
            write!(f, " at {}", location.display())?;
        }
        if let Some(detail) = &self.detail {
            write!(f, " ({detail})")?;
        }
        Ok(())
    }
}

impl std::error::Error for RdfDiagnostic {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_drift_is_refused_and_large_arguments_remain_exact() {
        let parameter =
            || DiagnosticParameter::new("offset", DiagnosticValue::Unsigned((1 << 53) + 1));
        assert!(matches!(
            DiagnosticPresentation::new("test", "{missing}", vec![parameter()]),
            Err(DiagnosticPresentationError::MissingParameter(_))
        ));
        assert!(matches!(
            DiagnosticPresentation::new("test", "constant", vec![parameter()]),
            Err(DiagnosticPresentationError::UnusedParameter(_))
        ));
        assert!(matches!(
            DiagnosticPresentation::new("test", "{offset}", vec![parameter(), parameter()]),
            Err(DiagnosticPresentationError::DuplicateParameter(_))
        ));
        assert!(matches!(
            DiagnosticPresentation::new("test", "{offset:x}", vec![parameter()]),
            Err(DiagnosticPresentationError::InvalidTemplate)
        ));
        assert!(matches!(
            DiagnosticPresentation::new("test", "{offset", vec![parameter()]),
            Err(DiagnosticPresentationError::InvalidTemplate)
        ));
        assert!(matches!(
            DiagnosticPresentation::new(
                "test",
                "{1offset}",
                vec![DiagnosticParameter::new(
                    "1offset",
                    DiagnosticValue::Unsigned(1)
                )],
            ),
            Err(DiagnosticPresentationError::InvalidParameter(_))
        ));
        let detail =
            DiagnosticPresentation::new("test.detail", "{{byte}} {offset}", vec![parameter()])
                .unwrap();
        let main = DiagnosticPresentation::new("test.main", "failure", vec![])
            .unwrap()
            .with_detail(detail)
            .unwrap();
        assert!(matches!(
            DiagnosticPresentation::new("test.outer", "outer", vec![])
                .unwrap()
                .with_detail(main.clone()),
            Err(DiagnosticPresentationError::NestedDetail)
        ));
        let diagnostic = RdfDiagnostic::error("stable-code", "old").with_presentation(main);
        assert_eq!(diagnostic.message, "failure");
        assert_eq!(
            diagnostic.detail.as_deref(),
            Some("{byte} 9007199254740993")
        );
        let record = diagnostic.presentation().unwrap().to_json().to_string();
        assert!(record.contains("9007199254740993"));
        assert!(record.contains("\"kind\":\"unsigned\""));
        assert_eq!(diagnostic.code, "stable-code");
    }

    #[test]
    fn iri_fields_render_the_existing_english() {
        use purrdf_iri::{BaseInScope, BaseOrigin, IriError};
        let cases = [
            IriError::Empty,
            IriError::MissingScheme,
            IriError::BadScheme("1_bad".to_owned()),
            IriError::BadPercentEncoding(7),
            IriError::DisallowedChar('漢', 12),
            IriError::BadAuthority("unclosed host".to_owned()),
            IriError::NonAbsoluteBase("relative".to_owned()),
            IriError::NoBase {
                reference: "relative\"\\漢".to_owned(),
            },
            IriError::NotAbsoluteByGrammar {
                reference: "relative".to_owned(),
                base: BaseInScope::Absent,
            },
            IriError::NotAbsoluteByGrammar {
                reference: "relative".to_owned(),
                base: BaseInScope::InForce {
                    iri: "https://example.org/".to_owned(),
                    origin: BaseOrigin::Caller,
                },
            },
            IriError::NotAbsoluteByGrammar {
                reference: "relative".to_owned(),
                base: BaseInScope::InForce {
                    iri: "https://example.org/漢".to_owned(),
                    origin: BaseOrigin::Directive {
                        line: 9_007_199_254_740_993,
                        column: 3,
                    },
                },
            },
            IriError::NotAbsoluteByGrammar {
                reference: "relative".to_owned(),
                base: BaseInScope::InForce {
                    iri: "https://example.org/".to_owned(),
                    origin: BaseOrigin::Enclosing,
                },
            },
        ];
        for error in cases {
            let diagnostic = RdfDiagnostic::from_iri(&error);
            assert_eq!(diagnostic.code, error.diagnostic_code());
            assert_eq!(diagnostic.message, error.to_string());
            assert!(diagnostic.presentation().is_some());
        }
        let directive = RdfDiagnostic::from_iri(&IriError::NotAbsoluteByGrammar {
            reference: "relative".to_owned(),
            base: BaseInScope::InForce {
                iri: "https://example.org/漢".to_owned(),
                origin: BaseOrigin::Directive {
                    line: 9_007_199_254_740_993,
                    column: 3,
                },
            },
        });
        let presentation = directive.presentation().unwrap();
        assert_eq!(
            presentation.message_id(),
            "iri-not-absolute-by-grammar.directive"
        );
        assert!(presentation.parameters().iter().any(|parameter| {
            parameter.name() == "base"
                && parameter.value() == &DiagnosticValue::Text("https://example.org/漢".to_owned())
        }));
        assert!(presentation.parameters().iter().any(|parameter| {
            parameter.name() == "line"
                && parameter.value() == &DiagnosticValue::Unsigned(9_007_199_254_740_993)
        }));
    }
}
