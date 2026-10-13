// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Format-neutral emission and digest-bound references under the one cover law.

use std::ops::Range;

use crate::ContentDigest;

use super::{ByteSpan, ReconstructError, reconstruct_bytes};

/// An invalid emitter range or a cover that does not satisfy the decode law.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EmitError {
    /// An inverted range or one beyond the supplied source.
    Range {
        /// Declared first byte.
        start: u64,
        /// Declared exclusive end.
        end: u64,
        /// Actual source length.
        byte_length: u64,
    },
    /// The original decoder's typed refusal, with no translation or repair.
    Cover(ReconstructError),
}

impl std::fmt::Display for EmitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Range {
                start,
                end,
                byte_length,
            } => {
                write!(
                    formatter,
                    "range {start}..{end} is outside source length {byte_length}"
                )
            }
            Self::Cover(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for EmitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Cover(error) => Some(error),
            Self::Range { .. } => None,
        }
    }
}

purrdf_lex::variant_from!(EmitError { Cover(ReconstructError) });

/// A complete, verified cover whose verbatim spans borrow their original bytes.
///
/// Content identity is the source digest, independent of the emitter, its
/// vocabulary, span segmentation, execution target or occurrence order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ByteCover<'a> {
    byte_length: u64,
    source_digest: ContentDigest,
    spans: Vec<ByteSpan<'a>>,
}

impl<'a> ByteCover<'a> {
    /// Cover opaque bytes with one unchanged whole-source span, including empty
    /// content. No format recognition or UTF-8 assumption is involved.
    pub fn identity(source: &'a [u8]) -> Self {
        Self {
            byte_length: source.len() as u64,
            source_digest: ContentDigest::of(source),
            spans: vec![ByteSpan {
                byte_start: 0,
                byte_end: source.len() as u64,
                bytes: source,
                continues: None,
            }],
        }
    }

    /// Exact source length in bytes.
    pub const fn byte_length(&self) -> u64 {
        self.byte_length
    }

    /// Identity proved by reconstruction, using the kernel's content digest.
    pub const fn source_digest(&self) -> ContentDigest {
        self.source_digest
    }

    /// Emitted occurrences in producer order, including declared continuations.
    pub fn spans(&self) -> &[ByteSpan<'a>] {
        &self.spans
    }

    /// Reconstruct through the one decoder, verifying the source identity again.
    ///
    /// # Errors
    /// Returns the original decoder's typed coverage and digest refusals.
    pub fn reconstruct(&self) -> Result<Vec<u8>, ReconstructError> {
        reconstruct_bytes(self.byte_length, &self.source_digest, &self.spans)
    }

    /// Bind an arbitrary byte window to this content identity. The window need
    /// not coincide with an occurrence or a Unicode scalar boundary.
    ///
    /// # Errors
    /// Refuses an inverted range or one past the covered source.
    pub fn reference(&self, range: Range<u64>) -> Result<SpanReference, EmitError> {
        validate_range(range.clone(), self.byte_length)?;
        Ok(SpanReference {
            source_digest: self.source_digest,
            byte_start: range.start,
            byte_end: range.end,
        })
    }
}

/// The shared emit side: a codec declares source ranges and continuation edges.
/// Every span quotes the source directly; finalization applies the decode law.
#[derive(Debug)]
pub struct CoverBuilder<'a> {
    source: &'a [u8],
    spans: Vec<ByteSpan<'a>>,
}

impl<'a> CoverBuilder<'a> {
    /// Start a format-neutral cover of caller-supplied verbatim bytes.
    pub const fn new(source: &'a [u8]) -> Self {
        Self {
            source,
            spans: Vec::new(),
        }
    }

    /// Declare an occurrence and, when applicable, the strictly earlier range it
    /// continues. No overlap declaration is inferred from byte agreement.
    ///
    /// # Errors
    /// Refuses an inverted range or a range outside the actual source. The final
    /// coverage and continuation relationship are checked by [`Self::finish`].
    pub fn emit(
        &mut self,
        range: Range<usize>,
        continues: Option<Range<usize>>,
    ) -> Result<usize, EmitError> {
        let byte_length = self.source.len() as u64;
        validate_range(range.start as u64..range.end as u64, byte_length)?;
        if let Some(previous) = &continues {
            validate_range(previous.start as u64..previous.end as u64, byte_length)?;
        }
        let index = self.spans.len();
        self.spans.push(ByteSpan {
            byte_start: range.start as u64,
            byte_end: range.end as u64,
            bytes: &self.source[range],
            continues: continues.map(|previous| (previous.start as u64, previous.end as u64)),
        });
        Ok(index)
    }

    /// Complete the cover only when the shared decoder proves it whole and
    /// lawful. A codec cannot publish a gap or an undeclared overlap.
    ///
    /// # Errors
    /// Returns the original typed cover-law refusal for incomplete or invalid
    /// declarations, without silently inserting spans or guessing continuations.
    pub fn finish(self) -> Result<ByteCover<'a>, EmitError> {
        let cover = ByteCover {
            byte_length: self.source.len() as u64,
            source_digest: ContentDigest::of(self.source),
            spans: self.spans,
        };
        cover.reconstruct()?;
        Ok(cover)
    }
}

fn validate_range(range: Range<u64>, byte_length: u64) -> Result<(), EmitError> {
    if range.start > range.end || range.end > byte_length {
        Err(EmitError::Range {
            start: range.start,
            end: range.end,
            byte_length,
        })
    } else {
        Ok(())
    }
}

/// A byte window under the identity of covered content, reusable by statements
/// describing repairs, extracted regions, OCR or embedding windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpanReference {
    /// The source content identity, not the identity of a copied window.
    pub source_digest: ContentDigest,
    /// Inclusive first byte of the window.
    pub byte_start: u64,
    /// Exclusive end of the window.
    pub byte_end: u64,
}

impl SpanReference {
    /// Resolve against the actual source bytes, proving their identity before
    /// returning any part. This also supports retained opaque source content.
    ///
    /// # Errors
    /// Refuses a mismatching source digest before slicing; refuses out-of-range
    /// windows under the actual source length, including on 32-bit targets.
    pub fn resolve_source<'a>(&self, source: &'a [u8]) -> Result<&'a [u8], EmitError> {
        let digest = ContentDigest::of(source);
        if digest != self.source_digest {
            return Err(ReconstructError::DigestMismatch {
                expected: self.source_digest.to_hex(),
                reconstructed: digest.to_hex(),
            }
            .into());
        }
        validate_range(self.byte_start..self.byte_end, source.len() as u64)?;
        Ok(&source[self.byte_start as usize..self.byte_end as usize])
    }

    /// Resolve a byte window from a lawful cover and prove the cover's bytes
    /// rather than trusting its asserted digest.
    ///
    /// # Errors
    /// Refuses a broken cover, different content identity or invalid window.
    pub fn resolve(&self, cover: &ByteCover<'_>) -> Result<Vec<u8>, EmitError> {
        let source = cover.reconstruct()?;
        Ok(self.resolve_source(&source)?.to_vec())
    }
}

/// A raw delimited-record codec using the shared emitter. Record and separator
/// occurrences remain distinct, including empty records and a final separator.
/// The caller supplies the byte delimiter; no quoting or format is guessed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DelimitedCover<'a> {
    /// The complete verbatim source cover.
    pub cover: ByteCover<'a>,
    /// Record windows in original occurrence order, including empty records.
    pub records: Vec<Range<usize>>,
    /// Every delimiter occurrence, in order.
    pub separators: Vec<Range<usize>>,
}

impl<'a> DelimitedCover<'a> {
    /// Describe records under an explicitly selected delimiter without changing
    /// payload bytes, line endings or empty record multiplicity.
    ///
    /// # Errors
    /// Returns a cover-law refusal if the emitted partition is not exact.
    pub fn analyze(source: &'a [u8], delimiter: u8) -> Result<Self, EmitError> {
        let mut builder = CoverBuilder::new(source);
        let mut records = Vec::new();
        let mut separators = Vec::new();
        let mut start = 0;
        while let Some(offset) = purrdf_lex::scan::find_byte(&source[start..], delimiter) {
            let at = start + offset;
            let record = start..at;
            builder.emit(record.clone(), None)?;
            records.push(record);
            let separator = at..at + 1;
            builder.emit(separator.clone(), None)?;
            separators.push(separator);
            start = at + 1;
        }
        let last = start..source.len();
        builder.emit(last.clone(), None)?;
        records.push(last);
        Ok(Self {
            cover: builder.finish()?,
            records,
            separators,
        })
    }
}
