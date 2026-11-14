// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One driving expansion's source order, admission and terminal work record.
//! Borrowed drains and movable cursors differ only in source ownership.

use super::{
    CompiledPattern, bind_row, components_of, for_each_slot_admitted, query_id, virtual_probe,
};
use crate::dataset_spec::GraphScope;
use crate::eval::EvalCtx;
use crate::solution::RetainedRow;
use crate::statement_layer::StatementProbe;
use crate::{AdmittedMap, EvalError, SolutionTerm};
use purrdf_core::{DatasetView, GraphMatch, QuadIds, ViewTermId};

pub(super) struct ExpansionInput<'a, I: ViewTermId, P> {
    pub(super) pattern: &'a CompiledPattern<I>,
    pub(super) row: &'a RetainedRow<I>,
    pub(super) scope: &'a GraphScope<I>,
    pub(super) plan: Option<P>,
    pub(super) reifies: Option<I>,
    pub(super) unit_driven: bool,
    pub(super) windowed: bool,
}

#[derive(Clone, Copy)]
pub(super) enum ExpansionSource<I: ViewTermId, P> {
    Window,
    Ordinary {
        plan: Option<P>,
        bounds: [Option<I>; 3],
        graph: GraphMatch<I>,
    },
    Virtual(StatementProbe<I>),
}

pub(super) enum ExpansionStep<I: ViewTermId> {
    Row(RetainedRow<I>),
    Skip,
    Stop,
}

#[cfg(test)]
thread_local! {
    /// Virtual statement-layer sources opened on this thread, for tests that pin when
    /// the phase is skipped.
    pub(crate) static VIRTUAL_SOURCES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub(super) struct Expansion<I: ViewTermId> {
    bounds: [Option<I>; 3],
    graph: usize,
    phase: u8,
    merged: bool,
    stopped: bool,
    seen: AdmittedMap<(I, I, I), ()>,
    charge: crate::governor::ItemCharge,
}

impl<I: ViewTermId> Expansion<I> {
    pub(super) fn new<D: DatasetView<Id = I> + Sync>(
        input: &ExpansionInput<'_, I, D::ProbePlan>,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<Self, EvalError> {
        let mut absent = false;
        if !input.unit_driven {
            for pos in [&input.pattern.s, &input.pattern.p, &input.pattern.o] {
                for_each_slot_admitted(pos, &ctx.growth, &mut |column| {
                    absent |= matches!(input.row[column], Some(SolutionTerm::Computed(_)));
                    Ok(())
                })?;
            }
        }
        let bounds = if absent {
            [None; 3]
        } else {
            [
                query_id(&input.pattern.s, input.row),
                query_id(&input.pattern.p, input.row),
                query_id(&input.pattern.o, input.row),
            ]
        };
        Ok(Self {
            bounds,
            graph: 0,
            phase: 0,
            merged: !input.windowed && matches!(input.scope, GraphScope::Merge(_)),
            stopped: absent,
            seen: AdmittedMap::default(),
            charge: crate::governor::ItemCharge::default(),
        })
    }

    pub(super) fn source<D: DatasetView<Id = I> + Sync>(
        &mut self,
        input: &ExpansionInput<'_, I, D::ProbePlan>,
        ctx: &EvalCtx<'_, D>,
    ) -> Option<ExpansionSource<I, D::ProbePlan>> {
        if self.stopped || self.phase == 2 {
            return None;
        }
        if input.windowed {
            self.phase = 2;
            return Some(ExpansionSource::Window);
        }
        match input.scope {
            GraphScope::One(graph) if self.phase == 0 => {
                self.phase = 1;
                let [s, p, o] = self.bounds;
                Some(ExpansionSource::Ordinary {
                    plan: Some(input.plan.unwrap_or_else(|| {
                        ctx.dataset
                            .probe_plan(s.is_some(), p.is_some(), o.is_some(), *graph)
                    })),
                    bounds: self.bounds,
                    graph: *graph,
                })
            }
            // A workspace that admits nothing over a view that certifies an empty
            // statement layer: the virtual phase yields no candidate, charges no
            // candidate, and admits nothing, so it is not opened. A bounded workspace
            // still opens it, because opening a source is admitted there.
            GraphScope::One(_)
                if !ctx.growth.is_bounded() && ctx.dataset.statement_layer_is_empty() =>
            {
                self.phase = 2;
                None
            }
            GraphScope::One(graph) => {
                self.phase = 2;
                #[cfg(test)]
                VIRTUAL_SOURCES.with(|opened| opened.set(opened.get() + 1));
                Some(ExpansionSource::Virtual(virtual_probe(
                    ctx.dataset,
                    input.pattern,
                    self.bounds,
                    components_of(&input.pattern.o, |part| query_id(part, input.row)),
                    input.reifies,
                    *graph,
                )))
            }
            GraphScope::Merge(graphs) => {
                let graph = graphs.get(self.graph).copied();
                self.graph += usize::from(graph.is_some());
                if graph.is_none() {
                    self.phase = 2;
                }
                graph.map(|graph| ExpansionSource::Ordinary {
                    plan: None,
                    bounds: self.bounds,
                    graph: GraphMatch::Named(graph),
                })
            }
        }
    }

    pub(super) fn candidate<D: DatasetView<Id = I> + Sync>(
        &mut self,
        input: &ExpansionInput<'_, I, D::ProbePlan>,
        quad: QuadIds<I>,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<ExpansionStep<I>, EvalError> {
        if ctx.stop_check().is_some() {
            self.stopped = true;
            return Ok(ExpansionStep::Stop);
        }
        self.charge.fuel = self.charge.fuel.saturating_add(1);
        if self.merged {
            let key = (quad.s, quad.p, quad.o);
            if self.seen.get(&key).is_some() {
                return Ok(ExpansionStep::Skip);
            }
            self.seen.insert_admitted(key, (), &ctx.growth)?;
        }
        Ok(match bind_row(input.row, input.pattern, &quad, ctx)? {
            Some(row) => ExpansionStep::Row(row),
            None => ExpansionStep::Skip,
        })
    }

    pub(super) fn committed(&mut self) {
        self.charge.committed = self.charge.committed.saturating_add(1);
    }

    pub(super) fn emit<D: DatasetView<Id = I> + Sync>(
        &mut self,
        input: &ExpansionInput<'_, I, D::ProbePlan>,
        quad: QuadIds<I>,
        ctx: &EvalCtx<'_, D>,
        sink: &mut crate::parallel::RowSink<'_, RetainedRow<I>>,
    ) -> std::ops::ControlFlow<()> {
        match self.candidate(input, quad, ctx) {
            Ok(ExpansionStep::Row(row)) => {
                if sink.push(row) {
                    self.committed();
                }
            }
            Ok(ExpansionStep::Skip) => {}
            Ok(ExpansionStep::Stop) => return std::ops::ControlFlow::Break(()),
            Err(error) => {
                sink.fail(error);
                return std::ops::ControlFlow::Break(());
            }
        }
        if sink.is_full() {
            std::ops::ControlFlow::Break(())
        } else {
            std::ops::ControlFlow::Continue(())
        }
    }

    pub(super) fn charge(&self) -> crate::governor::ItemCharge {
        self.charge
    }
}
