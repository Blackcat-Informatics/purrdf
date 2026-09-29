// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A complete RFC 8259 JSON reader and writer that keeps every number as the
//! text the document wrote.
//!
//! # Why this crate parses its own JSON
//!
//! `serde_json` — and every other general-purpose JSON reader — decides a number
//! at parse time, into `f64` (or `i64` when it fits). That single decision would
//! end this crate's exactness guarantee before any geometry code ran: a GeoJSON
//! coordinate such as `-83.42391749999999` would already have been rounded to the
//! nearest double by the time [`crate::geojson`] saw it, `0.1` and
//! `0.10000000000000000555` would have become the same value, and a
//! `wasm32-unknown-unknown` build and a native build could disagree about a
//! predicate that sits near a boundary. The crate root denies
//! `clippy::float_arithmetic` precisely so that no such path can be reintroduced,
//! and `str::parse::<f64>()` appears nowhere in `purrdf-geo`.
//!
//! So [`JsonValue::Number`] holds the **source lexeme verbatim**. The reader
//! validates the RFC 8259 number grammar and hands the text on unchanged;
//! deciding what the text denotes is the consumer's job, and
//! [`crate::exact::Rat::parse_decimal`] does it exactly, digit by digit, with
//! integer arithmetic alone.
//!
//! # Objects are ordered pairs, not a map
//!
//! RFC 8259 §4 permits an object to repeat a member name and says nothing about
//! which occurrence wins. A `HashMap`/`BTreeMap` representation therefore
//! *silently drops* one of them — and silently dropping data is exactly the
//! failure mode this repository hunts. [`JsonValue::Object`] keeps a `Vec` of
//! pairs in document order, so nothing is lost, [`JsonValue::get`] states the
//! first-match rule explicitly, and [`JsonValue::count`] lets a consumer notice
//! the ambiguity and refuse it (which [`crate::geojson`] does for the members
//! that decide a geometry).
//!
//! # Nesting is bounded by memory alone
//!
//! A document nests as deep as its author writes it. The reader keeps its open
//! arrays and objects on an explicit heap stack, the writer keeps its pending
//! values on another, and a [`JsonValue`] drops, clones, compares and prints over
//! work lists rather than the compiler's recursive glue (the `tree` module), so no
//! walk here spends a stack frame per level. Depth is bounded by memory and by
//! nothing else, identically on every host: a literal arrives from the dataset,
//! which is untrusted input, and a stack overflow would be an `abort` no host can
//! catch — but so would a fixed cap be a refusal of conforming data below any
//! figure that protected the stack, and there is no such figure to pick.

use core::mem;

use purrdf_iri::json_escape::{JsonEscapes, push_string};

use crate::error::GeoError;

/// A parsed JSON value.
///
/// [`Self::Number`] keeps the source lexeme verbatim, so a consumer can decide
/// the value exactly rather than inheriting a float's rounding; see the module
/// documentation for why that is the whole reason this reader exists.
///
/// A value nests as deep as its document writes it, so `Clone`, `PartialEq`,
/// `Debug` and `Drop` are the iterative impls in the `tree` module, not the
/// compiler's recursive glue.
#[derive(Eq)]
pub enum JsonValue {
    /// The literal `null`.
    Null,
    /// The literal `true` or `false`.
    Bool(bool),
    /// A number, as the exact characters the document wrote. Guaranteed by
    /// [`parse`] to match the RFC 8259 number grammar; nothing has interpreted
    /// it.
    Number(String),
    /// A string, with every escape already resolved.
    String(String),
    /// An array, in document order.
    Array(Vec<Self>),
    /// An object, as name/value pairs in document order. Names may repeat: RFC
    /// 8259 allows it, and dropping a repeat would be a silent loss.
    Object(Vec<(String, Self)>),
}

#[cfg(test)]
pub(crate) mod arbitrary;
mod tree;

impl JsonValue {
    /// The value of the **first** member named `name`, or `None` when this is not
    /// an object or has no such member.
    ///
    /// "First" is a deliberate, stated rule rather than an accident of a hash
    /// map's iteration order. A consumer for which a repeated name is an
    /// ambiguity rather than a shrug should ask [`Self::count`] and refuse.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Self> {
        match self {
            Self::Object(members) => members
                .iter()
                .find(|(member, _)| member == name)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    /// How many members are named `name`; `0` when this is not an object.
    #[must_use]
    pub fn count(&self, name: &str) -> usize {
        match self {
            Self::Object(members) => members.iter().filter(|(member, _)| member == name).count(),
            _ => 0,
        }
    }

    /// The name of this value's kind, for diagnostics: `null`, `a boolean`, `a
    /// number`, `a string`, `an array` or `an object`.
    #[must_use]
    pub const fn kind_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "a boolean",
            Self::Number(_) => "a number",
            Self::String(_) => "a string",
            Self::Array(_) => "an array",
            Self::Object(_) => "an object",
        }
    }
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

/// Parse `text` as a single RFC 8259 JSON document.
///
/// # Errors
///
/// [`GeoError::Literal`] naming the byte offset and what was expected there, for
/// any departure from RFC 8259: a malformed number (a leading `+`, a leading
/// zero, a bare `.`, `NaN`, `Infinity`), an unterminated string, array or
/// object, a trailing comma, an unescaped control character in a string, an
/// unpaired UTF-16 surrogate, or content after the top-level value.
pub fn parse(text: &str) -> Result<JsonValue, GeoError> {
    let mut parser = Parser {
        text,
        bytes: text.as_bytes(),
        pos: 0,
    };
    parser.skip_whitespace();
    let value = parser.document()?;
    parser.skip_whitespace();
    if parser.pos == text.len() {
        Ok(value)
    } else {
        Err(parser.expected(parser.pos, "the end of the text after the top-level value"))
    }
}

/// The reader. Positions are byte offsets into `text`; every structural character
/// JSON has is ASCII, so a byte offset the reader stops at is always a `char`
/// boundary.
struct Parser<'a> {
    text: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

/// An array or object whose `[` or `{` has been read and whose members are still
/// being read.
enum Open {
    /// The items read so far.
    Array(Vec<JsonValue>),
    /// The members read so far, and the name of the member whose value is being
    /// read.
    Object {
        members: Vec<(String, JsonValue)>,
        name: String,
    },
}

impl Open {
    /// The value the closed container denotes.
    fn close(self) -> JsonValue {
        match self {
            Self::Array(items) => JsonValue::Array(items),
            Self::Object { members, .. } => JsonValue::Object(members),
        }
    }
}

/// What reading at the cursor produced: a whole value, or a container opened onto
/// the stack whose first member is next.
enum Read {
    Value(JsonValue),
    Opened,
}

impl Parser<'_> {
    /// A refusal naming where it happened, what was expected and what was there.
    fn expected(&self, at: usize, what: &str) -> GeoError {
        let found = match self.text.get(at..).and_then(|rest| rest.chars().next()) {
            Some(character) => format!("`{character}`"),
            None => "the end of the text".to_owned(),
        };
        GeoError::literal(format!("JSON at byte {at}: expected {what}, found {found}"))
    }

    /// RFC 8259 §2 whitespace, which is these four characters and nothing else.
    fn skip_whitespace(&mut self) {
        while let Some(&byte) = self.bytes.get(self.pos) {
            if matches!(byte, b' ' | b'\t' | b'\n' | b'\r') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Read the value at the cursor, with every array and object it nests.
    ///
    /// Iterative: the open containers live in an explicit heap stack, and the loop
    /// alternates between reading the next value and closing the containers a
    /// finished member completes. Nesting therefore costs heap and never stack, so
    /// a document is read however deep it nests, and a diagnostic anywhere inside it
    /// is the same diagnostic at the same byte as it would be at the top level.
    fn document(&mut self) -> Result<JsonValue, GeoError> {
        let mut open: Vec<Open> = Vec::new();
        loop {
            let mut finished = match self.value(&mut open)? {
                Read::Value(value) => value,
                Read::Opened => continue,
            };
            // A finished value is the document, or a member of the innermost open
            // container: it is appended, and either the container continues with
            // the next member or it closes and is itself the finished value one
            // level up.
            loop {
                let Some(top) = open.last_mut() else {
                    return Ok(finished);
                };
                if self.append(top, finished)? {
                    break;
                }
                finished = open
                    .pop()
                    .expect("the container just read is still open")
                    .close();
            }
        }
    }

    /// Read one value at the cursor. A scalar is complete; an empty array or object
    /// is complete; any other array or object is opened onto `open` with the cursor
    /// on its first value.
    fn value(&mut self, open: &mut Vec<Open>) -> Result<Read, GeoError> {
        let start = self.pos;
        let value = match self.bytes.get(self.pos) {
            Some(b'n') => self.word("null", JsonValue::Null)?,
            Some(b't') => self.word("true", JsonValue::Bool(true))?,
            Some(b'f') => self.word("false", JsonValue::Bool(false))?,
            Some(b'"') => JsonValue::String(self.string()?),
            Some(b'[') => {
                self.pos += 1;
                self.skip_whitespace();
                if self.bytes.get(self.pos) == Some(&b']') {
                    self.pos += 1;
                    JsonValue::Array(Vec::new())
                } else {
                    open.push(Open::Array(Vec::new()));
                    return Ok(Read::Opened);
                }
            }
            Some(b'{') => {
                self.pos += 1;
                self.skip_whitespace();
                if self.bytes.get(self.pos) == Some(&b'}') {
                    self.pos += 1;
                    JsonValue::Object(Vec::new())
                } else {
                    let name = self.member_name()?;
                    open.push(Open::Object {
                        members: Vec::new(),
                        name,
                    });
                    return Ok(Read::Opened);
                }
            }
            Some(b'-' | b'0'..=b'9') => self.number()?,
            // `+1`, `.5`, `NaN` and `Infinity` all land here: none of them is a
            // JSON value, and naming the position is more useful than naming the
            // spelling the author probably meant.
            _ => return Err(self.expected(start, "a JSON value")),
        };
        Ok(Read::Value(value))
    }

    /// Append a finished `value` to the innermost open container and read what
    /// follows it: `Ok(true)` when another member does, the cursor on it (and, for
    /// an object, its name already read); `Ok(false)` when the container closed.
    fn append(&mut self, top: &mut Open, value: JsonValue) -> Result<bool, GeoError> {
        match top {
            Open::Array(items) => {
                items.push(value);
                self.skip_whitespace();
                match self.bytes.get(self.pos) {
                    Some(b',') => {
                        self.pos += 1;
                        self.skip_whitespace();
                        Ok(true)
                    }
                    Some(b']') => {
                        self.pos += 1;
                        Ok(false)
                    }
                    // Includes the unterminated case (`None`) and the trailing
                    // comma, which reappears on the next turn as "expected a JSON
                    // value".
                    _ => Err(self.expected(self.pos, "`,` or `]` in an array")),
                }
            }
            Open::Object { members, name } => {
                members.push((mem::take(name), value));
                self.skip_whitespace();
                match self.bytes.get(self.pos) {
                    Some(b',') => {
                        self.pos += 1;
                        self.skip_whitespace();
                        *name = self.member_name()?;
                        Ok(true)
                    }
                    Some(b'}') => {
                        self.pos += 1;
                        Ok(false)
                    }
                    _ => Err(self.expected(self.pos, "`,` or `}` in an object")),
                }
            }
        }
    }

    /// A member's quoted name and the `:` after it, the cursor left on its value.
    fn member_name(&mut self) -> Result<String, GeoError> {
        if self.bytes.get(self.pos) != Some(&b'"') {
            return Err(self.expected(self.pos, "a `\"`-quoted member name"));
        }
        let name = self.string()?;
        self.skip_whitespace();
        if self.bytes.get(self.pos) != Some(&b':') {
            return Err(self.expected(self.pos, "`:` after a member name"));
        }
        self.pos += 1;
        self.skip_whitespace();
        Ok(name)
    }

    fn word(&mut self, word: &str, value: JsonValue) -> Result<JsonValue, GeoError> {
        let start = self.pos;
        if self.bytes[start..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Ok(value)
        } else {
            Err(self.expected(start, &format!("`{word}`")))
        }
    }

    /// A string, with escapes resolved. The caller has already established that
    /// the current byte is `"`.
    fn string(&mut self) -> Result<String, GeoError> {
        self.pos += 1;
        let mut out = String::new();
        // The start of the run of bytes that need no processing. Copying runs
        // rather than characters keeps the common (escape-free) string one
        // `push_str`.
        let mut chunk = self.pos;
        loop {
            let Some(&byte) = self.bytes.get(self.pos) else {
                return Err(self.expected(self.pos, "the `\"` that closes a string"));
            };
            match byte {
                b'"' => {
                    out.push_str(&self.text[chunk..self.pos]);
                    self.pos += 1;
                    return Ok(out);
                }
                b'\\' => {
                    out.push_str(&self.text[chunk..self.pos]);
                    self.pos += 1;
                    self.escape(&mut out)?;
                    chunk = self.pos;
                }
                0x00..=0x1F => {
                    return Err(self.expected(
                        self.pos,
                        "a printable character; RFC 8259 requires a control character in a string \
                         to be escaped",
                    ));
                }
                // Every continuation byte of a multi-byte UTF-8 sequence is
                // >= 0x80 and falls here, so runs stay on `char` boundaries.
                _ => self.pos += 1,
            }
        }
    }

    /// One escape sequence, the backslash already consumed.
    fn escape(&mut self, out: &mut String) -> Result<(), GeoError> {
        let at = self.pos;
        let Some(&byte) = self.bytes.get(at) else {
            return Err(self.expected(at, "an escape character after `\\`"));
        };
        self.pos += 1;
        let short = match byte {
            b'"' => Some('"'),
            b'\\' => Some('\\'),
            b'/' => Some('/'),
            b'b' => Some('\u{8}'),
            b'f' => Some('\u{c}'),
            b'n' => Some('\n'),
            b'r' => Some('\r'),
            b't' => Some('\t'),
            b'u' => None,
            _ => {
                return Err(self.expected(
                    at,
                    "one of `\"`, `\\`, `/`, `b`, `f`, `n`, `r`, `t` \
                                              or `u` after `\\`",
                ));
            }
        };
        if let Some(character) = short {
            out.push(character);
            return Ok(());
        }
        out.push(self.escaped_char()?);
        Ok(())
    }

    /// The character a `\u` escape denotes, consuming a following low surrogate
    /// when the first unit is a high surrogate.
    fn escaped_char(&mut self) -> Result<char, GeoError> {
        const HIGH: core::ops::RangeInclusive<u16> = 0xD800..=0xDBFF;
        const LOW: core::ops::RangeInclusive<u16> = 0xDC00..=0xDFFF;

        let at = self.pos;
        let first = self.hex4()?;
        if LOW.contains(&first) {
            return Err(self.expected(
                at,
                "a Unicode scalar value or a high surrogate; this `\\u` escape is an unpaired low \
                 surrogate, which denotes no character",
            ));
        }
        if !HIGH.contains(&first) {
            return char::from_u32(u32::from(first))
                .ok_or_else(|| self.expected(at, "a `\\u` escape naming a Unicode scalar value"));
        }
        // A high surrogate is only half a character; RFC 8259 §7 says the pair
        // must be complete, and an unpaired one is refused rather than replaced
        // with U+FFFD, because a replacement character is a silent corruption.
        if self.bytes.get(self.pos) != Some(&b'\\') || self.bytes.get(self.pos + 1) != Some(&b'u') {
            return Err(self.expected(
                self.pos,
                "`\\u` introducing the low surrogate that completes a surrogate pair",
            ));
        }
        self.pos += 2;
        let second_at = self.pos;
        let second = self.hex4()?;
        if !LOW.contains(&second) {
            return Err(self.expected(
                second_at,
                "a low surrogate (`\\uDC00` to `\\uDFFF`) completing a surrogate pair",
            ));
        }
        let combined =
            0x1_0000 + ((u32::from(first) - 0xD800) << 10) + (u32::from(second) - 0xDC00);
        char::from_u32(combined)
            .ok_or_else(|| self.expected(at, "a surrogate pair naming a Unicode scalar value"))
    }

    /// Exactly four hexadecimal digits, as one UTF-16 code unit.
    fn hex4(&mut self) -> Result<u16, GeoError> {
        let at = self.pos;
        let mut unit: u16 = 0;
        for _ in 0..4 {
            let Some(&byte) = self.bytes.get(self.pos) else {
                return Err(self.expected(at, "four hexadecimal digits after `\\u`"));
            };
            let Some(digit) = purrdf_hash::hex::nibble(byte) else {
                return Err(self.expected(at, "four hexadecimal digits after `\\u`"));
            };
            unit = (unit << 4) | u16::from(digit);
            self.pos += 1;
        }
        Ok(unit)
    }

    /// A number: validated against the RFC 8259 grammar, then kept as text.
    ///
    /// `-? ( 0 | [1-9][0-9]* ) ( . [0-9]+ )? ( [eE] [+-]? [0-9]+ )?`
    fn number(&mut self) -> Result<JsonValue, GeoError> {
        let start = self.pos;
        if self.bytes.get(self.pos) == Some(&b'-') {
            self.pos += 1;
        }
        match self.bytes.get(self.pos) {
            Some(b'0') => {
                self.pos += 1;
                if matches!(self.bytes.get(self.pos), Some(b'0'..=b'9')) {
                    return Err(self.expected(
                        self.pos,
                        "no further digit; JSON forbids a leading zero, so `01` is not a number",
                    ));
                }
            }
            Some(b'1'..=b'9') => self.digits(),
            _ => return Err(self.expected(self.pos, "a digit")),
        }
        if self.bytes.get(self.pos) == Some(&b'.') {
            self.pos += 1;
            if !matches!(self.bytes.get(self.pos), Some(b'0'..=b'9')) {
                return Err(self.expected(self.pos, "at least one digit after the decimal point"));
            }
            self.digits();
        }
        if matches!(self.bytes.get(self.pos), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.bytes.get(self.pos), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if !matches!(self.bytes.get(self.pos), Some(b'0'..=b'9')) {
                return Err(self.expected(self.pos, "at least one digit in the exponent"));
            }
            self.digits();
        }
        // The lexeme, verbatim: nothing here has decided what it denotes.
        Ok(JsonValue::Number(self.text[start..self.pos].to_owned()))
    }

    fn digits(&mut self) {
        while matches!(self.bytes.get(self.pos), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------

/// One step of the writer: a value still to write, an object member still to write
/// (its name, `:`, then its value), or a fixed piece of the text around a
/// container's members.
enum Job<'a> {
    Value(&'a JsonValue),
    Member(&'a str, &'a JsonValue),
    Text(&'static str),
}

/// Render a value as compact JSON — no insignificant whitespace — deterministically.
///
/// The output is a pure function of the value: members are written in the
/// vector's order (never a hash order), a [`JsonValue::Number`] is emitted as its
/// stored lexeme character for character, and a string escapes only what RFC 8259
/// requires — `"`, `\`, and the control characters below `U+0020`, using the
/// short escapes `\b \f \n \r \t` where they exist and lowercase `\u00xx`
/// otherwise. Two equal values therefore always produce equal bytes, which is
/// what makes this usable behind a byte-deterministic serializer.
///
/// A `Number` lexeme is trusted, not re-validated: one that came from [`parse`]
/// is grammatical by construction, and one a caller built is that caller's
/// responsibility.
///
/// Iterative: a container's members and the `,` and `]` or `}` around them go onto
/// a heap work list in reverse, so they pop in document order. Nesting costs heap
/// and never stack.
#[must_use]
pub fn write(value: &JsonValue) -> String {
    let mut out = String::new();
    let mut jobs: Vec<Job<'_>> = vec![Job::Value(value)];
    while let Some(job) = jobs.pop() {
        match job {
            Job::Text(text) => out.push_str(text),
            Job::Member(name, member) => {
                write_string(name, &mut out);
                out.push(':');
                jobs.push(Job::Value(member));
            }
            Job::Value(value) => match value {
                JsonValue::Null => out.push_str("null"),
                JsonValue::Bool(true) => out.push_str("true"),
                JsonValue::Bool(false) => out.push_str("false"),
                JsonValue::Number(lexeme) => out.push_str(lexeme),
                JsonValue::String(text) => write_string(text, &mut out),
                JsonValue::Array(items) => {
                    out.push('[');
                    jobs.push(Job::Text("]"));
                    for (index, item) in items.iter().enumerate().rev() {
                        jobs.push(Job::Value(item));
                        if index > 0 {
                            jobs.push(Job::Text(","));
                        }
                    }
                }
                JsonValue::Object(members) => {
                    out.push('{');
                    jobs.push(Job::Text("}"));
                    for (index, (name, member)) in members.iter().enumerate().rev() {
                        jobs.push(Job::Member(name, member));
                        if index > 0 {
                            jobs.push(Job::Text(","));
                        }
                    }
                }
            },
        }
    }
    out
}

/// A JSON string: the workspace's one JSON escape law,
/// [`purrdf_iri::json_escape`], in its [`JsonEscapes::ShortForms`] spelling
/// (`\b` and `\f` short; DEL and every non-ASCII scalar legal raw and written
/// as itself).
fn write_string(text: &str, out: &mut String) {
    push_string(out, text, JsonEscapes::ShortForms);
}

#[cfg(test)]
mod tests {
    use super::{JsonValue, Parser, arbitrary, parse, write};
    use crate::error::GeoError;
    use crate::geom::arbitrary::{self as deep, Lcg};

    fn number(lexeme: &str) -> JsonValue {
        JsonValue::Number(lexeme.to_owned())
    }

    fn refusal(text: &str) -> String {
        match parse(text) {
            Err(GeoError::Literal(message)) => message,
            Err(other) => panic!("expected a Literal refusal for {text:?}, got {other:?}"),
            Ok(value) => panic!("expected a refusal for {text:?}, parsed {value:?}"),
        }
    }

    // ---- the value kinds -------------------------------------------------

    #[test]
    fn each_json_value_kind_parses_to_its_own_variant() {
        assert_eq!(parse("null"), Ok(JsonValue::Null), "null");
        assert_eq!(parse("true"), Ok(JsonValue::Bool(true)), "true");
        assert_eq!(parse("false"), Ok(JsonValue::Bool(false)), "false");
        assert_eq!(parse("1"), Ok(number("1")), "a number");
        assert_eq!(
            parse("\"a\""),
            Ok(JsonValue::String("a".to_owned())),
            "a string"
        );
        assert_eq!(parse("[]"), Ok(JsonValue::Array(vec![])), "an empty array");
        assert_eq!(
            parse("{}"),
            Ok(JsonValue::Object(vec![])),
            "an empty object"
        );
    }

    /// RFC 8259 §2 whitespace is these four characters, anywhere between tokens,
    /// and it must not change the parsed value.
    #[test]
    fn whitespace_between_tokens_is_insignificant() {
        let spaced = " \t\r\n { \"a\" : [ 1 , 2 ] } \n";
        let tight = "{\"a\":[1,2]}";
        assert_eq!(
            parse(spaced),
            parse(tight),
            "whitespace must not change the value"
        );
    }

    #[test]
    fn an_empty_document_is_refused_but_a_bare_scalar_is_a_document() {
        assert!(
            refusal("").contains("byte 0"),
            "an empty document names offset 0"
        );
        assert!(
            refusal("   ").contains("a JSON value"),
            "whitespace alone is not a value"
        );
        // The neighbouring VALID case: RFC 8259 §2 allows any value at the top.
        assert_eq!(parse(" 7 "), Ok(number("7")), "a bare scalar is a document");
    }

    // ---- numbers keep their text -----------------------------------------

    /// The whole reason this module exists: a number survives parsing as the
    /// characters that were written, never as a float.
    #[test]
    fn a_number_is_kept_as_its_source_lexeme_verbatim() {
        for lexeme in [
            "0",
            "-0",
            "1",
            "-1",
            "1.5",
            "1.50",
            "15e-1",
            "1E+2",
            "1e2",
            "0.1",
            "-83.42391749999999999999999999999999999999",
            "123456789012345678901234567890",
            "1e308",
            "1e-308",
        ] {
            assert_eq!(
                parse(lexeme),
                Ok(number(lexeme)),
                "the lexeme {lexeme:?} must survive character for character"
            );
        }
    }

    /// Two spellings of the same value stay distinguishable as text; it is the
    /// consumer, not the reader, that decides they denote the same number.
    #[test]
    fn equal_values_with_different_spellings_are_different_lexemes() {
        assert_ne!(
            parse("1.5"),
            parse("1.50"),
            "the reader reports text, not value"
        );
        assert_ne!(parse("1.5"), parse("15e-1"), "likewise for an exponent");
    }

    #[test]
    fn every_malformed_number_is_refused_and_its_valid_neighbour_is_not() {
        for (bad, good) in [
            ("+1", "1"),
            ("01", "0"),
            ("-01", "-0"),
            (".5", "0.5"),
            ("1.", "1.0"),
            ("1.e3", "1.0e3"),
            ("1e", "1e1"),
            ("1e+", "1e+1"),
            ("1e-", "1e-1"),
            ("-", "-1"),
            ("NaN", "0"),
            ("Infinity", "0"),
            ("-Infinity", "-1"),
            ("0x10", "0"),
            ("1_000", "1000"),
        ] {
            assert!(
                parse(bad).is_err(),
                "{bad:?} is not an RFC 8259 number and must be refused"
            );
            assert_eq!(
                parse(good),
                Ok(number(good)),
                "the neighbouring valid form {good:?} must still parse"
            );
        }
    }

    #[test]
    fn a_refusal_names_the_byte_offset_and_what_was_expected() {
        let message = refusal("[1, 2, x]");
        assert!(message.contains("byte 7"), "names the offset: {message}");
        assert!(
            message.contains("a JSON value"),
            "names the expectation: {message}"
        );
        assert!(message.contains("`x`"), "names what was there: {message}");
    }

    // ---- strings ---------------------------------------------------------

    #[test]
    fn every_short_escape_is_resolved() {
        assert_eq!(
            parse(r#""\" \\ \/ \b \f \n \r \t""#),
            Ok(JsonValue::String("\" \\ / \u{8} \u{c} \n \r \t".to_owned())),
            "the eight short escapes"
        );
    }

    #[test]
    fn a_unicode_escape_and_a_surrogate_pair_are_resolved() {
        assert_eq!(
            parse("\"\\u0041\\u00E9\\u20AC\""),
            Ok(JsonValue::String("A\u{e9}\u{20ac}".to_owned())),
            "basic multilingual plane escapes, in upper-case hexadecimal"
        );
        assert_eq!(
            parse("\"\\uD834\\uDD1E\""),
            Ok(JsonValue::String("\u{1d11e}".to_owned())),
            "a surrogate pair denotes one astral character"
        );
        assert_eq!(
            parse("\"\\ud83d\\ude38\""),
            Ok(JsonValue::String("\u{1f638}".to_owned())),
            "lower-case hexadecimal digits are equally valid"
        );
        assert_eq!(
            parse("\"\\u0000\""),
            Ok(JsonValue::String("\u{0}".to_owned())),
            "an escaped NUL is a legal string character"
        );
    }

    #[test]
    fn an_unpaired_surrogate_is_refused_but_a_complete_pair_is_not() {
        assert!(
            refusal(r#""\uD834""#).contains("low surrogate"),
            "a lone high surrogate names what is missing"
        );
        assert!(
            refusal(r#""\uD834A""#).contains("low surrogate"),
            "a high surrogate followed by a non-surrogate is still unpaired"
        );
        assert!(
            refusal(r#""\uDD1E""#).contains("unpaired low surrogate"),
            "a lone low surrogate denotes no character"
        );
        // The neighbouring VALID case: the same escapes, correctly paired.
        assert_eq!(
            parse("\"\\uD834\\uDD1E\""),
            Ok(JsonValue::String("\u{1d11e}".to_owned())),
            "the complete pair must still parse"
        );
    }

    #[test]
    fn a_bad_escape_or_short_hex_run_is_refused_but_the_good_one_is_not() {
        assert!(parse(r#""\q""#).is_err(), "`\\q` is not an escape");
        assert!(parse(r#""\u12""#).is_err(), "two hex digits is not four");
        assert!(parse(r#""\u12g4""#).is_err(), "`g` is not hexadecimal");
        assert!(
            parse(r#""\""#).is_err(),
            "a trailing backslash is not an escape"
        );
        // The neighbouring VALID cases.
        assert_eq!(
            parse("\"\\u1234\""),
            Ok(JsonValue::String("\u{1234}".to_owned())),
            "four hexadecimal digits parse"
        );
        assert_eq!(
            parse(r#""\\""#),
            Ok(JsonValue::String("\\".to_owned())),
            "an escaped backslash parses"
        );
    }

    #[test]
    fn an_unescaped_control_character_is_refused_but_the_escaped_one_is_not() {
        assert!(
            refusal("\"a\nb\"").contains("escaped"),
            "a raw newline inside a string is refused"
        );
        assert!(parse("\"a\tb\"").is_err(), "a raw tab likewise");
        // The neighbouring VALID case: the same text, escaped.
        assert_eq!(
            parse(r#""a\nb""#),
            Ok(JsonValue::String("a\nb".to_owned())),
            "the escaped form must still parse"
        );
        // And U+007F is NOT a control character for RFC 8259 purposes.
        assert_eq!(
            parse("\"a\u{7f}b\""),
            Ok(JsonValue::String("a\u{7f}b".to_owned())),
            "DEL is legal unescaped"
        );
    }

    #[test]
    fn multi_byte_characters_survive_unescaped() {
        assert_eq!(
            parse("\"caf\u{e9} \u{1f638} \u{4e2d}\""),
            Ok(JsonValue::String("caf\u{e9} \u{1f638} \u{4e2d}".to_owned())),
            "runs are copied on char boundaries"
        );
    }

    #[test]
    fn an_unterminated_string_is_refused_but_a_closed_one_is_not() {
        assert!(
            refusal("\"abc").contains("closes a string"),
            "the missing quote is named"
        );
        assert_eq!(
            parse("\"abc\""),
            Ok(JsonValue::String("abc".to_owned())),
            "the closed neighbour parses"
        );
    }

    // ---- structure -------------------------------------------------------

    #[test]
    fn an_unterminated_array_or_object_is_refused_but_a_closed_one_is_not() {
        assert!(parse("[1, 2").is_err(), "an unterminated array");
        assert!(parse("{\"a\": 1").is_err(), "an unterminated object");
        assert!(parse("{\"a\"").is_err(), "an object cut off after a name");
        assert!(parse("{\"a\":").is_err(), "an object cut off after a colon");
        // The neighbouring VALID cases.
        assert!(parse("[1, 2]").is_ok(), "the closed array parses");
        assert!(parse("{\"a\": 1}").is_ok(), "the closed object parses");
    }

    #[test]
    fn a_trailing_comma_is_refused_but_the_comma_free_neighbour_is_not() {
        assert!(parse("[1,]").is_err(), "a trailing comma in an array");
        assert!(
            parse("{\"a\":1,}").is_err(),
            "a trailing comma in an object"
        );
        assert!(parse("[,]").is_err(), "a comma with nothing around it");
        // The neighbouring VALID cases: the same documents, one character shorter.
        assert_eq!(
            parse("[1]"),
            Ok(JsonValue::Array(vec![number("1")])),
            "the array without the comma parses"
        );
        assert!(parse("{\"a\":1}").is_ok(), "the object without it parses");
    }

    #[test]
    fn an_unquoted_member_name_or_missing_colon_is_refused() {
        // Bound rather than inlined: a brace-bearing literal inside `assert!`
        // reads as a format argument to clippy, and it is not one.
        let bare_name = concat!("{", "a:1}");
        let single_quoted = "{'a':1}";
        assert!(parse(bare_name).is_err(), "a bare member name");
        assert!(parse(single_quoted).is_err(), "a single-quoted member name");
        assert!(parse("{\"a\" 1}").is_err(), "a missing colon");
        assert!(
            parse("{\"a\":1}").is_ok(),
            "the well-formed neighbour parses"
        );
    }

    #[test]
    fn trailing_content_after_the_top_level_value_is_refused_but_clean_input_is_not() {
        let message = refusal("{\"a\":1} garbage");
        assert!(
            message.contains("the end of the text"),
            "the refusal says what was expected: {message}"
        );
        assert!(parse("1 2").is_err(), "two values is not one document");
        assert!(parse("[1][2]").is_err(), "two arrays likewise");
        // The neighbouring VALID case: trailing whitespace is not content.
        assert!(
            parse("{\"a\":1}   \n").is_ok(),
            "trailing whitespace is insignificant"
        );
    }

    // ---- duplicate member names ------------------------------------------

    /// RFC 8259 §4 permits a repeated member name and does not say which wins, so
    /// both are retained (a map would drop one silently), `get` documents the
    /// first-match rule, and `count` lets a consumer notice and refuse.
    #[test]
    fn a_duplicate_member_name_is_retained_and_get_returns_the_first() {
        let value = parse(r#"{"a":1,"b":2,"a":3}"#).expect("a well-formed object");
        assert_eq!(
            value,
            JsonValue::Object(vec![
                ("a".to_owned(), number("1")),
                ("b".to_owned(), number("2")),
                ("a".to_owned(), number("3")),
            ]),
            "both `a` members are retained, in document order"
        );
        assert_eq!(value.get("a"), Some(&number("1")), "get returns the first");
        assert_eq!(value.count("a"), 2, "count sees both");
        assert_eq!(value.count("b"), 1, "and one of a unique name");
        assert_eq!(value.count("z"), 0, "and none of an absent name");
        assert_eq!(value.get("z"), None, "an absent name has no value");
    }

    #[test]
    fn get_and_count_are_empty_on_a_non_object() {
        for value in [
            JsonValue::Null,
            JsonValue::Bool(true),
            number("1"),
            JsonValue::String("a".to_owned()),
            JsonValue::Array(vec![number("1")]),
        ] {
            assert_eq!(value.get("a"), None, "{value:?} has no members");
            assert_eq!(value.count("a"), 0, "{value:?} counts none");
        }
    }

    #[test]
    fn kind_name_names_each_variant() {
        assert_eq!(JsonValue::Null.kind_name(), "null");
        assert_eq!(JsonValue::Bool(false).kind_name(), "a boolean");
        assert_eq!(number("1").kind_name(), "a number");
        assert_eq!(JsonValue::String(String::new()).kind_name(), "a string");
        assert_eq!(JsonValue::Array(vec![]).kind_name(), "an array");
        assert_eq!(JsonValue::Object(vec![]).kind_name(), "an object");
    }

    // ---- the recursive references ----------------------------------------

    /// The recursive-descent reading of one value: the reference the work-list
    /// reader is compared with on shallow documents. It shares the scalar readers
    /// and spells the container diagnostics identically, so the only thing the
    /// comparison can turn on is the walk.
    fn reference_value(parser: &mut Parser<'_>) -> Result<JsonValue, GeoError> {
        let start = parser.pos;
        match parser.bytes.get(parser.pos) {
            Some(b'n') => parser.word("null", JsonValue::Null),
            Some(b't') => parser.word("true", JsonValue::Bool(true)),
            Some(b'f') => parser.word("false", JsonValue::Bool(false)),
            Some(b'"') => parser.string().map(JsonValue::String),
            Some(b'[') => reference_array(parser),
            Some(b'{') => reference_object(parser),
            Some(b'-' | b'0'..=b'9') => parser.number(),
            _ => Err(parser.expected(start, "a JSON value")),
        }
    }

    fn reference_array(parser: &mut Parser<'_>) -> Result<JsonValue, GeoError> {
        parser.pos += 1;
        let mut items: Vec<JsonValue> = Vec::new();
        parser.skip_whitespace();
        if parser.bytes.get(parser.pos) == Some(&b']') {
            parser.pos += 1;
            return Ok(JsonValue::Array(items));
        }
        loop {
            parser.skip_whitespace();
            items.push(reference_value(parser)?);
            parser.skip_whitespace();
            match parser.bytes.get(parser.pos) {
                Some(b',') => parser.pos += 1,
                Some(b']') => {
                    parser.pos += 1;
                    return Ok(JsonValue::Array(items));
                }
                _ => return Err(parser.expected(parser.pos, "`,` or `]` in an array")),
            }
        }
    }

    fn reference_object(parser: &mut Parser<'_>) -> Result<JsonValue, GeoError> {
        parser.pos += 1;
        let mut members: Vec<(String, JsonValue)> = Vec::new();
        parser.skip_whitespace();
        if parser.bytes.get(parser.pos) == Some(&b'}') {
            parser.pos += 1;
            return Ok(JsonValue::Object(members));
        }
        loop {
            parser.skip_whitespace();
            if parser.bytes.get(parser.pos) != Some(&b'"') {
                return Err(parser.expected(parser.pos, "a `\"`-quoted member name"));
            }
            let name = parser.string()?;
            parser.skip_whitespace();
            if parser.bytes.get(parser.pos) != Some(&b':') {
                return Err(parser.expected(parser.pos, "`:` after a member name"));
            }
            parser.pos += 1;
            parser.skip_whitespace();
            let value = reference_value(parser)?;
            members.push((name, value));
            parser.skip_whitespace();
            match parser.bytes.get(parser.pos) {
                Some(b',') => parser.pos += 1,
                Some(b'}') => {
                    parser.pos += 1;
                    return Ok(JsonValue::Object(members));
                }
                _ => return Err(parser.expected(parser.pos, "`,` or `}` in an object")),
            }
        }
    }

    fn reference_parse(text: &str) -> Result<JsonValue, GeoError> {
        let mut parser = Parser {
            text,
            bytes: text.as_bytes(),
            pos: 0,
        };
        parser.skip_whitespace();
        let value = reference_value(&mut parser)?;
        parser.skip_whitespace();
        if parser.pos == text.len() {
            Ok(value)
        } else {
            Err(parser.expected(parser.pos, "the end of the text after the top-level value"))
        }
    }

    /// The recursive writer: the reference the work-list writer is compared with.
    fn reference_write(value: &JsonValue, out: &mut String) {
        match value {
            JsonValue::Null => out.push_str("null"),
            JsonValue::Bool(true) => out.push_str("true"),
            JsonValue::Bool(false) => out.push_str("false"),
            JsonValue::Number(lexeme) => out.push_str(lexeme),
            JsonValue::String(text) => super::write_string(text, out),
            JsonValue::Array(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    reference_write(item, out);
                }
                out.push(']');
            }
            JsonValue::Object(members) => {
                out.push('{');
                for (index, (name, member)) in members.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    super::write_string(name, out);
                    out.push(':');
                    reference_write(member, out);
                }
                out.push('}');
            }
        }
    }

    /// Over generated documents — every kind, nested, whitespace between tokens —
    /// the work-list reader and the recursive reference produce the same value; and
    /// over the same documents with one structural fault, the same refusal at the
    /// same byte.
    #[test]
    fn the_reader_agrees_with_the_recursive_reference_on_generated_documents() {
        let mut rng = Lcg::new(0x5eed_2001);
        for round in 0..400 {
            let value = arbitrary::value(&mut rng, 4);
            let mut text = String::new();
            arbitrary::spaced_text(&mut rng, &value, &mut text);
            assert_eq!(
                parse(&text),
                reference_parse(&text),
                "round {round}: the two readers disagree on {text}"
            );
            assert_eq!(
                parse(&text),
                Ok(value),
                "round {round}: the spaced text reads back to its value: {text}"
            );
            for _ in 0..4 {
                let faulted = arbitrary::faulted(&mut rng, &text);
                assert_eq!(
                    parse(&faulted),
                    reference_parse(&faulted),
                    "round {round}: the two readers disagree on {faulted:?}"
                );
            }
        }
    }

    /// Over generated values the work-list writer and the recursive reference emit
    /// the same bytes, and the bytes read back to the value.
    #[test]
    fn the_writer_agrees_with_the_recursive_reference_on_generated_values() {
        let mut rng = Lcg::new(0x5eed_2002);
        for round in 0..400 {
            let value = arbitrary::value(&mut rng, 5);
            let mut expected = String::new();
            reference_write(&value, &mut expected);
            assert_eq!(write(&value), expected, "round {round}: {value:?}");
            assert_eq!(
                parse(&expected),
                Ok(value),
                "round {round}: the bytes read back to the value: {expected}"
            );
        }
    }

    // ---- depth -----------------------------------------------------------

    /// `[` `depth` times, then `]` `depth` times.
    fn nested(depth: usize) -> String {
        let mut text = String::with_capacity(depth * 2);
        for _ in 0..depth {
            text.push('[');
        }
        for _ in 0..depth {
            text.push(']');
        }
        text
    }

    /// How many containers deep a chain of first members goes.
    fn levels(value: &JsonValue) -> usize {
        let mut depth = 0;
        let mut node = value;
        loop {
            node = match node {
                JsonValue::Array(items) => {
                    depth += 1;
                    match items.first() {
                        Some(item) => item,
                        None => return depth,
                    }
                }
                JsonValue::Object(members) => {
                    depth += 1;
                    match members.first() {
                        Some((_, member)) => member,
                        None => return depth,
                    }
                }
                _ => return depth,
            };
        }
    }

    /// A hundred thousand nested arrays, and a hundred thousand nested objects,
    /// are read, written back byte for byte and dropped on a 128 KiB stack; a
    /// refusal at the bottom of such a document is the refusal the same text
    /// produces at the top, at its own byte.
    #[test]
    fn nesting_is_bounded_by_memory_alone() {
        let depth = deep::DEEP;
        assert_eq!(nested(1), "[]");
        assert_eq!(nested(2), "[[]]");
        assert_eq!(levels(&parse(&nested(2)).expect("two arrays")), 2);
        deep::on_small_stack(move || {
            let arrays = nested(depth);
            let value = parse(&arrays).expect("a hundred thousand nested arrays parse");
            assert_eq!(levels(&value), depth);
            assert_eq!(write(&value), arrays, "and write back byte for byte");
            drop(value);

            let mut objects = String::with_capacity(depth * 6 + 1);
            for _ in 0..depth {
                objects.push_str("{\"a\":");
            }
            objects.push('1');
            for _ in 0..depth {
                objects.push('}');
            }
            let value = parse(&objects).expect("a hundred thousand nested objects parse");
            assert_eq!(levels(&value), depth);
            assert_eq!(write(&value), objects, "and write back byte for byte");
            drop(value);

            let mut unfinished = String::with_capacity(depth + 1);
            for _ in 0..depth {
                unfinished.push('[');
            }
            assert_eq!(
                refusal(&unfinished),
                format!("JSON at byte {depth}: expected a JSON value, found the end of the text")
            );
            unfinished.push('x');
            assert_eq!(
                refusal(&unfinished),
                format!("JSON at byte {depth}: expected a JSON value, found `x`")
            );
        });
    }

    /// Width is not depth: two thousand sibling arrays nest two levels.
    #[test]
    fn a_wide_shallow_document_is_two_levels_deep() {
        let mut text = String::from("[");
        for index in 0..2000 {
            if index > 0 {
                text.push(',');
            }
            text.push_str("[1,2]");
        }
        text.push(']');
        let value = parse(&text).expect("2000 sibling arrays parse");
        assert_eq!(levels(&value), 2, "2000 sibling arrays are depth 2");
    }

    // ---- writing ---------------------------------------------------------

    #[test]
    fn writing_is_compact_and_byte_exact() {
        for golden in [
            "null",
            "true",
            "false",
            "0",
            "-1.5e-3",
            "\"\"",
            "[]",
            "{}",
            "[1,2,3]",
            "{\"a\":1,\"b\":[true,null]}",
            "[[[]]]",
            "{\"\":{}}",
        ] {
            let value = parse(golden).expect("the golden is well-formed JSON");
            assert_eq!(
                write(&value),
                golden,
                "compact JSON round-trips byte for byte"
            );
        }
    }

    #[test]
    fn writing_preserves_member_order_including_duplicates() {
        let value = parse("{ \"b\" : 1 , \"a\" : 2 , \"b\" : 3 }").expect("well-formed");
        assert_eq!(
            write(&value),
            "{\"b\":1,\"a\":2,\"b\":3}",
            "member order is the document's, never a sorted or hashed order"
        );
    }

    #[test]
    fn writing_escapes_only_what_rfc_8259_requires() {
        let value = JsonValue::String(
            "quote \" backslash \\ solidus / bs \u{8} ff \u{c} nl \n cr \r tab \t unit \u{1f} \
             caf\u{e9} \u{1f638}"
                .to_owned(),
        );
        assert_eq!(
            write(&value),
            "\"quote \\\" backslash \\\\ solidus / bs \\b ff \\f nl \\n cr \\r tab \\t unit \
             \\u001f caf\u{e9} \u{1f638}\"",
            "a solidus and every non-control scalar are written as themselves"
        );
    }

    #[test]
    fn writing_a_number_emits_its_lexeme_unchanged() {
        assert_eq!(
            write(&number("1.500")),
            "1.500",
            "the writer never renormalizes a number"
        );
        assert_eq!(
            write(&number("123456789012345678901234567890.5")),
            "123456789012345678901234567890.5",
            "including one no float could hold"
        );
    }

    #[test]
    fn writing_then_reading_returns_the_same_value() {
        for text in [
            r#"{"type":"Point","coordinates":[1,2]}"#,
            "\"a \\\"quoted\\\" \\\\ string \\u0001\"",
            r#"[null,true,false,0,-0,1e10,"",[],{}]"#,
            r#"{"a":{"b":{"c":[1,[2,[3]]]}}}"#,
            "\"\\uD834\\uDD1E caf\\u00e9\"",
        ] {
            let once = parse(text).expect("the fixture is well-formed");
            let twice = parse(&write(&once)).expect("the writer emits well-formed JSON");
            assert_eq!(
                once, twice,
                "write then parse is the identity on a parsed value: {text}"
            );
        }
    }
}

/// The frozen JSON string-body decoding vectors, replayed against the GeoJSON reader.
#[cfg(test)]
mod json_string_frozen_vectors {
    use purrdf_testkit::vectors::{VectorFile, answer_digest, decode_str, encode_str};

    const STRINGS: &str = include_str!("../../lex/tests/vectors/json_string_vectors.txt");
    const UNITS: &str = include_str!("../../lex/tests/vectors/json_unit_vectors.txt");

    /// `body` (the text between the quotes) decoded, or `None` when refused.
    fn decode(body: &str) -> Option<String> {
        let text = format!("\"{body}\"");
        let mut parser = super::Parser {
            text: &text,
            bytes: text.as_bytes(),
            pos: 0,
        };
        parser.string().ok().filter(|_| parser.pos == text.len())
    }

    fn answer(body: &str) -> String {
        decode(body).map_or_else(|| "-".to_owned(), |text| encode_str(&format!("={text}")))
    }

    /// Whether this copy is known to answer `body` differently from `expected`.
    fn skipped(body: &str, expected: &str) -> bool {
        let _ = (body, expected);
        false
    }

    #[test]
    fn string_bodies_replay_the_frozen_vectors() {
        let file = VectorFile::parse(STRINGS).expect("json_string_vectors.txt");
        let mut replayed = 0;
        for record in file.records() {
            let body = decode_str(record.fields[0]).expect("an encoded body");
            let expected = decode_str(record.fields[1]).expect("an encoded answer");
            if skipped(&body, &expected) {
                continue;
            }
            assert_eq!(answer(&body), record.fields[1], "{body:?}");
            replayed += 1;
        }
        assert!(replayed > 1000, "{replayed}");
    }

    #[test]
    fn every_unit_and_pair_replays_the_frozen_digests() {
        let file = VectorFile::parse(UNITS).expect("json_unit_vectors.txt");
        for record in file.records() {
            let upper = record.fields[1] == "upper";
            let unit = |u: u32| {
                if upper {
                    format!("\\u{u:04X}")
                } else {
                    format!("\\u{u:04x}")
                }
            };
            let first = u32::from_str_radix(record.fields[2], 16).expect("hex");
            let last = u32::from_str_radix(record.fields[3], 16).expect("hex");
            let answers = (first..=last).map(|value| {
                if record.fields[0] == "unit" {
                    answer(&unit(value))
                } else {
                    let offset = value - 0x1_0000;
                    answer(&format!(
                        "{}{}",
                        unit(0xD800 + (offset >> 10)),
                        unit(0xDC00 + (offset & 0x3FF))
                    ))
                }
            });
            assert_eq!(
                answer_digest(answers),
                record.fields[4],
                "{:?}",
                record.fields
            );
        }
    }
}
