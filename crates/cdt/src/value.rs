// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! [`CdtValue`] — a parsed `cdt:List` or `cdt:Map`.

use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering;

use crate::datatype::CdtDatatype;
use crate::term::{CdtEntry, CdtKey, CdtTerm, CdtTripleTerm};

/// A parsed composite value.
///
/// # The map representation, and why rendering is a pure function of the value
///
/// A map is a `Vec<CdtEntry>` — a **sequence**, not a hash map — and this crate
/// maintains it in the crate's syntactic total key order ([`crate::total_key_cmp`]).
/// That choice is what makes [`CdtValue::canonical_lexical`] a pure function of the
/// value:
///
/// * A hash map would render in iteration order, which depends on the hasher seed
///   and on insertion history, so two equal values could render to different bytes —
///   directly against the workspace's byte-determinism rule.
/// * An insertion-ordered sequence would render in *authoring* order, so
///   `{1:2,3:4}` and `{3:4,1:2}` — the same value, since a map is unordered — would
///   render differently. Rendering would be a function of the parse input, not of
///   the value.
/// * Sorting by a total order defined on the key alone leaves exactly one admissible
///   sequence per value, because [`crate::parse_map`] rejects duplicate keys and the
///   order is strict on distinct keys. Two equal maps therefore hold byte-identical
///   entry sequences and render byte-identically, on every host and in every process.
///
/// [`CdtValue::map`] establishes the invariant for programmatically built maps.
#[derive(Debug, Clone)]
pub enum CdtValue {
    /// `cdt:List` — an ordered sequence of elements.
    List(Vec<CdtTerm>),
    /// `cdt:Map` — entries held in [`crate::total_key_cmp`] order, keys pairwise
    /// distinct.
    Map(Vec<CdtEntry>),
}

impl CdtValue {
    /// An empty list.
    #[must_use]
    pub const fn empty_list() -> Self {
        Self::List(Vec::new())
    }

    /// An empty map.
    #[must_use]
    pub const fn empty_map() -> Self {
        Self::Map(Vec::new())
    }

    /// Build a map from entries in any order, establishing the key-order invariant.
    ///
    /// Returns `None` when two entries share a key — a map with duplicate keys is
    /// not a value this type can hold, so the constructor refuses rather than
    /// silently keeping one of them.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_cdt::{CdtEntry, CdtKey, CdtLiteral, CdtTerm, CdtValue};
    ///
    /// let entry = |k: &str, v: &str| CdtEntry {
    ///     key: CdtKey::Literal(CdtLiteral::plain(k)),
    ///     value: CdtTerm::Literal(CdtLiteral::plain(v)),
    /// };
    /// // Authoring order does not reach the rendered bytes.
    /// let a = CdtValue::map(vec![entry("b", "2"), entry("a", "1")]).unwrap();
    /// let b = CdtValue::map(vec![entry("a", "1"), entry("b", "2")]).unwrap();
    /// assert_eq!(a.canonical_lexical(), b.canonical_lexical());
    ///
    /// // A duplicate key is refused, not silently deduplicated.
    /// assert!(CdtValue::map(vec![entry("a", "1"), entry("a", "2")]).is_none());
    /// ```
    #[must_use]
    pub fn map(mut entries: Vec<CdtEntry>) -> Option<Self> {
        entries.sort_by(|x, y| crate::ops::total_key_cmp(&x.key, &y.key));
        if entries
            .windows(2)
            .any(|w| crate::ops::total_key_cmp(&w[0].key, &w[1].key) == Ordering::Equal)
        {
            return None;
        }
        Some(Self::Map(entries))
    }

    /// The composite datatype of this value.
    #[must_use]
    pub const fn datatype(&self) -> CdtDatatype {
        match self {
            Self::List(_) => CdtDatatype::List,
            Self::Map(_) => CdtDatatype::Map,
        }
    }

    /// The number of elements at the top level (list items, or map entries).
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::List(items) => items.len(),
            Self::Map(entries) => entries.len(),
        }
    }

    /// `true` when this composite has no elements at the top level.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The nesting depth of this value: 1 for a composite containing no composite,
    /// 2 for one containing a composite, and so on.
    ///
    /// Computed **iteratively** with an explicit heap worklist — it never recurses,
    /// so it is safe to call on a value of unknown provenance in order to check it
    /// against [`crate::MAX_NESTING_DEPTH`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_cdt::{CdtDatatype, parse_cdt};
    ///
    /// assert_eq!(parse_cdt("[1,2]", CdtDatatype::List)?.depth(), 1);
    /// assert_eq!(parse_cdt("[1,[2,[3]]]", CdtDatatype::List)?.depth(), 3);
    /// # Ok::<(), purrdf_cdt::CdtError>(())
    /// ```
    #[must_use]
    pub fn depth(&self) -> usize {
        let mut max = 0usize;
        let mut work: Vec<(&Self, usize)> = Vec::new();
        work.push((self, 1));
        while let Some((value, depth)) = work.pop() {
            if depth > max {
                max = depth;
            }
            for term in value.values() {
                let mut term_work: Vec<&CdtTerm> = alloc::vec![term];
                while let Some(t) = term_work.pop() {
                    match t {
                        CdtTerm::Composite(inner) => work.push((inner.as_ref(), depth + 1)),
                        CdtTerm::TripleTerm(triple) => {
                            let CdtTripleTerm {
                                subject,
                                predicate,
                                object,
                            } = triple.as_ref();
                            term_work.push(subject);
                            term_work.push(predicate);
                            term_work.push(object);
                        }
                        _ => {}
                    }
                }
            }
        }
        max
    }

    /// The total number of elements at every level (list items plus map entries),
    /// counted **iteratively**.
    #[must_use]
    pub fn element_count(&self) -> usize {
        let mut total = 0usize;
        let mut work: Vec<&Self> = alloc::vec![self];
        while let Some(value) = work.pop() {
            total += value.len();
            for term in value.values() {
                let mut term_work: Vec<&CdtTerm> = alloc::vec![term];
                while let Some(t) = term_work.pop() {
                    match t {
                        CdtTerm::Composite(inner) => work.push(inner.as_ref()),
                        CdtTerm::TripleTerm(triple) => {
                            let CdtTripleTerm {
                                subject,
                                predicate,
                                object,
                            } = triple.as_ref();
                            term_work.push(subject);
                            term_work.push(predicate);
                            term_work.push(object);
                        }
                        _ => {}
                    }
                }
            }
        }
        total
    }

    /// The list items, or the map entries' values, in this value's own order.
    pub(crate) fn values(&self) -> impl Iterator<Item = &CdtTerm> {
        let (items, entries): (&[CdtTerm], &[CdtEntry]) = match self {
            Self::List(items) => (items.as_slice(), &[]),
            Self::Map(entries) => (&[], entries.as_slice()),
        };
        items.iter().chain(entries.iter().map(|e| &e.value))
    }

    /// The map's keys in key order, or nothing at all for a list.
    pub fn keys(&self) -> impl Iterator<Item = &CdtKey> {
        let entries: &[CdtEntry] = match self {
            Self::List(_) => &[],
            Self::Map(entries) => entries.as_slice(),
        };
        entries.iter().map(|e| &e.key)
    }

    /// The canonical lexical form PurRDF writes for this value.
    ///
    /// See [`crate::render::canonical_lexical`] for the full specification of the
    /// form and the argument for choosing one.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_cdt::{CdtDatatype, parse_cdt};
    ///
    /// // Whitespace, shorthand spellings and authoring order all normalize away.
    /// let v = parse_cdt("[ 1 , true ]", CdtDatatype::List)?;
    /// assert_eq!(
    ///     v.canonical_lexical(),
    ///     "[\"1\"^^<http://www.w3.org/2001/XMLSchema#integer>,\
    ///       \"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>]"
    /// );
    /// # Ok::<(), purrdf_cdt::CdtError>(())
    /// ```
    #[must_use]
    pub fn canonical_lexical(&self) -> String {
        crate::render::canonical_lexical(self)
    }
}

// ── Term identity ────────────────────────────────────────────────────────────────
//
// Unlike `purrdf_xsd::XsdValue` — which discards the lexical form at parse time and
// therefore refuses `PartialEq` so it can never be mistaken for `sameTerm` — this
// crate keeps every literal lexical-verbatim. Structural equality here therefore IS
// RDF term identity, which is a meaningful and useful relation, so it is provided.
// The *value* relations are the separate, partial, error-propagating `list_equal` /
// `map_equal` free functions, and the syntactic order is `total_value_cmp`; neither
// is offered as `Ord`, so a `BTreeMap` can never silently pick one of them.
//
// Both impls delegate to the ITERATIVE comparator in `ops`, so equality never
// recurses even though the type is a tree.

impl PartialEq for CdtValue {
    fn eq(&self, other: &Self) -> bool {
        crate::ops::total_value_cmp(self, other) == Ordering::Equal
    }
}

impl Eq for CdtValue {}

impl PartialEq for CdtTerm {
    fn eq(&self, other: &Self) -> bool {
        crate::ops::total_term_cmp(self, other) == Ordering::Equal
    }
}

impl Eq for CdtTerm {}

impl PartialEq for CdtEntry {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.value == other.value
    }
}

impl Eq for CdtEntry {}

impl PartialEq for CdtTripleTerm {
    fn eq(&self, other: &Self) -> bool {
        self.subject == other.subject
            && self.predicate == other.predicate
            && self.object == other.object
    }
}

impl Eq for CdtTripleTerm {}
