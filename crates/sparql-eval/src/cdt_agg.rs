// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! Phase 2 is nonetheless the ordinary [`crate::modifier::fold_builtin_admitted`] tail
//! over an ordinary [`crate::agg_fn::AggregateAccumulator`]
//! ([`FoldAccumulator`]) — one fold algebra, exactly as `crate::modifier`'s
//! dispatch documentation promises, not a second one bolted on beside it.
//!
//! # `DISTINCT` is TERM identity, and it reads only the exprlist
//!
//! `FOLD(DISTINCT ?v)` over `{"1"^^xsd:integer, "01"^^xsd:integer}` yields a list
//! of size **two**: the corpus asserts both elements with `SAMETERM`
//! (`fold-list-distinct-07.rq`, `-08.rq`), so the de-duplication is on the RDF
//! term, never on the value. The admitted map of [`SolutionTerm`] keys preserves
//! that identity — the scratch interner's promotion rule makes
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
//! projection of it, `crate::modifier`'s [`project_admitted`]/[`compare_keys_admitted`], so an
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
use crate::error::EvalError;
use crate::eval::EvalCtx;
use crate::governor::ChargePoint;
use crate::modifier::{compare_keys_admitted, fold_builtin_admitted, project_admitted};
use crate::scratch::SolutionTerm;
use crate::solution::VarSchema;
use crate::workspace::{AdmittedMap, AdmittedVec};

/// The `DISTINCT` witness set: one entry per exprlist tuple already folded, in
/// RDF-TERM identity (see the module docs), and `None` when the call carried no
/// `DISTINCT` at all.
///
/// A fixed two-slot array rather than a `Vec`, because `FOLD`s exprlist is one or
/// two expressions and nothing else — the unused second slot of the `cdt:List`
/// form is always `None`, which is one more term-identity value and never
/// conflates two distinct rows.
type FoldDistinct<I> = Option<AdmittedMap<[Option<SolutionTerm<I>>; 2], ()>>;

/// One row's already-evaluated contribution to a fold, in exprlist order.
///
/// `None` in either slot is the SEP-0009 `null`: the row's expression was unbound
/// or raised. Both are one state here because SEP-0009 treats them identically
/// (see `crate::cdt_fn`'s module docs); WHERE the null lands is what matters, and
/// that is decided by the aggregate arity, not here.
///
/// `Default` (both slots `None`) exists so the sort below can permute survivors
/// with [`std::mem::take`] rather than cloning every retained value a second time.
#[derive(Default, Debug)]
pub(crate) struct FoldRow {
    /// The first expression's value — the list element, or the map KEY.
    first: Option<crate::WorkspaceTerm>,
    /// The second expression's value — the map VALUE. Always `None` (and never
    /// read) in the list form.
    second: Option<crate::WorkspaceTerm>,
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
    owners: AdmittedVec<crate::cdt_fn::OwnedCdtTerm>,
    frame: crate::workspace::LexicalFrame,
    workspace: crate::WorkspaceCapability,
}

/// [`FoldAccumulator`]'s state, one arm per the aggregate arity.
enum FoldParts {
    /// `cdt:List` elements, in folded order.
    List(Vec<CdtTerm>),
    /// `cdt:Map` pairs, in folded order — NOT yet de-duplicated: a repeated key
    /// is resolved by [`purrdf_cdt::map_constructor`] at `finish`, which is the
    /// one place SEP-0009's "the last binding wins" rule is implemented.
    Map(Vec<(CdtTerm, CdtTerm)>),
}

impl FoldAccumulator {
    pub(crate) fn for_arguments_admitted(
        arity: usize,
        workspace: &crate::WorkspaceCapability,
    ) -> Self {
        Self {
            parts: if arity == 1 {
                FoldParts::List(Vec::new())
            } else {
                FoldParts::Map(Vec::new())
            },
            owners: AdmittedVec::new(workspace),
            frame: crate::workspace::LexicalFrame::new(workspace),
            workspace: workspace.clone(),
        }
    }
    fn len(&self) -> usize {
        match &self.parts {
            FoldParts::List(items) => items.len(),
            FoldParts::Map(items) => items.len(),
        }
    }
    fn push_values(
        &mut self,
        first: Option<&TermValue>,
        second: Option<&TermValue>,
    ) -> Result<(), EvalError> {
        if self.len() >= MAX_ELEMENTS {
            return Err(crate::cdt_fn::bound_admitted(
                &CdtError::TooManyElements {
                    offset: 0,
                    limit: MAX_ELEMENTS,
                },
                &self.workspace,
            ));
        }
        let first = self.element(first)?;
        let live = self.frame.admitted_bytes();
        let result = match &mut self.parts {
            FoldParts::List(items) => {
                purrdf_lex::allocation::Memory::resume(&mut self.frame, live).push(items, first)
            }
            FoldParts::Map(_) => {
                let second = self.element(second)?;
                let live = self.frame.admitted_bytes();
                let FoldParts::Map(items) = &mut self.parts else {
                    unreachable!()
                };
                purrdf_lex::allocation::Memory::resume(&mut self.frame, live)
                    .push(items, (first, second))
            }
        };
        result.map_err(|error| {
            crate::cdt_fn::storage_error(&mut self.frame, error, "FOLD state array")
        })
    }
    fn element(&mut self, value: Option<&TermValue>) -> Result<CdtTerm, EvalError> {
        let Some(value) = value else {
            return Ok(CdtTerm::Null);
        };
        let term = crate::cdt_fn::to_cdt_term_admitted(value, true, &self.workspace)?;
        self.keep_element(term)
    }
    fn keep_element(&mut self, term: crate::cdt_fn::OwnedCdtTerm) -> Result<CdtTerm, EvalError> {
        self.owners.push(term)?;
        Ok(std::mem::replace(
            &mut self
                .owners
                .as_mut_slice()
                .last_mut()
                .expect("element owner was just inserted")
                .value,
            CdtTerm::Null,
        ))
    }
    fn push(&mut self, row: &FoldRow) -> Result<(), EvalError> {
        self.push_values(row.first.as_deref(), row.second.as_deref())
    }
    fn finish_workspace(
        mut self,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<crate::WorkspaceTerm, EvalError> {
        let mut frame = crate::workspace::LexicalFrame::new(workspace);
        let map = matches!(self.parts, FoldParts::Map(_));
        let parts = std::mem::replace(&mut self.parts, FoldParts::List(Vec::new()));
        let (outcome, _) = match parts {
            FoldParts::List(items) => purrdf_cdt::functions::list_constructor_admitted(
                items,
                &mut crate::cdt_fn::CdtStorage::new(&mut frame),
            ),
            FoldParts::Map(pairs) => purrdf_cdt::functions::map_constructor_admitted(
                &pairs,
                &mut crate::cdt_fn::CdtStorage::new(&mut frame),
            ),
        }
        .map_err(|error| {
            crate::cdt_fn::storage_error(&mut frame, error, "FOLD composite construction")
        })?;
        match outcome {
            purrdf_cdt::CdtOutcome::Value(value) => {
                crate::cdt_fn::composite_literal_admitted(&value, workspace)
            }
            purrdf_cdt::CdtOutcome::Error(_) => crate::cdt_fn::composite_literal_admitted(
                &if map {
                    CdtValue::empty_map()
                } else {
                    CdtValue::empty_list()
                },
                workspace,
            ),
            purrdf_cdt::CdtOutcome::Bound(error) => {
                Err(crate::cdt_fn::bound_admitted(&error, workspace))
            }
        }
    }
}

impl AggregateAccumulator for FoldAccumulator {
    fn native_workspace(&self) -> Option<&crate::WorkspaceCapability> {
        Some(&self.workspace)
    }
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        self.push_values(args.first(), args.get(1))
    }
    fn step_admitted(
        &mut self,
        args: &[TermValue],
        _workspace: &crate::WorkspaceCapability,
    ) -> Result<(), EvalError> {
        self.push_values(args.first(), args.get(1))
    }
    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        let mut other: Self = crate::agg_fn::downcast_combine_partial(other)?;
        if matches!(
            (&self.parts, &other.parts),
            (FoldParts::List(_), FoldParts::Map(_)) | (FoldParts::Map(_), FoldParts::List(_))
        ) {
            return Err(crate::NativeDiagnostic::error(
                crate::NativeDiagnosticKind::Internal,
                "FOLD combined two partial accumulators built for different composite datatypes; every partial of one fold comes from the same init factory",
                &self.workspace,
            ));
        }
        self.owners.reserve_additional(other.owners.len())?;
        let result = (|| {
            let additional = match (&self.parts, &other.parts) {
                (FoldParts::List(_), FoldParts::List(more)) => more.len(),
                (FoldParts::Map(_), FoldParts::Map(more)) => more.len(),
                _ => return Err(purrdf_lex::allocation::StorageError::FormattingFailed),
            };
            let live = self.frame.admitted_bytes();
            let mut memory = purrdf_lex::allocation::Memory::resume(&mut self.frame, live);
            match &mut self.parts {
                FoldParts::List(items) => {
                    let required = items
                        .len()
                        .checked_add(additional)
                        .ok_or(purrdf_lex::allocation::StorageError::SizeOverflow)?;
                    memory.reserve(items, required)?;
                }
                FoldParts::Map(items) => {
                    let required = items
                        .len()
                        .checked_add(additional)
                        .ok_or(purrdf_lex::allocation::StorageError::SizeOverflow)?;
                    memory.reserve(items, required)?;
                }
            }
            Ok(())
        })();
        result.map_err(|error| {
            crate::cdt_fn::storage_error(&mut self.frame, error, "FOLD partial state merge")
        })?;
        self.owners.append_reserved(&mut other.owners);
        match (&mut self.parts, &mut other.parts) {
            (FoldParts::List(items), FoldParts::List(more)) => items.append(more),
            (FoldParts::Map(items), FoldParts::Map(more)) => items.append(more),
            _ => unreachable!("FOLD partial datatype was checked before transfer"),
        }
        Ok(())
    }
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }
    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        let workspace = self.workspace.clone();
        if workspace.is_bounded() {
            return Err(EvalError::WorkspaceUnpriced("raw FOLD result extraction"));
        }
        self.finish_workspace(&workspace)
            .map(|term| Some(term.into_parts().0))
    }
    fn finish_admitted(
        self: Box<Self>,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<Option<crate::WorkspaceTerm>, EvalError> {
        self.finish_workspace(workspace).map(Some)
    }
}

/// Complete the native parallel group kernel's non-suspendable FOLD.
pub(crate) fn complete_inline_fold<D: DatasetView + Sync>(
    agg: &AggregateExpression,
    links: &mut [crate::vm::Linked<'_, D::Id>],
    idxs: &[usize],
    rows: &[crate::solution::RetainedRow<D::Id>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let (arguments, keys) = links.split_at_mut(agg.args().len().min(links.len()));
    let (first_arg, mut second_arg) = match arguments {
        [first] => (first, None),
        [first, second, ..] => (first, Some(second)),
        [] => {
            return Err(EvalError::internal(
                "FOLD reached evaluation with an empty exprlist; AggregateExpression::new \
                 admits only one argument (the cdt:List form) or two (the cdt:Map form)",
            ));
        }
    };
    let order_by: &[OrderExpression] = agg.order_by();

    // Phase 1. `DISTINCT` keys on the exprlist's own terms — see the module docs
    // for why that is TERM identity and why it ignores the sort key.
    let mut seen: FoldDistinct<D::Id> = agg.distinct.then(AdmittedMap::default);
    let mut survivors = AdmittedVec::new(&ctx.growth);
    // Row `i`'s sort values are `sort_values[i * width..][..width]`, flattened so
    // the projected keys below can borrow one contiguous buffer.
    let mut sort_values = AdmittedVec::new(&ctx.growth);

    // Each value folded passes the row checkpoint (`crate::row_checkpoint`): a
    // latched trip first, then the `aggregate-accumulation` charge and its poll.
    let mut checkpoint =
        crate::row_checkpoint::RowCheckpoint::sequential(ctx, ChargePoint::AggregateAccumulation);
    for &i in idxs {
        let row = &rows[i];
        let first = first_arg.term(Some(&agg.args()[0]), row, schema, ctx)?;
        let second = match second_arg.as_mut() {
            Some(link) => link.term(Some(&agg.args()[1]), row, schema, ctx)?,
            None => None,
        };
        // Charged for every row `FOLD` inspects, whether or not `DISTINCT` keeps
        // it — see `ChargePoint::AggregateAccumulation`'s doc for why the charge
        // precedes the dedup check in every aggregate.
        if let Err(tripped) = checkpoint.pass(ctx) {
            ctx.record_barrier(tripped);
            return Ok(None);
        }
        if let Some(seen) = seen.as_mut() {
            let tuple = [first, second];
            if seen.get(&tuple).is_some() {
                continue;
            }
            let _ = seen.insert_admitted(tuple, (), &ctx.growth)?;
        }
        let retained = FoldRow {
            first: first
                .map(|term| crate::expr::owned_value_of(ctx, term))
                .transpose()?,
            second: second
                .map(|term| crate::expr::owned_value_of(ctx, term))
                .transpose()?,
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
            ctx.record_barrier(tripped);
            return Ok(None);
        }
        survivors.push(retained)?;

        // Only a SURVIVOR's sort key is ever needed: `DISTINCT` keeps each value's
        // first occurrence, so that row's own key is the one the element sorts by.
        for (key_link, order) in keys.iter_mut().zip(order_by) {
            let (OrderExpression::Asc(expression) | OrderExpression::Desc(expression)) = order;
            let key = key_link.term(Some(expression), row, schema, ctx)?;
            let key = key
                .map(|term| crate::expr::owned_value_of(ctx, term))
                .transpose()?;
            if let Err(tripped) = ctx.charge_amount(
                purrdf_core::ResourceDimension::ScratchBytes,
                key.as_deref().map_or(0, crate::scratch::value_bytes),
            ) {
                ctx.record_barrier(tripped);
                return Ok(None);
            }
            sort_values.push(key)?;
        }
    }

    finish_fold(agg, survivors, &sort_values, ctx)
}

enum FoldPhase {
    Arguments,
    Retain,
    Keys,
}

pub(crate) struct FoldSequence<'e, I: purrdf_core::ViewTermId> {
    links: AdmittedVec<crate::vm::Linked<'e, I>>,
    active: Option<crate::vm::RowExpressionFrame<'e, I>>,
    row: Option<crate::solution::RetainedRow<I>>,
    schema: crate::solution::SharedSchema,
    ordinal: usize,
    slot: usize,
    phase: FoldPhase,
    terms: [Option<SolutionTerm<I>>; 2],
    checkpoint: crate::row_checkpoint::RowCheckpoint,
    seen: FoldDistinct<I>,
    survivors: AdmittedVec<FoldRow>,
    sort_values: AdmittedVec<Option<crate::WorkspaceTerm>>,
}

#[expect(
    clippy::large_enum_variant,
    reason = "the driver admits the suspended FOLD owner"
)]
pub(crate) enum FoldStart<'e, I: purrdf_core::ViewTermId> {
    Complete {
        links: AdmittedVec<crate::vm::Linked<'e, I>>,
        value: Option<SolutionTerm<I>>,
    },
    Sequence(FoldSequence<'e, I>),
}

#[expect(
    clippy::large_enum_variant,
    reason = "the driver admits the suspended FOLD owner"
)]
pub(crate) enum FoldStep<'e, 'd, I: purrdf_core::ViewTermId> {
    Complete {
        links: AdmittedVec<crate::vm::Linked<'e, I>>,
        value: Option<SolutionTerm<I>>,
    },
    Suspend(FoldSequence<'e, I>, crate::vm::Suspend<'d>),
}

impl<'e, I: purrdf_core::ViewTermId> FoldSequence<'e, I> {
    pub(crate) fn begin<D: DatasetView<Id = I> + Sync>(
        aggregate: &AggregateExpression,
        links: AdmittedVec<crate::vm::Linked<'e, I>>,
        schema: crate::solution::SharedSchema,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<FoldStart<'e, I>, EvalError> {
        if let Err(tripped) = ctx.charge(ChargePoint::AggregateInvocation) {
            ctx.record_barrier(tripped);
            return Ok(FoldStart::Complete { links, value: None });
        }
        if !(1..=2).contains(&aggregate.args().len())
            || links.len() < aggregate.args().len() + aggregate.order_by().len()
        {
            return Err(EvalError::internal(
                "FOLD lost its argument or sort-key links",
            ));
        }
        Ok(FoldStart::Sequence(Self {
            links,
            active: None,
            row: None,
            schema,
            ordinal: 0,
            slot: 0,
            phase: FoldPhase::Arguments,
            terms: [None; 2],
            checkpoint: crate::row_checkpoint::RowCheckpoint::sequential(
                ctx,
                ChargePoint::AggregateAccumulation,
            ),
            seen: aggregate.distinct.then(AdmittedMap::default),
            survivors: AdmittedVec::new(&ctx.growth),
            sort_values: AdmittedVec::new(&ctx.growth),
        }))
    }
    /// Arguments come first, followed by the aggregate's own ORDER expressions.
    pub(crate) fn argument_index(&self) -> usize {
        self.slot
    }

    fn complete<'d>(self, value: Option<SolutionTerm<I>>) -> FoldStep<'e, 'd, I> {
        FoldStep::Complete {
            links: self.links,
            value,
        }
    }
    fn advance_row(&mut self) {
        self.ordinal += 1;
        self.row = None;
        self.terms = [None; 2];
        self.slot = 0;
        self.phase = FoldPhase::Arguments;
    }

    pub(crate) fn step<'d, D: DatasetView<Id = I> + Sync>(
        mut self,
        aggregate: &AggregateExpression,
        indices: &[usize],
        rows: &[crate::solution::RetainedRow<I>],
        ctx: &mut EvalCtx<'d, D>,
    ) -> Result<FoldStep<'e, 'd, I>, EvalError> {
        let arity = aggregate.args().len();
        loop {
            if self.row.is_none() && self.active.is_none() {
                let Some(&index) = indices.get(self.ordinal) else {
                    break;
                };
                self.row = Some(
                    rows.get(index)
                        .ok_or_else(|| EvalError::internal("FOLD source row is absent"))?
                        .clone_admitted(&ctx.growth)?,
                );
            }
            if matches!(self.phase, FoldPhase::Retain) {
                if let Err(tripped) = self.checkpoint.pass(ctx) {
                    ctx.record_barrier(tripped);
                    return Ok(self.complete(None));
                }
                if let Some(seen) = &mut self.seen {
                    if seen.get(&self.terms).is_some() {
                        self.advance_row();
                        continue;
                    }
                    let _ = seen.insert_admitted(self.terms, (), &ctx.growth)?;
                }
                let retained = FoldRow {
                    first: self.terms[0]
                        .map(|term| crate::expr::owned_value_of(ctx, term))
                        .transpose()?,
                    second: self.terms[1]
                        .map(|term| crate::expr::owned_value_of(ctx, term))
                        .transpose()?,
                };
                if let Err(tripped) = ctx.charge_amount(
                    purrdf_core::ResourceDimension::ScratchBytes,
                    row_bytes(&retained),
                ) {
                    ctx.record_barrier(tripped);
                    return Ok(self.complete(None));
                }
                self.survivors.push(retained)?;
                self.phase = FoldPhase::Keys;
                self.slot = arity;
            }
            if matches!(self.phase, FoldPhase::Keys)
                && self.slot == arity + aggregate.order_by().len()
            {
                self.advance_row();
                continue;
            }
            let mut frame = if let Some(frame) = self.active.take() {
                frame
            } else {
                let link = crate::vm::take_slot(&mut self.links, self.slot)?;
                crate::vm::RowExpressionFrame::begin(
                    link,
                    self.row.take().expect("one FOLD source row"),
                    ctx,
                )?
            };
            match frame.poll(ctx)? {
                crate::vm::VmStep::Suspend(request) => {
                    self.active = Some(frame);
                    return Ok(FoldStep::Suspend(self, request));
                }
                crate::vm::VmStep::Value(value) => {
                    let (link, row, term, _evidence) = frame.finish_term(value)?;
                    crate::vm::restore_slot(&mut self.links, self.slot, link);
                    self.row = Some(row);
                    if matches!(self.phase, FoldPhase::Arguments) {
                        self.terms[self.slot] = term;
                        self.slot += 1;
                        if self.slot == arity {
                            self.phase = FoldPhase::Retain;
                        }
                    } else {
                        let key = term
                            .map(|term| crate::expr::owned_value_of(ctx, term))
                            .transpose()?;
                        if let Err(tripped) = ctx.charge_amount(
                            purrdf_core::ResourceDimension::ScratchBytes,
                            key.as_deref().map_or(0, crate::scratch::value_bytes),
                        ) {
                            ctx.record_barrier(tripped);
                            return Ok(self.complete(None));
                        }
                        self.sort_values.push(key)?;
                        self.slot += 1;
                    }
                }
            }
        }
        let survivors = std::mem::replace(&mut self.survivors, AdmittedVec::new(&ctx.growth));
        let value = finish_fold(aggregate, survivors, &self.sort_values, ctx)?;
        Ok(self.complete(value))
    }
}

fn finish_fold<D: DatasetView + Sync>(
    agg: &AggregateExpression,
    mut survivors: AdmittedVec<FoldRow>,
    sort_values: &[Option<crate::WorkspaceTerm>],
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let order_by = agg.order_by();
    let width = order_by.len();
    if width > 0 {
        let mut keys = AdmittedVec::with_capacity(sort_values.len(), &ctx.growth)?;
        for value in sort_values {
            keys.push(project_admitted(value.as_deref(), &ctx.growth)?)?;
        }
        let cost = crate::modifier::sort_keys_numeric_cost_admitted(
            &keys.iter().map(|key| &key.key),
            keys.len(),
            width,
            &ctx.growth,
        )?;
        if !crate::expr::numeric_step_admitted(ctx, cost) {
            return Ok(None);
        }
        let order = crate::modifier::owners::order_permutation(
            survivors.len(),
            &ctx.growth,
            |left, right| {
                compare_keys_admitted(
                    &keys[left * width..],
                    &keys[right * width..],
                    order_by,
                    &ctx.growth,
                )
            },
        )?;
        let mut reordered = AdmittedVec::with_capacity(survivors.len(), &ctx.growth)?;
        for position in order {
            reordered.push(std::mem::take(&mut survivors.as_mut_slice()[position]))?;
        }
        survivors = reordered;
    }
    let workspace = ctx.growth.clone();
    let value = fold_builtin_admitted(
        ctx.sequential_operation_required(),
        &survivors,
        || {
            Ok(FoldAccumulator::for_arguments_admitted(
                agg.args().len(),
                &workspace,
            ))
        },
        FoldAccumulator::push,
        &workspace,
    )?;
    value
        .map(|value| ctx.intern_workspace_term(value))
        .transpose()
        .map(Option::flatten)
}

crate::vm::row_continuation!(FoldSequence, schema);

/// The scratch-byte cost of one retained [`FoldRow`], through the same
/// deterministic per-value proxy the arena's own automatic charge uses.
fn row_bytes(row: &FoldRow) -> u64 {
    let first = row.first.as_deref().map_or(0, crate::scratch::value_bytes);
    let second = row.second.as_deref().map_or(0, crate::scratch::value_bytes);
    first.saturating_add(second)
}
