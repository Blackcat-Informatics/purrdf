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
/// depth, the request asks for more rows than a read can be planned for, or a
/// registered relation's declaration refused to be read — rather than a plausible
/// plan built over a fallback.
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

    /// The request's own row bound is a number no read this layer plans can reach.
    ///
    /// A [`ReadBound::Bounded`](crate::ReadBound::Bounded) states how many fused
    /// rows the answer is for, and where the producers' declarations license it
    /// that number *is* each stratum's depth (see [`plan`](crate::plan)). A depth
    /// is a 32-bit rank and a unit is emitted one row deeper than its depth, so the
    /// deepest depth that can be *read* is one shallower than the deepest a plan
    /// can express. A bound above that names a read no registry could serve.
    ///
    /// `ceiling` is therefore the deepest readable depth and not `u32::MAX`, and
    /// the distinction is the whole of this refusal's honesty: a bound of exactly
    /// `ceiling` plans, records that depth, and is emitted with the probe row one
    /// past it, while `u32::MAX` would leave the emitted bound no room for that row
    /// and the read would be reported exhausted however many rows the producer
    /// still held. Naming `u32::MAX` here would have stated a ceiling this layer
    /// cannot actually serve a read at.
    ///
    /// Refused at the request rather than where it happens to bind, because the
    /// alternative is a refusal that depends on which registry the request reaches
    /// — served silently wherever some declaration was smaller, and refused
    /// wherever it was not. It is the caller's own number, which is what separates
    /// it from a *declared* row bound past the same ceiling: that one is a fact
    /// about a producer's data rather than a request for rows, so it is recorded at
    /// the ceiling and the read's ending reports that the planned depth stopped it
    /// (see [`plan`](crate::plan)).
    #[error(
        "the request bounds the answer at {requested} fused rows, and no read can be planned that \
         deep: where the producers' declarations license it that bound is each stratum's own \
         depth, a depth is a 32-bit rank, and a read is emitted one row deeper than its depth — so \
         {ceiling} is the largest bound this layer can plan a read for, and a bound of exactly \
         that is served"
    )]
    ReadBoundBeyondDepthRange {
        /// The bound the request stated.
        requested: usize,
        /// The largest bound a read can be planned for: the deepest depth whose
        /// emitted bound can still carry the probe row.
        ceiling: u64,
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

    /// A producer that placement accepted declares no row bound at the mode it
    /// will be invoked under.
    ///
    /// Unreachable against a registry that did not move under the plan, and
    /// refused rather than defaulted for exactly that reason.
    /// Placement admits an invocation only when some declared mode subsumes it,
    /// and the declared row bound is read by filtering on that same predicate —
    /// so a placement that succeeded has already proved the filter is non-empty.
    ///
    /// The alternative was a default, and every available default is a lie about
    /// a number the depth is derived from: zero declares an empty relation and
    /// floors the read to a single probing row, while
    /// [`u64::MAX`] declares an unbounded one. A missing declaration can refuse
    /// nothing and a zero is a measurement, so the absence is reported here
    /// rather than resolved into either.
    #[error(
        "producer {producer} was placed on stratum {stratum} but declares no row bound at the mode it is invoked under; the registry moved under the plan"
    )]
    UndeclaredRowBound {
        /// The stratum whose declaration went missing.
        stratum: String,
        /// The producer that was placed without one.
        producer: String,
    },

    /// A statistics snapshot named one subject twice.
    ///
    /// The snapshot is a record of what a provider reported about a subject, so
    /// two rows for one subject are two answers to one question with nothing
    /// saying which was used. The encoding sorts by subject, so a duplicate is
    /// also the one shape under which sorting does not make the bytes a pure
    /// function of the entries.
    #[error("statistics snapshot names subject {subject} more than once")]
    DuplicateStatisticsSubject {
        /// The repeated subject, as its recorded text.
        subject: String,
    },

    /// A recorded depth is not the depth its own recorded inputs derive.
    ///
    /// Raised only by [`Plan::certify`](crate::Plan::certify). A plan records
    /// every input its depths were derived from precisely so this is a checkable
    /// claim rather than an asserted one; a mismatch means the plan was edited,
    /// forged, or written by a build whose arithmetic differed, and in all three
    /// cases the depth beside the inputs describes a read the inputs do not
    /// license.
    #[error(
        "stratum {stratum} records depth {recorded}, but its recorded inputs derive depth {derived}"
    )]
    DepthNotDerivable {
        /// The stratum whose depth does not follow from its inputs.
        stratum: String,
        /// The depth the plan records.
        recorded: u32,
        /// The depth the plan's own recorded inputs derive.
        derived: u32,
    },

    /// A stratum carries a recorded depth with no recorded derivation.
    ///
    /// An unrecorded input cannot be checked, so a depth without its inputs is
    /// exactly the unverifiable claim the derivation record exists to abolish.
    #[error("stratum {stratum} records a depth with no recorded derivation")]
    DepthWithoutDerivation {
        /// The stratum whose derivation is missing.
        stratum: String,
    },

    /// A stratum carries a recorded derivation with no recorded depth.
    ///
    /// The mirror of [`DepthWithoutDerivation`](Self::DepthWithoutDerivation),
    /// and refused separately because it is a different edit: inputs for a
    /// stratum the plan does not read at all.
    #[error("stratum {stratum} records a derivation with no recorded depth")]
    DerivationWithoutDepth {
        /// The stratum whose depth is missing.
        stratum: String,
    },

    /// A stratum a depth was derived for is named by no statistics-snapshot row.
    ///
    /// The snapshot's whole claim is that it names every subject planning
    /// consulted, and a stratum is the subject planning consulted in order to
    /// *decide*. A snapshot missing one is a plan asserting it consulted nothing
    /// for a depth it recorded — so the omission is refused rather than repaired
    /// from the derivation, which would let the plan's two records drift apart
    /// silently in exactly the direction this check exists to catch.
    #[error("stratum {stratum} records a derivation with no statistics snapshot entry")]
    DerivationWithoutStatisticsEntry {
        /// The stratum the snapshot does not name.
        stratum: String,
    },

    /// A stratum's statistics-snapshot row contradicts its own recorded
    /// derivation.
    ///
    /// The row is written as a projection of the derivation, so the two agree by
    /// construction in any plan this build emitted. A plan in which they differ
    /// was edited or forged, and the two readings license different depths with
    /// nothing saying which is the measurement — so it is refused, naming the
    /// dimension that disagreed and both of its values.
    #[error(
        "stratum {stratum} records {dimension} {snapshot} in its statistics snapshot and {derivation} in its derivation"
    )]
    StatisticsEntryContradictsDerivation {
        /// The stratum whose two records disagree.
        stratum: String,
        /// Which statistic they disagree about.
        dimension: StatisticsDimension,
        /// What the snapshot entry records for that dimension.
        snapshot: String,
        /// What the derivation records for it.
        derivation: String,
    },
}

/// Which statistic a plan's two records of one stratum disagree about.
///
/// Carried by
/// [`PlanError::StatisticsEntryContradictsDerivation`](PlanError::StatisticsEntryContradictsDerivation)
/// so the refusal names a dimension rather than reporting that "the statistics"
/// differ: the three are measured separately, edited separately, and lead to
/// different depths, and a caller repairing a plan needs to know which one moved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StatisticsDimension {
    /// The row count the provider reported for the subject.
    Cardinality,
    /// The aggregate selectivity, in parts per million, that was applied.
    SelectivityPpm,
    /// The request-term indices that aggregate was summed over.
    SelectivityTerms,
}

impl core::fmt::Display for StatisticsDimension {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let name = match self {
            Self::Cardinality => "cardinality",
            Self::SelectivityPpm => "selectivity in parts per million",
            Self::SelectivityTerms => "selectivity terms",
        };
        formatter.write_str(name)
    }
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

    /// A class-width tolerance of zero was offered as a tolerance.
    ///
    /// An indifference class always contains its own rank, so the narrowest
    /// class that exists is one rank wide and a tolerance of zero admits no
    /// class at all. It is the same shape as a smoothing constant of zero —
    /// a question with no evaluable content — and it is refused for the same
    /// reason rather than quietly read as one, which would answer the most
    /// favourable question available instead of the one that was asked.
    ///
    /// It does **not** share [`InvalidRank`](Self::InvalidRank), even though
    /// both are "this count may not be zero". That variant is about the 1-based
    /// rank axis, and its message says `rank must be at least 1`; a tolerance
    /// is a width measured *across* that axis rather than a position on it, and
    /// reporting a rejected tolerance as a rejected rank would send a caller to
    /// inspect an argument that was never at fault.
    #[error(
        "class-width tolerance must be at least 1, got {max_width}; a tolerance of zero is not a tolerance, because a class always contains its own rank"
    )]
    InvalidWidth {
        /// The rejected tolerance.
        max_width: u64,
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

    /// A stream's depth was planned for one row bound and the fusion was run at
    /// another.
    ///
    /// # Why this is a sibling of [`PlanIdMismatch`](Self::PlanIdMismatch) and not
    /// a case of it
    ///
    /// Both are refusals read off the streams before a row is pulled, and both
    /// exist because a provenance that travels with the rows is the only one worth
    /// having. They are different facts with different repairs, though.
    /// `PlanIdMismatch` is a disagreement *among the streams*: two of them descend
    /// from different plans, and the repair is to stop mixing them. This one is a
    /// disagreement between the streams — which may agree perfectly — and the
    /// caller's own `top_k` argument, and the repair is either to pass the bound the
    /// plan was built for or to plan again at the bound that is wanted. Folding it
    /// into the other variant would report a provenance conflict for a call whose
    /// provenance is consistent, and would send a reader to inspect the streams
    /// instead of the argument.
    ///
    /// # Why a smaller bound is refused too
    ///
    /// A `top_k` above what was planned cannot be served: under declarations that
    /// let the planner narrow a depth to the planned bound, the rows past it were
    /// never read. A `top_k` below it would answer correctly, but out of a read
    /// deeper than the question needed — and the depths the bundle records, the
    /// resolution it reports and the identity it carries would all describe a
    /// different request. One rule, in both directions, keeps the bound the plan
    /// recorded and the bound the answer was assembled under the same number.
    #[error(
        "streams were planned for a bound of {planned} fused rows and the fusion was run at {requested}; a depth derived for one bound does not serve another"
    )]
    ReadBoundMismatch {
        /// The bound the streams' plan was built for.
        planned: crate::fuse::TopK,
        /// The bound this fusion was asked to run at.
        requested: crate::fuse::TopK,
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
    /// `saturates_at` is a **maximum over every weight**, not a reading taken at
    /// one of them. No weight was named in the question and none is named in the
    /// answer: the depth is quoted for the best weight there is, which under
    /// [`DecayRule::ReciprocalRank`](crate::DecayRule::ReciprocalRank) is any
    /// weight at or above one — below one the weight binds and the separating
    /// depth is shorter, at and above one the inner truncation binds instead and
    /// every such weight shares the same depth. Reading it as a property of some
    /// particular weight would invite the repair that does not exist, which is to
    /// raise that weight.
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
        /// The deepest 1-based depth **any** weight separates every adjacent
        /// pair within under this rule: the maximum of the per-weight
        /// separating depths, not the depth of some one weight. Exact rather
        /// than conservative, and the pair immediately past it carries one
        /// contribution whatever weight is applied — so no heavier weight
        /// reaches further, which is why the remedy is the other decay rule and
        /// never a larger number here.
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
