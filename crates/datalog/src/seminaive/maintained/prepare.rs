// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Retained transaction validation, dependency discovery and re-grounding preparation.

use super::{
    Admission, Arity, BTreeMap, BTreeSet, ChangeBatch, DictionaryTerm, EvalError, Executable,
    FastMap, FastSet, FixpointState, Impacts, MaintenanceError, Pending, RowId, RowSet,
    StorageError, StoreShape, Task, TermDictionary, TermIdentity, WorkList, push_task, reclaim,
    row_for, storage_failure,
};
use purrdf_lex::walk::HeapSortFrame;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Reserve,
    Validate,
    SortChanges,
    ReserveIndexes,
    Seeds,
    Fanout,
    ReachReserve,
    ReachSeeds,
    ReachEdges,
    Absence,
    ClosurePop,
    ClosureVisit,
    Gather,
    SortDelete,
    Heads,
    Complete,
}

#[derive(Clone, Copy, Debug)]
enum Bucket {
    Premise(RowId),
    Producer(usize),
}

pub(super) struct Inputs<'a, D: TermDictionary, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    pub(super) exe: &'a Executable<N>,
    pub(super) impacts: &'a Impacts,
    pub(super) levels: &'a [usize],
    pub(super) weights: &'a mut FastMap<[D::Id; N], i64>,
    pub(super) state: &'a FixpointState<D, N>,
}

pub(super) struct Frame<M, S: Admission, I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    batch: ChangeBatch<M, S, I, N>,
    changes: Vec<([I; N], i64)>,
    delete: Vec<(RowId, [I; N])>,
    deleted: FastMap<RowId, [I; N]>,
    roots: WorkList<(RowId, [I; N]), 8>,
    altered: FastSet<I>,
    reached: Vec<bool>,
    absence: BTreeSet<usize>,
    tasks: BTreeMap<usize, WorkList<Task<I, N>, 8>>,
    phase: Phase,
    cursor: usize,
    rule: usize,
    head: usize,
    extended: bool,
    bucket: Option<Bucket>,
    after_fanout: Phase,
    current: Option<(RowId, [I; N])>,
    sort: Option<HeapSortFrame>,
}

impl<M, S: Admission, I: TermIdentity, const N: usize> Frame<M, S, I, N>
where
    Arity<N>: StoreShape<N>,
{
    pub(super) fn new(batch: ChangeBatch<M, S, I, N>) -> Self {
        Self {
            batch,
            changes: Vec::new(),
            delete: Vec::new(),
            deleted: FastMap::default(),
            roots: WorkList::new(),
            altered: FastSet::default(),
            reached: Vec::new(),
            absence: BTreeSet::new(),
            tasks: BTreeMap::new(),
            phase: Phase::Reserve,
            cursor: 0,
            rule: 0,
            head: 0,
            extended: false,
            bucket: None,
            after_fanout: Phase::Seeds,
            current: None,
            sort: None,
        }
    }

    /// The original committed position remains owned throughout preparation.
    pub(super) fn watermark(&self) -> &M {
        self.batch.watermark()
    }

    /// Each admitted unit completes once before the next state transition.
    /// A hash-bucket fan-out admits its retained capacity before scanning it;
    /// its complete output is reserved before publishing any work-list member.
    pub(super) fn resume<D: TermDictionary<Id = I>>(
        &mut self,
        inputs: &mut Inputs<'_, D, N>,
        mut allow: impl FnMut(usize) -> bool,
    ) -> Result<bool, MaintenanceError<D::Error>> {
        let rel = &inputs.state.rel;
        let dictionary = rel.interner();
        dictionary.check_read().map_err(source_error)?;
        let native = inputs
            .state
            .native
            .as_ref()
            .expect("maintenance retains native chosen proofs");
        loop {
            match self.phase {
                Phase::Reserve => {
                    if !allow(1) {
                        return Ok(false);
                    }
                    self.changes
                        .try_reserve_exact(self.batch.changes().len())
                        .map_err(storage_failure)?;
                    self.phase = Phase::Validate;
                }
                Phase::Validate => {
                    let Some(&(terms, delta)) = self.batch.changes().get(self.cursor) else {
                        self.sort = Some(HeapSortFrame::new(self.changes.len()));
                        self.phase = Phase::SortChanges;
                        continue;
                    };
                    if !allow(1) {
                        return Ok(false);
                    }
                    let weight = i128::from(inputs.weights.get(&terms).copied().unwrap_or(0))
                        + i128::from(delta);
                    if !(0..=i128::from(i64::MAX)).contains(&weight) {
                        return Err(MaintenanceError::Multiplicity { weight });
                    }
                    self.changes.push((
                        terms,
                        i64::try_from(weight).expect("validated exact multiplicity"),
                    ));
                    self.cursor += 1;
                }
                Phase::SortChanges => {
                    if !self
                        .sort
                        .as_mut()
                        .expect("retained change sorter")
                        .resume(
                            &mut self.changes,
                            |(a, _), (b, _)| {
                                dictionary.compare_rows(
                                    &a.map(DictionaryTerm::Id),
                                    &b.map(DictionaryTerm::Id),
                                )
                            },
                            || allow(1),
                        )
                        .map_err(source_error)?
                    {
                        return Ok(false);
                    }
                    self.sort = None;
                    self.cursor = 0;
                    self.phase = Phase::ReserveIndexes;
                }
                Phase::ReserveIndexes => {
                    if !allow(1) {
                        return Ok(false);
                    }
                    inputs
                        .weights
                        .try_reserve(self.changes.len())
                        .map_err(storage_failure)?;
                    self.altered
                        .try_reserve(self.changes.len())
                        .map_err(storage_failure)?;
                    self.phase = Phase::Seeds;
                }
                Phase::Seeds => {
                    let Some(&(terms, weight)) = self.changes.get(self.cursor) else {
                        self.phase = Phase::ReachReserve;
                        continue;
                    };
                    if !allow(1) {
                        return Ok(false);
                    }
                    let old = inputs.weights.get(&terms).copied().unwrap_or(0);
                    if (old == 0) != (weight == 0) {
                        self.altered.insert(terms[1]);
                        if let Some(row) = row_for(rel, terms).map_err(source_error)? {
                            if weight == 0 {
                                self.roots.try_push((row, terms)).map_err(storage_failure)?;
                            } else {
                                self.bucket = Some(Bucket::Premise(row));
                                self.after_fanout = Phase::Seeds;
                                self.phase = Phase::Fanout;
                            }
                        }
                    }
                    self.cursor += 1;
                }
                Phase::Fanout => {
                    let rows = match self.bucket.expect("retained original dependency bucket") {
                        Bucket::Premise(row) => native.reverse.get(&row),
                        Bucket::Producer(rule) => native.producers.get(&rule),
                    };
                    let capacity = rows.map_or(0, RowSet::scan_work).checked_add(1).ok_or(
                        MaintenanceError::Failed(EvalError::Storage(StorageError::SizeOverflow)),
                    )?;
                    if !allow(capacity) {
                        return Ok(false);
                    }
                    let count = rows.map_or(0, RowSet::len);
                    let required =
                        self.roots
                            .len()
                            .checked_add(count)
                            .ok_or(MaintenanceError::Failed(EvalError::Storage(
                                StorageError::SizeOverflow,
                            )))?;
                    self.roots
                        .try_reserve_heap_exact(required.saturating_sub(8))
                        .map_err(storage_failure)?;
                    if let Some(rows) = rows {
                        for row in rows {
                            self.roots.push((*row, native.nodes[row].terms));
                        }
                    }
                    self.bucket = None;
                    self.phase = self.after_fanout;
                }
                Phase::ReachReserve => {
                    let count = inputs.impacts.predicates.len();
                    let cost = count.checked_add(1).ok_or(MaintenanceError::Failed(
                        EvalError::Storage(StorageError::SizeOverflow),
                    ))?;
                    if !allow(cost) {
                        return Ok(false);
                    }
                    self.reached
                        .try_reserve_exact(count)
                        .map_err(storage_failure)?;
                    self.reached.resize(count, false);
                    self.cursor = 0;
                    self.phase = if self.altered.is_empty() {
                        Phase::Absence
                    } else {
                        Phase::ReachSeeds
                    };
                }
                Phase::ReachSeeds => {
                    let Some(predicate) = inputs.impacts.predicates.get(self.cursor) else {
                        self.cursor = 0;
                        self.phase = Phase::ReachEdges;
                        continue;
                    };
                    if !allow(1) {
                        return Ok(false);
                    }
                    self.reached[self.cursor] = predicate == crate::plan::ANY_PREDICATE
                        || dictionary
                            .lookup_term(predicate)
                            .map_err(source_error)?
                            .is_some_and(|id| self.altered.contains(&id));
                    self.cursor += 1;
                }
                Phase::ReachEdges => {
                    let Some(edge) = inputs.impacts.edges.get(self.cursor) else {
                        self.cursor = 0;
                        if self.extended {
                            self.extended = false;
                        } else {
                            self.phase = Phase::Absence;
                        }
                        continue;
                    };
                    if !allow(1) {
                        return Ok(false);
                    }
                    if self.reached[edge.body] {
                        self.extended |= !self.reached[edge.head];
                        self.reached[edge.head] = true;
                        if edge.negative
                            && let Some(rule) = edge.rule
                        {
                            self.absence.insert(rule);
                        }
                    }
                    self.cursor += 1;
                }
                Phase::Absence => {
                    if self.cursor == inputs.exe.rule_count() {
                        self.phase = Phase::ClosurePop;
                        continue;
                    }
                    if !allow(1) {
                        return Ok(false);
                    }
                    let rule = self.cursor;
                    if self.absence.contains(&rule) {
                        push_task(&mut self.tasks, inputs.levels[rule], Task::Absence { rule })
                            .map_err(MaintenanceError::Failed)?;
                        self.bucket = Some(Bucket::Producer(rule));
                        self.after_fanout = Phase::Absence;
                        self.phase = Phase::Fanout;
                    }
                    self.cursor += 1;
                }
                Phase::ClosurePop => {
                    if !allow(1) {
                        return Ok(false);
                    }
                    self.current = self.roots.pop();
                    self.phase = if self.current.is_some() {
                        Phase::ClosureVisit
                    } else {
                        Phase::Gather
                    };
                }
                Phase::ClosureVisit => {
                    if !allow(1) {
                        return Ok(false);
                    }
                    let (row, terms) = self
                        .current
                        .expect("the original closure row remains owned");
                    let old = inputs.weights.get(&terms).copied().unwrap_or(0);
                    let weight = self
                        .batch
                        .changes()
                        .binary_search_by_key(&terms, |(ids, _)| *ids)
                        .map_or(old, |index| {
                            i64::try_from(
                                i128::from(old) + i128::from(self.batch.changes()[index].1),
                            )
                            .expect("every original batch multiplicity was validated")
                        });
                    if weight == 0 && !self.deleted.contains_key(&row) {
                        self.deleted.try_reserve(1).map_err(storage_failure)?;
                        self.deleted.insert(row, terms);
                        self.bucket = Some(Bucket::Premise(row));
                        self.after_fanout = Phase::ClosurePop;
                        self.phase = Phase::Fanout;
                    } else {
                        self.phase = Phase::ClosurePop;
                    }
                    self.current = None;
                }
                Phase::Gather => {
                    let cost = purrdf_core::hash::hash_table_allocation_bound::<(RowId, [I; N])>(
                        self.deleted.capacity(),
                    )
                    .and_then(|bytes| bytes.checked_add(1))
                    .ok_or(MaintenanceError::Failed(EvalError::Storage(
                        StorageError::SizeOverflow,
                    )))?;
                    if !allow(cost) {
                        return Ok(false);
                    }
                    self.delete
                        .try_reserve_exact(self.deleted.len())
                        .map_err(storage_failure)?;
                    self.delete.extend(self.deleted.drain());
                    self.deleted = FastMap::default();
                    self.sort = Some(HeapSortFrame::new(self.delete.len()));
                    self.phase = Phase::SortDelete;
                }
                Phase::SortDelete => {
                    if !self
                        .sort
                        .as_mut()
                        .expect("retained deletion sorter")
                        .resume(
                            &mut self.delete,
                            |(_, a), (_, b)| {
                                dictionary.compare_rows(
                                    &a.map(DictionaryTerm::Id),
                                    &b.map(DictionaryTerm::Id),
                                )
                            },
                            || allow(1),
                        )
                        .map_err(source_error)?
                    {
                        return Ok(false);
                    }
                    self.sort = None;
                    self.cursor = 0;
                    self.phase = Phase::Heads;
                }
                Phase::Heads => {
                    if self.cursor == self.delete.len() || inputs.exe.rule_count() == 0 {
                        if !allow(1) {
                            return Ok(false);
                        }
                        dictionary.check_read().map_err(source_error)?;
                        self.phase = Phase::Complete;
                        continue;
                    }
                    if self.rule == inputs.exe.rule_count() {
                        self.rule = 0;
                        self.cursor += 1;
                        continue;
                    }
                    if !allow(1) {
                        return Ok(false);
                    }
                    let runtime = &inputs.exe.runtimes()[self.rule];
                    if self.head == runtime.head.len() {
                        self.head = 0;
                        self.rule += 1;
                        continue;
                    }
                    push_task(
                        &mut self.tasks,
                        inputs.levels[self.rule],
                        Task::Head {
                            rule: self.rule,
                            head: self.head,
                            terms: self.delete[self.cursor].1,
                        },
                    )
                    .map_err(MaintenanceError::Failed)?;
                    self.head += 1;
                }
                Phase::Complete => return Ok(true),
            }
        }
    }

    pub(super) fn finish<'r, D: TermDictionary<Id = I>>(self) -> Pending<'r, M, S, D, N> {
        assert_eq!(
            self.phase,
            Phase::Complete,
            "only complete preparation publishes native tasks"
        );
        Pending {
            batch: self.batch,
            changes: self.changes,
            delete: self.delete,
            deletion: 0,
            input: 0,
            input_state: None,
            event: None,
            installed_weights: 0,
            settling: false,
            withdrawals_cleared: false,
            tasks: self.tasks,
            active: None,
            task: None,
            publication: None,
            retirement: 0,
            delta_gathered: false,
            delta_sort: None,
            reclaim: reclaim::Frame::default(),
            delta: Vec::new(),
            altered: self.altered,
            reached: self.reached,
        }
    }
}

fn source_error<E>(error: E) -> MaintenanceError<E> {
    MaintenanceError::Failed(EvalError::Source(error))
}
