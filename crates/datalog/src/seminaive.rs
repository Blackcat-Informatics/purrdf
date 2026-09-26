// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The stratified semi-naive bottom-up evaluator.
//!
//! This is the crate's fixpoint: it consumes an [`Executable`] (the terminal stage of
//! [`crate::plan`]'s `Parsed → Stratified → Planned → Executable` pipeline) plus a seeded
//! [`RelationStore`] EDB, and runs each stratum to its least fixpoint before the next one
//! starts. A negated body atom is therefore always decided against a relation that has
//! already reached its final extension — the stratified-negation semantics, not an
//! approximation of it.
//!
//! # Atoms are arity-4, and the predicate is data
//!
//! An atom is `triple(?s, ?p, ?o, ?g)`. The predicate and graph positions choose the store
//! PARTITION and the subject/object positions drive its `(subject, object)` index, and the
//! two are independent: an atom whose predicate and graph are known — a constant, or a
//! variable an earlier atom bound — addresses one partition through a single ordered-map
//! probe and then uses exactly the access path it always used, while an atom that
//! genuinely quantifies over the predicate or the graph sweeps the matching partitions in
//! lexical order and indexes inside each of them. Carrying the predicate as data therefore
//! costs an ordinary rule nothing and costs a meta-rule one partition sweep, not a scan.
//!
//! # The two physical joins
//!
//! Every rule's positive body is evaluated by ONE of two kernels, chosen by the planner:
//!
//! - the **indexed binary join** (the fallback, and the only path for an acyclic body):
//!   each planned atom, in sideways-information-passing order, extends the partial
//!   solutions by selecting exactly the rows its [`IndexChoice`] admits;
//! - the **leapfrog triejoin**: for a planner-certified cyclic component, a multiway
//!   intersection descends the component's variables through galloping
//!   [`ValueCursor`]s, so no binary intermediate relation is ever materialised.
//!
//! Two implementations of one contract must not be allowed to diverge silently, so the
//! binary fallback is retained as an executable oracle and
//! `leapfrog_and_binary_joins_agree` asserts the two produce identical relations over the
//! whole synthetic corpus.
//!
//! # Semi-naive decomposition
//!
//! A round evaluates, for each positive atom position `p`, the join
//! `{ a_p ∈ delta, a_{<p} ∈ everything, a_{>p} ∈ store \ delta }`. Exactly one `p`
//! matches any given tuple assignment (`p` is its LAST in-delta position), so the
//! positions partition the new derivations: no duplicate work, no missed derivation. The
//! delta itself is a contiguous [`RowId`] span, because the commit loop mints row ids
//! densely in one sorted pass — so delta membership is a range compare, not a hash probe.
//!
//! # Determinism
//!
//! Round candidates are keyed in a [`BTreeMap`] and winners are committed in lexical
//! `(subject, predicate, object, graph)` order. The partition sweep an unbound predicate
//! drives is in lexical `(predicate surface, graph surface)` order, never in the store's
//! mint order. Where two rules derive the same head fact in one
//! round the winner is chosen by a **total order over observable provenance**
//! — `(proof height, summed source heights, sorted source facts, rule index, source
//! facts)` — never by arrival order, so neither the row enumeration order nor the rule
//! scheduling can reach an output. Rule-parallel rounds
//! use rayon's indexed `par_iter` over a rule-index slice and merge the per-rule buffers
//! strictly in program order; `par_sort`/`par_bridge` are never used, because they are not
//! order-stable.
//!
//! # Budgets are constants
//!
//! [`MAX_JOIN_STEPS`], [`MAX_STORED_FACTS`] and [`MAX_TERM_ARENA_BYTES`] are fixed
//! `const`s. Exceeding one returns [`EvalError::BudgetExhausted`] carrying an accurate
//! [`BudgetReport`] — never a panic, never a truncated answer presented as complete. See
//! the crate docs for why a caller-supplied budget is not offered for them, and
//! [`TermGeneratingLimit`] for the one limit that IS the caller's ([`EvalOptions`]).

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use rayon::prelude::*;

use crate::clause::{ClauseAtom, ClauseTerm, DlClause, HeadForm};
use crate::cursor::{LendingIterator, VALUE_OBJECT, VALUE_SUBJECT, ValueCursor};
use crate::guard::{Guard, GuardCall, GuardEvaluator, GuardSite, Negation, NoGuards};
use crate::id::{RowId, TermId};
use smallvec::{SmallVec, smallvec};

use crate::plan::{
    ATOM_ARITY, AtomOperator, AtomShape, CyclicPlan, Executable, IndexChoice, JoinGroup,
    POSITION_GRAPH, POSITION_OBJECT, POSITION_PREDICATE, POSITION_SUBJECT, Parsed, PositionPlan,
    RulePlan, predicate_symbol,
};
use crate::stop::{StopSignal, is_stopped};
use crate::store::{Bound, Fact, PartitionRef, RelationStore};

// ── Budgets ─────────────────────────────────────────────────────────────────────

/// The maximum number of candidate solutions one evaluation may enumerate.
///
/// A "join step" is one partial or complete solution appended by a body-atom extension —
/// the unit that actually grows without bound when a rule set is accidentally Cartesian.
/// It is deliberately NOT "one committed derivation": committed derivations are already
/// bounded by [`MAX_STORED_FACTS`], so counting them would make this ceiling redundant,
/// while a blow-up that enumerates millions of candidates and commits three facts would
/// slip past unseen.
///
/// This is a `const`, not a parameter: a caller-supplied budget would mean two callers
/// running the same program over the same input get different answers — the same semantic
/// optionality the project's no-Cargo-features rule exists to prevent, merely arriving
/// through an argument. Consumption is *reported* in [`BudgetReport`] instead.
pub const MAX_JOIN_STEPS: u64 = 1 << 20;

/// The maximum number of facts (seeded plus derived) one evaluation's store may hold.
///
/// Sized so a saturated store stays comfortably inside a `wasm32` linear memory: at this
/// count the arrangement's columns, the row-id space and the term dictionary together are
/// a few tens of megabytes, well under the ceiling a browser imposes.
pub const MAX_STORED_FACTS: usize = 1 << 17;

/// The maximum bytes of interned term surfaces one evaluation's store may hold
/// ([`RelationStore::term_bytes`]).
///
/// Facts are counted by [`MAX_STORED_FACTS`], but a fact set of a legal size can still be
/// arbitrarily large if its terms are: one relation of a thousand megabyte-long IRIs is a
/// hundred-fold smaller in facts and a thousand-fold larger in bytes. This ceiling closes
/// that gap. It is checked on the seeded store and again after every round, so no future
/// term-minting extension can grow past it unobserved.
pub const MAX_TERM_ARENA_BYTES: usize = 1 << 24;

/// The floor of the default term-generating horizon ([`TermGeneratingLimit::Horizon`]).
pub const TERM_GENERATING_HORIZON_FLOOR: u64 = 256;

/// The term-generating rounds the default horizon grants per distinct term of the seeded
/// store ([`TermGeneratingLimit::Horizon`]).
pub const TERM_GENERATING_ROUNDS_PER_INPUT_TERM: u64 = 4;

/// The limit on TERM-GENERATING rounds ([`EvalOptions`]): rounds that commit a term a
/// guard ([`crate::guard`]) computed and the store had never interned.
///
/// # Why rounds, and why only these
///
/// The three fixed ceilings bound what a run HOLDS; none bounds how long a run that
/// holds little can keep going. A guard-free program cannot run away: every term it
/// can commit is a body binding or one of its own finitely many constants, so its
/// round count is bounded by its fact count. A guard can compute a NEW term each round
/// — `?n + 1`, `CONCAT(?s, "x")`, a triple term nesting its own match, a fresh blank
/// node — and a rule that feeds such a term back into its own body derives one more
/// fact per round, forever. The fact, arena and join ceilings stop it only after a
/// number of rounds whose total cost is super-linear in the ceiling (a rule that
/// re-reads everything it minted costs the square of the rounds); this limit stops it
/// after a bounded number of ROUNDS.
///
/// A round that commits only terms already interned is never counted: a
/// value-preserving recursion (a transitive closure, a label propagated down a chain)
/// is bounded by [`MAX_STORED_FACTS`] and runs as many rounds as it needs. So this
/// limit cannot bind a guard-free program at all — its constants are interned the first
/// time they are derived, and no guard exists to compute another.
///
/// Whether a guarded program terminates is undecidable, so any limit refuses some
/// program that terminates — a counter stepping `?n + 1` up to `FILTER (?n < N)`
/// terminates after `N` term-generating rounds for every `N`. The default is therefore
/// a DIVERGENCE criterion derived from the input, and the caller who knows a larger
/// bound states it ([`Self::Fixed`]). The limit can only REFUSE, never truncate — a
/// refused run returns no model, exactly as every other ceiling does — and it is part of
/// the result's identity: [`contract_hash_with`](crate::cache::contract_hash_with) folds
/// the limit in force into a guarded program's contract hash, so two runs under
/// different limits never claim the same calculus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TermGeneratingLimit {
    /// The default divergence criterion. With `N` the distinct terms of the seeded store
    /// (base facts, data blocks, and every predicate and graph name among them), a run is
    /// refused as DIVERGENT — [`EvalError::TermGenerationDiverged`], naming the rules that
    /// generated a term in the refused round — once it commits more than
    ///
    /// `max(TERM_GENERATING_HORIZON_FLOOR, TERM_GENERATING_ROUNDS_PER_INPUT_TERM × N)`
    ///
    /// term-generating rounds (256 and 4). The horizon is what a program whose term
    /// generation is BOUNDED BY ITS DATA needs: a depth counted along a chain, a label
    /// extended once per node or a value folded across a list derives each new term from
    /// a step through the input, one step per input term, so its derivation chains of
    /// generated terms — and so its term-generating rounds — are at most a small multiple
    /// of `N`. The floor admits a small computation bounded by a constant (a counter to a
    /// few hundred over a one-triple graph). A program still generating past the horizon
    /// is computing terms its input does not bound — a counter with no bound, a string
    /// extended every pass, a triple term nested every pass, a node minted from a node it
    /// minted — and is refused after at most the horizon's rounds. A program bounded by a
    /// constant past the horizon — a counter to 10,000 over one triple — terminates and
    /// is refused under this default; its caller states the bound with
    /// [`EvalOptions::with_max_term_generating_rounds`].
    #[default]
    Horizon,
    /// Exactly this many term-generating rounds; one more is refused with
    /// [`EvalError::BudgetExhausted`] naming [`BudgetResource::TermGeneratingRounds`].
    Fixed(u64),
}

impl TermGeneratingLimit {
    /// The term-generating rounds this limit permits a run whose seeded store holds
    /// `input_terms` distinct terms.
    #[must_use]
    pub fn rounds_for(self, input_terms: usize) -> u64 {
        match self {
            Self::Horizon => TERM_GENERATING_HORIZON_FLOOR.max(
                TERM_GENERATING_ROUNDS_PER_INPUT_TERM
                    .saturating_mul(u64::try_from(input_terms).unwrap_or(u64::MAX)),
            ),
            Self::Fixed(rounds) => rounds,
        }
    }
}

/// The caller's evaluation governors: today, the limit on term-generating rounds.
///
/// See [`TermGeneratingLimit`] for what the limit counts, why its default is derived
/// from the input, and why it cannot change a guard-free program's answer. Construct
/// with [`EvalOptions::default`] and state a fixed limit with
/// [`with_max_term_generating_rounds`](Self::with_max_term_generating_rounds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EvalOptions {
    /// The term-generating round limit.
    term_generating_limit: TermGeneratingLimit,
}

impl EvalOptions {
    /// Permit exactly `rounds` term-generating rounds ([`TermGeneratingLimit::Fixed`]);
    /// one more is refused with [`EvalError::BudgetExhausted`] naming
    /// [`BudgetResource::TermGeneratingRounds`].
    #[must_use]
    pub fn with_max_term_generating_rounds(mut self, rounds: u64) -> Self {
        self.term_generating_limit = TermGeneratingLimit::Fixed(rounds);
        self
    }

    /// Govern term-generating rounds by `limit`.
    #[must_use]
    pub fn with_term_generating_limit(mut self, limit: TermGeneratingLimit) -> Self {
        self.term_generating_limit = limit;
        self
    }

    /// The term-generating round limit in force.
    #[must_use]
    pub fn term_generating_limit(&self) -> TermGeneratingLimit {
        self.term_generating_limit
    }
}

/// What an evaluation actually consumed of the three fixed ceilings.
///
/// Returned on success ([`Evaluation::budget`]) and on failure
/// ([`EvalError::BudgetExhausted`]) alike; on failure the field naming the exhausted
/// resource holds the observation that proved the ceiling was passed, so the report is
/// never rounded down to the limit it exceeded.
///
/// Every field is a deterministic function of the input: the same program over the same
/// facts reports the same numbers on every target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetReport {
    /// Candidate solutions enumerated, against [`MAX_JOIN_STEPS`].
    join_steps: u64,
    /// Facts held by the store, against [`MAX_STORED_FACTS`].
    stored_facts: usize,
    /// Interned term surface bytes, against [`MAX_TERM_ARENA_BYTES`].
    term_arena_bytes: usize,
    /// Term-generating rounds run, against [`Self::term_generating_round_limit`].
    term_generating_rounds: u64,
    /// The term-generating round limit the run was governed by ([`EvalOptions`]).
    term_generating_round_limit: u64,
}

impl Default for BudgetReport {
    /// The zero measurement, governed by the default term-generating round limit.
    fn default() -> Self {
        Self::new(0, 0, 0)
    }
}

impl BudgetReport {
    /// A report over three already-measured coordinates.
    ///
    /// This crate's own evaluator fills the fields directly; the constructor exists so
    /// that one type can say "what did this evaluation consume" for a run this crate did
    /// not perform — `purrdf-entail`'s `Simple` lane, which copies a dataset and evaluates
    /// no program at all, reports the zero measurement through here rather than growing a
    /// second budget type for the case. It takes MEASUREMENTS, never limits: the three
    /// ceilings stay `const` and stay this crate's, so nothing here re-opens the
    /// caller-supplied-budget door the crate docs close.
    ///
    /// The coordinates mean exactly what they mean for [`evaluate`]: candidate solutions
    /// enumerated, facts held when evaluation stopped, and interned term surface bytes. An
    /// engine that reports numbers under a different definition is misreporting, not
    /// extending.
    pub fn new(join_steps: u64, stored_facts: usize, term_arena_bytes: usize) -> Self {
        Self {
            join_steps,
            stored_facts,
            term_arena_bytes,
            term_generating_rounds: 0,
            term_generating_round_limit: TERM_GENERATING_HORIZON_FLOOR,
        }
    }

    /// Candidate solutions enumerated across every round of every stratum.
    pub fn join_steps(self) -> u64 {
        self.join_steps
    }

    /// Facts held by the store when evaluation stopped (seeded plus derived).
    pub fn stored_facts(self) -> usize {
        self.stored_facts
    }

    /// Interned term surface bytes held by the store when evaluation stopped.
    pub fn term_arena_bytes(self) -> usize {
        self.term_arena_bytes
    }

    /// Term-generating rounds run ([`TermGeneratingLimit`]). Always zero
    /// for a guard-free program, which has no guard to compute a term.
    pub fn term_generating_rounds(self) -> u64 {
        self.term_generating_rounds
    }

    /// The term-generating round limit the run was governed by ([`EvalOptions`]): the
    /// fixed limit, or the horizon derived from the seeded store.
    pub fn term_generating_round_limit(self) -> u64 {
        self.term_generating_round_limit
    }
}

/// Which fixed ceiling an exhausted evaluation passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum BudgetResource {
    /// [`MAX_JOIN_STEPS`] — too many candidate solutions enumerated.
    JoinSteps,
    /// [`MAX_STORED_FACTS`] — too many facts seeded or derived.
    StoredFacts,
    /// [`MAX_TERM_ARENA_BYTES`] — too many interned term surface bytes.
    TermArenaBytes,
    /// The caller's fixed term-generating round limit ([`TermGeneratingLimit::Fixed`]) —
    /// too many rounds committed a guard-computed term.
    TermGeneratingRounds,
}

impl BudgetResource {
    /// The ceiling this resource is measured against, rendered for a diagnostic.
    fn limit(self, report: BudgetReport) -> u64 {
        match self {
            Self::JoinSteps => MAX_JOIN_STEPS,
            Self::StoredFacts => MAX_STORED_FACTS as u64,
            Self::TermArenaBytes => MAX_TERM_ARENA_BYTES as u64,
            Self::TermGeneratingRounds => report.term_generating_round_limit,
        }
    }

    /// The observation that tripped this resource, taken from `report`.
    fn observed(self, report: BudgetReport) -> u64 {
        match self {
            Self::JoinSteps => report.join_steps,
            Self::StoredFacts => report.stored_facts as u64,
            Self::TermArenaBytes => report.term_arena_bytes as u64,
            Self::TermGeneratingRounds => report.term_generating_rounds,
        }
    }

    /// The resource's name, for a diagnostic.
    fn name(self) -> &'static str {
        match self {
            Self::JoinSteps => "join steps",
            Self::StoredFacts => "stored facts",
            Self::TermArenaBytes => "term arena bytes",
            Self::TermGeneratingRounds => "term-generating rounds",
        }
    }
}

/// A per-rule allowance for one round's candidate enumeration.
///
/// The governor is the ported step budget, with the ceiling moved from a caller parameter
/// to [`MAX_JOIN_STEPS`]. Each rule task in a round is handed the SAME allowance — the
/// evaluation's remaining budget plus one — so a task that reaches it has proved the
/// ceiling is passed while bounding one round's work to `rules × allowance`. Whether the
/// round actually exceeded the ceiling is decided once, after the tasks are merged in
/// program order, so the decision never depends on which task ran first.
#[derive(Debug, Clone, Copy)]
struct StepGovernor {
    /// The most candidates this task may enumerate.
    allowance: u64,
    /// Candidates enumerated so far.
    consumed: u64,
}

impl StepGovernor {
    /// A governor permitting `allowance` candidates.
    fn new(allowance: u64) -> Self {
        Self {
            allowance,
            consumed: 0,
        }
    }

    /// Whether the allowance is spent — the next candidate may NOT be enumerated.
    #[inline]
    fn spent(self) -> bool {
        self.consumed >= self.allowance
    }

    /// Record one enumerated candidate.
    #[inline]
    fn charge(&mut self) {
        self.consumed = self.consumed.saturating_add(1);
    }
}

// ── Errors ──────────────────────────────────────────────────────────────────────

/// Why an evaluation could not run, or could not run to completion.
///
/// Every variant is a hard refusal. There is no partial answer and no best-effort mode:
/// an answer this crate returns is the complete least model of the program it was given.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EvalError {
    /// A clause's head is existential, disjunctive, conjunctive or `false`, so it is not a
    /// Datalog rule and this evaluator has no semantics for it.
    ///
    /// The DL-clause IR ([`crate::clause`]) represents all five head forms in one type, so a
    /// consumer of any of them needs no second IR: [`crate::chase`] takes the existential and
    /// conjunctive forms, `purrdf-entail`'s OWL-Direct hypertableau case-splits the
    /// disjunctive one (over its own concept-id atoms, through this crate's
    /// [`crate::clause::HeadForm`]), and both are refused by name HERE rather than
    /// parsed away. A semi-naive least-fixpoint evaluator computes the
    /// least model of a set of DEFINITE clauses — exactly one head atom, no quantifier: an
    /// existential mints witnesses, a disjunction has no single least model, a conjunction
    /// abbreviates several clauses at once, and `false` derives nothing while asserting
    /// its body is unsatisfiable. None of the four is a definite clause, so each is
    /// refused by its OWN name here rather than silently ignored, silently treated as
    /// atomic, or reported under a neighbouring form's name.
    ///
    /// The conjunctive case deserves the emphasis: `→ p(x) ∧ q(x)` *is* equivalent to two
    /// Datalog rules over one body, and this evaluator could have split it. It does not,
    /// because a [`Derivation`] names its producing clause by authored index, so splitting
    /// one clause into two would renumber the program and move an observable. A caller who
    /// wants the split performs it before compiling.
    NonDatalogHead {
        /// The clause's index in authored program order.
        rule: usize,
        /// The head form that has no Datalog semantics.
        form: HeadForm,
    },
    /// A negative dependency edge lies inside a cycle, so no stratification exists.
    ///
    /// The payload names the offending edge and one concrete cycle through it, in
    /// dependency order, so the diagnostic points at the rules to fix rather than merely
    /// declaring the program unsupported.
    NonStratifiable {
        /// The head predicate of the rule carrying the negated body atom.
        head: String,
        /// The negated body predicate the head depends on.
        negated: String,
        /// One cycle containing that edge, as a predicate sequence starting and
        /// implicitly closing at `head`.
        cycle: Vec<String>,
    },
    /// A rule head is not range-restricted: it carries a variable no positive body atom
    /// can bind.
    ///
    /// Admitting such a rule would mean either fabricating a term or silently dropping
    /// derivations, so it is refused at compile time rather than at some data-dependent
    /// point in the fixpoint.
    UnboundHeadVariable {
        /// The rule's index in authored program order.
        rule: usize,
        /// The unbindable variable, as authored.
        variable: String,
    },
    /// A fixed ceiling was passed. The report is accurate at the point evaluation stopped.
    BudgetExhausted {
        /// Which ceiling.
        resource: BudgetResource,
        /// Consumption of all three ceilings when evaluation stopped.
        report: BudgetReport,
    },
    /// The run passed the default term-generating horizon ([`TermGeneratingLimit::Horizon`]):
    /// it kept committing guard-computed terms for more rounds than a program whose term
    /// generation is bounded by its input needs. Distinct from
    /// [`Self::BudgetExhausted`], whose [`BudgetResource::TermGeneratingRounds`] is a
    /// limit the caller stated, because this one is a verdict about the program: it
    /// names the rules that generated a term in the refused round.
    TermGenerationDiverged {
        /// The rules whose derivations committed a guard-computed term in the refused
        /// round, in authored program order.
        rules: Vec<usize>,
        /// The distinct terms of the seeded store the horizon was derived from.
        input_terms: usize,
        /// Consumption when evaluation stopped; its
        /// [`term_generating_round_limit`](BudgetReport::term_generating_round_limit) is
        /// the horizon.
        report: BudgetReport,
    },
    /// The caller's [`crate::stop::StopSignal`] fired at a round boundary.
    ///
    /// A refusal exactly as total as [`Self::BudgetExhausted`]: there is no partial least
    /// model here either, and the rounds already committed are not an answer to anything.
    /// It is a DISTINCT variant because the two say different things to a caller — a fixed
    /// ceiling was passed by the program and the data, whereas this run was stopped by the
    /// host that asked for it — and only one of them is a reason to change the input.
    ///
    /// The report is accurate at the point evaluation stopped, so a host can say what the
    /// stopped run had already consumed rather than only that it was stopped.
    Stopped {
        /// Consumption of all three ceilings when the signal was observed.
        report: BudgetReport,
    },
    /// A guard ([`crate::guard`]) could not be evaluated: the caller's
    /// [`GuardEvaluator`] reported an error.
    ///
    /// Total, like every refusal here: a guard that could not decide is never read as
    /// a guard that said no, because that would silently drop derivations.
    Guard {
        /// The rule's index in authored program order.
        rule: usize,
        /// Which of the rule's guards failed.
        site: GuardSite,
        /// The guard's content identity.
        guard: String,
        /// The evaluator's message.
        message: String,
    },
    /// A rule carries a guard that reads the evaluation model
    /// ([`crate::guard::GuardReads::Model`]), which the stratified fixpoint cannot
    /// place: what the guard reads, and with which polarity, is caller code rather than
    /// clause text, so no stratification can be shown to decide it against completed
    /// relations. The ordered schedule ([`crate::schedule`]) evaluates such a rule
    /// under the schedule its rule language defines.
    ModelReadingGuard {
        /// The rule's index in authored program order.
        rule: usize,
    },
    /// No stratification of the RULES exists: a closed dependency — through a negation
    /// or into a run-once rule — lies inside a dependency cycle
    /// ([`crate::schedule::stratify_rules`]).
    ///
    /// The payload names one closed edge and one concrete cycle through it, as rule
    /// indices in dependency order, starting and implicitly closing at `rule`.
    NonStratifiableRules {
        /// The rule whose closed dependency lies in the cycle.
        rule: usize,
        /// The rule it depends on through that closed edge.
        depends_on: usize,
        /// The cycle, `rule -> depends_on -> … -> rule`, as rule indices.
        cycle: Vec<usize>,
    },
    /// An ordered schedule ([`crate::schedule::Schedule`]) is not a partition of the
    /// program's rules: a rule is scheduled twice, never, or does not exist.
    MalformedSchedule {
        /// What is wrong with the schedule.
        detail: String,
    },
    /// The caller's [`LayerHooks`](crate::schedule::LayerHooks) reported an error.
    LayerHook {
        /// The layer whose hook failed, or `None` for the end-of-evaluation hook.
        layer: Option<usize>,
        /// The hook's message.
        message: String,
    },
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonDatalogHead { rule, form } => write!(
                f,
                "clause {rule} has {} {form} head: the semi-naive evaluator runs Datalog \
                 clauses (one head atom, no existential) only",
                form.article()
            ),
            Self::NonStratifiable {
                head,
                negated,
                cycle,
            } => write!(
                f,
                "program is not stratifiable: the negated dependency {head} -> not {negated} \
                 lies inside the cycle {} -> {head}",
                cycle.join(" -> ")
            ),
            Self::UnboundHeadVariable { rule, variable } => write!(
                f,
                "rule {rule} is not range-restricted: head variable {variable} is not bound by \
                 any positive body atom"
            ),
            Self::BudgetExhausted {
                resource: BudgetResource::TermGeneratingRounds,
                report,
            } => write!(
                f,
                "evaluation exceeded the term-generating rounds limit: {} observed, {} \
                 permitted; if the rule set terminates, raise the limit with \
                 EvalOptions::with_max_term_generating_rounds",
                report.term_generating_rounds, report.term_generating_round_limit
            ),
            Self::TermGenerationDiverged {
                rules,
                input_terms,
                report,
            } => {
                let rules: Vec<String> = rules.iter().map(ToString::to_string).collect();
                write!(
                    f,
                    "evaluation diverged: {} rounds committed a term the store did not hold, \
                     past the horizon of {} such rounds that the input's {input_terms} terms \
                     grant (max({TERM_GENERATING_HORIZON_FLOOR}, \
                     {TERM_GENERATING_ROUNDS_PER_INPUT_TERM} per input term)); rule(s) {} \
                     generated a term in the last round; if the rule set terminates, state its \
                     bound with EvalOptions::with_max_term_generating_rounds",
                    report.term_generating_rounds,
                    report.term_generating_round_limit,
                    rules.join(", ")
                )
            }
            Self::BudgetExhausted { resource, report } => write!(
                f,
                "evaluation exceeded the fixed {} ceiling: {} observed, {} permitted",
                resource.name(),
                resource.observed(*report),
                resource.limit(*report)
            ),
            Self::Stopped { report } => write!(
                f,
                "evaluation was stopped by the caller's stop signal after {} join steps, \
                 holding {} facts: no least model was computed",
                report.join_steps, report.stored_facts
            ),
            Self::Guard {
                rule,
                site,
                guard,
                message,
            } => write!(
                f,
                "rule {rule}'s {site} ({guard}) could not be evaluated: {message}"
            ),
            Self::ModelReadingGuard { rule } => write!(
                f,
                "rule {rule} carries a guard that reads the evaluation model, which the \
                 stratified fixpoint cannot place; evaluate it under an ordered schedule"
            ),
            Self::NonStratifiableRules {
                rule,
                depends_on,
                cycle,
            } => {
                let path: Vec<String> = cycle.iter().map(|index| format!("rule {index}")).collect();
                write!(
                    f,
                    "rule set is not stratifiable: rule {rule} has a closed dependency on rule \
                     {depends_on} (through a negation or a run-once rule) inside the cycle {} -> \
                     rule {rule}",
                    path.join(" -> ")
                )
            }
            Self::MalformedSchedule { detail } => write!(f, "malformed schedule: {detail}"),
            Self::LayerHook {
                layer: Some(layer),
                message,
            } => write!(f, "the layer hook of layer {layer} failed: {message}"),
            Self::LayerHook {
                layer: None,
                message,
            } => write!(f, "the end-of-evaluation hook failed: {message}"),
        }
    }
}

impl std::error::Error for EvalError {}

// ── Results ─────────────────────────────────────────────────────────────────────

/// One committed derivation: a fact, the rule that produced it and the body facts it was
/// produced from.
///
/// `sources` is in the rule's AUTHORED body order regardless of the order the planner
/// chose to execute the atoms in, so provenance reads the way the rule is written.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Derivation {
    /// The derived fact.
    fact: Fact,
    /// The producing rule's index in authored program order.
    rule: usize,
    /// The matched positive body facts, in authored body order.
    sources: Vec<Fact>,
    /// `1 + max(source proof heights)`; a seeded fact has height 0.
    proof_height: u32,
}

impl Derivation {
    /// The derived fact.
    pub fn fact(&self) -> &Fact {
        &self.fact
    }

    /// The producing rule's index in authored program order.
    pub fn rule(&self) -> usize {
        self.rule
    }

    /// The matched positive body facts, in authored body order.
    pub fn sources(&self) -> &[Fact] {
        &self.sources
    }

    /// The derivation's proof height: `1 + max(source proof heights)`, seeds being 0.
    pub fn proof_height(&self) -> u32 {
        self.proof_height
    }
}

/// A completed evaluation: the saturated store, its derivations and the budget report.
#[derive(Debug, Clone)]
pub struct Evaluation {
    /// The least model: the seeded EDB plus every derived fact.
    facts: RelationStore,
    /// Every derivation, in lexical `(fact, rule, sources)` order.
    derivations: Vec<Derivation>,
    /// What the run consumed of the three fixed ceilings.
    budget: BudgetReport,
}

impl Evaluation {
    /// The least model: the seeded EDB plus every derived fact.
    pub fn facts(&self) -> &RelationStore {
        &self.facts
    }

    /// Take ownership of the least model.
    pub fn into_facts(self) -> RelationStore {
        self.facts
    }

    /// Every derivation, in a total lexical order.
    pub fn derivations(&self) -> &[Derivation] {
        &self.derivations
    }

    /// What the run consumed of the three fixed ceilings.
    pub fn budget(&self) -> BudgetReport {
        self.budget
    }
}

// ── Compilation: stratification with a named cycle ──────────────────────────────

/// Compile a rule program into the executor's [`Executable`], or refuse it.
///
/// This is [`crate::plan`]'s pipeline with three refusals attached: a clause whose head is
/// not a single unquantified atom is reported by head form, a non-stratifiable program is
/// reported with the concrete cycle its negative edge sits in, and a clause whose head
/// carries a variable no positive body atom can bind is reported as not range-restricted.
/// All three are hard errors; none has a best-effort fallback.
///
/// # Errors
///
/// [`EvalError::NonDatalogHead`], [`EvalError::NonStratifiable`] or
/// [`EvalError::UnboundHeadVariable`].
pub fn compile(rules: Vec<DlClause>) -> Result<Executable, EvalError> {
    // The head form is decided FIRST, because the other two checks are both defined in
    // terms of a single head atom: an existential, a disjunctive or an empty head has no
    // "the head predicate" to stratify on and no "the head variables" to range-restrict.
    // Reporting either of those instead would be reporting a consequence of the real
    // defect. This is the same gate `Parsed::new` enforces below; it runs here so the
    // diagnostic is an `EvalError` alongside the other two.
    for (index, rule) in rules.iter().enumerate() {
        let form = rule.head_form();
        if !form.is_datalog() {
            return Err(EvalError::NonDatalogHead { rule: index, form });
        }
    }

    // A guard that reads the model has no place in a stratification (see
    // [`EvalError::ModelReadingGuard`]); it is refused before the stratifier is asked
    // to decide over edges the clause does not state.
    if let Some(index) = rules.iter().position(DlClause::reads_model) {
        return Err(EvalError::ModelReadingGuard { rule: index });
    }

    // Stratifiability is decided next, and against the BORROWED program so the failing
    // branch can still walk the rules to name the offending cycle. It is the more
    // fundamental of the remaining two defects: an unstratifiable program has no least
    // model to be safe with respect to, so reporting a per-rule safety violation instead
    // would point at a symptom.
    if crate::plan::stratify(&rules).is_none() {
        return Err(negative_cycle(&rules));
    }

    // Range restriction ranges over all FOUR positions: a head that writes a variable
    // predicate or a variable graph needs that variable bound by the positive body just as
    // much as a head subject does, and a positive body atom binds all four of its own.
    check_range_restricted(&rules)?;

    Ok(Parsed::new(rules)
        .expect("every head form was just established to be atomic on the same rules")
        .stratify()
        .expect("stratifiability was just established on the same rules")
        .plan()
        .into_executable())
}

/// Refuse a rule whose head carries a variable nothing in its body binds.
///
/// Range restriction ranges over all FOUR positions of every head atom: a head that
/// writes a variable predicate or a variable graph needs that variable bound just as
/// much as a head subject does. A variable is bound by a positive body atom (which
/// binds all four of its own positions) or by a guard's output
/// ([`DlClause::bound_variables`]); a negated atom or a negated conjunction binds
/// nothing.
///
/// # Errors
///
/// [`EvalError::UnboundHeadVariable`] naming the first offending rule and variable.
pub(crate) fn check_range_restricted(rules: &[DlClause]) -> Result<(), EvalError> {
    for (index, rule) in rules.iter().enumerate() {
        let bound = rule.bound_variables();
        for head in rule.head_atoms() {
            for term in head.terms() {
                if let Some(name) = term.variable()
                    && !bound.contains(name)
                {
                    return Err(EvalError::UnboundHeadVariable {
                        rule: index,
                        variable: name.to_owned(),
                    });
                }
            }
        }
    }
    Ok(())
}

/// Name one negative dependency edge that lies inside a cycle.
///
/// The edges are scanned in authored program order and, for each negated body atom, the
/// dependency graph is searched breadth-first from the negated predicate for a path back
/// to the head. The FIRST such edge in program order is reported, with the shortest path
/// the search found — deterministic, because both the edge scan and the adjacency sets are
/// in a fixed order.
///
/// # Panics
///
/// Panics if no negative edge lies in a cycle. This is called only after
/// [`crate::plan::stratify`] has already proven the program non-stratifiable, and that is
/// exactly the condition, so reaching it would be a contradiction in the stratifier.
fn negative_cycle(rules: &[DlClause]) -> EvalError {
    // head -> {body}: "head depends on body", in a fixed lexical order, built from the
    // SAME edge set `crate::plan::stratify` decided over — including the coupling edges a
    // variable predicate position adds. Searching a smaller graph than the decision was
    // taken over could fail to find the cycle the stratifier proved exists.
    let (_, edges) = crate::plan::dependency_edges(rules);
    let mut depends: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (head, body, _negative) in &edges {
        depends
            .entry(head.clone())
            .or_default()
            .insert(body.clone());
    }

    for rule in rules {
        for head in rule.head_atoms() {
            let head_symbol = predicate_symbol(head);
            let negated_atoms = rule
                .body()
                .iter()
                .filter(|atom| atom.is_negated())
                .chain(rule.negations().iter().flat_map(Negation::atoms));
            for atom in negated_atoms {
                let negated = predicate_symbol(atom);
                let Some(path) = shortest_dependency_path(&depends, &negated, &head_symbol) else {
                    continue;
                };
                let mut cycle = vec![head_symbol.clone()];
                cycle.extend(path.into_iter().map(str::to_owned));
                return EvalError::NonStratifiable {
                    head: head_symbol,
                    negated,
                    cycle,
                };
            }
        }
    }
    unreachable!("a non-stratifiable program has a negative edge inside a dependency cycle")
}

/// The shortest `from -> … -> to` path through `depends`, inclusive of both ends.
///
/// A breadth-first search over lexically ordered adjacency, so the path is a pure function
/// of the rule program rather than of a traversal accident.
fn shortest_dependency_path<'a>(
    depends: &'a BTreeMap<String, BTreeSet<String>>,
    from: &'a str,
    to: &str,
) -> Option<Vec<&'a str>> {
    let mut parent: BTreeMap<&str, &str> = BTreeMap::new();
    let mut queue: VecDeque<&str> = VecDeque::from([from]);
    let mut seen: BTreeSet<&str> = BTreeSet::from([from]);
    while let Some(node) = queue.pop_front() {
        if node == to {
            // Walk the parent chain back to `from`, then reverse it.
            let mut path = vec![node];
            let mut cursor = node;
            while cursor != from {
                cursor = parent[cursor];
                path.push(cursor);
            }
            path.reverse();
            return Some(path);
        }
        for next in depends.get(node).into_iter().flatten() {
            if seen.insert(next.as_str()) {
                parent.insert(next.as_str(), node);
                queue.push_back(next.as_str());
            }
        }
    }
    None
}

// ── Semi-naive scan modes ───────────────────────────────────────────────────────

/// The semi-naive delta as a contiguous [`RowId`] range `[lo, hi)`.
///
/// Row ids are minted densely in the sorted commit loop, so a round's committed rows are
/// ALWAYS a contiguous span: delta membership is one range compare, with no per-round
/// bitset allocation and no hashing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Delta {
    /// Inclusive lower row index of the round's committed span.
    pub(crate) lo: usize,
    /// Exclusive upper row index of the round's committed span.
    pub(crate) hi: usize,
}

impl Delta {
    /// The round-1 seed: every accumulated row `[0, row_count)` is new this round.
    pub(crate) fn all(row_count: usize) -> Self {
        Self {
            lo: 0,
            hi: row_count,
        }
    }

    /// Whether `row` falls in the delta's committed span.
    #[inline]
    fn contains(self, row: RowId) -> bool {
        let index = row.index();
        self.lo <= index && index < self.hi
    }
}

/// Compile-time scan-mode code: bind only rows in the delta.
const SCAN_DELTA: u8 = 0;
/// Compile-time scan-mode code: bind any row.
const SCAN_FULL: u8 = 1;
/// Compile-time scan-mode code: bind only rows NOT in the delta.
const SCAN_OLD_ONLY: u8 = 2;

/// The semi-naive scan mode for one positive body atom in one decomposition position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scan {
    /// The atom at the delta position: only rows committed last round.
    Delta,
    /// A position before the delta position: any row.
    Full,
    /// A position after the delta position: only rows NOT committed last round.
    OldOnly,
}

/// The scan mode for a positive atom at `position`, given the round's delta position.
#[inline]
fn scan_for(position: usize, delta_position: usize) -> Scan {
    match position.cmp(&delta_position) {
        std::cmp::Ordering::Less => Scan::Full,
        std::cmp::Ordering::Equal => Scan::Delta,
        std::cmp::Ordering::Greater => Scan::OldOnly,
    }
}

/// The monomorphized per-row semi-naive keep test.
///
/// `SCAN` is a compile-time constant, so this folds to a single arm with no per-tuple
/// branch on the scan mode.
#[inline]
fn keep_row<const SCAN: u8>(delta: Delta, row: RowId) -> bool {
    match SCAN {
        SCAN_FULL => true,
        SCAN_DELTA => delta.contains(row),
        SCAN_OLD_ONLY => !delta.contains(row),
        _ => unreachable!("SCAN is SCAN_DELTA, SCAN_FULL or SCAN_OLD_ONLY"),
    }
}

/// The runtime-dispatched form of [`keep_row`], for the one call site whose scan mode is
/// not a monomorphization parameter (the cyclic component's ground source capture).
#[inline]
fn keep_row_for_scan(scan: Scan, delta: Delta, row: RowId) -> bool {
    match scan {
        Scan::Delta => keep_row::<SCAN_DELTA>(delta, row),
        Scan::Full => keep_row::<SCAN_FULL>(delta, row),
        Scan::OldOnly => keep_row::<SCAN_OLD_ONLY>(delta, row),
    }
}

/// Compile-time index code: full scan.
const INDEX_ANY: u8 = 0;
/// Compile-time index code: subject bound.
const INDEX_SUBJECT: u8 = 1;
/// Compile-time index code: object bound.
const INDEX_OBJECT: u8 = 2;
/// Compile-time index code: both columns bound.
const INDEX_BOTH: u8 = 3;

// ── The physical binding frame ──────────────────────────────────────────────────

/// One matched body row, held as ids only.
///
/// A source is `Copy` and carries all FOUR of the matched quad's positions: with the
/// predicate as data, an atom's predicate is no longer recoverable from the rule text, so
/// the row has to say which one it matched. The join never renders a term surface, never
/// clones a `String` and never re-hashes a fact; surfaces are resolved once, for committed
/// winners only, when the [`Derivation`] is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceRow {
    /// Index into the producing rule's authored body.
    body_index: usize,
    /// The matched row's subject term.
    subject: TermId,
    /// The matched row's predicate term — the key of the partition it came from.
    predicate: TermId,
    /// The matched row's object term.
    object: TermId,
    /// The matched row's graph term — the other half of that partition key.
    graph: TermId,
    /// The matched row's store-global row id — the key of the proof-height column.
    row: RowId,
}

impl SourceRow {
    /// This row as a [`Fact`] of lexical surfaces.
    fn fact(self, rel: &RelationStore) -> Fact {
        let interner = rel.interner();
        Fact {
            subject: interner.resolve(self.subject).to_owned(),
            predicate: interner.resolve(self.predicate).to_owned(),
            object: interner.resolve(self.object).to_owned(),
            graph: interner.resolve(self.graph).to_owned(),
        }
    }
}

/// A partial solution of one rule's positive body.
///
/// Bindings live in a flat frame indexed by the plan's variable slots, and hold interned
/// [`TermId`]s rather than surfaces, so a probe is an integer compare.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SlotSolution {
    /// One slot per plan variable, `None` until an atom binds it.
    ///
    /// Inline rather than heap. This frame is CLONED once per surviving candidate row
    /// in [`extend_atom`], so a heap frame is one allocation per intermediate solution
    /// — the join's most frequent allocation by a wide margin, and one the evaluator
    /// paid unconditionally.
    ///
    /// Capacity 8 is measured, not assumed: 8 is the widest frame the OWL 2 RL rule
    /// table and this workspace's own suites produce, so every real frame lives inline.
    /// It is cheap to be generous here because `Option<TermId>` occupies 4 bytes
    /// through [`Id`](crate::id::Id)'s `NonZeroU32` niche — the whole frame is 32
    /// inline bytes, less than the two pointers plus capacity a `Vec` header spends
    /// before addressing any element. A ninth variable does not break: it spills to the
    /// heap and behaves exactly as it did.
    bindings: SmallVec<[Option<TermId>; 8]>,
    /// The matched body rows, in physical execution order until the plan's swap program
    /// restores authored order.
    sources: Vec<SourceRow>,
    /// Slots a guard bound to a surface the store has never interned, with that
    /// surface. Empty — and therefore allocation-free to clone — for every solution of
    /// a guard-free rule; a slot listed here is `None` in `bindings`.
    computed: Vec<(usize, Box<str>)>,
}

/// What one frame slot holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotValue<'s> {
    /// A term the store has interned.
    Interned(TermId),
    /// A guard-computed surface the store has never interned.
    Computed(&'s str),
    /// Nothing yet.
    Unbound,
}

impl SlotSolution {
    /// The empty substitution over a frame of `slot_count` variables.
    fn empty(slot_count: usize) -> Self {
        Self {
            bindings: smallvec![None; slot_count],
            sources: Vec::new(),
            computed: Vec::new(),
        }
    }

    /// The interned value bound in `slot`, if any.
    #[inline]
    fn get(&self, slot: usize) -> Option<TermId> {
        self.bindings[slot]
    }

    /// Everything `slot` may hold, a computed surface included.
    fn value(&self, slot: usize) -> SlotValue<'_> {
        if let Some(id) = self.bindings[slot] {
            return SlotValue::Interned(id);
        }
        self.computed
            .iter()
            .find(|(computed, _)| *computed == slot)
            .map_or(SlotValue::Unbound, |(_, surface)| {
                SlotValue::Computed(surface)
            })
    }

    /// The lexical surface bound in `slot`, if any.
    fn surface<'s>(&'s self, slot: usize, rel: &'s RelationStore) -> Option<&'s str> {
        match self.value(slot) {
            SlotValue::Interned(id) => Some(rel.interner().resolve(id)),
            SlotValue::Computed(surface) => Some(surface),
            SlotValue::Unbound => None,
        }
    }

    /// Bind `slot` to `surface`: as an interned id when the store knows the term, as a
    /// computed surface otherwise.
    fn bind_surface(&mut self, slot: usize, surface: &str, rel: &RelationStore) {
        match rel.term_id(surface) {
            Some(id) => self.bindings[slot] = Some(id),
            None => self.computed.push((slot, surface.into())),
        }
    }
}

// ── The indexed binary join ─────────────────────────────────────────────────────

/// What one argument position resolves to under a partial solution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PositionValue {
    /// Pinned to this interned term — a constant the store knows, or a bound variable.
    Known(TermId),
    /// Free: the scan binds it.
    Free,
    /// Pinned to a term the store has never interned, so no row can match.
    Missing,
}

impl PositionValue {
    /// The pinned id, or `None` when the position is free.
    ///
    /// [`Missing`](Self::Missing) never reaches here: a solution carrying one is discarded
    /// before any partition is addressed.
    #[inline]
    fn known(self) -> Option<TermId> {
        match self {
            Self::Known(id) => Some(id),
            Self::Free | Self::Missing => None,
        }
    }
}

/// Resolve the constant positions of `shape` ONCE per operator invocation.
///
/// A constant's surface does not depend on the partial solution, so its dictionary probe
/// is hoisted out of the per-solution loop exactly as it was before the predicate became a
/// term. `None` marks a variable position, resolved per solution.
fn constant_positions(
    shape: &AtomShape,
    rel: &RelationStore,
) -> [Option<PositionValue>; ATOM_ARITY] {
    std::array::from_fn(|position| {
        shape.positions()[position].constant().map(|surface| {
            rel.term_id(surface)
                .map_or(PositionValue::Missing, PositionValue::Known)
        })
    })
}

/// Extend every partial solution by index-selecting `operator`'s matching rows.
///
/// This is the ONE-TIME scan-mode dispatch: once per operator invocation — not per row —
/// the scan mode is lifted to a `const SCAN` monomorphization parameter, so the per-row
/// delta filter has no runtime branch.
fn extend_slot_solutions(
    operator: &AtomOperator,
    rel: &RelationStore,
    delta: Delta,
    scan: Scan,
    solutions: &[SlotSolution],
    governor: &mut StepGovernor,
) -> Vec<SlotSolution> {
    match scan {
        Scan::Delta => {
            extend_slot_operator::<SCAN_DELTA>(operator, rel, delta, solutions, governor)
        }
        Scan::Full => extend_slot_operator::<SCAN_FULL>(operator, rel, delta, solutions, governor),
        Scan::OldOnly => {
            extend_slot_operator::<SCAN_OLD_ONLY>(operator, rel, delta, solutions, governor)
        }
    }
}

/// Dispatch one operator to its statically-shaped index kernel.
fn extend_slot_operator<const SCAN: u8>(
    operator: &AtomOperator,
    rel: &RelationStore,
    delta: Delta,
    solutions: &[SlotSolution],
    governor: &mut StepGovernor,
) -> Vec<SlotSolution> {
    match operator.index() {
        IndexChoice::Any => {
            extend_atom::<SCAN, INDEX_ANY>(operator, rel, delta, solutions, governor)
        }
        IndexChoice::Subject => {
            extend_atom::<SCAN, INDEX_SUBJECT>(operator, rel, delta, solutions, governor)
        }
        IndexChoice::Object => {
            extend_atom::<SCAN, INDEX_OBJECT>(operator, rel, delta, solutions, governor)
        }
        IndexChoice::Both => {
            extend_atom::<SCAN, INDEX_BOTH>(operator, rel, delta, solutions, governor)
        }
    }
}

/// The one arity-4 atom kernel: address the partitions the predicate and graph positions
/// admit, and inside each one select exactly the rows the `(subject, object)` index admits.
///
/// The two halves are independent, which is the whole performance argument. `INDEX` is a
/// monomorphization parameter, so the subject/object access path is decided at compile
/// time and is the SAME path a predicate-keyed store would have taken. The partition half
/// is a single ordered-map probe whenever the predicate and graph are known
/// ([`AtomShape::addresses_one_partition`]) — which is every atom of every rule that
/// names its predicate — and only degrades to a sweep of matching partitions when the rule
/// genuinely quantifies over the predicate or the graph.
fn extend_atom<const SCAN: u8, const INDEX: u8>(
    operator: &AtomOperator,
    rel: &RelationStore,
    delta: Delta,
    solutions: &[SlotSolution],
    governor: &mut StepGovernor,
) -> Vec<SlotSolution> {
    let shape = operator.shape();
    let body_index = operator.body_index();
    let constants = constant_positions(shape, rel);
    let mut next = Vec::new();

    'solutions: for solution in solutions {
        if governor.spent() {
            break;
        }
        // Resolve all four positions: constants were resolved once above, a bound variable
        // reads its frame slot, a free variable is bound by the scan.
        let mut values = [PositionValue::Free; ATOM_ARITY];
        for position in 0..ATOM_ARITY {
            values[position] = match (&shape.positions()[position], constants[position]) {
                (PositionPlan::Constant(_), Some(resolved)) => resolved,
                (PositionPlan::Constant(_), None) => {
                    unreachable!("a constant position always has a hoisted resolution")
                }
                (PositionPlan::Bound(slot), _) => solution
                    .get(*slot)
                    .map_or(PositionValue::Missing, PositionValue::Known),
                (PositionPlan::Free(_), _) => PositionValue::Free,
            };
        }
        if values.contains(&PositionValue::Missing) {
            // A pinned position the store has never interned matches nothing at all.
            continue;
        }

        // The `(subject, object)` index bound. `INDEX` is a compile-time constant, and the
        // planner only emits a bound index for a position it has proven known.
        let bound = match INDEX {
            INDEX_ANY => Bound::Any,
            INDEX_SUBJECT => match values[POSITION_SUBJECT].known() {
                Some(subject) => Bound::Subject(subject),
                None => continue,
            },
            INDEX_OBJECT => match values[POSITION_OBJECT].known() {
                Some(object) => Bound::Object(object),
                None => continue,
            },
            INDEX_BOTH => match (
                values[POSITION_SUBJECT].known(),
                values[POSITION_OBJECT].known(),
            ) {
                (Some(subject), Some(object)) => Bound::Both(subject, object),
                _ => continue,
            },
            _ => unreachable!("INDEX is a planned index code"),
        };

        for partition in rel.partitions(
            values[POSITION_PREDICATE].known(),
            values[POSITION_GRAPH].known(),
        ) {
            let (predicate, graph) = (partition.predicate(), partition.graph());
            let mut cursor = partition.select(bound);
            while let Some((subject, object, row)) = cursor.next() {
                if !keep_row::<SCAN>(delta, row) {
                    continue;
                }
                let matched = [subject, predicate, object, graph];
                // The generalized diagonal filter: any two positions holding the same
                // variable must agree, whichever two they are.
                if !shape
                    .equalities()
                    .iter()
                    .all(|&(left, right)| matched[left] == matched[right])
                {
                    continue;
                }
                if governor.spent() {
                    break 'solutions;
                }
                let mut merged = solution.clone();
                for (position, plan) in shape.positions().iter().enumerate() {
                    if let PositionPlan::Free(slot) = plan {
                        merged.bindings[*slot] = Some(matched[position]);
                    }
                }
                merged.sources.push(SourceRow {
                    body_index,
                    subject,
                    predicate,
                    object,
                    graph,
                    row,
                });
                governor.charge();
                next.push(merged);
            }
        }
    }
    next
}

// ── The leapfrog triejoin ───────────────────────────────────────────────────────

/// One relation's distinct, sorted trie-level values under a fixed semi-naive scan.
///
/// `SCAN` stays a monomorphization parameter here exactly as it is in the binary kernels,
/// so a leapfrog seek pays no per-row branch on the scan mode either.
#[derive(Debug)]
struct FilteredValueCursor<'a, const SCAN: u8, const COLUMN: u8> {
    /// The underlying globally value-ordered cursor.
    rows: ValueCursor<'a, COLUMN>,
    /// The round's delta span.
    delta: Delta,
    /// The current distinct admitted value, if any.
    current: Option<TermId>,
}

impl<'a, const SCAN: u8, const COLUMN: u8> FilteredValueCursor<'a, SCAN, COLUMN> {
    /// A filtered cursor positioned at its first admitted value.
    fn new(rows: ValueCursor<'a, COLUMN>, delta: Delta) -> Self {
        let mut cursor = Self {
            rows,
            delta,
            current: None,
        };
        cursor.fill(None);
        cursor
    }

    /// Fill `current` with the next distinct admitted value, skipping `prior`.
    fn fill(&mut self, prior: Option<TermId>) -> Option<TermId> {
        self.current = None;
        for (value, row) in self.rows.by_ref() {
            if keep_row::<SCAN>(self.delta, row) && Some(value) != prior {
                self.current = Some(value);
                break;
            }
        }
        self.current
    }

    /// Advance to the first admitted value `>= target`.
    fn seek(&mut self, target: TermId) -> Option<TermId> {
        if self.current.is_some_and(|value| value >= target) {
            return self.current;
        }
        self.rows.seek(target);
        self.fill(None)
    }

    /// Advance past the current value to the next distinct admitted one.
    fn advance(&mut self) -> Option<TermId> {
        let prior = self.current;
        self.fill(prior)
    }
}

/// Runtime scan/orientation selection at cursor construction.
///
/// Each variant wraps a const-generic filtered cursor, so the scan mode is resolved once
/// per cursor rather than per seek.
#[derive(Debug)]
enum LeapfrogValueCursor<'a> {
    /// Subject-ordered, delta rows only.
    DeltaSubject(FilteredValueCursor<'a, SCAN_DELTA, VALUE_SUBJECT>),
    /// Subject-ordered, every row.
    FullSubject(FilteredValueCursor<'a, SCAN_FULL, VALUE_SUBJECT>),
    /// Subject-ordered, non-delta rows only.
    OldOnlySubject(FilteredValueCursor<'a, SCAN_OLD_ONLY, VALUE_SUBJECT>),
    /// Object-ordered, delta rows only.
    DeltaObject(FilteredValueCursor<'a, SCAN_DELTA, VALUE_OBJECT>),
    /// Object-ordered, every row.
    FullObject(FilteredValueCursor<'a, SCAN_FULL, VALUE_OBJECT>),
    /// Object-ordered, non-delta rows only.
    OldOnlyObject(FilteredValueCursor<'a, SCAN_OLD_ONLY, VALUE_OBJECT>),
}

impl<'a> LeapfrogValueCursor<'a> {
    /// A subject-ordered cursor under `scan`.
    fn subject(rows: ValueCursor<'a, VALUE_SUBJECT>, scan: Scan, delta: Delta) -> Self {
        match scan {
            Scan::Delta => Self::DeltaSubject(FilteredValueCursor::new(rows, delta)),
            Scan::Full => Self::FullSubject(FilteredValueCursor::new(rows, delta)),
            Scan::OldOnly => Self::OldOnlySubject(FilteredValueCursor::new(rows, delta)),
        }
    }

    /// An object-ordered cursor under `scan`.
    fn object(rows: ValueCursor<'a, VALUE_OBJECT>, scan: Scan, delta: Delta) -> Self {
        match scan {
            Scan::Delta => Self::DeltaObject(FilteredValueCursor::new(rows, delta)),
            Scan::Full => Self::FullObject(FilteredValueCursor::new(rows, delta)),
            Scan::OldOnly => Self::OldOnlyObject(FilteredValueCursor::new(rows, delta)),
        }
    }

    /// The cursor's current value.
    fn current(&self) -> Option<TermId> {
        match self {
            Self::DeltaSubject(cursor) => cursor.current,
            Self::FullSubject(cursor) => cursor.current,
            Self::OldOnlySubject(cursor) => cursor.current,
            Self::DeltaObject(cursor) => cursor.current,
            Self::FullObject(cursor) => cursor.current,
            Self::OldOnlyObject(cursor) => cursor.current,
        }
    }

    /// Advance to the first value `>= target`.
    fn seek(&mut self, target: TermId) -> Option<TermId> {
        match self {
            Self::DeltaSubject(cursor) => cursor.seek(target),
            Self::FullSubject(cursor) => cursor.seek(target),
            Self::OldOnlySubject(cursor) => cursor.seek(target),
            Self::DeltaObject(cursor) => cursor.seek(target),
            Self::FullObject(cursor) => cursor.seek(target),
            Self::OldOnlyObject(cursor) => cursor.seek(target),
        }
    }

    /// Advance past the current value.
    fn advance(&mut self) -> Option<TermId> {
        match self {
            Self::DeltaSubject(cursor) => cursor.advance(),
            Self::FullSubject(cursor) => cursor.advance(),
            Self::OldOnlySubject(cursor) => cursor.advance(),
            Self::DeltaObject(cursor) => cursor.advance(),
            Self::FullObject(cursor) => cursor.advance(),
            Self::OldOnlyObject(cursor) => cursor.advance(),
        }
    }
}

/// A leapfrog intersection across sorted distinct value cursors.
#[derive(Debug)]
struct LeapfrogIntersection<'a> {
    /// One cursor per relation constrained on the descent variable.
    cursors: Vec<LeapfrogValueCursor<'a>>,
}

impl<'a> LeapfrogIntersection<'a> {
    /// An intersection over `cursors`.
    fn new(cursors: Vec<LeapfrogValueCursor<'a>>) -> Self {
        Self { cursors }
    }

    /// The next value present in EVERY cursor.
    ///
    /// The first cursor advances past the returned value before control returns, so
    /// repeated calls enumerate the intersection without duplicates.
    fn next(&mut self) -> Option<TermId> {
        let mut target = self
            .cursors
            .iter()
            .filter_map(LeapfrogValueCursor::current)
            .max()?;
        loop {
            let mut aligned = true;
            for cursor in &mut self.cursors {
                let value = cursor.seek(target)?;
                if value > target {
                    target = value;
                    aligned = false;
                }
            }
            if aligned {
                self.cursors[0].advance();
                return Some(target);
            }
        }
    }

    /// Whether an externally-bound value occurs in every cursor.
    fn contains(&mut self, wanted: TermId) -> bool {
        self.cursors
            .iter_mut()
            .all(|cursor| cursor.seek(wanted) == Some(wanted))
    }
}

/// The single partition a certified cycle atom addresses, or `None` if the store has no
/// such partition (in which case the component's intersection is empty).
///
/// Cycle certification admits only atoms whose predicate AND graph are constants — a trie
/// level is one sorted arrangement, and an atom that quantifies over its predicate denotes
/// a union of them — so the surfaces are always there to resolve.
fn cycle_atom_partition<'a>(shape: &AtomShape, rel: &'a RelationStore) -> Option<PartitionRef<'a>> {
    let (Some(predicate), Some(graph)) = (shape.predicate().constant(), shape.graph().constant())
    else {
        unreachable!("cycle certification admits only constant-predicate, constant-graph atoms")
    };
    let (predicate, graph) = (rel.term_id(predicate)?, rel.term_id(graph)?);
    rel.partition(predicate, graph)
}

/// The subject and object frame slots of a certified cycle atom.
///
/// Cycle certification admits only atoms with two DISTINCT variable positions there, so
/// both slots exist.
fn cycle_atom_slots(shape: &AtomShape) -> (usize, usize) {
    let (Some(subject), Some(object)) = (shape.subject().slot(), shape.object().slot()) else {
        unreachable!("cycle certification admits only distinct variable-variable atoms")
    };
    (subject, object)
}

/// Build one cycle atom's trie cursor for `variable_slot`, constrained by any binding of
/// its other variable.
fn cycle_atom_cursor<'a>(
    partition: PartitionRef<'a>,
    shape: &AtomShape,
    variable_slot: usize,
    solution: &SlotSolution,
    scan: Scan,
    delta: Delta,
) -> Option<LeapfrogValueCursor<'a>> {
    let (subject_slot, object_slot) = cycle_atom_slots(shape);
    if subject_slot == variable_slot {
        Some(LeapfrogValueCursor::subject(
            partition.values_subject(solution.get(object_slot)),
            scan,
            delta,
        ))
    } else if object_slot == variable_slot {
        Some(LeapfrogValueCursor::object(
            partition.values_object(solution.get(subject_slot)),
            scan,
            delta,
        ))
    } else {
        None
    }
}

/// Immutable state shared by every recursive variable level of one leapfrog component.
#[derive(Debug, Clone, Copy)]
struct LeapfrogRun<'a> {
    /// The rule's plan, for the per-atom operators.
    plan: &'a RulePlan,
    /// The certified component being descended.
    cycle: &'a CyclicPlan,
    /// The round's semi-naive delta position.
    delta_position: usize,
    /// The accumulated store.
    rel: &'a RelationStore,
    /// The round's delta span.
    delta: Delta,
}

impl LeapfrogRun<'_> {
    /// Capture the unique fully-ground row for every cycle atom, in the component's
    /// authored atom order.
    ///
    /// Returns `false` — restoring `solution` — if a scan-mode constraint excludes any of
    /// them, which is how the semi-naive decomposition is enforced on a multiway group.
    fn append_sources(&self, solution: &mut SlotSolution) -> bool {
        let original = solution.sources.len();
        for &planned in self.cycle.atoms() {
            let operator = self.plan.operator_at(planned.positive_position());
            let shape = operator.shape();
            let (subject_slot, object_slot) = cycle_atom_slots(shape);
            let (Some(subject), Some(object), Some(partition)) = (
                solution.get(subject_slot),
                solution.get(object_slot),
                cycle_atom_partition(shape, self.rel),
            ) else {
                solution.sources.truncate(original);
                return false;
            };
            let scan = scan_for(planned.positive_position(), self.delta_position);
            let mut rows = partition.select(Bound::Both(subject, object));
            let mut matched = None;
            while let Some((subject, object, row)) = rows.next() {
                if keep_row_for_scan(scan, self.delta, row) {
                    matched = Some(SourceRow {
                        body_index: planned.body_index(),
                        subject,
                        predicate: partition.predicate(),
                        object,
                        graph: partition.graph(),
                        row,
                    });
                    break;
                }
            }
            let Some(source) = matched else {
                solution.sources.truncate(original);
                return false;
            };
            solution.sources.push(source);
        }
        true
    }

    /// Recursive variable descent for one certified cycle component.
    fn recurse(
        &self,
        variable_position: usize,
        solution: &mut SlotSolution,
        out: &mut Vec<SlotSolution>,
        governor: &mut StepGovernor,
    ) {
        if governor.spent() {
            return;
        }
        if variable_position == self.cycle.variable_slots().len() {
            if self.append_sources(solution) {
                governor.charge();
                out.push(solution.clone());
                solution
                    .sources
                    .truncate(solution.sources.len() - self.cycle.atoms().len());
            }
            return;
        }

        let variable_slot = self.cycle.variable_slots()[variable_position];
        let externally_bound = solution.get(variable_slot);
        let mut cursors = Vec::new();
        for &planned in self.cycle.atoms() {
            let operator = self.plan.operator_at(planned.positive_position());
            let shape = operator.shape();
            let (subject_slot, object_slot) = cycle_atom_slots(shape);
            if subject_slot != variable_slot && object_slot != variable_slot {
                continue;
            }
            let scan = scan_for(planned.positive_position(), self.delta_position);
            let Some(partition) = cycle_atom_partition(shape, self.rel) else {
                return;
            };
            let Some(cursor) =
                cycle_atom_cursor(partition, shape, variable_slot, solution, scan, self.delta)
            else {
                return;
            };
            cursors.push(cursor);
        }
        if cursors.is_empty() {
            return;
        }
        let mut intersection = LeapfrogIntersection::new(cursors);

        if let Some(value) = externally_bound {
            if intersection.contains(value) {
                self.recurse(variable_position + 1, solution, out, governor);
            }
            return;
        }

        while let Some(value) = intersection.next() {
            debug_assert!(solution.bindings[variable_slot].is_none());
            solution.bindings[variable_slot] = Some(value);
            self.recurse(variable_position + 1, solution, out, governor);
            solution.bindings[variable_slot] = None;
            if governor.spent() {
                break;
            }
        }
    }
}

/// Extend every partial solution through one certified cyclic component, materialising no
/// binary intermediate relation.
fn extend_solutions_leapfrog(
    run: LeapfrogRun<'_>,
    solutions: &[SlotSolution],
    governor: &mut StepGovernor,
) -> Vec<SlotSolution> {
    let mut out = Vec::new();
    for solution in solutions {
        if governor.spent() {
            break;
        }
        let mut working = solution.clone();
        run.recurse(0, &mut working, &mut out, governor);
    }
    out
}

// ── Joining a rule body ─────────────────────────────────────────────────────────

/// Which physical join the evaluator runs for a rule.
///
/// Both strategies MUST produce the identical relation — that is the contract two
/// implementations of one join owe each other, and `leapfrog_and_binary_joins_agree`
/// asserts it over the whole synthetic corpus. The forced-binary strategy is therefore a
/// verification instrument, not a caller-facing option: the planner's choice is always at
/// least as good, so nothing outside the test suite has a reason to override it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JoinStrategy {
    /// Follow the plan: leapfrog for a certified cyclic subplan, binary otherwise.
    Planned,
    /// Force the indexed binary fallback even for a certified cyclic rule.
    #[cfg(test)]
    ForcedBinary,
}

/// Join a rule's positive body against the round snapshot.
///
/// The positive join is the semi-naive delta decomposition. Guards, negated atoms and
/// negated conjunctions are applied AFTER it by [`evaluate_rule`], against the
/// accumulated store — which, under the stratified fixpoint, stratification guarantees
/// holds every negated relation's final extension.
fn join_body(
    plan: &RulePlan,
    snapshot: JoinSnapshot<'_>,
    strategy: JoinStrategy,
    governor: &mut StepGovernor,
) -> Vec<SlotSolution> {
    let leapfrog = plan.has_cyclic_subplan() && matches!(strategy, JoinStrategy::Planned);
    if plan.positive().is_empty() {
        // The empty conjunction is relational identity: one empty substitution, so an
        // unconditional or NAF-only rule fires exactly once. Its head is suppressed on the
        // following round by the store's own membership test.
        vec![SlotSolution::empty(plan.variables().len())]
    } else if leapfrog {
        join_positive_leapfrog(plan, snapshot, governor)
    } else {
        join_positive_binary(plan, snapshot, governor)
    }
}

/// What a positive join reads: the accumulated store and the delta it is decomposed over.
#[derive(Debug, Clone, Copy)]
struct JoinSnapshot<'a> {
    /// The accumulated store.
    rel: &'a RelationStore,
    /// The rows this evaluation treats as new.
    delta: Delta,
}

/// The indexed binary positive join: every planned operator in execution order, for every
/// semi-naive delta position.
fn join_positive_binary(
    plan: &RulePlan,
    snapshot: JoinSnapshot<'_>,
    governor: &mut StepGovernor,
) -> Vec<SlotSolution> {
    let operators = plan.operators();
    let mut all: Vec<SlotSolution> = Vec::new();
    for delta_position in 0..operators.len() {
        let mut partial = vec![SlotSolution::empty(plan.variables().len())];
        for (position, operator) in operators.iter().enumerate() {
            partial = extend_slot_solutions(
                operator,
                snapshot.rel,
                snapshot.delta,
                scan_for(position, delta_position),
                &partial,
                governor,
            );
            if partial.is_empty() {
                break;
            }
        }
        all.extend(partial);
        if governor.spent() {
            break;
        }
    }
    for solution in &mut all {
        for &(left, right) in plan.operator_source_order_swaps() {
            solution.sources.swap(left, right);
        }
    }
    all
}

/// The hybrid positive join for a rule with at least one certified cyclic subplan: each
/// physical group in execution order, for every semi-naive delta position.
fn join_positive_leapfrog(
    plan: &RulePlan,
    snapshot: JoinSnapshot<'_>,
    governor: &mut StepGovernor,
) -> Vec<SlotSolution> {
    let mut all: Vec<SlotSolution> = Vec::new();
    for delta_position in 0..plan.positive().len() {
        let mut partial = vec![SlotSolution::empty(plan.variables().len())];
        for group in plan.join_groups() {
            partial = match group {
                JoinGroup::Binary(planned) => {
                    let operator = plan.operator_at(planned.positive_position());
                    extend_slot_solutions(
                        operator,
                        snapshot.rel,
                        snapshot.delta,
                        scan_for(planned.positive_position(), delta_position),
                        &partial,
                        governor,
                    )
                }
                JoinGroup::Leapfrog(cycle) => extend_solutions_leapfrog(
                    LeapfrogRun {
                        plan,
                        cycle,
                        delta_position,
                        rel: snapshot.rel,
                        delta: snapshot.delta,
                    },
                    &partial,
                    governor,
                ),
            };
            if partial.is_empty() {
                break;
            }
        }
        for solution in &mut partial {
            for &(left, right) in plan.hybrid_source_order_swaps() {
                solution.sources.swap(left, right);
            }
        }
        all.extend(partial);
        if governor.spent() {
            break;
        }
    }
    all
}

// ── Per-rule runtime shapes ─────────────────────────────────────────────────────

/// A head or negated-atom argument, lowered once per evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ArgShape {
    /// A variable, read from this frame slot.
    Slot(usize),
    /// A constant, rendered once to its lexical surface.
    Const(String),
}

/// What an [`ArgShape`] resolves to under one solution, for a store probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeValue {
    /// Pinned to this interned term.
    Known(TermId),
    /// Unconstrained: an unbound slot.
    Free,
    /// Pinned to a term the store has never interned — a constant it never saw or a
    /// guard-computed surface — so no stored fact can match.
    Absent,
}

impl ArgShape {
    /// Lower `term` against the plan's slot table.
    ///
    /// # Panics
    ///
    /// Panics on a variable the plan has no slot for. [`RulePlan`] assigns a slot to every
    /// variable of the body AND the head, in every one of their four positions, so that is
    /// a planner contradiction.
    fn of(term: &ClauseTerm, variables: &[String]) -> Self {
        match term.variable() {
            Some(name) => Self::Slot(slot_of(name, variables)),
            None => Self::Const(
                term.surface()
                    .expect("a non-variable term always has a lexical surface"),
            ),
        }
    }

    /// The four lowered arguments of `atom`, in `(subject, predicate, object, graph)`
    /// order.
    fn of_atom(atom: &ClauseAtom, variables: &[String]) -> [Self; ATOM_ARITY] {
        let terms = atom.terms();
        std::array::from_fn(|position| Self::of(terms[position], variables))
    }

    /// This argument under `solution`, for a store probe.
    fn probe(&self, solution: &SlotSolution, rel: &RelationStore) -> ProbeValue {
        match self {
            Self::Slot(slot) => match solution.value(*slot) {
                SlotValue::Interned(id) => ProbeValue::Known(id),
                SlotValue::Computed(_) => ProbeValue::Absent,
                SlotValue::Unbound => ProbeValue::Free,
            },
            Self::Const(surface) => rel
                .term_id(surface)
                .map_or(ProbeValue::Absent, ProbeValue::Known),
        }
    }
}

/// The frame slot of `name`.
///
/// # Panics
///
/// Panics if the plan has no slot for `name` — see [`ArgShape::of`].
fn slot_of(name: &str, variables: &[String]) -> usize {
    variables
        .iter()
        .position(|slot| slot == name)
        .expect("every rule variable has a plan frame slot")
}

/// A negated body atom, lowered once per evaluation.
#[derive(Debug, Clone)]
struct NegatedAtom {
    /// The four lowered arguments, in `(subject, predicate, object, graph)` order.
    args: [ArgShape; ATOM_ARITY],
}

impl NegatedAtom {
    /// Whether this negated atom is SATISFIED — i.e. some matching fact is present, so it
    /// blocks the rule.
    ///
    /// Two binding modes, both of them the stratified-negation truth value because the
    /// negated predicate's stratum has already completed:
    ///
    /// * **fully ground** — a unique-key membership probe in one partition;
    /// * **partially bound (existential NAF)** — "does SOME fact match the ground
    ///   positions?"; an unbound position is unconstrained, so `not p(?x, ?y)` with `?y`
    ///   free reads as "`?x` has no `p` at all". An unbound PREDICATE or GRAPH position is
    ///   unconstrained in exactly the same way, so `not T(?x, ?p, ?y, ?g)` with all three
    ///   free reads as "`?x` is the subject of nothing, anywhere" — the same rule applied
    ///   to the same kind of position. Repeated unbound variables are NOT required to
    ///   agree, matching the reference semantics exactly.
    ///
    /// A ground term the store never interned — a constant it never saw, or a surface a
    /// guard computed — constrains to zero rows, so the atom is not satisfied and the
    /// rule fires.
    fn satisfied(&self, solution: &SlotSolution, rel: &RelationStore) -> bool {
        let mut values = [None; ATOM_ARITY];
        for (position, arg) in self.args.iter().enumerate() {
            match arg.probe(solution, rel) {
                ProbeValue::Known(id) => values[position] = Some(id),
                ProbeValue::Free => {}
                // A ground position whose term is absent from the store matches nothing.
                ProbeValue::Absent => return false,
            }
        }
        let bound = match (values[POSITION_SUBJECT], values[POSITION_OBJECT]) {
            (Some(subject), Some(object)) => Bound::Both(subject, object),
            (Some(subject), None) => Bound::Subject(subject),
            (None, Some(object)) => Bound::Object(object),
            (None, None) => Bound::Any,
        };
        rel.partitions(values[POSITION_PREDICATE], values[POSITION_GRAPH])
            .any(|partition| partition.select(bound).any_remaining())
    }
}

/// One argument of a negated conjunction's atom or guard.
#[derive(Debug, Clone, PartialEq, Eq)]
enum GroupArg {
    /// A variable the enclosing rule binds: read from this frame slot.
    Outer(usize),
    /// A variable existentially quantified inside the group: this local slot.
    Local(usize),
    /// A constant surface.
    Const(String),
}

/// A value bound to one of a negated conjunction's local variables.
#[derive(Debug, Clone, PartialEq, Eq)]
enum LocalValue {
    /// An interned term.
    Interned(TermId),
    /// A guard-computed surface the store has never interned.
    Computed(Box<str>),
}

/// A guard inside a negated conjunction, lowered once per evaluation.
#[derive(Debug, Clone)]
struct GroupGuard {
    /// Where each input is read from.
    inputs: Vec<GroupArg>,
    /// The local slot each output binds.
    outputs: Vec<usize>,
}

/// A negated conjunction ([`Negation`]), lowered once per evaluation.
#[derive(Debug, Clone)]
struct NegationRuntime {
    /// The group's index in its rule.
    index: usize,
    /// The group's atoms, each with its four lowered arguments.
    atoms: Vec<[GroupArg; ATOM_ARITY]>,
    /// The group's guards, in authored order.
    guards: Vec<GroupGuard>,
    /// How many local variables the group quantifies.
    locals: usize,
}

/// What one probe of a negated conjunction needs besides the group itself.
#[derive(Clone, Copy)]
struct GroupProbe<'a> {
    /// The enclosing rule's solution.
    solution: &'a SlotSolution,
    /// The accumulated store.
    rel: &'a RelationStore,
    /// The caller's guard evaluator.
    guards: &'a dyn GuardEvaluator,
    /// The rule's index in authored program order.
    rule: usize,
    /// The rule itself, for its guards' declarations.
    clause: &'a DlClause,
}

impl NegationRuntime {
    /// Lower `negation` against the enclosing rule's frame.
    fn new(index: usize, negation: &Negation, rule: &DlClause, variables: &[String]) -> Self {
        let bound = rule.bound_variables();
        let mut locals: Vec<String> = Vec::new();
        let arg = |term: &ClauseTerm, locals: &mut Vec<String>| -> GroupArg {
            match term.variable() {
                Some(name) if bound.contains(name) => GroupArg::Outer(slot_of(name, variables)),
                Some(name) => GroupArg::Local(match locals.iter().position(|l| l == name) {
                    Some(local) => local,
                    None => {
                        locals.push(name.to_owned());
                        locals.len() - 1
                    }
                }),
                None => GroupArg::Const(
                    term.surface()
                        .expect("a non-variable term always has a lexical surface"),
                ),
            }
        };
        let atoms: Vec<[GroupArg; ATOM_ARITY]> = negation
            .atoms()
            .iter()
            .map(|atom| {
                let terms = atom.terms();
                std::array::from_fn(|position| arg(terms[position], &mut locals))
            })
            .collect();
        let guards = negation
            .guards()
            .iter()
            .map(|guard| GroupGuard {
                inputs: guard
                    .inputs()
                    .iter()
                    .map(|input| arg(&ClauseTerm::var(input.clone()), &mut locals))
                    .collect(),
                outputs: guard
                    .outputs()
                    .iter()
                    .map(
                        |output| match arg(&ClauseTerm::var(output.clone()), &mut locals) {
                            GroupArg::Local(local) => local,
                            GroupArg::Outer(_) | GroupArg::Const(_) => unreachable!(
                                "a group guard output is fresh in the group (DlClause scoping)"
                            ),
                        },
                    )
                    .collect(),
            })
            .collect();
        Self {
            index,
            atoms,
            guards,
            locals: locals.len(),
        }
    }

    /// Whether the group HOLDS under `probe.solution`: some extension of its local
    /// variables matches every atom and passes every guard.
    fn holds(&self, probe: GroupProbe<'_>) -> Result<bool, EvalError> {
        let mut locals: Vec<Option<LocalValue>> = vec![None; self.locals];
        self.match_atom(0, probe, &mut locals)
    }

    /// The store probe value of `arg` under the current bindings.
    fn probe_value(
        arg: &GroupArg,
        probe: GroupProbe<'_>,
        locals: &[Option<LocalValue>],
    ) -> ProbeValue {
        match arg {
            GroupArg::Outer(slot) => match probe.solution.value(*slot) {
                SlotValue::Interned(id) => ProbeValue::Known(id),
                SlotValue::Computed(_) => ProbeValue::Absent,
                SlotValue::Unbound => {
                    unreachable!("an outer group variable is bound by the enclosing rule")
                }
            },
            GroupArg::Local(local) => match &locals[*local] {
                Some(LocalValue::Interned(id)) => ProbeValue::Known(*id),
                Some(LocalValue::Computed(_)) => ProbeValue::Absent,
                None => ProbeValue::Free,
            },
            GroupArg::Const(surface) => probe
                .rel
                .term_id(surface)
                .map_or(ProbeValue::Absent, ProbeValue::Known),
        }
    }

    /// Match atom `k` onward.
    fn match_atom(
        &self,
        k: usize,
        probe: GroupProbe<'_>,
        locals: &mut Vec<Option<LocalValue>>,
    ) -> Result<bool, EvalError> {
        let Some(atom) = self.atoms.get(k) else {
            return self.match_guard(0, probe, locals);
        };
        let mut values = [ProbeValue::Free; ATOM_ARITY];
        for (position, arg) in atom.iter().enumerate() {
            values[position] = Self::probe_value(arg, probe, locals);
            if values[position] == ProbeValue::Absent {
                return Ok(false);
            }
        }
        let known = |value: ProbeValue| match value {
            ProbeValue::Known(id) => Some(id),
            ProbeValue::Free | ProbeValue::Absent => None,
        };
        let bound = match (
            known(values[POSITION_SUBJECT]),
            known(values[POSITION_OBJECT]),
        ) {
            (Some(subject), Some(object)) => Bound::Both(subject, object),
            (Some(subject), None) => Bound::Subject(subject),
            (None, Some(object)) => Bound::Object(object),
            (None, None) => Bound::Any,
        };
        let partitions: Vec<PartitionRef<'_>> = probe
            .rel
            .partitions(
                known(values[POSITION_PREDICATE]),
                known(values[POSITION_GRAPH]),
            )
            .collect();
        for partition in partitions {
            let (predicate, graph) = (partition.predicate(), partition.graph());
            let mut cursor = partition.select(bound);
            while let Some((subject, object, _row)) = cursor.next() {
                let matched = [subject, predicate, object, graph];
                // Bind every free local position, requiring a local repeated inside
                // the atom to agree with itself.
                let mut newly: Vec<usize> = Vec::new();
                let mut consistent = true;
                for (position, arg) in atom.iter().enumerate() {
                    if let GroupArg::Local(local) = arg {
                        match &locals[*local] {
                            None => {
                                locals[*local] = Some(LocalValue::Interned(matched[position]));
                                newly.push(*local);
                            }
                            Some(LocalValue::Interned(id)) if *id == matched[position] => {}
                            Some(_) => {
                                consistent = false;
                                break;
                            }
                        }
                    }
                }
                let found = consistent && self.match_atom(k + 1, probe, locals)?;
                for local in newly {
                    locals[local] = None;
                }
                if found {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Evaluate guard `j` onward, every atom having matched.
    fn match_guard(
        &self,
        j: usize,
        probe: GroupProbe<'_>,
        locals: &mut Vec<Option<LocalValue>>,
    ) -> Result<bool, EvalError> {
        let Some(guard) = self.guards.get(j) else {
            return Ok(true);
        };
        let declared = &probe.clause.negations()[self.index].guards()[j];
        let site = GuardSite::Negation {
            negation: self.index,
            guard: j,
        };
        let rows = {
            let mut inputs: Vec<&str> = Vec::with_capacity(guard.inputs.len());
            for input in &guard.inputs {
                let surface = match input {
                    GroupArg::Outer(slot) => probe
                        .solution
                        .surface(*slot, probe.rel)
                        .expect("an outer group variable is bound by the enclosing rule"),
                    GroupArg::Local(local) => match &locals[*local] {
                        Some(LocalValue::Interned(id)) => probe.rel.interner().resolve(*id),
                        Some(LocalValue::Computed(surface)) => surface,
                        None => unreachable!(
                            "a group guard input is bound before it (DlClause scoping)"
                        ),
                    },
                    GroupArg::Const(surface) => surface,
                };
                inputs.push(surface);
            }
            call_guard(
                probe.guards,
                &GuardCall {
                    rule: probe.rule,
                    site,
                    guard: declared,
                    inputs: &inputs,
                    model: probe.rel,
                },
            )?
        };
        for row in rows {
            for (&local, surface) in guard.outputs.iter().zip(&row) {
                locals[local] = Some(match probe.rel.term_id(surface) {
                    Some(id) => LocalValue::Interned(id),
                    None => LocalValue::Computed(surface.as_str().into()),
                });
            }
            let found = self.match_guard(j + 1, probe, locals)?;
            for &local in &guard.outputs {
                locals[local] = None;
            }
            if found {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// Call the caller's evaluator for one guard, checking the answer's shape.
///
/// # Errors
///
/// [`EvalError::Guard`] when the evaluator reports an error, or answers a row whose
/// width is not the guard's output count — a malformed answer is not read as some
/// answer.
fn call_guard(
    guards: &dyn GuardEvaluator,
    call: &GuardCall<'_>,
) -> Result<Vec<Vec<String>>, EvalError> {
    let refuse = |message: String| EvalError::Guard {
        rule: call.rule,
        site: call.site,
        guard: call.guard.name().to_owned(),
        message,
    };
    let rows = guards.evaluate(call).map_err(refuse)?;
    let width = call.guard.outputs().len();
    if let Some(row) = rows.iter().find(|row| row.len() != width) {
        return Err(refuse(format!(
            "the evaluator answered a row of {} values for {width} outputs",
            row.len()
        )));
    }
    Ok(rows)
}

/// A body guard, lowered once per evaluation.
#[derive(Debug, Clone)]
struct GuardRuntime {
    /// The frame slot of each input, in declared order.
    inputs: Vec<usize>,
    /// The frame slot of each output, in declared order.
    outputs: Vec<usize>,
}

/// Everything about one rule that is a static function of the rule, hoisted out of every
/// round: the head's lowered arguments, the guards' slots and the negations' probes.
///
/// Constant surfaces are rendered ONCE here rather than per candidate — including the head
/// predicate's, which is now a term like any other and so may equally well be a slot.
#[derive(Debug, Clone)]
pub(crate) struct RuleRuntime {
    /// Each head atom's four lowered arguments, in `(subject, predicate, object, graph)`
    /// order. One atom under the stratified fixpoint; a conjunctive head under the ordered
    /// schedule ([`crate::schedule`]) asserts every conjunct from one solution.
    head: Vec<[ArgShape; ATOM_ARITY]>,
    /// The rule's negated body atoms, in authored order.
    negated: Vec<NegatedAtom>,
    /// The rule's body guards, in authored order.
    guards: Vec<GuardRuntime>,
    /// The rule's negated conjunctions, in authored order.
    negations: Vec<NegationRuntime>,
}

impl RuleRuntime {
    /// Lower one rule's static shapes.
    pub(crate) fn new(rule: &DlClause, plan: &RulePlan) -> Self {
        let variables = plan.variables();
        Self {
            head: rule
                .head_atoms()
                .map(|atom| ArgShape::of_atom(atom, variables))
                .collect(),
            negated: plan
                .negated()
                .iter()
                .map(|&index| NegatedAtom {
                    args: ArgShape::of_atom(&rule.body()[index], variables),
                })
                .collect(),
            guards: rule
                .guards()
                .iter()
                .map(|guard: &Guard| GuardRuntime {
                    inputs: guard
                        .inputs()
                        .iter()
                        .map(|name| slot_of(name, variables))
                        .collect(),
                    outputs: guard
                        .outputs()
                        .iter()
                        .map(|name| slot_of(name, variables))
                        .collect(),
                })
                .collect(),
            negations: rule
                .negations()
                .iter()
                .enumerate()
                .map(|(index, negation)| NegationRuntime::new(index, negation, rule, variables))
                .collect(),
        }
    }
}

// ── Round candidates ────────────────────────────────────────────────────────────

/// One head argument of a candidate derivation.
///
/// A value that is already in the store is compared by its interned id; a term the store
/// has never seen — a head constant, or a surface a guard computed — has no id yet and is
/// compared by its surface. The two cases are disjoint — a value the store knows is always
/// keyed by its id — which is what stops one fact from being keyed two different ways.
///
/// The derived order is deterministic (interned before fresh, then by id / by surface); it
/// is a grouping order only, never an emission order: winners are re-sorted lexically
/// before they are committed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum HeadTerm<'r> {
    /// A term already present in the store's dictionary.
    Interned(TermId),
    /// A term the store has never interned, so no fact can already carry it: borrowed
    /// when it is a head constant, owned when a guard computed it.
    Fresh(Cow<'r, str>),
}

impl HeadTerm<'_> {
    /// This argument's interned id, if the store already holds the term.
    fn interned(&self) -> Option<TermId> {
        match self {
            Self::Interned(id) => Some(*id),
            Self::Fresh(_) => None,
        }
    }

    /// Whether this is a term a guard computed and the store never held.
    fn is_generated(&self) -> bool {
        matches!(self, Self::Fresh(Cow::Owned(_)))
    }

    /// This argument's lexical surface.
    fn surface(&self, rel: &RelationStore) -> String {
        match self {
            Self::Interned(id) => rel.interner().resolve(*id).to_owned(),
            Self::Fresh(surface) => surface.to_string(),
        }
    }
}

/// The identity of a candidate head fact.
///
/// All four positions are [`HeadTerm`]s: with the predicate carried as data, a head
/// predicate can be a bound variable, so it cannot be a borrowed `&str` naming a relation.
/// The field order is the [`Fact`] order, so the derived `Ord` groups candidates the way
/// the commit sweep will emit them.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct HeadKey<'r> {
    /// The head subject.
    subject: HeadTerm<'r>,
    /// The head predicate.
    predicate: HeadTerm<'r>,
    /// The head object.
    object: HeadTerm<'r>,
    /// The head graph.
    graph: HeadTerm<'r>,
}

impl HeadKey<'_> {
    /// The four positions, in fact order.
    fn terms(&self) -> [&HeadTerm<'_>; ATOM_ARITY] {
        [&self.subject, &self.predicate, &self.object, &self.graph]
    }
}

/// One rule firing, before the round's winner is chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Candidate {
    /// The producing rule's index in authored program order.
    rule: usize,
    /// The matched body rows, in authored body order.
    sources: Vec<SourceRow>,
    /// `1 + max(source proof heights)`.
    proof_height: u32,
    /// The sum of the source proof heights — the second tiebreak component.
    sum_source_height: u64,
}

impl Candidate {
    /// The candidate's source facts, in authored body order.
    ///
    /// Each source names the quad it actually matched, predicate included: a body atom
    /// with a variable predicate matched some concrete one, and the provenance has to say
    /// which.
    fn source_facts(&self, rel: &RelationStore) -> Vec<Fact> {
        self.sources.iter().map(|source| source.fact(rel)).collect()
    }

    /// Whether this candidate beats `other` for the same head fact.
    ///
    /// A TOTAL order over observable provenance:
    /// `(proof_height, sum_source_height, sorted source facts, rule index, source facts)`,
    /// smallest wins. Every component is content-derived — proof heights, lexical fact
    /// surfaces and the authored rule position — so the winner is a function of the
    /// program and the data, never of which rule the scheduler happened to run first.
    ///
    /// The comparison resolves lexical surfaces, so it is deliberately staged: the cheap
    /// numeric prefix decides almost every real collision, and the allocating tail runs
    /// only on an exact numeric tie.
    fn preferred_over(&self, other: &Self, rel: &RelationStore) -> bool {
        match (self.proof_height, self.sum_source_height)
            .cmp(&(other.proof_height, other.sum_source_height))
        {
            std::cmp::Ordering::Less => return true,
            std::cmp::Ordering::Greater => return false,
            std::cmp::Ordering::Equal => {}
        }
        let mine = self.source_facts(rel);
        let theirs = other.source_facts(rel);
        let mut mine_sorted = mine.clone();
        mine_sorted.sort();
        let mut theirs_sorted = theirs.clone();
        theirs_sorted.sort();
        (mine_sorted, self.rule, mine) < (theirs_sorted, other.rule, theirs)
    }
}

/// One round's candidate winners, keyed by head fact.
///
/// A `BTreeMap` rather than a hash table: the merge sweep and the commit sweep both
/// iterate it, so its order reaches an output path and must be total and content-derived.
#[derive(Debug)]
pub(crate) struct RoundBuffer<'r> {
    /// The best candidate seen so far per head fact.
    entries: BTreeMap<HeadKey<'r>, Candidate>,
    /// Candidate solutions enumerated by the task that produced this buffer.
    join_steps: u64,
    /// Whether some entry carries a term a guard computed and the store never held — a
    /// TERM-GENERATING round ([`MAX_TERM_GENERATING_ROUNDS`]).
    generates_terms: bool,
    /// The rules of the candidates that carry such a term, for a divergence refusal to
    /// name ([`EvalError::TermGenerationDiverged`]).
    generating_rules: BTreeSet<usize>,
    /// Rows already in the store that the snapshot marks as ASSUMED and a rule of this
    /// round derived again, each with the derivation that did — see
    /// [`RoundSnapshot::assumed`].
    confirmed: Vec<(RowId, Derivation)>,
}

impl<'r> RoundBuffer<'r> {
    /// An empty buffer.
    fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            join_steps: 0,
            generates_terms: false,
            generating_rules: BTreeSet::new(),
            confirmed: Vec::new(),
        }
    }

    /// Whether the round derived nothing new.
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The assumed rows a rule of the round derived again, with the derivations.
    pub(crate) fn confirmed(&self) -> &[(RowId, Derivation)] {
        &self.confirmed
    }

    /// Insert or quality-merge one candidate.
    fn insert(&mut self, key: HeadKey<'r>, candidate: Candidate, rel: &RelationStore) {
        if key.terms().iter().any(|term| term.is_generated()) {
            self.generates_terms = true;
            self.generating_rules.insert(candidate.rule);
        }
        match self.entries.get_mut(&key) {
            Some(existing) => {
                if candidate.preferred_over(existing, rel) {
                    *existing = candidate;
                }
            }
            None => {
                self.entries.insert(key, candidate);
            }
        }
    }

    /// Fold a completed rule-local buffer in at the scheduling-erasing serial boundary.
    fn merge_from(&mut self, other: Self, rel: &RelationStore) {
        self.join_steps = self.join_steps.saturating_add(other.join_steps);
        self.confirmed.extend(other.confirmed);
        for (key, candidate) in other.entries {
            self.insert(key, candidate, rel);
        }
    }
}

/// The immutable snapshot every rule task reads during one round.
///
/// No task may mutate any of it; the single sorted commit begins only after every task
/// buffer has been collected and merged in program order.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RoundSnapshot<'a> {
    /// The accumulated store.
    pub(crate) rel: &'a RelationStore,
    /// Per-row proof heights, indexed by [`RowId`].
    pub(crate) depth: &'a [u32],
    /// Per-row ASSUMED flags, indexed by [`RowId`], or empty when nothing is assumed.
    ///
    /// An assumed row is a fact the caller asserted for the duration of a schedule layer
    /// ([`crate::schedule::LayerHooks`]) and will retract at its end unless a rule derives
    /// it too; a rule that re-derives one is recorded in [`RoundBuffer::confirmed`].
    pub(crate) assumed: &'a [bool],
}

/// One rule scheduled into a round, with the delta its positive join is decomposed over.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RuleEntry<'r> {
    /// The rule's index in authored program order.
    pub(crate) index: usize,
    /// The rule.
    pub(crate) rule: &'r DlClause,
    /// Its store-independent join plan.
    pub(crate) plan: &'r RulePlan,
    /// Its lowered static shapes.
    pub(crate) runtime: &'r RuleRuntime,
    /// The rows this evaluation treats as new.
    pub(crate) delta: Delta,
}

/// How one round schedules its immutable per-rule candidate work.
///
/// Both policies feed the SAME program-order merge and the same sorted commit, so
/// scheduling cannot affect an answer or a budget observation — which is exactly what
/// `sequential_and_parallel_rounds_agree` asserts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RoundExecution {
    /// Evaluate rules into independent buffers, then merge them in program order.
    ///
    /// Production's only policy. It already degrades to the direct in-order path for a
    /// single-rule round, for a round holding a guarded rule (a guard runs on the calling
    /// thread — see [`crate::guard`]) and for a one-worker pool — which is every `wasm32`
    /// build.
    Parallel,
    /// Force the direct in-order path even where rayon would be used.
    ///
    /// Test-only, for the same reason [`JoinStrategy::ForcedBinary`] is: scheduling may
    /// not change an answer, so this exists to assert that it does not, never to let a
    /// caller weaken production execution.
    #[cfg(test)]
    Sequential,
}

impl RoundExecution {
    /// Whether this round has enough independent work and workers to use rayon.
    ///
    /// A single-rule round and a one-worker pool (which is every `wasm32` build) stay on
    /// the allocation-minimal direct path: there is no parallelism to recover in either.
    fn should_parallelize(self, rule_count: usize) -> bool {
        matches!(self, Self::Parallel) && rule_count > 1 && rayon::current_num_threads() > 1
    }
}

/// Evaluate one rule against the frozen round snapshot into a private buffer.
///
/// The positive join runs first; then each body guard in authored order, extending or
/// dropping each solution; then the negated atoms and negated conjunctions, each of which
/// drops a solution it is satisfied under. Every surviving solution yields one candidate
/// per head atom.
fn evaluate_rule<'r>(
    entry: RuleEntry<'r>,
    snapshot: RoundSnapshot<'_>,
    strategy: JoinStrategy,
    allowance: u64,
    guards: &dyn GuardEvaluator,
) -> Result<RoundBuffer<'r>, EvalError> {
    let (plan, runtime, rel) = (entry.plan, entry.runtime, snapshot.rel);
    let mut governor = StepGovernor::new(allowance);
    let mut solutions = join_body(
        plan,
        JoinSnapshot {
            rel,
            delta: entry.delta,
        },
        strategy,
        &mut governor,
    );

    for (position, guard) in runtime.guards.iter().enumerate() {
        let declared = &entry.rule.guards()[position];
        let mut extended = Vec::with_capacity(solutions.len());
        for solution in solutions {
            if governor.spent() {
                break;
            }
            let rows = {
                let inputs: Vec<&str> = guard
                    .inputs
                    .iter()
                    .map(|&slot| {
                        solution
                            .surface(slot, rel)
                            .expect("a guard input is bound before the guard (DlClause scoping)")
                    })
                    .collect();
                call_guard(
                    guards,
                    &GuardCall {
                        rule: entry.index,
                        site: GuardSite::Body(position),
                        guard: declared,
                        inputs: &inputs,
                        model: rel,
                    },
                )?
            };
            for row in rows {
                let mut next = solution.clone();
                for (&slot, surface) in guard.outputs.iter().zip(&row) {
                    next.bind_surface(slot, surface, rel);
                }
                governor.charge();
                extended.push(next);
            }
        }
        solutions = extended;
    }

    if !runtime.negated.is_empty() {
        solutions.retain(|solution| {
            !runtime
                .negated
                .iter()
                .any(|atom| atom.satisfied(solution, rel))
        });
    }
    if !runtime.negations.is_empty() {
        let mut kept = Vec::with_capacity(solutions.len());
        for solution in solutions {
            let mut blocked = false;
            for negation in &runtime.negations {
                if negation.holds(GroupProbe {
                    solution: &solution,
                    rel,
                    guards,
                    rule: entry.index,
                    clause: entry.rule,
                })? {
                    blocked = true;
                    break;
                }
            }
            if !blocked {
                kept.push(solution);
            }
        }
        solutions = kept;
    }

    let mut buffer = RoundBuffer::new();
    buffer.join_steps = governor.consumed;
    for solution in solutions {
        let mut proof_height = 0u32;
        let mut sum_source_height = 0u64;
        for source in &solution.sources {
            let height = snapshot.depth[source.row.index()];
            proof_height = proof_height.max(height);
            sum_source_height = sum_source_height.saturating_add(u64::from(height));
        }
        for head in &runtime.head {
            let key = HeadKey {
                subject: head_term(&head[POSITION_SUBJECT], &solution),
                predicate: head_term(&head[POSITION_PREDICATE], &solution),
                object: head_term(&head[POSITION_OBJECT], &solution),
                graph: head_term(&head[POSITION_GRAPH], &solution),
            };
            let key = intern_key(key, rel);
            // A fact a prior round or stratum already derived is not a derivation: earlier
            // wins, exactly as the reference fixpoint decides it. Every one of the four
            // positions must already be interned for the quad to be present.
            if let Some(row) = present_row(&key, rel) {
                if snapshot.assumed.get(row.index()).copied().unwrap_or(false) {
                    let [subject, predicate, object, graph] =
                        key.terms().map(|term| term.surface(rel));
                    buffer.confirmed.push((
                        row,
                        Derivation {
                            fact: Fact {
                                subject,
                                predicate,
                                object,
                                graph,
                            },
                            rule: entry.index,
                            sources: solution.sources.iter().map(|s| s.fact(rel)).collect(),
                            proof_height: proof_height.saturating_add(1),
                        },
                    ));
                }
                continue;
            }
            buffer.insert(
                key,
                Candidate {
                    rule: entry.index,
                    sources: solution.sources.clone(),
                    proof_height: proof_height.saturating_add(1),
                    sum_source_height,
                },
                rel,
            );
        }
    }
    Ok(buffer)
}

/// The store row of the fact `key` names, if it is present.
fn present_row(key: &HeadKey<'_>, rel: &RelationStore) -> Option<RowId> {
    let (subject, predicate, object, graph) = (
        key.subject.interned()?,
        key.predicate.interned()?,
        key.object.interned()?,
        key.graph.interned()?,
    );
    let partition = rel.partition(predicate, graph)?;
    let mut cursor = partition.select(Bound::Both(subject, object));
    cursor.next().map(|(_, _, row)| row)
}

/// Key a head term the store DOES know by its id, whatever produced it.
///
/// A guard may compute a surface the store already holds, and a head constant may have
/// been interned since the plan was lowered; keying either as fresh would let one fact be
/// keyed two ways and committed twice.
fn intern_key<'r>(key: HeadKey<'r>, rel: &RelationStore) -> HeadKey<'r> {
    let resolve = |term: HeadTerm<'r>| match term {
        HeadTerm::Fresh(surface) => rel
            .term_id(&surface)
            .map_or(HeadTerm::Fresh(surface), HeadTerm::Interned),
        interned @ HeadTerm::Interned(_) => interned,
    };
    HeadKey {
        subject: resolve(key.subject),
        predicate: resolve(key.predicate),
        object: resolve(key.object),
        graph: resolve(key.graph),
    }
}

/// Lower one head argument to its round-key term.
///
/// # Panics
///
/// Panics if a head variable is unbound. Compilation refuses a rule whose head carries a
/// variable nothing in its body binds, and every positive atom binds all four of its
/// variable positions before the join completes (every guard output is bound by its guard),
/// so an unbound head slot here would be a contradiction in the range-restriction check.
fn head_term<'r>(shape: &'r ArgShape, solution: &SlotSolution) -> HeadTerm<'r> {
    match shape {
        ArgShape::Slot(slot) => match solution.value(*slot) {
            SlotValue::Interned(id) => HeadTerm::Interned(id),
            SlotValue::Computed(surface) => HeadTerm::Fresh(Cow::Owned(surface.to_owned())),
            SlotValue::Unbound => {
                unreachable!("a range-restricted head variable is bound by the body")
            }
        },
        ArgShape::Const(surface) => HeadTerm::Fresh(Cow::Borrowed(surface.as_str())),
    }
}

/// Evaluate every scheduled rule of one round, erasing scheduling order.
///
/// A round holding a guarded rule runs on the calling thread in program order: a guard is
/// caller code, and the caller's thread-scoped evaluation context has to reach it. A
/// guard-free round keeps the rule-parallel path, which cannot reach a guard at all.
pub(crate) fn evaluate_round<'r>(
    entries: &[RuleEntry<'r>],
    snapshot: RoundSnapshot<'_>,
    execution: RoundExecution,
    strategy: JoinStrategy,
    allowance: u64,
    guards: &dyn GuardEvaluator,
) -> Result<RoundBuffer<'r>, EvalError> {
    let guarded = entries.iter().any(|entry| entry.rule.is_guarded());
    let mut round = RoundBuffer::new();
    if guarded || !execution.should_parallelize(entries.len()) {
        for &entry in entries {
            let buffer = evaluate_rule(entry, snapshot, strategy, allowance, guards)?;
            round.merge_from(buffer, snapshot.rel);
        }
        return Ok(round);
    }

    // `par_iter` over a slice is INDEXED, so `collect::<Vec<_>>()` restores program order
    // regardless of completion order; the merge below then folds strictly in that order.
    // This indexed form is the ONLY parallelism this crate uses — see the module docs for
    // the unordered rayon adaptors it must never reach for. No rule here is guarded, so the
    // evaluator every task receives is the one that refuses every call.
    let buffers: Vec<Result<RoundBuffer<'r>, EvalError>> = entries
        .par_iter()
        .map(|&entry| evaluate_rule(entry, snapshot, strategy, allowance, &NoGuards))
        .collect();

    for buffer in buffers {
        round.merge_from(buffer?, snapshot.rel);
    }
    Ok(round)
}

// ── The fixpoint ────────────────────────────────────────────────────────────────

/// The mutable working set carried across every stratum of one evaluation.
#[derive(Debug)]
pub(crate) struct FixpointState {
    /// The accumulated store: the seeded EDB plus everything derived so far.
    pub(crate) rel: RelationStore,
    /// Per-row proof heights, indexed by [`RowId`] and pushed in lockstep with the store.
    pub(crate) depth: Vec<u32>,
    /// Every committed derivation.
    pub(crate) derivations: Vec<Derivation>,
    /// Candidate solutions enumerated so far.
    pub(crate) join_steps: u64,
    /// Term-generating rounds committed so far.
    pub(crate) term_generating_rounds: u64,
    /// The term-generating round limit in force: the caller's fixed limit, or the
    /// horizon derived from the seeded store.
    pub(crate) term_generating_round_limit: u64,
    /// Whether that limit is the default horizon, whose refusal is a divergence verdict.
    horizon: bool,
    /// The distinct terms of the seeded store.
    input_terms: usize,
    /// The rules that generated a term in the latest term-generating round.
    generating_rules: BTreeSet<usize>,
}

impl FixpointState {
    /// A state over the seeded store `edb`, governed by `options`.
    pub(crate) fn seeded(edb: RelationStore, options: EvalOptions) -> Self {
        let depth = vec![0u32; edb.row_count()];
        let input_terms = edb.interner().len();
        let limit = options.term_generating_limit();
        Self {
            rel: edb,
            depth,
            derivations: Vec::new(),
            join_steps: 0,
            term_generating_rounds: 0,
            term_generating_round_limit: limit.rounds_for(input_terms),
            horizon: limit == TermGeneratingLimit::Horizon,
            input_terms,
            generating_rules: BTreeSet::new(),
        }
    }

    /// The budget consumption observed so far.
    pub(crate) fn report(&self) -> BudgetReport {
        BudgetReport {
            join_steps: self.join_steps,
            stored_facts: self.rel.row_count(),
            term_arena_bytes: self.rel.term_bytes(),
            term_generating_rounds: self.term_generating_rounds,
            term_generating_round_limit: self.term_generating_round_limit,
        }
    }

    /// The report that would describe the state if it also held `extra` more facts.
    fn projected_report(&self, extra: usize) -> BudgetReport {
        BudgetReport {
            stored_facts: self.rel.row_count().saturating_add(extra),
            ..self.report()
        }
    }

    /// The join-step allowance one round's rule tasks each receive.
    pub(crate) fn allowance(&self) -> u64 {
        MAX_JOIN_STEPS
            .saturating_sub(self.join_steps)
            .saturating_add(1)
    }

    /// Account for a completed round's work, commit its winners and check every ceiling.
    ///
    /// # Errors
    ///
    /// [`EvalError::BudgetExhausted`] when the round passed a ceiling — decided before a
    /// single surface is materialised where the fact count alone proves it.
    pub(crate) fn absorb(&mut self, round: RoundBuffer<'_>) -> Result<(), EvalError> {
        self.join_steps = self.join_steps.saturating_add(round.join_steps);
        check_budget(self)?;
        if round.entries.is_empty() {
            return Ok(());
        }
        // Every entry was gated on absence from the store when it was created, and the
        // entries are unique by head fact, so this projection is exact rather than an
        // over-estimate: the ceiling is decided before a single surface is materialised.
        if self.rel.row_count() + round.entries.len() > MAX_STORED_FACTS {
            return Err(EvalError::BudgetExhausted {
                resource: BudgetResource::StoredFacts,
                report: self.projected_report(round.entries.len()),
            });
        }
        if round.generates_terms {
            self.term_generating_rounds = self.term_generating_rounds.saturating_add(1);
            self.generating_rules.clone_from(&round.generating_rules);
        }
        commit_round(round, self);
        check_budget(self)
    }
}

/// Evaluate `exe` over the seeded store `edb`, running each stratum to its least fixpoint.
///
/// The store is consumed and returned saturated inside [`Evaluation`], so the least model
/// and its provenance stay one value.
///
/// # Errors
///
/// [`EvalError::BudgetExhausted`] if the run passes any of the fixed ceilings. There
/// is no partial answer: a budget refusal is total. [`EvalError::Guard`] if the program
/// carries a guard, which this entry point has no evaluator for — see
/// [`evaluate_guarded`].
pub fn evaluate(exe: &Executable, edb: RelationStore) -> Result<Evaluation, EvalError> {
    evaluate_with(
        exe,
        edb,
        None,
        &NoGuards,
        EvalOptions::default(),
        RoundExecution::Parallel,
        JoinStrategy::Planned,
    )
}

/// [`evaluate`], polling a caller-owned [`StopSignal`] once per semi-naive round.
///
/// Identical to [`evaluate`] in every respect but one: if `stop` reports stopped at a round
/// boundary the fixpoint refuses with [`EvalError::Stopped`] instead of running the next
/// round. It is not a budget — see [`crate::stop`] for why the distinction is the whole
/// reason this entry point may exist beside a crate that refuses caller-supplied ceilings —
/// and it cannot change an answer: an unstopped run does exactly what [`evaluate`] does,
/// and a stopped one produces no [`Evaluation`] at all.
///
/// The poll is at the ROUND boundary, which is the granularity the fixpoint has: one round
/// joins every fireable rule of a stratum against the accumulated store, so a signal that
/// fires mid-round is observed when that round commits. That is the honest resolution of
/// this entry point, and a host that needs a finer one is asking for a charge schedule
/// rather than a stop signal.
///
/// # Errors
///
/// [`EvalError::Stopped`] if `stop` fired, and every error [`evaluate`] returns.
pub fn evaluate_until(
    exe: &Executable,
    edb: RelationStore,
    stop: Option<&dyn StopSignal>,
) -> Result<Evaluation, EvalError> {
    evaluate_with(
        exe,
        edb,
        stop,
        &NoGuards,
        EvalOptions::default(),
        RoundExecution::Parallel,
        JoinStrategy::Planned,
    )
}

/// [`evaluate_until`] for a program carrying guard literals ([`crate::guard`]), whose
/// meaning `guards` supplies.
///
/// Every guard is a pure function of its bindings here — [`compile`] refuses one that
/// reads the model — so the semi-naive decomposition stays exact: a guard is re-evaluated
/// only for the solutions a round's delta produces, exactly as an atom is re-joined only
/// for them. A rule with no positive body atom is evaluated in the first round of its
/// stratum only, because nothing a later round adds can change its answer.
///
/// `options` carries the caller's governors ([`EvalOptions`]).
///
/// # Errors
///
/// [`EvalError::Guard`] when `guards` cannot evaluate a guard, and every error
/// [`evaluate_until`] returns.
pub fn evaluate_guarded(
    exe: &Executable,
    edb: RelationStore,
    guards: &dyn GuardEvaluator,
    options: &EvalOptions,
    stop: Option<&dyn StopSignal>,
) -> Result<Evaluation, EvalError> {
    evaluate_with(
        exe,
        edb,
        stop,
        guards,
        *options,
        RoundExecution::Parallel,
        JoinStrategy::Planned,
    )
}

/// The policy-selectable implementation behind [`evaluate`].
///
/// The policies are private: a caller may neither weaken production scheduling nor
/// override the planner's join choice. The tests use them to prove the alternatives agree.
fn evaluate_with(
    exe: &Executable,
    edb: RelationStore,
    stop: Option<&dyn StopSignal>,
    guards: &dyn GuardEvaluator,
    options: EvalOptions,
    execution: RoundExecution,
    strategy: JoinStrategy,
) -> Result<Evaluation, EvalError> {
    let runtimes: Vec<RuleRuntime> = (0..exe.rule_count())
        .map(|index| {
            let (rule, plan) = exe.rule_entry(index);
            RuleRuntime::new(rule, plan)
        })
        .collect();

    let mut state = FixpointState::seeded(edb, options);
    check_budget(&state)?;

    for stratum in 0..exe.stratum_count() {
        if exe.stratum_is_empty(stratum) {
            continue;
        }
        run_stratum(
            exe,
            &runtimes,
            stratum,
            &mut state,
            StratumRun {
                stop,
                guards,
                execution,
                strategy,
            },
        )?;
    }

    Ok(state.finish())
}

impl FixpointState {
    /// Seal the state into an [`Evaluation`].
    ///
    /// Derivations are produced in per-round lexical order; sorting the whole vector makes
    /// that one total order across strata as well, so the output is a pure function of the
    /// program and the facts rather than of the stratum boundaries.
    pub(crate) fn finish(mut self) -> Evaluation {
        self.derivations.sort();
        Evaluation {
            budget: self.report(),
            facts: self.rel,
            derivations: self.derivations,
        }
    }
}

/// Whether any ceiling is already passed.
pub(crate) fn check_budget(state: &FixpointState) -> Result<(), EvalError> {
    let report = state.report();
    if report.join_steps > MAX_JOIN_STEPS {
        return Err(EvalError::BudgetExhausted {
            resource: BudgetResource::JoinSteps,
            report,
        });
    }
    if report.stored_facts > MAX_STORED_FACTS {
        return Err(EvalError::BudgetExhausted {
            resource: BudgetResource::StoredFacts,
            report,
        });
    }
    if report.term_arena_bytes > MAX_TERM_ARENA_BYTES {
        return Err(EvalError::BudgetExhausted {
            resource: BudgetResource::TermArenaBytes,
            report,
        });
    }
    if report.term_generating_rounds > report.term_generating_round_limit && state.horizon {
        return Err(EvalError::TermGenerationDiverged {
            rules: state.generating_rules.iter().copied().collect(),
            input_terms: state.input_terms,
            report,
        });
    }
    if report.term_generating_rounds > report.term_generating_round_limit {
        return Err(EvalError::BudgetExhausted {
            resource: BudgetResource::TermGeneratingRounds,
            report,
        });
    }
    Ok(())
}

/// The per-evaluation policies one stratum runs under.
#[derive(Clone, Copy)]
struct StratumRun<'a> {
    /// The caller's stop signal.
    stop: Option<&'a dyn StopSignal>,
    /// The caller's guard evaluator.
    guards: &'a dyn GuardEvaluator,
    /// The round scheduling policy.
    execution: RoundExecution,
    /// The join-kernel policy.
    strategy: JoinStrategy,
}

/// Run one stratum's semi-naive fixpoint into `state`.
fn run_stratum(
    exe: &Executable,
    runtimes: &[RuleRuntime],
    stratum: usize,
    state: &mut FixpointState,
    run: StratumRun<'_>,
) -> Result<(), EvalError> {
    // Seed the delta with EVERY accumulated row, so this stratum's rules fire against the
    // whole accumulated store in round one. Row ids are dense, so "everything" is the
    // contiguous span `[0, row_count)` — no per-key materialisation and no bitset.
    let mut delta = Delta::all(state.rel.row_count());
    let mut first_round = true;

    loop {
        // The caller's stop signal, polled BEFORE the round it would prevent. Checking here
        // rather than after the round is what makes the refusal cost bounded by one round
        // rather than by two, and it is the same place `check_budget` is decided from.
        if is_stopped(run.stop) {
            return Err(EvalError::Stopped {
                report: state.report(),
            });
        }
        // A rule with no positive body atom has no delta to be decomposed over: its answer
        // is fixed by the relations below its stratum, so it runs in the first round only.
        let entries: Vec<RuleEntry<'_>> = exe
            .stratum_rule_indices(stratum)
            .iter()
            .filter_map(|&index| {
                let (rule, plan) = exe.rule_entry(index);
                (first_round || !plan.positive().is_empty()).then_some(RuleEntry {
                    index,
                    rule,
                    plan,
                    runtime: &runtimes[index],
                    delta,
                })
            })
            .collect();
        first_round = false;
        if entries.is_empty() {
            return Ok(());
        }
        let round = evaluate_round(
            &entries,
            RoundSnapshot {
                rel: &state.rel,
                depth: &state.depth,
                assumed: &[],
            },
            run.execution,
            run.strategy,
            state.allowance(),
            run.guards,
        )?;
        if round.is_empty() {
            state.join_steps = state.join_steps.saturating_add(round.join_steps);
            return check_budget(state); // stratum fixpoint
        }

        let round_lo = state.rel.row_count();
        state.absorb(round)?;

        // The next round's delta is exactly the rows committed this round — a contiguous
        // span, because the commit loop mints row ids densely in one sorted pass.
        delta = Delta {
            lo: round_lo,
            hi: state.rel.row_count(),
        };
    }
}

/// Commit a round's winners in lexical `(subject, predicate, object, graph)` order.
///
/// Lexical — never mint order — so store insertion order, row-id assignment and the
/// derivation sequence are all byte-deterministic. Row-id assignment is a purely additive
/// side effect of this sorted loop; it never orders the commit.
fn commit_round(round: RoundBuffer<'_>, state: &mut FixpointState) {
    let mut winners: Vec<(Fact, Candidate)> = round
        .entries
        .into_iter()
        .map(|(key, candidate)| {
            let fact = Fact {
                subject: key.subject.surface(&state.rel),
                predicate: key.predicate.surface(&state.rel),
                object: key.object.surface(&state.rel),
                graph: key.graph.surface(&state.rel),
            };
            (fact, candidate)
        })
        .collect();
    // Head facts are unique in a round buffer, so the lexical order is total.
    winners.sort_by(|(left, _), (right, _)| left.cmp(right));

    for (fact, candidate) in winners {
        let sources = candidate.source_facts(&state.rel);
        let inserted = state
            .rel
            .insert(&fact.subject, &fact.predicate, &fact.object, &fact.graph);
        let (_, _, row) = inserted.expect(
            "a round winner is absent from the store by construction, so it inserts a new row",
        );
        debug_assert_eq!(row.index(), state.depth.len(), "depth tracks store rows");
        state.depth.push(candidate.proof_height);
        state.derivations.push(Derivation {
            fact,
            rule: candidate.rule,
            sources,
            proof_height: candidate.proof_height,
        });
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::clause::{ClauseAtom, HeadDisjunct};
    use crate::synth_corpus::{self, SynthWorkload};
    use crate::test_support::permute;

    const P: &str = "https://example.org/p";
    const Q: &str = "https://example.org/q";
    const R: &str = "https://example.org/r";

    fn v(name: &str) -> ClauseTerm {
        ClauseTerm::var(name)
    }

    fn iri(name: &str) -> ClauseTerm {
        ClauseTerm::iri(name)
    }

    fn atom(subject: &str, predicate: &str, object: &str) -> ClauseAtom {
        ClauseAtom::positive(v(subject), predicate, v(object))
    }

    /// The lexical surface an IRI is stored under — for a predicate exactly as much as
    /// for a subject, because a predicate is an ordinary term.
    fn surface(name: &str) -> String {
        format!("<{name}>")
    }

    /// One EDB quad, as the surfaces the store interns, in the default graph.
    fn quad(subject: &str, predicate: &str, object: &str) -> (String, String, String, String) {
        (
            subject.to_owned(),
            surface(predicate),
            object.to_owned(),
            RelationStore::DEFAULT_GRAPH.to_owned(),
        )
    }

    fn store_of(triples: &[(&str, &str, &str)]) -> RelationStore {
        let mut store = RelationStore::new();
        for &(subject, predicate, object) in triples {
            store.insert(
                &surface(subject),
                &surface(predicate),
                &surface(object),
                RelationStore::DEFAULT_GRAPH,
            );
        }
        store
    }

    /// Every `(subject, object)` surface pair of one predicate IRI, across every graph.
    fn relation(evaluation: &Evaluation, predicate: &str) -> BTreeSet<(String, String)> {
        evaluation
            .facts()
            .facts_sorted()
            .into_iter()
            .filter(|fact| fact.predicate == surface(predicate))
            .map(|fact| (fact.subject, fact.object))
            .collect()
    }

    fn run(rules: Vec<DlClause>, edb: RelationStore) -> Evaluation {
        let exe = compile(rules).expect("the fixture program compiles");
        evaluate(&exe, edb).expect("the fixture program stays inside every ceiling")
    }

    // ── The binding frame's inline capacity ─────────────────────────────────────

    /// A frame as wide as the widest rule this workspace compiles stays INLINE.
    ///
    /// The capacity is 8 because 8 is the widest frame measured across the OWL 2 RL
    /// rule table and this workspace's suites. A measurement is not a guarantee, and a
    /// comment recording one is not checkable — so the claim is asserted here instead,
    /// on the two facts that make it worth having: a frame at the capacity does not
    /// reach the allocator, and one past it still WORKS rather than breaking.
    ///
    /// If a future rule needs a ninth variable this does not fail; it spills, exactly as
    /// a `Vec` always did. What would fail is someone shrinking the capacity below the
    /// width the rules actually use, which is the silent regression this pins.
    #[test]
    fn the_widest_measured_frame_never_reaches_the_allocator() {
        let inline = SlotSolution::empty(8);
        assert!(
            !inline.bindings.spilled(),
            "a frame of the widest measured width must live inline; it is cloned once \
             per surviving candidate row, so spilling here is one allocation per \
             intermediate solution"
        );

        let spilled = SlotSolution::empty(9);
        assert!(
            spilled.bindings.spilled(),
            "a frame past the inline capacity is expected to spill — this asserts the \
             capacity is where it is claimed to be, so the test above cannot pass \
             vacuously by the capacity having been raised"
        );
        assert_eq!(
            spilled.bindings.len(),
            9,
            "spilling changes where the frame lives, never how wide it is"
        );
    }

    // ── Basic fixpoint behaviour ────────────────────────────────────────────────

    /// Transitive closure over a three-edge chain, the smallest recursive program.
    #[test]
    fn fixpoint_closes_a_recursive_rule() {
        let rules = vec![
            DlClause::datalog(atom("?s", Q, "?o"), vec![atom("?s", P, "?o")]),
            DlClause::datalog(
                atom("?s", Q, "?o"),
                vec![atom("?s", P, "?m"), atom("?m", Q, "?o")],
            ),
        ];
        let edb = store_of(&[("a", P, "b"), ("b", P, "c"), ("c", P, "d")]);
        let evaluation = run(rules, edb);
        let expected: BTreeSet<(String, String)> = [
            ("a", "b"),
            ("b", "c"),
            ("c", "d"),
            ("a", "c"),
            ("b", "d"),
            ("a", "d"),
        ]
        .into_iter()
        .map(|(s, o)| (surface(s), surface(o)))
        .collect();
        assert_eq!(relation(&evaluation, Q), expected);
    }

    /// A derived fact's proof height is one more than its deepest source, and its sources
    /// are recorded in AUTHORED body order even when the planner reorders execution.
    #[test]
    fn derivations_carry_authored_order_provenance_and_proof_height() {
        let rules = vec![DlClause::datalog(
            atom("?s", R, "?o"),
            vec![atom("?m", Q, "?o"), atom("?s", P, "?m")],
        )];
        let edb = store_of(&[("a", P, "b"), ("b", Q, "c")]);
        let evaluation = run(rules, edb);
        assert_eq!(evaluation.derivations().len(), 1);
        let derivation = &evaluation.derivations()[0];
        assert_eq!(derivation.rule(), 0);
        assert_eq!(derivation.proof_height(), 1);
        assert_eq!(
            derivation
                .sources()
                .iter()
                .map(|fact| fact.predicate.as_str())
                .collect::<Vec<_>>(),
            [surface(Q), surface(P)],
            "sources follow the authored body, not the execution order"
        );
        assert_eq!(derivation.fact().subject, surface("a"));
        assert_eq!(derivation.fact().object, surface("c"));
    }

    /// Stratified negation: `r` is derived only where `q` is absent, and `q`'s stratum is
    /// fully closed first.
    #[test]
    fn stratified_negation_reads_a_completed_lower_stratum() {
        let rules = vec![
            DlClause::datalog(atom("?s", Q, "?o"), vec![atom("?s", P, "?o")]),
            DlClause::datalog(
                atom("?s", R, "?o"),
                vec![
                    ClauseAtom::positive(v("?s"), "https://example.org/base", v("?o")),
                    ClauseAtom::negated(v("?s"), Q, v("?o")),
                ],
            ),
        ];
        let edb = store_of(&[
            ("a", P, "b"),
            ("a", "https://example.org/base", "b"),
            ("c", "https://example.org/base", "d"),
        ]);
        let evaluation = run(rules, edb);
        assert_eq!(
            relation(&evaluation, R),
            BTreeSet::from([(surface("c"), surface("d"))]),
            "only the pair with no q fact survives negation"
        );
    }

    /// Existential negation-as-failure: an unbound position is unconstrained, so the atom
    /// reads "this subject has NO fact under the predicate at all".
    #[test]
    fn existential_negation_probes_the_ground_positions_only() {
        let rules = vec![DlClause::datalog(
            atom("?s", R, "?o"),
            vec![
                ClauseAtom::positive(v("?s"), "https://example.org/base", v("?o")),
                ClauseAtom::negated(v("?s"), Q, v("?free")),
            ],
        )];
        let edb = store_of(&[
            ("a", "https://example.org/base", "b"),
            ("c", "https://example.org/base", "d"),
            ("a", Q, "zzz"),
        ]);
        let evaluation = run(rules, edb);
        assert_eq!(
            relation(&evaluation, R),
            BTreeSet::from([(surface("c"), surface("d"))]),
            "a has SOME q fact, so the negated atom blocks it"
        );
    }

    /// A rule with a constant head introduces a term the store has never interned; the
    /// candidate key handles it and the fact commits exactly once.
    #[test]
    fn a_fresh_head_constant_commits_once() {
        let rules = vec![DlClause::datalog(
            ClauseAtom::positive(iri("https://example.org/new"), R, v("?o")),
            vec![atom("?s", P, "?o")],
        )];
        let edb = store_of(&[("a", P, "b"), ("c", P, "b")]);
        let evaluation = run(rules, edb);
        assert_eq!(
            relation(&evaluation, R),
            BTreeSet::from([(surface("https://example.org/new"), surface("b"))])
        );
        assert_eq!(evaluation.derivations().len(), 1);
    }

    /// A body with no positive atom is relational identity: it fires once and its head is
    /// suppressed on the next round by the store's own membership test.
    #[test]
    fn an_unconditional_rule_fires_exactly_once() {
        let rules = vec![DlClause::datalog(
            ClauseAtom::positive(
                iri("https://example.org/a"),
                R,
                iri("https://example.org/b"),
            ),
            Vec::new(),
        )];
        let evaluation = run(rules, RelationStore::new());
        assert_eq!(evaluation.derivations().len(), 1);
        assert_eq!(evaluation.facts().row_count(), 1);
    }

    /// Two rules deriving the same head in one round: the winner is the one the total
    /// provenance order prefers, whichever rule ran first.
    #[test]
    fn a_round_collision_is_decided_by_the_provenance_order() {
        // Both rules derive r(a, b): rule 0 from one source, rule 1 from another.
        let rules = vec![
            DlClause::datalog(atom("?s", R, "?o"), vec![atom("?s", P, "?o")]),
            DlClause::datalog(atom("?s", R, "?o"), vec![atom("?s", Q, "?o")]),
        ];
        let edb = store_of(&[("a", P, "b"), ("a", Q, "b")]);
        let evaluation = run(rules, edb);
        assert_eq!(evaluation.derivations().len(), 1);
        let derivation = &evaluation.derivations()[0];
        // Equal proof heights and equal sums, so the sorted source facts decide: the
        // `p` fact sorts before the `q` fact, so rule 0 wins.
        assert_eq!(derivation.rule(), 0);
        assert_eq!(derivation.sources()[0].predicate, surface(P));
    }

    // ── The predicate as data ───────────────────────────────────────────────────

    /// The fixture's own `rdf:type`-shaped and `rdfs:domain`-shaped predicates. PurRDF
    /// mints no vocabulary, so the fixture names its own under `example.org`.
    const TYPE: &str = "https://example.org/type";
    /// The `rdfs:domain`-shaped predicate.
    const DOMAIN: &str = "https://example.org/domain";
    /// The `rdfs:subClassOf`-shaped predicate.
    const SUB_CLASS_OF: &str = "https://example.org/subClassOf";

    /// `prp-dom` — the rule the whole predicate-as-data change exists for:
    ///
    /// ```text
    /// T(?p, domain, ?c) ∧ T(?x, ?p, ?y) → T(?x, type, ?c)
    /// ```
    ///
    /// `?p` stands in PREDICATE position of the second body atom. Under an IR that
    /// addressed a relation by its predicate this rule could not be written at all.
    fn prp_dom() -> DlClause {
        DlClause::datalog(
            ClauseAtom::positive(v("?x"), TYPE, v("?c")),
            vec![
                ClauseAtom::positive(v("?p"), DOMAIN, v("?c")),
                ClauseAtom::quad(v("?x"), v("?p"), v("?y"), ClauseTerm::DefaultGraph),
            ],
        )
    }

    /// The `prp-dom` fixture EDB: two properties with declared domains, one property with
    /// none, and assertions over all three.
    fn prp_dom_edb() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            ("https://example.org/p1", DOMAIN, "https://example.org/C1"),
            ("https://example.org/p2", DOMAIN, "https://example.org/C2"),
            (
                "https://example.org/x",
                "https://example.org/p1",
                "https://example.org/y",
            ),
            (
                "https://example.org/x",
                "https://example.org/p2",
                "https://example.org/z",
            ),
            (
                "https://example.org/w",
                "https://example.org/p1",
                "https://example.org/v",
            ),
            // A property with no declared domain derives nothing.
            (
                "https://example.org/x",
                "https://example.org/undeclared",
                "https://example.org/y",
            ),
        ]
    }

    /// A rule with a VARIABLE PREDICATE evaluates, and its derived set is exactly the one
    /// `prp-dom` licenses — no more (an undeclared property contributes nothing) and no
    /// less (every asserted use of a domained property types its subject).
    ///
    /// This is the test that proves the structural defect is fixed: the rule is
    /// inexpressible without a predicate-as-data encoding, so an evaluator that could not
    /// bind `?p` could not even be handed this program.
    #[test]
    fn a_variable_predicate_rule_evaluates_prp_dom() {
        let evaluation = run(vec![prp_dom()], store_of(&prp_dom_edb()));
        assert_eq!(
            relation(&evaluation, TYPE),
            [
                (
                    surface("https://example.org/w"),
                    surface("https://example.org/C1")
                ),
                (
                    surface("https://example.org/x"),
                    surface("https://example.org/C1")
                ),
                (
                    surface("https://example.org/x"),
                    surface("https://example.org/C2")
                ),
            ]
            .into_iter()
            .collect(),
            "exactly the three typings prp-dom licenses"
        );
        assert_eq!(
            evaluation.derivations().len(),
            3,
            "one derivation per derived typing"
        );
        // Provenance names the concrete predicate each body atom MATCHED, which for the
        // variable-predicate atom is data rather than rule text.
        let matched: BTreeSet<String> = evaluation
            .derivations()
            .iter()
            .map(|derivation| derivation.sources()[1].predicate.clone())
            .collect();
        assert_eq!(
            matched,
            [
                surface("https://example.org/p1"),
                surface("https://example.org/p2")
            ]
            .into_iter()
            .collect(),
            "the variable predicate bound to the properties that actually carry a domain"
        );
    }

    /// The `prp-dom` program is byte-stable under a permuted EDB insertion order: the
    /// arity-4 partitioned store and the partition sweep a variable predicate drives are
    /// both insertion-order independent.
    #[test]
    fn a_variable_predicate_rule_is_insertion_order_independent() {
        let triples = prp_dom_edb();
        let reference = run(vec![prp_dom()], store_of(&triples));
        for seed in 0..12u64 {
            let again = run(vec![prp_dom()], store_of(&permute(&triples, seed)));
            assert_eq!(
                again.facts().facts_sorted(),
                reference.facts().facts_sorted(),
                "seed {seed}: facts"
            );
            assert_eq!(
                again.derivations(),
                reference.derivations(),
                "seed {seed}: derivations"
            );
            assert_eq!(again.budget(), reference.budget(), "seed {seed}: budget");
        }
    }

    /// A rule with a VARIABLE GRAPH position reasons PER GRAPH: it joins only within one
    /// graph and writes its conclusion back into that same graph.
    ///
    /// The fixture is `cax-sco` over two graphs whose premises cross: taking the subclass
    /// axiom from one graph and the type assertion from the other would derive two extra
    /// facts, so the exact derived set is what proves the graph position is a join key and
    /// not decoration.
    #[test]
    fn a_variable_graph_rule_evaluates_per_graph() {
        let g1 = surface("https://example.org/g1");
        let g2 = surface("https://example.org/g2");
        let graph = ClauseTerm::var("?g");
        let rule = DlClause::datalog(
            ClauseAtom::quad(v("?x"), ClauseTerm::iri(TYPE), v("?d"), graph.clone()),
            vec![
                ClauseAtom::quad(v("?x"), ClauseTerm::iri(TYPE), v("?c"), graph.clone()),
                ClauseAtom::quad(v("?c"), ClauseTerm::iri(SUB_CLASS_OF), v("?d"), graph),
            ],
        );

        let mut edb = RelationStore::new();
        let quads = [
            // g1: x is a C, and C ⊑ D — but NOT C ⊑ E.
            ("x", TYPE, "C", &g1),
            ("C", SUB_CLASS_OF, "D", &g1),
            // g2: y is a C, and C ⊑ E — but NOT C ⊑ D.
            ("y", TYPE, "C", &g2),
            ("C", SUB_CLASS_OF, "E", &g2),
        ];
        for (subject, predicate, object, graph) in quads {
            edb.insert(
                &surface(&format!("https://example.org/{subject}")),
                &surface(predicate),
                &surface(&format!("https://example.org/{object}")),
                graph,
            );
        }

        let evaluation = run(vec![rule], edb);
        let derived: BTreeSet<(String, String, String)> = evaluation
            .derivations()
            .iter()
            .map(|derivation| {
                let fact = derivation.fact();
                (
                    fact.subject.clone(),
                    fact.object.clone(),
                    fact.graph.clone(),
                )
            })
            .collect();
        assert_eq!(
            derived,
            [
                (
                    surface("https://example.org/x"),
                    surface("https://example.org/D"),
                    g1.clone()
                ),
                (
                    surface("https://example.org/y"),
                    surface("https://example.org/E"),
                    g2.clone()
                ),
            ]
            .into_iter()
            .collect(),
            "each graph's conclusion stays in the graph its premises came from"
        );
        // The cross-graph joins are absent, which is the whole point.
        for (subject, object, graph) in [("x", "E", &g1), ("y", "D", &g2), ("x", "D", &g2)] {
            assert!(
                !evaluation.facts().contains(
                    &surface(&format!("https://example.org/{subject}")),
                    &surface(TYPE),
                    &surface(&format!("https://example.org/{object}")),
                    graph
                ),
                "{subject} must not be typed {object} in that graph"
            );
        }
        // The default graph carries nothing: no atom of this rule mentions it.
        assert_eq!(
            evaluation.facts().graphs().collect::<Vec<_>>(),
            vec![g1.as_str(), g2.as_str()]
        );
    }

    /// A FREE predicate is a sweep of the store's partitions, and the sweep is a real
    /// evaluation rather than a plan-time curiosity: this rule projects every quad's
    /// predicate into a term position, so the answer names exactly the predicates the EDB
    /// carries.
    #[test]
    fn a_free_predicate_sweeps_every_partition() {
        let used = "https://example.org/used";
        let rule = DlClause::datalog(
            ClauseAtom::positive(v("?p"), used, v("?g")),
            vec![ClauseAtom::quad(
                v("?s"),
                v("?p"),
                v("?o"),
                ClauseTerm::var("?g"),
            )],
        );
        let g1 = surface("https://example.org/g1");
        let mut edb = RelationStore::new();
        for (subject, predicate, object, graph) in [
            (
                "a",
                "https://example.org/p1",
                "b",
                RelationStore::DEFAULT_GRAPH,
            ),
            (
                "a",
                "https://example.org/p2",
                "b",
                RelationStore::DEFAULT_GRAPH,
            ),
            ("a", "https://example.org/p1", "b", g1.as_str()),
        ] {
            edb.insert(
                &surface(&format!("https://example.org/{subject}")),
                &surface(predicate),
                &surface(&format!("https://example.org/{object}")),
                graph,
            );
        }
        let evaluation = run(vec![rule], edb);
        assert_eq!(
            relation(&evaluation, used),
            [
                (
                    surface("https://example.org/p1"),
                    RelationStore::DEFAULT_GRAPH.to_owned()
                ),
                (surface("https://example.org/p1"), g1.clone()),
                (
                    surface("https://example.org/p2"),
                    RelationStore::DEFAULT_GRAPH.to_owned()
                ),
                // The rule's own output is a partition too, so the sweep reaches it on the
                // next round and the fixpoint closes over it — a free predicate really is
                // "every relation", including the one being derived.
                (surface(used), RelationStore::DEFAULT_GRAPH.to_owned()),
            ]
            .into_iter()
            .collect(),
            "every (predicate, graph) partition is visited, the derived one included"
        );
        // The derived `used` quads live in the default graph (the head names no graph),
        // and a graph NAME reached a term position — the default graph included, as the
        // empty surface it is denoted by.
        assert!(evaluation.facts().contains(
            &surface("https://example.org/p1"),
            &surface(used),
            RelationStore::DEFAULT_GRAPH,
            RelationStore::DEFAULT_GRAPH
        ));
    }

    // ── The caller's stop signal ────────────────────────────────────────────────

    /// A stop signal that fires from its `n`-th poll onward, and latches.
    #[derive(Debug)]
    struct StopsAfter {
        /// Polls remaining before the signal fires. Saturates at zero, which is what makes
        /// it latch.
        remaining: std::sync::atomic::AtomicU64,
    }

    impl StopSignal for StopsAfter {
        fn stopped(&self) -> bool {
            use std::sync::atomic::Ordering;
            let left = self.remaining.load(Ordering::Relaxed);
            if left == 0 {
                return true;
            }
            self.remaining.store(left - 1, Ordering::Relaxed);
            false
        }
    }

    /// A SIGNAL THAT NEVER FIRES CHANGES NOTHING — not the relations, not the derivations,
    /// not the budget report.
    ///
    /// This is the property that admits a stop signal into a crate whose budgets are
    /// deliberately constants: attaching one is observationally nothing until it fires.
    /// Asserted over the whole synthetic corpus rather than over one program, and against
    /// the ungoverned [`evaluate`] rather than against itself.
    #[test]
    fn an_unfired_stop_signal_changes_no_answer() {
        let never = StopsAfter {
            remaining: std::sync::atomic::AtomicU64::new(u64::MAX),
        };
        for workload in synth_corpus::all() {
            let exe = compile(workload.rules.clone()).expect("the corpus program compiles");
            let plain = evaluate(&exe, workload.edb()).expect("inside every ceiling");
            let watched =
                evaluate_until(&exe, workload.edb(), Some(&never)).expect("inside every ceiling");
            assert_eq!(
                plain.facts().facts_sorted(),
                watched.facts().facts_sorted(),
                "{}: facts",
                workload.name
            );
            assert_eq!(
                plain.derivations(),
                watched.derivations(),
                "{}: derivations",
                workload.name
            );
            assert_eq!(
                format!("{:?}", plain.budget()),
                format!("{:?}", watched.budget()),
                "{}: budget",
                workload.name
            );
        }
    }

    /// A SIGNAL THAT FIRES PRODUCES NO ANSWER AT ALL — never a truncated one.
    ///
    /// The refusal is its own variant rather than a budget's, because "the host stopped this
    /// run" and "the program passed a fixed ceiling" are different facts and only one of them
    /// is a reason to change the input. The report is the consumption measured at the point
    /// the signal was observed, so a stopped run can still say what it had spent.
    #[test]
    fn a_fired_stop_signal_refuses_rather_than_truncating() {
        for workload in synth_corpus::all() {
            let exe = compile(workload.rules.clone()).expect("the corpus program compiles");
            // Fires at the FIRST round boundary, before any rule of any stratum has run.
            let immediate = StopsAfter {
                remaining: std::sync::atomic::AtomicU64::new(0),
            };
            let error = evaluate_until(&exe, workload.edb(), Some(&immediate))
                .expect_err("a signal that is already firing must stop the fixpoint");
            let EvalError::Stopped { report } = error else {
                panic!("{}: a stop is not a ceiling: {error:?}", workload.name);
            };
            assert_eq!(
                report.join_steps, 0,
                "{}: nothing was enumerated before the first round",
                workload.name
            );
            assert!(
                error_mentions_stop(&EvalError::Stopped { report }),
                "{}: the message must say the run was stopped",
                workload.name
            );
        }
    }

    /// Whether an error renders as a stop rather than as a ceiling.
    fn error_mentions_stop(error: &EvalError) -> bool {
        let rendered = error.to_string();
        rendered.contains("stopped by the caller's stop signal")
            && rendered.contains("no least model was computed")
    }

    // ── The synthetic corpus against its analytic goldens ───────────────────────

    /// Every corpus program's computed relations equal their analytically-known goldens
    /// EXACTLY — set equality, not containment.
    ///
    /// This is the crate's only non-tautological correctness oracle: "byte-identical
    /// across 100 runs" proves determinism, and a systematically wrong evaluator passes it
    /// every time. A closed-form transitive closure, a complete strongly-connected
    /// component, the same-generation pairs of a two-level tree and a single-source
    /// reachability set are all computed here by construction, never by an engine.
    #[test]
    fn synth_corpus_matches_its_analytic_goldens() {
        for workload in synth_corpus::all() {
            let name = workload.name;
            let expected = workload.expected.clone();
            let evaluation = run(workload.rules.clone(), workload.edb());
            let facts: BTreeSet<Fact> = evaluation.facts().facts_sorted().into_iter().collect();
            let seeded: BTreeSet<Fact> = workload.edb().facts_sorted().into_iter().collect();
            let derived: BTreeSet<Fact> = facts.difference(&seeded).cloned().collect();
            assert_eq!(
                derived, expected,
                "{name}: derived relation vs analytic golden"
            );
            assert_eq!(
                derived.len() as u64,
                workload.expected_rows,
                "{name}: derived count vs closed-form formula"
            );
        }
    }

    /// The corpus programs are byte-stable under a permuted EDB insertion order: the same
    /// facts inserted in a different sequence yield the same relations and the same
    /// derivations.
    #[test]
    fn synth_corpus_is_insertion_order_independent() {
        for workload in synth_corpus::all() {
            let reference = run(workload.rules.clone(), workload.edb());
            for seed in 0..4u64 {
                let mut store = RelationStore::new();
                for quad in permute(&workload.triples, seed) {
                    store.insert(&quad.0, &quad.1, &quad.2, &quad.3);
                }
                let again = run(workload.rules.clone(), store);
                assert_eq!(
                    again.facts().facts_sorted(),
                    reference.facts().facts_sorted(),
                    "{}: seed {seed} facts",
                    workload.name
                );
                assert_eq!(
                    again.derivations(),
                    reference.derivations(),
                    "{}: seed {seed} derivations",
                    workload.name
                );
            }
        }
    }

    // ── The required differential tests ─────────────────────────────────────────

    /// The leapfrog triejoin and the indexed binary fallback are two implementations of
    /// one contract, so they must agree everywhere — on the whole synthetic corpus and on
    /// a certified triangle, which is the shape only the triejoin is chosen for.
    #[test]
    fn leapfrog_and_binary_joins_agree() {
        let mut cyclic_seen = false;
        for workload in synth_corpus::all().into_iter().chain([triangle_workload()]) {
            let exe = compile(workload.rules.clone()).expect("the corpus program compiles");
            let cyclic =
                (0..exe.rule_count()).any(|index| exe.rule_entry(index).1.has_cyclic_subplan());
            cyclic_seen |= cyclic;
            assert_eq!(
                cyclic,
                workload.name == "triangle",
                "{}: the triangle is the fixture that actually routes through the triejoin, \
                 and it is the only one — if that changed, this test's coverage changed with it",
                workload.name
            );

            let planned = evaluate_with(
                &exe,
                workload.edb(),
                None,
                &NoGuards,
                EvalOptions::default(),
                RoundExecution::Parallel,
                JoinStrategy::Planned,
            )
            .expect("planned join stays inside every ceiling");
            let binary = evaluate_with(
                &exe,
                workload.edb(),
                None,
                &NoGuards,
                EvalOptions::default(),
                RoundExecution::Parallel,
                JoinStrategy::ForcedBinary,
            )
            .expect("forced binary join stays inside every ceiling");

            assert_eq!(
                planned.facts().facts_sorted(),
                binary.facts().facts_sorted(),
                "{}: the two joins must derive the identical relation",
                workload.name
            );
            assert_eq!(
                planned.derivations(),
                binary.derivations(),
                "{}: the two joins must agree on provenance too",
                workload.name
            );
        }
        assert!(
            cyclic_seen,
            "the differential fixtures must include a certified cyclic rule, or the two \
             strategies were never actually different"
        );
    }

    /// The certified triangle `s(X, Z) :- p(X, Y), q(Y, Z), r(Z, X)` over a small circulant
    /// graph — the canonical worst-case-optimal-join shape, with an analytic golden.
    ///
    /// `p` is the COMPLETE `n × n` relation, `q` is the successor `Z = Y + 1` and `r` is
    /// `X = Z + 2`, all modulo `n`. Then `p(X, Y) ∧ q(Y, Z)` leaves `Z` free (every `Y` is
    /// reachable and every `Z` is some `Y`'s successor), and `r(Z, X)` pins `Z = X - 2`.
    /// The answer is therefore exactly the `n` pairs `(x, x - 2)` out of `n²` candidates —
    /// a join that filters, so an implementation that simply returned everything would
    /// fail rather than pass.
    fn triangle_workload() -> SynthWorkload {
        const N: usize = 5;
        let n = |i: usize| surface(&format!("https://example.org/n{i}"));
        let mut triples = Vec::new();
        for i in 0..N {
            for j in 0..N {
                triples.push(quad(&n(i), P, &n(j)));
            }
            triples.push(quad(&n(i), Q, &n((i + 1) % N)));
            triples.push(quad(&n(i), R, &n((i + 2) % N)));
        }
        let sink = "https://example.org/s";
        let rules = vec![DlClause::datalog(
            atom("?X", sink, "?Z"),
            vec![
                atom("?X", P, "?Y"),
                atom("?Y", Q, "?Z"),
                atom("?Z", R, "?X"),
            ],
        )];
        let expected: BTreeSet<Fact> = (0..N)
            .map(|x| Fact {
                subject: n(x),
                predicate: surface(sink),
                object: n((x + N - 2) % N),
                graph: RelationStore::DEFAULT_GRAPH.to_owned(),
            })
            .collect();
        let expected_rows = expected.len() as u64;
        assert_eq!(expected_rows, N as u64);
        SynthWorkload {
            name: "triangle",
            rules,
            triples,
            expected,
            expected_rows,
        }
    }

    /// The certified triangle's answer equals its analytic golden, so the differential
    /// test above is comparing two CORRECT joins rather than two identically wrong ones.
    #[test]
    fn the_certified_triangle_matches_its_analytic_golden() {
        let workload = triangle_workload();
        let evaluation = run(workload.rules.clone(), workload.edb());
        let seeded: BTreeSet<Fact> = workload.edb().facts_sorted().into_iter().collect();
        let facts: BTreeSet<Fact> = evaluation.facts().facts_sorted().into_iter().collect();
        let derived: BTreeSet<Fact> = facts.difference(&seeded).cloned().collect();
        assert_eq!(derived, workload.expected);
    }

    /// Sequential and rule-parallel rounds are two schedules of one computation: the
    /// program-order merge erases scheduling, so neither the answer nor the budget
    /// observation may move.
    #[test]
    fn sequential_and_parallel_rounds_agree() {
        for workload in synth_corpus::all() {
            let exe = compile(workload.rules.clone()).expect("the corpus program compiles");
            let sequential = evaluate_with(
                &exe,
                workload.edb(),
                None,
                &NoGuards,
                EvalOptions::default(),
                RoundExecution::Sequential,
                JoinStrategy::Planned,
            )
            .expect("sequential rounds stay inside every ceiling");
            let parallel = evaluate_with(
                &exe,
                workload.edb(),
                None,
                &NoGuards,
                EvalOptions::default(),
                RoundExecution::Parallel,
                JoinStrategy::Planned,
            )
            .expect("parallel rounds stay inside every ceiling");
            assert_eq!(
                sequential.facts().facts_sorted(),
                parallel.facts().facts_sorted(),
                "{}: facts",
                workload.name
            );
            assert_eq!(
                sequential.derivations(),
                parallel.derivations(),
                "{}: derivations",
                workload.name
            );
            assert_eq!(
                sequential.budget(),
                parallel.budget(),
                "{}: budget consumption",
                workload.name
            );
        }
    }

    // ── The required refusal tests ──────────────────────────────────────────────

    /// An EXISTENTIAL head is refused by name.
    ///
    /// `∃?y. q(?x, ?y) :- p(?x, ?z)` asks for a witness this evaluator cannot mint: a
    /// least-fixpoint Datalog evaluator derives only ground atoms over the terms it was
    /// given. The refusal names the clause and the form, and it is the permanent, correct
    /// answer this evaluator owes a caller who hands it a non-Datalog clause.
    #[test]
    fn an_existential_head_is_refused_by_name() {
        let rules = vec![DlClause::new(
            vec![HeadDisjunct::atom(atom("?x", Q, "?y"))],
            vec!["?y".to_owned()],
            vec![atom("?x", P, "?z")],
        )];
        let error = compile(rules).expect_err("an existential head is not a Datalog rule");
        assert_eq!(
            error,
            EvalError::NonDatalogHead {
                rule: 0,
                form: HeadForm::Existential,
            }
        );
        let rendered = error.to_string();
        assert!(rendered.contains("existential"), "{rendered}");
        assert!(rendered.contains("clause 0"), "{rendered}");
    }

    /// A DISJUNCTIVE head is refused by name.
    ///
    /// `q(?x, ?y) ∨ r(?x, ?y) :- p(?x, ?y)` has no single least model — a case split has
    /// to choose — so there is nothing for a least-fixpoint evaluator to compute. Deriving
    /// the first disjunct, or all of them, would both be wrong answers. The form HAS a
    /// consumer in the workspace (`purrdf-entail`'s OWL-Direct hypertableau branches on it);
    /// what this asserts is that the consumer is not this evaluator, and that the refusal
    /// names the form instead of failing to parse it.
    #[test]
    fn a_disjunctive_head_is_refused_by_name() {
        let rules = vec![DlClause::new(
            vec![
                HeadDisjunct::atom(atom("?x", Q, "?y")),
                HeadDisjunct::atom(atom("?x", R, "?y")),
            ],
            Vec::new(),
            vec![atom("?x", P, "?y")],
        )];
        let error = compile(rules).expect_err("a disjunctive head is not a Datalog rule");
        assert_eq!(
            error,
            EvalError::NonDatalogHead {
                rule: 0,
                form: HeadForm::Disjunctive,
            }
        );
        assert!(error.to_string().contains("a disjunctive head"));
    }

    /// A CONJUNCTIVE head is refused by a name that is TRUE.
    ///
    /// `q(?x, ?y) ∧ r(?x, ?y) :- p(?x, ?y)` is not a Datalog rule — a definite clause has
    /// exactly one head atom — but it is not a disjunction either, and there is no witness
    /// to mint. Reporting it as "disjunctive" or "existential" would name a property the
    /// clause does not have, so the refusal carries its own form. The clause IS equivalent
    /// to two Datalog rules, and the evaluator still refuses it rather than splitting it:
    /// the split would renumber the program that a derivation's clause index names.
    #[test]
    fn a_conjunctive_head_is_refused_by_its_own_name() {
        let rules = vec![DlClause::new(
            vec![HeadDisjunct::new(vec![
                atom("?x", Q, "?y"),
                atom("?x", R, "?y"),
            ])],
            Vec::new(),
            vec![atom("?x", P, "?y")],
        )];
        let error = compile(rules).expect_err("a conjunctive head is not a Datalog rule");
        assert_eq!(
            error,
            EvalError::NonDatalogHead {
                rule: 0,
                form: HeadForm::Conjunctive,
            }
        );
        let rendered = error.to_string();
        assert!(rendered.contains("a conjunctive head"), "{rendered}");
        assert!(
            !rendered.contains("disjunctive"),
            "the refusal must not name a property the clause lacks: {rendered}"
        );
        // The trailing "(one head atom, no existential)" states what a Datalog clause IS,
        // so only the phrase that NAMES this clause's form is asserted against.
        assert!(
            !rendered.contains("an existential head"),
            "the refusal must not name a property the clause lacks: {rendered}"
        );
    }

    /// The `A ⊑ ∃r.C` lowering — one disjunct, two conjuncts, one shared witness — is
    /// refused as EXISTENTIAL, because the quantifier outranks the conjunction in the
    /// documented precedence and it is the quantifier that a chase, not this evaluator,
    /// must consume.
    #[test]
    fn a_conjunctive_existential_head_is_refused_as_existential() {
        let rules = vec![DlClause::new(
            vec![HeadDisjunct::new(vec![
                atom("?x", R, "?y"),
                ClauseAtom::positive(
                    v("?y"),
                    "https://example.org/type",
                    ClauseTerm::iri("https://example.org/C"),
                ),
            ])],
            vec!["?y".to_owned()],
            vec![atom("?x", P, "?z")],
        )];
        let error = compile(rules).expect_err("an existential head is not a Datalog rule");
        assert_eq!(
            error,
            EvalError::NonDatalogHead {
                rule: 0,
                form: HeadForm::Existential,
            }
        );
    }

    /// An EMPTY head — the inconsistency clause `body → false` — is refused by name.
    ///
    /// It derives nothing and instead asserts its body is unsatisfiable, so silently
    /// admitting it would turn a claim about the model into a rule that does nothing at
    /// all: the caller would be told the program is consistent because the evaluator never
    /// checked.
    #[test]
    fn an_empty_head_is_refused_by_name() {
        let rules = vec![
            DlClause::datalog(atom("?s", Q, "?o"), vec![atom("?s", P, "?o")]),
            DlClause::inconsistency(vec![atom("?s", Q, "?o")]),
        ];
        let error = compile(rules).expect_err("an inconsistency clause is not a Datalog rule");
        assert_eq!(
            error,
            EvalError::NonDatalogHead {
                rule: 1,
                form: HeadForm::Inconsistency,
            }
        );
        assert!(error.to_string().contains("empty (false)"));
        let _: &dyn std::error::Error = &error;
    }

    /// The head-form refusal is decided BEFORE stratification and range restriction,
    /// because both of those are defined in terms of a single head atom. A clause that is
    /// non-Datalog AND not range-restricted is reported as non-Datalog: the other
    /// diagnostic would be a consequence of the real defect.
    #[test]
    fn the_head_form_refusal_precedes_the_other_two() {
        // `∃?y. q(?free, ?y) :- p(?x, ?z)` is also not range-restricted (`?free` is
        // unbindable) and its predicates form no cycle, so only the ordering decides.
        let rules = vec![DlClause::new(
            vec![HeadDisjunct::atom(atom("?free", Q, "?y"))],
            vec!["?y".to_owned()],
            vec![atom("?x", P, "?z")],
        )];
        assert!(matches!(
            compile(rules),
            Err(EvalError::NonDatalogHead { rule: 0, .. })
        ));
    }

    /// A negative edge inside a dependency cycle is a hard error naming the cycle — never
    /// a silent accept and never a best-effort evaluation.
    #[test]
    fn a_negative_edge_in_a_cycle_is_non_stratifiable() {
        // p :- base, not q.   q :- p.  — every variable is range-restricted, so the ONLY
        // defect is the negative edge inside the p -> q -> p cycle.
        let rules = vec![
            DlClause::datalog(
                atom("?s", P, "?o"),
                vec![
                    ClauseAtom::positive(v("?s"), "https://example.org/base", v("?o")),
                    ClauseAtom::negated(v("?s"), Q, v("?o")),
                ],
            ),
            DlClause::datalog(atom("?s", Q, "?o"), vec![atom("?s", P, "?o")]),
        ];
        let error = compile(rules).expect_err("a negative edge in a cycle has no stratification");
        let EvalError::NonStratifiable {
            head,
            negated,
            cycle,
        } = &error
        else {
            panic!("expected a stratification refusal, got {error:?}");
        };
        assert_eq!(head, &surface(P));
        assert_eq!(negated, &surface(Q));
        assert_eq!(cycle, &[surface(P), surface(Q), surface(P)]);
        assert!(
            error.to_string().contains("not "),
            "the message names the negated dependency: {error}"
        );
    }

    /// A longer negative cycle is named in full, through the intermediate predicate.
    #[test]
    fn a_longer_negative_cycle_is_named_through_its_path() {
        // p :- base, not q.   q :- r.   r :- p.
        let rules = vec![
            DlClause::datalog(
                atom("?s", P, "?o"),
                vec![
                    ClauseAtom::positive(v("?s"), "https://example.org/base", v("?o")),
                    ClauseAtom::negated(v("?s"), Q, v("?o")),
                ],
            ),
            DlClause::datalog(atom("?s", Q, "?o"), vec![atom("?s", R, "?o")]),
            DlClause::datalog(atom("?s", R, "?o"), vec![atom("?s", P, "?o")]),
        ];
        let error = compile(rules).expect_err("the negative edge sits in a three-predicate cycle");
        let EvalError::NonStratifiable { cycle, .. } = &error else {
            panic!("expected a stratification refusal, got {error:?}");
        };
        assert_eq!(cycle, &[surface(P), surface(Q), surface(R), surface(P)]);
    }

    /// Negation OUTSIDE a cycle is perfectly stratifiable and compiles.
    #[test]
    fn acyclic_negation_compiles() {
        let rules = vec![
            DlClause::datalog(atom("?s", Q, "?o"), vec![atom("?s", P, "?o")]),
            DlClause::datalog(
                atom("?s", R, "?o"),
                vec![
                    atom("?s", P, "?o"),
                    ClauseAtom::negated(v("?s"), Q, v("?o")),
                ],
            ),
        ];
        assert!(compile(rules).is_ok());
    }

    /// A head variable no positive body atom can bind is refused at compile time.
    #[test]
    fn an_unbindable_head_variable_is_refused() {
        let rules = vec![DlClause::datalog(
            atom("?s", R, "?free"),
            vec![atom("?s", P, "?o")],
        )];
        let error = compile(rules).expect_err("a head variable must be range-restricted");
        assert_eq!(
            error,
            EvalError::UnboundHeadVariable {
                rule: 0,
                variable: "?free".to_owned(),
            }
        );
        assert!(error.to_string().contains("?free"));
    }

    // ── Budgets ────────────────────────────────────────────────────────────────

    /// The join-step ceiling stops a Cartesian blow-up that commits almost nothing, and
    /// the returned report is the exact observation that tripped it — not a truncated
    /// answer, not a panic.
    #[test]
    fn the_join_step_ceiling_is_a_distinguishable_error() {
        // sink(?c, ?c) :- src(?x, ?c), src(?y, ?c). One head fact, n^2 candidates.
        let src = "https://example.org/src";
        let sink = "https://example.org/sink";
        let rules = vec![DlClause::datalog(
            ClauseAtom::positive(v("?c"), sink, v("?c")),
            vec![atom("?x", src, "?c"), atom("?y", src, "?c")],
        )];
        let n = 1100usize; // 1_210_000 candidate solutions > MAX_JOIN_STEPS
        let mut edb = RelationStore::new();
        for i in 0..n {
            edb.insert(
                &surface(&format!("https://example.org/n{i}")),
                &surface(src),
                &surface("https://example.org/hub"),
                RelationStore::DEFAULT_GRAPH,
            );
        }
        let exe = compile(rules).expect("the fixture compiles");
        let error = evaluate(&exe, edb).expect_err("the join-step ceiling must be passed");
        let EvalError::BudgetExhausted { resource, report } = error else {
            panic!("expected a budget refusal, got {error:?}");
        };
        assert_eq!(resource, BudgetResource::JoinSteps);
        assert_eq!(
            report.join_steps(),
            MAX_JOIN_STEPS + 1,
            "one task, so the report is exactly the observation that passed the ceiling"
        );
        assert!(report.stored_facts() <= n + 1);
    }

    /// The stored-fact ceiling stops a program whose least model is simply too large, and
    /// reports the count that would have resulted.
    #[test]
    fn the_stored_fact_ceiling_is_a_distinguishable_error() {
        // pair(?x, ?y) :- src(?x, ?c), src(?y, ?c). n^2 derived facts.
        let src = "https://example.org/src";
        let pair = "https://example.org/pair";
        let rules = vec![DlClause::datalog(
            atom("?x", pair, "?y"),
            vec![atom("?x", src, "?c"), atom("?y", src, "?c")],
        )];
        let n = 400usize; // 160_000 derived facts > MAX_STORED_FACTS
        let mut edb = RelationStore::new();
        for i in 0..n {
            edb.insert(
                &surface(&format!("https://example.org/n{i}")),
                &surface(src),
                &surface("https://example.org/hub"),
                RelationStore::DEFAULT_GRAPH,
            );
        }
        let exe = compile(rules).expect("the fixture compiles");
        let error = evaluate(&exe, edb).expect_err("the stored-fact ceiling must be passed");
        let EvalError::BudgetExhausted { resource, report } = error else {
            panic!("expected a budget refusal, got {error:?}");
        };
        assert_eq!(resource, BudgetResource::StoredFacts);
        assert_eq!(
            report.stored_facts(),
            n + n * n,
            "the report is the count the round would have produced"
        );
        assert!(report.join_steps() <= MAX_JOIN_STEPS);
    }

    /// The term-arena ceiling stops a legal-sized fact set whose TERMS are enormous, and
    /// reports the byte count that tripped it.
    #[test]
    fn the_term_arena_ceiling_is_a_distinguishable_error() {
        let rules = vec![DlClause::datalog(
            atom("?s", Q, "?o"),
            vec![atom("?s", P, "?o")],
        )];
        // 24 subjects of 1 MiB each: 24 facts, ~25 MiB of term surfaces.
        let mut edb = RelationStore::new();
        let huge = "x".repeat(1 << 20);
        for i in 0..24usize {
            edb.insert(
                &format!("<{huge}{i}>"),
                &surface(P),
                &surface("https://example.org/o"),
                RelationStore::DEFAULT_GRAPH,
            );
        }
        assert!(edb.row_count() < MAX_STORED_FACTS);
        let exe = compile(rules).expect("the fixture compiles");
        let error = evaluate(&exe, edb).expect_err("the term-arena ceiling must be passed");
        let EvalError::BudgetExhausted { resource, report } = error else {
            panic!("expected a budget refusal, got {error:?}");
        };
        assert_eq!(resource, BudgetResource::TermArenaBytes);
        assert!(report.term_arena_bytes() > MAX_TERM_ARENA_BYTES);
        assert_eq!(report.stored_facts(), 24);
    }

    /// A run that stays inside every ceiling reports its consumption accurately, and that
    /// consumption is itself deterministic.
    #[test]
    fn a_completed_run_reports_its_consumption() {
        let workload = synth_corpus::transitive_closure(6);
        let first = run(workload.rules.clone(), workload.edb());
        let second = run(workload.rules.clone(), workload.edb());
        assert_eq!(first.budget(), second.budget());
        assert_eq!(
            first.budget().stored_facts(),
            first.facts().row_count(),
            "the report tracks the store it describes"
        );
        assert_eq!(
            first.budget().term_arena_bytes(),
            first.facts().term_bytes()
        );
        assert!(first.budget().join_steps() > 0);
        assert!(first.budget().join_steps() <= MAX_JOIN_STEPS);
    }

    /// The budget refusal is a `std::error::Error` with a message naming the resource and
    /// both numbers — a diagnostic, not an opaque marker.
    #[test]
    fn budget_and_stratification_errors_render() {
        let error = EvalError::BudgetExhausted {
            resource: BudgetResource::JoinSteps,
            report: BudgetReport {
                join_steps: MAX_JOIN_STEPS + 1,
                stored_facts: 3,
                term_arena_bytes: 4,
                term_generating_rounds: 0,
                term_generating_round_limit: TERM_GENERATING_HORIZON_FLOOR,
            },
        };
        let rendered = error.to_string();
        assert!(rendered.contains("join steps"), "{rendered}");
        assert!(
            rendered.contains(&(MAX_JOIN_STEPS + 1).to_string()),
            "{rendered}"
        );
        let _: &dyn std::error::Error = &error;
    }

    // ── Determinism ────────────────────────────────────────────────────────────

    /// Repeated evaluation of the same program over the same facts is byte-identical.
    ///
    /// This proves reproducibility, NOT correctness — the analytic-golden test above is
    /// the correctness oracle. Both are needed: a systematically wrong evaluator is
    /// perfectly reproducible.
    #[test]
    fn evaluation_is_reproducible() {
        let workload = synth_corpus::same_generation(2);
        let reference = run(workload.rules.clone(), workload.edb());
        for _ in 0..8 {
            let again = run(workload.rules.clone(), workload.edb());
            assert_eq!(
                again.facts().facts_sorted(),
                reference.facts().facts_sorted()
            );
            assert_eq!(again.derivations(), reference.derivations());
            assert_eq!(again.budget(), reference.budget());
        }
    }

    /// Permuting the RULE order within a stratum cannot move the answer: the winner is
    /// decided by the provenance order, not by which rule fired first.
    #[test]
    fn rule_order_within_a_stratum_does_not_move_the_answer() {
        let base = "https://example.org/base";
        let rules: Vec<DlClause> = (0..4)
            .map(|i| {
                DlClause::datalog(
                    atom("?s", R, "?o"),
                    vec![atom("?s", &format!("{base}{i}"), "?o")],
                )
            })
            .collect();
        let edb = store_of(&[
            ("a", &format!("{base}0"), "b"),
            ("a", &format!("{base}1"), "b"),
            ("c", &format!("{base}2"), "d"),
            ("e", &format!("{base}3"), "f"),
        ]);
        let reference = run(rules.clone(), edb);
        for seed in 0..8u64 {
            let permuted = permute(&rules, seed);
            let edb = store_of(&[
                ("a", &format!("{base}0"), "b"),
                ("a", &format!("{base}1"), "b"),
                ("c", &format!("{base}2"), "d"),
                ("e", &format!("{base}3"), "f"),
            ]);
            let again = run(permuted, edb);
            assert_eq!(
                again.facts().facts_sorted(),
                reference.facts().facts_sorted(),
                "seed {seed}"
            );
            // Provenance still names the AUTHORED rule index, which moves with the
            // permutation, so compare only the facts and the source facts.
            assert_eq!(
                again
                    .derivations()
                    .iter()
                    .map(|d| (d.fact().clone(), d.sources().to_vec()))
                    .collect::<Vec<_>>(),
                reference
                    .derivations()
                    .iter()
                    .map(|d| (d.fact().clone(), d.sources().to_vec()))
                    .collect::<Vec<_>>(),
                "seed {seed}"
            );
        }
    }

    // ── Focused kernel behaviour ────────────────────────────────────────────────

    /// The delta span is a range test, and its three scan modes partition the rows.
    #[test]
    fn the_delta_span_partitions_the_rows() {
        let delta = Delta { lo: 2, hi: 5 };
        for index in 0..7usize {
            let row = RowId::from_index(index);
            assert_eq!(delta.contains(row), (2..5).contains(&index));
            assert!(keep_row::<SCAN_FULL>(delta, row));
            assert_eq!(keep_row::<SCAN_DELTA>(delta, row), delta.contains(row));
            assert_eq!(keep_row::<SCAN_OLD_ONLY>(delta, row), !delta.contains(row));
            assert_ne!(
                keep_row::<SCAN_DELTA>(delta, row),
                keep_row::<SCAN_OLD_ONLY>(delta, row)
            );
        }
        assert_eq!(Delta::all(4), Delta { lo: 0, hi: 4 });
    }

    /// The scan-mode selector is the semi-naive position decomposition.
    #[test]
    fn scan_for_selects_the_position_decomposition() {
        assert_eq!(scan_for(0, 1), Scan::Full);
        assert_eq!(scan_for(1, 1), Scan::Delta);
        assert_eq!(scan_for(2, 1), Scan::OldOnly);
        assert!(keep_row_for_scan(
            Scan::Full,
            Delta { lo: 0, hi: 1 },
            RowId::from_index(9)
        ));
        assert!(!keep_row_for_scan(
            Scan::Delta,
            Delta { lo: 0, hi: 1 },
            RowId::from_index(9)
        ));
    }

    /// A repeated variable in one atom is an equality filter: only the rows whose two
    /// columns agree survive.
    #[test]
    fn a_repeated_variable_filters_to_the_diagonal() {
        let rules = vec![DlClause::datalog(
            atom("?s", R, "?s"),
            vec![atom("?s", P, "?s")],
        )];
        let edb = store_of(&[("a", P, "a"), ("a", P, "b"), ("b", P, "b")]);
        let evaluation = run(rules, edb);
        assert_eq!(
            relation(&evaluation, R),
            [(surface("a"), surface("a")), (surface("b"), surface("b"))]
                .into_iter()
                .collect()
        );
    }

    /// A constant body position that the store has never interned matches nothing, and
    /// that is a normal empty answer rather than an error.
    #[test]
    fn an_unknown_body_constant_matches_nothing() {
        let rules = vec![DlClause::datalog(
            atom("?s", R, "?s"),
            vec![ClauseAtom::positive(
                v("?s"),
                P,
                iri("https://example.org/absent"),
            )],
        )];
        let edb = store_of(&[("a", P, "b")]);
        let evaluation = run(rules, edb);
        assert_eq!(evaluation.derivations(), []);
    }

    /// A fully ground body atom is a membership probe.
    #[test]
    fn a_ground_body_atom_is_a_membership_probe() {
        let present = DlClause::datalog(
            ClauseAtom::positive(
                iri("https://example.org/yes"),
                R,
                iri("https://example.org/yes"),
            ),
            vec![ClauseAtom::positive(
                iri("https://example.org/a"),
                P,
                iri("https://example.org/b"),
            )],
        );
        let edb = store_of(&[("https://example.org/a", P, "https://example.org/b")]);
        let evaluation = run(vec![present.clone()], edb);
        assert_eq!(evaluation.derivations().len(), 1);

        let evaluation = run(vec![present], store_of(&[("x", P, "y")]));
        assert_eq!(evaluation.derivations(), []);
    }

    /// An empty program over a seeded store is the store itself.
    #[test]
    fn an_empty_program_derives_nothing() {
        let edb = store_of(&[("a", P, "b")]);
        let evaluation = run(Vec::new(), edb);
        assert_eq!(evaluation.derivations(), []);
        assert_eq!(evaluation.facts().row_count(), 1);
        assert_eq!(evaluation.budget().join_steps(), 0);
    }

    /// `into_facts` hands the saturated store back to the caller.
    #[test]
    fn into_facts_yields_the_saturated_store() {
        let rules = vec![DlClause::datalog(
            atom("?s", Q, "?o"),
            vec![atom("?s", P, "?o")],
        )];
        let evaluation = run(rules, store_of(&[("a", P, "b")]));
        let facts = evaluation.into_facts();
        assert!(facts.contains(
            &surface("a"),
            &surface(Q),
            &surface("b"),
            RelationStore::DEFAULT_GRAPH
        ));
    }
}
