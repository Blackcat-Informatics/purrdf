// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! [`CdtValue`] — a parsed `cdt:List` or `cdt:Map`.

use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering;

use crate::datatype::CdtDatatype;
use crate::error::CdtError;
use crate::term::{CdtEntry, CdtKey, CdtTerm, CdtTripleTerm};

/// The owned contents of a [`CdtValue`], as [`CdtValue::into_parts`] hands them back.
///
/// Freely constructible, and deliberately so: it is *not* a [`CdtValue`], so building
/// one asserts nothing. Turning it back into a value goes through
/// [`CdtValue::list`] / [`CdtValue::map`], which is where the bounds are enforced.
#[derive(Debug, Clone)]
pub enum CdtParts {
    /// `cdt:List` — an ordered sequence of elements.
    List(Vec<CdtTerm>),
    /// `cdt:Map` — entries in [`crate::total_key_cmp`] order, keys pairwise distinct.
    Map(Vec<CdtEntry>),
}

/// A borrowed view of a [`CdtValue`]'s contents, as [`CdtValue::contents`] returns it.
///
/// This is how a consumer pattern-matches on which composite datatype it has while the
/// value itself stays sealed. It is `Copy`, borrows for as long as the value does, and
/// costs nothing to produce.
#[derive(Debug, Clone, Copy)]
pub enum CdtContents<'a> {
    /// The elements of a `cdt:List`, in the list's own order.
    List(&'a [CdtTerm]),
    /// The entries of a `cdt:Map`, in [`crate::total_key_cmp`] key order.
    Map(&'a [CdtEntry]),
}

/// A composite value: a `cdt:List` or a `cdt:Map`.
///
/// # The three bounds are an invariant of this type
///
/// The contents are **private**, and every constructor — [`CdtValue::list`],
/// [`CdtValue::map`], [`crate::parse_cdt`] and each minting function in
/// [`crate::functions`] — checks [`crate::MAX_NESTING_DEPTH`],
/// [`crate::MAX_ELEMENTS`] and [`crate::MAX_LEXICAL_BYTES`] before it will hand one
/// back. A public tuple variant would have made those checks a property of whichever
/// code path happened to run rather than of the value, and any consumer could then
/// have built an arbitrarily deep composite whose `Drop` glue overflows the stack —
/// an `abort` in Rust, catchable by nobody. See [`crate::limits`] for the full
/// argument.
///
/// Reading the contents costs nothing: [`CdtValue::contents`] borrows,
/// [`CdtValue::as_list`] / [`CdtValue::as_map`] borrow one arm, and
/// [`CdtValue::into_parts`] moves them out.
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
pub struct CdtValue {
    /// Private: the only way to put contents here is through a bounded constructor.
    parts: CdtParts,
}

impl CdtValue {
    /// An empty list.
    #[must_use]
    pub const fn empty_list() -> Self {
        Self {
            parts: CdtParts::List(Vec::new()),
        }
    }

    /// An empty map.
    #[must_use]
    pub const fn empty_map() -> Self {
        Self {
            parts: CdtParts::Map(Vec::new()),
        }
    }

    /// Build a list from elements, in the given order.
    ///
    /// Refuses — before the value is handed back — anything that would break one of
    /// the crate's three bounds: nesting deeper than [`crate::MAX_NESTING_DEPTH`],
    /// more than [`crate::MAX_ELEMENTS`] elements at every level together, or a
    /// canonical form longer than [`crate::MAX_LEXICAL_BYTES`].
    ///
    /// # Errors
    ///
    /// [`CdtError::DepthExceeded`], [`CdtError::TooManyElements`] or
    /// [`CdtError::InputTooLarge`], whichever bound the prospective list crosses first.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_cdt::{CdtError, CdtTerm, CdtValue, MAX_NESTING_DEPTH};
    ///
    /// let list = CdtValue::list(vec![CdtTerm::Null])?;
    /// assert_eq!(list.canonical_lexical(), "[null]");
    ///
    /// // Nesting one level at a time is refused at the bound, not at the stack.
    /// let mut deep = CdtValue::empty_list();
    /// for _ in 1..MAX_NESTING_DEPTH {
    ///     deep = CdtValue::list(vec![CdtTerm::composite(deep)?])?;
    /// }
    /// assert_eq!(deep.depth(), MAX_NESTING_DEPTH);
    /// // A value at the bound cannot become an element: there is nowhere to put it.
    /// assert!(matches!(
    ///     CdtTerm::composite(deep),
    ///     Err(CdtError::DepthExceeded { .. })
    /// ));
    /// # Ok::<(), purrdf_cdt::CdtError>(())
    /// ```
    pub fn list(items: Vec<CdtTerm>) -> Result<Self, CdtError> {
        crate::limits::check_extent(&crate::limits::list_extent(items.iter()))?;
        Ok(Self {
            parts: CdtParts::List(items),
        })
    }

    /// Build a map from entries in any order, establishing the key-order invariant.
    ///
    /// # A duplicate key is a diagnostic, not a bare refusal
    ///
    /// A map with two entries under one key is not a value this type can hold, so the
    /// constructor refuses rather than silently keeping one of them — and it says
    /// **which** key collided, in the same [`CdtError::DuplicateMapKey`] the scanner
    /// raises, carrying the key's canonical lexical form
    /// ([`crate::canonical_key_lexical`]). A caller that is handed a bare "no" cannot
    /// report what went wrong, so it either invents a message or drops the
    /// information; neither is an admissible outcome.
    ///
    /// The `offset` a scanner error carries is a position in the lexical form that was
    /// scanned, and a programmatically built map has no such input. The offset here is
    /// therefore the position the offending key **would** occupy in the map's own
    /// canonical form — the one lexical form this value is guaranteed to have — so it
    /// still points at the key it names.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_cdt::{CdtEntry, CdtError, CdtKey, CdtLiteral, CdtTerm, CdtValue};
    ///
    /// let entry = |k: &str, v: &str| CdtEntry {
    ///     key: CdtKey::Literal(CdtLiteral::plain(k)),
    ///     value: CdtTerm::Literal(CdtLiteral::plain(v)),
    /// };
    /// // Authoring order does not reach the rendered bytes.
    /// let a = CdtValue::map(vec![entry("b", "2"), entry("a", "1")])?;
    /// let b = CdtValue::map(vec![entry("a", "1"), entry("b", "2")])?;
    /// assert_eq!(a.canonical_lexical(), b.canonical_lexical());
    ///
    /// // A duplicate key is refused, and the refusal names the key.
    /// let error = CdtValue::map(vec![entry("a", "1"), entry("a", "2")]).unwrap_err();
    /// let CdtError::DuplicateMapKey { key, .. } = error else { unreachable!() };
    /// assert_eq!(key, "\"a\"^^<http://www.w3.org/2001/XMLSchema#string>");
    /// # Ok::<(), purrdf_cdt::CdtError>(())
    /// ```
    pub fn map(mut entries: Vec<CdtEntry>) -> Result<Self, CdtError> {
        entries.sort_by(|x, y| crate::ops::total_key_cmp(&x.key, &y.key));
        // `{`, then each preceding entry as `key` `:` `value` `,`.
        let mut offset = 1usize;
        for window in entries.windows(2) {
            // `key` `:` `value` `,` — the two punctuation bytes are the `+ 2`.
            offset = offset
                .saturating_add(crate::render::key_lexical_len(&window[0].key))
                .saturating_add(crate::render::term_lexical_len(&window[0].value))
                .saturating_add(2);
            if crate::ops::total_key_cmp(&window[0].key, &window[1].key) == Ordering::Equal {
                return Err(CdtError::DuplicateMapKey {
                    offset,
                    key: crate::render::canonical_key_lexical(&window[0].key),
                });
            }
        }
        crate::limits::check_extent(&crate::limits::map_extent(
            entries.iter().map(|entry| (&entry.key, &entry.value)),
        ))?;
        Ok(Self {
            parts: CdtParts::Map(entries),
        })
    }

    /// Build a list whose bounds the caller has **just** checked.
    ///
    /// The two callers are the lexical scanner — which enforces the depth and element
    /// bounds as it scans, before the offending element is allocated, and the byte
    /// bound on both the input and the finished canonical form — and
    /// [`crate::functions`], which measures every prospective result from borrowed
    /// parts and refuses before cloning. Re-measuring here would make the scanner
    /// quadratic in nesting depth, since every enclosing frame would re-walk the
    /// element it just closed.
    pub(crate) const fn from_checked_items(items: Vec<CdtTerm>) -> Self {
        Self {
            parts: CdtParts::List(items),
        }
    }

    /// Build a map whose bounds the caller has **just** checked, and whose entries are
    /// already sorted into [`crate::total_key_cmp`] order with pairwise distinct keys.
    ///
    /// See [`CdtValue::from_checked_items`] for who is allowed to call this and why it
    /// exists.
    pub(crate) const fn from_checked_entries(entries: Vec<CdtEntry>) -> Self {
        Self {
            parts: CdtParts::Map(entries),
        }
    }

    /// A borrowed view of the contents, for matching on which datatype this is.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_cdt::{CdtContents, parse_list, parse_map};
    ///
    /// let describe = |value: &purrdf_cdt::CdtValue| match value.contents() {
    ///     CdtContents::List(items) => items.len(),
    ///     CdtContents::Map(entries) => entries.len(),
    /// };
    /// assert_eq!(describe(&parse_list("[1,2,3]")?), 3);
    /// assert_eq!(describe(&parse_map("{1:2}")?), 1);
    /// # Ok::<(), purrdf_cdt::CdtError>(())
    /// ```
    #[must_use]
    pub fn contents(&self) -> CdtContents<'_> {
        match &self.parts {
            CdtParts::List(items) => CdtContents::List(items),
            CdtParts::Map(entries) => CdtContents::Map(entries),
        }
    }

    /// The elements, when this value is a `cdt:List`, and `None` when it is a map.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_cdt::{parse_list, parse_map};
    ///
    /// assert_eq!(parse_list("[1,2]")?.as_list().map(<[_]>::len), Some(2));
    /// assert!(parse_map("{}")?.as_list().is_none());
    /// # Ok::<(), purrdf_cdt::CdtError>(())
    /// ```
    #[must_use]
    pub fn as_list(&self) -> Option<&[CdtTerm]> {
        match &self.parts {
            CdtParts::List(items) => Some(items),
            CdtParts::Map(_) => None,
        }
    }

    /// The entries, when this value is a `cdt:Map`, and `None` when it is a list.
    #[must_use]
    pub fn as_map(&self) -> Option<&[CdtEntry]> {
        match &self.parts {
            CdtParts::Map(entries) => Some(entries),
            CdtParts::List(_) => None,
        }
    }

    /// Move the elements out, when this value is a `cdt:List`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_cdt::{CdtTerm, parse_list};
    ///
    /// let items = parse_list("[null]")?.into_list().expect("a cdt:List");
    /// assert_eq!(items, vec![CdtTerm::Null]);
    /// # Ok::<(), purrdf_cdt::CdtError>(())
    /// ```
    #[must_use]
    pub fn into_list(self) -> Option<Vec<CdtTerm>> {
        match self.parts {
            CdtParts::List(items) => Some(items),
            CdtParts::Map(_) => None,
        }
    }

    /// Move the entries out, when this value is a `cdt:Map`.
    #[must_use]
    pub fn into_map(self) -> Option<Vec<CdtEntry>> {
        match self.parts {
            CdtParts::Map(entries) => Some(entries),
            CdtParts::List(_) => None,
        }
    }

    /// Move the contents out of the value.
    ///
    /// The result is no longer a [`CdtValue`] and asserts nothing; putting it back
    /// together goes through [`CdtValue::list`] / [`CdtValue::map`], which is where the
    /// bounds are re-established.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_cdt::{CdtParts, CdtValue, parse_list};
    ///
    /// let CdtParts::List(mut items) = parse_list("[1,2]")?.into_parts() else {
    ///     unreachable!("parse_list yields a list")
    /// };
    /// items.reverse();
    /// assert_eq!(CdtValue::list(items)?.canonical_lexical(), parse_list("[2,1]")?.canonical_lexical());
    /// # Ok::<(), purrdf_cdt::CdtError>(())
    /// ```
    #[must_use]
    pub fn into_parts(self) -> CdtParts {
        self.parts
    }

    /// The composite datatype of this value.
    #[must_use]
    pub const fn datatype(&self) -> CdtDatatype {
        match &self.parts {
            CdtParts::List(_) => CdtDatatype::List,
            CdtParts::Map(_) => CdtDatatype::Map,
        }
    }

    /// The number of elements at the top level (list items, or map entries).
    #[must_use]
    pub fn len(&self) -> usize {
        match &self.parts {
            CdtParts::List(items) => items.len(),
            CdtParts::Map(entries) => entries.len(),
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
        let (items, entries): (&[CdtTerm], &[CdtEntry]) = match self.contents() {
            CdtContents::List(items) => (items, &[]),
            CdtContents::Map(entries) => (&[], entries),
        };
        items.iter().chain(entries.iter().map(|e| &e.value))
    }

    /// The map's keys in key order, or nothing at all for a list.
    pub fn keys(&self) -> impl Iterator<Item = &CdtKey> {
        self.as_map().unwrap_or_default().iter().map(|e| &e.key)
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
