// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Owned parser causes and explicit recovery coordinates.

use std::{fmt, ops::Range};

use purrdf_core::RdfDiagnostic;
use purrdf_iri::{Position, PositionError};

/// The native text carrier that refused a source extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextFormat {
    /// RDF 1.2 Turtle.
    Turtle,
    /// RDF 1.2 N-Triples.
    NTriples,
}

impl fmt::Display for TextFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Turtle => "Turtle",
            Self::NTriples => "N-Triples",
        })
    }
}

impl purrdf_hash::error::StructuredError for TextFormat {
    purrdf_hash::error_visit! {
        Turtle => "shacl-text-turtle" {},
        NTriples => "shacl-text-ntriples" {},
    }
}

/// One original parser refusal and its mapping back to the offered document.
///
/// The native diagnostic keeps its original candidate-relative location. The
/// separate source position removes replayed directives and trimmed layout.
/// No source text is borrowed, so this record survives the caller's input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDiagnostic {
    pub(super) format: TextFormat,
    pub(super) cause: RdfDiagnostic,
    pub(super) source_range: Range<usize>,
    pub(super) replay_bytes: usize,
    pub(super) source_position: Result<Option<Position>, PositionError>,
}

impl TextDiagnostic {
    /// The original native diagnostic, including its original parser cause.
    #[must_use]
    pub const fn cause(&self) -> &RdfDiagnostic {
        &self.cause
    }

    /// The carrier used by this parse.
    #[must_use]
    pub const fn format(&self) -> TextFormat {
        self.format
    }

    /// Exact original byte extent supplied after removing surrounding layout.
    #[must_use]
    pub fn source_range(&self) -> Range<usize> {
        self.source_range.clone()
    }

    /// Bytes of synthetic directives preceding the original extent.
    #[must_use]
    pub const fn replay_bytes(&self) -> usize {
        self.replay_bytes
    }

    /// Original document position, when the parser reported a source location.
    ///
    /// # Errors
    /// Preserves a native position refusal if the original column cannot fit.
    pub const fn source_position(&self) -> Result<Option<Position>, PositionError> {
        self.source_position
    }
}

impl fmt::Display for TextDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} parse error: {}", self.format, self.cause)
    }
}

impl std::error::Error for TextDiagnostic {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}

impl purrdf_hash::error::StructuredError for TextDiagnostic {
    purrdf_hash::error_visit! {
        @record self { format, cause, source_range, replay_bytes, source_position }
        => "shacl-text-diagnostic" {
            "format" => Record(&self.format), "cause" => Cause(&self.cause),
            "source_start" => Unsigned(self.source_range.start as u128),
            "source_end" => Unsigned(self.source_range.end as u128),
            "replay_bytes" => Unsigned(self.replay_bytes as u128),
            "source_byte_offset" => Value(self.source_position.ok().flatten().map_or(
                purrdf_hash::error::ErrorValue::Absent,
                |position| purrdf_hash::error::ErrorValue::Unsigned(position.byte_offset as u128))),
            "source_line" => Value(self.source_position.ok().flatten().map_or(
                purrdf_hash::error::ErrorValue::Absent,
                |position| purrdf_hash::error::ErrorValue::Unsigned(u128::from(position.line)))),
            "source_column" => Value(self.source_position.ok().flatten().map_or(
                purrdf_hash::error::ErrorValue::Absent,
                |position| purrdf_hash::error::ErrorValue::Unsigned(u128::from(position.column)))),
            "position_cause" => Value(self.source_position.as_ref().err().map_or(
                purrdf_hash::error::ErrorValue::Absent,
                |cause| purrdf_hash::error::ErrorValue::Cause(cause))),
        }
    }
}

/// Ordered original parser diagnostics from a failed text ingress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextErrors {
    pub(super) entries: Vec<TextDiagnostic>,
}

impl TextErrors {
    /// The original diagnostic sequence in document order.
    #[must_use]
    pub fn diagnostics(&self) -> &[TextDiagnostic] {
        &self.entries
    }
}

impl std::ops::Deref for TextErrors {
    type Target = [TextDiagnostic];

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl fmt::Display for TextErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, diagnostic) in self.entries.iter().enumerate() {
            if index != 0 {
                f.write_str("\n")?;
            }
            diagnostic.fmt(f)?;
        }
        Ok(())
    }
}

impl std::error::Error for TextErrors {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.entries.first().map(|diagnostic| diagnostic as _)
    }
}

impl purrdf_hash::error::StructuredError for TextErrors {
    purrdf_hash::error_visit! {
        @record self { entries } => "shacl-text-errors" {
            "diagnostics" => List(self.entries.iter().map(|diagnostic| purrdf_hash::error::ErrorValue::Cause(diagnostic))),
        }
    }
}

pub(super) fn mapped(
    format: TextFormat,
    cause: RdfDiagnostic,
    source_range: Range<usize>,
    replay_bytes: usize,
    candidate: &str,
    cursor: &mut purrdf_iri::PositionCursor<'_>,
) -> TextDiagnostic {
    let offset = cause.location.as_ref().and_then(|location| {
        let line = usize::try_from(location.line?.checked_sub(1)?).ok()?;
        let column = usize::try_from(location.column?.checked_sub(1)?).ok()?;
        let mut lines = candidate.split_inclusive('\n');
        let before = lines.by_ref().take(line).map(str::len).sum::<usize>();
        let text = lines.next().unwrap_or("");
        let within = text
            .char_indices()
            .nth(column)
            .map_or(text.len(), |(at, _)| at);
        let original = (before + within).checked_sub(replay_bytes)?;
        Some(source_range.start + original.min(source_range.len()))
    });
    let source_position = offset.map_or(Ok(None), |offset| {
        cursor
            .advance(offset, &mut purrdf_lex::control::Unmetered)
            .map(Some)
            .map_err(|failure| match failure {
                purrdf_lex::control::ControlledError::Native(cause) => cause,
                purrdf_lex::control::ControlledError::Control(cause) => match cause {},
            })
    });
    TextDiagnostic {
        format,
        cause,
        source_range,
        replay_bytes,
        source_position,
    }
}

pub(super) fn whole(format: TextFormat, cause: RdfDiagnostic, source: &str) -> TextErrors {
    TextErrors {
        entries: vec![mapped(
            format,
            cause,
            0..source.len(),
            0,
            source,
            &mut purrdf_iri::PositionCursor::new(source),
        )],
    }
}
