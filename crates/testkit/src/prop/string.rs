// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Strings matching a regular expression.
//!
//! The pattern is parsed by `regex-syntax` — the parser behind the `regex`
//! crate — into its high-level IR, and generation walks that IR, so a pattern
//! means here exactly what it means to `regex`: the same Unicode classes, the
//! same case folding, the same escapes. Every generated string is a full match
//! of the pattern. The shapes are:
//!
//! * a literal is emitted as is;
//! * a character class draws a character uniformly from the class, shrinking
//!   to the class's lowest character;
//! * an alternation draws a branch uniformly, shrinking to the first;
//! * a repetition draws a count uniformly between its bounds, shrinking to
//!   the minimum; an unbounded repetition (`*`, `+`, `{n,}`) is capped at 32
//!   repetitions beyond its minimum, or at 32 in all when the minimum is 0 or
//!   1;
//! * the anchors `^`, `$`, `\A` and `\z` generate nothing — every string is
//!   already a full match;
//! * a word boundary (`\b`, `\B` and their variants) cannot be generated
//!   character by character and is refused when the pattern is parsed, as is
//!   a byte-oriented class or literal that is not valid UTF-8.

use std::fmt;
use std::sync::Arc;

use regex_syntax::hir::{Class, Hir, HirKind, Look};

use super::choices::{Choices, Invalid};
use super::collection::{SizeRange, Sizer};
use super::strategy::Strategy;

/// The extra repetitions an unbounded repetition may generate.
const UNBOUNDED_EXTRA: u32 = 32;

/// A pattern `regex-syntax` refused, or one this generator cannot honour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegexError {
    pattern: String,
    message: String,
}

impl fmt::Display for RegexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cannot generate strings for /{}/: {}",
            self.pattern, self.message
        )
    }
}

impl std::error::Error for RegexError {}

#[derive(Debug)]
enum Node {
    Empty,
    Literal(String),
    /// Inclusive scalar-value ranges, sorted and disjoint, and their total
    /// number of characters.
    Class(Vec<(u32, u32)>, u64),
    Repeat(Box<Self>, SizeRange),
    Concat(Vec<Self>),
    Alternation(Vec<Self>),
}

/// Strings matching one pattern; see the module documentation.
#[derive(Clone)]
pub struct RegexStrategy {
    pattern: Arc<str>,
    root: Arc<Node>,
}

impl fmt::Debug for RegexStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RegexStrategy").field(&self.pattern).finish()
    }
}

impl RegexStrategy {
    /// The pattern the strategy generates matches of.
    pub fn pattern(&self) -> &str {
        &self.pattern
    }
}

/// Strings fully matching `pattern`, or the reason it cannot be generated.
pub fn string_regex(pattern: &str) -> Result<RegexStrategy, RegexError> {
    let error = |message: String| RegexError {
        pattern: pattern.to_owned(),
        message,
    };
    let hir = regex_syntax::Parser::new()
        .parse(pattern)
        .map_err(|parse| error(parse.to_string()))?;
    let root = compile(&hir).map_err(error)?;
    Ok(RegexStrategy {
        pattern: pattern.into(),
        root: Arc::new(root),
    })
}

/// Strings fully matching `pattern`. A pattern that cannot be generated is a
/// defect in the test that names it, so it panics with the reason.
pub fn regex(pattern: &str) -> RegexStrategy {
    string_regex(pattern).unwrap_or_else(|error| panic!("{error}"))
}

fn compile(hir: &Hir) -> Result<Node, String> {
    Ok(match hir.kind() {
        HirKind::Empty => Node::Empty,
        HirKind::Literal(literal) => Node::Literal(
            std::str::from_utf8(&literal.0)
                .map_err(|_| format!("the literal {:?} is not UTF-8", literal.0))?
                .to_owned(),
        ),
        HirKind::Class(Class::Unicode(class)) => class_node(
            class
                .ranges()
                .iter()
                .map(|range| (u32::from(range.start()), u32::from(range.end()))),
        )?,
        HirKind::Class(Class::Bytes(class)) => {
            if class.ranges().iter().any(|range| range.end() > 0x7f) {
                return Err("a byte class reaches past ASCII, so its bytes are not UTF-8".into());
            }
            class_node(
                class
                    .ranges()
                    .iter()
                    .map(|range| (u32::from(range.start()), u32::from(range.end()))),
            )?
        }
        HirKind::Look(look) => match look {
            Look::Start
            | Look::End
            | Look::StartLF
            | Look::EndLF
            | Look::StartCRLF
            | Look::EndCRLF => Node::Empty,
            other => return Err(format!("the assertion {other:?} cannot be generated")),
        },
        HirKind::Repetition(repetition) => {
            let max = repetition.max.unwrap_or_else(|| {
                if repetition.min <= 1 {
                    UNBOUNDED_EXTRA
                } else {
                    repetition.min.saturating_add(UNBOUNDED_EXTRA)
                }
            });
            Node::Repeat(
                Box::new(compile(&repetition.sub)?),
                SizeRange::new(repetition.min as usize, max as usize),
            )
        }
        HirKind::Capture(capture) => compile(&capture.sub)?,
        HirKind::Concat(parts) => {
            Node::Concat(parts.iter().map(compile).collect::<Result<_, _>>()?)
        }
        HirKind::Alternation(branches) => {
            Node::Alternation(branches.iter().map(compile).collect::<Result<_, _>>()?)
        }
    })
}

fn class_node(ranges: impl Iterator<Item = (u32, u32)>) -> Result<Node, String> {
    let ranges: Vec<(u32, u32)> = ranges.collect();
    let total: u64 = ranges
        .iter()
        .map(|&(start, end)| u64::from(end - start + 1) - surrogates_in(start, end))
        .sum();
    if total == 0 {
        return Err("a character class matches nothing".into());
    }
    Ok(Node::Class(ranges, total))
}

/// The surrogate code points inside `start..=end`, which no `char` can hold.
fn surrogates_in(start: u32, end: u32) -> u64 {
    let low = start.max(0xD800);
    let high = end.min(0xDFFF);
    if low > high {
        0
    } else {
        u64::from(high - low + 1)
    }
}

fn generate(node: &Node, choices: &mut Choices, out: &mut String) -> Result<(), Invalid> {
    match node {
        Node::Empty => {}
        Node::Literal(text) => out.push_str(text),
        Node::Class(ranges, total) => {
            let mut index = choices.draw(total - 1)?;
            for &(start, end) in ranges {
                let size = u64::from(end - start + 1) - surrogates_in(start, end);
                if index < size {
                    let mut code = start + index as u32;
                    if start <= 0xD7FF && code >= 0xD800 {
                        code += 0x800;
                    }
                    out.push(char::from_u32(code).expect("the index skips the surrogates"));
                    return Ok(());
                }
                index -= size;
            }
            unreachable!("the index is below the class's total");
        }
        Node::Repeat(sub, size) => {
            let sizer = Sizer::start(choices, *size);
            let mut count = 0;
            while sizer.more(choices, count, false)? {
                generate(sub, choices, out)?;
                count += 1;
            }
        }
        Node::Concat(parts) => {
            for part in parts {
                generate(part, choices, out)?;
            }
        }
        Node::Alternation(branches) => {
            let index = choices.draw((branches.len() - 1) as u64)? as usize;
            generate(&branches[index], choices, out)?;
        }
    }
    Ok(())
}

impl Strategy for RegexStrategy {
    type Value = String;

    fn generate(&self, choices: &mut Choices) -> Result<String, Invalid> {
        let mut out = String::new();
        generate(&self.root, choices, &mut out)?;
        Ok(out)
    }
}
