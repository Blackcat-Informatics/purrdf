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
//! # Limits are parameters; the arena ceiling is a constant
//!
//! The stored-fact limit and the join-step limit are the caller's, on [`EvalOptions`]
//! ([`EvalOptions::with_max_stored_facts`], [`EvalOptions::with_max_join_steps`]), with a
//! default sized for the target ([`DEFAULT_MAX_STORED_FACTS`], [`DEFAULT_MAX_JOIN_STEPS`]).
//! [`MAX_TERM_ARENA_BYTES`] stays a fixed `const`. Exceeding any of them returns
//! [`EvalError::BudgetExhausted`] carrying an accurate [`BudgetReport`] — never a panic,
//! never a truncated answer presented as complete. [`EvalOptions`] also carries the two
//! limits on term generation: term-generating rounds and generated terms.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

mod evidence;
mod factors;
pub use evidence::NegationScope;
use std::fmt;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use rayon::prelude::*;

use crate::clause::{ClauseAtom, ClauseTerm, DlClause, HeadForm};
use crate::cursor::{LendingIterator, VALUE_OBJECT, VALUE_SUBJECT, ValueCursor};
use crate::dictionary::{DictionaryError, DictionaryTerm, TermDictionary, TermIdentity};
use crate::guard::{Guard, GuardCall, GuardEvaluator, GuardSite, Negation, NoGuards};
use crate::id::{RowId, TermId};
use crate::paths::shortest_path;
use purrdf_core::{SmallVec, smallvec};

use crate::plan::{
    AtomOperator, AtomShape, CyclicPlan, Executable, IndexChoice, JoinGroup, POSITION_OBJECT,
    POSITION_SUBJECT, Parsed, PositionPlan, RulePlan, predicate_symbol,
};
use crate::stop::{StopSignal, is_stopped};
use crate::store::{
    Arity, Bound, PartitionRef, RelationStore, ShapeFact, StoreShape, TermInterner,
};

// ── Budgets ─────────────────────────────────────────────────────────────────────

/// The stored-fact limit's default on `wasm32` targets ([`DEFAULT_MAX_STORED_FACTS`]).
///
/// Sized so a saturated store stays comfortably inside a `wasm32` linear memory: at this
/// count the arrangement's columns, the row-id space and the term dictionary together are
/// a few tens of megabytes, well under the ceiling a browser imposes on one module's
/// memory.
pub const WASM_DEFAULT_MAX_STORED_FACTS: u64 = 1 << 17;

/// The join-step limit's default on `wasm32` targets ([`DEFAULT_MAX_JOIN_STEPS`]).
///
/// A browser runs the evaluation on the thread that paints its page, so the default keeps
/// the work an accidentally Cartesian body can enumerate there small; it is the value every
/// target used before the limit became a parameter.
pub const WASM_DEFAULT_MAX_JOIN_STEPS: u64 = 1 << 20;

/// The stored-fact limit's default on every target but `wasm32`
/// ([`DEFAULT_MAX_STORED_FACTS`]).
///
/// A native process addresses the host's memory rather than one linear memory, so the
/// default admits ordinary terminating workloads — a non-recursive rule copying one
/// predicate over tens of thousands of triples, the transitive closure of a thousand-node
/// chain (half a million facts) — and still refuses a least model of more than four
/// million facts before the arrangement, the provenance and the term dictionary grow
/// without bound.
pub const NATIVE_DEFAULT_MAX_STORED_FACTS: u64 = 1 << 22;

/// The join-step limit's default on every target but `wasm32`
/// ([`DEFAULT_MAX_JOIN_STEPS`]): the same value as [`WASM_DEFAULT_MAX_JOIN_STEPS`].
///
/// Unlike the stored-fact limit, the join-step limit does not grow with the target's
/// memory, and the reason is measured: a body's candidate solutions for one round are
/// materialised before the limit is checked. A body joining two unrelated atoms over
/// 20,000 nodes — 400 million candidates — is refused at this default after about two
/// seconds, having held under half a gigabyte; at 268,435,456 join steps (2^28) it had
/// allocated about 20 GB, and was still enumerating, when the process was stopped at a
/// 24 GB address-space cap. A default a Cartesian body can exhaust memory under before it
/// is refused is no limit at all.
///
/// It is no longer what stops a divergent rule. A SHACL shape rule minting one new focus
/// node per iteration used to re-execute for every earlier focus node every iteration,
/// and only the join-step limit refused it promptly (in about seven seconds, and still
/// running after minutes at 2^28). Shape rules are now re-executed only for the focus
/// nodes whose inputs changed, so such a rule is refused by the term limits within a few
/// seconds under this default and under 2^28 alike: by the term-generating round limit
/// when its new terms are of bounded length, and by the term-arena ceiling
/// ([`MAX_TERM_ARENA_BYTES`]) when every term is longer than the last.
///
/// Reach is the caller's to buy: a program that genuinely needs more work — the
/// NON-linear transitive closure of a thousand-node chain, `connected(x, z) :-
/// connected(x, y), connected(y, z)`, which enumerates more than 67 million candidates —
/// completes when the caller raises the limit ([`EvalOptions::with_max_join_steps`]).
/// The LINEAR closure of the same chain fits the default.
pub const NATIVE_DEFAULT_MAX_JOIN_STEPS: u64 = 1 << 20;

/// The stored-fact limit in force when the caller states none
/// ([`EvalOptions::with_max_stored_facts`]): [`WASM_DEFAULT_MAX_STORED_FACTS`] on `wasm32`,
/// [`NATIVE_DEFAULT_MAX_STORED_FACTS`] everywhere else.
///
/// The default differs by target because the memory the store lives in does; it is chosen
/// at compile time from the target architecture, never by a Cargo feature. A caller who
/// needs one answer on every target states the limit, and every caller passing the same
/// limit gets the same answer or the same refusal.
#[cfg(target_arch = "wasm32")]
pub const DEFAULT_MAX_STORED_FACTS: u64 = WASM_DEFAULT_MAX_STORED_FACTS;

/// The stored-fact limit in force when the caller states none
/// ([`EvalOptions::with_max_stored_facts`]): [`WASM_DEFAULT_MAX_STORED_FACTS`] on `wasm32`,
/// [`NATIVE_DEFAULT_MAX_STORED_FACTS`] everywhere else.
///
/// The default differs by target because the memory the store lives in does; it is chosen
/// at compile time from the target architecture, never by a Cargo feature. A caller who
/// needs one answer on every target states the limit, and every caller passing the same
/// limit gets the same answer or the same refusal.
#[cfg(not(target_arch = "wasm32"))]
pub const DEFAULT_MAX_STORED_FACTS: u64 = NATIVE_DEFAULT_MAX_STORED_FACTS;

/// The join-step limit in force when the caller states none
/// ([`EvalOptions::with_max_join_steps`]): [`WASM_DEFAULT_MAX_JOIN_STEPS`] on `wasm32`,
/// [`NATIVE_DEFAULT_MAX_JOIN_STEPS`] everywhere else — today the same value, for the
/// reasons [`NATIVE_DEFAULT_MAX_JOIN_STEPS`] gives.
#[cfg(target_arch = "wasm32")]
pub const DEFAULT_MAX_JOIN_STEPS: u64 = WASM_DEFAULT_MAX_JOIN_STEPS;

/// The join-step limit in force when the caller states none
/// ([`EvalOptions::with_max_join_steps`]): [`WASM_DEFAULT_MAX_JOIN_STEPS`] on `wasm32`,
/// [`NATIVE_DEFAULT_MAX_JOIN_STEPS`] everywhere else — today the same value, for the
/// reasons [`NATIVE_DEFAULT_MAX_JOIN_STEPS`] gives.
#[cfg(not(target_arch = "wasm32"))]
pub const DEFAULT_MAX_JOIN_STEPS: u64 = NATIVE_DEFAULT_MAX_JOIN_STEPS;

/// The maximum bytes of interned term surfaces one evaluation's store may hold
/// ([`RelationStore::term_bytes`]).
///
/// Facts are counted by the stored-fact limit, but a fact set of a legal size can still be
/// arbitrarily large if its terms are: one relation of a thousand megabyte-long IRIs is a
/// hundred-fold smaller in facts and a thousand-fold larger in bytes. This ceiling closes
/// that gap. It is checked on the seeded store and again after every round, so no future
/// term-minting extension can grow past it unobserved. Unlike the stored-fact and
/// join-step limits it is not a parameter: it is the same on every target.
pub const MAX_TERM_ARENA_BYTES: usize = 1 << 24;

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
/// Both are parameters ([`Self::with_max_stored_facts`], [`Self::with_max_join_steps`])
/// with a default sized for the target: [`DEFAULT_MAX_STORED_FACTS`] and
/// [`DEFAULT_MAX_JOIN_STEPS`], which are the `wasm32` values
/// ([`WASM_DEFAULT_MAX_STORED_FACTS`], [`WASM_DEFAULT_MAX_JOIN_STEPS`]) on `wasm32` and the
/// native values ([`NATIVE_DEFAULT_MAX_STORED_FACTS`], [`NATIVE_DEFAULT_MAX_JOIN_STEPS`])
/// everywhere else. A fixed ceiling sized for a browser refused ordinary terminating rule
/// sets on a server — a single non-recursive rule copying a predicate over 70,000 triples
/// — and a fixed ceiling sized for a server would let a browser evaluation exhaust its
/// linear memory, so neither value can serve every target.
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
    /// The caller's stored-fact limit, or `None` for the target's default.
    stored_facts: Option<u64>,
    /// The caller's join-step limit, or `None` for the target's default.
    join_steps: Option<u64>,
    /// The caller's term-generating round limit, or `None` for the default.
    term_generating_rounds: Option<u64>,
    /// The caller's generated-term budget, or `None` for the input-derived default.
    generated_terms: Option<u64>,
}

impl EvalOptions {
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

    /// The stored-fact limit in force: the caller's, or [`DEFAULT_MAX_STORED_FACTS`].
    #[must_use]
    pub fn max_stored_facts(&self) -> u64 {
        self.stored_facts.unwrap_or(DEFAULT_MAX_STORED_FACTS)
    }

    /// The caller's stored-fact limit, or `None` for the target's default.
    #[must_use]
    pub fn stated_max_stored_facts(&self) -> Option<u64> {
        self.stored_facts
    }

    /// The join-step limit in force: the caller's, or [`DEFAULT_MAX_JOIN_STEPS`].
    #[must_use]
    pub fn max_join_steps(&self) -> u64 {
        self.join_steps.unwrap_or(DEFAULT_MAX_JOIN_STEPS)
    }

    /// The caller's join-step limit, or `None` for the target's default.
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

/// The four limits one evaluation runs under, and which of them the caller stated.
///
/// The stated flags are one bit set rather than four `bool`s, which keeps a
/// [`BudgetReport`] — carried by value in every [`EvalError`] — small.
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
    /// Which limits the caller stated: the `STATED_*` bits.
    stated: u8,
}

/// [`Limits::stated`]: the join-step limit is the caller's.
const STATED_JOIN_STEPS: u8 = 1;
/// [`Limits::stated`]: the stored-fact limit is the caller's.
const STATED_STORED_FACTS: u8 = 1 << 1;
/// [`Limits::stated`]: the term-generating round limit is the caller's.
const STATED_TERM_GENERATING_ROUNDS: u8 = 1 << 2;
/// [`Limits::stated`]: the generated-term budget is the caller's.
const STATED_GENERATED_TERMS: u8 = 1 << 3;

impl Limits {
    /// The limits `options` states, over a seeded store of `input_terms` distinct terms.
    fn of(options: &EvalOptions, input_terms: u64) -> Self {
        let bit = |stated: Option<u64>, flag: u8| if stated.is_some() { flag } else { 0 };
        Self {
            join_steps: options.max_join_steps(),
            stored_facts: options.max_stored_facts(),
            term_generating_rounds: options.max_term_generating_rounds(),
            generated_terms: options.generated_term_budget(input_terms),
            stated: bit(options.stated_max_join_steps(), STATED_JOIN_STEPS)
                | bit(options.stated_max_stored_facts(), STATED_STORED_FACTS)
                | bit(
                    options.stated_max_term_generating_rounds(),
                    STATED_TERM_GENERATING_ROUNDS,
                )
                | bit(options.stated_max_generated_terms(), STATED_GENERATED_TERMS),
        }
    }

    /// Whether the caller stated the limit `flag` names.
    const fn is_stated(self, flag: u8) -> bool {
        self.stated & flag != 0
    }
}

/// What an evaluation actually consumed, against the limits it ran under.
///
/// Returned on success ([`Evaluation::budget`]) and on failure
/// ([`EvalError::BudgetExhausted`]) alike; on failure the field naming the exhausted
/// resource holds the observation that proved the limit was passed, so the report is
/// never rounded down to the limit it exceeded.
///
/// Every measurement is a deterministic function of the input: the same program over the
/// same facts reports the same numbers on every target. The limits it carries are the ones
/// in force ([`EvalOptions`]), so a default limit reads differently on `wasm32` than
/// elsewhere ([`DEFAULT_MAX_STORED_FACTS`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetReport {
    /// Candidate solutions enumerated, against [`Self::join_step_limit`].
    join_steps: u64,
    /// Facts held by the store, against [`Self::stored_fact_limit`].
    stored_facts: usize,
    /// Interned term surface bytes, against [`MAX_TERM_ARENA_BYTES`].
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

impl Default for BudgetReport {
    /// The zero measurement, governed by the default limits.
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
        let kept = self.limits.stated & !(STATED_JOIN_STEPS | STATED_STORED_FACTS);
        Self {
            limits: Limits {
                join_steps: capacity.join_steps,
                stored_facts: capacity.stored_facts,
                stated: kept | (capacity.stated & (STATED_JOIN_STEPS | STATED_STORED_FACTS)),
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

    /// Whether the join-step limit was the caller's rather than
    /// [`DEFAULT_MAX_JOIN_STEPS`].
    pub fn join_step_limit_stated(self) -> bool {
        self.limits.is_stated(STATED_JOIN_STEPS)
    }

    /// The stored-fact limit the run was governed by ([`EvalOptions::max_stored_facts`]).
    pub fn stored_fact_limit(self) -> u64 {
        self.limits.stored_facts
    }

    /// Whether the stored-fact limit was the caller's rather than
    /// [`DEFAULT_MAX_STORED_FACTS`].
    pub fn stored_fact_limit_stated(self) -> bool {
        self.limits.is_stated(STATED_STORED_FACTS)
    }

    /// Facts held by the store when evaluation stopped (seeded plus derived).
    pub fn stored_facts(self) -> usize {
        self.stored_facts
    }

    /// Interned term surface bytes held by the store when evaluation stopped.
    pub fn term_arena_bytes(self) -> usize {
        self.term_arena_bytes
    }

    /// Term-generating rounds run ([`EvalOptions`]). Always zero for a guard-free
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
        self.limits.is_stated(STATED_TERM_GENERATING_ROUNDS)
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
        self.limits.is_stated(STATED_GENERATED_TERMS)
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
    /// [`MAX_TERM_ARENA_BYTES`] — too many interned term surface bytes.
    TermArenaBytes,
    /// The term-generating round limit ([`EvalOptions`]) — too many rounds committed a
    /// guard-computed term.
    TermGeneratingRounds,
    /// The generated-term budget ([`EvalOptions`]) — too many terms added to the store.
    GeneratedTerms,
}

impl BudgetResource {
    /// The limit this resource is measured against in `report`.
    #[must_use]
    pub fn limit(self, report: BudgetReport) -> u64 {
        match self {
            Self::JoinSteps => report.limits.join_steps,
            Self::StoredFacts => report.limits.stored_facts,
            Self::TermArenaBytes => MAX_TERM_ARENA_BYTES as u64,
            Self::TermGeneratingRounds => report.limits.term_generating_rounds,
            Self::GeneratedTerms => report.limits.generated_terms,
        }
    }

    /// The observation that tripped this resource, taken from `report`.
    #[must_use]
    pub fn observed(self, report: BudgetReport) -> u64 {
        match self {
            Self::JoinSteps => report.join_steps,
            Self::StoredFacts => report.stored_facts as u64,
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
struct StepGovernor {
    /// Remaining successful credits, including the full u64 range.
    allowance: u64,
    /// Candidates enumerated so far.
    consumed: u64,
    /// One shared round-wide credit pool.
    pool: Arc<StepPool>,
}

/// Successful credits and refusal are separate so the full-width ceiling cannot
/// lose its final credit or silently truncate on an unrepresentable sentinel.
#[derive(Debug)]
struct StepPool {
    used: AtomicU64,
    refused: AtomicBool,
}

impl StepGovernor {
    /// A governor permitting `allowance` candidates.
    fn new(allowance: u64) -> Self {
        Self {
            allowance,
            consumed: 0,
            pool: Arc::new(StepPool {
                used: AtomicU64::new(0),
                refused: AtomicBool::new(false),
            }),
        }
    }

    /// Whether the allowance is spent — the next candidate may NOT be enumerated.
    #[inline]
    fn spent(&self) -> bool {
        self.pool.refused.load(Ordering::Relaxed)
    }

    /// Reserve a candidate; the single refusal sentinel is never expanded.
    #[inline]
    fn charge(&mut self) -> bool {
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
    fn observed(&self) -> u64 {
        self.pool
            .used
            .load(Ordering::Relaxed)
            .saturating_add(u64::from(self.spent()))
    }

    /// A rule-local counter sharing the round's credit pool.
    fn task(&self) -> Self {
        Self {
            allowance: self.allowance,
            consumed: 0,
            pool: Arc::clone(&self.pool),
        }
    }
}

// ── Errors ──────────────────────────────────────────────────────────────────────

/// Why an evaluation could not run, or could not run to completion.
///
/// Every variant is a hard refusal. There is no partial answer and no best-effort mode:
/// an answer this crate returns is the complete least model of the program it was given.
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
    Storage(purrdf_lex::allocation::StorageError),
    /// The original row-evidence account refused or encountered a broken invariant.
    Evidence(purrdf_core::evidence::EvidenceWorkspaceError),
    /// The full rule bundle violates an assessor-only declaration.
    NeverDerive(crate::admission::AdmissionRefusal),
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
    /// The stored-fact or join-step limit in force ([`EvalOptions`]), or the fixed
    /// term-arena ceiling, was passed. The report is accurate at the point evaluation
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
        /// round, in authored program order.
        rules: Vec<usize>,
        /// Consumption when evaluation stopped, the limit in force included.
        report: BudgetReport,
    },
    /// The caller's [`crate::stop::StopSignal`] fired at a round boundary.
    ///
    /// A refusal exactly as total as [`Self::BudgetExhausted`]: there is no partial least
    /// model here either, and the rounds already committed are not an answer to anything.
    /// It is a DISTINCT variant because the two say different things to a caller — a limit
    /// was passed by the program and the data, whereas this run was stopped by the host
    /// that asked for it — and only one of them is a reason to change the input or the
    /// limit.
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

/// Where a report's stored-fact limit came from, for a diagnostic.
#[must_use]
pub fn stored_fact_limit_origin(report: BudgetReport) -> &'static str {
    if report.limits.is_stated(STATED_STORED_FACTS) {
        "the caller's limit"
    } else {
        "the default for this target"
    }
}

/// Where a report's join-step limit came from, for a diagnostic.
#[must_use]
pub fn join_step_limit_origin(report: BudgetReport) -> &'static str {
    if report.limits.is_stated(STATED_JOIN_STEPS) {
        "the caller's limit"
    } else {
        "the default for this target"
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
            report.stored_facts,
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

/// Where a report's term-generating round limit came from, for a diagnostic.
#[must_use]
pub fn term_generating_round_limit_origin(report: BudgetReport) -> String {
    if report.limits.is_stated(STATED_TERM_GENERATING_ROUNDS) {
        "the caller's limit".to_owned()
    } else {
        "the default".to_owned()
    }
}

/// Where a report's generated-term budget came from, for a diagnostic.
#[must_use]
pub fn generated_term_budget_origin(report: BudgetReport) -> String {
    if report.limits.is_stated(STATED_GENERATED_TERMS) {
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
                    None => write!(
                        f,
                        "evaluation exceeded the fixed {} ceiling: {} observed, {} permitted",
                        resource.name(),
                        resource.observed(*report),
                        resource.limit(*report)
                    ),
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
            _ => None,
        }
    }
}

purrdf_lex::variant_from!(impl<E> EvalError<E> {
    Storage(purrdf_lex::allocation::StorageError),
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

    /// The matched positive body facts, in authored body order.
    pub fn sources(&self) -> &[ShapeFact<N>] {
        &self.sources
    }

    /// The derivation's proof height: `1 + max(source proof heights)`, seeds being 0.
    pub fn proof_height(&self) -> u32 {
        self.proof_height
    }
}

/// A completed evaluation: the saturated store, its derivations and the budget report.
#[derive(Debug, Clone)]
pub struct Evaluation<D: TermDictionary = TermInterner, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The least model: the seeded EDB plus every derived fact.
    facts: RelationStore<D, N>,
    /// Every derivation, in lexical `(fact, rule, sources)` order.
    derivations: Vec<Derivation<N>>,
    /// What the run consumed, against the limits it ran under.
    budget: BudgetReport,
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

    /// Every derivation, in a total lexical order.
    pub fn derivations(&self) -> &[Derivation<N>] {
        &self.derivations
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
/// not a single unquantified atom is reported by head form, a non-stratifiable program is
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
        let index = row.resident_index();
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
    /// in [`extend_atom`], so a heap frame is one allocation per intermediate solution
    /// — the join's most frequent allocation by a wide margin, and one the evaluator
    /// paid unconditionally.
    ///
    /// Capacity 8 is measured, not assumed: 8 is the widest frame the OWL 2 RL rule
    /// table and this workspace's own suites produce, so every real frame lives inline.
    /// The native identity keeps its NonZeroU64 niche in `Option`; a host identity
    /// determines its own inline footprint. A ninth variable spills to the heap.
    bindings: SmallVec<[Option<I>; 8]>,
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

/// Keeps the original dictionary pin alive while a guard borrows its text.
enum SlotSurface<'s, L: AsRef<str>> {
    Pinned(L),
    Computed(&'s str),
}

impl<L: AsRef<str>> AsRef<str> for SlotSurface<'_, L> {
    fn as_ref(&self) -> &str {
        match self {
            Self::Pinned(pin) => pin.as_ref(),
            Self::Computed(surface) => surface,
        }
    }
}

impl<'s, I: TermIdentity> SlotValue<'s, I> {
    /// Resolve only at the lexical boundary, retaining the supplied lexical owner.
    fn surface<D: TermDictionary<Id = I>, const N: usize>(
        self,
        rel: &'s RelationStore<D, N>,
    ) -> Result<Option<SlotSurface<'s, D::Lexical<'s>>>, D::Error>
    where
        Arity<N>: StoreShape<N>,
    {
        rel.interner().check_read()?;
        let surface = match self {
            Self::Interned(id) => Some(SlotSurface::Pinned(rel.interner().resolve_term(id)?)),
            Self::Computed(surface) => Some(SlotSurface::Computed(surface)),
            Self::Unbound => None,
        };
        rel.interner().check_read()?;
        Ok(surface)
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
    /// The lexical surface bound in `slot`, if any.
    fn surface<'s, D: TermDictionary<Id = I>>(
        &'s self,
        slot: usize,
        rel: &'s RelationStore<D, N>,
    ) -> Result<Option<SlotSurface<'s, D::Lexical<'s>>>, D::Error> {
        self.value(slot).surface(rel)
    }

    /// Bind `slot` to `surface`: as an interned id when the store knows the term, as a
    /// computed surface otherwise.
    fn bind_surface<D: TermDictionary<Id = I>>(
        &mut self,
        slot: usize,
        surface: &str,
        rel: &RelationStore<D, N>,
    ) -> Result<(), D::Error> {
        rel.interner().check_read()?;
        let id = rel.interner().lookup_term(surface)?;
        rel.interner().check_read()?;
        match id {
            Some(id) => self.bindings[slot] = Some(id),
            None => self.computed.push((slot, surface.into())),
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
fn extend_slot_solutions<D: TermDictionary, const N: usize>(
    operator: &AtomOperator<N>,
    rel: &RelationStore<D, N>,
    delta: Delta,
    scan: Scan,
    solutions: &[SlotSolution<D::Id, N>],
    governor: &mut StepGovernor,
) -> Result<Vec<SlotSolution<D::Id, N>>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    match scan {
        Scan::Delta => {
            extend_slot_operator::<SCAN_DELTA, D, N>(operator, rel, delta, solutions, governor)
        }
        Scan::Full => {
            extend_slot_operator::<SCAN_FULL, D, N>(operator, rel, delta, solutions, governor)
        }
        Scan::OldOnly => {
            extend_slot_operator::<SCAN_OLD_ONLY, D, N>(operator, rel, delta, solutions, governor)
        }
    }
}

/// Dispatch one operator to its statically-shaped index kernel.
fn extend_slot_operator<const SCAN: u8, D: TermDictionary, const N: usize>(
    operator: &AtomOperator<N>,
    rel: &RelationStore<D, N>,
    delta: Delta,
    solutions: &[SlotSolution<D::Id, N>],
    governor: &mut StepGovernor,
) -> Result<Vec<SlotSolution<D::Id, N>>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    match operator.index() {
        IndexChoice::Any => {
            extend_atom::<SCAN, INDEX_ANY, D, N>(operator, rel, delta, solutions, governor)
        }
        IndexChoice::Subject => {
            extend_atom::<SCAN, INDEX_SUBJECT, D, N>(operator, rel, delta, solutions, governor)
        }
        IndexChoice::Object => {
            extend_atom::<SCAN, INDEX_OBJECT, D, N>(operator, rel, delta, solutions, governor)
        }
        IndexChoice::Both => {
            extend_atom::<SCAN, INDEX_BOTH, D, N>(operator, rel, delta, solutions, governor)
        }
    }
}

/// The shared atom kernel: address the partitions the predicate and optional graph
/// admit, and inside each one select exactly the rows the `(subject, object)` index admits.
///
/// The two halves are independent, which is the whole performance argument. `INDEX` is a
/// monomorphization parameter, so the subject/object access path is decided at compile
/// time and is the SAME path a predicate-keyed store would have taken. The partition half
/// is a single ordered-map probe whenever the predicate and graph are known
/// ([`AtomShape::addresses_one_partition`]) — which is every atom of every rule that
/// names its predicate — and only degrades to a sweep of matching partitions when the rule
/// genuinely quantifies over the predicate or the graph.
fn extend_atom<const SCAN: u8, const INDEX: u8, D: TermDictionary, const N: usize>(
    operator: &AtomOperator<N>,
    rel: &RelationStore<D, N>,
    delta: Delta,
    solutions: &[SlotSolution<D::Id, N>],
    governor: &mut StepGovernor,
) -> Result<Vec<SlotSolution<D::Id, N>>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    let rel = rel.read_source(operator.shape().read_source());
    let shape = operator.shape();
    let body_index = operator.body_index();
    let constants = constant_positions(shape, rel)?;
    let mut next = Vec::new();

    'solutions: for solution in solutions {
        if governor.spent() {
            break;
        }
        // Resolve every stored position: constants were resolved once above, a bound variable
        // reads its frame slot, a free variable is bound by the scan.
        let mut values = [PositionValue::Free; N];
        for position in 0..N {
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

        for partition in rel.partitions_for(values.map(PositionValue::known)) {
            let mut cursor = partition.select(bound);
            while let Some((subject, object, row)) = cursor.next() {
                if !keep_row::<SCAN>(delta, row) {
                    continue;
                }
                let matched = partition.row(subject, object);
                // The generalized diagonal filter: any two positions holding the same
                // variable must agree, whichever two they are.
                if !shape
                    .equalities()
                    .iter()
                    .all(|&(left, right)| matched[left] == matched[right])
                {
                    continue;
                }
                if !governor.charge() {
                    break 'solutions;
                }
                let mut merged = solution.clone();
                for (position, plan) in shape.positions().iter().enumerate() {
                    if let PositionPlan::Free(slot) = plan {
                        merged.bindings[*slot] = Some(matched[position]);
                    }
                }
                merged.sources.push(SourceRow {
                    source: shape.read_source(),
                    body_index,
                    terms: matched,
                    row,
                });
                next.push(merged);
            }
        }
    }
    rel.interner().check_read()?;
    Ok(next)
}

// ── The leapfrog triejoin ───────────────────────────────────────────────────────

/// One relation's distinct, sorted trie-level values under a fixed semi-naive scan.
///
/// `SCAN` stays a monomorphization parameter here exactly as it is in the binary kernels,
/// so a leapfrog seek pays no per-row branch on the scan mode either.
#[derive(Debug)]
struct FilteredValueCursor<'a, const SCAN: u8, const COLUMN: u8, I: TermIdentity = TermId> {
    /// The underlying globally value-ordered cursor.
    rows: ValueCursor<'a, COLUMN, I>,
    /// The round's delta span.
    delta: Delta,
    /// The current distinct admitted value, if any.
    current: Option<I>,
}

impl<'a, const SCAN: u8, const COLUMN: u8, I: TermIdentity>
    FilteredValueCursor<'a, SCAN, COLUMN, I>
{
    /// A filtered cursor positioned at its first admitted value.
    fn new(rows: ValueCursor<'a, COLUMN, I>, delta: Delta) -> Self {
        let mut cursor = Self {
            rows,
            delta,
            current: None,
        };
        cursor.fill(None);
        cursor
    }

    /// Fill `current` with the next distinct admitted value, skipping `prior`.
    fn fill(&mut self, prior: Option<I>) -> Option<I> {
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
    fn seek(&mut self, target: I) -> Option<I> {
        if self.current.is_some_and(|value| value >= target) {
            return self.current;
        }
        self.rows.seek(target);
        self.fill(None)
    }

    /// Advance past the current value to the next distinct admitted one.
    fn advance(&mut self) -> Option<I> {
        let prior = self.current;
        self.fill(prior)
    }
}

/// Runtime scan/orientation selection at cursor construction.
///
/// Each variant wraps a const-generic filtered cursor, so the scan mode is resolved once
/// per cursor rather than per seek.
#[derive(Debug)]
enum LeapfrogValueCursor<'a, I: TermIdentity = TermId> {
    /// Subject-ordered, delta rows only.
    DeltaSubject(FilteredValueCursor<'a, SCAN_DELTA, VALUE_SUBJECT, I>),
    /// Subject-ordered, every row.
    FullSubject(FilteredValueCursor<'a, SCAN_FULL, VALUE_SUBJECT, I>),
    /// Subject-ordered, non-delta rows only.
    OldOnlySubject(FilteredValueCursor<'a, SCAN_OLD_ONLY, VALUE_SUBJECT, I>),
    /// Object-ordered, delta rows only.
    DeltaObject(FilteredValueCursor<'a, SCAN_DELTA, VALUE_OBJECT, I>),
    /// Object-ordered, every row.
    FullObject(FilteredValueCursor<'a, SCAN_FULL, VALUE_OBJECT, I>),
    /// Object-ordered, non-delta rows only.
    OldOnlyObject(FilteredValueCursor<'a, SCAN_OLD_ONLY, VALUE_OBJECT, I>),
}

impl<'a, I: TermIdentity> LeapfrogValueCursor<'a, I> {
    /// A subject-ordered cursor under `scan`.
    fn subject(rows: ValueCursor<'a, VALUE_SUBJECT, I>, scan: Scan, delta: Delta) -> Self {
        match scan {
            Scan::Delta => Self::DeltaSubject(FilteredValueCursor::new(rows, delta)),
            Scan::Full => Self::FullSubject(FilteredValueCursor::new(rows, delta)),
            Scan::OldOnly => Self::OldOnlySubject(FilteredValueCursor::new(rows, delta)),
        }
    }

    /// An object-ordered cursor under `scan`.
    fn object(rows: ValueCursor<'a, VALUE_OBJECT, I>, scan: Scan, delta: Delta) -> Self {
        match scan {
            Scan::Delta => Self::DeltaObject(FilteredValueCursor::new(rows, delta)),
            Scan::Full => Self::FullObject(FilteredValueCursor::new(rows, delta)),
            Scan::OldOnly => Self::OldOnlyObject(FilteredValueCursor::new(rows, delta)),
        }
    }

    /// The cursor's current value.
    fn current(&self) -> Option<I> {
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
    fn seek(&mut self, target: I) -> Option<I> {
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
    fn advance(&mut self) -> Option<I> {
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
struct LeapfrogIntersection<'a, I: TermIdentity = TermId> {
    /// One cursor per relation constrained on the descent variable.
    cursors: Vec<LeapfrogValueCursor<'a, I>>,
}

impl<'a, I: TermIdentity> LeapfrogIntersection<'a, I> {
    /// An intersection over `cursors`.
    fn new(cursors: Vec<LeapfrogValueCursor<'a, I>>) -> Self {
        Self { cursors }
    }

    /// The next value present in EVERY cursor.
    ///
    /// The first cursor advances past the returned value before control returns, so
    /// repeated calls enumerate the intersection without duplicates.
    fn next(&mut self) -> Option<I> {
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
    fn contains(&mut self, wanted: I) -> bool {
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
fn cycle_atom_partition<'a, D: TermDictionary, const N: usize>(
    shape: &AtomShape<N>,
    rel: &'a RelationStore<D, N>,
) -> Result<Option<PartitionRef<'a, D::Id, N>>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    let constants = constant_positions(shape, rel)?;
    let known = constants.map(|value| value.and_then(PositionValue::known));
    Ok(rel.partition_for(&known))
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

/// Build one cycle atom's trie cursor for `variable_slot`, constrained by any binding of
/// its other variable.
fn cycle_atom_cursor<'a, I: TermIdentity, const N: usize>(
    partition: PartitionRef<'a, I, N>,
    shape: &AtomShape<N>,
    variable_slot: usize,
    solution: &SlotSolution<I, N>,
    scan: Scan,
    delta: Delta,
) -> Option<LeapfrogValueCursor<'a, I>>
where
    Arity<N>: StoreShape<N>,
{
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
}

impl<D: TermDictionary, const N: usize> LeapfrogRun<'_, D, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Capture the unique fully-ground row for every cycle atom, in the component's
    /// authored atom order.
    ///
    /// Returns `false` — restoring `solution` — if a scan-mode constraint excludes any of
    /// them, which is how the semi-naive decomposition is enforced on a multiway group.
    fn append_sources(&self, solution: &mut SlotSolution<D::Id, N>) -> Result<bool, D::Error> {
        let original = solution.sources.len();
        for &planned in self.cycle.atoms() {
            let operator = self.plan.operator_at(planned.positive_position());
            let shape = operator.shape();
            let (subject_slot, object_slot) = cycle_atom_slots(shape);
            let (Some(subject), Some(object), Some(partition)) = (
                solution.get(subject_slot),
                solution.get(object_slot),
                cycle_atom_partition(shape, self.rel)?,
            ) else {
                solution.sources.truncate(original);
                return Ok(false);
            };
            let scan = self.scan.at(planned.positive_position());
            let mut rows = partition.select(Bound::Both(subject, object));
            let mut matched = None;
            while let Some((subject, object, row)) = rows.next() {
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

    /// Recursive variable descent for one certified cycle component.
    fn recurse(
        &self,
        variable_position: usize,
        solution: &mut SlotSolution<D::Id, N>,
        out: &mut Vec<SlotSolution<D::Id, N>>,
        governor: &mut StepGovernor,
    ) -> Result<(), D::Error> {
        if governor.spent() {
            return Ok(());
        }
        if variable_position == self.cycle.variable_slots().len() {
            if governor.charge() && self.append_sources(solution)? {
                out.push(solution.clone());
                solution
                    .sources
                    .truncate(solution.sources.len() - self.cycle.atoms().len());
            }
            return Ok(());
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
            let scan = self.scan.at(planned.positive_position());
            let Some(partition) = cycle_atom_partition(shape, self.rel)? else {
                return Ok(());
            };
            let Some(cursor) =
                cycle_atom_cursor(partition, shape, variable_slot, solution, scan, self.delta)
            else {
                return Ok(());
            };
            cursors.push(cursor);
        }
        if cursors.is_empty() {
            return Ok(());
        }
        let mut intersection = LeapfrogIntersection::new(cursors);

        if let Some(value) = externally_bound {
            if intersection.contains(value) {
                self.recurse(variable_position + 1, solution, out, governor)?;
            }
            return Ok(());
        }

        while let Some(value) = intersection.next() {
            debug_assert!(solution.bindings[variable_slot].is_none());
            solution.bindings[variable_slot] = Some(value);
            self.recurse(variable_position + 1, solution, out, governor)?;
            solution.bindings[variable_slot] = None;
            if governor.spent() {
                break;
            }
        }
        Ok(())
    }
}

/// Extend every partial solution through one certified cyclic component, materialising no
/// binary intermediate relation.
fn extend_solutions_leapfrog<D: TermDictionary, const N: usize>(
    run: &LeapfrogRun<'_, D, N>,
    solutions: &[SlotSolution<D::Id, N>],
    governor: &mut StepGovernor,
) -> Result<Vec<SlotSolution<D::Id, N>>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    run.rel.interner().check_read()?;
    let mut out = Vec::new();
    for solution in solutions {
        if governor.spent() {
            break;
        }
        let mut working = solution.clone();
        run.recurse(0, &mut working, &mut out, governor)?;
    }
    run.rel.interner().check_read()?;
    Ok(out)
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
fn join_body<D: TermDictionary, const N: usize>(
    plan: &RulePlan<N>,
    snapshot: JoinSnapshot<'_, D, N>,
    strategy: JoinStrategy,
    governor: &mut StepGovernor,
) -> Result<Vec<SlotSolution<D::Id, N>>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    snapshot.rel.interner().check_read()?;
    let leapfrog = plan.has_cyclic_subplan()
        && matches!(strategy, JoinStrategy::Planned)
        && plan.operators().iter().all(|operator| {
            operator.shape().read_source() == crate::clause::ReadSource::Evaluation
        });
    let joined = if plan.positive().is_empty() {
        // The empty conjunction is relational identity: one empty substitution, so an
        // unconditional or NAF-only rule fires exactly once. Its head is suppressed on the
        // following round by the store's own membership test.
        if governor.charge() {
            Ok(vec![SlotSolution::empty(plan.variables().len())])
        } else {
            Ok(Vec::new())
        }
    } else if leapfrog {
        join_positive_leapfrog(plan, snapshot, governor)
    } else {
        join_positive_binary(plan, snapshot, governor)
    }?;
    snapshot.rel.interner().check_read()?;
    Ok(joined)
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
    /// Every position before the last requires an OldOnly suffix. When the
    /// delta covers the entire frozen store, that suffix is provably empty;
    /// visiting its earlier prefixes can produce no solution. Partial deltas
    /// retain every position, including positions that become productive later.
    fn delta_positions(self, positive_count: usize) -> std::ops::Range<usize> {
        let first = if self.delta == Delta::all(self.rel.row_count()) {
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
    Ok(snapshot
        .rel
        .partitions_for(positions)
        .any(|partition| partition.has_row_from(snapshot.delta.lo)))
}

/// The indexed binary positive join: every planned operator in execution order, for every
/// semi-naive delta position whose delta atom can match ([`delta_can_match`]).
fn join_positive_binary<D: TermDictionary, const N: usize>(
    plan: &RulePlan<N>,
    snapshot: JoinSnapshot<'_, D, N>,
    governor: &mut StepGovernor,
) -> Result<Vec<SlotSolution<D::Id, N>>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    snapshot.rel.interner().check_read()?;
    let operators = plan.operators();
    let mut all: Vec<SlotSolution<D::Id, N>> = Vec::new();
    let base = operators
        .iter()
        .any(|operator| operator.shape().read_source() == crate::clause::ReadSource::Base);
    let anchors = if base {
        0..1
    } else {
        snapshot.delta_positions(operators.len())
    };
    for delta_position in anchors {
        if !base && !delta_can_match(&operators[delta_position], snapshot)? {
            continue;
        }
        let mut partial = vec![SlotSolution::empty(plan.variables().len())];
        for (position, operator) in operators.iter().enumerate() {
            partial = extend_slot_solutions(
                operator,
                snapshot.rel,
                snapshot.delta,
                if base {
                    Scan::Full
                } else {
                    scan_for(position, delta_position)
                },
                &partial,
                governor,
            )?;
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
    snapshot.rel.interner().check_read()?;
    Ok(all)
}

/// The hybrid positive join for a rule with at least one certified cyclic subplan: each
/// physical group in execution order, for every semi-naive delta position whose delta atom
/// can match ([`delta_can_match`]).
fn join_positive_leapfrog<D: TermDictionary, const N: usize>(
    plan: &RulePlan<N>,
    snapshot: JoinSnapshot<'_, D, N>,
    governor: &mut StepGovernor,
) -> Result<Vec<SlotSolution<D::Id, N>>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    let mut all: Vec<SlotSolution<D::Id, N>> = Vec::new();
    for delta_position in snapshot.delta_positions(plan.positive().len()) {
        if !delta_can_match(plan.operator_at(delta_position), snapshot)? {
            continue;
        }
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
                    )?
                }
                JoinGroup::Leapfrog(cycle) => extend_solutions_leapfrog(
                    &LeapfrogRun {
                        plan,
                        cycle,
                        scan: JoinScan::DeltaPosition(delta_position),
                        rel: snapshot.rel,
                        delta: snapshot.delta,
                    },
                    &partial,
                    governor,
                )?,
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
    snapshot.rel.interner().check_read()?;
    Ok(all)
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
    fn satisfied<D: TermDictionary>(
        &self,
        solution: &SlotSolution<D::Id, N>,
        rel: &RelationStore<D, N>,
        governor: &mut StepGovernor,
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
    fn visit_matches<D: TermDictionary, E>(
        &self,
        solution: &SlotSolution<D::Id, N>,
        rel: &RelationStore<D, N>,
        governor: &mut StepGovernor,
        mut visit: impl FnMut(RowId) -> Result<bool, E>,
        source_error: impl Fn(D::Error) -> E,
    ) -> Result<bool, E> {
        let rel = rel.read_source(self.source);
        rel.interner().check_read().map_err(&source_error)?;
        let result = (|| -> Result<bool, E> {
            if !governor.charge() {
                return Ok(false);
            }
            let mut values = [None; N];
            for (position, arg) in self.args.iter().enumerate() {
                match arg.probe(solution, rel).map_err(&source_error)? {
                    ProbeValue::Known(id) => values[position] = Some(id),
                    ProbeValue::Free => {}
                    // A ground position whose term is absent from the store matches nothing.
                    ProbeValue::Absent => return Ok(false),
                }
            }
            let bound = match (values[POSITION_SUBJECT], values[POSITION_OBJECT]) {
                (Some(subject), Some(object)) => Bound::Both(subject, object),
                (Some(subject), None) => Bound::Subject(subject),
                (None, Some(object)) => Bound::Object(object),
                (None, None) => Bound::Any,
            };
            for (index, partition) in rel.partitions_for(values).enumerate() {
                // The first probe was reserved above; broad predicate/graph scans also
                // reserve every further partition, including an empty selected index.
                if index != 0 && !governor.charge() {
                    return Ok(false);
                }
                let mut cursor = partition.select(bound);
                let mut first = true;
                while let Some((_, _, row)) = cursor.next() {
                    if !first && !governor.charge() {
                        return Ok(false);
                    }
                    first = false;
                    if visit(row)? {
                        return Ok(true);
                    }
                }
            }
            Ok(false)
        })()?;
        rel.interner().check_read().map_err(&source_error)?;
        Ok(result)
    }

    /// Retain signed witnesses on the native path; keep the ordinary boolean probe.
    fn blocks<D: TermDictionary>(
        &self,
        solution: &mut SlotSolution<D::Id, N>,
        rel: &RelationStore<D, N>,
        governor: &mut StepGovernor,
        evidence: &mut Option<&mut dyn evidence::RuleEvidence<D, N>>,
    ) -> Result<bool, EvalError<D::Error>> {
        let Some(owner) = evidence.as_deref_mut() else {
            return self
                .satisfied(solution, rel, governor)
                .map_err(EvalError::Source);
        };
        let mut witnesses = None;
        self.visit_matches(
            solution,
            rel,
            governor,
            |row| {
                let id = owner.source(row)?;
                witnesses = Some(match witnesses {
                    None => id,
                    Some(previous) => owner.combine(
                        previous,
                        id,
                        purrdf_core::evidence::EvidenceCombination::Alternatives,
                    )?,
                });
                Ok::<_, EvalError<D::Error>>(false)
            },
            EvalError::Source,
        )?;
        let (negative, blocked) = owner.negative(witnesses)?;
        solution.retain_negative(negative, owner)?;
        Ok(blocked)
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

/// Evidence accumulated by the original conjunction search, without replaying it.
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
    fn holds<D: TermDictionary>(
        &self,
        probe: GroupProbe<'_, D, N>,
        governor: &mut StepGovernor,
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
    fn blocks<D: TermDictionary>(
        &self,
        probe: GroupProbe<'_, D, N>,
        governor: &mut StepGovernor,
        evidence: &mut Option<&mut dyn evidence::RuleEvidence<D, N>>,
    ) -> Result<(bool, Option<purrdf_core::evidence::EvidenceId>), EvalError<D::Error>> {
        probe
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)?;
        let Some(owner) = evidence.as_deref_mut() else {
            return Ok((self.holds(probe, governor)?, None));
        };
        let mut trace = Some(GroupEvidence {
            owner,
            current: purrdf_core::evidence::EvidenceId::DEFAULT,
            witnesses: None,
        });
        if governor.charge() {
            let mut locals = vec![None; self.locals];
            self.match_atom(0, probe, &mut locals, governor, &mut trace)?;
        }
        let trace = trace.expect("the original search retains its evidence owner");
        probe
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)?;
        let (id, blocked) = trace.owner.negative(trace.witnesses)?;
        probe
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)?;
        Ok((blocked, Some(id)))
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
    fn match_atom<D: TermDictionary>(
        &self,
        k: usize,
        probe: GroupProbe<'_, D, N>,
        locals: &mut Vec<Option<LocalValue<D::Id>>>,
        governor: &mut StepGovernor,
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
    fn match_guard<D: TermDictionary>(
        &self,
        j: usize,
        probe: GroupProbe<'_, D, N>,
        locals: &mut Vec<Option<LocalValue<D::Id>>>,
        governor: &mut StepGovernor,
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
            let mut pinned = Vec::with_capacity(guard.inputs.len());
            for input in &guard.inputs {
                let surface = match input {
                    GroupArg::Outer(slot) => probe
                        .solution
                        .surface(*slot, probe.rel)
                        .map_err(EvalError::Source)?
                        .expect("an outer group variable is bound by the enclosing rule"),
                    GroupArg::Local(local) => match &locals[*local] {
                        Some(LocalValue::Interned(id)) => SlotValue::Interned(*id),
                        Some(LocalValue::Computed(surface)) => SlotValue::Computed(surface),
                        None => unreachable!(
                            "a group guard input is bound before it (DlClause scoping)"
                        ),
                    }
                    .surface(probe.rel)
                    .map_err(EvalError::Source)?
                    .expect("a local group input is bound before the guard"),
                    GroupArg::Const(surface) => SlotSurface::Computed(surface),
                };
                pinned.push(surface);
            }
            let inputs: Vec<&str> = pinned.iter().map(AsRef::as_ref).collect();
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
            for (&local, surface) in guard.outputs.iter().zip(&row) {
                locals[local] = Some(
                    match probe
                        .rel
                        .interner()
                        .lookup_term(surface)
                        .map_err(EvalError::Source)?
                    {
                        Some(id) => LocalValue::Interned(id),
                        None => LocalValue::Computed(surface.as_str().into()),
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
fn call_guard<D: TermDictionary, const N: usize>(
    guards: &dyn GuardEvaluator<D, N>,
    call: &GuardCall<'_, D, N>,
    governor: &mut StepGovernor,
) -> Result<Vec<Vec<String>>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    call.model
        .interner()
        .check_read()
        .map_err(EvalError::Source)?;
    let result = (|| {
        let refuse = |message: String| EvalError::Guard {
            rule: call.rule,
            site: call.site,
            guard: call.guard.name().to_owned(),
            message,
        };
        let rows = guards
            .evaluate(call)
            .map_err(|error| error.into_eval_error(refuse))?;
        let width = call.guard.outputs().len();
        for row in &rows {
            if !governor.charge() {
                return Ok(Vec::new());
            }
            if row.len() != width {
                return Err(refuse(format!(
                    "the evaluator answered a row of {} values for {width} outputs",
                    row.len()
                )));
            }
        }
        Ok(rows)
    })()?;
    call.model
        .interner()
        .check_read()
        .map_err(EvalError::Source)?;
    Ok(result)
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
    /// order. One atom under the stratified fixpoint; a conjunctive head under the ordered
    /// schedule ([`crate::schedule`]) asserts every conjunct from one solution.
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
        let value = match self {
            Self::Interned(id) => SlotValue::Interned(*id),
            Self::Fresh(surface) => SlotValue::Computed(surface.as_ref()),
        };
        Ok(value
            .surface(rel)?
            .expect("a head term is bound")
            .as_ref()
            .to_owned())
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
    /// `(proof_height, sum_source_height, sorted source facts, rule index, source facts)`,
    /// smallest wins. Every component is content-derived — proof heights, lexical fact
    /// surfaces and the authored rule position — so the winner is a function of the
    /// program and the data, never of which rule the scheduler happened to run first.
    ///
    /// The comparison resolves lexical surfaces, so it is deliberately staged: the cheap
    /// numeric prefix decides almost every real collision, and the allocating tail runs
    /// only on an exact numeric tie.
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
        Ok(order.is_lt())
    }
}

/// The canonical numeric prefix, resolving the lexical tail only on a tie.
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

/// The lexical tail of the canonical witness law, shared with projected frontiers.
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
    let mine: Vec<_> = mine
        .iter()
        .map(|source| source.fact(rel))
        .collect::<Result<_, _>>()?;
    let theirs: Vec<_> = theirs
        .iter()
        .map(|source| source.fact(rel))
        .collect::<Result<_, _>>()?;
    rel.interner().check_read()?;
    Ok(fact_preference(&mine, my_rule, &theirs, their_rule))
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
/// across schedule groups. Owned facts survive the layer's retraction/rebuild.
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
                Ok::<_, core::convert::Infallible>(fact_preference(
                    &other.derivation.sources,
                    other.derivation.rule,
                    &self.derivation.sources,
                    self.derivation.rule,
                ))
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
    depth: &[u32],
) -> (u32, u64)
where
    Arity<N>: StoreShape<N>,
{
    sources.iter().fold((0, 0u64), |(maximum, sum), source| {
        let height = match source.source {
            crate::clause::ReadSource::Evaluation => depth[source.row.resident_index()],
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
    pub(crate) depth: &'a [u32],
    /// Per-row ASSUMED flags, indexed by [`RowId`], or empty when nothing is assumed.
    ///
    /// An assumed row is a fact the caller asserted for the duration of a schedule layer
    /// ([`crate::schedule::LayerHooks`]) and will retract at its end unless a rule derives
    /// it too; a rule that re-derives one is recorded in [`RoundBuffer::confirmed`].
    pub(crate) assumed: &'a [bool],
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
fn evaluate_rule<'r, D: TermDictionary, const N: usize>(
    entry: RuleEntry<'r, N>,
    snapshot: RoundSnapshot<'_, D, N>,
    strategy: JoinStrategy,
    governor: StepGovernor,
    guards: &dyn GuardEvaluator<D, N>,
) -> Result<RoundBuffer<'r, D::Id, N>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    evaluate_rule_with_evidence(entry, snapshot, strategy, governor, guards, None)
}

fn evaluate_rule_with_evidence<'r, D: TermDictionary, const N: usize>(
    entry: RuleEntry<'r, N>,
    snapshot: RoundSnapshot<'_, D, N>,
    strategy: JoinStrategy,
    mut governor: StepGovernor,
    guards: &dyn GuardEvaluator<D, N>,
    mut evidence: Option<&mut dyn evidence::RuleEvidence<D, N>>,
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
        if runtime.factors.len() > 1 && matches!(strategy, JoinStrategy::Planned) {
            return factors::evaluate_with_evidence(entry, snapshot, &mut governor, evidence);
        }
        let mut solutions = join_body(
            plan,
            JoinSnapshot {
                rel,
                delta: entry.delta,
            },
            strategy,
            &mut governor,
        )
        .map_err(EvalError::Source)?;

        for (position, guard) in runtime.guards.iter().enumerate() {
            let declared = &entry.rule.guards()[position];
            let mut extended = Vec::with_capacity(solutions.len());
            for solution in solutions {
                if governor.spent() {
                    break;
                }
                let rows = {
                    let pinned = guard
                        .inputs
                        .iter()
                        .map(|&slot| {
                            Ok(solution.surface(slot, rel)?.expect(
                                "a guard input is bound before the guard (DlClause scoping)",
                            ))
                        })
                        .collect::<Result<Vec<_>, D::Error>>()
                        .map_err(EvalError::Source)?;
                    let inputs: Vec<&str> = pinned.iter().map(AsRef::as_ref).collect();
                    call_guard(
                        guards,
                        &GuardCall {
                            rule: entry.index,
                            site: GuardSite::Body(position),
                            guard: declared,
                            inputs: &inputs,
                            model: rel,
                        },
                        &mut governor,
                    )?
                };
                for row in rows {
                    let mut next = solution.clone();
                    for (&slot, surface) in guard.outputs.iter().zip(&row) {
                        next.bind_surface(slot, surface, rel)
                            .map_err(EvalError::Source)?;
                    }
                    extended.push(next);
                }
            }
            solutions = extended;
        }

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
    if let Some(row) = present_row(&key, rel).map_err(EvalError::Source)? {
        if snapshot
            .assumed
            .get(row.resident_index())
            .copied()
            .unwrap_or(false)
        {
            let confirmation = Confirmation {
                derivation: Derivation {
                    fact: key.fact(rel).map_err(EvalError::Source)?,
                    rule,
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

/// Evaluate every scheduled rule of one round, erasing scheduling order.
///
/// A round holding a guarded rule runs on the calling thread in program order: a guard is
/// caller code, and the caller's thread-scoped evaluation context has to reach it. A
/// guard-free round keeps the rule-parallel path, which cannot reach a guard at all.
pub(crate) fn evaluate_round<'r, D: TermDictionary, const N: usize>(
    entries: &[RuleEntry<'r, N>],
    snapshot: RoundSnapshot<'_, D, N>,
    execution: RoundExecution,
    strategy: JoinStrategy,
    allowance: u64,
    guards: &dyn GuardEvaluator<D, N>,
) -> Result<RoundBuffer<'r, D::Id, N>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    evaluate_round_with_evidence(
        entries, snapshot, execution, strategy, allowance, guards, None,
    )
}

fn evaluate_round_with_evidence<'r, D: TermDictionary, const N: usize>(
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
            for &entry in entries {
                let buffer = match evidence.as_mut() {
                    Some(owner) => evaluate_rule_with_evidence(
                        entry,
                        snapshot,
                        strategy,
                        pool.task(),
                        guards,
                        Some(&mut **owner),
                    )?,
                    None => evaluate_rule(entry, snapshot, strategy, pool.task(), guards)?,
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
            .map(|&entry| evaluate_rule(entry, snapshot, strategy, pool.task(), &NoGuards))
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
    /// Candidate solutions enumerated so far.
    pub(crate) join_steps: u64,
    /// A refused reservation, including an observation past u64::MAX.
    join_refused: bool,
    /// Term-generating rounds committed so far.
    pub(crate) term_generating_rounds: u64,
    /// The limits in force.
    limits: Limits,
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
        let depth = vec![0u32; edb.row_count()];
        let input_terms = edb.interner().term_count();
        Self {
            rel: edb,
            depth,
            derivations: Vec::new(),
            join_steps: 0,
            join_refused: false,
            term_generating_rounds: 0,
            limits: Limits::of(&options, input_terms),
            input_terms,
            generating_rules: BTreeSet::new(),
        }
    }

    /// The budget consumption observed so far.
    pub(crate) fn report(&self) -> BudgetReport {
        BudgetReport {
            join_steps: self.join_steps,
            stored_facts: self.rel.retained_rows(),
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

    /// The report that would describe the state if it also held `extra` more facts.
    fn projected_report(&self, extra: usize) -> BudgetReport {
        BudgetReport {
            stored_facts: self.rel.retained_rows().saturating_add(extra),
            ..self.report()
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
    /// [`EvalError::BudgetExhausted`] when the round passed a ceiling — decided before a
    /// single surface is materialised where the fact count alone proves it.
    pub(crate) fn absorb(
        &mut self,
        round: RoundBuffer<'_, D::Id, N>,
    ) -> Result<(), EvalError<D::Error>> {
        self.join_steps = self.join_steps.saturating_add(round.join_steps);
        self.join_refused |= round.join_refused;
        check_budget(self)?;
        if round.entries.is_empty() {
            return Ok(());
        }
        // Every entry was gated on absence from the store when it was created, and the
        // entries are unique by head fact, so this projection is exact rather than an
        // over-estimate: the limit is decided before a single surface is materialised.
        let projected = self.rel.retained_rows().saturating_add(round.entries.len());
        if u64::try_from(projected).unwrap_or(u64::MAX) > self.limits.stored_facts {
            return Err(EvalError::BudgetExhausted {
                resource: BudgetResource::StoredFacts,
                report: self.projected_report(round.entries.len()),
            });
        }
        if round.generates_terms {
            self.term_generating_rounds = self.term_generating_rounds.saturating_add(1);
            self.generating_rules.clone_from(&round.generating_rules);
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
/// [`EvalError::BudgetExhausted`] if the run passes the default stored-fact or join-step
/// limit ([`EvalOptions::default`]) or the term-arena ceiling. There is no partial answer: a budget refusal is total. [`EvalError::Guard`] if the program
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
        RoundExecution::Parallel,
        JoinStrategy::Planned,
    )
}

/// The policy-selectable implementation behind [`evaluate`].
///
/// The policies are private: a caller may neither weaken production scheduling nor
/// override the planner's join choice. The tests use them to prove the alternatives agree.
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
    if rows.row_count() != edb.row_count() {
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
            RoundExecution::Parallel,
            JoinStrategy::Planned,
        )?;
        for (row, _) in evaluation.facts.rows_from(rows.row_count()) {
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
            execution: RoundExecution::Parallel,
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
    let runtimes: Vec<RuleRuntime<N>> = (0..exe.rule_count())
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
        let owner = evidence
            .as_mut()
            .map(|owner| &mut **owner as &mut dyn evidence::RuleEvidence<D, N>);
        run_stratum(exe, &runtimes, stratum, &mut state, run, owner)?;
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
            facts: self.rel,
            derivations: self.derivations,
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
    if report.term_arena_bytes > MAX_TERM_ARENA_BYTES {
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
                rel: &state.rel,
                depth: &state.depth,
                assumed: &[],
            },
            run.execution,
            run.strategy,
            state.allowance(),
            run.guards,
            owner,
        )?;
        if round.is_empty() {
            state.join_steps = state.join_steps.saturating_add(round.join_steps);
            state.join_refused |= round.join_refused;
            check_budget(state)?;
            if let Some(owner) = evidence.as_mut()
                && owner.commit(&state.rel)?
            {
                delta = Delta::all(state.rel.row_count());
                continue;
            }
            return Ok(()); // fact and evidence fixpoint
        }

        let round_lo = state.rel.row_count();
        state.absorb(round)?;
        if let Some(owner) = evidence.as_mut()
            && owner.commit(&state.rel)?
        {
            delta = Delta::all(state.rel.row_count());
            continue;
        }

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
fn commit_round<D: TermDictionary, const N: usize>(
    round: RoundBuffer<'_, D::Id, N>,
    state: &mut FixpointState<D, N>,
) -> Result<(), EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    let mut winners = round
        .entries
        .into_iter()
        .map(|(key, candidate)| Ok::<_, D::Error>((key.fact(&state.rel)?, key, candidate)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(EvalError::Source)?;
    // Head facts are unique in a round buffer, so the lexical order is total.
    winners.sort_by(|(left, _, _), (right, _, _)| left.cmp(right));

    for (fact, key, candidate) in winners {
        let sources = candidate
            .source_facts(&state.rel)
            .map_err(EvalError::Source)?;
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
        state.derivations.push(Derivation {
            fact,
            rule: candidate.rule,
            sources,
            proof_height: candidate.proof_height,
        });
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
                rel: &store,
                delta: Delta::all(2),
            },
            JoinStrategy::Planned,
            &mut StepGovernor::new(u64::MAX),
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
                    rel: &store,
                    depth: &[0, 0],
                    assumed: &[],
                },
                &mut Some(&mut capture),
                HeadEvidence {
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
            ) -> Result<Vec<Vec<String>>, CallbackError<SourceLost>> {
                self.calls.fetch_add(1, Ordering::Relaxed);
                if call.guard.name() == "produce" {
                    assert_eq!(call.inputs, ["triple", "member"]);
                    assert_eq!(call.model.interner().pins.load(Ordering::Relaxed), 2);
                    if self.lose_source {
                        call.model.interner().lost.store(true, Ordering::Relaxed);
                    }
                    Ok(vec![vec!["fresh".into()]])
                } else {
                    assert_eq!(call.inputs, ["fresh"]);
                    assert_eq!(call.model.interner().pins.load(Ordering::Relaxed), 0);
                    Ok(vec![Vec::new()])
                }
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
                    rel: &store,
                    delta: Delta::all(3),
                },
                JoinStrategy::Planned,
                &mut StepGovernor::new(u64::MAX),
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
            binding.bind_surface(0, "triple", &store).unwrap();
            assert_eq!(binding.get(0), Some(TRIPLE));
            let before = binding.clone();
            store
                .interner()
                .lose_after_lookup
                .store(true, Ordering::Relaxed);
            assert_eq!(
                binding.bind_surface(0, "unknown", &store),
                Err(SourceLost(99))
            );
            assert_eq!(
                binding, before,
                "a refused lexical lookup never publishes a frame mutation"
            );
        }
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
            ) -> Result<Vec<Vec<String>>, CallbackError<SourceLost>> {
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
            let snapshot = JoinSnapshot { rel: &store, delta };
            let mut planned_governor = StepGovernor::new(u64::MAX);
            let planned =
                join_body(plan, snapshot, JoinStrategy::Planned, &mut planned_governor).unwrap();
            let binary = join_body(
                plan,
                snapshot,
                JoinStrategy::ForcedBinary,
                &mut StepGovernor::new(u64::MAX),
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
                    rel: &store,
                    delta: Delta::all(0)
                },
                JoinStrategy::Planned,
                &mut StepGovernor::new(0)
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
            },
            &[SlotSolution::empty(plan.variables().len())],
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
        assert_ne!(
            cause,
            EvalError::Storage(purrdf_lex::allocation::StorageError::AllocationFailed)
        );
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
                Delta::all(store.row_count()),
                Scan::Full,
                &partial,
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
            Scan::Full,
            &[],
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
            Delta::all(store.row_count()),
            Scan::Full,
            &[SlotSolution::empty(plan.variables().len())],
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
        evaluate(&exe, edb).expect("the fixture program stays inside every ceiling")
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
            &EvalOptions::default(),
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
            result.evaluation.derivations.len(),
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
            &EvalOptions::default(),
            &NoGuards,
            None,
            rows,
            evidence_scope(),
        )
        .unwrap();
        assert_eq!(result.evaluation.derivations.len(), 1);
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
                                    &EvalOptions::default(),
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
                &EvalOptions::default(),
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
                            &EvalOptions::default(),
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
            &EvalOptions::default(),
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
            &EvalOptions::default(),
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
            )
            .unwrap();
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
    /// deliberately constants: attaching one is observationally nothing until it fires.
    /// Asserted over the whole synthetic corpus rather than over one program, and against
    /// the ungoverned [`evaluate`] rather than against itself.
    #[test]
    fn an_unfired_stop_signal_changes_no_answer() {
        let never = StopsAfter {
            remaining: AtomicU64::new(u64::MAX),
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
                    EvalOptions::default(),
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
                    evaluation.derivations(),
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
                    &EvalOptions::default().with_max_join_steps(expected),
                    None,
                )
                .expect("exact credits");
                assert_eq!(exact.budget().join_steps(), expected);
                assert_eq!(exact.facts().row_count(), 2 * expected as usize);
                let answer = (
                    exact.facts().facts_sorted(),
                    exact.derivations().to_vec(),
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
                        &EvalOptions::default().with_max_join_steps(ceiling),
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
        let mut state =
            FixpointState::seeded(edb, EvalOptions::default().with_max_join_steps(u64::MAX));
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
                rel: &state.rel,
                depth: &state.depth,
                assumed: &[],
            },
            RoundExecution::Parallel,
            JoinStrategy::Planned,
            state.allowance(),
            &NoGuards,
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
            &EvalOptions::default().with_max_join_steps(128),
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
                &EvalOptions::default().with_max_join_steps(ceiling),
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
                &EvalOptions::default().with_max_join_steps(credits),
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
                    &EvalOptions::default().with_max_join_steps(ceiling),
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
            ) -> Result<Vec<Vec<String>>, crate::guard::CallbackError> {
                self.calls.fetch_add(1, Ordering::Relaxed);
                Ok(if !self.keep {
                    Vec::new()
                } else {
                    (0..self.count)
                        .map(|row| {
                            call.guard
                                .outputs()
                                .iter()
                                .map(|_| surface(&format!("https://example.org/fresh{row}")))
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
                &EvalOptions::default().with_max_join_steps(limit),
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
            &EvalOptions::default().with_max_join_steps(10001),
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
                &EvalOptions::default().with_max_join_steps(10000),
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
            &EvalOptions::default().with_max_join_steps(17),
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
                &EvalOptions::default().with_max_join_steps(limit),
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
            &EvalOptions::default().with_max_join_steps(203),
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
                &EvalOptions::default().with_max_join_steps(202),
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
        let n = 1100usize; // over a million candidate solutions > the wasm32 default
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
        let limited = EvalOptions::default().with_max_join_steps(WASM_DEFAULT_MAX_JOIN_STEPS);
        let error = evaluate_guarded(&exe, edb.clone(), &NoGuards, &limited, None)
            .expect_err("the join-step limit must be passed");
        let EvalError::BudgetExhausted { resource, report } = error.clone() else {
            panic!("expected a budget refusal, got {error:?}");
        };
        assert_eq!(resource, BudgetResource::JoinSteps);
        assert_eq!(
            report.join_steps(),
            WASM_DEFAULT_MAX_JOIN_STEPS + 1,
            "one task, so the report is exactly the observation that passed the limit"
        );
        assert_eq!(report.join_step_limit(), WASM_DEFAULT_MAX_JOIN_STEPS);
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
        let admitted = EvalOptions::default().with_max_join_steps(JOIN_STEPS_OBSERVED);
        let model = evaluate_guarded(&exe, edb.clone(), &NoGuards, &admitted, None)
            .expect("a limit the run fits under admits it");
        assert_eq!(model.facts().row_count(), n + 1);
        assert_eq!(model.budget().join_steps(), JOIN_STEPS_OBSERVED);
        let short = EvalOptions::default().with_max_join_steps(JOIN_STEPS_OBSERVED - 1);
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
        let n = 400usize; // 160_000 derived facts > the wasm32 default
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
        let limited = EvalOptions::default().with_max_stored_facts(WASM_DEFAULT_MAX_STORED_FACTS);
        let error = evaluate_guarded(&exe, edb.clone(), &NoGuards, &limited, None)
            .expect_err("the stored-fact limit must be passed");
        let EvalError::BudgetExhausted { resource, report } = error.clone() else {
            panic!("expected a budget refusal, got {error:?}");
        };
        assert_eq!(resource, BudgetResource::StoredFacts);
        assert_eq!(
            report.stored_facts(),
            n + n * n,
            "the report is the count the round would have produced"
        );
        assert_eq!(report.stored_fact_limit(), WASM_DEFAULT_MAX_STORED_FACTS);
        assert!(report.join_steps() <= report.join_step_limit());
        let rendered = error.to_string();
        assert!(
            rendered.contains("160400 facts observed, 131072 permitted (the caller's limit)")
                && rendered.ends_with("raise it with EvalOptions::with_max_stored_facts"),
            "{rendered}"
        );

        // The valid neighbour: a limit of exactly the least model's size admits it, and
        // one fact fewer does not.
        let exact = EvalOptions::default().with_max_stored_facts((n + n * n) as u64);
        let model = evaluate_guarded(&exe, edb.clone(), &NoGuards, &exact, None)
            .expect("a limit of exactly the model's size admits it");
        assert_eq!(model.facts().row_count(), n + n * n);
        let short = EvalOptions::default().with_max_stored_facts((n + n * n - 1) as u64);
        assert!(matches!(
            evaluate_guarded(&exe, edb, &NoGuards, &short, None),
            Err(EvalError::BudgetExhausted {
                resource: BudgetResource::StoredFacts,
                ..
            })
        ));
    }

    /// A store past the NATIVE default is refused under the default, naming the limit, the
    /// numbers and the knob, and the same store completes when the limit is raised to
    /// exactly what the run holds. The grid is 2,049 × 2,048 rows over one partition, so
    /// the interner stays small while the row count passes 4,194,304.
    #[test]
    fn a_store_past_the_native_default_is_refused_and_completes_when_raised() {
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
        assert!(seeded as u64 > NATIVE_DEFAULT_MAX_STORED_FACTS);
        let exe = compile(rules).expect("the fixture compiles");
        let native = EvalOptions::default().with_max_stored_facts(NATIVE_DEFAULT_MAX_STORED_FACTS);
        let error = evaluate_guarded(&exe, edb.clone(), &NoGuards, &native, None)
            .expect_err("the native default is passed by the seeded store");
        assert_eq!(
            error.to_string(),
            "evaluation exceeded the stored-fact limit: 4196353 facts observed, 4194304 \
             permitted (the caller's limit); raise it with EvalOptions::with_max_stored_facts"
        );
        if !cfg!(target_arch = "wasm32") {
            assert_eq!(
                evaluate(&exe, edb.clone())
                    .expect_err("the native default refuses it unstated too")
                    .to_string(),
                "evaluation exceeded the stored-fact limit: 4196353 facts observed, 4194304 \
                 permitted (the default for this target); raise it with \
                 EvalOptions::with_max_stored_facts"
            );
        }
        let raised = EvalOptions::default().with_max_stored_facts(seeded as u64 + 1);
        let model = evaluate_guarded(&exe, edb, &NoGuards, &raised, None)
            .expect("raised to exactly the model's size");
        assert_eq!(model.facts().row_count(), seeded + 1);
    }

    /// Each target's defaults are the values its documentation states: the `wasm32`
    /// values are the ceilings every target had before the limits became parameters, the
    /// native stored-fact limit admits a least model thirty-two times larger, and the
    /// join-step limit is the same everywhere. A native build runs the native defaults.
    #[test]
    fn the_default_limits_are_sized_per_target() {
        assert_eq!(WASM_DEFAULT_MAX_STORED_FACTS, 131_072);
        assert_eq!(WASM_DEFAULT_MAX_JOIN_STEPS, 1_048_576);
        assert_eq!(NATIVE_DEFAULT_MAX_STORED_FACTS, 4_194_304);
        assert_eq!(NATIVE_DEFAULT_MAX_JOIN_STEPS, 1_048_576);
        if cfg!(target_arch = "wasm32") {
            assert_eq!(DEFAULT_MAX_STORED_FACTS, WASM_DEFAULT_MAX_STORED_FACTS);
            assert_eq!(DEFAULT_MAX_JOIN_STEPS, WASM_DEFAULT_MAX_JOIN_STEPS);
        } else {
            assert_eq!(DEFAULT_MAX_STORED_FACTS, NATIVE_DEFAULT_MAX_STORED_FACTS);
            assert_eq!(DEFAULT_MAX_JOIN_STEPS, NATIVE_DEFAULT_MAX_JOIN_STEPS);
        }
        let options = EvalOptions::default();
        assert_eq!(options.max_stored_facts(), DEFAULT_MAX_STORED_FACTS);
        assert_eq!(options.max_join_steps(), DEFAULT_MAX_JOIN_STEPS);
        assert_eq!(options.stated_max_stored_facts(), None);
        assert_eq!(options.stated_max_join_steps(), None);
    }

    /// Under the default limits the store past the `wasm32` default completes natively —
    /// 160,400 facts — and its report says which limits governed it.
    #[test]
    fn a_default_native_run_admits_a_model_past_the_wasm32_default() {
        let src = "https://example.org/src";
        let pair = "https://example.org/pair";
        let rules = vec![DlClause::datalog(
            atom("?x", pair, "?y"),
            vec![atom("?x", src, "?c"), atom("?y", src, "?c")],
        )];
        let n = 400usize;
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
        let outcome = evaluate(&exe, edb);
        if cfg!(target_arch = "wasm32") {
            assert!(outcome.is_err(), "the wasm32 default refuses 160,400 facts");
        } else {
            let model = outcome.expect("the native default admits 160,400 facts");
            assert_eq!(model.facts().row_count(), n + n * n);
            assert_eq!(
                model.budget().stored_fact_limit(),
                NATIVE_DEFAULT_MAX_STORED_FACTS
            );
            assert!(!model.budget().stored_fact_limit_stated());
            assert_eq!(
                model.budget().join_step_limit(),
                NATIVE_DEFAULT_MAX_JOIN_STEPS
            );
        }
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
        assert!((edb.row_count() as u64) < DEFAULT_MAX_STORED_FACTS);
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
        assert!(first.budget().join_steps() <= first.budget().join_step_limit());
    }

    /// The budget refusal is a `std::error::Error` with a message naming the resource and
    /// both numbers — a diagnostic, not an opaque marker.
    #[test]
    fn budget_and_stratification_errors_render() {
        let error: EvalError = EvalError::BudgetExhausted {
            resource: BudgetResource::JoinSteps,
            report: BudgetReport {
                join_steps: DEFAULT_MAX_JOIN_STEPS + 1,
                stored_facts: 3,
                term_arena_bytes: 4,
                ..BudgetReport::default()
            },
        };
        let rendered = error.to_string();
        assert!(rendered.contains("join-step limit"), "{rendered}");
        assert!(
            rendered.contains(&format!(
                "{} join steps observed, {DEFAULT_MAX_JOIN_STEPS} permitted (the default for \
                 this target)",
                DEFAULT_MAX_JOIN_STEPS + 1
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
        let positions = |delta| JoinSnapshot { rel: &rel, delta }.delta_positions(3);
        assert_eq!(positions(Delta::all(3)), 2..3);
        // Later rounds keep the first and middle anchors, whose OldOnly suffixes
        // can match older facts. A prefix delta is not the whole frozen store.
        assert_eq!(positions(Delta { lo: 1, hi: 3 }), 0..3);
        assert_eq!(positions(Delta { lo: 0, hi: 2 }), 0..3);
        assert_eq!(positions(Delta { lo: 2, hi: 3 }), 0..3);
        assert_eq!(
            JoinSnapshot {
                rel: &rel,
                delta: Delta::all(3)
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
