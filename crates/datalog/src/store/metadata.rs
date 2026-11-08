// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Retained conversion of original metadata, without adopting resident buffers.

use super::rows::RowLog;
use super::{
    Arity, DictionaryError, PartitionId, Relation, RelationStore, StoreKey, StoreShape,
    TermDictionary, TermIdentity,
};
use purrdf_lex::allocation::{Admission, Memory, OrderedTable, StorageError};
use std::ops::Bound::{Excluded, Unbounded};

#[derive(Debug, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Rows,
    Lookup,
    Keys,
    Free,
    Relations,
    Publish,
    Complete,
}

#[derive(Debug)]
pub(crate) struct Preparation<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    rows: RowLog<I>,
    lookup: OrderedTable<StoreKey<I, N>, usize>,
    keys: Vec<Option<(PartitionId, StoreKey<I, N>)>>,
    free: Vec<usize>,
    relations: Vec<Relation<I>>,
    phase: Phase,
    cursor: u64,
    last_key: Option<StoreKey<I, N>>,
    tail_cursor: usize,
    retired: usize,
}

impl<I: TermIdentity, const N: usize> Default for Preparation<I, N>
where
    Arity<N>: StoreShape<N>,
{
    fn default() -> Self {
        Self {
            rows: RowLog::empty_sparse(0),
            lookup: OrderedTable::default(),
            keys: Vec::new(),
            free: Vec::new(),
            relations: Vec::new(),
            phase: Phase::Rows,
            cursor: 0,
            last_key: None,
            tail_cursor: 0,
            retired: 0,
        }
    }
}

impl<I: TermIdentity, const N: usize> Preparation<I, N>
where
    Arity<N>: StoreShape<N>,
{
    pub(crate) fn resume<D: TermDictionary<Id = I>, A: Admission + ?Sized>(
        &mut self,
        store: &mut RelationStore<D, N>,
        memory: &mut Memory<'_, A>,
        mut allow: impl FnMut(usize) -> bool,
    ) -> Result<bool, DictionaryError<D::Error>> {
        if self.phase == Phase::Complete {
            return Ok(true);
        }
        assert!(!store.metadata_supplemental);
        store
            .interner
            .check_read()
            .map_err(DictionaryError::Source)?;
        crate::allocation::flush_retired_bytes(&mut self.retired, memory)?;
        loop {
            match self.phase {
                Phase::Rows => {
                    if !store.row_log.copy_sparse_until_in(
                        &mut self.rows,
                        &mut self.cursor,
                        memory,
                        &mut allow,
                    )? {
                        return Ok(false);
                    }
                    self.phase = Phase::Lookup;
                }
                Phase::Lookup => {
                    let start = self.last_key.map_or(Unbounded, Excluded);
                    let Some((&key, &slot)) = store.by_key.range((start, Unbounded)).next() else {
                        self.cursor = 0;
                        self.phase = Phase::Keys;
                        continue;
                    };
                    crate::allocation::reserve_ordered_in(
                        &mut self.lookup,
                        memory,
                        &mut self.retired,
                    )?;
                    if !allow(1) {
                        return Ok(false);
                    }
                    assert!(self.lookup.insert(key, slot, memory)?.is_none());
                    self.last_key = Some(key);
                }
                Phase::Keys => {
                    crate::allocation::reserve_array_in(
                        &mut self.keys,
                        memory,
                        &mut self.retired,
                        |values, memory| memory.reserve(values, store.keys.len()),
                    )?;
                    while let Some(&key) = store.keys.get(self.keys.len()) {
                        if !allow(1) {
                            return Ok(false);
                        }
                        self.keys.push(key);
                    }
                    self.phase = Phase::Free;
                }
                Phase::Free => {
                    crate::allocation::reserve_array_in(
                        &mut self.free,
                        memory,
                        &mut self.retired,
                        |values, memory| memory.reserve(values, store.free_partitions.len()),
                    )?;
                    while let Some(&slot) = store.free_partitions.get(self.free.len()) {
                        if !allow(1) {
                            return Ok(false);
                        }
                        self.free.push(slot);
                    }
                    self.phase = Phase::Relations;
                }
                Phase::Relations => {
                    crate::allocation::reserve_array_in(
                        &mut self.relations,
                        memory,
                        &mut self.retired,
                        |values, memory| memory.reserve(values, store.relations.len()),
                    )?;
                    while let Some(original) = store
                        .relations
                        .get(usize::try_from(self.cursor).expect("resident metadata cursor"))
                    {
                        let index = usize::try_from(self.cursor).expect("resident metadata cursor");
                        if self.relations.len() == index {
                            if !allow(1) {
                                return Ok(false);
                            }
                            self.relations.push(Relation::supplemental());
                        }
                        let replacement = &mut self.relations[index];
                        replacement.tail.data.reserve_capacity_in(
                            super::TAIL_SEAL_THRESHOLD,
                            memory,
                            &mut self.retired,
                        )?;
                        crate::allocation::reserve_array_in(
                            &mut replacement.batches,
                            memory,
                            &mut self.retired,
                            |values, memory| memory.reserve(values, original.batches.len()),
                        )?;
                        while let Some((cells, row)) = original.tail.data.row(self.tail_cursor) {
                            if !allow(1) {
                                return Ok(false);
                            }
                            replacement.tail.data.push_reserved(cells, row);
                            self.tail_cursor += 1;
                        }
                        replacement.len = original.len;
                        replacement.maintain_objects = original.maintain_objects;
                        self.cursor += 1;
                        self.tail_cursor = 0;
                    }
                    self.phase = Phase::Publish;
                }
                Phase::Publish => {
                    crate::allocation::flush_retired_bytes(&mut self.retired, memory)?;
                    if !allow(
                        store
                            .relations
                            .len()
                            .checked_add(1)
                            .ok_or(StorageError::SizeOverflow)?,
                    ) {
                        return Ok(false);
                    }
                    store
                        .interner
                        .check_read()
                        .map_err(DictionaryError::Source)?;
                    for (replacement, original) in
                        self.relations.iter_mut().zip(&mut store.relations)
                    {
                        replacement.batches.append(&mut original.batches);
                    }
                    store.relations = core::mem::take(&mut self.relations);
                    store.keys = core::mem::take(&mut self.keys);
                    store.free_partitions = core::mem::take(&mut self.free);
                    store.row_log = core::mem::take(&mut self.rows);
                    store.by_key = core::mem::take(&mut self.lookup);
                    store.metadata_supplemental = true;
                    store.spare_relation = Relation::supplemental();
                    store.maintenance_indexes = false;
                    self.phase = Phase::Complete;
                }
                Phase::Complete => return Ok(true),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::TermInterner;
    use purrdf_lex::allocation::Resident;

    #[test]
    fn metadata_work_cuts_keep_original_coordinates_and_factory_progress() {
        let input = || {
            let mut input =
                RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
            for index in 0..200 {
                input
                    .insert_row([&format!("s{index}"), &format!("p{}", index % 7), "o"])
                    .unwrap();
            }
            let rows = input.rows_from(0).collect::<Vec<_>>();
            for (index, &(_, terms)) in rows.iter().enumerate() {
                if index % 9 == 0 {
                    input.remove_ids(terms).unwrap();
                }
            }
            input
        };
        let mut reference = input();
        let mut preparation = Preparation::default();
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut total = 0;
        assert!(
            preparation
                .resume(&mut reference, &mut memory, |cost| {
                    total += cost;
                    true
                })
                .unwrap()
        );
        let expected_bytes = memory.admitted_bytes();
        let expected = reference.rows_from(0).collect::<Vec<_>>();
        for limit in 0..=total {
            let mut original = input();
            let next = original.row_log.next();
            let keys = original.keys.clone();
            let mut preparation = Preparation::default();
            let mut resident = Resident;
            let mut memory = Memory::new(&mut resident);
            let mut used = 0;
            let complete = preparation
                .resume(&mut original, &mut memory, |cost| {
                    if used + cost > limit {
                        false
                    } else {
                        used += cost;
                        true
                    }
                })
                .unwrap();
            assert_eq!(original.rows_from(0).collect::<Vec<_>>(), expected);
            assert_eq!(original.row_log.next(), next);
            assert_eq!(original.keys, keys);
            if !complete {
                assert!(!original.metadata_supplemental);
                assert!(
                    preparation
                        .resume(&mut original, &mut memory, |cost| {
                            used += cost;
                            true
                        })
                        .unwrap()
                );
            }
            assert_eq!(used, total, "metadata cut {limit} repeats no admitted work");
            assert_eq!(memory.admitted_bytes(), expected_bytes);
            assert_eq!(original.rows_from(0).collect::<Vec<_>>(), expected);
            assert!(original.metadata_supplemental);
        }
    }
}
