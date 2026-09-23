// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The distance kernels the embedding kNN surface ranks by, and the total order it
//! ranks in.
//!
//! # Why binary64, when the workspace's other ranked-retrieval surface uses no floats
//!
//! Ranking is only reproducible if the numbers it compares are, and a scoring kernel is
//! the classic place for that to fail: two builds of one engine disagree by a unit in the
//! last place, two nearly-tied candidates swap, and the same query over the same data
//! returns rows in a different order depending on where it was compiled. That is an
//! answer divergence, not a rounding detail.
//!
//! The usual remedy is to leave floating point behind entirely. That is the right remedy
//! when the kernel needs a **transcendental** — a `ln`, an `exp`, a `pow` — because those
//! are the operations IEEE-754 does *not* require to be correctly rounded, so every libm
//! is entitled to a different answer and they take it. A kNN kernel needs none of them.
//! Squared Euclidean distance is subtraction, multiplication and addition; cosine
//! distance adds a division and a square root. **All five are correctly rounded by
//! IEEE-754**, which means each one has exactly one permissible result for a given pair
//! of inputs on every conforming target — `wasm32-unknown-unknown` included.
//!
//! So there are only two ways a float kernel can still diverge, and this module closes
//! both:
//!
//! * **Reassociation.** Floating-point addition is not associative, so a sum's value
//!   depends on the order it was accumulated in. Every fold here is
//!   [`purrdf_core::distance::Exact`], whose order is part of its definition: sixteen
//!   binary64 lanes, lane `l` summing the terms at indices `16·c + l` over the whole
//!   sixteen-element chunks in ascending `c`; then the pairwise tree `(l, l+8)`,
//!   `(l, l+4)`, `(l, l+2)`, `(0, 1)`; then the remaining terms added one at a time in
//!   ascending index. No accumulation is ever split across rayon workers. Because the
//!   order is the definition and not an accident of compilation, every target and every
//!   dispatch path computes it, and the sixteen independent lanes are what lets it
//!   vectorize without licensing the compiler to reorder anything.
//!   The test `accumulation_order_is_pinned_and_the_sequential_orders_genuinely_differ`
//!   exercises a vector whose sum genuinely differs under the sequential orders.
//! * **Fused multiply-add.** `a * b + c` computed as a single FMA rounds once where the
//!   written form rounds twice, and the two differ. Rust never contracts them
//!   implicitly, and PURREMB v1 forbids the fusion normatively ("performed in the written
//!   order without a fused multiply-add"). Every product is therefore bound to a named
//!   local before it is added — the same shape, for the same reason, that
//!   `purrdf_core`'s own deterministic L2 fold uses. There is no FMA on any exact path.
//!
//! This is not a weaker guarantee than the integer route; for these five operations it is
//! the same guarantee, and it is the one the artifact format itself already specifies.
//! Its limit is stated where it belongs: on [`Kernel::distance`], which reports whether
//! its result stayed finite rather than ranking by an infinity.
//!
//! # The reassociated entry points are a second contract, not a mode
//!
//! [`Kernel::distance_reassociated`] and [`Kernel::distance_bounded_reassociated`]
//! compute the same metric under [`Reassociated`]: the sum is reassociated and may be
//! contracted to fused multiply-add, so its last bits depend on the target, the build
//! and the dispatch path, and near-ties may order differently. They are separate
//! functions with their own names, they take the [`Resolved`] handle that
//! `Reassociated::resolve` returned after checking the float environment and picking
//! the path, and they never stand in for the exact entry points, nor those for them.
//! The divergence is stated in words by `Resolved::evidence`, for every consumer that
//! records it.
//!
//! # The metric definitions are PURREMB's, not this module's
//!
//! PURREMB v1 defines the three built-in metrics as `1 - dot(x, y) / (L2(x) · L2(y))`,
//! `-dot(x, y)` and `sum((x[i] - y[i])²)`, **smaller ranks first**, with cosine undefined
//! for a zero-norm operand. This module implements exactly those, in exactly that sense,
//! and does not invent a fourth.

use core::cmp::Ordering;

use purrdf_core::DistanceMetric;
use purrdf_core::distance::{Exact, Measure};

/// The stored-scalar width trait, the bound a partial fold is tested against, the
/// bounded outcome, and the reassociated arithmetic with the resolved handle its entry
/// points take, re-exported from the module that defines them so a caller names one
/// type for each.
pub use purrdf_core::distance::{Bound, Bounded, Reassociated, Resolved, Scalar};

/// The three built-in metrics, decoded from a family contract's declaration.
///
/// A separate type from [`DistanceMetric`] on purpose: that enum has a fourth variant for
/// a caller-defined extension metric whose parameters are opaque bytes this engine cannot
/// interpret, and admitting it here would mean ranking by a rule nobody in this process
/// knows. The conversion is the one place that refusal is expressed, so no code path
/// downstream has to remember it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kernel {
    /// `1 - dot(x, y) / (L2(x) · L2(y))` — undefined for a zero-norm operand.
    Cosine,
    /// `-dot(x, y)`.
    NegativeDot,
    /// `sum((x[i] - y[i])²)`.
    SquaredEuclidean,
}

impl Kernel {
    /// The kernel a declared [`DistanceMetric`] names, or `None` for an extension metric.
    #[must_use]
    pub const fn of(metric: &DistanceMetric) -> Option<Self> {
        match metric {
            DistanceMetric::Cosine => Some(Self::Cosine),
            DistanceMetric::NegativeDot => Some(Self::NegativeDot),
            DistanceMetric::SquaredEuclidean => Some(Self::SquaredEuclidean),
            DistanceMetric::Extension { .. } => None,
        }
    }

    /// The [`Measure`] the distance arithmetic computes for this kernel.
    #[must_use]
    pub const fn measure(self) -> Measure {
        match self {
            Self::Cosine => Measure::Cosine,
            Self::NegativeDot => Measure::NegativeDot,
            Self::SquaredEuclidean => Measure::SquaredEuclidean,
        }
    }

    /// Whether this kernel divides by an operand's L2 norm, and therefore needs every
    /// vector it ranks to have a non-zero one.
    #[must_use]
    pub const fn needs_norms(self) -> bool {
        matches!(self, Self::Cosine)
    }

    /// [`Kernel::distance`], permitted to stop once the answer cannot clear `bound`.
    ///
    /// Returns [`Bounded::Below`] carrying the distance when it is on the near side of the
    /// bound, and [`Bounded::Beyond`] — with no value — when it is not. A caller that only
    /// compares the distance against a threshold and discards it otherwise can use this
    /// instead of [`Kernel::distance`] and get a bit-identical answer wherever it gets one
    /// at all.
    ///
    /// Only [`Kernel::SquaredEuclidean`] can actually abandon: its terms are squares, so every
    /// lane of the exact fold is non-decreasing and a checkpoint every 64 components reads a
    /// value no larger than the total. `NegativeDot` and `Cosine` accumulate signed products
    /// whose partial sums can move in either direction, so a partial sum proves nothing
    /// about the total; those are computed in full and then classified. The API is total so
    /// a caller need not branch on the kernel to stay correct.
    #[must_use]
    pub fn distance_bounded<A: Scalar, B: Scalar>(
        self,
        query: &[A],
        query_norm: f64,
        candidate: &[B],
        candidate_norm: f64,
        bound: Bound,
    ) -> Bounded {
        Exact::distance_bounded(
            self.measure(),
            query,
            query_norm,
            candidate,
            candidate_norm,
            bound,
        )
    }

    /// The distance from `query` to `candidate` under this kernel, or `None` when the
    /// computation left the finite range.
    ///
    /// The operands may be stored at either width PURREMB defines, independently of each
    /// other -- an `f64` query against an `f32` corpus is the ordinary case for a search
    /// whose query was embedded at query time. Widening is exact and happens per component
    /// inside the fold, so the answer is bit-identical to one computed from operands widened
    /// in advance. See [`Scalar`].
    ///
    /// `query_norm` and `candidate_norm` are the operands' [`norm`]s. They are
    /// parameters rather than recomputed here because a candidate's norm does not depend
    /// on the query: a search over `n` candidates computes each one once at index
    /// construction instead of `n` times per invocation, and — more to the point — a norm
    /// computed once is a norm that cannot be computed two ways. They are ignored by the
    /// kernels that do not divide by them.
    ///
    /// # Why `None` rather than an infinity
    ///
    /// The stored scalars are finite (PURREMB rejects a non-finite one at read time), but
    /// finite inputs can still produce an infinite sum of squares near the top of the binary64
    /// range. An infinity would still *sort*, and it would sort last, so a caller would
    /// receive a confidently-ranked answer computed from a number that overflowed. Saying
    /// so instead is the whole difference between a wrong answer and an error.
    #[must_use]
    pub fn distance<A: Scalar, B: Scalar>(
        self,
        query: &[A],
        query_norm: f64,
        candidate: &[B],
        candidate_norm: f64,
    ) -> Option<f64> {
        // Cosine is written exactly as PURREMB v1 states it -- one product, one quotient,
        // one subtraction, each rounded on its own -- inside the exact arithmetic.
        Exact::distance(self.measure(), query, query_norm, candidate, candidate_norm)
    }

    /// The distance from `query` to `candidate` under the [`Reassociated`] arithmetic,
    /// or `None` when it left the finite range.
    ///
    /// The same metric as [`Kernel::distance`], computed under a different contract: the
    /// sum is reassociated and may be contracted to fused multiply-add along the
    /// `arithmetic` handle's dispatch path, so the result may differ in its last bits from
    /// [`Kernel::distance`] and between dispatch paths or builds, the sign of a zero
    /// result is unspecified, and near-ties may order differently
    /// ([`Resolved::evidence`] says so in the words a consumer records). On one path in
    /// one build it is a function of its operands. A non-finite result is refused
    /// exactly as [`Kernel::distance`] refuses it.
    ///
    /// `arithmetic` comes from `Reassociated::resolve`, called once per scan, which
    /// checked the float environment and chose the path.
    #[must_use]
    pub fn distance_reassociated<A: Scalar, B: Scalar>(
        self,
        arithmetic: Resolved<Reassociated>,
        query: &[A],
        query_norm: f64,
        candidate: &[B],
        candidate_norm: f64,
    ) -> Option<f64> {
        arithmetic.distance(self.measure(), query, query_norm, candidate, candidate_norm)
    }

    /// [`Kernel::distance_reassociated`], permitted to stop once the answer cannot clear
    /// `bound`.
    ///
    /// A [`Bounded::Below`] value is bit-identical to what
    /// [`Kernel::distance_reassociated`] returns on the same handle, and
    /// [`Bounded::Beyond`] is returned only when that value meets the bound: the
    /// reassociated fold combines its 64-element blocks in order, so every checkpoint is
    /// a true prefix of the full value. As with [`Kernel::distance_bounded`], only
    /// [`Kernel::SquaredEuclidean`] actually abandons.
    #[must_use]
    pub fn distance_bounded_reassociated<A: Scalar, B: Scalar>(
        self,
        arithmetic: Resolved<Reassociated>,
        query: &[A],
        query_norm: f64,
        candidate: &[B],
        candidate_norm: f64,
        bound: Bound,
    ) -> Bounded {
        arithmetic.distance_bounded(
            self.measure(),
            query,
            query_norm,
            candidate,
            candidate_norm,
            bound,
        )
    }
}

/// The Euclidean (L2) norm of `vector`, by the same scaled fold PURREMB's own
/// deterministic normalization uses.
///
/// The obvious `sum(x²).sqrt()` overflows for a vector whose components are individually
/// representable but whose squares are not, and underflows to zero for a vector of
/// subnormals — in both cases producing a norm that is wrong rather than imprecise, which
/// for cosine means dividing by it. The scaled fold carries a running maximum magnitude
/// and a sum of squared *ratios* instead, so it is exact in the same places and finite in
/// many more.
///
/// It is `purrdf_core`'s normative fold itself, not a copy of it: a space stored with
/// `PrefixPostprocessing::DeterministicL2` was normalized by that fold, and a cosine
/// kernel that measured its norms by a second transcription could drift from the
/// artifact's own arithmetic. PURREMB §13.2 fixes its sequential written order, so it is
/// never reordered.
///
/// A zero-length vector, and a vector of all zeros, both norm to `0.0`. That is reported
/// rather than refused here; refusing it is [`Kernel::needs_norms`]'s caller's job,
/// because a zero norm is fatal for cosine and harmless for the other two.
#[must_use]
pub fn norm<T: Scalar>(vector: &[T]) -> f64 {
    purrdf_core::distance::norm(vector)
}

/// One scored candidate: how far it is, and which row of the space it is.
///
/// # The order is strict, and that is load-bearing
///
/// `Ord` is `(distance ASC, row ASC)`. Row numbers are distinct, so **no two candidates
/// can compare equal**: this is a strict total order, not a partial one with a
/// tie-breaking convention bolted on. Two consequences follow, and both are relied on:
///
/// * a bounded top-`k` heap and a full sort cannot disagree about the answer, because
///   there is no pair whose relative order is left open for them to resolve differently;
/// * `sort_unstable` is canonical here rather than merely faster, because there is no
///   pair a stable sort would keep in input order and an unstable one would not.
///
/// The tie-break is meaningful rather than arbitrary. A PURREMB target set is *sorted and
/// deduplicated by `TargetId`* when it is built, and a `TargetId` is a domain-separated
/// digest of the target's canonical identity — so ascending row number is ascending
/// canonical content order, and two independently produced artifacts over the same
/// targets number their rows identically. Ties in *distance* are real (two identical
/// vectors are genuinely equidistant from everything); ties in *rank* are impossible.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ranked {
    /// The distance under the space's declared metric. Always finite.
    pub distance: f64,
    /// The candidate's row in the space.
    pub row: usize,
}

impl Ranked {
    /// The distance, with a negative zero folded to a positive one.
    ///
    /// [`f64::total_cmp`] is a total order over every bit pattern, which is what makes it
    /// usable as an `Ord` — but it orders `-0.0` strictly before `+0.0`, and those two are
    /// the *same distance*. `-dot(x, y)` produces `-0.0` for an orthogonal pair while
    /// squared Euclidean produces `+0.0`, so without this the rank of two genuinely
    /// equidistant candidates would depend on which sign of zero their arithmetic happened
    /// to land on rather than on the row tie-break. Folding once, here, makes the order
    /// agree with numeric equality everywhere it is defined.
    #[allow(
        clippy::float_cmp,
        reason = "an exact comparison against zero is the intent: this asks which of two \
                  bit patterns the value is, not whether it is near zero"
    )]
    fn key(self) -> f64 {
        if self.distance == 0.0 {
            0.0
        } else {
            self.distance
        }
    }
}

impl Eq for Ranked {}

impl Ord for Ranked {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key()
            .total_cmp(&other.key())
            .then_with(|| self.row.cmp(&other.row))
    }
}

impl PartialOrd for Ranked {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The `k` best of `candidates`, in rank order, using a heap bounded at `k`.
///
/// `candidates` is consumed in row order and never re-read, so this is a single pass over
/// the space. The heap holds at most `k` entries and its maximum is the *worst* entry
/// retained, which is exactly the one a better candidate should displace.
///
/// The result is a genuine prefix of the fully sorted candidate list — not merely a set of
/// `k` good ones — because [`Ranked`]'s order is strict and total (see its docs). That
/// property is what lets the caller hand these rows to the evaluator as an ordered stream
/// whose first `n` rows are the `n` nearest neighbours for every `n ≤ k`, which in turn is
/// what makes the engine's row ceiling sound against this relation.
pub fn best(k: usize, candidates: impl IntoIterator<Item = Ranked>) -> Vec<Ranked> {
    if k == 0 {
        return Vec::new();
    }
    let mut heap: std::collections::BinaryHeap<Ranked> = std::collections::BinaryHeap::new();
    for candidate in candidates {
        if heap.len() < k {
            heap.push(candidate);
            continue;
        }
        // `peek` is the worst retained entry; a candidate no better than it cannot enter.
        if heap.peek().is_some_and(|worst| candidate < *worst) {
            heap.pop();
            heap.push(candidate);
        }
    }
    heap.into_sorted_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ranked candidate, tersely.
    const fn r(distance: f64, row: usize) -> Ranked {
        Ranked { distance, row }
    }

    // ---- the kernels ------------------------------------------------------

    #[test]
    fn the_three_built_in_metrics_map_and_an_extension_metric_does_not() {
        assert_eq!(Kernel::of(&DistanceMetric::Cosine), Some(Kernel::Cosine));
        assert_eq!(
            Kernel::of(&DistanceMetric::NegativeDot),
            Some(Kernel::NegativeDot)
        );
        assert_eq!(
            Kernel::of(&DistanceMetric::SquaredEuclidean),
            Some(Kernel::SquaredEuclidean)
        );
        assert_eq!(
            Kernel::of(&DistanceMetric::Extension {
                identifier: "https://example.org/metric".to_owned(),
                parameter_encoding: "application/cbor".to_owned(),
                parameters: vec![1],
            }),
            None,
            "an opaque caller-defined metric names a rule this engine cannot evaluate"
        );
        assert!(Kernel::Cosine.needs_norms());
        assert!(!Kernel::NegativeDot.needs_norms());
        assert!(!Kernel::SquaredEuclidean.needs_norms());
    }

    #[test]
    fn each_kernel_matches_its_hand_computation() {
        let a = [3.0_f64, 4.0];
        let b = [0.0_f64, 5.0];
        let norm_a = norm(&a);
        let norm_b = norm(&b);
        // |a| = 5 exactly, |b| = 5 exactly; dot = 20.
        assert_eq!(norm_a, 5.0);
        assert_eq!(norm_b, 5.0);

        // sum((3-0)^2 + (4-5)^2) = 9 + 1 = 10.
        assert_eq!(
            Kernel::SquaredEuclidean.distance(&a, norm_a, &b, norm_b),
            Some(10.0)
        );
        // -dot = -20.
        assert_eq!(
            Kernel::NegativeDot.distance(&a, norm_a, &b, norm_b),
            Some(-20.0)
        );
        // 1 - 20/(5*5) = 1 - 0.8 = 0.19999999999999996 in binary64: pinned by BITS, not
        // by a decimal that would hide which of two adjacent doubles this is.
        let cosine = Kernel::Cosine
            .distance(&a, norm_a, &b, norm_b)
            .expect("finite");
        assert_eq!(
            cosine.to_bits(),
            (1.0_f64 - 0.8_f64).to_bits(),
            "cosine is the written `1 - dot/(|x||y|)`, rounded once per operation"
        );
    }

    #[test]
    fn a_vector_is_at_distance_zero_from_itself_and_cosine_is_only_nearly_so() {
        // Squared Euclidean is EXACTLY zero: every difference is exactly zero and every
        // square of it is too, so nothing rounds.
        let v = [1.0_f64, 2.0, 3.0];
        let n = norm(&v);
        assert_eq!(
            Kernel::SquaredEuclidean.distance(&v, n, &v, n),
            Some(0.0),
            "sum of squared zero differences"
        );

        // Cosine is NOT, and pretending otherwise would be the wrong claim to build a
        // surface on. `dot(v, v)` and `|v| · |v|` are two different roundings of the same
        // real number, so their quotient is one ULP off 1 and the distance is one ULP off
        // zero — here, slightly NEGATIVE. That is a property of the definition PURREMB
        // states, not a defect in this kernel: what the surface actually needs is that a
        // vector ranks ahead of everything else, which it does, and it is asserted that
        // way rather than by a zero that is not there.
        let cosine = Kernel::Cosine.distance(&v, n, &v, n).expect("finite");
        assert!(
            cosine.abs() <= 4.0 * f64::EPSILON,
            "cosine self-distance is within a few ULP of zero, got {cosine}"
        );
        let other = [3.0_f64, 2.0, 1.0];
        let other_norm = norm(&other);
        let across = Kernel::Cosine
            .distance(&v, n, &other, other_norm)
            .expect("finite");
        assert!(
            cosine < across,
            "and it still ranks strictly ahead of a different direction: {cosine} vs \
             {across}"
        );
    }

    #[test]
    fn accumulation_order_is_pinned_and_the_sequential_orders_genuinely_differ() {
        // The determinism claim with teeth. `1e16 + 1 - 1e16` is `0` folded left-to-right
        // and `1` when the two large terms meet first, because binary64 addition is not
        // associative. This vector puts `1e16` in lane 0, `1` in lane 1 and `-1e16` in
        // lane 8 of the exact fold's sixteen lanes, so its dot product with the all-ones
        // vector has different correct answers under different orders, and asserting
        // WHICH one this kernel produces is asserting that the order is the pinned one.
        let mut a = [0.0_f64; 16];
        a[0] = 1e16;
        a[1] = 1.0;
        a[8] = -1e16;
        let ones = [1.0_f64; 16];

        let pinned = Kernel::NegativeDot
            .distance(&a, 0.0, &ones, 0.0)
            .expect("finite");
        assert_eq!(
            pinned, -1.0,
            "the lane tree pairs lane 0 with lane 8 first: (1e16 - 1e16) cancels exactly, \
             and the 1 in lane 1 survives"
        );

        // The two sequential orders, computed here rather than assumed, so the test proves
        // they really do differ on this input instead of merely asserting that they might.
        let mut forward = 0.0_f64;
        for (x, y) in a.iter().zip(ones.iter()) {
            let product = x * y;
            forward += product;
        }
        let mut reversed = 0.0_f64;
        for (x, y) in a.iter().zip(ones.iter()).rev() {
            let product = x * y;
            reversed += product;
        }
        assert_eq!(
            forward, 0.0,
            "ascending index order: 1e16 + 1 rounds the 1 away, then - 1e16 is 0"
        );
        assert_eq!(
            reversed, 0.0,
            "descending index order: -1e16 + 1 rounds the 1 away, then + 1e16 is 0"
        );
        assert_ne!(
            -pinned, forward,
            "if these agreed, this test would be watching nothing"
        );
    }

    #[test]
    fn the_scaled_norm_survives_magnitudes_the_naive_one_does_not() {
        // The naive `sum(x*x).sqrt()` overflows here: 1e200^2 is infinity in binary64.
        let huge = [1e200_f64, 1e200];
        let naive = {
            let mut sum = 0.0_f64;
            for value in &huge {
                let square = value * value;
                sum += square;
            }
            sum.sqrt()
        };
        assert!(
            naive.is_infinite(),
            "the naive fold must genuinely overflow"
        );
        let scaled = norm(&huge);
        assert!(scaled.is_finite());
        assert_eq!(scaled, 1e200 * core::f64::consts::SQRT_2);

        // And the underflow direction: squares of subnormals flush to zero.
        let tiny = [1e-200_f64, 1e-200];
        let naive_tiny = {
            let mut sum = 0.0_f64;
            for value in &tiny {
                let square = value * value;
                sum += square;
            }
            sum.sqrt()
        };
        assert_eq!(naive_tiny, 0.0, "the naive fold must genuinely underflow");
        assert_eq!(norm(&tiny), 1e-200 * core::f64::consts::SQRT_2);
    }

    #[test]
    fn a_zero_vector_norms_to_zero_rather_than_to_a_nan() {
        assert_eq!(norm(&[0.0_f64, 0.0, 0.0]), 0.0);
        assert_eq!(norm::<f64>(&[]), 0.0);
        assert_eq!(norm(&[-0.0_f64]), 0.0);
    }

    #[test]
    fn an_overflowing_distance_is_reported_rather_than_ranked_as_an_infinity() {
        let a = [f64::MAX, f64::MAX];
        let b = [-f64::MAX, -f64::MAX];
        assert_eq!(
            Kernel::SquaredEuclidean.distance(&a, 0.0, &b, 0.0),
            None,
            "an infinite sum of squares still sorts, and would sort LAST — a confidently \
             ranked answer computed from a number that overflowed"
        );

        // The neighbouring VALID case: large but not overflowing must still rank. A
        // finiteness check that refused both would be an over-refusal wearing a
        // correctness costume.
        let c = [1e150_f64, 1e150];
        let d = [0.0_f64, 0.0];
        // Written the same way the kernel writes it, so the expectation is the value
        // binary64 actually has rather than the decimal a reader would guess (`1e150`
        // squared is not exactly `1e300`).
        let square = 1e150_f64 * 1e150_f64;
        let expected = 0.0_f64 + square + square;
        assert!(expected.is_finite());
        assert_eq!(
            Kernel::SquaredEuclidean.distance(&c, 0.0, &d, 0.0),
            Some(expected),
            "two squares of 1e150 are finite and must be ranked, not refused"
        );
    }

    // ---- the order --------------------------------------------------------

    #[test]
    fn the_rank_order_is_distance_then_row() {
        assert!(r(1.0, 9) < r(2.0, 0), "a nearer candidate ranks first");
        assert!(r(1.0, 3) < r(1.0, 4), "an equal distance breaks by row");
        assert_eq!(r(1.0, 3).cmp(&r(1.0, 3)), Ordering::Equal);
    }

    #[test]
    fn negative_zero_and_positive_zero_are_the_same_distance() {
        // Otherwise `-dot` (which produces `-0.0` for an orthogonal pair) and squared
        // Euclidean (which produces `+0.0`) would rank equidistant candidates by the sign
        // of a zero rather than by the row tie-break.
        assert_eq!(r(-0.0, 5).cmp(&r(0.0, 5)), Ordering::Equal);
        assert!(
            r(-0.0, 5) < r(0.0, 6),
            "so the row is what separates them, in both spellings"
        );
        assert!(r(0.0, 5) < r(-0.0, 6));
    }

    #[test]
    fn no_two_distinct_candidates_ever_compare_equal() {
        // The strictness the bounded heap depends on, over a grid that deliberately
        // repeats every distance.
        let all: Vec<Ranked> = (0..8_usize)
            .map(|row| r(f64::from(u32::try_from(row % 3).expect("small")), row))
            .collect();
        for (index, left) in all.iter().enumerate() {
            for right in &all[index + 1..] {
                assert_ne!(
                    left.cmp(right),
                    Ordering::Equal,
                    "{left:?} and {right:?} tied, so a heap and a sort could disagree"
                );
            }
        }
    }

    // ---- selection --------------------------------------------------------

    /// A deliberately unsorted candidate list with repeated distances.
    fn candidates() -> Vec<Ranked> {
        vec![
            r(3.0, 0),
            r(1.0, 1),
            r(2.0, 2),
            r(1.0, 3),
            r(5.0, 4),
            r(2.0, 5),
            r(0.5, 6),
        ]
    }

    #[test]
    fn the_bounded_heap_equals_the_full_sort_prefix_at_every_k() {
        let mut sorted = candidates();
        sorted.sort_unstable();
        for k in 0..=sorted.len() + 2 {
            let selected = best(k, candidates());
            let expected = &sorted[..k.min(sorted.len())];
            assert_eq!(
                selected, expected,
                "the heap and the sort must agree ROW FOR ROW at k = {k}, not merely on \
                 how many rows there are"
            );
        }
    }

    #[test]
    fn selection_returns_exactly_k_when_k_rows_exist() {
        // The short-bag guard: `>= k` is satisfied by returning everything, and `<= k` by
        // returning nothing, so the count is asserted exactly at every k below the total.
        let total = candidates().len();
        for k in 0..=total {
            assert_eq!(
                best(k, candidates()).len(),
                k,
                "k = {k} of {total} candidates must yield EXACTLY k rows"
            );
        }
        assert_eq!(
            best(total + 1, candidates()).len(),
            total,
            "and asking for more than exist yields every one of them, not a short bag \
             nor a padded one"
        );
    }

    #[test]
    fn selecting_from_nothing_is_empty_rather_than_an_error() {
        assert_eq!(best(5, Vec::new()), [] as [_; 0]);
        assert_eq!(best(0, candidates()), [] as [_; 0]);
    }

    #[test]
    fn selection_is_a_pure_function_of_the_candidate_set_not_its_order() {
        // The heap's retention decisions depend on arrival order; its ANSWER must not.
        let mut reversed = candidates();
        reversed.reverse();
        assert_eq!(best(4, candidates()), best(4, reversed));
    }

    /// A deterministic stream of values in `[-1, 1)`, for the differential tests below.
    fn stream(len: usize, seed: u64) -> Vec<f64> {
        let mut state = seed;
        (0..len)
            .map(|_| {
                state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
                let mut z = state;
                z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
                z ^= z >> 31;
                ((z >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0)
            })
            .collect()
    }

    #[test]
    fn a_bounded_fold_that_does_not_abandon_is_bit_identical_to_the_full_one() {
        // The whole licence for early abandonment. An index may only use the bounded fold
        // because a candidate it does NOT abandon is scored exactly as `distance` scores it
        // -- not approximately, bit for bit -- so no ranked answer can move. Lengths either
        // side of the block size, so the chunking is exercised rather than skipped.
        for len in [1_usize, 63, 64, 65, 127, 128, 200, 4_096] {
            let a = stream(len, 0x5EED_0001);
            let b = stream(len, 0x5EED_0002);
            for kernel in [
                Kernel::SquaredEuclidean,
                Kernel::NegativeDot,
                Kernel::Cosine,
            ] {
                let (na, nb) = (norm(&a), norm(&b));
                let full = kernel.distance(&a, na, &b, nb).expect("finite");
                let bounded = kernel.distance_bounded(&a, na, &b, nb, Bound::Above(f64::INFINITY));
                match bounded {
                    Bounded::Below(value) => assert_eq!(
                        value.to_bits(),
                        full.to_bits(),
                        "{kernel:?} at len {len}: the bounded fold must agree bit for bit"
                    ),
                    other => panic!("{kernel:?} at len {len}: expected a value, got {other:?}"),
                }
            }
        }
    }

    #[test]
    fn abandoning_never_contradicts_the_full_distance() {
        // Swept across bounds that land below, at, and above the true distance, in both
        // strictness modes. `Beyond` must only ever be returned when the full distance
        // genuinely meets the bound -- an abandonment that fired early would be a wrong
        // answer, not a fast one.
        let a = stream(300, 0xB0B_0001);
        let b = stream(300, 0xB0B_0002);
        let (na, nb) = (norm(&a), norm(&b));
        let truth = Kernel::SquaredEuclidean
            .distance(&a, na, &b, nb)
            .expect("finite");

        for scale in [0.0_f64, 0.25, 0.5, 0.99, 1.0, 1.01, 2.0] {
            let limit = truth * scale;
            for bound in [Bound::AtOrAbove(limit), Bound::Above(limit)] {
                let got = Kernel::SquaredEuclidean.distance_bounded(&a, na, &b, nb, bound);
                match got {
                    Bounded::Below(value) => {
                        assert_eq!(value.to_bits(), truth.to_bits());
                        assert!(
                            !bound.is_met_by(truth),
                            "returned a value for a distance that meets {bound:?}"
                        );
                    }
                    Bounded::Beyond => assert!(
                        bound.is_met_by(truth),
                        "abandoned at {bound:?} but the true distance {truth} does not meet it"
                    ),
                    Bounded::NonFinite => panic!("the fixture is finite"),
                }
            }
        }
    }

    #[test]
    fn an_overflowing_fold_is_reported_rather_than_abandoned() {
        // The hard fail survives. A pair whose squares overflow must be NonFinite, not
        // silently swallowed as "far away" -- a caller that treated overflow as a large
        // distance would rank a candidate on a number that never existed.
        let huge = vec![f64::MAX / 2.0; 8];
        let zero = vec![0.0_f64; 8];
        assert_eq!(
            Kernel::SquaredEuclidean.distance_bounded(
                &huge,
                0.0,
                &zero,
                0.0,
                Bound::AtOrAbove(1.0)
            ),
            Bounded::NonFinite,
            "an overflow is an error even when a bound would otherwise have abandoned"
        );
        assert_eq!(
            Kernel::SquaredEuclidean.distance(&huge, 0.0, &zero, 0.0),
            None,
            "and the unbounded fold still refuses it too"
        );
    }

    /// The reassociated arithmetic this process resolves.
    fn reassociated() -> Resolved<Reassociated> {
        use purrdf_core::distance::Arithmetic as _;
        Reassociated::resolve().expect("the test thread runs the default float environment")
    }

    #[test]
    fn the_reassociated_entry_points_compute_the_metric_within_the_error_bound() {
        let fast = reassociated();
        for len in [1_usize, 63, 64, 65, 200, 4_096] {
            let a = stream(len, 0xFA57_0001);
            let b = stream(len, 0xFA57_0002);
            let (na, nb) = (norm(&a), norm(&b));
            // Every term is at most 4 in magnitude (components are in [-1, 1)), so
            // `Σ|tᵢ| ≤ 4·n` and the contract's `2·n·ε·Σ|tᵢ|` is at most `8·n²·ε`. Each
            // arithmetic is within `n·ε·Σ|tᵢ|` of the true sum, so the two are within
            // that bound of each other.
            let bound = 8.0 * (len * len) as f64 * f64::EPSILON;
            for kernel in [Kernel::SquaredEuclidean, Kernel::NegativeDot] {
                let exact = kernel.distance(&a, na, &b, nb).expect("finite");
                let reassociated = kernel
                    .distance_reassociated(fast, &a, na, &b, nb)
                    .expect("finite");
                assert!(
                    (exact - reassociated).abs() <= bound,
                    "{kernel:?} at len {len} on {}: {exact} vs {reassociated}",
                    fast.path()
                );
                assert_eq!(
                    kernel.distance_bounded_reassociated(
                        fast,
                        &a,
                        na,
                        &b,
                        nb,
                        Bound::Above(f64::INFINITY)
                    ),
                    Bounded::Below(reassociated),
                    "{kernel:?} at len {len}: the bounded form agrees bit for bit"
                );
            }
            // Bounded abandonment agrees with the full reassociated value on both sides
            // of it: one ulp above is not met, the value itself is `AtOrAbove`.
            let full = Kernel::SquaredEuclidean
                .distance_reassociated(fast, &a, na, &b, nb)
                .expect("finite");
            assert_eq!(
                Kernel::SquaredEuclidean.distance_bounded_reassociated(
                    fast,
                    &a,
                    na,
                    &b,
                    nb,
                    Bound::AtOrAbove(full.next_up())
                ),
                Bounded::Below(full)
            );
            assert_eq!(
                Kernel::SquaredEuclidean.distance_bounded_reassociated(
                    fast,
                    &a,
                    na,
                    &b,
                    nb,
                    Bound::AtOrAbove(full)
                ),
                Bounded::Beyond
            );
        }
        assert!(
            fast.evidence()
                .is_some_and(|text| text.contains(fast.path().name())),
            "the reassociated handle names its divergence and its path"
        );
    }

    #[test]
    fn the_reassociated_entry_points_refuse_an_overflow_as_the_exact_ones_do() {
        let fast = reassociated();
        let a = [f64::MAX, f64::MAX];
        let b = [-f64::MAX, -f64::MAX];
        assert_eq!(
            Kernel::SquaredEuclidean.distance_reassociated(fast, &a, 0.0, &b, 0.0),
            None
        );
        assert_eq!(
            Kernel::SquaredEuclidean.distance_bounded_reassociated(
                fast,
                &a,
                0.0,
                &b,
                0.0,
                Bound::AtOrAbove(1.0)
            ),
            Bounded::NonFinite
        );
        // The valid neighbour ranks.
        let c = [1e150_f64, 1e150];
        let d = [0.0_f64, 0.0];
        assert!(
            Kernel::SquaredEuclidean
                .distance_reassociated(fast, &c, 0.0, &d, 0.0)
                .is_some_and(f64::is_finite)
        );
    }

    #[test]
    fn an_f32_operand_scores_exactly_as_its_widened_copy_does() {
        // The licence for storing a corpus at the width PURREMB stored it. Widening an f32
        // is exact, so folding widened-per-component must give the SAME BITS as folding a
        // matrix widened in advance -- otherwise halving the resident size would quietly be
        // a different index. Asserted on bits, not on an epsilon.
        let mut state = 0xF32_0000_1234_ABCD_u64;
        let mut next = || {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^= z >> 31;
            ((z >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0) as f32
        };
        for len in [1_usize, 63, 64, 65, 200, 4_096] {
            let a32: Vec<f32> = (0..len).map(|_| next()).collect();
            let b32: Vec<f32> = (0..len).map(|_| next()).collect();
            let a64: Vec<f64> = a32.iter().copied().map(f64::from).collect();
            let b64: Vec<f64> = b32.iter().copied().map(f64::from).collect();

            assert_eq!(
                norm(&a32).to_bits(),
                norm(&a64).to_bits(),
                "len {len}: the norm fold must not see the storage width"
            );

            for kernel in [
                Kernel::SquaredEuclidean,
                Kernel::NegativeDot,
                Kernel::Cosine,
            ] {
                let (na, nb) = (norm(&a64), norm(&b64));
                let narrow = kernel.distance(&a32, na, &b32, nb).expect("finite");
                let wide = kernel.distance(&a64, na, &b64, nb).expect("finite");
                assert_eq!(
                    narrow.to_bits(),
                    wide.to_bits(),
                    "{kernel:?} at len {len}: storing narrow must not move a single bit"
                );

                // And the mixed case, which is the ordinary one: a query embedded at query
                // time is f64, the corpus it searches is f32.
                let mixed = kernel.distance(&a64, na, &b32, nb).expect("finite");
                assert_eq!(
                    mixed.to_bits(),
                    wide.to_bits(),
                    "{kernel:?} at len {len}: a mixed-width pair must agree too"
                );
            }
        }
    }
}
