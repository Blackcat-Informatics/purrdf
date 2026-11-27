// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Merge certified subject-ordered scans without retaining equal-key runs.

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
    // Finite execution and evidence retain their existing charge sequence.
    if ctx.stop_signal().is_some()
        || ctx.governor_state().is_some()
        || ctx.may_have_evidence()
        || ctx.row_ceiling().is_some()
        || ctx.annotation_range.is_some()
        || ctx.options.force_structural_bgp_order
    {
        return Ok(None);
    }
    let Some(patterns @ [driver, _, ..]) = program.compiled.as_deref() else {
        return Ok(None);
    };
    let Pos::Slot(subject) = driver.s else {
        return Ok(None);
    };
    let GraphScope::One(graph) = ctx.active_dataset.scope_for(ctx.active_graph) else {
        return Ok(None);
    };
    // A second shared slot requires a composite order, not filtering after
    // constructing a potentially much larger run product. Complete structural
    // selection before opening a source cursor or admitting its storage.
    for (index, pattern) in patterns.iter().enumerate() {
        let Some(arm) = Arm::from_pattern(pattern, graph) else {
            return Ok(None);
        };
        if !matches!(pattern.s, Pos::Slot(key) if key == subject)
            || matches!(pattern.o, Pos::Slot(slot)
                if slot == subject || patterns[..index]
                    .iter()
                    .any(|previous| matches!(previous.o, Pos::Slot(previous_slot) if previous_slot == slot)))
            || !ctx
                .dataset
                .may_scan_in_subject_order(Some(arm.predicate), arm.object, arm.graph)
        {
            return Ok(None);
        }
    }
    let required_arm = |pattern: &CompiledPattern<D::Id>| {
        Arm::from_pattern(pattern, graph)
            .ok_or_else(|| EvalError::internal("ordered join lost an eligible pattern"))
    };
    let driver_arm = required_arm(driver)?;
    let Some(driver_scan) = driver_arm.open(ctx, None, None)? else {
        return Ok(None);
    };
    let probes = match &patterns[1..] {
        [only] => Probes::One(Probe::new(required_arm(only)?)),
        many => {
            let mut probes = AdmittedVec::with_capacity(many.len(), &ctx.growth)?;
            for pattern in many {
                probes.push_reserved(Probe::new(required_arm(pattern)?));
            }
            Probes::Many(probes)
        }
    };

    let width = program.working.len();
    let unit = RetainedRow::from_cells(width, core::iter::repeat_n(None, width), &ctx.growth)?;
    Ok(Some(Cursor {
        driver: driver_scan,
        driver_arm,
        probes,
        unit,
        group: None,
        depth: None,
        initialized: false,
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

enum ProbeRun<I> {
    Unread,
    One(QuadIds<I>),
    Many,
}

enum ProbePass<'a, I: ViewTermId> {
    Primary,
    Singleton(Option<QuadIds<I>>),
    Replay(Scan<'a, I>),
}

struct Probe<'a, I: ViewTermId> {
    arm: Arm<I>,
    primary: Option<Scan<'a, I>>,
    run: ProbeRun<I>,
    pass: ProbePass<'a, I>,
    prefix: Option<RetainedRow<I>>,
}

impl<'a, I: ViewTermId> Probe<'a, I> {
    fn new(arm: Arm<I>) -> Self {
        Self {
            arm,
            primary: None,
            run: ProbeRun::Unread,
            pass: ProbePass::Primary,
            prefix: None,
        }
    }

    fn open<'d: 'a, D: DatasetView<Id = I> + Sync>(
        &self,
        ctx: &EvalCtx<'d, D>,
        first: Option<I>,
        last: Option<I>,
    ) -> Result<Scan<'a, I>, EvalError> {
        self.arm
            .open(ctx, first, last)?
            .ok_or_else(|| EvalError::internal("ordered subject capability changed within a read"))
    }

    fn begin<'d: 'a, D: DatasetView<Id = I> + Sync>(
        &mut self,
        key: I,
        prefix: RetainedRow<I>,
        ctx: &EvalCtx<'d, D>,
    ) -> Result<(), EvalError> {
        self.pass = match self.run {
            ProbeRun::Unread => ProbePass::Primary,
            ProbeRun::One(row) => ProbePass::Singleton(Some(row)),
            ProbeRun::Many => ProbePass::Replay(self.open(ctx, Some(key), Some(key))?),
        };
        self.prefix = Some(prefix);
        Ok(())
    }

    fn next<D: DatasetView<Id = I> + Sync>(
        &mut self,
        pattern: &CompiledPattern<I>,
        key: I,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<Option<RetainedRow<I>>, EvalError> {
        let prefix = self
            .prefix
            .as_ref()
            .ok_or_else(|| EvalError::internal("ordered probe lost its parent binding"))?;
        loop {
            let row = match &mut self.pass {
                ProbePass::Primary => {
                    let scan = self.primary.as_mut().ok_or_else(|| {
                        EvalError::internal("ordered probe lost its primary scan")
                    })?;
                    let row = scan.peek(ctx)?.filter(|row| row.s == key);
                    if let Some(row) = row {
                        scan.pop();
                        self.run = match self.run {
                            ProbeRun::Unread => ProbeRun::One(row),
                            ProbeRun::One(_) | ProbeRun::Many => ProbeRun::Many,
                        };
                    }
                    row
                }
                ProbePass::Singleton(row) => row.take(),
                ProbePass::Replay(scan) => {
                    let row = scan.peek(ctx)?;
                    scan.pop();
                    row
                }
            };
            let Some(row) = row else {
                self.prefix = None;
                self.pass = ProbePass::Primary;
                return Ok(None);
            };
            if let Some(binding) = bind_row(prefix, pattern, &row, ctx)? {
                return Ok(Some(binding));
            }
        }
    }
}

#[expect(
    clippy::large_enum_variant,
    reason = "a binary join retains its existing inline probe without a second admission"
)]
enum Probes<'a, I: ViewTermId> {
    One(Probe<'a, I>),
    Many(AdmittedVec<Probe<'a, I>>),
}

impl<'a, I: ViewTermId> Probes<'a, I> {
    fn len(&self) -> usize {
        match self {
            Self::One(_) => 1,
            Self::Many(probes) => probes.len(),
        }
    }

    fn as_mut_slice(&mut self) -> &mut [Probe<'a, I>] {
        match self {
            Self::One(probe) => core::slice::from_mut(probe),
            Self::Many(probes) => probes.as_mut_slice(),
        }
    }
}

pub(super) struct Cursor<'a, I: ViewTermId> {
    driver: Scan<'a, I>,
    driver_arm: Arm<I>,
    probes: Probes<'a, I>,
    unit: RetainedRow<I>,
    group: Option<I>,
    // The active probe depth, if a driver row has bound successfully.
    depth: Option<usize>,
    initialized: bool,
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
        let patterns = program
            .compiled
            .as_deref()
            .filter(|patterns| patterns.len() == self.probes.len() + 1)
            .ok_or_else(|| EvalError::internal("ordered join lost its patterns"))?;
        if !self.initialized {
            let Some(first) = self.driver.peek(ctx)? else {
                self.ended = true;
                return Ok(None);
            };
            let upper = ctx
                .dataset
                .ordered_subject_upper_bound(
                    Some(self.driver_arm.predicate),
                    self.driver_arm.object,
                    self.driver_arm.graph,
                )
                .map_err(|error| ctx.workspace.source_error(error))?;
            if upper.is_some_and(|bound| bound < first.s) {
                return Err(EvalError::internal(
                    "ordered subject upper bound excludes its first row",
                ));
            }
            for probe in self.probes.as_mut_slice() {
                probe.primary = Some(probe.open(ctx, Some(first.s), upper)?);
            }
            self.initialized = true;
        }

        loop {
            if let Some(depth) = self.depth {
                let key = self
                    .group
                    .ok_or_else(|| EvalError::internal("ordered product lost its subject key"))?;
                let probes = self.probes.as_mut_slice();
                if let Some(prefix) = probes[depth].next(&patterns[depth + 1], key, ctx)? {
                    let next = depth + 1;
                    let Some(probe) = probes.get_mut(next) else {
                        return Ok(Some(prefix));
                    };
                    probe.begin(key, prefix, ctx)?;
                    self.depth = Some(next);
                } else {
                    self.depth = depth.checked_sub(1);
                }
                continue;
            }

            let driver = if let Some(driver) = self
                .driver
                .peek(ctx)?
                .filter(|row| Some(row.s) == self.group)
            {
                driver
            } else {
                let Some(driver) = self.align(ctx)? else {
                    self.ended = true;
                    return Ok(None);
                };
                self.group = Some(driver.s);
                for probe in self.probes.as_mut_slice() {
                    probe.run = ProbeRun::Unread;
                }
                driver
            };
            self.driver.pop();
            if let Some(prefix) = bind_row(&self.unit, &patterns[0], &driver, ctx)? {
                self.probes.as_mut_slice()[0].begin(driver.s, prefix, ctx)?;
                self.depth = Some(0);
            }
        }
    }

    fn align<D: DatasetView<Id = I> + Sync>(
        &mut self,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<Option<QuadIds<I>>, EvalError> {
        'driver: while let Some(driver) = self.driver.peek(ctx)? {
            for probe in self.probes.as_mut_slice() {
                let scan = probe.primary.as_mut().ok_or_else(|| {
                    EvalError::internal("ordered alignment lost its primary scan")
                })?;
                if !scan.ended && scan.head.is_none_or(|row| row.s < driver.s) {
                    scan.advance_to(driver.s, ctx)?;
                }
                let Some(candidate) = scan.peek(ctx)? else {
                    return Ok(None);
                };
                if candidate.s > driver.s {
                    self.driver.advance_to(candidate.s, ctx)?;
                    continue 'driver;
                }
            }
            return Ok(Some(driver));
        }
        Ok(None)
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
