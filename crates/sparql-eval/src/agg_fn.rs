// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Host-injected **custom aggregates** — the fold algebra `AGG(<iri>, args…)`
//! dispatches against.
//!
//! `AggregateFunction::Custom(iri)`
//! ([`purrdf_sparql_algebra::AggregateFunction::Custom`]) names a `GROUP BY`
//! reduction the closed SPARQL 1.1 built-in set (`COUNT`/`SUM`/`AVG`/`MIN`/`MAX`/
//! `SAMPLE`/`GROUP_CONCAT`) does not cover, resolved at evaluation time against a
//! caller-injected [`AggregateRegistry`]. Where [`crate::property_fn`] injects a
//! *relation* (a row source in graph-pattern position) and [`crate::user_fn`]
//! injects a *scalar function* (one value per call, in expression position), this
//! module injects a **fold**: a `GROUP BY` group's rows reduce to one value through
//! a caller-supplied accumulator, exactly as a built-in aggregate does.
//!
//! # The fold shape, shared with the built-ins — literally, not just in prose
//!
//! Every aggregate — built-in or custom — is `init` (an aggregate's answer over the
//! empty group, `finish(init())`, is never absent: `COUNT` → `0`, `SUM` → `0`,
//! `AVG`/`MIN`/`MAX`/`SAMPLE` → unbound, `GROUP_CONCAT` → `""`), `step` (fold one
//! row's already-evaluated argument tuple in), and `finish` (produce the group's
//! answer, or `None` for unbound) — exactly [`AggregateAccumulator`]'s three
//! methods. Every built-in aggregate (`crate::modifier`'s `CountAccumulator`/
//! `SumAccumulator`/`AvgAccumulator`/`MinAccumulator`/`MaxAccumulator`/
//! `SampleAccumulator`/`GroupConcatAccumulator`) is a genuine
//! `impl AggregateAccumulator`, the SAME trait a [`CustomAggregate::init`] call
//! produces from a HOST registration — one fold algebra, not two. Only the
//! DISPATCH differs: a built-in's concrete accumulator type is known at compile
//! time, so `crate::modifier::fold_builtin` drives it by generic monomorphization
//! (an ordinary, inlinable call on every `step`, no vtable); a registered
//! aggregate's concrete type is not known until [`AggregateRegistry::resolve`]
//! looks up an IRI at RUN time, so it is necessarily driven through
//! `Box<dyn AggregateAccumulator>` — the one place this module's own dynamic
//! dispatch earns its cost. See `crate::modifier::eval_aggregate`'s doc comment
//! for the one dispatch SITE both paths share, and `crate::modifier::fold_builtin`'s
//! for why the static path never pays a per-row cost for implementing the
//! identical trait a dynamically dispatched aggregate uses.
//!
//! # The trust boundary
//!
//! A registered aggregate is arbitrary host Rust. As with [`crate::property_fn`],
//! every invariant the evaluator needs from it is either checked before host code
//! runs (arity, at prepare time — see `crate::property_fn_plan::plan_query`'s
//! walk), contained when host code panics (every one of `init`/`step`/`combine`/
//! `finish` and every declaration read, via the `crate::contain` helpers this
//! module shares with [`crate::property_fn`]), or applied to what host code
//! returns. A host aggregate that misbehaves cannot make the engine unsound; it can
//! only make its own call fail.
//!
//! # `combine` and determinism
//!
//! [`AggregateAccumulator::combine`] merges one accumulator's state into another's,
//! **in the chunk order the caller presents them** — never reordered, never run
//! concurrently on the same accumulator. That is what lets a non-commutative,
//! order-sensitive custom aggregate (a running median with a tie-break rule, a
//! last-write-wins accumulator) stay deterministic even when its group's rows were
//! folded by more than one worker: the merge order is fixed by source order, not by
//! which worker finished first. [`CustomAggregate::algebraic_class`] documents which
//! algebraic law a given aggregate satisfies, for a caller that wants to reason
//! about it; the evaluator itself does not need the classification to fold correctly,
//! since `combine`'s fixed chunk-order contract already makes the fixed-order reduce
//! safe for every class. `crate::modifier::eval_custom_aggregate` is the caller that
//! actually drives this: for a large enough group whose aggregate declares
//! [`Volatility::Stable`], it folds the group's rows in chunks (one accumulator per
//! chunk, via `crate::parallel::par_chunk_reduce_init`) and reduces the partial
//! accumulators through `combine`, strictly in chunk-index order — a `Volatile`
//! aggregate stays on the single-accumulator sequential fold this module always
//! supported, exactly as `crate::eval::EvalCtx::may_fork_aggregate`'s per-GROUP fork
//! gate excludes it there too.
//!
//! # Merging structural state through `into_any`
//!
//! `combine` receives `other` as a fully type-erased `Box<dyn
//! AggregateAccumulator>` — reachable, naively, through only the trait's own
//! object-safe surface. For an aggregate whose finished, single-term answer IS
//! sufficient mergeable state (`SUM`'s running total, `GROUP_CONCAT`'s joined
//! string), `other.finish()` is all `combine` ever needs — the shape this
//! module's own `SumAccumulator` test fixture and `crate::modifier`'s
//! `ListCollector` use. For an aggregate whose finished answer throws away
//! information a correct merge needs — a running median's whole value list, a
//! running mode's per-value counts, a running variance's `(n, Σx, Σx²)` — the
//! finish-only path is not merely inconvenient, it is WRONG, because it would
//! fold the finished, already-reduced answer back in as if it were one more
//! row rather than merging the aggregate's actual state.
//! [`AggregateAccumulator::into_any`] is the escape hatch for exactly this
//! case: it recovers `other`'s original concrete type, so `combine` can merge
//! the SAME structural state `step` builds. The downcast is same-type BY
//! CONSTRUCTION — every partial accumulator a single `combine` chain ever
//! merges was created by the SAME [`CustomAggregate::init`] factory
//! (`crate::parallel::par_chunk_reduce_init` never mixes accumulators from two
//! different registrations) — so a mismatch can only mean a host bug, never a
//! state this crate's own evaluator can produce; `downcast_combine_partial`
//! is the standard way to consume it, returning `Err(EvalError::Function)`
//! rather than panicking on a mismatch — `combine`'s own `Result` return type
//! (not `()`) is what makes that a plain propagated error instead of a panic
//! `combine_contained`'s `crate::contain` containment would otherwise have to
//! catch, which matters because that containment cannot run at all under
//! `panic = "abort"` (see the wasm32 note below). `crate::stat_agg`'s
//! `MEDIAN`/`PERCENTILE`/`STDDEV`-family/`MODE`/`TOPK` members are this crate's
//! first-party example of the pattern.
//!
//! # wasm32 note
//!
//! `combine_contained` still wraps every `combine` call in `crate::contain`'s
//! `catch_unwind`-based containment, for an implementation that panics for a
//! reason of its own outside the downcast — that containment still requires
//! `panic = "unwind"` and is unavailable under `panic = "abort"` exactly as every
//! other extension seam's is (see `crate::contain`'s module docs). The
//! `downcast_combine_partial` mismatch case above no longer depends on it: it is
//! a typed `Err`, not a panic, so it degrades cleanly on every target regardless
//! of panic strategy.

use std::sync::Arc;

use purrdf_core::TermValue;

use crate::DetHashMap;
use crate::error::EvalError;
use crate::user_fn::{Arity, Volatility};

// ---------------------------------------------------------------------------
// The algebraic shape
// ---------------------------------------------------------------------------

/// The algebraic law a custom aggregate's fold satisfies — informational, read by a
/// caller (or a future planner) that wants to reason about reordering or
/// parallelizing a fold, never by the evaluator to change what it computes.
///
/// `#[non_exhaustive]`: a finer class is addable without a breaking change.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum AlgebraicClass {
    /// `step(a); step(b)` and `step(b); step(a)` yield the same [`finish`](AggregateAccumulator::finish)
    /// (`SUM`, `COUNT`, `AVG`'s two running totals): order-independent, so a
    /// group's rows may be folded, or partial folds combined, in ANY order.
    Commutative,
    /// Order-independent only under [`AggregateAccumulator::combine`]'s fixed
    /// merge rule — reassociating the fold is safe, but the input order still
    /// matters for a single `step` sequence (a running min/max under a
    /// non-total order, say). Weaker than [`Self::Commutative`], stronger than
    /// [`Self::OrderDependent`].
    Associative,
    /// The fold's answer depends on row order (`SAMPLE`-like "first value wins",
    /// `GROUP_CONCAT`-like ordered concatenation): `step` must see rows in the
    /// query's row order, and [`AggregateAccumulator::combine`]'s chunk-order
    /// contract is what keeps this deterministic when a group was folded by more
    /// than one worker.
    OrderDependent,
}

impl AlgebraicClass {
    /// A stable diagnostic label — used by `registry_fingerprint`, mirroring
    /// [`Volatility::label`].
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Commutative => "commutative",
            Self::Associative => "associative",
            Self::OrderDependent => "order-dependent",
        }
    }
}

// ---------------------------------------------------------------------------
// Named scalar-value parameters
// ---------------------------------------------------------------------------

/// The value kind a [`ScalarvalSpec`] admits — checked at PREPARE time
/// (`crate::property_fn_plan::plan_aggregate`) against the LITERAL's datatype
/// the query text supplied for a `; NAME=value` scalarval clause (see
/// [`purrdf_sparql_algebra::AggregateExpression::scalarvals`]'s docs), never
/// against a runtime-evaluated value — a scalarval is never per-row.
///
/// `#[non_exhaustive]`: a finer kind (e.g. a specific numeric sub-tower) is
/// addable without a breaking change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScalarvalKind {
    /// Any member of the SPARQL numeric tower (`xsd:integer`/`xsd:decimal`/
    /// `xsd:float`/`xsd:double`).
    Numeric,
    /// A plain string (`xsd:string`, no language tag).
    String,
}

impl ScalarvalKind {
    /// A stable diagnostic label — used by `registry_fingerprint` and prepare-time
    /// error messages, mirroring [`AlgebraicClass::label`].
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Numeric => "numeric",
            Self::String => "string",
        }
    }
}

/// One named scalar-value parameter a [`CustomAggregate`] declares accepting —
/// the `AGG(<iri>, …; NAME=value)` surface's per-aggregate contract (see
/// [`CustomAggregate::scalarvals`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScalarvalSpec {
    /// The canonical, upper-cased name — matched against the parser's own
    /// upper-cased `NAME` (see `purrdf_sparql_algebra`'s `parse_agg_scalarvals`
    /// docs), so a declaration should itself be written upper-case.
    pub name: &'static str,
    /// The value kind this parameter admits.
    pub kind: ScalarvalKind,
}

impl ScalarvalSpec {
    /// Declare a named scalarval parameter.
    #[must_use]
    pub const fn new(name: &'static str, kind: ScalarvalKind) -> Self {
        Self { name, kind }
    }
}

/// The per-invocation folding state a [`CustomAggregate`] hands out from
/// [`CustomAggregate::init`].
///
/// Object-safe and boxed: one instance per `GROUP BY` group (or, for an implicit
/// single-group aggregate with no `GROUP BY` at all, one instance for the query),
/// created by `init_contained`, driven by `step_contained`/`combine_contained`,
/// and consumed exactly once by `finish_contained`. Never shared across groups and
/// never read after `finish` — the accumulator is the aggregate's entire mutable
/// state, and the trait's three methods are the entire lifecycle.
///
/// `Send` (not `Sync`): an accumulator is driven by exactly one thread through its
/// whole lifetime the way this crate's evaluation currently folds a group (see the
/// module docs), but `combine` accepting a `Box<dyn AggregateAccumulator>` built by
/// ANOTHER thread's partial fold requires the type to be movable across threads —
/// `Send` is what that requires, without also claiming (falsely) that two threads
/// may drive the SAME accumulator concurrently.
///
/// `'static`: required for [`into_any`](Self::into_any)'s downcast —
/// [`std::any::Any`] requires it — and not a new restriction in practice: every
/// accumulator this crate has ever boxed holds only owned data, never a borrow,
/// so every existing and future implementor already satisfies it.
pub trait AggregateAccumulator: Send + 'static {
    /// Fold one row's already-evaluated, positional argument tuple into this
    /// accumulator's state.
    ///
    /// `args` is exactly the tuple [`CustomAggregate::arity`] admits — checked
    /// before evaluation ever reaches here (see `crate::property_fn_plan::plan_query`'s
    /// prepare-time walk) — and every position is **already bound**: a row with
    /// an unbound positional argument never reaches `step` at all, mirroring
    /// `crate::user_fn::eval_native_function`'s "no per-parameter optionality"
    /// contract for a native scalar function's arguments. An expression that
    /// raised an evaluation error is likewise never folded — the row is skipped
    /// upstream, exactly as a built-in aggregate's argument expression is.
    ///
    /// # Errors
    ///
    /// Any [`EvalError`] the aggregate raises to refuse an argument tuple it
    /// cannot fold. Per the hard-fail doctrine this aborts the query rather than
    /// silently dropping the row, which would be indistinguishable from an
    /// honest omission.
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError>;

    /// Merge `other`'s folded state into `self`, **in the order the caller
    /// presents accumulators** — i.e. `self` holds the earlier (in source/chunk
    /// order) partial fold and `other` the later one. See the module docs' note
    /// on [`AlgebraicClass`] for why this fixed order is what keeps a
    /// non-commutative fold deterministic under a parallel partial-fold split.
    ///
    /// `other.finish()` is enough to merge correctly when the aggregate's
    /// finished answer IS its whole mergeable state; when it is not, recover
    /// `other`'s concrete type via [`into_any`](Self::into_any) (see the module
    /// docs' "Merging structural state" section) and merge the real state
    /// `step` built instead.
    ///
    /// Called by `crate::modifier::eval_custom_aggregate`'s within-group chunked
    /// fold whenever the aggregate declares [`Volatility::Stable`] and the group
    /// is large enough to chunk (see `crate::parallel::par_chunk_reduce_init`) —
    /// part of the algebra from the start, so an aggregate registered before
    /// that caller existed was already correct under it, needing no later,
    /// breaking addition.
    ///
    /// # Errors
    ///
    /// Any [`EvalError`] the aggregate raises to refuse a merge — in particular,
    /// the typed refusal `downcast_combine_partial` returns when `other`'s
    /// concrete type does not match `Self`, which an implementation reaching for
    /// [`into_any`](Self::into_any)'s escape hatch should propagate with `?`
    /// rather than discard. `Result` here (not `()`) is what lets that mismatch —
    /// a host contract violation this crate cannot rule out at compile time, only
    /// by construction (see [`into_any`](Self::into_any)'s docs) — surface as an
    /// ordinary refusal on every target, including `wasm32-unknown-unknown` under
    /// `panic = "abort"`, where `crate::contain`'s panic containment cannot run
    /// at all: a `Result` return needs no unwind to report the violation, so this
    /// seam degrades to a clean error instead of aborting the whole module on
    /// that target.
    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError>;

    /// Recover this accumulator's original concrete type from behind the trait
    /// object — [`combine`](Self::combine)'s escape hatch for a merge that needs
    /// more than `finish()`'s single answer (see the module docs' "Merging
    /// structural state" section). `self: Box<Self>` (consuming, not `&mut
    /// self`) because `combine` always consumes `other` too, and
    /// `downcast_combine_partial` is the standard way to consume the result.
    ///
    /// No default body: a default here would need to type-check against a
    /// fully generic, possibly-unsized `Self`, which the `Box<Self> -> Box<dyn
    /// Any + Send>` unsizing coercion cannot do (it needs a concretely `Sized`
    /// `Self`, known only inside each `impl` block, not inside the trait
    /// definition). Every implementor's body is therefore the same one line —
    /// `self` — [`std::any::Any`]'s ordinary upcast; an aggregate that always
    /// merges through `finish()` (a `SUM`-alike, a list collector) never calls
    /// it but must still supply the line.
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send>;

    /// Produce this group's answer and consume the accumulator.
    ///
    /// `None` is an explicit unbound answer — the aggregate's own choice, and
    /// legitimate for any group including the empty one (an aggregate whose
    /// `finish(init())` is `None` is declaring "unbound over the empty group",
    /// exactly as the built-in `AVG`/`MIN`/`MAX`/`SAMPLE` do).
    ///
    /// # Errors
    ///
    /// Any [`EvalError`] the aggregate raises while producing its answer.
    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError>;
}

/// A host-injected custom aggregate: the registered FACTORY [`AggregateRegistry`]
/// resolves an `AGG(<iri>, …)` call's IRI to.
///
/// Object-safe and shared behind an [`Arc`] across the whole evaluation, hence
/// `Send + Sync`. Works entirely in dataset-independent [`TermValue`] space — an
/// aggregate never sees a dataset-local [`purrdf_core::TermId`] — so a registry
/// built once is valid against any dataset, exactly like [`crate::property_fn::PropertyFunction`].
pub trait CustomAggregate: Send + Sync {
    /// The declared positional arity of `AGG(<iri>, args…)`'s argument list.
    /// Checked against the call site's supplied expression count **before any
    /// host code runs** — at prepare time (see the module docs), the same
    /// fail-fast doctrine [`crate::property_fn::PfArity`] and
    /// [`Arity`] apply to their own seams.
    fn arity(&self) -> Arity;

    /// This aggregate's determinism class. Read by the fork-join per-GROUP
    /// parallel gate exactly as [`crate::user_fn::NativeFunction`]'s is: only
    /// [`Volatility::Stable`] may have its groups folded across workers, and an
    /// aggregate that misdeclares itself diverges silently under parallel
    /// evaluation.
    fn volatility(&self) -> Volatility;

    /// The algebraic law this aggregate's fold satisfies — see [`AlgebraicClass`].
    fn algebraic_class(&self) -> AlgebraicClass;

    /// A declared upper bound, in bytes, on the retained state ONE accumulator
    /// (one `GROUP BY` group) may grow to hold across its whole `step` sequence.
    ///
    /// Charged once per accumulator, at admission (see
    /// `crate::modifier::eval_custom_aggregate`), against the evaluator's
    /// scratch-arena resource ceiling (`ResourceDimension::ScratchBytes`) — the
    /// same dimension `crate::eval::EvalCtx::charge_scratch_growth` meters a
    /// built-in's minted values against. A custom accumulator's internal state is
    /// opaque host Rust the evaluator cannot observe mid-fold (unlike a minted
    /// [`TermValue`], which the scratch interner sees the instant it is
    /// produced), so this declared bound is what stands in for a live
    /// measurement: the honesty contract is the one
    /// [`purrdf_core::DatasetView::cardinality_estimate`] states for its own
    /// declared bound — an upper bound the aggregate actually respects, not a
    /// guess, because an under-statement here lets a query mint unbounded memory
    /// a ceiling was supposed to catch. A stateless accumulator (most `Commutative`
    /// folds: a running sum, a running count) declares `0`.
    fn state_bound(&self) -> u64;

    /// The named scalar-value parameters this aggregate accepts on the
    /// `AGG(<iri>, …; NAME=value)` surface (see
    /// [`purrdf_sparql_algebra::AggregateExpression::scalarvals`]'s docs).
    ///
    /// Checked at PREPARE time (`crate::property_fn_plan::plan_aggregate`),
    /// the same fail-fast doctrine [`Self::arity`] applies to its own seam: an
    /// unrecognized name, a duplicate, a missing REQUIRED name (every declared
    /// name is required — there is no optional-scalarval declaration, mirroring
    /// how every declared positional argument is required), or a supplied
    /// value whose [`ScalarvalKind`] does not match is refused before any
    /// governor charge.
    ///
    /// Default: none — most custom aggregates take no named scalar parameter
    /// at all, exactly as most take a fixed, purely positional arity.
    fn scalarvals(&self) -> &[ScalarvalSpec] {
        &[]
    }

    /// Begin one invocation — one fresh accumulator for one `GROUP BY` group (or
    /// for the query's single implicit group, when there is no `GROUP BY` at
    /// all).
    ///
    /// `scalarvals` is the call site's `; NAME=value` clauses, already resolved
    /// to [`TermValue`] and ALREADY VALIDATED against [`Self::scalarvals`]'s
    /// declaration at prepare time (name known, no duplicate, right kind, every
    /// declared name present) — never per-row, never re-evaluated: the SAME
    /// slice for every row this accumulator, and every accumulator a chunked
    /// fold creates alongside it, ever folds (see the module docs' "`combine`
    /// and determinism" section for why every partial accumulator in one
    /// `combine` chain shares one `init` factory call site). An aggregate that
    /// declares [`Self::scalarvals`] empty may ignore this parameter entirely;
    /// one that declares names reads them back by [`ScalarvalSpec::name`]
    /// (e.g. `scalarvals.iter().find(|(k, _)| k == "P")`).
    fn init(&self, scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator>;
}

// ---------------------------------------------------------------------------
// Panic containment
// ---------------------------------------------------------------------------

/// The fixed diagnostic label every containment message and the registry
/// fingerprint use to name this seam — the `kind` argument every
/// `crate::contain` call below supplies.
const KIND: &str = "custom aggregate";

/// Begin one invocation of `agg` with the host call contained — the
/// [`crate::property_fn::open_contained`] twin for this seam.
///
/// # Errors
///
/// [`EvalError::Function`] on a caught panic; otherwise `Ok` of the boxed
/// accumulator.
pub(crate) fn init_contained(
    agg: &dyn CustomAggregate,
    iri: &str,
    scalarvals: &[(String, TermValue)],
) -> Result<Box<dyn AggregateAccumulator>, EvalError> {
    crate::contain::declaration_contained(KIND, iri, "initial accumulator", || agg.init(scalarvals))
}

/// Fold one row's argument tuple into `accumulator` with the host call contained.
///
/// # Errors
///
/// [`EvalError::Function`] on a caught panic; otherwise the accumulator's own
/// error, propagated unchanged.
pub(crate) fn step_contained(
    accumulator: &mut dyn AggregateAccumulator,
    iri: &str,
    args: &[TermValue],
) -> Result<(), EvalError> {
    crate::contain::call_contained(KIND, iri, "folding a row", || accumulator.step(args))
}

/// Merge `other` into `accumulator` with the host call contained. See
/// [`AggregateAccumulator::combine`]'s docs for the chunk-order contract this
/// wraps rather than changes.
///
/// # Errors
///
/// [`EvalError::Function`] on a caught panic.
pub(crate) fn combine_contained(
    accumulator: &mut dyn AggregateAccumulator,
    iri: &str,
    other: Box<dyn AggregateAccumulator>,
) -> Result<(), EvalError> {
    crate::contain::call_contained(KIND, iri, "combining partial state", || {
        accumulator.combine(other)
    })
}

/// Recover `other`'s original concrete type — the standard way for an
/// [`AggregateAccumulator::combine`] implementation to consume
/// [`AggregateAccumulator::into_any`]'s escape hatch. See the module docs'
/// "Merging structural state" section for why the downcast is same-type by
/// construction; the `Err` this returns on a mismatch is a defense-in-depth
/// refusal for a case that should never arise, not an expected outcome.
///
/// # Errors
///
/// [`EvalError::Function`] if `other`'s concrete type is not `T` — never true
/// when `other` was created by the SAME [`CustomAggregate::init`] factory as the
/// accumulator calling this, which is the only way
/// `crate::modifier::eval_custom_aggregate`'s chunked fold ever calls `combine`.
/// A host mixing accumulator types is a bug, but — unlike the panic this used to
/// raise — reporting it is now a plain typed refusal an implementation propagates
/// with `?`, needing no [`catch_unwind`](std::panic::catch_unwind)-based
/// containment to keep it from aborting the call: the seam degrades the same way
/// on every target, `wasm32-unknown-unknown` under `panic = "abort"` included,
/// where [`crate::contain`]'s containment cannot run at all.
pub(crate) fn downcast_combine_partial<T: 'static>(
    other: Box<dyn AggregateAccumulator>,
) -> Result<T, EvalError> {
    other
        .into_any()
        .downcast::<T>()
        .map(|boxed| *boxed)
        .map_err(|_| {
            EvalError::function(
                "AggregateAccumulator::combine received a partial accumulator of a different \
             concrete type than Self — every partial combine merges was created by the SAME \
             CustomAggregate::init factory, so a mismatch here is a host bug in how partial \
             accumulators were constructed, not a state this crate's own evaluator can produce",
            )
        })
}

/// Consume `accumulator` and produce its group's answer with the host call
/// contained.
///
/// # Errors
///
/// [`EvalError::Function`] on a caught panic; otherwise the accumulator's own
/// error, propagated unchanged.
pub(crate) fn finish_contained(
    accumulator: Box<dyn AggregateAccumulator>,
    iri: &str,
) -> Result<Option<TermValue>, EvalError> {
    crate::contain::call_contained(KIND, iri, "finishing", || accumulator.finish())
}

/// Read one of `agg`'s DECLARATIONS (`arity`/`volatility`/`algebraic_class`/
/// `state_bound`) with the panic contained — the
/// [`crate::property_fn::declaration_contained`] twin for this seam.
///
/// Four dedicated entry points (one per declaration) rather than one generic
/// combinator taking a `fn(&dyn CustomAggregate) -> T` read: passing a trait
/// method path (`CustomAggregate::arity`) as a value fixes its implicit `&self`
/// lifetime to one concrete lifetime rather than the higher-ranked `for<'a>` a
/// generic combinator's `impl FnOnce(&dyn CustomAggregate) -> T` parameter
/// needs, so each wraps its own `|| agg.method()` closure instead.
///
/// # Errors
///
/// [`EvalError::Function`] on a caught panic.
pub(crate) fn arity_contained(agg: &dyn CustomAggregate, iri: &str) -> Result<Arity, EvalError> {
    crate::contain::declaration_contained(KIND, iri, "arity", || agg.arity())
}

/// [`arity_contained`]'s twin for [`CustomAggregate::volatility`].
///
/// # Errors
///
/// [`EvalError::Function`] on a caught panic.
pub(crate) fn volatility_contained(
    agg: &dyn CustomAggregate,
    iri: &str,
) -> Result<Volatility, EvalError> {
    crate::contain::declaration_contained(KIND, iri, "determinism class", || agg.volatility())
}

/// [`arity_contained`]'s twin for [`CustomAggregate::algebraic_class`].
///
/// # Errors
///
/// [`EvalError::Function`] on a caught panic.
pub(crate) fn algebraic_class_contained(
    agg: &dyn CustomAggregate,
    iri: &str,
) -> Result<AlgebraicClass, EvalError> {
    crate::contain::declaration_contained(KIND, iri, "algebraic class", || agg.algebraic_class())
}

/// Read `agg`'s declared per-accumulator [`CustomAggregate::state_bound`] with
/// the panic contained — the one declaration `crate::modifier::eval_custom_aggregate`
/// reads directly (to meter it), so it gets its own named entry point rather
/// than only being reachable through [`AggregateRegistry::describe`].
///
/// # Errors
///
/// [`EvalError::Function`] on a caught panic.
pub(crate) fn state_bound_contained(
    agg: &dyn CustomAggregate,
    iri: &str,
) -> Result<u64, EvalError> {
    crate::contain::declaration_contained(KIND, iri, "state bound", || agg.state_bound())
}

/// [`arity_contained`]'s twin for [`CustomAggregate::scalarvals`] — read by
/// `crate::property_fn_plan::plan_aggregate` (prepare-time scalarval
/// validation) and by [`AggregateRegistry::describe`]/`registry_fingerprint`.
///
/// # Errors
///
/// [`EvalError::Function`] on a caught panic.
pub(crate) fn scalarvals_contained(
    agg: &dyn CustomAggregate,
    iri: &str,
) -> Result<Vec<ScalarvalSpec>, EvalError> {
    crate::contain::declaration_contained(KIND, iri, "scalarval declaration", || {
        agg.scalarvals().to_vec()
    })
}

// ---------------------------------------------------------------------------
// The registry
// ---------------------------------------------------------------------------

/// A registered aggregate's self-description — the channel through which a
/// diagnostic or the plan cache's fingerprint reads what a registry actually
/// contains without holding a `dyn` reference to each aggregate. Mirrors
/// [`crate::property_fn::PfDescriptor`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggDescriptor {
    /// The IRI the aggregate is registered under, byte-exact.
    pub iri: String,
    /// The declared positional arity.
    pub arity: Arity,
    /// The declared determinism class.
    pub volatility: Volatility,
    /// The declared algebraic class.
    pub algebraic_class: AlgebraicClass,
    /// The declared per-accumulator state bound, in bytes.
    pub state_bound: u64,
    /// The declared named scalar-value parameters.
    pub scalarvals: Vec<ScalarvalSpec>,
}

/// A caller-injected table of custom aggregates, keyed by the `AGG(<iri>, …)`
/// IRI.
///
/// Built once per host configuration and borrowed into evaluation via
/// [`crate::eval::EvalCtx::with_aggregates`] or threaded through
/// [`crate::engine::QueryOptions::aggregates`]. Deterministic by construction:
/// the map is the crate's fixed-key deterministic hash map, and every ordered
/// surface ([`describe`](Self::describe), [`Debug`]) sorts by IRI rather than
/// reading iteration order — the exact shape [`crate::property_fn::PropertyFunctionRegistry`]
/// uses, for the same reasons.
///
/// # Registration is fail-fast on a duplicate
///
/// [`register`](Self::register) **panics** when an IRI is already registered —
/// mirroring [`crate::property_fn::PropertyFunctionRegistry::register`], not
/// [`crate::user_fn::UserFunctionRegistry`]'s "last write wins": a shadowed
/// aggregate silently changes a `GROUP BY` group's computed value under an IRI a
/// query already spelled one way, with no textual difference between the two
/// registrations to reveal it. That is a host misconfiguration, caught where it
/// is committed.
///
/// # Instance identity, not just declared contents
///
/// `id` is a `RegistryId` (`crate::registry_id::RegistryId`) minted fresh by
/// `Default`/[`new`](Self::new) — see that type's docs for why a counter, why a
/// counter is enough, and why [`Clone`] inherits rather than re-mints it. It
/// exists because DECLARED metadata (arity, volatility, algebraic class, state
/// bound) cannot distinguish two independently built registries that happen to
/// register the SAME IRI to two DIFFERENT [`CustomAggregate`] implementations
/// with identical declarations — `registry_fingerprint` folds this id in ahead
/// of the declaration digest so those two registries can never be mistaken for
/// each other by a prepared plan's identity.
#[derive(Default, Clone)]
pub struct AggregateRegistry {
    id: crate::registry_id::RegistryId,
    aggregates: DetHashMap<String, Arc<dyn CustomAggregate>>,
}

impl core::fmt::Debug for AggregateRegistry {
    /// A `dyn CustomAggregate` has no `Debug` impl, so this lists the registered
    /// IRIs, sorted for deterministic output, rather than deriving. The instance
    /// id rides along too, since it is exactly the thing that can make two
    /// otherwise-identical-looking registries diagnostically distinguishable.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut iris: Vec<&str> = self.aggregates.keys().map(String::as_str).collect();
        iris.sort_unstable();
        f.debug_struct("AggregateRegistry")
            .field("id", &self.id)
            .field("aggregates", &iris)
            .finish()
    }
}

impl AggregateRegistry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The canonical empty registry — the non-optional "no aggregates
    /// registered" value every registry-carrying seam
    /// ([`crate::engine::QueryOptions::aggregates`],
    /// [`crate::eval::EvalCtx::aggregates`](crate::eval::EvalCtx),
    /// `crate::parallel::SafetyRegistries::aggregates`) now uses in place of the
    /// old `Option::None` spelling, so "no registry" and "an empty registry" are
    /// the same value rather than two spellings of one state.
    ///
    /// A `const`, not merely a fresh [`Self::new`] call per use: every one of
    /// those seams can borrow the SAME `'static` value, and — see
    /// `RegistryId::EMPTY`'s (`crate::registry_id::RegistryId::EMPTY`) docs —
    /// sharing one fixed instance id across every `EMPTY` reference is the
    /// correct semantics here, not a weakening of the plan-identity guard: an
    /// empty registry can never resolve any IRI, so no plan's admitted behavior
    /// can depend on WHICH empty registry it was prepared against.
    pub const EMPTY: Self = Self {
        id: crate::registry_id::RegistryId::EMPTY,
        aggregates: DetHashMap::with_hasher(crate::DetHasher::new()),
    };

    /// Register `aggregate` under `iri`.
    ///
    /// # Panics
    ///
    /// Panics if `iri` is already registered — see the type's docs for why an
    /// aggregate may not be silently shadowed.
    pub fn register(&mut self, iri: impl Into<String>, aggregate: Arc<dyn CustomAggregate>) {
        let iri = iri.into();
        assert!(
            !self.aggregates.contains_key(&iri),
            "IRI <{iri}> is already registered as a custom aggregate; an aggregate may not be \
             silently shadowed, because both spellings of the call are identical and the only \
             observable difference is the value a GROUP BY group computes"
        );
        self.aggregates.insert(iri, aggregate);
    }

    /// Resolve an `AGG(<iri>, …)` IRI to its registered aggregate, if any.
    #[must_use]
    pub fn resolve(&self, iri: &str) -> Option<&Arc<dyn CustomAggregate>> {
        self.aggregates.get(iri)
    }

    /// Whether the registry holds no aggregates — the common case, in which
    /// evaluation carries no registry at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.aggregates.is_empty()
    }

    /// The number of registered aggregates.
    #[must_use]
    pub fn len(&self) -> usize {
        self.aggregates.len()
    }

    /// Describe every registered aggregate, sorted by IRI — see the type's docs
    /// for why the sort makes this a pure function of contents, not construction
    /// order.
    ///
    /// Every declaration read goes through its dedicated `*_contained` entry
    /// point, because this feeds the plan cache's fingerprint (see
    /// `registry_fingerprint`) — an aggregate whose declaration methods panic
    /// must fail that prepare cleanly, not abort it.
    ///
    /// # Errors
    ///
    /// [`EvalError::Function`] if any registered aggregate's declaration methods
    /// panic.
    pub fn describe(&self) -> Result<Vec<AggDescriptor>, EvalError> {
        let mut out: Vec<AggDescriptor> = Vec::with_capacity(self.aggregates.len());
        for (iri, aggregate) in &self.aggregates {
            let aggregate = aggregate.as_ref();
            out.push(AggDescriptor {
                iri: iri.clone(),
                arity: arity_contained(aggregate, iri)?,
                volatility: volatility_contained(aggregate, iri)?,
                algebraic_class: algebraic_class_contained(aggregate, iri)?,
                state_bound: state_bound_contained(aggregate, iri)?,
                scalarvals: scalarvals_contained(aggregate, iri)?,
            });
        }
        out.sort_by(|a, b| a.iri.cmp(&b.iri));
        Ok(out)
    }
}

/// A deterministic fingerprint of everything about `aggregates` that can change
/// either the prepare-time admission of an `AGG(<iri>, …)` call OR the
/// answer/parallel-safety of the execution that runs it: the registry's own
/// [`RegistryId`](crate::registry_id::RegistryId), followed by every registered
/// IRI's declared arity, volatility, algebraic class, and state bound, IRI-sorted.
///
/// # The instance id comes first, and is load-bearing
///
/// Declared metadata alone cannot tell two registries apart when they happen to
/// agree on every declaration for a shared IRI while resolving it to two
/// DIFFERENT [`CustomAggregate`] implementations — a SUM and a PRODUCT can both
/// declare `Arity::Exact(1)`, [`Volatility::Stable`], [`AlgebraicClass::Commutative`],
/// and `state_bound() == 0`, and compute entirely different answers. The
/// [`RegistryId`](crate::registry_id::RegistryId) each registry mints at
/// construction closes that hole: two registries can never share a fingerprint
/// unless they are the SAME instance (or a [`Clone`] of it, which shares the
/// identical `Arc<dyn CustomAggregate>` implementations — see that type's docs),
/// regardless of how identical their declarations read.
///
/// The exact twin of `crate::property_fn_plan::registry_fingerprint`, for the
/// exact same reason: it belongs in the plan cache's key and in the identity a
/// prepared plan is validated against before evaluation (see
/// `crate::engine::check_plan_matches_relations`), because two differently
/// configured — or merely differently CONSTRUCTED — registries can admit, or
/// evaluate, the same query text differently, and a plan admitted under one must
/// never silently run under another.
///
/// # Errors
///
/// [`EvalError::Function`] if a registered aggregate's declaration methods panic
/// — [`AggregateRegistry::describe`]'s own failure, propagated unchanged. Never
/// raised when `aggregates` is empty (which [`AggregateRegistry::EMPTY`] — the
/// canonical "no registry" value — always is).
pub(crate) fn registry_fingerprint(aggregates: &AggregateRegistry) -> Result<String, EvalError> {
    if aggregates.is_empty() {
        return Ok(String::new());
    }
    let registry = aggregates;
    let mut out = String::new();
    out.push_str(&registry.id.stable_encoding().to_string());
    out.push('\u{5}');
    for descriptor in registry.describe()? {
        out.push_str(&descriptor.iri);
        out.push('\u{2}');
        out.push_str(&descriptor.arity.stable_encoding());
        out.push('\u{2}');
        out.push_str(descriptor.volatility.label());
        out.push('\u{2}');
        out.push_str(descriptor.algebraic_class.label());
        out.push('\u{2}');
        out.push_str(&descriptor.state_bound.to_string());
        out.push('\u{2}');
        // Declared scalarvals affect prepare-time admission (an unrecognized or
        // wrong-typed `; NAME=value` is refused there) exactly as arity does, so
        // they must fold into the fingerprint too: two registries that declare
        // the SAME IRI with different accepted scalarval names/kinds must not
        // share a fingerprint, or a plan admitted under one's (looser or
        // stricter) declaration could be silently reused under the other's.
        for spec in &descriptor.scalarvals {
            out.push_str(spec.name);
            out.push('\u{3}');
            out.push_str(spec.kind.label());
            out.push('\u{3}');
        }
        out.push('\u{4}');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EX_SUM: &str = "http://example.org/ns#customSum";
    const EX_OTHER: &str = "http://example.org/ns#customOther";
    const EX_PANIC: &str = "http://example.org/ns#customPanic";

    /// A simple `Commutative` custom `SUM`-alike over a single numeric-lexical
    /// argument: folds the integer lexical forms it is handed, ignoring anything
    /// that does not parse. Exercises the ordinary success path end to end.
    struct SumAccumulator {
        total: i64,
    }

    impl AggregateAccumulator for SumAccumulator {
        fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
            if let Some(TermValue::Literal { lexical_form, .. }) = args.first()
                && let Ok(n) = lexical_form.parse::<i64>()
            {
                self.total += n;
            }
            Ok(())
        }

        fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
            // A running total IS its own sufficient merge state, so this
            // fixture does not need `into_any`'s downcast escape hatch: merge
            // through the same public surface a caller has — finish the other
            // partial and fold its value back in.
            if let Some(TermValue::Literal { lexical_form, .. }) = other.finish()?
                && let Ok(n) = lexical_form.parse::<i64>()
            {
                self.total += n;
            }
            Ok(())
        }

        /// Unused (this fixture merges through `finish()`) — see
        /// [`AggregateAccumulator::into_any`]'s trait docs for why every
        /// implementor still supplies the one-line body.
        fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
            self
        }

        fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
            Ok(Some(TermValue::typed_literal(
                self.total.to_string(),
                "http://www.w3.org/2001/XMLSchema#integer",
            )))
        }
    }

    struct SumAggregate;

    impl CustomAggregate for SumAggregate {
        fn arity(&self) -> Arity {
            Arity::Exact(1)
        }
        fn volatility(&self) -> Volatility {
            Volatility::Stable
        }
        fn algebraic_class(&self) -> AlgebraicClass {
            AlgebraicClass::Commutative
        }
        fn state_bound(&self) -> u64 {
            0
        }
        fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
            Box::new(SumAccumulator { total: 0 })
        }
    }

    fn int(n: i64) -> TermValue {
        TermValue::typed_literal(n.to_string(), "http://www.w3.org/2001/XMLSchema#integer")
    }

    // ---- registry ----------------------------------------------------------

    #[test]
    fn register_and_resolve() {
        let mut registry = AggregateRegistry::new();
        assert!(registry.is_empty());
        registry.register(EX_SUM, Arc::new(SumAggregate));
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
        assert!(registry.resolve(EX_SUM).is_some());
        assert!(registry.resolve(EX_OTHER).is_none());
    }

    #[test]
    #[should_panic(expected = "already registered as a custom aggregate")]
    fn duplicate_registration_panics() {
        let mut registry = AggregateRegistry::new();
        registry.register(EX_SUM, Arc::new(SumAggregate));
        registry.register(EX_SUM, Arc::new(SumAggregate));
    }

    #[test]
    fn describe_is_sorted_by_iri_and_reports_declarations() {
        let mut registry = AggregateRegistry::new();
        registry.register(EX_SUM, Arc::new(SumAggregate));
        registry.register(EX_OTHER, Arc::new(SumAggregate));
        let described = registry.describe().expect("no aggregate panics");
        assert_eq!(
            described.iter().map(|d| d.iri.as_str()).collect::<Vec<_>>(),
            vec![EX_OTHER, EX_SUM],
            "describe sorts by IRI, not by registration order"
        );
        let first = &described[0];
        assert_eq!(first.arity, Arity::Exact(1));
        assert_eq!(first.volatility, Volatility::Stable);
        assert_eq!(first.algebraic_class, AlgebraicClass::Commutative);
        assert_eq!(first.state_bound, 0);
    }

    #[test]
    fn debug_lists_iris_sorted() {
        let mut registry = AggregateRegistry::new();
        registry.register(EX_SUM, Arc::new(SumAggregate));
        registry.register(EX_OTHER, Arc::new(SumAggregate));
        let rendered = format!("{registry:?}");
        let other_at = rendered.find(EX_OTHER).expect("other listed");
        let sum_at = rendered.find(EX_SUM).expect("sum listed");
        assert!(other_at < sum_at, "Debug output is IRI-sorted: {rendered}");
    }

    /// [`AggregateRegistry::EMPTY`] (the canonical "no registry" value every
    /// registry-carrying seam now uses) and a freshly constructed, still-empty
    /// [`AggregateRegistry::new`] must produce the IDENTICAL "" fingerprint —
    /// they are two different registry INSTANCES (different underlying maps),
    /// yet both resolve every IRI to `None`, so a plan's admitted behavior can
    /// never depend on which one it was prepared against. This makes a
    /// `None`-shaped call and a `Some(&AggregateRegistry::new())`-shaped call
    /// disagreeing about behavior structurally impossible rather than merely
    /// tested: there is no `Option` left to spell two ways in the first place —
    /// see [`RegistryId::EMPTY`](crate::registry_id::RegistryId::EMPTY)'s
    /// docs for the full reasoning on why this is the CORRECT identity behavior,
    /// not a weakening of the plan-identity guard the two later tests below pin.
    #[test]
    fn empty_const_and_a_freshly_built_empty_registry_share_the_same_fingerprint() {
        assert_eq!(
            registry_fingerprint(&AggregateRegistry::EMPTY).expect("ok"),
            ""
        );
        let empty = AggregateRegistry::new();
        assert_eq!(registry_fingerprint(&empty).expect("ok"), "");

        let mut registry = AggregateRegistry::new();
        registry.register(EX_SUM, Arc::new(SumAggregate));
        let first = registry_fingerprint(&registry).expect("ok");
        let second = registry_fingerprint(&registry).expect("ok");
        assert_eq!(
            first, second,
            "the fingerprint is a pure function of contents"
        );
        assert!(!first.is_empty());
    }

    /// GAP (registry instance identity): two INDEPENDENTLY constructed registries
    /// that register the SAME IRI to the SAME declared metadata (arity, volatility,
    /// algebraic class, state bound) — byte-identical `describe()` output — must
    /// still produce DIFFERENT fingerprints, because nothing about identical
    /// declarations proves the two registries resolve the IRI to the same
    /// implementation. Without the instance id this fingerprint folds in, these two
    /// registries would be indistinguishable, and a plan prepared under one would
    /// silently be accepted for evaluation under the other.
    #[test]
    fn two_independently_built_registries_with_identical_declarations_still_differ() {
        let mut a = AggregateRegistry::new();
        a.register(EX_SUM, Arc::new(SumAggregate));
        let mut b = AggregateRegistry::new();
        b.register(EX_SUM, Arc::new(SumAggregate));

        // The declared content is byte-identical...
        assert_eq!(a.describe().expect("ok"), b.describe().expect("ok"));
        // ...yet the fingerprint — which is what a prepared plan's identity is
        // checked against — must still differ, because `a` and `b` are two
        // different registry instances.
        assert_ne!(
            registry_fingerprint(&a).expect("ok"),
            registry_fingerprint(&b).expect("ok"),
            "two independently constructed registries must never share a fingerprint, \
             even when every declaration they report is identical"
        );
    }

    /// The flip side: a [`Clone`] of a registry shares the SAME `Arc<dyn
    /// CustomAggregate>` implementations as its source, so it must produce the
    /// SAME fingerprint — a caller that clones a registry to move it into an
    /// `Arc`/closure must not have every previously-prepared plan spuriously
    /// invalidated.
    #[test]
    fn a_clone_shares_its_source_registrys_fingerprint() {
        let mut registry = AggregateRegistry::new();
        registry.register(EX_SUM, Arc::new(SumAggregate));
        let cloned = registry.clone();
        assert_eq!(
            registry_fingerprint(&registry).expect("ok"),
            registry_fingerprint(&cloned).expect("ok"),
            "a clone shares the source's actual implementations, so it is the same \
             registry instance for fingerprint purposes"
        );
    }

    // ---- end-to-end fold ----------------------------------------------------

    #[test]
    fn sum_accumulator_folds_and_finishes() {
        let aggregate = SumAggregate;
        let mut accumulator = init_contained(&aggregate, EX_SUM, &[]).expect("init");
        step_contained(accumulator.as_mut(), EX_SUM, &[int(2)]).expect("step");
        step_contained(accumulator.as_mut(), EX_SUM, &[int(40)]).expect("step");
        let value = finish_contained(accumulator, EX_SUM).expect("finish");
        assert_eq!(value, Some(int(42)));
    }

    #[test]
    fn empty_group_still_answers_explicitly() {
        // No `step` at all — `finish(init())` — and the aggregate still answers,
        // rather than the evaluator inventing a default.
        let aggregate = SumAggregate;
        let accumulator = init_contained(&aggregate, EX_SUM, &[]).expect("init");
        let value = finish_contained(accumulator, EX_SUM).expect("finish");
        assert_eq!(value, Some(int(0)));
    }

    #[test]
    fn combine_merges_in_the_order_given() {
        let aggregate = SumAggregate;
        let mut a = init_contained(&aggregate, EX_SUM, &[]).expect("init");
        step_contained(a.as_mut(), EX_SUM, &[int(10)]).expect("step");
        let mut b = init_contained(&aggregate, EX_SUM, &[]).expect("init");
        step_contained(b.as_mut(), EX_SUM, &[int(32)]).expect("step");
        combine_contained(a.as_mut(), EX_SUM, b).expect("combine");
        let value = finish_contained(a, EX_SUM).expect("finish");
        assert_eq!(value, Some(int(42)));
    }

    // ---- a mismatched `into_any` downcast is a typed refusal, never a panic ----

    /// An accumulator that merges through [`AggregateAccumulator::into_any`]'s
    /// downcast escape hatch — deliberately, unlike [`SumAccumulator`] above,
    /// which merges through `finish()` instead — so that handing its `combine` a
    /// partial of some OTHER concrete type exercises `downcast_combine_partial`'s
    /// mismatch path exactly as `crate::stat_agg`'s real members would.
    struct DowncastingAccumulator {
        total: i64,
    }

    impl AggregateAccumulator for DowncastingAccumulator {
        fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
            if let Some(TermValue::Literal { lexical_form, .. }) = args.first()
                && let Ok(n) = lexical_form.parse::<i64>()
            {
                self.total += n;
            }
            Ok(())
        }

        fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
            let other = downcast_combine_partial::<Self>(other)?;
            self.total += other.total;
            Ok(())
        }

        fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
            self
        }

        fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
            Ok(Some(int(self.total)))
        }
    }

    /// Regression coverage: `combine` receiving a partial built by a DIFFERENT
    /// `CustomAggregate::init` factory than `self` — a host contract violation
    /// (see [`downcast_combine_partial`]'s docs for why this cannot arise from
    /// this crate's own evaluator) — must produce a typed [`EvalError`], not a
    /// panic. Previously [`downcast_combine_partial`] could only report this by
    /// panicking, relying on [`combine_contained`]'s `catch_unwind` containment;
    /// containment is unavailable under `panic = "abort"`
    /// (`wasm32-unknown-unknown`), so a `Result`-returning `combine` is what
    /// makes this refusal reach every target, not just the ones where unwinding
    /// works. `without_panic_output` is kept around this call as a belt-and-braces
    /// check: if this ever regresses back to a panic, the test would still pass
    /// via `combine_contained`'s containment, but the panic-hook assertion below
    /// would fail, catching the regression either way.
    #[test]
    fn a_mismatched_downcast_in_combine_is_a_typed_error_not_a_panic() {
        let mut a: Box<dyn AggregateAccumulator> = Box::new(DowncastingAccumulator { total: 0 });
        step_contained(a.as_mut(), EX_SUM, &[int(41)]).expect("step");

        // `b` is a DIFFERENT concrete accumulator type than `a` — the mismatch
        // `downcast_combine_partial::<DowncastingAccumulator>` inside `a.combine`
        // must refuse.
        let aggregate = SumAggregate;
        let b = init_contained(&aggregate, EX_SUM, &[]).expect("init");

        let error = without_panic_output(|| {
            combine_contained(a.as_mut(), EX_SUM, b)
                .expect_err("a mismatched downcast must be a typed refusal, not a silent merge")
        });
        assert!(
            error
                .to_string()
                .contains("partial accumulator of a different concrete type"),
            "got {error}"
        );
        // Distinguishes the typed refusal from `combine_contained`'s own
        // catch_unwind-containment message — this must be `downcast_combine_partial`'s
        // OWN `Err`, reached without ever unwinding.
        assert!(
            !error.to_string().contains("panicked while combining"),
            "the mismatch must be reported as an ordinary Err, not a caught panic: got {error}"
        );

        // The accumulator's own state is left untouched by the failed combine —
        // a refused merge does not silently apply a partial mutation.
        let value = finish_contained(a, EX_SUM).expect("finish");
        assert_eq!(value, Some(int(41)));
    }

    // ---- panic containment ---------------------------------------------------

    struct PanickingAccumulator {
        panic_on_step: bool,
        panic_on_finish: bool,
    }

    impl AggregateAccumulator for PanickingAccumulator {
        fn step(&mut self, _args: &[TermValue]) -> Result<(), EvalError> {
            assert!(!self.panic_on_step, "accumulator exploded in step");
            Ok(())
        }
        fn combine(&mut self, _other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
            panic!("accumulator exploded in combine")
        }
        fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
            self
        }
        fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
            assert!(!self.panic_on_finish, "accumulator exploded in finish");
            Ok(None)
        }
    }

    struct PanickingAggregate {
        panic_on_init: bool,
        panic_on_arity: bool,
    }

    impl CustomAggregate for PanickingAggregate {
        fn arity(&self) -> Arity {
            assert!(!self.panic_on_arity, "aggregate exploded reporting arity");
            Arity::Exact(1)
        }
        fn volatility(&self) -> Volatility {
            Volatility::Stable
        }
        fn algebraic_class(&self) -> AlgebraicClass {
            AlgebraicClass::Commutative
        }
        fn state_bound(&self) -> u64 {
            0
        }
        fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
            assert!(!self.panic_on_init, "aggregate exploded in init");
            Box::new(PanickingAccumulator {
                panic_on_step: true,
                panic_on_finish: true,
            })
        }
    }

    /// Run `body` with the default panic hook suppressed, so an *expected*, caught
    /// panic does not dump to stderr (mirrors every other seam's test helper).
    fn without_panic_output<R>(body: impl FnOnce() -> R) -> R {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let out = body();
        std::panic::set_hook(default_hook);
        out
    }

    #[test]
    fn a_panic_in_init_is_a_clean_payload_free_error() {
        let aggregate = PanickingAggregate {
            panic_on_init: true,
            panic_on_arity: false,
        };
        let error = without_panic_output(|| {
            // `Box<dyn AggregateAccumulator>` has no `Debug` impl, so `expect_err`
            // (which would need to format the `Ok` side) is not usable here.
            let Err(error) = init_contained(&aggregate, EX_PANIC, &[]) else {
                panic!("a panicking init must not escape");
            };
            error
        });
        assert!(
            error
                .to_string()
                .contains("panicked while reporting its initial accumulator"),
            "got {error}"
        );
        assert!(!error.to_string().contains("exploded"), "got {error}");
    }

    #[test]
    fn a_panic_in_step_is_a_clean_payload_free_error() {
        let aggregate = PanickingAggregate {
            panic_on_init: false,
            panic_on_arity: false,
        };
        let mut accumulator = init_contained(&aggregate, EX_PANIC, &[]).expect("init");
        let error = without_panic_output(|| {
            step_contained(accumulator.as_mut(), EX_PANIC, &[int(1)])
                .expect_err("a panicking step must not escape")
        });
        assert!(
            error.to_string().contains("panicked while folding a row"),
            "got {error}"
        );
        assert!(!error.to_string().contains("exploded"), "got {error}");
    }

    #[test]
    fn a_panic_in_combine_is_a_clean_payload_free_error() {
        let aggregate = PanickingAggregate {
            panic_on_init: false,
            panic_on_arity: false,
        };
        let mut a = init_contained(&aggregate, EX_PANIC, &[]).expect("init");
        let b = init_contained(&aggregate, EX_PANIC, &[]).expect("init");
        let error = without_panic_output(|| {
            combine_contained(a.as_mut(), EX_PANIC, b)
                .expect_err("a panicking combine must not escape")
        });
        assert!(
            error
                .to_string()
                .contains("panicked while combining partial state"),
            "got {error}"
        );
        assert!(!error.to_string().contains("exploded"), "got {error}");
    }

    #[test]
    fn a_panic_in_finish_is_a_clean_payload_free_error() {
        let aggregate = PanickingAggregate {
            panic_on_init: false,
            panic_on_arity: false,
        };
        let accumulator = init_contained(&aggregate, EX_PANIC, &[]).expect("init");
        let error = without_panic_output(|| {
            finish_contained(accumulator, EX_PANIC).expect_err("a panicking finish must not escape")
        });
        assert!(
            error.to_string().contains("panicked while finishing"),
            "got {error}"
        );
        assert!(!error.to_string().contains("exploded"), "got {error}");
    }

    #[test]
    fn a_panic_in_a_declaration_read_is_a_clean_payload_free_error() {
        let aggregate = PanickingAggregate {
            panic_on_init: false,
            panic_on_arity: true,
        };
        let error = without_panic_output(|| {
            arity_contained(&aggregate, EX_PANIC)
                .expect_err("a panicking arity read must not escape")
        });
        assert!(
            error
                .to_string()
                .contains("panicked while reporting its arity"),
            "got {error}"
        );
        assert!(!error.to_string().contains("exploded"), "got {error}");
    }

    #[test]
    fn describe_contains_a_panicking_aggregates_declaration() {
        let mut registry = AggregateRegistry::new();
        registry.register(
            EX_PANIC,
            Arc::new(PanickingAggregate {
                panic_on_init: false,
                panic_on_arity: true,
            }),
        );
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
    }
}
