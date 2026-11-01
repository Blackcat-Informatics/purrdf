// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The workspace's single, fixed-key hashing policy for in-memory lookup tables.
//!
//! Every hash map / set on a hot IR or evaluator path uses [`FastHasher`] —
//! the workspace's **fixed-key**, non-cryptographic
//! [`FixedHasher`](purrdf_hash::fixed::FixedHasher). The keys are compile-time
//! constants (no runtime RNG seeding), which keeps the dependency tree
//! wasm-clean and free of per-process nondeterminism.
//!
//! # Determinism is never hash-order
//!
//! These aliases are a performance policy, not an ordering guarantee. Iteration
//! order over a [`FastMap`]/[`FastSet`]/[`IdSet`] is **unspecified** and must
//! never be observed by a serializer, the GTS writer, or any byte-stable egress.
//! Determinism in this workspace comes exclusively from **id-sorting** and
//! **`BTree` boundaries** applied before egress — never from hash iteration
//! order. Use these types for lookup and membership; sort explicitly when order
//! matters.

/// Shared native table engine and its original allocation refusals.
pub mod table;

use purrdf_hash::fixed::FixedHasher;
use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};

/// The workspace's fixed-key hasher builder,
/// [`FixedState`](purrdf_hash::fixed::FixedState). Non-cryptographic, no
/// runtime RNG seeding — see the [module docs](self) for the determinism policy.
pub type FastHasher = purrdf_hash::fixed::FixedState;

/// A [`std::collections::HashMap`] keyed by the workspace [`FastHasher`].
pub type FastMap<K, V> = std::collections::HashMap<K, V, FastHasher>;

/// A [`std::collections::HashSet`] hashed by the workspace [`FastHasher`].
pub type FastSet<T> = std::collections::HashSet<T, FastHasher>;

/// A [`FastSet`] of interned [`TermId`](crate::TermId)s — the common id-membership set.
pub type IdSet = FastSet<crate::TermId>;

/// Checked allocation bound for a fresh `table::HashTable<T>` holding
/// `capacity` elements. This is a physical layout certificate, not a governor
/// proxy. It includes bucket storage, control bytes and their alignment.
///
/// The shared native table preserves its pinned small-table minimum: at most a
/// 16-byte control group and its larger tables reserve one eighth of buckets.
/// The 16-byte bound also covers the portable eight-byte implementation. A
/// change to those allocation rules must update this physical certificate.
#[must_use]
pub fn hash_table_allocation_bound<T>(capacity: usize) -> Option<usize> {
    if capacity == 0 {
        return Some(0);
    }
    let buckets = if capacity < 15 {
        let minimum = match size_of::<T>() {
            0..=1 => 14,
            2..=3 => 7,
            _ => 3,
        };
        match capacity.max(minimum) {
            0..=3 => 4,
            4..=7 => 8,
            _ => 16,
        }
    } else {
        (capacity.checked_mul(8)? / 7).checked_next_power_of_two()?
    };
    table_bucket_allocation_bound::<T>(buckets)
}

/// Retained physical table bound, independent of deletion tombstones.
///
/// `capacity()` measures insertion headroom, which can fall after `retain` and
/// rise on reinsertion without changing the allocation. Native buffer owners
/// must use the actual bucket count when resuming or refunding their grant.
#[must_use]
pub fn hash_table_retained_bound<T>(table: &table::HashTable<T>) -> Option<usize> {
    if table.allocation_size() == 0 {
        return Some(0);
    }
    table_bucket_allocation_bound::<T>(table.num_buckets())
}

fn table_bucket_allocation_bound<T>(buckets: usize) -> Option<usize> {
    let alignment = align_of::<T>().max(16);
    let controls = buckets
        .checked_mul(size_of::<T>())?
        .checked_next_multiple_of(alignment)?;
    let bytes = controls.checked_add(buckets)?.checked_add(16)?;
    std::alloc::Layout::from_size_align(bytes, alignment)
        .ok()
        .map(|layout| layout.size())
}

/// The [`FastHasher`] hash of `value`: the bucket hash of the IR's store-once
/// tables and interners. Equal values hash alike within one build; the hash only
/// chooses a bucket, is never persisted, and never orders an output.
pub(crate) use purrdf_hash::fixed::hash_one as hash_of;

/// The coarse size fingerprint of a dataset holding `quads` quads over `terms`
/// distinct terms: the answer every counted backend gives to
/// [`DatasetView::stats_fingerprint`](crate::DatasetView::stats_fingerprint).
///
/// A *cache discriminator* for a dataset-aware cache key (a join-order cache),
/// not a content digest: a collision can only make a cache reuse an order
/// computed for a same-size dataset, which is at worst suboptimal. It is a
/// [`FixedState`](purrdf_hash::fixed::FixedState) hash, so it is stable within
/// one build and never persisted.
#[inline]
pub(crate) fn stats_fingerprint(quads: u64, terms: u64) -> u64 {
    use core::hash::BuildHasher as _;
    FastHasher::new().hash_one((quads, terms))
}

/// The direct interner protocols below never use an infallible `Hash` adapter.
/// Every variable byte field enters the original native body once, with its
/// original write boundary; metadata keeps its specialized integer writes.
struct KeyHash<'m, 's, S: Admission + ?Sized> {
    hash: FixedHasher,
    memory: &'m mut Memory<'s, S>,
}

impl<'m, 's, S: Admission + ?Sized> KeyHash<'m, 's, S> {
    #[inline]
    fn new(memory: &'m mut Memory<'s, S>) -> Result<Self, StorageError> {
        memory.admission_mut().checkpoint(0)?;
        Ok(Self {
            hash: FixedHasher::default(),
            memory,
        })
    }

    #[inline]
    fn read(&mut self, bytes: usize) -> Result<(), StorageError> {
        self.memory.admission_mut().checkpoint(bytes as u64)
    }

    #[inline]
    fn word(&mut self, word: u64) -> Result<(), StorageError> {
        use core::hash::Hasher;
        self.read(8)?;
        self.hash.write_u64(word);
        Ok(())
    }

    #[inline]
    fn wide(&mut self, word: u128) -> Result<(), StorageError> {
        use core::hash::Hasher;
        self.read(16)?;
        self.hash.write_u128(word);
        Ok(())
    }

    #[inline]
    fn bytes(&mut self, bytes: &[u8]) -> Result<(), StorageError> {
        self.hash.try_write(bytes, &mut |units| {
            self.memory.admission_mut().checkpoint(units)
        })
    }

    #[inline]
    fn finish(self) -> Result<u64, StorageError> {
        use core::hash::Hasher;
        self.memory.admission_mut().checkpoint(0)?;
        Ok(self.hash.finish())
    }
}

#[inline]
pub(crate) fn resident_hash(
    body: impl FnOnce(&mut Memory<'_, Resident>) -> Result<u64, StorageError>,
) -> u64 {
    let mut resident = Resident;
    body(&mut Memory::new(&mut resident))
        .expect("resident table hashing has no fallible storage or progress operation")
}

/// Hash an IRI for the primary term index, including its variant tag.
///
/// The builder, global dictionary and frozen dataset must use this exact
/// protocol for their borrowed and stored lookups. An IRI is the whole key,
/// so the native hasher can compress its bytes and length in one operation.
#[inline]
pub(crate) fn hash_iri_for_interner(iri: &str) -> u64 {
    resident_hash(|memory| hash_iri_for_interner_with_memory(iri, memory))
}

/// The same terminal-key hash through the original caller's work authority.
/// Refusal returns before any hash can be published or index updated.
#[inline]
pub(crate) fn hash_iri_for_interner_with_memory<S: Admission + ?Sized>(
    iri: &str,
    memory: &mut Memory<'_, S>,
) -> Result<u64, StorageError> {
    FixedHasher::try_hash_terminal(0, iri.as_bytes(), &mut |units| {
        memory.admission_mut().checkpoint(units)
    })
}

/// Pack up to sixteen bytes without allocation; callers encode both field
/// lengths in metadata before using this in an integer-only hash protocol.
#[inline]
fn packed_text(first: &[u8], second: &[u8]) -> u128 {
    let packed = purrdf_hash::fixed::pack_short_bytes(first);
    if second.is_empty() {
        packed
    } else {
        packed | (purrdf_hash::fixed::pack_short_bytes(second) << (8 * first.len()))
    }
}

/// Short labels share integer blocks with scope, variant and length.
/// Longer labels retain their original individually framed byte blocks.
#[inline]
pub(crate) fn hash_blank_for_interner(label: &str, scope: u32) -> u64 {
    resident_hash(|memory| hash_blank_for_interner_with_memory(label, scope, memory))
}

#[inline]
pub(crate) fn hash_blank_for_interner_with_memory<S: Admission + ?Sized>(
    label: &str,
    scope: u32,
    memory: &mut Memory<'_, S>,
) -> Result<u64, StorageError> {
    if label.len() <= 16 {
        hash_short_blank(label.as_bytes(), scope, memory)
    } else {
        hash_blank_bytes_for_interner_with_memory(label.len(), label.bytes(), scope, memory)
    }
}

#[inline]
fn hash_short_blank<S: Admission + ?Sized>(
    bytes: &[u8],
    scope: u32,
    memory: &mut Memory<'_, S>,
) -> Result<u64, StorageError> {
    let mut hash = KeyHash::new(memory)?;
    let metadata = u64::from(scope) | (1 << 32) | ((bytes.len() as u64) << 40);
    hash.read(bytes.len())?;
    let text = packed_text(bytes, &[]);
    if bytes.len() <= 8 {
        hash.wide(text | (u128::from(metadata) << 64))?;
    } else {
        hash.word(metadata)?;
        hash.wide(text)?;
    }
    hash.finish()
}

/// The same blank-key protocol for a decoded lexical view. Fixed-size blocks
/// avoid materializing an escaped label merely to probe an existing identity.
#[inline]
pub(crate) fn hash_blank_bytes_for_interner(
    len: usize,
    bytes: impl Iterator<Item = u8>,
    scope: u32,
) -> u64 {
    resident_hash(|memory| hash_blank_bytes_for_interner_with_memory(len, bytes, scope, memory))
}

#[inline]
pub(crate) fn hash_blank_bytes_for_interner_with_memory<S: Admission + ?Sized>(
    len: usize,
    mut bytes: impl Iterator<Item = u8>,
    scope: u32,
    memory: &mut Memory<'_, S>,
) -> Result<u64, StorageError> {
    memory.admission_mut().checkpoint(0)?;
    let mut block = [0_u8; 32];
    if len <= 16 {
        memory.admission_mut().checkpoint(len as u64)?;
        for slot in &mut block[..len] {
            *slot = bytes.next().expect("declared blank label byte length");
        }
        return hash_short_blank(&block[..len], scope, memory);
    }
    let mut hash = KeyHash::new(memory)?;
    hash.word(u64::from(scope) | (1 << 32))?;
    // Framing each fixed-size block through the existing hasher preserves
    // length distinctions, including a short final block. This is an internal
    // index protocol, never a persisted hash or an ordering key.
    let mut remaining = len;
    while remaining != 0 {
        let count = remaining.min(block.len());
        hash.read(count)?;
        for slot in &mut block[..count] {
            *slot = bytes.next().expect("declared blank label byte length");
        }
        hash.bytes(&block[..count])?;
        remaining -= count;
    }
    hash.finish()
}

/// One protocol for borrowed, stored and frozen literal keys. Keep the full
/// 64-bit datatype id for global dictionaries, beside packed variant,
/// language-presence and direction fields. Short strings include explicit
/// lengths in their metadata; longer strings use length-aware byte writes.
/// `None` remains distinct from `Some("")` through the presence bit.
#[inline]
pub(crate) fn hash_literal_for_interner(
    lexical: &str,
    datatype: u64,
    language: Option<&str>,
    direction: Option<crate::RdfTextDirection>,
) -> u64 {
    resident_hash(|memory| {
        hash_literal_for_interner_with_memory(lexical, datatype, language, direction, memory)
    })
}

#[inline]
pub(crate) fn hash_literal_for_interner_with_memory<S: Admission + ?Sized>(
    lexical: &str,
    datatype: u64,
    language: Option<&str>,
    direction: Option<crate::RdfTextDirection>,
    memory: &mut Memory<'_, S>,
) -> Result<u64, StorageError> {
    let mut hash = KeyHash::new(memory)?;
    let direction = match direction {
        None => 0u128,
        Some(crate::RdfTextDirection::Ltr) => 1,
        Some(crate::RdfTextDirection::Rtl) => 2,
    };
    let metadata = 2 | (u128::from(language.is_some()) << 2) | (direction << 3);
    let language_bytes = language.unwrap_or("").as_bytes();
    if lexical.len() <= 16 && language_bytes.len() <= 16 - lexical.len() {
        hash.read(lexical.len() + language_bytes.len())?;
        let text = packed_text(lexical.as_bytes(), language_bytes);
        let lengths = ((lexical.len() as u128) << 8) | ((language_bytes.len() as u128) << 13);
        if lexical.len() + language_bytes.len() <= 8 && u32::try_from(datatype).is_ok() {
            let high = u128::from(datatype) | ((metadata | lengths) << 32);
            hash.wide(text | (high << 64))?;
        } else {
            hash.wide(u128::from(datatype) | ((metadata | lengths) << 64))?;
            hash.wide(text)?;
        }
        return hash.finish();
    }
    hash.wide(u128::from(datatype) | (metadata << 64))?;
    hash.bytes(lexical.as_bytes())?;
    if let Some(language) = language {
        hash.bytes(language.as_bytes())?;
    }
    hash.finish()
}

/// Local triple ids and their tag fit one block; full-width global ids take
/// two blocks. Every bit is preserved, including the high half of global ids.
#[inline]
pub(crate) fn hash_triple_for_interner(s: u64, p: u64, o: u64) -> u64 {
    resident_hash(|memory| hash_triple_for_interner_with_memory(s, p, o, memory))
}

#[inline]
pub(crate) fn hash_triple_for_interner_with_memory<S: Admission + ?Sized>(
    s: u64,
    p: u64,
    o: u64,
    memory: &mut Memory<'_, S>,
) -> Result<u64, StorageError> {
    let mut hash = KeyHash::new(memory)?;
    if u32::try_from(s | p | o).is_ok() {
        hash.wide(u128::from(s) | (u128::from(p) << 32) | (u128::from(o) << 64) | (3 << 96))?;
    } else {
        hash.wide(u128::from(s) | (u128::from(p) << 64))?;
        hash.wide(u128::from(o) | (3 << 64))?;
    }
    hash.finish()
}

/// Grow a native table under the original buffer admission. Both old and new
/// tables remain covered until the old allocation has been destroyed.
///
/// # Errors
/// Returns layout overflow, original admission refusal or allocator refusal.
/// A refund refusal after a successful native swap leaves the replacement live
/// and its capacity changed; its original grant remains retained.
pub fn reserve_table_with_memory<T, S: Admission + ?Sized>(
    table: &mut table::HashTable<T>,
    required: usize,
    hash: impl Fn(&T) -> u64,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    try_reserve_table_with_memory(table, required, |value, _| Ok(hash(value)), memory)
}

/// The native table growth home with fallible hashing through the same original
/// account. Callback state retains its own grants on every outcome; only the
/// replacement layout is refunded after native transactional failure.
///
/// # Errors
/// Returns the original work/admission refusal or native allocation failure.
pub fn try_reserve_table_with_memory<T, S: Admission + ?Sized>(
    table: &mut table::HashTable<T>,
    required: usize,
    mut hash: impl FnMut(&T, &mut Memory<'_, S>) -> Result<u64, StorageError>,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    if required <= table.capacity() {
        return Ok(());
    }
    // Refuse original work/stop authority before growth can mutate or refuse
    // admission. The established no-allocation fast path remains unchanged.
    memory.admission_mut().checkpoint(0)?;
    let old = hash_table_retained_bound(table).ok_or(StorageError::SizeOverflow)?;
    let additional = required
        .checked_sub(table.len())
        .ok_or(StorageError::SizeOverflow)?;
    // required > items + growth_left, so additional > growth_left and the
    // native reserve plan necessarily contains a replacement capacity.
    let capacity = table
        .capacity_for_reserve(additional)
        .map_err(|_| StorageError::SizeOverflow)?
        .expect("required insertion headroom exceeds current table capacity");
    let next = hash_table_allocation_bound::<T>(capacity).ok_or(StorageError::SizeOverflow)?;
    memory.add_bytes(next)?;
    let reserved = table
        .try_reserve_with_control(additional, &mut hash, memory)
        .map_err(table_storage_error);
    if let Err(cause) = reserved {
        // The native failed replacement has died. Hash callbacks may retain
        // additional original-account state, so refund only this exact layout.
        let _ = memory.release_bytes(next);
        return Err(cause);
    }
    // The successful native swap already destroyed the original allocation.
    memory.release_bytes(old)
}

/// Preserve a native table's physical or original admission refusal.
pub fn table_storage_error(
    cause: purrdf_lex::control::ControlledError<table::TryReserveError, StorageError>,
) -> StorageError {
    match cause {
        purrdf_lex::control::ControlledError::Control(cause) => cause,
        purrdf_lex::control::ControlledError::Native(table::TryReserveError::CapacityOverflow) => {
            StorageError::SizeOverflow
        }
        purrdf_lex::control::ControlledError::Native(table::TryReserveError::AllocError {
            ..
        }) => StorageError::AllocationFailed,
    }
}

/// Grow a fixed-key map at the same physical table home as native hash tables.
///
/// # Errors
/// Returns layout overflow, original admission or allocator refusal.
#[expect(
    clippy::implicit_hasher,
    reason = "physical growth preserves the workspace's required fixed-key native map"
)]
pub fn reserve_map_with_memory<K: Eq + core::hash::Hash, V, S: Admission + ?Sized>(
    map: &mut FastMap<K, V>,
    required: usize,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    let mut retained =
        hash_table_allocation_bound::<(K, V)>(map.capacity()).ok_or(StorageError::SizeOverflow)?;
    reserve_map_with_retained_memory(map, &mut retained, required, memory)
}

/// Grow a deletion-capable fixed-key map using its original retained charge.
///
/// The owner initializes `retained` to zero with an empty map, keeps it beside
/// that map through removals, and uses it for final buffer release. The std map
/// exposes insertion headroom, not physical bucket count, after deletion.
///
/// # Errors
/// Returns layout overflow, original admission refusal or allocator refusal.
#[expect(
    clippy::implicit_hasher,
    reason = "preserves the workspace fixed-key map"
)]
pub fn reserve_map_with_retained_memory<K: Eq + core::hash::Hash, V, S: Admission + ?Sized>(
    map: &mut FastMap<K, V>,
    retained: &mut usize,
    required: usize,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    if required <= map.capacity() {
        return Ok(());
    }
    let old = *retained;
    let next = hash_table_allocation_bound::<(K, V)>(required).ok_or(StorageError::SizeOverflow)?;
    let mut replacement = memory.scope(|memory| {
        memory.add_bytes(next)?;
        let mut replacement = FastMap::default();
        replacement
            .try_reserve(required)
            .map_err(|_| StorageError::AllocationFailed)?;
        Ok(replacement)
    })?;
    replacement.extend(map.drain());
    drop(core::mem::replace(map, replacement));
    *retained = next;
    memory.release_bytes(old)
}

#[cfg(test)]
mod tests {
    use super::{
        FastSet, hash_blank_bytes_for_interner_with_memory, hash_blank_for_interner,
        hash_blank_for_interner_with_memory, hash_iri_for_interner,
        hash_iri_for_interner_with_memory, hash_literal_for_interner,
        hash_literal_for_interner_with_memory, hash_triple_for_interner,
        hash_triple_for_interner_with_memory,
    };
    use crate::RdfTextDirection::{Ltr, Rtl};
    use purrdf_lex::allocation::{Admission, Memory, StorageError};

    #[test]
    fn packed_fields_preserve_boundaries_presence_and_high_id_bits() {
        struct Work {
            calls: usize,
            refuse: Option<usize>,
        }
        impl Admission for Work {
            fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
                assert_eq!(bytes, 0, "direct hashing does not allocate");
                Ok(())
            }
            fn progress_enabled(&self) -> bool {
                true
            }
            fn checkpoint(&mut self, units: u64) -> Result<(), StorageError> {
                assert!(units <= 64, "original native block or fixed metadata");
                let at = self.calls;
                self.calls += 1;
                if self.refuse == Some(at) {
                    Err(StorageError::AdmissionFailed)
                } else {
                    Ok(())
                }
            }
        }
        fn checked(
            expected: u64,
            sweep: bool,
            mut hash: impl FnMut(&mut Memory<'_, Work>) -> Result<u64, StorageError>,
        ) {
            let mut work = Work {
                calls: 0,
                refuse: None,
            };
            let mut memory = Memory::new(&mut work);
            assert_eq!(
                hash(&mut memory),
                Ok(expected),
                "accepting native hash parity"
            );
            assert_eq!(memory.admitted_bytes(), 0);
            let calls = work.calls;
            if sweep {
                for at in 0..calls {
                    let mut work = Work {
                        calls: 0,
                        refuse: Some(at),
                    };
                    let mut memory = Memory::new(&mut work);
                    assert_eq!(
                        hash(&mut memory),
                        Err(StorageError::AdmissionFailed),
                        "no partial hash on refusal"
                    );
                    assert_eq!(memory.admitted_bytes(), 0);
                    assert_eq!(work.calls, at + 1, "no callback after refusal");
                }
            }
        }
        let mut seen = FastSet::default();
        for lexical in [
            "",
            "a",
            "ab",
            "abc",
            "a\0b",
            "a\u{ff}b",
            "12345678",
            "123456789",
            "1234567890123456",
            "12345678901234567",
        ] {
            for datatype in [0, 1, u64::from(u32::MAX), 1 << 32, 1 << 63, u64::MAX] {
                for language in [
                    None,
                    Some(""),
                    Some("a"),
                    Some("ab"),
                    Some("bc"),
                    Some("12345678"),
                    Some("12345678901234567"),
                ] {
                    for direction in [None, Some(Ltr), Some(Rtl)] {
                        let expected =
                            hash_literal_for_interner(lexical, datatype, language, direction);
                        checked(expected, false, |memory| {
                            hash_literal_for_interner_with_memory(
                                lexical, datatype, language, direction, memory,
                            )
                        });
                        assert!(
                            seen.insert(expected),
                            "distinct literal fields collided: {lexical:?}/{datatype}/{language:?}/{direction:?}"
                        );
                    }
                }
            }
            for scope in [0, 1, 1 << 31, u32::MAX] {
                let expected = hash_blank_for_interner(lexical, scope);
                checked(expected, false, |memory| {
                    hash_blank_for_interner_with_memory(lexical, scope, memory)
                });
                assert!(
                    seen.insert(expected),
                    "blank scope, label and variant must participate"
                );
            }
        }
        assert_eq!(seen.len(), 1300);
        for s in [0, 1, u64::from(u32::MAX), 1 << 32, 1 << 63, u64::MAX] {
            for p in [0, 1, u64::from(u32::MAX), 1 << 32, 1 << 63, u64::MAX] {
                for o in [0, 1, u64::from(u32::MAX), 1 << 32, 1 << 63, u64::MAX] {
                    let expected = hash_triple_for_interner(s, p, o);
                    checked(expected, false, |memory| {
                        hash_triple_for_interner_with_memory(s, p, o, memory)
                    });
                    assert!(seen.insert(expected), "triple fields collided: {s}/{p}/{o}");
                }
            }
        }
        assert_eq!(seen.len(), 1516);
        let long = "π_雪.x".repeat(513);
        for text in ["", "short", "123456789", "1234567890123456", long.as_str()] {
            checked(hash_iri_for_interner(text), true, |memory| {
                hash_iri_for_interner_with_memory(text, memory)
            });
            checked(hash_blank_for_interner(text, u32::MAX), true, |memory| {
                hash_blank_for_interner_with_memory(text, u32::MAX, memory)
            });
            checked(hash_blank_for_interner(text, u32::MAX), true, |memory| {
                hash_blank_bytes_for_interner_with_memory(
                    text.len(),
                    text.bytes(),
                    u32::MAX,
                    memory,
                )
            });
            checked(
                hash_literal_for_interner(text, u64::MAX, Some("en"), Some(Rtl)),
                true,
                |memory| {
                    hash_literal_for_interner_with_memory(
                        text,
                        u64::MAX,
                        Some("en"),
                        Some(Rtl),
                        memory,
                    )
                },
            );
        }
        checked(hash_triple_for_interner(u64::MAX, 1, 2), true, |memory| {
            hash_triple_for_interner_with_memory(u64::MAX, 1, 2, memory)
        });

        // The Core storage marker must restore the unchanged caller error at
        // its original enclosing boundary, without a later publication poll.
        struct Stop(Box<u8>);
        let marker = Box::new(7);
        let identity = core::ptr::from_ref(marker.as_ref());
        let mut marker = Some(marker);
        let mut calls = 0;
        let mut resident = purrdf_lex::allocation::Resident;
        let mut memory = Memory::new(&mut resident);
        let result = purrdf_lex::control::with_memory_control(
            &mut memory,
            &mut |_| {
                calls += 1;
                if calls == 4 {
                    Err(Stop(marker.take().expect("one original failure")))
                } else {
                    Ok(())
                }
            },
            |memory| hash_iri_for_interner_with_memory(&long, memory),
        );
        match result {
            Err(purrdf_lex::control::ControlledError::Control(Stop(marker))) => {
                assert_eq!(core::ptr::from_ref(marker.as_ref()), identity)
            }
            _ => panic!("original caller error must survive native hashing"),
        }
        assert_eq!(calls, 4);
    }
}
