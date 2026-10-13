// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The deterministic JSON writer, over a heap work list.

use super::Value;
use crate::allocation::{Admission, Memory, Resident, StorageError};
use crate::json_escape::{JsonEscapes, push_string_with_memory};

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
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    memory.push_char(out, '\n')?;
    for _ in 0..depth {
        memory.push_str(out, unit)?;
    }
    Ok(())
}

/// Write a scalar, or open its original container onto the admitted work list.
#[inline]
fn write_value<'v, S: Admission + ?Sized>(
    out: &mut String,
    value: &'v Value,
    escapes: JsonEscapes,
    frames: &mut Vec<Frame<'v>>,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    match value {
        Value::Null => memory.push_str(out, "null"),
        Value::Bool(true) => memory.push_str(out, "true"),
        Value::Bool(false) => memory.push_str(out, "false"),
        Value::Number(number) => memory.push_str(out, number.lexeme()),
        Value::String(text) => push_string_with_memory(out, text, escapes, memory),
        Value::Array(items) if items.is_empty() => memory.push_str(out, "[]"),
        Value::Object(object) if object.is_empty() => memory.push_str(out, "{}"),
        Value::Array(items) => {
            memory.push_char(out, '[')?;
            memory.push(frames, Frame::Array(items.iter(), false))
        }
        Value::Object(object) => {
            memory.push_char(out, '{')?;
            memory.push(frames, Frame::Object(object.members().iter(), false))
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
    let baseline = memory.admitted_bytes();
    let output_capacity = out.capacity();
    let result = (|| {
        let separator = if format.indent.is_some() { ": " } else { ":" };
        let escapes = format.escapes;
        let mut frames: Vec<Frame<'_>> = Vec::new();
        write_value(out, value, escapes, &mut frames, memory)?;
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
                    line_break(out, unit, depth - 1, memory)?;
                }
                memory.push_char(out, close)?;
                continue;
            };
            if comma {
                memory.push_char(out, ',')?;
            }
            if let Some(unit) = format.indent {
                line_break(out, unit, depth, memory)?;
            }
            if let Some(name) = name {
                push_string_with_memory(out, name, escapes, memory)?;
                memory.push_str(out, separator)?;
            }
            write_value(out, item, escapes, &mut frames, memory)?;
        }
        memory.release_vec(frames)
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
