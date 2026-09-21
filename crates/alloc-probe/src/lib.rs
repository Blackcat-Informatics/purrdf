// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! One counting allocator for the whole workspace, and the two measurement
//! windows that read it.
//!
//! Every allocation claim in this repository — "iteration allocates nothing",
//! "the fused frontier's peak is bounded by the profile", "this validation phase
//! costs N allocations" — is operational: a `#[global_allocator]` counts, a
//! window brackets the measured region, and the numbers come out the other side.
//! What each of those claims needed was the same instrument, so it lives here
//! once instead of being re-derived per test file, where every copy was free to
//! drift in what it counted and in which threads it counted on.
//!
//! # The allocator is installed by the binary, not by this crate
//!
//! `#[global_allocator]` may be declared exactly once per binary, so this crate
//! cannot declare it for you. It supplies the type; each test, bench or example
//! registers it:
//!
//! ```ignore
//! #[global_allocator]
//! static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;
//! ```
//!
//! # The two windows measure different quantities — pick by situation
//!
//! [`CurrentThreadWindow`] and [`WholeProcessWindow`] are not two flavours of
//! one measurement. They answer different questions, and the wrong one does not
//! fail loudly: it returns a plausible number that means nothing.
//!
//! * [`CurrentThreadWindow`] counts only what the thread that opened it
//!   allocated. Use it in `cargo test`, where the harness runs a binary's test
//!   functions **concurrently** over one global allocator: a process-wide
//!   counter there is contaminated by whatever a sibling test happened to be
//!   doing between the two snapshots, so the same assertion passes or fails
//!   depending on scheduling. Its hazard is the mirror image — if the measured
//!   code fans out over `rayon`, the worker threads' allocations happen
//!   somewhere this window cannot see, and a parallel phase is reported as
//!   nearly free.
//! * [`WholeProcessWindow`] counts every thread. Use it when the measured code
//!   is parallel — SHACL validation above its focus-node threshold, for
//!   instance, which is precisely the regime whose allocation behaviour is worth
//!   measuring. Its hazard is the one above in reverse: opened inside a
//!   `cargo test` binary it will happily attribute a sibling test's traffic to
//!   the region under measurement.
//!
//! # Four quantities, kept apart
//!
//! [`Measurement`] reports allocation **count**, **requested bytes**, **retained
//! bytes** and **peak working bytes** separately, and there is deliberately no
//! blended figure. A phase that allocates and frees one large buffer repeatedly
//! has high traffic and a modest peak; a phase that assembles one large
//! structure has the reverse. Collapsing the four would erase exactly the
//! distinction that makes an allocation report worth reading.
//!
//! # What "counted" means, precisely
//!
//! * `alloc` and `realloc` each record **one** allocation, on success only.
//!   `alloc_zeroed` is the default `GlobalAlloc` implementation, which calls
//!   `alloc`, so it is counted there.
//! * `realloc` adds its **new** size to the requested-byte traffic, and moves
//!   live bytes by the difference between the new and old layouts.
//! * `dealloc` and the shrinking half of `realloc` move live bytes down. They
//!   never reduce the allocation count or the requested-byte traffic, which are
//!   monotone within a window by construction.
//! * Live bytes are layout sizes, not what the system allocator actually mapped.
//!   The figures are the program's demand, which is the quantity a code change
//!   moves; they are not an RSS measurement.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};

/// What one measurement window observed, in four quantities that are never
/// combined into one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Measurement {
    /// How many successful `alloc`/`realloc` calls happened inside the window.
    pub allocations: u64,
    /// Total bytes requested through the allocator inside the window, including
    /// memory that was freed again before the window closed. This is allocator
    /// *traffic*, not a footprint.
    pub requested_bytes: u64,
    /// The change in live bytes across the window: what the measured region's
    /// result is still holding when the window closes.
    ///
    /// Signed, and legitimately negative for a region that frees more than it
    /// allocates (a `WholeProcessWindow` sees only the frees that occur inside
    /// it, so releasing a structure built before the window opened reads as a
    /// negative retention).
    ///
    /// # Reading this from a [`CurrentThreadWindow`]: it is a per-thread
    /// ledger, not process-wide ownership accounting
    ///
    /// When this [`Measurement`] came from a [`CurrentThreadWindow`], this
    /// field is the OPENING THREAD's live-byte delta — `alloc`/`dealloc`
    /// calls made ON THAT THREAD, nothing else. It does not track which
    /// thread logically *owns* a buffer, only which thread's ledger recorded
    /// the call. A buffer allocated on the window's thread and later freed by
    /// a different thread (a `rayon` worker, a spawned thread the caller
    /// handed it to) never records a `dealloc` on the opening thread's
    /// ledger, so it reads here as still retained even though it has, in
    /// fact, been freed. The mirror case under-reports: a buffer allocated on
    /// another thread and freed on the window's thread lowers this thread's
    /// live-byte count (and can drive it negative) for a buffer this window
    /// never saw allocated. Both are the ledger doing exactly what it is
    /// documented to do — count this thread's calls — not a bug; the bug
    /// would be trusting this figure as ownership accounting when allocation
    /// and deallocation of the measured region's results can cross threads.
    /// When ownership crosses threads, use [`WholeProcessWindow`] instead: it
    /// reads one ledger that every thread's `alloc`/`dealloc` calls update,
    /// so a cross-thread free is still counted against the region that freed
    /// it.
    pub retained_bytes: i64,
    /// The largest working-set span observed during the window: the
    /// high-water mark of live bytes minus the low-water mark, both measured
    /// relative to the live-byte level the window held when it opened.
    ///
    /// Both marks are anchored at the value live bytes held at open, so the
    /// low-water mark can never rise above that anchor and the high-water
    /// mark can never fall below it — the span is therefore non-negative by
    /// construction. In the common case, where nothing allocated before the
    /// window is freed inside it, the low-water mark never drops below the
    /// anchor and this is exactly the old high-water-mark-relative-to-open
    /// figure: a previously reported number does not move. The only case
    /// where it changes is the one that used to be wrong — a window (chiefly
    /// [`WholeProcessWindow`]) freeing memory that was allocated before it
    /// opened, which used to pull live below the anchor and depress the
    /// reported peak below the measured region's true working set. Fixing
    /// that can only move this figure upward, never down.
    ///
    /// From a [`CurrentThreadWindow`], both marks derive from the same
    /// per-thread live-byte cell as [`Measurement::retained_bytes`], so this
    /// span carries the identical cross-thread-ownership caveat documented
    /// there: it is the opening thread's high-water span, not the measured
    /// region's, when the region's allocations and deallocations do not all
    /// happen on that one thread.
    pub peak_working_bytes: i64,
}

/// Widen a layout size, saturating rather than wrapping on a size no allocator
/// could ever have served.
fn widen_u64(size: usize) -> u64 {
    u64::try_from(size).unwrap_or(u64::MAX)
}

/// The signed widening of [`widen_u64`], for the live-byte ledger.
fn widen_i64(size: usize) -> i64 {
    i64::try_from(size).unwrap_or(i64::MAX)
}

// ---------------------------------------------------------------------------
// Per-thread accounting
// ---------------------------------------------------------------------------

thread_local! {
    /// Successful allocation calls made on this thread, monotone for its life.
    static THREAD_ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
    /// Bytes requested on this thread, monotone for its life.
    static THREAD_REQUESTED_BYTES: Cell<u64> = const { Cell::new(0) };
    /// Live bytes currently held by allocations made on this thread.
    static THREAD_LIVE_BYTES: Cell<i64> = const { Cell::new(0) };
    /// High-water mark of [`THREAD_LIVE_BYTES`] since the last window opened.
    static THREAD_PEAK_BYTES: Cell<i64> = const { Cell::new(0) };
    /// Low-water mark of [`THREAD_LIVE_BYTES`] since the last window opened.
    static THREAD_TROUGH_BYTES: Cell<i64> = const { Cell::new(0) };
    /// Whether a [`CurrentThreadWindow`] is already open on this thread.
    static THREAD_WINDOW_OPEN: Cell<bool> = const { Cell::new(false) };
}

/// Record `size` bytes handed out on the current thread.
///
/// The per-thread ledger is uncontended `Cell` traffic, so it runs
/// unconditionally rather than behind a gate: a window is a pair of snapshots
/// over counters that were always being kept. `try_with` rather than `with`,
/// because an allocation can happen while this thread's TLS is being
/// initialized or destroyed, and a probe must never be the reason a program
/// aborts.
fn thread_record_allocation(size: usize) {
    let _ = THREAD_ALLOCATIONS.try_with(|count| count.set(count.get().saturating_add(1)));
    let _ = THREAD_REQUESTED_BYTES
        .try_with(|bytes| bytes.set(bytes.get().saturating_add(widen_u64(size))));
    let _ = THREAD_LIVE_BYTES.try_with(|live| {
        let now = live.get().saturating_add(widen_i64(size));
        live.set(now);
        let _ = THREAD_PEAK_BYTES.try_with(|peak| {
            if now > peak.get() {
                peak.set(now);
            }
        });
    });
}

/// Record `size` bytes returned on the current thread.
fn thread_record_deallocation(size: usize) {
    let _ = THREAD_LIVE_BYTES.try_with(|live| {
        let now = live.get().saturating_sub(widen_i64(size));
        live.set(now);
        let _ = THREAD_TROUGH_BYTES.try_with(|trough| {
            if now < trough.get() {
                trough.set(now);
            }
        });
    });
}

// ---------------------------------------------------------------------------
// Whole-process accounting
// ---------------------------------------------------------------------------

/// High bit of [`PROCESS_STATE`]: set while a [`WholeProcessWindow`] is
/// armed.
const PROCESS_ARMED_BIT: u64 = 1 << 63;
/// The remaining bits of [`PROCESS_STATE`]: how many recorders are currently
/// inside the counted region, between their armed check and their counter
/// updates.
const PROCESS_COUNT_MASK: u64 = !PROCESS_ARMED_BIT;

/// Whether the process-wide ledger is armed, and how many recorders are
/// currently between their armed check and their counter updates, packed
/// into one word.
///
/// The two used to be a separate `AtomicBool` for armed and nothing at all
/// tracking in-flight recorders. That let a recorder pass the armed check,
/// get preempted, and only touch the counters after `close()` had already
/// sampled them — or, worse, after a later `open()` had zeroed them and
/// re-armed, in which case the stale recorder's update landed on the NEXT
/// window instead of the one it observed as armed. Packing the flag and the
/// count into one word lets the check and the registration happen as a
/// single compare-exchange (see [`process_enter`]), so a recorder can never
/// register against a window that has already started closing, and closing
/// can wait for exactly the recorders it let in (see
/// [`process_disarm_and_drain`]).
///
/// Unlike the per-thread cells, the ledger this guards is four atomics
/// shared by every thread, and keeping it is not free on a parallel
/// workload. It is therefore armed only for the duration of a
/// [`WholeProcessWindow`], so a bench that measures *time* outside its probe
/// windows pays one `Acquire` load per allocation and nothing else. The load
/// is `Acquire` rather than `Relaxed` because entry has to pair with the
/// `Release` that clears the armed bit in [`process_disarm_and_drain`], or a
/// recorder could still observe "armed" after the bit was already cleared.
static PROCESS_STATE: AtomicU64 = AtomicU64::new(0);
/// Whether a [`WholeProcessWindow`] token is outstanding; the nesting guard.
static PROCESS_WINDOW_HELD: AtomicBool = AtomicBool::new(false);
/// Successful allocation calls on every thread since the window opened.
static PROCESS_ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
/// Bytes requested on every thread since the window opened.
static PROCESS_REQUESTED_BYTES: AtomicU64 = AtomicU64::new(0);
/// Live bytes, relative to the open of the window (which zeroes it).
static PROCESS_LIVE_BYTES: AtomicI64 = AtomicI64::new(0);
/// High-water mark of [`PROCESS_LIVE_BYTES`] since the window opened.
static PROCESS_PEAK_BYTES: AtomicI64 = AtomicI64::new(0);
/// Low-water mark of [`PROCESS_LIVE_BYTES`] since the window opened.
static PROCESS_TROUGH_BYTES: AtomicI64 = AtomicI64::new(0);

/// Try to enter the counted region: atomically check the armed bit and, if
/// set, register one more in-flight recorder in [`PROCESS_STATE`].
///
/// The check and the registration happen in the same compare-exchange on
/// purpose. Two separate steps — load the bit, then increment a count — would
/// leave a window between them wide enough for `close()` to disarm and drain
/// through it: the recorder would have observed "armed" but not yet be
/// counted, `close()` would see zero in-flight and sample, and the recorder
/// would then update counters a later window may already have zeroed. The
/// CAS below closes that: a recorder is either counted before it can be
/// missed, or it never gets past the check at all.
///
/// Returns `false` (a single `Acquire` load, nothing else) when no window is
/// armed, which is the fast path this ledger promises outside a
/// [`WholeProcessWindow`].
fn process_enter() -> bool {
    loop {
        let state = PROCESS_STATE.load(Ordering::Acquire);
        if state & PROCESS_ARMED_BIT == 0 {
            return false;
        }
        if PROCESS_STATE
            .compare_exchange_weak(state, state + 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            return true;
        }
    }
}

/// Leave the counted region entered via [`process_enter`].
///
/// `Release` so that the counter updates the caller just made happen-before
/// [`process_disarm_and_drain`]'s spin observes the decremented count — the
/// half of the pairing that lets a drain trust a zero count as "no recorder
/// is still touching the counters", not just "no recorder is still counted".
fn process_leave() {
    PROCESS_STATE.fetch_sub(1, Ordering::Release);
}

/// Disarm the ledger and wait for every recorder already inside the counted
/// region to leave it.
///
/// Once the armed bit is cleared, [`process_enter`]'s compare-exchange can
/// never succeed again — it only ever succeeds while the bit is still set —
/// so the in-flight count is monotonically non-increasing from this point on
/// and the spin below is guaranteed to terminate. Callers must run this
/// before sampling or zeroing the counters: it is what makes "no window is
/// open" mean "no thread is still updating the counters", not just "no
/// thread will start".
fn process_disarm_and_drain() {
    PROCESS_STATE.fetch_and(!PROCESS_ARMED_BIT, Ordering::AcqRel);
    while PROCESS_STATE.load(Ordering::Acquire) & PROCESS_COUNT_MASK != 0 {
        std::hint::spin_loop();
    }
}

/// Record `size` bytes handed out on any thread, if a window is armed.
fn process_record_allocation(size: usize) {
    if !process_enter() {
        return;
    }
    PROCESS_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    PROCESS_REQUESTED_BYTES.fetch_add(widen_u64(size), Ordering::Relaxed);
    let size = widen_i64(size);
    let live = PROCESS_LIVE_BYTES
        .fetch_add(size, Ordering::Relaxed)
        .saturating_add(size);
    let mut peak = PROCESS_PEAK_BYTES.load(Ordering::Relaxed);
    while live > peak {
        match PROCESS_PEAK_BYTES.compare_exchange_weak(
            peak,
            live,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(observed) => peak = observed,
        }
    }
    process_leave();
}

/// Record `size` bytes returned on any thread, if a window is armed.
fn process_record_deallocation(size: usize) {
    if !process_enter() {
        return;
    }
    let size = widen_i64(size);
    let live = PROCESS_LIVE_BYTES
        .fetch_sub(size, Ordering::Relaxed)
        .saturating_sub(size);
    let mut trough = PROCESS_TROUGH_BYTES.load(Ordering::Relaxed);
    while live < trough {
        match PROCESS_TROUGH_BYTES.compare_exchange_weak(
            trough,
            live,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(observed) => trough = observed,
        }
    }
    process_leave();
}

// ---------------------------------------------------------------------------
// The allocator
// ---------------------------------------------------------------------------

/// A pass-through global allocator that feeds both ledgers.
///
/// Register it in the binary that does the measuring:
///
/// ```ignore
/// #[global_allocator]
/// static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;
/// ```
///
/// One type serves both windows on purpose. A binary has exactly one global
/// allocator, and `crates/shapes/benches/validate.rs` needs both measurements
/// in one process — per-thread figures for its single-threaded import and
/// emission probes, whole-process figures for the `rayon`-parallel validation
/// phases. Two mutually exclusive allocator types could not serve it, and a
/// third combined type would be the optional-component sprawl this workspace
/// refuses. The choice a caller makes is which **window** to open, and the
/// window names say what they measure.
#[derive(Clone, Copy, Debug, Default)]
pub struct CountingAllocator;

// SAFETY: every method delegates to `System` with the caller's exact pointer,
// layout and size; the only added behaviour is counter arithmetic, which owns
// no memory and can never make the returned pointer wrong.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: delegated with the caller's exact layout.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            thread_record_allocation(layout.size());
            process_record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        thread_record_deallocation(layout.size());
        process_record_deallocation(layout.size());
        // SAFETY: delegated with the caller's exact pointer and layout.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: delegated with the caller's exact pointer, layout and size.
        let resized = unsafe { System.realloc(pointer, layout, new_size) };
        if !resized.is_null() {
            thread_record_deallocation(layout.size());
            process_record_deallocation(layout.size());
            thread_record_allocation(new_size);
            process_record_allocation(new_size);
        }
        resized
    }
}

// ---------------------------------------------------------------------------
// The windows
// ---------------------------------------------------------------------------

/// A measurement window over **the allocations made on the thread that opened
/// it**.
///
/// This is the window for `cargo test`. The harness runs a binary's test
/// functions concurrently over one `#[global_allocator]`, so a process-wide
/// counter is contaminated by whatever a sibling test allocated between the two
/// snapshots, and an assertion written over it passes or fails by scheduling
/// luck. A per-thread ledger isolates the measuring thread whatever the harness
/// does.
///
/// # Hazard: it cannot see other threads
///
/// If the measured region fans out — `rayon`, a scoped thread pool, a spawned
/// worker — the allocations those threads make are recorded against *their*
/// ledgers, not this one, and this window will report a parallel phase as
/// almost free. That is a silently wrong number, not an error. When the region
/// under measurement is parallel, [`WholeProcessWindow`] is the only correct
/// choice.
///
/// # Hazard: its byte fields are a per-thread ledger, not ownership accounting
///
/// [`Measurement::retained_bytes`] and [`Measurement::peak_working_bytes`]
/// read this window's OPENING THREAD's live-byte cell — they track which
/// thread's `alloc`/`dealloc` calls the counting allocator recorded, not
/// which thread logically owns the memory. If the measured region hands a
/// buffer it allocated to another thread, and that thread is the one that
/// drops it, no `dealloc` is ever recorded against the opening thread's
/// ledger, so the buffer reads here as still retained after it has actually
/// been freed. The inverse also happens: a buffer allocated elsewhere and
/// freed on this thread lowers this thread's live-byte count for memory this
/// window never saw allocated, which can even drive the reported trough (and
/// so the peak span) below what the opening thread's own work produced. Both
/// readings are the ledger doing exactly what it is documented to do; the
/// mistake would be reading them as a process-wide ownership account when the
/// measured region's allocations and frees do not all happen on the one
/// thread that opened the window. [`WholeProcessWindow`] is the correct
/// choice whenever object ownership can cross threads, precisely because it
/// reads one ledger every thread's calls update, so a cross-thread free is
/// still counted against the region that freed it rather than misattributed
/// to whichever thread happened to hold the token.
///
/// The window is neither `Send` nor `Sync`: the thread that opens it is the
/// thread it measures, and moving the token elsewhere would read a ledger that
/// is not the one it opened over.
#[derive(Debug)]
pub struct CurrentThreadWindow {
    /// The allocation count on this thread when the window opened.
    allocations: u64,
    /// The requested-byte total on this thread when the window opened.
    requested_bytes: u64,
    /// The live-byte level on this thread when the window opened; both the
    /// retained and the peak figures are stated relative to it.
    live_bytes: i64,
    /// Pins the token to its thread, and to the scope that opened it.
    thread_bound: PhantomData<*const ()>,
}

impl CurrentThreadWindow {
    /// Open a window over the current thread's allocations.
    ///
    /// Allocates nothing itself, which is what lets a zero-allocation claim be
    /// measured with it.
    ///
    /// # Panics
    ///
    /// If a window is already open on this thread. Nesting is a programming
    /// error rather than a supported mode: the inner window would re-pin the
    /// shared high-water mark and the outer one would then report a peak it
    /// never saw.
    pub fn open() -> Self {
        let already_open = THREAD_WINDOW_OPEN.with(|open| open.replace(true));
        assert!(
            !already_open,
            "a CurrentThreadWindow is already open on this thread"
        );
        let live_bytes = THREAD_LIVE_BYTES.with(Cell::get);
        THREAD_PEAK_BYTES.with(|peak| peak.set(live_bytes));
        THREAD_TROUGH_BYTES.with(|trough| trough.set(live_bytes));
        Self {
            allocations: THREAD_ALLOCATIONS.with(Cell::get),
            requested_bytes: THREAD_REQUESTED_BYTES.with(Cell::get),
            live_bytes,
            thread_bound: PhantomData,
        }
    }

    /// Read the window without closing it.
    pub fn sample(&self) -> Measurement {
        let live = THREAD_LIVE_BYTES.with(Cell::get);
        let peak = THREAD_PEAK_BYTES.with(Cell::get);
        let trough = THREAD_TROUGH_BYTES.with(Cell::get);
        Measurement {
            allocations: THREAD_ALLOCATIONS
                .with(Cell::get)
                .saturating_sub(self.allocations),
            requested_bytes: THREAD_REQUESTED_BYTES
                .with(Cell::get)
                .saturating_sub(self.requested_bytes),
            retained_bytes: live.saturating_sub(self.live_bytes),
            peak_working_bytes: peak.saturating_sub(trough),
        }
    }

    /// Close the window, returning what it observed.
    ///
    /// The measurement is taken before the token drops, so it describes the
    /// region the window bracketed and not whatever the caller does next.
    pub fn close(self) -> Measurement {
        self.sample()
    }
}

impl Drop for CurrentThreadWindow {
    fn drop(&mut self) {
        // Releasing the nesting guard on drop rather than only in `close` is
        // what makes an early return or a panicking assertion inside the
        // measured region leave the thread usable for the next window.
        let _ = THREAD_WINDOW_OPEN.try_with(|open| open.set(false));
    }
}

/// A measurement window over **the allocations made on every thread** while it
/// is open.
///
/// This is the window for parallel code. SHACL validation, for one, fans focus
/// nodes out over `rayon` above a threshold, so the allocations worth measuring
/// happen on worker threads; a [`CurrentThreadWindow`] would miss them entirely
/// and report the parallel path as the cheapest one in the run.
///
/// While no window is open the process-wide ledger is not kept at all, so a
/// timed benchmark that opens a window only around its probe pays one
/// `Acquire` load per allocation the rest of the time.
///
/// # Hazard: it cannot separate the measured region from the rest of the process
///
/// Anything else running concurrently is counted. In a `cargo test` binary,
/// where the harness runs test functions concurrently over one allocator, that
/// means a sibling test's traffic lands in this measurement — and the resulting
/// figure changes run to run for reasons that have nothing to do with the code
/// under test. In a test binary, [`CurrentThreadWindow`] is the correct choice
/// unless the measured region is itself parallel, in which case the test must
/// also be the only thing running.
#[derive(Debug)]
pub struct WholeProcessWindow(());

impl WholeProcessWindow {
    /// Open a window over the whole process, zeroing the ledger and arming it.
    ///
    /// # Panics
    ///
    /// If a window is already open. The process-wide ledger has exactly one
    /// set of counters, so two live windows would each report the union of both
    /// regions while appearing to report their own.
    pub fn open() -> Self {
        let already_held = PROCESS_WINDOW_HELD.swap(true, Ordering::AcqRel);
        assert!(!already_held, "a WholeProcessWindow is already open");
        PROCESS_ALLOCATIONS.store(0, Ordering::Relaxed);
        PROCESS_REQUESTED_BYTES.store(0, Ordering::Relaxed);
        PROCESS_LIVE_BYTES.store(0, Ordering::Relaxed);
        PROCESS_PEAK_BYTES.store(0, Ordering::Relaxed);
        PROCESS_TROUGH_BYTES.store(0, Ordering::Relaxed);
        // Release: every thread that observes the armed bit must also observe
        // the zeroed counters, or it would add to a total from the last
        // window. The in-flight count starts at zero here because the
        // previous window's `Drop` ran `process_disarm_and_drain` — which
        // waits for the count to reach zero — before it released
        // `PROCESS_WINDOW_HELD`, and `already_held` above proves this `open`
        // could not have run until that release happened.
        PROCESS_STATE.store(PROCESS_ARMED_BIT, Ordering::Release);
        Self(())
    }

    /// Read the window without closing it.
    pub fn sample(&self) -> Measurement {
        Measurement {
            allocations: PROCESS_ALLOCATIONS.load(Ordering::Relaxed),
            requested_bytes: PROCESS_REQUESTED_BYTES.load(Ordering::Relaxed),
            retained_bytes: PROCESS_LIVE_BYTES.load(Ordering::Relaxed),
            peak_working_bytes: PROCESS_PEAK_BYTES
                .load(Ordering::Relaxed)
                .saturating_sub(PROCESS_TROUGH_BYTES.load(Ordering::Relaxed)),
        }
    }

    /// Measure ONE LEG of this window's work, without disturbing what this
    /// window goes on to report.
    ///
    /// A window reports one number for everything it did. That is the right
    /// answer to "what must the memory ceiling hold?" and the wrong answer to
    /// "which half of this workload costs that?" — and the second question is
    /// the one that attributes a change to the thing that was changed. A leg
    /// measured here answers it: two ways of doing the same work, metered at
    /// the same scale with the same data already resident, so what is compared
    /// is the ADDITIONAL residency each way requires rather than the residency
    /// they share.
    ///
    /// # This is the only legal nesting, and `&self` is what makes it legal
    ///
    /// [`WholeProcessWindow::open`] panics if a window is already open, because
    /// two live windows over one set of counters would each report the union of
    /// both regions while appearing to report their own. That guard is not
    /// weakened here and must never be: a leg is reachable only through a
    /// window that is already open, so it cannot reach `open` at all.
    ///
    /// # The restore is load-bearing — do not remove it
    ///
    /// Anchoring the marks at the leg's start is what lets the leg report its
    /// own span, and it necessarily discards the enclosing window's marks. They
    /// are handed back afterwards, folded with the leg's own. Without that, an
    /// enclosing window's reported peak silently becomes "whatever happened
    /// after the last leg" — a number that is wrong in the direction that looks
    /// like an improvement, and that is produced entirely by the act of
    /// measuring. That failure has already been observed once on this probe: an
    /// early run reported a workload peak falling to 122 MB, and none of it was
    /// real.
    ///
    /// The fold is `fetch_max`/`fetch_min` rather than a store, so a recorder on
    /// another thread that moved a mark between the reads below and the restore
    /// is not clobbered.
    ///
    /// # What a leg charges
    ///
    /// Every thread, exactly as the enclosing window does — so a leg that fans
    /// out over `rayon` is charged for its workers. The corollary is that
    /// unrelated traffic on another thread during the leg lands in the leg; a
    /// leg is a time slice of a process-wide ledger, not a thread of it.
    pub fn metered<T>(&self, leg: impl FnOnce() -> T) -> (T, Measurement) {
        let enclosing_peak = PROCESS_PEAK_BYTES.load(Ordering::Relaxed);
        let enclosing_trough = PROCESS_TROUGH_BYTES.load(Ordering::Relaxed);
        let allocations_before = PROCESS_ALLOCATIONS.load(Ordering::Relaxed);
        let requested_before = PROCESS_REQUESTED_BYTES.load(Ordering::Relaxed);
        let live_before = PROCESS_LIVE_BYTES.load(Ordering::Relaxed);

        // Both marks anchor at the level live bytes hold NOW, so the leg's span
        // is measured from where the leg starts rather than from where the
        // enclosing window opened.
        PROCESS_PEAK_BYTES.store(live_before, Ordering::Relaxed);
        PROCESS_TROUGH_BYTES.store(live_before, Ordering::Relaxed);

        let value = leg();

        let leg_peak = PROCESS_PEAK_BYTES.load(Ordering::Relaxed);
        let leg_trough = PROCESS_TROUGH_BYTES.load(Ordering::Relaxed);
        let live_after = PROCESS_LIVE_BYTES.load(Ordering::Relaxed);
        let allocations_after = PROCESS_ALLOCATIONS.load(Ordering::Relaxed);
        let requested_after = PROCESS_REQUESTED_BYTES.load(Ordering::Relaxed);

        PROCESS_PEAK_BYTES.fetch_max(enclosing_peak, Ordering::Relaxed);
        PROCESS_TROUGH_BYTES.fetch_min(enclosing_trough, Ordering::Relaxed);

        let measured = Measurement {
            allocations: allocations_after.saturating_sub(allocations_before),
            requested_bytes: requested_after.saturating_sub(requested_before),
            retained_bytes: live_after.saturating_sub(live_before),
            peak_working_bytes: leg_peak.saturating_sub(leg_trough),
        };
        (value, measured)
    }

    /// Close the window, returning what it observed.
    ///
    /// Disarms the ledger and waits for every in-flight recorder to leave it
    /// BEFORE sampling, so the returned measurement cannot still be updated
    /// by a recorder that passed the armed check a moment before `close` was
    /// called; sampling first (the previous behaviour) left exactly that gap
    /// open.
    pub fn close(self) -> Measurement {
        process_disarm_and_drain();
        self.sample()
    }
}

impl Drop for WholeProcessWindow {
    fn drop(&mut self) {
        // Disarm-and-drain first (a no-op if `close` already ran it), and
        // only then release the nesting guard. `PROCESS_WINDOW_HELD` gates
        // the next `open`'s zeroing of the counters, so it must not drop
        // while a recorder from this window could still be updating them —
        // otherwise the next window's `open` could zero counters a straggler
        // from this one is about to add to.
        process_disarm_and_drain();
        PROCESS_WINDOW_HELD.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use std::hint::black_box;
    use std::panic::{self, AssertUnwindSafe};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
    use std::thread;
    use std::time::Duration;

    use super::{
        CountingAllocator, CurrentThreadWindow, PROCESS_ARMED_BIT, PROCESS_COUNT_MASK,
        PROCESS_STATE, PROCESS_WINDOW_HELD, WholeProcessWindow, process_enter, process_leave,
    };

    // The instrument can only be tested through the allocator it instruments,
    // so this crate's own test binary installs it.
    #[global_allocator]
    static GLOBAL: CountingAllocator = CountingAllocator;

    /// Serializes this module's tests, so the whole-process one is alone.
    ///
    /// [`WholeProcessWindow`]'s documented precondition is that the test holding
    /// one "must also be the only thing running". That is not free under a
    /// harness that runs test functions concurrently over one allocator: a
    /// sibling test's still-live buffer lands in this window's `retained_bytes`
    /// indistinguishably from retention by the measured region, which is the
    /// documented hazard rather than a defect in the ledger. The precondition
    /// has to be taken, so every test here takes it.
    static EXCLUSIVE: Mutex<()> = Mutex::new(());

    /// Take [`EXCLUSIVE`] for the rest of the caller's test.
    ///
    /// Poisoning is expected, not exceptional: `nesting_a_current_thread_window_is_refused`
    /// panics by design while holding the guard. A poisoned lock still excludes,
    /// and there is no shared state behind it to have been left inconsistent.
    fn exclusive() -> MutexGuard<'static, ()> {
        EXCLUSIVE.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// How large the buffer the cross-thread tests allocate is; big enough that
    /// no incidental traffic could be mistaken for it.
    const PROBE_BYTES: usize = 1 << 20;

    /// A vector of `PROBE_BYTES` bytes, returned so the caller decides when it
    /// is freed.
    fn probe_buffer() -> Vec<u8> {
        black_box(vec![7u8; PROBE_BYTES])
    }

    #[test]
    fn a_current_thread_window_reports_the_four_quantities_apart() {
        let _exclusive = exclusive();
        let window = CurrentThreadWindow::open();
        let transient = probe_buffer();
        let transient_len = transient.len();
        drop(transient);
        let retained = probe_buffer();
        let measured = window.close();

        assert!(measured.allocations >= 2, "{measured:?}");
        assert!(
            measured.requested_bytes >= 2 * PROBE_BYTES as u64,
            "traffic counts the freed buffer too: {measured:?}"
        );
        assert!(
            measured.retained_bytes >= PROBE_BYTES as i64,
            "the surviving buffer is retained: {measured:?}"
        );
        assert!(
            measured.retained_bytes < 2 * PROBE_BYTES as i64,
            "the freed buffer must not be retained: {measured:?}"
        );
        assert!(
            measured.peak_working_bytes >= PROBE_BYTES as i64,
            "{measured:?}"
        );
        assert_eq!(retained.len(), transient_len);
        drop(retained);
    }

    #[test]
    fn a_current_thread_window_does_not_see_another_thread() {
        // The documented hazard, executed: this is why a window over a
        // `rayon`-parallel region has to be the whole-process one.
        let _exclusive = exclusive();
        let window = CurrentThreadWindow::open();
        thread::spawn(|| drop(probe_buffer()))
            .join()
            .expect("the probe thread completes");
        let measured = window.close();
        assert!(
            measured.requested_bytes < PROBE_BYTES as u64,
            "a per-thread window counted another thread's allocation: {measured:?}"
        );
    }

    /// Executes the documented cross-thread-ownership hazard rather than just
    /// asserting it in prose: a buffer this thread allocates, then hands off
    /// to a DIFFERENT thread to free, never records a `dealloc` against this
    /// window's per-thread ledger, so it must read here as still retained
    /// even though the process has, in fact, freed it. `CurrentThreadWindow`
    /// counts allocator calls made on the thread that opened it, not object
    /// ownership — this is that distinction observed operationally, matching
    /// the semantics documented on [`Measurement::retained_bytes`] and on
    /// [`CurrentThreadWindow`] itself.
    #[test]
    fn a_current_thread_window_reads_a_buffer_freed_on_another_thread_as_retained() {
        let _exclusive = exclusive();
        let window = CurrentThreadWindow::open();
        // Allocated HERE, on the thread that opened the window.
        let buffer = probe_buffer();

        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        let dropper = thread::spawn(move || {
            let buffer = rx.recv().expect("the buffer is sent");
            // Freed on the OTHER thread — the dealloc lands on ITS ledger,
            // never on the opening thread's.
            drop(buffer);
        });
        tx.send(buffer).expect("the dropper thread is alive");
        dropper.join().expect("the dropper thread completes");

        let measured = window.close();
        assert!(
            measured.retained_bytes >= PROBE_BYTES as i64,
            "a buffer freed on another thread must still read as retained on \
             the allocating thread's per-thread ledger: {measured:?}"
        );
    }

    #[test]
    #[should_panic(expected = "a CurrentThreadWindow is already open")]
    fn nesting_a_current_thread_window_is_refused() {
        let _exclusive = exclusive();
        let _outer = CurrentThreadWindow::open();
        let _inner = CurrentThreadWindow::open();
    }

    /// Everything the whole-process window claims, in ONE test function.
    ///
    /// The ledger it reads has exactly one set of counters for the process, so
    /// two test functions exercising it would race under a harness that runs
    /// tests concurrently — the second `open` would meet the first's token and
    /// refuse. Keeping the whole-process surface in a single test settles that
    /// much, but not the measurement: the window counts every thread, so a
    /// sibling test's live buffer is retention as far as the ledger can tell,
    /// and the upper bound below would fail on scheduling alone. [`EXCLUSIVE`]
    /// is what makes these assertions statements about the instrument.
    #[test]
    fn the_whole_process_window_sees_every_thread_and_only_while_it_is_open() {
        let _exclusive = exclusive();
        let window = WholeProcessWindow::open();
        thread::spawn(|| drop(probe_buffer()))
            .join()
            .expect("the probe thread completes");

        // A second window cannot exist: the counters it would zero are the ones
        // this window is reading.
        let nested = panic::catch_unwind(AssertUnwindSafe(|| {
            let _inner = WholeProcessWindow::open();
        }));
        assert!(nested.is_err(), "a nested whole-process window was allowed");

        let measured = window.close();
        assert!(
            measured.requested_bytes >= PROBE_BYTES as u64,
            "a whole-process window missed another thread's allocation: {measured:?}"
        );
        assert!(measured.allocations >= 1, "{measured:?}");
        assert!(
            measured.peak_working_bytes >= PROBE_BYTES as i64,
            "a whole-process window missed another thread's working set: {measured:?}"
        );
        // The buffer was freed inside the window, so it is traffic and peak but
        // not retention — the distinction the four quantities exist to keep.
        assert!(
            measured.retained_bytes < PROBE_BYTES as i64,
            "a buffer freed inside the window was reported as retained: {measured:?}"
        );

        // And the ledger is disarmed again, with no in-flight recorder left
        // registered, so nothing after this point is charged to a window
        // nobody has open.
        let state = PROCESS_STATE.load(Ordering::Acquire);
        assert!(
            state & PROCESS_ARMED_BIT == 0,
            "the closed window left the process ledger armed"
        );
        assert!(
            state & PROCESS_COUNT_MASK == 0,
            "the closed window left an in-flight recorder registered"
        );
        assert!(
            !PROCESS_WINDOW_HELD.load(Ordering::Relaxed),
            "the closed window left its nesting guard held"
        );
    }

    /// The regression test for the defect trough-tracking fixes: memory
    /// allocated BEFORE the window opened, then freed INSIDE it, must not
    /// depress the reported peak below a buffer allocated later in the same
    /// window.
    ///
    /// Against the old code — a bare high-water mark of a live-byte delta
    /// that opens at zero — freeing `baseline` here drives `PROCESS_LIVE_BYTES`
    /// negative before the probe buffer is allocated, so the probe's own peak
    /// landed under `PROBE_BYTES` and only a slack constant made the
    /// assertion pass. Tracking the trough alongside the peak and reporting
    /// their difference removes the need for slack: the span is exact, no
    /// tolerance.
    #[test]
    fn peak_working_bytes_is_exact_when_a_pre_window_buffer_is_freed_inside_it() {
        let _exclusive = exclusive();
        let baseline = probe_buffer();
        let window = WholeProcessWindow::open();
        drop(baseline);
        let probe = probe_buffer();
        let measured = window.close();
        drop(probe);

        assert!(
            measured.peak_working_bytes >= PROBE_BYTES as i64,
            "freeing a pre-window buffer inside the window depressed the peak: {measured:?}"
        );
    }

    /// A window that only frees memory allocated before it opened can never
    /// report a negative working-set span: the low-water mark falls below the
    /// anchor, but the high-water mark it is subtracted from can fall no
    /// lower than that same anchor, since nothing inside the window raises
    /// live bytes above it.
    #[test]
    fn peak_working_bytes_is_never_negative_for_a_window_that_only_frees() {
        let _exclusive = exclusive();
        let baseline = probe_buffer();
        let window = WholeProcessWindow::open();
        drop(baseline);
        let measured = window.close();

        assert!(
            measured.peak_working_bytes >= 0,
            "a window that only frees reported a negative working-set span: {measured:?}"
        );
    }

    /// Pins the drain itself: `close` must not return — and therefore must
    /// not sample — while a recorder that passed the armed check is still
    /// inside the counted region.
    ///
    /// This is the exact defect the packed `PROCESS_STATE` word exists to
    /// close. Under the old design (a bare `AtomicBool` for armed, and
    /// nothing tracking in-flight recorders) `close` sampled immediately: a
    /// recorder that had already passed the armed check but not yet touched
    /// the counters was invisible to it, so this test's closer thread would
    /// send on `done_tx` almost at once, well inside the 50ms timeout below,
    /// and the first assertion would fail. Against the fix, entering via
    /// [`process_enter`] and withholding [`process_leave`] must block
    /// `close` in `process_disarm_and_drain` until the entered recorder
    /// leaves, which this test observes directly through a channel rather
    /// than by timing.
    #[test]
    fn closing_a_whole_process_window_waits_for_an_in_flight_recorder_to_leave() {
        let _exclusive = exclusive();
        let window = WholeProcessWindow::open();

        // Simulate a recorder that has passed the armed check and is
        // between that check and its matching `process_leave` — exactly the
        // gap the old check-then-act code left unguarded.
        assert!(
            process_enter(),
            "process_enter must succeed while the window is armed"
        );

        let (done_tx, done_rx) = mpsc::channel();
        let closer = thread::spawn(move || {
            let measured = window.close();
            done_tx
                .send(())
                .expect("the test thread is still waiting on done_rx");
            measured
        });

        // The closer cannot have finished: `process_disarm_and_drain` spins
        // until the in-flight count reaches zero, and it cannot, because the
        // entered recorder above has not left yet. A short timeout is enough
        // to prove "has not finished" without asserting exact timing — no
        // amount of waiting here can make the closer finish, because nothing
        // has permitted it to.
        assert_eq!(
            done_rx.recv_timeout(Duration::from_millis(50)),
            Err(mpsc::RecvTimeoutError::Timeout),
            "close() returned while a recorder was still inside the counted region"
        );

        // Now let the simulated recorder leave, which is the only thing that
        // can unblock the drain.
        process_leave();

        done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("close() must complete once the in-flight recorder leaves");
        closer.join().expect("the closer thread completes");

        let state = PROCESS_STATE.load(Ordering::Acquire);
        assert_eq!(state, 0, "the drained window left a nonzero process state");
    }

    /// Stress-shaped companion to the deterministic test above: hammer real
    /// allocations from worker threads that are still mid-flight when a
    /// window closes, under heavy contention on [`PROCESS_STATE`], and
    /// confirm a freshly opened second window's count is exactly the known
    /// quantity allocated inside it.
    ///
    /// What this pins precisely: the workers are joined — meaning every
    /// `fetch_add` any of them will ever do has already happened — before
    /// the second window opens, so this test cannot by itself force the
    /// narrow interleaving the previous test injects directly (a worker
    /// straggling past a `close`/`open` boundary that has already gone by).
    /// It is a heavy-load sanity check that `process_disarm_and_drain`
    /// actually waits out real concurrent allocator traffic — not synthetic,
    /// single manually-held entries — before `close` returns, and that doing
    /// so under contention still leaves the ledger exactly zeroed for the
    /// next window. The deterministic test above is what pins the exact
    /// race described in the task; this one is additional coverage at
    /// volume.
    #[test]
    fn a_second_window_counts_exactly_its_own_allocations_after_heavy_contention() {
        let _exclusive = exclusive();
        const WORKER_ALLOCATION_BYTES: usize = 64;
        const SECOND_WINDOW_ALLOCATIONS: u64 = 32;
        const WORKER_ITERATIONS: u64 = 20_000;

        let keep_running = Arc::new(AtomicU64::new(1));
        let workers: Vec<_> = (0..4)
            .map(|_| {
                let keep_running = Arc::clone(&keep_running);
                thread::spawn(move || {
                    let mut done = 0u64;
                    while keep_running.load(Ordering::Relaxed) != 0 && done < WORKER_ITERATIONS {
                        drop(black_box(vec![9u8; WORKER_ALLOCATION_BYTES]));
                        done += 1;
                    }
                })
            })
            .collect();

        let first = WholeProcessWindow::open();
        // Let the workers pile up real contention on `PROCESS_STATE` while
        // the window is still armed, then close through it.
        thread::sleep(Duration::from_millis(5));
        let _ = first.close();

        // Signal and join before the second window opens: every worker
        // `fetch_add` that will ever happen is complete before `open` below
        // zeroes the counters, which is what makes the exact-equality
        // assertion below sound (see the doc comment above).
        keep_running.store(0, Ordering::Relaxed);
        for worker in workers {
            worker.join().expect("worker thread completes");
        }

        let second = WholeProcessWindow::open();
        for _ in 0..SECOND_WINDOW_ALLOCATIONS {
            drop(black_box(vec![3u8; WORKER_ALLOCATION_BYTES]));
        }
        let measured = second.close();

        assert_eq!(
            measured.allocations, SECOND_WINDOW_ALLOCATIONS,
            "a fresh window's count did not match its own known allocations: {measured:?}"
        );
        let state = PROCESS_STATE.load(Ordering::Acquire);
        assert_eq!(
            state, 0,
            "the second window left the process ledger non-zero after closing"
        );
    }
}
