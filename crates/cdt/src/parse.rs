// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The SEP-0009 lexical scanner.
//!
//! # The grammar, as implemented
//!
//! ```text
//! [1]    List                 ::= '[' (NonEmptyListContent)? ']'
//! [2]    NonEmptyListContent  ::= ListElement (',' ListElement)*
//! [3]    ListElement          ::= IRIREF | BLANK_NODE_LABEL | RDFLiteral | NumericLiteral
//!                                | BooleanLiteral | NULL | List | Map
//!                                | TripleTerm                       (PurRDF superset)
//! [4]    Map                  ::= '{' (NonEmptyMapContent)? '}'
//! [5]    NonEmptyMapContent   ::= MapEntry (',' MapEntry)*
//! [6]    MapEntry             ::= MapKey ':' MapValue
//! [7]    MapKey               ::= IRIREF | RDFLiteral | NumericLiteral | BooleanLiteral
//! [8]    MapValue             ::= IRIREF | BLANK_NODE_LABEL | RDFLiteral | NumericLiteral
//!                                | BooleanLiteral | NULL | List | Map
//!                                | TripleTerm                       (PurRDF superset)
//! [9]    NULL                 ::= 'null'
//! [128s] RDFLiteral           ::= String (LANGTAG | '^^' IRIREF)?
//! [P1]   TripleTerm           ::= '<<(' Element Element Element ')>>'   (PurRDF superset)
//! ```
//!
//! `IRIREF`, `BLANK_NODE_LABEL`, `String`, `LANGTAG`, `NumericLiteral` and
//! `BooleanLiteral` are the SPARQL terminals, with RDF 1.2's `LANG_DIR` extension to
//! `LANGTAG` (`@lang--ltr` / `@lang--rtl`). Whitespace (space, tab, CR, LF) may
//! appear between any two terminals and nowhere inside one; comments are not part of
//! this lexical space and a `#` outside a string is an error.
//!
//! Beyond the productions, two constraints:
//!
//! * every `IRIREF`, after escape processing, must be an **absolute** IRI — a CDT
//!   lexical form carries no base, so a relative reference could never be resolved;
//! * a map's keys must be pairwise distinct (see [`parse_map`] for the exact rule).
//!
//! # The two PurRDF supersets
//!
//! Both are documented in the crate docs. Neither mints an IRI: the datatype stays
//! `cdt:List` / `cdt:Map`, and each form is only ever *emitted* for a term SEP-0009
//! cannot express at all, so a value that SEP-0009 can express is written in
//! SEP-0009's own lexical space and conformance is preserved.
//!
//! # Iterative by construction
//!
//! The scanner keeps its open composites in an explicit heap `Vec` of frames and
//! runs a two-state machine over them. There is no recursive descent anywhere, so
//! nesting depth costs heap, not stack, and a hostile `[[[[…` yields
//! [`CdtError::DepthExceeded`] rather than the uncatchable `abort` a stack overflow
//! would be.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::datatype::{
    CdtDatatype, RDF_DIR_LANG_STRING, RDF_LANG_STRING, XSD_BOOLEAN, XSD_DECIMAL, XSD_DOUBLE,
    XSD_INTEGER,
};
use crate::error::CdtError;
use crate::limits::{MAX_ELEMENTS, MAX_LEXICAL_BYTES, MAX_NESTING_DEPTH};
use crate::render::canonical_key_lexical;
use crate::term::{CdtEntry, CdtKey, CdtLiteral, CdtTerm, CdtTripleTerm, TextDirection};
use crate::value::CdtValue;

/// Parse a `cdt:List` lexical form.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtValue, parse_list};
///
/// let value = parse_list("[1, \"a\", null]")?;
/// assert_eq!(value.len(), 3);
/// assert!(matches!(value, CdtValue::List(_)));
///
/// // An unterminated list is a typed error carrying a byte offset.
/// assert!(parse_list("[1, 2").is_err());
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
pub fn parse_list(lexical: &str) -> Result<CdtValue, CdtError> {
    parse_cdt(lexical, CdtDatatype::List)
}

/// Parse a `cdt:Map` lexical form.
///
/// # Key distinctness
///
/// SEP-0009 requires a map's keys to be pairwise distinct, and distinguishes them by
/// **lexical form rather than by value** — deliberately, so `"1"^^xsd:integer` and
/// `"01"^^xsd:integer` are two different keys. This scanner enforces distinctness on
/// the key **term** (lexical form, datatype, language tag and base direction
/// together), which is strictly stronger than distinctness of the key *substrings*:
/// two equal substrings necessarily denote the same term, so everything the spec
/// rejects is rejected here too. The extra case it also rejects is the shorthand
/// collision — `{1: "a", "1"^^xsd:integer: "b"}` writes one and the same RDF term
/// twice, in two spellings. Admitting it would make the canonical form
/// non-injective (both entries render identically) and the map's own value
/// ill-defined, so it is a [`CdtError::DuplicateMapKey`].
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtValue, parse_map};
///
/// let value = parse_map("{\"a\": 1, \"b\": 2}")?;
/// assert_eq!(value.len(), 2);
///
/// // The same key twice — in either spelling — is refused.
/// assert!(parse_map("{\"a\": 1, \"a\": 2}").is_err());
/// assert!(parse_map("{1: \"a\", \"1\"^^<http://www.w3.org/2001/XMLSchema#integer>: \"b\"}").is_err());
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
pub fn parse_map(lexical: &str) -> Result<CdtValue, CdtError> {
    parse_cdt(lexical, CdtDatatype::Map)
}

/// Parse a lexical form as a known [`CdtDatatype`].
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtDatatype, parse_cdt};
///
/// assert_eq!(parse_cdt("[]", CdtDatatype::List)?.len(), 0);
/// assert_eq!(parse_cdt("{}", CdtDatatype::Map)?.len(), 0);
/// // A list lexical form is not a map lexical form.
/// assert!(parse_cdt("[]", CdtDatatype::Map).is_err());
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
pub fn parse_cdt(lexical: &str, datatype: CdtDatatype) -> Result<CdtValue, CdtError> {
    if lexical.len() > MAX_LEXICAL_BYTES {
        return Err(CdtError::InputTooLarge {
            offset: MAX_LEXICAL_BYTES,
            length: lexical.len(),
        });
    }
    let mut scanner = Scanner::new(lexical);
    let value = scanner.parse_root(datatype)?;
    scanner.skip_whitespace();
    if scanner.position < scanner.bytes.len() {
        return Err(CdtError::TrailingText {
            offset: scanner.position,
        });
    }
    Ok(value)
}

/// Parse a lexical form by datatype IRI, preserving the same tri-state
/// `purrdf_xsd::parse_by_iri` does.
///
/// * `Ok(Some(value))` — the IRI is a composite datatype and the lexical form is
///   well-formed.
/// * `Ok(None)` — the IRI is **not** a composite datatype. The literal belongs to
///   some other value space (or none); this is not a failure.
/// * `Err(_)` — the IRI *is* a composite datatype but the lexical form is malformed.
///
/// Collapsing the second and third cases would tell a caller that an ill-typed
/// `cdt:List` literal is an ordinary opaque term, which is exactly the confusion the
/// tri-state exists to prevent.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::parse_cdt_by_iri;
///
/// let list = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List";
/// assert!(parse_cdt_by_iri("[1]", list)?.is_some());
/// // Not a composite datatype at all.
/// assert!(parse_cdt_by_iri("anything", "http://example.org/custom")?.is_none());
/// // A composite datatype with a malformed lexical IS an error.
/// assert!(parse_cdt_by_iri("[1", list).is_err());
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
pub fn parse_cdt_by_iri(lexical: &str, datatype_iri: &str) -> Result<Option<CdtValue>, CdtError> {
    match CdtDatatype::from_iri(datatype_iri) {
        Some(datatype) => parse_cdt(lexical, datatype).map(Some),
        None => Ok(None),
    }
}

// ── The frame machine ───────────────────────────────────────────────────────────

/// An open production the scanner is inside.
enum Frame {
    /// An open `[ … ]`.
    List(Vec<CdtTerm>),
    /// An open `{ … }`. `entries` carries each key's byte offset so a duplicate can
    /// be reported at the position of its *second* occurrence.
    Map {
        entries: Vec<(usize, CdtEntry)>,
        pending: Option<(usize, CdtKey)>,
    },
    /// An open `<<( … )>>`.
    Triple(Vec<CdtTerm>),
}

impl Frame {
    fn new(datatype: CdtDatatype) -> Self {
        match datatype {
            CdtDatatype::List => Self::List(Vec::new()),
            CdtDatatype::Map => Self::Map {
                entries: Vec::new(),
                pending: None,
            },
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Self::List(items) | Self::Triple(items) => items.is_empty(),
            Self::Map { entries, .. } => entries.is_empty(),
        }
    }

    const fn close(&self) -> u8 {
        match self {
            Self::List(_) => b']',
            Self::Map { .. } => b'}',
            Self::Triple(_) => b')',
        }
    }
}

/// Which of the two scanner states the machine is in.
enum Step {
    /// Read the next element (or close an empty composite).
    Item,
    /// Read the separator or the closing delimiter after an element.
    Delim,
}

struct Scanner<'a> {
    input: &'a str,
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Scanner<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            bytes: input.as_bytes(),
            position: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn bump(&mut self) {
        self.position += 1;
    }

    fn starts_with(&self, text: &str) -> bool {
        self.input[self.position..].starts_with(text)
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.bump();
        }
    }

    /// Consume one byte, or report what the grammar admitted here.
    fn expect(&mut self, byte: u8, expected: &'static str) -> Result<(), CdtError> {
        match self.peek() {
            Some(found) if found == byte => {
                self.bump();
                Ok(())
            }
            Some(_) => Err(CdtError::Unexpected {
                offset: self.position,
                expected,
            }),
            None => Err(CdtError::UnexpectedEnd {
                offset: self.position,
                expected,
            }),
        }
    }

    fn expect_str(&mut self, text: &'static str, expected: &'static str) -> Result<(), CdtError> {
        if self.starts_with(text) {
            self.position += text.len();
            Ok(())
        } else if self.position < self.bytes.len() {
            Err(CdtError::Unexpected {
                offset: self.position,
                expected,
            })
        } else {
            Err(CdtError::UnexpectedEnd {
                offset: self.position,
                expected,
            })
        }
    }

    /// Consume and return the next Unicode scalar value.
    fn next_char(&mut self) -> Option<char> {
        let ch = self.input[self.position..].chars().next()?;
        self.position += ch.len_utf8();
        Some(ch)
    }

    /// The whole two-state machine. Returns the root composite.
    fn parse_root(&mut self, datatype: CdtDatatype) -> Result<CdtValue, CdtError> {
        self.skip_whitespace();
        self.expect(
            datatype.open(),
            match datatype {
                CdtDatatype::List => "`[` opening a cdt:List",
                CdtDatatype::Map => "`{` opening a cdt:Map",
            },
        )?;

        let mut stack: Vec<Frame> = Vec::new();
        stack.push(Frame::new(datatype));
        let mut elements: usize = 0;
        let mut step = Step::Item;

        loop {
            match step {
                Step::Item => {
                    self.skip_whitespace();
                    let top = stack.last().expect("the frame stack is never empty here");
                    // An empty composite closes immediately. A triple term has a
                    // fixed arity of three, so it is never closed while empty and the
                    // shortcut does not apply to it.
                    if !matches!(top, Frame::Triple(_))
                        && top.is_empty()
                        && self.peek() == Some(top.close())
                    {
                        self.bump();
                        if let Some(value) = self.close_frame(&mut stack)? {
                            return Ok(value);
                        }
                        step = Step::Delim;
                        continue;
                    }
                    if matches!(top, Frame::Map { .. }) {
                        let key_offset = self.position;
                        let key = self.parse_key()?;
                        self.skip_whitespace();
                        self.expect(b':', "`:` separating a map key from its value")?;
                        match stack
                            .last_mut()
                            .expect("the frame stack is never empty here")
                        {
                            Frame::Map { pending, .. } => *pending = Some((key_offset, key)),
                            Frame::List(_) | Frame::Triple(_) => {
                                unreachable!("the frame was just matched as a map")
                            }
                        }
                    }
                    self.skip_whitespace();
                    elements += 1;
                    if elements > MAX_ELEMENTS {
                        return Err(CdtError::TooManyElements {
                            offset: self.position,
                            limit: MAX_ELEMENTS,
                        });
                    }
                    let opening = match self.peek() {
                        Some(b'[') => Some(Frame::List(Vec::new())),
                        Some(b'{') => Some(Frame::Map {
                            entries: Vec::new(),
                            pending: None,
                        }),
                        Some(b'<') if self.starts_with("<<(") => Some(Frame::Triple(Vec::new())),
                        _ => None,
                    };
                    if let Some(frame) = opening {
                        if stack.len() >= MAX_NESTING_DEPTH {
                            return Err(CdtError::DepthExceeded {
                                offset: self.position,
                                limit: MAX_NESTING_DEPTH,
                            });
                        }
                        self.position += if matches!(frame, Frame::Triple(_)) {
                            3
                        } else {
                            1
                        };
                        stack.push(frame);
                        continue;
                    }
                    let term = self.parse_element()?;
                    push_item(&mut stack, term);
                    step = Step::Delim;
                }
                Step::Delim => {
                    self.skip_whitespace();
                    let top = stack.last().expect("the frame stack is never empty here");
                    if let Frame::Triple(parts) = top {
                        if parts.len() < 3 {
                            step = Step::Item;
                            continue;
                        }
                        self.expect_str(")>>", "`)>>` closing a triple term")?;
                        if let Some(value) = self.close_frame(&mut stack)? {
                            return Ok(value);
                        }
                        continue;
                    }
                    let close = top.close();
                    match self.peek() {
                        Some(b',') => {
                            self.bump();
                            step = Step::Item;
                        }
                        Some(found) if found == close => {
                            self.bump();
                            if let Some(value) = self.close_frame(&mut stack)? {
                                return Ok(value);
                            }
                        }
                        Some(_) => {
                            return Err(CdtError::Unexpected {
                                offset: self.position,
                                expected: "`,` or the closing delimiter",
                            });
                        }
                        None => {
                            return Err(CdtError::UnexpectedEnd {
                                offset: self.position,
                                expected: "`,` or the closing delimiter",
                            });
                        }
                    }
                }
            }
        }
    }

    /// Pop the top frame. Returns `Some` when the root composite just closed, and
    /// otherwise appends the finished element to its parent.
    fn close_frame(&self, stack: &mut Vec<Frame>) -> Result<Option<CdtValue>, CdtError> {
        let frame = stack.pop().expect("the frame stack is never empty here");
        let term = match frame {
            Frame::List(items) => CdtTerm::composite(CdtValue::List(items)),
            Frame::Map { entries, .. } => CdtTerm::composite(finish_map(entries)?),
            Frame::Triple(mut parts) => {
                let object = parts.pop().expect("a triple frame closes with three parts");
                let predicate = parts.pop().expect("a triple frame closes with three parts");
                let subject = parts.pop().expect("a triple frame closes with three parts");
                CdtTerm::TripleTerm(alloc::boxed::Box::new(CdtTripleTerm {
                    subject,
                    predicate,
                    object,
                }))
            }
        };
        if stack.is_empty() {
            return match term {
                CdtTerm::Composite(value) => Ok(Some(*value)),
                _ => unreachable!("the root frame is always a list or a map"),
            };
        }
        push_item(stack, term);
        Ok(None)
    }

    // ── Terminals ───────────────────────────────────────────────────────────────

    /// `[3]` / `[8]`, minus the composite and triple-term alternatives (which the
    /// frame machine opens itself).
    fn parse_element(&mut self) -> Result<CdtTerm, CdtError> {
        match self.peek() {
            Some(b'<') => Ok(CdtTerm::Iri(self.parse_iriref()?)),
            Some(b'_') => Ok(CdtTerm::Blank(self.parse_blank_node_label()?)),
            Some(b'"' | b'\'') => Ok(CdtTerm::Literal(self.parse_rdf_literal()?)),
            Some(b'0'..=b'9' | b'+' | b'-' | b'.') => Ok(CdtTerm::Literal(self.parse_numeric()?)),
            Some(b't' | b'f') => Ok(CdtTerm::Literal(self.parse_boolean()?)),
            Some(b'n') => {
                self.expect_str("null", "the `null` element")?;
                Ok(CdtTerm::Null)
            }
            Some(_) => Err(CdtError::Unexpected {
                offset: self.position,
                expected: "an element: an IRI, a blank node, a literal, `null`, a list, a map or a triple term",
            }),
            None => Err(CdtError::UnexpectedEnd {
                offset: self.position,
                expected: "an element",
            }),
        }
    }

    /// `[7] MapKey ::= IRIREF | RDFLiteral | NumericLiteral | BooleanLiteral`.
    ///
    /// Narrower than [`Self::parse_element`] by construction: a blank node, `null`,
    /// a nested composite and a triple term are all refused here, so the restriction
    /// is enforced at the one place the grammar states it.
    fn parse_key(&mut self) -> Result<CdtKey, CdtError> {
        const EXPECTED: &str = "a map key: an IRI, an RDF literal, a number or a boolean";
        match self.peek() {
            Some(b'<') if !self.starts_with("<<(") => Ok(CdtKey::Iri(self.parse_iriref()?)),
            Some(b'"' | b'\'') => Ok(CdtKey::Literal(self.parse_rdf_literal()?)),
            Some(b'0'..=b'9' | b'+' | b'-' | b'.') => Ok(CdtKey::Literal(self.parse_numeric()?)),
            Some(b't' | b'f') => Ok(CdtKey::Literal(self.parse_boolean()?)),
            Some(_) => Err(CdtError::Unexpected {
                offset: self.position,
                expected: EXPECTED,
            }),
            None => Err(CdtError::UnexpectedEnd {
                offset: self.position,
                expected: EXPECTED,
            }),
        }
    }

    /// `IRIREF ::= '<' ([^<>"{}|^\`\\] - [#x00-#x20])* '>'`, with `UCHAR` escapes,
    /// followed by the absolute-IRI constraint.
    fn parse_iriref(&mut self) -> Result<String, CdtError> {
        let start = self.position;
        self.expect(b'<', "`<` opening an IRI")?;
        let mut out = String::new();
        loop {
            match self.peek() {
                None => {
                    return Err(CdtError::UnexpectedEnd {
                        offset: self.position,
                        expected: "`>` closing an IRI",
                    });
                }
                Some(b'>') => {
                    self.bump();
                    break;
                }
                Some(b'\\') => out.push(self.parse_uchar()?),
                Some(b'<' | b'"' | b'{' | b'}' | b'|' | b'^' | b'`' | 0x00..=0x20) => {
                    return Err(CdtError::Unexpected {
                        offset: self.position,
                        expected: "an IRI character (delimiters and controls must ride as \\u escapes)",
                    });
                }
                Some(_) => {
                    let ch = self.next_char().expect("peek reported a byte");
                    out.push(ch);
                }
            }
        }
        // CDT lexical forms carry no base, so only an absolute IRI is usable.
        match purrdf_iri::parse(&out) {
            Ok(iri) if iri.has_scheme() => Ok(out),
            Ok(_) => Err(CdtError::NotAbsoluteIri {
                offset: start,
                iri: out,
                reason: "a relative IRI reference has no base to resolve against here",
            }),
            Err(_) => Err(CdtError::NotAbsoluteIri {
                offset: start,
                iri: out,
                reason: "not a syntactically valid IRI",
            }),
        }
    }

    /// `BLANK_NODE_LABEL ::= '_:' (PN_CHARS_U | [0-9]) ((PN_CHARS | '.')* PN_CHARS)?`
    fn parse_blank_node_label(&mut self) -> Result<String, CdtError> {
        let start = self.position;
        self.expect(b'_', "`_:` opening a blank node label")?;
        self.expect(b':', "`_:` opening a blank node label")?;
        let body_start = self.position;
        while let Some(ch) = self.input[self.position..].chars().next() {
            if is_pn_chars(ch) || ch == '.' {
                self.position += ch.len_utf8();
            } else {
                break;
            }
        }
        let label = &self.input[body_start..self.position];
        if label.is_empty() {
            return Err(CdtError::BadBlankNodeLabel {
                offset: start,
                reason: "the label is empty",
            });
        }
        if label.ends_with('.') {
            return Err(CdtError::BadBlankNodeLabel {
                offset: start,
                reason: "the label must not end with `.`",
            });
        }
        Ok(label.to_string())
    }

    /// `[128s] RDFLiteral ::= String (LANGTAG | '^^' IRIREF)?`
    fn parse_rdf_literal(&mut self) -> Result<CdtLiteral, CdtError> {
        let lexical = self.parse_string()?;
        if self.peek() == Some(b'@') {
            let (language, direction) = self.parse_langtag()?;
            let datatype = if direction.is_some() {
                RDF_DIR_LANG_STRING
            } else {
                RDF_LANG_STRING
            };
            return Ok(CdtLiteral {
                lexical,
                datatype: datatype.to_string(),
                language: Some(language),
                direction,
            });
        }
        if self.starts_with("^^") {
            self.position += 2;
            let datatype = self.parse_iriref()?;
            return Ok(CdtLiteral::typed(lexical, datatype));
        }
        Ok(CdtLiteral::plain(lexical))
    }

    /// `String ::= STRING_LITERAL1 | STRING_LITERAL2 | STRING_LITERAL_LONG1 | STRING_LITERAL_LONG2`
    fn parse_string(&mut self) -> Result<String, CdtError> {
        let (quote, long) = match self.peek() {
            Some(b'"') if self.starts_with("\"\"\"") => (b'"', true),
            Some(b'\'') if self.starts_with("'''") => (b'\'', true),
            Some(b'"') => (b'"', false),
            Some(b'\'') => (b'\'', false),
            Some(_) => {
                return Err(CdtError::Unexpected {
                    offset: self.position,
                    expected: "a quoted string",
                });
            }
            None => {
                return Err(CdtError::UnexpectedEnd {
                    offset: self.position,
                    expected: "a quoted string",
                });
            }
        };
        self.position += if long { 3 } else { 1 };
        let mut out = String::new();
        loop {
            match self.peek() {
                None => {
                    return Err(CdtError::UnexpectedEnd {
                        offset: self.position,
                        expected: "the closing quote of a string",
                    });
                }
                Some(b'\\') => out.push(self.parse_escape()?),
                Some(found) if found == quote => {
                    if long {
                        if self.starts_with(if quote == b'"' { "\"\"\"" } else { "'''" }) {
                            self.position += 3;
                            break;
                        }
                        self.bump();
                        out.push(quote as char);
                    } else {
                        self.bump();
                        break;
                    }
                }
                Some(b'\n' | b'\r') if !long => {
                    return Err(CdtError::Unexpected {
                        offset: self.position,
                        expected: "a raw newline is not allowed in a short string",
                    });
                }
                Some(_) => {
                    let ch = self.next_char().expect("peek reported a byte");
                    out.push(ch);
                }
            }
        }
        Ok(out)
    }

    /// `ECHAR ::= '\\' [tbnrf"'\\]`, or a `UCHAR`.
    fn parse_escape(&mut self) -> Result<char, CdtError> {
        let start = self.position;
        match self.bytes.get(self.position + 1) {
            Some(b'u' | b'U') => self.parse_uchar(),
            Some(b't') => {
                self.position += 2;
                Ok('\t')
            }
            Some(b'b') => {
                self.position += 2;
                Ok('\u{8}')
            }
            Some(b'n') => {
                self.position += 2;
                Ok('\n')
            }
            Some(b'r') => {
                self.position += 2;
                Ok('\r')
            }
            Some(b'f') => {
                self.position += 2;
                Ok('\u{c}')
            }
            Some(b'"') => {
                self.position += 2;
                Ok('"')
            }
            Some(b'\'') => {
                self.position += 2;
                Ok('\'')
            }
            Some(b'\\') => {
                self.position += 2;
                Ok('\\')
            }
            Some(_) => Err(CdtError::BadEscape {
                offset: start,
                reason: "only \\t \\b \\n \\r \\f \\\" \\' \\\\ \\uXXXX and \\UXXXXXXXX are escapes",
            }),
            None => Err(CdtError::BadEscape {
                offset: start,
                reason: "the lexical form ends inside an escape sequence",
            }),
        }
    }

    /// `UCHAR ::= '\\u' HEX HEX HEX HEX | '\\U' HEX HEX HEX HEX HEX HEX HEX HEX`
    fn parse_uchar(&mut self) -> Result<char, CdtError> {
        let start = self.position;
        let digits = match self.bytes.get(self.position + 1) {
            Some(b'u') => 4usize,
            Some(b'U') => 8usize,
            _ => {
                return Err(CdtError::BadEscape {
                    offset: start,
                    reason: "expected \\uXXXX or \\UXXXXXXXX",
                });
            }
        };
        let from = self.position + 2;
        let to = from + digits;
        let Some(hex) = self.input.get(from..to) else {
            return Err(CdtError::BadEscape {
                offset: start,
                reason: "the lexical form ends inside a \\u escape",
            });
        };
        let mut value: u32 = 0;
        for byte in hex.bytes() {
            let nibble = match byte {
                b'0'..=b'9' => u32::from(byte - b'0'),
                b'a'..=b'f' => u32::from(byte - b'a') + 10,
                b'A'..=b'F' => u32::from(byte - b'A') + 10,
                _ => {
                    return Err(CdtError::BadEscape {
                        offset: start,
                        reason: "a \\u escape takes hexadecimal digits only",
                    });
                }
            };
            value = value * 16 + nibble;
        }
        let Some(ch) = char::from_u32(value) else {
            return Err(CdtError::BadEscape {
                offset: start,
                reason: "the escape does not name a Unicode scalar value",
            });
        };
        self.position = to;
        Ok(ch)
    }

    /// `LANGTAG ::= '@' [a-zA-Z]+ ('-' [a-zA-Z0-9]+)*`, plus RDF 1.2's `'--' [a-zA-Z]+`
    /// base-direction suffix.
    fn parse_langtag(&mut self) -> Result<(String, Option<TextDirection>), CdtError> {
        let start = self.position;
        self.expect(b'@', "`@` opening a language tag")?;
        let primary_start = self.position;
        while matches!(self.peek(), Some(b'a'..=b'z' | b'A'..=b'Z')) {
            self.bump();
        }
        if self.position == primary_start {
            return Err(CdtError::BadLanguageTag {
                offset: start,
                reason: "the primary subtag must be one or more letters",
            });
        }
        let mut direction = None;
        // The language tag ends where the `--` direction suffix begins; the suffix
        // is a separate component of the term, not part of the tag.
        let mut language_end = self.position;
        loop {
            if self.starts_with("--") {
                self.position += 2;
                let token_start = self.position;
                while matches!(self.peek(), Some(b'a'..=b'z' | b'A'..=b'Z')) {
                    self.bump();
                }
                let token = &self.input[token_start..self.position];
                let Some(found) = TextDirection::from_str_token(token) else {
                    return Err(CdtError::BadLanguageTag {
                        offset: start,
                        reason: "a base direction must be `ltr` or `rtl`",
                    });
                };
                direction = Some(found);
                break;
            }
            if self.peek() != Some(b'-') {
                break;
            }
            self.bump();
            let subtag_start = self.position;
            while matches!(self.peek(), Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9')) {
                self.bump();
            }
            if self.position == subtag_start {
                return Err(CdtError::BadLanguageTag {
                    offset: start,
                    reason: "a subtag must be one or more letters or digits",
                });
            }
            language_end = self.position;
        }
        Ok((self.input[start + 1..language_end].to_string(), direction))
    }

    /// `NumericLiteral`, in all three of its SPARQL shapes and all three signs.
    fn parse_numeric(&mut self) -> Result<CdtLiteral, CdtError> {
        let start = self.position;
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.bump();
        }
        let integer_digits = self.digit_run();
        let mut fraction_digits = 0usize;
        let mut has_point = false;
        if self.peek() == Some(b'.') {
            has_point = true;
            self.bump();
            fraction_digits = self.digit_run();
        }
        let mut has_exponent = false;
        if matches!(self.peek(), Some(b'e' | b'E')) {
            has_exponent = true;
            self.bump();
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.bump();
            }
            if self.digit_run() == 0 {
                return Err(CdtError::BadNumericLiteral {
                    offset: start,
                    reason: "an exponent needs at least one digit",
                });
            }
        }
        let datatype = if has_exponent {
            // DOUBLE ::= [0-9]+ '.' [0-9]* EXPONENT | '.' [0-9]+ EXPONENT | [0-9]+ EXPONENT
            let shape_ok = if has_point {
                integer_digits > 0 || fraction_digits > 0
            } else {
                integer_digits > 0
            };
            if !shape_ok {
                return Err(CdtError::BadNumericLiteral {
                    offset: start,
                    reason: "a double needs at least one digit before the exponent",
                });
            }
            XSD_DOUBLE
        } else if has_point {
            // DECIMAL ::= [0-9]* '.' [0-9]+
            if fraction_digits == 0 {
                return Err(CdtError::BadNumericLiteral {
                    offset: start,
                    reason: "a decimal needs at least one digit after the `.`",
                });
            }
            XSD_DECIMAL
        } else {
            // INTEGER ::= [0-9]+
            if integer_digits == 0 {
                return Err(CdtError::BadNumericLiteral {
                    offset: start,
                    reason: "an integer needs at least one digit",
                });
            }
            XSD_INTEGER
        };
        Ok(CdtLiteral::typed(
            &self.input[start..self.position],
            datatype,
        ))
    }

    fn digit_run(&mut self) -> usize {
        let start = self.position;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.bump();
        }
        self.position - start
    }

    /// `BooleanLiteral ::= 'true' | 'false'`
    fn parse_boolean(&mut self) -> Result<CdtLiteral, CdtError> {
        if self.starts_with("true") {
            self.position += 4;
            return Ok(CdtLiteral::typed("true", XSD_BOOLEAN));
        }
        if self.starts_with("false") {
            self.position += 5;
            return Ok(CdtLiteral::typed("false", XSD_BOOLEAN));
        }
        Err(CdtError::Unexpected {
            offset: self.position,
            expected: "the boolean literal `true` or `false`",
        })
    }
}

/// Append a finished element to the top frame.
fn push_item(stack: &mut [Frame], term: CdtTerm) {
    match stack
        .last_mut()
        .expect("the frame stack is never empty here")
    {
        Frame::List(items) | Frame::Triple(items) => items.push(term),
        Frame::Map { entries, pending } => {
            let (offset, key) = pending
                .take()
                .expect("a map value is only read after its key");
            entries.push((offset, CdtEntry { key, value: term }));
        }
    }
}

/// Sort a map's entries into key order and reject duplicate keys.
fn finish_map(mut entries: Vec<(usize, CdtEntry)>) -> Result<CdtValue, CdtError> {
    entries.sort_by(|(_, left), (_, right)| crate::ops::total_key_cmp(&left.key, &right.key));
    for window in entries.windows(2) {
        let (left_offset, left) = &window[0];
        let (right_offset, right) = &window[1];
        if left.key == right.key {
            return Err(CdtError::DuplicateMapKey {
                offset: *left_offset.max(right_offset),
                key: canonical_key_lexical(&left.key),
            });
        }
    }
    Ok(CdtValue::Map(
        entries.into_iter().map(|(_, entry)| entry).collect(),
    ))
}

/// `PN_CHARS` (SPARQL) — the character class a blank node label body admits.
fn is_pn_chars(ch: char) -> bool {
    is_pn_chars_u(ch)
        || ch == '-'
        || ch.is_ascii_digit()
        || ch == '\u{b7}'
        || matches!(ch, '\u{300}'..='\u{36f}' | '\u{203f}'..='\u{2040}')
}

/// `PN_CHARS_U ::= PN_CHARS_BASE | '_'`
fn is_pn_chars_u(ch: char) -> bool {
    ch == '_' || is_pn_chars_base(ch)
}

/// `PN_CHARS_BASE` (SPARQL).
fn is_pn_chars_base(ch: char) -> bool {
    ch.is_ascii_alphabetic()
        || matches!(ch,
            '\u{c0}'..='\u{d6}'
            | '\u{d8}'..='\u{f6}'
            | '\u{f8}'..='\u{2ff}'
            | '\u{370}'..='\u{37d}'
            | '\u{37f}'..='\u{1fff}'
            | '\u{200c}'..='\u{200d}'
            | '\u{2070}'..='\u{218f}'
            | '\u{2c00}'..='\u{2fef}'
            | '\u{3001}'..='\u{d7ff}'
            | '\u{f900}'..='\u{fdcf}'
            | '\u{fdf0}'..='\u{fffd}'
            | '\u{10000}'..='\u{effff}')
}
