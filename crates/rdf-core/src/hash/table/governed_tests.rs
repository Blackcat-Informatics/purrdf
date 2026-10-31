// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native group refusal, rollback, resource ownership and healthy neighbours.

use super::alloc::Allocator;
use super::{Global, HashTable, TryReserveError};
use purrdf_lex::allocation::{Admission, Memory, StorageError};
use purrdf_lex::control::{ControlledError, Unmetered};
use std::alloc::Layout;
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(Debug, PartialEq, Eq)]
struct Stop(usize);

fn hash_with_tag(tag: u8, bucket: usize) -> u64 {
    (u64::from(tag) << (usize::BITS.min(64) - 7)) | bucket as u64
}

#[test]
fn probes_without_fingerprint_matches_stop_and_tombstones_remain_reusable() {
    let hash = hash_with_tag(1, 0);
    let mut table = HashTable::with_capacity(128);
    for value in 0..70 {
        table.insert_unique(hash, value, |_| hash);
    }
    let mut groups = 0;
    let mut equality = 0;
    let stopped = table.find_with_control(
        hash_with_tag(2, 0),
        |_, _| {
            equality += 1;
            Ok(false)
        },
        &mut |_| {
            groups += 1;
            if groups == 2 { Err(Stop(37)) } else { Ok(()) }
        },
    );
    assert_eq!(stopped, Err(Stop(37)));
    assert_eq!(groups, 2);
    assert_eq!(
        equality, 0,
        "absent fingerprints still incur native group work"
    );
    assert_eq!(table.find(hash, |&value| value == 69), Some(&69));

    for value in 0..35 {
        table
            .find_entry(hash, |&stored| stored == value)
            .unwrap()
            .remove();
    }
    let allocation = table.num_buckets();
    table
        .try_insert_unique_with_control(hash, 90, |_, _| Ok(hash), &mut Unmetered)
        .unwrap();
    assert_eq!(
        table.num_buckets(),
        allocation,
        "native tombstone reuse avoids growth"
    );
    assert_eq!(table.find(hash, |&value| value == 90), Some(&90));
    let mut calls = 0;
    let failed = table
        .try_insert_unique_with_control(hash, 91, |_, _| Ok(hash), &mut |_| {
            calls += 1;
            Err(Stop(41))
        })
        .unwrap_err();
    assert_eq!(failed.value, 91);
    assert_eq!(failed.cause, ControlledError::Control(Stop(41)));
    assert_eq!(calls, 1);
    assert!(table.find(hash, |&value| value == 91).is_none());
}

#[test]
fn sparse_native_iteration_stops_between_occupied_groups_and_can_resume() {
    let mut table = HashTable::with_capacity(128);
    table.insert_unique(hash_with_tag(1, 0), 0, |&value| hash_with_tag(1, value));
    table.insert_unique(hash_with_tag(1, 96), 96, |&value| hash_with_tag(1, value));
    let mut iter = table.iter_with_control(&mut Unmetered).unwrap();
    assert_eq!(iter.next_with_control(&mut Unmetered), Ok(Some(&0)));
    let mut groups = 0;
    let stopped = iter.next_with_control(&mut |units| {
        if units > 1 {
            groups += 1;
            if groups == 2 {
                return Err(Stop(43));
            }
        }
        Ok(())
    });
    assert_eq!(stopped, Err(Stop(43)));
    assert_eq!(
        groups, 2,
        "empty native groups are charged before being scanned"
    );
    assert_eq!(iter.next_with_control(&mut Unmetered), Ok(Some(&96)));
    assert_eq!(iter.next_with_control(&mut Unmetered), Ok(None));
}

#[derive(Clone)]
struct TestAllocator {
    refuse: Arc<AtomicBool>,
    live: Arc<AtomicUsize>,
}

// SAFETY: allocations and deallocations use the same native Global allocator and
// unmodified layouts. The atomics observe ownership without changing pointers.
unsafe impl Allocator for TestAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, ()> {
        if self.refuse.load(Ordering::Relaxed) {
            return Err(());
        }
        let block = Global.allocate(layout)?;
        self.live.fetch_add(layout.size(), Ordering::Relaxed);
        Ok(block)
    }
    unsafe fn deallocate(&self, pointer: NonNull<u8>, layout: Layout) {
        // SAFETY: raw tables return each allocation with its original layout.
        unsafe {
            Global.deallocate(pointer, layout);
        }
        self.live.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}

#[derive(Debug)]
struct Entry {
    id: usize,
    // Inline payload exercises interruption between bounded raw copy extents.
    payload: [u8; 600],
    drops: Arc<Vec<AtomicUsize>>,
}
impl Drop for Entry {
    fn drop(&mut self) {
        self.drops[self.id].fetch_add(1, Ordering::Relaxed);
    }
}

fn entries(
    allocator: TestAllocator,
    drops: &Arc<Vec<AtomicUsize>>,
) -> HashTable<Entry, TestAllocator> {
    let mut table = HashTable::with_capacity_in(20, allocator);
    for id in 0..18 {
        table.insert_unique(
            hash_with_tag(1, 0),
            Entry {
                id,
                payload: [id as u8; 600],
                drops: Arc::clone(drops),
            },
            |_| hash_with_tag(1, 0),
        );
    }
    table
}
fn drop_counts() -> Arc<Vec<AtomicUsize>> {
    Arc::new((0..19).map(|_| AtomicUsize::new(0)).collect())
}
fn allocator() -> TestAllocator {
    TestAllocator {
        refuse: Arc::new(AtomicBool::new(false)),
        live: Arc::new(AtomicUsize::new(0)),
    }
}
fn originals_survive(table: &HashTable<Entry, TestAllocator>, drops: &Arc<Vec<AtomicUsize>>) {
    assert_eq!(table.len(), 18);
    for id in 0..18 {
        let entry = table
            .find(hash_with_tag(1, 0), |entry| entry.id == id)
            .unwrap();
        assert_eq!(entry.payload, [id as u8; 600]);
        assert_eq!(drops[id].load(Ordering::Relaxed), 0);
    }
}

#[test]
fn transactional_growth_refunds_every_refusal_without_owning_copied_entries() {
    let original_allocator = allocator();
    let drops = drop_counts();
    let mut table = entries(original_allocator.clone(), &drops);
    let mut checkpoints = 0;
    table
        .try_reserve_with_control(100, |_, _| Ok(hash_with_tag(1, 0)), &mut |_| {
            checkpoints += 1;
            Ok::<(), Stop>(())
        })
        .unwrap();
    originals_survive(&table, &drops);
    drop(table);
    assert_eq!(original_allocator.live.load(Ordering::Relaxed), 0);
    assert!(
        drops[..18]
            .iter()
            .all(|drops| drops.load(Ordering::Relaxed) == 1)
    );

    for stop_at in 0..checkpoints {
        let original_allocator = allocator();
        let drops = drop_counts();
        let mut table = entries(original_allocator.clone(), &drops);
        let original_live = original_allocator.live.load(Ordering::Relaxed);
        let mut calls = 0;
        let failed =
            table.try_reserve_with_control(100, |_, _| Ok(hash_with_tag(1, 0)), &mut |_| {
                let index = calls;
                calls += 1;
                if index == stop_at {
                    Err(Stop(stop_at))
                } else {
                    Ok(())
                }
            });
        assert_eq!(failed, Err(ControlledError::Control(Stop(stop_at))));
        assert_eq!(calls, stop_at + 1, "never poll a refused callback again");
        originals_survive(&table, &drops);
        assert_eq!(
            original_allocator.live.load(Ordering::Relaxed),
            original_live
        );
        table
            .try_insert_unique_with_control(
                hash_with_tag(1, 0),
                Entry {
                    id: 18,
                    payload: [18; 600],
                    drops: Arc::clone(&drops),
                },
                |_, _| Ok(hash_with_tag(1, 0)),
                &mut Unmetered,
            )
            .unwrap();
        assert_eq!(table.len(), 19);
        drop(table);
        assert_eq!(original_allocator.live.load(Ordering::Relaxed), 0);
        assert!(drops.iter().all(|drops| drops.load(Ordering::Relaxed) == 1));
    }

    let original_allocator = allocator();
    let drops = drop_counts();
    let mut table = entries(original_allocator.clone(), &drops);
    let failed = table
        .try_insert_unique_with_control(
            hash_with_tag(1, 0),
            Entry {
                id: 18,
                payload: [18; 600],
                drops: Arc::clone(&drops),
            },
            |_, _| Ok(hash_with_tag(1, 0)),
            &mut |_| Err(Stop(59)),
        )
        .unwrap_err();
    assert_eq!(failed.cause, ControlledError::Control(Stop(59)));
    assert_eq!(failed.value.payload, [18; 600]);
    assert_eq!(drops[18].load(Ordering::Relaxed), 0);
    drop(failed.value);
    assert_eq!(drops[18].load(Ordering::Relaxed), 1);
    originals_survive(&table, &drops);
    let original_live = original_allocator.live.load(Ordering::Relaxed);
    original_allocator.refuse.store(true, Ordering::Relaxed);
    let failed =
        table.try_reserve_with_control(100, |_, _| Ok(hash_with_tag(1, 0)), &mut Unmetered);
    assert!(matches!(
        failed,
        Err(ControlledError::Native(TryReserveError::AllocError { .. }))
    ));
    originals_survive(&table, &drops);
    assert_eq!(
        original_allocator.live.load(Ordering::Relaxed),
        original_live
    );
    original_allocator.refuse.store(false, Ordering::Relaxed);
    table
        .try_reserve_with_control(100, |_, _| Ok(hash_with_tag(1, 0)), &mut Unmetered)
        .unwrap();
    originals_survive(&table, &drops);
    drop(table);
    assert_eq!(original_allocator.live.load(Ordering::Relaxed), 0);
    assert!(
        drops[..18]
            .iter()
            .all(|drops| drops.load(Ordering::Relaxed) == 1)
    );
}

struct Account {
    live: usize,
    growth_attempts: usize,
    limit: usize,
    interrupt: bool,
    hashes: Arc<AtomicUsize>,
}
impl Admission for Account {
    fn resize(&mut self, live: usize) -> Result<(), StorageError> {
        if live > self.live {
            self.growth_attempts += 1;
        }
        if live > self.limit {
            return Err(StorageError::AdmissionFailed);
        }
        self.live = live;
        Ok(())
    }
    fn progress_enabled(&self) -> bool {
        true
    }
    fn checkpoint(&mut self, units: u64) -> Result<(), StorageError> {
        if self.interrupt && units == 1 && self.hashes.load(Ordering::Relaxed) > 0 {
            return Err(StorageError::AdmissionFailed);
        }
        Ok(())
    }
}

#[test]
fn actual_core_capacity_helper_retains_original_admission_on_refused_rehash() {
    let hash = hash_with_tag(1, 0);
    let mut table = HashTable::with_capacity(20);
    for value in 0..18 {
        table.insert_unique(hash, value, |_| hash);
    }
    let original = crate::hash::hash_table_retained_bound(&table).unwrap();
    let hashes = Arc::new(AtomicUsize::new(0));
    let mut account = Account {
        live: 0,
        growth_attempts: 0,
        limit: original,
        interrupt: false,
        hashes: Arc::clone(&hashes),
    };
    let mut memory = Memory::new(&mut account);
    memory.add_bytes(original).unwrap();
    // The enclosing boundary admits entry, then the original helper checkpoint
    // refuses. Even a growth-refusing account must never see an allocation
    // admission first or replace the typed caller cause with its own refusal.
    let growth_attempts = memory.admission_mut().growth_attempts;
    let mut callbacks = 0;
    let refused = purrdf_lex::control::with_memory_control(
        &mut memory,
        &mut |_| {
            callbacks += 1;
            if callbacks == 2 {
                Err(Stop(61))
            } else {
                Ok(())
            }
        },
        |memory| crate::hash::reserve_table_with_memory(&mut table, 100, |_| hash, memory),
    );
    assert_eq!(refused, Err(ControlledError::Control(Stop(61))));
    assert_eq!(callbacks, 2);
    assert_eq!(memory.admission_mut().growth_attempts, growth_attempts);
    assert_eq!(memory.admitted_bytes(), original);
    assert_eq!(table.len(), 18);
    assert_eq!(
        crate::hash::reserve_table_with_memory(&mut table, 100, |_| hash, &mut memory),
        Err(StorageError::AdmissionFailed)
    );
    assert_eq!(memory.admitted_bytes(), original);
    memory.admission_mut().limit = usize::MAX;
    memory.admission_mut().interrupt = true;
    assert_eq!(
        crate::hash::reserve_table_with_memory(
            &mut table,
            100,
            |_| {
                hashes.fetch_add(1, Ordering::Relaxed);
                hash
            },
            &mut memory
        ),
        Err(StorageError::AdmissionFailed)
    );
    assert!(hashes.load(Ordering::Relaxed) > 0);
    assert_eq!(memory.admitted_bytes(), original);
    for value in 0..18 {
        assert_eq!(table.find(hash, |&stored| stored == value), Some(&value));
    }
    memory.admission_mut().interrupt = false;
    crate::hash::reserve_table_with_memory(&mut table, 100, |_| hash, &mut memory).unwrap();
    let retained = crate::hash::hash_table_retained_bound(&table).unwrap();
    assert_eq!(memory.admitted_bytes(), retained);
    drop(table);
    memory.release_bytes(retained).unwrap();
    assert_eq!(memory.admitted_bytes(), 0);
}
