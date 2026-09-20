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
//! # The window is per-thread
//!
//! `cargo test` runs a binary's tests concurrently on shared threads over one
//! `#[global_allocator]`, so a whole-process counter would be contaminated by a
//! sibling test's allocations between two snapshots. A [`CurrentThreadWindow`]
//! counts only the measuring thread — which is the whole of this measurement,
//! because `fuse` drives its streams on the polling thread and spawns nothing.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_retrieval::{
    CandidateDomains, DecayRule, DomainTag, DuplicatePolicy, Fixed, FusionProfile, Iri,
    ProducerReceipt, ProducerStatus, ProtocolError, RankFidelity, RankedRow, RankedStream,
    RowBlock, StreamContract, Term, TopK, contribution, fuse,
};

// ---------------------------------------------------------------------------
// The tracking allocator
// ---------------------------------------------------------------------------

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

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
    /// Which blocks of the candidate universe this stream declares it may name.
    ///
    /// The overlapping fixtures declare [`CandidateDomains::Unrestricted`],
    /// which is what they honestly are: every stratum emits the same universe
    /// permuted, so every stream really can name anything. The disjoint fixture
    /// declares one block per stratum, which is likewise what it is.
    domains: CandidateDomains,
    /// The block this stream draws its candidates from, or `None` for the
    /// overlapping fixture's shared, permuted universe.
    ///
    /// `Some` is what makes the candidate sets genuinely disjoint: each stratum
    /// mints its items under its own prefix, so no candidate is ever named
    /// twice and no confirmation ever arrives from another stratum. That is the
    /// shape the overlapping fixture cannot produce and the one the drain lived
    /// in.
    block: Option<usize>,
    /// The stratum weight this stream's contributions are computed at. Read from
    /// the fixture's own profile, so the rows always carry the value that
    /// profile would re-derive for them.
    weight: Fixed,
}

// The trait's methods are `async`; this fixture's body is synchronous because it
// mints its rows arithmetically. The keyword is the trait's, not a signal that
// the body awaits.
#[allow(clippy::unused_async_trait_impl)]
impl RankedStream for LazyStream {
    type Item = Term;

    async fn next(&mut self) -> Result<Option<RankedRow<Self::Item>>, ProtocolError> {
        if self.emitted >= self.total {
            return Ok(None);
        }
        self.emitted += 1;
        self.pulls.fetch_add(1, Ordering::SeqCst);
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
        // every row. It is read off this stream's own declaration rather than
        // stored twice: the disjoint fixture declares exactly one block per
        // stratum, so that block IS where each of its rows comes from, and the
        // overlapping fixture declares nothing and names nothing.
        let block = match self.domains.tags().and_then(|tags| tags.iter().next()) {
            Some(tag) => RowBlock::Declared(tag.clone()),
            None => RowBlock::Undeclared,
        };
        Ok(Some(RankedRow::new(rank, value, Term::new(item), block)))
    }

    async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError> {
        Ok(ProducerReceipt::Exhausted {
            rows_emitted: self.emitted,
        })
    }

    fn contract(&self) -> StreamContract {
        StreamContract::new(self.duplicates, RankFidelity::EXACT, self.domains.clone())
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
    weight: Fixed,
) -> (FusionProfile, Vec<(Iri, LazyStream)>) {
    let weights: BTreeMap<Iri, Fixed> = STRATA.iter().map(|name| (stratum(name), weight)).collect();
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
                    domains: CandidateDomains::Unrestricted,
                    block: None,
                    weight,
                },
            )
        })
        .collect();
    (profile, streams)
}

/// The caller-named block each stratum of the disjoint fixture draws from.
///
/// Three tags for three strata, in the same order as [`STRATA`]. Nothing here
/// mints them: they are `example.org` IRIs, exactly as the strata are.
const BLOCKS: [&str; 3] = [
    "http://example.org/domain/block-0",
    "http://example.org/domain/block-1",
    "http://example.org/domain/block-2",
];

/// The fixture profile and three streams whose candidate sets are DISJOINT,
/// each declaring the one block it draws from.
///
/// This is the shape the overlapping fixture above cannot produce and the one
/// the drain lived in: with no declaration, no candidate here is ever confirmed
/// by another stratum, so nothing certifies while another stream is open and a
/// bounded fusion reads every row of every stream. The declaration is what
/// makes the reading bounded, and the measurement below is what proves it —
/// peak bytes AND pulls, because either one alone can be flat while the other
/// tracks the input.
fn disjoint_fixture(
    total: u64,
    duplicates: DuplicatePolicy,
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
                    duplicates,
                    domains: CandidateDomains::within([DomainTag::parse(BLOCKS[stream_index])
                        .expect("the fixture block tags are valid IRIs")]),
                    block: Some(stream_index),
                    weight,
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
    /// How many rows the run certified.
    ///
    /// The precondition every peak *equality* below rests on, and it is
    /// asserted rather than assumed. The engine keeps one entry per row
    /// **emitted** — the record that makes `a fused answer never contains the
    /// same entity twice` unconditional — so two runs that certified different
    /// numbers of rows hold different numbers of entries, and an equality
    /// between their peaks would be an accident rather than the bound. Equal
    /// emission counts are what make "only the stream length differed" true.
    rows_certified: usize,
    /// How many rows were pulled from the three streams in total.
    pulls: usize,
    /// How many `(stratum, rank, contribution)` triples the certified rows
    /// carried, summed — the evidence the frontier was actually populated.
    contributions: usize,
    /// How many of the three strata the trailer closed at a contribution bound
    /// rather than at exhaustion — the evidence the bound stopped the reading
    /// and not merely the returning.
    bounded: usize,
    /// Adjacent ranks whose contributions the profile could not separate,
    /// summed across the three strata — the evidence a run really entered the
    /// regime where certification has to wait on a plateau.
    collisions: u64,
}

/// Fuse [`CERTIFIED_ROWS`] rows out of three streams of `total` rows each
/// through the shipped [`fuse`], measuring the peak heap it held while doing it.
fn measure(total: u64, duplicates: DuplicatePolicy) -> Measurement {
    measure_rows(total, CERTIFIED_ROWS, duplicates)
}

/// The same measurement at a weight whose contributions collide, so the fusion
/// is driven through the plateau regime rather than around it.
fn measure_at_weight(total: u64, duplicates: DuplicatePolicy, weight: Fixed) -> Measurement {
    measure_rows_at_weight(total, CERTIFIED_ROWS, duplicates, weight)
}

/// Fuse `rows` rows out of three streams of `total` rows each through the
/// shipped [`fuse`], measuring the peak heap it held while doing it.
fn measure_rows(total: u64, rows: usize, duplicates: DuplicatePolicy) -> Measurement {
    measure_rows_at_weight(total, rows, duplicates, Fixed::ONE)
}

/// Fuse `rows` rows out of three streams of `total` rows each, at `weight`,
/// through the shipped [`fuse`], measuring the peak heap it held.
///
/// # The first reading on a thread is discarded
///
/// A peak is the high-water mark of *live* bytes, so it charges whatever is
/// allocated inside the window whether or not the fusion is what allocated it.
/// The first fusion a thread performs pays one-time costs the second does not,
/// and those land inside the first measured window and nowhere else.
///
/// Every claim in this file is a COMPARISON between two readings, so a one-time
/// cost charged to whichever ran first is a difference that has nothing to do
/// with the quantity under test. It stays invisible while it is small relative
/// to an allocator size class and becomes a failure the moment the per-row
/// record grows enough to push the two readings onto opposite sides of one --
/// at which point a test about stream length reports on initialisation order
/// instead, and says so in the voice of the claim it was meant to check.
///
/// Discarding a first run puts both readings in the same steady state. It
/// weakens nothing: the equalities stay exact, and a fusion that really did hold
/// more for a longer stream still shows it, because the extra is charged to the
/// warmed run too.
fn measure_rows_at_weight(
    total: u64,
    rows: usize,
    duplicates: DuplicatePolicy,
    weight: Fixed,
) -> Measurement {
    let _warm_up = measure_rows_at_weight_once(total, rows, duplicates, weight);
    measure_rows_at_weight_once(total, rows, duplicates, weight)
}

/// One reading, taken however warm the thread happens to be.
fn measure_rows_at_weight_once(
    total: u64,
    rows: usize,
    duplicates: DuplicatePolicy,
    weight: Fixed,
) -> Measurement {
    let pulls = Arc::new(AtomicUsize::new(0));
    let (profile, streams) = fixture(total, duplicates, &pulls, weight);
    measure_built(&profile, streams, rows, &pulls)
}

/// The same measurement over three DISJOINT streams of `total` rows each, whose
/// producers declare the one block they draw from.
fn measure_disjoint_rows(total: u64, rows: usize, duplicates: DuplicatePolicy) -> Measurement {
    let pulls = Arc::new(AtomicUsize::new(0));
    let (profile, streams) = disjoint_fixture(total, duplicates, &pulls, Fixed::ONE);
    measure_built(&profile, streams, rows, &pulls)
}

/// Fuse `rows` rows out of `streams` through the shipped [`fuse`], measuring the
/// peak heap it held.
///
/// One measuring body for both fixtures, so the overlapping and disjoint claims
/// are measured by the same instrument and a difference between them is a
/// difference in the fusion rather than in how it was weighed.
fn measure_built(
    profile: &FusionProfile,
    streams: Vec<(Iri, LazyStream)>,
    rows: usize,
    pulls: &Arc<AtomicUsize>,
) -> Measurement {
    // The window opens after the fixture is built, so what is measured is the
    // fusion's own working set and not the fixture's.
    let window = CurrentThreadWindow::open();
    let result = block_on(fuse::<LazyStream, Term>(streams, profile, TopK::new(rows)))
        .expect("the fixture streams obey the protocol");
    let peak_bytes = window.close().peak_working_bytes;
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
    let collisions = result
        .trailer
        .resolution
        .values()
        .map(|measured| measured.collisions_observed)
        .sum::<u64>();
    let rows_certified = result.rows.len();
    // The peak was read before this: what is measured is what the *fusion* held,
    // not what the caller then chose to keep.
    drop(result);
    Measurement {
        peak_bytes,
        rows_certified,
        pulls,
        contributions,
        bounded,
        collisions,
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

    // And the precondition the equality below actually rests on: both runs
    // certified the same number of rows. The engine keeps one entry per row
    // *emitted* — the record that makes the answer's no-duplicate-entity
    // invariant unconditional rather than bounded by the frontier — so two runs
    // that emitted different numbers of rows would hold different numbers of
    // entries, and their peaks matching would be a coincidence rather than the
    // bound. Asserted before the measurement is read, not inferred from it.
    assert_eq!(
        short.rows_certified, CERTIFIED_ROWS,
        "the short run must certify the rows the comparison assumes"
    );
    assert_eq!(
        long.rows_certified, CERTIFIED_ROWS,
        "the long run must certify the rows the comparison assumes"
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
    //
    // The ceiling is a smell test against that two-orders-of-magnitude gap, not
    // a derived bound — the derived claim is the equality above, which is what
    // says the peak does not track the input. It is stated with room for the
    // per-row record to carry more than it does today: each row already holds
    // its score interval and the block it was drawn from, and a field added to
    // either moves this number by `CERTIFIED_ROWS` times its width while
    // changing nothing about what is being claimed.
    assert!(
        long.peak_bytes < 128 * 1024,
        "a bounded frontier should not cost {} bytes",
        long.peak_bytes
    );

    // The same claim under the other declaration, because the per-emitted-row
    // record is charged whatever a stream declared — it is the answer's
    // invariant, not a policy a producer can opt out of. `Allowed` above is the
    // harder case for the *per-stream identity set*; `Unique` is the case where
    // that set is absent and the emitted record is therefore the only thing
    // left that could have tracked the input. Both must be flat.
    let unique_short = measure(1_000, DuplicatePolicy::Unique);
    let unique_long = measure(1_000_000, DuplicatePolicy::Unique);
    assert_eq!(
        unique_short.rows_certified, CERTIFIED_ROWS,
        "the short `Unique` run must certify the rows the comparison assumes"
    );
    assert_eq!(
        unique_long.rows_certified, CERTIFIED_ROWS,
        "the long `Unique` run must certify the rows the comparison assumes"
    );
    assert!(
        unique_short.peak_bytes > 0,
        "the `Unique` fusion allocated nothing, so this measures nothing"
    );
    assert_eq!(
        unique_short.peak_bytes, unique_long.peak_bytes,
        "peak heap tracked the stream length under `Unique`: {} bytes over 1e3 \
         rows against {} over 1e6",
        unique_short.peak_bytes, unique_long.peak_bytes
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

/// **The bound survives the regime that removing a refusal made reachable.**
///
/// Fusion used to refuse a stream whose adjacent ranks carried one contribution.
/// That refusal is gone, and it was — accidentally — also the guard on this:
/// certification requires a candidate's score to be strictly above the threshold
/// over the stream heads, and on a plateau the next head carries the *same*
/// contribution, so nothing certifies while the plateau lasts and the frontier
/// accumulates. `select_emittable` already names exact ties as the thing that
/// would otherwise make the frontier grow with the input; quantization is a
/// second source of them, and unlike symmetric disagreement it arrives
/// systematically with depth.
///
/// So the claim the whole change rests on — that reading past the separating
/// depth costs resolution and *nothing else* — is measured here rather than
/// assumed. The weight is a thousand raw units, at which adjacent ranks collide
/// from the very first pair, so every certified row in this fixture is decided
/// inside the plateau regime.
#[test]
fn the_frontier_stays_bounded_past_the_collision() {
    // A thousand raw units is `10^-9`, not the number one thousand: at this
    // weight ranks one and two already share a contribution.
    let colliding = Fixed::from_raw(1_000);
    // Warm the lazy one-time allocations outside the measured window, at this
    // weight, exactly as the other measurements in this file do. Without it the
    // first window on this thread can be the one that pays a one-time initialization
    // — which window that is depends on how the test threads interleave, so the two
    // peaks below differed by one small allocation on some schedules and on none of
    // the others, and an equality that holds on most schedules is not a bound.
    let warm = measure_at_weight(1_000, DuplicatePolicy::Unique, colliding);
    assert!(warm.pulls > 0);
    let short = measure_at_weight(1_000, DuplicatePolicy::Unique, colliding);
    let long = measure_at_weight(1_000_000, DuplicatePolicy::Unique, colliding);

    // First, the crossing guard. Without it every assertion below could hold
    // because the run never entered the regime at all — the same false all-clear
    // a too-small row bound produces.
    assert!(
        short.collisions > 0 && long.collisions > 0,
        "this fixture must fuse inside the plateau regime, saw {} and {} collisions",
        short.collisions,
        long.collisions
    );

    // Second, the measurement is live, so the comparison is not vacuous — and
    // both runs emitted the same number of rows, which is the precondition the
    // equality rests on: the per-emitted-row record grows with emissions, so
    // unequal emission counts would make equal peaks an accident.
    assert_eq!(
        short.rows_certified, long.rows_certified,
        "the two runs must emit the same rows; only the streams' length differs"
    );
    assert_eq!(
        short.rows_certified, CERTIFIED_ROWS,
        "each run must certify the rows the comparison assumes"
    );
    assert!(
        short.peak_bytes > 0,
        "the fusion allocated nothing, so this measures nothing"
    );
    assert!(
        short.contributions > 0 && long.contributions > 0,
        "the certified rows carried no provenance, so the frontier was never populated"
    );

    // Third, the claim: a thousand-fold longer stream, the same certified rows,
    // and a peak that does not follow the input.
    assert_eq!(
        short.peak_bytes, long.peak_bytes,
        "peak heap tracked the stream length inside the plateau regime: {} bytes over 1e3 \
         rows against {} over 1e6 — the removed refusal was load-bearing after all",
        short.peak_bytes, long.peak_bytes
    );
    // The absolute figure, on the same terms as its two siblings: a smell test
    // against the tens of megabytes materializing a 1e6-row stream would cost,
    // not a derived bound. The derived claim is the equality directly above,
    // which is what says the peak does not track the input, and it is untouched.
    //
    // It was `64 * 1024` and the measurement sat 49 bytes under it. A margin of
    // 0.07% is not a smell test, it is a coincidence: the next per-row field
    // anyone adds trips it, for a reason that has nothing to do with the claim
    // the assertion makes, and the cheapest way out for whoever hits it is to
    // weaken the equality instead. Stated with room, it keeps saying "kilobytes,
    // not megabytes" — which is the only thing it was ever able to say.
    assert!(
        long.peak_bytes < 128 * 1024,
        "a bounded frontier should not cost {} bytes",
        long.peak_bytes
    );

    // And the reading is bounded too, not merely the returning: a run that
    // drained a million rows to answer thirty-two would have a bounded frontier
    // and an unbounded cost.
    assert_eq!(
        short.pulls, long.pulls,
        "the plateau must not make a longer stream get read further: {} pulls against {}",
        short.pulls, long.pulls
    );
    assert_eq!(
        long.bounded,
        STRATA.len(),
        "every stream still held rows, so each closes at a contribution bound"
    );
}

/// **The disjoint arm: a declared domain bounds the reading as well as the
/// frontier.**
///
/// Every other measurement in this file fuses strata that emit the same
/// universe permuted, which is the case where confirmations arrive early and
/// the reading is bounded whatever anybody declared. That is exactly why none
/// of them caught the drain: over strata whose candidate sets do not overlap,
/// no confirmation ever arrives, nothing certifies while another stream is
/// open, and a thirty-two-row answer reads every row of every stream. The rows
/// stay bounded and the reading does not, so a peak measured over the
/// overlapping fixture reports a success.
///
/// So this arm measures the shape that failed: three disjoint streams, each
/// declaring the one block it draws from, at two lengths three orders of
/// magnitude apart. Both quantities are asserted, because either alone can be
/// flat while the other tracks the input — a fusion that drained but held
/// nothing would show a flat peak, and a fusion that read little but retained
/// everything would show flat pulls.
#[test]
fn disjoint_strata_declaring_their_blocks_read_and_hold_the_same_at_any_length() {
    // Warm the lazy one-time allocations outside the measured window, exactly
    // as the overlapping measurements do.
    let warm = measure_disjoint_rows(1_000, CERTIFIED_ROWS, DuplicatePolicy::Unique);
    assert!(warm.pulls > 0);

    let short = measure_disjoint_rows(1_000, CERTIFIED_ROWS, DuplicatePolicy::Unique);
    let long = measure_disjoint_rows(1_000_000, CERTIFIED_ROWS, DuplicatePolicy::Unique);

    // The preconditions, asserted before the equalities that rest on them: both
    // runs certified the same rows, and each row carries exactly one
    // contribution, because the strata are disjoint and no candidate is named
    // twice. A run where some candidate had two contributions would not be the
    // disjoint fixture at all.
    assert_eq!(short.rows_certified, CERTIFIED_ROWS);
    assert_eq!(long.rows_certified, CERTIFIED_ROWS);
    assert_eq!(
        short.contributions, CERTIFIED_ROWS,
        "the disjoint fixture's candidates are named by exactly one stratum each"
    );
    assert_eq!(long.contributions, CERTIFIED_ROWS);
    assert!(
        short.peak_bytes > 0,
        "the fusion allocated nothing, so this measures nothing"
    );

    // The reading. This is the assertion the defect fails: without the
    // declaration the long run pulls three million rows here and the short one
    // three thousand.
    assert_eq!(
        short.pulls, long.pulls,
        "a thousand-fold longer disjoint stream must not be read further: {} \
         pulls against {}",
        short.pulls, long.pulls
    );
    assert!(
        short.pulls < 1_000,
        "a thirty-two-row answer over disjoint strata must not read a thousand \
         rows, read {}",
        short.pulls
    );

    // And the holding, which the reading bound is worth nothing without.
    assert_eq!(
        short.peak_bytes, long.peak_bytes,
        "peak heap tracked the disjoint streams' length: {} bytes over 1e3 rows \
         against {} over 1e6",
        short.peak_bytes, long.peak_bytes
    );
    // And in absolute terms it is kilobytes rather than megabytes. The
    // equality above is the exact claim; this one is the order of magnitude,
    // stated with room to spare because it is about the difference between a
    // bounded working set and a materialized one — three million candidate
    // terms would be tens of megabytes.
    assert!(
        long.peak_bytes < 128 * 1024,
        "a bounded frontier should not cost {} bytes",
        long.peak_bytes
    );

    // Every stratum still held rows, so each closes at the contribution it was
    // read down to. A stratum reported `Exhausted` here would be one that had
    // been drained to earn the word.
    assert_eq!(
        long.bounded,
        STRATA.len(),
        "a disjoint stratum stopped by the bound must report the bound, not \
         exhaustion it would have read a million rows to claim"
    );
}
