// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one error type of [`purrdf_core::csv`](super), with where it happened.

use std::fmt;

use purrdf_iri::LineIndex;

/// Where in the input a reading error was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CsvPosition {
    /// The 1-based source row number (CSVW's "source row number"): every row
    /// read counts, skipped, comment and header rows included.
    pub row: u64,
    /// The 1-based line: one plus the number of LINE FEEDs before `byte`.
    pub line: u64,
    /// The 1-based column, in Unicode scalar values from the line start (bytes
    /// that are not UTF-8 count one scalar per replacement character).
    pub column: u32,
    /// The byte offset into the input as given, a byte order mark included.
    pub byte: u64,
}

impl CsvPosition {
    /// Resolve `byte` in `input` for source row `row`. Runs on the error path
    /// only: it scans the input prefix once.
    pub(crate) fn locate(input: &[u8], row: u64, byte: usize) -> Result<Self, CsvErrorKind> {
        let byte = byte.min(input.len());
        let prefix = String::from_utf8_lossy(&input[..byte]);
        let position = LineIndex::new(&prefix)
            .try_locate(&prefix, prefix.len())
            .map_err(|_| CsvErrorKind::ColumnLimit)?;
        Ok(Self {
            row,
            line: position.line,
            column: position.column,
            byte: byte as u64,
        })
    }
}

/// What went wrong.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CsvErrorKind {
    /// The next source row would exceed `u64::MAX`.
    SourceRowExhausted,
    /// An error position exceeds the bounded `u32` Unicode-scalar column space.
    ColumnLimit,
    /// A cell of the row is not UTF-8. `field` is its 0-based index in the row
    /// as parsed (skipped columns included); `valid_up_to` is how many of the
    /// cell's bytes (after quote and escape removal) are valid UTF-8.
    NotUtf8 {
        /// The 0-based index of the first cell that is not UTF-8.
        field: usize,
        /// The length of that cell's longest valid UTF-8 prefix, in bytes.
        valid_up_to: usize,
    },
    /// A line terminator is the empty string.
    EmptyLineTerminator,
    /// The input ends inside a quoted value.
    UnterminatedQuote,
    /// A quote character appears in an unquoted cell that already holds text
    /// (CSVW §8 "parse a row": "If the current cell value is not an empty
    /// string, raise an error"; RFC 4180 §2 rule 5).
    QuoteInUnquotedField,
    /// Text other than the delimiter follows a closing quote (CSVW §8 "parse a
    /// row": "If the remaining string does not start with the delimiter, raise
    /// an error").
    TextAfterClosingQuote,
    /// A record's field count differs from the first record's, in a dialect
    /// that is not [`flexible`](super::Dialect::flexible).
    UnequalLengths {
        /// The first record's field count.
        expected: usize,
        /// This record's field count.
        found: usize,
    },
    /// A record with no fields was written; RFC 4180's `record = field
    /// *(COMMA field)` has at least one.
    EmptyRecord,
    /// A field needs quoting (or holds a quote character that needs escaping)
    /// and the dialect has no way to write it.
    UnquotableField {
        /// The 0-based index of the field in its record.
        field: usize,
    },
    /// The dialect's own fields contradict each other or the byte-level
    /// scanner (a non-ASCII delimiter, a delimiter equal to the quote
    /// character, ...).
    InvalidDialect {
        /// Which rule the dialect breaks.
        reason: &'static str,
    },
    /// The writer's sink failed.
    Io {
        /// The sink's error kind.
        kind: std::io::ErrorKind,
        /// The sink's error, rendered.
        message: String,
    },
}

impl fmt::Display for CsvErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceRowExhausted => f.write_str("CSV source row exceeds u64::MAX"),
            Self::ColumnLimit => f.write_str("CSV source column exceeds u32::MAX"),
            Self::NotUtf8 { field, valid_up_to } => write!(
                f,
                "field {field} is not UTF-8 (valid for its first {valid_up_to} bytes)"
            ),
            Self::EmptyLineTerminator => f.write_str("a line terminator is the empty string"),
            Self::UnterminatedQuote => f.write_str("the input ends inside a quoted value"),
            Self::QuoteInUnquotedField => {
                f.write_str("a quote character appears inside an unquoted field")
            }
            Self::TextAfterClosingQuote => {
                f.write_str("a closing quote is followed by text other than the delimiter")
            }
            Self::UnequalLengths { expected, found } => write!(
                f,
                "the record has {found} fields where the first record has {expected}"
            ),
            Self::EmptyRecord => f.write_str("a record needs at least one field"),
            Self::UnquotableField { field } => write!(
                f,
                "field {field} needs quoting, and the dialect cannot quote or escape it"
            ),
            Self::InvalidDialect { reason } => write!(f, "invalid dialect: {reason}"),
            Self::Io { message, .. } => write!(f, "the output failed: {message}"),
        }
    }
}

/// A refused input, dialect or write, with where it happened.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CsvError {
    kind: CsvErrorKind,
    position: Option<CsvPosition>,
}

impl CsvError {
    /// An error with no input position (a dialect or writer error).
    pub(crate) const fn new(kind: CsvErrorKind) -> Self {
        Self {
            kind,
            position: None,
        }
    }

    /// An error at `position`.
    pub(crate) const fn at(kind: CsvErrorKind, position: CsvPosition) -> Self {
        Self {
            kind,
            position: Some(position),
        }
    }

    /// A sink failure.
    pub(crate) fn io(error: &std::io::Error) -> Self {
        Self::new(CsvErrorKind::Io {
            kind: error.kind(),
            message: error.to_string(),
        })
    }

    /// What went wrong.
    #[must_use]
    pub const fn kind(&self) -> &CsvErrorKind {
        &self.kind
    }

    /// Where in the input it went wrong, for a reading error.
    #[must_use]
    pub const fn position(&self) -> Option<&CsvPosition> {
        self.position.as_ref()
    }
}

impl fmt::Display for CsvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.position {
            Some(position) => write!(
                f,
                "row {} (line {}, column {}, byte {}): {}",
                position.row, position.line, position.column, position.byte, self.kind
            ),
            None => self.kind.fmt(f),
        }
    }
}

impl std::error::Error for CsvError {}
