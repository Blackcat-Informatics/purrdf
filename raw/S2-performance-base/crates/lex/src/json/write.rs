// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The deterministic JSON writer, over a heap work list.

use super::Value;
use crate::json_escape::{JsonEscapes, push_string};

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
fn line_break(out: &mut String, unit: &str, depth: usize) {
    out.push('\n');
    for _ in 0..depth {
        out.push_str(unit);
    }
}

/// Write a scalar, or open a container onto `frames` (an empty one is written
/// whole).
#[inline]
fn write_value<'v>(
    out: &mut String,
    value: &'v Value,
    escapes: JsonEscapes,
    frames: &mut Vec<Frame<'v>>,
) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(number) => out.push_str(number.lexeme()),
        Value::String(text) => push_string(out, text, escapes),
        Value::Array(items) if items.is_empty() => out.push_str("[]"),
        Value::Object(object) if object.is_empty() => out.push_str("{}"),
        Value::Array(items) => {
            out.push('[');
            frames.push(Frame::Array(items.iter(), false));
        }
        Value::Object(object) => {
            out.push('{');
            frames.push(Frame::Object(object.members().iter(), false));
        }
    }
}

/// Append `value` to `out`, laid out per `format`.
///
/// The output is a pure function of the value and the format: members in the
/// object's order, a number as its lexeme, a string in one spelling. Each open
/// container is one frame on a heap stack holding its remaining members, so a
/// scalar is written where it stands and nesting costs heap, never stack.
pub fn write_into(out: &mut String, value: &Value, format: Format<'_>) {
    let separator = if format.indent.is_some() { ": " } else { ":" };
    let escapes = format.escapes;
    let mut frames: Vec<Frame<'_>> = Vec::new();
    write_value(out, value, escapes, &mut frames);
    loop {
        let depth = frames.len();
        let Some(top) = frames.last_mut() else {
            break;
        };
        // The next member of the innermost container, and whether one came
        // before it; `None` when the container is spent.
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
                line_break(out, unit, depth - 1);
            }
            out.push(close);
            continue;
        };
        if comma {
            out.push(',');
        }
        if let Some(unit) = format.indent {
            line_break(out, unit, depth);
        }
        if let Some(name) = name {
            push_string(out, name, escapes);
            out.push_str(separator);
        }
        write_value(out, item, escapes, &mut frames);
    }
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
