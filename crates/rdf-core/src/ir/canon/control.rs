// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Caller-selected search policy and measured, per-run execution evidence.

use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};
use std::alloc::Layout;
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{
    CANON_PROFILE_VERSION, CanonHash, CanonPresentation, RdfcRefusalObservation, RdfcRootProgress,
};

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
    /// The digest of the canonical line sequence of the nested-refinement
    /// labelling.
    LinesDigest,
    /// The canonical document, label table and the digest of the canonical
    /// line sequence together.
    DocumentLinesDigest,
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
    /// The nested-refinement labelling (`CANON_LABELLING_NAME` at
    /// `CANON_LABELLING_VERSION`).
    Labelling {
        /// The version of the labelling.
        version: u64,
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

/// Borrowed coordinates until the control can own the exact recorded graph.
pub(super) enum SearchRequest<'a> {
    Rdfc {
        hash: CanonHash,
        presentation: CanonPresentation,
        graph: Option<&'a str>,
        output: CanonOutput,
    },
    Labelling {
        graph: Option<&'a str>,
        output: CanonOutput,
    },
    DatasetStateV1,
    ComparisonPreparation,
}

impl<'a> SearchRequest<'a> {
    pub(super) const fn labelling(graph: Option<&'a str>, output: CanonOutput) -> Self {
        Self::Labelling { graph, output }
    }

    pub(super) const fn rdfc(
        hash: CanonHash,
        presentation: CanonPresentation,
        graph: Option<&'a str>,
        output: CanonOutput,
    ) -> Self {
        Self::Rdfc {
            hash,
            presentation,
            graph,
            output,
        }
    }

    fn prepare(
        self,
        memory: &mut Memory<'_, impl Admission>,
    ) -> Result<SearchConfiguration, StorageError> {
        let mut selection = |graph| match graph {
            Some(iri) => memory.string(iri).map(CanonSelection::NamedGraph),
            None => Ok(CanonSelection::Dataset),
        };
        Ok(match self {
            Self::Rdfc {
                hash,
                presentation,
                graph,
                output,
            } => SearchConfiguration::Rdfc {
                profile_version: CANON_PROFILE_VERSION,
                hash,
                presentation,
                selection: selection(graph)?,
                output,
            },
            Self::Labelling { graph, output } => SearchConfiguration::Labelling {
                version: crate::canonical_labelling::VERSION,
                selection: selection(graph)?,
                output,
            },
            Self::DatasetStateV1 => SearchConfiguration::DatasetStateV1,
            Self::ComparisonPreparation => SearchConfiguration::ComparisonPreparation,
        })
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
    /// One limited unit of the nested-refinement labelling (8.1 of its
    /// definition), as the running total of the walk of 8.3: its total when the
    /// labelling completes, and on a refusal with `limit` its value after the
    /// charge that passed the limit, the refusal's `units`.
    LabellingUnit,
}

impl SearchEvent {
    const fn index(self) -> usize {
        match self {
            Self::RdfcFrame => 0,
            Self::RdfcPermutation => 1,
            Self::StateNode => 2,
            Self::StateCandidate => 3,
            Self::StateAutomorphism => 4,
            Self::LabellingUnit => 5,
        }
    }
}

/// Actual admitted search events, including work interrupted after its charge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SearchWork {
    total: u64,
    events: [u64; 6],
}

impl purrdf_hash::error::StructuredError for SearchWork {
    purrdf_hash::error_visit! {
        @record self { total, events } => "canonical-search-work" {
            "total" => Unsigned(u128::from(self.total)),
            "events" => List(self.events.iter().map(|count| purrdf_hash::error::ErrorValue::Unsigned(u128::from(*count)))),
        }
    }
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

impl purrdf_hash::error::StructuredError for SearchRefusal {
    purrdf_hash::error_visit! {
        Limit => "canonical-search-limit" {},
        Stopped => "canonical-search-stopped" {},
        CounterCapacity => "canonical-search-counter-capacity" {},
    }
}

/// Immutable execution evidence finalized by the actual engine.
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
/// this evidence and may choose to discard it. Canonicalization obtains receipt
/// storage before starting. Complete-state receipts grow after the scoped source
/// workspace is released. This is not whole-operation memory admission.
#[derive(Debug)]
pub struct CanonicalizationControl<'a> {
    policy: SearchPolicy,
    stop: Option<&'a dyn crate::StopSignal>,
    receipts: Vec<SearchReceipt>,
    last_rdfc_refusal: Option<RdfcRefusalObservation>,
}

impl<'a> CanonicalizationControl<'a> {
    /// Select the policy and optional caller-owned stop signal; there is no default.
    #[must_use]
    pub const fn new(policy: SearchPolicy, stop: Option<&'a dyn crate::StopSignal>) -> Self {
        Self {
            policy,
            stop,
            receipts: Vec::new(),
            last_rdfc_refusal: None,
        }
    }

    /// The caller-owned stop signal this control carries, for a sibling engine
    /// that honours the same request.
    pub(crate) const fn stop_signal(&self) -> Option<&'a dyn crate::StopSignal> {
        self.stop
    }

    /// Borrow all finalized receipts from this control.
    #[must_use]
    pub fn receipts(&self) -> &[SearchReceipt] {
        &self.receipts
    }

    /// Read the latest finalized receipt's finite RDFC observation and position.
    /// A successful or non-RDFC receipt has no such observation. Earlier receipts
    /// are never substituted, and consuming the receipts does not transfer it.
    #[must_use]
    pub fn last_rdfc_refusal(&self) -> Option<(SearchPosition, RdfcRefusalObservation)> {
        self.last_rdfc_refusal.map(|observation| {
            // Finalization installs an observation only after appending its receipt.
            let index = self
                .receipts
                .len()
                .checked_sub(1)
                .expect("a finalized RDFC observation has a receipt");
            (SearchPosition(index), observation)
        })
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
    /// An empty child preserves the last observation. A nonempty child replaces
    /// it with its last run's observation, including absence after success.
    pub fn append_completed(&mut self, mut child: Self) {
        if !child.receipts.is_empty() {
            self.last_rdfc_refusal = child.last_rdfc_refusal;
        }
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
        request: SearchRequest<'_>,
        storage_error: impl Fn(StorageError) -> E,
        operation: impl FnOnce(&SearchRun<'_>) -> Result<T, E>,
    ) -> Result<T, E> {
        self.run_with_receipt_storage(request, &mut Resident, storage_error, operation)
    }

    pub(super) fn run_with_receipt_storage<T, E>(
        &mut self,
        request: SearchRequest<'_>,
        storage: &mut impl Admission,
        storage_error: impl Fn(StorageError) -> E,
        operation: impl FnOnce(&SearchRun<'_>) -> Result<T, E>,
    ) -> Result<T, E> {
        let complete_state = matches!(request, SearchRequest::DatasetStateV1);
        let mut memory = Memory::new(storage);
        let configuration = request.prepare(&mut memory).map_err(&storage_error)?;
        if !complete_state {
            // A canonical run can only start once its exact coordinates and
            // final slot are owned. Failure preserves the previous run.
            self.reserve_receipt_slot(&mut memory)
                .map_err(&storage_error)?;
        }
        let run = SearchRun::new(self.policy, self.stop);
        let result = operation(&run);
        // A complete-state run admits all of its source work under a single
        // scoped owner. Receipt growth must not add a detached allocation
        // to that working set. Preserve an original refusal if both fail.
        if complete_state && let Err(error) = self.reserve_receipt_slot(&mut memory) {
            return result.and_then(|_| Err(storage_error(error)));
        }
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
        self.last_rdfc_refusal = run.rdfc_refusal.get();
        result
    }

    fn reserve_receipt_slot(
        &mut self,
        memory: &mut Memory<'_, impl Admission>,
    ) -> Result<(), StorageError> {
        if self.receipts.len() == self.receipts.capacity() {
            let old = Layout::array::<SearchReceipt>(self.receipts.capacity())
                .map_err(|_| StorageError::SizeOverflow)?;
            memory.add_bytes(old.size())?;
            memory.reserve_for_push(&mut self.receipts)?;
        }
        Ok(())
    }
}

pub(super) struct SearchRun<'a> {
    policy: SearchPolicy,
    stop: ObservedStop<'a>,
    work: Cell<SearchWork>,
    phase: Cell<CanonPhase>,
    refusal: Cell<Option<SearchRefusal>>,
    rdfc_refusal: Cell<Option<RdfcRefusalObservation>>,
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
                events: [0; 6],
            }),
            phase: Cell::new(CanonPhase::Preparation),
            refusal: Cell::new(None),
            rdfc_refusal: Cell::new(None),
        }
    }

    pub(super) fn observe_rdfc_refusal(&self, observation: &RdfcRefusalObservation) {
        self.rdfc_refusal.set(Some(*observation));
    }

    pub(super) fn observe_rdfc_root_progress(&self, progress: RdfcRootProgress) {
        // Only the RDFC driver's BudgetExceeded branch calls this, after charge
        // refusal has installed the observation for that same invocation.
        let mut observation = self
            .rdfc_refusal
            .get()
            .expect("an RDFC budget refusal has active-path evidence");
        observation.first_degree_group = Some(progress);
        self.rdfc_refusal.set(Some(observation));
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

    /// Record the limited units the labelling spent, as its own count. The
    /// engine refuses by its own limit; this is the evidence of what it spent,
    /// not a ceiling checked here.
    pub(super) fn record_units(&self, units: u64) {
        let mut work = self.work.get();
        work.total = work.total.saturating_add(units);
        let slot = &mut work.events[SearchEvent::LabellingUnit.index()];
        *slot = slot.saturating_add(units);
        self.work.set(work);
    }

    /// Record a refusal the engine reached by its own limit.
    pub(super) fn observe_limit(&self) {
        self.refusal.set(Some(SearchRefusal::Limit));
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

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_lex::allocation::Ceiling;

    #[test]
    fn a_refused_receipt_slot_does_not_start_a_run() {
        let mut control = CanonicalizationControl::new(SearchPolicy::Unlimited, None);
        let initial = control.position();
        let result: Result<(), StorageError> = control.run_with_receipt_storage(
            SearchRequest::ComparisonPreparation,
            &mut Ceiling(0),
            std::convert::identity,
            |_| panic!("a run must own its receipt slot before executing"),
        );
        assert_eq!(result, Err(StorageError::AdmissionFailed));
        assert_eq!(control.position(), initial);

        control
            .run(
                SearchRequest::ComparisonPreparation,
                std::convert::identity,
                |_| Ok(()),
            )
            .unwrap();
        let [receipt] = control.since(initial).unwrap() else {
            panic!("only the admitted run finalizes a receipt");
        };
        assert!(receipt.completed);
        assert_eq!(
            receipt.configuration,
            SearchConfiguration::ComparisonPreparation
        );
    }

    #[test]
    fn a_complete_state_receipt_refusal_does_not_displace_an_operation_refusal() {
        let mut control = CanonicalizationControl::new(SearchPolicy::Unlimited, None);
        let initial = control.position();
        for (operation, expected) in [
            (Ok(()), StorageError::AdmissionFailed),
            (Err(StorageError::SizeOverflow), StorageError::SizeOverflow),
        ] {
            let ended = Cell::new(false);
            let result = control.run_with_receipt_storage(
                SearchRequest::DatasetStateV1,
                &mut Ceiling(0),
                std::convert::identity,
                |_| {
                    ended.set(true);
                    operation
                },
            );
            assert!(ended.get());
            assert_eq!(result, Err(expected));
            assert_eq!(control.position(), initial);
        }
    }
}
