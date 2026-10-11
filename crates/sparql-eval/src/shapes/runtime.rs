// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::{
    ShapeBinding, ShapeInput, ShapeOffer, ShapeRequest, ShapeRow, ShapeRuntime, ShapeWork,
};
use crate::eval::EvalCtx;
use crate::governor::lift::{Evaluated, Truncation};
use crate::solution::{RetainedRow, RowsBuilder, SolutionSeq, VarSchema};
use crate::workspace::SharedWorkspace;
use crate::{AdmittedMap, AdmittedVec, EvalError, WorkspaceCapability, WorkspaceTerm};
use purrdf_core::dataset_view::lock_read_state;
use purrdf_core::{DatasetView, GraphMatch, ViewTermId};
use purrdf_sparql_algebra::{GraphPattern, Variable};

type SelectionKey<I> = (
    crate::plan::TreeKey,
    crate::plan::NodeId,
    u64,
    u8,
    Option<I>,
);
type Selections<I> = AdmittedMap<SelectionKey<I>, Option<SharedWorkspace<ShapeOffer>>>;
type InputRow<'a, I> = (&'a SolutionSeq<I>, &'a RetainedRow<I>);

/// Execution-owned selections, keyed by the actual rewritten node AND graph.
/// Substituted trees are re-numbered; no borrowed query is cached here.
pub(crate) struct ShapeState<I: ViewTermId> {
    selections: Selections<I>,
    reports: Option<SharedWorkspace<std::sync::Mutex<AdmittedVec<ShapeReport>>>>,
    pub(crate) work: ShapeCounters,
}
impl<I: ViewTermId> Default for ShapeState<I> {
    fn default() -> Self {
        Self {
            selections: AdmittedMap::default(),
            reports: None,
            work: ShapeCounters::default(),
        }
    }
}
impl<I: ViewTermId> ShapeState<I> {
    pub(crate) fn fork(&self) -> Self {
        Self {
            reports: self.reports.clone(),
            ..Self::default()
        }
    }
    pub(crate) fn ensure_reports(
        &mut self,
        workspace: &WorkspaceCapability,
    ) -> Result<(), EvalError> {
        if self.reports.is_none() {
            self.reports = Some(SharedWorkspace::new_admitted(
                std::sync::Mutex::new(AdmittedVec::new(workspace)),
                workspace,
            )?);
        }
        Ok(())
    }
    fn report(
        &mut self,
        report: ShapeReport,
        workspace: &WorkspaceCapability,
    ) -> Result<(), EvalError> {
        self.ensure_reports(workspace)?;
        lock_read_state(self.reports.as_ref().expect("report home installed")).push(report)
    }
    pub(crate) fn take_reports(
        &self,
        workspace: &WorkspaceCapability,
    ) -> Result<AdmittedVec<ShapeReport>, EvalError> {
        let Some(reports) = &self.reports else {
            return Ok(AdmittedVec::new(workspace));
        };
        let mut reports =
            std::mem::replace(&mut *lock_read_state(reports), AdmittedVec::new(workspace));
        reports
            .as_mut_slice()
            .sort_unstable_by_key(|report| (report.node, report.operator));
        // Aggregate repeated worker/site invocations at the same original node.
        let mut combined: AdmittedVec<ShapeReport> = AdmittedVec::new(workspace);
        for report in reports {
            if let Some(prior) = combined.as_mut_slice().last_mut()
                && prior.node == report.node
                && prior.operator == report.operator
                && prior.estimate == report.estimate
            {
                prior.work.source_visits = prior
                    .work
                    .source_visits
                    .checked_add(report.work.source_visits)
                    .ok_or(EvalError::WorkspaceBoundOverflow)?;
                prior.work.transitions = prior
                    .work
                    .transitions
                    .checked_add(report.work.transitions)
                    .ok_or(EvalError::WorkspaceBoundOverflow)?;
                prior.work.output_rows = prior
                    .work
                    .output_rows
                    .checked_add(report.work.output_rows)
                    .ok_or(EvalError::WorkspaceBoundOverflow)?;
            } else {
                combined.push(report)?;
            }
        }
        Ok(combined)
    }
}
#[derive(Default)]
pub(crate) struct ShapeCounters {
    visits: std::sync::atomic::AtomicU64,
    transitions: std::sync::atomic::AtomicU64,
}
impl ShapeCounters {
    pub(crate) fn snapshot(&self) -> ShapeWork {
        let order = std::sync::atomic::Ordering::Relaxed;
        ShapeWork {
            source_visits: self.visits.load(order),
            transitions: self.transitions.load(order),
            output_rows: 0,
        }
    }
    pub(crate) fn add(&self, visits: u64, transitions: u64) -> Result<(), EvalError> {
        let order = std::sync::atomic::Ordering::Relaxed;
        self.visits
            .try_update(order, order, |n| n.checked_add(visits))
            .map_err(|_| EvalError::WorkspaceBoundOverflow)?;
        self.transitions
            .try_update(order, order, |n| n.checked_add(transitions))
            .map_err(|_| EvalError::WorkspaceBoundOverflow)?;
        Ok(())
    }
}
/// Selection and actual work recorded for the original logical node.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShapeReport {
    /// Dense original plan node, when numbered.
    pub node: Option<usize>,
    /// Selected operator, or a legitimate decline.
    pub operator: Option<&'static str>,
    /// Original registered provider, or the native built-in home.
    pub provider: Option<&'static str>,
    /// Physical host registry instance; separate from logical content identity.
    pub registry: Option<crate::registry_id::RegistryId>,
    /// Actual visits/transitions/output, never an admission fee.
    pub work: ShapeWork,
    /// The accepted comparable estimate, separately from actual measured work.
    pub estimate: super::ShapeEstimate,
}

fn violation(workspace: &WorkspaceCapability, message: &'static str) -> EvalError {
    crate::error::NativeDiagnostic::error(
        crate::error::NativeDiagnosticKind::Function,
        message,
        workspace,
    )
}

pub(crate) fn eval_shape<D: DatasetView + Sync>(
    pattern: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<Evaluated<D::Id>>, EvalError> {
    if ctx.options.disable_shapes || ctx.expanded_dictionary.is_some() {
        return Ok(None);
    }
    if ctx.shapes.providers.is_empty()
        && !matches!(
            pattern,
            GraphPattern::Filter { .. }
                | GraphPattern::Group { .. }
                | GraphPattern::Lateral { .. }
                | GraphPattern::Union { .. }
        )
    {
        return Ok(None);
    }
    let graph = match ctx.active_graph {
        GraphMatch::Default => (0, None),
        GraphMatch::Any => (1, None),
        GraphMatch::Named(id) => (2, Some(id)),
    };
    let key = ctx.plan.as_ref().and_then(|plan| {
        plan.node_of(pattern)
            .map(|node| (plan.key(), node, ctx.shapes.id().as_u64(), graph.0, graph.1))
    });
    let cached = key.and_then(|key| ctx.shape_state.selections.get(&key));
    let was_cached = cached.is_some();
    let offer = if let Some(cached) = cached {
        cached.clone()
    } else {
        let mut selected = None;
        if !ctx.shapes.providers.is_empty() {
            let schema = crate::eval::syntactic_schema_admitted(pattern, &ctx.growth)?;
            let costs = NativeCosts(ctx);
            let request = || ShapeRequest {
                pattern,
                schema: &schema,
                relations: ctx.property_functions,
                named_graph: matches!(ctx.active_graph, GraphMatch::Named(_)),
                has_evidence_source: ctx.evidence_input.is_some(),
                costs: &costs,
            };
            for (name, provider) in &ctx.shapes.providers {
                selected = crate::contain::call_contained_admitted(
                    "shape provider",
                    name,
                    "offer",
                    || provider.offer(request(), &ctx.growth),
                    &ctx.growth,
                )?;
                if let Some(offer) = &mut selected {
                    offer.provider = Some(*name);
                    break;
                }
            }
        }
        if selected.is_none() {
            selected = super::dictionary::offer(
                pattern,
                ctx.property_functions,
                ctx.evidence_input.is_some(),
                &ctx.growth,
            )?;
        }
        if selected.is_none() && ctx.dataset.certifies_probe_order() {
            selected = super::window::offer(pattern, &ctx.growth)?;
        }
        if selected.is_none() {
            selected = super::bucket::offer(pattern, &ctx.growth)?;
        }
        let selected = selected
            .map(|offer| SharedWorkspace::new_admitted(offer, &ctx.growth))
            .transpose()?;
        if let Some(key) = key
            && (selected.is_some() || !ctx.shapes.providers.is_empty())
        {
            ctx.shape_state
                .selections
                .insert_admitted(key, selected.clone(), &ctx.growth)?;
        }
        selected
    };
    let Some(offer) = offer else {
        if !was_cached && !ctx.shapes.providers.is_empty() {
            let node = ctx.plan_node(pattern).map(crate::plan::NodeId::index);
            ctx.shape_state.report(
                ShapeReport {
                    node,
                    registry: Some(ctx.shapes.id()),
                    ..ShapeReport::default()
                },
                &ctx.growth,
            )?;
        }
        return Ok(None);
    };
    let growth = ctx.growth.clone();
    if let super::OperatorOwner::Builtin(kind) = &offer.operator {
        let before = ctx.shape_state.work.snapshot();
        let evaluated = match kind {
            super::Builtin::Dictionary => crate::eval::eval_literal_node(pattern, ctx)?,
            super::Builtin::ExpandedDictionary(spec) => {
                super::dictionary::execute(pattern, spec, ctx)?
            }
            super::Builtin::Bucket => super::bucket::execute(pattern, ctx)?,
            super::Builtin::Window(spec) => super::window::execute(pattern, spec, ctx)?,
        };
        let after = ctx.shape_state.work.snapshot();
        let work = ShapeWork {
            source_visits: after
                .source_visits
                .checked_sub(before.source_visits)
                .ok_or(EvalError::WorkspaceBoundOverflow)?,
            transitions: after
                .transitions
                .checked_sub(before.transitions)
                .ok_or(EvalError::WorkspaceBoundOverflow)?,
            output_rows: match &evaluated {
                Evaluated::Complete(seq) => seq.rows.len() as u64,
                Evaluated::Truncated(_) => 0,
            },
        };
        let node = ctx
            .plan
            .as_ref()
            .and_then(|plan| plan.node_of(pattern))
            .map(crate::plan::NodeId::index);
        ctx.shape_state.report(
            ShapeReport {
                node,
                operator: Some(offer.name),
                provider: offer.provider,
                registry: Some(ctx.shapes.id()),
                work,
                estimate: offer.estimate,
            },
            &growth,
        )?;
        return Ok(Some(evaluated));
    }
    let schema = crate::eval::syntactic_schema_admitted(pattern, &growth)?;
    let mut runtime = NativeRuntime {
        root: pattern,
        ctx,
        inputs: AdmittedVec::new(&growth),
        schema,
        output: RowsBuilder::new(&growth),
        withheld: None,
        work: ShapeWork::default(),
    };
    crate::contain::call_contained_admitted(
        "physical shape",
        offer.name,
        "execute",
        || offer.execute(&mut runtime),
        &growth,
    )?;
    let NativeRuntime {
        ctx,
        schema,
        output,
        withheld,
        work,
        inputs,
        ..
    } = runtime;
    let node = ctx
        .plan
        .as_ref()
        .and_then(|plan| plan.node_of(pattern))
        .map(crate::plan::NodeId::index);
    // Re-home the report array before its first growth, because the context's
    // allocation-free default may have been constructed before admission.
    ctx.shape_state.report(
        ShapeReport {
            node,
            operator: Some(offer.name),
            provider: offer.provider,
            registry: Some(ctx.shapes.id()),
            work,
            estimate: offer.estimate,
        },
        &growth,
    )?;
    if let Some(withheld) = withheld {
        return Ok(Some(withheld));
    }
    let mut rows = output.finish()?;
    let mut search = crate::evidence_exec::RowEvidence::default();
    for input in &inputs {
        search = search.combine(
            input.rows.search_evidence(),
            purrdf_core::evidence::EvidenceCombination::Conjunction,
        )?;
    }
    rows.retain_search_evidence(search);
    Ok(Some(Evaluated::Complete(SolutionSeq { schema, rows })))
}

struct NativeCosts<'a, 'd, D: DatasetView + Sync>(&'a EvalCtx<'d, D>);
impl<D: DatasetView + Sync> super::ShapeCosts for NativeCosts<'_, '_, D> {
    fn visit(
        &self,
        pattern: &purrdf_sparql_algebra::TriplePattern,
        visit: &mut dyn FnMut(purrdf_core::AccessCost),
    ) -> Result<(), EvalError> {
        crate::bgp::visit_shape_cost(pattern, self.0, visit)
    }
}

struct NativeRuntime<'p, 'c, 'd, D: DatasetView + Sync> {
    root: &'p GraphPattern,
    ctx: &'c mut EvalCtx<'d, D>,
    inputs: AdmittedVec<SolutionSeq<D::Id>>,
    schema: crate::solution::SharedSchema,
    output: RowsBuilder<D::Id>,
    withheld: Option<Evaluated<D::Id>>,
    work: ShapeWork,
}
impl<D: DatasetView + Sync> NativeRuntime<'_, '_, '_, D> {
    fn row(&self, row: ShapeRow) -> Result<InputRow<'_, D::Id>, EvalError> {
        let input = self.inputs.get(row.input).ok_or_else(|| {
            violation(
                &self.ctx.growth,
                "shape referenced an input outside this invocation",
            )
        })?;
        let value = input.rows.get(row.ordinal).ok_or_else(|| {
            violation(&self.ctx.growth, "shape referenced a row outside its input")
        })?;
        Ok((input, value))
    }
    fn publish(
        &mut self,
        source: &[ShapeRow],
        mut bindings: AdmittedVec<ShapeBinding>,
    ) -> Result<bool, EvalError> {
        if self.withheld.is_some() {
            return Err(violation(
                &self.ctx.growth,
                "shape published after its input was withheld",
            ));
        }
        let mut cells = AdmittedVec::with_capacity(self.schema.len(), &self.ctx.growth)?;
        for _ in self.schema.vars() {
            cells.push(None)?;
        }
        let mut evidence: Option<crate::RowEvidence> = None;
        for &row in source {
            let (input, row) = self.row(row)?;
            evidence = Some(match evidence {
                None => row.evidence().clone(),
                Some(previous) => previous.combine(
                    row.evidence(),
                    purrdf_core::evidence::EvidenceCombination::Conjunction,
                )?,
            });
            for (variable, &cell) in input.schema.vars().iter().zip(row.iter()) {
                let Some(column) = self.schema.index_of(variable) else {
                    continue;
                };
                if let Some(cell) = cell {
                    if cells[column].is_some_and(|existing| existing != cell) {
                        return Ok(false);
                    }
                    cells.as_mut_slice()[column] = Some(cell);
                }
            }
        }
        for binding in bindings.drain_from(0) {
            let column = self.schema.index_of(&binding.variable).ok_or_else(|| {
                violation(
                    &self.ctx.growth,
                    "shape emitted a binding outside its original schema",
                )
            })?;
            let value = self.ctx.intern_workspace_term(binding.value)?;
            if cells[column].is_some_and(|existing| Some(existing) != value) {
                return Ok(false);
            }
            cells.as_mut_slice()[column] = value;
        }
        let row = RetainedRow::from_cells(cells.len(), cells.iter().copied(), &self.ctx.growth)?
            .with_evidence(evidence.unwrap_or_default(), &self.ctx.growth)?;
        self.output.push_row(row)?;
        self.work.output_rows = self
            .work
            .output_rows
            .checked_add(1)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        Ok(true)
    }
}
impl<D: DatasetView + Sync> ShapeRuntime for NativeRuntime<'_, '_, '_, D> {
    fn workspace(&self) -> &WorkspaceCapability {
        &self.ctx.growth
    }
    fn input(&mut self, path: &[usize]) -> Result<Option<ShapeInput>, EvalError> {
        if self.withheld.is_some() {
            return Ok(None);
        }
        let mut node = self.root;
        for &ordinal in path {
            let mut next = None;
            let mut at = 0;
            crate::governor::soundness::visit_classified_children_admitted(
                node,
                &self.ctx.growth,
                &mut |child, _| {
                    if at == ordinal {
                        next = Some(child);
                    }
                    at += 1;
                    Ok(false)
                },
            )?;
            node = next.ok_or_else(|| {
                violation(
                    &self.ctx.growth,
                    "shape input path is not an original logical edge",
                )
            })?;
        }
        let evaluated = if path.is_empty() {
            crate::eval::eval_literal_node(node, self.ctx)?
        } else {
            crate::eval::eval_evaluated(node, self.ctx)?
        };
        match evaluated {
            Evaluated::Complete(input) => {
                let result = ShapeInput {
                    slot: self.inputs.len(),
                    length: input.rows.len(),
                };
                self.inputs.push(input)?;
                Ok(Some(result))
            }
            Evaluated::Truncated(truncation) => {
                // An opaque host strategy has no license to invent a prefix
                // certificate for computations it did not finish.
                self.withheld = Some(Evaluated::Truncated(Truncation::barred_at(
                    self.root,
                    truncation.tripped(),
                    self.schema.clone(),
                )));
                Ok(None)
            }
        }
    }
    fn schema(&self, input: ShapeInput) -> Result<&VarSchema, EvalError> {
        self.inputs
            .get(input.slot)
            .map(|input| &*input.schema)
            .ok_or_else(|| violation(&self.ctx.growth, "unknown physical input"))
    }
    fn binding(
        &self,
        row: ShapeRow,
        variable: &Variable,
    ) -> Result<Option<WorkspaceTerm>, EvalError> {
        let (input, row) = self.row(row)?;
        input
            .schema
            .index_of(variable)
            .and_then(|column| row[column])
            .map(|cell| crate::expr::owned_value_of(self.ctx, cell))
            .transpose()
    }
    fn emit(&mut self, rows: &[ShapeRow]) -> Result<bool, EvalError> {
        self.publish(rows, AdmittedVec::new(&self.ctx.growth))
    }
    fn emit_with(
        &mut self,
        rows: &[ShapeRow],
        bindings: AdmittedVec<ShapeBinding>,
    ) -> Result<bool, EvalError> {
        self.publish(rows, bindings)
    }
    fn record_work(&mut self, work: ShapeWork) -> Result<bool, EvalError> {
        self.work.source_visits = self
            .work
            .source_visits
            .checked_add(work.source_visits)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        self.work.transitions = self
            .work
            .transitions
            .checked_add(work.transitions)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        self.work.output_rows = self
            .work
            .output_rows
            .checked_add(work.output_rows)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        if let Err(tripped) = self.ctx.charge_occurrences(
            crate::governor::ChargePoint::PropertyFunctionWork,
            work.source_visits
                .checked_add(work.transitions)
                .ok_or(EvalError::WorkspaceBoundOverflow)?,
        ) {
            self.withheld = Some(Evaluated::Truncated(Truncation::barred_at(
                self.root,
                tripped,
                self.schema.clone(),
            )));
            return Ok(false);
        }
        Ok(true)
    }
}
