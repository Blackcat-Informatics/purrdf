// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Structural navigation is not CBOR admission. String bodies remain unvisited.

use std::fmt;
use std::io::{self, Read, Seek, SeekFrom};

use super::super::{
    Capturing, DecodeErrorKind, EncodedItem, Limits, OwnedOutput, Source, SpanOutput, Stream,
    Value, WalkError, argument, decode_item_to, display_error,
};

/// A structural-navigation failure with its absolute source byte position.
/// A storage failure retains its original native cause.
#[derive(Clone, Debug)]
pub struct StructuralError {
    kind: DecodeErrorKind,
    offset: u64,
}

impl StructuralError {
    /// Original CBOR grammar, resource or storage refusal.
    pub fn kind(&self) -> DecodeErrorKind {
        self.kind.clone()
    }
    /// Absolute byte offset in the caller's source, without native-size narrowing.
    pub const fn offset(&self) -> u64 {
        self.offset
    }
}

impl From<WalkError> for StructuralError {
    fn from(error: WalkError) -> Self {
        Self {
            kind: error.kind,
            offset: error.offset,
        }
    }
}

impl fmt::Display for StructuralError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        display_error(self.offset, &self.kind, f)
    }
}
crate::error_source!(StructuralError => {
    Self { kind: DecodeErrorKind::Io(cause), .. } => cause,
});
impl purrdf_hash::error::StructuredError for StructuralError {
    purrdf_hash::error_visit! {
        @record self { kind, offset } => "cbor-structural-range" {
            "kind" => Cause(&self.kind),
            "offset" => ByteCount(u128::from(self.offset)),
        }
    }
}

fn seek<R: Seek + ?Sized>(reader: &mut R, position: u64) -> Result<(), WalkError> {
    let actual = reader
        .seek(SeekFrom::Start(position))
        .map_err(|cause| WalkError::new(DecodeErrorKind::Io(cause.into()), position))?;
    if actual != position {
        return Err(WalkError::new(
            DecodeErrorKind::Io(
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("seek to byte {position} returned byte {actual}"),
                )
                .into(),
            ),
            position,
        ));
    }
    Ok(())
}

struct StructuralSource<'a, R: ?Sized> {
    reader: &'a mut R,
    position: u64,
    end: u64,
}

impl<'a, R: Read + Seek + ?Sized> StructuralSource<'a, R> {
    fn new(reader: &'a mut R, position: u64, end: u64) -> Result<Self, WalkError> {
        if position > end {
            return Err(WalkError::new(
                DecodeErrorKind::Malformed(
                    "a structural range starts beyond its pinned source extent",
                ),
                position,
            ));
        }
        seek(reader, position)?;
        Ok(Self {
            reader,
            position,
            end,
        })
    }
}

impl<R: Read + Seek + ?Sized> Source for StructuralSource<'_, R> {
    fn offset(&self) -> u64 {
        self.position
    }
    fn consumed(&self) -> Option<&[u8]> {
        None
    }
    fn byte(&mut self) -> Result<u8, WalkError> {
        if self.position == self.end {
            return Err(WalkError::new(DecodeErrorKind::Truncated, self.position));
        }
        let mut source = Stream {
            reader: &mut *self.reader,
            pos: 0,
        };
        let result = source.byte();
        let start = self.position;
        self.position += source.pos as u64;
        result.map_err(|mut error| {
            error.offset += start;
            error
        })
    }
    fn take(&mut self, length: u64, output: &mut Vec<u8>) -> Result<(), WalkError> {
        if usize::try_from(length).is_err() {
            return Err(WalkError::new(DecodeErrorKind::TooLong, self.position));
        }
        if length > self.end - self.position {
            return Err(WalkError::new(DecodeErrorKind::Truncated, self.end));
        }
        let mut bounded = (&mut *self.reader).take(length);
        let mut source = Stream {
            reader: &mut bounded,
            pos: 0,
        };
        let result = source.take(length, output);
        let start = self.position;
        self.position += source.pos as u64;
        result.map_err(|mut error| {
            error.offset += start;
            error
        })
    }
    fn skip_bytes(&mut self, length: u64) -> Result<(), WalkError> {
        let end = self
            .position
            .checked_add(length)
            .filter(|end| *end <= self.end)
            .ok_or_else(|| WalkError::new(DecodeErrorKind::Truncated, self.end))?;
        seek(self.reader, end)?;
        self.position = end;
        Ok(())
    }
}

/// A structurally located encoded item. String bodies have not been validated.
/// This type cannot produce an admitted ItemRange or BorrowedItem.
///
/// Every operation must use the same unchanged caller-owned source that was
/// scanned. The supplied source length must be its actual pinned length. An
/// extent carries neither reader identity nor an integrity certificate; callers
/// must preserve that source contract across scans, cursor reads and decoding.
#[derive(Clone, Copy, Debug)]
pub struct StructuralRange {
    start: u64,
    end: u64,
    limits: Limits,
}

impl StructuralRange {
    /// Locate one item's extent within the pinned source length. Only its CBOR
    /// structure is inspected: byte and text bodies are skipped by checked seeks.
    /// An item starting at the source end is clean EOF; an incomplete item fails.
    pub fn scan<R: Read + Seek + ?Sized>(
        reader: &mut R,
        start: u64,
        source_length: u64,
        limits: Limits,
    ) -> Result<Option<Self>, StructuralError> {
        let mut source = StructuralSource::new(reader, start, source_length)?;
        if start == source_length {
            return Ok(None);
        }
        let mut ignore = |_| {};
        decode_item_to(
            &mut source,
            limits,
            false,
            &mut SpanOutput::<_, true, true> {
                depth: 0,
                visit: &mut ignore,
            },
        )?;
        Ok(Some(Self {
            start,
            end: source.position,
            limits,
        }))
    }
    /// Absolute first encoded byte.
    pub const fn start(self) -> u64 {
        self.start
    }
    /// Absolute byte immediately after the item.
    pub const fn end(self) -> u64 {
        self.end
    }
    /// Encoded extent, including the item head.
    pub const fn encoded_len(self) -> u64 {
        self.end - self.start
    }
    /// Inspect the root major type without admitting any payload bytes.
    pub fn major<R: Read + Seek + ?Sized>(self, reader: &mut R) -> Result<u8, StructuralError> {
        Ok(StructuralSource::new(reader, self.start, self.end)?.byte()? >> 5)
    }
    /// Inspect a tag and its structural child without admitting that child's
    /// payload. The child keeps its original absolute source extent.
    pub fn tag<R: Read + Seek + ?Sized>(
        self,
        reader: &mut R,
    ) -> Result<Option<(u64, Self)>, StructuralError> {
        let (tag, start) = {
            let mut source = StructuralSource::new(reader, self.start, self.end)?;
            let first = source.byte()?;
            if first >> 5 != 6 {
                return Ok(None);
            }
            let tag = argument(&mut source, first, self.start, false)?.ok_or_else(|| {
                WalkError::new(
                    DecodeErrorKind::Malformed("a tag argument must be definite"),
                    self.start,
                )
            })?;
            (tag, source.position)
        };
        let child = Self::scan(reader, start, self.end, self.limits)?.ok_or_else(|| {
            StructuralError::from(WalkError::new(DecodeErrorKind::Truncated, start))
        })?;
        if child.end != self.end {
            return Err(WalkError::new(DecodeErrorKind::Trailing, child.end).into());
        }
        Ok(Some((tag, child)))
    }

    /// Materialize and validate precisely this selected item through the original
    /// native decoder. Its local materialization must fit the address space.
    pub fn decode<R: Read + Seek + ?Sized>(self, reader: &mut R) -> Result<Value, StructuralError> {
        if usize::try_from(self.encoded_len()).is_err() {
            return Err(WalkError::new(DecodeErrorKind::TooLong, self.start).into());
        }
        let mut source = StructuralSource::new(reader, self.start, self.end)?;
        let value = decode_item_to(&mut source, self.limits, false, &mut OwnedOutput)?;
        if source.position != self.end {
            return Err(WalkError::new(DecodeErrorKind::Trailing, source.position).into());
        }
        Ok(value)
    }
    /// Validate the selected item through the original decoder while retaining
    /// its encoded bytes instead of an owned value tree. Text is validated;
    /// this grants admitted byte access only after the entire range succeeds.
    /// One primitive string can coexist with the captured item during admission.
    /// # Errors
    /// Returns original grammar/storage failures at absolute source positions
    /// or a local address-space refusal before retaining the item.
    pub fn encoded<R: Read + Seek + ?Sized>(
        self,
        reader: &mut R,
    ) -> Result<EncodedItem, StructuralError> {
        if usize::try_from(self.encoded_len()).is_err() {
            return Err(WalkError::new(DecodeErrorKind::TooLong, self.start).into());
        }
        let mut source = Capturing {
            source: StructuralSource::new(reader, self.start, self.end)?,
            encoded: Vec::new(),
        };
        let mut ignore = |_| {};
        decode_item_to(
            &mut source,
            self.limits,
            false,
            &mut SpanOutput::<_, false> {
                depth: 0,
                visit: &mut ignore,
            },
        )?;
        if source.offset() != self.end {
            return Err(WalkError::new(DecodeErrorKind::Trailing, source.offset()).into());
        }
        Ok(EncodedItem {
            encoded: source.encoded,
            limits: self.limits,
        })
    }
    fn children<R: Read + Seek + ?Sized>(
        self,
        reader: &mut R,
        major: u8,
    ) -> Result<Option<StructuralCursor>, StructuralError> {
        let mut source = StructuralSource::new(reader, self.start, self.end)?;
        let first = source.byte()?;
        if first >> 5 != major {
            return Ok(None);
        }
        let remaining = argument(&mut source, first, self.start, false)?;
        let remaining = if major == 5 {
            remaining
                .map(|n| {
                    n.checked_mul(2).ok_or_else(|| {
                        WalkError::new(
                            DecodeErrorKind::Malformed("a structural map item count exceeds u64"),
                            self.start,
                        )
                    })
                })
                .transpose()?
        } else {
            remaining
        };
        Ok(Some(StructuralCursor {
            range: self,
            position: source.position,
            remaining,
            done: false,
        }))
    }
    /// Iterate structural array children in source order without retaining them.
    pub fn array<R: Read + Seek + ?Sized>(
        self,
        reader: &mut R,
    ) -> Result<Option<StructuralCursor>, StructuralError> {
        self.children(reader, 4)
    }
    /// Iterate alternating map keys and values in original order. Duplicate keys
    /// retain their positions. Each returned item remains structural-only.
    pub fn map<R: Read + Seek + ?Sized>(
        self,
        reader: &mut R,
    ) -> Result<Option<StructuralCursor>, StructuralError> {
        self.children(reader, 5)
    }
}

/// Forward children of one structural array or map, with wide absolute extents.
#[derive(Debug)]
pub struct StructuralCursor {
    range: StructuralRange,
    position: u64,
    remaining: Option<u64>,
    done: bool,
}

impl StructuralCursor {
    /// Locate the next structural child. A malformed or incomplete suffix fails;
    /// an indefinite container must end in its own break byte.
    pub fn next<R: Read + Seek + ?Sized>(
        &mut self,
        reader: &mut R,
    ) -> Result<Option<StructuralRange>, StructuralError> {
        if self.done {
            return Ok(None);
        }
        if self.remaining == Some(0) {
            if self.position != self.range.end {
                return Err(WalkError::new(DecodeErrorKind::Trailing, self.position).into());
            }
            self.done = true;
            return Ok(None);
        }
        let mut source = StructuralSource::new(reader, self.position, self.range.end)?;
        if self.remaining.is_none() && source.byte()? == 0xff {
            if source.position != self.range.end {
                return Err(WalkError::new(DecodeErrorKind::Trailing, source.position).into());
            }
            self.done = true;
            return Ok(None);
        }
        let child =
            StructuralRange::scan(reader, self.position, self.range.end, self.range.limits)?
                .ok_or_else(|| {
                    StructuralError::from(WalkError::new(DecodeErrorKind::Truncated, self.position))
                })?;
        self.position = child.end;
        if let Some(remaining) = &mut self.remaining {
            *remaining -= 1;
        }
        Ok(Some(child))
    }
}

#[cfg(test)]
mod tests;
