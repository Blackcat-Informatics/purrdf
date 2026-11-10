// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Retained positive execution with the original selected drivers and source slots.

use super::{
    EvalCtx, NodeBoundary, OrdinaryPosition, PositiveEntry, PositiveInput, PositiveSlot,
    PositiveSlots, ResidentAbort, ResidentPending, pop_inline_frame, positive_input,
};
use crate::bgp::{BgpCursor, BgpCursorPoll, BgpSourceFactory, BgpStart, PositivePlan};
use crate::binop::{CartesianProduct, HashJoinCursor};
use crate::governor::lift::{DetachedLift, Evaluated, Lift, Truncation};
use crate::solution::{RowBagIntoIter, RowsBuilder, SolutionSeq};
use crate::{AdmittedVec, EvalError, WorkspaceCapability};
use purrdf_core::{DatasetView, ViewTermId};
use purrdf_sparql_algebra::GraphPattern;

pub(super) enum PositivePoll<I: ViewTermId> {
    Rows(SolutionSeq<I>),
    Complete {
        evaluated: Evaluated<I>,
        produced: usize,
    },
}

enum After<'a, I: ViewTermId> {
    First {
        driven: OrdinaryPosition<'a>,
        first: usize,
        independent: bool,
        lift: DetachedLift,
    },
    Second {
        first: usize,
        driver: Option<SolutionSeq<I>>,
        lift: DetachedLift,
    },
    Union {
        ordinal: usize,
        input: Option<PositiveSlot>,
        blocks: AdmittedVec<SolutionSeq<I>>,
        lift: DetachedLift,
    },
}

struct Frame<'a, I: ViewTermId> {
    position: OrdinaryPosition<'a>,
    boundary: Option<NodeBoundary<I>>,
    after: After<'a, I>,
    streamed: bool,
    produced: usize,
}

#[expect(
    clippy::large_enum_variant,
    reason = "the producer lives inside the admitted execution owner without another allocation"
)]
enum Producer<'a, I: ViewTermId, P> {
    Bgp(BgpCursor<'a, I, P>),
    Values(crate::modifier::ValuesRows),
    Path(crate::path::PathRows<'a, I>),
    Join(HashJoinCursor<I>),
    Resident {
        rows: Option<RowBagIntoIter<I>>,
        header: SolutionSeq<I>,
        produced: usize,
    },
}

impl<'a, I: ViewTermId, P: Copy + Send + Sync> Producer<'a, I, P> {
    fn resident(rows: SolutionSeq<I>) -> Self {
        let produced = rows.rows.len();
        let mut header = SolutionSeq::empty(rows.schema);
        header
            .rows
            .retain_search_evidence(rows.rows.search_evidence().clone());
        Self::Resident {
            rows: Some(rows.rows.into_iter()),
            header,
            produced,
        }
    }

    fn stop<D: DatasetView<Id = I, ProbePlan = P> + Sync>(
        &mut self,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<(), EvalError> {
        match self {
            Self::Bgp(cursor) => cursor.stop(ctx),
            Self::Values(cursor) => {
                cursor.stop();
                Ok(())
            }
            Self::Path(cursor) => {
                cursor.stop();
                Ok(())
            }
            Self::Join(cursor) => {
                cursor.stop();
                Ok(())
            }
            Self::Resident { rows, .. } => {
                *rows = None;
                Ok(())
            }
        }
    }

    fn poll<'d: 'a, D: DatasetView<Id = I, ProbePlan = P> + Sync>(
        &mut self,
        node: &GraphPattern,
        ctx: &mut EvalCtx<'d, D>,
        factory: BgpSourceFactory<D>,
    ) -> Result<BgpCursorPoll<I>, EvalError> {
        let (row, schema, produced) = match self {
            Self::Bgp(cursor) => return cursor.next(ctx, factory),
            Self::Values(cursor) => {
                let GraphPattern::Values { bindings, .. } = node else {
                    return Err(EvalError::internal(
                        "retained VALUES lost its algebra position",
                    ));
                };
                (
                    cursor.next(bindings, ctx)?,
                    cursor.schema.clone(),
                    cursor.produced(),
                )
            }
            Self::Path(cursor) => (cursor.next(ctx)?, cursor.schema.clone(), cursor.produced()),
            Self::Join(cursor) => (
                cursor.next(ctx)?,
                cursor.schema().clone(),
                cursor.produced(),
            ),
            Self::Resident {
                rows,
                header,
                produced,
            } => {
                let Some(row) = rows.as_mut().and_then(Iterator::next) else {
                    return Ok(BgpCursorPoll::Complete {
                        header: header.clone(),
                        produced: *produced,
                    });
                };
                (Some(row), header.schema.clone(), *produced)
            }
        };
        let Some(row) = row else {
            return Ok(BgpCursorPoll::Complete {
                header: SolutionSeq::empty(schema),
                produced,
            });
        };
        let mut rows = RowsBuilder::new(&ctx.growth);
        rows.push_row(row)?;
        Ok(BgpCursorPoll::Row(SolutionSeq {
            schema,
            rows: rows.finish()?,
        }))
    }
}

struct State<'a, 'i, I: ViewTermId, P> {
    first: Option<Frame<'a, I>>,
    pending: AdmittedVec<Frame<'a, I>>,
    active: Option<NodeBoundary<I>>,
    inputs: PositiveSlots<'i, I>,
    input: Option<PositiveSlot>,
    current: OrdinaryPosition<'a>,
    root: bool,
    producer: Option<Producer<'a, I, P>>,
    producer_lift: Option<DetachedLift>,
    stopped: bool,
    produced: usize,
}

impl<'a, 'i, I: ViewTermId, P: Copy + Send + Sync> State<'a, 'i, I, P> {
    fn new(
        position: OrdinaryPosition<'a>,
        input: Option<PositiveInput<'i, I>>,
        workspace: &WorkspaceCapability,
    ) -> Result<Self, EvalError> {
        let mut inputs = PositiveSlots {
            first: PositiveEntry::Vacant(None),
            rest: AdmittedVec::new(workspace),
            free: Some(0),
        };
        let input = input.map(|input| inputs.insert(input)).transpose()?;
        Ok(Self {
            first: None,
            pending: AdmittedVec::new(workspace),
            active: None,
            inputs,
            input,
            current: position,
            root: true,
            producer: None,
            producer_lift: None,
            stopped: false,
            produced: 0,
        })
    }

    fn reserve(&mut self) -> Result<(), EvalError> {
        if self.first.is_some() {
            self.pending.reserve_next()?;
        }
        Ok(())
    }
    fn push(&mut self, frame: Frame<'a, I>) {
        if self.first.is_none() {
            self.first = Some(frame);
        } else {
            self.pending.push_reserved(frame);
        }
    }
    fn pop(&mut self) -> Option<Frame<'a, I>> {
        pop_inline_frame(&mut self.pending, &mut self.first)
    }

    fn can_deliver(&self, delivery: bool) -> bool {
        delivery
            && self.first.iter().chain(self.pending.iter()).all(|frame| {
                matches!(
                    frame.after,
                    After::Second { driver: None, .. } | After::Union { .. }
                )
            })
    }

    fn note_output(&mut self, count: usize) -> Result<(), EvalError> {
        self.produced = self
            .produced
            .checked_add(count)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        for frame in self
            .first
            .iter_mut()
            .chain(self.pending.as_mut_slice().iter_mut())
        {
            frame.streamed = true;
            frame.produced = frame
                .produced
                .checked_add(count)
                .ok_or(EvalError::WorkspaceBoundOverflow)?;
        }
        Ok(())
    }

    fn finish<D: DatasetView<Id = I> + Sync>(
        &mut self,
        pattern: &GraphPattern,
        result: Result<Evaluated<I>, EvalError>,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<Evaluated<I>, EvalError> {
        match self.active.as_mut() {
            Some(boundary) => {
                let done = boundary.finish(pattern, ctx, result);
                self.active = None;
                done
            }
            None => result,
        }
    }

    fn abort<D: DatasetView<Id = I> + Sync>(&mut self, ctx: &mut EvalCtx<'_, D>) {
        self.producer = None;
        self.producer_lift = None;
        if let Some(boundary) = &mut self.active {
            boundary.abort(ctx);
        }
        self.active = None;
        while let Some(mut frame) = self.pop() {
            if let Some(boundary) = &mut frame.boundary {
                boundary.abort(ctx);
            }
        }
    }

    fn poll<'d: 'a, D: DatasetView<Id = I, ProbePlan = P> + Sync>(
        &mut self,
        plan: Option<&PositivePlan>,
        fork: bool,
        delivery: bool,
        ctx: &mut EvalCtx<'d, D>,
    ) -> Result<PositivePoll<I>, EvalError> {
        let mut pending = ResidentPending {
            child: Some(self),
            ctx,
        };
        let result = pending
            .child
            .as_deref_mut()
            .expect("one positive continuation")
            .run(plan, fork, delivery, pending.ctx);
        if result.is_ok() {
            let _ = pending.child.take();
        }
        result
    }

    fn run<'d: 'a, D: DatasetView<Id = I, ProbePlan = P> + Sync>(
        &mut self,
        plan: Option<&PositivePlan>,
        fork: bool,
        delivery: bool,
        ctx: &mut EvalCtx<'d, D>,
    ) -> Result<PositivePoll<I>, EvalError> {
        let workspace = ctx.growth.clone();
        'descend: loop {
            let position = self.current.clone();
            let node = position.pattern();
            let result = if let Some(producer) = &mut self.producer {
                let factory = ctx.bgp_cursor_sources.ok_or_else(|| {
                    EvalError::internal("positive read lost its source capability")
                })?;
                match producer.poll(node, ctx, factory)? {
                    BgpCursorPoll::Row(rows) => {
                        self.note_output(rows.rows.len())?;
                        return Ok(PositivePoll::Rows(rows));
                    }
                    BgpCursorPoll::Complete { header, produced } => {
                        self.producer = None;
                        if let Some(boundary) = &mut self.active {
                            boundary.streamed_output = Some(produced);
                        }
                        Ok(match self.producer_lift.take() {
                            Some(lift) => lift.attach(node).finish(header),
                            None => Evaluated::Complete(header),
                        })
                    }
                }
            } else {
                if !self.root {
                    self.active = Some(NodeBoundary::enter(node, ctx)?);
                }
                self.root = false;
                let early = self
                    .active
                    .as_mut()
                    .map(|boundary| boundary.start(node, ctx))
                    .transpose()?
                    .flatten();
                if let Some(early) = early {
                    self.inputs.release(self.input.take());
                    Ok(early)
                } else {
                    match node {
                        GraphPattern::Bgp { patterns } => {
                            let order = self
                                .input
                                .is_none()
                                .then(|| plan.and_then(|plan| plan.unit_order(node)))
                                .flatten();
                            let readable = self.can_deliver(delivery);
                            let input = self.input.take().map(|slot| self.inputs.value(slot));
                            if readable {
                                match crate::bgp::start_bgp_cursor(patterns, input, order, ctx)? {
                                    BgpStart::Cursor(cursor) => {
                                        self.producer = Some(Producer::Bgp(cursor));
                                        continue 'descend;
                                    }
                                    BgpStart::Complete(rows) if !rows.rows.is_empty() => {
                                        self.producer = Some(Producer::resident(rows));
                                        continue 'descend;
                                    }
                                    BgpStart::Complete(rows) => Ok(Evaluated::Complete(rows)),
                                }
                            } else {
                                crate::bgp::eval_bgp_seeded(patterns, input, order, ctx)
                                    .map(Evaluated::Complete)
                            }
                        }
                        GraphPattern::Join { left, right } => {
                            let plan = plan.ok_or_else(|| {
                                EvalError::internal("a seeded leaf entered a JOIN")
                            })?;
                            self.reserve()?;
                            let first = usize::from(!plan.driver_left(node));
                            let (driver, driven) = if first == 0 {
                                (&**left, &**right)
                            } else {
                                (&**right, &**left)
                            };
                            let next = position.child(driver)?;
                            let after = After::First {
                                driven: position.child(driven)?,
                                first,
                                independent: self.input.is_none() && plan.children_disjoint(node),
                                lift: Lift::at_admitted(node, &workspace)?.detach(),
                            };
                            let boundary = self.active.take();
                            self.push(Frame {
                                position: position.clone(),
                                boundary,
                                after,
                                streamed: false,
                                produced: 0,
                            });
                            self.current = next;
                            continue 'descend;
                        }
                        GraphPattern::Union { arms } => {
                            let plan = plan.ok_or_else(|| {
                                EvalError::internal("a seeded leaf entered a UNION")
                            })?;
                            if fork
                                && !self.can_deliver(delivery)
                                && crate::binop::positive_union_may_fork(arms, ctx)
                            {
                                let input = self.input.take().map(|slot| self.inputs.value(slot));
                                crate::binop::eval_positive_union_parallel(
                                    node, arms, input, plan, ctx,
                                )
                            } else if !arms.is_empty() {
                                let arm = &arms[0];
                                self.reserve()?;
                                let next = position.child(arm)?;
                                let producer = self.input.take();
                                let child = producer.map(PositiveSlot::borrowed);
                                let boundary = self.active.take();
                                self.push(Frame {
                                    position: position.clone(),
                                    boundary,
                                    after: After::Union {
                                        ordinal: 0,
                                        input: producer,
                                        blocks: AdmittedVec::new(&workspace),
                                        lift: Lift::at_admitted(node, &workspace)?.detach(),
                                    },
                                    streamed: false,
                                    produced: 0,
                                });
                                self.input = child;
                                self.current = next;
                                continue 'descend;
                            } else {
                                self.inputs.release(self.input.take());
                                Ok(Lift::at_admitted(node, &workspace)?
                                    .finish(crate::binop::concat_union(std::iter::empty(), ctx)?))
                            }
                        }
                        _ => {
                            return Err(EvalError::internal(
                                "positive plan contains an unsupported node",
                            ));
                        }
                    }
                }
            };
            let mut completed = self.finish(node, result, ctx);
            loop {
                let Some(frame) = self.pop() else {
                    return completed.map(|evaluated| PositivePoll::Complete {
                        evaluated,
                        produced: self.produced,
                    });
                };
                let position = frame.position;
                self.current = position.clone();
                let node = position.pattern();
                self.active = frame.boundary;
                if frame.streamed
                    && let Some(boundary) = &mut self.active
                {
                    boundary.streamed_output = Some(frame.produced);
                }
                let result = match (frame.after, completed) {
                    (_, Err(error)) => Err(error),
                    (
                        After::First {
                            driven,
                            first,
                            independent,
                            lift,
                        },
                        Ok(child),
                    ) => {
                        let mut lift = lift.attach(node);
                        match lift.absorb(first, child) {
                            None => Ok(lift.withheld()),
                            Some(rows) if lift.is_truncated() => {
                                Ok(lift.finish(SolutionSeq::empty(rows.schema)))
                            }
                            Some(rows) => {
                                ctx.note_cursor_buffer(super::CursorBufferReason::DriverRelation);
                                let plan = plan.ok_or_else(|| {
                                    EvalError::internal("a seeded leaf resumed a JOIN")
                                })?;
                                self.reserve()?;
                                let driver = if independent {
                                    self.input = None;
                                    Some(rows)
                                } else {
                                    let GraphPattern::Join { left, right } = node else {
                                        return Err(EvalError::internal(
                                            "positive driver lost its join",
                                        ));
                                    };
                                    let producer = if first == 0 { &**left } else { &**right };
                                    self.input = Some(
                                        self.inputs
                                            .insert(positive_input(plan, producer, rows)?)?,
                                    );
                                    None
                                };
                                let boundary = self.active.take();
                                self.push(Frame {
                                    position: position.clone(),
                                    boundary,
                                    after: After::Second {
                                        first,
                                        driver,
                                        lift: lift.detach(),
                                    },
                                    streamed: false,
                                    produced: 0,
                                });
                                self.current = driven;
                                continue 'descend;
                            }
                        }
                    }
                    (
                        After::Second {
                            first,
                            driver,
                            lift,
                        },
                        Ok(child),
                    ) => {
                        let mut lift = lift.attach(node);
                        match lift.absorb(1 - first, child) {
                            None => Ok(lift.withheld()),
                            Some(rows) => {
                                let rows = if let Some(driver) = driver {
                                    ctx.note_cursor_buffer(
                                        super::CursorBufferReason::ProbeRelation,
                                    );
                                    if self.can_deliver(delivery) && !lift.is_truncated() {
                                        self.producer =
                                            Some(Producer::Join(HashJoinCursor::Cartesian(
                                                CartesianProduct::new(driver, rows, ctx)?,
                                            )));
                                        self.producer_lift = Some(lift.detach());
                                        continue 'descend;
                                    }
                                    crate::binop::hash_join(&driver, &rows, ctx)?
                                } else {
                                    rows
                                };
                                Ok(lift.finish(rows))
                            }
                        }
                    }
                    (
                        After::Union {
                            ordinal,
                            input: producer,
                            mut blocks,
                            lift,
                        },
                        Ok(child),
                    ) => {
                        let mut lift = lift.attach(node);
                        match lift.absorb(ordinal, child) {
                            None => {
                                self.inputs.release(producer);
                                Ok(lift.withheld())
                            }
                            Some(rows) => {
                                blocks.push(rows)?;
                                let GraphPattern::Union { arms } = node else {
                                    return Err(EvalError::internal("positive arm lost its UNION"));
                                };
                                if self.stopped || lift.is_truncated() || ordinal + 1 == arms.len()
                                {
                                    self.inputs.release(producer);
                                    Ok(lift.finish(crate::binop::concat_union(blocks, ctx)?))
                                } else {
                                    self.reserve()?;
                                    let next = position.child(&arms[ordinal + 1])?;
                                    let boundary = self.active.take();
                                    self.push(Frame {
                                        position: position.clone(),
                                        boundary,
                                        after: After::Union {
                                            ordinal: ordinal + 1,
                                            input: producer,
                                            blocks,
                                            lift: lift.detach(),
                                        },
                                        streamed: frame.streamed,
                                        produced: frame.produced,
                                    });
                                    self.input = producer.map(PositiveSlot::borrowed);
                                    self.current = next;
                                    continue 'descend;
                                }
                            }
                        }
                    }
                };
                completed = self.finish(node, result, ctx);
            }
        }
    }
}

impl<D: DatasetView + Sync> ResidentAbort<D> for &mut State<'_, '_, D::Id, D::ProbePlan> {
    fn abort(&mut self, ctx: &mut EvalCtx<'_, D>) {
        State::abort(self, ctx);
    }
}

pub(super) struct PositiveExecution<'a, I: ViewTermId, P> {
    state: State<'a, 'a, I, P>,
    plan: Option<PositivePlan>,
    output: crate::solution::SharedSchema,
}

impl<'a, I: ViewTermId, P: Copy + Send + Sync> PositiveExecution<'a, I, P> {
    pub(super) fn leaf<'d: 'a, D: DatasetView<Id = I, ProbePlan = P> + Sync>(
        position: OrdinaryPosition<'a>,
        ctx: &EvalCtx<'d, D>,
    ) -> Result<Self, EvalError> {
        let (producer, output) = match position.pattern() {
            GraphPattern::Values { variables, .. } => {
                let source = crate::modifier::ValuesRows::new(variables, ctx)?;
                let output = source.schema.clone();
                (Producer::Values(source), output)
            }
            GraphPattern::Path {
                subject,
                path,
                object,
            } => {
                let source = crate::path::PathRows::new(subject, path, object, ctx)?;
                let output = source.schema.clone();
                (Producer::Path(source), output)
            }
            _ => return Err(EvalError::internal("retained leaf has no row source")),
        };
        let mut state = State::new(position, None, &ctx.growth)?;
        state.producer = Some(producer);
        Ok(Self {
            state,
            plan: None,
            output,
        })
    }

    pub(super) fn new(
        position: OrdinaryPosition<'a>,
        plan: PositivePlan,
        workspace: &WorkspaceCapability,
    ) -> Result<Self, EvalError> {
        if !plan.contains(position.pattern()) {
            return Err(EvalError::internal(
                "a retained positive relation is outside its plan",
            ));
        }
        let output = plan.root_schema().clone().shared_admitted(workspace)?;
        Ok(Self {
            state: State::new(position, None, workspace)?,
            plan: Some(plan),
            output,
        })
    }

    pub(super) fn joined<D: DatasetView<Id = I, ProbePlan = P> + Sync>(
        position: OrdinaryPosition<'a>,
        left: SolutionSeq<I>,
        right: SolutionSeq<I>,
        lift: DetachedLift,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<Self, EvalError> {
        let producer = HashJoinCursor::new(left, right, ctx)?;
        let output = producer.schema().clone();
        let mut state = State::new(position, None, &ctx.growth)?;
        state.producer = Some(Producer::Join(producer));
        state.producer_lift = Some(lift);
        Ok(Self {
            state,
            plan: None,
            output,
        })
    }

    pub(super) fn seeded(
        position: OrdinaryPosition<'a>,
        prepared: crate::binop::SeededJoinPreparation<I>,
        workspace: &WorkspaceCapability,
    ) -> Result<Self, EvalError> {
        if prepared
            .plan
            .as_ref()
            .is_some_and(|plan| !plan.contains(position.pattern()))
        {
            return Err(EvalError::internal(
                "a retained seeded relation is outside its original plan",
            ));
        }
        let mut state = State::new(
            position,
            Some(PositiveInput::owned(prepared.driver)),
            workspace,
        )?;
        // The join owns its outer boundary; this source must enter and finish
        // the original driven child's own checkpoint before completing that join.
        state.root = false;
        Ok(Self {
            state,
            plan: prepared.plan,
            output: prepared.output.shared_admitted(workspace)?,
        })
    }

    pub(super) fn poll<'d: 'a, D: DatasetView<Id = I, ProbePlan = P> + Sync>(
        &mut self,
        stopped: bool,
        ctx: &mut EvalCtx<'d, D>,
    ) -> Result<PositivePoll<I>, EvalError> {
        if stopped {
            self.state.stopped = true;
            self.state
                .producer
                .as_mut()
                .ok_or_else(|| EvalError::internal("positive delivery has no retained producer"))?
                .stop(ctx)?;
        }
        match self.state.poll(self.plan.as_ref(), true, true, ctx)? {
            PositivePoll::Rows(rows) => Ok(PositivePoll::Rows(
                rows.reorder_like_admitted(&self.output, &ctx.growth)?,
            )),
            PositivePoll::Complete {
                evaluated,
                produced,
            } => {
                let evaluated = match evaluated {
                    Evaluated::Complete(rows) => {
                        Evaluated::Complete(rows.reorder_like_admitted(&self.output, &ctx.growth)?)
                    }
                    Evaluated::Truncated(truncated) => {
                        let (rows, proof) = truncated.split();
                        Evaluated::Truncated(Truncation::new(
                            rows.reorder_like_admitted(&self.output, &ctx.growth)?,
                            proof,
                        ))
                    }
                };
                Ok(PositivePoll::Complete {
                    evaluated,
                    produced,
                })
            }
        }
    }

    pub(super) fn abort<D: DatasetView<Id = I> + Sync>(&mut self, ctx: &mut EvalCtx<'_, D>) {
        self.state.abort(ctx);
    }
}

pub(super) fn body<'a, 'i, 'd: 'a, D: DatasetView + Sync>(
    position: OrdinaryPosition<'a>,
    input: Option<PositiveInput<'i, D::Id>>,
    plan: &PositivePlan,
    fork: bool,
    ctx: &mut EvalCtx<'d, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    if !plan.contains(position.pattern()) {
        return Err(EvalError::internal(
            "a seeded relation is outside its positive plan",
        ));
    }
    let mut state = State::new(position, input, &ctx.growth)?;
    match state.poll(Some(plan), fork, false, ctx)? {
        PositivePoll::Complete { evaluated, .. } => Ok(evaluated),
        PositivePoll::Rows(_) => Err(EvalError::internal(
            "a completed positive drain returned an unconsumed row",
        )),
    }
}
