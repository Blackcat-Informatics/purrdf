// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A bounded, non-recursive RFC 8259 JSON reader that keeps every number as
//! the text the document wrote.
//!
//! # What it is for
//!
//! Every PurRDF crate that reads a small JSON document it does not own the
//! shape of — a context, a manifest, a `sh:jsonSchema` body, a `cdt:Map`
//! literal — needs the same three things: RFC 8259 exactly, nothing decided
//! for it (a number stays its lexeme, a repeated member name stays repeated),
//! and a bound it can hand untrusted bytes to. This is that reader, in the
//! leaf every JSON-touching crate already reaches, beside the string-body
//! [`scanner`](crate::terminals::find_first_json_string_special) and
//! [escape law](crate::json_escape) it shares with the writers.
//!
//! # Numbers are lexemes
//!
//! [`Value::Number`] holds the source text verbatim, validated against the
//! §6 grammar and otherwise untouched. Deciding what it denotes — an
//! `xsd:integer`, an `xsd:decimal`, an `xsd:double`, an exact rational — is
//! the consumer's decision, and taking it here into an `f64` would make
//! `0.1` and `0.10000000000000000555` the same document.
//!
//! # Objects are ordered pairs, not a map
//!
//! §4 permits a repeated member name and says nothing about which occurrence
//! wins; a map representation silently drops one. [`Value::Object`] keeps
//! every member in document order, so nothing is lost, and a consumer for
//! which a repeat is an ambiguity can see it and refuse.
//!
//! # Bounded, and never recursive
//!
//! [`parse`] takes a [`Bounds`]: a depth the document may nest to and a size
//! it may have. The reader keeps its open arrays and objects on an explicit
//! heap stack and alternates between reading the next value and closing the
//! containers a finished member completes, so reading spends no stack frame
//! per level: a document that nests ten thousand deep is refused at the
//! bound with a [`JsonErrorKind::Depth`] at the byte where it crosses, and
//! costs nothing but the bytes before it. The bound is what lets the value
//! type keep the compiler's derived glue — see [`Value`].

use crate::json_escape::{JsonUnescapeErrorKind, decode_escape};
use crate::scan::{find_first_json_string_special, find_first_trivia};
use core::fmt;
use core::mem;

/// A parsed JSON value.
///
/// # Nesting and the derived walks
///
/// `Clone`, `PartialEq`, `Debug` and `Drop` are the compiler's derived
/// recursive glue, one stack frame per level of nesting. That is sound
/// because a value [`parse`] builds nests no deeper than the
/// [`Bounds::max_depth`] it was parsed under: at the default bound of
/// [`Bounds::DEFAULT_MAX_DEPTH`] all four walks over a value at exactly that
/// depth are exercised on a 128 KiB thread in this module's tests. A caller
/// who raises the bound raises the stack every one of those walks needs in
/// proportion, and a value built by hand rather than by [`parse`] is under
/// no bound at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// The literal `null`.
    Null,
    /// The literal `true` or `false`.
    Bool(bool),
    /// A number, as the exact characters the document wrote. Guaranteed by
    /// [`parse`] to match the RFC 8259 §6 grammar; nothing has interpreted
    /// it.
    Number(String),
    /// A string, with every escape resolved.
    String(String),
    /// An array, in document order.
    Array(Vec<Self>),
    /// An object, as name/value pairs in document order. Names may repeat:
    /// RFC 8259 §4 allows it, and dropping a repeat would be a silent loss.
    Object(Vec<(String, Self)>),
}

/// What a document may cost the reader.
///
/// Both bounds are inclusive: a document nesting exactly `max_depth` deep
/// or measuring exactly `max_bytes` bytes is admitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    /// The deepest an array or object may nest. The top-level container is
    /// depth 1, so `0` admits scalars only. Every `[` and `{` is checked,
    /// an empty container included: `[]` at depth `max_depth + 1` is refused.
    pub max_depth: usize,
    /// The longest text, in bytes, the reader will look at. A longer text is
    /// refused before any of it is read, as [`JsonErrorKind::Size`] at the
    /// bound.
    pub max_bytes: usize,
}

impl Bounds {
    /// The depth [`Bounds::default`] admits: 128, deep enough for any
    /// document a person or a serializer writes on purpose, and shallow
    /// enough that the derived walks over a [`Value`] at that depth fit in a
    /// small thread's stack (see [`Value`]).
    pub const DEFAULT_MAX_DEPTH: usize = 128;
}

impl Default for Bounds {
    /// [`DEFAULT_MAX_DEPTH`](Self::DEFAULT_MAX_DEPTH) levels, and no size
    /// bound (`usize::MAX`): the text is already in memory, so its size has
    /// already been paid, and only its nesting can cost more than its length.
    fn default() -> Self {
        Self {
            max_depth: Self::DEFAULT_MAX_DEPTH,
            max_bytes: usize::MAX,
        }
    }
}

/// Why a text was not read as a JSON document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsonErrorKind {
    /// An array or object would nest deeper than [`Bounds::max_depth`].
    Depth,
    /// The text is longer than [`Bounds::max_bytes`].
    Size,
    /// A departure from the RFC 8259 grammar outside a string body, naming
    /// what was expected at the offset.
    Syntax(&'static str),
    /// A string body RFC 8259 §7 does not admit: a bad escape or a raw
    /// control character. The [`JsonUnescapeErrorKind`] names the clause.
    Escape(JsonUnescapeErrorKind),
    /// Content other than whitespace after the top-level value.
    Trailing,
}

/// A refusal to read a text as JSON: where, and why.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsonError {
    /// The byte offset in the text the refusal is about: the byte that was
    /// not what the grammar expected, the `\` of a bad escape, the `[` or `{`
    /// that would cross the depth bound, or the size bound itself.
    pub at: usize,
    /// What was wrong there.
    pub kind: JsonErrorKind,
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON at byte {}: ", self.at)?;
        match self.kind {
            JsonErrorKind::Depth => f.write_str("an array or object nests deeper than the bound"),
            JsonErrorKind::Size => f.write_str("the text is longer than the bound"),
            JsonErrorKind::Syntax(expected) => write!(f, "expected {expected}"),
            JsonErrorKind::Escape(kind) => write!(f, "in a string, {kind}"),
            JsonErrorKind::Trailing => {
                f.write_str("expected the end of the text after the top-level value")
            }
        }
    }
}

impl std::error::Error for JsonError {}

/// A refusal at `at` naming what the grammar expected there.
const fn syntax(at: usize, expected: &'static str) -> JsonError {
    JsonError {
        at,
        kind: JsonErrorKind::Syntax(expected),
    }
}

/// Parse `text` as a single RFC 8259 JSON document within `bounds`.
///
/// Any value may be the document (§2), surrounded by the four `ws` bytes and
/// nothing else. Refused, as a [`JsonError`] naming the byte and the reason:
/// a text longer than [`Bounds::max_bytes`]; a container nesting deeper than
/// [`Bounds::max_depth`]; a malformed number (a leading `+`, a leading zero,
/// a bare `.`, `NaN`, `Infinity`); an unterminated string, array or object;
/// a trailing comma; an unescaped control character or a bad escape in a
/// string; and content after the top-level value.
///
/// ```
/// use purrdf_iri::json::{Bounds, JsonErrorKind, Value, parse};
///
/// let value = parse(r#"{"a": [1, -0.5e3, "caf\u00e9"], "a": null}"#, &Bounds::default())?;
/// assert_eq!(
///     value,
///     Value::Object(vec![
///         (
///             "a".to_owned(),
///             Value::Array(vec![
///                 Value::Number("1".to_owned()),
///                 Value::Number("-0.5e3".to_owned()),
///                 Value::String("caf\u{e9}".to_owned()),
///             ])
///         ),
///         ("a".to_owned(), Value::Null),
///     ])
/// );
/// // The bound is the reader's, not the stack's.
/// let deep = "[".repeat(10_000);
/// assert_eq!(
///     parse(&deep, &Bounds::default()).map_err(|e| e.kind),
///     Err(JsonErrorKind::Depth)
/// );
/// # Ok::<(), purrdf_iri::json::JsonError>(())
/// ```
pub fn parse(text: &str, bounds: &Bounds) -> Result<Value, JsonError> {
    if text.len() > bounds.max_bytes {
        return Err(JsonError {
            at: bounds.max_bytes,
            kind: JsonErrorKind::Size,
        });
    }
    let mut parser = Parser {
        text,
        bytes: text.as_bytes(),
        pos: 0,
        max_depth: bounds.max_depth,
    };
    parser.skip_whitespace();
    let value = parser.document()?;
    parser.skip_whitespace();
    if parser.pos == text.len() {
        Ok(value)
    } else {
        Err(JsonError {
            at: parser.pos,
            kind: JsonErrorKind::Trailing,
        })
    }
}

/// The reader. Positions are byte offsets into `text`; every structural
/// character JSON has is ASCII, so a byte offset the reader stops at is
/// always a `char` boundary.
struct Parser<'a> {
    text: &'a str,
    bytes: &'a [u8],
    pos: usize,
    max_depth: usize,
}

/// An array or object whose `[` or `{` has been read and whose members are
/// still being read.
enum Open {
    /// The items read so far.
    Array(Vec<Value>),
    /// The members read so far, and the name of the member whose value is
    /// being read.
    Object {
        members: Vec<(String, Value)>,
        name: String,
    },
}

impl Open {
    /// The value the closed container denotes.
    fn close(self) -> Value {
        match self {
            Self::Array(items) => Value::Array(items),
            Self::Object { members, .. } => Value::Object(members),
        }
    }
}

/// What reading at the cursor produced: a whole value, or a container opened
/// onto the stack whose first member is next.
enum Read {
    Value(Value),
    Opened,
}

impl Parser<'_> {
    /// The byte at the cursor, or `None` at the end of the text.
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    /// Skip RFC 8259 §2 `ws`, which is `WS ::= #x20 | #x9 | #xD | #xA` and
    /// nothing else: the chunked [`find_first_trivia`] over the shared `WS`
    /// table, so FORM FEED and NO-BREAK SPACE end the run as the grammar says.
    fn skip_whitespace(&mut self) {
        self.pos = find_first_trivia(&self.bytes[self.pos..])
            .map_or(self.bytes.len(), |offset| self.pos + offset);
    }

    /// Read the value at the cursor, with every array and object it nests.
    ///
    /// Iterative: the open containers live in an explicit heap stack, and the
    /// loop alternates between reading the next value and closing the
    /// containers a finished member completes. Nesting therefore costs heap
    /// and never stack, and a diagnostic anywhere inside the document is the
    /// same diagnostic at the same byte as it would be at the top level.
    fn document(&mut self) -> Result<Value, JsonError> {
        let mut open: Vec<Open> = Vec::new();
        loop {
            let mut finished = match self.value(&mut open)? {
                Read::Value(value) => value,
                Read::Opened => continue,
            };
            // A finished value is the document, or a member of the innermost
            // open container: it is appended, and either the container
            // continues with the next member or it closes and is itself the
            // finished value one level up.
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

    /// The depth check at every `[` and `{`: the container about to open
    /// would nest one deeper than the containers already open.
    fn enter(&self, open: &[Open]) -> Result<(), JsonError> {
        if open.len() < self.max_depth {
            Ok(())
        } else {
            Err(JsonError {
                at: self.pos,
                kind: JsonErrorKind::Depth,
            })
        }
    }

    /// Read one value at the cursor. A scalar is complete; an empty array or
    /// object is complete; any other array or object is opened onto `open`
    /// with the cursor on its first value.
    fn value(&mut self, open: &mut Vec<Open>) -> Result<Read, JsonError> {
        let start = self.pos;
        let value = match self.peek() {
            Some(b'n') => self.word("null", Value::Null)?,
            Some(b't') => self.word("true", Value::Bool(true))?,
            Some(b'f') => self.word("false", Value::Bool(false))?,
            Some(b'"') => Value::String(self.string()?),
            Some(b'[') => {
                self.enter(open)?;
                self.pos += 1;
                self.skip_whitespace();
                if self.peek() == Some(b']') {
                    self.pos += 1;
                    Value::Array(Vec::new())
                } else {
                    open.push(Open::Array(Vec::new()));
                    return Ok(Read::Opened);
                }
            }
            Some(b'{') => {
                self.enter(open)?;
                self.pos += 1;
                self.skip_whitespace();
                if self.peek() == Some(b'}') {
                    self.pos += 1;
                    Value::Object(Vec::new())
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
            // `+1`, `.5`, `NaN` and `Infinity` all land here: none of them is
            // a JSON value, and naming the position is more useful than
            // naming the spelling the author probably meant.
            _ => return Err(syntax(start, "a JSON value")),
        };
        Ok(Read::Value(value))
    }

    /// Append a finished `value` to the innermost open container and read
    /// what follows it: `Ok(true)` when another member does, the cursor on it
    /// (and, for an object, its name already read); `Ok(false)` when the
    /// container closed.
    fn append(&mut self, top: &mut Open, value: Value) -> Result<bool, JsonError> {
        match top {
            Open::Array(items) => {
                items.push(value);
                self.skip_whitespace();
                match self.peek() {
                    Some(b',') => {
                        self.pos += 1;
                        self.skip_whitespace();
                        Ok(true)
                    }
                    Some(b']') => {
                        self.pos += 1;
                        Ok(false)
                    }
                    // Includes the unterminated case (`None`) and the
                    // trailing comma, which reappears on the next turn as
                    // "expected a JSON value".
                    _ => Err(syntax(self.pos, "`,` or `]` in an array")),
                }
            }
            Open::Object { members, name } => {
                members.push((mem::take(name), value));
                self.skip_whitespace();
                match self.peek() {
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
                    _ => Err(syntax(self.pos, "`,` or `}` in an object")),
                }
            }
        }
    }

    /// A member's quoted name and the `:` after it, the cursor left on its
    /// value.
    fn member_name(&mut self) -> Result<String, JsonError> {
        if self.peek() != Some(b'"') {
            return Err(syntax(self.pos, "a `\"`-quoted member name"));
        }
        let name = self.string()?;
        self.skip_whitespace();
        if self.peek() != Some(b':') {
            return Err(syntax(self.pos, "`:` after a member name"));
        }
        self.pos += 1;
        self.skip_whitespace();
        Ok(name)
    }

    /// One of the three literal names, whole.
    fn word(&mut self, word: &'static str, value: Value) -> Result<Value, JsonError> {
        let start = self.pos;
        if self.bytes[start..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Ok(value)
        } else {
            Err(syntax(start, word))
        }
    }

    /// A string, with escapes resolved. The caller has already established
    /// that the byte at the cursor is `"`.
    ///
    /// Each clean run is found by the chunked
    /// [`find_first_json_string_special`] scan and copied whole; only a `"`,
    /// a `\` or a C0 control stops it, and the escape decoder is the one
    /// [`json_escape::unescape`](crate::json_escape::unescape) uses, so the
    /// reader refuses exactly the bodies it refuses.
    fn string(&mut self) -> Result<String, JsonError> {
        self.pos += 1;
        let mut out = String::new();
        let mut run_start = self.pos;
        loop {
            let Some(offset) = find_first_json_string_special(&self.bytes[self.pos..]) else {
                return Err(syntax(self.bytes.len(), "the `\"` that closes a string"));
            };
            let hit = self.pos + offset;
            match self.bytes[hit] {
                b'"' => {
                    out.push_str(&self.text[run_start..hit]);
                    self.pos = hit + 1;
                    return Ok(out);
                }
                b'\\' => {
                    out.push_str(&self.text[run_start..hit]);
                    let (ch, next) = decode_escape(self.bytes, hit).map_err(|error| JsonError {
                        at: error.at,
                        kind: JsonErrorKind::Escape(error.kind),
                    })?;
                    out.push(ch);
                    self.pos = next;
                    run_start = next;
                }
                _ => {
                    return Err(JsonError {
                        at: hit,
                        kind: JsonErrorKind::Escape(JsonUnescapeErrorKind::RawControl),
                    });
                }
            }
        }
    }

    /// A number: validated against the RFC 8259 §6 grammar, then kept as text.
    ///
    /// `-? ( 0 | [1-9][0-9]* ) ( . [0-9]+ )? ( [eE] [+-]? [0-9]+ )?`
    fn number(&mut self) -> Result<Value, JsonError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        match self.peek() {
            Some(b'0') => {
                self.pos += 1;
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    return Err(syntax(
                        self.pos,
                        "no further digit; JSON forbids a leading zero, so `01` is not a number",
                    ));
                }
            }
            Some(b'1'..=b'9') => self.digits(),
            _ => return Err(syntax(self.pos, "a digit")),
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(syntax(
                    self.pos,
                    "at least one digit after the decimal point",
                ));
            }
            self.digits();
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(syntax(self.pos, "at least one digit in the exponent"));
            }
            self.digits();
        }
        // The lexeme, verbatim: nothing here has decided what it denotes.
        Ok(Value::Number(self.text[start..self.pos].to_owned()))
    }

    fn digits(&mut self) {
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Bounds, JsonError, JsonErrorKind, Value, parse};
    use crate::json_escape::JsonUnescapeErrorKind;
    use std::fmt::Write as _;

    fn number(lexeme: &str) -> Value {
        Value::Number(lexeme.to_owned())
    }

    fn string(text: &str) -> Value {
        Value::String(text.to_owned())
    }

    fn read(text: &str) -> Result<Value, JsonError> {
        parse(text, &Bounds::default())
    }

    fn refusal(text: &str) -> JsonError {
        match read(text) {
            Err(error) => error,
            Ok(value) => panic!("expected a refusal for {text:?}, parsed {value:?}"),
        }
    }

    /// Run `f` on a thread with a 128 KiB stack, so a recursive walk that
    /// spends a frame per level shows up as a crash rather than passing on
    /// the test harness's generous main stack.
    fn on_small_stack(f: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(f)
            .expect("spawn a small-stack thread")
            .join()
            .expect("the small-stack thread completed");
    }

    // ---- the value kinds -------------------------------------------------

    #[test]
    fn each_json_value_kind_parses_to_its_own_variant() {
        assert_eq!(read("null"), Ok(Value::Null));
        assert_eq!(read("true"), Ok(Value::Bool(true)));
        assert_eq!(read("false"), Ok(Value::Bool(false)));
        assert_eq!(read("1"), Ok(number("1")));
        assert_eq!(read("\"a\""), Ok(string("a")));
        assert_eq!(read("[]"), Ok(Value::Array(vec![])));
        assert_eq!(read("{}"), Ok(Value::Object(vec![])));
        // A misspelled literal name is refused at its start.
        assert_eq!(
            refusal("nul"),
            JsonError {
                at: 0,
                kind: JsonErrorKind::Syntax("null")
            }
        );
        assert_eq!(refusal("[tru]").at, 1);
    }

    /// RFC 8259 §2 whitespace is the four `WS` bytes anywhere between tokens,
    /// and it must not change the parsed value; a fifth byte that a Unicode
    /// or ASCII whitespace property would admit is not whitespace here.
    #[test]
    fn whitespace_is_the_four_ws_bytes_and_nothing_else() {
        let spaced = " \t\r\n { \"a\" : [ 1 , 2 ] } \n";
        let tight = "{\"a\":[1,2]}";
        assert_eq!(read(spaced), read(tight));
        assert!(read(tight).is_ok());
        // FORM FEED is `is_ascii_whitespace`, NO-BREAK SPACE `is_whitespace`;
        // neither is `ws`.
        assert_eq!(refusal("[1,\u{c}2]").at, 3);
        assert_eq!(refusal("[1,\u{a0}2]").at, 3);
        assert_eq!(refusal("1\u{c}").kind, JsonErrorKind::Trailing);
        // The neighbours: the same documents with a `ws` byte instead.
        assert!(read("[1,\t2]").is_ok());
        assert!(read("[1,\r2]").is_ok());
        assert!(read("1\n").is_ok());
    }

    #[test]
    fn an_empty_document_is_refused_but_a_bare_scalar_is_a_document() {
        assert_eq!(refusal("").at, 0);
        assert_eq!(
            refusal("   "),
            JsonError {
                at: 3,
                kind: JsonErrorKind::Syntax("a JSON value")
            }
        );
        assert_eq!(read(" 7 "), Ok(number("7")));
    }

    // ---- numbers keep their text -----------------------------------------

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
            "1e5",
            "0.1",
            "-83.42391749999999999999999999999999999999",
            "123456789012345678901234567890",
            "1e308",
            "1e-308",
        ] {
            assert_eq!(read(lexeme), Ok(number(lexeme)), "{lexeme:?}");
        }
        // Two spellings of one value stay distinguishable as text.
        assert_ne!(read("1.5"), read("1.50"));
        assert_ne!(read("1.5"), read("15e-1"));
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
            assert!(read(bad).is_err(), "{bad:?} must be refused");
            assert_eq!(read(good), Ok(number(good)), "{good:?} must parse");
        }
        assert_eq!(
            refusal("01"),
            JsonError {
                at: 1,
                kind: JsonErrorKind::Syntax(
                    "no further digit; JSON forbids a leading zero, so `01` is not a number"
                )
            }
        );
    }

    // ---- strings ---------------------------------------------------------

    #[test]
    fn every_escape_is_resolved() {
        assert_eq!(
            read(r#""\" \\ \/ \b \f \n \r \t""#),
            Ok(string("\" \\ / \u{8} \u{c} \n \r \t"))
        );
        assert_eq!(
            read("\"\\u0041\\u00E9\\u20AC\""),
            Ok(string("A\u{e9}\u{20ac}"))
        );
        assert_eq!(read("\"\\uD834\\uDD1E\""), Ok(string("\u{1d11e}")));
        assert_eq!(read("\"\\ud83d\\ude00\""), Ok(string("\u{1f600}")));
        assert_eq!(
            read("\"\\u0000\""),
            Ok(string("\u{0}")),
            "an escaped NUL is a legal character"
        );
        assert_eq!(read("\"a\\u0000b\""), Ok(string("a\u{0}b")));
    }

    #[test]
    fn a_bad_string_is_refused_at_its_byte_and_the_good_neighbour_is_not() {
        use JsonUnescapeErrorKind as K;
        for (bad, at, kind, good) in [
            (r#"["\uD834"]"#, 2, K::LoneSurrogate, r#"["\uD834\uDD1E"]"#),
            (
                r#"["\uD834A"]"#,
                2,
                K::LoneSurrogate,
                r#"["\uD834\uDD1EA"]"#,
            ),
            (r#"["\uDD1E"]"#, 2, K::LoneSurrogate, r#"["\u00E9"]"#),
            (r#"["\q"]"#, 2, K::UnknownEscape, r#"["\n"]"#),
            (r#"["\u12"]"#, 2, K::MalformedUnicodeEscape, r#"["\u1234"]"#),
            (
                r#"["\u12g4"]"#,
                2,
                K::MalformedUnicodeEscape,
                r#"["\u1234"]"#,
            ),
            (
                r#"["\u+041"]"#,
                2,
                K::MalformedUnicodeEscape,
                r#"["\u0041"]"#,
            ),
            ("[\"a\nb\"]", 3, K::RawControl, r#"["a\nb"]"#),
            ("[\"a\tb\"]", 3, K::RawControl, r#"["a\tb"]"#),
        ] {
            assert_eq!(
                refusal(bad),
                JsonError {
                    at,
                    kind: JsonErrorKind::Escape(kind)
                },
                "{bad:?}"
            );
            assert!(read(good).is_ok(), "{good:?}");
        }
        // A trailing backslash is a truncated escape inside an unterminated
        // string: the escape is what the reader reaches first.
        assert_eq!(
            refusal("\"\\").kind,
            JsonErrorKind::Escape(K::TruncatedEscape)
        );
        // DEL and every non-ASCII scalar are legal raw.
        assert_eq!(read("\"a\u{7f}b\""), Ok(string("a\u{7f}b")));
        assert_eq!(
            read("\"caf\u{e9} \u{1f638} \u{4e2d}\u{85}\""),
            Ok(string("caf\u{e9} \u{1f638} \u{4e2d}\u{85}"))
        );
    }

    #[test]
    fn an_unterminated_string_is_refused_but_a_closed_one_is_not() {
        assert_eq!(
            refusal("\"abc"),
            JsonError {
                at: 4,
                kind: JsonErrorKind::Syntax("the `\"` that closes a string")
            }
        );
        assert_eq!(refusal("[\"abc\\\"").at, 7);
        assert_eq!(read("\"abc\""), Ok(string("abc")));
    }

    // ---- structure -------------------------------------------------------

    #[test]
    fn an_unterminated_array_or_object_is_refused_but_a_closed_one_is_not() {
        assert_eq!(refusal("[1, 2").at, 5);
        assert_eq!(refusal("{\"a\": 1").at, 7);
        assert_eq!(refusal("{\"a\"").at, 4);
        assert_eq!(refusal("{\"a\":").at, 5);
        assert!(read("[1, 2]").is_ok());
        assert!(read("{\"a\": 1}").is_ok());
    }

    #[test]
    fn a_trailing_comma_is_refused_but_the_comma_free_neighbour_is_not() {
        assert_eq!(
            refusal("[1,]"),
            JsonError {
                at: 3,
                kind: JsonErrorKind::Syntax("a JSON value")
            }
        );
        assert_eq!(refusal("{\"a\":1,}").at, 7);
        assert_eq!(refusal("[,]").at, 1);
        assert_eq!(read("[1]"), Ok(Value::Array(vec![number("1")])));
        assert!(read("{\"a\":1}").is_ok());
    }

    #[test]
    fn an_unquoted_member_name_or_missing_colon_is_refused() {
        let bare_name = concat!("{", "a:1}");
        let single_quoted = "{'a':1}";
        assert_eq!(
            refusal(bare_name),
            JsonError {
                at: 1,
                kind: JsonErrorKind::Syntax("a `\"`-quoted member name")
            }
        );
        assert_eq!(refusal(single_quoted).at, 1);
        assert_eq!(
            refusal("{\"a\" 1}"),
            JsonError {
                at: 5,
                kind: JsonErrorKind::Syntax("`:` after a member name")
            }
        );
        assert!(read("{\"a\":1}").is_ok());
    }

    #[test]
    fn trailing_content_after_the_top_level_value_is_refused_but_clean_input_is_not() {
        assert_eq!(
            refusal("{\"a\":1} garbage"),
            JsonError {
                at: 8,
                kind: JsonErrorKind::Trailing
            }
        );
        assert_eq!(refusal("1 2").kind, JsonErrorKind::Trailing);
        assert_eq!(refusal("[1][2]").kind, JsonErrorKind::Trailing);
        assert!(read("{\"a\":1}   \n").is_ok());
    }

    // ---- duplicate member names ------------------------------------------

    #[test]
    fn a_duplicate_member_name_is_kept_in_document_order() {
        assert_eq!(
            read(r#"{"a":1,"b":2,"a":3}"#),
            Ok(Value::Object(vec![
                ("a".to_owned(), number("1")),
                ("b".to_owned(), number("2")),
                ("a".to_owned(), number("3")),
            ]))
        );
    }

    // ---- bounds ----------------------------------------------------------

    /// `open` `depth` times, then `close` `depth` times, around `core`.
    fn nested(open: &str, core: &str, close: &str, depth: usize) -> String {
        let mut text = String::with_capacity(depth * (open.len() + close.len()) + core.len());
        for _ in 0..depth {
            text.push_str(open);
        }
        text.push_str(core);
        for _ in 0..depth {
            text.push_str(close);
        }
        text
    }

    /// How many containers deep a chain of first members goes.
    fn levels(value: &Value) -> usize {
        let mut depth = 0;
        let mut node = value;
        loop {
            node = match node {
                Value::Array(items) => {
                    depth += 1;
                    match items.first() {
                        Some(item) => item,
                        None => return depth,
                    }
                }
                Value::Object(members) => {
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

    /// Ten thousand nested arrays are refused at the default bound, on a
    /// 128 KiB stack, at the byte of the `[` that crosses it; a hundred deep
    /// parses. The same for objects.
    #[test]
    fn nesting_past_the_bound_is_refused_without_touching_the_stack() {
        on_small_stack(|| {
            let bound = Bounds::DEFAULT_MAX_DEPTH;
            for (open, core, close) in [("[", "", "]"), ("{\"a\":", "1", "}")] {
                let deep = nested(open, core, close, 10_000);
                assert_eq!(
                    refusal(&deep),
                    JsonError {
                        at: bound * open.len(),
                        kind: JsonErrorKind::Depth
                    },
                    "{open}"
                );
                // An unterminated deep document is refused for its depth
                // before its end is missed.
                assert_eq!(refusal(&open.repeat(10_000)).kind, JsonErrorKind::Depth);

                let hundred = nested(open, core, close, 100);
                assert_eq!(levels(&read(&hundred).expect("a hundred deep parses")), 100);
                // The exact edge: the bound is admitted, one past it refused.
                let at_bound = nested(open, core, close, bound);
                assert_eq!(
                    levels(&read(&at_bound).expect("the bound is admitted")),
                    bound
                );
                let past = nested(open, core, close, bound + 1);
                assert_eq!(refusal(&past).kind, JsonErrorKind::Depth, "{open}");
            }
        });
    }

    /// The derived `Clone`, `PartialEq`, `Debug` and `Drop` walks over a value
    /// at the default bound fit in a 128 KiB stack, which is what lets the
    /// value type keep them.
    #[test]
    fn the_derived_walks_fit_a_small_stack_at_the_default_bound() {
        on_small_stack(|| {
            let bound = Bounds::DEFAULT_MAX_DEPTH;
            let text = nested("[", "", "]", bound);
            let value = read(&text).expect("the bound is admitted");
            let copy = value.clone();
            assert_eq!(copy, value);
            let mut shown = String::new();
            write!(shown, "{value:?}").expect("a String sink never fails");
            assert!(shown.starts_with("Array([Array(["));
            drop(copy);
            drop(value);
            let objects = read(&nested("{\"a\":", "1", "}", bound)).expect("the bound is admitted");
            assert_eq!(objects.clone(), objects);
            drop(objects);
        });
    }

    #[test]
    fn an_empty_container_counts_its_own_depth_and_a_scalar_at_the_bound_is_admitted() {
        let bounds = Bounds {
            max_depth: 2,
            max_bytes: usize::MAX,
        };
        assert!(parse("[[]]", &bounds).is_ok());
        assert!(parse("[[1]]", &bounds).is_ok());
        assert_eq!(
            parse("[[[]]]", &bounds),
            Err(JsonError {
                at: 2,
                kind: JsonErrorKind::Depth
            })
        );
        assert_eq!(
            parse("[[{}]]", &bounds).map_err(|e| e.kind),
            Err(JsonErrorKind::Depth)
        );
        // Depth zero admits scalars only.
        let scalars_only = Bounds {
            max_depth: 0,
            max_bytes: usize::MAX,
        };
        assert_eq!(parse("1", &scalars_only), Ok(number("1")));
        assert_eq!(parse("\"a\"", &scalars_only), Ok(string("a")));
        assert_eq!(
            parse("[]", &scalars_only),
            Err(JsonError {
                at: 0,
                kind: JsonErrorKind::Depth
            })
        );
        // Width is not depth.
        let mut wide = String::from("[");
        for index in 0..2000 {
            if index > 0 {
                wide.push(',');
            }
            wide.push_str("[1,2]");
        }
        wide.push(']');
        assert_eq!(levels(&parse(&wide, &bounds).expect("two levels")), 2);
    }

    #[test]
    fn a_text_past_the_size_bound_is_refused_before_it_is_read_and_one_at_it_is_not() {
        let bounds = Bounds {
            max_depth: Bounds::DEFAULT_MAX_DEPTH,
            max_bytes: 5,
        };
        assert_eq!(
            parse("[1,2]", &bounds),
            Ok(Value::Array(vec![number("1"), number("2")]))
        );
        assert_eq!(
            parse("[1,23]", &bounds),
            Err(JsonError {
                at: 5,
                kind: JsonErrorKind::Size
            })
        );
        // Refused for size even when it would also be refused for syntax, so
        // the bound is proven to come first.
        assert_eq!(
            parse("[1,2,,", &bounds).map_err(|e| e.kind),
            Err(JsonErrorKind::Size)
        );
        assert_eq!(Bounds::default().max_bytes, usize::MAX);
        assert_eq!(Bounds::default().max_depth, 128);
    }

    #[test]
    fn a_refusal_displays_its_byte_and_its_reason() {
        assert_eq!(
            refusal("[1, 2, x]").to_string(),
            "JSON at byte 7: expected a JSON value"
        );
        assert_eq!(
            refusal("[\"\\q\"]").to_string(),
            "JSON at byte 2: in a string, an escape other than `\\\"`, `\\\\`, `\\/`, `\\b`, \
             `\\f`, `\\n`, `\\r`, `\\t` or `\\u`"
        );
        assert_eq!(
            refusal(&"[".repeat(200)).to_string(),
            "JSON at byte 128: an array or object nests deeper than the bound"
        );
        assert_eq!(
            refusal("1 2").to_string(),
            "JSON at byte 2: expected the end of the text after the top-level value"
        );
        let bounds = Bounds {
            max_depth: 1,
            max_bytes: 1,
        };
        assert_eq!(
            parse("12", &bounds).unwrap_err().to_string(),
            "JSON at byte 1: the text is longer than the bound"
        );
    }

    // ---- the recursive reference -----------------------------------------

    /// A fixed-seed generator (SplitMix64), so every run draws the same inputs.
    struct SplitMix(u64);

    impl SplitMix {
        const fn next(&mut self) -> u64 {
            purrdf_testkit::rng::splitmix64_next(&mut self.0)
        }

        fn below(&mut self, n: usize) -> usize {
            usize::try_from(self.next() % n as u64).expect("below n")
        }
    }

    /// A random value nesting at most `depth` containers.
    fn arbitrary(rng: &mut SplitMix, depth: usize) -> Value {
        const STRINGS: [&str; 6] = [
            "",
            "a",
            "caf\u{e9}",
            "q\"b\\s\n\t",
            "\u{1}\u{7f}\u{85}",
            "\u{1f600}",
        ];
        const NUMBERS: [&str; 6] = ["0", "-0", "1", "1.5", "-2e10", "3.25E-2"];
        let kind = rng.below(if depth == 0 { 4 } else { 6 });
        match kind {
            0 => Value::Null,
            1 => Value::Bool(rng.below(2) == 1),
            2 => number(NUMBERS[rng.below(NUMBERS.len())]),
            3 => string(STRINGS[rng.below(STRINGS.len())]),
            4 => Value::Array(
                (0..rng.below(4))
                    .map(|_| arbitrary(rng, depth - 1))
                    .collect(),
            ),
            _ => Value::Object(
                (0..rng.below(4))
                    .map(|_| (STRINGS[rng.below(3)].to_owned(), arbitrary(rng, depth - 1)))
                    .collect(),
            ),
        }
    }

    fn maybe_ws(rng: &mut SplitMix, out: &mut String) {
        match rng.below(4) {
            0 => out.push(' '),
            1 => out.push_str("\n\t"),
            _ => {}
        }
    }

    /// A random spacing of `value`, strings in the `ShortForms` spelling.
    fn spaced_text(rng: &mut SplitMix, value: &Value, out: &mut String) {
        maybe_ws(rng, out);
        match value {
            Value::Null => out.push_str("null"),
            Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Value::Number(lexeme) => out.push_str(lexeme),
            Value::String(text) => {
                crate::json_escape::push_string(
                    out,
                    text,
                    crate::json_escape::JsonEscapes::ShortForms,
                );
            }
            Value::Array(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        maybe_ws(rng, out);
                        out.push(',');
                    }
                    spaced_text(rng, item, out);
                }
                maybe_ws(rng, out);
                out.push(']');
            }
            Value::Object(members) => {
                out.push('{');
                for (index, (name, member)) in members.iter().enumerate() {
                    if index > 0 {
                        maybe_ws(rng, out);
                        out.push(',');
                    }
                    maybe_ws(rng, out);
                    crate::json_escape::push_string(
                        out,
                        name,
                        crate::json_escape::JsonEscapes::Ascii,
                    );
                    maybe_ws(rng, out);
                    out.push(':');
                    spaced_text(rng, member, out);
                }
                maybe_ws(rng, out);
                out.push('}');
            }
        }
        maybe_ws(rng, out);
    }

    /// The recursive-descent reading of one value: the reference the
    /// work-list reader is compared with on shallow documents. It shares the
    /// scalar readers and spells the container diagnostics identically, so
    /// the only thing the comparison can turn on is the walk.
    fn reference_value(parser: &mut super::Parser<'_>) -> Result<Value, JsonError> {
        let start = parser.pos;
        match parser.peek() {
            Some(b'n') => parser.word("null", Value::Null),
            Some(b't') => parser.word("true", Value::Bool(true)),
            Some(b'f') => parser.word("false", Value::Bool(false)),
            Some(b'"') => parser.string().map(Value::String),
            Some(b'[') => {
                parser.pos += 1;
                let mut items = Vec::new();
                parser.skip_whitespace();
                if parser.peek() == Some(b']') {
                    parser.pos += 1;
                    return Ok(Value::Array(items));
                }
                loop {
                    parser.skip_whitespace();
                    items.push(reference_value(parser)?);
                    parser.skip_whitespace();
                    match parser.peek() {
                        Some(b',') => parser.pos += 1,
                        Some(b']') => {
                            parser.pos += 1;
                            return Ok(Value::Array(items));
                        }
                        _ => return Err(super::syntax(parser.pos, "`,` or `]` in an array")),
                    }
                }
            }
            Some(b'{') => {
                parser.pos += 1;
                let mut members = Vec::new();
                parser.skip_whitespace();
                if parser.peek() == Some(b'}') {
                    parser.pos += 1;
                    return Ok(Value::Object(members));
                }
                loop {
                    parser.skip_whitespace();
                    let name = parser.member_name()?;
                    let value = reference_value(parser)?;
                    members.push((name, value));
                    parser.skip_whitespace();
                    match parser.peek() {
                        Some(b',') => parser.pos += 1,
                        Some(b'}') => {
                            parser.pos += 1;
                            return Ok(Value::Object(members));
                        }
                        _ => return Err(super::syntax(parser.pos, "`,` or `}` in an object")),
                    }
                }
            }
            Some(b'-' | b'0'..=b'9') => parser.number(),
            _ => Err(super::syntax(start, "a JSON value")),
        }
    }

    fn reference_parse(text: &str) -> Result<Value, JsonError> {
        let mut parser = super::Parser {
            text,
            bytes: text.as_bytes(),
            pos: 0,
            max_depth: usize::MAX,
        };
        parser.skip_whitespace();
        let value = reference_value(&mut parser)?;
        parser.skip_whitespace();
        if parser.pos == text.len() {
            Ok(value)
        } else {
            Err(JsonError {
                at: parser.pos,
                kind: JsonErrorKind::Trailing,
            })
        }
    }

    /// One byte of `text` replaced, deleted or duplicated: a structural fault
    /// somewhere in a document, at which both readers must agree.
    fn faulted(rng: &mut SplitMix, text: &str) -> String {
        const FAULTS: [&str; 8] = [",", "]", "}", "\"", "\\", ":", "x", " "];
        let boundaries: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
        if boundaries.is_empty() {
            return FAULTS[rng.below(FAULTS.len())].to_owned();
        }
        let at = boundaries[rng.below(boundaries.len())];
        let width = text[at..].chars().next().map_or(0, char::len_utf8);
        match rng.below(3) {
            0 => format!(
                "{}{}{}",
                &text[..at],
                FAULTS[rng.below(FAULTS.len())],
                &text[at + width..]
            ),
            1 => format!("{}{}", &text[..at], &text[at + width..]),
            _ => format!(
                "{}{}{}",
                &text[..at + width],
                &text[at..at + width],
                &text[at + width..]
            ),
        }
    }

    /// Over generated documents — every kind, nested, whitespace between
    /// tokens — the work-list reader and the recursive reference produce the
    /// same value, which is the value the text was written from; and over
    /// the same documents with one fault, the same refusal at the same byte.
    #[test]
    fn the_reader_agrees_with_the_recursive_reference_on_generated_documents() {
        let mut rng = SplitMix(0x005E_ED20_0100_0001);
        let mut refused = 0_usize;
        for round in 0..400 {
            let value = arbitrary(&mut rng, 4);
            let mut text = String::new();
            spaced_text(&mut rng, &value, &mut text);
            assert_eq!(read(&text), reference_parse(&text), "round {round}: {text}");
            assert_eq!(read(&text), Ok(value), "round {round}: {text}");
            for _ in 0..4 {
                let broken = faulted(&mut rng, &text);
                let got = read(&broken);
                assert_eq!(got, reference_parse(&broken), "round {round}: {broken:?}");
                refused += usize::from(got.is_err());
            }
        }
        assert!(refused > 0, "non-vacuity: some fault was a refusal");
    }
}
