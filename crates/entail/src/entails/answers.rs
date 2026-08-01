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
//! # The rows arrive WITH the run that produced them
//!
//! [`CertainAnswers::report`] is the chase underneath, carried for exactly the reason
//! [`EntailmentCertificate`](super::EntailmentCertificate) carries one. An empty row set is
//! the answer a caller is most likely to act on and the one that says least on its own:
//! whether it means "nothing is entailed" depends on which rule table ran, what that run
//! could not fully handle, and which calculus it was. All three are on the report, and there
//! is no entry point that returns rows without it.
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
use crate::entails::EntailmentMechanism;
use crate::entails::precondition::UndecidedReason;
use crate::report::ReasoningReport;

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
    /// What the chase underneath did, with the mechanism attached.
    report: ReasoningReport,
}

impl CertainAnswers {
    /// Assemble an answer set. Crate-internal: the only producer is the service that ran
    /// the mechanism, so a row cannot exist without the run that justifies it.
    ///
    /// The mechanism attached to `report` is not a parameter and never could be: this
    /// service runs the homomorphism and nothing else — the five lanes beyond it are
    /// [`entails`](super::entails)-only, for the reasons that module's docs set out — so the
    /// mechanism is [`EntailmentMechanism::StrictTable`] by definition, exactly as the
    /// report's contract hash is its regime's calculus by definition.
    pub(crate) fn new(
        regime: Regime,
        vars: Vec<String>,
        rows: Vec<Vec<TermValue>>,
        limits: Vec<UndecidedReason>,
        report: ReasoningReport,
    ) -> Self {
        Self {
            regime,
            vars,
            rows,
            limits,
            report: report.with_mechanism(EntailmentMechanism::StrictTable),
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

    /// What the chase underneath did — its boundaries, the rules it fired, the extensions
    /// its calculus states, its budget, and its contract hash.
    ///
    /// Carried for the same reason [`EntailmentCertificate`](super::EntailmentCertificate)
    /// carries one: rows without the run that produced them are half an answer. A caller
    /// reading an EMPTY row set beside `is_complete` needs to know which rule table produced
    /// the closure those rows were drawn from, and there is no second call to get it from —
    /// the alternative, an entry point that drops the report, is how "there are no answers"
    /// comes to mean "there were no rules".
    ///
    /// Its [`ReasoningReport::mechanism`] is always
    /// [`EntailmentMechanism::StrictTable`](super::EntailmentMechanism::StrictTable), and
    /// that is a claim rather than a placeholder: the five mechanisms beyond the rule table
    /// are [`entails`](super::entails)-only, each because a projected variable over what it
    /// decides would be a different question than the one this service answers.
    #[must_use]
    pub const fn report(&self) -> &ReasoningReport {
        &self.report
    }
}
