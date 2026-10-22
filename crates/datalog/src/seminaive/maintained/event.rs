// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Retained authored-atom event fan-out and all-before-publication reservation.

use super::{
    Arity, BTreeMap, EvalError, Executable, RelationStore, StorageError, StoreShape, Task,
    TermDictionary, TermIdentity, WorkList,
};

enum Phase {
    Scan,
    Prior,
    Count,
    Reserve,
    Check,
    Publish,
    Complete,
}

pub(super) struct Frame<I, const N: usize> {
    terms: [I; N],
    prepared: Vec<(usize, Task<I, N>)>,
    phase: Phase,
    rule: usize,
    atom: usize,
    position: usize,
    matches: bool,
    cursor: usize,
    scan: usize,
    count: usize,
}

impl<I: TermIdentity, const N: usize> Frame<I, N> {
    pub(super) fn new(terms: [I; N]) -> Self {
        Self {
            terms,
            prepared: Vec::new(),
            phase: Phase::Scan,
            rule: 0,
            atom: 0,
            position: 0,
            matches: true,
            cursor: 0,
            scan: 0,
            count: 0,
        }
    }

    pub(super) fn resume<D: TermDictionary<Id = I>>(
        &mut self,
        exe: &Executable<N>,
        levels: &[usize],
        tasks: &mut BTreeMap<usize, WorkList<Task<I, N>, 8>>,
        rel: &RelationStore<D, N>,
        mut allow: impl FnMut(usize) -> bool,
    ) -> Result<bool, EvalError<D::Error>>
    where
        Arity<N>: StoreShape<N>,
    {
        rel.interner().check_read().map_err(EvalError::Source)?;
        loop {
            match self.phase {
                Phase::Scan => {
                    if self.rule == exe.rule_count() {
                        self.phase = Phase::Prior;
                        continue;
                    }
                    let (_, plan) = exe.rule_entry(self.rule);
                    let Some(atom) = plan.operators().get(self.atom) else {
                        if !allow(1) {
                            return Ok(false);
                        }
                        self.rule += 1;
                        self.atom = 0;
                        continue;
                    };
                    let positions = atom.shape().positions();
                    if self.position == N || !self.matches {
                        if !allow(1) {
                            return Ok(false);
                        }
                        if self.matches {
                            self.prepared
                                .try_reserve(1)
                                .map_err(|_| EvalError::Storage(StorageError::AllocationFailed))?;
                            self.prepared.push((
                                levels[self.rule],
                                Task::Body {
                                    rule: self.rule,
                                    body: atom.body_index(),
                                    terms: self.terms,
                                },
                            ));
                        }
                        self.atom += 1;
                        self.position = 0;
                        self.matches = true;
                        continue;
                    }
                    if !allow(1) {
                        return Ok(false);
                    }
                    if let Some(surface) = positions[self.position].constant() {
                        self.matches = rel
                            .interner()
                            .lookup_term(surface)
                            .map_err(EvalError::Source)?
                            == Some(self.terms[self.position]);
                    }
                    self.position += 1;
                }
                Phase::Prior => {
                    if self.cursor == self.prepared.len() {
                        self.phase = Phase::Check;
                        continue;
                    }
                    if self.scan == self.cursor {
                        self.scan = 0;
                        self.count = 0;
                        self.phase = Phase::Count;
                        continue;
                    }
                    if !allow(1) {
                        return Ok(false);
                    }
                    if self.prepared[self.scan].0 == self.prepared[self.cursor].0 {
                        self.cursor += 1;
                        self.scan = 0;
                    } else {
                        self.scan += 1;
                    }
                }
                Phase::Count => {
                    if self.scan == self.prepared.len() {
                        self.phase = Phase::Reserve;
                        continue;
                    }
                    if !allow(1) {
                        return Ok(false);
                    }
                    if self.prepared[self.scan].0 == self.prepared[self.cursor].0 {
                        self.count += 1;
                    }
                    self.scan += 1;
                }
                Phase::Reserve => {
                    if !allow(1) {
                        return Ok(false);
                    }
                    let work = tasks.entry(self.prepared[self.cursor].0).or_default();
                    let required = work
                        .len()
                        .checked_add(self.count)
                        .ok_or(EvalError::Storage(StorageError::SizeOverflow))?;
                    work.try_reserve_heap_exact(required.saturating_sub(8))
                        .map_err(|_| EvalError::Storage(StorageError::AllocationFailed))?;
                    self.cursor += 1;
                    self.scan = 0;
                    self.phase = Phase::Prior;
                }
                Phase::Check => {
                    if !allow(1) {
                        return Ok(false);
                    }
                    rel.interner().check_read().map_err(EvalError::Source)?;
                    self.cursor = 0;
                    self.phase = Phase::Publish;
                }
                Phase::Publish => {
                    let Some(&(level, task)) = self.prepared.get(self.cursor) else {
                        self.phase = Phase::Complete;
                        continue;
                    };
                    if !allow(1) {
                        return Ok(false);
                    }
                    tasks
                        .get_mut(&level)
                        .expect("the entire original event was admitted")
                        .push(task);
                    self.cursor += 1;
                }
                Phase::Complete => return Ok(true),
            }
        }
    }
}
