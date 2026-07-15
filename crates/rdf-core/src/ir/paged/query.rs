// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Operation-scoped, fallible reads over a sealed [`PagedDataset`].
//!
//! [`PagedQueryView`] keeps its own page cache, exact resource accounting, and sticky
//! terminal error. Its [`DatasetView`] iterators stop at the first operational fault;
//! a query engine then samples [`FallibleDatasetView::operation_status`] before it can
//! publish any internally-computed rows as a complete result.

use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use crate::RdfStoreCapabilities;
use crate::dataset_view::{DatasetView, FallibleDatasetView, GraphMatch, ViewOperationStatus};
use crate::ir::{GlobalTermId, QuadIds, QuadRef, RdfDataset, TermId, TermValue};

use super::{
    PageFault, PageFaultKind, PageGeneration, PageId, PageMaterialization, PagedDataset,
    map_quad_to_global, translate_pattern,
};

/// Exact resource ceilings for one [`PagedQueryView`].
///
/// Limits are inclusive: a page is admitted when its addition leaves consumption
/// equal to the corresponding ceiling, and refused only when it would exceed it.
/// Cached re-reads of an already-admitted page consume neither another page nor more
/// bytes. Zero is a valid hard limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PagedQueryLimits {
    /// Maximum number of distinct pages the operation may admit.
    pub max_pages: u64,
    /// Maximum sum of provider-reported page byte charges the operation may admit.
    pub max_bytes: u64,
}

impl PagedQueryLimits {
    /// Construct explicit inclusive page and byte ceilings.
    #[must_use]
    pub const fn new(max_pages: u64, max_bytes: u64) -> Self {
        Self {
            max_pages,
            max_bytes,
        }
    }

    /// No practical resource ceiling. Provider failures, cancellation, deadlines,
    /// generation drift, and invalid data remain fully checked.
    pub const UNBOUNDED: Self = Self::new(u64::MAX, u64::MAX);
}

/// Deterministic evidence accumulated by one paged query operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PagedQueryEvidence {
    /// The immutable snapshot generation requested by the operation.
    pub generation: PageGeneration,
    /// Every first page request in evaluation order, including the request that
    /// encountered a fault or exceeded a budget.
    pub requested_pages: Vec<PageId>,
    /// Number of distinct pages successfully validated and admitted.
    pub consumed_pages: u64,
    /// Sum of the provider-reported byte charges of admitted pages.
    pub consumed_bytes: u64,
}

impl PagedQueryEvidence {
    fn new(generation: PageGeneration) -> Self {
        Self {
            generation,
            requested_pages: Vec::new(),
            consumed_pages: 0,
            consumed_bytes: 0,
        }
    }
}

/// The typed terminal error of a [`PagedQueryView`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PagedQueryError {
    /// The provider failed for an implementation-specific operational reason.
    Provider {
        /// The requested page.
        page: PageId,
        /// Provider diagnostic detail.
        message: String,
    },
    /// A page or the provider belongs to a different immutable snapshot.
    StaleGeneration {
        /// The requested page at which drift was observed, or `None` when a status
        /// checkpoint detected provider-wide drift before any page request.
        page: Option<PageId>,
        /// The operation's sealed generation.
        expected: PageGeneration,
        /// The provider/materialization generation observed.
        actual: PageGeneration,
    },
    /// Admitting another distinct page would exceed the page ceiling.
    PageBudgetExceeded {
        /// The page whose admission was refused.
        page: PageId,
        /// Inclusive page ceiling.
        limit: u64,
        /// Successfully admitted pages before the refused request.
        consumed: u64,
    },
    /// Admitting a page's charged bytes would exceed the byte ceiling.
    ByteBudgetExceeded {
        /// The page whose admission was refused.
        page: PageId,
        /// Inclusive byte ceiling.
        limit: u64,
        /// Successfully charged bytes before the refused request.
        consumed: u64,
        /// The sealed byte charge of the refused page.
        page_bytes: u64,
    },
    /// The host or caller cancelled the operation.
    Cancelled {
        /// The requested page.
        page: PageId,
        /// Provider diagnostic detail.
        message: String,
    },
    /// A host-owned deadline expired. PurRDF never reads a clock.
    DeadlineExceeded {
        /// The requested page.
        page: PageId,
        /// Provider diagnostic detail.
        message: String,
    },
    /// Materialized data or metadata violated the sealed page contract.
    InvalidData {
        /// The requested page.
        page: PageId,
        /// Validation diagnostic detail.
        message: String,
    },
}

impl From<PageFault> for PagedQueryError {
    fn from(fault: PageFault) -> Self {
        match fault.kind {
            PageFaultKind::Provider => Self::Provider {
                page: fault.page,
                message: fault.message,
            },
            PageFaultKind::StaleGeneration { expected, actual } => Self::StaleGeneration {
                page: Some(fault.page),
                expected,
                actual,
            },
            PageFaultKind::Cancelled => Self::Cancelled {
                page: fault.page,
                message: fault.message,
            },
            PageFaultKind::DeadlineExceeded => Self::DeadlineExceeded {
                page: fault.page,
                message: fault.message,
            },
            PageFaultKind::InvalidData => Self::InvalidData {
                page: fault.page,
                message: fault.message,
            },
        }
    }
}

impl std::fmt::Display for PagedQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Provider { page, message } => {
                write!(
                    f,
                    "provider failure materializing page {}: {message}",
                    page.0
                )
            }
            Self::StaleGeneration {
                page,
                expected,
                actual,
            } => match page {
                Some(page) => write!(
                    f,
                    "stale generation materializing page {}: expected {expected}, got {actual}",
                    page.0
                ),
                None => write!(
                    f,
                    "stale provider generation at operation checkpoint: expected {expected}, \
                     got {actual}"
                ),
            },
            Self::PageBudgetExceeded {
                page,
                limit,
                consumed,
            } => write!(
                f,
                "page budget exceeded requesting page {}: consumed {consumed}, limit {limit}",
                page.0
            ),
            Self::ByteBudgetExceeded {
                page,
                limit,
                consumed,
                page_bytes,
            } => write!(
                f,
                "byte budget exceeded requesting page {}: consumed {consumed}, page charge \
                 {page_bytes}, limit {limit}",
                page.0
            ),
            Self::Cancelled { page, message } => {
                write!(f, "cancelled materializing page {}: {message}", page.0)
            }
            Self::DeadlineExceeded { page, message } => write!(
                f,
                "deadline exceeded materializing page {}: {message}",
                page.0
            ),
            Self::InvalidData { page, message } => {
                write!(f, "invalid data materializing page {}: {message}", page.0)
            }
        }
    }
}

impl std::error::Error for PagedQueryError {}

#[derive(Debug)]
struct QueryPageCache {
    materialization: OnceLock<Result<Arc<RdfDataset>, PagedQueryError>>,
}

impl QueryPageCache {
    fn new() -> Self {
        Self {
            materialization: OnceLock::new(),
        }
    }
}

#[derive(Debug)]
struct QueryState {
    evidence: PagedQueryEvidence,
    error: Option<PagedQueryError>,
}

/// An operation-local, fallible [`DatasetView`] over a sealed [`PagedDataset`].
///
/// Construct a fresh view for each execution. Successfully admitted pages are cached
/// only for that operation. The first failure is sticky: no later iterator yields a
/// row, and [`FallibleDatasetView::operation_status`] continues to report the same
/// root cause and evidence.
pub struct PagedQueryView<'dataset> {
    dataset: &'dataset PagedDataset,
    limits: PagedQueryLimits,
    pages: Box<[QueryPageCache]>,
    state: Mutex<QueryState>,
}

impl std::fmt::Debug for PagedQueryView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PagedQueryView")
            .field("dataset", &self.dataset)
            .field("limits", &self.limits)
            .field("status", &self.operation_status())
            .finish_non_exhaustive()
    }
}

impl PagedDataset {
    /// Start a fresh operation-scoped fallible view with explicit resource limits.
    ///
    /// Use the returned view only through an execution boundary that checks
    /// [`FallibleDatasetView::operation_status`] before and after evaluation. Passing
    /// it to an ordinary `DatasetView`-only query entry point would discard its
    /// completeness signal. Construct a new view for every operation; caches,
    /// evidence, limits, and the first sticky error are operation-local.
    #[must_use]
    pub fn query_view(&self, limits: PagedQueryLimits) -> PagedQueryView<'_> {
        PagedQueryView::new(self, limits)
    }
}

impl<'dataset> PagedQueryView<'dataset> {
    /// Start a fresh operation over `dataset` with explicit resource limits.
    #[must_use]
    pub fn new(dataset: &'dataset PagedDataset, limits: PagedQueryLimits) -> Self {
        let pages = (0..dataset.pages.len())
            .map(|_| QueryPageCache::new())
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self {
            dataset,
            limits,
            pages,
            state: Mutex::new(QueryState {
                evidence: PagedQueryEvidence::new(dataset.generation),
                error: None,
            }),
        }
    }

    /// The immutable resource ceilings for this operation.
    #[must_use]
    pub const fn limits(&self) -> PagedQueryLimits {
        self.limits
    }

    fn state(&self) -> MutexGuard<'_, QueryState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn failed(&self) -> bool {
        self.state().error.is_some()
    }

    fn page(&self, id: PageId) -> Option<&Arc<RdfDataset>> {
        // Once any page fails, no cached page may leak further rows from internal
        // partial evaluation.
        if self.failed() {
            return None;
        }
        let index = usize::try_from(id.0).expect("page id fits usize");
        let cache = self.pages.get(index)?;
        cache
            .materialization
            .get_or_init(|| self.materialize_page(id))
            .as_ref()
            .ok()
    }

    fn materialize_page(&self, id: PageId) -> Result<Arc<RdfDataset>, PagedQueryError> {
        let index = usize::try_from(id.0).expect("page id fits usize");
        let slot = &self.dataset.pages[index];
        // Hold the operation lock across this first materialization. The public query
        // boundary forces sequential evaluation; the lock additionally makes a
        // misused concurrent view admit each page and update evidence atomically.
        let mut state = self.state();
        if let Some(error) = &state.error {
            return Err(error.clone());
        }
        state.evidence.requested_pages.push(id);

        let provider_generation = self.dataset.provider.generation();
        if provider_generation != self.dataset.generation {
            return fail(
                &mut state,
                PagedQueryError::StaleGeneration {
                    page: Some(id),
                    expected: self.dataset.generation,
                    actual: provider_generation,
                },
            );
        }
        if state.evidence.consumed_pages >= self.limits.max_pages {
            let consumed = state.evidence.consumed_pages;
            return fail(
                &mut state,
                PagedQueryError::PageBudgetExceeded {
                    page: id,
                    limit: self.limits.max_pages,
                    consumed,
                },
            );
        }
        let Some(next_bytes) = state.evidence.consumed_bytes.checked_add(slot.byte_len) else {
            let consumed = state.evidence.consumed_bytes;
            return fail(
                &mut state,
                PagedQueryError::ByteBudgetExceeded {
                    page: id,
                    limit: self.limits.max_bytes,
                    consumed,
                    page_bytes: slot.byte_len,
                },
            );
        };
        if next_bytes > self.limits.max_bytes {
            let consumed = state.evidence.consumed_bytes;
            return fail(
                &mut state,
                PagedQueryError::ByteBudgetExceeded {
                    page: id,
                    limit: self.limits.max_bytes,
                    consumed,
                    page_bytes: slot.byte_len,
                },
            );
        }

        let materialization = match self.dataset.provider.materialize(id) {
            Ok(materialization) => materialization,
            Err(fault) => return fail(&mut state, fault.into()),
        };
        if let Err(error) = self.validate_materialization(id, &materialization) {
            return fail(&mut state, error);
        }
        let current_generation = self.dataset.provider.generation();
        if current_generation != self.dataset.generation {
            return fail(
                &mut state,
                PagedQueryError::StaleGeneration {
                    page: Some(id),
                    expected: self.dataset.generation,
                    actual: current_generation,
                },
            );
        }

        state.evidence.consumed_pages += 1;
        state.evidence.consumed_bytes = next_bytes;
        drop(state);
        Ok(materialization.dataset)
    }

    fn validate_materialization(
        &self,
        id: PageId,
        materialization: &PageMaterialization,
    ) -> Result<(), PagedQueryError> {
        let index = usize::try_from(id.0).expect("page id fits usize");
        let slot = &self.dataset.pages[index];
        if materialization.generation != self.dataset.generation {
            return Err(PagedQueryError::StaleGeneration {
                page: Some(id),
                expected: self.dataset.generation,
                actual: materialization.generation,
            });
        }
        if materialization.byte_len != slot.byte_len {
            return Err(PagedQueryError::InvalidData {
                page: id,
                message: format!(
                    "byte charge changed from sealed {} to materialized {}",
                    slot.byte_len, materialization.byte_len
                ),
            });
        }
        if materialization.dataset.term_count() != slot.translation.term_count() {
            return Err(PagedQueryError::InvalidData {
                page: id,
                message: format!(
                    "term count changed from sealed {} to materialized {}",
                    slot.translation.term_count(),
                    materialization.dataset.term_count()
                ),
            });
        }
        if materialization.dataset.quad_count() != slot.quad_count {
            return Err(PagedQueryError::InvalidData {
                page: id,
                message: format!(
                    "quad count changed from sealed {} to materialized {}",
                    slot.quad_count,
                    materialization.dataset.quad_count()
                ),
            });
        }
        if materialization.dataset.capabilities() != slot.caps {
            return Err(PagedQueryError::InvalidData {
                page: id,
                message: "page capabilities changed after sealing".to_owned(),
            });
        }
        // The translation is built over the page-local term table. Equal counts are
        // insufficient: reordered or corrupted values would silently remap quads to
        // the wrong global ids. Compare each value before admitting any row.
        for local_index in 0..slot.translation.term_count() {
            let local =
                TermId::from_index(u32::try_from(local_index).expect("page term index fits u32"));
            let global = slot.translation.to_global(local);
            if materialization.dataset.term_value(local)
                != self.dataset.dictionary.term_value(global)
            {
                return Err(PagedQueryError::InvalidData {
                    page: id,
                    message: format!("term value changed at local index {local_index}"),
                });
            }
        }
        Ok(())
    }
}

fn fail<T>(state: &mut QueryState, error: PagedQueryError) -> Result<T, PagedQueryError> {
    state.error = Some(error.clone());
    Err(error)
}

impl FallibleDatasetView for PagedQueryView<'_> {
    type Error = PagedQueryError;
    type Evidence = PagedQueryEvidence;

    fn operation_status(&self) -> ViewOperationStatus<Self::Error, Self::Evidence> {
        let mut state = self.state();
        // A query may require no page at all (for example, a constants-only algebra
        // expression). Check the provider generation at both engine checkpoints so
        // such an operation cannot certify a stale snapshot merely because no lazy
        // materialization occurred.
        if state.error.is_none() {
            let actual = self.dataset.provider.generation();
            if actual != self.dataset.generation {
                state.error = Some(PagedQueryError::StaleGeneration {
                    page: None,
                    expected: self.dataset.generation,
                    actual,
                });
            }
        }
        match &state.error {
            None => ViewOperationStatus::Ready {
                evidence: state.evidence.clone(),
            },
            Some(error) => ViewOperationStatus::Failed {
                error: error.clone(),
                evidence: state.evidence.clone(),
            },
        }
    }
}

impl DatasetView for PagedQueryView<'_> {
    type Id = GlobalTermId;
    type ProbePlan = ();

    fn quads(&self) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        self.dataset.pages.iter().flat_map(move |slot| {
            self.page(slot.id).into_iter().flat_map(move |page| {
                page.quads()
                    .map(move |quad| map_quad_to_global(&slot.translation, quad))
            })
        })
    }

    fn quad_refs(&self) -> impl Iterator<Item = QuadRef<'_, GlobalTermId>> + '_ {
        self.quads().map(move |quad| QuadRef {
            s: self.dataset.dictionary.resolve(quad.s),
            p: self.dataset.dictionary.resolve(quad.p),
            o: self.dataset.dictionary.resolve(quad.o),
            g: quad.g.map(|graph| self.dataset.dictionary.resolve(graph)),
        })
    }

    fn resolve(&self, id: GlobalTermId) -> crate::ir::TermRef<'_, GlobalTermId> {
        self.dataset.dictionary.resolve(id)
    }

    fn quads_for_pattern(
        &self,
        s: Option<GlobalTermId>,
        p: Option<GlobalTermId>,
        o: Option<GlobalTermId>,
        g: GraphMatch<GlobalTermId>,
    ) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        self.dataset.pages.iter().flat_map(move |slot| {
            translate_pattern(&slot.translation, s, p, o, g)
                .into_iter()
                .flat_map(move |(local_s, local_p, local_o, local_g)| {
                    self.page(slot.id).into_iter().flat_map(move |page| {
                        page.quads_for_pattern_indexed(local_s, local_p, local_o, local_g)
                            .map(move |quad| map_quad_to_global(&slot.translation, quad))
                    })
                })
        })
    }

    fn term_id_by_value(&self, value: &TermValue) -> Option<GlobalTermId> {
        self.dataset.dictionary.term_id_by_value(value)
    }

    fn capabilities(&self) -> RdfStoreCapabilities {
        self.dataset.caps
    }

    fn len_hint(&self) -> Option<usize> {
        Some(self.dataset.total_quads)
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
        self.quads_for_pattern(s, p, o, g)
    }

    fn cardinality_estimate(
        &self,
        s: Option<GlobalTermId>,
        p: Option<GlobalTermId>,
        o: Option<GlobalTermId>,
        g: GraphMatch<GlobalTermId>,
    ) -> usize {
        let mut total = 0_usize;
        for slot in &self.dataset.pages {
            let Some((local_s, local_p, local_o, local_g)) =
                translate_pattern(&slot.translation, s, p, o, g)
            else {
                continue;
            };
            let index = usize::try_from(slot.id.0).expect("page id fits usize");
            let estimate = self.pages[index]
                .materialization
                .get()
                .and_then(|result| result.as_ref().ok())
                .map_or(slot.quad_count, |page| {
                    page.cardinality_estimate(local_s, local_p, local_o, local_g)
                });
            // Planning must not materialize provider pages: doing so would consume
            // operation budgets and make requested-page evidence depend on whether
            // the evaluator's BGP-order cache is warm. The sealed quad count is a
            // valid upper bound until this operation has already admitted the page.
            total = total.saturating_add(estimate);
        }
        total
    }

    fn term_count(&self) -> usize {
        self.dataset.dictionary.len()
    }

    fn stats_fingerprint(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.dataset.total_quads.hash(&mut hasher);
        self.dataset.dictionary.len().hash(&mut hasher);
        hasher.finish()
    }

    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        self.dataset.pages.iter().flat_map(move |slot| {
            self.page(slot.id).into_iter().flat_map(move |page| {
                page.reifier_quads()
                    .map(move |quad| map_quad_to_global(&slot.translation, quad))
            })
        })
    }

    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        self.dataset.pages.iter().flat_map(move |slot| {
            self.page(slot.id).into_iter().flat_map(move |page| {
                page.annotation_quads()
                    .map(move |quad| map_quad_to_global(&slot.translation, quad))
            })
        })
    }

    fn annotations_of_with_graph(
        &self,
        reifier: GlobalTermId,
    ) -> impl Iterator<Item = (GlobalTermId, GlobalTermId, Option<GlobalTermId>)> + '_ {
        self.dataset.pages.iter().flat_map(move |slot| {
            slot.translation
                .to_local(reifier)
                .into_iter()
                .flat_map(move |local_reifier| {
                    self.page(slot.id).into_iter().flat_map(move |page| {
                        page.annotations_of_with_graph(local_reifier).map(
                            move |(predicate, object, graph)| {
                                (
                                    slot.translation.to_global(predicate),
                                    slot.translation.to_global(object),
                                    graph.map(|id| slot.translation.to_global(id)),
                                )
                            },
                        )
                    })
                })
        })
    }
}
