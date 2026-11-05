// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The deterministic JSON writer, over a heap work list.

use super::Value;
use crate::allocation::{Admission, Memory, Resident, StorageError};
use crate::json_escape::{JsonEscapes, try_escape_body};

/// Original storage refusal or an independent serialized-byte ceiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteError {
    /// The next append would exceed this logical UTF-8 byte ceiling.
    Bytes {
        /// Maximum total bytes in the destination.
        limit: u64,
    },
    /// Original checked-layout, allocator or admission refusal.
    Storage(StorageError),
}
impl core::fmt::Display for WriteError {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Bytes { limit } => write!(out, "JSON output exceeds {limit} bytes"),
            Self::Storage(cause) => cause.fmt(out),
        }
    }
}
purrdf_hash::error_sources!(WriteError, Storage);
impl purrdf_hash::error::StructuredError for WriteError {
    purrdf_hash::error_visit! {
        Bytes { limit } => "json-output-bytes" { "limit" => ByteCount(u128::from(*limit)) },
        Storage(cause) => "json-output-storage" { "cause" => Cause(cause) },
    }
}
crate::variant_from!(WriteError { Storage(StorageError) });

fn append<S: Admission + ?Sized>(
    out: &mut String,
    piece: &str,
    maximum: Option<u64>,
    memory: &mut Memory<'_, S>,
) -> Result<(), WriteError> {
    if let Some(limit) = maximum {
        let length = out
            .len()
            .checked_add(piece.len())
            .ok_or(StorageError::SizeOverflow)?;
        if u64::try_from(length).map_or(true, |length| length > limit) {
            return Err(WriteError::Bytes { limit });
        }
    }
    memory.push_str(out, piece).map_err(Into::into)
}
fn character<S: Admission + ?Sized>(
    out: &mut String,
    ch: char,
    maximum: Option<u64>,
    memory: &mut Memory<'_, S>,
) -> Result<(), WriteError> {
    append(out, ch.encode_utf8(&mut [0; 4]), maximum, memory)
}
fn string<S: Admission + ?Sized>(
    out: &mut String,
    text: &str,
    escapes: JsonEscapes,
    maximum: Option<u64>,
    memory: &mut Memory<'_, S>,
) -> Result<(), WriteError> {
    append(out, "\"", maximum, memory)?;
    try_escape_body(text, escapes, |piece| append(out, piece, maximum, memory))?;
    append(out, "\"", maximum, memory)
}

/// How the writer lays a value out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Format<'a> {
    /// `None` writes compact JSON with no insignificant whitespace. `Some(unit)`
    /// writes one member or element per line, each nesting level indented by
    /// `unit`, `": "` after a member name, and an empty array or object as
    /// `[]` or `{}`: `serde_json`'s pretty layout, which [`Format::PRETTY`]
    /// states with a two-space unit.
    pub indent: Option<&'a str>,
    /// The string spelling ([`crate::json_escape`]).
    pub escapes: JsonEscapes,
}

impl Format<'static> {
    /// Compact, with [`JsonEscapes::ShortForms`] strings: `serde_json::to_string`,
    /// byte for byte, for a value whose members are in the order it wrote them.
    pub const COMPACT: Self = Self {
        indent: None,
        escapes: JsonEscapes::ShortForms,
    };

    /// Two-space pretty, with [`JsonEscapes::ShortForms`] strings:
    /// `serde_json::to_string_pretty`'s layout.
    pub const PRETTY: Self = Self {
        indent: Some("  "),
        escapes: JsonEscapes::ShortForms,
    };
}

/// A container being written: the members not yet written, and whether one
/// already has been (so the next needs a `,`).
enum Frame<'v> {
    /// An array's remaining elements.
    Array(core::slice::Iter<'v, Value>, bool),
    /// An object's remaining members.
    Object(core::slice::Iter<'v, (String, Value)>, bool),
}

/// The line break and indentation of a nesting depth (pretty only).
fn line_break<S: Admission + ?Sized>(
    out: &mut String,
    unit: &str,
    depth: usize,
    maximum: Option<u64>,
    memory: &mut Memory<'_, S>,
) -> Result<(), WriteError> {
    append(out, "\n", maximum, memory)?;
    for _ in 0..depth {
        append(out, unit, maximum, memory)?;
    }
    Ok(())
}

/// Write a scalar, or open its original container onto the admitted work list.
#[inline]
fn write_value<'v, S: Admission + ?Sized>(
    out: &mut String,
    value: &'v Value,
    escapes: JsonEscapes,
    maximum: Option<u64>,
    frames: &mut Vec<Frame<'v>>,
    memory: &mut Memory<'_, S>,
) -> Result<(), WriteError> {
    match value {
        Value::Null => append(out, "null", maximum, memory),
        Value::Bool(true) => append(out, "true", maximum, memory),
        Value::Bool(false) => append(out, "false", maximum, memory),
        Value::Number(number) => append(out, number.lexeme(), maximum, memory),
        Value::String(text) => string(out, text, escapes, maximum, memory),
        Value::Array(items) if items.is_empty() => append(out, "[]", maximum, memory),
        Value::Object(object) if object.is_empty() => append(out, "{}", maximum, memory),
        Value::Array(items) => {
            append(out, "[", maximum, memory)?;
            memory
                .push(frames, Frame::Array(items.iter(), false))
                .map_err(Into::into)
        }
        Value::Object(object) => {
            append(out, "{", maximum, memory)?;
            memory
                .push(frames, Frame::Object(object.members().iter(), false))
                .map_err(Into::into)
        }
    }
}

/// Append `value` to `out`, laid out per `format`.
///
/// Members remain in their original order and numbers keep their lexemes.
/// Open containers use the same native work list as the admitted writer.
pub fn write_into(out: &mut String, value: &Value, format: Format<'_>) {
    let mut resident = Resident;
    let mut memory = Memory::resume(&mut resident, out.capacity());
    write_into_with_memory(out, value, format, &mut memory)
        .expect("resident JSON writer allocation");
}

/// Append through the original physical account. Partial output retains its
/// original grant on refusal; only destroyed writer scratch is released.
///
/// # Errors
///
/// Returns checked layout, allocator or original admission refusal.
pub fn write_into_with_memory<S: Admission + ?Sized>(
    out: &mut String,
    value: &Value,
    format: Format<'_>,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    write_body(out, value, format, None, memory).map_err(|error| match error {
        WriteError::Storage(cause) => cause,
        // No byte ceiling is installed, so append cannot construct this arm.
        WriteError::Bytes { .. } => unreachable!("unbounded writer has no byte refusal"),
    })
}

/// Append with a separate logical ceiling on the destination's total UTF-8 bytes.
/// Each append is refused before growth; partial output keeps its original grant.
/// # Errors
/// Returns the byte refusal or original native storage failure.
pub fn write_into_bounded_with_memory<S: Admission + ?Sized>(
    out: &mut String,
    value: &Value,
    format: Format<'_>,
    max_bytes: u64,
    memory: &mut Memory<'_, S>,
) -> Result<(), WriteError> {
    write_body(out, value, format, Some(max_bytes), memory)
}

fn write_body<S: Admission + ?Sized>(
    out: &mut String,
    value: &Value,
    format: Format<'_>,
    maximum: Option<u64>,
    memory: &mut Memory<'_, S>,
) -> Result<(), WriteError> {
    let baseline = memory.admitted_bytes();
    let output_capacity = out.capacity();
    let result = (|| {
        let separator = if format.indent.is_some() { ": " } else { ":" };
        let escapes = format.escapes;
        let mut frames: Vec<Frame<'_>> = Vec::new();
        write_value(out, value, escapes, maximum, &mut frames, memory)?;
        loop {
            let depth = frames.len();
            let Some(top) = frames.last_mut() else { break };
            let (next, comma, close) = match top {
                Frame::Array(items, started) => (
                    items.next().map(|item| (None, item)),
                    core::mem::replace(started, true),
                    ']',
                ),
                Frame::Object(members, started) => (
                    members.next().map(|(name, member)| (Some(name), member)),
                    core::mem::replace(started, true),
                    '}',
                ),
            };
            let Some((name, item)) = next else {
                frames.pop();
                if let Some(unit) = format.indent {
                    line_break(out, unit, depth - 1, maximum, memory)?;
                }
                character(out, close, maximum, memory)?;
                continue;
            };
            if comma {
                append(out, ",", maximum, memory)?;
            }
            if let Some(unit) = format.indent {
                line_break(out, unit, depth, maximum, memory)?;
            }
            if let Some(name) = name {
                string(out, name, escapes, maximum, memory)?;
                append(out, separator, maximum, memory)?;
            }
            write_value(out, item, escapes, maximum, &mut frames, memory)?;
        }
        memory.release_vec(frames).map_err(WriteError::from)
    })();
    if result.is_err() {
        // The closure's original frame array is already destroyed; out survives.
        let retained = baseline.checked_add(out.capacity() - output_capacity);
        if let Some(released) =
            retained.and_then(|retained| memory.admitted_bytes().checked_sub(retained))
        {
            let _ = memory.release_bytes(released);
        }
    }
    result
}

/// Write the same value into a new, ownership-carrying admitted destination.
///
/// # Errors
///
/// Returns the original physical refusal; failed output dies before refund.
pub fn write_with_memory<S: Admission + ?Sized>(
    value: &Value,
    format: Format<'_>,
    memory: &mut Memory<'_, S>,
) -> Result<String, StorageError> {
    memory.scope(|memory| {
        let mut out = String::new();
        write_into_with_memory(&mut out, value, format, memory)?;
        Ok(out)
    })
}

/// Construct JSON under independent serialized-byte and original storage authority.
/// Failed output is destroyed before its original grant is refunded.
/// # Errors
/// Returns the byte refusal or original native storage failure.
pub fn write_bounded_with_memory<S: Admission + ?Sized>(
    value: &Value,
    format: Format<'_>,
    max_bytes: u64,
    memory: &mut Memory<'_, S>,
) -> Result<String, WriteError> {
    memory.try_scope(|memory| {
        let mut out = String::new();
        write_into_bounded_with_memory(&mut out, value, format, max_bytes, memory)?;
        Ok(out)
    })
}

pub(super) const fn frame_bytes() -> usize {
    size_of::<Frame<'static>>()
}

/// `value` laid out per `format`.
pub fn write(value: &Value, format: Format<'_>) -> String {
    let mut out = String::new();
    write_into(&mut out, value, format);
    out
}

/// `value` as compact JSON ([`Format::COMPACT`]).
pub fn write_compact(value: &Value) -> String {
    write(value, Format::COMPACT)
}

/// `value` as two-space pretty JSON ([`Format::PRETTY`]), with no trailing
/// line break.
pub fn write_pretty(value: &Value) -> String {
    write(value, Format::PRETTY)
}
