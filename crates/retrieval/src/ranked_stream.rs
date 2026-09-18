// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The ranked-stream input protocol.
//!
//! A stratum's producer emits its rows in rank order, one row at a time, and
//! declares when it is done. Fusion is only sound if those declarations are
//! true, so the boundary is validated: the fusion engine checks rank contiguity
//! and monotonicity as rows arrive and compares each producer's terminal receipt
//! against what it actually emitted. A producer that cannot describe its own
//! completeness cleanly reports it through this channel; it never gets to return
//! a plausible-looking answer that quietly omitted a row.
//!
//! # Validated against what the producer declared, not against one fixed law
//!
//! Two of those checks are not absolute, because the registry does not state
//! them absolutely. A ranked producer declares its rank ordering and its
//! duplicate handling where it is registered
//! ([`RankedDeclaration`]), and the consumer of its rows is this layer — so the
//! consumer reads both and holds the stream to the promise it actually made.
//! [`StreamContract`] is that pair, carried with the stream through
//! [`RankedStream::contract`], and every refusal below that names an ordering or
//! a repeat says which declaration it was measured against.

use purrdf_sparql_eval::{DuplicatePolicy, RankOrdering, RankedDeclaration};
use purrdf_text::Fixed;

use crate::iri::Term;

/// The two promises a ranked producer makes about one invocation's rows: how
/// they are ordered, and whether an item may repeat.
///
/// Both halves are the producer's own — [`RankedDeclaration::ordering`] and
/// [`RankedDeclaration::duplicates`], supplied by the host where the producer is
/// registered. Neither is inferred and neither has a default: a stream either
/// repeats items or it does not, and the consumer's behaviour differs, so there
/// is no honest "unstated" answer the way there is for a stream that descends
/// from no plan ([`RankedStream::plan_id`]).
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
    /// The ordering guarantee the producer declared. Fusion holds a
    /// [`RankOrdering::StrictlyDescending`] stream to strict descent and a
    /// [`RankOrdering::NonIncreasing`] one to non-increasing.
    pub ordering: RankOrdering,
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
            ordering: declaration.ordering,
            duplicates: declaration.duplicates,
        }
    }

    /// A contract stated directly, for a stream a caller built itself.
    #[must_use]
    pub const fn new(ordering: RankOrdering, duplicates: DuplicatePolicy) -> Self {
        Self {
            ordering,
            duplicates,
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

    /// A producer contribution rose from one rank to the next. Contributions
    /// must be monotonically non-increasing with rank, or the threshold that
    /// bounds fusion would not be an upper bound.
    ///
    /// Raised under either [`RankOrdering`], because neither admits a
    /// contribution that rises.
    #[error("stream contribution rose from {previous:?} to {got:?} with rank")]
    NonMonotoneContribution {
        /// The previous rank's contribution.
        previous: Fixed,
        /// The contribution that rose above it.
        got: Fixed,
    },

    /// A producer that declared [`RankOrdering::StrictlyDescending`] emitted the
    /// same contribution at two adjacent ranks.
    ///
    /// The declaration says every row has an unambiguous rank; two adjacent
    /// ranks carrying one contribution is exactly the condition under which the
    /// fused sum stops separating them, so the stream is no longer strictly
    /// descending in the only quantity fusion sums. A producer whose ranks may
    /// legitimately tie declares [`RankOrdering::NonIncreasing`] and is admitted
    /// here, which is the difference between the two spellings.
    ///
    /// A plan admitted through [`compile`](crate::compile) cannot reach this: a
    /// contribution is the profile's own function of the rank, and
    /// [`AdmissionError::DepthBeyondMonotoneRange`](crate::AdmissionError::DepthBeyondMonotoneRange)
    /// already refuses a per-stratum depth past the rank at which that function
    /// stops separating adjacent ranks. This is the same claim held against a
    /// stream that reached fusion without passing the waist — a hand-built one,
    /// or one read deeper than its weight can order.
    #[error(
        "stream declared strictly descending contributions but repeated {value:?} at rank {rank}"
    )]
    RepeatedContribution {
        /// The 1-based rank that repeated its predecessor's contribution.
        rank: u64,
        /// The contribution both ranks carried.
        value: Fixed,
    },

    /// A producer's contribution does not equal the profile's declared
    /// reciprocal-rank value for its stratum and rank. The fusion engine
    /// recomputes the contribution from the profile and refuses a mismatch
    /// rather than fuse a number the profile did not authorize.
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
/// [`contribution`](crate::contribution) — and must be monotonically
/// non-increasing with rank. Fusion re-verifies both and refuses a stream that
/// violates either.
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

    /// The two promises this stream makes about its rows: its rank ordering and
    /// its duplicate handling.
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
