// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Retained transaction validation, dependency discovery and re-grounding preparation.

use super::{
    Admission, Arity, ChangeBatch, DictionaryTerm, EvalError, Executable, FastMap, FixpointState,
    Impacts, MaintenanceError, Memory, Pending, RowId, RowSet, StorageError, StoreShape, Task,
    Tasks, TermDictionary, TermIdentity, WorkList, flush_retired_bytes, push_task, reclaim,
    reserve_array_in, reserve_index_in, reserve_work_in, row_for,
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
    pub(super) weights_bytes: &'a mut usize,
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
    deleted_bytes: usize,
    roots: WorkList<(RowId, [I; N]), 8>,
    altered: RowSet<I>,
    reached: Vec<bool>,
    absence: Vec<bool>,
    pub(super) tasks: Tasks<I, N>,
    retired_bytes: usize,
    phase: Phase,
    cursor: usize,
    rule: usize,
    head: usize,
    extended: bool,
    bucket: Option<Bucket>,
    after_fanout: Phase,
    current: Option<(RowId, [I; N])>,
    sort: Option<HeapSortFrame>,
    unit_admitted: bool,
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
            deleted_bytes: 0,
            roots: WorkList::new(),
            altered: RowSet::default(),
            reached: Vec::new(),
            absence: Vec::new(),
            tasks: Tasks::default(),
            retired_bytes: 0,
            phase: Phase::Reserve,
            cursor: 0,
            rule: 0,
            head: 0,
            extended: false,
            bucket: None,
            after_fanout: Phase::Seeds,
            current: None,
            sort: None,
            unit_admitted: false,
        }
    }

    /// The original committed position remains owned throughout preparation.
    pub(super) fn watermark(&self) -> &M {
        self.batch.watermark()
    }

    /// One phase iteration may retain multiple successful factories before the
    /// next factory refuses. Its logical operation credit is consumed only once.
    fn allow_unit(&mut self, cost: usize, allow: &mut impl FnMut(usize) -> bool) -> bool {
        if self.unit_admitted {
            return allow(0); // A fresh stop signal still applies to a retained unit.
        }
        if !allow(cost) {
            return false;
        }
        self.unit_admitted = true;
        true
    }

    pub(super) fn reject(self) -> usize {
        assert_eq!(self.phase, Phase::Validate);
        let bytes = self.changes.capacity() * size_of::<([I; N], i64)>();
        let retired = self
            .retired_bytes
            .checked_add(bytes)
            .expect("rejected change storage fits its original grant");
        drop(self);
        retired
    }

    /// Each admitted unit completes once before the next state transition.
    /// A hash-bucket fan-out admits its retained capacity before scanning it;
    /// its complete output is reserved before publishing any work-list member.
    pub(super) fn resume<D: TermDictionary<Id = I>, A: Admission + ?Sized>(
        &mut self,
        inputs: &mut Inputs<'_, D, N>,
        memory: &mut Memory<'_, A>,
        mut allow: impl FnMut(usize) -> bool,
    ) -> Result<bool, MaintenanceError<D::Error>> {
        flush_retired_bytes(&mut self.retired_bytes, memory)
            .map_err(|error| MaintenanceError::Failed(EvalError::Storage(error)))?;
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
                    if !self.allow_unit(1, &mut allow) {
                        return Ok(false);
                    }
                    reserve_array_in(
                        &mut self.changes,
                        memory,
                        &mut self.retired_bytes,
                        |values, memory| memory.reserve(values, self.batch.changes().len()),
                    )
                    .map_err(storage_error)?;
                    self.phase = Phase::Validate;
                }
                Phase::Validate => {
                    let Some(&(terms, delta)) = self.batch.changes().get(self.cursor) else {
                        self.sort = Some(HeapSortFrame::new(self.changes.len()));
                        self.phase = Phase::SortChanges;
                        continue;
                    };
                    if !self.allow_unit(1, &mut allow) {
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
                    if !self.allow_unit(1, &mut allow) {
                        return Ok(false);
                    }
                    reserve_index_in(
                        inputs.weights,
                        inputs.weights_bytes,
                        self.changes.len(),
                        memory,
                        &mut self.retired_bytes,
                    )
                    .map_err(storage_error)?;
                    self.altered
                        .reserve_in(self.changes.len(), memory, &mut self.retired_bytes)
                        .map_err(storage_error)?;
                    self.phase = Phase::Seeds;
                }
                Phase::Seeds => {
                    let Some(&(terms, weight)) = self.changes.get(self.cursor) else {
                        self.phase = Phase::ReachReserve;
                        continue;
                    };
                    if !self.allow_unit(1, &mut allow) {
                        return Ok(false);
                    }
                    let old = inputs.weights.get(&terms).copied().unwrap_or(0);
                    if (old == 0) != (weight == 0) {
                        self.altered.insert(terms[1]);
                        if let Some(row) = row_for(rel, terms).map_err(source_error)? {
                            if weight == 0 {
                                reserve_work_in(
                                    &mut self.roots,
                                    memory,
                                    &mut self.retired_bytes,
                                    |roots, memory| roots.try_push_admitted((row, terms), memory),
                                )
                                .map_err(storage_error)?;
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
                    if !self.allow_unit(capacity, &mut allow) {
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
                    reserve_work_in(
                        &mut self.roots,
                        memory,
                        &mut self.retired_bytes,
                        |roots, memory| {
                            roots.try_reserve_heap_admitted(required.saturating_sub(8), memory)
                        },
                    )
                    .map_err(storage_error)?;
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
                    if !self.allow_unit(cost, &mut allow) {
                        return Ok(false);
                    }
                    reserve_array_in(
                        &mut self.reached,
                        memory,
                        &mut self.retired_bytes,
                        |values, memory| memory.reserve(values, count),
                    )
                    .map_err(storage_error)?;
                    reserve_array_in(
                        &mut self.absence,
                        memory,
                        &mut self.retired_bytes,
                        |values, memory| memory.reserve(values, inputs.exe.rule_count()),
                    )
                    .map_err(storage_error)?;
                    self.reached.resize(count, false);
                    self.absence.resize(inputs.exe.rule_count(), false);
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
                    if !self.allow_unit(1, &mut allow) {
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
                    if !self.allow_unit(1, &mut allow) {
                        return Ok(false);
                    }
                    if self.reached[edge.body] {
                        self.extended |= !self.reached[edge.head];
                        self.reached[edge.head] = true;
                        if edge.negative
                            && let Some(rule) = edge.rule
                        {
                            self.absence[rule] = true;
                        }
                    }
                    self.cursor += 1;
                }
                Phase::Absence => {
                    if self.cursor == inputs.exe.rule_count() {
                        self.phase = Phase::ClosurePop;
                        continue;
                    }
                    if !self.allow_unit(1, &mut allow) {
                        return Ok(false);
                    }
                    let rule = self.cursor;
                    if self.absence[rule] {
                        push_task(
                            &mut self.tasks,
                            inputs.levels[rule],
                            Task::Absence { rule },
                            memory,
                            &mut self.retired_bytes,
                        )
                        .map_err(MaintenanceError::Failed)?;
                        self.bucket = Some(Bucket::Producer(rule));
                        self.after_fanout = Phase::Absence;
                        self.phase = Phase::Fanout;
                    }
                    self.cursor += 1;
                }
                Phase::ClosurePop => {
                    if !self.allow_unit(1, &mut allow) {
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
                    if !self.allow_unit(1, &mut allow) {
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
                        reserve_index_in(
                            &mut self.deleted,
                            &mut self.deleted_bytes,
                            1,
                            memory,
                            &mut self.retired_bytes,
                        )
                        .map_err(storage_error)?;
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
                    if !self.allow_unit(cost, &mut allow) {
                        return Ok(false);
                    }
                    reserve_array_in(
                        &mut self.delete,
                        memory,
                        &mut self.retired_bytes,
                        |values, memory| memory.reserve(values, self.deleted.len()),
                    )
                    .map_err(storage_error)?;
                    self.delete.extend(self.deleted.drain());
                    self.deleted = FastMap::default();
                    self.retired_bytes = self
                        .retired_bytes
                        .checked_add(self.deleted_bytes)
                        .expect("retired closure table fits its original grant");
                    self.deleted_bytes = 0;
                    self.sort = Some(HeapSortFrame::new(self.delete.len()));
                    self.phase = Phase::SortDelete;
                    flush_retired_bytes(&mut self.retired_bytes, memory).map_err(storage_error)?;
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
                        if !self.allow_unit(1, &mut allow) {
                            return Ok(false);
                        }
                        dictionary.check_read().map_err(source_error)?;
                        self.phase = Phase::Complete;
                        self.unit_admitted = false;
                        continue;
                    }
                    if self.rule == inputs.exe.rule_count() {
                        self.rule = 0;
                        self.cursor += 1;
                        continue;
                    }
                    if !self.allow_unit(1, &mut allow) {
                        return Ok(false);
                    }
                    let runtime = &inputs.exe.runtimes()[self.rule];
                    if self.head == runtime.head.len() {
                        self.head = 0;
                        self.rule += 1;
                        self.unit_admitted = false;
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
                        memory,
                        &mut self.retired_bytes,
                    )
                    .map_err(MaintenanceError::Failed)?;
                    self.head += 1;
                }
                Phase::Complete => return Ok(true),
            }
            self.unit_admitted = false;
        }
    }

    pub(super) fn finish<'r, D: TermDictionary<Id = I>>(mut self) -> Pending<'r, M, S, D, N> {
        assert_eq!(
            self.phase,
            Phase::Complete,
            "only complete preparation publishes native tasks"
        );
        let roots_bytes = self.roots.heap_capacity() * size_of::<(RowId, [I; N])>();
        let absence_bytes = self.absence.capacity() * size_of::<bool>();
        drop(self.roots);
        drop(self.absence);
        self.retired_bytes = self
            .retired_bytes
            .checked_add(roots_bytes)
            .and_then(|bytes| bytes.checked_add(absence_bytes))
            .expect("retired preparation layouts fit their original grant");
        Pending {
            batch: self.batch,
            changes: self.changes,
            delete: self.delete,
            deletion: 0,
            input: 0,
            input_state: None,
            event: None,
            installed_weights: 0,
            phase: super::MutationPhase::Tasks,
            withdrawals_cleared: false,
            tasks: self.tasks,
            active: None,
            task: None,
            publication: None,
            output_work_admitted: false,
            retired_bytes: self.retired_bytes,
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

fn storage_error<E>(error: StorageError) -> MaintenanceError<E> {
    MaintenanceError::Failed(EvalError::Storage(error))
}
