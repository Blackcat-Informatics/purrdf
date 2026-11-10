// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The CBOR decoders: well-formed (RFC 8949 §4.1, with Appendix C's rules)
//! and core deterministic (§4.2.1), over an explicit stack of open items.

use core::fmt;
use std::io::{self, Read};

use super::{Integer, Value};

mod replay;
pub use replay::{
    ByteRangeReader, ItemRange, MapRangeCursor, RangeCursor, ReplayBuffer, ReplayStorage,
    SpooledItem, StructuralCursor, StructuralError, StructuralRange, spool_item,
};

/// The resource bounds of one decode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// The most arrays, maps and tags open at once.
    pub max_depth: usize,
}

impl Limits {
    /// 256 open arrays, maps and tags.
    pub const DEFAULT: Self = Self { max_depth: 256 };
}

impl Default for Limits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// What is wrong with a CBOR item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeErrorKind {
    /// The input ends inside the item.
    Truncated,
    /// The item is not well-formed (RFC 8949 §3, Appendix F); the text names
    /// the rule.
    Malformed(&'static str),
    /// A text string is not UTF-8.
    InvalidUtf8,
    /// More arrays, maps and tags are open at once than
    /// [`Limits::max_depth`] allows.
    Depth {
        /// The cap that was exceeded.
        limit: usize,
    },
    /// A length does not fit this platform's address space.
    TooLong,
    /// The item is well-formed but not in core deterministic encoding (RFC
    /// 8949 §4.2.1); the text names the rule.
    NotDeterministic(&'static str),
    /// Bytes follow the item where the whole input was to be one item.
    Trailing,
    /// Caller-owned replay bytes differ from the admitted item.
    ReplayMismatch {
        /// Original admitted encoded-byte identity.
        expected: purrdf_hash::hex::Digest32,
        /// Actual replayed encoded-byte identity.
        actual: purrdf_hash::hex::Digest32,
    },
    /// The reader failed.
    Io(purrdf_hash::error::IoFailure),
    /// A reader violated its buffer-length contract.
    ReadCount {
        /// Number of bytes the reader claimed to write.
        actual: usize,
        /// Number of bytes supplied for writing.
        maximum: usize,
    },
    /// Consumed bytes exceed the decoder's address space.
    OffsetOverflow {
        /// Position before the read.
        position: usize,
        /// Bytes consumed by this read.
        additional: usize,
    },
}

/// A refused CBOR item: what is wrong, and the byte offset where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodeError {
    kind: DecodeErrorKind,
    offset: usize,
}

impl DecodeError {
    const fn new(kind: DecodeErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    /// What is wrong.
    pub fn kind(&self) -> DecodeErrorKind {
        self.kind.clone()
    }

    /// The byte offset of the defect, from the start of the input.
    pub const fn offset(&self) -> usize {
        self.offset
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        display_error(self.offset, &self.kind, f)
    }
}

fn display_error(
    offset: impl fmt::Display,
    kind: &DecodeErrorKind,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    write!(f, "CBOR byte {offset}: ")?;
    match kind {
        DecodeErrorKind::Truncated => f.write_str("the input ends inside an item"),
        DecodeErrorKind::Malformed(rule) => write!(f, "not well-formed: {rule}"),
        DecodeErrorKind::InvalidUtf8 => f.write_str("a text string is not UTF-8"),
        DecodeErrorKind::Depth { limit } => {
            write!(f, "more than {limit} nested arrays, maps and tags")
        }
        DecodeErrorKind::TooLong => f.write_str("a length exceeds the address space"),
        DecodeErrorKind::NotDeterministic(rule) => {
            write!(f, "not deterministically encoded: {rule}")
        }
        DecodeErrorKind::Trailing => f.write_str("bytes follow the item"),
        DecodeErrorKind::ReplayMismatch { .. } => {
            f.write_str("replay storage differs from the admitted CBOR item")
        }
        DecodeErrorKind::Io(cause) => write!(f, "read failed: {}", cause.kind()),
        DecodeErrorKind::ReadCount { actual, maximum } => write!(
            f,
            "reader returned {actual} bytes for a {maximum}-byte buffer"
        ),
        DecodeErrorKind::OffsetOverflow {
            position,
            additional,
        } => write!(
            f,
            "read of {additional} bytes at {position} exceeds the address space"
        ),
    }
}

crate::error_source!(DecodeError => {
    Self { kind: DecodeErrorKind::Io(cause), .. } => cause,
});

impl purrdf_hash::error::StructuredError for DecodeErrorKind {
    purrdf_hash::error_visit! {
        Truncated => "cbor-truncated" {},
        Malformed(rule) => "cbor-malformed" { "rule" => Text(rule) },
        InvalidUtf8 => "cbor-invalid-utf8" {},
        Depth { limit } => "cbor-depth-limit" { "limit" => Unsigned(*limit as u128) },
        TooLong => "cbor-length-overflow" {},
        NotDeterministic(rule) => "cbor-not-deterministic" { "rule" => Text(rule) },
        Trailing => "cbor-trailing-data" {},
        ReplayMismatch { expected, actual } => "cbor-replay-mismatch" {
            "expected" => Record(expected), "actual" => Record(actual),
        },
        Io(cause) => "cbor-io" { "cause" => Cause(cause) },
        ReadCount { actual, maximum } => "cbor-reader-count" { "actual" => Unsigned(*actual as u128), "maximum" => Unsigned(*maximum as u128) },
        OffsetOverflow { position, additional } => "cbor-offset-overflow" { "position" => Unsigned(*position as u128), "additional" => Unsigned(*additional as u128) },
    }
}

impl purrdf_hash::error::StructuredError for DecodeError {
    purrdf_hash::error_visit! {
        @record self { kind, offset } => "cbor-document" {
            "kind" => Cause(&self.kind),
            "offset" => ByteCount(self.offset as u128),
        }
    }
}

/// Private wide walker failure. Legacy sources keep checked native positions;
/// structural seek sources retain their original absolute byte positions.
struct WalkError {
    kind: DecodeErrorKind,
    offset: u64,
}

impl WalkError {
    fn new(kind: DecodeErrorKind, offset: impl TryInto<u64>) -> Self {
        Self {
            kind,
            offset: offset.try_into().ok().expect("a decoder offset fits u64"),
        }
    }
}

impl From<DecodeError> for WalkError {
    fn from(error: DecodeError) -> Self {
        Self::new(error.kind, error.offset)
    }
}

impl From<WalkError> for DecodeError {
    fn from(error: WalkError) -> Self {
        // Slice, Stream and admitted replay keep their checked usize cursor.
        // Structural sources use their separate wide error boundary instead.
        Self::new(error.kind, native_offset(error.offset))
    }
}

fn native_offset(offset: u64) -> usize {
    usize::try_from(offset).expect("a native decoder source keeps a checked usize extent")
}

fn native_span((start, end): WalkSpan) -> ItemSpan {
    (native_offset(start), native_offset(end))
}

/// Where the decoder's bytes come from.
trait Source {
    /// The offset of the next byte.
    fn offset(&self) -> u64;
    /// The next byte.
    fn byte(&mut self) -> Result<u8, WalkError>;
    /// Append the next `len` bytes to `out`.
    fn take(&mut self, len: u64, out: &mut Vec<u8>) -> Result<(), WalkError>;
    /// Consume uninterpreted bytes without retaining a complete string.
    fn skip_bytes(&mut self, mut len: u64) -> Result<(), WalkError> {
        let mut scratch = Vec::new();
        while len != 0 {
            scratch.clear();
            let count = len.min(32768);
            self.take(count, &mut scratch)?;
            len -= count;
        }
        Ok(())
    }
    /// The bytes already read, for the deterministic key-order check.
    fn consumed(&self) -> Option<&[u8]>;
}

struct Slice<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Slice<'_> {
    fn extent(&self, len: u64) -> Result<core::ops::Range<usize>, WalkError> {
        let available = self.bytes.len() - self.pos;
        let len = usize::try_from(len)
            .ok()
            .filter(|&len| len <= available)
            .ok_or_else(|| WalkError::new(DecodeErrorKind::Truncated, self.bytes.len()))?;
        Ok(self.pos..self.pos + len)
    }
}

impl Source for Slice<'_> {
    fn skip_bytes(&mut self, len: u64) -> Result<(), WalkError> {
        self.pos = self.extent(len)?.end;
        Ok(())
    }
    fn offset(&self) -> u64 {
        self.pos as u64
    }

    fn byte(&mut self) -> Result<u8, WalkError> {
        let byte = *self
            .bytes
            .get(self.pos)
            .ok_or_else(|| WalkError::new(DecodeErrorKind::Truncated, self.pos))?;
        self.pos += 1;
        Ok(byte)
    }

    fn take(&mut self, len: u64, out: &mut Vec<u8>) -> Result<(), WalkError> {
        let extent = self.extent(len)?;
        out.extend_from_slice(&self.bytes[extent.clone()]);
        self.pos = extent.end;
        Ok(())
    }

    fn consumed(&self) -> Option<&[u8]> {
        Some(&self.bytes[..self.pos])
    }
}

struct Stream<'r, R: ?Sized> {
    reader: &'r mut R,
    pos: usize,
}

impl<R: Read + ?Sized> Source for Stream<'_, R> {
    fn offset(&self) -> u64 {
        self.pos as u64
    }

    fn byte(&mut self) -> Result<u8, WalkError> {
        let mut byte = [0_u8];
        loop {
            match self.reader.read(&mut byte) {
                Ok(1) => break,
                Ok(0) => return Err(WalkError::new(DecodeErrorKind::Truncated, self.pos)),
                Ok(actual) => {
                    return Err(WalkError::new(
                        DecodeErrorKind::ReadCount { actual, maximum: 1 },
                        self.pos,
                    ));
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => {
                    return Err(WalkError::new(DecodeErrorKind::Io(error.into()), self.pos));
                }
            }
        }
        self.pos = self.pos.checked_add(1).ok_or_else(|| {
            WalkError::new(
                DecodeErrorKind::OffsetOverflow {
                    position: self.pos,
                    additional: 1,
                },
                self.pos,
            )
        })?;
        Ok(byte[0])
    }

    fn take(&mut self, len: u64, out: &mut Vec<u8>) -> Result<(), WalkError> {
        // `take` + `read_to_end` grows the buffer as bytes arrive, so a
        // declared length the stream does not back allocates nothing extra.
        let before = out.len();
        let read = (&mut *self.reader)
            .take(len)
            .read_to_end(out)
            .map_err(|error| WalkError::new(DecodeErrorKind::Io(error.into()), self.pos))?;
        self.pos = self.pos.checked_add(read).ok_or_else(|| {
            WalkError::new(
                DecodeErrorKind::OffsetOverflow {
                    position: self.pos,
                    additional: read,
                },
                self.pos,
            )
        })?;
        if (out.len() - before) as u64 == len {
            Ok(())
        } else {
            Err(WalkError::new(DecodeErrorKind::Truncated, self.pos))
        }
    }

    fn consumed(&self) -> Option<&[u8]> {
        None
    }
}

/// An array, map or tag whose items are still being read.
/// One complete encoded CBOR item, relative to the current input slice.
type ItemSpan = (usize, usize);
type WalkSpan = (u64, u64);

trait DecodeOutput {
    const WIDE: bool = false;
    type Item;
    type Array: Default;
    type Map: Default;

    fn scalar(&mut self, value: Value, span: WalkSpan, depth: usize) -> Self::Item;
    fn push_array(&mut self, array: &mut Self::Array, item: Self::Item);
    fn push_map(&mut self, map: &mut Self::Map, key: Self::Item, value: Self::Item);
    fn array(&mut self, array: Self::Array, span: WalkSpan, depth: usize) -> Self::Item;
    fn map(&mut self, map: Self::Map, span: WalkSpan, depth: usize) -> Self::Item;
    fn tag(&mut self, tag: u64, item: Self::Item, span: WalkSpan, depth: usize) -> Self::Item;
}

struct OwnedOutput;

impl DecodeOutput for OwnedOutput {
    type Item = Value;
    type Array = Vec<Value>;
    type Map = Vec<(Value, Value)>;

    fn scalar(&mut self, value: Value, _: WalkSpan, _: usize) -> Value {
        value
    }
    fn push_array(&mut self, array: &mut Self::Array, item: Value) {
        array.push(item);
    }
    fn push_map(&mut self, map: &mut Self::Map, key: Value, value: Value) {
        map.push((key, value));
    }
    fn array(&mut self, array: Self::Array, _: WalkSpan, _: usize) -> Value {
        Value::Array(array)
    }
    fn map(&mut self, map: Self::Map, _: WalkSpan, _: usize) -> Value {
        Value::Map(map)
    }
    fn tag(&mut self, tag: u64, item: Value, _: WalkSpan, _: usize) -> Value {
        Value::Tag(tag, Box::new(item))
    }
}

struct SpanOutput<'a, F, const TRUSTED: bool, const WIDE: bool = false> {
    depth: usize,
    visit: &'a mut F,
}

impl<F: FnMut(WalkSpan), const TRUSTED: bool, const WIDE: bool> SpanOutput<'_, F, TRUSTED, WIDE> {
    fn record(&mut self, span: WalkSpan, depth: usize) {
        if depth == self.depth {
            (self.visit)(span);
        }
    }
}

impl<F: FnMut(WalkSpan), const TRUSTED: bool, const WIDE: bool> DecodeOutput
    for SpanOutput<'_, F, TRUSTED, WIDE>
{
    const WIDE: bool = WIDE;
    type Item = ();
    type Array = ();
    type Map = ();

    fn scalar(&mut self, _: Value, span: WalkSpan, depth: usize) {
        self.record(span, depth);
    }
    fn push_array(&mut self, (): &mut (), (): ()) {}
    fn push_map(&mut self, (): &mut (), (): (), (): ()) {}
    fn array(&mut self, (): (), span: WalkSpan, depth: usize) {
        self.record(span, depth);
    }
    fn map(&mut self, (): (), span: WalkSpan, depth: usize) {
        self.record(span, depth);
    }
    fn tag(&mut self, _: u64, (): (), span: WalkSpan, depth: usize) {
        self.record(span, depth);
    }
}

/// String output separates validating admission from trusted or explicitly
/// structural extent navigation. Only the structural policy permits wide extents.
trait DecodeString<S: Source>: DecodeOutput {
    fn string(
        &mut self,
        source: &mut S,
        major: u8,
        length: Option<u64>,
        deterministic: bool,
        at: u64,
        depth: usize,
    ) -> Result<Self::Item, WalkError> {
        string_value(self, source, major, length, deterministic, at, depth)
    }
}

fn string_value<S: Source, B: DecodeOutput + ?Sized>(
    output: &mut B,
    source: &mut S,
    major: u8,
    length: Option<u64>,
    deterministic: bool,
    at: u64,
    depth: usize,
) -> Result<B::Item, WalkError> {
    let bytes = string_bytes(source, major, length, deterministic, at)?;
    let value = if major == 2 {
        Value::Bytes(bytes)
    } else {
        Value::Text(
            String::from_utf8(bytes)
                .map_err(|_| WalkError::new(DecodeErrorKind::InvalidUtf8, at))?,
        )
    };
    Ok(output.scalar(value, (at, source.offset()), depth))
}

impl<S: Source> DecodeString<S> for OwnedOutput {}
impl<S: Source, F: FnMut(WalkSpan)> DecodeString<S> for SpanOutput<'_, F, false> {
    fn string(
        &mut self,
        source: &mut S,
        major: u8,
        length: Option<u64>,
        deterministic: bool,
        at: u64,
        depth: usize,
    ) -> Result<(), WalkError> {
        if major != super::head::BYTES {
            return string_value(self, source, major, length, deterministic, at, depth);
        }
        string_chunks(
            source,
            major,
            length,
            deterministic,
            at,
            true,
            |source, len, _| source.skip_bytes(len),
        )?;
        self.record((at, source.offset()), depth);
        Ok(())
    }
}

impl<S: Source, F: FnMut(WalkSpan), const WIDE: bool> DecodeString<S>
    for SpanOutput<'_, F, true, WIDE>
{
    fn string(
        &mut self,
        source: &mut S,
        major: u8,
        length: Option<u64>,
        deterministic: bool,
        at: u64,
        depth: usize,
    ) -> Result<(), WalkError> {
        string_chunks(
            source,
            major,
            length,
            deterministic,
            at,
            !WIDE,
            |source, len, _| source.skip_bytes(len),
        )?;
        self.record((at, source.offset()), depth);
        Ok(())
    }
}

enum Open<B: DecodeOutput> {
    Array {
        start: u64,
        remaining: Option<u64>,
        items: B::Array,
    },
    Map {
        start: u64,
        remaining: Option<u64>,
        entries: B::Map,
        key: Option<B::Item>,
        last_key: Option<WalkSpan>,
    },
    Tag {
        start: u64,
        tag: u64,
    },
}

// This adapter keeps every existing owned decoder on the same implementation.
fn decode_item<S: Source>(
    source: &mut S,
    limits: Limits,
    deterministic: bool,
) -> Result<Value, DecodeError> {
    decode_item_to(source, limits, deterministic, &mut OwnedOutput).map_err(Into::into)
}

/// Visit complete item spans at `depth`, where the root has depth zero.
/// The callback records source locations; it must not publish graph events.
/// Its records are provisional until this function returns Ok, because a later
/// child can make the enclosing item malformed. The original input owns bytes.
///
/// Retains only open-container state and the current primitive. The existing
/// string decoder can still own one complete string; this does not yet provide
/// a borrowed-string decoder or a complete GTS memory bound.
fn visit_prefix(
    bytes: &[u8],
    limits: Limits,
    depth: usize,
    mut visit: impl FnMut(ItemSpan),
) -> Result<usize, DecodeError> {
    let mut source = Slice { bytes, pos: 0 };
    let mut record = |span| visit(native_span(span));
    decode_item_to(
        &mut source,
        limits,
        false,
        &mut SpanOutput::<_, false> {
            depth,
            visit: &mut record,
        },
    )?;
    Ok(source.pos)
}

fn attach_to<S: Source, B: DecodeOutput>(
    open: &mut Vec<Open<B>>,
    item: B::Item,
    mut start: u64,
    source: &S,
    deterministic: bool,
    output: &mut B,
) -> Result<Option<B::Item>, WalkError> {
    let mut item = Some(item);
    loop {
        let completed = match open.last_mut() {
            None => return Ok(item),
            Some(Open::Tag { .. }) => true,
            Some(Open::Array {
                remaining, items, ..
            }) => {
                output.push_array(items, item.take().expect("one completed CBOR child"));
                match remaining {
                    Some(n) => {
                        *n -= 1;
                        *n == 0
                    }
                    None => false,
                }
            }
            Some(Open::Map {
                remaining,
                entries,
                key,
                last_key,
                ..
            }) => {
                if let Some(pending) = key.take() {
                    output.push_map(entries, pending, item.take().expect("one CBOR map value"));
                    match remaining {
                        Some(n) => {
                            *n -= 1;
                            *n == 0
                        }
                        None => false,
                    }
                } else {
                    if deterministic {
                        let end = source.offset();
                        let bytes = source
                            .consumed()
                            .expect("the deterministic decoder reads a slice");
                        if let Some((from, to)) = *last_key
                            && bytes[native_offset(from)..native_offset(to)]
                                >= bytes[native_offset(start)..native_offset(end)]
                        {
                            return Err(WalkError::new(
                                DecodeErrorKind::NotDeterministic(
                                    "map keys must be in strictly ascending bytewise order",
                                ),
                                start,
                            ));
                        }
                        *last_key = Some((start, end));
                    }
                    *key = item.take();
                    return Ok(None);
                }
            }
        };
        if !completed {
            return Ok(None);
        }
        let depth = open.len() - 1;
        let end = source.offset();
        item = Some(match open.pop() {
            Some(Open::Tag { start: at, tag }) => {
                start = at;
                output.tag(
                    tag,
                    item.take().expect("one CBOR tag child"),
                    (start, end),
                    depth,
                )
            }
            Some(Open::Array {
                start: at, items, ..
            }) => {
                start = at;
                output.array(items, (start, end), depth)
            }
            Some(Open::Map {
                start: at, entries, ..
            }) => {
                start = at;
                output.map(entries, (start, end), depth)
            }
            None => unreachable!("an item was just completed"),
        });
    }
}

fn decode_item_to<S: Source, B: DecodeString<S>>(
    source: &mut S,
    limits: Limits,
    deterministic: bool,
    output: &mut B,
) -> Result<B::Item, WalkError> {
    let mut open: Vec<Open<B>> = Vec::new();
    loop {
        let at = source.offset();
        let initial = source.byte()?;
        let major = initial >> 5;
        if initial == 0xff {
            let (value, start) = match open.pop() {
                Some(Open::Array {
                    start,
                    remaining: None,
                    items,
                }) => {
                    let span = (start, source.offset());
                    (output.array(items, span, open.len()), start)
                }
                Some(Open::Map {
                    start,
                    remaining: None,
                    entries,
                    key: None,
                    ..
                }) => {
                    let span = (start, source.offset());
                    (output.map(entries, span, open.len()), start)
                }
                _ => {
                    return Err(WalkError::new(
                        DecodeErrorKind::Malformed("a break outside an indefinite array or map"),
                        at,
                    ));
                }
            };
            if let Some(done) = attach_to(&mut open, value, start, source, deterministic, output)? {
                return Ok(done);
            }
            continue;
        }
        let argument = argument(source, initial, at, deterministic)?;
        let depth = open.len();
        let scalar =
            |output: &mut B, value: Value, end: u64| output.scalar(value, (at, end), depth);
        let value = match (major, argument) {
            (0, Some(n)) => scalar(output, Value::Integer(Integer::from(n)), source.offset()),
            (1, Some(n)) => scalar(
                output,
                Value::Integer(
                    Integer::try_from(-1 - i128::from(n)).expect("-1 - u64 is within range"),
                ),
                source.offset(),
            ),
            (2 | 3, length) => output.string(source, major, length, deterministic, at, depth)?,
            (4..=6, _) if open.len() >= limits.max_depth => {
                return Err(WalkError::new(
                    DecodeErrorKind::Depth {
                        limit: limits.max_depth,
                    },
                    at,
                ));
            }
            (4 | 5, None) if deterministic => {
                return Err(WalkError::new(
                    DecodeErrorKind::NotDeterministic("a length must be definite"),
                    at,
                ));
            }
            (4, Some(0)) => output.array(B::Array::default(), (at, source.offset()), depth),
            (5, Some(0)) => output.map(B::Map::default(), (at, source.offset()), depth),
            (4, remaining) => {
                if !B::WIDE && remaining.is_some_and(|n| usize::try_from(n).is_err()) {
                    return Err(WalkError::new(DecodeErrorKind::TooLong, at));
                }
                open.push(Open::Array {
                    start: at,
                    remaining,
                    items: B::Array::default(),
                });
                continue;
            }
            (5, remaining) => {
                if !B::WIDE && remaining.is_some_and(|n| usize::try_from(n).is_err()) {
                    return Err(WalkError::new(DecodeErrorKind::TooLong, at));
                }
                open.push(Open::Map {
                    start: at,
                    remaining,
                    entries: B::Map::default(),
                    key: None,
                    last_key: None,
                });
                continue;
            }
            (6, Some(tag)) => {
                open.push(Open::Tag { start: at, tag });
                continue;
            }
            (7, Some(n)) => scalar(
                output,
                simple_or_float(initial, n, at, deterministic)?,
                source.offset(),
            ),
            (0 | 1 | 6 | 7, None) => {
                return Err(WalkError::new(
                    DecodeErrorKind::Malformed(
                        "only strings, arrays and maps have an indefinite length",
                    ),
                    at,
                ));
            }
            _ => unreachable!("the major type is three bits"),
        };
        if let Some(done) = attach_to(&mut open, value, at, source, deterministic, output)? {
            return Ok(done);
        }
    }
}

/// A head's argument: its value, or `None` for an indefinite length.
fn argument<S: Source>(
    source: &mut S,
    initial: u8,
    at: u64,
    deterministic: bool,
) -> Result<Option<u64>, WalkError> {
    let info = initial & 31;
    let width = match info {
        0..=23 => return Ok(Some(u64::from(info))),
        24 => 1,
        25 => 2,
        26 => 4,
        27 => 8,
        31 => return Ok(None),
        _ => {
            return Err(WalkError::new(
                DecodeErrorKind::Malformed("additional information 28 to 30 is reserved"),
                at,
            ));
        }
    };
    let mut value = 0_u64;
    for _ in 0..width {
        value = (value << 8) | u64::from(source.byte()?);
    }
    // A float's width is its precision, not an argument length; its own rule
    // is checked where it is decoded.
    if deterministic && initial >> 5 != 7 {
        let shortest = match width {
            1 => value >= 24,
            2 => value > 0xff,
            4 => value > 0xffff,
            _ => value > 0xffff_ffff,
        };
        if !shortest {
            return Err(WalkError::new(
                DecodeErrorKind::NotDeterministic("an argument must use its shortest form"),
                at,
            ));
        }
    }
    Ok(Some(value))
}

/// Widen a binary16 bit pattern.
fn f16_to_f64(half: u16) -> f64 {
    let sign = u64::from(half >> 15) << 63;
    let exponent = u64::from((half >> 10) & 0x1f);
    let mantissa = u64::from(half & 0x3ff);
    let bits = match exponent {
        0 if mantissa == 0 => sign,
        0 => {
            // A subnormal: normalize the mantissa.
            let shift = mantissa.leading_zeros() - 53;
            let normalized = (mantissa << shift) & 0x3ff;
            sign | ((1023 - 15 + 1 - u64::from(shift)) << 52) | (normalized << 42)
        }
        31 => sign | (0x7ff << 52) | (mantissa << 42),
        _ => sign | ((exponent + 1023 - 15) << 52) | (mantissa << 42),
    };
    f64::from_bits(bits)
}

/// A string's bytes: definite, or the concatenation of an indefinite string's
/// definite chunks of the same major type (RFC 8949 §3.2.3).
fn string_bytes<S: Source>(
    source: &mut S,
    major: u8,
    length: Option<u64>,
    deterministic: bool,
    at: u64,
) -> Result<Vec<u8>, WalkError> {
    let mut bytes = Vec::new();
    let indefinite = length.is_none();
    string_chunks(
        source,
        major,
        length,
        deterministic,
        at,
        true,
        |source, len, chunk_at| {
            let from = bytes.len();
            source.take(len, &mut bytes)?;
            if indefinite && major == 3 && core::str::from_utf8(&bytes[from..]).is_err() {
                return Err(WalkError::new(DecodeErrorKind::InvalidUtf8, chunk_at));
            }
            Ok(())
        },
    )?;
    Ok(bytes)
}

/// The one definite/indefinite string grammar, shared by admission and extent
/// navigation. Structural ranges skip bodies without granting admitted authority.
fn string_chunks<S: Source>(
    source: &mut S,
    major: u8,
    length: Option<u64>,
    deterministic: bool,
    at: u64,
    local_lengths: bool,
    mut chunk: impl FnMut(&mut S, u64, u64) -> Result<(), WalkError>,
) -> Result<(), WalkError> {
    let Some(length) = length else {
        if deterministic {
            return Err(WalkError::new(
                DecodeErrorKind::NotDeterministic("a length must be definite"),
                at,
            ));
        }
        while let Some((length, chunk_at)) = next_string_chunk(source, major)? {
            chunk(source, length, chunk_at)?;
        }
        return Ok(());
    };
    if local_lengths && usize::try_from(length).is_err() {
        return Err(WalkError::new(DecodeErrorKind::TooLong, at));
    }
    chunk(source, length, at)
}

fn next_string_chunk<S: Source>(
    source: &mut S,
    major: u8,
) -> Result<Option<(u64, u64)>, WalkError> {
    let at = source.offset();
    let initial = source.byte()?;
    if initial == 0xff {
        return Ok(None);
    }
    if initial >> 5 != major {
        return Err(WalkError::new(
            DecodeErrorKind::Malformed(
                "an indefinite string's chunks must be strings of its own type",
            ),
            at,
        ));
    }
    let length = argument(source, initial, at, false)?.ok_or_else(|| {
        WalkError::new(
            DecodeErrorKind::Malformed("an indefinite string's chunks must be definite"),
            at,
        )
    })?;
    Ok(Some((length, at)))
}

/// A major type 7 item: a simple value or a float.
fn simple_or_float(
    initial: u8,
    argument: u64,
    at: u64,
    deterministic: bool,
) -> Result<Value, WalkError> {
    let value = match initial & 31 {
        20 => Value::Bool(false),
        21 => Value::Bool(true),
        22 => Value::Null,
        info @ (0..=19 | 23) => Value::Simple(info),
        24 => {
            if argument < 32 {
                return Err(WalkError::new(
                    DecodeErrorKind::Malformed("a two-byte simple value must be at least 32"),
                    at,
                ));
            }
            Value::Simple(argument as u8)
        }
        25 => Value::Float(f16_to_f64(argument as u16)),
        26 => Value::Float(f64::from(f32::from_bits(argument as u32))),
        _ => Value::Float(f64::from_bits(argument)),
    };
    if deterministic && let Value::Float(float) = value {
        let (_, shortest) = super::encode::float_item(float);
        let width = match initial & 31 {
            25 => 3,
            26 => 5,
            _ => 9,
        };
        if width != shortest {
            return Err(WalkError::new(
                DecodeErrorKind::NotDeterministic("a float must use its shortest exact form"),
                at,
            ));
        }
    }
    Ok(value)
}

/// Decode the one item at the front of `bytes`: the item and how many bytes it
/// took. Trailing bytes are left for the caller.
///
/// # Errors
///
/// [`DecodeError`] for a malformed or truncated item or a [`Limits`] bound.
pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Value, usize), DecodeError> {
    let mut source = Slice { bytes, pos: 0 };
    let value = decode_item(&mut source, limits, false)?;
    Ok((value, source.pos))
}

/// Decode `bytes` as exactly one well-formed item.
///
/// Every well-formed item is accepted: long arguments, indefinite lengths,
/// unsorted and repeated map keys, and any tag or simple value.
///
/// # Errors
///
/// [`DecodeError`] for a malformed or truncated item, a [`Limits`] bound, or
/// [`DecodeErrorKind::Trailing`] bytes after it.
pub fn decode(bytes: &[u8], limits: Limits) -> Result<Value, DecodeError> {
    let (value, used) = decode_prefix(bytes, limits)?;
    if used != bytes.len() {
        return Err(DecodeError::new(DecodeErrorKind::Trailing, used));
    }
    Ok(value)
}

/// Decode `bytes` as exactly one item in core deterministic encoding (RFC 8949
/// §4.2.1): [`decode`], also refusing a long argument, an indefinite length, a
/// float that a shorter float holds exactly, and a map whose keys are not in
/// strictly ascending bytewise order of their encodings (so a repeated key is
/// refused too). Whatever [`super::canonical`] writes is accepted.
///
/// # Errors
///
/// [`DecodeErrorKind::NotDeterministic`] naming the rule, or any refusal of
/// [`decode`].
pub fn decode_deterministic(bytes: &[u8], limits: Limits) -> Result<Value, DecodeError> {
    let mut source = Slice { bytes, pos: 0 };
    let value = decode_item(&mut source, limits, true)?;
    if source.pos != bytes.len() {
        return Err(DecodeError::new(DecodeErrorKind::Trailing, source.pos));
    }
    Ok(value)
}

/// Decode the next well-formed item from `reader`, reading exactly its bytes
/// and no more; `Ok(None)` when the reader is at its end before the item's
/// first byte.
///
/// # Errors
///
/// [`DecodeError`] for a malformed or truncated item (the offset counts from
/// the item's first byte), a [`Limits`] bound, or the reader's failure.
pub fn read_from<R: Read + ?Sized>(
    reader: &mut R,
    limits: Limits,
) -> Result<Option<Value>, DecodeError> {
    let mut source = Stream { reader, pos: 0 };
    match decode_item(&mut source, limits, false) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind == DecodeErrorKind::Truncated && error.offset == 0 => Ok(None),
        Err(error) => Err(error),
    }
}

/// Decode a CBOR sequence (RFC 8742): every complete item with the offset it
/// starts at, and — when the bytes end inside an item or an item is malformed
/// — the offset of that item, where a torn append begins. The intact prefix is
/// returned either way.
pub fn decode_sequence(bytes: &[u8], limits: Limits) -> (Vec<(usize, Value)>, Option<usize>) {
    decode_sequence_with(bytes, limits, |offset, _cause| offset)
}

/// A complete sequence prefix and its original optional decode refusal.
#[derive(Debug, Default)]
pub struct DecodedSequence {
    /// Complete items in input order, with their original byte offsets.
    pub items: Vec<(usize, Value)>,
    /// First refusal, boxed only on failure to keep successful results compact.
    pub failure: Option<Box<SequenceError>>,
}

/// A sequence item refused at its original start offset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SequenceError {
    /// Byte offset where the refused item starts.
    pub offset: usize,
    /// Original refusal, whose offset is relative to that item.
    pub cause: DecodeError,
}

impl fmt::Display for SequenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CBOR sequence item at byte {}: {}",
            self.offset, self.cause
        )
    }
}

crate::error_source!(SequenceError, cause);

impl purrdf_hash::error::StructuredError for SequenceError {
    purrdf_hash::error_visit! {
        @record self { offset, cause } => "cbor-sequence-refusal" {
            "offset" => ByteCount(self.offset as u128), "cause" => Cause(&self.cause)
        }
    }
}

/// Decode a sequence once, retaining the complete prefix and original refusal.
///
/// The ordinary [`decode_sequence`] projects only the torn-item offset from
/// this same parser for callers that need its established tuple result.
pub fn decode_sequence_checked(bytes: &[u8], limits: Limits) -> DecodedSequence {
    let (items, failure) = decode_sequence_with(bytes, limits, |offset, cause| {
        Box::new(SequenceError { offset, cause })
    });
    DecodedSequence { items, failure }
}

fn decode_sequence_with<E>(
    bytes: &[u8],
    limits: Limits,
    refuse: impl FnOnce(usize, DecodeError) -> E,
) -> (Vec<(usize, Value)>, Option<E>) {
    let mut items = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match decode_prefix(&bytes[at..], limits) {
            Ok((value, used)) => {
                items.push((at, value));
                at += used;
            }
            Err(cause) => return (items, Some(refuse(at, cause))),
        }
    }
    (items, None)
}

/// An immutable source-backed CBOR item validated by the shared decoder.
#[derive(Clone, Copy, Debug)]
pub struct BorrowedItem<'a> {
    encoded: &'a [u8],
    limits: Limits,
}

/// Original byte-string chunks without concatenating indefinite encodings.
#[derive(Clone, Copy)]
pub struct ByteChunks<'a> {
    pending: Option<&'a [u8]>,
    encoded: &'a [u8],
}

purrdf_hash::debug_non_exhaustive!(['a] ByteChunks<'a> {});

impl<'a> ByteChunks<'a> {
    /// Borrow one existing byte slice as a single chunk.
    pub const fn from_slice(bytes: &'a [u8]) -> Self {
        Self {
            pending: Some(bytes),
            encoded: &[],
        }
    }
    /// Exact remaining payload length, excluding CBOR chunk heads.
    #[must_use]
    pub fn byte_len(self) -> usize {
        // Every chunk is disjoint inside one admitted slice, so their combined
        // payload length cannot exceed that slice's addressable extent.
        self.map(<[u8]>::len).sum()
    }
    /// Borrow a contiguous backing slice when no encoded chunks remain.
    pub fn contiguous(self) -> Option<&'a [u8]> {
        self.encoded
            .is_empty()
            .then_some(self.pending.unwrap_or(&[]))
    }
}

#[allow(
    clippy::copy_iterator,
    reason = "A borrowed cursor is intentionally copied to hash and replay the same admitted bytes without retaining payloads"
)]
impl<'a> Iterator for ByteChunks<'a> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(bytes) = self.pending.take() {
            return Some(bytes);
        }
        if self.encoded.is_empty() {
            return None;
        }
        // Initial admission established that every indefinite byte-string
        // child is a complete definite byte string of this same major type.
        Some(
            super::head::read_bytes(&mut self.encoded)
                .expect("INVARIANT: admitted byte-string chunks have definite byte heads"),
        )
    }
}

impl Read for ByteChunks<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let mut written = 0;
        while written < output.len() {
            let Some(chunk) = self.next() else {
                break;
            };
            let count = chunk.len().min(output.len() - written);
            output[written..written + count].copy_from_slice(&chunk[..count]);
            written += count;
            if count < chunk.len() {
                self.pending = Some(&chunk[count..]);
            }
        }
        Ok(written)
    }
}

/// Validate the front item without retaining a Value tree and borrow its bytes.
pub fn borrowed_prefix(
    bytes: &[u8],
    limits: Limits,
) -> Result<(BorrowedItem<'_>, usize), DecodeError> {
    let end = visit_prefix(bytes, limits, 0, |_| {})?;
    Ok((
        BorrowedItem {
            encoded: &bytes[..end],
            limits,
        },
        end,
    ))
}

impl<'a> BorrowedItem<'a> {
    /// Borrow the chunks of an admitted byte string without flattening them.
    pub fn byte_chunks(self) -> Option<ByteChunks<'a>> {
        if self.major() != super::head::BYTES {
            return None;
        }
        if self.encoded[0] & 31 == 31 {
            return Some(ByteChunks {
                pending: None,
                encoded: &self.encoded[1..self.encoded.len() - 1],
            });
        }
        let mut encoded = self.encoded;
        let bytes = super::head::read_bytes(&mut encoded)
            .expect("INVARIANT: admitted definite byte strings have their declared extent");
        Some(ByteChunks::from_slice(bytes))
    }
    /// The exact original encoding, including indefinite and non-shortest forms.
    pub const fn encoded(self) -> &'a [u8] {
        self.encoded
    }

    /// Materialize only this item through the existing owned decoder.
    pub fn decode(self) -> Result<Value, DecodeError> {
        decode(self.encoded, self.limits)
    }

    /// The already-validated major type; every item has an initial byte.
    pub fn major(self) -> u8 {
        self.encoded[0] >> 5
    }

    fn children(self, major: u8) -> Option<BorrowedChildren<'a>> {
        if self.major() != major {
            return None;
        }
        let mut body = self.encoded;
        if super::head::read_argument(&mut body, major).is_none() {
            // The shared decoder admitted an indefinite array/map. Its last
            // byte is its own break, not a descendant's or a string's byte.
            assert_eq!(
                self.encoded[0] & 31,
                31,
                "validated container has a valid head"
            );
            body = &self.encoded[1..self.encoded.len() - 1];
        }
        Some(BorrowedChildren {
            remaining: body,
            limits: self.limits,
        })
    }

    /// Array elements in original order, retaining only the unvisited suffix.
    pub fn array(self) -> Option<BorrowedChildren<'a>> {
        self.children(super::head::ARRAY)
    }

    /// Map entries in original order. Duplicate keys retain their positions.
    pub fn map(self) -> Option<BorrowedEntries<'a>> {
        self.children(super::head::MAP)
            .map(|children| BorrowedEntries { children })
    }

    /// A tag and its sole checked child. No owned child box is introduced.
    pub fn tag(self) -> Option<(u64, Self)> {
        if self.major() != super::head::TAG {
            return None;
        }
        let mut body = self.encoded;
        let tag = super::head::read_argument(&mut body, super::head::TAG)
            .expect("the shared decoder validated the tag argument");
        Some((
            tag,
            Self {
                encoded: body,
                limits: self.limits,
            },
        ))
    }

    /// Borrow definite text; concatenate indefinite chunks through the same
    /// string decoder the owned API uses. A non-text item returns None.
    pub fn text(self) -> Result<Option<std::borrow::Cow<'a, str>>, DecodeError> {
        if self.major() != super::head::TEXT {
            return Ok(None);
        }
        let mut encoded = self.encoded;
        if let Some(text) = super::head::read_text(&mut encoded) {
            assert!(
                encoded.is_empty(),
                "a validated text item has no trailing bytes"
            );
            return Ok(Some(std::borrow::Cow::Borrowed(text)));
        }
        let text = self
            .decode()?
            .into_text()
            .expect("the shared decoder admitted this text item");
        Ok(Some(std::borrow::Cow::Owned(text)))
    }

    /// Borrow definite bytes, or own the existing decoder's concatenated chunks.
    pub fn bytes(self) -> Result<Option<std::borrow::Cow<'a, [u8]>>, DecodeError> {
        if self.major() != super::head::BYTES {
            return Ok(None);
        }
        let mut encoded = self.encoded;
        if let Some(bytes) = super::head::read_bytes(&mut encoded) {
            assert!(
                encoded.is_empty(),
                "a validated byte string has no trailing bytes"
            );
            return Ok(Some(std::borrow::Cow::Borrowed(bytes)));
        }
        let bytes = self
            .decode()?
            .into_bytes()
            .expect("the shared decoder admitted this byte string");
        Ok(Some(std::borrow::Cow::Owned(bytes)))
    }

    /// The first text-keyed entry, matching the existing Value map_get rule.
    pub fn get(self, name: &str) -> Result<Option<Self>, DecodeError> {
        let Some(entries) = self.map() else {
            return Ok(None);
        };
        for entry in entries {
            let (key, value) = entry?;
            if key.text()?.is_some_and(|key| key == name) {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }
}

#[derive(Clone, Debug)]
/// Forward children of a validated array or map, borrowing their source bytes.
pub struct BorrowedChildren<'a> {
    remaining: &'a [u8],
    limits: Limits,
}

impl<'a> Iterator for BorrowedChildren<'a> {
    type Item = Result<BorrowedItem<'a>, DecodeError>;

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.remaining.is_empty() {
            (0, Some(0))
        } else {
            (1, None)
        }
    }

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining.is_empty() {
            return None;
        }
        // This private cursor only receives children of an immutable item
        // already admitted by the shared decoder. Reuse its container machine
        // and string grammar, but skip admitted string bytes without allocating
        // or validating their UTF-8 again.
        let mut source = Slice {
            bytes: self.remaining,
            pos: 0,
        };
        let mut ignore = |_| {};
        let result = decode_item_to(
            &mut source,
            self.limits,
            false,
            &mut SpanOutput::<_, true> {
                depth: 0,
                visit: &mut ignore,
            },
        );
        match result {
            Ok(()) => {
                let (encoded, remaining) = self.remaining.split_at(source.pos);
                self.remaining = remaining;
                Some(Ok(BorrowedItem {
                    encoded,
                    limits: self.limits,
                }))
            }
            Err(error) => {
                self.remaining = &[];
                Some(Err(error.into()))
            }
        }
    }
}

impl std::iter::FusedIterator for BorrowedChildren<'_> {}

#[derive(Clone, Debug)]
/// Forward map entries, preserving duplicate keys and original order.
pub struct BorrowedEntries<'a> {
    children: BorrowedChildren<'a>,
}

impl<'a> Iterator for BorrowedEntries<'a> {
    type Item = Result<(BorrowedItem<'a>, BorrowedItem<'a>), DecodeError>;

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.children.size_hint().1 == Some(0) {
            (0, Some(0))
        } else {
            (1, None)
        }
    }

    fn next(&mut self) -> Option<Self::Item> {
        let key = match self.children.next()? {
            Ok(key) => key,
            Err(error) => return Some(Err(error)),
        };
        let value = match self
            .children
            .next()
            .expect("a checked map has a value for every key")
        {
            Ok(value) => value,
            Err(error) => return Some(Err(error)),
        };
        Some(Ok((key, value)))
    }
}

impl std::iter::FusedIterator for BorrowedEntries<'_> {}

/// One validated encoded item, retaining its original accepted byte spelling.
#[derive(Debug)]
pub struct EncodedItem {
    encoded: Vec<u8>,
    limits: Limits,
}

impl EncodedItem {
    /// Borrow the already validated item without parsing or copying it again.
    pub fn view(&self) -> BorrowedItem<'_> {
        BorrowedItem {
            encoded: &self.encoded,
            limits: self.limits,
        }
    }
}

/// Observe only successful Source operations. The native Source remains the
/// owner of malformed reader counts, Interrupted handling and exact offsets.
struct Capturing<S> {
    source: S,
    encoded: Vec<u8>,
}

impl<S: Source> Source for Capturing<S> {
    fn offset(&self) -> u64 {
        self.source.offset()
    }

    fn byte(&mut self) -> Result<u8, WalkError> {
        let byte = self.source.byte()?;
        self.encoded.push(byte);
        Ok(byte)
    }

    fn take(&mut self, len: u64, out: &mut Vec<u8>) -> Result<(), WalkError> {
        let start = out.len();
        // Keep the exact original result. A partial failed read can append
        // bytes, but its owner is never published as a validated item.
        let result = self.source.take(len, out);
        self.encoded.extend_from_slice(&out[start..]);
        result
    }

    fn consumed(&self) -> Option<&[u8]> {
        Some(&self.encoded)
    }
}

/// Read exactly the next item, validating through the shared decoder while
/// retaining encoded bytes rather than an owned Value tree. Clean EOF and all
/// native refusal positions follow read_from's exact decision.
///
/// The existing primitive decoder can still allocate one complete string.
/// During that read its bytes coexist with the captured item. This is not yet
/// a bound on primitive-string workspace, validation terms or source size.
pub fn read_encoded_from<R: Read + ?Sized>(
    reader: &mut R,
    limits: Limits,
) -> Result<Option<EncodedItem>, DecodeError> {
    let mut source = Capturing {
        source: Stream { reader, pos: 0 },
        encoded: Vec::new(),
    };
    let mut ignore = |_| {};
    let mut output = SpanOutput::<_, false> {
        depth: 0,
        visit: &mut ignore,
    };
    match decode_item_to(&mut source, limits, false, &mut output) {
        Ok(()) => Ok(Some(EncodedItem {
            encoded: source.encoded,
            limits,
        })),
        Err(error) if error.kind == DecodeErrorKind::Truncated && error.offset == 0 => Ok(None),
        Err(error) => Err(error.into()),
    }
}

impl EncodedItem {
    /// Validate one complete item and retain its existing byte owner.
    ///
    /// # Errors
    /// Preserves the original CBOR refusal, including trailing bytes.
    pub fn from_bytes(encoded: Vec<u8>, limits: Limits) -> Result<Self, DecodeError> {
        let end = visit_prefix(&encoded, limits, 0, |_| {})?;
        if end != encoded.len() {
            return Err(DecodeError::new(DecodeErrorKind::Trailing, end));
        }
        Ok(Self { encoded, limits })
    }
    /// Materialize this validated item through the shared decoder.
    ///
    /// # Errors
    /// Preserves the original CBOR inspection refusal.
    pub fn decode(&self) -> Result<Value, DecodeError> {
        decode(&self.encoded, self.limits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_refuses_invalid_read_counts_and_counter_overflow() {
        struct InvalidRead;
        impl Read for InvalidRead {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Ok(usize::MAX)
            }
        }
        let error = read_from(&mut InvalidRead, Limits::DEFAULT).unwrap_err();
        assert!(matches!(
            error.kind(),
            DecodeErrorKind::ReadCount {
                actual: usize::MAX,
                maximum: 1
            }
        ));
        // Counter state is positioned directly; no huge stream is executed.
        let mut cursor = io::Cursor::new([0]);
        let mut stream = Stream {
            reader: &mut cursor,
            pos: usize::MAX,
        };
        let error = DecodeError::from(stream.byte().unwrap_err());
        assert_eq!(error.offset(), usize::MAX);
        assert!(matches!(
            error.kind(),
            DecodeErrorKind::OffsetOverflow {
                position: usize::MAX,
                additional: 1
            }
        ));
        assert_eq!(stream.pos, usize::MAX);
        let mut cursor = io::Cursor::new([7]);
        let mut stream = Stream {
            reader: &mut cursor,
            pos: usize::MAX,
        };
        let error = DecodeError::from(stream.take(1, &mut Vec::new()).unwrap_err());
        assert_eq!(error.offset(), usize::MAX);
        assert!(matches!(
            error.kind(),
            DecodeErrorKind::OffsetOverflow {
                position: usize::MAX,
                additional: 1
            }
        ));
        assert_eq!(stream.pos, usize::MAX);
    }
}
