// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic two-phase parallel evaluation primitives.
//!
//! [`crate::bgp`]'s per-batch evaluation, `binop`'s `Join`/`LeftJoin`/`MINUS`/
//! `Union`, `expr::eval_filter`/`eval_extend`, `binop::left_outer_join_filtered`,
//! and `modifier::eval_group`'s per-group aggregates are all wired to the
//! fork-join model below. The two phases, always in this order:
//!
//! 1. **Fork.** [`crate::eval::EvalCtx::fork_for_worker`] gives each worker a
//!    `Send` child context with its own scratch/constructed state, so workers
//!    never contend on a lock or share mutable evaluation state.
//! 2. **Join.** [`par_chunk_try_map_init`]/[`par_chunk_map`]/[`par_retain`] run
//!    the workers via rayon's *indexed* `par_chunks`/`par_iter` (never
//!    `par_sort`/`par_bridge`, which are not order-stable) and then reduce
//!    strictly in source-index order: successes concatenate in chunk (hence
//!    source) order and the first `Err` **by chunk index** wins, regardless of
//!    which worker finished first.
//!
//! A read-only FILTER predicate discards its child's scratch mints entirely (the
//! surviving rows are the original rows, nothing new escapes). A **minting** node
//! — `UNION`, per-group aggregates, `BIND`/`Extend` — is different: its output
//! rows can carry a cell the child *just interned*, and the child (and its
//! scratch) is dropped the moment the fork-join call returns, so that cell's
//! `ScratchId` cannot be resolved against the child after the fact. Those callers
//! instead materialize each escaping row to a dataset-independent
//! ([`PortableTerm`]) form **while the child is still alive** ([`portable_row`])
//! and the node re-interns it against the **parent** scratch afterwards, strictly
//! in source-index order ([`reintern_portable_row`]) — see those two functions'
//! doc comments for the base-aware id rule that makes this exact, not just
//! value-equal, to the sequential path.
//!
//! Note there is no `constructed`-merging counterpart here: the parallel minting
//! path only ever runs when [`is_parallel_safe`]/[`is_parallel_safe_pattern`]
//! passes, which excludes every builtin that pushes to
//! [`crate::eval::EvalCtx::constructed`] (the blank-minting list constructors) —
//! so a forked child on this path never populates `constructed`, and there is
//! nothing to fold back.
//!
//! [`is_parallel_safe`] is the gate deciding whether an expression may run under
//! this model at all: any builtin whose result depends on the per-query mutable
//! `bnode_counter`/`rng_state` (or that mints into [`crate::eval::EvalCtx::constructed`])
//! is excluded, because the fork model gives every worker an *independent* copy of
//! that state rather than a shared, ordered one — running such a builtin under
//! fork-join would make its result depend on worker scheduling, not just row
//! content. [`Function::Custom`] — a caller-injected user function resolved
//! against a [`UserFunctionRegistry`] — is likewise excluded unless the registry
//! attests the callee is safe: a native function is safe only when registered
//! [`Volatility::Stable`], and a SPARQL-bodied function is conservatively always
//! unsafe (its body can itself reach `RAND`/`UUID`/`BNODE`/the list constructors,
//! and that per-call state would merge into a forked child instead of `ctx`).

use purrdf_core::{DatasetView, TermId, TermValue, ViewTermId};
use purrdf_sparql_algebra::{Expression, Function, GraphPattern};

use crate::agg_fn::AggregateRegistry;
use crate::error::EvalError;
use crate::governor::ItemCharge;
use crate::governor::soundness::{
    ExpressionPart, PatternPart, visit_expression_parts, visit_pattern_parts,
};
use crate::property_fn::PropertyFunctionRegistry;
use crate::scratch::{ScratchInterner, SolutionTerm};
use crate::solution::Solution;
use crate::user_fn::{UserFunctionRegistry, Volatility};

/// The caller-injected tables the parallel-safety walk consults when it reaches a
/// host-supplied callee: a [`Function::Custom`] expression call, or a
/// [`GraphPattern::PropertyFunction`] node.
///
/// Carried as one `Copy` value rather than as two arguments so a future third table is
/// threaded through the recursion once instead of at every call site — and so a call
/// site cannot pass the two in the wrong order. Every field is non-optional
/// (`Registry::EMPTY` — the canonical "no such table" value — stands in for the old
/// `None` spelling); the two classifications below still read an EMPTY function table
/// and an EMPTY relation table DIFFERENTLY, and deliberately: an unresolvable function
/// IRI is a deterministic XSD cast or a hard error (safe), while an unresolvable
/// relation is a callee whose volatility is unknown (unsafe). See
/// [`function_is_unsafe`] and [`property_function_is_unsafe`].
#[derive(Debug, Clone, Copy)]
pub(crate) struct SafetyRegistries<'a> {
    /// The SHACL-AF / native function table (`EvalCtx::user_functions`).
    pub(crate) functions: &'a UserFunctionRegistry,
    /// The property-function table (`EvalCtx::property_functions`).
    pub(crate) relations: &'a PropertyFunctionRegistry,
    /// The custom-aggregate table (`EvalCtx::aggregates`), consulted by
    /// [`crate::eval::EvalCtx::may_fork_aggregate`] rather than by the
    /// expression/pattern walks above — a `Custom` aggregate is not an
    /// `Expression` node, so it never reaches [`function_is_unsafe`] or
    /// [`property_function_is_unsafe`]. Carried here anyway so this struct stays
    /// the ONE place every registry the fork-join safety decision needs is
    /// paired, per the type's own doc.
    pub(crate) aggregates: &'a AggregateRegistry,
}

/// Rows/groups at or below this stay sequential (thread spin-up would dominate
/// the work for small inputs). Tuned against the criterion benches in
/// `crates/sparql-eval/benches/`, which are report-only and assert no timing.
pub(crate) const PARALLEL_MIN_ROWS: usize = 1024;

/// Floor on a chunk's item count: below this, splitting further would hand
/// rayon workers slivers dominated by per-chunk overhead (the fork, the `Vec`
/// staging) rather than real work. Mirrors the byte-based floor
/// `purrdf_rdf::native_codecs::text_parse::PARALLEL_MIN_CHUNK_BYTES` applies to
/// the parser's chunk geometry, just in item-count terms.
const PARALLEL_MIN_CHUNK_ITEMS: usize = 16;

/// The chunk size for a [`par_chunk_map`]/[`par_chunk_map_metered`]/
/// [`par_chunk_try_map_init`]/[`par_retain`] run over `len` items: aim for
/// roughly four chunks per rayon worker thread, so work-stealing has enough
/// slices to balance ragged per-item costs (a BGP pattern whose candidate
/// count varies row to row, a GROUP BY group whose size varies group to
/// group) without handing every thread only one coarse-grained slice. Clamped
/// below by [`PARALLEL_MIN_CHUNK_ITEMS`] so a small-but-still-parallel input
/// (just over [`PARALLEL_MIN_ROWS`]) on a many-thread machine never
/// degenerates into chunks of a handful of items each — mirrors the parser's
/// `len / (threads * 4)` geometry.
///
/// **Not** used by [`par_chunk_reduce_init`] — see [`aggregate_chunk_size_for`]
/// for why the within-group aggregate fold needs a chunk plan that does NOT
/// track the live thread count. Every caller this function DOES serve is safe
/// to scale with `rayon::current_num_threads()` because none of them folds
/// chunk-local state back together: [`par_chunk_map`]/[`par_chunk_try_map_init`]/
/// [`par_retain`] each emit one independent output row per input item (a chunk
/// boundary changes scheduling, never the result), and [`par_chunk_map_metered`]
/// charges its governor **per item**, not per chunk (see that function's docs),
/// so a chunk boundary never moves where a ceiling trips either.
fn chunk_size_for(len: usize) -> usize {
    #[cfg(test)]
    if let Some(forced) = FORCE_CHUNK_SIZE.with(std::cell::Cell::get) {
        return forced.max(1);
    }
    let threads = rayon::current_num_threads().max(1);
    (len / (threads * 4).max(1)).max(PARALLEL_MIN_CHUNK_ITEMS)
}

/// The fixed reference parallelism [`aggregate_chunk_size_for`] assumes in place of
/// `rayon::current_num_threads()`. Chosen, not measured — any constant makes the plan a
/// pure function of `len`, and this one keeps the chunk COUNT in the same rough range
/// [`chunk_size_for`]'s live-thread-count formula already produced on a modestly
/// parallel host, so admission behaviour for an existing deployment does not jump.
const AGGREGATE_CHUNK_REFERENCE_THREADS: usize = 8;

/// The chunk size for a [`par_chunk_reduce_init`] within-GROUP aggregate fold over `len`
/// already-DISTINCT-resolved survivor values.
///
/// # Why this is a separate function from [`chunk_size_for`]
///
/// [`chunk_size_for`] deliberately tracks the live host's `rayon::current_num_threads()`.
/// That is harmless for its own callers (see its doc comment) because none of them folds
/// chunk-local state back together. [`par_chunk_reduce_init`] is exactly that: it reduces
/// every chunk's partial accumulator into one final accumulator through
/// [`crate::agg_fn::AggregateAccumulator::combine`] — every built-in aggregate in
/// [`crate::modifier`] implements that SAME trait now, so this applies identically
/// whether the accumulator is a built-in's or a registered custom aggregate's — strictly
/// in chunk order. That makes the chunk COUNT part of the observable computation rather
/// than merely its schedule, in two ways `crate::modifier::eval_custom_aggregate` and
/// `crate::modifier::NumericFold` both rely on being fixed:
///
/// - a custom accumulator's declared [`crate::agg_fn::CustomAggregate::state_bound`] is
///   charged once per LIVE chunk accumulator (`chunk_count - 1` beyond the first,
///   admitted separately) — [`par_chunk_reduce_init`] really does collect every chunk's
///   finished accumulator into one `Vec` before folding them down (see its own doc
///   comment), so a bigger chunk count is a bigger real peak allocation, not an
///   approximation of one;
/// - a bounded-arithmetic fold (`SUM`/`AVG`, or any other accumulator whose `combine` is
///   not exactly reassociation-proof) sees a different sequence of intermediate values
///   depending on exactly where the chunk boundaries fall.
///
/// A chunk count that tracked `rayon::current_num_threads()` therefore made both the
/// governor's CHARGE and, for some accumulators, the query's ANSWER depend on the machine
/// the query happened to run on — the same (query, data, ceiling) triple could be admitted
/// on a small box and refused on a big one. That is a portability break `.goals`' MAXIMAL
/// PORTABILITY forbids, so this planner assumes a FIXED reference parallelism
/// ([`AGGREGATE_CHUNK_REFERENCE_THREADS`]) instead of reading the live thread count: the
/// chunk size (hence the chunk count, hence the exact `combine` sequence, hence the
/// charge) becomes a pure function of `len` alone — identical on a 1-thread host, a
/// 32-thread host, and wasm32 (which has no threads at all, and so already computed
/// today's formula's `threads == 1` case every time — this makes every OTHER host match
/// wasm32's plan instead of the other way around).
///
/// Real parallelism is unaffected: [`par_chunk_reduce_init`] still hands the resulting
/// chunks to `rayon::par_chunks`, which work-steals them across however many threads the
/// host actually has. A fixed PLAN is a fixed grouping of rows into the partials that
/// execution folds — it says nothing about how many of those partials run concurrently,
/// which is exactly what a work-stealing scheduler is for.
fn aggregate_chunk_size_for(len: usize) -> usize {
    #[cfg(test)]
    if let Some(forced) = FORCE_CHUNK_SIZE.with(std::cell::Cell::get) {
        return forced.max(1);
    }
    (len / (AGGREGATE_CHUNK_REFERENCE_THREADS * 4).max(1)).max(PARALLEL_MIN_CHUNK_ITEMS)
}

std::thread_local! {
    /// Production operation-scope override used by fallible lazy views. Their page
    /// request order and exact budget boundary are observable evidence, so no
    /// evaluator fork may race requests while this flag is set.
    static FORCE_SEQUENTIAL_OPERATION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Force every evaluator fork gate to remain sequential on the current thread until
/// the returned guard is dropped. Nested scopes restore the previous value.
#[must_use]
pub(crate) fn force_sequential_operation() -> SequentialOperationGuard {
    let previous = FORCE_SEQUENTIAL_OPERATION.with(|cell| cell.replace(true));
    SequentialOperationGuard { previous }
}

/// Whether the current operation requires deterministic sequential evaluation.
pub(crate) fn sequential_operation_required() -> bool {
    FORCE_SEQUENTIAL_OPERATION.with(std::cell::Cell::get)
}

/// RAII restoration for [`force_sequential_operation`].
pub(crate) struct SequentialOperationGuard {
    previous: bool,
}

impl Drop for SequentialOperationGuard {
    fn drop(&mut self) {
        FORCE_SEQUENTIAL_OPERATION.with(|cell| cell.set(self.previous));
    }
}

#[cfg(test)]
std::thread_local! {
    /// Test-only override for [`should_parallelize`], so a bench/test can force
    /// the parallel or sequential branch regardless of `work_items`. Never
    /// consulted outside `cfg(test)` — the shipping decision is purely
    /// `work_items > PARALLEL_MIN_ROWS`.
    static FORCE_PARALLEL: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

/// Force [`should_parallelize`] to always return `force` for the current thread
/// until the returned guard is dropped (restores the prior override). Test-only.
#[cfg(test)]
#[must_use]
pub(crate) fn force_parallel_for_test(force: bool) -> ForceParallelGuard {
    let previous = FORCE_PARALLEL.with(|cell| cell.replace(Some(force)));
    ForceParallelGuard { previous }
}

/// RAII guard restoring the prior [`FORCE_PARALLEL`] override on drop.
#[cfg(test)]
pub(crate) struct ForceParallelGuard {
    previous: Option<bool>,
}

#[cfg(test)]
impl Drop for ForceParallelGuard {
    fn drop(&mut self) {
        FORCE_PARALLEL.with(|cell| cell.set(self.previous));
    }
}

#[cfg(test)]
std::thread_local! {
    /// Test-only override for [`chunk_size_for`]'s AND [`aggregate_chunk_size_for`]'s
    /// result, so a test can pin an exact chunk size (hence an exact chunk count)
    /// regardless of `rayon::current_num_threads()` (which varies by machine/CI runner)
    /// or of [`AGGREGATE_CHUNK_REFERENCE_THREADS`]. Shared between the two functions
    /// rather than given a second cell, because a test that wants an exact chunk count
    /// never cares which of the two chunked primitives it is pinning. Never consulted
    /// outside `cfg(test)`.
    static FORCE_CHUNK_SIZE: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

/// Force [`chunk_size_for`] and [`aggregate_chunk_size_for`] to always return `size` for
/// the current thread until the returned guard is dropped (restores the prior override).
/// Test-only — lets a test span an exact number of chunks deterministically.
#[cfg(test)]
#[must_use]
pub(crate) fn force_chunk_size_for_test(size: usize) -> ForceChunkSizeGuard {
    let previous = FORCE_CHUNK_SIZE.with(|cell| cell.replace(Some(size)));
    ForceChunkSizeGuard { previous }
}

/// RAII guard restoring the prior [`FORCE_CHUNK_SIZE`] override on drop.
#[cfg(test)]
pub(crate) struct ForceChunkSizeGuard {
    previous: Option<usize>,
}

#[cfg(test)]
impl Drop for ForceChunkSizeGuard {
    fn drop(&mut self) {
        FORCE_CHUNK_SIZE.with(|cell| cell.set(self.previous));
    }
}

/// The exact number of chunks a [`par_chunk_reduce_init`] call over `len` items will
/// actually create: `1` when [`should_parallelize`] declines (the sequential fallback —
/// `init` runs exactly once), else `len.div_ceil(aggregate_chunk_size_for(len))`.
///
/// Exposed so a caller that must METER a per-chunk cost BEFORE folding — a custom
/// aggregate's declared per-accumulator [`crate::agg_fn::CustomAggregate::state_bound`],
/// charged once per accumulator chunking actually creates — charges the exact count
/// rather than an estimate that could drift from what [`par_chunk_reduce_init`] does at
/// runtime (both read the same [`should_parallelize`]/[`aggregate_chunk_size_for`] pair,
/// so there is only one source of truth for "how many chunks"). Named for the ONE caller
/// of [`par_chunk_reduce_init`] this crate has — the within-group aggregate fold — rather
/// than generically: [`aggregate_chunk_size_for`]'s doc comment is where the reasoning for
/// a fixed, host-independent plan lives, and this function must never drift onto
/// [`chunk_size_for`]'s live-thread-count formula, which every OTHER fork-join primitive
/// still correctly uses.
pub(crate) fn planned_aggregate_chunk_count(len: usize) -> usize {
    if !should_parallelize(len) {
        return 1;
    }
    len.div_ceil(aggregate_chunk_size_for(len))
}

/// Whether a batch of `work_items` (rows, groups, branches) should run in
/// parallel rather than sequentially. Small inputs stay sequential because
/// rayon thread hand-off cost would dominate the actual work.
pub(crate) fn should_parallelize(work_items: usize) -> bool {
    if sequential_operation_required() {
        return false;
    }
    #[cfg(test)]
    if let Some(forced) = FORCE_PARALLEL.with(std::cell::Cell::get) {
        return forced;
    }
    work_items > PARALLEL_MIN_ROWS
}

/// Whether `expr` (and everything it recursively contains, including nested
/// `EXISTS` patterns) is safe to evaluate under the fork-join parallel model.
///
/// Unsafe means the expression can reach a builtin whose result depends on
/// per-query mutable counter/RNG/mint state that [`crate::eval::EvalCtx::fork_for_worker`]
/// deliberately does NOT share across workers:
///
/// - [`Function::Rand`], [`Function::Uuid`], [`Function::StrUuid`] — draw from
///   `EvalCtx::rng_state`, which advances per call; forked workers would each
///   restart from the same seed and diverge from (or duplicate) the sequential
///   stream.
/// - [`Function::BNode`] (**every** arity, including `BNODE(?x)`) — mints from
///   `EvalCtx::bnode_counter`. Even the one-argument form is unsafe here: SPARQL
///   only requires "same argument value within one query ⇒ same blank node", but
///   under the fork model each worker has its own independent counter, so two
///   workers minting for the *same* argument value would produce two different
///   labels. A worker-local counter cannot honor that invariant across workers.
/// - The PurRDF list constructors `listSlice`/`listConcat`
///   ([`purrdf_sparql_algebra::PurrdfFn::ListSlice`] /
///   [`purrdf_sparql_algebra::PurrdfFn::ListConcat`], reached through
///   [`Function::Purrdf`]) — `crate::list_fn::materialize_list` both mints
///   fresh blank nodes from the shared `bnode_counter` (so a list cell's label
///   never collides with a `BNODE()` or CONSTRUCT-template blank) AND pushes
///   the new cell quads onto `EvalCtx::constructed`. `constructed` is
///   dataset-independent so the cells themselves would fold back
///   deterministically if ever needed, but the *label* is only collision-free
///   against the single shared counter; two forked workers each minting from
///   their own fresh `bnode_counter` could produce colliding cell labels. (In
///   practice this whole builtin is excluded from the parallel path anyway —
///   see the module docs' note on why no `constructed`-merge exists here.)
///
/// Every other reader-only PurRDF list function (`listLength`/`listGet`/
/// `listIndexOf`/`listContains`) and `heldIn` touch neither counter, so they are
/// left safe. When in doubt this walk flags UNSAFE — a sequential fallback is
/// always a correct (if slower) answer.
///
/// `registries` carries the caller's injected tables (see [`SafetyRegistries`]); they
/// are consulted only when the walk reaches a host-supplied callee — a
/// [`Function::Custom`] call ([`function_is_unsafe`]) or, inside an `EXISTS` pattern, a
/// property-function node ([`property_function_is_unsafe`]).
pub(crate) fn is_parallel_safe(expr: &Expression, registries: SafetyRegistries<'_>) -> bool {
    !expr_reaches_unsafe_builtin(expr, registries)
}

/// Whether evaluating `expr` for one row can **re-enter whole-pattern evaluation**, and
/// therefore charge a governor, from inside a forked worker.
///
/// # Why this question is asked separately from [`is_parallel_safe`]
///
/// The two fork lanes account for a budget in fundamentally different ways.
/// [`par_chunk_map_metered`] hands each chunk a private, atomic-free
/// [`ItemCharge`] ledger and the caller folds those in source-item order, so its reported
/// consumption is a pure function of `(query, data, budget)`. The fork-per-worker lane
/// ([`par_chunk_try_map_init`]) has no ledger at all: each worker forks a child
/// [`EvalCtx`](crate::eval::EvalCtx) that shares one `Arc<GovernorState>`, so anything the
/// closure charges goes straight into shared atomics with no ordering.
///
/// For every expression that stays *within* expression evaluation that is harmless,
/// because expression evaluation charges nothing — the row's whole cost is charged once,
/// before the loop, at the operator's own `row-expression-evaluation` charge point on the
/// main thread. Exactly one construct escapes that: an expression-embedded `EXISTS`, which
/// calls back into `eval_evaluated` and charges the full per-node schedule for its inner
/// pattern. And because each *chunk* forks its own child — whose `exists_inner_cache` is a
/// snapshot taken at fork time — the inner pattern is evaluated once per chunk, and the
/// chunk count is derived from `rayon::current_num_threads()`. The reported fuel would
/// then be a function of the machine's thread count, which is exactly the dependence the
/// ordered fold exists to remove; measured on a 1500-row `FILTER EXISTS`, one worker
/// reported 13507 fuel and eight reported 57036 for the identical query and data.
///
/// A SPARQL-bodied user function is the other construct that re-enters evaluation, and it
/// needs no mention here because [`function_is_unsafe`] already classifies it UNSAFE
/// outright, so it never reaches a worker on any path.
///
/// So the rule the evaluator applies is: **a governed execution does not fork a row loop
/// whose expression can re-enter evaluation.** Ungoverned execution is untouched (there is
/// no meter to be exact about), a governed expression that cannot re-enter keeps full
/// parallelism (it charges nothing from a worker), and the narrow remainder runs
/// sequentially — where the charge order is the row order by construction.
pub(crate) fn expression_re_enters_evaluation(expr: &Expression) -> bool {
    let mut found = false;
    visit_expression_parts(expr, &mut |part| {
        found |= match part {
            ExpressionPart::Sub(sub) => expression_re_enters_evaluation(sub),
            ExpressionPart::Call(_) => false,
            ExpressionPart::Exists(_) => true,
        };
        found
    });
    found
}

/// Whether `pattern` (recursively) is safe to evaluate under the fork-join
/// parallel model — the pattern-level twin of [`is_parallel_safe`], for callers
/// (e.g. `UNION`) that must gate a whole sub-pattern rather than a single
/// expression. Exposes the same walk [`is_parallel_safe`] already runs
/// internally for `EXISTS`. `registries` is threaded through exactly as in
/// [`is_parallel_safe`].
pub(crate) fn is_parallel_safe_pattern(
    pattern: &GraphPattern,
    registries: SafetyRegistries<'_>,
) -> bool {
    !pattern_reaches_unsafe_builtin(pattern, registries)
}

/// `true` iff `expr` (recursively) reaches an unsafe builtin — see
/// [`is_parallel_safe`].
///
/// The structural decomposition is [`crate::governor::soundness`]'s, not this
/// module's: the recursion below only decides what to *do* at each part. That
/// module owns the single exhaustive, wildcard-free match over [`Expression`],
/// so a new expression variant is one compile error there rather than a silent
/// omission here (which would classify an unsafe builtin SAFE and let it run
/// under fork-join). The short-circuit is preserved exactly — the visitor stops
/// the moment this closure returns `true` — so the answer is identical to the
/// `||` chain this replaces, including for expressions whose later arms would
/// have been skipped.
fn expr_reaches_unsafe_builtin(expr: &Expression, registries: SafetyRegistries<'_>) -> bool {
    let mut found = false;
    visit_expression_parts(expr, &mut |part| {
        found |= match part {
            ExpressionPart::Sub(sub) => expr_reaches_unsafe_builtin(sub, registries),
            ExpressionPart::Call(f) => function_is_unsafe(f, registries.functions),
            ExpressionPart::Exists(pattern) => pattern_reaches_unsafe_builtin(pattern, registries),
        };
        found
    });
    found
}

/// Whether `f` is itself one of the stateful-mint builtins (see
/// [`is_parallel_safe`]'s doc comment for the full rationale), or an unsafe
/// [`Function::Custom`] user-function call.
///
/// A `Custom(iri)` call is resolved against `registry` (`ctx.user_functions`):
///
/// - No registry, or the IRI resolves to neither custom kind (an XSD cast, or an
///   undefined-function hard error) — deterministic, so safe.
/// - Resolves to a **native** function — safe iff its declared
///   [`Volatility`] is NOT [`Volatile`](Volatility::Volatile): `Stable` is
///   deterministic for the lifetime of the query, so it may run across
///   fork-join workers exactly like a pure builtin.
/// - Resolves to a **SPARQL-bodied** [`crate::user_fn::UserFunction`] —
///   ALWAYS unsafe, conservatively: its body is itself an arbitrary SPARQL
///   query that can mint `RAND`/`UUID`/`BNODE`/list cells, and that per-call
///   state would merge into a forked child (see
///   `crate::user_fn::eval_user_function`'s state merge-back) rather than the
///   real `ctx` — silently diverging from the sequential stream exactly like
///   the builtins above. A sequential fallback is always correct.
fn function_is_unsafe(f: &Function, registry: &UserFunctionRegistry) -> bool {
    match f {
        Function::Rand | Function::Uuid | Function::StrUuid | Function::BNode => true,
        Function::Purrdf(call) => matches!(
            call.fn_kind,
            purrdf_sparql_algebra::PurrdfFn::ListSlice
                | purrdf_sparql_algebra::PurrdfFn::ListConcat
        ),
        Function::Custom(iri) => {
            // An EMPTY registry resolves neither kind, naturally falling through to
            // `false` below — exactly the old "no registry configured" fallback,
            // with no separate absent-registry branch needed now that there is only
            // one spelling of "nothing registered".
            if let Some(native) = registry.resolve_native(iri.as_str()) {
                // Native fn: unsafe iff declared Volatile; Stable is
                // deterministic-within-query, hence fork-join safe. (Wildcard
                // arm — Volatility is `#[non_exhaustive]`.)
                return matches!(native.volatility, Volatility::Volatile);
            }
            // A SPARQL-bodied user function's body may mint RAND/UUID/BNODE/list
            // cells whose per-query state merges into a *forked* child and is
            // then discarded — silently diverging from the sequential stream.
            // Classify it UNSAFE (conservative + correct; sequential is always
            // right). An IRI resolving to neither custom kind is an XSD cast or
            // a hard error — both deterministic — so it stays safe.
            registry.resolve(iri.as_str()).is_some()
        }
        _ => false,
    }
}

/// `true` iff `pattern` (recursively) reaches an unsafe builtin through any
/// expression-bearing variant — see [`is_parallel_safe`].
///
/// Expressed in terms of [`crate::governor::soundness::visit_pattern_parts`],
/// the one exhaustive, wildcard-free walk over [`GraphPattern`] this crate owns.
/// The [`ChildEdge`](crate::governor::soundness::ChildEdge) that walk attaches to
/// each child is the answer-completeness certificate's business, not this gate's,
/// so it is discarded here: parallel safety is a property of the builtins a
/// subtree can reach, and every child can reach them regardless of how a
/// truncation would propagate through it. Sharing the *decomposition* is the
/// point — a new algebra variant must be one compile error, not three
/// independent edits of which only two get found.
fn pattern_reaches_unsafe_builtin(
    pattern: &GraphPattern,
    registries: SafetyRegistries<'_>,
) -> bool {
    // A property-function node is a LEAF for the shared decomposition — it has neither
    // a child pattern nor an attached expression — so the visitor yields nothing for
    // it and the walk below would classify it safe by omission. Its callee is host
    // Rust, which is exactly the case this gate exists for, so it is decided here,
    // before the walk.
    if let GraphPattern::PropertyFunction(call) = pattern {
        return property_function_is_unsafe(&call.iri, registries.relations);
    }
    let mut found = false;
    visit_pattern_parts(pattern, &mut |part| {
        found |= match part {
            PatternPart::Child(child, _edge) => pattern_reaches_unsafe_builtin(child, registries),
            PatternPart::Expression(expr) => expr_reaches_unsafe_builtin(expr, registries),
        };
        found
    });
    found
}

/// Whether a property-function call on `iri` is unsafe to evaluate from a forked
/// worker.
///
/// Safe in exactly one case: the registry is present, it resolves `iri`, and the
/// resolved relation declares [`Volatility::Stable`] — deterministic for the lifetime
/// of the query, hence identical whichever worker runs it, exactly like a `Stable`
/// native function.
///
/// Every other case is UNSAFE, including the two absences. This is the reverse of
/// [`function_is_unsafe`]'s treatment of an unresolvable IRI, and the asymmetry is the
/// point: an unresolved *function* IRI has a defined deterministic meaning (an XSD cast,
/// or a hard error), so nothing can diverge; an unresolved *relation* IRI has no
/// meaning yet, and classifying an unknown callee's volatility as `Stable` would be a
/// guess in the one direction that can silently change an answer. A sequential fallback
/// is always correct, so "when in doubt, UNSAFE".
fn property_function_is_unsafe(iri: &str, relations: &PropertyFunctionRegistry) -> bool {
    let Some(relation) = relations.resolve(iri) else {
        return true;
    };
    // Wildcard-shaped match — `Volatility` is `#[non_exhaustive]`, and a class added
    // later must be unsafe here until it is deliberately admitted.
    !matches!(relation.volatility(), Volatility::Stable)
}

/// Whether a `Custom` aggregate call on `iri` is unsafe to fold from a forked
/// per-GROUP worker — the aggregate twin of [`property_function_is_unsafe`], read
/// by [`crate::eval::EvalCtx::may_fork_aggregate`] rather than by the
/// expression/pattern walks above (an `AggregateFunction` is not an `Expression`
/// node, so it never reaches [`function_is_unsafe`]).
///
/// Safe in exactly one case: `aggregates` resolves `iri`, and the resolved
/// aggregate declares [`Volatility::Stable`] — deterministic for the lifetime of
/// the query, hence identical whichever worker folds the group.
///
/// Every other case is UNSAFE, including an EMPTY registry (which resolves
/// nothing) — the SAME conservative treatment [`property_function_is_unsafe`]
/// gives an unresolved relation, and the DELIBERATE OPPOSITE of
/// [`function_is_unsafe`]'s treatment of
/// an unresolved scalar `Custom` function IRI: an unresolved scalar function has
/// a defined deterministic meaning (an XSD cast, or a hard error), so nothing can
/// diverge, while an unresolved aggregate's volatility is simply unknown, and
/// guessing `Stable` would be a guess in the one direction that can silently
/// change an answer. A sequential fallback is always correct, so "when in doubt,
/// UNSAFE".
pub(crate) fn aggregate_is_unsafe(iri: &str, aggregates: &AggregateRegistry) -> bool {
    let Some(aggregate) = aggregates.resolve(iri) else {
        return true;
    };
    !matches!(aggregate.volatility(), Volatility::Stable)
}

/// Chunk-based, infallible parallel collect: split `items` into index-ordered
/// chunks (never `par_sort`/`par_bridge`), give each chunk worker ONE `Vec<R>`
/// accumulator (`push` is called once per item, appending into it), and
/// concatenate the per-chunk accumulators in chunk order. This is the
/// allocation shape [`purrdf_rdf::native_codecs::text_parse::parse_lines_parallel_with_chunk_size`]
/// uses for its phase 1: one allocation per CHUNK, not one per item — the
/// per-item shape (a fresh `Vec` returned by every worker call, flattened
/// afterwards) this replaces cost an extra small allocation for every row of
/// an N-row BGP/join/filter loop, pure overhead the chunk shape avoids.
///
/// Internally gated on [`should_parallelize`]: at or below [`PARALLEL_MIN_ROWS`]
/// this is a single sequential pass pushing into one `Vec` (bit-identical to a
/// hand-written loop, no rayon hand-off); above it, `items.par_chunks(..)` (an
/// *indexed*, order-preserving split) runs `push` over each chunk into its own
/// accumulator, and the chunk accumulators are concatenated strictly in chunk
/// (hence source) order — so the result is byte-identical to the sequential
/// pass regardless of chunk geometry or worker scheduling.
pub(crate) fn par_chunk_map<T, R>(items: &[T], push: impl Fn(&mut Vec<R>, &T) + Sync) -> Vec<R>
where
    T: Sync,
    R: Send,
{
    if !should_parallelize(items.len()) {
        let mut out = Vec::new();
        for item in items {
            push(&mut out, item);
        }
        return out;
    }

    use rayon::prelude::*;

    let size = chunk_size_for(items.len());
    let chunk_outs: Vec<Vec<R>> = items
        .par_chunks(size)
        .map(|chunk| {
            let mut acc = Vec::new();
            for item in chunk {
                push(&mut acc, item);
            }
            acc
        })
        .collect();

    let mut out = Vec::with_capacity(chunk_outs.iter().map(Vec::len).sum());
    for chunk_out in chunk_outs {
        out.extend(chunk_out);
    }
    out
}

/// The output accumulator an operator's row loop pushes into, carrying the row ceiling the
/// answer-cap pushdown put on this node.
///
/// # Why an accumulator type rather than a `Vec` and a length check
///
/// The pushdown has to stop a scan **inside** one input item, not merely between items:
/// the plan that motivates it is `SELECT * WHERE { ?s ?p ?o } LIMIT 10`, whose row loop has
/// exactly **one** input item — the all-unbound seed row — and whose entire cost is the
/// index scan that item performs. A driver that could only stop between items would save
/// nothing at all on precisely the query the cap exists to protect.
///
/// So the ceiling travels with the accumulator and the operator asks it, and the two
/// drivers below differ only in whether the ceiling is finite. The unbounded sink compiles
/// to the `Vec` push it replaces plus one `usize` compare against `usize::MAX`.
#[derive(Debug)]
pub(crate) struct RowSink<'a, R> {
    /// The rows accumulated for the current chunk.
    out: &'a mut Vec<R>,
    /// The total this sink will accept. `usize::MAX` is "no ceiling".
    ceiling: usize,
    /// Whether reaching the ceiling itself closes the sink (semantic LIMIT/answer-cap
    /// pushdown), or whether the sink stays open until a further row is refused
    /// (intermediate-cell governance, whose inclusive boundary must distinguish exactly
    /// full from overflowing).
    stop_when_full: bool,
    /// A qualifying row was offered after `ceiling` rows were already stored.
    overflowed: bool,
}

impl<'a, R> RowSink<'a, R> {
    /// A sink that accepts every row.
    fn unbounded(out: &'a mut Vec<R>) -> Self {
        Self {
            out,
            ceiling: usize::MAX,
            stop_when_full: true,
            overflowed: false,
        }
    }

    /// A sink that accepts rows until `out` holds `ceiling` of them.
    fn bounded(out: &'a mut Vec<R>, ceiling: usize) -> Self {
        Self {
            out,
            ceiling,
            stop_when_full: true,
            overflowed: false,
        }
    }

    /// A sink for an inclusive allocation ceiling: retain at most `ceiling` rows, but keep
    /// probing until a qualifying row beyond it is offered. The refused row is never
    /// stored, and its existence is the proof that the exactly-full bag was not complete.
    fn overflow_bounded(out: &'a mut Vec<R>, ceiling: usize) -> Self {
        Self {
            out,
            ceiling,
            stop_when_full: false,
            overflowed: false,
        }
    }

    /// Accept `row`, unless this sink is already full.
    ///
    /// Silently dropping the row past the ceiling rather than returning a refusal is the
    /// right shape here: a caller that ignores [`Self::is_full`] then produces exactly the
    /// ceiling's worth of rows, which is a prefix of what it would have produced — never a
    /// different bag.
    #[inline]
    pub(crate) fn push(&mut self, row: R) {
        if self.out.len() < self.ceiling {
            self.out.push(row);
        } else {
            self.overflowed = true;
        }
    }

    /// Whether this sink has reached its ceiling, so the loop feeding it may stop.
    #[inline]
    pub(crate) fn is_full(&self) -> bool {
        if self.stop_when_full {
            self.out.len() >= self.ceiling
        } else {
            self.overflowed
        }
    }

    /// Whether this sink refused a qualifying row beyond its inclusive ceiling.
    #[inline]
    fn overflowed(&self) -> bool {
        self.overflowed
    }
}

/// The **bounded, sequential** sibling of [`par_chunk_map_metered`]: fold `items` in source
/// order into at most `ceiling` output rows, stopping the moment the ceiling is reached.
///
/// Sequential on purpose. Parallel chunks would each have to be given the whole ceiling —
/// no worker can know how many rows the chunks before it produced — so the ceiling would
/// stop nothing until the chunks were concatenated, which is the materialisation it exists
/// to prevent. The bounded path is entered only when the plan proved a ceiling applies, so
/// the work it forgoes parallelising is bounded by that ceiling.
///
/// The returned ledger covers exactly the items that were folded, so the caller's
/// [`crate::governor::GovernorState::commit_ordered_items`] charges for the work that was
/// actually done and no more.
pub(crate) fn bounded_chunk_map_metered<T, R>(
    items: &[T],
    metered: bool,
    ceiling: usize,
    push: impl Fn(&mut RowSink<'_, R>, &mut u64, &T),
) -> (Vec<R>, Vec<ItemCharge>) {
    let mut out = Vec::new();
    let mut ledger = Vec::with_capacity(if metered { items.len() } else { 0 });
    for item in items {
        if out.len() >= ceiling {
            break;
        }
        let before = out.len();
        let mut fuel = 0_u64;
        push(&mut RowSink::bounded(&mut out, ceiling), &mut fuel, item);
        if metered {
            ledger.push(ItemCharge {
                fuel,
                committed: (out.len() - before) as u64,
            });
        }
    }
    (out, ledger)
}

/// The allocation-ceiling sibling of [`bounded_chunk_map_metered`]. It is sequential for
/// the same reason, but reaching `ceiling` does not itself stop the fold: an inclusive
/// ceiling admits an exactly-full bag. The fold stops only when the producer offers one
/// further qualifying row; that row is refused before vector growth and `true` is returned
/// as the overflow flag.
pub(crate) fn cell_bounded_chunk_map_metered<T, R>(
    items: &[T],
    metered: bool,
    ceiling: usize,
    push: impl Fn(&mut RowSink<'_, R>, &mut u64, &T),
) -> (Vec<R>, Vec<ItemCharge>, bool) {
    let mut out = Vec::with_capacity(ceiling.min(items.len()));
    let mut ledger = Vec::with_capacity(if metered { items.len() } else { 0 });
    let mut overflowed = false;
    for item in items {
        let before = out.len();
        let mut fuel = 0_u64;
        let mut sink = RowSink::overflow_bounded(&mut out, ceiling);
        push(&mut sink, &mut fuel, item);
        overflowed = sink.overflowed();
        if metered {
            ledger.push(ItemCharge {
                fuel,
                committed: (out.len() - before) as u64,
            });
        }
        if overflowed {
            break;
        }
    }
    (out, ledger, overflowed)
}

/// [`par_chunk_map`] with a **deterministic charge meter**: alongside its output rows,
/// each chunk worker records what every input item spent, so a governed caller can find
/// the exact item at which a ceiling is crossed.
///
/// # How a governed parallel run stays deterministic
///
/// A shared atomic counter would make the *total* correct and the *trip point* a lottery:
/// whichever worker reached the counter first would decide which item was blamed. What is
/// recorded here instead is per-item and chunk-local — no atomics, no contention, and no
/// shared counter to double-charge — and the chunk ledgers concatenate in chunk (hence
/// source) order exactly like the rows do. The caller then folds them through
/// [`crate::governor::GovernorState::commit_ordered_items`], which walks that one fixed sequence on one
/// thread. The trip point is therefore a pure function of `(query, data, budget)`:
/// independent of worker count, of chunk geometry, and of scheduling.
///
/// `metered` is the caller's short-circuit. When it is `false` — an ungoverned execution,
/// or one whose fuel dimension carries no ceiling — the returned ledger is empty and this
/// function is [`par_chunk_map`] exactly: the `&mut u64` handed to `push` is a stack
/// local, so no per-item record is written and nothing is allocated.
///
/// The ledger costs two `u64` per input item on the metered path, which is why it is a
/// dedicated compact record rather than a full per-item resource vector — see
/// [`ItemCharge`] for both halves of that reasoning.
pub(crate) fn par_chunk_map_metered<T, R>(
    items: &[T],
    metered: bool,
    push: impl Fn(&mut RowSink<'_, R>, &mut u64, &T) + Sync,
) -> (Vec<R>, Vec<ItemCharge>)
where
    T: Sync,
    R: Send,
{
    /// Fold one chunk, recording each item's charge when `metered`.
    fn run_chunk<T, R>(
        chunk: &[T],
        metered: bool,
        push: &(impl Fn(&mut RowSink<'_, R>, &mut u64, &T) + Sync),
    ) -> (Vec<R>, Vec<ItemCharge>) {
        let mut out = Vec::new();
        let mut ledger = Vec::with_capacity(if metered { chunk.len() } else { 0 });
        for item in chunk {
            let before = out.len();
            let mut fuel = 0_u64;
            push(&mut RowSink::unbounded(&mut out), &mut fuel, item);
            if metered {
                ledger.push(ItemCharge {
                    fuel,
                    committed: (out.len() - before) as u64,
                });
            }
        }
        (out, ledger)
    }

    if !should_parallelize(items.len()) {
        return run_chunk(items, metered, &push);
    }

    use rayon::prelude::*;

    let size = chunk_size_for(items.len());
    let per_chunk: Vec<(Vec<R>, Vec<ItemCharge>)> = items
        .par_chunks(size)
        .map(|chunk| run_chunk(chunk, metered, &push))
        .collect();

    let mut rows = Vec::with_capacity(per_chunk.iter().map(|(out, _)| out.len()).sum());
    let mut ledger = Vec::with_capacity(per_chunk.iter().map(|(_, l)| l.len()).sum());
    for (chunk_rows, chunk_ledger) in per_chunk {
        rows.extend(chunk_rows);
        ledger.extend(chunk_ledger);
    }
    (rows, ledger)
}

/// The fallible, fork-per-worker sibling of [`par_chunk_map`]: each rayon
/// *chunk* worker first runs `init` **once** to build its own `S` (e.g. an
/// `EvalCtx::fork_for_worker` child), then folds `push` over every item of its
/// chunk into one `Vec<R>` accumulator, short-circuiting the chunk on the
/// first `Err`. This forks one child per CHUNK, not per item — the fork
/// (cloning the scratch interner, the `exists_inner_cache`, etc.) is real, if
/// cheap, work that should happen a handful of times, not once per row — and
/// gives the chunk exactly one output allocation instead of one per item.
///
/// Internally gated on [`should_parallelize`]: at or below [`PARALLEL_MIN_ROWS`],
/// `init` runs exactly once and every item folds sequentially over that single
/// state into one `Vec` (bit-identical to a hand-written sequential loop, no
/// rayon hand-off, no extra `init` calls); above it, `items.par_chunks(..)`
/// (an *indexed*, order-preserving split) runs each chunk to completion,
/// collecting `Vec<Result<Vec<R>, EvalError>>` in chunk order, then reduces
/// strictly in that order: successes concatenate in chunk (hence source)
/// order and the first `Err` **by chunk index** wins regardless of which
/// worker finished first — so a fast late chunk can never race ahead of an
/// earlier chunk's diagnostic (mirrors
/// `purrdf_rdf::native_codecs::text_parse::parse_lines_parallel_with_chunk_size`'s
/// phase 2 reduce). Within a chunk, items are folded in source order, so the
/// overall output is exactly source order.
///
/// Generic over the returned element type `R` (not just [`Solution`]): a
/// minting caller (e.g. `eval_extend`, `eval_group`'s per-group compute) can
/// push [`MintedRow`]s instead, since the worker's forked child (and its
/// scratch) is gone by the time the caller can re-intern against the parent.
pub(crate) fn par_chunk_try_map_init<T, S, R>(
    items: &[T],
    init: impl Fn() -> S + Sync,
    push: impl Fn(&mut S, &mut Vec<R>, &T) -> Result<(), EvalError> + Sync,
) -> Result<Vec<R>, EvalError>
where
    T: Sync,
    R: Send,
{
    if !should_parallelize(items.len()) {
        let mut state = init();
        let mut out = Vec::new();
        for item in items {
            push(&mut state, &mut out, item)?;
        }
        return Ok(out);
    }

    use rayon::prelude::*;

    let size = chunk_size_for(items.len());
    let per_chunk: Vec<Result<Vec<R>, EvalError>> = items
        .par_chunks(size)
        .map(|chunk| {
            let mut state = init();
            let mut acc = Vec::new();
            for item in chunk {
                push(&mut state, &mut acc, item)?;
            }
            Ok(acc)
        })
        .collect();

    let mut out = Vec::with_capacity(
        per_chunk
            .iter()
            .map(|r| r.as_ref().map_or(0, Vec::len))
            .sum(),
    );
    for chunk_result in per_chunk {
        out.extend(chunk_result?);
    }
    Ok(out)
}

/// The reducing sibling of [`par_chunk_try_map_init`]: rather than flattening
/// every chunk's pushed items into one `Vec<R>`, each chunk folds down to
/// exactly ONE accumulator `S` (`init` once per chunk, `step` once per item),
/// and the per-chunk accumulators are then **reduced left-to-right, strictly
/// in chunk-index order**, via `combine` — never reassociated, never merged
/// out of order. That fixed order is what makes this primitive safe for a
/// NON-commutative fold (string concatenation, "first value wins", a running
/// extreme under a non-total order): the result is byte-identical to a plain
/// sequential `init(); for item in items { step(&mut s, item); }` fold over
/// the same `items` in source order, for every accumulator regardless of its
/// algebraic class — see `crate::agg_fn::AlgebraicClass`'s module docs, whose
/// `combine`-in-chunk-order contract this primitive is the evaluator-side
/// counterpart of.
///
/// Used by `modifier::eval_aggregate`/`eval_custom_aggregate` to fold ONE
/// `GROUP BY` group's (already-evaluated, already-`DISTINCT`-deduped) values
/// in parallel when the group itself is large — the fork [`par_chunk_try_map_init`]
/// drives in `modifier::eval_group` parallelizes ACROSS groups; this
/// primitive parallelizes WITHIN one group's fold, the case a single huge
/// group (one `GROUP BY` key, millions of rows) never benefits from the
/// across-groups fork at all. Deliberately given NO [`crate::eval::EvalCtx`]
/// access: every item this primitive folds is already a plain, dataset-
/// independent value (a [`crate::scratch::SolutionTerm`] minted by the
/// UNCHANGED sequential row-evaluation loop that runs before chunking even
/// begins), so `step`/`combine` here touch no governor, no scratch interner,
/// no `EXISTS` re-entrancy — none of [`par_chunk_try_map_init`]'s fork-per-
/// chunk `EvalCtx` machinery is needed, because nothing here evaluates an
/// expression or charges a fuel point. That is also why charging stays
/// byte-identical between the sequential and chunked fold: every charge this
/// aggregate's row loop makes happens in that unchanged pre-chunking pass,
/// never in here.
///
/// Internally gated on [`should_parallelize`], mirroring [`par_chunk_try_map_init`]
/// exactly: at or below [`PARALLEL_MIN_ROWS`], `init` runs once and every item
/// folds directly into that one state (`combine` is never called) — bit-
/// identical to a hand-written sequential loop; above it, `items.par_chunks(..)`
/// folds each chunk independently and the results are reduced in chunk order.
/// `items.is_empty()` is `init()`'s state with no `step` at all, exactly the
/// same empty-group answer either branch gives.
///
/// The chunk size is [`aggregate_chunk_size_for`], NOT [`chunk_size_for`] — see the
/// former's doc comment for why this one primitive needs a chunk plan that is a pure
/// function of `items.len()` rather than of the live host's thread count.
pub(crate) fn par_chunk_reduce_init<T, S>(
    items: &[T],
    init: impl Fn() -> Result<S, EvalError> + Sync,
    step: impl Fn(&mut S, &T) -> Result<(), EvalError> + Sync,
    combine: impl Fn(&mut S, S) -> Result<(), EvalError>,
) -> Result<S, EvalError>
where
    T: Sync,
    S: Send,
{
    if !should_parallelize(items.len()) {
        let mut state = init()?;
        for item in items {
            step(&mut state, item)?;
        }
        return Ok(state);
    }

    use rayon::prelude::*;

    let size = aggregate_chunk_size_for(items.len());
    let per_chunk: Vec<Result<S, EvalError>> = items
        .par_chunks(size)
        .map(|chunk| {
            let mut state = init()?;
            for item in chunk {
                step(&mut state, item)?;
            }
            Ok(state)
        })
        .collect();

    // Reduce strictly in chunk-index order: the first `Err` **by chunk index**
    // wins (via `?` on the sequential `for` below), regardless of which worker
    // finished first — mirrors `par_chunk_try_map_init`'s reduce exactly.
    let mut iter = per_chunk.into_iter();
    let mut acc = match iter.next() {
        Some(first) => first?,
        None => init()?,
    };
    for chunk_result in iter {
        combine(&mut acc, chunk_result?)?;
    }
    Ok(acc)
}

/// An order-stable, internally-gated parallel filter-clone: keep every item of
/// `items` for which `keep` returns `true`, cloning it into the output in
/// source order. Sequential at/below [`PARALLEL_MIN_ROWS`] (a plain retain);
/// above it, rayon's indexed `par_iter().filter().cloned()`, which preserves
/// source order exactly like the sequential path (never `par_sort`/
/// `par_bridge`). Used by `MINUS`, whose predicate is a pure read-only
/// compatibility check.
pub(crate) fn par_retain<T, F>(items: &[T], keep: F) -> Vec<T>
where
    T: Clone + Sync + Send,
    F: Fn(&T) -> bool + Sync + Send,
{
    if !should_parallelize(items.len()) {
        return items.iter().filter(|item| keep(item)).cloned().collect();
    }

    use rayon::prelude::*;

    items
        .par_iter()
        .filter(|item| keep(item))
        .cloned()
        .collect()
}

/// One cell of a minting node's output row, materialized to a form that
/// survives the forked child (and its scratch) being dropped.
///
/// A forked child's scratch is a **clone** of the parent's at fork time (see
/// [`crate::eval::EvalCtx::fork_for_worker`]), so a [`crate::scratch::SolutionTerm::Computed`]
/// id already carries meaning independent of *which* scratch resolves it, as
/// long as that id was minted before the fork: `base` (the parent's
/// [`ScratchInterner::computed_count`] at fork time) is the dividing line.
///
/// - `sid < base` — already a valid PARENT id (the child inherited it via the
///   clone); pass it through unchanged as [`PortableTerm::Parent`].
/// - `sid >= base` — a term the CHILD freshly minted after the fork; the
///   parent has never seen it, so it is captured as its dataset-independent
///   [`TermValue`] ([`PortableTerm::Fresh`]) while the child (and its scratch)
///   is still alive, for the caller to re-intern against the parent later.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PortableTerm<I = TermId> {
    /// A term already valid in the parent's id space: an `Existing` dataset
    /// term, or a `Computed` id minted before the fork.
    Parent(SolutionTerm<I>),
    /// A value the child minted after the fork; not yet interned anywhere but
    /// the child's own (about-to-be-dropped) scratch.
    Fresh(TermValue),
}

/// Materialize one output `row` produced against a forked child's `local`
/// scratch into a portable form, while `local` is still alive. `base` is the
/// parent's [`ScratchInterner::computed_count`] captured **at fork time** —
/// see [`PortableTerm`] for the id rule this relies on.
pub(crate) fn portable_row<I: ViewTermId>(
    local: &ScratchInterner,
    base: usize,
    row: &Solution<I>,
) -> Vec<Option<PortableTerm<I>>> {
    row.iter()
        .map(|cell| match cell {
            None => None,
            Some(SolutionTerm::Computed(sid)) if sid.index() >= base => {
                Some(PortableTerm::Fresh(local.computed_value(*sid).clone()))
            }
            Some(term) => Some(PortableTerm::Parent(*term)),
        })
        .collect()
}

/// Re-intern a [`portable_row`] output back into the `main` (parent) scratch:
/// a [`PortableTerm::Parent`] cell passes through unchanged (already valid in
/// `main`'s id space); a [`PortableTerm::Fresh`] cell is interned against
/// `main`/`dataset`, deduplicating against anything `main` (or an
/// earlier-reinterned sibling row, when the caller processes rows in source
/// order as required) already holds.
///
/// Callers MUST invoke this once per row **in source-index order** across all
/// workers — that ordering, not anything in this function, is what makes two
/// workers minting the same fresh value converge on the same parent id
/// deterministically (whichever reinterns first wins the id; the same value
/// reinterned again is deduplicated against it, not re-minted).
pub(crate) fn reintern_portable_row<D: DatasetView>(
    main: &mut ScratchInterner,
    dataset: &D,
    prow: Vec<Option<PortableTerm<D::Id>>>,
) -> Solution<D::Id> {
    prow.into_iter()
        .map(|cell| match cell {
            None => None,
            Some(PortableTerm::Parent(term)) => Some(term),
            Some(PortableTerm::Fresh(value)) => Some(main.intern(dataset, value)),
        })
        .collect()
}

/// One row escaping a minting fork-join worker (a `UNION` branch, a GROUP BY
/// group, a `BIND`): either already valid in the PARENT id space, or one that
/// must be re-interned via a [`portable_row`] captured while the minting
/// child's scratch is still alive.
///
/// This is the "no-mint fast path": a `Computed(sid)` cell with `sid < base`
/// is already a valid parent id (the child inherited it via the fork-time
/// scratch clone — see [`PortableTerm`]'s doc comment for the exact rule), so
/// a row none of whose cells is a POST-fork mint needs no remap at all — the
/// [`portable_row`]/[`reintern_portable_row`] round trip would be a correct
/// no-op that still pays a per-cell match and a `Vec<Option<PortableTerm>>`
/// allocation. This matters most for a UNION branch that is pure BGP (mints
/// nothing) or a `MIN`/`MAX`/`SAMPLE` group (whose result is an existing bound
/// value passed through) — `BIND`/most aggregates mint a genuinely new value
/// and always take the `Portable` arm, exactly as before this fast path.
pub(crate) enum MintedRow<I = TermId> {
    /// No cell of this row is a post-fork mint — pass it through untouched.
    Direct(Solution<I>),
    /// At least one cell was freshly minted by the child after the fork;
    /// captured in portable form for [`reintern_minted_row`] to re-intern.
    Portable(Vec<Option<PortableTerm<I>>>),
}

/// Classify one worker-produced `row` into a [`MintedRow`]: `Direct` iff no
/// cell is a `Computed(sid)` with `sid >= base` (a post-fork mint), else
/// `Portable` (materialized against `local` — the minting child's scratch —
/// while it is still alive). See [`MintedRow`] for the reasoning.
pub(crate) fn minted_row<I: ViewTermId>(
    local: &ScratchInterner,
    base: usize,
    row: Solution<I>,
) -> MintedRow<I> {
    let has_fresh_mint = row
        .iter()
        .any(|cell| matches!(cell, Some(SolutionTerm::Computed(sid)) if sid.index() >= base));
    if has_fresh_mint {
        MintedRow::Portable(portable_row(local, base, &row))
    } else {
        MintedRow::Direct(row)
    }
}

/// Re-intern one [`MintedRow`] back into the parent (`main`) scratch: a
/// `Direct` row passes through unchanged; a `Portable` row goes through
/// [`reintern_portable_row`]. Callers MUST invoke this once per row in
/// source-index order across all workers — see [`reintern_portable_row`]'s doc
/// comment for why that ordering (not anything in this function) is what makes
/// the result deterministic.
pub(crate) fn reintern_minted_row<D: DatasetView>(
    main: &mut ScratchInterner,
    dataset: &D,
    row: MintedRow<D::Id>,
) -> Solution<D::Id> {
    match row {
        MintedRow::Direct(solution) => solution,
        MintedRow::Portable(prow) => reintern_portable_row(main, dataset, prow),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::RdfDatasetBuilder;
    use purrdf_sparql_algebra::{Literal, NamedNode, PurrdfCall, PurrdfFn, TriplePattern};

    // ---- should_parallelize -------------------------------------------------

    #[test]
    fn should_parallelize_boundary() {
        assert!(!should_parallelize(PARALLEL_MIN_ROWS));
        assert!(should_parallelize(PARALLEL_MIN_ROWS + 1));
    }

    #[test]
    fn should_parallelize_force_seam() {
        {
            let _guard = force_parallel_for_test(true);
            assert!(should_parallelize(0));
        }
        {
            let _guard = force_parallel_for_test(false);
            assert!(!should_parallelize(usize::MAX));
        }
        // Guard dropped: back to the real threshold.
        assert!(!should_parallelize(1));
    }

    #[test]
    fn fallible_operation_scope_overrides_forced_parallelism() {
        assert!(!sequential_operation_required());
        let _parallel = force_parallel_for_test(true);
        assert!(should_parallelize(0), "test seam forces parallel first");
        {
            let _sequential = force_sequential_operation();
            assert!(sequential_operation_required());
            assert!(
                !should_parallelize(usize::MAX),
                "operation completeness takes precedence over every fork gate"
            );
            let _nested = force_sequential_operation();
            assert!(
                sequential_operation_required(),
                "nested scope remains active"
            );
        }
        assert!(!sequential_operation_required());
        assert!(
            should_parallelize(0),
            "dropping the operation guard restores the prior test override"
        );
    }

    // ---- is_parallel_safe ----------------------------------------------------

    /// No caller-injected table of either kind — every registry the canonical
    /// `EMPTY` value, exactly what [`crate::eval::EvalCtx::new`] carries before any
    /// `with_*` setter runs.
    const NONE: SafetyRegistries<'static> = SafetyRegistries {
        functions: &UserFunctionRegistry::EMPTY,
        relations: &PropertyFunctionRegistry::EMPTY,
        aggregates: &AggregateRegistry::EMPTY,
    };

    /// Only a function table configured.
    fn fns(registry: &UserFunctionRegistry) -> SafetyRegistries<'_> {
        SafetyRegistries {
            functions: registry,
            relations: NONE.relations,
            aggregates: NONE.aggregates,
        }
    }

    /// Only a relation table configured.
    fn relations(registry: &PropertyFunctionRegistry) -> SafetyRegistries<'_> {
        SafetyRegistries {
            functions: NONE.functions,
            relations: registry,
            aggregates: NONE.aggregates,
        }
    }

    fn call(f: Function, args: Vec<Expression>) -> Expression {
        Expression::FunctionCall(f, args)
    }

    #[test]
    fn plain_arithmetic_and_regex_are_safe() {
        let arith = Expression::Add(
            Box::new(Expression::Literal(Literal::new_simple("1"))),
            Box::new(Expression::Literal(Literal::new_simple("2"))),
        );
        assert!(is_parallel_safe(&arith, NONE));

        let regex = call(
            Function::Regex,
            vec![
                Expression::Variable(purrdf_sparql_algebra::Variable::new("x")),
                Expression::Literal(Literal::new_simple("^a")),
            ],
        );
        assert!(is_parallel_safe(&regex, NONE));
    }

    #[test]
    fn rand_uuid_struuid_bnode_are_unsafe() {
        assert!(!is_parallel_safe(&call(Function::Rand, vec![]), NONE));
        assert!(!is_parallel_safe(&call(Function::Uuid, vec![]), NONE));
        assert!(!is_parallel_safe(&call(Function::StrUuid, vec![]), NONE));
        assert!(!is_parallel_safe(&call(Function::BNode, vec![]), NONE));
        assert!(!is_parallel_safe(
            &call(
                Function::BNode,
                vec![Expression::Variable(purrdf_sparql_algebra::Variable::new(
                    "x"
                ))]
            ),
            NONE
        ));
    }

    #[test]
    fn list_constructors_are_unsafe_readers_are_safe() {
        let mk = |kind: PurrdfFn, iri: &str| {
            call(
                Function::Purrdf(PurrdfCall {
                    fn_kind: kind,
                    iri: iri.to_owned(),
                }),
                vec![],
            )
        };
        assert!(!is_parallel_safe(
            &mk(PurrdfFn::ListSlice, "http://ex/listSlice"),
            NONE
        ));
        assert!(!is_parallel_safe(
            &mk(PurrdfFn::ListConcat, "http://ex/listConcat"),
            NONE
        ));
        assert!(is_parallel_safe(
            &mk(PurrdfFn::ListLength, "http://ex/listLength"),
            NONE
        ));
        assert!(is_parallel_safe(
            &mk(PurrdfFn::HeldIn, "http://ex/heldIn"),
            NONE
        ));
    }

    #[test]
    fn unsafe_nested_in_if_coalesce_and_function_args_is_detected() {
        let cond = Expression::Bound(purrdf_sparql_algebra::Variable::new("x"));
        let rand = call(Function::Rand, vec![]);
        let safe = Expression::Literal(Literal::new_simple("ok"));

        let in_if = Expression::If(
            Box::new(cond),
            Box::new(safe.clone()),
            Box::new(rand.clone()),
        );
        assert!(!is_parallel_safe(&in_if, NONE));

        let in_coalesce = Expression::Coalesce(vec![safe.clone(), rand.clone()]);
        assert!(!is_parallel_safe(&in_coalesce, NONE));

        let in_fn_args = call(Function::Concat, vec![safe, rand]);
        assert!(!is_parallel_safe(&in_fn_args, NONE));
    }

    #[test]
    fn unsafe_inside_nested_exists_filter_is_detected() {
        let vp = |n: &str| {
            purrdf_sparql_algebra::TermPattern::Variable(purrdf_sparql_algebra::Variable::new(n))
        };
        let pred = |iri: &str| {
            purrdf_sparql_algebra::NamedNodePattern::NamedNode(NamedNode::new_unchecked(iri))
        };
        let inner_bgp = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: vp("s"),
                predicate: pred("http://ex/p"),
                object: vp("o"),
            }],
        };
        let filtered_inner = GraphPattern::Filter {
            expr: call(Function::Rand, vec![]),
            inner: Box::new(inner_bgp),
        };
        let exists = Expression::Exists(Box::new(filtered_inner));
        assert!(!is_parallel_safe(&exists, NONE));

        // Sanity: the same shape without RAND() is safe.
        let inner_bgp2 = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: vp("s"),
                predicate: pred("http://ex/p"),
                object: vp("o"),
            }],
        };
        let safe_exists = Expression::Exists(Box::new(inner_bgp2));
        assert!(is_parallel_safe(&safe_exists, NONE));
    }

    // ---- is_parallel_safe: Function::Custom / UserFunctionRegistry ----------

    const CUSTOM_NATIVE_IRI: &str = "http://example.org/ns#customNative";
    const CUSTOM_SPARQL_IRI: &str = "http://example.org/ns#customSparql";
    const CUSTOM_UNKNOWN_IRI: &str = "http://example.org/ns#customUnknown";

    fn custom_call(iri: &str) -> Expression {
        call(Function::Custom(NamedNode::new_unchecked(iri)), Vec::new())
    }

    fn trivial_native_body() -> crate::user_fn::NativeFnBody {
        std::sync::Arc::new(|_args: &[&TermValue]| {
            Ok(TermValue::typed_literal(
                "1",
                "http://www.w3.org/2001/XMLSchema#integer",
            ))
        })
    }

    fn trivial_sparql_function() -> crate::user_fn::UserFunction {
        crate::user_fn::UserFunction {
            params: Vec::new(),
            required: 0,
            body: std::sync::Arc::new(
                purrdf_sparql_algebra::SparqlParser::new()
                    .parse_query("SELECT (1 AS ?result) WHERE {}")
                    .expect("parse trivial function body"),
            ),
            kind: crate::user_fn::UserFnBody::Select,
            return_constraint: crate::user_fn::TypeConstraint::default(),
        }
    }

    #[test]
    fn native_stable_custom_is_parallel_safe() {
        let mut reg = UserFunctionRegistry::new();
        reg.register_native(
            CUSTOM_NATIVE_IRI,
            crate::user_fn::Arity::Exact(0),
            Volatility::Stable,
            trivial_native_body(),
        );
        assert!(is_parallel_safe(&custom_call(CUSTOM_NATIVE_IRI), fns(&reg)));
    }

    #[test]
    fn native_volatile_custom_is_parallel_unsafe() {
        let mut reg = UserFunctionRegistry::new();
        reg.register_native(
            CUSTOM_NATIVE_IRI,
            crate::user_fn::Arity::Exact(0),
            Volatility::Volatile,
            trivial_native_body(),
        );
        assert!(!is_parallel_safe(
            &custom_call(CUSTOM_NATIVE_IRI),
            fns(&reg)
        ));
    }

    #[test]
    fn sparql_bodied_custom_is_parallel_unsafe() {
        let mut reg = UserFunctionRegistry::new();
        reg.insert(CUSTOM_SPARQL_IRI, trivial_sparql_function());
        assert!(!is_parallel_safe(
            &custom_call(CUSTOM_SPARQL_IRI),
            fns(&reg)
        ));
    }

    #[test]
    fn unknown_custom_without_registry_stays_safe() {
        assert!(is_parallel_safe(&custom_call(CUSTOM_UNKNOWN_IRI), NONE));

        let reg = UserFunctionRegistry::new();
        assert!(is_parallel_safe(
            &custom_call(CUSTOM_UNKNOWN_IRI),
            fns(&reg)
        ));
    }

    // ---- is_parallel_safe_pattern: property functions ----------------------

    const RELATION_IRI: &str = "http://example.org/ns#split";

    /// A one-subject/one-object property-function call node.
    fn property_function_call() -> GraphPattern {
        GraphPattern::PropertyFunction(purrdf_sparql_algebra::PropertyFunctionCall {
            iri: RELATION_IRI.to_owned(),
            subject_args: vec![purrdf_sparql_algebra::TermPattern::Variable(
                purrdf_sparql_algebra::Variable::new("s"),
            )],
            object_args: vec![purrdf_sparql_algebra::TermPattern::Variable(
                purrdf_sparql_algebra::Variable::new("o"),
            )],
        })
    }

    /// A relation whose declared volatility is `volatility`, with no rows.
    #[derive(Debug)]
    struct DeclaredRelation {
        volatility: Volatility,
        modes: [purrdf_core::binding_pattern::BindingPattern; 1],
    }

    impl DeclaredRelation {
        fn new(volatility: Volatility) -> Self {
            Self {
                volatility,
                modes: [crate::property_fn::PfArity::new(1, 1).all_free_mode()],
            }
        }
    }

    impl crate::property_fn::PropertyFunction for DeclaredRelation {
        fn volatility(&self) -> Volatility {
            self.volatility
        }

        fn arity(&self) -> crate::property_fn::PfArity {
            crate::property_fn::PfArity::new(1, 1)
        }

        fn modes(&self) -> &[purrdf_core::binding_pattern::BindingPattern] {
            &self.modes
        }

        fn rows_per_invocation(&self, _mode: purrdf_core::binding_pattern::BindingPattern) -> u64 {
            0
        }

        fn open(
            &self,
            _args: &crate::property_fn::PfArgs<'_>,
            _ceiling: Option<u64>,
        ) -> Result<Box<dyn crate::property_fn::PfCursor>, EvalError> {
            Err(EvalError::function("not invoked by the safety gate"))
        }
    }

    fn registry_with(volatility: Volatility) -> PropertyFunctionRegistry {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(
            RELATION_IRI,
            std::sync::Arc::new(DeclaredRelation::new(volatility)),
        );
        registry
    }

    #[test]
    fn stable_property_function_is_parallel_safe() {
        let registry = registry_with(Volatility::Stable);
        assert!(is_parallel_safe_pattern(
            &property_function_call(),
            relations(&registry)
        ));
    }

    #[test]
    fn volatile_property_function_is_parallel_unsafe() {
        let registry = registry_with(Volatility::Volatile);
        assert!(!is_parallel_safe_pattern(
            &property_function_call(),
            relations(&registry)
        ));
    }

    #[test]
    fn property_function_without_a_registry_is_parallel_unsafe() {
        // Both absences: no registry at all, and a registry that does not resolve the
        // IRI. An unknown relation's volatility is unknown, so the gate refuses.
        assert!(!is_parallel_safe_pattern(&property_function_call(), NONE));
        let empty = PropertyFunctionRegistry::new();
        assert!(!is_parallel_safe_pattern(
            &property_function_call(),
            relations(&empty)
        ));
    }

    #[test]
    fn an_unsafe_property_function_taints_the_enclosing_pattern() {
        // The classification must survive being nested: a `UNION` arm containing the
        // call is unsafe, and so is an `EXISTS` whose inner pattern contains it.
        let volatile = registry_with(Volatility::Volatile);
        let union = GraphPattern::Union {
            left: Box::new(GraphPattern::Bgp {
                patterns: Vec::new(),
            }),
            right: Box::new(property_function_call()),
        };
        assert!(!is_parallel_safe_pattern(&union, relations(&volatile)));

        let exists = Expression::Exists(Box::new(property_function_call()));
        assert!(!is_parallel_safe(&exists, relations(&volatile)));

        let stable = registry_with(Volatility::Stable);
        assert!(is_parallel_safe_pattern(&union, relations(&stable)));
        assert!(is_parallel_safe(&exists, relations(&stable)));
    }

    // ---- Does the pattern walk descend into EXISTS pattern positions, not just
    // an EXISTS's own top-level node? ----
    //
    // `unsafe_inside_nested_exists_filter_is_detected` and
    // `an_unsafe_property_function_taints_the_enclosing_pattern` above already
    // answer this — `pattern_reaches_unsafe_builtin`'s
    // `ExpressionPart::Exists(pattern) => pattern_reaches_unsafe_builtin(pattern,
    // ..)` arm recurses into the inner pattern exactly like any other pattern, so
    // the walk already descends (the variable-widening fix `expr_vars`'s `Exists`
    // arm needed elsewhere in this crate does not have a sibling gap in THIS
    // walk's decomposition, which is a boolean OR over `visit_pattern_parts`/
    // `visit_expression_parts`, not a variable SET). These two tests exist purely
    // as non-regression pins, nested one level deeper (behind a `Join`) than
    // either test above, so a future change that stops the descent — even one
    // that only breaks multi-hop recursion — trips a test immediately.
    #[test]
    fn parallel_safety_descends_into_exists_pattern_positions() {
        let vp = |n: &str| {
            purrdf_sparql_algebra::TermPattern::Variable(purrdf_sparql_algebra::Variable::new(n))
        };
        let pred = |iri: &str| {
            purrdf_sparql_algebra::NamedNodePattern::NamedNode(NamedNode::new_unchecked(iri))
        };
        let safe_bgp = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: vp("s"),
                predicate: pred("http://ex/p"),
                object: vp("o"),
            }],
        };

        // `BIND(RAND() AS ?r)`, two pattern levels below the EXISTS inner's own top
        // node (behind a `Join`) — the ONLY unsafe construct anywhere in the tree.
        let bind_rand = GraphPattern::Extend {
            inner: Box::new(GraphPattern::Bgp {
                patterns: Vec::new(),
            }),
            variable: purrdf_sparql_algebra::Variable::new("r"),
            expression: call(Function::Rand, vec![]),
        };
        let buried_builtin = GraphPattern::Join {
            left: Box::new(safe_bgp.clone()),
            right: Box::new(bind_rand),
        };
        let exists_builtin = Expression::Exists(Box::new(buried_builtin));
        assert!(
            !is_parallel_safe(&exists_builtin, NONE),
            "RAND() buried two pattern levels below the EXISTS inner (behind a \
             Join) must still taint the enclosing expression"
        );

        // Same shape, an unsafe PROPERTY FUNCTION instead of a stateful builtin —
        // buried the same two levels down, behind the same kind of `Join`.
        let buried_relation = GraphPattern::Join {
            left: Box::new(safe_bgp),
            right: Box::new(property_function_call()),
        };
        let volatile = registry_with(Volatility::Volatile);
        let exists_relation = Expression::Exists(Box::new(buried_relation));
        assert!(
            !is_parallel_safe(&exists_relation, relations(&volatile)),
            "an unsafe property-function call buried behind a Join inside the \
             EXISTS inner must still taint the enclosing expression"
        );
    }

    /// The positive twin: the SAME nested shape (a `Join` two levels below the
    /// EXISTS inner) with no unsafe construct anywhere stays parallel-safe.
    #[test]
    fn parallel_safety_pure_exists_inner_stays_safe() {
        let vp = |n: &str| {
            purrdf_sparql_algebra::TermPattern::Variable(purrdf_sparql_algebra::Variable::new(n))
        };
        let pred = |iri: &str| {
            purrdf_sparql_algebra::NamedNodePattern::NamedNode(NamedNode::new_unchecked(iri))
        };
        let safe_bgp = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: vp("s"),
                predicate: pred("http://ex/p"),
                object: vp("o"),
            }],
        };
        let bind_pure = GraphPattern::Extend {
            inner: Box::new(GraphPattern::Bgp {
                patterns: Vec::new(),
            }),
            variable: purrdf_sparql_algebra::Variable::new("r"),
            expression: call(
                Function::Abs,
                vec![Expression::Variable(purrdf_sparql_algebra::Variable::new(
                    "s",
                ))],
            ),
        };
        let pure = GraphPattern::Join {
            left: Box::new(safe_bgp),
            right: Box::new(bind_pure),
        };
        let exists = Expression::Exists(Box::new(pure));
        assert!(
            is_parallel_safe(&exists, NONE),
            "a fully-pure EXISTS inner (no stateful builtin, no unsafe relation \
             anywhere in the nested pattern) must stay parallel-safe"
        );
    }

    // ---- aggregate_is_unsafe: custom-aggregate fork-gate volatility --------

    const AGG_IRI: &str = "http://example.org/agg#custom";

    /// A no-op accumulator — never actually driven by these gate-classification
    /// tests, which read only [`crate::agg_fn::CustomAggregate::volatility`].
    struct NoOpAccumulator;
    impl crate::agg_fn::AggregateAccumulator for NoOpAccumulator {
        fn step(&mut self, _args: &[TermValue]) -> Result<(), EvalError> {
            Ok(())
        }
        fn combine(
            &mut self,
            _other: Box<dyn crate::agg_fn::AggregateAccumulator>,
        ) -> Result<(), EvalError> {
            Ok(())
        }
        fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
            self
        }
        fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
            Ok(None)
        }
    }

    /// A custom aggregate whose declared volatility is `volatility`.
    struct DeclaredAggregate {
        volatility: Volatility,
    }

    impl crate::agg_fn::CustomAggregate for DeclaredAggregate {
        fn arity(&self) -> crate::user_fn::Arity {
            crate::user_fn::Arity::Exact(1)
        }
        fn volatility(&self) -> Volatility {
            self.volatility
        }
        fn algebraic_class(&self) -> crate::agg_fn::AlgebraicClass {
            crate::agg_fn::AlgebraicClass::Commutative
        }
        fn state_bound(&self) -> u64 {
            0
        }
        fn init(
            &self,
            _scalarvals: &[(String, TermValue)],
        ) -> Box<dyn crate::agg_fn::AggregateAccumulator> {
            Box::new(NoOpAccumulator)
        }
    }

    fn agg_registry_with(volatility: Volatility) -> AggregateRegistry {
        let mut registry = AggregateRegistry::new();
        registry.register(
            AGG_IRI,
            std::sync::Arc::new(DeclaredAggregate { volatility }),
        );
        registry
    }

    #[test]
    fn stable_custom_aggregate_is_parallel_safe() {
        let registry = agg_registry_with(Volatility::Stable);
        assert!(!aggregate_is_unsafe(AGG_IRI, &registry));
    }

    #[test]
    fn volatile_custom_aggregate_is_parallel_unsafe() {
        let registry = agg_registry_with(Volatility::Volatile);
        assert!(aggregate_is_unsafe(AGG_IRI, &registry));
    }

    /// `aggregate_is_unsafe` takes `&AggregateRegistry`, never
    /// `Option<&AggregateRegistry>` — there is no "absent registry" call this could
    /// exercise as a case DISTINCT from an empty one, which makes "a `None`-shaped
    /// call and a `Some(&AggregateRegistry::new())`-shaped call behave identically"
    /// structurally impossible to violate rather than merely tested: the type
    /// system admits only the one spelling. [`AggregateRegistry::EMPTY`]
    /// and a freshly built, still-empty [`AggregateRegistry::new`] both resolve
    /// `AGG_IRI` to nothing, so the gate refuses under either — the SAME
    /// conservative treatment an unresolved property function gets
    /// (`property_function_without_a_registry_is_parallel_unsafe`), never the
    /// scalar-function treatment (`unknown_custom_without_registry_stays_safe`).
    #[test]
    fn custom_aggregate_without_a_registry_is_parallel_unsafe() {
        assert!(aggregate_is_unsafe(AGG_IRI, &AggregateRegistry::EMPTY));
        let empty = AggregateRegistry::new();
        assert!(aggregate_is_unsafe(AGG_IRI, &empty));
    }

    // ---- par_chunk_map ----------------------------------------------------

    #[test]
    fn par_chunk_map_matches_sequential_one_chunk() {
        // A chunk size far bigger than the input: everything lands in a
        // single chunk, exercising the "one chunk" boundary.
        let _parallel_guard = force_parallel_for_test(true);
        let _chunk_guard = force_chunk_size_for_test(1000);
        let items: Vec<usize> = (0..64).collect();
        let result = par_chunk_map(&items, |acc, &item| {
            if item % 7 != 0 {
                std::thread::yield_now();
            }
            acc.push(item * 2);
        });
        let expected: Vec<usize> = (0..64).map(|i| i * 2).collect();
        assert_eq!(result, expected);
    }

    #[test]
    fn par_chunk_map_matches_sequential_many_chunks() {
        // A tiny chunk size over a larger input spans many chunk boundaries
        // (100 items / chunk size 7 ⇒ 15 chunks, several ragged).
        let _parallel_guard = force_parallel_for_test(true);
        let _chunk_guard = force_chunk_size_for_test(7);
        let items: Vec<usize> = (0..100).collect();
        let result = par_chunk_map(&items, |acc, &item| {
            if item % 3 == 0 {
                std::thread::yield_now();
            }
            acc.push(item * 2);
        });
        let expected: Vec<usize> = (0..100).map(|i| i * 2).collect();
        assert_eq!(result, expected);
    }

    #[test]
    fn par_chunk_map_forced_sequential_matches_forced_parallel() {
        let items: Vec<usize> = (0..100).collect();
        let push = |acc: &mut Vec<usize>, &item: &usize| acc.push(item * 2);

        let sequential = {
            let _guard = force_parallel_for_test(false);
            par_chunk_map(&items, push)
        };
        let parallel = {
            let _parallel_guard = force_parallel_for_test(true);
            let _chunk_guard = force_chunk_size_for_test(9);
            par_chunk_map(&items, push)
        };
        assert_eq!(sequential, parallel);
    }

    // ---- par_chunk_try_map_init ---------------------------------------------

    #[test]
    fn par_chunk_try_map_init_flattens_in_index_order_one_chunk() {
        let _parallel_guard = force_parallel_for_test(true);
        let _chunk_guard = force_chunk_size_for_test(1000);
        let init_calls = std::sync::atomic::AtomicUsize::new(0);
        let items: Vec<usize> = (0..64).collect();
        let result = par_chunk_try_map_init(
            &items,
            || {
                init_calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                0_u64 // per-chunk state: unused counter, just proves init ran.
            },
            |_state, acc, &item| {
                if item % 7 != 0 {
                    std::thread::yield_now();
                }
                acc.push(vec![Some(SolutionTerm::Existing(TermId::from_index(
                    item as u32,
                )))]);
                Ok(())
            },
        )
        .expect("no errors");
        let indices: Vec<u32> = result
            .iter()
            .map(|row| match row[0] {
                Some(SolutionTerm::Existing(id)) => id.index() as u32,
                _ => unreachable!(),
            })
            .collect();
        let expected: Vec<u32> = (0..64).collect();
        assert_eq!(indices, expected);
        // A single chunk ⇒ `init` runs exactly once.
        assert_eq!(init_calls.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[test]
    fn par_chunk_try_map_init_flattens_in_index_order_many_chunks() {
        let _parallel_guard = force_parallel_for_test(true);
        let _chunk_guard = force_chunk_size_for_test(7);
        let init_calls = std::sync::atomic::AtomicUsize::new(0);
        let items: Vec<usize> = (0..100).collect();
        let result = par_chunk_try_map_init(
            &items,
            || {
                init_calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                0_u64
            },
            |_state, acc, &item| {
                if item % 3 == 0 {
                    std::thread::yield_now();
                }
                acc.push(vec![Some(SolutionTerm::Existing(TermId::from_index(
                    item as u32,
                )))]);
                Ok(())
            },
        )
        .expect("no errors");
        let indices: Vec<u32> = result
            .iter()
            .map(|row| match row[0] {
                Some(SolutionTerm::Existing(id)) => id.index() as u32,
                _ => unreachable!(),
            })
            .collect();
        let expected: Vec<u32> = (0..100).collect();
        assert_eq!(indices, expected);
        // 100 items / chunk size 7 ⇒ 15 chunks, so `init` ran more than once but
        // never once per item.
        let inits = init_calls.load(std::sync::atomic::Ordering::Relaxed);
        assert!((1..=15).contains(&inits), "inits={inits}");
    }

    #[test]
    fn par_chunk_try_map_init_forced_sequential_runs_init_exactly_once() {
        let _guard = force_parallel_for_test(false);
        let init_calls = std::sync::atomic::AtomicUsize::new(0);
        let items: Vec<usize> = (0..64).collect();
        let result = par_chunk_try_map_init(
            &items,
            || {
                init_calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                0_u64
            },
            |_state, acc, &item| {
                acc.push(vec![Some(SolutionTerm::Existing(TermId::from_index(
                    item as u32,
                )))]);
                Ok(())
            },
        )
        .expect("no errors");
        let indices: Vec<u32> = result
            .iter()
            .map(|row| match row[0] {
                Some(SolutionTerm::Existing(id)) => id.index() as u32,
                _ => unreachable!(),
            })
            .collect();
        let expected: Vec<u32> = (0..64).collect();
        assert_eq!(indices, expected);
        assert_eq!(init_calls.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[test]
    fn par_chunk_try_map_init_surfaces_the_lower_chunk_indexed_error() {
        // Chunk size 5 over 40 items ⇒ 8 chunks; index 22 (chunk 4) and index 6
        // (chunk 1) both error, index 22's chunk is nudged to finish first via
        // `yield_now`, but the chunk-index-ordered reduce must still surface
        // chunk 1's (index 6's) error, never chunk 4's.
        let _parallel_guard = force_parallel_for_test(true);
        let _chunk_guard = force_chunk_size_for_test(5);
        let items: Vec<usize> = (0..40).collect();
        let result: Result<Vec<Solution>, EvalError> = par_chunk_try_map_init(
            &items,
            || (),
            |(), _acc, &i| {
                if i == 22 {
                    std::thread::yield_now();
                    return Err(EvalError::internal("error at 22"));
                }
                if i == 6 {
                    return Err(EvalError::internal("error at 6"));
                }
                Ok(())
            },
        );
        let err = result.unwrap_err();
        assert_eq!(err, EvalError::internal("error at 6"));
    }

    // ---- par_retain -------------------------------------------------------

    #[test]
    fn par_retain_preserves_order_forced_parallel() {
        let _guard = force_parallel_for_test(true);
        let items: Vec<usize> = (0..64).collect();
        let kept = par_retain(&items, |&i| i % 3 == 0);
        let expected: Vec<usize> = (0..64).filter(|i| i % 3 == 0).collect();
        assert_eq!(kept, expected);
    }

    #[test]
    fn par_retain_preserves_order_forced_sequential() {
        let _guard = force_parallel_for_test(false);
        let items: Vec<usize> = (0..64).collect();
        let kept = par_retain(&items, |&i| i % 3 == 0);
        let expected: Vec<usize> = (0..64).filter(|i| i % 3 == 0).collect();
        assert_eq!(kept, expected);
    }

    // ---- fork_for_worker + portable_row/reintern_portable_row -----------------

    fn lit(s: &str) -> TermValue {
        TermValue::Literal {
            lexical_form: s.to_owned(),
            datatype: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
            language: None,
            direction: None,
        }
    }

    #[test]
    fn portable_row_round_trips_fresh_and_pre_fork_and_existing_and_none() {
        let ds = RdfDatasetBuilder::new()
            .freeze()
            .expect("freeze empty dataset");
        let mut parent = crate::eval::EvalCtx::new(&ds);

        // Seed the PARENT scratch with an already-minted value BEFORE forking, so
        // an input row carrying that `Computed` id (as a real parallel worker's
        // input rows would) is something the fork must be able to resolve, and
        // `portable_row` must classify it as `Parent` (sid < base), not `Fresh`.
        let pre_fork_value = lit("already minted");
        let pre_fork_term = parent.scratch.intern(&ds, pre_fork_value.clone());
        let base = parent.scratch.computed_count();

        let mut child = parent.fork_for_worker();
        assert_eq!(
            child.scratch.value_of(&ds, pre_fork_term),
            pre_fork_value,
            "child must resolve a Computed id it inherited from the parent scratch"
        );

        // The child mints a NEW value (not known to the parent at fork time) —
        // `portable_row` must classify this as `Fresh` (sid >= base).
        let fresh_value = lit("hello parallel");
        let fresh_term = child.scratch.intern(&ds, fresh_value.clone());
        let row: Solution = smallvec::smallvec![None, Some(pre_fork_term), Some(fresh_term)];

        let prow = portable_row(&child.scratch, base, &row);
        assert_eq!(prow[0], None);
        assert_eq!(prow[1], Some(PortableTerm::Parent(pre_fork_term)));
        assert_eq!(prow[2], Some(PortableTerm::Fresh(fresh_value.clone())));

        let reinterned = reintern_portable_row(&mut parent.scratch, &ds, prow);
        assert_eq!(reinterned[0], None);
        // The pre-fork term passes through unchanged and still resolves in the
        // parent (which already owned it).
        assert_eq!(reinterned[1], Some(pre_fork_term));
        assert_eq!(
            parent.scratch.value_of(&ds, reinterned[1].unwrap()),
            pre_fork_value
        );
        // The child's fresh mint is folded into the parent's id space and
        // resolves to the same value there.
        let reinterned_fresh = reinterned[2].expect("cell present");
        assert_eq!(parent.scratch.value_of(&ds, reinterned_fresh), fresh_value);
    }

    #[test]
    fn reintern_portable_row_dedups_two_children_minting_the_same_value() {
        let ds = RdfDatasetBuilder::new()
            .freeze()
            .expect("freeze empty dataset");
        let mut parent = crate::eval::EvalCtx::new(&ds);
        let base = parent.scratch.computed_count();

        let mut child_a = parent.fork_for_worker();
        let mut child_b = parent.fork_for_worker();
        let shared_value = lit("same value from two workers");
        let term_a = child_a.scratch.intern(&ds, shared_value.clone());
        let term_b = child_b.scratch.intern(&ds, shared_value);

        let row_a: Solution = smallvec::smallvec![Some(term_a)];
        let row_b: Solution = smallvec::smallvec![Some(term_b)];
        let prow_a = portable_row(&child_a.scratch, base, &row_a);
        let prow_b = portable_row(&child_b.scratch, base, &row_b);

        let reinterned_a = reintern_portable_row(&mut parent.scratch, &ds, prow_a);
        let reinterned_b = reintern_portable_row(&mut parent.scratch, &ds, prow_b);

        assert_eq!(
            reinterned_a[0], reinterned_b[0],
            "two workers minting the same fresh value must reintern to the same parent id"
        );
    }
}
