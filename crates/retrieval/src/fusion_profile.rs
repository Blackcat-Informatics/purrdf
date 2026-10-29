// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The fusion profile: the identity-bearing law a fusion runs under.
//!
//! A profile is not a bag of knobs. It fixes the decay rule and its smoothing
//! constant, every stratum's weight, and the declared total tie-break; the
//! maximum number of contributions a candidate may receive follows from the
//! weights, because a candidate may surface at most once per stratum. Two
//! answers fused under profiles differing in any of those are answers to
//! different questions, so the profile is content-addressed: its identity is a
//! domain-separated BLAKE3 digest over a canonical, versioned, length-framed
//! encoding, never over a JSON document or a `Hash`.

use std::collections::{BTreeMap, btree_map::Entry};

use crate::score::{Precision, Score};

use crate::canonical::{Reader, Writer};
use crate::error::{FusionError, ProfileRefusal};
use crate::id::{FUSION_PROFILE_VERSION, FusionProfileId};
use crate::iri::Iri;
use crate::reciprocal_rank::{
    ClassWidth, MonotoneDepth, ToleratedDepth, class_width, deepest_rank_within_width,
    minimum_weight_for_depth, monotone_depth,
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
/// `S = 10^precision.digits()`, truncating toward zero, and neither needs a
/// transcendental. They differ in where the weight enters, and that decides how
/// deep a stratum can be read before the arithmetic stops separating ranks:
///
/// * [`ReciprocalRank`](Self::ReciprocalRank) rounds the reciprocal first —
///   `trunc(w_raw · trunc(S / D) / S)` with `D = K + r`. The inner truncation is
///   a ceiling no weight can lift, so the monotone range ends just past
///   `D = sqrt(S)` for **every** weight at or above one.
/// * [`WeightedReciprocalRank`](Self::WeightedReciprocalRank) folds the weight
///   into the numerator — `trunc(w_raw / D)`, one exactly-rounded division. Its
///   value is never below the other's and never more than `⌊w⌋ + 1` raw units above
///   it, and its monotone range runs to roughly `sqrt(S · w)`, so a heavier
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
    /// Four refusals, and they are four different facts.
    ///
    /// [`FusionError::InvalidK`] when this rule's smoothing constant is zero.
    /// That is the rule being unusable rather than the depth being unreachable,
    /// and it is refused before `depth` is read at all — including at a depth of
    /// one, which has no adjacent pair to separate and would otherwise answer
    /// the lightest weight there is, quoting a real price under a law that
    /// cannot be evaluated. [`FusionProfile::with_decay`] refuses such a rule
    /// where a profile is declared; this is the same fact reached one level
    /// lower, and it is deliberately *not* told as the rule saturating at depth
    /// one, because the rule's arithmetic never ran.
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
    /// depth grows with the weight — so it never raises this, and that is a
    /// proof rather than an expectation: the folded rule's own arithmetic is the
    /// single division `trunc(w_raw / D)`, whose adjacent values differ by
    /// `w_raw / (D · (D + 1))`, so the weight `(K + depth)²` this search bounds
    /// itself by separates every pair in the range outright. The saturation
    /// condition the other rule meets — two ranks equal *before* the weight
    /// arrives — has no counterpart here to detect.
    ///
    /// [`FusionError::DepthBeyondPlanRange`] when `depth` is deeper than a plan
    /// can record, which a plan does as a `u32`. Of the two walls, this is the
    /// only one [`Self::WeightedReciprocalRank`] ever meets, and it is a limit of
    /// that encoding rather than of the rule: the folded arithmetic separates ranks
    /// past `u32::MAX` given a heavy enough weight, and at `u32::MAX` itself it
    /// answers. Nothing downstream could carry a deeper depth, so quoting a
    /// weight for one would price a plan that cannot be written.
    pub fn weight_for_depth(self, precision: Precision, depth: u64) -> Result<Score, FusionError> {
        minimum_weight_for_depth(self, precision, depth)
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
    /// [`ClassWidth::SpansTo`] of one means `rank` is still separated from both
    /// its neighbours by score alone. A width of `w` means `w` consecutive ranks
    /// share a contribution, so their relative order in a fused answer falls
    /// through to the declared tie-break's later keys rather than being decided
    /// by relevance. This is the resolution curve, of which
    /// [`Self::weight_for_depth`] prices a single point.
    ///
    /// [`ClassWidth::ExceedsAnyPlan`] is a class still running at the deepest
    /// rank a plan can express, which has no width to report rather than a huge
    /// one. Under this rule at a raw weight of one every contribution truncates
    /// to zero and rank one's class is the whole expressible range; at a raw
    /// weight of fifty, fifty times heavier, it still is. A number there would
    /// say those two classes are the same width.
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
    pub fn class_width(self, weight: Score, rank: u64) -> Result<ClassWidth, FusionError> {
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
    /// `max_width` of one is the separating depth itself — the deepest depth a
    /// read can stop at with every rank it **actually read** separated from
    /// both of its neighbours *within that read*. It is a depth, not a rank
    /// property, and the difference is visible one call away:
    /// [`Self::class_width`] at that rank never reports the one a "still
    /// separated from both neighbours" reading would predict, because the
    /// unbounded curve it walks also looks at the one rank the bounded read
    /// never reaches. Where it counts a width at all that width is at least
    /// `max_width + 1`, and it is exactly `max_width + 1` only where the run
    /// that ends the walk is one rank longer than the tolerance — the smooth
    /// case, not the rule. A light weight is where the difference shows: under
    /// [`Self::ReciprocalRank`] with `k` of 60 at a raw weight of `10^2`, a
    /// tolerance of one lands on depth one, whose class is forty ranks wide.
    /// Where the run reaches the end of the expressible range there is no width
    /// at all and the answer is [`ClassWidth::ExceedsAnyPlan`]. The two agree in
    /// every such case; they are answering a depth question and a rank
    /// question. Larger tolerances answer the question a caller reading deeply
    /// actually has — not "where does this stop being exact" but "how far can I
    /// read and still have ranks ordered to within the resolution I can live
    /// with".
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
    /// the caller's [`Precision`] declares — and the separating
    /// depth is about `sqrt(B)`. So the answer lands near `sqrt(max_width)`
    /// times that depth and the walk is about `sqrt(max_width) - 1` times it,
    /// one integer division per step. A `max_width` of one does not walk at
    /// all; a small tolerance costs a fraction of the separating depth; a large
    /// tolerance at a heavy weight under the folded rule — where the separating
    /// depth itself grows as `sqrt(w)` — walks very far.
    ///
    /// The walk stops at the deepest depth a plan can record, so it is bounded
    /// by fewer than `2^32` steps however it is asked and cannot fail to
    /// terminate. A walk that gets there has found no bound inside the range a
    /// plan can express, and says so as
    /// [`ToleratedDepth::ReadsBeyondAnyPlan`] rather than quoting the ceiling
    /// as a reading. It is a design-time question all the same — price a depth
    /// budget once while choosing weights — and not something to put in a hot
    /// loop.
    ///
    /// # Errors
    ///
    /// [`FusionError::InvalidK`] when this rule's smoothing constant is zero
    /// and [`FusionError::InvalidWidth`] when `max_width` is zero — a class
    /// always contains its own rank, so a tolerance of zero is not a tolerance
    /// — both refused before anything is measured, and
    /// [`FusionError::Overflow`] when a contribution leaves the fixed-point
    /// range. A refusal is reported and never rendered as a depth: the rank the
    /// walk stopped at is where the arithmetic gave out, not a depth this rule
    /// was measured to deliver, and returning it as one would quote a
    /// resolution nothing established.
    pub fn deepest_rank_within_width(
        self,
        weight: Score,
        max_width: u64,
    ) -> Result<ToleratedDepth, FusionError> {
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

/// An immutable fusion law with explicit unsigned decimal units.
///
/// Positive weights are raw values at `precision`. The admitted ceiling is the
/// exact sum of rank-one contributions, rather than a bound on the weights.
/// Every candidate receives at most one contribution per stratum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FusionProfile {
    decay: DecayRule,
    precision: Precision,
    weights: BTreeMap<Iri, Score>,
    tie_break: TieBreak,
    max_contributions: u32,
    ceiling: Score,
    monotone_depths: BTreeMap<Iri, u64>,
}

impl FusionProfile {
    /// Admit a complete fusion law, computing its exact representable ceiling.
    ///
    /// # Errors
    /// Refuses zero smoothing, empty or zero weights, mismatched precision,
    /// an unrepresentable rank-one sum, or an unrepresentable stratum count.
    pub fn with_decay(
        weights: BTreeMap<Iri, Score>,
        decay: DecayRule,
        precision: Precision,
    ) -> Result<Self, FusionError> {
        if decay.k() == 0 {
            return Err(FusionError::InvalidK { k: 0 });
        }
        if weights.is_empty() {
            return Err(FusionError::EmptyWeights);
        }
        let mut ceiling = Score::zero(precision);
        for (stratum, &weight) in &weights {
            weight.require_precision(precision)?;
            if weight.into_raw() == 0 {
                return Err(FusionError::NonPositiveWeight {
                    stratum: stratum.as_str().to_owned(),
                    weight,
                });
            }
            ceiling = ceiling.checked_add(crate::contribution_under(decay, weight, 1)?)?;
        }
        let max_contributions = u32::try_from(weights.len()).map_err(|_| FusionError::Overflow)?;
        let monotone_depths = weights
            .iter()
            .map(|(stratum, &weight)| Ok((stratum.clone(), monotone_depth(decay, weight)?)))
            .collect::<Result<_, FusionError>>()?;
        Ok(Self {
            decay,
            precision,
            weights,
            tie_break: TieBreak::default(),
            max_contributions,
            ceiling,
            monotone_depths,
        })
    }

    /// The law's smoothing constant.
    #[must_use]
    pub const fn k_parameter(&self) -> u32 {
        self.decay.k()
    }
    /// The declared decay and rounding law.
    #[must_use]
    pub const fn decay(&self) -> DecayRule {
        self.decay
    }
    /// The decimal units all of this law's values use.
    #[must_use]
    pub const fn precision(&self) -> Precision {
        self.precision
    }
    /// The declared total ordering after scores.
    #[must_use]
    pub const fn tie_break(&self) -> TieBreak {
        self.tie_break
    }
    /// At most one contribution per declared stratum.
    #[must_use]
    pub const fn max_contributions(&self) -> u32 {
        self.max_contributions
    }
    /// The exact checked sum of every stratum's rank-one contribution.
    #[must_use]
    pub const fn ceiling(&self) -> Score {
        self.ceiling
    }
    /// Weights in canonical stratum order, with their decimal units.
    #[must_use]
    pub const fn weights(&self) -> &BTreeMap<Iri, Score> {
        &self.weights
    }
    /// The declared weight, or absence for an unweighted stratum.
    #[must_use]
    pub fn weight(&self, stratum: &Iri) -> Option<Score> {
        self.weights.get(stratum).copied()
    }
    /// The exact fully separating depth, if this stratum is weighted.
    #[must_use]
    pub fn monotone_depth(&self, stratum: &Iri) -> Option<MonotoneDepth> {
        self.monotone_depths
            .get(stratum)
            .copied()
            .map(MonotoneDepth::from_rank)
    }
    /// Measure the exact indifference class at a positive rank.
    ///
    /// # Errors
    /// Carries the arithmetic's typed rank or smoothing refusal.
    pub fn class_width(&self, stratum: &Iri, rank: u64) -> Result<Option<ClassWidth>, FusionError> {
        self.weight(stratum)
            .map(|weight| class_width(self.decay, weight, rank))
            .transpose()
    }
    /// The deepest prefix whose rank classes fit the declared tolerance.
    ///
    /// # Errors
    /// Refuses a zero tolerance or an unusable arithmetic operand.
    pub fn deepest_rank_within_width(
        &self,
        stratum: &Iri,
        max_width: u64,
    ) -> Result<Option<ToleratedDepth>, FusionError> {
        self.weight(stratum)
            .map(|weight| deepest_rank_within_width(self.decay, weight, max_width))
            .transpose()
    }
    /// Versioned unsigned canonical law, including the explicitly declared units.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut writer = Writer::new();
        writer.u16(FUSION_PROFILE_VERSION);
        writer.u32(self.precision.digits());
        writer.u8(self.decay.tag());
        writer.u32(self.decay.k());
        writer.u64(self.weights.len() as u64);
        for (stratum, weight) in &self.weights {
            writer.string(stratum.as_str());
            writer.u128(weight.into_raw());
        }
        writer.u8(self.tie_break.tag());
        writer.u32(self.max_contributions);
        writer.u8(ROUNDING_TRUNCATE_TOWARD_ZERO);
        writer.u8(ITEM_ENCODING_CANONICAL_LEXICAL);
        writer.into_bytes()
    }
    /// Decode only this version's complete canonical law.
    ///
    /// # Errors
    /// Refuses obsolete versions, unsupported precision, unordered or repeated
    /// strata, invalid discriminants, inconsistent counts, and trailing bytes.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, FusionError> {
        let mut reader = Reader::new(bytes);
        let version = reader.u16()?;
        if version != FUSION_PROFILE_VERSION {
            return Err(ProfileRefusal::Version {
                found: version,
                expected: FUSION_PROFILE_VERSION,
            }
            .into());
        }
        let precision = Precision::new(reader.u32()?)?;
        let tag = reader.u8()?;
        let k = reader.u32()?;
        let decay = DecayRule::from_tag(tag, k).ok_or(ProfileRefusal::DecayTag { found: tag })?;
        let count = reader.count()?;
        let mut weights = BTreeMap::new();
        for _ in 0..count {
            let stratum = Iri::parse(&reader.string("fusion profile stratum")?)?;
            let raw = reader.u128()?;
            if let Some((previous, _)) = weights.last_key_value()
                && &stratum < previous
            {
                return Err(ProfileRefusal::StratumOrder {
                    previous: previous.clone(),
                    current: stratum,
                }
                .into());
            }
            match weights.entry(stratum) {
                Entry::Vacant(entry) => {
                    entry.insert(precision.score(raw));
                }
                Entry::Occupied(entry) => {
                    return Err(ProfileRefusal::DuplicateStratum {
                        stratum: entry.remove_entry().0,
                    }
                    .into());
                }
            }
        }
        let tie_break = reader.u8()?;
        if tie_break != TIE_BREAK_SCORE_DESC_STRATUM_RANK_ASC_CANONICAL_TERM {
            return Err(ProfileRefusal::TieBreakTag {
                found: tie_break,
                expected: TIE_BREAK_SCORE_DESC_STRATUM_RANK_ASC_CANONICAL_TERM,
            }
            .into());
        }
        let maximum = reader.u32()?;
        let strata =
            u32::try_from(weights.len()).map_err(|source| ProfileRefusal::StrataCount {
                count: weights.len(),
                limit: u32::MAX,
                source,
            })?;
        if maximum != strata {
            return Err(ProfileRefusal::Contributions {
                found: maximum,
                strata,
            }
            .into());
        }
        let rounding = reader.u8()?;
        if rounding != ROUNDING_TRUNCATE_TOWARD_ZERO {
            return Err(ProfileRefusal::RoundingTag {
                found: rounding,
                expected: ROUNDING_TRUNCATE_TOWARD_ZERO,
            }
            .into());
        }
        let encoding = reader.u8()?;
        if encoding != ITEM_ENCODING_CANONICAL_LEXICAL {
            return Err(ProfileRefusal::ItemEncodingTag {
                found: encoding,
                expected: ITEM_ENCODING_CANONICAL_LEXICAL,
            }
            .into());
        }
        reader.finish()?;
        Self::with_decay(weights, decay, precision)
    }
    /// Content identity of the entire scale-bearing law.
    #[must_use]
    pub fn id(&self) -> FusionProfileId {
        FusionProfileId::from_canonical(&self.canonical_bytes())
    }
}
