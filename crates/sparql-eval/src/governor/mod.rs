// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Caller-supplied execution governors for the evaluation tier: configuration
//! ([`QueryGovernors`]), the stop-signal primitive ([`StopSignal`] and its two shipped
//! implementations), live operation-local accounting ([`GovernorState`]), and the
//! content-hashed identity of the charge schedule ([`GOVERNOR_PROFILE_ID`],
//! [`GOVERNOR_PROFILE_VERSION`], [`GOVERNOR_PROFILE_DIGEST`]).
//!
//! `lift` holds the partial-lift channel — the third result an operator can produce,
//! carrying the rows it holds together with a machine-checked statement of what they
//! bound — and `soundness` holds the companion static analysis: the one exhaustive algebra visitor
//! that decides what a truncated bag at a node still certifies about the root answer, and
//! — the same computation read for a different purpose — whether the answer cap may be
//! pushed down to that node.
//!
//! # A governor never changes an answer, only an outcome
//!
//! A resource ceiling decides whether a caller receives the complete answer or a
//! certified subset plus a typed cause. The rows themselves are never different. That is
//! what separates a governor from semantic optionality: two callers running the same
//! query over the same data with different budgets never disagree about *which* rows are
//! answers, only about *how many* of them were reached.
//!
//! # Determinism
//!
//! Charging is deterministic by construction on [`ResourceDimension::Fuel`],
//! [`ResourceDimension::AnswerRows`], and [`ResourceDimension::IntermediateCells`]: the
//! same query over the same data under the same ceilings consumes exactly the same
//! amount and trips at exactly the same point. Parallel evaluation preserves this
//! because charges are accumulated **chunk-locally** ([`ItemCharge`], one record per
//! input row, no atomics and nothing shared) and folded in source-item order
//! ([`GovernorState::commit_ordered_items`]) — the same order-stable reduction
//! `crate::parallel` already uses for errors. A deadline trip is inherently
//! time-dependent and carries no such determinism claim.
//!
//! # The schema of a partial result (a stated contract, not an accident)
//!
//! A complete result reports the columns the query produces. A **partial** result reports
//! the columns of the operator arms that were actually evaluated, and for four operators
//! those are not the same thing:
//!
//! | Operator | Truncated arm | Columns reported |
//! |---|---|---|
//! | `JOIN`, `OPTIONAL`, `LATERAL`, `UNION` | left | the **left arm's** columns only |
//! | everything else, and the right arm of all four | — | the node's true columns |
//!
//! When the left arm of one of those four truncates, the right arm is deliberately never
//! evaluated — starting a fresh subtree after the budget is spent is unbounded work a
//! governor must not license — and this engine's column ORDER is chosen during
//! evaluation, not parsed off the query: a basic graph pattern's columns appear in the
//! order the cost-based join order visits them. So the right arm's columns are not
//! derivable without doing the work that was just refused, and reporting a guess would be
//! worse than reporting less.
//!
//! No row is affected: in every one of these cases the surviving rows are exactly the
//! left arm's, or none at all. Only the reported column list is narrower than a complete
//! run's, and a caller that diffs column lists across the complete and partial paths must
//! expect that. Where the true schema *is* derivable without the refused work — every
//! unary operator, `MINUS` (whose schema is its left arm's), `PROJECT` (whose schema is
//! its variable list), and `GROUP BY` (grouping variables then aggregate outputs) — the
//! true schema is reported, including when no rows cross at all.
//!
//! # The vocabulary is a kernel type
//!
//! [`StopCause`], [`ResourceDimension`], [`ResourceVector`], [`TrippedGovernor`], and
//! [`GovernorEvidence`] all live in [`purrdf_core`] so that the demand-paging tier and
//! the evaluation tier name one taxonomy. This module adds only the evaluation tier's
//! configuration, live state, and profile identity.

#[cfg(test)]
mod charge_points;
pub(crate) mod lift;
pub(crate) mod soundness;

pub use lift::NonMonotoneBarrier;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, OnceLock};
use std::time::Duration;

use purrdf_core::{
    GovernorEvidence, ResourceDimension, ResourceVector, StopCause, TrippedGovernor,
};
use sha2::Digest;

use crate::eval::MAX_UDF_DEPTH;

// ---------------------------------------------------------------------------
// Stop signals
// ---------------------------------------------------------------------------

/// A host-supplied reason to stop evaluating, polled by the evaluator at its charge
/// points.
///
/// A deadline and a cancellation are the same primitive and differ only in the
/// [`StopCause`] reported, so one trait covers both. The evaluator core never reads a
/// clock: every deadline check is a `poll` of one of these.
///
/// # The latching contract (binding on implementors)
///
/// Once `poll` returns `Some(cause)`, it **must** return `Some(cause)` — the same cause —
/// on every later call, forever. An implementation that can un-trip is not a governor: a
/// query that observed its deadline at one charge point and then observed a clear signal
/// at the next would resume past that deadline, and the caller who set the deadline would
/// be told the query completed inside a budget it had already blown through. Latching is
/// what makes "stopped" a terminal state rather than a momentary observation, and it is
/// what lets [`GovernorState`] record the cause once and report it consistently
/// afterwards.
///
/// Both shipped implementations latch: [`CancellationFlag`] by construction (its flag is
/// monotone), [`WallDeadline`] by holding an explicit latch bit that short-circuits every
/// later poll without re-reading the clock.
pub trait StopSignal: Send + Sync + std::fmt::Debug {
    /// The cause, if this signal has fired. Latching: see the trait documentation.
    fn poll(&self) -> Option<StopCause>;
}

/// A shareable cancellation bit a host can flip from any thread.
///
/// Clone it: every clone shares one flag, so a caller keeps a handle, hands another to
/// the engine as an `Arc<dyn StopSignal>`, and cancels a running query through its own
/// handle.
///
/// Latching is by construction — the bit only ever moves from clear to set, and nothing
/// clears it. Build a new flag for a new query rather than resetting one.
#[derive(Debug, Clone, Default)]
pub struct CancellationFlag {
    /// The shared monotone bit. `Relaxed` on both sides is sufficient: it carries no
    /// accompanying data that a reader must see, so there is nothing for an
    /// acquire/release pair to order.
    cancelled: Arc<AtomicBool>,
}

impl CancellationFlag {
    /// A fresh, uncancelled flag.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Cancel every clone of this flag. Idempotent, and never reversible.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Whether this flag has been cancelled.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

impl StopSignal for CancellationFlag {
    fn poll(&self) -> Option<StopCause> {
        if self.is_cancelled() {
            Some(StopCause::Cancelled)
        } else {
            None
        }
    }
}

/// The host clock read, in milliseconds since a fixed origin, written per target exactly
/// as [`crate::clock::wall_clock_now`] is.
///
/// Native: [`std::time::Instant`], which is monotonic, so the rewind branch in
/// [`WallDeadline::poll`] is unreachable through this reader on native targets. The
/// origin is a process-wide first-read instant; only differences of the returned value
/// are ever meaningful.
#[cfg(not(target_arch = "wasm32"))]
fn host_millis() -> f64 {
    static ORIGIN: OnceLock<std::time::Instant> = OnceLock::new();
    ORIGIN
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_secs_f64()
        * 1000.0
}

/// The host clock read, in milliseconds since a fixed origin, written per target exactly
/// as [`crate::clock::wall_clock_now`] is.
///
/// wasm32: `js_sys::Date::now()`, milliseconds since the Unix epoch. This is a **wall**
/// clock, not a monotonic one — see [`WallDeadline`] for why that is load-bearing.
#[cfg(target_arch = "wasm32")]
fn host_millis() -> f64 {
    js_sys::Date::now()
}

/// Where a [`WallDeadline`] reads the time from.
#[derive(Debug)]
enum DeadlineClock {
    /// The host clock, through this crate's single target-split reader.
    Host,
    /// A test-scripted millisecond source. A real host clock cannot be made to step
    /// backwards on demand, so the rewind rule is only checkable against a source the
    /// test controls.
    #[cfg(test)]
    Scripted(Arc<ScriptedClock>),
}

impl DeadlineClock {
    /// Read the current millisecond count.
    fn now_millis(&self) -> f64 {
        match self {
            Self::Host => host_millis(),
            #[cfg(test)]
            Self::Scripted(clock) => clock.now_millis(),
        }
    }
}

/// A wall-clock deadline, shipped so that setting one is not a caller obligation.
///
/// Requiring every caller to hand-roll a [`StopSignal`] to get a deadline would be
/// optionality by another name: two callers would arrive at two different notions of
/// "expired". This type is the one clock read on the governor path, and it is not
/// consulted at all when the caller supplies their own signal.
///
/// # Latching
///
/// The first poll that observes expiry sets a latch bit; every later poll short-circuits
/// on that bit without re-reading the clock. `Relaxed` ordering suffices — the bit is
/// monotone and carries no accompanying data. A thread that has not yet observed another
/// thread's latch simply re-reads the clock and reaches the same conclusion, because the
/// expiry predicate below is itself monotone in observed time.
///
/// # The rewind rule
///
/// On wasm32 the clock is `js_sys::Date::now()`, a wall clock that is NTP-steppable. A
/// backwards step would make a naive `now >= deadline` deadline **un-trippable** — a
/// silent failure in which a query outlives a budget the caller believes is enforced. So
/// the start instant is snapshotted at construction and the deadline latches on **either**
/// `now >= deadline` **or** `now < start`: an observed rewind is treated as expired, never
/// as extra budget. Failing closed is the only safe reading, because the engine cannot
/// distinguish a small clock correction from a large one.
///
/// The rule is written once and enforced on both targets. On native the source is
/// [`std::time::Instant`], which is monotonic, so the rewind branch cannot fire there;
/// it is retained rather than target-gated so that one predicate governs both builds.
#[derive(Debug)]
pub struct WallDeadline {
    /// The clock reading taken at construction, in milliseconds.
    start_millis: f64,
    /// The reading at or after which the deadline has expired, in milliseconds.
    deadline_millis: f64,
    /// The latch. Once set, no later poll reads the clock again.
    expired: AtomicBool,
    /// The time source.
    clock: DeadlineClock,
}

impl WallDeadline {
    /// A deadline `budget` from now.
    ///
    /// A zero budget is valid and expires on the first poll, matching the inclusive,
    /// zero-is-valid reading every other ceiling in this module uses.
    #[must_use]
    pub fn after(budget: Duration) -> Self {
        Self::from_clock(DeadlineClock::Host, budget)
    }

    /// Whether this deadline has already latched, without consulting the clock.
    #[must_use]
    pub fn has_expired(&self) -> bool {
        self.expired.load(Ordering::Relaxed)
    }

    /// Build a deadline of `budget` against an explicit clock, snapshotting the start.
    fn from_clock(clock: DeadlineClock, budget: Duration) -> Self {
        let start_millis = clock.now_millis();
        Self {
            start_millis,
            deadline_millis: budget.as_secs_f64().mul_add(1000.0, start_millis),
            expired: AtomicBool::new(false),
            clock,
        }
    }

    /// A deadline of `budget` over a test-controlled clock.
    #[cfg(test)]
    fn scripted(clock: &Arc<ScriptedClock>, budget: Duration) -> Self {
        Self::from_clock(DeadlineClock::Scripted(Arc::clone(clock)), budget)
    }
}

impl StopSignal for WallDeadline {
    fn poll(&self) -> Option<StopCause> {
        if self.has_expired() {
            return Some(StopCause::Deadline);
        }
        let now = self.clock.now_millis();
        // Either the budget is spent, or the clock stepped backwards past the start
        // snapshot. A rewind is treated as expiry, never as recovered budget.
        if now >= self.deadline_millis || now < self.start_millis {
            self.expired.store(true, Ordering::Relaxed);
            return Some(StopCause::Deadline);
        }
        None
    }
}

/// A millisecond source a test can step forwards and backwards at will.
#[cfg(test)]
#[derive(Debug)]
struct ScriptedClock {
    /// The reading every [`DeadlineClock::now_millis`] call returns until it is moved.
    millis: std::sync::Mutex<f64>,
}

#[cfg(test)]
impl ScriptedClock {
    /// A clock reading `millis`.
    fn new(millis: f64) -> Arc<Self> {
        Arc::new(Self {
            millis: std::sync::Mutex::new(millis),
        })
    }

    /// Move the clock to `millis`, forwards or backwards.
    fn set(&self, millis: f64) {
        *self.millis.lock().expect("scripted clock is uncontended") = millis;
    }

    /// The current reading.
    fn now_millis(&self) -> f64 {
        *self.millis.lock().expect("scripted clock is uncontended")
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// The dimensions a caller may set a ceiling on.
///
/// Every other dimension either carries a fixed build ceiling that a caller must not be
/// able to relax ([`ResourceDimension::UdfDepth`]) or belongs to the demand-paging tier,
/// which is configured through
/// [`PagedQueryLimits`](purrdf_core::ir::PagedQueryLimits) instead
/// ([`ResourceDimension::Pages`], [`ResourceDimension::Bytes`]).
const CALLER_SETTABLE_DIMENSIONS: [ResourceDimension; 5] = [
    ResourceDimension::Fuel,
    ResourceDimension::AnswerRows,
    ResourceDimension::IntermediateCells,
    ResourceDimension::ScratchBytes,
    ResourceDimension::RemoteRequests,
];

/// The ceiling [`QueryGovernors::METERED`] puts on every caller-settable dimension:
/// engaged, so the counter runs, and one below the largest representable, so nothing an
/// execution can actually consume reaches it.
const METERING_CEILING: u64 = u64::MAX - 1;

/// [`QueryGovernors::METERED`]'s ceiling vector.
///
/// Derived from [`CALLER_SETTABLE_DIMENSIONS`] rather than written out, so a dimension
/// added to that table is metered automatically instead of being silently left
/// unmeasurable — the failure mode would be a consumer sizing a budget from evidence with
/// a hole in it, which is worse than no evidence at all. Fixed-ceiling dimensions are
/// untouched: this starts from [`QueryGovernors::UNBOUNDED`]'s vector, so the recursion
/// guard survives.
const fn metering_limits() -> ResourceVector {
    let mut limits = QueryGovernors::UNBOUNDED.limits;
    let mut index = 0;
    while index < CALLER_SETTABLE_DIMENSIONS.len() {
        limits.set(CALLER_SETTABLE_DIMENSIONS[index], METERING_CEILING);
        index += 1;
    }
    limits
}

/// Whether `dimension` is compared against the **maximum** of any single observation
/// rather than against a running sum.
///
/// Two dimensions are maxima rather than totals, for the same reason in two different
/// shapes:
///
/// - [`ResourceDimension::IntermediateCells`] bounds how large one operator instance's
///   materialized bag may get. Summing it across operators would make a long, cheap
///   query indistinguishable from a single catastrophic cross product, which is the
///   failure the ceiling exists to stop.
/// - [`ResourceDimension::UdfDepth`] is a nesting *depth*. A query that calls a
///   user-defined function a thousand times at depth one has reached depth one, not
///   depth one thousand; summing it would report a stack that was never that deep and
///   refuse a call chain that never came close to the recursion guard.
///
/// Every other dimension is a genuine cumulative cost and sums.
const fn is_peak_tracked(dimension: ResourceDimension) -> bool {
    matches!(
        dimension,
        ResourceDimension::IntermediateCells | ResourceDimension::UdfDepth
    )
}

/// The fuel interval at which a [`StopSignal`] is polled.
///
/// Polling on every unit of fuel would put a virtual call on the hottest path in the
/// evaluator; polling too rarely lets a deadline overshoot. This value is a build
/// constant, never a knob: a caller-tunable poll interval would make the exact charge
/// point at which a deadline is observed part of the caller's configuration, and two
/// callers would then disagree about where an identical query stopped.
///
/// **It must not be arithmetically related to any fuel ceiling.** If the interval divided
/// (or were divided by) a natural ceiling, the stop poll and the fuel trip would land on
/// the same charge point systematically rather than incidentally, and the precedence rule
/// between them would be exercised by every query instead of by the queries that genuinely
/// race. 4093 is prime, is not a power of two, and is not a round decimal, so it shares no
/// factor with the round decimals and powers of two callers actually pick.
pub const STOP_POLL_FUEL: u64 = 4093;

/// Caller-supplied execution ceilings and stop signal for one query execution.
///
/// Ceilings are **inclusive**: consumption equal to the ceiling is admitted, and only
/// consumption that would exceed it trips. Zero is a valid ceiling and trips on the first
/// charged unit of work — the same reading
/// [`PagedQueryLimits`](purrdf_core::ir::PagedQueryLimits) documents for the I/O tier.
///
/// # There is deliberately no `Default`
///
/// [`Self::UNBOUNDED`] must be named at every construction site. An ambient default would
/// let a code path acquire ungoverned status implicitly — a caller adds a call, forgets
/// the governors, and the compiler says nothing. Governance is a security-relevant policy;
/// acquiring it (or declining it) is always written down.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct QueryGovernors {
    /// The inclusive ceilings in force, one slot per [`ResourceDimension`].
    limits: ResourceVector,
    /// The host-supplied stop signal, if any.
    stop: Option<Arc<dyn StopSignal>>,
}

impl QueryGovernors {
    /// No caller-settable ceiling and no stop signal.
    ///
    /// "Unbounded" means unbounded on every **caller-settable** dimension. Dimensions
    /// with a fixed build ceiling keep it and gain only *reporting* through
    /// [`GovernorEvidence`]: [`ResourceDimension::UdfDepth`] stays pinned at the
    /// evaluator's recursion guard here, exactly as it is on every ungoverned query. A
    /// caller cannot raise or remove that guard through this type, because a
    /// caller-relaxable stack-recursion bound is not a bound.
    ///
    /// This is an associated const rather than a constructor function, mirroring
    /// [`PagedQueryLimits::UNBOUNDED`](purrdf_core::ir::PagedQueryLimits::UNBOUNDED).
    pub const UNBOUNDED: Self = Self {
        limits: ResourceVector::UNBOUNDED.with(ResourceDimension::UdfDepth, MAX_UDF_DEPTH as u64),
        stop: None,
    };

    /// **Measure** this query's cost without bounding it: every caller-settable dimension
    /// is engaged, at a ceiling no query can reach.
    ///
    /// Run a query under this, read [`GovernorEvidence`] off the completed result, and use
    /// the numbers to choose the real ceilings. That is the intended way to size a budget,
    /// and it is why evidence is returned on the *complete* path at all.
    ///
    /// # Why [`Self::UNBOUNDED`] does not do this
    ///
    /// [`Self::UNBOUNDED`] reports **nothing**, because it charges nothing: every charge
    /// site short-circuits on [`GovernorState::is_engaged_in`] before it touches a
    /// counter, which is precisely what makes an ungoverned query cost exactly what it
    /// cost before governors existed. That is a deliberate trade, not an oversight — you
    /// cannot have a meter that is free and also a meter that reads. So the two are
    /// separate, named states: `UNBOUNDED` declines both the ceilings and the accounting;
    /// `METERED` takes the accounting and declines the ceilings.
    ///
    /// # The ceiling is high, not absent
    ///
    /// "Engaged" *means* "carries a ceiling" — an absent ceiling is exactly what the
    /// charge sites short-circuit on — so metering requires a finite number. It is one
    /// below the largest representable, so a query would have to consume `u64::MAX` units
    /// of a dimension to trip it, which no execution that fits in memory can do.
    /// Charging saturates rather than overflowing, so even the impossible case fails
    /// closed with a typed budget trip instead of a panic.
    ///
    /// Fixed-ceiling dimensions are **not** relaxed by this door either:
    /// [`ResourceDimension::UdfDepth`] keeps the evaluator's recursion guard, exactly as
    /// it does under [`Self::UNBOUNDED`].
    pub const METERED: Self = Self {
        limits: metering_limits(),
        stop: None,
    };

    /// Bound abstract execution steps.
    #[must_use]
    pub fn with_fuel(mut self, fuel: u64) -> Self {
        self.limits.set(ResourceDimension::Fuel, fuel);
        self
    }

    /// Bound rows committed to the final answer sequence.
    ///
    /// This is an operational cap, never `LIMIT`: `LIMIT` is query semantics and applies
    /// before the cap is tested.
    #[must_use]
    pub fn with_max_answers(mut self, rows: u64) -> Self {
        self.limits.set(ResourceDimension::AnswerRows, rows);
        self
    }

    /// Bound the largest intermediate bag, in cells (`rows * columns`).
    ///
    /// Compared against the maximum of any single observation, not a running sum.
    #[must_use]
    pub fn with_max_intermediate_cells(mut self, cells: u64) -> Self {
        self.limits.set(ResourceDimension::IntermediateCells, cells);
        self
    }

    /// Bound bytes minted into the per-query scratch arena by value-constructing
    /// operations, which grow independently of any row or cell count.
    #[must_use]
    pub fn with_max_scratch_bytes(mut self, bytes: u64) -> Self {
        self.limits.set(ResourceDimension::ScratchBytes, bytes);
        self
    }

    /// Bound requests issued to remote or federated endpoints.
    #[must_use]
    pub fn with_max_remote_requests(mut self, requests: u64) -> Self {
        self.limits.set(ResourceDimension::RemoteRequests, requests);
        self
    }

    /// Attach a host-supplied stop signal (a deadline, a cancellation, or both composed
    /// by the host).
    ///
    /// The signal must latch — see [`StopSignal`].
    #[must_use]
    pub fn with_stop_signal(mut self, signal: Arc<dyn StopSignal>) -> Self {
        self.stop = Some(signal);
        self
    }

    /// The inclusive ceilings in force.
    #[must_use]
    pub const fn limits(&self) -> ResourceVector {
        self.limits
    }

    /// The host-supplied stop signal, if one was attached.
    #[must_use]
    pub fn stop_signal(&self) -> Option<&Arc<dyn StopSignal>> {
        self.stop.as_ref()
    }

    /// Whether any caller-settable governor is engaged.
    ///
    /// `false` means the execution is ungoverned in the sense that matters for cost: no
    /// caller ceiling and no stop signal, so no charge site has anything to enforce.
    /// Fixed-ceiling dimensions are excluded deliberately — they are enforced on every
    /// query, governed or not, so counting them here would make every execution look
    /// governed and defeat the short-circuit.
    #[must_use]
    pub fn is_engaged(&self) -> bool {
        self.stop.is_some()
            || CALLER_SETTABLE_DIMENSIONS
                .iter()
                .any(|&dimension| self.limits.is_bounded(dimension))
    }

    /// Whether `dimension` carries a ceiling that must actually be enforced.
    ///
    /// Per-dimension engagement is what keeps a deadline-only configuration from dragging
    /// fuel-charge overhead onto the query: the fuel charge site asks this question about
    /// [`ResourceDimension::Fuel`] alone and short-circuits before touching a counter.
    ///
    /// [`ResourceDimension::UdfDepth`] answers `true` even under [`Self::UNBOUNDED`],
    /// because its fixed build ceiling is always in force.
    #[must_use]
    pub const fn is_engaged_in(&self, dimension: ResourceDimension) -> bool {
        self.limits.is_bounded(dimension)
    }
}

// ---------------------------------------------------------------------------
// Precedence
// ---------------------------------------------------------------------------

/// Pick the governor that wins when several are true at the same charge point.
///
/// The order itself is the kernel vocabulary's pinned contract,
/// [`TrippedGovernor::precedence_rank`] — stop signal (cancellation ahead of deadline)
/// ahead of the intermediate-cell ceiling, ahead of fuel, the answer cap, scratch bytes,
/// remote requests, and the recursion guard. It is defined once, beside the type, so that
/// every tier resolves a simultaneous trip identically and a new governor cannot be added
/// without being ranked.
///
/// A genuine [`EvalError`](crate::EvalError) outranks all of them: reporting "budget
/// exhausted" for a query that was in fact malformed would hand the caller a partial
/// answer for a question that has no answer. That comparison is made where evaluation
/// results are combined, not here, because this function's domain is [`TrippedGovernor`]
/// alone.
///
/// Because the stop signal is polled every [`STOP_POLL_FUEL`] units, fuel can cross its
/// ceiling *between* polls, and two conditions can become true at different charge points.
/// The rule is therefore: at each charge point, evaluate in precedence order over the
/// conditions **already true at that point** — never over conditions that might become
/// true later. Ties in rank resolve to the first candidate supplied.
#[must_use]
pub fn resolve_precedence<I>(candidates: I) -> Option<TrippedGovernor>
where
    I: IntoIterator<Item = TrippedGovernor>,
{
    candidates
        .into_iter()
        .min_by_key(|governor| governor.precedence_rank())
}

// ---------------------------------------------------------------------------
// Chunk-local accumulation
// ---------------------------------------------------------------------------

/// One **input item's** contribution to an ordered charge fold: the fuel that item spent
/// and the output rows it committed.
///
/// # Why the fold unit is an item and not a chunk
///
/// A chunk-granular ledger is the obvious design and it is wrong. Chunk geometry is
/// derived from `rayon::current_num_threads()` — `crate::parallel` splits `len` items
/// into `len / (threads * 4)`-sized slices — so folding a row loop at chunk granularity
/// would make the reported trip point depend on the machine's thread count, which is
/// exactly the dependence the ordered fold exists to remove. It is also invisible without
/// a test that varies the worker count, because every run on one machine agrees with
/// itself. Recording a charge per *item* and folding those in source-item order gives a
/// trip point that is a pure function of `(query, data, budget)`, while the accumulation
/// stays chunk-local and atomic-free.
///
/// # Why it is two counters and not a full resource vector
///
/// A record with one `u64` per dimension would allocate more per row than the rows
/// themselves cost — an allocation ceiling whose own meter is the largest allocation in
/// the query is not a ceiling. Every dimension a chunked row loop can charge is a count
/// of events, and the two counts below are all of them; the wider dimensions (the
/// intermediate-cell peak, remote requests, scratch bytes) are observed once per operator
/// instance on the main thread, where width costs nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ItemCharge {
    /// Fuel this item spent.
    pub fuel: u64,
    /// Output rows this item appended to its operator's output, used to cut the output
    /// at the item the fold refuses. Never charged here — an operator's committed rows
    /// are charged at its own boundary, so counting them again would double-charge.
    pub committed: u64,
}

// ---------------------------------------------------------------------------
// Live state
// ---------------------------------------------------------------------------

/// Live, operation-local governor accounting.
///
/// # Build one per execution
///
/// Construct a new state for every execution. Never hold one on an engine and never reuse
/// one across queries: consumption is cumulative, so a shared state would drain one
/// query's budget into the next and produce an intermittent, essentially undiscoverable
/// "this query was fine yesterday" bug. This is the same rule the demand-paging tier
/// states for [`PagedQueryView`](purrdf_core::ir::PagedQueryView) — construct a new view
/// for every operation, because caches, evidence, and limits are operation-local.
///
/// # Every field is `Sync`
///
/// The state is reached through an `Arc` from the evaluation context, which carries a
/// compile-time `Send + Sync` proof, so the counters are atomics, the tripped cell is a
/// write-once [`OnceLock`] rather than a mutex on the hot path, and the abandon flag is a
/// plain [`AtomicBool`].
#[derive(Debug)]
pub struct GovernorState {
    /// The inclusive ceilings in force.
    limits: ResourceVector,
    /// Summed consumption, one slot per dimension.
    consumed: [AtomicU64; ResourceDimension::COUNT],
    /// Maximum single observations, one slot per dimension. Only peak-tracked dimensions
    /// (see [`is_peak_tracked`]) are reported from here.
    peaks: [AtomicU64; ResourceDimension::COUNT],
    /// The governor that stopped this execution. Write-once: the first trip is the
    /// reported trip.
    tripped: OnceLock<TrippedGovernor>,
    /// Best-effort early-abandon hint. See [`GovernorState::should_abandon`].
    abandon: AtomicBool,
    /// The host-supplied stop signal, if any.
    stop: Option<Arc<dyn StopSignal>>,
}

impl GovernorState {
    /// Fresh state for one execution under `governors`.
    #[must_use]
    pub fn new(governors: &QueryGovernors) -> Self {
        Self {
            limits: governors.limits(),
            consumed: std::array::from_fn(|_| AtomicU64::new(0)),
            peaks: std::array::from_fn(|_| AtomicU64::new(0)),
            tripped: OnceLock::new(),
            abandon: AtomicBool::new(false),
            stop: governors.stop.clone(),
        }
    }

    /// The inclusive ceilings in force.
    #[must_use]
    pub const fn limits(&self) -> ResourceVector {
        self.limits
    }

    /// Charge `amount` against `dimension`, the sequential charge path.
    ///
    /// Arithmetic saturates rather than overflowing. The ceiling is **inclusive**:
    /// consumption equal to the ceiling is admitted, and only consumption that exceeds it
    /// trips. An unbounded dimension can never trip, so the accumulation still runs and
    /// still lands in [`GovernorState::evidence`] — a completed query's cost is how a
    /// caller sizes the next query's budget. Whether the charge site is entered at all is
    /// the caller's short-circuit, through
    /// [`QueryGovernors::is_engaged_in`](QueryGovernors::is_engaged_in).
    ///
    /// Once any governor has tripped the state is sticky: every later charge returns the
    /// same [`TrippedGovernor`] without doing more work.
    ///
    /// A stop signal is polled every [`STOP_POLL_FUEL`] units of
    /// [`ResourceDimension::Fuel`], and additionally at the moment a ceiling is crossed,
    /// so that conditions true at the same charge point resolve through
    /// [`resolve_precedence`] rather than by whichever happened to be tested first.
    pub fn charge(&self, dimension: ResourceDimension, amount: u64) -> Result<(), TrippedGovernor> {
        if let Some(&tripped) = self.tripped.get() {
            return Err(tripped);
        }

        let slot = &self.consumed[Self::slot(dimension)];
        let previous = slot
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                Some(current.saturating_add(amount))
            })
            .unwrap_or(0);
        let updated = previous.saturating_add(amount);

        let limit = self.limits.get(dimension);
        if updated > limit {
            return Err(self.trip(TrippedGovernor::Budget {
                dimension,
                limit,
                consumed: updated,
            }));
        }

        if dimension == ResourceDimension::Fuel
            && self.stop.is_some()
            && previous / STOP_POLL_FUEL != updated / STOP_POLL_FUEL
        {
            self.check_stop()?;
        }

        Ok(())
    }

    /// Record `observed` as a single observation of a peak-tracked `dimension`.
    ///
    /// The ceiling is compared inclusively against the **maximum** of any single
    /// observation, never against a running sum: this dimension bounds how large one
    /// intermediate bag may get, not how many bags a query is allowed to build.
    pub fn observe_peak(
        &self,
        dimension: ResourceDimension,
        observed: u64,
    ) -> Result<(), TrippedGovernor> {
        if let Some(&tripped) = self.tripped.get() {
            return Err(tripped);
        }

        self.peaks[Self::slot(dimension)].fetch_max(observed, Ordering::Relaxed);

        let limit = self.limits.get(dimension);
        if observed > limit {
            return Err(self.trip(TrippedGovernor::Budget {
                dimension,
                limit,
                consumed: observed,
            }));
        }
        Ok(())
    }

    /// Poll the stop signal, latching a fired signal into the tripped cell.
    ///
    /// Returns the cause the signal reported, which — because signals latch — is stable
    /// for the rest of the execution. If some other governor tripped first, that earlier
    /// trip stays the reported one: precedence is evaluated over conditions true at a
    /// charge point, and an earlier point's conditions were resolved when they were true.
    pub fn poll_stop(&self) -> Option<StopCause> {
        let cause = self.stop.as_ref()?.poll()?;
        self.tripped
            .get_or_init(|| TrippedGovernor::Stopped { cause });
        self.abandon.store(true, Ordering::Relaxed);
        Some(cause)
    }

    /// The host-supplied stop signal this execution runs under, if one was attached.
    ///
    /// Handed to seams that leave the evaluator and can block there — the `SERVICE`
    /// federation source is the one that exists today. A signal that only the evaluator
    /// can poll is unpollable for exactly as long as the evaluator is not running, which
    /// is precisely the window a federated call occupies, so the signal has to travel
    /// with the call rather than wait for it to return.
    #[must_use]
    pub fn stop_signal(&self) -> Option<&Arc<dyn StopSignal>> {
        self.stop.as_ref()
    }

    /// Record a trip observed **outside** the evaluator, by a seam that was handed this
    /// execution's stop signal (see [`Self::stop_signal`]).
    ///
    /// Resolved against any stop condition already true and write-once, exactly as an
    /// evaluator-side trip is: the first trip stays the reported one. Without this, a
    /// federated call that stopped would report a truncation the evidence knew nothing
    /// about — a result the caller is told is partial beside a receipt that says every
    /// governor is intact.
    pub fn record_trip(&self, candidate: TrippedGovernor) -> TrippedGovernor {
        self.trip(candidate)
    }

    /// The governor that stopped this execution, or `None` if none has.
    #[must_use]
    pub fn tripped(&self) -> Option<TrippedGovernor> {
        self.tripped.get().copied()
    }

    /// Consumption reported for `dimension`: the maximum single observation for a
    /// peak-tracked dimension, and the running sum otherwise.
    #[must_use]
    pub fn consumed_in(&self, dimension: ResourceDimension) -> u64 {
        let slot = Self::slot(dimension);
        if is_peak_tracked(dimension) {
            self.peaks[slot].load(Ordering::Relaxed)
        } else {
            self.consumed[slot].load(Ordering::Relaxed)
        }
    }

    /// A snapshot of this execution's consumption, ceilings, and trip.
    ///
    /// Produced on the complete path as well as the exhausted one: "completed, cost N
    /// fuel, peak M cells" is how a caller sizes a budget in the first place.
    #[must_use]
    pub fn evidence(&self) -> GovernorEvidence {
        let mut evidence = GovernorEvidence::new(self.limits);
        for dimension in ResourceDimension::ALL {
            evidence
                .consumed
                .set(dimension, self.consumed_in(dimension));
        }
        evidence.tripped = self.tripped();
        evidence
    }

    /// A best-effort hint that a governor has already tripped and in-flight work can stop.
    ///
    /// Read with [`Ordering::Relaxed`], so a worker may observe it late — or never — and
    /// that is deliberate.
    ///
    /// **This flag bounds *actual* work only. It is NEVER consulted to decide the reported
    /// trip point.** The reported trip is a pure function of the query, the data, and the
    /// budget, computed by the ordered fold in [`Self::commit_ordered_items`]. Letting a
    /// `Relaxed` read influence *where* a query stopped would make the answer depend on
    /// worker count and scheduling — the same query, data, and budget would produce
    /// different partial answers on different machines, which is precisely the property
    /// the ordered fold exists to guarantee.
    #[must_use]
    pub fn should_abandon(&self) -> bool {
        self.abandon.load(Ordering::Relaxed)
    }

    /// Whether `dimension` carries a ceiling this execution must enforce.
    ///
    /// This is the predicate every charge site tests **before** touching a counter, and
    /// it is what makes an ungoverned dimension free: one `Copy` array read and one
    /// compare, no atomic, no allocation. Per-dimension rather than per-execution so
    /// that a caller who set only a deadline never pays fuel-charge overhead.
    #[inline]
    #[must_use]
    pub const fn is_engaged_in(&self, dimension: ResourceDimension) -> bool {
        self.limits.is_bounded(dimension)
    }

    /// Charge `amount` against `dimension`, **only if** that dimension carries a ceiling.
    ///
    /// The short-circuit is the whole point: see [`Self::is_engaged_in`].
    ///
    /// # Errors
    ///
    /// The governor that stopped this execution, once one has.
    #[inline]
    pub fn charge_if_engaged(
        &self,
        dimension: ResourceDimension,
        amount: u64,
    ) -> Result<(), TrippedGovernor> {
        if self.is_engaged_in(dimension) {
            self.charge(dimension, amount)
        } else {
            Ok(())
        }
    }

    /// Charge one occurrence of `point` against fuel, only if fuel carries a ceiling.
    ///
    /// # Errors
    ///
    /// The governor that stopped this execution, once one has.
    #[inline]
    pub fn charge_point_if_engaged(&self, point: ChargePoint) -> Result<(), TrippedGovernor> {
        self.charge_if_engaged(ResourceDimension::Fuel, point.cost())
    }

    /// Record `observed` as a single observation of `dimension`, only if that dimension
    /// carries a ceiling.
    ///
    /// # Errors
    ///
    /// The governor that stopped this execution, once one has.
    #[inline]
    pub fn observe_peak_if_engaged(
        &self,
        dimension: ResourceDimension,
        observed: u64,
    ) -> Result<(), TrippedGovernor> {
        if self.is_engaged_in(dimension) {
            self.observe_peak(dimension, observed)
        } else {
            Ok(())
        }
    }

    /// Fold per-**item** charges in source-item order, returning the first item index at
    /// which a ceiling is crossed, the number of output rows committed strictly before
    /// that item, and the governor that tripped.
    ///
    /// This is the one ordered fold: see [`ItemCharge`] for why a row loop must be folded
    /// per item rather than per chunk — a chunk-granular sibling is deliberately not
    /// offered, because its granularity is the wrong one and an API that lets a caller
    /// reach for it invites a bug no single-machine test can see — and
    /// [`Self::should_abandon`] for why the best-effort abandon flag is never read here.
    ///
    /// Each item is charged through [`Self::charge`], the same sequential charge path a
    /// non-parallel operator uses, so a forced-sequential run and a forced-parallel run
    /// of the same operator over the same rows trip at the same item by construction
    /// rather than by two implementations agreeing.
    ///
    /// Items after the crossing are not folded in: their work is not part of the
    /// certified prefix, so charging for it would report a cost the answer does not
    /// reflect. Actual work may still overshoot the reported trip point by at most one
    /// parallel round, because every chunk already in flight runs to completion before
    /// this fold discovers the crossing. Reported work is exact.
    pub fn commit_ordered_items(
        &self,
        per_item: &[ItemCharge],
    ) -> Option<(usize, u64, TrippedGovernor)> {
        let mut committed = 0_u64;
        for (index, item) in per_item.iter().enumerate() {
            if let Err(tripped) = self.charge(ResourceDimension::Fuel, item.fuel) {
                return Some((index, committed, tripped));
            }
            committed = committed.saturating_add(item.committed);
        }
        None
    }

    /// The dense slot `dimension` occupies in the counter arrays.
    ///
    /// Written as a match rather than as a search of [`ResourceDimension::ALL`] because
    /// this runs on the charge path, which is the hottest code in a governed query. The
    /// mapping is pinned to `ALL`'s order by test, so it cannot drift from the kernel
    /// vocabulary.
    const fn slot(dimension: ResourceDimension) -> usize {
        match dimension {
            ResourceDimension::Fuel => 0,
            ResourceDimension::AnswerRows => 1,
            ResourceDimension::IntermediateCells => 2,
            ResourceDimension::ScratchBytes => 3,
            ResourceDimension::RemoteRequests => 4,
            ResourceDimension::UdfDepth => 5,
            ResourceDimension::Pages => 6,
            ResourceDimension::Bytes => 7,
        }
    }

    /// Latch a trip, resolving it against any stop signal that is already firing.
    ///
    /// The signal is polled here — once per execution, at the single moment a ceiling is
    /// crossed — so that a stop condition already true at this charge point outranks the
    /// ceiling, per [`resolve_precedence`]. The tripped cell is write-once, so an earlier
    /// trip stays the reported one.
    fn trip(&self, candidate: TrippedGovernor) -> TrippedGovernor {
        let stopped = self
            .stop
            .as_ref()
            .and_then(|signal| signal.poll())
            .map(|cause| TrippedGovernor::Stopped { cause });
        let winner = resolve_precedence(stopped.into_iter().chain(std::iter::once(candidate)))
            .unwrap_or(candidate);
        let latched = *self.tripped.get_or_init(|| winner);
        self.abandon.store(true, Ordering::Relaxed);
        latched
    }

    /// Poll the stop signal and convert a fired signal into a trip.
    fn check_stop(&self) -> Result<(), TrippedGovernor> {
        match self.poll_stop() {
            Some(cause) => Err(self.trip(TrippedGovernor::Stopped { cause })),
            None => Ok(()),
        }
    }
}

// ---------------------------------------------------------------------------
// Profile identity
// ---------------------------------------------------------------------------

/// The identifier of the execution-governor profile this build implements.
///
/// A fuel budget is meaningless across builds unless the schedule that spends it is
/// pinned. A consumer pins this identifier, [`GOVERNOR_PROFILE_VERSION`],
/// [`GOVERNOR_PROFILE_DIGEST`], and [`GOVERNOR_CORPUS_DIGEST`] together: the first two say
/// which schedule was agreed, the third says exactly what that schedule is, and the fourth
/// says which evidence was agreed to demonstrate it.
///
/// This is an IDENTIFIER, not a vocabulary term: a bare token rather than an IRI,
/// precisely so nothing can dereference it, assert with it, or mistake it for an ontology
/// PurRDF does not publish.
pub const GOVERNOR_PROFILE_ID: &str = "purrdf-sparql-governors";

/// The version of [`GOVERNOR_PROFILE_ID`] this build implements.
///
/// Incremented by any change to what a given query costs, to the precedence order, or to
/// the inclusive-boundary rule — i.e. by any change that could move the point at which a
/// caller's budget trips. A change that cannot move a charge (a refactor, a clearer
/// diagnostic) does not increment it, which is what makes the number worth pinning.
pub const GOVERNOR_PROFILE_VERSION: u32 = 1;

/// Charge schedule v1, as data rather than as scattered literals.
///
/// Each entry is `(label, cost)`. The labels are a pinned contract — a frozen corpus and
/// a per-node ledger record them — so renaming one is a breaking change, not a cosmetic
/// edit. [`ChargePoint`] is the type-safe index into this table; the table is the single
/// source of both label and cost.
///
/// Every charge point costs one unit in v1. That is deliberate: a schedule whose costs
/// are all 1 makes fuel a count of *observable evaluation events*, which is a quantity a
/// caller can reason about and a corpus can pin, rather than a weighted score whose units
/// mean nothing outside this build.
pub const CHARGE_SCHEDULE: [(&str, u64); 8] = [
    ("algebra-node-entry", 1),
    ("committed-output-row", 1),
    ("bgp-candidate-quad", 1),
    ("path-frontier-expansion", 1),
    ("row-expression-evaluation", 1),
    ("user-function-invocation", 1),
    ("remote-request-issued", 1),
    ("remote-row-ingested", 1),
];

/// A deterministic counting point in the evaluator, and the type-safe index into
/// [`CHARGE_SCHEDULE`].
///
/// Naming charge points with an enum rather than with strings means a charge site cannot
/// silently spend fuel under a label the schedule does not define, and a schedule entry
/// cannot become unreachable without the compiler noticing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChargePoint {
    /// Entering one algebra node during evaluation.
    AlgebraNodeEntry,
    /// One row committed to an operator's output.
    CommittedOutputRow,
    /// One candidate quad examined while matching a basic graph pattern.
    BgpCandidateQuad,
    /// One property-path frontier node expanded.
    PathFrontierExpansion,
    /// One expression evaluated over one row. Charged per row, not per sub-expression, so
    /// the cost is stable across planner changes.
    RowExpressionEvaluation,
    /// One user-defined function invocation.
    UserFunctionInvocation,
    /// One request issued to a remote endpoint.
    RemoteRequestIssued,
    /// One row ingested from a remote endpoint's response.
    RemoteRowIngested,
}

impl ChargePoint {
    /// Every charge point, in [`CHARGE_SCHEDULE`] order.
    pub const ALL: [Self; CHARGE_SCHEDULE.len()] = [
        Self::AlgebraNodeEntry,
        Self::CommittedOutputRow,
        Self::BgpCandidateQuad,
        Self::PathFrontierExpansion,
        Self::RowExpressionEvaluation,
        Self::UserFunctionInvocation,
        Self::RemoteRequestIssued,
        Self::RemoteRowIngested,
    ];

    /// This point's row in [`CHARGE_SCHEDULE`].
    const fn index(self) -> usize {
        match self {
            Self::AlgebraNodeEntry => 0,
            Self::CommittedOutputRow => 1,
            Self::BgpCandidateQuad => 2,
            Self::PathFrontierExpansion => 3,
            Self::RowExpressionEvaluation => 4,
            Self::UserFunctionInvocation => 5,
            Self::RemoteRequestIssued => 6,
            Self::RemoteRowIngested => 7,
        }
    }

    /// This point's pinned schedule label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        CHARGE_SCHEDULE[self.index()].0
    }

    /// The fuel this point costs under the schedule.
    #[must_use]
    pub const fn cost(self) -> u64 {
        CHARGE_SCHEDULE[self.index()].1
    }
}

impl std::fmt::Display for ChargePoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

impl GovernorState {
    /// Charge one occurrence of `point` against [`ResourceDimension::Fuel`], at the cost
    /// the schedule defines.
    ///
    /// Charging through the schedule rather than through a literal is what keeps
    /// [`GOVERNOR_PROFILE_DIGEST`] an honest description of what a query actually costs.
    pub fn charge_point(&self, point: ChargePoint) -> Result<(), TrippedGovernor> {
        self.charge(ResourceDimension::Fuel, point.cost())
    }
}

/// The canonical byte encoding of a charge schedule, which
/// [`GOVERNOR_PROFILE_DIGEST`] hashes.
///
/// Line-oriented and unambiguous: the profile identifier, then the profile version, then
/// one `label\tcost` line per entry in table order, every line terminated by `\n`. Tab and
/// newline cannot occur in a label, so no entry can be encoded two ways and no two
/// distinct schedules can encode alike.
fn schedule_preimage(id: &str, version: u32, schedule: &[(&str, u64)]) -> String {
    let mut preimage = String::new();
    preimage.push_str(id);
    preimage.push('\n');
    preimage.push_str(&version.to_string());
    preimage.push('\n');
    for (label, cost) in schedule {
        preimage.push_str(label);
        preimage.push('\t');
        preimage.push_str(&cost.to_string());
        preimage.push('\n');
    }
    preimage
}

/// The lowercase-hex SHA-256 of [`schedule_preimage`].
fn schedule_digest(id: &str, version: u32, schedule: &[(&str, u64)]) -> String {
    let digest = sha2::Sha256::digest(schedule_preimage(id, version, schedule).as_bytes());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        write!(hex, "{byte:02x}").expect("writing to a String cannot fail");
    }
    hex
}

/// The content-addressed identity of [`CHARGE_SCHEDULE`]: the lowercase-hex SHA-256 of its
/// canonical encoding (see `schedule_preimage`).
///
/// This is **derived**, not declared. A hand-maintained version number with a "remember to
/// bump it when costs move" convention is a rule enforced by discipline, which is to say
/// not enforced: the one time it is forgotten, a consumer's pinned identity silently
/// describes a schedule that no longer exists and every budget they sized against it is
/// wrong. Hashing the table means a schedule change **cannot** keep the old identity —
/// the digest moves whether or not anyone remembers to move it.
///
/// SHA-256 through the `sha2` crate, which this crate already depends on for the SPARQL
/// digest built-ins and which is pure software with no entropy source, so the derivation
/// is `wasm32-unknown-unknown`-clean and reproducible on every target.
///
/// It is a lazily-initialized `static` rather than a `const` because SHA-256 is not
/// const-evaluable; pinning a literal instead would reintroduce exactly the
/// discipline-enforced identity this value exists to eliminate.
pub static GOVERNOR_PROFILE_DIGEST: LazyLock<String> = LazyLock::new(|| {
    schedule_digest(
        GOVERNOR_PROFILE_ID,
        GOVERNOR_PROFILE_VERSION,
        &CHARGE_SCHEDULE,
    )
});

/// The content-addressed identity of this profile's normative vector corpus.
///
/// The SHA-256 of the corpus freeze manifest, which in turn covers every payload byte of
/// the corpus. Defining it over the manifest rather than over a bespoke traversal means a
/// consumer can reproduce it with one `sha256sum` and without running any of this crate's
/// code — a digest only its author can compute is not one anybody can independently check.
///
/// While the profile has no frozen corpus, this is the SHA-256 of the empty byte string:
/// the honest content-address of an empty manifest, and a well-formed value a consumer can
/// pin, compare, and reproduce today. It is re-pinned by the same freeze step that writes
/// the corpus manifest, and it moves the moment the corpus has any content at all.
pub const GOVERNOR_CORPUS_DIGEST: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    /// Compile-time proof that the state can be shared across evaluation workers, which
    /// the evaluation context's own `Send + Sync` proof requires of everything it holds.
    const _: fn() = || {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<GovernorState>();
        assert_send_sync::<QueryGovernors>();
        assert_send_sync::<WallDeadline>();
        assert_send_sync::<CancellationFlag>();
        fn assert_send<T: Send>() {}
        assert_send::<ItemCharge>();
    };

    /// A signal that has already latched to `cause`, for tests that need a deterministic
    /// stop without a clock.
    #[derive(Debug)]
    struct LatchedSignal(StopCause);

    impl StopSignal for LatchedSignal {
        fn poll(&self) -> Option<StopCause> {
            Some(self.0)
        }
    }

    #[test]
    fn wall_deadline_latches_once_tripped() {
        let clock = ScriptedClock::new(1_000.0);
        let deadline = WallDeadline::scripted(&clock, Duration::from_millis(50));

        assert_eq!(deadline.poll(), None, "inside the budget");
        assert!(!deadline.has_expired());

        clock.set(1_050.0);
        assert_eq!(
            deadline.poll(),
            Some(StopCause::Deadline),
            "the deadline is inclusive: reaching it expires it"
        );
        assert!(deadline.has_expired());

        // Winding the clock back inside the budget must not un-trip the deadline: a
        // governor that can un-trip would let a query resume past its deadline.
        clock.set(1_001.0);
        assert_eq!(deadline.poll(), Some(StopCause::Deadline));
        clock.set(1_000.0);
        assert_eq!(deadline.poll(), Some(StopCause::Deadline));
    }

    #[test]
    fn wall_deadline_treats_a_backwards_clock_step_as_expired() {
        let clock = ScriptedClock::new(10_000.0);
        let deadline = WallDeadline::scripted(&clock, Duration::from_secs(30));

        assert_eq!(deadline.poll(), None, "thirty seconds of budget remain");

        // A wall clock is steppable. A backwards step would make a naive
        // `now >= deadline` test un-trippable for far longer than the caller asked for,
        // so an observed rewind is treated as expiry.
        clock.set(9_999.0);
        assert_eq!(deadline.poll(), Some(StopCause::Deadline));
        assert!(deadline.has_expired());

        // And it stays expired once the clock is stepped forward again.
        clock.set(10_001.0);
        assert_eq!(deadline.poll(), Some(StopCause::Deadline));
    }

    #[test]
    fn wall_deadline_over_the_host_clock_expires_on_a_zero_budget() {
        let deadline = WallDeadline::after(Duration::ZERO);
        assert_eq!(
            deadline.poll(),
            Some(StopCause::Deadline),
            "zero is a valid, immediately-expiring budget"
        );
    }

    #[test]
    fn cancellation_flag_latches() {
        let flag = CancellationFlag::new();
        assert!(!flag.is_cancelled());
        assert_eq!(flag.poll(), None);

        let handle = flag.clone();
        handle.cancel();

        assert!(flag.is_cancelled(), "clones share one flag");
        assert_eq!(flag.poll(), Some(StopCause::Cancelled));
        // Latching: repeated polls report the same cause forever.
        assert_eq!(flag.poll(), Some(StopCause::Cancelled));
        assert_eq!(handle.poll(), Some(StopCause::Cancelled));
    }

    #[test]
    fn unbounded_still_bounds_udf_depth() {
        let governors = QueryGovernors::UNBOUNDED;
        assert_eq!(
            governors.limits().get(ResourceDimension::UdfDepth),
            u64::from(MAX_UDF_DEPTH),
            "the recursion guard is a fixed build ceiling, not an opt-in"
        );
        assert!(
            governors.is_engaged_in(ResourceDimension::UdfDepth),
            "the recursion guard is enforced on every execution"
        );

        // There is no caller-facing setter that could raise or remove it, so the ceiling
        // survives every builder.
        let configured = QueryGovernors::UNBOUNDED
            .with_fuel(1)
            .with_max_answers(2)
            .with_max_intermediate_cells(3)
            .with_max_scratch_bytes(4)
            .with_max_remote_requests(5)
            .with_stop_signal(Arc::new(CancellationFlag::new()));
        assert_eq!(
            configured.limits().get(ResourceDimension::UdfDepth),
            u64::from(MAX_UDF_DEPTH)
        );

        // And it is actually enforced by the live state.
        let state = GovernorState::new(&QueryGovernors::UNBOUNDED);
        assert_eq!(
            state.charge(ResourceDimension::UdfDepth, u64::from(MAX_UDF_DEPTH)),
            Ok(())
        );
        assert_eq!(
            state.charge(ResourceDimension::UdfDepth, 1),
            Err(TrippedGovernor::Budget {
                dimension: ResourceDimension::UdfDepth,
                limit: u64::from(MAX_UDF_DEPTH),
                consumed: u64::from(MAX_UDF_DEPTH) + 1,
            })
        );
    }

    #[test]
    fn unbounded_is_not_engaged_on_any_caller_settable_dimension() {
        let governors = QueryGovernors::UNBOUNDED;
        assert!(!governors.is_engaged());
        for dimension in CALLER_SETTABLE_DIMENSIONS {
            assert!(
                !governors.is_engaged_in(dimension),
                "{} must cost an ungoverned query nothing",
                dimension.label()
            );
        }
        // The paged tier's dimensions are configured through its own limits type, so this
        // vector leaves them unbounded and no charge site here enforces them.
        assert!(!governors.is_engaged_in(ResourceDimension::Pages));
        assert!(!governors.is_engaged_in(ResourceDimension::Bytes));

        // A deadline-only configuration engages the execution without engaging fuel, so a
        // fuel charge site stays short-circuited.
        let deadline_only =
            QueryGovernors::UNBOUNDED.with_stop_signal(Arc::new(CancellationFlag::new()));
        assert!(deadline_only.is_engaged());
        assert!(!deadline_only.is_engaged_in(ResourceDimension::Fuel));

        let fuel_only = QueryGovernors::UNBOUNDED.with_fuel(10);
        assert!(fuel_only.is_engaged());
        assert!(fuel_only.is_engaged_in(ResourceDimension::Fuel));
        assert!(!fuel_only.is_engaged_in(ResourceDimension::AnswerRows));
    }

    #[test]
    fn zero_budget_trips_immediately() {
        for dimension in CALLER_SETTABLE_DIMENSIONS {
            let governors = match dimension {
                ResourceDimension::Fuel => QueryGovernors::UNBOUNDED.with_fuel(0),
                ResourceDimension::AnswerRows => QueryGovernors::UNBOUNDED.with_max_answers(0),
                ResourceDimension::IntermediateCells => {
                    QueryGovernors::UNBOUNDED.with_max_intermediate_cells(0)
                }
                ResourceDimension::ScratchBytes => {
                    QueryGovernors::UNBOUNDED.with_max_scratch_bytes(0)
                }
                _ => QueryGovernors::UNBOUNDED.with_max_remote_requests(0),
            };
            let state = GovernorState::new(&governors);
            assert_eq!(
                state.charge(dimension, 1),
                Err(TrippedGovernor::Budget {
                    dimension,
                    limit: 0,
                    consumed: 1,
                }),
                "a zero ceiling on {} is valid and admits no charged work",
                dimension.label()
            );
            assert_eq!(state.tripped(), state.evidence().tripped());
            assert!(state.should_abandon());
        }
    }

    #[test]
    fn inclusive_ceiling_admits_consumption_equal_to_the_limit() {
        let state = GovernorState::new(&QueryGovernors::UNBOUNDED.with_fuel(10));

        assert_eq!(state.charge(ResourceDimension::Fuel, 6), Ok(()));
        assert_eq!(
            state.charge(ResourceDimension::Fuel, 4),
            Ok(()),
            "consumption equal to the ceiling is admitted"
        );
        assert_eq!(state.consumed_in(ResourceDimension::Fuel), 10);
        assert_eq!(state.tripped(), None);

        assert_eq!(
            state.charge(ResourceDimension::Fuel, 1),
            Err(TrippedGovernor::Budget {
                dimension: ResourceDimension::Fuel,
                limit: 10,
                consumed: 11,
            }),
            "only exceeding the ceiling trips"
        );

        // The trip is sticky: a later charge reports the same governor.
        assert_eq!(
            state.charge(ResourceDimension::AnswerRows, 1),
            Err(TrippedGovernor::Budget {
                dimension: ResourceDimension::Fuel,
                limit: 10,
                consumed: 11,
            })
        );

        let evidence = state.evidence();
        assert_eq!(evidence.consumed_in(ResourceDimension::Fuel), 11);
        assert_eq!(evidence.limit_for(ResourceDimension::Fuel), 10);
        assert!(!evidence.is_complete());
    }

    #[test]
    fn charging_saturates_instead_of_overflowing() {
        let state = GovernorState::new(&QueryGovernors::UNBOUNDED);
        assert_eq!(state.charge(ResourceDimension::Fuel, u64::MAX), Ok(()));
        assert_eq!(
            state.charge(ResourceDimension::Fuel, u64::MAX),
            Ok(()),
            "an unbounded dimension can never trip, and must never panic"
        );
        assert_eq!(state.consumed_in(ResourceDimension::Fuel), u64::MAX);
    }

    #[test]
    fn peak_dimension_uses_maximum_not_running_sum() {
        let state = GovernorState::new(&QueryGovernors::UNBOUNDED.with_max_intermediate_cells(10));

        assert_eq!(
            state.observe_peak(ResourceDimension::IntermediateCells, 6),
            Ok(())
        );
        assert_eq!(
            state.observe_peak(ResourceDimension::IntermediateCells, 7),
            Ok(()),
            "6 + 7 would exceed the ceiling, but the maximum single observation is 7"
        );
        assert_eq!(state.consumed_in(ResourceDimension::IntermediateCells), 7);
        assert_eq!(
            state.observe_peak(ResourceDimension::IntermediateCells, 10),
            Ok(()),
            "an observation equal to the ceiling is admitted"
        );
        assert_eq!(state.consumed_in(ResourceDimension::IntermediateCells), 10);

        assert_eq!(
            state.observe_peak(ResourceDimension::IntermediateCells, 11),
            Err(TrippedGovernor::Budget {
                dimension: ResourceDimension::IntermediateCells,
                limit: 10,
                consumed: 11,
            })
        );
        assert_eq!(
            state
                .evidence()
                .consumed_in(ResourceDimension::IntermediateCells),
            11
        );
    }

    #[test]
    fn commit_ordered_items_returns_the_same_index_regardless_of_worker_completion_order() {
        // Six input items of four fuel each, against a ceiling of eleven: the running
        // total crosses at item index 2 (4, 8, 12). Each item also committed one output
        // row, so the certified prefix is the two rows produced before the crossing.
        let item_fuel = [4_u64; 6];
        let expected = Some((
            2,
            2,
            TrippedGovernor::Budget {
                dimension: ResourceDimension::Fuel,
                limit: 11,
                consumed: 12,
            },
        ));

        let build = |order: &[usize]| {
            // Fill the indexed slots in the given "completion order", exactly as a
            // work-stealing pool would: the slot index is the item's position in the
            // source, never the order the worker holding it finished in.
            let mut ledger = vec![ItemCharge::default(); item_fuel.len()];
            for &slot in order {
                ledger[slot] = ItemCharge {
                    fuel: item_fuel[slot],
                    committed: 1,
                };
            }
            ledger
        };

        let in_order: Vec<usize> = (0..item_fuel.len()).collect();
        let reversed: Vec<usize> = (0..item_fuel.len()).rev().collect();
        let interleaved = vec![3, 0, 5, 2, 4, 1];

        for order in [&in_order, &reversed, &interleaved] {
            let state = GovernorState::new(&QueryGovernors::UNBOUNDED.with_fuel(11));
            assert_eq!(
                state.commit_ordered_items(&build(order)),
                expected,
                "the trip point is a function of the item index, not of completion order"
            );
            // Items after the crossing are not folded in, so the reported cost reflects
            // exactly the prefix that was certified.
            assert_eq!(state.consumed_in(ResourceDimension::Fuel), 12);
        }

        // Concurrently produced records reach the same answer: production order is
        // irrelevant because the fold reads the slice by index.
        let state = GovernorState::new(&QueryGovernors::UNBOUNDED.with_fuel(11));
        let produced: Vec<ItemCharge> = std::thread::scope(|scope| {
            let handles: Vec<_> = reversed
                .iter()
                .map(|&slot| scope.spawn(move || (slot, item_fuel[slot])))
                .collect();
            let mut ledger = vec![ItemCharge::default(); item_fuel.len()];
            for handle in handles {
                let (slot, fuel) = handle.join().expect("worker did not panic");
                ledger[slot] = ItemCharge { fuel, committed: 1 };
            }
            ledger
        });
        assert_eq!(state.commit_ordered_items(&produced), expected);
    }

    #[test]
    fn commit_ordered_items_reports_no_crossing_when_the_whole_fold_fits() {
        let state = GovernorState::new(&QueryGovernors::UNBOUNDED.with_fuel(12));
        let ledger: Vec<ItemCharge> = (0..3)
            .map(|_| ItemCharge {
                fuel: 4,
                committed: 2,
            })
            .collect();

        assert_eq!(state.commit_ordered_items(&ledger), None);
        assert_eq!(state.consumed_in(ResourceDimension::Fuel), 12);
        assert!(state.evidence().is_complete());
    }

    #[test]
    fn commit_ordered_items_charges_nothing_beyond_the_certified_prefix() {
        // A ceiling of one, and three items of one fuel each. The first item is admitted
        // (the ceiling is inclusive), the second crosses it — and the third is never
        // charged at all, because its work is not part of what the caller receives.
        let state = GovernorState::new(&QueryGovernors::UNBOUNDED.with_fuel(1));
        let ledger = vec![
            ItemCharge {
                fuel: 1,
                committed: 3,
            },
            ItemCharge {
                fuel: 1,
                committed: 7,
            },
            ItemCharge {
                fuel: 9_000,
                committed: 11,
            },
        ];

        assert_eq!(
            state.commit_ordered_items(&ledger),
            Some((
                1,
                3,
                TrippedGovernor::Budget {
                    dimension: ResourceDimension::Fuel,
                    limit: 1,
                    consumed: 2,
                }
            )),
            "the certified prefix is the rows the items before the crossing committed"
        );
        assert_eq!(
            state.consumed_in(ResourceDimension::Fuel),
            2,
            "the item after the crossing must not be charged"
        );
    }

    #[test]
    fn precedence_prefers_the_stop_signal_over_a_simultaneous_fuel_trip() {
        let governors = QueryGovernors::UNBOUNDED
            .with_fuel(0)
            .with_stop_signal(Arc::new(LatchedSignal(StopCause::Cancelled)));
        let state = GovernorState::new(&governors);

        // Both conditions are true at this charge point: fuel would cross its zero
        // ceiling, and the stop signal has already fired.
        assert_eq!(
            state.charge(ResourceDimension::Fuel, 1),
            Err(TrippedGovernor::Stopped {
                cause: StopCause::Cancelled
            }),
            "the stop signal outranks a simultaneous budget crossing"
        );
        assert_eq!(
            state.evidence().tripped(),
            Some(TrippedGovernor::Stopped {
                cause: StopCause::Cancelled
            })
        );

        // A deadline loses to a cancellation, and both outrank every budget.
        let deadline_state = GovernorState::new(
            &QueryGovernors::UNBOUNDED
                .with_fuel(0)
                .with_stop_signal(Arc::new(LatchedSignal(StopCause::Deadline))),
        );
        assert_eq!(
            deadline_state.charge(ResourceDimension::Fuel, 1),
            Err(TrippedGovernor::Stopped {
                cause: StopCause::Deadline
            })
        );

        assert_eq!(
            resolve_precedence([
                TrippedGovernor::Stopped {
                    cause: StopCause::Deadline
                },
                TrippedGovernor::Stopped {
                    cause: StopCause::Cancelled
                },
            ]),
            Some(TrippedGovernor::Stopped {
                cause: StopCause::Cancelled
            })
        );
        assert_eq!(resolve_precedence([]), None);
    }

    #[test]
    fn resolution_follows_the_kernel_precedence_order() {
        // The order itself is pinned beside the type in the kernel vocabulary; what is
        // this crate's business is that resolution actually honours it, and that an empty
        // candidate set is not a trip.
        for dimension in ResourceDimension::ALL {
            let budget = TrippedGovernor::Budget {
                dimension,
                limit: 0,
                consumed: 1,
            };
            for cause in [StopCause::Cancelled, StopCause::Deadline] {
                let stopped = TrippedGovernor::Stopped { cause };
                assert_eq!(
                    resolve_precedence([budget, stopped]),
                    Some(stopped),
                    "{} must lose to {}",
                    dimension.label(),
                    cause.label()
                );
                assert_eq!(
                    resolve_precedence([stopped, budget]),
                    Some(stopped),
                    "resolution must not depend on the order candidates are supplied in"
                );
            }
        }
        assert_eq!(resolve_precedence([]), None);

        // Budget against budget, too: the allocation ceiling defends the failure mode
        // there is no recovering from, so it outranks every other ceiling when several
        // are true at one charge point.
        let cells = TrippedGovernor::Budget {
            dimension: ResourceDimension::IntermediateCells,
            limit: 1,
            consumed: 9,
        };
        for dimension in ResourceDimension::ALL {
            if dimension == ResourceDimension::IntermediateCells {
                continue;
            }
            let other = TrippedGovernor::Budget {
                dimension,
                limit: 1,
                consumed: 9,
            };
            assert_eq!(resolve_precedence([other, cells]), Some(cells));
            assert_eq!(resolve_precedence([cells, other]), Some(cells));
        }
    }

    #[test]
    fn the_stop_signal_is_polled_on_the_fuel_interval() {
        let flag = CancellationFlag::new();
        let state =
            GovernorState::new(&QueryGovernors::UNBOUNDED.with_stop_signal(Arc::new(flag.clone())));

        // Fuel is unbounded, so nothing but the signal can stop this execution.
        for _ in 0..STOP_POLL_FUEL {
            assert_eq!(state.charge(ResourceDimension::Fuel, 1), Ok(()));
        }
        assert_eq!(state.tripped(), None);

        flag.cancel();
        let mut charged = 0_u64;
        let tripped = loop {
            match state.charge(ResourceDimension::Fuel, 1) {
                Ok(()) => charged += 1,
                Err(tripped) => break tripped,
            }
            assert!(
                charged <= STOP_POLL_FUEL,
                "a latched signal must be observed within one poll interval"
            );
        };
        assert_eq!(
            tripped,
            TrippedGovernor::Stopped {
                cause: StopCause::Cancelled
            }
        );
        assert!(state.should_abandon());
    }

    #[test]
    fn an_unpolled_stop_signal_costs_an_ungoverned_execution_nothing() {
        // No signal attached: the fuel interval check never reaches a poll, and an
        // unbounded execution never trips however much it charges.
        let state = GovernorState::new(&QueryGovernors::UNBOUNDED);
        for _ in 0..(STOP_POLL_FUEL * 2) {
            assert_eq!(state.charge(ResourceDimension::Fuel, 1), Ok(()));
        }
        assert_eq!(state.poll_stop(), None);
        assert!(!state.should_abandon());
        assert!(state.evidence().is_complete());
        assert_eq!(
            state.evidence().consumed_in(ResourceDimension::Fuel),
            STOP_POLL_FUEL * 2,
            "consumption is reported on the complete path so a caller can size a budget"
        );
    }

    #[test]
    fn poll_stop_latches_the_cause_into_the_evidence() {
        let flag = CancellationFlag::new();
        let state =
            GovernorState::new(&QueryGovernors::UNBOUNDED.with_stop_signal(Arc::new(flag.clone())));
        assert_eq!(state.poll_stop(), None);

        flag.cancel();
        assert_eq!(state.poll_stop(), Some(StopCause::Cancelled));
        assert_eq!(
            state.tripped(),
            Some(TrippedGovernor::Stopped {
                cause: StopCause::Cancelled
            })
        );
        assert_eq!(state.poll_stop(), Some(StopCause::Cancelled));
    }

    #[test]
    fn an_empty_item_charge_records_nothing_and_commits_nothing() {
        // A chunk worker fills its ledger by index, so an item a worker never reached
        // leaves the default record behind. That record must be inert in the fold: it
        // charges nothing and it certifies no rows, so a hole in a ledger can never
        // manufacture budget or manufacture answers.
        let empty = ItemCharge::default();
        assert_eq!(empty.fuel, 0);
        assert_eq!(empty.committed, 0);

        let state = GovernorState::new(&QueryGovernors::UNBOUNDED.with_fuel(0));
        assert_eq!(
            state.commit_ordered_items(&[empty, empty]),
            None,
            "a zero charge cannot cross even a zero ceiling, which is inclusive"
        );
        assert_eq!(state.consumed_in(ResourceDimension::Fuel), 0);
    }

    #[test]
    fn profile_digest_changes_when_the_charge_schedule_changes() {
        let pinned = schedule_digest(
            GOVERNOR_PROFILE_ID,
            GOVERNOR_PROFILE_VERSION,
            &CHARGE_SCHEDULE,
        );
        assert_eq!(
            *GOVERNOR_PROFILE_DIGEST, pinned,
            "the published digest is derived from the shipped table"
        );
        assert_eq!(pinned.len(), 64, "lowercase-hex SHA-256");
        assert!(
            pinned
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        );

        // A changed cost cannot keep the old identity.
        let mut recosted = CHARGE_SCHEDULE;
        recosted[0].1 += 1;
        assert_ne!(
            schedule_digest(GOVERNOR_PROFILE_ID, GOVERNOR_PROFILE_VERSION, &recosted),
            pinned
        );

        // Nor can a renamed charge point.
        let mut relabelled = CHARGE_SCHEDULE;
        relabelled[3].0 = "path-frontier-expansion-v2";
        assert_ne!(
            schedule_digest(GOVERNOR_PROFILE_ID, GOVERNOR_PROFILE_VERSION, &relabelled),
            pinned
        );

        // Nor can a reordered table, a dropped entry, or an added one.
        let mut reordered = CHARGE_SCHEDULE;
        reordered.swap(0, 1);
        assert_ne!(
            schedule_digest(GOVERNOR_PROFILE_ID, GOVERNOR_PROFILE_VERSION, &reordered),
            pinned
        );
        assert_ne!(
            schedule_digest(
                GOVERNOR_PROFILE_ID,
                GOVERNOR_PROFILE_VERSION,
                &CHARGE_SCHEDULE[..CHARGE_SCHEDULE.len() - 1]
            ),
            pinned
        );

        // And the identity is bound to the profile name and version too.
        assert_ne!(
            schedule_digest(
                "purrdf-other-governors",
                GOVERNOR_PROFILE_VERSION,
                &CHARGE_SCHEDULE
            ),
            pinned
        );
        assert_ne!(
            schedule_digest(
                GOVERNOR_PROFILE_ID,
                GOVERNOR_PROFILE_VERSION + 1,
                &CHARGE_SCHEDULE
            ),
            pinned
        );
    }

    #[test]
    fn the_charge_schedule_is_the_single_source_of_labels_and_costs() {
        assert_eq!(ChargePoint::ALL.len(), CHARGE_SCHEDULE.len());
        for (point, &(label, cost)) in ChargePoint::ALL.iter().zip(CHARGE_SCHEDULE.iter()) {
            assert_eq!(point.label(), label);
            assert_eq!(point.cost(), cost);
            assert_eq!(point.to_string(), label);
        }

        let mut labels: Vec<&str> = CHARGE_SCHEDULE.iter().map(|&(label, _)| label).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(
            labels.len(),
            CHARGE_SCHEDULE.len(),
            "duplicate labels would make the ledger ambiguous"
        );
        for label in labels {
            assert!(
                !label.contains('\t') && !label.contains('\n'),
                "a label containing an encoding separator would make the digest ambiguous"
            );
        }
    }

    #[test]
    fn charging_a_schedule_point_spends_its_pinned_cost() {
        let state = GovernorState::new(&QueryGovernors::UNBOUNDED.with_fuel(2));
        assert_eq!(state.charge_point(ChargePoint::AlgebraNodeEntry), Ok(()));
        assert_eq!(state.charge_point(ChargePoint::CommittedOutputRow), Ok(()));
        assert_eq!(state.consumed_in(ResourceDimension::Fuel), 2);
        assert_eq!(
            state.charge_point(ChargePoint::BgpCandidateQuad),
            Err(TrippedGovernor::Budget {
                dimension: ResourceDimension::Fuel,
                limit: 2,
                consumed: 3,
            })
        );
    }

    #[test]
    fn the_corpus_digest_is_a_well_formed_reproducible_sha256() {
        assert_eq!(GOVERNOR_CORPUS_DIGEST.len(), 64);
        assert!(
            GOVERNOR_CORPUS_DIGEST
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        );

        let empty = sha2::Sha256::digest([]);
        let mut hex = String::new();
        for byte in empty {
            use std::fmt::Write as _;
            write!(hex, "{byte:02x}").expect("writing to a String cannot fail");
        }
        assert_eq!(
            GOVERNOR_CORPUS_DIGEST, hex,
            "with no frozen corpus the digest is the content-address of an empty manifest"
        );
    }

    #[test]
    fn the_stop_poll_interval_shares_no_factor_with_a_natural_ceiling() {
        assert_eq!(STOP_POLL_FUEL, 4093);
        assert!(!STOP_POLL_FUEL.is_power_of_two());
        assert!(
            (2..STOP_POLL_FUEL).all(|factor| {
                factor * factor > STOP_POLL_FUEL || !STOP_POLL_FUEL.is_multiple_of(factor)
            }),
            "a prime interval cannot divide, or be divided by, a caller's round ceiling"
        );
        for ceiling in [10_u64, 100, 1_000, 1_000_000, 1_024, 4_096, 65_536] {
            assert_ne!(ceiling % STOP_POLL_FUEL, 0);
            assert_ne!(STOP_POLL_FUEL % ceiling, 0);
        }
    }

    #[test]
    fn every_dimension_has_its_own_counter_slot() {
        for (expected, dimension) in ResourceDimension::ALL.into_iter().enumerate() {
            assert_eq!(
                GovernorState::slot(dimension),
                expected,
                "the counter slot of {} must track its position in the kernel vocabulary",
                dimension.label()
            );
        }

        // Charging one dimension must not move another's counter.
        let state = GovernorState::new(&QueryGovernors::UNBOUNDED);
        state
            .charge(ResourceDimension::ScratchBytes, 7)
            .expect("unbounded");
        for dimension in ResourceDimension::ALL {
            let expected = u64::from(dimension == ResourceDimension::ScratchBytes) * 7;
            assert_eq!(
                state.consumed_in(dimension),
                expected,
                "{} was disturbed by a charge on another dimension",
                dimension.label()
            );
        }
    }

    #[test]
    fn state_is_built_fresh_per_execution_and_never_carries_consumption_over() {
        let governors = QueryGovernors::UNBOUNDED.with_fuel(4);

        let first = GovernorState::new(&governors);
        assert_eq!(first.charge(ResourceDimension::Fuel, 4), Ok(()));
        assert_eq!(first.consumed_in(ResourceDimension::Fuel), 4);

        let second = GovernorState::new(&governors);
        assert_eq!(
            second.consumed_in(ResourceDimension::Fuel),
            0,
            "a fresh state starts a fresh budget"
        );
        assert_eq!(second.charge(ResourceDimension::Fuel, 4), Ok(()));
        assert_eq!(second.tripped(), None);
    }
}
