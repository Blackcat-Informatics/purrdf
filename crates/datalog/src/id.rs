// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Branded niche IDs for the evaluator's entity classes.
//!
//! # Doctrine
//!
//! Every dense handle in the Datalog evaluator — an interned term, a predicate, a
//! rule, a materialised row — is an [`Id<C>`]: a `NonZeroU32` wearing a
//! `PhantomData` brand. The brand `C` makes IDs of different classes DISTINCT
//! TYPES, so a [`TermId`] can never be passed where a [`PartitionId`] is expected;
//! cross-class ID confusion is a compile error, not a runtime bug. The
//! `NonZeroU32` niche keeps `Option<Id<C>>` the same width as `Id<C>` itself, so a
//! vacant term/row slot costs no extra word.
//!
//! # Ordering (read this before sorting on an `Id`)
//!
//! [`Id`]'s [`Ord`] is by RAW INDEX — that is, MINT ORDER: insertion order within
//! the space that minted it. Mint order is **meaningless for emission**. Two runs
//! that intern the same terms in the same sequence mint the same ids, but the
//! integers themselves carry no lexical meaning, so they may only be used where the
//! code is already operating on mint order: dense `Vec` indexing, and the sorted
//! row buckets an intersection cursor gallops over. Every emission, commit and
//! budget-charge ordering derives from the resolved lexical surface at the sorted
//! round commit — NEVER from `Id` order.
//!
//! An `Id` integer is a runtime handle only. It is never serialized and never
//! hashed into a stable identity: the RDF term surface is the persistent identity,
//! and the id is just this run's dense address for it.
//!
//! # Determinism
//!
//! Mint order is a function of the input sequence alone. No map iteration, clock,
//! address or RNG participates in minting, so identical input mints identical ids
//! on every target.

use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::num::NonZeroU32;

/// A dense, per-space handle for entity class `C`.
///
/// Stored as a `NonZeroU32` (the niche makes `Option<Id<C>>` the same width as
/// `Id<C>`) branded by `PhantomData<fn() -> C>`. The `fn() -> C` form is covariant
/// in `C` and imposes no auto-trait bound on `C`, so `Id<C>: Copy + Send + Sync`
/// regardless of the brand — and the brand type never needs to be constructible,
/// which is why the markers below are uninhabited.
pub struct Id<C>(NonZeroU32, PhantomData<fn() -> C>);

impl<C> Id<C> {
    /// The zero-based slot index this id addresses in its space.
    ///
    /// The niche offset is `+1`: slot `0` is stored as `NonZeroU32(1)`.
    #[inline]
    pub fn index(self) -> usize {
        (self.0.get() - 1) as usize
    }

    /// Mint the id for zero-based slot `index`.
    ///
    /// The niche offset is `+1`, so slot `0` becomes `NonZeroU32(1)` and
    /// `Option<Id<C>>` stays the width of `Id<C>`.
    ///
    /// # Panics
    ///
    /// Panics if `index + 1` exceeds `u32::MAX` — an id space overflow, which is a
    /// programming error (more than `u32::MAX - 1` distinct entities in one space),
    /// never a reachable data state.
    #[inline]
    pub fn from_index(index: usize) -> Self {
        let raw = u32::try_from(index + 1)
            .expect("Id space overflow: more than u32::MAX - 1 distinct entities in one space");
        Self(
            NonZeroU32::new(raw).expect("index + 1 is nonzero by construction"),
            PhantomData,
        )
    }
}

// Manual trait impls: deriving would place spurious `C: Trait` bounds on the brand,
// which is uninhabited and can never satisfy them. The `fn() -> C` brand carries no
// data, so every impl is over the `NonZeroU32` payload alone.

impl<C> Clone for Id<C> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<C> Copy for Id<C> {}

impl<C> PartialEq for Id<C> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<C> Eq for Id<C> {}

impl<C> Hash for Id<C> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<C> Ord for Id<C> {
    /// By raw index, i.e. mint order. See the module doctrine: mint order is NEVER
    /// an emission-order source.
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<C> PartialOrd for Id<C> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<C> fmt::Debug for Id<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Print the 0-based slot index; the brand type is elided (it is never a
        // value, only a phantom).
        write!(f, "Id({})", self.index())
    }
}

// ── Brand markers (uninhabited: pure type-level tags, never constructed) ─────────

/// Brand: an interned atomic-term handle. See [`TermId`].
#[derive(Debug)]
pub enum Term {}
/// Brand: a relation-partition handle. See [`PartitionId`].
#[derive(Debug)]
pub enum Partition {}
/// Brand: a rule handle. See [`RuleId`].
#[derive(Debug)]
pub enum Rule {}
/// Brand: a materialised-row handle. See [`RowId`].
#[derive(Debug)]
pub enum Row {}
/// Brand: a hash-consed proof-term handle. See [`ProofId`].
#[derive(Debug)]
pub enum Proof {}

/// A dense per-interner atomic-term handle.
pub type TermId = Id<Term>;
/// A dense per-store relation-partition handle.
///
/// The store holds ONE arity-4 relation `triple(subject, predicate, object, graph)`,
/// physically partitioned by its `(predicate, graph)` positions; a `PartitionId` addresses
/// one such partition. It is a distinct brand from [`TermId`] because a partition slot and
/// an interned term are different index spaces, even though a partition's key is a pair of
/// term ids.
pub type PartitionId = Id<Partition>;
/// A dense per-program rule handle.
pub type RuleId = Id<Rule>;
/// A dense per-stratum materialised-row handle.
pub type RowId = Id<Row>;
/// A dense per-arena hash-consed proof-term handle.
///
/// Minted by [`crate::proof::ProofArena`] in interning order, and — like every other `Id` —
/// a runtime handle only. A proof's SERIALIZED form carries no `ProofId`: it numbers nodes
/// by their position in a canonical emission walk, so two arenas that built the same proof
/// through different sequences encode to the same bytes.
pub type ProofId = Id<Proof>;

/// The argument handle every arena'd row tuple uses.
///
/// It is always an atomic interned [`TermId`] — a plain newtype, not an enum. The
/// newtype is nominal on purpose: it marks "this term handle occupies an argument
/// position in a materialised row tuple" distinctly from an interner-facing
/// [`TermId`], so a row-tuple signature and an interner signature cannot be
/// silently interchanged. It is exactly as wide as the [`TermId`] it wraps, so the
/// distinction costs nothing.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TermRef(TermId);

impl TermRef {
    /// Wrap an atomic interned term as a row-tuple argument handle.
    #[inline]
    pub const fn term(id: TermId) -> Self {
        Self(id)
    }

    /// The interned [`TermId`] this handle addresses.
    #[inline]
    pub const fn id(self) -> TermId {
        self.0
    }
}

impl fmt::Debug for TermRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TermRef({})", self.0.index())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `index()`/`from_index()` round-trip the 0-based slot to the 1-based niche at
    /// the boundary values — the `+1` niche offset must be exact everywhere, for
    /// every brand.
    #[test]
    fn id_niche_offset_round_trips_at_boundaries() {
        for slot in [0usize, 1, (u32::MAX - 2) as usize] {
            assert_eq!(TermId::from_index(slot).index(), slot, "slot {slot} term");
            assert_eq!(
                PartitionId::from_index(slot).index(),
                slot,
                "slot {slot} partition"
            );
            assert_eq!(RuleId::from_index(slot).index(), slot, "slot {slot} rule");
            assert_eq!(RowId::from_index(slot).index(), slot, "slot {slot} row");
            assert_eq!(ProofId::from_index(slot).index(), slot, "slot {slot} proof");
        }
        // Slot 0 is stored as NonZeroU32(1) — the niche is genuinely used.
        assert_eq!(PartitionId::from_index(0).index(), 0);
    }

    /// The `NonZeroU32` niche makes `Option<Id<C>>` no wider than `Id<C>` (no
    /// discriminant word), for EVERY brand.
    #[test]
    fn id_option_is_niche_packed() {
        assert_eq!(
            size_of::<Option<TermId>>(),
            size_of::<TermId>(),
            "Option<TermId> must be niche-packed to TermId's width"
        );
        assert_eq!(size_of::<TermId>(), size_of::<u32>());
        assert_eq!(size_of::<Option<PartitionId>>(), size_of::<PartitionId>());
        assert_eq!(size_of::<Option<RowId>>(), size_of::<RowId>());
        assert_eq!(size_of::<Option<RuleId>>(), size_of::<RuleId>());
        assert_eq!(size_of::<Option<ProofId>>(), size_of::<ProofId>());
        // A TermRef is exactly its wrapped TermId — the row-tuple argument handle
        // adds no width over the atomic handle it carries, and inherits the niche.
        assert_eq!(size_of::<TermRef>(), size_of::<TermId>());
        assert_eq!(size_of::<Option<TermRef>>(), size_of::<TermRef>());
    }

    /// `Ord` is by raw index (mint order) — earlier-minted sorts first.
    #[test]
    fn id_ord_is_mint_order() {
        let a = PartitionId::from_index(0);
        let b = PartitionId::from_index(1);
        assert!(a < b, "mint order: slot 0 precedes slot 1");
        assert_eq!(a, PartitionId::from_index(0));
    }

    /// Determinism: an id's identity and order depend on its slot alone, never on
    /// the sequence in which the ids were constructed. Sorting any permutation of
    /// the same slot set yields the same ascending sequence.
    #[test]
    fn id_order_is_independent_of_construction_order() {
        let slots: Vec<usize> = (0..64).collect();
        let ascending: Vec<RowId> = slots.iter().copied().map(RowId::from_index).collect();
        for seed in 0..16u64 {
            let permuted = crate::test_support::permute(&slots, seed);
            let mut built: Vec<RowId> = permuted.into_iter().map(RowId::from_index).collect();
            built.sort_unstable();
            assert_eq!(
                built, ascending,
                "seed {seed}: sorted ids ignore mint order"
            );
        }
    }

    /// The `Debug` surface prints the 0-based slot, not the stored niche integer,
    /// and a `TermRef` prints the term it wraps.
    #[test]
    fn debug_prints_zero_based_slots() {
        assert_eq!(format!("{:?}", PartitionId::from_index(7)), "Id(7)");
        assert_eq!(
            format!("{:?}", TermRef::term(TermId::from_index(7))),
            "TermRef(7)"
        );
    }
}
