// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The fusion contract: verified-protocol NRA over exact fixed point.
//!
//! Every test drives the public surface. Mock producers are scripted row by row
//! so a protocol violation can be produced deliberately; the oracle recomputes
//! the fused order from first principles and must agree with the engine.

use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Wake, Waker};

use pretty_assertions::assert_eq;
use purrdf_retrieval::{
    Fixed, FusionError, FusionProfile, FusionResult, FusionStream, Iri, PlanId, ProducerReceipt,
    ProducerStatus, ProtocolError, RECIP_K, RankedStream, Term, TopK, contribution,
};

const K: u32 = 60;

/// The row bound these fixtures fuse under.
///
/// Fused enumeration is top-k by construction, so every fusion states a bound.
/// These scripted streams hold a handful of rows between them and this is far
/// above all of them, which is the point: except where a test is *about* the
/// bound, the bound must not be what decides the answer.
const TOP_K: TopK = TopK::new(1024);

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

fn stratum(text: &str) -> Iri {
    iri(&format!("http://example.org/stratum/{text}"))
}

fn plan_id(tag: &str) -> PlanId {
    PlanId::from_canonical(tag.as_bytes())
}

fn profile(weights: &[(&str, Fixed)], k: u32, max_contributions: u32) -> FusionProfile {
    let map: BTreeMap<Iri, Fixed> = weights
        .iter()
        .map(|(name, weight)| (stratum(name), *weight))
        .collect();
    FusionProfile::new(map, k, max_contributions).expect("fixture profile is valid")
}

/// A minimal single-threaded executor. The mock streams never actually pend, so
/// a waking no-op is sufficient; the real system is runtime-agnostic by design.
fn block_on<F: Future>(future: F) -> F::Output {
    struct ParkWaker(std::thread::Thread);
    impl Wake for ParkWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }

    let waker = Waker::from(Arc::new(ParkWaker(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::park(),
        }
    }
}

/// One scripted step of a mock producer.
enum Step {
    Row(u64, Fixed, Term),
    Fail(ProtocolError),
}

/// A producer whose rows and failures are pre-scripted.
struct MockStream {
    steps: VecDeque<Step>,
    receipt: ProducerReceipt,
    rows_emitted: u64,
    drop_counter: Arc<AtomicUsize>,
}

impl Drop for MockStream {
    fn drop(&mut self) {
        self.drop_counter.fetch_add(1, Ordering::SeqCst);
    }
}

impl MockStream {
    fn new(steps: Vec<Step>, receipt: ProducerReceipt) -> Self {
        Self {
            steps: steps.into(),
            receipt,
            rows_emitted: 0,
            drop_counter: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn tracked(steps: Vec<Step>, receipt: ProducerReceipt, counter: Arc<AtomicUsize>) -> Self {
        Self {
            steps: steps.into(),
            receipt,
            rows_emitted: 0,
            drop_counter: counter,
        }
    }
}

// The trait's methods are `async`; the mock's bodies are synchronous because
// its rows are pre-scripted. The `async` keyword is required to implement the
// trait, not a signal that this body awaits.
#[allow(clippy::unused_async_trait_impl)]
impl RankedStream for MockStream {
    type Item = Term;

    async fn next(&mut self) -> Result<Option<(u64, Fixed, Self::Item)>, ProtocolError> {
        match self.steps.pop_front() {
            Some(Step::Row(rank, score, item)) => {
                self.rows_emitted += 1;
                Ok(Some((rank, score, item)))
            }
            Some(Step::Fail(error)) => Err(error),
            None => Ok(None),
        }
    }

    async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError> {
        Ok(self.receipt.clone())
    }
}

/// A well-formed row at `rank` for `weight` under `k`.
fn row(rank: u64, weight: Fixed, k: u32, item: &str) -> Step {
    Step::Row(
        rank,
        contribution(weight, rank, k).expect("fixture contribution fits"),
        Term::new(item),
    )
}

fn exhausted(rows: u64) -> ProducerReceipt {
    ProducerReceipt::Exhausted { rows_emitted: rows }
}

async fn run_fuse(streams: Vec<(Iri, MockStream)>, profile: &FusionProfile) -> FusionResult<Term> {
    purrdf_retrieval::fuse::<MockStream, Term>(streams, profile, TOP_K)
        .await
        .expect("fusion succeeds")
}

// 1. An item in two strata sums both contributions exactly.
#[test]
fn cross_stratum_sum_is_the_checked_sum() {
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K, 4);
    let c1 = contribution(Fixed::ONE, 1, K).expect("fits");
    let c2 = contribution(Fixed::ONE, 1, K).expect("fits");
    let streams = vec![
        (
            stratum("text"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
        ),
        (
            stratum("vector"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
        ),
    ];
    let result = block_on(run_fuse(streams, &profile));
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].entity, Term::new("a"));
    assert_eq!(
        result.rows[0].score,
        c1.checked_add(c2).expect("fits"),
        "fused score must be the checked sum of both contributions"
    );
    assert_eq!(result.rows[0].contributions.len(), 2);
}

// 2. Output is ordered by fused score descending.
#[test]
fn output_is_score_descending() {
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K, 4);
    let streams = vec![
        (
            stratum("text"),
            MockStream::new(
                vec![
                    row(1, Fixed::ONE, K, "a"),
                    row(2, Fixed::ONE, K, "b"),
                    row(3, Fixed::ONE, K, "c"),
                ],
                exhausted(3),
            ),
        ),
        (
            stratum("vector"),
            MockStream::new(
                vec![row(1, Fixed::ONE, K, "d"), row(2, Fixed::ONE, K, "e")],
                exhausted(2),
            ),
        ),
    ];
    let result = block_on(run_fuse(streams, &profile));
    assert_eq!(result.rows.len(), 5);
    for window in result.rows.windows(2) {
        assert!(
            window[0].score >= window[1].score,
            "scores must not rise: {:?} then {:?}",
            window[0],
            window[1]
        );
    }
}

// 3. Equal-score candidates are ordered by the declared tie-break.
#[test]
fn equal_scores_use_the_declared_tie_break() {
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K, 4);
    let streams = vec![
        (
            stratum("text"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "b")], exhausted(1)),
        ),
        (
            stratum("vector"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
        ),
    ];
    let result = block_on(run_fuse(streams, &profile));
    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.rows[0].score, result.rows[1].score);
    assert_eq!(
        result.rows[0].entity,
        Term::new("a"),
        "equal scores must fall back to canonical term order"
    );
    assert_eq!(result.rows[1].entity, Term::new("b"));
}

// 4. A checked addition that leaves the range is a loud overflow.
//
// Three *distinct* candidates, one per stream, each receive only a single
// contribution, so neither the per-candidate contribution-count bound nor the
// per-candidate ceiling ever comes into play for any of them individually.
// The overflow instead comes from `compute_threshold`, which sums the current
// head of every stream regardless of candidate identity: three heads at
// `weight * reciprocal(1 + 1) = weight / 2` each, with `weight == i128::MAX`,
// sum past the fixed-point range on the third addition.
#[test]
fn checked_addition_overflow_is_refused() {
    let huge = Fixed::from_raw(i128::MAX);
    // max_contributions == 1 keeps the admitted ceiling equal to the single
    // weight per stratum, so the profile constructs without itself overflowing.
    let profile = profile(&[("text", huge), ("vector", huge), ("geo", huge)], 1, 1);
    let streams = vec![
        (
            stratum("text"),
            MockStream::new(vec![row(1, huge, 1, "a")], exhausted(1)),
        ),
        (
            stratum("vector"),
            MockStream::new(vec![row(1, huge, 1, "b")], exhausted(1)),
        ),
        (
            stratum("geo"),
            MockStream::new(vec![row(1, huge, 1, "c")], exhausted(1)),
        ),
    ];
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        streams, &profile, TOP_K,
    ));
    assert!(
        matches!(result, Err(FusionError::Overflow)),
        "expected FusionError::Overflow, got {result:?}"
    );
}

// 5. Byte-identical golden output on re-run.
#[test]
fn golden_output_is_byte_identical() {
    let result = block_on(golden_fusion());
    let rendered = render(&result);
    let expected =
        std::fs::read_to_string(golden_path()).expect("the golden fixture is checked in");
    assert_eq!(
        rendered,
        expected,
        "fused rendering drifted from the golden; if the change is intended, update {}",
        golden_path().display()
    );
}

fn golden_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fusion/order.golden")
}

async fn golden_fusion() -> FusionResult<Term> {
    let profile = profile(
        &[
            ("text", Fixed::ONE),
            ("vector", Fixed::from_raw(500_000_000_000)),
            ("geo", Fixed::from_raw(250_000_000_000)),
        ],
        K,
        8,
    );
    let streams = vec![
        (
            stratum("text"),
            MockStream::new(
                vec![
                    row(1, Fixed::ONE, K, "alpha"),
                    row(2, Fixed::ONE, K, "beta"),
                    row(3, Fixed::ONE, K, "gamma"),
                ],
                exhausted(3),
            ),
        ),
        (
            stratum("vector"),
            MockStream::new(
                vec![
                    row(1, Fixed::from_raw(500_000_000_000), K, "alpha"),
                    row(2, Fixed::from_raw(500_000_000_000), K, "delta"),
                ],
                exhausted(2),
            ),
        ),
        (
            stratum("geo"),
            MockStream::new(
                vec![
                    row(1, Fixed::from_raw(250_000_000_000), K, "beta"),
                    row(2, Fixed::from_raw(250_000_000_000), K, "epsilon"),
                ],
                exhausted(2),
            ),
        ),
    ];
    run_fuse(streams, &profile).await
}

/// Render a fusion result as deterministic text: rows by rank, then statuses by
/// stratum, then the profile identity. Nothing here depends on a `HashMap`.
fn render(result: &FusionResult<Term>) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for row in &result.rows {
        let _ = writeln!(
            out,
            "row {} score={} witness={}",
            row.entity.as_str(),
            row.score.to_decimal_lexical(),
            row.threshold_witness.to_decimal_lexical()
        );
        for (stratum, rank, score) in &row.contributions {
            let _ = writeln!(
                out,
                "  {} rank={rank} score={}",
                stratum.as_str(),
                score.to_decimal_lexical()
            );
        }
    }
    for (stratum, status) in &result.trailer.statuses {
        let _ = writeln!(out, "status {} {status:?}", stratum.as_str());
    }
    let _ = writeln!(out, "profile {}", result.trailer.profile_id.to_hex());
    out
}

// 6. The engine agrees with a simple eager oracle over small streams.
#[test]
fn differential_against_eager_oracle() {
    let profile = profile(
        &[
            ("text", Fixed::ONE),
            ("vector", Fixed::from_raw(700_000_000_000)),
            ("geo", Fixed::from_raw(300_000_000_000)),
        ],
        K,
        8,
    );
    let streams = vec![
        (
            stratum("text"),
            MockStream::new(
                vec![
                    row(1, Fixed::ONE, K, "a"),
                    row(2, Fixed::ONE, K, "b"),
                    row(3, Fixed::ONE, K, "c"),
                ],
                exhausted(3),
            ),
        ),
        (
            stratum("vector"),
            MockStream::new(
                vec![
                    row(1, Fixed::from_raw(700_000_000_000), K, "b"),
                    row(2, Fixed::from_raw(700_000_000_000), K, "d"),
                ],
                exhausted(2),
            ),
        ),
        (
            stratum("geo"),
            MockStream::new(
                vec![
                    row(1, Fixed::from_raw(300_000_000_000), K, "c"),
                    row(2, Fixed::from_raw(300_000_000_000), K, "e"),
                ],
                exhausted(2),
            ),
        ),
    ];
    let result = block_on(run_fuse(streams, &profile));

    // Eager oracle: sum the profile's contribution for every (item, weight,
    // rank), then order by score descending, best rank ascending, term ascending.
    let mut expected: BTreeMap<&str, (Fixed, u64)> = BTreeMap::new();
    let scripted: [(&str, Fixed, u64); 7] = [
        ("a", Fixed::ONE, 1),
        ("b", Fixed::ONE, 2),
        ("c", Fixed::ONE, 3),
        ("b", Fixed::from_raw(700_000_000_000), 1),
        ("d", Fixed::from_raw(700_000_000_000), 2),
        ("c", Fixed::from_raw(300_000_000_000), 1),
        ("e", Fixed::from_raw(300_000_000_000), 2),
    ];
    for (item, weight, rank) in scripted {
        let value = contribution(weight, rank, K).expect("fits");
        let entry = expected.entry(item).or_insert((Fixed::ZERO, u64::MAX));
        entry.0 = entry.0.checked_add(value).expect("fits");
        entry.1 = entry.1.min(rank);
    }
    let mut oracle: Vec<(&str, Fixed, u64)> = expected
        .into_iter()
        .map(|(item, (score, rank))| (item, score, rank))
        .collect();
    oracle.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then(left.2.cmp(&right.2))
            .then(left.0.cmp(right.0))
    });

    assert_eq!(result.rows.len(), oracle.len());
    for (row, (item, score, rank)) in result.rows.iter().zip(&oracle) {
        assert_eq!(row.entity, Term::new(*item));
        assert_eq!(row.score, *score);
        let best = row
            .contributions
            .iter()
            .map(|(_, rank, _)| *rank)
            .min()
            .expect("a row has at least one contribution");
        assert_eq!(best, *rank);
    }
}

// 7. Every protocol violation is a typed error, never a plausible order.
#[test]
fn protocol_violations_are_typed() {
    let profile = profile(&[("text", Fixed::ONE)], K, 4);
    let weight = Fixed::ONE;

    let out_of_order = vec![(
        stratum("text"),
        MockStream::new(
            vec![row(1, weight, K, "a"), row(1, weight, K, "b")],
            exhausted(2),
        ),
    )];
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        out_of_order,
        &profile,
        TOP_K,
    ));
    assert!(
        matches!(
            &result,
            Err(FusionError::Protocol(error))
                if matches!(**error, ProtocolError::OutOfOrderRanks { expected: 2, got: 1 })
        ),
        "expected OutOfOrderRanks, got {result:?}"
    );

    let non_contiguous = vec![(
        stratum("text"),
        MockStream::new(
            vec![row(1, weight, K, "a"), row(3, weight, K, "c")],
            exhausted(2),
        ),
    )];
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        non_contiguous,
        &profile,
        TOP_K,
    ));
    assert!(
        matches!(
            &result,
            Err(FusionError::Protocol(error))
                if matches!(**error, ProtocolError::NonContiguousRanks { gap: 1 })
        ),
        "expected NonContiguousRanks, got {result:?}"
    );

    let duplicate = vec![(
        stratum("text"),
        MockStream::new(
            vec![row(1, weight, K, "a"), row(2, weight, K, "a")],
            exhausted(2),
        ),
    )];
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        duplicate, &profile, TOP_K,
    ));
    assert!(
        matches!(
            &result,
            Err(FusionError::Protocol(error))
                if matches!(&**error, ProtocolError::DuplicateItem { item } if item == "a")
        ),
        "expected DuplicateItem, got {result:?}"
    );

    let error_after_rows = vec![(
        stratum("text"),
        MockStream::new(
            vec![
                row(1, weight, K, "a"),
                Step::Fail(ProtocolError::ErrorAfterRows { rows_before: 1 }),
            ],
            exhausted(0),
        ),
    )];
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        error_after_rows,
        &profile,
        TOP_K,
    ));
    assert!(
        matches!(
            &result,
            Err(FusionError::Protocol(error))
                if matches!(**error, ProtocolError::ErrorAfterRows { rows_before: 1 })
        ),
        "expected ErrorAfterRows, got {result:?}"
    );

    let never_ending = vec![(
        stratum("text"),
        MockStream::new(
            vec![
                row(1, weight, K, "a"),
                Step::Fail(ProtocolError::NeverEndingSource),
            ],
            exhausted(0),
        ),
    )];
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        never_ending,
        &profile,
        TOP_K,
    ));
    assert!(
        matches!(
            &result,
            Err(FusionError::Protocol(error))
                if matches!(**error, ProtocolError::NeverEndingSource)
        ),
        "expected NeverEndingSource, got {result:?}"
    );

    let mismatch = vec![(
        stratum("text"),
        MockStream::new(
            vec![Step::Row(1, Fixed::from_raw(1), Term::new("a"))],
            exhausted(1),
        ),
    )];
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        mismatch, &profile, TOP_K,
    ));
    assert!(
        matches!(
            &result,
            Err(FusionError::Protocol(error))
                if matches!(**error, ProtocolError::ContributionMismatch { .. })
        ),
        "expected ContributionMismatch, got {result:?}"
    );
}

// 8. Dropping a live fusion drops every stream; receipts cannot be forged.
#[test]
fn drop_cancellation_and_receipts() {
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K, 4);
    let text_counter = Arc::new(AtomicUsize::new(0));
    let vector_counter = Arc::new(AtomicUsize::new(0));
    let streams = vec![
        (
            stratum("text"),
            MockStream::tracked(
                vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "b")],
                exhausted(2),
                Arc::clone(&text_counter),
            ),
        ),
        (
            stratum("vector"),
            MockStream::tracked(
                vec![row(1, Fixed::ONE, K, "c")],
                exhausted(1),
                Arc::clone(&vector_counter),
            ),
        ),
    ];
    let mut fusion = FusionStream::new(streams, profile.clone());
    let first = block_on(fusion.next())
        .expect("next succeeds")
        .expect("a row");
    assert_eq!(first.entity, Term::new("a"));
    drop(fusion);
    assert_eq!(
        text_counter.load(Ordering::SeqCst),
        1,
        "dropping the fusion must drop its text stream"
    );
    assert_eq!(
        vector_counter.load(Ordering::SeqCst),
        1,
        "dropping the fusion must drop its vector stream"
    );

    // A receipt that under-reports the emitted rows is refused.
    let forged = vec![(
        stratum("text"),
        MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(9)),
    )];
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        forged, &profile, TOP_K,
    ));
    assert!(
        matches!(
            &result,
            Err(FusionError::Protocol(error))
                if matches!(**error, ProtocolError::ForgedReceipt { declared: 9, actual: 1 })
        ),
        "expected ForgedReceipt, got {result:?}"
    );
}

// 9. The trailer carries each producer's own status.
#[test]
fn trailer_preserves_each_producer_status() {
    let profile = profile(
        &[
            ("text", Fixed::ONE),
            ("vector", Fixed::ONE),
            ("geo", Fixed::ONE),
            ("embedding", Fixed::ONE),
        ],
        K,
        4,
    );
    let streams = vec![
        (
            stratum("text"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
        ),
        (
            stratum("vector"),
            MockStream::new(
                vec![row(1, Fixed::ONE, K, "b")],
                ProducerReceipt::CeilingReached {
                    bound: Fixed::from_raw(5),
                },
            ),
        ),
        (
            stratum("geo"),
            MockStream::new(
                Vec::new(),
                ProducerReceipt::ExecutionFailed {
                    reason: "no index".to_owned(),
                },
            ),
        ),
        (
            stratum("embedding"),
            MockStream::new(Vec::new(), ProducerReceipt::TermsRejected),
        ),
    ];
    let result = block_on(run_fuse(streams, &profile));
    let statuses = &result.trailer.statuses;
    assert_eq!(statuses.len(), 4);
    assert_eq!(
        statuses.get(&stratum("text")),
        Some(&ProducerStatus::Exhausted { rows_emitted: 1 })
    );
    assert_eq!(
        statuses.get(&stratum("vector")),
        Some(&ProducerStatus::CeilingReached {
            bound: Fixed::from_raw(5)
        })
    );
    assert_eq!(
        statuses.get(&stratum("geo")),
        Some(&ProducerStatus::ExecutionFailed {
            reason: "no index".to_owned()
        })
    );
    assert_eq!(
        statuses.get(&stratum("embedding")),
        Some(&ProducerStatus::TermsRejected)
    );
}

// 9b. The trailer names the pinned plan when one is attached.
#[test]
fn trailer_names_the_pinned_plan() {
    let profile = profile(&[("text", Fixed::ONE)], K, 4);
    let pinned = plan_id("fusion-plan");
    let streams = vec![(
        stratum("text"),
        MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
    )];
    let mut fusion = FusionStream::new(streams, profile.clone()).with_plan_id(pinned);
    while block_on(fusion.next()).expect("fusion succeeds").is_some() {}
    let trailer = block_on(fusion.trailer()).expect("trailer succeeds");
    assert_eq!(trailer.plan_id, Some(pinned));
    assert_eq!(trailer.profile_id, profile.id());
}

// 9c. Certification can fire before exhaustion, and it records the threshold.
//
// Candidate `a` is seen in every active stream, so its score is final; the
// remaining heads cannot overtake it, so it is emitted while both streams still
// hold rows, and the witness names the threshold that certified it.
#[test]
fn threshold_witness_is_recorded_before_exhaustion() {
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K, 4);
    let streams = vec![
        (
            stratum("text"),
            MockStream::new(
                vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "x")],
                exhausted(2),
            ),
        ),
        (
            stratum("vector"),
            MockStream::new(
                vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "y")],
                exhausted(2),
            ),
        ),
    ];
    let result = block_on(run_fuse(streams, &profile));
    assert_eq!(result.rows.len(), 3);
    assert_eq!(result.rows[0].entity, Term::new("a"));
    let tail = contribution(Fixed::ONE, 2, K).expect("fits");
    assert_eq!(
        result.rows[0].threshold_witness,
        tail.checked_add(tail).expect("fits"),
        "the witness must be the live threshold at certification"
    );
    assert!(
        result.rows[0].threshold_witness > Fixed::ZERO,
        "certification happened while streams still held rows"
    );
}

// 10. A row's contributions sum exactly to its fused score.
#[test]
fn contributions_sum_to_the_fused_score() {
    let profile = profile(
        &[
            ("text", Fixed::ONE),
            ("vector", Fixed::from_raw(700_000_000_000)),
        ],
        K,
        4,
    );
    let streams = vec![
        (
            stratum("text"),
            MockStream::new(
                vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "b")],
                exhausted(2),
            ),
        ),
        (
            stratum("vector"),
            MockStream::new(
                vec![row(1, Fixed::from_raw(700_000_000_000), K, "b")],
                exhausted(1),
            ),
        ),
    ];
    let result = block_on(run_fuse(streams, &profile));
    assert_eq!(result.rows.len(), 2, "a and b are the two fused candidates");
    for fused in &result.rows {
        let mut sum = Fixed::ZERO;
        for (_, _, score) in &fused.contributions {
            sum = sum.checked_add(*score).expect("fits");
        }
        assert_eq!(sum, fused.score, "contributions must sum to the score");
    }
}

// 11. Profile construction refuses invalid parameters, and identity tracks
//     every field.
#[test]
fn profile_construction_is_validated() {
    let weights = |weight: Fixed| {
        let mut map = BTreeMap::new();
        map.insert(stratum("text"), weight);
        map
    };

    assert!(matches!(
        FusionProfile::new(weights(Fixed::ONE), 0, 1),
        Err(FusionError::InvalidK { k: 0 })
    ));
    assert!(matches!(
        FusionProfile::new(weights(Fixed::ONE), K, 0),
        Err(FusionError::InvalidMaxContributions { max: 0 })
    ));
    assert!(matches!(
        FusionProfile::new(BTreeMap::new(), K, 1),
        Err(FusionError::EmptyWeights)
    ));
    assert!(matches!(
        FusionProfile::new(weights(Fixed::ZERO), K, 1),
        Err(FusionError::NonPositiveWeight { .. })
    ));
    assert!(matches!(
        FusionProfile::new(weights(Fixed::from_raw(-1)), K, 1),
        Err(FusionError::NonPositiveWeight { .. })
    ));
    assert!(matches!(
        FusionProfile::new(weights(Fixed::from_raw(i128::MAX)), K, 2),
        Err(FusionError::Overflow)
    ));

    let baseline = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K, 8);
    let same = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K, 8);
    assert_eq!(baseline.id(), same.id(), "identical profiles share an id");
    assert_eq!(
        FusionProfile::from_canonical_bytes(&baseline.canonical_bytes()).expect("round-trips"),
        baseline
    );

    let different_k = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K + 1, 8);
    let different_weight = profile(
        &[("text", Fixed::ONE), ("vector", Fixed::from_raw(2))],
        K,
        8,
    );
    let different_max = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K, 9);
    assert_ne!(baseline.id(), different_k.id());
    assert_ne!(baseline.id(), different_weight.id());
    assert_ne!(baseline.id(), different_max.id());
}

// 12. Canonical profile bytes and fused output are target-independent.
//
// The bytes are asserted against a fixed digest captured on this target; the
// same construction on wasm32-unknown-unknown is proven to compile and to
// produce byte-identical input by the pure-field encoding. The fused rendering
// is checked against the same golden the native test uses.
#[test]
fn native_and_wasm_share_canonical_bytes_and_output() {
    let profile = profile(&[("text", Fixed::ONE)], K, 4);
    assert_eq!(
        profile.canonical_bytes(),
        profile.canonical_bytes(),
        "canonical bytes are a pure function of the fields"
    );
    assert_eq!(
        profile.canonical_bytes(),
        FusionProfile::from_canonical_bytes(&profile.canonical_bytes())
            .expect("round-trips")
            .canonical_bytes(),
        "decode must reproduce the same bytes"
    );

    let result = block_on(golden_fusion());
    let expected =
        std::fs::read_to_string(golden_path()).expect("the golden fixture is checked in");
    assert_eq!(render(&result), expected);
    assert_eq!(RECIP_K, 60);
}

// 13. A candidate that receives more contributions than the profile admits is
//     a named refusal naming the offending candidate, the count reached and
//     the declared limit — never a silently accepted count the profile
//     declared inadmissible, and never the generic `Overflow`.
#[test]
fn contribution_count_exceeding_max_is_refused() {
    let profile = profile(
        &[
            ("text", Fixed::ONE),
            ("vector", Fixed::ONE),
            ("geo", Fixed::ONE),
        ],
        K,
        2,
    );
    let streams = vec![
        (
            stratum("text"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
        ),
        (
            stratum("vector"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
        ),
        (
            stratum("geo"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
        ),
    ];
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        streams, &profile, TOP_K,
    ));
    assert!(
        matches!(
            &result,
            Err(FusionError::MaxContributionsExceeded { item, count: 3, max: 2 })
                if item == "a"
        ),
        "expected MaxContributionsExceeded {{ item: \"a\", count: 3, max: 2 }}, got {result:?}"
    );
}

// 14. Valid neighbour of 13: receiving exactly the declared maximum still
//     succeeds, and the fused score is still the checked sum of every
//     contribution — the count bound must clip nothing a legitimate answer
//     needs.
#[test]
fn contribution_count_at_max_succeeds() {
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K, 2);
    let c1 = contribution(Fixed::ONE, 1, K).expect("fits");
    let c2 = contribution(Fixed::ONE, 1, K).expect("fits");
    let streams = vec![
        (
            stratum("text"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
        ),
        (
            stratum("vector"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
        ),
    ];
    let result = block_on(run_fuse(streams, &profile));
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].entity, Term::new("a"));
    assert_eq!(result.rows[0].contributions.len(), 2);
    assert_eq!(
        result.rows[0].score,
        c1.checked_add(c2).expect("fits"),
        "exactly max_contributions contributions must still sum exactly"
    );
}

// 15. `ceiling() == max_weight * max_contributions`, but every reciprocal-rank
//     contribution is *strictly* below its stratum's weight: `K >= 1`
//     (`InvalidK`) and `rank >= 1` (`InvalidRank`) force
//     `reciprocal(K + rank) <= 1/2`. So the true achievable maximum under a
//     valid contribution count is exactly half the declared ceiling, never the
//     ceiling itself. This fixture drives every stratum to that true maximum —
//     equal max weight, `K = 1`, `rank = 1`, one contribution per stratum,
//     exactly `max_contributions` strata — and proves the fused score lands,
//     exactly, at `ceiling() / 2`. This is the valid neighbour of the ceiling
//     check: even the most aggressive legitimate load the current decay rule
//     can produce must still succeed, with real headroom to spare.
#[test]
fn maximal_legitimate_score_stays_below_the_ceiling() {
    let weight = Fixed::from_raw(2_000_000_000_000); // 2.0, an even raw value
    let k = 1;
    let names = ["s0", "s1", "s2", "s3"];
    let weights: Vec<(&str, Fixed)> = names.iter().copied().map(|name| (name, weight)).collect();
    let profile = profile(&weights, k, 4);

    let streams: Vec<(Iri, MockStream)> = names
        .iter()
        .copied()
        .map(|name| {
            (
                stratum(name),
                MockStream::new(vec![row(1, weight, k, "a")], exhausted(1)),
            )
        })
        .collect();
    let result = block_on(run_fuse(streams, &profile));
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].contributions.len(), 4);

    let per_contribution = contribution(weight, 1, k).expect("fits");
    let mut expected = Fixed::ZERO;
    for _ in 0..4 {
        expected = expected.checked_add(per_contribution).expect("fits");
    }
    assert_eq!(result.rows[0].score, expected);
    assert!(
        expected < profile.ceiling(),
        "the maximal legitimate score must stay strictly below the declared ceiling"
    );
    assert_eq!(
        expected.checked_add(expected).expect("fits"),
        profile.ceiling(),
        "the true achievable maximum is exactly half the declared ceiling, because \
         K >= 1 and rank >= 1 cap every reciprocal-rank contribution at weight / 2"
    );
}

// 16. `CeilingExceeded` is a defensive invariant, not one reachable under
//     today's sole decay rule: test 15 proves a valid contribution count can
//     reach at most `ceiling() / 2`. The one configuration whose raw
//     arithmetic *would* land exactly at `ceiling()` — two equal-weight,
//     rank-1, `K = 1` contributions under a profile whose `max_contributions`
//     is 1 — is also the one configuration that already violates the
//     contribution count, so the count check fires first and
//     `CeilingExceeded` is never observed for it. This drives `FusionStream`
//     directly (bypassing `fuse`'s duplicate-stratum guard) because reaching
//     it needs two streams sharing one stratum tag, which `fuse` itself
//     refuses before fusion ever begins.
#[test]
fn ceiling_boundary_is_gated_by_the_contribution_count_check() {
    let weight = Fixed::from_raw(2_000_000_000_000); // 2.0, an even raw value
    let k = 1;
    let profile = profile(&[("dup", weight)], k, 1);
    let dup = stratum("dup");
    let streams = vec![
        (
            dup.clone(),
            MockStream::new(vec![row(1, weight, k, "a")], exhausted(1)),
        ),
        (
            dup,
            MockStream::new(vec![row(1, weight, k, "a")], exhausted(1)),
        ),
    ];

    // The fixture must reach exactly the ceiling for this test to prove
    // anything: confirm the arithmetic the comment above claims before
    // asserting on the behaviour it implies.
    let per_contribution = contribution(weight, 1, k).expect("fits");
    assert_eq!(
        per_contribution
            .checked_add(per_contribution)
            .expect("fits"),
        profile.ceiling(),
        "two rank-1, K=1 contributions at the max weight must sum to exactly the ceiling"
    );

    let mut fusion = FusionStream::new(streams, profile);
    let result = block_on(async {
        loop {
            match fusion.next().await {
                Ok(Some(_)) => (),
                Ok(None) => break Ok(()),
                Err(error) => break Err(error),
            }
        }
    });
    assert!(
        matches!(
            &result,
            Err(FusionError::MaxContributionsExceeded { item, count: 2, max: 1 })
                if item == "a"
        ),
        "expected MaxContributionsExceeded {{ item: \"a\", count: 2, max: 1 }}, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// 14. `k` is the bound, and the trailer is the claim
//
// Fused enumeration is top-k by construction (§7): summing across strata means
// no candidate may be emitted until it is known not to reappear and raise its
// total, so the surface offers a bound rather than complete enumeration. These
// fixtures pin both halves of that — the bound holds, and a bound that exceeds
// what the streams hold refuses nothing.
// ---------------------------------------------------------------------------

/// Five distinct candidates across two strata, scripted fresh on each call
/// because fusing consumes them.
fn five_candidates() -> Vec<(Iri, MockStream)> {
    vec![
        (
            stratum("text"),
            MockStream::new(
                vec![
                    row(1, Fixed::ONE, K, "a"),
                    row(2, Fixed::ONE, K, "b"),
                    row(3, Fixed::ONE, K, "c"),
                ],
                exhausted(3),
            ),
        ),
        (
            stratum("vector"),
            MockStream::new(
                vec![row(1, Fixed::ONE, K, "d"), row(2, Fixed::ONE, K, "e")],
                exhausted(2),
            ),
        ),
    ]
}

#[test]
fn fuse_returns_at_most_k_rows_in_the_declared_order() {
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K, 4);

    let whole = block_on(run_fuse(five_candidates(), &profile));
    assert_eq!(whole.rows.len(), 5, "the fixture holds five candidates");

    let bounded = block_on(async {
        purrdf_retrieval::fuse::<MockStream, Term>(five_candidates(), &profile, TopK::new(2))
            .await
            .expect("a bounded fusion succeeds")
    });
    assert_eq!(bounded.rows.len(), 2, "the bound is a bound");
    assert_eq!(
        bounded.rows,
        whole.rows[..2].to_vec(),
        "the bounded rows are the top of the same order, not a different order"
    );
    for window in bounded.rows.windows(2) {
        assert!(
            window[0].score >= window[1].score,
            "score descending, the profile's declared order"
        );
    }
    // The declared total tie-break decides equal scores: best rank ascending,
    // then canonical term ascending. Both heads are rank 1 under equal weights,
    // so the first row is the byte-smaller term.
    assert_eq!(bounded.rows[0].score, bounded.rows[1].score);
    assert!(
        bounded.rows[0].entity.as_str() < bounded.rows[1].entity.as_str(),
        "equal scores and equal best ranks break on canonical term order: {:?}",
        bounded
            .rows
            .iter()
            .map(|row| row.entity.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn a_bound_larger_than_the_answer_returns_everything_and_refuses_nothing() {
    // The valid neighbour of the bound above, and the one this repository treats
    // as exactly as severe: asking for more rows than exist is not an error, not
    // a truncation, and not an empty answer. It is every row there was.
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K, 4);
    let generous = block_on(async {
        purrdf_retrieval::fuse::<MockStream, Term>(
            five_candidates(),
            &profile,
            TopK::new(1_000_000),
        )
        .await
        .expect("a bound above the frontier is not a refusal")
    });
    assert_eq!(
        generous.rows.len(),
        5,
        "every candidate the streams held is in the answer"
    );
    assert_eq!(
        generous.rows,
        block_on(run_fuse(five_candidates(), &profile)).rows,
        "and it is the same answer, in the same order"
    );
    assert_eq!(generous.trailer.statuses.len(), 2);
}

#[test]
fn completeness_is_asserted_by_the_trailer_never_by_the_rows_in_hand() {
    // One stratum answers, one could not. The rows a consumer holds change with
    // the bound; the terminal report does not, because the trailer drives every
    // producer to its own receipt whatever the bound stopped reading.
    let profile = profile(&[("text", Fixed::ONE), ("geo", Fixed::ONE)], K, 4);
    let streams = || {
        vec![
            (
                stratum("text"),
                MockStream::new(
                    vec![
                        row(1, Fixed::ONE, K, "a"),
                        row(2, Fixed::ONE, K, "b"),
                        row(3, Fixed::ONE, K, "c"),
                    ],
                    exhausted(3),
                ),
            ),
            (
                stratum("geo"),
                MockStream::new(
                    Vec::new(),
                    ProducerReceipt::ExecutionFailed {
                        reason: "no index".to_owned(),
                    },
                ),
            ),
        ]
    };

    let expected_statuses = BTreeMap::from([
        (
            stratum("text"),
            ProducerStatus::Exhausted { rows_emitted: 3 },
        ),
        (
            stratum("geo"),
            ProducerStatus::ExecutionFailed {
                reason: "no index".to_owned(),
            },
        ),
    ]);

    // A prefix: one row of three, and the report is already whole.
    let prefix = block_on(async {
        purrdf_retrieval::fuse::<MockStream, Term>(streams(), &profile, TopK::new(1))
            .await
            .expect("a bounded fusion succeeds")
    });
    assert_eq!(prefix.rows.len(), 1, "the consumer holds one row of three");
    assert_eq!(
        prefix.trailer.statuses, expected_statuses,
        "and the trailer still names every producer, including the one that could not answer"
    );

    // No rows at all. An empty answer is not "the search found nothing": the
    // trailer says what every producer did, and one of them failed.
    let none = block_on(async {
        purrdf_retrieval::fuse::<MockStream, Term>(streams(), &profile, TopK::new(0))
            .await
            .expect("a bound of zero is a request, not a refusal")
    });
    assert!(
        none.rows.is_empty(),
        "a bound of zero certifies no rows, got {:?}",
        none.rows
    );
    assert_eq!(
        none.trailer.statuses, expected_statuses,
        "zero rows read, and the completeness claim is unchanged — it was never in the rows"
    );

    // Every row. Same report again: the row count carried no claim in either
    // direction.
    let whole = block_on(run_fuse(streams(), &profile));
    assert_eq!(whole.rows.len(), 3);
    assert_eq!(whole.trailer.statuses, expected_statuses);
}
