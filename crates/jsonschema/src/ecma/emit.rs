// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Writing a parsed ECMA-262 pattern in `regex` crate syntax.
//!
//! Every literal is written as `\x{…}`, so no character of the source can be
//! re-read by `regex` as syntax, and every class escape is written as the
//! ECMA-262 set it denotes rather than as `regex`'s escape of the same name.

use std::fmt::Write as _;

use super::{Ast, Class, ClassItem, PatternError, Property};

/// ECMA-262 `LineTerminator` (§12.3): LF, CR, LS, PS.
const LINE_TERMINATORS: &[(u32, u32)] = &[(0x0A, 0x0A), (0x0D, 0x0D), (0x2028, 0x2029)];

/// ECMA-262 `\s` (§22.2.2.9 `CharacterClassEscape :: s`): `WhiteSpace` (§12.2)
/// ∪ `LineTerminator` (§12.3). `WhiteSpace` is TAB, VT, FF, ZWNBSP and every
/// `Space_Separator` (`Zs`) code point: U+0020, U+00A0, U+1680,
/// U+2000..=U+200A, U+202F, U+205F and U+3000.
pub(super) const SPACE: &[(u32, u32)] = &[
    (0x09, 0x0D),
    (0x20, 0x20),
    (0xA0, 0xA0),
    (0x1680, 0x1680),
    (0x2000, 0x200A),
    (0x2028, 0x2029),
    (0x202F, 0x202F),
    (0x205F, 0x205F),
    (0x3000, 0x3000),
    (0xFEFF, 0xFEFF),
];

/// ECMA-262 `\d`: `0-9`.
pub(super) const DIGIT: &[(u32, u32)] = &[(0x30, 0x39)];

/// ECMA-262 `\w` without the `i` flag (`WordCharacters`): `0-9A-Za-z_`.
pub(super) const WORD: &[(u32, u32)] = &[(0x30, 0x39), (0x41, 0x5A), (0x5F, 0x5F), (0x61, 0x7A)];

/// A class that matches no code point.
const NOTHING: &str = r"[^\x{0}-\x{10FFFF}]";
/// A class that matches every code point.
const EVERYTHING: &str = r"[\x{0}-\x{10FFFF}]";

pub(super) fn emit(ast: &Ast) -> Result<String, PatternError> {
    let mut out = String::new();
    write_ast(ast, &mut out)?;
    Ok(out)
}

fn write_ast(ast: &Ast, out: &mut String) -> Result<(), PatternError> {
    match ast {
        Ast::Empty => out.push_str("(?:)"),
        Ast::Char(code) => match char::from_u32(*code) {
            Some(_) => write_code(*code, out),
            // A lone surrogate is a UTF-16 code unit no Rust string holds.
            None => out.push_str(NOTHING),
        },
        Ast::Dot => {
            out.push_str("[^");
            write_ranges(LINE_TERMINATORS, out);
            out.push(']');
        }
        Ast::Class(class) => write_class(class, out),
        Ast::Start => out.push_str(r"\A"),
        Ast::End => out.push_str(r"\z"),
        Ast::WordBoundary(false) => out.push_str(r"(?-u:\b)"),
        Ast::WordBoundary(true) => out.push_str(r"(?-u:\B)"),
        Ast::Group(body) => {
            out.push_str("(?:");
            write_ast(body, out)?;
            out.push(')');
        }
        Ast::Look { offset, behind } => {
            return Err(PatternError::Unsupported {
                offset: *offset,
                construct: if *behind { "lookbehind" } else { "lookahead" },
            });
        }
        Ast::Backreference { offset } => {
            return Err(PatternError::Unsupported {
                offset: *offset,
                construct: "backreference",
            });
        }
        Ast::Modifiers { offset } => {
            return Err(PatternError::Unsupported {
                offset: *offset,
                construct: "modifier group",
            });
        }
        Ast::Repeat {
            body,
            min,
            max,
            greedy,
        } => {
            out.push_str("(?:");
            write_ast(body, out)?;
            out.push(')');
            match (min, max) {
                (0, None) => out.push('*'),
                (1, None) => out.push('+'),
                (0, Some(1)) => out.push('?'),
                (min, None) => {
                    let _ = write!(out, "{{{min},}}");
                }
                (min, Some(max)) if min == max => {
                    let _ = write!(out, "{{{min}}}");
                }
                (min, Some(max)) => {
                    let _ = write!(out, "{{{min},{max}}}");
                }
            }
            if !greedy {
                out.push('?');
            }
        }
        Ast::Concat(items) => {
            for item in items {
                write_ast(item, out)?;
            }
        }
        Ast::Alternation(alternatives) => {
            out.push_str("(?:");
            for (index, alternative) in alternatives.iter().enumerate() {
                if index > 0 {
                    out.push('|');
                }
                write_ast(alternative, out)?;
            }
            out.push(')');
        }
    }
    Ok(())
}

fn write_code(code: u32, out: &mut String) {
    let _ = write!(out, r"\x{{{code:X}}}");
}

/// Write `ranges` as class members, dropping the surrogate block: no Rust
/// string holds a surrogate, so a range is clipped to the scalar values it
/// covers and a range of surrogates alone contributes nothing.
fn write_ranges(ranges: &[(u32, u32)], out: &mut String) -> usize {
    let mut written = 0;
    for &(low, high) in ranges {
        for (low, high) in clip_surrogates(low, high) {
            write_code(low, out);
            if high != low {
                out.push('-');
                write_code(high, out);
            }
            written += 1;
        }
    }
    written
}

fn clip_surrogates(low: u32, high: u32) -> impl Iterator<Item = (u32, u32)> {
    let below = (low < 0xD800).then(|| (low, high.min(0xD7FF)));
    let above = (high > 0xDFFF).then(|| (low.max(0xE000), high));
    below.into_iter().chain(above)
}

fn write_class(class: &Class, out: &mut String) {
    let mut body = String::new();
    let mut members = 0;
    for item in &class.items {
        members += write_item(item, &mut body);
    }
    if members == 0 {
        out.push_str(if class.negated { EVERYTHING } else { NOTHING });
        return;
    }
    out.push('[');
    if class.negated {
        out.push('^');
    }
    out.push_str(&body);
    out.push(']');
}

/// Write one class member; returns how many members it contributed (zero for
/// a range of surrogates alone).
fn write_item(item: &ClassItem, out: &mut String) -> usize {
    let (ranges, negated): (&[(u32, u32)], bool) = match item {
        ClassItem::Range(low, high) => return write_ranges(&[(*low, *high)], out),
        ClassItem::Digit(negated) => (DIGIT, *negated),
        ClassItem::Word(negated) => (WORD, *negated),
        ClassItem::Space(negated) => (SPACE, *negated),
        ClassItem::Property(property, negated) => {
            write_property(property, *negated, out);
            return 1;
        }
    };
    if negated {
        out.push_str("[^");
        write_ranges(ranges, out);
        out.push(']');
    } else {
        write_ranges(ranges, out);
    }
    1
}

fn write_property(property: &Property, negated: bool, out: &mut String) {
    match property {
        Property::Any => out.push_str(if negated { NOTHING } else { EVERYTHING }),
        Property::Ascii => out.push_str(if negated {
            r"[^\x{0}-\x{7F}]"
        } else {
            r"[\x{0}-\x{7F}]"
        }),
        Property::Assigned => out.push_str(if negated { r"\p{gc=Cn}" } else { r"\P{gc=Cn}" }),
        Property::Named(name) => {
            let _ = write!(out, r"\{}{{{name}}}", if negated { 'P' } else { 'p' });
        }
    }
}
