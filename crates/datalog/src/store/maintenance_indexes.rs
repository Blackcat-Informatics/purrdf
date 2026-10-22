// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Pausable original partition directory and object-index preparation.

use super::{
    Admission, Arity, DictionaryError, Infallible, Memory, ObjectIndex, RelationStore, StoreShape,
    TermDictionary, directory, index,
};
use purrdf_lex::walk::HeapSortFrame;

#[derive(Default)]
enum Phase {
    #[default]
    Directory,
    Empty,
    Objects,
    Complete,
}

#[derive(Default)]
pub(crate) struct Preparation {
    phase: Phase,
    slot: usize,
    batch: usize,
    permutation: Vec<usize>,
    sort: Option<HeapSortFrame>,
    relation_admitted: bool,
    unit_admitted: bool,
    retired: usize,
}

impl Preparation {
    pub(crate) fn resume<D: TermDictionary, const N: usize, A: Admission + ?Sized>(
        &mut self,
        store: &mut RelationStore<D, N>,
        memory: &mut Memory<'_, A>,
        mut allow: impl FnMut(usize) -> bool,
    ) -> Result<bool, DictionaryError<D::Error>>
    where
        Arity<N>: StoreShape<N>,
    {
        store
            .interner
            .check_read()
            .map_err(DictionaryError::Source)?;
        store.flush_metadata_retirement_in(memory)?;
        crate::allocation::flush_retired_bytes(&mut self.retired, memory)?;
        loop {
            match self.phase {
                Phase::Directory => {
                    assert!(store.metadata_supplemental && store.row_log.is_supplemental());
                    store.reserve_partition_slots_in(store.relations.len(), memory)?;
                    let frame = store
                        .directory_preparation
                        .get_or_insert_with(directory::Preparation::default);
                    while frame.next < store.keys.len() {
                        if !self.unit_admitted {
                            if !allow(1) {
                                return Ok(false);
                            }
                            self.unit_admitted = true;
                        }
                        let slot = frame.next;
                        if store.relations[slot].row_count() != 0 {
                            let (id, key) = store.keys[slot].expect("live original key");
                            frame.directory.insert_live_in::<D, N, A>(
                                &store.interner,
                                (slot, id, key),
                                memory,
                            )?;
                        }
                        frame.next += 1;
                        self.unit_admitted = false;
                    }
                    if !allow(1) {
                        return Ok(false);
                    }
                    store
                        .interner
                        .check_read()
                        .map_err(DictionaryError::Source)?;
                    store.order = store
                        .directory_preparation
                        .take()
                        .expect("completed original directory")
                        .directory;
                    self.phase = Phase::Empty;
                }
                Phase::Empty => {
                    while self.slot < store.keys.len() {
                        if !allow(1) {
                            return Ok(false);
                        }
                        let slot = self.slot;
                        if store.relations[slot].row_count() == 0 && store.keys[slot].is_some() {
                            let (_, key) = store.keys[slot].take().expect("occupied empty key");
                            store.by_key.remove(&key);
                            store.retire_relation_slot(slot);
                            store.free_partitions.push(slot);
                        }
                        self.slot += 1;
                    }
                    store.flush_metadata_retirement_in(memory)?;
                    self.slot = 0;
                    self.phase = Phase::Objects;
                }
                Phase::Objects => {
                    while let Some(relation) = store.relations.get_mut(self.slot) {
                        if !self.relation_admitted {
                            if !allow(1) {
                                return Ok(false);
                            }
                            self.relation_admitted = true;
                        }
                        relation.maintain_objects = true;
                        while let Some(batch) = relation.batches.get_mut(self.batch) {
                            if !self.unit_admitted {
                                if !allow(1) {
                                    return Ok(false);
                                }
                                self.unit_admitted = true;
                            }
                            if batch.object_index.get().is_some_and(|index| index.admitted) {
                                self.batch += 1;
                                self.unit_admitted = false;
                                continue;
                            }
                            crate::allocation::reserve_array_in(
                                &mut self.permutation,
                                memory,
                                &mut self.retired,
                                |values, memory| memory.reserve(values, batch.len()),
                            )?;
                            while self.permutation.len() < batch.len() {
                                if !allow(1) {
                                    return Ok(false);
                                }
                                let index = self.permutation.len();
                                self.permutation.push(
                                    batch
                                        .object_index
                                        .get()
                                        .map_or(index, |original| original.perm[index]),
                                );
                            }
                            if batch.object_index.get().is_none() {
                                let sort = self.sort.get_or_insert_with(|| {
                                    HeapSortFrame::new(self.permutation.len())
                                });
                                let complete = sort
                                    .resume(
                                        &mut self.permutation,
                                        |&left, &right| {
                                            Ok::<_, Infallible>(index::compare_row_positions(
                                                left,
                                                right,
                                                &[batch.data.column(0), batch.data.column(1)],
                                                &[1, 0],
                                            ))
                                        },
                                        || allow(1),
                                    )
                                    .unwrap_or_else(|never| match never {});
                                if !complete {
                                    return Ok(false);
                                }
                            }
                            if !allow(1) {
                                return Ok(false);
                            }
                            store
                                .interner
                                .check_read()
                                .map_err(DictionaryError::Source)?;
                            let previous = batch.object_index.take();
                            batch
                                .object_index
                                .set(ObjectIndex {
                                    perm: core::mem::take(&mut self.permutation),
                                    admitted: true,
                                })
                                .expect("original object permutation publication");
                            drop(previous);
                            self.sort = None;
                            self.batch += 1;
                            self.unit_admitted = false;
                        }
                        self.batch = 0;
                        self.slot += 1;
                        self.relation_admitted = false;
                    }
                    store.maintenance_indexes = true;
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
    fn object_preparation_charges_retired_partition_slots_before_advancing() {
        let mut store = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
        for index in 0..257 {
            store.insert_row(["s", &index.to_string(), "o"]).unwrap();
        }
        for index in 0..257 {
            store.remove_row(["s", &index.to_string(), "o"]).unwrap();
        }
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        store.admit_native_metadata_in(&mut memory).unwrap();
        let mut frame = Preparation::default();
        while !matches!(frame.phase, Phase::Objects) {
            let mut credit = 1;
            assert!(
                !frame
                    .resume(&mut store, &mut memory, |cost| {
                        if cost > credit {
                            return false;
                        }
                        credit -= cost;
                        true
                    })
                    .unwrap()
            );
        }
        let slots = store.relations.len();
        assert_eq!(slots, 257);
        assert!(
            store
                .relations
                .iter()
                .all(|relation| relation.batches.is_empty())
        );
        let bytes = memory.admitted_bytes();
        for slot in 0..slots {
            assert!(!frame.resume(&mut store, &mut memory, |_| false).unwrap());
            assert_eq!(frame.slot, slot);
            let mut credit = 1;
            let complete = frame
                .resume(&mut store, &mut memory, |cost| {
                    if cost > credit {
                        return false;
                    }
                    credit -= cost;
                    true
                })
                .unwrap();
            assert_eq!(frame.slot, slot + 1);
            assert_eq!(complete, slot + 1 == slots);
            assert_eq!(memory.admitted_bytes(), bytes);
        }
        assert_eq!(store.rows_from(0).count(), 0);
    }

    #[test]
    fn directory_and_object_indexes_retain_every_work_cut() {
        for cached in [false, true] {
            let input = || {
                let mut store =
                    RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
                for index in 0..150 {
                    store
                        .insert_row([&format!("s{index:03}"), "p", &format!("o{:03}", index % 11)])
                        .unwrap();
                }
                store.insert_row(["retired", "empty", "o"]).unwrap();
                let empty =
                    ["retired", "empty", "o"].map(|surface| store.term_id(surface).unwrap());
                store.remove_ids(empty).unwrap();
                if cached {
                    for relation in &store.relations {
                        for batch in &relation.batches {
                            batch.object_order();
                        }
                    }
                }
                store
            };
            let mut reference = input();
            let mut resident = Resident;
            let mut memory = Memory::new(&mut resident);
            reference.admit_native_metadata_in(&mut memory).unwrap();
            let mut frame = Preparation::default();
            let mut total = 0;
            assert!(
                frame
                    .resume(&mut reference, &mut memory, |cost| {
                        total += cost;
                        true
                    })
                    .unwrap()
            );
            let expected_bytes = memory.admitted_bytes();
            let expected = reference.rows_from(0).collect::<Vec<_>>();
            for limit in 0..=total {
                let mut store = input();
                let mut resident = Resident;
                let mut memory = Memory::new(&mut resident);
                store.admit_native_metadata_in(&mut memory).unwrap();
                let mut frame = Preparation::default();
                let mut used = 0;
                let complete = frame
                    .resume(&mut store, &mut memory, |cost| {
                        if used + cost > limit {
                            false
                        } else {
                            used += cost;
                            true
                        }
                    })
                    .unwrap();
                assert_eq!(store.rows_from(0).collect::<Vec<_>>(), expected);
                if !complete {
                    assert!(
                        frame
                            .resume(&mut store, &mut memory, |cost| {
                                used += cost;
                                true
                            })
                            .unwrap()
                    );
                }
                assert_eq!(used, total, "cached={cached}, work cut={limit}");
                assert_eq!(memory.admitted_bytes(), expected_bytes);
                for relation in &store.relations {
                    for batch in &relation.batches {
                        let permutation = &batch.object_index.get().unwrap().perm;
                        assert!(batch.object_index.get().unwrap().admitted);
                        assert!(permutation.windows(2).all(|pair| {
                            index::compare_row_positions(
                                pair[0],
                                pair[1],
                                &[batch.data.column(0), batch.data.column(1)],
                                &[1, 0],
                            )
                            .is_le()
                        }));
                    }
                }
            }
        }
    }
}
