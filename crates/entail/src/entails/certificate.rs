// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! What one conclusion-directed question answered, and the run that answered it.
//!
//! # A verdict without its run is half an answer
//!
//! [`super::entails`] used to return an [`EntailmentOutcome`] alone. That verdict carries the
//! mechanism's own evidence — a mapping, a refutation, a frozen chase — and none of the
//! CHASE's: which rules fired, which constructs the run could not fully handle, which
//! calculus it ran under. So the one caller who most needed to know whether a `NotEntailed`
//! came out of a complete rule table was the caller with no way to ask, and the answer's
//! provenance had to be reconstructed from prose. [`EntailmentCertificate`] is both halves,
//! returned together, with no certificate-free entry point to route around: the alternative —
//! two functions, one of which drops the report — is how "the reasoner says no" comes to mean
//! "the reasoner had no rule for it".
//!
//! # It is NOT [`Certified<T>`](crate::Certified)
//!
//! That type pairs an answer with a [`DlCertificate`](crate::DlCertificate), whose `steps`,
//! `budget` and `decisions` count HYPERTABLEAU ROUNDS. No tableau runs here — this service is
//! a chase and a graph match — so those three would be zero for every answer, and
//! `DlCertificate::completeness` would read a zero-round, boundary-free run as
//! [`DlCompleteness::Decided`](crate::DlCompleteness::Decided): a strong claim minted by a
//! procedure that never executed. The chase already has its own certificate, with its own
//! completeness notion computed from its own rule inventory, and that is the one this type
//! carries.
//!
//! # Nothing here is stored that can be derived
//!
//! Three facts a naive shape would store as fields are functions of what is already present:
//!
//! * **The mechanism.** [`Self::mechanism`] reads the outcome — the warrant's arm for a
//!   `yes`, [`UndecidedReason::mechanism`] for an `undecided`, and
//!   [`EntailmentMechanism::StrictTable`] for a `no`, because refuting needs a completeness
//!   claim and only the rule table has one.
//! * **Budget exhaustion.** [`Self::is_budget_exhausted`] reads the outcome too: a lane that
//!   stopped early says so in its [`UndecidedReason`], and every other refusal to run is an
//!   [`EntailError`](crate::EntailError) that produces no certificate at all.
//! * **Completeness.** It is not a field of this type, of the report inside it, or of
//!   [`DlCertificate`](crate::DlCertificate). [`ReasoningReport::completeness`] computes the
//!   chase's from the regime and the boundary list, and the ENTITLEMENT half — whether a
//!   failed match refutes — is the outcome's own three-way split. A stored copy of either
//!   could disagree with the evidence beside it, and a disagreement no reader can adjudicate
//!   is the failure this crate's whole certificate discipline exists against.
//!
//! [`EntailmentOutcome`]: super::EntailmentOutcome
//! [`UndecidedReason`]: super::UndecidedReason
//! [`UndecidedReason::mechanism`]: super::UndecidedReason::mechanism

use crate::Regime;
use crate::entails::{EntailmentMechanism, EntailmentOutcome, EntailmentWarrant};
use crate::report::ReasoningReport;

/// A conclusion-directed verdict, the mechanism's evidence for it, and the chase that ran
/// underneath.
///
/// See the [module docs](self) for why this is not [`Certified<T>`](crate::Certified) and why
/// it stores no completeness, no mechanism tag and no exhaustion flag of its own.
#[derive(Debug, Clone)]
pub struct EntailmentCertificate {
    /// The verdict, carrying the mechanism's own evidence.
    outcome: EntailmentOutcome,
    /// What the chase underneath did, with the mechanism attached.
    report: ReasoningReport,
}

impl EntailmentCertificate {
    /// Pair `outcome` with the `report` of the chase that produced it.
    ///
    /// The ONE constructor, and the one place a mechanism is ever computed: it is derived
    /// from `outcome` here and attached to `report`, so
    /// [`ReasoningReport::mechanism`] can never name a mechanism other than the one that
    /// answered. There is no parameter for it, exactly as there is no `completeness`
    /// parameter on [`ReasoningReport::new`] and no `contract_hash` one.
    pub(crate) fn new(outcome: EntailmentOutcome, report: ReasoningReport) -> Self {
        let mechanism = mechanism_of(&outcome);
        Self {
            outcome,
            report: report.with_mechanism(mechanism),
        }
    }

    /// The verdict: entailed, not entailed, or undecided.
    #[must_use]
    pub const fn outcome(&self) -> &EntailmentOutcome {
        &self.outcome
    }

    /// What the chase underneath did — its boundaries, the rules it fired, the extensions its
    /// calculus states, its budget, and its contract hash.
    ///
    /// The report a [`materialize`](crate::materialize) of the same premise under the same
    /// regime would have produced, with one line added: [`ReasoningReport::mechanism`] is
    /// `Some` here and `None` there.
    #[must_use]
    pub const fn report(&self) -> &ReasoningReport {
        &self.report
    }

    /// The regime the question was asked under.
    ///
    /// Read from the report rather than stored, so it is the regime the chase actually ran.
    #[must_use]
    pub const fn regime(&self) -> Regime {
        self.report.regime()
    }

    /// WHICH of the six mechanisms answered.
    ///
    /// Derived from [`Self::outcome`] on every call. [`EntailmentMechanism::StrictTable`] is
    /// the regime's own rule table deciding the question — a positive claim, and the only
    /// mechanism a [`NotEntailed`](super::EntailmentOutcome::NotEntailed) can carry.
    #[must_use]
    pub const fn mechanism(&self) -> EntailmentMechanism {
        mechanism_of(&self.outcome)
    }

    /// The evidence for a `yes`, or `None` for the other two outcomes.
    ///
    /// A convenience over matching [`Self::outcome`], and the argument
    /// [`verify`](super::verify) wants: a warrant exists exactly when there is something to
    /// re-decide.
    #[must_use]
    pub const fn warrant(&self) -> Option<&EntailmentWarrant> {
        match &self.outcome {
            EntailmentOutcome::Entailed(warrant) => Some(warrant),
            EntailmentOutcome::NotEntailed(_) | EntailmentOutcome::Undecided(_) => None,
        }
    }

    /// Whether a lane stopped because a BUDGET ran out rather than because it had nothing
    /// left to find.
    ///
    /// Derived from the outcome: only [`UndecidedReason::RefutationBudget`] and
    /// [`UndecidedReason::FreezeBudget`] are budget exhaustion, and every other way a run can
    /// stop short — an exhausted match, an exhausted evaluation ceiling — is an
    /// [`EntailError`](crate::EntailError) that yields no certificate to read this off.
    ///
    /// `false` is therefore a CHECKED claim rather than a default: the mechanisms all ran to
    /// completion and the answer is the answer, not the point they gave up at.
    ///
    /// [`UndecidedReason::RefutationBudget`]: super::UndecidedReason::RefutationBudget
    /// [`UndecidedReason::FreezeBudget`]: super::UndecidedReason::FreezeBudget
    #[must_use]
    pub const fn is_budget_exhausted(&self) -> bool {
        match &self.outcome {
            EntailmentOutcome::Undecided(reason) => reason.is_budget_exhausted(),
            EntailmentOutcome::Entailed(_) | EntailmentOutcome::NotEntailed(_) => false,
        }
    }

    /// Whether the question was DECIDED — entailed or not entailed, never undecided.
    ///
    /// The one bit a caller that must act on a verdict needs, and it is deliberately not a
    /// `bool` conversion on the outcome itself: `Undecided` collapsing into a `false` at a
    /// call site is the overclaim [`precondition`](super::precondition) exists to prevent,
    /// and a method that says `is_decided` cannot be mistaken for one that says "no".
    #[must_use]
    pub const fn is_decided(&self) -> bool {
        match &self.outcome {
            EntailmentOutcome::Entailed(_) | EntailmentOutcome::NotEntailed(_) => true,
            EntailmentOutcome::Undecided(_) => false,
        }
    }

    /// Take the two halves apart.
    #[must_use]
    pub fn into_parts(self) -> (EntailmentOutcome, ReasoningReport) {
        (self.outcome, self.report)
    }
}

/// The mechanism `outcome` came out of — the single derivation every accessor above shares.
///
/// A free function rather than a method on [`EntailmentOutcome`] because it is this module's
/// claim about the outcome, not the outcome's about itself: the enum is the three-way answer,
/// and which lane produced it is a fact the certificate carries.
const fn mechanism_of(outcome: &EntailmentOutcome) -> EntailmentMechanism {
    match outcome {
        EntailmentOutcome::Entailed(warrant) => warrant.mechanism(),
        // No mechanism ever refutes: each hands its verdict back unchanged and the
        // precondition decides what a failed search meant, so a refutation is always the rule
        // table's own.
        EntailmentOutcome::NotEntailed(_) => EntailmentMechanism::StrictTable,
        EntailmentOutcome::Undecided(reason) => reason.mechanism(),
    }
}
