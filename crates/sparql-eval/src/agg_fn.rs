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
//! # The fold shape, shared with the built-ins
//!
//! Every aggregate — built-in or custom — is `init` (an aggregate's answer over the
//! empty group, `finish(init())`, is never absent: `COUNT` → `0`, `SUM` → `0`,
//! `AVG`/`MIN`/`MAX`/`SAMPLE` → unbound, `GROUP_CONCAT` → `""`), `step` (fold one
//! row's already-evaluated argument tuple in), and `finish` (produce the group's
//! answer, or `None` for unbound). The built-ins instantiate this shape internally,
//! with static dispatch and no per-row boxing (see `crate::modifier::BuiltinFold`);
//! this module is where a HOST instantiates it dynamically, through
//! [`CustomAggregate`]/[`AggregateAccumulator`] and an
//! [`AggregateRegistry`] entry.
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
//! about it; the evaluator itself does not need the classification to fold a single
//! group sequentially, which is the only shape this crate's evaluation currently
//! drives (parallel WITHIN one group's fold is a later increment — see
//! `crate::parallel`'s per-GROUP, not per-ROW, fork gate for aggregates).
//!
//! # wasm32 note
//!
//! See `crate::contain`'s module docs: `catch_unwind` requires
//! `panic = "unwind"`, and this seam's containment is unavailable under
//! `panic = "abort"` exactly as every other extension seam's is.

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
pub trait AggregateAccumulator: Send {
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
    /// Not exercised by this crate's evaluation yet — folding one `GROUP BY`
    /// group's rows across more than one worker is a later increment — but part
    /// of the algebra from the start, so an aggregate registered today is already
    /// correct under it rather than needing a later, breaking addition.
    fn combine(&mut self, other: Box<dyn AggregateAccumulator>);

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

    /// Begin one invocation — one fresh accumulator for one `GROUP BY` group (or
    /// for the query's single implicit group, when there is no `GROUP BY` at
    /// all).
    fn init(&self) -> Box<dyn AggregateAccumulator>;
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
) -> Result<Box<dyn AggregateAccumulator>, EvalError> {
    crate::contain::declaration_contained(KIND, iri, "initial accumulator", || agg.init())
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
#[allow(
    dead_code,
    reason = "the fourth member of the init/step/combine/finish algebra — part of this \
              module's trust boundary from the start, exercised directly by this module's own \
              tests, and the entry point within-group parallel partial folds (a later \
              increment) will call; not yet invoked by `crate::modifier`, which currently \
              folds each group sequentially on a single worker (see `crate::agg_fn`'s module \
              docs)"
)]
pub(crate) fn combine_contained(
    accumulator: &mut dyn AggregateAccumulator,
    iri: &str,
    other: Box<dyn AggregateAccumulator>,
) -> Result<(), EvalError> {
    crate::contain::call_contained(KIND, iri, "combining partial state", || {
        accumulator.combine(other);
        Ok(())
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
#[derive(Default, Clone)]
pub struct AggregateRegistry {
    aggregates: DetHashMap<String, Arc<dyn CustomAggregate>>,
}

impl core::fmt::Debug for AggregateRegistry {
    /// A `dyn CustomAggregate` has no `Debug` impl, so this lists the registered
    /// IRIs, sorted for deterministic output, rather than deriving.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut iris: Vec<&str> = self.aggregates.keys().map(String::as_str).collect();
        iris.sort_unstable();
        f.debug_struct("AggregateRegistry")
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
            });
        }
        out.sort_by(|a, b| a.iri.cmp(&b.iri));
        Ok(out)
    }
}

/// A deterministic fingerprint of everything about `aggregates` that can change
/// either the prepare-time admission of an `AGG(<iri>, …)` call OR the
/// answer/parallel-safety of the execution that runs it: every registered IRI,
/// its declared arity, volatility, algebraic class, and state bound, IRI-sorted.
///
/// The exact twin of `crate::property_fn_plan::registry_fingerprint`, for the
/// exact same reason: it belongs in the plan cache's key and in the identity a
/// prepared plan is validated against before evaluation (see
/// `crate::engine::check_plan_matches_relations`), because two differently
/// configured registries can admit — or evaluate — the same query text
/// differently, and a plan admitted under one must never silently run under
/// another.
///
/// # Errors
///
/// [`EvalError::Function`] if a registered aggregate's declaration methods panic
/// — [`AggregateRegistry::describe`]'s own failure, propagated unchanged. Never
/// raised when `aggregates` is `None` or empty.
pub(crate) fn registry_fingerprint(
    aggregates: Option<&AggregateRegistry>,
) -> Result<String, EvalError> {
    let Some(registry) = aggregates.filter(|registry| !registry.is_empty()) else {
        return Ok(String::new());
    };
    let mut out = String::new();
    for descriptor in registry.describe()? {
        out.push_str(&descriptor.iri);
        out.push('\u{2}');
        out.push_str(&descriptor.arity.to_string());
        out.push('\u{2}');
        out.push_str(descriptor.volatility.label());
        out.push('\u{2}');
        out.push_str(descriptor.algebraic_class.label());
        out.push('\u{2}');
        out.push_str(&descriptor.state_bound.to_string());
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

        fn combine(&mut self, other: Box<dyn AggregateAccumulator>) {
            // No downcasting seam is required of the trait, so a test/host
            // combine merges through the same public surface a caller has:
            // finish the other partial and fold its value back in.
            if let Ok(Some(TermValue::Literal { lexical_form, .. })) = other.finish()
                && let Ok(n) = lexical_form.parse::<i64>()
            {
                self.total += n;
            }
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
        fn init(&self) -> Box<dyn AggregateAccumulator> {
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

    #[test]
    fn registry_fingerprint_is_empty_for_no_registry_and_stable_for_one() {
        assert_eq!(registry_fingerprint(None).expect("ok"), "");
        let empty = AggregateRegistry::new();
        assert_eq!(registry_fingerprint(Some(&empty)).expect("ok"), "");

        let mut registry = AggregateRegistry::new();
        registry.register(EX_SUM, Arc::new(SumAggregate));
        let first = registry_fingerprint(Some(&registry)).expect("ok");
        let second = registry_fingerprint(Some(&registry)).expect("ok");
        assert_eq!(
            first, second,
            "the fingerprint is a pure function of contents"
        );
        assert!(!first.is_empty());
    }

    // ---- end-to-end fold ----------------------------------------------------

    #[test]
    fn sum_accumulator_folds_and_finishes() {
        let aggregate = SumAggregate;
        let mut accumulator = init_contained(&aggregate, EX_SUM).expect("init");
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
        let accumulator = init_contained(&aggregate, EX_SUM).expect("init");
        let value = finish_contained(accumulator, EX_SUM).expect("finish");
        assert_eq!(value, Some(int(0)));
    }

    #[test]
    fn combine_merges_in_the_order_given() {
        let aggregate = SumAggregate;
        let mut a = init_contained(&aggregate, EX_SUM).expect("init");
        step_contained(a.as_mut(), EX_SUM, &[int(10)]).expect("step");
        let mut b = init_contained(&aggregate, EX_SUM).expect("init");
        step_contained(b.as_mut(), EX_SUM, &[int(32)]).expect("step");
        combine_contained(a.as_mut(), EX_SUM, b).expect("combine");
        let value = finish_contained(a, EX_SUM).expect("finish");
        assert_eq!(value, Some(int(42)));
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
        fn combine(&mut self, _other: Box<dyn AggregateAccumulator>) {
            panic!("accumulator exploded in combine")
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
        fn init(&self) -> Box<dyn AggregateAccumulator> {
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
            let Err(error) = init_contained(&aggregate, EX_PANIC) else {
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
        let mut accumulator = init_contained(&aggregate, EX_PANIC).expect("init");
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
        let mut a = init_contained(&aggregate, EX_PANIC).expect("init");
        let b = init_contained(&aggregate, EX_PANIC).expect("init");
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
        let accumulator = init_contained(&aggregate, EX_PANIC).expect("init");
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
