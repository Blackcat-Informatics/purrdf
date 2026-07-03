// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic two-phase parallel evaluation primitives.
//!
//! Nothing in this module is wired into the evaluation recursion yet — it is the
//! shared foundation the per-operator fork-join tasks (parallel BGP batches,
//! parallel `GROUP BY` group evaluation, parallel `UNION` branches) build on. The
//! two phases, always in this order:
//!
//! 1. **Fork.** [`crate::eval::EvalCtx::fork_for_worker`] gives each worker a
//!    `Send` child context with its own scratch/constructed state, so workers
//!    never contend on a lock or share mutable evaluation state.
//! 2. **Join.** [`par_try_flat_map`] runs the workers via rayon's *indexed*
//!    `collect` (never `par_sort`/`par_bridge`, which are not order-stable) and
//!    then reduces strictly in source-index order: successes flatten in index
//!    order and the first `Err` **by index** wins, regardless of which worker
//!    finished first. [`absorb_row`] / [`absorb_constructed`] fold a worker's
//!    fresh scratch/constructed state back into the parent, also index-ordered
//!    by the caller. The result is bit-identical to the sequential evaluation of
//!    the same pattern — parallelism is purely a scheduling change.
//!
//! [`is_parallel_safe`] is the gate deciding whether an expression may run under
//! this model at all: any builtin whose result depends on the per-query mutable
//! `bnode_counter`/`rng_state` (or that mints into [`crate::eval::EvalCtx::constructed`])
//! is excluded, because the fork model gives every worker an *independent* copy of
//! that state rather than a shared, ordered one — running such a builtin under
//! fork-join would make its result depend on worker scheduling, not just row
//! content.
//!
//! Every item here is exercised by this module's unit tests but, until Tasks
//! 4-6 wire it into `bgp`/`modifier`/`binop`, has no production caller — hence
//! the crate-build-only `allow(dead_code)` below (lifted the moment a caller
//! lands).

#![cfg_attr(not(test), allow(dead_code))]

use purrdf_core::{RdfDataset, TermValue};
use purrdf_sparql_algebra::{
    AggregateExpression, Expression, Function, GraphPattern, OrderExpression,
};

use crate::error::EvalError;
use crate::scratch::ScratchInterner;
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
///   dataset-independent so the cells themselves fold back deterministically
///   (see [`absorb_constructed`]), but the *label* is only collision-free
///   against the single shared counter; two forked workers each minting from
///   their own fresh `bnode_counter` could produce colliding cell labels.
///
/// Every other reader-only PurRDF list function (`listLength`/`listGet`/
/// `listIndexOf`/`listContains`) and `heldIn` touch neither counter, so they are
/// left safe. When in doubt this walk flags UNSAFE — a sequential fallback is
/// always a correct (if slower) answer.
pub(crate) fn is_parallel_safe(expr: &Expression) -> bool {
    !expr_reaches_unsafe_builtin(expr)
}

/// `true` iff `expr` (recursively) reaches an unsafe builtin — see
/// [`is_parallel_safe`].
fn expr_reaches_unsafe_builtin(expr: &Expression) -> bool {
    match expr {
        Expression::NamedNode(_) | Expression::Literal(_) | Expression::Variable(_) | Expression::Bound(_) => {
            false
        }
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
        | Expression::Divide(a, b) => expr_reaches_unsafe_builtin(a) || expr_reaches_unsafe_builtin(b),
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
            purrdf_sparql_algebra::PurrdfFn::ListSlice | purrdf_sparql_algebra::PurrdfFn::ListConcat
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
                || expression
                    .as_ref()
                    .is_some_and(expr_reaches_unsafe_builtin)
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

/// Run `worker` over every item of `items` and reduce the results
/// **deterministically**: order-stable success flattening (rayon's indexed
/// `collect` preserves source order — never `par_sort`/`par_bridge`, which are
/// not order-stable) and, on failure, the first `Err` **by source index** wins
/// regardless of which worker finished first.
///
/// Internally gated on [`should_parallelize`]: at or below [`PARALLEL_MIN_ROWS`]
/// this runs a plain sequential `iter().enumerate()` fold (bit-identical output,
/// no rayon hand-off cost); above it, rayon's indexed `par_iter`. Callers get a
/// single call site — the `#[cfg(test)]` force seam in [`should_parallelize`]
/// routes through here.
///
/// Mirrors `purrdf_rdf::native_codecs::text_parse::parse_lines_parallel_with_chunk_size`:
/// every worker result is collected into a plain `Vec` FIRST, then walked in
/// order and `?`-propagated, so a fast late item can never race ahead of an
/// earlier item's diagnostic.
pub(crate) fn par_try_flat_map<T, F>(items: &[T], worker: F) -> Result<Vec<Solution>, EvalError>
where
    T: Sync,
    F: Fn(usize, &T) -> Result<Vec<Solution>, EvalError> + Sync + Send,
{
    if !should_parallelize(items.len()) {
        let mut out = Vec::new();
        for (i, item) in items.iter().enumerate() {
            out.extend(worker(i, item)?);
        }
        return Ok(out);
    }

    use rayon::prelude::*;

    let per_item: Vec<Result<Vec<Solution>, EvalError>> = items
        .par_iter()
        .enumerate()
        .map(|(i, item)| worker(i, item))
        .collect();

    let mut out = Vec::with_capacity(per_item.iter().map(|r| r.as_ref().map_or(0, Vec::len)).sum());
    for result in per_item {
        out.extend(result?);
    }
    Ok(out)
}

/// The infallible sibling of [`par_try_flat_map`]: run `worker` over every item
/// of `items` and flatten the per-item `Vec<R>` results in source-index order.
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

    let per_item: Vec<Vec<R>> = items.par_iter().enumerate().map(|(i, item)| worker(i, item)).collect();

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

    items.par_iter().filter(|item| keep(item)).cloned().collect()
}

/// Fold one row produced by a forked worker's `local` scratch back into the
/// `main` (parent) scratch, re-interning any [`crate::scratch::SolutionTerm::Computed`]
/// cell against `main`/`dataset` so its [`crate::scratch::ScratchId`] is valid in the
/// parent's id space. `Existing`/`None` cells pass through unchanged (they are
/// already dataset-space ids, valid in any scratch).
pub(crate) fn absorb_row(
    main: &mut ScratchInterner,
    dataset: &RdfDataset,
    local: &ScratchInterner,
    row: &Solution,
) -> Solution {
    row.iter()
        .map(|cell| match cell {
            Some(crate::scratch::SolutionTerm::Computed(sid)) => {
                Some(main.intern(dataset, local.computed_value(*sid).clone()))
            }
            other => *other,
        })
        .collect()
}

/// Append a forked worker's constructed cells onto `main`, in the order given.
/// The cells are dataset-independent [`TermValue`] triples (no id space to
/// re-map, unlike [`absorb_row`]) — the caller is responsible for invoking this
/// once per child **in source-index order**, which is what makes the merged
/// buffer deterministic.
pub(crate) fn absorb_constructed(
    main: &mut Vec<(TermValue, TermValue, TermValue)>,
    child: Vec<(TermValue, TermValue, TermValue)>,
) {
    main.extend(child);
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

        let in_if = Expression::If(Box::new(cond), Box::new(safe.clone()), Box::new(rand.clone()));
        assert!(!is_parallel_safe(&in_if));

        let in_coalesce = Expression::Coalesce(vec![safe.clone(), rand.clone()]);
        assert!(!is_parallel_safe(&in_coalesce));

        let in_fn_args = call(Function::Concat, vec![safe, rand]);
        assert!(!is_parallel_safe(&in_fn_args));
    }

    #[test]
    fn unsafe_inside_nested_exists_filter_is_detected() {
        let vp = |n: &str| purrdf_sparql_algebra::TermPattern::Variable(
            purrdf_sparql_algebra::Variable::new(n),
        );
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

    // ---- par_try_flat_map ----------------------------------------------------

    #[test]
    fn par_try_flat_map_flattens_in_index_order() {
        let items: Vec<usize> = (0..64).collect();
        let result = par_try_flat_map(&items, |i, &item| {
            // Deliberately makes later-indexed items "finish faster" by doing less
            // work, to prove the reduce is still index-ordered rather than
            // completion-ordered.
            if item % 7 != 0 {
                std::thread::yield_now();
            }
            Ok(vec![vec![
                Some(crate::scratch::SolutionTerm::Existing(
                    purrdf_core::TermId::from_index(i as u32),
                )),
            ]])
        })
        .expect("no errors");
        let indices: Vec<u32> = result
            .iter()
            .map(|row| match row[0] {
                Some(crate::scratch::SolutionTerm::Existing(id)) => id.index() as u32,
                _ => unreachable!(),
            })
            .collect();
        let expected: Vec<u32> = (0..64).collect();
        assert_eq!(indices, expected);
    }

    #[test]
    fn par_try_flat_map_surfaces_the_lower_indexed_error() {
        let items: Vec<usize> = (0..32).collect();
        let result: Result<Vec<Solution>, EvalError> = par_try_flat_map(&items, |i, _| {
            if i == 20 {
                // The "slow" earlier error: give the scheduler a chance to let a
                // later index finish first.
                std::thread::yield_now();
                return Err(EvalError::internal("error at 20"));
            }
            if i == 5 {
                return Err(EvalError::internal("error at 5"));
            }
            Ok(vec![])
        });
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

    // ---- fork_for_worker + absorb_row -----------------------------------------

    #[test]
    fn fork_and_absorb_row_round_trips_computed_values() {
        let ds = RdfDatasetBuilder::new()
            .freeze()
            .expect("freeze empty dataset");
        let mut parent = crate::eval::EvalCtx::new(&ds);
        let mut child = parent.fork_for_worker();

        let value = TermValue::Literal {
            lexical_form: "hello parallel".to_owned(),
            datatype: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
            language: None,
            direction: None,
        };
        let child_term = child.scratch.intern(&ds, value.clone());
        let row: Solution = vec![Some(child_term)];

        let absorbed = absorb_row(&mut parent.scratch, &ds, &child.scratch, &row);
        let main_term = absorbed[0].expect("cell present");
        assert_eq!(parent.scratch.value_of(&ds, main_term), value);
    }

    #[test]
    fn absorb_constructed_appends_in_call_order() {
        let cell = |n: &str| TermValue::Iri(format!("http://ex/{n}"));
        let mut main: Vec<(TermValue, TermValue, TermValue)> = vec![(
            cell("s0"),
            cell("p0"),
            cell("o0"),
        )];
        let child_a = vec![(cell("s1"), cell("p1"), cell("o1"))];
        let child_b = vec![(cell("s2"), cell("p2"), cell("o2"))];

        absorb_constructed(&mut main, child_a);
        absorb_constructed(&mut main, child_b);

        assert_eq!(
            main,
            vec![
                (cell("s0"), cell("p0"), cell("o0")),
                (cell("s1"), cell("p1"), cell("o1")),
                (cell("s2"), cell("p2"), cell("o2")),
            ]
        );
    }
}
