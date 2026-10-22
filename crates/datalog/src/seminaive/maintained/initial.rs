// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Initial capture and execution through the retained native maintenance kernel.

use super::{
    Admission, Arity, Cell, EvalError, EvalOptions, Limits, Maintained, MaintenanceError,
    ReclaimDictionary, ResumeProgress, Run, StopSignal, StorageError, StoreShape, Task,
    TermDictionary, advance, check_budget, flush_retired_bytes, native_error, push_task,
    reserve_index_in, work,
};

#[derive(Default)]
enum Phase {
    #[default]
    Metadata,
    Weights,
    References,
    OrphanCount,
    Orphans,
    Indexes,
    Tasks,
    Complete,
}

struct Owned<D: TermDictionary> {
    visit: fn(&D, &mut dyn FnMut(D::Id)),
    work: fn(&D) -> Result<usize, StorageError>,
}

pub(super) struct Frame<D: TermDictionary, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    metadata: crate::store::MetadataPreparation<D::Id, N>,
    indexes: crate::store::MaintenanceIndexPreparation,
    phase: Phase,
    cursor: u64,
    rule: usize,
    orphan_count: usize,
    owned: Option<Owned<D>>,
    unit_admitted: bool,
}

impl<D: TermDictionary, const N: usize> Default for Frame<D, N>
where
    Arity<N>: StoreShape<N>,
{
    fn default() -> Self {
        Self {
            metadata: crate::store::MetadataPreparation::default(),
            indexes: crate::store::MaintenanceIndexPreparation::default(),
            phase: Phase::Metadata,
            cursor: 0,
            rule: 0,
            orphan_count: 0,
            owned: None,
            unit_admitted: false,
        }
    }
}

impl<D: TermDictionary, const N: usize> Frame<D, N>
where
    Arity<N>: StoreShape<N>,
{
    pub(super) fn reclaiming(&mut self)
    where
        D: ReclaimDictionary,
    {
        self.owned = Some(Owned {
            visit: |dictionary, visit| dictionary.owned_terms().for_each(visit),
            work: D::owned_term_scan_work,
        });
    }
}

impl<M, S: Admission, D: TermDictionary, const N: usize, A: Admission> Maintained<'_, M, S, D, N, A>
where
    Arity<N>: StoreShape<N>,
{
    pub(super) fn resume_initial(
        &mut self,
        options: EvalOptions,
        limit: u128,
        stop: Option<&dyn StopSignal>,
    ) -> Result<ResumeProgress<D::Error>, MaintenanceError<D::Error>> {
        if self.initial_work > limit {
            return Err(MaintenanceError::WorkLimit {
                limit,
                consumed: self.initial_work,
                needed: self.initial_work,
            });
        }
        let state = self
            .session
            .state
            .as_mut()
            .expect("initial model remains owned");
        state.limits = Limits::of(&options, state.input_terms);
        state.stated = super::super::BudgetReport::stated_limits(&options);
        state.execution = options.round_execution();
        let used = Cell::new(self.initial_work);
        let meter = work::Meter::new(&used, 0, limit, stop);
        let before = self.storage.live();
        let frame = &mut self.initial;
        let pending = self
            .pending
            .as_mut()
            .expect("initial native execution remains owned");
        let result = self.storage.with_memory(|memory| {
            meter.with_memory(memory, |memory| {
                check_budget(state)?;
                flush_retired_bytes(&mut self.retired_bytes, memory).map_err(EvalError::Storage)?;
                loop {
                    match frame.phase {
                        Phase::Metadata => {
                            if !frame
                                .metadata
                                .resume(&mut state.rel, memory, |cost| meter.allow(cost))
                                .map_err(native_error)?
                            {
                                return meter.paused(state.report());
                            }
                            frame.phase = Phase::Weights;
                        }
                        Phase::Weights => {
                            let additional = state.rel.row_count() - self.weights.len();
                            reserve_index_in(
                                &mut self.weights,
                                &mut self.weights_bytes,
                                additional,
                                memory,
                                &mut self.retired_bytes,
                            )
                            .map_err(EvalError::Storage)?;
                            while let Some((row, terms)) = state.rel.rows_from(frame.cursor).next()
                            {
                                if !meter.allow(1) {
                                    return meter.paused(state.report());
                                }
                                self.weights.insert(terms, 1);
                                frame.cursor = row.index() + 1;
                            }
                            frame.cursor = 0;
                            frame.phase = Phase::References;
                        }
                        Phase::References => {
                            if let Some(uses) =
                                &mut state.native.as_mut().expect("native owner").term_uses
                            {
                                while let Some((row, terms)) =
                                    state.rel.rows_from(frame.cursor).next()
                                {
                                    uses.reserve_in(N, memory, &mut self.retired_bytes)
                                        .map_err(EvalError::Storage)?;
                                    if !meter.allow(N) {
                                        return meter.paused(state.report());
                                    }
                                    uses.change(terms, 1);
                                    frame.cursor = row.index() + 1;
                                }
                            }
                            frame.phase = Phase::OrphanCount;
                        }
                        Phase::OrphanCount => {
                            if let Some(owned) = &frame.owned {
                                let cost = (owned.work)(state.rel.interner())
                                    .map_err(EvalError::Storage)?;
                                if !meter.allow(cost) {
                                    return meter.paused(state.report());
                                }
                                let uses = state
                                    .native
                                    .as_ref()
                                    .expect("native owner")
                                    .term_uses
                                    .as_ref()
                                    .expect("reclaiming references");
                                (owned.visit)(state.rel.interner(), &mut |id| {
                                    if !uses.counts.contains_key(&id) {
                                        frame.orphan_count += 1;
                                    }
                                });
                            }
                            frame.phase = Phase::Orphans;
                        }
                        Phase::Orphans => {
                            if frame.orphan_count != 0 {
                                let owned =
                                    frame.owned.as_ref().expect("original owned identities");
                                let uses = state
                                    .native
                                    .as_mut()
                                    .expect("native owner")
                                    .term_uses
                                    .as_mut()
                                    .expect("reclaiming references");
                                uses.zero
                                    .reserve_in(frame.orphan_count, memory, &mut self.retired_bytes)
                                    .map_err(EvalError::Storage)?;
                                let cost = (owned.work)(state.rel.interner())
                                    .map_err(EvalError::Storage)?;
                                if !meter.allow(cost) {
                                    return meter.paused(state.report());
                                }
                                (owned.visit)(state.rel.interner(), &mut |id| {
                                    if !uses.counts.contains_key(&id) {
                                        uses.zero.insert(id);
                                    }
                                });
                            }
                            frame.phase = Phase::Indexes;
                        }
                        Phase::Indexes => {
                            if !frame
                                .indexes
                                .resume(&mut state.rel, memory, |cost| meter.allow(cost))
                                .map_err(native_error)?
                            {
                                return meter.paused(state.report());
                            }
                            frame.phase = Phase::Tasks;
                        }
                        Phase::Tasks => {
                            while frame.rule < self.session.exe.rule_count() {
                                if !frame.unit_admitted {
                                    if !meter.allow(1) {
                                        return meter.paused(state.report());
                                    }
                                    frame.unit_admitted = true;
                                }
                                push_task(
                                    &mut pending.tasks,
                                    self.levels[frame.rule],
                                    Task::Absence { rule: frame.rule },
                                    memory,
                                    &mut pending.retired_bytes,
                                )?;
                                frame.rule += 1;
                                frame.unit_admitted = false;
                            }
                            frame.phase = Phase::Complete;
                        }
                        Phase::Complete => break,
                    }
                }
                advance(
                    &Run {
                        exe: self.session.exe,
                        guards: self.session.guards,
                        levels: self.levels,
                    },
                    &mut self.weights,
                    pending,
                    state,
                    &meter,
                    memory,
                )
            })
        });
        self.initial_work = used.get();
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
                self.delta = self
                    .pending
                    .take()
                    .expect("completed initial execution")
                    .delta;
                self.session.maintained_complete();
                self.prepared = true;
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
