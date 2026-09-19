// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The ranked-stream input protocol.
//!
//! A stratum's producer emits its rows in rank order, one row at a time, and
//! declares when it is done. Fusion is only sound if those declarations are
//! true, so the boundary is validated: the fusion engine checks rank contiguity
//! and re-derives every contribution as rows arrive, and compares each
//! producer's terminal receipt against what it actually emitted. A producer that
//! cannot describe its own completeness cleanly reports it through this channel;
//! it never gets to return a plausible-looking answer that quietly omitted a
//! row.
//!
//! # One fixed rank law, and two declared promises
//!
//! The rank check is absolute. Ranks are 1-based, contiguous and ascending for
//! every ranked stream there is, with no registration able to soften it, and the
//! fusion engine measures every row against the next rank it expects from that
//! stream ([`ProtocolError::OutOfOrderRanks`],
//! [`ProtocolError::NonContiguousRanks`]).
//!
//! The other two checks are not absolute, because the registry does not state
//! them absolutely. A ranked producer declares its duplicate handling and the
//! blocks of the candidate universe it may name where it is registered
//! ([`RankedDeclaration`]), and the consumer of its rows is this layer — so the
//! consumer reads those declarations and holds the stream to the promises it
//! actually made. [`StreamContract`] carries both with the stream through
//! [`RankedStream::contract`], and the two refusals below —
//! [`ProtocolError::DuplicateItem`] and
//! [`ProtocolError::OutsideDeclaredDomain`] — each say which declaration the
//! row was measured against.
//!
//! Both are promises about rows nobody has read yet, so both are verified
//! exactly as far as the rows actually pulled reach, and no further. That is
//! stated on each term rather than implied: a declaration no pulled row
//! contradicts is believed, because there is nothing else a consumer could do
//! with it short of reading the whole stream, which is the cost the declaration
//! exists to avoid.

use purrdf_sparql_eval::{CandidateDomains, DuplicatePolicy, PfAttestation, RankedDeclaration};
use purrdf_text::Fixed;

use crate::iri::Term;

/// The promise a ranked producer makes about one invocation's rows: whether an
/// item may repeat.
///
/// It is the producer's own — [`RankedDeclaration::duplicates`], supplied by the
/// host where the producer is registered. It is not inferred and has no default:
/// a stream either repeats items or it does not, and the consumer's behaviour
/// differs, so there is no honest "unstated" answer the way there is for a
/// stream that descends from no plan ([`RankedStream::plan_id`]).
///
/// # Why rank order is not a term of the contract
///
/// Rank order is a law, not a promise a producer gets to phrase. Every ranked
/// stream owes its consumer the same one: ranks are 1-based, contiguous and
/// ascending, so rows arrive numbered 1, 2, 3 with no gap, no repeat and no step
/// backwards, and rows a producer scores equally still take distinct
/// consecutive ranks under whatever total tie-break it applies. Because the law
/// is identical for every stream, there is nothing for a contract to carry and
/// no variant a consumer could branch on.
///
/// It is enforced rather than believed, and enforced on the rank itself:
/// [`FusionStream`](crate::FusionStream) holds the next rank it expects from
/// each stream and checks every row against it, raising
/// [`ProtocolError::OutOfOrderRanks`] below that rank and
/// [`ProtocolError::NonContiguousRanks`] above it. The only other quantity
/// fusion can observe is the *contribution*, which it computes itself from
/// `(decay rule, K, weight, rank)` and refuses if the producer's copy disagrees
/// ([`ProtocolError::ContributionMismatch`]); the producer supplies no term of
/// that either. Reading a rank claim in contribution space instead refused
/// conforming producers whenever fixed-point decay quantized two adjacent ranks
/// to one value — a property of the profile's arithmetic and of the depth read,
/// never of the stream.
///
/// # Why the contract travels with the stream
///
/// A stratum carries exactly one producer, refused at registration by
/// [`register_ranked`](purrdf_sparql_eval::PropertyFunctionRegistry::register_ranked)
/// and again at the admission waist, so a stratum's contract *is* its producer's
/// contract — nothing is combined, averaged or weakened across producers to
/// obtain it. The admission waist reads it off the registry once, into
/// [`StratumUnit::contract`](crate::StratumUnit), [`execute`](crate::execute)
/// tags every stream it returns with it
/// ([`StratumStream::contract`](crate::StratumStream)), and
/// [`FusionStream`](crate::FusionStream) asks each stream for it before pulling
/// a row. That is the same route a plan identity takes, and for the same reason:
/// a fact re-fetched at the end would be true of the registry rather than of the
/// stream that is actually being read.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StreamContract {
    /// The duplicate handling the producer declared. A
    /// [`DuplicatePolicy::Unique`] stream is believed and a repeat is a protocol
    /// violation; a [`DuplicatePolicy::Allowed`] stream is de-duplicated by the
    /// consumer, which is what that policy says a consumer must do.
    pub duplicates: DuplicatePolicy,
    /// Which blocks of the candidate universe the producer declared it may
    /// name.
    ///
    /// [`CandidateDomains::Unrestricted`] says "anything", which is the wider
    /// promise and the one every stream made before this term existed:
    /// [`FusionStream`](crate::FusionStream) then behaves exactly as it always
    /// did. A [`CandidateDomains::Within`] declaration is what lets fusion
    /// certify a candidate without first reading a stream that was never going
    /// to name it — the drain that makes a top-ten answer over two disjoint
    /// million-row strata read two million rows.
    ///
    /// # Why the consumer cannot derive this for itself
    ///
    /// Because the input protocol has no random access. A
    /// [`RankedStream`] offers `next` and `receipt`, so the only way to learn
    /// that a stream does *not* hold a candidate is to read it to its end. A
    /// consumer that certified earlier without a declaration would be emitting
    /// a score that a still-open stream might have raised — a lower bound
    /// presented as an exact value — so exact scores and a k-bounded read over
    /// strata that do not overlap are jointly unachievable unless the producers
    /// say which candidates they can name. They say it here.
    ///
    /// # It is verified over the rows pulled, and nowhere else
    ///
    /// Fusion refuses a stream that names a candidate its declaration cannot
    /// reach ([`ProtocolError::OutsideDeclaredDomain`]), which catches every
    /// contradiction that arrives in a row it actually read. A false
    /// declaration that no pulled row contradicts yields a score that
    /// declaration made wrong, and this layer does not claim otherwise. It is
    /// the identical trust the [`Self::duplicates`] term already carries: a
    /// false [`DuplicatePolicy::Unique`] is detected when the repeat is pulled,
    /// and not before.
    pub domains: CandidateDomains,
}

impl StreamContract {
    /// The contract `declaration` states, verbatim.
    #[must_use]
    pub fn declared(declaration: &RankedDeclaration) -> Self {
        Self {
            duplicates: declaration.duplicates,
            domains: declaration.domains.clone(),
        }
    }

    /// A contract stated directly, for a stream a caller built itself.
    #[must_use]
    pub const fn new(duplicates: DuplicatePolicy, domains: CandidateDomains) -> Self {
        Self {
            duplicates,
            domains,
        }
    }
}

/// A producer's declaration of how its stream ended.
///
/// This is the *receipt* a producer returns from [`RankedStream::receipt`]. It
/// becomes a [`ProducerStatus`](crate::ProducerStatus) in the fused trailer, and
/// the trailer preserves every producer's own status rather than reducing them
/// to one aggregate flag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProducerReceipt {
    /// The producer emitted every row it had.
    Exhausted {
        /// How many rows it emitted. Must equal the number fusion pulled.
        rows_emitted: u64,
    },
    /// The producer stopped at the depth it was given, and more rows existed.
    ///
    /// A read ending authored by the producer, stated in **rank** space: ranks
    /// one through `rank` were read and emitted, and the producer looked at
    /// nothing below `rank`. The rows did not run out — the depth did. A
    /// producer handed a depth of fifty that still had a fifty-first row says
    /// this; a producer handed a depth of fifty whose index held forty rows says
    /// [`Self::Exhausted`], because the depth is not what stopped it and its
    /// stratum *is* complete.
    ///
    /// # Why this is not [`Self::CeilingReached`]
    ///
    /// Both say "this stratum is not complete", and that is the whole of what
    /// they share. They differ in who stopped the read and in what space the
    /// stopping point is written:
    ///
    /// * this one is the **producer's** ending, and its stopping point is a
    ///   rank — a number the producer was handed as its depth and can act on
    ///   directly;
    /// * [`Self::CeilingReached`] as fusion writes it
    ///   ([`FusionStream::trailer`](crate::FusionStream::trailer)) is
    ///   **fusion's** ending, and its stopping point is a contribution — a
    ///   value in the profile's fixed-point space that the producer was never
    ///   given and that names no depth the producer understands.
    ///
    /// Collapsing them into one "stopped early" would destroy exactly the
    /// distinction a consumer acts on. A consumer that wants more rows must
    /// decide which knob to turn, and the two variants name different knobs:
    /// this one is answered by re-planning at a greater depth, the other by
    /// certifying further rows against the same streams. Neither number
    /// converts into the other, either — a rank becomes a contribution only
    /// under a profile the producer never saw, and a contribution does not
    /// convert back into a rank at all once fixed-point decay has quantized two
    /// adjacent ranks to one value. A consumer handed the merged fact would
    /// have to guess, and half its guesses would be wrong.
    ///
    /// A producer may stop at its depth; it may not miscount what it emitted.
    /// Fusion checks `rank` against the rows it actually pulled and refuses a
    /// disagreement as [`ProtocolError::ForgedReceipt`], exactly as it checks
    /// [`Self::Exhausted`]'s count — the depth is a licence to stop reading,
    /// never a licence to misreport.
    DepthReached {
        /// The last 1-based rank the producer emitted. Because ranks are
        /// contiguous from one, this is also the number of rows it emitted,
        /// which is what fusion measures it against.
        rank: u64,
    },
    /// The producer stopped at a declared score bound rather than at
    /// exhaustion.
    CeilingReached {
        /// The inclusive bound at which it stopped.
        bound: Fixed,
    },
    /// The producer could not run. It emits no rows.
    ExecutionFailed {
        /// A human-readable reason, carried verbatim.
        reason: String,
    },
    /// The producer declined the request terms it was given. It emits no rows.
    TermsRejected,
}

/// A producer's violation of the ranked-stream protocol.
///
/// Every variant names the exact dimension that failed. The fusion engine
/// returns these unchanged; it never repairs a malformed stream into a
/// plausible order.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ProtocolError {
    /// A rank arrived below the next expected rank — a producer that went
    /// backwards.
    #[error("stream rank went backwards: expected {expected}, got {got}")]
    OutOfOrderRanks {
        /// The rank fusion expected next.
        expected: u64,
        /// The rank the producer supplied.
        got: u64,
    },

    /// A rank arrived above the next expected rank — a producer that skipped
    /// ranks.
    #[error("stream ranks are not contiguous: skipped {gap} rank(s)")]
    NonContiguousRanks {
        /// How many ranks were skipped.
        gap: u64,
    },

    /// The same item was emitted twice within one stream that declared
    /// [`DuplicatePolicy::Unique`].
    ///
    /// Raised only under that declaration. A stream declaring
    /// [`DuplicatePolicy::Allowed`] says repeats happen and the consumer must
    /// de-duplicate, and this layer is that consumer: it de-duplicates such a
    /// stream instead of refusing it — one contribution per `(stratum, item)`,
    /// at the best rank the stream gave it. Raising this for a policy that
    /// predicted the repeat would be a refusal of valid input.
    ///
    /// # The promise is held for the whole fusion, not merely for the frontier
    ///
    /// Raised wherever the repeat is observed: while the earlier occurrence is
    /// still an un-emitted frontier candidate, and equally after that
    /// occurrence has been certified and left the frontier. A fused answer may
    /// not contain one entity twice, and that is not a property a bound on how
    /// long fusion remembers can be allowed to weaken — so the engine
    /// remembers, and a producer that cannot keep the promise it declared is
    /// told so rather than smoothed over. See
    /// [`FusionStream::pull`](crate::FusionStream) for what the remembering
    /// costs and why it is bounded by the rows *emitted*.
    ///
    /// # Both fields, because "which stream lied" is the actionable half
    ///
    /// A repeated item names *what* went wrong; the stratum names *who*. A
    /// consumer fusing several producers can act on the pair — the named
    /// producer's declared [`DuplicatePolicy`] is wrong, and until it is fixed
    /// that stratum's ranks are not trustworthy — and can act on neither half
    /// alone: the item alone does not say which of five strata to go and fix,
    /// and the stratum alone does not say which of its rows to look at.
    #[error("stream for stratum {stratum} emitted item {item:?} more than once")]
    DuplicateItem {
        /// The repeated item's canonical text.
        item: String,
        /// The stratum whose stream repeated it, exactly as that stream was
        /// tagged when it was handed to fusion.
        stratum: String,
    },

    /// A stream named a candidate that its declared domains cannot reach.
    ///
    /// Raised only against a [`CandidateDomains::Within`] declaration, and only
    /// where another stream has already named the same candidate: a domain tag
    /// names a block of a partition of the candidate universe, so a candidate
    /// lies in exactly one block, and two producers whose declarations put it
    /// in blocks with nothing in common cannot both be telling the truth about
    /// it. One of the two declarations is wrong, and the consumer cannot know
    /// which — so it reports the contradiction rather than picking a side.
    ///
    /// # This is not a refusal of an overlapping index
    ///
    /// Two producers that really do rank the same entities are declared over
    /// the same tag, or over tag sets that share one, or
    /// [`CandidateDomains::Unrestricted`], and every one of those fuses
    /// normally — the same rows, the same score, one row carrying both
    /// contributions. What is refused is the *pair of declarations* that says
    /// those entities are in two disjoint blocks while the rows say otherwise.
    /// A host that finds this refusal firing has a tagging that does not
    /// describe its corpus, and widening the declaration is a one-line fix that
    /// costs only the early certification the narrower claim would have bought.
    ///
    /// # Why the answer is not silently widened instead
    ///
    /// Because the declaration has already been *used*. Fusion certifies
    /// candidates early on the strength of it, so by the time this row arrives
    /// an earlier answer may already have been emitted on the assumption this
    /// stream would never name it — and quietly merging the late contribution
    /// would put a score in the caller's hands that its own provenance
    /// contradicts. The same reasoning
    /// [`Self::DuplicateItem`] gives for refusing rather than dropping applies
    /// here: the evidence has been shown to be unreliable, and a plausible
    /// answer computed from it is the worst of the available outcomes.
    ///
    /// # All three fields, because each answers a different question
    ///
    /// The item says *what*, the stratum says *who broke it*, and `named_by`
    /// says *against what* — the stratum whose own declaration, already
    /// applied, put the candidate out of this one's reach. Without the third a
    /// reader sees a producer refused for naming one of its own documents and
    /// has no way to find the declaration it collided with.
    #[error(
        "stream for stratum {stratum} named item {item:?}, which its declared candidate domains \
         cannot reach; stratum {named_by} already named it"
    )]
    OutsideDeclaredDomain {
        /// The candidate's canonical text.
        item: String,
        /// The stratum whose stream named it outside its declaration, exactly
        /// as that stream was tagged when it was handed to fusion.
        stratum: String,
        /// A stratum that had already named the candidate, and whose own
        /// declared domains are what this one's cannot meet.
        ///
        /// Where several strata had named it, this is the first one whose
        /// declaration is genuinely disjoint from the offender's, in the order
        /// the streams were handed to fusion — the witness that makes the
        /// contradiction visible, rather than an arbitrary member of the set.
        named_by: String,
    },

    /// A producer's contribution does not equal the profile's declared
    /// reciprocal-rank value for its stratum and rank. The fusion engine
    /// recomputes the contribution from the profile and refuses a mismatch
    /// rather than fuse a number the profile did not authorize.
    ///
    /// # Non-increase with rank is enforced here, and only here
    ///
    /// Fusion's certification argument depends on contributions being
    /// monotonically non-increasing with rank: the threshold summed over the
    /// stream heads is an upper bound only while they are. That law has no
    /// separate check, because after this one there is nothing left to check.
    /// Ranks reach the comparison already held contiguous and ascending
    /// ([`Self::OutOfOrderRanks`], [`Self::NonContiguousRanks`]), the stratum's
    /// weight is fixed for the whole stream, and
    /// `reciprocal_rank::contribution_under(decay, weight, rank)` is
    /// non-increasing in the rank for every rule and every weight a
    /// [`FusionProfile`](crate::FusionProfile) admits — a profile refuses a
    /// weight at or below zero, and the property is proven over the surviving
    /// domain by
    /// `reciprocal_rank::tests::every_decay_rule_is_non_increasing_in_the_rank`.
    /// So a value that rises with rank is necessarily a value the profile did
    /// not compute, and it is refused by this variant, as the wrong number it
    /// is. A separate "your contribution rose" refusal would blame the stream's
    /// shape for a wrong number, and nothing — conforming or hostile — could
    /// ever reach it.
    ///
    /// Equality between adjacent ranks is **not** a violation. The contribution
    /// is the consumer's own function of the rank, so two adjacent ranks carry
    /// one value exactly when the profile's fixed-point decay has stopped
    /// separating them at that depth. The answer stays correct and
    /// deterministic there — the declared tie-break is total — at a lower rank
    /// resolution, which the fused trailer reports per stratum rather than
    /// refusing.
    #[error("stream contribution {got:?} does not match the profile's {expected:?}")]
    ContributionMismatch {
        /// The value the profile computes for the stratum and rank.
        expected: Fixed,
        /// The value the producer supplied.
        got: Fixed,
    },

    /// A producer supplied a terminal receipt that contradicts what it emitted
    /// — declaring fewer rows than were pulled, or claiming it emitted none
    /// after it emitted some.
    #[error("producer receipt is inconsistent: declared {declared}, actually emitted {actual}")]
    ForgedReceipt {
        /// The row count the receipt declared (zero for the zero-row statuses).
        declared: u64,
        /// The row count fusion actually pulled.
        actual: u64,
    },

    /// A producer failed after it had already emitted rows. A clean
    /// [`ProducerReceipt`] cannot follow rows; this is the only honest way to
    /// report a mid-stream failure.
    #[error("producer failed after emitting {rows_before} row(s)")]
    ErrorAfterRows {
        /// How many rows the producer emitted before failing.
        rows_before: u64,
    },

    /// A producer did not terminate within its declared contribution budget.
    /// The producer reports this itself; fusion never invents an end for it.
    #[error("producer did not terminate")]
    NeverEndingSource,

    /// A row's contribution could not be formed at all, so the producer has no
    /// value it could honestly emit for that rank.
    ///
    /// This is the one variant that is not a producer's *misbehaviour*: it is a
    /// producer reporting that the fusion law it was asked to compute under
    /// cannot produce a number for this row — the fixed-point arithmetic left
    /// its range. Emitting a wrapped or substituted value instead would be a
    /// wrong order presented as a right one, which is what the whole protocol
    /// exists to prevent.
    ///
    /// No profile this crate can build reaches it: a validated
    /// [`FusionProfile`](crate::FusionProfile) fixes `K >= 1` and a weight whose
    /// admitted ceiling already fits, and a contribution is that weight times a
    /// reciprocal at most one, so the product cannot leave the range. That is an
    /// argument, though, and this variant is what keeps it from having to be an
    /// assertion: the conversion from a rank to a contribution is total, it runs
    /// on a library path a caller reaches with its own stream, and a wrong
    /// argument here should fail that caller's request rather than abort its
    /// process.
    #[error("no contribution can be formed at rank {rank}: {reason}")]
    UncomputableContribution {
        /// The 1-based rank whose contribution could not be formed.
        rank: u64,
        /// The arithmetic refusal, rendered.
        reason: String,
    },
}

/// A producer's ranked rows, pulled one at a time.
///
/// `next` yields `(rank, contribution, item)` in rank order and `Ok(None)` once
/// the stream is exhausted; `receipt` then reports how it ended. Ranks are
/// 1-based and contiguous. Contributions are the profile's reciprocal-rank
/// values — a conforming producer computes them with
/// [`contribution`](crate::contribution). Fusion re-derives every one of them
/// from `(decay rule, K, weight, rank)` and refuses a stream that supplies a
/// different number ([`ProtocolError::ContributionMismatch`]), which is also
/// what holds the sequence non-increasing with rank: the profile's own curve
/// never rises, so a rising value is a value the profile did not compute.
// The trait is consumed only by `FusionStream` in this crate; its futures are
// awaited in the same task and never cross a thread boundary, so the `Send`
// bound the lint wants to express would add nothing. Making it a required bound
// would instead exclude legitimate single-threaded producers.
#[allow(async_fn_in_trait)]
pub trait RankedStream {
    /// The item a row carries. It must round-trip through the caller's canonical
    /// term lexical so the layer can order and report it without parsing RDF.
    type Item: Clone + Ord + Into<Term>;

    /// Pull the next row, or `Ok(None)` when the stream is exhausted.
    ///
    /// # Errors
    ///
    /// A [`ProtocolError`] when the producer cannot report its own rows
    /// honestly — for example [`ProtocolError::ErrorAfterRows`] after a failure
    /// that followed emitted rows, or [`ProtocolError::NeverEndingSource`].
    async fn next(&mut self) -> Result<Option<(u64, Fixed, Self::Item)>, ProtocolError>;

    /// Report how the stream ended. Called only after `next` returned
    /// `Ok(None)`.
    ///
    /// # Errors
    ///
    /// A [`ProtocolError`] when the producer cannot produce a consistent
    /// receipt.
    async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError>;

    /// The promises this stream makes about its rows: its duplicate handling,
    /// and which blocks of the candidate universe it may name.
    ///
    /// Read once by [`FusionStream::new`](crate::FusionStream::new), before any
    /// row is pulled, and then applied to every row of this stream. A producer
    /// registered through the seam reports exactly what it declared — see
    /// [`StreamContract::declared`] — and a stream a caller assembled by hand
    /// states its own.
    ///
    /// There is deliberately **no default**, which is where this differs from
    /// [`plan_id`](Self::plan_id). A stream may honestly descend from no plan,
    /// so `None` is an answer; a stream cannot honestly decline to say whether
    /// it repeats items, because it either does or it does not and the consumer
    /// keeps a different amount of state for each. A default would be this
    /// layer fabricating a declaration the producer never made, and then holding
    /// the producer to it.
    ///
    /// The domain term has no default for a subtler reason, because it does
    /// have an obviously safe value —
    /// [`CandidateDomains::Unrestricted`] licenses nothing and is what every
    /// stream effectively said before the term existed. It is still written
    /// rather than assumed: the value is a *promise about the producer's
    /// corpus*, the host is the only party that knows it, and a consumer that
    /// filled it in would be choosing, on the host's behalf, between an answer
    /// that costs a full drain and one that might be missing a contribution.
    /// A caller that means "anything" says so in one word.
    fn contract(&self) -> StreamContract;

    /// The pinned plan these rows descend from, when the stream has one.
    ///
    /// This is how a plan's identity reaches the answer: [`execute`] tags each
    /// stream it returns with the plan its unit was compiled from, and
    /// [`fuse`](crate::fuse) reads the tag back off the streams it is handed and
    /// puts it in the [`FusionTrailer`](crate::FusionTrailer). The identity in a
    /// fused answer is therefore the one that travelled the pipeline, not one
    /// re-fetched from the plan at the end, and a stream whose tag changed on
    /// the way is a stream `fuse` refuses to fuse beside its siblings
    /// ([`FusionError::PlanIdMismatch`](crate::FusionError::PlanIdMismatch)).
    ///
    /// The default is `None`, which is the honest answer for a producer that
    /// descends from no plan at all — a hand-built stream, or one a caller
    /// assembled outside the ladder. Such a stream still fuses; the fused
    /// answer simply names no pinned plan, because no single plan produced it.
    ///
    /// [`execute`]: crate::execute
    fn plan_id(&self) -> Option<crate::id::PlanId> {
        None
    }

    /// What the index behind these rows attests: which generation answered, and
    /// whether that generation was whole.
    ///
    /// Read once by [`FusionStream::new`](crate::FusionStream::new), **before
    /// any row is pulled**, and carried into the fused trailer
    /// ([`FusionTrailer::attestations`](crate::FusionTrailer::attestations)).
    /// Reading it first is reading the truth for the whole read rather than a
    /// convenience: a generation is pinned when the index is opened, so the
    /// version that answers row one is the version that answers row `n`, and
    /// asking at the end would ask a stream that a bounded fusion may have
    /// stopped mid-read — or, worse, would let a top-k stop *overwrite* the
    /// answer. That is the same failure the two identities above already avoid
    /// by travelling with the stream instead of being re-fetched from the
    /// registry at the end.
    ///
    /// # Why this is not a [`ProducerReceipt`] variant
    ///
    /// An incomplete index is not a read ending. Every [`ProducerReceipt`]
    /// variant answers "what stopped this read", and "a shard of my index
    /// failed to load" answers a different question: it is true of the whole
    /// invocation from the instant it opened, and it stays true whichever way
    /// the read then ends. Made terminal it would be *destroyed* by a top-k
    /// stop, because a bounded fusion never collects a receipt from a stream it
    /// stopped — the incompleteness would vanish exactly in the runs where the
    /// bound mattered. Held as an attestation, both facts survive: the stratum
    /// reports the bounded stop it got and the short index it had.
    ///
    /// # Why this has a default and [`contract`](Self::contract) does not
    ///
    /// The default is [`PfAttestation::UNDECLARED`], for precisely the reason
    /// [`plan_id`](Self::plan_id) defaults to `None`: a stream may honestly
    /// descend from no index at all — a hand-built stream, a computation over
    /// its arguments, a walk of the dataset already being queried — and
    /// "nothing to name" is that stream's true answer rather than a gap in it.
    /// The absence stays an absence all the way out, too: `Undeclared` is
    /// silence, never a certificate that the index was current or whole (see
    /// [`ServiceLevel`](purrdf_sparql_eval::ServiceLevel), which has no
    /// `Whole` variant on purpose).
    ///
    /// [`contract`](Self::contract) is the opposite case and is defaulted
    /// nowhere. A stream either repeats items or it does not; there is no third
    /// state to be honestly silent about, and the consumer holds different
    /// state for each answer, so a default there would be this layer inventing
    /// a declaration and then enforcing it. A default here invents nothing — it
    /// says the producer said nothing, which is what happened.
    fn attestation(&self) -> PfAttestation {
        PfAttestation::UNDECLARED
    }
}
