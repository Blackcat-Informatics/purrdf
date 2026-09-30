// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The pull reader, and the tree, skip and occurrence readers built on it.

use std::borrow::Cow;
use std::collections::HashSet;

use purrdf_hash::fixed::FixedState;
use std::ops::Range;

use super::number::number_end;
use super::{Error, ErrorKind, Number, Object, Value};
use crate::json_escape::{self, JsonEscapeErrorKind};
use crate::scan::find_first_json_string_special;
use crate::terminals::skip_ws;

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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Str<'a> {
    raw: &'a str,
    at: usize,
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
    pub fn has_escapes(&self) -> bool {
        self.raw.as_bytes().contains(&b'\\')
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
#[derive(Debug)]
pub struct Reader<'a> {
    text: &'a str,
    pos: usize,
    limits: Limits,
    /// One entry per open container: `true` for an object.
    open: Vec<bool>,
    expect: Expect,
    values: u64,
}

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
        }
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

    // RFC 8259 §2 lists four whitespace bytes, never a Unicode property.
    fn skip_whitespace(&mut self) {
        self.pos = skip_ws(self.text.as_bytes(), self.pos);
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
    pub fn next_event(&mut self) -> Result<Event<'a>, Error> {
        loop {
            self.skip_whitespace();
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
                self.pos = number_end(self.text.as_bytes(), at)?;
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
    fn key(&mut self) -> Result<Event<'a>, Error> {
        if self.peek() != Some(b'"') {
            return Err(self.error(ErrorKind::Expected("a `\"`-quoted member name")));
        }
        let name = self.string()?;
        self.skip_whitespace();
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
    /// RFC 8259 §7 calls `unescaped` — is crossed by one chunked scan of exactly
    /// that class; only the byte it stops at takes the per-byte grammar. The
    /// text is a `&str`, so a run needs no UTF-8 check, and the class holds only
    /// ASCII bytes, so the scan always stops on a `char` boundary.
    fn string(&mut self) -> Result<Str<'a>, Error> {
        let bytes = self.text.as_bytes();
        self.pos += 1;
        let start = self.pos;
        loop {
            let rest = &bytes[self.pos..];
            self.pos += find_first_json_string_special(rest).unwrap_or(rest.len());
            match self.peek() {
                None => {
                    return Err(self.error(ErrorKind::Expected("the `\"` that closes a string")));
                }
                Some(b'"') => {
                    let raw = &self.text[start..self.pos];
                    self.pos += 1;
                    return Ok(Str { raw, at: start });
                }
                Some(b'\\') => match bytes.get(self.pos + 1) {
                    Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => self.pos += 2,
                    Some(b'u') => {
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
        let at = self.pos;
        match self.next_event()? {
            Event::BeginArray { .. } => Ok(()),
            _ => Err(Error::new(ErrorKind::Expected("an array"), at)),
        }
    }

    /// Syntax-check the next value without building it: its byte span.
    ///
    /// Strings are scanned but not decoded, so the string bound and
    /// [`Limits::unique_members`] do not apply here; depth and value bounds do.
    ///
    /// # Errors
    ///
    /// [`Error`] at a grammar or bound refusal.
    pub fn skip_value(&mut self) -> Result<Range<usize>, Error> {
        self.skip_whitespace();
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
    /// # Errors
    ///
    /// [`Error`] at a grammar refusal, an unpaired surrogate, or a [`Limits`]
    /// bound.
    pub fn read_value(&mut self) -> Result<Value, Error> {
        enum Open {
            Array(Vec<Value>),
            Object {
                members: Object,
                name: Option<String>,
                seen: Option<HashSet<String, FixedState>>,
            },
        }
        let mut open: Vec<Open> = Vec::new();
        loop {
            let event = self.next_event()?;
            let value = match event {
                Event::BeginObject { .. } => {
                    open.push(Open::Object {
                        members: Object::new(),
                        name: None,
                        seen: self
                            .limits
                            .unique_members
                            .then(|| HashSet::with_hasher(FixedState::new())),
                    });
                    continue;
                }
                Event::BeginArray { .. } => {
                    open.push(Open::Array(Vec::new()));
                    continue;
                }
                Event::Key(raw) => {
                    let decoded = self.decoded(raw)?;
                    let Some(Open::Object { name, seen, .. }) = open.last_mut() else {
                        unreachable!("a member name is read only inside an object");
                    };
                    if let Some(seen) = seen
                        && !seen.insert(decoded.clone())
                    {
                        return Err(Error::new(ErrorKind::DuplicateMember, raw.at - 1));
                    }
                    *name = Some(decoded);
                    continue;
                }
                Event::String(raw) => Value::String(self.decoded(raw)?),
                Event::Number { lexeme, .. } => {
                    Value::Number(Number::from_valid(lexeme.to_owned()))
                }
                Event::Bool { value, .. } => Value::Bool(value),
                Event::Null { .. } => Value::Null,
                Event::EndArray { .. } => match open.pop() {
                    Some(Open::Array(items)) => Value::Array(items),
                    _ => unreachable!("`]` closes the array the reader opened"),
                },
                Event::EndObject { .. } => match open.pop() {
                    Some(Open::Object { members, .. }) => Value::Object(members),
                    _ => unreachable!("`}}` closes the object the reader opened"),
                },
                Event::End => return Err(self.error(ErrorKind::Expected("a JSON value"))),
            };
            match open.last_mut() {
                None => return Ok(value),
                Some(Open::Array(items)) => items.push(value),
                Some(Open::Object { members, name, .. }) => members.push(
                    name.take().expect("a member's value follows its name"),
                    value,
                ),
            }
        }
    }

    /// A string or member name, decoded and held to the string bound.
    fn decoded(&self, raw: Str<'a>) -> Result<String, Error> {
        let decoded = raw.decode()?;
        if decoded.len() > self.limits.max_string_bytes {
            return Err(Error::new(
                ErrorKind::StringBytes {
                    limit: self.limits.max_string_bytes,
                },
                raw.at,
            ));
        }
        Ok(decoded.into_owned())
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
