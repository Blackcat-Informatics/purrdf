// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! nesting depth costs heap, not stack: a `[[[[…` a million deep is a million frames
//! on the heap, an ordinary value, and never the uncatchable `abort` a stack overflow
//! would be. What bounds it is [`MAX_ELEMENTS`] — every level is an element of the one
//! that holds it — and [`MAX_LEXICAL_BYTES`]; there is no depth bound of its own.
//!
//! Each frame that closes measures the value it produced from the extents its
//! children already carry, so the scan is linear in the input however the input
//! nests, and the byte bound on the canonical form is answered at the root from the
//! root's own measure.

use alloc::string::String;
use alloc::vec::Vec;

use purrdf_iri::{langtag, terminals};

use crate::datatype::{CdtDatatype, XSD_BOOLEAN, XSD_DECIMAL, XSD_DOUBLE, XSD_INTEGER};
use crate::error::CdtError;
use crate::limits::{MAX_ELEMENTS, MAX_LEXICAL_BYTES, try_list_extent, try_map_extent};
use crate::memory::{CdtMemory, Memory, ReadError, Resident, Storage};
use crate::render::try_canonical_key_lexical;
use crate::term::{CdtEntry, CdtKey, CdtLiteral, CdtTerm, CdtTripleTerm, TextDirection};
use crate::value::CdtValue;

/// The acceptance language a `LANGTAG` inside a CDT lexical form is decided
/// against.
///
/// [`langtag::Profile::ConcreteSyntaxLangtagBounded`]: the `LANGTAG` terminal
/// the CDT grammar borrows verbatim from Turtle, plus the RFC 5646 §2.1
/// eight-character subtag ceiling outside private use. A CDT literal embeds RDF
/// terms in its own lexical form and those terms are serialized into ordinary
/// RDF documents, so its acceptance language has to be the one every native
/// codec in this workspace names — anything wider lets a `cdt:List` hold a
/// literal that no serialization of it can be read back from.
const LANGTAG_PROFILE: langtag::Profile = langtag::Profile::ConcreteSyntaxLangtagBounded;

/// Parse a `cdt:List` lexical form.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtDatatype, parse_list};
///
/// let value = parse_list("[1, \"a\", null]")?;
/// assert_eq!(value.len(), 3);
/// assert_eq!(value.datatype(), CdtDatatype::List);
/// assert!(value.as_list().is_some());
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
    match try_parse_cdt(lexical, datatype, &mut Resident) {
        Ok((value, _)) => Ok(value),
        Err(ReadError::Lexical(error)) => Err(error),
        Err(ReadError::Storage(error)) => panic!("resident composite allocation failed: {error}"),
    }
}

/// Run the same scanner with a caller's physical admission and fallible boxes.
/// Returns the parsed value and the exact surviving requested heap bytes. The
/// caller must retain its original storage admission with the returned value;
/// lexical diagnostics likewise die before that admission is released.
///
/// # Errors
/// The existing lexical diagnostic or a distinct native physical failure.
pub fn try_parse_cdt(
    lexical: &str,
    datatype: CdtDatatype,
    storage: &mut impl Storage,
) -> Result<(CdtValue, usize), ReadError> {
    if lexical.len() > MAX_LEXICAL_BYTES {
        return Err(CdtError::InputTooLarge {
            offset: MAX_LEXICAL_BYTES,
            length: lexical.len(),
        }
        .into());
    }
    let mut scanner = Scanner::new(lexical, storage);
    let value = scanner.parse_root(datatype)?;
    scanner.skip_whitespace();
    if scanner.position < scanner.bytes.len() {
        return Err(CdtError::TrailingText {
            offset: scanner.position,
        }
        .into());
    }
    // The canonical form spells every shorthand out, so it can be far longer than the
    // input that was accepted: `[1]` is three bytes in and forty-eight out. The
    // invariant is on the value, not on the input, so the form the value would be
    // *written* as is checked too — from the measure the root frame accumulated as it
    // closed, which every frame below it computed with the same walker
    // `canonical_lexical` uses, so the two can never disagree about what the bytes
    // would be.
    let bytes = crate::render::canonical_lexical_len(&value);
    if bytes > MAX_LEXICAL_BYTES {
        return Err(CdtError::InputTooLarge {
            offset: MAX_LEXICAL_BYTES,
            length: bytes,
        }
        .into());
    }
    Ok((value, scanner.memory.admitted_bytes()))
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

struct Scanner<'a, 'm> {
    memory: Memory<'m>,
    input: &'a str,
    bytes: &'a [u8],
    position: usize,
}

impl<'a, 'm> Scanner<'a, 'm> {
    fn new(input: &'a str, storage: &'m mut dyn Storage) -> Self {
        Self {
            memory: Memory::new(storage),
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
    fn expect(&mut self, byte: u8, expected: &'static str) -> Result<(), ReadError> {
        match self.peek() {
            Some(found) if found == byte => {
                self.bump();
                Ok(())
            }
            Some(_) => Err(CdtError::Unexpected {
                offset: self.position,
                expected,
            }
            .into()),
            None => Err(CdtError::UnexpectedEnd {
                offset: self.position,
                expected,
            }
            .into()),
        }
    }

    fn expect_str(&mut self, text: &'static str, expected: &'static str) -> Result<(), ReadError> {
        if self.starts_with(text) {
            self.position += text.len();
            Ok(())
        } else if self.position < self.bytes.len() {
            Err(CdtError::Unexpected {
                offset: self.position,
                expected,
            }
            .into())
        } else {
            Err(CdtError::UnexpectedEnd {
                offset: self.position,
                expected,
            }
            .into())
        }
    }

    /// Consume and return the next Unicode scalar value.
    fn next_char(&mut self) -> Option<char> {
        let ch = self.input[self.position..].chars().next()?;
        self.position += ch.len_utf8();
        Some(ch)
    }

    /// The whole two-state machine. Returns the root composite.
    fn parse_root(&mut self, datatype: CdtDatatype) -> Result<CdtValue, ReadError> {
        self.skip_whitespace();
        self.expect(
            datatype.open(),
            match datatype {
                CdtDatatype::List => "`[` opening a cdt:List",
                CdtDatatype::Map => "`{` opening a cdt:Map",
            },
        )?;

        let mut stack: Vec<Frame> = Vec::new();
        self.memory.push(&mut stack, Frame::new(datatype))?;
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
                            self.memory.release_vec(stack)?;
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
                        }
                        .into());
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
                        self.position += if matches!(frame, Frame::Triple(_)) {
                            3
                        } else {
                            1
                        };
                        self.memory.push(&mut stack, frame)?;
                        continue;
                    }
                    let term = self.parse_element()?;
                    push_item(&mut stack, term, &mut self.memory)?;
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
                            self.memory.release_vec(stack)?;
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
                                self.memory.release_vec(stack)?;
                                return Ok(value);
                            }
                        }
                        Some(_) => {
                            return Err(CdtError::Unexpected {
                                offset: self.position,
                                expected: "`,` or the closing delimiter",
                            }
                            .into());
                        }
                        None => {
                            return Err(CdtError::UnexpectedEnd {
                                offset: self.position,
                                expected: "`,` or the closing delimiter",
                            }
                            .into());
                        }
                    }
                }
            }
        }
    }

    /// Pop the top frame. Returns `Some` when the root composite just closed, and
    /// otherwise appends the finished element to its parent.
    fn close_frame(&mut self, stack: &mut Vec<Frame>) -> Result<Option<CdtValue>, ReadError> {
        let frame = stack.pop().expect("the frame stack is never empty here");
        let term = match frame {
            Frame::List(items) => {
                let extent = try_list_extent(items.iter(), &mut self.memory)?;
                let value = CdtValue::from_checked_items(items, extent);
                if stack.is_empty() {
                    return Ok(Some(value));
                }
                CdtTerm::Composite(self.memory.boxed_value(value)?)
            }
            Frame::Map { entries, .. } => {
                let value = finish_map(entries, &mut self.memory)?;
                if stack.is_empty() {
                    return Ok(Some(value));
                }
                CdtTerm::Composite(self.memory.boxed_value(value)?)
            }
            Frame::Triple(mut parts) => {
                let object = parts.pop().expect("a triple frame closes with three parts");
                let predicate = parts.pop().expect("a triple frame closes with three parts");
                let subject = parts.pop().expect("a triple frame closes with three parts");
                let triple = self.memory.boxed_triple(CdtTripleTerm {
                    subject,
                    predicate,
                    object,
                })?;
                self.memory.release_vec(parts)?;
                CdtTerm::TripleTerm(triple)
            }
        };
        push_item(stack, term, &mut self.memory)?;
        Ok(None)
    }

    // ── Terminals ───────────────────────────────────────────────────────────────

    /// `[3]` / `[8]`, minus the composite and triple-term alternatives (which the
    /// frame machine opens itself).
    fn parse_element(&mut self) -> Result<CdtTerm, ReadError> {
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
            }.into()),
            None => Err(CdtError::UnexpectedEnd {
                offset: self.position,
                expected: "an element",
            }.into()),
        }
    }

    /// `[7] MapKey ::= IRIREF | RDFLiteral | NumericLiteral | BooleanLiteral`.
    ///
    /// Narrower than [`Self::parse_element`] by construction: a blank node, `null`,
    /// a nested composite and a triple term are all refused here, so the restriction
    /// is enforced at the one place the grammar states it.
    fn parse_key(&mut self) -> Result<CdtKey, ReadError> {
        const EXPECTED: &str = "a map key: an IRI, an RDF literal, a number or a boolean";
        match self.peek() {
            Some(b'<') if !self.starts_with("<<(") => Ok(CdtKey::Iri(self.parse_iriref()?)),
            Some(b'"' | b'\'') => Ok(CdtKey::Literal(self.parse_rdf_literal()?)),
            Some(b'0'..=b'9' | b'+' | b'-' | b'.') => Ok(CdtKey::Literal(self.parse_numeric()?)),
            Some(b't' | b'f') => Ok(CdtKey::Literal(self.parse_boolean()?)),
            Some(_) => Err(CdtError::Unexpected {
                offset: self.position,
                expected: EXPECTED,
            }
            .into()),
            None => Err(CdtError::UnexpectedEnd {
                offset: self.position,
                expected: EXPECTED,
            }
            .into()),
        }
    }

    /// `IRIREF ::= '<' ([^<>"{}|^\`\\] - [#x00-#x20])* '>'`, with `UCHAR` escapes,
    /// followed by the absolute-IRI constraint.
    fn parse_iriref(&mut self) -> Result<String, ReadError> {
        let start = self.position;
        self.expect(b'<', "`<` opening an IRI")?;
        let mut out = String::new();
        loop {
            match self.peek() {
                None => {
                    return Err(CdtError::UnexpectedEnd {
                        offset: self.position,
                        expected: "`>` closing an IRI",
                    }
                    .into());
                }
                Some(b'>') => {
                    self.bump();
                    break;
                }
                Some(b'\\') => {
                    let ch = self.parse_uchar()?;
                    self.memory.push_char(&mut out, ch)?;
                }
                Some(b'<' | b'"' | b'{' | b'}' | b'|' | b'^' | b'`' | 0x00..=0x20) => {
                    return Err(CdtError::Unexpected {
                        offset: self.position,
                        expected: "an IRI character (delimiters and controls must ride as \\u escapes)",
                    }.into());
                }
                Some(_) => {
                    let ch = self.next_char().expect("peek reported a byte");
                    self.memory.push_char(&mut out, ch)?;
                }
            }
        }
        // CDT lexical forms carry no base, so only an absolute IRI is usable.
        match purrdf_iri::absolute_verdict(&out) {
            Some(true) => Ok(out),
            Some(false) => Err(CdtError::NotAbsoluteIri {
                offset: start,
                iri: out,
                reason: "a relative IRI reference has no base to resolve against here",
            }
            .into()),
            None => Err(CdtError::NotAbsoluteIri {
                offset: start,
                iri: out,
                reason: "not a syntactically valid IRI",
            }
            .into()),
        }
    }

    /// `BLANK_NODE_LABEL ::= '_:' (PN_CHARS_U | [0-9]) ((PN_CHARS | '.')* PN_CHARS)?`
    ///
    /// The label body is scanned with
    /// [`terminals::is_pn_chars`], the workspace's single
    /// transcription of
    ///
    /// > `PN_CHARS ::= PN_CHARS_U | '-' | [0-9] | #xB7 | [#x300-#x36F] |`
    /// > `[#x203F-#x2040]`
    ///
    /// (SPARQL 1.2 §19.8 / Turtle 1.2 §6.5), rather than a copy kept here. The
    /// class is what decides where the label STOPS, not merely what may appear
    /// inside it: the loop below is maximal-munch, so a class one scalar too
    /// wide does not widen the accepted language, it moves the token boundary
    /// and re-reads a document both spellings accept. U+00A0 NO-BREAK SPACE is
    /// the standing example — it is neither `WS` nor `PN_CHARS`, so it can only
    /// end the label and then fail, and any local table that admitted it would
    /// swallow the following term instead.
    ///
    /// # The head is a different class from the tail
    ///
    /// The first scalar after `_:` is `( PN_CHARS_U | [0-9] )` —
    /// [`terminals::is_blank_node_label_start`] — which is strictly
    /// narrower than the `PN_CHARS` the rest of the label is made of. `'-'`,
    /// U+00B7 MIDDLE DOT, the combining marks `[#x300-#x36F]` and the ties
    /// `[#x203F-#x2040]` may continue a label and may not open one, and `'.'`
    /// may appear only between name characters. Answering the head with the tail
    /// class admitted `_:-a`, `_:.a` and `_:\u{300}a`, none of which any
    /// conforming parser reads and none of which
    /// `purrdf_rdf_core::blank_label::is_valid_blank_node_label` — the same
    /// production on egress — will emit.
    ///
    /// The lawful neighbours this must not touch: `_:0a` and `_:_a` (a digit and
    /// an underscore ARE the head class), `_:a-b` and `_:a.b` (hyphen and
    /// internal dot in the tail), and `_:café` in either normalization.
    fn parse_blank_node_label(&mut self) -> Result<String, ReadError> {
        let start = self.position;
        self.expect(b'_', "`_:` opening a blank node label")?;
        self.expect(b':', "`_:` opening a blank node label")?;
        let body_start = self.position;
        match self.input[self.position..].chars().next() {
            Some(ch) if terminals::is_blank_node_label_start(ch) => {
                self.position += ch.len_utf8();
            }
            _ => {
                return Err(CdtError::BadBlankNodeLabel {
                    offset: start,
                    reason: "the label must begin with PN_CHARS_U or [0-9]",
                }
                .into());
            }
        }
        while let Some(ch) = self.input[self.position..].chars().next() {
            if terminals::is_pn_chars(ch) || ch == '.' {
                self.position += ch.len_utf8();
            } else {
                break;
            }
        }
        let label = &self.input[body_start..self.position];
        if label.ends_with('.') {
            return Err(CdtError::BadBlankNodeLabel {
                offset: start,
                reason: "the label must not end with `.`",
            }
            .into());
        }
        Ok(self.memory.string(label)?)
    }

    /// `[128s] RDFLiteral ::= String (LANGTAG | '^^' IRIREF)?`
    fn parse_rdf_literal(&mut self) -> Result<CdtLiteral, ReadError> {
        let lexical = self.parse_string()?;
        if self.peek() == Some(b'@') {
            let (language, direction) = self.parse_langtag()?;
            let datatype = purrdf_iri::vocab::language_datatype_iri(direction.is_some());
            return Ok(CdtLiteral {
                lexical,
                datatype: self.memory.string(datatype)?,
                language: Some(language),
                direction,
            });
        }
        if self.starts_with("^^") {
            self.position += 2;
            let datatype = self.parse_iriref()?;
            return Ok(CdtLiteral {
                lexical,
                datatype,
                language: None,
                direction: None,
            });
        }
        Ok(CdtLiteral {
            lexical,
            datatype: self.memory.string(crate::datatype::XSD_STRING)?,
            language: None,
            direction: None,
        })
    }

    /// `String ::= STRING_LITERAL1 | STRING_LITERAL2 | STRING_LITERAL_LONG1 | STRING_LITERAL_LONG2`
    fn parse_string(&mut self) -> Result<String, ReadError> {
        let (quote, long) = match self.peek() {
            Some(b'"') if self.starts_with("\"\"\"") => (b'"', true),
            Some(b'\'') if self.starts_with("'''") => (b'\'', true),
            Some(b'"') => (b'"', false),
            Some(b'\'') => (b'\'', false),
            Some(_) => {
                return Err(CdtError::Unexpected {
                    offset: self.position,
                    expected: "a quoted string",
                }
                .into());
            }
            None => {
                return Err(CdtError::UnexpectedEnd {
                    offset: self.position,
                    expected: "a quoted string",
                }
                .into());
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
                    }
                    .into());
                }
                Some(b'\\') => {
                    let ch = self.parse_escape()?;
                    self.memory.push_char(&mut out, ch)?;
                }
                Some(found) if found == quote => {
                    if long {
                        if self.starts_with(if quote == b'"' { "\"\"\"" } else { "'''" }) {
                            self.position += 3;
                            break;
                        }
                        self.bump();
                        self.memory.push_char(&mut out, quote as char)?;
                    } else {
                        self.bump();
                        break;
                    }
                }
                Some(b'\n' | b'\r') if !long => {
                    return Err(CdtError::Unexpected {
                        offset: self.position,
                        expected: "a raw newline is not allowed in a short string",
                    }
                    .into());
                }
                Some(_) => {
                    let ch = self.next_char().expect("peek reported a byte");
                    self.memory.push_char(&mut out, ch)?;
                }
            }
        }
        Ok(out)
    }

    /// `ECHAR ::= '\\' [tbnrf"'\\]`, or a `UCHAR`.
    fn parse_escape(&mut self) -> Result<char, ReadError> {
        let start = self.position;
        match self.bytes.get(self.position + 1) {
            Some(b'u' | b'U') => self.parse_uchar(),
            Some(&letter) => {
                let decoded = terminals::echar_value(letter).ok_or(CdtError::BadEscape {
                    offset: start,
                    reason: "only \\t \\b \\n \\r \\f \\\" \\' \\\\ \\uXXXX and \\UXXXXXXXX are escapes",
                })?;
                self.position += 2;
                Ok(decoded)
            }
            None => Err(CdtError::BadEscape {
                offset: start,
                reason: "the lexical form ends inside an escape sequence",
            }
            .into()),
        }
    }

    /// `UCHAR ::= '\\u' HEX HEX HEX HEX | '\\U' HEX HEX HEX HEX HEX HEX HEX HEX`
    fn parse_uchar(&mut self) -> Result<char, ReadError> {
        let start = self.position;
        let (decoded, consumed) =
            terminals::decode_uchar(&self.bytes[start..]).map_err(|defect| {
                CdtError::BadEscape {
                    offset: start,
                    reason: match defect {
                        terminals::UcharError::NotAnEscape => "expected \\uXXXX or \\UXXXXXXXX",
                        terminals::UcharError::BadHex => {
                            if self.bytes.len() - start
                                < match self.bytes.get(start + 1) {
                                    Some(b'U') => 10,
                                    _ => 6,
                                }
                            {
                                "the lexical form ends inside a \\u escape"
                            } else {
                                "a \\u escape takes hexadecimal digits only"
                            }
                        }
                        terminals::UcharError::NotAScalar => {
                            "the escape does not name a Unicode scalar value"
                        }
                    },
                }
            })?;
        self.position = start + consumed;
        Ok(decoded)
    }

    /// `LANGTAG ::= '@' [a-zA-Z]+ ('-' [a-zA-Z0-9]+)*`, plus RDF 1.2's `'--' [a-zA-Z]+`
    /// base-direction suffix.
    ///
    /// Two jobs, and only the first is this function's own. **Finding the token's
    /// extent** is: the tag runs to the first character that is neither
    /// alphanumeric nor `-`, or to the `--` that opens the direction suffix,
    /// whichever comes first. That boundary rule is what keeps `@en--ltr` a tag
    /// plus a direction rather than a tag spelled `en--ltr`, and it is local to
    /// this scanner because only this scanner knows where the surrounding CDT
    /// literal continues.
    ///
    /// **Deciding whether the extent is a language tag** is not. That judgement
    /// belongs to [`purrdf_iri::langtag`], the workspace's single owner of the
    /// grammar, under [`LANGTAG_PROFILE`]. The hand-rolled scan that used to
    /// decide it here was a fourth private transcription, and it was looser than
    /// every codec that reads the datasets a CDT literal names: `@cantbethislong`
    /// parsed here and is refused by N-Triples, so a `cdt:List` could hold a
    /// literal no serialization of it could round-trip.
    ///
    /// Each refusal keeps its [`CdtError::BadLanguageTag`] shape and its `offset`;
    /// `reason` now carries [`langtag::LanguageTagError::message`], which is a
    /// `&'static str` and so fits the field exactly, naming the production that
    /// refused instead of restating the terminal.
    fn parse_langtag(&mut self) -> Result<(String, Option<TextDirection>), ReadError> {
        let start = self.position;
        self.expect(b'@', "`@` opening a language tag")?;
        // The language tag ends where the `--` direction suffix begins; the suffix
        // is a separate component of the term, not part of the tag.
        let language_start = self.position;
        while matches!(
            self.peek(),
            Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-')
        ) && !self.starts_with("--")
        {
            self.bump();
        }
        let language_end = self.position;
        let direction = self.parse_direction_suffix(start)?;
        let language = &self.input[language_start..language_end];
        if let Err(error) = langtag::parse_with(language, LANGTAG_PROFILE) {
            return Err(CdtError::BadLanguageTag {
                offset: start,
                reason: error.message(),
            }
            .into());
        }
        Ok((self.memory.string(language)?, direction))
    }

    /// RDF 1.2's `'--' ('ltr' | 'rtl')` base-direction suffix, when one follows
    /// the language tag; `Ok(None)` when none does.
    ///
    /// The suffix is a separate component of the term, so it is parsed apart
    /// from the tag and never folded into it. It is also narrowed by prose
    /// rather than by the terminal — RDF 1.2 admits `'--' [a-zA-Z]+` but fixes
    /// the vocabulary at exactly `ltr` and `rtl`, lower case — so an unknown or
    /// wrongly-cased suffix is an error here rather than a silently dropped
    /// direction. `offset` is the whole tag's, matching every other refusal this
    /// production makes.
    fn parse_direction_suffix(&mut self, start: usize) -> Result<Option<TextDirection>, ReadError> {
        if !self.starts_with("--") {
            return Ok(None);
        }
        self.position += 2;
        let token_start = self.position;
        while matches!(self.peek(), Some(b'a'..=b'z' | b'A'..=b'Z')) {
            self.bump();
        }
        let token = &self.input[token_start..self.position];
        TextDirection::from_str_token(token)
            .map(Some)
            .ok_or(CdtError::BadLanguageTag {
                offset: start,
                reason: "a base direction must be `ltr` or `rtl`",
            })
            .map_err(Into::into)
    }

    /// `NumericLiteral`, in all three of its SPARQL shapes and all three signs.
    fn parse_numeric(&mut self) -> Result<CdtLiteral, ReadError> {
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
                }
                .into());
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
                }
                .into());
            }
            XSD_DOUBLE
        } else if has_point {
            // DECIMAL ::= [0-9]* '.' [0-9]+
            if fraction_digits == 0 {
                return Err(CdtError::BadNumericLiteral {
                    offset: start,
                    reason: "a decimal needs at least one digit after the `.`",
                }
                .into());
            }
            XSD_DECIMAL
        } else {
            // INTEGER ::= [0-9]+
            if integer_digits == 0 {
                return Err(CdtError::BadNumericLiteral {
                    offset: start,
                    reason: "an integer needs at least one digit",
                }
                .into());
            }
            XSD_INTEGER
        };
        Ok(CdtLiteral {
            lexical: self.memory.string(&self.input[start..self.position])?,
            datatype: self.memory.string(datatype)?,
            language: None,
            direction: None,
        })
    }

    fn digit_run(&mut self) -> usize {
        let start = self.position;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.bump();
        }
        self.position - start
    }

    /// `BooleanLiteral ::= 'true' | 'false'`
    fn parse_boolean(&mut self) -> Result<CdtLiteral, ReadError> {
        if self.starts_with("true") {
            self.position += 4;
            return self.boolean_literal("true");
        }
        if self.starts_with("false") {
            self.position += 5;
            return self.boolean_literal("false");
        }
        Err(CdtError::Unexpected {
            offset: self.position,
            expected: "the boolean literal `true` or `false`",
        }
        .into())
    }

    fn boolean_literal(&mut self, lexical: &str) -> Result<CdtLiteral, ReadError> {
        Ok(CdtLiteral {
            lexical: self.memory.string(lexical)?,
            datatype: self.memory.string(XSD_BOOLEAN)?,
            language: None,
            direction: None,
        })
    }
}

/// Append a finished element to the top frame.
fn push_item(stack: &mut [Frame], term: CdtTerm, memory: &mut Memory<'_>) -> Result<(), ReadError> {
    match stack
        .last_mut()
        .expect("the frame stack is never empty here")
    {
        Frame::List(items) | Frame::Triple(items) => memory.push(items, term)?,
        Frame::Map { entries, pending } => {
            let (offset, key) = pending
                .take()
                .expect("a map value is only read after its key");
            memory.push(entries, (offset, CdtEntry { key, value: term }))?;
        }
    }
    Ok(())
}

/// Sort a map's entries into key order and reject duplicate keys.
fn finish_map(
    mut entries: Vec<(usize, CdtEntry)>,
    memory: &mut Memory<'_>,
) -> Result<CdtValue, ReadError> {
    // Offsets strictly follow authoring order. This total tie-break reproduces
    // stable key sorting (including the exact first duplicate diagnostic) with
    // the native allocation-free unstable sorter and no hidden merge scratch.
    entries.sort_unstable_by(|(lo, left), (ro, right)| {
        crate::ops::total_key_cmp(&left.key, &right.key).then_with(|| lo.cmp(ro))
    });
    for window in entries.windows(2) {
        let (left_offset, left) = &window[0];
        let (right_offset, right) = &window[1];
        if left.key == right.key {
            let key = try_canonical_key_lexical(&left.key, memory)?;
            return Err(CdtError::DuplicateMapKey {
                offset: *left_offset.max(right_offset),
                key,
            }
            .into());
        }
    }
    let old_bytes = core::alloc::Layout::array::<(usize, CdtEntry)>(entries.capacity())
        .map_err(|_| crate::memory::StorageError::SizeOverflow)?
        .size();
    let mut output = Vec::new();
    memory.reserve(&mut output, entries.len())?;
    // Move entries while the old and destination arrays are both admitted.
    let mut source = entries.into_iter();
    for (_, entry) in source.by_ref() {
        output.push(entry);
    }
    drop(source);
    memory.release_bytes(old_bytes)?;
    let extent = try_map_extent(
        output.iter().map(|entry| (&entry.key, &entry.value)),
        memory,
    )?;
    Ok(CdtValue::from_checked_entries(output, extent))
}

// `PN_CHARS`, `PN_CHARS_U` and `PN_CHARS_BASE` were transcribed here once, as a
// fourth independent copy of a table the workspace already owns. The copies
// agreed, which is exactly why the arrangement was dangerous: four tables that
// agree today are four tables that can stop agreeing one edit at a time, and a
// scanner's class is a token BOUNDARY, so a divergence is silent. The single
// transcription now lives in `purrdf_iri::terminals`, with the W3C production
// quoted at each predicate and its ranges proved sorted and disjoint at compile
// time; `parse_blank_node_label` reaches it directly.
