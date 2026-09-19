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
//! # One fixed rank law, and one declared duplicate policy
//!
//! The rank check is absolute. Ranks are 1-based, contiguous and ascending for
//! every ranked stream there is, with no registration able to soften it, and the
//! fusion engine measures every row against the next rank it expects from that
//! stream ([`ProtocolError::OutOfOrderRanks`],
//! [`ProtocolError::NonContiguousRanks`]).
//!
//! The duplicate check is the one that is not absolute, because the registry
//! does not state it absolutely. A ranked producer declares its duplicate
//! handling where it is registered ([`RankedDeclaration`]), and the consumer of
//! its rows is this layer — so the consumer reads that declaration and holds the
//! stream to the promise it actually made. [`StreamContract`] carries it with
//! the stream through [`RankedStream::contract`], and the refusal below that
//! names a repeat says which declaration it was measured against.

use purrdf_sparql_eval::{DuplicatePolicy, RankedDeclaration};
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StreamContract {
    /// The duplicate handling the producer declared. A
    /// [`DuplicatePolicy::Unique`] stream is believed and a repeat is a protocol
    /// violation; a [`DuplicatePolicy::Allowed`] stream is de-duplicated by the
    /// consumer, which is what that policy says a consumer must do.
    pub duplicates: DuplicatePolicy,
}

impl StreamContract {
    /// The contract `declaration` states, verbatim.
    #[must_use]
    pub const fn declared(declaration: &RankedDeclaration) -> Self {
        Self {
            duplicates: declaration.duplicates,
        }
    }

    /// A contract stated directly, for a stream a caller built itself.
    #[must_use]
    pub const fn new(duplicates: DuplicatePolicy) -> Self {
        Self { duplicates }
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
    #[error("stream emitted item {item:?} more than once")]
    DuplicateItem {
        /// The repeated item's canonical text.
        item: String,
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

    /// The promise this stream makes about its rows: its duplicate handling.
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
}
