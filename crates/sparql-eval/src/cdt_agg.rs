// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The SEP-0009 **`FOLD` aggregate**: a `GROUP BY` group's rows collected into a
//! single `cdt:List` or `cdt:Map` literal.
//!
//! `FOLD` is [`AggregateFunction::Fold`], a keyword alternative of the SPARQL
//! `Aggregate` production, and it is the composite-datatype dual of
//! [`crate::cdt_unfold`]: one folds a group's rows into a composite, the other
//! expands a composite back into rows.
//!
//! ```text
//! FOLD( DISTINCT? Expression ( ',' Expression )? ( ORDER BY OrderCondition+ )? )
//! ```
//!
//! # Why it needs its own phase 1
//!
//! `crate::modifier`'s `eval_aggregate` runs one shared phase 1 for every
//! built-in: evaluate the single argument per row, SKIP the row when it is
//! unbound, apply `DISTINCT`, and hand the survivor list to a
//! [`crate::agg_fn::AggregateAccumulator`]. `FOLD` disagrees with three of those
//! four steps, so it is dispatched away at the top of that function and runs the
//! phase 1 below instead:
//!
//! * an unbound row is **retained**, as the SEP-0009 `null` element — it counts
//!   toward the list's length and occupies its sorted position
//!   (`vectors/sparql-cdt/fold/fold-list-04.rq`, `fold-list-05.rq`,
//!   `fold-list-orderby-04.rq`), which is the exact opposite of every other
//!   aggregate's error-row rule;
//! * the `cdt:Map` form evaluates **two** expressions per row, not one;
//! * the survivors are **re-ordered** by the aggregate's own `ORDER BY` before
//!   anything is folded.
//!
//! Phase 2 is nonetheless the ordinary [`crate::modifier::fold_builtin`] tail
//! over an ordinary [`crate::agg_fn::AggregateAccumulator`]
//! ([`FoldAccumulator`]) — one fold algebra, exactly as `crate::modifier`'s
//! dispatch documentation promises, not a second one bolted on beside it.
//!
//! # `DISTINCT` is TERM identity, and it reads only the exprlist
//!
//! `FOLD(DISTINCT ?v)` over `{"1"^^xsd:integer, "01"^^xsd:integer}` yields a list
//! of size **two**: the corpus asserts both elements with `SAMETERM`
//! (`fold-list-distinct-07.rq`, `-08.rq`), so the de-duplication is on the RDF
//! term, never on the value. That is what this module's `DetHashSet` of
//! [`SolutionTerm`]s gives for free — the scratch interner's promotion rule makes
//! `SolutionTerm` equality exactly term identity (see `crate::scratch`).
//!
//! It reads the FOLD expression list and nothing else: two rows agreeing on `?v`
//! collapse however much they disagree elsewhere, including on the sort key
//! (`fold-list-distinct-orderby-02.rq`). An unbound value is one more distinct
//! key, so a run of unbound rows collapses to a SINGLE retained `null`
//! (`fold-list-distinct-05.rq`, `-06.rq`).
//!
//! De-duplication happens BEFORE the sort, keeping each value's first occurrence
//! in row order and then placing that row by its own sort key —
//! `fold-list-distinct-orderby-03.rq` is the case that pins the order of the two
//! steps.
//!
//! For the `cdt:Map` form the corpus writes no `FOLD(DISTINCT ?k, ?v)` at all, so
//! the rule here is a first-party decision: de-duplication is on the WHOLE
//! `(key, value)` tuple, the same rule `crate::modifier`'s
//! `eval_custom_aggregate` applies to a multi-argument custom aggregate. Keying
//! on the key alone would silently change WHICH value a repeated key keeps —
//! turning `FOLD(DISTINCT ?k, ?v)`'s documented last-in-sort-order-wins rule into
//! first-in-row-order-wins — which is a different answer, not a smaller one.
//! Pinned by `distinct_over_the_map_form_deduplicates_the_whole_pair`.
//!
//! # `ORDER BY`, and what it decides for a map
//!
//! The sort is SPARQL's own solution ordering (§15.1) — this crate's one
//! projection of it, `crate::modifier`'s [`project`]/[`compare_keys`], so an
//! unbound key sorts below every bound one exactly as it does in a query's own
//! `ORDER BY` (`fold-list-orderby-05.rq`) — applied left to right across the
//! conditions (`fold-list-orderby-06.rq`) and STABLE, so rows the conditions do
//! not separate keep their row order.
//!
//! For the `cdt:Map` form the order is not merely cosmetic: duplicate keys
//! collapse and the LAST entry in the folded order wins, so `ORDER BY ?sort` and
//! `ORDER BY DESC(?sort)` over the same rows produce maps with DIFFERENT values
//! under the repeated key (`fold-map-orderby-01.rq` yields `203`, `-02.rq` yields
//! `201`). Without an `ORDER BY` which one survives is unspecified
//! (`fold-map-06.rq` accepts either), and this implementation answers with the
//! last in ROW order, which is the only order it has.
//!
//! # The empty group is a bound composite, never unbound
//!
//! `finish` over an accumulator nothing was ever folded into is `"[]"^^cdt:List`
//! / `"{}"^^cdt:Map` (`fold-list-02.rq`, `fold-map-01.rq`) — `FOLD` joins
//! `COUNT`/`SUM`/`GROUP_CONCAT` in the set of aggregates whose empty-group answer
//! is a value rather than the unbound `AVG`/`MIN`/`MAX`/`SAMPLE` give.

use purrdf_cdt::{CdtError, CdtTerm, CdtValue, MAX_ELEMENTS};
use purrdf_core::{DatasetView, TermValue};
use purrdf_sparql_algebra::{AggregateExpression, OrderExpression};

use crate::agg_fn::AggregateAccumulator;
use crate::cdt_fn::{argument_element, composite_literal};
use crate::error::EvalError;
use crate::eval::EvalCtx;
use crate::expr::eval_expr;
use crate::governor::ChargePoint;
use crate::modifier::{SortKey, compare_keys, fold_builtin, order_sort_key, project};
use crate::scratch::SolutionTerm;
use crate::solution::{Solution, VarSchema};

/// The `DISTINCT` witness set: one entry per exprlist tuple already folded, in
/// RDF-TERM identity (see the module docs), and `None` when the call carried no
/// `DISTINCT` at all.
///
/// A fixed two-slot array rather than a `Vec`, because `FOLD`s exprlist is one or
/// two expressions and nothing else — the unused second slot of the `cdt:List`
/// form is always `None`, which is one more term-identity value and never
/// conflates two distinct rows.
type FoldDistinct<I> = Option<crate::DetHashSet<[Option<SolutionTerm<I>>; 2]>>;

/// Which composite a `FOLD` call builds, decided once from its exprlist length:
/// one expression is the `cdt:List` form, two the `cdt:Map` form.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FoldTarget {
    /// `FOLD(?v)` — a `cdt:List` of the per-row values.
    List,
    /// `FOLD(?k, ?v)` — a `cdt:Map` of the per-row `(key, value)` pairs.
    Map,
}

/// One row's already-evaluated contribution to a fold, in exprlist order.
///
/// `None` in either slot is the SEP-0009 `null`: the row's expression was unbound
/// or raised. Both are one state here because SEP-0009 treats them identically
/// (see `crate::cdt_fn`'s module docs); WHERE the null lands is what matters, and
/// that is decided by [`FoldTarget`], not here.
///
/// `Default` (both slots `None`) exists so the sort below can permute survivors
/// with [`std::mem::take`] rather than cloning every retained value a second time.
#[derive(Clone, Default, Debug)]
pub(crate) struct FoldRow {
    /// The first expression's value — the list element, or the map KEY.
    first: Option<TermValue>,
    /// The second expression's value — the map VALUE. Always `None` (and never
    /// read) in the [`FoldTarget::List`] form.
    second: Option<TermValue>,
}

/// The `FOLD` fold state: the composite being built, one element per folded row.
///
/// A genuine [`AggregateAccumulator`], so `FOLD` shares the one fold algebra this
/// crate has rather than introducing a second. Its algebraic class is
/// [`crate::agg_fn::AlgebraicClass::OrderDependent`] — the order rows are folded
/// in IS the list's element order, and for a map it decides which value a
/// repeated key keeps — which is exactly the class
/// [`AggregateAccumulator::combine`]'s fixed chunk-order contract exists to keep
/// deterministic when a group is folded by more than one worker.
pub(crate) struct FoldAccumulator {
    /// The list elements, or the map's `(key, value)` pairs, in folded order.
    parts: FoldParts,
}

/// [`FoldAccumulator`]'s state, one arm per [`FoldTarget`].
enum FoldParts {
    /// `cdt:List` elements, in folded order.
    List(Vec<CdtTerm>),
    /// `cdt:Map` pairs, in folded order — NOT yet de-duplicated: a repeated key
    /// is resolved by [`purrdf_cdt::map_constructor`] at `finish`, which is the
    /// one place SEP-0009's "the last binding wins" rule is implemented.
    Map(Vec<(CdtTerm, CdtTerm)>),
}

impl FoldAccumulator {
    /// A fresh, empty accumulator for `target`.
    fn new(target: FoldTarget) -> Self {
        Self {
            parts: match target {
                FoldTarget::List => FoldParts::List(Vec::new()),
                FoldTarget::Map => FoldParts::Map(Vec::new()),
            },
        }
    }

    /// How many top-level elements this accumulator holds.
    fn len(&self) -> usize {
        match &self.parts {
            FoldParts::List(items) => items.len(),
            FoldParts::Map(pairs) => pairs.len(),
        }
    }

    /// Fold one already-evaluated row in.
    ///
    /// # Errors
    ///
    /// [`EvalError::CompositeBound`] when the row's value cannot be an element of
    /// any composite (a nested composite already at the nesting bound — see
    /// [`crate::cdt_fn::to_cdt_term`]), or when this fold has already reached
    /// [`purrdf_cdt::MAX_ELEMENTS`] top-level elements.
    ///
    /// The element bound is checked HERE, per row, as well as at `finish`. The
    /// `finish` check is the authoritative one (it counts nested elements and
    /// measures the canonical form too); this one exists so a group of a hundred
    /// million rows is refused at the bound instead of after building a hundred
    /// million `CdtTerm`s that the authoritative check would then reject. It can
    /// only ever fire on a fold `finish` would refuse anyway, so it changes no
    /// answer — only the peak memory of reaching the refusal.
    fn push(&mut self, row: &FoldRow) -> Result<(), EvalError> {
        if self.len() >= MAX_ELEMENTS {
            return Err(crate::cdt_fn::bound(&CdtError::TooManyElements {
                offset: 0,
                limit: MAX_ELEMENTS,
            }));
        }
        match &mut self.parts {
            FoldParts::List(items) => {
                // A row whose expression was unbound or raised is the `null`
                // ELEMENT, retained and counted — `argument_element` is precisely
                // the constructor-argument rule, and `FOLD`'s elements are
                // constructor arguments spread over rows.
                items.push(argument_element(row.first.as_ref())?);
            }
            FoldParts::Map(pairs) => {
                // Same rule on both halves, and the ASYMMETRY that follows is
                // `purrdf_cdt::map_constructor`'s, not this module's: a `null`
                // (or blank-node) KEY drops the whole entry, while a `null` VALUE
                // keeps it and stores the null. `fold-map-03.rq`, `-04.rq` and
                // `-05.rq` pin the three cases.
                pairs.push((
                    argument_element(row.first.as_ref())?,
                    argument_element(row.second.as_ref())?,
                ));
            }
        }
        Ok(())
    }
}

impl AggregateAccumulator for FoldAccumulator {
    /// The trait's fully-bound step: `args` is one already-bound value for the
    /// `cdt:List` form, two for the `cdt:Map` form.
    ///
    /// This is a special case of [`FoldAccumulator::push`] — the one where no
    /// position is the SEP-0009 `null` — and it is what makes this type a genuine
    /// member of the crate's one fold algebra rather than a lookalike. A missing
    /// position is read as `null`, matching what `push` does with an unbound one.
    ///
    /// # Errors
    ///
    /// [`FoldAccumulator::push`]'s errors.
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        self.push(&FoldRow {
            first: args.first().cloned(),
            second: args.get(1).cloned(),
        })
    }

    /// Append `other`'s elements after this accumulator's own.
    ///
    /// `self` holds the EARLIER partial fold and `other` the later one (the
    /// chunk-order contract on [`AggregateAccumulator::combine`]), so appending
    /// is exactly the order-dependent merge this aggregate needs: it reproduces
    /// the sequential fold's element order, and therefore also which value a
    /// repeated map key keeps.
    ///
    /// # Errors
    ///
    /// [`EvalError::Function`] if `other` is not a [`FoldAccumulator`] (impossible
    /// by construction — see [`crate::agg_fn::downcast_combine_partial`]), and
    /// [`EvalError::internal`] if the two partials somehow disagree about their
    /// composite datatype, which one `FOLD` call's single `init` factory cannot
    /// produce.
    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        let other: Self = crate::agg_fn::downcast_combine_partial(other)?;
        match (&mut self.parts, other.parts) {
            (FoldParts::List(items), FoldParts::List(more)) => items.extend(more),
            (FoldParts::Map(pairs), FoldParts::Map(more)) => pairs.extend(more),
            (FoldParts::List(_) | FoldParts::Map(_), _) => {
                return Err(EvalError::internal(
                    "FOLD combined two partial accumulators built for different composite \
                     datatypes; every partial of one fold comes from the same init factory",
                ));
            }
        }
        Ok(())
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    /// Mint the group's composite, in `purrdf-cdt`'s canonical form.
    ///
    /// Never unbound, including for a group nothing was folded into: an empty
    /// fold is `"[]"^^cdt:List` / `"{}"^^cdt:Map`, not `None`.
    ///
    /// # Errors
    ///
    /// [`EvalError::CompositeBound`] when the folded value crosses one of
    /// `purrdf-cdt`'s three bounds. A hard failure of the query, never an unbound
    /// answer — see `crate::cdt_fn`'s tri-state contract for why degrading a
    /// refused mint to `None` would let a resource refusal change a result set.
    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        let (outcome, empty) = match self.parts {
            FoldParts::List(items) => (purrdf_cdt::list_constructor(items), CdtValue::empty_list()),
            // Where SEP-0009's "the last binding wins" rule for a repeated key
            // is applied, over the folded order this accumulator preserved.
            FoldParts::Map(pairs) => (purrdf_cdt::map_constructor(&pairs), CdtValue::empty_map()),
        };
        match outcome {
            purrdf_cdt::CdtOutcome::Value(value) => Ok(Some(composite_literal(&value))),
            // Neither constructor has an `Error` outcome — every argument shape is
            // a value one of them accepts — so this arm exists only because
            // `CdtOutcome` is a three-state type this crate does not own. The empty
            // composite of the same datatype is the honest answer for "nothing to
            // build", and it keeps the never-unbound rule above true on every path.
            purrdf_cdt::CdtOutcome::Error(_) => Ok(Some(composite_literal(&empty))),
            purrdf_cdt::CdtOutcome::Bound(error) => Err(crate::cdt_fn::bound(&error)),
        }
    }
}

/// Evaluate one `FOLD` aggregate over a group's rows.
///
/// `idxs` indexes `rows` in the group's own row order, exactly as
/// `crate::modifier`'s `eval_aggregate` supplies it. The two phases are the ones
/// this module's docs describe: evaluate + de-duplicate + order (here), then fold
/// (through [`FoldAccumulator`], driven by [`fold_builtin`]).
///
/// # Errors
///
/// Any error the argument or sort-key expressions raise, and
/// [`EvalError::CompositeBound`] for a fold that crosses one of `purrdf-cdt`'s
/// bounds. A refused governor charge is recorded on `ctx.expression_barrier` and
/// answered as unbound, the doctrine every aggregate in `crate::modifier` follows.
pub(crate) fn eval_fold<D: DatasetView + Sync>(
    agg: &AggregateExpression,
    idxs: &[usize],
    rows: &[Solution<D::Id>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let Some(first_arg) = agg.args().first() else {
        return Err(EvalError::internal(
            "FOLD reached evaluation with an empty exprlist; AggregateExpression::new admits \
             only one argument (the cdt:List form) or two (the cdt:Map form)",
        ));
    };
    let second_arg = agg.args().get(1);
    let target = if second_arg.is_some() {
        FoldTarget::Map
    } else {
        FoldTarget::List
    };
    let order_by: &[OrderExpression] = agg.order_by();
    let width = order_by.len();

    // Phase 1. `DISTINCT` keys on the exprlist's own terms — see the module docs
    // for why that is TERM identity and why it ignores the sort key.
    let mut seen: FoldDistinct<D::Id> = agg.distinct.then(crate::DetHashSet::default);
    let mut survivors: Vec<FoldRow> = Vec::new();
    // Row `i`'s sort values are `sort_values[i * width..][..width]`, flattened so
    // the projected keys below can borrow one contiguous buffer.
    let mut sort_values: Vec<Option<TermValue>> = Vec::new();

    for &i in idxs {
        let row = &rows[i];
        let first = eval_expr(first_arg, row, schema, ctx)?;
        let second = match second_arg {
            Some(expression) => eval_expr(expression, row, schema, ctx)?,
            None => None,
        };
        // Charged for every row `FOLD` inspects, whether or not `DISTINCT` keeps
        // it — see `ChargePoint::AggregateAccumulation`'s doc for why the charge
        // precedes the dedup check in every aggregate.
        if let Err(tripped) = ctx.charge(ChargePoint::AggregateAccumulation) {
            ctx.expression_barrier.record(tripped);
            return Ok(None);
        }
        if let Some(seen) = seen.as_mut()
            && !seen.insert([first, second])
        {
            continue;
        }
        let retained = FoldRow {
            first: first.map(|term| ctx.scratch.value_of(ctx.dataset, term)),
            second: second.map(|term| ctx.scratch.value_of(ctx.dataset, term)),
        };
        // `value_of` mints nothing (it clones an already-interned value back out),
        // so the arena's automatic per-node charge never sees this buffer; the
        // retained clones are real, otherwise-uncharged memory proportional to the
        // group's cardinality, charged here exactly as `eval_aggregate`'s own
        // survivor buffer is.
        if let Err(tripped) = ctx.charge_amount(
            purrdf_core::ResourceDimension::ScratchBytes,
            row_bytes(&retained),
        ) {
            ctx.expression_barrier.record(tripped);
            return Ok(None);
        }
        survivors.push(retained);

        // Only a SURVIVOR's sort key is ever needed: `DISTINCT` keeps each value's
        // first occurrence, so that row's own key is the one the element sorts by.
        for order in order_by {
            let key = eval_expr(order_sort_key(order), row, schema, ctx)?;
            let key = key.map(|term| ctx.scratch.value_of(ctx.dataset, term));
            if let Err(tripped) = ctx.charge_amount(
                purrdf_core::ResourceDimension::ScratchBytes,
                key.as_ref().map_or(0, crate::scratch::value_bytes),
            ) {
                ctx.expression_barrier.record(tripped);
                return Ok(None);
            }
            sort_values.push(key);
        }
    }

    // The aggregate's own `ORDER BY`, over the crate's ONE projection of SPARQL's
    // solution ordering. `sort_by` is stable, so conditions that do not separate
    // two rows leave them in row order.
    if width > 0 {
        let keys: Vec<SortKey<'_>> = sort_values.iter().map(|v| project(v.as_ref())).collect();
        let mut order: Vec<usize> = (0..survivors.len()).collect();
        order.sort_by(|a, b| compare_keys(&keys[a * width..], &keys[b * width..], order_by));
        let mut source = survivors;
        survivors = order
            .into_iter()
            .map(|i| std::mem::take(&mut source[i]))
            .collect();
    }

    // Phase 2: the shared built-in tail, over this module's accumulator.
    let value = fold_builtin(
        &survivors,
        || FoldAccumulator::new(target),
        FoldAccumulator::push,
    )?;
    Ok(value.map(|v| ctx.scratch.intern(ctx.dataset, v)))
}

/// The scratch-byte cost of one retained [`FoldRow`], through the same
/// deterministic per-value proxy the arena's own automatic charge uses.
fn row_bytes(row: &FoldRow) -> u64 {
    let first = row.first.as_ref().map_or(0, crate::scratch::value_bytes);
    let second = row.second.as_ref().map_or(0, crate::scratch::value_bytes);
    first.saturating_add(second)
}
