// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit, strict GB 18030 editions with incremental source contributions.
//!
//! No encoding detection, replacement, normalization or web/GBK aliases occur.
//! The codec owns only a four-byte incomplete scalar and counters; output and
//! any retained source map belong to the caller. See `data/gb18030/PROVENANCE.md`.

use crate::control::Progress;

mod error;
mod tables;
pub use error::{Error, ErrorKind, StreamError};

/// A complete, explicitly selected mapping edition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Profile {
    /// GB 18030-2022, including Amendment 1, with reciprocal Annex D.1 mappings.
    Gb18030_2022Amendment1,
    /// GB 18030-2005 mapping, preserving its separate legacy character meanings.
    Gb18030_2005,
}

impl Profile {
    /// Stable, explicit mapping identity for receipts and diagnostics.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Gb18030_2022Amendment1 => "GB18030-2022+A1",
            Self::Gb18030_2005 => "GB18030-2005",
        }
    }
    fn overrides(self, direction: Direction) -> &'static [(u32, u32)] {
        match (self, direction) {
            (Self::Gb18030_2022Amendment1, Direction::Decode) => tables::CURRENT_DECODE,
            (Self::Gb18030_2022Amendment1, Direction::Encode) => tables::CURRENT_ENCODE,
            (Self::Gb18030_2005, Direction::Decode) => tables::LEGACY_DECODE,
            (Self::Gb18030_2005, Direction::Encode) => tables::LEGACY_ENCODE,
        }
    }
}

/// Whether source bytes are GB 18030 or UTF-8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Strict GB 18030 bytes to UTF-8.
    Decode,
    /// Strict UTF-8 bytes to GB 18030.
    Encode,
}

/// Half-open original byte coordinates, independent of chunk boundaries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Extent {
    /// Inclusive start.
    pub start: u64,
    /// Exclusive end.
    pub end: u64,
}

/// All output bytes for one scalar have its whole original source contribution.
///
/// There are no one-to-many mappings in these profiles. Interior output bytes
/// have no fabricated source boundary: use the complete `source` extent. The
/// boundary after this emission maps exactly to `source.end`.
#[derive(Debug, Clone, Copy)]
pub struct Emission<'a> {
    /// Original decoded scalar, without normalization.
    pub scalar: char,
    /// Exact source bytes contributing this scalar.
    pub source: Extent,
    /// Exact output byte extent of `bytes`.
    pub output: Extent,
    /// UTF-8 or GB 18030 bytes, as selected by direction.
    pub bytes: &'a [u8],
}

/// Inclusive bounds checked before source consumption and output emission.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Total original source bytes.
    pub input_bytes: u64,
    /// Total encoded or decoded output bytes.
    pub output_bytes: u64,
}

/// Cumulative accepted work, including an unfinished source scalar's bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Totals {
    /// Original bytes consumed.
    pub input_bytes: u64,
    /// Bytes whose emission callback succeeded.
    pub output_bytes: u64,
    /// Scalars whose emission callback succeeded.
    pub scalars: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Open,
    Finished,
    Failed,
}

/// Constant-workspace streaming conversion; each chunk may split any scalar.
///
/// A failed stream is poisoned. Output accepted before a failure remains the
/// caller's prefix, and neither another chunk nor finalization can report a
/// complete success. The codec allocates nothing: the sink admits its output
/// and mapping storage and returns its original error through `StreamError`.
#[derive(Debug)]
pub struct Transcoder {
    profile: Profile,
    direction: Direction,
    limits: Limits,
    pending: [u8; 4],
    length: u8,
    totals: Totals,
    state: State,
}

impl Transcoder {
    /// Select an edition and direction explicitly; no default encoding exists.
    pub const fn new(profile: Profile, direction: Direction, limits: Limits) -> Self {
        Self {
            profile,
            direction,
            limits,
            pending: [0; 4],
            length: 0,
            totals: Totals {
                input_bytes: 0,
                output_bytes: 0,
                scalars: 0,
            },
            state: State::Open,
        }
    }

    fn error(&self, kind: ErrorKind, utf8: Option<std::str::Utf8Error>) -> Error {
        Error {
            profile: self.profile,
            direction: self.direction,
            source: Extent {
                start: self.totals.input_bytes - u64::from(self.length),
                end: self.totals.input_bytes,
            },
            bytes: self.pending,
            length: self.length,
            kind,
            utf8,
        }
    }

    /// Consume a chunk, or consume the final chunk and close the stream.
    ///
    /// Each source byte is charged before consumption. A zero-unit poll precedes
    /// the successful chunk result, so empty/final calls observe cancellation.
    /// Emissions accepted before that poll remain the caller's prefix. A callback
    /// receives one complete scalar, never a chunk-dependent partial character.
    ///
    /// # Errors
    /// Returns exact malformed/incomplete source and profile, source/output
    /// ceilings or offset overflow, or the unchanged progress/output failure.
    /// A failure poisons the stream before returning its original cause.
    pub fn push<P: Progress + ?Sized, O>(
        &mut self,
        input: &[u8],
        final_input: bool,
        progress: &mut P,
        mut emit: impl FnMut(Emission<'_>) -> Result<(), O>,
    ) -> Result<Totals, StreamError<P::Error, O>> {
        if self.state != State::Open {
            return Err(StreamError::Native(self.error(ErrorKind::Closed, None)));
        }
        let result = self.drive(input, final_input, progress, &mut emit);
        if result.is_err() {
            self.state = State::Failed;
        }
        result
    }

    fn drive<P: Progress + ?Sized, O>(
        &mut self,
        input: &[u8],
        final_input: bool,
        progress: &mut P,
        emit: &mut impl FnMut(Emission<'_>) -> Result<(), O>,
    ) -> Result<Totals, StreamError<P::Error, O>> {
        for &byte in input {
            progress.charge(1).map_err(StreamError::Control)?;
            let next =
                self.totals.input_bytes.checked_add(1).ok_or_else(|| {
                    StreamError::Native(self.error(ErrorKind::OffsetOverflow, None))
                })?;
            if next > self.limits.input_bytes {
                return Err(StreamError::Native(self.error(
                    ErrorKind::InputLimit {
                        limit: self.limits.input_bytes,
                    },
                    None,
                )));
            }
            self.pending[usize::from(self.length)] = byte;
            self.length += 1;
            self.totals.input_bytes = next;
            let scalar = self.scalar().map_err(StreamError::Native)?;
            let Some(scalar) = scalar else {
                continue;
            };
            let mut output = [0u8; 4];
            let width = match self.direction {
                Direction::Decode => scalar.encode_utf8(&mut output).len(),
                Direction::Encode => encode_scalar(self.profile, scalar, &mut output),
            };
            let end = self
                .totals
                .output_bytes
                .checked_add(width as u64)
                .ok_or_else(|| StreamError::Native(self.error(ErrorKind::OffsetOverflow, None)))?;
            if end > self.limits.output_bytes {
                return Err(StreamError::Native(self.error(
                    ErrorKind::OutputLimit {
                        limit: self.limits.output_bytes,
                    },
                    None,
                )));
            }
            emit(Emission {
                scalar,
                source: Extent {
                    start: next - u64::from(self.length),
                    end: next,
                },
                output: Extent {
                    start: self.totals.output_bytes,
                    end,
                },
                bytes: &output[..width],
            })
            .map_err(StreamError::Output)?;
            self.totals.output_bytes = end;
            self.totals.scalars += 1;
            self.length = 0;
            self.pending = [0; 4];
        }
        if final_input && self.length != 0 {
            let utf8 = (self.direction == Direction::Encode)
                .then(|| std::str::from_utf8(&self.pending[..usize::from(self.length)]).err())
                .flatten();
            return Err(StreamError::Native(self.error(ErrorKind::Incomplete, utf8)));
        }
        progress.charge(0).map_err(StreamError::Control)?;
        if final_input {
            self.state = State::Finished;
        }
        Ok(self.totals)
    }

    fn scalar(&self) -> Result<Option<char>, Error> {
        if self.direction == Direction::Encode {
            return match std::str::from_utf8(&self.pending[..usize::from(self.length)]) {
                Ok(text) => Ok(text.chars().next()),
                Err(error) if error.error_len().is_none() => Ok(None),
                Err(error) => Err(self.error(ErrorKind::Malformed, Some(error))),
            };
        }
        let [a, b, c, d] = self.pending;
        let code = match self.length {
            1 if a < 0x80 => return Ok(Some(char::from(a))),
            1 if (0x81..=0xfe).contains(&a) => return Ok(None),
            2 if (0x40..=0xfe).contains(&b) && b != 0x7f => u32::from(u16::from_be_bytes([a, b])),
            2 if b.is_ascii_digit() => return Ok(None),
            3 if (0x81..=0xfe).contains(&c) => return Ok(None),
            4 if d.is_ascii_digit() => u32::from_be_bytes([a, b, c, d]),
            _ => return Err(self.error(ErrorKind::Malformed, None)),
        };
        decode_scalar(self.profile, code)
            .and_then(char::from_u32)
            .map(Some)
            .ok_or_else(|| self.error(ErrorKind::Unassigned, None))
    }
}

fn lookup(rows: &[(u32, u32)], value: u32) -> Option<u32> {
    rows.binary_search_by_key(&value, |row| row.0)
        .ok()
        .map(|index| rows[index].1)
}

fn decode_scalar(profile: Profile, code: u32) -> Option<u32> {
    if let Some(value) = lookup(profile.overrides(Direction::Decode), code) {
        return Some(value);
    }
    if code <= 0xffff {
        let [a, b] = (code as u16).to_be_bytes();
        let index =
            usize::from(a - 0x81) * 190 + usize::from(b - if b < 0x7f { 0x40 } else { 0x41 });
        return Some(u32::from(tables::TWO[index]));
    }
    let [a, b, c, d] = code.to_be_bytes();
    let pointer = (((u32::from(a) - 0x81) * 10 + u32::from(b) - 0x30) * 126 + u32::from(c) - 0x81)
        * 10
        + u32::from(d)
        - 0x30;
    match pointer {
        0..39420 => {
            let index = tables::FOUR.partition_point(|row| row.0 <= pointer) - 1;
            Some(tables::FOUR[index].1 + pointer - tables::FOUR[index].0)
        }
        189_000..=1_237_575 => Some(pointer - 189_000 + 0x1_0000),
        _ => None,
    }
}

fn encode_scalar(profile: Profile, scalar: char, output: &mut [u8; 4]) -> usize {
    let value = u32::from(scalar);
    if value < 0x80 {
        output[0] = value as u8;
        return 1;
    }
    if let Some(code) = lookup(profile.overrides(Direction::Encode), value) {
        if code <= 0xffff {
            output[..2].copy_from_slice(&(code as u16).to_be_bytes());
            return 2;
        }
        *output = code.to_be_bytes();
        return 4;
    }
    if value <= 0xffff
        && let Ok(index) = tables::TWO_INVERSE
            .binary_search_by_key(&(value as u16), |&index| tables::TWO[usize::from(index)])
    {
        let pair = usize::from(tables::TWO_INVERSE[index]);
        output[0] = (pair / 190 + 0x81) as u8;
        let last = pair % 190;
        output[1] = (last + if last < 63 { 0x40 } else { 0x41 }) as u8;
        return 2;
    }
    let pointer = if value >= 0x1_0000 {
        value - 0x1_0000 + 189_000
    } else {
        // Every non-ASCII BMP scalar is either in TWO or the frozen complement.
        // The generated first range starts at U+0080, so a predecessor exists.
        let index = tables::FOUR.partition_point(|row| row.1 <= value) - 1;
        tables::FOUR[index].0 + value - tables::FOUR[index].1
    };
    *output = [
        (pointer / 12600 + 0x81) as u8,
        (pointer / 1260 % 10 + 0x30) as u8,
        (pointer / 10 % 126 + 0x81) as u8,
        (pointer % 10 + 0x30) as u8,
    ];
    4
}
