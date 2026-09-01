// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SEP-0009's `FOLD` set function: fold a `GROUP BY` group's solution sequence
//! into a `cdt:List` or `cdt:Map` literal.
//!
//! # The two set functions, and why they are one code path
//!
//! The spec defines `Fold1` (one argument, a `cdt:List`) and `Fold2` (two
//! arguments, a `cdt:Map`) and then says, in a note under each, that the result
//! is what the `cdt:List` / `cdt:Map` CONSTRUCTOR function would return if it
//! were passed the flattened group as separate arguments. That is not a
//! coincidence to be re-derived here: this module builds the flattened argument
//! sequence and hands it to `purrdf_cdt::list_constructor` /
//! `purrdf_cdt::map_constructor`, the same two functions `cdt:List(…)` and
//! `cdt:Map(…)` already evaluate through (see [`crate::cdt_fn`]). The
//! constructor already owns every rule that follows from that:
//!
//! * an argument that came out unbound or errored is the composite `null`;
//! * a map pair whose KEY is `null`, or is a blank node (not a map key at all),
//!   VANISHES — key and value together;
//! * a map pair whose VALUE is `null` keeps its key with a `null` value;
//! * a repeated map key keeps the LAST binding;
//! * the three extent bounds are checked before anything is minted.
//!
//! Writing those rules a second time here is how the two surfaces would drift.
//!
//! # `ORDER BY` inside the aggregate
//!
//! `FOLD` is the only SPARQL aggregate whose grammar carries
//! `( 'ORDER' 'BY' OrderCondition+ )?`. The spec models it as a distinct algebra
//! symbol, `OrderGroups`, applied to the grouped input BEFORE the aggregation:
//! each group's own solution sequence is sorted by the conditions, and the fold
//! then runs over that order. This module implements it exactly there — by
//! ordering the group's row indices — rather than by sorting the folded result,
//! which would be a different (and for `Fold2`, wrong) answer: which binding of
//! a repeated key survives depends on the ROW order, not on the key order.
//!
//! Without `ORDER BY`, the fold runs in the group's inner-operator row order,
//! which is deterministic for this engine (see [`crate::modifier`]'s module
//! docs) but which the spec leaves undefined — so a query that needs a
//! particular element order must write the clause.
//!
//! # `DISTINCT`
//!
//! `FOLD(DISTINCT …)` dedups on the aggregate's ARGUMENT TUPLE, keeping the
//! first occurrence in the (possibly `ORDER BY`-sorted) row order — the same
//! `Dedup(M(Ψ))` rule §18.6.1 gives every other aggregate, and the same rule
//! [`crate::modifier::eval_custom_aggregate`] applies to a multi-argument custom
//! aggregate. An UNBOUND position is part of the tuple's identity, so
//! `FOLD(DISTINCT ?v)` over `{1, UNDEF, UNDEF}` folds two elements — `1` and one
//! `null` — not three and not one.
//!
//! # Riding the accumulator seam
//!
//! Both accumulators implement [`crate::agg_fn::AggregateAccumulator`], the one
//! fold algebra this crate has, so a large group folds through the same chunked
//! `init`/`step`/`combine`/`finish` lifecycle every other aggregate uses.
//! `combine` **appends the element vectors** — it never finishes-then-steps,
//! because `finish` returns a composite LITERAL and re-stepping that would nest
//! a finished list as a single element of the next chunk. Appending is what
//! makes the chunked answer identical to the sequential one, which is what
//! `AlgebraicClass::OrderDependent` requires of an order-dependent fold.

#[cfg(test)]
use std::cmp::Ordering;

use purrdf_cdt::{CdtOutcome, CdtTerm, CdtValue};
use purrdf_core::{DatasetView, TermValue};
use purrdf_sparql_algebra::{AggregateExpression, OrderExpression};

use crate::DetHashSet;
use crate::agg_fn::AggregateAccumulator;
use crate::cdt_fn::{argument_element, composite_bound, composite_literal};
use crate::error::EvalError;
use crate::eval::EvalCtx;
use crate::expr::eval_expr;
use crate::governor::ChargePoint;
use crate::modifier::{compare_keys, project};
use crate::scratch::SolutionTerm;
use crate::solution::{Solution, VarSchema};

/// One group's argument tuple for a `FOLD`, already evaluated: one slot for
/// `FOLD(?v)`, two for `FOLD(?k, ?v)`. `None` is "unbound or errored", which the
/// composite constructors read as `null`.
type FoldTuple = (Option<TermValue>, Option<TermValue>);

/// The identity `FOLD(DISTINCT …)` dedups on: the aggregate's argument tuple as
/// already-interned solution terms, an UNBOUND position included as itself. Two
/// slots, matching [`FoldTuple`]'s two.
type FoldIdentity<Id> = (Option<SolutionTerm<Id>>, Option<SolutionTerm<Id>>);

/// Evaluate one `FOLD` over one group.
///
/// Phase 1 orders the group's rows (when the aggregate carries `ORDER BY`),
/// evaluates every row's one or two argument expressions, applies `DISTINCT` to
/// the resulting tuples, and charges for each. Phase 2 folds the buffer through
/// [`FoldListAccumulator`] or [`FoldMapAccumulator`] — the same two-phase shape
/// [`crate::modifier`]'s built-in path uses, and for the same reason: every
/// charge is spent in phase 1, so phase 2 needs no [`EvalCtx`] at all and is
/// safe to chunk.
///
/// # Errors
///
/// [`EvalError`] from an argument or sort-key expression, or
/// [`EvalError::CompositeBound`] when the folded value would cross one of
/// `purrdf-cdt`'s three extent bounds. A refused governor charge is recorded on
/// [`EvalCtx::expression_barrier`] and answered as unbound, exactly as the
/// built-in path does.
pub(crate) fn eval_fold_aggregate<D: DatasetView + Sync>(
    agg: &AggregateExpression,
    idxs: &[usize],
    rows: &[Solution<D::Id>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let ordered = order_group_rows(agg.order_by(), idxs, rows, schema, ctx)?;
    let Some(ordered) = ordered else {
        return Ok(None);
    };

    let key_expr = agg.args().first();
    let value_expr = agg.args().get(1);
    let mut seen: Option<DetHashSet<FoldIdentity<D::Id>>> = agg.distinct.then(DetHashSet::default);
    let mut tuples: Vec<FoldTuple> = Vec::with_capacity(ordered.len());

    for i in ordered {
        let first = match key_expr {
            Some(expr) => eval_expr(expr, &rows[i], schema, ctx)?,
            // `AggregateExpression::new_fold` refuses an empty `args`, so this
            // arm is unreachable for anything the parser produced. Folding an
            // all-`null` tuple is the honest answer for a state that cannot be
            // constructed, and it never silently becomes a DIFFERENT aggregate.
            None => None,
        };
        let second = match value_expr {
            Some(expr) => eval_expr(expr, &rows[i], schema, ctx)?,
            None => None,
        };
        // Charged per ROW, before `DISTINCT` — the same ordering, and for the
        // same reason, as `ChargePoint::AggregateAccumulation`'s doc gives for
        // every other aggregate: the work of producing a duplicate was still
        // done.
        if let Err(tripped) = ctx.charge(ChargePoint::AggregateAccumulation) {
            ctx.expression_barrier.record(tripped);
            return Ok(None);
        }
        if let Some(seen) = seen.as_mut()
            && !seen.insert((first, second))
        {
            continue;
        }
        let first = first.map(|t| ctx.scratch.value_of(ctx.dataset, t));
        let second = second.map(|t| ctx.scratch.value_of(ctx.dataset, t));
        for value in [first.as_ref(), second.as_ref()].into_iter().flatten() {
            if let Err(tripped) = ctx.charge_amount(
                purrdf_core::ResourceDimension::ScratchBytes,
                crate::scratch::value_bytes(value),
            ) {
                ctx.expression_barrier.record(tripped);
                return Ok(None);
            }
        }
        tuples.push((first, second));
    }

    let value = if value_expr.is_some() {
        fold_tuples(
            &tuples,
            FoldMapAccumulator::default,
            FoldMapAccumulator::push,
        )?
    } else {
        fold_tuples(
            &tuples,
            FoldListAccumulator::default,
            FoldListAccumulator::push,
        )?
    };
    Ok(value.map(|v| ctx.scratch.intern(ctx.dataset, v)))
}

/// The spec's `OrderGroups`: this group's row indices, sorted by the aggregate's
/// own `ORDER BY` conditions.
///
/// Returns `idxs` unchanged (as an owned `Vec`) when the aggregate wrote no
/// `ORDER BY`, and `None` when a governor charge was refused while evaluating a
/// sort key — the caller answers the whole aggregate as unbound, exactly as it
/// does for a refused accumulation charge.
///
/// The sort keys are evaluated ONCE per row and [`project`]ed once, then the
/// PERMUTATION is sorted, so the per-literal XSD parse is paid `O(n)` times
/// rather than inside an `O(n log n)` comparator — the same shape
/// [`crate::modifier::eval_order_by`] uses for a query-level `ORDER BY`, and the
/// reason both go through the same [`project`] / [`compare_keys`] pair rather
/// than a second hand-written ordering.
fn order_group_rows<D: DatasetView + Sync>(
    conditions: &[OrderExpression],
    idxs: &[usize],
    rows: &[Solution<D::Id>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<Vec<usize>>, EvalError> {
    if conditions.is_empty() {
        return Ok(Some(idxs.to_vec()));
    }
    let width = conditions.len();
    let mut values: Vec<Option<TermValue>> = Vec::with_capacity(idxs.len() * width);
    for &i in idxs {
        for condition in conditions {
            let (OrderExpression::Asc(e) | OrderExpression::Desc(e)) = condition;
            let term = eval_expr(e, &rows[i], schema, ctx)?;
            values.push(term.map(|t| ctx.scratch.value_of(ctx.dataset, t)));
        }
    }
    let keys: Vec<_> = values.iter().map(|v| project(v.as_ref())).collect();
    let mut order: Vec<usize> = (0..idxs.len()).collect();
    order.sort_by(|a, b| compare_keys(&keys[a * width..], &keys[b * width..], conditions));
    Ok(Some(order.into_iter().map(|slot| idxs[slot]).collect()))
}

/// Fold an already-evaluated, already-`DISTINCT`-resolved tuple buffer through
/// one accumulator instance — chunked in parallel for a large enough group,
/// sequential for a small one.
///
/// The exact counterpart of [`crate::modifier`]'s `fold_builtin`, differing only
/// in the per-row item type: a `FOLD` row's item is a PAIR of optional values (a
/// `null` is a value here, not a skipped row), which no `&[TermValue]` slice can
/// express, so `step` is reached through each accumulator's own inherent `push`
/// rather than through the trait's slice-shaped method. `combine` and `finish`
/// are the trait's, unchanged, which is what keeps this on the one fold algebra.
fn fold_tuples<A: AggregateAccumulator>(
    tuples: &[FoldTuple],
    init: impl Fn() -> A + Sync,
    push: impl Fn(&mut A, &FoldTuple) -> Result<(), EvalError> + Sync,
) -> Result<Option<TermValue>, EvalError> {
    let fold = crate::parallel::par_chunk_reduce_init(
        tuples,
        || Ok(init()),
        push,
        |acc: &mut A, other: A| acc.combine(Box::new(other)),
    )?;
    Box::new(fold).finish()
}

/// The spec's `Fold1`: collect one composite element per group row into a
/// `cdt:List` literal.
#[derive(Default)]
pub(crate) struct FoldListAccumulator {
    items: Vec<CdtTerm>,
}

impl FoldListAccumulator {
    /// Fold one row's argument tuple. Only the first slot is read — `Fold1` has
    /// one argument — and an unbound one is the composite `null`, which is a
    /// real element of the list, not a skipped row: `FOLD(?v)` over `{1, UNDEF}`
    /// is a list of SIZE TWO.
    ///
    /// # Errors
    ///
    /// [`EvalError::CompositeBound`] when the element itself cannot appear in any
    /// composite (a nested value already at the nesting bound).
    fn push(&mut self, tuple: &FoldTuple) -> Result<(), EvalError> {
        self.items.push(argument_element(tuple.0.as_ref())?);
        Ok(())
    }
}

impl AggregateAccumulator for FoldListAccumulator {
    /// Unreachable for `FOLD`: the fold driver calls [`Self::push`], which can
    /// carry the `null` an already-evaluated slice of bound values cannot.
    /// Implemented as the faithful bound-value case rather than a panic, so a
    /// caller that reached this accumulator through the generic trait still gets
    /// the right answer.
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        for value in args {
            self.items.push(argument_element(Some(value))?);
        }
        Ok(())
    }

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        let other = other.into_any().downcast::<Self>().map_err(|_| {
            EvalError::internal("FOLD's cdt:List accumulator was combined with a foreign partial")
        })?;
        self.items.extend(other.items);
        Ok(())
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        outcome_to_value(purrdf_cdt::list_constructor(self.items))
    }
}

/// The spec's `Fold2`: collect one key/value pair per group row into a
/// `cdt:Map` literal.
#[derive(Default)]
pub(crate) struct FoldMapAccumulator {
    pairs: Vec<(CdtTerm, CdtTerm)>,
}

impl FoldMapAccumulator {
    /// Fold one row's `(key, value)` tuple. Both slots may be unbound and both
    /// become the composite `null`; `purrdf_cdt::map_constructor` then applies
    /// the spec's asymmetry — a `null` (or blank-node) KEY drops the pair
    /// entirely, a `null` VALUE keeps it — at `finish`, over the whole ordered
    /// pair list, which is also where "a repeated key keeps the last binding"
    /// is decided. Deciding either here, per row, would give the wrong answer
    /// under the chunked fold, where `self` has seen only part of the group.
    ///
    /// # Errors
    ///
    /// [`EvalError::CompositeBound`] when either term cannot appear in any
    /// composite.
    fn push(&mut self, tuple: &FoldTuple) -> Result<(), EvalError> {
        let key = argument_element(tuple.0.as_ref())?;
        let value = argument_element(tuple.1.as_ref())?;
        self.pairs.push((key, value));
        Ok(())
    }
}

impl AggregateAccumulator for FoldMapAccumulator {
    /// Unreachable for `FOLD` — see [`FoldListAccumulator::step`]. A two-element
    /// `args` slice is the bound-value case of one key/value pair.
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        let [key, value] = args else {
            return Err(EvalError::internal(
                "FOLD's cdt:Map accumulator takes a key/value pair, not a single value",
            ));
        };
        self.pairs
            .push((argument_element(Some(key))?, argument_element(Some(value))?));
        Ok(())
    }

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        let other = other.into_any().downcast::<Self>().map_err(|_| {
            EvalError::internal("FOLD's cdt:Map accumulator was combined with a foreign partial")
        })?;
        self.pairs.extend(other.pairs);
        Ok(())
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        outcome_to_value(purrdf_cdt::map_constructor(&self.pairs))
    }
}

/// A constructor outcome as the composite literal it minted.
///
/// PurRDF COMPUTED this value, so it is spelled in `purrdf-cdt`'s canonical form
/// — the same rule [`crate::cdt_fn`] applies to every value the `cdt:` functions
/// mint, and what makes two independent evaluations of the same `FOLD` produce
/// the same RDF term.
///
/// [`CdtOutcome::Error`] is unbound (no composite exists to return) and
/// [`CdtOutcome::Bound`] is a HARD failure: the group is genuinely too large to
/// be a composite, and answering unbound there would report an absent value for
/// one that exists.
fn outcome_to_value(outcome: CdtOutcome<CdtValue>) -> Result<Option<TermValue>, EvalError> {
    match outcome {
        CdtOutcome::Value(value) => Ok(Some(composite_literal(&value))),
        CdtOutcome::Error(_) => Ok(None),
        CdtOutcome::Bound(error) => Err(composite_bound(&error)),
    }
}

/// Total order over two folded results, used only by this module's tests.
#[cfg(test)]
fn debug_cmp(a: Option<&TermValue>, b: Option<&TermValue>) -> Ordering {
    format!("{a:?}").cmp(&format!("{b:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `xsd:integer` term with the given lexical form.
    fn int(lexical: &str) -> TermValue {
        TermValue::Literal {
            lexical_form: lexical.to_owned(),
            datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
            language: None,
            direction: None,
        }
    }

    /// The canonical spelling of an `xsd:integer` element — a composite literal
    /// is ALWAYS explicit in `purrdf-cdt`'s canonical form, never the bare `1`
    /// shorthand the lexical space also admits (see `purrdf_cdt::render`).
    fn canon_int(lexical: &str) -> String {
        format!("\"{lexical}\"^^<http://www.w3.org/2001/XMLSchema#integer>")
    }

    fn lexical(value: Option<&TermValue>) -> &str {
        match value {
            Some(TermValue::Literal { lexical_form, .. }) => lexical_form,
            other => panic!("expected a composite literal, got {other:?}"),
        }
    }

    /// `Fold1` over an empty group is the EMPTY list, never unbound: SEP-0009
    /// §11.3.1.1's note says so in as many words, and `fold/fold-list-02.rq`
    /// requires `FOLD(?v)` over a group with no rows to equal `"[]"^^cdt:List`.
    #[test]
    fn fold1_over_an_empty_group_is_the_empty_list() {
        let folded = fold_tuples(&[], FoldListAccumulator::default, FoldListAccumulator::push)
            .expect("an empty fold cannot fail");
        assert_eq!(lexical(folded.as_ref()), "[]");
    }

    /// `Fold2` over an empty group is the EMPTY map — §11.3.1.2's matching note,
    /// and `fold/fold-map-01.rq`.
    #[test]
    fn fold2_over_an_empty_group_is_the_empty_map() {
        let folded = fold_tuples(&[], FoldMapAccumulator::default, FoldMapAccumulator::push)
            .expect("an empty fold cannot fail");
        assert_eq!(lexical(folded.as_ref()), "{}");
    }

    /// An unbound argument is a `null` ELEMENT, not a skipped row: the list from
    /// `{1, UNDEF}` has size two (`fold/fold-list-04.rq`).
    #[test]
    fn fold1_writes_an_unbound_argument_as_a_null_element() {
        let tuples = vec![(Some(int("1")), None), (None, None)];
        let folded = fold_tuples(
            &tuples,
            FoldListAccumulator::default,
            FoldListAccumulator::push,
        )
        .expect("folding two elements cannot fail");
        assert_eq!(
            lexical(folded.as_ref()),
            format!("[{},null]", canon_int("1"))
        );
    }

    /// A `null` KEY drops the whole pair; a `null` VALUE keeps its key
    /// (`fold/fold-map-03.rq`, `fold-map-04.rq`).
    #[test]
    fn fold2_drops_a_null_key_and_keeps_a_null_value() {
        let tuples = vec![
            (None, Some(int("2"))),
            (Some(int("1")), None),
            (Some(int("3")), Some(int("4"))),
        ];
        let folded = fold_tuples(
            &tuples,
            FoldMapAccumulator::default,
            FoldMapAccumulator::push,
        )
        .expect("folding three pairs cannot fail");
        assert_eq!(
            lexical(folded.as_ref()),
            format!(
                "{{{}:null,{}:{}}}",
                canon_int("1"),
                canon_int("3"),
                canon_int("4")
            )
        );
    }

    /// A blank-node key is not a map key at all, so its pair vanishes
    /// (`fold/fold-map-05.rq`).
    #[test]
    fn fold2_drops_a_blank_node_key() {
        let tuples = vec![
            (
                Some(TermValue::Blank {
                    label: "b0".to_owned(),
                    scope: purrdf_core::BlankScope::default(),
                }),
                Some(int("1")),
            ),
            (Some(int("42")), Some(int("2"))),
        ];
        let folded = fold_tuples(
            &tuples,
            FoldMapAccumulator::default,
            FoldMapAccumulator::push,
        )
        .expect("folding two pairs cannot fail");
        assert_eq!(
            lexical(folded.as_ref()),
            format!("{{{}:{}}}", canon_int("42"), canon_int("2"))
        );
    }

    /// A repeated key keeps the LAST binding in row order — which is why
    /// `combine` must APPEND rather than merge finished maps
    /// (`fold/fold-map-orderby-01.rq`).
    #[test]
    fn fold2_keeps_the_last_binding_of_a_repeated_key() {
        let tuples = vec![
            (Some(int("2")), Some(int("201"))),
            (Some(int("2")), Some(int("202"))),
            (Some(int("2")), Some(int("203"))),
        ];
        let folded = fold_tuples(
            &tuples,
            FoldMapAccumulator::default,
            FoldMapAccumulator::push,
        )
        .expect("folding three pairs cannot fail");
        assert_eq!(
            lexical(folded.as_ref()),
            format!("{{{}:{}}}", canon_int("2"), canon_int("203"))
        );
    }

    /// The chunked fold and the sequential one must produce the SAME term, which
    /// is what `combine`-by-append buys and what finish-then-step would break: a
    /// finished `cdt:List` re-stepped into the next chunk would nest.
    #[test]
    fn combine_appends_rather_than_nesting_a_finished_partial() {
        let mut left = Box::new(FoldListAccumulator::default());
        left.push(&(Some(int("1")), None)).expect("push 1");
        let mut right = Box::new(FoldListAccumulator::default());
        right.push(&(Some(int("2")), None)).expect("push 2");
        left.combine(right).expect("combine two partials");
        let combined = left.finish().expect("finish the combined fold");

        let sequential = fold_tuples(
            &[(Some(int("1")), None), (Some(int("2")), None)],
            FoldListAccumulator::default,
            FoldListAccumulator::push,
        )
        .expect("sequential fold");

        assert_eq!(
            debug_cmp(combined.as_ref(), sequential.as_ref()),
            Ordering::Equal,
            "chunked and sequential folds disagree: {combined:?} vs {sequential:?}"
        );
        assert_eq!(
            lexical(combined.as_ref()),
            format!("[{},{}]", canon_int("1"), canon_int("2"))
        );
    }

    /// Combining a `cdt:List` partial with a `cdt:Map` one is a host-contract
    /// violation the trait cannot rule out at compile time; it must be a typed
    /// refusal, never a silently wrong merge.
    #[test]
    fn combining_two_different_fold_accumulators_is_refused() {
        let mut list = Box::new(FoldListAccumulator::default());
        let map = Box::new(FoldMapAccumulator::default());
        let error = list
            .combine(map)
            .expect_err("a cdt:List partial cannot absorb a cdt:Map partial");
        assert!(
            error.to_string().contains("foreign partial"),
            "the refusal must name what it caught, got: {error}"
        );
    }

    /// The neighbouring VALID case for the refusal above: two partials of the
    /// SAME kind still combine.
    #[test]
    fn combining_two_map_accumulators_succeeds() {
        let mut left = Box::new(FoldMapAccumulator::default());
        left.push(&(Some(int("1")), Some(int("2")))).expect("push");
        let mut right = Box::new(FoldMapAccumulator::default());
        right.push(&(Some(int("3")), Some(int("4")))).expect("push");
        left.combine(right).expect("two cdt:Map partials combine");
        let folded = left.finish().expect("finish");
        assert_eq!(
            lexical(folded.as_ref()),
            format!(
                "{{{}:{},{}:{}}}",
                canon_int("1"),
                canon_int("2"),
                canon_int("3"),
                canon_int("4")
            )
        );
    }
}
