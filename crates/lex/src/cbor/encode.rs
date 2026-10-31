// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The CBOR encoders: as-is and RFC 8949 §4.2.1 core deterministic, streamed
//! over a heap work list.

use std::io::{self, Write};

use super::Value;
use super::head::{self, ARRAY, BYTES, MAP, SIMPLE, TAG, TEXT};

/// The binary16 bit pattern that is exactly `value`, when there is one.
fn f16_bits(value: f64) -> Option<u16> {
    let bits = value.to_bits();
    let sign = ((bits >> 63) as u16) << 15;
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let mantissa = bits & ((1 << 52) - 1);
    if exponent == 0x7ff {
        // An infinity, or a NaN whose payload fits ten bits.
        return (mantissa & ((1 << 42) - 1) == 0)
            .then_some(sign | 0x7c00 | (mantissa >> 42) as u16);
    }
    if exponent == 0 {
        // Zero keeps its sign; a binary64 subnormal is far below binary16.
        return (mantissa == 0).then_some(sign);
    }
    let unbiased = exponent - 1023;
    if (-14..=15).contains(&unbiased) {
        return (mantissa & ((1 << 42) - 1) == 0)
            .then_some(sign | (((unbiased + 15) as u16) << 10) | (mantissa >> 42) as u16);
    }
    if (-24..-14).contains(&unbiased) {
        // A binary16 subnormal: the value is m * 2^-24 for an integer m < 1024.
        let significand = (1_u64 << 52) | mantissa;
        let shift = (52 - (unbiased + 24)) as u32;
        return (significand & ((1_u64 << shift) - 1) == 0)
            .then_some(sign | (significand >> shift) as u16);
    }
    None
}

/// The binary32 bit pattern that is exactly `value`, when there is one.
fn f32_bits(value: f64) -> Option<u32> {
    let bits = value.to_bits();
    let sign = ((bits >> 63) as u32) << 31;
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let mantissa = bits & ((1 << 52) - 1);
    if exponent == 0x7ff {
        return (mantissa & ((1 << 29) - 1) == 0)
            .then_some(sign | 0x7f80_0000 | (mantissa >> 29) as u32);
    }
    if exponent == 0 {
        return (mantissa == 0).then_some(sign);
    }
    let unbiased = exponent - 1023;
    if (-126..=127).contains(&unbiased) {
        return (mantissa & ((1 << 29) - 1) == 0)
            .then_some(sign | (((unbiased + 127) as u32) << 23) | (mantissa >> 29) as u32);
    }
    if (-149..-126).contains(&unbiased) {
        let significand = (1_u64 << 52) | mantissa;
        let shift = (52 - (unbiased + 149)) as u32;
        return (significand & ((1_u64 << shift) - 1) == 0)
            .then_some(sign | (significand >> shift) as u32);
    }
    None
}

/// The shortest float item that is exactly `value`: `f9` + binary16, `fa` +
/// binary32 or `fb` + binary64, comparing bit patterns.
pub(crate) fn float_item(value: f64) -> ([u8; 9], usize) {
    let mut bytes = [0_u8; 9];
    if let Some(half) = f16_bits(value) {
        bytes[0] = 0xf9;
        bytes[1..3].copy_from_slice(&half.to_be_bytes());
        (bytes, 3)
    } else if let Some(single) = f32_bits(value) {
        bytes[0] = 0xfa;
        bytes[1..5].copy_from_slice(&single.to_be_bytes());
        (bytes, 5)
    } else {
        bytes[0] = 0xfb;
        bytes[1..9].copy_from_slice(&value.to_bits().to_be_bytes());
        (bytes, 9)
    }
}

/// Write a value that owns no value; `false` for a container, which the
/// caller walks.
fn write_scalar<W: Write + ?Sized>(value: &Value, out: &mut W) -> io::Result<bool> {
    match value {
        Value::Integer(integer) => {
            let (major, argument) = integer.head();
            head::write_head(out, major, argument)?;
        }
        Value::Bytes(bytes) => {
            head::write_head(out, BYTES, bytes.len() as u64)?;
            out.write_all(bytes)?;
        }
        Value::Text(text) => {
            head::write_head(out, TEXT, text.len() as u64)?;
            out.write_all(text.as_bytes())?;
        }
        Value::Float(float) => {
            let (bytes, len) = float_item(*float);
            out.write_all(&bytes[..len])?;
        }
        Value::Bool(false) => out.write_all(&[0xf4])?,
        Value::Bool(true) => out.write_all(&[0xf5])?,
        Value::Null => out.write_all(&[0xf6])?,
        Value::Simple(simple) => head::write_head(out, SIMPLE, u64::from(*simple))?,
        Value::Tag(..) | Value::Array(_) | Value::Map(_) => return Ok(false),
    }
    Ok(true)
}

use super::decode::{BorrowedItem, DecodeError};
use super::value_ref::{Entries, ValueRef, Values};

#[derive(Debug)]
/// An original output failure or refusal while inspecting validated source bytes.
pub enum WriteError {
    /// Original writer failure.
    Io(purrdf_hash::error::IoFailure),
    /// Original CBOR inspection refusal.
    Decode(DecodeError),
    /// The map-only writer was given another item kind.
    ExpectedMap,
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => error.fmt(f),
            Self::Decode(error) => error.fmt(f),
            Self::ExpectedMap => f.write_str("canonical map writer requires a map"),
        }
    }
}
purrdf_hash::error_sources!(WriteError, Io, Decode);
impl purrdf_hash::error::StructuredError for WriteError {
    purrdf_hash::error_visit! {
        Io(cause) => "cbor-write-io" { "cause" => Cause(cause) },
        Decode(cause) => "cbor-write-source" { "cause" => Cause(cause) },
        ExpectedMap => "cbor-write-map-required" {},
    }
}

crate::variant_from!(WriteError {
    Io(purrdf_hash::error::IoFailure), Decode(DecodeError)
});
#[derive(Debug)]
enum WriteFault {
    Io(io::Error),
    Decode(DecodeError),
}
crate::variant_from!(WriteFault { Io(io::Error), Decode(DecodeError) });
impl From<WriteFault> for WriteError {
    fn from(error: WriteFault) -> Self {
        match error {
            WriteFault::Io(error) => Self::Io(error.into()),
            WriteFault::Decode(error) => Self::Decode(error),
        }
    }
}

trait Encodable: Copy {
    type Values: Iterator<Item = Result<Self, DecodeError>>;
    type Entries: Iterator<Item = Result<(Self, Self), DecodeError>>;
    fn scalar<W: Write + ?Sized>(self, out: &mut W) -> Result<bool, WriteFault>;
    fn excluded(self, excluded: &[&str]) -> Result<bool, DecodeError>;
    fn tag(self) -> Option<(u64, Self)>;
    fn array(self) -> Result<Option<(usize, Self::Values)>, DecodeError>;
    fn map(self) -> Result<Option<(usize, Self::Entries)>, DecodeError>;
    fn enter_array(items: Self::Values, steps: &mut Vec<Step<Self>>) -> Result<(), DecodeError>;
    fn enter_map(entries: Self::Entries, steps: &mut Vec<Step<Self>>) -> Result<(), DecodeError>;
}

fn owned_child(value: &Value) -> Result<&Value, DecodeError> {
    Ok(value)
}
fn owned_entry(entry: &(Value, Value)) -> Result<(&Value, &Value), DecodeError> {
    Ok((&entry.0, &entry.1))
}

impl<'v> Encodable for &'v Value {
    type Values = core::iter::Map<
        core::slice::Iter<'v, Value>,
        fn(&'v Value) -> Result<&'v Value, DecodeError>,
    >;
    type Entries = core::iter::Map<
        core::slice::Iter<'v, (Value, Value)>,
        fn(&'v (Value, Value)) -> Result<(&'v Value, &'v Value), DecodeError>,
    >;

    fn scalar<W: Write + ?Sized>(self, out: &mut W) -> Result<bool, WriteFault> {
        write_scalar(self, out).map_err(WriteFault::Io)
    }
    fn excluded(self, excluded: &[&str]) -> Result<bool, DecodeError> {
        Ok(self.as_text().is_some_and(|text| excluded.contains(&text)))
    }
    fn tag(self) -> Option<(u64, Self)> {
        match self {
            Value::Tag(tag, item) => Some((*tag, item.as_ref())),
            _ => None,
        }
    }
    fn array(self) -> Result<Option<(usize, Self::Values)>, DecodeError> {
        Ok(match self {
            Value::Array(items) => Some((items.len(), items.iter().map(owned_child))),
            _ => None,
        })
    }
    fn map(self) -> Result<Option<(usize, Self::Entries)>, DecodeError> {
        Ok(match self {
            Value::Map(entries) => Some((entries.len(), entries.iter().map(owned_entry))),
            _ => None,
        })
    }
    fn enter_array(items: Self::Values, steps: &mut Vec<Step<Self>>) -> Result<(), DecodeError> {
        steps.extend(
            items
                .rev()
                .map(|item| Step::Enter(item.expect("owned children cannot decode"))),
        );
        Ok(())
    }
    fn enter_map(entries: Self::Entries, steps: &mut Vec<Step<Self>>) -> Result<(), DecodeError> {
        for entry in entries.rev() {
            let (key, value) = entry.expect("owned entries cannot decode");
            steps.push(Step::Enter(value));
            steps.push(Step::Enter(key));
        }
        Ok(())
    }
}

impl<'v> Encodable for BorrowedItem<'v> {
    type Values = core::iter::Map<
        Values<'v>,
        fn(Result<ValueRef<'v>, DecodeError>) -> Result<Self, DecodeError>,
    >;
    type Entries = core::iter::Map<
        Entries<'v>,
        fn(Result<(ValueRef<'v>, ValueRef<'v>), DecodeError>) -> Result<(Self, Self), DecodeError>,
    >;

    fn scalar<W: Write + ?Sized>(self, out: &mut W) -> Result<bool, WriteFault> {
        match self.major() {
            BYTES => {
                let bytes = self.bytes()?.expect("validated byte-string major type");
                head::write_head(out, BYTES, bytes.len() as u64)?;
                out.write_all(&bytes)?;
            }
            TEXT => {
                let text = self.text()?.expect("validated text major type");
                head::write_head(out, TEXT, text.len() as u64)?;
                out.write_all(text.as_bytes())?;
            }
            0 | 1 | SIMPLE => {
                return write_scalar(&self.decode()?, out).map_err(WriteFault::Io);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
    fn excluded(self, excluded: &[&str]) -> Result<bool, DecodeError> {
        Ok(BorrowedItem::text(self)?.is_some_and(|text| excluded.contains(&text.as_ref())))
    }
    fn tag(self) -> Option<(u64, Self)> {
        BorrowedItem::tag(self)
    }
    fn array(self) -> Result<Option<(usize, Self::Values)>, DecodeError> {
        Ok(ValueRef::Encoded(self).array()?.map(|items| {
            (
                items.len(),
                items.into_iter().map(encoded_value as fn(_) -> _),
            )
        }))
    }
    fn map(self) -> Result<Option<(usize, Self::Entries)>, DecodeError> {
        Ok(ValueRef::Encoded(self).map()?.map(|entries| {
            (
                entries.len(),
                entries.into_iter().map(encoded_entry as fn(_) -> _),
            )
        }))
    }
    fn enter_array(items: Self::Values, steps: &mut Vec<Step<Self>>) -> Result<(), DecodeError> {
        let items: Vec<_> = items.collect::<Result<_, _>>()?;
        steps.extend(items.into_iter().rev().map(Step::Enter));
        Ok(())
    }
    fn enter_map(entries: Self::Entries, steps: &mut Vec<Step<Self>>) -> Result<(), DecodeError> {
        let entries: Vec<_> = entries.collect::<Result<_, _>>()?;
        for (key, value) in entries.into_iter().rev() {
            steps.push(Step::Enter(value));
            steps.push(Step::Enter(key));
        }
        Ok(())
    }
}

fn encoded_value(
    value: Result<ValueRef<'_>, DecodeError>,
) -> Result<BorrowedItem<'_>, DecodeError> {
    match value? {
        ValueRef::Encoded(value) => Ok(value),
        ValueRef::Owned(_) => unreachable!("an encoded iterator retains encoded children"),
    }
}
fn encoded_entry<'a>(
    entry: Result<(ValueRef<'a>, ValueRef<'a>), DecodeError>,
) -> Result<(BorrowedItem<'a>, BorrowedItem<'a>), DecodeError> {
    let (key, value) = entry?;
    Ok((encoded_value(Ok(key))?, encoded_value(Ok(value))?))
}

enum Job<I: Encodable> {
    Value(I),
    Array(I::Values),
    Map(I::Entries),
    Encoded(Vec<u8>),
}
enum Step<I> {
    Enter(I),
    Exit(I),
}

fn canonical_key<I: Encodable>(key: I) -> Result<Vec<u8>, WriteFault> {
    let mut bytes = Vec::new();
    if !key.scalar(&mut bytes)? {
        bytes = canonical_bottom_up(key)?;
    }
    Ok(bytes)
}

fn sorted_entries<I: Encodable>(
    entries: I::Entries,
    excluded: &[&str],
) -> Result<Vec<(Vec<u8>, I)>, WriteFault> {
    let mut keyed = Vec::new();
    for entry in entries {
        let (key, value) = entry?;
        if !excluded.is_empty() && key.excluded(excluded)? {
            continue;
        }
        keyed.push((canonical_key(key)?, value));
    }
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(keyed)
}

fn run<I: Encodable, W: Write + ?Sized>(
    mut jobs: Vec<Job<I>>,
    out: &mut W,
    canonical: bool,
) -> Result<(), WriteFault> {
    while let Some(job) = jobs.pop() {
        let value = match job {
            Job::Encoded(bytes) => {
                out.write_all(&bytes)?;
                continue;
            }
            Job::Value(value) => value,
            Job::Array(mut items) => {
                let Some(value) = items.next().transpose()? else {
                    continue;
                };
                if items.size_hint().1 != Some(0) {
                    jobs.push(Job::Array(items));
                }
                value
            }
            Job::Map(mut entries) => {
                let Some((key, value)) = entries.next().transpose()? else {
                    continue;
                };
                jobs.push(Job::Map(entries));
                jobs.push(Job::Value(value));
                key
            }
        };
        if value.scalar(out)? {
            continue;
        }
        if let Some((tag, item)) = value.tag() {
            head::write_head(out, TAG, tag)?;
            jobs.push(Job::Value(item));
        } else if let Some((len, items)) = value.array()? {
            head::write_head(out, ARRAY, len as u64)?;
            if len != 0 {
                jobs.push(Job::Array(items));
            }
        } else if let Some((len, entries)) = value.map()? {
            head::write_head(out, MAP, len as u64)?;
            if canonical {
                for (key, value) in sorted_entries::<I>(entries, &[])?.into_iter().rev() {
                    jobs.push(Job::Value(value));
                    jobs.push(Job::Encoded(key));
                }
            } else if len != 0 {
                jobs.push(Job::Map(entries));
            }
        } else {
            unreachable!("every non-scalar is a tag, array or map");
        }
    }
    Ok(())
}

fn canonical_bottom_up<I: Encodable>(root: I) -> Result<Vec<u8>, WriteFault> {
    let mut steps = vec![Step::Enter(root)];
    let mut done: Vec<Vec<u8>> = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            Step::Enter(value) => {
                let mut bytes = Vec::new();
                if value.scalar(&mut bytes)? {
                    done.push(bytes);
                    continue;
                }
                steps.push(Step::Exit(value));
                if let Some((_, item)) = value.tag() {
                    steps.push(Step::Enter(item));
                } else if let Some((_, items)) = value.array()? {
                    I::enter_array(items, &mut steps)?;
                } else if let Some((_, entries)) = value.map()? {
                    I::enter_map(entries, &mut steps)?;
                } else {
                    unreachable!("every non-scalar is a container");
                }
            }
            Step::Exit(value) => {
                let mut bytes = Vec::new();
                if let Some((tag, _)) = value.tag() {
                    let item = done.pop().expect("a tag's item is complete");
                    head::push_head(&mut bytes, TAG, tag);
                    bytes.extend_from_slice(&item);
                } else if let Some((len, _)) = value.array()? {
                    head::push_head(&mut bytes, ARRAY, len as u64);
                    let first = done.len() - len;
                    for item in done.drain(first..) {
                        bytes.extend_from_slice(&item);
                    }
                } else if let Some((len, _)) = value.map()? {
                    head::push_head(&mut bytes, MAP, len as u64);
                    let first = done.len() - 2 * len;
                    let mut children = done.drain(first..);
                    let mut pairs = Vec::with_capacity(len);
                    while let Some(key) = children.next() {
                        pairs.push((key, children.next().expect("an entry has a value")));
                    }
                    drop(children);
                    pairs.sort_by(|a, b| a.0.cmp(&b.0));
                    for (key, value) in pairs {
                        bytes.extend_from_slice(&key);
                        bytes.extend_from_slice(&value);
                    }
                } else {
                    unreachable!("only a container is exited");
                }
                done.push(bytes);
            }
        }
    }
    Ok(done.pop().expect("the root's encoding is assembled last"))
}

fn owned_io(result: Result<(), WriteFault>) -> io::Result<()> {
    match result {
        Ok(()) => Ok(()),
        Err(WriteFault::Io(error)) => Err(error),
        Err(WriteFault::Decode(error)) => {
            unreachable!("an owned CBOR traversal cannot decode source bytes: {error}")
        }
    }
}

fn write_map<I: Encodable, W: Write + ?Sized>(
    entries: I::Entries,
    excluded: &[&str],
    out: &mut W,
) -> Result<(), WriteFault> {
    let sorted = sorted_entries::<I>(entries, excluded)?;
    head::write_head(out, MAP, sorted.len() as u64)?;
    let mut jobs = Vec::with_capacity(sorted.len() * 2);
    for (key, value) in sorted.into_iter().rev() {
        jobs.push(Job::Value(value));
        jobs.push(Job::Encoded(key));
    }
    run(jobs, out, true)
}

/// Write a source-backed map, excluding exactly the named text keys.
///
/// # Errors
/// Requires a map and preserves the original writer or CBOR inspection failure.
pub fn write_canonical_borrowed_map<W: Write + ?Sized>(
    value: BorrowedItem<'_>,
    excluded: &[&str],
    out: &mut W,
) -> Result<(), WriteError> {
    let (_, entries) =
        <BorrowedItem<'_> as Encodable>::map(value)?.ok_or(WriteError::ExpectedMap)?;
    write_map::<BorrowedItem<'_>, W>(entries, excluded, out).map_err(WriteError::from)
}

/// Write `value` to `out` as-is: shortest heads, shortest exact floats,
/// definite lengths, map entries in the order the value holds them.
///
/// # Errors
///
/// The writer's error.
pub fn write<W: Write + ?Sized>(value: &Value, out: &mut W) -> io::Result<()> {
    owned_io(run(vec![Job::Value(value)], out, false))
}

/// `value` encoded as-is ([`write`](fn@write)).
pub fn encode(value: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    encode_into(value, &mut out);
    out
}

/// Append `value`, encoded as-is ([`write`](fn@write)), to `out`.
pub fn encode_into(value: &Value, out: &mut Vec<u8>) {
    write(value, out).expect("writing to a Vec cannot fail");
}

/// Write `value` to `out` in RFC 8949 §4.2.1 core deterministic encoding:
/// [`write`](fn@write)'s rules, and every map's entries in the bytewise order of their
/// keys' deterministic encodings (a stable sort).
///
/// # Errors
///
/// The writer's error.
pub fn write_canonical<W: Write + ?Sized>(value: &Value, out: &mut W) -> io::Result<()> {
    owned_io(run(vec![Job::Value(value)], out, true))
}

/// `value` in core deterministic encoding ([`write_canonical`]).
pub fn canonical(value: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    canonical_into(value, &mut out);
    out
}

/// Append `value`, in core deterministic encoding, to `out`.
pub fn canonical_into(value: &Value, out: &mut Vec<u8>) {
    write_canonical(value, out).expect("writing to a Vec cannot fail");
}

/// Write the map of `entries` to `out` in core deterministic encoding,
/// leaving out every entry whose key is a text string named in `excluded`.
///
/// This is the preimage of a content identifier computed over a map minus its
/// own identifier fields, streamed straight into the hasher without building
/// the reduced map.
///
/// # Errors
///
/// The writer's error.
pub fn write_canonical_map<W: Write + ?Sized>(
    entries: &[(Value, Value)],
    excluded: &[&str],
    out: &mut W,
) -> io::Result<()> {
    owned_io(write_map::<&Value, W>(
        entries.iter().map(owned_entry),
        excluded,
        out,
    ))
}

/// How many bytes [`encode`] (equally, [`canonical`]) writes for `value`.
pub fn encoded_len(value: &Value) -> usize {
    struct Count(usize);
    impl Write for Count {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0 += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut count = Count(0);
    write(value, &mut count).expect("counting cannot fail");
    count.0
}

#[cfg(test)]
pub(super) fn bottom_up(value: &Value) -> Vec<u8> {
    canonical_bottom_up(value).expect("an owned key encoder writes only to a Vec")
}
