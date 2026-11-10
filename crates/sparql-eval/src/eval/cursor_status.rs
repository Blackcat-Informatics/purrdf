// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original query completion certificates without a retained answer payload.

/// The delivered rows covered by a stopped query's original certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPrefix {
    /// Number of rows delivered to this consumer.
    pub rows: usize,
    /// Whether the original certificate guarantees their positions.
    pub positional: bool,
}

/// What the original execution established when the consumer settled it.
#[derive(Debug, Clone)]
pub enum SelectCompletion {
    /// Every answer row was read and execution finished successfully.
    Complete,
    /// The original query stopped under a caller-selected governor.
    Truncated {
        /// The original typed governor trip.
        tripped: purrdf_core::TrippedGovernor,
        /// The original lower, upper or withheld bound over the delivered rows.
        answers: crate::PartialAnswers<CursorPrefix>,
    },
    /// The consumer stopped before reading the execution's end.
    ConsumerStopped,
}

/// A settled read receipt; it owns no second copy of the answer rows.
#[derive(Debug, Clone)]
pub struct SelectSettlement {
    /// Whether all rows were delivered or why the read ended sooner.
    pub completion: SelectCompletion,
    /// Number of rows actually handed to the consumer.
    pub delivered: usize,
    /// Delivered rows in the original established evidence partition.
    pub established: usize,
    /// Delivered rows in the original undecided partition.
    pub undecided: usize,
    /// The original counters and trip, read once at settlement.
    pub governors: Option<purrdf_core::GovernorEvidence>,
}

/// Availability of the caller-requested native semantic answer status.
#[derive(Debug, Clone, Copy)]
pub enum CursorAnswerStatus<'a> {
    /// No semantic answer status was requested for this read.
    Unrequested,
    /// The consumer left an unread suffix, so no final semantic status is claimed.
    UnreadSuffix,
    /// The original accumulator completed enumeration or proved its requested verdict.
    Settled(purrdf_core::evidence::AnswerStatus<'a>),
}
