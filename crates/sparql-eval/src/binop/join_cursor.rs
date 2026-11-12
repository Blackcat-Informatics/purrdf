// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared candidate expansion for borrowed drains and retained hash joins.

use super::{
    JoinIndex, JoinKey, KeySide, bound_key, build_index, merge_admitted, reserve_join_rows,
    right_to_out_map,
};
use crate::eval::EvalCtx;
use crate::solution::{
    RetainedRow, RowBag, RowsBuilder, SharedSchema, SolutionSeq, VarSchema, compatible,
};
use crate::workspace::{NativeSharedWorkspace, ProjectedNative};
use crate::{AdmittedVec, EvalError, WorkspaceCapability};
use purrdf_core::{DatasetView, ViewTermId};
use std::ops::Deref;

pub(super) struct JoinLayout {
    pub(super) output: VarSchema,
    pub(super) right_to_out: AdmittedVec<usize>,
    pub(super) shared: AdmittedVec<(usize, usize)>,
}

pub(super) fn layout<I: ViewTermId>(
    left: &SolutionSeq<I>,
    right: &SolutionSeq<I>,
    workspace: &WorkspaceCapability,
) -> Result<JoinLayout, EvalError> {
    let output = left.schema.union_admitted(&right.schema, workspace)?;
    let right_to_out = right_to_out_map(&right.schema, &output, workspace)?;
    let shared = left
        .schema
        .shared_columns_admitted(&right.schema, workspace)?;
    Ok(JoinLayout {
        output,
        right_to_out,
        shared,
    })
}

enum Phase<I: ViewTermId, B> {
    Bound {
        // Preserve the original key's admission until this left row's expansion ends.
        _key: JoinKey<I>,
        bucket: Option<B>,
        indexed: usize,
        wild: usize,
    },
    All {
        next: usize,
    },
    Done,
}

struct Candidates<I: ViewTermId, B> {
    phase: Phase<I, B>,
}

impl<I: ViewTermId, B: Deref<Target = [usize]>> Candidates<I, B> {
    fn new(
        row: &RetainedRow<I>,
        shared: &[(usize, usize)],
        workspace: &WorkspaceCapability,
        lookup: impl FnOnce(&JoinKey<I>) -> Option<B>,
    ) -> Result<Self, EvalError> {
        let phase = match bound_key(row, shared, KeySide::Left, workspace)? {
            Some(key) => {
                let bucket = lookup(&key);
                Phase::Bound {
                    _key: key,
                    bucket,
                    indexed: 0,
                    wild: 0,
                }
            }
            None => Phase::All { next: 0 },
        };
        Ok(Self { phase })
    }

    fn next(
        &mut self,
        row: &RetainedRow<I>,
        right: &[RetainedRow<I>],
        shared: &[(usize, usize)],
        wild_rows: &[usize],
    ) -> Option<usize> {
        match &mut self.phase {
            Phase::Bound {
                bucket,
                indexed,
                wild,
                ..
            } => {
                if let Some(index) = bucket.as_ref().and_then(|bucket| bucket.get(*indexed)) {
                    *indexed += 1;
                    return Some(*index);
                }
                while let Some(&index) = wild_rows.get(*wild) {
                    *wild += 1;
                    if compatible(row, &right[index], shared) {
                        return Some(index);
                    }
                }
            }
            Phase::All { next } => {
                while let Some(candidate) = right.get(*next) {
                    let index = *next;
                    *next += 1;
                    if compatible(row, candidate, shared) {
                        return Some(index);
                    }
                }
            }
            Phase::Done => return None,
        }
        self.phase = Phase::Done;
        None
    }
}

/// The completed path retains its original parallel per-left-row production;
/// both lanes use the same keyed/wild/full-scan candidate state above.
pub(super) fn drain_keyed<D: DatasetView + Sync>(
    left: &SolutionSeq<D::Id>,
    right: &SolutionSeq<D::Id>,
    layout: &JoinLayout,
    ctx: &EvalCtx<'_, D>,
) -> Result<RowBag<D::Id>, EvalError> {
    if layout.shared.is_empty() {
        return Err(EvalError::internal("a keyed join requires shared columns"));
    }
    let (keyed, wild) = build_index(right, &layout.shared, &ctx.growth)?;
    let ceiling = ctx.cell_row_ceiling(layout.output.len());
    if ctx.growth.is_bounded() || ceiling.is_some() {
        let mut rows = RowsBuilder::new(&ctx.growth);
        'left: for row in &left.rows {
            let mut candidates =
                Candidates::new(row, &layout.shared, &ctx.growth, |key| keyed.get(key))?;
            while let Some(index) = candidates.next(row, &right.rows, &layout.shared, &wild) {
                if ceiling.is_some_and(|ceiling| rows.len() >= ceiling) {
                    let _ = ctx.observe_cells(rows.len().saturating_add(1), layout.output.len());
                    break 'left;
                }
                rows.push_row(merge_admitted(
                    row,
                    &right.rows[index],
                    left.schema.len(),
                    &layout.right_to_out,
                    layout.output.len(),
                    &ctx.growth,
                )?)?;
            }
        }
        return rows.finish();
    }
    let (rows, _) = crate::parallel::par_chunk_try_map_init(
        ctx.sequential_operation_required(),
        &left.rows,
        || (),
        |(), output, row| {
            let mut candidates =
                Candidates::new(row, &layout.shared, &ctx.growth, |key| keyed.get(key))?;
            while let Some(index) = candidates.next(row, &right.rows, &layout.shared, &wild) {
                reserve_join_rows(output, 1, usize::MAX)?;
                output.push(merge_admitted(
                    row,
                    &right.rows[index],
                    left.schema.len(),
                    &layout.right_to_out,
                    layout.output.len(),
                    &ctx.growth,
                )?);
            }
            Ok(())
        },
        |()| (),
    )?;
    RowsBuilder::from_storage(
        AdmittedVec::from_parts(rows, None, &ctx.growth),
        &ctx.growth,
    )
    .finish()
}

struct Storage<I: ViewTermId> {
    left: SolutionSeq<I>,
    right: SolutionSeq<I>,
    keyed: JoinIndex<I>,
    wild: AdmittedVec<usize>,
    shared: AdmittedVec<(usize, usize)>,
    right_to_out: AdmittedVec<usize>,
    schema: SharedSchema,
}

struct OwnedBucket<I: ViewTermId>(ProjectedNative<Storage<I>, AdmittedVec<usize>>);
impl<I: ViewTermId> Deref for OwnedBucket<I> {
    type Target = [usize];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub(crate) struct KeyedCursor<I: ViewTermId> {
    storage: ProjectedNative<Storage<I>, Storage<I>>,
    left: usize,
    candidates: Option<Candidates<I, OwnedBucket<I>>>,
    produced: usize,
    ceiling: Option<usize>,
    stopped: bool,
}

impl<I: ViewTermId> KeyedCursor<I> {
    fn new<D: DatasetView<Id = I> + Sync>(
        left: SolutionSeq<I>,
        right: SolutionSeq<I>,
        layout: JoinLayout,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<Self, EvalError> {
        let (keyed, wild) = build_index(&right, &layout.shared, &ctx.growth)?;
        let ceiling = ctx.cell_row_ceiling(layout.output.len());
        let storage = NativeSharedWorkspace::new_admitted(
            Storage {
                left,
                right,
                keyed,
                wild,
                shared: layout.shared,
                right_to_out: layout.right_to_out,
                schema: layout.output.shared_admitted(&ctx.growth)?,
            },
            &ctx.growth,
        )?
        .into_projected(|storage| storage);
        Ok(Self {
            storage,
            left: 0,
            candidates: None,
            produced: 0,
            ceiling,
            stopped: false,
        })
    }

    fn next<D: DatasetView<Id = I> + Sync>(
        &mut self,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<Option<RetainedRow<I>>, EvalError> {
        while !self.stopped {
            let Some(row) = self.storage.left.rows.get(self.left) else {
                return Ok(None);
            };
            if self.candidates.is_none() {
                self.candidates = Some(Candidates::new(
                    row,
                    &self.storage.shared,
                    &ctx.growth,
                    |key| {
                        self.storage
                            .try_project(|storage| storage.keyed.bucket(key))
                            .map(OwnedBucket)
                    },
                )?);
            }
            let Some(index) = self
                .candidates
                .as_mut()
                .expect("one original left-row expansion")
                .next(
                    row,
                    &self.storage.right.rows,
                    &self.storage.shared,
                    &self.storage.wild,
                )
            else {
                self.candidates = None;
                self.left += 1;
                continue;
            };
            if self.ceiling.is_some_and(|ceiling| self.produced >= ceiling) {
                let _ =
                    ctx.observe_cells(self.produced.saturating_add(1), self.storage.schema.len());
                self.stopped = true;
                return Ok(None);
            }
            let output = merge_admitted(
                row,
                &self.storage.right.rows[index],
                self.storage.left.schema.len(),
                &self.storage.right_to_out,
                self.storage.schema.len(),
                &ctx.growth,
            )?;
            self.produced = self
                .produced
                .checked_add(1)
                .ok_or(EvalError::WorkspaceBoundOverflow)?;
            return Ok(Some(output));
        }
        Ok(None)
    }
}

pub(crate) enum HashJoinCursor<I: ViewTermId> {
    Cartesian(super::CartesianProduct<I>),
    Keyed(KeyedCursor<I>),
}

impl<I: ViewTermId> HashJoinCursor<I> {
    pub(crate) fn new<D: DatasetView<Id = I> + Sync>(
        left: SolutionSeq<I>,
        right: SolutionSeq<I>,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<Self, EvalError> {
        ctx.note_cursor_buffer(crate::eval::CursorBufferReason::ProbeRelation);
        let layout = layout(&left, &right, &ctx.growth)?;
        if layout.shared.is_empty() {
            super::CartesianProduct::from_layout(
                left,
                right,
                layout.output,
                layout.right_to_out,
                ctx,
            )
            .map(Self::Cartesian)
        } else {
            KeyedCursor::new(left, right, layout, ctx).map(Self::Keyed)
        }
    }

    pub(crate) fn schema(&self) -> &SharedSchema {
        match self {
            Self::Cartesian(cursor) => &cursor.schema,
            Self::Keyed(cursor) => &cursor.storage.schema,
        }
    }
    pub(crate) fn produced(&self) -> usize {
        match self {
            Self::Cartesian(cursor) => cursor.produced(),
            Self::Keyed(cursor) => cursor.produced,
        }
    }
    pub(crate) fn stop(&mut self) {
        match self {
            Self::Cartesian(cursor) => cursor.stop(),
            Self::Keyed(cursor) => {
                cursor.candidates = None;
                cursor.stopped = true;
            }
        }
    }
    pub(crate) fn next<D: DatasetView<Id = I> + Sync>(
        &mut self,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<Option<RetainedRow<I>>, EvalError> {
        match self {
            Self::Cartesian(cursor) => cursor.next(ctx),
            Self::Keyed(cursor) => cursor.next(ctx),
        }
    }
}
