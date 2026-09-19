// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The streaming fusion engine: verified-protocol Fagin NRA over exact fixed
//! point.
//!
//! Each stream contributes a monotone, non-increasing contribution sequence to
//! the candidates it carries. A candidate's fused score is the checked sum of
//! its contributions across strata; its lower bound `L(x)` is the sum of the
//! contributions seen so far and its upper bound `U(x)` adds the current head of
//! every stream that has not yet contributed to `x`. Fusion pulls greedily from
//! the highest-contribution head, maintains the threshold `T` over all heads,
//! and emits a candidate only when its score is final — no stream that could
//! still name it is open — and it provably outranks every other candidate and
//! every not-yet-seen item. Emitted candidates are removed, so the frontier
//! holds only the un-emitted candidates and never grows with how long the
//! streams are.
//!
//! Finality is a membership question and is asked as one
//! ([`FusionStream::is_final`]); `U(x)` compares candidates and is asked
//! nothing else. The two are not interchangeable: a live head of contribution
//! zero leaves `U(x)` equal to `L(x)` while the stream carrying it can still
//! name `x`.
//!
//! *Could* still name it is where a producer's own declaration enters. A stream
//! that declared [`CandidateDomains::Within`] has promised its candidates lie in
//! named blocks of the candidate universe, so a candidate outside those blocks
//! is one it will never name — and waiting for it would be waiting forever. The
//! membership test is therefore asked of the streams that can name `x` and
//! skipped for the streams that provably cannot, which is a *smaller
//! quantifier*, not a weaker test: nothing about what a live stream owes a
//! candidate changes. A stream that then names a candidate its declaration
//! cannot reach is refused ([`ProtocolError::OutsideDeclaredDomain`]), and a
//! fusion whose streams declare [`CandidateDomains::Unrestricted`] skips
//! nothing and computes exactly what it computed before the term existed. What
//! the licence buys is the reading: without it, strata whose candidate sets do
//! not overlap are read to their ends however small the caller's top-k, because
//! no confirmation is ever coming.
//!
//! The frontier is not the whole of what a fusion holds, and saying otherwise
//! would overstate it. De-duplicating a stream that declared
//! [`DuplicatePolicy::Allowed`] needs the set of items that stream has already
//! emitted, and that set is proportional to the rows **pulled**. The frontier
//! argument bounds exactly that: nothing here ever pulls a row it was not asked
//! for. Certifying the top `k` through [`FusionStream::next`] pulls what the
//! threshold argument requires and no more, and [`FusionStream::trailer`] pulls
//! nothing at all — it reports the state each stream is already in. A caller
//! that wants every row still pays for every row, because it asked for them one
//! at a time.
//!
//! A stream that declared [`DuplicatePolicy::Unique`] is not charged for a
//! per-row identity set — there is nothing to de-duplicate, so none is built
//! for it — and that set is the one structure here that grows with the rows
//! pulled rather than with the disagreement window. Declaring uniqueness
//! truthfully is therefore what makes a deep answer affordable, and
//! `fusion_frontier_alloc` measures the difference rather than asserting it.
//!
//! # What every fusion is charged, whatever any stream declared
//!
//! One entry per row **emitted**, in [`FusionStream`]'s `emitted` map: the
//! candidate's term and the set of stream indexes that named it. That is the
//! price of the answer invariant — *a fused answer never contains the same
//! entity twice* — and it is stated here rather than left to be discovered,
//! because it is the one cost that is not a stream's own declaration to avoid.
//!
//! It is charged against **emissions, not pulls**, and the difference is the
//! whole argument:
//!
//! * A bounded call — [`fuse`](crate::fuse) under a caller's
//!   [`TopK`](crate::TopK) — emits at most `k` rows, so the map holds at most
//!   `k` entries however long the streams are. The bound that already governs
//!   the answer governs this too, with no second knob.
//! * A caller that drains gets no asymptotic surprise either, because what
//!   enters the map is what *left* the frontier, and strictly smaller: a
//!   certified candidate's contribution vector and its lower bound are dropped
//!   and only the stream-index set survives, moved rather than rebuilt. Peak
//!   live bytes therefore do not grow by a term the frontier was not already
//!   paying.
//!
//! What the map is emphatically not is the retained-forever *per-stream*
//! identity set a [`DuplicatePolicy::Unique`] declaration buys its way out of.
//! That one grows with every row a stream emits, duplicated per stream; this
//! one grows with the rows the *fusion* returns, once for all strata together.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet, HashMap, btree_map};

use purrdf_sparql_eval::{
    CandidateDomains, DomainTag, DuplicatePolicy, IndexGeneration, PfAttestation, ServiceLevel,
};
use purrdf_text::Fixed;

use crate::canonical::Writer;
use crate::error::FusionError;
use crate::fusion_profile::FusionProfile;
use crate::id::{EVIDENCE_VERSION, EvidenceId, PlanId};
use crate::iri::{Iri, Term};
use crate::ranked_stream::{ProducerReceipt, ProtocolError, RankedStream};
use crate::reciprocal_rank::MonotoneDepth;

/// A candidate's identity in the frontier: its canonical term.
pub type CandidateId = Term;

/// The status of one producer in a fused trailer.
///
/// This mirrors [`ProducerReceipt`] but belongs to the answer: every producer
/// that contributed — and every applicable one that could not — keeps its own
/// status, so "all producers answered" and "one could not" stay distinguishable.
///
/// # `Exhausted` is the only completeness claim in the vocabulary
///
/// There are five variants and exactly one of them says a stratum's rows ran
/// out. Every other one names *who stopped the read*, and they are four
/// different parties stopping it at four different places:
///
/// * [`Self::DepthReached`] — the **plan's depth** stopped the producer, at a
///   rank. The rows below it exist and were not looked at.
/// * [`Self::CeilingReached`] — a **contribution bound** stopped the read, at a
///   value in the profile's fixed-point space. Either the producer's own
///   declared bound or, far more often, the one a bounded fusion wrote down
///   when the caller's [`TopK`](crate::TopK) was satisfied.
/// * [`Self::TermsRejected`] — the **producer** stopped it before it began, by
///   declining the request terms it was handed.
/// * [`Self::ExecutionFailed`] — the **run** stopped it: the unit could not
///   execute at all.
///
/// A consumer therefore reads completeness by looking for one variant rather
/// than by eliminating the others, which is the property that makes this
/// vocabulary safe to extend: a read ending added later is incomplete by
/// construction instead of complete by omission.
///
/// The first two are the pair most easily confused, and collapsing them would
/// lose the fact a consumer acts on. Both say "this stratum is not complete",
/// but they differ in who stopped the read and in what space the stopping point
/// is expressed — a rank the producer was handed as its depth, versus a
/// contribution the producer never saw. The remedy differs with them:
/// [`Self::DepthReached`] is answered by re-planning deeper, and
/// [`Self::CeilingReached`] by certifying further rows from the same streams.
/// Neither number converts into the other — a rank becomes a contribution only
/// under a profile, and a contribution does not become a rank again once
/// fixed-point decay has quantized two adjacent ranks to one value — so a
/// consumer handed the merged fact could only guess which knob to turn.
///
/// # Where each variant comes from
///
/// Four of the five are the producer's own declaration, converted from the
/// receipt it returned through [`RankedStream::receipt`] — plus, for the strata
/// that never became a stream, the executor's report carried in by
/// [`FusionTrailer::completed_with`].
///
/// [`Self::CeilingReached`] has a second author, and only one meaning. It says
/// that reading stopped at a contribution bound rather than at the end of the
/// rows: everything at or above `bound` was read, and what lies below it was
/// not. A producer says it when it stops at a bound of its own. The fusion says
/// it when the caller's row bound stopped it reading a stream that still held
/// rows — see [`FusionStream::trailer`], which is where a bounded stop is
/// written down. Either way the claim is the same and it is falsifiable in the
/// same way: this stratum is not complete below `bound`. Draining the stream to
/// make it say [`Self::Exhausted`] instead would be both a lie about the answer
/// and the end of the memory bound that makes top-k fusion worth having.
///
/// [`Self::TermsRejected`] has exactly one author, the producer, and it is
/// deliberately not minted anywhere else. A producer that accepts none of the
/// request's terms is rejected at planning
/// ([`RejectionReason::NoAcceptedTerm`](crate::RejectionReason)) and so compiles
/// no unit to run: it has no stratum in the answer to report under, and the
/// per-term fact that nothing served a term is reported per term instead, in
/// [`Plan::unserved_terms`](crate::Plan::unserved_terms). Minting a stratum
/// status from that would report one fact twice, in two shapes that could
/// disagree. What the variant is for is the producer that *was* handed terms and
/// declined them: "I did not look" and "I looked and found nothing"
/// ([`Self::Exhausted`] with no rows) are different answers, and a caller
/// composing [`RankedStream`] producers by hand through [`fuse`](crate::fuse)
/// can say either. The fusion engine validates the claim against the rows the
/// stream actually emitted, so it cannot be used to hide them.
///
/// [`Self::DepthReached`] has exactly one author too, the producer, and it is
/// the only ending in this enum that fusion has no way of observing for itself:
/// a stream that stops at its depth and a stream that stops because it ran out
/// look identical from here — both simply stop yielding rows. So it is believed
/// where it is stated and checked where it is checkable: fusion verifies the
/// rank against the rows it actually pulled
/// ([`ProtocolError::ForgedReceipt`]) and takes the producer's word for the
/// rows it says it did not look at, which is the only party that knows.
///
/// [`RankedStreamImpl`](crate::RankedStreamImpl), this crate's own executor
/// stream, materializes its rows and declares [`Self::Exhausted`]; a unit that
/// cannot run at all becomes [`Self::ExecutionFailed`].
///
/// # What is deliberately not in this enum: an incomplete index
///
/// A producer whose index was short — a shard that failed to load, a segment
/// mid-rebuild, a replica that has not caught up — has *not* described a read
/// ending, and it gets no variant here. That fact is true of the invocation
/// from the instant it opened and stays true however the read then ends, so it
/// travels as an attestation instead
/// ([`RankedStream::attestation`], reported in
/// [`FusionTrailer::attestations`]), pinned at `open` before a row is pulled.
///
/// The reason is not taxonomy, it is survival. Terminal statuses are
/// overwritten by a bounded stop: a fusion that satisfies its
/// [`TopK`](crate::TopK) writes [`Self::CeilingReached`] over every stream it
/// stopped, and a stream it stopped never returns a receipt at all. An
/// incompleteness held as a terminal status would therefore be destroyed
/// exactly in the runs where it mattered most — the bounded ones — and the
/// answer would name the bound while silently losing the hole beneath it.
/// Pinned at open, both facts reach the trailer together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProducerStatus {
    /// The producer emitted every row it had.
    Exhausted {
        /// How many rows it emitted.
        rows_emitted: u64,
    },
    /// The producer stopped at the depth it was given, and more rows existed.
    ///
    /// The mirror of [`ProducerReceipt::DepthReached`], verified: `rank` is the
    /// last rank the producer emitted, and fusion has checked it against the
    /// rows it actually pulled. Ranks one through `rank` were read; nothing
    /// below `rank` was looked at.
    ///
    /// Authored by the producer, in **rank** space — which is what separates it
    /// from [`Self::CeilingReached`], authored (usually) by fusion in
    /// contribution space. See this type's header for why the two are not one
    /// variant.
    DepthReached {
        /// The last 1-based rank the producer emitted, equal to the number of
        /// rows fusion pulled from it.
        rank: u64,
    },
    /// Reading stopped at a contribution bound rather than at the end of the
    /// rows. Declared by the producer, or written down by a fusion the caller's
    /// row bound stopped; see this type's header.
    CeilingReached {
        /// The inclusive bound at which it stopped: every row at or above this
        /// contribution was read, and the rows below it were not.
        bound: Fixed,
    },
    /// The producer could not run.
    ExecutionFailed {
        /// A human-readable reason, carried verbatim.
        reason: String,
    },
    /// The producer declined the request terms it was handed. Declared by the
    /// producer itself; see this type's header.
    TermsRejected,
}

impl From<ProducerReceipt> for ProducerStatus {
    /// Carry a producer's own terminal declaration into the fused trailer,
    /// variant for variant.
    ///
    /// The two enums are deliberately separate types for the same five facts:
    /// a [`ProducerReceipt`] is what a producer *claims* on the input protocol,
    /// and a [`ProducerStatus`] is what the trailer *reports* after fusion has
    /// checked that claim against the rows it actually pulled. Keeping them
    /// apart is what stops an unverified claim from being mistaken for a
    /// verified one at the type level. The conversion is total and lossless, so
    /// nothing a producer said is reworded on the way through.
    ///
    /// Not every status arrives this way, and that is the other half of keeping
    /// the types apart: a stream the caller's row bound stopped never returned
    /// a receipt at all, so the fusion records its
    /// [`ProducerStatus::CeilingReached`] directly rather than mint a
    /// [`ProducerReceipt`] the producer never uttered.
    fn from(receipt: ProducerReceipt) -> Self {
        match receipt {
            ProducerReceipt::Exhausted { rows_emitted } => Self::Exhausted { rows_emitted },
            ProducerReceipt::DepthReached { rank } => Self::DepthReached { rank },
            ProducerReceipt::CeilingReached { bound } => Self::CeilingReached { bound },
            ProducerReceipt::ExecutionFailed { reason } => Self::ExecutionFailed { reason },
            ProducerReceipt::TermsRejected => Self::TermsRejected,
        }
    }
}

/// One fused row: a candidate with its fused score and its provenance.
///
/// `contributions` names, for every stratum the candidate surfaced in, the
/// 1-based rank and the contribution that stratum made. Their checked sum is
/// `score` — exact over the strata that answered, and a lower bound where one
/// of them served from an index it attested was not whole (see
/// [`FusionTrailer::exactness`]). `threshold_witness` is the global threshold
/// in force when the row was certified, so a reader can replay the
/// certification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FusedRow {
    /// The candidate, in its canonical term lexical — `<http://example.org/doc>`,
    /// `"lex"@en`, `<<( s p o )>>`.
    ///
    /// That is the same spelling
    /// [`RequestTerm::EntitySeed`](crate::RequestTerm::EntitySeed) takes, so a
    /// fused row can be handed straight back as the seed of a follow-up request.
    pub entity: Term,
    /// The fused score: the checked sum of the contributions in
    /// [`Self::contributions`], exact over the strata that answered.
    ///
    /// The arithmetic is exact and always was — every summand is a checked
    /// fixed-point value the engine re-derived itself — but arithmetic is not
    /// the whole of the claim. Whether this number is the candidate's *whole*
    /// score depends on whether every stratum that could have contributed to it
    /// had a whole index to contribute from, and that is a fact about the
    /// producers, not about the sum. A stratum serving from a short index omits
    /// the rows its missing shard held, so a candidate that shard would have
    /// named is summed without that contribution and scores lower than it
    /// should.
    ///
    /// So this field is not an unconditional exactness claim, and reading it as
    /// one is the very fault this protocol exists to prevent — a bound on the
    /// read silently becoming a value. The trailer says which reading applies:
    /// [`FusionTrailer::exactness`] is [`ScoreExactness::Exact`] when no handed
    /// stream attested an incomplete index, and
    /// [`ScoreExactness::LowerBounds`] naming the short strata otherwise, in
    /// which case every score here is a **lower bound** on the true one. This
    /// is the same refusal to overstate that the row list itself already makes:
    /// a returned row is never a completeness claim, and the trailer is where
    /// completeness is asserted.
    ///
    /// [`Self::threshold_witness`] certifies the *order* under the scores that
    /// were summed, and it stays replayable either way — it is a fact about
    /// this fusion's own arithmetic. It does not, and cannot, certify that the
    /// summands were all the summands there were.
    pub score: Fixed,
    /// Per-stratum provenance: `(stratum, rank, contribution)`.
    pub contributions: Vec<(Iri, u64, Fixed)>,
    /// The threshold in force when this row was certified.
    pub threshold_witness: Fixed,
}

/// What one stratum's rank resolution actually cost this fusion.
///
/// [`PlannedResolution`](crate::PlannedResolution) answers the same question at
/// the waist, from the depth a plan records, before anything runs. This answers
/// it from the rows that were really pulled. They differ whenever a top-k
/// certified early — and that gap is the point, because a bound a fusion never
/// reached cost it nothing.
///
/// Every field is as of the moment [`FusionStream::trailer`] was called. That
/// method does not consume the fusion, so a caller that reads a trailer and
/// keeps pulling will see these numbers grow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StratumResolution {
    /// Where this profile stops separating adjacent ranks in this stratum.
    ///
    /// A property of the law, identical to what
    /// [`FusionProfile::monotone_depth`](crate::FusionProfile::monotone_depth)
    /// reports, and independent of how deep this particular run read.
    pub separation: MonotoneDepth,
    /// The highest 1-based rank this fusion actually pulled from the stream.
    ///
    /// Named for ranks rather than depth because that is what it counts: under
    /// [`DuplicatePolicy::Allowed`] a row discarded as a repeat still advances
    /// the rank counter, so this is not the number of contributions merged.
    ///
    /// Zero means exactly one thing: this stream ended without ever emitting a
    /// row. It does not mean the stream went unread. Producing a trailer
    /// requires a terminal status per producer, which requires pulling from
    /// every stream until it yields either its first row or its receipt — so a
    /// [`TopK`](crate::TopK) of zero still reports one here for every stream
    /// that has a first row to give, and a stream that has none is reported at
    /// zero after being asked.
    ///
    /// A caller reading this as "how much did fusion spend on this stratum"
    /// therefore gets the honest answer at both ends, and one reading zero as
    /// "untouched" is wrong in the same way at both: the stream was pulled from
    /// either way, and what differs is only whether it had anything to hand
    /// back.
    pub ranks_pulled: u64,
    /// Adjacent ranks whose contributions this fusion could not tell apart.
    ///
    /// Counted by direct observation on every row, not inferred by comparing
    /// `ranks_pulled` against `separation`. A non-zero count is proof this run
    /// entered the range where the fused score stops ordering; zero is proof it
    /// did not. Nothing is wrong when it is non-zero — those ranks are ordered
    /// by the tie-break's later keys instead — but a caller that needs its
    /// answer ordered by score alone has its answer here.
    pub collisions_observed: u64,
}

/// Whether the fused scores in an answer are the scores, or floors under them.
///
/// A fused score is the checked sum of a candidate's contributions across
/// strata. The arithmetic is exact in every case; what varies is whether all
/// the summands were there to be summed. A stratum serving from an index it
/// attests was **not whole** ([`ServiceLevel::Incomplete`]) omits whatever its
/// missing shard held, so any candidate that shard would have named is summed
/// short — a real score, one contribution light.
///
/// Labelling such a score "exact" would be the fault this whole protocol
/// exists to remove: a bound on the read presented as a value. So the trailer
/// states which of the two readings applies, once, for the whole answer, and
/// the rows are still returned either way. Refusing to answer at all would be
/// the mirror error — a short index still produced real rows in a real order,
/// and a caller that knows the scores are floors can use them.
///
/// # Why the answer is per fusion rather than per row
///
/// The honest per-row answer is not computable. Knowing which *particular*
/// candidates lost a contribution would mean knowing what the missing shard
/// held, which is precisely what nobody has — the producer least of all, since
/// a shard that failed to load cannot be consulted about its contents. What is
/// knowable is the set of strata that could have contributed and could not
/// fully, and that is what [`Self::LowerBounds`] names. A per-row flag would
/// have to be either fabricated or set on every row, and the second is this
/// value spelled once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScoreExactness {
    /// Every handed stream served from an index it did not attest was short, so
    /// each score is the exact sum of every contribution that was due.
    ///
    /// Not a certificate that every index was whole. Most producers attest
    /// nothing at all ([`ServiceLevel::Undeclared`] is silence, and there is no
    /// `Whole` variant to attest), so this says the narrower true thing: no
    /// stratum in this fusion declared itself short. It is the strongest claim
    /// the seam can carry, and reading it as more would put words in the mouth
    /// of every producer that stayed silent.
    Exact,
    /// At least one handed stream attested an incomplete index, so every score
    /// in this answer is a lower bound on the score the whole index would have
    /// produced.
    ///
    /// The order is still this fusion's own certified order over the
    /// contributions it did receive, and the rows are real rows. What cannot be
    /// concluded is that a row absent from the answer would have stayed absent,
    /// or that the emitted order would have survived the missing contributions.
    LowerBounds {
        /// Exactly the strata that attested [`ServiceLevel::Incomplete`], in
        /// canonical stratum order.
        ///
        /// Naming them rather than raising a flag is what makes the fact
        /// actionable: these are the indexes to rebuild, and a caller can read
        /// each one's verbatim reason out of
        /// [`FusionTrailer::attestations`] under the same key.
        strata: BTreeSet<Iri>,
    },
}

/// The terminal report of a fusion: every producer's status, what their indexes
/// attested, and all three identities.
///
/// The trailer is the only place completeness may be asserted. A consumer that
/// read a prefix and stopped holds evidence the answer is incomplete; nothing
/// mid-stream entitles it to claim otherwise.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FusionTrailer {
    /// Every producer's own status, keyed by its stratum.
    pub statuses: BTreeMap<Iri, ProducerStatus>,
    /// What each handed stream's index attested, keyed by its stratum: which
    /// generation answered, and whether that generation was whole.
    ///
    /// Read from [`RankedStream::attestation`] in
    /// [`FusionStream::new`](FusionStream::new), **before any row is pulled**.
    /// A generation is pinned when an index is opened, so the value read there
    /// is the truth for the whole read — asking at the end would ask a stream
    /// that a bounded fusion may have stopped, and the answer would depend on
    /// how deep the caller happened to read.
    ///
    /// # Both facts survive a bounded stop
    ///
    /// This is the load-bearing consequence of reading at construction. A
    /// stratum that attests [`ServiceLevel::Incomplete`] and is then stopped by
    /// the caller's [`TopK`](crate::TopK) appears **twice** in this trailer: as
    /// [`ProducerStatus::CeilingReached`] in [`Self::statuses`], and as
    /// `Incomplete` here. Had incompleteness been a terminal receipt instead,
    /// the bounded stop would have overwritten it — a stopped stream never
    /// returns a receipt at all — and the answer would have lost the hole in
    /// the index exactly in the runs where the bound made it matter.
    ///
    /// # The key set is a subset of [`Self::statuses`]'
    ///
    /// Only the streams this fusion was handed are keyed. A stratum that never
    /// became a stream — one whose unit failed to run, added afterwards through
    /// [`FusionTrailer::completed_with`] — has no attestation to name, because
    /// no index of its was ever opened. Its absence from this map is the honest
    /// answer, and it is deliberately not filled with
    /// [`PfAttestation::UNDECLARED`]: `Undeclared` means a producer was asked
    /// and said nothing, and fabricating it here would report a producer that
    /// was never asked as one that declined to answer.
    pub attestations: BTreeMap<Iri, PfAttestation>,
    /// Whether the fused scores are exact, or floors under the scores a whole
    /// index would have produced.
    ///
    /// [`ScoreExactness::Exact`] when no handed stream attested an incomplete
    /// index, and [`ScoreExactness::LowerBounds`] naming exactly the strata
    /// that did.
    ///
    /// # As of when
    ///
    /// Like every other field here it is as of the moment
    /// [`FusionStream::trailer`] was called, but unlike [`Self::statuses`] and
    /// [`Self::resolution`] it does not move between calls, and that difference
    /// is the point. Its inputs are [`Self::attestations`], pinned before the
    /// first row was pulled; how deep a caller then read changes which streams
    /// are still open, never whether an index was whole. A caller that reads a
    /// trailer, certifies more rows and reads another gets a different set of
    /// statuses and the same exactness — which is exactly the invariance a read
    /// bound must not be able to destroy.
    pub exactness: ScoreExactness,
    /// The pinned plan the fused rows came from, when one is attached.
    pub plan_id: Option<PlanId>,
    /// The fusion profile in force.
    pub profile_id: crate::id::FusionProfileId,
    /// The evidence these rows were produced against: the content identity of
    /// [`Self::attestations`].
    ///
    /// The third of the three identities an answer carries.
    /// [`Self::plan_id`] pins the question, [`Self::profile_id`] pins the law,
    /// and this pins the index generations that answered — so one equality
    /// comparison over the triple decides whether two answers are comparable at
    /// all. See [`EvidenceId`] for why the plan and the law are not enough on
    /// their own.
    ///
    /// Derived from the attestation map alone, so it is as immovable across
    /// repeated [`FusionStream::trailer`] calls as that map is.
    pub evidence_id: EvidenceId,
    /// What each stream's rank resolution cost this fusion, keyed by stratum.
    ///
    /// Keyed by every stream this fusion was handed whose stratum the profile
    /// weights — *including* one that ended without emitting a row, which is
    /// reported with [`StratumResolution::ranks_pulled`] of zero. That is the
    /// more useful answer than omitting it: [`StratumResolution::separation`] is
    /// the resolution recorded for that stratum whether or not rows arrived, and
    /// the caller learns none arrived from the zero rather than from an absent
    /// key it would have to interpret.
    ///
    /// A stratum the profile declares no weight for is absent, because a profile
    /// silent about a stratum has said nothing about its resolution either. No
    /// such stream can have contributed anyway — the first row of one is refused
    /// as [`FusionError::UnknownStratum`](crate::FusionError::UnknownStratum),
    /// and [`fuse`](crate::fuse) refuses one before pulling at all — so this
    /// absence is reachable only by driving this type directly over a stream
    /// that had no rows to offer.
    ///
    /// The key set is a subset of [`Self::statuses`]', never the other way
    /// round. That map answers "what happened to this producer" and additionally
    /// names producers fusion never saw at all, which
    /// [`FusionTrailer::completed_with`] adds; this one answers "what did fusion
    /// read from this stream", and a producer that never became a stream was
    /// never read from.
    pub resolution: BTreeMap<Iri, StratumResolution>,
    /// The candidate-domain declaration each handed stream fused under, keyed
    /// by its stratum.
    ///
    /// Read from [`RankedStream::contract`] in
    /// [`FusionStream::new`](FusionStream::new), **before any row is pulled**,
    /// and reported verbatim. It is in the trailer because it is an input to
    /// the answer that the answer cannot otherwise be audited against: these
    /// declarations decide which streams fusion was allowed to skip when it
    /// certified a row, so a reader asking *why did this stratum stop at a
    /// bound instead of being read further* is asking about exactly this map.
    ///
    /// An answer whose every entry is
    /// [`CandidateDomains::Unrestricted`] was certified with no licence to skip
    /// anything, which is the strongest reading a fused row can have and is
    /// what every answer this engine produced before domains existed carried.
    /// An answer carrying a [`CandidateDomains::Within`] entry was certified
    /// partly on that producer's word, and the map is where a reader finds
    /// whose word it was.
    ///
    /// # The key set matches [`Self::attestations`]'
    ///
    /// Every stream this fusion was handed, and nothing else. A stratum that
    /// never became a stream — added afterwards by
    /// [`FusionTrailer::completed_with`] — declared nothing to this fusion
    /// because it was never read, and filling it with `Unrestricted` would
    /// report a producer that was never asked as one that promised to name
    /// anything.
    pub domains: BTreeMap<Iri, CandidateDomains>,
    /// Whether the last row in the answer ties on score with a *settled* rival
    /// left outside it.
    ///
    /// `true` means the top-k boundary was decided by the tie-break's later keys
    /// — best stratum rank ascending, then canonical term bytes — rather than by
    /// relevance, so a different-but-equally-scoring candidate could have taken
    /// the final place. It is the consequence of coarse resolution a caller
    /// actually feels, and neither [`StratumResolution::separation`] nor
    /// [`StratumResolution::ranks_pulled`] can reveal it.
    ///
    /// # What `false` does and does not say
    ///
    /// The scan behind this flag counts only rivals that were already final when
    /// the last row was emitted, because only a final rival has a settled score
    /// to tie with — one that can still accumulate contributions is not yet tied
    /// with anything.
    ///
    /// So from `false` a caller may conclude exactly this: **no settled rival
    /// tied with the emitted row.** That covers the cases where the answer was
    /// not bounded by the top-k at all and where nothing was excluded, and it
    /// covers a cut that fell on a strict score difference.
    ///
    /// A caller may **not** conclude from `false` that the cut was decided on a
    /// strict score difference. A rival still live at that moment could have
    /// risen to exactly the emitted row's score had fusion read further; it is
    /// not counted here, and `false` is not evidence it does not exist. `false`
    /// is therefore not proof the final place was earned on relevance.
    ///
    /// This is what the field means, not a shortfall standing in for a stronger
    /// one. Settling every live rival would require pulling at least one row
    /// past the top-k, and how deep this fusion read is itself reported — as
    /// [`StratumResolution::ranks_pulled`], in this same trailer. Reading
    /// further to sharpen this field would falsify that one, so the flag is
    /// defined over what the bounded read had already settled.
    pub cut_on_a_tie: bool,
}

// Canonical discriminators for the evidence encoding. One tag space per enum,
// never reused, so a variant added to either enum takes the next free tag and
// leaves every identity already issued exactly where it was.
const GENERATION_UNDECLARED: u8 = 0;
const GENERATION_DECLARED: u8 = 1;
const SERVICE_UNDECLARED: u8 = 0;
const SERVICE_INCOMPLETE: u8 = 1;

/// Write `attestations` as the canonical bytes an [`EvidenceId`] digests.
///
/// One encoder, reached both by [`FusionTrailer::evidence_canonical_bytes`] and
/// by the fusion that mints the identity, so the bytes an answer's id was taken
/// over are the same bytes a holder of that answer can re-derive. Two encoders
/// agreeing today is a property that decays; one encoder is a property that
/// holds.
fn evidence_canonical_bytes(attestations: &BTreeMap<Iri, PfAttestation>) -> Vec<u8> {
    let mut writer = Writer::new();
    writer.u16(EVIDENCE_VERSION);
    writer.u64(attestations.len() as u64);
    for (stratum, attestation) in attestations {
        writer.string(stratum.as_str());
        match &attestation.generation {
            IndexGeneration::Undeclared => writer.u8(GENERATION_UNDECLARED),
            IndexGeneration::Declared(generation) => {
                writer.u8(GENERATION_DECLARED);
                writer.string(generation);
            }
        }
        match &attestation.service {
            ServiceLevel::Undeclared => writer.u8(SERVICE_UNDECLARED),
            ServiceLevel::Incomplete { reason } => {
                writer.u8(SERVICE_INCOMPLETE);
                // The host's verbatim reason is part of the identity rather
                // than decoration beside it: one generation read twice, once
                // missing a shard and once behind a lagging replica, is
                // different evidence and must not digest alike.
                writer.string(reason);
            }
        }
    }
    writer.into_bytes()
}

impl FusionTrailer {
    /// The canonical bytes [`Self::evidence_id`] is the digest of.
    ///
    /// A pure function of [`Self::attestations`]: the layout version, the entry
    /// count, then every `(stratum, generation, service level)` in canonical
    /// stratum order, each variable-length part framed by its own length and
    /// every integer little-endian. Sorting comes free from the `BTreeMap` and
    /// is relied on, so no iteration order can reach the digest; the framing is
    /// what makes the encoding injective, so no two different maps can share
    /// bytes by running their fields together; the little-endian integers are
    /// what make it identical on every target, wasm32 included.
    ///
    /// Public because an identity nobody can re-derive is an identity nobody
    /// can audit. A caller holding a trailer can recompute
    /// [`EvidenceId::from_canonical`] over these bytes and confirm the id it
    /// was handed, or archive the bytes beside an answer so a later run can be
    /// compared against the evidence this one actually had — the same reason
    /// [`Plan::canonical_bytes`](crate::Plan::canonical_bytes) is public.
    #[must_use]
    pub fn evidence_canonical_bytes(&self) -> Vec<u8> {
        evidence_canonical_bytes(&self.attestations)
    }

    /// Add the status of every producer that never became a stream, and return
    /// the completed trailer.
    ///
    /// A fusion knows only the streams it was handed, and that is strictly fewer
    /// producers than a request reaches: a stratum whose unit failed to run has
    /// no stream to fuse, and one the profile declares no weight for is set
    /// aside before fusion sees it. Both are "an applicable producer that could
    /// not contribute", which §6 of the design record says the answer must carry
    /// alongside the ones that did — so the stage that *does* know them hands
    /// them here, rather than the answer reducing them to a count of strata that
    /// happened to fuse.
    ///
    /// A status this fusion verified is never replaced. Those were checked
    /// against the rows actually pulled ([`ProtocolError::ForgedReceipt`]), and
    /// an unverified report of the same stratum cannot be allowed to overwrite
    /// one that was; `statuses` fills only the strata the fusion never saw.
    ///
    /// Only [`Self::statuses`] grows. A producer that never became a stream
    /// gets no [`Self::attestations`] entry and does not change
    /// [`Self::evidence_id`], because no index of its was opened and there is
    /// nothing it attested — naming it `Undeclared` would report a producer
    /// that was never asked as one that answered with silence, and would make
    /// the evidence identity depend on a stratum that contributed no evidence.
    ///
    /// The result does not depend on `statuses`' iteration order: its keys are
    /// distinct and each is inserted only where nothing stands.
    ///
    /// This is a terminal operation on a terminal value — the trailer exists
    /// only after every stream reached its receipt — so nothing here makes a
    /// status readable mid-stream.
    #[must_use]
    pub fn completed_with<I>(mut self, statuses: I) -> Self
    where
        I: IntoIterator<Item = (Iri, ProducerStatus)>,
    {
        for (stratum, status) in statuses {
            self.statuses.entry(stratum).or_insert(status);
        }
        self
    }
}

/// The current head of one stream, already validated and contribution-checked.
#[derive(Clone, Debug)]
struct Head {
    rank: u64,
    contribution: Fixed,
    item: Term,
}

/// A candidate's accumulated NRA state.
#[derive(Clone, Debug)]
struct CandidateState {
    lower_bound: Fixed,
    contributions: Vec<(Iri, u64, Fixed)>,
    /// The streams that have named this candidate.
    ///
    /// It answers two questions, and the second is why no separate field holds
    /// `Dom(x)`. It is the membership half of [`FusionStream::is_final`]; and
    /// it *is* `Dom(x)` — the set of blocks this candidate can lie in is
    /// exactly the intersection of the declared domains of the streams in here,
    /// so storing that intersection would be storing a pure function of a set
    /// the engine already holds. Derived on demand by
    /// [`FusionStream::could_name`], which costs a scan over a handful of tags
    /// and no allocation at all; a stored copy would cost one heap-allocated
    /// tag set **per candidate**, in the one structure whose size the frontier
    /// argument is about.
    seen_streams: BTreeSet<usize>,
}

impl CandidateState {
    /// A candidate that has been named by some stream but has accumulated
    /// nothing yet.
    ///
    /// A zero lower bound is the honest starting point rather than a placeholder:
    /// before any contribution is summed, zero is exactly what this candidate is
    /// known to score.
    fn new() -> Self {
        Self {
            lower_bound: Fixed::ZERO,
            contributions: Vec::new(),
            seen_streams: BTreeSet::new(),
        }
    }

    /// The best (minimum) rank this candidate holds in any stratum.
    fn best_rank(&self) -> u64 {
        self.contributions
            .iter()
            .map(|(_, rank, _)| *rank)
            .min()
            .unwrap_or(u64::MAX)
    }
}

/// What survives a candidate's certification, so the answer invariant can
/// outlive the frontier.
///
/// Deliberately not a second `CandidateState`. Once a row is emitted its score
/// is settled and its provenance has been handed to the caller, so the only
/// question anyone can still ask about it is the one this answers: *which
/// streams already named it*. Keeping anything more would be keeping a copy of
/// the answer, which is the caller's to hold and not this engine's.
#[derive(Clone, Debug)]
struct EmittedRecord {
    /// The stream indexes that contributed to the candidate before it
    /// certified — [`CandidateState::seen_streams`], moved out of the state
    /// being dropped rather than rebuilt.
    /// The stream indexes that contributed to the candidate before it
    /// certified.
    ///
    /// It carries the candidate's `Dom(x)` with it, at no extra cost: the
    /// blocks a certified candidate could lie in are the intersection of the
    /// declarations of exactly these streams, so a stream naming it *after* it
    /// certified is held to the same declaration a stream naming it before
    /// would have been. Without that, the domain promise would hold only while
    /// a candidate sat in the frontier — which is to say it would hold or not
    /// depending on how deep the caller happened to read, the identical failure
    /// the [`DuplicatePolicy::Unique`] promise is kept past the frontier to
    /// avoid.
    named_by: BTreeSet<usize>,
}

/// The lookup policy for [`FusionStream`]'s emitted map: the workspace's
/// fixed-key, seed-free [`ahash`] hasher.
///
/// # Why a hasher at all, in a crate that is otherwise byte-deterministic
///
/// Because this is the one structure here whose *order* is provably unobserved.
/// It is consulted by key and never iterated — no `for` loop, no `values()`, no
/// `keys()` — so its iteration order reaches no output, no identity and no
/// ordering decision. Every ordered structure in this engine is ordered because
/// something reads it in order: the frontier is scanned to pick an emittable
/// candidate, `statuses` and `attestations` are serialized into the trailer and
/// digested into [`EvidenceId`], and `seen_streams` is both scanned and moved
/// into [`EmittedRecord`]. Those are `BTree`s and must stay `BTree`s. This one
/// answers `contains` and nothing else, so the repo's hot-map rule applies and
/// the lookup is paid at hash speed rather than at `log n` comparisons over
/// whole candidate terms.
///
/// Fixed-key rather than `RandomState` for the reason every interner in this
/// workspace is: `wasm32-unknown-unknown` has no random source to seed one
/// from, and a per-process seed would put nondeterminism into a crate whose
/// entire claim is the same answer on every target. See `purrdf-core`'s
/// `hash` module for the workspace policy this follows.
type EmittedHasher = core::hash::BuildHasherDefault<ahash::AHasher>;

/// The NRA fusion engine over a set of verified ranked streams.
///
/// `FusionStream` is generic over the stream type and produces [`FusedRow`]s on
/// demand. Call [`next`](FusionStream::next) until it returns `Ok(None)`, then
/// [`trailer`](FusionStream::trailer) for the producers' statuses.
pub struct FusionStream<S: RankedStream> {
    streams: Vec<(Iri, S)>,
    profile: FusionProfile,
    plan_id: Option<PlanId>,
    initialized: bool,
    heads: Vec<Option<Head>>,
    exhausted: Vec<bool>,
    next_ranks: Vec<u64>,
    rows_pulled: Vec<u64>,
    last_contribution: Vec<Option<Fixed>>,
    /// Per-stream count of adjacent ranks whose contributions were equal.
    ///
    /// Counted by [`Self::count_collision`] against the previous row's
    /// contribution, one equality compare per row. It is the exact number of
    /// ranks this run could not separate — an observation, not the profile's
    /// a-priori bound.
    collisions_observed: Vec<u64>,
    /// The identity set of every stream that declared
    /// [`DuplicatePolicy::Allowed`], and `None` for every stream that declared
    /// [`DuplicatePolicy::Unique`].
    ///
    /// The absence is the point, not a micro-optimization: this is the only
    /// structure in the engine that grows with the rows pulled, and a stream
    /// that promised no repeats has nothing for it to hold. `None` also makes
    /// the promise structural — there is no set for a later edit to start
    /// filling on a stream that declined to pay for one.
    seen_items: Vec<Option<BTreeSet<Term>>>,
    /// What every handed stream declared about the blocks of the candidate
    /// universe it may name, read in [`Self::new`] before a single row was
    /// pulled, and held for the whole fusion.
    ///
    /// Indexed by stream, exactly as `heads` and `seen_items` are, because that
    /// is how every question asked of it is shaped: *can stream `s` still name
    /// this candidate* is asked of `s` at the moment `s` has an open head. Held
    /// rather than consumed — unlike the duplicate term, which becomes
    /// structural in `seen_items` and is never asked again — because the answer
    /// is needed on every finality test, every upper bound, every threshold and
    /// every pull.
    domains: Vec<CandidateDomains>,
    /// What every handed stream attested about the index behind it, read in
    /// [`Self::new`] before a single row was pulled and never read again.
    ///
    /// Held rather than re-asked for the same reason it is read early: a
    /// generation is pinned when the index opens, so one read is the truth for
    /// the whole fusion, and a second read at the end would be a read of
    /// whatever state a bounded stop left the stream in.
    attestations: BTreeMap<Iri, PfAttestation>,
    statuses: BTreeMap<Iri, ProducerStatus>,
    frontier: BTreeMap<CandidateId, CandidateState>,
    /// Every candidate that has left the frontier by being certified, and the
    /// streams that named it while it was there.
    ///
    /// This is what makes *a fused answer never contains the same entity twice*
    /// an unconditional property of this engine rather than a property of how
    /// deep a caller happened to read. The frontier alone cannot carry it: a
    /// candidate is removed the instant it certifies, so a stream naming it
    /// afterwards would find nothing to collide with and would start a fresh
    /// candidate — the same entity emitted a second time, with a wrong, small
    /// score and a best rank drawn from one stratum instead of all of them.
    ///
    /// Written in [`Self::next`], at the one place a candidate leaves the
    /// frontier, and read in [`Self::pull`], before the frontier is touched.
    /// Nothing else may write it: an entry here means *this row is in the
    /// caller's answer*, and a second author would make that stop being true.
    ///
    /// # What it costs, and why that is the right unit
    ///
    /// One entry per row emitted — the term, and the stream-index set moved out
    /// of the certified [`CandidateState`] rather than allocated afresh. The
    /// module header carries the full argument; the short form is that this
    /// grows with the rows this fusion **returns**, never with the rows it
    /// pulls, so a bounded [`TopK`](crate::TopK) bounds it and a drain pays for
    /// something strictly smaller than the frontier entry it replaces.
    emitted: HashMap<Term, EmittedRecord, EmittedHasher>,
    threshold: Fixed,
    /// Whether the most recently emitted row took its place over a rival it tied
    /// with exactly on score.
    ///
    /// Reported for the *last* row an answer contains, where it is the fact a
    /// caller needs: the rival it beat is precisely the candidate that fell
    /// outside a top-k cut, and only the declared tie-break separated them.
    last_row_won_a_tie: bool,
}

impl<S: RankedStream> fmt::Debug for FusionStream<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let strata: Vec<&str> = self.streams.iter().map(|(iri, _)| iri.as_str()).collect();
        f.debug_struct("FusionStream")
            .field("strata", &strata)
            .field("profile", &self.profile)
            .field("frontier_len", &self.frontier.len())
            .field("threshold", &self.threshold)
            .finish_non_exhaustive()
    }
}

impl<S: RankedStream> FusionStream<S> {
    /// Build a fusion engine over `streams` under `profile`.
    ///
    /// Every stream's declared contract is read here, before any row is pulled,
    /// through [`RankedStream::contract`] — so what the engine holds, and what
    /// it may certify, is decided by the producers' own declarations rather
    /// than by one law applied to all of them. A stream declaring
    /// [`DuplicatePolicy::Allowed`] gets an identity set to de-duplicate
    /// against; one declaring [`DuplicatePolicy::Unique`] gets none. That term
    /// is consumed here rather than retained, because it is already structural
    /// in `seen_items` and nothing later asks for it again.
    ///
    /// The contract's other term — [`StreamContract::domains`] — is retained,
    /// because it is asked on every step. It is what lets this engine skip a
    /// stream that *provably* cannot name a candidate when deciding whether
    /// that candidate's score is final, and skipping those streams is the
    /// difference between a top-ten answer that reads a few dozen rows and one
    /// that drains two million. Reading it before the first pull matters for
    /// the same reason the duplicate term is read there: the declaration
    /// governs row one, so a declaration consulted later would be a declaration
    /// that had not applied to the rows already merged under it.
    ///
    /// Every stream's attestation is read here too, and it is read *here*
    /// rather than at the end for a reason the contract does not share: an
    /// index generation is pinned when the index is opened, so the version that
    /// answers the first row is the version that answers the last, and reading
    /// it before the first pull is reading the truth for the whole fusion at
    /// the one instant every stream is guaranteed to be in. Reading it at the
    /// end would instead ask each stream in whatever state this fusion happened
    /// to leave it — and for a stream a bounded top-k stopped, that is a state
    /// with no terminal report at all, so an attested incompleteness would be
    /// lost precisely when the bound made it matter. Unlike the contract, it is
    /// retained: the trailer reports it verbatim and derives both
    /// [`FusionTrailer::exactness`] and [`FusionTrailer::evidence_id`] from it.
    ///
    /// No plan identity is attached; use [`with_plan_id`](Self::with_plan_id) to
    /// name the pinned plan the streams came from. [`fuse`](crate::fuse) does
    /// that for its caller, reading the identity off the streams themselves
    /// through [`RankedStream::plan_id`]. No stream is pulled until the first
    /// [`next`](Self::next) call.
    #[must_use]
    pub fn new(streams: Vec<(Iri, S)>, profile: FusionProfile) -> Self {
        let count = streams.len();
        // One read of each contract, both terms taken from it. Asking twice
        // would let a stream answer differently the second time and leave the
        // engine holding two halves of two declarations.
        let contracts: Vec<_> = streams
            .iter()
            .map(|(_, stream)| stream.contract())
            .collect();
        let seen_items = contracts
            .iter()
            .map(|contract| match contract.duplicates {
                DuplicatePolicy::Unique => None,
                DuplicatePolicy::Allowed => Some(BTreeSet::new()),
            })
            .collect();
        let domains: Vec<CandidateDomains> = contracts
            .into_iter()
            .map(|contract| contract.domains)
            .collect();
        // Keyed by stratum, exactly as `statuses` is, so the two maps are read
        // under one key. A caller driving this type directly may hand it two
        // streams tagged with one stratum — `fuse` refuses that before pulling
        // — and both maps collapse such a pair the same way rather than
        // disagreeing about how many producers there were.
        let attestations = streams
            .iter()
            .map(|(stratum, stream)| (stratum.clone(), stream.attestation()))
            .collect();
        Self {
            streams,
            profile,
            plan_id: None,
            initialized: false,
            heads: (0..count).map(|_| None).collect(),
            exhausted: vec![false; count],
            next_ranks: vec![1; count],
            rows_pulled: vec![0; count],
            last_contribution: vec![None; count],
            collisions_observed: vec![0; count],
            seen_items,
            domains,
            attestations,
            statuses: BTreeMap::new(),
            frontier: BTreeMap::new(),
            emitted: HashMap::default(),
            threshold: Fixed::ZERO,
            last_row_won_a_tie: false,
        }
    }

    /// Attach the pinned plan identity these streams came from.
    #[must_use]
    pub fn with_plan_id(mut self, plan_id: PlanId) -> Self {
        self.plan_id = Some(plan_id);
        self
    }

    /// The next certified fused row, or `Ok(None)` when the frontier is empty.
    ///
    /// # Errors
    ///
    /// [`FusionError::Overflow`] when a checked sum leaves the fixed-point
    /// range; [`FusionError::Protocol`] when a stream violates the input
    /// protocol; [`FusionError::MaxContributionsExceeded`] when a candidate is
    /// contributed to more times than there are strata — an invariant
    /// violation, reachable only from a hand-built stream set that repeats a
    /// stratum.
    pub async fn next(&mut self) -> Result<Option<FusedRow>, FusionError> {
        self.ensure_initialized().await?;
        loop {
            self.threshold = self.compute_threshold()?;
            if let Some(id) = self.select_emittable()? {
                let state = self.frontier.remove(&id).ok_or_else(|| {
                    FusionError::MalformedProfile(
                        "selected candidate vanished from the frontier".to_owned(),
                    )
                })?;
                // Whether this row won its place on score or on the tie-break.
                // Scanned here rather than counted inside `select_emittable`,
                // because that pass filters an exactly-tied rival out as
                // dominated — `outranks` settles a tie by the declared order —
                // so the loser is already gone by the time a best is chosen. A
                // rival must be final to have been a real contender: one that
                // can still gain rank is not yet tied with anything.
                self.last_row_won_a_tie = self
                    .frontier
                    .values()
                    .any(|rival| rival.lower_bound == state.lower_bound && self.is_final(rival));
                // The candidate leaves the frontier here and nowhere else, so
                // this is the only place the record of its emission can be
                // written. `seen_streams` is **moved** out of a state that is
                // being dropped either way: rebuilding the set would allocate a
                // second copy of something already owned, to hold exactly the
                // same members. The key is cloned because the row carries the
                // term to the caller — one clone per row *emitted*, which is
                // the same unit the map itself is charged in, and not a clone
                // on the per-row-pulled path.
                let CandidateState {
                    lower_bound,
                    mut contributions,
                    seen_streams,
                } = state;
                self.emitted.insert(
                    id.clone(),
                    EmittedRecord {
                        named_by: seen_streams,
                    },
                );
                contributions.sort_by(|left, right| {
                    left.0
                        .as_str()
                        .cmp(right.0.as_str())
                        .then(left.1.cmp(&right.1))
                });
                return Ok(Some(FusedRow {
                    entity: id,
                    score: lower_bound,
                    contributions,
                    threshold_witness: self.threshold,
                }));
            }
            match self.best_head_index() {
                Some(index) => self.pull(index).await?,
                None => return Ok(None),
            }
        }
    }

    /// Close every stream that is still open at the bound reading stopped at,
    /// and return the terminal trailer.
    ///
    /// Every stream this fusion was handed gets a status, and no stream is read
    /// to produce one. A stream that reached its own end already returned a
    /// receipt, which was checked against the rows it emitted
    /// ([`ProtocolError::ForgedReceipt`]) and is reported unchanged. A stream
    /// still holding rows — because the caller stopped certifying, or because
    /// the threshold argument never needed to look further — is closed with
    /// [`ProducerStatus::CeilingReached`] at the contribution of its current
    /// head: the last row read from it, and therefore the inclusive bound above
    /// which it was read completely.
    ///
    /// # A bounded stop rather than a drain
    ///
    /// Draining here would force each producer to utter `Exhausted`, and the
    /// trailer would then say every stratum was complete — which is false
    /// whenever a bound stopped the reading, and expensive exactly when the
    /// bound mattered most: it pulls every row of every stream, so a top-ten
    /// answer over three million-row strata would read three million rows.
    /// §7's memory bound is the whole reason fused enumeration is top-k, and a
    /// terminal report is not allowed to spend it.
    ///
    /// The bounded status is not a weaker claim, it is an honest one. §6 asks
    /// that every producer keep its own status, not that every producer be
    /// exhausted, and "read down to this contribution and no further" is a
    /// status a consumer can act on: it names precisely where this stratum's
    /// evidence ends. Completeness is still asserted only here, never
    /// mid-stream, and still never by the rows a caller happens to hold.
    ///
    /// A producer that never became a stream is not here and cannot be; the
    /// stage that ran it adds it through [`FusionTrailer::completed_with`].
    ///
    /// Nothing is consumed: the streams are left exactly as they were, so a
    /// caller that reads the trailer and then decides to certify further rows
    /// through [`next`](Self::next) still can, and each stream's own receipt
    /// replaces its bounded status if it later reaches the end.
    ///
    /// # Errors
    ///
    /// [`FusionError::Protocol`] when a stream violates the protocol while its
    /// first row is pulled, and [`FusionError::Overflow`] on a checked sum
    /// overflow — both only on a fusion whose streams have not been read at all
    /// yet, because a bound of zero rows must still report a status per
    /// producer.
    pub async fn trailer(&mut self) -> Result<FusionTrailer, FusionError> {
        self.ensure_initialized().await?;
        for index in 0..self.streams.len() {
            // A head is a row already pulled and not yet merged, so its
            // contribution is the lowest this fusion read from the stream;
            // contributions are monotonically non-increasing with rank, so
            // everything above it was read too. A stream with no head has
            // already recorded its own receipt.
            let Some(bound) = self.heads[index].as_ref().map(|head| head.contribution) else {
                continue;
            };
            let stratum = self.streams[index].0.clone();
            self.statuses
                .insert(stratum, ProducerStatus::CeilingReached { bound });
        }
        // Derived per stratum at trailer time, from counters the row loop
        // already maintains. Nothing here runs per row: `separation` is a map
        // lookup on a value the profile derived once at construction, and the
        // other two are reads.
        let resolution = (0..self.streams.len())
            .filter_map(|index| {
                let stratum = &self.streams[index].0;
                self.profile.monotone_depth(stratum).map(|separation| {
                    (
                        stratum.clone(),
                        StratumResolution {
                            separation,
                            // `next_ranks` starts at one and advances once per
                            // row pulled, so this is the highest rank read and
                            // cannot underflow.
                            ranks_pulled: self.next_ranks[index] - 1,
                            collisions_observed: self.collisions_observed[index],
                        },
                    )
                })
            })
            .collect();

        // Derived from the attestations pinned at construction, never from the
        // statuses just written. A stream stopped at a bound has a bounded
        // status and may still have served from a short index; those are two
        // facts and this is the second one, so nothing about how deep this
        // fusion read may reach this derivation.
        let short_strata: BTreeSet<Iri> = self
            .attestations
            .iter()
            .filter(|(_, attestation)| {
                matches!(attestation.service, ServiceLevel::Incomplete { .. })
            })
            .map(|(stratum, _)| stratum.clone())
            .collect();
        let exactness = if short_strata.is_empty() {
            ScoreExactness::Exact
        } else {
            ScoreExactness::LowerBounds {
                strata: short_strata,
            }
        };

        Ok(FusionTrailer {
            statuses: self.statuses.clone(),
            // Digested through the one encoder the trailer's own
            // `evidence_canonical_bytes` calls, so the identity in an answer
            // and the identity a holder of that answer re-derives cannot come
            // from two encodings that drifted apart.
            evidence_id: EvidenceId::from_canonical(&evidence_canonical_bytes(&self.attestations)),
            attestations: self.attestations.clone(),
            exactness,
            plan_id: self.plan_id,
            profile_id: self.profile.id(),
            resolution,
            // Keyed like `attestations`, from the same pre-pull read, so a pair
            // of streams a caller tagged with one stratum collapses the same
            // way in both maps rather than making them disagree about how many
            // producers there were.
            domains: self
                .streams
                .iter()
                .enumerate()
                .map(|(index, (stratum, _))| (stratum.clone(), self.domains[index].clone()))
                .collect(),
            cut_on_a_tie: self.last_row_won_a_tie,
        })
    }

    /// Pull the first head of every stream.
    async fn ensure_initialized(&mut self) -> Result<(), FusionError> {
        if self.initialized {
            return Ok(());
        }
        for index in 0..self.streams.len() {
            self.heads[index] = self.fetch(index).await?;
        }
        self.initialized = true;
        Ok(())
    }

    /// Pull and validate rows from stream `index` until one of them is a head,
    /// or the stream ends.
    ///
    /// Validation is the whole point of the boundary: rank contiguity, the
    /// profile's contribution value, the declared duplicate handling, and a
    /// consistent terminal receipt. Every one of those
    /// is checked on every row the producer emits — including a row this function
    /// then drops as a declared duplicate, because a dropped row is still a row
    /// the producer made claims about, and a claim nobody checks is a hole a
    /// permissive policy could be used to hide a bad row in.
    ///
    /// # Why this loops
    ///
    /// Under [`DuplicatePolicy::Allowed`] a repeat contributes nothing: the item
    /// already holds this stratum's best rank, which is its first and lowest,
    /// and counting it again would add a second contribution for one stratum.
    /// So the row is validated and discarded, and the loop goes on to the next
    /// one. A discarded row never becomes a head, so nothing downstream has to
    /// know a stream repeats at all — including the frontier's own
    /// once-per-stream check, which stays the exact refusal it was.
    ///
    /// The producer is still charged for it. The rank counter advances, the
    /// collision baseline advances, and `rows_pulled` counts it, so the terminal
    /// receipt is still measured against every row the producer actually emitted
    /// ([`ProtocolError::ForgedReceipt`]).
    async fn fetch(&mut self, index: usize) -> Result<Option<Head>, FusionError> {
        loop {
            let pulled = {
                let (_, stream) = &mut self.streams[index];
                stream.next().await
            };
            let Some((rank, producer_contribution, item)) = pulled? else {
                let receipt = {
                    let (_, stream) = &mut self.streams[index];
                    stream.receipt().await
                }?;
                self.finish(index, receipt)?;
                return Ok(None);
            };

            let expected_rank = self.next_ranks[index];
            if rank < expected_rank {
                return Err(ProtocolError::OutOfOrderRanks {
                    expected: expected_rank,
                    got: rank,
                }
                .into());
            }
            if rank > expected_rank {
                return Err(ProtocolError::NonContiguousRanks {
                    gap: rank - expected_rank,
                }
                .into());
            }
            // The stratum is read by reference, not cloned: an `Iri` owns its
            // text, so cloning one here would be a heap allocation on every row
            // every stream emits, to serve a lookup that only borrows and an
            // error arm that is taken once at most.
            let weight = {
                let stratum = &self.streams[index].0;
                self.profile
                    .weight(stratum)
                    .ok_or_else(|| FusionError::UnknownStratum {
                        stratum: stratum.as_str().to_owned(),
                    })?
            };
            let expected =
                crate::reciprocal_rank::contribution_under(self.profile.decay(), weight, rank)?;
            if producer_contribution != expected {
                return Err(ProtocolError::ContributionMismatch {
                    expected,
                    got: producer_contribution,
                }
                .into());
            }

            // Ordered after the re-derivation on purpose: the collision count is
            // a claim about the *profile's* value at this rank, so it must run
            // on a value already proven to be that one. A forged contribution is
            // rejected above as the mismatch it is, rather than counted as a
            // shape of the decay curve.
            self.count_collision(index, producer_contribution);

            self.last_contribution[index] = Some(producer_contribution);
            self.next_ranks[index] = rank + 1;
            self.rows_pulled[index] += 1;

            let item: Term = item.into();
            match self.seen_items[index].as_mut() {
                // `Unique`: the producer promised no repeats, so there is no
                // per-stream identity set to consult and none to grow — that
                // set is exactly what this arm declines to pay for, and it is
                // the structure that would grow with every row this stream
                // emits. The promise is not thereby unchecked: `pull` refuses a
                // repeat whether the earlier occurrence is still a frontier
                // candidate or has already been certified and emitted, from
                // state the engine keeps per *emitted row* rather than per
                // pulled row. See [`Self::pull`].
                //
                // Nothing is dropped here, which is the difference that matters
                // to a caller: under this declaration a repeat becomes a
                // refusal naming the item and the stratum, not a silently
                // discarded row.
                None => {
                    return Ok(Some(Head {
                        rank,
                        contribution: expected,
                        item,
                    }));
                }
                // `Allowed`: the producer said repeats happen and the consumer
                // must de-duplicate, so the consumer de-duplicates. The insert's
                // own answer is the test, so the repeat costs one set operation
                // rather than a lookup and an insert.
                Some(seen) => {
                    if seen.insert(item.clone()) {
                        return Ok(Some(Head {
                            rank,
                            contribution: expected,
                            item,
                        }));
                    }
                }
            }
        }
    }

    /// Count the adjacent ranks the profile's arithmetic can no longer
    /// separate.
    ///
    /// This measures; it refuses nothing, and there is nothing here left for it
    /// to refuse. By the time a value reaches this function it has been proven
    /// equal to `contribution_under(decay, weight, rank)` and the ranks that
    /// produced it are contiguous and ascending, so the sequence is the
    /// profile's own curve read left to right — and that curve never rises (see
    /// [`ProtocolError::ContributionMismatch`], which is where a rising value is
    /// actually caught, as the wrong number it is). A comparison for a rise here
    /// would be a branch no input can take.
    ///
    /// Equality is **measured, not refused**. An equal adjacent pair carries no
    /// information about the producer at all: it says the profile's fixed-point
    /// decay stopped separating those two ranks at this depth, which is a
    /// property of `(decay rule, K, weight, depth)`. Refusing it rejected
    /// conforming streams for the consumer's own quantization. The count is
    /// reported per stratum in the fused trailer, where it is an exact
    /// observation rather than a bound inferred from the profile.
    fn count_collision(&mut self, index: usize, contribution: Fixed) {
        if self.last_contribution[index] == Some(contribution) {
            self.collisions_observed[index] += 1;
        }
    }

    /// Record a terminal receipt, refusing one the rows contradict.
    ///
    /// Every receipt that states a count is measured against the rows this
    /// fusion actually pulled, and the two counting receipts are measured
    /// identically. [`ProducerReceipt::Exhausted`] states how many rows it
    /// emitted; [`ProducerReceipt::DepthReached`] states the last rank it
    /// emitted, which is the same number because ranks are contiguous from one
    /// — a law already enforced row by row in [`Self::fetch`]. A producer may
    /// stop at the depth it was given; it may not miscount what it emitted, and
    /// the licence to stop reading is deliberately not also a licence to
    /// misreport. What fusion cannot check either way is the claim about rows
    /// nobody pulled: that a depth-stopped stream still held rows below its
    /// last rank is the producer's word, and the producer is the only party who
    /// can know it.
    fn finish(&mut self, index: usize, receipt: ProducerReceipt) -> Result<(), FusionError> {
        let actual = self.rows_pulled[index];
        match &receipt {
            ProducerReceipt::Exhausted { rows_emitted } if *rows_emitted != actual => {
                return Err(ProtocolError::ForgedReceipt {
                    declared: *rows_emitted,
                    actual,
                }
                .into());
            }
            ProducerReceipt::DepthReached { rank } if *rank != actual => {
                return Err(ProtocolError::ForgedReceipt {
                    declared: *rank,
                    actual,
                }
                .into());
            }
            ProducerReceipt::TermsRejected | ProducerReceipt::ExecutionFailed { .. }
                if actual > 0 =>
            {
                return Err(ProtocolError::ForgedReceipt {
                    declared: 0,
                    actual,
                }
                .into());
            }
            _ => {}
        }
        let stratum = self.streams[index].0.clone();
        self.exhausted[index] = true;
        self.heads[index] = None;
        self.statuses.insert(stratum, ProducerStatus::from(receipt));
        Ok(())
    }

    /// The index of the stream whose head has the highest contribution.
    fn best_head_index(&self) -> Option<usize> {
        let mut best: Option<(usize, Fixed)> = None;
        for (index, head) in self.heads.iter().enumerate() {
            let Some(head) = head else { continue };
            match best {
                None => best = Some((index, head.contribution)),
                Some((best_index, best_contribution)) => {
                    if head.contribution > best_contribution
                        || (head.contribution == best_contribution && index < best_index)
                    {
                        best = Some((index, head.contribution));
                    }
                }
            }
        }
        best.map(|(index, _)| index)
    }

    /// The threshold `T`: the most an item nobody has named yet could score.
    ///
    /// The largest, over the blocks of the candidate universe the open streams
    /// can still reach, of the summed contribution of the open heads that can
    /// reach that block. Every [`CandidateDomains::Unrestricted`] head counts
    /// in every block's sum, because such a stream may name anything.
    ///
    /// # Why the largest per-block sum, and not the global sum
    ///
    /// Because `T` bounds the score of an item **not yet in the frontier**, and
    /// such an item lies in exactly one block ([`DomainTag`] is a block of a
    /// partition). So the only contributions it can still collect are those of
    /// the streams that can reach *its* block, and the tightest sound bound is
    /// the best that any single block offers. Summing every head instead would
    /// also be sound — it is an upper bound of this one — but it is a looser
    /// bound, and a threshold that is too high is precisely what stops
    /// certification: no candidate rises above it, nothing is emitted, and the
    /// fusion pulls rows it had no need for. That is the drain this whole
    /// mechanism removes, so leaving it in the threshold would give the rest of
    /// the work back.
    ///
    /// A per-*candidate* sum would be tighter still and is not available here:
    /// this bound is about items nobody has seen, and there is no candidate to
    /// take a domain from. Using a frontier candidate's narrowed `Dom(x)` would
    /// bound the unseen items by what some *other* item's namers declared,
    /// which is unsound.
    ///
    /// # It reduces exactly to the old sum
    ///
    /// When every open stream declares `Unrestricted` there are no blocks to
    /// range over, the `Unrestricted` running sum is the sum of every open
    /// head, and that is the answer — the identical value this function
    /// returned before domains existed, for every fusion that does not declare
    /// them.
    fn compute_threshold(&self) -> Result<Fixed, FusionError> {
        // The floor of every block's sum: a stream that may name anything may
        // name the unseen item, whichever block it is in. It is also the whole
        // answer when no open stream restricts itself, and the honest bound for
        // an unseen item lying in a block no restricted stream declared.
        let mut unrestricted = Fixed::ZERO;
        for (index, head) in self.heads.iter().enumerate() {
            let Some(head) = head else { continue };
            if matches!(self.domains[index], CandidateDomains::Unrestricted) {
                unrestricted = unrestricted
                    .checked_add(head.contribution)
                    .map_err(|_| FusionError::Overflow)?;
            }
        }
        let mut threshold = unrestricted;
        // Every block any open restricted stream named is a block an unseen
        // item could be in, and no other block can beat the floor above.
        // Collected into an ordered set so the scan is over distinct tags and
        // is identical on every target.
        let mut blocks: BTreeSet<&DomainTag> = BTreeSet::new();
        for (index, head) in self.heads.iter().enumerate() {
            if head.is_none() {
                continue;
            }
            if let Some(tags) = self.domains[index].tags() {
                blocks.extend(tags);
            }
        }
        for block in blocks {
            let mut sum = unrestricted;
            for (index, head) in self.heads.iter().enumerate() {
                let Some(head) = head else { continue };
                match &self.domains[index] {
                    // Already counted in the floor.
                    CandidateDomains::Unrestricted => {}
                    CandidateDomains::Within(tags) => {
                        if tags.contains(block) {
                            sum = sum
                                .checked_add(head.contribution)
                                .map_err(|_| FusionError::Overflow)?;
                        }
                    }
                }
            }
            if sum > threshold {
                threshold = sum;
            }
        }
        Ok(threshold)
    }

    /// Whether `state`'s score is final: no stream that could still name it is
    /// open.
    ///
    /// Membership, not arithmetic. `U(x) == L(x)` was standing in for this
    /// question and cannot answer it: a live head whose contribution is
    /// [`Fixed::ZERO`] adds nothing to the sum, so an open stream that has not
    /// yet reached `x` is indistinguishable from an exhausted one. Certifying
    /// on that sum emits `x` and removes it from the frontier while a stream
    /// that could still name it is open — a candidate certified on a score that
    /// was not yet final, from a conforming producer under a legal profile.
    ///
    /// Two things rest on this being the membership test. The obvious one is
    /// the score: `x` is emitted having been paid a contribution it had not
    /// yet received. The second is [`Self::pull`]'s impossibility proof for the
    /// post-emission duplicate check — `named_by` is the `seen_streams` of the
    /// certifying moment, and it is only because certification requires *no
    /// open unseen stream* that a later name from an unseen stream is
    /// contradictory rather than routine. Under the arithmetic gate that arm
    /// would be reachable by ordinary input.
    ///
    /// A zero contribution is not hypothetical. The profile admits any weight
    /// strictly above zero, and
    /// [`contribution`](crate::reciprocal_rank::contribution) truncates toward
    /// zero at the declared scale, so a weight small enough that
    /// `w · trunc(S / (K + r))` falls below one unit of the scale contributes
    /// exactly zero at every rank — a legal profile, and a stream whose every
    /// row is live evidence that adds nothing to any sum.
    ///
    /// The structural test is strictly the stronger one: no open stream implies
    /// every absent contribution is zero, so everything this admits `U == L`
    /// admitted too. The two differ on exactly the candidates the arithmetic
    /// was wrong about, which is why no ordinary fusion certifies a row later
    /// than it did — where every live head contributes something, `U(x) > L(x)`
    /// held for precisely the candidates this rejects.
    ///
    /// What it does cost is the degenerate profile itself: a stratum that
    /// contributes zero at every rank has to be read to its end before any
    /// candidate it might still name can certify, because every one of its rows
    /// can still change a best rank and a provenance list even though none can
    /// change a score. That is the price of a weight the caller declared and the
    /// scale cannot represent, and it is paid in pulls rather than in wrong
    /// answers.
    ///
    /// # The declared-domain clause ADDS a licence; it weakens no test
    ///
    /// "Could still name it" is answered by two facts, not one. A stream can
    /// still name `x` when it is open **and** its declared domains meet
    /// `Dom(x)` — where `Dom(x)` is the intersection of the declarations of the
    /// streams that already named `x`. A stream whose declaration cannot reach
    /// `Dom(x)` is not a stream this test is unsure about: it is a stream that
    /// has promised, at registration, never to name this candidate, and a
    /// stream that breaks that promise is refused in [`Self::pull`] before it
    /// can reach any of this.
    ///
    /// So nothing above is relaxed. For every stream that *can* name `x` the
    /// test is the same membership question it has always been, zero-valued
    /// heads included, and both arguments that rest on it survive intact:
    ///
    /// * the score, because a skipped stream is one whose contribution to `x`
    ///   is not merely zero but impossible; and
    /// * [`Self::pull`]'s impossibility proof for the post-emission duplicate
    ///   check, which now has two ways to reach its contradiction rather than
    ///   one — the stream was finished, or the stream was outside `Dom(x)` and
    ///   was refused at the domain gate that runs first. Neither arm lets a
    ///   conforming stream name a certified candidate it had not contributed
    ///   to.
    ///
    /// Where every stream declares [`CandidateDomains::Unrestricted`] the extra
    /// clause is never true — everything meets everything — and this is exactly
    /// the test it was before domains existed.
    fn is_final(&self, state: &CandidateState) -> bool {
        self.heads.iter().enumerate().all(|(index, head)| {
            head.is_none()
                || state.seen_streams.contains(&index)
                || !self.could_name(index, &state.seen_streams)
        })
    }

    /// Whether stream `index` could still name a candidate that the streams in
    /// `named_by` have already named.
    ///
    /// This is `domains[index] ∩ Dom(x) ≠ ∅`, computed without building
    /// `Dom(x)`. `Dom(x)` is the intersection of the declarations of the
    /// streams in `named_by`, so a block both sides admit is exactly a block
    /// this stream declares that *every* namer also declares — which is the
    /// loop below, and which allocates nothing. A stream that declared
    /// [`CandidateDomains::Unrestricted`] may name anything and is always a
    /// yes; a candidate named only by `Unrestricted` streams constrains
    /// nothing and every stream is a yes for it, which is why a fusion with no
    /// declarations anywhere behaves exactly as it did before they existed.
    ///
    /// `named_by` is never empty where this is called from — a candidate is in
    /// the frontier, or in `emitted`, only because some stream put it there —
    /// and on an empty set the answer is `true` for every stream, which is the
    /// right answer anyway: nothing has been declared about a candidate nobody
    /// has named.
    fn could_name(&self, index: usize, named_by: &BTreeSet<usize>) -> bool {
        match &self.domains[index] {
            CandidateDomains::Unrestricted => true,
            CandidateDomains::Within(tags) => tags.iter().any(|tag| {
                named_by
                    .iter()
                    .all(|namer| self.domains[*namer].admits(tag))
            }),
        }
    }

    /// `U(x)`: `L(x)` plus the current head of every stream that could still
    /// contribute to `x` — open, unseen, and declared over a domain `Dom(x)`
    /// meets.
    ///
    /// This is a *comparison* quantity — the most `x` could still become — and
    /// is asked nothing else. Whether `x` is done is [`Self::is_final`]'s
    /// question, and a sum cannot answer it.
    ///
    /// The domain clause is the same licence [`Self::is_final`] takes, and it
    /// must be taken in both or neither: a stream that cannot name `x` adds
    /// nothing to what `x` can become, so counting its head here would inflate
    /// `U(x)` with a contribution that will never arrive and would keep `x`
    /// blocking rivals it cannot actually beat. Where every stream declares
    /// [`CandidateDomains::Unrestricted`] this is the sum it always was.
    fn upper_bound(&self, state: &CandidateState) -> Result<Fixed, FusionError> {
        let mut bound = state.lower_bound;
        for (index, head) in self.heads.iter().enumerate() {
            if state.seen_streams.contains(&index) {
                continue;
            }
            if !self.could_name(index, &state.seen_streams) {
                continue;
            }
            if let Some(head) = head {
                bound = bound
                    .checked_add(head.contribution)
                    .map_err(|_| FusionError::Overflow)?;
            }
        }
        Ok(bound)
    }

    /// Whether candidate `left` should be emitted before candidate `right`.
    ///
    /// The declared total tie-break: score descending, then best rank ascending,
    /// then canonical term byte order ascending.
    fn is_better(
        left_id: &CandidateId,
        left: &CandidateState,
        right_id: &CandidateId,
        right: &CandidateState,
    ) -> bool {
        match left.lower_bound.cmp(&right.lower_bound) {
            core::cmp::Ordering::Greater => true,
            core::cmp::Ordering::Less => false,
            core::cmp::Ordering::Equal => match left.best_rank().cmp(&right.best_rank()) {
                core::cmp::Ordering::Less => true,
                core::cmp::Ordering::Greater => false,
                core::cmp::Ordering::Equal => left_id.as_str() < right_id.as_str(),
            },
        }
    }

    /// Whether `other` could still be ordered ahead of the finalized `state`.
    ///
    /// `state` is known final ([`Self::is_final`]), so the question is only what
    /// `other` can still become. Three cases, and only the third is subtle:
    ///
    /// * `U(other) > L(state)` — `other` may still outscore it. Blocked.
    /// * `U(other) < L(state)` — `other` can never catch it. Free.
    /// * `U(other) == L(state)` — `other` can at most tie. If `other` is itself
    ///   final its tie-break keys are settled, so the declared total order
    ///   decides and exactly one of the pair goes first. If `other` is not yet
    ///   final its best rank can still improve — a later stream may report it at
    ///   a better rank — so it could win a tie it cannot yet be compared on, and
    ///   the conservative answer is to wait.
    ///
    /// "Is `other` itself final" is the structural test, for the same reason the
    /// emission gate uses it: a stream still able to name `other` at a better
    /// rank is exactly the stream that would change the tie-break, and a zero
    /// contribution hides it from `U(other)` while changing the rank all the
    /// same.
    ///
    /// The third case is what keeps the frontier bounded: without it a pair of
    /// finalized, exactly-tied candidates blocks on itself forever.
    fn outranks(
        &self,
        other_id: &CandidateId,
        other: &CandidateState,
        id: &CandidateId,
        state: &CandidateState,
    ) -> Result<bool, FusionError> {
        let other_upper = self.upper_bound(other)?;
        Ok(match other_upper.cmp(&state.lower_bound) {
            core::cmp::Ordering::Greater => true,
            core::cmp::Ordering::Less => false,
            core::cmp::Ordering::Equal => {
                if self.is_final(other) {
                    Self::is_better(other_id, other, id, state)
                } else {
                    true
                }
            }
        })
    }

    /// The best candidate that is safe to emit, if any.
    ///
    /// A candidate is emittable when its score is final ([`Self::is_final`]), it
    /// is above the threshold, and no other candidate could still be ordered
    /// ahead of it. Once every stream is exhausted the threshold is zero and all
    /// remaining candidates are ordered by the declared tie-break instead.
    ///
    /// # Ties are broken here, not only among the already-emittable
    ///
    /// "Could be ordered ahead of it" is the declared total order — score, then
    /// best rank, then canonical term bytes — and not score alone. A rival whose
    /// upper bound merely *equals* this candidate's final score cannot outscore
    /// it; it can at most tie, and a tie is what the tie-break exists to settle.
    ///
    /// Comparing scores alone would make two candidates with exactly equal final
    /// scores each dominate the other, so neither would ever be emittable and
    /// fusion would pull every stream to exhaustion before the exhausted branch
    /// below could order them. That is not a corner case: reciprocal-rank fusion
    /// over strata that disagree symmetrically produces exact ties routinely —
    /// one candidate at ranks 1 and 2, another at 2 and 1, sum to the same value
    /// — and the cost is the bounded frontier this type exists to provide.
    fn select_emittable(&self) -> Result<Option<CandidateId>, FusionError> {
        if self.frontier.is_empty() {
            return Ok(None);
        }
        let active = self.heads.iter().any(Option::is_some);
        let mut best: Option<CandidateId> = None;
        for (id, state) in &self.frontier {
            if !self.is_final(state) {
                continue;
            }
            if active {
                if state.lower_bound <= self.threshold {
                    continue;
                }
                let mut dominated = false;
                for (other_id, other) in &self.frontier {
                    if other_id == id {
                        continue;
                    }
                    if self.outranks(other_id, other, id, state)? {
                        dominated = true;
                        break;
                    }
                }
                if dominated {
                    continue;
                }
            }
            match &best {
                None => best = Some(id.clone()),
                Some(best_id) => {
                    let best_state = &self.frontier[best_id];
                    if Self::is_better(id, state, best_id, best_state) {
                        best = Some(id.clone());
                    }
                }
            }
        }
        Ok(best)
    }

    /// Process stream `index`'s head into the frontier, then advance it.
    ///
    /// # Errors
    ///
    /// [`FusionError::Overflow`] when the checked sum leaves the fixed-point
    /// range, [`FusionError::MaxContributionsExceeded`] when the candidate has
    /// been contributed to once more than there are strata, and
    /// [`ProtocolError::DuplicateItem`] when this stream would name one
    /// candidate twice — whether the earlier occurrence is still a frontier
    /// candidate or has already been certified and emitted.
    ///
    /// # Where a declared-`Unique` stream's promise is checked
    ///
    /// Here, in two places, and the pair is deliberately not one place.
    ///
    /// The first is the frontier itself, and it is free: `seen_streams` must
    /// already mean "this stream has contributed, once" at the point
    /// [`Self::is_final`] reads it, so the insert's own return value is the
    /// test and the check costs one `BTreeSet` operation and no extra memory at
    /// all. It catches every repeat whose earlier occurrence is still
    /// un-emitted — that is, every repeat that could corrupt a candidate's
    /// score or its best rank before the caller ever sees the row.
    ///
    /// The second is `emitted`, consulted below, and it is what the first one
    /// cannot be: unconditional. A candidate is removed from the frontier the
    /// instant it certifies, so a stream naming it afterwards collides with
    /// nothing and would open a fresh candidate — **the same entity in the
    /// answer twice**, the second time scored from one stratum's late rank
    /// alone. Bounding the promise by the frontier would make the answer
    /// invariant depend on how deep the caller happened to read, and "the same
    /// entity appears twice, but only in long answers" is not a weaker
    /// guarantee than the invariant, it is the absence of one.
    ///
    /// So the promise is held for the whole fusion, and the cost is stated
    /// rather than hidden: one `emitted` entry per row **returned**. That is
    /// the unit that matters. A [`TopK`](crate::TopK)-bounded call cannot hold
    /// more entries than the answer it is building, and a caller that drains
    /// pays, per candidate, strictly less than the frontier entry the emission
    /// just released — the contribution vector and lower bound are dropped and
    /// only the stream-index set survives, moved rather than rebuilt. What a
    /// [`DuplicatePolicy::Unique`] declaration buys its way out of is untouched
    /// by this: that is the *per-stream* identity set, one per stream and
    /// growing with every row that stream emits, and no such set is built here.
    ///
    /// # The repeat is surfaced, not smoothed over
    ///
    /// The alternative — silently dropping the late repeat, as
    /// [`Self::fetch`] does for a stream that declared
    /// [`DuplicatePolicy::Allowed`] — would also keep the answer invariant, and
    /// it is the wrong answer for a `Unique` declaration. The declaration is a
    /// claim about the producer's index, and a producer that breaks it has a
    /// stream whose *ranks* can no longer be trusted either: the same entity at
    /// two ranks means one of them is wrong, and every rank below the repeat is
    /// suspect with it. Dropping the row would return a plausible answer
    /// computed from evidence the layer has just proven unreliable. Refusing
    /// names the entity and the stratum, which is what a consumer can act on.
    /// A producer that cannot make the promise declares `Allowed` instead and
    /// is de-duplicated in [`Self::fetch`], completely and at the cost the
    /// policy implies — that route is one line away and is the supported one.
    ///
    /// # Why the second arm below is an internal error and not a variant
    ///
    /// The `emitted` lookup has two outcomes, and only one of them is a
    /// producer's fault. If the record says this stream named the candidate
    /// before, the `Unique` promise is broken and
    /// [`ProtocolError::DuplicateItem`] is exactly what happened. If it says
    /// this stream did *not*, no producer has done anything wrong and the
    /// engine's own certification argument has failed — so it is reported as
    /// the invariant violation it is, in the same shape [`Self::next`] uses for
    /// a candidate that vanished from the frontier.
    ///
    /// That arm is unreachable, and here is the proof. Suppose stream `s` names
    /// `x` now, `x` has certified, and `s` is not in `x`'s `named_by`.
    /// Certification required [`Self::is_final`], which requires of *every*
    /// index either that its head was `None` or that it is in `seen_streams` —
    /// and `named_by` is precisely the `seen_streams` of that moment. So `s`'s
    /// head was `None` when `x` certified. A head is `None` only once
    /// [`Self::finish`] has recorded the stream's terminal receipt, and a
    /// finished stream is never pulled again. Therefore `s` could not name `x`
    /// now, and the hypothesis is contradictory.
    ///
    /// Note what that proof rests on: `is_final` being a *membership* test over
    /// open streams rather than the arithmetic `U(x) == L(x)` it replaced.
    /// Under the arithmetic gate a stream with a zero-valued head could be open
    /// and unseen while `x` certified, and this arm would be reachable — by a
    /// conforming producer, through a legal profile. The two guards are one
    /// argument, and weakening either re-opens the other.
    ///
    /// # The proof under declared domains
    ///
    /// [`Self::is_final`] now admits a second way for a stream to be open and
    /// unseen at certification: `s`'s declared domains did not meet `Dom(x)`.
    /// The proof extends by a case rather than breaking, because that case is
    /// refused before it can arrive here. Suppose again that `s` names `x` now,
    /// `x` has certified, and `s` is not in `named_by`. Certification required
    /// of `s` either that its head was `None` — the finished-stream case above,
    /// unchanged — or that `s`'s declaration could not reach `Dom(x)`. In the
    /// second case `s` is naming a candidate it promised never to name, and the
    /// domain gate immediately above this arm has already refused the row as
    /// [`ProtocolError::OutsideDeclaredDomain`]. Either way this arm is
    /// unreachable, and in neither way is a producer's fault reported as an
    /// engine invariant violation.
    ///
    /// That is also why `Dom(x)` is carried into the emitted record rather than
    /// dropped with the frontier entry: the gate needs the declaration the
    /// candidate certified under, and a candidate that has left the frontier no
    /// longer has one anywhere else.
    ///
    /// Under `Allowed` neither arm is reachable rather than merely unused: a
    /// repeat is dropped in [`Self::fetch`] and never becomes a head, so no
    /// second row from one stream ever reaches the frontier or this lookup.
    /// The stratum to name as `named_by` in an
    /// [`ProtocolError::OutsideDeclaredDomain`], given the streams that already
    /// named the candidate and the stream that just did.
    ///
    /// Preference goes to a namer whose own declaration is genuinely disjoint
    /// from the offender's, because that is the pair a reader can act on: two
    /// declarations that directly contradict each other. `Dom(x)` is an
    /// intersection, so a stream can fail to meet it without being disjoint
    /// from any single namer — several narrow declarations can close a door no
    /// one of them closed alone — and in that case the first namer is reported
    /// as the witness that the narrowing began.
    ///
    /// Deterministic either way: `named_by` is an ordered set of stream
    /// indexes, and the streams are in the order the caller handed them over,
    /// so two runs of the same fusion name the same stratum.
    fn domain_witness(&self, named_by: &BTreeSet<usize>, offender: usize) -> String {
        let disjoint = named_by
            .iter()
            .find(|namer| !self.domains[**namer].intersects(&self.domains[offender]));
        let witness = disjoint.or_else(|| named_by.iter().next());
        witness.map_or_else(
            // No namer at all cannot happen — a candidate is in the frontier or
            // the emitted map only because some stream put it there — but the
            // lookup is total and the total answer says exactly that rather
            // than panicking on a library path a caller reaches with its own
            // streams.
            || "<none>".to_owned(),
            |namer| self.streams[*namer].0.as_str().to_owned(),
        )
    }

    async fn pull(&mut self, index: usize) -> Result<(), FusionError> {
        let Some(head) = self.heads[index].take() else {
            return Ok(());
        };
        let stratum = self.streams[index].0.clone();

        // Asked before the frontier is consulted, let alone touched, and before
        // the two profile bounds below — the same discipline the rest of this
        // function keeps: a refused row leaves every structure here exactly as
        // it found it. Ordering against the bounds is not load-bearing for
        // reachability (a certified candidate is absent from the frontier, so
        // its contribution count would restart at one and cross nothing), but
        // it is load-bearing for the diagnosis: the honest report for this row
        // is that a stream repeated an item, not that some budget was spent.
        if let Some(record) = self.emitted.get(&head.item) {
            if record.named_by.contains(&index) {
                return Err(ProtocolError::DuplicateItem {
                    item: head.item.as_str().to_owned(),
                    stratum: stratum.as_str().to_owned(),
                }
                .into());
            }
            // Ordered before the impossibility arm below, which is what keeps
            // that arm's proof true: a stream outside `Dom(x)` is one of the
            // two ways certification could have left this stream open and
            // unseen, and it is answered here as the broken declaration it is
            // rather than reported as an engine invariant violation. The
            // promise is held past the frontier for the same reason the
            // duplicate promise is: a guarantee that lapses once a row is
            // emitted is a guarantee that depends on how deep the caller read.
            if !self.could_name(index, &record.named_by) {
                let named_by = self.domain_witness(&record.named_by, index);
                return Err(ProtocolError::OutsideDeclaredDomain {
                    item: head.item.as_str().to_owned(),
                    stratum: stratum.as_str().to_owned(),
                    named_by,
                }
                .into());
            }
            return Err(FusionError::MalformedProfile(format!(
                "stream {index} named candidate {} after it certified without having \
                 contributed to it, which certification forbids",
                head.item.as_str()
            )));
        }

        // The candidate's next state is derived before the frontier is touched,
        // for two reasons. It lets the bounds be refused while `head.item` is
        // still owned here, so the frontier key is *moved* into the map rather
        // than cloned — an allocation that would otherwise be paid on every row
        // every stream emits, including the ones already in the frontier. And a
        // refused row then leaves the frontier exactly as it found it, instead
        // of a half-updated candidate no later call may read.
        //
        // The bound is read from the profile's own accessor, never recomputed,
        // so a future change to its derivation cannot drift enforcement away
        // from what the profile declares. It is the profile's stratum count, so
        // crossing it is not a budget a corpus spent — it says a stream named
        // this candidate twice, or two streams were handed the same stratum
        // tag. The profile's score ceiling needs no companion check: a
        // contribution is at most half its stratum's weight, so a sum bounded by
        // the count above is bounded by half the ceiling — see
        // [`FusionProfile::ceiling`](crate::FusionProfile::ceiling).
        let existing = self.frontier.get(&head.item);
        // The declaration is checked before anything is written, so a refused
        // row leaves the frontier exactly as it found it — the same discipline
        // the two profile bounds below keep. A candidate nobody has named yet
        // constrains nothing, so only an existing entry can contradict this
        // stream: `Dom(x)` narrows as streams name it, and a stream whose own
        // blocks it no longer meets is a stream naming a candidate it promised
        // never to name.
        if let Some(state) = existing
            && !self.could_name(index, &state.seen_streams)
        {
            let named_by = self.domain_witness(&state.seen_streams, index);
            return Err(ProtocolError::OutsideDeclaredDomain {
                item: head.item.as_str().to_owned(),
                stratum: stratum.as_str().to_owned(),
                named_by,
            }
            .into());
        }
        let lower_bound = existing
            .map_or(Fixed::ZERO, |state| state.lower_bound)
            .checked_add(head.contribution)
            .map_err(|_| FusionError::Overflow)?;
        let contribution_count = existing.map_or(0, |state| state.contributions.len()) + 1;
        let count = u32::try_from(contribution_count).unwrap_or(u32::MAX);
        if count > self.profile.max_contributions() {
            return Err(FusionError::MaxContributionsExceeded {
                item: head.item.as_str().to_owned(),
                count,
                max: self.profile.max_contributions(),
            });
        }
        match self.frontier.entry(head.item) {
            btree_map::Entry::Vacant(vacant) => {
                // A candidate nobody has contributed to yet cannot collide, so
                // the insert's answer here is `true` by construction.
                let state = vacant.insert(CandidateState::new());
                state.lower_bound = lower_bound;
                state
                    .contributions
                    .push((stratum, head.rank, head.contribution));
                state.seen_streams.insert(index);
            }
            btree_map::Entry::Occupied(mut occupied) => {
                if !occupied.get_mut().seen_streams.insert(index) {
                    // Refused before anything is written, so the frontier is
                    // left exactly as it was found — the same discipline the
                    // two profile bounds above keep.
                    //
                    // The stratum is named here for the same reason it is named
                    // at the post-emission site: one refusal, one meaning, and
                    // a consumer that cannot tell which of the two places
                    // caught the repeat also does not have to. The item is read
                    // off the occupied entry's key rather than off `head`,
                    // because the frontier took ownership of the term when this
                    // row's predecessor put it there.
                    return Err(ProtocolError::DuplicateItem {
                        item: occupied.key().as_str().to_owned(),
                        stratum: stratum.as_str().to_owned(),
                    }
                    .into());
                }
                let state = occupied.get_mut();
                state.lower_bound = lower_bound;
                state
                    .contributions
                    .push((stratum, head.rank, head.contribution));
            }
        }

        self.heads[index] = self.fetch(index).await?;
        Ok(())
    }
}
