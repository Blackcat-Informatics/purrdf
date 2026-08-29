// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! What a Description-Logic reasoning service actually decided — the DL lane's own
//! completeness notion.
//!
//! # Why the chase's [`Completeness`](crate::Completeness) will not do
//!
//! [`Completeness::for_regime`](crate::Completeness::for_regime) is
//! [`rules`](crate::rules) minus [`implemented`](crate::implemented): a difference of two
//! rule tables. The DL lane has no FIXED rule table — its clause set is derived per
//! knowledge base. `rules(Regime::OwlDirect)` and
//! `implemented(Regime::OwlDirect)` are both empty, so that subtraction is `∅ ∖ ∅ = ∅` and
//! reports [`Completeness::Exact`](crate::Completeness::Exact) — for a hypertableau, for every
//! input, including one whose axioms it could not read. Reusing it here would manufacture
//! an overclaim out of a vacuous truth, which is why the DL services report [`DlCertificate`]
//! instead.
//!
//! # What this measures instead
//!
//! A hypertableau's answer is complete for the DL-clause set it was actually handed, inside
//! the search budget it actually ran in — and with this decision core the phrase is literal
//! rather than figurative: `owl_dl::clause` derives that clause set, and a bounded construct
//! is an axiom that never became one of its clauses. Both halves can fail, and this
//! certificate can report either:
//!
//! * **The clause set.** The OWL-2-RDF reverse mapping is a total function over the
//!   reserved vocabulary — this crate's OWL 2 construct registry marks each of its terms handled,
//!   an operand, inert, or BOUNDED — and a bounded term's axiom does not become a DL
//!   clause. The knowledge base then describes a strictly smaller ontology than the caller
//!   supplied, and [`DlCertificate::boundaries`] names every construct that was left out.
//! * **The budget.** Every decision runs under two deterministic caps, both pure functions of
//!   the knowledge base's size: a ROUND cap ([`DlCertificate::budget`]) and a WORK cap
//!   ([`DlCertificate::work_budget`]). Two, because a round is a pass rather than a unit of
//!   work — an ontology can make each round enormously more expensive without making the
//!   search take more rounds, and a rounds-only budget watches that case grind at a few
//!   percent of a ceiling it never reaches. A search that reaches EITHER has closed some
//!   branches and not others; reporting "no branch succeeded *yet*" as "no model exists"
//!   would turn a resource limit into an entailment, so an exhausted run answers
//!   [`Verdict::Unknown`] and drives the certificate to
//!   [`DlCompleteness::BudgetExhausted`] whichever cap it was.
//!
//! A search the caller's stop signal ended, rather than a cap, is not a fourth state: it
//! reaches the same [`DlCompleteness::BudgetExhausted`] a capped search does, because
//! `consistent` is exactly as meaningless under either — see
//! the crate-private search decision record. [`DlCertificate::stopped`] is where the two stay
//! tellable apart. There is no way to construct a certificate that omits any of its
//! signals: the crate-internal session type is the only producer, and it derives the
//! verdict from what the decision core reported.
//!
//! # There is no overclaim, because there is no field to disagree with
//!
//! The failure this certificate is built against is one that states
//! [`DlCompleteness::Decided`] — "the whole ontology was read, and every search finished" —
//! beside a non-empty [`DlCertificate::boundaries`] naming a construct that was NOT read. A
//! reader of such a certificate cannot tell which half to believe.
//!
//! That state is not detected here. It is UNREPRESENTABLE: [`DlCertificate`] stores no
//! completeness field at all, only the exhausted flag, the stopped flag and the boundary
//! list the reasoning session actually measured. [`DlCertificate::completeness`] COMPUTES
//! the verdict from those three on every call, so `Decided` beside a non-empty boundary
//! list is a value no caller — inside this crate or outside it — has a constructor for.
//! This crate's tests exercise the derivation over every reachable combination rather than
//! gating a predicate that could only ever answer `false`.
//!
//! # Determinism
//!
//! [`DlCertificate::boundaries`] is in [`Construct`] declaration order, never map order.
//! [`DlCertificate::steps`] is a count of saturation rounds and [`DlCertificate::work`] a
//! count of charged work units, not clock readings — so both are identical run to run and on
//! `wasm32`, where there is no clock to read. The same holds
//! of the three shape counters [`DlCertificate::peak_nodes`],
//! [`DlCertificate::disjunctions`] and [`DlCertificate::peak_depth`], which say where those
//! rounds went: all three are counts over the decision core's deterministic search, so a
//! certificate is byte-identical run to run in every field it renders.

use std::collections::BTreeSet;

use super::proof::{
    Claim, MAX_RECORDED_RUNS, Question, RunAssumptions, RunProof, ServiceProof, StopPoint,
    receipt_of,
};
use crate::owl_dl::Kb;
use crate::owl_dl::graph::{Assumptions, Budget, Decision};
use crate::owl_dl::hyper::{decide, decide_recording};
use crate::owl_dl::proof::{DlProof, ProofAnswer, contract_of};
use crate::report::{Boundary, Construct};

/// A three-valued answer from a step-bounded decision procedure.
///
/// The third value is not hedging: a search that stops at its round cap has explored part
/// of a search tree, and both `True` and `False` would be claims that part does not
/// support. Every boolean DL service answers this rather than `bool`, and the certificate
/// beside it says why an `Unknown` happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    /// The question is answered yes: the refutation closed every branch.
    True,
    /// The question is answered no: a clash-free completion witnesses a counter-model.
    False,
    /// The search stopped at one of its two caps before deciding. See
    /// [`DlCertificate::steps`]/[`DlCertificate::budget`] for the round cap and
    /// [`DlCertificate::work`]/[`DlCertificate::work_budget`] for the work cap.
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

/// How complete a DL service's answer is, w.r.t. the DL-clause set the hypertableau decided.
///
/// See the [module docs](self) for why this is a separate notion from the chase's
/// [`Completeness`](crate::Completeness) rather than a reuse of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DlCompleteness {
    /// Every axiom of the ontology became a DL-clause, and every hypertableau run this
    /// service made saturated inside both of its caps.
    ///
    /// The strongest thing the DL lane can say: the answer is the OWL 2 Direct-Semantics
    /// answer for the ontology as supplied. [`DlCertificate::completeness`] returns this
    /// variant only when [`DlCertificate::boundaries`] is empty, so a certificate reporting
    /// it beside a non-empty boundary list is not a state anything can construct.
    Decided,
    /// Every hypertableau run saturated inside both caps, and the ontology ALSO carried at
    /// least one construct the reverse mapping bounds.
    ///
    /// The answer is sound and complete for the sub-ontology that was read. The bounded
    /// axioms are premises this run did not have, and premises can only ADD entailments —
    /// so a `True` here stays true for the full ontology, while a `False` means only "not
    /// entailed by what was read". [`DlCertificate::boundaries`] names each one.
    DecidedWithinBoundaries,
    /// At least one hypertableau run reached its round cap or its work cap before deciding,
    /// OR the caller's stop signal ended one before it finished.
    ///
    /// Strictly the weakest state, and it takes precedence over the other two: a service
    /// that could not decide one sub-question has not decided the aggregate either. Every
    /// answer that IS reported alongside this is still sound — a refutation that closed is
    /// a refutation — but the answer set is not complete, and a boolean service reports
    /// [`Verdict::Unknown`] rather than guessing.
    ///
    /// A cap reached and a caller cancellation are different FACTS about a run — one is a
    /// termination-bug backstop tripping, the other is the host asking to stop — but they
    /// are the same fact about the ANSWER: neither leaves a decided result, so both drive
    /// this one variant. [`DlCertificate::stopped`] is where the two are told apart.
    BudgetExhausted,
}

impl DlCompleteness {
    /// Whether every hypertableau run decided its question.
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
    /// Whether any decision this run made reached its round cap or its work cap.
    ///
    /// Together with [`Self::boundaries`] and [`Self::stopped`], this is the minimal state
    /// [`Self::completeness`] derives its verdict from — there is deliberately no
    /// separately stored completeness for the three to disagree with.
    exhausted: bool,
    /// Whether any decision this run made ended because the caller's stop signal fired,
    /// rather than because it reached a cap.
    ///
    /// A run cannot be both: the underlying search decision reports exactly one of
    /// `exhausted`/`stopped` for a truncated search, and this field is that same fact
    /// carried through the session. [`Self::completeness`] folds it into
    /// [`DlCompleteness::BudgetExhausted`] exactly as it folds `exhausted` — a stopped run
    /// is precisely as incomplete as a capped one — but a reader who needs to tell a
    /// cancellation apart from a resource ceiling reads this rather than guessing from
    /// [`Self::steps`] falling short of [`Self::budget`].
    stopped: bool,
    /// The constructs the reverse mapping could not turn into DL clauses.
    boundaries: Vec<Boundary>,
    /// Derivation rounds consumed, summed over every hypertableau run the service made.
    steps: u64,
    /// The per-decision round cap each of those runs ran under.
    budget: u64,
    /// WORK units consumed, summed over every hypertableau run the service made.
    work: u64,
    /// The per-decision work cap each of those runs ran under.
    work_budget: u64,
    /// How many hypertableau runs the service made.
    decisions: u64,
    /// The largest completion graph any of those runs built, in nodes.
    peak_nodes: u64,
    /// `⊔`-rule applications, summed over every run.
    disjunctions: u64,
    /// The deepest branch stack any of those runs reached, in levels.
    peak_depth: u64,
}

impl DlCertificate {
    /// How complete the answer is.
    ///
    /// DERIVED on every call from [`Self::boundaries`] and the run's own exhausted flag,
    /// never stored: an exhausted run is [`DlCompleteness::BudgetExhausted`] whatever else
    /// happened; failing that, a non-empty boundary list is
    /// [`DlCompleteness::DecidedWithinBoundaries`]; and only a run with neither is
    /// [`DlCompleteness::Decided`]. That is what makes a `Decided` verdict beside a
    /// non-empty boundary list a value nothing can construct — not the producer inside
    /// this crate,
    /// and not a consumer assembling one from parts, because there is no second field for
    /// the two to disagree over.
    #[must_use]
    pub fn completeness(&self) -> DlCompleteness {
        if self.exhausted || self.stopped {
            DlCompleteness::BudgetExhausted
        } else if self.boundaries.is_empty() {
            DlCompleteness::Decided
        } else {
            DlCompleteness::DecidedWithinBoundaries
        }
    }

    /// Whether any decision this run made ended because the caller's stop signal fired
    /// rather than because it reached a cap.
    ///
    /// `false` under every OTHER completeness, [`DlCompleteness::BudgetExhausted`]
    /// included: that variant covers both a cap reached and a cancellation, and this is
    /// the one bit that tells a reader which. See the field doc for why the two are folded
    /// into one [`DlCompleteness`] variant rather than a fourth.
    #[must_use]
    pub const fn stopped(&self) -> bool {
        self.stopped
    }

    /// The constructs the reverse mapping could not turn into DL clauses, in
    /// [`Construct`] declaration order.
    #[must_use]
    pub fn boundaries(&self) -> &[Boundary] {
        &self.boundaries
    }

    /// DERIVATION ROUNDS consumed, summed over every hypertableau run this service made.
    ///
    /// One round is one pass of hyperresolution and the `≥`-rule over the whole completion
    /// graph — the unit the cap is denominated in, unchanged in NAME and in units from when
    /// the decision core was a concept-tree tableau and a round was one pass of its
    /// completion rules. The two calculi reach their fixpoints by different routes, so a
    /// count for one ontology is not comparable across that change; what it remains is a
    /// MEASUREMENT in the cap's own units, letting a caller see how close a run came to
    /// [`Self::budget`] without changing anything. It is a round count rather than an elapsed
    /// time: a clock reading is neither reproducible nor available on `wasm32`.
    #[must_use]
    pub const fn steps(&self) -> u64 {
        self.steps
    }

    /// The per-decision round cap every run of this service ran under.
    ///
    /// Per DECISION, not per service call: a service may ask several questions — realization
    /// asks one per (individual, class) pair — and a budget shared across them would turn a
    /// large-but-easy ontology into a `BudgetExhausted` report for no reason. So the cap
    /// bounds each question and [`Self::steps`] reports the sum.
    #[must_use]
    pub const fn budget(&self) -> u64 {
        self.budget
    }

    /// WORK UNITS consumed, summed over every hypertableau run this service made.
    ///
    /// The measurement [`Self::steps`] structurally cannot make. A round is a PASS, not a
    /// unit of work: its cost is the completion graph it runs over times the clauses it
    /// matches against it, so an ontology can make each round enormously more expensive
    /// without making the search take more rounds — and a service reporting three percent of
    /// its round budget while it grinds for hours is reporting the only number it had. This
    /// one is charged where the work happens: each clause-match join step, each successor
    /// subset enumerated, each achiever closure, each neighbour scan's edges, each identified
    /// node's label, each branch-state clone.
    ///
    /// A SUM over the service's decisions, like [`Self::steps`], because it is work done
    /// rather than a size reached. Every charge is an integer counted off the search — never a
    /// clock, a float or a hash iteration order — so it is byte-identical run to run and on
    /// `wasm32`.
    #[must_use]
    pub const fn work(&self) -> u64 {
        self.work
    }

    /// The per-decision WORK cap every run of this service ran under.
    ///
    /// Per DECISION for the reason [`Self::budget`] is, and derived from the knowledge base's
    /// size by the same discipline: a pure function of the input, narrowable by a caller and
    /// never widenable. When [`Self::work`] equals this and [`Self::decisions`] is one, the
    /// work cap is what ended the run — which is the fact a `budget-exhausted` certificate
    /// otherwise leaves a reader to guess at.
    #[must_use]
    pub const fn work_budget(&self) -> u64 {
        self.work_budget
    }

    /// How many hypertableau runs this service made.
    ///
    /// Unchanged in meaning: one per question a service put to the decision core, whichever
    /// calculus that core runs. The denominator for [`Self::steps`], and the thing that makes
    /// a service's cost legible. It counts REFUTATIONS, so a service whose answer is derived rather than
    /// refuted reports a small number: `classify` used to make `n² + 1` of these and now
    /// makes exactly ONE on an ontology inside the classifying saturation's fragment, plus
    /// one per pair that saturation left underived on an ontology outside it. A number that
    /// drops when an algorithm is replaced is the point of measuring it.
    #[must_use]
    pub const fn decisions(&self) -> u64 {
        self.decisions
    }

    /// The largest completion graph any run of this service built, in NODES.
    ///
    /// A maximum over runs and over the branches inside each, never a sum: every branch is a
    /// completion graph of its own, and the question this answers is how big one got. It
    /// counts nodes the graph allocated, merged-away ones included — a merge forwards a node
    /// rather than freeing it — which is the quantity the calculus's blocking discipline is
    /// there to bound, so a number that grows with the ontology instead of with its distinct
    /// label signatures is blocking failing to bite.
    ///
    /// Together with [`Self::disjunctions`] and [`Self::peak_depth`] this says WHERE
    /// [`Self::steps`] went. A round count alone cannot distinguish a search that built one
    /// enormous graph from one that split a thousand times over a small one, and those are
    /// different problems with different fixes.
    #[must_use]
    pub const fn peak_nodes(&self) -> u64 {
        self.peak_nodes
    }

    /// `⊔`-RULE APPLICATIONS, summed over every run this service made.
    ///
    /// One per case split opened — the number of interior nodes of the search tree the
    /// service walked, and so the direct measure of how much non-determinism survived
    /// clausification. Zero is the good case and an ordinary one: an ontology whose every
    /// inclusion absorbs into a guarded clause is decided without a single split.
    ///
    /// A SUM, unlike the two peaks, because a split is WORK done rather than a size reached.
    #[must_use]
    pub const fn disjunctions(&self) -> u64 {
        self.disjunctions
    }

    /// The deepest the `⊔`-rule's branch stack got, in LEVELS.
    ///
    /// A maximum over runs. With [`Self::disjunctions`] it separates a wide, shallow search
    /// from a narrow, deep one — two shapes that cost the same rounds and mean opposite
    /// things — and it is the number that says how much of the search's memory went into
    /// held-open alternatives rather than into any one graph.
    #[must_use]
    pub const fn peak_depth(&self) -> u64 {
        self.peak_depth
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
    /// The proof term of the run that produced it.
    proof: ServiceProof,
}

impl<T> Certified<T> {
    /// Pair an answer with its certificate and its proof term.
    pub(crate) const fn new(answer: T, certificate: DlCertificate, proof: ServiceProof) -> Self {
        Self {
            answer,
            certificate,
            proof,
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

    /// The PROOF TERM of the run that produced it.
    ///
    /// Bound to this service's own question, naming every tableau run it made and which run
    /// decides which reported claim — see [`ServiceProof`]. There is deliberately no
    /// proof-free variant, for the reason there is no certificate-free one: a second entry
    /// point that discarded the evidence is how a proof stops being carried.
    ///
    /// Two calls make it useful, and both take the consumer's own inputs:
    /// [`ServiceProof::verify`] replays every run against the consumer's ontology and question,
    /// and [`ServiceProof::covers`] compares the proof's established claims against the ones
    /// this answer actually reports. Verifying without covering leaves a genuine proof of some
    /// other answer able to travel beside this one.
    pub const fn proof(&self) -> &ServiceProof {
        &self.proof
    }

    /// Take the answer, discarding the certificate and the proof.
    #[must_use]
    pub fn into_answer(self) -> T {
        self.answer
    }

    /// Split into the answer and its certificate, discarding the proof.
    #[must_use]
    pub fn into_parts(self) -> (T, DlCertificate) {
        (self.answer, self.certificate)
    }

    /// Split into the answer, its certificate and its proof term.
    #[must_use]
    pub fn into_certified_parts(self) -> (T, DlCertificate, ServiceProof) {
        (self.answer, self.certificate, self.proof)
    }
}

/// One service call's decision budget, tally, and boundary list.
///
/// The single seam between a service and the hypertableau. Every decision goes through
/// [`Session::refutes`] or [`Session::decide`], so the step tally and the exhausted flag
/// cannot be bypassed, and [`Session::certificate`] is the only producer of a
/// [`DlCertificate`] — which is what makes a certificate that omits an exhausted run
/// unconstructible rather than merely discouraged.
pub(crate) struct Session<'a> {
    /// The knowledge base every decision is made against.
    kb: &'a Kb,
    /// The per-decision budget: a round cap and a work cap.
    budget: Budget,
    /// Steps consumed so far, summed over every decision.
    steps: u64,
    /// Work units consumed so far, summed over every decision.
    work: u64,
    /// Decisions made so far.
    decisions: u64,
    /// Whether any decision reached either cap.
    exhausted: bool,
    /// Whether any decision ended because the caller's stop signal fired.
    stopped: bool,
    /// The largest completion graph any decision built — a MAXIMUM, see
    /// [`DlCertificate::peak_nodes`].
    peak_nodes: u64,
    /// `⊔`-rule applications, SUMMED — see [`DlCertificate::disjunctions`].
    disjunctions: u64,
    /// The deepest branch stack any decision reached — a MAXIMUM, see
    /// [`DlCertificate::peak_depth`].
    peak_depth: u64,
    /// The producer-independent identity of the ontology every run of this session is about.
    input: [u8; 32],
    /// The calculus/clausification contract of `kb`, computed ONCE.
    ///
    /// The knowledge base is borrowed immutably for the session's whole life, so its clause
    /// set cannot change between decisions and neither can this. Computing it per run would
    /// re-clausify the ontology once per (individual, class) pair a realization asks about.
    contract: [u8; 32],
    /// Every decision this session made, in order, with its assumptions and its trace.
    runs: Vec<RunProof>,
    /// What the FIRST decision that did not decide measured — the stopping point.
    stop: Option<StopPoint>,
    /// Whether the recording reached [`MAX_RECORDED_RUNS`], so some runs carry no trace.
    truncated: bool,
}

impl<'a> Session<'a> {
    /// Open a session over `kb` in which each decision may spend `budget`.
    ///
    /// `input` is the producer-independent identity of the ontology `kb` was reverse-mapped
    /// from — computed once per [`Reasoner`](super::Reasoner) rather than once per service
    /// call, because it is a canonicalization of the whole dataset and it does not change
    /// between questions.
    pub(crate) fn new(kb: &'a Kb, budget: Budget, input: [u8; 32]) -> Self {
        Self {
            kb,
            budget,
            contract: contract_of(kb),
            steps: 0,
            work: 0,
            decisions: 0,
            exhausted: false,
            stopped: false,
            peak_nodes: 0,
            disjunctions: 0,
            peak_depth: 0,
            input,
            runs: Vec::new(),
            stop: None,
            truncated: false,
        }
    }

    /// The knowledge base this session reasons over.
    pub(crate) const fn kb(&self) -> &'a Kb {
        self.kb
    }

    /// The index of the run this session made most recently.
    ///
    /// How a service names the run that decides a claim it is about to file. Read immediately
    /// after the decision it refers to; the run list only ever grows, so the index stays
    /// valid.
    pub(crate) fn last_run(&self) -> usize {
        self.runs.len().saturating_sub(1)
    }

    /// Run one hypertableau decision, tallying its cost and writing down its proof term.
    ///
    /// Two aggregations, and which one each counter gets is not a style choice: WORK sums
    /// and SIZE peaks. `steps` and `disjunctions` are work a service spent and are summed
    /// over its decisions; `peak_nodes` and `peak_depth` are how large one search got, and
    /// summing them would report a service that made a thousand tiny decisions as having
    /// built one enormous graph.
    ///
    /// # Recording changes no decision
    ///
    /// An instrumented run and an uninstrumented one reach the IDENTICAL [`Decision`]:
    /// recording never consults and never charges the work meter, which is the standing
    /// obligation the decision core's own tests pin. So the two entry points below are
    /// interchangeable as far as the answer is concerned, and this picks between them purely
    /// on the recording ceiling — past [`MAX_RECORDED_RUNS`] a service stops paying for
    /// traces it will not keep, and says so through
    /// [`ServiceProof::truncated`](super::ServiceProof::truncated).
    pub(crate) fn decide(&mut self, assumptions: &Assumptions<'_>) -> Decision {
        let recording = self.runs.len() < MAX_RECORDED_RUNS;
        let (decision, recorder) = if recording {
            let (decision, recorder) = decide_recording(self.kb, assumptions, self.budget);
            (decision, Some(recorder))
        } else {
            self.truncated = true;
            (decide(self.kb, assumptions, self.budget), None)
        };
        let answer = if decision.exhausted || decision.stopped {
            ProofAnswer::Undecided
        } else if decision.consistent {
            ProofAnswer::Consistent
        } else {
            ProofAnswer::Inconsistent
        };
        let proof: Option<DlProof> = recorder
            .map(|recorder| recorder.into_proof(self.kb, self.input, self.contract, answer));
        if answer == ProofAnswer::Undecided && self.stop.is_none() {
            // The FIRST decision that did not finish is the one that explains the answer: a
            // service that could not decide one sub-question has not decided the aggregate.
            self.stop = Some(StopPoint {
                run: self.runs.len(),
                steps: decision.steps,
                work: decision.work,
                stopped: decision.stopped,
            });
        }
        self.runs.push(RunProof::new(
            RunAssumptions::of(assumptions),
            answer,
            proof,
        ));
        self.steps = self.steps.saturating_add(decision.steps);
        self.work = self.work.saturating_add(decision.work);
        self.decisions += 1;
        self.exhausted |= decision.exhausted;
        self.stopped |= decision.stopped;
        self.peak_nodes = self.peak_nodes.max(decision.peak_nodes);
        self.disjunctions = self.disjunctions.saturating_add(decision.disjunctions);
        self.peak_depth = self.peak_depth.max(decision.peak_depth);
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
        // `stopped` is checked beside `exhausted` for the reason [`Decision`] documents: a
        // cancelled run is exactly as undecided as a capped one, and `decision.consistent`
        // is not a fact about the refutation under either.
        if decision.exhausted || decision.stopped {
            Verdict::Unknown
        } else if decision.consistent {
            Verdict::False
        } else {
            Verdict::True
        }
    }

    /// Seal the session into a certificate over `boundaries`.
    ///
    /// The exhausted flag and the boundary list are stored as this session measured them;
    /// there is no completeness parameter here to pass in, because
    /// [`DlCertificate::completeness`] computes that verdict from exactly these two on every
    /// call rather than being told it. That is what makes a `Decided` verdict beside a
    /// non-empty boundary list a value this function has no way to return.
    pub(crate) fn certificate(&self, boundaries: &BTreeSet<Construct>) -> DlCertificate {
        let boundaries: Vec<Boundary> = Construct::ALL
            .into_iter()
            .filter(|construct| boundaries.contains(construct))
            .map(Boundary::of)
            .collect();
        DlCertificate {
            exhausted: self.exhausted,
            stopped: self.stopped,
            boundaries,
            steps: self.steps,
            budget: self.budget.steps,
            work: self.work,
            work_budget: self.budget.work,
            decisions: self.decisions,
            peak_nodes: self.peak_nodes,
            disjunctions: self.disjunctions,
            peak_depth: self.peak_depth,
        }
    }

    /// Seal the session into a PROOF TERM for `question`, binding `claims` to its runs.
    ///
    /// The companion of [`Self::certificate`] and the only producer of a [`ServiceProof`]
    /// inside this crate. The STOPPING RECEIPT is attached exactly when some decision did not
    /// finish — the same condition [`DlCertificate::completeness`] derives
    /// [`DlCompleteness::BudgetExhausted`] from — and it is built from the counters this
    /// session measured, so a receipt and a certificate cannot disagree.
    pub(crate) fn proof(
        self,
        question: Question,
        claims: Vec<Claim>,
        certificate: &DlCertificate,
    ) -> ServiceProof {
        // A receipt is attached exactly when a decision did not finish, which is the same
        // condition `DlCertificate::completeness` derives `BudgetExhausted` from: the two are
        // read off one measurement, so a receipt and a certificate cannot disagree about
        // whether the service decided.
        let receipt = self
            .stop
            .map(|stop| receipt_of(stop, certificate, &self.runs));
        ServiceProof::new(
            self.input,
            question,
            self.runs,
            claims,
            receipt,
            self.truncated,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_datalog::StopSignal;

    use super::*;
    use crate::owl_dl::Kb;

    /// A stop signal that has already fired. `Reasoner::new` never threads a signal into the
    /// [`Kb`] it builds, so this is how the stop-aware path — real code with real callers
    /// elsewhere in this crate (`materialize_dl_reported_until`'s cousins) — gets exercised
    /// here: attach one directly to a [`Kb`] built for the test, exactly as the stop-aware
    /// constructors do.
    #[derive(Debug)]
    struct AlreadyStopped;

    impl StopSignal for AlreadyStopped {
        fn stopped(&self) -> bool {
            true
        }
    }

    /// An otherwise-trivial knowledge base whose stop signal has already fired.
    fn stopped_kb() -> Kb {
        let mut kb = Kb::empty();
        kb.stop = Some(Arc::new(AlreadyStopped) as Arc<dyn StopSignal>);
        kb
    }

    /// The decision core must report a cancelled run as `stopped`, never as `exhausted` — the
    /// two are different facts ([`Decision`]'s doc), and collapsing them would make a
    /// `budget-exhausted` certificate lie about why the run did not decide.
    #[test]
    fn a_stopped_decision_is_reported_as_stopped_not_exhausted() {
        let kb = stopped_kb();
        let budget = Budget::for_kb(&kb);
        let mut session = Session::new(&kb, budget, [0; 32]);
        let decision = session.decide(&Assumptions::of_kb());
        assert!(
            decision.stopped,
            "the kb's stop signal must be read by the search"
        );
        assert!(
            !decision.exhausted,
            "stopped and exhausted are different facts about a run"
        );
    }

    /// Before this fix, [`Session::decide`]'s caller could see `exhausted: false, consistent:
    /// false` on a stopped run and report a definite `Verdict::False` — a cancellation
    /// rendered as "no model". The certificate must instead say `budget-exhausted` and name
    /// the stop as the reason.
    #[test]
    fn session_certificate_reports_a_stopped_run_as_budget_exhausted_and_says_why() {
        let kb = stopped_kb();
        let budget = Budget::for_kb(&kb);
        let mut session = Session::new(&kb, budget, [0; 32]);
        session.decide(&Assumptions::of_kb());
        let certificate = session.certificate(&BTreeSet::new());
        assert_eq!(certificate.completeness(), DlCompleteness::BudgetExhausted);
        assert!(
            certificate.stopped(),
            "the certificate must say the run was CANCELLED, not merely incomplete"
        );
    }

    /// [`Session::refutes`] must not read a cancelled run's `consistent` field either.
    #[test]
    fn refutes_answers_unknown_rather_than_a_guessed_verdict_when_stopped() {
        let kb = stopped_kb();
        let budget = Budget::for_kb(&kb);
        let mut session = Session::new(&kb, budget, [0; 32]);
        assert_eq!(session.refutes(&Assumptions::of_kb()), Verdict::Unknown);
    }
}
