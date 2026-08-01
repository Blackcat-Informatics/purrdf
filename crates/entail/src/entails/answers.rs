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
//! # A LANE NOT RUN IS A LIMIT, NOT A SILENCE
//!
//! The limits are not only the rule table's own completeness conditions. A pattern with a
//! projected variable is enumerated by matching the closure, and the five mechanisms beyond
//! the table — refutation, freeze, comprehension, reflexivity, datatype containment — are not
//! run for it, because a projected variable over what any of them decides is a different
//! question: "which individuals is `a` entailed to differ from?" needs a refutation per
//! candidate over the whole domain.
//!
//! That argument licenses not running them. It does NOT license reporting the resulting empty
//! row set as exhaustive, which is what an empty limit list claims. So every lane is asked
//! what it RECOGNIZES in the question — its own whitelist, over syntax and the closure's
//! index, at no chase — and each that reads anything contributes an
//! [`UndecidedReason::ConstructNotRead`] naming itself and the construct. `?x
//! owl:differentFrom ex:Peter` is therefore an INCOMPLETE empty answer that names the
//! refutation lane, rather than an exhaustive one that names nothing.
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
    /// `mechanism` is a parameter because it is now a MEASUREMENT rather than a definition. A
    /// pattern with something to project is enumerated by matching the closure and nothing
    /// else, so it is [`EntailmentMechanism::StrictTable`]; a pattern with NOTHING to project
    /// is a conclusion graph and is routed through the same fold [`entails`](super::entails)
    /// runs, so it can be answered by any of the seven — a `owl:differentFrom` between two
    /// named individuals is `refutation`, and a conclusion needing two lanes is `composite`.
    /// Hard-coding the table's name here is what let this service render `strict-table` beside
    /// an answer the table had not reached.
    ///
    /// It is attached to `report` rather than stored beside it, so [`Self::mechanism`] and
    /// [`Self::report`] read one value and cannot disagree.
    pub(crate) fn new(
        regime: Regime,
        vars: Vec<String>,
        rows: Vec<Vec<TermValue>>,
        limits: Vec<UndecidedReason>,
        report: ReasoningReport,
        mechanism: EntailmentMechanism,
    ) -> Self {
        Self {
            regime,
            vars,
            rows,
            limits,
            report: report.with_mechanism(mechanism),
        }
    }

    /// WHICH of the seven mechanisms answered.
    ///
    /// Read off [`Self::report`] rather than stored beside it. For a pattern with something to
    /// project this is always [`EntailmentMechanism::StrictTable`]; for one with nothing to
    /// project it is whichever mechanism [`entails`](super::entails)'s own fold reached, which
    /// is what makes the two entry points render the same answer to the same question.
    ///
    /// # Panics
    ///
    /// Never: the crate-internal constructor is the only producer and it always attaches one.
    #[must_use]
    pub fn mechanism(&self) -> EntailmentMechanism {
        self.report
            .mechanism()
            .expect("the one constructor attaches the mechanism that answered")
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
    /// Its [`ReasoningReport::mechanism`] is [`Self::mechanism`] — the mechanism that actually
    /// answered, which is a MEASUREMENT and not a constant. A pattern with something to
    /// project is [`super::EntailmentMechanism::StrictTable`], because the five mechanisms
    /// beyond the rule table are not run for one: a projected variable over what any of them
    /// decides would be a different question than the one this service answers, and the fact
    /// that one of them WOULD have been needed reaches the caller as a limit instead. A
    /// pattern with nothing to project is a conclusion graph, is routed through the same fold
    /// [`entails`](super::entails) runs, and names whichever of the seven reached it.
    #[must_use]
    pub const fn report(&self) -> &ReasoningReport {
        &self.report
    }
}
