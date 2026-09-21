// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
    CandidateDomains, Completeness, DecayRule, DomainTag, DuplicatePolicy, Fixed, FusionProfile,
    FusionStream, Iri, OrderFidelity, ProducerReceipt, ProtocolError, RankFidelity, RankedRow,
    RankedStream, RowBlock, ScoreInterval, StreamContract, Term, contribution,
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
    /// The block this stream draws its candidates from, or `None` for the
    /// overlapping fixture's shared, permuted universe.
    ///
    /// `Some` makes the three strata's candidate sets disjoint and makes each
    /// producer declare the one block it draws from — the shape whose reading
    /// is unbounded without a declaration, because no confirmation from
    /// another stratum is ever coming.
    block: Option<usize>,
    /// What this producer declares about the rows it emits.
    ///
    /// [`RankFidelity::EXACT`] for every phase that is not about the interval.
    /// A stratum declaring anything else puts the whole per-row score-interval
    /// arithmetic on the certifying path — the deficit charged to strata that
    /// did not name a row, the inflation charged to the degraded strata that
    /// did — which is otherwise never measured here, because nothing in an
    /// undegraded fusion reaches it.
    fidelity: RankFidelity,
}

// The trait's methods are `async`; this fixture's body is synchronous because it
// mints its rows arithmetically.
#[allow(clippy::unused_async_trait_impl)]
impl RankedStream for LazyStream {
    type Item = Term;

    async fn next(&mut self) -> Result<Option<RankedRow<Self::Item>>, ProtocolError> {
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
        // The block this row was drawn from, which a restricted stream owes on
        // every row. It is the same block the contract declares — the disjoint
        // fixture's streams each draw from exactly one, so that block IS where
        // every one of their rows comes from — and the overlapping fixture
        // declares nothing and so names nothing.
        let drawn_from = match self.block {
            None => RowBlock::Undeclared,
            Some(block) => RowBlock::Declared(
                DomainTag::parse(BLOCKS[block]).expect("the fixture block tags are valid IRIs"),
            ),
        };
        Ok(Some(RankedRow::new(
            rank,
            value,
            Term::new(item),
            drawn_from,
        )))
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
                self.fidelity.clone(),
                CandidateDomains::Unrestricted,
            ),
            Some(block) => StreamContract::new(
                DuplicatePolicy::Unique,
                self.fidelity.clone(),
                CandidateDomains::within([
                    DomainTag::parse(BLOCKS[block]).expect("the fixture block tags are valid IRIs")
                ]),
            ),
        }
    }
}

/// The profile and its three streams, each `total` rows long, every one of them
/// declaring the top of the fidelity lattice.
fn fixture(
    total: u64,
    pulls: &Arc<AtomicUsize>,
    weight: Fixed,
) -> (FusionProfile, Vec<(Iri, LazyStream)>) {
    fixture_declaring(
        total,
        pulls,
        weight,
        &[
            RankFidelity::EXACT,
            RankFidelity::EXACT,
            RankFidelity::EXACT,
        ],
    )
}

/// The same fixture with each stratum declaring `fidelities[i]` about itself.
fn fixture_declaring(
    total: u64,
    pulls: &Arc<AtomicUsize>,
    weight: Fixed,
    fidelities: &[RankFidelity; STRATA.len()],
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
                    fidelity: fidelities[stream_index].clone(),
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

/// A lossy but order-faithful declaration: what an HNSW graph is, and the only
/// shape that reaches the *bounded* interval arithmetic.
fn lossy() -> RankFidelity {
    RankFidelity {
        completeness: Completeness::Lossy {
            evidence: Arc::from("approximate: beam search, recall unmeasured"),
        },
        order: OrderFidelity::Faithful,
    }
}

/// A declaration for which no finite bound on the error exists, which is the
/// branch that short-circuits instead.
fn perturbed() -> RankFidelity {
    RankFidelity {
        completeness: Completeness::Lossy {
            evidence: Arc::from("approximate: beam search, recall unmeasured"),
        },
        order: OrderFidelity::Perturbed {
            evidence: Arc::from("quantized: distances compared in 8-bit space"),
        },
    }
}

/// The same fusion with one stratum declaring `fidelity`, so every certified row
/// is priced through the per-row score interval.
///
/// # Why this phase exists
///
/// Every other phase in this file fuses strata that all declare
/// [`RankFidelity::EXACT`], and for such a fusion the interval is zero-width by
/// construction: no stratum can have withheld a contribution and none can have
/// been promoted, so the arithmetic that computes those two terms is skipped
/// whole. That leaves the per-emitted-row work the interval actually costs
/// entirely unmeasured — which is the one thing worth reporting about it,
/// because it runs on the certifying path of every fused search rather than at
/// the end of one.
///
/// The two declarations reach *different* branches and both are reported:
///
/// * a **lossy, order-faithful** stratum takes the bounded path, charging a
///   deficit to every stratum that could still have named the row and an
///   inflation to every degraded stratum that did;
/// * a **perturbed** stratum takes the short-circuit, because an emitted rank
///   bounds nothing under it and there is no number to compute.
///
/// Report-only, like every phase above. Nothing here asserts a time, a ratio or
/// a bound: this machine is not quiet, and a threshold would be noise wearing a
/// gate's clothes.
fn degraded_phase(label: &str, total: u64, rows: usize, fidelity: &RankFidelity) {
    let pulls = Arc::new(AtomicUsize::new(0));
    let (profile, streams) = fixture_declaring(
        total,
        &pulls,
        Fixed::ONE,
        // One of the three, so the fusion holds degraded and undegraded strata
        // at once: the deficit loop then has both a stratum to charge a
        // rank-one contribution to and strata to charge only their open heads.
        &[fidelity.clone(), RankFidelity::EXACT, RankFidelity::EXACT],
    );
    let mut fusion = FusionStream::new(streams, profile);

    let rss_before = rss_kb();
    let window = WholeProcessWindow::open();
    let mut fused = 0usize;
    let mut contributions = 0usize;
    // Counted, not asserted: a phase that reported on the interval path without
    // ever reaching it would look identical to one that did.
    let mut bounded = 0usize;
    let mut unbounded = 0usize;
    while fused < rows && pulls.load(Ordering::Relaxed) < PULL_BUDGET {
        let Some(row) = block_on(fusion.next()).expect("the fixture obeys the protocol") else {
            break;
        };
        contributions += row.contributions.len();
        match &row.interval {
            ScoreInterval::Bounded { .. } => bounded += 1,
            ScoreInterval::Unbounded { .. } => unbounded += 1,
        }
        fused += 1;
    }
    let peak = window.close().peak_working_bytes;
    let rss_after = rss_kb();
    let pulled = pulls.load(Ordering::Relaxed);
    drop(fusion);

    black_box(contributions);
    report(
        &format!(
            "{label} fused={fused}/{rows} pulled={pulled} bounded={bounded} unbounded={unbounded}"
        ),
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
                    fidelity: RankFidelity::EXACT,
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
    // The score-interval path, which every phase above leaves unmeasured: one
    // stratum declares a shortfall, so each certified row is priced rather than
    // handed a zero-width interval. Both branches are reported — a lossy,
    // order-faithful stratum takes the bounded arithmetic, a perturbed one takes
    // the short-circuit — because they cost different things and only one of
    // them sums anything.
    println!(
        "[fusion_frontier_alloc] --- one lossy stratum (bounded interval), rows fused fixed at 32 ---"
    );
    let lossy = lossy();
    for total in [1_000_u64, 10_000, 100_000, 1_000_000] {
        degraded_phase(
            &format!("strata=3 rows=32 stream={total} lossy"),
            total,
            32,
            &lossy,
        );
    }
    println!(
        "[fusion_frontier_alloc] --- one perturbed stratum (unbounded interval), rows fused fixed at 32 ---"
    );
    let perturbed = perturbed();
    for total in [1_000_u64, 10_000, 100_000, 1_000_000] {
        degraded_phase(
            &format!("strata=3 rows=32 stream={total} perturbed"),
            total,
            32,
            &perturbed,
        );
    }
}
