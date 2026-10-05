// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The dated XPath grammar over the existing XSD/XPath token cursor.
//!
//! The program and character sets are flat arenas: nesting never becomes
//! recursive Rust ownership, parsing or destruction. Quantities are compared
//! as exact decimal strings before their bounded execution representation is
//! chosen. Neither the compatibility regex engine nor its Unicode tables
//! participate in native recognition.

use std::cmp::Ordering;
use std::ops::Range;

use super::{Budget, Error, Limits, Profile, Resource, unicode_tables};
use crate::xsd_regex::scan::{Scanner, Token};

#[derive(Debug, Clone, Copy)]
// These are the independently combinable XPath flags, not exclusive states.
#[allow(clippy::struct_excessive_bools)]
pub(super) struct Modes {
    pub insensitive: bool,
    pub dot_all: bool,
    pub multiline: bool,
    pub whitespace: bool,
    pub quoted: bool,
}

impl Modes {
    fn parse(profile: Profile, flags: &str, budget: &mut Budget) -> Result<Self, Error> {
        let mut result = Self {
            insensitive: false,
            dot_all: false,
            multiline: false,
            whitespace: false,
            quoted: false,
        };
        for (offset, flag) in flags.char_indices() {
            budget.charge(Resource::CompileSteps, 1)?;
            match flag {
                'i' => result.insensitive = true,
                's' => result.dot_all = true,
                'm' => result.multiline = true,
                'x' => result.whitespace = true,
                'q' if profile == Profile::Xpath31 => result.quoted = true,
                _ => {
                    return Err(Error::Flags {
                        offset,
                        flag,
                        profile,
                    });
                }
            }
        }
        Ok(result)
    }
}

/// A quantity larger than u64 is not malformed and is never saturated.
///
/// Every attempted repetition spends fresh finite u64 work before incrementing
/// its u64 count. No admitted execution can reach AboveU64. Exact ordering of
/// arbitrarily large minimum/maximum quantities is checked before this lowering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Count {
    Finite(u64),
    AboveU64,
}

impl Count {
    fn from_decimal(decimal: &str) -> Self {
        decimal.parse::<u64>().map_or(Self::AboveU64, Self::Finite)
    }
}

#[derive(Debug)]
pub(super) enum Node {
    Empty,
    Character(usize),
    Start,
    End,
    Backreference(usize),
    Sequence(usize, usize),
    Choice(usize, usize),
    Capture {
        number: usize,
        body: usize,
    },
    Repeat {
        body: usize,
        min: Count,
        max: Option<Count>,
        greedy: bool,
    },
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Set {
    Range { lo: char, hi: char, folded: bool },
    Table(&'static [(u32, u32)]),
    Space,
    Word,
    Dot,
    Complement(usize),
    Union(usize, usize),
    Difference(usize, usize),
}

#[derive(Debug, Clone, Copy)]
struct Admission {
    nodes: u64,
    slots: u64,
}

/// An admitted flat program under one explicitly dated XPath law.
///
/// The source and flags identify the law exactly, including quoted source and
/// whitespace removal. Reuse must call admit with the current request's bounds;
/// successful compilation under earlier, larger bounds is not current admission.
#[derive(Debug)]
pub struct CompiledPattern {
    pub(super) profile: Profile,
    pub(super) source: String,
    pub(super) flags: String,
    pub(super) modes: Modes,
    pub(super) nodes: Vec<Node>,
    pub(super) sets: Vec<Set>,
    pub(super) root: usize,
    pub(super) captures: usize,
    admission: Admission,
}

impl CompiledPattern {
    /// The dated Recommendation used for recognition and execution.
    #[must_use]
    pub const fn profile(&self) -> Profile {
        self.profile
    }

    /// The original UTF-8 source, before any x-flag removal.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The original flags. Repeated flags are legal in both Recommendations.
    #[must_use]
    pub fn flags(&self) -> &str {
        &self.flags
    }

    /// Capturing groups, excluding the entire-pattern capture zero.
    #[must_use]
    pub const fn capture_count(&self) -> usize {
        self.captures
    }

    /// Owned program payload bytes, including retained buffer capacities.
    ///
    /// Shared Unicode tables and allocator metadata are excluded. A sum that
    /// exceeds the host's address width saturates so bounded caches can decline
    /// retention without changing execution or admission.
    #[must_use]
    pub fn storage_bytes(&self) -> usize {
        size_of::<Self>()
            .saturating_add(self.source.capacity())
            .saturating_add(self.flags.capacity())
            .saturating_add(self.nodes.capacity().saturating_mul(size_of::<Node>()))
            .saturating_add(self.sets.capacity().saturating_mul(size_of::<Set>()))
    }

    /// Admit a reused artifact without treating it as newly executed work.
    ///
    /// The source, program nodes and peak construction-storage contract are
    /// checked before a cache hit. Each match still receives fresh work fuel.
    ///
    /// # Errors
    ///
    /// A typed resource refusal under the current request's bounds.
    pub fn admit(&self, limits: Limits) -> Result<(), Error> {
        limits.admit_pattern(&self.source)?;
        limits.admit(Resource::ProgramNodes, u128::from(self.admission.nodes))?;
        limits.admit(Resource::CompileSlots, u128::from(self.admission.slots))?;
        Ok(())
    }
}

/// Recognize the complete grammar of the selected dated Recommendation.
///
/// XPath 2.0 already defines backreferences and reluctant quantifiers. XPath
/// 3.1 additionally defines non-capturing groups and the q flag. The grammar
/// uses XML Schema Part 2 Second Edition Appendix F with the XPath corrections.
///
/// # Errors
///
/// Invalid grammar is Syntax; source, work, program or storage refusal is an
/// operational Resource error. Source admission precedes any parsing or copy.
pub fn compile(
    profile: Profile,
    pattern: &str,
    flags: &str,
    limits: Limits,
) -> Result<CompiledPattern, Error> {
    limits.admit_pattern(pattern)?;
    let mut budget = Budget::new(limits);
    let modes = Modes::parse(profile, flags, &mut budget)?;
    let source = owned(pattern, &mut budget)?;
    let flag_source = owned(flags, &mut budget)?;
    let normalized = if modes.whitespace && !modes.quoted {
        Some(crate::xsd_regex::xflag::strip_bounded(
            pattern,
            &mut budget,
        )?)
    } else {
        None
    };
    let text = normalized
        .as_ref()
        .map_or(pattern, |(text, _)| text.as_str());
    let offsets = normalized.as_ref().map(|(_, offsets)| offsets.as_slice());
    let before = budget.used(Resource::CompileSlots);
    let scanner = Scanner::bounded(text, &mut budget, false)?;
    let scanner_slots = budget.used(Resource::CompileSlots) - before;
    let mut parser = Parser {
        scanner,
        budget,
        profile,
        modes,
        text,
        offsets,
        pending: None,
        nodes: Vec::new(),
        sets: Vec::new(),
        frames: Vec::new(),
        captures: 0,
    };
    let root = if modes.quoted {
        parser.quoted()?
    } else {
        parser.expression()?
    };
    let admission = Admission {
        nodes: parser.budget.used(Resource::ProgramNodes),
        slots: parser.budget.peak_compile_slots,
    };
    // No token cursor or construction-only frame survives in the artifact.
    let Parser {
        scanner,
        mut budget,
        nodes,
        sets,
        captures,
        ..
    } = parser;
    drop(scanner);
    budget.release_compile_slots(scanner_slots);
    Ok(CompiledPattern {
        profile,
        source,
        flags: flag_source,
        modes,
        nodes,
        sets,
        root,
        captures,
        admission,
    })
}

pub(super) fn syntax(offset: usize, message: &str) -> Error {
    Error::Syntax {
        offset,
        message: message.to_owned(),
    }
}

pub(super) fn owned(text: &str, budget: &mut Budget) -> Result<String, Error> {
    budget.charge_wide(Resource::CompileSlots, text.len() as u128)?;
    budget.charge_wide(Resource::CompileSteps, text.len() as u128)?;
    let mut out = String::new();
    out.try_reserve_exact(text.len())
        .map_err(|_| Error::Allocation {
            resource: Resource::CompileSlots,
            units: text.len() as u64,
        })?;
    out.push_str(text);
    Ok(out)
}

/// Arena capacity remains retained after a frame is popped. Admit deterministic
/// capacity growth, including copying the existing elements, before allocation.
fn grow<T>(budget: &mut Budget, arena: &mut Vec<T>, cells: u64) -> Result<(), Error> {
    if arena.len() < arena.capacity() {
        return Ok(());
    }
    let old = arena.capacity() as u128;
    let capacity = (old * 2).max(1);
    let slots = (capacity - old) * u128::from(cells);
    budget.charge_wide(Resource::CompileSlots, slots)?;
    budget.charge_wide(Resource::CompileSteps, arena.len() as u128)?;
    let units = u64::try_from(capacity * u128::from(cells))
        .expect("admitted capacity fits its finite u64 slot bound");
    let allocation = || Error::Allocation {
        resource: Resource::CompileSlots,
        units,
    };
    let capacity = usize::try_from(capacity).map_err(|_| allocation())?;
    let bytes = capacity
        .checked_mul(size_of::<T>())
        .ok_or_else(allocation)?;
    if bytes > isize::MAX as usize {
        return Err(allocation());
    }
    arena
        .try_reserve_exact(capacity - arena.len())
        .map_err(|_| allocation())
}

#[derive(Clone)]
struct Spanned<'a> {
    token: Token<'a>,
    span: Range<usize>,
}

struct Frame {
    opening: usize,
    capture: Option<usize>,
    sequence: Option<usize>,
    choices: Option<usize>,
    atom: Option<usize>,
    quantified: bool,
}

impl Frame {
    fn new(opening: usize, capture: Option<usize>) -> Self {
        Self {
            opening,
            capture,
            sequence: None,
            choices: None,
            atom: None,
            quantified: false,
        }
    }
}

#[derive(Clone, Copy)]
struct ClassFrame {
    negated: bool,
    union: Option<usize>,
    difference: Option<usize>,
}

struct Parser<'a> {
    scanner: Scanner<'a>,
    budget: Budget,
    profile: Profile,
    modes: Modes,
    text: &'a str,
    offsets: Option<&'a [usize]>,
    pending: Option<Spanned<'a>>,
    nodes: Vec<Node>,
    sets: Vec<Set>,
    frames: Vec<Frame>,
    captures: usize,
}

impl<'a> Parser<'a> {
    fn offset(&self, offset: usize) -> usize {
        self.offsets.map_or(offset, |map| map[offset])
    }

    fn peek(&mut self) -> Result<Option<Spanned<'a>>, Error> {
        if self.pending.is_none()
            && let Some(token) = self.scanner.next_bounded(&mut self.budget)?
        {
            let span = self.scanner.source_span();
            let token =
                token.map_err(|error| syntax(self.offset(span.start), &error.to_string()))?;
            self.pending = Some(Spanned { token, span });
        }
        Ok(self.pending.clone())
    }

    fn take(&mut self) -> Result<Option<Spanned<'a>>, Error> {
        self.peek()?;
        self.budget.charge(Resource::CompileSteps, 1)?;
        Ok(self.pending.take())
    }

    fn node(&mut self, node: Node) -> Result<usize, Error> {
        self.budget.charge(Resource::CompileSteps, 1)?;
        self.budget.charge(Resource::ProgramNodes, 1)?;
        grow(&mut self.budget, &mut self.nodes, 6)?;
        let id = self.nodes.len();
        self.nodes.push(node);
        Ok(id)
    }

    fn set(&mut self, set: Set) -> Result<usize, Error> {
        self.budget.charge(Resource::CompileSteps, 1)?;
        self.budget.charge(Resource::ProgramNodes, 1)?;
        grow(&mut self.budget, &mut self.sets, 3)?;
        let id = self.sets.len();
        self.sets.push(set);
        Ok(id)
    }

    fn frame(&mut self, opening: usize, capture: Option<usize>) -> Result<(), Error> {
        grow(&mut self.budget, &mut self.frames, 6)?;
        self.frames.push(Frame::new(opening, capture));
        Ok(())
    }

    fn append(&mut self, atom: usize) -> Result<(), Error> {
        self.flush_atom()?;
        let frame = self
            .frames
            .last_mut()
            .expect("an expression frame is present");
        frame.atom = Some(atom);
        frame.quantified = false;
        Ok(())
    }

    fn flush_atom(&mut self) -> Result<(), Error> {
        let frame = self
            .frames
            .last_mut()
            .expect("an expression frame is present");
        if let Some(atom) = frame.atom.take() {
            let previous = frame.sequence.take();
            let sequence = if let Some(previous) = previous {
                self.node(Node::Sequence(previous, atom))?
            } else {
                atom
            };
            self.frames.last_mut().expect("frame retained").sequence = Some(sequence);
        }
        Ok(())
    }

    fn branch(&mut self) -> Result<usize, Error> {
        self.flush_atom()?;
        let frame = self
            .frames
            .last_mut()
            .expect("an expression frame is present");
        let sequence = frame.sequence.take();
        let choices = frame.choices.take();
        let sequence = if let Some(sequence) = sequence {
            sequence
        } else {
            self.node(Node::Empty)?
        };
        if let Some(choices) = choices {
            self.node(Node::Choice(choices, sequence))
        } else {
            Ok(sequence)
        }
    }

    fn quoted(&mut self) -> Result<usize, Error> {
        self.frame(0, None)?;
        for ch in self.text.chars() {
            self.budget.charge(Resource::CompileSteps, 1)?;
            let set = self.range(ch, ch)?;
            let node = self.node(Node::Character(set))?;
            self.append(node)?;
        }
        self.branch()
    }

    fn expression(&mut self) -> Result<usize, Error> {
        self.frame(0, None)?;
        while let Some(spanned) = self.take()? {
            let offset = self.offset(spanned.span.start);
            match spanned.token {
                Token::GroupOpen { capturing } => {
                    if !capturing && self.profile == Profile::Xpath20 {
                        return Err(syntax(offset, "non-capturing groups require XPath 3.1"));
                    }
                    let capture = if capturing {
                        self.captures += 1;
                        Some(self.captures)
                    } else {
                        None
                    };
                    self.frame(offset, capture)?;
                }
                Token::GroupClose => {
                    if self.frames.len() == 1 {
                        return Err(syntax(offset, "unmatched closing parenthesis"));
                    }
                    let body = self.branch()?;
                    let frame = self.frames.pop().expect("non-root group present");
                    let node = if let Some(number) = frame.capture {
                        self.node(Node::Capture { number, body })?
                    } else {
                        body
                    };
                    self.append(node)?;
                }
                Token::Literal('|') => {
                    let choices = self.branch()?;
                    self.frames.last_mut().expect("frame retained").choices = Some(choices);
                }
                Token::Literal(quantifier @ ('?' | '*' | '+' | '{')) => {
                    self.quantifier(quantifier, offset)?;
                }
                Token::Literal('}') => {
                    return Err(syntax(offset, "a closing brace must end a quantity"));
                }
                Token::Literal('^') => {
                    let node = self.node(Node::Start)?;
                    self.append(node)?;
                }
                Token::Literal('$') => {
                    let node = self.node(Node::End)?;
                    self.append(node)?;
                }
                Token::Backreference(number) => {
                    let node = self.node(Node::Backreference(number))?;
                    self.append(node)?;
                }
                Token::ClassOpen { negated } => {
                    let set = self.class(negated)?;
                    let node = self.node(Node::Character(set))?;
                    self.append(node)?;
                }
                Token::ClassClose | Token::Subtract | Token::ClassMember(_) => {
                    return Err(syntax(offset, "character-class syntax outside a class"));
                }
                token => {
                    let set = self.member(token, offset)?;
                    let node = self.node(Node::Character(set))?;
                    self.append(node)?;
                }
            }
        }
        if self.frames.len() != 1 {
            return Err(syntax(
                self.frames.last().expect("open group").opening,
                "unclosed group",
            ));
        }
        self.branch()
    }

    fn quantifier(&mut self, quantifier: char, offset: usize) -> Result<(), Error> {
        let frame = self
            .frames
            .last_mut()
            .expect("an expression frame is present");
        if frame.quantified {
            return Err(syntax(offset, "an atom has only one quantifier"));
        }
        let Some(body) = frame.atom.take() else {
            return Err(syntax(offset, "quantifier has no preceding atom"));
        };
        let (min, max) = match quantifier {
            '?' => (Count::Finite(0), Some(Count::Finite(1))),
            '*' => (Count::Finite(0), None),
            '+' => (Count::Finite(1), None),
            '{' => self.quantity(offset)?,
            _ => unreachable!("caller selects the closed quantifier grammar"),
        };
        let greedy = if self
            .peek()?
            .is_some_and(|spanned| spanned.token == Token::Literal('?'))
        {
            self.take()?;
            false
        } else {
            true
        };
        let node = self.node(Node::Repeat {
            body,
            min,
            max,
            greedy,
        })?;
        let frame = self.frames.last_mut().expect("frame retained");
        frame.atom = Some(node);
        frame.quantified = true;
        Ok(())
    }

    fn decimal(&mut self, offset: usize) -> Result<&'a str, Error> {
        let start = self
            .peek()?
            .map_or(self.text.len(), |spanned| spanned.span.start);
        let mut end = start;
        while let Some(spanned) = self.peek()? {
            if !matches!(spanned.token, Token::Literal('0'..='9')) {
                break;
            }
            end = spanned.span.end;
            self.take()?;
        }
        if start == end {
            return Err(syntax(
                offset,
                "a quantity requires one or more ASCII digits",
            ));
        }
        let text = self.text[start..end].trim_start_matches('0');
        Ok(if text.is_empty() { "0" } else { text })
    }

    fn quantity(&mut self, offset: usize) -> Result<(Count, Option<Count>), Error> {
        let min = self.decimal(offset)?;
        let Some(next) = self.take()? else {
            return Err(syntax(offset, "unclosed quantity"));
        };
        match next.token {
            Token::Literal('}') => Ok((Count::from_decimal(min), Some(Count::from_decimal(min)))),
            Token::Literal(',') => {
                if self
                    .peek()?
                    .is_some_and(|spanned| spanned.token == Token::Literal('}'))
                {
                    self.take()?;
                    return Ok((Count::from_decimal(min), None));
                }
                let max = self.decimal(offset)?;
                self.budget.charge_wide(
                    Resource::CompileSteps,
                    min.len() as u128 + max.len() as u128,
                )?;
                if min.len().cmp(&max.len()).then_with(|| min.cmp(max)) == Ordering::Greater {
                    return Err(syntax(offset, "quantity minimum exceeds its maximum"));
                }
                if !self
                    .take()?
                    .is_some_and(|spanned| spanned.token == Token::Literal('}'))
                {
                    return Err(syntax(offset, "a quantity must end with its closing brace"));
                }
                Ok((Count::from_decimal(min), Some(Count::from_decimal(max))))
            }
            _ => Err(syntax(offset, "quantity must be n, n, or n,m")),
        }
    }

    fn range(&mut self, lo: char, hi: char) -> Result<usize, Error> {
        self.set(Set::Range {
            lo,
            hi,
            folded: self.modes.insensitive,
        })
    }

    fn property(&mut self, name: &str, offset: usize) -> Result<usize, Error> {
        self.budget
            .charge_wide(Resource::CompileSteps, (name.len() as u128) * 10)?;
        let found = if name.starts_with("Is") {
            unicode_tables::BLOCKS
                .binary_search_by_key(&name, |&(name, _)| name)
                .ok()
                .map(|index| unicode_tables::BLOCKS[index].1)
        } else {
            unicode_tables::CATEGORIES
                .binary_search_by_key(&name, |&(name, _)| name)
                .ok()
                .map(|index| unicode_tables::CATEGORIES[index].1)
        };
        let Some(ranges) = found else {
            return Err(syntax(
                offset,
                "unknown Unicode category or Is-prefixed block",
            ));
        };
        self.set(Set::Table(ranges))
    }

    fn member(&mut self, token: Token<'_>, offset: usize) -> Result<usize, Error> {
        let (set, negated) = match token {
            Token::Literal(ch) | Token::ClassMember(ch) => return self.range(ch, ch),
            Token::Escape('n') => return self.range('\n', '\n'),
            Token::Escape('r') => return self.range('\r', '\r'),
            Token::Escape('t') => return self.range('\t', '\t'),
            Token::Escape('d' | 'D') => (self.property("Nd", offset)?, token == Token::Escape('D')),
            Token::Escape('&' | '~') => {
                return Err(syntax(offset, "escape is outside XPath SingleCharEsc"));
            }
            Token::Escape(ch) => return self.range(ch, ch),
            Token::NameEscape { negated, chars } => {
                let ranges = if chars {
                    purrdf_iri::terminals::xml_name_char_ranges()
                } else {
                    purrdf_iri::terminals::xml_name_start_char_ranges()
                };
                (self.set(Set::Table(ranges))?, negated)
            }
            Token::SpaceEscape { negated } => (self.set(Set::Space)?, negated),
            Token::WordEscape { negated } => (self.set(Set::Word)?, negated),
            Token::UnicodeProperty { negated, name } => (self.property(name, offset)?, negated),
            Token::Dot => return self.set(Set::Dot),
            _ => return Err(syntax(offset, "construct is not a character-class member")),
        };
        if negated {
            self.set(Set::Complement(set))
        } else {
            Ok(set)
        }
    }

    fn scalar(token: &Token<'_>) -> Option<char> {
        match token {
            Token::Literal(ch) | Token::ClassMember(ch) => Some(*ch),
            Token::Escape('n') => Some('\n'),
            Token::Escape('r') => Some('\r'),
            Token::Escape('t') => Some('\t'),
            Token::Escape('d' | 'D' | '&' | '~') => None,
            Token::Escape(ch) => Some(*ch),
            _ => None,
        }
    }

    fn class_union(&mut self, frame: &mut ClassFrame, set: usize) -> Result<(), Error> {
        frame.union = Some(if let Some(previous) = frame.union {
            self.set(Set::Union(previous, set))?
        } else {
            set
        });
        Ok(())
    }

    fn class(&mut self, negated: bool) -> Result<usize, Error> {
        let mut frames = Vec::new();
        self.class_frame(&mut frames, negated)?;
        loop {
            let Some(spanned) = self.take()? else {
                return Err(syntax(
                    self.offset(self.text.len()),
                    "unclosed character class",
                ));
            };
            let offset = self.offset(spanned.span.start);
            match spanned.token {
                Token::ClassOpen { negated } => self.class_frame(&mut frames, negated)?,
                Token::ClassClose => {
                    let frame = frames.pop().expect("a class frame is present");
                    let set = self.finish_class(frame, offset)?;
                    if let Some(parent) = frames.last_mut() {
                        let base = parent
                            .difference
                            .take()
                            .expect("nested class is a subtraction operand");
                        parent.union = Some(self.set(Set::Difference(base, set))?);
                        parent.negated = false;
                    } else {
                        let slots = frames.capacity() as u64 * 3;
                        drop(frames);
                        self.budget.release_compile_slots(slots);
                        return Ok(set);
                    }
                }
                Token::Subtract => {
                    let frame = frames.last_mut().expect("a class frame is present");
                    let base = frame
                        .union
                        .take()
                        .ok_or_else(|| syntax(offset, "subtraction has no left character group"))?;
                    frame.difference = Some(if frame.negated {
                        self.set(Set::Complement(base))?
                    } else {
                        base
                    });
                }
                token => {
                    let frame = frames.last_mut().expect("a class frame is present");
                    if token == Token::Literal('-') {
                        let at_end = self.peek()?.is_some_and(|next| {
                            matches!(next.token, Token::ClassClose | Token::Subtract)
                        });
                        if frame.union.is_some() && !at_end {
                            return Err(syntax(
                                offset,
                                "an unescaped hyphen is only a group endpoint or range operator",
                            ));
                        }
                    }
                    let set = self.class_member(token, offset)?;
                    self.class_union(frame, set)?;
                }
            }
        }
    }

    fn class_frame(&mut self, frames: &mut Vec<ClassFrame>, negated: bool) -> Result<(), Error> {
        grow(&mut self.budget, frames, 3)?;
        frames.push(ClassFrame {
            negated,
            union: None,
            difference: None,
        });
        Ok(())
    }

    fn finish_class(&mut self, frame: ClassFrame, offset: usize) -> Result<usize, Error> {
        let Some(set) = frame.union else {
            return Err(syntax(offset, "character group is empty"));
        };
        if frame.difference.is_some() {
            return Err(syntax(offset, "subtraction operand is missing"));
        }
        if frame.negated {
            self.set(Set::Complement(set))
        } else {
            Ok(set)
        }
    }

    fn class_member(&mut self, token: Token<'_>, offset: usize) -> Result<usize, Error> {
        if let Some(lo) = Self::scalar(&token)
            && token != Token::Literal('-')
            && self
                .peek()?
                .is_some_and(|next| next.token == Token::Literal('-'))
        {
            self.take()?;
            let Some(next) = self.peek()? else {
                return Err(syntax(offset, "unclosed character range"));
            };
            if matches!(next.token, Token::ClassClose | Token::Subtract) {
                let lo = self.range(lo, lo)?;
                let dash = self.range('-', '-')?;
                return self.set(Set::Union(lo, dash));
            }
            let hi = Self::scalar(&next.token)
                .filter(|_| next.token != Token::Literal('-'))
                .ok_or_else(|| {
                    syntax(
                        self.offset(next.span.start),
                        "range endpoint must be one character",
                    )
                })?;
            self.take()?;
            if lo > hi {
                return Err(syntax(offset, "character range is reversed"));
            }
            return self.range(lo, hi);
        }
        self.member(token, offset)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    fn accepted(profile: Profile, source: &str, flags: &str) -> CompiledPattern {
        compile(profile, source, flags, Limits::new())
            .unwrap_or_else(|error| panic!("{profile:?} {source:?} {flags:?}: {error}"))
    }

    #[test]
    fn dated_grammar_controls_groups_flags_backreferences_and_reluctance() {
        for profile in [Profile::Xpath20, Profile::Xpath31] {
            for source in [
                "",
                "|",
                "a|",
                "(a|b)*?",
                "(a)\\12",
                "(a)(b)\\2\\1",
                "a{0002,003}?",
                "^+$?",
            ] {
                accepted(profile, source, "ismxi");
            }
            for source in [
                "(a\\1)",
                "((?:a)\\1)",
                "(a)\\2",
                "*a",
                "a**",
                "a{,2}",
                "a{2,1}",
                "a{1",
                "a}",
                "(a",
                "a)",
                "(?i)a",
                r"\&",
                r"\~",
            ] {
                assert!(
                    matches!(
                        compile(profile, source, "", Limits::new()),
                        Err(Error::Syntax { .. })
                    ),
                    "{source}"
                );
            }
        }
        accepted(Profile::Xpath31, "(?:a)(b)\\1", "");
        assert!(matches!(
            compile(Profile::Xpath20, "(?:a)", "", Limits::new()),
            Err(Error::Syntax { .. })
        ));
        accepted(Profile::Xpath31, r"[(\1.*?", "qsimx");
        assert!(matches!(
            compile(Profile::Xpath20, "literal", "q", Limits::new()),
            Err(Error::Flags { .. })
        ));
        assert!(matches!(
            compile(Profile::Xpath31, "literal", "Q", Limits::new()),
            Err(Error::Flags { .. })
        ));
    }

    #[test]
    fn flag_errors_are_typed_and_operational_admission_has_precedence() {
        for (profile, flags, offset, flag) in [
            (Profile::Xpath20, "q", 0, 'q'),
            (Profile::Xpath31, "Q", 0, 'Q'),
            (Profile::Xpath31, "ié", 1, 'é'),
        ] {
            let error = compile(profile, "a", flags, Limits::new()).unwrap_err();
            assert_eq!(
                error,
                Error::Flags {
                    offset,
                    flag,
                    profile
                }
            );
            assert!(!error.is_operational());
            assert!(error.to_string().contains("FORX0001"));
            let source = compile(
                profile,
                "a",
                flags,
                Limits::new().with(Resource::PatternBytes, 0),
            )
            .unwrap_err();
            assert!(source.is_operational());
            assert!(matches!(
                source,
                Error::Resource(super::super::Refusal {
                    resource: Resource::PatternBytes,
                    ..
                })
            ));
            let work = compile(
                profile,
                "a",
                flags,
                Limits::new().with(Resource::CompileSteps, 0),
            )
            .unwrap_err();
            assert!(work.is_operational());
            assert!(matches!(
                work,
                Error::Resource(super::super::Refusal {
                    resource: Resource::CompileSteps,
                    ..
                })
            ));
        }
    }

    #[test]
    fn class_grammar_has_nested_subtraction_exact_properties_and_ranges() {
        for source in [
            r"[a-z-[aeiou]]",
            r"[^a-[b-[c]]]",
            r"[-a]",
            r"[a-]",
            r"[a--[b]]",
            r"[\--a]",
            r"[a&&b~~c]",
            r"[\i\I\c\C\s\S\w\W\d\D]",
            r"[\p{Lu}\P{IsGreekandCoptic}]",
            r"\p{IsHighSurrogates}",
            r"[\n-\r]",
        ] {
            accepted(Profile::Xpath31, source, "i");
        }
        for source in [
            r"[]",
            r"[z-a]",
            r"[a-b-c]",
            r"[a-\d]",
            r"[\d-a]",
            r"[a-[b]c]",
            r"[a[b]]",
            r"\p{Greek}",
            r"\p{Script=Greek}",
            r"\p{IsBasic_Latin}",
            r"\p{isBasicLatin}",
            r"\p{Lu",
        ] {
            assert!(
                matches!(
                    compile(Profile::Xpath31, source, "", Limits::new()),
                    Err(Error::Syntax { .. })
                ),
                "{source}"
            );
        }
        let program = accepted(Profile::Xpath31, r"[^A-Z-[IO]]", "i");
        assert!(matches!(program.sets.last(), Some(Set::Difference(..))));
        let program = accepted(Profile::Xpath31, r"\p{Lu}", "i");
        assert!(matches!(program.sets.as_slice(), [Set::Table(_)]));
    }

    #[test]
    fn arbitrary_decimal_quantities_keep_order_without_syntax_overflow() {
        let large = "18446744073709551616";
        let program = accepted(Profile::Xpath31, &format!("a{{{large},{large}}}"), "");
        assert!(matches!(
            program.nodes.last(),
            Some(Node::Repeat {
                min: Count::AboveU64,
                max: Some(Count::AboveU64),
                ..
            })
        ));
        accepted(
            Profile::Xpath20,
            &format!("a{{000{large},999999999999999999999999999999}}"),
            "",
        );
        accepted(
            Profile::Xpath31,
            "a{00000000000000000000000000000000000000002}",
            "",
        );
        assert!(matches!(
            compile(
                Profile::Xpath31,
                "a{9999999999999999999999999,18446744073709551616}",
                "",
                Limits::new()
            ),
            Err(Error::Syntax { .. })
        ));
    }

    #[test]
    fn whitespace_removal_uses_the_existing_cursor_and_preserves_raw_offsets() {
        let source = "é  a  * *";
        let error = compile(Profile::Xpath31, source, "x", Limits::new()).unwrap_err();
        assert!(matches!(error, Error::Syntax { offset: 9, .. }), "{error}");
        let program = accepted(Profile::Xpath31, r"hello\ sworld", "x");
        assert_eq!(program.source(), r"hello\ sworld");
        assert!(program.sets.iter().any(|set| matches!(set, Set::Space)));
        let quoted = accepted(Profile::Xpath31, r"hello\ sworld", "qx");
        assert_eq!(quoted.captures, 0);
        assert!(!quoted.sets.iter().any(|set| matches!(set, Set::Space)));
    }

    #[test]
    fn all_compiler_refusals_are_operational_and_cached_admission_is_current() {
        let source = "(a|b)+";
        let program = accepted(Profile::Xpath31, source, "");
        for (resource, required) in [
            (Resource::PatternBytes, source.len() as u64),
            (Resource::ProgramNodes, program.admission.nodes),
            (Resource::CompileSlots, program.admission.slots),
        ] {
            let high = Limits::new().with(resource, required);
            assert!(
                compile(Profile::Xpath31, source, "", high).is_ok(),
                "{resource:?}"
            );
            let low = high.with(resource, required - 1);
            assert!(
                matches!(
                    compile(Profile::Xpath31, source, "", low),
                    Err(Error::Resource(_))
                ),
                "{resource:?}"
            );
            assert!(
                matches!(program.admit(low), Err(Error::Resource(_))),
                "{resource:?}"
            );
        }
        assert!(matches!(
            compile(
                Profile::Xpath31,
                "[",
                "",
                Limits::new().with(Resource::PatternBytes, 0)
            ),
            Err(Error::Resource(_))
        ));
        assert!(matches!(
            compile(
                Profile::Xpath31,
                source,
                "",
                Limits::new().with(Resource::CompileSteps, 0)
            ),
            Err(Error::Resource(_))
        ));
    }

    #[test]
    fn nested_groups_and_subtractions_construct_and_drop_on_a_small_native_stack() {
        purrdf_stack::on_stack(256 * 1024, || {
            let groups = format!("{}a{}", "(".repeat(6000), ")".repeat(6000));
            let program = accepted(Profile::Xpath31, &groups, "");
            assert_eq!(program.capture_count(), 6000);
            drop(program);
            let classes = format!("{}a{}", "[a-".repeat(3000), "]".repeat(3000));
            let program = accepted(Profile::Xpath31, &classes, "");
            assert!(program.sets.len() > 3000);
            drop(program);
        })
        .unwrap();
    }
}
