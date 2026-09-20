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
//!   through the global allocator while the rows were being fused, read from a
//!   whole-process window of the workspace's shared instrument. This is the
//!   fusion's own working set: the frontier, the per-stream duplicate sets and
//!   whatever transient each pull makes.
//! * **`rss_delta_kb`** — the change in the process resident set
//!   (`/proc/self/statm`) across the same window, which includes pages the
//!   allocator never handed back to the system and which no allocator ledger can
//!   supply. Conflating the two would hide exactly the difference this bench
//!   exists to show: a fusion can hold a small frontier and still have grown the
//!   process.
//!
//! The fixture is multi-stratum with genuine cross-stratum disagreement. A
//! single-stratum fusion has no frontier to measure — nothing is ever held
//! awaiting confirmation — so it would report the same bytes whether the
//! frontier were bounded or not.

use std::collections::BTreeMap;
use std::future::Future;
use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};
use purrdf_retrieval::{
    DecayRule, DuplicatePolicy, Fixed, FusionProfile, FusionStream, Iri, ProducerReceipt,
    ProtocolError, RankedStream, StreamContract, Term, contribution,
};

// ---------------------------------------------------------------------------
// The tracking allocator
// ---------------------------------------------------------------------------

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

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

    /// Each stream emits a permutation of the universe, so its items really are
    /// distinct and its contributions really do strictly descend.
    fn contract(&self) -> StreamContract {
        StreamContract::new(DuplicatePolicy::Unique)
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
    let window = WholeProcessWindow::open();
    let mut fused = 0usize;
    let mut contributions = 0usize;
    while fused < rows && pulls.load(Ordering::Relaxed) < PULL_BUDGET {
        let Some(row) = block_on(fusion.next()).expect("the fixture obeys the protocol") else {
            break;
        };
        contributions += row.contributions.len();
        fused += 1;
    }
    let peak = window.close().peak_working_bytes;
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
}
