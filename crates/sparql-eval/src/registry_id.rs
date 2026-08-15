// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! An unforgeable per-process identity for a registry **instance** — as opposed to
//! its contents.
//!
//! [`crate::agg_fn::AggregateRegistry`] and
//! [`crate::property_fn::PropertyFunctionRegistry`] both fold this into their
//! `registry_fingerprint`, ahead of the declaration digest, because the declaration
//! digest alone cannot do the one job a plan's identity needs: two registries built
//! independently can register the SAME IRI to two DIFFERENT trait-object
//! implementations that happen to declare identical arity, volatility, and every
//! other observable metadata — indistinguishable by declaration, yet computing
//! different answers. A [`RegistryId`] gives every registry instance a value no
//! OTHER instance can ever share, so a plan prepared against one instance refuses
//! to run against any other, regardless of how similar the two describe themselves.

use std::sync::atomic::{AtomicU64, Ordering};

/// A monotonically increasing, process-lifetime-unique registry instance identity.
///
/// # Why a counter, not `Arc::as_ptr`
///
/// Pointer identity can be recycled: once an `Arc`'s backing allocation is freed,
/// a later, wholly unrelated allocation can land at the same address, so two
/// registries that are never alive at the same time could still collide on a
/// pointer-derived identity. A monotonically increasing counter never repeats a
/// value within one process's lifetime, so two [`RegistryId`]s can never collide
/// no matter what the allocator does with freed memory.
///
/// # Why per-process monotonicity is sufficient
///
/// A prepared plan is validated against a registry only within the SAME process
/// that prepared it — nothing here is serialized, persisted, or compared across a
/// process boundary. A [`crate::engine::PreparedQuery`] is itself an in-memory
/// object that cannot outlive the process that built it, so an identity that is
/// merely unique among every registry constructed during this process's lifetime
/// is exactly as strong a guarantee as the plan it protects ever needs.
///
/// # wasm32
///
/// A plain [`AtomicU64`] read and written with [`Ordering::Relaxed`] — no
/// `getrandom`, no clock, no thread id — so this stays available and
/// deterministic under a single-threaded `wasm32-unknown-unknown` build exactly
/// like the rest of this crate's determinism-sensitive state (see the fixed-key
/// `DetHashMap`/`DetHashSet` this crate uses for the same reason).
///
/// # Assigned at construction, inherited by `Clone`
///
/// [`AggregateRegistry`](crate::agg_fn::AggregateRegistry) and
/// [`PropertyFunctionRegistry`](crate::property_fn::PropertyFunctionRegistry) both
/// mint a fresh id in their `Default`/`new` (the same one path, since `new` calls
/// `Self::default()`), and both derive [`Clone`] rather than minting a new id on
/// clone. That is deliberate, not an oversight: `Clone` on either registry type
/// clones the underlying map of `Arc<dyn …>` trait objects, so a clone shares the
/// exact SAME registered implementations as its source — every call a clone can
/// resolve, it resolves to the identical code the source would have. Two values
/// that can only ever answer identically are the same registry instance for every
/// purpose a plan's identity cares about, so they keep the same id.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct RegistryId(u64);

impl RegistryId {
    /// Mint a fresh identity, distinct from every other identity minted by this
    /// function so far in this process.
    pub(crate) fn fresh() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    /// The identity every canonical `EMPTY` registry constant shares —
    /// [`AggregateRegistry::EMPTY`](crate::agg_fn::AggregateRegistry::EMPTY) and
    /// [`PropertyFunctionRegistry::EMPTY`](crate::property_fn::PropertyFunctionRegistry::EMPTY).
    ///
    /// Reserved: [`fresh`](Self::fresh) starts its counter at `1` and only ever
    /// increments, so `0` is never minted by it and can never collide with a real,
    /// constructed registry's id.
    ///
    /// Sharing this one fixed id across every `EMPTY` constant, rather than each
    /// minting its own, is deliberate, not a shortcut: an empty registry's
    /// `resolve` always returns `None` for every IRI regardless of which `EMPTY`
    /// value asks, so no plan's admitted behavior can ever depend on WHICH empty
    /// registry it was prepared against — the two are observably interchangeable,
    /// and `registry_fingerprint`'s own `is_empty` short-circuit (in both
    /// `crate::agg_fn` and `crate::property_fn_plan`) already collapses every
    /// empty registry to the identical empty-string fingerprint, independent of
    /// its id, which predates this constant. Giving `EMPTY` a distinguishing id
    /// of its own would claim a distinction the rest of this crate does not
    /// honor anywhere.
    pub(crate) const EMPTY: Self = Self(0);

    /// This identity's fingerprint encoding — an explicit, `Display`-independent
    /// rendering (a bare decimal `u64`) folded into a registry's content
    /// fingerprint ahead of its declaration digest.
    pub(crate) fn stable_encoding(self) -> u64 {
        self.0
    }
}

impl Default for RegistryId {
    /// A fresh identity, so `#[derive(Default)]` on a registry type mints a new id
    /// exactly as an explicit `new()` does — every default-constructed registry
    /// gets its own identity rather than every one colliding on the same value.
    fn default() -> Self {
        Self::fresh()
    }
}

#[cfg(test)]
mod tests {
    use super::RegistryId;

    #[test]
    fn fresh_ids_never_collide() {
        let a = RegistryId::fresh();
        let b = RegistryId::fresh();
        assert_ne!(a, b, "two fresh ids must never collide");
        assert_ne!(a.stable_encoding(), b.stable_encoding());
    }

    #[test]
    fn default_mints_a_fresh_id_too() {
        let a = RegistryId::default();
        let b = RegistryId::default();
        assert_ne!(
            a, b,
            "Default must mint a fresh id, not a fixed sentinel value"
        );
    }
}
