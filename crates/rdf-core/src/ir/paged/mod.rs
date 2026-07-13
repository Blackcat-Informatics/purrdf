// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A reference, in-memory, demand-paged dataset — [`PagedDataset`] — that composes
//! many frozen [`RdfDataset`] pages into ONE logical dataset queried through the
//! id-agnostic [`DatasetView`] trait, with `type Id = GlobalTermId`.
//!
//! # What a page is, and how ids compose
//!
//! Each page is a frozen [`RdfDataset`] with its own dense, dataset-local
//! [`TermId`](crate::ir::TermId) space. The paged view addresses terms in the shared
//! [`GlobalTermId`](crate::ir::GlobalTermId) space of a single
//! [`GlobalDictionary`](crate::ir::GlobalDictionary): every page term is re-interned
//! BY VALUE into that dictionary (boundary G1, in
//! [`PageTranslation::build`](translation::PageTranslation::build)), so equal RDF
//! values across pages collapse onto one `GlobalTermId` and cross-page joins unify
//! automatically. A per-page [`PageTranslation`] maps local↔global.
//!
//! # Seal-then-lazy contract
//!
//! [`PagedDataset::from_provider`] runs an EAGER seal pass: it materializes every
//! page exactly once, folds its terms into the shared dictionary, ORs its
//! capabilities, and tracks the total quad count — the dictionary MUST be complete
//! before any query, so [`term_id_by_value`](DatasetView::term_id_by_value) is
//! correct for terms on pages that have not been re-queried. The seal pass then DROPS
//! the materialized page (it is not kept resident); query-time access re-materializes
//! it through the [`PageProvider`] and caches the result in the slot's
//! [`OnceLock`]. For an [`InMemoryPageProvider`](provider::InMemoryPageProvider) the
//! re-materialize is a cheap `Arc::clone`; for a
//! [`CountingDemandProvider`](provider::CountingDemandProvider) it is a counted
//! rebuild — which is exactly what makes the lazy hook observable at query time.
//!
//! # Determinism
//!
//! Pages iterate in ascending [`PageId`] order and each page yields in its frozen
//! in-page order, so every egress (`quads`, `quads_for_pattern`, the reifier/
//! annotation views) is deterministic. Pages are quad-disjoint (G3, enforced at
//! freeze in a later task), so no cross-page dedup is needed.

pub mod provider;
pub mod translation;

use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock};

use crate::RdfStoreCapabilities;
use crate::dataset_view::{DatasetView, GraphMatch};
use crate::ir::{GlobalDictionary, GlobalTermId, QuadIds, QuadRef, RdfDataset, TermId, TermValue};

pub use provider::{CountingDemandProvider, InMemoryPageProvider, PageFault, PageId, PageProvider};
pub use translation::PageTranslation;

/// One page of a [`PagedDataset`]: its [`PageId`], the local↔global
/// [`PageTranslation`] built at seal time, and a lazily-cached resident
/// [`RdfDataset`] (empty after the seal pass; filled on first query-time access).
#[derive(Debug)]
struct PageSlot {
    /// The page's dense ordinal (equals its index in `PagedDataset::pages`).
    id: PageId,
    /// The seal-time local↔global term-id map for this page.
    translation: PageTranslation,
    /// The query-time re-materialization cache. Empty after the seal pass; filled by
    /// [`PagedDataset::page`] on first access (deterministic per the provider
    /// contract).
    resident: OnceLock<Arc<RdfDataset>>,
}

/// A reference, in-memory, demand-paged dataset composing many frozen
/// [`RdfDataset`] pages into one logical [`DatasetView`] keyed on
/// [`GlobalTermId`]. See the [module docs](self) for the id-composition and
/// seal-then-lazy contracts.
pub struct PagedDataset {
    /// The shared value-interner: the single global id space over all pages. Its own
    /// reverse value index answers [`term_id_by_value`](DatasetView::term_id_by_value)
    /// and [`resolve`](DatasetView::resolve) WITHOUT materializing any page.
    dictionary: GlobalDictionary,
    /// The pages in ascending [`PageId`] order (`pages[i].id == PageId(i)`).
    pages: Box<[PageSlot]>,
    /// The demand-paging hook. Shared (`Arc`) so the dataset stays `Send + Sync`.
    provider: Arc<dyn PageProvider>,
    /// The OR of every page's capabilities (an honest composite: a flag is on iff
    /// some page surfaces it).
    caps: RdfStoreCapabilities,
    /// The total quad count, summed at seal time so [`len_hint`](DatasetView::len_hint)
    /// never materializes a page.
    total_quads: usize,
}

impl std::fmt::Debug for PagedDataset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `dyn PageProvider` is not `Debug`; summarize by page count instead.
        f.debug_struct("PagedDataset")
            .field("page_count", &self.pages.len())
            .field("term_count", &self.dictionary.len())
            .field("total_quads", &self.total_quads)
            .field("caps", &self.caps)
            // `dictionary`, `pages`, and the `dyn` `provider` are intentionally
            // summarized above rather than dumped.
            .finish_non_exhaustive()
    }
}

impl PagedDataset {
    /// Seal a provider's pages into a queryable paged dataset (the eager
    /// correctness-required pass; see the [module docs](self)).
    ///
    /// For each `PageId` in `0..page_count` this materializes the page ONCE, folds
    /// its terms into the shared [`GlobalDictionary`] by value, ORs its capabilities,
    /// and adds its quad count to the running total — then drops the materialized
    /// page (query-time access re-materializes it, cached). Returns the first
    /// [`PageFault`] if any page fails to materialize.
    ///
    /// # Errors
    ///
    /// Propagates a [`PageFault`] from the provider if a page cannot be materialized.
    pub fn from_provider(provider: Arc<dyn PageProvider>) -> Result<Self, PageFault> {
        let page_count = provider.page_count();
        let mut dictionary = GlobalDictionary::new();
        let mut caps = RdfStoreCapabilities::plain_rdf();
        let mut total_quads = 0usize;
        let mut pages: Vec<PageSlot> = Vec::with_capacity(page_count);
        for i in 0..page_count {
            let id = PageId(u32::try_from(i).expect("page count fits u32"));
            let page = provider.materialize(id)?;
            // Boundary G1: fold this page's terms into the shared dictionary BY VALUE.
            // This must happen for every page before any query so the dictionary is
            // complete (value lookups are correct for terms on not-yet-requeried
            // pages).
            let translation = PageTranslation::build(&page, &mut dictionary);
            caps = caps.union(page.capabilities());
            total_quads += page.quad_count();
            pages.push(PageSlot {
                id,
                translation,
                // Sealed page is intentionally NOT kept resident: query-time access
                // re-materializes via the provider (cheap Arc::clone for the in-memory
                // provider; a counted rebuild for the demand provider — the observable
                // lazy hook).
                resident: OnceLock::new(),
            });
        }
        Ok(Self {
            dictionary,
            pages: pages.into_boxed_slice(),
            provider,
            caps,
            total_quads,
        })
    }

    /// The number of pages composing this dataset.
    #[must_use]
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// The shared global dictionary over all pages (read-only).
    #[must_use]
    pub fn dictionary(&self) -> &GlobalDictionary {
        &self.dictionary
    }

    /// The [`PageTranslation`] of a page, by ordinal. `None` if `id` is out of range.
    /// Exposed so a test can prove one [`GlobalTermId`] maps to DISTINCT local
    /// [`TermId`]s on different pages.
    #[must_use]
    pub fn translation(&self, id: PageId) -> Option<&PageTranslation> {
        let index = usize::try_from(id.0).ok()?;
        self.pages.get(index).map(|slot| &slot.translation)
    }

    /// The cached, fallible per-page getter: fast-path the resident [`OnceLock`],
    /// otherwise re-materialize through the provider and cache it. Deterministic per
    /// the [`PageProvider`] contract.
    fn page(&self, id: PageId) -> Result<&Arc<RdfDataset>, PageFault> {
        let index = usize::try_from(id.0).expect("page id fits usize");
        let slot = &self.pages[index];
        if let Some(ds) = slot.resident.get() {
            return Ok(ds);
        }
        let ds = self.provider.materialize(id)?;
        let _ = slot.resident.set(ds);
        Ok(slot
            .resident
            .get()
            .expect("resident cell set immediately above"))
    }
}

/// Translate a whole `(s, p, o, g)` global pattern to this page's local id space, or
/// `None` if ANY bound id (including a `Named` graph) is absent on the page — in which
/// case the page cannot match and is skipped. An unbound axis (`None`) stays unbound;
/// a bound axis present on the page becomes its local `TermId`.
#[allow(clippy::type_complexity)]
fn translate_pattern(
    translation: &PageTranslation,
    s: Option<GlobalTermId>,
    p: Option<GlobalTermId>,
    o: Option<GlobalTermId>,
    g: GraphMatch<GlobalTermId>,
) -> Option<(
    Option<TermId>,
    Option<TermId>,
    Option<TermId>,
    GraphMatch<TermId>,
)> {
    // For each bound axis, `?` short-circuits to `None` (skip the page) when the term
    // is absent; an unbound axis passes through as `None`.
    let s = match s {
        None => None,
        Some(global) => Some(translation.to_local(global)?),
    };
    let p = match p {
        None => None,
        Some(global) => Some(translation.to_local(global)?),
    };
    let o = match o {
        None => None,
        Some(global) => Some(translation.to_local(global)?),
    };
    let g = match g {
        GraphMatch::Any => GraphMatch::Any,
        GraphMatch::Default => GraphMatch::Default,
        GraphMatch::Named(gid) => GraphMatch::Named(translation.to_local(gid)?),
    };
    Some((s, p, o, g))
}

/// Map a page-local [`QuadIds`] back to the shared global id space.
fn map_quad_to_global(translation: &PageTranslation, q: QuadIds<TermId>) -> QuadIds<GlobalTermId> {
    QuadIds {
        s: translation.to_global(q.s),
        p: translation.to_global(q.p),
        o: translation.to_global(q.o),
        g: q.g.map(|g| translation.to_global(g)),
    }
}

impl DatasetView for PagedDataset {
    type Id = GlobalTermId;
    type ProbePlan = ();

    fn quads(&self) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        let mut out: Vec<QuadIds<GlobalTermId>> = Vec::with_capacity(self.total_quads);
        for slot in &self.pages {
            let page = self
                .page(slot.id)
                .expect("sealed page must re-materialize deterministically");
            for q in page.quads() {
                out.push(map_quad_to_global(&slot.translation, q));
            }
        }
        out.into_iter()
    }

    fn quad_refs(&self) -> impl Iterator<Item = QuadRef<'_, GlobalTermId>> + '_ {
        // `quads()` owns its rows (the pages are materialized transiently inside it),
        // so resolving each through the shared dictionary borrows only `self`.
        self.quads().map(move |q| QuadRef {
            s: self.dictionary.resolve(q.s),
            p: self.dictionary.resolve(q.p),
            o: self.dictionary.resolve(q.o),
            g: q.g.map(|g| self.dictionary.resolve(g)),
        })
    }

    fn resolve(&self, id: GlobalTermId) -> crate::ir::TermRef<'_, GlobalTermId> {
        // O(1); never materializes a page.
        self.dictionary.resolve(id)
    }

    fn quads_for_pattern(
        &self,
        s: Option<GlobalTermId>,
        p: Option<GlobalTermId>,
        o: Option<GlobalTermId>,
        g: GraphMatch<GlobalTermId>,
    ) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        let mut out: Vec<QuadIds<GlobalTermId>> = Vec::new();
        for slot in &self.pages {
            // Skip a page that cannot possibly match a bound id (incl. a Named graph).
            let Some((ls, lp, lo, lg)) = translate_pattern(&slot.translation, s, p, o, g) else {
                continue;
            };
            let page = self
                .page(slot.id)
                .expect("sealed page must re-materialize deterministically");
            for q in page.quads_for_pattern_indexed(ls, lp, lo, lg) {
                out.push(map_quad_to_global(&slot.translation, q));
            }
        }
        out.into_iter()
    }

    fn term_id_by_value(&self, value: &TermValue) -> Option<GlobalTermId> {
        // The dictionary is complete after the seal pass; never materializes a page.
        self.dictionary.term_id_by_value(value)
    }

    fn capabilities(&self) -> RdfStoreCapabilities {
        self.caps
    }

    fn len_hint(&self) -> Option<usize> {
        Some(self.total_quads)
    }

    fn probe_plan(
        &self,
        _s_bound: bool,
        _p_bound: bool,
        _o_bound: bool,
        _g: GraphMatch<GlobalTermId>,
    ) {
    }

    fn quads_for_pattern_with_plan(
        &self,
        _plan: &(),
        s: Option<GlobalTermId>,
        p: Option<GlobalTermId>,
        o: Option<GlobalTermId>,
        g: GraphMatch<GlobalTermId>,
    ) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        // The unit plan carries nothing; forward to the pattern query.
        self.quads_for_pattern(s, p, o, g)
    }

    fn cardinality_estimate(
        &self,
        s: Option<GlobalTermId>,
        p: Option<GlobalTermId>,
        o: Option<GlobalTermId>,
        g: GraphMatch<GlobalTermId>,
    ) -> usize {
        // The Merge-scope summation: Σ over non-skipped pages of each page's own
        // O(log n) estimate on the translated pattern.
        let mut total = 0usize;
        for slot in &self.pages {
            let Some((ls, lp, lo, lg)) = translate_pattern(&slot.translation, s, p, o, g) else {
                continue;
            };
            let page = self
                .page(slot.id)
                .expect("sealed page must re-materialize deterministically");
            total += page.cardinality_estimate(ls, lp, lo, lg);
        }
        total
    }

    fn term_count(&self) -> usize {
        self.dictionary.len()
    }

    fn stats_fingerprint(&self) -> u64 {
        // Mirror RdfDataset's coarse fingerprint: hash (total quads, distinct terms).
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.total_quads.hash(&mut h);
        self.dictionary.len().hash(&mut h);
        h.finish()
    }

    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        let mut out: Vec<QuadIds<GlobalTermId>> = Vec::new();
        for slot in &self.pages {
            let page = self
                .page(slot.id)
                .expect("sealed page must re-materialize deterministically");
            for q in page.reifier_quads() {
                out.push(map_quad_to_global(&slot.translation, q));
            }
        }
        out.into_iter()
    }

    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        let mut out: Vec<QuadIds<GlobalTermId>> = Vec::new();
        for slot in &self.pages {
            let page = self
                .page(slot.id)
                .expect("sealed page must re-materialize deterministically");
            for q in page.annotation_quads() {
                out.push(map_quad_to_global(&slot.translation, q));
            }
        }
        out.into_iter()
    }

    fn annotations_of_with_graph(
        &self,
        reifier: GlobalTermId,
    ) -> impl Iterator<Item = (GlobalTermId, GlobalTermId, Option<GlobalTermId>)> + '_ {
        let mut out: Vec<(GlobalTermId, GlobalTermId, Option<GlobalTermId>)> = Vec::new();
        for slot in &self.pages {
            // Translate the reifier to each page's local id; a page that lacks it
            // contributes nothing.
            let Some(local_reifier) = slot.translation.to_local(reifier) else {
                continue;
            };
            let page = self
                .page(slot.id)
                .expect("sealed page must re-materialize deterministically");
            for (p, o, g) in page.annotations_of_with_graph(local_reifier) {
                out.push((
                    slot.translation.to_global(p),
                    slot.translation.to_global(o),
                    g.map(|g| slot.translation.to_global(g)),
                ));
            }
        }
        out.into_iter()
    }
}
