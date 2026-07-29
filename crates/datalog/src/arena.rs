// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The phase-scoped row/tuple bump arena.
//!
//! # Two arenas, one contradiction resolved
//!
//! The evaluator needs two DIFFERENT arenas that a single structure cannot be:
//!
//! * a **persistent term arena** — never reset within its lifetime — backing the
//!   interned terms. That is the relation store's term interner: insertion-ordered,
//!   per-store, never truncated, because a [`TermId`] handed out
//!   in round 1 must still resolve in round 40. This module does NOT duplicate it.
//!
//! * a **phase-scoped row/tuple arena** — genuinely reset every round — where the
//!   semi-naive fixpoint bump-allocates a round's argument tuples, reads them back
//!   at the sorted commit, then TRUNCATES the backing buffer at the round boundary:
//!   allocate-within-round, sort-commit, reset. That is [`RowArena`] below.
//!
//! A single arena cannot be both persistent AND per-round-reset, so they are split.
//!
//! # Why a bump arena and not a `Vec<Vec<TermRef>>`
//!
//! An argument tuple is small and short-lived — one round. The arena keeps every
//! round's tuples in ONE contiguous [`TermRef`] buffer and hands out `(start, len)`
//! offset ranges, so a round's worth of tuples is one allocation that a single
//! [`RowArena::reset`] reclaims, with no per-tuple `Vec` alloc/free churn. A tuple
//! whose arity fits inline (at most [`INLINE`] arguments, the binary and ternary
//! common case) skips the buffer entirely by living in a fixed-size array carried
//! in the handle itself; only wider n-ary tuples spill into the contiguous backing
//! buffer. Reset is a real length-truncation of that real buffer, never a no-op.
//!
//! The inline array is a plain `[TermRef; INLINE]` plus a length, not a
//! small-vector type from a third-party crate: [`TermRef`] is `Copy` and the
//! spill-to-arena path already exists for the wide case, so the growable half of a
//! small-vector would be dead weight.
//!
//! # Determinism
//!
//! An arena is a positional structure, not an associative one: a tuple's arguments
//! read back in exactly the order they were allocated, and the same allocation
//! sequence always yields the same handles and the same backing buffer. No map,
//! clock or address participates. Emission order is never taken from arena offsets;
//! it comes from the resolved-lexical sort at round commit.
//!
//! # Thread-locality
//!
//! A `RowArena` is owned by the rule/phase invocation that creates it. Arenas are
//! never shared across rule tasks, and a completed task's buffer crosses a
//! scheduling boundary only as an owned value, before the single sorted commit.
//! There is no cross-thread arena aliasing to guard.

use core::fmt;

use crate::id::{TermId, TermRef};

/// The inline argument-tuple arity: a tuple of at most this many [`TermRef`]s stays
/// in the handle (the binary evaluator's arity-2 rows, plus headroom for the
/// ternary world-slotted and small n-ary shapes) and never touches the arena's
/// backing buffer.
pub const INLINE: usize = 4;

/// The filler occupying the unused tail slots of an [`InlineTuple`].
///
/// [`TermRef`] has no `Default`, so the fixed-size inline array must be initialised
/// with *some* value. Slots at or past the tuple's length are never observable —
/// [`InlineTuple::as_slice`] cuts the array at the length, and equality, hashing
/// and `Debug` all go through that slice — so the filler's value carries no meaning
/// and is not part of a tuple's identity.
#[inline]
fn pad() -> TermRef {
    TermRef::term(TermId::from_index(0))
}

/// A row tuple whose arity fits inline, carried in the handle itself.
///
/// The argument array is private: only the first `len` slots are meaningful and the
/// rest hold an unobservable filler. Read the arguments through
/// [`as_slice`](Self::as_slice).
#[derive(Clone, Copy)]
pub struct InlineTuple {
    /// Argument slots. Only `args[..len]` is meaningful; the tail holds [`pad`].
    args: [TermRef; INLINE],
    /// The tuple's arity, at most [`INLINE`].
    len: u8,
}

impl InlineTuple {
    /// The tuple's arguments, in allocation order.
    #[inline]
    pub fn as_slice(&self) -> &[TermRef] {
        &self.args[..self.len as usize]
    }

    /// The tuple's arity.
    #[inline]
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Whether the tuple has no arguments (arity 0).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl fmt::Debug for InlineTuple {
    /// Prints only the meaningful prefix, so the filler never appears in a
    /// diagnostic and two equal tuples always render identically.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}

impl PartialEq for InlineTuple {
    /// Compares the meaningful prefix only; the filler tail is not part of the
    /// tuple's identity.
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Eq for InlineTuple {}

/// A handle to one argument tuple allocated in a [`RowArena`] round.
///
/// Either the tuple's [`TermRef`]s inline (arity at most [`INLINE`]) or a
/// `(start, len)` offset range into the arena's contiguous backing buffer (wider
/// n-ary tuples). A handle is only valid against the arena that produced it, and
/// only until that arena is [`reset`](RowArena::reset).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowTuple {
    /// The tuple's arguments inline — no backing-buffer slot used.
    Inline(InlineTuple),
    /// A `[start, start + len)` range into the arena's contiguous backing buffer.
    Arena {
        /// The first backing-buffer slot of the tuple.
        start: u32,
        /// The tuple's arity, in backing-buffer slots.
        len: u32,
    },
}

/// A phase-scoped bump arena for a round's argument tuples.
///
/// Allocate every tuple of a round with [`alloc`](Self::alloc), read them back
/// through [`get`](Self::get) at the sorted commit, then [`reset`](Self::reset) at
/// the round boundary. The backing buffer is a single contiguous [`Vec`] truncated
/// to length 0 on reset — a genuine buffer reclaim, not an inline-storage no-op.
#[derive(Debug, Default)]
pub struct RowArena {
    /// Contiguous backing buffer for the tuples that overflow the inline arity.
    backing: Vec<TermRef>,
}

impl RowArena {
    /// A fresh, empty arena.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bump-allocate `args` as one tuple, returning its handle.
    ///
    /// A tuple whose arity fits inline stays in the handle; a wider tuple is
    /// appended to the contiguous backing buffer as a `(start, len)` range.
    ///
    /// # Panics
    ///
    /// Panics if the backing buffer would exceed `u32::MAX` slots, or if a single
    /// tuple's arity exceeds `u32::MAX`. Both are programming errors — a round that
    /// large has already blown the fact budget — never a reachable data state.
    pub fn alloc(&mut self, args: &[TermRef]) -> RowTuple {
        if args.len() <= INLINE {
            let mut slots = [pad(); INLINE];
            slots[..args.len()].copy_from_slice(args);
            RowTuple::Inline(InlineTuple {
                args: slots,
                len: args.len() as u8,
            })
        } else {
            let start = u32::try_from(self.backing.len())
                .expect("RowArena backing overflow: more than u32::MAX TermRefs in one round");
            let len =
                u32::try_from(args.len()).expect("RowArena tuple overflow: arity exceeds u32::MAX");
            self.backing.extend_from_slice(args);
            RowTuple::Arena { start, len }
        }
    }

    /// The argument slice a handle addresses, in allocation order.
    ///
    /// # Panics
    ///
    /// Panics if `tuple` is a [`RowTuple::Arena`] range that falls outside the
    /// current backing buffer — i.e. a handle from a prior, already-[`reset`](Self::reset)
    /// round. That is a programming error, never a data state.
    pub fn get<'a>(&'a self, tuple: &'a RowTuple) -> &'a [TermRef] {
        match tuple {
            RowTuple::Inline(inline) => inline.as_slice(),
            RowTuple::Arena { start, len } => {
                let start = *start as usize;
                let end = start + *len as usize;
                self.backing.get(start..end).unwrap_or_else(|| {
                    panic!(
                        "RowArena handle [{start}, {end}) is out of bounds (backing len {}): \
                         a tuple handle must never outlive its round's reset",
                        self.backing.len()
                    )
                })
            }
        }
    }

    /// Truncate the backing buffer to length 0 — the round/stratum-boundary reset.
    ///
    /// A real length-truncation of the contiguous buffer to zero (`Vec::clear`,
    /// which retains the buffer's capacity for the next round), NOT a no-op on
    /// inline storage: every [`RowTuple::Arena`] handle minted before the reset is
    /// invalidated, matching the fixpoint's allocate, commit, reset phases.
    pub fn reset(&mut self) {
        self.backing.clear();
    }

    /// The number of [`TermRef`]s currently held in the backing buffer.
    ///
    /// A cost probe: inline tuples are not counted here, because they never touch
    /// the buffer.
    pub fn backing_len(&self) -> usize {
        self.backing.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tref(slot: usize) -> TermRef {
        TermRef::term(TermId::from_index(slot))
    }

    /// A binary/ternary tuple stays inline and never touches the backing buffer.
    #[test]
    fn arena_small_tuple_is_inline_and_leaves_buffer_empty() {
        let mut arena = RowArena::new();
        let binary = arena.alloc(&[tref(0), tref(1)]);
        assert!(
            matches!(binary, RowTuple::Inline(_)),
            "arity 2 must be inline"
        );
        assert_eq!(arena.get(&binary), &[tref(0), tref(1)]);
        // A full-inline-capacity tuple (arity == INLINE) still stays inline.
        let quad = arena.alloc(&[tref(2), tref(3), tref(4), tref(5)]);
        assert!(
            matches!(quad, RowTuple::Inline(_)),
            "arity == INLINE must be inline"
        );
        assert_eq!(arena.get(&quad), &[tref(2), tref(3), tref(4), tref(5)]);
        assert_eq!(
            arena.backing_len(),
            0,
            "inline tuples must never grow the backing buffer"
        );
    }

    /// An arity-0 tuple is inline, empty, and does not touch the buffer — the
    /// padding filler must not leak into the read-back slice.
    #[test]
    fn arena_zero_arity_tuple_reads_back_empty() {
        let mut arena = RowArena::new();
        let nullary = arena.alloc(&[]);
        assert_eq!(arena.get(&nullary), &[] as &[TermRef]);
        match nullary {
            RowTuple::Inline(inline) => {
                assert!(inline.is_empty());
                assert_eq!(inline.len(), 0);
                assert_eq!(format!("{inline:?}"), "[]", "padding must not be printed");
            }
            RowTuple::Arena { .. } => panic!("arity 0 must be inline"),
        }
        assert_eq!(arena.backing_len(), 0);
    }

    /// A wider-than-inline n-ary tuple spills into the contiguous backing buffer,
    /// and `reset` genuinely truncates that real buffer (not a no-op).
    #[test]
    fn arena_wide_tuple_spills_and_reset_truncates_real_buffer() {
        let mut arena = RowArena::new();
        // Arity 5 > INLINE(4): must spill into the backing buffer as a range.
        let args: Vec<TermRef> = (0..5).map(tref).collect();
        let wide = arena.alloc(&args);
        match wide {
            RowTuple::Arena { start, len } => {
                assert_eq!((start, len), (0, 5), "first spill occupies [0, 5)");
            }
            RowTuple::Inline(_) => panic!("arity 5 must spill into the arena buffer"),
        }
        assert_eq!(arena.backing_len(), 5, "the buffer holds the spilled tuple");
        assert_eq!(arena.get(&wide), args.as_slice());

        // A second spill appends after the first.
        let more: Vec<TermRef> = (10..16).map(tref).collect();
        let wide2 = arena.alloc(&more);
        assert!(matches!(wide2, RowTuple::Arena { start: 5, len: 6 }));
        assert_eq!(arena.backing_len(), 11);
        assert_eq!(arena.get(&wide2), more.as_slice());

        // Reset is a REAL truncation of the contiguous buffer.
        arena.reset();
        assert_eq!(
            arena.backing_len(),
            0,
            "reset must truncate the real buffer to 0"
        );

        // After reset the buffer is reusable and offsets restart at 0.
        let reused = arena.alloc(&args);
        assert!(matches!(reused, RowTuple::Arena { start: 0, len: 5 }));
        assert_eq!(arena.get(&reused), args.as_slice());
    }

    /// A handle from an already-reset round is a programming error, and `get`
    /// reports it as a panic rather than reading a stale or truncated slice.
    #[test]
    #[should_panic(expected = "must never outlive its round's reset")]
    fn arena_stale_handle_panics_after_reset() {
        let mut arena = RowArena::new();
        let args: Vec<TermRef> = (0..6).map(tref).collect();
        let stale = arena.alloc(&args);
        arena.reset();
        let _ = arena.get(&stale);
    }

    /// Determinism: within a tuple, argument order is positional and preserved
    /// exactly, and replaying the same allocation sequence reproduces the same
    /// handles and the same backing buffer bit for bit. Two arenas fed the same
    /// sequence are indistinguishable; a permuted sequence changes only which
    /// offsets belong to which tuple, never a tuple's own contents.
    #[test]
    fn arena_allocation_is_positional_and_replayable() {
        let tuples: Vec<Vec<TermRef>> = vec![
            (0..2).map(tref).collect(),
            (10..17).map(tref).collect(),
            (20..24).map(tref).collect(),
            (30..36).map(tref).collect(),
        ];

        let replay = |order: &[usize]| {
            let mut arena = RowArena::new();
            let handles: Vec<RowTuple> = order.iter().map(|&i| arena.alloc(&tuples[i])).collect();
            let read: Vec<Vec<TermRef>> = handles
                .iter()
                .map(|h| arena.get(h).to_vec())
                .collect::<Vec<_>>();
            (handles, read, arena.backing_len())
        };

        let identity: Vec<usize> = (0..tuples.len()).collect();
        let (handles_a, read_a, backing_a) = replay(&identity);
        let (handles_b, read_b, backing_b) = replay(&identity);
        assert_eq!(handles_a, handles_b, "same sequence, same handles");
        assert_eq!(read_a, read_b, "same sequence, same read-back");
        assert_eq!(backing_a, backing_b, "same sequence, same buffer length");

        // Whatever the allocation order, each tuple reads back its own arguments in
        // its own order, and the total spilled volume is order-independent.
        for seed in 0..24u64 {
            let order = crate::test_support::permute(&identity, seed);
            let (_, read, backing) = replay(&order);
            assert_eq!(
                backing, backing_a,
                "seed {seed}: spilled volume is order-independent"
            );
            for (slot, &idx) in order.iter().enumerate() {
                assert_eq!(
                    read[slot], tuples[idx],
                    "seed {seed}: tuple {idx} reads back its own arguments in order"
                );
            }
        }
    }
}
