// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The SHACL engine's native RDF 1.2 term value model.
//!
//! The engine, constraint evaluator, path evaluator, shape parser, and report all
//! work over ONE term value type. Historically that type was
//! `oxigraph::model::Term`; this module replaces it with an oxigraph-free native
//! model built from `String` IRIs and [`purrdf::ir::TermRef`] resolution.
//!
//! # Rendering contract (behavior-preserving)
//!
//! `Term::to_string` reproduces oxigraph's `Term::to_string()` **byte-for-byte**,
//! because the engine uses the string rendering as its deterministic sort key
//! ([`crate::engine`]) and the report serialization / Python surface
//! ([`crate::report`]) compare on it. The contract verified against
//! oxigraph 0.5 is:
//!
//! - IRI → `<iri>`
//! - blank node → `_:label`
//! - plain `xsd:string` / lang-string-typed literal → `"lex"` (NO datatype)
//! - other typed literal → `"lex"^^<datatype>`
//! - language-tagged literal → `"lex"@tag` (plus `--ltr`/`--rtl` when directional)
//! - quoted triple → `<<( <s> <p> <o> )>>`
//!
//! Literal lexical forms escape `\\ \" \n \r \t` plus C0 control chars as `\u00XX`,
//! exactly as oxigraph's N-Triples literal writer.

use crate::data_view::ShaclRead;

use std::cmp::Ordering;

use ::purrdf::blank_label::ESCAPE_MARKER;
use ::purrdf::{BlankScope, RdfLiteral, TermRef};
use ::purrdf::{RdfTextDirection, TermId, TermValue};
use smallvec::SmallVec;

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// Stable canonical ordering for RDF-facing values whose display form is the
/// byte-level ordering contract. Each key is rendered exactly once.
pub(crate) fn sort_canonical<T: ToString>(values: &mut [T]) {
    values.sort_by_cached_key(ToString::to_string);
}

/// Stable canonical ordering for RDF terms without materializing display keys.
pub(crate) fn sort_terms_canonical(values: &mut [Term]) {
    values.sort_by(canonical_cmp);
}

/// A native RDF term IRI (named node). Wraps a `String`; mirrors the slice of the
/// oxigraph `NamedNode` API the engine actually uses (`as_str`, `Ord`, `Display`).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NamedNode(String);

impl NamedNode {
    /// Construct from an IRI string without validation (the IR has already validated
    /// lexical well-formedness at ingest).
    #[inline]
    pub fn new_unchecked(iri: impl Into<String>) -> Self {
        Self(iri.into())
    }

    /// The IRI string.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume into the owned IRI string.
    #[inline]
    pub fn into_string(self) -> String {
        self.0
    }

    /// Wrap this IRI into a [`Term::NamedNode`].
    #[inline]
    pub fn into_term(self) -> Term {
        Term::NamedNode(self)
    }
}

impl std::fmt::Display for NamedNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{}>", self.0)
    }
}

impl From<&str> for NamedNode {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

/// A native RDF literal. Carries the lexical form, the datatype IRI (always present
/// — the IR expands `xsd:string`/`rdf:langString` per C0.1), and the optional
/// language tag + base direction.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Literal {
    lexical: String,
    datatype: String,
    language: Option<String>,
    direction: Option<RdfTextDirection>,
}

impl Literal {
    /// A plain `xsd:string` literal.
    #[inline]
    pub fn new_simple_literal(value: impl Into<String>) -> Self {
        Self {
            lexical: value.into(),
            datatype: XSD_STRING.to_owned(),
            language: None,
            direction: None,
        }
    }

    /// A typed literal with an explicit datatype IRI.
    #[inline]
    pub fn new_typed_literal(value: impl Into<String>, datatype: NamedNode) -> Self {
        Self {
            lexical: value.into(),
            datatype: datatype.0,
            language: None,
            direction: None,
        }
    }

    /// A language-tagged literal (datatype `rdf:langString`).
    #[inline]
    pub fn new_language_tagged_literal_unchecked(
        value: impl Into<String>,
        language: impl Into<String>,
    ) -> Self {
        Self {
            lexical: value.into(),
            datatype: RDF_LANG_STRING.to_owned(),
            language: Some(language.into()),
            direction: None,
        }
    }

    /// A directional language-tagged literal (RDF 1.2).
    #[inline]
    pub fn new_directional_language_tagged_literal_unchecked(
        value: impl Into<String>,
        language: impl Into<String>,
        direction: RdfTextDirection,
    ) -> Self {
        Self {
            lexical: value.into(),
            datatype: RdfLiteral::language_datatype_iri(Some(direction)).to_owned(),
            language: Some(language.into()),
            direction: Some(direction),
        }
    }

    /// The lexical form.
    #[inline]
    pub fn value(&self) -> &str {
        &self.lexical
    }

    /// The language tag, if this is a language-tagged literal.
    #[inline]
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// The datatype IRI as a [`NamedNode`] view.
    #[inline]
    pub fn datatype(&self) -> NamedNode {
        NamedNode(self.datatype.clone())
    }

    /// The datatype IRI string (allocation-free).
    #[inline]
    pub fn datatype_str(&self) -> &str {
        &self.datatype
    }

    /// The RDF 1.2 base direction, if present.
    #[inline]
    pub fn direction(&self) -> Option<RdfTextDirection> {
        self.direction
    }
}

/// A native RDF 1.2 quoted triple (statement-layer term).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Triple {
    /// The subject term.
    pub subject: Term,
    /// The predicate IRI.
    pub predicate: NamedNode,
    /// The object term.
    pub object: Term,
}

impl Triple {
    /// Construct a quoted triple from its three components.
    #[inline]
    pub fn new(subject: Term, predicate: NamedNode, object: Term) -> Self {
        Self {
            subject,
            predicate,
            object,
        }
    }
}

/// A native RDF 1.2 term — the SHACL engine's value model. Variants mirror
/// `oxigraph::model::Term` so the constraint/shape/path logic keeps its shape.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Term {
    /// An IRI.
    NamedNode(NamedNode),
    /// A blank node (label only; the IR scope-qualifies the label at conversion).
    BlankNode(String),
    /// A literal.
    Literal(Literal),
    /// A quoted triple (RDF 1.2).
    Triple(Box<Triple>),
}

/// Resolve an interned id to its borrowed IR payload.
///
/// Object-safe on purpose. [`CanonicalBytes`] has to stream a dataset term
/// WITHOUT materializing it, which means holding the dataset it resolves
/// against; making the cursor generic over the dataset would infect
/// [`canonical_cmp`] — which resolves nothing — and every one of its callers
/// with a type parameter that has no value to supply. One `&dyn` pointer, read
/// only on the id path, keeps the owned-term cursor exactly what it was.
pub(crate) trait TermResolve {
    /// The borrowed IR payload of `id` in this dataset.
    fn resolve_id(&self, id: TermId) -> TermRef<'_>;
}

impl<D: ShaclRead + ?Sized> TermResolve for D {
    #[inline]
    fn resolve_id(&self, id: TermId) -> TermRef<'_> {
        ::purrdf::DatasetView::resolve(self, id)
    }
}

#[derive(Clone, Copy)]
enum CanonicalPart<'a> {
    Raw(&'a [u8]),
    Escaped(&'a [u8]),
    Term(&'a Term),
    /// An interned term, expanded through the cursor's resolver.
    Id(TermId),
    /// The decimal scope digits of the blank-node envelope being written.
    ScopeDigits,
    /// The encoded body of the blank-node envelope being written.
    EnvelopeBody,
}

/// Allocation-free iterator over the canonical display bytes of a valid IR term,
/// held either as an owned [`Term`] or as an interned [`TermId`].
///
/// The IR limits quoted-triple nesting to 16 levels. The inline stack therefore
/// covers every dataset term without spilling; manually-constructed terms beyond
/// that bound remain correct and may spill to the `SmallVec` backing allocation.
///
/// # One envelope at a time
///
/// A blank node whose `(label, scope)` pair does not spell itself is written as
/// the [`BlankScope::qualify_label`] envelope, whose scope digits and encoded
/// body are generated here rather than into an owned `String`. The two cursors
/// for that live on the struct rather than inside the part, because parts are
/// expanded only when they reach the TOP of the stack and a part is popped only
/// once it is exhausted — so at most one envelope is ever mid-flight, even
/// inside a quoted triple whose subject and object are both scoped blanks.
struct CanonicalBytes<'a> {
    /// The dataset behind [`CanonicalPart::Id`], absent for an owned-term cursor.
    resolver: Option<&'a dyn TermResolve>,
    parts: SmallVec<[CanonicalPart<'a>; 96]>,
    /// `\uXXXX` (6) and the envelope's `_` plus six hex digits (7) are the two
    /// widest replacements a single input character expands to.
    pending_escape: [u8; 7],
    pending_len: u8,
    pending_pos: u8,
    /// The scope digits of the envelope being written.
    digits: [u8; 10],
    digits_len: u8,
    digits_pos: u8,
    /// The unwritten characters of the envelope body being written.
    body: std::str::Chars<'a>,
}

impl<'a> CanonicalBytes<'a> {
    fn empty() -> Self {
        Self {
            resolver: None,
            parts: SmallVec::new(),
            pending_escape: [0; 7],
            pending_len: 0,
            pending_pos: 0,
            digits: [0; 10],
            digits_len: 0,
            digits_pos: 0,
            body: "".chars(),
        }
    }

    fn new(term: &'a Term) -> Self {
        let mut bytes = Self::empty();
        bytes.parts.push(CanonicalPart::Term(term));
        bytes
    }

    /// A cursor over the canonical bytes of `id` as `dataset` interns it.
    ///
    /// Byte-for-byte what [`CanonicalBytes::new`] yields for
    /// `term_id_to_native(dataset, id)`, which is the equivalence
    /// `canonical_bytes_of_an_id_match_the_materialized_term` executes over
    /// every term of a dataset holding every term kind.
    fn of_id(dataset: &'a dyn TermResolve, id: TermId) -> Self {
        let mut bytes = Self::empty();
        bytes.resolver = Some(dataset);
        bytes.parts.push(CanonicalPart::Id(id));
        bytes
    }

    #[inline]
    fn queue_escape(&mut self, replacement: &[u8]) {
        self.pending_escape[..replacement.len()].copy_from_slice(replacement);
        self.pending_len = u8::try_from(replacement.len()).expect("escape fits in seven bytes");
        self.pending_pos = 0;
    }

    #[inline]
    fn queue_control_escape(&mut self, byte: u8) {
        self.pending_escape = [
            b'\\',
            b'u',
            b'0',
            b'0',
            hex_digit(u32::from(byte) >> 4),
            hex_digit(u32::from(byte) & 0x0f),
            0,
        ];
        self.pending_len = 6;
        self.pending_pos = 0;
    }

    /// Queue `_` plus the six uppercase hex digits of `scalar` — the envelope
    /// body's encoding of a character that is not an ASCII letter or digit.
    #[inline]
    fn queue_envelope_escape(&mut self, scalar: u32) {
        self.pending_escape = [
            b'_',
            hex_digit(scalar >> 20),
            hex_digit((scalar >> 16) & 0xf),
            hex_digit((scalar >> 12) & 0xf),
            hex_digit((scalar >> 8) & 0xf),
            hex_digit((scalar >> 4) & 0xf),
            hex_digit(scalar & 0xf),
        ];
        self.pending_len = 7;
        self.pending_pos = 0;
    }

    fn expand_term(&mut self, term: &'a Term) {
        match term {
            Term::NamedNode(node) => {
                self.parts.push(CanonicalPart::Raw(b">"));
                self.parts.push(CanonicalPart::Raw(node.0.as_bytes()));
                self.parts.push(CanonicalPart::Raw(b"<"));
            }
            Term::BlankNode(label) => {
                self.parts.push(CanonicalPart::Raw(label.as_bytes()));
                self.parts.push(CanonicalPart::Raw(b"_:"));
            }
            Term::Literal(literal) => {
                self.push_literal(
                    literal.lexical.as_bytes(),
                    &literal.datatype,
                    literal.language.as_deref(),
                    literal.direction,
                );
            }
            Term::Triple(triple) => {
                self.parts.push(CanonicalPart::Raw(b" )>>"));
                self.parts.push(CanonicalPart::Term(&triple.object));
                self.parts.push(CanonicalPart::Raw(b"> "));
                self.parts
                    .push(CanonicalPart::Raw(triple.predicate.0.as_bytes()));
                self.parts.push(CanonicalPart::Raw(b" <"));
                self.parts.push(CanonicalPart::Term(&triple.subject));
                self.parts.push(CanonicalPart::Raw(b"<<( "));
            }
        }
    }

    /// Expand an interned term, mirroring [`term_ref_to_native`] arm for arm so
    /// the two produce the same bytes for the same id.
    fn expand_id(&mut self, id: TermId) {
        let dataset = self
            .resolver
            .expect("an id part is only ever pushed by an id cursor");
        match dataset.resolve_id(id) {
            TermRef::Iri(iri) => {
                self.parts.push(CanonicalPart::Raw(b">"));
                self.parts.push(CanonicalPart::Raw(iri.as_bytes()));
                self.parts.push(CanonicalPart::Raw(b"<"));
            }
            TermRef::Blank { label, scope } => {
                self.push_blank(label, scope);
            }
            TermRef::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                let datatype_iri = match dataset.resolve_id(datatype) {
                    TermRef::Iri(iri) => iri,
                    other => {
                        unreachable!("a literal datatype must resolve to an IRI, got {other:?}")
                    }
                };
                self.push_literal(lexical.as_bytes(), datatype_iri, language, direction);
            }
            TermRef::Triple { s, p, o } => {
                let predicate = match dataset.resolve_id(p) {
                    TermRef::Iri(iri) => iri,
                    other => unreachable!("a triple predicate must be an IRI, got {other:?}"),
                };
                self.parts.push(CanonicalPart::Raw(b" )>>"));
                self.parts.push(CanonicalPart::Id(o));
                self.parts.push(CanonicalPart::Raw(b"> "));
                self.parts.push(CanonicalPart::Raw(predicate.as_bytes()));
                self.parts.push(CanonicalPart::Raw(b" <"));
                self.parts.push(CanonicalPart::Id(s));
                self.parts.push(CanonicalPart::Raw(b"<<( "));
            }
        }
    }

    /// `_:` followed by the bytes [`BlankScope::qualify_label`] would have
    /// written, generated rather than allocated.
    fn push_blank(&mut self, label: &'a str, scope: BlankScope) {
        if scope == BlankScope::DEFAULT && !label.starts_with(ESCAPE_MARKER) {
            // The owned-model alphabet is unconstrained, so this is the whole of
            // the encoder's verbatim rule: the label spells itself.
            self.parts.push(CanonicalPart::Raw(label.as_bytes()));
            self.parts.push(CanonicalPart::Raw(b"_:"));
            return;
        }
        debug_assert!(
            self.body.as_str().is_empty() && self.digits_pos >= self.digits_len,
            "an envelope is written to completion before the next one begins"
        );
        self.body = label.chars();
        self.digits_len = 0;
        self.digits_pos = 0;
        if scope != BlankScope::DEFAULT {
            // Canonical decimal, never zero-padded, exactly as the encoder writes
            // it; `u32::MAX` is ten digits, so the buffer always fits.
            let mut ordinal = scope.ordinal();
            let mut written = 0usize;
            while ordinal > 0 {
                self.digits[written] = b'0' + u8::try_from(ordinal % 10).expect("a decimal digit");
                ordinal /= 10;
                written += 1;
            }
            self.digits[..written].reverse();
            self.digits_len = u8::try_from(written).expect("ten digits at most");
        }
        self.parts.push(CanonicalPart::EnvelopeBody);
        self.parts.push(CanonicalPart::Raw(b"_"));
        self.parts.push(CanonicalPart::ScopeDigits);
        self.parts
            .push(CanonicalPart::Raw(ESCAPE_MARKER.as_bytes()));
        self.parts.push(CanonicalPart::Raw(b"_:"));
    }

    /// The literal rendering rule [`render_literal`] states, shared by the owned
    /// and interned arms so they cannot drift.
    fn push_literal(
        &mut self,
        lexical: &'a [u8],
        datatype: &'a str,
        language: Option<&'a str>,
        direction: Option<RdfTextDirection>,
    ) {
        if let Some(language) = language {
            if let Some(direction) = direction {
                self.parts.push(CanonicalPart::Raw(match direction {
                    RdfTextDirection::Ltr => b"--ltr",
                    RdfTextDirection::Rtl => b"--rtl",
                }));
            }
            self.parts.push(CanonicalPart::Raw(language.as_bytes()));
            self.parts.push(CanonicalPart::Raw(b"\"@"));
        } else if datatype == XSD_STRING || datatype == RDF_LANG_STRING {
            self.parts.push(CanonicalPart::Raw(b"\""));
        } else {
            self.parts.push(CanonicalPart::Raw(b">"));
            self.parts.push(CanonicalPart::Raw(datatype.as_bytes()));
            self.parts.push(CanonicalPart::Raw(b"\"^^<"));
        }
        self.parts.push(CanonicalPart::Escaped(lexical));
        self.parts.push(CanonicalPart::Raw(b"\""));
    }
}

/// One uppercase hex digit of `nibble`'s low four bits.
#[inline]
fn hex_digit(nibble: u32) -> u8 {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    HEX[(nibble & 0xf) as usize]
}

impl Iterator for CanonicalBytes<'_> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.pending_pos < self.pending_len {
                let byte = self.pending_escape[usize::from(self.pending_pos)];
                self.pending_pos += 1;
                return Some(byte);
            }

            let part = self.parts.last_mut()?;
            match part {
                CanonicalPart::Raw(bytes) => {
                    let Some((&byte, remaining)) = bytes.split_first() else {
                        self.parts.pop();
                        continue;
                    };
                    *bytes = remaining;
                    return Some(byte);
                }
                CanonicalPart::Escaped(bytes) => {
                    let Some((&byte, remaining)) = bytes.split_first() else {
                        self.parts.pop();
                        continue;
                    };
                    *bytes = remaining;
                    match byte {
                        b'\\' => self.queue_escape(b"\\\\"),
                        b'\"' => self.queue_escape(b"\\\""),
                        b'\n' => self.queue_escape(b"\\n"),
                        b'\r' => self.queue_escape(b"\\r"),
                        b'\t' => self.queue_escape(b"\\t"),
                        control if control < 0x20 => self.queue_control_escape(control),
                        other => return Some(other),
                    }
                }
                CanonicalPart::Term(term) => {
                    let term = *term;
                    self.parts.pop();
                    self.expand_term(term);
                }
                CanonicalPart::Id(id) => {
                    let id = *id;
                    self.parts.pop();
                    self.expand_id(id);
                }
                CanonicalPart::ScopeDigits => {
                    if self.digits_pos >= self.digits_len {
                        self.parts.pop();
                        continue;
                    }
                    let byte = self.digits[usize::from(self.digits_pos)];
                    self.digits_pos += 1;
                    return Some(byte);
                }
                CanonicalPart::EnvelopeBody => {
                    let Some(character) = self.body.next() else {
                        self.parts.pop();
                        continue;
                    };
                    if character.is_ascii_alphanumeric() {
                        return Some(character as u8);
                    }
                    self.queue_envelope_escape(character as u32);
                }
            }
        }
    }
}

/// Order two IRIs by their `<iri>` renderings, from the IRI bytes alone.
///
/// `None` means the answer is not decidable from the IRIs: the shorter one is a
/// prefix of the longer and the next byte of the longer is the closing `>`, which
/// a valid IRI cannot contain but an unchecked hand-built term can. The caller
/// falls back to streaming both renderings, preserving exact behaviour there.
#[inline]
fn cmp_rendered_iri(left: &[u8], right: &[u8]) -> Option<Ordering> {
    let shared = left.len().min(right.len());
    match left[..shared].cmp(&right[..shared]) {
        Ordering::Equal if left.len() == right.len() => Some(Ordering::Equal),
        Ordering::Equal if left.len() < right.len() => match b'>'.cmp(&right[shared]) {
            Ordering::Equal => None,
            order => Some(order),
        },
        Ordering::Equal => match left[shared].cmp(&b'>') {
            Ordering::Equal => None,
            order => Some(order),
        },
        order => Some(order),
    }
}

/// The rendering's LEADING byte, which alone settles every cross-kind order.
///
/// `"` (0x22) for a literal, `<` (0x3C) for an IRI, `<` again for a quoted triple
/// and `_` (0x5F) for a blank node — so literal < IRI ≍ triple < blank, and only
/// the IRI/triple pair needs more than this.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RenderKind {
    Literal,
    Iri,
    Triple,
    Blank,
}

impl RenderKind {
    #[inline]
    fn of_term(term: &Term) -> Self {
        match term {
            Term::Literal(_) => Self::Literal,
            Term::NamedNode(_) => Self::Iri,
            Term::Triple(_) => Self::Triple,
            Term::BlankNode(_) => Self::Blank,
        }
    }

    #[inline]
    fn of_ref(term: &TermRef<'_>) -> Self {
        match term {
            TermRef::Literal { .. } => Self::Literal,
            TermRef::Iri(_) => Self::Iri,
            TermRef::Triple { .. } => Self::Triple,
            TermRef::Blank { .. } => Self::Blank,
        }
    }

    /// The order the leading byte establishes, or `None` when both sides render
    /// the same leading byte and the answer needs the full streams.
    #[inline]
    fn cross_cmp(self, other: Self) -> Option<Ordering> {
        if self == other {
            return None;
        }
        let rank = |kind: Self| match kind {
            Self::Literal => 0u8,
            // An IRI and a quoted triple both open with `<`, so they are ranked
            // equal here and settled by streaming.
            Self::Iri | Self::Triple => 1,
            Self::Blank => 2,
        };
        match rank(self).cmp(&rank(other)) {
            Ordering::Equal => None,
            order => Some(order),
        }
    }
}

/// Compare two terms by the exact bytes produced by [`Term::to_string`] without
/// materializing either rendered value.
pub(crate) fn canonical_cmp(left: &Term, right: &Term) -> Ordering {
    match (left, right) {
        // These two cases cover the overwhelmingly common focus-node terms and
        // avoid constructing even the inline rendering cursor.
        (Term::BlankNode(left), Term::BlankNode(right)) => left.cmp(right),
        (Term::NamedNode(left_node), Term::NamedNode(right_node)) => {
            cmp_rendered_iri(left_node.0.as_bytes(), right_node.0.as_bytes())
                .unwrap_or_else(|| CanonicalBytes::new(left).cmp(CanonicalBytes::new(right)))
        }
        _ => RenderKind::of_term(left)
            .cross_cmp(RenderKind::of_term(right))
            .unwrap_or_else(|| CanonicalBytes::new(left).cmp(CanonicalBytes::new(right))),
    }
}

/// [`canonical_cmp`] between two terms `dataset` interns, from their ids.
///
/// **The ids themselves are never compared.** A `TermId` is an INSERTION-order
/// handle: the interner mints them as terms arrive, so id order and canonical
/// order are unrelated, and a comparator that took the integer shortcut for the
/// interned/interned pair would sort every all-interned focus set wrongly while
/// agreeing with the owned comparator on every all-foreign one. The key is
/// derived from what the id DENOTES — its term kind, then the interner's bytes —
/// exactly as it is for an owned term, and
/// `canonical_order_is_insertion_order_independent` pins that over a dataset
/// interned in deliberately anti-canonical order.
pub(crate) fn canonical_cmp_ids(dataset: &impl ShaclRead, left: TermId, right: TermId) -> Ordering {
    if left == right {
        return Ordering::Equal;
    }
    let resolver: &dyn TermResolve = dataset;
    let (left_ref, right_ref) = (resolver.resolve_id(left), resolver.resolve_id(right));
    match (&left_ref, &right_ref) {
        (TermRef::Blank { .. }, TermRef::Blank { .. }) => {
            // Both render as `_:` plus the qualified label, and the shared prefix
            // cancels — but the qualification is not always the label itself, so
            // the comparison is over the ENVELOPE bytes, streamed.
            stream_cmp_ids(resolver, left, right)
        }
        (TermRef::Iri(left_iri), TermRef::Iri(right_iri)) => {
            cmp_rendered_iri(left_iri.as_bytes(), right_iri.as_bytes())
                .unwrap_or_else(|| stream_cmp_ids(resolver, left, right))
        }
        _ => RenderKind::of_ref(&left_ref)
            .cross_cmp(RenderKind::of_ref(&right_ref))
            .unwrap_or_else(|| stream_cmp_ids(resolver, left, right)),
    }
}

/// [`canonical_cmp`] between an interned term and an owned one.
pub(crate) fn canonical_cmp_id_term(
    dataset: &impl ShaclRead,
    left: TermId,
    right: &Term,
) -> Ordering {
    let resolver: &dyn TermResolve = dataset;
    let left_ref = resolver.resolve_id(left);
    match (&left_ref, right) {
        (TermRef::Iri(left_iri), Term::NamedNode(right_node)) => {
            cmp_rendered_iri(left_iri.as_bytes(), right_node.0.as_bytes())
        }
        _ => RenderKind::of_ref(&left_ref).cross_cmp(RenderKind::of_term(right)),
    }
    .unwrap_or_else(|| CanonicalBytes::of_id(resolver, left).cmp(CanonicalBytes::new(right)))
}

/// Compare two interned terms by streaming both canonical renderings.
fn stream_cmp_ids(resolver: &dyn TermResolve, left: TermId, right: TermId) -> Ordering {
    CanonicalBytes::of_id(resolver, left).cmp(CanonicalBytes::of_id(resolver, right))
}

impl Term {
    /// Construct a blank-node term from its label.
    #[inline]
    pub fn blank(label: impl Into<String>) -> Self {
        Self::BlankNode(label.into())
    }

    /// The blank-node label, if this term is a blank node.
    #[inline]
    pub fn blank_label(&self) -> Option<&str> {
        match self {
            Self::BlankNode(b) => Some(b.as_str()),
            _ => None,
        }
    }

    /// Whether this term can occupy a subject position (IRI or blank node).
    #[inline]
    pub fn is_subject(&self) -> bool {
        matches!(self, Self::NamedNode(_) | Self::BlankNode(_))
    }

    /// Convert this native term into the owned [`RdfTerm`](purrdf::RdfTerm) model — used when
    /// building a report dataset for serialization.
    pub fn to_rdf_term(&self) -> ::purrdf::RdfTerm {
        use purrdf::{RdfLiteral, RdfTerm, RdfTriple};
        match self {
            Self::NamedNode(n) => RdfTerm::iri(n.0.clone()),
            Self::BlankNode(b) => RdfTerm::blank_node(b.clone()),
            Self::Literal(l) => {
                // The owned model carries `datatype: None` for a plain `xsd:string`
                // and for a language-tagged literal (the tag implies rdf:langString);
                // an explicit datatype otherwise — matching how the codec round-trips.
                let datatype = if l.language.is_some() || l.datatype == XSD_STRING {
                    None
                } else {
                    Some(l.datatype.clone())
                };
                RdfTerm::Literal(RdfLiteral {
                    lexical_form: l.lexical.clone(),
                    datatype,
                    language: l.language.clone(),
                    direction: l.direction,
                })
            }
            Self::Triple(t) => RdfTerm::triple(RdfTriple::new(
                t.subject.to_rdf_term(),
                t.predicate.0.clone(),
                t.object.to_rdf_term(),
            )),
        }
    }

    /// Convert this native term into a dataset-independent [`TermValue`] — the SPARQL
    /// substitution value and the canonical lookup key.
    pub fn to_term_value(&self) -> TermValue {
        match self {
            Self::NamedNode(n) => TermValue::Iri(n.0.clone()),
            // [`term_ref_to_native`] scope-qualified the label on the way out of the
            // IR; decoding it here is the exact inverse, so a native term used as a
            // SPARQL pre-binding denotes the SAME node the dataset holds rather than
            // a second, doubly-qualified one.
            Self::BlankNode(b) => {
                let (label, scope) = ::purrdf::BlankScope::unqualify_label(b);
                TermValue::Blank {
                    label: label.into_owned(),
                    scope,
                }
            }
            Self::Literal(l) => TermValue::Literal {
                lexical_form: l.lexical.clone(),
                datatype: l.datatype.clone(),
                language: l.language.clone(),
                direction: l.direction,
            },
            Self::Triple(t) => TermValue::Triple {
                s: Box::new(t.subject.to_term_value()),
                p: Box::new(t.predicate.to_term_value_iri()),
                o: Box::new(t.object.to_term_value()),
            },
        }
    }

    /// The candidate [`TermValue`] lookup keys to resolve this pattern term against a
    /// dataset's value→id index.
    ///
    /// For most terms this is a single key ([`to_term_value`](Self::to_term_value)).
    /// A blank node is the exception: [`term_ref_to_native`] flattens the IR's
    /// `(label, scope)` into ONE qualified label string (the raw label itself at
    /// the default scope, otherwise the `purrdfesc{n}_{body}` envelope), and
    /// [`to_term_value`](Self::to_term_value) decodes that spelling back to the
    /// `(label, scope)` pair the dataset holds. That decoded key is the right one
    /// for any dataset that stores the IR pair faithfully.
    ///
    /// A caller may nonetheless hand this a blank label it MINTED rather than read
    /// out of a dataset (a hand-written shapes-graph label, a test fixture), which
    /// is a raw label rather than a qualified one. Whenever the two spellings
    /// differ, the verbatim DEFAULT-scope key is offered as a fallback and the
    /// caller tries each until one resolves.
    ///
    /// # Deprecated: use [`PreparedValidator::term_id`] instead
    ///
    /// This hands out *lookup keys* and leaves the caller to run the search, try
    /// the fallback in the right order, and decide what a miss means. Nothing in
    /// this workspace does that any more — the engine resolves a [`Term`] to its
    /// interned identity internally, and the supported public route is
    /// [`PreparedValidator::term_id`], which answers the identity itself against
    /// the binding whose term table the answer indexes. That matters beyond
    /// convenience: a [`TermId`] is meaningful only relative to one
    /// dataset, and a key-returning helper cannot enforce that pairing while an
    /// accessor on the binding cannot avoid it.
    ///
    /// Deprecated rather than removed because removal is a breaking change and
    /// this release is not one; it is additive today and the attribute is how an
    /// out-of-tree caller finds out before the next major. There is no
    /// functionality here that the supported route does not cover, so nothing is
    /// waiting on a replacement.
    ///
    /// [`PreparedValidator::term_id`]: crate::engine::PreparedValidator::term_id
    // No `since`: the version this deprecation first ships in is not knowable from
    // inside the commit that writes it, and a wrong `since` is a claim about a
    // release rather than a pointer to the supported route.
    #[deprecated(
        note = "resolve a Term through PreparedValidator::term_id, which answers the interned \
                identity against the binding it indexes, instead of handing back lookup keys"
    )]
    pub fn lookup_term_values(&self) -> Vec<TermValue> {
        match self {
            Self::BlankNode(b) => {
                let decoded = self.to_term_value();
                let verbatim = TermValue::blank(b.clone());
                if decoded == verbatim {
                    vec![decoded]
                } else {
                    vec![decoded, verbatim]
                }
            }
            other => vec![other.to_term_value()],
        }
    }
}

impl NamedNode {
    /// The IRI as a [`TermValue::Iri`].
    #[inline]
    fn to_term_value_iri(&self) -> TermValue {
        TermValue::Iri(self.0.clone())
    }
}

impl std::fmt::Display for Term {
    /// Render byte-for-byte as `oxigraph::model::Term::to_string()` — the engine's
    /// deterministic sort key and report identity depend on this.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NamedNode(n) => write!(f, "<{}>", n.0),
            Self::BlankNode(b) => write!(f, "_:{b}"),
            Self::Literal(l) => write!(f, "{}", render_literal(l)),
            Self::Triple(t) => write!(f, "<<( {} <{}> {} )>>", t.subject, t.predicate.0, t.object),
        }
    }
}

/// Render a literal exactly as oxigraph's `Term::to_string()` does.
fn render_literal(l: &Literal) -> String {
    let lex = escape_literal(&l.lexical);
    if let Some(lang) = &l.language {
        return match l.direction {
            Some(RdfTextDirection::Ltr) => format!("\"{lex}\"@{lang}--ltr"),
            Some(RdfTextDirection::Rtl) => format!("\"{lex}\"@{lang}--rtl"),
            None => format!("\"{lex}\"@{lang}"),
        };
    }
    // Plain `xsd:string` (and the rare `rdf:langString` without a tag) render with
    // NO datatype suffix, matching oxigraph.
    if l.datatype == XSD_STRING || l.datatype == RDF_LANG_STRING {
        return format!("\"{lex}\"");
    }
    format!("\"{lex}\"^^<{}>", l.datatype)
}

/// Escape a literal lexical form exactly as oxigraph's N-Triples literal writer:
/// `\\ \" \n \r \t` plus C0 control characters as `\u00XX`.
fn escape_literal(s: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            other => out.push(other),
        }
    }
    out
}

/// Convert a resolved IR [`TermRef`] into a native [`Term`], recursing into triple
/// components via the dataset's [`resolve`](::purrdf::RdfDataset::resolve).
///
/// Blank labels are scope-qualified so two same-label blanks from different
/// [`BlankScope`]s never conflate (C0.2); a DEFAULT-scope
/// label outside the reserved marker namespace stays bare so single-scope data
/// is byte-unchanged.
pub fn term_ref_to_native(dataset: &impl ShaclRead, term: TermRef<'_>) -> Term {
    match term {
        TermRef::Iri(iri) => Term::NamedNode(NamedNode::new_unchecked(iri)),
        TermRef::Blank { label, scope } => Term::BlankNode(scope.qualify_label(label).into_owned()),
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            let datatype_iri = match dataset.resolve(datatype) {
                TermRef::Iri(iri) => iri.to_owned(),
                other => unreachable!("a literal datatype must resolve to an IRI, got {other:?}"),
            };
            Term::Literal(Literal {
                lexical: lexical.to_owned(),
                datatype: datatype_iri,
                language: language.map(str::to_owned),
                direction,
            })
        }
        TermRef::Triple { s, p, o } => {
            let subject = term_ref_to_native(dataset, dataset.resolve(s));
            let predicate = match term_ref_to_native(dataset, dataset.resolve(p)) {
                Term::NamedNode(n) => n,
                other => unreachable!("a triple predicate must be an IRI, got {other:?}"),
            };
            let object = term_ref_to_native(dataset, dataset.resolve(o));
            Term::Triple(Box::new(Triple::new(subject, predicate, object)))
        }
    }
}

/// Convert a resolved IR term id into a native [`Term`].
#[inline]
pub fn term_id_to_native(dataset: &impl ShaclRead, id: TermId) -> Term {
    term_ref_to_native(dataset, dataset.resolve(id))
}

/// Convert a dataset-independent [`TermValue`] (e.g. a SPARQL egress binding) into a
/// native [`Term`].
pub fn term_value_to_native(value: &TermValue) -> Term {
    match value {
        TermValue::Iri(iri) => Term::NamedNode(NamedNode::new_unchecked(iri.clone())),
        TermValue::Blank { label, .. } => Term::BlankNode(label.clone()),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => Term::Literal(Literal {
            lexical: lexical_form.clone(),
            datatype: datatype.clone(),
            language: language.clone(),
            direction: *direction,
        }),
        TermValue::Triple { s, p, o } => {
            let predicate = match term_value_to_native(p) {
                Term::NamedNode(n) => n,
                other => unreachable!("a triple predicate must be an IRI, got {other:?}"),
            };
            Term::Triple(Box::new(Triple::new(
                term_value_to_native(s),
                predicate,
                term_value_to_native(o),
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nn(iri: &str) -> Term {
        Term::NamedNode(NamedNode::new_unchecked(iri))
    }

    #[test]
    fn renders_iri_and_blank_like_oxigraph() {
        assert_eq!(nn("http://e/s").to_string(), "<http://e/s>");
        assert_eq!(Term::blank("b0").to_string(), "_:b0");
    }

    #[test]
    fn renders_plain_string_without_datatype() {
        let t = Term::Literal(Literal::new_simple_literal("hi"));
        assert_eq!(t.to_string(), "\"hi\"");
    }

    #[test]
    fn renders_typed_literal_with_datatype() {
        let t = Term::Literal(Literal::new_typed_literal(
            "42",
            NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#integer"),
        ));
        assert_eq!(
            t.to_string(),
            "\"42\"^^<http://www.w3.org/2001/XMLSchema#integer>"
        );
    }

    #[test]
    fn renders_lang_and_directional_literal() {
        let lang = Term::Literal(Literal::new_language_tagged_literal_unchecked("hi", "en"));
        assert_eq!(lang.to_string(), "\"hi\"@en");
        let dir = Term::Literal(Literal::new_directional_language_tagged_literal_unchecked(
            "hi",
            "en",
            RdfTextDirection::Rtl,
        ));
        assert_eq!(dir.to_string(), "\"hi\"@en--rtl");
    }

    #[test]
    fn renders_quoted_triple_like_oxigraph() {
        let t = Term::Triple(Box::new(Triple::new(
            NamedNode::new_unchecked("http://e/s").into_term(),
            NamedNode::new_unchecked("http://e/p"),
            NamedNode::new_unchecked("http://e/o").into_term(),
        )));
        assert_eq!(
            t.to_string(),
            "<<( <http://e/s> <http://e/p> <http://e/o> )>>"
        );
    }

    #[test]
    fn escapes_special_chars_like_oxigraph() {
        let t = Term::Literal(Literal::new_simple_literal("a\"b\nc\td\\e\u{0007}f"));
        assert_eq!(t.to_string(), "\"a\\\"b\\nc\\td\\\\e\\u0007f\"");
    }

    #[test]
    fn canonical_comparator_matches_rendered_byte_order() {
        let plain_lang_string = Term::Literal(Literal {
            lexical: "plain".to_owned(),
            datatype: RDF_LANG_STRING.to_owned(),
            language: None,
            direction: None,
        });
        let triple = Term::Triple(Box::new(Triple::new(
            nn("http://e/s"),
            NamedNode::new_unchecked("http://e/p"),
            Term::Literal(Literal::new_simple_literal("quoted\nvalue")),
        )));
        let nested_triple = Term::Triple(Box::new(Triple::new(
            triple.clone(),
            NamedNode::new_unchecked("http://e/p2"),
            Term::blank("nested"),
        )));
        let terms = vec![
            nn("http://e/a"),
            nn("http://e/a/"),
            nn("http://e/z"),
            Term::blank("a"),
            Term::blank("z"),
            Term::Literal(Literal::new_simple_literal("")),
            Term::Literal(Literal::new_simple_literal("a\"b\\c\n\r\t\u{0000}\u{001f}")),
            Term::Literal(Literal::new_simple_literal("é🐈")),
            Term::Literal(Literal::new_typed_literal(
                "42",
                NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#integer"),
            )),
            Term::Literal(Literal::new_language_tagged_literal_unchecked(
                "bonjour", "fr",
            )),
            Term::Literal(Literal::new_directional_language_tagged_literal_unchecked(
                "مرحبا",
                "ar",
                RdfTextDirection::Rtl,
            )),
            Term::Literal(Literal::new_directional_language_tagged_literal_unchecked(
                "hello",
                "en",
                RdfTextDirection::Ltr,
            )),
            plain_lang_string,
            triple,
            nested_triple,
        ];

        for left in &terms {
            for right in &terms {
                assert_eq!(
                    canonical_cmp(left, right),
                    left.to_string().cmp(&right.to_string()),
                    "canonical comparison drifted for {left:?} and {right:?}"
                );
            }
        }

        let mut expected = terms.clone();
        expected.sort_by_cached_key(ToString::to_string);
        let mut actual = terms;
        sort_terms_canonical(&mut actual);
        assert_eq!(actual, expected);
    }

    /// A dataset holding every term kind, interned in DELIBERATELY ANTI-CANONICAL
    /// order.
    ///
    /// The ids therefore ascend as the terms DESCEND in rendered byte order (for
    /// the IRIs, which are the bulk of it), so any comparator that reached for an
    /// id's number instead of what the id denotes produces a visibly reversed
    /// answer rather than a plausible one. A dataset built in the natural order
    /// would let that mistake pass.
    fn anti_canonical_dataset() -> std::sync::Arc<::purrdf::RdfDataset> {
        use ::purrdf::{BlankScope, RdfDatasetBuilder, RdfLiteral};

        let mut builder = RdfDatasetBuilder::new();
        // Descending IRIs, so id order is the reverse of canonical order.
        for local in ["z", "y", "x", "c", "b", "a"] {
            builder.intern_iri(&format!("http://example.org/{local}"));
        }
        // Blank nodes, also descending, including a NON-DEFAULT scope whose
        // rendered label is the `purrdfesc` envelope rather than the raw label.
        builder.intern_blank("zeta", BlankScope::DEFAULT);
        builder.intern_blank("alpha", BlankScope::DEFAULT);
        builder.intern_blank("scoped", BlankScope(7));
        builder.intern_blank("purrdfesc_looks_like_an_envelope", BlankScope::DEFAULT);
        builder.intern_blank("needs. escaping/é", BlankScope(2));
        // Literals of every rendered shape.
        builder.intern_literal(RdfLiteral::simple("zzz"));
        builder.intern_literal(RdfLiteral::simple(""));
        builder.intern_literal(RdfLiteral::simple("a\"b\\c\n\r\t\u{0000}\u{001f}"));
        builder.intern_literal(RdfLiteral::simple("é🐈"));
        builder.intern_literal(RdfLiteral::typed(
            "42",
            "http://www.w3.org/2001/XMLSchema#integer",
        ));
        builder.intern_literal(RdfLiteral::language_tagged("bonjour", "fr"));
        builder.intern_literal(RdfLiteral::language_tagged("hello", "en"));
        // A quoted triple, and a triple nesting it.
        let s = builder.intern_iri("http://example.org/a");
        let p = builder.intern_iri("http://example.org/p");
        let o = builder.intern_literal(RdfLiteral::simple("quoted\nvalue"));
        let inner = builder.intern_triple(s, p, o);
        // RDF 1.2 nests a triple term only in the OBJECT position, so the outer
        // triple wraps the inner one there.
        let p2 = builder.intern_iri("http://example.org/p2");
        let nested_subject = builder.intern_blank("nested", BlankScope(3));
        builder.intern_triple(nested_subject, p2, inner);
        builder.push_quad(s, p, o, None);
        builder.freeze().expect("the fixture dataset freezes")
    }

    /// **The bytes a cursor streams from an id are the bytes the materialized
    /// term renders — for every term in a dataset holding every kind.**
    ///
    /// This is the equivalence the whole id-native comparator rests on, stated
    /// directly rather than inferred from an ordering that happened to agree. If
    /// it holds, `canonical_cmp_ids` and `canonical_cmp` cannot disagree, because
    /// they are comparing the same byte sequences.
    #[test]
    fn canonical_bytes_of_an_id_match_the_materialized_term() {
        use ::purrdf::TermId;

        let dataset = anti_canonical_dataset();
        assert!(dataset.term_count() > 20, "the fixture must be non-trivial");
        let resolver: &dyn TermResolve = dataset.as_ref();
        let mut kinds = [false; 4];
        for index in 0..dataset.term_count() {
            let id = TermId::from_index(u32::try_from(index).expect("fixture fits in u32"));
            kinds[match ::purrdf::DatasetView::resolve(dataset.as_ref(), id) {
                TermRef::Iri(_) => 0,
                TermRef::Blank { .. } => 1,
                TermRef::Literal { .. } => 2,
                TermRef::Triple { .. } => 3,
            }] = true;
            let streamed: Vec<u8> = CanonicalBytes::of_id(resolver, id).collect();
            let rendered = term_id_to_native(dataset.as_ref(), id).to_string();
            assert_eq!(
                String::from_utf8(streamed).as_deref(),
                Ok(rendered.as_str()),
                "the id cursor drifted from the rendered term at id {index}"
            );
        }
        assert!(
            kinds.iter().all(|seen| *seen),
            "the fixture must hold an IRI, a blank node, a literal and a quoted triple, or the \
             equivalence is only pinned for the kinds it happens to contain"
        );
    }

    /// **Comparing two ids orders them canonically, not by insertion.**
    ///
    /// Over a dataset whose interning order is the reverse of its canonical
    /// order, so a comparator that shortcut to the ids' numbers would be exactly
    /// backwards on the IRIs rather than subtly off.
    #[test]
    fn id_comparison_is_canonical_and_not_insertion_order() {
        use ::purrdf::TermId;

        let dataset = anti_canonical_dataset();
        let ids: Vec<TermId> = (0..dataset.term_count())
            .map(|index| TermId::from_index(u32::try_from(index).expect("fixture fits in u32")))
            .collect();
        for &left in &ids {
            for &right in &ids {
                let rendered = term_id_to_native(dataset.as_ref(), left)
                    .to_string()
                    .cmp(&term_id_to_native(dataset.as_ref(), right).to_string());
                assert_eq!(
                    canonical_cmp_ids(dataset.as_ref(), left, right),
                    rendered,
                    "id comparison drifted at {}/{}",
                    left.index(),
                    right.index()
                );
                assert_eq!(
                    canonical_cmp_id_term(
                        dataset.as_ref(),
                        left,
                        &term_id_to_native(dataset.as_ref(), right)
                    ),
                    rendered,
                    "mixed id/term comparison drifted at {}/{}",
                    left.index(),
                    right.index()
                );
            }
        }

        // And the fixture really is anti-canonical: ordering the ids by their
        // NUMBERS must not be ordering them canonically, or this test would pass
        // against a comparator that never looked at the terms at all.
        let canonical_agrees_with_id_order = ids
            .windows(2)
            .all(|pair| canonical_cmp_ids(dataset.as_ref(), pair[0], pair[1]) != Ordering::Greater);
        assert!(
            !canonical_agrees_with_id_order,
            "the fixture must be interned out of canonical order, or comparing ids by number \
             would look correct"
        );
    }
}
