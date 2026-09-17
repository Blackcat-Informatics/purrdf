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

use purrdf_text::Fixed;

use crate::iri::Term;

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

    /// The same item was emitted twice within one stream.
    #[error("stream emitted item {item:?} more than once")]
    DuplicateItem {
        /// The repeated item's canonical text.
        item: String,
    },

    /// A producer contribution rose from one rank to the next. Contributions
    /// must be monotonically non-increasing with rank, or the threshold that
    /// bounds fusion would not be an upper bound.
    #[error("stream contribution rose from {previous:?} to {got:?} with rank")]
    NonMonotoneContribution {
        /// The previous rank's contribution.
        previous: Fixed,
        /// The contribution that rose above it.
        got: Fixed,
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
}
