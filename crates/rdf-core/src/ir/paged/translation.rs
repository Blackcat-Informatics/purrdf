// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The per-page local↔global term-id map, [`PageTranslation`].
//!
//! A page is a frozen [`RdfDataset`] with its own dense, `u32`-scoped
//! [`TermId`](crate::ir::TermId) space; a [`PagedDataset`](super::PagedDataset)
//! addresses terms in the shared, `u64`-scoped
//! [`GlobalTermId`](crate::ir::GlobalTermId) space. `PageTranslation` bridges the two
//! for ONE page:
//!
//! - `local -> global` is an `O(1)` table lookup indexed by the page's dense
//!   [`TermId::index`](crate::ir::TermId::index).
//! - `global -> local` is an `O(log n)` binary search of a `GlobalTermId`-sorted
//!   side table (absent ⇒ the term does not occur on this page).
//!
//! The translation is built by **re-interning every page term BY VALUE** into the
//! shared [`GlobalDictionary`](crate::ir::GlobalDictionary) (boundary G1): equal RDF
//! values across pages fold onto one `GlobalTermId`, so cross-page joins unify
//! automatically. It is NEVER a numeric offset remap — a page's local id space is
//! meaningless outside that page (C0.8).

use crate::ir::{GlobalDictionary, GlobalTermId, RdfDataset, TermId};

/// The local↔global term-id map for a single page of a
/// [`PagedDataset`](super::PagedDataset).
///
/// See the [module docs](self) for the two directions and the by-value re-intern
/// boundary.
#[derive(Debug)]
pub struct PageTranslation {
    /// `local_to_global[TermId::index()]` is the page-local term's shared
    /// [`GlobalTermId`]. Dense, indexed by the page's `0..term_count` term table.
    local_to_global: Box<[GlobalTermId]>,
    /// `(global, local)` pairs SORTED by `GlobalTermId`, binary-searched by
    /// [`to_local`](Self::to_local). A `GlobalTermId` absent here does not occur on
    /// this page.
    global_to_local: Box<[(GlobalTermId, TermId)]>,
}

impl PageTranslation {
    /// Build the translation for `page`, folding EVERY page term into the shared
    /// `dict` by value (boundary G1).
    ///
    /// Walks the page's dense term table (`0..term_count`), resolves each local
    /// [`TermId`] to its dataset-independent
    /// [`TermValue`](crate::ir::TermValue), interns that value into `dict` to obtain
    /// the shared [`GlobalTermId`], and records both directions. After this returns,
    /// `dict` contains a global id for every term on the page — the invariant that
    /// makes the paged view's value lookups correct for terms on not-yet-requeried
    /// pages.
    #[must_use]
    pub fn build(page: &RdfDataset, dict: &mut GlobalDictionary) -> Self {
        let term_count = page.term_count();
        let mut local_to_global: Vec<GlobalTermId> = Vec::with_capacity(term_count);
        let mut global_to_local: Vec<(GlobalTermId, TermId)> = Vec::with_capacity(term_count);
        for i in 0..term_count {
            let local = TermId::from_index(u32::try_from(i).expect("page term index fits u32"));
            // The by-value re-intern boundary: resolve to a dataset-INDEPENDENT value,
            // then intern that value into the shared dictionary. Equal values across
            // pages collapse to one GlobalTermId.
            let value = page.term_value(local);
            let global = dict.intern(&value);
            local_to_global.push(global);
            global_to_local.push((global, local));
        }
        // Sort the reverse table by GlobalTermId for the binary search. A page's term
        // table has distinct terms, so the keys are unique.
        global_to_local.sort_unstable_by_key(|&(g, _)| g);
        Self {
            local_to_global: local_to_global.into_boxed_slice(),
            global_to_local: global_to_local.into_boxed_slice(),
        }
    }

    /// The shared [`GlobalTermId`] for a page-local [`TermId`] (`O(1)`).
    #[must_use]
    #[inline]
    pub fn to_global(&self, local: TermId) -> GlobalTermId {
        self.local_to_global[local.index()]
    }

    /// The page-local [`TermId`] for a shared [`GlobalTermId`], or `None` if the term
    /// does not occur on this page (`O(log n)` binary search).
    #[must_use]
    #[inline]
    pub fn to_local(&self, global: GlobalTermId) -> Option<TermId> {
        self.global_to_local
            .binary_search_by_key(&global, |&(g, _)| g)
            .ok()
            .map(|pos| self.global_to_local[pos].1)
    }

    /// The number of terms on this page (the length of the local id space).
    #[must_use]
    #[inline]
    pub fn term_count(&self) -> usize {
        self.local_to_global.len()
    }
}
