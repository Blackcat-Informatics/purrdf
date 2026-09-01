// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The SEP-0009 **`UNFOLD` graph pattern**: one solution per element of a
//! composite value.
//!
//! ```text
//! [174] Unfold ::= 'UNFOLD' '(' Expression 'AS' Var ( ',' Var )? ')'
//! ```
//!
//! [`crate::cdt_agg`]'s dual: `FOLD` collapses a group's rows into one composite,
//! `UNFOLD` expands one composite back into rows. Each input solution contributes
//! its own contiguous block of output solutions, in the composite's own order,
//! and the blocks follow their input rows' order — which is what makes this
//! operator's edge to its child monotone (see
//! `crate::governor::soundness::visit_pattern_parts`).
//!
//! # What the two variables bind
//!
//! Decided by the value's DATATYPE, not by the syntax — the same expression
//! reaching a `cdt:List` in one row and a `cdt:Map` in the next binds different
//! things in each, which is why [`purrdf_sparql_algebra::GraphPattern::Unfold`]'s
//! fields are named for their position:
//!
//! | value | `element` | `companion` |
//! |---|---|---|
//! | `cdt:List` | the element, in list order, duplicates preserved (`unfold-list-1var-01/02`) | the **1-based** `xsd:integer` index (`unfold-list-2vars-01`) |
//! | `cdt:Map` | the entry's key (`unfold-map-1var-01`) | the entry's value (`unfold-map-2vars-01`) |
//!
//! A `null` in either position binds NOTHING and does **not** suppress the row: a
//! null list element yields a row whose `element` is unbound while its index is
//! still bound (`unfold-list-1var-09/10`, `unfold-list-2vars-09`), and a null map
//! value yields a row binding the key alone (`unfold-map-2vars-08`). That is the
//! same `null`-is-not-a-term rule `cdt:get` follows when it answers an expression
//! error for a null position — read as a graph pattern rather than an expression,
//! "no term" is an unbound column, not a dropped row.
//!
//! A blank node inside the composite is a real blank node scoped to the literal
//! and shared within it, so two occurrences of one label bind the SAME node
//! (`unfold-list-1var-08`) and two labels bind two (`-07`). Nothing here does that
//! on purpose: it falls out of [`crate::cdt_fn::from_cdt_term`]'s `(label, scope)`
//! round trip, which is also why an unfolded element is `sameTerm` with what
//! `cdt:get` returns for the same position (all ten `unfold-get-*` cases).
//!
//! # A row whose expression denotes no composite contributes ZERO rows
//!
//! Unbound, raised, not `cdt:`-typed, or a `cdt:`-typed literal whose lexical form
//! does not parse: all four are "this row has nothing to expand", and all four
//! contribute no output row. The corpus pins none of them, so this is a
//! first-party decision, and it is the SPARQL-shaped one rather than a refusal:
//! `UNFOLD` is a graph pattern, and a graph pattern with no solutions for a row is
//! ordinary — it is what a BGP that matches nothing does, and it is what makes
//! `OPTIONAL { UNFOLD(?x AS ?e) }` over a column that is only sometimes a
//! composite behave the way every other optional pattern does. Raising instead
//! would make one non-composite value in one row abort a whole query, which no
//! other pattern in SPARQL does.
//!
//! An EMPTY composite is the same answer for the same reason: `UNFOLD` is one row
//! per element, and zero elements is zero rows. It is not one row with an unbound
//! target — that would claim an element exists whose value is `null`, which is a
//! different value from an empty list.
//!
//! The one thing that is NOT swallowed is a hard failure while evaluating the
//! expression itself ([`crate::error::EvalError::CompositeBound`] from a
//! constructor whose result crosses a `purrdf-cdt` bound): that propagates and
//! fails the query, exactly as it does in every other expression position.
//!
//! # A pre-bound target is a join, not a shadow
//!
//! The parser refuses `UNFOLD` over a variable already in scope (`BIND`'s §19.6
//! rule, applied to both targets), so query text never reaches the case below.
//! Algebra built directly through the public API can, and the answer is SPARQL's
//! own: the produced binding is JOINED against what the row already holds — the
//! row survives only if the two agree — rather than overwriting it. A `null`
//! position produces no binding at all, so it agrees with whatever was there and
//! leaves it untouched.
//!
//! # A nested composite unfolds again
//!
//! An element that is itself a composite comes back as a `cdt:`-typed literal in
//! canonical form — the same term `cdt:get` returns for that position — because
//! [`crate::cdt_fn::from_cdt_term`] re-renders a nested value rather than
//! flattening it. A second `UNFOLD` over the bound element therefore expands the
//! inner composite with no further ceremony, which is what makes the operator
//! compose with itself.

use std::sync::Arc;

use purrdf_cdt::{CdtContents, CdtValue};
use purrdf_core::{DatasetView, TermValue, TrippedGovernor};
use purrdf_sparql_algebra::{Expression, GraphPattern, Variable};

use crate::error::EvalError;
use crate::eval::{EvalCtx, eval_evaluated};
use crate::expr::eval_expr;
use crate::governor::lift::{Evaluated, Lift, Truncation};
use crate::row_ingest::{GovernedRowIngest, RowAdmission};
use crate::solution::{Solution, SolutionSeq, VarSchema};

const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// Evaluate `UNFOLD(expression AS ?element[, ?companion])` over `inner`.
///
/// # Errors
///
/// Any error `expression` raises — including
/// [`EvalError::CompositeBound`](crate::error::EvalError::CompositeBound), which
/// is a hard failure of the query rather than an empty expansion (see the module
/// docs).
pub(crate) fn eval_unfold<D: DatasetView + Sync>(
    node: &GraphPattern,
    inner: &GraphPattern,
    expression: &Expression,
    element: &Variable,
    companion: Option<&Variable>,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let mut lift = Lift::at(node);
    let Some(mut seq) = lift.absorb(0, eval_evaluated(inner, ctx)?) else {
        // No rows cross, but the COLUMNS still do — this node's output schema is
        // syntactic (the inner's columns plus its own targets), exactly as
        // `Extend`'s is.
        let mut schema = lift.absorbed_schema().map_or_else(
            || (*crate::eval::syntactic_schema(inner)).clone(),
            |s| (*s).clone(),
        );
        schema.push(element.clone());
        if let Some(companion) = companion {
            schema.push(companion.clone());
        }
        return Ok(lift.finish(SolutionSeq::empty(Arc::new(schema))));
    };
    // The `row-expression-evaluation` charge point, paid per INPUT row: one
    // expression evaluation per input row is what this operator spends before it
    // knows how far that row expands. See `crate::expr::eval_filter` for why the
    // refused rows are cut before the expression runs rather than after.
    let _ = ctx.admit_rows(
        &mut seq.rows,
        crate::governor::ChargePoint::RowExpressionEvaluation,
    );

    let in_width = seq.schema.len();
    let mut schema = (*seq.schema).clone();
    let element_col = schema.push(element.clone());
    let companion_col = companion.map(|companion| schema.push(companion.clone()));
    let width = schema.len();
    let schema = Arc::new(schema);

    // No dedicated per-row charge point: the rows this operator emits come from a
    // composite literal the dataset or the query text already carried, not from an
    // outside party the way `SERVICE`'s and a property function's do. They are
    // metered by the generic per-node accounting every algebra node pays, plus the
    // intermediate-CELL ceiling this ingest enforces — which is the dimension that
    // actually bounds the expansion, since one literal can hold up to
    // `purrdf_cdt::MAX_ELEMENTS` elements.
    let ingest = GovernedRowIngest::new(ctx, width, None);
    let mut rows: Vec<Solution<D::Id>> = Vec::new();
    let mut tripped: Option<TrippedGovernor> = None;

    'input: for (idx, mu) in seq.rows.iter().enumerate() {
        // §17.4.2.2: `BNODE(strExpr)` memoizes per solution — see `ctx.current_row`'s
        // doc. The position here is the INPUT row's, which is the solution the
        // expression is evaluated against, and it advances once per input row
        // however many output rows that row expands to.
        ctx.current_row = idx as u64;
        let Some(value) = composite_of(expression, mu, &seq.schema, ctx)? else {
            // Unbound, raised, not composite-typed, or an ill-formed composite
            // literal: nothing to expand, so this row contributes no output row.
            continue;
        };
        for (element_term, companion_term) in expansion(&value) {
            if let Some(governor) = ctx.stop_check() {
                tripped = Some(governor);
                break 'input;
            }
            match ingest.admit(ctx, rows.len()) {
                RowAdmission::Abandoned(governor) => {
                    tripped = governor;
                    break 'input;
                }
                RowAdmission::Admitted => {}
            }
            let mut row: Solution<D::Id> = smallvec::smallvec![None; width];
            row[..in_width].copy_from_slice(mu);
            if !bind(&mut row, Some(element_col), element_term, ctx)
                || !bind(&mut row, companion_col, companion_term, ctx)
            {
                // A target this row already bound incompatibly — see the module
                // docs' note on a pre-bound target. Not reachable from query text,
                // which the parser refuses, and an ordinary non-match when it is.
                continue;
            }
            rows.push(row);
        }
    }

    // This node's materialized bag, measured against the intermediate-cell ceiling
    // exactly as every other producer measures its output.
    if tripped.is_none() {
        tripped = ctx.observe_cells(rows.len(), width).err();
    }
    // An `EXISTS` inside the expanded expression is an opaque edge; see
    // `crate::binop::eval_left_join`.
    if let Some(tripped) = ctx.expression_barrier.observed() {
        return Ok(Evaluated::Truncated(Truncation::barred_at(
            node, tripped, schema,
        )));
    }
    let seq = SolutionSeq { schema, rows };
    Ok(match tripped {
        // This node stopped its own expansion, and nothing below it had already
        // truncated: the bag is exactly the prefix it committed before the
        // governor tripped, so the truncation ORIGINATES here.
        Some(tripped) if !lift.is_truncated() => {
            Evaluated::Truncated(Truncation::origin(seq, tripped))
        }
        // Either nothing tripped at all, or a CHILD had already truncated and this
        // node then stopped too. In the second case the child's certificate — the
        // one `lift.finish` composes, already ascended through this node's monotone
        // edge — is the report that must survive: it names the node the truncation
        // actually originated at, and its claim ("a positional prefix") is exactly
        // what stopping this expansion early also leaves. Overwriting it with a
        // fresh origin here would re-attribute a child's trip to this node. The
        // governor this node saw is not lost either way — a governor state latches
        // its first trip, and the query-level report reads it from there.
        _ => lift.finish(seq),
    })
}

/// The composite value `expression` denotes for `mu`, or `None` when it denotes
/// none (unbound, raised, not `cdt:`-typed, or an ill-formed composite literal).
///
/// # Errors
///
/// Any hard failure `expression` itself raises.
fn composite_of<D: DatasetView + Sync>(
    expression: &Expression,
    mu: &Solution<D::Id>,
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<CdtValue>, EvalError> {
    let Some(term) = eval_expr(expression, mu, schema, ctx)? else {
        return Ok(None);
    };
    let value = ctx.scratch.value_of(ctx.dataset, term);
    Ok(crate::cdt_fn::as_composite(&value))
}

/// One composite's expansion: `(element, companion)` per output row, in the
/// composite's own order.
///
/// The two readings the module docs tabulate, and the ONE place the choice
/// between them is made. Both positions are `Option<TermValue>` because a
/// SEP-0009 `null` has no term to bind — `from_cdt_term` answers `None` for it —
/// and the row is still produced.
fn expansion(value: &CdtValue) -> Vec<(Option<TermValue>, Option<TermValue>)> {
    match value.contents() {
        CdtContents::List(items) => items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                // 1-BASED, matching `cdt:get`'s own index space: the corpus asserts
                // `SAMETERM(?elmt, cdt:get(?list, ?idx))` for the index this binds
                // (`unfold-get-list-2vars-05.rq`, `-06.rq`), which a 0-based index
                // would fail on every element.
                (crate::cdt_fn::from_cdt_term(item), Some(index_term(i + 1)))
            })
            .collect(),
        CdtContents::Map(entries) => entries
            .iter()
            .map(|entry| {
                // A key is an IRI or a literal and never `null` (production `[7]
                // MapKey` admits nothing else), so the key position is always
                // bound; the VALUE may be `null`, and then binds nothing.
                (
                    crate::cdt_fn::from_cdt_term(&entry.key.to_term()),
                    crate::cdt_fn::from_cdt_term(&entry.value),
                )
            })
            .collect(),
    }
}

/// A 1-based list index as an `xsd:integer` term.
fn index_term(index: usize) -> TermValue {
    TermValue::Literal {
        lexical_form: index.to_string(),
        datatype: XSD_INTEGER.to_owned(),
        language: None,
        direction: None,
    }
}

/// Write one produced binding into `row`, reporting whether the row survives.
///
/// `column` is `None` for the one-variable form's absent companion, and `value`
/// is `None` for a `null` position: neither produces a binding, and both leave
/// whatever the row already held in place. A produced binding is JOINED against
/// an existing one — see the module docs' note on a pre-bound target — so the
/// answer is `false` exactly when the two disagree.
fn bind<D: DatasetView + Sync>(
    row: &mut Solution<D::Id>,
    column: Option<usize>,
    value: Option<TermValue>,
    ctx: &mut EvalCtx<'_, D>,
) -> bool {
    let (Some(column), Some(value)) = (column, value) else {
        return true;
    };
    let term = ctx.scratch.intern(ctx.dataset, value);
    match row[column] {
        None => {
            row[column] = Some(term);
            true
        }
        Some(existing) => existing == term,
    }
}
