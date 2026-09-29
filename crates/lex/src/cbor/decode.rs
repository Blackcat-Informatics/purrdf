// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The CBOR decoders: well-formed (RFC 8949 §4.1, with Appendix C's rules)
//! and core deterministic (§4.2.1), over an explicit stack of open items.

use core::fmt;
use std::io::{self, Read};

use super::{Integer, Value};

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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    /// The reader failed.
    Io(io::ErrorKind),
}

/// A refused CBOR item: what is wrong, and the byte offset where.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodeError {
    kind: DecodeErrorKind,
    offset: usize,
}

impl DecodeError {
    const fn new(kind: DecodeErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    /// What is wrong.
    pub const fn kind(&self) -> DecodeErrorKind {
        self.kind
    }

    /// The byte offset of the defect, from the start of the input.
    pub const fn offset(&self) -> usize {
        self.offset
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CBOR byte {}: ", self.offset)?;
        match self.kind {
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
            DecodeErrorKind::Io(kind) => write!(f, "read failed: {kind}"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Where the decoder's bytes come from.
trait Source {
    /// The offset of the next byte.
    fn offset(&self) -> usize;
    /// The next byte.
    fn byte(&mut self) -> Result<u8, DecodeError>;
    /// Append the next `len` bytes to `out`.
    fn take(&mut self, len: u64, out: &mut Vec<u8>) -> Result<(), DecodeError>;
    /// The bytes already read, for the deterministic key-order check.
    fn consumed(&self) -> Option<&[u8]>;
}

struct Slice<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Source for Slice<'_> {
    fn offset(&self) -> usize {
        self.pos
    }

    fn byte(&mut self) -> Result<u8, DecodeError> {
        let byte = *self
            .bytes
            .get(self.pos)
            .ok_or_else(|| DecodeError::new(DecodeErrorKind::Truncated, self.pos))?;
        self.pos += 1;
        Ok(byte)
    }

    fn take(&mut self, len: u64, out: &mut Vec<u8>) -> Result<(), DecodeError> {
        let available = self.bytes.len() - self.pos;
        let len = usize::try_from(len)
            .ok()
            .filter(|&len| len <= available)
            .ok_or_else(|| DecodeError::new(DecodeErrorKind::Truncated, self.bytes.len()))?;
        out.extend_from_slice(&self.bytes[self.pos..self.pos + len]);
        self.pos += len;
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
    fn offset(&self) -> usize {
        self.pos
    }

    fn byte(&mut self) -> Result<u8, DecodeError> {
        let mut byte = [0_u8];
        self.reader.read_exact(&mut byte).map_err(|error| {
            let kind = if error.kind() == io::ErrorKind::UnexpectedEof {
                DecodeErrorKind::Truncated
            } else {
                DecodeErrorKind::Io(error.kind())
            };
            DecodeError::new(kind, self.pos)
        })?;
        self.pos += 1;
        Ok(byte[0])
    }

    fn take(&mut self, len: u64, out: &mut Vec<u8>) -> Result<(), DecodeError> {
        // `take` + `read_to_end` grows the buffer as bytes arrive, so a
        // declared length the stream does not back allocates nothing extra.
        let before = out.len();
        let read = (&mut *self.reader)
            .take(len)
            .read_to_end(out)
            .map_err(|error| DecodeError::new(DecodeErrorKind::Io(error.kind()), self.pos))?;
        self.pos += read;
        if (out.len() - before) as u64 == len {
            Ok(())
        } else {
            Err(DecodeError::new(DecodeErrorKind::Truncated, self.pos))
        }
    }

    fn consumed(&self) -> Option<&[u8]> {
        None
    }
}

/// An array, map or tag whose items are still being read.
enum Open {
    Array {
        start: usize,
        remaining: Option<u64>,
        items: Vec<Value>,
    },
    Map {
        start: usize,
        remaining: Option<u64>,
        entries: Vec<(Value, Value)>,
        key: Option<Value>,
        /// The span of the previous key's encoding (deterministic mode).
        last_key: Option<(usize, usize)>,
    },
    Tag {
        start: usize,
        tag: u64,
    },
}

/// A head's argument: its value, or `None` for an indefinite length.
fn argument<S: Source>(
    source: &mut S,
    initial: u8,
    at: usize,
    deterministic: bool,
) -> Result<Option<u64>, DecodeError> {
    let info = initial & 31;
    let width = match info {
        0..=23 => return Ok(Some(u64::from(info))),
        24 => 1,
        25 => 2,
        26 => 4,
        27 => 8,
        31 => return Ok(None),
        _ => {
            return Err(DecodeError::new(
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
            return Err(DecodeError::new(
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
    at: usize,
) -> Result<Vec<u8>, DecodeError> {
    let mut bytes = Vec::new();
    let Some(length) = length else {
        if deterministic {
            return Err(DecodeError::new(
                DecodeErrorKind::NotDeterministic("a length must be definite"),
                at,
            ));
        }
        loop {
            let chunk_at = source.offset();
            let initial = source.byte()?;
            if initial == 0xff {
                return Ok(bytes);
            }
            if initial >> 5 != major {
                return Err(DecodeError::new(
                    DecodeErrorKind::Malformed(
                        "an indefinite string's chunks must be strings of its own type",
                    ),
                    chunk_at,
                ));
            }
            let Some(chunk) = argument(source, initial, chunk_at, false)? else {
                return Err(DecodeError::new(
                    DecodeErrorKind::Malformed("an indefinite string's chunks must be definite"),
                    chunk_at,
                ));
            };
            let from = bytes.len();
            source.take(chunk, &mut bytes)?;
            if major == 3 && core::str::from_utf8(&bytes[from..]).is_err() {
                return Err(DecodeError::new(DecodeErrorKind::InvalidUtf8, chunk_at));
            }
        }
    };
    if usize::try_from(length).is_err() {
        return Err(DecodeError::new(DecodeErrorKind::TooLong, at));
    }
    source.take(length, &mut bytes)?;
    Ok(bytes)
}

/// Decode one item from `source`.
fn decode_item<S: Source>(
    source: &mut S,
    limits: Limits,
    deterministic: bool,
) -> Result<Value, DecodeError> {
    let mut open: Vec<Open> = Vec::new();
    loop {
        let at = source.offset();
        let initial = source.byte()?;
        let major = initial >> 5;
        if initial == 0xff {
            // A break closes the innermost indefinite array or map.
            let (value, start) = match open.pop() {
                Some(Open::Array {
                    start,
                    remaining: None,
                    items,
                }) => (Value::Array(items), start),
                Some(Open::Map {
                    start,
                    remaining: None,
                    entries,
                    key: None,
                    ..
                }) => (Value::Map(entries), start),
                _ => {
                    return Err(DecodeError::new(
                        DecodeErrorKind::Malformed("a break outside an indefinite array or map"),
                        at,
                    ));
                }
            };
            if let Some(done) = attach(&mut open, value, start, source, deterministic)? {
                return Ok(done);
            }
            continue;
        }
        let argument = argument(source, initial, at, deterministic)?;
        let value = match (major, argument) {
            (0, Some(n)) => Value::Integer(Integer::from(n)),
            (1, Some(n)) => Value::Integer(
                Integer::try_from(-1 - i128::from(n)).expect("-1 - u64 is within range"),
            ),
            (2, length) => Value::Bytes(string_bytes(source, 2, length, deterministic, at)?),
            (3, length) => {
                let bytes = string_bytes(source, 3, length, deterministic, at)?;
                Value::Text(
                    String::from_utf8(bytes)
                        .map_err(|_| DecodeError::new(DecodeErrorKind::InvalidUtf8, at))?,
                )
            }
            // Every array, map and tag counts against the cap, an empty one
            // included, as every JSON container does.
            (4..=6, _) if open.len() >= limits.max_depth => {
                return Err(DecodeError::new(
                    DecodeErrorKind::Depth {
                        limit: limits.max_depth,
                    },
                    at,
                ));
            }
            (4 | 5, None) if deterministic => {
                return Err(DecodeError::new(
                    DecodeErrorKind::NotDeterministic("a length must be definite"),
                    at,
                ));
            }
            (4, Some(0)) => Value::Array(Vec::new()),
            (5, Some(0)) => Value::Map(Vec::new()),
            (4, remaining) => {
                if remaining.is_some_and(|n| usize::try_from(n).is_err()) {
                    return Err(DecodeError::new(DecodeErrorKind::TooLong, at));
                }
                open.push(Open::Array {
                    start: at,
                    remaining,
                    items: Vec::new(),
                });
                continue;
            }
            (5, remaining) => {
                if remaining.is_some_and(|n| usize::try_from(n).is_err()) {
                    return Err(DecodeError::new(DecodeErrorKind::TooLong, at));
                }
                open.push(Open::Map {
                    start: at,
                    remaining,
                    entries: Vec::new(),
                    key: None,
                    last_key: None,
                });
                continue;
            }
            (6, Some(tag)) => {
                open.push(Open::Tag { start: at, tag });
                continue;
            }
            (7, Some(n)) => simple_or_float(initial, n, at, deterministic)?,
            (0 | 1 | 6 | 7, None) => {
                return Err(DecodeError::new(
                    DecodeErrorKind::Malformed(
                        "only strings, arrays and maps have an indefinite length",
                    ),
                    at,
                ));
            }
            _ => unreachable!("the major type is three bits"),
        };
        if let Some(done) = attach(&mut open, value, at, source, deterministic)? {
            return Ok(done);
        }
    }
}

/// A major type 7 item: a simple value or a float.
fn simple_or_float(
    initial: u8,
    argument: u64,
    at: usize,
    deterministic: bool,
) -> Result<Value, DecodeError> {
    let value = match initial & 31 {
        20 => Value::Bool(false),
        21 => Value::Bool(true),
        22 => Value::Null,
        info @ (0..=19 | 23) => Value::Simple(info),
        24 => {
            if argument < 32 {
                return Err(DecodeError::new(
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
            return Err(DecodeError::new(
                DecodeErrorKind::NotDeterministic("a float must use its shortest exact form"),
                at,
            ));
        }
    }
    Ok(value)
}

/// Hand a complete `value` (encoded from `start`) to the innermost open item,
/// closing every item it completes; the whole decoded item once the stack is
/// empty.
fn attach<S: Source>(
    open: &mut Vec<Open>,
    mut value: Value,
    mut start: usize,
    source: &S,
    deterministic: bool,
) -> Result<Option<Value>, DecodeError> {
    loop {
        let completed = match open.last_mut() {
            None => return Ok(Some(value)),
            Some(Open::Tag { .. }) => true,
            Some(Open::Array {
                remaining, items, ..
            }) => {
                items.push(value);
                value = Value::Null;
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
                    entries.push((pending, value));
                    value = Value::Null;
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
                            && bytes[from..to] >= bytes[start..end]
                        {
                            return Err(DecodeError::new(
                                DecodeErrorKind::NotDeterministic(
                                    "map keys must be in strictly ascending bytewise order",
                                ),
                                start,
                            ));
                        }
                        *last_key = Some((start, end));
                    }
                    *key = Some(value);
                    return Ok(None);
                }
            }
        };
        if !completed {
            return Ok(None);
        }
        value = match open.pop() {
            Some(Open::Tag { start: at, tag }) => {
                start = at;
                Value::Tag(tag, Box::new(value))
            }
            Some(Open::Array {
                start: at, items, ..
            }) => {
                start = at;
                Value::Array(items)
            }
            Some(Open::Map {
                start: at, entries, ..
            }) => {
                start = at;
                Value::Map(entries)
            }
            None => unreachable!("an item was just completed"),
        };
    }
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
    let mut items = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match decode_prefix(&bytes[at..], limits) {
            Ok((value, used)) => {
                items.push((at, value));
                at += used;
            }
            Err(_) => return (items, Some(at)),
        }
    }
    (items, None)
}
