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
//! # Work charged inside a row is charged at its row
//!
//! A row's expression can charge from inside its own evaluation: an
//! `xsd:integer`/`xsd:decimal` operation past the machine words charges its limb work as
//! fuel and its working set as scratch before it runs, after bringing the arena's own
//! charge up to date; a custom aggregate charges its state; a fold charges the values it
//! keeps. On a forked loop the worker that evaluates the row cannot make those charges,
//! for the same reason it cannot charge the row's admission. So each worker of a governed
//! forked loop gets an [`ExactDeferral`], which records every such charge in the order
//! the worker's context makes it — the arena's growth as the size its arena had reached
//! — and [`RowCheckpoint::settle`] ends the row's ledger entry there. After the join, the
//! ordered commit ([`commit_items`]) makes each row's admission and then each of its
//! charges again on the evaluation's own context, in source order, counting the worker's
//! own mints into the evaluation's arena up to each growth charge — kept there as ghosts,
//! which a later intern of an equal value stores without counting again
//! (`ScratchInterner::count_worker_mint`), so that the arena's copies the loop forks
//! carry only real terms. The evaluation's arena then accounts, at every charge, for
//! what the loop run in order would hold, so the commit trips at the charge that loop
//! trips at, with the same consumption; and as in that loop,
//! a trip inside a row's evaluation is recorded on the expression barrier, which withholds
//! the operator's output, while a trip on a row's admission keeps the rows before it.
//!
//! Each charge is admitted in the worker against the headroom the ceilings had when the
//! loop forked — a snapshot, so the decision depends on the row alone and never on the
//! schedule — and a refused charge stops the worker, because the commit is certain to trip
//! at or before it. A worker also stops once its own rows have spent the fuel snapshot or
//! the scratch snapshot, the bytes they minted included for a loop that keeps its mints
//! ([`RowCheckpoint::settle_minted`]), which bounds what a forked loop does and holds past
//! a ceiling by one snapshot per chunk. A worker's mints can count terms another chunk
//! minted first, which the commit charges once, so a worker can stop at a row the commit
//! admits: the commit then reports the row to resume at, and the loop finishes in order.
//!
//! [`ItemLedger`] is the same arrangement for a loop whose items are not rows of one
//! input — the groups of a `GROUP BY`, the left rows of an `OPTIONAL` filter — and which
//! has no admission of its own to charge.
//!
//! A governed loop forks only under fuel and scratch ceilings
//! ([`EvalCtx::may_fork_governed_loop`]); an answer cap, a cell ceiling or a
//! remote-request ceiling is charged where no ordered commit sees it.
//!
//! # Work is reported once
//!
//! A sequential row's fuel is reported to the signal by the charge that spends it. A forked
//! row's work is reported by the worker's stride poll, and the rows a worker passed after
//! its last poll are handed to the governor state's pending work at the commit, for the
//! next poll to report. The commit's own charges report nothing, because the work they pay
//! for has already been reported. See [`StopSignal::poll_after_work`](crate::governor::StopSignal::poll_after_work).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use purrdf_core::{DatasetView, ResourceDimension, TermValue, TrippedGovernor};

use crate::error::EvalError;
use crate::eval::EvalCtx;
use crate::governor::{ChargePoint, ItemCharge, STOP_POLL_FUEL};

/// One charge a forked loop's worker made from inside an item's evaluation, in the order
/// it made them, for the loop's ordered commit ([`commit_items`]) to make again on the
/// evaluation's own context.
#[derive(Debug, Clone, Copy)]
enum Deferred {
    /// Fuel, at `point` when the charge named one.
    Fuel {
        /// The charge point, for the explanation's ledger.
        point: Option<ChargePoint>,
        /// The units charged.
        units: u64,
    },
    /// Scratch bytes charged explicitly (a custom aggregate's state, a fold's values).
    Scratch(u64),
    /// The arena's charge brought up to date, when the worker's arena held this many
    /// computed values: the commit counts the worker's values up to it into the
    /// evaluation's arena and charges that arena's growth, as the in-order loop charges
    /// its own at the same point.
    Growth(usize),
    /// A transient working set (an arbitrary-precision operation's), admitted beside
    /// everything already charged.
    Transient(u64),
}

/// A forked loop's worker-side account of what its items charge from inside their own
/// evaluation: see the module docs' "Work charged inside a row is charged at its row".
///
/// One worker's context holds it ([`EvalCtx::exact_deferral`]); the worker evaluates its
/// items one at a time, so it is only ever touched by one thread, and the atomics and the
/// lock are there only so that the context stays `Sync`.
#[derive(Debug)]
pub(crate) struct ExactDeferral {
    /// The fuel the ceiling admitted when the loop forked; `u64::MAX` unbounded.
    fuel_left: u64,
    /// The scratch bytes the ceiling admitted when the loop forked; `u64::MAX` unbounded.
    scratch_left: u64,
    /// Whether scratch is engaged, so that its charges are recorded for the commit.
    scratch_engaged: bool,
    /// Fuel the current item has been charged, the refused charge included.
    fuel: AtomicU64,
    /// Scratch bytes the current item has charged explicitly, the refused charge included.
    scratch: AtomicU64,
    /// Whether a charge of the current item was refused.
    refused: AtomicBool,
    /// Every charge this worker's items made, in order.
    charges: Mutex<Vec<Deferred>>,
}

impl ExactDeferral {
    fn new(headroom: (u64, u64), scratch_engaged: bool) -> Self {
        Self {
            fuel_left: headroom.0,
            scratch_left: headroom.1,
            scratch_engaged,
            fuel: AtomicU64::new(0),
            scratch: AtomicU64::new(0),
            refused: AtomicBool::new(false),
            charges: Mutex::new(Vec::new()),
        }
    }

    fn record(&self, charge: Deferred) {
        self.charges
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(charge);
    }

    /// The refusal every charge of an item reports once one of its charges was refused.
    const fn refusal(dimension: ResourceDimension, limit: u64, consumed: u64) -> TrippedGovernor {
        TrippedGovernor::Budget {
            dimension,
            limit,
            consumed,
        }
    }

    /// Admit `units` of fuel at `point` (`None` for fuel charged under no point) for the
    /// current item, or refuse them: once refused, an item admits nothing more. The
    /// refusal names the ceiling it would pass; the loop's ordered commit, not this
    /// value, decides the trip the execution reports.
    ///
    /// # Errors
    ///
    /// The budget the charge would pass.
    pub(crate) fn charge_fuel(
        &self,
        point: Option<ChargePoint>,
        units: u64,
    ) -> Result<(), TrippedGovernor> {
        let spent = self.fuel.load(Ordering::Relaxed);
        if self.refused.load(Ordering::Relaxed) {
            return Err(Self::refusal(
                ResourceDimension::Fuel,
                self.fuel_left,
                spent,
            ));
        }
        let would = spent.saturating_add(units);
        self.fuel.store(would, Ordering::Relaxed);
        // Unbounded headroom is fuel not engaged: the commit would charge nothing.
        if self.fuel_left != u64::MAX {
            self.record(Deferred::Fuel { point, units });
        }
        if would > self.fuel_left {
            self.refused.store(true, Ordering::Relaxed);
            return Err(Self::refusal(
                ResourceDimension::Fuel,
                self.fuel_left,
                would,
            ));
        }
        Ok(())
    }

    /// Admit `bytes` of scratch charged explicitly for the current item.
    ///
    /// # Errors
    ///
    /// The budget the charge would pass.
    pub(crate) fn charge_scratch(&self, bytes: u64) -> Result<(), TrippedGovernor> {
        let spent = self.scratch.load(Ordering::Relaxed);
        if self.refused.load(Ordering::Relaxed) {
            return Err(Self::refusal(
                ResourceDimension::ScratchBytes,
                self.scratch_left,
                spent,
            ));
        }
        let would = spent.saturating_add(bytes);
        self.scratch.store(would, Ordering::Relaxed);
        self.record(Deferred::Scratch(bytes));
        if would > self.scratch_left {
            self.refused.store(true, Ordering::Relaxed);
            return Err(Self::refusal(
                ResourceDimension::ScratchBytes,
                self.scratch_left,
                would,
            ));
        }
        Ok(())
    }

    /// Record that the worker's context brought its arena's charge up to date, its arena
    /// then holding `computed` values. Never refused here: the commit charges the growth.
    pub(crate) fn note_growth(&self, computed: usize) {
        if self.scratch_engaged && !self.refused.load(Ordering::Relaxed) {
            self.record(Deferred::Growth(computed));
        }
    }

    /// Admit a transient working set of `bytes` for the current item.
    ///
    /// # Errors
    ///
    /// The budget the admission would pass.
    pub(crate) fn admit_transient(&self, bytes: u64) -> Result<(), TrippedGovernor> {
        if self.refused.load(Ordering::Relaxed) {
            return Err(Self::refusal(
                ResourceDimension::ScratchBytes,
                self.scratch_left,
                bytes,
            ));
        }
        self.record(Deferred::Transient(bytes));
        if bytes > self.scratch_left {
            self.refused.store(true, Ordering::Relaxed);
            return Err(Self::refusal(
                ResourceDimension::ScratchBytes,
                self.scratch_left,
                bytes,
            ));
        }
        Ok(())
    }

    /// Whether scratch is engaged for this loop.
    pub(crate) const fn scratch_engaged(&self) -> bool {
        self.scratch_engaged
    }

    /// End the current item: its fuel, its explicit scratch, whether a charge of it was
    /// refused, and how many charges the worker has recorded so far.
    fn end_item(&self) -> (u64, u64, bool, usize) {
        let recorded = self
            .charges
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len();
        (
            self.fuel.swap(0, Ordering::Relaxed),
            self.scratch.swap(0, Ordering::Relaxed),
            self.refused.swap(false, Ordering::Relaxed),
            recorded,
        )
    }

    fn take_charges(&self) -> Vec<Deferred> {
        core::mem::take(&mut *self.charges.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

/// The [`ExactDeferral`] a context of a forked loop's worker charges through: the
/// worker's own context, whose arena the loop's commit replays, or a context derived
/// from it (a function body's), whose arena it does not.
#[derive(Debug, Clone)]
pub(crate) struct WorkerDeferral {
    /// The worker's deferral.
    shared: Arc<ExactDeferral>,
    /// Whether the context's arena is the worker's own.
    owns_arena: bool,
}

impl WorkerDeferral {
    /// The deferral a context derived from this one charges through.
    pub(crate) fn inherited(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
            owns_arena: false,
        }
    }

    /// Whether the context's arena is the worker's own, replayed by the commit.
    pub(crate) const fn owns_arena(&self) -> bool {
        self.owns_arena
    }
}

impl core::ops::Deref for WorkerDeferral {
    type Target = ExactDeferral;

    fn deref(&self) -> &ExactDeferral {
        &self.shared
    }
}

/// One item of a forked loop, as its worker recorded it for the ordered commit.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct LedgerItem {
    /// The fuel the item's own admission charges, before its work.
    admission: u64,
    /// The worker's recorded charges up to the end of this item.
    charges_end: usize,
    /// The worker's arena held this many computed values when the item ended.
    values_end: usize,
    /// The output rows the item produced.
    rows: usize,
}

/// One worker's ledger of a forked loop: the items it evaluated, what they charged and
/// minted, and whether it stopped. Shared by [`RowCheckpoint`] and [`ItemLedger`].
#[derive(Debug, Clone)]
struct WorkerLedger {
    /// The headroom `(fuel, scratch bytes)` at the fork.
    headroom: (u64, u64),
    /// This worker's deferral, installed by [`Self::defer`].
    deferral: Option<Arc<ExactDeferral>>,
    /// The worker's arena size at the fork: its own mints start here.
    base: usize,
    /// The worker's own mints, in mint order, taken by [`Self::finish`] when the commit
    /// replays the arena's growth.
    values: Vec<(u64, TermValue)>,
    /// The worker's recorded charges, taken by [`Self::finish`].
    charges: Vec<Deferred>,
    /// One entry per item this worker evaluated, in item order.
    items: Vec<LedgerItem>,
    /// The fuel this worker's items have charged, admissions included.
    spent: u64,
    /// The scratch bytes this worker's items have charged explicitly or minted.
    scratch_spent: u64,
    /// Whether this worker stopped at an item, skipping every item after it.
    stopped: bool,
}

impl WorkerLedger {
    const fn at(headroom: (u64, u64)) -> Self {
        Self {
            headroom,
            deferral: None,
            base: 0,
            values: Vec::new(),
            charges: Vec::new(),
            items: Vec::new(),
            spent: 0,
            scratch_spent: 0,
            stopped: false,
        }
    }

    fn defer<D: DatasetView + Sync>(&mut self, worker: &mut EvalCtx<'_, D>) {
        let scratch_engaged = worker
            .governor_state()
            .is_some_and(|state| state.is_engaged_in(ResourceDimension::ScratchBytes));
        let deferral = Arc::new(ExactDeferral::new(self.headroom, scratch_engaged));
        worker.exact_deferral = Some(WorkerDeferral {
            shared: Arc::clone(&deferral),
            owns_arena: true,
        });
        self.deferral = Some(deferral);
        self.base = worker.scratch.computed_count();
        // The worker's arena is a copy of the evaluation's: only what its items mint from
        // here on is theirs.
        let _ = worker.scratch.claim_uncharged_growth();
    }

    /// End the item just evaluated on `worker`, which produced `rows` output rows (`None`:
    /// the item [`RowCheckpoint::pass`] pushed, whose rows were counted as kept).
    fn settle<D: DatasetView + Sync>(
        &mut self,
        worker: &EvalCtx<'_, D>,
        rows: Option<usize>,
        minting: bool,
    ) {
        let Some(deferral) = &self.deferral else {
            return;
        };
        let (fuel, scratch, refused, charges_end) = deferral.end_item();
        let minted = if minting && deferral.scratch_engaged() {
            worker.scratch.claim_uncharged_growth()
        } else {
            0
        };
        let values_end = worker.scratch.computed_count();
        self.spent = self.spent.saturating_add(fuel);
        self.scratch_spent = self
            .scratch_spent
            .saturating_add(scratch)
            .saturating_add(minted);
        match rows {
            Some(rows) => self.items.push(LedgerItem {
                admission: 0,
                charges_end,
                values_end,
                rows,
            }),
            None => {
                if let Some(last) = self.items.last_mut() {
                    last.charges_end = charges_end;
                    last.values_end = values_end;
                }
            }
        }
        if refused || self.spent > self.headroom.0 || self.scratch_spent > self.headroom.1 {
            self.stopped = true;
        }
    }

    /// Take the worker's charges, and its mints when the commit replays the arena's
    /// growth, once its chunk is done.
    fn finish<D: DatasetView + Sync>(&mut self, worker: &mut EvalCtx<'_, D>) {
        if let Some(deferral) = &self.deferral {
            self.charges = deferral.take_charges();
            if deferral.scratch_engaged() {
                self.values = worker.scratch.take_values_from(self.base);
            }
        }
    }
}

/// The headroom `(fuel, scratch bytes)` the ceilings have now: what a forked loop's
/// workers admit their items' work against.
fn headroom<D: DatasetView + Sync>(ctx: &EvalCtx<'_, D>) -> (u64, u64) {
    let Some(state) = ctx.governor_state() else {
        return (u64::MAX, u64::MAX);
    };
    let left = |dimension| {
        if state.is_engaged_in(dimension) {
            state
                .limit_for(dimension)
                .saturating_sub(state.consumed_in(dimension))
        } else {
            u64::MAX
        }
    };
    (
        left(ResourceDimension::Fuel),
        left(ResourceDimension::ScratchBytes),
    )
}

/// Where [`commit_items`] tripped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemTrip {
    /// On an item's admission, before its work: every item before it is whole.
    Admission,
    /// On a charge from inside an item's evaluation, which the sequential loop records on
    /// the expression barrier: the item's output was computed by a run the sequential
    /// loop would have cut short.
    Work,
}

/// What [`commit_items`] commits: the workers' ledgers in chunk order, up to and including
/// the first that stopped, and whether a worker stopped.
struct Committing {
    chunks: Vec<WorkerLedger>,
    stopped: bool,
}

impl Committing {
    fn of(chunks: impl IntoIterator<Item = WorkerLedger>) -> Self {
        let mut kept = Vec::new();
        let mut stopped = false;
        for chunk in chunks {
            stopped = chunk.stopped;
            kept.push(chunk);
            if stopped {
                break;
            }
        }
        Self {
            chunks: kept,
            stopped,
        }
    }

    fn items(&self) -> usize {
        self.chunks.iter().map(|chunk| chunk.items.len()).sum()
    }
}

/// Commit a forked loop's items, recorded by its workers in source order: for each item
/// in turn, charge its admission, then make again every charge it made from inside its
/// evaluation, in the order it made them — fuel, explicit scratch, transient admissions,
/// and the arena's growth, which the commit reproduces by counting the worker's own
/// mints into the evaluation's arena up to that point, so the arena accounts for what
/// the in-order loop's would hold and grows by what it would — and hand its rows (the
/// next of
/// `rows`, which hold every item's output in order) to `admit_row`, which re-interns a
/// minted row into the evaluation's arena. The first refusal ends the loop: the item it
/// refuses keeps no row, and nothing after it is admitted. Every charge happens on the
/// evaluation's own context, in source order, so the trip, the consumption and the kept
/// prefix are the in-order loop's however the loop was forked.
///
/// An item whose totals fit beside what is already charged, whatever their order, cannot
/// be refused at any one of its charges, so it is charged in one step per dimension, to
/// the same consumption; only an item that might trip has its charges made one by one.
///
/// `withheld` is a refused admission to charge after the last item when no item stopped
/// the loop: the first row a fuel-sized fork did not evaluate.
///
/// Returns the kept rows, and where an item tripped a ceiling, if one did (a trip on
/// `withheld` alone is not an item's).
///
/// # Errors
///
/// An error `admit_row` raises, or a read of the dataset an intern needs.
fn commit_items<D: DatasetView + Sync, R, S>(
    ctx: &mut EvalCtx<'_, D>,
    point: Option<ChargePoint>,
    committing: Committing,
    rows: Vec<R>,
    withheld: Option<u64>,
    mut admit_row: impl FnMut(&mut EvalCtx<'_, D>, R) -> Result<S, EvalError>,
) -> Result<(Vec<S>, Option<ItemTrip>), EvalError> {
    let mut out = Vec::with_capacity(rows.len());
    let mut rows = rows.into_iter();
    let Some(state) = ctx.governor_state().cloned() else {
        for row in rows {
            out.push(admit_row(ctx, row)?);
        }
        return Ok((out, None));
    };
    let fuel = state.is_engaged_in(ResourceDimension::Fuel);
    let scratch = state.is_engaged_in(ResourceDimension::ScratchBytes);
    let charge_fuel = |units: u64| {
        fuel && state
            .commit_reported_items(&[ItemCharge {
                fuel: units,
                committed: 0,
            }])
            .is_some()
    };
    if scratch {
        ctx.scratch.reserve_ghosts(
            committing
                .chunks
                .iter()
                .map(|chunk| chunk.values.len())
                .sum(),
        );
    }
    for chunk in committing.chunks {
        let charges = chunk.charges;
        let mut charged = 0_usize;
        let mut values = chunk.values.into_iter();
        let mut interned = chunk.base;
        for item in chunk.items {
            if item.admission > 0 {
                if charge_fuel(item.admission) {
                    return Ok((out, Some(ItemTrip::Admission)));
                }
                if let Some(point) = point
                    && fuel
                {
                    ctx.note_fuel(point, item.admission);
                }
            }
            let end = item.charges_end.clamp(charged, charges.len());
            let span = &charges[charged..end];
            charged = end;
            // The item's totals, and whether they fit whatever the order inside the item:
            // then the ceilings cannot refuse any one of its charges, and the item is
            // charged in one step per dimension, to the same consumption. Otherwise its
            // charges are made one by one, so the trip lands on the one the in-order loop
            // refuses.
            let (work, explicit, transient) = span.iter().fold(
                (0_u64, 0_u64, 0_u64),
                |(work, explicit, transient), charge| match *charge {
                    Deferred::Fuel { units, .. } => {
                        (work.saturating_add(units), explicit, transient)
                    }
                    Deferred::Scratch(bytes) => (work, explicit.saturating_add(bytes), transient),
                    Deferred::Growth(_) => (work, explicit, transient),
                    Deferred::Transient(bytes) => (work, explicit, transient.max(bytes)),
                },
            );
            let minted = if scratch {
                let pending = item.values_end.saturating_sub(interned);
                values
                    .as_slice()
                    .iter()
                    .take(pending)
                    .fold(0_u64, |sum, (_, value)| {
                        sum.saturating_add(crate::scratch::value_bytes(value))
                    })
            } else {
                0
            };
            let fits = |dimension: ResourceDimension, more: u64| {
                state.consumed_in(dimension).saturating_add(more) <= state.limit_for(dimension)
            };
            let whole = state.tripped().is_none()
                && (!fuel || fits(ResourceDimension::Fuel, work))
                && (!scratch
                    || fits(
                        ResourceDimension::ScratchBytes,
                        ctx.scratch
                            .uncharged_growth()
                            .saturating_add(minted)
                            .saturating_add(explicit)
                            .saturating_add(transient),
                    ));
            let mut count_to = |ctx: &mut EvalCtx<'_, D>, upto: usize| {
                while interned < upto {
                    let Some((hash, value)) = values.next() else {
                        break;
                    };
                    interned += 1;
                    ctx.scratch.count_worker_mint(hash, value);
                }
            };
            if whole {
                if fuel {
                    for charge in span {
                        if let Deferred::Fuel {
                            point: Some(point),
                            units,
                        } = *charge
                        {
                            ctx.note_fuel(point, units);
                        }
                    }
                }
                count_to(ctx, item.values_end);
                let refused = (work > 0 && charge_fuel(work))
                    || (scratch
                        && (state
                            .charge_if_engaged(ResourceDimension::ScratchBytes, explicit)
                            .is_err()
                            || ctx.charge_scratch_growth().is_err()
                            || (transient > 0
                                && state
                                    .admit_transient(ResourceDimension::ScratchBytes, transient)
                                    .is_err())));
                // Refused only when another evaluation shares the ceilings and charged in
                // between.
                if refused {
                    return Ok((out, Some(ItemTrip::Work)));
                }
            } else {
                for charge in span {
                    let refused = match *charge {
                        Deferred::Fuel { point, units } => {
                            let refused = charge_fuel(units);
                            if !refused
                                && fuel
                                && let Some(point) = point
                            {
                                ctx.note_fuel(point, units);
                            }
                            refused
                        }
                        Deferred::Scratch(bytes) => {
                            scratch
                                && state
                                    .charge_if_engaged(ResourceDimension::ScratchBytes, bytes)
                                    .is_err()
                        }
                        Deferred::Growth(upto) => {
                            count_to(ctx, upto);
                            scratch && ctx.charge_scratch_growth().is_err()
                        }
                        Deferred::Transient(bytes) => {
                            scratch
                                && state
                                    .admit_transient(ResourceDimension::ScratchBytes, bytes)
                                    .is_err()
                        }
                    };
                    if refused {
                        return Ok((out, Some(ItemTrip::Work)));
                    }
                }
                // The rest of what the item minted, so the arena holds what the in-order
                // loop's holds when the next item, or the operator's own output, charges
                // it.
                if scratch {
                    count_to(ctx, item.values_end);
                }
            }
            for row in rows.by_ref().take(item.rows) {
                out.push(admit_row(ctx, row)?);
            }
        }
    }
    if let (Some(units), false) = (withheld, committing.stopped) {
        let _ = charge_fuel(units);
    }
    Ok((out, None))
}

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
/// A forked loop builds one with [`Self::for_rows`] and hands every worker a clone of it;
/// a worker's clone accumulates that worker's rows, and the clones come back, in chunk
/// order, to [`Self::commit`] on the loop's own copy.
#[derive(Debug, Clone)]
pub(crate) struct RowCheckpoint {
    /// The mode this checkpoint runs in.
    admission: RowAdmission,
    /// The charge point one row's admission charges.
    point: ChargePoint,
    /// Forked: whether fuel is engaged, so that each row's admission is charged at the
    /// commit.
    fuel: bool,
    /// Forked: rows between polls of the stop signal; `0` never polls.
    stride: u64,
    /// Forked: the work one row stands for when a poll reports it.
    row_work: u64,
    /// Forked: rows this worker passed since its last poll.
    since: u64,
    /// How many leading input rows a forked loop forks; unbounded unless fuel is engaged.
    forked_len: usize,
    /// Forked: whether input rows past `forked_len` were withheld because the fuel
    /// ceiling cannot admit them, so the commit charges the first of them as the
    /// sequential loop's refused charge.
    withheld: bool,
    /// Forked: this worker's ledger.
    ledger: WorkerLedger,
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
            forked_len: usize::MAX,
            withheld: false,
            ledger: WorkerLedger::at((u64::MAX, u64::MAX)),
        }
    }

    /// The checkpoint for a `FILTER`, `BIND` or `UNFOLD` row loop over `len` input rows
    /// admitting at `point`, forked across workers when `forked`.
    ///
    /// Such a loop passes a live checkpoint only when fuel or a scratch ceiling is
    /// engaged or a stop signal is attached. Otherwise its rows are admitted for free and
    /// nothing is observed per row, which is how an execution governed by an answer cap
    /// or a cell ceiling alone has always evaluated these loops.
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
        let scratch = state.is_engaged_in(ResourceDimension::ScratchBytes);
        let stop = state.stop_signal().is_some();
        if !(fuel || stop || (forked && scratch)) {
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
            ledger: WorkerLedger::at(headroom(ctx)),
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
            self.ledger.stopped = true;
            return Err(tripped);
        }
        if self.admission == RowAdmission::Sequential {
            // Charge, then poll: `EvalCtx::charge` polls on the fuel interval when fuel is
            // engaged and at every charge when only a signal is attached.
            return ctx.charge(self.point);
        }
        // A worker stopped at a row of its own skips every later row.
        if self.ledger.stopped {
            return Err(TrippedGovernor::Budget {
                dimension: ResourceDimension::Fuel,
                limit: self.ledger.headroom.0,
                consumed: self.ledger.spent,
            });
        }
        if self.stride > 0 {
            self.since += 1;
            if self.since >= self.stride {
                let work = self.since.saturating_mul(self.row_work);
                self.since = 0;
                if let Some(cause) = state.poll_stop_after(work) {
                    self.ledger.stopped = true;
                    return Err(state
                        .tripped()
                        .unwrap_or(TrippedGovernor::Stopped { cause }));
                }
            }
        }
        let admission = if self.fuel { self.point.cost() } else { 0 };
        self.ledger.spent = self.ledger.spent.saturating_add(admission);
        self.ledger.items.push(LedgerItem {
            admission,
            ..LedgerItem::default()
        });
        Ok(())
    }

    /// Install a fresh [`ExactDeferral`] on `worker`, the context this checkpoint's
    /// worker evaluates its rows on, when the loop is forked under a governor. A no-op
    /// otherwise.
    pub(crate) fn defer<D: DatasetView + Sync>(&mut self, worker: &mut EvalCtx<'_, D>) {
        if self.admission == RowAdmission::Forked && worker.governor_state().is_some() {
            self.ledger.defer(worker);
        }
    }

    /// Settle the row just evaluated on `worker`: end its ledger entry, and stop the
    /// worker when a charge of the row was refused or the worker's rows have spent the
    /// headroom the loop forked with — in which case the ordered commit trips at or
    /// before this row. A no-op without a deferral.
    pub(crate) fn settle<D: DatasetView + Sync>(&mut self, worker: &EvalCtx<'_, D>) {
        self.ledger.settle(worker, None, false);
    }

    /// [`Self::settle`] for a loop whose kept rows carry the terms they mint (a `BIND`):
    /// the bytes the row minted into `worker`'s arena also count against the scratch
    /// headroom, so a worker stops once its rows have minted past it rather than minting
    /// the rest of its chunk.
    ///
    /// The worker's growth can exceed what the commit charges for the same rows (the
    /// evaluation's arena may already hold a term another chunk minted), so a worker can
    /// stop at a row the commit admits; [`Self::commit_resuming`] then reports where the
    /// loop resumes.
    pub(crate) fn settle_minted<D: DatasetView + Sync>(&mut self, worker: &EvalCtx<'_, D>) {
        self.ledger.settle(worker, None, true);
    }

    /// Count one output row the row just admitted kept. Only a forked loop needs to: its
    /// commit hands each row's output on with it.
    #[inline]
    pub(crate) fn keep(&mut self) {
        if self.admission != RowAdmission::Forked {
            return;
        }
        if let Some(last) = self.ledger.items.last_mut() {
            last.rows += 1;
        }
    }

    /// This worker's checkpoint once its chunk is done, `fresh` taking its place.
    pub(crate) fn finish<D: DatasetView + Sync>(
        &mut self,
        worker: &mut EvalCtx<'_, D>,
        fresh: &Self,
    ) -> Self {
        self.ledger.finish(worker);
        core::mem::replace(self, fresh.clone())
    }

    /// Commit a forked loop after the join: `rows` is the reduced output of the workers
    /// whose checkpoints are `chunks`, in chunk order. Every chunk before the first one
    /// that stopped ran to its end, and that one stopped at a row, so the items to commit
    /// are a positional prefix of the loop's rows; [`commit_items`] charges them in that
    /// order and hands each admitted item's rows to `admit_row`, so the consumption, the
    /// trip and the kept prefix are the sequential loop's. A trip inside a row's work is
    /// recorded on the expression barrier, as the sequential loop's is. For a loop that did
    /// not fork, every row goes to `admit_row` unchanged.
    ///
    /// # Errors
    ///
    /// An error `admit_row` raises.
    pub(crate) fn commit<D: DatasetView + Sync, R, S>(
        &self,
        ctx: &mut EvalCtx<'_, D>,
        rows: Vec<R>,
        chunks: impl IntoIterator<Item = Self>,
        admit_row: impl FnMut(&mut EvalCtx<'_, D>, R) -> Result<S, EvalError>,
    ) -> Result<Vec<S>, EvalError> {
        let (rows, resume) = self.commit_resuming(ctx, rows, chunks, admit_row)?;
        // Only a worker settled with `settle_minted` stops on an estimate.
        debug_assert!(
            resume.is_none(),
            "a loop that settles exactly never resumes"
        );
        Ok(rows)
    }

    /// [`Self::commit`] for a loop settled with [`Self::settle_minted`]: also returns the
    /// input row the loop must resume at, on the evaluation's own context and in order,
    /// when a worker stopped on its minted bytes at a row the commit admitted — every row
    /// before it committed, and no ceiling tripped.
    ///
    /// # Errors
    ///
    /// An error `admit_row` raises.
    pub(crate) fn commit_resuming<D: DatasetView + Sync, R, S>(
        &self,
        ctx: &mut EvalCtx<'_, D>,
        rows: Vec<R>,
        chunks: impl IntoIterator<Item = Self>,
        mut admit_row: impl FnMut(&mut EvalCtx<'_, D>, R) -> Result<S, EvalError>,
    ) -> Result<(Vec<S>, Option<usize>), EvalError> {
        if self.admission != RowAdmission::Forked || ctx.governor_state().is_none() {
            let rows = rows
                .into_iter()
                .map(|row| admit_row(ctx, row))
                .collect::<Result<_, _>>()?;
            return Ok((rows, None));
        }
        let chunks: Vec<Self> = chunks.into_iter().collect();
        // The rows each worker passed after its last poll, for the next poll to report.
        let unreported = chunks.iter().fold(0_u64, |sum, chunk| {
            sum.saturating_add(chunk.since.saturating_mul(chunk.row_work))
        });
        if let Some(state) = ctx.governor_state() {
            state.note_work(unreported);
        }
        let committing = Committing::of(chunks.into_iter().map(|chunk| chunk.ledger));
        let withheld = self.withheld.then(|| self.point.cost());
        let evaluated = committing.items();
        let stopped = committing.stopped;
        let (out, trip) =
            commit_items(ctx, Some(self.point), committing, rows, withheld, admit_row)?;
        Ok((out, settle_commit(ctx, trip, stopped, evaluated)))
    }
}

/// After a commit that tripped at `trip`: record a trip inside an item's work on the
/// expression barrier, as the sequential loop's trip there is, and report the item the
/// loop resumes at when a worker `stopped` at an item the commit admitted.
fn settle_commit<D: DatasetView + Sync>(
    ctx: &EvalCtx<'_, D>,
    trip: Option<ItemTrip>,
    stopped: bool,
    evaluated: usize,
) -> Option<usize> {
    let latched = ctx.governor_state().and_then(|state| state.tripped());
    if trip == Some(ItemTrip::Work)
        && let Some(tripped) = latched
    {
        ctx.record_barrier(tripped);
    }
    (stopped && trip.is_none() && latched.is_none()).then_some(evaluated)
}

/// The ordered ledger of a forked loop over items that are not rows of one input — the
/// groups of a `GROUP BY`, the left rows of an `OPTIONAL` filter — and that have no
/// admission of their own to charge: see the module docs.
///
/// The loop builds one with [`Self::for_items`] and hands every worker a clone; a
/// worker's clone [`defers`](Self::defer) its context's charges, records one entry per
/// item it evaluates, and comes back, in chunk order, to [`Self::commit`]. Without a
/// governor it records nothing.
#[derive(Debug, Clone)]
pub(crate) struct ItemLedger {
    /// Whether the loop is governed, so that its items are recorded and committed.
    governed: bool,
    /// This worker's ledger.
    ledger: WorkerLedger,
}

impl ItemLedger {
    /// The ledger of a loop about to fork.
    pub(crate) fn for_items<D: DatasetView + Sync>(ctx: &EvalCtx<'_, D>) -> Self {
        Self {
            governed: ctx.governor_state().is_some(),
            ledger: WorkerLedger::at(headroom(ctx)),
        }
    }

    /// Install a fresh [`ExactDeferral`] on `worker`. A no-op without a governor.
    pub(crate) fn defer<D: DatasetView + Sync>(&mut self, worker: &mut EvalCtx<'_, D>) {
        if self.governed {
            self.ledger.defer(worker);
        }
    }

    /// Whether the worker evaluates its next item: `false` once it stopped.
    pub(crate) const fn admits(&self) -> bool {
        !self.ledger.stopped
    }

    /// Record the item just evaluated on `worker`, which produced `rows` output rows:
    /// what it charged, and a stop when a charge of it was refused or the worker's items
    /// have spent the headroom — in either case the commit trips at or before this item.
    pub(crate) fn settle<D: DatasetView + Sync>(&mut self, rows: usize, worker: &EvalCtx<'_, D>) {
        self.ledger.settle(worker, Some(rows), false);
    }

    /// [`Self::settle`] for a loop whose output rows carry the terms its items mint (a
    /// group's aggregate values): see [`RowCheckpoint::settle_minted`].
    pub(crate) fn settle_minted<D: DatasetView + Sync>(
        &mut self,
        rows: usize,
        worker: &EvalCtx<'_, D>,
    ) {
        self.ledger.settle(worker, Some(rows), true);
    }

    /// This worker's ledger once its chunk is done, `fresh` taking its place.
    pub(crate) fn finish<D: DatasetView + Sync>(
        &mut self,
        worker: &mut EvalCtx<'_, D>,
        fresh: &Self,
    ) -> Self {
        self.ledger.finish(worker);
        core::mem::replace(self, fresh.clone())
    }

    /// Commit the loop after the join: [`commit_items`] over the workers' items in item
    /// order, `rows` being their reduced output in chunk order. An item's output is a
    /// function of its whole evaluation (a group's aggregates, a left row's padding), and
    /// a trip inside it is recorded on the expression barrier, as the sequential loop's
    /// is. Without a governor every row goes to `admit_row` unchanged.
    ///
    /// # Errors
    ///
    /// An error `admit_row` raises.
    pub(crate) fn commit<D: DatasetView + Sync, R, S>(
        &self,
        ctx: &mut EvalCtx<'_, D>,
        rows: Vec<R>,
        chunks: impl IntoIterator<Item = Self>,
        admit_row: impl FnMut(&mut EvalCtx<'_, D>, R) -> Result<S, EvalError>,
    ) -> Result<Vec<S>, EvalError> {
        let (rows, resume) = self.commit_resuming(ctx, rows, chunks, admit_row)?;
        debug_assert!(
            resume.is_none(),
            "a loop that settles exactly never resumes"
        );
        Ok(rows)
    }

    /// [`Self::commit`] for a loop settled with [`Self::settle_minted`]: also returns the
    /// item the loop must resume at, on the evaluation's own context and in order, when a
    /// worker stopped on its minted bytes at an item the commit admitted.
    ///
    /// # Errors
    ///
    /// An error `admit_row` raises.
    pub(crate) fn commit_resuming<D: DatasetView + Sync, R, S>(
        &self,
        ctx: &mut EvalCtx<'_, D>,
        rows: Vec<R>,
        chunks: impl IntoIterator<Item = Self>,
        mut admit_row: impl FnMut(&mut EvalCtx<'_, D>, R) -> Result<S, EvalError>,
    ) -> Result<(Vec<S>, Option<usize>), EvalError> {
        if !self.governed {
            let rows = rows
                .into_iter()
                .map(|row| admit_row(ctx, row))
                .collect::<Result<_, _>>()?;
            return Ok((rows, None));
        }
        let committing = Committing::of(chunks.into_iter().map(|chunk| chunk.ledger));
        let evaluated = committing.items();
        let stopped = committing.stopped;
        let (out, trip) = commit_items(ctx, None, committing, rows, None, admit_row)?;
        Ok((out, settle_commit(ctx, trip, stopped, evaluated)))
    }
}
