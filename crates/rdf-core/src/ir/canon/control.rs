// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Caller-selected search policy and measured, per-run execution evidence.

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{CANON_PROFILE_VERSION, CanonHash, CanonPresentation};

/// A logical search ceiling, selected explicitly by the caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchPolicy {
    /// Admit at most this many charged search events in each canonicalization run.
    Finite(u64),
    /// Impose no logical search ceiling. Counter representability and stop still apply.
    Unlimited,
}

/// The source selection whose canonical form a run computes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanonSelection {
    /// The complete dataset statement layer.
    Dataset,
    /// One named graph, selected by its exact IRI, emitted without its graph slot.
    NamedGraph(String),
}

/// The actual output operation, independent of the blank-label hash algorithm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanonOutput {
    /// A canonical document and label table.
    Document,
    /// A native dataset rewrite.
    RelabeledDataset,
    /// A native dataset rewrite and its source-term mapping.
    RelabeledMapping,
    /// SHA-256 over the canonical document.
    Sha256Digest,
    /// BLAKE3-256 over the canonical document.
    Blake3Digest,
    /// The canonical document, label table and its SHA-256 identity together.
    DocumentSha256Digest,
}

/// Exact algorithm coordinates, separate from the canonical content identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchConfiguration {
    /// The RDF 1.2 canonicalization profile.
    Rdfc {
        /// The profile's refusal and algorithm version.
        profile_version: u32,
        /// The blank-label hashing algorithm.
        hash: CanonHash,
        /// The selected canonical spelling.
        presentation: CanonPresentation,
        /// The selected statement layer.
        selection: CanonSelection,
        /// The output operation performed within the measured run.
        output: CanonOutput,
    },
    /// Complete typed dataset state, with unchanged version-one content preimages.
    DatasetStateV1,
    /// Structural comparison preparation, including cancellable exact counts.
    ComparisonPreparation,
}

impl SearchConfiguration {
    pub(super) fn rdfc(
        hash: CanonHash,
        presentation: CanonPresentation,
        graph: Option<&str>,
        output: CanonOutput,
    ) -> Self {
        Self::Rdfc {
            profile_version: CANON_PROFILE_VERSION,
            hash,
            presentation,
            selection: graph.map_or(CanonSelection::Dataset, |iri| {
                CanonSelection::NamedGraph(iri.to_owned())
            }),
            output,
        }
    }
}

/// The phase at completion or refusal. A stopped phase is never claimed completed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanonPhase {
    /// Source admission and component capture.
    Preparation,
    /// Canonical label or complete-state search.
    Search,
    /// Canonical output construction, relabeling or digest computation.
    Output,
}

/// A charged event. RDFC and dataset-state use distinct version-one charging laws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchEvent {
    /// RDFC n-degree frame entry.
    RdfcFrame,
    /// RDFC permutation attempt, including a subsequently pruned permutation.
    RdfcPermutation,
    /// Dataset-state search node entry.
    StateNode,
    /// Dataset-state candidate consideration.
    StateCandidate,
    /// Dataset-state explicit automorphism test.
    StateAutomorphism,
}

impl SearchEvent {
    const fn index(self) -> usize {
        match self {
            Self::RdfcFrame => 0,
            Self::RdfcPermutation => 1,
            Self::StateNode => 2,
            Self::StateCandidate => 3,
            Self::StateAutomorphism => 4,
        }
    }
}

/// Actual admitted search events, including work interrupted after its charge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SearchWork {
    total: u64,
    events: [u64; 5],
}

impl SearchWork {
    /// Total charged events. This is not a count of completed events or CPU time.
    #[must_use]
    pub const fn total(self) -> u64 {
        self.total
    }

    /// Charged events of the stated kind.
    #[must_use]
    pub const fn count(self, event: SearchEvent) -> u64 {
        self.events[event.index()]
    }
}

/// An observed search-control refusal, before admitting the next event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchRefusal {
    /// The explicitly selected finite ceiling was reached.
    Limit,
    /// The caller's latching signal was observed. Its underlying cause is not inferred.
    Stopped,
    /// Another exact event count cannot be represented.
    CounterCapacity,
}

/// Immutable execution evidence finalized by the actual engine on every return.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchReceipt {
    /// Exact algorithm and selection coordinates.
    pub configuration: SearchConfiguration,
    /// The caller's effective per-run policy.
    pub policy: SearchPolicy,
    /// Version of this algorithm's charging law.
    pub charging_version: u32,
    /// Actual charged work; rejected events are excluded.
    pub work: SearchWork,
    /// The final phase reached.
    pub phase: CanonPhase,
    /// Whether the complete operation, including source checkpoints, succeeded.
    pub completed: bool,
    /// A control refusal observed inside this run, if any. A source failure may
    /// take precedence in the returned native error without erasing this observation.
    pub observed_refusal: Option<SearchRefusal>,
}

/// A position in one caller-owned control's ordered receipt list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchPosition(usize);

/// Explicit caller policy, stop signal and ordered per-run execution evidence.
///
/// Each engine invocation gets a fresh ceiling. Composite operations retain all
/// constituent receipts and have no aggregate search ceiling. Native callers own
/// this evidence and may choose to discard it. Receipt storage does not yet claim
/// whole-operation physical memory admission.
#[derive(Debug)]
pub struct CanonicalizationControl<'a> {
    policy: SearchPolicy,
    stop: Option<&'a dyn crate::StopSignal>,
    receipts: Vec<SearchReceipt>,
}

impl<'a> CanonicalizationControl<'a> {
    /// Select the policy and optional caller-owned stop signal; there is no default.
    #[must_use]
    pub const fn new(policy: SearchPolicy, stop: Option<&'a dyn crate::StopSignal>) -> Self {
        Self {
            policy,
            stop,
            receipts: Vec::new(),
        }
    }

    /// Borrow all finalized receipts from this control.
    #[must_use]
    pub fn receipts(&self) -> &[SearchReceipt] {
        &self.receipts
    }

    /// Consume this control and transfer its finalized execution evidence.
    #[must_use]
    pub fn into_receipts(self) -> Vec<SearchReceipt> {
        self.receipts
    }

    /// Start an independent child with the same explicit policy and stop signal.
    /// Its per-run ceilings remain independent; it initially has no receipts.
    #[must_use]
    pub fn fork(&self) -> Self {
        Self::new(self.policy, self.stop)
    }

    /// Append only finalized evidence owned by an actual child control. Composite
    /// callers choose their deterministic join order; no arbitrary receipt can be
    /// inserted through this API and no aggregate ceiling is introduced.
    pub fn append_completed(&mut self, mut child: Self) {
        self.receipts.append(&mut child.receipts);
    }

    /// Mark the start of a composite operation's own receipt range.
    #[must_use]
    pub fn position(&self) -> SearchPosition {
        SearchPosition(self.receipts.len())
    }

    /// Borrow this operation's finalized suffix. A position beyond the current
    /// list is invalid; callers must use a position obtained from this control.
    #[must_use]
    pub fn since(&self, position: SearchPosition) -> Option<&[SearchReceipt]> {
        self.receipts.get(position.0..)
    }

    pub(super) fn run<T, E>(
        &mut self,
        configuration: SearchConfiguration,
        operation: impl FnOnce(&SearchRun<'_>) -> Result<T, E>,
    ) -> Result<T, E> {
        let run = SearchRun::new(self.policy, self.stop);
        let result = operation(&run);
        self.receipts.push(SearchReceipt {
            configuration,
            policy: self.policy,
            charging_version: 1,
            work: run.work.get(),
            phase: run.phase.get(),
            completed: result.is_ok(),
            observed_refusal: run.refusal.get().or_else(|| {
                run.stop
                    .observed
                    .load(Ordering::Relaxed)
                    .then_some(SearchRefusal::Stopped)
            }),
        });
        result
    }
}

pub(super) struct SearchRun<'a> {
    policy: SearchPolicy,
    stop: ObservedStop<'a>,
    work: Cell<SearchWork>,
    phase: Cell<CanonPhase>,
    refusal: Cell<Option<SearchRefusal>>,
}

impl<'a> SearchRun<'a> {
    pub(super) const fn new(policy: SearchPolicy, stop: Option<&'a dyn crate::StopSignal>) -> Self {
        Self {
            policy,
            stop: ObservedStop {
                signal: stop,
                observed: AtomicBool::new(false),
            },
            work: Cell::new(SearchWork {
                total: 0,
                events: [0; 5],
            }),
            phase: Cell::new(CanonPhase::Preparation),
            refusal: Cell::new(None),
        }
    }
    pub(super) fn stop(&self) -> Option<&dyn crate::StopSignal> {
        self.stop
            .signal
            .map(|_| &self.stop as &dyn crate::StopSignal)
    }
    pub(super) fn phase(&self, phase: CanonPhase) {
        self.phase.set(phase);
    }

    pub(super) fn work(&self) -> SearchWork {
        self.work.get()
    }

    pub(super) fn policy(&self) -> SearchPolicy {
        self.policy
    }

    pub(super) fn check_stop(&self) -> Result<(), SearchRefusal> {
        if crate::governor::is_stopped(self.stop()) {
            self.refusal.set(Some(SearchRefusal::Stopped));
            Err(SearchRefusal::Stopped)
        } else {
            Ok(())
        }
    }

    pub(super) fn charge(&self, event: SearchEvent) -> Result<(), SearchRefusal> {
        self.check_stop()?;
        let mut work = self.work.get();
        if matches!(self.policy, SearchPolicy::Finite(limit) if work.total == limit) {
            self.refusal.set(Some(SearchRefusal::Limit));
            return Err(SearchRefusal::Limit);
        }
        let next = work
            .total
            .checked_add(1)
            .zip(work.events[event.index()].checked_add(1));
        let Some((total, count)) = next else {
            self.refusal.set(Some(SearchRefusal::CounterCapacity));
            return Err(SearchRefusal::CounterCapacity);
        };
        work.total = total;
        work.events[event.index()] = count;
        self.work.set(work);
        Ok(())
    }
}

#[derive(Debug)]
struct ObservedStop<'a> {
    signal: Option<&'a dyn crate::StopSignal>,
    observed: AtomicBool,
}

impl crate::StopSignal for ObservedStop<'_> {
    fn stopped(&self) -> bool {
        let stopped = self.signal.is_some_and(crate::StopSignal::stopped);
        if stopped {
            self.observed.store(true, Ordering::Relaxed);
        }
        stopped
    }
}
