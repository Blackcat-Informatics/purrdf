// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The per-row checkpoint every row loop passes before each row's work.
//!
//! `FILTER`, `BIND`, `UNFOLD` and an aggregate's accumulation each do a bounded amount of
//! work per input row, and each row is where three things must happen, in this order:
//!
//! 1. **An already-latched trip is observed.** A governor that stopped this execution —
//!    below this operator, on another worker, or at a seam outside the evaluator — ends
//!    the loop before the next row's work, never after it.
//! 2. **The row is admitted.** Its charge point is charged, so the rows the loop keeps
//!    are a positional prefix of its input and a refused row's work never runs.
//! 3. **The stop signal is polled**, as a checkpoint standing for the work since the
//!    previous one — which is where a deadline or a cancellation is observed and where an
//!    asynchronous job slices its evaluation into turns.
//!
//! [`RowCheckpoint::pass`] is that sequence, once, for every one of those loops.
//!
//! # Sequential and forked
//!
//! A loop evaluated on the evaluation's own context ([`RowAdmission::Sequential`]) charges
//! each row through [`EvalCtx::charge`], which charges and then polls — at every row when
//! only a signal is attached, on the fuel interval when fuel is engaged.
//!
//! A loop forked across workers ([`RowAdmission::Forked`]) cannot charge from a worker:
//! workers run in no order, and a charge made from one would land in the shared counters
//! in whatever order the scheduler picked. So a worker **records** each row's admission
//! in a ledger of its own instead, and polls the signal once a stride of rows — one row
//! when only a signal is attached, the rows one [`STOP_POLL_FUEL`] interval of admissions
//! pays for when fuel is engaged. After the join, [`RowCheckpoint::commit`] folds the
//! workers' ledgers in source order through
//! [`GovernorState::commit_reported_items`](crate::governor::GovernorState::commit_reported_items),
//! which charges each admission exactly as the sequential loop does. The fold trips at the
//! row the sequential loop trips at, charges the same fuel up to and including the refused
//! row, and the output is cut to the rows admitted before it: the same certified prefix.
//!
//! Before forking, the loop forks only the rows the fuel ceiling can admit
//! ([`RowCheckpoint::forked_len`]), so a refused row's work does not run on the forked path
//! either. That count is a prediction read from the counters; the ordered fold after the
//! join is what decides.
//!
//! # Work is reported once
//!
//! A sequential row's fuel is reported to the signal by the charge that spends it. A forked
//! row's work is reported by the worker's stride poll, and the rows a worker passed after
//! its last poll are handed to the governor state's pending work at the commit, for the
//! next poll to report. The commit's own charges report nothing, because the work they pay
//! for has already been reported. See [`StopSignal::poll_after_work`](crate::governor::StopSignal::poll_after_work).

use purrdf_core::{DatasetView, ResourceDimension, TrippedGovernor};

use crate::eval::EvalCtx;
use crate::governor::{ChargePoint, ItemCharge, STOP_POLL_FUEL};

/// How a row loop admits its rows: the mode a [`RowCheckpoint`] runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RowAdmission {
    /// Nothing to charge and no signal to poll: the checkpoint does nothing.
    Free,
    /// The loop runs on the evaluation's own context; each row is charged, then the
    /// signal polled, immediately before the row's work.
    Sequential,
    /// The loop is forked; each worker records its rows' admissions and polls the signal
    /// once a stride, and the admissions are committed in source order after the join.
    Forked,
}

/// One row loop's checkpoint, passed before each row's work. See the module docs.
///
/// A forked loop builds one with [`Self::forked`] and hands every worker a clone of it; a
/// worker's clone accumulates that worker's rows, and the clones come back, in chunk
/// order, to [`Self::commit`] on the loop's own copy.
#[derive(Debug, Clone)]
pub(crate) struct RowCheckpoint {
    /// The mode this checkpoint runs in.
    admission: RowAdmission,
    /// The charge point one row's admission charges.
    point: ChargePoint,
    /// Forked: whether fuel is engaged, so that each row's admission is recorded in
    /// `ledger` and committed after the join.
    fuel: bool,
    /// Forked: rows between polls of the stop signal; `0` never polls.
    stride: u64,
    /// Forked: the work one row stands for when a poll reports it.
    row_work: u64,
    /// Forked: rows this worker passed since its last poll.
    since: u64,
    /// Forked, under fuel: one entry per row this worker admitted, in source order, with
    /// the output rows it kept.
    ledger: Vec<ItemCharge>,
    /// Forked, without fuel: output rows this worker kept.
    kept: usize,
    /// Forked: whether this worker stopped at a row, skipping every row after it.
    stopped: bool,
    /// How many leading input rows a forked loop forks; unbounded unless fuel is engaged.
    forked_len: usize,
    /// Forked: whether input rows past `forked_len` were withheld because the fuel
    /// ceiling cannot admit them, so the commit charges the first of them as the
    /// sequential loop's refused charge.
    withheld: bool,
}

impl RowCheckpoint {
    /// A checkpoint in `admission` mode charging `point`, before any row.
    const fn with_mode(admission: RowAdmission, point: ChargePoint) -> Self {
        Self {
            admission,
            point,
            fuel: false,
            stride: 0,
            row_work: 0,
            since: 0,
            ledger: Vec::new(),
            kept: 0,
            stopped: false,
            forked_len: usize::MAX,
            withheld: false,
        }
    }

    /// The checkpoint for a `FILTER`, `BIND` or `UNFOLD` row loop over `len` input rows
    /// admitting at `point`, forked across workers when `forked`.
    ///
    /// Such a loop passes a live checkpoint only when fuel is engaged or a stop signal is
    /// attached. Otherwise its rows are admitted for free and nothing is observed per row,
    /// which is how an execution governed by an answer cap or a cell ceiling alone has
    /// always evaluated these loops.
    pub(crate) fn for_rows<D: DatasetView + Sync>(
        ctx: &EvalCtx<'_, D>,
        point: ChargePoint,
        forked: bool,
        len: usize,
    ) -> Self {
        let Some(state) = ctx.governor_state() else {
            return Self::with_mode(RowAdmission::Free, point);
        };
        let fuel = state.is_engaged_in(ResourceDimension::Fuel);
        let stop = state.stop_signal().is_some();
        if !fuel && !stop {
            return Self::with_mode(RowAdmission::Free, point);
        }
        if !forked {
            return Self::with_mode(RowAdmission::Sequential, point);
        }
        let (stride, row_work) = match (stop, fuel) {
            (false, _) => (0, 0),
            (true, false) => (1, 1),
            (true, true) => {
                let cost = point.cost().max(1);
                ((STOP_POLL_FUEL / cost).max(1), cost)
            }
        };
        let forked_len = if fuel {
            state.fuel_admits(point.cost()).map_or(len, |admits| {
                usize::try_from(admits).map_or(len, |n| n.min(len))
            })
        } else {
            len
        };
        Self {
            fuel,
            stride,
            row_work,
            forked_len,
            // Under a trip latched before the loop, the sequential loop's first charge is
            // refused without being counted, so there is no refused admission to commit.
            withheld: forked_len < len && state.tripped().is_none(),
            ..Self::with_mode(RowAdmission::Forked, point)
        }
    }

    /// The checkpoint for a sequential row loop that charges `point` per row under any
    /// governor — an aggregate's accumulation, whose charge observes a latched trip
    /// whenever the execution is governed at all.
    pub(crate) fn sequential<D: DatasetView + Sync>(
        ctx: &EvalCtx<'_, D>,
        point: ChargePoint,
    ) -> Self {
        let admission = if ctx.governor_state().is_some() {
            RowAdmission::Sequential
        } else {
            RowAdmission::Free
        };
        Self::with_mode(admission, point)
    }

    /// How many of a forked loop's `len` leading input rows it forks: every row, unless
    /// fuel is engaged and cannot admit them all. Rows past it are never evaluated.
    pub(crate) fn forked_len(&self, len: usize) -> usize {
        self.forked_len.min(len)
    }

    /// Pass the checkpoint before one row's work.
    ///
    /// `Err` ends the loop: the row is not admitted and its work must not run. On the
    /// sequential path the error is the trip that stopped the execution; on the forked
    /// path the worker also skips every later row it is handed.
    ///
    /// # Errors
    ///
    /// The governor that stopped this execution, once one has.
    #[inline]
    pub(crate) fn pass<D: DatasetView + Sync>(
        &mut self,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<(), TrippedGovernor> {
        if self.admission == RowAdmission::Free {
            return Ok(());
        }
        let Some(state) = ctx.governor_state() else {
            return Ok(());
        };
        // An already-latched trip is observed before anything else, on either path.
        if let Some(tripped) = state.tripped() {
            self.stopped = true;
            return Err(tripped);
        }
        if self.admission == RowAdmission::Sequential {
            // Charge, then poll: `EvalCtx::charge` polls on the fuel interval when fuel is
            // engaged and at every charge when only a signal is attached.
            return ctx.charge(self.point);
        }
        if self.stride > 0 {
            self.since += 1;
            if self.since >= self.stride {
                let work = self.since.saturating_mul(self.row_work);
                self.since = 0;
                if let Some(cause) = state.poll_stop_after(work) {
                    self.stopped = true;
                    return Err(state
                        .tripped()
                        .unwrap_or(TrippedGovernor::Stopped { cause }));
                }
            }
        }
        if self.fuel {
            self.ledger.push(ItemCharge {
                fuel: self.point.cost(),
                committed: 0,
            });
        }
        Ok(())
    }

    /// Count one output row the row just admitted kept. Only a forked loop needs to: its
    /// commit cuts the output at the row its fold refuses.
    #[inline]
    pub(crate) fn keep(&mut self) {
        if self.admission != RowAdmission::Forked {
            return;
        }
        if self.fuel {
            if let Some(last) = self.ledger.last_mut() {
                last.committed += 1;
            }
        } else {
            self.kept += 1;
        }
    }

    /// Commit a forked loop after the join: fold the workers' admissions in source order,
    /// and cut `rows` — the reduced output of the workers whose checkpoints are `chunks`,
    /// in chunk order — to the rows admitted before the first refusal or stop.
    ///
    /// Every chunk before the first one that stopped ran to its end, and that one stopped
    /// at a row, so what is kept is a positional prefix of the loop's output. Under fuel
    /// the fold then charges the admissions of that prefix — plus, when rows were
    /// withheld from the fork and no worker stopped, the refused admission of the first
    /// withheld row — so the consumption, the trip and the kept prefix are the sequential
    /// loop's. A no-op for a loop that did not fork.
    pub(crate) fn commit<D: DatasetView + Sync, R>(
        &self,
        ctx: &EvalCtx<'_, D>,
        rows: &mut Vec<R>,
        chunks: impl IntoIterator<Item = Self>,
    ) {
        if self.admission != RowAdmission::Forked {
            return;
        }
        let Some(state) = ctx.governor_state() else {
            return;
        };
        let mut ledger: Vec<ItemCharge> = Vec::new();
        let mut through = 0_usize;
        let mut stopped = false;
        let mut unreported = 0_u64;
        for chunk in chunks {
            unreported = unreported.saturating_add(chunk.since.saturating_mul(chunk.row_work));
            if stopped {
                continue;
            }
            if self.fuel {
                ledger.extend(chunk.ledger);
            } else {
                through += chunk.kept;
            }
            stopped = chunk.stopped;
        }
        // The rows each worker passed after its last poll, for the next poll to report.
        state.note_work(unreported);
        if !self.fuel {
            if stopped {
                rows.truncate(through);
            }
            return;
        }
        if self.withheld && !stopped {
            ledger.push(ItemCharge {
                fuel: self.point.cost(),
                committed: 0,
            });
        }
        let (admitted, kept) = match state.commit_reported_items(&ledger) {
            Some((index, committed, _)) => (index, committed),
            None => (
                ledger.len(),
                ledger.iter().map(|item| item.committed).sum::<u64>(),
            ),
        };
        rows.truncate(usize::try_from(kept).unwrap_or(usize::MAX));
        ctx.note_fuel(
            self.point,
            self.point
                .cost()
                .saturating_mul(u64::try_from(admitted).unwrap_or(u64::MAX)),
        );
    }
}
