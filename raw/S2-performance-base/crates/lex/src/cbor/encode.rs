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

/// One step of the streaming encoder.
enum Job<'v> {
    /// A value to encode.
    Value(&'v Value),
    /// Bytes already encoded: a map key's deterministic encoding.
    Encoded(Vec<u8>),
}

/// A key's deterministic encoding.
fn canonical_key(key: &Value) -> Vec<u8> {
    let mut bytes = Vec::new();
    if !write_scalar(key, &mut bytes).expect("writing to a Vec cannot fail") {
        bytes = canonical_bottom_up(key);
    }
    bytes
}

/// A map's entries in deterministic order, each with its key's encoding,
/// leaving out entries whose key is a text string in `excluded`.
fn sorted_entries<'v>(
    entries: &'v [(Value, Value)],
    excluded: &[&str],
) -> Vec<(Vec<u8>, &'v Value)> {
    let mut keyed: Vec<(Vec<u8>, &Value)> = entries
        .iter()
        .filter(|(key, _)| !key.as_text().is_some_and(|text| excluded.contains(&text)))
        .map(|(key, value)| (canonical_key(key), value))
        .collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    keyed
}

/// Stream `jobs` to `out`: every container's parts go onto the work list in
/// reverse, so they pop in order and nesting costs heap, never stack.
fn run<W: Write + ?Sized>(mut jobs: Vec<Job<'_>>, out: &mut W, canonical: bool) -> io::Result<()> {
    while let Some(job) = jobs.pop() {
        let value = match job {
            Job::Encoded(bytes) => {
                out.write_all(&bytes)?;
                continue;
            }
            Job::Value(value) => value,
        };
        if write_scalar(value, out)? {
            continue;
        }
        match value {
            Value::Tag(tag, item) => {
                head::write_head(out, TAG, *tag)?;
                jobs.push(Job::Value(item));
            }
            Value::Array(items) => {
                head::write_head(out, ARRAY, items.len() as u64)?;
                jobs.extend(items.iter().rev().map(Job::Value));
            }
            Value::Map(entries) => {
                head::write_head(out, MAP, entries.len() as u64)?;
                if canonical {
                    for (key, value) in sorted_entries(entries, &[]).into_iter().rev() {
                        jobs.push(Job::Value(value));
                        jobs.push(Job::Encoded(key));
                    }
                } else {
                    for (key, value) in entries.iter().rev() {
                        jobs.push(Job::Value(value));
                        jobs.push(Job::Value(key));
                    }
                }
            }
            _ => unreachable!("every other value is a scalar"),
        }
    }
    Ok(())
}

/// The deterministic encoding of `root`, built bottom-up: each container's
/// encoding is assembled from its children's once they are complete, so a map
/// nested inside a map KEY is sorted without a recursive call.
fn canonical_bottom_up(root: &Value) -> Vec<u8> {
    enum Step<'v> {
        Enter(&'v Value),
        Exit(&'v Value),
    }
    let mut steps = vec![Step::Enter(root)];
    let mut done: Vec<Vec<u8>> = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            Step::Enter(value) => {
                let mut bytes = Vec::new();
                if write_scalar(value, &mut bytes).expect("writing to a Vec cannot fail") {
                    done.push(bytes);
                    continue;
                }
                steps.push(Step::Exit(value));
                match value {
                    Value::Tag(_, item) => steps.push(Step::Enter(item)),
                    Value::Array(items) => steps.extend(items.iter().rev().map(Step::Enter)),
                    Value::Map(entries) => {
                        for (key, value) in entries.iter().rev() {
                            steps.push(Step::Enter(value));
                            steps.push(Step::Enter(key));
                        }
                    }
                    _ => unreachable!("every other value is a scalar"),
                }
            }
            Step::Exit(value) => {
                let mut bytes = Vec::new();
                match value {
                    Value::Tag(tag, _) => {
                        let item = done.pop().expect("a tag's item is complete");
                        head::push_head(&mut bytes, TAG, *tag);
                        bytes.extend_from_slice(&item);
                    }
                    Value::Array(items) => {
                        head::push_head(&mut bytes, ARRAY, items.len() as u64);
                        let first = done.len() - items.len();
                        for item in done.drain(first..) {
                            bytes.extend_from_slice(&item);
                        }
                    }
                    Value::Map(entries) => {
                        head::push_head(&mut bytes, MAP, entries.len() as u64);
                        let first = done.len() - 2 * entries.len();
                        let mut children = done.drain(first..);
                        let mut pairs = Vec::with_capacity(entries.len());
                        while let Some(key) = children.next() {
                            pairs.push((key, children.next().expect("an entry has a value")));
                        }
                        drop(children);
                        pairs.sort_by(|a, b| a.0.cmp(&b.0));
                        for (key, value) in pairs {
                            bytes.extend_from_slice(&key);
                            bytes.extend_from_slice(&value);
                        }
                    }
                    _ => unreachable!("only a container is exited"),
                }
                done.push(bytes);
            }
        }
    }
    done.pop().expect("the root's encoding is assembled last")
}

/// Write `value` to `out` as-is: shortest heads, shortest exact floats,
/// definite lengths, map entries in the order the value holds them.
///
/// # Errors
///
/// The writer's error.
pub fn write<W: Write + ?Sized>(value: &Value, out: &mut W) -> io::Result<()> {
    run(vec![Job::Value(value)], out, false)
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
    run(vec![Job::Value(value)], out, true)
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
    let sorted = sorted_entries(entries, excluded);
    head::write_head(out, MAP, sorted.len() as u64)?;
    let mut jobs = Vec::with_capacity(sorted.len() * 2);
    for (key, value) in sorted.into_iter().rev() {
        jobs.push(Job::Value(value));
        jobs.push(Job::Encoded(key));
    }
    run(jobs, out, true)
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
    canonical_bottom_up(value)
}
