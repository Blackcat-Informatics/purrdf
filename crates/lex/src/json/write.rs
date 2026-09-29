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

/// One step of the writer.
enum Job<'v> {
    /// A value at a nesting depth.
    Value(&'v Value, usize),
    /// A member name, its separator, then its value.
    Member(&'v str, &'v Value, usize),
    /// Fixed text.
    Text(&'static str),
    /// A line break and the indentation of a depth (pretty only).
    Break(usize),
}

/// Append `value` to `out`, laid out per `format`.
///
/// The output is a pure function of the value and the format: members in the
/// object's order, a number as its lexeme, a string in one spelling. A
/// container's parts go onto a heap work list in reverse, so they pop in
/// document order and nesting costs heap, never stack.
pub fn write_into(out: &mut String, value: &Value, format: Format<'_>) {
    let pretty = format.indent.is_some();
    let separator = if pretty { ": " } else { ":" };
    let mut jobs = vec![Job::Value(value, 0)];
    while let Some(job) = jobs.pop() {
        match job {
            Job::Text(text) => out.push_str(text),
            Job::Break(depth) => {
                out.push('\n');
                if let Some(unit) = format.indent {
                    for _ in 0..depth {
                        out.push_str(unit);
                    }
                }
            }
            Job::Member(name, member, depth) => {
                push_string(out, name, format.escapes);
                out.push_str(separator);
                jobs.push(Job::Value(member, depth));
            }
            Job::Value(value, depth) => match value {
                Value::Null => out.push_str("null"),
                Value::Bool(true) => out.push_str("true"),
                Value::Bool(false) => out.push_str("false"),
                Value::Number(number) => out.push_str(number.lexeme()),
                Value::String(text) => push_string(out, text, format.escapes),
                Value::Array(items) if items.is_empty() => out.push_str("[]"),
                Value::Object(object) if object.is_empty() => out.push_str("{}"),
                Value::Array(items) => {
                    out.push('[');
                    jobs.push(Job::Text("]"));
                    if pretty {
                        jobs.push(Job::Break(depth));
                    }
                    for (index, item) in items.iter().enumerate().rev() {
                        jobs.push(Job::Value(item, depth + 1));
                        if pretty {
                            jobs.push(Job::Break(depth + 1));
                        }
                        if index > 0 {
                            jobs.push(Job::Text(","));
                        }
                    }
                }
                Value::Object(object) => {
                    out.push('{');
                    jobs.push(Job::Text("}"));
                    if pretty {
                        jobs.push(Job::Break(depth));
                    }
                    for (index, (name, member)) in object.iter().enumerate().rev() {
                        jobs.push(Job::Member(name, member, depth + 1));
                        if pretty {
                            jobs.push(Job::Break(depth + 1));
                        }
                        if index > 0 {
                            jobs.push(Job::Text(","));
                        }
                    }
                }
            },
        }
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
