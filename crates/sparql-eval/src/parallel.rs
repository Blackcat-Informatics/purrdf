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
use purrdf_sparql_algebra::{
    AggregateExpression, Expression, Function, GraphPattern, OrderExpression,
};

use crate::error::EvalError;
use crate::scratch::{ScratchInterner, SolutionTerm};
use crate::solution::Solution;
use crate::user_fn::{UserFunctionRegistry, Volatility};

/// Rows/groups at or below this stay sequential (thread spin-up would dominate
/// the work for small inputs). Initial value; Task 7's bench tunes it.
pub(crate) const PARALLEL_MIN_ROWS: usize = 1024;

/// Floor on a chunk's item count: below this, splitting further would hand
/// rayon workers slivers dominated by per-chunk overhead (the fork, the `Vec`
/// staging) rather than real work. Mirrors the byte-based floor
/// `purrdf_rdf::native_codecs::text_parse::PARALLEL_MIN_CHUNK_BYTES` applies to
/// the parser's chunk geometry, just in item-count terms.
const PARALLEL_MIN_CHUNK_ITEMS: usize = 16;

/// The chunk size for a [`par_chunk_map`]/[`par_chunk_try_map_init`] run over
/// `len` items: aim for roughly four chunks per rayon worker thread, so
/// work-stealing has enough slices to balance ragged per-item costs (a BGP
/// pattern whose candidate count varies row to row, a GROUP BY group whose
/// size varies group to group) without handing every thread only one
/// coarse-grained slice. Clamped below by [`PARALLEL_MIN_CHUNK_ITEMS`] so a
/// small-but-still-parallel input (just over [`PARALLEL_MIN_ROWS`]) on a
/// many-thread machine never degenerates into chunks of a handful of items
/// each — mirrors the parser's `len / (threads * 4)` geometry.
fn chunk_size_for(len: usize) -> usize {
    #[cfg(test)]
    if let Some(forced) = FORCE_CHUNK_SIZE.with(std::cell::Cell::get) {
        return forced.max(1);
    }
    let threads = rayon::current_num_threads().max(1);
    (len / (threads * 4).max(1)).max(PARALLEL_MIN_CHUNK_ITEMS)
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
    /// Test-only override for [`chunk_size_for`]'s result, so a test can pin an
    /// exact chunk size (hence an exact chunk count) regardless of
    /// `rayon::current_num_threads()`, which varies by machine/CI runner.
    /// Never consulted outside `cfg(test)`.
    static FORCE_CHUNK_SIZE: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

/// Force [`chunk_size_for`] to always return `size` for the current thread
/// until the returned guard is dropped (restores the prior override).
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

/// Whether a batch of `work_items` (rows, groups, branches) should run in
/// parallel rather than sequentially. Small inputs stay sequential because
/// rayon thread hand-off cost would dominate the actual work.
pub(crate) fn should_parallelize(work_items: usize) -> bool {
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
/// `registry` is the caller's [`UserFunctionRegistry`] (`ctx.user_functions`), if
/// any: it is consulted only when the walk reaches a [`Function::Custom`] call —
/// see [`function_is_unsafe`] for exactly how a native-vs-SPARQL-bodied callee is
/// classified. `None` (no registry configured) makes every `Custom` IRI resolve
/// to the deterministic XSD-cast/hard-error fallback, hence safe.
pub(crate) fn is_parallel_safe(expr: &Expression, registry: Option<&UserFunctionRegistry>) -> bool {
    !expr_reaches_unsafe_builtin(expr, registry)
}

/// Whether `pattern` (recursively) is safe to evaluate under the fork-join
/// parallel model — the pattern-level twin of [`is_parallel_safe`], for callers
/// (e.g. `UNION`) that must gate a whole sub-pattern rather than a single
/// expression. Exposes the same walk [`is_parallel_safe`] already runs
/// internally for `EXISTS`. `registry` is threaded through exactly as in
/// [`is_parallel_safe`].
pub(crate) fn is_parallel_safe_pattern(
    pattern: &GraphPattern,
    registry: Option<&UserFunctionRegistry>,
) -> bool {
    !pattern_reaches_unsafe_builtin(pattern, registry)
}

/// `true` iff `expr` (recursively) reaches an unsafe builtin — see
/// [`is_parallel_safe`].
fn expr_reaches_unsafe_builtin(expr: &Expression, registry: Option<&UserFunctionRegistry>) -> bool {
    match expr {
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Variable(_)
        | Expression::Bound(_) => false,
        Expression::Or(a, b)
        | Expression::And(a, b)
        | Expression::Equal(a, b)
        | Expression::SameTerm(a, b)
        | Expression::Greater(a, b)
        | Expression::GreaterOrEqual(a, b)
        | Expression::Less(a, b)
        | Expression::LessOrEqual(a, b)
        | Expression::Add(a, b)
        | Expression::Subtract(a, b)
        | Expression::Multiply(a, b)
        | Expression::Divide(a, b) => {
            expr_reaches_unsafe_builtin(a, registry) || expr_reaches_unsafe_builtin(b, registry)
        }
        Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
            expr_reaches_unsafe_builtin(a, registry)
        }
        Expression::In(head, list) => {
            expr_reaches_unsafe_builtin(head, registry)
                || list
                    .iter()
                    .any(|e| expr_reaches_unsafe_builtin(e, registry))
        }
        Expression::If(a, b, c) => {
            expr_reaches_unsafe_builtin(a, registry)
                || expr_reaches_unsafe_builtin(b, registry)
                || expr_reaches_unsafe_builtin(c, registry)
        }
        Expression::Coalesce(list) => list
            .iter()
            .any(|e| expr_reaches_unsafe_builtin(e, registry)),
        Expression::FunctionCall(f, args) => {
            function_is_unsafe(f, registry)
                || args
                    .iter()
                    .any(|e| expr_reaches_unsafe_builtin(e, registry))
        }
        Expression::Exists(pattern) => pattern_reaches_unsafe_builtin(pattern, registry),
    }
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
fn function_is_unsafe(f: &Function, registry: Option<&UserFunctionRegistry>) -> bool {
    match f {
        Function::Rand | Function::Uuid | Function::StrUuid | Function::BNode => true,
        Function::Purrdf(call) => matches!(
            call.fn_kind,
            purrdf_sparql_algebra::PurrdfFn::ListSlice
                | purrdf_sparql_algebra::PurrdfFn::ListConcat
        ),
        Function::Custom(iri) => {
            let Some(reg) = registry else { return false };
            if let Some(native) = reg.resolve_native(iri.as_str()) {
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
            reg.resolve(iri.as_str()).is_some()
        }
        _ => false,
    }
}

/// `true` iff `pattern` (recursively) reaches an unsafe builtin through any
/// expression-bearing variant — see [`is_parallel_safe`].
fn pattern_reaches_unsafe_builtin(
    pattern: &GraphPattern,
    registry: Option<&UserFunctionRegistry>,
) -> bool {
    match pattern {
        GraphPattern::Bgp { .. } | GraphPattern::Path { .. } | GraphPattern::Values { .. } => false,
        GraphPattern::Join { left, right }
        | GraphPattern::Lateral { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right } => {
            pattern_reaches_unsafe_builtin(left, registry)
                || pattern_reaches_unsafe_builtin(right, registry)
        }
        GraphPattern::Graph { inner, .. }
        | GraphPattern::Service { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. }
        | GraphPattern::Project { inner, .. } => pattern_reaches_unsafe_builtin(inner, registry),
        GraphPattern::Filter { expr, inner } => {
            expr_reaches_unsafe_builtin(expr, registry)
                || pattern_reaches_unsafe_builtin(inner, registry)
        }
        GraphPattern::Extend {
            inner, expression, ..
        } => {
            expr_reaches_unsafe_builtin(expression, registry)
                || pattern_reaches_unsafe_builtin(inner, registry)
        }
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            pattern_reaches_unsafe_builtin(left, registry)
                || pattern_reaches_unsafe_builtin(right, registry)
                || expression
                    .as_ref()
                    .is_some_and(|e| expr_reaches_unsafe_builtin(e, registry))
        }
        GraphPattern::OrderBy { inner, expression } => {
            pattern_reaches_unsafe_builtin(inner, registry)
                || expression.iter().any(|oe| match oe {
                    OrderExpression::Asc(e) | OrderExpression::Desc(e) => {
                        expr_reaches_unsafe_builtin(e, registry)
                    }
                })
        }
        GraphPattern::Group {
            inner, aggregates, ..
        } => {
            pattern_reaches_unsafe_builtin(inner, registry)
                || aggregates.iter().any(|(_, agg)| match agg {
                    AggregateExpression::CountStar { .. } => false,
                    AggregateExpression::FunctionCall { expression, .. } => {
                        expr_reaches_unsafe_builtin(expression, registry)
                    }
                })
        }
    }
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

    // ---- is_parallel_safe ----------------------------------------------------

    fn call(f: Function, args: Vec<Expression>) -> Expression {
        Expression::FunctionCall(f, args)
    }

    #[test]
    fn plain_arithmetic_and_regex_are_safe() {
        let arith = Expression::Add(
            Box::new(Expression::Literal(Literal::new_simple("1"))),
            Box::new(Expression::Literal(Literal::new_simple("2"))),
        );
        assert!(is_parallel_safe(&arith, None));

        let regex = call(
            Function::Regex,
            vec![
                Expression::Variable(purrdf_sparql_algebra::Variable::new("x")),
                Expression::Literal(Literal::new_simple("^a")),
            ],
        );
        assert!(is_parallel_safe(&regex, None));
    }

    #[test]
    fn rand_uuid_struuid_bnode_are_unsafe() {
        assert!(!is_parallel_safe(&call(Function::Rand, vec![]), None));
        assert!(!is_parallel_safe(&call(Function::Uuid, vec![]), None));
        assert!(!is_parallel_safe(&call(Function::StrUuid, vec![]), None));
        assert!(!is_parallel_safe(&call(Function::BNode, vec![]), None));
        assert!(!is_parallel_safe(
            &call(
                Function::BNode,
                vec![Expression::Variable(purrdf_sparql_algebra::Variable::new(
                    "x"
                ))]
            ),
            None
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
            None
        ));
        assert!(!is_parallel_safe(
            &mk(PurrdfFn::ListConcat, "http://ex/listConcat"),
            None
        ));
        assert!(is_parallel_safe(
            &mk(PurrdfFn::ListLength, "http://ex/listLength"),
            None
        ));
        assert!(is_parallel_safe(
            &mk(PurrdfFn::HeldIn, "http://ex/heldIn"),
            None
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
        assert!(!is_parallel_safe(&in_if, None));

        let in_coalesce = Expression::Coalesce(vec![safe.clone(), rand.clone()]);
        assert!(!is_parallel_safe(&in_coalesce, None));

        let in_fn_args = call(Function::Concat, vec![safe, rand]);
        assert!(!is_parallel_safe(&in_fn_args, None));
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
        assert!(!is_parallel_safe(&exists, None));

        // Sanity: the same shape without RAND() is safe.
        let inner_bgp2 = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: vp("s"),
                predicate: pred("http://ex/p"),
                object: vp("o"),
            }],
        };
        let safe_exists = Expression::Exists(Box::new(inner_bgp2));
        assert!(is_parallel_safe(&safe_exists, None));
    }

    // ---- is_parallel_safe: Function::Custom / UserFunctionRegistry ----------

    const CUSTOM_NATIVE_IRI: &str = "http://example.org/ns#customNative";
    const CUSTOM_SPARQL_IRI: &str = "http://example.org/ns#customSparql";
    const CUSTOM_UNKNOWN_IRI: &str = "http://example.org/ns#customUnknown";

    fn custom_call(iri: &str) -> Expression {
        call(Function::Custom(NamedNode::new_unchecked(iri)), Vec::new())
    }

    fn trivial_native_body() -> crate::user_fn::NativeFnBody {
        std::sync::Arc::new(|_args: &[TermValue]| {
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
        assert!(is_parallel_safe(
            &custom_call(CUSTOM_NATIVE_IRI),
            Some(&reg)
        ));
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
            Some(&reg)
        ));
    }

    #[test]
    fn sparql_bodied_custom_is_parallel_unsafe() {
        let mut reg = UserFunctionRegistry::new();
        reg.insert(CUSTOM_SPARQL_IRI, trivial_sparql_function());
        assert!(!is_parallel_safe(
            &custom_call(CUSTOM_SPARQL_IRI),
            Some(&reg)
        ));
    }

    #[test]
    fn unknown_custom_without_registry_stays_safe() {
        assert!(is_parallel_safe(&custom_call(CUSTOM_UNKNOWN_IRI), None));

        let reg = UserFunctionRegistry::new();
        assert!(is_parallel_safe(
            &custom_call(CUSTOM_UNKNOWN_IRI),
            Some(&reg)
        ));
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
