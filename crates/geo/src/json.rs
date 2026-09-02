// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
//! # Bounded work
//!
//! Nesting is capped at [`MAX_DEPTH`] containers. The reader is recursive
//! descent, so an unbounded document would be a stack overflow — an abort, not an
//! error a host can catch — and a literal arrives from the dataset, which is
//! untrusted input. The cap is deliberately far above anything GeoJSON needs (a
//! `MultiPolygon` reaches five), and the tests prove that a document one level
//! *below* the cap still parses, because an over-refusal here would reject
//! conforming data just as surely as a missing check would accept malformed data.

use crate::error::GeoError;

/// The greatest number of nested arrays and objects [`parse`] will accept.
///
/// A document nested more deeply than this is refused rather than recursed into.
/// GeoSPARQL needs five levels at the very most (a `MultiPolygon`'s
/// `coordinates`, its polygons, their rings, their positions, their ordinates),
/// so the cap is a stack guard against hostile input, never a limit real data
/// meets.
pub const MAX_DEPTH: usize = 128;

/// A parsed JSON value.
///
/// [`Self::Number`] keeps the source lexeme verbatim, so a consumer can decide
/// the value exactly rather than inheriting a float's rounding; see the module
/// documentation for why that is the whole reason this reader exists.
#[derive(Clone, Debug, PartialEq, Eq)]
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
/// unpaired UTF-16 surrogate, content after the top-level value, or nesting
/// deeper than [`MAX_DEPTH`].
pub fn parse(text: &str) -> Result<JsonValue, GeoError> {
    let mut parser = Parser {
        text,
        bytes: text.as_bytes(),
        pos: 0,
        depth: 0,
    };
    parser.skip_whitespace();
    let value = parser.value()?;
    parser.skip_whitespace();
    if parser.pos == text.len() {
        Ok(value)
    } else {
        Err(parser.expected(parser.pos, "the end of the text after the top-level value"))
    }
}

/// The recursive-descent reader. Positions are byte offsets into `text`; every
/// structural character JSON has is ASCII, so a byte offset the reader stops at
/// is always a `char` boundary.
struct Parser<'a> {
    text: &'a str,
    bytes: &'a [u8],
    pos: usize,
    depth: usize,
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

    /// Count one more open container, refusing past [`MAX_DEPTH`].
    fn enter(&mut self) -> Result<(), GeoError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(GeoError::literal(format!(
                "JSON at byte {}: nesting deeper than {MAX_DEPTH} arrays and objects; the reader \
                 caps depth because it is recursive and a literal is untrusted input",
                self.pos
            )));
        }
        Ok(())
    }

    fn value(&mut self) -> Result<JsonValue, GeoError> {
        let start = self.pos;
        match self.bytes.get(self.pos) {
            Some(b'n') => self.word("null", JsonValue::Null),
            Some(b't') => self.word("true", JsonValue::Bool(true)),
            Some(b'f') => self.word("false", JsonValue::Bool(false)),
            Some(b'"') => self.string().map(JsonValue::String),
            Some(b'[') => self.array(),
            Some(b'{') => self.object(),
            Some(b'-' | b'0'..=b'9') => self.number(),
            // `+1`, `.5`, `NaN` and `Infinity` all land here: none of them is a
            // JSON value, and naming the position is more useful than naming the
            // spelling the author probably meant.
            _ => Err(self.expected(start, "a JSON value")),
        }
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

    fn array(&mut self) -> Result<JsonValue, GeoError> {
        self.enter()?;
        self.pos += 1;
        let mut items: Vec<JsonValue> = Vec::new();
        self.skip_whitespace();
        if self.bytes.get(self.pos) == Some(&b']') {
            self.pos += 1;
            self.depth -= 1;
            return Ok(JsonValue::Array(items));
        }
        loop {
            self.skip_whitespace();
            items.push(self.value()?);
            self.skip_whitespace();
            match self.bytes.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    self.depth -= 1;
                    return Ok(JsonValue::Array(items));
                }
                // Includes the unterminated case (`None`) and the trailing comma,
                // which reappears on the next turn as "expected a JSON value".
                _ => return Err(self.expected(self.pos, "`,` or `]` in an array")),
            }
        }
    }

    fn object(&mut self) -> Result<JsonValue, GeoError> {
        self.enter()?;
        self.pos += 1;
        let mut members: Vec<(String, JsonValue)> = Vec::new();
        self.skip_whitespace();
        if self.bytes.get(self.pos) == Some(&b'}') {
            self.pos += 1;
            self.depth -= 1;
            return Ok(JsonValue::Object(members));
        }
        loop {
            self.skip_whitespace();
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
            let value = self.value()?;
            members.push((name, value));
            self.skip_whitespace();
            match self.bytes.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    self.depth -= 1;
                    return Ok(JsonValue::Object(members));
                }
                _ => return Err(self.expected(self.pos, "`,` or `}` in an object")),
            }
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
            let digit = match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => return Err(self.expected(at, "four hexadecimal digits after `\\u`")),
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
#[must_use]
pub fn write(value: &JsonValue) -> String {
    let mut out = String::new();
    write_into(value, &mut out);
    out
}

fn write_into(value: &JsonValue, out: &mut String) {
    match value {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(true) => out.push_str("true"),
        JsonValue::Bool(false) => out.push_str("false"),
        JsonValue::Number(lexeme) => out.push_str(lexeme),
        JsonValue::String(text) => write_string(text, out),
        JsonValue::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_into(item, out);
            }
            out.push(']');
        }
        JsonValue::Object(members) => {
            out.push('{');
            for (index, (name, member)) in members.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_string(name, out);
                out.push(':');
                write_into(member, out);
            }
            out.push('}');
        }
    }
}

fn write_string(text: &str, out: &mut String) {
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if u32::from(control) < 0x20 => {
                out.push_str("\\u");
                let code = u32::from(control);
                for shift in [12_u32, 8, 4, 0] {
                    let nibble = (code >> shift) & 0xF;
                    // Every nibble is < 16, so `from_digit` cannot fail; the
                    // fallback keeps the function total without an unwrap.
                    out.push(char::from_digit(nibble, 16).unwrap_or('0'));
                }
            }
            // Everything else, including U+007F and every non-ASCII scalar, is
            // legal unescaped in RFC 8259 and is written as itself.
            other => out.push(other),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::{JsonValue, MAX_DEPTH, parse, write};
    use crate::error::GeoError;

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

    // ---- depth -----------------------------------------------------------

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

    /// The over-refusal control for the depth cap: one level below the cap, and
    /// the cap itself, must still parse. A guard that rejected ordinary data
    /// would be as much a bug as no guard at all.
    #[test]
    fn nesting_up_to_the_cap_parses_and_beyond_it_is_refused() {
        assert!(parse(&nested(1)).is_ok(), "one array parses");
        assert!(parse(&nested(127)).is_ok(), "depth 127 parses");
        assert!(
            parse(&nested(MAX_DEPTH)).is_ok(),
            "depth {MAX_DEPTH} is the cap and is inclusive"
        );
        let message = refusal(&nested(MAX_DEPTH + 1));
        assert!(
            message.contains("nesting deeper"),
            "the refusal names the cap: {message}"
        );
        assert!(
            parse(&nested(1000)).is_err(),
            "a deeply hostile document is refused rather than recursed into"
        );
    }

    #[test]
    fn the_depth_cap_counts_objects_too() {
        let mut text = String::new();
        for _ in 0..=MAX_DEPTH {
            text.push_str("{\"a\":");
        }
        text.push('1');
        for _ in 0..=MAX_DEPTH {
            text.push('}');
        }
        assert!(parse(&text).is_err(), "objects count against the same cap");
    }

    /// Depth is per open container, not cumulative: a wide document is not a deep
    /// one, and refusing it would be an over-refusal.
    #[test]
    fn a_wide_shallow_document_is_not_deep() {
        let mut text = String::from("[");
        for index in 0..2000 {
            if index > 0 {
                text.push(',');
            }
            text.push_str("[1,2]");
        }
        text.push(']');
        assert!(parse(&text).is_ok(), "2000 sibling arrays are depth 2");
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
