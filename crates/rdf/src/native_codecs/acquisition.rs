// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original storage and progress authority for native source acquisition.

use std::borrow::Cow;

use purrdf_lex::allocation::{Admission, Memory, OwnedAdmission, StorageError};
use purrdf_lex::control::{ControlledAdmission, ControlledError, Progress, with_memory_control};

use crate::RdfDiagnostic;

/// Native source failures stay distinct from the caller's operational failure.
#[derive(Debug)]
pub enum NativeParseError {
    /// The original RDF diagnostic, including presentation and locations.
    Diagnostic(RdfDiagnostic),
    /// Physical allocation, layout or memory-admission refusal.
    Storage(StorageError),
    /// An inconsistent source association from native lowering or folding.
    SourceAssociation(super::source_occurrence::SourceAssociationError),
    /// Original composite literal grammar or blank-binding failure.
    Composite(purrdf_core::cdt_blank::CdtBlankError),
    /// Controls and exact source occurrences are unsupported for this format.
    UnsupportedFormat(super::NativeRdfFormat),
    /// The original invalid caller-supplied base.
    Base {
        /// Original native IRI failure.
        cause: purrdf_iri::IriError,
    },
    /// Original UTF-8 failure within its document extent.
    Utf8 {
        /// Original standard-library decoder failure.
        cause: std::str::Utf8Error,
        /// Document byte offset of the decoded extent.
        offset: u64,
    },
    /// A source position exceeds its native representation.
    Position {
        /// Original position failure.
        cause: purrdf_iri::PositionError,
        /// Original document line.
        line: u64,
    },
    /// Original IRI failure at a grammar occurrence.
    Iri {
        /// Original native IRI failure.
        cause: purrdf_iri::IriError,
        /// Document line, or zero before a location is available.
        line: u64,
        /// Document column, or zero before a location is available.
        column: u32,
    },
    /// Original native lexical failure and its document position.
    Lexical {
        /// Original native tokenization failure.
        cause: purrdf_sparql_algebra::ParseError,
        /// Document line, or zero before a location is available.
        line: u64,
        /// Document column, or zero before a location is available.
        column: u32,
    },
}

purrdf_lex::variant_from!(NativeParseError {
    Storage(StorageError),
    SourceAssociation(super::source_occurrence::SourceAssociationError),
    Diagnostic(RdfDiagnostic),
});

impl From<purrdf_core::ir::NativeBuildError> for NativeParseError {
    fn from(cause: purrdf_core::ir::NativeBuildError) -> Self {
        match cause {
            purrdf_core::ir::NativeBuildError::Storage(cause) => Self::Storage(cause),
            purrdf_core::ir::NativeBuildError::Diagnostic(cause) => Self::Diagnostic(cause),
        }
    }
}

/// A native parsing refusal or the caller's unchanged progress failure.
pub type TextError<E> = ControlledError<NativeParseError, E>;

impl purrdf_hash::error::StructuredError for NativeParseError {
    purrdf_hash::error_visit! {
        Diagnostic(cause) => "native-parse-diagnostic" { "cause" => Cause(cause) },
        Storage(cause) => "native-parse-storage" { "cause" => Cause(cause) },
        SourceAssociation(cause) => "native-parse-source-association" { "cause" => Cause(cause) },
        Composite(cause) => "native-parse-composite" { "cause" => Cause(cause) },
        UnsupportedFormat(format) => "native-parse-unsupported-format" { "format" => Code(format.media_type()) },
        Base { cause } => "native-parse-base" { "cause" => Cause(cause) },
        Utf8 { cause, offset } => "native-parse-utf8" { "cause" => Cause(cause), "offset" => ByteCount(*offset as u128) },
        Position { cause, line } => "native-parse-position" { "cause" => Cause(cause), "line" => Unsigned(*line as u128) },
        Iri { cause, line, column } => "native-parse-iri" { "cause" => Cause(cause), "line" => Unsigned(*line as u128), "column" => Unsigned(*column as u128) },
        Lexical { cause, line, column } => "native-parse-lexical" { "cause" => Cause(cause), "line" => Unsigned(*line as u128), "column" => Unsigned(*column as u128) },
    }
}

impl core::fmt::Display for NativeParseError {
    fn fmt(&self, output: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Diagnostic(cause) => cause.fmt(output),
            Self::Storage(cause) => cause.fmt(output),
            Self::Composite(cause) => cause.fmt(output),
            Self::SourceAssociation(cause) => cause.fmt(output),
            Self::UnsupportedFormat(format) => write!(
                output,
                "controlled source acquisition does not support {}",
                format.media_type()
            ),
            Self::Base { cause } => write!(output, "invalid base IRI: {cause}"),
            Self::Utf8 { cause, offset } => write!(
                output,
                "invalid UTF-8 in source extent at byte {offset}: {cause}"
            ),
            Self::Position { cause, line } => write!(output, "source line {line}: {cause}"),
            Self::Iri {
                cause,
                line,
                column,
            } => write!(output, "source line {line}, column {column}: {cause}"),
            Self::Lexical {
                cause,
                line,
                column,
            } => write!(output, "source line {line}, column {column}: {cause}"),
        }
    }
}

impl NativeParseError {
    fn native_cause(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Diagnostic(cause) => Some(cause),
            Self::Storage(cause) => Some(cause),
            Self::Composite(cause) => Some(cause),
            Self::SourceAssociation(cause) => Some(cause),
            Self::Base { cause } | Self::Iri { cause, .. } => Some(cause),
            Self::Utf8 { cause, .. } => Some(cause),
            Self::Position { cause, .. } => Some(cause),
            Self::Lexical { cause, .. } => Some(cause),
            Self::UnsupportedFormat(_) => None,
        }
    }
}
purrdf_hash::error_sources!(NativeParseError, native native_cause);

/// The full caller input survives every acquisition refusal, including when no
/// owned copy could be admitted. The original account is retained last.
pub(crate) struct SourceFailure<'a, A: Admission, C: Progress> {
    // Retain the caller's entire input even when the source copy was refused.
    input: &'a [u8],
    cause: TextError<C::Error>,
    // Lifetime owner, last: the first cause dies before its original grant.
    _context: RunContext<A, C>,
}

impl<'a, A: Admission, C: Progress> SourceFailure<'a, A, C> {
    pub(crate) fn unsupported(
        input: &'a [u8],
        format: super::NativeRdfFormat,
        context: RunContext<A, C>,
    ) -> Self {
        Self {
            input,
            cause: NativeParseError::UnsupportedFormat(format).into(),
            _context: context,
        }
    }

    pub(crate) fn input(&self) -> &[u8] {
        self.input
    }

    pub(crate) fn cause(&self) -> &TextError<C::Error> {
        &self.cause
    }

    #[cfg(test)]
    pub(crate) fn admitted_bytes(&self) -> usize {
        self._context.live
    }
}

impl SourceFailure<'_, purrdf_lex::allocation::Resident, purrdf_lex::control::Unmetered> {
    /// Only the existing resident codec detaches its owned diagnostic. A
    /// controlled failure keeps both its first cause and original owner.
    pub(crate) fn into_resident_error(self) -> RdfDiagnostic {
        resident_error(self.cause)
    }
}

/// Source storage is destroyed before its one original admission guard. The
/// scoped view cannot detach text or the account while a parser borrows them.
pub(crate) struct SourceRun<'a, A: Admission, C> {
    source: Cow<'a, str>,
    context: RunContext<A, C>,
}

impl<A: Admission, C: Progress> SourceRun<'_, A, C> {
    pub(crate) fn bytes(&self) -> &[u8] {
        self.source.as_bytes()
    }

    pub(crate) fn with_text<T>(
        &mut self,
        body: impl FnOnce(&str, &mut RunContext<A, C>) -> T,
    ) -> T {
        body(self.source.as_ref(), &mut self.context)
    }
}

/// A temporary parser either owns its existing resident context or borrows the
/// original source owner's context. Borrowing creates no admission guard and
/// updates the same progress carrier and live-byte counter in place.
/// Unlike the core's immutable [`purrdf_core::ownership::OwnedOrBorrowed`], this
/// carrier lends a mutable original context. Both projections must retain that
/// exclusive loan so progress and live admission updates reach the same account.
pub(crate) enum ParserContext<'a, A: Admission, C> {
    Owned(RunContext<A, C>),
    Borrowed(&'a mut RunContext<A, C>),
}

purrdf_lex::variant_from!(impl ['a, A, C] ParserContext<'a, A, C> where [A: Admission] {
    Owned(RunContext<A, C>),
    Borrowed(&'a mut RunContext<A, C>),
});

impl<A: Admission, C> core::ops::Deref for ParserContext<'_, A, C> {
    type Target = RunContext<A, C>;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(context) => context,
            Self::Borrowed(context) => context,
        }
    }
}

impl<A: Admission, C> core::ops::DerefMut for ParserContext<'_, A, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Owned(context) => context,
            Self::Borrowed(context) => context,
        }
    }
}

/// The account itself is owned, never replaced by a measurement after parsing.
/// Parser payload fields must be destroyed before this account is released.
pub(crate) struct RunContext<A: Admission, C> {
    pub(crate) progress: C,
    pub(crate) live: usize,
    // Last: parser payloads and the progress carrier die before final refund.
    pub(crate) admission: OwnedAdmission<A>,
}

impl<A: Admission, C: Progress> RunContext<A, C> {
    pub(crate) fn new(admission: A, progress: C) -> Self {
        Self {
            progress,
            live: 0,
            admission: OwnedAdmission::new(admission),
        }
    }

    pub(crate) fn charge(&mut self, units: u64) -> Result<(), TextError<C::Error>> {
        self.progress
            .charge(units)
            .map_err(ControlledError::Control)
    }

    pub(crate) fn native<T>(
        &mut self,
        body: impl FnOnce(
            &mut Memory<'_, ControlledAdmission<'_, OwnedAdmission<A>, C>>,
        ) -> Result<T, NativeParseError>,
    ) -> Result<T, TextError<C::Error>> {
        let mut memory = Memory::resume(&mut self.admission, self.live);
        let result = with_memory_control(&mut memory, &mut self.progress, body);
        self.live = memory.admitted_bytes();
        result
    }

    pub(crate) fn memory<T>(
        &mut self,
        body: impl FnOnce(
            &mut Memory<'_, ControlledAdmission<'_, OwnedAdmission<A>, C>>,
        ) -> Result<T, StorageError>,
    ) -> Result<T, TextError<C::Error>> {
        self.native(|memory| body(memory).map_err(NativeParseError::Storage))
    }

    /// Destroy native temporary owners before shrinking their original grant.
    /// Cleanup polls no work/stop callback and cannot manufacture a success.
    pub(crate) fn release(
        &mut self,
        body: impl FnOnce(&mut Memory<'_, OwnedAdmission<A>>) -> Result<(), StorageError>,
    ) -> Result<(), TextError<C::Error>> {
        let mut memory = Memory::resume(&mut self.admission, self.live);
        let result = body(&mut memory);
        self.live = memory.admitted_bytes();
        result.map_err(|cause| ControlledError::Native(NativeParseError::Storage(cause)))
    }

    /// Native diagnostic construction is the failure path itself. A completed
    /// diagnostic is returned as Native, so no success publication poll can
    /// replace it after it has been fully constructed.
    pub(crate) fn diagnostic(
        &mut self,
        body: impl FnOnce(
            &mut Memory<'_, ControlledAdmission<'_, OwnedAdmission<A>, C>>,
        ) -> Result<RdfDiagnostic, StorageError>,
    ) -> TextError<C::Error> {
        let result: Result<core::convert::Infallible, _> = self.native(|memory| {
            let diagnostic = body(memory).map_err(NativeParseError::Storage)?;
            Err(NativeParseError::Diagnostic(diagnostic))
        });
        match result {
            Err(cause) => cause,
            Ok(impossible) => match impossible {},
        }
    }

    pub(crate) fn iri<T>(
        &mut self,
        line: u64,
        column: u32,
        body: impl FnOnce(
            &mut Memory<'_, ControlledAdmission<'_, OwnedAdmission<A>, C>>,
        ) -> Result<T, purrdf_iri::IriReadError>,
    ) -> Result<T, TextError<C::Error>> {
        self.native(|memory| {
            body(memory).map_err(|cause| match cause {
                purrdf_iri::IriReadError::Storage(cause) => NativeParseError::Storage(cause),
                purrdf_iri::IriReadError::Lexical(cause) => NativeParseError::Iri {
                    cause,
                    line,
                    column,
                },
            })
        })
    }

    /// Validate and retain original UTF-8 under this same parser account.
    /// Resident execution borrows the native full-buffer result. Governed
    /// execution copies only bounded native-validated extents; an incomplete
    /// scalar at an extent boundary stays in the original input for the next
    /// extent. No replacement text, separate Unicode grammar or unchecked
    /// reconstruction is involved.
    pub(crate) fn source<'a>(
        mut self,
        input: &'a [u8],
    ) -> Result<SourceRun<'a, A, C>, SourceFailure<'a, A, C>> {
        if !C::ENABLED {
            return match std::str::from_utf8(input) {
                Ok(text) => Ok(SourceRun {
                    source: Cow::Borrowed(text),
                    context: self,
                }),
                Err(cause) => Err(SourceFailure {
                    input,
                    cause: ControlledError::Native(NativeParseError::Utf8 { cause, offset: 0 }),
                    _context: self,
                }),
            };
        }
        let mut text = String::new();
        let result = (|| {
            self.charge(0)?;
            // The exact source layout is admitted before allocation. Reserving
            // the empty destination once avoids unbounded old-buffer copies
            // during growth; each subsequent source copy is a bounded extent.
            self.memory(|memory| memory.reserve_string(&mut text, input.len()))?;
            let mut at = 0;
            while at < input.len() {
                let end = at.saturating_add(256).min(input.len());
                self.charge((end - at) as u64)?;
                let extent = &input[at..end];
                let valid = match std::str::from_utf8(extent) {
                    Ok(valid) => valid,
                    Err(cause) if cause.error_len().is_none() && end < input.len() => {
                        // The original kernel identified this valid prefix. A
                        // bounded second native call obtains its safe str view;
                        // only the short incomplete suffix is visited next time.
                        let prefix = &extent[..cause.valid_up_to()];
                        self.charge(prefix.len() as u64)?;
                        std::str::from_utf8(prefix).map_err(|cause| {
                            ControlledError::Native(NativeParseError::Utf8 {
                                cause,
                                offset: at as u64,
                            })
                        })?
                    }
                    Err(cause) => {
                        return Err(ControlledError::Native(NativeParseError::Utf8 {
                            cause,
                            offset: at as u64,
                        }));
                    }
                };
                self.memory(|memory| memory.push_str_with_progress(&mut text, valid))?;
                at += valid.len();
            }
            self.charge(0)
        })();
        match result {
            Ok(()) => Ok(SourceRun {
                source: Cow::Owned(text),
                context: self,
            }),
            Err(cause) => {
                // The original refusal wins over any shrink refusal. Cleanup
                // polls no progress authority and drops the partial copy first.
                let _ = self.release(|memory| memory.release_string(text));
                Err(SourceFailure {
                    input,
                    cause,
                    _context: self,
                })
            }
        }
    }

    pub(crate) fn tokenize<'a>(
        &mut self,
        text: &'a str,
    ) -> Result<Vec<purrdf_sparql_algebra::lexer::Spanned<'a>>, TextError<C::Error>> {
        use purrdf_sparql_algebra::lexer::{
            LexerOptions, tokenize_with_control, tokenize_with_memory,
        };
        let mut memory = Memory::resume(&mut self.admission, self.live);
        let options = LexerOptions {
            pn_local_allows_slash: true,
        };
        let result = if C::ENABLED {
            tokenize_with_control(text, options, &mut memory, &mut |units| {
                self.progress.charge(units)
            })
        } else {
            tokenize_with_memory(text, options, &mut memory).map_err(ControlledError::Native)
        };
        self.live = memory.admitted_bytes();
        result.map_err(|cause| match cause {
            ControlledError::Control(cause) => ControlledError::Control(cause),
            ControlledError::Native(cause) => {
                // Line/column rendering is the native parser's reporting boundary.
                ControlledError::Native(NativeParseError::Lexical {
                    cause,
                    line: 0,
                    column: 0,
                })
            }
        })
    }
}

pub(crate) fn resident_error(error: TextError<core::convert::Infallible>) -> RdfDiagnostic {
    match error {
        ControlledError::Native(NativeParseError::Diagnostic(diagnostic)) => diagnostic,
        ControlledError::Native(NativeParseError::Storage(error)) => error.into(),
        ControlledError::Native(NativeParseError::Composite(error)) => error.into(),
        ControlledError::Native(NativeParseError::SourceAssociation(error)) => {
            RdfDiagnostic::error("native-source-association", error.to_string())
        }
        ControlledError::Native(NativeParseError::UnsupportedFormat(format)) => {
            RdfDiagnostic::error(
                "native-control-format",
                format!(
                    "controlled source acquisition does not support {}",
                    format.media_type()
                ),
            )
        }
        ControlledError::Native(NativeParseError::Base { cause }) => RdfDiagnostic::error(
            cause.diagnostic_code(),
            format!("invalid base IRI: {cause}"),
        ),
        ControlledError::Native(NativeParseError::Utf8 { cause, offset }) => {
            super::stream::utf8_error(&cause, offset)
        }
        ControlledError::Native(NativeParseError::Position { cause, line }) => {
            RdfDiagnostic::error("native-codec-limit", cause.to_string())
                .with_location(crate::RdfLocation::default().with_line(line))
        }
        ControlledError::Native(NativeParseError::Iri {
            cause,
            line,
            column,
        }) => super::text_parse::iri_err_at(&cause, line, column),
        ControlledError::Native(NativeParseError::Lexical {
            cause,
            line,
            column,
        }) => super::text_parse::err_at(cause.to_string(), line, column),
        ControlledError::Control(impossible) => match impossible {},
    }
}
