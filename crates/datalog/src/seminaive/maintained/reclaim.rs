// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Retained result pins and native term-retirement progress.

use super::{
    Arity, EvalError, FastSet, FixpointState, StorageError, StoreShape, TermDictionary,
    TermIdentity, native_error,
};
use purrdf_lex::walk::HeapSortFrame;

#[derive(Default)]
enum Phase {
    #[default]
    Pins,
    Gather,
    Sort,
    Release,
    Complete,
}

pub(super) struct Frame<I> {
    pinned: FastSet<I>,
    candidates: Vec<I>,
    phase: Phase,
    cursor: usize,
    sort: Option<HeapSortFrame>,
}

impl<I> Default for Frame<I> {
    fn default() -> Self {
        Self {
            pinned: FastSet::default(),
            candidates: Vec::new(),
            phase: Phase::Pins,
            cursor: 0,
            sort: None,
        }
    }
}

impl<I: TermIdentity> Frame<I> {
    pub(super) fn resume<D: TermDictionary<Id = I>, const N: usize>(
        &mut self,
        state: &mut FixpointState<D, N>,
        delta: &[([I; N], i64)],
        mut allow: impl FnMut(usize) -> bool,
    ) -> Result<bool, EvalError<D::Error>>
    where
        Arity<N>: StoreShape<N>,
    {
        let Some(release) = state.release_term else {
            return Ok(true);
        };
        let uses = state
            .native
            .as_mut()
            .expect("native proof owner")
            .term_uses
            .as_mut()
            .expect("reclaiming initialization retains term uses");
        loop {
            match self.phase {
                Phase::Pins => {
                    let Some(&(terms, _)) = delta.get(self.cursor) else {
                        self.cursor = 0;
                        self.phase = Phase::Gather;
                        continue;
                    };
                    if !allow(N + 1) {
                        return Ok(false);
                    }
                    self.pinned
                        .try_reserve(N)
                        .map_err(|_| EvalError::Storage(StorageError::AllocationFailed))?;
                    self.pinned.extend(terms);
                    self.cursor += 1;
                }
                Phase::Gather => {
                    let cost = uses
                        .zero
                        .scan_work()
                        .checked_add(1)
                        .ok_or(EvalError::Storage(StorageError::SizeOverflow))?;
                    if !allow(cost) {
                        return Ok(false);
                    }
                    self.candidates
                        .try_reserve_exact(uses.zero.len())
                        .map_err(|_| EvalError::Storage(StorageError::AllocationFailed))?;
                    self.candidates.extend(
                        (&uses.zero)
                            .into_iter()
                            .copied()
                            .filter(|id| !self.pinned.contains(id)),
                    );
                    self.sort = Some(HeapSortFrame::new(self.candidates.len()));
                    self.phase = Phase::Sort;
                }
                Phase::Sort => {
                    if !self
                        .sort
                        .as_mut()
                        .expect("retained retirement sorter")
                        .resume(
                            &mut self.candidates,
                            |a, b| Ok::<_, D::Error>(a.cmp(b)),
                            || allow(1),
                        )
                        .map_err(EvalError::Source)?
                    {
                        return Ok(false);
                    }
                    self.phase = Phase::Release;
                }
                Phase::Release => {
                    let Some(&id) = self.candidates.get(self.cursor) else {
                        self.phase = Phase::Complete;
                        continue;
                    };
                    if !allow(1) {
                        return Ok(false);
                    }
                    release(state.rel.interner_mut(), id).map_err(native_error)?;
                    assert!(uses.zero.remove(&id));
                    self.cursor += 1;
                }
                Phase::Complete => return Ok(true),
            }
        }
    }
}
