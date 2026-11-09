// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit GB 18030 acquisition into the existing native RDF grammars.

use super::{HexTuplesStreamParser, LineEnd, LineStreamParser};
use crate::{RdfDataset, RdfDiagnostic};
use purrdf_lex::allocation::{Admission, Memory, StorageError};
use purrdf_lex::control::Progress;
use purrdf_lex::gb18030::{
    Direction, Emission, Extent, Limits, Profile, StreamError, Totals, Transcoder,
};
use std::io::Read;
use std::sync::Arc;

mod error;
pub use error::EncodedReadError;

const INPUT_WINDOW: usize = 8192;

/// Explicit encoding and conversion-workspace ceilings; no encoding default.
#[derive(Debug, Clone, Copy)]
pub struct EncodedReadOptions {
    /// Complete mapping edition selected by the caller.
    pub profile: Profile,
    /// Whole-stream source and UTF-8 output bounds.
    pub limits: Limits,
    /// Maximum decoded physical line bytes for line-oriented grammars.
    pub line_bytes: usize,
    /// Maximum conversion workspace, including the fixed input window, codec,
    /// line/document text and source maps, including replacement allocation peaks.
    /// Parser state and the resulting dataset are separate existing resources.
    pub workspace_bytes: usize,
}

/// Complete native dataset and exact conversion work, without changing ParseOutcome.
#[derive(Debug)]
pub struct EncodedReadOutcome {
    /// Dataset produced by the existing native grammar and statement-layer fold.
    pub dataset: Arc<RdfDataset>,
    /// Original input bytes, UTF-8 bytes and scalar emissions.
    pub totals: Totals,
    /// Explicit edition used for every contribution.
    pub profile: Profile,
}

#[derive(Clone, Copy)]
struct Scalar {
    value: char,
    source: Extent,
}

struct Input<R> {
    reader: R,
    bytes: [u8; INPUT_WINDOW],
    used: usize,
    available: usize,
    codec: Transcoder,
    totals: Totals,
    eof: bool,
    finished: bool,
}

impl<R: Read> Input<R> {
    fn peek<C: Progress + ?Sized, O>(
        &mut self,
        progress: &mut C,
    ) -> Result<Option<u8>, EncodedReadError<C::Error, O>> {
        if self.used == self.available && !self.eof {
            self.available = loop {
                progress.charge(0).map_err(EncodedReadError::Control)?;
                match self.reader.read(&mut self.bytes) {
                    Ok(available) => break available,
                    Err(cause) if cause.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(cause) => {
                        return Err(EncodedReadError::Read {
                            cause,
                            offset: self.totals.input_bytes,
                        });
                    }
                }
            };
            if self.available > self.bytes.len() {
                return Err(EncodedReadError::ReadCount {
                    actual: self.available,
                    maximum: self.bytes.len(),
                });
            }
            self.used = 0;
            self.eof = self.available == 0;
        }
        Ok((!self.eof).then(|| self.bytes[self.used]))
    }

    fn next<C: Progress + ?Sized, O>(
        &mut self,
        progress: &mut C,
        emit: &mut impl FnMut(Emission<'_>) -> Result<(), O>,
    ) -> Result<Option<Scalar>, EncodedReadError<C::Error, O>> {
        if self.finished {
            return Ok(None);
        }
        loop {
            let Some(byte) = self.peek(progress)? else {
                self.totals = self
                    .codec
                    .push(&[], true, progress, |_| Ok::<_, O>(()))
                    .map_err(EncodedReadError::Codec)?;
                self.finished = true;
                return Ok(None);
            };
            self.used += 1;
            let mut scalar = None;
            self.totals = self
                .codec
                .push(&[byte], false, progress, |item| {
                    emit(item)?;
                    scalar = Some(Scalar {
                        value: item.scalar,
                        source: item.source,
                    });
                    Ok(())
                })
                .map_err(EncodedReadError::Codec)?;
            if scalar.is_some() {
                return Ok(scalar);
            }
        }
    }
}

#[derive(Default)]
struct Workspace {
    text: String,
    sources: Vec<Extent>,
    end: u64,
}

impl Workspace {
    fn check<E, O>(
        &self,
        additional: usize,
        options: EncodedReadOptions,
    ) -> Result<(), EncodedReadError<E, O>> {
        let required = self
            .text
            .capacity()
            .checked_add(
                self.sources
                    .capacity()
                    .checked_mul(size_of::<Extent>())
                    .ok_or(StorageError::SizeOverflow)?,
            )
            .and_then(|n| n.checked_add(size_of::<Input<()>>() + size_of::<Self>()))
            .and_then(|n| n.checked_add(additional))
            .ok_or(StorageError::SizeOverflow)?;
        if required > options.workspace_bytes {
            return Err(EncodedReadError::Limit {
                resource: "transcode-workspace-bytes",
                required: required as u64,
                limit: options.workspace_bytes as u64,
            });
        }
        Ok(())
    }

    fn growth<E, O>(
        &self,
        minimum: usize,
        preferred: usize,
        options: EncodedReadOptions,
    ) -> Result<usize, EncodedReadError<E, O>> {
        self.check(minimum, options)?;
        let retained = self.text.capacity()
            + self.sources.capacity() * size_of::<Extent>()
            + size_of::<Input<()>>()
            + size_of::<Self>();
        Ok(preferred.min(options.workspace_bytes - retained))
    }

    fn append<A: Admission + ?Sized, C: Progress + ?Sized, O>(
        &mut self,
        scalar: Scalar,
        line: bool,
        options: EncodedReadOptions,
        memory: &mut Memory<'_, A>,
        progress: &mut C,
    ) -> Result<(), EncodedReadError<C::Error, O>> {
        let required = self
            .text
            .len()
            .checked_add(scalar.value.len_utf8())
            .ok_or(StorageError::SizeOverflow)?;
        if line && required > options.line_bytes {
            return Err(EncodedReadError::Limit {
                resource: "transcode-line-bytes",
                required: required as u64,
                limit: options.line_bytes as u64,
            });
        }
        if required > self.text.capacity() {
            // Leave room for the next map slot after the old text buffer dies.
            let map_next = if self.sources.len() == self.sources.capacity() {
                self.sources
                    .len()
                    .checked_add(1)
                    .and_then(|n| n.checked_mul(size_of::<Extent>()))
                    .ok_or(StorageError::SizeOverflow)?
            } else {
                0
            };
            let reserve_map = map_next.saturating_sub(self.text.capacity());
            let minimum = required
                .checked_add(reserve_map)
                .ok_or(StorageError::SizeOverflow)?;
            let preferred = required
                .max(self.text.capacity().saturating_mul(2))
                .max(64)
                .checked_add(reserve_map)
                .ok_or(StorageError::SizeOverflow)?;
            let capacity = self.growth(minimum, preferred, options)? - reserve_map;
            progress
                .charge(self.text.len() as u64)
                .map_err(EncodedReadError::Control)?;
            memory.reserve_string(&mut self.text, capacity)?;
        }
        if self.sources.len() == self.sources.capacity() {
            let minimum = self
                .sources
                .len()
                .checked_add(1)
                .ok_or(StorageError::SizeOverflow)?;
            let preferred = minimum
                .max(self.sources.capacity().saturating_mul(2))
                .max(16)
                .checked_mul(size_of::<Extent>())
                .ok_or(StorageError::SizeOverflow)?;
            let capacity = self.growth(
                minimum
                    .checked_mul(size_of::<Extent>())
                    .ok_or(StorageError::SizeOverflow)?,
                preferred,
                options,
            )? / size_of::<Extent>();
            progress
                .charge(self.sources.len() as u64)
                .map_err(EncodedReadError::Control)?;
            memory.reserve(&mut self.sources, capacity)?;
        }
        self.text.push(scalar.value);
        self.sources.push(scalar.source);
        self.end = scalar.source.end;
        Ok(())
    }

    fn source_at(&self, line: u64, column: u32) -> Option<Extent> {
        if line == 0 || column == 0 {
            return None;
        }
        let mut current_line = 1u64;
        let mut current_column = 1u32;
        // Native LineIndex/PositionCursor count LF only; CR remains a scalar
        // in a buffered grammar. Line-oriented callers pass already split text.
        for (index, scalar) in self.text.chars().enumerate() {
            if current_line == line && current_column == column {
                return self.sources.get(index).copied();
            }
            if scalar == '\n' {
                current_line = current_line.checked_add(1)?;
                current_column = 1;
            } else {
                current_column = current_column.checked_add(1)?;
            }
        }
        (current_line == line && current_column == column).then_some(Extent {
            start: self.end,
            end: self.end,
        })
    }

    fn diagnostic<E, O>(
        &self,
        diagnostic: RdfDiagnostic,
        line: Option<u64>,
    ) -> EncodedReadError<E, O> {
        let source = diagnostic
            .location
            .as_ref()
            .and_then(|location| self.source_at(line.or(location.line)?, location.column?));
        EncodedReadError::Parse { diagnostic, source }
    }

    fn release<A: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, A>,
    ) -> Result<(), StorageError> {
        let text = memory.release_string(self.text);
        let sources = memory.release_vec(self.sources);
        text.and(sources)
    }
}

fn next_line<R: Read, A: Admission + ?Sized, C: Progress + ?Sized, O>(
    input: &mut Input<R>,
    workspace: &mut Workspace,
    end: LineEnd,
    options: EncodedReadOptions,
    memory: &mut Memory<'_, A>,
    progress: &mut C,
    emit: &mut impl FnMut(Emission<'_>) -> Result<(), O>,
) -> Result<bool, EncodedReadError<C::Error, O>> {
    workspace.text.clear();
    workspace.sources.clear();
    workspace.end = input.totals.input_bytes;
    loop {
        let Some(scalar) = input.next(progress, emit)? else {
            return Ok(!workspace.text.is_empty());
        };
        match (end, scalar.value) {
            (_, '\n') => {
                if matches!(end, LineEnd::Ndjson) && workspace.text.ends_with('\r') {
                    workspace.text.pop();
                    if let Some(cr) = workspace.sources.pop() {
                        workspace.end = cr.start;
                    }
                }
                return Ok(true);
            }
            (LineEnd::RdfEol, '\r') => {
                // LF is ASCII in both encodings and cannot be inside a GB sequence.
                // Peek raw bytes only: decoding the next line could mask this
                // line's grammar failure with a later malformed scalar.
                if input.peek(progress)? == Some(b'\n') {
                    input.next(progress, emit)?;
                }
                return Ok(true);
            }
            _ => workspace.append(scalar, true, options, memory, progress)?,
        }
    }
}

/// Parse explicitly encoded input through the existing native RDF grammars.
///
/// Every decoded scalar reaches `contribution` with exact original bytes. The
/// callback owns any retained mapping and its original output/admission errors.
/// Conversion text/maps use `memory` and are destroyed and refunded before return.
/// Line grammars retain one physical line; CR lookahead inspects raw bytes. The
/// fixed 8 KiB source window may prefetch raw bytes, but does not decode ahead.
/// Other grammars retain the complete admitted UTF-8 text/map under the explicit
/// workspace and output ceilings, as their existing parser requires. The native
/// grammar state and returned dataset retain their existing resident ownership.
///
/// Scalar columns remain native columns; an outer parse refusal adds its exact
/// original scalar extent (or end boundary) when the native diagnostic supplies
/// a position. No interior output byte is asserted to be an original boundary.
/// UTF-8 defaults and existing public option/outcome structures are unchanged.
///
/// # Errors
/// Preserves original codec, work/cancellation, mapping-output, allocation,
/// reader and native parser causes. No failed conversion yields a dataset.
pub fn parse_dataset_from_encoded_reader<
    R: Read,
    A: Admission + ?Sized,
    C: Progress + ?Sized,
    O,
>(
    reader: R,
    media_type: &str,
    base_iri: Option<&str>,
    options: EncodedReadOptions,
    memory: &mut Memory<'_, A>,
    progress: &mut C,
    mut contribution: impl FnMut(Emission<'_>) -> Result<(), O>,
) -> Result<EncodedReadOutcome, EncodedReadError<C::Error, O>> {
    let format = super::classify(media_type).map_err(|diagnostic| EncodedReadError::Parse {
        diagnostic,
        source: None,
    })?;
    let mut workspace = Workspace::default();
    workspace.check(0, options)?;
    let mut input = Input {
        reader,
        bytes: [0; INPUT_WINDOW],
        used: 0,
        available: 0,
        codec: Transcoder::new(options.profile, Direction::Decode, options.limits),
        totals: Totals::default(),
        eof: false,
        finished: false,
    };
    // Keep conversion ownership outside the guard so an unwind is diagnosed
    // through the native codec boundary before both buffers are refunded.
    let result = super::super::parse::catch_codec_panic(format, || {
        Ok((|| {
            let dataset = if format.is_line_oriented() {
                let base = super::super::parse::base_scope_for(base_iri).map_err(|diagnostic| {
                    EncodedReadError::Parse {
                        diagnostic,
                        source: None,
                    }
                })?;
                match format {
                    super::NativeRdfFormat::HexTuples => {
                        let mut parser = HexTuplesStreamParser::new(base);
                        while next_line(
                            &mut input,
                            &mut workspace,
                            LineEnd::Ndjson,
                            options,
                            memory,
                            progress,
                            &mut contribution,
                        )? {
                            parser
                                .push_line(&workspace.text)
                                .map_err(|diagnostic| workspace.diagnostic(diagnostic, Some(1)))?;
                        }
                        parser
                            .finish()
                            .map_err(|diagnostic| EncodedReadError::Parse {
                                diagnostic,
                                source: None,
                            })?
                    }
                    other => {
                        let mut parser =
                            LineStreamParser::new(other, base).map_err(|diagnostic| {
                                EncodedReadError::Parse {
                                    diagnostic,
                                    source: None,
                                }
                            })?;
                        while next_line(
                            &mut input,
                            &mut workspace,
                            LineEnd::RdfEol,
                            options,
                            memory,
                            progress,
                            &mut contribution,
                        )? {
                            parser
                                .push_line(&workspace.text)
                                .map_err(|diagnostic| workspace.diagnostic(diagnostic, Some(1)))?;
                        }
                        super::super::parse::dataset_from_text_ser_graph(&parser.finish()).map_err(
                            |diagnostic| EncodedReadError::Parse {
                                diagnostic,
                                source: None,
                            },
                        )?
                    }
                }
            } else {
                while let Some(scalar) = input.next(progress, &mut contribution)? {
                    workspace.append(scalar, false, options, memory, progress)?;
                }
                super::super::parse_dataset(workspace.text.as_bytes(), media_type, base_iri)
                    .map_err(|diagnostic| workspace.diagnostic(diagnostic, None))?
            };
            progress.charge(0).map_err(EncodedReadError::Control)?;
            Ok(EncodedReadOutcome {
                dataset,
                totals: input.totals,
                profile: options.profile,
            })
        })())
    })
    .unwrap_or_else(|diagnostic| {
        Err(EncodedReadError::Parse {
            diagnostic,
            source: None,
        })
    });
    let release = workspace.release(memory).map_err(EncodedReadError::Storage);
    match result {
        Err(error) => Err(error),
        Ok(output) => release.map(|()| output),
    }
}
