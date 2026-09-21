// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Registry identity in two tiers: an unforgeable per-process identity for a
//! registry **instance**, and the framing every content-only fingerprint of a
//! registry's **declarations** is built from.
//!
//! [`crate::agg_fn::AggregateRegistry`] and
//! [`crate::property_fn::PropertyFunctionRegistry`] both fold [`RegistryId`] into
//! their `registry_fingerprint`, ahead of the declaration digest, because the
//! declaration digest alone cannot do the one job a plan's identity needs: two
//! registries built independently can register the SAME IRI to two DIFFERENT
//! trait-object implementations that happen to declare identical arity, volatility,
//! and every other observable metadata — indistinguishable by declaration, yet
//! computing different answers. A [`RegistryId`] gives every registry instance a
//! value no OTHER instance can ever share, so a plan prepared against one instance
//! refuses to run against any other, regardless of how similar the two describe
//! themselves.
//!
//! # The two tiers, and which one crosses a process boundary
//!
//! - The **instance** id ([`RegistryId`]) is a process-lifetime counter. It is
//!   strictly process-local and deliberately unpersistable: writing one down and
//!   reading it back in another process compares two counters that were never
//!   drawn from the same sequence, so the comparison carries no information at
//!   all. It answers exactly one question — "is this the same live registry
//!   object the plan in my hand was prepared against?" — and that question only
//!   exists within one process.
//!
//! - The **content** fingerprint (`content_fingerprint`, one per registry module:
//!   `crate::property_fn_plan`, `crate::agg_fn`, `crate::user_fn`) folds the same
//!   declared descriptor fields through [`append_framed_part`]'s injective framing
//!   with the instance id OMITTED, and digests the result. Being a pure function
//!   of declarations, it reproduces byte-for-byte in any process from any
//!   equivalently-declared registry — so it, not the instance id, is the identity
//!   a persisted artifact binds itself to when it must name the host registries it
//!   requires.
//!
//! The two tiers are not interchangeable and neither subsumes the other: the
//! instance id is strictly stronger in-process (it distinguishes implementations
//! that declare identically) and worthless across one; the content fingerprint is
//! the reverse. A caller crossing a process boundary therefore binds declarations,
//! and must pair that binding with whatever separately identifies the
//! implementations behind those declarations — for a registry built from a shapes
//! graph, the content digest of that graph.

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
/// # Why per-process monotonicity is sufficient — and what it is NOT sufficient for
///
/// A prepared plan is validated against a registry only within the SAME process
/// that prepared it. A [`crate::engine::PreparedQuery`] is itself an in-memory
/// object that cannot outlive the process that built it, so an identity that is
/// merely unique among every registry constructed during this process's lifetime
/// is exactly as strong a guarantee as the plan it protects ever needs.
///
/// This value must therefore never be serialized, persisted, or compared across a
/// process boundary. The counter restarts at `1` in every process, so two ids from
/// two processes are two readings of two unrelated sequences: they can collide
/// between registries that share nothing, and they differ between registries that
/// declare identically. Neither outcome means anything. An artifact that must name
/// the registries it requires across a process boundary binds their
/// `content_fingerprint` instead — see this module's own docs for the two tiers.
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
pub struct RegistryId(u64);

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

    /// The raw counter value behind this identity, so a value that records what
    /// it was planned against — a composition layer's serializable `Plan` — can
    /// carry it as plain data.
    ///
    /// The value is meaningful only while this process is alive; see the type's
    /// docs. A caller that persists it and reloads it later (or elsewhere) gets
    /// an identity that names no live registry, which is exactly why a
    /// deserialized plan is admitted against the durable content fingerprint
    /// rather than against this number.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Rebuild an identity from its encoded counter value — the decode half of
    /// [`Self::as_u64`], used when a serialized plan is read back.
    ///
    /// Reconstructing an identity does **not** make it name a live registry: the
    /// counter is per-process and monotonic, so a value decoded from a document
    /// can never collide with one this process mints. A decoded plan is matched
    /// against a registry by its content fingerprint, never by this value.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
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

/// Append one `label`/`value` pair to a content fingerprint's byte buffer under an
/// **injective** framing: each half is written as its length (big-endian `u64`)
/// followed by its bytes.
///
/// Every `content_fingerprint` in this crate builds its pre-digest bytes solely
/// through this function, so the encoding of a registry's declarations can never be
/// ambiguous: length-prefixing both halves means no combination of field values can
/// forge the byte sequence another combination produces. Without it, a delimiter- or
/// concatenation-based encoding lets two different registries digest identically the
/// moment a declared string contains the delimiter — a silent collision between two
/// registries that answer differently, which is precisely what a fingerprint exists
/// to make impossible.
///
/// This is the same framing `append_key_part` uses in `purrdf-shapes`' JSON-Schema
/// compilation key; it is duplicated rather than shared because `purrdf-sparql-eval`
/// does not depend on that crate (and must not: the dependency runs the other way).
///
/// It lives here, with [`RegistryId`], because this module owns registry identity:
/// the instance tier and the content tier are two encodings of the same question,
/// and keeping both spellings in one file is what stops them drifting into two
/// different answers.
pub(crate) fn append_framed_part(out: &mut Vec<u8>, label: &str, value: &[u8]) {
    out.extend_from_slice(&(label.len() as u64).to_be_bytes());
    out.extend_from_slice(label.as_bytes());
    out.extend_from_slice(&(value.len() as u64).to_be_bytes());
    out.extend_from_slice(value);
}

#[cfg(test)]
mod tests {
    use super::{RegistryId, append_framed_part};

    #[test]
    fn fresh_ids_never_collide() {
        let a = RegistryId::fresh();
        let b = RegistryId::fresh();
        assert_ne!(a, b, "two fresh ids must never collide");
        assert_ne!(a.stable_encoding(), b.stable_encoding());
    }

    /// The framing is injective across a field boundary: moving a byte from the
    /// end of one value to the start of the next must change the encoding, or two
    /// registries whose declarations differ only in where an IRI ends would digest
    /// identically.
    #[test]
    fn framing_is_injective_across_a_field_boundary() {
        let mut left = Vec::new();
        append_framed_part(&mut left, "iri", b"http://example.org/ns#ab");
        append_framed_part(&mut left, "iri", b"c");
        let mut right = Vec::new();
        append_framed_part(&mut right, "iri", b"http://example.org/ns#a");
        append_framed_part(&mut right, "iri", b"bc");
        assert_ne!(left, right);
    }

    /// An empty value is distinguishable from an absent field: the label is still
    /// framed, with a zero length.
    #[test]
    fn framing_records_an_empty_value() {
        let mut with_empty = Vec::new();
        append_framed_part(&mut with_empty, "datatype", b"");
        assert_ne!(with_empty, Vec::<u8>::new());
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
