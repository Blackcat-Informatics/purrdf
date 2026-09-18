// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The fusion profile: the identity-bearing law a fusion runs under.
//!
//! A profile is not a bag of knobs. It fixes the decay rule and its smoothing
//! constant, every stratum's weight, and the declared total tie-break; the
//! maximum number of contributions a candidate may receive follows from the
//! weights, because a candidate may surface at most once per stratum. Two
//! answers fused under profiles differing in any of those are answers to
//! different questions, so the profile is content-addressed: its identity is a
//! domain-separated BLAKE3 digest over a canonical, versioned, length-framed
//! encoding, never over a serde document or a `Hash`.

use std::collections::BTreeMap;

use purrdf_text::{Fixed, SCALE_DIGITS};

use crate::canonical::{Reader, Writer};
use crate::error::FusionError;
use crate::id::{FUSION_PROFILE_VERSION, FusionProfileId};
use crate::iri::Iri;
use crate::reciprocal_rank::{
    MonotoneDepth, class_width, deepest_rank_within_width, minimum_weight_for_depth, monotone_depth,
};

// Canonical discriminators. One tag space per enum, never reused.
const DECAY_RECIPROCAL_RANK: u8 = 0;
const DECAY_WEIGHTED_RECIPROCAL_RANK: u8 = 1;
const TIE_BREAK_SCORE_DESC_STRATUM_RANK_ASC_CANONICAL_TERM: u8 = 0;
const ROUNDING_TRUNCATE_TOWARD_ZERO: u8 = 0;
const ITEM_ENCODING_CANONICAL_LEXICAL: u8 = 0;

/// How a candidate's contribution decays with its rank within a stratum.
///
/// The enum is closed. Further monotone decay rules are additive variants, and
/// because the rule is part of the profile's canonical bytes, adding one never
/// re-opens a previously-issued identity: a profile built on
/// [`ReciprocalRank`](Self::ReciprocalRank) encodes tag zero and hashes to the
/// value it always did, whatever else the enum grows.
///
/// # The two rules compute the same quantity to different accuracies
///
/// Both express `w / (K + r)` at the layer's declared scale
/// `S = 10^SCALE_DIGITS`, truncating toward zero, and neither needs a
/// transcendental. They differ in where the weight enters, and that decides how
/// deep a stratum can be read before the arithmetic stops separating ranks:
///
/// * [`ReciprocalRank`](Self::ReciprocalRank) rounds the reciprocal first —
///   `trunc(w_raw · trunc(S / D) / S)` with `D = K + r`. The inner truncation is
///   a ceiling no weight can lift, so the monotone range ends just past
///   `D = sqrt(S) = 10^6` for **every** weight at or above one.
/// * [`WeightedReciprocalRank`](Self::WeightedReciprocalRank) folds the weight
///   into the numerator — `trunc(w_raw / D)`, one exactly-rounded division. Its
///   value is never below the other's and never more than `⌊w⌋ + 1` raw units above
///   it, and its monotone range runs to roughly `10^6 · sqrt(w)`, so a heavier
///   stratum is legitimately readable deeper.
///
/// Neither is a default. A profile names the rule it runs under, and the choice
/// is part of what its identity fixes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DecayRule {
    /// `weight · reciprocal(K + rank)`, with the reciprocal truncated to the
    /// declared scale before the weight is applied, and `K >= 1`.
    ReciprocalRank {
        /// The smoothing constant `K`.
        k: u32,
    },
    /// `weight / (K + rank)` as a single exactly-rounded division at the
    /// declared scale, with `K >= 1`.
    WeightedReciprocalRank {
        /// The smoothing constant `K`.
        k: u32,
    },
}

impl DecayRule {
    /// The smoothing constant this rule carries.
    #[must_use]
    pub const fn k(self) -> u32 {
        match self {
            Self::ReciprocalRank { k } | Self::WeightedReciprocalRank { k } => k,
        }
    }

    /// The smallest stratum weight that still separates every adjacent pair of
    /// ranks up to `depth` under this rule.
    ///
    /// This is the profile-design calculus read in reverse. Rather than build a
    /// profile, read its [`FusionProfile::monotone_depth`] and adjust, a caller
    /// that knows how deep it must read asks for the weight that buys it.
    ///
    /// The answer is the true minimum, not a sufficient over-estimate, and it is
    /// **not** bisected: whether a weight reaches a depth is not monotone in the
    /// weight, so a binary search over it would land on whichever side of an
    /// oscillation its probes happened to sample. Instead each adjacent-rank
    /// constraint names the next weight that could satisfy it, and the search
    /// walks those candidates without ever skipping one — so it never names a
    /// heavier weight than the arithmetic actually requires. Remember that
    /// weights are read as **ratios**, so raising one stratum to reach a depth
    /// changes its share of every fused score — this reports what the depth
    /// costs, and whether to pay it is the caller's.
    ///
    /// # Errors
    ///
    /// Three refusals, and they are three different facts.
    ///
    /// [`FusionError::InvalidRank`] when `depth` is zero. A depth counts 1-based
    /// ranks read from rank one, so zero names no rank to separate.
    ///
    /// [`FusionError::DepthUnreachable`] when no weight reaches `depth`. Under
    /// [`Self::ReciprocalRank`] that is a real wall and not a conservative one:
    /// the reciprocal is truncated before the weight is applied, so once two
    /// adjacent ranks collide there they are equal for every weight. The error
    /// carries the exact rank where the rule saturates.
    /// [`Self::WeightedReciprocalRank`] has no such wall at all — its reachable
    /// depth grows with the weight — so it never raises this.
    ///
    /// [`FusionError::DepthBeyondPlanRange`] when `depth` is deeper than a plan
    /// can record, which a plan does as a `u32`. This is the only refusal
    /// [`Self::WeightedReciprocalRank`] makes, and it is a limit of that
    /// encoding rather than of the rule: the folded arithmetic separates ranks
    /// past `u32::MAX` given a heavy enough weight, and at `u32::MAX` itself it
    /// answers. Nothing downstream could carry a deeper depth, so quoting a
    /// weight for one would price a plan that cannot be written.
    pub fn weight_for_depth(self, depth: u64) -> Result<Fixed, FusionError> {
        minimum_weight_for_depth(self, depth)
    }

    /// How many consecutive ranks around `rank` a stratum weighted `weight`
    /// cannot be told apart at under this rule.
    ///
    /// The resolution algebra with no stratum in it. A width is a property of
    /// four numbers — the rule, its smoothing constant, the weight and the rank
    /// — and of nothing else, so a caller asking about *arithmetic* rather than
    /// about a configured stratum asks here, exactly as it asks
    /// [`Self::weight_for_depth`] here. [`FusionProfile::class_width`] is the
    /// same answer looked up by stratum, for a caller that already holds a law
    /// and means one of the strata that law weights.
    ///
    /// One means `rank` is still separated from both its neighbours by score
    /// alone. A width of `w` means `w` consecutive ranks share a contribution,
    /// so their relative order in a fused answer falls through to the declared
    /// tie-break's later keys rather than being decided by relevance. This is
    /// the resolution curve, of which [`Self::weight_for_depth`] prices a single
    /// point.
    ///
    /// # Errors
    ///
    /// [`FusionError::InvalidRank`] when `rank` is zero. Ranks are 1-based, so
    /// there is no rank zero for a class to form around.
    ///
    /// [`FusionError::InvalidK`] when this rule's smoothing constant is zero,
    /// and [`FusionError::Overflow`] when a contribution leaves the fixed-point
    /// range. A refusal is reported and never rendered as a width: a width of
    /// one is the most favourable claim this algebra can make about a
    /// resolution, and making it where the arithmetic produced nothing would be
    /// a false claim at exactly the point no answer exists.
    pub fn class_width(self, weight: Fixed, rank: u64) -> Result<u64, FusionError> {
        class_width(self, weight, rank)
    }

    /// The deepest depth that can be read with every rank in it sitting in a
    /// class no wider than `max_width`.
    ///
    /// The resolution algebra with no stratum in it, exactly as
    /// [`Self::class_width`] is: this arithmetic is a property of the rule,
    /// its smoothing constant, the weight and the tolerance, and of nothing
    /// else, so a caller asking about arithmetic rather than about a
    /// configured stratum asks here. [`FusionProfile::deepest_rank_within_width`]
    /// is the same answer looked up by stratum, for a caller that already
    /// holds a law and means one of the strata that law weights.
    ///
    /// `max_width` of one agrees exactly with the depth
    /// [`Self::weight_for_depth`]'s inverse would report: the deepest rank
    /// still separated from both its neighbours. Larger values answer the
    /// question a caller reading deeply actually has — not "where does this
    /// stop being exact" but "how far can I read and still have ranks ordered
    /// to within the resolution I can live with".
    ///
    /// # What it costs to ask
    ///
    /// The answer is found by walking rank by rank, because the class width is
    /// not monotone in the rank and a bisection over it silently over-reports.
    /// The walk starts at the separating depth [`Self::weight_for_depth`]
    /// prices rather than at rank one — every class below that point is a
    /// singleton by definition — and runs until the first run of
    /// `max_width + 1` ranks sharing one contribution.
    ///
    /// A class at depth `D` is about `D²/B` ranks wide for the rule's own `B`
    /// — `min(w, 1) · S` under [`Self::ReciprocalRank`] and `w · S` under
    /// [`Self::WeightedReciprocalRank`], where `S` is the fixed-point scale
    /// [`SCALE_DIGITS`](purrdf_text::SCALE_DIGITS) declares — and the separating
    /// depth is about `sqrt(B)`. So the answer lands near `sqrt(max_width)`
    /// times that depth and the walk is about `sqrt(max_width) - 1` times it,
    /// one integer division per step. A `max_width` of one does not walk at
    /// all; a small tolerance costs a fraction of the separating depth; a large
    /// tolerance at a heavy weight under the folded rule — where the separating
    /// depth itself grows as `sqrt(w)` — walks very far.
    ///
    /// The walk stops at the deepest depth a plan can record, saturating there
    /// and returning it, so it is bounded by fewer than `2^32` steps however it
    /// is asked and cannot fail to terminate. It is a design-time question all
    /// the same — price a depth budget once while choosing weights — and not
    /// something to put in a hot loop.
    ///
    /// # Errors
    ///
    /// [`FusionError::InvalidK`] when this rule's smoothing constant is zero,
    /// and [`FusionError::Overflow`] when a contribution leaves the
    /// fixed-point range. A refusal is reported and never rendered as a
    /// depth: the rank the walk stopped at is where the arithmetic gave out,
    /// not a depth this rule was measured to deliver, and returning it as one
    /// would quote a resolution nothing established.
    pub fn deepest_rank_within_width(
        self,
        weight: Fixed,
        max_width: u64,
    ) -> Result<u64, FusionError> {
        deepest_rank_within_width(self, weight, max_width)
    }

    /// The canonical discriminator byte for this rule.
    const fn tag(self) -> u8 {
        match self {
            Self::ReciprocalRank { .. } => DECAY_RECIPROCAL_RANK,
            Self::WeightedReciprocalRank { .. } => DECAY_WEIGHTED_RECIPROCAL_RANK,
        }
    }

    /// The rule a canonical discriminator byte names, or `None` for a tag this
    /// build does not write.
    const fn from_tag(tag: u8, k: u32) -> Option<Self> {
        match tag {
            DECAY_RECIPROCAL_RANK => Some(Self::ReciprocalRank { k }),
            DECAY_WEIGHTED_RECIPROCAL_RANK => Some(Self::WeightedReciprocalRank { k }),
            _ => None,
        }
    }
}

/// The declared total order applied to candidates with equal fused scores.
///
/// The enum is closed and holds the one law this layer ships: fused score
/// descending, then the candidate's best (minimum) stratum rank ascending, then
/// canonical term byte order ascending. The final key is total because two
/// distinct candidates never share a canonical term.
///
/// There is deliberately **no** setter for it. A profile carries this law
/// because it is the only one, and an API to select among one alternative is a
/// choice a caller cannot make — it would read as a knob while doing nothing. A
/// second tie-break is an additive variant plus the selector it would then
/// genuinely need, written when there is a second law to select; the value is
/// already in the profile's canonical bytes, so adding one re-opens no
/// previously-issued identity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TieBreak {
    /// Score descending, best rank ascending, canonical term ascending.
    #[default]
    ScoreDescStratumRankAscCanonicalTerm,
}

impl TieBreak {
    /// The canonical discriminator byte for this tie-break.
    fn tag(self) -> u8 {
        match self {
            Self::ScoreDescStratumRankAscCanonicalTerm => {
                TIE_BREAK_SCORE_DESC_STRATUM_RANK_ASC_CANONICAL_TERM
            }
        }
    }
}

/// A validated fusion law.
///
/// Construction refuses every unusable configuration: `K = 0`, an empty or
/// non-positive weight map, and a maximum admitted fused score that leaves the
/// fixed-point range. Once constructed, the profile is immutable and its
/// [`FusionProfile::id`] names it.
///
/// # The contribution maximum is derived, never declared
///
/// A candidate may surface at most **once per stratum** — that is the ranked
/// streams' own per-stream uniqueness, enforced row by row — so the largest
/// number of contributions any candidate can legitimately receive is exactly
/// the number of strata the profile weights. That is what
/// [`max_contributions`](Self::max_contributions) reports, and it is computed
/// from `weights` rather than taken from the caller.
///
/// It was once a parameter, and the parameter had exactly one usable value. A
/// value above the stratum count named a bound nothing could reach. A value
/// below it bought a refusal that depended on the *corpus* rather than on the
/// request: the same profile served every query until some document happened to
/// surface in one stratum more than the caller guessed, and then the whole
/// fusion failed. A caller cannot predict that, because it is not a property of
/// anything the caller wrote.
///
/// The value is still part of the profile's canonical bytes and therefore of
/// its identity, so a profile that used to pass its stratum count explicitly
/// hashes to exactly the identity it always did.
///
/// # Weight, scale and depth are one coupled quantity
///
/// Four declared quantities decide how deep a stratum's ranks stay *ordered*:
/// the **decay rule**, the stratum's **weight** `w`, the fixed-point **scale**
/// `S = 10^SCALE_DIGITS` the whole layer computes in, and the **per-stratum
/// depth** the plan records.
///
/// Under [`DecayRule::ReciprocalRank`] a contribution at 1-based rank `r` is
/// formed as
///
/// ```text
/// trunc( w · trunc(S / (K + r)) / S )
/// ```
///
/// and the inner truncation is the one that matters: the reciprocal is rounded
/// to the scale **before** the weight is applied, so the reciprocal's own
/// resolution is a ceiling no weight can lift. For any `w ≥ 1` the condition for
/// two adjacent ranks to stay distinct reduces exactly to `trunc(S / D) ≠
/// trunc(S / (D + 1))` with `D = K + r`, which first fails just above
/// `D = sqrt(S) = 10^6` **whatever the weight is** — a weight of a thousand buys
/// no more depth than a weight of one. Below one the weight does bind, and
/// distinctness is guaranteed while `w · trunc(S / (D · (D + 1))) ≥ S`, which is
/// roughly `(K + r)² ≲ w · S`.
///
/// Under [`DecayRule::WeightedReciprocalRank`] the weight is folded into the
/// numerator instead — `trunc(w_raw / (K + r))`, where `w_raw = w · S` — so
/// there is one truncation rather than two and no inner ceiling at all.
/// Adjacent values then differ by `w_raw / (D · (D + 1))`, and distinctness is
/// guaranteed exactly while `D · (D + 1) ≤ w_raw`.
///
/// Either way the guarantee is only sufficient, never necessary — two
/// contributions often differ when it fails, because a truncation can straddle
/// an integer — so the exact first-collision rank is what
/// [`FusionProfile::monotone_depth`] reports, and every quantity derived from it
/// is measured against that exact value rather than against a conservative
/// closed form. A conservative bound here would understate the depth a profile
/// really orders, and nothing in this layer is entitled to refuse or discourage
/// a depth its own arithmetic in fact delivers.
///
/// # A deep stratum requires a heavy one — the coupling read the other way
///
/// The paragraph above reads the coupling as a limit: *given* a weight, here is
/// the depth past which ordering degrades. Under
/// [`DecayRule::WeightedReciprocalRank`] it reads just as usefully in reverse,
/// and a profile author needs the reverse reading to choose a weight at all.
///
/// The relation is
///
/// ```text
/// depth ≲ 10^6 · sqrt(w)
/// ```
///
/// — a weight of one admits about one million ranks, a weight of one hundred
/// about ten million, and a weight of two hundred about fourteen million.
/// Squaring it gives the requirement directly: a stratum that
/// must be read `depth` ranks deep needs `w ≳ (depth / 10^6)²`. A weight is
/// therefore not only "how much this stratum counts relative to its neighbours";
/// under this rule it is also the budget that buys ordered depth. A profile that
/// weights a deep stratum lightly answers at a coarser rank resolution there,
/// and says so — on the compiled plan as
/// [`PlannedResolution`](crate::PlannedResolution) before anything runs, and in
/// the fused trailer afterwards — rather than quietly returning rows the fusion
/// cannot separate. [`DecayRule::weight_for_depth`] reads the relation in the
/// direction a profile author actually needs: name the depth, get the weight.
///
/// The two readings do not conflict, because weights are only ever compared with
/// each other: scaling every stratum's weight by one hundred leaves every
/// relative ratio, every fused order and every tie-break untouched, while
/// multiplying every monotone range by ten. A profile that needs depth raises
/// the whole weight vector, not one stratum's share of it. (It does raise
/// [`FusionProfile::ceiling`] by the same factor, and that is checked against
/// the fixed-point range at construction, so the scaling is bounded by
/// arithmetic rather than by a rule.) Under
/// [`DecayRule::ReciprocalRank`] the same scaling buys nothing, which is the
/// whole difference between the rules.
///
/// Beyond that depth nothing errors and nothing becomes nondeterministic: the
/// tie-break is total, so the answer stays a pure function of its inputs. What
/// is lost is that the fused **score** stops separating ranks. Two candidates
/// one rank apart accumulate the same number, the sum across strata stops being
/// rank-weighted — a candidate at ranks 5000 and 5001 totals exactly what one at
/// 4000 and 6001 does — and their relative order falls through to the declared
/// tie-break's later keys, best stratum rank ascending and then canonical term
/// byte order. Within one stratum best-rank still reproduces rank order, so the
/// visible damage is across strata; the arithmetic damage is everywhere.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FusionProfile {
    /// The rank-decay rule, carrying its smoothing constant.
    decay: DecayRule,
    /// Per-stratum weights, keyed by stratum IRI. A `BTreeMap` so the canonical
    /// encoding is a pure function of the fields, never of iteration order.
    weights: BTreeMap<Iri, Fixed>,
    /// The declared total tie-break.
    tie_break: TieBreak,
    /// The maximum number of contributions one candidate may receive: the
    /// number of strata this profile weights, since a candidate may surface at
    /// most once in each.
    ///
    /// Derived at construction from `weights`, and — unlike [`Self::ceiling`]
    /// and [`Self::monotone_depths`] — still written into the canonical bytes,
    /// because it is the bound the fusion engine is held to and an identity
    /// that did not record it would not say what law an answer ran under.
    max_contributions: u32,
    /// The largest fused score the profile admits, derived at construction and
    /// checked against the fixed-point ceiling.
    ceiling: Fixed,
    /// Per-stratum monotone depth: the largest depth at which that stratum's
    /// contributions are still strictly rank-ordered under this profile's
    /// weight and smoothing constant.
    ///
    /// Derived at construction exactly as [`Self::ceiling`] is, and absent from
    /// the canonical bytes for the same reason: it is a pure function of the
    /// fields already encoded, so encoding it would put one fact in the identity
    /// twice.
    monotone_depths: BTreeMap<Iri, u64>,
}

impl FusionProfile {
    /// Build a profile that names its decay rule explicitly, with explicit
    /// weights and smoothing constant, and the canonical tie-break.
    ///
    /// A caller that needs a stratum read deeper than the truncated reciprocal
    /// can order — see this type's own documentation for the `depth ≲ 10^6 ·
    /// sqrt(w)` relation — names [`DecayRule::WeightedReciprocalRank`] here.
    /// The choice is part of the profile's canonical bytes, so it is part of
    /// its identity and never changes the numbers under a name already in
    /// circulation.
    ///
    /// # `weights` is read as ratios only — mind which constructor made them
    ///
    /// Nothing here reads a weight as an absolute quantity. A fusion compares
    /// weights with each other and with nothing else, so multiplying every
    /// stratum's weight by the same factor leaves every fused score's *order*,
    /// every tie-break and every identity-bearing decision untouched. That is
    /// the useful property this type's own documentation reads in reverse to
    /// buy ordered depth, and it is also the hazard.
    ///
    /// [`Fixed`] has two constructors that read almost the same and differ by
    /// `10^SCALE_DIGITS`: [`Fixed::from_integer(1)`](Fixed::from_integer) is the
    /// number one, and [`Fixed::from_raw(1)`](Fixed::from_raw) is one raw unit,
    /// which is `10^-12`. Mixing them inside one weight map is a ratio error of
    /// a *trillion* that this constructor cannot see and will not refuse: every
    /// weight is strictly positive, the ceiling still fits, the fusion still
    /// runs, and the answer is a plausible-looking ranking in which one stratum
    /// has been switched off. Build the whole map with one constructor, and
    /// prefer the one that takes the number a reader means.
    ///
    /// # A weight here acts in rank space, which is what a bounded score cannot
    ///
    /// A weight declared here multiplies a contribution derived from a **rank**,
    /// never from a producer's own score, and that is the property it is worth
    /// declaring. A stratum weighted twice its neighbour contributes twice as
    /// much at rank 1 and still twice as much at rank 1000: the declared ratio
    /// holds at every magnitude.
    ///
    /// A weight folded into a *bounded* score does not behave that way, and the
    /// difference decides a configuration question one layer below. Merging two
    /// producers into one takes two things, not one — comparable scores, and a
    /// weighting the score itself can carry — and a shared scoring law buys only
    /// the first. Two producers over one embedding space (a heading class and a
    /// body class) share a law in every sense, so the registry's
    /// one-stratum-one-producer refusal reads as an invitation to merge them. If
    /// the host also means one class to **outweigh** the other and the score is a
    /// cosine distance in `[0, 2]` (lower-better), that merge cannot carry the
    /// weight: sorting merged by `d / w`, the favoured class wins only past a
    /// similarity edge of `(1 - s)(1 - w_low/w_high)`, which at a weight ratio of
    /// one half is `0.45` against a match at similarity `0.1`, `0.0005` against
    /// one at `0.999`, and exactly zero against a perfect match — largest for the
    /// worst matches and vanishing where the top-k contest is decided. The margin
    /// is not the only half of it: for a weighted threshold `s'` the expected
    /// number of competitors outranking a hit is `n · (1 - F(s'))`, linear in
    /// corpus size, so no fixed weight ratio survives corpus growth even away
    /// from the boundary. Such a pair belongs in **two strata weighted here**,
    /// despite the shared law. An unbounded score — BM25F's field weights are
    /// natively one — carries a differential weight through a merge and needs no
    /// second stratum for it.
    ///
    /// # Fusion precision degrades as a weight approaches the raw unit
    ///
    /// A weight near `Fixed::from_raw(1)` is legal, and sometimes exactly what a
    /// caller means; what it costs is resolution, paid per contribution rather
    /// than once at the end. A contribution is an integer count of raw units
    /// truncated toward zero, and at rank `r` it is about `w_raw / (K + r)` of
    /// them, so the truncation discards at most one unit out of that many. The
    /// relative loss is therefore about
    ///
    /// ```text
    /// (K + rank) / w_raw
    /// ```
    ///
    /// — a floor set by the weight's **raw magnitude**, not by its ratio to its
    /// neighbours, and the one quantity in this layer that reads a weight
    /// absolutely. Two anchors, both at `K = 60` and rank 1: a weight of 1000 raw
    /// units makes `1000 / 61 = 16.39…`, emitted as exactly **16** raw units, so
    /// about 2.4% of every contribution is gone before anything is summed; the
    /// whole-number weight `Fixed::from_integer(1)` (`10^12` raw) makes
    /// `16393442622.95…`, emitted as `16393442622`, losing about `6 · 10^-11`
    /// relative.
    ///
    /// Nothing refuses a small weight and nothing should. Scaling the entire
    /// weight vector up raises this floor out of sight while preserving every
    /// ratio, every fused order and every tie-break — the same scaling the type's
    /// own documentation reads for depth.
    ///
    /// # Errors
    ///
    /// * [`FusionError::InvalidK`] when `k == 0`.
    /// * [`FusionError::EmptyWeights`] when no stratum is declared.
    /// * [`FusionError::NonPositiveWeight`] for any weight `<= 0`.
    /// * [`FusionError::Overflow`] when the admitted maximum fused score does
    ///   not fit the fixed-point range, or the stratum count does not fit the
    ///   `u32` the canonical encoding writes.
    pub fn with_decay(
        weights: BTreeMap<Iri, Fixed>,
        decay: DecayRule,
    ) -> Result<Self, FusionError> {
        let k = decay.k();
        if k == 0 {
            return Err(FusionError::InvalidK { k });
        }
        if weights.is_empty() {
            return Err(FusionError::EmptyWeights);
        }
        for (stratum, weight) in &weights {
            if *weight <= Fixed::ZERO {
                return Err(FusionError::NonPositiveWeight {
                    stratum: stratum.as_str().to_owned(),
                    weight: *weight,
                });
            }
        }

        // One contribution per stratum is the most any candidate can receive,
        // so the stratum count *is* the bound. The empty map is already refused
        // above, so this is at least one.
        let max_contributions = u32::try_from(weights.len()).map_err(|_| FusionError::Overflow)?;
        let ceiling = Self::compute_ceiling(&weights, max_contributions)?;
        // Derived once, here, rather than on every admission: a profile is
        // immutable and is routinely reused across many searches, and the search
        // is bounded but not free.
        //
        // The derivation carries the decay rule's own refusal rather than
        // rendering one as a depth. The two operands that could provoke one are
        // already refused above — a zero smoothing constant and a non-positive
        // weight — so no profile this constructor accepts reaches it; it is
        // propagated so that relaxing either check surfaces the failure instead
        // of quoting a separation nothing measured.
        let monotone_depths = weights
            .iter()
            .map(|(stratum, weight)| Ok((stratum.clone(), monotone_depth(decay, *weight)?)))
            .collect::<Result<BTreeMap<Iri, u64>, FusionError>>()?;
        Ok(Self {
            decay,
            weights,
            tie_break: TieBreak::default(),
            max_contributions,
            ceiling,
            monotone_depths,
        })
    }

    /// The smoothing constant `K` this profile fixes.
    #[must_use]
    pub const fn k_parameter(&self) -> u32 {
        self.decay.k()
    }

    /// The rank-decay rule this profile fixes.
    #[must_use]
    pub const fn decay(&self) -> DecayRule {
        self.decay
    }

    /// The declared total tie-break.
    #[must_use]
    pub const fn tie_break(&self) -> TieBreak {
        self.tie_break
    }

    /// The maximum number of contributions one candidate may receive: the
    /// number of strata this profile weights.
    ///
    /// This is a fact about the profile, not a policy a caller chose — see the
    /// type's own documentation for why it is derived. A candidate that
    /// receives more than this has not crossed a budget; some stream or stream
    /// set broke its uniqueness, which is
    /// [`FusionError::MaxContributionsExceeded`](crate::FusionError::MaxContributionsExceeded).
    #[must_use]
    pub const fn max_contributions(&self) -> u32 {
        self.max_contributions
    }

    /// The admitted maximum fused score: the largest weight times the number of
    /// strata, which is the true bound on a candidate's sum.
    ///
    /// It is a bound the arithmetic already guarantees, so fusion enforces it by
    /// construction rather than by testing each sum against it. A candidate
    /// receives at most one contribution per stratum — that count is what fusion
    /// checks, as
    /// [`FusionError::MaxContributionsExceeded`](crate::FusionError::MaxContributionsExceeded)
    /// — and `K >= 1` with a 1-based rank makes every reciprocal at most one
    /// half, so every contribution is at most half its stratum's weight. The
    /// largest sum a fusion can reach is therefore exactly half of this value,
    /// never this value, whichever decay rule is in force. A runtime refusal for
    /// crossing it would be a branch no profile and no stream could take.
    #[must_use]
    pub const fn ceiling(&self) -> Fixed {
        self.ceiling
    }

    /// The per-stratum weights, in canonical stratum order.
    #[must_use]
    pub const fn weights(&self) -> &BTreeMap<Iri, Fixed> {
        &self.weights
    }

    /// The weight declared for `stratum`, if any.
    #[must_use]
    pub fn weight(&self, stratum: &Iri) -> Option<Fixed> {
        self.weights.get(stratum).copied()
    }

    /// The largest depth at which `stratum`'s contributions are still strictly
    /// rank-ordered under this profile, or `None` when the profile declares no
    /// weight for it.
    ///
    /// This is the exact first-collision rank, not a conservative estimate: at
    /// this depth every adjacent pair of ranks still produces a distinct
    /// contribution, and at one rank more the first pair collides. See this
    /// type's own documentation for the coupling it reports on.
    ///
    /// Reading past it is **permitted and reports itself**. The answer stays
    /// correct and deterministic — the declared tie-break is total — at a
    /// coarser rank resolution, and the fused trailer says per stratum how deep
    /// the answer in hand was actually read. Use [`Self::class_width`] to ask
    /// how coarse, rather than only whether.
    ///
    /// `None` is the honest answer for an unweighted stratum rather than zero:
    /// a profile that says nothing about a stratum has said nothing about how
    /// deep it may be read either, and that stratum contributes nothing to a
    /// fusion under this profile.
    #[must_use]
    pub fn monotone_depth(&self, stratum: &Iri) -> Option<MonotoneDepth> {
        self.monotone_depths
            .get(stratum)
            .copied()
            .map(MonotoneDepth::from_rank)
    }

    /// How many consecutive ranks around `rank` this profile cannot tell apart
    /// in `stratum`.
    ///
    /// One means `rank` is still separated from both neighbours. A width of `w`
    /// means `w` consecutive ranks share a contribution, so their relative order
    /// in the fused answer is decided by the tie-break's later keys — best
    /// stratum rank ascending, then canonical term bytes — rather than by score.
    ///
    /// This is the resolution curve [`Self::monotone_depth`] reports a single
    /// point of. Past that point the width grows rather than jumping to
    /// nonsense, and knowing it is four ranks rather than ten thousand is the
    /// difference between an answer a caller can use and one it cannot.
    ///
    /// The width itself is [`DecayRule::class_width`], which takes a weight
    /// rather than a stratum. This is the same arithmetic reached by the name a
    /// law gave the weight, so a caller holding a profile need not restate a
    /// weight the profile already carries; a caller with no stratum in hand asks
    /// the rule directly instead of inventing one to ask through.
    ///
    /// `Ok(None)` is a stratum this profile declares no weight for. That is an
    /// absence and not a failure — a profile silent about a stratum has said
    /// nothing about its resolution either, and that stratum contributes
    /// nothing to a fusion under this profile — so it is kept distinct from the
    /// refusal below rather than folded into it.
    ///
    /// # Errors
    ///
    /// [`FusionError::InvalidRank`] when `rank` is zero. Ranks are 1-based, so
    /// there is no rank zero for a class to form around.
    ///
    /// [`FusionError::InvalidK`] when the profile's smoothing constant is zero,
    /// and [`FusionError::Overflow`] when a contribution leaves the fixed-point
    /// range. Neither is reachable through a profile this type built —
    /// [`Self::with_decay`] refuses a zero constant, and no contribution
    /// exceeds its own weight in magnitude — and both are carried rather than
    /// assumed away, because the alternative is to report the most favourable
    /// width there is at exactly the point the arithmetic produced no width at
    /// all.
    pub fn class_width(&self, stratum: &Iri, rank: u64) -> Result<Option<u64>, FusionError> {
        let Some(weight) = self.weights.get(stratum) else {
            return Ok(None);
        };
        self.decay.class_width(*weight, rank).map(Some)
    }

    /// The deepest rank in `stratum` whose indifference class is still no wider
    /// than `max_width`.
    ///
    /// `max_width` of one is [`Self::monotone_depth`]. Larger values answer the
    /// question a caller reading deeply actually has: not "where does this stop
    /// being exact" but "how far can I read and still have ranks ordered to
    /// within the resolution I can live with".
    ///
    /// `Ok(None)` is a stratum this profile declares no weight for, exactly as
    /// in [`Self::class_width`], and it is an absence rather than a failure.
    ///
    /// # What it costs to ask
    ///
    /// The answer is walked rank by rank — the class width is not monotone in
    /// the rank, so bisecting it would silently over-report — starting at this
    /// stratum's separating depth ([`Self::monotone_depth`]) rather than at rank
    /// one, and stopping at the first run of `max_width + 1` ranks that share a
    /// contribution. The answer lands near `sqrt(max_width)` times the
    /// separating depth, so the walk is about `sqrt(max_width) - 1` times that
    /// depth, one integer division per step: a `max_width` of one does no
    /// walking at all, a small tolerance costs a fraction of the separating
    /// depth, and a large tolerance at a heavy weight under
    /// [`DecayRule::WeightedReciprocalRank`] — the case whose separating depth
    /// itself grows with the weight — walks very far.
    ///
    /// The walk saturates at the deepest depth a plan can record and returns
    /// it, which bounds it at fewer than `2^32` steps however it is asked; it
    /// cannot fail to terminate. It is a design-time question all the same —
    /// price a depth budget once while choosing weights — and not something to
    /// put in a hot loop. See [`DecayRule::deepest_rank_within_width`] for the
    /// arithmetic behind the estimate.
    ///
    /// # Errors
    ///
    /// [`FusionError::InvalidK`] when the profile's smoothing constant is zero,
    /// and [`FusionError::Overflow`] when a contribution leaves the fixed-point
    /// range. Neither is reachable through a profile this type built, and both
    /// are carried rather than assumed away: the rank the walk stopped at is
    /// where the arithmetic gave out, not a depth this profile was measured to
    /// deliver.
    pub fn deepest_rank_within_width(
        &self,
        stratum: &Iri,
        max_width: u64,
    ) -> Result<Option<u64>, FusionError> {
        let Some(weight) = self.weights.get(stratum) else {
            return Ok(None);
        };
        self.decay
            .deepest_rank_within_width(*weight, max_width)
            .map(Some)
    }

    /// The profile's canonical, length-framed bytes.
    ///
    /// The encoding is a pure function of the fields: weights are sorted by
    /// stratum and every integer is little-endian, so the bytes are identical on
    /// every target.
    ///
    /// The contribution maximum is written even though it is derived from the
    /// weight count already encoded above it. That is deliberate: it is the
    /// bound the engine enforces, the layout predates its derivation, and
    /// keeping it means a profile that used to declare its stratum count
    /// explicitly still hashes to the identity it was issued under.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut writer = Writer::new();
        writer.u16(FUSION_PROFILE_VERSION);
        writer.u8(self.decay.tag());
        writer.u32(self.decay.k());
        writer.u64(self.weights.len() as u64);
        for (stratum, weight) in &self.weights {
            writer.string(stratum.as_str());
            writer.i128(weight.into_raw());
        }
        writer.u8(self.tie_break.tag());
        writer.u32(self.max_contributions);
        // The declared scale, rounding direction and item encoding are part of
        // the law, so a change to any of them changes the identity.
        writer.u32(SCALE_DIGITS);
        writer.u8(ROUNDING_TRUNCATE_TOWARD_ZERO);
        writer.u8(ITEM_ENCODING_CANONICAL_LEXICAL);
        writer.into_bytes()
    }

    /// Decode a profile from its canonical bytes.
    ///
    /// # Errors
    ///
    /// [`FusionError::MalformedProfile`] when the encoding is truncated, carries
    /// an unknown tag or invalid UTF-8, holds an invalid IRI, has trailing
    /// bytes, states a contribution maximum that is not its own stratum count,
    /// or begins with a version this build does not write.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, FusionError> {
        let mut reader = Reader::new(bytes);
        let version = reader
            .u16()
            .map_err(|error| FusionError::MalformedProfile(error.to_string()))?;
        if version != FUSION_PROFILE_VERSION {
            return Err(FusionError::MalformedProfile(format!(
                "unsupported fusion profile version {version}; this build writes {FUSION_PROFILE_VERSION}"
            )));
        }
        let decay_tag = reader
            .u8()
            .map_err(|error| FusionError::MalformedProfile(error.to_string()))?;
        let k = reader
            .u32()
            .map_err(|error| FusionError::MalformedProfile(error.to_string()))?;
        let Some(decay) = DecayRule::from_tag(decay_tag, k) else {
            return Err(FusionError::MalformedProfile(format!(
                "unknown decay rule tag {decay_tag}"
            )));
        };
        let count = reader
            .count()
            .map_err(|error| FusionError::MalformedProfile(error.to_string()))?;
        let mut weights = BTreeMap::new();
        for _ in 0..count {
            let text = reader
                .string("fusion profile stratum")
                .map_err(|error| FusionError::MalformedProfile(error.to_string()))?;
            let stratum = Iri::parse(&text)
                .map_err(|error| FusionError::MalformedProfile(error.to_string()))?;
            let raw = reader
                .i128()
                .map_err(|error| FusionError::MalformedProfile(error.to_string()))?;
            if weights.insert(stratum, Fixed::from_raw(raw)).is_some() {
                return Err(FusionError::MalformedProfile(
                    "fusion profile repeats a stratum".to_owned(),
                ));
            }
        }
        let tie_break_tag = reader
            .u8()
            .map_err(|error| FusionError::MalformedProfile(error.to_string()))?;
        if tie_break_tag != TIE_BREAK_SCORE_DESC_STRATUM_RANK_ASC_CANONICAL_TERM {
            return Err(FusionError::MalformedProfile(format!(
                "unknown tie-break tag {tie_break_tag}"
            )));
        }
        let max_contributions = reader
            .u32()
            .map_err(|error| FusionError::MalformedProfile(error.to_string()))?;
        // The field is derived from the weight count at construction, so bytes
        // that disagree with their own stratum count describe a profile this
        // build cannot build. Decoding them anyway would hand back a value
        // whose canonical bytes are not the bytes it was decoded from.
        let declared_strata = u32::try_from(weights.len())
            .map_err(|_| FusionError::MalformedProfile("too many strata".to_owned()))?;
        if max_contributions != declared_strata {
            return Err(FusionError::MalformedProfile(format!(
                "profile states a contribution maximum of {max_contributions} over \
                 {declared_strata} strata; a candidate may surface at most once per stratum"
            )));
        }
        let scale = reader
            .u32()
            .map_err(|error| FusionError::MalformedProfile(error.to_string()))?;
        if scale != SCALE_DIGITS {
            return Err(FusionError::MalformedProfile(format!(
                "profile scale {scale} is not this build's {SCALE_DIGITS}"
            )));
        }
        let rounding = reader
            .u8()
            .map_err(|error| FusionError::MalformedProfile(error.to_string()))?;
        if rounding != ROUNDING_TRUNCATE_TOWARD_ZERO {
            return Err(FusionError::MalformedProfile(format!(
                "unknown rounding tag {rounding}"
            )));
        }
        let item_encoding = reader
            .u8()
            .map_err(|error| FusionError::MalformedProfile(error.to_string()))?;
        if item_encoding != ITEM_ENCODING_CANONICAL_LEXICAL {
            return Err(FusionError::MalformedProfile(format!(
                "unknown item-encoding tag {item_encoding}"
            )));
        }
        reader
            .finish()
            .map_err(|error| FusionError::MalformedProfile(error.to_string()))?;

        Self::with_decay(weights, decay)
    }

    /// The profile's content identity.
    #[must_use]
    pub fn id(&self) -> FusionProfileId {
        FusionProfileId::from_canonical(&self.canonical_bytes())
    }

    /// The largest score the profile admits: the largest weight, times the
    /// number of strata, checked against the fixed-point ceiling.
    ///
    /// This is the true bound rather than a declared one. A candidate receives
    /// at most one contribution per stratum and each is at most that stratum's
    /// own weight, so no legitimate sum can pass `max_weight × strata`.
    fn compute_ceiling(
        weights: &BTreeMap<Iri, Fixed>,
        max_contributions: u32,
    ) -> Result<Fixed, FusionError> {
        let max_weight = weights
            .values()
            .copied()
            .max()
            .ok_or(FusionError::EmptyWeights)?;
        let bound = max_weight
            .into_raw()
            .checked_mul(i128::from(max_contributions))
            .ok_or(FusionError::Overflow)?;
        Ok(Fixed::from_raw(bound))
    }
}
