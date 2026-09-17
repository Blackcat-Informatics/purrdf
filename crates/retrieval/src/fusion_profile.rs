// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The fusion profile: the identity-bearing law a fusion runs under.
//!
//! A profile is not a bag of knobs. It fixes the decay rule and its smoothing
//! constant, every stratum's weight, the declared total tie-break, and the
//! admitted maximum number of contributions a candidate may receive. Two
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
use crate::reciprocal_rank::monotone_depth;

// Canonical discriminators. One tag space per enum, never reused.
const DECAY_RECIPROCAL_RANK: u8 = 0;
const TIE_BREAK_SCORE_DESC_STRATUM_RANK_ASC_CANONICAL_TERM: u8 = 0;
const ROUNDING_TRUNCATE_TOWARD_ZERO: u8 = 0;
const ITEM_ENCODING_CANONICAL_LEXICAL: u8 = 0;

/// How a candidate's contribution decays with its rank within a stratum.
///
/// The enum is closed and today holds one variant. Further monotone decay rules
/// are additive variants, and because the rule is part of the profile's
/// canonical bytes, adding one never re-opens a previously-issued identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DecayRule {
    /// `reciprocal_rank(K + rank)`, with `K >= 1`.
    ReciprocalRank {
        /// The smoothing constant `K`.
        k: u32,
    },
}

impl DecayRule {
    /// The smoothing constant this rule carries.
    #[must_use]
    pub const fn k(self) -> u32 {
        match self {
            Self::ReciprocalRank { k } => k,
        }
    }

    /// The canonical discriminator byte for this rule.
    fn tag(self) -> u8 {
        match self {
            Self::ReciprocalRank { .. } => DECAY_RECIPROCAL_RANK,
        }
    }
}

/// The declared total order applied to candidates with equal fused scores.
///
/// The enum is closed and holds the one law this layer ships: fused score
/// descending, then the candidate's best (minimum) stratum rank ascending, then
/// canonical term byte order ascending. The final key is total because two
/// distinct candidates never share a canonical term.
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
/// non-positive weight map, a zero contribution ceiling, and a maximum admitted
/// fused score that leaves the fixed-point range. Once constructed, the profile
/// is immutable and its [`FusionProfile::id`] names it.
///
/// # Weight, scale and depth are one coupled quantity
///
/// Three declared quantities decide how deep a stratum's ranks stay *ordered*:
/// the stratum's **weight** `w`, the fixed-point **scale** `S = 10^SCALE_DIGITS`
/// the whole layer computes in, and the **per-stratum depth** the plan records.
/// A contribution at 1-based rank `r` is formed as
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
/// roughly `(K + r)² ≲ w · S`. That guarantee is only sufficient, never
/// necessary — two contributions often differ when it fails, because the outer
/// truncations can straddle an integer — so the exact first-collision rank is
/// what [`FusionProfile::monotone_depth`] reports, and the refusal at admission
/// is against that exact value rather than against a conservative closed form
/// that would refuse legitimate depths.
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
    /// The maximum number of contributions one candidate may receive. This is
    /// the number of strata the profile admits a candidate may surface in.
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
    /// Build a profile from explicit weights, smoothing constant and maximum
    /// contribution count, with the canonical tie-break.
    ///
    /// # Errors
    ///
    /// * [`FusionError::InvalidK`] when `k == 0`.
    /// * [`FusionError::InvalidMaxContributions`] when `max_contributions == 0`.
    /// * [`FusionError::EmptyWeights`] when no stratum is declared.
    /// * [`FusionError::NonPositiveWeight`] for any weight `<= 0`.
    /// * [`FusionError::Overflow`] when the admitted maximum fused score does
    ///   not fit the fixed-point range.
    pub fn new(
        weights: BTreeMap<Iri, Fixed>,
        k: u32,
        max_contributions: u32,
    ) -> Result<Self, FusionError> {
        if k == 0 {
            return Err(FusionError::InvalidK { k });
        }
        if max_contributions == 0 {
            return Err(FusionError::InvalidMaxContributions {
                max: max_contributions,
            });
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

        let ceiling = Self::compute_ceiling(&weights, max_contributions)?;
        // Derived once, here, rather than on every admission: a profile is
        // immutable and is routinely reused across many searches, and the search
        // is bounded but not free.
        let monotone_depths = weights
            .iter()
            .map(|(stratum, weight)| (stratum.clone(), monotone_depth(*weight, k)))
            .collect();
        Ok(Self {
            decay: DecayRule::ReciprocalRank { k },
            weights,
            tie_break: TieBreak::default(),
            max_contributions,
            ceiling,
            monotone_depths,
        })
    }

    /// Replace the declared total tie-break. The default is the canonical law;
    /// this exists so a future additive tie-break can be selected deliberately.
    #[must_use]
    pub fn with_tie_break(mut self, tie_break: TieBreak) -> Self {
        self.tie_break = tie_break;
        self
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

    /// The maximum number of contributions one candidate may receive.
    #[must_use]
    pub const fn max_contributions(&self) -> u32 {
        self.max_contributions
    }

    /// The admitted maximum fused score.
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
    /// type's own documentation for the coupling it reports on, and
    /// [`AdmissionError::DepthBeyondMonotoneRange`](crate::AdmissionError::DepthBeyondMonotoneRange)
    /// for where a plan is held to it.
    ///
    /// `None` is the honest answer for an unweighted stratum rather than zero:
    /// a profile that says nothing about a stratum has said nothing about how
    /// deep it may be read either, and that stratum contributes nothing to a
    /// fusion under this profile.
    #[must_use]
    pub fn monotone_depth(&self, stratum: &Iri) -> Option<u64> {
        self.monotone_depths.get(stratum).copied()
    }

    /// The profile's canonical, length-framed bytes.
    ///
    /// The encoding is a pure function of the fields: weights are sorted by
    /// stratum and every integer is little-endian, so the bytes are identical on
    /// every target.
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
    /// bytes, or begins with a version this build does not write.
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
        if decay_tag != DECAY_RECIPROCAL_RANK {
            return Err(FusionError::MalformedProfile(format!(
                "unknown decay rule tag {decay_tag}"
            )));
        }
        let k = reader
            .u32()
            .map_err(|error| FusionError::MalformedProfile(error.to_string()))?;
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

        Self::new(weights, k, max_contributions)
    }

    /// The profile's content identity.
    #[must_use]
    pub fn id(&self) -> FusionProfileId {
        FusionProfileId::from_canonical(&self.canonical_bytes())
    }

    /// The largest score the profile admits: the largest weight, times the
    /// maximum contribution count, checked against the fixed-point ceiling.
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
