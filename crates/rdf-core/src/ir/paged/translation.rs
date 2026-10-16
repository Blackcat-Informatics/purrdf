// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The per-page local↔global term-id map, [`PageTranslation`].
//!
//! A page is a frozen [`RdfDataset`] with its own dense, `u32`-scoped
//! [`TermId`] space; a [`PagedDataset`](super::PagedDataset)
//! addresses terms in the shared, `u64`-scoped
//! [`GlobalTermId`] space. `PageTranslation` bridges the two
//! for ONE page:
//!
//! - `local -> global` is an `O(1)` table lookup indexed by the page's dense
//!   [`TermId::index`](crate::ir::TermId::index).
//! - `global -> local` is an `O(log n)` binary search of a `GlobalTermId`-sorted
//!   side table (absent ⇒ the term does not occur on this page).
//!
//! The translation is built by **re-interning every page term BY VALUE** into the
//! shared [`GlobalDictionary`] (boundary G1): equal RDF
//! values across pages fold onto one `GlobalTermId`, so cross-page joins unify
//! automatically. It is NEVER a numeric offset remap — a page's local id space is
//! meaningless outside that page (C0.8).

use super::summary::{PageSummary, SummaryDefect, SummarySealError};
use crate::ir::{GlobalDictionary, GlobalTermId, RdfDataset, TermId};
use purrdf_lex::allocation::StorageError;
use std::sync::Arc;

pub(crate) enum TranslationError {
    Storage(StorageError),
    Summary(SummaryDefect),
}

impl purrdf_hash::error::StructuredError for TranslationError {
    purrdf_hash::error_visit! {
        Storage(cause) => "page-translation-storage" { "cause" => Cause(cause) },
        Summary(cause) => "page-translation-summary" { "cause" => Cause(cause) },
    }
}
purrdf_lex::variant_from!(TranslationError { Storage(StorageError), Summary(SummaryDefect) });

impl From<SummarySealError> for TranslationError {
    fn from(error: SummarySealError) -> Self {
        match error {
            SummarySealError::Contents(error) => Self::Summary(error),
            SummarySealError::Storage(error) => Self::Storage(error),
        }
    }
}

/// The local↔global term-id map for a single page of a
/// [`PagedDataset`](super::PagedDataset).
///
/// See the [module docs](self) for the two directions and the by-value re-intern
/// boundary.
///
/// `Clone` copies both id tables — [`PagedDataset::with_pages`](super::PagedDataset::with_pages)
/// carries a retained page's translation into the pruned dataset unchanged (its
/// local id space and the global ids it points at are untouched by dropping OTHER
/// pages).
#[derive(Debug, Clone)]
pub struct PageTranslation {
    /// Clone-stable metadata identity; remapping creates a fresh generation.
    pub(super) identity: Arc<()>,
    /// Shared exact merged counts retained by certified page parts. This has no
    /// back-reference to a page or translation, so it cannot retain RDF rows.
    pub(super) statistics_bundle: Option<Arc<super::PagedStatistics>>,
    /// `local_to_global[TermId::index()]` is the page-local term's shared
    /// [`GlobalTermId`]. Dense, indexed by the page's `0..term_count` term table.
    local_to_global: Box<[GlobalTermId]>,
    /// `(global, local)` pairs SORTED by `GlobalTermId`, binary-searched by
    /// [`to_local`](Self::to_local). A `GlobalTermId` absent here does not occur on
    /// this page.
    global_to_local: Box<[(GlobalTermId, TermId)]>,
    /// The page's exact per-term and per-graph row counts, in this page's own LOCAL
    /// `TermId` space (never renumbered by [`remap`](Self::remap) — see there).
    /// Built exactly once, by [`PageSummary::seal`], the sole producer.
    summary: Arc<PageSummary>,
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
    ///
    /// # Panics
    ///
    /// Panics if `page`'s own rows name a term or a named graph absent from its own
    /// frozen tables — structurally impossible for an `RdfDataset`, whose only
    /// producer is `RdfDatasetBuilder::freeze`. Callers holding PROVIDER-supplied
    /// content use `try_build` instead and report the typed refusal on their own
    /// surface.
    pub fn build(page: &RdfDataset, dict: &mut GlobalDictionary) -> Result<Self, StorageError> {
        match Self::try_build(page, dict) {
            Ok(translation) => Ok(translation),
            Err(TranslationError::Storage(error)) => Err(error),
            Err(TranslationError::Summary(defect)) => panic!("page_translation: {defect}"),
        }
    }

    /// [`build`](Self::build), returning the typed refusal instead of panicking.
    ///
    /// # Errors
    ///
    /// [`SummaryDefect`] if the page cannot be summarized honestly.
    pub(crate) fn try_build(
        page: &RdfDataset,
        dict: &mut GlobalDictionary,
    ) -> Result<Self, TranslationError> {
        let term_count = page.term_count();
        let mut local_to_global: Vec<GlobalTermId> = Vec::with_capacity(term_count);
        let mut global_to_local: Vec<(GlobalTermId, TermId)> = Vec::with_capacity(term_count);
        for i in 0..term_count {
            let local = TermId::from_index(u32::try_from(i).expect("page term index fits u32"))
                .expect("stored term index is representable");
            // The by-value re-intern boundary: resolve to a dataset-INDEPENDENT value,
            // then intern that value into the shared dictionary. Equal values across
            // pages collapse to one GlobalTermId.
            let value = page.term_value(local);
            // `page` is a FROZEN `RdfDataset`, and the only way to obtain one is
            // `RdfDatasetBuilder::freeze`, which refuses a relative IRI. Every value
            // here therefore already satisfies the IR-boundary absoluteness invariant,
            // so this walk re-interns without re-parsing — see
            // `GlobalDictionary::try_reintern_validated` for why that entry point exists
            // and why a new caller must not use it.
            let global = dict.try_reintern_validated(&value)?;
            local_to_global.push(global);
            global_to_local.push((global, local));
        }
        // Sort the reverse table by GlobalTermId for the binary search. A page's term
        // table has distinct terms, so the keys are unique.
        global_to_local.sort_unstable_by_key(|&(g, _)| g);
        // The page's exact per-term/per-graph row counts, in LOCAL id space — built
        // once here, the only place a summary is KEPT (the certification paths seal or
        // digest a page to compare against this one, then drop the result).
        let summary = PageSummary::seal(page)?;
        Ok(Self {
            identity: Arc::new(()),
            statistics_bundle: None,
            local_to_global: local_to_global.into_boxed_slice(),
            global_to_local: global_to_local.into_boxed_slice(),
            summary: Arc::new(summary),
        })
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

    /// The shared [`GlobalTermId`] of every page-local term, in local index order.
    /// Because a frozen page's term table is closed over triple components and
    /// literal datatypes, this slice IS the set of global ids the page keeps live —
    /// which [`PagedDataset::compact`](super::PagedDataset::compact) unions across the
    /// retained pages to mark-live before reclaiming dead ids.
    #[must_use]
    #[inline]
    pub fn global_ids(&self) -> &[GlobalTermId] {
        &self.local_to_global
    }

    /// Rebuild this translation with every global id passed through `remap`. The
    /// page's LOCAL id space and its quad table are unchanged — only the global side
    /// moves — so the reverse `(global, local)` table is re-sorted for the fresh
    /// global ids. Used by [`PagedDataset::compact`](super::PagedDataset::compact) to
    /// point a retained page at the compacted dictionary.
    #[must_use]
    pub(crate) fn remap(&self, remap: impl Fn(GlobalTermId) -> GlobalTermId) -> Self {
        let local_to_global: Vec<GlobalTermId> =
            self.local_to_global.iter().map(|&g| remap(g)).collect();
        let mut global_to_local: Vec<(GlobalTermId, TermId)> = local_to_global
            .iter()
            .enumerate()
            .map(|(i, &g)| {
                (
                    g,
                    TermId::from_index(u32::try_from(i).expect("page term index fits u32"))
                        .expect("stored term index is representable"),
                )
            })
            .collect();
        // Re-sort the reverse table by the NEW global ids for the binary search. The
        // remap is a bijection over live ids, so keys stay unique.
        global_to_local.sort_unstable_by_key(|&(g, _)| g);
        Self {
            identity: Arc::new(()),
            statistics_bundle: None,
            local_to_global: local_to_global.into_boxed_slice(),
            global_to_local: global_to_local.into_boxed_slice(),
            // The summary is keyed entirely in this page's LOCAL TermId space, which
            // compaction never touches — only the global side is renumbered above — so
            // it carries over verbatim rather than being resealed.
            summary: self.summary.clone(),
        }
    }

    /// This page's exact per-term and per-graph row counts (LOCAL `TermId` space).
    /// Read by [`admit_pattern`](super::admission::admit_pattern), the page-admission
    /// law both `quads_for_pattern` and `cardinality_estimate` apply in `mod.rs` and
    /// `query.rs`.
    #[must_use]
    #[inline]
    pub(crate) fn summary(&self) -> &PageSummary {
        &self.summary
    }
}
