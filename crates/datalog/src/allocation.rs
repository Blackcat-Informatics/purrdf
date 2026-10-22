// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native factory retirement bookkeeping under the original supplemental grant.

use purrdf_core::FastMap;
use purrdf_lex::allocation::{Admission, Memory, OrderedTable, StorageError};

pub(crate) fn reserve_ordered_in<K, V, A: Admission + ?Sized>(
    table: &mut OrderedTable<K, V>,
    memory: &mut Memory<'_, A>,
    retired: &mut usize,
) -> Result<(), StorageError> {
    let before = memory.admitted_bytes();
    let old = table.storage_bytes()?;
    let result = table.reserve_insert(memory);
    if result.is_err() {
        record_factory_retirement(
            retired,
            before,
            old,
            table.storage_bytes()?,
            memory.admitted_bytes(),
        );
    }
    result
}

pub(crate) fn reserve_index_in<K: Eq + std::hash::Hash, V, A: Admission + ?Sized>(
    map: &mut FastMap<K, V>,
    retained: &mut usize,
    additional: usize,
    memory: &mut Memory<'_, A>,
    retired: &mut usize,
) -> Result<(), StorageError> {
    let required = map
        .len()
        .checked_add(additional)
        .ok_or(StorageError::SizeOverflow)?;
    if required <= map.capacity() {
        return Ok(());
    }
    let before = memory.admitted_bytes();
    let old = *retained;
    let result =
        purrdf_core::hash::reserve_map_with_retained_memory(map, retained, required, memory);
    if result.is_err() {
        record_factory_retirement(retired, before, old, *retained, memory.admitted_bytes());
    }
    result
}

/// Native factories already admit their replacement before allocation. If the
/// old allocation dies but its shrink refuses, retain that still-held charge;
/// this bookkeeping admits no payload after construction and clones no grant.
pub(crate) fn record_factory_retirement(
    retired: &mut usize,
    before: usize,
    old: usize,
    new: usize,
    after: usize,
) {
    let retained = before
        .checked_sub(old)
        .and_then(|bytes| bytes.checked_add(new))
        .expect("a native factory retains its original protected layout");
    let released = after
        .checked_sub(retained)
        .expect("a native factory never releases a surviving layout");
    *retired = retired
        .checked_add(released)
        .expect("outstanding released layouts fit their original grant");
}

pub(crate) fn reserve_array_in<T, A: Admission + ?Sized>(
    values: &mut Vec<T>,
    memory: &mut Memory<'_, A>,
    retired: &mut usize,
    reserve: impl FnOnce(&mut Vec<T>, &mut Memory<'_, A>) -> Result<(), StorageError>,
) -> Result<(), StorageError> {
    let before = memory.admitted_bytes();
    let old = values
        .capacity()
        .checked_mul(size_of::<T>())
        .expect("an original valid array layout");
    let result = reserve(values, memory);
    if result.is_err() {
        let new = values
            .capacity()
            .checked_mul(size_of::<T>())
            .expect("a native valid array layout");
        record_factory_retirement(retired, before, old, new, memory.admitted_bytes());
    }
    result
}

pub(crate) fn reserve_array_growth<T, A: Admission + ?Sized>(
    values: &mut Vec<T>,
    required: usize,
    memory: &mut Memory<'_, A>,
    retired: &mut usize,
) -> Result<(), StorageError> {
    reserve_array_in(values, memory, retired, |values, memory| {
        let capacity = purrdf_lex::allocation::growth_capacity::<T>(values.capacity(), required)?;
        memory.reserve(values, capacity)
    })
}

pub(crate) fn flush_retired_bytes<A: Admission + ?Sized>(
    bytes: &mut usize,
    memory: &mut Memory<'_, A>,
) -> Result<(), StorageError> {
    memory.release_bytes(*bytes)?;
    *bytes = 0;
    Ok(())
}
