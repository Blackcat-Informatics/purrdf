// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::fmt;
use std::hash::Hash;

use purrdf_hash::error::{ErrorValue, StructuredError};

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

impl StructuredError for RdfSeverity {
    fn visit<'error>(&'error self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'error>) {
        visitor.variant(self.as_str());
    }
}

/// Concrete or logical location attached to an RDF diagnostic.
///
/// Every field is `Ord` (plain `Option<String>`/`Option<u32>`/`Option<u64>`),
/// so the struct derives a total order directly — the loss ledger relies on
/// this to sort runtime entries deterministically without a separate
/// `display()`-string comparison.
#[derive(Debug, PartialEq, Eq, Default, Hash, PartialOrd, Ord)]
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

purrdf_lex::resident_clone!(RdfLocation);

impl StructuredError for RdfLocation {
    purrdf_hash::error_visit! {
        @record self { path, line, column, logical, subject, gts_term_id,
            gts_quad_index, gts_reifier_id, gts_frame_index, gts_segment_index }
        => "rdf-location" {
            "path" => Value(self.path.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)),
            "line" => Value(self.line.map_or(ErrorValue::Absent, |value| ErrorValue::Unsigned(u128::from(value)))),
            "column" => Value(self.column.map_or(ErrorValue::Absent, |value| ErrorValue::Unsigned(u128::from(value)))),
            "logical" => Value(self.logical.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)),
            "subject" => Value(self.subject.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)),
            "gts_term_id" => Value(self.gts_term_id.map_or(ErrorValue::Absent, |value| ErrorValue::Unsigned(u128::from(value)))),
            "gts_quad_index" => Value(self.gts_quad_index.map_or(ErrorValue::Absent, |value| ErrorValue::Unsigned(u128::from(value)))),
            "gts_reifier_id" => Value(self.gts_reifier_id.map_or(ErrorValue::Absent, |value| ErrorValue::Unsigned(u128::from(value)))),
            "gts_frame_index" => Value(self.gts_frame_index.map_or(ErrorValue::Absent, |value| ErrorValue::Unsigned(u128::from(value)))),
            "gts_segment_index" => Value(self.gts_segment_index.map_or(ErrorValue::Absent, |value| ErrorValue::Unsigned(u128::from(value)))),
        }
    }
}

impl RdfLocation {
    /// Copy physical and logical locations without allocating ahead of admission.
    ///
    /// # Errors
    /// Returns the original physical refusal, preserving the source location.
    pub fn clone_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
        &self,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> Result<Self, purrdf_lex::allocation::StorageError> {
        Ok(Self {
            path: self
                .path
                .as_deref()
                .map(|text| memory.string(text))
                .transpose()?,
            line: self.line,
            column: self.column,
            logical: self
                .logical
                .as_deref()
                .map(|text| memory.string(text))
                .transpose()?,
            subject: self
                .subject
                .as_deref()
                .map(|text| memory.string(text))
                .transpose()?,
            gts_term_id: self.gts_term_id,
            gts_quad_index: self.gts_quad_index,
            gts_reifier_id: self.gts_reifier_id,
            gts_frame_index: self.gts_frame_index,
            gts_segment_index: self.gts_segment_index,
        })
    }

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
        self.to_string()
    }
}

impl fmt::Display for RdfLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            self.path
                .as_deref()
                .or(self.logical.as_deref())
                .unwrap_or("<unknown>"),
        )?;
        if let Some(line) = self.line {
            write!(f, ":{line}")?;
            if let Some(column) = self.column {
                write!(f, ":{column}")?;
            }
        }
        for (label, value) in [
            ("term", self.gts_term_id),
            ("quad", self.gts_quad_index),
            ("reifier", self.gts_reifier_id),
            ("frame", self.gts_frame_index),
            ("segment", self.gts_segment_index),
        ] {
            if let Some(value) = value {
                write!(f, " {label}#{value}")?;
            }
        }
        if let Some(subject) = &self.subject {
            write!(f, " subject={subject}")?;
        }
        Ok(())
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
    attachments: Option<Box<DiagnosticAttachments>>,
}

/// Cold reporting payloads share one owner so ordinary diagnostics retain the
/// original single optional pointer. Native ingress carries StorageError inline
/// until this reporting boundary; no attachment is allocated on successful work.
struct DiagnosticAttachments {
    presentation: Option<DiagnosticPresentation>,
    storage: Option<purrdf_lex::allocation::StorageError>,
    cause: Option<DiagnosticCause>,
}

/// An immutable native cause retaining its independently admitted original owner.
/// Its grant covers both the erased error box and this shared control; clones
/// retain that same grant rather than charging or copying the original payload.
pub type DiagnosticCause =
    purrdf_lex::allocation::SharedOwned<Box<dyn purrdf_hash::error::NativeError>>;

impl StructuredError for DiagnosticAttachments {
    purrdf_hash::error_visit! {
        @record self { presentation, storage, cause } => "rdf-diagnostic-attachments" {
            "presentation" => Value(self.presentation.as_ref().map_or(ErrorValue::Absent, |value| ErrorValue::Record(value))),
            "storage" => Value(self.storage.as_ref().map_or(ErrorValue::Absent, |value| ErrorValue::Cause(value))),
            "native_cause" => Value(self.cause.as_ref().map_or(ErrorValue::Absent, |value| ErrorValue::Cause(&***value))),
        }
    }
}

impl StructuredError for RdfDiagnostic {
    purrdf_hash::error_visit! {
        @record self { severity, code, message, detail, location, attachments }
        => "rdf-diagnostic" {
            "severity" => Record(&self.severity),
            "code" => Code(&self.code),
            "message" => Text(&self.message),
            "detail" => Value(self.detail.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)),
            "location" => Value(self.location.as_deref().map_or(ErrorValue::Absent, |value| ErrorValue::Record(value))),
            "attachments" => Value(self.attachments.as_deref().map_or(ErrorValue::Absent, |value| ErrorValue::Record(value))),
        }
    }
}

// These fields were exposed by the original derives. Keep their names, order
// and value semantics independent of the private attachment allocation layout.
impl fmt::Debug for RdfDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let attachments = self.attachments.as_deref();
        f.debug_struct("RdfDiagnostic")
            .field("severity", &self.severity)
            .field("code", &self.code)
            .field("message", &self.message)
            .field("detail", &self.detail)
            .field("location", &self.location)
            .field(
                "presentation",
                &attachments.and_then(|payload| payload.presentation.as_ref()),
            )
            .field("storage", &attachments.and_then(|payload| payload.storage))
            .finish()
    }
}

impl Hash for RdfDiagnostic {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.severity.hash(state);
        self.code.hash(state);
        self.message.hash(state);
        self.detail.hash(state);
        self.location.hash(state);
        self.presentation().hash(state);
        self.storage_error().hash(state);
    }
}

// Native causes have no generic value equality. Keep the established diagnostic
// equality/hash over its compatibility fields and typed presentation/storage,
// independently of supplemental provenance and private attachment allocation.
impl PartialEq for RdfDiagnostic {
    fn eq(&self, other: &Self) -> bool {
        self.severity == other.severity
            && self.code == other.code
            && self.message == other.message
            && self.detail == other.detail
            && self.location == other.location
            && self.presentation() == other.presentation()
            && self.storage_error() == other.storage_error()
    }
}
impl Eq for RdfDiagnostic {}

purrdf_lex::resident_clone!(RdfDiagnostic);

impl RdfDiagnostic {
    /// Copy original typed diagnostic data through its actual allocation owner.
    /// All boxes and text are admitted before physical allocation or copying.
    ///
    /// # Errors
    /// Returns the first physical refusal before the replacement is published.
    pub fn clone_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
        &self,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> Result<Self, purrdf_lex::allocation::StorageError> {
        Ok(Self {
            severity: self.severity,
            code: memory.string(&self.code)?,
            message: memory.string(&self.message)?,
            detail: self
                .detail
                .as_deref()
                .map(|text| memory.string(text))
                .transpose()?,
            location: self
                .location
                .as_deref()
                .map(|location| {
                    let location = location.clone_with_memory(memory)?;
                    memory.boxed(location)
                })
                .transpose()?,
            attachments: self
                .attachments
                .as_deref()
                .map(|attachments| {
                    let presentation = attachments
                        .presentation
                        .as_ref()
                        .map(|presentation| presentation.clone_with_memory(memory))
                        .transpose()?;
                    memory.boxed(DiagnosticAttachments {
                        presentation,
                        storage: attachments.storage,
                        cause: attachments.cause.clone(),
                    })
                })
                .transpose()?,
        })
    }

    /// Exact concrete heap capacity, excluding the outer diagnostic value.
    /// Location and attachment boxes, spare arrays and all owned text count.
    /// Shared native causes retain independent grants and are not charged again.
    #[must_use]
    pub fn retained_bytes(&self) -> Option<usize> {
        let mut bytes = self
            .code
            .capacity()
            .checked_add(self.message.capacity())?
            .checked_add(self.detail.as_ref().map_or(0, String::capacity))?;
        if let Some(location) = &self.location {
            bytes = bytes.checked_add(size_of::<RdfLocation>())?;
            for text in [&location.path, &location.logical, &location.subject]
                .into_iter()
                .flatten()
            {
                bytes = bytes.checked_add(text.capacity())?;
            }
        }
        if let Some(attachments) = &self.attachments {
            bytes = bytes.checked_add(size_of::<DiagnosticAttachments>())?;
            if let Some(presentation) = &attachments.presentation {
                bytes = bytes.checked_add(presentation.retained_bytes()?)?;
            }
        }
        Some(bytes)
    }

    /// Construct a native error through its original physical admission before
    /// copying the code or rendering the borrowed message once. The formatter
    /// must allocate no private payload; all destination growth uses this memory.
    ///
    /// # Errors
    /// Returns the physical refusal, allocator failure or formatter failure.
    pub fn try_error_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
        code: &str,
        message: &(impl fmt::Display + ?Sized),
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> Result<Self, purrdf_lex::allocation::StorageError> {
        let owned_code = memory.string_with_progress(code)?;
        let message = memory.format_with_progress(message)?;
        Ok(Self::error(owned_code, message))
    }

    /// A diagnostic from its severity, code, and message, with no detail or
    /// location.
    pub fn new(severity: RdfSeverity, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
            detail: None,
            location: None,
            attachments: None,
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

    /// The exact original capacity or physical refusal, when this diagnostic
    /// transports a native storage failure through a codec's reporting API.
    #[must_use]
    pub const fn storage_error(&self) -> Option<purrdf_lex::allocation::StorageError> {
        match &self.attachments {
            Some(attachments) => attachments.storage,
            None => None,
        }
    }

    /// Borrow the original native error, without formatting or copying it.
    #[must_use]
    pub fn native_cause(&self) -> Option<&(dyn purrdf_hash::error::NativeError + 'static)> {
        self.attachments
            .as_ref()?
            .cause
            .as_ref()
            .map(|cause| &***cause)
    }

    /// Attach an already admitted native cause through this diagnostic's owner.
    /// The cause's independent grant survives copies of the diagnostic.
    /// Existing diagnostic buffers must already belong to this memory account;
    /// this operation admits only a new attachment box when one is needed.
    ///
    /// # Errors
    /// Returns admission or allocation refusal before allocating an attachment.
    pub fn with_native_cause<S: purrdf_lex::allocation::Admission + ?Sized>(
        mut self,
        cause: DiagnosticCause,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> Result<Self, purrdf_lex::allocation::StorageError> {
        match &mut self.attachments {
            Some(attachments) => attachments.cause = Some(cause),
            None => {
                self.attachments = Some(memory.boxed(DiagnosticAttachments {
                    presentation: None,
                    storage: None,
                    cause: Some(cause),
                })?);
            }
        }
        Ok(self)
    }

    /// Attach validated presentation data and render its compatibility English.
    #[must_use]
    pub fn with_presentation(mut self, presentation: DiagnosticPresentation) -> Self {
        presentation.english().clone_into(&mut self.message);
        self.detail = presentation
            .detail()
            .map(|detail| detail.english().to_owned());
        match &mut self.attachments {
            Some(attachments) => attachments.presentation = Some(presentation),
            None => {
                self.attachments = Some(Box::new(DiagnosticAttachments {
                    presentation: Some(presentation),
                    storage: None,
                    cause: None,
                }));
            }
        }
        self
    }

    /// The stable presentation identity and typed arguments, when supplied.
    pub fn presentation(&self) -> Option<&DiagnosticPresentation> {
        self.attachments.as_ref()?.presentation.as_ref()
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
        if let Some(purrdf_lex::allocation::StorageError::Capacity(error)) = self.storage_error() {
            let mut capacity = Object::new();
            capacity.insert("space".to_owned(), Value::from(error.space.as_str()));
            capacity.insert(
                "attempted".to_owned(),
                Value::from(error.attempted().to_string()),
            );
            capacity.insert("limit".to_owned(), Value::from(error.limit().to_string()));
            object.insert("capacity".to_owned(), Value::Object(capacity));
        }
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
            write!(f, " at {location}")?;
        }
        if let Some(detail) = &self.detail {
            write!(f, " ({detail})")?;
        }
        Ok(())
    }
}

impl From<purrdf_lex::allocation::StorageError> for RdfDiagnostic {
    fn from(error: purrdf_lex::allocation::StorageError) -> Self {
        let code = match error {
            purrdf_lex::allocation::StorageError::Capacity(_) => "rdf-capacity-exhausted",
            _ => "rdf-storage-refused",
        };
        let mut diagnostic = Self::error(code, error.to_string());
        diagnostic.attachments = Some(Box::new(DiagnosticAttachments {
            presentation: None,
            storage: Some(error),
            cause: None,
        }));
        diagnostic
    }
}
impl From<purrdf_iri::IriReadError> for RdfDiagnostic {
    fn from(error: purrdf_iri::IriReadError) -> Self {
        match error {
            purrdf_iri::IriReadError::Lexical(error) => Self::from_iri(&error),
            purrdf_iri::IriReadError::Storage(error) => error.into(),
        }
    }
}
impl std::error::Error for RdfDiagnostic {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Some(cause) = self.native_cause() {
            return Some(cause);
        }
        self.attachments
            .as_ref()
            .and_then(|attachments| attachments.storage.as_ref())
            .map(|error| error as &dyn std::error::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use purrdf_lex::allocation::{Admission, Memory, StorageError};

    struct Grant {
        live: usize,
        limit: usize,
    }
    impl Admission for Grant {
        fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
            if bytes > self.limit {
                return Err(StorageError::AdmissionFailed);
            }
            self.live = bytes;
            Ok(())
        }
    }

    #[test]
    fn native_diagnostic_copy_preserves_typed_fields_and_exact_original_layouts() {
        let primary = DiagnosticPresentation::new(
            "primary",
            "bad {value}",
            vec![DiagnosticParameter::new(
                "value",
                DiagnosticValue::Text("verbatim λ".into()),
            )],
        )
        .unwrap();
        let detail = DiagnosticPresentation::new(
            "detail",
            "at {offset}",
            vec![DiagnosticParameter::new(
                "offset",
                DiagnosticValue::Unsigned((1 << 53) + 1),
            )],
        )
        .unwrap();
        let mut source = RdfDiagnostic::error("typed-copy", "")
            .with_presentation(primary.with_detail(detail).unwrap());
        source.location = Some(Box::new(RdfLocation {
            path: Some("document.ttl".into()),
            logical: Some("source stage".into()),
            subject: Some("example subject".into()),
            line: Some(17),
            column: Some(9),
            gts_term_id: Some(41),
            gts_quad_index: Some(42),
            gts_reifier_id: Some(43),
            gts_frame_index: Some(44),
            gts_segment_index: Some(45),
        }));
        let mut refused = Grant { live: 0, limit: 0 };
        {
            let mut memory = Memory::new(&mut refused);
            assert!(matches!(
                source.clone_with_memory(&mut memory),
                Err(StorageError::AdmissionFailed)
            ));
            assert_eq!(
                memory.admitted_bytes(),
                0,
                "first copy refuses before payload allocation"
            );
        }
        assert_eq!(refused.live, 0);
        let mut admitted = Grant {
            live: 0,
            limit: usize::MAX,
        };
        {
            let mut memory = Memory::new(&mut admitted);
            let copy = source.clone_with_memory(&mut memory).unwrap();
            assert_eq!(copy, source, "all typed fields and exact English survive");
            let bytes = copy.retained_bytes().unwrap();
            assert_eq!(
                memory.admitted_bytes(),
                bytes,
                "the original grant covers concrete copied capacities and boxes"
            );
            drop(copy);
            memory.release_bytes(bytes).unwrap();
            assert_eq!(memory.admitted_bytes(), 0);
        }
        assert_eq!(admitted.live, 0);
    }

    #[test]
    fn storage_cause_keeps_exact_fields_in_an_admitted_compact_diagnostic() {
        use purrdf_lex::allocation::{CapacityError, CapacitySpace};
        let cause = StorageError::from(
            CapacityError::slot(CapacitySpace::GlobalTerms, u64::MAX, u64::MAX - 1).unwrap_err(),
        );
        let source = RdfDiagnostic::from(cause);
        assert!(size_of::<RdfDiagnostic>() <= 128);
        assert_eq!(source.storage_error(), Some(cause));
        assert_eq!(
            std::error::Error::source(&source)
                .unwrap()
                .downcast_ref::<StorageError>(),
            Some(&cause)
        );
        let mut grant = Grant {
            live: 0,
            limit: usize::MAX,
        };
        let mut memory = Memory::new(&mut grant);
        let copy = source.clone_with_memory(&mut memory).unwrap();
        assert_eq!(copy, source);
        assert_eq!(copy.to_json(), source.to_json());
        let bytes = copy.retained_bytes().unwrap();
        assert_eq!(memory.admitted_bytes(), bytes);
        drop(copy);
        memory.release_bytes(bytes).unwrap();
        memory.admission_mut().limit = bytes - size_of::<StorageError>();
        assert_eq!(
            memory.scope(|memory| source.clone_with_memory(memory)),
            Err(StorageError::AdmissionFailed)
        );
        assert_eq!(memory.admitted_bytes(), 0);
        assert_eq!(source.storage_error(), Some(cause));
    }

    #[test]
    fn shared_attachments_preserve_original_reporting_and_admitted_ownership() {
        use std::hash::Hasher;

        #[derive(Debug, Hash)]
        struct OriginalDiagnostic<'a> {
            severity: RdfSeverity,
            code: &'a String,
            message: &'a String,
            detail: &'a Option<String>,
            location: &'a Option<Box<RdfLocation>>,
            presentation: Option<&'a DiagnosticPresentation>,
            storage: Option<StorageError>,
        }

        let cause = StorageError::AdmissionFailed;
        let source = RdfDiagnostic::from(cause).with_presentation(
            DiagnosticPresentation::new("original-refusal", "original refusal", vec![]).unwrap(),
        );
        let original = OriginalDiagnostic {
            severity: source.severity,
            code: &source.code,
            message: &source.message,
            detail: &source.detail,
            location: &source.location,
            presentation: source.presentation(),
            storage: Some(cause),
        };
        for alternate in [false, true] {
            let (expected, actual) = if alternate {
                (format!("{original:#?}"), format!("{source:#?}"))
            } else {
                (format!("{original:?}"), format!("{source:?}"))
            };
            assert_eq!(
                actual,
                expected.replacen("OriginalDiagnostic", "RdfDiagnostic", 1)
            );
        }
        let mut expected_hash = purrdf_hash::fixed::FixedHasher::default();
        let mut actual_hash = purrdf_hash::fixed::FixedHasher::default();
        original.hash(&mut expected_hash);
        source.hash(&mut actual_hash);
        assert_eq!(actual_hash.finish(), expected_hash.finish());
        assert_eq!(source.storage_error(), Some(cause));
        assert_eq!(
            std::error::Error::source(&source)
                .unwrap()
                .downcast_ref::<StorageError>(),
            Some(&cause)
        );

        let mut grant = Grant {
            live: 0,
            limit: usize::MAX,
        };
        let mut memory = Memory::new(&mut grant);
        let copy = source.clone_with_memory(&mut memory).unwrap();
        let bytes = copy.retained_bytes().unwrap();
        assert_eq!(memory.admitted_bytes(), bytes);
        assert_eq!(copy, source);
        assert_eq!(copy.to_json(), source.to_json());
        assert_eq!(copy.to_string(), source.to_string());
        drop(copy);
        memory.release_bytes(bytes).unwrap();
        memory.admission_mut().limit = bytes - 1;
        assert_eq!(
            memory.scope(|memory| source.clone_with_memory(memory)),
            Err(StorageError::AdmissionFailed)
        );
        assert_eq!(memory.admitted_bytes(), 0);
        assert_eq!(source.storage_error(), Some(cause));

        // The no-attachment constructor and its empty-text copy need no box.
        let ordinary = RdfDiagnostic::error("", "");
        memory.admission_mut().limit = 0;
        let copy = ordinary.clone_with_memory(&mut memory).unwrap();
        assert!(copy.attachments.is_none());
        assert_eq!(copy.retained_bytes(), Some(0));
        assert_eq!(memory.admitted_bytes(), 0);
    }

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
            IriError::BadAuthority(purrdf_iri::AuthorityError::UnterminatedIpLiteral),
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
