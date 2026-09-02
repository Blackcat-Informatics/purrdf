// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Host-injected **property functions** — relations invoked from predicate position.
//!
//! A predicate IRI under a caller-configured
//! [`ParserOptions::property_fn_namespaces`](purrdf_sparql_algebra::ParserOptions)
//! (prefix match), or one that exactly matches an entry of
//! [`ParserOptions::property_fn_iris`](purrdf_sparql_algebra::ParserOptions) — the
//! set [`NativeSparqlEngine`](crate::NativeSparqlEngine) derives one-to-one from
//! this registry's keys, so a registered relation is reachable without also
//! reclassifying every other IRI under its namespace — is lowered by the parser to
//! [`purrdf_sparql_algebra::GraphPattern::PropertyFunction`]
//! instead of an ordinary triple pattern, and resolved at evaluation time against a
//! caller-injected [`PropertyFunctionRegistry`]. Where [`crate::user_fn`] injects
//! *functions* — one value per call, in an expression position — this module injects
//! *relations*: a call is a row source in a graph-pattern position, and it may emit
//! zero, one, or many rows per invocation.
//!
//! # Why a relation is not a function with extra steps
//!
//! Three properties follow from being a row source rather than a value, and each one
//! shows up as a distinct piece of this module's contract:
//!
//! * **Access patterns.** A relation is rarely computable in every direction. A
//!   `split(?whole, ?part)` relation can enumerate parts from a whole but not wholes
//!   from a part. So a relation *declares* the argument-binding patterns it can serve
//!   ([`PropertyFunction::modes`]), and an invocation is admitted only when one of
//!   them is general enough to cover it ([`PropertyFunction::admits`]).
//! * **Cardinality.** A call can be an unbounded generator, so the planner needs a
//!   declared upper bound per access pattern ([`PropertyFunction::rows_per_invocation`])
//!   to order calls and to admit them against a ceiling.
//! * **Emission order.** A bag of rows has an order, and SPARQL results are
//!   reproducible only if that order is. So the order a relation emits in is part of
//!   its contract, and the engine preserves it.
//!
//! # The trust boundary
//!
//! A relation is arbitrary host Rust. Every invariant the evaluator needs from it is
//! therefore either **checked before host code runs** (arity, in
//! [`open_contained`]), **contained when host code misbehaves** (a panic, in
//! [`open_contained`]/[`next_contained`] for `open`/`next`, and in
//! [`declaration_contained`] for `arity`/`modes`/`volatility`/`rows_per_invocation` —
//! every one of the four is host code exactly as `open`/`next` is, so nothing reads one
//! directly), or **applied to what host code returns** (equality on bound positions, and
//! the row-width check, in the dispatch). A relation that ignores its own declarations
//! cannot make the engine unsound; it can only make it slow, or make its own call fail.
//!
//! The row ceiling [`PropertyFunction::open`] receives is the one place that asks a
//! relation for cooperation rather than checking it, because "I stopped early" and "I
//! am exhausted" are the same empty cursor and no engine-side measure can tell them
//! apart. The obligation is kept as small as the seam allows: the ceiling counts rows
//! the relation emits that agree with the bound values it was handed — everything the
//! engine filters on that the relation cannot see makes the engine withhold the ceiling
//! instead (see [`PropertyFunction`]'s ceiling contract).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use purrdf_core::binding_pattern::BindingPattern;
use purrdf_core::{DatasetView, GraphMatch, TermValue};

use crate::DetHashMap;
use crate::error::EvalError;
use crate::user_fn::Volatility;

// ---------------------------------------------------------------------------
// Positional shape
// ---------------------------------------------------------------------------

/// A relation's declared positional arity: how many arguments it takes on the
/// subject side of the predicate and how many on the object side.
///
/// Checked against the call site **before any host code runs** — the same fail-fast
/// doctrine [`crate::user_fn::Arity`] applies to a native function, and for the same
/// reason: a relation must never be handed a short or long argument vector.
///
/// # The flattening order
///
/// Everything downstream of arity — [`PropertyFunction::modes`], [`PfArgs`], [`PfRow`]
/// — indexes **flattened** positions: the subject-side arguments first, in written
/// order, then the object-side arguments, in written order. Flattened position `p` is
/// subject argument `p` when `p < subject`, and object argument `p - subject`
/// otherwise. One numbering, used by every surface, so a mode and a row cannot
/// disagree about which position they are talking about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PfArity {
    /// The number of subject-side arguments.
    pub subject: usize,
    /// The number of object-side arguments.
    pub object: usize,
}

impl PfArity {
    /// A declared arity of `subject` subject-side and `object` object-side arguments.
    #[must_use]
    pub const fn new(subject: usize, object: usize) -> Self {
        Self { subject, object }
    }

    /// The total number of flattened positions (`subject + object`).
    #[must_use]
    pub const fn total(self) -> usize {
        self.subject + self.object
    }

    /// The all-free [`BindingPattern`] over this arity — the ⊥ of the lattice, the
    /// most general mode, and the one a relation declares when it can serve every
    /// access pattern (see [`PropertyFunction::modes`]).
    #[must_use]
    pub fn all_free_mode(self) -> BindingPattern {
        BindingPattern::from_bound_positions(self.total(), [])
    }
}

impl core::fmt::Display for PfArity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} subject / {} object", self.subject, self.object)
    }
}

/// The already-evaluated argument values of one invocation, per position.
///
/// A `Some` cell is a **bound** input — a constant written at the call site, or a
/// variable the incoming solution row binds. A `None` cell is **free**: the relation
/// is being asked to produce a value there. The two slices are the subject side and
/// the object side; together they are the flattened positions
/// [`PfArity`] documents.
///
/// The values are lent as borrows for the duration of the call, exactly as
/// [`crate::user_fn::NativeFnBody`] lends its arguments: they already live in the
/// evaluator's per-invocation buffer, and deep-cloning heap-string-owning
/// [`TermValue`]s on a per-row invocation path would be pure overhead. A relation
/// that must retain a value past [`PropertyFunction::open`] clones it itself.
#[derive(Debug, Clone, Copy)]
pub struct PfArgs<'a> {
    subject: &'a [Option<&'a TermValue>],
    object: &'a [Option<&'a TermValue>],
}

impl<'a> PfArgs<'a> {
    /// Build the argument view of one invocation from its two sides.
    #[must_use]
    pub const fn new(
        subject: &'a [Option<&'a TermValue>],
        object: &'a [Option<&'a TermValue>],
    ) -> Self {
        Self { subject, object }
    }

    /// The subject-side arguments, in written order.
    #[must_use]
    pub const fn subject(&self) -> &'a [Option<&'a TermValue>] {
        self.subject
    }

    /// The object-side arguments, in written order.
    #[must_use]
    pub const fn object(&self) -> &'a [Option<&'a TermValue>] {
        self.object
    }

    /// The invocation's positional shape, as counted from the argument vectors
    /// themselves (what the call site actually supplied, which
    /// [`open_contained`] checks against what the relation declared).
    #[must_use]
    pub const fn arity(&self) -> PfArity {
        PfArity::new(self.subject.len(), self.object.len())
    }

    /// Every position's value in flattened order (subject side, then object side).
    pub fn flattened(&self) -> impl Iterator<Item = Option<&'a TermValue>> + '_ {
        self.subject.iter().chain(self.object.iter()).copied()
    }

    /// The value at flattened position `pos`, or `None` when that position is free
    /// **or** out of range.
    #[must_use]
    pub fn get(&self, pos: usize) -> Option<&'a TermValue> {
        if pos < self.subject.len() {
            self.subject[pos]
        } else {
            self.object.get(pos - self.subject.len()).copied().flatten()
        }
    }

    /// The invocation's own access pattern: bit `p` set iff flattened position `p` is
    /// bound. This is the pattern a declared mode must
    /// [`subsume`](BindingPattern::subsumes) for the invocation to be feasible.
    #[must_use]
    pub fn mode(&self) -> BindingPattern {
        BindingPattern::from_bools(self.flattened().map(|value| value.is_some()))
    }
}

/// One row produced by an invocation: a value for **every** flattened position, in
/// the order [`PfArity`] documents — free positions and bound positions alike.
///
/// A bound position may be echoed back (the usual, and cheapest, thing for a relation
/// to do) or filled with any candidate the relation likes, because **the engine
/// applies equality filtering on bound positions**: a row whose value at a bound
/// position differs from the input value there is dropped before it becomes a
/// solution. A relation is therefore free to emit candidates and let the engine
/// filter, and echoing the input is simply the case where nothing is ever dropped.
pub type PfRow = Vec<TermValue>;

/// The row stream of one invocation, drained by the engine.
///
/// A cursor is opened, drained, and dropped inside a single invocation, so it never
/// crosses a thread boundary and carries no `Send` bound; the *relation* is shared
/// and therefore `Send + Sync` (see [`PropertyFunction`]).
///
/// # Emission order is the relation's contract
///
/// `next` MUST return rows in an order that is a pure function of the invocation's
/// arguments — never of iteration over a randomly-seeded map, of wall-clock time, or
/// of thread scheduling. The engine preserves that order and never re-sorts it, so it
/// reaches the query's answer verbatim: a relation with an unstable order makes the
/// query's result unstable, which no engine-side measure can repair.
///
/// # The deaf-relation doctrine
///
/// The evaluator polls its stop signal before opening a cursor and between successive
/// `next` calls, and charges the work a row costs **before** that row is consumed. A
/// relation that blocks forever inside one `next` call therefore degrades the
/// stop-check granularity to one call — it can make a stop *late*, never a partial
/// answer unsound, and never an answer wrong.
pub trait PfCursor {
    /// The next row, or `None` when the invocation is exhausted.
    ///
    /// # Errors
    ///
    /// Any [`EvalError`] the relation raises. Per the hard-fail doctrine this aborts
    /// the query rather than silently truncating the row stream — a short stream
    /// offered as complete is exactly the wrong answer the doctrine forbids.
    fn next(&mut self) -> Result<Option<PfRow>, EvalError>;

    /// The **internal work** this cursor has performed since this method last returned,
    /// *taken* — the count resets to zero, so consecutive calls partition the work rather
    /// than re-report it.
    ///
    /// This is the seam's answer to a question the engine cannot answer for itself: what
    /// did that call actually cost? The two quantities the engine can see — invocations
    /// driven and rows accepted — describe the *answer*, and for a generator relation the
    /// answer is not where the work is. A nearest-neighbour search that examines a million
    /// vectors to return the five closest emits five rows, and priced by rows it is
    /// indistinguishable from a five-row table. Reporting the million is what makes a
    /// caller's budget a bound on the execution rather than on the result set.
    ///
    /// The engine charges [`ChargePoint::PropertyFunctionWork`](crate::governor::ChargePoint::PropertyFunctionWork)
    /// once per reported unit, after every [`Self::next`] — including the terminating call
    /// that returns `None`, so a cursor that searches lazily on its first pull and one
    /// that searched eagerly in [`PropertyFunction::open`] are charged the same total. A
    /// cursor that reports work it has not yet done is not wrong, merely early.
    ///
    /// # What a unit is
    ///
    /// Whatever the implementing relation's own documentation says it is: one candidate
    /// examined, one posting decoded, one row of an external table read. The engine cannot
    /// define the unit for host code and does not try; it prices each reported unit at one
    /// and requires the relation to say what it counted. A relation whose work is
    /// genuinely proportional to the rows it emits has nothing to add here and keeps the
    /// default.
    ///
    /// # Why over-reporting is not a hazard, and under-reporting is not a hole
    ///
    /// The count is *spent*, not merely recorded, so a relation that inflates it exhausts
    /// its own caller's budget — an incentive pointing the right way. A relation that
    /// under-reports (or, by default, reports nothing) makes its query cheaper than it
    /// should be, but every other ceiling stays in force unchanged: the invocation point,
    /// the row point, the intermediate-cell peak, the answer cap and the wall deadline all
    /// bound it exactly as they did before this method existed. Under-reporting can
    /// therefore cost a caller precision in a receipt; it can never cost them soundness,
    /// and no engine-side measure can see inside host code to do better.
    ///
    /// # Default
    ///
    /// Zero. Every relation written before this method existed reports no work and charges
    /// nothing, which is what makes a budget sized against the previous profile version
    /// buy the same execution.
    fn take_work(&mut self) -> u64 {
        0
    }
}

// ---------------------------------------------------------------------------
// The relation trait
// ---------------------------------------------------------------------------

/// A host-injected relation invoked from predicate position.
///
/// Object-safe and shared behind an [`Arc`] across the whole evaluation (including
/// fork-join workers), hence `Send + Sync`. It works entirely in dataset-independent
/// [`TermValue`] space: a relation never sees a dataset-local
/// [`TermId`](purrdf_core::TermId), so a registry built once is valid against any
/// dataset.
///
/// # Feasibility: which invocations a relation can serve
///
/// [`modes`](Self::modes) declares the access patterns the relation can compute,
/// over the flattened positions [`PfArity`] documents. An invocation whose own
/// pattern is `p` is **feasible** iff some declared mode `m` satisfies
/// `m.subsumes(p)` — that is, `bound(m) ⊆ bound(p)`, the subsumption order
/// [`BindingPattern`] defines.
///
/// The subset direction is the useful one: a relation that can serve `fb` (object
/// bound, subject free) can also serve `bb`, by producing the `fb` rows and letting
/// the engine's equality filter on the now-bound subject position discard the
/// mismatches — generate-then-filter. The converse does not hold, which is why a
/// relation that declares only `bf` genuinely cannot answer an `ff` invocation. A
/// relation that can serve everything declares exactly one mode, the all-free
/// [`PfArity::all_free_mode`], which subsumes every pattern of its arity.
///
/// # The ceiling is a licence, not a contract
///
/// [`open`](Self::open) receives the row ceiling the engine's answer-cap pushdown
/// computed for the node this invocation's rows belong to, when it has one — a `LIMIT`
/// or an answer cap, arrived at through the plan's soundness certificate. It is
/// permission to stop early, so that a generator (a text index, a nearest-neighbour
/// search) can bound its own work instead of producing rows nobody will read. The
/// engine stops consuming at its own ceiling regardless, so a relation that ignores the
/// argument entirely is correct, merely less efficient.
///
/// A relation that *does* use it counts **rows it emits that agree with the bound
/// positions it was handed**. That distinction is the whole of the obligation, and it
/// exists because a relation is entitled to generate candidates and let the engine's
/// equality filter cut them (see [`PfRow`]): candidates the relation itself can see are
/// doomed must not be counted against the licence, or a stop at `k` would hand back
/// fewer than `k` usable rows and the engine would read the short bag as an exhausted
/// one. Everything the engine filters on that a relation could *not* see — a repeated
/// variable across two free positions, a partially-bound quoted triple — is handled on
/// the engine's side: it withholds the ceiling entirely for such a call rather than ask
/// a relation to account for something it was never told.
pub trait PropertyFunction: Send + Sync {
    /// The relation's determinism class. Read by the fork-join parallel gate exactly
    /// as [`crate::user_fn::NativeFunction`]'s is: only
    /// [`Volatility::Stable`] may run across workers, and a
    /// relation that misdeclares itself diverges silently under parallel evaluation.
    fn volatility(&self) -> Volatility;

    /// The relation's declared positional arity. Checked against the call site before
    /// any of this trait's other methods run.
    fn arity(&self) -> PfArity;

    /// The access patterns this relation can compute, over the flattened positions
    /// [`PfArity`] documents. See the trait docs for the subsumption rule that turns
    /// this list into a yes/no answer for one invocation.
    ///
    /// Every returned pattern must have arity [`PfArity::total`]; a pattern of any
    /// other arity is incomparable with every invocation and so admits nothing.
    fn modes(&self) -> &[BindingPattern];

    /// The declared upper bound on the number of rows one invocation emits under
    /// `mode` — the cardinality class the planner reads.
    ///
    /// This is consulted twice: to order a call against the other operators of its
    /// group (a call that emits at most one row belongs before one that emits
    /// thousands), and to admit the call against a row ceiling before it runs. It is
    /// held to the same honesty contract as
    /// [`purrdf_core::DatasetView::cardinality_estimate`]:
    /// it must be an upper bound the relation actually respects, not a guess, because
    /// a bound that under-states reality turns an admission decision into a wrong one.
    /// A genuinely unbounded generator declares [`u64::MAX`].
    fn rows_per_invocation(&self, mode: BindingPattern) -> u64;

    /// Begin one invocation, returning its row cursor.
    ///
    /// `ceiling` is the optimization licence described in the trait docs — the number
    /// of further emitted-and-agreeing rows that can still reach the query's answer, or
    /// `None` for "no bound, or none the engine can offer here". `args` carries the
    /// bound/free shape of this call.
    ///
    /// # Errors
    ///
    /// Any [`EvalError`] the relation raises — including its refusal of an argument
    /// value it cannot use. Per the hard-fail doctrine a refused invocation aborts the
    /// query rather than contributing zero rows, which would be indistinguishable from
    /// an honest empty answer.
    fn open(&self, args: &PfArgs<'_>, ceiling: Option<u64>)
    -> Result<Box<dyn PfCursor>, EvalError>;

    /// Whether this relation can serve an invocation whose access pattern is
    /// `invocation`: some declared [`mode`](Self::modes) subsumes it.
    ///
    /// Provided rather than required — the rule is the lattice's, not the relation's,
    /// and a relation that could restate it could also restate it wrongly.
    fn admits(&self, invocation: BindingPattern) -> bool {
        self.modes().iter().any(|mode| mode.subsumes(invocation))
    }
}

// ---------------------------------------------------------------------------
// Panic containment
// ---------------------------------------------------------------------------

/// Arity-check `args` against `relation`'s declaration, then open the invocation with
/// the host call contained.
///
/// This is the ONLY way the evaluator enters a relation's `open`, and the supported way
/// for any other caller to do so. It is where the two guarantees that must hold before
/// host code runs are established:
///
/// 1. **Fail-fast arity.** A call whose argument vectors do not match the declared
///    [`PfArity`] never reaches the relation.
/// 2. **Panic containment.** A panicking relation becomes a clean
///    [`EvalError::Function`] rather than aborting a rayon worker. The message is
///    fixed and payload-free, so it is identical no matter which worker panicked —
///    the same treatment [`crate::user_fn`]'s native-function entry gives a native
///    closure, for the same determinism reason.
///
/// # Errors
///
/// [`EvalError::Function`] on an arity mismatch or a caught panic; otherwise the
/// relation's own error, propagated unchanged.
pub fn open_contained(
    relation: &dyn PropertyFunction,
    iri: &str,
    args: &PfArgs<'_>,
    ceiling: Option<u64>,
) -> Result<Box<dyn PfCursor>, EvalError> {
    let declared = declaration_contained(iri, "arity", || relation.arity())?;
    let supplied = args.arity();
    if declared != supplied {
        return Err(EvalError::function(format!(
            "property function <{iri}> expects {declared} argument(s), got {supplied}"
        )));
    }
    match catch_unwind(AssertUnwindSafe(|| relation.open(args, ceiling))) {
        Ok(opened) => opened,
        Err(_) => Err(EvalError::function(format!(
            "property function <{iri}> panicked while opening an invocation"
        ))),
    }
}

/// Pull one row from `cursor` with the host call contained.
///
/// The [`open_contained`] twin, for the other half of the invocation: a relation can
/// panic on any `next`, not merely the first, so every pull crosses this boundary.
///
/// # Errors
///
/// [`EvalError::Function`] on a caught panic; otherwise the cursor's own error,
/// propagated unchanged.
pub fn next_contained(cursor: &mut dyn PfCursor, iri: &str) -> Result<Option<PfRow>, EvalError> {
    match catch_unwind(AssertUnwindSafe(|| cursor.next())) {
        Ok(row) => row,
        Err(_) => Err(EvalError::function(format!(
            "property function <{iri}> panicked while producing a row"
        ))),
    }
}

/// Take `cursor`'s reported work with the host call contained.
///
/// The third member of the [`open_contained`]/[`next_contained`] family, for the third
/// thing a cursor can be asked. [`PfCursor::take_work`] is host code exactly as `next`
/// is — a counter that overflows an index, an assertion left in by mistake — and its
/// answer is *spent* against the caller's fuel, so it crosses the same boundary. A panic
/// becomes a clean, payload-free [`EvalError::Function`] rather than aborting a worker.
///
/// # Errors
///
/// [`EvalError::Function`] on a caught panic; otherwise `Ok` of the reported count.
pub fn take_work_contained(cursor: &mut dyn PfCursor, iri: &str) -> Result<u64, EvalError> {
    match catch_unwind(AssertUnwindSafe(|| cursor.take_work())) {
        Ok(units) => Ok(units),
        Err(_) => Err(EvalError::function(format!(
            "property function <{iri}> panicked while reporting its work"
        ))),
    }
}

/// Read one of a relation's DECLARATIONS — `arity`, `modes`, `volatility`, or
/// `rows_per_invocation` — with the panic contained.
///
/// The [`open_contained`]/[`next_contained`] twin for the other host calls a relation
/// receives. `arity`, `modes`, `volatility` and `rows_per_invocation` are exactly as much
/// host Rust as `open`/`next` — a lazily-built mode table indexed out of bounds, an
/// assertion a relation's author left in by mistake — and every one of them is reachable
/// from a caller-supplied `dyn PropertyFunction` exactly as `open`/`next` are. Routing
/// every declaration read through this is what makes this module's trust-boundary claim
/// (every invariant is checked, contained, or applied) true of the declaration surface
/// too, not merely of `open`/`next`: `what` names the declaration being read, so every
/// caller's message says which one panicked without leaking the panic's own payload.
///
/// # Errors
///
/// [`EvalError::Function`] on a caught panic; otherwise `Ok` of `read`'s result.
pub fn declaration_contained<T>(
    iri: &str,
    what: &str,
    read: impl FnOnce() -> T,
) -> Result<T, EvalError> {
    // Delegates to the shared containment helper every extension seam uses
    // (`crate::contain`) — see that module's docs for why the panic payload is
    // never interpolated. Kept as a thin `kind = "property function"` wrapper
    // here rather than inlined at call sites, so this function's public
    // signature and its message shape (`property function <iri> panicked while
    // reporting its {what}`) stay exactly what every existing caller and test
    // already depends on.
    crate::contain::declaration_contained("property function", iri, what, read)
}

// ---------------------------------------------------------------------------
// The registry
// ---------------------------------------------------------------------------

/// One declared mode of a relation, as reported by [`PropertyFunctionRegistry::describe`]:
/// the mode's lattice [`code`](BindingPattern::code) paired with the row bound the
/// relation declares for it.
///
/// Paired rather than carried as two parallel vectors, so a reader cannot mis-align a
/// cardinality with a mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PfMode {
    /// The mode's per-position code (`'b'` bound, `'f'` free, position 0 first).
    pub code: String,
    /// The relation's declared upper bound on rows per invocation under this mode.
    pub rows_per_invocation: u64,
}

/// A registered relation's self-description — the channel through which a host, a
/// diagnostic, or an explain surface can read what a registry actually contains
/// without holding a `dyn` reference to each relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PfDescriptor {
    /// The IRI the relation is registered under, byte-exact.
    pub iri: String,
    /// The declared number of subject-side arguments.
    pub subject_arity: usize,
    /// The declared number of object-side arguments.
    pub object_arity: usize,
    /// The declared determinism class.
    pub volatility: Volatility,
    /// The declared access patterns, in the order the relation returns them, each
    /// with its declared row bound.
    pub modes: Vec<PfMode>,
}

/// A caller-injected table of property functions, keyed by predicate IRI.
///
/// Built once per host configuration and borrowed into evaluation via
/// [`EvalCtx::with_property_functions`](crate::eval::EvalCtx::with_property_functions)
/// or [`NativeSparqlEngine::query_with_options_view`](crate::NativeSparqlEngine::query_with_options_view)
/// (via `QueryOptions::property_functions`).
/// Deterministic by construction: the map is the crate's fixed-key
/// deterministic fixed-key hash map, and every ordered surface
/// ([`describe`](Self::describe), [`Debug`]) sorts by IRI rather than reading
/// iteration order.
///
/// # Registration is fail-fast on a duplicate — deliberately stricter than
/// [`UserFunctionRegistry`](crate::user_fn::UserFunctionRegistry)
///
/// [`register`](Self::register) **panics** when an IRI is already registered, where
/// the user-function registry lets a second registration of the same kind win. The
/// asymmetry is not an oversight. A shadowed *function* returns a different value
/// under an IRI the host chose twice; a shadowed *relation* silently changes which
/// rows a graph pattern produces — it is a wrong-answer channel that no query text
/// can reveal, because both spellings of the call are identical. Two relations under
/// one IRI is a host misconfiguration, and it is caught where it is committed.
///
/// # Instance identity, not just declared contents
///
/// `id` is a `RegistryId` (`crate::registry_id::RegistryId`) minted fresh by
/// `Default`/[`new`](Self::new) — see that type's docs for why a counter, why a
/// counter is enough, and why [`Clone`] inherits rather than re-mints it. It
/// exists because DECLARED metadata (arity, volatility, modes and their row
/// bounds) cannot distinguish two independently built registries that happen to
/// register the SAME IRI to two DIFFERENT [`PropertyFunction`] implementations
/// with identical declarations — `crate::property_fn_plan::registry_fingerprint`
/// folds this id in ahead of the declaration digest so those two registries can
/// never be mistaken for each other by a prepared plan's identity.
#[derive(Default, Clone)]
pub struct PropertyFunctionRegistry {
    id: crate::registry_id::RegistryId,
    relations: DetHashMap<String, Arc<dyn PropertyFunction>>,
}

impl core::fmt::Debug for PropertyFunctionRegistry {
    /// A `dyn PropertyFunction` has no `Debug` impl, so this lists the registered
    /// IRIs, sorted for deterministic output, rather than deriving. The instance
    /// id rides along too, since it is exactly the thing that can make two
    /// otherwise-identical-looking registries diagnostically distinguishable.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut iris: Vec<&str> = self.relations.keys().map(String::as_str).collect();
        iris.sort_unstable();
        f.debug_struct("PropertyFunctionRegistry")
            .field("id", &self.id)
            .field("relations", &iris)
            .finish()
    }
}

impl PropertyFunctionRegistry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The canonical empty registry — the non-optional "no relations
    /// registered" value every registry-carrying seam
    /// ([`crate::engine::QueryOptions::property_functions`],
    /// [`crate::eval::EvalCtx::property_functions`](crate::eval::EvalCtx),
    /// `crate::parallel::SafetyRegistries::relations`) now uses in place of the
    /// old `Option::None` spelling. See
    /// [`crate::agg_fn::AggregateRegistry::EMPTY`]'s docs for why a single shared
    /// `'static` value, with a fixed reserved
    /// `RegistryId` (`crate::registry_id::RegistryId`), is the correct (not merely
    /// convenient) choice here: an empty registry resolves no IRI regardless of
    /// which `EMPTY` value is asked, so no plan's admitted behavior can depend on
    /// which one it was prepared against.
    pub const EMPTY: Self = Self {
        id: crate::registry_id::RegistryId::EMPTY,
        relations: DetHashMap::with_hasher(crate::DetHasher::new()),
    };

    /// Register `relation` under `iri`.
    ///
    /// # Panics
    ///
    /// Panics if `iri` is already registered — see the type's docs for why a relation
    /// may not be silently shadowed.
    pub fn register(&mut self, iri: impl Into<String>, relation: Arc<dyn PropertyFunction>) {
        let iri = iri.into();
        assert!(
            !self.relations.contains_key(&iri),
            "IRI <{iri}> is already registered as a property function; a relation may not be \
             silently shadowed, because both spellings of the call are identical and the only \
             observable difference is which rows the query returns"
        );
        self.relations.insert(iri, relation);
    }

    /// Resolve a predicate IRI to its registered relation, if any.
    #[must_use]
    pub fn resolve(&self, iri: &str) -> Option<&Arc<dyn PropertyFunction>> {
        self.relations.get(iri)
    }

    /// Whether the registry holds no relations — the common case, in which evaluation
    /// carries no registry at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.relations.is_empty()
    }

    /// The number of registered relations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.relations.len()
    }

    /// This registry's instance identity — read by
    /// `crate::property_fn_plan::registry_fingerprint`, which lives in a sibling
    /// module and so cannot reach the private `id` field directly. See the type's
    /// docs' "Instance identity" section for why this exists and why `Clone`
    /// inherits rather than re-mints it.
    pub(crate) const fn instance_id(&self) -> crate::registry_id::RegistryId {
        self.id
    }

    /// Describe every registered relation, sorted by IRI.
    ///
    /// The sort makes the output a pure function of the registry's contents rather
    /// than of its construction order, so two hosts that register the same relations
    /// in different orders describe identically.
    ///
    /// Every declaration read (`arity`, `volatility`, `modes`, `rows_per_invocation`)
    /// goes through [`declaration_contained`], because this runs on every prepare (it
    /// feeds the plan cache's fingerprint) — a relation whose declaration methods panic
    /// must fail that prepare cleanly, not abort it.
    ///
    /// # Errors
    ///
    /// [`EvalError::Function`] if any registered relation's declaration methods panic.
    pub fn describe(&self) -> Result<Vec<PfDescriptor>, EvalError> {
        let mut out: Vec<PfDescriptor> = Vec::with_capacity(self.relations.len());
        for (iri, relation) in &self.relations {
            let arity = declaration_contained(iri, "arity", || relation.arity())?;
            let volatility =
                declaration_contained(iri, "determinism class", || relation.volatility())?;
            let modes = declaration_contained(iri, "declared modes", || relation.modes().to_vec())?;
            let mut described_modes = Vec::with_capacity(modes.len());
            for mode in modes {
                let rows_per_invocation =
                    declaration_contained(iri, "row bound", || relation.rows_per_invocation(mode))?;
                described_modes.push(PfMode {
                    code: mode.code(),
                    rows_per_invocation,
                });
            }
            out.push(PfDescriptor {
                iri: iri.clone(),
                subject_arity: arity.subject,
                object_arity: arity.object,
                volatility,
                modes: described_modes,
            });
        }
        out.sort_by(|a, b| a.iri.cmp(&b.iri));
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// The in-memory reference relation
// ---------------------------------------------------------------------------

/// A deterministic in-memory relation: a fixed table of rows, scanned linearly.
///
/// This is the reference implementation of [`PropertyFunction`] — the shape every
/// other relation is measured against — and it is directly useful: a host with a
/// small extensional table (a code list, a unit-conversion table, a fixture in a
/// test) can expose it to SPARQL without writing an evaluator.
///
/// * **Every mode is served.** It declares exactly one mode, the all-free
///   [`PfArity::all_free_mode`], which subsumes every access pattern of its arity —
///   so any invocation is feasible. Bound positions are applied as per-position term
///   equality during the scan; the engine's own equality filter on bound positions
///   then has nothing left to remove.
/// * **Order is insertion order.** Rows are emitted in the order they were supplied,
///   filtered but never reordered.
/// * **The row ceiling is honoured.** Given a ceiling, the cursor stops after emitting
///   that many rows — counting the rows it emits, not the rows it skips, which is the
///   accounting the licence requires (see [`PropertyFunction`]'s ceiling contract).
///   Because the emitted rows are the first ones the unbounded scan would have produced,
///   the answer is the same one, computed over less of the table.
/// * **The row bound is exact.** [`PropertyFunction::rows_per_invocation`] reports the
///   table's row count, which no invocation can exceed.
/// * **It is [`Volatility::Stable`]**: a frozen table is deterministic for the
///   lifetime of a query, so it may run across fork-join workers.
///
/// ```
/// use std::sync::Arc;
///
/// use purrdf_core::TermValue;
/// use purrdf_sparql_eval::{MemoryRelation, PfArgs, PropertyFunction};
///
/// let iri = |s: &str| TermValue::iri(s);
/// // A two-column relation: one subject-side argument, one object-side argument.
/// let relation = MemoryRelation::new(
///     1,
///     1,
///     vec![
///         vec![iri("http://example.org/a"), iri("http://example.org/1")],
///         vec![iri("http://example.org/b"), iri("http://example.org/2")],
///     ],
/// )
/// .expect("every row is two columns wide");
///
/// // Invoke it with the subject bound and the object free: `bf`.
/// let subject_value = iri("http://example.org/b");
/// let subject = [Some(&subject_value)];
/// let object = [None];
/// let args = PfArgs::new(&subject, &object);
/// assert_eq!(args.mode().code(), "bf");
/// assert!(relation.admits(args.mode()));
///
/// let mut cursor = relation.open(&args, None).expect("open");
/// let row = cursor.next().expect("no error").expect("one matching row");
/// assert_eq!(row, vec![iri("http://example.org/b"), iri("http://example.org/2")]);
/// assert!(cursor.next().expect("no error").is_none());
/// ```
#[derive(Debug, Clone)]
pub struct MemoryRelation {
    arity: PfArity,
    rows: Arc<Vec<PfRow>>,
    /// The single declared mode (all-free), materialized once so
    /// [`PropertyFunction::modes`] can hand out a slice.
    modes: [BindingPattern; 1],
}

impl MemoryRelation {
    /// Build a relation over `rows`, split into `subject_arity` subject-side and
    /// `object_arity` object-side positions.
    ///
    /// # Errors
    ///
    /// [`EvalError::Config`] if any row's width differs from
    /// `subject_arity + object_arity`. A ragged table is a host configuration error,
    /// caught where it is supplied rather than surfacing later as a row the engine
    /// cannot bind.
    pub fn new(
        subject_arity: usize,
        object_arity: usize,
        rows: Vec<PfRow>,
    ) -> Result<Self, EvalError> {
        let arity = PfArity::new(subject_arity, object_arity);
        let width = arity.total();
        for (index, row) in rows.iter().enumerate() {
            if row.len() != width {
                return Err(EvalError::config(format!(
                    "in-memory property-function row {index} has {} value(s); the declared \
                     arity ({arity}) requires {width}",
                    row.len()
                )));
            }
        }
        Ok(Self {
            arity,
            rows: Arc::new(rows),
            modes: [arity.all_free_mode()],
        })
    }

    /// Build a relation by reading its rows out of `dataset`: an `rdf:List` whose head
    /// is `head` and whose members are themselves `rdf:List`s, one per row, each
    /// holding that row's values in flattened order.
    ///
    /// The nested-list encoding is the one RDF already has for an ordered tuple, so a
    /// host can ship a relation as data — in the same graph as the shapes or the
    /// configuration that names it — instead of as Rust.
    ///
    /// ```text
    /// @prefix ex: <http://example.org/> .
    ///
    /// ex:codes ex:table ( ( ex:a ex:1 ) ( ex:b ex:2 ) ) .
    /// ```
    ///
    /// Reading `ex:table`'s object as `head`, with `subject_arity` 1 and
    /// `object_arity` 1, yields the two-row relation of the [`MemoryRelation`] example.
    /// Row order is list order, so the relation's emission order is the order the data
    /// was written in.
    ///
    /// # Errors
    ///
    /// * [`EvalError::Data`] if `head` or any row head is a torn or malformed
    ///   `rdf:List` (a cell missing its `rdf:first`, a multi-valued `rdf:first`, or an
    ///   `rdf:rest` pointing at a non-cell), or if a row's length differs from
    ///   `subject_arity + object_arity`. Both are bad *input* rather than a caller API
    ///   misuse, so they are distinguished from [`Self::new`]'s
    ///   [`EvalError::Config`].
    /// * [`EvalError::Data`] if `head` is not interned in `dataset` at all: a head
    ///   naming a list that does not exist is a configuration pointing at nothing, not
    ///   an empty relation.
    pub fn from_graph<D: DatasetView>(
        dataset: &D,
        head: &TermValue,
        graph: GraphMatch<D::Id>,
        subject_arity: usize,
        object_arity: usize,
    ) -> Result<Self, EvalError> {
        let arity = PfArity::new(subject_arity, object_arity);
        let width = arity.total();
        let Some(head_id) = dataset.term_id_by_value(head) else {
            return Err(EvalError::data(format!(
                "property-function table head {head:?} is not present in the dataset"
            )));
        };
        let row_heads = dataset.rdf_list(head_id, graph).map_err(|e| {
            EvalError::data(format!("property-function table is not an rdf:List: {e}"))
        })?;

        let mut rows: Vec<PfRow> = Vec::with_capacity(row_heads.len());
        for (index, row_head) in row_heads.into_iter().enumerate() {
            let cells = dataset.rdf_list(row_head, graph).map_err(|e| {
                EvalError::data(format!(
                    "property-function table row {index} is not an rdf:List: {e}"
                ))
            })?;
            if cells.len() != width {
                return Err(EvalError::data(format!(
                    "property-function table row {index} has {} value(s); the declared arity \
                     ({arity}) requires {width}",
                    cells.len()
                )));
            }
            rows.push(
                cells
                    .into_iter()
                    .map(|id| crate::scratch::term_id_to_value(dataset, id))
                    .collect(),
            );
        }

        Ok(Self {
            arity,
            rows: Arc::new(rows),
            modes: [arity.all_free_mode()],
        })
    }

    /// The relation's rows, in emission order.
    #[must_use]
    pub fn rows(&self) -> &[PfRow] {
        &self.rows
    }
}

impl PropertyFunction for MemoryRelation {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        self.arity
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        // Exact, and mode-independent: a filtered scan of the table cannot emit more
        // rows than the table holds, whichever positions are bound.
        self.rows.len() as u64
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        // `open_contained` already checked this for every engine-driven call; a
        // direct caller (a doctest, a host wiring its own harness) gets the same
        // answer rather than a panic on a short row.
        let supplied = args.arity();
        if supplied != self.arity {
            return Err(EvalError::function(format!(
                "in-memory property function expects {} argument(s), got {supplied}",
                self.arity
            )));
        }
        // The bound positions are cloned into the cursor: a cursor outlives the
        // borrow `args` lends, and a bound argument vector is at most `arity` values
        // wide — the per-invocation clone the row scan then reads.
        let filter: Vec<Option<TermValue>> =
            args.flattened().map(<Option<&TermValue>>::cloned).collect();
        Ok(Box::new(MemoryCursor {
            rows: Arc::clone(&self.rows),
            filter,
            next_index: 0,
            remaining: ceiling,
        }))
    }
}

/// The cursor [`MemoryRelation::open`] returns: a linear scan of the shared row
/// table, emitting the rows that agree with every bound position.
#[derive(Debug)]
struct MemoryCursor {
    rows: Arc<Vec<PfRow>>,
    /// The invocation's bound values by flattened position (`None` = free).
    filter: Vec<Option<TermValue>>,
    next_index: usize,
    /// The rows this invocation may still emit under the engine's licence, or `None`
    /// when it was given no ceiling.
    ///
    /// This is the reference implementation of the licence, and the shape a host
    /// relation copies. Two properties make it sound, and both are load-bearing:
    ///
    /// * It counts **emitted** rows, never scanned ones. The rows this scan skips
    ///   disagree with a bound position and would have been cut by the engine's own
    ///   equality filter anyway, so counting them would spend the licence on rows the
    ///   engine was never going to keep — the miscount the trait docs warn about.
    /// * It stops **producing**, it does not report an error or a short-but-different
    ///   bag: the rows already emitted are the first rows of the full scan, in the
    ///   same order, which is exactly what the licence was granted against.
    remaining: Option<u64>,
}

impl PfCursor for MemoryCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        if self.remaining == Some(0) {
            return Ok(None);
        }
        while let Some(row) = self.rows.get(self.next_index) {
            self.next_index += 1;
            let matches = self
                .filter
                .iter()
                .zip(row.iter())
                .all(|(bound, value)| bound.as_ref().is_none_or(|bound| bound == value));
            if matches {
                if let Some(remaining) = self.remaining.as_mut() {
                    *remaining = remaining.saturating_sub(1);
                }
                return Ok(Some(row.clone()));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_core::binding_pattern::BindingPattern;
    use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};

    use super::*;

    const EX_A: &str = "http://example.org/a";
    const EX_B: &str = "http://example.org/b";
    const EX_ONE: &str = "http://example.org/1";
    const EX_TWO: &str = "http://example.org/2";
    const EX_SPLIT: &str = "http://example.org/ns#split";
    const EX_OTHER: &str = "http://example.org/ns#other";

    fn iri(s: &str) -> TermValue {
        TermValue::iri(s)
    }

    /// The two-row, one-subject/one-object reference table.
    fn table() -> MemoryRelation {
        MemoryRelation::new(
            1,
            1,
            vec![vec![iri(EX_A), iri(EX_ONE)], vec![iri(EX_B), iri(EX_TWO)]],
        )
        .expect("uniform rows")
    }

    /// Drain a cursor into a vector of rows.
    fn drain(cursor: &mut dyn PfCursor) -> Vec<PfRow> {
        let mut out = Vec::new();
        while let Some(row) = cursor.next().expect("no error") {
            out.push(row);
        }
        out
    }

    /// Open `relation` with the given per-position bindings (flattened, split at
    /// `subject_arity`) and drain it.
    fn invoke(relation: &MemoryRelation, bound: &[Option<TermValue>]) -> Vec<PfRow> {
        let refs: Vec<Option<&TermValue>> = bound.iter().map(Option::as_ref).collect();
        let (subject, object) = refs.split_at(relation.arity().subject);
        let args = PfArgs::new(subject, object);
        let mut cursor = relation.open(&args, None).expect("open");
        drain(&mut *cursor)
    }

    // ---- the positional model --------------------------------------------

    #[test]
    fn flattened_positions_are_subject_then_object() {
        let arity = PfArity::new(2, 1);
        assert_eq!(arity.total(), 3);
        let s0 = iri(EX_A);
        let o0 = iri(EX_ONE);
        let subject = [Some(&s0), None];
        let object = [Some(&o0)];
        let args = PfArgs::new(&subject, &object);
        assert_eq!(args.arity(), arity);
        assert_eq!(args.get(0), Some(&s0));
        assert_eq!(args.get(1), None);
        assert_eq!(args.get(2), Some(&o0));
        assert_eq!(args.get(3), None, "out of range reads as free");
        assert_eq!(args.mode().code(), "bfb");
    }

    // ---- registry --------------------------------------------------------

    #[test]
    fn register_and_resolve() {
        let mut registry = PropertyFunctionRegistry::new();
        assert!(registry.is_empty());
        registry.register(EX_SPLIT, Arc::new(table()));
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
        assert!(registry.resolve(EX_SPLIT).is_some());
        assert!(registry.resolve(EX_OTHER).is_none());
    }

    #[test]
    #[should_panic(expected = "already registered as a property function")]
    fn duplicate_registration_panics() {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(EX_SPLIT, Arc::new(table()));
        registry.register(EX_SPLIT, Arc::new(table()));
    }

    #[test]
    fn describe_is_sorted_by_iri_and_pairs_modes_with_bounds() {
        let mut registry = PropertyFunctionRegistry::new();
        // Registered in reverse IRI order, so the sort is observable.
        registry.register(EX_SPLIT, Arc::new(table()));
        registry.register(EX_OTHER, Arc::new(table()));
        let described = registry.describe().expect("no relation panics");
        assert_eq!(
            described.iter().map(|d| d.iri.as_str()).collect::<Vec<_>>(),
            vec![EX_OTHER, EX_SPLIT],
            "describe sorts by IRI, not by registration order"
        );
        let first = &described[0];
        assert_eq!(first.subject_arity, 1);
        assert_eq!(first.object_arity, 1);
        assert_eq!(first.volatility, Volatility::Stable);
        assert_eq!(
            first.modes,
            vec![PfMode {
                code: "ff".to_owned(),
                rows_per_invocation: 2,
            }]
        );
    }

    #[test]
    fn debug_lists_iris_sorted() {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(EX_SPLIT, Arc::new(table()));
        registry.register(EX_OTHER, Arc::new(table()));
        let rendered = format!("{registry:?}");
        let other_at = rendered.find(EX_OTHER).expect("other listed");
        let split_at = rendered.find(EX_SPLIT).expect("split listed");
        assert!(
            other_at < split_at,
            "Debug output is IRI-sorted: {rendered}"
        );
    }

    // ---- feasibility -----------------------------------------------------

    #[test]
    fn the_all_free_mode_admits_every_invocation() {
        let relation = table();
        for code in ["ff", "bf", "fb", "bb"] {
            assert!(
                relation.admits(BindingPattern::from_code(code)),
                "the all-free declaration must admit {code}"
            );
        }
    }

    #[test]
    fn a_declared_mode_admits_only_its_supersets() {
        // A relation that can only enumerate objects from a bound subject declares
        // `bf`: it serves `bf` and `bb` (generate-then-filter), never `ff` or `fb`.
        let bf = BindingPattern::from_code("bf");
        assert!(bf.subsumes(BindingPattern::from_code("bf")));
        assert!(bf.subsumes(BindingPattern::from_code("bb")));
        assert!(!bf.subsumes(BindingPattern::from_code("ff")));
        assert!(!bf.subsumes(BindingPattern::from_code("fb")));
    }

    // ---- MemoryRelation over every mode ----------------------------------

    #[test]
    fn memory_relation_serves_every_mode() {
        let relation = table();

        // ff — the whole table, in insertion order.
        assert_eq!(
            invoke(&relation, &[None, None]),
            vec![vec![iri(EX_A), iri(EX_ONE)], vec![iri(EX_B), iri(EX_TWO)],]
        );
        // bf — bound subject.
        assert_eq!(
            invoke(&relation, &[Some(iri(EX_B)), None]),
            vec![vec![iri(EX_B), iri(EX_TWO)]]
        );
        // fb — bound object.
        assert_eq!(
            invoke(&relation, &[None, Some(iri(EX_ONE))]),
            vec![vec![iri(EX_A), iri(EX_ONE)]]
        );
        // bb — both bound, agreeing.
        assert_eq!(
            invoke(&relation, &[Some(iri(EX_A)), Some(iri(EX_ONE))]),
            vec![vec![iri(EX_A), iri(EX_ONE)]]
        );
        // bb — both bound, disagreeing: no row.
        assert!(invoke(&relation, &[Some(iri(EX_A)), Some(iri(EX_TWO))]).is_empty());
    }

    #[test]
    fn memory_relation_emission_order_is_insertion_order() {
        let relation = MemoryRelation::new(
            1,
            1,
            vec![vec![iri(EX_B), iri(EX_TWO)], vec![iri(EX_A), iri(EX_ONE)]],
        )
        .expect("uniform rows");
        assert_eq!(
            invoke(&relation, &[None, None]),
            vec![vec![iri(EX_B), iri(EX_TWO)], vec![iri(EX_A), iri(EX_ONE)],],
            "rows are emitted as supplied, never sorted"
        );
    }

    /// Open `relation` all-free under `ceiling` and drain it.
    fn invoke_under_ceiling(relation: &MemoryRelation, ceiling: Option<u64>) -> Vec<PfRow> {
        let subject = [None];
        let object = [None];
        let args = PfArgs::new(&subject, &object);
        let mut cursor = relation.open(&args, ceiling).expect("open");
        drain(&mut *cursor)
    }

    #[test]
    fn memory_relation_stops_at_the_row_ceiling() {
        // The reference implementation of the licence, and the behaviour a host relation
        // copies: a ceiling of `k` yields the FIRST `k` rows of the unbounded scan, in
        // the same order, and then reports exhaustion.
        let rows: Vec<PfRow> = (0..100)
            .map(|i| vec![iri(EX_A), iri(&format!("http://example.org/r{i:03}"))])
            .collect();
        let relation = MemoryRelation::new(1, 1, rows.clone()).expect("uniform rows");

        let full = invoke_under_ceiling(&relation, None);
        assert_eq!(full.len(), 100, "no ceiling scans the whole table");

        let capped = invoke_under_ceiling(&relation, Some(3));
        assert_eq!(
            capped,
            rows[..3].to_vec(),
            "a ceiling of 3 emits the first three rows and nothing else"
        );

        // A ceiling of zero emits nothing at all, without a first pull into the table.
        assert!(invoke_under_ceiling(&relation, Some(0)).is_empty());
        // A ceiling above the table is simply never reached.
        assert_eq!(invoke_under_ceiling(&relation, Some(1_000)).len(), 100);
    }

    #[test]
    fn memory_relation_counts_emitted_rows_not_scanned_ones() {
        // The accounting that makes the licence sound: rows the scan SKIPS disagree with
        // a bound position and would have been cut by the engine's equality filter
        // anyway, so spending the ceiling on them would hand back fewer usable rows than
        // the engine asked for.
        let relation = MemoryRelation::new(
            1,
            1,
            vec![
                vec![iri(EX_A), iri(EX_ONE)],
                vec![iri(EX_B), iri(EX_TWO)],
                vec![iri(EX_B), iri(EX_ONE)],
            ],
        )
        .expect("uniform rows");

        let subject_value = iri(EX_B);
        let subject = [Some(&subject_value)];
        let object = [None];
        let args = PfArgs::new(&subject, &object);
        let mut cursor = relation.open(&args, Some(1)).expect("open");
        assert_eq!(
            drain(&mut *cursor),
            vec![vec![iri(EX_B), iri(EX_TWO)]],
            "the skipped ex:a row must not have consumed the single-row licence"
        );
    }

    #[test]
    fn memory_relation_row_bound_is_the_row_count() {
        let relation = table();
        for code in ["ff", "bf", "fb", "bb"] {
            assert_eq!(
                relation.rows_per_invocation(BindingPattern::from_code(code)),
                2
            );
        }
    }

    #[test]
    fn ragged_rows_are_a_configuration_error() {
        let error = MemoryRelation::new(1, 1, vec![vec![iri(EX_A)]])
            .expect_err("a one-value row cannot fill two positions");
        assert!(matches!(error, EvalError::Config(_)), "got {error:?}");
        assert!(error.to_string().contains("requires 2"), "got {error}");
    }

    #[test]
    fn wrong_argument_count_is_refused_before_the_scan() {
        let relation = table();
        let subject: [Option<&TermValue>; 0] = [];
        let object = [None];
        let args = PfArgs::new(&subject, &object);
        let Err(error) = relation.open(&args, None) else {
            panic!("a zero-argument subject side does not match the declaration");
        };
        assert!(matches!(error, EvalError::Function(_)), "got {error:?}");
    }

    // ---- from_graph ------------------------------------------------------

    /// `( ( ex:a ex:1 ) ( ex:b ex:2 ) )` reachable from `ex:codes ex:table ?head`,
    /// plus the `rows`-controlled row widths.
    fn table_dataset(rows: &[Vec<&str>]) -> (Arc<RdfDataset>, TermValue) {
        let mut b = RdfDatasetBuilder::new();
        let first = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#first");
        let rest = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#rest");
        let nil = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#nil");

        // Build each row list, then the outer list of row heads, tail-first so every
        // `rdf:rest` target already exists.
        let mut row_heads = Vec::new();
        for (row_index, row) in rows.iter().enumerate() {
            let mut cell = nil;
            for (cell_index, value) in row.iter().enumerate().rev() {
                let node = b.intern_blank(
                    &format!("r{row_index}c{cell_index}"),
                    purrdf_core::BlankScope::DEFAULT,
                );
                let value = b.intern_iri(value);
                b.push_quad(node, first, value, None);
                b.push_quad(node, rest, cell, None);
                cell = node;
            }
            row_heads.push(cell);
        }
        let mut outer = nil;
        for (index, row_head) in row_heads.iter().enumerate().rev() {
            let node = b.intern_blank(&format!("t{index}"), purrdf_core::BlankScope::DEFAULT);
            b.push_quad(node, first, *row_head, None);
            b.push_quad(node, rest, outer, None);
            outer = node;
        }
        let head_label = format!("t{}", 0);
        let subject = b.intern_iri("http://example.org/codes");
        let predicate = b.intern_iri("http://example.org/table");
        b.push_quad(subject, predicate, outer, None);
        let dataset = b.freeze().expect("freeze");
        (
            dataset,
            TermValue::Blank {
                label: head_label,
                scope: purrdf_core::BlankScope::DEFAULT,
            },
        )
    }

    #[test]
    fn from_graph_reads_a_list_of_lists_in_order() {
        let (dataset, head) = table_dataset(&[vec![EX_A, EX_ONE], vec![EX_B, EX_TWO]]);
        let relation =
            MemoryRelation::from_graph(&*dataset, &head, GraphMatch::Default, 1, 1).expect("read");
        assert_eq!(
            relation.rows(),
            &[vec![iri(EX_A), iri(EX_ONE)], vec![iri(EX_B), iri(EX_TWO)],]
        );
        assert_eq!(
            invoke(&relation, &[Some(iri(EX_B)), None]),
            vec![vec![iri(EX_B), iri(EX_TWO)]]
        );
    }

    #[test]
    fn from_graph_rejects_a_row_of_the_wrong_width() {
        let (dataset, head) = table_dataset(&[vec![EX_A, EX_ONE], vec![EX_B]]);
        let error = MemoryRelation::from_graph(&*dataset, &head, GraphMatch::Default, 1, 1)
            .expect_err("a one-value row cannot fill two positions");
        assert!(matches!(error, EvalError::Data(_)), "got {error:?}");
        assert!(error.to_string().contains("row 1"), "got {error}");
    }

    #[test]
    fn from_graph_rejects_an_absent_head() {
        let (dataset, _head) = table_dataset(&[vec![EX_A, EX_ONE]]);
        let error = MemoryRelation::from_graph(
            &*dataset,
            &iri("http://example.org/missing"),
            GraphMatch::Default,
            1,
            1,
        )
        .expect_err("a head that is not interned names no list");
        assert!(matches!(error, EvalError::Data(_)), "got {error:?}");
    }

    // ---- panic containment ------------------------------------------------

    /// A relation that panics wherever the constructor says to.
    #[derive(Debug)]
    struct PanickingRelation {
        on_open: bool,
        modes: [BindingPattern; 1],
    }

    impl PanickingRelation {
        fn new(on_open: bool) -> Self {
            Self {
                on_open,
                modes: [PfArity::new(1, 1).all_free_mode()],
            }
        }
    }

    impl PropertyFunction for PanickingRelation {
        fn volatility(&self) -> Volatility {
            Volatility::Stable
        }

        fn arity(&self) -> PfArity {
            PfArity::new(1, 1)
        }

        fn modes(&self) -> &[BindingPattern] {
            &self.modes
        }

        fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
            1
        }

        fn open(
            &self,
            _args: &PfArgs<'_>,
            _ceiling: Option<u64>,
        ) -> Result<Box<dyn PfCursor>, EvalError> {
            assert!(!self.on_open, "relation exploded in open");
            Ok(Box::new(PanickingCursor))
        }
    }

    struct PanickingCursor;

    impl PfCursor for PanickingCursor {
        fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
            panic!("relation exploded in next")
        }

        fn take_work(&mut self) -> u64 {
            panic!("relation exploded in take_work")
        }
    }

    /// Run `body` with the default panic hook suppressed, so an *expected*, caught
    /// panic does not dump to stderr (mirrors `user_fn`'s panic test).
    fn without_panic_output<R>(body: impl FnOnce() -> R) -> R {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let out = body();
        std::panic::set_hook(default_hook);
        out
    }

    #[test]
    fn a_panic_in_open_is_a_clean_payload_free_error() {
        let relation = PanickingRelation::new(true);
        let subject_value = iri(EX_A);
        let subject = [Some(&subject_value)];
        let object = [None];
        let args = PfArgs::new(&subject, &object);
        let error = without_panic_output(|| {
            let Err(error) = open_contained(&relation, EX_SPLIT, &args, None) else {
                panic!("a panicking open must not escape");
            };
            error
        });
        assert!(
            error.to_string().contains("panicked while opening"),
            "got {error}"
        );
        assert!(
            !error.to_string().contains("exploded"),
            "the payload must not leak into the deterministic message: {error}"
        );
    }

    #[test]
    fn a_panic_in_next_is_a_clean_payload_free_error() {
        let relation = PanickingRelation::new(false);
        let subject_value = iri(EX_A);
        let subject = [Some(&subject_value)];
        let object = [None];
        let args = PfArgs::new(&subject, &object);
        let mut cursor = open_contained(&relation, EX_SPLIT, &args, None).expect("open");
        let error = without_panic_output(|| {
            next_contained(&mut *cursor, EX_SPLIT).expect_err("a panicking next must not escape")
        });
        assert!(
            error.to_string().contains("panicked while producing a row"),
            "got {error}"
        );
        assert!(
            !error.to_string().contains("exploded"),
            "the payload must not leak into the deterministic message: {error}"
        );
    }

    #[test]
    fn take_work_defaults_to_zero_and_is_not_a_charge_a_relation_must_opt_out_of() {
        // The default is what makes the work channel additive rather than breaking: every
        // relation written before it existed reports nothing and is charged nothing.
        let relation = table();
        let subject = [None];
        let object = [None];
        let args = PfArgs::new(&subject, &object);
        let mut cursor = relation.open(&args, None).expect("open");
        assert_eq!(cursor.take_work(), 0);
        assert!(cursor.next().expect("no error").is_some());
        assert_eq!(
            cursor.take_work(),
            0,
            "an in-memory scan's cost IS its rows, so it has nothing to add"
        );
    }

    #[test]
    fn a_panic_in_take_work_is_a_clean_payload_free_error() {
        // `take_work` is host code exactly as `next` is, and its answer is SPENT against
        // the caller's fuel, so it crosses the same containment boundary.
        let relation = PanickingRelation::new(false);
        let subject_value = iri(EX_A);
        let subject = [Some(&subject_value)];
        let object = [None];
        let args = PfArgs::new(&subject, &object);
        let mut cursor = open_contained(&relation, EX_SPLIT, &args, None).expect("open");
        let error = without_panic_output(|| {
            take_work_contained(&mut *cursor, EX_SPLIT)
                .expect_err("a panicking take_work must not escape")
        });
        assert!(
            error
                .to_string()
                .contains("panicked while reporting its work"),
            "got {error}"
        );
        assert!(
            !error.to_string().contains("exploded"),
            "the payload must not leak into the deterministic message: {error}"
        );
    }

    #[test]
    fn open_contained_checks_arity_before_the_relation_runs() {
        // The relation would panic in `open`; the arity check must refuse the call
        // first, so the error names the arity rather than a panic.
        let relation = PanickingRelation::new(true);
        let subject: [Option<&TermValue>; 0] = [];
        let object = [None];
        let args = PfArgs::new(&subject, &object);
        let Err(error) = open_contained(&relation, EX_SPLIT, &args, None) else {
            panic!("a mismatched arity is refused");
        };
        assert!(error.to_string().contains("expects"), "got {error}");
        assert!(
            !error.to_string().contains("panicked"),
            "the check must run before the host code: {error}"
        );
    }

    // ---- declaration-read panic containment --------------------------------

    /// A relation whose `arity` panics — the declaration-read half of the trust
    /// boundary, distinct from [`PanickingRelation`]'s `open`/`next` half.
    #[derive(Debug)]
    struct PanickingArityRelation;

    impl PropertyFunction for PanickingArityRelation {
        fn volatility(&self) -> Volatility {
            Volatility::Stable
        }

        fn arity(&self) -> PfArity {
            panic!("arity exploded")
        }

        fn modes(&self) -> &[BindingPattern] {
            &[]
        }

        fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
            0
        }

        fn open(
            &self,
            _args: &PfArgs<'_>,
            _ceiling: Option<u64>,
        ) -> Result<Box<dyn PfCursor>, EvalError> {
            unreachable!("arity panics before open is ever reached")
        }
    }

    #[test]
    fn declaration_contained_catches_a_panic() {
        let error = without_panic_output(|| {
            declaration_contained(EX_SPLIT, "arity", || {
                let relation = PanickingArityRelation;
                relation.arity()
            })
            .expect_err("a panicking read must not escape")
        });
        assert!(
            error
                .to_string()
                .contains("panicked while reporting its arity"),
            "got {error}"
        );
        assert!(
            !error.to_string().contains("exploded"),
            "the payload must not leak into the deterministic message: {error}"
        );
    }

    #[test]
    fn describe_contains_a_panicking_relations_declaration() {
        // `describe()` is the registry fingerprint's engine — read on every prepare — so
        // a relation whose declaration methods panic must fail it cleanly, not abort.
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(EX_SPLIT, Arc::new(PanickingArityRelation));
        let error = without_panic_output(|| {
            registry
                .describe()
                .expect_err("a panicking arity must not escape describe")
        });
        assert!(
            error
                .to_string()
                .contains("panicked while reporting its arity"),
            "got {error}"
        );
        assert!(
            !error.to_string().contains("exploded"),
            "the payload must not leak into the deterministic message: {error}"
        );
    }

    #[test]
    fn prepare_contains_a_panicking_relations_declaration() {
        // The same failure, reached through the query-lane entry a host actually calls:
        // `registry_fingerprint` (which `prepare_with_relations`/`prepare_for` consult on
        // every prepare) drives `describe()` internally, so a panicking declaration must
        // surface as a contained `EvalError`, never an abort of the caller's thread.
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(EX_SPLIT, Arc::new(PanickingArityRelation));
        let error = without_panic_output(|| {
            crate::property_fn_plan::registry_fingerprint(&registry)
                .expect_err("a panicking declaration must not escape the fingerprint")
        });
        assert!(
            error
                .to_string()
                .contains("panicked while reporting its arity"),
            "got {error}"
        );
    }
}
