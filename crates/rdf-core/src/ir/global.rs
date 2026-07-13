// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The **global** `u64`-scaled term-identity layer: [`GlobalTermId`] and the
//! [`GlobalDictionary`] value-interner (backend seam, purrdf P4/paged backends).
//!
//! This is a SEPARATE id space from the frozen [`RdfDataset`](super::RdfDataset)'s
//! [`TermId`]. A paged / cross-segment backend needs a term identity that can span
//! more than a single frozen dataset's `u32`-bounded table, so it mints
//! [`GlobalTermId`]s from a [`GlobalDictionary`] keyed on the dataset-INDEPENDENT
//! [`TermValue`]. Nothing here widens [`TermId`]: the load-bearing `NonZeroU32`
//! niche of `TermId` (why `Option<TermId>` — and every quad row's graph slot — costs
//! no extra word) is untouched, and a `const` assertion below fails the build if it
//! ever regresses.
//!
//! The dictionary mirrors the immutable IR's interning discipline exactly, one
//! width up: a store-once byte arena (each interned string owned ONCE), a dense
//! [`GlobalInternedTerm`] table, a value→dense-index [`HashTable`] using fixed-key
//! ahash, and a lazily-built reverse value index. Ids are minted in insertion order,
//! so a fixed push sequence is reproducible; no observable output depends on hash
//! iteration order (buckets are populated in ascending-id order and reads return the
//! first match).

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::num::NonZeroU64;
use std::sync::OnceLock;

use hashbrown::HashTable;

use crate::RdfTextDirection;
use crate::dataset_view::ViewTermId;
use crate::hash::FastHasher;

use super::dataset::TermRef;
use super::term::{BlankScope, StrRange, TermId, TermValue, arena_str};

/// Opaque **global** term identity — the id space a paged / cross-segment backend
/// mints, one width up from [`TermId`]. Like `TermId` it is opaque, is NOT
/// `Serialize`/`Deserialize`, and is meaningful only within the
/// [`GlobalDictionary`] that minted it (C0.8): a durable identifier must resolve the
/// term to its RDF value rather than retain a `GlobalTermId`.
///
/// # Layout
///
/// The inner value is a [`NonZeroU64`] holding `dense_index + 1`, so the all-zero
/// bit pattern is free for the [`Option`] niche: `Option<GlobalTermId>` is **8
/// bytes**, not 16. `#[repr(transparent)]` keeps the FFI layout a plain `u64`. Id
/// `0` is reserved as the niche sentinel and never minted. The `+1` offset is
/// confined entirely to [`index`](GlobalTermId::index) /
/// [`from_index`](GlobalTermId::from_index) — every other site is offset-agnostic,
/// so insertion (allocation) order, and therefore the `Ord` sort, is preserved
/// exactly. This mirrors [`TermId`]'s `NonZeroU32` discipline at `u64` width.
///
/// [`Hash`] is implemented by hand to hash the **0-based dense index as a `u64`**, so
/// the `+1` storage offset never leaks into the hash and cannot perturb any
/// hash-iteration order of a `HashMap<GlobalTermId, _>` / `HashSet<GlobalTermId>`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct GlobalTermId(NonZeroU64);

impl Hash for GlobalTermId {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        // `self.0.get() - 1` is the 0-based dense index; hashing it (not the stored
        // `+1`) keeps the niche a pure memory optimization with no ordering effect.
        (self.0.get() - 1).hash(state);
    }
}

impl GlobalTermId {
    /// The dense index this id addresses in the dictionary's term table.
    ///
    /// The stored value is `index + 1` (id 0 is the niche sentinel), so the dense
    /// index is one less. Never underflows (the inner is `>= 1`). Uses `try_from`
    /// rather than an `as` cast so it stays truncation-clean on a 32-bit `usize`
    /// target (wasm32): a `u64` index that could not fit `usize` cannot address a
    /// real `Vec`-backed term on that platform, so a hard-fail is correct.
    #[inline]
    #[must_use]
    pub fn index(self) -> usize {
        usize::try_from(self.0.get() - 1)
            .expect("global term id index exceeds usize on this platform")
    }

    /// Construct a `GlobalTermId` from a dense table index. Hard-fails (rather than
    /// wrapping) if `index` is `u64::MAX`, since `index + 1` would overflow the id
    /// space — the largest dense index is `u64::MAX - 1`.
    #[inline]
    #[must_use]
    pub fn from_index(index: u64) -> Self {
        let raw = index
            .checked_add(1)
            .expect("global dictionary cannot exceed u64::MAX-1 entries");
        // `raw = index + 1 >= 1`, so the `NonZeroU64` invariant always holds.
        Self(NonZeroU64::new(raw).expect("index + 1 is always >= 1"))
    }
}

/// A `GlobalTermId` is a valid [`DatasetView`](crate::DatasetView) id, so a paged /
/// global backend (Task 4) can use it as its `DatasetView::Id`.
impl ViewTermId for GlobalTermId {}

// The `NonZeroU64` niche is the global layer's `Option`-packing invariant, exactly
// as `TermId`'s `NonZeroU32` niche is for the frozen dataset. The THIRD assertion is
// the ring-fence: it proves the new `u64` layer did NOT perturb the load-bearing
// `u32` `TermId` niche (the reason `Option<TermId>` costs no extra word).
const _: () = assert!(size_of::<GlobalTermId>() == 8);
const _: () = assert!(size_of::<Option<GlobalTermId>>() == 8);
const _: () = assert!(size_of::<TermId>() == 4);

/// An interned literal in the global dictionary — the `u64`-scaled twin of
/// [`InternedLiteral`](super::term). The datatype is always an interned IRI, stored
/// as a [`GlobalTermId`] (never a `TermId`); string components are [`StrRange`]s into
/// the dictionary's byte arena.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct GlobalInternedLiteral {
    /// The lexical form, byte-for-byte as authored (C0.1).
    lexical_form: StrRange,
    /// The expanded datatype IRI's interned id (always present).
    datatype: GlobalTermId,
    /// The language tag, lowercased for the identity key (C0.1).
    language: Option<StrRange>,
    /// The RDF 1.2 base direction; distinct directions are distinct literals.
    direction: Option<RdfTextDirection>,
}

/// An interned term in the global dictionary — the storage form behind a
/// [`GlobalTermId`]. Mirrors [`InternedTerm`](super::term), but every id-carrying
/// component (a literal's datatype, a triple term's `s`/`p`/`o`) holds a
/// [`GlobalTermId`]. Strings are [`StrRange`]s into the dictionary's arena.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum GlobalInternedTerm {
    /// An IRI, by its arena range.
    Iri(StrRange),
    /// A blank node, identified by `(label, scope)` (C0.2).
    Blank { label: StrRange, scope: BlankScope },
    /// A literal, identified per C0.1.
    Literal(GlobalInternedLiteral),
    /// A triple term (RDF 1.2 quoted triple), identified structurally by its
    /// resolved `(s, p, o)` (C0.3).
    Triple {
        s: GlobalTermId,
        p: GlobalTermId,
        o: GlobalTermId,
    },
}

/// A borrowed term lookup key: carries string components by reference so the
/// dictionary can dedup BY VALUE *before* anything is pushed to the arena. Mirrors
/// [`GlobalInternedTerm`] but with `&str` where the stored form holds a [`StrRange`].
/// The id-carrying components (datatype, triple `s`/`p`/`o`) are already-interned
/// [`GlobalTermId`]s, so dedup on them is id-based, exactly like the frozen IR's
/// interner.
#[derive(Clone, Copy)]
enum GlobalTermLookup<'a> {
    Iri(&'a str),
    Blank {
        label: &'a str,
        scope: BlankScope,
    },
    Literal {
        lexical: &'a str,
        datatype: GlobalTermId,
        language: Option<&'a str>,
        direction: Option<RdfTextDirection>,
    },
    Triple {
        s: GlobalTermId,
        p: GlobalTermId,
        o: GlobalTermId,
    },
}

/// Fixed-key ahash of a lookup (id-based dedup path). MUST hash byte-identically to
/// [`hash_stored`] for equal values — explicit discriminant tags + `str::hash`.
fn hash_lookup<H: Hasher>(lookup: &GlobalTermLookup<'_>, state: &mut H) {
    match lookup {
        GlobalTermLookup::Iri(iri) => {
            0u8.hash(state);
            iri.hash(state);
        }
        GlobalTermLookup::Blank { label, scope } => {
            1u8.hash(state);
            label.hash(state);
            scope.hash(state);
        }
        GlobalTermLookup::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            2u8.hash(state);
            lexical.hash(state);
            datatype.hash(state);
            language.hash(state);
            direction.hash(state);
        }
        GlobalTermLookup::Triple { s, p, o } => {
            3u8.hash(state);
            s.hash(state);
            p.hash(state);
            o.hash(state);
        }
    }
}

/// Fixed-key ahash of a stored term, resolving its `StrRange`s through `arena`
/// (id-based dedup path). MUST match [`hash_lookup`] for equal values.
fn hash_stored<H: Hasher>(arena: &[u8], term: &GlobalInternedTerm, state: &mut H) {
    match term {
        GlobalInternedTerm::Iri(r) => {
            0u8.hash(state);
            arena_str(arena, *r).hash(state);
        }
        GlobalInternedTerm::Blank { label, scope } => {
            1u8.hash(state);
            arena_str(arena, *label).hash(state);
            scope.hash(state);
        }
        GlobalInternedTerm::Literal(lit) => {
            2u8.hash(state);
            arena_str(arena, lit.lexical_form).hash(state);
            lit.datatype.hash(state);
            lit.language.map(|r| arena_str(arena, r)).hash(state);
            lit.direction.hash(state);
        }
        GlobalInternedTerm::Triple { s, p, o } => {
            3u8.hash(state);
            s.hash(state);
            p.hash(state);
            o.hash(state);
        }
    }
}

fn hash_lookup_value(lookup: &GlobalTermLookup<'_>) -> u64 {
    let mut hasher = ahash::AHasher::default();
    hash_lookup(lookup, &mut hasher);
    hasher.finish()
}

fn hash_stored_value(arena: &[u8], term: &GlobalInternedTerm) -> u64 {
    let mut hasher = ahash::AHasher::default();
    hash_stored(arena, term, &mut hasher);
    hasher.finish()
}

/// Whether a stored term equals a lookup, resolving the stored ranges through
/// `arena` (id-based dedup path).
fn term_eq(arena: &[u8], term: &GlobalInternedTerm, lookup: &GlobalTermLookup<'_>) -> bool {
    match (term, lookup) {
        (GlobalInternedTerm::Iri(r), GlobalTermLookup::Iri(s)) => arena_str(arena, *r) == *s,
        (
            GlobalInternedTerm::Blank { label, scope },
            GlobalTermLookup::Blank {
                label: ls,
                scope: ss,
            },
        ) => arena_str(arena, *label) == *ls && scope == ss,
        (
            GlobalInternedTerm::Literal(lit),
            GlobalTermLookup::Literal {
                lexical,
                datatype,
                language,
                direction,
            },
        ) => {
            arena_str(arena, lit.lexical_form) == *lexical
                && lit.datatype == *datatype
                && lit.direction == *direction
                && lit.language.map(|r| arena_str(arena, r)) == *language
        }
        (
            GlobalInternedTerm::Triple { s, p, o },
            GlobalTermLookup::Triple {
                s: ls,
                p: lp,
                o: lo,
            },
        ) => s == ls && p == lp && o == lo,
        _ => false,
    }
}

/// The lazily-built reverse value index: a canonical **value hash** → dense-index
/// bucket map. The buckets store [`GlobalTermId`]s (NOT strings), so building it
/// duplicates no term strings; the rare hash collision is resolved by comparing BY
/// VALUE through the arena. `GlobalTermId`s are pushed in ascending (0..n) order, so
/// a single-answer read (`term_id_by_value`) is deterministic without an explicit
/// sort.
type GlobalValueIndex = HashMap<u64, Vec<GlobalTermId>, FastHasher>;

/// A `u64`-scaled value-interner keyed on the dataset-independent [`TermValue`] — the
/// global-identity twin of the frozen IR's `Interner`. Owns a store-once byte arena,
/// a dense [`GlobalInternedTerm`] table, a value→dense-index [`HashTable`] (fixed-key
/// ahash, hash/eq resolving into the arena BY VALUE), and a lazily-built reverse
/// value index. `Send + Sync` (the lazy index rides an [`OnceLock`], like
/// [`RdfDataset`](super::RdfDataset)).
#[derive(Debug)]
pub struct GlobalDictionary {
    /// The byte arena owning every interned string ONCE; terms hold ranges.
    arena: Vec<u8>,
    /// Dense table of interned terms (range-backed); the sole structural owner.
    terms: Vec<GlobalInternedTerm>,
    /// Store-once value→id index: `u64` dense indices into `terms`, with hash/eq
    /// resolving ranges through the arena and comparing BY VALUE.
    index: HashTable<u64>,
    /// Lazy value-hash → id reverse index for [`term_id_by_value`](Self::term_id_by_value).
    /// Built on first query and cached; `OnceLock` keeps the dictionary `Send + Sync`.
    /// Because the dictionary is MUTABLE (unlike the frozen `RdfDataset`), this cache
    /// is invalidated (`take`n) on every structural growth in
    /// [`intern_lookup`](Self::intern_lookup) and rebuilt on the next query.
    value_index: OnceLock<GlobalValueIndex>,
}

impl Default for GlobalDictionary {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalDictionary {
    /// A fresh, empty dictionary.
    #[must_use]
    pub fn new() -> Self {
        Self {
            arena: Vec::new(),
            terms: Vec::new(),
            index: HashTable::new(),
            value_index: OnceLock::new(),
        }
    }

    /// The number of distinct interned terms.
    #[must_use]
    pub fn len(&self) -> usize {
        self.terms.len()
    }

    /// Whether the dictionary has interned no terms yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// Append a string to the arena, returning its range. Validates the range fits
    /// `u32` BEFORE mutating the arena, so a checked overflow fails fast and leaves
    /// the dictionary consistent.
    fn push_str(&mut self, s: &str) -> StrRange {
        let offset = u32::try_from(self.arena.len()).expect("term arena exceeds u32::MAX bytes");
        let len = u32::try_from(s.len()).expect("term string exceeds u32::MAX bytes");
        offset
            .checked_add(len)
            .expect("term arena exceeds u32::MAX bytes");
        self.arena.extend_from_slice(s.as_bytes());
        StrRange { offset, len }
    }

    /// Intern a borrowed lookup BY VALUE: dedups against existing terms (resolving
    /// their ranges through the arena) and pushes strings to the arena only on a
    /// MISS. Idempotent: equal values map to one id, minted in insertion order.
    fn intern_lookup(&mut self, lookup: GlobalTermLookup<'_>) -> GlobalTermId {
        let hash = hash_lookup_value(&lookup);
        {
            let (arena, terms) = (&self.arena, &self.terms);
            if let Some(&i) = self
                .index
                .find(hash, |&i| term_eq(arena, &terms[i as usize], &lookup))
            {
                return GlobalTermId::from_index(i);
            }
        }
        let i = u64::try_from(self.terms.len())
            .expect("global dictionary table exceeds u64::MAX entries");
        // Miss: now (and only now) push the strings to the arena and build the term.
        let term = match lookup {
            GlobalTermLookup::Iri(iri) => GlobalInternedTerm::Iri(self.push_str(iri)),
            GlobalTermLookup::Blank { label, scope } => GlobalInternedTerm::Blank {
                label: self.push_str(label),
                scope,
            },
            GlobalTermLookup::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                let lexical_form = self.push_str(lexical);
                let language = language.map(|l| self.push_str(l));
                GlobalInternedTerm::Literal(GlobalInternedLiteral {
                    lexical_form,
                    datatype,
                    language,
                    direction,
                })
            }
            GlobalTermLookup::Triple { s, p, o } => GlobalInternedTerm::Triple { s, p, o },
        };
        self.terms.push(term);
        // A new term makes any previously-cached reverse value index stale. Unlike
        // the FROZEN `RdfDataset` (whose `OnceLock` value index is built once and
        // never invalidated), a `GlobalDictionary` keeps growing, so the lazy cache
        // must be dropped on every structural growth and rebuilt on the next query.
        // Only the MISS branch reaches here, so an idempotent re-intern (a hit,
        // above) leaves an already-built index intact.
        self.value_index.take();
        let (arena, terms) = (&self.arena, &self.terms);
        self.index
            .insert_unique(hash, i, |&i| hash_stored_value(arena, &terms[i as usize]));
        GlobalTermId::from_index(i)
    }

    /// Intern an IRI term. Idempotent: the same IRI string yields the same id.
    pub fn intern_iri(&mut self, iri: &str) -> GlobalTermId {
        self.intern_lookup(GlobalTermLookup::Iri(iri))
    }

    /// Intern a blank node. Identity is `(label, scope)` (C0.2).
    pub fn intern_blank(&mut self, label: &str, scope: BlankScope) -> GlobalTermId {
        self.intern_lookup(GlobalTermLookup::Blank { label, scope })
    }

    /// Intern a literal from its already-expanded components (C0.1): the datatype is
    /// an already-interned IRI [`GlobalTermId`], and the language, if present, is
    /// expected already-lowercased by the caller (as the frozen IR's builder does at
    /// intern time). The lexical form is preserved byte-for-byte.
    pub fn intern_literal(
        &mut self,
        lexical: &str,
        datatype: GlobalTermId,
        language: Option<&str>,
        direction: Option<RdfTextDirection>,
    ) -> GlobalTermId {
        self.intern_lookup(GlobalTermLookup::Literal {
            lexical,
            datatype,
            language,
            direction,
        })
    }

    /// Intern a triple term (RDF 1.2 quoted triple) from already-resolved component
    /// ids. Identified structurally by the resolved `(s, p, o)` (C0.3).
    pub fn intern_triple(
        &mut self,
        s: GlobalTermId,
        p: GlobalTermId,
        o: GlobalTermId,
    ) -> GlobalTermId {
        self.intern_lookup(GlobalTermLookup::Triple { s, p, o })
    }

    /// Intern a dataset-independent [`TermValue`], recursively for triple terms and
    /// literal datatypes. Idempotent, store-once, insertion order.
    ///
    /// The datatype of a [`TermValue::Literal`] is interned as its own IRI term
    /// first, so a literal's datatype is itself a [`GlobalTermId`] in this space; a
    /// [`TermValue::Triple`]'s components are interned before the enclosing triple.
    pub fn intern(&mut self, value: &TermValue) -> GlobalTermId {
        match value {
            TermValue::Iri(iri) => self.intern_iri(iri),
            TermValue::Blank { label, scope } => self.intern_blank(label, *scope),
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } => {
                let datatype_id = self.intern_iri(datatype);
                self.intern_literal(lexical_form, datatype_id, language.as_deref(), *direction)
            }
            TermValue::Triple { s, p, o } => {
                let s = self.intern(s);
                let p = self.intern(p);
                let o = self.intern(o);
                self.intern_triple(s, p, o)
            }
        }
    }

    /// Resolve a global id to a borrowed [`TermRef`] (arena-borrow; no allocation).
    /// Triple components and a literal's datatype are returned as [`GlobalTermId`]s;
    /// resolve them recursively if their values are needed.
    #[must_use]
    pub fn resolve(&self, id: GlobalTermId) -> TermRef<'_, GlobalTermId> {
        match &self.terms[id.index()] {
            GlobalInternedTerm::Iri(iri) => TermRef::Iri(arena_str(&self.arena, *iri)),
            GlobalInternedTerm::Blank { label, scope } => TermRef::Blank {
                label: arena_str(&self.arena, *label),
                scope: *scope,
            },
            GlobalInternedTerm::Literal(lit) => TermRef::Literal {
                lexical: arena_str(&self.arena, lit.lexical_form),
                datatype: lit.datatype,
                language: lit.language.map(|r| arena_str(&self.arena, r)),
                direction: lit.direction,
            },
            GlobalInternedTerm::Triple { s, p, o } => TermRef::Triple {
                s: *s,
                p: *p,
                o: *o,
            },
        }
    }

    /// Hash the IRI string of a term known to be an interned IRI (a literal
    /// datatype), for the value-based reverse index. Mirrors the frozen dataset's
    /// `hash_iri_string`.
    fn hash_datatype_iri<H: Hasher>(&self, id: GlobalTermId, state: &mut H) {
        match &self.terms[id.index()] {
            GlobalInternedTerm::Iri(iri) => arena_str(&self.arena, *iri).hash(state),
            // Unreachable for a well-formed literal (a datatype is always an IRI);
            // hash the Debug form rather than panic.
            other => format!("{other:?}").hash(state),
        }
    }

    /// Canonical **value** hash of a stored term, resolving the literal datatype id
    /// to its IRI string and recursing triple components to their values. MUST match
    /// [`hash_value`] (i.e. [`TermValue`]'s `Hash`) for equal values.
    fn hash_term_value<H: Hasher>(&self, id: GlobalTermId, state: &mut H) {
        match &self.terms[id.index()] {
            GlobalInternedTerm::Iri(iri) => {
                0u8.hash(state);
                arena_str(&self.arena, *iri).hash(state);
            }
            GlobalInternedTerm::Blank { label, scope } => {
                1u8.hash(state);
                arena_str(&self.arena, *label).hash(state);
                scope.hash(state);
            }
            GlobalInternedTerm::Literal(lit) => {
                2u8.hash(state);
                arena_str(&self.arena, lit.lexical_form).hash(state);
                self.hash_datatype_iri(lit.datatype, state);
                lit.language.map(|r| arena_str(&self.arena, r)).hash(state);
                lit.direction.hash(state);
            }
            GlobalInternedTerm::Triple { s, p, o } => {
                3u8.hash(state);
                self.hash_term_value(*s, state);
                self.hash_term_value(*p, state);
                self.hash_term_value(*o, state);
            }
        }
    }

    /// Whether a stored term equals a dataset-independent [`TermValue`], compared BY
    /// VALUE directly against the interned representation (zero allocations).
    fn term_matches_value(&self, id: GlobalTermId, value: &TermValue) -> bool {
        match (&self.terms[id.index()], value) {
            (GlobalInternedTerm::Iri(iri), TermValue::Iri(v)) => arena_str(&self.arena, *iri) == v,
            (
                GlobalInternedTerm::Blank { label, scope },
                TermValue::Blank {
                    label: vl,
                    scope: vs,
                },
            ) => arena_str(&self.arena, *label) == vl && scope == vs,
            (
                GlobalInternedTerm::Literal(lit),
                TermValue::Literal {
                    lexical_form,
                    datatype,
                    language,
                    direction,
                },
            ) => {
                arena_str(&self.arena, lit.lexical_form) == lexical_form
                    && lit.direction == *direction
                    && lit.language.map(|r| arena_str(&self.arena, r)) == language.as_deref()
                    && self.iri_matches(lit.datatype, datatype)
            }
            (
                GlobalInternedTerm::Triple { s, p, o },
                TermValue::Triple {
                    s: vs,
                    p: vp,
                    o: vo,
                },
            ) => {
                self.term_matches_value(*s, vs)
                    && self.term_matches_value(*p, vp)
                    && self.term_matches_value(*o, vo)
            }
            _ => false,
        }
    }

    /// Whether a term known to be an interned IRI equals `expected` (zero-alloc).
    fn iri_matches(&self, id: GlobalTermId, expected: &str) -> bool {
        matches!(&self.terms[id.index()], GlobalInternedTerm::Iri(iri) if arena_str(&self.arena, *iri) == expected)
    }

    /// The lazily-built reverse value index (built once, cached).
    fn value_index(&self) -> &GlobalValueIndex {
        self.value_index.get_or_init(|| {
            let mut map: GlobalValueIndex =
                HashMap::with_capacity_and_hasher(self.terms.len(), FastHasher::default());
            for i in 0..self.terms.len() {
                let id = GlobalTermId::from_index(
                    u64::try_from(i).expect("dense index fits u64 for a Vec-bounded table"),
                );
                let mut hasher = ahash::AHasher::default();
                self.hash_term_value(id, &mut hasher);
                map.entry(hasher.finish()).or_default().push(id);
            }
            map
        })
    }

    /// The id of an interned term given its **dataset-independent** value, or `None`
    /// if the dictionary contains no such term. Backed by the lazy reverse value
    /// index; keying on [`TermValue`] (not [`TermRef`]) is the correctness rule — a
    /// `TermRef`'s ids are local to whichever dictionary/dataset minted them.
    #[must_use]
    pub fn term_id_by_value(&self, value: &TermValue) -> Option<GlobalTermId> {
        let hash = hash_value(value);
        self.value_index()
            .get(&hash)?
            .iter()
            .copied()
            .find(|&id| self.term_matches_value(id, value))
    }
}

/// Fixed-key ahash of a dataset-independent [`TermValue`] (value-based path). Uses
/// [`TermValue`]'s hand-written `Hash`, so it matches
/// [`GlobalDictionary::hash_term_value`] for equal values.
fn hash_value(value: &TermValue) -> u64 {
    let mut hasher = ahash::AHasher::default();
    value.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_is_idempotent_and_insertion_ordered() {
        let mut dict = GlobalDictionary::new();
        let a = dict.intern(&TermValue::iri("http://example.org/x"));
        let a_again = dict.intern(&TermValue::iri("http://example.org/x"));
        let b = dict.intern(&TermValue::iri("http://example.org/y"));
        assert_eq!(a, a_again, "same value → same id");
        assert_ne!(a, b, "different values → different ids");
        // Ids are minted in insertion order: N then N+1.
        assert_eq!(b.index(), a.index() + 1);
        assert_eq!(dict.len(), 2);
        assert!(!dict.is_empty());
    }

    #[test]
    fn new_dictionary_is_empty() {
        let dict = GlobalDictionary::new();
        assert!(dict.is_empty());
        assert_eq!(dict.len(), 0);
    }

    #[test]
    fn resolve_round_trips_iri() {
        let mut dict = GlobalDictionary::new();
        let id = dict.intern(&TermValue::iri("http://example.org/thing"));
        assert_eq!(dict.resolve(id), TermRef::Iri("http://example.org/thing"));
    }

    #[test]
    fn resolve_round_trips_blank() {
        let mut dict = GlobalDictionary::new();
        let value = TermValue::Blank {
            label: "b0".to_string(),
            scope: BlankScope(3),
        };
        let id = dict.intern(&value);
        assert_eq!(
            dict.resolve(id),
            TermRef::Blank {
                label: "b0",
                scope: BlankScope(3),
            }
        );
    }

    #[test]
    fn resolve_round_trips_typed_literal_with_interned_datatype() {
        let mut dict = GlobalDictionary::new();
        let value = TermValue::typed_literal("42", "http://www.w3.org/2001/XMLSchema#integer");
        let id = dict.intern(&value);
        let TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } = dict.resolve(id)
        else {
            panic!("expected a literal");
        };
        assert_eq!(lexical, "42");
        assert_eq!(language, None);
        assert_eq!(direction, None);
        // The datatype is itself an interned GlobalTermId that resolves to its IRI.
        assert_eq!(
            dict.resolve(datatype),
            TermRef::Iri("http://www.w3.org/2001/XMLSchema#integer")
        );
    }

    #[test]
    fn resolve_round_trips_language_literal() {
        let mut dict = GlobalDictionary::new();
        // `lang_literal` lowercases the tag and expands to rdf:langString.
        let value = TermValue::lang_literal("Bonjour", "FR");
        let id = dict.intern(&value);
        let TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } = dict.resolve(id)
        else {
            panic!("expected a literal");
        };
        assert_eq!(lexical, "Bonjour");
        assert_eq!(language, Some("fr"));
        assert_eq!(direction, None);
        assert_eq!(
            dict.resolve(datatype),
            TermRef::Iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#langString")
        );
    }

    #[test]
    fn resolve_round_trips_triple_with_interned_components() {
        let mut dict = GlobalDictionary::new();
        let s = TermValue::iri("http://example.org/s");
        let p = TermValue::iri("http://example.org/p");
        let o = TermValue::simple_literal("obj");
        let value = TermValue::Triple {
            s: Box::new(s.clone()),
            p: Box::new(p.clone()),
            o: Box::new(o.clone()),
        };
        let id = dict.intern(&value);
        let TermRef::Triple {
            s: sid,
            p: pid,
            o: oid,
        } = dict.resolve(id)
        else {
            panic!("expected a triple term");
        };
        // Each component was interned and resolves back to its value.
        assert_eq!(dict.resolve(sid), TermRef::Iri("http://example.org/s"));
        assert_eq!(dict.resolve(pid), TermRef::Iri("http://example.org/p"));
        // And the components share identity with a direct intern of the same value.
        assert_eq!(dict.term_id_by_value(&s), Some(sid));
        assert_eq!(dict.term_id_by_value(&p), Some(pid));
        assert_eq!(dict.term_id_by_value(&o), Some(oid));
    }

    #[test]
    fn term_id_by_value_hits_and_misses() {
        let mut dict = GlobalDictionary::new();
        let present = TermValue::iri("http://example.org/present");
        let id = dict.intern(&present);
        assert_eq!(dict.term_id_by_value(&present), Some(id));
        // A value never interned yields None (absence is an empty match, not error).
        let absent = TermValue::iri("http://example.org/absent");
        assert_eq!(dict.term_id_by_value(&absent), None);
    }

    #[test]
    fn term_id_by_value_matches_intern_for_all_shapes() {
        let mut dict = GlobalDictionary::new();
        let values = [
            TermValue::iri("http://example.org/i"),
            TermValue::blank("b1"),
            TermValue::simple_literal("plain"),
            TermValue::typed_literal("3.14", "http://www.w3.org/2001/XMLSchema#double"),
            TermValue::lang_literal("hello", "en"),
        ];
        for v in &values {
            let id = dict.intern(v);
            assert_eq!(dict.term_id_by_value(v), Some(id), "value {v:?}");
        }
    }

    #[test]
    fn intern_is_deterministic_across_runs() {
        let sequence = [
            TermValue::iri("http://example.org/a"),
            TermValue::blank("b"),
            TermValue::typed_literal("1", "http://www.w3.org/2001/XMLSchema#integer"),
            TermValue::iri("http://example.org/a"), // repeat → same id
            TermValue::lang_literal("x", "en"),
            TermValue::Triple {
                s: Box::new(TermValue::iri("http://example.org/a")),
                p: Box::new(TermValue::iri("http://example.org/p")),
                o: Box::new(TermValue::simple_literal("o")),
            },
        ];

        let mut first = GlobalDictionary::new();
        let ids_first: Vec<usize> = sequence.iter().map(|v| first.intern(v).index()).collect();

        let mut second = GlobalDictionary::new();
        let ids_second: Vec<usize> = sequence.iter().map(|v| second.intern(v).index()).collect();

        assert_eq!(ids_first, ids_second, "id assignments must be reproducible");
        assert_eq!(first.len(), second.len());
    }

    #[test]
    fn ring_fence_term_id_stays_four_bytes() {
        // The new u64 global layer must NOT perturb the load-bearing u32 TermId
        // niche (mirrors the const asserts near GlobalTermId).
        assert_eq!(size_of::<TermId>(), 4);
        assert_eq!(size_of::<Option<TermId>>(), 4);
        assert_eq!(size_of::<GlobalTermId>(), 8);
        assert_eq!(size_of::<Option<GlobalTermId>>(), 8);
    }

    #[test]
    fn blank_scope_participates_in_identity() {
        let mut dict = GlobalDictionary::new();
        let s1 = dict.intern(&TermValue::Blank {
            label: "b".to_string(),
            scope: BlankScope(1),
        });
        let s2 = dict.intern(&TermValue::Blank {
            label: "b".to_string(),
            scope: BlankScope(2),
        });
        assert_ne!(s1, s2, "same label, different scope → different id (C0.2)");
    }

    #[test]
    fn global_term_id_index_round_trips() {
        for raw in [0u64, 1, 42, u64::MAX - 1] {
            let id = GlobalTermId::from_index(raw);
            assert_eq!(u64::try_from(id.index()).expect("index fits u64"), raw);
        }
    }

    #[test]
    #[should_panic(expected = "cannot exceed u64::MAX-1 entries")]
    fn global_term_id_from_index_rejects_u64_max() {
        let _ = GlobalTermId::from_index(u64::MAX);
    }
}
