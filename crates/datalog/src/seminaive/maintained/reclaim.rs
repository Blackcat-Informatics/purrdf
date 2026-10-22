// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Retained result pins and native term-retirement progress.

use super::{
    Admission, Arity, EvalError, FixpointState, Memory, RowSet, StorageError, StoreShape,
    TermDictionary, TermIdentity, flush_retired_bytes, native_error, reserve_array_in,
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
    pinned: RowSet<I>,
    candidates: Vec<I>,
    retired_bytes: usize,
    phase: Phase,
    cursor: usize,
    sort: Option<HeapSortFrame>,
}

impl<I> Default for Frame<I> {
    fn default() -> Self {
        Self {
            pinned: RowSet::default(),
            candidates: Vec::new(),
            retired_bytes: 0,
            phase: Phase::Pins,
            cursor: 0,
            sort: None,
        }
    }
}

impl<I: TermIdentity> Frame<I> {
    pub(super) fn resume<D: TermDictionary<Id = I>, const N: usize, A: Admission + ?Sized>(
        &mut self,
        state: &mut FixpointState<D, N>,
        delta: &[([I; N], i64)],
        memory: &mut Memory<'_, A>,
        mut allow: impl FnMut(usize) -> bool,
    ) -> Result<bool, EvalError<D::Error>>
    where
        Arity<N>: StoreShape<N>,
    {
        flush_retired_bytes(&mut self.retired_bytes, memory).map_err(EvalError::Storage)?;
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
                        .reserve_in(N, memory, &mut self.retired_bytes)
                        .map_err(EvalError::Storage)?;
                    for term in terms {
                        self.pinned.insert(term);
                    }
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
                    reserve_array_in(
                        &mut self.candidates,
                        memory,
                        &mut self.retired_bytes,
                        |values, memory| memory.reserve(values, uses.zero.len()),
                    )
                    .map_err(EvalError::Storage)?;
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
                Phase::Complete => {
                    let pinned = std::mem::take(&mut self.pinned);
                    let candidates = std::mem::take(&mut self.candidates);
                    let bytes = candidates.capacity() * size_of::<I>();
                    self.retired_bytes = self
                        .retired_bytes
                        .checked_add(pinned.retained)
                        .and_then(|held| held.checked_add(bytes))
                        .expect("retired term pins fit their original grant");
                    drop(pinned);
                    drop(candidates);
                    flush_retired_bytes(&mut self.retired_bytes, memory)
                        .map_err(EvalError::Storage)?;
                    return Ok(true);
                }
            }
        }
    }
}
