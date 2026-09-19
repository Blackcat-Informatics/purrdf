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
    pub retained_bytes: i64,
    /// The high-water mark of live bytes during the window, relative to where
    /// the window started — the peak working set of the measured region.
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
    let _ = THREAD_LIVE_BYTES.try_with(|live| live.set(live.get().saturating_sub(widen_i64(size))));
}

// ---------------------------------------------------------------------------
// Whole-process accounting
// ---------------------------------------------------------------------------

/// Whether the allocator should keep the process-wide ledger at all.
///
/// Unlike the per-thread cells, this ledger is four atomics shared by every
/// thread, and keeping it is not free on a parallel workload. It is therefore
/// armed only for the duration of a [`WholeProcessWindow`], so a bench that
/// measures *time* outside its probe windows pays one relaxed load per
/// allocation and nothing else.
static PROCESS_ARMED: AtomicBool = AtomicBool::new(false);
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

/// Record `size` bytes handed out on any thread, if a window is armed.
fn process_record_allocation(size: usize) {
    if !PROCESS_ARMED.load(Ordering::Relaxed) {
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
}

/// Record `size` bytes returned on any thread, if a window is armed.
fn process_record_deallocation(size: usize) {
    if !PROCESS_ARMED.load(Ordering::Relaxed) {
        return;
    }
    PROCESS_LIVE_BYTES.fetch_sub(widen_i64(size), Ordering::Relaxed);
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
        Measurement {
            allocations: THREAD_ALLOCATIONS
                .with(Cell::get)
                .saturating_sub(self.allocations),
            requested_bytes: THREAD_REQUESTED_BYTES
                .with(Cell::get)
                .saturating_sub(self.requested_bytes),
            retained_bytes: live.saturating_sub(self.live_bytes),
            peak_working_bytes: peak.saturating_sub(self.live_bytes),
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
/// timed benchmark that opens a window only around its probe pays one relaxed
/// load per allocation the rest of the time.
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
        // Release: every thread that observes the armed flag must also observe
        // the zeroed counters, or it would add to a total from the last window.
        PROCESS_ARMED.store(true, Ordering::Release);
        Self(())
    }

    /// Read the window without closing it.
    pub fn sample(&self) -> Measurement {
        Measurement {
            allocations: PROCESS_ALLOCATIONS.load(Ordering::Relaxed),
            requested_bytes: PROCESS_REQUESTED_BYTES.load(Ordering::Relaxed),
            retained_bytes: PROCESS_LIVE_BYTES.load(Ordering::Relaxed),
            peak_working_bytes: PROCESS_PEAK_BYTES.load(Ordering::Relaxed),
        }
    }

    /// Close the window, returning what it observed.
    pub fn close(self) -> Measurement {
        self.sample()
    }
}

impl Drop for WholeProcessWindow {
    fn drop(&mut self) {
        PROCESS_ARMED.store(false, Ordering::Release);
        PROCESS_WINDOW_HELD.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use std::hint::black_box;
    use std::panic::{self, AssertUnwindSafe};
    use std::sync::atomic::Ordering;
    use std::sync::{Mutex, MutexGuard, PoisonError};
    use std::thread;

    use super::{
        CountingAllocator, CurrentThreadWindow, PROCESS_ARMED, PROCESS_WINDOW_HELD,
        WholeProcessWindow,
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

    /// How far a whole-process window's live-byte baseline may drift below
    /// where it opened before the peak assertion stops meaning anything.
    ///
    /// A whole-process window opens over a RUNNING process and reports deltas
    /// from that instant, so memory allocated before it and freed inside it
    /// reads as negative retention — the documented behaviour of
    /// [`Measurement::retained_bytes`]. `peak_working_bytes` is a high-water
    /// mark of that same signed delta, so the identical drift puts the peak of
    /// a `PROBE_BYTES` buffer just UNDER `PROBE_BYTES`, and a bound at exactly
    /// `PROBE_BYTES` claims a baseline of zero that this window never promised.
    /// Ambient drift is a few hundred bytes; this slack is far above that and
    /// far below the buffer, so nothing but the buffer can clear the bound.
    const BASELINE_DRIFT_SLACK: i64 = 64 * 1024;

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
            measured.peak_working_bytes >= PROBE_BYTES as i64 - BASELINE_DRIFT_SLACK,
            "a whole-process window missed another thread's working set: {measured:?}"
        );
        // The buffer was freed inside the window, so it is traffic and peak but
        // not retention — the distinction the four quantities exist to keep.
        assert!(
            measured.retained_bytes < PROBE_BYTES as i64,
            "a buffer freed inside the window was reported as retained: {measured:?}"
        );

        // And the ledger is disarmed again, so nothing after this point is
        // charged to a window nobody has open.
        assert!(
            !PROCESS_ARMED.load(Ordering::Relaxed),
            "the closed window left the process ledger armed"
        );
        assert!(
            !PROCESS_WINDOW_HELD.load(Ordering::Relaxed),
            "the closed window left its nesting guard held"
        );
    }
}
