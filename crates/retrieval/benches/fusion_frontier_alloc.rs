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
//!
//! # The counters the later phases add, and why they are still report-only
//!
//! The shared-block phases report two more numbers beside the bytes:
//! `lookups`, the point queries a fusion spent to settle finality, and `work`,
//! the rows the reads behind its streams returned. They are here because a
//! narrowing judged on `pulled` alone is judged on the counter it was built to
//! lower — the rows a read materialised are the price, and a read that halved
//! the ranks while doubling the rows would look like a win in every other
//! number this file prints.
//!
//! **No phase here asserts anything, and none of them takes a clock reading.**
//! That is unchanged by the additions: this machine is not quiet, a timing
//! threshold would be a flaky gate, and a speedup claimed from a number
//! measured here would be a claim about the load on the box. The
//! non-regression claims live on the deterministic counters, in
//! `tests/multimodal_read_bound.rs` and `tests/exclusion_lookup.rs`, which
//! compare two runs against each other.

use std::collections::BTreeMap;
use std::future::Future;
use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};
use purrdf_retrieval::{
    CandidateDomains, Completeness, DecayRule, DomainTag, DuplicatePolicy, ExclusionBasis,
    ExclusionVerdict, Fixed, FusionProfile, FusionStream, Iri, OrderFidelity, ProducerReceipt,
    ProtocolError, RankFidelity, RankedRow, RankedStream, RowBlock, ScoreInterval, StreamContract,
    Term, contribution,
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
    /// Whether every stream of this fixture declares ONE block while minting
    /// candidates nobody else can name.
    ///
    /// The shape finality forbids stopping early in, and therefore the shape an
    /// exclusion lookup exists for: the declarations are all true, and the fact
    /// that is false — that no other stream will ever name this candidate — is
    /// one no promise about blocks can express. `block` stays `None` here
    /// because the declaration is not per stream, and the item prefix is the
    /// stream's own so the universes really are disjoint.
    shared_block: bool,
    /// What this producer declares its exclusion answers are a fact about.
    ///
    /// [`ExclusionBasis::Unavailable`] for every phase that predates the
    /// mechanism, which is every phase but the shared-block pair.
    exclusion: ExclusionBasis,
    /// How many candidate lookups this producer was asked, shared with the
    /// phase that reports it.
    lookups: Arc<AtomicUsize>,
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
        let item = if self.shared_block {
            // One declared block, one universe per stream: exactly the
            // configuration whose candidates never become final on a
            // declaration alone.
            format!("shared/stream-{}/candidate-{rank:08}", self.stream_index)
        } else {
            match self.block {
                None => {
                    let index = permuted_index(self.stream_index, rank);
                    format!("candidate-{index:08}")
                }
                Some(block) => format!("block-{block}/candidate-{rank:08}"),
            }
        };
        // The block this row was drawn from, which a restricted stream owes on
        // every row. It is the same block the contract declares — the disjoint
        // fixture's streams each draw from exactly one, so that block IS where
        // every one of their rows comes from — and the overlapping fixture
        // declares nothing and so names nothing.
        let drawn_from = if self.shared_block {
            RowBlock::Declared(
                DomainTag::parse(BLOCKS[0]).expect("the fixture block tags are valid IRIs"),
            )
        } else {
            match self.block {
                None => RowBlock::Undeclared,
                Some(block) => RowBlock::Declared(
                    DomainTag::parse(BLOCKS[block]).expect("the fixture block tags are valid IRIs"),
                ),
            }
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
        if self.shared_block {
            return StreamContract::new(
                DuplicatePolicy::Unique,
                self.fidelity.clone(),
                CandidateDomains::within([
                    DomainTag::parse(BLOCKS[0]).expect("the fixture block tags are valid IRIs")
                ]),
                self.exclusion,
            );
        }
        match self.block {
            None => StreamContract::new(
                DuplicatePolicy::Unique,
                self.fidelity.clone(),
                CandidateDomains::Unrestricted,
                self.exclusion,
            ),
            Some(block) => StreamContract::new(
                DuplicatePolicy::Unique,
                self.fidelity.clone(),
                CandidateDomains::within([
                    DomainTag::parse(BLOCKS[block]).expect("the fixture block tags are valid IRIs")
                ]),
                self.exclusion,
            ),
        }
    }

    /// Answer whether `candidate` is one this stream's universe holds, where
    /// this fixture declares a basis for the question.
    ///
    /// Read off the candidate's own prefix rather than by scanning, which is
    /// what makes it a point lookup: this stream mints
    /// `shared/stream-{i}/candidate-{rank}` and nothing else, so a term of any
    /// other shape is one it will never name. A stream that declared no basis
    /// answers the disagreement [`ProtocolError::ExclusionUnavailable`] names
    /// rather than a verdict it did not measure; fusion never asks it.
    async fn exclusion(&mut self, candidate: &Term) -> Result<ExclusionVerdict, ProtocolError> {
        if !self.exclusion.is_declared() {
            return Err(ProtocolError::ExclusionUnavailable);
        }
        self.lookups.fetch_add(1, Ordering::Relaxed);
        let mine = format!("shared/stream-{}/candidate-", self.stream_index);
        Ok(if candidate.as_str().starts_with(&mine) {
            ExclusionVerdict::Possible
        } else {
            ExclusionVerdict::Excluded
        })
    }

    /// The rows the read behind this stream returns, which for a fixture that
    /// mints arithmetically is the whole of the stream it was built for.
    ///
    /// Reported rather than declined because the phases below are about exactly
    /// this: a read taken at the fused frontier and a read taken at the planned
    /// depth cost different numbers of rows, and the fusion that consumes them
    /// pulls the same ranks either way.
    fn rows_materialised(&self) -> Option<u64> {
        Some(self.total)
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
                    shared_block: false,
                    exclusion: ExclusionBasis::Unavailable,
                    lookups: Arc::new(AtomicUsize::new(0)),
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
                    shared_block: false,
                    exclusion: ExclusionBasis::Unavailable,
                    lookups: Arc::new(AtomicUsize::new(0)),
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

/// The profile and three streams that all declare ONE block while minting
/// candidates nobody else can name, each answering lookups on `basis`.
fn shared_block_fixture(
    total: u64,
    pulls: &Arc<AtomicUsize>,
    lookups: &Arc<AtomicUsize>,
    basis: ExclusionBasis,
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
                    block: None,
                    fidelity: RankFidelity::EXACT,
                    shared_block: true,
                    exclusion: basis,
                    lookups: Arc::clone(lookups),
                },
            )
        })
        .collect();
    (profile, streams)
}

/// The shared-block phase: the configuration a declaration cannot narrow, with
/// and without the lookups that can.
///
/// **Report-only, and it asserts nothing about time.** No phase in this file
/// takes a clock reading and this one does not either; the machine is not quiet
/// and a timing threshold here would be a flaky gate rather than a measurement.
/// What it reports is the pair of quantities the mechanism is judged on, and
/// both are deterministic: `pulled`, the ranks the fusion consumed, and
/// `lookups`, the point queries it spent to get there. The claims that those
/// numbers are the right ones are made — and enforced — by
/// `tests/multimodal_read_bound.rs` and `tests/exclusion_lookup.rs`, which
/// compare two runs against each other rather than against a number.
///
/// `work` is the third number, and it is here because it is the one a headline
/// about materialised rows is a headline about: the rows the reads behind these
/// streams returned, summed over the strata, straight off the trailer.
fn shared_block_phase(label: &str, total: u64, rows: usize, basis: ExclusionBasis) {
    let pulls = Arc::new(AtomicUsize::new(0));
    let lookups = Arc::new(AtomicUsize::new(0));
    let (profile, streams) = shared_block_fixture(total, &pulls, &lookups, basis);
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

    // Read after the window closes: the trailer allocates a map per stratum and
    // that is reporting rather than fusing.
    let trailer = block_on(fusion.trailer()).expect("the fixture obeys the protocol");
    let counted: u64 = trailer
        .resolution
        .values()
        .map(|measured| measured.exclusion_lookups)
        .sum();
    let work: u64 = trailer
        .resolution
        .values()
        .filter_map(|measured| measured.rows_materialised)
        .sum();
    drop(fusion);

    black_box(contributions);
    report(
        &format!(
            "{label} fused={fused}/{rows} pulled={pulled} lookups={counted}/{served} work={work}",
            served = lookups.load(Ordering::Relaxed),
        ),
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
    // One declared block and three disjoint universes: the shape no declaration
    // can narrow, because both halves of every declaration are true and the
    // fact that is false is a fact about the corpus. Reported in pairs — the
    // same fixture with the lookup declared and withheld — because the only
    // reading of either number that means anything is the comparison.
    println!(
        "[fusion_frontier_alloc] --- one shared block, disjoint universes, rows fused fixed at 32 ---"
    );
    for total in [1_000_u64, 10_000, 100_000] {
        shared_block_phase(
            &format!("strata=3 rows=32 stream={total} shared silent"),
            total,
            32,
            ExclusionBasis::Unavailable,
        );
        shared_block_phase(
            &format!("strata=3 rows=32 stream={total} shared asking"),
            total,
            32,
            ExclusionBasis::Membership,
        );
    }
    // The length of the read behind a stream, against the rows the fusion
    // takes from it. The fusion consumes the same ranks whatever the stream
    // holds past them — that is the soundness claim — and `work` is what each
    // stream's read reports it cost, which is the number `pulled` alone cannot
    // show.
    println!(
        "[fusion_frontier_alloc] --- read length against rows fused, rows fused fixed at 32 ---"
    );
    for total in [35_u64, 1_000, 100_000] {
        shared_block_phase(
            &format!("strata=3 rows=32 read={total} read-length"),
            total,
            32,
            ExclusionBasis::Membership,
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
