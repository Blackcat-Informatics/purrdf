// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fmt;
use std::fmt::Write as _;

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
/// Every field is `Ord` (plain `Option<String>`/`Option<u32>`/`Option<usize>`),
/// so the struct derives a total order directly — the loss ledger relies on
/// this to sort runtime entries deterministically without a separate
/// `display()`-string comparison.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash, PartialOrd, Ord)]
pub struct RdfLocation {
    /// The source file path (physical location anchor).
    pub path: Option<String>,
    /// The 1-based source line within [`path`](Self::path).
    pub line: Option<u32>,
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
    pub gts_term_id: Option<usize>,
    /// The GTS quad index the diagnostic refers to.
    pub gts_quad_index: Option<usize>,
    /// The GTS reifier id the diagnostic refers to.
    pub gts_reifier_id: Option<usize>,
    /// The GTS frame index the diagnostic refers to.
    pub gts_frame_index: Option<usize>,
    /// The GTS segment index the diagnostic refers to.
    pub gts_segment_index: Option<usize>,
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
    /// `physicalLocation` (Task 12).
    pub fn file(path: impl Into<String>) -> Self {
        Self {
            path: Some(path.into()),
            ..Self::default()
        }
    }

    /// Attaches a 1-based source line.
    #[must_use]
    pub fn with_line(mut self, line: u32) -> Self {
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
    pub fn with_gts_term(mut self, term_id: usize) -> Self {
        self.gts_term_id = Some(term_id);
        self
    }

    /// Attaches the GTS quad index the diagnostic refers to.
    #[must_use]
    pub fn with_gts_quad(mut self, quad_index: usize) -> Self {
        self.gts_quad_index = Some(quad_index);
        self
    }

    /// Attaches the GTS reifier id the diagnostic refers to.
    #[must_use]
    pub fn with_gts_reifier(mut self, reifier_id: usize) -> Self {
        self.gts_reifier_id = Some(reifier_id);
        self
    }

    /// Attaches the GTS frame index the diagnostic refers to.
    #[must_use]
    pub fn with_gts_frame(mut self, frame_index: usize) -> Self {
        self.gts_frame_index = Some(frame_index);
        self
    }

    /// Attaches the GTS segment index the diagnostic refers to.
    #[must_use]
    pub fn with_gts_segment(mut self, segment_index: usize) -> Self {
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
        }
    }

    /// An [`RdfSeverity::Error`]-level diagnostic from its code and message.
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(RdfSeverity::Error, code, message)
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
