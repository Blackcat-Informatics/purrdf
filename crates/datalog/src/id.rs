// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Branded niche IDs for the evaluator's entity classes.
//!
//! # Doctrine
//!
//! Every dense handle in the Datalog evaluator — an interned term, a predicate, a
//! rule, a materialised row — is an [`Id<C>`]: a `NonZeroU64` wearing a
//! `PhantomData` brand. The brand `C` makes IDs of different classes DISTINCT
//! TYPES, so a [`TermId`] can never be passed where a [`PartitionId`] is expected;
//! cross-class ID confusion is a compile error, not a runtime bug. The
//! `NonZeroU64` niche keeps `Option<Id<C>>` the same width as `Id<C>` itself, so a
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
use core::num::NonZeroU64;
use purrdf_lex::allocation::{CapacityError, CapacitySpace, StorageError};

/// A dense, per-space handle for entity class `C`.
///
/// Stored as a `NonZeroU64` (the niche makes `Option<Id<C>>` the same width as
/// `Id<C>`) branded by `PhantomData<fn() -> C>`. The `fn() -> C` form is covariant
/// in `C` and imposes no auto-trait bound on `C`, so `Id<C>: Copy + Send + Sync`
/// regardless of the brand — and the brand type never needs to be constructible,
/// which is why the markers below are uninhabited.
pub struct Id<C>(NonZeroU64, PhantomData<fn() -> C>);

impl<C> Id<C> {
    /// The zero-based slot index this id addresses in its space.
    ///
    /// The niche offset is `+1`: slot `0` is stored as `NonZeroU64(1)`.
    #[inline]
    pub fn index(self) -> u64 {
        self.0.get() - 1
    }

    /// Checked physical address, separate from this handle's logical width.
    ///
    /// # Errors
    /// Returns layout refusal when this target cannot address the logical slot.
    pub fn address(self) -> Result<usize, StorageError> {
        usize::try_from(self.index()).map_err(|_| StorageError::SizeOverflow)
    }

    /// Address of a handle already minted by an actual resident arena. Foreign
    /// handles remain a programming error, distinct from a data-capacity refusal.
    pub(crate) fn resident_index(self) -> usize {
        self.address()
            .expect("a resident arena minted this physical address")
    }

    /// Mint a logical slot independently of the target pointer width.
    ///
    /// # Errors
    /// Returns this brand's exact requested count and actual u64 niche limit.
    #[inline]
    pub fn from_index(index: u64) -> Result<Self, CapacityError>
    where
        C: IdClass,
    {
        Self::from_index_with_limit(index, u64::MAX)
    }

    /// The one logical mint, also used with an owner-local reduced test limit.
    fn from_index_with_limit(index: u64, limit: u64) -> Result<Self, CapacityError>
    where
        C: IdClass,
    {
        CapacityError::slot(C::SPACE, index, limit)?;
        Ok(Self(
            NonZeroU64::new(index + 1).expect("checked nonzero id representation"),
            PhantomData,
        ))
    }

    /// Mint the next logical handle from an actual resident arena length.
    ///
    /// # Errors
    /// Returns physical address overflow or this brand's exact capacity refusal.
    pub(crate) fn from_address(index: usize) -> Result<Self, StorageError>
    where
        C: IdClass,
    {
        Self::from_address_with_limit(index, u64::MAX)
    }

    /// Convert the real resident length and apply the same native mint check.
    pub(crate) fn from_address_with_limit(index: usize, limit: u64) -> Result<Self, StorageError>
    where
        C: IdClass,
    {
        let index = u64::try_from(index).map_err(|_| StorageError::SizeOverflow)?;
        Ok(Self::from_index_with_limit(index, limit)?)
    }

    /// The slot this id addresses in `slots`, the arena that minted it.
    ///
    /// Every per-arena id space ([`NodeId`], [`SymId`], [`ProofId`], …) resolves through
    /// this one lookup, so a foreign id fails with the same diagnosis everywhere. `kind`
    /// names the id type (`"NodeId"`) and `noun` the ids in prose (`"term"`).
    ///
    /// # Panics
    ///
    /// Panics if the id was not minted by that arena: ids are per-arena handles, so a
    /// foreign id is a programming error, never a data state.
    #[track_caller]
    pub(crate) fn slot_in<'a, T>(self, slots: &'a [T], kind: &str, noun: &str) -> &'a T {
        slots.get(self.resident_index()).unwrap_or_else(|| {
            panic!(
                "{kind} {self:?} was not minted by this arena (len {}): {noun} ids are \
                 per-arena handles and must never cross arena boundaries",
                slots.len()
            )
        })
    }
}

// Manual trait impls: deriving would place spurious `C: Trait` bounds on the brand,
// which is uninhabited and can never satisfy them. The `fn() -> C` brand carries no
// data, so every impl is over the `NonZeroU64` payload alone.

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

/// The native logical space belonging to a dense Datalog brand.
pub trait IdClass {
    /// The producer's closed capacity identity.
    const SPACE: CapacitySpace;
}

// ── Brand markers (uninhabited: pure type-level tags, never constructed) ─────────
//
// Each brand wraps `Infallible` in a private field rather than being an empty
// `enum`. Both spellings are uninhabited, but the wrapper is the form the
// compiler's `empty_enums` lint asks for, and it keeps the intent legible: the
// private `Infallible` field is a value that cannot exist, so the brand cannot be
// constructed here or by any consumer. The `!` type the lint mentions first is
// still unstable, and this workspace uses no nightly-only features, so the
// wrapper is the only spelling available at the declared MSRV.

use core::convert::Infallible;

/// Brand: an interned atomic-term handle. See [`TermId`].
#[derive(Debug)]
pub struct Term(Infallible);
/// Brand: a relation-partition handle. See [`PartitionId`].
#[derive(Debug)]
pub struct Partition(Infallible);
/// Brand: a rule handle. See [`RuleId`].
#[derive(Debug)]
pub struct Rule(Infallible);
/// Brand: a materialised-row handle. See [`RowId`].
#[derive(Debug)]
pub struct Row(Infallible);
/// Brand: a hash-consed proof-term handle. See [`ProofId`].
#[derive(Debug)]
pub struct Proof(Infallible);
/// Brand: a hash-consed compound-term arena node handle. See [`NodeId`].
#[derive(Debug)]
pub struct Node(Infallible);
/// Brand: a unification metavariable handle. See [`MetaId`].
#[derive(Debug)]
pub struct Meta(Infallible);
/// Brand: an interned atomic-symbol handle (a compound term's leaf/free lexical
/// surface), distinct from `TermId`'s own atomic-fact-term interner. See [`SymId`].
#[derive(Debug)]
pub struct Sym(Infallible);

impl IdClass for Term {
    const SPACE: CapacitySpace = CapacitySpace::DatalogTerms;
}
impl IdClass for Partition {
    const SPACE: CapacitySpace = CapacitySpace::DatalogPartitions;
}
impl IdClass for Rule {
    const SPACE: CapacitySpace = CapacitySpace::DatalogRules;
}
impl IdClass for Row {
    const SPACE: CapacitySpace = CapacitySpace::DatalogRows;
}
impl IdClass for Proof {
    const SPACE: CapacitySpace = CapacitySpace::DatalogProofs;
}
impl IdClass for Node {
    const SPACE: CapacitySpace = CapacitySpace::DatalogNodes;
}
impl IdClass for Meta {
    const SPACE: CapacitySpace = CapacitySpace::DatalogMetas;
}
impl IdClass for Sym {
    const SPACE: CapacitySpace = CapacitySpace::DatalogSymbols;
}

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
/// A dense per-arena hash-consed compound-term node handle, minted by
/// [`crate::term::TermDag`] in interning order.
pub type NodeId = Id<Node>;
/// A dense per-arena unification metavariable handle, minted by
/// [`crate::term::TermDag::fresh_meta`].
pub type MetaId = Id<Meta>;
/// A dense per-arena interned atomic-symbol handle — the lexical surface a
/// [`NodeId`]'s [`Leaf`](crate::term::NodeData::Leaf) or
/// [`Free`](crate::term::NodeData::Free) variant carries, kept in its own interner
/// distinct from [`TermId`] because the two arenas serve different consumers and
/// have no reason to share an index space.
pub type SymId = Id<Sym>;

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

    #[test]
    fn every_native_brand_preserves_exact_capacity_and_wide_logical_identity() {
        fn check<C: IdClass>() {
            let wide = Id::<C>::from_index(u64::from(u32::MAX) + 1).unwrap();
            assert_eq!(wide.index(), u64::from(u32::MAX) + 1);
            assert_eq!(
                wide.address(),
                usize::try_from(wide.index()).map_err(|_| StorageError::SizeOverflow)
            );
            assert_eq!(
                Id::<C>::from_index(u64::MAX).unwrap_err(),
                CapacityError::new(C::SPACE, u128::from(u64::MAX) + 1, u128::from(u64::MAX))
            );
            assert_eq!(
                Id::<C>::from_index(u64::MAX - 1).unwrap().index(),
                u64::MAX - 1
            );
        }
        check::<Term>();
        check::<Partition>();
        check::<Rule>();
        check::<Row>();
        check::<Proof>();
        check::<Node>();
        check::<Meta>();
        check::<Sym>();
    }

    /// An id resolves to its slot in the arena that minted it, up to the last slot.
    #[test]
    fn slot_in_resolves_every_minted_slot() {
        let slots = ["a", "b", "c"];
        assert_eq!(
            *NodeId::from_index(0)
                .unwrap()
                .slot_in(&slots, "NodeId", "term"),
            "a"
        );
        assert_eq!(
            *NodeId::from_index(2)
                .unwrap()
                .slot_in(&slots, "NodeId", "term"),
            "c"
        );
    }

    /// One past the last slot is a foreign id, refused with the id and the arena length.
    #[test]
    #[should_panic(
        expected = "SymId Id(3) was not minted by this arena (len 3): symbol ids are per-arena handles"
    )]
    fn slot_in_refuses_an_id_past_the_arena() {
        let slots = ["a", "b", "c"];
        let _ = SymId::from_index(3)
            .unwrap()
            .slot_in(&slots, "SymId", "symbol");
    }

    /// `index()`/`from_index()` round-trip the 0-based slot to the 1-based niche at
    /// the boundary values — the `+1` niche offset must be exact everywhere, for
    /// every brand.
    #[test]
    fn id_niche_offset_round_trips_at_boundaries() {
        for slot in [
            0u64,
            1,
            u64::from(u32::MAX) - 1,
            u64::from(u32::MAX),
            u64::MAX - 1,
        ] {
            assert_eq!(
                TermId::from_index(slot).unwrap().index(),
                slot,
                "slot {slot} term"
            );
            assert_eq!(
                PartitionId::from_index(slot).unwrap().index(),
                slot,
                "slot {slot} partition"
            );
            assert_eq!(
                RuleId::from_index(slot).unwrap().index(),
                slot,
                "slot {slot} rule"
            );
            assert_eq!(
                RowId::from_index(slot).unwrap().index(),
                slot,
                "slot {slot} row"
            );
            assert_eq!(
                ProofId::from_index(slot).unwrap().index(),
                slot,
                "slot {slot} proof"
            );
            assert_eq!(
                NodeId::from_index(slot).unwrap().index(),
                slot,
                "slot {slot} node"
            );
            assert_eq!(
                MetaId::from_index(slot).unwrap().index(),
                slot,
                "slot {slot} meta"
            );
            assert_eq!(
                SymId::from_index(slot).unwrap().index(),
                slot,
                "slot {slot} sym"
            );
        }
        // Slot 0 is stored as NonZeroU64(1) — the niche is genuinely used.
        assert_eq!(PartitionId::from_index(0).unwrap().index(), 0);
    }

    /// The `NonZeroU64` niche makes `Option<Id<C>>` no wider than `Id<C>` (no
    /// discriminant word), for EVERY brand.
    #[test]
    fn id_option_is_niche_packed() {
        assert_eq!(
            size_of::<Option<TermId>>(),
            size_of::<TermId>(),
            "Option<TermId> must be niche-packed to TermId's width"
        );
        assert_eq!(size_of::<TermId>(), size_of::<u64>());
        assert_eq!(size_of::<Option<PartitionId>>(), size_of::<PartitionId>());
        assert_eq!(size_of::<Option<RowId>>(), size_of::<RowId>());
        assert_eq!(size_of::<Option<RuleId>>(), size_of::<RuleId>());
        assert_eq!(size_of::<Option<ProofId>>(), size_of::<ProofId>());
        assert_eq!(size_of::<Option<NodeId>>(), size_of::<NodeId>());
        assert_eq!(size_of::<Option<MetaId>>(), size_of::<MetaId>());
        assert_eq!(size_of::<Option<SymId>>(), size_of::<SymId>());
        // A TermRef is exactly its wrapped TermId — the row-tuple argument handle
        // adds no width over the atomic handle it carries, and inherits the niche.
        assert_eq!(size_of::<TermRef>(), size_of::<TermId>());
        assert_eq!(size_of::<Option<TermRef>>(), size_of::<TermRef>());
    }

    /// `Ord` is by raw index (mint order) — earlier-minted sorts first.
    #[test]
    fn id_ord_is_mint_order() {
        let a = PartitionId::from_index(0).unwrap();
        let b = PartitionId::from_index(1).unwrap();
        assert!(a < b, "mint order: slot 0 precedes slot 1");
        assert_eq!(a, PartitionId::from_index(0).unwrap());
    }

    /// Determinism: an id's identity and order depend on its slot alone, never on
    /// the sequence in which the ids were constructed. Sorting any permutation of
    /// the same slot set yields the same ascending sequence.
    #[test]
    fn id_order_is_independent_of_construction_order() {
        let slots: Vec<u64> = (0..64).collect();
        let ascending: Vec<RowId> = slots
            .iter()
            .copied()
            .map(|index| RowId::from_index(index).unwrap())
            .collect();
        for seed in 0..16u64 {
            let permuted = crate::test_support::permute(&slots, seed);
            let mut built: Vec<RowId> = permuted
                .into_iter()
                .map(|index| RowId::from_index(index).unwrap())
                .collect();
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
        assert_eq!(
            format!("{:?}", PartitionId::from_index(7).unwrap()),
            "Id(7)"
        );
        assert_eq!(
            format!("{:?}", TermRef::term(TermId::from_index(7).unwrap())),
            "TermRef(7)"
        );
    }
}
