// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! An indexed ORIGINAL existential inner, never an absence/index boolean.
//! The same native correlated evaluator produces the retained barrier rows.
//! Numeric indexing only selects its between interval; the existential owner
//! still receives those rows and the WHOLE reached search handle, even empty.

use super::window::{compare, parsed};
use crate::eval::EvalCtx;
use crate::expr::SubstitutionSourceMap;
use crate::governor::lift::{Evaluated, Truncation};
use crate::parsed_value::ParsedValue;
use crate::scratch::SolutionTerm;
use crate::solution::{RetainedRow, RowsBuilder, SolutionSeq, VarSchema};
use crate::workspace::{LexicalFrame, SharedWorkspace};
use crate::{AdmittedMap, AdmittedVec, EvalError, WorkspaceAllocation, WorkspaceCapability};
use core::cmp::Ordering;
use purrdf_core::{DatasetView, ViewTermId};
use purrdf_lex::allocation::Memory;
use purrdf_sparql_algebra::{Child, Expression, GraphPattern, Query, Variable};

#[derive(Clone)]
struct Between {
    position: Variable,
    left: Variable,
    right: Variable,
}
struct Plan {
    base: Query,
    range: Query,
    between: Between,
    reads: VarSchema,
    source: SubstitutionSourceMap,
    sites: std::sync::OnceLock<SharedWorkspace<crate::deferred_exists::NestedSites>>,
    program: SharedWorkspace<crate::vm::ExprProgram>,
    _allocation: Option<WorkspaceAllocation>,
}
struct Indexed<I: ViewTermId> {
    inner: SolutionSeq<I>,
    numeric: AdmittedVec<(usize, ParsedValue)>,
    unknown: AdmittedVec<usize>,
}
type DomainKey<I> = (usize, (u8, Option<I>), VarSchema, RetainedRow<I>);
pub(crate) struct BarrierState<I: ViewTermId> {
    pub(crate) preparing: bool,
    plans: AdmittedMap<usize, Option<SharedWorkspace<Plan>>>,
    domains: AdmittedMap<DomainKey<I>, SharedWorkspace<Indexed<I>>>,
}
impl<I: ViewTermId> Default for BarrierState<I> {
    fn default() -> Self {
        Self {
            preparing: false,
            plans: AdmittedMap::default(),
            domains: AdmittedMap::default(),
        }
    }
}
impl<I: ViewTermId> BarrierState<I> {
    pub(crate) fn fork(&self) -> Self {
        Self {
            preparing: self.preparing,
            ..Self::default()
        }
    }
}

/// Exact between-position form with an eager, source-defined conjunction.
/// Nested NAF remains the original dataset-only body, not a generated predicate.
pub(crate) fn is_between(
    pattern: &GraphPattern,
    workspace: &WorkspaceCapability,
) -> Result<bool, EvalError> {
    Ok(certificate(pattern, workspace)?.is_some())
}
fn certificate(
    pattern: &GraphPattern,
    workspace: &WorkspaceCapability,
) -> Result<Option<Between>, EvalError> {
    let GraphPattern::Filter { inner, expr } = pattern else {
        return Ok(None);
    };
    if !matches!(&**inner, GraphPattern::Bgp { .. }) {
        return Ok(None);
    }
    let Expression::And(_) = expr else {
        return Ok(None);
    };
    let mut pending = AdmittedVec::new(workspace);
    pending.push(expr)?;
    let mut lower = None;
    let mut upper = None;
    while let Some(expression) = pending.pop() {
        if let Expression::And(parts) = expression {
            for part in parts {
                pending.push(part)?;
            }
            continue;
        }
        if let Some((between, is_upper)) = comparison(expression) {
            let place = if is_upper { &mut upper } else { &mut lower };
            if place.is_some() {
                return Ok(None);
            }
            *place = Some(between);
            continue;
        }
        if let Expression::Not(inner) = expression
            && let Expression::Exists(body) = &**inner
            && dataset_only(body, workspace)?
        {
            continue;
        }
        if !super::window::pure_ordinal_predicate(expression, workspace)? {
            return Ok(None);
        }
    }
    let (Some(lower), Some(upper)) = (lower, upper) else {
        return Ok(None);
    };
    if lower.position != upper.position
        || lower.left != upper.left
        || lower.right != upper.right
        || lower.position == lower.left
        || lower.position == lower.right
        || lower.left == lower.right
    {
        return Ok(None);
    }
    Ok(Some(lower))
}
fn comparison(expression: &Expression) -> Option<(Between, bool)> {
    let (position, endpoint, upper) = match expression {
        Expression::Greater(a, b) => match (&**a, &**b) {
            (Expression::Variable(_), _) => (&**a, &**b, false),
            _ => (&**b, &**a, true),
        },
        Expression::Less(a, b) => match (&**a, &**b) {
            (Expression::Variable(_), _) => (&**a, &**b, true),
            _ => (&**b, &**a, false),
        },
        _ => return None,
    };
    let Expression::Variable(position) = position else {
        return None;
    };
    let Expression::If(test, yes, no) = endpoint else {
        return None;
    };
    let Expression::Less(a, b) = &**test else {
        return None;
    };
    let (
        Expression::Variable(left),
        Expression::Variable(right),
        Expression::Variable(yes),
        Expression::Variable(no),
    ) = (&**a, &**b, &**yes, &**no)
    else {
        return None;
    };
    if if upper {
        yes != right || no != left
    } else {
        yes != left || no != right
    } {
        return None;
    }
    Some((
        Between {
            position: position.clone(),
            left: left.clone(),
            right: right.clone(),
        },
        upper,
    ))
}
fn dataset_only(
    pattern: &GraphPattern,
    workspace: &WorkspaceCapability,
) -> Result<bool, EvalError> {
    let mut pending = AdmittedVec::new(workspace);
    pending.push(pattern)?;
    while let Some(pattern) = pending.pop() {
        match pattern {
            GraphPattern::Bgp { .. } => {}
            GraphPattern::Filter { inner, expr }
                if super::window::pure_ordinal_predicate(expr, workspace)? =>
            {
                pending.push(inner)?;
            }
            GraphPattern::Join { left, right } => {
                pending.push(left)?;
                pending.push(right)?;
            }
            _ => return Ok(false),
        }
    }
    Ok(true)
}

fn modify(
    expression: &mut Expression,
    keep_range: bool,
    truth: &purrdf_sparql_algebra::Literal,
    workspace: &WorkspaceCapability,
) -> Result<(), EvalError> {
    let mut pending = AdmittedVec::new(workspace);
    pending.push(expression)?;
    while let Some(expression) = pending.pop() {
        if let Expression::And(parts) = expression {
            for part in parts {
                pending.push(part)?;
            }
        } else if comparison(expression).is_some() != keep_range {
            *expression = Expression::Literal(truth.clone());
        }
    }
    Ok(())
}
impl Plan {
    fn prepare(
        pattern: &GraphPattern,
        upstream: &SubstitutionSourceMap,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<Self>, EvalError> {
        let Some(between) = certificate(pattern, workspace)? else {
            return Ok(None);
        };
        let mut frame = LexicalFrame::new(workspace);
        let mut memory = Memory::new(&mut frame);
        let mut base = pattern.clone_with_memory(&mut memory).map_err(|error| {
            memory
                .admission_mut()
                .storage_error(error, "ordinal barrier clone")
        })?;
        let mut source = SubstitutionSourceMap::default();
        crate::enf::map_clone_admitted(pattern, &base, &mut source, workspace)?;
        for (_, origin) in source.iter_mut() {
            if let Some(parent) = upstream.get(&origin.source) {
                origin.source = parent.source;
                origin.counts_rows &= parent.counts_rows;
            }
        }
        let GraphPattern::Filter { expr, .. } = &mut base else {
            unreachable!("certificate is a Filter");
        };
        let mut range_expr = expr.clone_with_memory(&mut memory).map_err(|error| {
            memory
                .admission_mut()
                .storage_error(error, "ordinal comparison clone")
        })?;
        let truth = crate::expr::correlated_boolean(true, workspace)?;
        modify(expr, false, &truth, workspace)?;
        modify(&mut range_expr, true, &truth, workspace)?;
        let reads = crate::expr::pattern_all_vars_admitted(&base, workspace)?;
        if reads.contains(&between.left) || reads.contains(&between.right) {
            return Ok(None);
        }
        let empty = Child::try_new(
            GraphPattern::Bgp {
                patterns: Vec::new(),
            },
            &mut memory,
        )
        .map_err(|error| {
            memory
                .admission_mut()
                .storage_error(error, "ordinal comparison child")
        })?;
        let base = Query::ask(base);
        let range = Query::ask(GraphPattern::Filter {
            inner: empty,
            expr: range_expr,
        });
        // These are refunds of ORIGINAL clone/box allocations after replaced
        // branches died. A retained census never acquires storage after birth.
        let actual = base
            .raw_owned_bytes_with_memory(&mut memory)
            .and_then(|base| {
                range
                    .raw_owned_bytes_with_memory(&mut memory)
                    .and_then(|range| {
                        base.checked_add(range)
                            .ok_or(purrdf_lex::allocation::StorageError::SizeOverflow)
                    })
            })
            .map_err(|error| {
                memory
                    .admission_mut()
                    .storage_error(error, "ordinal clone release")
            })?;
        let released = memory
            .admitted_bytes()
            .checked_sub(actual)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        memory.release_bytes(released).map_err(|error| {
            memory
                .admission_mut()
                .storage_error(error, "ordinal clone refund")
        })?;
        let GraphPattern::Filter { expr, .. } = range.pattern() else {
            unreachable!("range wrapper");
        };
        let program = SharedWorkspace::new_admitted(
            crate::vm::ExprProgram::compile_admitted(expr, workspace)?,
            workspace,
        )?;
        Ok(Some(Self {
            base,
            range,
            between,
            reads,
            source,
            sites: std::sync::OnceLock::new(),
            program,
            _allocation: frame.into_allocation(),
        }))
    }
}

fn stopped<D: DatasetView + Sync>(
    pattern: &GraphPattern,
    ctx: &EvalCtx<'_, D>,
) -> Result<Option<Evaluated<D::Id>>, EvalError> {
    let Some(tripped) = ctx.stop_check() else {
        return Ok(None);
    };
    Ok(Some(Evaluated::Truncated(Truncation::barred_at(
        pattern,
        tripped,
        crate::eval::syntactic_schema_admitted(pattern, &ctx.growth)?,
    ))))
}
/// Return an indexed original inner for the existing existential publication
/// seam, or None for a nonmatch. An accepted producer never retries on failure.
/// The caller must continue through existential_evidence/absence/VM negation.
pub(crate) fn inner<D: DatasetView + Sync>(
    site: usize,
    pattern: &GraphPattern,
    source: &SubstitutionSourceMap,
    outer: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<Evaluated<D::Id>>, EvalError> {
    // Numeric false does not discharge a reached arbitrary-source expression
    // declaration. No no-gap certificate exists in the original source API.
    if ctx.options.disable_shapes
        || ctx.ordinal_barriers.preparing
        || ctx.in_substituted_exists
        || ctx.evidence_input.is_some()
    {
        return Ok(None);
    }
    if let Some(stopped) = stopped(pattern, ctx)? {
        return Ok(Some(stopped));
    }
    let growth = ctx.growth.clone();
    let plan = if let Some(plan) = ctx.ordinal_barriers.plans.get(&site) {
        plan.clone()
    } else {
        let plan = Plan::prepare(pattern, source, &growth)?
            .map(|plan| SharedWorkspace::new_admitted(plan, &growth))
            .transpose()?;
        ctx.ordinal_barriers
            .plans
            .insert_admitted(site, plan.clone(), &growth)?;
        plan
    };
    let Some(plan) = plan else {
        return Ok(None);
    };
    let mut binding = AdmittedVec::with_capacity(schema.len(), &growth)?;
    for (column, variable) in schema.vars().iter().enumerate() {
        binding.push(if plan.reads.contains(variable) {
            outer[column]
        } else {
            None
        })?;
    }
    let domain = RetainedRow::from_cells(binding.len(), binding.iter().copied(), &growth)?;
    let key = (site, ctx.graph_key(), schema.clone(), domain);
    let indexed = if let Some(indexed) = ctx.ordinal_barriers.domains.get(&key) {
        indexed.clone()
    } else {
        let previous = ctx.ordinal_barriers.preparing;
        ctx.ordinal_barriers.preparing = true;
        let source = crate::deferred_exists::CorrelatedSource {
            sites: crate::deferred_exists::SiteSlot::Prepared(&plan.sites),
            plan_map: Some(&plan.source),
        };
        let result =
            crate::binop::eval_correlated(plan.base.pattern(), &binding, schema, source, ctx);
        ctx.ordinal_barriers.preparing = previous;
        let inner = match result? {
            Evaluated::Complete(inner) => inner,
            truncated @ Evaluated::Truncated(_) => return Ok(Some(truncated)),
        };
        let mut numeric = AdmittedVec::new(&growth);
        let mut unknown = AdmittedVec::new(&growth);
        let column = inner.schema.index_of(&plan.between.position);
        for (ordinal, row) in inner.rows.iter().enumerate() {
            if let Some(stopped) = stopped(pattern, ctx)? {
                return Ok(Some(stopped));
            }
            ctx.shape_state.work.add(1, 0)?;
            let value = column
                .and_then(|column| row[column])
                .map(|cell| crate::expr::owned_value_of(ctx, cell))
                .transpose()?
                .map(|value| parsed(&value, &growth))
                .transpose()?
                .flatten();
            if let Some(value) = value {
                numeric.push((ordinal, value))?;
            } else {
                unknown.push(ordinal)?;
            }
        }
        let mut failure = None;
        numeric.as_mut_slice().sort_unstable_by(|a, b| {
            if failure.is_some() || ctx.stop_check().is_some() {
                return Ordering::Equal;
            }
            if let Err(error) = ctx.shape_state.work.add(0, 1) {
                failure = Some(error);
                return Ordering::Equal;
            }
            match compare(&a.1, &b.1, &growth) {
                Ok(order) => order.then_with(|| a.0.cmp(&b.0)),
                Err(error) => {
                    failure = Some(error);
                    Ordering::Equal
                }
            }
        });
        if let Some(error) = failure {
            return Err(error);
        }
        if let Some(stopped) = stopped(pattern, ctx)? {
            return Ok(Some(stopped));
        }
        let indexed = SharedWorkspace::new_admitted(
            Indexed {
                inner,
                numeric,
                unknown,
            },
            &growth,
        )?;
        ctx.ordinal_barriers
            .domains
            .insert_admitted(key, indexed.clone(), &growth)?;
        indexed
    };
    let value = |variable: &Variable| -> Result<Option<ParsedValue>, EvalError> {
        schema
            .index_of(variable)
            .and_then(|column| outer[column])
            .map(|cell| crate::expr::owned_value_of(ctx, cell))
            .transpose()?
            .map(|value| parsed(&value, &growth))
            .transpose()
            .map(Option::flatten)
    };
    let left = value(&plan.between.left)?;
    let right = value(&plan.between.right)?;
    let mut selected = AdmittedVec::new(&growth);
    if let (Some(left), Some(right)) = (left, right) {
        let (lower, upper) = if compare(&left, &right, &growth)? == Ordering::Less {
            (left, right)
        } else {
            (right, left)
        };
        let start = boundary(
            &indexed.numeric,
            &lower,
            true,
            &growth,
            &ctx.shape_state.work,
        )?;
        let end = boundary(
            &indexed.numeric,
            &upper,
            false,
            &growth,
            &ctx.shape_state.work,
        )?;
        if start < end {
            for &(ordinal, _) in &indexed.numeric[start..end] {
                selected.push(ordinal)?;
            }
        }
        for &ordinal in &indexed.unknown {
            selected.push(ordinal)?;
        }
    } else {
        for ordinal in 0..indexed.inner.rows.len() {
            selected.push(ordinal)?;
        }
    }
    selected.as_mut_slice().sort_unstable();
    let combined = indexed.inner.schema.union_admitted(schema, &growth)?;
    let GraphPattern::Filter { expr, .. } = plan.range.pattern() else {
        unreachable!("range wrapper");
    };
    let mut linked = crate::vm::Linked::link_admitted(plan.program.clone(), expr, &combined, ctx)?;
    let mut rows = RowsBuilder::new(&growth);
    for ordinal in selected {
        if let Some(tripped) = ctx.stop_check() {
            return Ok(Some(Evaluated::Truncated(Truncation::barred_at(
                pattern,
                tripped,
                crate::eval::syntactic_schema_admitted(pattern, &growth)?,
            ))));
        }
        if let Err(tripped) = ctx.charge(crate::governor::ChargePoint::RowExpressionEvaluation) {
            return Ok(Some(Evaluated::Truncated(Truncation::barred_at(
                pattern,
                tripped,
                crate::eval::syntactic_schema_admitted(pattern, &growth)?,
            ))));
        }
        let original = &indexed.inner.rows[ordinal];
        let probe = RetainedRow::from_cells(
            combined.len(),
            combined.vars().iter().map(|variable| {
                indexed
                    .inner
                    .schema
                    .index_of(variable)
                    .and_then(|column| original[column])
                    .or_else(|| schema.index_of(variable).and_then(|column| outer[column]))
            }),
            &growth,
        )?;
        if linked.ebv(&probe, &combined, ctx)? == Some(true) {
            rows.push_row(original.clone_admitted(&growth)?)?;
        }
    }
    let mut rows = rows.finish()?;
    rows.retain_search_evidence(indexed.inner.rows.search_evidence().clone());
    Ok(Some(Evaluated::Complete(SolutionSeq {
        schema: indexed.inner.schema.clone(),
        rows,
    })))
}
fn boundary(
    values: &[(usize, ParsedValue)],
    bound: &ParsedValue,
    after_equal: bool,
    workspace: &WorkspaceCapability,
    counters: &super::runtime::ShapeCounters,
) -> Result<usize, EvalError> {
    let mut lo = 0;
    let mut hi = values.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        counters.add(0, 1)?;
        let order = compare(&values[mid].1, bound, workspace)?;
        if order == Ordering::Less || (after_equal && order == Ordering::Equal) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    Ok(lo)
}
