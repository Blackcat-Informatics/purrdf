// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The crate's **own** closed element type.
//!
//! `purrdf-cdt` is a closed leaf: it does not depend on `purrdf-core` in either
//! direction, so it cannot name the kernel's term type and does not try to. It owns
//! [`CdtTerm`], which is exactly the set of things the SEP-0009 grammar (plus this
//! crate's two documented supersets) admits as a list element or a map value —
//! nothing wider, nothing narrower. Converting between [`CdtTerm`] and a host's own
//! term representation is the *consumer's* job, and happens above this crate.
//!
//! # Literals are kept lexical-verbatim
//!
//! [`CdtLiteral`] stores the lexical form byte-for-byte as authored. That is not an
//! accident of implementation: SEP-0009 distinguishes map keys **by lexical form
//! rather than by value**, deliberately, so `"1"^^xsd:integer` and
//! `"01"^^xsd:integer` are two different keys. Canonicalizing the value at parse
//! time would silently merge them. It also matches the PurRDF kernel's own rule
//! that the IR keeps literals lexical-verbatim.

use alloc::boxed::Box;
use alloc::string::String;

use crate::datatype::{RDF_DIR_LANG_STRING, RDF_LANG_STRING};
use crate::value::CdtValue;

/// The RDF 1.2 base direction of a directional language-tagged string.
///
/// Carried exactly as the workspace's concrete syntaxes already write it: the
/// `--ltr` / `--rtl` suffix after a language tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextDirection {
    /// Left-to-right base direction (`ltr`).
    Ltr,
    /// Right-to-left base direction (`rtl`).
    Rtl,
}

impl TextDirection {
    /// The lowercase direction token (`"ltr"` or `"rtl"`) as it appears in concrete
    /// syntaxes.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
        }
    }

    /// Parse the token after `--`, or `None` when it is neither direction.
    #[must_use]
    pub fn from_str_token(token: &str) -> Option<Self> {
        match token {
            "ltr" => Some(Self::Ltr),
            "rtl" => Some(Self::Rtl),
            _ => None,
        }
    }
}

/// An RDF literal appearing inside a composite lexical form.
///
/// # Invariants
///
/// Every literal this crate constructs satisfies all of:
///
/// * `language.is_some()` **iff** `datatype` is [`RDF_LANG_STRING`] or
///   [`RDF_DIR_LANG_STRING`];
/// * `direction.is_some()` **iff** `datatype` is [`RDF_DIR_LANG_STRING`] (and then
///   `language.is_some()` too);
/// * `datatype` is a syntactically valid absolute IRI.
///
/// [`CdtLiteral::plain`], [`CdtLiteral::typed`], [`CdtLiteral::lang`] and
/// [`CdtLiteral::dir_lang`] establish them; the scanner establishes them too.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CdtLiteral {
    /// The lexical form, byte-for-byte as authored (escapes already decoded).
    pub lexical: String,
    /// The datatype IRI. Always explicit — a bare `"abc"` carries
    /// `http://www.w3.org/2001/XMLSchema#string`, a `"abc"@en` carries
    /// `rdf:langString`, and a `"abc"@en--rtl` carries `rdf:dirLangString`.
    pub datatype: String,
    /// The language tag, for language-tagged strings.
    pub language: Option<String>,
    /// The RDF 1.2 base direction, for directional language-tagged strings.
    pub direction: Option<TextDirection>,
}

impl CdtLiteral {
    /// A plain `xsd:string` literal.
    #[must_use]
    pub fn plain(lexical: impl Into<String>) -> Self {
        Self::typed(lexical, crate::datatype::XSD_STRING)
    }

    /// A datatyped literal.
    #[must_use]
    pub fn typed(lexical: impl Into<String>, datatype: impl Into<String>) -> Self {
        Self {
            lexical: lexical.into(),
            datatype: datatype.into(),
            language: None,
            direction: None,
        }
    }

    /// A language-tagged string with no base direction (`rdf:langString`).
    #[must_use]
    pub fn lang(lexical: impl Into<String>, language: impl Into<String>) -> Self {
        Self {
            lexical: lexical.into(),
            datatype: String::from(RDF_LANG_STRING),
            language: Some(language.into()),
            direction: None,
        }
    }

    /// A **directional** language-tagged string (`rdf:dirLangString`) — the RDF 1.2
    /// term type this crate's second lexical superset exists to carry.
    #[must_use]
    pub fn dir_lang(
        lexical: impl Into<String>,
        language: impl Into<String>,
        direction: TextDirection,
    ) -> Self {
        Self {
            lexical: lexical.into(),
            datatype: String::from(RDF_DIR_LANG_STRING),
            language: Some(language.into()),
            direction: Some(direction),
        }
    }
}

/// An RDF 1.2 **triple term** appearing as a composite element.
///
/// This is the payload of this crate's first documented superset of the SEP-0009
/// lexical space. The three components are unrestricted [`CdtTerm`]s because the
/// superset production spells them as `Element`; whether the resulting triple is
/// well-formed RDF is the consumer's question, not this lexical layer's.
#[derive(Debug, Clone)]
pub struct CdtTripleTerm {
    /// The subject component.
    pub subject: CdtTerm,
    /// The predicate component.
    pub predicate: CdtTerm,
    /// The object component.
    pub object: CdtTerm,
}

/// An element of a `cdt:List`, or the value of a `cdt:Map` entry.
///
/// # Nesting depth is an invariant, not a suggestion
///
/// This is an owning tree, so its `Drop`, `Clone` and `Debug` glue is the
/// compiler-generated recursive one and each costs stack proportional to the
/// nesting depth. [`crate::parse_cdt`] enforces [`crate::MAX_NESTING_DEPTH`], which
/// is what makes those safe. Code that assembles a value **programmatically** owns
/// the same invariant; [`CdtValue::depth`] reports the depth iteratively (it never
/// recurses) so it can be checked without risking what it is checking for.
#[derive(Debug, Clone)]
pub enum CdtTerm {
    /// `IRIREF` — always absolute (CDT lexical forms carry no base).
    Iri(String),
    /// `BLANK_NODE_LABEL` — the label **without** the `_:` prefix.
    Blank(String),
    /// `RDFLiteral`, `NumericLiteral` or `BooleanLiteral`. The two shorthands are
    /// resolved to their explicit datatype at parse time; the lexical form is kept
    /// verbatim.
    Literal(CdtLiteral),
    /// `<<( s p o )>>` — an RDF 1.2 triple term. **PurRDF superset** of SEP-0009.
    TripleTerm(Box<CdtTripleTerm>),
    /// A nested `List` or `Map`.
    Composite(Box<CdtValue>),
    /// `null` — the SEP-0009 null element. Nulls are mutually **indistinguishable**:
    /// `"[null]"` equals `"[null]"`.
    Null,
}

impl CdtTerm {
    /// A nested composite element.
    #[must_use]
    pub fn composite(value: CdtValue) -> Self {
        Self::Composite(Box::new(value))
    }

    /// A triple-term element.
    #[must_use]
    pub fn triple(subject: Self, predicate: Self, object: Self) -> Self {
        Self::TripleTerm(Box::new(CdtTripleTerm {
            subject,
            predicate,
            object,
        }))
    }

    /// `true` for [`CdtTerm::Null`].
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// The ordering rank of this term's category in the crate's syntactic total
    /// order (see [`crate::total_term_cmp`]).
    pub(crate) const fn rank(&self) -> u8 {
        match self {
            Self::Null => 0,
            Self::Blank(_) => 1,
            Self::Iri(_) => 2,
            Self::Literal(_) => 3,
            Self::TripleTerm(_) => 4,
            Self::Composite(_) => 5,
        }
    }
}

/// A `cdt:Map` key.
///
/// Production `[7] MapKey ::= IRIREF | RDFLiteral | NumericLiteral | BooleanLiteral`
/// — narrower than [`CdtTerm`]: a key is never a blank node, never `null`, never a
/// nested composite and never a triple term. Modelling that as its own closed enum
/// makes the restriction unrepresentable rather than merely checked.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CdtKey {
    /// `IRIREF` — always absolute.
    Iri(String),
    /// `RDFLiteral`, `NumericLiteral` or `BooleanLiteral`.
    Literal(CdtLiteral),
}

impl CdtKey {
    /// The key seen as a [`CdtTerm`] (an owning conversion; keys are always leaves,
    /// so this allocates at most the key's own strings and never recurses).
    #[must_use]
    pub fn to_term(&self) -> CdtTerm {
        match self {
            Self::Iri(iri) => CdtTerm::Iri(iri.clone()),
            Self::Literal(lit) => CdtTerm::Literal(lit.clone()),
        }
    }

    /// The key a term denotes, or `None` when the term is **not admissible as a map
    /// key** at all.
    ///
    /// The inverse of [`CdtKey::to_term`], and total in the only way it can be:
    /// production `[7] MapKey` admits an `IRIREF` and a literal and nothing else, so
    /// a blank node, a `null`, a nested composite and a triple term each have no key
    /// to denote. That is a real distinction with observable consequences in
    /// [`crate::functions`] — `cdt:put` with a blank-node key raises, while
    /// `cdt:remove` with one leaves the map alone — so it is answered here once,
    /// rather than re-derived at each call site.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_cdt::{CdtKey, CdtLiteral, CdtTerm};
    ///
    /// let literal = CdtTerm::Literal(CdtLiteral::plain("a"));
    /// assert_eq!(CdtKey::from_term(&literal), Some(CdtKey::Literal(CdtLiteral::plain("a"))));
    /// // A blank node can never be a map key.
    /// assert_eq!(CdtKey::from_term(&CdtTerm::Blank("b0".into())), None);
    /// assert_eq!(CdtKey::from_term(&CdtTerm::Null), None);
    /// ```
    #[must_use]
    pub fn from_term(term: &CdtTerm) -> Option<Self> {
        match term {
            CdtTerm::Iri(iri) => Some(Self::Iri(iri.clone())),
            CdtTerm::Literal(literal) => Some(Self::Literal(literal.clone())),
            CdtTerm::Blank(_) | CdtTerm::Null | CdtTerm::TripleTerm(_) | CdtTerm::Composite(_) => {
                None
            }
        }
    }

    /// The ordering rank of this key's category, aligned with [`CdtTerm::rank`] so
    /// key order and term order agree.
    pub(crate) const fn rank(&self) -> u8 {
        match self {
            Self::Iri(_) => 2,
            Self::Literal(_) => 3,
        }
    }
}

/// One `cdt:Map` entry: `MapKey ':' MapValue`.
#[derive(Debug, Clone)]
pub struct CdtEntry {
    /// The entry's key.
    pub key: CdtKey,
    /// The entry's value.
    pub value: CdtTerm,
}
