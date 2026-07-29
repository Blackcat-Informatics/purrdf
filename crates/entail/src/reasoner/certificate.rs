// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! What a Description-Logic reasoning service actually decided — the DL lane's own
//! completeness notion.
//!
//! # Why the chase's [`Completeness`](crate::Completeness) will not do
//!
//! [`Completeness::for_regime`](crate::Completeness::for_regime) is
//! [`rules`](crate::rules) minus [`implemented`](crate::implemented): a difference of two
//! rule tables. The DL lane has no rule table. `rules(Regime::OwlDirect)` and
//! `implemented(Regime::OwlDirect)` are both empty, so that subtraction is `∅ ∖ ∅ = ∅` and
//! reports [`Completeness::Exact`](crate::Completeness::Exact) — for a tableau, for every
//! input, including one whose axioms it could not read. Reusing it here would manufacture
//! an overclaim out of a vacuous truth, which is why the DL services report [`DlCertificate`]
//! instead.
//!
//! # What this measures instead
//!
//! A tableau's answer is complete for the clause set it was actually handed, inside the
//! search budget it actually ran in. Both halves can fail, and this certificate can report
//! either:
//!
//! * **The clause set.** The OWL-2-RDF reverse mapping is a total function over the
//!   reserved vocabulary — this crate's OWL 2 construct registry marks each of its terms handled,
//!   an operand, inert, or BOUNDED — and a bounded term's axiom does not become a DL
//!   clause. The knowledge base then describes a strictly smaller ontology than the caller
//!   supplied, and [`DlCertificate::boundaries`] names every construct that was left out.
//! * **The budget.** Every decision runs under a deterministic step cap
//!   (a pure function of the knowledge base's size). A search that reaches it has closed some
//!   branches and not others; reporting "no branch succeeded *yet*" as "no model exists"
//!   would turn a resource limit into an entailment, so an exhausted run answers
//!   [`Verdict::Unknown`] and drives the certificate to
//!   [`DlCompleteness::BudgetExhausted`].
//!
//! There is deliberately no fourth state and no way to construct a certificate that omits
//! both signals: the crate-internal session type is the only producer, it derives the verdict from what the
//! tableau reported, and [`DlCertificate::overclaims`] is the gate a consumer can apply
//! for itself.
//!
//! # Determinism
//!
//! [`DlCertificate::boundaries`] is in [`Construct`] declaration order, never map order.
//! [`DlCertificate::steps`] is a count of saturation rounds, not a clock reading — so it
//! is identical run to run and on `wasm32`, where there is no clock to read.

use std::collections::BTreeSet;

use crate::owl_dl::Kb;
use crate::owl_dl::tableau::{Assumptions, Decision, decide};
use crate::report::{Boundary, Construct};

/// A three-valued answer from a step-bounded decision procedure.
///
/// The third value is not hedging: a tableau that stops at its step cap has explored part
/// of a search tree, and both `True` and `False` would be claims that part does not
/// support. Every boolean DL service answers this rather than `bool`, and the certificate
/// beside it says why an `Unknown` happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    /// The question is answered yes: the refutation closed every branch.
    True,
    /// The question is answered no: a clash-free completion witnesses a counter-model.
    False,
    /// The search stopped at its step cap before deciding. See
    /// [`DlCertificate::steps`] and [`DlCertificate::budget`].
    Unknown,
}

impl Verdict {
    /// Whether this is [`Verdict::True`] — `false` for BOTH of the other two.
    ///
    /// Named `is_true` rather than offered as a `bool` conversion so that an undecided
    /// answer cannot be silently read as a negative one at a call site.
    #[must_use]
    pub const fn is_true(self) -> bool {
        matches!(self, Self::True)
    }

    /// Whether the search decided the question either way.
    #[must_use]
    pub const fn is_decided(self) -> bool {
        !matches!(self, Self::Unknown)
    }
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::True => "true",
            Self::False => "false",
            Self::Unknown => "unknown",
        })
    }
}

/// How complete a DL service's answer is, w.r.t. the clause set the tableau decided.
///
/// See the [module docs](self) for why this is a separate notion from the chase's
/// [`Completeness`](crate::Completeness) rather than a reuse of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DlCompleteness {
    /// Every axiom of the ontology became a DL clause, and every tableau run this service
    /// made saturated inside its step cap.
    ///
    /// The strongest thing the DL lane can say: the answer is the OWL 2 Direct-Semantics
    /// answer for the ontology as supplied. A certificate reporting this beside a non-empty
    /// [`DlCertificate::boundaries`] is contradicting its own evidence; see
    /// [`DlCertificate::overclaims`].
    Decided,
    /// Every tableau run saturated, and the ontology ALSO carried at least one construct
    /// the reverse mapping bounds.
    ///
    /// The answer is sound and complete for the sub-ontology that was read. The bounded
    /// axioms are premises this run did not have, and premises can only ADD entailments —
    /// so a `True` here stays true for the full ontology, while a `False` means only "not
    /// entailed by what was read". [`DlCertificate::boundaries`] names each one.
    DecidedWithinBoundaries,
    /// At least one tableau run reached its step cap before deciding.
    ///
    /// Strictly the weakest state, and it takes precedence over the other two: a service
    /// that could not decide one sub-question has not decided the aggregate either. Every
    /// answer that IS reported alongside this is still sound — a refutation that closed is
    /// a refutation — but the answer set is not complete, and a boolean service reports
    /// [`Verdict::Unknown`] rather than guessing.
    BudgetExhausted,
}

impl DlCompleteness {
    /// Whether every tableau run decided its question.
    ///
    /// True for [`Self::Decided`] AND [`Self::DecidedWithinBoundaries`]: both say the
    /// SEARCH finished, and they differ only in whether the clause set was the whole
    /// ontology. A caller asking "is this the Direct-Semantics answer?" must check
    /// [`DlCertificate::boundaries`] too, which is exactly the distinction the second
    /// variant exists to make visible.
    #[must_use]
    pub const fn is_decided(&self) -> bool {
        matches!(self, Self::Decided | Self::DecidedWithinBoundaries)
    }
}

impl std::fmt::Display for DlCompleteness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Decided => "decided",
            Self::DecidedWithinBoundaries => "decided-within-boundaries",
            Self::BudgetExhausted => "budget-exhausted",
        })
    }
}

/// What one DL reasoning service call decided, and what it consumed deciding it.
///
/// Returned with every service answer through [`Certified`], never optional and never
/// behind a second entry point — the same discipline
/// [`ReasoningReport`](crate::ReasoningReport) imposes on the chase, for the same reason:
/// the interesting failure of a reasoner is a missing answer presented as a complete one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DlCertificate {
    /// How complete the answer is.
    completeness: DlCompleteness,
    /// The constructs the reverse mapping could not turn into DL clauses.
    boundaries: Vec<Boundary>,
    /// Saturation rounds consumed, summed over every tableau run the service made.
    steps: u64,
    /// The per-decision step cap each of those runs ran under.
    budget: u64,
    /// How many tableau runs the service made.
    decisions: u64,
}

impl DlCertificate {
    /// How complete the answer is.
    #[must_use]
    pub const fn completeness(&self) -> &DlCompleteness {
        &self.completeness
    }

    /// The constructs the reverse mapping could not turn into DL clauses, in
    /// [`Construct`] declaration order.
    #[must_use]
    pub fn boundaries(&self) -> &[Boundary] {
        &self.boundaries
    }

    /// Saturation rounds consumed, summed over every tableau run this service made.
    ///
    /// A MEASUREMENT, in the units the cap is denominated in, so a caller can see how
    /// close a run came to [`Self::budget`] without changing anything. It is a step count
    /// rather than an elapsed time: a clock reading is neither reproducible nor available
    /// on `wasm32`.
    #[must_use]
    pub const fn steps(&self) -> u64 {
        self.steps
    }

    /// The per-decision step cap every run of this service ran under.
    ///
    /// Per DECISION, not per service call: classification asks one subsumption question
    /// per ordered pair of named classes, and a budget shared across them would turn a
    /// large-but-easy ontology into a `BudgetExhausted` report for no reason. So the cap
    /// bounds each question and [`Self::steps`] reports the sum.
    #[must_use]
    pub const fn budget(&self) -> u64 {
        self.budget
    }

    /// How many tableau runs this service made.
    ///
    /// The denominator for [`Self::steps`], and the thing that makes a service's cost
    /// legible: `classify` over `n` named classes makes `n² + 1` of them.
    #[must_use]
    pub const fn decisions(&self) -> u64 {
        self.decisions
    }

    /// Whether this certificate claims more than its own evidence supports.
    ///
    /// True when [`DlCompleteness::Decided`] — the variant that means "and the whole
    /// ontology was read" — is reported alongside a non-empty [`Self::boundaries`].
    /// [`DlCompleteness::DecidedWithinBoundaries`] is the honest way to say the first half
    /// of that and does not trip the gate.
    ///
    /// No certificate this crate produces may return `true`; the crate's tests assert it
    /// for every service call they make. It is public so a consumer combining certificates
    /// from several calls can apply the same gate.
    #[must_use]
    pub fn overclaims(&self) -> bool {
        matches!(self.completeness, DlCompleteness::Decided) && !self.boundaries.is_empty()
    }
}

/// An answer plus the certificate of the run that produced it.
///
/// There is deliberately no certificate-free variant of any service. A caller that ignores
/// the certificate must still bind it, because the alternative — two entry points, one of
/// which discards the evidence — is how "the reasoner says no" comes to mean "the reasoner
/// ran out of steps".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Certified<T> {
    /// The service's answer.
    answer: T,
    /// The certificate of the run that produced it.
    certificate: DlCertificate,
}

impl<T> Certified<T> {
    /// Pair an answer with its certificate.
    pub(crate) const fn new(answer: T, certificate: DlCertificate) -> Self {
        Self {
            answer,
            certificate,
        }
    }

    /// The service's answer.
    pub const fn answer(&self) -> &T {
        &self.answer
    }

    /// The certificate of the run that produced it.
    pub const fn certificate(&self) -> &DlCertificate {
        &self.certificate
    }

    /// Take the answer, discarding the certificate.
    #[must_use]
    pub fn into_answer(self) -> T {
        self.answer
    }

    /// Split into the answer and its certificate.
    #[must_use]
    pub fn into_parts(self) -> (T, DlCertificate) {
        (self.answer, self.certificate)
    }
}

/// One service call's tableau budget, tally, and boundary list.
///
/// The single seam between a service and the tableau. Every decision goes through
/// [`Session::refutes`] or [`Session::decide`], so the step tally and the exhausted flag
/// cannot be bypassed, and [`Session::certificate`] is the only producer of a
/// [`DlCertificate`] — which is what makes a certificate that omits an exhausted run
/// unconstructible rather than merely discouraged.
pub(crate) struct Session<'a> {
    /// The knowledge base every decision is made against.
    kb: &'a Kb,
    /// The per-decision step cap.
    cap: u64,
    /// Steps consumed so far, summed over every decision.
    steps: u64,
    /// Decisions made so far.
    decisions: u64,
    /// Whether any decision reached the cap.
    exhausted: bool,
}

impl<'a> Session<'a> {
    /// Open a session over `kb` in which each decision may spend `cap` steps.
    pub(crate) const fn new(kb: &'a Kb, cap: u64) -> Self {
        Self {
            kb,
            cap,
            steps: 0,
            decisions: 0,
            exhausted: false,
        }
    }

    /// The knowledge base this session reasons over.
    pub(crate) const fn kb(&self) -> &'a Kb {
        self.kb
    }

    /// Run one tableau decision, tallying its cost.
    pub(crate) fn decide(&mut self, assumptions: &Assumptions<'_>) -> Decision {
        let decision = decide(self.kb, assumptions, self.cap);
        self.steps = self.steps.saturating_add(decision.steps);
        self.decisions += 1;
        self.exhausted |= decision.exhausted;
        decision
    }

    /// Whether `assumptions` REFUTE — i.e. whether the conclusion they negate is entailed.
    ///
    /// Entailment by refutation: `KB ⊨ α` exactly when `KB ∪ {¬α}` has no model. So an
    /// INCONSISTENT completion is [`Verdict::True`] (the conclusion holds), a clash-free
    /// one is [`Verdict::False`] (the completion is a counter-model), and an exhausted
    /// search is [`Verdict::Unknown`] — never `False`, which is the mistake this method
    /// exists to make impossible to write at a call site.
    pub(crate) fn refutes(&mut self, assumptions: &Assumptions<'_>) -> Verdict {
        let decision = self.decide(assumptions);
        if decision.exhausted {
            Verdict::Unknown
        } else if decision.consistent {
            Verdict::False
        } else {
            Verdict::True
        }
    }

    /// Seal the session into a certificate over `boundaries`.
    ///
    /// The completeness is derived, never passed in: an exhausted run is
    /// [`DlCompleteness::BudgetExhausted`] whatever else happened, a non-empty boundary
    /// list narrows [`DlCompleteness::Decided`] to
    /// [`DlCompleteness::DecidedWithinBoundaries`], and only a run with neither is
    /// `Decided`. That is why [`DlCertificate::overclaims`] cannot be true of anything
    /// this function returns — and the crate's tests assert it anyway, because an
    /// invariant nothing checks is a comment.
    pub(crate) fn certificate(&self, boundaries: &BTreeSet<Construct>) -> DlCertificate {
        let boundaries: Vec<Boundary> = Construct::ALL
            .into_iter()
            .filter(|construct| boundaries.contains(construct))
            .map(Boundary::of)
            .collect();
        let completeness = if self.exhausted {
            DlCompleteness::BudgetExhausted
        } else if boundaries.is_empty() {
            DlCompleteness::Decided
        } else {
            DlCompleteness::DecidedWithinBoundaries
        };
        DlCertificate {
            completeness,
            boundaries,
            steps: self.steps,
            budget: self.cap,
            decisions: self.decisions,
        }
    }
}
