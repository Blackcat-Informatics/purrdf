// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The pull reader, and the tree, skip and occurrence readers built on it.

use std::borrow::Cow;
use std::collections::HashSet;

use purrdf_hash::fixed::FixedState;
use std::ops::Range;

use super::number::{number_end, number_end_observed};
use super::{Error, ErrorKind, Number, Object, Value};
use crate::json_escape::{self, JsonEscapeErrorKind};
use crate::scan::find_first_json_string_special;
use crate::terminals::{is_json_string_forbidden_byte, is_ws, skip_ws};

/// The clean run, in bytes, a string body is scanned a byte at a time before
/// the chunked scan ([`find_first_json_string_special`]) takes over: one scan
/// chunk. A run shorter than a chunk is all tail to the chunked scan, which
/// answers its tail a byte at a time too, so the byte walk here does the same
/// tests without the call; a member name or a short value ends inside it.
const SHORT_RUN: usize = 16;

/// The whitespace run, in bytes, past which [`Reader`] hands the rest of the
/// run to the chunked scan: one scan chunk.
const WS_RUN: usize = 16;
const PROGRESS_CHUNK: usize = 256;

/// Borrowed input-work capability, without access to the reader or its caller.
/// The reader polls inside long lexical runs as well as between value events.
pub trait ReadObserver {
    /// Admit the next scanned byte chunk. `false` latches an interrupted read.
    fn advance(&mut self, bytes: usize) -> bool;
}

/// The resource bounds of one read.
///
/// Every bound is explicit: [`Limits::DEFAULT`] states each figure, and a
/// caller that wants a bound lifted says so with `usize::MAX` or `u64::MAX`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// The most arrays and objects open at once. `0` admits scalars only, `1`
    /// admits `[1]` and refuses `[[1]]`.
    pub max_depth: usize,
    /// The most value occurrences, the top-level value included. Member names
    /// are not values.
    pub max_values: u64,
    /// The most bytes one decoded string or member name may hold. Enforced
    /// wherever the reader decodes a string: [`Reader::read_value`], [`read`]
    /// and their callers.
    pub max_string_bytes: usize,
    /// Refuse an object that repeats a member name, at the repeat (RFC 7493
    /// §2.3). Names are compared after their escapes are decoded. Enforced by
    /// [`Reader::read_value`] and its callers.
    pub unique_members: bool,
}

impl Limits {
    /// 128 open containers, and no bound on values or string length;
    /// repeated member names are kept.
    pub const DEFAULT: Self = Self {
        max_depth: 128,
        max_values: u64::MAX,
        max_string_bytes: usize::MAX,
        unique_members: false,
    };

    /// [`Limits::DEFAULT`] with `max_depth` open containers.
    pub const fn with_depth(max_depth: usize) -> Self {
        Self {
            max_depth,
            ..Self::DEFAULT
        }
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A JSON value's syntactic kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// `null`.
    Null,
    /// `true`.
    True,
    /// `false`.
    False,
    /// A number.
    Number,
    /// A string.
    String,
    /// An array.
    Array,
    /// An object.
    Object,
}

impl Kind {
    /// The kind's name: `null`, `true`, `false`, `number`, `string`, `array` or
    /// `object`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::True => "true",
            Self::False => "false",
            Self::Number => "number",
            Self::String => "string",
            Self::Array => "array",
            Self::Object => "object",
        }
    }

    /// Whether the kind is a scalar (neither an array nor an object).
    pub const fn is_scalar(self) -> bool {
        !matches!(self, Self::Array | Self::Object)
    }
}

/// A string token: its body as written (escapes undecoded) and where it is.
///
/// The reader records whether the body holds an escape while it scans it, so
/// [`Str::has_escapes`] is a field read and [`Str::decode`] of a body without
/// one borrows it without looking at its bytes again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Str<'a> {
    raw: &'a str,
    at: usize,
    escaped: bool,
}

impl<'a> Str<'a> {
    /// The body between the quotes, escapes as written.
    pub const fn raw(&self) -> &'a str {
        self.raw
    }

    /// The byte span of the body, quotes excluded.
    pub const fn span(&self) -> Range<usize> {
        self.at..self.at + self.raw.len()
    }

    /// Whether the body holds an escape.
    pub const fn has_escapes(&self) -> bool {
        self.escaped
    }

    /// The body with every escape decoded, borrowed when it holds none.
    ///
    /// The reader validated the escape SYNTAX; this is where a `\u` escape that
    /// names an unpaired surrogate is refused, since it denotes no character.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Escape`] at the escape's byte in the document.
    pub fn decode(&self) -> Result<Cow<'a, str>, Error> {
        if !self.escaped {
            return Ok(Cow::Borrowed(self.raw));
        }
        json_escape::unescape(self.raw)
            .map_err(|error| Error::new(ErrorKind::Escape(error.kind), self.at + error.offset))
    }
}

/// One step of a document, in order. Each event names the bytes it covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event<'a> {
    /// `{`, at this offset.
    BeginObject {
        /// The offset of `{`.
        at: usize,
    },
    /// `}`, at this offset.
    EndObject {
        /// The offset of `}`.
        at: usize,
    },
    /// `[`, at this offset.
    BeginArray {
        /// The offset of `[`.
        at: usize,
    },
    /// `]`, at this offset.
    EndArray {
        /// The offset of `]`.
        at: usize,
    },
    /// A member name; the `:` after it has been read.
    Key(Str<'a>),
    /// A string value.
    String(Str<'a>),
    /// A number value.
    Number {
        /// The lexeme, grammar-checked.
        lexeme: &'a str,
        /// Its offset.
        at: usize,
    },
    /// `true` or `false`.
    Bool {
        /// Which.
        value: bool,
        /// Its offset.
        at: usize,
    },
    /// `null`.
    Null {
        /// Its offset.
        at: usize,
    },
    /// The document is complete, and nothing but whitespace follows it.
    End,
}

/// What the reader expects at the cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Expect {
    /// A value.
    Value,
    /// The first element of an array, or `]`.
    FirstItem,
    /// The first member of an object, or `}`.
    FirstMember,
    /// A member name (after a comma).
    Member,
    /// A comma or the innermost container's close, or the end of the document.
    Next,
    /// Nothing: [`Event::End`] has been returned.
    Done,
}

/// A pull reader over one JSON document.
///
/// [`Reader::next_event`] hands the document over one [`Event`] at a time;
/// [`Reader::next_key`], [`Reader::next_item`], [`Reader::skip_value`] and
/// [`Reader::read_value`] are the structural steps a streaming decoder takes
/// with it, so a caller can materialize the part of a document it wants and
/// syntax-check the rest without building it.
///
/// The open containers are a heap stack, so no step spends a machine-stack
/// frame per nesting level; [`Limits::max_depth`] bounds that stack.
pub struct Reader<'a> {
    text: &'a str,
    pos: usize,
    limits: Limits,
    /// One entry per open container: `true` for an object.
    open: Vec<bool>,
    expect: Expect,
    values: u64,
    observer: Option<&'a mut dyn ReadObserver>,
    observed: usize,
    interrupted: bool,
}
purrdf_hash::debug_non_exhaustive!(Reader<'_> { pos, limits, values, interrupted });

impl<'a> Reader<'a> {
    /// A reader over `text`, bounded by `limits`.
    pub fn new(text: &'a str, limits: Limits) -> Self {
        Self {
            text,
            pos: 0,
            limits,
            open: Vec::new(),
            expect: Expect::Value,
            values: 0,
            observer: None,
            observed: 0,
            interrupted: false,
        }
    }

    /// A reader with bounded progress polling, including long strings,
    /// numbers and whitespace. Completed events retain their ordinary bytes.
    pub fn new_observed(text: &'a str, limits: Limits, observer: &'a mut dyn ReadObserver) -> Self {
        Self {
            observer: Some(observer),
            ..Self::new(text, limits)
        }
    }

    fn progress(&mut self, force: bool) -> Result<(), Error> {
        if self.interrupted {
            return Err(self.error(ErrorKind::Interrupted));
        }
        let bytes = self.pos.saturating_sub(self.observed);
        if (force || bytes >= PROGRESS_CHUNK)
            && let Some(observer) = self.observer.as_deref_mut()
        {
            self.observed = self.pos;
            if !observer.advance(bytes) {
                self.interrupted = true;
                return Err(self.error(ErrorKind::Interrupted));
            }
        }
        Ok(())
    }

    /// A reader over `bytes`, refused unless they are UTF-8 (RFC 8259 §8.1).
    ///
    /// # Errors
    ///
    /// [`ErrorKind::InvalidUtf8`] at the first byte that is not.
    pub fn from_slice(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        let text = core::str::from_utf8(bytes)
            .map_err(|error| Error::new(ErrorKind::InvalidUtf8, error.valid_up_to()))?;
        Ok(Self::new(text, limits))
    }

    /// The byte offset of the cursor.
    pub const fn offset(&self) -> usize {
        self.pos
    }

    /// How many arrays and objects are open.
    pub fn depth(&self) -> usize {
        self.open.len()
    }

    /// The document.
    pub const fn text(&self) -> &'a str {
        self.text
    }

    const fn error(&self, kind: ErrorKind) -> Error {
        Error::new(kind, self.pos)
    }

    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.pos).copied()
    }

    /// Cross the whitespace at the cursor. RFC 8259 §2 lists four whitespace
    /// bytes ([`is_ws`]), never a Unicode property.
    ///
    /// Between two tokens there is usually no whitespace at all, or a byte or
    /// two of it, so the run is crossed here a byte at a time and the chunked
    /// scan ([`skip_ws`]) takes over only once the run is [`WS_RUN`] bytes long:
    /// a pretty-printer's indentation. A shorter run is all tail to the chunked
    /// scan, answered a byte at a time there too, so crossing it here saves
    /// only the call.
    #[allow(
        clippy::inline_always,
        reason = "crossed before every token: the zero- and one-byte runs must cost a compare \
                  in the caller, not a call"
    )]
    #[inline(always)]
    fn skip_whitespace(&mut self) {
        if self.observer.is_some() {
            loop {
                let end = self.pos.saturating_add(PROGRESS_CHUNK).min(self.text.len());
                let length = end - self.pos;
                let consumed = skip_ws(&self.text.as_bytes()[self.pos..end], 0);
                self.pos += consumed;
                if self.progress(false).is_err() || consumed < length || self.pos == self.text.len()
                {
                    return;
                }
            }
        }
        let bytes = self.text.as_bytes();
        let mut pos = self.pos;
        while let Some(&byte) = bytes.get(pos) {
            if !is_ws(byte) {
                self.pos = pos;
                return;
            }
            pos += 1;
            if pos - self.pos == WS_RUN {
                self.pos = skip_ws(bytes, pos);
                return;
            }
        }
        self.pos = pos;
    }

    /// The expectation after a value completes.
    const fn after_value(&mut self) {
        self.expect = Expect::Next;
    }

    /// The next event.
    ///
    /// After [`Event::End`] every further call returns [`Event::End`] again.
    ///
    /// # Errors
    ///
    /// [`Error`] at the first byte the grammar or a [`Limits`] bound refuses.
    #[inline]
    pub fn next_event(&mut self) -> Result<Event<'a>, Error> {
        self.step()
    }

    /// [`Reader::next_event`], inlined into the loops of this module that
    /// build or check a value, so the event it returns never passes through
    /// memory.
    #[allow(
        clippy::inline_always,
        reason = "the tree and check loops are one event loop each; the step must be inlined \
                  into them so the event stays in registers"
    )]
    #[inline(always)]
    fn step(&mut self) -> Result<Event<'a>, Error> {
        loop {
            self.skip_whitespace();
            self.progress(false)?;
            self.progress(self.pos == self.text.len())?;
            match self.expect {
                Expect::Done => return Ok(Event::End),
                Expect::Value => return self.value(),
                Expect::FirstItem => {
                    if self.peek() == Some(b']') {
                        return Ok(self.close(false));
                    }
                    return self.value();
                }
                Expect::FirstMember => {
                    if self.peek() == Some(b'}') {
                        return Ok(self.close(true));
                    }
                    return self.key();
                }
                Expect::Member => return self.key(),
                Expect::Next => match self.open.last().copied() {
                    None => {
                        if self.pos == self.text.len() {
                            self.expect = Expect::Done;
                            return Ok(Event::End);
                        }
                        return Err(self.error(ErrorKind::Trailing));
                    }
                    Some(object) => match self.peek() {
                        Some(b',') => {
                            self.pos += 1;
                            self.expect = if object {
                                Expect::Member
                            } else {
                                Expect::Value
                            };
                        }
                        Some(b'}') if object => return Ok(self.close(true)),
                        Some(b']') if !object => return Ok(self.close(false)),
                        _ => {
                            return Err(self.error(ErrorKind::Expected(if object {
                                "`,` or `}` in an object"
                            } else {
                                "`,` or `]` in an array"
                            })));
                        }
                    },
                },
            }
        }
    }

    /// Close the innermost container at the cursor's `]` or `}`.
    #[inline]
    fn close(&mut self, object: bool) -> Event<'a> {
        let at = self.pos;
        self.pos += 1;
        self.open.pop();
        self.after_value();
        if object {
            Event::EndObject { at }
        } else {
            Event::EndArray { at }
        }
    }

    /// The value at the cursor: a scalar whole, or a container opened.
    #[allow(
        clippy::inline_always,
        reason = "one arm of the event step, inlined into it with the step"
    )]
    #[inline(always)]
    fn value(&mut self) -> Result<Event<'a>, Error> {
        if self.values >= self.limits.max_values {
            return Err(self.error(ErrorKind::Values {
                limit: self.limits.max_values,
            }));
        }
        let at = self.pos;
        let event = match self.peek() {
            Some(open @ (b'{' | b'[')) => {
                if self.open.len() >= self.limits.max_depth {
                    return Err(self.error(ErrorKind::Depth {
                        limit: self.limits.max_depth,
                    }));
                }
                self.pos += 1;
                let object = open == b'{';
                self.open.push(object);
                self.values += 1;
                if object {
                    self.expect = Expect::FirstMember;
                    return Ok(Event::BeginObject { at });
                }
                self.expect = Expect::FirstItem;
                return Ok(Event::BeginArray { at });
            }
            Some(b'"') => Event::String(self.string()?),
            Some(b't') => {
                self.keyword("true")?;
                Event::Bool { value: true, at }
            }
            Some(b'f') => {
                self.keyword("false")?;
                Event::Bool { value: false, at }
            }
            Some(b'n') => {
                self.keyword("null")?;
                Event::Null { at }
            }
            Some(b'-' | b'0'..=b'9') => {
                self.pos = if self.observer.is_some() {
                    number_end_observed(self.text.as_bytes(), at, &mut |offset| {
                        self.pos = offset;
                        self.progress(false)
                    })?
                } else {
                    number_end(self.text.as_bytes(), at)?
                };
                Event::Number {
                    lexeme: &self.text[at..self.pos],
                    at,
                }
            }
            _ => return Err(self.error(ErrorKind::Expected("a JSON value"))),
        };
        self.values += 1;
        self.after_value();
        Ok(event)
    }

    /// A member name and the `:` after it.
    #[allow(
        clippy::inline_always,
        reason = "one arm of the event step, inlined into it with the step"
    )]
    #[inline(always)]
    fn key(&mut self) -> Result<Event<'a>, Error> {
        if self.peek() != Some(b'"') {
            return Err(self.error(ErrorKind::Expected("a `\"`-quoted member name")));
        }
        let name = self.string()?;
        self.skip_whitespace();
        self.progress(false)?;
        if self.peek() != Some(b':') {
            return Err(self.error(ErrorKind::Expected("`:` after a member name")));
        }
        self.pos += 1;
        self.expect = Expect::Value;
        Ok(Event::Key(name))
    }

    fn keyword(&mut self, word: &'static str) -> Result<(), Error> {
        if !self.text.as_bytes()[self.pos..].starts_with(word.as_bytes()) {
            return Err(self.error(ErrorKind::Expected(match word {
                "true" => "`true`",
                "false" => "`false`",
                _ => "`null`",
            })));
        }
        self.pos += word.len();
        Ok(())
    }

    /// One string, the cursor on its opening quote: its body, escape syntax
    /// checked and escapes undecoded.
    ///
    /// Each clean run — everything up to the next `"`, `\` or C0 control, which
    /// RFC 8259 §7 calls `unescaped` — is crossed by a test of exactly that
    /// class: a byte at a time for the first [`SHORT_RUN`] bytes, then one
    /// chunked scan; only the byte it stops at takes the per-byte grammar. The
    /// text is a `&str`, so a run needs no UTF-8 check, and the class holds only
    /// ASCII bytes, so the scan always stops on a `char` boundary.
    fn string(&mut self) -> Result<Str<'a>, Error> {
        let bytes = self.text.as_bytes();
        self.pos += 1;
        let start = self.pos;
        let mut escaped = false;
        loop {
            let rest = &bytes[self.pos..];
            self.pos += match rest
                .iter()
                .take(SHORT_RUN)
                .position(|&byte| is_json_string_forbidden_byte(byte))
            {
                Some(run) => run,
                None if rest.len() <= SHORT_RUN => rest.len(),
                None => {
                    let end = if self.observer.is_some() {
                        rest.len().min(PROGRESS_CHUNK)
                    } else {
                        rest.len()
                    };
                    let long = &rest[SHORT_RUN..end];
                    SHORT_RUN + find_first_json_string_special(long).unwrap_or(long.len())
                }
            };
            self.progress(false)?;
            if self.observer.is_some()
                && self
                    .peek()
                    .is_some_and(|byte| !is_json_string_forbidden_byte(byte))
            {
                continue;
            }
            match self.peek() {
                None => {
                    return Err(self.error(ErrorKind::Expected("the `\"` that closes a string")));
                }
                Some(b'"') => {
                    let raw = &self.text[start..self.pos];
                    self.pos += 1;
                    self.progress(true)?;
                    return Ok(Str {
                        raw,
                        at: start,
                        escaped,
                    });
                }
                Some(b'\\') => match bytes.get(self.pos + 1) {
                    Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => {
                        escaped = true;
                        self.pos += 2;
                    }
                    Some(b'u') => {
                        escaped = true;
                        let digits = self.pos + 2;
                        json_escape::code_unit(&bytes[digits..]).map_err(|error| {
                            Error::new(ErrorKind::Escape(error.kind), digits + error.offset)
                        })?;
                        self.pos = digits + 4;
                    }
                    None => {
                        return Err(self.error(ErrorKind::Escape(JsonEscapeErrorKind::Truncated)));
                    }
                    Some(_) => {
                        return Err(self.error(ErrorKind::Escape(JsonEscapeErrorKind::BadEscape)));
                    }
                },
                // The scan stops only at `"`, `\` and the C0 controls.
                Some(_) => return Err(self.error(ErrorKind::RawControl)),
            }
        }
    }

    /// The kind of the value that begins at the cursor, without reading it:
    /// `None` where no value begins (the reader's next step refuses that byte,
    /// or closes a container, or the document has ended).
    ///
    /// Asked where a value is expected — after a member name from
    /// [`Reader::next_key`], after [`Reader::next_item`] has returned `true`,
    /// or before the document's value — this is what lets a streaming decoder
    /// take a value it wants whole and hand one it does not to
    /// [`Reader::check_value`] or [`Reader::skip_value`]. It reads only the
    /// value's first byte: a `t` is [`Kind::True`] before the reader has
    /// checked that `true` follows.
    ///
    /// ```rust
    /// use purrdf_lex::json::{Kind, Limits, Reader};
    ///
    /// let mut reader = Reader::new(r#"{"a": [1], "b": "x"}"#, Limits::DEFAULT);
    /// reader.begin_object().unwrap();
    /// reader.next_key().unwrap();
    /// assert_eq!(reader.peek_kind(), Some(Kind::Array));
    /// reader.check_value().unwrap();
    /// reader.next_key().unwrap();
    /// assert_eq!(reader.peek_kind(), Some(Kind::String));
    /// ```
    pub fn peek_kind(&mut self) -> Option<Kind> {
        self.skip_whitespace();
        if self.progress(false).is_err() {
            return None;
        }
        match self.peek()? {
            b'{' => Some(Kind::Object),
            b'[' => Some(Kind::Array),
            b'"' => Some(Kind::String),
            b't' => Some(Kind::True),
            b'f' => Some(Kind::False),
            b'n' => Some(Kind::Null),
            b'-' | b'0'..=b'9' => Some(Kind::Number),
            _ => None,
        }
    }

    /// Inside an object: the next member's name (its `:` read), or `None` once
    /// the object's `}` has been read.
    ///
    /// # Errors
    ///
    /// [`Error`] at a grammar refusal, or where the reader is not between an
    /// object's members.
    pub fn next_key(&mut self) -> Result<Option<Str<'a>>, Error> {
        if !matches!(
            self.expect,
            Expect::FirstMember | Expect::Member | Expect::Next
        ) || self.open.last() != Some(&true)
        {
            return Err(self.error(ErrorKind::Expected("an object member")));
        }
        match self.next_event()? {
            Event::Key(name) => Ok(Some(name)),
            Event::EndObject { .. } => Ok(None),
            _ => Err(self.error(ErrorKind::Expected("an object member"))),
        }
    }

    /// Inside an array: `true` with the cursor on the next element, or `false`
    /// once the array's `]` has been read.
    ///
    /// # Errors
    ///
    /// [`Error`] at a grammar refusal, or where the reader is not between an
    /// array's elements.
    pub fn next_item(&mut self) -> Result<bool, Error> {
        if self.open.last() != Some(&false) {
            return Err(self.error(ErrorKind::Expected("an array element")));
        }
        self.skip_whitespace();
        self.progress(false)?;
        match (self.expect, self.peek()) {
            (Expect::FirstItem | Expect::Next, Some(b']')) => {
                self.close(false);
                Ok(false)
            }
            (Expect::FirstItem, _) => {
                self.expect = Expect::Value;
                Ok(true)
            }
            (Expect::Next, Some(b',')) => {
                self.pos += 1;
                self.expect = Expect::Value;
                Ok(true)
            }
            (Expect::Next, _) => Err(self.error(ErrorKind::Expected("`,` or `]` in an array"))),
            _ => Err(self.error(ErrorKind::Expected("an array element"))),
        }
    }

    /// Read the next event and require it to open an object.
    ///
    /// # Errors
    ///
    /// [`Error`] at a grammar refusal, or `expected an object` where another
    /// value begins.
    pub fn begin_object(&mut self) -> Result<(), Error> {
        self.skip_whitespace();
        self.progress(false)?;
        let at = self.pos;
        match self.next_event()? {
            Event::BeginObject { .. } => Ok(()),
            _ => Err(Error::new(ErrorKind::Expected("an object"), at)),
        }
    }

    /// Read the next event and require it to open an array.
    ///
    /// # Errors
    ///
    /// [`Error`] at a grammar refusal, or `expected an array` where another
    /// value begins.
    pub fn begin_array(&mut self) -> Result<(), Error> {
        self.skip_whitespace();
        self.progress(false)?;
        let at = self.pos;
        match self.next_event()? {
            Event::BeginArray { .. } => Ok(()),
            _ => Err(Error::new(ErrorKind::Expected("an array"), at)),
        }
    }

    /// Syntax-check the next value without building it: its byte span.
    ///
    /// Strings are scanned but not decoded, so the string bound and
    /// [`Limits::unique_members`] do not apply here, nor is an unpaired
    /// surrogate escape refused; depth and value bounds do.
    /// [`Reader::check_value`] is the skip that gives [`Reader::read_value`]'s
    /// verdict.
    ///
    /// # Errors
    ///
    /// [`Error`] at a grammar or bound refusal.
    pub fn skip_value(&mut self) -> Result<Range<usize>, Error> {
        self.skip_whitespace();
        self.progress(false)?;
        let start = self.pos;
        let mut depth = 0_usize;
        loop {
            match self.next_event()? {
                Event::BeginObject { .. } | Event::BeginArray { .. } => depth += 1,
                Event::EndObject { .. } | Event::EndArray { .. } if depth > 0 => depth -= 1,
                Event::Key(_) if depth > 0 => {}
                Event::String(_)
                | Event::Number { .. }
                | Event::Bool { .. }
                | Event::Null { .. } => {}
                _ => return Err(Error::new(ErrorKind::Expected("a JSON value"), start)),
            }
            if depth == 0 {
                return Ok(start..self.pos);
            }
        }
    }

    /// Build the next value, however deep, over a heap stack.
    ///
    /// The values of every open array wait on one shared stack, and the members
    /// of every open object on another; a container's close moves its own
    /// suffix of that stack into a vector of exactly its length. A container
    /// therefore costs one allocation, not the doubling series a vector grown
    /// element by element pays, and a two-number position is one allocation of
    /// two values.
    ///
    /// # Errors
    ///
    /// [`Error`] at a grammar refusal, an unpaired surrogate, or a [`Limits`]
    /// bound.
    pub fn read_value(&mut self) -> Result<Value, Error> {
        /// An open container: where its elements or members begin on the
        /// shared stack, and which kind it is.
        #[derive(Clone, Copy)]
        struct Open {
            base: usize,
            object: bool,
        }
        let mut open: Vec<Open> = Vec::new();
        let mut items: Vec<Value> = Vec::new();
        let mut members: Vec<(String, Value)> = Vec::new();
        // The name of each open object's member whose value is being read.
        let mut names: Vec<String> = Vec::new();
        // Each open object's names so far, when repeats are refused.
        let mut seen: Vec<HashSet<String, FixedState>> = Vec::new();
        // A finished value goes to the innermost open container, or is the
        // result.
        macro_rules! finish {
            ($value:expr) => {
                match open.last() {
                    None => return Ok($value),
                    Some(Open { object: false, .. }) => {
                        items.push($value);
                    }
                    Some(Open { object: true, .. }) => {
                        let name = names.pop().expect("a member's value follows its name");
                        members.push((name, $value));
                    }
                }
            };
        }
        loop {
            match self.step()? {
                Event::BeginObject { .. } => {
                    open.push(Open {
                        base: members.len(),
                        object: true,
                    });
                    if self.limits.unique_members {
                        seen.push(HashSet::with_hasher(FixedState::new()));
                    }
                }
                Event::BeginArray { .. } => open.push(Open {
                    base: items.len(),
                    object: false,
                }),
                Event::Key(raw) => {
                    let decoded = self.decoded(raw)?.into_owned();
                    if let Some(seen) = seen.last_mut()
                        && !seen.insert(decoded.clone())
                    {
                        return Err(Error::new(ErrorKind::DuplicateMember, raw.at - 1));
                    }
                    names.push(decoded);
                }
                Event::String(raw) => {
                    let text = self.decoded(raw)?.into_owned();
                    finish!(Value::String(text));
                }
                Event::Number { lexeme, .. } => {
                    finish!(Value::Number(Number::from_valid(lexeme.to_owned())));
                }
                Event::Bool { value, .. } => finish!(Value::Bool(value)),
                Event::Null { .. } => finish!(Value::Null),
                Event::EndArray { .. } => {
                    let Some(Open { base, .. }) = open.pop() else {
                        unreachable!("`]` closes the array the reader opened");
                    };
                    let array = items.split_off(base);
                    finish!(Value::Array(array));
                }
                Event::EndObject { .. } => {
                    let Some(Open { base, .. }) = open.pop() else {
                        unreachable!("`}}` closes the object the reader opened");
                    };
                    if self.limits.unique_members {
                        seen.pop();
                    }
                    let object = members.split_off(base);
                    finish!(Value::Object(Object::from(object)));
                }
                Event::End => return Err(self.error(ErrorKind::Expected("a JSON value"))),
            }
        }
    }

    /// Check the next value exactly as [`Reader::read_value`] would, without
    /// building it: its byte span.
    ///
    /// Every string and member name is decoded and held to
    /// [`Limits::max_string_bytes`], and [`Limits::unique_members`] is
    /// enforced, so a value this accepts is one [`Reader::read_value`] reads
    /// and a value it refuses is refused with the same [`Error`]. A string
    /// without an escape is not looked at again: the scan that found its end
    /// already knows it has nothing to decode. This is the skip a streaming
    /// decoder that owes its caller the tree reader's verdict takes over the
    /// members it does not want; [`Reader::skip_value`] is the cheaper
    /// syntax-only skip.
    ///
    /// # Errors
    ///
    /// [`Error`] at a grammar refusal, an unpaired surrogate, or a [`Limits`]
    /// bound.
    pub fn check_value(&mut self) -> Result<Range<usize>, Error> {
        self.skip_whitespace();
        self.progress(false)?;
        let start = self.pos;
        self.check(start)?;
        Ok(start..self.pos)
    }

    /// The loop behind [`Reader::check_value`], for the value that starts at
    /// `start`.
    fn check(&mut self, start: usize) -> Result<(), Error> {
        // One entry per container open inside the value: the names its
        // members have used, when repeats are refused.
        let mut open: Vec<Option<HashSet<String, FixedState>>> = Vec::new();
        loop {
            match self.step()? {
                Event::BeginObject { .. } => open.push(
                    self.limits
                        .unique_members
                        .then(|| HashSet::with_hasher(FixedState::new())),
                ),
                Event::BeginArray { .. } => open.push(None),
                Event::EndObject { .. } | Event::EndArray { .. } => {
                    if open.pop().is_none() {
                        // The cursor was not on a value: a container closes.
                        return Err(Error::new(ErrorKind::Expected("a JSON value"), start));
                    }
                }
                Event::Key(raw) => {
                    let Some(names) = open.last_mut() else {
                        // The cursor was between an object's members.
                        return Err(Error::new(ErrorKind::Expected("a JSON value"), start));
                    };
                    let decoded = self.decoded(raw)?;
                    if let Some(names) = names
                        && !names.insert(decoded.into_owned())
                    {
                        return Err(Error::new(ErrorKind::DuplicateMember, raw.at - 1));
                    }
                    continue;
                }
                Event::String(raw) => {
                    self.decoded(raw)?;
                }
                Event::Number { .. } | Event::Bool { .. } | Event::Null { .. } => {}
                Event::End => return Err(Error::new(ErrorKind::Expected("a JSON value"), start)),
            }
            if open.is_empty() {
                return Ok(());
            }
        }
    }

    /// A string or member name, decoded (borrowed when it holds no escape)
    /// and held to the string bound.
    fn decoded(&self, raw: Str<'a>) -> Result<Cow<'a, str>, Error> {
        let decoded = raw.decode()?;
        if decoded.len() > self.limits.max_string_bytes {
            return Err(Error::new(
                ErrorKind::StringBytes {
                    limit: self.limits.max_string_bytes,
                },
                raw.at,
            ));
        }
        Ok(decoded)
    }

    /// Require the document to be complete: only whitespace after the
    /// top-level value.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Trailing`] at the first byte after the value that is not
    /// whitespace, or a grammar refusal where the value is not complete.
    pub fn finish(&mut self) -> Result<(), Error> {
        match self.next_event()? {
            Event::End => Ok(()),
            _ => Err(self.error(ErrorKind::Expected("the end of the document"))),
        }
    }
}

/// Read `text` as one JSON document under [`Limits::DEFAULT`].
///
/// ```rust
/// use purrdf_lex::json::{self, ErrorKind};
///
/// assert_eq!(json::read(" [1, \"\\ud83d\\ude00\"] ").unwrap()[1], "😀");
/// assert_eq!(json::read("1 2").unwrap_err().kind(), ErrorKind::Trailing);
/// ```
///
/// # Errors
///
/// [`Error`] at the first refusal.
pub fn read(text: &str) -> Result<Value, Error> {
    read_with(text, Limits::DEFAULT)
}

/// Read `text` as one JSON document under `limits`.
///
/// # Errors
///
/// [`Error`] at the first refusal.
pub fn read_with(text: &str, limits: Limits) -> Result<Value, Error> {
    let mut reader = Reader::new(text, limits);
    let value = reader.read_value()?;
    reader.finish()?;
    Ok(value)
}

/// Read `bytes` as one UTF-8 JSON document under `limits`.
///
/// # Errors
///
/// [`ErrorKind::InvalidUtf8`], or [`Error`] at the first refusal.
pub fn read_slice(bytes: &[u8], limits: Limits) -> Result<Value, Error> {
    let mut reader = Reader::from_slice(bytes, limits)?;
    let value = reader.read_value()?;
    reader.finish()?;
    Ok(value)
}

/// One value occurrence of a document, in preorder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Occurrence {
    /// The syntactic kind.
    pub kind: Kind,
    /// The lexical span: a string's body without its quotes, a container's
    /// whole syntax from its opening to its closing bracket, and otherwise the
    /// token.
    pub span: Range<usize>,
    /// The index of the enclosing container's occurrence; `None` for the
    /// top-level value.
    pub parent: Option<usize>,
    /// The zero-based position within the parent (member or element); `0` for
    /// the top-level value.
    pub ordinal: usize,
    /// A container's immediate member or element count, repeated names
    /// included; `0` for a scalar.
    pub size: usize,
    /// For an object member's value, the span of its name's body (escapes
    /// undecoded).
    pub key: Option<Range<usize>>,
}

/// Every value occurrence of `text`, in document preorder.
///
/// Repeated member names are separate occurrences, and nothing is decoded: a
/// caller that needs a member name as text decodes its [`Occurrence::key`]
/// span with [`crate::json_escape::unescape`].
///
/// ```rust
/// use purrdf_lex::json::{Kind, Limits, occurrences};
///
/// let all = occurrences(r#"{"a": [true], "a": "x"}"#, Limits::DEFAULT).unwrap();
/// assert_eq!(all.len(), 4);
/// assert_eq!((all[0].kind, all[0].size), (Kind::Object, 2));
/// assert_eq!((all[2].kind, all[2].parent, all[2].ordinal), (Kind::True, Some(1), 0));
/// assert_eq!((all[3].span.clone(), all[3].key.clone()), (20..21, Some(15..16)));
/// ```
///
/// # Errors
///
/// [`Error`] at the first refusal.
pub fn occurrences(text: &str, limits: Limits) -> Result<Vec<Occurrence>, Error> {
    let mut reader = Reader::new(text, limits);
    let mut all: Vec<Occurrence> = Vec::new();
    // The open containers: their occurrence index and child count so far.
    let mut open: Vec<(usize, usize)> = Vec::new();
    let mut key: Option<Range<usize>> = None;
    loop {
        let event = reader.next_event()?;
        let (kind, span) = match event {
            Event::End => return Ok(all),
            Event::Key(name) => {
                key = Some(name.span());
                continue;
            }
            Event::EndObject { at } | Event::EndArray { at } => {
                let (index, size) = open.pop().expect("a close pairs with an open");
                all[index].span.end = at + 1;
                all[index].size = size;
                continue;
            }
            Event::BeginObject { at } => (Kind::Object, at..at),
            Event::BeginArray { at } => (Kind::Array, at..at),
            Event::String(body) => (Kind::String, body.span()),
            Event::Number { lexeme, at } => (Kind::Number, at..at + lexeme.len()),
            Event::Bool { value, at } => {
                if value {
                    (Kind::True, at..at + 4)
                } else {
                    (Kind::False, at..at + 5)
                }
            }
            Event::Null { at } => (Kind::Null, at..at + 4),
        };
        let (parent, ordinal) = match open.last_mut() {
            Some((parent, count)) => {
                *count += 1;
                (Some(*parent), *count - 1)
            }
            None => (None, 0),
        };
        let index = all.len();
        all.push(Occurrence {
            kind,
            span,
            parent,
            ordinal,
            size: 0,
            key: key.take(),
        });
        if !kind.is_scalar() {
            open.push((index, 0));
        }
    }
}
