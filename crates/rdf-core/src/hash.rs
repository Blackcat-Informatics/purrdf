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

/// Checked allocation bound for a fresh `hashbrown::HashTable<T>` holding
/// `capacity` elements. This is a physical layout certificate, not a governor
/// proxy. It includes bucket storage, control bytes and their alignment.
///
/// The workspace pins hashbrown 0.17: its small-table minimum uses at most a
/// 16-byte control group and its larger tables reserve one eighth of buckets.
/// The 16-byte bound also covers the portable eight-byte implementation. A
/// dependency upgrade changing those allocation rules must update this home.
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
pub fn hash_table_retained_bound<T>(table: &hashbrown::HashTable<T>) -> Option<usize> {
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

/// Hash an IRI for the primary term index, including its variant tag.
///
/// The builder, global dictionary and frozen dataset must use this exact
/// protocol for their borrowed and stored lookups. An IRI is the whole key,
/// so the native hasher can compress its bytes and length in one operation.
#[inline]
pub(crate) fn hash_iri_for_interner(iri: &str) -> u64 {
    purrdf_hash::fixed::FixedHasher::hash_terminal(0, iri.as_bytes())
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
/// Longer labels use a byte write that already incorporates its length.
#[inline]
pub(crate) fn hash_blank_for_interner(label: &str, scope: u32) -> u64 {
    if label.len() <= 16 {
        hash_short_blank(label.as_bytes(), scope)
    } else {
        hash_blank_bytes_for_interner(label.len(), label.bytes(), scope)
    }
}

#[inline]
fn hash_short_blank(bytes: &[u8], scope: u32) -> u64 {
    use core::hash::Hasher;
    let mut hash = purrdf_hash::fixed::FixedHasher::default();
    let metadata = u64::from(scope) | (1 << 32) | ((bytes.len() as u64) << 40);
    let text = packed_text(bytes, &[]);
    if bytes.len() <= 8 {
        hash.write_u128(text | (u128::from(metadata) << 64));
    } else {
        hash.write_u64(metadata);
        hash.write_u128(text);
    }
    hash.finish()
}

/// The same blank-key protocol for a decoded lexical view. Fixed-size blocks
/// avoid materializing an escaped label merely to probe an existing identity.
#[inline]
pub(crate) fn hash_blank_bytes_for_interner(
    len: usize,
    mut bytes: impl Iterator<Item = u8>,
    scope: u32,
) -> u64 {
    use core::hash::Hasher;
    let mut hash = purrdf_hash::fixed::FixedHasher::default();
    let mut block = [0_u8; 32];
    if len <= 16 {
        for slot in &mut block[..len] {
            *slot = bytes.next().expect("declared blank label byte length");
        }
        return hash_short_blank(&block[..len], scope);
    }
    hash.write_u64(u64::from(scope) | (1 << 32));
    // Framing each fixed-size block through the existing hasher preserves
    // length distinctions, including a short final block. This is an internal
    // index protocol, never a persisted hash or an ordering key.
    let mut remaining = len;
    while remaining != 0 {
        let count = remaining.min(block.len());
        for slot in &mut block[..count] {
            *slot = bytes.next().expect("declared blank label byte length");
        }
        hash.write(&block[..count]);
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
    use core::hash::Hasher;
    let direction = match direction {
        None => 0u128,
        Some(crate::RdfTextDirection::Ltr) => 1,
        Some(crate::RdfTextDirection::Rtl) => 2,
    };
    let metadata = 2 | (u128::from(language.is_some()) << 2) | (direction << 3);
    let mut hash = purrdf_hash::fixed::FixedHasher::default();
    let language_bytes = language.unwrap_or("").as_bytes();
    if lexical.len() <= 16 && language_bytes.len() <= 16 - lexical.len() {
        let text = packed_text(lexical.as_bytes(), language_bytes);
        let lengths = ((lexical.len() as u128) << 8) | ((language_bytes.len() as u128) << 13);
        if lexical.len() + language_bytes.len() <= 8 && u32::try_from(datatype).is_ok() {
            let high = u128::from(datatype) | ((metadata | lengths) << 32);
            hash.write_u128(text | (high << 64));
        } else {
            hash.write_u128(u128::from(datatype) | ((metadata | lengths) << 64));
            hash.write_u128(text);
        }
        return hash.finish();
    }
    hash.write_u128(u128::from(datatype) | (metadata << 64));
    hash.write(lexical.as_bytes());
    if let Some(language) = language {
        hash.write(language.as_bytes());
    }
    hash.finish()
}

/// Local triple ids and their tag fit one block; full-width global ids take
/// two blocks. Every bit is preserved, including the high half of global ids.
#[inline]
pub(crate) fn hash_triple_for_interner(s: u64, p: u64, o: u64) -> u64 {
    use core::hash::Hasher;
    let mut hash = purrdf_hash::fixed::FixedHasher::default();
    if u32::try_from(s | p | o).is_ok() {
        hash.write_u128(u128::from(s) | (u128::from(p) << 32) | (u128::from(o) << 64) | (3 << 96));
    } else {
        hash.write_u128(u128::from(s) | (u128::from(p) << 64));
        hash.write_u128(u128::from(o) | (3 << 64));
    }
    hash.finish()
}

/// Grow a native table under the original buffer admission. Both old and new
/// tables remain covered until the old allocation has been destroyed.
///
/// # Errors
/// Returns layout overflow, original admission refusal or allocator refusal.
pub fn reserve_table_with_memory<T, S: purrdf_lex::allocation::Admission + ?Sized>(
    table: &mut hashbrown::HashTable<T>,
    required: usize,
    hash: impl Fn(&T) -> u64,
    memory: &mut purrdf_lex::allocation::Memory<'_, S>,
) -> Result<(), purrdf_lex::allocation::StorageError> {
    use purrdf_lex::allocation::StorageError;
    if required <= table.capacity() {
        return Ok(());
    }
    let old = hash_table_retained_bound(table).ok_or(StorageError::SizeOverflow)?;
    let next = hash_table_allocation_bound::<T>(required).ok_or(StorageError::SizeOverflow)?;
    let mut replacement = memory.scope(|memory| {
        memory.add_bytes(next)?;
        let mut replacement = hashbrown::HashTable::new();
        replacement
            .try_reserve(required, &hash)
            .map_err(|_| StorageError::AllocationFailed)?;
        Ok(replacement)
    })?;
    for value in table.drain() {
        replacement.insert_unique(hash(&value), value, &hash);
    }
    drop(core::mem::replace(table, replacement));
    memory.release_bytes(old)
}

/// Grow a fixed-key map at the same physical table home as native hash tables.
///
/// # Errors
/// Returns layout overflow, original admission or allocator refusal.
#[expect(
    clippy::implicit_hasher,
    reason = "physical growth preserves the workspace's required fixed-key native map"
)]
pub fn reserve_map_with_memory<
    K: Eq + core::hash::Hash,
    V,
    S: purrdf_lex::allocation::Admission + ?Sized,
>(
    map: &mut FastMap<K, V>,
    required: usize,
    memory: &mut purrdf_lex::allocation::Memory<'_, S>,
) -> Result<(), purrdf_lex::allocation::StorageError> {
    use purrdf_lex::allocation::StorageError;
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
pub fn reserve_map_with_retained_memory<
    K: Eq + core::hash::Hash,
    V,
    S: purrdf_lex::allocation::Admission + ?Sized,
>(
    map: &mut FastMap<K, V>,
    retained: &mut usize,
    required: usize,
    memory: &mut purrdf_lex::allocation::Memory<'_, S>,
) -> Result<(), purrdf_lex::allocation::StorageError> {
    use purrdf_lex::allocation::StorageError;
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
        FastSet, hash_blank_for_interner, hash_literal_for_interner, hash_triple_for_interner,
    };
    use crate::RdfTextDirection::{Ltr, Rtl};

    #[test]
    fn packed_fields_preserve_boundaries_presence_and_high_id_bits() {
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
                        assert!(
                            seen.insert(hash_literal_for_interner(
                                lexical, datatype, language, direction
                            )),
                            "distinct literal fields collided: {lexical:?}/{datatype}/{language:?}/{direction:?}"
                        );
                    }
                }
            }
            for scope in [0, 1, 1 << 31, u32::MAX] {
                assert!(
                    seen.insert(hash_blank_for_interner(lexical, scope)),
                    "blank scope, label and variant must participate"
                );
            }
        }
        assert_eq!(seen.len(), 1300);
        for s in [0, 1, u64::from(u32::MAX), 1 << 32, 1 << 63, u64::MAX] {
            for p in [0, 1, u64::from(u32::MAX), 1 << 32, 1 << 63, u64::MAX] {
                for o in [0, 1, u64::from(u32::MAX), 1 << 32, 1 << 63, u64::MAX] {
                    assert!(
                        seen.insert(hash_triple_for_interner(s, p, o)),
                        "triple fields collided: {s}/{p}/{o}"
                    );
                }
            }
        }
        assert_eq!(seen.len(), 1516);
    }
}
