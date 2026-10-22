// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Signed maintenance over the original native model and chosen finite proofs.

mod event;
mod prepare;
mod reclaim;
mod storage;
mod work;

use super::{
    Candidate, EvalError, EvalOptions, EvaluationSession, Executable, FixpointState,
    GuardEvaluator, HeadKey, HeadTerm, Limits, ResumeProgress, SessionContext, SessionError,
    StopSignal, TermDictionary, TermIdentity, check_budget, commit, is_stopped, probe,
};
use crate::{
    allocation::{
        flush_retired_bytes, record_factory_retirement, reserve_array_in, reserve_index_in,
    },
    dictionary::{DictionaryError, DictionaryTerm, ReclaimDictionary},
    id::RowId,
    maintenance::{ChangeApplyError, ChangeBatch, ChangeSource},
    store::{Arity, RelationStore, StoreShape},
};
use purrdf_core::FastMap;
use purrdf_lex::{
    allocation::{Admission, Memory, OrderedTable, Resident, StorageError},
    walk::WorkList,
};
use std::{cell::Cell, fmt};

#[derive(Debug)]
pub(super) struct Node<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    terms: [I; N],
    pub(super) proof: Candidate<I, N>,
}

#[derive(Debug)]
struct TermUses<I: TermIdentity> {
    counts: FastMap<I, usize>,
    counts_bytes: usize,
    zero: RowSet<I>,
}

/// A native row bucket keeps its original retained layout through tombstones.
/// Insertion headroom is not a certificate of the storage a scan must visit.
#[derive(Debug)]
struct RowSet<I = RowId> {
    rows: FastMap<I, ()>,
    retained: usize,
}

impl<I> Default for RowSet<I> {
    fn default() -> Self {
        Self {
            rows: FastMap::default(),
            retained: 0,
        }
    }
}

impl<I: Copy + std::hash::Hash + Eq> RowSet<I> {
    fn reserve_in<A: Admission + ?Sized>(
        &mut self,
        additional: usize,
        memory: &mut Memory<'_, A>,
        retired: &mut usize,
    ) -> Result<(), StorageError> {
        reserve_index_in(
            &mut self.rows,
            &mut self.retained,
            additional,
            memory,
            retired,
        )
    }

    fn insert(&mut self, row: I) {
        self.rows.insert(row, ());
    }
    fn remove(&mut self, row: &I) -> bool {
        self.rows.remove(row).is_some()
    }
    fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    fn contains(&self, row: &I) -> bool {
        self.rows.contains_key(row)
    }
    fn len(&self) -> usize {
        self.rows.len()
    }
    fn scan_work(&self) -> usize {
        self.retained
    }
}

impl<'a, I> IntoIterator for &'a RowSet<I> {
    type Item = &'a I;
    type IntoIter = std::collections::hash_map::Keys<'a, I, ()>;
    fn into_iter(self) -> Self::IntoIter {
        self.rows.keys()
    }
}

impl<I: TermIdentity> TermUses<I> {
    fn reserve_in<A: Admission + ?Sized>(
        &mut self,
        count: usize,
        memory: &mut Memory<'_, A>,
        retired: &mut usize,
    ) -> Result<(), StorageError> {
        reserve_index_in(
            &mut self.counts,
            &mut self.counts_bytes,
            count,
            memory,
            retired,
        )?;
        self.zero.reserve_in(count, memory, retired)
    }
    fn change<const N: usize>(&mut self, terms: [I; N], delta: i64) {
        for id in terms {
            if delta > 0 {
                // Every reference is a physically stored row cell, so its count
                // fits the address width of the row storage that owns it.
                let count = self.counts.entry(id).or_default();
                *count = count
                    .checked_add(1)
                    .expect("physical row-cell count fits its address width");
                self.zero.remove(&id);
            } else {
                let count = self
                    .counts
                    .get_mut(&id)
                    .expect("a removed row retains its original term use");
                *count -= 1;
                if *count == 0 {
                    self.counts.remove(&id);
                    self.zero.insert(id);
                }
            }
        }
    }
}

/// One selected native proof, with reverse premise and producer lookup indexes.
/// Alternatives are re-grounded through native frames rather than retained as
/// an otherwise unused Cartesian product of independent factors.
#[derive(Debug)]
pub(crate) struct Supports<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    pub(super) nodes: FastMap<RowId, Node<I, N>>,
    reverse: FastMap<RowId, RowSet>,
    producers: FastMap<usize, RowSet>,
    nodes_bytes: usize,
    reverse_bytes: usize,
    producers_bytes: usize,
    pub(super) withdrawn: FastMap<[I; N], crate::store::WithdrawnRow<I>>,
    changed: Vec<(RowId, [I; N])>,
    membership: FastMap<[I; N], i64>,
    membership_bytes: usize,
    withdrawn_bytes: usize,
    retired_bytes: usize,
    tracking: bool,
    native_heights: bool,
    term_uses: Option<TermUses<I>>,
}

impl<I: TermIdentity, const N: usize> Default for Supports<I, N>
where
    Arity<N>: StoreShape<N>,
{
    fn default() -> Self {
        Self {
            nodes: FastMap::default(),
            reverse: FastMap::default(),
            producers: FastMap::default(),
            nodes_bytes: 0,
            reverse_bytes: 0,
            producers_bytes: 0,
            withdrawn: FastMap::default(),
            changed: Vec::new(),
            membership: FastMap::default(),
            membership_bytes: 0,
            withdrawn_bytes: 0,
            retired_bytes: 0,
            tracking: false,
            native_heights: false,
            term_uses: None,
        }
    }
}

impl<I: TermIdentity, const N: usize> Supports<I, N>
where
    Arity<N>: StoreShape<N>,
{
    pub(super) fn height(&self, row: RowId) -> u32 {
        self.nodes
            .get(&row)
            .map_or(0, |node| node.proof.proof_height)
    }

    pub(super) const fn uses_native_heights(&self) -> bool {
        self.native_heights
    }

    pub(super) fn native_heights() -> Self {
        Self {
            native_heights: true,
            ..Self::default()
        }
    }
    /// Reserve every companion before a native row or replacement is published.
    pub(super) fn reserve<A: Admission + ?Sized>(
        &mut self,
        candidate: &Candidate<I, N>,
        memory: &mut Memory<'_, A>,
    ) -> Result<(), StorageError> {
        self.reserve_terms(memory)?;
        reserve_index_in(
            &mut self.nodes,
            &mut self.nodes_bytes,
            1,
            memory,
            &mut self.retired_bytes,
        )?;
        reserve_index_in(
            &mut self.reverse,
            &mut self.reverse_bytes,
            candidate.sources.len(),
            memory,
            &mut self.retired_bytes,
        )?;
        reserve_index_in(
            &mut self.producers,
            &mut self.producers_bytes,
            1,
            memory,
            &mut self.retired_bytes,
        )?;
        for source in &candidate.sources {
            debug_assert_eq!(source.source, crate::clause::ReadSource::Evaluation);
            self.reverse.entry(source.row).or_default().reserve_in(
                1,
                memory,
                &mut self.retired_bytes,
            )?;
        }
        self.producers
            .entry(candidate.rule)
            .or_default()
            .reserve_in(1, memory, &mut self.retired_bytes)?;
        if self.tracking {
            reserve_index_in(
                &mut self.membership,
                &mut self.membership_bytes,
                1,
                memory,
                &mut self.retired_bytes,
            )?;
            reserve_array_in(
                &mut self.changed,
                memory,
                &mut self.retired_bytes,
                |values, memory| memory.reserve_for_push(values),
            )?;
        }
        Ok(())
    }

    fn reserve_terms<A: Admission + ?Sized>(
        &mut self,
        memory: &mut Memory<'_, A>,
    ) -> Result<(), StorageError> {
        self.flush_retired(memory)?;
        if let Some(uses) = &mut self.term_uses {
            uses.reserve_in(N, memory, &mut self.retired_bytes)?;
        }
        Ok(())
    }

    /// Keep a refused shrink outstanding until the next native boundary.
    pub(super) fn flush_retired<A: Admission + ?Sized>(
        &mut self,
        memory: &mut Memory<'_, A>,
    ) -> Result<(), StorageError> {
        flush_retired_bytes(&mut self.retired_bytes, memory)
    }

    pub(super) fn retire_proof(&mut self, proof: Candidate<I, N>) {
        let bytes = proof
            .sources
            .capacity()
            .checked_mul(size_of::<super::SourceRow<I, N>>())
            .expect("the original proof array has a valid physical layout");
        drop(proof);
        self.retired_bytes = self
            .retired_bytes
            .checked_add(bytes)
            .expect("destroyed proof layouts fit their original admitted total");
    }

    pub(super) fn record(
        &mut self,
        row: RowId,
        terms: [I; N],
        proof: Candidate<I, N>,
        inserted: bool,
    ) {
        for source in &proof.sources {
            self.reverse
                .get_mut(&source.row)
                .expect("premise bucket was admitted")
                .insert(row);
        }
        self.producers
            .get_mut(&proof.rule)
            .expect("producer bucket was admitted")
            .insert(row);
        if let Some(old) = self.nodes.insert(row, Node { terms, proof }) {
            let current = &self.nodes[&row].proof;
            let mut released = 0_usize;
            for source in &old.proof.sources {
                if !current.sources.iter().any(|s| s.row == source.row) {
                    released = released
                        .checked_add(unlink(&mut self.reverse, source.row, row))
                        .expect("retired premise buckets fit their original grant");
                }
            }
            if old.proof.rule != current.rule {
                released = released
                    .checked_add(unlink(&mut self.producers, old.proof.rule, row))
                    .expect("retired producer buckets fit their original grant");
            }
            self.retire_proof(old.proof);
            self.retired_bytes = self
                .retired_bytes
                .checked_add(released)
                .expect("retired buckets fit their original grant");
        }
        if self.tracking {
            self.changed.push((row, terms));
            if inserted {
                self.membership(terms, 1);
            }
        } else if inserted && let Some(uses) = &mut self.term_uses {
            uses.change(terms, 1);
        }
    }

    fn membership(&mut self, terms: [I; N], delta: i64) {
        if let Some(uses) = &mut self.term_uses {
            uses.change(terms, delta);
        }
        let weight = self.membership.get(&terms).copied().unwrap_or(0) + delta;
        if weight == 0 {
            self.membership.remove(&terms);
        } else {
            self.membership.insert(terms, weight);
        }
    }

    fn remove(&mut self, row: RowId) {
        if let Some(old) = self.nodes.remove(&row) {
            let mut released = 0_usize;
            for source in &old.proof.sources {
                released = released
                    .checked_add(unlink(&mut self.reverse, source.row, row))
                    .expect("retired premise buckets fit their original grant");
            }
            released = released
                .checked_add(unlink(&mut self.producers, old.proof.rule, row))
                .expect("retired producer buckets fit their original grant");
            self.retire_proof(old.proof);
            self.retired_bytes = self
                .retired_bytes
                .checked_add(released)
                .expect("retired buckets fit their original grant");
        }
    }
}

fn unlink<K: Copy + std::hash::Hash + Eq>(
    map: &mut FastMap<K, RowSet>,
    key: K,
    row: RowId,
) -> usize {
    if let Some(rows) = map.get_mut(&key) {
        rows.remove(&row);
        if rows.is_empty() {
            let retired = map.remove(&key).expect("an empty original bucket");
            let bytes = retired.retained;
            drop(retired);
            return bytes;
        }
    }
    0
}

/// A transaction cannot be accepted or its native execution failed.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MaintenanceError<E = core::convert::Infallible> {
    /// Original committed-input construction or admission failure.
    Change(crate::maintenance::ChangeError),
    /// The initial least model has not completed.
    InitialIncomplete,
    /// The original pending transaction must finish first.
    Busy,
    /// A resulting asserted multiplicity is negative or exceeds `i64::MAX`.
    Multiplicity {
        /// Exact resulting weight before narrowing.
        weight: i128,
    },
    /// The cumulative transaction work allowance refused its next native step.
    /// The original transaction remains pending and can resume with a larger allowance.
    WorkLimit {
        /// Caller-stated cumulative allowance.
        limit: u128,
        /// Work already admitted, including preparation and native joins.
        consumed: u128,
        /// Cumulative allowance required by the next retained operation.
        needed: u128,
    },
    /// Original initial-session failure.
    Session(SessionError<E>),
    /// Original native execution failure.
    Failed(EvalError<E>),
}

impl<E: fmt::Display> fmt::Display for MaintenanceError<E> {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Change(error) => error.fmt(out),
            Self::InitialIncomplete => out.write_str("initial maintained model is incomplete"),
            Self::Busy => out.write_str("a maintained transaction is still pending"),
            Self::Multiplicity { weight } => write!(
                out,
                "asserted multiplicity {weight} is outside 0..=i64::MAX"
            ),
            Self::Session(error) => error.fmt(out),
            Self::WorkLimit {
                limit,
                consumed,
                needed,
            } => write!(
                out,
                "maintained transaction consumed {consumed} work units; next operation needs {needed}, limit {limit}"
            ),
            Self::Failed(error) => error.fmt(out),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for MaintenanceError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Change(error) => Some(error),
            Self::Session(error) => Some(error),
            Self::Failed(error) => Some(error),
            _ => None,
        }
    }
}
impl<E: purrdf_hash::error::StructuredError> purrdf_hash::error::StructuredError
    for MaintenanceError<E>
{
    purrdf_hash::error_visit! {
        Change(cause) => "datalog-maintenance-input" { "cause" => Cause(cause) },
        InitialIncomplete => "datalog-maintenance-initial-incomplete" {},
        Busy => "datalog-maintenance-busy" {},
        Multiplicity { weight } => "datalog-maintenance-multiplicity" { "weight" => Signed(*weight) },
        WorkLimit { limit, consumed, needed } => "datalog-maintenance-work" {
            "limit" => Unsigned(*limit), "consumed" => Unsigned(*consumed), "needed" => Unsigned(*needed)
        },
        Session(cause) => "datalog-maintenance-session" { "cause" => Cause(cause) },
        Failed(cause) => "datalog-maintenance-failed" { "cause" => Cause(cause) },
    }
}

#[derive(Debug, Clone, Copy)]
enum Task<I, const N: usize> {
    Body {
        rule: usize,
        body: usize,
        terms: [I; N],
    },
    Head {
        rule: usize,
        head: usize,
        terms: [I; N],
    },
    Absence {
        rule: usize,
    },
}

type Impacts = crate::plan::DependencyGraph;

#[derive(Clone, Copy, PartialEq, Eq)]
enum InputPhase {
    Reserve,
    Event,
    Publication,
    Publish,
}

struct InputState {
    existing: Option<RowId>,
    changes_proof: bool,
    phase: InputPhase,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MutationPhase {
    Tasks,
    Settlement,
    Complete,
}

struct Pending<'r, M, S: Admission, D: TermDictionary, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    // Retain the complete original batch and grant until atomic acknowledgement.
    batch: ChangeBatch<M, S, D::Id, N>,
    changes: Vec<([D::Id; N], i64)>,
    delete: Vec<(RowId, [D::Id; N])>,
    deletion: usize,
    input: usize,
    input_state: Option<InputState>,
    event: Option<event::Frame<D::Id, N>>,
    installed_weights: usize,
    phase: MutationPhase,
    withdrawals_cleared: bool,
    tasks: Tasks<D::Id, N>,
    active: Option<probe::Frame<'r, D, N>>,
    task: Option<Task<D::Id, N>>,
    publication: Option<commit::Frame<'r, D::Id, N>>,
    output_work_admitted: bool,
    retired_bytes: usize,
    retirement: usize,
    delta_gathered: bool,
    delta_sort: Option<purrdf_lex::walk::HeapSortFrame>,
    reclaim: reclaim::Frame<D::Id>,
    delta: Vec<([D::Id; N], i64)>,
    altered: RowSet<D::Id>,
    reached: Vec<bool>,
}

/// Certified progress of one predicate across all its original graph partitions.
/// A withheld relation exposes no iterator; a sound lower bound is explicitly
/// distinct from a complete extension through the target host position.
pub struct RelationProgress<'a, M, D: TermDictionary, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    status: super::StratumStatus,
    applied: Option<&'a M>,
    target: Option<&'a M>,
    store: Option<&'a RelationStore<D, N>>,
    dictionary: &'a D,
    predicate: D::Id,
}

impl<M, D: TermDictionary, const N: usize> fmt::Debug for RelationProgress<'_, M, D, N>
where
    Arity<N>: StoreShape<N>,
{
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_struct("RelationProgress")
            .field("status", &self.status)
            .field("predicate", &self.predicate)
            .finish_non_exhaustive()
    }
}

impl<M, D: TermDictionary, const N: usize> RelationProgress<'_, M, D, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Whether returned rows are complete, a sound lower bound, or withheld.
    pub const fn status(&self) -> super::StratumStatus {
        self.status
    }
    /// Original host position through which this relation is complete.
    pub const fn applied_watermark(&self) -> Option<&M> {
        self.applied
    }
    /// Original host position being evaluated, or the latest completed one.
    pub const fn target_watermark(&self) -> Option<&M> {
        self.target
    }

    /// Original source dictionary for explicit rendering of this view's native rows.
    pub const fn dictionary(&self) -> &D {
        self.dictionary
    }
    /// Original native rows when certified; `None` explicitly means withheld.
    pub fn rows(&self) -> Option<impl Iterator<Item = (RowId, [D::Id; N])> + '_> {
        self.store
            .map(|store| store.rows_for_predicate(self.predicate))
    }
}

/// Exact net membership changes of one completed transaction in canonical fact order.
/// Temporary over-deletion/restoration and witness-only repairs contribute no row.
#[derive(Debug)]
pub struct ModelDelta<'a, M, I: TermIdentity, const N: usize = 4> {
    watermark: &'a M,
    changes: &'a [([I; N], i64)],
}

/// Borrowed completed delta, preserving the original dictionary's failure type.
pub type DeltaResult<'a, M, D, const N: usize = 4> = Result<
    Option<ModelDelta<'a, M, <D as TermDictionary>::Id, N>>,
    MaintenanceError<<D as TermDictionary>::Error>,
>;

impl<M, I: TermIdentity, const N: usize> ModelDelta<'_, M, I, N> {
    /// Original host position acknowledged by this completed result.
    pub const fn watermark(&self) -> &M {
        self.watermark
    }
    /// Native inserted (+1) and finally removed (-1) facts; zeros are absent.
    pub const fn changes(&self) -> &[([I; N], i64)] {
        self.changes
    }
}

/// Why transaction preparation retained its original unfinished frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparationPause {
    /// The original caller's stop signal fired.
    Stopped,
    /// The next metadata unit would exceed the stated cumulative allowance.
    WorkLimit {
        /// Caller-stated metadata-work limit.
        limit: u128,
        /// Total required to admit the next original operation.
        needed: u128,
    },
}

/// Preparation progress, separate from completion of a model transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparationProgress {
    /// Native mutation tasks are prepared; the transaction is not acknowledged.
    Ready,
    /// Original validation/discovery/sort positions remain owned for resumption.
    Paused {
        /// Cumulative metadata-work units already admitted.
        consumed: u128,
        /// Original stop or work refusal.
        reason: PreparationPause,
    },
}

/// A long-lived native model, its finite chosen proofs and original transaction owner.
///
/// Initialization uses the normal evaluator. Updates retain the same store,
/// dictionary and rule/guard kernels. Reads of a new complete snapshot are withheld
/// until its entire transaction settles; the opaque applied watermark advances once.
/// Native witnesses are retained without constructing a parallel lexical proof table.
pub struct Maintained<
    'r,
    M,
    S: Admission,
    D: TermDictionary,
    const N: usize = 4,
    A: Admission = Resident,
> where
    Arity<N>: StoreShape<N>,
{
    session: EvaluationSession<'r, D, N>,
    weights: FastMap<[D::Id; N], i64>,
    weights_bytes: usize,
    levels: Vec<usize>,
    impacts: Impacts,
    pending: Option<Pending<'r, M, S, D, N>>,
    preparing: Option<prepare::Frame<M, S, D::Id, N>>,
    preparation_work: u128,
    mutation_work: u128,
    retired_bytes: usize,
    applied: Option<ChangeBatch<M, S, D::Id, N>>,
    delta: Vec<([D::Id; N], i64)>,
    failure: Option<EvalError<D::Error>>,
    prepared: bool,
    initial_reclaim: reclaim::Frame<D::Id>,
    // Rust destroys fields in declaration order. The original grant must outlive
    // the session, every pending frame, the result and their dependent payloads.
    storage: storage::Storage<A>,
}

pub(super) type ReleaseTerm<D> = fn(
    &mut D,
    <D as TermDictionary>::Id,
) -> Result<(), DictionaryError<<D as TermDictionary>::Error>>;

impl<M, S: Admission, D: TermDictionary, const N: usize, A: Admission> fmt::Debug
    for Maintained<'_, M, S, D, N, A>
where
    Arity<N>: StoreShape<N>,
{
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_struct("Maintained")
            .field("session", &self.session)
            .field("pending", &self.pending.is_some())
            .field("preparing", &self.preparing.is_some())
            .field("admitted_metadata_bytes", &self.storage.live())
            .finish_non_exhaustive()
    }
}

impl<'r, M, S: Admission, D: TermDictionary, const N: usize> Maintained<'r, M, S, D, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Own the original seed; every input row initially has asserted multiplicity one.
    /// Native chosen witnesses are always retained; lexical projection is explicit.
    ///
    /// # Errors
    /// Preserves original source, memory and unsupported base-schedule failures.
    pub fn new(
        exe: &'r Executable<N>,
        edb: RelationStore<D, N>,
        guards: &'r dyn GuardEvaluator<D, N>,
        context: SessionContext,
        options: EvalOptions,
    ) -> Result<Self, EvalError<D::Error>> {
        Self::new_in(exe, edb, guards, context, options, Resident)
    }

    /// Own a model with explicit retirement of unused dictionary terms.
    /// Current rows, proofs and the most recent delta retain their terms;
    /// retired IDs are never reassigned.
    ///
    /// # Errors
    /// Preserves the original initialization or reference-storage failure.
    pub fn new_reclaiming(
        exe: &'r Executable<N>,
        edb: RelationStore<D, N>,
        guards: &'r dyn GuardEvaluator<D, N>,
        context: SessionContext,
        options: EvalOptions,
    ) -> Result<Self, EvalError<D::Error>>
    where
        D: ReclaimDictionary,
    {
        Self::new_reclaiming_in(exe, edb, guards, context, options, Resident)
    }
}

impl<'r, M, S: Admission, D: TermDictionary, const N: usize, A: Admission>
    Maintained<'r, M, S, D, N, A>
where
    Arity<N>: StoreShape<N>,
{
    fn new_in(
        exe: &'r Executable<N>,
        edb: RelationStore<D, N>,
        guards: &'r dyn GuardEvaluator<D, N>,
        context: SessionContext,
        options: EvalOptions,
        admission: A,
    ) -> Result<Self, EvalError<D::Error>> {
        let mut storage = storage::Storage::new(admission);
        let mut weights = FastMap::default();
        let mut weights_bytes = 0;
        storage
            .with_memory(|memory| {
                purrdf_core::hash::reserve_map_with_retained_memory(
                    &mut weights,
                    &mut weights_bytes,
                    edb.row_count(),
                    memory,
                )
            })
            .map_err(EvalError::Storage)?;
        weights.extend(edb.rows_from(0).map(|(_, ids)| (ids, 1)));
        let mut levels = Vec::new();
        storage
            .with_memory(|memory| memory.reserve(&mut levels, exe.rule_count()))
            .map_err(EvalError::Storage)?;
        levels.resize(exe.rule_count(), 0);
        for level in 0..exe.stratum_count() {
            for &rule in exe.stratum_rule_indices(level) {
                levels[rule] = level;
            }
        }
        let impacts = storage
            .with_memory(|memory| Impacts::new_in(exe.rules(), memory))
            .map_err(EvalError::Storage)?;
        let mut edb = edb;
        let before_store = storage.live();
        storage
            .with_memory(|memory| edb.admit_native_metadata_in(memory))
            .map_err(native_error)?;
        let mut session = EvaluationSession::new(
            exe,
            edb,
            guards,
            context,
            options.with_derivation_mode(super::DerivationMode::ModelOnly),
        )?;
        let state = session.state.as_mut().expect("new session owns its seed");
        state.account_native_memory(before_store, storage.live());
        state.native.get_or_insert_with(Supports::default);
        Ok(Self {
            session,
            weights,
            weights_bytes,
            levels,
            impacts,
            pending: None,
            preparing: None,
            preparation_work: 0,
            mutation_work: 0,
            retired_bytes: 0,
            applied: None,
            delta: Vec::new(),
            failure: None,
            prepared: false,
            initial_reclaim: reclaim::Frame::default(),
            storage,
        })
    }

    /// Own a model with explicit retirement of unused dictionary terms.
    ///
    /// Current rows, proofs and the most recent ModelDelta retain their terms.
    /// A raw ID outside these owners does not pin its spelling. Retired IDs are
    /// never reassigned, and resolution follows the dictionary's expiry contract.
    ///
    /// # Errors
    /// Preserves the original initialization or reference-storage failure.
    fn new_reclaiming_in(
        exe: &'r Executable<N>,
        edb: RelationStore<D, N>,
        guards: &'r dyn GuardEvaluator<D, N>,
        context: SessionContext,
        options: EvalOptions,
        admission: A,
    ) -> Result<Self, EvalError<D::Error>>
    where
        D: ReclaimDictionary,
    {
        let mut owner = Self::new_in(exe, edb, guards, context, options, admission)?;
        let state = owner
            .session
            .state
            .as_mut()
            .expect("new model owns its seed");
        let mut uses = TermUses {
            counts: FastMap::default(),
            counts_bytes: 0,
            zero: RowSet::default(),
        };
        let mut retired = 0;
        for (_, terms) in state.rel.rows_from(0) {
            owner
                .storage
                .with_memory(|memory| uses.reserve_in(N, memory, &mut retired))
                .map_err(EvalError::Storage)?;
            uses.change(terms, 1);
        }
        for id in state.rel.interner().owned_terms() {
            if !uses.counts.contains_key(&id) {
                owner
                    .storage
                    .with_memory(|memory| uses.zero.reserve_in(1, memory, &mut retired))
                    .map_err(EvalError::Storage)?;
                uses.zero.insert(id);
            }
        }
        state.native_bytes = state
            .native_bytes
            .checked_add(uses.counts_bytes)
            .and_then(|bytes| bytes.checked_add(uses.zero.retained))
            .expect("admitted native term metadata fits its original grant");
        assert_eq!(
            retired, 0,
            "successful construction settled its native factories"
        );
        state
            .native
            .as_mut()
            .expect("maintenance captures native proofs")
            .term_uses = Some(uses);
        state.release_term = Some(D::release_term);
        Ok(owner)
    }

    /// Position of the last completely applied transaction, absent before the first.
    pub fn applied_watermark(&self) -> Option<&M> {
        self.applied.as_ref().map(ChangeBatch::watermark)
    }

    /// Original position still retained by an incomplete transaction.
    pub fn pending_watermark(&self) -> Option<&M> {
        self.pending
            .as_ref()
            .map(|p| p.batch.watermark())
            .or_else(|| self.preparing.as_ref().map(prepare::Frame::watermark))
    }

    /// Read one relation's applied position and certified extension while an
    /// unrelated transaction runs. Affected relations initially withhold all rows;
    /// after over-deletion and base publication they expose only a sound lower bound.
    ///
    /// # Errors
    /// Preserves poisoned execution and original source failures.
    pub fn relation(
        &self,
        predicate: D::Id,
    ) -> Result<RelationProgress<'_, M, D, N>, MaintenanceError<D::Error>> {
        if let Some(error) = &self.failure {
            return Err(MaintenanceError::Failed(error.clone()));
        }
        let state = self
            .session
            .state
            .as_ref()
            .expect("maintenance retains its model");
        state
            .rel
            .interner()
            .check_read()
            .map_err(|error| MaintenanceError::Failed(EvalError::Source(error)))?;
        if !self.session.complete {
            self.session
                .partial_model()
                .map_err(MaintenanceError::Session)?;
        }
        let mut status = if self.session.complete && self.prepared {
            super::StratumStatus::Complete
        } else {
            super::StratumStatus::SoundLowerBound
        };
        let mut applied = self.applied_watermark();
        let mut target = applied;
        if let Some(preparing) = &self.preparing {
            target = Some(preparing.watermark());
            status = super::StratumStatus::Withheld;
        } else if let Some(pending) = &self.pending {
            target = Some(pending.batch.watermark());
            if pending.phase == MutationPhase::Complete {
                return Ok(RelationProgress {
                    status: super::StratumStatus::Complete,
                    applied: target,
                    target,
                    store: Some(&state.rel),
                    predicate,
                    dictionary: state.rel.interner(),
                });
            }
            let mut affected = pending.altered.contains(&predicate);
            for (index, symbol) in self.impacts.predicates.iter().enumerate() {
                if !pending.reached[index] {
                    continue;
                }
                if symbol == crate::plan::ANY_PREDICATE
                    || state
                        .rel
                        .interner()
                        .lookup_term(symbol)
                        .map_err(|error| MaintenanceError::Failed(EvalError::Source(error)))?
                        == Some(predicate)
                {
                    affected = true;
                    break;
                }
            }
            if affected {
                status = if pending.deletion == pending.delete.len()
                    && pending.input == pending.changes.len()
                {
                    super::StratumStatus::SoundLowerBound
                } else {
                    super::StratumStatus::Withheld
                };
            } else {
                applied = target;
            }
        }
        state
            .rel
            .interner()
            .check_read()
            .map_err(|error| MaintenanceError::Failed(EvalError::Source(error)))?;
        Ok(RelationProgress {
            status,
            applied,
            target,
            store: (status != super::StratumStatus::Withheld).then_some(&state.rel),
            predicate,
            dictionary: state.rel.interner(),
        })
    }

    /// Borrow the last atomically completed membership delta and its original position.
    ///
    /// # Errors
    /// Refuses incomplete snapshots or original execution/source failures.
    pub fn model_delta(&self) -> DeltaResult<'_, M, D, N> {
        self.facts()?;
        Ok(self.applied.as_ref().map(|batch| ModelDelta {
            watermark: batch.watermark(),
            changes: &self.delta,
        }))
    }

    /// Cumulative native work and retained physical row extent, including pending work.
    pub fn budget(&self) -> super::BudgetReport {
        self.session
            .state
            .as_ref()
            .expect("maintenance retains its model")
            .report()
    }

    /// Prepare host input terms in this model's original dictionary, without
    /// changing any asserted multiplicity or acknowledging a transaction.
    /// Term growth is included in the next resume's original arena governor.
    ///
    /// # Errors
    /// Refuses pending execution and preserves original dictionary/storage failures.
    pub fn admit_terms(
        &mut self,
        terms: [DictionaryTerm<'_, D::Id>; N],
    ) -> Result<[D::Id; N], MaintenanceError<D::Error>> {
        self.ready()?;
        let state = self
            .session
            .state
            .as_mut()
            .expect("maintenance retains its model");
        let before = self.storage.live();
        let reserved = self.storage.with_memory(|memory| {
            state
                .native
                .as_mut()
                .expect("native proof owner")
                .reserve_terms(memory)
        });
        state.account_native_memory(before, self.storage.live());
        reserved.map_err(|error| MaintenanceError::Failed(EvalError::Storage(error)))?;
        let ids = state
            .rel
            .intern_terms(terms)
            .map_err(|error| MaintenanceError::Failed(native_error(error)))?;
        if let Some(uses) = &mut state.native.as_mut().expect("native proof owner").term_uses {
            for id in ids {
                if !uses.counts.contains_key(&id) {
                    uses.zero.insert(id);
                }
            }
        }
        Ok(ids)
    }

    /// Borrow all current chosen proofs without projecting a lexical witness table.
    /// Iteration is a lookup order; callers choose their output's canonical order.
    ///
    /// # Errors
    /// Refuses pending execution and preserves original source failures.
    pub fn witnesses(
        &self,
    ) -> Result<
        impl ExactSizeIterator<Item = probe::HeadWitness<'_, D, N>>,
        MaintenanceError<D::Error>,
    > {
        let rel = self.facts()?;
        let native = self
            .session
            .state
            .as_ref()
            .expect("maintenance retains its model")
            .native
            .as_ref()
            .expect("maintenance captures native proofs");
        Ok(native.nodes.values().map(move |node| probe::HeadWitness {
            rel,
            terms: &node.terms,
            candidate: &node.proof,
        }))
    }

    /// Borrow only a completed snapshot; pending deletions never leak stale certainty.
    ///
    /// # Errors
    /// Refuses initialization, pending work or the original source/execution failure.
    pub fn facts(&self) -> Result<&RelationStore<D, N>, MaintenanceError<D::Error>> {
        self.ready()?;
        let state = self
            .session
            .state
            .as_ref()
            .expect("maintenance retains its model");
        state
            .rel
            .interner()
            .check_read()
            .map_err(|e| MaintenanceError::Failed(EvalError::Source(e)))?;
        Ok(&state.rel)
    }

    /// Borrow the current chosen native witness; asserted facts have height zero
    /// and need no rule application. The borrow pins the complete model.
    ///
    /// # Errors
    /// Refuses pending work and preserves the original source failure.
    pub fn witness(
        &self,
        terms: [D::Id; N],
    ) -> Result<Option<probe::HeadWitness<'_, D, N>>, MaintenanceError<D::Error>> {
        let rel = self.facts()?;
        let Some(row) =
            row_for(rel, terms).map_err(|e| MaintenanceError::Failed(EvalError::Source(e)))?
        else {
            return Ok(None);
        };
        let native = self
            .session
            .state
            .as_ref()
            .expect("maintenance retains its model")
            .native
            .as_ref()
            .expect("maintenance captures native proofs");
        Ok(native.nodes.get(&row).map(|node| probe::HeadWitness {
            rel,
            terms: &node.terms,
            candidate: &node.proof,
        }))
    }

    fn ready(&self) -> Result<(), MaintenanceError<D::Error>> {
        if let Some(error) = &self.failure {
            return Err(MaintenanceError::Failed(error.clone()));
        }
        if self.pending.is_some() || self.preparing.is_some() {
            return Err(MaintenanceError::Busy);
        }
        if !self.session.complete || !self.prepared {
            return Err(MaintenanceError::InitialIncomplete);
        }
        Ok(())
    }

    /// Accept one complete committed signed transaction without acknowledging it.
    ///
    /// Validation and dependency discovery precede mutation. The batch's original
    /// watermark and admission remain owned through every governor/stop refusal.
    ///
    /// # Errors
    /// Refuses concurrent work, invalid multiplicity or original source/storage failure.
    pub fn apply(
        &mut self,
        batch: ChangeBatch<M, S, D::Id, N>,
    ) -> Result<(), MaintenanceError<D::Error>> {
        self.begin(batch)?;
        let progress = self.prepare_until(u128::MAX, None)?;
        assert_eq!(
            progress,
            PreparationProgress::Ready,
            "unlimited preparation finishes before synchronous apply returns"
        );
        Ok(())
    }

    /// Queue an original batch without unbounded validation or dependency discovery.
    /// `prepare_until` can suspend that work before native model mutation.
    ///
    /// # Errors
    /// Refuses concurrent or uninitialized execution before taking the transaction.
    pub fn begin(
        &mut self,
        batch: ChangeBatch<M, S, D::Id, N>,
    ) -> Result<(), MaintenanceError<D::Error>> {
        self.ready()?;
        self.preparing = Some(prepare::Frame::new(batch));
        self.preparation_work = 0;
        self.mutation_work = 0;
        Ok(())
    }

    /// Metadata work admitted for the current or most recently prepared batch.
    pub const fn preparation_work(&self) -> u128 {
        self.preparation_work
    }

    /// Admitted preparation, metadata, native joins and replacement layout bytes.
    pub fn transaction_work(&self) -> u128 {
        self.preparation_work
            .checked_add(self.mutation_work)
            .expect("admitted transaction work is representable")
    }

    /// Prepare a queued batch through a cumulative metadata-work allowance.
    /// Rows, graph edges, native comparisons and task publication take one unit;
    /// table scans reserve their retained layout-byte bound and initialization
    /// reserves its element count. Native replacement allocations add their
    /// requested layout bytes before invoking the original storage grant.
    /// These are declared metadata-work units.
    /// Ready permits model execution; it does not acknowledge the transaction.
    ///
    /// # Errors
    /// Preserves source/storage failures and rejects invalid multiplicity without
    /// changing the preceding model. Storage refusals retain the original frame.
    pub fn prepare_until(
        &mut self,
        limit: u128,
        stop: Option<&dyn StopSignal>,
    ) -> Result<PreparationProgress, MaintenanceError<D::Error>> {
        if let Some(error) = &self.failure {
            return Err(MaintenanceError::Failed(error.clone()));
        }
        if !self.prepared {
            return Err(MaintenanceError::InitialIncomplete);
        }
        let Some(frame) = self.preparing.as_mut() else {
            return Ok(PreparationProgress::Ready);
        };
        let used = Cell::new(self.preparation_work);
        let meter = work::Meter::new(&used, 0, limit, stop);
        let mut inputs = prepare::Inputs {
            exe: self.session.exe,
            impacts: &self.impacts,
            levels: &self.levels,
            weights: &mut self.weights,
            weights_bytes: &mut self.weights_bytes,
            state: self
                .session
                .state
                .as_ref()
                .expect("preparation retains its original model"),
        };
        let before = self.storage.live();
        let result = self.storage.with_memory(|memory| {
            meter.with_memory(memory, |memory| {
                flush_retired_bytes(&mut self.retired_bytes, memory)
                    .map_err(|error| MaintenanceError::Failed(EvalError::Storage(error)))?;
                frame.resume(&mut inputs, memory, |cost| meter.allow(cost))
            })
        });
        self.preparation_work = used.get();
        self.session
            .state
            .as_mut()
            .expect("preparation retains its original model")
            .account_native_memory(before, self.storage.live());
        let result =
            match result {
                Err(MaintenanceError::Failed(EvalError::Storage(
                    StorageError::AdmissionFailed,
                ))) if meter.refused() => Ok(false),
                result => result,
            };
        match result {
            Ok(true) => {
                let mut pending = self
                    .preparing
                    .take()
                    .expect("completed preparation remains owned")
                    .finish();
                pending.delta = std::mem::take(&mut self.delta);
                pending.delta.clear();
                self.pending = Some(pending);
                Ok(PreparationProgress::Ready)
            }
            Ok(false) => Ok(PreparationProgress::Paused {
                consumed: self.preparation_work,
                reason: meter
                    .preparation_pause()
                    .map_err(|error| MaintenanceError::Failed(EvalError::Storage(error)))?,
            }),
            Err(error) => {
                match &error {
                    MaintenanceError::Multiplicity { .. } => {
                        let rejected = self.preparing.take().expect("rejected original batch");
                        self.retired_bytes = self
                            .retired_bytes
                            .checked_add(rejected.reject())
                            .expect("rejected preparation fits its original grant");
                    }
                    MaintenanceError::Failed(EvalError::Storage(_)) => {}
                    MaintenanceError::Failed(failure) => self.failure = Some(failure.clone()),
                    _ => {}
                }
                Err(error)
            }
        }
    }

    /// Poll one host transaction into this ready model, without acknowledging it.
    /// Returns `false` only when the host reports no available committed batch.
    /// Pending execution refuses before polling, so a second batch is never lost
    /// merely because the preceding one still needs to resume.
    ///
    /// # Errors
    /// Preserves the original host failure separately from model refusal.
    pub fn apply_next<T>(
        &mut self,
        source: &mut T,
    ) -> Result<bool, ChangeApplyError<D::Error, T::Error>>
    where
        T: ChangeSource<N, Id = D::Id, Watermark = M, Admission = S>,
    {
        if !self.begin_next(source)? {
            return Ok(false);
        }
        let progress = self
            .prepare_until(u128::MAX, None)
            .map_err(ChangeApplyError::Model)?;
        assert_eq!(
            progress,
            PreparationProgress::Ready,
            "unlimited host preparation retains no unfinished phase"
        );
        Ok(true)
    }

    /// Poll one batch into bounded preparation, preserving the host's error type.
    ///
    /// # Errors
    /// Refuses busy execution before polling and preserves original source failures.
    pub fn begin_next<T>(
        &mut self,
        source: &mut T,
    ) -> Result<bool, ChangeApplyError<D::Error, T::Error>>
    where
        T: ChangeSource<N, Id = D::Id, Watermark = M, Admission = S>,
    {
        self.ready().map_err(ChangeApplyError::Model)?;
        let Some(batch) = source.next_change().map_err(ChangeApplyError::Source)? else {
            return Ok(false);
        };
        self.begin(batch).map_err(ChangeApplyError::Model)?;
        Ok(true)
    }

    /// Resume initialization or the original transaction under cumulative governors.
    ///
    /// # Errors
    /// Source and guard failures poison this owner. Resource/stop refusals retain
    /// the pending batch and native frames and return a typed paused result.
    pub fn resume(
        &mut self,
        options: EvalOptions,
        stop: Option<&dyn StopSignal>,
    ) -> Result<ResumeProgress<D::Error>, MaintenanceError<D::Error>> {
        self.resume_inner(options, u128::MAX, stop)
    }

    /// Resume a queued transaction under a cumulative native-work allowance.
    /// Native replacement layout bytes are admitted before the original storage
    /// callback. Refusal by that callback consumes no factory work credit;
    /// refunds require none. An accepted allocation attempt retains its charge.
    /// Use `begin` to include preparation in the grant. Initialize through
    /// `resume` under the original evaluator governors before this method.
    ///
    /// # Errors
    /// Retryable WorkLimit retains the original transaction and native frames.
    /// Source, storage and stop failures follow `resume`'s original contracts.
    pub fn resume_until(
        &mut self,
        options: EvalOptions,
        limit: u128,
        stop: Option<&dyn StopSignal>,
    ) -> Result<ResumeProgress<D::Error>, MaintenanceError<D::Error>> {
        if !self.prepared {
            return Err(MaintenanceError::InitialIncomplete);
        }
        self.resume_inner(options, limit, stop)
    }

    fn resume_inner(
        &mut self,
        options: EvalOptions,
        limit: u128,
        stop: Option<&dyn StopSignal>,
    ) -> Result<ResumeProgress<D::Error>, MaintenanceError<D::Error>> {
        if let Some(error) = &self.failure {
            return Err(MaintenanceError::Failed(error.clone()));
        }
        if self.preparing.is_some() {
            match self.prepare_until(limit, stop) {
                Ok(PreparationProgress::Ready) => {}
                Ok(PreparationProgress::Paused {
                    reason: PreparationPause::Stopped,
                    ..
                }) => {
                    return Ok(ResumeProgress::Paused(EvalError::Stopped {
                        report: self.budget(),
                    }));
                }
                Ok(PreparationProgress::Paused {
                    reason: PreparationPause::WorkLimit { limit, needed },
                    ..
                }) => {
                    return Err(MaintenanceError::WorkLimit {
                        limit,
                        consumed: self.preparation_work,
                        needed,
                    });
                }
                Err(MaintenanceError::Failed(error @ EvalError::Storage(_))) => {
                    return Ok(ResumeProgress::Paused(error));
                }
                Err(error) => return Err(error),
            }
        }
        if self.pending.is_none() && self.prepared {
            if let Err(error) = self
                .session
                .state
                .as_ref()
                .expect("completed model remains owned")
                .rel
                .interner()
                .check_read()
            {
                let error = EvalError::Source(error);
                self.failure = Some(error.clone());
                return Err(MaintenanceError::Failed(error));
            }
            return Ok(ResumeProgress::Complete);
        }
        let options = options.with_derivation_mode(super::DerivationMode::ModelOnly);
        if self.pending.is_none() {
            let progress = self
                .storage
                .with_memory(|memory| {
                    self.session
                        .resume_in_memory(self.session.binding(), options, stop, memory)
                })
                .map_err(MaintenanceError::Session)?;
            if progress == ResumeProgress::Complete {
                let state = self
                    .session
                    .state
                    .as_mut()
                    .expect("initial state remains owned");
                let before = self.storage.live();
                let preparation = self.storage.with_memory(|memory| {
                    state
                        .rel
                        .prepare_maintenance_indexes_in(memory)
                        .map_err(native_error)
                        .and_then(|()| {
                            self.initial_reclaim
                                .resume(state, &[], memory, |_| !is_stopped(stop))
                        })
                });
                state.account_native_memory(before, self.storage.live());
                if matches!(preparation, Ok(false)) {
                    return Ok(ResumeProgress::Paused(EvalError::Stopped {
                        report: state.report(),
                    }));
                }
                if let Err(error) = preparation {
                    if matches!(error, EvalError::Storage(_)) {
                        return Ok(ResumeProgress::Paused(error));
                    }
                    self.failure = Some(error.clone());
                    return Err(MaintenanceError::Failed(error));
                }
                self.session
                    .state
                    .as_mut()
                    .expect("initial state remains owned")
                    .native
                    .as_mut()
                    .expect("native capture is active")
                    .tracking = true;
                self.session
                    .state
                    .as_mut()
                    .expect("initial state remains owned")
                    .native
                    .as_mut()
                    .expect("native capture is active")
                    .native_heights = true;
                self.session
                    .state
                    .as_mut()
                    .expect("initial state remains owned")
                    .depth = Vec::new();
                self.prepared = true;
                self.initial_reclaim = reclaim::Frame::default();
            }
            return Ok(progress);
        }
        let consumed = self.transaction_work();
        if consumed > limit {
            return Err(MaintenanceError::WorkLimit {
                limit,
                consumed,
                needed: consumed,
            });
        }
        let state = self
            .session
            .state
            .as_mut()
            .expect("maintenance retains its model");
        state.limits = Limits::of(&options, state.input_terms);
        state.stated = super::BudgetReport::stated_limits(&options);
        state.execution = options.round_execution();
        let used = Cell::new(self.mutation_work);
        let meter = work::Meter::new(&used, self.preparation_work, limit, stop);
        let before = self.storage.live();
        let result = self.storage.with_memory(|memory| {
            meter.with_memory(memory, |memory| {
                advance(
                    &Run {
                        exe: self.session.exe,
                        guards: self.session.guards,
                        levels: &self.levels,
                    },
                    &mut self.weights,
                    self.pending.as_mut().expect("checked pending transaction"),
                    state,
                    &meter,
                    memory,
                )
            })
        });
        self.mutation_work = used.get();
        state.account_native_memory(before, self.storage.live());
        let result = match result {
            Err(EvalError::Storage(StorageError::AdmissionFailed)) if meter.refused() => {
                meter.paused(state.report())
            }
            result => result,
        };
        match result {
            Ok(false) => Err(meter.refusal()),
            Ok(true) => {
                let completed = self
                    .pending
                    .take()
                    .expect("the completed transaction is owned");
                self.delta = completed.delta;
                self.applied = Some(completed.batch);

                Ok(ResumeProgress::Complete)
            }
            Err(
                error @ (EvalError::BudgetExhausted { .. }
                | EvalError::Stopped { .. }
                | EvalError::Storage(_)
                | EvalError::TermLimitExceeded { .. }),
            ) => Ok(ResumeProgress::Paused(error)),
            Err(error) => {
                self.failure = Some(error.clone());
                Err(MaintenanceError::Failed(error))
            }
        }
    }
}

fn row_for<D: TermDictionary, const N: usize>(
    rel: &RelationStore<D, N>,
    terms: [D::Id; N],
) -> Result<Option<RowId>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    super::present_row(
        &HeadKey {
            positions: terms.map(HeadTerm::Interned),
        },
        rel,
    )
}

type Tasks<I, const N: usize> = OrderedTable<usize, WorkList<Task<I, N>, 8>>;

fn task_bucket_in<'a, I: TermIdentity, const N: usize, A: Admission + ?Sized>(
    tasks: &'a mut Tasks<I, N>,
    level: usize,
    memory: &mut Memory<'_, A>,
    retired: &mut usize,
) -> Result<&'a mut WorkList<Task<I, N>, 8>, StorageError> {
    if tasks.get(&level).is_none() {
        let before = memory.admitted_bytes();
        let old = tasks.storage_bytes()?;
        let result = tasks.insert(level, WorkList::new(), memory);
        if result.is_err() {
            record_factory_retirement(
                retired,
                before,
                old,
                tasks.storage_bytes()?,
                memory.admitted_bytes(),
            );
        }
        assert!(result?.is_none(), "a new original task level");
    }
    Ok(tasks
        .get_mut(&level)
        .expect("the original task level was admitted"))
}

fn reserve_work_in<T, A: Admission + ?Sized>(
    work: &mut WorkList<T, 8>,
    memory: &mut Memory<'_, A>,
    retired: &mut usize,
    reserve: impl FnOnce(&mut WorkList<T, 8>, &mut Memory<'_, A>) -> Result<(), StorageError>,
) -> Result<(), StorageError> {
    let before = memory.admitted_bytes();
    let old = work
        .heap_capacity()
        .checked_mul(size_of::<T>())
        .expect("an original spill layout");
    let result = reserve(work, memory);
    if result.is_err() {
        let new = work
            .heap_capacity()
            .checked_mul(size_of::<T>())
            .expect("a native spill layout");
        record_factory_retirement(retired, before, old, new, memory.admitted_bytes());
    }
    result
}

fn push_task<I: TermIdentity, E, const N: usize, A: Admission + ?Sized>(
    tasks: &mut Tasks<I, N>,
    level: usize,
    task: Task<I, N>,
    memory: &mut Memory<'_, A>,
    retired: &mut usize,
) -> Result<(), EvalError<E>> {
    let work = task_bucket_in(tasks, level, memory, retired).map_err(EvalError::Storage)?;
    reserve_work_in(work, memory, retired, |work, memory| {
        work.try_push_admitted(task, memory)
    })
    .map_err(EvalError::Storage)
}

struct Run<'a, 'r, D: TermDictionary, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    exe: &'r Executable<N>,
    guards: &'a dyn GuardEvaluator<D, N>,
    levels: &'a [usize],
}

fn advance<'r, M, S: Admission, D: TermDictionary, const N: usize, A: Admission + ?Sized>(
    run: &Run<'_, 'r, D, N>,
    weights: &mut FastMap<[D::Id; N], i64>,
    pending: &mut Pending<'r, M, S, D, N>,
    state: &mut FixpointState<D, N>,
    meter: &work::Meter<'_, '_>,
    memory: &mut Memory<'_, A>,
) -> Result<bool, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    let &Run {
        exe,
        guards,
        levels,
    } = run;
    let stop = meter.stop();
    check_budget(state)?;
    if is_stopped(stop) {
        return Err(EvalError::Stopped {
            report: state.report(),
        });
    }
    state
        .native
        .as_mut()
        .expect("native proof owner")
        .flush_retired(memory)
        .map_err(EvalError::Storage)?;
    flush_retired_bytes(&mut pending.retired_bytes, memory).map_err(EvalError::Storage)?;
    state
        .rel
        .flush_metadata_retirement_in(memory)
        .map_err(EvalError::Storage)?;
    if pending.phase == MutationPhase::Complete {
        state
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)?;
        return Ok(true);
    }
    while let Some(&(terms, weight)) = pending.changes.get(pending.installed_weights) {
        if !meter.allow(1) {
            return meter.paused(state.report());
        }
        if weight == 0 {
            weights.remove(&terms);
        } else {
            weights.insert(terms, weight);
        }
        pending.installed_weights += 1;
    }
    while let Some(&(row, terms)) = pending.delete.get(pending.deletion) {
        let sources = state
            .native
            .as_ref()
            .expect("native proof owner")
            .nodes
            .get(&row)
            .map_or(0, |node| node.proof.sources.len());
        let cost = sources
            .checked_add(N + 1)
            .ok_or(EvalError::Storage(StorageError::SizeOverflow))?;
        let native = state.native.as_mut().expect("native proof owner");
        native.reserve_terms(memory).map_err(EvalError::Storage)?;
        reserve_index_in(
            &mut native.withdrawn,
            &mut native.withdrawn_bytes,
            1,
            memory,
            &mut native.retired_bytes,
        )
        .map_err(EvalError::Storage)?;
        reserve_index_in(
            &mut native.membership,
            &mut native.membership_bytes,
            1,
            memory,
            &mut native.retired_bytes,
        )
        .map_err(EvalError::Storage)?;
        if !meter.allow(cost) {
            return meter.paused(state.report());
        }
        let removed = state
            .rel
            .withdraw_ids(terms)
            .map_err(EvalError::Source)?
            .expect("the chosen original row remains live before withdrawal");
        assert_eq!(
            removed.id(),
            row,
            "over-deletion retains original row handles"
        );
        native.withdrawn.insert(terms, removed);
        native.membership(terms, -1);
        native.remove(row);
        pending.deletion += 1;
        native.flush_retired(memory).map_err(EvalError::Storage)?;
    }
    while let Some(&(terms, weight)) = pending.changes.get(pending.input) {
        if weight <= 0 {
            if !meter.allow(1) {
                return meter.paused(state.report());
            }
            pending.input += 1;
            continue;
        }
        if pending.input_state.is_none() {
            if !meter.allow(1) {
                return meter.paused(state.report());
            }
            let existing = row_for(&state.rel, terms).map_err(EvalError::Source)?;
            let old_depth = existing.map(|row| {
                state
                    .native
                    .as_ref()
                    .expect("native proof owner")
                    .height(row)
            });
            pending.input_state = Some(InputState {
                existing,
                changes_proof: existing.is_none() || old_depth != Some(0),
                phase: InputPhase::Reserve,
            });
        }
        let input = pending
            .input_state
            .as_mut()
            .expect("retained original input lookup");
        if input.changes_proof {
            if input.phase == InputPhase::Reserve {
                let native = state.native.as_mut().expect("native proof owner");
                native.reserve_terms(memory).map_err(EvalError::Storage)?;
                reserve_index_in(
                    &mut native.membership,
                    &mut native.membership_bytes,
                    1,
                    memory,
                    &mut native.retired_bytes,
                )
                .map_err(EvalError::Storage)?;
                if !meter.allow(N + 1) {
                    return meter.paused(state.report());
                }
                input.phase = InputPhase::Event;
            }
            if input.phase == InputPhase::Event {
                let event = pending
                    .event
                    .get_or_insert_with(|| event::Frame::new(terms));
                if !event.resume(
                    &mut event::Inputs {
                        exe,
                        levels,
                        tasks: &mut pending.tasks,
                        rel: &state.rel,
                    },
                    memory,
                    &mut pending.retired_bytes,
                    |cost| meter.allow(cost),
                )? {
                    return meter.paused(state.report());
                }
                input.phase = InputPhase::Publication;
                let completed = pending.event.take().expect("completed original event");
                pending.retired_bytes = pending
                    .retired_bytes
                    .checked_add(completed.retire())
                    .expect("retired event layout fits its original grant");
                flush_retired_bytes(&mut pending.retired_bytes, memory)
                    .map_err(EvalError::Storage)?;
            }
            let sources = input
                .existing
                .and_then(|row| {
                    state
                        .native
                        .as_ref()
                        .expect("native proof owner")
                        .nodes
                        .get(&row)
                })
                .map_or(0, |node| node.proof.sources.len());
            let cost = sources
                .checked_add(N + 1)
                .ok_or(EvalError::Storage(StorageError::SizeOverflow))?;
            if input.phase == InputPhase::Publication {
                if !meter.allow(cost) {
                    return meter.paused(state.report());
                }
                input.phase = InputPhase::Publish;
            }
            let row = if let Some(row) = input.existing {
                row
            } else {
                state
                    .report()
                    .admit_fact()
                    .map_err(|report| EvalError::BudgetExhausted {
                        resource: super::BudgetResource::StoredFacts,
                        report,
                    })?;
                assert!(
                    !state
                        .native
                        .as_ref()
                        .expect("native proof owner")
                        .withdrawn
                        .contains_key(&terms),
                    "preparation preserves every row whose resulting asserted weight is positive"
                );
                state
                    .rel
                    .insert_terms_in(terms.map(DictionaryTerm::Id), memory)
                    .map_err(native_error)?
                    .expect("absent input inserts")
                    .2
            };
            let native = state.native.as_mut().expect("native proof owner");
            native.remove(row);
            if input.existing.is_none() {
                native.membership(terms, 1);
            }
        }
        pending.input += 1;
        pending.input_state = None;
        state
            .native
            .as_mut()
            .expect("native proof owner")
            .flush_retired(memory)
            .map_err(EvalError::Storage)?;
        state
            .rel
            .flush_metadata_retirement_in(memory)
            .map_err(EvalError::Storage)?;
    }

    while pending.phase == MutationPhase::Tasks {
        check_budget(state)?;
        if is_stopped(stop) {
            return Err(EvalError::Stopped {
                report: state.report(),
            });
        }
        if let Some(publication) = pending.publication.as_mut() {
            if !publication.prepare(state, memory, &mut pending.retired_bytes, |cost| {
                meter.allow(cost)
            })? || !publication.advance_until(state, memory, |cost| meter.allow(cost))?
            {
                return meter.paused(state.report());
            }
            let completed = pending
                .publication
                .take()
                .expect("completed original publication");
            pending.retired_bytes = pending
                .retired_bytes
                .checked_add(completed.retire())
                .expect("retired publication layouts fit their original grant");
            flush_retired_bytes(&mut pending.retired_bytes, memory).map_err(EvalError::Storage)?;
        }
        // One event is retained until all dependent native tasks are admitted.
        while let Some(&(_, terms)) = state
            .native
            .as_ref()
            .expect("native proof owner")
            .changed
            .last()
        {
            let event = pending
                .event
                .get_or_insert_with(|| event::Frame::new(terms));
            if !event.resume(
                &mut event::Inputs {
                    exe,
                    levels,
                    tasks: &mut pending.tasks,
                    rel: &state.rel,
                },
                memory,
                &mut pending.retired_bytes,
                |cost| meter.allow(cost),
            )? {
                return meter.paused(state.report());
            }
            state
                .native
                .as_mut()
                .expect("native proof owner")
                .changed
                .pop();
            let completed = pending.event.take().expect("completed original event");
            pending.retired_bytes = pending
                .retired_bytes
                .checked_add(completed.retire())
                .expect("retired event layout fits its original grant");
            flush_retired_bytes(&mut pending.retired_bytes, memory).map_err(EvalError::Storage)?;
        }
        if let Some(frame) = pending.active.as_mut() {
            let before = frame.join_steps();
            let allowance = state.allowance();
            let granted = allowance.min(meter.remaining_joins());
            let limit = before.saturating_add(granted);
            let result = frame.resume(state, guards, limit, stop);
            let consumed = frame.join_steps() - before;
            state.join_steps = state.join_steps.saturating_add(consumed);
            meter.account_joins(consumed);
            let progress = result.map_err(|error| match error {
                probe::ProbeError::Failed(error) => error,
                _ => unreachable!("private maintenance tasks use validated original coordinates"),
            })?;
            if let ResumeProgress::Paused(error) = progress {
                if matches!(
                    error,
                    EvalError::BudgetExhausted {
                        resource: super::BudgetResource::JoinSteps,
                        ..
                    }
                ) && granted < allowance
                {
                    meter.refuse_join();
                    return meter.paused(state.report());
                }
                return Err(match error {
                    EvalError::BudgetExhausted { resource, .. } => {
                        let mut report = state.report();
                        report.join_steps = report.join_steps.saturating_add(1);
                        EvalError::BudgetExhausted { resource, report }
                    }
                    EvalError::Stopped { .. } => EvalError::Stopped {
                        report: state.report(),
                    },
                    error => error,
                });
            }
            // Admission precedes consuming the completed native output; refusal
            // leaves that exact original frame available for the next call.
            if !pending.output_work_admitted {
                if !meter.allow(1) {
                    return meter.paused(state.report());
                }
                pending.output_work_admitted = true;
            }
            let slot = commit::Frame::reserve_deferred(memory, &mut pending.retired_bytes)
                .map_err(EvalError::Storage)?;
            let output = pending
                .active
                .take()
                .expect("completed native task")
                .into_output();
            pending.publication = Some(commit::Frame::deferred(output, slot));
            pending.output_work_admitted = false;
            continue;
        }
        if pending.task.is_none() {
            if !meter.allow(1) {
                return meter.paused(state.report());
            }
            let Some((&level, _)) = pending.tasks.first_key_value() else {
                pending.phase = MutationPhase::Settlement;
                break;
            };
            let Some(task) = pending
                .tasks
                .get_mut(&level)
                .expect("the first native task level")
                .pop()
            else {
                let (_, work) = pending
                    .tasks
                    .pop_first()
                    .expect("the emptied native task level");
                let bytes = work
                    .heap_capacity()
                    .checked_mul(size_of::<Task<D::Id, N>>())
                    .expect("the original task spill has a valid layout");
                drop(work);
                pending.retired_bytes = pending
                    .retired_bytes
                    .checked_add(bytes)
                    .expect("retired task spills fit their original grant");
                flush_retired_bytes(&mut pending.retired_bytes, memory)
                    .map_err(EvalError::Storage)?;
                continue;
            };
            pending.task = Some(task);
        }
        let task = pending
            .task
            .expect("the original task remains owned until entry succeeds");
        let frame = match task {
            Task::Body { rule, body, terms } => {
                probe::Frame::from_body(exe, state, rule, body, terms)?
            }
            Task::Head { rule, head, terms } => probe::Frame::new(exe, state, rule, head, terms)
                .map_err(|e| match e {
                    probe::ProbeError::Failed(error) => error,
                    _ => unreachable!("re-grounding uses original authored head coordinates"),
                })?,
            Task::Absence { rule } => probe::Frame::all(exe, state, rule),
        };
        pending.active = Some(frame);
        pending.task = None;
    }
    if !pending.withdrawals_cleared {
        let native = state.native.as_mut().expect("native proof owner");
        let cost = native
            .withdrawn_bytes
            .checked_add(1)
            .ok_or(EvalError::Storage(StorageError::SizeOverflow))?;
        if !meter.allow(cost) {
            return meter.paused(state.report());
        }
        native.withdrawn.clear();
        pending.withdrawals_cleared = true;
    }
    while let Some(&(_, terms)) = pending.delete.get(pending.retirement) {
        if !meter.allow(1) {
            return meter.paused(state.report());
        }
        state
            .rel
            .retire_empty_partition(terms)
            .map_err(native_error)?;
        pending.retirement += 1;
    }
    if !pending.delta_gathered {
        let native = state.native.as_ref().expect("native proof owner");
        let cost = native
            .membership_bytes
            .checked_add(1)
            .ok_or(EvalError::Storage(StorageError::SizeOverflow))?;
        reserve_array_in(
            &mut pending.delta,
            memory,
            &mut pending.retired_bytes,
            |values, memory| memory.reserve(values, native.membership.len()),
        )
        .map_err(EvalError::Storage)?;
        if !meter.allow(cost) {
            return meter.paused(state.report());
        }
        pending.delta.extend(
            native
                .membership
                .iter()
                .map(|(&terms, &weight)| (terms, weight)),
        );
        pending.delta_sort = Some(purrdf_lex::walk::HeapSortFrame::new(pending.delta.len()));
        pending.delta_gathered = true;
    }
    if !pending
        .delta_sort
        .as_mut()
        .expect("retained model-delta sorter")
        .resume(
            &mut pending.delta,
            |(a, _), (b, _)| {
                state
                    .rel
                    .interner()
                    .compare_rows(&a.map(DictionaryTerm::Id), &b.map(DictionaryTerm::Id))
            },
            || meter.allow(1),
        )
        .map_err(EvalError::Source)?
    {
        return meter.paused(state.report());
    }
    if !pending
        .reclaim
        .resume(state, &pending.delta, memory, |cost| meter.allow(cost))?
    {
        return meter.paused(state.report());
    }
    let native = state.native.as_mut().expect("native proof owner");
    native.flush_retired(memory).map_err(EvalError::Storage)?;
    flush_retired_bytes(&mut pending.retired_bytes, memory).map_err(EvalError::Storage)?;
    let cost = native
        .membership_bytes
        .checked_add(1)
        .ok_or(EvalError::Storage(StorageError::SizeOverflow))?;
    if !meter.allow(cost) {
        return meter.paused(state.report());
    }
    native.membership.clear();
    assert!(
        pending.tasks.is_empty(),
        "settled native work has no queued task"
    );
    let task_bytes = pending.tasks.storage_bytes().map_err(EvalError::Storage)?;
    state
        .rel
        .interner()
        .check_read()
        .map_err(EvalError::Source)?;
    // Every native result and source check is complete. Keep this state across
    // every final release so retries cannot clear or charge membership twice.
    pending.phase = MutationPhase::Complete;
    let tasks = std::mem::take(&mut pending.tasks);
    drop(tasks);
    pending.retired_bytes = pending
        .retired_bytes
        .checked_add(task_bytes)
        .expect("retired task arenas fit their original grant");
    let changes = std::mem::take(&mut pending.changes);
    let delete = std::mem::take(&mut pending.delete);
    let altered = std::mem::take(&mut pending.altered);
    let reached = std::mem::take(&mut pending.reached);
    let bytes = changes.capacity() * size_of::<([D::Id; N], i64)>();
    let bytes = bytes
        .checked_add(delete.capacity() * size_of::<(RowId, [D::Id; N])>())
        .and_then(|bytes| bytes.checked_add(altered.retained))
        .and_then(|bytes| bytes.checked_add(reached.capacity() * size_of::<bool>()))
        .expect("settled preparation layouts fit their original grant");
    drop(changes);
    drop(delete);
    drop(altered);
    drop(reached);
    pending.retired_bytes = pending
        .retired_bytes
        .checked_add(bytes)
        .expect("settled retirement fits its original grant");
    flush_retired_bytes(&mut pending.retired_bytes, memory).map_err(EvalError::Storage)?;
    state
        .rel
        .flush_metadata_retirement_in(memory)
        .map_err(EvalError::Storage)?;
    Ok(true)
}

fn native_error<E>(error: DictionaryError<E>) -> EvalError<E> {
    match error {
        DictionaryError::Source(error) => EvalError::Source(error),
        DictionaryError::Storage(error) => EvalError::Storage(error),
    }
}
