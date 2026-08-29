// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A PROOF TERM FOR A REASONING SERVICE: the question it was asked, every tableau run it
//! made, which run decides which reported claim, and — when it did not decide — exactly why
//! and where it stopped.
//!
//! # Why a service needs its own proof term
//!
//! [`DlProof`] proves one thing: that ONE hypertableau run over ONE clause set reached the
//! answer it says it did. That is not what a caller of [`Reasoner::classify`](super::Reasoner::classify) asked. A
//! classification is a set of subsumptions, each of which is either a consequence the
//! classifying saturation derived or a separate refutation the tableau closed; a realization
//! is one refutation per (individual, class) pair; an entailment is a refutation of the
//! NEGATION of the axiom the caller wrote. Attaching a consistency proof to any of those and
//! calling it covered would be an equivocation — the proof would be true and would be about a
//! different question.
//!
//! So a [`ServiceProof`] binds three things a bare tableau proof cannot:
//!
//! 1. **THE QUESTION.** [`Self::question`](ServiceProof::question) is the service's question in
//!    the CALLER's own vocabulary — the axiom, the class, the signature — and
//!    [`ServiceProof::verify`] refuses a proof whose question is not the one the consumer
//!    holds. An `entails` proof for a different axiom does not check.
//! 2. **EVERY RUN.** [`RunProof`] carries the ASSUMPTIONS the search actually ran under, read
//!    off the value the decision core received, together with that run's own [`DlProof`]. A
//!    service that decomposes into several runs carries several.
//! 3. **THE ANSWER BINDING.** [`Claim`] names, for every claim the answer reports, the basis
//!    that establishes it — which run refuted it, or that the classifying saturation derived
//!    it, or that it is an axiom of the logic. A reported subsumption with no claim behind it
//!    is a rejection, and so is a claim the answer does not report.
//!
//! # What a run's assumptions being checked means, exactly
//!
//! `KB ⊨ α` exactly when `KB ∪ {¬α}` has no model, and the encoding of `¬α` into tableau
//! assumptions is [`crate::reasoner::axiom`]'s, [`crate::reasoner::classify`]'s and
//! [`crate::reasoner::realize`]'s. The checker RE-DERIVES those assumptions from the caller's
//! own question and compares them against the ones the SEARCH recorded — so a service that
//! decided the wrong question is caught, because the two disagree. What the check cannot be
//! independent of is the encoding itself: if `¬α` is encoded wrongly, the checker encodes it
//! wrongly too. That surface is named [`TrustBaseEntry::RefutationEncoding`] and every check
//! resting on it is reported `trusted`, never `attested`.
//!
//! The same is true of the subsumptions [`crate::reasoner::classify`] derives without opening a
//! tableau at all: the checker re-runs the saturation and checks it derives them, which rests
//! on [`TrustBaseEntry::ClassifyingSaturation`].
//!
//! # The two services with no search to prove
//!
//! [`profile`](super::profile()) and [`extract_module`](super::extract_module)
//! decide their answers SYNTACTICALLY — profile membership is a walk over the axioms, and
//! locality-based module extraction is a fixpoint over the triples. Neither opens a tableau, so
//! neither has a refutation to replay, and inventing a proof term shaped like one would be a
//! fiction. `profile` therefore carries none at all. `extract_module` carries a
//! [`ServiceProof`] with ZERO runs whose question binds the signature and the method and whose
//! single claim binds the extracted module's own canonical identity — which is a real binding
//! (a module proof presented against a different extraction is rejected) and which says out
//! loud, through [`ServiceReplay::runs`] being zero, that there was no search to check.
//!
//! # Determinism
//!
//! Every field is an integer, a fixed-order enum ordinal, a length-prefixed byte string or a
//! term rendered through the same total key the services sort by. Runs are emitted in the order
//! the service made them and claims in the order the answer reports them, so
//! [`ServiceProof::encode`] is byte-identical run to run and on `wasm32`.

use purrdf_core::{RdfDataset, TermValue};

use super::axiom::DlAxiom;
use super::certificate::{DlCertificate, DlCompleteness, Verdict};
use super::classify::ClassHierarchy;
use super::module::ModuleMethod;
use super::realize::Realization;
use super::term_key;
use crate::owl_dl::graph::Assumptions;
use crate::owl_dl::proof::{
    CheckReport, DlProof, DlProofContext, DlProofError, ProofAnswer, TrustBaseEntry,
};

/// Domain-separation tag leading every [`ServiceProof::encode`]d proof.
///
/// Bumped whenever the encoding changes shape, so bytes written under an older layout can never
/// be decoded as if they were current.
const SERVICE_ENCODING_TAG: &str = "purrdf-dl-service-proof-v1";

/// The declared ceiling on how many RUN TRACES one service proof keeps.
///
/// A service is bounded like every other budget in this crate: a constant, a pure function of
/// nothing, and a flag when it bites. Realization asks one question per (individual, class)
/// pair, and keeping a completion graph for each would make a proof term larger than the
/// ontology it is about. Past this ceiling a run is still ACCOUNTED FOR — its assumptions and
/// its answer are recorded, so the answer binding still names it — but its tableau trace is
/// absent, [`ServiceProof::truncated`] is set, and [`ServiceProof::verify`] reports the missing
/// traces [`CheckReport::unattested`] rather than passing them.
pub const MAX_RECORDED_RUNS: usize = 256;

// ── The seven services ──────────────────────────────────────────────────────────

/// WHICH reasoning service a proof term is about.
///
/// The order is the wire order and is fixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Service {
    /// [`Reasoner::consistency`](crate::reasoner::Reasoner::consistency).
    Consistency,
    /// [`Reasoner::class_satisfiability`](crate::reasoner::Reasoner::class_satisfiability).
    ClassSatisfiability,
    /// [`Reasoner::classify`](super::Reasoner::classify)(crate::reasoner::Reasoner::classify).
    Classification,
    /// [`Reasoner::realize`](crate::reasoner::Reasoner::realize).
    Realization,
    /// [`Reasoner::instances`](crate::reasoner::Reasoner::instances).
    InstanceRetrieval,
    /// [`Reasoner::entails`](crate::reasoner::Reasoner::entails).
    AxiomEntailment,
    /// [`extract_module`](super::extract_module) — syntactic, so it makes no tableau
    /// run and its proof term carries none. See the [module docs](self).
    ModuleExtraction,
}

impl Service {
    /// Every service, in wire order.
    pub const ALL: [Self; 7] = [
        Self::Consistency,
        Self::ClassSatisfiability,
        Self::Classification,
        Self::Realization,
        Self::InstanceRetrieval,
        Self::AxiomEntailment,
        Self::ModuleExtraction,
    ];

    /// A short, stable name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Consistency => "consistency",
            Self::ClassSatisfiability => "class-satisfiability",
            Self::Classification => "classification",
            Self::Realization => "realization",
            Self::InstanceRetrieval => "instance-retrieval",
            Self::AxiomEntailment => "axiom-entailment",
            Self::ModuleExtraction => "module-extraction",
        }
    }

    /// The wire ordinal — the service's position in [`Self::ALL`].
    fn ordinal(self) -> u64 {
        Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .expect("every Service is in Service::ALL") as u64
    }
}

/// WHAT a certified answer was asked, in the CALLER's own vocabulary.
///
/// Never in the reasoner's interned ids: a question a consumer cannot write down is a question
/// they cannot check a proof against, and the whole point of this type is that
/// [`ServiceProof::verify`] refuses a proof whose question is not the one the consumer holds.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Question {
    /// Does the ontology have a model at all.
    Consistency,
    /// Can this class have an instance in some model.
    ClassSatisfiability {
        /// The class asked about.
        class: TermValue,
    },
    /// The subsumption relation over these named classes, in the order the reasoner visits
    /// them.
    Classification {
        /// The named classes the answer ranges over.
        classes: Vec<TermValue>,
    },
    /// The entailed types of these named individuals over these named classes.
    Realization {
        /// The named individuals the answer ranges over.
        individuals: Vec<TermValue>,
        /// The named classes it ranges over.
        classes: Vec<TermValue>,
    },
    /// Which named individuals are entailed instances of this class.
    InstanceRetrieval {
        /// The class asked about.
        class: TermValue,
    },
    /// Does the ontology entail this axiom.
    AxiomEntailment {
        /// The axiom asked about.
        axiom: Box<DlAxiom>,
    },
    /// Which axioms the ontology needs for this signature, under this locality notion.
    ModuleExtraction {
        /// The seed signature, as the caller supplied it.
        signature: Vec<TermValue>,
        /// The locality notion.
        method: ModuleMethod,
    },
}

impl Question {
    /// The service this question belongs to.
    #[must_use]
    pub const fn service(&self) -> Service {
        match self {
            Self::Consistency => Service::Consistency,
            Self::ClassSatisfiability { .. } => Service::ClassSatisfiability,
            Self::Classification { .. } => Service::Classification,
            Self::Realization { .. } => Service::Realization,
            Self::InstanceRetrieval { .. } => Service::InstanceRetrieval,
            Self::AxiomEntailment { .. } => Service::AxiomEntailment,
            Self::ModuleExtraction { .. } => Service::ModuleExtraction,
        }
    }
}

// ── One tableau run ─────────────────────────────────────────────────────────────

/// The ASSUMPTIONS one tableau run was made under — the sub-question, verbatim.
///
/// Read off the value the decision core received, never predicted from the question: it is the
/// thing [`ServiceProof::verify`]'s own re-derivation of the question's encoding is compared
/// AGAINST, and a record produced by that same re-derivation would make the comparison vacuous.
///
/// The ids are the reasoner's interned term and concept ids, which mean nothing without the
/// reverse mapping — the same reading a [`DlProof`]'s concept ids have, and the reason
/// [`TrustBaseEntry::ReverseMapping`] is in the trust base.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RunAssumptions {
    /// Whether the ABox was pulled in.
    include_abox: bool,
    /// Extra concept assertions `a : C`, as `(individual term id, concept id)`.
    types: Vec<(u32, u32)>,
    /// Extra role assertions, as `(subject, property, object)` term ids.
    roles: Vec<(u32, u32, u32)>,
    /// Concept ids placed on one fresh, anonymous, unnamed root.
    fresh_types: Vec<u32>,
}

impl RunAssumptions {
    /// The assumptions a search received.
    pub(crate) fn of(assumptions: &Assumptions<'_>) -> Self {
        Self {
            include_abox: assumptions.include_abox,
            types: assumptions.types.to_vec(),
            roles: assumptions.roles.to_vec(),
            fresh_types: assumptions.fresh_types.to_vec(),
        }
    }

    /// Whether the ABox was pulled in.
    #[must_use]
    pub const fn include_abox(&self) -> bool {
        self.include_abox
    }

    /// Extra concept assertions `a : C`.
    #[must_use]
    pub fn types(&self) -> &[(u32, u32)] {
        &self.types
    }

    /// Extra role assertions.
    #[must_use]
    pub fn roles(&self) -> &[(u32, u32, u32)] {
        &self.roles
    }

    /// Concept ids placed on one fresh, anonymous root.
    #[must_use]
    pub fn fresh_types(&self) -> &[u32] {
        &self.fresh_types
    }
}

/// ONE hypertableau run a service made: what it was asked, what it answered, and its trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunProof {
    /// The assumptions the search ran under.
    assumptions: RunAssumptions,
    /// The answer that run reached.
    answer: ProofAnswer,
    /// The recorded tableau proof term, absent past [`MAX_RECORDED_RUNS`].
    proof: Option<DlProof>,
}

impl RunProof {
    /// Pair a run's assumptions, answer and trace. Crate-private: the only producer is the
    /// instrumented session.
    pub(crate) const fn new(
        assumptions: RunAssumptions,
        answer: ProofAnswer,
        proof: Option<DlProof>,
    ) -> Self {
        Self {
            assumptions,
            answer,
            proof,
        }
    }

    /// The assumptions the search ran under — the sub-question, verbatim.
    #[must_use]
    pub const fn assumptions(&self) -> &RunAssumptions {
        &self.assumptions
    }

    /// The answer this run reached.
    #[must_use]
    pub const fn answer(&self) -> ProofAnswer {
        self.answer
    }

    /// The recorded tableau proof term.
    ///
    /// `None` past [`MAX_RECORDED_RUNS`]: the run is still accounted for — its assumptions and
    /// its answer are here, so the answer binding still names it — but there is no trace to
    /// replay, and [`ServiceProof::verify`] counts that [`CheckReport::unattested`].
    #[must_use]
    pub const fn proof(&self) -> Option<&DlProof> {
        self.proof.as_ref()
    }
}

// ── The answer binding ──────────────────────────────────────────────────────────

/// ONE thing a service's answer reports, in the caller's own vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClaimSubject {
    /// The ontology has a model.
    Consistent,
    /// The class can have an instance in some model.
    ClassSatisfiable {
        /// The class.
        class: TermValue,
    },
    /// `sub ⊑ sup` between two named classes.
    Subsumption {
        /// The subsumed class.
        sub: TermValue,
        /// The subsuming class.
        sup: TermValue,
    },
    /// `a : C` — a named individual is an entailed instance of a named class.
    Type {
        /// The individual.
        individual: TermValue,
        /// The class.
        class: TermValue,
    },
    /// The ontology entails this axiom.
    Axiom {
        /// The axiom.
        axiom: Box<DlAxiom>,
    },
    /// The extracted module has this canonical identity — BLAKE3 over its RDFC-1.0 canonical
    /// N-Quads, the same producer-independent identity [`DlProof::input`] uses.
    Module {
        /// The module's canonical identity.
        digest: [u8; 32],
    },
}

/// WHAT establishes a claim.
///
/// The answer binding: a claim with no basis is a claim nothing decided, and
/// [`ServiceProof::verify`] refuses one.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClaimBasis {
    /// Every one of these runs closed every branch, which ESTABLISHES the claim: the claim's
    /// negation has no model. Several runs for a claim that decomposes — an equivalence is two
    /// subsumptions.
    ClosedRefutation {
        /// The runs, into [`ServiceProof::runs`].
        runs: Vec<usize>,
    },
    /// This run exhibited a clash-free completion, which ESTABLISHES the claim.
    ///
    /// The two services whose claim IS a model — consistency and class satisfiability — and
    /// the only two whose positive answer rests on a countermodel rather than on a refutation.
    /// Kept apart from [`Self::CounterModel`] because the same tableau answer establishes one
    /// kind of claim and refutes the other, and a single variant would leave a reader unable to
    /// tell which.
    ExhibitedModel {
        /// The run, into [`ServiceProof::runs`].
        run: usize,
    },
    /// This run exhibited a clash-free completion, which REFUTES the claim: the claim's
    /// negation has a model, so the claim does not hold.
    CounterModel {
        /// The run, into [`ServiceProof::runs`].
        run: usize,
    },
    /// This run reached a cap or was stopped, so the claim is UNDECIDED.
    Undecided {
        /// The run, into [`ServiceProof::runs`].
        run: usize,
    },
    /// The CLASSIFYING SATURATION derived the claim without opening a tableau at all.
    ///
    /// Verified by re-running the saturation, which rests on
    /// [`TrustBaseEntry::ClassifyingSaturation`].
    Saturated,
    /// The claim is an axiom of the logic — `C ⊑ C` — so no search decided it and none needed
    /// to.
    Reflexive,
    /// No search was made at all: the service's own consistency pre-check did not decide, so
    /// nothing downstream of it ran.
    ///
    /// Never a discharged obligation. It is what a `budget-exhausted` classification's claims
    /// rest on, and [`ServiceReplay`] counts it unattested.
    NotDecided,
    /// The claim is decided SYNTACTICALLY, with no search — the module extractor's locality
    /// fixpoint. See the [module docs](self).
    Syntactic,
}

/// One claim a service's answer reports, and what establishes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    /// What is claimed.
    subject: ClaimSubject,
    /// What establishes it.
    basis: ClaimBasis,
}

impl Claim {
    /// Pair a claim with its basis. Crate-private: the only producer is a reasoning service.
    pub(crate) const fn new(subject: ClaimSubject, basis: ClaimBasis) -> Self {
        Self { subject, basis }
    }

    /// What is claimed.
    #[must_use]
    pub const fn subject(&self) -> &ClaimSubject {
        &self.subject
    }

    /// What establishes it.
    #[must_use]
    pub const fn basis(&self) -> &ClaimBasis {
        &self.basis
    }

    /// Whether this basis ESTABLISHES the claim, rather than refuting or withholding it.
    ///
    /// The one place a service's answer and its proof term are compared, so it is a function of
    /// the basis alone: there is no separate "holds" field for a basis to disagree with.
    #[must_use]
    pub const fn is_established(&self) -> bool {
        matches!(
            self.basis,
            ClaimBasis::ClosedRefutation { .. }
                | ClaimBasis::ExhibitedModel { .. }
                | ClaimBasis::Saturated
                | ClaimBasis::Reflexive
                | ClaimBasis::Syntactic
        )
    }
}

// ── The stopping receipt ────────────────────────────────────────────────────────

/// WHY a search stopped.
///
/// DERIVED from [`StopReceipt`]'s counters on every call, never stored — the same discipline
/// [`DlCompleteness`] keeps, and for the same reason: a stored cause
/// beside counters that say otherwise is a state a reader cannot resolve, and the only way to
/// make it unrepresentable is not to have a field for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum StopCause {
    /// The caller's stop signal fired.
    CallerStop,
    /// The run reached its WORK cap — the matcher, scan, closure and clone work done inside a
    /// round.
    WorkCap,
    /// The run reached its ROUND cap.
    RoundCap,
    /// An enumeration INSIDE a round reached a ceiling of its own — the `≠`-clique search and
    /// the counting-witness bound the `≥`-rule runs under.
    ///
    /// Neither of the two caps this receipt reports is the one that bit, and saying "round cap"
    /// or "work cap" for it would be a lie a reader has no way to detect: both counters are
    /// short of their budgets. The variant exists because the fallback arm of
    /// [`StopReceipt::cause`] has to name something true.
    NestedCeiling,
}

impl StopCause {
    /// A short, stable name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CallerStop => "caller-stop",
            Self::WorkCap => "work-cap",
            Self::RoundCap => "round-cap",
            Self::NestedCeiling => "nested-ceiling",
        }
    }
}

/// EXACTLY why and where a service stopped without deciding.
///
/// Present on a [`ServiceProof`] if and only if the service's certificate reports
/// [`DlCompleteness::BudgetExhausted`], and bound to
/// that certificate's own counters — [`Self::steps`] against [`Self::budget`], [`Self::work`]
/// against [`Self::work_budget`] — so a receipt cannot claim a budget that was not exhausted.
///
/// # Where, not just why
///
/// [`Self::run`] names the decision that did not finish, by index into
/// [`ServiceProof::runs`], and [`Self::branches_reached`] / [`Self::clashes_found`] bind the
/// size of the PARTIAL TRACE that run recorded. The trace itself is the run's own [`DlProof`],
/// whose branch points and clash steps are replayed by
/// [`DlProof::replay_partial`] exactly as a deciding run's are — so the partial trace is the
/// real recorded prefix rather than a summary a reader has to take on faith.
///
/// # The counters are the RUN's, not the session's
///
/// [`DlCertificate::steps`] and [`DlCertificate::work`] are SUMS over every decision a service
/// made, while the caps are PER DECISION. Summing across decisions and comparing against a
/// per-decision cap would make the cause unreadable for every multi-run service, so these are
/// the counters of the one decision that stopped. [`Self::session_steps`] and
/// [`Self::session_work`] carry the certificate's sums beside them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopReceipt {
    /// Whether the caller's stop signal fired.
    stopped: bool,
    /// The run that did not decide, into [`ServiceProof::runs`].
    run: usize,
    /// That run's derivation rounds.
    steps: u64,
    /// The per-decision round cap it ran under.
    budget: u64,
    /// That run's work units.
    work: u64,
    /// The per-decision work cap it ran under.
    work_budget: u64,
    /// Derivation rounds summed over the whole service call.
    session_steps: u64,
    /// Work units summed over the whole service call.
    session_work: u64,
    /// How many hypertableau runs the service made.
    decisions: u64,
    /// The largest completion graph any run built, in nodes.
    peak_nodes: u64,
    /// `⊔`-rule applications, summed.
    disjunctions: u64,
    /// The deepest branch stack any run reached.
    peak_depth: u64,
    /// The constructs the reverse mapping could not turn into DL clauses, as short names in
    /// `Construct::ALL` order.
    boundaries: Vec<String>,
    /// Branch points the stopped run recorded before it stopped.
    branches_reached: usize,
    /// Clash instances it found before it stopped.
    clashes_found: usize,
}

impl StopReceipt {
    /// WHY the search stopped, derived from the counters on every call.
    ///
    /// A caller cancellation first — it can fire with either cap still far away — then the work
    /// cap, then the round cap. A run that reached NEITHER cap and was not cancelled stopped
    /// inside a round, on one of the `≥`-rule's own enumeration ceilings, which is
    /// [`StopCause::NestedCeiling`].
    #[must_use]
    pub const fn cause(&self) -> StopCause {
        if self.stopped {
            StopCause::CallerStop
        } else if self.work >= self.work_budget {
            StopCause::WorkCap
        } else if self.steps >= self.budget {
            StopCause::RoundCap
        } else {
            StopCause::NestedCeiling
        }
    }

    /// The run that did not decide, into [`ServiceProof::runs`].
    #[must_use]
    pub const fn run(&self) -> usize {
        self.run
    }

    /// That run's derivation rounds.
    #[must_use]
    pub const fn steps(&self) -> u64 {
        self.steps
    }

    /// The per-decision round cap it ran under.
    #[must_use]
    pub const fn budget(&self) -> u64 {
        self.budget
    }

    /// That run's work units.
    #[must_use]
    pub const fn work(&self) -> u64 {
        self.work
    }

    /// The per-decision work cap it ran under.
    #[must_use]
    pub const fn work_budget(&self) -> u64 {
        self.work_budget
    }

    /// Derivation rounds summed over the whole service call — [`DlCertificate::steps`].
    #[must_use]
    pub const fn session_steps(&self) -> u64 {
        self.session_steps
    }

    /// Work units summed over the whole service call — [`DlCertificate::work`].
    #[must_use]
    pub const fn session_work(&self) -> u64 {
        self.session_work
    }

    /// How many hypertableau runs the service made — [`DlCertificate::decisions`].
    #[must_use]
    pub const fn decisions(&self) -> u64 {
        self.decisions
    }

    /// The largest completion graph any run built — [`DlCertificate::peak_nodes`].
    #[must_use]
    pub const fn peak_nodes(&self) -> u64 {
        self.peak_nodes
    }

    /// `⊔`-rule applications — [`DlCertificate::disjunctions`].
    #[must_use]
    pub const fn disjunctions(&self) -> u64 {
        self.disjunctions
    }

    /// The deepest branch stack any run reached — [`DlCertificate::peak_depth`].
    #[must_use]
    pub const fn peak_depth(&self) -> u64 {
        self.peak_depth
    }

    /// The constructs the reverse mapping bounded, as short names.
    ///
    /// Carried on the receipt as well as on the certificate because an undecided answer over a
    /// BOUNDED ontology is undecided about a strictly smaller ontology than the caller
    /// supplied, and a reader deciding what to do next needs both facts in one place. A bounded
    /// construct is not itself a reason a search stopped — [`Self::cause`] never answers with
    /// one — and saying so is the point of keeping the two separate.
    #[must_use]
    pub fn boundaries(&self) -> &[String] {
        &self.boundaries
    }

    /// Branch points the stopped run recorded before it stopped.
    #[must_use]
    pub const fn branches_reached(&self) -> usize {
        self.branches_reached
    }

    /// Clash instances the stopped run found before it stopped.
    #[must_use]
    pub const fn clashes_found(&self) -> usize {
        self.clashes_found
    }
}

/// What a session measured about the decision that did not finish.
///
/// Crate-private, and taken at the FIRST such decision: a service that could not decide one
/// sub-question has not decided the aggregate, so the first failure is the one that explains
/// the answer.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StopPoint {
    /// The run, into the session's own run list.
    pub(crate) run: usize,
    /// That run's derivation rounds.
    pub(crate) steps: u64,
    /// That run's work units.
    pub(crate) work: u64,
    /// Whether the caller's stop signal fired rather than a cap being reached.
    pub(crate) stopped: bool,
}

// ── The proof term ──────────────────────────────────────────────────────────────

/// A deterministic proof term for ONE reasoning service call.
///
/// See the [module docs](self) for what a replay establishes. Fields are private and there are
/// exactly two producers inside this crate — the instrumented session and
/// the module extractor — so there is no third way to get an unjustified claim into one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceProof {
    /// BLAKE3 over the RDFC-1.0 canonical N-Quads of the ontology — the PRODUCER-INDEPENDENT
    /// input identity, recomputable by the consumer.
    input: [u8; 32],
    /// The producer-shared components this proof's checks rest on.
    trust_base: Vec<TrustBaseEntry>,
    /// Which service.
    service: Service,
    /// The question, in the caller's own vocabulary.
    question: Question,
    /// Every tableau run the service made, in the order it made them.
    runs: Vec<RunProof>,
    /// Every claim the answer reports, and what establishes it.
    claims: Vec<Claim>,
    /// Why and where the search stopped, when it did not decide.
    receipt: Option<StopReceipt>,
    /// Whether the recording reached [`MAX_RECORDED_RUNS`].
    truncated: bool,
}

impl ServiceProof {
    /// Assemble a service proof. Crate-private: the only producer is the instrumented session.
    pub(crate) fn new(
        input: [u8; 32],
        question: Question,
        runs: Vec<RunProof>,
        claims: Vec<Claim>,
        receipt: Option<StopReceipt>,
        truncated: bool,
    ) -> Self {
        Self {
            input,
            trust_base: TrustBaseEntry::ALL.to_vec(),
            service: question.service(),
            question,
            runs,
            claims,
            receipt,
            truncated,
        }
    }

    /// The producer-independent input identity: BLAKE3 over the ontology's canonical N-Quads.
    #[must_use]
    pub const fn input(&self) -> [u8; 32] {
        self.input
    }

    /// The PRODUCER-SHARED components this proof's checks rest on.
    #[must_use]
    pub fn trust_base(&self) -> &[TrustBaseEntry] {
        &self.trust_base
    }

    /// Which service this proof is about.
    #[must_use]
    pub const fn service(&self) -> Service {
        self.service
    }

    /// The question, in the caller's own vocabulary.
    #[must_use]
    pub const fn question(&self) -> &Question {
        &self.question
    }

    /// Every tableau run the service made, in the order it made them.
    #[must_use]
    pub fn runs(&self) -> &[RunProof] {
        &self.runs
    }

    /// Every claim the answer reports, and what establishes it.
    #[must_use]
    pub fn claims(&self) -> &[Claim] {
        &self.claims
    }

    /// Why and where the search stopped, when it did not decide.
    ///
    /// `Some` exactly when the service's certificate reports
    /// [`DlCompleteness::BudgetExhausted`].
    #[must_use]
    pub const fn receipt(&self) -> Option<&StopReceipt> {
        self.receipt.as_ref()
    }

    /// Whether the recording reached [`MAX_RECORDED_RUNS`], so some runs carry no trace.
    #[must_use]
    pub const fn truncated(&self) -> bool {
        self.truncated
    }

    /// Check that this proof is about `question`, over `ontology`.
    ///
    /// **The binding.** Both halves are recomputed by the CONSUMER: the input identity from
    /// their own dataset with [`purrdf_core::canonicalize`], and the question from the question
    /// they are holding. An `entails` proof for a different axiom, or a `classify` proof over a
    /// different class list, fails here before a single run is replayed.
    ///
    /// # Errors
    ///
    /// [`DlProofError::InputMismatch`] or [`DlProofError::WrongQuestion`].
    pub fn binds(&self, ontology: &RdfDataset, question: &Question) -> Result<(), DlProofError> {
        let expected = crate::owl_dl::proof::ontology_identity(ontology);
        if expected != self.input {
            return Err(DlProofError::InputMismatch {
                expected: hex(expected),
                stated: hex(self.input),
            });
        }
        if self.question != *question || self.service != question.service() {
            return Err(DlProofError::WrongQuestion {
                expected: format!("{:?}", question.service().as_str()),
                stated: format!("{:?}", self.service.as_str()),
            });
        }
        if self.trust_base != TrustBaseEntry::ALL {
            return Err(DlProofError::TrustBaseMismatch {
                expected: TrustBaseEntry::ALL
                    .iter()
                    .map(|entry| entry.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
                stated: self
                    .trust_base
                    .iter()
                    .map(|entry| entry.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            });
        }
        Ok(())
    }

    /// The claims this proof states, as the set a service answer is compared against.
    fn stated(&self) -> Vec<&ClaimSubject> {
        self.claims
            .iter()
            .filter(|claim| claim.is_established())
            .map(Claim::subject)
            .collect()
    }

    /// Check that this proof's ESTABLISHED claims are exactly `reported`.
    ///
    /// A MULTISET equality, decided by canonicalizing both sides through the claim encoding and
    /// comparing them element for element. One comparison rather than three overlapping ones,
    /// so every part of it is load-bearing: a claim the answer reports and the proof does not
    /// establish, a claim the proof establishes and the answer does not report, a claim swapped
    /// for another, and a claim listed twice are all the same single failure. Order is NOT
    /// compared — a service's answer is a set, sorted for readability — which is exactly what
    /// canonicalizing before comparing achieves.
    ///
    /// # Errors
    ///
    /// [`DlProofError::AnswerNotCovered`].
    pub fn covers(&self, reported: &[ClaimSubject]) -> Result<(), DlProofError> {
        let canonical = |subjects: &mut Vec<&ClaimSubject>| {
            subjects.sort_by_cached_key(|subject| {
                let mut key = Vec::new();
                encode_subject(&mut key, subject);
                key
            });
        };
        let mut stated = self.stated();
        let mut reported: Vec<&ClaimSubject> = reported.iter().collect();
        canonical(&mut stated);
        canonical(&mut reported);
        if let Some((at, (stated, reported))) = stated
            .iter()
            .zip(&reported)
            .enumerate()
            .find(|(_, (stated, reported))| stated != reported)
        {
            return Err(DlProofError::AnswerNotCovered {
                detail: format!(
                    "the proof establishes {stated:?} where the answer reports {reported:?} \
                     (canonical position {at})"
                ),
            });
        }
        if stated.len() != reported.len() {
            return Err(DlProofError::AnswerNotCovered {
                detail: format!(
                    "the proof establishes {} claims and the answer reports {}",
                    stated.len(),
                    reported.len()
                ),
            });
        }
        Ok(())
    }

    /// VERIFY every run and every claim against the consumer's own ontology.
    ///
    /// 1. [`Self::binds`] — the proof is about this ontology and this question;
    /// 2. every claim names a basis whose run exists and whose recorded answer is the one the
    ///    basis requires: a `Refuted` claim's runs must all be
    ///    [`ProofAnswer::Inconsistent`], a `Countermodel` claim's run
    ///    [`ProofAnswer::Consistent`], an `Undecided` claim's run
    ///    [`ProofAnswer::Undecided`];
    /// 3. the STOPPING RECEIPT is present exactly when some run did not decide, names a run
    ///    that did not decide, and does not claim a budget that was not exhausted;
    /// 4. every run that carries a trace is replayed against `ctx` — a refutation tree, a
    ///    model-checked completion, or a partial trace, whichever its answer is bound to.
    ///
    /// `certificate` is the one this proof arrived beside —
    /// [`Certified::certificate`](super::Certified::certificate) — and the stopping receipt is
    /// checked against ITS counters, so a receipt cannot state a cap, a cost or a boundary set
    /// the service did not report. `None` for a service that issues no certificate at all,
    /// which is the syntactic module extractor and only it; a proof that carries a stopping
    /// receipt beside no certificate is rejected, because there is nothing for the receipt to
    /// be a receipt of.
    ///
    /// `ctx` must be built over the SAME knowledge base the service reasoned about — see
    /// [`Reasoner::proof_context`](crate::reasoner::Reasoner::proof_context), which is the one
    /// constructor that applies the question's own interning. That is the step that rests on
    /// [`TrustBaseEntry::RefutationEncoding`].
    ///
    /// # Errors
    ///
    /// Any [`DlProofError`] — every one of them is a rejection of an invalid proof.
    pub fn verify(
        &self,
        ontology: &RdfDataset,
        question: &Question,
        certificate: Option<&DlCertificate>,
        ctx: &DlProofContext,
    ) -> Result<ServiceReplay, DlProofError> {
        self.binds(ontology, question)?;
        let mut checks = CheckReport::new();
        // The input identity and the question are the consumer's own recomputations.
        checks.attest(2);
        self.check_claims(&mut checks)?;
        self.check_receipt(certificate, &mut checks)?;
        let mut replayed = 0_usize;
        for (index, run) in self.runs.iter().enumerate() {
            let Some(proof) = run.proof.as_ref() else {
                // A run past the recording ceiling is accounted for but not traced. Never
                // presented as checked.
                checks.leave(1);
                continue;
            };
            if proof.answer() != run.answer {
                return Err(DlProofError::AnswerNotCovered {
                    detail: format!(
                        "run {index} states answer {} but its trace is bound to {}",
                        run.answer.as_str(),
                        proof.answer().as_str()
                    ),
                });
            }
            match run.answer {
                ProofAnswer::Inconsistent => {
                    checks.absorb(proof.replay_refutation(ctx)?.checks());
                }
                ProofAnswer::Consistent => {
                    checks.absorb(proof.replay_completion(ctx)?.checks());
                }
                ProofAnswer::Undecided => {
                    checks.absorb(proof.replay_partial(ctx)?.checks());
                }
            }
            replayed += 1;
        }
        // That the recorded assumptions are the QUESTION's own encoding is a statement about
        // the encoding, which the checker shares with the producer.
        checks.trust(self.runs.len(), &[TrustBaseEntry::RefutationEncoding]);
        if self
            .claims
            .iter()
            .any(|claim| matches!(claim.basis, ClaimBasis::Saturated))
        {
            checks.cite(&[TrustBaseEntry::ClassifyingSaturation]);
        }
        if self.truncated {
            checks.leave(1);
        }
        Ok(ServiceReplay {
            runs: self.runs.len(),
            replayed,
            claims: self.claims.len(),
            checks,
        })
    }

    /// Check that every claim names a basis whose run exists and answers the right way.
    fn check_claims(&self, checks: &mut CheckReport) -> Result<(), DlProofError> {
        let dangling = |detail: String| DlProofError::AnswerNotCovered { detail };
        for (at, claim) in self.claims.iter().enumerate() {
            let expect = |run: usize, answer: ProofAnswer| {
                let recorded = self
                    .runs
                    .get(run)
                    .ok_or_else(|| {
                        dangling(format!("claim {at} names run {run}, which is absent"))
                    })?
                    .answer;
                if recorded == answer {
                    return Ok(());
                }
                Err(dangling(format!(
                    "claim {at} rests on run {run} answering {}, but that run answered {}",
                    answer.as_str(),
                    recorded.as_str()
                )))
            };
            match claim.basis {
                ClaimBasis::ClosedRefutation { ref runs } => {
                    if runs.is_empty() {
                        return Err(dangling(format!("claim {at} is refuted by no run at all")));
                    }
                    for &run in runs {
                        expect(run, ProofAnswer::Inconsistent)?;
                    }
                }
                ClaimBasis::ExhibitedModel { run } | ClaimBasis::CounterModel { run } => {
                    expect(run, ProofAnswer::Consistent)?;
                }
                ClaimBasis::Undecided { run } => expect(run, ProofAnswer::Undecided)?,
                ClaimBasis::Saturated
                | ClaimBasis::Reflexive
                | ClaimBasis::NotDecided
                | ClaimBasis::Syntactic => {}
            }
            match claim.basis {
                // A saturated subsumption has no refutation behind it.
                ClaimBasis::Saturated => checks.trust(1, &[TrustBaseEntry::ClassifyingSaturation]),
                // …and a not-decided claim has nothing behind it at all.
                ClaimBasis::NotDecided => checks.leave(1),
                // Reflexivity, the module extractor's locality fixpoint, and the agreement
                // between a basis and the run it names are all arithmetic over the proof term
                // and the caller's own terms.
                ClaimBasis::Reflexive
                | ClaimBasis::Syntactic
                | ClaimBasis::ClosedRefutation { .. }
                | ClaimBasis::ExhibitedModel { .. }
                | ClaimBasis::CounterModel { .. }
                | ClaimBasis::Undecided { .. } => checks.attest(1),
            }
        }
        Ok(())
    }

    /// Check the STOPPING RECEIPT against the runs it describes.
    ///
    /// A receipt is present exactly when some run did not decide; it names a run that did not
    /// decide; the partial-trace sizes it states are the ones that run's trace actually
    /// carries; and its cause is not a budget that was not reached.
    fn check_receipt(
        &self,
        certificate: Option<&DlCertificate>,
        checks: &mut CheckReport,
    ) -> Result<(), DlProofError> {
        let mismatch = |detail: String| DlProofError::ReceiptMismatch { detail };
        let undecided = self
            .runs
            .iter()
            .position(|run| run.answer == ProofAnswer::Undecided);
        let Some(receipt) = self.receipt.as_ref() else {
            if let Some(run) = undecided {
                return Err(mismatch(format!(
                    "run {run} did not decide, and the proof carries no stopping receipt"
                )));
            }
            // A decided answer must also carry a COMPLETE trace: a truncated recording is a
            // trace with a hole in it, and presenting one beside a decided answer is exactly
            // the overclaim this check exists for.
            if self.truncated {
                return Err(mismatch(
                    "every run decided, but the recording was truncated, so the trace beside \
                     the answer is partial"
                        .to_owned(),
                ));
            }
            checks.attest(1);
            return Ok(());
        };
        // THE RUN THAT STOPPED is a fact about the search, not a slot a forger fills: it has to
        // be a run this proof carries, and it has to be one that did not decide.
        let run = self.runs.get(receipt.run).ok_or_else(|| {
            mismatch(format!(
                "the receipt names run {}, which is absent",
                receipt.run
            ))
        })?;
        if run.answer != ProofAnswer::Undecided {
            return Err(mismatch(format!(
                "the receipt names run {} as the one that stopped, but that run answered {}",
                receipt.run,
                run.answer.as_str()
            )));
        }
        // THE RECEIPT IS THE CERTIFICATE'S, READING FOR READING.
        //
        // One comparison rather than nine, so every entry is load-bearing: a widened cap, a
        // rewritten cost, an edited boundary set and a fabricated cancellation are all the same
        // single failure. [`StopCause`] is DERIVED from these readings, which is what makes the
        // derived cause honest rather than self-certifying — a receipt free to state a budget
        // of its own could widen the one it claims to have exhausted until the claim was
        // unfalsifiable.
        let certificate = certificate.ok_or_else(|| {
            mismatch(
                "the proof carries a stopping receipt, and no certificate accompanies it for \
                 the receipt to be a receipt of"
                    .to_owned(),
            )
        })?;
        let bounded: Vec<String> = certificate
            .boundaries()
            .iter()
            .map(|boundary| boundary.construct().as_str().to_owned())
            .collect();
        let stated = [
            (
                "the completeness".to_owned(),
                DlCompleteness::BudgetExhausted.to_string(),
            ),
            ("the round cap".to_owned(), receipt.budget.to_string()),
            ("the work cap".to_owned(), receipt.work_budget.to_string()),
            (
                "the round total".to_owned(),
                receipt.session_steps.to_string(),
            ),
            (
                "the work total".to_owned(),
                receipt.session_work.to_string(),
            ),
            (
                "the decision count".to_owned(),
                receipt.decisions.to_string(),
            ),
            ("the node peak".to_owned(), receipt.peak_nodes.to_string()),
            (
                "the disjunction count".to_owned(),
                receipt.disjunctions.to_string(),
            ),
            ("the depth peak".to_owned(), receipt.peak_depth.to_string()),
            (
                "the cancellation flag".to_owned(),
                receipt.stopped.to_string(),
            ),
            ("the boundary set".to_owned(), receipt.boundaries.join(",")),
        ];
        let reported = [
            (
                "the completeness".to_owned(),
                certificate.completeness().to_string(),
            ),
            ("the round cap".to_owned(), certificate.budget().to_string()),
            (
                "the work cap".to_owned(),
                certificate.work_budget().to_string(),
            ),
            (
                "the round total".to_owned(),
                certificate.steps().to_string(),
            ),
            ("the work total".to_owned(), certificate.work().to_string()),
            (
                "the decision count".to_owned(),
                certificate.decisions().to_string(),
            ),
            (
                "the node peak".to_owned(),
                certificate.peak_nodes().to_string(),
            ),
            (
                "the disjunction count".to_owned(),
                certificate.disjunctions().to_string(),
            ),
            (
                "the depth peak".to_owned(),
                certificate.peak_depth().to_string(),
            ),
            (
                "the cancellation flag".to_owned(),
                certificate.stopped().to_string(),
            ),
            ("the boundary set".to_owned(), bounded.join(",")),
        ];
        if let Some((name, mine, theirs)) =
            stated
                .iter()
                .zip(&reported)
                .find_map(|((name, mine), (_, theirs))| {
                    (mine != theirs).then_some((name, mine, theirs))
                })
        {
            return Err(mismatch(format!(
                "the receipt states {name} as {mine}, and the certificate reports {theirs}"
            )));
        }
        // A run's own cost cannot exceed the whole service's.
        if receipt.steps > receipt.session_steps || receipt.work > receipt.session_work {
            return Err(mismatch(
                "the stopped run states a cost larger than the whole service's".to_owned(),
            ));
        }
        // THE PARTIAL TRACE IS THE REAL RECORDED PREFIX. A receipt that claims more branch
        // points or clashes than the trace carries is describing a search that did not happen.
        if let Some(proof) = run.proof.as_ref() {
            if receipt.branches_reached != proof.branches().len() {
                return Err(mismatch(format!(
                    "the receipt states {} branch points reached, and the trace carries {}",
                    receipt.branches_reached,
                    proof.branches().len()
                )));
            }
            if receipt.clashes_found != proof.clashes().len() {
                return Err(mismatch(format!(
                    "the receipt states {} clashes found, and the trace carries {}",
                    receipt.clashes_found,
                    proof.clashes().len()
                )));
            }
            checks.attest(2);
        } else {
            checks.leave(2);
        }
        checks.attest(4);
        Ok(())
    }

    /// The canonical byte encoding of the proof.
    ///
    /// Layout, all integers little-endian and every variable-length field length-prefixed, so
    /// no concatenation of two fields can be confused with a different split of the same bytes:
    ///
    /// ```text
    /// u64 tag_len, tag bytes                      -- SERVICE_ENCODING_TAG
    /// 32 bytes input identity
    /// u8  service ordinal
    /// u8  truncated
    /// u64 trust_base_count, u64 TrustBaseEntry::ALL ordinal each
    /// question                                    -- kind byte and that kind's terms
    /// u64 run_count, then per run:
    ///     u8 include_abox, u8 answer ordinal
    ///     u64 type_count, u32 pair each
    ///     u64 role_count, u32 triple each
    ///     u64 fresh_count, u32 each
    ///     u8 has_proof, then when set: u64 len, DlProof::encode bytes
    /// u64 claim_count, then per claim: subject, basis
    /// u8  has_receipt, then when set the receipt's fields in declaration order
    /// ```
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        frame(&mut out, SERVICE_ENCODING_TAG.as_bytes());
        out.extend_from_slice(&self.input);
        out.push(self.service.ordinal() as u8);
        out.push(u8::from(self.truncated));
        length(&mut out, self.trust_base.len());
        for entry in &self.trust_base {
            length(
                &mut out,
                TrustBaseEntry::ALL
                    .iter()
                    .position(|candidate| candidate == entry)
                    .expect("every TrustBaseEntry is in TrustBaseEntry::ALL"),
            );
        }
        encode_question(&mut out, &self.question);
        length(&mut out, self.runs.len());
        for run in &self.runs {
            out.push(u8::from(run.assumptions.include_abox));
            out.push(answer_ordinal(run.answer));
            length(&mut out, run.assumptions.types.len());
            for &(a, b) in &run.assumptions.types {
                out.extend_from_slice(&a.to_le_bytes());
                out.extend_from_slice(&b.to_le_bytes());
            }
            length(&mut out, run.assumptions.roles.len());
            for triple in &run.assumptions.roles {
                for value in <[u32; 3]>::from(*triple) {
                    out.extend_from_slice(&value.to_le_bytes());
                }
            }
            length(&mut out, run.assumptions.fresh_types.len());
            for &concept in &run.assumptions.fresh_types {
                out.extend_from_slice(&concept.to_le_bytes());
            }
            match run.proof.as_ref() {
                Some(proof) => {
                    out.push(1);
                    frame(&mut out, &proof.encode());
                }
                None => out.push(0),
            }
        }
        length(&mut out, self.claims.len());
        for claim in &self.claims {
            encode_claim(&mut out, claim);
        }
        match self.receipt.as_ref() {
            Some(receipt) => {
                out.push(1);
                encode_receipt(&mut out, receipt);
            }
            None => out.push(0),
        }
        out
    }

    /// The BLAKE3 digest of [`Self::encode`] — the proof term's stable identity.
    ///
    /// A CONTENT digest, never an IRI: **PurRDF mints no vocabulary**.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        *blake3::hash(&self.encode()).as_bytes()
    }

    /// [`Self::digest`] as 64 lowercase hex characters.
    #[must_use]
    pub fn digest_hex(&self) -> String {
        hex(self.digest())
    }
}

/// What [`ServiceProof::verify`] established about a whole service call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceReplay {
    /// Tableau runs the service made.
    runs: usize,
    /// How many of them carried a trace the checker replayed.
    replayed: usize,
    /// Claims the answer binding accounts for.
    claims: usize,
    /// The three counts and what they rest on.
    checks: CheckReport,
}

impl ServiceReplay {
    /// Tableau runs the service made.
    ///
    /// ZERO for the two syntactic services — see the [module docs](self) — which is the report
    /// saying that there was no search to check rather than that a search checked out.
    #[must_use]
    pub const fn runs(&self) -> usize {
        self.runs
    }

    /// How many of those runs carried a trace the checker replayed.
    ///
    /// Below [`Self::runs`] exactly when the recording reached [`MAX_RECORDED_RUNS`]; the
    /// difference is counted [`CheckReport::unattested`].
    #[must_use]
    pub const fn replayed(&self) -> usize {
        self.replayed
    }

    /// Claims the answer binding accounts for.
    #[must_use]
    pub const fn claims(&self) -> usize {
        self.claims
    }

    /// The full classification — see [`CheckReport`].
    #[must_use]
    pub const fn checks(&self) -> &CheckReport {
        &self.checks
    }
}

// ── The claims an answer reports ────────────────────────────────────────────────

/// The claims a [`ClassHierarchy`] reports, as [`ServiceProof::covers`] compares them.
///
/// Every established subsumption between two distinct named classes, which is exactly what
/// [`ClassHierarchy::subsumptions`] holds — the equivalences, the unsatisfiable list and the
/// transitive reduction are all VIEWS of that relation rather than independent claims, so
/// listing them again would demand a second proof of the same fact.
#[must_use]
pub fn hierarchy_claims(hierarchy: &ClassHierarchy) -> Vec<ClaimSubject> {
    hierarchy
        .subsumptions()
        .iter()
        .map(|(sub, sup)| ClaimSubject::Subsumption {
            sub: sub.clone(),
            sup: sup.clone(),
        })
        .collect()
}

/// The claims a [`Realization`] reports.
///
/// [`Realization::types`] — the direct types are the subset of them no other entailed type
/// specializes, a view rather than a separate claim.
#[must_use]
pub fn realization_claims(realization: &Realization) -> Vec<ClaimSubject> {
    realization
        .types()
        .iter()
        .map(|(individual, class)| ClaimSubject::Type {
            individual: individual.clone(),
            class: class.clone(),
        })
        .collect()
}

/// The claims an instance-retrieval answer reports, over the class that was asked about.
#[must_use]
pub fn instance_claims(class: &TermValue, individuals: &[TermValue]) -> Vec<ClaimSubject> {
    individuals
        .iter()
        .map(|individual| ClaimSubject::Type {
            individual: individual.clone(),
            class: class.clone(),
        })
        .collect()
}

/// The claims a boolean answer reports about `subject`.
///
/// A [`Verdict::True`] reports the claim; [`Verdict::False`] and [`Verdict::Unknown`] report
/// NOTHING, which is the whole reason a DL service answers three-valued: "not established" and
/// "established false" are both the absence of a claim, and the certificate beside the answer
/// is what tells them apart.
#[must_use]
pub fn verdict_claims(subject: &ClaimSubject, answer: Verdict) -> Vec<ClaimSubject> {
    if answer.is_true() {
        vec![subject.clone()]
    } else {
        Vec::new()
    }
}

// ── Assembling a receipt ────────────────────────────────────────────────────────

/// Assemble a [`StopReceipt`] from a session's stop point and its certificate.
///
/// Crate-private: a receipt is derived from what a session measured, never assembled from
/// parts by a caller, which is what keeps its counters and the certificate's from disagreeing.
pub(crate) fn receipt_of(
    point: StopPoint,
    certificate: &DlCertificate,
    runs: &[RunProof],
) -> StopReceipt {
    let trace = runs.get(point.run).and_then(RunProof::proof);
    StopReceipt {
        stopped: point.stopped,
        run: point.run,
        steps: point.steps,
        budget: certificate.budget(),
        work: point.work,
        work_budget: certificate.work_budget(),
        session_steps: certificate.steps(),
        session_work: certificate.work(),
        decisions: certificate.decisions(),
        peak_nodes: certificate.peak_nodes(),
        disjunctions: certificate.disjunctions(),
        peak_depth: certificate.peak_depth(),
        boundaries: certificate
            .boundaries()
            .iter()
            .map(|boundary| boundary.construct().as_str().to_owned())
            .collect(),
        branches_reached: trace.map_or(0, |proof| proof.branches().len()),
        clashes_found: trace.map_or(0, |proof| proof.clashes().len()),
    }
}

// ── Byte plumbing ───────────────────────────────────────────────────────────────

/// 32 bytes as 64 lowercase hex characters.
fn hex(digest: [u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push(char::from_digit(u32::from(byte >> 4), 16).expect("a nibble is one hex digit"));
        out.push(char::from_digit(u32::from(byte & 0x0f), 16).expect("a nibble is one hex digit"));
    }
    out
}

/// Append a length-prefixed byte string.
fn frame(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(bytes);
}

/// Append a `usize` as a little-endian `u64`.
fn length(out: &mut Vec<u8>, value: usize) {
    out.extend_from_slice(&(value as u64).to_le_bytes());
}

/// The wire ordinal of a tableau answer.
const fn answer_ordinal(answer: ProofAnswer) -> u8 {
    match answer {
        ProofAnswer::Consistent => 0,
        ProofAnswer::Inconsistent => 1,
        ProofAnswer::Undecided => 2,
    }
}

/// Append a term through the same total, dataset-independent key the services sort by.
///
/// A term is encoded by its KEY rather than by a structural walk so that two terms that sort
/// equal encode equal — which is what makes a claim's identity the same notion the answer's own
/// ordering uses.
fn encode_term(out: &mut Vec<u8>, term: &TermValue) {
    let (kind, key) = term_key(term);
    out.push(kind);
    frame(out, key.as_bytes());
}

/// Append a list of terms.
fn encode_terms(out: &mut Vec<u8>, terms: &[TermValue]) {
    length(out, terms.len());
    for term in terms {
        encode_term(out, term);
    }
}

/// Append a [`Question`].
fn encode_question(out: &mut Vec<u8>, question: &Question) {
    match question {
        Question::Consistency => out.push(0),
        Question::ClassSatisfiability { class } => {
            out.push(1);
            encode_term(out, class);
        }
        Question::Classification { classes } => {
            out.push(2);
            encode_terms(out, classes);
        }
        Question::Realization {
            individuals,
            classes,
        } => {
            out.push(3);
            encode_terms(out, individuals);
            encode_terms(out, classes);
        }
        Question::InstanceRetrieval { class } => {
            out.push(4);
            encode_term(out, class);
        }
        Question::AxiomEntailment { axiom } => {
            out.push(5);
            encode_axiom(out, axiom);
        }
        Question::ModuleExtraction { signature, method } => {
            out.push(6);
            encode_terms(out, signature);
            out.push(
                ModuleMethod::ALL
                    .iter()
                    .position(|candidate| candidate == method)
                    .expect("every ModuleMethod is in ModuleMethod::ALL") as u8,
            );
        }
    }
}

/// Append a [`DlAxiom`] — a kind byte and then its terms in declaration order.
fn encode_axiom(out: &mut Vec<u8>, axiom: &DlAxiom) {
    let (kind, terms): (u8, [&TermValue; 3]) = match axiom {
        DlAxiom::SubClassOf { sub, sup } => (0, [sub, sup, sub]),
        DlAxiom::EquivalentClasses { left, right } => (1, [left, right, left]),
        DlAxiom::DisjointClasses { left, right } => (2, [left, right, left]),
        DlAxiom::ClassAssertion { individual, class } => (3, [individual, class, individual]),
        DlAxiom::ObjectPropertyAssertion {
            subject,
            property,
            object,
        } => (4, [subject, property, object]),
        DlAxiom::SameIndividual { left, right } => (5, [left, right, left]),
        DlAxiom::DifferentIndividuals { left, right } => (6, [left, right, left]),
        DlAxiom::SubObjectPropertyOf { sub, sup } => (7, [sub, sup, sub]),
    };
    out.push(kind);
    for term in terms {
        encode_term(out, term);
    }
}

/// Append a [`Claim`].
fn encode_claim(out: &mut Vec<u8>, claim: &Claim) {
    encode_subject(out, &claim.subject);
    encode_basis(out, &claim.basis);
}

/// Append a [`ClaimSubject`].
///
/// Also the CANONICAL KEY [`ServiceProof::covers`] sorts by, which is why it is a function of
/// its own: two claims are the same claim exactly when these bytes agree, and having one
/// definition of that is what keeps the wire format and the answer binding from drifting.
fn encode_subject(out: &mut Vec<u8>, subject: &ClaimSubject) {
    match subject {
        ClaimSubject::Consistent => out.push(0),
        ClaimSubject::ClassSatisfiable { class } => {
            out.push(1);
            encode_term(out, class);
        }
        ClaimSubject::Subsumption { sub, sup } => {
            out.push(2);
            encode_term(out, sub);
            encode_term(out, sup);
        }
        ClaimSubject::Type { individual, class } => {
            out.push(3);
            encode_term(out, individual);
            encode_term(out, class);
        }
        ClaimSubject::Axiom { axiom } => {
            out.push(4);
            encode_axiom(out, axiom);
        }
        ClaimSubject::Module { digest } => {
            out.push(5);
            out.extend_from_slice(digest);
        }
    }
}

/// Append a [`ClaimBasis`].
fn encode_basis(out: &mut Vec<u8>, basis: &ClaimBasis) {
    match basis {
        ClaimBasis::ClosedRefutation { runs } => {
            out.push(0);
            length(out, runs.len());
            for &run in runs {
                length(out, run);
            }
        }
        ClaimBasis::ExhibitedModel { run } => {
            out.push(1);
            length(out, *run);
        }
        ClaimBasis::CounterModel { run } => {
            out.push(2);
            length(out, *run);
        }
        ClaimBasis::Undecided { run } => {
            out.push(3);
            length(out, *run);
        }
        ClaimBasis::Saturated => out.push(4),
        ClaimBasis::Reflexive => out.push(5),
        ClaimBasis::NotDecided => out.push(6),
        ClaimBasis::Syntactic => out.push(7),
    }
}

/// Append a [`StopReceipt`], every field in declaration order.
fn encode_receipt(out: &mut Vec<u8>, receipt: &StopReceipt) {
    out.push(u8::from(receipt.stopped));
    length(out, receipt.run);
    for value in [
        receipt.steps,
        receipt.budget,
        receipt.work,
        receipt.work_budget,
        receipt.session_steps,
        receipt.session_work,
        receipt.decisions,
        receipt.peak_nodes,
        receipt.disjunctions,
        receipt.peak_depth,
    ] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    length(out, receipt.boundaries.len());
    for boundary in &receipt.boundaries {
        frame(out, boundary.as_bytes());
    }
    length(out, receipt.branches_reached);
    length(out, receipt.clashes_found);
}

/// The claim a REFUTATION-DECIDED boolean question reports, for a service to file.
///
/// Crate-private helper shared by the services whose positive answer is a closed refutation —
/// entailment, subsumption, instance membership — so the mapping from a [`Verdict`] to a
/// [`ClaimBasis`] lives in ONE place rather than four. A `True` is the refutation that closed,
/// a `False` is the run that exhibited a counter-model, and an `Unknown` is the run that did
/// not decide; when no run was made at all — the service's consistency pre-check did not
/// decide — the basis is [`ClaimBasis::NotDecided`], which establishes nothing.
///
/// Consistency and class satisfiability do NOT go through here: their positive answer rests on
/// an exhibited model rather than on a refutation, and folding both polarities into one helper
/// is exactly how a countermodel comes to stand for a proof.
pub(crate) fn refutation_claim(subject: ClaimSubject, answer: Verdict, runs: &[usize]) -> Claim {
    let basis = match answer {
        Verdict::True if !runs.is_empty() => ClaimBasis::ClosedRefutation {
            runs: runs.to_vec(),
        },
        Verdict::True => ClaimBasis::Reflexive,
        Verdict::False => {
            runs.last()
                .map_or(ClaimBasis::NotDecided, |&run| ClaimBasis::CounterModel {
                    run,
                })
        }
        Verdict::Unknown => runs
            .last()
            .map_or(ClaimBasis::NotDecided, |&run| ClaimBasis::Undecided { run }),
    };
    Claim::new(subject, basis)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_core::{RdfDatasetBuilder, TermValue};

    use super::*;
    use crate::reasoner::{ModuleMethod, Reasoner, extract_module};

    /// Fixture terms are `example.org`: **PurRDF mints no vocabulary**.
    const EX_CAT: &str = "http://example.org/Cat";
    /// A second fixture class.
    const EX_ANIMAL: &str = "http://example.org/Animal";
    /// A third fixture class.
    const EX_FISH: &str = "http://example.org/Fish";
    /// A fixture individual.
    const EX_TOM: &str = "http://example.org/tom";
    /// `rdfs:subClassOf`.
    const RDFS_SUBCLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
    /// `rdf:type`.
    const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

    /// `Cat ⊑ Animal`, `Fish ⊑ Animal`, `tom : Cat` — consistent, with a real taxonomy and a
    /// real realization to bind.
    fn taxonomy() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let cat = b.intern_iri(EX_CAT);
        let animal = b.intern_iri(EX_ANIMAL);
        let fish = b.intern_iri(EX_FISH);
        let tom = b.intern_iri(EX_TOM);
        let sub = b.intern_iri(RDFS_SUBCLASS_OF);
        let ty = b.intern_iri(RDF_TYPE);
        b.push_quad(cat, sub, animal, None);
        b.push_quad(fish, sub, animal, None);
        b.push_quad(tom, ty, cat, None);
        b.freeze().expect("the fixture freezes")
    }

    /// A different consistent ontology, for the wrong-ontology negatives.
    fn other_ontology() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri("http://example.org/a");
        let p = b.intern_iri("http://example.org/p");
        let c = b.intern_iri("http://example.org/c");
        b.push_quad(a, p, c, None);
        b.freeze().expect("the fixture freezes")
    }

    /// A checking context built the way a CONSUMER builds one: from their own copy of the data
    /// and their own copy of the question, with nothing the producer shipped.
    fn context(ontology: &RdfDataset, question: &Question) -> DlProofContext {
        let mut checker = Reasoner::new(ontology).expect("the fixture reverse-maps");
        checker.prepare(question);
        checker.proof_context()
    }

    /// The `Cat ⊑ Animal` entailment question.
    fn subclass_axiom() -> DlAxiom {
        DlAxiom::SubClassOf {
            sub: TermValue::iri(EX_CAT),
            sup: TermValue::iri(EX_ANIMAL),
        }
    }

    // ── Every certified service carries a proof bound to its own question ────────

    /// All six certified services produce a proof that VERIFIES against a consumer's own
    /// ontology and question, and every run they made is replayed.
    ///
    /// The headline of this stage. Before it, only consistency produced a proof term at all.
    #[test]
    fn every_certified_service_carries_a_proof_bound_to_its_own_question() {
        let ontology = taxonomy();
        let cat = TermValue::iri(EX_CAT);
        let axiom = subclass_axiom();

        let mut reasoner = Reasoner::new(&ontology).expect("reverse-maps");
        let questions: Vec<(Question, ServiceProof, DlCertificate)> = vec![
            {
                let answer = reasoner.consistency();
                (
                    Question::Consistency,
                    answer.proof().clone(),
                    answer.certificate().clone(),
                )
            },
            {
                let answer = reasoner.class_satisfiability(&cat).expect("consistent");
                (
                    Question::ClassSatisfiability { class: cat.clone() },
                    answer.proof().clone(),
                    answer.certificate().clone(),
                )
            },
            {
                let answer = reasoner.instances(&cat).expect("consistent");
                (
                    Question::InstanceRetrieval { class: cat.clone() },
                    answer.proof().clone(),
                    answer.certificate().clone(),
                )
            },
            {
                let answer = reasoner.entails(&axiom).expect("consistent");
                (
                    Question::AxiomEntailment {
                        axiom: Box::new(axiom.clone()),
                    },
                    answer.proof().clone(),
                    answer.certificate().clone(),
                )
            },
            {
                let answer = reasoner.classify().expect("consistent");
                (
                    reasoner.classification_question(),
                    answer.proof().clone(),
                    answer.certificate().clone(),
                )
            },
            {
                let answer = reasoner.realize().expect("consistent");
                (
                    reasoner.realization_question(),
                    answer.proof().clone(),
                    answer.certificate().clone(),
                )
            },
        ];
        assert_eq!(questions.len(), 6, "six services return `Certified<T>`");
        for (question, proof, certificate) in questions {
            let ctx = context(&ontology, &question);
            let replay = proof
                .verify(&ontology, &question, Some(&certificate), &ctx)
                .unwrap_or_else(|error| {
                    panic!("{} must verify: {error}", question.service().as_str())
                });
            assert_eq!(
                proof.service(),
                question.service(),
                "a proof states the service its question belongs to"
            );
            assert_eq!(
                replay.runs(),
                replay.replayed(),
                "the fixture is far below MAX_RECORDED_RUNS, so every run carries a trace: \
                 {replay:?}"
            );
            assert!(
                !replay.checks().is_fully_attested(),
                "reading a clause set is a trusted check, and no service proof may claim \
                 otherwise: {:?}",
                replay.checks()
            );
        }
    }

    /// A CLASSIFICATION proof binds the subsumptions the answer reports — each naming what
    /// decided it — and covers them exactly.
    #[test]
    fn a_classification_proof_binds_every_subsumption_it_reports() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology).expect("reverse-maps");
        let answer = reasoner.classify().expect("consistent");
        let hierarchy = answer.answer();
        assert!(
            !hierarchy.subsumptions().is_empty(),
            "the fixture has a taxonomy"
        );
        answer
            .proof()
            .covers(&hierarchy_claims(hierarchy))
            .expect("the proof establishes exactly the subsumptions the answer reports");
        assert!(
            answer
                .proof()
                .claims()
                .iter()
                .any(|claim| matches!(claim.basis(), ClaimBasis::Saturated)),
            "an EL-shaped taxonomy is classified by the saturation, and the proof says so \
             rather than claiming a refutation nothing made: {:?}",
            answer.proof().claims()
        );
    }

    /// A REALIZATION proof binds every type the answer reports.
    #[test]
    fn a_realization_proof_binds_every_type_it_reports() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology).expect("reverse-maps");
        let answer = reasoner.realize().expect("consistent");
        assert!(!answer.answer().types().is_empty(), "tom has types");
        answer
            .proof()
            .covers(&realization_claims(answer.answer()))
            .expect("the proof establishes exactly the types the answer reports");
    }

    /// An INSTANCE-RETRIEVAL proof binds the individuals the answer returns.
    #[test]
    fn an_instance_retrieval_proof_binds_the_individuals_it_returns() {
        let ontology = taxonomy();
        let mut reasoner = Reasoner::new(&ontology).expect("reverse-maps");
        let animal = TermValue::iri(EX_ANIMAL);
        let answer = reasoner.instances(&animal).expect("consistent");
        assert_eq!(answer.answer().len(), 1, "tom is the only Animal");
        answer
            .proof()
            .covers(&instance_claims(&animal, answer.answer()))
            .expect("the proof establishes exactly the individuals the answer returns");
    }

    /// A BOOLEAN answer binds through [`verdict_claims`]: a `True` reports its claim, and a
    /// `False` reports none at all.
    ///
    /// The three-valued answer is the reason the two directions are separate: "not established"
    /// and "established false" are both the absence of a claim, and a proof that established
    /// one anyway would be a proof of the other answer.
    #[test]
    fn a_boolean_answer_binds_through_its_claim_or_through_its_absence() {
        let ontology = taxonomy();
        let mut reasoner = Reasoner::new(&ontology).expect("reverse-maps");
        let holds = subclass_axiom();
        let answer = reasoner.entails(&holds).expect("consistent");
        assert_eq!(*answer.answer(), Verdict::True);
        let subject = ClaimSubject::Axiom {
            axiom: Box::new(holds),
        };
        answer
            .proof()
            .covers(&verdict_claims(&subject, *answer.answer()))
            .expect("a True answer reports its claim, and the proof establishes it");

        let fails = DlAxiom::SubClassOf {
            sub: TermValue::iri(EX_ANIMAL),
            sup: TermValue::iri(EX_CAT),
        };
        let answer = reasoner.entails(&fails).expect("consistent");
        assert_eq!(*answer.answer(), Verdict::False);
        let subject = ClaimSubject::Axiom {
            axiom: Box::new(fails),
        };
        assert!(
            verdict_claims(&subject, *answer.answer()).is_empty(),
            "a False answer reports nothing"
        );
        answer.proof().covers(&[]).expect(
            "…and the proof establishes nothing, though it still names the run that \
                     exhibited the counter-model",
        );
        assert!(
            matches!(
                answer.proof().claims()[0].basis(),
                ClaimBasis::CounterModel { .. }
            ),
            "the counter-model is REPORTED rather than dropped: {:?}",
            answer.proof().claims()
        );
    }

    // ── Tamper-negatives: written as a forger ───────────────────────────────────

    /// **THE HEADLINE NEGATIVE.** An `entails` proof for one axiom does not check against
    /// another.
    ///
    /// The equivocation this whole type exists to prevent: every run inside the proof is
    /// genuine and every claim inside it is established, and it is still not a proof of the
    /// axiom the consumer is asking about.
    #[test]
    fn an_entails_proof_does_not_check_against_a_different_axiom() {
        let ontology = taxonomy();
        let mut reasoner = Reasoner::new(&ontology).expect("reverse-maps");
        let honest = subclass_axiom();
        let answer = reasoner.entails(&honest).expect("consistent");
        let other = DlAxiom::SubClassOf {
            sub: TermValue::iri(EX_FISH),
            sup: TermValue::iri(EX_CAT),
        };
        let question = Question::AxiomEntailment {
            axiom: Box::new(other),
        };
        let ctx = context(&ontology, &question);
        assert!(matches!(
            answer
                .proof()
                .verify(&ontology, &question, Some(answer.certificate()), &ctx),
            Err(DlProofError::WrongQuestion { .. })
        ));
    }

    /// A proof of one SERVICE does not check as a proof of another, even over the same
    /// ontology — a consistency proof is not a classification.
    #[test]
    fn a_consistency_proof_does_not_check_as_a_classification() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology).expect("reverse-maps");
        let consistency = reasoner.consistency();
        let question = reasoner.classification_question();
        let ctx = context(&ontology, &question);
        assert!(matches!(
            consistency
                .proof()
                .verify(&ontology, &question, Some(consistency.certificate()), &ctx),
            Err(DlProofError::WrongQuestion { .. })
        ));
    }

    /// A CLASSIFY proof missing a reported subsumption is rejected.
    ///
    /// The forgery a classification invites: report the taxonomy, and quietly leave one
    /// subsumption's claim out so nothing has to establish it.
    #[test]
    fn a_classify_proof_missing_a_reported_subsumption_is_rejected() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology).expect("reverse-maps");
        let answer = reasoner.classify().expect("consistent");
        let reported = hierarchy_claims(answer.answer());
        // The CANONICALLY LAST established claim, so what remains is an exact prefix of what
        // the answer reports: the element-wise comparison agrees at every position it reaches,
        // and only the count says the claim is missing. Dropping an arbitrary one would leave
        // the arithmetic beside the comparison unconstrained.
        let mut proof = answer.proof().clone();
        let last = proof
            .claims
            .iter()
            .enumerate()
            .filter(|(_, claim)| claim.is_established())
            .max_by_key(|(_, claim)| {
                let mut key = Vec::new();
                encode_subject(&mut key, claim.subject());
                key
            })
            .map(|(at, _)| at)
            .expect("the fixture establishes subsumptions");
        proof.claims.remove(last);
        match proof.covers(&reported) {
            Err(DlProofError::AnswerNotCovered { .. }) => {}
            other => panic!("a reported subsumption with nothing behind it: {other:?}"),
        }
    }

    /// …and a claim the answer does NOT report is rejected too: a proof of more than the
    /// answer says is a proof of a different answer.
    #[test]
    fn a_classify_proof_stating_a_subsumption_the_answer_omits_is_rejected() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology).expect("reverse-maps");
        let answer = reasoner.classify().expect("consistent");
        let reported = hierarchy_claims(answer.answer());
        // SWAPPED, not appended: the claim count stays exactly what the answer reports, so
        // this constrains the containment check rather than the arithmetic beside it.
        let mut proof = answer.proof().clone();
        let at = proof
            .claims
            .iter()
            .position(Claim::is_established)
            .expect("the fixture establishes subsumptions");
        proof.claims[at] = Claim::new(
            ClaimSubject::Subsumption {
                sub: TermValue::iri(EX_ANIMAL),
                sup: TermValue::iri(EX_FISH),
            },
            ClaimBasis::Saturated,
        );
        assert_eq!(
            proof.stated().len(),
            reported.len(),
            "the forgery keeps the count honest"
        );
        assert!(matches!(
            proof.covers(&reported),
            Err(DlProofError::AnswerNotCovered { .. })
        ));
        // …and appending one is rejected too, from the other side.
        let mut proof = answer.proof().clone();
        proof.claims.push(Claim::new(
            ClaimSubject::Subsumption {
                sub: TermValue::iri(EX_ANIMAL),
                sup: TermValue::iri(EX_FISH),
            },
            ClaimBasis::Saturated,
        ));
        assert!(matches!(
            proof.covers(&reported),
            Err(DlProofError::AnswerNotCovered { .. })
        ));
    }

    /// A service proof presented against ANOTHER ontology is rejected before a run is read.
    #[test]
    fn a_service_proof_does_not_check_against_a_different_ontology() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology).expect("reverse-maps");
        let answer = reasoner.consistency();
        let other = other_ontology();
        let ctx = context(&other, &Question::Consistency);
        assert!(matches!(
            answer.proof().verify(
                &other,
                &Question::Consistency,
                Some(answer.certificate()),
                &ctx
            ),
            Err(DlProofError::InputMismatch { .. })
        ));
    }

    /// A claim naming a run that answered the OTHER way is rejected: a refutation basis must
    /// name a run that closed.
    #[test]
    fn a_claim_naming_a_run_that_answered_otherwise_is_rejected() {
        let ontology = taxonomy();
        let mut reasoner = Reasoner::new(&ontology).expect("reverse-maps");
        let axiom = subclass_axiom();
        let answer = reasoner.entails(&axiom).expect("consistent");
        let question = Question::AxiomEntailment {
            axiom: Box::new(axiom),
        };
        let mut proof = answer.proof().clone();
        // Run 0 is the consistency pre-check, which found a MODEL. Claiming it closed is the
        // forgery: it turns a countermodel into a refutation.
        proof.claims[0] = Claim::new(
            proof.claims[0].subject().clone(),
            ClaimBasis::ClosedRefutation { runs: vec![0] },
        );
        let ctx = context(&ontology, &question);
        match proof.verify(&ontology, &question, Some(answer.certificate()), &ctx) {
            Err(DlProofError::AnswerNotCovered { .. }) => {}
            other => panic!("a countermodel cannot stand in for a refutation: {other:?}"),
        }
    }

    /// A claim naming a run that is not there is a rejection rather than an index panic.
    #[test]
    fn a_claim_naming_an_absent_run_is_rejected() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology).expect("reverse-maps");
        let answer = reasoner.consistency();
        let mut proof = answer.proof().clone();
        proof.claims[0] = Claim::new(
            ClaimSubject::Consistent,
            ClaimBasis::ClosedRefutation {
                runs: vec![usize::MAX],
            },
        );
        let ctx = context(&ontology, &Question::Consistency);
        assert!(matches!(
            proof.verify(
                &ontology,
                &Question::Consistency,
                Some(answer.certificate()),
                &ctx
            ),
            Err(DlProofError::AnswerNotCovered { .. })
        ));
    }

    /// A claim refuted by NO run at all is rejected — an empty refutation is not a refutation.
    #[test]
    fn a_claim_refuted_by_no_run_is_rejected() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology).expect("reverse-maps");
        let answer = reasoner.consistency();
        let mut proof = answer.proof().clone();
        proof.claims[0] = Claim::new(
            ClaimSubject::Consistent,
            ClaimBasis::ClosedRefutation { runs: Vec::new() },
        );
        let ctx = context(&ontology, &Question::Consistency);
        assert!(matches!(
            proof.verify(
                &ontology,
                &Question::Consistency,
                Some(answer.certificate()),
                &ctx
            ),
            Err(DlProofError::AnswerNotCovered { .. })
        ));
    }

    /// A run whose stated answer disagrees with the answer its own TRACE is bound to is
    /// rejected.
    #[test]
    fn a_run_whose_answer_contradicts_its_trace_is_rejected() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology).expect("reverse-maps");
        let answer = reasoner.consistency();
        let mut proof = answer.proof().clone();
        // The claim moves with the run, so the answer binding stays self-consistent and the
        // ONLY thing left disagreeing is the run's own trace.
        proof.runs[0].answer = ProofAnswer::Inconsistent;
        proof.claims[0] = Claim::new(
            ClaimSubject::Consistent,
            ClaimBasis::ClosedRefutation { runs: vec![0] },
        );
        let ctx = context(&ontology, &Question::Consistency);
        assert!(matches!(
            proof.verify(
                &ontology,
                &Question::Consistency,
                Some(answer.certificate()),
                &ctx
            ),
            Err(DlProofError::AnswerNotCovered { .. })
        ));
    }

    /// A claim resting on an EXHIBITED MODEL whose run did not exhibit one is rejected.
    ///
    /// The mirror of the refutation forgery, and the one the two model-shaped bases invite:
    /// consistency and class satisfiability are established by a clash-free completion, so a
    /// forger who points such a claim at a run that CLOSED is claiming a model from a
    /// refutation.
    #[test]
    fn a_claim_resting_on_a_model_whose_run_closed_is_rejected() {
        let ontology = taxonomy();
        let mut reasoner = Reasoner::new(&ontology).expect("reverse-maps");
        let axiom = subclass_axiom();
        let answer = reasoner.entails(&axiom).expect("consistent");
        let question = Question::AxiomEntailment {
            axiom: Box::new(axiom),
        };
        let closed = answer
            .proof()
            .runs()
            .iter()
            .position(|run| run.answer() == ProofAnswer::Inconsistent)
            .expect("an entailed subsumption closes a refutation");
        let mut proof = answer.proof().clone();
        proof.claims[0] = Claim::new(
            proof.claims[0].subject().clone(),
            ClaimBasis::ExhibitedModel { run: closed },
        );
        let ctx = context(&ontology, &question);
        match proof.verify(&ontology, &question, Some(answer.certificate()), &ctx) {
            Err(DlProofError::AnswerNotCovered { .. }) => {}
            other => panic!("a closed refutation exhibits no model: {other:?}"),
        }
    }

    /// A proof with NO RUNS AT ALL still refuses a different ontology.
    ///
    /// The module extractor is the case where the proof's OWN input binding is the only thing
    /// that can fire: there is no tableau trace whose identity check could catch the
    /// substitution instead.
    #[test]
    fn a_runless_proof_still_refuses_a_different_ontology() {
        let ontology = taxonomy();
        let seed = [TermValue::iri(EX_CAT)];
        let extracted = extract_module(&ontology, &seed, ModuleMethod::Bot).expect("extracts");
        assert!(
            extracted.proof().runs().is_empty(),
            "locality extraction opens no tableau"
        );
        let other = other_ontology();
        let question = Question::ModuleExtraction {
            signature: seed.to_vec(),
            method: ModuleMethod::Bot,
        };
        let ctx = context(&other, &question);
        match extracted.proof().verify(&other, &question, None, &ctx) {
            Err(DlProofError::InputMismatch { .. }) => {}
            other => panic!("a proof for another ontology: {other:?}"),
        }
    }

    // ── The stopping receipt ────────────────────────────────────────────────────

    /// An UNDECIDED answer carries a receipt that names the cap it reached, the run that
    /// stopped, and the partial trace that run recorded.
    #[test]
    fn an_undecided_answer_carries_a_stopping_receipt_bound_to_its_counters() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology)
            .expect("reverse-maps")
            .with_step_cap(0);
        let answer = reasoner.consistency();
        assert_eq!(*answer.answer(), Verdict::Unknown, "a zero round cap");
        let receipt = answer
            .proof()
            .receipt()
            .expect("an undecided answer carries a receipt");
        assert_eq!(receipt.cause(), StopCause::RoundCap);
        assert_eq!(receipt.budget(), 0, "the cap it reached");
        assert_eq!(receipt.steps(), receipt.budget());
        assert_eq!(receipt.decisions(), 1);
        assert_eq!(
            receipt.run(),
            0,
            "the run that did not decide, named rather than described"
        );
        let question = Question::Consistency;
        let ctx = context(&ontology, &question);
        let replay = answer
            .proof()
            .verify(&ontology, &question, Some(answer.certificate()), &ctx)
            .expect("an undecided proof's partial trace still replays");
        assert_eq!(replay.runs(), 1);
    }

    /// A DECIDED answer carries NO receipt, and the check is what makes that structural rather
    /// than conventional.
    #[test]
    fn a_decided_answer_carries_no_stopping_receipt() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology).expect("reverse-maps");
        let answer = reasoner.consistency();
        assert_eq!(*answer.answer(), Verdict::True);
        assert!(answer.proof().receipt().is_none());
        assert!(
            answer.certificate().completeness().is_decided(),
            "and the certificate agrees, which is the pair that must never disagree"
        );
    }

    /// A receipt CLAIMING A BUDGET THAT WAS NOT EXHAUSTED is rejected.
    ///
    /// The forgery: dress a cancellation, or a run that stopped for no stated reason, as a
    /// resource ceiling nobody actually reached.
    #[test]
    fn a_receipt_claiming_a_budget_that_was_not_exhausted_is_rejected() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology)
            .expect("reverse-maps")
            .with_step_cap(0);
        let answer = reasoner.consistency();
        let mut proof = answer.proof().clone();
        let receipt = proof.receipt.as_mut().expect("undecided");
        // The honest receipt reports steps == budget == 0. Widening the cap without moving the
        // counter is exactly "a budget that was not exhausted".
        receipt.budget = 4_096;
        let question = Question::Consistency;
        let ctx = context(&ontology, &question);
        match proof.verify(&ontology, &question, Some(answer.certificate()), &ctx) {
            Err(DlProofError::ReceiptMismatch { .. }) => {}
            other => panic!("an unreached cap is not a stopping reason: {other:?}"),
        }
    }

    /// A receipt WHOSE PARTIAL TRACE OMITS A BRANCH POINT THAT WAS REACHED is rejected.
    ///
    /// The receipt's counts are bound to the trace the run actually carries, so a forger
    /// cannot shrink the frontier they report while keeping the trace that contradicts it.
    #[test]
    fn a_receipt_whose_partial_trace_understates_the_branch_points_is_rejected() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology)
            .expect("reverse-maps")
            .with_step_cap(0);
        let answer = reasoner.consistency();
        let mut proof = answer.proof().clone();
        let receipt = proof.receipt.as_mut().expect("undecided");
        receipt.branches_reached += 1;
        let question = Question::Consistency;
        let ctx = context(&ontology, &question);
        assert!(matches!(
            proof.verify(&ontology, &question, Some(answer.certificate()), &ctx),
            Err(DlProofError::ReceiptMismatch { .. })
        ));
        // …and the same from the clash side.
        let mut proof = answer.proof().clone();
        let receipt = proof.receipt.as_mut().expect("undecided");
        receipt.clashes_found += 1;
        assert!(matches!(
            proof.verify(&ontology, &question, Some(answer.certificate()), &ctx),
            Err(DlProofError::ReceiptMismatch { .. })
        ));
    }

    /// A receipt NAMING A RUN THAT DECIDED is rejected: the run that stopped is a fact about
    /// the search, not a slot a forger fills.
    #[test]
    fn a_receipt_naming_a_run_that_decided_is_rejected() {
        let ontology = taxonomy();
        let mut reasoner = Reasoner::new(&ontology).expect("reverse-maps");
        let cat = TermValue::iri(EX_CAT);
        let answer = reasoner.class_satisfiability(&cat).expect("consistent");
        let mut proof = answer.proof().clone();
        assert!(proof.receipt.is_none(), "the fixture decides");
        proof.receipt = Some(receipt_of(
            StopPoint {
                run: 0,
                steps: 1,
                work: 1,
                stopped: false,
            },
            answer.certificate(),
            &proof.runs,
        ));
        let question = Question::ClassSatisfiability { class: cat };
        let ctx = context(&ontology, &question);
        assert!(matches!(
            proof.verify(&ontology, &question, Some(answer.certificate()), &ctx),
            Err(DlProofError::ReceiptMismatch { .. })
        ));
    }

    /// An UNDECIDED run with NO receipt is rejected: "the search stopped" must never be a
    /// silent fact.
    #[test]
    fn an_undecided_run_with_no_receipt_is_rejected() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology)
            .expect("reverse-maps")
            .with_step_cap(0);
        let answer = reasoner.consistency();
        let mut proof = answer.proof().clone();
        proof.receipt = None;
        let question = Question::Consistency;
        let ctx = context(&ontology, &question);
        assert!(matches!(
            proof.verify(&ontology, &question, Some(answer.certificate()), &ctx),
            Err(DlProofError::ReceiptMismatch { .. })
        ));
    }

    /// A DECIDED ANSWER CARRYING A TRUNCATED TRACE is rejected.
    ///
    /// A truncated recording is a trace with a hole in it. Presenting one beside an answer
    /// that claims every run finished is the overclaim this check exists for.
    #[test]
    fn a_decided_answer_carrying_a_truncated_trace_is_rejected() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology).expect("reverse-maps");
        let answer = reasoner.consistency();
        let mut proof = answer.proof().clone();
        proof.truncated = true;
        let question = Question::Consistency;
        let ctx = context(&ontology, &question);
        match proof.verify(&ontology, &question, Some(answer.certificate()), &ctx) {
            Err(DlProofError::ReceiptMismatch { .. }) => {}
            other => panic!("a partial trace beside a decided answer: {other:?}"),
        }
    }

    /// A receipt naming a run that is NOT there is a rejection rather than an index panic.
    #[test]
    fn a_receipt_naming_an_absent_run_is_rejected() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology)
            .expect("reverse-maps")
            .with_step_cap(0);
        let answer = reasoner.consistency();
        let mut proof = answer.proof().clone();
        proof.receipt.as_mut().expect("undecided").run = usize::MAX;
        let question = Question::Consistency;
        let ctx = context(&ontology, &question);
        match proof.verify(&ontology, &question, Some(answer.certificate()), &ctx) {
            Err(DlProofError::ReceiptMismatch { .. }) => {}
            other => panic!("a receipt for a run nobody made: {other:?}"),
        }
    }

    /// A receipt POINTED AT A RUN THAT DECIDED is rejected, even when another run genuinely
    /// did not.
    ///
    /// The forgery: keep the honest undecided run in the trace, and blame the stop on a
    /// different, deciding one — which would misreport where the search ran out.
    #[test]
    fn a_receipt_pointed_at_a_deciding_run_is_rejected() {
        let ontology = taxonomy();
        let undecided = Reasoner::new(&ontology)
            .expect("reverse-maps")
            .with_step_cap(0)
            .consistency();
        let decided = Reasoner::new(&ontology)
            .expect("reverse-maps")
            .consistency();
        let mut proof = undecided.proof().clone();
        // A second run, this one decided — and the receipt moved onto it.
        proof.runs.push(decided.proof().runs()[0].clone());
        proof.receipt.as_mut().expect("undecided").run = 1;
        let question = Question::Consistency;
        let ctx = context(&ontology, &question);
        match proof.verify(&ontology, &question, Some(undecided.certificate()), &ctx) {
            Err(DlProofError::ReceiptMismatch { .. }) => {}
            other => panic!("a run that decided did not stop: {other:?}"),
        }
    }

    /// A receipt checked against NO certificate at all is refused: there is nothing for its
    /// counters to be the counters of.
    #[test]
    fn a_receipt_checked_against_no_certificate_is_refused() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology)
            .expect("reverse-maps")
            .with_step_cap(0);
        let answer = reasoner.consistency();
        let question = Question::Consistency;
        let ctx = context(&ontology, &question);
        match answer.proof().verify(&ontology, &question, None, &ctx) {
            Err(DlProofError::ReceiptMismatch { .. }) => {}
            other => panic!("a receipt with nothing to be a receipt of: {other:?}"),
        }
    }

    /// A receipt checked against a certificate that says the service DECIDED is rejected: the
    /// two halves of an answer must agree about whether it decided.
    #[test]
    fn a_receipt_beside_a_decided_certificate_is_rejected() {
        let ontology = taxonomy();
        let undecided = Reasoner::new(&ontology)
            .expect("reverse-maps")
            .with_step_cap(0)
            .consistency();
        let decided = Reasoner::new(&ontology)
            .expect("reverse-maps")
            .consistency();
        assert!(decided.certificate().completeness().is_decided());
        let question = Question::Consistency;
        let ctx = context(&ontology, &question);
        match undecided
            .proof()
            .verify(&ontology, &question, Some(decided.certificate()), &ctx)
        {
            Err(DlProofError::ReceiptMismatch { .. }) => {}
            other => panic!("a stopping receipt beside a decided certificate: {other:?}"),
        }
    }

    /// A receipt whose BOUNDARY SET is not the certificate's is rejected.
    ///
    /// An undecided answer over a bounded ontology is undecided about a strictly smaller
    /// ontology than the caller supplied. Editing that fact out of the receipt would leave a
    /// reader deciding what to do next on half the story.
    #[test]
    fn a_receipt_whose_boundary_set_is_not_the_certificates_is_rejected() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology)
            .expect("reverse-maps")
            .with_step_cap(0);
        let answer = reasoner.consistency();
        let mut proof = answer.proof().clone();
        proof
            .receipt
            .as_mut()
            .expect("undecided")
            .boundaries
            .push("property-chain".to_owned());
        let question = Question::Consistency;
        let ctx = context(&ontology, &question);
        match proof.verify(&ontology, &question, Some(answer.certificate()), &ctx) {
            Err(DlProofError::ReceiptMismatch { .. }) => {}
            other => panic!("an invented boundary is not a boundary: {other:?}"),
        }
    }

    /// A receipt claiming a CALLER CANCELLATION the certificate does not report is rejected —
    /// a cap reached and a host asking to stop are different facts about a run.
    #[test]
    fn a_receipt_inventing_a_caller_cancellation_is_rejected() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology)
            .expect("reverse-maps")
            .with_step_cap(0);
        let answer = reasoner.consistency();
        assert!(!answer.certificate().stopped(), "a cap, not a cancellation");
        let mut proof = answer.proof().clone();
        let receipt = proof.receipt.as_mut().expect("undecided");
        receipt.stopped = true;
        assert_eq!(
            receipt.cause(),
            StopCause::CallerStop,
            "and the derived cause moves with it, which is what the check has to catch"
        );
        let question = Question::Consistency;
        let ctx = context(&ontology, &question);
        match proof.verify(&ontology, &question, Some(answer.certificate()), &ctx) {
            Err(DlProofError::ReceiptMismatch { .. }) => {}
            other => panic!("an invented cancellation: {other:?}"),
        }
    }

    /// A receipt whose STOPPED RUN costs more than the whole service is rejected: a sum cannot
    /// be smaller than one of its terms.
    #[test]
    fn a_receipt_whose_run_costs_more_than_the_service_is_rejected() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology)
            .expect("reverse-maps")
            .with_step_cap(0);
        let answer = reasoner.consistency();
        let mut proof = answer.proof().clone();
        proof.receipt.as_mut().expect("undecided").work = u64::MAX;
        let question = Question::Consistency;
        let ctx = context(&ontology, &question);
        match proof.verify(&ontology, &question, Some(answer.certificate()), &ctx) {
            Err(DlProofError::ReceiptMismatch { .. }) => {}
            other => panic!("a term larger than its own sum: {other:?}"),
        }
    }

    // ── Module extraction: syntactic, and it says so ────────────────────────────

    /// A module extraction's proof binds the signature, the notion AND the extracted module's
    /// own identity — and reports ZERO runs, because it makes none.
    #[test]
    fn a_module_extraction_proof_binds_its_question_and_reports_no_search() {
        let ontology = taxonomy();
        let seed = [TermValue::iri(EX_CAT)];
        let extracted =
            extract_module(&ontology, &seed, ModuleMethod::Bot).expect("the fixture extracts");
        let question = Question::ModuleExtraction {
            signature: seed.to_vec(),
            method: ModuleMethod::Bot,
        };
        let ctx = context(&ontology, &question);
        let replay = extracted
            .proof()
            .verify(&ontology, &question, None, &ctx)
            .expect("a genuine extraction's proof checks");
        assert_eq!(
            replay.runs(),
            0,
            "locality extraction opens no tableau, and the report says so rather than \
             claiming a search checked out"
        );
        extracted
            .proof()
            .covers(&[ClaimSubject::Module {
                digest: crate::owl_dl::proof::ontology_identity(extracted.module()),
            }])
            .expect("the claim is the module's own canonical identity");
    }

    /// A module proof does not check against a DIFFERENT extraction of the same ontology.
    #[test]
    fn a_module_proof_does_not_cover_a_different_extraction() {
        let ontology = taxonomy();
        let seed = [TermValue::iri(EX_CAT)];
        let bot = extract_module(&ontology, &seed, ModuleMethod::Bot).expect("extracts");
        let other = extract_module(&ontology, &seed, ModuleMethod::Top).expect("extracts");
        assert!(matches!(
            bot.proof().covers(&[ClaimSubject::Module {
                digest: crate::owl_dl::proof::ontology_identity(other.module()),
            }]),
            Err(DlProofError::AnswerNotCovered { .. })
        ));
    }

    /// …and a module proof for one NOTION does not check as one for another.
    #[test]
    fn a_module_proof_does_not_check_against_a_different_notion() {
        let ontology = taxonomy();
        let seed = [TermValue::iri(EX_CAT)];
        let bot = extract_module(&ontology, &seed, ModuleMethod::Bot).expect("extracts");
        let question = Question::ModuleExtraction {
            signature: seed.to_vec(),
            method: ModuleMethod::Star,
        };
        let ctx = context(&ontology, &question);
        assert!(matches!(
            bot.proof().verify(&ontology, &question, None, &ctx),
            Err(DlProofError::WrongQuestion { .. })
        ));
    }

    /// A proof carrying a STOPPING RECEIPT beside runs that all decided is rejected on the
    /// proof's own terms, with no certificate needed.
    ///
    /// The module extractor is the one service that issues no certificate, so this is the case
    /// where the proof-internal check is the only one that can fire: a receipt describes a stop
    /// that did not happen, and it is refused whether or not there is a certificate to compare
    /// it against.
    #[test]
    fn a_receipt_beside_a_proof_whose_runs_all_decided_is_rejected_without_a_certificate() {
        let ontology = taxonomy();
        let seed = [TermValue::iri(EX_CAT)];
        let extracted =
            extract_module(&ontology, &seed, ModuleMethod::Bot).expect("the fixture extracts");
        let question = Question::ModuleExtraction {
            signature: seed.to_vec(),
            method: ModuleMethod::Bot,
        };
        let mut proof = extracted.proof().clone();
        assert!(
            proof.runs.is_empty(),
            "locality extraction opens no tableau"
        );
        proof.receipt = Some(receipt_of(
            StopPoint {
                run: 0,
                steps: 0,
                work: 0,
                stopped: true,
            },
            &Reasoner::new(&ontology)
                .expect("reverse-maps")
                .consistency()
                .certificate()
                .clone(),
            &proof.runs,
        ));
        let ctx = context(&ontology, &question);
        match proof.verify(&ontology, &question, None, &ctx) {
            Err(DlProofError::ReceiptMismatch { .. }) => {}
            other => panic!("a receipt for a stop that did not happen: {other:?}"),
        }
    }

    // ── Determinism ─────────────────────────────────────────────────────────────

    /// Two independent runs of a service produce BYTE-IDENTICAL proof terms.
    #[test]
    fn a_service_proof_is_byte_identical_run_to_run() {
        let ontology = taxonomy();
        for _ in 0..2 {
            let first = Reasoner::new(&ontology).expect("reverse-maps");
            let again = Reasoner::new(&ontology).expect("reverse-maps");
            assert_eq!(
                first.realize().expect("consistent").proof().encode(),
                again.realize().expect("consistent").proof().encode(),
                "two runs, one proof"
            );
        }
        let first = Reasoner::new(&ontology).expect("reverse-maps");
        let again = Reasoner::new(&ontology).expect("reverse-maps");
        let one = first.classify().expect("consistent");
        let two = again.classify().expect("consistent");
        assert_eq!(one.proof().digest(), two.proof().digest());
        assert_eq!(one.proof().digest_hex().len(), 64);
    }

    /// Editing ANY part of a service proof moves its digest — the question, a run, a claim and
    /// the receipt alike.
    #[test]
    fn every_part_of_a_service_proof_is_digested() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology).expect("reverse-maps");
        let answer = reasoner.classify().expect("consistent");
        let base = answer.proof().digest();

        let mut edited = answer.proof().clone();
        edited.claims.pop();
        assert_ne!(base, edited.digest(), "a dropped claim is a different term");

        let mut edited = answer.proof().clone();
        edited.runs.pop();
        assert_ne!(base, edited.digest(), "a dropped run is a different term");

        let mut edited = answer.proof().clone();
        edited.question = Question::Consistency;
        edited.service = Service::Consistency;
        assert_ne!(
            base,
            edited.digest(),
            "a rewritten question is a different term"
        );

        let mut edited = answer.proof().clone();
        edited.truncated = true;
        assert_ne!(base, edited.digest());

        let mut edited = answer.proof().clone();
        edited.input[0] ^= 0xff;
        assert_ne!(base, edited.digest());
    }

    /// The trust base a service proof states is the one this checker classifies against, and a
    /// proof stating another is REJECTED rather than checked against a different meaning of
    /// "verified".
    #[test]
    fn a_service_proof_stating_another_trust_base_is_rejected() {
        let ontology = taxonomy();
        let reasoner = Reasoner::new(&ontology).expect("reverse-maps");
        let answer = reasoner.consistency();
        assert_eq!(answer.proof().trust_base(), TrustBaseEntry::ALL);
        let mut proof = answer.proof().clone();
        proof.trust_base.pop();
        let ctx = context(&ontology, &Question::Consistency);
        assert!(matches!(
            proof.verify(
                &ontology,
                &Question::Consistency,
                Some(answer.certificate()),
                &ctx
            ),
            Err(DlProofError::TrustBaseMismatch { .. })
        ));
    }
}
