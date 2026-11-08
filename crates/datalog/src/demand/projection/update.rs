// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Staged private assertions for one already validated original transaction.

use super::{
    Admission, Arity, DictionaryError, DictionaryTerm, Error, HeapSortFrame, Memory, OrderedTable,
    Progress, RelationStore, Runtime, StorageError, StoreShape, TermDictionary, TermIdentity,
    flush_retired_bytes, reserve_array_in, reserve_ordered_in,
};

#[derive(Default)]
enum Phase {
    #[default]
    Collect,
    Sort,
    Members,
    Mirrors,
    Complete,
}

pub(crate) struct Update<I: TermIdentity, const N: usize> {
    phase: Phase,
    original_len: usize,
    cursor: usize,
    coordinate: usize,
    deltas: OrderedTable<I, i64>,
    order: Vec<I>,
    sort: Option<HeapSortFrame>,
    retired_bytes: usize,
    unit_admitted: bool,
    member_minted: bool,
}

impl<I: TermIdentity, const N: usize> Update<I, N> {
    pub(crate) fn new(original_len: usize) -> Self {
        Self {
            phase: Phase::Collect,
            original_len,
            cursor: 0,
            coordinate: 0,
            deltas: OrderedTable::default(),
            order: Vec::new(),
            sort: None,
            retired_bytes: 0,
            unit_admitted: false,
            member_minted: false,
        }
    }

    pub(crate) fn retire(self) -> Result<usize, StorageError> {
        self.retired_bytes
            .checked_add(self.deltas.storage_bytes()?)
            .and_then(|bytes| bytes.checked_add(self.order.capacity() * size_of::<I>()))
            .ok_or(StorageError::SizeOverflow)
    }

    pub(crate) fn resume<D: TermDictionary<Id = I>, A: Admission + ?Sized>(
        &mut self,
        runtime: &mut Runtime<I, N>,
        rel: &mut RelationStore<D, N>,
        changes: &mut Vec<([I; N], i64)>,
        old_weight: impl Fn(&[I; N]) -> i64,
        memory: &mut Memory<'_, A>,
        mut allow: impl FnMut(usize) -> bool,
    ) -> Result<Progress, Error<D::Error>>
    where
        Arity<N>: StoreShape<N>,
    {
        flush_retired_bytes(&mut self.retired_bytes, memory)?;
        loop {
            match self.phase {
                Phase::Collect => {
                    if self.cursor == self.original_len {
                        self.sort = Some(HeapSortFrame::new(self.order.len()));
                        self.cursor = 0;
                        self.phase = Phase::Sort;
                        continue;
                    }
                    if !crate::allocation::allow_unit(&mut self.unit_admitted, 1, &mut allow) {
                        return Ok(Progress::Paused);
                    }
                    let (terms, weight) = changes[self.cursor];
                    let old = old_weight(&terms);
                    if (old == 0) != (weight == 0) {
                        let id = terms[self.coordinate];
                        if runtime.uses_map()
                            || (self.coordinate == 1
                                && runtime.fixed_original(id, rel.interner())?)
                        {
                            if self.deltas.get(&id).is_none() {
                                reserve_ordered_in(
                                    &mut self.deltas,
                                    memory,
                                    &mut self.retired_bytes,
                                )?;
                                reserve_array_in(
                                    &mut self.order,
                                    memory,
                                    &mut self.retired_bytes,
                                    |order, memory| memory.reserve_for_push(order),
                                )?;
                                self.deltas.insert(id, 0, memory)?;
                                self.order.push(id);
                            }
                            let delta = self.deltas.get_mut(&id).expect("admitted domain delta");
                            *delta = delta
                                .checked_add(if weight == 0 { -1 } else { 1 })
                                .ok_or(StorageError::SizeOverflow)?;
                        }
                    }
                    self.coordinate += 1;
                    if self.coordinate == N {
                        self.coordinate = 0;
                        self.cursor += 1;
                    }
                    self.unit_admitted = false;
                }
                Phase::Sort => {
                    if !self
                        .sort
                        .as_mut()
                        .expect("domain delta sort")
                        .resume(
                            &mut self.order,
                            |a, b| {
                                rel.interner()
                                    .compare_terms(DictionaryTerm::Id(*a), DictionaryTerm::Id(*b))
                                    .map(|order| order.then_with(|| a.cmp(b)))
                            },
                            || allow(1),
                        )
                        .map_err(DictionaryError::Source)?
                    {
                        return Ok(Progress::Paused);
                    }
                    self.sort = None;
                    self.phase = Phase::Members;
                }
                Phase::Members => {
                    let Some(&id) = self.order.get(self.cursor) else {
                        self.cursor = 0;
                        self.phase = Phase::Mirrors;
                        continue;
                    };
                    if !crate::allocation::allow_unit(&mut self.unit_admitted, 1, &mut allow) {
                        return Ok(Progress::Paused);
                    }
                    let old = runtime.references(id);
                    let count = i128::try_from(old).map_err(|_| StorageError::SizeOverflow)?
                        + i128::from(*self.deltas.get(&id).expect("staged domain delta"));
                    let count = usize::try_from(count).map_err(|_| StorageError::SizeOverflow)?;
                    let old_active = runtime.active_with(id, old);
                    let active = runtime.active_with(id, count);
                    if active || runtime.shadow_target(id).is_some() {
                        if !self.member_minted {
                            if !runtime.mint_in(id, rel, memory, &mut allow)? {
                                return Ok(Progress::Paused);
                            }
                            self.member_minted = true;
                            return Ok(Progress::Advanced);
                        }
                        if active != old_active && runtime.uses_map() {
                            reserve_array_in(
                                changes,
                                memory,
                                &mut self.retired_bytes,
                                |changes, memory| memory.reserve_for_push(changes),
                            )?;
                            let row = runtime.map_row(id, rel)?.expect("mapped domain member");
                            changes.push((row, i64::from(active)));
                        }
                        runtime.set_references(id, count);
                    }
                    self.cursor += 1;
                    self.unit_admitted = false;
                    self.member_minted = false;
                    // Each term factory returns to the native governor owner.
                    return Ok(Progress::Advanced);
                }
                Phase::Mirrors => {
                    if !runtime.mirrors() || self.cursor == self.original_len {
                        self.phase = Phase::Complete;
                        continue;
                    }
                    if !crate::allocation::allow_unit(&mut self.unit_admitted, 1, &mut allow) {
                        return Ok(Progress::Paused);
                    }
                    let (mut terms, weight) = changes[self.cursor];
                    if let Some(target) = runtime.shadow_target(terms[1]) {
                        reserve_array_in(
                            changes,
                            memory,
                            &mut self.retired_bytes,
                            |changes, memory| memory.reserve_for_push(changes),
                        )?;
                        terms[1] = target;
                        changes.push((terms, weight));
                    }
                    self.cursor += 1;
                    self.unit_admitted = false;
                }
                Phase::Complete => return Ok(Progress::Complete),
            }
        }
    }
}
