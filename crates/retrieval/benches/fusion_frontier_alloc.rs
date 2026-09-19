// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// This bench is a plain `main`, but the workspace `missing_docs` lint applies to
// its items just the same; the reporting helpers below are internal probes, not
// API.
#![allow(missing_docs)]

//! Peak-allocation evidence for the fused frontier, in its own process so the
//! tracking allocator's atomics skew no timed measurement.
//!
//! **This bench asserts nothing.** It reports. The machine it runs on is not
//! quiet, so a threshold here would be a flaky gate rather than a measurement;
//! the claim that the frontier is bounded is made — and enforced — by
//! `tests/fusion_frontier_alloc.rs`, which compares two runs against each other
//! rather than against a number. What this adds is the shape of the curve: the
//! same numbers across a grid of stream lengths, fused-row counts and strata
//! counts, so a regression is visible as a *trend* and not only as a tripped
//! bound.
//!
//! For each phase it reports two DISTINCT numbers, on purpose:
//!
//! * **`peak_allocated_bytes`** — the high-water mark of live bytes requested
//!   through the global allocator while the rows were being fused. This is
//!   the fusion's own working set: the frontier, the per-stream duplicate sets
//!   and whatever transient each pull makes.
//! * **`rss_delta_kb`** — the change in the process resident set
//!   (`/proc/self/statm`) across the same window, which includes pages the
//!   allocator never handed back to the system. Conflating the two would hide
//!   exactly the difference this bench exists to show: a fusion can hold a small
//!   frontier and still have grown the process.
//!
//! The fixture is multi-stratum with genuine cross-stratum disagreement. A
//! single-stratum fusion has no frontier to measure — nothing is ever held
//! awaiting confirmation — so it would report the same bytes whether the
//! frontier were bounded or not.

use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::BTreeMap;
use std::future::Future;
use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

use purrdf_retrieval::{
    CandidateDomains, DecayRule, DomainTag, DuplicatePolicy, Fixed, FusionProfile, FusionStream,
    Iri, ProducerReceipt, ProtocolError, RankFidelity, RankedStream, StreamContract, Term,
    contribution,
};

// ---------------------------------------------------------------------------
// The tracking allocator
// ---------------------------------------------------------------------------

static LIVE_BYTES: AtomicI64 = AtomicI64::new(0);
static PEAK_BYTES: AtomicI64 = AtomicI64::new(0);

fn to_i64(size: usize) -> i64 {
    i64::try_from(size).unwrap_or(i64::MAX)
}

fn record_allocation(size: usize) {
    let size = to_i64(size);
    let live = LIVE_BYTES
        .fetch_add(size, Ordering::Relaxed)
        .saturating_add(size);
    let mut peak = PEAK_BYTES.load(Ordering::Relaxed);
    while live > peak {
        match PEAK_BYTES.compare_exchange_weak(peak, live, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(observed) => peak = observed,
        }
    }
}

fn record_deallocation(size: usize) {
    LIVE_BYTES.fetch_sub(to_i64(size), Ordering::Relaxed);
}

struct CountingAllocator;

// SAFETY: every operation delegates to `System` with the exact incoming pointer
// and layout; the atomic accounting does not affect allocator ownership.
unsafe impl GlobalAlloc for CountingAllocator {
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
        // SAFETY: delegated with the caller's exact pointer/layout.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: delegated with the caller's exact pointer/layout and size.
        let resized = unsafe { System.realloc(pointer, layout, new_size) };
        if !resized.is_null() {
            record_deallocation(layout.size());
            record_allocation(new_size);
        }
        resized
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Set the peak high-water mark to the current live bytes, returning that
/// baseline.
fn reset_peak() -> i64 {
    let live = LIVE_BYTES.load(Ordering::Relaxed);
    PEAK_BYTES.store(live, Ordering::Relaxed);
    live
}

/// Peak allocated bytes since `baseline` (the value [`reset_peak`] returned).
fn peak_since(baseline: i64) -> i64 {
    PEAK_BYTES.load(Ordering::Relaxed).saturating_sub(baseline)
}

/// The process resident set size in KiB, read from `/proc/self/statm` (field 2
/// is the resident page count). Linux-only; on any other platform this reports
/// `0` and only the allocator figures carry the evidence.
fn rss_kb() -> u64 {
    #[cfg(target_os = "linux")]
    {
        let statm = std::fs::read_to_string("/proc/self/statm").unwrap_or_default();
        let resident_pages: u64 = statm
            .split_whitespace()
            .nth(1)
            .and_then(|field| field.parse().ok())
            .unwrap_or(0);
        // 4 KiB pages on every Linux target this runs on; a report-only figure.
        resident_pages * 4
    }
    #[cfg(not(target_os = "linux"))]
    {
        0
    }
}

fn report(label: &str, peak_allocated_bytes: i64, rss_before_kb: u64, rss_after_kb: u64) {
    println!(
        "[fusion_frontier_alloc] {label}: peak_allocated_bytes={peak_allocated_bytes} \
         rss_delta_kb={}",
        i64::try_from(rss_after_kb).unwrap_or(i64::MAX)
            - i64::try_from(rss_before_kb).unwrap_or(i64::MAX),
    );
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

/// The reciprocal-rank smoothing constant every phase fuses under.
const K: u32 = 60;

/// How many candidates the strata disagree about at once: each stratum emits the
/// same universe permuted within blocks of this size.
const DISAGREEMENT_BLOCK: u64 = 4;

/// The three strata every phase fuses, in their permutation order.
///
/// # Why three, and why these permutations
///
/// The stratum count is fixed rather than swept, because the *fixture* — not the
/// engine — has to be chosen so no two fully-confirmed candidates end up with the
/// same fused score. A fused score is a sum of `contribution(rank)` terms, so two
/// candidates whose rank multisets coincide across the strata score identically,
/// and certification asks a candidate to **strictly** beat every rival's upper
/// bound. Two exactly-tied finalists therefore each block the other for as long
/// as any stream is live, and the frontier grows with the input instead of with
/// the disagreement window — which would make this bench measure the fixture's
/// degeneracy rather than the engine's discipline.
///
/// These three permutations — identity, block reversal, and a rotation by half a
/// block — give the four candidates of every block the rank multisets
/// `{1,3,4}`, `{2,3,4}`, `{1,2,3}` and `{1,2,4}`. All four are distinct, so all
/// four scores are distinct and every block certifies in order.
const STRATA: [&str; 3] = ["nra/a", "nra/b", "nra/c"];

/// A hard ceiling on the rows one phase may pull, so a fixture that stopped
/// converging would report a short answer rather than run until the streams are
/// exhausted.
const PULL_BUDGET: usize = 1 << 16;

fn stratum(suffix: &str) -> Iri {
    Iri::parse(&format!("http://example.org/stratum/{suffix}")).expect("fixture IRIs are valid")
}

/// The candidate index stream `stream` emits at 1-based `rank`.
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
struct LazyStream {
    stream_index: usize,
    emitted: u64,
    total: u64,
    pulls: Arc<AtomicUsize>,
    /// The stratum weight this stream's contributions are computed at, so the
    /// rows always carry the value the fixture's own profile re-derives.
    weight: Fixed,
    /// The block this stream draws its candidates from, or `None` for the
    /// overlapping fixture's shared, permuted universe.
    ///
    /// `Some` makes the three strata's candidate sets disjoint and makes each
    /// producer declare the one block it draws from — the shape whose reading
    /// is unbounded without a declaration, because no confirmation from
    /// another stratum is ever coming.
    block: Option<usize>,
}

// The trait's methods are `async`; this fixture's body is synchronous because it
// mints its rows arithmetically.
#[allow(clippy::unused_async_trait_impl)]
impl RankedStream for LazyStream {
    type Item = Term;

    async fn next(&mut self) -> Result<Option<(u64, Fixed, Self::Item)>, ProtocolError> {
        if self.emitted >= self.total {
            return Ok(None);
        }
        self.emitted += 1;
        self.pulls.fetch_add(1, Ordering::Relaxed);
        let rank = self.emitted;
        let value = contribution(self.weight, rank, K).expect("the contribution fits");
        let item = match self.block {
            None => {
                let index = permuted_index(self.stream_index, rank);
                format!("candidate-{index:08}")
            }
            Some(block) => format!("block-{block}/candidate-{rank:08}"),
        };
        Ok(Some((rank, value, Term::new(item))))
    }

    async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError> {
        Ok(ProducerReceipt::Exhausted {
            rows_emitted: self.emitted,
        })
    }

    /// Each stream emits distinct items whose contributions strictly descend,
    /// under either fixture.
    ///
    /// The overlapping fixture's streams each emit a permutation of one
    /// universe, so every stream really can name every candidate, which is
    /// what [`CandidateDomains::Unrestricted`] says. The disjoint fixture's
    /// streams each mint their items under their own prefix, so each really
    /// does draw from one block, and says so.
    fn contract(&self) -> StreamContract {
        match self.block {
            None => StreamContract::new(
                DuplicatePolicy::Unique,
                RankFidelity::EXACT,
                CandidateDomains::Unrestricted,
            ),
            Some(block) => StreamContract::new(
                DuplicatePolicy::Unique,
                RankFidelity::EXACT,
                CandidateDomains::within([
                    DomainTag::parse(BLOCKS[block]).expect("the fixture block tags are valid IRIs")
                ]),
            ),
        }
    }
}

/// The profile and its three streams, each `total` rows long.
fn fixture(
    total: u64,
    pulls: &Arc<AtomicUsize>,
    weight: Fixed,
) -> (FusionProfile, Vec<(Iri, LazyStream)>) {
    let weights: BTreeMap<Iri, Fixed> = STRATA.iter().map(|name| (stratum(name), weight)).collect();
    let profile = FusionProfile::with_decay(weights, DecayRule::ReciprocalRank { k: K })
        .expect("the fixture profile is valid");
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
                    weight,
                    block: None,
                },
            )
        })
        .collect();
    (profile, streams)
}

/// Certify `rows` rows from three streams of `total` rows each, reporting the
/// peak heap and the resident-set delta across the certifying window.
fn phase(label: &str, total: u64, rows: usize) {
    phase_at_weight(label, total, rows, Fixed::ONE);
}

/// The same phase at a stratum weight whose adjacent ranks collide, so the
/// frontier is driven through the plateau regime.
///
/// Certification needs a score strictly above the threshold over the stream
/// heads; on a plateau the next head carries the same contribution, so nothing
/// certifies until the plateau ends. At a unit weight that regime begins past a
/// million ranks — beyond `PULL_BUDGET`, so every phase above stops short of it
/// and this bench has never once reported on it.
fn phase_at_weight(label: &str, total: u64, rows: usize, weight: Fixed) {
    let pulls = Arc::new(AtomicUsize::new(0));
    let (profile, streams) = fixture(total, &pulls, weight);
    let mut fusion = FusionStream::new(streams, profile);

    let rss_before = rss_kb();
    let baseline = reset_peak();
    let mut fused = 0usize;
    let mut contributions = 0usize;
    while fused < rows && pulls.load(Ordering::Relaxed) < PULL_BUDGET {
        let Some(row) = block_on(fusion.next()).expect("the fixture obeys the protocol") else {
            break;
        };
        contributions += row.contributions.len();
        fused += 1;
    }
    let peak = peak_since(baseline);
    let rss_after = rss_kb();
    let pulled = pulls.load(Ordering::Relaxed);
    drop(fusion);

    black_box(contributions);
    report(
        &format!("{label} fused={fused}/{rows} pulled={pulled}"),
        peak,
        rss_before,
        rss_after,
    );
}

/// The caller-named block each stratum of the disjoint fixture draws from, in
/// [`STRATA`]'s own order. Nothing here mints them.
const BLOCKS: [&str; 3] = [
    "http://example.org/domain/block-0",
    "http://example.org/domain/block-1",
    "http://example.org/domain/block-2",
];

/// The profile and three streams whose candidate sets are disjoint, each
/// declaring the one block it draws from.
fn disjoint_fixture(
    total: u64,
    pulls: &Arc<AtomicUsize>,
) -> (FusionProfile, Vec<(Iri, LazyStream)>) {
    let weights: BTreeMap<Iri, Fixed> = STRATA
        .iter()
        .map(|name| (stratum(name), Fixed::ONE))
        .collect();
    let profile = FusionProfile::with_decay(weights, DecayRule::ReciprocalRank { k: K })
        .expect("the fixture profile is valid");
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
                    weight: Fixed::ONE,
                    block: Some(stream_index),
                },
            )
        })
        .collect();
    (profile, streams)
}

/// The same phase over the disjoint fixture: the shape whose *reading* a
/// declaration bounds.
///
/// Every other phase in this bench fuses strata that emit one universe
/// permuted, where confirmations arrive early and the reading is bounded
/// whatever anybody declared — which is why the numbers here are the ones to
/// watch. `pulled` is the quantity of interest: over disjoint strata it is
/// `rows` plus a head per stream when the producers declare their blocks, and
/// the whole of every stream when they do not.
fn disjoint_phase(label: &str, total: u64, rows: usize) {
    let pulls = Arc::new(AtomicUsize::new(0));
    let (profile, streams) = disjoint_fixture(total, &pulls);
    let mut fusion = FusionStream::new(streams, profile);

    let rss_before = rss_kb();
    let baseline = reset_peak();
    let mut fused = 0usize;
    let mut contributions = 0usize;
    while fused < rows && pulls.load(Ordering::Relaxed) < PULL_BUDGET {
        let Some(row) = block_on(fusion.next()).expect("the fixture obeys the protocol") else {
            break;
        };
        contributions += row.contributions.len();
        fused += 1;
    }
    let peak = peak_since(baseline);
    let rss_after = rss_kb();
    let pulled = pulls.load(Ordering::Relaxed);
    drop(fusion);

    black_box(contributions);
    report(
        &format!("{label} fused={fused}/{rows} pulled={pulled}"),
        peak,
        rss_before,
        rss_after,
    );
}

fn main() {
    // Warm every lazy one-time allocation outside the reported phases.
    phase("warmup", 1_000, 32);
    println!("[fusion_frontier_alloc] --- stream length varies, rows fused fixed at 32 ---");
    for total in [1_000_u64, 10_000, 100_000, 1_000_000] {
        phase(&format!("strata=3 rows=32 stream={total}"), total, 32);
    }
    println!("[fusion_frontier_alloc] --- rows fused varies, stream fixed at 1000000 ---");
    for rows in [8_usize, 32, 128, 512, 2048] {
        phase(
            &format!("strata=3 rows={rows} stream=1000000"),
            1_000_000,
            rows,
        );
    }
    // The regime a removed refusal made reachable. One thousand raw units is
    // `10^-9`, at which adjacent ranks collide from the first pair, so every row
    // certified here is decided inside a plateau rather than by a strict score
    // difference. Report-only, like every phase above: what matters is that the
    // peak does not follow the stream length here either.
    println!(
        "[fusion_frontier_alloc] --- collided regime (weight 1e-9), rows fused fixed at 32 ---"
    );
    let colliding = Fixed::from_raw(1_000);
    for total in [1_000_u64, 10_000, 100_000, 1_000_000] {
        phase_at_weight(
            &format!("strata=3 rows=32 stream={total} collided"),
            total,
            32,
            colliding,
        );
    }
    // Disjoint strata, each producer declaring the block it draws from. This is
    // the configuration the reading bound is about: `pulled` should stay flat
    // as the streams grow by three orders of magnitude, where without a
    // declaration it would follow them exactly.
    println!(
        "[fusion_frontier_alloc] --- disjoint strata, declared blocks, rows fused fixed at 32 ---"
    );
    for total in [1_000_u64, 10_000, 100_000, 1_000_000] {
        disjoint_phase(
            &format!("strata=3 rows=32 stream={total} disjoint"),
            total,
            32,
        );
    }
}
