// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The governed row-ingest core: the one admission sequence every operator that
//! ingests rows **from outside the dataset** runs before each row it keeps.
//!
//! Two operators produce rows the dataset's own indexes never sized: `SERVICE`
//! (whatever a remote endpoint chose to send) and a property-function call (whatever
//! a host relation chose to emit). Both are therefore the only bags in the evaluator
//! whose size an outside party picks, and both need exactly the same three things to
//! happen, in exactly this order, for every row:
//!
//! 1. **Observe the cell ceiling BEFORE interning.** The ceiling is an admission
//!    boundary: the first over-limit row must be refused *before* its storage is
//!    constructed and before its values enter the scratch arena, or the ceiling has
//!    already been exceeded by the time it is consulted.
//! 2. **Charge the per-row point, if the caller minted one.** Charging in row order
//!    is what makes the ingested prefix a **positional prefix** of the producer's
//!    answer, which is what lets the truncation certificate describe it.
//! 3. **Intern, then store.** Only a row that survived both gates is materialized.
//!
//! Writing that sequence once is not tidiness. The two call sites disagreeing about
//! the order — charging before observing, or interning before either — would make the
//! governor that trips depend on which operator produced the row, and a frozen
//! governor vector is precisely a record of which governor tripped when.
//!
//! # What this core deliberately does NOT do
//!
//! It does not pull rows. `SERVICE` ingests an already-materialized response; a
//! property-function call pulls from a live cursor and must poll its stop signal
//! between pulls (the deaf-relation doctrine on [`crate::property_fn::PfCursor`]).
//! Folding the pull into the core would force one of those two shapes onto the other.
//! The core is the **admission** step; the producers keep their own loops.

use purrdf_core::{DatasetView, TermValue, TrippedGovernor};

use crate::eval::EvalCtx;
use crate::governor::ChargePoint;
use crate::solution::Solution;

/// The verdict of [`GovernedRowIngest::admit`] for one candidate row.
#[derive(Debug)]
pub(crate) enum RowAdmission {
    /// The row may be interned and stored.
    Admitted,
    /// The ingest is over. The candidate is NOT stored, and the producer stops: its
    /// bag is the prefix already accepted, and the payload is the certificate for it.
    ///
    /// The payload is an `Option` because a ceiling can be *reached* without a
    /// governor reporting a trip — an observation the state declines to trip on
    /// (already-latched, or a disengaged dimension) — and the honest report of that is
    /// "stop here, with no new governor to name" rather than a fabricated cause.
    Abandoned(Option<TrippedGovernor>),
}

/// The per-operator admission state of one ingest: the row width the ceiling is
/// denominated against, the ceiling itself, and the per-row charge point.
///
/// Built once, immediately before the producer's row loop, so an ungoverned execution
/// answers every question below from `None` fields and performs no atomic operation
/// per row.
#[derive(Debug)]
pub(crate) struct GovernedRowIngest {
    /// The output schema width — the `columns` half of the cell denomination.
    width: usize,
    /// The largest number of rows this bag may hold under the intermediate-cell
    /// ceiling, or `None` when the dimension is not engaged (see
    /// [`EvalCtx::cell_row_ceiling`]).
    cell_ceiling: Option<usize>,
    /// The per-row fuel charge point, when the operator has a dedicated one.
    ///
    /// `None` is not "free": it means the operator's rows are metered by the generic
    /// per-node accounting every algebra node already pays
    /// ([`ChargePoint::AlgebraNodeEntry`] on the way in, and
    /// [`ChargePoint::CommittedOutputRow`] over the node's committed bag on the way
    /// out) rather than by a point of its own. It is the single seam a dedicated
    /// point slots into.
    charge_point: Option<ChargePoint>,
}

impl GovernedRowIngest {
    /// Prepare the ingest of a `width`-column bag, charging `charge_point` per row.
    pub(crate) fn new<D: DatasetView + Sync>(
        ctx: &EvalCtx<'_, D>,
        width: usize,
        charge_point: Option<ChargePoint>,
    ) -> Self {
        Self {
            width,
            cell_ceiling: ctx.cell_row_ceiling(width),
            charge_point,
        }
    }

    /// The allocation to reserve for a producer that already knows how many rows it
    /// could at most deliver: the ceiling, when one is tighter than that count.
    ///
    /// A producer with no such count (a cursor) passes `0` and grows normally.
    pub(crate) fn capacity_for(&self, upper_bound: usize) -> usize {
        self.cell_ceiling
            .map_or(upper_bound, |cap| cap.min(upper_bound))
    }

    /// Decide one candidate row, given how many rows the bag already holds.
    ///
    /// `accepted` is the current bag length, so the candidate would be row
    /// `accepted + 1`: that is the size recorded against the cell ceiling, which is why
    /// an exactly-full bag is complete and only the *next* qualifying row is a
    /// truncation.
    pub(crate) fn admit<D: DatasetView + Sync>(
        &self,
        ctx: &EvalCtx<'_, D>,
        accepted: usize,
    ) -> RowAdmission {
        if self.cell_ceiling.is_some_and(|cap| accepted >= cap) {
            // The attempted peak, recorded exactly — and the ingest ends here whatever
            // the observation reports, because storing the row would put the bag past
            // the ceiling that was just consulted.
            return RowAdmission::Abandoned(
                ctx.observe_cells(accepted.saturating_add(1), self.width)
                    .err(),
            );
        }
        if let Some(point) = self.charge_point
            && let Err(tripped) = ctx.charge(point)
        {
            return RowAdmission::Abandoned(Some(tripped));
        }
        RowAdmission::Admitted
    }

    /// Intern one admitted row's owned values into the per-query scratch space,
    /// producing a `width`-wide [`Solution`].
    ///
    /// Cells past `width` are dropped and missing cells stay unbound, so a producer
    /// that miscounts its own columns cannot produce a ragged bag.
    ///
    /// This is the positional form, for a producer whose row arrives as a whole tuple in
    /// output-column order. A producer that fills its row by COLUMN — the
    /// property-function dispatch, which copies its input row's cells through untouched
    /// and interns only the call's own positions — interns through the same
    /// [`ScratchInterner`](crate::scratch::ScratchInterner) at the same point in the
    /// sequence, immediately after [`Self::admit`] returned
    /// [`RowAdmission::Admitted`].
    pub(crate) fn intern_row<D: DatasetView + Sync>(
        &self,
        ctx: &mut EvalCtx<'_, D>,
        cells: impl IntoIterator<Item = Option<TermValue>>,
    ) -> Solution<D::Id> {
        let mut row: Solution<D::Id> = smallvec::smallvec![None; self.width];
        for (i, cell) in cells.into_iter().enumerate().take(self.width) {
            if let Some(value) = cell {
                row[i] = Some(ctx.scratch.intern(ctx.dataset, value));
            }
        }
        row
    }
}
