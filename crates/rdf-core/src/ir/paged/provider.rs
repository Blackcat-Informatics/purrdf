// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Demand-paging contracts for [`PagedDataset`](super::PagedDataset).
//!
//! A provider identifies one immutable snapshot with [`PageGeneration`] and returns
//! each page's dataset, generation, and byte charge together in a single
//! [`PageMaterialization`]. Keeping those facts atomic is what lets a query refuse a
//! generation change or enforce an exact byte budget without treating an unavailable
//! page as an absent RDF fact.
//!
//! Nothing here touches `std::fs`, threads, clocks, or randomness. Durable storage,
//! cancellation sources, and deadlines belong to provider implementations outside
//! PurRDF; this module only carries their typed outcomes and remains
//! `wasm32-unknown-unknown` compatible.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::ir::{RdfDataset, TermId, TermRef};

/// A dense page ordinal. Pages of a [`PagedDataset`](super::PagedDataset) are
/// numbered `0..page_count` and iterated in ascending [`PageId`] order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PageId(pub u32);

/// The immutable provider snapshot to which page translations and byte metadata
/// belong.
///
/// A provider must change this value whenever any page's RDF value, local term-id
/// layout, or charged byte length changes. It need not be consecutive; equality is
/// the only semantic operation PurRDF performs.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct PageGeneration(pub u64);

impl PageGeneration {
    /// The generation used by the in-memory reference providers unless a caller
    /// supplies another value.
    pub const INITIAL: Self = Self(0);
}

impl std::fmt::Display for PageGeneration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// The typed reason a provider could not produce a valid page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageFaultKind {
    /// The provider failed to materialize the page for an implementation-specific
    /// reason such as I/O, authentication, or a missing object.
    Provider,
    /// The materialized page belongs to a different immutable snapshot.
    StaleGeneration {
        /// The generation captured for the paged dataset or query operation.
        expected: PageGeneration,
        /// The generation returned by the provider.
        actual: PageGeneration,
    },
    /// The caller or host cancelled the operation.
    Cancelled,
    /// A host-owned deadline expired. PurRDF itself never reads a clock.
    DeadlineExceeded,
    /// The provider returned data or metadata that violates the sealed page
    /// contract.
    InvalidData,
}

impl PageFaultKind {
    /// A stable diagnostic label for this fault category.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Provider => "provider failure",
            Self::StaleGeneration { .. } => "stale generation",
            Self::Cancelled => "cancelled",
            Self::DeadlineExceeded => "deadline exceeded",
            Self::InvalidData => "invalid page data",
        }
    }
}

/// A page could not be materialized as part of the requested snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageFault {
    /// The logical page whose materialization failed.
    pub page: PageId,
    /// The typed failure category.
    pub kind: PageFaultKind,
    /// Provider-supplied or contract-supplied diagnostic detail.
    pub message: String,
}

impl PageFault {
    /// Construct a provider/materialization failure.
    pub fn provider(page: PageId, message: impl Into<String>) -> Self {
        Self {
            page,
            kind: PageFaultKind::Provider,
            message: message.into(),
        }
    }

    /// Construct a snapshot-generation mismatch.
    #[must_use]
    pub fn stale_generation(
        page: PageId,
        expected: PageGeneration,
        actual: PageGeneration,
    ) -> Self {
        Self {
            page,
            kind: PageFaultKind::StaleGeneration { expected, actual },
            message: format!(
                "page belongs to generation {actual}, but generation {expected} was requested"
            ),
        }
    }

    /// Construct a host-reported cancellation.
    pub fn cancelled(page: PageId, message: impl Into<String>) -> Self {
        Self {
            page,
            kind: PageFaultKind::Cancelled,
            message: message.into(),
        }
    }

    /// Construct a host-reported deadline failure.
    pub fn deadline_exceeded(page: PageId, message: impl Into<String>) -> Self {
        Self {
            page,
            kind: PageFaultKind::DeadlineExceeded,
            message: message.into(),
        }
    }

    /// Construct an invalid/corrupt page-data failure.
    pub fn invalid_data(page: PageId, message: impl Into<String>) -> Self {
        Self {
            page,
            kind: PageFaultKind::InvalidData,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for PageFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} materializing page {}: {}",
            self.kind.label(),
            self.page.0,
            self.message
        )
    }
}

impl std::error::Error for PageFault {}

/// One atomic page-materialization result.
///
/// `generation` certifies which immutable snapshot produced `dataset`; `byte_len`
/// is the deterministic charge applied when a query first admits this page. A
/// durable provider should report the actual bytes it had to materialize. Reference
/// in-memory providers use a stable, platform-independent logical RDF size.
#[derive(Debug, Clone)]
pub struct PageMaterialization {
    /// The frozen page dataset.
    pub dataset: Arc<RdfDataset>,
    /// The immutable snapshot that produced the page.
    pub generation: PageGeneration,
    /// The provider-defined deterministic byte charge for this page.
    pub byte_len: u64,
}

impl PageMaterialization {
    /// Construct a materialization with provider-reported metadata.
    #[must_use]
    pub const fn new(dataset: Arc<RdfDataset>, generation: PageGeneration, byte_len: u64) -> Self {
        Self {
            dataset,
            generation,
            byte_len,
        }
    }

    /// Construct the deterministic reference charge used for an in-memory page.
    #[must_use]
    pub fn in_memory(dataset: Arc<RdfDataset>, generation: PageGeneration) -> Self {
        let byte_len = logical_rdf_byte_len(&dataset);
        Self::new(dataset, generation, byte_len)
    }
}

/// The demand-paging boundary between a [`PagedDataset`](super::PagedDataset) and
/// its frozen [`RdfDataset`] pages.
///
/// # Contract
///
/// - [`generation`](PageProvider::generation) identifies the provider's current
///   immutable snapshot. It changes whenever page content, local term ids, or byte
///   charges change.
/// - [`materialize`](PageProvider::materialize) returns dataset, generation, and byte
///   charge atomically. Repeated calls for the same page and generation must return
///   RDF-value-identical datasets with identical local term layout and byte charge.
/// - The trait is `Send + Sync`; callers may share a provider across query operations.
///
/// A provider may succeed during sealing and fail on a later materialization. Query
/// such a provider through [`PagedDataset::query_view`](super::PagedDataset::query_view)
/// and a [`FallibleDatasetView`](crate::FallibleDatasetView)-aware execution boundary.
/// Direct `DatasetView` use of `PagedDataset` is reserved for providers that guarantee
/// infallible re-materialization after sealing.
pub trait PageProvider: Send + Sync {
    /// The number of dense pages in the current snapshot.
    fn page_count(&self) -> usize;

    /// The provider's current immutable snapshot generation.
    fn generation(&self) -> PageGeneration;

    /// Materialize one page or return a typed operational failure.
    fn materialize(&self, page: PageId) -> Result<PageMaterialization, PageFault>;
}

/// The trivial reference provider: every page is already resident in memory.
#[derive(Debug)]
pub struct InMemoryPageProvider {
    generation: PageGeneration,
    pages: Box<[PageMaterialization]>,
}

impl InMemoryPageProvider {
    /// Wrap frozen pages at [`PageGeneration::INITIAL`] using the deterministic
    /// reference byte charge.
    #[must_use]
    pub fn new(pages: Vec<Arc<RdfDataset>>) -> Self {
        Self::with_generation(pages, PageGeneration::INITIAL)
    }

    /// Wrap frozen pages at an explicit generation using the deterministic reference
    /// byte charge.
    #[must_use]
    pub fn with_generation(pages: Vec<Arc<RdfDataset>>, generation: PageGeneration) -> Self {
        let pages = pages
            .into_iter()
            .map(|dataset| PageMaterialization::in_memory(dataset, generation))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self { generation, pages }
    }

    /// Wrap frozen pages with explicit deterministic byte charges.
    ///
    /// This is useful for tests and for in-memory mirrors that must preserve the
    /// accounting of an external store. The tuple order defines dense page ids.
    #[must_use]
    pub fn with_byte_lengths(
        pages: Vec<(Arc<RdfDataset>, u64)>,
        generation: PageGeneration,
    ) -> Self {
        let pages = pages
            .into_iter()
            .map(|(dataset, byte_len)| PageMaterialization::new(dataset, generation, byte_len))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self { generation, pages }
    }
}

impl PageProvider for InMemoryPageProvider {
    fn page_count(&self) -> usize {
        self.pages.len()
    }

    fn generation(&self) -> PageGeneration {
        self.generation
    }

    fn materialize(&self, page: PageId) -> Result<PageMaterialization, PageFault> {
        let index = usize::try_from(page.0).expect("page id fits usize");
        self.pages.get(index).cloned().ok_or_else(|| {
            PageFault::provider(
                page,
                format!("page index {index} out of range 0..{}", self.pages.len()),
            )
        })
    }
}

/// A provider exposing a subset of another provider's pages under fresh dense ids.
pub struct SubsetPageProvider {
    inner: Arc<dyn PageProvider>,
    indices: Box<[PageId]>,
}

impl std::fmt::Debug for SubsetPageProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubsetPageProvider")
            .field("indices", &self.indices)
            .finish_non_exhaustive()
    }
}

impl SubsetPageProvider {
    /// Expose `indices` from `inner` as pages `0..indices.len()` in the given order.
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

    fn generation(&self) -> PageGeneration {
        self.inner.generation()
    }

    fn materialize(&self, page: PageId) -> Result<PageMaterialization, PageFault> {
        let index = usize::try_from(page.0).expect("page id fits usize");
        let Some(&original) = self.indices.get(index) else {
            return Err(PageFault::provider(
                page,
                format!(
                    "subset page index {index} out of range 0..{}",
                    self.indices.len()
                ),
            ));
        };
        self.inner.materialize(original).map_err(|mut fault| {
            // The composed dataset exposes dense subset ids, so diagnostics must name
            // the page requested by its caller rather than leak the backing ordinal.
            fault.page = page;
            fault
        })
    }
}

/// A reference provider that observes demand by rebuilding each requested page and
/// incrementing an atomic hit counter.
pub struct CountingDemandProvider {
    #[allow(clippy::type_complexity)]
    thunks: Box<[Box<dyn Fn() -> Arc<RdfDataset> + Send + Sync>]>,
    generation: PageGeneration,
    hits: AtomicUsize,
}

impl std::fmt::Debug for CountingDemandProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CountingDemandProvider")
            .field("page_count", &self.thunks.len())
            .field("generation", &self.generation)
            .field("hits", &self.hits.load(Ordering::Relaxed))
            .finish()
    }
}

impl CountingDemandProvider {
    /// Wrap one deterministic rebuild thunk per page at the initial generation.
    #[must_use]
    pub fn new(thunks: Vec<Box<dyn Fn() -> Arc<RdfDataset> + Send + Sync>>) -> Self {
        Self::with_generation(thunks, PageGeneration::INITIAL)
    }

    /// Wrap one deterministic rebuild thunk per page at an explicit generation.
    #[must_use]
    pub fn with_generation(
        thunks: Vec<Box<dyn Fn() -> Arc<RdfDataset> + Send + Sync>>,
        generation: PageGeneration,
    ) -> Self {
        Self {
            thunks: thunks.into_boxed_slice(),
            generation,
            hits: AtomicUsize::new(0),
        }
    }

    /// The number of materialization calls served so far.
    #[must_use]
    pub fn hits(&self) -> usize {
        self.hits.load(Ordering::Relaxed)
    }
}

impl PageProvider for CountingDemandProvider {
    fn page_count(&self) -> usize {
        self.thunks.len()
    }

    fn generation(&self) -> PageGeneration {
        self.generation
    }

    fn materialize(&self, page: PageId) -> Result<PageMaterialization, PageFault> {
        let index = usize::try_from(page.0).expect("page id fits usize");
        let thunk = self.thunks.get(index).ok_or_else(|| {
            PageFault::provider(
                page,
                format!("page index {index} out of range 0..{}", self.thunks.len()),
            )
        })?;
        self.hits.fetch_add(1, Ordering::Relaxed);
        Ok(PageMaterialization::in_memory(thunk(), self.generation))
    }
}

/// A platform-independent logical size for reference in-memory accounting.
///
/// The charge covers every term's value bytes plus fixed-width structural ids, base
/// quads, reifier rows, and annotation rows. It deliberately excludes Rust object
/// layout and lazy indexes, whose sizes differ by target and query history.
fn logical_rdf_byte_len(dataset: &RdfDataset) -> u64 {
    let mut total = 0_u64;
    for index in 0..dataset.term_count() {
        let id = TermId::from_index(u32::try_from(index).expect("term count fits u32"));
        let term_len = match dataset.resolve(id) {
            TermRef::Iri(iri) => 1_u64 + len_u64(iri.len()),
            TermRef::Blank { label, .. } => 1_u64 + len_u64(label.len()) + 4,
            TermRef::Literal {
                lexical,
                language,
                direction,
                ..
            } => {
                1_u64
                    + len_u64(lexical.len())
                    + 4
                    + 1
                    + language.map_or(0, |tag| len_u64(tag.len()))
                    + 1
                    + u64::from(direction.is_some())
            }
            TermRef::Triple { .. } => 1_u64 + 12,
        };
        total = total
            .checked_add(term_len)
            .expect("logical page size fits u64");
    }

    let row_bytes = len_u64(dataset.quad_count())
        .checked_mul(16)
        .and_then(|bytes| {
            bytes.checked_add(
                len_u64(dataset.reifier_quads().count())
                    .checked_mul(12)
                    .expect("reifier table size fits u64"),
            )
        })
        .and_then(|bytes| {
            bytes.checked_add(
                len_u64(dataset.annotation_quads().count())
                    .checked_mul(16)
                    .expect("annotation table size fits u64"),
            )
        })
        .expect("logical page size fits u64");
    total
        .checked_add(row_bytes)
        .expect("logical page size fits u64")
}

fn len_u64(value: usize) -> u64 {
    u64::try_from(value).expect("logical page size fits u64")
}
