// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Validated item ranges over caller-owned provisional replay storage.

use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};

use super::{
    DecodeError, DecodeErrorKind, Limits, OwnedOutput, Source, SpanOutput, Stream, Value,
    WalkError, argument, decode_item_to, next_string_chunk,
};

mod structural;
pub use structural::{StructuralCursor, StructuralError, StructuralRange};

/// Caller-owned scratch storage. The codec opens no files or services.
pub trait ReplayStorage: Read + Write + Seek {
    /// Release scratch after successful interpretation and integrity verification.
    fn finish(&mut self) -> io::Result<()>;
}

/// A buffered replay reader that preserves cached bytes on nearby seeks.
#[derive(Debug)]
pub struct ReplayBuffer<R: Read + Seek> {
    inner: BufReader<R>,
    position: u64,
}

impl<R: Read + Seek> ReplayBuffer<R> {
    /// Inspect the original caller-owned storage without moving its position
    /// or changing its bytes. This preserves the cached replay window.
    pub fn get_ref(&self) -> &R {
        self.inner.get_ref()
    }
    /// Recover the caller-owned storage, discarding only the read buffer.
    pub fn into_inner(self) -> R {
        self.inner.into_inner()
    }
    /// Start at the underlying storage's current byte position.
    pub fn new(mut inner: R) -> io::Result<Self> {
        let position = inner.stream_position()?;
        Ok(Self {
            inner: BufReader::with_capacity(32768, inner),
            position,
        })
    }
}
impl<R: Read + Seek> Read for ReplayBuffer<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.read(output)?;
        self.position = self.position.checked_add(count as u64).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "replay position exceeds u64")
        })?;
        Ok(count)
    }
}
impl<R: Read + Seek> Seek for ReplayBuffer<R> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let target = match position {
            SeekFrom::Start(target) => Some(target),
            SeekFrom::Current(delta) => self.position.checked_add_signed(delta),
            SeekFrom::End(_) => None,
        };
        if let Some(target) = target
            && let Ok(delta) = i64::try_from(i128::from(target) - i128::from(self.position))
        {
            self.inner.seek_relative(delta)?;
            let unread = self.inner.buffer().len() as u64;
            let physical = self.inner.get_mut().stream_position()?;
            self.position = physical.checked_sub(unread).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("buffered source position {physical} is below {unread} unread bytes"),
                )
            })?;
            if self.position != target {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "buffered seek to byte {target} reached byte {}",
                        self.position
                    ),
                ));
            }
        } else {
            self.position = self.inner.seek(position)?;
        }
        Ok(self.position)
    }
}

/// An immutable admitted item's byte extent in replay storage.
#[derive(Clone, Copy, Debug)]
pub struct ItemRange {
    start: u64,
    length: usize,
    limits: Limits,
}

/// A complete admitted item with its original replay identity.
#[derive(Clone, Copy, Debug)]
pub struct SpooledItem {
    item: ItemRange,
    identity: purrdf_hash::hex::Digest32,
}
impl SpooledItem {
    /// Inspect this admitted item's root range.
    pub const fn item(self) -> ItemRange {
        self.item
    }
    /// Confirm caller storage still contains exactly the admitted item bytes.
    pub fn verify<R: Read + Seek + ?Sized>(self, reader: &mut R) -> Result<(), DecodeError> {
        let mut source = RangeSource::new(reader, self.item)?;
        let mut hash = purrdf_hash::blake3::Hasher::new();
        let mut scratch = Vec::new();
        while source.pos < self.item.length {
            scratch.clear();
            let count = (self.item.length - source.pos).min(32768);
            source.take(count as u64, &mut scratch)?;
            hash.update(&scratch);
        }
        let actual = purrdf_hash::hex::Digest32::new(*hash.finalize().as_bytes());
        if actual != self.identity {
            return Err(DecodeError::new(
                DecodeErrorKind::ReplayMismatch {
                    expected: self.identity,
                    actual,
                },
                0,
            ));
        }
        Ok(())
    }
}

struct Tee<'a, R: ?Sized, W: ?Sized> {
    source: &'a mut R,
    output: &'a mut W,
    hash: purrdf_hash::blake3::Hasher,
}
impl<R: Read + ?Sized, W: Write + ?Sized> Read for Tee<'_, R, W> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let count = self.source.read(bytes)?;
        // The native Stream remains responsible for refusing an invalid count.
        if count <= bytes.len() {
            self.output.write_all(&bytes[..count])?;
            self.hash.update(&bytes[..count]);
        }
        Ok(count)
    }
}

/// Validate one complete item while copying its encoded bytes to scratch.
/// Byte strings use bounded native chunk consumption. All writes are provisional
/// until validation, trailing-data checks and decoded-source EOF succeed.
pub fn spool_item<R: Read + ?Sized, W: ReplayStorage + ?Sized>(
    reader: &mut R,
    scratch: &mut W,
    limits: Limits,
) -> Result<SpooledItem, DecodeError> {
    let start = scratch
        .stream_position()
        .map_err(|cause| io_error(cause, 0))?;
    let mut output = BufWriter::with_capacity(32768, scratch);
    let mut tee = Tee {
        source: reader,
        output: &mut output,
        hash: purrdf_hash::blake3::Hasher::new(),
    };
    let mut source = Stream {
        reader: &mut tee,
        pos: 0,
    };
    let mut ignore = |_| {};
    decode_item_to(
        &mut source,
        limits,
        false,
        &mut SpanOutput::<_, false> {
            depth: 0,
            visit: &mut ignore,
        },
    )?;
    let length = source.pos;
    match source.byte() {
        Err(error) if error.kind == DecodeErrorKind::Truncated && error.offset == length as u64 => {
        }
        Err(error) => return Err(error.into()),
        Ok(_) => return Err(DecodeError::new(DecodeErrorKind::Trailing, length)),
    }
    let identity = purrdf_hash::hex::Digest32::new(*tee.hash.finalize().as_bytes());
    output.flush().map_err(|cause| io_error(cause, length))?;
    start
        .checked_add(length as u64)
        .ok_or_else(|| DecodeError::new(DecodeErrorKind::TooLong, length))?;
    Ok(SpooledItem {
        item: ItemRange {
            start,
            length,
            limits,
        },
        identity,
    })
}

fn io_error(cause: io::Error, offset: usize) -> DecodeError {
    DecodeError::new(DecodeErrorKind::Io(cause.into()), offset)
}

struct RangeSource<'a, R: ?Sized> {
    reader: &'a mut R,
    item: ItemRange,
    pos: usize,
}
impl<'a, R: Read + Seek + ?Sized> RangeSource<'a, R> {
    fn new(reader: &'a mut R, item: ItemRange) -> Result<Self, DecodeError> {
        reader
            .seek(SeekFrom::Start(item.start))
            .map_err(|cause| io_error(cause, 0))?;
        Ok(Self {
            reader,
            item,
            pos: 0,
        })
    }
}
impl<R: Read + Seek + ?Sized> Source for RangeSource<'_, R> {
    fn offset(&self) -> u64 {
        self.pos as u64
    }
    fn consumed(&self) -> Option<&[u8]> {
        None
    }
    fn byte(&mut self) -> Result<u8, WalkError> {
        if self.pos == self.item.length {
            return Err(WalkError::new(DecodeErrorKind::Truncated, self.pos));
        }
        let mut source = Stream {
            reader: &mut *self.reader,
            pos: self.pos,
        };
        let result = source.byte();
        self.pos = source.pos;
        result
    }
    fn take(&mut self, len: u64, output: &mut Vec<u8>) -> Result<(), WalkError> {
        let mut bounded = (&mut *self.reader).take((self.item.length - self.pos) as u64);
        let mut source = Stream {
            reader: &mut bounded,
            pos: self.pos,
        };
        let result = source.take(len, output);
        self.pos = source.pos;
        result
    }
    fn skip_bytes(&mut self, len: u64) -> Result<(), WalkError> {
        let length =
            usize::try_from(len).map_err(|_| WalkError::new(DecodeErrorKind::TooLong, self.pos))?;
        let end = self
            .pos
            .checked_add(length)
            .ok_or_else(|| WalkError::new(DecodeErrorKind::TooLong, self.pos))?;
        if end > self.item.length {
            return Err(WalkError::new(DecodeErrorKind::Truncated, self.item.length));
        }
        let target = self
            .item
            .start
            .checked_add(end as u64)
            .ok_or_else(|| WalkError::new(DecodeErrorKind::TooLong, self.pos))?;
        self.reader
            .seek(SeekFrom::Start(target))
            .map_err(|cause| io_error(cause, self.pos))?;
        self.pos = end;
        Ok(())
    }
}

impl ItemRange {
    /// Encoded extent, including this item's own head.
    pub const fn encoded_len(self) -> usize {
        self.length
    }
    /// Native CBOR major type, without decoding a value tree.
    pub fn major<R: Read + Seek + ?Sized>(self, reader: &mut R) -> Result<u8, DecodeError> {
        Ok(RangeSource::new(reader, self)?.byte()? >> 5)
    }
    /// Materialize one item through the authoritative native decoder.
    pub fn decode<R: Read + Seek + ?Sized>(self, reader: &mut R) -> Result<Value, DecodeError> {
        decode_item_to(
            &mut RangeSource::new(reader, self)?,
            self.limits,
            false,
            &mut OwnedOutput,
        )
        .map_err(Into::into)
    }
    fn cursor<R: Read + Seek + ?Sized>(
        self,
        reader: &mut R,
        major: u8,
    ) -> Result<Option<RangeCursor>, DecodeError> {
        let mut source = RangeSource::new(reader, self)?;
        let first = source.byte()?;
        if first >> 5 != major {
            return Ok(None);
        }
        let length = argument(&mut source, first, 0, false)?;
        let end = self
            .length
            .checked_sub(usize::from(length.is_none()))
            .filter(|end| source.pos <= *end)
            .ok_or_else(|| DecodeError::new(DecodeErrorKind::Truncated, source.pos))?;
        Ok(Some(RangeCursor {
            item: self,
            offset: source.pos,
            end,
            remaining: if major == 4 { length } else { None },
        }))
    }
    /// Iterate an array by ranges, retaining no child collection.
    pub fn array<R: Read + Seek + ?Sized>(
        self,
        reader: &mut R,
    ) -> Result<Option<RangeCursor>, DecodeError> {
        self.cursor(reader, 4)
    }
    /// Iterate original map pairs in order, including duplicate keys.
    pub fn map<R: Read + Seek + ?Sized>(
        self,
        reader: &mut R,
    ) -> Result<Option<MapRangeCursor>, DecodeError> {
        Ok(self
            .cursor(reader, 5)?
            .map(|items| MapRangeCursor { items }))
    }
    /// Find the first matching text key, preserving native map lookup semantics.
    pub fn get<R: Read + Seek + ?Sized>(
        self,
        reader: &mut R,
        name: &str,
    ) -> Result<Option<Self>, DecodeError> {
        let Some(mut entries) = self.map(reader)? else {
            return Ok(None);
        };
        while let Some((key, value)) = entries.next(reader)? {
            if key.major(reader)? == 3 && key.decode(reader)?.as_text() == Some(name) {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }
    /// Select first occurrences of named fields in one pass over the map.
    pub fn first_fields<R: Read + Seek + ?Sized, const N: usize>(
        self,
        reader: &mut R,
        names: [&str; N],
    ) -> Result<[Option<Self>; N], DecodeError> {
        let mut selected = [None; N];
        let Some(mut entries) = self.map(reader)? else {
            return Ok(selected);
        };
        while let Some((key, value)) = entries.next(reader)? {
            if key.major(reader)? != 3 {
                continue;
            }
            let key = key.decode(reader)?;
            if let Some(index) = names.iter().position(|name| key.as_text() == Some(*name))
                && selected[index].is_none()
            {
                selected[index] = Some(value);
            }
        }
        Ok(selected)
    }
    /// Read a definite or indefinite byte string without retaining its payload.
    pub fn byte_reader<R: Read + Seek + ?Sized>(
        self,
        reader: &mut R,
    ) -> Result<Option<ByteRangeReader<'_, R>>, DecodeError> {
        let mut source = RangeSource::new(reader, self)?;
        let first = source.byte()?;
        if first >> 5 != 2 {
            return Ok(None);
        }
        let length = argument(&mut source, first, 0, false)?;
        Ok(Some(ByteRangeReader {
            source,
            remaining: length.unwrap_or(0),
            indefinite: length.is_none(),
            done: false,
            scratch: Vec::new(),
        }))
    }
}

/// An array cursor over admitted replay ranges.
#[derive(Debug)]
pub struct RangeCursor {
    item: ItemRange,
    offset: usize,
    end: usize,
    remaining: Option<u64>,
}
impl RangeCursor {
    /// Locate the next child through the native decoder's extent mode.
    pub fn next<R: Read + Seek + ?Sized>(
        &mut self,
        reader: &mut R,
    ) -> Result<Option<ItemRange>, DecodeError> {
        if self.remaining == Some(0) || self.offset == self.end {
            return Ok(None);
        }
        let extent = self
            .end
            .checked_sub(self.offset)
            .ok_or_else(|| DecodeError::new(DecodeErrorKind::Truncated, self.offset))?;
        let child = ItemRange {
            start: self.item.start + self.offset as u64,
            length: extent,
            limits: self.item.limits,
        };
        let mut source = RangeSource::new(reader, child)?;
        let mut ignore = |_| {};
        decode_item_to(
            &mut source,
            child.limits,
            false,
            &mut SpanOutput::<_, false> {
                depth: 0,
                visit: &mut ignore,
            },
        )?;
        let length = source.pos;
        self.offset += length;
        if let Some(remaining) = &mut self.remaining {
            *remaining -= 1;
        }
        Ok(Some(ItemRange { length, ..child }))
    }
}

/// Original map keys and values, retained only as two current ranges.
#[derive(Debug)]
pub struct MapRangeCursor {
    items: RangeCursor,
}
impl MapRangeCursor {
    /// Locate the next original map pair.
    pub fn next<R: Read + Seek + ?Sized>(
        &mut self,
        reader: &mut R,
    ) -> Result<Option<(ItemRange, ItemRange)>, DecodeError> {
        let Some(key) = self.items.next(reader)? else {
            return Ok(None);
        };
        let value = self.items.next(reader)?.ok_or_else(|| {
            DecodeError::new(
                DecodeErrorKind::Malformed("a replay map has no value for its key"),
                self.items.offset,
            )
        })?;
        Ok(Some((key, value)))
    }
}

/// A bounded byte-string reader using the native definite/chunk grammar.
pub struct ByteRangeReader<'a, R: Read + Seek + ?Sized> {
    source: RangeSource<'a, R>,
    remaining: u64,
    indefinite: bool,
    done: bool,
    scratch: Vec<u8>,
}
purrdf_hash::debug_non_exhaustive!(['a, R: Read + Seek + ?Sized] ByteRangeReader<'a, R> {});
impl<R: Read + Seek + ?Sized> Read for ByteRangeReader<'_, R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() || self.done {
            return Ok(0);
        }
        while self.remaining == 0 {
            if !self.indefinite {
                self.done = true;
                return Ok(0);
            }
            match next_string_chunk(&mut self.source, 2).map_err(native_io)? {
                Some((length, _)) => self.remaining = length,
                None => {
                    self.done = true;
                    return Ok(0);
                }
            }
        }
        let count = self.remaining.min(output.len() as u64).min(32768) as usize;
        self.scratch.clear();
        self.source
            .take(count as u64, &mut self.scratch)
            .map_err(native_io)?;
        output[..count].copy_from_slice(&self.scratch);
        self.remaining -= count as u64;
        Ok(count)
    }
}
fn native_io(error: impl Into<DecodeError>) -> io::Error {
    let error = error.into();
    purrdf_hash::error::io_error(io::ErrorKind::InvalidData, error)
}
