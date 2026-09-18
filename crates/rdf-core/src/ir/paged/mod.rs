// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A reference, in-memory, demand-paged dataset — [`PagedDataset`] — that composes
//! many frozen [`RdfDataset`] pages into ONE logical dataset queried through the
//! id-agnostic [`DatasetView`] trait, with `type Id = GlobalTermId`.
//!
//! # What a page is, and how ids compose
//!
//! Each page is a frozen [`RdfDataset`] with its own dense, dataset-local
//! [`TermId`] space. The paged view addresses terms in the shared
//! [`GlobalTermId`] space of a single
//! [`GlobalDictionary`]: every page term is re-interned
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
//! [`OnceLock`]. For an [`InMemoryPageProvider`] the
//! re-materialize is a cheap `Arc::clone`; for a
//! [`CountingDemandProvider`] it is a counted
//! rebuild — which is exactly what makes the lazy hook observable at query time.
//!
//! # Infallible and fallible query surfaces
//!
//! Direct use of `PagedDataset` as a [`DatasetView`] requires a provider that cannot
//! fail after sealing, such as [`InMemoryPageProvider`]. If provider materialization
//! can fail, drift generations, be cancelled, reach a deadline, or exceed a resource
//! budget, construct a fresh [`PagedQueryView`] with [`PagedDataset::query_view`] for
//! each execution and use an evaluator entry point that accepts
//! [`FallibleDatasetView`](crate::FallibleDatasetView). Only its final ready status is
//! a completeness certificate; iterator exhaustion alone is not.
//!
//! # Page admission (G10)
//!
//! A page is materialized only when it can PROVABLY contribute a row, and the
//! decision is made from sealed metadata before any provider call. Each page's
//! [`PageTranslation`] carries an exact summary — per-term occurrence counts for the
//! base-quad subject, predicate and object positions and for each side table's
//! reifier column, plus per-graph row counts for all three composed streams — keyed
//! in that page's own LOCAL [`TermId`] space. Local keying is what makes the summary
//! invariant under [`compact`](PagedDataset::compact), which renumbers only the
//! global side.
//!
//! `admission::admit_pattern` applies the law and reports a named reason when it
//! refuses, so a refusal can be asserted against the clause that produced it. Each
//! clause is EXACT — it refuses only when no base quad on that page can match that
//! axis — and their conjunction is a sound but not complete filter: an admitted page
//! may still yield nothing, which is correct. Graph-constrained reads additionally
//! walk a dataset-level graph-to-page index rather than every page; that index is
//! derived at every constructor and never persisted, so page and global renumbering
//! are both picked up automatically.
//!
//! The summary authorizes SKIPPING, and a skipped page is never materialized, so a
//! check at admission can only ever observe the harmless over-reporting direction.
//! Admission therefore carries a `debug_assertions` re-derivation, and
//! [`PagedDataset::verify_parts`] is the explicitly paid pass that reads every page
//! and is the only place the check is complete.
//!
//! # Determinism
//!
//! Pages iterate in ascending [`PageId`] order and each page yields in its frozen
//! in-page order, so every egress (`quads`, `quads_for_pattern`, the reifier/
//! annotation views) is deterministic. Pages are quad-disjoint (G3, enforced at
//! freeze), so no cross-page dedup is needed. A [`PagedQueryView`] additionally
//! records first page requests in evaluation order and charges each admitted page
//! exactly once. Cardinality estimation reads the same sealed counts on both
//! surfaces and never consults page residency, so plan choice — and therefore the
//! recorded request sequence — is a function of the snapshot and the pattern alone.

pub(crate) mod admission;
pub(crate) mod graph_index;
pub mod provider;
pub mod query;
pub(crate) mod summary;
pub mod translation;

use std::collections::{BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock};

use crate::RdfStoreCapabilities;
use crate::dataset_view::{DatasetView, GraphMatch};
use crate::ir::{GlobalDictionary, GlobalTermId, QuadIds, QuadRef, RdfDataset, TermId, TermValue};
use admission::PageAdmission;
use graph_index::GraphPageIndex;
use summary::{PageStream, PageSummary};

pub use provider::{
    CountingDemandProvider, InMemoryPageProvider, PageFault, PageFaultKind, PageGeneration, PageId,
    PageMaterialization, PageProvider, SubsetPageProvider,
};
pub use query::{PagedQueryError, PagedQueryEvidence, PagedQueryLimits, PagedQueryView};
pub use translation::PageTranslation;

/// One page of a [`PagedDataset`]: its [`PageId`], the local↔global
/// [`PageTranslation`] built at seal time, and a lazily-cached resident
/// [`RdfDataset`] (empty after the seal pass; filled on first query-time access).
#[derive(Debug)]
pub(crate) struct PageSlot {
    /// The page's dense ordinal (equals its index in `PagedDataset::pages`).
    id: PageId,
    /// The seal-time local↔global term-id map for this page.
    translation: PageTranslation,
    /// The query-time re-materialization cache. Empty after the seal pass; filled by
    /// [`PagedDataset::page`] on first access (deterministic per the provider
    /// contract).
    resident: OnceLock<Arc<RdfDataset>>,
    /// This page's capabilities, captured at seal time so
    /// [`with_pages`](PagedDataset::with_pages) can recompute the honest composite of
    /// a page SUBSET without re-materializing.
    caps: RdfStoreCapabilities,
    /// This page's quad count, captured at seal time so a page-subset dataset can sum
    /// its own `total_quads` without re-materializing.
    quad_count: usize,
    /// The deterministic provider-reported byte charge captured at seal time.
    byte_len: u64,
}

/// Why sealing a provider into a [`PagedDataset`] failed.
///
/// The seal pass is the checked construction path: besides a provider
/// [`PageFault`], it REFUSES (never silently dedups) when the pages are not
/// quad-disjoint in [`GlobalTermId`] space — i.e. the same global quad `(s, p, o, g)`
/// occurs on more than one page (G3). Naming both offending pages and the quad makes
/// the refusal actionable.
#[derive(Debug)]
#[non_exhaustive]
pub enum PagedFreezeError {
    /// A page could not be materialized by the provider.
    Page(PageFault),
    /// The provider changed snapshots while sealing, or a warm-restart certificate
    /// names a different generation from the provider's current snapshot.
    GenerationMismatch {
        /// The generation captured by sealing or stored with the warm metadata.
        expected: PageGeneration,
        /// The provider's observed generation.
        actual: PageGeneration,
    },
    /// Warm metadata and the provider describe different numbers of pages.
    PageCountMismatch {
        /// The number of persisted page metadata records.
        metadata: usize,
        /// The number of pages exposed by the provider.
        provider: usize,
    },
    /// Two pages carry the SAME global quad — the pages are not quad-disjoint, so the
    /// seal refuses rather than collapse the duplicate. Boxed to keep the enum (and
    /// therefore the seal `Result`) small.
    QuadOverlap(Box<PagedQuadOverlap>),
    /// A page's freshly re-derived `PageSummary` disagrees with the one it was
    /// sealed with — its materialized content has drifted since certification.
    ///
    /// The pruning law (the `admit_pattern` law, over the dataset-level
    /// graph-to-page index) trusts the sealed summary to authorize SKIPPING a page
    /// without materializing it. An UNDER-reporting drift (the sealed summary claims
    /// fewer rows, in fewer places, than the page actually holds) is exactly the
    /// direction no materialization-time check can ever observe, because a page the
    /// pruning law skips is never materialized in the first place — only
    /// [`PagedDataset::verify_parts`], which materializes every page
    /// unconditionally, can reach it. Raised only there.
    SummaryDrift {
        /// The page whose materialized content no longer matches its sealed summary.
        page: PageId,
        /// Diagnostic detail describing the disagreement.
        message: String,
    },
}

/// Which composed quad stream a [`PagedFreezeError::QuadOverlap`] refusal came from.
/// The paged view exposes three cross-page streams that each assume disjoint pages and
/// do NO cross-page dedup — the primary quads and the two RDF 1.2 side tables — so the
/// seal enforces disjointness on all three, not just the primary quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagedQuadTable {
    /// The base (asserted) quads, surfaced by [`DatasetView::quads`].
    Primary,
    /// The reifier bindings, surfaced by [`DatasetView::reifier_quads`] as
    /// `(reifier, rdf:reifies, triple-term)` virtual quads.
    Reifier,
    /// The annotation triples, surfaced by [`DatasetView::annotation_quads`].
    Annotation,
}

impl PagedQuadTable {
    /// A short human label for the refusal message.
    const fn label(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Reifier => "reifier side-table",
            Self::Annotation => "annotation side-table",
        }
    }
}

/// The offending quad of a [`PagedFreezeError::QuadOverlap`] refusal: the two pages
/// that share it, which composed stream it belongs to, and the quad resolved to
/// dataset-independent [`TermValue`]s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PagedQuadOverlap {
    /// The lower-numbered page that first carried the quad.
    pub first_page: PageId,
    /// The higher-numbered page that repeats it.
    pub second_page: PageId,
    /// Which cross-page quad stream (primary or a side table) the duplicate is in.
    pub table: PagedQuadTable,
    /// The shared quad's subject value.
    pub subject: TermValue,
    /// The shared quad's predicate value.
    pub predicate: TermValue,
    /// The shared quad's object value.
    pub object: TermValue,
    /// The shared quad's graph value (`None` = the default graph).
    pub graph: Option<TermValue>,
}

impl From<PageFault> for PagedFreezeError {
    fn from(fault: PageFault) -> Self {
        Self::Page(fault)
    }
}

impl std::fmt::Display for PagedFreezeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Page(fault) => write!(f, "{fault}"),
            Self::GenerationMismatch { expected, actual } => write!(
                f,
                "paged snapshot generation mismatch: metadata/seal generation {expected}, \
                 provider generation {actual}"
            ),
            Self::PageCountMismatch { metadata, provider } => write!(
                f,
                "paged snapshot page-count mismatch: {metadata} metadata records, \
                 {provider} provider pages"
            ),
            Self::QuadOverlap(o) => write!(
                f,
                "pages {} and {} are not quad-disjoint in the {} stream: the global quad \
                 (s={:?}, p={:?}, o={:?}, g={:?}) occurs on both; PagedDataset refuses \
                 to silently dedup (G3)",
                o.first_page.0,
                o.second_page.0,
                o.table.label(),
                o.subject,
                o.predicate,
                o.object,
                o.graph
            ),
            Self::SummaryDrift { page, message } => write!(
                f,
                "page {}'s materialized content disagrees with its sealed summary: {message}",
                page.0
            ),
        }
    }
}

impl std::error::Error for PagedFreezeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Page(fault) => Some(fault),
            Self::GenerationMismatch { .. }
            | Self::PageCountMismatch { .. }
            | Self::QuadOverlap(_)
            | Self::SummaryDrift { .. } => None,
        }
    }
}

/// One page's pre-built seal metadata, the unit of
/// [`PagedDataset::from_parts`]/[`to_parts`](PagedDataset::to_parts).
///
/// A [`PageTranslation`] can only be produced by walking a page (its by-value
/// re-intern boundary), so this is NOT how a fresh corpus is ingested — it is how an
/// ALREADY-INDEXED store is reconstituted. A backend that persists its
/// [`GlobalDictionary`] and per-page translations reloads a [`PagedDataset`] from these
/// parts WITHOUT re-scanning every page (the warm-restart path), where
/// [`from_provider`](PagedDataset::from_provider) would eagerly re-materialize all of
/// them. `capabilities`, `quad_count`, and `byte_len` are the page's seal-time
/// metadata, kept so the reconstituted dataset answers
/// [`capabilities`](DatasetView::capabilities) and [`len_hint`](DatasetView::len_hint)
/// without a materialization.
#[derive(Debug, Clone)]
pub struct PagePart {
    /// The page's local↔global term-id map (as built at its original seal).
    pub translation: PageTranslation,
    /// The page's capabilities, captured at seal time.
    pub capabilities: RdfStoreCapabilities,
    /// The page's quad count, captured at seal time.
    pub quad_count: usize,
    /// The provider-reported byte charge, captured at seal time.
    pub byte_len: u64,
}

/// A reference demand-paged dataset composing many frozen [`RdfDataset`] pages into
/// one logical [`DatasetView`] keyed on [`GlobalTermId`].
///
/// Direct `DatasetView` reads carry the infallible-provider contract. For a provider
/// that can fail after sealing, start each operation with [`Self::query_view`] and
/// execute it through a [`FallibleDatasetView`](crate::FallibleDatasetView)-aware
/// boundary. See the [module docs](self) for the id-composition, seal-then-lazy, and
/// completeness contracts.
pub struct PagedDataset {
    /// The shared value-interner: the single global id space over all pages. Its own
    /// reverse value index answers [`term_id_by_value`](DatasetView::term_id_by_value)
    /// and [`resolve`](DatasetView::resolve) WITHOUT materializing any page.
    dictionary: GlobalDictionary,
    /// The pages in ascending [`PageId`] order (`pages[i].id == PageId(i)`).
    pages: Box<[PageSlot]>,
    /// The demand-paging hook. Shared (`Arc`) so the dataset stays `Send + Sync`.
    provider: Arc<dyn PageProvider>,
    /// The immutable provider snapshot to which the dictionary, translations, and
    /// per-page metadata belong.
    generation: PageGeneration,
    /// The OR of every page's capabilities (an honest composite: a flag is on iff
    /// some page surfaces it).
    caps: RdfStoreCapabilities,
    /// The total quad count, summed at seal time so [`len_hint`](DatasetView::len_hint)
    /// never materializes a page.
    total_quads: usize,
    /// The dataset-level "which pages carry graph G" index, DERIVED (never
    /// persisted) from the completed `pages` in every constructor — see
    /// [`GraphPageIndex::derive`]. Read via [`graph_index`](Self::graph_index) by
    /// the page-admission candidate selection (`admission::candidate_pages`) in
    /// both `quads_for_pattern` and `cardinality_estimate` below.
    graph_index: GraphPageIndex,
}

impl std::fmt::Debug for PagedDataset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `dyn PageProvider` is not `Debug`; summarize by page count instead.
        f.debug_struct("PagedDataset")
            .field("page_count", &self.pages.len())
            .field("term_count", &self.dictionary.len())
            .field("total_quads", &self.total_quads)
            .field("generation", &self.generation)
            .field("caps", &self.caps)
            // `dictionary`, `pages`, and the `dyn` `provider` are intentionally
            // summarized above rather than dumped.
            .finish_non_exhaustive()
    }
}

impl PagedDataset {
    /// Seal a provider's pages into a queryable paged dataset (the eager
    /// correctness-required pass; see the [module docs](self)). This is the CHECKED
    /// construction path — the only public constructor — so the quad-disjointness
    /// refusal below is always on the production path, never a test-only helper.
    ///
    /// For each `PageId` in `0..page_count` this materializes the page ONCE, folds
    /// its terms into the shared [`GlobalDictionary`] by value, ORs its capabilities,
    /// and adds its quad count to the running total — then drops the materialized
    /// page (query-time access re-materializes it, cached).
    ///
    /// # Quad-disjointness (G3)
    ///
    /// Pages MUST be quad-disjoint in [`GlobalTermId`] space. After mapping each
    /// page's quads to the shared id space, if the SAME global quad `(s, p, o, g)`
    /// appears on more than one page the seal REFUSES with
    /// [`PagedFreezeError::QuadOverlap`] — it never silently dedups (the paged read
    /// path assumes disjoint pages and does no cross-page dedup). The disjoint success
    /// path is behavior-identical to before.
    ///
    /// # Errors
    ///
    /// [`PagedFreezeError::Page`] if a page cannot be materialized or reports the
    /// wrong generation; [`PagedFreezeError::GenerationMismatch`] if the provider
    /// changes snapshots during the seal; [`PagedFreezeError::PageCountMismatch`] if
    /// its page count changes; or [`PagedFreezeError::QuadOverlap`] if two pages share
    /// a global quad.
    pub fn from_provider(provider: Arc<dyn PageProvider>) -> Result<Self, PagedFreezeError> {
        let generation = provider.generation();
        let page_count = provider.page_count();
        let mut dictionary = GlobalDictionary::new();
        let mut caps = RdfStoreCapabilities::plain_rdf();
        let mut total_quads = 0usize;
        let mut pages: Vec<PageSlot> = Vec::with_capacity(page_count);
        // G3 refusal ledgers: the first page on which each global quad was seen, kept
        // PER composed stream. A second sighting on a LATER page is a non-disjoint
        // overlap in that stream. The paged read path concatenates each stream across
        // pages and does NO cross-page dedup, so the seal enforces disjointness on all
        // three — the primary quads AND both RDF 1.2 side tables (reifier bindings,
        // annotation triples). Insertion order is deterministic (pages ascending, each
        // page's quads in frozen order), so the refusal is reproducible. The ledgers
        // are SEPARATE: a base quad and a reifier virtual quad may legitimately share
        // the same `(s, p, o, g)` values without being a duplicate emission, because
        // they surface through different trait methods.
        type GlobalQuad = (
            GlobalTermId,
            GlobalTermId,
            GlobalTermId,
            Option<GlobalTermId>,
        );
        let mut seen_primary: HashMap<GlobalQuad, PageId> = HashMap::new();
        let mut seen_reifier: HashMap<GlobalQuad, PageId> = HashMap::new();
        let mut seen_annotation: HashMap<GlobalQuad, PageId> = HashMap::new();
        for i in 0..page_count {
            let id = PageId(u32::try_from(i).expect("page count fits u32"));
            let materialization = provider.materialize(id)?;
            if materialization.generation != generation {
                return Err(PageFault::stale_generation(
                    id,
                    generation,
                    materialization.generation,
                )
                .into());
            }
            let page = materialization.dataset;
            // Boundary G1: fold this page's terms into the shared dictionary BY VALUE.
            // This must happen for every page before any query so the dictionary is
            // complete (value lookups are correct for terms on not-yet-requeried
            // pages).
            let translation = PageTranslation::build(&page, &mut dictionary);
            let page_caps = page.capabilities();
            let page_quads = page.quad_count();
            // G3: map each of this page's quads (primary + side tables) to the shared id
            // space and refuse on a cross-page collision. A page's own quads within a
            // stream are distinct and its terms map injectively to global ids, so any
            // collision here is with an EARLIER page. `key` is `Copy`, so it survives
            // the move into the ledger for the error report.
            let overlap = |key: GlobalQuad, first_page: PageId, table: PagedQuadTable| {
                PagedFreezeError::QuadOverlap(Box::new(PagedQuadOverlap {
                    first_page,
                    second_page: id,
                    table,
                    subject: dictionary.term_value(key.0),
                    predicate: dictionary.term_value(key.1),
                    object: dictionary.term_value(key.2),
                    graph: key.3.map(|g| dictionary.term_value(g)),
                }))
            };
            for q in page.quads() {
                let g = map_quad_to_global(&translation, q);
                let key: GlobalQuad = (g.s, g.p, g.o, g.g);
                if let Some(first_page) = seen_primary.insert(key, id) {
                    return Err(overlap(key, first_page, PagedQuadTable::Primary));
                }
            }
            for q in page.reifier_quads() {
                let g = map_quad_to_global(&translation, q);
                let key: GlobalQuad = (g.s, g.p, g.o, g.g);
                if let Some(first_page) = seen_reifier.insert(key, id) {
                    return Err(overlap(key, first_page, PagedQuadTable::Reifier));
                }
            }
            for q in page.annotation_quads() {
                let g = map_quad_to_global(&translation, q);
                let key: GlobalQuad = (g.s, g.p, g.o, g.g);
                if let Some(first_page) = seen_annotation.insert(key, id) {
                    return Err(overlap(key, first_page, PagedQuadTable::Annotation));
                }
            }
            caps = caps.union(page_caps);
            total_quads += page_quads;
            pages.push(PageSlot {
                id,
                translation,
                // Sealed page is intentionally NOT kept resident: query-time access
                // re-materializes via the provider (cheap Arc::clone for the in-memory
                // provider; a counted rebuild for the demand provider — the observable
                // lazy hook).
                resident: OnceLock::new(),
                caps: page_caps,
                quad_count: page_quads,
                byte_len: materialization.byte_len,
            });
        }
        let current_generation = provider.generation();
        if current_generation != generation {
            return Err(PagedFreezeError::GenerationMismatch {
                expected: generation,
                actual: current_generation,
            });
        }
        let current_page_count = provider.page_count();
        if current_page_count != page_count {
            return Err(PagedFreezeError::PageCountMismatch {
                metadata: page_count,
                provider: current_page_count,
            });
        }
        let pages = pages.into_boxed_slice();
        // Derived from the FINAL, densely-numbered `pages` slice, after every page id
        // is assigned — the sole producer, per page-set, of the dataset-level graph
        // index.
        let graph_index = GraphPageIndex::derive(&pages);
        Ok(Self {
            dictionary,
            pages,
            provider,
            generation,
            caps,
            total_quads,
            graph_index,
        })
    }

    /// Reconstitute a paged dataset from PRE-BUILT parts WITHOUT materializing any page
    /// — the warm-restart / already-indexed constructor.
    ///
    /// [`from_provider`](Self::from_provider) is the eager path: it materializes every
    /// page once to fold its terms into the shared dictionary and to check G3
    /// quad-disjointness. A store that has ALREADY done that and persisted the resulting
    /// [`GlobalDictionary`] and per-page [`PagePart`]s does not need to re-scan — this
    /// constructor rebuilds the dataset from those parts and defers every page load to
    /// query time (each page's [`OnceLock`] starts empty). For a large store that is the
    /// difference between an O(all pages) reload and an O(1) one.
    ///
    /// `caps` and `total_quads` are DERIVED from the parts (the OR of the per-page
    /// capabilities and the sum of the per-page quad counts), so the reconstituted
    /// dataset answers [`capabilities`](DatasetView::capabilities) and
    /// [`len_hint`](DatasetView::len_hint) without a materialization.
    ///
    /// # Disjointness
    ///
    /// Unlike [`from_provider`](Self::from_provider), this path does NOT re-verify G3
    /// quad-disjointness — it cannot without reading the pages, which is the cost it
    /// exists to avoid. The caller warrants that `parts` came from a previously-sealed
    /// dataset (e.g. [`to_parts`](Self::to_parts)) whose pages were disjoint; the read
    /// path relies on that invariant exactly as `from_provider`'s output does.
    ///
    /// # Errors
    ///
    /// Returns [`PagedFreezeError::PageCountMismatch`] if the metadata and provider
    /// page counts differ, or [`PagedFreezeError::GenerationMismatch`] if the
    /// provider no longer exposes the certified snapshot. No page is materialized.
    pub fn from_parts(
        dictionary: GlobalDictionary,
        provider: Arc<dyn PageProvider>,
        generation: PageGeneration,
        parts: Vec<PagePart>,
    ) -> Result<Self, PagedFreezeError> {
        let provider_page_count = provider.page_count();
        if parts.len() != provider_page_count {
            return Err(PagedFreezeError::PageCountMismatch {
                metadata: parts.len(),
                provider: provider_page_count,
            });
        }
        let provider_generation = provider.generation();
        if generation != provider_generation {
            return Err(PagedFreezeError::GenerationMismatch {
                expected: generation,
                actual: provider_generation,
            });
        }
        let mut caps = RdfStoreCapabilities::plain_rdf();
        let mut total_quads = 0usize;
        let mut pages: Vec<PageSlot> = Vec::with_capacity(parts.len());
        for (i, part) in parts.into_iter().enumerate() {
            caps = caps.union(part.capabilities);
            total_quads += part.quad_count;
            pages.push(PageSlot {
                id: PageId(u32::try_from(i).expect("page count fits u32")),
                translation: part.translation,
                // Lazy: query-time access materializes through the provider (the whole
                // point — construction touches no page).
                resident: OnceLock::new(),
                caps: part.capabilities,
                quad_count: part.quad_count,
                byte_len: part.byte_len,
            });
        }
        let pages = pages.into_boxed_slice();
        // Derived from `slot.translation.summary()` alone (already carried by each
        // `PagePart`), so this reads no page — the warm-restart cost stays
        // O(page count), never O(page count × page size).
        let graph_index = GraphPageIndex::derive(&pages);
        Ok(Self {
            dictionary,
            pages,
            provider,
            generation,
            caps,
            total_quads,
            graph_index,
        })
    }

    /// Decompose this dataset into its shared [`GlobalDictionary`] and per-page
    /// [`PagePart`]s — the inverse of [`from_parts`](Self::from_parts).
    ///
    /// A pure clone of the seal-time metadata (no page is materialized): a store can
    /// persist these parts and later reload via [`from_parts`](Self::from_parts) without
    /// the eager re-scan. Pages are returned in ascending [`PageId`] order.
    #[must_use]
    pub fn to_parts(&self) -> (GlobalDictionary, PageGeneration, Vec<PagePart>) {
        let parts = self
            .pages
            .iter()
            .map(|slot| PagePart {
                translation: slot.translation.clone(),
                capabilities: slot.caps,
                quad_count: slot.quad_count,
                byte_len: slot.byte_len,
            })
            .collect();
        (self.dictionary.clone(), self.generation, parts)
    }

    /// Produce a variant retaining `keep` (each an existing [`PageId`]) as fresh dense
    /// pages `0..keep.len()`, in the given order — the page-eviction primitive.
    ///
    /// The returned dataset keeps the ORIGINAL (now possibly oversized) dictionary: a
    /// dropped page's ids are NOT reclaimed here — that is exactly what
    /// [`compact`](Self::compact) later does. This models the expected consumer's real use case:
    /// pages are evicted over time, the shared dictionary accumulates dead ids, and a
    /// periodic compaction reclaims them. The retained per-page translations are
    /// carried over verbatim (their local id spaces and the global ids they point at
    /// are untouched by dropping OTHER pages), and `caps` / `total_quads` are the
    /// honest recomputation over ONLY the retained pages.
    ///
    /// # Panics
    ///
    /// Panics if any id in `keep` is out of range (`>= page_count`).
    #[must_use]
    pub fn with_pages(&self, keep: &[PageId]) -> Self {
        // The retained pages keep the oversized dictionary; its dead ids survive until
        // `compact`. The clone is a deep copy over an identical term table, so its lazy
        // value index stays valid.
        let dictionary = self.dictionary.clone();
        let mut caps = RdfStoreCapabilities::plain_rdf();
        let mut total_quads = 0usize;
        let mut pages: Vec<PageSlot> = Vec::with_capacity(keep.len());
        for (new_index, &original) in keep.iter().enumerate() {
            let src = usize::try_from(original.0).expect("page id fits usize");
            let slot = &self.pages[src];
            caps = caps.union(slot.caps);
            total_quads += slot.quad_count;
            pages.push(PageSlot {
                id: PageId(u32::try_from(new_index).expect("page count fits u32")),
                translation: slot.translation.clone(),
                resident: OnceLock::new(),
                caps: slot.caps,
                quad_count: slot.quad_count,
                byte_len: slot.byte_len,
            });
        }
        let indices: Vec<PageId> = keep.to_vec();
        let pages = pages.into_boxed_slice();
        // Each retained page's PageSummary is keyed in that page's own LOCAL TermId
        // space, which subsetting never touches, so it carries over unchanged inside
        // `slot.translation` above. The graph INDEX, however, is keyed by GLOBAL id
        // and by PageId — and page ids are renumbered densely here — so it must be
        // rebuilt from the final `pages` slice rather than carried over.
        let graph_index = GraphPageIndex::derive(&pages);
        Self {
            dictionary,
            pages,
            provider: Arc::new(SubsetPageProvider::new(self.provider.clone(), indices)),
            generation: self.generation,
            caps,
            total_quads,
            graph_index,
        }
    }

    /// Drop one page, returning a variant over the remaining pages (a thin wrapper
    /// over [`with_pages`](Self::with_pages)). The surviving pages keep the oversized
    /// dictionary until [`compact`](Self::compact) reclaims the dropped page's dead
    /// ids.
    ///
    /// # Panics
    ///
    /// Panics if `drop` is out of range (`>= page_count`).
    #[must_use]
    pub fn drop_page(&self, drop: PageId) -> Self {
        assert!(
            (usize::try_from(drop.0).expect("page id fits usize")) < self.pages.len(),
            "drop_page: page {} out of range 0..{}",
            drop.0,
            self.pages.len()
        );
        let keep: Vec<PageId> = self
            .pages
            .iter()
            .map(|s| s.id)
            .filter(|&p| p != drop)
            .collect();
        self.with_pages(&keep)
    }

    /// Reclaim dead global ids and renumber the survivors DETERMINISTICALLY.
    ///
    /// Compaction is a pure function of the LIVE term-VALUE set:
    ///
    /// 1. **Mark-live** — union the retained pages' term tables into a
    ///    `BTreeSet<GlobalTermId>` (sorted, deterministic). A frozen page's term table
    ///    is closed over triple components and literal datatypes, so this union IS the
    ///    transitive set of ids the pages' quads and side tables reference.
    /// 2. **Renumber canonically** — resolve each live id to its dataset-independent
    ///    [`TermValue`], sort those VALUES in canonical order (see
    ///    [`TermValue`]'s `Ord`, which mirrors the serializer), and re-intern them in
    ///    that order into a fresh [`GlobalDictionary`]. The new id assignment therefore
    ///    depends only on the live value set — not the old numbering, ingest order, or
    ///    page order — so compacting the same live set twice yields identical ids.
    /// 3. **Rebuild translations** — pass each retained page's global side through the
    ///    old→new remap (a private [`PageTranslation`] pass); local id spaces and quad tables
    ///    are unchanged. The lazy `value_index` resets with the fresh dictionary.
    ///
    /// The result has no dead ids and a canonical global numbering, and preserves
    /// every quad's meaning (each survivor resolves to the identical `TermValue`).
    #[must_use]
    pub fn compact(&self) -> Self {
        // 1. Mark-live: the sorted union of every retained page's global ids.
        let mut live: BTreeSet<GlobalTermId> = BTreeSet::new();
        for slot in &self.pages {
            live.extend(slot.translation.global_ids().iter().copied());
        }
        // 2. Resolve to dataset-independent values and sort them CANONICALLY, so the
        // fresh id assignment is a pure function of the live value set.
        let mut live_values: Vec<(GlobalTermId, TermValue)> = live
            .iter()
            .map(|&old| (old, self.dictionary.term_value(old)))
            .collect();
        live_values.sort_by(|(_, a), (_, b)| a.cmp(b));
        // Re-intern in canonical order into a fresh dictionary; record old→new.
        let mut dictionary = GlobalDictionary::new();
        let mut remap: HashMap<GlobalTermId, GlobalTermId> =
            HashMap::with_capacity(live_values.len());
        for (old, value) in &live_values {
            // Every value here was read out of `self.dictionary`, where it passed
            // `GlobalDictionary::intern_iri` on the way in, so compaction re-interns
            // without re-parsing. See `GlobalDictionary::reintern_validated`.
            remap.insert(*old, dictionary.reintern_validated(value));
        }
        // 3. Rebuild each page's translation through the remap; local spaces unchanged.
        let pages: Vec<PageSlot> = self
            .pages
            .iter()
            .map(|slot| PageSlot {
                id: slot.id,
                translation: slot.translation.remap(|g| {
                    *remap
                        .get(&g)
                        .expect("every retained page global id is live and remapped")
                }),
                resident: OnceLock::new(),
                caps: slot.caps,
                quad_count: slot.quad_count,
                byte_len: slot.byte_len,
            })
            .collect();
        let pages = pages.into_boxed_slice();
        // Each page's PageSummary is keyed in that page's own LOCAL TermId space,
        // which compaction never touches (only the global side moves — see
        // `PageTranslation::remap`), so it carries over unchanged. The graph INDEX is
        // keyed by GLOBAL id, which compaction DOES renumber, so it must be rebuilt
        // from the completed `pages` slice rather than remapped in place.
        let graph_index = GraphPageIndex::derive(&pages);
        Self {
            dictionary,
            pages,
            provider: self.provider.clone(),
            generation: self.generation,
            caps: self.caps,
            total_quads: self.total_quads,
            graph_index,
        }
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

    /// The immutable provider snapshot certified by this paged dataset.
    #[must_use]
    pub const fn generation(&self) -> PageGeneration {
        self.generation
    }

    /// The [`PageTranslation`] of a page, by ordinal. `None` if `id` is out of range.
    /// Exposed so a test can prove one [`GlobalTermId`] maps to DISTINCT local
    /// [`TermId`]s on different pages.
    #[must_use]
    pub fn translation(&self, id: PageId) -> Option<&PageTranslation> {
        let index = usize::try_from(id.0).ok()?;
        self.pages.get(index).map(|slot| &slot.translation)
    }

    /// The dataset-level "which pages carry graph G" index (read-only). See
    /// [`GraphPageIndex`] — derived at every constructor, never persisted.
    #[must_use]
    pub(crate) fn graph_index(&self) -> &GraphPageIndex {
        &self.graph_index
    }

    /// The ascending [`PageId`]s the `admit_pattern` page-admission law admits for
    /// the global `(s, p, o, g)` pattern — the pre-execution footprint of that
    /// pattern, computed entirely from sealed `PageSummary` metadata. Materializes
    /// NOTHING.
    ///
    /// This is the prediction that pairs with [`PagedQueryEvidence::requested_pages`]:
    /// a query run over [`PagedQueryLimits::UNBOUNDED`] against the same pattern on
    /// the same snapshot requests exactly this page set (in this ascending order),
    /// because both this method and `quads_for_pattern`/`PagedQueryView::quads_for_pattern`
    /// apply the identical `admit_pattern` law to the identical candidate page set
    /// before any materialization. The admission law is SOUND but not COMPLETE: an
    /// admitted page may still yield zero matching rows once actually scanned, so
    /// this is an upper bound on rows touched, not a promise every listed page
    /// contributes a row.
    #[must_use]
    pub fn pages_for_pattern(
        &self,
        s: Option<GlobalTermId>,
        p: Option<GlobalTermId>,
        o: Option<GlobalTermId>,
        g: GraphMatch<GlobalTermId>,
    ) -> Vec<PageId> {
        let page_count = u32::try_from(self.pages.len()).expect("page count fits u32");
        admission::candidate_pages(self.graph_index(), page_count, g)
            .filter(|&page_id| {
                let index = usize::try_from(page_id.0).expect("page id fits usize");
                let slot = &self.pages[index];
                matches!(
                    admission::admit_pattern(&slot.translation, s, p, o, g),
                    PageAdmission::Admit(_)
                )
            })
            .collect()
    }

    /// The ascending [`PageId`]s owning at least one base-quad row in named graph
    /// `g`, computed from the dataset-level graph-to-page index alone (itself
    /// derived from sealed per-page `PageSummary` metadata). Materializes NOTHING.
    /// Equivalent to (and implemented via)
    /// `pages_for_pattern(None, None, None, GraphMatch::Named(g))`.
    #[must_use]
    pub fn pages_for_graph(&self, g: GlobalTermId) -> Vec<PageId> {
        self.pages_for_pattern(None, None, None, GraphMatch::Named(g))
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
        let materialization = self.provider.materialize(id)?;
        if materialization.generation != self.generation {
            return Err(PageFault::stale_generation(
                id,
                self.generation,
                materialization.generation,
            ));
        }
        if materialization.byte_len != slot.byte_len {
            return Err(PageFault::invalid_data(
                id,
                format!(
                    "byte charge changed from sealed {} to materialized {}",
                    slot.byte_len, materialization.byte_len
                ),
            ));
        }
        if materialization.dataset.term_count() != slot.translation.term_count() {
            return Err(PageFault::invalid_data(
                id,
                format!(
                    "term count changed from sealed {} to materialized {}",
                    slot.translation.term_count(),
                    materialization.dataset.term_count()
                ),
            ));
        }
        if materialization.dataset.quad_count() != slot.quad_count {
            return Err(PageFault::invalid_data(
                id,
                format!(
                    "quad count changed from sealed {} to materialized {}",
                    slot.quad_count,
                    materialization.dataset.quad_count()
                ),
            ));
        }
        if materialization.dataset.capabilities() != slot.caps {
            return Err(PageFault::invalid_data(
                id,
                "page capabilities changed after sealing",
            ));
        }
        // Debug-only full certification: re-derive the WHOLE `PageSummary` from the
        // materialized page and assert it equals the one sealed for this slot. The
        // O(1) checks above only ever catch OVER-reporting (a page admitted for
        // nothing, which is harmless): a page the pruning law skips on an
        // under-reporting summary is never materialized at all, so no check that
        // runs here — on the admit path — can ever observe that direction (see
        // `PagedFreezeError::SummaryDrift` and `verify_parts`, the only thing that
        // can). This assertion exists purely to catch a broken-invariant BUG in this
        // crate's own seal/admission machinery on every already-exercised test and
        // conformance page, at zero cost in a release build: `make check` compiles
        // with `debug-assertions = on` at opt-level 3 (see `AGENTS.md` section 4), so
        // this full re-derive runs across the entire test surface for free, while a
        // release build — which never sets `debug_assertions` — never pays the
        // O(page size) cost on this hot admission path.
        #[cfg(debug_assertions)]
        {
            let fresh = PageSummary::seal(&materialization.dataset);
            assert_eq!(
                &fresh,
                slot.translation.summary(),
                "page {}: materialized content's re-derived PageSummary disagrees with the \
                 summary it was sealed with",
                id.0
            );
        }
        let _ = slot.resident.set(materialization.dataset);
        Ok(slot
            .resident
            .get()
            .expect("resident cell set immediately above"))
    }

    /// Materialize EVERY page through the provider and certify each one's freshly
    /// re-derived `PageSummary` against the summary it was sealed with — the
    /// explicitly-paid, cold certification pass.
    ///
    /// The pruning law never materializes a page it decides to skip, so an
    /// UNDER-reporting sealed summary (one that claims fewer rows, or rows in fewer
    /// graphs, than the page actually holds) can silently authorize skipping a page
    /// that in fact holds matching rows — the worst failure mode in this codebase,
    /// because the short answer still comes wrapped in a completeness certificate.
    /// Nothing on the admission or query path can ever observe that direction: a
    /// skipped page is never read. `verify_parts` is the only thing that can, because
    /// it reads every page regardless of what any pruning decision would have done.
    ///
    /// Call this once, out of band, when a consumer reloads a persisted warm-restart
    /// index via [`from_parts`](Self::from_parts) and wants to prove the reloaded
    /// index is honest before trusting its pruning — rather than paying an
    /// `O(all pages)` scan on every ordinary restart. Contrast with
    /// [`from_provider`](Self::from_provider), which already certifies every page's
    /// summary AS it seals (each page is materialized once there specifically to
    /// build its summary), so a dataset built that way never needs this pass.
    ///
    /// # Errors
    ///
    /// [`PagedFreezeError::Page`] if the provider cannot materialize a page;
    /// [`PagedFreezeError::GenerationMismatch`] if a materialization reports a
    /// different generation than this dataset's certified one; or
    /// [`PagedFreezeError::SummaryDrift`], naming the first page whose freshly
    /// re-derived summary disagrees with the one it was sealed with.
    pub fn verify_parts(&self) -> Result<(), PagedFreezeError> {
        for slot in &self.pages {
            let materialization = self.provider.materialize(slot.id)?;
            if materialization.generation != self.generation {
                return Err(PagedFreezeError::GenerationMismatch {
                    expected: self.generation,
                    actual: materialization.generation,
                });
            }
            let fresh = PageSummary::seal(&materialization.dataset);
            if &fresh != slot.translation.summary() {
                return Err(PagedFreezeError::SummaryDrift {
                    page: slot.id,
                    message: format!(
                        "page {}: the summary sealed at construction time does not match the \
                         summary re-derived from the page's current materialized content — a \
                         pruning decision trusting the sealed summary could skip or misroute \
                         rows this page actually holds",
                        slot.id.0
                    ),
                });
            }
        }
        Ok(())
    }
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
        // Stream page-by-page: each page is materialized lazily on first touch (cached
        // in its `OnceLock`) and its quads are translated to the shared id space on the
        // fly. Peak extra memory is one page's translated rows, never the whole dataset.
        self.pages.iter().flat_map(move |slot| {
            let page = self
                .page(slot.id)
                .expect("sealed page must re-materialize deterministically");
            page.quads()
                .map(move |q| map_quad_to_global(&slot.translation, q))
        })
    }

    fn quad_refs(&self) -> impl Iterator<Item = QuadRef<'_, GlobalTermId>> + '_ {
        // `quads()` yields OWNED id rows (each page is materialized behind `self`'s
        // per-page cache, not by the caller), so resolving each through the shared
        // dictionary borrows only `self`.
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
        // Narrow the candidate page set from the graph axis first (zero allocation:
        // either every page, or a graph-index posting list — both ascending), then
        // apply the full per-axis admission law to each candidate BEFORE it is
        // materialized: a `Skip` verdict yields an empty inner iterator, so `self.page`
        // — the only materialization — never runs for a skipped page (the lazy hook is
        // preserved by construction).
        let page_count = u32::try_from(self.pages.len()).expect("page count fits u32");
        admission::candidate_pages(self.graph_index(), page_count, g).flat_map(move |page_id| {
            let index = usize::try_from(page_id.0).expect("page id fits usize");
            let slot = &self.pages[index];
            let admitted = match admission::admit_pattern(&slot.translation, s, p, o, g) {
                PageAdmission::Skip(_) => None,
                PageAdmission::Admit(local) => Some(local),
            };
            admitted.into_iter().flat_map(move |local| {
                let page = self
                    .page(slot.id)
                    .expect("sealed page must re-materialize deterministically");
                page.quads_for_pattern_indexed(local.s, local.p, local.o, local.g)
                    .map(move |q| map_quad_to_global(&slot.translation, q))
            })
        })
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
        // The Merge-scope summation: Σ over admitted pages of
        // `admission::estimate_admitted_page`'s minimum-axis count, read from that
        // page's sealed `PageSummary` alone — NO page is materialized, here or
        // transitively. The candidate page set narrows on the graph axis exactly as
        // in `quads_for_pattern`.
        //
        // This is a pure function of `(snapshot, pattern)`: it never depends on
        // whether a page happens to be resident. A residency-dependent estimate
        // would make plan choice — and therefore the `requested_pages` evidence
        // sequence a G-clause treats as proof of what a query actually touched —
        // depend on incidental cache state rather than on the snapshot and the
        // pattern alone, so two runs of the identical query against the identical
        // snapshot could pick different plans. For a pattern with exactly one bound
        // axis the per-page contribution is EXACT (see `estimate_admitted_page`),
        // not merely an upper bound.
        let page_count = u32::try_from(self.pages.len()).expect("page count fits u32");
        let mut total = 0usize;
        for page_id in admission::candidate_pages(self.graph_index(), page_count, g) {
            let index = usize::try_from(page_id.0).expect("page id fits usize");
            let slot = &self.pages[index];
            let PageAdmission::Admit(local) =
                admission::admit_pattern(&slot.translation, s, p, o, g)
            else {
                continue;
            };
            let estimate = admission::estimate_admitted_page(
                slot.translation.summary(),
                local,
                slot.quad_count,
            );
            total = total.saturating_add(estimate);
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
        // Stream the reifier side table page-by-page (same lazy composition as `quads`).
        self.pages.iter().flat_map(move |slot| {
            let page = self
                .page(slot.id)
                .expect("sealed page must re-materialize deterministically");
            page.reifier_quads()
                .map(move |q| map_quad_to_global(&slot.translation, q))
        })
    }

    fn reifier_quads_of(
        &self,
        reifier: GlobalTermId,
    ) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        // Per-page narrowing, exactly like `annotations_of_with_graph`: a page is
        // skipped BEFORE materialization unless its sealed `PageSummary` proves it
        // owns at least one REIFIER row for this term (`reifier_rows(local) > 0`) —
        // role-agnostic term-table presence (`to_local` alone) is not enough, because
        // a page can mention a term only in its base-quad table and never as a
        // reifier. A page that clears the check addresses its contiguous run in
        // `O(log n)` (`RdfDataset::reifier_quads_of`). Page order and within-page
        // frozen order are both unchanged, so this is row-for-row identical to the
        // trait default's filter over `reifier_quads`.
        self.pages.iter().flat_map(move |slot| {
            slot.translation
                .to_local(reifier)
                .filter(|&local| slot.translation.summary().reifier_rows(local) > 0)
                .into_iter()
                .flat_map(move |local_reifier| {
                    let page = self
                        .page(slot.id)
                        .expect("sealed page must re-materialize deterministically");
                    page.reifier_quads_of(local_reifier)
                        .map(move |q| map_quad_to_global(&slot.translation, q))
                })
        })
    }

    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        // Stream the annotation side table page-by-page.
        self.pages.iter().flat_map(move |slot| {
            let page = self
                .page(slot.id)
                .expect("sealed page must re-materialize deterministically");
            page.annotation_quads()
                .map(move |q| map_quad_to_global(&slot.translation, q))
        })
    }

    fn annotations_of_with_graph(
        &self,
        reifier: GlobalTermId,
    ) -> impl Iterator<Item = (GlobalTermId, GlobalTermId, Option<GlobalTermId>)> + '_ {
        // A page is skipped BEFORE materialization (empty inner iterator) unless its
        // sealed `PageSummary` proves it owns at least one ANNOTATION row for this
        // term (`annotation_rows(local) > 0`) — see `reifier_quads_of` above for why
        // mere `to_local` presence is not enough.
        self.pages.iter().flat_map(move |slot| {
            slot.translation
                .to_local(reifier)
                .filter(|&local| slot.translation.summary().annotation_rows(local) > 0)
                .into_iter()
                .flat_map(move |local_reifier| {
                    let page = self
                        .page(slot.id)
                        .expect("sealed page must re-materialize deterministically");
                    page.annotations_of_with_graph(local_reifier)
                        .map(move |(p, o, g)| {
                            (
                                slot.translation.to_global(p),
                                slot.translation.to_global(o),
                                g.map(|g| slot.translation.to_global(g)),
                            )
                        })
                })
        })
    }

    fn reifier_quads_in_graph(
        &self,
        g: GraphMatch<GlobalTermId>,
    ) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        // An override, not a new obligation (see the trait's doc comment): narrows the
        // candidate page set to the REIFIER stream's graph postings BEFORE any page is
        // materialized, exactly as `quads_for_pattern` narrows on the base-quad
        // postings. Soundness: a page absent from `g`'s reifier posting list has zero
        // reifier rows in that graph (per `GraphPageIndex::derive`), so it can
        // contribute nothing. A listed page may also hold reifier rows in OTHER
        // graphs, so the per-row `g.matches` filter still runs after materialization —
        // narrowing chooses pages, it does not replace the row predicate.
        let page_count = u32::try_from(self.pages.len()).expect("page count fits u32");
        admission::candidate_pages_for_stream(
            self.graph_index(),
            page_count,
            g,
            PageStream::Reifier,
        )
        .flat_map(move |page_id| {
            let index = usize::try_from(page_id.0).expect("page id fits usize");
            let slot = &self.pages[index];
            let page = self
                .page(slot.id)
                .expect("sealed page must re-materialize deterministically");
            page.reifier_quads()
                .map(move |q| map_quad_to_global(&slot.translation, q))
                .filter(move |q| g.matches(q.g))
        })
    }

    fn annotation_quads_in_graph(
        &self,
        g: GraphMatch<GlobalTermId>,
    ) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        // See `reifier_quads_in_graph` above: same narrowing, over the ANNOTATION
        // stream's graph postings instead.
        let page_count = u32::try_from(self.pages.len()).expect("page count fits u32");
        admission::candidate_pages_for_stream(
            self.graph_index(),
            page_count,
            g,
            PageStream::Annotation,
        )
        .flat_map(move |page_id| {
            let index = usize::try_from(page_id.0).expect("page id fits usize");
            let slot = &self.pages[index];
            let page = self
                .page(slot.id)
                .expect("sealed page must re-materialize deterministically");
            page.annotation_quads()
                .map(move |q| map_quad_to_global(&slot.translation, q))
                .filter(move |q| g.matches(q.g))
        })
    }

    fn named_graphs(&self) -> impl Iterator<Item = GlobalTermId> + '_ {
        // O(1) charge, no page materialized: `GraphPageIndex::keys` is already every
        // named graph any page knows about (declared-empty graphs included), ascending
        // by `GlobalTermId` and deduplicated, folded from each page's sealed
        // `PageSummary` alone.
        //
        // Order: ascending `GlobalTermId`, which is INTERN order — page-arrival order,
        // then within-page local order — not canonical `TermValue` order. The two
        // coincide only after `compact()` (clause G2). This matches what the trait
        // default already produced here (it collected the same ids into a
        // `BTreeSet<Self::Id>`), so this override changes MEMBERSHIP, not order.
        //
        // Membership is a deliberate fix, not a side effect: SPARQL 1.1 §8.3 and §18.6
        // range `GRAPH ?g` over every named graph in the active dataset, including ones
        // with no matching triples. The default derives graphs only from `quads()`, so
        // it misses a graph a page declares but leaves empty, or one named only by a
        // reifier or annotation side-table row. Each page's own
        // `RdfDataset::named_graphs()` already unions declared graphs with the graph
        // slots of quads, reifiers, and annotations; this override brings the composed
        // paged surface into line with that per-page answer, and with
        // `RdfDataset`/`CompositeDatasetView`.
        self.graph_index().keys().iter().copied()
    }
}
