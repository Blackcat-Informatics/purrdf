// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! ECMA-262 regular expressions, as JSON Schema's `pattern`,
//! `patternProperties` and `format: "regex"` define them. Regular patterns
//! use `regex`; lookaround, backreferences and scoped modifiers use a
//! bounded, explicit-stack matcher.
//!
//! JSON Schema 2020-12 (Validation §4.3, Core §6.4) says a pattern is an
//! ECMA-262 regular expression, and the official test suite pins the
//! `u`-flag reading: patterns match code points, `\p{…}` property escapes are
//! available, and the Annex B leniencies (`\a` as a literal `a`, a lone `]` or
//! `{`) are syntax errors. This module is a complete parser for that grammar
//! (ECMA-262 §22.2.1 with the `[UnicodeMode]` parameter set). Its regular
//! emitter and explicit-stack matcher share these character sets:
//!
//! * `\d`, `\w` and `\s` are the ECMA-262 sets written out code point by code
//!   point — `[0-9]`, `[0-9A-Za-z_]`, and the WhiteSpace ∪ LineTerminator list
//!   of §12.2/§12.3 — never `regex`'s Unicode classes of the same spelling,
//!   which are far larger.
//! * `.` excludes exactly the four LineTerminators; `^` and `$` anchor to the
//!   whole input (`$` does not match before a final newline); `\b` is the
//!   ASCII word boundary of `IsWordChar`.
//! * `\p{…}` accepts only the exact aliases ECMA-262 lists — every
//!   `General_Category` and `Script` alias of the Unicode 17.0.0
//!   `PropertyValueAliases.txt`, and ECMA-262's closed table of binary
//!   properties — and executes the vendored Unicode 17.0.0 ranges rather than
//!   the `regex` crate's Unicode version.
//! * A `\u` escape naming a lone surrogate can never match a Rust string, and
//!   is emitted as a set that matches nothing; a surrogate-pair escape is the
//!   one code point it spells.
//!
//! [`translate`] refuses constructs that cannot be represented by `regex`.
//! [`compile`] executes them through the VM. Invalid grammar is a
//! [`PatternError::Syntax`]; exhausted resources are a
//! [`PatternError::Resource`], never a negative match verdict.

mod emit;
mod property;
mod property_tables;
mod unicode_ranges;
mod vm;
pub use vm::MatchLimits;

use std::fmt;

use regex::Regex;

/// Why a pattern could not be translated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternError {
    /// The pattern is not an ECMA-262 regular expression (with the `u` flag).
    Syntax {
        /// Character offset of the offending construct.
        offset: usize,
        /// What is wrong.
        message: String,
    },
    /// The pattern is valid ECMA-262 but cannot be translated into the
    /// finite-automaton syntax returned by [`translate`]. [`compile`] uses
    /// the explicit-stack matcher for this construct.
    Unsupported {
        /// Character offset of the construct.
        offset: usize,
        /// The construct: `"lookahead"`, `"lookbehind"`, `"backreference"`,
        /// or `"modifier group"`.
        construct: &'static str,
    },
    /// A compiled character class was refused by the `regex` engine.
    Engine(String),
    /// A valid pattern exceeds an evaluator resource limit.
    Resource {
        /// Character offset where the limit was encountered.
        offset: usize,
        /// The exceeded limit.
        message: String,
    },
}

impl fmt::Display for PatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax { offset, message } => {
                write!(
                    f,
                    "not an ECMA-262 regular expression at offset {offset}: {message}"
                )
            }
            Self::Unsupported { offset, construct } => write!(
                f,
                "ECMA-262 {construct} at offset {offset} has no finite-automaton translation"
            ),
            Self::Engine(message) => write!(f, "the translated pattern was refused: {message}"),
            Self::Resource { offset, message } => {
                write!(f, "pattern resource limit at offset {offset}: {message}")
            }
        }
    }
}

impl std::error::Error for PatternError {}

/// Whether `pattern` is a syntactically valid ECMA-262 regular expression
/// under the `u` flag — the `format: "regex"` assertion. Exhausted parser
/// resources remain distinguishable from invalid syntax.
pub fn is_valid_syntax(pattern: &str) -> Result<bool, PatternError> {
    match parse(pattern) {
        Ok(_) => Ok(true),
        Err(PatternError::Syntax { .. }) => Ok(false),
        Err(error) => Err(error),
    }
}

/// Translate `pattern` into `regex` crate syntax with the same meaning.
pub fn translate(pattern: &str) -> Result<String, PatternError> {
    let ast = parse(pattern)?;
    emit::emit(&ast)
}

/// Compile `pattern`. Matching searches (it is not anchored), as
/// ECMA-262 `RegExp.prototype.test` does.
pub fn compile(pattern: &str) -> Result<CompiledPattern, PatternError> {
    let ast = parse(pattern)?;
    let captures = census(&pattern.chars().collect::<Vec<_>>()).groups as usize;
    match emit::emit(&ast) {
        Ok(source) => {
            if let Ok(regex) = Regex::new(&source) {
                return Ok(CompiledPattern {
                    backend: Backend::Regex(regex),
                });
            }
            // A valid regular pattern may exceed `regex`'s automaton-size
            // limit. The VM can execute it under the caller's budget.
        }
        Err(PatternError::Unsupported { .. }) => {}
        Err(error) => return Err(error),
    }
    Ok(CompiledPattern {
        backend: Backend::Vm(ast, captures),
    })
}

/// A parsed ECMA-262 expression, using the regular engine when it preserves
/// meaning and the explicit-stack matcher for the remaining constructs.
#[derive(Debug, Clone)]
pub struct CompiledPattern {
    backend: Backend,
}

#[derive(Debug, Clone)]
enum Backend {
    Regex(Regex),
    Vm(Ast, usize),
}

impl CompiledPattern {
    /// Search `input`, consuming the caller's shared resource budget.
    pub fn is_match(&self, input: &str, limits: &mut MatchLimits) -> Result<bool, PatternError> {
        match &self.backend {
            Backend::Regex(regex) => Ok(regex.is_match(input)),
            Backend::Vm(ast, captures) => vm::is_match(ast, input, limits, *captures),
        }
    }
}

/// A parsed pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Ast {
    Empty,
    /// One code point; may be a lone surrogate from a `\u` escape.
    Char(u32),
    /// `.`
    Dot,
    Class(Class),
    /// `^`
    Start,
    /// `$`
    End,
    /// `\b` (`false`) or `\B` (`true`).
    WordBoundary(bool),
    Group {
        body: Box<Self>,
        capture: Option<u32>,
    },
    Look {
        offset: usize,
        behind: bool,
        positive: bool,
        body: Box<Self>,
    },
    Backreference {
        offset: usize,
        targets: Vec<u32>,
    },
    Modifiers {
        offset: usize,
        body: Box<Self>,
        enable: u8,
        disable: u8,
    },
    Repeat {
        body: Box<Self>,
        min: u32,
        max: Option<u32>,
        greedy: bool,
    },
    Concat(Vec<Self>),
    Alternation(Vec<Self>),
}

/// A bracketed class, or a class escape standing alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Class {
    pub(crate) negated: bool,
    pub(crate) items: Vec<ClassItem>,
}

/// One member of a class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ClassItem {
    /// An inclusive code-point range; either end may be a surrogate.
    Range(u32, u32),
    /// `\d` / `\D`.
    Digit(bool),
    /// `\w` / `\W`.
    Word(bool),
    /// `\s` / `\S`.
    Space(bool),
    /// `\p{…}` / `\P{…}` — the `regex` spelling of the property, and whether
    /// it is negated.
    Property(Property, bool),
}

/// A resolved property escape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Property {
    /// Every code point.
    Any,
    /// U+0000..=U+007F.
    Ascii,
    /// Every assigned code point (`General_Category ≠ Cn`).
    Assigned,
    /// A `regex` property body, e.g. `gc=Lu`, `Script=Latin`, `Alphabetic`.
    Named(String),
}

fn syntax<T>(offset: usize, message: impl Into<String>) -> Result<T, PatternError> {
    Err(PatternError::Syntax {
        offset,
        message: message.into(),
    })
}

/// Parse `pattern` as an ECMA-262 `Pattern[+UnicodeMode, +NamedCaptureGroups]`.
/// How deeply groups — capturing, non-capturing, lookaround and modifier
/// groups alike — may nest in one pattern.
///
/// The parser descends once per group and every later pass over the parsed
/// expression follows the same shape, so the bound keeps the thread's stack
/// safe on every target, a wasm32 host's included. A pattern nested deeper is
/// valid ECMA-262 and is refused with [`PatternError::Resource`], never
/// reported as a syntax error.
pub const MAX_GROUP_NESTING: usize = 250;

pub(crate) fn parse(pattern: &str) -> Result<Ast, PatternError> {
    let chars: Vec<char> = pattern.chars().collect();
    let census = census(&chars);
    let mut parser = Parser {
        chars,
        pos: 0,
        groups: census.groups,
        names: census.names,
        next_capture: 0,
        disjunctions: 0,
        path: Vec::new(),
        declared: Vec::new(),
        depth: 0,
    };
    let ast = parser.disjunction()?;
    if parser.pos < parser.chars.len() {
        // Only an unmatched `)` stops the top-level disjunction early.
        return syntax(parser.pos, "unmatched `)`");
    }
    parser.check_duplicate_names()?;
    Ok(ast)
}

/// The capturing groups a pattern declares, counted before parsing because a
/// `\N` or `\k<name>` may refer forward (ECMA-262 §22.2.1.1 early errors).
struct Census {
    groups: u32,
    names: Vec<(String, u32)>,
}

fn census(chars: &[char]) -> Census {
    let mut groups = 0_u32;
    let mut names = Vec::new();
    let mut index = 0;
    let mut in_class = false;
    while index < chars.len() {
        match chars[index] {
            '\\' => index += 1,
            '[' if !in_class => in_class = true,
            ']' if in_class => in_class = false,
            '(' if !in_class => {
                if chars.get(index + 1) != Some(&'?') {
                    groups = groups.saturating_add(1);
                } else if chars.get(index + 2) == Some(&'<')
                    && !matches!(chars.get(index + 3), Some('=' | '!'))
                {
                    groups = groups.saturating_add(1);
                    let name: String = chars[index + 3..]
                        .iter()
                        .take_while(|&&ch| ch != '>')
                        .collect();
                    names.push((name, groups));
                }
            }
            _ => {}
        }
        index += 1;
    }
    Census { groups, names }
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
    groups: u32,
    /// Group names as the census spelled them (raw, escapes undecoded).
    names: Vec<(String, u32)>,
    next_capture: u32,
    disjunctions: u32,
    /// The (disjunction, alternative) pairs enclosing the current position.
    path: Vec<(u32, u32)>,
    /// Every named group: decoded name and the path it was declared under.
    declared: Vec<(String, Vec<(u32, u32)>)>,
    depth: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, ahead: usize) -> Option<char> {
        self.chars.get(self.pos + ahead).copied()
    }

    fn eat(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn disjunction(&mut self) -> Result<Ast, PatternError> {
        let id = self.disjunctions;
        self.disjunctions += 1;
        let mut alternatives = Vec::new();
        let mut index = 0;
        loop {
            self.path.push((id, index));
            let alternative = self.alternative();
            self.path.pop();
            alternatives.push(alternative?);
            if !self.eat('|') {
                break;
            }
            index += 1;
        }
        Ok(if alternatives.len() == 1 {
            alternatives.pop().unwrap_or(Ast::Empty)
        } else {
            Ast::Alternation(alternatives)
        })
    }

    fn alternative(&mut self) -> Result<Ast, PatternError> {
        let mut terms = Vec::new();
        while let Some(ch) = self.peek() {
            if ch == '|' || ch == ')' {
                break;
            }
            terms.push(self.term()?);
        }
        Ok(match terms.len() {
            0 => Ast::Empty,
            1 => terms.pop().unwrap_or(Ast::Empty),
            _ => Ast::Concat(terms),
        })
    }

    fn term(&mut self) -> Result<Ast, PatternError> {
        let start = self.pos;
        let (atom, quantifiable) = match self.peek() {
            Some('^') => {
                self.pos += 1;
                (Ast::Start, false)
            }
            Some('$') => {
                self.pos += 1;
                (Ast::End, false)
            }
            Some('\\') if matches!(self.peek_at(1), Some('b' | 'B')) => {
                let negated = self.peek_at(1) == Some('B');
                self.pos += 2;
                (Ast::WordBoundary(negated), false)
            }
            Some('(') if self.peek_at(1) == Some('?') => self.special_group(start)?,
            _ => (self.atom()?, true),
        };
        if let Some((min, max, greedy)) = self.quantifier()? {
            if !quantifiable {
                return syntax(
                    start,
                    "nothing to repeat: an assertion cannot be quantified",
                );
            }
            return Ok(Ast::Repeat {
                body: Box::new(atom),
                min,
                max,
                greedy,
            });
        }
        Ok(atom)
    }

    /// `(?=`, `(?!`, `(?<=`, `(?<!`, `(?:`, `(?<name>` and modifier groups.
    fn special_group(&mut self, start: usize) -> Result<(Ast, bool), PatternError> {
        self.pos += 2;
        match self.peek() {
            Some(':') => {
                self.pos += 1;
                let body = self.group_body(start)?;
                Ok((
                    Ast::Group {
                        body: Box::new(body),
                        capture: None,
                    },
                    true,
                ))
            }
            Some('=' | '!') => {
                let positive = self.peek() == Some('=');
                self.pos += 1;
                let body = self.group_body(start)?;
                // Lookaheads are not quantifiable under the `u` flag.
                Ok((
                    Ast::Look {
                        offset: start,
                        behind: false,
                        positive,
                        body: Box::new(body),
                    },
                    false,
                ))
            }
            Some('<') if matches!(self.peek_at(1), Some('=' | '!')) => {
                let positive = self.peek_at(1) == Some('=');
                self.pos += 2;
                let body = self.group_body(start)?;
                Ok((
                    Ast::Look {
                        offset: start,
                        behind: true,
                        positive,
                        body: Box::new(body),
                    },
                    false,
                ))
            }
            Some('<') => {
                self.pos += 1;
                let name = self.group_name()?;
                self.declared.push((name, self.path.clone()));
                self.next_capture += 1;
                let capture = self.next_capture;
                let body = self.group_body(start)?;
                Ok((
                    Ast::Group {
                        body: Box::new(body),
                        capture: Some(capture),
                    },
                    true,
                ))
            }
            _ => {
                let (enable, disable) = self.modifiers(start)?;
                let body = self.group_body(start)?;
                Ok((
                    Ast::Modifiers {
                        offset: start,
                        body: Box::new(body),
                        enable,
                        disable,
                    },
                    true,
                ))
            }
        }
    }

    /// `(?ims-ims:` (ES2025 `RegularExpressionModifiers`): each flag once
    /// across both sides, at least one flag, and the colon.
    fn modifiers(&mut self, start: usize) -> Result<(u8, u8), PatternError> {
        let mut seen = Vec::new();
        let mut adding = 0_u8;
        let mut removing = 0_u8;
        let mut after_dash = false;
        loop {
            match self.peek() {
                Some(flag @ ('i' | 'm' | 's')) => {
                    if seen.contains(&flag) {
                        return syntax(self.pos, "a modifier flag is repeated");
                    }
                    seen.push(flag);
                    if after_dash {
                        removing |= match flag {
                            'i' => 1,
                            'm' => 2,
                            _ => 4,
                        };
                    } else {
                        adding |= match flag {
                            'i' => 1,
                            'm' => 2,
                            _ => 4,
                        };
                    }
                    self.pos += 1;
                }
                Some('-') if !after_dash => {
                    after_dash = true;
                    self.pos += 1;
                }
                Some(':') => {
                    self.pos += 1;
                    break;
                }
                _ => return syntax(start, "invalid group: `(?` must begin a known group form"),
            }
        }
        if adding | removing == 0 && after_dash {
            return syntax(start, "a modifier group must name at least one flag");
        }
        if adding | removing == 0 {
            return syntax(start, "invalid group: `(?` must begin a known group form");
        }
        Ok((adding, removing))
    }

    fn group_body(&mut self, start: usize) -> Result<Ast, PatternError> {
        if self.depth >= MAX_GROUP_NESTING {
            return Err(PatternError::Resource {
                offset: start,
                message: format!("group nesting exceeds {MAX_GROUP_NESTING}"),
            });
        }
        self.depth += 1;
        let parsed = self.disjunction();
        self.depth -= 1;
        let body = parsed?;
        if !self.eat(')') {
            return syntax(start, "unterminated group");
        }
        Ok(body)
    }

    fn atom(&mut self) -> Result<Ast, PatternError> {
        let start = self.pos;
        let Some(ch) = self.peek() else {
            return syntax(start, "unexpected end of pattern");
        };
        match ch {
            '.' => {
                self.pos += 1;
                Ok(Ast::Dot)
            }
            '(' => {
                self.pos += 1;
                self.next_capture += 1;
                let capture = self.next_capture;
                let body = self.group_body(start)?;
                Ok(Ast::Group {
                    body: Box::new(body),
                    capture: Some(capture),
                })
            }
            '[' => {
                self.pos += 1;
                Ok(Ast::Class(self.class()?))
            }
            '\\' => {
                self.pos += 1;
                self.atom_escape(start)
            }
            '*' | '+' | '?' => syntax(start, "nothing to repeat"),
            '{' => syntax(
                start,
                "a lone `{` is not a pattern character under the `u` flag",
            ),
            ')' => syntax(start, "unmatched `)`"),
            ']' | '}' => syntax(
                start,
                format!("a lone `{ch}` is not a pattern character under the `u` flag"),
            ),
            other => {
                self.pos += 1;
                Ok(Ast::Char(u32::from(other)))
            }
        }
    }

    /// `* + ? {n} {n,} {n,m}`, each optionally followed by `?`.
    fn quantifier(&mut self) -> Result<Option<(u32, Option<u32>, bool)>, PatternError> {
        let start = self.pos;
        let bounds = match self.peek() {
            Some('*') => {
                self.pos += 1;
                (0, None)
            }
            Some('+') => {
                self.pos += 1;
                (1, None)
            }
            Some('?') => {
                self.pos += 1;
                (0, Some(1))
            }
            Some('{') => {
                self.pos += 1;
                let Some(min) = self.decimal()? else {
                    return syntax(
                        start,
                        "a `{` must begin a `{n}`, `{n,}` or `{n,m}` quantifier",
                    );
                };
                let max = if self.eat(',') {
                    if self.peek() == Some('}') {
                        None
                    } else {
                        let Some(max) = self.decimal()? else {
                            return syntax(start, "malformed `{n,m}` quantifier");
                        };
                        Some(max)
                    }
                } else {
                    Some(min)
                };
                if !self.eat('}') {
                    return syntax(start, "unterminated `{…}` quantifier");
                }
                if max.is_some_and(|max| max < min) {
                    return syntax(start, "numbers out of order in `{n,m}` quantifier");
                }
                (min, max)
            }
            _ => return Ok(None),
        };
        let greedy = !self.eat('?');
        Ok(Some((bounds.0, bounds.1, greedy)))
    }

    /// DecimalDigits. This matcher bounds repetitions at `u32::MAX`; a larger
    /// valid bound is a resource refusal rather than a saturated wrong match.
    fn decimal(&mut self) -> Result<Option<u32>, PatternError> {
        let start = self.pos;
        let mut value = 0_u32;
        while let Some(digit) = self.peek().and_then(|ch| ch.to_digit(10)) {
            value = value
                .checked_mul(10)
                .and_then(|n| n.checked_add(digit))
                .ok_or_else(|| PatternError::Resource {
                    offset: start,
                    message: "decimal repetition or backreference exceeds u32".to_owned(),
                })?;
            self.pos += 1;
        }
        Ok((self.pos > start).then_some(value))
    }

    fn atom_escape(&mut self, start: usize) -> Result<Ast, PatternError> {
        let Some(ch) = self.peek() else {
            return syntax(start, "`\\` at end of pattern");
        };
        match ch {
            '1'..='9' => {
                let value = self.decimal()?.unwrap_or(0);
                if value > self.groups {
                    return syntax(start, "backreference to a group the pattern does not have");
                }
                Ok(Ast::Backreference {
                    offset: start,
                    targets: vec![value],
                })
            }
            'k' => {
                self.pos += 1;
                if !self.eat('<') {
                    return syntax(start, "`\\k` must name a group under the `u` flag");
                }
                let name = self.group_name()?;
                let targets: Vec<u32> = self
                    .names_decoded()
                    .into_iter()
                    .filter_map(|(candidate, index)| (candidate == name).then_some(index))
                    .collect();
                if targets.is_empty() {
                    return syntax(start, "`\\k<…>` names a group the pattern does not have");
                }
                Ok(Ast::Backreference {
                    offset: start,
                    targets,
                })
            }
            _ => match self.class_or_char_escape(start, false)? {
                Escaped::Char(code) => Ok(Ast::Char(code)),
                Escaped::Item(item) => Ok(Ast::Class(Class {
                    negated: false,
                    items: vec![item],
                })),
            },
        }
    }

    fn names_decoded(&self) -> Vec<(String, u32)> {
        self.names
            .iter()
            .filter_map(|(raw, index)| {
                let mut sub = Self {
                    chars: raw.chars().chain(std::iter::once('>')).collect(),
                    pos: 0,
                    groups: 0,
                    names: Vec::new(),
                    next_capture: 0,
                    disjunctions: 0,
                    path: Vec::new(),
                    declared: Vec::new(),
                    depth: 0,
                };
                sub.group_name().ok().map(|name| (name, *index))
            })
            .collect()
    }

    /// A `CharacterClassEscape` or `CharacterEscape` after the `\`. Inside a
    /// class, `\b` is U+0008 and `\-` is `-`.
    fn class_or_char_escape(
        &mut self,
        start: usize,
        in_class: bool,
    ) -> Result<Escaped, PatternError> {
        let Some(ch) = self.peek() else {
            return syntax(start, "`\\` at end of pattern");
        };
        self.pos += 1;
        let code = match ch {
            'd' => return Ok(Escaped::Item(ClassItem::Digit(false))),
            'D' => return Ok(Escaped::Item(ClassItem::Digit(true))),
            'w' => return Ok(Escaped::Item(ClassItem::Word(false))),
            'W' => return Ok(Escaped::Item(ClassItem::Word(true))),
            's' => return Ok(Escaped::Item(ClassItem::Space(false))),
            'S' => return Ok(Escaped::Item(ClassItem::Space(true))),
            'p' | 'P' => {
                let property = self.property(start)?;
                return Ok(Escaped::Item(ClassItem::Property(property, ch == 'P')));
            }
            'b' if in_class => 0x08,
            '-' if in_class => u32::from('-'),
            'f' => 0x0C,
            'n' => 0x0A,
            'r' => 0x0D,
            't' => 0x09,
            'v' => 0x0B,
            'c' => match self.peek() {
                Some(letter) if letter.is_ascii_alphabetic() => {
                    self.pos += 1;
                    u32::from(letter) % 32
                }
                _ => return syntax(start, "`\\c` must be followed by an ASCII letter"),
            },
            '0' => {
                if self.peek().is_some_and(|next| next.is_ascii_digit()) {
                    return syntax(start, "octal escapes are not allowed under the `u` flag");
                }
                0
            }
            'x' => {
                let Some(value) = self.hex_digits(2) else {
                    return syntax(start, "`\\x` must be followed by two hex digits");
                };
                value
            }
            'u' => self.unicode_escape(start)?,
            '^' | '$' | '\\' | '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|'
            | '/' => u32::from(ch),
            _ => {
                return syntax(
                    start,
                    format!("`\\{ch}` is not an escape under the `u` flag"),
                );
            }
        };
        Ok(Escaped::Char(code))
    }

    fn hex_digits(&mut self, count: usize) -> Option<u32> {
        let mut value = 0_u32;
        for ahead in 0..count {
            let digit = self.peek_at(ahead)?.to_digit(16)?;
            value = value * 16 + digit;
        }
        self.pos += count;
        Some(value)
    }

    /// `\uHHHH`, `\uHHHH\uHHHH` (a surrogate pair is one code point), and
    /// `\u{H…}` — the `u` is already consumed.
    fn unicode_escape(&mut self, start: usize) -> Result<u32, PatternError> {
        if self.eat('{') {
            let mut value = 0_u32;
            let mut digits = 0;
            while let Some(digit) = self.peek().and_then(|ch| ch.to_digit(16)) {
                value = value.saturating_mul(16).saturating_add(digit);
                digits += 1;
                self.pos += 1;
            }
            if digits == 0 || !self.eat('}') || value > 0x10_FFFF {
                return syntax(
                    start,
                    "`\\u{…}` must hold a code point no greater than 10FFFF",
                );
            }
            return Ok(value);
        }
        let Some(lead) = self.hex_digits(4) else {
            return syntax(start, "`\\u` must be followed by four hex digits or `{…}`");
        };
        if (0xD800..=0xDBFF).contains(&lead)
            && self.peek() == Some('\\')
            && self.peek_at(1) == Some('u')
        {
            let saved = self.pos;
            self.pos += 2;
            match self.hex_digits(4) {
                Some(trail) if (0xDC00..=0xDFFF).contains(&trail) => {
                    return Ok(0x1_0000 + ((lead - 0xD800) << 10) + (trail - 0xDC00));
                }
                _ => self.pos = saved,
            }
        }
        Ok(lead)
    }

    /// `\p{Name}` / `\p{Name=Value}` — the `p` is already consumed.
    fn property(&mut self, start: usize) -> Result<Property, PatternError> {
        if !self.eat('{') {
            return syntax(start, "`\\p` must be followed by `{`");
        }
        let body_start = self.pos;
        while let Some(ch) = self.peek() {
            if ch == '}' {
                break;
            }
            if !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '=') {
                return syntax(
                    self.pos,
                    "a property name may hold only ASCII letters, digits and `_`",
                );
            }
            self.pos += 1;
        }
        let body: String = self.chars[body_start..self.pos].iter().collect();
        if !self.eat('}') {
            return syntax(start, "unterminated `\\p{…}`");
        }
        let unknown = || {
            syntax(
                start,
                format!("`\\p{{{body}}}` is not a property ECMA-262 lists"),
            )
        };
        if let Some((name, value)) = body.split_once('=') {
            return match name {
                "General_Category" | "gc" => property::lookup(property::GENERAL_CATEGORY, value)
                    .map_or_else(unknown, |short| Ok(Property::Named(format!("gc={short}")))),
                "Script" | "sc" => property::lookup(property::SCRIPT, value)
                    .map_or_else(unknown, |long| {
                        Ok(Property::Named(format!("Script={long}")))
                    }),
                "Script_Extensions" | "scx" => property::lookup(property::SCRIPT, value)
                    .map_or_else(unknown, |long| {
                        Ok(Property::Named(format!("Script_Extensions={long}")))
                    }),
                _ => unknown(),
            };
        }
        if let Some(short) = property::lookup(property::GENERAL_CATEGORY, &body) {
            return Ok(Property::Named(format!("gc={short}")));
        }
        match property::lookup(property::BINARY, &body) {
            Some("Any") => Ok(Property::Any),
            Some("ASCII") => Ok(Property::Ascii),
            Some("Assigned") => Ok(Property::Assigned),
            Some(canonical) => Ok(Property::Named(canonical.to_owned())),
            None => unknown(),
        }
    }

    /// `RegExpIdentifierName >` — the `<` is already consumed.
    fn group_name(&mut self) -> Result<String, PatternError> {
        let start = self.pos;
        let mut name = String::new();
        loop {
            let Some(ch) = self.peek() else {
                return syntax(start, "unterminated group name");
            };
            if ch == '>' {
                self.pos += 1;
                break;
            }
            self.pos += 1;
            let code = if ch == '\\' {
                if !self.eat('u') {
                    return syntax(self.pos, "only `\\u` escapes may appear in a group name");
                }
                self.unicode_escape(start)?
            } else {
                u32::from(ch)
            };
            let Some(ch) = char::from_u32(code) else {
                return syntax(start, "a group name may not hold a lone surrogate");
            };
            let admitted = if name.is_empty() {
                identifier_start(ch)
            } else {
                identifier_part(ch)
            };
            if !admitted {
                return syntax(start, "a group name must be an ECMAScript IdentifierName");
            }
            name.push(ch);
        }
        if name.is_empty() {
            return syntax(start, "a group name may not be empty");
        }
        Ok(name)
    }

    /// `[` is already consumed.
    fn class(&mut self) -> Result<Class, PatternError> {
        let start = self.pos - 1;
        let negated = self.eat('^');
        let mut items = Vec::new();
        loop {
            match self.peek() {
                None => return syntax(start, "unterminated character class"),
                Some(']') => {
                    self.pos += 1;
                    break;
                }
                Some(_) => {}
            }
            let first_at = self.pos;
            let first = self.class_atom()?;
            if self.peek() == Some('-') && self.peek_at(1).is_some_and(|next| next != ']') {
                self.pos += 1;
                let second = self.class_atom()?;
                let (Escaped::Char(low), Escaped::Char(high)) = (&first, &second) else {
                    return syntax(
                        first_at,
                        "a class escape cannot bound a range under the `u` flag",
                    );
                };
                if low > high {
                    return syntax(first_at, "range out of order in character class");
                }
                items.push(ClassItem::Range(*low, *high));
                continue;
            }
            items.push(match first {
                Escaped::Char(code) => ClassItem::Range(code, code),
                Escaped::Item(item) => item,
            });
        }
        Ok(Class { negated, items })
    }

    fn class_atom(&mut self) -> Result<Escaped, PatternError> {
        let start = self.pos;
        match self.peek() {
            Some('\\') => {
                self.pos += 1;
                if self
                    .peek()
                    .is_some_and(|ch| ch.is_ascii_digit() && ch != '0')
                    || self.peek() == Some('k')
                {
                    return syntax(start, "a backreference cannot appear in a character class");
                }
                if self.peek() == Some('B') {
                    return syntax(start, "`\\B` is not a class escape");
                }
                self.class_or_char_escape(start, true)
            }
            Some(ch) => {
                self.pos += 1;
                Ok(Escaped::Char(u32::from(ch)))
            }
            None => syntax(start, "unterminated character class"),
        }
    }

    /// ES2025: two groups may share a name only when no match can take both,
    /// i.e. some disjunction holds them in different alternatives.
    fn check_duplicate_names(&self) -> Result<(), PatternError> {
        for (index, (name, path)) in self.declared.iter().enumerate() {
            for (other, other_path) in &self.declared[index + 1..] {
                if name != other {
                    continue;
                }
                let exclusive = path.iter().any(|&(disjunction, alternative)| {
                    other_path
                        .iter()
                        .any(|&(d, a)| d == disjunction && a != alternative)
                });
                if !exclusive {
                    return syntax(0, format!("duplicate capture group name `{name}`"));
                }
            }
        }
        Ok(())
    }
}

/// What an escape denotes: one code point, or a set.
#[derive(Debug)]
enum Escaped {
    Char(u32),
    Item(ClassItem),
}

/// `ID_Start ∪ {$, _}` — ECMA-262 `IdentifierStartChar`, which names the
/// Unicode `ID_Start` property.
fn identifier_start(ch: char) -> bool {
    ch == '$' || ch == '_' || contains_property("ID_Start", u32::from(ch))
}

/// `ID_Continue ∪ {$, ZWNJ, ZWJ}` — ECMA-262 `IdentifierPartChar`, which names
/// the Unicode `ID_Continue` property.
fn identifier_part(ch: char) -> bool {
    matches!(ch, '$' | '\u{200C}' | '\u{200D}') || contains_property("ID_Continue", u32::from(ch))
}

fn property_ranges(name: &str) -> &'static [(u32, u32)] {
    let index = unicode_ranges::RANGES
        .binary_search_by_key(&name, |(key, _)| key)
        .expect("accepted property has UCD ranges");
    unicode_ranges::RANGES[index].1
}

fn contains_property(name: &str, code: u32) -> bool {
    let ranges = property_ranges(name);
    let index = ranges.partition_point(|(_, high)| *high < code);
    ranges.get(index).is_some_and(|(low, _)| *low <= code)
}

fn canonicalize(code: u32) -> u32 {
    let table = unicode_ranges::CASE_FOLD;
    table
        .binary_search_by_key(&code, |(source, _)| *source)
        .map_or(code, |index| table[index].1)
}

#[cfg(test)]
mod tests;
