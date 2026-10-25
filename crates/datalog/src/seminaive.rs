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
//! # Static triple or quad atoms, with the predicate as data
//!
//! A quad atom is `triple(?s, ?p, ?o, ?g)`; a triple atom stores only its first three
//! positions. The predicate and, for quads, graph choose the store
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
//!   solutions by selecting exactly the rows its [`crate::plan::IndexChoice`] admits;
//! - the **leapfrog triejoin**: for a planner-certified cyclic component, a multiway
//!   intersection descends the component's variables through the same galloping
//!   positions as [`crate::cursor::ValueCursor`], so no binary intermediate relation is
//!   ever materialised.
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
//! Guard-free bodies with certified independent factors use the same physical kernels
//! once per factor in Full mode. Borrowed mode tables classify a factor tuple New when
//! any premise is in delta. A first-new-FACTOR decomposition (earlier factors Old,
//! anchor New, later Full) partitions these tuples without constructing an unused
//! Cartesian body relation. Each head projects its required bindings; exact witness
//! frontiers retain the canonical proof under globally masked heights and sums.
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
//! # Limits and exported evidence are caller parameters
//!
//! The stored-fact limit and the join-step limit are the caller's, on [`EvalOptions`]
//! ([`EvalOptions::with_max_stored_facts`], [`EvalOptions::with_max_join_steps`]).
//! Unstated fact, join and arena settings impose no logical ceiling on any target;
//! physical storage admission still applies. Exceeding a caller-selected limit returns
//! [`EvalError::BudgetExhausted`] carrying an accurate [`BudgetReport`] — never a panic,
//! never a truncated answer presented as complete. [`EvalOptions`] also carries the two
//! limits on term generation: term-generating rounds and generated terms.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

mod atom;
mod commit;
mod cycle;
mod evidence;
mod factors;
mod guards;
mod join;
mod maintained;
mod negation;
mod negative;
mod ordered;
mod probe;
mod round;
mod rule;
mod session;
mod stratum;
pub use evidence::NegationScope;
pub use maintained::{
    DeltaResult, Maintained, MaintenanceError, ModelDelta, PreparationPause, PreparationProgress,
    RelationProgress,
};
pub use probe::{HeadProbe, HeadWitness, ProbeError};
pub use session::{
    ContinuationBinding, EvaluationSession, PartialModel, ResumeProgress, SessionContext,
    SessionError, StratumStatus,
};
use std::fmt;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

#[cfg(test)]
use rayon::prelude::*;

use crate::clause::{ClauseAtom, ClauseTerm, DlClause, HeadForm};
use crate::cursor::LendingIterator;
use crate::dictionary::{
    DictionaryError, DictionaryTerm, DictionaryValue, TermDictionary, TermIdentity,
};
use crate::guard::{Guard, GuardCall, GuardEvaluator, GuardSite, Negation, NoGuards};
use crate::id::{RowId, TermId};
use crate::paths::shortest_path;
use purrdf_core::{SmallVec, smallvec};
use purrdf_lex::allocation::StorageError;

pub use crate::plan::Executable;
use crate::plan::{
    AtomOperator, AtomShape, CyclicPlan, JoinGroup, POSITION_OBJECT, POSITION_SUBJECT, Parsed,
    PositionPlan, RulePlan, predicate_symbol,
};
use crate::stop::{StopSignal, is_stopped};
use crate::store::{
    Arity, Bound, DeltaRun, PartitionRef, RelationStore, ShapeFact, StoreShape, TermInterner,
};

// ── Budgets ─────────────────────────────────────────────────────────────────────

/// Whether a completed evaluation retains rendered derivation records.
/// Internal row support and proof preference do not depend on this choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DerivationMode {
    /// Return the least model without allocating exported derivation records.
    #[default]
    ModelOnly,
    /// Retain the canonical derivation of every derived fact.
    Record,
}

/// The default limit on TERM-GENERATING rounds ([`EvalOptions`]).
///
/// Chosen by measurement: a rule counting a value up with no bound — the cheapest
/// program that never stops generating, one new term per round — reaches this many rounds
/// over a one-triple graph in well under a second, and twice as many in about three, so
/// the default refuses a runaway counter promptly while admitting any program that needs
/// fewer than sixteen thousand generating rounds. A counter stepping to 10,000 needs
/// 10,000 and completes.
pub const DEFAULT_MAX_TERM_GENERATING_ROUNDS: u64 = 16_384;

/// The floor of the default GENERATED-TERM budget ([`EvalOptions`]).
pub const GENERATED_TERM_BUDGET_FLOOR: u64 = 65_536;

/// The generated terms the default budget grants per distinct term of the seeded store
/// ([`EvalOptions`]).
pub const GENERATED_TERMS_PER_INPUT_TERM: u64 = 4;

/// The caller's evaluation governors: the STORED-FACT and JOIN-STEP limits, the limit on
/// term-generating ROUNDS and the budget of GENERATED TERMS.
///
/// # The stored-fact and join-step limits
///
/// The stored-fact limit bounds the facts (seeded plus derived) one evaluation's store may
/// hold; the join-step limit bounds the candidate solutions it may enumerate. A "join
/// step" is one partial or complete solution appended by a body-atom extension — the unit
/// that actually grows without bound when a rule set is accidentally Cartesian. It is
/// deliberately NOT "one committed derivation": committed derivations are already bounded
/// by the stored-fact limit, so counting them would make this limit redundant, while a
/// blow-up that enumerates millions of candidates and commits three facts would slip past
/// unseen.
///
/// Both are optional caller parameters ([`Self::with_max_stored_facts`],
/// [`Self::with_max_join_steps`]). Unstated capacity settings impose no logical
/// ceiling on any target. Physical storage admission still applies. The same
/// rule applies to retained term surfaces ([`Self::with_max_term_arena_bytes`]).
/// `u64::MAX` denotes an unlimited effective setting; counter overflow remains
/// a typed refusal rather than wrapping or accepting a truncated model.
///
/// What keeps a parameter from making the answer the caller's choice is that it can only
/// REFUSE. A run inside both limits returns the program's least model — the same model
/// under every limit that admits it, on every target — and a run past one returns no model
/// at all ([`EvalError::BudgetExhausted`]), never a truncated one. Every caller passing
/// the same options gets the same answer or the same refusal. The join-step count is a
/// property of the evaluator's plan as well as of the program, so a limit sized tightly
/// against one release's count can refuse under another's; a limit with headroom cannot
/// change a completed answer under either. The limits in force are part of the result's
/// identity: [`contract_hash_with`](crate::cache::contract_hash_with) folds their
/// effective values into every program's contract hash.
///
/// # The term limits
///
/// The stored-fact, join-step and arena limits bound what a run HOLDS and ENUMERATES;
/// none bounds how long a run that holds little can keep going. A guard-free program
/// cannot run away: every term it can commit is a body binding or one of its own finitely
/// many constants, so its round count is bounded by its fact count. A guard can compute a
/// NEW term each round — `?n + 1`, `CONCAT(?s, "x")`, a triple term nesting its own match,
/// a fresh blank node — and a rule that feeds such a term back into its own body derives
/// more facts every round, forever. Two shapes of that run away, and each has its limit:
///
/// * A **round** is term-generating when it commits a term a guard computed and the store
///   had never interned. A program that keeps generating a few terms per round — a
///   counter with no bound — is stopped after at most
///   [`max_term_generating_rounds`](Self::max_term_generating_rounds) such rounds,
///   [`DEFAULT_MAX_TERM_GENERATING_ROUNDS`] unless the caller states another.
/// * The **generated terms** are the terms the evaluation added to the store beyond the
///   `N` distinct terms it was seeded with. A program whose terms multiply every round —
///   each term begetting two — reaches any round limit only after exhausting memory, so
///   it is stopped once more than [`generated_term_budget`](Self::generated_term_budget)
///   terms have been added: `max(GENERATED_TERM_BUDGET_FLOOR, GENERATED_TERMS_PER_INPUT_TERM
///   × N)` (65,536 and 4) unless the caller states another. The budget grows with the
///   input, since a program whose new terms its data bounds adds a few per input term.
///
/// A round that commits only terms already interned is never counted, and adds no term:
/// a value-preserving recursion (a transitive closure, a label propagated down a chain)
/// is bounded by the stored-fact limit and runs as many rounds as it needs.
///
/// Whether a guarded program terminates is undecidable, so no limit PROVES divergence,
/// and a refusal says only which limit a run passed and by how much — a counter stepping
/// `?n + 1` up to `FILTER (?n < N)` terminates after `N` term-generating rounds for every
/// `N`, and a caller who knows a larger bound states it. The limits can only REFUSE,
/// never truncate — a refused run returns no model, exactly as every other limit does —
/// and they are part of the result's identity:
/// [`contract_hash_with`](crate::cache::contract_hash_with) folds both into a guarded
/// program's contract hash, so two runs under different limits never claim the same
/// calculus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EvalOptions {
    /// The caller's stored-fact limit, or `None` for no logical ceiling.
    stored_facts: Option<u64>,
    /// The caller's join-step limit, or `None` for no logical ceiling.
    join_steps: Option<u64>,
    /// The caller's term-generating round limit, or `None` for the default.
    term_generating_rounds: Option<u64>,
    /// The caller's generated-term budget, or `None` for the input-derived default.
    generated_terms: Option<u64>,
    /// Interned surface bytes, or no logical ceiling when unstated.
    term_arena_bytes: Option<u64>,
    /// Whether to retain exported derivations.
    derivation_mode: DerivationMode,
    /// The per-call rule scheduling policy.
    execution: RoundExecution,
}

impl EvalOptions {
    /// Select exported derivation recording. The default returns the model only.
    #[must_use]
    pub fn with_derivation_mode(mut self, mode: DerivationMode) -> Self {
        self.derivation_mode = mode;
        self
    }

    /// The exported evidence requested for this run.
    #[must_use]
    pub fn derivation_mode(&self) -> DerivationMode {
        self.derivation_mode
    }

    /// Select rule scheduling for this call without changing the global Rayon pool.
    #[must_use]
    pub fn with_round_execution(mut self, execution: RoundExecution) -> Self {
        self.execution = execution;
        self
    }

    /// The per-call rule scheduling policy.
    #[must_use]
    pub fn round_execution(&self) -> RoundExecution {
        self.execution
    }

    /// Permit exactly `bytes` interned surface bytes. `u64::MAX` removes the
    /// logical ceiling; actual storage admission can still refuse allocation.
    #[must_use]
    pub fn with_max_term_arena_bytes(mut self, bytes: u64) -> Self {
        self.term_arena_bytes = Some(bytes);
        self
    }

    /// Remove the logical term-arena ceiling while retaining storage admission.
    #[must_use]
    pub fn without_term_arena_limit(self) -> Self {
        self.with_max_term_arena_bytes(u64::MAX)
    }

    /// The effective surface-byte ceiling, or `u64::MAX` when unlimited.
    #[must_use]
    pub fn max_term_arena_bytes(&self) -> u64 {
        self.term_arena_bytes.unwrap_or(u64::MAX)
    }

    /// The caller's surface-byte setting: `None` means unstated;
    /// `Some(u64::MAX)` is explicitly unlimited.
    #[must_use]
    pub fn stated_max_term_arena_bytes(&self) -> Option<u64> {
        self.term_arena_bytes
    }

    /// Permit a store of exactly `facts` facts, seeded plus derived; one more is refused
    /// with [`EvalError::BudgetExhausted`] naming [`BudgetResource::StoredFacts`].
    #[must_use]
    pub fn with_max_stored_facts(mut self, facts: u64) -> Self {
        self.stored_facts = Some(facts);
        self
    }

    /// Permit exactly `steps` enumerated candidate solutions; one more is refused with
    /// [`EvalError::BudgetExhausted`] naming [`BudgetResource::JoinSteps`].
    #[must_use]
    pub fn with_max_join_steps(mut self, steps: u64) -> Self {
        self.join_steps = Some(steps);
        self
    }

    /// Remove the logical stored-fact ceiling while retaining storage admission.
    #[must_use]
    pub fn without_stored_fact_limit(self) -> Self {
        self.with_max_stored_facts(u64::MAX)
    }

    /// Remove the logical join-step ceiling. Counter overflow still refuses.
    #[must_use]
    pub fn without_join_step_limit(self) -> Self {
        self.with_max_join_steps(u64::MAX)
    }

    /// The effective stored-fact ceiling, or `u64::MAX` when unlimited.
    #[must_use]
    pub fn max_stored_facts(&self) -> u64 {
        self.stored_facts.unwrap_or(u64::MAX)
    }

    /// The caller's stored-fact setting: `None` means unstated;
    /// `Some(u64::MAX)` is explicitly unlimited.
    #[must_use]
    pub fn stated_max_stored_facts(&self) -> Option<u64> {
        self.stored_facts
    }

    /// The effective join-step ceiling, or `u64::MAX` when unlimited.
    #[must_use]
    pub fn max_join_steps(&self) -> u64 {
        self.join_steps.unwrap_or(u64::MAX)
    }

    /// The caller's join-step setting: `None` means unstated;
    /// `Some(u64::MAX)` is explicitly unlimited.
    #[must_use]
    pub fn stated_max_join_steps(&self) -> Option<u64> {
        self.join_steps
    }

    /// Permit exactly `rounds` term-generating rounds; one more is refused with
    /// [`EvalError::TermLimitExceeded`] naming [`BudgetResource::TermGeneratingRounds`].
    #[must_use]
    pub fn with_max_term_generating_rounds(mut self, rounds: u64) -> Self {
        self.term_generating_rounds = Some(rounds);
        self
    }

    /// Permit exactly `terms` generated terms; one more is refused with
    /// [`EvalError::TermLimitExceeded`] naming [`BudgetResource::GeneratedTerms`].
    #[must_use]
    pub fn with_max_generated_terms(mut self, terms: u64) -> Self {
        self.generated_terms = Some(terms);
        self
    }

    /// The term-generating round limit in force.
    #[must_use]
    pub fn max_term_generating_rounds(&self) -> u64 {
        self.term_generating_rounds
            .unwrap_or(DEFAULT_MAX_TERM_GENERATING_ROUNDS)
    }

    /// The caller's term-generating round limit, or `None` for the default.
    #[must_use]
    pub fn stated_max_term_generating_rounds(&self) -> Option<u64> {
        self.term_generating_rounds
    }

    /// The generated-term budget in force over a seeded store of `input_terms` distinct
    /// terms.
    #[must_use]
    pub fn generated_term_budget(&self, input_terms: u64) -> u64 {
        self.generated_terms.unwrap_or_else(|| {
            GENERATED_TERM_BUDGET_FLOOR
                .max(GENERATED_TERMS_PER_INPUT_TERM.saturating_mul(input_terms))
        })
    }

    /// The caller's generated-term budget, or `None` for the input-derived default.
    #[must_use]
    pub fn stated_max_generated_terms(&self) -> Option<u64> {
        self.generated_terms
    }
}

/// The five effective limits. Origins and refusal flags are packed separately
/// in [`BudgetReport`] to keep operational errors inline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Limits {
    /// The join-step limit.
    join_steps: u64,
    /// The stored-fact limit.
    stored_facts: u64,
    /// The term-generating round limit.
    term_generating_rounds: u64,
    /// The generated-term budget.
    generated_terms: u64,
    /// The interned surface-byte ceiling.
    term_arena_bytes: u64,
}

impl purrdf_hash::error::StructuredError for Limits {
    purrdf_hash::error_visit! {
        @record self { join_steps, stored_facts, term_generating_rounds, generated_terms, term_arena_bytes } => "datalog-limits" {
            "join_steps" => Unsigned(u128::from(self.join_steps)), "stored_facts" => Unsigned(u128::from(self.stored_facts)),
            "term_generating_rounds" => Unsigned(u128::from(self.term_generating_rounds)), "generated_terms" => Unsigned(u128::from(self.generated_terms)),
            "term_arena_bytes" => ByteCount(u128::from(self.term_arena_bytes)),
        }
    }
}

/// [`BudgetReport::flags`]: the join-step limit is the caller's.
const STATED_JOIN_STEPS: u8 = 1;
/// [`BudgetReport::flags`]: the stored-fact limit is the caller's.
const STATED_STORED_FACTS: u8 = 1 << 1;
/// [`BudgetReport::flags`]: the term-generating round limit is the caller's.
const STATED_TERM_GENERATING_ROUNDS: u8 = 1 << 2;
/// [`BudgetReport::flags`]: the generated-term budget is the caller's.
const STATED_GENERATED_TERMS: u8 = 1 << 3;
/// [`BudgetReport::flags`]: the surface-byte ceiling is the caller's.
const STATED_TERM_ARENA_BYTES: u8 = 1 << 4;
/// The first unique stored-fact insertion was refused before mutation.
const REFUSED_STORED_FACT: u8 = 1 << 7;

impl Limits {
    /// The limits `options` states, over a seeded store of `input_terms` distinct terms.
    fn of(options: &EvalOptions, input_terms: u64) -> Self {
        Self {
            join_steps: options.max_join_steps(),
            stored_facts: options.max_stored_facts(),
            term_generating_rounds: options.max_term_generating_rounds(),
            generated_terms: options.generated_term_budget(input_terms),
            term_arena_bytes: options.max_term_arena_bytes(),
        }
    }
}

/// What an evaluation actually consumed, against the limits it ran under.
///
/// Returned on success ([`Evaluation::budget`]) and on failure
/// ([`EvalError::BudgetExhausted`]) alike. Stored facts count actual admissions;
/// [`Self::observed_stored_facts`] includes the first refused insertion. Each
/// [`BudgetResource::observed`] names the observation proving the refusal.
///
/// Every measurement is a deterministic function of the input: the same program over the
/// same facts reports the same numbers on every target. The limits it carries are the ones
/// in force ([`EvalOptions`]); unstated capacity settings are unlimited on every target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetReport {
    /// Candidate solutions enumerated, against [`Self::join_step_limit`].
    join_steps: u64,
    /// Facts held by the store, against [`Self::stored_fact_limit`].
    stored_facts: usize,
    /// Caller-stated limits and the first refused fact admission.
    flags: u8,
    /// Interned term surface bytes, against [`Self::term_arena_limit`].
    term_arena_bytes: usize,
    /// Term-generating rounds run, against [`Self::term_generating_round_limit`].
    term_generating_rounds: u64,
    /// Terms added to the store beyond the seeded ones, against
    /// [`Self::generated_term_budget`].
    generated_terms: u64,
    /// The distinct terms of the seeded store.
    input_terms: u64,
    /// The limits the run was governed by ([`EvalOptions`]).
    limits: Limits,
}

impl purrdf_hash::error::StructuredError for BudgetReport {
    purrdf_hash::error_visit! {
        @record self { join_steps, stored_facts, flags, term_arena_bytes, term_generating_rounds, generated_terms, input_terms, limits } => "datalog-budget-report" {
            "join_steps" => Unsigned(u128::from(self.join_steps)), "stored_facts" => Unsigned(self.stored_facts as u128),
            "stored_fact_refused" => Boolean(self.flags & REFUSED_STORED_FACT != 0),
            "stated_limits" => Unsigned(u128::from(self.flags & !REFUSED_STORED_FACT)),
            "term_arena_bytes" => ByteCount(self.term_arena_bytes as u128), "term_generating_rounds" => Unsigned(u128::from(self.term_generating_rounds)),
            "generated_terms" => Unsigned(u128::from(self.generated_terms)), "input_terms" => Unsigned(u128::from(self.input_terms)),
            "limits" => Record(&self.limits)
        }
    }
}

impl Default for BudgetReport {
    /// The zero measurement, governed by the default limits.
    fn default() -> Self {
        Self::new(0, 0, 0)
    }
}

impl BudgetReport {
    /// Pack limit origins alongside refusal state without padding each limit record.
    fn stated_limits(options: &EvalOptions) -> u8 {
        let bit = |stated: Option<u64>, flag: u8| if stated.is_some() { flag } else { 0 };
        bit(options.stated_max_join_steps(), STATED_JOIN_STEPS)
            | bit(options.stated_max_stored_facts(), STATED_STORED_FACTS)
            | bit(
                options.stated_max_term_generating_rounds(),
                STATED_TERM_GENERATING_ROUNDS,
            )
            | bit(options.stated_max_generated_terms(), STATED_GENERATED_TERMS)
            | bit(
                options.stated_max_term_arena_bytes(),
                STATED_TERM_ARENA_BYTES,
            )
    }

    const fn is_stated(self, flag: u8) -> bool {
        self.flags & flag != 0
    }
    /// A report over three already-measured coordinates.
    ///
    /// This crate's own evaluator fills the fields directly; the constructor exists so
    /// that one type can say "what did this evaluation consume" for a run this crate did
    /// not perform — `purrdf-entail`'s `Simple` lane, which copies a dataset and evaluates
    /// no program at all, reports the zero measurement through here rather than growing a
    /// second budget type for the case. It takes MEASUREMENTS, never limits: the report it
    /// builds carries the default limits ([`EvalOptions::default`]), and
    /// [`Self::governed_by`] restates them for a run that was governed by others.
    ///
    /// The coordinates mean exactly what they mean for [`evaluate`]: candidate solutions
    /// enumerated, facts held when evaluation stopped, and interned term surface bytes. An
    /// engine that reports numbers under a different definition is misreporting, not
    /// extending.
    pub fn new(join_steps: u64, stored_facts: usize, term_arena_bytes: usize) -> Self {
        Self {
            join_steps,
            stored_facts,
            flags: Self::stated_limits(&EvalOptions::default()),
            term_arena_bytes,
            term_generating_rounds: 0,
            generated_terms: 0,
            input_terms: 0,
            limits: Limits::of(&EvalOptions::default(), 0),
        }
    }

    /// This report, stating the stored-fact and join-step limits of `options` as the ones
    /// the run was governed by. The measurements are unchanged.
    #[must_use]
    pub fn governed_by(self, options: &EvalOptions) -> Self {
        let capacity = Limits::of(options, 0);
        let capacity_flags = STATED_JOIN_STEPS | STATED_STORED_FACTS | STATED_TERM_ARENA_BYTES;
        let kept = self.flags & !capacity_flags;
        Self {
            flags: kept | (Self::stated_limits(options) & capacity_flags),
            limits: Limits {
                join_steps: capacity.join_steps,
                stored_facts: capacity.stored_facts,
                term_arena_bytes: capacity.term_arena_bytes,
                ..self.limits
            },
            ..self
        }
    }

    /// Join, guard, negative-probe and head candidate admissions across all rounds,
    /// including one refused reservation when a join-step ceiling is exceeded.
    /// Saturates at u64::MAX; an unrepresentable refusal observation still returns
    /// the typed JoinSteps exhaustion rather than accepting a truncated result.
    pub fn join_steps(self) -> u64 {
        self.join_steps
    }

    /// The join-step limit the run was governed by ([`EvalOptions::max_join_steps`]).
    pub fn join_step_limit(self) -> u64 {
        self.limits.join_steps
    }

    /// Whether the caller explicitly selected the join-step setting.
    pub fn join_step_limit_stated(self) -> bool {
        self.is_stated(STATED_JOIN_STEPS)
    }

    /// The stored-fact limit the run was governed by ([`EvalOptions::max_stored_facts`]).
    pub fn stored_fact_limit(self) -> u64 {
        self.limits.stored_facts
    }

    /// Whether the caller explicitly selected the stored-fact setting.
    pub fn stored_fact_limit_stated(self) -> bool {
        self.is_stated(STATED_STORED_FACTS)
    }

    /// Retained row slots when evaluation stopped, including the captured base.
    /// Ordinary dense logs retain vacant append slots after removal. Maintained
    /// live-row logs reclaim those entries; logical append extent is separate.
    /// Live model cardinality is available from [`RelationStore::row_count`].
    pub fn stored_facts(self) -> usize {
        self.stored_facts
    }

    /// The first refused insertion, or the retained count if none was refused.
    /// Unlike a round projection, a refusal is exactly one past the admitted count.
    pub fn observed_stored_facts(self) -> usize {
        self.stored_facts
            .saturating_add(usize::from(self.flags & REFUSED_STORED_FACT != 0))
    }

    /// Admit one absent fact before insertion, retaining the first refusal separately
    /// from facts actually stored. Shared by ordinary, ordered and chase commits.
    pub(crate) fn admit_fact(self) -> Result<(), Self> {
        if self.stored_facts as u64 >= self.limits.stored_facts {
            Err(Self {
                flags: self.flags | REFUSED_STORED_FACT,
                ..self
            })
        } else {
            Ok(())
        }
    }

    /// The effective interned surface-byte ceiling.
    pub fn term_arena_limit(self) -> u64 {
        self.limits.term_arena_bytes
    }

    /// Whether the caller stated the interned surface-byte ceiling.
    pub fn term_arena_limit_stated(self) -> bool {
        self.is_stated(STATED_TERM_ARENA_BYTES)
    }

    /// Interned term surface bytes held by the store when evaluation stopped.
    pub fn term_arena_bytes(self) -> usize {
        self.term_arena_bytes
    }

    /// Completed term-generating rounds ([`EvalOptions`]). Always zero for a guard-free
    /// program, which has no guard to compute a term.
    pub fn term_generating_rounds(self) -> u64 {
        self.term_generating_rounds
    }

    /// The term-generating round limit the run was governed by ([`EvalOptions`]).
    pub fn term_generating_round_limit(self) -> u64 {
        self.limits.term_generating_rounds
    }

    /// Whether the term-generating round limit was the caller's rather than
    /// [`DEFAULT_MAX_TERM_GENERATING_ROUNDS`].
    pub fn term_generating_round_limit_stated(self) -> bool {
        self.is_stated(STATED_TERM_GENERATING_ROUNDS)
    }

    /// Terms the evaluation added to the store beyond the seeded ones ([`EvalOptions`]).
    pub fn generated_terms(self) -> u64 {
        self.generated_terms
    }

    /// The generated-term budget the run was governed by ([`EvalOptions`]).
    pub fn generated_term_budget(self) -> u64 {
        self.limits.generated_terms
    }

    /// Whether the generated-term budget was the caller's rather than the input-derived
    /// default.
    pub fn generated_term_budget_stated(self) -> bool {
        self.is_stated(STATED_GENERATED_TERMS)
    }

    /// The distinct terms of the seeded store.
    pub fn input_terms(self) -> u64 {
        self.input_terms
    }
}

/// Which limit an exhausted evaluation passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum BudgetResource {
    /// The join-step limit ([`EvalOptions::with_max_join_steps`]) — too many candidate
    /// solutions enumerated.
    JoinSteps,
    /// The stored-fact limit ([`EvalOptions::with_max_stored_facts`]) — too many facts
    /// seeded or derived.
    StoredFacts,
    /// The caller's term-arena ceiling — too many interned term surface bytes.
    TermArenaBytes,
    /// The term-generating round limit ([`EvalOptions`]) — too many rounds committed a
    /// guard-computed term.
    TermGeneratingRounds,
    /// The generated-term budget ([`EvalOptions`]) — too many terms added to the store.
    GeneratedTerms,
}

impl purrdf_hash::error::StructuredError for BudgetResource {
    purrdf_hash::error_visit! {
        JoinSteps => "datalog-budget-join-steps" {},
        StoredFacts => "datalog-budget-stored-facts" {},
        TermArenaBytes => "datalog-budget-term-arena-bytes" {},
        TermGeneratingRounds => "datalog-budget-term-generating-rounds" {},
        GeneratedTerms => "datalog-budget-generated-terms" {},
    }
}

impl BudgetResource {
    /// The limit this resource is measured against in `report`.
    #[must_use]
    pub fn limit(self, report: BudgetReport) -> u64 {
        match self {
            Self::JoinSteps => report.limits.join_steps,
            Self::StoredFacts => report.limits.stored_facts,
            Self::TermArenaBytes => report.limits.term_arena_bytes,
            Self::TermGeneratingRounds => report.limits.term_generating_rounds,
            Self::GeneratedTerms => report.limits.generated_terms,
        }
    }

    /// The observation that tripped this resource, taken from `report`.
    #[must_use]
    pub fn observed(self, report: BudgetReport) -> u64 {
        match self {
            Self::JoinSteps => report.join_steps,
            Self::StoredFacts => report.observed_stored_facts() as u64,
            Self::TermArenaBytes => report.term_arena_bytes as u64,
            Self::TermGeneratingRounds => report.term_generating_rounds,
            Self::GeneratedTerms => report.generated_terms,
        }
    }

    /// The resource's name, for a diagnostic.
    fn name(self) -> &'static str {
        match self {
            Self::JoinSteps => "join steps",
            Self::StoredFacts => "stored facts",
            Self::TermArenaBytes => "term arena bytes",
            Self::TermGeneratingRounds => "term-generating rounds",
            Self::GeneratedTerms => "generated terms",
        }
    }
}

/// One shared round allowance, with a separate, single refusal observation.
///
/// Reservations precede evaluator-owned candidate expansion. The final reservation
/// records exhaustion without expanding its candidate. Parallel tasks share this
/// ceiling, and their private buffers are folded in authored order before any commit.
#[derive(Debug, Clone)]
struct StepGovernor<'s> {
    /// Remaining successful credits, including the full u64 range.
    allowance: u64,
    /// Candidates enumerated so far.
    consumed: u64,
    /// Actual binary cursor rows, before semi-naive filtering, for complexity checks.
    #[cfg(test)]
    row_visits: u64,
    /// One shared round-wide credit pool.
    pool: Arc<StepPool>,
    /// Borrowed host cancellation; never part of the program's identity.
    stop: Option<&'s dyn StopSignal>,
}

/// Successful credits and refusal are separate so the full-width ceiling cannot
/// lose its final credit or silently truncate on an unrepresentable sentinel.
#[derive(Debug)]
struct StepPool {
    used: AtomicU64,
    refused: AtomicBool,
    stopped: AtomicBool,
}

impl<'s> StepGovernor<'s> {
    /// A governor permitting `allowance` candidates.
    fn new(allowance: u64) -> Self {
        Self {
            allowance,
            consumed: 0,
            #[cfg(test)]
            row_visits: 0,
            pool: Arc::new(StepPool {
                used: AtomicU64::new(0),
                refused: AtomicBool::new(false),
                stopped: AtomicBool::new(false),
            }),
            stop: None,
        }
    }

    /// Attach one borrowed signal to this round and all its rule-local counters.
    fn with_stop(mut self, stop: Option<&'s dyn StopSignal>) -> Self {
        self.stop = stop;
        self
    }

    /// Cancellation is latched separately from an actually refused reservation.
    #[inline]
    fn stopped(&self) -> bool {
        let Some(stop) = self.stop else {
            return false;
        };
        if self.pool.stopped.load(Ordering::Relaxed) {
            return true;
        }
        if stop.stopped() {
            self.pool.stopped.store(true, Ordering::Relaxed);
            return true;
        }
        false
    }

    #[inline]
    fn refused(&self) -> bool {
        self.pool.refused.load(Ordering::Relaxed)
    }

    /// Whether the allowance is spent — the next candidate may NOT be enumerated.
    #[inline]
    fn spent(&self) -> bool {
        self.refused() || self.stopped()
    }

    /// Reserve a candidate; the single refusal sentinel is never expanded.
    #[inline]
    fn charge(&mut self) -> bool {
        if self.stopped() {
            return false;
        }
        let reserved = self
            .pool
            .used
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
                (used < self.allowance).then(|| used + 1)
            });
        if reserved.is_ok() {
            self.consumed = self.consumed.saturating_add(1);
            true
        } else {
            if !self.pool.refused.swap(true, Ordering::Relaxed) {
                self.consumed = self.consumed.saturating_add(1);
            }
            false
        }
    }

    /// Saturated public observation; refusal remains explicit at full width.
    #[cfg(test)]
    fn observed(&self) -> u64 {
        self.pool
            .used
            .load(Ordering::Relaxed)
            .saturating_add(u64::from(self.refused()))
    }

    /// A rule-local counter sharing the round's credit pool.
    fn task(&self) -> Self {
        Self {
            allowance: self.allowance,
            consumed: 0,
            #[cfg(test)]
            row_visits: 0,
            pool: Arc::clone(&self.pool),
            stop: self.stop,
        }
    }
}

// ── Errors ──────────────────────────────────────────────────────────────────────

/// Why an evaluation could not run, or could not run to completion.
///
/// Complete-only entry points return these refusals without a model.
/// [`EvaluationSession`] instead retains resource and stop refusals beside a typed
/// [`PartialModel`]; source and guard failures remain hard execution failures.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EvalError<E = core::convert::Infallible> {
    /// The original failure of the caller-owned dictionary or its pinned source.
    Source(E),
    /// An immutable-base body probe requires the ordered evaluation contract.
    BaseReadRequiresSchedule {
        /// The first rule whose base authority requires the ordered executor.
        rule: usize,
    },
    /// An authored Base body probe has no captured immutable base input.
    MissingBase {
        /// The first rule needing the absent authority.
        rule: usize,
    },
    /// The original logical capacity or physical storage refusal.
    Storage(StorageError),
    /// The original row-evidence account refused or encountered a broken invariant.
    Evidence(purrdf_core::evidence::EvidenceWorkspaceError),
    /// The full rule bundle violates an assessor-only declaration.
    NeverDerive(crate::admission::AdmissionRefusal),
    /// A clause's head is existential, disjunctive or `false`, so it is not a
    /// Datalog rule and this evaluator has no semantics for it.
    ///
    /// The DL-clause IR ([`crate::clause`]) represents all five head forms in one type, so a
    /// consumer of any of them needs no second IR: [`crate::chase`] takes the existential
    /// form, `purrdf-entail`'s OWL-Direct hypertableau case-splits the
    /// disjunctive one (over its own concept-id atoms, through this crate's
    /// [`crate::clause::HeadForm`]), and both are refused by name HERE rather than
    /// parsed away. A semi-naive least-fixpoint evaluator computes the
    /// least model of definite clauses: an existential mints witnesses, a disjunction
    /// has no single least model, and `false` derives nothing while asserting
    /// its body is unsatisfiable. Each is
    /// refused by its OWN name here rather than silently ignored, silently treated as
    /// atomic, or reported under a neighbouring form's name.
    ///
    /// A head conjunction without existential variables shares one body evaluation.
    /// Its derivations retain both the authored clause index and head ordinal.
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
    /// The stored-fact, join-step or term-arena limit in force ([`EvalOptions`])
    /// was passed. The report is accurate at the point evaluation
    /// stopped, and carries the limit that was passed.
    BudgetExhausted {
        /// Which ceiling.
        resource: BudgetResource,
        /// Consumption, and the limits in force, when evaluation stopped.
        report: BudgetReport,
    },
    /// The run passed a term limit ([`EvalOptions`]): the term-generating round limit
    /// ([`BudgetResource::TermGeneratingRounds`]) or the generated-term budget
    /// ([`BudgetResource::GeneratedTerms`]). It is a statement about the limit, never a
    /// verdict that the program diverges — whether it would have terminated is
    /// undecidable — and it names the rules that generated a term in the refused round.
    TermLimitExceeded {
        /// Which limit.
        resource: BudgetResource,
        /// The rules whose derivations committed a guard-computed term in the refused
        /// round, in authored program order. A fixed slice keeps the operational
        /// error compact when embedded by entailment and host errors.
        rules: Box<[usize]>,
        /// Consumption when evaluation stopped, the limit in force included.
        report: BudgetReport,
    },
    /// The caller's [`crate::stop::StopSignal`] fired at an owned work boundary.
    ///
    /// Complete-only entry points return no model. [`EvaluationSession`] retains
    /// certified published facts and can resume with a fresh signal. This remains
    /// distinct from a numeric ceiling: the host stopped the run, rather than the
    /// program and data exhausting an allowance.
    ///
    /// The report is accurate at the point evaluation stopped, so a host can say what the
    /// stopped run had already consumed rather than only that it was stopped.
    Stopped {
        /// Consumption, and the limits in force, when the signal was observed.
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
        message: crate::guard::CallbackDiagnostic,
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
        detail: crate::schedule::ScheduleRefusal,
    },
    /// The caller's [`LayerHooks`](crate::schedule::LayerHooks) reported an error.
    LayerHook {
        /// The layer whose hook failed, or `None` for the end-of-evaluation hook.
        layer: Option<usize>,
        /// The hook's message.
        message: crate::guard::CallbackDiagnostic,
    },
}

impl<E> EvalError<E> {
    /// Transform only the original source failure, preserving every other refusal.
    ///
    /// # Errors
    /// Returns the converter's original failure without rendering or replacing it.
    pub fn try_map_source<F, R>(
        self,
        convert: impl FnOnce(E) -> Result<F, R>,
    ) -> Result<EvalError<F>, R> {
        Ok(match self {
            Self::Source(error) => EvalError::Source(convert(error)?),
            Self::BaseReadRequiresSchedule { rule } => EvalError::BaseReadRequiresSchedule { rule },
            Self::MissingBase { rule } => EvalError::MissingBase { rule },
            Self::Storage(error) => EvalError::Storage(error),
            Self::Evidence(error) => EvalError::Evidence(error),
            Self::NeverDerive(error) => EvalError::NeverDerive(error),
            Self::NonDatalogHead { rule, form } => EvalError::NonDatalogHead { rule, form },
            Self::NonStratifiable {
                head,
                negated,
                cycle,
            } => EvalError::NonStratifiable {
                head,
                negated,
                cycle,
            },
            Self::UnboundHeadVariable { rule, variable } => {
                EvalError::UnboundHeadVariable { rule, variable }
            }
            Self::BudgetExhausted { resource, report } => {
                EvalError::BudgetExhausted { resource, report }
            }
            Self::TermLimitExceeded {
                resource,
                rules,
                report,
            } => EvalError::TermLimitExceeded {
                resource,
                rules,
                report,
            },
            Self::Stopped { report } => EvalError::Stopped { report },
            Self::Guard {
                rule,
                site,
                guard,
                message,
            } => EvalError::Guard {
                rule,
                site,
                guard,
                message,
            },
            Self::ModelReadingGuard { rule } => EvalError::ModelReadingGuard { rule },
            Self::NonStratifiableRules {
                rule,
                depends_on,
                cycle,
            } => EvalError::NonStratifiableRules {
                rule,
                depends_on,
                cycle,
            },
            Self::MalformedSchedule { detail } => EvalError::MalformedSchedule { detail },
            Self::LayerHook { layer, message } => EvalError::LayerHook { layer, message },
        })
    }
}

impl<E: purrdf_hash::error::StructuredError> purrdf_hash::error::StructuredError for EvalError<E> {
    purrdf_hash::error_visit! {
        Source(cause) => "datalog-eval-source" { "cause" => Cause(cause) },
        BaseReadRequiresSchedule { rule } => "datalog-eval-base-requires-schedule" { "rule" => Unsigned(*rule as u128) },
        MissingBase { rule } => "datalog-eval-missing-base" { "rule" => Unsigned(*rule as u128) },
        Storage(cause) => "datalog-eval-storage" { "cause" => Cause(cause) },
        Evidence(cause) => "datalog-eval-evidence" { "cause" => Cause(cause) },
        NeverDerive(cause) => "datalog-eval-never-derive" { "cause" => Cause(cause) },
        NonDatalogHead { rule, form } => "datalog-eval-non-datalog-head" { "rule" => Unsigned(*rule as u128), "form" => Record(form) },
        NonStratifiable { head, negated, cycle } => "datalog-eval-non-stratifiable" { "head" => Text(head), "negated" => Text(negated), "cycle" => List(cycle.iter().map(|value| purrdf_hash::error::ErrorValue::Text(value))) },
        UnboundHeadVariable { rule, variable } => "datalog-eval-unbound-head-variable" { "rule" => Unsigned(*rule as u128), "variable" => Text(variable) },
        BudgetExhausted { resource, report } => "datalog-eval-budget-exhausted" { "resource" => Record(resource), "report" => Record(report) },
        TermLimitExceeded { resource, rules, report } => "datalog-eval-term-limit-exceeded" { "resource" => Record(resource), "rules" => List(rules.iter().map(|value| purrdf_hash::error::ErrorValue::Unsigned(*value as u128))), "report" => Record(report) },
        Stopped { report } => "datalog-eval-stopped" { "report" => Record(report) },
        Guard { rule, site, guard, message } => "datalog-eval-guard" { "rule" => Unsigned(*rule as u128), "site" => Record(site), "guard" => Text(guard), "message" => Cause(message) },
        ModelReadingGuard { rule } => "datalog-eval-model-reading-guard" { "rule" => Unsigned(*rule as u128) },
        NonStratifiableRules { rule, depends_on, cycle } => "datalog-eval-non-stratifiable-rules" { "rule" => Unsigned(*rule as u128), "depends_on" => Unsigned(*depends_on as u128), "cycle" => List(cycle.iter().map(|value| purrdf_hash::error::ErrorValue::Unsigned(*value as u128))) },
        MalformedSchedule { detail } => "datalog-eval-malformed-schedule" { "detail" => Cause(detail) },
        LayerHook { layer, message } => "datalog-eval-layer-hook" { "layer" => Value(layer.map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Unsigned(value as u128))), "message" => Cause(message) },
    }
}

/// Where a report's stored-fact limit came from, for a diagnostic.
#[must_use]
pub fn stored_fact_limit_origin(report: BudgetReport) -> &'static str {
    if report.is_stated(STATED_STORED_FACTS) {
        "the caller's limit"
    } else {
        "the unlimited setting"
    }
}

/// Where a report's join-step limit came from, for a diagnostic.
#[must_use]
pub fn join_step_limit_origin(report: BudgetReport) -> &'static str {
    if report.is_stated(STATED_JOIN_STEPS) {
        "the caller's limit"
    } else {
        "the unlimited setting"
    }
}

/// Render a passed stored-fact or join-step limit, naming `knob` as what raises it — the
/// one wording every host gives the refusal, each with its own spelling of the knob.
///
/// `None` for any other resource.
#[must_use]
pub fn render_capacity_refusal(
    resource: BudgetResource,
    report: BudgetReport,
    knob: &str,
) -> Option<String> {
    match resource {
        BudgetResource::StoredFacts => Some(format!(
            "evaluation exceeded the stored-fact limit: {} facts observed, {} permitted ({}); \
             raise it with {knob}",
            report.observed_stored_facts(),
            report.limits.stored_facts,
            stored_fact_limit_origin(report),
        )),
        BudgetResource::JoinSteps
            if report.join_steps == u64::MAX && report.limits.join_steps == u64::MAX =>
        {
            Some(format!(
                "evaluation exceeded the full-width join-step limit: a further candidate was \
             refused after {} permitted steps ({}); the observation saturates at {}; \
             reduce the rule work or input because this limit cannot be raised",
                report.limits.join_steps,
                join_step_limit_origin(report),
                report.join_steps,
            ))
        }
        BudgetResource::JoinSteps => Some(format!(
            "evaluation exceeded the join-step limit: {} join steps observed, {} permitted \
             ({}); raise it with {knob}",
            report.join_steps,
            report.limits.join_steps,
            join_step_limit_origin(report),
        )),
        _ => None,
    }
}

/// Render an exact resource refusal with remediation available to the caller.
/// Hosts supply an action from their own entry point rather than an inner API.
#[must_use]
pub fn render_budget_refusal(
    resource: BudgetResource,
    report: BudgetReport,
    remediation: &str,
) -> String {
    format!(
        "evaluation exceeded the {} limit: {} observed, {} permitted; {remediation}",
        resource.name(),
        resource.observed(report),
        resource.limit(report)
    )
}

/// Where a report's term-generating round limit came from, for a diagnostic.
#[must_use]
pub fn term_generating_round_limit_origin(report: BudgetReport) -> String {
    if report.is_stated(STATED_TERM_GENERATING_ROUNDS) {
        "the caller's limit".to_owned()
    } else {
        "the default".to_owned()
    }
}

/// Where a report's generated-term budget came from, for a diagnostic.
#[must_use]
pub fn generated_term_budget_origin(report: BudgetReport) -> String {
    if report.is_stated(STATED_GENERATED_TERMS) {
        "the caller's budget".to_owned()
    } else {
        format!(
            "the default, max({GENERATED_TERM_BUDGET_FLOOR}, {GENERATED_TERMS_PER_INPUT_TERM} per \
             input term)"
        )
    }
}

impl<E: fmt::Display> fmt::Display for EvalError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => error.fmt(f),
            Self::BaseReadRequiresSchedule { rule } => write!(
                f,
                "rule {rule} reads immutable base facts and requires ordered evaluation"
            ),
            Self::MissingBase { rule } => {
                write!(f, "rule {rule} requires an explicit immutable base input")
            }
            Self::Storage(error) => error.fmt(f),
            Self::Evidence(error) => error.fmt(f),
            Self::NeverDerive(refusal) => fmt::Display::fmt(refusal, f),
            Self::NonDatalogHead { rule, form } => write!(
                f,
                "clause {rule} has {} {form} head: the semi-naive evaluator runs Datalog \
                 clauses (an atomic or conjunctive head, no existential) only",
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
            Self::TermLimitExceeded {
                resource,
                rules,
                report,
            } => {
                let rules: Vec<String> = rules.iter().map(ToString::to_string).collect();
                let rules = if rules.is_empty() {
                    String::new()
                } else {
                    format!(
                        "; rule(s) {} generated a term in the last round",
                        rules.join(", ")
                    )
                };
                match resource {
                    BudgetResource::GeneratedTerms => write!(
                        f,
                        "evaluation exceeded the generated-term budget: {} terms were added to \
                         the store beyond its {} input terms, past the budget of {} ({}){rules}; \
                         if the rule set terminates, raise it with \
                         EvalOptions::with_max_generated_terms",
                        report.generated_terms,
                        report.input_terms,
                        report.limits.generated_terms,
                        generated_term_budget_origin(*report),
                    ),
                    _ => write!(
                        f,
                        "evaluation exceeded the term-generating round limit: {} rounds \
                         committed a term the store did not hold, past the limit of {} ({}){rules}; \
                         if the rule set terminates, raise it with \
                         EvalOptions::with_max_term_generating_rounds",
                        report.term_generating_rounds,
                        report.limits.term_generating_rounds,
                        term_generating_round_limit_origin(*report),
                    ),
                }
            }
            Self::BudgetExhausted { resource, report } => {
                let knob = match resource {
                    BudgetResource::StoredFacts => "EvalOptions::with_max_stored_facts",
                    _ => "EvalOptions::with_max_join_steps",
                };
                match render_capacity_refusal(*resource, *report, knob) {
                    Some(rendered) => f.write_str(&rendered),
                    None => f.write_str(&render_budget_refusal(
                        *resource,
                        *report,
                        "raise it with EvalOptions::with_max_term_arena_bytes",
                    )),
                }
            }
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

impl<E: std::error::Error + 'static> std::error::Error for EvalError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source(error) => Some(error),
            Self::Storage(error) => Some(error),
            Self::Evidence(error) => Some(error),
            Self::MalformedSchedule { detail } => Some(detail),
            Self::Guard { message, .. } | Self::LayerHook { message, .. } => Some(message),
            _ => None,
        }
    }
}

purrdf_lex::variant_from!(impl<E> EvalError<E> {
    Storage(StorageError),
    Evidence(purrdf_core::evidence::EvidenceWorkspaceError),
});

impl<E> From<core::convert::Infallible> for EvalError<E> {
    fn from(never: core::convert::Infallible) -> Self {
        match never {}
    }
}

// ── Results ─────────────────────────────────────────────────────────────────────

/// One committed derivation: a fact, the rule that produced it and the body facts it was
/// produced from.
///
/// `sources` is in the rule's AUTHORED body order regardless of the order the planner
/// chose to execute the atoms in, so provenance reads the way the rule is written.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Derivation<const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The derived fact.
    fact: ShapeFact<N>,
    /// The producing rule's index in authored program order.
    rule: usize,
    /// The producing atom's position in the authored head conjunction.
    head: usize,
    /// The matched positive body facts, in authored body order.
    sources: Vec<ShapeFact<N>>,
    /// `1 + max(source proof heights)`; a seeded fact has height 0.
    proof_height: u32,
}

impl<const N: usize> Derivation<N>
where
    Arity<N>: StoreShape<N>,
{
    /// The derived fact.
    pub fn fact(&self) -> &ShapeFact<N> {
        &self.fact
    }

    /// The producing rule's index in authored program order.
    pub fn rule(&self) -> usize {
        self.rule
    }

    /// The producing atom's position in the authored head conjunction.
    pub fn head(&self) -> usize {
        self.head
    }

    /// The matched positive body facts, in authored body order.
    pub fn sources(&self) -> &[ShapeFact<N>] {
        &self.sources
    }

    /// The derivation's proof height: `1 + max(source proof heights)`, seeds being 0.
    pub fn proof_height(&self) -> u32 {
        self.proof_height
    }
}

/// Semantic output counts alongside the planner's physical candidate count.
///
/// For an ordinary completed stratified evaluation, seeded and newly committed
/// fact counts are invariant under join planning and derivation recording.
/// Scheduled retractions can cause another commit of a previously withdrawn fact;
/// that is another commit event. Physical candidates may change between planners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkReport {
    /// Unique seeded facts.
    input_facts: u64,
    /// Successful unique rule insertions.
    committed_facts: u64,
    /// Physical candidate admissions.
    join_steps: u64,
}

impl WorkReport {
    /// Unique facts in the seeded store.
    pub fn input_facts(self) -> u64 {
        self.input_facts
    }
    /// Unique fact insertions performed by rule commits.
    pub fn committed_facts(self) -> u64 {
        self.committed_facts
    }
    /// Physical candidate admissions; divide by committed facts to compare plans.
    pub fn join_steps(self) -> u64 {
        self.join_steps
    }
    /// Physical candidate admissions per committed fact, as an exact numerator
    /// and denominator. No ratio is defined when nothing was committed.
    pub fn candidates_per_committed_fact(self) -> Option<(u64, u64)> {
        (self.committed_facts != 0).then_some((self.join_steps, self.committed_facts))
    }

    /// Version of the physical planner that produced the candidate count.
    pub fn planner_version(self) -> &'static str {
        crate::cache::PLAN_SOLVER_VERSION
    }
}

/// A completed least model, optional derivations and measured consumption.
#[derive(Debug, Clone)]
pub struct Evaluation<D: TermDictionary = TermInterner, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The least model: the seeded EDB plus every derived fact.
    facts: RelationStore<D, N>,
    /// Every derivation, in lexical `(fact, rule, head, sources)` order.
    derivations: Option<Vec<Derivation<N>>>,
    /// What the run consumed, against the limits it ran under.
    budget: BudgetReport,
    /// Stable semantic counts and plan-dependent candidate work.
    work: WorkReport,
}

impl<D: TermDictionary, const N: usize> Evaluation<D, N>
where
    Arity<N>: StoreShape<N>,
{
    /// The least model: the seeded EDB plus every derived fact.
    pub fn facts(&self) -> &RelationStore<D, N> {
        &self.facts
    }

    /// Take ownership of the least model.
    pub fn into_facts(self) -> RelationStore<D, N> {
        self.facts
    }

    /// Recorded derivations in lexical order; `None` means recording was disabled,
    /// while `Some([])` means recording was enabled and no fact was derived.
    pub fn derivations(&self) -> Option<&[Derivation<N>]> {
        self.derivations.as_deref()
    }

    /// Semantic fact counts and the physical work of this execution.
    pub fn work(&self) -> WorkReport {
        self.work
    }

    /// What the run consumed, against the limits it ran under.
    pub fn budget(&self) -> BudgetReport {
        self.budget
    }
}

// ── Compilation: stratification with a named cycle ──────────────────────────────

/// The original [`compile`] with sourced full-IR never-derive admission.
/// # Errors
/// Returns exact protected/undecidable head refusal before the ordinary
/// executor's fragment checks, then the same errors as [`compile`].
pub fn compile_with_declarations<const N: usize>(
    rules: Vec<DlClause<N>>,
    declarations: &crate::admission::NeverDeriveDeclarations,
) -> Result<Executable<N>, EvalError>
where
    Arity<N>: StoreShape<N>,
{
    declarations.admit(&rules).map_err(EvalError::NeverDerive)?;
    compile(rules)?
        .admit_declarations(declarations)
        .map_err(EvalError::NeverDerive)
}

/// Compile a rule program into the executor's [`Executable`], or refuse it.
///
/// This is [`crate::plan`]'s pipeline with three refusals attached: a clause whose head is
/// existential, disjunctive or false is reported by head form, a non-stratifiable program is
/// reported with the concrete cycle its negative edge sits in, and a clause whose head
/// carries a variable no positive body atom can bind is reported as not range-restricted.
/// All three are hard errors; none has a best-effort fallback.
///
/// # Errors
///
/// [`EvalError::NonDatalogHead`], [`EvalError::NonStratifiable`] or
/// [`EvalError::UnboundHeadVariable`].
pub fn compile<const N: usize>(rules: Vec<DlClause<N>>) -> Result<Executable<N>, EvalError>
where
    Arity<N>: StoreShape<N>,
{
    // Decide the fragment before stratification and safety: neither check supplies
    // semantics for witnesses, branching or inconsistency.
    for (index, rule) in rules.iter().enumerate() {
        let form = rule.head_form();
        if !form.is_datalog() {
            return Err(EvalError::NonDatalogHead { rule: index, form });
        }
    }

    if let Some(rule) = rules.iter().position(DlClause::reads_base) {
        return Err(EvalError::BaseReadRequiresSchedule { rule });
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
        .expect("every head form was just established to be Datalog on the same rules")
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
pub(crate) fn check_range_restricted<const N: usize>(rules: &[DlClause<N>]) -> Result<(), EvalError>
where
    Arity<N>: StoreShape<N>,
{
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
fn negative_cycle<const N: usize>(rules: &[DlClause<N>]) -> EvalError
where
    Arity<N>: StoreShape<N>,
{
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
                let Some(path) = shortest_path(&depends, &negated, &head_symbol) else {
                    continue;
                };
                let mut cycle = vec![head_symbol.clone()];
                cycle.extend(path);
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

// ── Semi-naive scan modes ───────────────────────────────────────────────────────

/// The semi-naive delta as a contiguous [`RowId`] range `[lo, hi)`.
///
/// Row ids are minted densely in the sorted commit loop, so a round's committed rows are
/// ALWAYS a contiguous span: delta membership is one range compare, with no per-round
/// bitset allocation and no hashing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Delta {
    /// Inclusive lower row index of the round's committed span.
    pub(crate) lo: u64,
    /// Exclusive upper row index of the round's committed span.
    pub(crate) hi: u64,
}

impl Delta {
    /// The round-1 seed: every accumulated row `[0, row_count)` is new this round.
    pub(crate) fn all(row_count: u64) -> Self {
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

/// One native arrangement per distinct partial delta, shared by every rule task.
/// An all-row seed borrows the accumulated arrangements instead of copying them.
#[derive(Debug)]
pub(crate) struct DeltaIndexes<I: TermIdentity> {
    runs: Vec<((u64, u64), DeltaRun<I>)>,
}

impl<I: TermIdentity> DeltaIndexes<I> {
    fn build<D: TermDictionary<Id = I>, const N: usize>(
        rel: &RelationStore<D, N>,
        deltas: impl IntoIterator<Item = Delta>,
    ) -> Result<Self, EvalError<D::Error>>
    where
        Arity<N>: StoreShape<N>,
    {
        rel.interner().check_read().map_err(EvalError::Source)?;
        let mut ranges = Vec::new();
        for delta in deltas {
            if delta != Delta::all(rel.row_slots()) {
                ranges
                    .try_reserve(1)
                    .map_err(|_| EvalError::Storage(StorageError::AllocationFailed))?;
                ranges.push((delta.lo, delta.hi));
            }
        }
        ranges.sort_unstable();
        ranges.dedup();
        let mut runs = Vec::new();
        runs.try_reserve(ranges.len())
            .map_err(|_| EvalError::Storage(StorageError::AllocationFailed))?;
        for (lo, hi) in ranges {
            let run = rel.delta_run(lo..hi).map_err(|error| match error {
                DictionaryError::Storage(error) => EvalError::Storage(error),
                DictionaryError::Source(error) => EvalError::Source(error),
            })?;
            runs.push(((lo, hi), run));
        }
        Ok(Self { runs })
    }

    fn get(&self, delta: Delta, row_count: u64) -> Option<&DeltaRun<I>> {
        if delta == Delta::all(row_count) {
            return None;
        }
        let position = self
            .runs
            .binary_search_by_key(&(delta.lo, delta.hi), |&(range, _)| range)
            .expect("every scheduled partial delta has an admitted arrangement");
        Some(&self.runs[position].1)
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

/// One shared scan selection for uniform factor modes and semi-naive anchors.
#[derive(Debug, Clone, Copy)]
enum JoinScan {
    Uniform(Scan),
    DeltaPosition(usize),
}

impl JoinScan {
    fn at(self, position: usize) -> Scan {
        match self {
            Self::Uniform(scan) => scan,
            Self::DeltaPosition(anchor) => scan_for(position, anchor),
        }
    }
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
/// Rebind a reordered operator against the current frame rather than its old prefix.
const INDEX_REBOUND: u8 = 4;

// ── The physical binding frame ──────────────────────────────────────────────────

/// One matched body row, held as ids only.
///
/// A source is `Copy` and carries all FOUR of the matched quad's positions: with the
/// predicate as data, an atom's predicate is no longer recoverable from the rule text, so
/// the row has to say which one it matched. The join never renders a term surface, never
/// clones a `String` and never re-hashes a fact; surfaces are resolved once, for committed
/// winners only, when the [`Derivation`] is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceRow<I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The read authority owning this row ID.
    source: crate::clause::ReadSource,
    /// Index into the producing rule's authored body.
    body_index: usize,
    /// The original matched row, without a graph position in triple execution.
    terms: [I; N],
    /// The matched row's store-global row id — the key of the proof-height column.
    row: RowId,
}

impl<I: TermIdentity, const N: usize> SourceRow<I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// This row as a [`ShapeFact<N>`] of lexical surfaces.
    fn fact<D: TermDictionary<Id = I>>(
        self,
        rel: &RelationStore<D, N>,
    ) -> Result<ShapeFact<N>, D::Error> {
        HeadKey {
            positions: self.terms.map(HeadTerm::Interned),
        }
        .fact(rel)
    }
}

/// Inline width shared by binding frames and transient positive-guard input views.
const INLINE_BINDING_SLOTS: usize = 8;

/// A partial solution of one rule's positive body.
///
/// Bindings live in a flat frame indexed by the plan's variable slots, and hold interned
/// [`TermId`]s rather than surfaces, so a probe is an integer compare.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SlotSolution<I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// One slot per plan variable, `None` until an atom binds it.
    ///
    /// Inline rather than heap. This frame is CLONED once per surviving candidate row
    /// in the indexed atom frame, so a heap frame is one allocation per intermediate solution
    /// — the join's most frequent allocation by a wide margin, and one the evaluator
    /// paid unconditionally.
    ///
    /// Capacity 8 is measured, not assumed: 8 is the widest frame the OWL 2 RL rule
    /// table and this workspace's own suites produce, so every real frame lives inline.
    /// The native identity keeps its `NonZeroU64` niche in `Option`; a host identity
    /// determines its own inline footprint. A ninth variable spills to the heap.
    bindings: SmallVec<[Option<I>; INLINE_BINDING_SLOTS]>,
    /// The matched body rows, in physical execution order until the plan's swap program
    /// restores authored order.
    sources: Vec<SourceRow<I, N>>,
    /// Slots a guard bound to a surface the store has never interned, with that
    /// surface. Empty — and therefore allocation-free to clone — for every solution of
    /// a guard-free rule; a slot listed here is `None` in `bindings`.
    computed: Vec<(usize, Box<str>)>,
    /// Original negative-body witness, present only for evidenced execution.
    negative_evidence: Option<purrdf_core::evidence::EvidenceId>,
}

/// What one frame slot holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotValue<'s, I: TermIdentity = TermId> {
    /// A term the store has interned.
    Interned(I),
    /// A guard-computed surface the store has never interned.
    Computed(&'s str),
    /// Nothing yet.
    Unbound,
}

impl<'s, I: TermIdentity> SlotValue<'s, I> {
    /// Carry a binding through the guard boundary without rendering an identity.
    fn as_term(self) -> Option<DictionaryTerm<'s, I>> {
        match self {
            Self::Interned(id) => Some(DictionaryTerm::Id(id)),
            Self::Computed(surface) => Some(DictionaryTerm::Surface(surface)),
            Self::Unbound => None,
        }
    }
}

impl<I: TermIdentity, const N: usize> SlotSolution<I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Conjoin the original negative premise without changing positive provenance.
    fn retain_negative<D: TermDictionary<Id = I>>(
        &mut self,
        id: purrdf_core::evidence::EvidenceId,
        owner: &mut dyn evidence::RuleEvidence<D, N>,
    ) -> Result<(), EvalError<D::Error>> {
        self.negative_evidence = Some(match self.negative_evidence {
            None => id,
            Some(previous) => owner.combine(
                previous,
                id,
                purrdf_core::evidence::EvidenceCombination::Conjunction,
            )?,
        });
        Ok(())
    }

    /// The empty substitution over a frame of `slot_count` variables.
    fn empty(slot_count: usize) -> Self {
        Self {
            bindings: smallvec![None; slot_count],
            sources: Vec::new(),
            computed: Vec::new(),
            negative_evidence: None,
        }
    }

    /// The interned value bound in `slot`, if any.
    #[inline]
    fn get(&self, slot: usize) -> Option<I> {
        self.bindings[slot]
    }

    /// Everything `slot` may hold, a computed surface included.
    fn value(&self, slot: usize) -> SlotValue<'_, I> {
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
}

impl<I: TermIdentity, const N: usize> SlotSolution<I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// A bound head variable constrains a guard output rather than being overwritten.
    fn unify_guard_value<D: TermDictionary<Id = I>>(
        &mut self,
        slot: usize,
        value: &DictionaryValue<I>,
        rel: &RelationStore<D, N>,
    ) -> Result<bool, D::Error> {
        let Some(previous) = self.value(slot).as_term() else {
            self.bind_value(slot, value, rel)?;
            return Ok(true);
        };
        let previous = previous.canonicalize(rel.interner())?;
        let incoming = value.as_term().canonicalize(rel.interner())?;
        let same = match (previous, incoming) {
            (DictionaryTerm::Id(left), DictionaryTerm::Id(right)) => left == right,
            (DictionaryTerm::Surface(left), DictionaryTerm::Surface(right)) => left == right,
            _ => false,
        };
        rel.interner().check_read()?;
        Ok(same)
    }

    /// Retain an original identity or canonicalize a genuinely computed spelling.
    fn bind_value<D: TermDictionary<Id = I>>(
        &mut self,
        slot: usize,
        value: &DictionaryValue<I>,
        rel: &RelationStore<D, N>,
    ) -> Result<(), D::Error> {
        match value.as_term().canonicalize(rel.interner())? {
            DictionaryTerm::Id(id) => self.bindings[slot] = Some(id),
            DictionaryTerm::Surface(surface) => self.computed.push((slot, surface.into())),
        }
        Ok(())
    }
}

// ── The indexed binary join ─────────────────────────────────────────────────────

/// What one argument position resolves to under a partial solution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PositionValue<I: TermIdentity = TermId> {
    /// Pinned to this interned term — a constant the store knows, or a bound variable.
    Known(I),
    /// Free: the scan binds it.
    Free,
    /// Pinned to a term the store has never interned, so no row can match.
    Missing,
}

impl<I: TermIdentity> PositionValue<I> {
    /// The pinned id, or `None` when the position is free.
    ///
    /// [`Missing`](Self::Missing) never reaches here: a solution carrying one is discarded
    /// before any partition is addressed.
    #[inline]
    fn known(self) -> Option<I> {
        match self {
            Self::Known(id) => Some(id),
            Self::Free | Self::Missing => None,
        }
    }
}

/// Per-position constant bindings resolved once for an atom invocation.
type ConstantPositions<I, const N: usize> = [Option<PositionValue<I>>; N];

/// Resolve the constant positions of `shape` ONCE per operator invocation.
///
/// A constant's surface does not depend on the partial solution, so its dictionary probe
/// is hoisted out of the per-solution loop exactly as it was before the predicate became a
/// term. `None` marks a variable position, resolved per solution.
fn constant_positions<D: TermDictionary, const N: usize>(
    shape: &AtomShape<N>,
    rel: &RelationStore<D, N>,
) -> Result<ConstantPositions<D::Id, N>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    rel.interner().check_read()?;
    let mut constants = [None; N];
    for (position, value) in constants.iter_mut().enumerate() {
        if let Some(surface) = shape.positions()[position].constant() {
            *value = Some(
                rel.interner()
                    .lookup_term(surface)?
                    .map_or(PositionValue::Missing, PositionValue::Known),
            );
        }
    }
    rel.interner().check_read()?;
    Ok(constants)
}

/// Extend every partial solution by index-selecting `operator`'s matching rows.
///
/// This is the ONE-TIME scan-mode dispatch: once per operator invocation — not per row —
/// the scan mode is lifted to a `const SCAN` monomorphization parameter, so the per-row
/// delta filter has no runtime branch.
#[cfg(test)]
fn extend_slot_solutions<D: TermDictionary, const N: usize>(
    operator: &AtomOperator<N>,
    rel: &RelationStore<D, N>,
    delta: Delta,
    indexes: &DeltaIndexes<D::Id>,
    scan: Scan,
    solutions: Vec<SlotSolution<D::Id, N>>,
    governor: &mut StepGovernor<'_>,
) -> Result<Vec<SlotSolution<D::Id, N>>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    extend_selected_atom(
        operator,
        rel,
        delta,
        indexes,
        atom::Access::Planned(scan),
        solutions,
        governor,
    )
}

/// Probe a physically reordered atom while retaining its logical scan ordinal.
#[cfg(test)]
fn extend_rebound_solutions<D: TermDictionary, const N: usize>(
    operator: &AtomOperator<N>,
    rel: &RelationStore<D, N>,
    delta: Delta,
    indexes: &DeltaIndexes<D::Id>,
    scan: Scan,
    solutions: Vec<SlotSolution<D::Id, N>>,
    governor: &mut StepGovernor<'_>,
) -> Result<Vec<SlotSolution<D::Id, N>>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    extend_selected_atom(
        operator,
        rel,
        delta,
        indexes,
        atom::Access::Rebound(scan),
        solutions,
        governor,
    )
}

/// Drive one original native atom frame through the shared mode dispatch.
#[cfg(test)]
fn extend_selected_atom<D: TermDictionary, const N: usize>(
    operator: &AtomOperator<N>,
    rel: &RelationStore<D, N>,
    delta: Delta,
    indexes: &DeltaIndexes<D::Id>,
    access: atom::Access,
    solutions: Vec<SlotSolution<D::Id, N>>,
    governor: &mut StepGovernor<'_>,
) -> Result<Vec<SlotSolution<D::Id, N>>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    let mut frame = atom::Frame::new(operator, rel, solutions)?;
    frame.advance_selected(operator, rel, delta, indexes, access, governor)?;
    // The existing round owner observes this same refusal flag and returns no
    // complete model. The resumable owner retains the frame instead of dropping it.
    debug_assert!(frame.is_complete() || governor.spent());
    Ok(frame.into_output())
}

// ── The leapfrog triejoin ───────────────────────────────────────────────────────

/// The single partition a certified cycle atom addresses, or `None` if the store has no
/// such partition (in which case the component's intersection is empty).
///
/// Cycle certification admits only atoms whose predicate AND graph are constants — a trie
/// level is one sorted arrangement, and an atom that quantifies over its predicate denotes
/// a union of them — so the surfaces are always there to resolve.
fn cycle_atom_partition<'a, D: TermDictionary, const N: usize>(
    shape: &AtomShape<N>,
    rel: &'a RelationStore<D, N>,
    delta_run: Option<&'a DeltaRun<D::Id>>,
) -> Result<Option<PartitionRef<'a, D::Id, N>>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    let constants = constant_positions(shape, rel)?;
    let known = constants.map(|value| value.and_then(PositionValue::known));
    Ok(rel
        .partition_for(&known)
        .and_then(|partition| match delta_run {
            Some(run) => run.project(partition),
            None => Some(partition),
        }))
}

/// The subject and object frame slots of a certified cycle atom.
///
/// Cycle certification admits only atoms with two DISTINCT variable positions there, so
/// both slots exist.
fn cycle_atom_slots<const N: usize>(shape: &AtomShape<N>) -> (usize, usize)
where
    Arity<N>: StoreShape<N>,
{
    let (Some(subject), Some(object)) = (shape.subject().slot(), shape.object().slot()) else {
        unreachable!("cycle certification admits only distinct variable-variable atoms")
    };
    (subject, object)
}

/// Immutable state shared by every variable level of one leapfrog component.
#[derive(Debug)]
struct LeapfrogRun<'a, D: TermDictionary = TermInterner, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The rule's plan, for the per-atom operators.
    plan: &'a RulePlan<N>,
    /// The certified component being descended.
    cycle: &'a CyclicPlan,
    /// Uniform factor mode or the round's semi-naive delta position.
    scan: JoinScan,
    /// The accumulated store.
    rel: &'a RelationStore<D, N>,
    /// The round's delta span.
    delta: Delta,
    /// Shared native new-row arrangements, selected only for delta atoms.
    indexes: &'a DeltaIndexes<D::Id>,
}

impl<D: TermDictionary, const N: usize> LeapfrogRun<'_, D, N>
where
    Arity<N>: StoreShape<N>,
{
    fn delta_run(&self, position: usize) -> Option<&DeltaRun<D::Id>> {
        if matches!(self.scan.at(position), Scan::Delta) {
            self.indexes.get(self.delta, self.rel.row_slots())
        } else {
            None
        }
    }
    /// Capture the unique fully-ground row for every cycle atom, in the component's
    /// authored atom order.
    ///
    /// Returns `false` — restoring `solution` — if a scan-mode constraint excludes any of
    /// them, which is how the semi-naive decomposition is enforced on a multiway group.
    fn append_sources(
        &self,
        solution: &mut SlotSolution<D::Id, N>,
        work: &cycle::Work,
    ) -> Result<bool, D::Error> {
        let original = solution.sources.len();
        for &planned in self.cycle.atoms() {
            let operator = self.plan.operator_at(planned.positive_position());
            let shape = operator.shape();
            let (subject_slot, object_slot) = cycle_atom_slots(shape);
            work.record(cycle::Probe::PartitionLookup);
            let (Some(subject), Some(object), Some(partition)) = (
                solution.get(subject_slot),
                solution.get(object_slot),
                cycle_atom_partition(shape, self.rel, self.delta_run(planned.positive_position()))?,
            ) else {
                solution.sources.truncate(original);
                return Ok(false);
            };
            let scan = self.scan.at(planned.positive_position());
            let mut rows = partition.select(Bound::Both(subject, object));
            let mut matched = None;
            loop {
                work.record(cycle::Probe::LeafRow);
                let Some((subject, object, row)) = rows.next() else {
                    break;
                };
                if keep_row_for_scan(scan, self.delta, row) {
                    matched = Some(SourceRow {
                        source: crate::clause::ReadSource::Evaluation,
                        body_index: planned.body_index(),
                        terms: partition.row(subject, object),
                        row,
                    });
                    break;
                }
            }
            let Some(source) = matched else {
                solution.sources.truncate(original);
                return Ok(false);
            };
            solution.sources.push(source);
        }
        Ok(true)
    }
}

/// Extend the certified component through its one owned traversal frame.
#[cfg(test)]
fn extend_solutions_leapfrog<D: TermDictionary, const N: usize>(
    run: &LeapfrogRun<'_, D, N>,
    solutions: Vec<SlotSolution<D::Id, N>>,
    governor: &mut StepGovernor<'_>,
) -> Result<Vec<SlotSolution<D::Id, N>>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    let mut frame = cycle::Frame::new(solutions);
    frame.advance(run, governor)?;
    debug_assert!(frame.is_complete() || governor.spent());
    Ok(frame.into_output())
}

// ── Joining a rule body ─────────────────────────────────────────────────────────

/// Indexed cardinality of a constant-key full pattern or a native delta pattern.
/// A free-key full pattern returns `None` rather than sweeping historical partitions.
/// Subject/object bounds use existing run lengths and the bounded mutable tail.
fn constant_pattern_count<D: TermDictionary, const N: usize>(
    operator: &AtomOperator<N>,
    rel: &RelationStore<D, N>,
    delta: Option<&DeltaRun<D::Id>>,
    seed: Option<&SlotSolution<D::Id, N>>,
) -> Result<Option<usize>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    let mut constants = constant_positions(operator.shape(), rel)?;
    if let Some(seed) = seed {
        for (value, position) in constants.iter_mut().zip(operator.shape().positions()) {
            if let Some(id) = position.slot().and_then(|slot| seed.get(slot)) {
                *value = Some(PositionValue::Known(id));
            }
        }
    }
    if constants.contains(&Some(PositionValue::Missing)) {
        return Ok(Some(0));
    }
    let positions = constants.map(|value| value.and_then(PositionValue::known));
    if delta.is_none()
        && !Arity::<N>::KEY_POSITIONS
            .iter()
            .all(|&position| positions[position].is_some())
    {
        return Ok(None);
    }
    let bound = match (positions[POSITION_SUBJECT], positions[POSITION_OBJECT]) {
        (Some(subject), Some(object)) => Bound::Both(subject, object),
        (Some(subject), None) => Bound::Subject(subject),
        (None, Some(object)) => Bound::Object(object),
        (None, None) => Bound::Any,
    };
    let mut count = 0usize;
    for partition in rel.select_partitions(positions, delta) {
        let selected = if delta.is_some() {
            match bound {
                Bound::Any => partition.row_count(),
                _ => partition.select(bound).remaining_count(),
            }
        } else if let Some(selected) = partition.indexed_count(bound) {
            selected
        } else {
            return Ok(None);
        };
        count = count.saturating_add(selected);
    }
    rel.interner().check_read()?;
    Ok(Some(count))
}

/// Freeze metadata advice before any task can warm an access path. Empty means
/// this all-row, single-atom or Base-reading join needs no adaptive advice.
fn constant_pattern_counts<D: TermDictionary, const N: usize>(
    plan: &RulePlan<N>,
    rel: &RelationStore<D, N>,
    delta: Delta,
) -> Result<Vec<Option<usize>>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    rel.interner().check_read()?;
    if delta == Delta::all(rel.row_slots())
        || plan.operators().len() <= 1
        || plan
            .operators()
            .iter()
            .any(|operator| operator.shape().read_source() == crate::clause::ReadSource::Base)
    {
        return Ok(Vec::new());
    }
    let counts = plan
        .operators()
        .iter()
        .map(|operator| constant_pattern_count(operator, rel, None, None))
        .collect::<Result<_, _>>()?;
    rel.interner().check_read()?;
    Ok(counts)
}

/// Prefer an old indexed seed only when it is strictly smaller than the delta.
/// This admits at most the delta's cardinality before probing the delta itself;
/// a selective delta never pays for a large old prefix. Unknown/free-key full
/// patterns cannot displace the delta. Counts read run metadata, not old tuples.
fn decomposition_seed<D: TermDictionary, const N: usize>(
    plan: &RulePlan<N>,
    snapshot: JoinSnapshot<'_, D, N>,
    anchor: usize,
    full_counts: &[Option<usize>],
) -> Result<usize, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    let delta = snapshot
        .indexes
        .get(snapshot.delta, snapshot.rel.row_slots());
    let mut best = constant_pattern_count(&plan.operators()[anchor], snapshot.rel, delta, None)?
        .expect("a partial delta has native indexed runs");
    let mut seed = anchor;
    for (position, count) in full_counts.iter().copied().enumerate() {
        if position == anchor {
            continue;
        }
        if let Some(count) = count
            && count < best
        {
            seed = position;
            best = count;
        }
    }
    Ok(seed)
}

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
#[cfg(test)]
fn join_body<D: TermDictionary, const N: usize>(
    plan: &RulePlan<N>,
    snapshot: JoinSnapshot<'_, D, N>,
    strategy: JoinStrategy,
    governor: &mut StepGovernor<'_>,
    full_counts: Option<&[Option<usize>]>,
) -> Result<Vec<SlotSolution<D::Id, N>>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    let mut frame = join::Frame::new(plan, snapshot, strategy, full_counts)?;
    frame.advance(plan, snapshot, governor)?;
    debug_assert!(frame.is_complete() || governor.spent());
    Ok(frame.into_output())
}

/// What a positive join reads: the accumulated store and the delta it is decomposed over.
#[derive(Debug)]
struct JoinSnapshot<'a, D: TermDictionary = TermInterner, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The accumulated store.
    rel: &'a RelationStore<D, N>,
    /// The rows this evaluation treats as new.
    delta: Delta,
    /// The round's shared arrangements, including this delta when it is partial.
    indexes: &'a DeltaIndexes<D::Id>,
    /// Original native bindings for a head-directed body probe, if requested.
    seed: Option<&'a SlotSolution<D::Id, N>>,
}

impl<D: TermDictionary, const N: usize> Copy for JoinSnapshot<'_, D, N> where Arity<N>: StoreShape<N>
{}
impl<D: TermDictionary, const N: usize> Clone for JoinSnapshot<'_, D, N>
where
    Arity<N>: StoreShape<N>,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: TermDictionary, const N: usize> JoinSnapshot<'_, D, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Copy the original native substitution only when entering a new anchor.
    fn initial(self, plan: &RulePlan<N>) -> SlotSolution<D::Id, N> {
        self.seed
            .cloned()
            .unwrap_or_else(|| SlotSolution::empty(plan.variables().len()))
    }

    /// Every position before the last requires an OldOnly suffix. When the
    /// delta covers the entire frozen store, that suffix is provably empty;
    /// visiting its earlier prefixes can produce no solution. Partial deltas
    /// retain every position, including positions that become productive later.
    fn delta_positions(self, positive_count: usize) -> std::ops::Range<usize> {
        let first = if self.delta == Delta::all(self.rel.row_slots()) {
            positive_count.saturating_sub(1)
        } else {
            0
        };
        first..positive_count
    }
}

/// Whether the atom `operator` can match a row of `snapshot`'s delta: some partition its
/// constant predicate and graph positions admit holds a row minted at or after the
/// delta's first row.
///
/// A semi-naive decomposition whose delta atom cannot match is skipped. It would otherwise
/// enumerate every atom before the delta position in full — a whole relation, for a rule
/// whose recursive atom is planned first — and then match nothing, so a linear recursion
/// such as `connected(x, z) :- connected(x, y), link(y, z)` would spend a scan of the
/// whole closure per round on the decomposition anchored at `link`, which gains no row
/// after the first round. Skipping it changes no solution: the delta scan admits only
/// rows at or after the delta's first row, and there are none to admit. It changes only
/// the candidates enumerated, which is the join-step count.
fn delta_can_match<D: TermDictionary, const N: usize>(
    operator: &AtomOperator<N>,
    snapshot: JoinSnapshot<'_, D, N>,
) -> Result<bool, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    let shape = operator.shape();
    let constants = constant_positions(shape, snapshot.rel)?;
    let mut positions = [None; N];
    for &position in Arity::<N>::KEY_POSITIONS {
        positions[position] = match constants[position] {
            Some(PositionValue::Known(id)) => Some(id),
            Some(PositionValue::Missing) => return Ok(false),
            Some(PositionValue::Free) | None => None,
        };
    }
    let run = snapshot
        .indexes
        .get(snapshot.delta, snapshot.rel.row_slots());
    Ok(snapshot
        .rel
        .select_partitions(positions, run)
        .next()
        .is_some())
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
enum ProbeValue<I: TermIdentity = TermId> {
    /// Pinned to this interned term.
    Known(I),
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
    /// variable of the body AND the head, in every one of their stored positions, so that is
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
    fn of_atom<const N: usize>(atom: &ClauseAtom<N>, variables: &[String]) -> [Self; N] {
        let terms = atom.terms();
        std::array::from_fn(|position| Self::of(terms[position], variables))
    }

    /// This argument under `solution`, for a store probe.
    fn probe<D: TermDictionary, const N: usize>(
        &self,
        solution: &SlotSolution<D::Id, N>,
        rel: &RelationStore<D, N>,
    ) -> Result<ProbeValue<D::Id>, D::Error>
    where
        Arity<N>: StoreShape<N>,
    {
        Ok(match self {
            Self::Slot(slot) => match solution.value(*slot) {
                SlotValue::Interned(id) => ProbeValue::Known(id),
                SlotValue::Computed(_) => ProbeValue::Absent,
                SlotValue::Unbound => ProbeValue::Free,
            },
            Self::Const(surface) => rel
                .interner()
                .lookup_term(surface)?
                .map_or(ProbeValue::Absent, ProbeValue::Known),
        })
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
struct NegatedAtom<const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The four lowered arguments, in `(subject, predicate, object, graph)` order.
    args: [ArgShape; N],
    /// The original body read authority.
    source: crate::clause::ReadSource,
}

impl<const N: usize> NegatedAtom<N>
where
    Arity<N>: StoreShape<N>,
{
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
    #[cfg(test)]
    fn satisfied<D: TermDictionary>(
        &self,
        solution: &SlotSolution<D::Id, N>,
        rel: &RelationStore<D, N>,
        governor: &mut StepGovernor<'_>,
    ) -> Result<bool, D::Error> {
        self.visit_matches(
            solution,
            rel,
            governor,
            |_| Ok::<_, D::Error>(true),
            core::convert::identity,
        )
    }

    /// Visit the original indexed probe, stopping only when its visitor requests it.
    /// Validate the source even on an empty, exhausted or early-match path; preserve
    /// an earlier visitor failure before inspecting the final source state.
    #[cfg(test)]
    fn visit_matches<D: TermDictionary, E>(
        &self,
        solution: &SlotSolution<D::Id, N>,
        rel: &RelationStore<D, N>,
        governor: &mut StepGovernor<'_>,
        visit: impl FnMut(RowId) -> Result<bool, E>,
        source_error: impl Fn(D::Error) -> E,
    ) -> Result<bool, E> {
        let mut probe = negative::Probe::new(self.source);
        let found = probe.advance(self, solution, rel, governor, visit, source_error)?;
        debug_assert!(probe.is_complete() || governor.spent());
        Ok(found)
    }

    /// Retain signed witnesses on the native path; keep the ordinary boolean probe.
    #[cfg(test)]
    fn blocks<D: TermDictionary>(
        &self,
        solution: &mut SlotSolution<D::Id, N>,
        rel: &RelationStore<D, N>,
        governor: &mut StepGovernor<'_>,
        evidence: &mut Option<&mut dyn evidence::RuleEvidence<D, N>>,
    ) -> Result<bool, EvalError<D::Error>> {
        let mut frame = negative::Atom::new(self.source, evidence.is_some());
        frame.advance(self, solution, rel, governor, evidence)?;
        debug_assert!(frame.is_complete() || governor.spent());
        Ok(frame.blocked())
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
enum LocalValue<I: TermIdentity = TermId> {
    /// An interned term.
    Interned(I),
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
struct NegationRuntime<const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The group's index in its rule.
    index: usize,
    /// The group's atoms, each with its four lowered arguments.
    atoms: Vec<[GroupArg; N]>,
    /// Source of each authored negative-conjunction atom.
    sources: Vec<crate::clause::ReadSource>,
    /// The group's guards, in authored order.
    guards: Vec<GroupGuard>,
    /// How many local variables the group quantifies.
    locals: usize,
}

/// What one probe of a negated conjunction needs besides the group itself.
struct GroupProbe<'a, D: TermDictionary = TermInterner, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The enclosing rule's solution.
    solution: &'a SlotSolution<D::Id, N>,
    /// The accumulated store.
    rel: &'a RelationStore<D, N>,
    /// The caller's guard evaluator.
    guards: &'a dyn GuardEvaluator<D, N>,
    /// The rule's index in authored program order.
    rule: usize,
    /// The rule itself, for its guards' declarations.
    clause: &'a DlClause<N>,
}

impl<D: TermDictionary, const N: usize> Copy for GroupProbe<'_, D, N> where Arity<N>: StoreShape<N> {}

impl<D: TermDictionary, const N: usize> Clone for GroupProbe<'_, D, N>
where
    Arity<N>: StoreShape<N>,
{
    fn clone(&self) -> Self {
        *self
    }
}

/// Independent recursive test oracle; shipping execution uses the owned heap DFS.
#[cfg(test)]
struct GroupEvidence<'a, D: TermDictionary = TermInterner, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    owner: &'a mut dyn evidence::RuleEvidence<D, N>,
    current: purrdf_core::evidence::EvidenceId,
    witnesses: Option<purrdf_core::evidence::EvidenceId>,
}

impl<const N: usize> NegationRuntime<N>
where
    Arity<N>: StoreShape<N>,
{
    /// Lower `negation` against the enclosing rule's frame.
    fn new(index: usize, negation: &Negation<N>, rule: &DlClause<N>, variables: &[String]) -> Self {
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
        let atoms: Vec<[GroupArg; N]> = negation
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
            sources: negation
                .atoms()
                .iter()
                .map(ClauseAtom::read_source)
                .collect(),
            guards,
            locals: locals.len(),
        }
    }
}

impl<const N: usize> NegationRuntime<N>
where
    Arity<N>: StoreShape<N>,
{
    /// Whether the group HOLDS under `probe.solution`: some extension of its local
    /// variables matches every atom and passes every guard.
    /// Retained only as an independent oracle for the owned production search.
    #[cfg(test)]
    fn holds<D: TermDictionary>(
        &self,
        probe: GroupProbe<'_, D, N>,
        governor: &mut StepGovernor<'_>,
    ) -> Result<bool, EvalError<D::Error>> {
        probe
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)?;
        let result = if governor.charge() {
            let mut locals: Vec<Option<LocalValue<D::Id>>> = vec![None; self.locals];
            self.match_atom(0, probe, &mut locals, governor, &mut None)?
        } else {
            false
        };
        probe
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)?;
        Ok(result)
    }

    /// Probe all signed conjunction witnesses through the same atom/guard traversal.
    #[cfg(test)]
    fn blocks<D: TermDictionary>(
        &self,
        probe: GroupProbe<'_, D, N>,
        governor: &mut StepGovernor<'_>,
        evidence: &mut Option<&mut dyn evidence::RuleEvidence<D, N>>,
    ) -> Result<(bool, Option<purrdf_core::evidence::EvidenceId>), EvalError<D::Error>> {
        let mut frame = negation::Frame::new(evidence.is_some());
        frame.advance(self, probe, governor, evidence)?;
        debug_assert!(frame.is_complete() || governor.spent());
        Ok(frame.outcome())
    }

    /// The store probe value of `arg` under the current bindings.
    fn probe_value<D: TermDictionary>(
        arg: &GroupArg,
        probe: GroupProbe<'_, D, N>,
        locals: &[Option<LocalValue<D::Id>>],
    ) -> Result<ProbeValue<D::Id>, D::Error> {
        Ok(match arg {
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
                .interner()
                .lookup_term(surface)?
                .map_or(ProbeValue::Absent, ProbeValue::Known),
        })
    }

    /// Match atom `k` onward.
    #[cfg(test)]
    fn match_atom<D: TermDictionary>(
        &self,
        k: usize,
        probe: GroupProbe<'_, D, N>,
        locals: &mut Vec<Option<LocalValue<D::Id>>>,
        governor: &mut StepGovernor<'_>,
        evidence: &mut Option<GroupEvidence<'_, D, N>>,
    ) -> Result<bool, EvalError<D::Error>> {
        let Some(atom) = self.atoms.get(k) else {
            return self.match_guard(0, probe, locals, governor, evidence);
        };
        let mut values = [ProbeValue::Free; N];
        for (position, arg) in atom.iter().enumerate() {
            values[position] = Self::probe_value(arg, probe, locals).map_err(EvalError::Source)?;
            if values[position] == ProbeValue::Absent {
                return Ok(false);
            }
        }
        let known = |value: ProbeValue<D::Id>| match value {
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
        for partition in probe
            .rel
            .read_source(self.sources[k])
            .partitions_for(values.map(known))
        {
            if !governor.charge() {
                return Ok(false);
            }
            let mut cursor = partition.select(bound);
            while let Some((subject, object, row)) = cursor.next() {
                if !governor.charge() {
                    return Ok(false);
                }
                let matched = partition.row(subject, object);
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
                let previous = if consistent {
                    if let Some(trace) = evidence.as_mut() {
                        let previous = trace.current;
                        let source = trace.owner.source(row)?;
                        trace.current = trace.owner.combine(
                            previous,
                            source,
                            purrdf_core::evidence::EvidenceCombination::Conjunction,
                        )?;
                        Some(previous)
                    } else {
                        None
                    }
                } else {
                    None
                };
                let found =
                    consistent && self.match_atom(k + 1, probe, locals, governor, evidence)?;
                if let (Some(trace), Some(previous)) = (evidence.as_mut(), previous) {
                    trace.current = previous;
                }
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
    #[cfg(test)]
    fn match_guard<D: TermDictionary>(
        &self,
        j: usize,
        probe: GroupProbe<'_, D, N>,
        locals: &mut Vec<Option<LocalValue<D::Id>>>,
        governor: &mut StepGovernor<'_>,
        evidence: &mut Option<GroupEvidence<'_, D, N>>,
    ) -> Result<bool, EvalError<D::Error>> {
        let Some(guard) = self.guards.get(j) else {
            if let Some(trace) = evidence.as_mut() {
                trace.witnesses = Some(match trace.witnesses {
                    None => trace.current,
                    Some(previous) => trace.owner.combine(
                        previous,
                        trace.current,
                        purrdf_core::evidence::EvidenceCombination::Alternatives,
                    )?,
                });
                return Ok(false);
            }
            return Ok(true);
        };
        if !governor.charge() {
            return Ok(false);
        }
        let declared = &probe.clause.negations()[self.index].guards()[j];
        let site = GuardSite::Negation {
            negation: self.index,
            guard: j,
        };
        let rows = {
            let mut inputs = Vec::with_capacity(guard.inputs.len());
            for input in &guard.inputs {
                let value = match input {
                    GroupArg::Outer(slot) => probe
                        .solution
                        .value(*slot)
                        .as_term()
                        .expect("an outer group variable is bound by the enclosing rule"),
                    GroupArg::Local(local) => match &locals[*local] {
                        Some(LocalValue::Interned(id)) => SlotValue::Interned(*id),
                        Some(LocalValue::Computed(surface)) => SlotValue::Computed(surface),
                        None => unreachable!(
                            "a group guard input is bound before it (DlClause scoping)"
                        ),
                    }
                    .as_term()
                    .expect("a local group input is bound before the guard"),
                    GroupArg::Const(surface) => DictionaryTerm::Surface(surface)
                        .canonicalize(probe.rel.interner())
                        .map_err(EvalError::Source)?,
                };
                inputs.push(value);
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
                governor,
            )?
        };
        for row in rows {
            if governor.spent() {
                return Ok(false);
            }
            for (&local, value) in guard.outputs.iter().zip(&row) {
                locals[local] = Some(
                    match value
                        .as_term()
                        .canonicalize(probe.rel.interner())
                        .map_err(EvalError::Source)?
                    {
                        DictionaryTerm::Id(id) => LocalValue::Interned(id),
                        DictionaryTerm::Surface(surface) => LocalValue::Computed(surface.into()),
                    },
                );
            }
            let found = self.match_guard(j + 1, probe, locals, governor, evidence)?;
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
#[cfg(test)]
fn call_guard<D: TermDictionary, const N: usize>(
    guards: &dyn GuardEvaluator<D, N>,
    call: &GuardCall<'_, D, N>,
    governor: &mut StepGovernor<'_>,
) -> Result<crate::guard::GuardRows<D::Id>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    let mut invocation = guards::Invocation::Uncalled;
    invocation.advance(guards, call, governor)?;
    debug_assert!(invocation.is_complete() || governor.spent());
    Ok(invocation.into_rows())
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
pub(crate) struct RuleRuntime<const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// Independent positive components, certified without opaque callbacks.
    factors: Vec<factors::Factor>,
    /// Each head atom's four lowered arguments, in `(subject, predicate, object, graph)`
    /// order. A conjunctive head asserts every conjunct from one body solution under
    /// both the stratified fixpoint and the ordered schedule ([`crate::schedule`]).
    head: Vec<[ArgShape; N]>,
    /// The rule's negated body atoms, in authored order.
    negated: Vec<NegatedAtom<N>>,
    /// The rule's body guards, in authored order.
    guards: Vec<GuardRuntime>,
    /// The rule's negated conjunctions, in authored order.
    negations: Vec<NegationRuntime<N>>,
}

impl<const N: usize> RuleRuntime<N>
where
    Arity<N>: StoreShape<N>,
{
    /// Lower one rule's static shapes.
    pub(crate) fn new(rule: &DlClause<N>, plan: &RulePlan<N>) -> Self {
        let variables = plan.variables();
        let mut runtime = Self {
            factors: Vec::new(),
            head: rule
                .head_atoms()
                .map(|atom| ArgShape::of_atom(atom, variables))
                .collect(),
            negated: plan
                .negated()
                .iter()
                .map(|&index| NegatedAtom {
                    source: rule.body()[index].read_source(),
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
        };
        runtime.factors = factors::certify(plan, &runtime);
        runtime
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
enum HeadTerm<'r, I: TermIdentity = TermId> {
    /// A term already present in the store's dictionary.
    Interned(I),
    /// A term the store has never interned, so no fact can already carry it: borrowed
    /// when it is a head constant, owned when a guard computed it.
    Fresh(Cow<'r, str>),
}

impl<I: TermIdentity> HeadTerm<'_, I> {
    /// This argument's interned id, if the store already holds the term.
    fn interned(&self) -> Option<I> {
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
    fn surface<D: TermDictionary<Id = I>, const N: usize>(
        &self,
        rel: &RelationStore<D, N>,
    ) -> Result<String, D::Error>
    where
        Arity<N>: StoreShape<N>,
    {
        Ok(self.as_term().lexical(rel.interner())?.as_ref().to_owned())
    }

    /// Borrow a publication argument without resolving its original identity.
    fn as_term(&self) -> DictionaryTerm<'_, I> {
        match self {
            Self::Interned(id) => DictionaryTerm::Id(*id),
            Self::Fresh(surface) => DictionaryTerm::Surface(surface.as_ref()),
        }
    }
}

/// The identity of a candidate head fact.
///
/// All stored positions are [`HeadTerm`]s: with the predicate carried as data, a head
/// predicate can be a bound variable, so it cannot be a borrowed `&str` naming a relation.
/// The field order is the [`ShapeFact<N>`] order, so the derived `Ord` groups candidates the way
/// the commit sweep will emit them.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct HeadKey<'r, I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The head's actual ordered positions.
    positions: [HeadTerm<'r, I>; N],
}

impl<I: TermIdentity, const N: usize> HeadKey<'_, I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// The actual positions, in fact order.
    fn terms(&self) -> [&HeadTerm<'_, I>; N] {
        self.positions.each_ref()
    }
}

impl<I: TermIdentity, const N: usize> HeadKey<'_, I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Resolve a publication fact in position order, preserving the first refusal.
    fn fact<D: TermDictionary<Id = I>>(
        &self,
        rel: &RelationStore<D, N>,
    ) -> Result<ShapeFact<N>, D::Error> {
        Ok(Arity::<N>::fact(crate::dictionary::try_map_row(
            self.terms(),
            |term| term.surface(rel),
        )?))
    }
}

/// One rule firing, before the round's winner is chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Candidate<I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The producing rule's index in authored program order.
    rule: usize,
    /// The authored head ordinal, also distinguishing repeated head atoms.
    head: usize,
    /// The matched body rows, in authored body order.
    sources: Vec<SourceRow<I, N>>,
    /// `1 + max(source proof heights)`.
    proof_height: u32,
    /// The sum of the source proof heights — the second tiebreak component.
    sum_source_height: u64,
}

impl<I: TermIdentity, const N: usize> Candidate<I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Retain native proof cells through their original supplemental admission.
    fn copy_in<A: purrdf_lex::allocation::Admission + ?Sized>(
        &self,
        memory: &mut purrdf_lex::allocation::Memory<'_, A>,
    ) -> Result<Self, StorageError> {
        Ok(Self {
            rule: self.rule,
            head: self.head,
            sources: memory.collect(self.sources.iter().copied())?,
            proof_height: self.proof_height,
            sum_source_height: self.sum_source_height,
        })
    }

    /// The candidate's source facts, in authored body order.
    ///
    /// Each source names the quad it actually matched, predicate included: a body atom
    /// with a variable predicate matched some concrete one, and the provenance has to say
    /// which.
    fn source_facts<D: TermDictionary<Id = I>>(
        &self,
        rel: &RelationStore<D, N>,
    ) -> Result<Vec<ShapeFact<N>>, D::Error> {
        rel.interner().check_read()?;
        let facts = self
            .sources
            .iter()
            .map(|source| source.fact(rel))
            .collect::<Result<_, _>>()?;
        rel.interner().check_read()?;
        Ok(facts)
    }

    /// Whether this candidate beats `other` for the same head fact.
    ///
    /// A TOTAL order over observable provenance:
    /// `(proof_height, sum_source_height, sorted source facts, rule index, source facts,
    /// head ordinal)`,
    /// smallest wins. Every component is content-derived — proof heights, the
    /// dictionary's canonical fact order and the authored rule position — so the winner is a function of the
    /// program and the data, never of which rule the scheduler happened to run first.
    ///
    /// The comparison is deliberately staged: the cheap numeric prefix decides
    /// almost every real collision, and the dictionary-ordered source tail runs
    /// only on an exact numeric tie. Native dictionaries need no lexical projection.
    fn preferred_over<D: TermDictionary<Id = I>>(
        &self,
        other: &Self,
        rel: &RelationStore<D, N>,
    ) -> Result<bool, D::Error> {
        rel.interner().check_read()?;
        let order = witness_preference(
            (self.proof_height, self.sum_source_height),
            (other.proof_height, other.sum_source_height),
            || source_preference(&self.sources, self.rule, &other.sources, other.rule, rel),
        )?;
        rel.interner().check_read()?;
        Ok(order.then_with(|| self.head.cmp(&other.head)).is_lt())
    }
}

/// The canonical numeric prefix, comparing the source tail only on a tie.
fn witness_preference<E>(
    mine: (u32, u64),
    theirs: (u32, u64),
    lexical: impl FnOnce() -> Result<std::cmp::Ordering, E>,
) -> Result<std::cmp::Ordering, E> {
    match mine.cmp(&theirs) {
        std::cmp::Ordering::Equal => lexical(),
        order => Ok(order),
    }
}

/// The dictionary-ordered tail of the canonical witness law.
fn source_preference<D: TermDictionary, const N: usize>(
    mine: &[SourceRow<D::Id, N>],
    my_rule: usize,
    theirs: &[SourceRow<D::Id, N>],
    their_rule: usize,
    rel: &RelationStore<D, N>,
) -> Result<std::cmp::Ordering, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    rel.interner().check_read()?;
    let compare = |left: &SourceRow<D::Id, N>, right: &SourceRow<D::Id, N>| {
        rel.interner().compare_rows(
            &left.terms.map(DictionaryTerm::Id),
            &right.terms.map(DictionaryTerm::Id),
        )
    };
    let mut mine_sorted = mine.to_vec();
    let mut theirs_sorted = theirs.to_vec();
    purrdf_lex::walk::try_sort_unstable_by(&mut mine_sorted, compare)?;
    purrdf_lex::walk::try_sort_unstable_by(&mut theirs_sorted, compare)?;
    let mut order = crate::dictionary::compare_sequences(&mine_sorted, &theirs_sorted, compare)?;
    if order.is_eq() {
        order = my_rule.cmp(&their_rule);
    }
    if order.is_eq() {
        order = crate::dictionary::compare_sequences(mine, theirs, compare)?;
    }
    rel.interner().check_read()?;
    Ok(order)
}

/// The lexical tail over owned surfaces, also valid after a store rebuild.
fn fact_preference<G: Clone + Ord>(
    mine: &[crate::store::Fact<G>],
    my_rule: usize,
    theirs: &[crate::store::Fact<G>],
    their_rule: usize,
) -> std::cmp::Ordering {
    let mut mine_sorted = mine.to_vec();
    mine_sorted.sort();
    let mut theirs_sorted = theirs.to_vec();
    theirs_sorted.sort();
    (mine_sorted, my_rule, mine).cmp(&(theirs_sorted, their_rule, theirs))
}

/// An assumed fact's best proof, retaining the otherwise unobservable sum needed
/// across schedule groups. Owned facts survive the layer's indexed retraction.
#[derive(Debug, Clone)]
pub(crate) struct Confirmation<const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    derivation: Derivation<N>,
    sum_source_height: u64,
}

impl<const N: usize> Confirmation<N>
where
    Arity<N>: StoreShape<N>,
{
    /// The confirmed fact, independent of store row identities.
    pub(crate) fn fact(&self) -> &ShapeFact<N> {
        self.derivation.fact()
    }

    /// Keep the canonical proof across rounds and scheduled groups.
    pub(crate) fn merge(&mut self, other: Self) {
        if witness_preference(
            (other.derivation.proof_height, other.sum_source_height),
            (self.derivation.proof_height, self.sum_source_height),
            || {
                Ok::<_, core::convert::Infallible>(
                    fact_preference(
                        &other.derivation.sources,
                        other.derivation.rule,
                        &self.derivation.sources,
                        self.derivation.rule,
                    )
                    .then_with(|| other.derivation.head.cmp(&self.derivation.head)),
                )
            },
        )
        .unwrap_or_else(|never| match never {})
        .is_lt()
        {
            *self = other;
        }
    }

    /// Commit only public provenance, after every group has competed.
    pub(crate) fn into_derivation(self) -> Derivation<N> {
        self.derivation
    }
}

/// Raw maximum and saturated sum of a witness's source heights.
fn source_heights<I: TermIdentity, const N: usize>(
    sources: &[SourceRow<I, N>],
    depth: Depths<'_, I, N>,
) -> (u32, u64)
where
    Arity<N>: StoreShape<N>,
{
    sources.iter().fold((0, 0u64), |(maximum, sum), source| {
        let height = match source.source {
            crate::clause::ReadSource::Evaluation => depth.height(source.row),
            crate::clause::ReadSource::Base => 0,
        };
        (maximum.max(height), sum.saturating_add(u64::from(height)))
    })
}

/// One round's candidate winners, keyed by head fact.
///
/// A `BTreeMap` rather than a hash table: the merge sweep and the commit sweep both
/// iterate it, so its order reaches an output path and must be total and content-derived.
#[derive(Debug)]
pub(crate) struct RoundBuffer<'r, I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The best candidate seen so far per head fact.
    entries: BTreeMap<HeadKey<'r, I, N>, Candidate<I, N>>,
    /// Candidate solutions enumerated by the task that produced this buffer.
    join_steps: u64,
    /// A refused reservation; independent of saturated observable counts.
    join_refused: bool,
    /// The round stopped before its winners could be published.
    stopped: bool,
    /// Whether some entry carries a term a guard computed and the store never held — a
    /// TERM-GENERATING round ([`EvalOptions`]).
    generates_terms: bool,
    /// The rules of the candidates that carry such a term, for a term-limit refusal to
    /// name ([`EvalError::TermLimitExceeded`]).
    generating_rules: BTreeSet<usize>,
    /// Rows already in the store that the snapshot marks as ASSUMED and a rule of this
    /// round derived again, each with the derivation that did — see
    /// [`RoundSnapshot::assumed`].
    confirmed: BTreeMap<RowId, Confirmation<N>>,
}

impl<'r, I: TermIdentity, const N: usize> RoundBuffer<'r, I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// An empty buffer.
    fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            join_steps: 0,
            join_refused: false,
            stopped: false,
            generates_terms: false,
            generating_rules: BTreeSet::new(),
            confirmed: BTreeMap::new(),
        }
    }

    /// Whether the round derived nothing new.
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The assumed rows a rule of the round derived again, with the derivations.
    pub(crate) fn confirmed(&self) -> impl Iterator<Item = &Confirmation<N>> {
        self.confirmed.values()
    }

    /// Insert or quality-merge one candidate.
    fn insert<D: TermDictionary<Id = I>>(
        &mut self,
        key: HeadKey<'r, I, N>,
        candidate: Candidate<I, N>,
        rel: &RelationStore<D, N>,
    ) -> Result<(), D::Error> {
        rel.interner().check_read()?;
        let replace = match self.entries.get(&key) {
            Some(existing) => candidate.preferred_over(existing, rel)?,
            None => true,
        };
        rel.interner().check_read()?;
        if key.terms().iter().any(|term| term.is_generated()) {
            self.generates_terms = true;
            self.generating_rules.insert(candidate.rule);
        }
        if replace {
            self.entries.insert(key, candidate);
        }
        Ok(())
    }

    /// Fold a completed rule-local buffer in at the scheduling-erasing serial boundary.
    fn merge_from<D: TermDictionary<Id = I>>(
        &mut self,
        other: Self,
        rel: &RelationStore<D, N>,
    ) -> Result<(), D::Error> {
        rel.interner().check_read()?;
        self.join_steps = self.join_steps.saturating_add(other.join_steps);
        self.join_refused |= other.join_refused;
        self.stopped |= other.stopped;
        for (row, proof) in other.confirmed {
            match self.confirmed.get_mut(&row) {
                Some(existing) => existing.merge(proof),
                None => {
                    self.confirmed.insert(row, proof);
                }
            }
        }
        for (key, candidate) in other.entries {
            self.insert(key, candidate, rel)?;
        }
        rel.interner().check_read()
    }
}

/// The original target and native substitution of a read-only head probe.
#[derive(Debug)]
pub(crate) struct RuleBindings<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    target: Option<(usize, [I; N])>,
    seed: SlotSolution<I, N>,
}

/// Cold evaluation keeps dense heights; maintenance reads its one native proof owner.
/// An absent native derivation is an asserted height-zero row by construction.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Depths<'a, I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    Dense(&'a [u32]),
    Native(&'a maintained::Supports<I, N>),
}

impl<I: TermIdentity, const N: usize> Depths<'_, I, N>
where
    Arity<N>: StoreShape<N>,
{
    fn height(self, row: RowId) -> u32 {
        match self {
            Self::Dense(values) => values[row.resident_index()],
            Self::Native(native) => native.height(row),
        }
    }
}

impl<'a, I: TermIdentity, const N: usize> From<&'a [u32]> for Depths<'a, I, N>
where
    Arity<N>: StoreShape<N>,
{
    fn from(values: &'a [u32]) -> Self {
        Self::Dense(values)
    }
}

/// The immutable snapshot every rule task reads during one round.
///
/// No task may mutate any of it; the single sorted commit begins only after every task
/// buffer has been collected and merged in program order.
#[derive(Debug)]
pub(crate) struct RoundSnapshot<'a, D: TermDictionary = TermInterner, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The accumulated store.
    pub(crate) rel: &'a RelationStore<D, N>,
    /// Per-row proof heights, indexed by [`RowId`].
    pub(crate) depth: Depths<'a, D::Id, N>,
    /// Per-row ASSUMED flags, indexed by [`RowId`], or empty when nothing is assumed.
    ///
    /// An assumed row is a fact the caller asserted for the duration of a schedule layer
    /// ([`crate::schedule::LayerHooks`]) and will retract at its end unless a rule derives
    /// it too; a rule that re-derives one is recorded in [`RoundBuffer::confirmed`].
    pub(crate) assumed: &'a purrdf_core::FastSet<RowId>,
    /// Shared physical runs; direct rule probes prepare their own owner.
    pub(crate) delta_indexes: Option<&'a DeltaIndexes<D::Id>>,
    /// Read-only head demand; ordinary fixpoints publish only absent heads.
    pub(crate) probe: Option<&'a RuleBindings<D::Id, N>>,
}

impl<D: TermDictionary, const N: usize> Copy for RoundSnapshot<'_, D, N> where
    Arity<N>: StoreShape<N>
{
}
impl<D: TermDictionary, const N: usize> Clone for RoundSnapshot<'_, D, N>
where
    Arity<N>: StoreShape<N>,
{
    fn clone(&self) -> Self {
        *self
    }
}

/// One rule scheduled into a round, with the delta its positive join is decomposed over.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RuleEntry<'r, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The rule's index in authored program order.
    pub(crate) index: usize,
    /// The rule.
    pub(crate) rule: &'r DlClause<N>,
    /// Its store-independent join plan.
    pub(crate) plan: &'r RulePlan<N>,
    /// Its lowered static shapes.
    pub(crate) runtime: &'r RuleRuntime<N>,
    /// The rows this evaluation treats as new.
    pub(crate) delta: Delta,
}

/// How one round schedules its immutable per-rule candidate work.
///
/// Both policies feed the SAME program-order merge and the same sorted commit, so
/// scheduling cannot affect an answer or a budget observation — which is exactly what
/// `sequential_and_parallel_rounds_agree` asserts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoundExecution {
    /// Evaluate rules into independent buffers, then merge them in program order.
    ///
    /// The default policy. It degrades to the direct in-order path for a
    /// single-rule round, for a round holding a guarded rule (a guard runs on the calling
    /// thread — see [`crate::guard`]) and for a one-worker pool — which is every `wasm32`
    /// build.
    #[default]
    Parallel,
    /// Force the direct in-order path even where rayon would be used.
    ///
    /// Selectable per call through [`EvalOptions::with_round_execution`]. The
    /// same kernels and merge order run without activating a global worker pool.
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
#[cfg(test)]
fn evaluate_rule<'r, D: TermDictionary, const N: usize>(
    entry: RuleEntry<'r, N>,
    snapshot: RoundSnapshot<'_, D, N>,
    strategy: JoinStrategy,
    governor: StepGovernor<'_>,
    guards: &dyn GuardEvaluator<D, N>,
    full_counts: Option<&[Option<usize>]>,
) -> Result<RoundBuffer<'r, D::Id, N>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    evaluate_rule_with_evidence(
        entry,
        snapshot,
        strategy,
        governor,
        guards,
        None,
        full_counts,
    )
}

/// Apply an authored guard range through the one production callback/filter path.
#[cfg(test)]
fn apply_body_guards<D: TermDictionary, const N: usize>(
    entry: RuleEntry<'_, N>,
    rel: &RelationStore<D, N>,
    solutions: Vec<SlotSolution<D::Id, N>>,
    positions: std::ops::Range<usize>,
    guards: &dyn GuardEvaluator<D, N>,
    governor: &mut StepGovernor<'_>,
) -> Result<Vec<SlotSolution<D::Id, N>>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    let mut frame = guards::Body::new(positions, solutions);
    frame.advance(entry, rel, guards, governor)?;
    debug_assert!(frame.is_complete() || governor.spent());
    Ok(frame.into_output())
}

/// Join authored positive segments, applying each guard before later probes.
/// Binary and certified cyclic segments use the existing join kernels.
#[cfg(test)]
fn join_ordered_body<D: TermDictionary, const N: usize>(
    entry: RuleEntry<'_, N>,
    snapshot: JoinSnapshot<'_, D, N>,
    strategy: JoinStrategy,
    guards: &dyn GuardEvaluator<D, N>,
    governor: &mut StepGovernor<'_>,
) -> Result<Vec<SlotSolution<D::Id, N>>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    let mut frame = ordered::Frame::new(entry, snapshot, strategy)?;
    frame.advance(entry, snapshot, guards, governor)?;
    debug_assert!(frame.is_complete() || governor.spent());
    Ok(frame.into_output())
}

#[cfg(test)]
fn evaluate_rule_with_evidence<'r, D: TermDictionary, const N: usize>(
    entry: RuleEntry<'r, N>,
    snapshot: RoundSnapshot<'_, D, N>,
    strategy: JoinStrategy,
    mut governor: StepGovernor<'_>,
    guards: &dyn GuardEvaluator<D, N>,
    mut evidence: Option<&mut dyn evidence::RuleEvidence<D, N>>,
    full_counts: Option<&[Option<usize>]>,
) -> Result<RoundBuffer<'r, D::Id, N>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    let owned_indexes = if snapshot.delta_indexes.is_none() {
        Some(DeltaIndexes::build(snapshot.rel, [entry.delta])?)
    } else {
        None
    };
    let snapshot = RoundSnapshot {
        delta_indexes: snapshot.delta_indexes.or(owned_indexes.as_ref()),
        ..snapshot
    };
    let owned_counts = if full_counts.is_none() {
        Some(
            constant_pattern_counts(entry.plan, snapshot.rel, entry.delta)
                .map_err(EvalError::Source)?,
        )
    } else {
        None
    };
    let full_counts = full_counts
        .or(owned_counts.as_deref())
        .expect("the rule's metadata advice was prepared");
    let mut frame = rule::Frame::new(entry, snapshot, strategy, full_counts)?;
    frame.advance(entry, snapshot, guards, &mut governor, &mut evidence)?;
    debug_assert!(frame.is_complete() || governor.spent());
    let mut buffer = frame.into_buffer();
    buffer.join_steps = governor.consumed;
    Ok(buffer)
}

/// Original call-stack phase ordering, retained only as a differential rule oracle.
#[cfg(test)]
fn reference_evaluate_rule<'r, D: TermDictionary, const N: usize>(
    entry: RuleEntry<'r, N>,
    snapshot: RoundSnapshot<'_, D, N>,
    strategy: JoinStrategy,
    mut governor: StepGovernor<'_>,
    guards: &dyn GuardEvaluator<D, N>,
    mut evidence: Option<&mut dyn evidence::RuleEvidence<D, N>>,
    full_counts: &[Option<usize>],
) -> Result<RoundBuffer<'r, D::Id, N>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    snapshot
        .rel
        .interner()
        .check_read()
        .map_err(EvalError::Source)?;
    let result = (|| {
        let (plan, runtime, rel) = (entry.plan, entry.runtime, snapshot.rel);
        if entry.delta != Delta::all(rel.row_slots())
            && entry.rule.guard_positions().is_none()
            && plan.operators().len() > 1
            && !entry.rule.reads_base()
            && full_counts.contains(&Some(0))
        {
            return Ok(RoundBuffer::new());
        }
        if entry.rule.guard_positions().is_none()
            && runtime.factors.len() > 1
            && matches!(strategy, JoinStrategy::Planned)
        {
            return factors::evaluate_with_evidence(entry, snapshot, &mut governor, evidence);
        }
        let join_snapshot = JoinSnapshot {
            seed: None,
            rel,
            delta: entry.delta,
            indexes: snapshot.delta_indexes.expect("delta indexes were prepared"),
        };
        let mut solutions = if entry.rule.guard_positions().is_some() {
            join_ordered_body(entry, join_snapshot, strategy, guards, &mut governor)?
        } else {
            let solutions = join_body(
                plan,
                join_snapshot,
                strategy,
                &mut governor,
                Some(full_counts),
            )
            .map_err(EvalError::Source)?;
            apply_body_guards(
                entry,
                rel,
                solutions,
                0..runtime.guards.len(),
                guards,
                &mut governor,
            )?
        };

        if !runtime.negated.is_empty() || !runtime.negations.is_empty() {
            let mut kept = Vec::with_capacity(solutions.len());
            for mut solution in solutions {
                if governor.spent() {
                    break;
                }
                let mut blocked = false;
                for atom in &runtime.negated {
                    if governor.spent()
                        || atom.blocks(&mut solution, rel, &mut governor, &mut evidence)?
                    {
                        blocked = true;
                        break;
                    }
                }
                if blocked {
                    continue;
                }
                for negation in &runtime.negations {
                    let (holds, negative) = negation.blocks(
                        GroupProbe {
                            solution: &solution,
                            rel,
                            guards,
                            rule: entry.index,
                            clause: entry.rule,
                        },
                        &mut governor,
                        &mut evidence,
                    )?;
                    if let (Some(id), Some(owner)) = (negative, evidence.as_deref_mut()) {
                        solution.retain_negative(id, owner)?;
                    }
                    if holds {
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
            if governor.spent() {
                break;
            }
            for (position, head) in runtime.head.iter().enumerate() {
                // The terminal positive/guard/identity row pays for its first head.
                // Every additional conjunct owns another candidate/proof expansion.
                if governor.spent() || (position != 0 && !governor.charge()) {
                    break;
                }
                emit_solution(
                    &mut buffer,
                    head,
                    &solution,
                    entry.index,
                    snapshot,
                    &mut evidence,
                    HeadEvidence {
                        head: position,
                        witness: None,
                        uses_absence: !runtime.negated.is_empty() || !runtime.negations.is_empty(),
                    },
                )?;
            }
        }
        buffer.join_steps = governor.consumed;
        Ok(buffer)
    })()?;
    snapshot
        .rel
        .interner()
        .check_read()
        .map_err(EvalError::Source)?;
    Ok(result)
}

/// The witness and absence dependency of one emitted head.
#[derive(Clone, Copy)]
struct HeadEvidence {
    head: usize,
    witness: Option<purrdf_core::evidence::EvidenceId>,
    uses_absence: bool,
}

/// Emit one head using the shared ordinary and factorized provenance path.
fn emit_solution<'r, D: TermDictionary, const N: usize>(
    buffer: &mut RoundBuffer<'r, D::Id, N>,
    head: &'r [ArgShape; N],
    solution: &SlotSolution<D::Id, N>,
    rule: usize,
    snapshot: RoundSnapshot<'_, D, N>,
    evidence: &mut Option<&mut dyn evidence::RuleEvidence<D, N>>,
    head_evidence: HeadEvidence,
) -> Result<(), EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    let rel = snapshot.rel;
    rel.interner().check_read().map_err(EvalError::Source)?;
    let (proof_height, sum_source_height) = source_heights(&solution.sources, snapshot.depth);
    let key = HeadKey {
        positions: head.each_ref().map(|term| head_term(term, solution)),
    };
    let key = intern_key(key, rel).map_err(EvalError::Source)?;
    if let Some((head, terms)) = snapshot.probe.and_then(|probe| probe.target)
        && (head != head_evidence.head || key.terms().map(HeadTerm::interned) != terms.map(Some))
    {
        return Ok(());
    }
    if let Some(owner) = evidence.as_mut() {
        let mut witness = match head_evidence.witness {
            Some(witness) => witness,
            None => owner.conjunction(&solution.sources)?,
        };
        if let Some(negative) = solution.negative_evidence {
            witness = owner.combine(
                witness,
                negative,
                purrdf_core::evidence::EvidenceCombination::Conjunction,
            )?;
        }
        owner.emit(&key, witness, head_evidence.uses_absence)?;
    }
    // A fact a prior round or stratum already derived is not a derivation: earlier
    // wins, exactly as the reference fixpoint decides it. Every one of the four
    // positions must already be interned for the quad to be present.
    if snapshot.probe.is_none()
        && let Some(row) = present_row(&key, rel).map_err(EvalError::Source)?
    {
        if snapshot.assumed.contains(&row) {
            let confirmation = Confirmation {
                derivation: Derivation {
                    fact: key.fact(rel).map_err(EvalError::Source)?,
                    rule,
                    head: head_evidence.head,
                    sources: solution
                        .sources
                        .iter()
                        .map(|s| s.fact(rel))
                        .collect::<Result<_, _>>()
                        .map_err(EvalError::Source)?,
                    proof_height: proof_height.saturating_add(1),
                },
                sum_source_height,
            };
            match buffer.confirmed.get_mut(&row) {
                Some(existing) => existing.merge(confirmation),
                None => {
                    buffer.confirmed.insert(row, confirmation);
                }
            }
        }
        return Ok(());
    }
    buffer
        .insert(
            key,
            Candidate {
                rule,
                head: head_evidence.head,
                sources: solution.sources.clone(),
                proof_height: proof_height.saturating_add(1),
                sum_source_height,
            },
            rel,
        )
        .map_err(EvalError::Source)?;
    rel.interner().check_read().map_err(EvalError::Source)?;
    Ok(())
}

/// The store row of the fact `key` names, if it is present.
fn present_row<D: TermDictionary, const N: usize>(
    key: &HeadKey<'_, D::Id, N>,
    rel: &RelationStore<D, N>,
) -> Result<Option<RowId>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    rel.interner().check_read()?;
    let result = (|| {
        let positions = key.terms().map(HeadTerm::interned);
        let (subject, object) = (positions[0]?, positions[2]?);
        let partition = rel.partition_for(&positions)?;
        let mut cursor = partition.select(Bound::Both(subject, object));
        cursor.next().map(|(_, _, row)| row)
    })();
    rel.interner().check_read()?;
    Ok(result)
}

/// Key a head term the store DOES know by its id, whatever produced it.
///
/// A guard may compute a surface the store already holds, and a head constant may have
/// been interned since the plan was lowered; keying either as fresh would let one fact be
/// keyed two ways and committed twice.
fn intern_key<'r, D: TermDictionary, const N: usize>(
    key: HeadKey<'r, D::Id, N>,
    rel: &RelationStore<D, N>,
) -> Result<HeadKey<'r, D::Id, N>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    rel.interner().check_read()?;
    let resolve = |term: HeadTerm<'r, D::Id>| {
        Ok::<_, D::Error>(match term {
            HeadTerm::Fresh(surface) => rel
                .interner()
                .lookup_term(&surface)?
                .map_or(HeadTerm::Fresh(surface), HeadTerm::Interned),
            interned @ HeadTerm::Interned(_) => interned,
        })
    };
    let key = HeadKey {
        positions: crate::dictionary::try_map_row(key.positions, resolve)?,
    };
    rel.interner().check_read()?;
    Ok(key)
}

/// Lower one head argument to its round-key term.
///
/// # Panics
///
/// Panics if a head variable is unbound. Compilation refuses a rule whose head carries a
/// variable nothing in its body binds, and every positive atom binds all four of its
/// variable positions before the join completes (every guard output is bound by its guard),
/// so an unbound head slot here would be a contradiction in the range-restriction check.
fn head_term<'r, I: TermIdentity, const N: usize>(
    shape: &'r ArgShape,
    solution: &SlotSolution<I, N>,
) -> HeadTerm<'r, I>
where
    Arity<N>: StoreShape<N>,
{
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

/// The caller's guard and cancellation authority for one round.
pub(crate) struct RoundControl<'s, D: TermDictionary, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    /// The original caller's guard evaluator.
    pub(crate) guards: &'s dyn GuardEvaluator<D, N>,
    /// Cooperative cancellation, separate from the candidate allowance.
    pub(crate) stop: Option<&'s dyn StopSignal>,
}

impl<D: TermDictionary, const N: usize> Copy for RoundControl<'_, D, N> where Arity<N>: StoreShape<N>
{}
impl<D: TermDictionary, const N: usize> Clone for RoundControl<'_, D, N>
where
    Arity<N>: StoreShape<N>,
{
    fn clone(&self) -> Self {
        *self
    }
}

/// Evaluate every scheduled rule against the original frozen round snapshot.
/// Guarded rules retain caller-thread order; guard-free rules may run in parallel.
pub(crate) fn evaluate_round<'r, D: TermDictionary, const N: usize>(
    entries: &[RuleEntry<'r, N>],
    snapshot: RoundSnapshot<'_, D, N>,
    execution: RoundExecution,
    strategy: JoinStrategy,
    allowance: u64,
    control: RoundControl<'_, D, N>,
) -> Result<RoundBuffer<'r, D::Id, N>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    evaluate_round_with_evidence(
        entries, snapshot, execution, strategy, allowance, control, None,
    )
}

fn evaluate_round_with_evidence<'r, D: TermDictionary, const N: usize>(
    entries: &[RuleEntry<'r, N>],
    snapshot: RoundSnapshot<'_, D, N>,
    execution: RoundExecution,
    strategy: JoinStrategy,
    allowance: u64,
    control: RoundControl<'_, D, N>,
    mut evidence: Option<&mut dyn evidence::RuleEvidence<D, N>>,
) -> Result<RoundBuffer<'r, D::Id, N>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    let mut frame = round::Frame::new(entries, snapshot)?;
    frame.advance(
        snapshot,
        execution,
        strategy,
        allowance,
        control,
        &mut evidence,
    )?;
    debug_assert!(frame.is_complete() || frame.refused() || frame.stopped());
    let mut buffer = frame.into_buffer(snapshot.rel)?;
    buffer.stopped |= is_stopped(control.stop);
    Ok(buffer)
}

/// The original call-stack round is the differential scheduling oracle.
#[cfg(test)]
fn reference_evaluate_round<'r, D: TermDictionary, const N: usize>(
    entries: &[RuleEntry<'r, N>],
    snapshot: RoundSnapshot<'_, D, N>,
    execution: RoundExecution,
    strategy: JoinStrategy,
    allowance: u64,
    guards: &dyn GuardEvaluator<D, N>,
    mut evidence: Option<&mut dyn evidence::RuleEvidence<D, N>>,
) -> Result<RoundBuffer<'r, D::Id, N>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    // Every task sees the same pre-task access-path state, including on the
    // sequential policy. An earlier task's lazy index cannot change a later
    // task's join order or candidate charges.
    let full_counts: Vec<_> = entries
        .iter()
        .map(|entry| constant_pattern_counts(entry.plan, snapshot.rel, entry.delta))
        .collect::<Result<_, _>>()
        .map_err(EvalError::Source)?;
    let indexes = DeltaIndexes::build(snapshot.rel, entries.iter().map(|entry| entry.delta))?;
    let snapshot = RoundSnapshot {
        delta_indexes: Some(&indexes),
        ..snapshot
    };
    snapshot
        .rel
        .interner()
        .check_read()
        .map_err(EvalError::Source)?;
    let result = (|| -> Result<RoundBuffer<'r, D::Id, N>, EvalError<D::Error>> {
        let pool = StepGovernor::new(allowance);
        let guarded = entries.iter().any(|entry| entry.rule.is_guarded());
        let mut round = RoundBuffer::new();
        if evidence.is_some() || guarded || !execution.should_parallelize(entries.len()) {
            for (position, &entry) in entries.iter().enumerate() {
                let buffer = match evidence.as_mut() {
                    Some(owner) => evaluate_rule_with_evidence(
                        entry,
                        snapshot,
                        strategy,
                        pool.task(),
                        guards,
                        Some(&mut **owner),
                        Some(&full_counts[position]),
                    )?,
                    None => evaluate_rule(
                        entry,
                        snapshot,
                        strategy,
                        pool.task(),
                        guards,
                        Some(&full_counts[position]),
                    )?,
                };
                round
                    .merge_from(buffer, snapshot.rel)
                    .map_err(EvalError::Source)?;
                if pool.spent() {
                    break;
                }
            }
            round.join_steps = pool.observed();
            round.join_refused = pool.spent();
            return Ok(round);
        }

        // `par_iter` over a slice is INDEXED, so `collect::<Vec<_>>()` restores program order
        // regardless of completion order; the merge below then folds strictly in that order.
        // This indexed form is the ONLY parallelism this crate uses — see the module docs for
        // the unordered rayon adaptors it must never reach for. No rule here is guarded, so the
        // evaluator every task receives is the one that refuses every call.
        let buffers: Vec<_> = entries
            .par_iter()
            .enumerate()
            .map(|(position, &entry)| {
                evaluate_rule(
                    entry,
                    snapshot,
                    strategy,
                    pool.task(),
                    &NoGuards,
                    Some(&full_counts[position]),
                )
            })
            .collect();

        for buffer in buffers {
            round
                .merge_from(buffer?, snapshot.rel)
                .map_err(EvalError::Source)?;
        }
        round.join_steps = pool.observed();
        round.join_refused = pool.spent();
        Ok(round)
    })()?;
    snapshot
        .rel
        .interner()
        .check_read()
        .map_err(EvalError::Source)?;
    Ok(result)
}

// ── The fixpoint ────────────────────────────────────────────────────────────────

/// The mutable working set carried across every stratum of one evaluation.
#[derive(Debug)]
pub(crate) struct FixpointState<D: TermDictionary = TermInterner, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The accumulated store: the seeded EDB plus everything derived so far.
    pub(crate) rel: RelationStore<D, N>,
    /// Per-row proof heights, indexed by [`RowId`] and pushed in lockstep with the store.
    pub(crate) depth: Vec<u32>,
    /// Every committed derivation.
    pub(crate) derivations: Vec<Derivation<N>>,
    /// Native chosen proofs retained only by a maintained execution.
    native: Option<maintained::Supports<D::Id, N>>,
    /// Exact admitted native metadata across this execution's retained frames.
    native_bytes: usize,
    /// Explicit original dictionary retirement authority, owned with this model.
    release_term: Option<maintained::ReleaseTerm<D>>,
    /// Exported evidence requested by the caller.
    pub(crate) derivation_mode: DerivationMode,
    /// Per-call scheduling, also used by ordered execution.
    pub(crate) execution: RoundExecution,
    /// Unique input facts, before any schedule assumptions.
    input_facts: u64,
    /// Successful unique rule insertions.
    committed_facts: u64,
    /// Candidate solutions enumerated so far.
    pub(crate) join_steps: u64,
    /// A refused reservation, including an observation past u64::MAX.
    join_refused: bool,
    /// Term-generating rounds committed so far.
    pub(crate) term_generating_rounds: u64,
    /// The limits in force.
    limits: Limits,
    /// Caller-stated limits, retained for budget reporting.
    stated: u8,
    /// The distinct terms of the seeded store.
    input_terms: u64,
    /// The rules that generated a term in the latest term-generating round.
    generating_rules: BTreeSet<usize>,
}

impl<D: TermDictionary, const N: usize> FixpointState<D, N>
where
    Arity<N>: StoreShape<N>,
{
    /// A state over the seeded store `edb`, governed by `options`.
    pub(crate) fn seeded(edb: RelationStore<D, N>, options: EvalOptions) -> Self {
        Self::seeded_with_native(edb, options, false)
    }

    pub(super) fn seeded_with_native(
        edb: RelationStore<D, N>,
        options: EvalOptions,
        native: bool,
    ) -> Self {
        let sparse = native || edb.has_sparse_rows();
        let depth = if sparse {
            Vec::new()
        } else {
            vec![0u32; edb.retained_rows()]
        };
        let input_terms = edb.interner().term_count();
        let input_facts = edb.retained_live_rows() as u64;
        Self {
            rel: edb,
            depth,
            derivations: Vec::new(),
            native: sparse.then(maintained::Supports::native_heights),
            native_bytes: 0,
            release_term: None,
            derivation_mode: options.derivation_mode(),
            execution: options.round_execution(),
            input_facts,
            committed_facts: 0,
            join_steps: 0,
            join_refused: false,
            term_generating_rounds: 0,
            limits: Limits::of(&options, input_terms),
            stated: BudgetReport::stated_limits(&options),
            input_terms,
            generating_rules: BTreeSet::new(),
        }
    }

    pub(super) fn account_native_memory(&mut self, before: usize, after: usize) {
        self.native_bytes = if after >= before {
            self.native_bytes.checked_add(after - before)
        } else {
            self.native_bytes.checked_sub(before - after)
        }
        .expect("native metadata retains its original admitted layout counter");
    }

    /// The budget consumption observed so far.
    pub(crate) fn depths(&self) -> Depths<'_, D::Id, N> {
        self.native
            .as_ref()
            .filter(|native| native.uses_native_heights())
            .map_or_else(|| self.depth.as_slice().into(), Depths::Native)
    }

    /// Record one asserted row without tying its logical ID to a resident address.
    pub(crate) fn seed_height(&mut self, row: RowId) {
        if self
            .native
            .as_ref()
            .is_none_or(|native| !native.uses_native_heights())
        {
            assert_eq!(row.index(), self.depth.len() as u64);
            self.depth.push(0);
        }
    }

    /// The budget consumption observed so far.
    pub(crate) fn report(&self) -> BudgetReport {
        BudgetReport {
            join_steps: self.join_steps,
            stored_facts: self.rel.retained_rows(),
            flags: self.stated,
            term_arena_bytes: self.rel.term_bytes(),
            term_generating_rounds: self.term_generating_rounds,
            generated_terms: self
                .rel
                .interner()
                .term_count()
                .saturating_sub(self.input_terms),
            input_terms: self.input_terms,
            limits: self.limits,
        }
    }

    /// One round's shared remaining successful credits.
    pub(crate) fn allowance(&self) -> u64 {
        self.limits.join_steps.saturating_sub(self.join_steps)
    }

    /// Account for a completed round's work, commit its winners and check every ceiling.
    ///
    /// # Errors
    ///
    /// [`EvalError::BudgetExhausted`] when work or an individual fact admission
    /// passes a ceiling. Facts already admitted remain within the stated limit.
    pub(crate) fn absorb(
        &mut self,
        round: RoundBuffer<'_, D::Id, N>,
    ) -> Result<(), EvalError<D::Error>> {
        self.join_steps = self.join_steps.saturating_add(round.join_steps);
        self.join_refused |= round.join_refused;
        check_budget(self)?;
        if round.stopped {
            return Err(EvalError::Stopped {
                report: self.report(),
            });
        }
        if round.entries.is_empty() {
            return Ok(());
        }
        commit_round(round, self)?;
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
/// This entry point uses [`EvalOptions::default`], with no logical stored-fact,
/// join-step or term-arena ceiling. [`EvalError::BudgetExhausted`] still refuses
/// unrepresentable work counts; a budget refusal never returns a partial answer.
/// [`EvalError::Guard`] if the program
/// carries a guard, which this entry point has no evaluator for — see
/// [`evaluate_guarded`].
pub fn evaluate<D: TermDictionary, const N: usize>(
    exe: &Executable<N>,
    edb: RelationStore<D, N>,
) -> Result<Evaluation<D, N>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
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
pub fn evaluate_until<D: TermDictionary, const N: usize>(
    exe: &Executable<N>,
    edb: RelationStore<D, N>,
    stop: Option<&dyn StopSignal>,
) -> Result<Evaluation<D, N>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
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
pub fn evaluate_guarded<D: TermDictionary, const N: usize>(
    exe: &Executable<N>,
    edb: RelationStore<D, N>,
    guards: &dyn GuardEvaluator<D, N>,
    options: &EvalOptions,
    stop: Option<&dyn StopSignal>,
) -> Result<Evaluation<D, N>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    evaluate_with(
        exe,
        edb,
        stop,
        guards,
        *options,
        options.round_execution(),
        JoinStrategy::Planned,
    )
}

/// The policy-selectable implementation behind [`evaluate`].
///
/// Scheduling is caller-selected; the join strategy remains planner-owned.
/// Tests use the private join override to prove both kernels agree.
fn evaluate_with<D: TermDictionary, const N: usize>(
    exe: &Executable<N>,
    edb: RelationStore<D, N>,
    stop: Option<&dyn StopSignal>,
    guards: &dyn GuardEvaluator<D, N>,
    options: EvalOptions,
    execution: RoundExecution,
    strategy: JoinStrategy,
) -> Result<Evaluation<D, N>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    evaluate_with_evidence(
        exe,
        edb,
        options,
        StratumRun {
            stop,
            guards,
            execution,
            strategy,
        },
        &mut None,
    )
}

/// A completed fact store and its original aligned evidence owner.
///
/// The evidence is retained with the result; query-local ids never escape their
/// dictionary, and a physical failure returns no completed model.
#[derive(Debug)]
pub struct EvidencedEvaluation<
    S: purrdf_lex::allocation::Admission,
    D: TermDictionary = TermInterner,
    const N: usize = 4,
> where
    Arity<N>: StoreShape<N>,
{
    /// The ordinary deterministic fact and preferred-provenance result.
    pub evaluation: Evaluation<D, N>,
    /// All independent row witnesses and their original admission.
    pub evidence: crate::evidence::DerivationEvidence<S>,
}

/// Evaluate through the original semi-naive engine with admitted row evidence.
///
/// Seed evidence must align with the actual input store. Default rows in a
/// complete, explicitly closed scope use the original execution path without
/// allocating a row-id array, record table or witness-capture buffer.
///
/// # Errors
/// Returns the original execution failure or native evidence refusal.
pub fn evaluate_evidenced<S: purrdf_lex::allocation::Admission, D: TermDictionary, const N: usize>(
    exe: &Executable<N>,
    edb: RelationStore<D, N>,
    options: &EvalOptions,
    guards: &dyn GuardEvaluator<D, N>,
    stop: Option<&dyn StopSignal>,
    mut rows: crate::evidence::DerivationEvidence<S>,
    scope: NegationScope,
) -> Result<EvidencedEvaluation<S, D, N>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    edb.interner().check_read().map_err(EvalError::Source)?;
    if rows.row_count() as u64 != edb.row_slots() {
        return Err(EvalError::Evidence(
            purrdf_core::evidence::EvidenceWorkspaceError::Column(
                purrdf_core::evidence::EvidenceColumnError::RowOutOfBounds,
            ),
        ));
    }
    if rows.rows().is_uniform_default() && scope.completeness.admits_absence(scope.closure_declared)
    {
        let evaluation = evaluate_with(
            exe,
            edb,
            stop,
            guards,
            *options,
            options.round_execution(),
            JoinStrategy::Planned,
        )?;
        for (row, _) in evaluation.facts.rows_from(rows.row_count() as u64) {
            rows.push_derived(row, purrdf_core::evidence::EvidenceId::DEFAULT)
                .map_err(EvalError::Evidence)?;
        }
        return Ok(EvidencedEvaluation {
            evaluation,
            evidence: rows,
        });
    }
    let mut capture = evidence::Capture::new(rows, scope);
    let evaluation = evaluate_with_evidence(
        exe,
        edb,
        *options,
        StratumRun {
            stop,
            guards,
            execution: options.round_execution(),
            strategy: JoinStrategy::Planned,
        },
        &mut Some(&mut capture),
    )?;
    Ok(EvidencedEvaluation {
        evaluation,
        evidence: capture.rows,
    })
}

fn evaluate_with_evidence<D: TermDictionary, const N: usize>(
    exe: &Executable<N>,
    edb: RelationStore<D, N>,
    options: EvalOptions,
    run: StratumRun<'_, D, N>,
    evidence: &mut Option<&mut dyn evidence::RuleEvidence<D, N>>,
) -> Result<Evaluation<D, N>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    if let Some(rule) = (0..exe.rule_count()).find(|&index| exe.rule_entry(index).0.reads_base()) {
        return Err(EvalError::BaseReadRequiresSchedule { rule });
    }
    edb.interner().check_read().map_err(EvalError::Source)?;
    let mut state = FixpointState::seeded(edb, options);
    check_budget(&state)?;

    for stratum in 0..exe.stratum_count() {
        if exe.stratum_is_empty(stratum) {
            continue;
        }
        let owner = evidence
            .as_mut()
            .map(|owner| &mut **owner as &mut dyn evidence::RuleEvidence<D, N>);
        run_stratum(exe, exe.runtimes(), stratum, &mut state, run, owner)?;
    }

    state
        .rel
        .interner()
        .check_read()
        .map_err(EvalError::Source)?;
    Ok(state.finish())
}

impl<D: TermDictionary, const N: usize> FixpointState<D, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Seal the state into an [`Evaluation`].
    ///
    /// Derivations are produced in per-round lexical order; sorting the whole vector makes
    /// that one total order across strata as well, so the output is a pure function of the
    /// program and the facts rather than of the stratum boundaries.
    pub(crate) fn finish(mut self) -> Evaluation<D, N> {
        self.derivations.sort();
        Evaluation {
            budget: self.report(),
            work: WorkReport {
                input_facts: self.input_facts,
                committed_facts: self.committed_facts,
                join_steps: self.join_steps,
            },
            facts: self.rel,
            derivations: (self.derivation_mode == DerivationMode::Record)
                .then_some(self.derivations),
        }
    }
}

/// Whether any ceiling is already passed.
pub(crate) fn check_budget<D: TermDictionary, const N: usize>(
    state: &FixpointState<D, N>,
) -> Result<(), EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    state
        .rel
        .interner()
        .check_read()
        .map_err(EvalError::Source)?;
    let report = state.report();
    if state.join_refused || report.join_steps > report.limits.join_steps {
        return Err(EvalError::BudgetExhausted {
            resource: BudgetResource::JoinSteps,
            report,
        });
    }
    if u64::try_from(report.stored_facts).unwrap_or(u64::MAX) > report.limits.stored_facts {
        return Err(EvalError::BudgetExhausted {
            resource: BudgetResource::StoredFacts,
            report,
        });
    }
    if report.term_arena_bytes as u64 > report.limits.term_arena_bytes {
        return Err(EvalError::BudgetExhausted {
            resource: BudgetResource::TermArenaBytes,
            report,
        });
    }
    for resource in [
        BudgetResource::TermGeneratingRounds,
        BudgetResource::GeneratedTerms,
    ] {
        if resource.observed(report) > resource.limit(report) {
            return Err(EvalError::TermLimitExceeded {
                resource,
                rules: state.generating_rules.iter().copied().collect(),
                report,
            });
        }
    }
    Ok(())
}

/// The per-evaluation policies one stratum runs under.
struct StratumRun<'a, D: TermDictionary, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The caller's stop signal.
    stop: Option<&'a dyn StopSignal>,
    /// The caller's guard evaluator.
    guards: &'a dyn GuardEvaluator<D, N>,
    /// The round scheduling policy.
    execution: RoundExecution,
    /// The join-kernel policy.
    strategy: JoinStrategy,
}

impl<D: TermDictionary, const N: usize> Copy for StratumRun<'_, D, N> where Arity<N>: StoreShape<N> {}
impl<D: TermDictionary, const N: usize> Clone for StratumRun<'_, D, N>
where
    Arity<N>: StoreShape<N>,
{
    fn clone(&self) -> Self {
        *self
    }
}

/// Run one stratum's semi-naive fixpoint into `state`.
fn run_stratum<D: TermDictionary, const N: usize>(
    exe: &Executable<N>,
    runtimes: &[RuleRuntime<N>],
    stratum: usize,
    state: &mut FixpointState<D, N>,
    run: StratumRun<'_, D, N>,
    mut evidence: Option<&mut dyn evidence::RuleEvidence<D, N>>,
) -> Result<(), EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    let mut frame = stratum::Frame::new(exe, runtimes, stratum, state);
    frame.advance(state, run, &mut evidence)?;
    debug_assert!(frame.is_complete());
    Ok(())
}

/// The original call-stack fixpoint is the differential stratum oracle.
#[cfg(test)]
fn reference_run_stratum<D: TermDictionary, const N: usize>(
    exe: &Executable<N>,
    runtimes: &[RuleRuntime<N>],
    stratum: usize,
    state: &mut FixpointState<D, N>,
    run: StratumRun<'_, D, N>,
    mut evidence: Option<&mut dyn evidence::RuleEvidence<D, N>>,
) -> Result<(), EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    // Seed the delta with EVERY accumulated row, so this stratum's rules fire against the
    // whole accumulated store in round one. Row ids are dense, so "everything" is the
    // contiguous span `[0, row_count)` — no per-key materialisation and no bitset.
    let mut delta = Delta::all(state.rel.row_slots());
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
        let entries: Vec<RuleEntry<'_, N>> = exe
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
        let owner = evidence
            .as_mut()
            .map(|owner| &mut **owner as &mut dyn evidence::RuleEvidence<D, N>);
        let round = evaluate_round_with_evidence(
            &entries,
            RoundSnapshot {
                probe: None,
                rel: &state.rel,
                depth: state.depths(),
                assumed: &purrdf_core::FastSet::default(),
                delta_indexes: None,
            },
            run.execution,
            run.strategy,
            state.allowance(),
            RoundControl {
                guards: run.guards,
                stop: None,
            },
            owner,
        )?;
        if round.is_empty() {
            state.join_steps = state.join_steps.saturating_add(round.join_steps);
            state.join_refused |= round.join_refused;
            check_budget(state)?;
            if let Some(owner) = evidence.as_mut()
                && owner.commit(&state.rel)?
            {
                delta = Delta::all(state.rel.row_slots());
                continue;
            }
            return Ok(()); // fact and evidence fixpoint
        }

        let round_lo = state.rel.row_slots();
        state.absorb(round)?;
        if let Some(owner) = evidence.as_mut()
            && owner.commit(&state.rel)?
        {
            delta = Delta::all(state.rel.row_slots());
            continue;
        }

        // The next round's delta is exactly the rows committed this round — a contiguous
        // span, because the commit loop mints row ids densely in one sorted pass.
        delta = Delta {
            lo: round_lo,
            hi: state.rel.row_slots(),
        };
    }
}

/// Commit a round's winners in lexical `(subject, predicate, object, graph)` order.
///
/// Lexical — never mint order — so store insertion order, row-id assignment and the
/// derivation sequence are all byte-deterministic. Row-id assignment is a purely additive
/// side effect of this sorted loop; it never orders the commit.
fn commit_round<D: TermDictionary, const N: usize>(
    round: RoundBuffer<'_, D::Id, N>,
    state: &mut FixpointState<D, N>,
) -> Result<(), EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    let mut frame = commit::Frame::new(round, state)?;
    frame.advance(state)?;
    debug_assert!(frame.is_complete());
    Ok(())
}

/// The original publication loop is a differential commit oracle.
#[cfg(test)]
fn reference_commit_round<D: TermDictionary, const N: usize>(
    round: RoundBuffer<'_, D::Id, N>,
    state: &mut FixpointState<D, N>,
) -> Result<(), EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    let mut winners: Vec<_> = round
        .entries
        .into_iter()
        .map(|(key, candidate)| {
            let fact = if state.derivation_mode == DerivationMode::Record {
                Some(key.fact(&state.rel)?)
            } else {
                None
            };
            Ok::<_, D::Error>((key, candidate, fact))
        })
        .collect::<Result<_, _>>()
        .map_err(EvalError::Source)?;
    // Head facts are unique in a round buffer. The dictionary supplies the
    // canonical order without requiring a lexical projection of model-only rows.
    purrdf_lex::walk::try_sort_unstable_by(&mut winners, |(left, _, _), (right, _, _)| {
        state.rel.interner().compare_rows(
            &left.terms().map(HeadTerm::as_term),
            &right.terms().map(HeadTerm::as_term),
        )
    })
    .map_err(EvalError::Source)?;

    for (key, candidate, fact) in winners {
        state
            .report()
            .admit_fact()
            .map_err(|report| EvalError::BudgetExhausted {
                resource: BudgetResource::StoredFacts,
                report,
            })?;
        let sources = if state.derivation_mode == DerivationMode::Record {
            Some(
                candidate
                    .source_facts(&state.rel)
                    .map_err(EvalError::Source)?,
            )
        } else {
            None
        };
        let terms = key.terms().map(|term| match term {
            HeadTerm::Interned(id) => DictionaryTerm::Id(*id),
            HeadTerm::Fresh(surface) => DictionaryTerm::Surface(surface.as_ref()),
        });
        let inserted = state.rel.insert_terms(terms).map_err(|error| match error {
            DictionaryError::Source(error) => EvalError::Source(error),
            DictionaryError::Storage(error) => EvalError::Storage(error),
        })?;
        let (_, _, row) = inserted.expect(
            "a round winner is absent from the store by construction, so it inserts a new row",
        );
        debug_assert_eq!(
            row.resident_index(),
            state.depth.len(),
            "depth tracks store rows"
        );
        state.depth.push(candidate.proof_height);
        state.committed_facts = state.committed_facts.saturating_add(1);
        if let Some(sources) = sources {
            state.derivations.push(Derivation {
                fact: fact.expect("recorded winners project their facts before publication"),
                rule: candidate.rule,
                head: candidate.head,
                sources,
                proof_height: candidate.proof_height,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::store::Fact;
    use std::collections::BTreeSet;

    use super::*;
    use crate::clause::{ClauseAtom, HeadDisjunct};
    use crate::synth_corpus::{self, SynthWorkload};
    use crate::test_support::evidence_scope;
    use crate::test_support::permute;

    #[test]
    fn impossible_constant_patterns_refuse_before_broad_delta_seeding() {
        let broad = ClauseAtom::triple(
            ClauseTerm::var("?s"),
            ClauseTerm::var("?p"),
            ClauseTerm::var("?o"),
        );
        let declared = ClauseAtom::triple(
            ClauseTerm::var("?p"),
            ClauseTerm::literal("declared"),
            ClauseTerm::literal("missing"),
        );
        let head = ClauseAtom::triple(
            ClauseTerm::var("?s"),
            ClauseTerm::literal("out"),
            ClauseTerm::var("?o"),
        );
        let program = compile(vec![DlClause::datalog(head, vec![broad, declared])]).unwrap();
        let (rule, plan) = program.rule_entry(0);
        let runtime = RuleRuntime::new(rule, plan);
        let mut store = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
        for row in [
            ["p", "declared", "active"],
            ["missing", "exists", "yes"],
            ["a", "p", "b"],
        ] {
            store.insert_row(row).unwrap();
        }
        let depth = vec![0; store.row_count()];
        let pool = StepGovernor::new(0);
        let result = evaluate_rule(
            RuleEntry {
                index: 0,
                rule,
                plan,
                runtime: &runtime,
                delta: Delta { lo: 2, hi: 3 },
            },
            RoundSnapshot {
                probe: None,
                rel: &store,
                depth: depth.as_slice().into(),
                assumed: &purrdf_core::FastSet::default(),
                delta_indexes: None,
            },
            JoinStrategy::Planned,
            pool.task(),
            &NoGuards,
            None,
        )
        .unwrap();
        assert!(result.is_empty());
        assert_eq!(pool.observed(), 0);
        assert!(
            !pool.spent(),
            "an impossible indexed pattern needs no candidate admission"
        );
    }

    #[test]
    fn round_work_does_not_depend_on_another_tasks_index_warming() {
        let atom = |subject: &str, predicate: &str, object: &str| {
            ClauseAtom::triple(
                if subject.starts_with('?') {
                    ClauseTerm::var(subject)
                } else {
                    ClauseTerm::literal(subject)
                },
                ClauseTerm::literal(predicate),
                if object.starts_with('?') {
                    ClauseTerm::var(object)
                } else {
                    ClauseTerm::literal(object)
                },
            )
        };
        let program = compile(vec![
            DlClause::datalog(
                atom("?s", "out", "?tag"),
                vec![atom("?s", "p", "needle"), atom("?s", "q", "?tag")],
            ),
            DlClause::datalog(
                atom("?s", "warmed", "needle"),
                vec![atom("?s", "p", "needle")],
            ),
        ])
        .unwrap();
        let (target, target_plan) = program.rule_entry(0);
        let (warmer, warmer_plan) = program.rule_entry(1);
        let target_runtime = RuleRuntime::new(target, target_plan);
        let warmer_runtime = RuleRuntime::new(warmer, warmer_plan);
        let mut original =
            RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
        for index in 0..1000 {
            let subject = format!("s{index}");
            let object = if index == 0 {
                "needle".to_owned()
            } else {
                format!("other{index}")
            };
            original.insert_row([&subject, "p", &object]).unwrap();
        }
        let lo = original.row_count();
        for index in 0..5 {
            original
                .insert_row([&format!("s{index}"), "q", "tag"])
                .unwrap();
        }
        let mut observations = Vec::new();
        for warm_first in [false, true] {
            let store = original.clone();
            let target = RuleEntry {
                index: 0,
                rule: target,
                plan: target_plan,
                runtime: &target_runtime,
                delta: Delta {
                    lo: lo as u64,
                    hi: store.row_count() as u64,
                },
            };
            let warmer = RuleEntry {
                index: 1,
                rule: warmer,
                plan: warmer_plan,
                runtime: &warmer_runtime,
                delta: Delta::all(store.row_slots()),
            };
            let entries = if warm_first {
                [warmer, target]
            } else {
                [target, warmer]
            };
            let round = evaluate_round(
                &entries,
                RoundSnapshot {
                    probe: None,
                    rel: &store,
                    depth: vec![0; store.row_count()].as_slice().into(),
                    assumed: &purrdf_core::FastSet::default(),
                    delta_indexes: None,
                },
                RoundExecution::Sequential,
                JoinStrategy::Planned,
                u64::MAX,
                RoundControl {
                    guards: &NoGuards,
                    stop: None,
                },
            )
            .unwrap();
            observations.push((
                round.join_steps,
                round.entries.keys().cloned().collect::<Vec<_>>(),
            ));
        }
        assert_eq!(
            observations[0], observations[1],
            "task order must not select a different physical join"
        );
    }

    #[test]
    fn partial_delta_cursors_never_visit_the_old_label_history() {
        const LABEL: &str = "https://example.org/label";
        const EDGE: &str = "https://example.org/edge";
        let label = |subject: &str| {
            ClauseAtom::positive(
                ClauseTerm::var(subject),
                LABEL,
                ClauseTerm::iri("https://example.org/marked"),
            )
        };
        let program = compile(vec![DlClause::datalog(
            label("?y"),
            vec![
                label("?x"),
                ClauseAtom::positive(ClauseTerm::var("?x"), EDGE, ClauseTerm::var("?y")),
            ],
        )])
        .unwrap();
        let (_, plan) = program.rule_entry(0);
        for count in [100, 1000, 10_000] {
            let mut store = RelationStore::default();
            for index in 0..count {
                store
                    .insert(
                        &format!("old{index}"),
                        &format!("<{LABEL}>"),
                        "<https://example.org/marked>",
                        "",
                    )
                    .unwrap();
            }
            let edge = store
                .insert("frontier", &format!("<{EDGE}>"), "end", "")
                .unwrap()
                .unwrap()
                .2;
            let lo = store.row_count();
            let fresh = store
                .insert(
                    "frontier",
                    &format!("<{LABEL}>"),
                    "<https://example.org/marked>",
                    "",
                )
                .unwrap()
                .unwrap()
                .2;
            let delta = Delta {
                lo: lo as u64,
                hi: store.row_count() as u64,
            };
            let indexes =
                DeltaIndexes::build(&store, [delta, delta, Delta::all(store.row_slots())]).unwrap();
            assert_eq!(
                indexes.runs.len(),
                1,
                "rules share each partial run and borrow the all-row seed"
            );
            let mut governor = StepGovernor::new(10);
            let solutions = join_body(
                plan,
                JoinSnapshot {
                    seed: None,
                    rel: &store,
                    delta,
                    indexes: &indexes,
                },
                JoinStrategy::Planned,
                &mut governor,
                None,
            )
            .unwrap();
            assert_eq!(solutions.len(), 1);
            assert_eq!(
                governor.row_visits, 2,
                "cursor work must exclude old label rows"
            );
            assert_eq!(
                solutions[0]
                    .sources
                    .iter()
                    .map(|source| source.row)
                    .collect::<Vec<_>>(),
                [fresh, edge]
            );
        }
    }

    #[test]
    fn late_delta_anchor_binds_before_a_large_old_prefix() {
        for count in [100, 1000] {
            let mut store = RelationStore::default();
            for index in 0..count {
                store
                    .insert(
                        &format!("s{index}"),
                        "<https://example.org/p>",
                        &format!("m{index}"),
                        "",
                    )
                    .unwrap();
            }
            let lo = store.row_count();
            store
                .insert("m0", "<https://example.org/q>", "end", "")
                .unwrap();
            let atom = |subject: &str, predicate: &str, object: &str| {
                ClauseAtom::positive(ClauseTerm::var(subject), predicate, ClauseTerm::var(object))
            };
            let program = compile(vec![DlClause::datalog(
                atom("?s", "https://example.org/out", "?o"),
                vec![
                    atom("?s", "https://example.org/p", "?m"),
                    atom("?m", "https://example.org/q", "?o"),
                ],
            )])
            .unwrap();
            let (_, plan) = program.rule_entry(0);
            assert_eq!(
                plan.operators()
                    .iter()
                    .map(AtomOperator::body_index)
                    .collect::<Vec<_>>(),
                [0, 1]
            );
            let mut governor = StepGovernor::new(10);
            let delta = Delta {
                lo: lo as u64,
                hi: store.row_count() as u64,
            };
            let indexes = DeltaIndexes::build(&store, [delta]).unwrap();
            let solutions = join_body(
                plan,
                JoinSnapshot {
                    seed: None,
                    rel: &store,
                    delta,
                    indexes: &indexes,
                },
                JoinStrategy::ForcedBinary,
                &mut governor,
                Some(&constant_pattern_counts(plan, &store, delta).unwrap()),
            )
            .unwrap();
            assert_eq!(solutions.len(), 1);
            assert_eq!(
                governor.observed(),
                2,
                "old prefixes must not consume candidate credit"
            );
            assert_eq!(
                solutions[0]
                    .sources
                    .iter()
                    .map(|source| source.body_index)
                    .collect::<Vec<_>>(),
                [0, 1]
            );
        }
    }

    #[test]
    fn capacity_cause_stays_inline_without_inflating_the_operational_error() {
        use purrdf_lex::allocation::{CapacityError, CapacitySpace, StorageError};
        let cause = CapacityError::new(CapacitySpace::DatalogTerms, u128::MAX, u128::MAX - 1);
        let error: EvalError = EvalError::Storage(StorageError::Capacity(cause));
        let EvalError::Storage(StorageError::Capacity(retained)) = error else {
            panic!("the original capacity cause must survive unchanged");
        };
        assert_eq!(retained, cause);
        assert_eq!(retained.attempted(), u128::MAX);
        assert_eq!(retained.limit(), u128::MAX - 1);
        assert!(size_of::<EvalError>() < 128);
    }

    #[test]
    fn host_candidate_publication_uses_original_ids_and_lexical_witness_order() {
        use crate::test_support::{HostDictionary, SourceLost};
        use std::sync::atomic::{AtomicBool, AtomicUsize};
        const FIRST: u64 = 0xa000_0000_1234_5678;
        const P_ID: u64 = 7;
        const GRAPH: u64 = 1;
        let mut store = RelationStore::with_dictionary(HostDictionary {
            surfaces: [
                (FIRST, "aaa".into()),
                (0, "zzz".into()),
                (u64::MAX, "value".into()),
                (P_ID, surface(P)),
                (GRAPH, String::new()),
                (9, surface(R)),
                (10, surface("https://example.org/out")),
                (11, surface("https://example.org/value")),
            ]
            .into_iter()
            .collect(),
            resolves: AtomicUsize::new(0),
            lookups: AtomicUsize::new(0),
            lost: AtomicBool::new(false),
            lose_after_lookup: AtomicBool::new(false),
            pins: AtomicUsize::new(0),
            refuse_after_interned: AtomicBool::new(false),
            extra_terms: 0,
        });
        for subject in [0, FIRST] {
            store.insert_ids([subject, P_ID, u64::MAX, GRAPH]).unwrap();
        }
        let rule = DlClause::datalog(
            ClauseAtom::positive(
                iri("https://example.org/out"),
                R,
                iri("https://example.org/value"),
            ),
            vec![atom("?a", P, "?b")],
        );
        let executable = compile(vec![rule]).unwrap();
        let (rule, plan) = executable.rule_entry(0);
        let head = ArgShape::of_atom(rule.datalog_head().unwrap(), plan.variables());
        let before = store.interner().resolves.load(Ordering::Relaxed);
        let solutions = join_body(
            plan,
            JoinSnapshot {
                seed: None,
                rel: &store,
                delta: Delta::all(2),
                indexes: &DeltaIndexes { runs: Vec::new() },
            },
            JoinStrategy::Planned,
            &mut StepGovernor::new(u64::MAX),
            None,
        )
        .unwrap();
        assert_eq!(solutions.len(), 2);
        assert_eq!(store.interner().resolves.load(Ordering::Relaxed), before);
        for reverse in [false, true] {
            let mut round = RoundBuffer::new();
            for position in if reverse { [1, 0] } else { [0, 1] } {
                let solution = &solutions[position];
                let key = intern_key(
                    HeadKey {
                        positions: head.each_ref().map(|term| head_term(term, solution)),
                    },
                    &store,
                )
                .unwrap();
                assert_eq!(
                    key.terms().map(HeadTerm::interned),
                    [Some(10), Some(9), Some(11), Some(GRAPH)]
                );
                assert_eq!(present_row(&key, &store).unwrap(), None);
                let mut local = RoundBuffer::new();
                local
                    .insert(
                        key,
                        Candidate {
                            rule: 0,
                            head: 0,
                            sources: solution.sources.clone(),
                            proof_height: 1,
                            sum_source_height: 0,
                        },
                        &store,
                    )
                    .unwrap();
                round.merge_from(local, &store).unwrap();
            }
            assert_eq!(round.entries.len(), 1);
            let (key, winner) = round.entries.first_key_value().unwrap();
            assert_eq!(
                winner.sources[0].terms[0], FIRST,
                "lexical witness order is independent of host ID order and scheduling"
            );
            let published = key.fact(&store).unwrap();
            assert_eq!(published.subject, surface("https://example.org/out"));
            assert_eq!(published.predicate, surface(R));
            assert_eq!(winner.source_facts(&store).unwrap()[0].subject, "aaa");
            assert_eq!(store.interner().pins.load(Ordering::Relaxed), 0);
            let key = key.clone();
            let expected = winner.clone();
            let mut refused = expected.clone();
            refused.sources[0].terms[0] = 404;
            assert_eq!(
                round.insert(key.clone(), refused, &store),
                Err(SourceLost(404))
            );
            assert_eq!(round.entries[&key], expected);
            assert_eq!(store.interner().pins.load(Ordering::Relaxed), 0);
            store.interner().lost.store(true, Ordering::Relaxed);
            let empty = Candidate {
                sources: Vec::new(),
                ..expected
            };
            assert_eq!(empty.source_facts(&store), Err(SourceLost(99)));
            let mut better = expected.clone();
            better.proof_height = 0;
            assert_eq!(
                round.insert(key.clone(), better, &store),
                Err(SourceLost(99))
            );
            assert_eq!(round.entries[&key], expected);
            assert_eq!(present_row(&key, &store), Err(SourceLost(99)));
            assert_eq!(
                round.merge_from(RoundBuffer::new(), &store),
                Err(SourceLost(99))
            );
            store.interner().lost.store(false, Ordering::Relaxed);
            store
                .interner()
                .lose_after_lookup
                .store(true, Ordering::Relaxed);
            let mut fresh = key;
            fresh.positions[0] = HeadTerm::Fresh(Cow::Borrowed("missing"));
            assert_eq!(intern_key(fresh, &store), Err(SourceLost(99)));
            store.interner().lost.store(false, Ordering::Relaxed);
            store
                .interner()
                .lose_after_lookup
                .store(false, Ordering::Relaxed);
        }
        use evidence::RuleEvidence;
        use purrdf_core::evidence::{
            EvidenceRecord, Frame, Information, InformationMap, SearchCompleteness,
        };
        use purrdf_lex::allocation::Resident;
        let mut rows = crate::evidence::DerivationEvidence::with_admission(2, Resident);
        rows.seed(RowId::from_index(1).unwrap(), |memory| {
            EvidenceRecord::from_parts(
                &InformationMap::uniform(Information::Opposed),
                SearchCompleteness::Complete,
                &Frame::actual(),
                &[],
                &[],
                memory,
            )
        })
        .unwrap();
        let mut capture = evidence::Capture::new(rows, evidence_scope());
        let mut emitted = RoundBuffer::new();
        for solution in &solutions {
            emit_solution(
                &mut emitted,
                &head,
                solution,
                0,
                RoundSnapshot {
                    probe: None,
                    rel: &store,
                    depth: (&[0, 0][..]).into(),
                    assumed: &purrdf_core::FastSet::default(),
                    delta_indexes: None,
                },
                &mut Some(&mut capture),
                HeadEvidence {
                    head: 0,
                    witness: None,
                    uses_absence: false,
                },
            )
            .unwrap();
        }
        assert_eq!(emitted.entries.len(), 1);
        assert_eq!(
            emitted.entries.first_key_value().unwrap().1.sources[0].terms[0],
            FIRST
        );
        let (_, _, row) = store.insert_ids([10, 9, 11, GRAPH]).unwrap().unwrap();
        assert!(
            !capture.commit(&store).unwrap(),
            "a newly appended evidence row does not revise an old row"
        );
        assert_eq!(capture.rows.row_count(), 3);
        assert_eq!(
            capture.rows.record(row).unwrap().information().fallback(),
            Information::Both
        );
        assert_eq!(store.interner().pins.load(Ordering::Relaxed), 0);
        store.interner().lost.store(true, Ordering::Relaxed);
        assert_eq!(
            capture.commit(&store),
            Err(EvalError::Source(SourceLost(99))),
            "an empty pending evidence commit still validates the original source"
        );
    }

    #[test]
    fn host_group_guards_hold_lexical_pins_and_release_them_on_every_exit() {
        use crate::guard::{CallbackError, GuardReads};
        use crate::test_support::{HostDictionary, SourceLost};
        use std::sync::atomic::{AtomicBool, AtomicUsize};
        const P_ID: u64 = 7;
        const Q_ID: u64 = 9;
        const GRAPH: u64 = 0;
        const TRIPLE: u64 = 0xa000_0000_1234_5678;
        struct Guards {
            lose_source: bool,
            calls: AtomicUsize,
        }
        impl GuardEvaluator<HostDictionary> for Guards {
            fn evaluate(
                &self,
                call: &GuardCall<'_, HostDictionary>,
            ) -> Result<crate::guard::GuardRows<u64>, CallbackError<SourceLost>> {
                call.evaluate_lexical(|inputs| {
                    self.calls.fetch_add(1, Ordering::Relaxed);
                    if call.guard.name() == "produce" {
                        assert_eq!(inputs, ["triple", "member"]);
                        assert_eq!(call.model.interner().pins.load(Ordering::Relaxed), 2);
                        if self.lose_source {
                            call.model.interner().lost.store(true, Ordering::Relaxed);
                        }
                        Ok(vec![vec!["fresh".into()]])
                    } else {
                        assert_eq!(inputs, ["fresh"]);
                        assert_eq!(call.model.interner().pins.load(Ordering::Relaxed), 0);
                        Ok(vec![Vec::new()])
                    }
                })
            }
        }
        let rule = DlClause::datalog(atom("?a", R, "?b"), vec![atom("?a", P, "?b")])
            .with_negations(vec![Negation::new(
                vec![atom("?a", Q, "?local"), atom("?local", Q, "?local")],
                vec![
                    Guard::new(
                        "produce",
                        vec!["?a".into(), "?local".into()],
                        vec!["?fresh".into()],
                        GuardReads::Bindings,
                    ),
                    Guard::new(
                        "check",
                        vec!["?fresh".into()],
                        Vec::new(),
                        GuardReads::Bindings,
                    ),
                ],
            )]);
        let executable = compile(vec![rule]).unwrap();
        let (rule, plan) = executable.rule_entry(0);
        let group = NegationRuntime::new(0, &rule.negations()[0], rule, plan.variables());
        for (local, lose_source) in [(22, false), (23, false), (22, true)] {
            let mut store = RelationStore::with_dictionary(HostDictionary {
                surfaces: [
                    (P_ID, surface(P)),
                    (Q_ID, surface(Q)),
                    (GRAPH, String::new()),
                    (TRIPLE, "triple".into()),
                    (22, "member".into()),
                ]
                .into_iter()
                .collect(),
                resolves: AtomicUsize::new(0),
                lookups: AtomicUsize::new(0),
                lost: AtomicBool::new(false),
                lose_after_lookup: AtomicBool::new(false),
                pins: AtomicUsize::new(0),
                refuse_after_interned: AtomicBool::new(false),
                extra_terms: 0,
            });
            for row in [
                [TRIPLE, P_ID, u64::MAX, GRAPH],
                [TRIPLE, Q_ID, local, GRAPH],
                [local, Q_ID, local, GRAPH],
            ] {
                store.insert_ids(row).unwrap();
            }
            let mut solutions = join_body(
                plan,
                JoinSnapshot {
                    seed: None,
                    rel: &store,
                    delta: Delta::all(3),
                    indexes: &DeltaIndexes { runs: Vec::new() },
                },
                JoinStrategy::Planned,
                &mut StepGovernor::new(u64::MAX),
                None,
            )
            .unwrap();
            assert_eq!(solutions.len(), 1);
            let frame = solutions.pop().unwrap();
            let guards = Guards {
                lose_source,
                calls: AtomicUsize::new(0),
            };
            let probe = GroupProbe {
                solution: &frame,
                rel: &store,
                guards: &guards,
                rule: 0,
                clause: rule,
            };
            let result = group.blocks(probe, &mut StepGovernor::new(u64::MAX), &mut None);
            if local == 23 {
                assert_eq!(result, Err(EvalError::Source(SourceLost(23))));
                assert_eq!(guards.calls.load(Ordering::Relaxed), 0);
            } else if lose_source {
                assert_eq!(result, Err(EvalError::Source(SourceLost(99))));
                assert_eq!(guards.calls.load(Ordering::Relaxed), 1);
            } else {
                assert_eq!(result, Ok((true, None)));
                assert_eq!(guards.calls.load(Ordering::Relaxed), 2);
            }
            assert_eq!(
                store.interner().pins.load(Ordering::Relaxed),
                0,
                "all original lexical owners release after success or refusal"
            );
            store.interner().lost.store(true, Ordering::Relaxed);
            assert_eq!(
                group.holds(probe, &mut StepGovernor::new(0)),
                Err(EvalError::Source(SourceLost(99)))
            );
            store.interner().lost.store(false, Ordering::Relaxed);
            let mut binding = SlotSolution::empty(1);
            binding
                .bind_value(0, &DictionaryValue::from("triple"), &store)
                .unwrap();
            assert_eq!(binding.get(0), Some(TRIPLE));
            let before = binding.clone();
            store
                .interner()
                .lose_after_lookup
                .store(true, Ordering::Relaxed);
            assert_eq!(
                binding.bind_value(0, &DictionaryValue::from("unknown"), &store),
                Err(SourceLost(99))
            );
            assert_eq!(
                binding, before,
                "a refused lexical lookup never publishes a frame mutation"
            );
        }
    }

    #[test]
    fn native_guard_identity_flow_never_requests_lexical_values() {
        use crate::guard::{CallbackError, GuardRows};
        use crate::test_support::{HostDictionary, SourceLost};
        use std::sync::atomic::Ordering;
        const ORIGINAL: u64 = 0xf000_0000_0000_0001;
        const OUTPUT: u64 = u64::MAX;
        struct Identity;
        impl GuardEvaluator<HostDictionary> for Identity {
            fn evaluate(
                &self,
                call: &GuardCall<'_, HostDictionary>,
            ) -> Result<GuardRows<u64>, CallbackError<SourceLost>> {
                assert_eq!(call.inputs, [DictionaryTerm::Id(ORIGINAL)]);
                Ok(vec![vec![DictionaryValue::Id(OUTPUT)]])
            }
        }
        let store = RelationStore::with_dictionary(HostDictionary {
            extra_terms: 2,
            ..HostDictionary::default()
        });
        let guard = Guard::assign("identity", vec!["?in".into()], "?out");
        let call = GuardCall {
            rule: 0,
            site: GuardSite::Body(0),
            guard: &guard,
            inputs: &[DictionaryTerm::Id(ORIGINAL)],
            model: &store,
        };
        let rows = call_guard(&Identity, &call, &mut StepGovernor::new(10)).unwrap();
        assert_eq!(rows, [vec![DictionaryValue::Id(OUTPUT)]]);
        let mut frame = SlotSolution::empty(1);
        frame.bind_value(0, &rows[0][0], &store).unwrap();
        assert_eq!(frame.get(0), Some(OUTPUT));
        let native = Guard::not_equal("?left", "?right");
        for (right, kept) in [(ORIGINAL, 0), (OUTPUT, 1)] {
            let call = GuardCall {
                rule: 0,
                site: GuardSite::Body(0),
                guard: &native,
                inputs: &[DictionaryTerm::Id(ORIGINAL), DictionaryTerm::Id(right)],
                model: &store,
            };
            assert_eq!(
                call_guard(&NoGuards, &call, &mut StepGovernor::new(10))
                    .unwrap()
                    .len(),
                kept
            );
        }
        assert_eq!(store.interner().lookups.load(Ordering::Relaxed), 0);
        assert_eq!(store.interner().resolves.load(Ordering::Relaxed), 0);
        store.interner().lost.store(true, Ordering::Relaxed);
        let before = frame.clone();
        assert_eq!(
            frame.bind_value(0, &DictionaryValue::Id(ORIGINAL), &store),
            Err(SourceLost(99))
        );
        assert_eq!(frame, before, "a source refusal never publishes a binding");
        assert_eq!(
            call_guard(&Identity, &call, &mut StepGovernor::new(10)),
            Err(EvalError::Source(SourceLost(99)))
        );
    }

    #[test]
    fn host_guard_retains_typed_source_failure_and_checks_empty_callback() {
        use crate::guard::CallbackError;
        use crate::test_support::{HostDictionary, SourceLost};
        use std::sync::atomic::{AtomicBool, AtomicUsize};
        struct HostGuard {
            fail: bool,
        }
        impl GuardEvaluator<HostDictionary> for HostGuard {
            fn evaluate(
                &self,
                call: &GuardCall<'_, HostDictionary>,
            ) -> Result<crate::guard::GuardRows<u64>, CallbackError<SourceLost>> {
                assert_eq!(
                    call.model.interner().lookup_term("original").unwrap(),
                    Some(u64::MAX)
                );
                call.model.interner().lost.store(true, Ordering::Relaxed);
                if self.fail {
                    Err(CallbackError::Source(SourceLost(17)))
                } else {
                    Ok(Vec::new())
                }
            }
        }
        let store = RelationStore::with_dictionary(HostDictionary {
            surfaces: std::iter::once((u64::MAX, "original".into())).collect(),
            resolves: AtomicUsize::new(0),
            lookups: AtomicUsize::new(0),
            lost: AtomicBool::new(false),
            lose_after_lookup: AtomicBool::new(false),
            pins: AtomicUsize::new(0),
            refuse_after_interned: AtomicBool::new(false),
            extra_terms: 0,
        });
        let guard = Guard::new(
            "host",
            Vec::new(),
            Vec::new(),
            crate::guard::GuardReads::Model,
        );
        let call = GuardCall {
            rule: 0,
            site: GuardSite::Body(0),
            guard: &guard,
            inputs: &[],
            model: &store,
        };
        for (fail, cause) in [(false, SourceLost(99)), (true, SourceLost(17))] {
            store.interner().lost.store(false, Ordering::Relaxed);
            let error =
                call_guard(&HostGuard { fail }, &call, &mut StepGovernor::new(0)).unwrap_err();
            assert_eq!(
                std::error::Error::source(&error)
                    .unwrap()
                    .downcast_ref::<SourceLost>(),
                Some(&cause)
            );
            assert_eq!(error, EvalError::Source(cause));
        }
        assert_eq!(store.interner().lookups.load(Ordering::Relaxed), 2);
        assert_eq!(
            call_guard(&HostGuard { fail: false }, &call, &mut StepGovernor::new(0)).unwrap_err(),
            EvalError::Source(SourceLost(99))
        );
        assert_eq!(
            store.interner().lookups.load(Ordering::Relaxed),
            2,
            "a lost source refuses before invoking caller code"
        );
    }

    #[test]
    fn host_negative_probe_preserves_identity_and_refuses_undecidable_absence() {
        use crate::test_support::{HostDictionary, SourceLost};
        use std::sync::atomic::{AtomicBool, AtomicUsize};
        const P_ID: u64 = 0xf000_0000_0000_0007;
        const GRAPH: u64 = 0x8000_0000_0000_0000;
        const TRIPLE: u64 = 0xa000_0000_1234_5678;
        let dictionary = HostDictionary {
            surfaces: [(P_ID, surface(P)), (GRAPH, String::new())]
                .into_iter()
                .collect(),
            resolves: AtomicUsize::new(0),
            lookups: AtomicUsize::new(0),
            lost: AtomicBool::new(false),
            lose_after_lookup: AtomicBool::new(false),
            pins: AtomicUsize::new(0),
            refuse_after_interned: AtomicBool::new(false),
            extra_terms: 0,
        };
        let mut store = RelationStore::with_dictionary(dictionary);
        store.insert_ids([TRIPLE, P_ID, u64::MAX, GRAPH]).unwrap();
        let resolves = store.interner().resolves.load(Ordering::Relaxed);
        let variables = vec!["?a".into(), "?b".into()];
        let probe = NegatedAtom {
            source: crate::clause::ReadSource::Evaluation,
            args: ArgShape::of_atom(&atom("?a", P, "?b"), &variables),
        };
        let mut frame = SlotSolution::empty(2);
        for bindings in [
            [None, None],
            [Some(TRIPLE), None],
            [None, Some(u64::MAX)],
            [Some(TRIPLE), Some(u64::MAX)],
        ] {
            frame.bindings.copy_from_slice(&bindings);
            assert!(
                probe
                    .satisfied(&frame, &store, &mut StepGovernor::new(u64::MAX))
                    .unwrap()
            );
        }
        frame.bindings[0] = Some(0);
        assert!(
            !probe
                .satisfied(&frame, &store, &mut StepGovernor::new(u64::MAX))
                .unwrap()
        );
        let absent = NegatedAtom {
            source: crate::clause::ReadSource::Evaluation,
            args: ArgShape::of_atom(&atom("?a", Q, "?b"), &variables),
        };
        assert!(
            !absent
                .satisfied(&frame, &store, &mut StepGovernor::new(u64::MAX))
                .unwrap()
        );
        assert_eq!(store.interner().resolves.load(Ordering::Relaxed), resolves);

        // An absent lexical lookup that loses its source cannot license negation.
        store
            .interner()
            .lose_after_lookup
            .store(true, Ordering::Relaxed);
        assert_eq!(
            absent
                .satisfied(&frame, &store, &mut StepGovernor::new(u64::MAX))
                .unwrap_err(),
            SourceLost(99)
        );
        // Exhausted work must not conceal an already lost snapshot.
        assert_eq!(
            probe
                .satisfied(&frame, &store, &mut StepGovernor::new(0))
                .unwrap_err(),
            SourceLost(99)
        );
        store.interner().lost.store(false, Ordering::Relaxed);
        store
            .interner()
            .lose_after_lookup
            .store(false, Ordering::Relaxed);
        frame.bindings[0] = Some(TRIPLE);
        // Validate after the early successful witness, too.
        assert_eq!(
            probe
                .visit_matches(
                    &frame,
                    &store,
                    &mut StepGovernor::new(u64::MAX),
                    |_| {
                        store.interner().lost.store(true, Ordering::Relaxed);
                        Ok::<_, SourceLost>(true)
                    },
                    core::convert::identity
                )
                .unwrap_err(),
            SourceLost(99)
        );
        store.interner().lost.store(false, Ordering::Relaxed);
        assert_eq!(
            probe
                .visit_matches(
                    &frame,
                    &store,
                    &mut StepGovernor::new(u64::MAX),
                    |_| {
                        store.interner().lost.store(true, Ordering::Relaxed);
                        Err(SourceLost(17))
                    },
                    core::convert::identity
                )
                .unwrap_err(),
            SourceLost(17),
            "the visitor's first failure survives final source loss"
        );
    }

    #[test]
    fn compiled_host_cycle_agrees_across_delta_schedules_and_refuses_source_loss() {
        use crate::test_support::{HostDictionary, SourceLost};
        use std::sync::atomic::{AtomicBool, AtomicUsize};
        const P_ID: u64 = 0xf000_0000_0000_0007;
        const Q_ID: u64 = 0x1000_0000_0000_0003;
        const R_ID: u64 = 0x9000_0000_0000_0005;
        const GRAPH: u64 = 0x8000_0000_0000_0000;
        const TRIPLE: u64 = 0xa000_0000_1234_5678;
        let dictionary = HostDictionary {
            surfaces: [
                (P_ID, surface(P)),
                (Q_ID, surface(Q)),
                (R_ID, surface(R)),
                (GRAPH, String::new()),
            ]
            .into_iter()
            .collect(),
            resolves: AtomicUsize::new(0),
            lookups: AtomicUsize::new(0),
            lost: AtomicBool::new(false),
            lose_after_lookup: AtomicBool::new(false),
            pins: AtomicUsize::new(0),
            refuse_after_interned: AtomicBool::new(false),
            extra_terms: 0,
        };
        let mut store = RelationStore::with_dictionary(dictionary);
        for ids in [
            [0, P_ID, TRIPLE, GRAPH],
            [TRIPLE, Q_ID, u64::MAX, GRAPH],
            [u64::MAX, R_ID, 0, GRAPH],
        ] {
            store.insert_ids(ids).unwrap();
        }
        let resolves = store.interner().resolves.load(Ordering::Relaxed);
        let rule = DlClause::datalog(
            atom("?a", "https://example.org/result", "?c"),
            vec![
                atom("?a", P, "?b"),
                atom("?b", Q, "?c"),
                atom("?c", R, "?a"),
            ],
        );
        let executable = compile(vec![rule]).unwrap();
        let (_, plan) = executable.rule_entry(0);
        assert!(plan.has_cyclic_subplan());
        for delta in [
            Delta::all(3),
            Delta { lo: 0, hi: 1 },
            Delta { lo: 1, hi: 2 },
            Delta { lo: 2, hi: 3 },
        ] {
            let indexes = DeltaIndexes::build(&store, [delta]).unwrap();
            let snapshot = JoinSnapshot {
                seed: None,
                rel: &store,
                delta,
                indexes: &indexes,
            };
            let mut planned_governor = StepGovernor::new(u64::MAX);
            let planned = join_body(
                plan,
                snapshot,
                JoinStrategy::Planned,
                &mut planned_governor,
                None,
            )
            .unwrap();
            let binary = join_body(
                plan,
                snapshot,
                JoinStrategy::ForcedBinary,
                &mut StepGovernor::new(u64::MAX),
                None,
            )
            .unwrap();
            assert_eq!(planned, binary);
            assert_eq!(
                planned.len(),
                1,
                "each delta decomposition emits the triangle exactly once"
            );
            for (name, id) in [("?a", 0), ("?b", TRIPLE), ("?c", u64::MAX)] {
                let slot = plan
                    .variables()
                    .iter()
                    .position(|variable| variable == name)
                    .unwrap();
                assert_eq!(planned[0].get(slot), Some(id));
            }
            assert_eq!(planned[0].sources.len(), 3);
        }
        assert_eq!(
            store.interner().resolves.load(Ordering::Relaxed),
            resolves,
            "both join strategies preserve original row IDs without lexical resolution"
        );
        store.interner().lost.store(true, Ordering::Relaxed);
        assert_eq!(
            join_body(
                plan,
                JoinSnapshot {
                    seed: None,
                    rel: &store,
                    delta: Delta::all(0),
                    indexes: &DeltaIndexes { runs: Vec::new() },
                },
                JoinStrategy::Planned,
                &mut StepGovernor::new(0),
                None,
            )
            .unwrap_err(),
            SourceLost(99)
        );
        store.interner().lost.store(false, Ordering::Relaxed);
        store
            .interner()
            .lose_after_lookup
            .store(true, Ordering::Relaxed);
        let cycle = plan
            .join_groups()
            .iter()
            .find_map(|group| match group {
                JoinGroup::Leapfrog(cycle) => Some(cycle),
                JoinGroup::Binary(_) => None,
            })
            .unwrap();
        let error = extend_solutions_leapfrog(
            &LeapfrogRun {
                plan,
                cycle,
                scan: JoinScan::Uniform(Scan::Full),
                rel: &store,
                delta: Delta::all(3),
                indexes: &DeltaIndexes { runs: Vec::new() },
            },
            vec![SlotSolution::empty(plan.variables().len())],
            &mut StepGovernor::new(u64::MAX),
        )
        .unwrap_err();
        assert_eq!(
            error,
            SourceLost(99),
            "the actual cyclic descent propagates its source failure"
        );
        let cause = EvalError::Source(error);
        assert_eq!(
            std::error::Error::source(&cause)
                .unwrap()
                .downcast_ref::<SourceLost>(),
            Some(&SourceLost(99))
        );
        assert_eq!(cause.to_string(), "source lost: 99");
        assert_ne!(cause, EvalError::Storage(StorageError::AllocationFailed));
    }

    #[test]
    fn compiled_binary_operators_carry_host_ids_and_refuse_source_loss() {
        use crate::test_support::{HostDictionary, SourceLost};
        use std::sync::atomic::{AtomicBool, AtomicUsize};
        const P_ID: u64 = 0xf000_0000_0000_0007;
        const Q_ID: u64 = 0x1000_0000_0000_0003;
        const GRAPH: u64 = 0x8000_0000_0000_0000;
        const TRIPLE: u64 = 0xa000_0000_1234_5678;
        let dictionary = HostDictionary {
            surfaces: [
                (P_ID, surface(P)),
                (Q_ID, surface(Q)),
                (GRAPH, String::new()),
            ]
            .into_iter()
            .collect(),
            resolves: AtomicUsize::new(0),
            lookups: AtomicUsize::new(0),
            lost: AtomicBool::new(false),
            lose_after_lookup: AtomicBool::new(false),
            pins: AtomicUsize::new(0),
            refuse_after_interned: AtomicBool::new(false),
            extra_terms: 0,
        };
        let mut store = RelationStore::with_dictionary(dictionary);
        store.insert_ids([0, P_ID, TRIPLE, GRAPH]).unwrap();
        store.insert_ids([TRIPLE, Q_ID, u64::MAX, GRAPH]).unwrap();
        let resolves = store.interner().resolves.load(Ordering::Relaxed);
        let rule = DlClause::datalog(
            atom("?s", R, "?o"),
            vec![atom("?s", P, "?m"), atom("?m", Q, "?o")],
        );
        let plan = RulePlan::for_rule(&rule);
        let mut partial = vec![SlotSolution::<u64>::empty(plan.variables().len())];
        let mut governor = StepGovernor::new(u64::MAX);
        for operator in plan.operators() {
            partial = extend_slot_solutions(
                operator,
                &store,
                Delta::all(store.row_slots()),
                &DeltaIndexes { runs: Vec::new() },
                Scan::Full,
                partial,
                &mut governor,
            )
            .unwrap();
        }
        assert_eq!(partial.len(), 1);
        for (name, id) in [("?s", 0), ("?m", TRIPLE), ("?o", u64::MAX)] {
            let slot = plan
                .variables()
                .iter()
                .position(|variable| variable == name)
                .unwrap();
            assert_eq!(partial[0].get(slot), Some(id));
        }
        assert_eq!(partial[0].sources.len(), 2);
        assert_eq!(store.interner().resolves.load(Ordering::Relaxed), resolves);
        assert_eq!(
            store.interner().lookups.load(Ordering::Relaxed),
            4,
            "only two authored constants per operator are looked up, never matched row IDs"
        );
        store.interner().lost.store(true, Ordering::Relaxed);
        let error = extend_slot_solutions(
            &plan.operators()[0],
            &store,
            Delta::all(0),
            &DeltaIndexes { runs: Vec::new() },
            Scan::Full,
            Vec::new(),
            &mut governor,
        )
        .unwrap_err();
        assert_eq!(
            error,
            SourceLost(99),
            "an empty input cannot hide source loss"
        );
        store.interner().lost.store(false, Ordering::Relaxed);
        store
            .interner()
            .lose_after_lookup
            .store(true, Ordering::Relaxed);
        let error = extend_slot_solutions(
            &plan.operators()[0],
            &store,
            Delta::all(store.row_slots()),
            &DeltaIndexes { runs: Vec::new() },
            Scan::Full,
            vec![SlotSolution::empty(plan.variables().len())],
            &mut governor,
        )
        .unwrap_err();
        assert_eq!(
            error,
            SourceLost(99),
            "source loss during constant resolution is never a missing constant"
        );
    }

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
            store
                .insert(
                    &surface(subject),
                    &surface(predicate),
                    &surface(object),
                    RelationStore::DEFAULT_GRAPH,
                )
                .unwrap();
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
        evaluate_guarded(
            &exe,
            edb,
            &NoGuards,
            &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
            None,
        )
        .expect("the fixture program stays inside every ceiling")
    }

    #[test]
    fn evidence_changes_reach_dependents_of_preexisting_facts() {
        use purrdf_core::evidence::{
            EvidenceRecord, Frame, Information, InformationMap, SearchCompleteness,
        };
        use purrdf_lex::allocation::Resident;
        let next = "https://example.org/next";
        let final_predicate = "https://example.org/final";
        let edb = store_of(&[
            ("a", P, "b"),
            ("a", Q, "b"),
            ("a", R, "b"),
            ("a", final_predicate, "b"),
        ]);
        let mut rows =
            crate::evidence::DerivationEvidence::with_admission(edb.row_count(), Resident);
        rows.seed(RowId::from_index(1).unwrap(), |memory| {
            EvidenceRecord::from_parts(
                &InformationMap::uniform(Information::Opposed),
                SearchCompleteness::Complete,
                &Frame::actual(),
                &[],
                &[],
                memory,
            )
        })
        .unwrap();
        let exe = compile(vec![
            DlClause::datalog(atom("?s", R, "?o"), vec![atom("?s", P, "?o")]),
            DlClause::datalog(atom("?s", next, "?o"), vec![atom("?s", Q, "?o")]),
            DlClause::datalog(atom("?s", R, "?o"), vec![atom("?s", next, "?o")]),
            DlClause::datalog(atom("?s", final_predicate, "?o"), vec![atom("?s", R, "?o")]),
        ])
        .unwrap();
        let result = evaluate_evidenced(
            &exe,
            edb,
            &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
            &NoGuards,
            None,
            rows,
            evidence_scope(),
        )
        .unwrap();
        for predicate in [R, final_predicate] {
            let predicate = result
                .evaluation
                .facts
                .term_id(&surface(predicate))
                .unwrap();
            let row = result
                .evaluation
                .facts
                .rows_from(0)
                .find(|(_, terms)| terms[1] == predicate)
                .unwrap()
                .0;
            assert_eq!(
                result
                    .evidence
                    .record(row)
                    .unwrap()
                    .information()
                    .fallback(),
                Information::Both
            );
        }
        assert_eq!(
            result.evidence.row_count(),
            result.evaluation.facts.row_count()
        );
        assert_eq!(
            result
                .evaluation
                .derivations
                .as_deref()
                .expect("recorded")
                .len(),
            1,
            "evidence changes do not invent preferred provenance proofs for preexisting facts"
        );
    }

    #[test]
    fn factorized_evidence_retains_unselected_aggregate_witnesses() {
        use purrdf_core::evidence::{
            AggregateBound, EvidenceRecord, Frame, InformationMap, SearchCompleteness,
        };
        use purrdf_hash::hex::Digest32;
        use purrdf_lex::allocation::Resident;
        let edb = store_of(&[("a", P, "b"), ("c", Q, "d"), ("e", Q, "f")]);
        let column = Digest32::new([9; 32]);
        let mut rows =
            crate::evidence::DerivationEvidence::with_admission(edb.row_count(), Resident);
        for (index, bound) in [
            AggregateBound::Lower,
            AggregateBound::Exact,
            AggregateBound::Undetermined,
        ]
        .into_iter()
        .enumerate()
        {
            rows.seed(RowId::from_address(index).unwrap(), |memory| {
                EvidenceRecord::from_parts(
                    &InformationMap::default(),
                    SearchCompleteness::Complete,
                    &Frame::actual(),
                    &[],
                    &[(column, bound)],
                    memory,
                )
            })
            .unwrap();
        }
        let exe = compile(vec![DlClause::datalog(
            ClauseAtom::positive(
                iri("https://example.org/result"),
                R,
                iri("https://example.org/value"),
            ),
            vec![atom("?s", P, "?o"), atom("?x", Q, "?y")],
        )])
        .unwrap();
        let result = evaluate_evidenced(
            &exe,
            edb,
            &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
            &NoGuards,
            None,
            rows,
            evidence_scope(),
        )
        .unwrap();
        assert_eq!(
            result
                .evaluation
                .derivations
                .as_deref()
                .expect("recorded")
                .len(),
            1
        );
        let row = result.evaluation.facts.rows_from(3).next().unwrap().0;
        assert_eq!(
            result.evidence.record(row).unwrap().aggregate_bounds(),
            &[(column, AggregateBound::Undetermined)]
        );
    }

    #[test]
    fn evidence_negation_preserves_signed_atom_group_and_factor_witnesses() {
        use purrdf_core::evidence::{
            EvidenceRecord, Frame, Information, InformationMap, SearchCompleteness,
        };
        use purrdf_lex::allocation::Resident;
        let output = "https://example.org/negative-result";
        for grouped in [false, true] {
            for factorized in [false, true] {
                for ground in [false, true] {
                    for information in [
                        Information::Neither,
                        Information::Supported,
                        Information::Opposed,
                        Information::Both,
                    ] {
                        for closed in [false, true] {
                            for complete in [false, true] {
                                let edb = store_of(&[("a", P, "b"), ("a", Q, "b"), ("c", R, "d")]);
                                let mut rows = crate::evidence::DerivationEvidence::with_admission(
                                    edb.row_count(),
                                    Resident,
                                );
                                rows.seed(RowId::from_index(1).unwrap(), |memory| {
                                    EvidenceRecord::from_parts(
                                        &InformationMap::uniform(information),
                                        if complete {
                                            SearchCompleteness::Complete
                                        } else {
                                            SearchCompleteness::Undetermined
                                        },
                                        &Frame::actual(),
                                        &[],
                                        &[],
                                        memory,
                                    )
                                })
                                .unwrap();
                                let subject = if ground { iri("a") } else { v("?s") };
                                let object = if ground { iri("b") } else { v("?o") };
                                let mut body = vec![atom("?s", P, "?o")];
                                if factorized {
                                    body.push(atom("?x", R, "?y"));
                                }
                                let rule = if grouped {
                                    DlClause::datalog(atom("?s", output, "?o"), body)
                                        .with_negations(vec![Negation::new(
                                            vec![ClauseAtom::positive(subject, Q, object)],
                                            Vec::new(),
                                        )])
                                } else {
                                    body.push(ClauseAtom::negated(subject, Q, object));
                                    DlClause::datalog(atom("?s", output, "?o"), body)
                                };
                                let exe = compile(vec![rule]).unwrap();
                                let mut scope = evidence_scope();
                                scope.closure_declared = closed;
                                let result = evaluate_evidenced(
                                    &exe,
                                    edb,
                                    &EvalOptions::default()
                                        .with_derivation_mode(DerivationMode::Record),
                                    &NoGuards,
                                    None,
                                    rows,
                                    scope,
                                )
                                .unwrap();
                                let derived = result.evaluation.facts.rows_from(3).next();
                                let blocked =
                                    closed && complete && information == Information::Supported;
                                assert_eq!(
                                    derived.is_none(),
                                    blocked,
                                    "group={grouped}, factor={factorized}, ground={ground}, information={information:?}, closed={closed}, complete={complete}"
                                );
                                if let Some((row, _)) = derived {
                                    let record = result.evidence.record(row).unwrap();
                                    let expected = if closed && complete {
                                        information.negate()
                                    } else {
                                        information
                                    };
                                    assert_eq!(
                                        record.information().fallback(),
                                        expected,
                                        "group={grouped}, factor={factorized}, ground={ground}, information={information:?}, closed={closed}, complete={complete}"
                                    );
                                    assert_eq!(
                                        record.completeness(),
                                        if closed && complete {
                                            SearchCompleteness::Complete
                                        } else {
                                            SearchCompleteness::Undetermined
                                        }
                                    );
                                    if !closed || !complete {
                                        assert_eq!(record.reasons(), &[scope.missing_scope_reason]);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn evidence_negation_combines_every_matching_witness_before_negating() {
        use purrdf_core::evidence::{
            EvidenceRecord, Frame, Information, InformationMap, SearchCompleteness,
        };
        use purrdf_lex::allocation::Resident;
        for grouped in [false, true] {
            let edb = store_of(&[("a", P, "b"), ("a", Q, "b"), ("a", Q, "c")]);
            let mut rows =
                crate::evidence::DerivationEvidence::with_admission(edb.row_count(), Resident);
            rows.seed(RowId::from_index(2).unwrap(), |memory| {
                EvidenceRecord::from_parts(
                    &InformationMap::uniform(Information::Opposed),
                    SearchCompleteness::Complete,
                    &Frame::actual(),
                    &[],
                    &[],
                    memory,
                )
            })
            .unwrap();
            let mut body = vec![atom("?s", P, "?o")];
            let rule = if grouped {
                DlClause::datalog(atom("?s", R, "?o"), body).with_negations(vec![Negation::new(
                    vec![atom("?s", Q, "?local")],
                    Vec::new(),
                )])
            } else {
                body.push(ClauseAtom::negated(v("?s"), Q, v("?free")));
                DlClause::datalog(atom("?s", R, "?o"), body)
            };
            let exe = compile(vec![rule]).unwrap();
            let result = evaluate_evidenced(
                &exe,
                edb,
                &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
                &NoGuards,
                None,
                rows,
                evidence_scope(),
            )
            .unwrap();
            let row = result
                .evaluation
                .facts
                .rows_from(3)
                .next()
                .expect("both signed witnesses survive negation")
                .0;
            assert_eq!(
                result
                    .evidence
                    .record(row)
                    .unwrap()
                    .information()
                    .fallback(),
                Information::Both
            );
        }
    }

    #[test]
    fn evidence_negation_absence_requires_declared_complete_scope() {
        use purrdf_core::evidence::{
            EvidenceRecord, Frame, Information, InformationMap, SearchCompleteness,
        };
        use purrdf_lex::allocation::Resident;
        for grouped in [false, true] {
            for factorized in [false, true] {
                for closed in [false, true] {
                    for complete in [false, true] {
                        let edb = store_of(&[("a", P, "b"), ("c", R, "d")]);
                        let mut scope = evidence_scope();
                        scope.closure_declared = closed;
                        scope.completeness = if complete {
                            SearchCompleteness::Complete
                        } else {
                            SearchCompleteness::Undetermined
                        };
                        let mut rows = crate::evidence::DerivationEvidence::with_admission(
                            edb.row_count(),
                            Resident,
                        );
                        // Keep capture active even for complete closed absence.
                        rows.seed(RowId::from_index(0).unwrap(), |memory| {
                            EvidenceRecord::from_parts(
                                &InformationMap::default(),
                                SearchCompleteness::Complete,
                                &Frame::actual(),
                                &[scope.missing_scope_reason],
                                &[],
                                memory,
                            )
                        })
                        .unwrap();
                        let mut body = vec![atom("?s", P, "?o")];
                        if factorized {
                            body.push(atom("?x", R, "?y"));
                        }
                        let head = atom("?s", "https://example.org/absent-result", "?o");
                        let rule = if grouped {
                            DlClause::datalog(head, body).with_negations(vec![Negation::new(
                                vec![ClauseAtom::positive(iri("a"), Q, iri("b"))],
                                Vec::new(),
                            )])
                        } else {
                            body.push(ClauseAtom::negated(iri("a"), Q, iri("b")));
                            DlClause::datalog(head, body)
                        };
                        let exe = compile(vec![rule]).unwrap();
                        let result = evaluate_evidenced(
                            &exe,
                            edb,
                            &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
                            &NoGuards,
                            None,
                            rows,
                            scope,
                        )
                        .unwrap();
                        let row = result.evaluation.facts.rows_from(2).next().unwrap().0;
                        let record = result.evidence.record(row).unwrap();
                        assert_eq!(
                            record.information().fallback(),
                            if closed && complete {
                                Information::Supported
                            } else {
                                Information::Neither
                            }
                        );
                        assert_eq!(
                            record.completeness(),
                            if closed && complete {
                                SearchCompleteness::Complete
                            } else {
                                SearchCompleteness::Undetermined
                            }
                        );
                        assert_eq!(record.reasons(), &[scope.missing_scope_reason]);
                    }
                }
            }
        }
    }

    #[test]
    fn default_evidenced_execution_needs_no_metadata_admission() {
        use purrdf_lex::allocation::{Admission, StorageError};
        #[derive(Debug)]
        struct NoCapacity;
        impl Admission for NoCapacity {
            fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
                if bytes == 0 {
                    Ok(())
                } else {
                    Err(StorageError::AdmissionFailed)
                }
            }
        }
        let edb = store_of(&[("a", P, "b")]);
        let rows = crate::evidence::DerivationEvidence::with_admission(edb.row_count(), NoCapacity);
        let exe = compile(vec![DlClause::datalog(
            atom("?s", Q, "?o"),
            vec![atom("?s", P, "?o")],
        )])
        .unwrap();
        let result = evaluate_evidenced(
            &exe,
            edb,
            &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
            &NoGuards,
            None,
            rows,
            evidence_scope(),
        )
        .unwrap();
        assert_eq!(result.evidence.row_count(), 2);
        assert!(result.evidence.rows().is_uniform_default());

        // An undeclared negative scope needs a real record. Its native physical
        // refusal must not become an unknown result or an ordinary derivation.
        let edb = store_of(&[("a", P, "b")]);
        let rows = crate::evidence::DerivationEvidence::with_admission(edb.row_count(), NoCapacity);
        let exe = compile(vec![DlClause::datalog(
            atom("?s", R, "?o"),
            vec![
                atom("?s", P, "?o"),
                ClauseAtom::negated(v("?s"), Q, v("?o")),
            ],
        )])
        .unwrap();
        let mut scope = evidence_scope();
        scope.closure_declared = false;
        let result = evaluate_evidenced(
            &exe,
            edb,
            &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
            &NoGuards,
            None,
            rows,
            scope,
        );
        assert!(matches!(result, Err(EvalError::Evidence(_))));
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
        let inline = SlotSolution::<TermId>::empty(8);
        assert!(
            !inline.bindings.spilled(),
            "a frame of the widest measured width must live inline; it is cloned once \
             per surviving candidate row, so spilling here is one allocation per \
             intermediate solution"
        );

        let spilled = SlotSolution::<TermId>::empty(9);
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
        assert_eq!(
            evaluation
                .derivations()
                .expect("derivation recording was requested")
                .len(),
            1
        );
        let derivation = &evaluation
            .derivations()
            .expect("derivation recording was requested")[0];
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
        assert_eq!(
            evaluation
                .derivations()
                .expect("derivation recording was requested")
                .len(),
            1
        );
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
        assert_eq!(
            evaluation
                .derivations()
                .expect("derivation recording was requested")
                .len(),
            1
        );
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
        assert_eq!(
            evaluation
                .derivations()
                .expect("derivation recording was requested")
                .len(),
            1
        );
        let derivation = &evaluation
            .derivations()
            .expect("derivation recording was requested")[0];
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
            evaluation
                .derivations()
                .expect("derivation recording was requested")
                .len(),
            3,
            "one derivation per derived typing"
        );
        // Provenance names the concrete predicate each body atom MATCHED, which for the
        // variable-predicate atom is data rather than rule text.
        let matched: BTreeSet<String> = evaluation
            .derivations()
            .expect("derivation recording was requested")
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
                again
                    .derivations()
                    .expect("derivation recording was requested"),
                reference
                    .derivations()
                    .expect("derivation recording was requested"),
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
            )
            .unwrap();
        }

        let evaluation = run(vec![rule], edb);
        let derived: BTreeSet<(String, String, String)> = evaluation
            .derivations()
            .expect("derivation recording was requested")
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
            )
            .unwrap();
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
        remaining: AtomicU64,
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
    /// deterministic: attaching one is observationally nothing until it fires.
    /// Asserted over the whole synthetic corpus rather than over one program, and against
    /// the same execution without a stop signal rather than against itself.
    #[test]
    fn an_unfired_stop_signal_changes_no_answer() {
        let never = StopsAfter {
            remaining: AtomicU64::new(u64::MAX),
        };
        for workload in synth_corpus::all() {
            let exe = compile(workload.rules.clone()).expect("the corpus program compiles");
            let plain = evaluate_guarded(
                &exe,
                workload.edb(),
                &NoGuards,
                &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
                None,
            )
            .expect("inside every ceiling");
            let model_only =
                evaluate_until(&exe, workload.edb(), Some(&never)).expect("inside every ceiling");
            assert!(model_only.derivations().is_none());
            assert_eq!(
                plain.facts().facts_sorted(),
                model_only.facts().facts_sorted()
            );
            assert_eq!(plain.budget(), model_only.budget());
            let watched = evaluate_guarded(
                &exe,
                workload.edb(),
                &NoGuards,
                &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
                Some(&never),
            )
            .expect("inside every ceiling");
            assert_eq!(
                plain.facts().facts_sorted(),
                watched.facts().facts_sorted(),
                "{}: facts",
                workload.name
            );
            assert_eq!(
                plain
                    .derivations()
                    .expect("derivation recording was requested"),
                watched
                    .derivations()
                    .expect("derivation recording was requested"),
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
    /// run" and "the program passed a limit" are different facts and only one of them
    /// is a reason to change the input. The report is the consumption measured at the point
    /// the signal was observed, so a stopped run can still say what it had spent.
    #[test]
    fn a_fired_stop_signal_refuses_rather_than_truncating() {
        for workload in synth_corpus::all() {
            let exe = compile(workload.rules.clone()).expect("the corpus program compiles");
            // Fires at the FIRST round boundary, before any rule of any stratum has run.
            let immediate = StopsAfter {
                remaining: AtomicU64::new(0),
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
                    store.insert(&quad.0, &quad.1, &quad.2, &quad.3).unwrap();
                }
                let again = run(workload.rules.clone(), store);
                assert_eq!(
                    again.facts().facts_sorted(),
                    reference.facts().facts_sorted(),
                    "{}: seed {seed} facts",
                    workload.name
                );
                assert_eq!(
                    again
                        .derivations()
                        .expect("derivation recording was requested"),
                    reference
                        .derivations()
                        .expect("derivation recording was requested"),
                    "{}: seed {seed} derivations",
                    workload.name
                );
            }
        }
    }

    // ── The required differential tests ─────────────────────────────────────────

    /// The leapfrog triejoin and the indexed binary fallback are two implementations of
    /// one contract, so they must agree everywhere — on the whole synthetic corpus and on
    /// a certified triangle and a connected hybrid plan, the triejoin shapes.
    #[test]
    fn leapfrog_and_binary_joins_agree() {
        let mut cyclic_seen = false;
        for workload in synth_corpus::all()
            .into_iter()
            .chain([triangle_workload(), connected_hybrid_workload()])
        {
            let exe = compile(workload.rules.clone()).expect("the corpus program compiles");
            let cyclic =
                (0..exe.rule_count()).any(|index| exe.rule_entry(index).1.has_cyclic_subplan());
            cyclic_seen |= cyclic;
            assert_eq!(
                cyclic,
                matches!(workload.name, "triangle" | "connected-hybrid"),
                "{}: the triangle and hybrid fixtures must route through the triejoin; \
                 the remaining fixtures must cover the binary path",
                workload.name
            );

            let planned = evaluate_with(
                &exe,
                workload.edb(),
                None,
                &NoGuards,
                EvalOptions::default().with_derivation_mode(DerivationMode::Record),
                RoundExecution::Parallel,
                JoinStrategy::Planned,
            )
            .expect("planned join stays inside every ceiling");
            let binary = evaluate_with(
                &exe,
                workload.edb(),
                None,
                &NoGuards,
                EvalOptions::default().with_derivation_mode(DerivationMode::Record),
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
                planned
                    .derivations()
                    .expect("derivation recording was requested"),
                binary
                    .derivations()
                    .expect("derivation recording was requested"),
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

    /// A selective seed, a disconnected type, a bridge and the triangle. The bridge
    /// reaches every triangle Z, so the analytic answer stays the triangle's answer.
    /// Two independent type witnesses also exercise canonical authored provenance.
    fn connected_hybrid_workload() -> SynthWorkload {
        let mut workload = triangle_workload();
        let kind = "https://example.org/kind";
        let entry = "https://example.org/entry";
        let root = "https://example.org/root";
        let seed = "https://example.org/Seed";
        let other = "https://example.org/Other";
        let mut body = vec![
            ClauseAtom::positive(v("?V"), kind, iri(seed)),
            ClauseAtom::positive(v("?U"), kind, iri(other)),
            atom("?V", entry, "?Z"),
        ];
        body.extend_from_slice(workload.rules[0].body());
        let head = workload.rules[0]
            .head_atoms()
            .next()
            .expect("the triangle has one head")
            .clone();
        workload.rules = vec![DlClause::datalog(head, body)];
        workload.triples.extend([
            quad(&surface(root), kind, &surface(seed)),
            quad(
                &surface("https://example.org/other-a"),
                kind,
                &surface(other),
            ),
            quad(
                &surface("https://example.org/other-b"),
                kind,
                &surface(other),
            ),
        ]);
        for fact in &workload.expected {
            workload
                .triples
                .push(quad(&surface(root), entry, &fact.object));
        }
        workload.name = "connected-hybrid";
        workload
    }

    /// Physical group movement preserves both the analytic relation and the exact best
    /// proof in authored body order, independently of store insertion coordinates.
    #[test]
    fn connected_hybrid_joins_preserve_the_analytic_facts_and_authored_proofs() {
        let workload = connected_hybrid_workload();
        let exe = compile(workload.rules.clone()).expect("the hybrid program compiles");
        let plan = exe.rule_entry(0).1;
        assert!(plan.has_cyclic_subplan());
        assert_eq!(
            plan.operators()
                .iter()
                .map(AtomOperator::positive_position)
                .collect::<Vec<_>>(),
            [0, 2, 3, 4, 5, 1],
            "the fixture must exercise reordered groups"
        );
        let seeded: BTreeSet<Fact> = workload.edb().facts_sorted().into_iter().collect();
        let expected_facts: Vec<Fact> = seeded.union(&workload.expected).cloned().collect();
        let seed_fact = |subject: &str, predicate: &str, object: &str| {
            seeded
                .iter()
                .find(|fact| {
                    fact.subject == subject
                        && fact.predicate == surface(predicate)
                        && fact.object == object
                })
                .expect("every oracle premise is seeded")
                .clone()
        };
        let root = surface("https://example.org/root");
        let expected_proofs: Vec<Derivation> = workload
            .expected
            .iter()
            .map(|fact| {
                let q = seeded
                    .iter()
                    .find(|source| source.predicate == surface(Q) && source.object == fact.object)
                    .expect("the circulant Q relation has exactly one predecessor")
                    .clone();
                Derivation {
                    fact: fact.clone(),
                    rule: 0,
                    head: 0,
                    sources: vec![
                        seed_fact(
                            &root,
                            "https://example.org/kind",
                            &surface("https://example.org/Seed"),
                        ),
                        seed_fact(
                            &surface("https://example.org/other-a"),
                            "https://example.org/kind",
                            &surface("https://example.org/Other"),
                        ),
                        seed_fact(&root, "https://example.org/entry", &fact.object),
                        seed_fact(&fact.subject, P, &q.subject),
                        q,
                        seed_fact(&fact.object, R, &fact.subject),
                    ],
                    proof_height: 1,
                }
            })
            .collect();
        for seed in 0..8u64 {
            let mut shuffled = workload.clone();
            shuffled.triples = permute(&workload.triples, seed);
            for strategy in [JoinStrategy::Planned, JoinStrategy::ForcedBinary] {
                let evaluation = evaluate_with(
                    &exe,
                    shuffled.edb(),
                    None,
                    &NoGuards,
                    EvalOptions::default().with_derivation_mode(DerivationMode::Record),
                    RoundExecution::Parallel,
                    strategy,
                )
                .expect("the hybrid fixture stays inside every ceiling");
                assert_eq!(
                    evaluation.facts().facts_sorted(),
                    expected_facts,
                    "seed {seed}, {strategy:?}: analytic facts"
                );
                assert_eq!(
                    evaluation
                        .derivations()
                        .expect("derivation recording was requested"),
                    expected_proofs,
                    "seed {seed}, {strategy:?}: exact canonical authored proofs"
                );
            }
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
                EvalOptions::default().with_derivation_mode(DerivationMode::Record),
                RoundExecution::Sequential,
                JoinStrategy::Planned,
            )
            .expect("sequential rounds stay inside every ceiling");
            let parallel = evaluate_with(
                &exe,
                workload.edb(),
                None,
                &NoGuards,
                EvalOptions::default().with_derivation_mode(DerivationMode::Record),
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
                sequential
                    .derivations()
                    .expect("derivation recording was requested"),
                parallel
                    .derivations()
                    .expect("derivation recording was requested"),
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

    /// Unequal rule cardinalities share one ceiling, including one refusal
    /// observation, rather than each receiving a fresh copy of the budget.
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn asymmetric_rules_share_exact_round_credits_at_every_worker_count() {
        let mut edb = RelationStore::new();
        let rules: Vec<_> = (1..=32)
            .map(|count| {
                let input = format!("https://example.org/input{count}");
                for row in 0..count {
                    edb.insert(
                        &surface(&format!("https://example.org/s{row}")),
                        &surface(&input),
                        &surface("https://example.org/o"),
                        RelationStore::DEFAULT_GRAPH,
                    )
                    .unwrap();
                }
                DlClause::datalog(
                    atom("?s", &format!("https://example.org/output{count}"), "?o"),
                    vec![atom("?s", &input, "?o")],
                )
            })
            .collect();
        let exe = compile(rules).expect("valid independent rules");
        let expected = 32 * 33 / 2;
        let mut reference = None;
        for workers in [1, 4, 32] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(workers)
                .build()
                .expect("test pool");
            pool.install(|| {
                let exact = evaluate_guarded(
                    &exe,
                    edb.clone(),
                    &NoGuards,
                    &EvalOptions::default()
                        .with_derivation_mode(DerivationMode::Record)
                        .with_max_join_steps(expected),
                    None,
                )
                .expect("exact credits");
                assert_eq!(exact.budget().join_steps(), expected);
                assert_eq!(exact.facts().row_count(), 2 * expected as usize);
                let answer = (
                    exact.facts().facts_sorted(),
                    exact
                        .derivations()
                        .expect("derivation recording was requested")
                        .to_vec(),
                    exact.budget(),
                );
                if let Some(prior) = &reference {
                    assert_eq!(&answer, prior);
                } else {
                    reference = Some(answer);
                }
                for ceiling in [0, 17, expected - 1] {
                    let error = evaluate_guarded(
                        &exe,
                        edb.clone(),
                        &NoGuards,
                        &EvalOptions::default()
                            .with_derivation_mode(DerivationMode::Record)
                            .with_max_join_steps(ceiling),
                        None,
                    )
                    .expect_err("total refusal");
                    let EvalError::BudgetExhausted { resource, report } = error else {
                        panic!("{error:?}");
                    };
                    assert_eq!(resource, BudgetResource::JoinSteps);
                    assert_eq!(report.join_steps(), ceiling + 1);
                    assert_eq!(
                        report.stored_facts(),
                        expected as usize,
                        "no refused round commits"
                    );
                }
            });
        }
    }

    /// Full-width counters retain their final successful credit and latch a typed
    /// refusal even when its observation cannot be represented above u64::MAX.
    #[test]
    fn full_width_admission_never_silently_truncates() {
        let mut governor = StepGovernor::new(u64::MAX);
        governor.pool.used.store(u64::MAX - 1, Ordering::Relaxed);
        assert!(governor.charge());
        assert!(!governor.spent());
        assert_eq!(governor.observed(), u64::MAX);
        assert!(!governor.charge());
        assert!(governor.spent());
        assert_eq!(governor.observed(), u64::MAX);
        assert!(!governor.task().charge());

        let rule = DlClause::datalog(atom("?s", Q, "?o"), vec![atom("?s", P, "?o")]);
        let plan = RulePlan::for_rule(&rule);
        let runtime = RuleRuntime::new(&rule, &plan);
        let edb = store_of(&[
            ("https://example.org/a", P, "https://example.org/o"),
            ("https://example.org/b", P, "https://example.org/o"),
        ]);
        let mut state = FixpointState::seeded(
            edb,
            EvalOptions::default()
                .with_derivation_mode(DerivationMode::Record)
                .with_max_join_steps(u64::MAX),
        );
        state.join_steps = u64::MAX - 1;
        let entry = RuleEntry {
            index: 0,
            rule: &rule,
            plan: &plan,
            runtime: &runtime,
            delta: Delta::all(2),
        };
        let round = evaluate_round(
            &[entry],
            RoundSnapshot {
                probe: None,
                rel: &state.rel,
                depth: state.depths(),
                assumed: &purrdf_core::FastSet::default(),
                delta_indexes: None,
            },
            RoundExecution::Parallel,
            JoinStrategy::Planned,
            state.allowance(),
            RoundControl {
                guards: &NoGuards,
                stop: None,
            },
        )
        .expect("round buffer");
        assert!(round.join_refused);
        let error = state
            .absorb(round)
            .expect_err("overflowed observation is a total refusal");
        let rendered = error.to_string();
        assert!(
            rendered.contains("full-width join-step limit")
                && rendered.contains("observation saturates"),
            "{rendered}"
        );
        assert!(
            rendered.contains("this limit cannot be raised"),
            "{rendered}"
        );
        let EvalError::BudgetExhausted { resource, report } = error else {
            panic!("{error:?}");
        };
        assert_eq!(resource, BudgetResource::JoinSteps);
        assert_eq!(report.join_steps(), u64::MAX);
        assert_eq!(report.join_step_limit(), u64::MAX);
        assert_eq!(state.rel.row_count(), 2);
        assert_eq!(state.derivations, []);
    }

    /// A terminal body row covers its first head only. A broad conjunctive head
    /// cannot multiply unmetered owned candidates and proofs.
    #[test]
    fn wide_conjunctive_heads_are_admitted_before_owned_expansion() {
        use crate::clause::HeadDisjunct;
        use crate::schedule::{Layer, NoHooks, Schedule, compile_scheduled, evaluate_scheduled};
        let head: Vec<_> = (0..128)
            .map(|index| atom("?s", &format!("https://example.org/head{index}"), "?o"))
            .collect();
        let rule = DlClause::new(
            vec![HeadDisjunct::new(head)],
            Vec::new(),
            vec![atom("?s", P, "?o")],
        );
        let program = compile_scheduled(
            vec![rule],
            Schedule::new(vec![Layer::new(vec![vec![0]], Vec::new())]),
        )
        .expect("valid conjunctive scheduled rule");
        let edb = store_of(&[("https://example.org/s", P, "https://example.org/o")]);
        let exact = evaluate_scheduled(
            &program,
            edb.clone(),
            &NoGuards,
            &mut NoHooks,
            &EvalOptions::default()
                .with_derivation_mode(DerivationMode::Record)
                .with_max_join_steps(128),
            None,
        )
        .expect("exact conjunctive credits");
        assert_eq!(exact.budget().join_steps(), 128);
        assert_eq!(exact.facts().row_count(), 129);
        for ceiling in [0, 1, 127] {
            let error = evaluate_scheduled(
                &program,
                edb.clone(),
                &NoGuards,
                &mut NoHooks,
                &EvalOptions::default()
                    .with_derivation_mode(DerivationMode::Record)
                    .with_max_join_steps(ceiling),
                None,
            )
            .expect_err("conjunctive total refusal");
            let EvalError::BudgetExhausted { resource, report } = error else {
                panic!("{error:?}");
            };
            assert_eq!(resource, BudgetResource::JoinSteps);
            assert_eq!(report.join_steps(), ceiling + 1);
            assert_eq!(report.stored_facts(), 1);
        }
    }

    /// Negative existence probes must bound broad partition traversal even when
    /// every addressed subject/object index is empty.
    #[test]
    fn negative_empty_partition_traversal_has_exact_admission_boundaries() {
        use crate::schedule::{Layer, NoHooks, Schedule, compile_scheduled, evaluate_scheduled};
        let mut edb = store_of(&[("https://example.org/a", P, "https://example.org/o")]);
        for index in 0..1000 {
            edb.insert(
                &surface("https://example.org/b"),
                &surface(&format!("https://example.org/other{index}")),
                &surface("https://example.org/other-object"),
                RelationStore::DEFAULT_GRAPH,
            )
            .unwrap();
        }
        let subject = v("?s");
        let predicate = v("?negative-predicate");
        let object = iri("https://example.org/other-object");
        let single = DlClause::datalog(
            atom("?s", Q, "?o"),
            vec![
                atom("?s", P, "?o"),
                ClauseAtom::negated_quad(
                    subject.clone(),
                    predicate.clone(),
                    object.clone(),
                    ClauseTerm::default_graph(),
                ),
            ],
        );
        let group = DlClause::datalog(atom("?s", Q, "?o"), vec![atom("?s", P, "?o")])
            .with_negations(vec![Negation::new(
                vec![ClauseAtom::quad(
                    subject,
                    predicate,
                    object,
                    ClauseTerm::default_graph(),
                )],
                Vec::new(),
            )]);
        for (rule, credits) in [(single, 1002), (group, 1003)] {
            let program = compile_scheduled(
                vec![rule],
                Schedule::new(vec![Layer::new(vec![vec![0]], Vec::new())]),
            )
            .expect("scheduled negative predicate probe");
            let exact = evaluate_scheduled(
                &program,
                edb.clone(),
                &NoGuards,
                &mut NoHooks,
                &EvalOptions::default()
                    .with_derivation_mode(DerivationMode::Record)
                    .with_max_join_steps(credits),
                None,
            )
            .expect("exact traversal credits");
            assert_eq!(exact.budget().join_steps(), credits);
            assert_eq!(exact.facts().row_count(), 1002);
            for ceiling in [0, 17, credits - 1] {
                let error = evaluate_scheduled(
                    &program,
                    edb.clone(),
                    &NoGuards,
                    &mut NoHooks,
                    &EvalOptions::default()
                        .with_derivation_mode(DerivationMode::Record)
                        .with_max_join_steps(ceiling),
                    None,
                )
                .expect_err("empty-index traversal refused");
                let EvalError::BudgetExhausted { resource, report } = error else {
                    panic!("{error:?}");
                };
                assert_eq!(resource, BudgetResource::JoinSteps);
                assert_eq!(report.join_steps(), ceiling + 1);
                assert_eq!(report.stored_facts(), 1001);
            }
        }
    }

    #[test]
    fn guard_first_empty_output_requires_unit_binding_credit() {
        use crate::clause::BodyLiteral::{Atom, Guard as Compute};
        struct DropAll(std::cell::Cell<usize>);
        impl GuardEvaluator for DropAll {
            fn evaluate(
                &self,
                _call: &GuardCall<'_>,
            ) -> Result<crate::guard::GuardRows, crate::guard::CallbackError> {
                self.0.set(self.0.get() + 1);
                Ok(Vec::new())
            }
        }
        let program = compile(vec![
            DlClause::datalog(atom("?s", Q, "?o"), Vec::new()).with_ordered_body(vec![
                Compute(Guard::assign("drop", Vec::new(), "?s")),
                Atom(atom("?s", P, "?o")),
            ]),
        ])
        .unwrap();
        for ceiling in [0, 1] {
            let callback = DropAll(std::cell::Cell::new(0));
            let result = evaluate_guarded(
                &program,
                store_of(&[("a", P, "b")]),
                &callback,
                &EvalOptions::default().with_max_join_steps(ceiling),
                None,
            );
            assert_eq!(callback.0.get(), usize::from(ceiling != 0));
            if ceiling == 0 {
                assert!(matches!(result, Err(EvalError::BudgetExhausted { .. })));
            } else {
                let result = result.unwrap();
                assert_eq!(result.work().join_steps(), 1);
                assert_eq!(result.work().committed_facts(), 0);
            }
        }
    }

    #[test]
    fn ordered_guard_outputs_select_quad_predicate_and_graph_keys() {
        use crate::clause::BodyLiteral::{Atom, Guard as Compute};
        use crate::guard::GuardReads;
        const GRAPH: &str = "https://example.org/selected-graph";
        const MISSING_PREDICATE: &str = "https://example.org/absent-guard-predicate";
        const MISSING_GRAPH: &str = "https://example.org/absent-guard-graph";
        struct Partition;
        impl GuardEvaluator for Partition {
            fn evaluate(
                &self,
                _call: &GuardCall<'_>,
            ) -> Result<crate::guard::GuardRows, crate::guard::CallbackError> {
                Ok(vec![
                    vec![surface(R).into(), surface(GRAPH).into()],
                    vec![surface(MISSING_PREDICATE).into(), surface(GRAPH).into()],
                    vec![surface(R).into(), surface(MISSING_GRAPH).into()],
                ])
            }
        }
        let subject = "https://example.org/subject";
        let winner = "https://example.org/selected-value";
        let mut input = store_of(&[(subject, P, "https://example.org/seed")]);
        for (predicate, object, graph) in [
            (R, winner, GRAPH),
            (
                R,
                "https://example.org/wrong-graph-value",
                "https://example.org/other-graph",
            ),
            (
                "https://example.org/other-predicate",
                "https://example.org/wrong-predicate-value",
                GRAPH,
            ),
        ] {
            input
                .insert(
                    &surface(subject),
                    &surface(predicate),
                    &surface(object),
                    &surface(graph),
                )
                .unwrap();
        }
        let program = compile(vec![
            DlClause::datalog(atom("?s", Q, "?o"), Vec::new()).with_ordered_body(vec![
                Atom(atom("?s", P, "?seed")),
                Compute(Guard::new(
                    "partition",
                    Vec::new(),
                    vec!["?p".into(), "?g".into()],
                    GuardReads::Bindings,
                )),
                Atom(ClauseAtom::quad(v("?s"), v("?p"), v("?o"), v("?g"))),
            ]),
        ])
        .unwrap();
        let result = evaluate_guarded(
            &program,
            input,
            &Partition,
            &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
            None,
        )
        .unwrap();
        assert_eq!(result.work().committed_facts(), 1);
        assert!(result.facts().contains_row([
            &surface(subject),
            &surface(Q),
            &surface(winner),
            RelationStore::DEFAULT_GRAPH
        ]));
        assert!(
            result
                .facts()
                .term_id(&surface(MISSING_PREDICATE))
                .is_none()
        );
        assert!(result.facts().term_id(&surface(MISSING_GRAPH)).is_none());
        let derivations = result.derivations().unwrap();
        assert_eq!(derivations.len(), 1);
        assert_eq!(derivations[0].sources()[0].predicate, surface(P));
        assert_eq!(derivations[0].sources()[1].predicate, surface(R));
        assert_eq!(derivations[0].sources()[1].graph, surface(GRAPH));
    }

    /// Consumer admission bounds a large callback Vec before cloning bindings and
    /// computed surfaces. The caller's own eager Vec allocation is a separate seam.
    #[test]
    fn guard_rows_and_negative_callbacks_cannot_expand_past_shared_credits() {
        use crate::guard::GuardReads;
        struct Rows {
            calls: AtomicU64,
            count: usize,
            keep: bool,
        }
        impl GuardEvaluator for Rows {
            fn evaluate(
                &self,
                call: &GuardCall<'_>,
            ) -> Result<crate::guard::GuardRows, crate::guard::CallbackError> {
                self.calls.fetch_add(1, Ordering::Relaxed);
                Ok(if !self.keep {
                    Vec::new()
                } else {
                    (0..self.count)
                        .map(|row| {
                            call.guard
                                .outputs()
                                .iter()
                                .map(|_| surface(&format!("https://example.org/fresh{row}")).into())
                                .collect()
                        })
                        .collect()
                })
            }
        }
        let edb = store_of(&[("https://example.org/s", P, "https://example.org/o")]);
        let producer = DlClause::datalog(atom("?s", Q, "?fresh"), vec![atom("?s", P, "?o")])
            .with_guards(vec![Guard::new(
                "rows",
                vec!["?s".into()],
                vec!["?fresh".into()],
                GuardReads::Bindings,
            )]);
        let exe = compile(vec![producer]).expect("valid producer");
        let rows = Rows {
            calls: AtomicU64::new(0),
            count: 10000,
            keep: true,
        };
        for limit in [0, 1, 17] {
            rows.calls.store(0, Ordering::Relaxed);
            let error = evaluate_guarded(
                &exe,
                edb.clone(),
                &rows,
                &EvalOptions::default()
                    .with_derivation_mode(DerivationMode::Record)
                    .with_max_join_steps(limit),
                None,
            )
            .expect_err("producer refused");
            let EvalError::BudgetExhausted { resource, report } = error else {
                panic!("{error:?}");
            };
            assert_eq!(resource, BudgetResource::JoinSteps);
            assert_eq!(report.join_steps(), limit + 1);
            assert_eq!(report.stored_facts(), 1);
            assert_eq!(rows.calls.load(Ordering::Relaxed), u64::from(limit != 0));
        }
        let exact = evaluate_guarded(
            &exe,
            edb.clone(),
            &rows,
            &EvalOptions::default()
                .with_derivation_mode(DerivationMode::Record)
                .with_max_join_steps(10001),
            None,
        )
        .expect("exact producer boundary");
        assert_eq!(exact.budget().join_steps(), 10001);
        assert_eq!(exact.facts().row_count(), 10001);
        assert!(matches!(
            evaluate_guarded(
                &exe,
                edb.clone(),
                &rows,
                &EvalOptions::default()
                    .with_derivation_mode(DerivationMode::Record)
                    .with_max_join_steps(10000),
                None
            ),
            Err(EvalError::BudgetExhausted { .. })
        ));
        let mut negative_edb = edb;
        for row in 0..100 {
            negative_edb
                .insert(
                    &surface("https://example.org/s"),
                    &surface(R),
                    &surface(&format!("https://example.org/n{row}")),
                    RelationStore::DEFAULT_GRAPH,
                )
                .unwrap();
        }
        let negative = DlClause::datalog(atom("?s", Q, "?o"), vec![atom("?s", P, "?o")])
            .with_negations(vec![Negation::new(
                vec![atom("?s", R, "?local")],
                vec![Guard::new(
                    "reject",
                    vec!["?local".into()],
                    vec![],
                    GuardReads::Bindings,
                )],
            )]);
        let exe = compile(vec![negative]).expect("valid negative traversal");
        let multirow = Rows {
            calls: AtomicU64::new(0),
            count: 10000,
            keep: true,
        };
        let error = evaluate_guarded(
            &exe,
            negative_edb.clone(),
            &multirow,
            &EvalOptions::default()
                .with_derivation_mode(DerivationMode::Record)
                .with_max_join_steps(17),
            None,
        )
        .expect_err("negative callback rows are admitted too");
        let EvalError::BudgetExhausted { report, .. } = error else {
            panic!("{error:?}");
        };
        assert_eq!(report.join_steps(), 18);
        assert_eq!(report.stored_facts(), 101);
        assert_eq!(multirow.calls.load(Ordering::Relaxed), 1);
        let reject = Rows {
            calls: AtomicU64::new(0),
            count: 0,
            keep: false,
        };
        for limit in [0, 1, 17] {
            reject.calls.store(0, Ordering::Relaxed);
            let error = evaluate_guarded(
                &exe,
                negative_edb.clone(),
                &reject,
                &EvalOptions::default()
                    .with_derivation_mode(DerivationMode::Record)
                    .with_max_join_steps(limit),
                None,
            )
            .expect_err("negative traversal refused");
            let EvalError::BudgetExhausted { report, .. } = error else {
                panic!("{error:?}");
            };
            assert_eq!(report.join_steps(), limit + 1);
            assert_eq!(report.stored_facts(), 101);
            assert!(reject.calls.load(Ordering::Relaxed) <= limit / 2);
        }
        let exact = evaluate_guarded(
            &exe,
            negative_edb.clone(),
            &reject,
            &EvalOptions::default()
                .with_derivation_mode(DerivationMode::Record)
                .with_max_join_steps(203),
            None,
        )
        .expect("one positive, group and partition admission, and two per negative row");
        assert_eq!(exact.budget().join_steps(), 203);
        assert_eq!(exact.facts().row_count(), 102);
        assert!(matches!(
            evaluate_guarded(
                &exe,
                negative_edb,
                &reject,
                &EvalOptions::default()
                    .with_derivation_mode(DerivationMode::Record)
                    .with_max_join_steps(202),
                None
            ),
            Err(EvalError::BudgetExhausted { .. })
        ));
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

    /// A conjunction preserves its authored clause index and checks every head's safety.
    #[test]
    fn a_conjunctive_head_checks_every_authored_atom() {
        let rules = vec![DlClause::new(
            vec![HeadDisjunct::new(vec![
                atom("?x", Q, "?y"),
                atom("?x", R, "?y"),
            ])],
            Vec::new(),
            vec![atom("?x", P, "?y")],
        )];
        let executable = compile(rules).expect("ordinary conjunction is Datalog");
        assert_eq!(executable.rule_count(), 1);
        assert_eq!(executable.rule_entry(0).0.head_atoms().count(), 2);
        let unsafe_rule = DlClause::new(
            vec![HeadDisjunct::new(vec![
                atom("?x", Q, "?y"),
                atom("?x", R, "?unbound"),
            ])],
            Vec::new(),
            vec![atom("?x", P, "?y")],
        );
        assert!(
            matches!(compile(vec![unsafe_rule]), Err(EvalError::UnboundHeadVariable { rule: 0, variable }) if variable == "?unbound")
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

    /// The join-step limit stops a Cartesian blow-up that commits almost nothing, and
    /// the returned report is the exact observation that tripped it — not a truncated
    /// answer, not a panic. A limit of exactly the candidates the run enumerates admits
    /// it, and one fewer refuses it.
    #[test]
    fn the_join_step_ceiling_is_a_distinguishable_error() {
        // The productive initial variant scans n first atoms and n² pairs.
        // The other initial variant requires an empty OldOnly suffix and is skipped.
        const JOIN_STEPS_OBSERVED: u64 = 1_211_100;
        // sink(?c, ?c) :- src(?x, ?c), src(?y, ?c). One head fact, n^2 candidates.
        let src = "https://example.org/src";
        let sink = "https://example.org/sink";
        let rules = vec![DlClause::datalog(
            ClauseAtom::positive(v("?c"), sink, v("?c")),
            vec![atom("?x", src, "?c"), atom("?y", src, "?c")],
        )];
        let n = 1100usize; // over a million candidate solutions > the selected finite ceiling
        let mut edb = RelationStore::new();
        for i in 0..n {
            edb.insert(
                &surface(&format!("https://example.org/n{i}")),
                &surface(src),
                &surface("https://example.org/hub"),
                RelationStore::DEFAULT_GRAPH,
            )
            .unwrap();
        }
        let exe = compile(rules).expect("the fixture compiles");
        let limited = EvalOptions::default()
            .with_derivation_mode(DerivationMode::Record)
            .with_max_join_steps(1_048_576);
        let error = evaluate_guarded(&exe, edb.clone(), &NoGuards, &limited, None)
            .expect_err("the join-step limit must be passed");
        let EvalError::BudgetExhausted { resource, report } = error.clone() else {
            panic!("expected a budget refusal, got {error:?}");
        };
        assert_eq!(resource, BudgetResource::JoinSteps);
        assert_eq!(
            report.join_steps(),
            1_048_576 + 1,
            "one task, so the report is exactly the observation that passed the limit"
        );
        assert_eq!(report.join_step_limit(), 1_048_576);
        assert!(report.join_step_limit_stated());
        assert!(report.stored_facts() <= n + 1);
        let rendered = error.to_string();
        assert!(
            rendered
                .contains("1048577 join steps observed, 1048576 permitted (the caller's limit)")
                && rendered.ends_with("raise it with EvalOptions::with_max_join_steps"),
            "{rendered}"
        );

        // The valid neighbour: the same program over the same store under a limit of
        // exactly the candidates it enumerates completes, with its one head fact.
        let admitted = EvalOptions::default()
            .with_derivation_mode(DerivationMode::Record)
            .with_max_join_steps(JOIN_STEPS_OBSERVED);
        let model = evaluate_guarded(&exe, edb.clone(), &NoGuards, &admitted, None)
            .expect("a limit the run fits under admits it");
        assert_eq!(model.facts().row_count(), n + 1);
        assert_eq!(model.budget().join_steps(), JOIN_STEPS_OBSERVED);
        let unbounded =
            evaluate_guarded(&exe, edb.clone(), &NoGuards, &EvalOptions::default(), None)
                .expect("unstated join capacity is unlimited");
        assert_eq!(unbounded.facts().row_count(), n + 1);
        assert_eq!(unbounded.budget().join_steps(), JOIN_STEPS_OBSERVED);
        assert_eq!(unbounded.budget().join_step_limit(), u64::MAX);
        assert!(!unbounded.budget().join_step_limit_stated());
        let short = EvalOptions::default()
            .with_derivation_mode(DerivationMode::Record)
            .with_max_join_steps(JOIN_STEPS_OBSERVED - 1);
        assert!(matches!(
            evaluate_guarded(&exe, edb, &NoGuards, &short, None),
            Err(EvalError::BudgetExhausted {
                resource: BudgetResource::JoinSteps,
                ..
            })
        ));
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
        let n = 400usize; // 160_000 derived facts > the selected finite ceiling
        let mut edb = RelationStore::new();
        for i in 0..n {
            edb.insert(
                &surface(&format!("https://example.org/n{i}")),
                &surface(src),
                &surface("https://example.org/hub"),
                RelationStore::DEFAULT_GRAPH,
            )
            .unwrap();
        }
        let exe = compile(rules).expect("the fixture compiles");
        let limited = EvalOptions::default()
            .with_derivation_mode(DerivationMode::Record)
            .with_max_stored_facts(131_072);
        let error = evaluate_guarded(&exe, edb.clone(), &NoGuards, &limited, None)
            .expect_err("the stored-fact limit must be passed");
        let EvalError::BudgetExhausted { resource, report } = error.clone() else {
            panic!("expected a budget refusal, got {error:?}");
        };
        assert_eq!(resource, BudgetResource::StoredFacts);
        assert_eq!(
            report.stored_facts(),
            131_072_usize,
            "only admitted facts are stored"
        );
        assert_eq!(report.stored_fact_limit(), 131_072);
        assert!(report.join_steps() <= report.join_step_limit());
        let rendered = error.to_string();
        assert!(
            rendered.contains("131073 facts observed, 131072 permitted (the caller's limit)")
                && rendered.ends_with("raise it with EvalOptions::with_max_stored_facts"),
            "{rendered}"
        );

        // The valid neighbour: a limit of exactly the least model's size admits it, and
        // one fact fewer does not.
        let exact = EvalOptions::default()
            .with_derivation_mode(DerivationMode::Record)
            .with_max_stored_facts((n + n * n) as u64);
        let model = evaluate_guarded(&exe, edb.clone(), &NoGuards, &exact, None)
            .expect("a limit of exactly the model's size admits it");
        assert_eq!(model.facts().row_count(), n + n * n);
        let short = EvalOptions::default()
            .with_derivation_mode(DerivationMode::Record)
            .with_max_stored_facts((n + n * n - 1) as u64);
        assert!(matches!(
            evaluate_guarded(&exe, edb, &NoGuards, &short, None),
            Err(EvalError::BudgetExhausted {
                resource: BudgetResource::StoredFacts,
                ..
            })
        ));
    }

    /// An explicit finite ceiling refuses the seeded store. The same store and
    /// derived neighbour complete with unstated capacity settings, beyond the
    /// former 4,194,304-fact default, without changing physical admission.
    #[test]
    fn unstated_capacity_admits_a_store_past_the_former_native_ceiling() {
        const SIDE: usize = 2_048;
        let grid = "https://example.org/grid";
        let small = "https://example.org/small";
        let copy = "https://example.org/copy";
        let rules = vec![DlClause::datalog(
            atom("?x", copy, "?y"),
            vec![atom("?x", small, "?y")],
        )];
        let mut edb = RelationStore::new();
        let objects: Vec<String> = (0..SIDE)
            .map(|j| surface(&format!("https://example.org/o{j}")))
            .collect();
        let (grid, small) = (surface(grid), surface(small));
        for i in 0..=SIDE {
            let subject = surface(&format!("https://example.org/s{i}"));
            for object in &objects {
                edb.insert(&subject, &grid, object, RelationStore::DEFAULT_GRAPH)
                    .unwrap();
            }
        }
        edb.insert(
            &objects[0],
            &small,
            &objects[1],
            RelationStore::DEFAULT_GRAPH,
        )
        .unwrap();
        let seeded = (SIDE + 1) * SIDE + 1;
        assert!(seeded as u64 > 4_194_304);
        let exe = compile(rules).expect("the fixture compiles");
        let native = EvalOptions::default()
            .with_derivation_mode(DerivationMode::Record)
            .with_max_stored_facts(4_194_304);
        let error = evaluate_guarded(&exe, edb.clone(), &NoGuards, &native, None)
            .expect_err("the explicit finite ceiling refuses the seeded store");
        assert_eq!(
            error.to_string(),
            "evaluation exceeded the stored-fact limit: 4196353 facts observed, 4194304 \
             permitted (the caller's limit); raise it with EvalOptions::with_max_stored_facts"
        );
        let model = evaluate_guarded(
            &exe,
            edb,
            &NoGuards,
            &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
            None,
        )
        .expect("unstated capacity has no logical ceiling");
        assert_eq!(model.facts().row_count(), seeded + 1);
        assert_eq!(model.budget().stored_fact_limit(), u64::MAX);
        assert_eq!(model.budget().join_step_limit(), u64::MAX);
        assert!(!model.budget().stored_fact_limit_stated());
        assert!(!model.budget().join_step_limit_stated());
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
            )
            .unwrap();
        }
        let exe = compile(rules).expect("the fixture compiles");
        let error = evaluate_guarded(
            &exe,
            edb,
            &NoGuards,
            &EvalOptions::default()
                .with_derivation_mode(DerivationMode::Record)
                .with_max_term_arena_bytes(1 << 24),
            None,
        )
        .expect_err("the term-arena ceiling must be passed");
        let EvalError::BudgetExhausted { resource, report } = error else {
            panic!("expected a budget refusal, got {error:?}");
        };
        assert_eq!(resource, BudgetResource::TermArenaBytes);
        assert!(report.term_arena_bytes() > (1 << 24));
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
        assert!(first.budget().join_steps() <= first.budget().join_step_limit());
    }

    /// The budget refusal is a `std::error::Error` with a message naming the resource and
    /// both numbers — a diagnostic, not an opaque marker.
    #[test]
    fn budget_and_stratification_errors_render() {
        let error: EvalError = EvalError::BudgetExhausted {
            resource: BudgetResource::JoinSteps,
            report: BudgetReport {
                join_steps: 1_048_576 + 1,
                stored_facts: 3,
                term_arena_bytes: 4,
                ..BudgetReport::default()
                    .governed_by(&EvalOptions::default().with_max_join_steps(1_048_576))
            },
        };
        let rendered = error.to_string();
        let limit = 1_048_576;
        assert!(rendered.contains("join-step limit"), "{rendered}");
        assert!(
            rendered.contains(&format!(
                "{} join steps observed, {limit} permitted (the caller's limit)",
                1_048_576 + 1
            )),
            "{rendered}"
        );
        assert!(
            rendered.ends_with("raise it with EvalOptions::with_max_join_steps"),
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
            assert_eq!(
                again
                    .derivations()
                    .expect("derivation recording was requested"),
                reference
                    .derivations()
                    .expect("derivation recording was requested")
            );
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
                    .expect("derivation recording was requested")
                    .iter()
                    .map(|d| (d.fact().clone(), d.sources().to_vec()))
                    .collect::<Vec<_>>(),
                reference
                    .derivations()
                    .expect("derivation recording was requested")
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
            let row = RowId::from_address(index).unwrap();
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

    #[test]
    fn only_a_delta_covering_the_whole_snapshot_eliminates_old_suffixes() {
        let rel = store_of(&[("a", P, "b"), ("b", Q, "c"), ("c", R, "d")]);
        let indexes = DeltaIndexes { runs: Vec::new() };
        let positions = |delta| {
            JoinSnapshot {
                seed: None,
                rel: &rel,
                delta,
                indexes: &indexes,
            }
            .delta_positions(3)
        };
        assert_eq!(positions(Delta::all(3)), 2..3);
        // Later rounds keep the first and middle anchors, whose OldOnly suffixes
        // can match older facts. A prefix delta is not the whole frozen store.
        assert_eq!(positions(Delta { lo: 1, hi: 3 }), 0..3);
        assert_eq!(positions(Delta { lo: 0, hi: 2 }), 0..3);
        assert_eq!(positions(Delta { lo: 2, hi: 3 }), 0..3);
        assert_eq!(
            JoinSnapshot {
                seed: None,
                rel: &rel,
                delta: Delta::all(3),
                indexes: &indexes,
            }
            .delta_positions(0),
            0..0
        );
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
            RowId::from_index(9).unwrap()
        ));
        assert!(!keep_row_for_scan(
            Scan::Delta,
            Delta { lo: 0, hi: 1 },
            RowId::from_index(9).unwrap()
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
        assert_eq!(
            evaluation
                .derivations()
                .expect("derivation recording was requested"),
            []
        );
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
        assert_eq!(
            evaluation
                .derivations()
                .expect("derivation recording was requested")
                .len(),
            1
        );

        let evaluation = run(vec![present], store_of(&[("x", P, "y")]));
        assert_eq!(
            evaluation
                .derivations()
                .expect("derivation recording was requested"),
            []
        );
    }

    /// An empty program over a seeded store is the store itself.
    #[test]
    fn an_empty_program_derives_nothing() {
        let edb = store_of(&[("a", P, "b")]);
        let evaluation = run(Vec::new(), edb);
        assert_eq!(
            evaluation
                .derivations()
                .expect("derivation recording was requested"),
            []
        );
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
