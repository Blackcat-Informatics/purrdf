// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original row order for joins with no shared columns.

use super::{merge_admitted, reserve_join_rows, right_to_out_map};
use crate::eval::EvalCtx;
use crate::solution::{RetainedRow, RowBag, RowsBuilder, SharedSchema, SolutionSeq};
use crate::{AdmittedVec, EvalError};
use purrdf_core::{DatasetView, ViewTermId};

#[derive(Clone, Copy)]
pub(super) struct CartesianInput<'a, I: ViewTermId> {
    pub(super) left: &'a [RetainedRow<I>],
    pub(super) right: &'a [RetainedRow<I>],
    pub(super) left_width: usize,
    pub(super) right_to_out: &'a [usize],
    pub(super) output_width: usize,
}

/// Complete drains and retained factor reads use the same advancement,
/// inclusive cell refusal, sink admission and original merge implementation.
#[derive(Default)]
pub(super) struct CartesianRows {
    left: usize,
    right: usize,
    produced: usize,
    ceiling: Option<usize>,
    stopped: bool,
}

impl CartesianRows {
    pub(super) fn with_ceiling(ceiling: Option<usize>) -> Self {
        Self {
            ceiling,
            ..Self::default()
        }
    }

    pub(super) fn next<D: DatasetView + Sync>(
        &mut self,
        input: &CartesianInput<'_, D::Id>,
        ctx: &EvalCtx<'_, D>,
        admit: impl FnOnce() -> Result<(), EvalError>,
    ) -> Result<Option<RetainedRow<D::Id>>, EvalError> {
        if self.stopped || input.right.is_empty() || self.left >= input.left.len() {
            return Ok(None);
        }
        if self.ceiling.is_some_and(|ceiling| self.produced >= ceiling) {
            let _ = ctx.observe_cells(self.produced.saturating_add(1), input.output_width);
            self.stopped = true;
            return Ok(None);
        }
        admit()?;
        let row = merge_admitted(
            &input.left[self.left],
            &input.right[self.right],
            input.left_width,
            input.right_to_out,
            input.output_width,
            &ctx.growth,
        )?;
        self.right += 1;
        if self.right == input.right.len() {
            self.right = 0;
            self.left += 1;
        }
        self.produced = self
            .produced
            .checked_add(1)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        Ok(Some(row))
    }
}

pub(super) fn drain<D: DatasetView + Sync>(
    input: &CartesianInput<'_, D::Id>,
    ctx: &EvalCtx<'_, D>,
) -> Result<RowBag<D::Id>, EvalError> {
    let ceiling = ctx.cell_row_ceiling(input.output_width);
    if ctx.growth.is_bounded() || ceiling.is_some() {
        let mut cursor = CartesianRows::with_ceiling(ceiling);
        let mut rows = RowsBuilder::new(&ctx.growth);
        while let Some(row) = cursor.next(input, ctx, || Ok(()))? {
            rows.push_row(row)?;
        }
        return rows.finish();
    }
    let (rows, _) = crate::parallel::par_chunk_try_map_init(
        ctx.sequential_operation_required(),
        input.left,
        || (),
        |(), output, left| {
            let input = CartesianInput {
                left: std::slice::from_ref(left),
                ..*input
            };
            let mut cursor = CartesianRows::default();
            while let Some(row) =
                cursor.next(&input, ctx, || reserve_join_rows(output, 1, usize::MAX))?
            {
                output.push(row);
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

/// Only the two factors and their original admitted layout remain resident;
/// the Cartesian answer is never retained by this owner.
pub(crate) struct CartesianProduct<I: ViewTermId> {
    left: SolutionSeq<I>,
    right: SolutionSeq<I>,
    pub(crate) schema: SharedSchema,
    right_to_out: AdmittedVec<usize>,
    rows: CartesianRows,
}

impl<I: ViewTermId> CartesianProduct<I> {
    pub(crate) fn stop(&mut self) {
        self.rows.stopped = true;
    }

    pub(crate) fn produced(&self) -> usize {
        self.rows.produced
    }

    pub(crate) fn new<D: DatasetView<Id = I> + Sync>(
        left: SolutionSeq<I>,
        right: SolutionSeq<I>,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<Self, EvalError> {
        if !left
            .schema
            .shared_columns_admitted(&right.schema, &ctx.growth)?
            .is_empty()
        {
            return Err(EvalError::internal(
                "a disjoint join retained shared columns",
            ));
        }
        let schema = left.schema.union_admitted(&right.schema, &ctx.growth)?;
        let right_to_out = right_to_out_map(&right.schema, &schema, &ctx.growth)?;
        Self::from_layout(left, right, schema, right_to_out, ctx)
    }

    pub(crate) fn from_layout<D: DatasetView<Id = I> + Sync>(
        left: SolutionSeq<I>,
        right: SolutionSeq<I>,
        schema: crate::solution::VarSchema,
        right_to_out: AdmittedVec<usize>,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<Self, EvalError> {
        let rows = CartesianRows::with_ceiling(ctx.cell_row_ceiling(schema.len()));
        Ok(Self {
            left,
            right,
            right_to_out,
            rows,
            schema: schema.shared_admitted(&ctx.growth)?,
        })
    }

    pub(crate) fn next<D: DatasetView<Id = I> + Sync>(
        &mut self,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<Option<RetainedRow<I>>, EvalError> {
        self.rows.next(
            &CartesianInput {
                left: &self.left.rows,
                right: &self.right.rows,
                left_width: self.left.schema.len(),
                right_to_out: &self.right_to_out,
                output_width: self.schema.len(),
            },
            ctx,
            || Ok(()),
        )
    }
}
