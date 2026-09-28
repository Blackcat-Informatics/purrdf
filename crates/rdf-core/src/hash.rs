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

/// Hash an IRI for the primary term index, including its variant tag.
///
/// The builder, global dictionary and frozen dataset must use this exact
/// protocol for their borrowed and stored lookups. An IRI is the whole key,
/// so the native hasher can compress its bytes and length in one operation.
#[inline]
pub(crate) fn hash_iri_for_interner(iri: &str) -> u64 {
    purrdf_hash::fixed::FixedHasher::hash_terminal(0, iri.as_bytes())
}

/// The scope and variant share one word; the byte write already mixes its
/// length, so a string terminator would add a redundant state transition.
#[inline]
pub(crate) fn hash_blank_for_interner(label: &str, scope: u32) -> u64 {
    use core::hash::Hasher;
    let mut hash = purrdf_hash::fixed::FixedHasher::default();
    hash.write_u64(u64::from(scope) | (1 << 32));
    hash.write(label.as_bytes());
    hash.finish()
}

/// One protocol for borrowed, stored and frozen literal keys. Keep the full
/// 64-bit datatype id for global dictionaries, beside packed variant,
/// language-presence and direction fields. Each string write mixes its own
/// length; `None` remains distinct from `Some("")` through the presence bit.
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
    hash.write_u128(u128::from(datatype) | (metadata << 64));
    hash.write(lexical.as_bytes());
    if let Some(language) = language {
        hash.write(language.as_bytes());
    }
    hash.finish()
}

/// Triple ids retain separate word writes: packing them widened arithmetic
/// without improving the measured lookup cost.
#[inline]
pub(crate) fn hash_triple_for_interner(s: u64, p: u64, o: u64) -> u64 {
    use core::hash::Hasher;
    let mut hash = purrdf_hash::fixed::FixedHasher::default();
    hash.write_u8(3);
    hash.write_u64(s);
    hash.write_u64(p);
    hash.write_u64(o);
    hash.finish()
}

#[cfg(test)]
mod tests {
    use super::{hash_blank_for_interner, hash_literal_for_interner};
    use crate::RdfTextDirection::{Ltr, Rtl};

    #[test]
    fn packed_fields_preserve_boundaries_presence_and_high_id_bits() {
        let mut seen = std::collections::HashSet::new();
        for lexical in ["", "a", "ab", "abc", "a\0b", "a\u{ff}b"] {
            for datatype in [0, 1, u64::from(u32::MAX), 1 << 32, 1 << 63, u64::MAX] {
                for language in [None, Some(""), Some("a"), Some("ab"), Some("bc")] {
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
        assert_eq!(seen.len(), 564);
    }
}
