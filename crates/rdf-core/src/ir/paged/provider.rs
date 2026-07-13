// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The demand-paging hook for the reference [`PagedDataset`](super::PagedDataset):
//! [`PageProvider`], the page identity [`PageId`], the materialization error
//! [`PageFault`], and two reference providers ([`InMemoryPageProvider`],
//! [`CountingDemandProvider`]).
//!
//! A [`PageProvider`] is the sole source of a page's frozen [`RdfDataset`]. It MUST
//! be **deterministic** — the same [`PageId`] materializes byte-identical quads on
//! every call — and `Send + Sync`, so a [`PagedDataset`](super::PagedDataset) can be
//! queried from any thread and its per-page id translation stays stable across
//! re-materialization. Nothing here touches `std::fs`/`thread`/`time`/rng: the two
//! reference providers keep their pages (or page thunks) in memory, so the layer is
//! `wasm32-unknown-unknown`-clean.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::ir::RdfDataset;

/// A dense page ordinal. Pages of a [`PagedDataset`](super::PagedDataset) are
/// numbered `0..page_count` and iterated in ascending `PageId` order, which is the
/// backbone of the paged view's determinism (page order, then in-page frozen order).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PageId(pub u32);

/// A page could not be materialized: the [`PageProvider`] failed to produce the
/// frozen [`RdfDataset`] for [`page`](PageFault::page). Carries a human-readable
/// [`message`](PageFault::message) for diagnostics.
#[derive(Debug)]
pub struct PageFault {
    /// The page whose materialization failed.
    pub page: PageId,
    /// A human-readable description of the failure.
    pub message: String,
}

impl std::fmt::Display for PageFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "page fault materializing page {}: {}",
            self.page.0, self.message
        )
    }
}

impl std::error::Error for PageFault {}

/// The demand-paging hook: the lazy materialization boundary between a
/// [`PagedDataset`](super::PagedDataset) and the frozen [`RdfDataset`] pages that
/// compose it.
///
/// # Contract
///
/// - **Deterministic.** [`materialize`](PageProvider::materialize) called twice with
///   the same [`PageId`] MUST yield a dataset with byte-identical quads (and term
///   table), so the per-page [`PageTranslation`](super::PageTranslation) built once at
///   seal time stays valid across every later re-materialization.
/// - **`Send + Sync`.** The provider is shared behind an `Arc` and read from the query
///   path; it must be thread-safe.
pub trait PageProvider: Send + Sync {
    /// The number of pages this provider can materialize. `PageId(0..page_count)` are
    /// exactly the valid page ordinals.
    fn page_count(&self) -> usize;

    /// Materialize one page into its frozen [`RdfDataset`], or report a
    /// [`PageFault`] (e.g. an out-of-range `page`). Must be deterministic per the
    /// trait contract.
    fn materialize(&self, page: PageId) -> Result<Arc<RdfDataset>, PageFault>;
}

/// The trivial reference provider: every page is already resident in memory, so
/// [`materialize`](PageProvider::materialize) is an infallible `Arc::clone` (an
/// out-of-range `PageId` is the only failure).
#[derive(Debug)]
pub struct InMemoryPageProvider {
    pages: Box<[Arc<RdfDataset>]>,
}

impl InMemoryPageProvider {
    /// Wrap an ordered collection of frozen pages. Page `i` is materialized by
    /// `PageId(i)`.
    #[must_use]
    pub fn new(pages: Vec<Arc<RdfDataset>>) -> Self {
        Self {
            pages: pages.into_boxed_slice(),
        }
    }
}

impl PageProvider for InMemoryPageProvider {
    fn page_count(&self) -> usize {
        self.pages.len()
    }

    fn materialize(&self, page: PageId) -> Result<Arc<RdfDataset>, PageFault> {
        let index = usize::try_from(page.0).expect("page id fits usize");
        self.pages
            .get(index)
            .map(Arc::clone)
            .ok_or_else(|| PageFault {
                page,
                message: format!("page index {index} out of range 0..{}", self.pages.len()),
            })
    }
}

/// A provider that exposes a SUBSET of another provider's pages under fresh, dense
/// [`PageId`]s — the page-eviction primitive behind
/// [`PagedDataset::with_pages`](super::PagedDataset::with_pages) /
/// [`drop_page`](super::PagedDataset::drop_page).
///
/// `indices[new_id]` is the ORIGINAL page id in `inner` that new dense page `new_id`
/// re-materializes, so the pruned dataset keeps dense `0..kept` page ordinals (the
/// [`PagedDataset`](super::PagedDataset) invariant) while its retained per-page
/// translations still address the ORIGINAL — now oversized — dictionary. Holds only
/// an `Arc` and a boxed slice, so it stays `Send + Sync` and `wasm32`-clean.
pub struct SubsetPageProvider {
    /// The backing provider whose pages are being subset.
    inner: Arc<dyn PageProvider>,
    /// `indices[new_id] = original PageId`; its length is the pruned page count.
    indices: Box<[PageId]>,
}

impl std::fmt::Debug for SubsetPageProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `dyn PageProvider` is not `Debug`; summarize by the retained id map.
        f.debug_struct("SubsetPageProvider")
            .field("indices", &self.indices)
            .finish_non_exhaustive()
    }
}

impl SubsetPageProvider {
    /// Expose exactly `indices` (each an original page id in `inner`) as dense pages
    /// `0..indices.len()`. Order is preserved: new page `i` materializes
    /// `inner`'s `indices[i]`.
    #[must_use]
    pub fn new(inner: Arc<dyn PageProvider>, indices: Vec<PageId>) -> Self {
        Self {
            inner,
            indices: indices.into_boxed_slice(),
        }
    }
}

impl PageProvider for SubsetPageProvider {
    fn page_count(&self) -> usize {
        self.indices.len()
    }

    fn materialize(&self, page: PageId) -> Result<Arc<RdfDataset>, PageFault> {
        let index = usize::try_from(page.0).expect("page id fits usize");
        let Some(&original) = self.indices.get(index) else {
            return Err(PageFault {
                page,
                message: format!(
                    "subset page index {index} out of range 0..{}",
                    self.indices.len()
                ),
            });
        };
        self.inner.materialize(original)
    }
}

/// A reference provider that OBSERVES demand: each [`materialize`](PageProvider::materialize)
/// call increments an atomic hit counter, so a test can assert exactly which pages
/// were pulled and when.
///
/// It holds a rebuild thunk per page (`Fn() -> Arc<RdfDataset>`), so a re-materialize
/// is a genuine counted rebuild rather than a cached `Arc::clone` — this is what lets
/// a test watch the lazy hook fire AT QUERY TIME (a
/// [`PagedDataset`](super::PagedDataset) drops the sealed page and re-materializes on
/// first query). The counter is an [`AtomicUsize`], which is `wasm32`-clean.
pub struct CountingDemandProvider {
    #[allow(clippy::type_complexity)]
    thunks: Box<[Box<dyn Fn() -> Arc<RdfDataset> + Send + Sync>]>,
    hits: AtomicUsize,
}

impl std::fmt::Debug for CountingDemandProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CountingDemandProvider")
            .field("page_count", &self.thunks.len())
            .field("hits", &self.hits.load(Ordering::Relaxed))
            .finish()
    }
}

impl CountingDemandProvider {
    /// Wrap one rebuild thunk per page. Thunk `i` is invoked (and the hit counter
    /// bumped) each time `PageId(i)` is materialized. Each thunk MUST be deterministic
    /// per the [`PageProvider`] contract.
    #[must_use]
    pub fn new(thunks: Vec<Box<dyn Fn() -> Arc<RdfDataset> + Send + Sync>>) -> Self {
        Self {
            thunks: thunks.into_boxed_slice(),
            hits: AtomicUsize::new(0),
        }
    }

    /// The number of [`materialize`](PageProvider::materialize) calls served so far.
    #[must_use]
    pub fn hits(&self) -> usize {
        self.hits.load(Ordering::Relaxed)
    }
}

impl PageProvider for CountingDemandProvider {
    fn page_count(&self) -> usize {
        self.thunks.len()
    }

    fn materialize(&self, page: PageId) -> Result<Arc<RdfDataset>, PageFault> {
        let index = usize::try_from(page.0).expect("page id fits usize");
        let thunk = self.thunks.get(index).ok_or_else(|| PageFault {
            page,
            message: format!("page index {index} out of range 0..{}", self.thunks.len()),
        })?;
        // Count the demand BEFORE rebuilding so a test observing `hits()` sees the
        // pull even if the rebuild itself is trivial.
        self.hits.fetch_add(1, Ordering::Relaxed);
        Ok(thunk())
    }
}
