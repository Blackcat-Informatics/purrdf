// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! **The fused frontier's peak heap is bounded by the profile, not by how long
//! the streams are.**
//!
//! Counting *pulls* proves the streams are consumed lazily. It cannot prove the
//! fusion is bounded in memory: an engine that pulled ten rows and retained a
//! million in a map would be lazy and unbounded at the same time, and a pull
//! counter would report it as a success. So this file counts **bytes**.
//!
//! The proof is operational. A `#[global_allocator]` tracks live heap bytes on
//! the measuring thread and records their high-water mark; the same fusion is
//! then run over stream sets three orders of magnitude apart in length, with the
//! same number of rows certified from each, and the two high-water marks are
//! compared. If the frontier — or the per-stream duplicate set, or anything else
//! the engine keeps — grew with the input, the longer run would show it.
//!
//! # It is [`fuse`] that is measured, not a fusion assembled here
//!
//! The bound belongs to the shipped entry point or it belongs to nothing. A
//! measurement taken over a hand-driven [`FusionStream`] would be measuring the
//! engine while the function every caller actually reaches — the one `search`
//! composes — went unmeasured, and that is exactly the gap a terminal report
//! that drained its streams would hide in: the rows would be bounded, the
//! reading would not, and no assertion here would notice. So each run below is a
//! whole `fuse` call, including the trailer it returns.
//!
//! # The fixture is multi-stratum on purpose
//!
//! The Fagin NRA frontier exists to hold candidates seen in *some* streams while
//! awaiting confirmation from the others. A single-stratum fusion has nothing to
//! hold and so measures nothing: it would report the same bytes whether the
//! frontier were bounded or unbounded. Three strata emit the same universe of
//! candidates permuted **within blocks**, so at every moment the strata genuinely
//! disagree about the order of a block's worth of candidates and the frontier is
//! genuinely populated — which the assertions below check before they measure.
//!
//! # The counter is thread-local
//!
//! `cargo test` runs a binary's tests concurrently on shared threads over one
//! `#[global_allocator]`, so a process-global counter would be contaminated by a
//! sibling test's allocations between two snapshots. A thread-local cell counts
//! only the measuring thread.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

use purrdf_retrieval::{
    DecayRule, DuplicatePolicy, Fixed, FusionProfile, Iri, ProducerReceipt, ProducerStatus,
    ProtocolError, RankedStream, StreamContract, Term, TopK, contribution, fuse,
};

// ---------------------------------------------------------------------------
// The tracking allocator
// ---------------------------------------------------------------------------

thread_local! {
    /// Live heap bytes requested on this thread through the global allocator.
    static LIVE_BYTES: Cell<i64> = const { Cell::new(0) };
    /// The high-water mark of [`LIVE_BYTES`] since the last reset.
    static PEAK_BYTES: Cell<i64> = const { Cell::new(0) };
}

/// Widen a layout size, saturating rather than wrapping on a size no allocator
/// could ever have served.
fn to_i64(size: usize) -> i64 {
    i64::try_from(size).unwrap_or(i64::MAX)
}

/// Record `size` bytes handed out, raising the high-water mark if it rose.
fn record_allocation(size: usize) {
    let _ = LIVE_BYTES.try_with(|live| {
        let now = live.get().saturating_add(to_i64(size));
        live.set(now);
        let _ = PEAK_BYTES.try_with(|peak| {
            if now > peak.get() {
                peak.set(now);
            }
        });
    });
}

/// Record `size` bytes returned.
fn record_deallocation(size: usize) {
    let _ = LIVE_BYTES.try_with(|live| live.set(live.get().saturating_sub(to_i64(size))));
}

/// A pass-through allocator that tracks live bytes on the current thread.
struct TrackingAllocator;

// SAFETY: every operation delegates to `System` with the caller's exact pointer
// and layout; the thread-local accounting does not affect allocator ownership.
unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: delegated with the caller's exact layout.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record_deallocation(layout.size());
        // SAFETY: delegated with the caller's exact pointer and layout.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: delegated with the caller's exact pointer, layout and size.
        let resized = unsafe { System.realloc(pointer, layout, new_size) };
        if !resized.is_null() {
            record_deallocation(layout.size());
            record_allocation(new_size);
        }
        resized
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

/// Pin the high-water mark to the current live bytes, returning that baseline.
fn reset_peak() -> i64 {
    let live = LIVE_BYTES.with(Cell::get);
    PEAK_BYTES.with(|peak| peak.set(live));
    live
}

/// Peak bytes allocated above `baseline` (the value [`reset_peak`] returned).
fn peak_since(baseline: i64) -> i64 {
    PEAK_BYTES.with(Cell::get).saturating_sub(baseline)
}

// ---------------------------------------------------------------------------
// A single-threaded executor (the fixture streams never actually pend)
// ---------------------------------------------------------------------------

fn block_on<F: Future>(future: F) -> F::Output {
    // `Waker::noop()` needs no thread and no allocation, so this drives a
    // future on any target, `wasm32-unknown-unknown` included.
    let mut context = Context::from_waker(Waker::noop());
    let mut future = Box::pin(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("the fixture streams are synchronous and never pend"),
    }
}

// ---------------------------------------------------------------------------
// The multi-stratum fixture
// ---------------------------------------------------------------------------

/// The reciprocal-rank smoothing constant the fixture profile fixes.
const K: u32 = 60;

/// How many candidates the strata may disagree about at once.
///
/// Each stratum emits the same universe permuted **within** blocks of this size,
/// so a candidate's rank differs across strata by less than one block. The
/// frontier therefore holds a couple of blocks' worth of candidates at any
/// moment, whatever the streams' length — which is the quantity under test.
const DISAGREEMENT_BLOCK: u64 = 4;

/// The three strata the fixture fuses, in their permutation order.
const STRATA: [&str; 3] = ["nra/a", "nra/b", "nra/c"];

/// How many fused rows each measured run certifies.
const CERTIFIED_ROWS: usize = 32;

fn stratum(suffix: &str) -> Iri {
    Iri::parse(&format!("http://example.org/stratum/{suffix}")).expect("fixture IRIs are valid")
}

/// The candidate index one stratum emits at 1-based `rank`.
///
/// Stratum 0 emits the universe in order, stratum 1 reverses each block and
/// stratum 2 rotates each block by half its width. Each is a permutation, so
/// every stream emits distinct items and every candidate is eventually confirmed
/// by every stratum.
fn permuted_index(stream: usize, rank: u64) -> u64 {
    let index = rank - 1;
    let block = index / DISAGREEMENT_BLOCK;
    let offset = index % DISAGREEMENT_BLOCK;
    let permuted = match stream {
        0 => offset,
        1 => DISAGREEMENT_BLOCK - 1 - offset,
        _ => (offset + DISAGREEMENT_BLOCK / 2) % DISAGREEMENT_BLOCK,
    };
    block * DISAGREEMENT_BLOCK + permuted
}

/// A producer that mints its rows lazily, so nothing is materialized up front.
#[derive(Debug)]
struct LazyStream {
    stream_index: usize,
    emitted: u64,
    total: u64,
    pulls: Arc<AtomicUsize>,
    /// What this stream declares about its own rows.
    ///
    /// Its rows are the same either way — each stratum emits a permutation, so
    /// no item ever repeats — and that is the point: the declaration alone
    /// decides whether the engine builds a per-stream identity set, so measuring
    /// the two against identical rows measures exactly what the declaration
    /// costs.
    duplicates: DuplicatePolicy,
}

// The trait's methods are `async`; this fixture's body is synchronous because it
// mints its rows arithmetically. The keyword is the trait's, not a signal that
// the body awaits.
#[allow(clippy::unused_async_trait_impl)]
impl RankedStream for LazyStream {
    type Item = Term;

    async fn next(&mut self) -> Result<Option<(u64, Fixed, Self::Item)>, ProtocolError> {
        if self.emitted >= self.total {
            return Ok(None);
        }
        self.emitted += 1;
        self.pulls.fetch_add(1, Ordering::SeqCst);
        let rank = self.emitted;
        let value = contribution(Fixed::ONE, rank, K).expect("the contribution fits");
        let index = permuted_index(self.stream_index, rank);
        Ok(Some((
            rank,
            value,
            Term::new(format!("candidate-{index:08}")),
        )))
    }

    async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError> {
        Ok(ProducerReceipt::Exhausted {
            rows_emitted: self.emitted,
        })
    }

    fn contract(&self) -> StreamContract {
        StreamContract::new(self.duplicates)
    }
}

/// The fixture profile and its three streams, each `total` rows long.
///
/// The profile's contribution maximum is three — one per stratum — because it
/// weights three strata; that is exactly the number of contributions a
/// candidate here can accumulate.
fn fixture(
    total: u64,
    duplicates: DuplicatePolicy,
    pulls: &Arc<AtomicUsize>,
) -> (FusionProfile, Vec<(Iri, LazyStream)>) {
    let weights: BTreeMap<Iri, Fixed> = STRATA
        .iter()
        .map(|name| (stratum(name), Fixed::ONE))
        .collect();
    let profile = FusionProfile::with_decay(weights, DecayRule::ReciprocalRank { k: K })
        .expect("the fixture profile is valid");
    assert_eq!(
        profile.max_contributions(),
        u32::try_from(STRATA.len()).expect("three strata"),
        "the contribution maximum is the stratum count, derived rather than declared"
    );
    let streams = STRATA
        .iter()
        .enumerate()
        .map(|(stream_index, name)| {
            (
                stratum(name),
                LazyStream {
                    stream_index,
                    emitted: 0,
                    total,
                    pulls: Arc::clone(pulls),
                    duplicates,
                },
            )
        })
        .collect();
    (profile, streams)
}

/// What one measured run observed.
#[derive(Clone, Copy, Debug)]
struct Measurement {
    /// Peak heap bytes above the baseline while the rows were certified.
    peak_bytes: i64,
    /// How many rows were pulled from the three streams in total.
    pulls: usize,
    /// How many `(stratum, rank, contribution)` triples the certified rows
    /// carried, summed — the evidence the frontier was actually populated.
    contributions: usize,
    /// How many of the three strata the trailer closed at a contribution bound
    /// rather than at exhaustion — the evidence the bound stopped the reading
    /// and not merely the returning.
    bounded: usize,
}

/// Fuse [`CERTIFIED_ROWS`] rows out of three streams of `total` rows each
/// through the shipped [`fuse`], measuring the peak heap it held while doing it.
fn measure(total: u64, duplicates: DuplicatePolicy) -> Measurement {
    measure_rows(total, CERTIFIED_ROWS, duplicates)
}

/// Fuse `rows` rows out of three streams of `total` rows each through the
/// shipped [`fuse`], measuring the peak heap it held while doing it.
fn measure_rows(total: u64, rows: usize, duplicates: DuplicatePolicy) -> Measurement {
    let pulls = Arc::new(AtomicUsize::new(0));
    let (profile, streams) = fixture(total, duplicates, &pulls);

    // The baseline is taken after the fixture is built, so what is measured is
    // the fusion's own working set and not the fixture's.
    let baseline = reset_peak();
    let result = block_on(fuse::<LazyStream, Term>(streams, &profile, TopK::new(rows)))
        .expect("the fixture streams obey the protocol");
    let peak_bytes = peak_since(baseline);
    let pulls = pulls.load(Ordering::SeqCst);

    assert_eq!(
        result.rows.len(),
        rows,
        "the bound is what stopped this run, so every measured run did the same work"
    );
    let contributions = result
        .rows
        .iter()
        .map(|row| row.contributions.len())
        .sum::<usize>();
    let bounded = result
        .trailer
        .statuses
        .values()
        .filter(|status| matches!(status, ProducerStatus::CeilingReached { .. }))
        .count();
    // The peak was read before this: what is measured is what the *fusion* held,
    // not what the caller then chose to keep.
    drop(result);
    Measurement {
        peak_bytes,
        pulls,
        contributions,
        bounded,
    }
}

#[test]
fn the_frontier_peak_tracks_the_profile_bound_and_not_the_stream_length() {
    // Warm every lazy one-time allocation (the format machinery, the IRI parser
    // tables) outside the measured window, so the first run is not charged for
    // state the later runs inherit.
    let warm = measure(1_000, DuplicatePolicy::Allowed);
    assert!(warm.pulls > 0);

    // Measured under `Allowed`, which is the harder of the two: it is the
    // declaration that makes the engine keep a per-stream identity set, so it is
    // the one under which the peak could still have tracked the input.
    let short = measure(1_000, DuplicatePolicy::Allowed);
    let long = measure(1_000_000, DuplicatePolicy::Allowed);

    // First: the frontier was genuinely exercised. Every certified row carries
    // one contribution per stratum, which it could only have done by being held
    // while the other two strata caught up.
    assert_eq!(
        short.contributions,
        CERTIFIED_ROWS * STRATA.len(),
        "the short run certified rows without every stratum confirming them"
    );
    assert_eq!(
        long.contributions,
        CERTIFIED_ROWS * STRATA.len(),
        "the long run certified rows without every stratum confirming them"
    );
    assert_eq!(
        short.pulls, long.pulls,
        "the two runs must do identical work; only the streams' length differs"
    );

    // Second: the bound stopped the *reading*, which is the only way the
    // comparison below can hold through `fuse` at all. Every stratum still held
    // rows when the bound was reached, and the trailer says so at the
    // contribution each was read down to instead of claiming exhaustion it would
    // have had to read a million rows to earn.
    assert_eq!(
        short.bounded,
        STRATA.len(),
        "a stratum that still held rows reported something other than a bounded stop"
    );
    assert_eq!(
        long.bounded,
        STRATA.len(),
        "a stratum that still held rows reported something other than a bounded stop"
    );

    // Third: the measurement is live. A fusion that held nothing at all would
    // read zero here, and then the comparison below would be vacuous.
    assert!(
        short.peak_bytes > 0,
        "the fusion allocated nothing, so this measures nothing"
    );

    // Fourth, the claim. The streams are a thousand times longer in the second
    // run and the peak does not follow: the frontier is bounded by the profile's
    // stratum count over the disagreement window, and the
    // duplicate-detection sets are bounded by the rows pulled — neither by the
    // rows available.
    assert_eq!(
        short.peak_bytes, long.peak_bytes,
        "peak heap tracked the stream length: {} bytes over 1e3 rows against {} over 1e6",
        short.peak_bytes, long.peak_bytes
    );

    // And in absolute terms it is small. Materializing even the candidate terms
    // of one 1e6-row stream would cost tens of megabytes; the whole fusion's
    // working set here is kilobytes.
    assert!(
        long.peak_bytes < 64 * 1024,
        "a bounded frontier should not cost {} bytes",
        long.peak_bytes
    );
}

#[test]
fn the_peak_follows_the_rows_certified_when_a_stream_declares_allowed_duplicates() {
    // The mirror of the test above, and the reason it is not vacuous: the peak
    // is not simply constant. Holding the streams at one length and pulling more
    // rows *does* move it, because a stream that declared `Allowed` is
    // de-duplicated against an identity set, and that set grows with the rows
    // pulled. So "the peak did not move when the input grew a thousandfold" is a
    // statement about the input, not an artefact of a peak that never moves.
    let _warm = measure(1_000, DuplicatePolicy::Allowed);
    let few = measure(1_000, DuplicatePolicy::Allowed);
    let many = measure_rows(1_000, CERTIFIED_ROWS * 8, DuplicatePolicy::Allowed);

    assert!(
        many.peak_bytes > few.peak_bytes,
        "certifying eight times as many rows must move the peak; \
         {} against {}",
        many.peak_bytes,
        few.peak_bytes
    );
}

/// **What a `Unique` declaration costs, measured rather than asserted.**
///
/// The identity set is the one structure a fusion holds that grows with the rows
/// *pulled* rather than with the disagreement window, and a stream that promised
/// no repeats is not charged for one. The rows below are identical under both
/// declarations — each stratum emits a permutation, so nothing ever repeats —
/// so every byte of difference between the two runs is the set and nothing else.
#[test]
fn a_unique_declaration_is_what_keeps_a_deep_answer_affordable() {
    // Warm the one-time allocations under both declarations, so neither run is
    // charged for state the other inherits.
    let _warm = measure(1_000, DuplicatePolicy::Allowed);
    let _warm = measure(1_000, DuplicatePolicy::Unique);

    let deep = CERTIFIED_ROWS * 8;
    let allowed = measure_rows(1_000, deep, DuplicatePolicy::Allowed);
    let unique = measure_rows(1_000, deep, DuplicatePolicy::Unique);

    // The rows are the same rows: the declaration decides what is *held*, never
    // what is answered.
    assert_eq!(
        allowed.pulls, unique.pulls,
        "the two declarations read the same stream to the same depth"
    );
    assert_eq!(
        allowed.contributions, unique.contributions,
        "and certify the same evidence"
    );

    assert!(
        unique.peak_bytes < allowed.peak_bytes,
        "a promise of uniqueness must cost less to keep than to check; \
         unique held {} bytes against allowed's {}",
        unique.peak_bytes,
        allowed.peak_bytes
    );

    // And the saving is the part that *grows*. Some growth with `k` is the
    // answer itself — `fuse` returns the certified rows, and eight times as many
    // rows is eight times as much answer under either declaration — so the claim
    // is about what the two runs do NOT share: the identity set. Comparing the
    // growth rather than the peak subtracts the answer from both sides.
    let shallow_allowed = measure_rows(1_000, CERTIFIED_ROWS, DuplicatePolicy::Allowed);
    let shallow_unique = measure_rows(1_000, CERTIFIED_ROWS, DuplicatePolicy::Unique);
    let allowed_growth = allowed.peak_bytes - shallow_allowed.peak_bytes;
    let unique_growth = unique.peak_bytes - shallow_unique.peak_bytes;
    assert!(
        unique_growth < allowed_growth,
        "reading eight times as deep must cost a `Unique` stream less than an \
         `Allowed` one; unique grew {unique_growth} bytes against allowed's \
         {allowed_growth}"
    );
}
