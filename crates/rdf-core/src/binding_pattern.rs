// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The arity-generic **binding pattern** — the adornment lattice shared by every
//! consumer that must decide, for one atom or triple pattern, which argument
//! positions are already bound before it runs: backward magic-sets demand
//! keying, a forward evaluator's query-plan index selection, and a SPARQL
//! property-function's access-pattern feasibility check alike.
//!
//! A [`BindingPattern`] is a bitset over an atom's argument positions: position `i`
//! set means that position is **bound**, i.e. a constant, or a variable already
//! bound by the sideways-information-passing chain or by the goal. Being
//! arity-generic it covers RDF's arity-2 and arity-3 shapes and the wider n-ary
//! relations a rule table can declare with one type, rather than a binary-only
//! `subject-bound / object-bound` pair, and it carries the Boolean subsumption
//! lattice those consumers need.
//!
//! # The subsumption order (picked deliberately)
//!
//! **A is more general than B (`A ⊑ B`) iff `bound(A) ⊆ bound(B)`.** Fewer bound
//! positions means more general. The all-free pattern is the bottom (⊥, most
//! general — it demands nothing and restricts nothing); the all-bound pattern is
//! the top (⊤, most specific). A demand keyed on the more-general `A` serves any
//! more-specific `B`, because `A` propagates a superset of the bindings `B` would.
//!
//! The lattice operations are consistent with that order — the pattern positions
//! form a Boolean algebra isomorphic to the powerset of `{0..arity}` ordered by ⊆:
//!
//! - [`meet`](BindingPattern::meet) — greatest lower bound = the most-general
//!   common subsumer = the **intersection** of the two bound sets.
//! - [`join`](BindingPattern::join) — least upper bound = the **union** of the two
//!   bound sets.
//!
//! Same-arity is a precondition of both (all patterns for one predicate share its
//! arity); it is asserted.
//!
//! # Determinism
//!
//! A pattern is a `u64` bitset plus an arity, so it is a pure value: the order in
//! which bound positions are supplied to a constructor cannot affect the resulting
//! pattern, its [`code`](BindingPattern::code) string, or any lattice answer. The
//! code string is the only surface that reaches an output path — magic-predicate
//! naming — and it is generated position 0 first, ascending, never from an
//! iteration over a set.

/// A compact bitset over an atom's argument positions: bit `i` set means position
/// `i` is bound.
///
/// Positions are dense small integers, per the workspace's dense-ID doctrine, and
/// a `u64` bitset covers every arity a caller can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BindingPattern {
    /// The atom's arity (number of argument positions). Positions `>= arity` are
    /// never set.
    ///
    /// Ordered before `bound` so that the derived `Ord` groups patterns by arity
    /// first, which is the only cross-arity ordering that means anything.
    arity: u16,
    /// Bit `i` set means argument position `i` is bound.
    bound: u64,
}

impl BindingPattern {
    /// The maximum arity a `u64` bound-bitset can represent.
    pub const MAX_ARITY: usize = 64;

    /// Build a pattern from a per-position boundness iterator (position 0 first).
    ///
    /// # Panics
    ///
    /// Panics if the iterator yields more than [`Self::MAX_ARITY`] positions.
    pub fn from_bools<I: IntoIterator<Item = bool>>(bits: I) -> Self {
        let mut bound: u64 = 0;
        let mut arity: u16 = 0;
        for (i, b) in bits.into_iter().enumerate() {
            assert!(
                i < Self::MAX_ARITY,
                "BindingPattern arity exceeds the {} the u64 bitset carries",
                Self::MAX_ARITY
            );
            if b {
                bound |= 1u64 << i;
            }
            arity += 1;
        }
        Self { arity, bound }
    }

    /// Build an all-free pattern of the given `arity`, then set the given bound
    /// positions.
    ///
    /// The positions may arrive in any order and may repeat; the result depends
    /// only on the set of positions supplied.
    ///
    /// # Panics
    ///
    /// Panics if `arity` exceeds [`Self::MAX_ARITY`] or a bound position is
    /// `>= arity`.
    pub fn from_bound_positions<I: IntoIterator<Item = usize>>(arity: usize, positions: I) -> Self {
        assert!(
            arity <= Self::MAX_ARITY,
            "BindingPattern arity {arity} exceeds the {} the u64 bitset carries",
            Self::MAX_ARITY
        );
        let mut bound: u64 = 0;
        for p in positions {
            assert!(
                p < arity,
                "bound position {p} out of range for arity {arity}"
            );
            bound |= 1u64 << p;
        }
        Self {
            // `arity <= MAX_ARITY` (64) was asserted above, so this cannot truncate.
            arity: arity as u16,
            bound,
        }
    }

    /// The atom's arity (number of argument positions).
    pub fn arity(self) -> usize {
        self.arity as usize
    }

    /// Is argument position `pos` bound? Positions `>= arity` are never bound.
    pub fn is_bound(self, pos: usize) -> bool {
        pos < self.arity() && (self.bound & (1u64 << pos)) != 0
    }

    /// The bound argument positions, ascending.
    pub fn bound_positions(self) -> impl Iterator<Item = usize> {
        (0..self.arity()).filter(move |&p| self.is_bound(p))
    }

    /// `true` iff NO position is bound — the all-free adornment, the ⊥ of the
    /// lattice, demanding and restricting nothing.
    pub fn is_all_free(self) -> bool {
        self.bound == 0
    }

    /// `self ⊑ other`: `self` is MORE GENERAL than (or equal to) `other`.
    ///
    /// That is, every position bound in `self` is bound in `other`
    /// (`bound(self) ⊆ bound(other)`), at the same arity. A demand keyed on the
    /// more-general `self` serves any more-specific `other`. Patterns of differing
    /// arity are incomparable, so this is `false` for them.
    pub fn subsumes(self, other: Self) -> bool {
        self.arity == other.arity && (self.bound & !other.bound) == 0
    }

    /// Greatest lower bound: the most-general common subsumer = the
    /// **intersection** of the two bound sets.
    ///
    /// # Panics
    ///
    /// Panics on an arity mismatch (all patterns for one predicate share its
    /// arity).
    #[must_use]
    pub fn meet(self, other: Self) -> Self {
        assert_eq!(
            self.arity, other.arity,
            "meet requires equal arity (one predicate, one arity)"
        );
        Self {
            arity: self.arity,
            bound: self.bound & other.bound,
        }
    }

    /// Least upper bound: the **union** of the two bound sets.
    ///
    /// # Panics
    ///
    /// Panics on an arity mismatch (all patterns for one predicate share its
    /// arity).
    #[must_use]
    pub fn join(self, other: Self) -> Self {
        assert_eq!(
            self.arity, other.arity,
            "join requires equal arity (one predicate, one arity)"
        );
        Self {
            arity: self.arity,
            bound: self.bound | other.bound,
        }
    }

    /// The deterministic per-position code string: one char per position, `'b'`
    /// (bound) or `'f'` (free), position 0 first.
    ///
    /// Arity 2 yields `"bb"`, `"bf"`, `"fb"` or `"ff"`; arity 3 yields e.g.
    /// `"bfb"`. Round-trips with [`from_code`](BindingPattern::from_code).
    pub fn code(self) -> String {
        (0..self.arity())
            .map(|p| if self.is_bound(p) { 'b' } else { 'f' })
            .collect()
    }

    /// Reconstruct a pattern from its per-position [`code`](BindingPattern::code)
    /// string (`'b'` = bound, `'f'` = free); the string length is the arity.
    ///
    /// # Panics
    ///
    /// Panics on a char other than `'b'`/`'f'`, or a length over
    /// [`Self::MAX_ARITY`].
    pub fn from_code(code: &str) -> Self {
        Self::from_bools(code.chars().map(|c| match c {
            'b' => true,
            'f' => false,
            other => panic!("invalid binding-pattern code char {other:?} in {code:?}"),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One step of the SplitMix64 mixing function — a pure, seed-driven integer
    /// hash with no ambient state.
    fn mix(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A deterministic permutation of `items` selected by `seed`.
    ///
    /// A Fisher-Yates shuffle driven by [`mix`]; the same `seed` always yields the
    /// same order, on every target.
    fn permute<T: Clone>(items: &[T], seed: u64) -> Vec<T> {
        let mut out = items.to_vec();
        let mut state = seed;
        for i in (1..out.len()).rev() {
            let j = (mix(&mut state) % (i as u64 + 1)) as usize;
            out.swap(i, j);
        }
        out
    }

    /// The 8 arity-3 patterns (every subset of `{0,1,2}`), for exhaustive
    /// lattice-law coverage.
    fn all_arity3() -> Vec<BindingPattern> {
        (0..8u8)
            .map(|m| BindingPattern::from_bools((0..3).map(|i| (m >> i) & 1 == 1)))
            .collect()
    }

    #[test]
    fn arity_and_is_bound_agree_with_constructors() {
        let p = BindingPattern::from_bools([true, false, true]);
        assert_eq!(p.arity(), 3);
        assert!(p.is_bound(0));
        assert!(!p.is_bound(1));
        assert!(p.is_bound(2));
        assert!(!p.is_bound(3), "out-of-range position is never bound");

        let q = BindingPattern::from_bound_positions(3, [0, 2]);
        assert_eq!(p, q, "from_bools and from_bound_positions agree");
        assert_eq!(
            q.bound_positions().collect::<Vec<_>>(),
            vec![0, 2],
            "bound_positions ascending"
        );
    }

    #[test]
    fn is_all_free_only_when_nothing_bound() {
        assert!(BindingPattern::from_bools([false, false, false]).is_all_free());
        assert!(!BindingPattern::from_bools([false, true, false]).is_all_free());
    }

    #[test]
    fn subsumes_is_reflexive() {
        for p in all_arity3() {
            assert!(p.subsumes(p), "reflexive: {} ⊑ {}", p.code(), p.code());
        }
    }

    #[test]
    fn subsumes_is_antisymmetric() {
        for a in all_arity3() {
            for b in all_arity3() {
                if a.subsumes(b) && b.subsumes(a) {
                    assert_eq!(a, b, "antisymmetry: {} ⊑⊒ {} ⇒ equal", a.code(), b.code());
                }
            }
        }
    }

    #[test]
    fn subsumes_is_transitive() {
        for a in all_arity3() {
            for b in all_arity3() {
                for c in all_arity3() {
                    if a.subsumes(b) && b.subsumes(c) {
                        assert!(
                            a.subsumes(c),
                            "transitivity: {} ⊑ {} ⊑ {}",
                            a.code(),
                            b.code(),
                            c.code()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn all_free_subsumes_everything_top_subsumes_nothing_else() {
        let bottom = BindingPattern::from_bound_positions(3, []);
        let top = BindingPattern::from_bound_positions(3, [0, 1, 2]);
        for p in all_arity3() {
            assert!(bottom.subsumes(p), "⊥ (all-free) is the most general");
            assert!(p.subsumes(top), "⊤ (all-bound) is the most specific");
        }
    }

    /// Patterns of different arity are incomparable — a demand keyed on one
    /// predicate never serves a different-arity predicate.
    #[test]
    fn subsumes_is_false_across_arities() {
        let two = BindingPattern::from_bound_positions(2, []);
        let three = BindingPattern::from_bound_positions(3, []);
        assert!(!two.subsumes(three));
        assert!(!three.subsumes(two));
    }

    #[test]
    fn meet_is_bound_set_intersection_and_a_lower_bound() {
        for a in all_arity3() {
            for b in all_arity3() {
                let m = a.meet(b);
                assert_eq!(m.bound, a.bound & b.bound, "meet = bound-set intersection");
                // A lower bound under ⊑: m ⊑ a and m ⊑ b.
                assert!(m.subsumes(a), "meet ⊑ a");
                assert!(m.subsumes(b), "meet ⊑ b");
                // Greatest such: any common lower bound l ⊑ a, l ⊑ b has l ⊑ m.
                for l in all_arity3() {
                    if l.subsumes(a) && l.subsumes(b) {
                        assert!(l.subsumes(m), "meet is the GREATEST lower bound");
                    }
                }
            }
        }
    }

    #[test]
    fn join_is_bound_set_union_and_an_upper_bound() {
        for a in all_arity3() {
            for b in all_arity3() {
                let j = a.join(b);
                assert_eq!(j.bound, a.bound | b.bound, "join = bound-set union");
                // An upper bound under ⊑: a ⊑ j and b ⊑ j.
                assert!(a.subsumes(j), "a ⊑ join");
                assert!(b.subsumes(j), "b ⊑ join");
                // Least such: any common upper bound u ⊒ a, u ⊒ b has j ⊑ u.
                for u in all_arity3() {
                    if a.subsumes(u) && b.subsumes(u) {
                        assert!(j.subsumes(u), "join is the LEAST upper bound");
                    }
                }
            }
        }
    }

    #[test]
    fn meet_and_join_are_commutative() {
        for a in all_arity3() {
            for b in all_arity3() {
                assert_eq!(a.meet(b), b.meet(a), "meet commutes");
                assert_eq!(a.join(b), b.join(a), "join commutes");
            }
        }
    }

    #[test]
    fn absorption_laws() {
        for a in all_arity3() {
            for b in all_arity3() {
                assert_eq!(a.meet(a.join(b)), a, "a ∧ (a ∨ b) = a");
                assert_eq!(a.join(a.meet(b)), a, "a ∨ (a ∧ b) = a");
            }
        }
    }

    #[test]
    #[should_panic(expected = "meet requires equal arity")]
    fn meet_arity_mismatch_panics() {
        let a = BindingPattern::from_bools([true, false]);
        let b = BindingPattern::from_bools([true, false, true]);
        let _ = a.meet(b);
    }

    #[test]
    #[should_panic(expected = "join requires equal arity")]
    fn join_arity_mismatch_panics() {
        let a = BindingPattern::from_bools([true, false]);
        let b = BindingPattern::from_bools([true, false, true]);
        let _ = a.join(b);
    }

    #[test]
    #[should_panic(expected = "bound position 3 out of range for arity 3")]
    fn bound_position_beyond_arity_panics() {
        let _ = BindingPattern::from_bound_positions(3, [3]);
    }

    #[test]
    #[should_panic(expected = "invalid binding-pattern code char")]
    fn from_code_rejects_foreign_chars() {
        let _ = BindingPattern::from_code("bxf");
    }

    #[test]
    fn code_round_trips_arity2() {
        for code in ["bb", "bf", "fb", "ff"] {
            let p = BindingPattern::from_code(code);
            assert_eq!(p.arity(), 2);
            assert_eq!(p.code(), code, "arity-2 code round-trips");
        }
    }

    #[test]
    fn code_round_trips_arity3() {
        for m in 0..8u8 {
            let p = BindingPattern::from_bools((0..3).map(|i| (m >> i) & 1 == 1));
            let round = BindingPattern::from_code(&p.code());
            assert_eq!(p, round, "arity-3 code round-trips");
        }
    }

    #[test]
    fn arity2_codes_are_the_canonical_binary_adornments() {
        // The canonical binary adornment mapping — magic-predicate IRIs are minted
        // from these codes, so the strings are a stability surface.
        assert_eq!(BindingPattern::from_bound_positions(2, [0, 1]).code(), "bb");
        assert_eq!(BindingPattern::from_bound_positions(2, [0]).code(), "bf");
        assert_eq!(BindingPattern::from_bound_positions(2, [1]).code(), "fb");
        assert_eq!(BindingPattern::from_bound_positions(2, []).code(), "ff");
    }

    /// The maximum representable arity is genuinely usable end to end: the top
    /// position of a 64-ary atom sets the highest bit without overflowing the
    /// `u64`, and the code string round-trips.
    #[test]
    fn max_arity_is_representable_end_to_end() {
        let arity = BindingPattern::MAX_ARITY;
        let top = BindingPattern::from_bound_positions(arity, 0..arity);
        assert_eq!(top.arity(), arity);
        assert!(top.is_bound(arity - 1), "the 64th position is addressable");
        assert_eq!(top.bound_positions().count(), arity);
        assert_eq!(top.code(), "b".repeat(arity));
        assert_eq!(BindingPattern::from_code(&top.code()), top);
    }

    /// Determinism contract, property style: the order in which bound positions are
    /// supplied cannot affect the stored pattern or any observable derived from it
    /// — the pattern itself, its `code` string, its ascending `bound_positions`, or
    /// any lattice answer. Duplicated positions are likewise absorbed.
    #[test]
    fn position_order_does_not_affect_stored_or_observable_state() {
        let arity = 7usize;
        for mask in 0..(1u32 << arity) {
            let positions: Vec<usize> = (0..arity).filter(|p| mask >> p & 1 == 1).collect();
            let reference = BindingPattern::from_bound_positions(arity, positions.clone());
            let reference_code = reference.code();
            let reference_bound: Vec<usize> = reference.bound_positions().collect();
            assert_eq!(
                reference_bound, positions,
                "bound_positions is ascending regardless of input order"
            );
            for seed in 0..8u64 {
                let mut shuffled = permute(&positions, seed);
                // Duplicates must be idempotent too: a position supplied twice is
                // the same set.
                shuffled.extend(permute(&positions, seed ^ 0x5EED));
                let built = BindingPattern::from_bound_positions(arity, shuffled);
                assert_eq!(built, reference, "mask {mask:b} seed {seed}: same pattern");
                assert_eq!(built.code(), reference_code, "same code string");
                assert_eq!(
                    built.bound_positions().collect::<Vec<_>>(),
                    reference_bound,
                    "same ascending bound positions"
                );
                assert!(built.subsumes(reference) && reference.subsumes(built));
                assert_eq!(built.meet(reference), reference);
                assert_eq!(built.join(reference), reference);
            }
        }
    }
}
