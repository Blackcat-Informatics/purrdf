// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Merge two certified subject-ordered scans without retaining equal-key runs.

use purrdf_core::{
    DatasetView, GraphMatch, QuadIds, StatisticsStream, SubjectOrderedScan, ViewTermId,
};

use super::{CompiledBgp, CompiledPattern, Pos, bind_row};
use crate::dataset_spec::GraphScope;
use crate::eval::EvalCtx;
use crate::solution::{RetainedRow, SolutionSeq};
use crate::workspace::AdmittedVec;
use crate::{EvalError, WorkspaceBox};

struct Arm<I> {
    predicate: I,
    object: Option<I>,
    graph: GraphMatch<I>,
}

impl<I: ViewTermId> Arm<I> {
    fn from_pattern(pattern: &CompiledPattern<I>, graph: GraphMatch<I>) -> Option<Self> {
        let Pos::Bound(predicate) = pattern.p else {
            return None;
        };
        let object = match pattern.o {
            Pos::Bound(id) => Some(id),
            Pos::Slot(_) => None,
            Pos::Triple(_) => return None,
        };
        Some(Self {
            predicate,
            object,
            graph,
        })
    }

    fn rows<'a, D: DatasetView<Id = I>>(
        &self,
        dataset: &'a D,
        first: Option<I>,
        last: Option<I>,
    ) -> Option<impl SubjectOrderedScan<I> + 'a + use<'a, I, D>> {
        let open = |stream| {
            dataset.ordered_subject_quads(
                stream,
                first,
                last,
                Some(self.predicate),
                self.object,
                self.graph,
            )
        };
        // The store applies exact predicate/object/graph constraints to every
        // stream. No stream can be appended after another without losing key
        // order, even when the query only names one predicate.
        Some(MergeStreams {
            scans: [
                open(StatisticsStream::Asserted)?,
                open(StatisticsStream::Reifiers)?,
                open(StatisticsStream::Annotations)?,
            ],
            heads: [StreamHead::Unread; 3],
        })
    }
}

#[derive(Clone, Copy)]
enum StreamHead<I> {
    Unread,
    Row(QuadIds<I>),
    Exhausted,
}

struct MergeStreams<I, S> {
    scans: [S; 3],
    heads: [StreamHead<I>; 3],
}

impl<I: ViewTermId, S: SubjectOrderedScan<I>> Iterator for MergeStreams<I, S> {
    type Item = QuadIds<I>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut winner = None::<(I, usize)>;
        for (index, (scan, head)) in self.scans.iter_mut().zip(&mut self.heads).enumerate() {
            if matches!(head, StreamHead::Unread) {
                *head = scan.next().map_or(StreamHead::Exhausted, StreamHead::Row);
            }
            if let StreamHead::Row(row) = head {
                let key = (row.s, index);
                winner = Some(winner.map_or(key, |best| best.min(key)));
            }
        }
        let StreamHead::Row(row) =
            core::mem::replace(&mut self.heads[winner?.1], StreamHead::Unread)
        else {
            unreachable!("the winning stream has a lookahead row");
        };
        Some(row)
    }
}

impl<I: ViewTermId, S: SubjectOrderedScan<I>> SubjectOrderedScan<I> for MergeStreams<I, S> {
    fn advance_to(&mut self, minimum: I) {
        for (scan, head) in self.scans.iter_mut().zip(&mut self.heads) {
            match head {
                StreamHead::Exhausted => continue,
                StreamHead::Row(row) if row.s >= minimum => continue,
                StreamHead::Row(_) => *head = StreamHead::Unread,
                StreamHead::Unread => {}
            }
            scan.advance_to(minimum);
        }
    }
}

pub(super) fn start<'a, D: DatasetView + Sync>(
    program: &CompiledBgp<D::Id>,
    ctx: &EvalCtx<'a, D>,
) -> Result<Option<Cursor<'a, D::Id>>, EvalError> {
    // Finite production/order evidence still belongs to the existing plan.
    // Selecting this operator does not alter its declared charge sequence.
    if ctx.stop_signal().is_some()
        || ctx.governor_state().is_some()
        || ctx.may_have_evidence()
        || ctx.row_ceiling().is_some()
        || ctx.annotation_range.is_some()
        || ctx.options.force_structural_bgp_order
    {
        return Ok(None);
    }
    let Some([left_pattern, right_pattern]) = program.compiled.as_deref() else {
        return Ok(None);
    };
    let (Pos::Slot(left_key), Pos::Slot(right_key)) = (&left_pattern.s, &right_pattern.s) else {
        return Ok(None);
    };
    if left_key != right_key {
        return Ok(None);
    }
    // A second shared slot needs a composite order. Replaying a subject
    // run and then filtering on object could otherwise turn a selective
    // indexed lookup into quadratic candidate work.
    if matches!(
        (&left_pattern.o, &right_pattern.o),
        (Pos::Slot(left), Pos::Slot(right)) if left == right
    ) || [&left_pattern.o, &right_pattern.o]
        .into_iter()
        .any(|object| matches!(object, Pos::Slot(slot) if slot == left_key))
    {
        return Ok(None);
    }
    let GraphScope::One(graph) = ctx.active_dataset.scope_for(ctx.active_graph) else {
        return Ok(None);
    };
    let (Some(left_arm), Some(right_arm)) = (
        Arm::from_pattern(left_pattern, graph),
        Arm::from_pattern(right_pattern, graph),
    ) else {
        return Ok(None);
    };
    let Some(left_rows) = left_arm.open(ctx, None, None)? else {
        return Ok(None);
    };
    let width = program.working.len();
    let unit = RetainedRow::from_cells(width, core::iter::repeat_n(None, width), &ctx.growth)?;
    Ok(Some(Cursor {
        left: left_rows,
        right: None,
        left_arm,
        right_arm,
        unit,
        group: None,
        ended: false,
    }))
}

impl<I: ViewTermId> Arm<I> {
    fn open<'a, D: DatasetView<Id = I> + Sync>(
        &self,
        ctx: &EvalCtx<'a, D>,
        first: Option<I>,
        last: Option<I>,
    ) -> Result<Option<Scan<'a, I>>, EvalError> {
        let Some(rows) = self.rows(ctx.dataset, first, last) else {
            return Ok(None);
        };
        let rows = WorkspaceBox::new_admitted(rows, &ctx.growth)?
            .coerce(|rows| -> Box<dyn SubjectOrderedScan<I> + 'a> { rows });
        Ok(Some(Scan {
            rows,
            head: None,
            ended: false,
        }))
    }
}

struct Scan<'a, I: ViewTermId> {
    rows: WorkspaceBox<dyn SubjectOrderedScan<I> + 'a>,
    head: Option<QuadIds<I>>,
    ended: bool,
}

impl<I: ViewTermId> Scan<'_, I> {
    fn peek<D: DatasetView<Id = I> + Sync>(
        &mut self,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<Option<QuadIds<I>>, EvalError> {
        if self.head.is_none() && !self.ended {
            self.head = ctx
                .dataset
                .checked_read(|_| self.rows.next())
                .map_err(|error| ctx.workspace.source_error(error))?;
            self.ended = self.head.is_none();
        }
        Ok(self.head)
    }

    fn pop(&mut self) {
        self.head = None;
    }

    fn advance_to<D: DatasetView<Id = I> + Sync>(
        &mut self,
        minimum: I,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<(), EvalError> {
        self.head = ctx
            .dataset
            .checked_read(|_| {
                self.rows.advance_to(minimum);
                self.rows.next()
            })
            .map_err(|error| ctx.workspace.source_error(error))?;
        self.ended = self.head.is_none();
        Ok(())
    }
}

struct Group<'a, I: ViewTermId> {
    key: I,
    first: Option<RetainedRow<I>>,
    run: ProbeRun<I>,
    replay: Option<(Scan<'a, I>, RetainedRow<I>)>,
}

enum ProbeRun<I> {
    Unread,
    One(QuadIds<I>),
    Many,
}

pub(super) struct Cursor<'a, I: ViewTermId> {
    left: Scan<'a, I>,
    right: Option<Scan<'a, I>>,
    left_arm: Arm<I>,
    right_arm: Arm<I>,
    unit: RetainedRow<I>,
    group: Option<Group<'a, I>>,
    ended: bool,
}

impl<'a, I: ViewTermId> Cursor<'a, I> {
    pub(super) fn next<'d: 'a, D: DatasetView<Id = I> + Sync>(
        &mut self,
        program: &CompiledBgp<I>,
        ctx: &EvalCtx<'d, D>,
    ) -> Result<Option<RetainedRow<I>>, EvalError> {
        if self.ended {
            return Ok(None);
        }
        let Some([left, right]) = program.compiled.as_deref() else {
            return Err(EvalError::internal("ordered join lost its two patterns"));
        };
        if self.right.is_none() {
            let Some(first) = self.left.peek(ctx)? else {
                self.ended = true;
                return Ok(None);
            };
            // The first actual driver key and the certified final candidate
            // bound enclose every possible matching probe subject.
            let upper = ctx
                .dataset
                .ordered_subject_upper_bound(
                    Some(self.left_arm.predicate),
                    self.left_arm.object,
                    self.left_arm.graph,
                )
                .map_err(|error| ctx.workspace.source_error(error))?;
            if upper.is_some_and(|bound| bound < first.s) {
                return Err(EvalError::internal(
                    "ordered subject upper bound excludes its first row",
                ));
            }
            self.right = Some(self.open_probe(ctx, Some(first.s), upper)?);
        }
        loop {
            let probe = self.right.as_mut().expect("initialized probe scan");
            if let Some(group) = self.group.as_mut() {
                if let Some(binding) = &group.first {
                    if let Some(quad) = probe.peek(ctx)?.filter(|quad| quad.s == group.key) {
                        probe.pop();
                        group.run = match group.run {
                            ProbeRun::Unread => ProbeRun::One(quad),
                            ProbeRun::One(_) | ProbeRun::Many => ProbeRun::Many,
                        };
                        if let Some(row) = bind_row(binding, right, &quad, ctx)? {
                            return Ok(Some(row));
                        }
                        continue;
                    }
                    group.first = None;
                }
                if let Some((scan, binding)) = &mut group.replay {
                    if let Some(quad) = scan.peek(ctx)? {
                        scan.pop();
                        if let Some(row) = bind_row(binding, right, &quad, ctx)? {
                            return Ok(Some(row));
                        }
                        continue;
                    }
                    group.replay = None;
                }
                if let Some(quad) = self.left.peek(ctx)?.filter(|quad| quad.s == group.key) {
                    self.left.pop();
                    if let Some(binding) = bind_row(&self.unit, left, &quad, ctx)? {
                        match group.run {
                            // Repeated drivers against a single probe row
                            // do not need to revisit the storage directory.
                            ProbeRun::One(candidate) => {
                                if let Some(row) = bind_row(&binding, right, &candidate, ctx)? {
                                    return Ok(Some(row));
                                }
                            }
                            ProbeRun::Many => {
                                let scan = self
                                    .right_arm
                                    .open(ctx, Some(group.key), Some(group.key))?
                                    .ok_or_else(|| {
                                        EvalError::internal("ordered subject replay unavailable")
                                    })?;
                                group.replay = Some((scan, binding));
                            }
                            ProbeRun::Unread => {
                                return Err(EvalError::internal("matching probe run was not read"));
                            }
                        }
                    }
                    continue;
                }
                self.group = None;
            }
            let (Some(driver), Some(candidate)) = (self.left.peek(ctx)?, probe.peek(ctx)?) else {
                self.ended = true;
                return Ok(None);
            };
            match driver.s.cmp(&candidate.s) {
                core::cmp::Ordering::Less => self.left.advance_to(candidate.s, ctx)?,
                core::cmp::Ordering::Greater => probe.advance_to(driver.s, ctx)?,
                core::cmp::Ordering::Equal => {
                    self.left.pop();
                    if let Some(binding) = bind_row(&self.unit, left, &driver, ctx)? {
                        self.group = Some(Group {
                            key: driver.s,
                            first: Some(binding),
                            run: ProbeRun::Unread,
                            replay: None,
                        });
                    }
                }
            }
        }
    }

    fn open_probe<'d: 'a, D: DatasetView<Id = I> + Sync>(
        &self,
        ctx: &EvalCtx<'d, D>,
        first: Option<I>,
        last: Option<I>,
    ) -> Result<Scan<'a, I>, EvalError> {
        self.right_arm
            .open(ctx, first, last)?
            .ok_or_else(|| EvalError::internal("ordered subject capability changed within a read"))
    }
}

pub(super) fn evaluate<D: DatasetView + Sync>(
    program: &CompiledBgp<D::Id>,
    ctx: &EvalCtx<'_, D>,
) -> Result<Option<SolutionSeq<D::Id>>, EvalError> {
    let Some(mut cursor) = start(program, ctx)? else {
        return Ok(None);
    };
    let mut output = AdmittedVec::new(&ctx.growth);
    while let Some(row) = cursor.next(program, ctx)? {
        output.push(row)?;
    }
    program.project(output, &ctx.growth).map(Some)
}
