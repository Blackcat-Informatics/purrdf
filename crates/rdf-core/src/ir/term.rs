// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Typed term identity and interned-term storage for the immutable IR (C1).
//!
//! These types realize the normative C0 identity contract (see
//! `docs/design/819-rdf-ir-dataflow.md`, *Appendix C0*):
//!
//! - [`TermId`] is opaque and **local to one frozen `RdfDataset`** — never
//!   serialized, never merge-stable, never meaningful across datasets (C0.8).
//! - Literal identity is defined by the IR, not a backend (C0.1): the datatype is
//!   always expanded (`xsd:string` / `rdf:langString`), the language tag is
//!   lowercased for the key, base direction participates in identity, and the
//!   lexical spelling is preserved verbatim.
//! - Blank-node scope participates in the interning key (C0.2).
//! - Triple terms are identified structurally by their resolved `(s, p, o)` (C0.3).

use std::num::NonZeroU32;

use crate::RdfTextDirection;

/// The `xsd:string` datatype IRI — the default datatype of a plain literal (C0.1).
pub(crate) const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// The `rdf:langString` datatype IRI — the default datatype of a language-tagged
/// literal (C0.1).
pub(crate) const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// Opaque term identity, LOCAL to one frozen `RdfDataset`. Deliberately NOT
/// `Serialize`/`Deserialize`, not merge-stable, not meaningful across datasets
/// (C0.8). Any consumer needing a durable identifier MUST resolve the term to its
/// RDF value rather than retaining a `TermId`.
///
/// # Layout (P3a)
///
/// The inner value is a [`NonZeroU32`] holding `dense_index + 1`, so the all-zero
/// bit pattern is free for the [`Option`] niche: `Option<TermId>` is **4 bytes**
/// (not 8), which shrinks [`QuadRow`](crate::ir::dataset) from 20 to 16 — ~20% off
/// the quad table — because the absent-graph slot (`g: Option<TermId>`) no longer
/// needs a discriminant word. `#[repr(transparent)]` keeps the FFI layout a plain
/// `u32`. Id `0` is reserved as the niche sentinel and is never minted. The `+1`
/// offset is confined entirely to [`index`](TermId::index) /
/// [`from_index`](TermId::from_index); every other site addresses terms through
/// those two methods and is offset-agnostic, so allocation order — and therefore
/// the `Ord` sort used at freeze — is preserved exactly.
///
/// [`Hash`] is implemented by hand to hash the **0-based dense index as a `u32`**,
/// byte-identical to the former `TermId(u32)` derive. The `+1` storage offset must
/// NOT leak into the hash: keeping it out preserves every `HashMap<TermId, _>` /
/// `HashSet<TermId>` iteration order, so the niche is a pure memory optimization
/// with no observable behavioral effect (a perf change must not silently reorder
/// any hash-iteration-dependent output).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct TermId(NonZeroU32);

impl std::hash::Hash for TermId {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // `self.0.get() - 1` is the dense index (a `u32`) — identical to what the
        // old `TermId(u32)` derive hashed. See the type doc above for why.
        (self.0.get() - 1).hash(state);
    }
}

impl TermId {
    /// The dense index this id addresses in the interner's term table.
    ///
    /// Low-level kernel API: the inner `NonZeroU32` stays private (so the `+1`
    /// niche offset never leaks and ids can't be byte-forged), but the dense
    /// index is exposed so the sibling `purrdf` adapters — the canonical
    /// Turtle serializer in particular — can address terms by position within a
    /// SINGLE dataset. It remains dataset-local and is never serialized or
    /// compared across datasets (C0.8).
    #[inline]
    pub fn index(self) -> usize {
        // The stored value is `index + 1` (id 0 is the niche sentinel), so the
        // dense index is one less. Never underflows: the inner is `>= 1`.
        (self.0.get() - 1) as usize
    }

    /// Construct a `TermId` from a dense table index.
    ///
    /// Low-level kernel API: the interner mints ids in allocation order; the
    /// sibling `purrdf` adapters (canonical Turtle serializer) also re-mint an
    /// id while scanning `0..term_count()` of a single dataset. The result is only
    /// meaningful against the dataset whose table has `index` (C0.8). Hard-fails
    /// (rather than wrapping) if `index` is `u32::MAX`, since `index + 1` would
    /// overflow the id space — the largest dense index is `u32::MAX - 1`, so the
    /// table can hold up to `u32::MAX` terms.
    #[inline]
    pub fn from_index(index: u32) -> Self {
        let raw = index
            .checked_add(1)
            .expect("term table cannot exceed u32::MAX entries");
        // `raw = index + 1 >= 1`, so the `NonZeroU32` invariant always holds.
        Self(NonZeroU32::new(raw).expect("index + 1 is always >= 1"))
    }
}

// The NonZeroU32 niche is the load-bearing P3a invariant: it is *why*
// `Option<TermId>` — and the `g` graph slot of every quad row — costs no extra
// word. These compile-time assertions fail the build if the niche ever regresses.
const _: () = assert!(size_of::<TermId>() == 4);
const _: () = assert!(size_of::<Option<TermId>>() == 4);

/// Blank-node scope. Participates in the interning key (C0.2): two blank nodes
/// from different scopes are distinct even with the same label; two blank nodes in
/// the same scope with the same label are the same node. `0` = default/global
/// scope; `> 0` = a per-segment scope assigned by the streaming importer.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct BlankScope(pub u32);

impl BlankScope {
    /// The default/global blank-node scope.
    pub const DEFAULT: Self = Self(0);

    /// The raw scope ordinal.
    #[inline]
    pub fn ordinal(self) -> u32 {
        self.0
    }

    /// Render a blank node's owned-model label, qualifying it deterministically by
    /// scope so two same-label blanks from DIFFERENT scopes never collapse into one
    /// owned blank for legacy consumers (compat bridge / oxigraph / SHACL).
    ///
    /// The DEFAULT scope keeps the bare label verbatim, so real single-scope data is
    /// byte-unchanged; a non-default scope `n` qualifies as `"{label}.s{n}"` (C0.2).
    #[inline]
    pub fn qualify_label(self, label: &str) -> std::borrow::Cow<'_, str> {
        if self == Self::DEFAULT {
            std::borrow::Cow::Borrowed(label)
        } else {
            std::borrow::Cow::Owned(format!("{label}.s{}", self.0))
        }
    }
}

impl Default for BlankScope {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// An interned literal. The identity key per C0.1: datatype is ALWAYS expanded to
/// an interned IRI [`TermId`]; the language tag is lowercased; base direction is in
/// the key; and the lexical spelling is preserved verbatim.
/// A `(offset, len)` range into the interner's byte arena (P3b). Each interned
/// string is stored once in the arena rather than as its own `Box<str>`, so a term
/// holds only this 8-byte range — `InternedTerm` becomes `Copy` and per-term heap
/// allocations collapse to one growable arena.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct StrRange {
    pub offset: u32,
    pub len: u32,
}

/// Borrow an arena range as `&str`. The arena only ever receives validated UTF-8
/// (it is appended from `&str` values) and ranges are recorded at push time, so the
/// sub-slice is always valid UTF-8.
#[inline]
pub(crate) fn arena_str(arena: &[u8], range: StrRange) -> &str {
    let bytes = &arena[range.offset as usize..range.offset as usize + range.len as usize];
    debug_assert!(
        std::str::from_utf8(bytes).is_ok(),
        "arena range is valid UTF-8"
    );
    // SAFETY: see the doc comment — the arena is append-only of validated UTF-8 and
    // every `StrRange` was recorded over a pushed `&str`, so `bytes` is valid UTF-8.
    unsafe { std::str::from_utf8_unchecked(bytes) }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct InternedLiteral {
    /// The lexical form, byte-for-byte as authored — never canonicalized (C0.1).
    pub lexical_form: StrRange,
    /// The expanded datatype, always present (`xsd:string` / `rdf:langString`
    /// expanded at intern time), stored as the id of its interned IRI term.
    pub datatype: TermId,
    /// The language tag, lowercased for the identity key (C0.1).
    pub language: Option<StrRange>,
    /// The RDF 1.2 base direction; distinct directions are distinct literals.
    pub direction: Option<RdfTextDirection>,
}

/// An interned term — the storage form behind a [`TermId`]. Crate-private: the IR
/// exposes terms through resolved views, never this internal representation. Strings
/// are `StrRange`s into the interner's byte arena.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum InternedTerm {
    /// An IRI, by its arena range.
    Iri(StrRange),
    /// A blank node, identified by `(label, scope)` (C0.2).
    Blank { label: StrRange, scope: BlankScope },
    /// A literal, identified per C0.1.
    Literal(InternedLiteral),
    /// A triple term (RDF 1.2 quoted triple), identified structurally by its
    /// resolved `(s, p, o)` (C0.3).
    Triple { s: TermId, p: TermId, o: TermId },
}

/// A **dataset-independent** term value — the lookup key for
/// [`RdfDataset::term_id_by_value`](super::RdfDataset::term_id_by_value) (purrdf P4,
/// ).
///
/// Unlike [`crate::ir::TermRef`] (whose literal-datatype and triple-component slots carry
/// dataset-local [`TermId`]s), `TermValue` expresses every component **by value** —
/// the literal datatype is its IRI string, triple terms recurse by value. This is
/// the issue's core correctness rule: keying value→id lookup on `TermRef` would
/// smuggle ids local to *another* dataset and silently return wrong answers, so the
/// key carries no `TermId` at all. A `&TermValue` is the spec's "TermValueRef".
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TermValue {
    /// An IRI, by its full string.
    Iri(String),
    /// A blank node, by `(label, scope)` (C0.2). `scope` is a structural ordinal,
    /// not a term-table id, so it is dataset-independent.
    Blank { label: String, scope: BlankScope },
    /// A literal (C0.1): lexical form, the datatype **IRI by value**, optional
    /// (lowercased) language tag, and optional base direction.
    Literal {
        lexical_form: String,
        datatype: String,
        language: Option<String>,
        direction: Option<RdfTextDirection>,
    },
    /// A triple term, identified structurally by its `(s, p, o)` **values** (C0.3).
    Triple {
        s: Box<Self>,
        p: Box<Self>,
        o: Box<Self>,
    },
}

impl TermValue {
    /// An IRI term from its full string.
    #[inline]
    pub fn iri(value: impl Into<String>) -> Self {
        Self::Iri(value.into())
    }

    /// A blank node in the default scope, from its bare label.
    #[inline]
    pub fn blank(label: impl Into<String>) -> Self {
        Self::Blank {
            label: label.into(),
            scope: BlankScope::DEFAULT,
        }
    }

    /// A plain `xsd:string` literal (datatype expanded per C0.1).
    #[inline]
    pub fn simple_literal(lexical_form: impl Into<String>) -> Self {
        Self::Literal {
            lexical_form: lexical_form.into(),
            datatype: XSD_STRING.to_owned(),
            language: None,
            direction: None,
        }
    }

    /// A typed literal with an explicit datatype IRI.
    #[inline]
    pub fn typed_literal(lexical_form: impl Into<String>, datatype: impl Into<String>) -> Self {
        Self::Literal {
            lexical_form: lexical_form.into(),
            datatype: datatype.into(),
            language: None,
            direction: None,
        }
    }

    /// A language-tagged literal (datatype expanded to `rdf:langString`, language
    /// **lowercased** for the identity key per C0.1).
    #[inline]
    pub fn lang_literal(lexical_form: impl Into<String>, language: impl AsRef<str>) -> Self {
        Self::Literal {
            lexical_form: lexical_form.into(),
            datatype: RDF_LANG_STRING.to_owned(),
            language: Some(language.as_ref().to_lowercase()),
            direction: None,
        }
    }

    /// The IRI string, if this term is an [`TermValue::Iri`].
    #[inline]
    pub fn as_iri(&self) -> Option<&str> {
        match self {
            Self::Iri(iri) => Some(iri.as_str()),
            _ => None,
        }
    }

    /// The blank-node `(label, scope)`, if this term is a [`TermValue::Blank`].
    #[inline]
    pub fn as_blank(&self) -> Option<(&str, BlankScope)> {
        match self {
            Self::Blank { label, scope } => Some((label.as_str(), *scope)),
            _ => None,
        }
    }

    /// `true` iff this term is an IRI.
    #[inline]
    pub fn is_iri(&self) -> bool {
        matches!(self, Self::Iri(_))
    }

    /// `true` iff this term is a literal.
    #[inline]
    pub fn is_literal(&self) -> bool {
        matches!(self, Self::Literal { .. })
    }

    /// `true` iff this term is a blank node.
    #[inline]
    pub fn is_blank(&self) -> bool {
        matches!(self, Self::Blank { .. })
    }
}

// `Hash` is hand-written (not derived) with **explicit** discriminant tags so it is
// robust against compiler-dependent enum-discriminant hashing AND matches the
// allocation-free `RdfDataset::hash_term` (which hashes the interned representation
// directly) byte-for-byte. The two MUST stay in sync — the
// `term_id_by_value` round-trip tests fail if they diverge. `String`/`Box<str>`/
// `&str` all hash via `str`, so the by-value datatype here matches the resolved IRI
// string there.
impl core::hash::Hash for TermValue {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Iri(iri) => {
                0u8.hash(state);
                iri.hash(state);
            }
            Self::Blank { label, scope } => {
                1u8.hash(state);
                label.hash(state);
                scope.hash(state);
            }
            Self::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } => {
                2u8.hash(state);
                lexical_form.hash(state);
                datatype.hash(state);
                language.hash(state);
                direction.hash(state);
            }
            Self::Triple { s, p, o } => {
                3u8.hash(state);
                s.hash(state);
                p.hash(state);
                o.hash(state);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn term_id_index_round_trips() {
        // `u32::MAX` is no longer a valid index (the stored value is `index + 1`,
        // so the last addressable index is `u32::MAX - 1`).
        for raw in [0u32, 1, 42, u32::MAX - 1] {
            let id = TermId::from_index(raw);
            assert_eq!(id.index(), raw as usize);
        }
    }

    #[test]
    fn term_id_option_uses_the_nonzero_niche() {
        // The whole point of P3a: `Option<TermId>` rides the NonZeroU32 niche.
        assert_eq!(size_of::<Option<TermId>>(), 4);
        assert_eq!(size_of::<TermId>(), 4);
    }

    #[test]
    #[should_panic(expected = "cannot exceed u32::MAX entries")]
    fn term_id_from_index_rejects_u32_max() {
        // `index + 1` would overflow the id space; the mint hard-fails.
        let _ = TermId::from_index(u32::MAX);
    }

    #[test]
    fn blank_scope_default_is_zero() {
        assert_eq!(BlankScope::default(), BlankScope(0));
        assert_eq!(BlankScope::DEFAULT, BlankScope(0));
    }

    #[test]
    fn datatype_constants_are_the_expected_iris() {
        assert_eq!(XSD_STRING, "http://www.w3.org/2001/XMLSchema#string");
        assert_eq!(
            RDF_LANG_STRING,
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString"
        );
    }

    #[test]
    fn interned_literal_equality_includes_direction() {
        let a = InternedLiteral {
            // The arena range is irrelevant here — this pins that base direction
            // participates in literal identity (: lexical form is now a range).
            lexical_form: StrRange { offset: 0, len: 1 },
            datatype: TermId::from_index(0),
            language: None,
            direction: Some(RdfTextDirection::Ltr),
        };
        let mut b = a;
        assert_eq!(a, b);
        b.direction = Some(RdfTextDirection::Rtl);
        assert_ne!(a, b);
    }
}
