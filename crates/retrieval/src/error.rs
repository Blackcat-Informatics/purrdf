// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Typed, structured failures of the retrieval-plan layer.
//!
//! Every failure carries the dimension that failed, so admission and decode
//! paths can name an exact reason rather than report a generic parse problem.
//! A version mismatch is its own variant precisely because a plan written by a
//! different build must be refused loudly, never silently reinterpreted under
//! the current layout.

/// A failure to plan a request, validate an IRI, or encode/decode a canonical
/// [`Plan`].
///
/// The planning variants are refusals with a named dimension — no producer
/// reaches the request, a request term is malformed, a stratum has no finite
/// depth, or a registered relation's declaration refused to be read — rather
/// than a plausible plan built over a fallback.
///
/// [`Plan`]: crate::Plan
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PlanError {
    /// A plan's canonical encoding carries a version this build does not write.
    ///
    /// This is a refusal, not a warning: a plan whose layout a newer or older
    /// build defined cannot be decoded under this build's field order.
    #[error("unsupported plan version {found}; this build writes version {expected}")]
    VersionMismatch {
        /// The version found in the encoded plan.
        found: u16,
        /// The version this build writes and understands.
        expected: u16,
    },

    /// No registered producer accepts any term of the request.
    ///
    /// A request that reaches nothing is refused rather than answered by an
    /// empty plan: an empty plan is indistinguishable from a registry that
    /// needs no producers, and the caller asked for an answer.
    #[error("no registered producer accepts any term of the request")]
    NoApplicableProducers,

    /// A request term is malformed and cannot be planned.
    #[error("invalid request term: {reason}")]
    InvalidRequestTerm {
        /// The offending term, carried whole so the caller can name it. Boxed so
        /// recording it does not inflate every `Result<_, PlanError>`.
        term: Box<crate::request::RequestTerm>,
        /// Why the term cannot be planned.
        reason: String,
    },

    /// A stratum has no finite depth and no statistic supplies one.
    ///
    /// Raised when every selected producer in the stratum declares a genuinely
    /// unbounded row count ([`u64::MAX`](u64::MAX)) and statistics report no
    /// cardinality to bound it. Recording `u32::MAX` would claim a bound no
    /// producer declared, so the plan is refused instead.
    #[error("no cardinality is available for {predicate}")]
    StatisticsUnavailable {
        /// The stratum whose depth could not be bounded. Boxed so recording it
        /// does not inflate every `Result<_, PlanError>`.
        predicate: Box<crate::iri::Iri>,
    },

    /// A registered relation's declaration could not be read while planning.
    ///
    /// Every declaration read is panic-contained by the seam, so this reports a
    /// host relation whose own declaration panicked — a loud refusal rather than
    /// a producer silently dropped from the plan.
    #[error("registry declaration failed while planning: {message}")]
    RegistryDeclaration {
        /// The contained declaration failure, rendered.
        message: String,
    },

    /// The canonical encoding ended before a complete value was read.
    #[error("plan canonical encoding is truncated at byte {offset}")]
    Truncated {
        /// The byte offset at which more data was needed.
        offset: usize,
    },

    /// A canonical discriminator byte named no known variant.
    #[error("plan canonical encoding carries invalid tag {tag} for {what}")]
    InvalidTag {
        /// The field being decoded when the tag was read.
        what: &'static str,
        /// The tag byte that named no variant.
        tag: u8,
    },

    /// A canonical variable-length field was not valid UTF-8.
    #[error("plan canonical encoding carries invalid UTF-8 in {what}")]
    InvalidUtf8 {
        /// The field being decoded.
        what: &'static str,
    },

    /// A canonical field did not parse as an IRI.
    #[error("invalid IRI {text:?}: {source}")]
    InvalidIri {
        /// The offending IRI text.
        text: String,
        /// The kernel parser's refusal.
        #[source]
        source: purrdf_core::IriError,
    },

    /// The canonical encoding held bytes after the last field.
    #[error("plan canonical encoding has {extra} trailing byte(s)")]
    TrailingBytes {
        /// The number of unconsumed bytes.
        extra: usize,
    },
}

/// A failure raised while building a profile or fusing ranked streams.
///
/// Every variant is a refusal, never a repaired or partial answer: an invalid
/// profile is rejected where it is supplied, a malformed stream is returned to
/// its producer, and an arithmetic intermediate that does not fit is reported
/// rather than wrapped.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FusionError {
    /// A checked arithmetic step left the range it is computed in: a
    /// fixed-point addition, the profile's derived ceiling
    /// (`max_weight × stratum count`), or a stratum count too large for the
    /// `u32` the canonical profile encoding writes. A wrapped value would be a
    /// wrong order presented as a right one, so it is refused.
    #[error("fusion overflowed the fixed-point range")]
    Overflow,

    /// A ranked stream violated the input protocol.
    #[error("ranked-stream protocol violation: {0}")]
    Protocol(Box<crate::ranked_stream::ProtocolError>),

    /// The reciprocal-rank smoothing constant `K` was zero. `K >= 1` keeps a
    /// rank-one item's contribution finite and below one.
    #[error("fusion profile K must be at least 1, got {k}")]
    InvalidK {
        /// The rejected value.
        k: u32,
    },

    /// A contribution was asked for at rank zero, or a depth of zero was asked
    /// to be reached.
    ///
    /// Ranks are 1-based, and a depth is a count of them read from rank one, so
    /// zero names no rank under either reading. The two share this variant
    /// because they are the same fact about the same 1-based axis; a depth-zero
    /// request is rejected rather than answered vacuously, which is what
    /// returning the lightest weight there is would be.
    #[error("rank must be at least 1, got {rank}")]
    InvalidRank {
        /// The rejected rank, or the rejected depth expressed as the rank it
        /// would have to reach.
        rank: u64,
    },

    /// A fusion profile carried no stratum weights.
    #[error("fusion profile must declare at least one stratum weight")]
    EmptyWeights,

    /// A stratum weight was not strictly positive.
    #[error("stratum {stratum} has non-positive weight {weight:?}")]
    NonPositiveWeight {
        /// The stratum whose weight was rejected, as its canonical IRI text.
        stratum: String,
        /// The rejected weight.
        weight: purrdf_text::Fixed,
    },

    /// A stream emitted under a stratum the profile declares no weight for.
    #[error("fusion profile declares no weight for stratum {stratum}")]
    UnknownStratum {
        /// The undeclared stratum, as its canonical IRI text.
        stratum: String,
    },

    /// Two streams were tagged with the same stratum.
    #[error("two streams are tagged with stratum {stratum}")]
    DuplicateStratum {
        /// The repeated stratum, as its canonical IRI text.
        stratum: String,
    },

    /// The streams of one fusion do not descend from the same pinned plan.
    ///
    /// A fused answer names **one** plan, so streams that name two cannot be
    /// fused under a single trailer: whichever identity the answer carried
    /// would be right about some of its rows and wrong about the others. The
    /// refusal is narrow on purpose — streams that all name the same plan fuse,
    /// and streams that name no plan at all fuse too, producing an answer that
    /// simply names no plan. Only a disagreement is refused.
    #[error("fused streams descend from different pinned plans: expected {expected}, got {got:?}")]
    PlanIdMismatch {
        /// The plan the fusion's other streams name.
        expected: crate::id::PlanId,
        /// The plan this stream names, or `None` when a stream that should have
        /// named one named nothing.
        got: Option<crate::id::PlanId>,
    },

    /// A fusion profile's canonical bytes could not be decoded.
    #[error("malformed fusion profile: {0}")]
    MalformedProfile(String),

    /// A candidate received more contributions than there are strata.
    ///
    /// This is an **invariant violation, not a policy refusal**. A profile's
    /// contribution maximum is its stratum count
    /// ([`FusionProfile::max_contributions`](crate::FusionProfile::max_contributions)),
    /// derived rather than declared, and a candidate may surface at most once
    /// per stratum — so reaching `max + 1` means one of two things happened:
    /// a stream emitted the same candidate twice under a stratum whose
    /// per-stream uniqueness check did not see it, or the stream set handed to
    /// [`FusionStream::new`](crate::FusionStream::new) tagged two streams with
    /// the same stratum. [`fuse`](crate::fuse) refuses the second before a row
    /// is read ([`DuplicateStratum`](Self::DuplicateStratum)), so this arrives
    /// only from a hand-built fusion.
    ///
    /// It is not something a corpus can provoke. A candidate that surfaces in
    /// *every* stratum is the fusion working, not a bound being crossed, and
    /// this error can never name it.
    ///
    /// Checked immediately after the contribution that crossed the bound is
    /// recorded, so `count` is exactly `max + 1`, never a later, larger tally,
    /// and the offending candidate is named because "some candidate" is not a
    /// report anybody can act on.
    #[error(
        "candidate {item} received {count} contributions across {max} strata; a candidate may surface at most once per stratum"
    )]
    MaxContributionsExceeded {
        /// The candidate that exceeded the bound, as its canonical term text.
        item: String,
        /// The contribution count reached.
        count: u32,
        /// The profile's stratum count, which is its contribution maximum.
        max: u32,
    },

    /// A candidate's accumulated score exceeded the profile's declared
    /// ceiling.
    ///
    /// Checked immediately after the contribution that crossed the bound is
    /// summed into the candidate's lower bound, so `score` is the exact sum
    /// that first left the admitted range.
    #[error(
        "candidate {item} reached score {score:?}, exceeding the fusion profile's declared ceiling of {ceiling:?}"
    )]
    CeilingExceeded {
        /// The candidate that exceeded the bound, as its canonical term text.
        item: String,
        /// The accumulated score reached.
        score: purrdf_text::Fixed,
        /// The profile's declared ceiling.
        ceiling: purrdf_text::Fixed,
    },

    /// No weight at all reaches the requested depth under this decay rule.
    ///
    /// This is a property of the rule's arithmetic, not a budget or a policy,
    /// and it is exact rather than conservative.
    /// [`DecayRule::ReciprocalRank`](crate::DecayRule::ReciprocalRank) truncates
    /// the reciprocal *before* applying the weight, so once `trunc(S / D)` and
    /// `trunc(S / (D + 1))` are equal the two ranks have already merged at the
    /// point the weight arrives and no weight can part them again. A caller that
    /// needs depth past `saturates_at` names
    /// [`DecayRule::WeightedReciprocalRank`](crate::DecayRule::WeightedReciprocalRank),
    /// which folds the weight into the numerator and whose reachable depth does
    /// grow with it.
    ///
    /// Reaching this is not an error in the answer: a profile read past its
    /// separating range still produces a correct, deterministic result at a
    /// coarser rank resolution. It is only a refusal to claim a depth the
    /// arithmetic cannot deliver.
    ///
    /// `saturates_at` is always a measured rank and never a saturating case
    /// standing in for one: the rule was observed to stop separating, so there
    /// is an exact rank to report, and it is strictly below the depth asked
    /// for — that is what makes this refusal different from the one below. A
    /// depth that no *plan* can carry is a different fact and carries a
    /// different variant ([`DepthBeyondPlanRange`](Self::DepthBeyondPlanRange)),
    /// because nothing about the rule's arithmetic failed there.
    #[error(
        "no weight separates ranks to depth {depth} under this decay rule; it separates to depth {saturates_at} and no further"
    )]
    DepthUnreachable {
        /// The depth that was asked for.
        depth: u64,
        /// The exact 1-based depth this rule separates every adjacent pair
        /// within, at any weight. The pair immediately past it carries one
        /// contribution whatever weight is applied.
        saturates_at: u64,
    },

    /// The requested depth is deeper than a plan is able to record.
    ///
    /// A plan carries a per-stratum depth as a `u32`, so `limit` is the deepest
    /// depth anything downstream can name. This is a limit of that encoding and
    /// says nothing about the decay rule: under
    /// [`DecayRule::WeightedReciprocalRank`](crate::DecayRule::WeightedReciprocalRank)
    /// the arithmetic separates ranks well past this point given a heavy enough
    /// weight, and a weight quoted for such a depth would buy a depth no plan
    /// could ever ask for. It is therefore refused as a range, never reported as
    /// the rule saturating ([`DepthUnreachable`](Self::DepthUnreachable)).
    #[error(
        "depth {depth} is deeper than a plan can record; a plan carries a per-stratum depth as a 32-bit rank, so {limit} is the deepest expressible depth"
    )]
    DepthBeyondPlanRange {
        /// The depth that was asked for.
        depth: u64,
        /// The deepest depth a plan's 32-bit depth field can carry.
        limit: u64,
    },
}

impl From<crate::ranked_stream::ProtocolError> for FusionError {
    /// Wrap a producer's protocol violation without growing `FusionError` to the
    /// producer error's own size.
    fn from(error: crate::ranked_stream::ProtocolError) -> Self {
        Self::Protocol(Box::new(error))
    }
}
