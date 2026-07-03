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
//! 2. **Join.** [`par_try_flat_map_init`]/[`par_flat_map`]/[`par_retain`] run the
//!    workers via rayon's *indexed* `collect` (never `par_sort`/`par_bridge`,
//!    which are not order-stable) and then reduce strictly in source-index
//!    order: successes flatten in index order and the first `Err` **by index**
//!    wins, regardless of which worker finished first.
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
//! content.

use purrdf_core::{RdfDataset, TermValue};
use purrdf_sparql_algebra::{
    AggregateExpression, Expression, Function, GraphPattern, OrderExpression,
};

use crate::error::EvalError;
use crate::scratch::{ScratchInterner, SolutionTerm};
use crate::solution::Solution;

/// Rows/groups at or below this stay sequential (thread spin-up would dominate
/// the work for small inputs). Initial value; Task 7's bench tunes it.
pub(crate) const PARALLEL_MIN_ROWS: usize = 1024;

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
pub(crate) fn is_parallel_safe(expr: &Expression) -> bool {
    !expr_reaches_unsafe_builtin(expr)
}

/// Whether `pattern` (recursively) is safe to evaluate under the fork-join
/// parallel model — the pattern-level twin of [`is_parallel_safe`], for callers
/// (e.g. `UNION`) that must gate a whole sub-pattern rather than a single
/// expression. Exposes the same walk [`is_parallel_safe`] already runs
/// internally for `EXISTS`.
pub(crate) fn is_parallel_safe_pattern(pattern: &GraphPattern) -> bool {
    !pattern_reaches_unsafe_builtin(pattern)
}

/// `true` iff `expr` (recursively) reaches an unsafe builtin — see
/// [`is_parallel_safe`].
fn expr_reaches_unsafe_builtin(expr: &Expression) -> bool {
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
            expr_reaches_unsafe_builtin(a) || expr_reaches_unsafe_builtin(b)
        }
        Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
            expr_reaches_unsafe_builtin(a)
        }
        Expression::In(head, list) => {
            expr_reaches_unsafe_builtin(head) || list.iter().any(expr_reaches_unsafe_builtin)
        }
        Expression::If(a, b, c) => {
            expr_reaches_unsafe_builtin(a)
                || expr_reaches_unsafe_builtin(b)
                || expr_reaches_unsafe_builtin(c)
        }
        Expression::Coalesce(list) => list.iter().any(expr_reaches_unsafe_builtin),
        Expression::FunctionCall(f, args) => {
            function_is_unsafe(f) || args.iter().any(expr_reaches_unsafe_builtin)
        }
        Expression::Exists(pattern) => pattern_reaches_unsafe_builtin(pattern),
    }
}

/// Whether `f` is itself one of the stateful-mint builtins (see
/// [`is_parallel_safe`]'s doc comment for the full rationale).
fn function_is_unsafe(f: &Function) -> bool {
    match f {
        Function::Rand | Function::Uuid | Function::StrUuid | Function::BNode => true,
        Function::Purrdf(call) => matches!(
            call.fn_kind,
            purrdf_sparql_algebra::PurrdfFn::ListSlice
                | purrdf_sparql_algebra::PurrdfFn::ListConcat
        ),
        _ => false,
    }
}

/// `true` iff `pattern` (recursively) reaches an unsafe builtin through any
/// expression-bearing variant — see [`is_parallel_safe`].
fn pattern_reaches_unsafe_builtin(pattern: &GraphPattern) -> bool {
    match pattern {
        GraphPattern::Bgp { .. } | GraphPattern::Path { .. } | GraphPattern::Values { .. } => false,
        GraphPattern::Join { left, right }
        | GraphPattern::Lateral { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right } => {
            pattern_reaches_unsafe_builtin(left) || pattern_reaches_unsafe_builtin(right)
        }
        GraphPattern::Graph { inner, .. }
        | GraphPattern::Service { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. }
        | GraphPattern::Project { inner, .. } => pattern_reaches_unsafe_builtin(inner),
        GraphPattern::Filter { expr, inner } => {
            expr_reaches_unsafe_builtin(expr) || pattern_reaches_unsafe_builtin(inner)
        }
        GraphPattern::Extend {
            inner, expression, ..
        } => expr_reaches_unsafe_builtin(expression) || pattern_reaches_unsafe_builtin(inner),
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            pattern_reaches_unsafe_builtin(left)
                || pattern_reaches_unsafe_builtin(right)
                || expression.as_ref().is_some_and(expr_reaches_unsafe_builtin)
        }
        GraphPattern::OrderBy { inner, expression } => {
            pattern_reaches_unsafe_builtin(inner)
                || expression.iter().any(|oe| match oe {
                    OrderExpression::Asc(e) | OrderExpression::Desc(e) => {
                        expr_reaches_unsafe_builtin(e)
                    }
                })
        }
        GraphPattern::Group {
            inner, aggregates, ..
        } => {
            pattern_reaches_unsafe_builtin(inner)
                || aggregates.iter().any(|(_, agg)| match agg {
                    AggregateExpression::CountStar { .. } => false,
                    AggregateExpression::FunctionCall { expression, .. } => {
                        expr_reaches_unsafe_builtin(expression)
                    }
                })
        }
    }
}

/// The fork-per-worker sibling of [`par_flat_map`]/[`par_retain`]: instead of one
/// immutable `worker` closure applied per item, each rayon *worker thread* first
/// runs `init` **once** to build its own `S` (e.g. an `EvalCtx::fork_for_worker`
/// child) and then reuses that state across every item it is scheduled, via
/// rayon's `map_init`. This avoids forking a fresh child per row — the fork
/// (cloning the scratch interner, the `exists_inner_cache`, etc.) is real, if
/// cheap, work that should happen once per worker thread, not once per row.
///
/// Internally gated on [`should_parallelize`]: at or below [`PARALLEL_MIN_ROWS`],
/// `init` runs exactly once and every item is folded sequentially over that
/// single state (bit-identical to a hand-written sequential loop — no rayon
/// hand-off, no extra `init` calls); above it, `par_iter().map_init` gives each
/// worker thread its own `S` and the results are collected into an indexed
/// `Vec` first, then reduced strictly in source order: successes flatten in
/// index order and the first `Err` **by index** wins regardless of which
/// worker finished first — so a fast late item can never race ahead of an
/// earlier item's diagnostic (mirrors
/// `purrdf_rdf::native_codecs::text_parse::parse_lines_parallel_with_chunk_size`).
///
/// Generic over the returned element type `R` (not just [`Solution`]): a
/// minting caller (e.g. `eval_extend`, `eval_group`'s per-group compute) returns
/// [`PortableTerm`] rows instead, since the worker's forked child (and its
/// scratch) is gone by the time the caller can re-intern against the parent.
pub(crate) fn par_try_flat_map_init<T, S, R, Init, F>(
    items: &[T],
    init: Init,
    worker: F,
) -> Result<Vec<R>, EvalError>
where
    T: Sync,
    S: Send,
    R: Send,
    Init: Fn() -> S + Sync + Send,
    F: Fn(&mut S, usize, &T) -> Result<Vec<R>, EvalError> + Sync + Send,
{
    if !should_parallelize(items.len()) {
        let mut state = init();
        let mut out = Vec::new();
        for (i, item) in items.iter().enumerate() {
            out.extend(worker(&mut state, i, item)?);
        }
        return Ok(out);
    }

    use rayon::prelude::*;

    let per_item: Vec<Result<Vec<R>, EvalError>> = items
        .par_iter()
        .enumerate()
        .map_init(&init, |state, (i, item)| worker(state, i, item))
        .collect();

    let mut out = Vec::with_capacity(
        per_item
            .iter()
            .map(|r| r.as_ref().map_or(0, Vec::len))
            .sum(),
    );
    for result in per_item {
        out.extend(result?);
    }
    Ok(out)
}

/// The infallible sibling of [`par_try_flat_map_init`]: run `worker` over every
/// item of `items` and flatten the per-item `Vec<R>` results in source-index order.
/// Same internal [`should_parallelize`] gate — sequential at/below the
/// threshold, rayon's indexed `par_iter` above it — so there is exactly one
/// call site per caller and the `#[cfg(test)]` force seam applies uniformly.
pub(crate) fn par_flat_map<T, R, F>(items: &[T], worker: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(usize, &T) -> Vec<R> + Sync + Send,
{
    if !should_parallelize(items.len()) {
        let mut out = Vec::new();
        for (i, item) in items.iter().enumerate() {
            out.extend(worker(i, item));
        }
        return out;
    }

    use rayon::prelude::*;

    let per_item: Vec<Vec<R>> = items
        .par_iter()
        .enumerate()
        .map(|(i, item)| worker(i, item))
        .collect();

    let mut out = Vec::with_capacity(per_item.iter().map(Vec::len).sum());
    for result in per_item {
        out.extend(result);
    }
    out
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
pub(crate) enum PortableTerm {
    /// A term already valid in the parent's id space: an `Existing` dataset
    /// term, or a `Computed` id minted before the fork.
    Parent(SolutionTerm),
    /// A value the child minted after the fork; not yet interned anywhere but
    /// the child's own (about-to-be-dropped) scratch.
    Fresh(TermValue),
}

/// Materialize one output `row` produced against a forked child's `local`
/// scratch into a portable form, while `local` is still alive. `base` is the
/// parent's [`ScratchInterner::computed_count`] captured **at fork time** —
/// see [`PortableTerm`] for the id rule this relies on.
pub(crate) fn portable_row(
    local: &ScratchInterner,
    base: usize,
    row: &Solution,
) -> Vec<Option<PortableTerm>> {
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
pub(crate) fn reintern_portable_row(
    main: &mut ScratchInterner,
    dataset: &RdfDataset,
    prow: &[Option<PortableTerm>],
) -> Solution {
    prow.iter()
        .map(|cell| match cell {
            None => None,
            Some(PortableTerm::Parent(term)) => Some(*term),
            Some(PortableTerm::Fresh(value)) => Some(main.intern(dataset, value.clone())),
        })
        .collect()
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
        assert!(is_parallel_safe(&arith));

        let regex = call(
            Function::Regex,
            vec![
                Expression::Variable(purrdf_sparql_algebra::Variable::new("x")),
                Expression::Literal(Literal::new_simple("^a")),
            ],
        );
        assert!(is_parallel_safe(&regex));
    }

    #[test]
    fn rand_uuid_struuid_bnode_are_unsafe() {
        assert!(!is_parallel_safe(&call(Function::Rand, vec![])));
        assert!(!is_parallel_safe(&call(Function::Uuid, vec![])));
        assert!(!is_parallel_safe(&call(Function::StrUuid, vec![])));
        assert!(!is_parallel_safe(&call(Function::BNode, vec![])));
        assert!(!is_parallel_safe(&call(
            Function::BNode,
            vec![Expression::Variable(purrdf_sparql_algebra::Variable::new(
                "x"
            ))]
        )));
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
        assert!(!is_parallel_safe(&mk(
            PurrdfFn::ListSlice,
            "http://ex/listSlice"
        )));
        assert!(!is_parallel_safe(&mk(
            PurrdfFn::ListConcat,
            "http://ex/listConcat"
        )));
        assert!(is_parallel_safe(&mk(
            PurrdfFn::ListLength,
            "http://ex/listLength"
        )));
        assert!(is_parallel_safe(&mk(PurrdfFn::HeldIn, "http://ex/heldIn")));
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
        assert!(!is_parallel_safe(&in_if));

        let in_coalesce = Expression::Coalesce(vec![safe.clone(), rand.clone()]);
        assert!(!is_parallel_safe(&in_coalesce));

        let in_fn_args = call(Function::Concat, vec![safe, rand]);
        assert!(!is_parallel_safe(&in_fn_args));
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
        assert!(!is_parallel_safe(&exists));

        // Sanity: the same shape without RAND() is safe.
        let inner_bgp2 = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: vp("s"),
                predicate: pred("http://ex/p"),
                object: vp("o"),
            }],
        };
        let safe_exists = Expression::Exists(Box::new(inner_bgp2));
        assert!(is_parallel_safe(&safe_exists));
    }

    // ---- par_try_flat_map_init -------------------------------------------------

    #[test]
    fn par_try_flat_map_init_flattens_in_index_order_forced_parallel() {
        let _guard = force_parallel_for_test(true);
        let init_calls = std::sync::atomic::AtomicUsize::new(0);
        let items: Vec<usize> = (0..64).collect();
        let result = par_try_flat_map_init(
            &items,
            || {
                init_calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                0_u64 // per-worker state: unused counter, just proves init ran.
            },
            |_state, i, &item| {
                if item % 7 != 0 {
                    std::thread::yield_now();
                }
                Ok(vec![vec![Some(SolutionTerm::Existing(
                    purrdf_core::TermId::from_index(i as u32),
                ))]])
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
        // At least one worker thread ran `init` (forced parallel with 64 items and
        // rayon's default pool); it never runs per-row (64 items, far fewer inits).
        assert!(init_calls.load(std::sync::atomic::Ordering::Relaxed) >= 1);
        assert!(init_calls.load(std::sync::atomic::Ordering::Relaxed) <= 64);
    }

    #[test]
    fn par_try_flat_map_init_flattens_in_index_order_forced_sequential() {
        let _guard = force_parallel_for_test(false);
        let init_calls = std::sync::atomic::AtomicUsize::new(0);
        let items: Vec<usize> = (0..64).collect();
        let result = par_try_flat_map_init(
            &items,
            || {
                init_calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                0_u64
            },
            |_state, i, _item| {
                Ok(vec![vec![Some(SolutionTerm::Existing(
                    purrdf_core::TermId::from_index(i as u32),
                ))]])
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
        // Sequential path: `init` runs exactly once, never per row.
        assert_eq!(init_calls.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[test]
    fn par_try_flat_map_init_surfaces_the_lower_indexed_error() {
        let items: Vec<usize> = (0..32).collect();
        let result: Result<Vec<Solution>, EvalError> = par_try_flat_map_init(
            &items,
            || (),
            |(), i, _| {
                if i == 20 {
                    std::thread::yield_now();
                    return Err(EvalError::internal("error at 20"));
                }
                if i == 5 {
                    return Err(EvalError::internal("error at 5"));
                }
                Ok(vec![])
            },
        );
        let err = result.unwrap_err();
        assert_eq!(err, EvalError::internal("error at 5"));
    }

    // ---- par_flat_map ---------------------------------------------------------

    #[test]
    fn par_flat_map_flattens_in_index_order_forced_parallel() {
        let _guard = force_parallel_for_test(true);
        let items: Vec<usize> = (0..64).collect();
        let result = par_flat_map(&items, |i, &item| {
            if item % 7 != 0 {
                std::thread::yield_now();
            }
            vec![i]
        });
        let expected: Vec<usize> = (0..64).collect();
        assert_eq!(result, expected);
    }

    #[test]
    fn par_flat_map_flattens_in_index_order_forced_sequential() {
        let _guard = force_parallel_for_test(false);
        let items: Vec<usize> = (0..64).collect();
        let result = par_flat_map(&items, |i, _| vec![i]);
        let expected: Vec<usize> = (0..64).collect();
        assert_eq!(result, expected);
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
        let row: Solution = vec![None, Some(pre_fork_term), Some(fresh_term)];

        let prow = portable_row(&child.scratch, base, &row);
        assert_eq!(prow[0], None);
        assert_eq!(prow[1], Some(PortableTerm::Parent(pre_fork_term)));
        assert_eq!(prow[2], Some(PortableTerm::Fresh(fresh_value.clone())));

        let reinterned = reintern_portable_row(&mut parent.scratch, &ds, &prow);
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

        let row_a: Solution = vec![Some(term_a)];
        let row_b: Solution = vec![Some(term_b)];
        let prow_a = portable_row(&child_a.scratch, base, &row_a);
        let prow_b = portable_row(&child_b.scratch, base, &row_b);

        let reinterned_a = reintern_portable_row(&mut parent.scratch, &ds, &prow_a);
        let reinterned_b = reintern_portable_row(&mut parent.scratch, &ds, &prow_b);

        assert_eq!(
            reinterned_a[0], reinterned_b[0],
            "two workers minting the same fresh value must reintern to the same parent id"
        );
    }
}
