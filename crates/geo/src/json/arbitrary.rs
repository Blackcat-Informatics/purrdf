// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A deterministic generator of JSON values and documents, for the tests that
//! compare an iterative walk over a value with its recursive reference.
//!
//! The generator recurses, on the shallow depths it is asked for; it is the reference
//! side of every such comparison, never the code under test.

use purrdf_iri::json_escape::{JsonEscapes, push_string};

use super::JsonValue;
use crate::geom::arbitrary::Lcg;

/// Strings carrying what both `Debug` and the JSON writer escape, and what neither
/// does.
const TEXTS: [&str; 6] = [
    "",
    "a",
    "quote \" and \\ backslash",
    "tab\t",
    "caf\u{e9}",
    "\u{1f638}",
];

/// Number lexemes of every RFC 8259 shape.
const LEXEMES: [&str; 5] = ["0", "-1", "1.5", "15e-1", "123456789012345678901234567890"];

/// A random value nesting at most `depth` levels, every kind included, with the
/// strings drawn from [`TEXTS`].
pub(crate) fn value(rng: &mut Lcg, depth: usize) -> JsonValue {
    let choice = if depth > 0 && rng.chance(2) {
        4 + rng.below(2)
    } else {
        rng.below(4)
    };
    match choice {
        0 => JsonValue::Null,
        1 => JsonValue::Bool(rng.chance(2)),
        2 => JsonValue::Number(LEXEMES[rng.below(5) as usize].to_owned()),
        3 => JsonValue::String(TEXTS[rng.below(6) as usize].to_owned()),
        4 => JsonValue::Array((0..rng.below(4)).map(|_| value(rng, depth - 1)).collect()),
        _ => JsonValue::Object(
            (0..rng.below(4))
                .map(|_| {
                    (
                        TEXTS[rng.below(6) as usize].to_owned(),
                        value(rng, depth - 1),
                    )
                })
                .collect(),
        ),
    }
}

/// One run of RFC 8259 whitespace, one time in three.
fn space(rng: &mut Lcg, out: &mut String) {
    if rng.chance(3) {
        out.push_str([" ", "\t", "\n", "\r\n", "  "][rng.below(5) as usize]);
    }
}

/// `value` as a document, with whitespace sprinkled between its tokens so a reader's
/// whitespace handling is exercised at every position.
pub(crate) fn spaced_text(rng: &mut Lcg, value: &JsonValue, out: &mut String) {
    match value {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(true) => out.push_str("true"),
        JsonValue::Bool(false) => out.push_str("false"),
        JsonValue::Number(lexeme) => out.push_str(lexeme),
        JsonValue::String(text) => push_string(out, text, JsonEscapes::ShortForms),
        JsonValue::Array(items) => {
            out.push('[');
            space(rng, out);
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    space(rng, out);
                    out.push(',');
                    space(rng, out);
                }
                spaced_text(rng, item, out);
            }
            space(rng, out);
            out.push(']');
        }
        JsonValue::Object(members) => {
            out.push('{');
            space(rng, out);
            for (index, (name, member)) in members.iter().enumerate() {
                if index > 0 {
                    space(rng, out);
                    out.push(',');
                    space(rng, out);
                }
                push_string(out, name, JsonEscapes::ShortForms);
                space(rng, out);
                out.push(':');
                space(rng, out);
                spaced_text(rng, member, out);
            }
            space(rng, out);
            out.push('}');
        }
    }
}

/// `text` with one fault at a character boundary: a piece of JSON syntax inserted,
/// a character deleted, or one replaced. The result may still be well-formed; two
/// readers must agree on that too.
pub(crate) fn faulted(rng: &mut Lcg, text: &str) -> String {
    const PIECES: [&str; 12] = [
        "[", "]", "{", "}", ",", ":", "\"", " ", "1", "x", "\\", "\u{e9}",
    ];
    let boundaries: Vec<usize> = text
        .char_indices()
        .map(|(index, _)| index)
        .chain(core::iter::once(text.len()))
        .collect();
    let at = boundaries[rng.below(boundaries.len() as u32) as usize];
    let piece = PIECES[rng.below(12) as usize];
    let (before, after) = text.split_at(at);
    let mut rest = after.chars();
    let mut out = String::with_capacity(text.len() + piece.len());
    out.push_str(before);
    match rng.below(3) {
        0 => {
            out.push_str(piece);
            out.push_str(after);
        }
        1 => {
            rest.next();
            out.push_str(rest.as_str());
        }
        _ => {
            out.push_str(piece);
            rest.next();
            out.push_str(rest.as_str());
        }
    }
    out
}
