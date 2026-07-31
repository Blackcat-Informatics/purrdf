// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The answer set of a basic graph pattern, and what it is allowed to claim.
//!
//! # A certain answer is not merely an answer
//!
//! SPARQL's entailment regimes define the answers to a basic graph pattern as the CERTAIN
//! answers: the substitutions `σ` over the scoping graph's terms for which the knowledge
//! base entails `BGPσ` — true in every model, not merely present in one closure. Each row
//! below is therefore a claim of entailment, minted by the same mechanism and holding by
//! the same soundness argument as [`entails`](super::entails).
//!
//! # Rows are sound; ABSENCE is the part that needs a precondition
//!
//! Every row is a certain answer, unconditionally, because the mechanism that found it is
//! sound. What needs the completeness precondition is the claim a caller makes about a row
//! that is NOT there — "no such answer exists" — and that claim is only available when
//! [`CertainAnswers::is_complete`] holds.
//!
//! So the limits are carried WITH the rows rather than in a second call a caller can skip,
//! for the same reason [`materialize`](crate::materialize) has no report-free variant: the
//! cheap call wins, and a row set read as exhaustive when it is not is how an incomplete
//! procedure comes to be described as a decision.
//!
//! # There is no field to disagree with
//!
//! [`CertainAnswers`] stores no completeness flag. [`CertainAnswers::is_complete`] DERIVES
//! it from the limit list on every call, so "complete" beside a non-empty limit list is a
//! value no caller — inside this crate or outside it — has a constructor for. That is the
//! same construction [`ReasoningReport::completeness`](crate::ReasoningReport::completeness)
//! and [`DlCertificate::completeness`](crate::DlCertificate::completeness) use, and it is
//! used here for the same reason: a state that cannot be built needs no check.

use purrdf_core::TermValue;

use crate::Regime;
use crate::entails::precondition::UndecidedReason;

/// The certain answers of a basic graph pattern under an entailment regime.
///
/// A relation: [`vars`](Self::vars) names the columns, each row of [`rows`](Self::rows) is
/// one certain answer, positionally aligned to them.
#[derive(Debug, Clone)]
pub struct CertainAnswers {
    /// The regime the answers are certain under.
    regime: Regime,
    /// The projected variables, in the order the query wrote them.
    vars: Vec<String>,
    /// One row per certain answer, deduplicated and ordered by the row itself.
    rows: Vec<Vec<TermValue>>,
    /// Why the row set may not be exhaustive. EMPTY is the claim that it is.
    limits: Vec<UndecidedReason>,
}

impl CertainAnswers {
    /// Assemble an answer set. Crate-internal: the only producer is the service that ran
    /// the mechanism, so a row cannot exist without the run that justifies it.
    pub(crate) const fn new(
        regime: Regime,
        vars: Vec<String>,
        rows: Vec<Vec<TermValue>>,
        limits: Vec<UndecidedReason>,
    ) -> Self {
        Self {
            regime,
            vars,
            rows,
            limits,
        }
    }

    /// The regime these answers are certain under.
    #[must_use]
    pub const fn regime(&self) -> Regime {
        self.regime
    }

    /// The projected variables, in query order — the columns a row is read against.
    #[must_use]
    pub fn vars(&self) -> &[String] {
        &self.vars
    }

    /// The rows, each positionally aligned to [`vars`](Self::vars).
    ///
    /// Deduplicated and in a total order that is a function of the terms alone, so two runs
    /// over one input produce the same sequence.
    #[must_use]
    pub fn rows(&self) -> &[Vec<TermValue>] {
        &self.rows
    }

    /// How many certain answers were found.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether no certain answer was found.
    ///
    /// Read together with [`is_complete`](Self::is_complete): empty AND complete means
    /// there is no answer; empty and incomplete means none was FOUND, which is a different
    /// statement and the one an incomplete procedure is entitled to make.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Why the row set may not be exhaustive — empty when it is.
    #[must_use]
    pub fn limits(&self) -> &[UndecidedReason] {
        &self.limits
    }

    /// Whether the row set is EXHAUSTIVE: every certain answer is present, so a
    /// substitution absent from it is not one.
    ///
    /// Derived from [`limits`](Self::limits) rather than stored beside it, so the two
    /// cannot disagree.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.limits.is_empty()
    }
}
