// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The workspace's single, fixed-key hashing policy for in-memory lookup tables.
//!
//! Every hash map / set on a hot IR or evaluator path uses [`FastHasher`] — a
//! **fixed-key**, non-cryptographic [`ahash`] hasher. The key is fixed (no
//! runtime RNG seeding), which keeps the dependency tree wasm-clean and free of
//! per-process nondeterminism.
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

/// The workspace fixed-key [`ahash`] hasher builder. Non-cryptographic, no
/// runtime RNG seeding — see the [module docs](self) for the determinism policy.
pub type FastHasher = core::hash::BuildHasherDefault<ahash::AHasher>;

/// A [`std::collections::HashMap`] keyed by the workspace [`FastHasher`].
pub type FastMap<K, V> = std::collections::HashMap<K, V, FastHasher>;

/// A [`std::collections::HashSet`] hashed by the workspace [`FastHasher`].
pub type FastSet<T> = std::collections::HashSet<T, FastHasher>;

/// A [`FastSet`] of interned [`TermId`](crate::TermId)s — the common id-membership set.
pub type IdSet = FastSet<crate::TermId>;
