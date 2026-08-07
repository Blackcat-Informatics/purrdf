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

    /// Render a blank node's owned-model label, encoding the `(label, scope)` pair
    /// into the owned model's single string slot so two same-label blanks from
    /// DIFFERENT scopes never collapse into one owned blank for legacy consumers
    /// (compat bridge / oxigraph / SHACL).
    ///
    /// This is exactly
    /// [`encode_blank_label`](crate::blank_label::encode_blank_label) under the
    /// [`Unconstrained`](crate::blank_label::LabelAlphabet::Unconstrained)
    /// alphabet — the owned model is a `String`, not a document syntax, so no
    /// character is illegal there — and it inherits that function's properties
    /// verbatim:
    ///
    /// - a DEFAULT-scope label that does not begin with the reserved
    ///   [`ESCAPE_MARKER`](crate::blank_label::ESCAPE_MARKER) is returned
    ///   VERBATIM and borrowed, so real single-scope data is byte-unchanged
    ///   whatever dots, spaces or scalars it carries;
    /// - anything else — a non-default scope, or a label inside the reserved
    ///   marker namespace — becomes the single envelope `purrdfesc{n}_{body}`
    ///   that carries BOTH the scope and the label (C0.2).
    ///
    /// The encoding is INJECTIVE over `(label, scope)`: raw `"a.s1"` at the
    /// DEFAULT scope is the label `a.s1` itself, while `"a"` at scope 1 is
    /// `purrdfesc1_a`, so the two can never collide.
    #[inline]
    pub fn qualify_label(self, label: &str) -> std::borrow::Cow<'_, str> {
        crate::blank_label::encode_blank_label(
            label,
            self,
            crate::blank_label::LabelAlphabet::Unconstrained,
        )
    }

    /// Decode a qualified blank label back into its `(label, scope)` pair — the
    /// EXACT inverse of [`qualify_label`](Self::qualify_label).
    ///
    /// `unqualify_label(qualify_label(label, scope)) == (label, scope)` for every
    /// `(label, scope)` pair (the property is pinned by a sweep in this module's
    /// tests). That inverse is what makes an owned-model round trip
    /// **identity-preserving** rather than merely isomorphism-preserving: without
    /// it, re-interning an already-qualified label would encode it a second time
    /// and sever co-reference with the node the label came from.
    ///
    /// # The grammar this decodes, and nothing else
    ///
    /// This is
    /// [`decode_blank_label`](crate::blank_label::decode_blank_label) under the
    /// [`Unconstrained`](crate::blank_label::LabelAlphabet::Unconstrained)
    /// alphabet, so it decodes exactly what
    /// [`qualify_label`](Self::qualify_label) writes and nothing else:
    ///
    /// - a label that does not begin with the reserved
    ///   [`ESCAPE_MARKER`](crate::blank_label::ESCAPE_MARKER) is returned
    ///   VERBATIM at [`BlankScope::DEFAULT`], with no transformation — an
    ///   external document's organically-dotted label (`a.b`, `a..b`, `c1.s5`)
    ///   decodes to itself, and distinct labels stay distinct;
    /// - a marker-prefixed label is decoded as an envelope and accepted ONLY if
    ///   re-encoding the pair reproduces it byte for byte, so a label merely
    ///   SHAPED like an envelope (`purrdfesc_abc`, which the encoder would never
    ///   write for the label `abc`) also stands for itself.
    #[inline]
    #[must_use]
    pub fn unqualify_label(qualified: &str) -> (std::borrow::Cow<'_, str>, Self) {
        crate::blank_label::decode_blank_label(
            qualified,
            crate::blank_label::LabelAlphabet::Unconstrained,
        )
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
/// [`RdfDataset::term_id_by_value`](super::RdfDataset::term_id_by_value) (purrdf P4).
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
    Blank {
        /// The blank-node label (without the `_:` prefix).
        label: String,
        /// The blank-node scope the label is local to.
        scope: BlankScope,
    },
    /// A literal (C0.1): lexical form, the datatype **IRI by value**, optional
    /// (lowercased) language tag, and optional base direction.
    Literal {
        /// The lexical form, byte-for-byte as authored.
        lexical_form: String,
        /// The datatype IRI, by value (never a dataset-local id).
        datatype: String,
        /// The lowercased language tag, for language-tagged strings.
        language: Option<String>,
        /// The RDF 1.2 base direction, for directional language-tagged strings.
        direction: Option<RdfTextDirection>,
    },
    /// A triple term, identified structurally by its `(s, p, o)` **values** (C0.3).
    Triple {
        /// The quoted triple's subject value.
        s: Box<Self>,
        /// The quoted triple's predicate value.
        p: Box<Self>,
        /// The quoted triple's object value.
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

    /// The canonical kind tag used to order terms of DIFFERENT kinds. It mirrors the
    /// canonical Turtle renderer's `ObjKey` kind ordering (IRI < Literal < Blank <
    /// Triple, see `turtle_render`), so the total order below AGREES with the
    /// serializer's notion of canonical term order rather than inventing a second,
    /// conflicting one. (Note this is NOT the derive order, which would put Blank
    /// before Literal — hence the hand-written `Ord`.)
    #[inline]
    fn canonical_tag(&self) -> u8 {
        match self {
            Self::Iri(_) => 0,
            Self::Literal { .. } => 1,
            Self::Blank { .. } => 2,
            Self::Triple { .. } => 3,
        }
    }
}

// A TOTAL, dataset-independent order over `TermValue` — the canonical order in which
// `PagedDataset::compact` re-interns the live terms, so the renumbered
// `GlobalTermId` assignment is a pure function of the live term-VALUE set (never of
// ingest order, page order, or the old numbering). Cross-kind order follows
// [`canonical_tag`](TermValue::canonical_tag) (the serializer's IRI < Literal < Blank
// < Triple); within a kind the components compare in the same (datatype, language,
// lexical) precedence the renderer's `ObjKey` uses, with `direction` as a final
// tiebreak so two literals differing ONLY in base direction (distinct values) still
// order deterministically.
impl Ord for TermValue {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.canonical_tag()
            .cmp(&other.canonical_tag())
            .then_with(|| match (self, other) {
                (Self::Iri(a), Self::Iri(b)) => a.cmp(b),
                (
                    Self::Literal {
                        lexical_form: la,
                        datatype: da,
                        language: ga,
                        direction: dira,
                    },
                    Self::Literal {
                        lexical_form: lb,
                        datatype: db,
                        language: gb,
                        direction: dirb,
                    },
                ) => da
                    .cmp(db)
                    .then_with(|| ga.cmp(gb))
                    .then_with(|| la.cmp(lb))
                    .then_with(|| dira.cmp(dirb)),
                (
                    Self::Blank {
                        label: la,
                        scope: sa,
                    },
                    Self::Blank {
                        label: lb,
                        scope: sb,
                    },
                ) => la.cmp(lb).then_with(|| sa.cmp(sb)),
                (
                    Self::Triple {
                        s: sa,
                        p: pa,
                        o: oa,
                    },
                    Self::Triple {
                        s: sb,
                        p: pb,
                        o: ob,
                    },
                ) => sa.cmp(sb).then_with(|| pa.cmp(pb)).then_with(|| oa.cmp(ob)),
                // Equal tags guarantee the same variant, so every reachable pair is
                // matched above; a mixed-variant pair is unreachable here.
                _ => core::cmp::Ordering::Equal,
            })
    }
}

impl PartialOrd for TermValue {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
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
    fn qualify_label_default_scope_is_verbatim() {
        // Real single-scope data must stay byte-unchanged: borrowed, no rewrite,
        // whatever dots the label carries.
        for label in ["b0", "a.b", "a..b", "a...b", "c1.s5", "a b", ""] {
            let qualified = BlankScope::DEFAULT.qualify_label(label);
            assert_eq!(qualified, label);
            assert!(
                matches!(qualified, std::borrow::Cow::Borrowed(_)),
                "{label:?} must qualify without allocating"
            );
        }
    }

    #[test]
    fn qualify_label_envelopes_scoped_and_reserved_labels() {
        assert_eq!(BlankScope(1).qualify_label("a"), "purrdfesc1_a");
        assert_eq!(BlankScope::DEFAULT.qualify_label("a.s1"), "a.s1");
        assert_eq!(BlankScope(1).qualify_label("a.b"), "purrdfesc1_a_00002Eb");
        assert_eq!(BlankScope(3).qualify_label("a."), "purrdfesc3_a_00002E");
        // A label inside the reserved marker namespace is enveloped even at the
        // default scope — the one case where a raw label does not pass through.
        assert_eq!(
            BlankScope::DEFAULT.qualify_label("purrdfesc_abc"),
            "purrdfesc_purrdfesc_00005Fabc"
        );
    }

    #[test]
    fn qualify_label_is_injective_over_label_scope_pairs() {
        // The historical conflation pairs: raw "a" at scope 1 vs raw "a.s1" at
        // the default scope (which used to both render as "a.s1"), and the
        // dotted family the old dot-doubling folded together ("a.b" and "a..b"
        // both surfaced as "a..b"). Pairwise-distinct across a hostile sample.
        let pairs: &[(&str, BlankScope)] = &[
            ("a", BlankScope(1)),
            ("a.s1", BlankScope::DEFAULT),
            ("a.", BlankScope(1)),
            ("a.b", BlankScope::DEFAULT),
            ("a..b", BlankScope::DEFAULT),
            ("a...b", BlankScope::DEFAULT),
            ("a..s1", BlankScope::DEFAULT),
            ("a.s1.s2", BlankScope::DEFAULT),
            ("a.s1", BlankScope(2)),
            ("a", BlankScope(12)),
            ("a.s12", BlankScope::DEFAULT),
            ("purrdfesc_abc", BlankScope::DEFAULT),
            ("abc", BlankScope::DEFAULT),
            ("purrdfesc1_a", BlankScope::DEFAULT),
        ];
        let mut seen = std::collections::HashMap::new();
        for &(label, scope) in pairs {
            let qualified = scope.qualify_label(label).into_owned();
            if let Some(previous) = seen.insert(qualified.clone(), (label, scope)) {
                panic!(
                    "qualified label {qualified:?} conflates {previous:?} with {:?}",
                    (label, scope)
                );
            }
        }
    }

    /// The labels a hostile producer can put in the blank-node position: dots in
    /// every position, strings that MIMIC the scope suffix, non-ASCII, controls,
    /// and the empty label.
    const HOSTILE_LABELS: &[&str] = &[
        "",
        "b0",
        "a.b",
        "a.b.c",
        "a..b",
        "a....b",
        ".",
        "..",
        "...",
        "a.",
        ".a",
        "a.s1",
        "a..s1",
        "a.s1.s2",
        "s0.b0",
        "c1.s5",
        "x.s0",
        "x.s01",
        "x.s00",
        "x.s010",
        "x.s4294967296",
        "purrdfesc",
        "purrdfesc_a",
        "purrdfesc_",
        "purrdfesc1_a",
        "purrdfesc01_a",
        "a\u{d7}b",
        "bad\u{1f}label",
        "日本",
    ];

    /// The scopes to sweep every hostile label against, including the `u32`
    /// boundary the envelope's scope decimal must survive.
    const SWEEP_SCOPES: &[BlankScope] = &[
        BlankScope::DEFAULT,
        BlankScope(1),
        BlankScope(2),
        BlankScope(5),
        BlankScope(12),
        BlankScope(u32::MAX),
    ];

    /// The load-bearing property: qualification is EXACTLY invertible, so a term
    /// that leaves the IR as a qualified label and comes back re-denotes the same
    /// node rather than a second, doubly-qualified one.
    #[test]
    fn unqualify_label_inverts_qualify_label_for_every_label_and_scope() {
        for &label in HOSTILE_LABELS {
            for &scope in SWEEP_SCOPES {
                let qualified = scope.qualify_label(label);
                let (decoded, decoded_scope) = BlankScope::unqualify_label(&qualified);
                assert_eq!(
                    (decoded.as_ref(), decoded_scope),
                    (label, scope),
                    "round trip of {label:?} @ {scope:?} through {qualified:?}"
                );
            }
        }
    }

    /// The same property over an exhaustive sweep of short dot/`s`/digit/`_`
    /// strings — the characters the encoding actually reasons about — at several
    /// scopes.
    #[test]
    fn unqualify_label_inverts_qualify_label_over_an_exhaustive_short_alphabet() {
        const ALPHABET: &[char] = &['.', 's', '1', 'a', '_'];
        let mut labels: Vec<String> = vec![String::new()];
        let mut frontier: Vec<String> = vec![String::new()];
        for _ in 0..4 {
            let mut next = Vec::with_capacity(frontier.len() * ALPHABET.len());
            for prefix in &frontier {
                for &ch in ALPHABET {
                    let mut candidate = prefix.clone();
                    candidate.push(ch);
                    next.push(candidate);
                }
            }
            labels.extend(next.iter().cloned());
            frontier = next;
        }
        for label in &labels {
            for &scope in &[BlankScope::DEFAULT, BlankScope(1), BlankScope(11)] {
                let qualified = scope.qualify_label(label);
                let (decoded, decoded_scope) = BlankScope::unqualify_label(&qualified);
                assert_eq!(
                    (decoded.as_ref(), decoded_scope),
                    (label.as_str(), scope),
                    "round trip of {label:?} @ {scope:?} through {qualified:?}"
                );
            }
        }
    }

    /// Decoding leaves every label outside the reserved marker namespace exactly
    /// as authored: a label an external document wrote comes back verbatim at the
    /// default scope, and the whole dotted family stays pairwise distinct.
    #[test]
    fn unqualify_label_leaves_unencoded_labels_verbatim() {
        for label in [
            "b0", "s0.b0", "a.b", "a..b", "a...b", "x.s0", "x.s1", "x.s01", "-lead", "", "日本",
        ] {
            let (decoded, scope) = BlankScope::unqualify_label(label);
            assert_eq!(decoded.as_ref(), label, "{label:?}");
            assert_eq!(scope, BlankScope::DEFAULT, "{label:?}");
            assert!(
                matches!(decoded, std::borrow::Cow::Borrowed(_)),
                "{label:?} must decode without allocating"
            );
        }
    }

    /// A marker-prefixed label decodes ONLY when re-encoding the pair it names
    /// reproduces it byte for byte. Every near miss stands for itself.
    #[test]
    fn unqualify_label_rejects_envelopes_outside_the_encoder_image() {
        for label in [
            "purrdfesc",             // the bare marker: no `_`, no body
            "purrdfesc1",            // scope digits with no body separator
            "purrdfesc_abc",         // the label `abc` is written verbatim
            "purrdfesc01_a",         // zero-padded scope: outside the image
            "purrdfesc0_a",          // scope 0 never spells its ordinal
            "purrdfesc4294967296_a", // out of `u32` range
            "purrdfesc_a_00002",     // a hex group shorter than six digits
            "purrdfesc_a_00002e",    // lowercase hex is not what the encoder writes
            "purrdfesc_a.b",         // '.' is not a body pass-through character
            "purrdfesc__00D800",     // a surrogate code point is not a scalar
            "purrdfesc__110000",     // beyond the last Unicode scalar
            "purrdfesc_日本",        // a non-ASCII pass-through never survives encoding
        ] {
            let (decoded, scope) = BlankScope::unqualify_label(label);
            assert_eq!(scope, BlankScope::DEFAULT, "{label:?}");
            assert_eq!(decoded.as_ref(), label, "{label:?}");
        }
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
