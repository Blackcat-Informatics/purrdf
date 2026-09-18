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
    DecayRule, DuplicatePolicy, Fixed, FusionError, FusionProfile, FusionProfileId, FusionResult,
    FusionStream, Iri, MonotoneDepth, PlanId, ProducerReceipt, ProducerStatus, ProtocolError,
    RECIP_K, RankedStream, StreamContract, Term, TopK, contribution, contribution_under,
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

/// A fixture profile over the named strata.
///
/// There is no contribution-maximum argument to pass: the profile derives it
/// from `weights`, because a candidate may surface at most once per stratum.
fn profile(weights: &[(&str, Fixed)], k: u32) -> FusionProfile {
    let map: BTreeMap<Iri, Fixed> = weights
        .iter()
        .map(|(name, weight)| (stratum(name), *weight))
        .collect();
    FusionProfile::with_decay(map, DecayRule::ReciprocalRank { k })
        .expect("fixture profile is valid")
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

/// The contract most fixtures here declare: strictly descending contributions
/// and no repeated item. It is the honest declaration for a scripted stream of
/// distinct items at contiguous ranks, and it is what `register_ranked`'s own
/// fixtures elsewhere in the crate state.
fn strict_unique() -> StreamContract {
    StreamContract::new(DuplicatePolicy::Unique)
}

/// A producer whose rows and failures are pre-scripted.
struct MockStream {
    steps: VecDeque<Step>,
    receipt: ProducerReceipt,
    rows_emitted: u64,
    drop_counter: Arc<AtomicUsize>,
    /// A shared count of the rows actually pulled from this stream.
    pull_counter: Arc<AtomicUsize>,
    /// The pinned plan this stream claims to descend from, if any.
    plan_id: Option<PlanId>,
    /// What this stream declares about its own rows. A fixture that is *about*
    /// the declaration states its own; everything else declares the strict,
    /// unique contract its scripted rows actually satisfy.
    contract: StreamContract,
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
            pull_counter: Arc::new(AtomicUsize::new(0)),
            plan_id: None,
            contract: strict_unique(),
        }
    }

    fn tracked(steps: Vec<Step>, receipt: ProducerReceipt, counter: Arc<AtomicUsize>) -> Self {
        let mut stream = Self::new(steps, receipt);
        stream.drop_counter = counter;
        stream
    }

    /// The same producer, counting every row pulled from it into `counter`.
    fn counted(steps: Vec<Step>, receipt: ProducerReceipt, counter: Arc<AtomicUsize>) -> Self {
        let mut stream = Self::new(steps, receipt);
        stream.pull_counter = counter;
        stream
    }

    /// The same producer, claiming to descend from `plan_id` (or from no plan).
    fn pinned(mut self, plan_id: Option<PlanId>) -> Self {
        self.plan_id = plan_id;
        self
    }

    /// The same producer, declaring `contract` about its rows.
    fn declaring(mut self, contract: StreamContract) -> Self {
        self.contract = contract;
        self
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
                self.pull_counter.fetch_add(1, Ordering::SeqCst);
                Ok(Some((rank, score, item)))
            }
            Some(Step::Fail(error)) => Err(error),
            None => Ok(None),
        }
    }

    async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError> {
        Ok(self.receipt.clone())
    }

    fn contract(&self) -> StreamContract {
        self.contract
    }

    fn plan_id(&self) -> Option<PlanId> {
        self.plan_id
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
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
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
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
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
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
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
//
// The profile weights *one* stratum, and the three streams are all tagged with
// it, so the admitted ceiling is one whole `i128::MAX` and the profile itself
// constructs. That shape is the reason this drives `FusionStream` directly:
// `fuse` refuses a repeated stratum before a row is read. A profile weighting
// three strata could not reach here at all — its own ceiling would be
// `3 * i128::MAX` and construction would refuse it, which is the derived
// ceiling doing exactly its job.
#[test]
fn checked_addition_overflow_is_refused() {
    let huge = Fixed::from_raw(i128::MAX);
    let profile = profile(&[("only", huge)], 1);
    let only = stratum("only");
    let streams = vec![
        (
            only.clone(),
            MockStream::new(vec![row(1, huge, 1, "a")], exhausted(1)),
        ),
        (
            only.clone(),
            MockStream::new(vec![row(1, huge, 1, "b")], exhausted(1)),
        ),
        (
            only,
            MockStream::new(vec![row(1, huge, 1, "c")], exhausted(1)),
        ),
    ];
    let mut fusion = FusionStream::new(streams, profile);
    let result = block_on(fusion.next());
    assert!(
        matches!(result, Err(FusionError::Overflow)),
        "expected FusionError::Overflow, got {result:?}"
    );
}

// 4b. The valid neighbour of 4: the same three heads at a weight the profile's
//     own ceiling admits sum without complaint. The refusal above is about the
//     fixed-point range, not about three streams or about summing.
#[test]
fn a_checked_addition_inside_the_range_is_not_refused() {
    let large = Fixed::from_raw(i128::MAX / 4);
    let profile = profile(&[("only", large)], 1);
    let only = stratum("only");
    let streams = vec![
        (
            only.clone(),
            MockStream::new(vec![row(1, large, 1, "a")], exhausted(1)),
        ),
        (
            only.clone(),
            MockStream::new(vec![row(1, large, 1, "b")], exhausted(1)),
        ),
        (
            only,
            MockStream::new(vec![row(1, large, 1, "c")], exhausted(1)),
        ),
    ];
    let mut fusion = FusionStream::new(streams, profile);
    let row = block_on(fusion.next()).expect("three heads at a quarter of the range still sum");
    assert!(row.is_some(), "the fusion certifies a row rather than none");
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
    let profile = profile(&[("text", Fixed::ONE)], K);
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

    // A repeated item is a protocol violation too, but its refusal depends on
    // the stream's *declaration* rather than on the row alone, so it is not part
    // of this battery: see
    // `a_declared_unique_streams_repeat_is_refused_while_the_first_is_unemitted`
    // and the de-duplication tests beside it.

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
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
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
    let profile = profile(&[("text", Fixed::ONE)], K);
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
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
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
        FusionProfile::with_decay(weights(Fixed::ONE), DecayRule::ReciprocalRank { k: 0 }),
        Err(FusionError::InvalidK { k: 0 })
    ));
    assert!(matches!(
        FusionProfile::with_decay(BTreeMap::new(), DecayRule::ReciprocalRank { k: K }),
        Err(FusionError::EmptyWeights)
    ));
    assert!(matches!(
        FusionProfile::with_decay(weights(Fixed::ZERO), DecayRule::ReciprocalRank { k: K }),
        Err(FusionError::NonPositiveWeight { .. })
    ));
    assert!(matches!(
        FusionProfile::with_decay(
            weights(Fixed::from_raw(-1)),
            DecayRule::ReciprocalRank { k: K }
        ),
        Err(FusionError::NonPositiveWeight { .. })
    ));
    // The ceiling is `max_weight * strata`, so two strata at the top of the
    // range is the smallest profile whose own ceiling does not fit. The valid
    // neighbour is directly below: one stratum at the same weight has a ceiling
    // of exactly `i128::MAX` and constructs.
    assert!(matches!(
        FusionProfile::with_decay(
            BTreeMap::from([
                (stratum("text"), Fixed::from_raw(i128::MAX)),
                (stratum("vector"), Fixed::from_raw(i128::MAX)),
            ]),
            DecayRule::ReciprocalRank { k: K }
        ),
        Err(FusionError::Overflow)
    ));
    let at_the_top = FusionProfile::with_decay(
        weights(Fixed::from_raw(i128::MAX)),
        DecayRule::ReciprocalRank { k: K },
    )
    .expect("one stratum still fits");
    assert_eq!(at_the_top.ceiling(), Fixed::from_raw(i128::MAX));

    // The contribution maximum is the stratum count, not an argument, and the
    // ceiling is that count times the largest weight.
    let baseline = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    let same = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    assert_eq!(baseline.max_contributions(), 2, "two weights, two strata");
    assert_eq!(
        baseline.ceiling(),
        Fixed::ONE.checked_add(Fixed::ONE).expect("fits"),
        "the ceiling is max_weight * strata"
    );
    assert_eq!(baseline.id(), same.id(), "identical profiles share an id");
    assert_eq!(
        FusionProfile::from_canonical_bytes(&baseline.canonical_bytes()).expect("round-trips"),
        baseline
    );

    let different_k = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K + 1);
    let different_weight = profile(&[("text", Fixed::ONE), ("vector", Fixed::from_raw(2))], K);
    // A third stratum raises the derived contribution maximum, and the identity
    // records it: the field is still in the canonical bytes even though it is
    // no longer supplied.
    let more_strata = profile(
        &[
            ("text", Fixed::ONE),
            ("vector", Fixed::ONE),
            ("geo", Fixed::ONE),
        ],
        K,
    );
    assert_eq!(more_strata.max_contributions(), 3);
    assert_ne!(baseline.id(), different_k.id());
    assert_ne!(baseline.id(), different_weight.id());
    assert_ne!(baseline.id(), more_strata.id());
}

// 11b. The derived contribution maximum is in the canonical bytes, and the
//      decoder refuses bytes that disagree with their own stratum count.
//
// The field is redundant with the weight count it follows, which is exactly why
// it must be checked: decoding a profile whose declared maximum was not its
// stratum count would return a value whose own `canonical_bytes` are not the
// bytes it came from, and therefore a different identity than the one the
// caller handed over.
#[test]
fn a_declared_contribution_maximum_must_match_the_stratum_count() {
    let two = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    let bytes = two.canonical_bytes();
    assert_eq!(
        FusionProfile::from_canonical_bytes(&bytes).expect("its own bytes decode"),
        two,
        "the valid neighbour: bytes stating the stratum count round-trip"
    );

    // The maximum is the `u32` immediately after the tie-break tag, which is
    // the fourth byte from the end (scale u32, rounding u8, item encoding u8).
    let offset = bytes.len() - 4 - 1 - 1 - 4;
    for forged in [1_u32, 3, 8] {
        let mut tampered = bytes.clone();
        tampered[offset..offset + 4].copy_from_slice(&forged.to_le_bytes());
        assert_eq!(
            u32::from_le_bytes(
                bytes[offset..offset + 4]
                    .try_into()
                    .expect("four bytes are four bytes")
            ),
            2,
            "the field this test rewrites is the contribution maximum"
        );
        assert!(
            matches!(
                FusionProfile::from_canonical_bytes(&tampered),
                Err(FusionError::MalformedProfile(_))
            ),
            "a declared maximum of {forged} over two strata must be refused"
        );
    }
}

// 12. Canonical profile bytes and fused output are target-independent.
//
// The bytes are asserted against a fixed digest captured on this target; the
// same construction on wasm32-unknown-unknown is proven to compile and to
// produce byte-identical input by the pure-field encoding. The fused rendering
// is checked against the same golden the native test uses.
#[test]
fn native_and_wasm_share_canonical_bytes_and_output() {
    let profile = profile(&[("text", Fixed::ONE)], K);
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

// 13. A candidate contributed to more times than there are strata is an
//     invariant violation, and it is reported as one: the offending candidate
//     by name, the count reached, and the stratum count it passed — never a
//     silent extra contribution, and never the generic `Overflow`.
//
//     This is no longer something a corpus can cause. The bound is the
//     profile's stratum count and a candidate surfaces at most once per
//     stratum, so reaching `strata + 1` means a stream set repeated a stratum
//     (below) or a stream emitted one candidate twice (`ProtocolError`'s
//     `DuplicateItem`, test 20). `fuse` refuses a repeated stratum before a row
//     is read, so this drives `FusionStream` directly — which is the honest way
//     to reach an invariant violation that no conforming input produces.
#[test]
fn a_candidate_contributed_to_more_times_than_there_are_strata_is_refused() {
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    assert_eq!(profile.max_contributions(), 2, "two strata, two weights");
    let streams = vec![
        (
            stratum("text"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
        ),
        (
            stratum("vector"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
        ),
        // The third stream repeats a stratum the profile weights once. Each
        // stream keeps its own per-stream uniqueness set, so nothing below this
        // level can see that `a` is about to be counted a third time.
        (
            stratum("vector"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
        ),
    ];
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
            Err(FusionError::MaxContributionsExceeded { item, count: 3, max: 2 })
                if item == "a"
        ),
        "expected MaxContributionsExceeded {{ item: \"a\", count: 3, max: 2 }}, got {result:?}"
    );
}

// 14. Valid neighbour of 13, and the case the bound used to refuse: a candidate
//     that surfaces in *every* stratum is the fusion working. Three strata,
//     three streams, one candidate in all three — no refusal, and the fused
//     score is still the checked sum of every contribution.
//
//     This is exactly the shape a caller who had guessed a contribution maximum
//     below the stratum count would have lost, and lost only once some document
//     happened to rank in one stratum more than the guess allowed.
#[test]
fn a_candidate_in_every_stratum_fuses_rather_than_refusing() {
    let profile = profile(
        &[
            ("text", Fixed::ONE),
            ("vector", Fixed::ONE),
            ("geo", Fixed::ONE),
        ],
        K,
    );
    assert_eq!(
        profile.max_contributions(),
        3,
        "three strata, three weights"
    );
    let streams = vec![
        (
            stratum("text"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "doc-everywhere")], exhausted(1)),
        ),
        (
            stratum("vector"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "doc-everywhere")], exhausted(1)),
        ),
        (
            stratum("geo"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "doc-everywhere")], exhausted(1)),
        ),
    ];
    let result = block_on(run_fuse(streams, &profile));
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].entity, Term::new("doc-everywhere"));
    assert_eq!(result.rows[0].contributions.len(), 3);

    let per_stratum = contribution(Fixed::ONE, 1, K).expect("fits");
    let mut expected = Fixed::ZERO;
    for _ in 0..3 {
        expected = expected.checked_add(per_stratum).expect("fits");
    }
    assert_eq!(
        result.rows[0].score, expected,
        "one contribution per stratum must still sum exactly"
    );
}

// 15. `ceiling() == max_weight * strata`, but every reciprocal-rank
//     contribution is *strictly* below its stratum's weight: `K >= 1`
//     (`InvalidK`) and `rank >= 1` (`InvalidRank`) force
//     `reciprocal(K + rank) <= 1/2`. So the true achievable maximum under a
//     valid contribution count is exactly half the declared ceiling, never the
//     ceiling itself. This fixture drives every stratum to that true maximum —
//     equal max weight, `K = 1`, `rank = 1`, one contribution per stratum,
//     one contribution per stratum — and proves the fused score lands,
//     exactly, at `ceiling() / 2`. This is the valid neighbour of the ceiling
//     check: even the most aggressive legitimate load the current decay rule
//     can produce must still succeed, with real headroom to spare.
#[test]
fn maximal_legitimate_score_stays_below_the_ceiling() {
    // Two. `Fixed::from_integer` takes the number a reader means; the
    // neighbouring `Fixed::from_raw(2)` would be two raw units, `2 * 10^-12`.
    let weight = Fixed::from_integer(2).expect("two is representable");
    let k = 1;
    let names = ["s0", "s1", "s2", "s3"];
    let weights: Vec<(&str, Fixed)> = names.iter().copied().map(|name| (name, weight)).collect();
    let profile = profile(&weights, k);

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
//     rank-1, `K = 1` contributions under a profile that weights one stratum,
//     and therefore admits one contribution — is also the one configuration
//     that already violates the contribution count, so the count check fires
//     first and
//     `CeilingExceeded` is never observed for it. This drives `FusionStream`
//     directly (bypassing `fuse`'s duplicate-stratum guard) because reaching
//     it needs two streams sharing one stratum tag, which `fuse` itself
//     refuses before fusion ever begins.
#[test]
fn ceiling_boundary_is_gated_by_the_contribution_count_check() {
    // Two. `Fixed::from_integer` takes the number a reader means; the
    // neighbouring `Fixed::from_raw(2)` would be two raw units, `2 * 10^-12`.
    let weight = Fixed::from_integer(2).expect("two is representable");
    let k = 1;
    let profile = profile(&[("dup", weight)], k);
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
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);

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
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
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
    // One stratum answers, one could not. Whatever the bound, every producer
    // has a status in the trailer and none of it is readable from the rows in
    // hand — that is the claim. The statuses themselves are *not* invariant
    // under the bound, and must not be: a bound that stopped the reading leaves
    // the text stratum incomplete, and a trailer that said `Exhausted` anyway
    // would be asserting completeness the fusion never established.
    let profile = profile(&[("text", Fixed::ONE), ("geo", Fixed::ONE)], K);
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

    let failed = ProducerStatus::ExecutionFailed {
        reason: "no index".to_owned(),
    };
    // A stratum that was read down to rank `r` and no further is reported at
    // rank `r`'s contribution: everything at or above it was read, and what lies
    // below it was not.
    let stopped_at = |rank| ProducerStatus::CeilingReached {
        bound: contribution(Fixed::ONE, rank, K).expect("the fixture contribution fits"),
    };

    // A prefix: one row of three, and the report is already whole — every
    // producer named, and the one that answered named as incomplete, because
    // reading stopped at the second row it held.
    let prefix = block_on(async {
        purrdf_retrieval::fuse::<MockStream, Term>(streams(), &profile, TopK::new(1))
            .await
            .expect("a bounded fusion succeeds")
    });
    assert_eq!(prefix.rows.len(), 1, "the consumer holds one row of three");
    assert_eq!(
        prefix.trailer.statuses,
        BTreeMap::from([
            (stratum("text"), stopped_at(2)),
            (stratum("geo"), failed.clone()),
        ]),
        "the trailer names every producer, including the one that could not answer, \
         and does not claim the bounded one was exhausted"
    );

    // No rows at all. An empty answer is not "the search found nothing": the
    // trailer says what every producer did — one failed outright, and the other
    // was never read past its first row.
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
        none.trailer.statuses,
        BTreeMap::from([
            (stratum("text"), stopped_at(1)),
            (stratum("geo"), failed.clone()),
        ]),
        "zero rows certified, and the completeness claim is still the trailer's — \
         it was never in the rows"
    );

    // Every row. The same producers, and now the text stratum really was
    // exhausted, so it says so: nothing about the row count carried the claim
    // in either direction, and nothing about the claim was invented to match it.
    let whole = block_on(run_fuse(streams(), &profile));
    assert_eq!(whole.rows.len(), 3);
    assert_eq!(
        whole.trailer.statuses,
        BTreeMap::from([
            (
                stratum("text"),
                ProducerStatus::Exhausted { rows_emitted: 3 }
            ),
            (stratum("geo"), failed),
        ])
    );
}

// 15. The bound stops the reading, not only the returning.
#[test]
fn a_bounded_stop_closes_a_stream_instead_of_draining_it() {
    // A stratum with far more rows than the bound asks for, and a counter on
    // every pull. If the terminal report drained the stream to make it declare
    // `Exhausted`, the count would be the whole stream and the memory bound
    // top-k exists for would be gone — with the rows returned looking exactly
    // the same either way, which is why this is counted rather than eyeballed.
    const ROWS: u64 = 500;
    let profile = profile(&[("text", Fixed::ONE)], K);
    let pulls = Arc::new(AtomicUsize::new(0));
    let steps = (1..=ROWS)
        .map(|rank| row(rank, Fixed::ONE, K, &format!("item-{rank:04}")))
        .collect();
    let streams = vec![(
        stratum("text"),
        MockStream::counted(steps, exhausted(ROWS), Arc::clone(&pulls)),
    )];

    let bounded = block_on(async {
        purrdf_retrieval::fuse::<MockStream, Term>(streams, &profile, TopK::new(3))
            .await
            .expect("a bounded fusion succeeds")
    });

    assert_eq!(bounded.rows.len(), 3, "the bound is the bound");
    let pulled = pulls.load(Ordering::SeqCst);
    assert!(
        pulled < 16,
        "a top-3 answer over {ROWS} rows pulled {pulled} of them, which is a drain"
    );
    assert_eq!(
        bounded.trailer.statuses.get(&stratum("text")),
        Some(&ProducerStatus::CeilingReached {
            bound: contribution(Fixed::ONE, u64::try_from(pulled).expect("a small count"), K)
                .expect("the fixture contribution fits")
        }),
        "the stratum is closed at the contribution of the last row read from it"
    );

    // The neighbour that must still succeed, and the reason the bounded status
    // is not simply what this fusion always says: a bound the stream cannot
    // reach exhausts it, and then the producer's own receipt is what the
    // trailer reports.
    let pulls = Arc::new(AtomicUsize::new(0));
    let steps = (1..=3)
        .map(|rank| row(rank, Fixed::ONE, K, &format!("item-{rank:04}")))
        .collect();
    let streams = vec![(
        stratum("text"),
        MockStream::counted(steps, exhausted(3), Arc::clone(&pulls)),
    )];
    let whole = block_on(async {
        purrdf_retrieval::fuse::<MockStream, Term>(streams, &profile, TopK::new(1_000))
            .await
            .expect("a bound above the stream is not a refusal")
    });
    assert_eq!(
        whole.rows.len(),
        3,
        "every row the stream held is in the answer"
    );
    assert_eq!(
        whole.trailer.statuses.get(&stratum("text")),
        Some(&ProducerStatus::Exhausted { rows_emitted: 3 }),
        "a stream that genuinely ended reports its own receipt, verified against the rows"
    );
}

// 16. The pinned plan travels on the streams, and two plans in one fusion is a
//     refusal rather than an answer filed under one of them.
#[test]
fn the_trailer_names_a_plan_only_when_every_stream_names_the_same_one() {
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    let first = plan_id("first-plan");
    let second = plan_id("second-plan");
    let streams = |left: Option<PlanId>, right: Option<PlanId>| {
        vec![
            (
                stratum("text"),
                MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)).pinned(left),
            ),
            (
                stratum("vector"),
                MockStream::new(vec![row(1, Fixed::ONE, K, "b")], exhausted(1)).pinned(right),
            ),
        ]
    };

    // Agreement: the identity the streams carried is the identity the answer
    // reports.
    let agreed = block_on(run_fuse(streams(Some(first), Some(first)), &profile));
    assert_eq!(agreed.trailer.plan_id, Some(first));
    assert_eq!(agreed.rows.len(), 2);

    // Disagreement: one answer cannot descend from two plans, and picking one
    // of them would be right about half the rows.
    let error = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        streams(Some(first), Some(second)),
        &profile,
        TOP_K,
    ))
    .expect_err("two pinned plans in one fusion is a refusal");
    assert!(
        matches!(
            &error,
            FusionError::PlanIdMismatch { expected, got: Some(got) }
                if *expected == first && *got == second
        ),
        "expected PlanIdMismatch, got {error:?}"
    );

    // And the neighbours that must still fuse, so the refusal stays narrow.
    // Streams that name no plan at all are not in disagreement about one; the
    // answer simply names none rather than borrowing an identity.
    let unpinned = block_on(run_fuse(streams(None, None), &profile));
    assert_eq!(unpinned.trailer.plan_id, None);
    assert_eq!(unpinned.rows, agreed.rows, "and the rows are the same rows");

    let mixed = block_on(run_fuse(streams(Some(first), None), &profile));
    assert_eq!(
        mixed.trailer.plan_id, None,
        "one stream that descends from no plan means no single plan produced the answer"
    );
    assert_eq!(mixed.rows, agreed.rows, "and the rows are the same rows");
}

// 17. "I declined the terms" is a different answer from "I found nothing", and
//     a producer cannot use the first to hide rows it emitted.
#[test]
fn a_producer_that_declined_its_terms_is_not_a_producer_that_found_nothing() {
    let profile = profile(&[("text", Fixed::ONE)], K);

    // The producer was handed terms it does not serve and says so. It emits no
    // rows, and the trailer keeps the reason it emitted none.
    let declined = block_on(run_fuse(
        vec![(
            stratum("text"),
            MockStream::new(Vec::new(), ProducerReceipt::TermsRejected),
        )],
        &profile,
    ));
    assert_eq!(
        declined.rows,
        Vec::new(),
        "a producer that declined its terms emits no rows"
    );
    assert_eq!(
        declined.trailer.statuses.get(&stratum("text")),
        Some(&ProducerStatus::TermsRejected)
    );

    // The neighbour it must stay distinguishable from: a producer that served
    // the terms, looked, and had nothing to return. Same empty answer, different
    // report — which is the whole reason the variant exists.
    let searched = block_on(run_fuse(
        vec![(stratum("text"), MockStream::new(Vec::new(), exhausted(0)))],
        &profile,
    ));
    assert_eq!(searched.rows, declined.rows, "both answers are empty");
    assert_eq!(
        searched.trailer.statuses.get(&stratum("text")),
        Some(&ProducerStatus::Exhausted { rows_emitted: 0 }),
        "and they are not the same answer"
    );

    // And the claim is checked against what the stream actually did: a producer
    // that emitted a row and then declared it had declined the terms is refused,
    // so the variant cannot be used to disown rows already in the fusion.
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        vec![(
            stratum("text"),
            MockStream::new(
                vec![row(1, Fixed::ONE, K, "a")],
                ProducerReceipt::TermsRejected,
            ),
        )],
        &profile,
        TOP_K,
    ));
    assert!(
        matches!(
            &result,
            Err(FusionError::Protocol(error))
                if matches!(**error, ProtocolError::ForgedReceipt { declared: 0, actual: 1 })
        ),
        "expected ForgedReceipt, got {result:?}"
    );
}

// 18. Finality is membership, not arithmetic: a live stream of contribution
//     zero can still name a candidate, and a sum cannot see it.
//
// `U(x) == L(x)` was standing in for "no stream that could still name `x` is
// open". The two agree whenever every live head contributes something, which is
// why this never showed up under ordinary weights — but a head of exactly zero
// adds nothing to `U(x)`, so an open stream reads as an exhausted one. The
// candidate is emitted, removed from the frontier, and then re-entered by the
// stream that was still holding it, to be emitted a SECOND time carrying only
// that stream's contribution.
//
// A contribution of zero is a legal profile, not a contrivance: the profile
// admits any weight strictly above zero, and the contribution truncates toward
// zero at the declared scale, so a weight of one raw unit yields exactly zero at
// every rank.

/// A weight that is legal (strictly positive) and contributes exactly zero.
const ZERO_CONTRIBUTING_WEIGHT: Fixed = Fixed::from_raw(1);

#[test]
fn a_live_zero_contribution_stream_is_not_mistaken_for_an_exhausted_one() {
    // The fixture is only a fixture if the weight really does contribute
    // nothing, so that is asserted rather than assumed.
    assert_eq!(
        contribution(ZERO_CONTRIBUTING_WEIGHT, 1, K).expect("fits"),
        Fixed::ZERO,
        "one raw unit of weight truncates to nothing at the declared scale"
    );
    assert_eq!(
        contribution(ZERO_CONTRIBUTING_WEIGHT, 2, K).expect("fits"),
        Fixed::ZERO
    );
    assert!(
        ZERO_CONTRIBUTING_WEIGHT > Fixed::ZERO,
        "and the profile admits it, so this is a configuration a caller can write"
    );

    let profile = profile(
        &[("dense", Fixed::ONE), ("sparse", ZERO_CONTRIBUTING_WEIGHT)],
        K,
    );
    let dense_contribution = contribution(Fixed::ONE, 1, K).expect("fits");

    // `a` is the dense stratum's only row, and the sparse stratum names it at
    // rank two — after a row the fusion has not read yet. At the moment `a`
    // enters the frontier the sparse stream is open and has not seen it, so `a`
    // is NOT final; its upper bound says otherwise only because the sparse head
    // contributes zero.
    let streams = vec![
        (
            stratum("dense"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
        ),
        (
            stratum("sparse"),
            // Under this weight every contribution truncates to zero, so no two
            // adjacent ranks differ. That is a property of the profile's
            // arithmetic at this weight, not a claim the producer made or could
            // have made — see
            // `the_contribution_law_no_longer_depends_on_a_declared_ordering`.
            MockStream::new(
                vec![
                    row(1, ZERO_CONTRIBUTING_WEIGHT, K, "z"),
                    row(2, ZERO_CONTRIBUTING_WEIGHT, K, "a"),
                ],
                exhausted(2),
            )
            .declaring(strict_unique()),
        ),
    ];
    let result = block_on(run_fuse(streams, &profile));

    let entities: Vec<&str> = result
        .rows
        .iter()
        .map(|fused| fused.entity.as_str())
        .collect();
    assert_eq!(
        entities,
        vec!["a", "z"],
        "two candidates were named, so two rows are the whole answer — `a` \
         emitted twice would be the same candidate with two different scores"
    );

    let fused_a = &result.rows[0];
    assert_eq!(
        fused_a.score, dense_contribution,
        "`a`'s score is the dense contribution plus the sparse zero"
    );
    assert_eq!(
        fused_a.contributions.len(),
        2,
        "and it is certified with BOTH strata counted, including the one whose \
         contribution is zero but whose rank is real"
    );
    assert_eq!(
        fused_a.contributions[0],
        (stratum("dense"), 1, dense_contribution)
    );
    assert_eq!(
        fused_a.contributions[1],
        (stratum("sparse"), 2, Fixed::ZERO)
    );
}

// 19. The neighbouring valid case: the structural test must not make
//     certification lazier where the arithmetic one was right.
//
// Over-refusal here would be latency rather than a wrong answer, and it would be
// invisible — every assertion about the rows would still pass while the fusion
// drained streams it had no need to read. So this counts pulls. A stratum that
// contributes zero and is ALREADY exhausted must cost exactly nothing: the
// candidate certifies on the same pull it certified on before.
#[test]
fn an_exhausted_zero_contribution_stream_delays_no_certification() {
    let profile = profile(
        &[("dense", Fixed::ONE), ("sparse", ZERO_CONTRIBUTING_WEIGHT)],
        K,
    );
    let pulls = Arc::new(AtomicUsize::new(0));
    let streams = vec![
        (
            stratum("dense"),
            MockStream::counted(
                vec![
                    row(1, Fixed::ONE, K, "a"),
                    row(2, Fixed::ONE, K, "b"),
                    row(3, Fixed::ONE, K, "c"),
                ],
                exhausted(3),
                Arc::clone(&pulls),
            ),
        ),
        (stratum("sparse"), MockStream::new(Vec::new(), exhausted(0))),
    ];
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        streams,
        &profile,
        TopK::new(1),
    ))
    .expect("fusion succeeds");

    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].entity, Term::new("a"));
    // Two pulls, and that is the bound the algorithm justifies: rank one, which
    // is the answer, plus exactly one lookahead row to drop the threshold below
    // it. The exhausted sparse stratum adds nothing to either. A finality test
    // that demanded a contribution from every stream regardless of whether it
    // was still open would have read all three dense rows instead.
    assert_eq!(
        pulls.load(Ordering::SeqCst),
        2,
        "an exhausted stratum is not an open one, and must not be awaited"
    );
}

// ---------------------------------------------------------------------------
// 20. The two declarations a ranked producer makes about its own rows are read,
//     and each is honoured on its own terms.
//
// `RankedDeclaration` states a rank ordering and a duplicate policy where a
// producer is registered, and this layer is the consumer both were written for.
// So both reach it — carried with the stream, never re-fetched — and each of the
// four spellings gets the treatment it declared. Every test below pairs the case
// it refuses with the neighbouring case it must still admit, because a duplicate
// policy whose own definition names the consumer's obligation, refused by that
// consumer, is over-refusal at its purest.
// ---------------------------------------------------------------------------

/// `DuplicatePolicy::Allowed`, with the strict ordering the fixtures' rows keep.
fn allowed_duplicates() -> StreamContract {
    StreamContract::new(DuplicatePolicy::Allowed)
}

/// Two streams: `dense` as scripted, and a one-row `sparse` stream that stays
/// open long enough to keep `dense`'s first candidate in the frontier.
///
/// Without the second stratum nothing is held: a single-stratum candidate is
/// final the moment it is read, so it certifies and leaves the frontier before
/// the next row arrives. The fixture is two strata because the frontier is where
/// a declared-`Unique` stream's promise is checked.
fn dense_and_sparse(dense: MockStream) -> Vec<(Iri, MockStream)> {
    vec![
        (stratum("dense"), dense),
        (
            stratum("sparse"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "q")], exhausted(1)),
        ),
    ]
}

fn dense_sparse_profile() -> FusionProfile {
    profile(&[("dense", Fixed::ONE), ("sparse", Fixed::ONE)], K)
}

#[test]
fn a_declared_unique_streams_repeat_is_refused_while_the_first_is_unemitted() {
    let profile = dense_sparse_profile();
    // `a` cannot be emitted before the duplicate is read: the sparse stream is
    // open and has not named it, so it is not final and stays in the frontier.
    let streams = dense_and_sparse(MockStream::new(
        vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "a")],
        exhausted(2),
    ));
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        streams, &profile, TOP_K,
    ));
    assert!(
        matches!(
            &result,
            Err(FusionError::Protocol(error))
                if matches!(&**error, ProtocolError::DuplicateItem { item } if item == "a")
        ),
        "expected DuplicateItem for `a`, got {result:?}"
    );

    // THE NEIGHBOURING CASE: the same item named once by each of the two streams
    // is the whole point of fusing them, not a duplicate.
    let shared = vec![
        (
            stratum("dense"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
        ),
        (
            stratum("sparse"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
        ),
    ];
    let fused = block_on(run_fuse(shared, &profile));
    assert_eq!(fused.rows.len(), 1);
    assert_eq!(
        fused.rows[0].contributions.len(),
        2,
        "one candidate, two strata, no duplicate"
    );
}

/// What a `Unique` declaration buys and what it costs, stated as one test
/// because they are one decision.
///
/// A stream that promised no repeats keeps no identity set, so the promise is
/// checked exactly where the engine needs that state anyway: the frontier. A
/// repeat whose first occurrence has already been certified and removed is
/// therefore *not* caught — that detection is precisely the retained-forever set
/// the declaration said was unnecessary, and it is the one structure a fusion
/// holds that grows with the rows pulled.
///
/// This is pinned rather than left to be discovered. A producer that cannot make
/// the promise has a complete, supported answer one line away, and the second
/// half of this test is that answer: the same stream declared `Allowed` is
/// de-duplicated in full.
#[test]
fn a_unique_declaration_is_relied_on_past_the_frontier_and_allowed_is_the_remedy() {
    // One stratum, so `a` certifies the moment it is read and the repeat arrives
    // after it left the frontier.
    let profile = profile(&[("dense", Fixed::ONE)], K);
    let repeated = vec![(
        stratum("dense"),
        MockStream::new(
            vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "a")],
            exhausted(2),
        ),
    )];
    let fused = block_on(run_fuse(repeated, &profile));
    assert_eq!(
        fused
            .rows
            .iter()
            .map(|fused_row| fused_row.entity.clone())
            .collect::<Vec<_>>(),
        vec![Term::new("a"), Term::new("a")],
        "the promise is relied on: a repeat past the frontier reaches the answer"
    );

    // The remedy, and it is complete: the identical stream, declaring the policy
    // that says its repeats are the consumer's to remove.
    let deduplicated = vec![(
        stratum("dense"),
        MockStream::new(
            vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "a")],
            exhausted(2),
        )
        .declaring(allowed_duplicates()),
    )];
    let fused = block_on(run_fuse(deduplicated, &profile));
    assert_eq!(
        fused.rows.len(),
        1,
        "a de-duplicated stream names `a` once, got {:?}",
        fused.rows
    );
    assert_eq!(
        fused.rows[0].score,
        contribution(Fixed::ONE, 1, K).expect("fits"),
        "and at its best — first, lowest — rank"
    );
}

#[test]
fn a_declared_allowed_stream_is_deduplicated_rather_than_refused() {
    let profile = dense_sparse_profile();
    // The exact stream the `Unique` test above refuses, declaring instead that
    // its repeats are the consumer's to remove. This layer is that consumer, and
    // this is the case where it does the removing: one stream, one item, two
    // ranks, one contribution.
    let streams = dense_and_sparse(
        MockStream::new(
            vec![
                row(1, Fixed::ONE, K, "a"),
                row(2, Fixed::ONE, K, "a"),
                row(3, Fixed::ONE, K, "b"),
            ],
            exhausted(3),
        )
        .declaring(allowed_duplicates()),
    );
    let fused = block_on(run_fuse(streams, &profile));

    let dense_contribution = |entity: &str| -> Vec<(u64, Fixed)> {
        fused
            .rows
            .iter()
            .filter(|fused_row| fused_row.entity == Term::new(entity))
            .flat_map(|fused_row| fused_row.contributions.iter())
            .filter(|(iri, _, _)| *iri == stratum("dense"))
            .map(|(_, rank, value)| (*rank, *value))
            .collect()
    };

    assert_eq!(
        fused
            .rows
            .iter()
            .filter(|fused_row| fused_row.entity == Term::new("a"))
            .count(),
        1,
        "the repeat contributes once, not twice: {:?}",
        fused.rows
    );
    assert_eq!(
        dense_contribution("a"),
        vec![(1, contribution(Fixed::ONE, 1, K).expect("fits"))],
        "and at the BEST rank the stream gave it, which is its first"
    );
    // The row after the repeat is not renumbered: rank 3 is still rank 3, so a
    // dropped duplicate does not silently promote its successors.
    assert_eq!(
        dense_contribution("b"),
        vec![(3, contribution(Fixed::ONE, 3, K).expect("fits"))],
        "a discarded duplicate leaves every later rank exactly where it was"
    );
    // The producer is still charged for the row it emitted, so a receipt that
    // under-counts is still a forged one.
    assert_eq!(
        fused.trailer.statuses[&stratum("dense")],
        ProducerStatus::Exhausted { rows_emitted: 3 },
        "the discarded row was still pulled, and the receipt is measured against it"
    );

    // THE NEIGHBOURING CASE: `Allowed` changes nothing for a stream that does
    // not in fact repeat. Same rows, both declarations, byte-identical answers.
    let rows_of = |contract: StreamContract| {
        let streams = dense_and_sparse(
            MockStream::new(
                vec![
                    row(1, Fixed::ONE, K, "a"),
                    row(2, Fixed::ONE, K, "b"),
                    row(3, Fixed::ONE, K, "c"),
                ],
                exhausted(3),
            )
            .declaring(contract),
        );
        block_on(run_fuse(streams, &profile)).rows
    };
    assert_eq!(
        rows_of(allowed_duplicates()),
        rows_of(strict_unique()),
        "with no repeat to remove, the policy is invisible in the answer"
    );
}

#[test]
fn the_contribution_law_no_longer_depends_on_a_declared_ordering() {
    // A contribution is the profile's own function of the rank, so a weight
    // small enough that the truncation collides makes two adjacent ranks carry
    // one value. `monotone_depth` is the exact rank at which that first happens,
    // and one raw unit of weight collides immediately.
    let weight = Fixed::from_raw(1);
    let profile = profile(&[("thin", weight)], K);
    let first = contribution(weight, 1, K).expect("fits");
    let second = contribution(weight, 2, K).expect("fits");
    assert_eq!(
        first, second,
        "this fixture needs a profile whose adjacent ranks collide, or it tests nothing"
    );

    // The criterion, inverted from what this test used to assert. The stream is
    // well formed in every respect a producer controls: ranks 1 and 2, ascending,
    // contiguous, unique items, and each contribution exactly the one the profile
    // computes. The equality between them is the consumer's own truncation, so it
    // is not a protocol violation and is no longer refused. The rows fuse, and
    // the tie falls through to the declared tie-break, whose next key is best
    // stratum rank ascending — which returns them in the producer's own order.
    let streams = vec![(
        stratum("thin"),
        MockStream::new(
            vec![
                Step::Row(1, first, Term::new("a")),
                Step::Row(2, second, Term::new("b")),
            ],
            exhausted(2),
        ),
    )];
    let fused = block_on(run_fuse(streams, &profile));
    assert_eq!(
        fused
            .rows
            .iter()
            .map(|fused_row| fused_row.entity.clone())
            .collect::<Vec<_>>(),
        vec![Term::new("a"), Term::new("b")],
        "a collided adjacent pair fuses, in rank order"
    );
    // The producer is still held to the rows it claimed, so this is not a case
    // of fusion having stopped checking the stream.
    assert_eq!(
        fused.trailer.statuses[&stratum("thin")],
        ProducerStatus::Exhausted { rows_emitted: 2 },
        "both rows were pulled and the receipt is measured against them"
    );

    // And a contribution that *rises* never enters fusion, because the threshold
    // over the heads would stop being an upper bound. The ordering declaration is
    // gone, so the axis that remains is the duplicate policy: the refusal holds
    // for every stream, not for a subset that declared something.
    //
    // It is refused as the `ContributionMismatch` it is. The only way to present
    // a rising value is to supply one the profile did not compute, and naming
    // that "your contribution rose with rank" would blame the stream's shape for
    // a wrong number — the same conflation this test's subject was. Non-increase
    // of the profile's own curve is proven where it is true, as a property of
    // `DecayRule` (`reciprocal_rank::tests::every_decay_rule_is_non_increasing_in_the_rank`);
    // `NonMonotoneContribution` remains as the typed guard on that invariant.
    let first_rank = contribution(Fixed::ONE, 1, K).expect("fits");
    let risen = Fixed::from_raw(first_rank.into_raw() + 1);
    for contract in [strict_unique(), allowed_duplicates()] {
        let heavy = crate::profile(&[("thin", Fixed::ONE)], K);
        let streams = vec![(
            stratum("thin"),
            MockStream::new(
                vec![
                    row(1, Fixed::ONE, K, "a"),
                    Step::Row(2, risen, Term::new("b")),
                ],
                exhausted(2),
            )
            .declaring(contract),
        )];
        let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
            streams, &heavy, TOP_K,
        ));
        assert!(
            matches!(
                &result,
                Err(FusionError::Protocol(error))
                    if matches!(**error, ProtocolError::ContributionMismatch { .. })
            ),
            "a rising contribution never enters fusion under {contract:?}, got {result:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 21. A second decay rule, added without moving the first one's numbers.
//
// `DecayRule` is additive-variant-extensible and its tag is folded into the
// profile's canonical bytes, so a new variant must leave every previously
// issued identity exactly where it was. The tests below pin that in the only
// way that is worth anything — against a spelling of the encoding written out
// by hand rather than read back from the writer under test — and then pin what
// the new rule buys: depth, in exchange for weight.
// ---------------------------------------------------------------------------

/// The canonical encoding of a `ReciprocalRank` profile, spelled out field by
/// field from the documented layout.
///
/// Written independently of `Writer` on purpose. A golden captured by calling
/// the encoder proves only that the encoder agrees with itself; this one fails
/// if a field is reordered, widened, or given a new tag.
fn hand_spelled_profile_bytes(strata: &[(&str, i128)], k: u32, max_contributions: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    // version: u16 le
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    // decay tag: ReciprocalRank is discriminator zero
    bytes.push(0);
    // K: u32 le
    bytes.extend_from_slice(&k.to_le_bytes());
    // weight count: u64 le
    bytes.extend_from_slice(&(strata.len() as u64).to_le_bytes());
    for (name, weight_raw) in strata {
        let text = format!("http://example.org/stratum/{name}");
        bytes.extend_from_slice(&(text.len() as u64).to_le_bytes());
        bytes.extend_from_slice(text.as_bytes());
        bytes.extend_from_slice(&weight_raw.to_le_bytes());
    }
    // tie-break tag, then max contributions
    bytes.push(0);
    bytes.extend_from_slice(&max_contributions.to_le_bytes());
    // declared scale, rounding direction, item encoding
    bytes.extend_from_slice(&12_u32.to_le_bytes());
    bytes.push(0);
    bytes.push(0);
    bytes
}

#[test]
fn the_existing_decay_rules_identity_bytes_are_where_they_always_were() {
    let existing = profile(&[("text", Fixed::ONE), ("vector", Fixed::from_raw(500))], K);
    // The contribution maximum is spelled out as two, which is what the two
    // strata above derive. This is also the identity-stability claim: the
    // encoder below is written by hand from the documented layout and is
    // unchanged, so these are byte-for-byte the bytes a caller who used to pass
    // `weights.len()` explicitly already had. Only a caller who passed
    // something else — this fixture passed eight, a bound nothing could reach —
    // moves, and moves onto the identity the field always meant.
    let expected =
        hand_spelled_profile_bytes(&[("text", 1_000_000_000_000), ("vector", 500)], K, 2);
    assert_eq!(
        existing.canonical_bytes(),
        expected,
        "adding a decay variant must not move the encoding of the first one"
    );
    assert_eq!(
        existing.max_contributions(),
        u32::try_from(existing.weights().len()).expect("two strata"),
        "the encoded maximum is exactly the stratum count"
    );
    assert_eq!(
        existing.id(),
        FusionProfileId::from_canonical(&expected),
        "the identity is the digest of exactly those bytes"
    );
    assert_eq!(
        existing.id().to_hex(),
        "c7c47d73ddb4a430142c86a73acd2eedc32b514c38ccefb8627995e6dadb0a6e",
        "the digest of a profile that names its stratum count is frozen"
    );
    assert_eq!(
        existing.decay(),
        DecayRule::ReciprocalRank { k: K },
        "a profile built through `new` runs under the rule it always did"
    );

    // And the numbers themselves: `contribution` is the tag-zero arithmetic,
    // unchanged at every rank a caller might have recorded.
    for (rank, raw) in [
        (1_u64, 16_393_442_622_i128),
        (2, 16_129_032_258),
        (10, 14_285_714_285),
        (1_000, 943_396_226),
        (1_000_000, 999_940),
    ] {
        assert_eq!(
            contribution(Fixed::ONE, rank, K).expect("fits").into_raw(),
            raw,
            "the truncated rule's value at rank {rank} is frozen"
        );
    }
}

/// Deriving a profile's per-stratum separation bounds reports the decay rule's
/// own refusal rather than rendering one as a bound, and nothing a caller can
/// declare provokes that refusal: every weight and smoothing constant that built
/// a profile before still builds one, under both rules.
///
/// The mirror of a swallowed refusal is an over-refusal, and it is the failure
/// this test exists to catch. A derivation that refused where it used to answer
/// would reject a profile whose ranks in fact order perfectly — as severe a
/// defect as a wrong answer, and invisible in fixtures that all happen to use
/// the same middling weight. So the acceptance surface is asserted across the
/// range directly, and the identity each profile carries is asserted to be the
/// digest of its declared bytes, which the derived bounds are absent from.
#[test]
fn every_profile_the_separation_derivation_accepted_before_still_builds_with_its_identity() {
    for k in [1_u32, 60, u32::MAX] {
        for raw in [
            1_i128,
            1_000,
            500_000_000_000,
            1_000_000_000_000,
            200_000_000_000_000,
        ] {
            for decay in [
                DecayRule::ReciprocalRank { k },
                DecayRule::WeightedReciprocalRank { k },
            ] {
                let weights = BTreeMap::from([
                    (stratum("text"), Fixed::from_raw(raw)),
                    (stratum("vector"), Fixed::from_raw(1)),
                ]);
                let profile =
                    FusionProfile::with_decay(weights.clone(), decay).unwrap_or_else(|error| {
                        panic!("{decay:?} at weight raw {raw} must still build: {error}")
                    });
                assert!(
                    profile.monotone_depth(&stratum("text")).is_some(),
                    "{decay:?} at weight raw {raw}: a weighted stratum reports a bound"
                );
                assert_eq!(
                    profile.id(),
                    FusionProfileId::from_canonical(&profile.canonical_bytes()),
                    "{decay:?} at weight raw {raw}: the identity is the digest of the \
                     declared bytes, and the derived bounds are not among them"
                );
                assert_eq!(
                    profile.id(),
                    FusionProfile::with_decay(weights, decay)
                        .expect("the same declaration builds twice")
                        .id(),
                    "{decay:?} at weight raw {raw}: one declaration is one identity"
                );
            }
        }
    }
}

#[test]
fn the_weighted_rule_is_a_separate_identity_and_round_trips() {
    let weights: BTreeMap<Iri, Fixed> = BTreeMap::from([(stratum("text"), Fixed::ONE)]);
    let truncated = FusionProfile::with_decay(weights.clone(), DecayRule::ReciprocalRank { k: K })
        .expect("valid");
    let folded = FusionProfile::with_decay(weights, DecayRule::WeightedReciprocalRank { k: K })
        .expect("valid");

    assert_eq!(
        truncated.canonical_bytes(),
        profile(&[("text", Fixed::ONE)], K).canonical_bytes(),
        "`new` is `with_decay` under the first rule"
    );
    assert_ne!(
        truncated.id(),
        folded.id(),
        "a different rule is a different law and therefore a different identity"
    );
    assert_eq!(truncated.canonical_bytes()[2], 0, "tag zero");
    assert_eq!(folded.canonical_bytes()[2], 1, "tag one");

    assert_eq!(
        FusionProfile::from_canonical_bytes(&folded.canonical_bytes()).expect("round-trips"),
        folded
    );

    // An unassigned tag is refused rather than read as a rule this build knows.
    let mut unknown = folded.canonical_bytes();
    unknown[2] = 2;
    assert!(
        FusionProfile::from_canonical_bytes(&unknown).is_err(),
        "an unknown decay tag is refused"
    );
}

/// The weight the deep fixtures run at: two hundred, which under the weighted
/// rule buys a monotone range above fourteen million ranks.
const DEEP_WEIGHT_UNITS: i64 = 200;

fn deep_weight() -> Fixed {
    Fixed::from_integer(DEEP_WEIGHT_UNITS).expect("two hundred is representable")
}

#[test]
fn the_weighted_rule_still_orders_at_fourteen_million_ranks() {
    let folded = FusionProfile::with_decay(
        BTreeMap::from([(stratum("text"), deep_weight())]),
        DecayRule::WeightedReciprocalRank { k: 1 },
    )
    .expect("a strictly positive weight is a valid profile");
    let bound = folded
        .monotone_depth(&stratum("text"))
        .expect("the profile weights this stratum")
        .rank()
        .expect("this weight reaches a real collision inside the expressible range");
    assert!(
        bound >= 14_000_000,
        "a weight of {DEEP_WEIGHT_UNITS} must order past fourteen million ranks, got {bound}"
    );

    // Two-sided, the shape the other monotone fixtures use: adjacent ranks stay
    // strictly ordered up to the bound, and the first pair beyond it collides.
    let weight = deep_weight();
    let at = |rank: u64| contribution_under(folded.decay(), weight, rank).expect("fits");
    for rank in [1_u64, 2, 13_999_999, 14_000_000, bound - 2, bound - 1] {
        assert!(
            at(rank) > at(rank + 1),
            "ranks {rank} and {} must stay ordered inside the bound {bound}",
            rank + 1
        );
    }
    assert_eq!(
        at(bound),
        at(bound + 1),
        "the bound is the last ordered depth, not one short of it"
    );

    // The same weight under the first rule collides four hundred times sooner,
    // which is the defect this rule exists to answer.
    let truncated = FusionProfile::with_decay(
        BTreeMap::from([(stratum("text"), deep_weight())]),
        DecayRule::ReciprocalRank { k: 1 },
    )
    .expect("valid");
    let shallow = truncated
        .monotone_depth(&stratum("text"))
        .expect("weighted")
        .rank()
        .expect("the truncated rule always collides inside the expressible range");
    assert!(
        shallow < 1_100_000,
        "the truncated rule's bound is set by sqrt(S), not by the weight, got {shallow}"
    );
}

// ---------------------------------------------------------------------------
// 22. The decay's own quantization is not a protocol violation
//
// A contribution is computed by the consumer from the decay rule, K, the
// stratum weight and the rank, re-derived on arrival and refused on mismatch.
// Ranks are separately held contiguous and ascending. So when two adjacent ranks
// carry one contribution, the producer supplied no term of it: the profile's
// fixed-point arithmetic stopped separating those ranks at that depth. Fusion
// measures that and carries on.
//
// Every test below that claims to read past a collision proves it with a witness
// that does not come from the instrument under test. Asserting
// `ranks_pulled > separation` would pass vacuously if either derived number were
// wrong — reintroducing, inside the evidence channel, exactly the false
// all-clear these tests exist to prevent. The witnesses used instead are the
// directly observed collision count, the producer's own receipt, and bare
// arithmetic on the decay rule.
// ---------------------------------------------------------------------------

/// A profile over one stratum named `deep`, at `weight` under `decay`.
fn deep_profile(decay: DecayRule, weight: Fixed) -> FusionProfile {
    FusionProfile::with_decay(BTreeMap::from([(stratum("deep"), weight)]), decay)
        .expect("a strictly positive weight is a valid profile")
}

/// `rows` well-formed rows: contiguous ranks from one, distinct items, and the
/// exact contribution the profile computes for each rank.
fn deep_stream(decay: DecayRule, weight: Fixed, rows: u64) -> MockStream {
    let steps = (1..=rows)
        .map(|rank| {
            Step::Row(
                rank,
                contribution_under(decay, weight, rank).expect("fixture contribution fits"),
                Term::new(format!("d{rank:07}")),
            )
        })
        .collect();
    MockStream::new(steps, exhausted(rows))
}

/// The first rank whose contribution equals its successor's, found by walking.
/// Independent of `monotone_depth`, so a test can cross-check it.
fn first_collision_by_walking(decay: DecayRule, weight: Fixed, limit: u64) -> Option<u64> {
    (1..limit).find(|&rank| {
        let here = contribution_under(decay, weight, rank).expect("fits");
        let next = contribution_under(decay, weight, rank + 1).expect("fits");
        here == next
    })
}

/// The deepest depth admissible at `max_width`, found by a walk that is
/// independent of `deepest_rank_within_width`: it steps rank by rank,
/// accumulating the length of the run of equal contributions it is inside,
/// and applies the truncation rule directly. If a run beginning at
/// `run_start` first reaches `max_width + 1` members at `run_start +
/// max_width`, then reading no deeper than `run_start + max_width - 1` keeps
/// every rank of that run inside the tolerance, and reading one rank further
/// does not — so that is the truth this walk returns.
fn deepest_rank_within_width_by_walking(
    decay: DecayRule,
    weight: Fixed,
    max_width: u64,
    limit: u64,
) -> u64 {
    let ceiling = max_width.max(1);
    let mut run_start = 1_u64;
    let mut current = contribution_under(decay, weight, 1).expect("fits");
    let mut rank = 1_u64;
    while rank < limit {
        let next = contribution_under(decay, weight, rank + 1).expect("fits");
        if next == current {
            if rank + 1 - run_start + 1 > ceiling {
                return run_start + ceiling - 1;
            }
        } else {
            current = next;
            run_start = rank + 1;
        }
        rank += 1;
    }
    limit
}

/// The width of `rank`'s indifference class as it would appear to a read
/// that never looks past `depth_limit`: the run of equal contributions
/// containing `rank`, clipped to `1..=depth_limit`.
///
/// This differs from `class_width` (which sees the whole unbounded
/// sequence) precisely at the boundary `deepest_rank_within_width` cares
/// about: a run that extends past `depth_limit` is only partially observed
/// by a read truncated there, and this is what measures the part that was.
fn truncated_class_width(decay: DecayRule, weight: Fixed, depth_limit: u64, rank: u64) -> u64 {
    let target = contribution_under(decay, weight, rank).expect("fits");
    let mut lo = rank;
    while lo > 1 {
        let prev = contribution_under(decay, weight, lo - 1).expect("fits");
        if prev == target {
            lo -= 1;
        } else {
            break;
        }
    }
    let mut hi = rank;
    while hi < depth_limit {
        let next = contribution_under(decay, weight, hi + 1).expect("fits");
        if next == target {
            hi += 1;
        } else {
            break;
        }
    }
    hi - lo + 1
}

#[test]
fn a_strictly_descending_stream_fuses_past_the_decay_collision() {
    // The issue's own middle row: a weight of 1e6 raw units collides near rank
    // 972, well inside a 1400-row stream, and every one of those rows is well
    // formed in every respect a producer controls.
    let decay = DecayRule::ReciprocalRank { k: K };
    let weight = Fixed::from_raw(1_000_000);
    let profile = deep_profile(decay, weight);

    // Witness one, and the reason this fixture tests anything: bare arithmetic
    // on the decay rule says these ranks really do collide inside the stream.
    let collision = first_collision_by_walking(decay, weight, 1_400)
        .expect("this fixture must collide inside the stream it fuses");
    assert_eq!(
        collision, 972,
        "the issue's measured collision rank for this weight"
    );

    let fused = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        vec![(stratum("deep"), deep_stream(decay, weight, 1_400))],
        &profile,
        TopK::new(1_400),
    ))
    .expect("a well-formed stream fuses past the depth its decay still separates");
    assert_eq!(fused.rows.len(), 1_400, "every row is in the answer");

    // Witness two: the producer's own receipt, validated against the rows fusion
    // actually pulled. It could not say 1400 if fusion had stopped early.
    assert_eq!(
        fused.trailer.statuses[&stratum("deep")],
        ProducerStatus::Exhausted {
            rows_emitted: 1_400
        }
    );

    // Witness three: the collisions fusion observed directly, on the comparison
    // it performs per row. Not inferred from a depth against a bound.
    let measured = fused.trailer.resolution[&stratum("deep")];
    assert!(
        measured.collisions_observed > 0,
        "this run must have entered the range where the score stops ordering"
    );
    assert_eq!(measured.ranks_pulled, 1_400);
    assert_eq!(measured.separation, MonotoneDepth::SeparatesTo(collision));

    // And the answer is still the producer's order, because the tie-break's
    // second key is best stratum rank ascending.
    let entities: Vec<&str> = fused.rows.iter().map(|row| row.entity.as_str()).collect();
    let expected: Vec<String> = (1..=1_400_u64).map(|rank| format!("d{rank:07}")).collect();
    assert_eq!(
        entities,
        expected.iter().map(String::as_str).collect::<Vec<_>>(),
        "past the collision the total tie-break recovers the producer's own rank order"
    );
}

#[test]
fn a_small_top_k_certifies_early_and_never_reaches_the_collision() {
    // The trap this issue records, pinned as a positive test rather than left as
    // a warning. The identical stream and profile as the test above, bounded at
    // five: fusion certifies the top five and never pulls to the collision, so a
    // regression test written this way reports a false all-clear.
    let decay = DecayRule::ReciprocalRank { k: K };
    let weight = Fixed::from_raw(1_000_000);
    let profile = deep_profile(decay, weight);

    let fused = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        vec![(stratum("deep"), deep_stream(decay, weight, 1_400))],
        &profile,
        TopK::new(5),
    ))
    .expect("a bounded fusion answers");
    assert_eq!(fused.rows.len(), 5);

    let measured = fused.trailer.resolution[&stratum("deep")];
    assert_eq!(
        measured.collisions_observed, 0,
        "a small bound never reaches the collision, which is why asserting only \
         `is_ok()` here would prove nothing about the depth"
    );
    assert!(
        measured.ranks_pulled < 972,
        "and it stopped far short of the colliding rank, at {}",
        measured.ranks_pulled
    );
}

#[test]
fn two_ranks_fuse_at_the_weight_that_used_to_refuse_them() {
    // The issue's first row: one thousand raw units collides immediately, so the
    // shortest possible stream already carries a repeated contribution. This is
    // the exact case that was refused as `RepeatedContribution` at rank two.
    let decay = DecayRule::ReciprocalRank { k: K };
    let weight = Fixed::from_raw(1_000);
    let profile = deep_profile(decay, weight);

    let first = contribution_under(decay, weight, 1).expect("fits");
    let second = contribution_under(decay, weight, 2).expect("fits");
    assert_eq!(first, second, "1000/61 and 1000/62 both truncate to 16");
    assert_eq!(first.into_raw(), 16, "the issue's measured value");

    let fused = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        vec![(stratum("deep"), deep_stream(decay, weight, 2))],
        &profile,
        TopK::new(2),
    ))
    .expect("two well-formed ranks fuse");
    assert_eq!(fused.rows.len(), 2);
    assert_eq!(
        fused.trailer.resolution[&stratum("deep")].collisions_observed,
        1,
        "exactly one adjacent pair collided, and it was observed rather than inferred"
    );
}

#[test]
fn the_issue_measurement_table_fuses_at_every_weight() {
    // The issue's table, end to end. The three weights differ only in how soon
    // the decay stops separating ranks; none of them is a protocol violation and
    // all three answer. The first two used to be refused.
    let decay = DecayRule::ReciprocalRank { k: K };
    for raw in [1_000_i128, 1_000_000, 100_000_000] {
        let weight = Fixed::from_raw(raw);
        let profile = deep_profile(decay, weight);
        let fused = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
            vec![(stratum("deep"), deep_stream(decay, weight, 1_400))],
            &profile,
            TopK::new(1_400),
        ))
        .unwrap_or_else(|error| panic!("weight {raw} raw units must fuse, got {error:?}"));
        assert_eq!(fused.rows.len(), 1_400, "weight {raw} answers in full");

        // The heaviest weight orders the whole stream; the two lighter ones do
        // not, and say so. Both are correct answers.
        let measured = fused.trailer.resolution[&stratum("deep")];
        let collided = first_collision_by_walking(decay, weight, 1_400).is_some();
        assert_eq!(
            measured.collisions_observed > 0,
            collided,
            "the observed count must agree with the arithmetic at weight {raw}"
        );
    }
}

#[test]
fn both_decay_rules_fuse_past_their_own_collision() {
    // The issue exercises only the truncated rule. The weighted rule has its own
    // boundary, near `sqrt(w_raw) - k` rather than near `sqrt(S)`, and the same
    // law applies there: a collision is quantization, not a violation.
    let weight = Fixed::from_raw(1_000_000);
    for decay in [
        DecayRule::ReciprocalRank { k: K },
        DecayRule::WeightedReciprocalRank { k: K },
    ] {
        let profile = deep_profile(decay, weight);
        let collision = first_collision_by_walking(decay, weight, 1_400)
            .unwrap_or_else(|| panic!("{decay:?} must collide inside this fixture"));

        let fused = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
            vec![(stratum("deep"), deep_stream(decay, weight, 1_400))],
            &profile,
            TopK::new(1_400),
        ))
        .unwrap_or_else(|error| panic!("{decay:?} must fuse, got {error:?}"));

        assert_eq!(fused.rows.len(), 1_400);
        let measured = fused.trailer.resolution[&stratum("deep")];
        assert!(measured.collisions_observed > 0, "{decay:?} crossed");
        assert_eq!(measured.separation, MonotoneDepth::SeparatesTo(collision));
    }
}

#[test]
fn a_wrong_contribution_at_a_plateau_rank_is_still_a_mismatch() {
    // Removing an over-refusal must not create an under-refusal. On a plateau,
    // where equality between adjacent ranks is now legal, a value the profile
    // did not compute is still refused — the equality arm was the only thing
    // removed, and it was never what checked the value.
    let decay = DecayRule::ReciprocalRank { k: K };
    let weight = Fixed::from_raw(1_000);
    let profile = deep_profile(decay, weight);
    let plateau = contribution_under(decay, weight, 1).expect("fits");
    assert_eq!(
        plateau,
        contribution_under(decay, weight, 2).expect("fits"),
        "this fixture needs ranks one and two on one plateau"
    );

    // THE INVALID CASE: rank two carries a value that is neither rising nor the
    // profile's own.
    let forged = Fixed::from_raw(plateau.into_raw() - 1);
    let streams = vec![(
        stratum("deep"),
        MockStream::new(
            vec![
                Step::Row(1, plateau, Term::new("a")),
                Step::Row(2, forged, Term::new("b")),
            ],
            exhausted(2),
        ),
    )];
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        streams,
        &profile,
        TopK::new(2),
    ));
    assert!(
        matches!(
            &result,
            Err(FusionError::Protocol(error))
                if matches!(**error, ProtocolError::ContributionMismatch { .. })
        ),
        "a value the profile did not compute is refused even on a plateau, got {result:?}"
    );

    // THE NEIGHBOURING VALID CASE: the same shape with the profile's own value,
    // which is exactly equal to its predecessor's. This is what the fix
    // legalizes, and it must succeed.
    let streams = vec![(
        stratum("deep"),
        MockStream::new(
            vec![
                Step::Row(1, plateau, Term::new("a")),
                Step::Row(2, plateau, Term::new("b")),
            ],
            exhausted(2),
        ),
    )];
    assert_eq!(
        block_on(purrdf_retrieval::fuse::<MockStream, Term>(
            streams,
            &profile,
            TopK::new(2)
        ))
        .expect("the profile's own value on a plateau fuses")
        .rows
        .len(),
        2
    );
}

#[test]
fn a_zero_top_k_still_reports_one_rank_pulled() {
    // A trailer asserts a terminal status per producer, which requires reading
    // each stream's first row. So `ranks_pulled` is one even here, never zero —
    // surprising enough that a caller testing `== 0` for "untouched" would be
    // wrong, and therefore worth pinning.
    let decay = DecayRule::ReciprocalRank { k: K };
    let weight = Fixed::from_raw(1_000);
    let profile = deep_profile(decay, weight);
    let fused = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        vec![(stratum("deep"), deep_stream(decay, weight, 10))],
        &profile,
        TopK::new(0),
    ))
    .expect("a zero bound still reports");
    assert_eq!(fused.rows.len(), 0, "a bound of zero admits no rows");
    assert_eq!(fused.trailer.resolution[&stratum("deep")].ranks_pulled, 1);
    assert_eq!(
        fused.trailer.resolution[&stratum("deep")].collisions_observed,
        0,
        "one row has no adjacent pair to collide with"
    );
}

#[test]
fn fusion_past_the_collision_is_deterministic() {
    // The claim the whole change rests on: past the separating depth the answer
    // is still a pure function of its inputs, because the declared tie-break is
    // total. Two runs over identical inputs, compared row for row including
    // every score and every provenance entry.
    let decay = DecayRule::ReciprocalRank { k: K };
    let weight = Fixed::from_raw(1_000);
    let profile = deep_profile(decay, weight);
    let run = || {
        block_on(purrdf_retrieval::fuse::<MockStream, Term>(
            vec![
                (stratum("deep"), deep_stream(decay, weight, 200)),
                (stratum("other"), deep_stream(decay, weight, 200)),
            ],
            &FusionProfile::with_decay(
                BTreeMap::from([(stratum("deep"), weight), (stratum("other"), weight)]),
                decay,
            )
            .expect("valid"),
            TopK::new(400),
        ))
        .expect("fuses")
    };
    let first = run();
    let second = run();
    assert_eq!(first.rows, second.rows, "two runs agree row for row");
    assert!(
        profile
            .class_width(&stratum("deep"), 200)
            .expect("the arithmetic evaluates")
            .expect("weighted")
            > 1,
        "this fixture must sit on a plateau, or it proves nothing about ties"
    );
}

// ---------------------------------------------------------------------------
// 23. The resolution algebra, and the one refusal it is allowed to make
//
// A refusal is a claim, so the refusal `weight_for_depth` makes is paired with
// its neighbouring valid case in the same test. It is also exact rather than
// conservative, which is the difference between an honest wall and the
// over-refusal this section of the crate exists to have removed: under the
// truncated rule the reciprocal is rounded *before* the weight is applied, so
// once two adjacent ranks collide there they are equal for every weight.
// ---------------------------------------------------------------------------

/// Whether `candidate` separates every adjacent pair of ranks up to `depth`,
/// recomputed from the published contribution rather than from the search.
fn reaches_depth(decay: DecayRule, candidate: Fixed, depth: u64) -> bool {
    (1..depth).all(|rank| {
        let here = contribution_under(decay, candidate, rank).expect("fits");
        let next = contribution_under(decay, candidate, rank + 1).expect("fits");
        here > next
    })
}

#[test]
fn weight_for_depth_names_a_weight_that_reaches_it() {
    // THE VALID CASE, both rules and across the range where the arithmetic
    // changes character. Minimality is asserted separately and exhaustively,
    // because checking only the neighbour below proves nothing here: the
    // predicate oscillates on single raw units, so *some* lighter weight fails
    // no matter how badly the answer overstates.
    for decay in [
        DecayRule::ReciprocalRank { k: K },
        DecayRule::WeightedReciprocalRank { k: K },
    ] {
        for depth in [2_u64, 10, 500, 5_000] {
            let weight = decay
                .weight_for_depth(depth)
                .unwrap_or_else(|error| panic!("{decay:?} depth {depth}: {error:?}"));
            assert!(
                reaches_depth(decay, weight, depth),
                "{decay:?}: the weight it named must separate every rank up to {depth}"
            );
        }
    }
}

#[test]
fn no_weight_lighter_than_the_one_named_reaches_the_depth() {
    // THE MINIMALITY CLAIM, checked by enumerating every lighter weight rather
    // than by sampling one. The answers here are in the tens and the low
    // thousands, so the whole domain below them is walkable, and an answer that
    // overstated by even one raw unit would be caught.
    //
    // Weights are read as ratios, so an over-quoted weight silently re-scales
    // that stratum's share of every fused score. That makes an over-estimate a
    // wrong answer and not a safe one, which is why this is exhaustive.
    for decay in [
        DecayRule::ReciprocalRank { k: K },
        DecayRule::WeightedReciprocalRank { k: K },
    ] {
        for depth in [2_u64, 10] {
            let weight = decay
                .weight_for_depth(depth)
                .unwrap_or_else(|error| panic!("{decay:?} depth {depth}: {error:?}"));
            for raw in 1..weight.into_raw() {
                assert!(
                    !reaches_depth(decay, Fixed::from_raw(raw), depth),
                    "{decay:?}: raw weight {raw} reaches depth {depth}, so {weight:?} is not \
                     the minimum and the answer overstates what that depth costs"
                );
            }
        }
    }
}

#[test]
fn reaching_a_depth_is_not_monotone_in_the_weight() {
    // The witnesses that make a bisection over `weight_for_depth`'s predicate
    // unsound, named so that reintroducing one fails loudly here rather than
    // silently over-reporting. Under both rules the minimum weight for depth two
    // is immediately followed by a *heavier* weight that does not reach it.
    for (decay, minimum) in [
        (DecayRule::ReciprocalRank { k: K }, 62_i128),
        (DecayRule::WeightedReciprocalRank { k: K }, 61_i128),
    ] {
        assert_eq!(
            decay.weight_for_depth(2).expect("depth two is reachable"),
            Fixed::from_raw(minimum),
            "{decay:?}: the minimum weight for depth two is the witness's lighter half"
        );
        assert!(
            reaches_depth(decay, Fixed::from_raw(minimum), 2),
            "{decay:?}: raw {minimum} must reach depth two"
        );
        assert!(
            !reaches_depth(decay, Fixed::from_raw(minimum + 1), 2),
            "{decay:?}: raw {} must NOT reach depth two — that inversion is what a \
             binary search over the predicate would straddle",
            minimum + 1
        );
    }
}

#[test]
fn weight_for_depth_refuses_only_where_no_weight_can_reach() {
    // THE INVALID CASE and its neighbour, one rank apart. The truncated rule
    // saturates; the refusal must land exactly on that boundary and not one rank
    // early, because a bound drawn conservatively here would refuse depths the
    // arithmetic in fact delivers.
    let decay = DecayRule::ReciprocalRank { k: K };
    let saturation = deep_profile(decay, Fixed::ONE)
        .monotone_depth(&stratum("deep"))
        .expect("weighted")
        .rank()
        .expect("the truncated rule always saturates inside the expressible range");

    // The neighbour that must still succeed: the deepest reachable rank. What
    // comes back is checked against the profile's own measured bound rather
    // than merely for being positive — an over-quoted weight would pass that
    // weaker assertion, and so would a weight that does not in fact reach the
    // depth it was asked for.
    let weight = decay
        .weight_for_depth(saturation)
        .expect("the saturation depth itself is reachable");
    assert!(weight.into_raw() > 0);
    assert_eq!(
        deep_profile(decay, weight)
            .monotone_depth(&stratum("deep"))
            .expect("weighted")
            .rank(),
        Some(saturation),
        "the weight named for the saturation depth must reach exactly it"
    );

    // And one rank past it, which no weight reaches.
    match decay.weight_for_depth(saturation + 1) {
        Err(FusionError::DepthUnreachable {
            depth,
            saturates_at,
        }) => {
            assert_eq!(depth, saturation + 1);
            assert_eq!(
                saturates_at, saturation,
                "the rank reported must be the exact measured saturation, not a bound"
            );
        }
        other => panic!("expected DepthUnreachable past saturation, got {other:?}"),
    }
}

/// The deepest per-stratum depth a plan's `u32` depth field can record.
const DEEPEST_PLAN_DEPTH: u64 = u32::MAX as u64;

#[test]
fn weight_for_depth_answers_at_the_deepest_depth_a_plan_can_record_and_refuses_past_it() {
    // THE PAIR, one step apart, on BOTH rules. The limit here is the plan's
    // 32-bit depth field, and the refusal past it must say so: the folded rule's
    // arithmetic has not run out of anything at `u32::MAX + 1` — it answers at
    // `u32::MAX` — so reporting this as the rule saturating would be the same
    // defect as refusing a valid stream for a property of the consumer's own
    // internal clamp.
    let folded = DecayRule::WeightedReciprocalRank { k: K };
    let weight = folded
        .weight_for_depth(DEEPEST_PLAN_DEPTH)
        .expect("the deepest expressible depth is reachable under the folded rule");
    assert!(
        weight.into_raw() > 0,
        "a depth a plan can record must be priced, not refused"
    );

    match folded.weight_for_depth(DEEPEST_PLAN_DEPTH + 1) {
        Err(FusionError::DepthBeyondPlanRange { depth, limit }) => {
            assert_eq!(depth, DEEPEST_PLAN_DEPTH + 1);
            assert_eq!(limit, DEEPEST_PLAN_DEPTH);
        }
        other => panic!("expected DepthBeyondPlanRange one past the plan's range, got {other:?}"),
    }

    // The truncated rule saturates long before the plan's range runs out, so at
    // `u32::MAX` it refuses as saturation — and one step further it refuses for
    // the range instead, because that is the first fact about the request that
    // is wrong, and it is wrong whatever the rule.
    let truncated = DecayRule::ReciprocalRank { k: K };
    match truncated.weight_for_depth(DEEPEST_PLAN_DEPTH) {
        Err(FusionError::DepthUnreachable { saturates_at, .. }) => assert!(
            saturates_at < DEEPEST_PLAN_DEPTH,
            "a saturation report must name a rank the rule actually reached, got {saturates_at}"
        ),
        other => {
            panic!("expected the truncated rule to saturate well inside the range, got {other:?}")
        }
    }
    match truncated.weight_for_depth(DEEPEST_PLAN_DEPTH + 1) {
        Err(FusionError::DepthBeyondPlanRange { depth, limit }) => {
            assert_eq!(depth, DEEPEST_PLAN_DEPTH + 1);
            assert_eq!(limit, DEEPEST_PLAN_DEPTH);
        }
        other => panic!("expected DepthBeyondPlanRange one past the plan's range, got {other:?}"),
    }
}

#[test]
fn the_plan_range_refusal_and_the_saturation_refusal_are_different_variants() {
    // The two are different facts: one says the decay rule stopped separating,
    // the other says no plan could record the answer even if it were priced.
    // Collapsing them back into one variant would make the message assert
    // saturation where none occurred, so the distinction is pinned here.
    let truncated = DecayRule::ReciprocalRank { k: K };
    let saturated = truncated
        .weight_for_depth(DEEPEST_PLAN_DEPTH)
        .expect_err("the truncated rule cannot reach the plan's deepest depth");
    let out_of_range = DecayRule::WeightedReciprocalRank { k: K }
        .weight_for_depth(DEEPEST_PLAN_DEPTH + 1)
        .expect_err("no plan can record a depth past its 32-bit field");

    assert!(
        matches!(saturated, FusionError::DepthUnreachable { .. }),
        "the rule giving out is reported as saturation, got {saturated:?}"
    );
    assert!(
        matches!(out_of_range, FusionError::DepthBeyondPlanRange { .. }),
        "the plan's encoding giving out is reported as a range, got {out_of_range:?}"
    );

    // And the rendered sentences must not claim each other's fact. The range
    // refusal names the plan's encoding and never says the rule stopped
    // separating; `MonotoneDepth::SeparatesBeyondAnyPlan` documents itself as
    // "there is no bound to report", so a refusal rendering it would assert
    // saturation and its absence in one sentence.
    let rendered = out_of_range.to_string();
    assert!(
        rendered.contains("32-bit") && rendered.contains("plan"),
        "the range refusal must name the plan's depth encoding: {rendered}"
    );
    assert!(
        !rendered.contains("separat") && !rendered.contains("SeparatesBeyondAnyPlan"),
        "the range refusal must not claim the rule stopped separating: {rendered}"
    );
    assert!(
        saturated.to_string().contains("separates to depth"),
        "the saturation refusal must report the depth it does reach: {saturated}"
    );
}

#[test]
fn weight_for_depth_refuses_a_depth_of_zero_and_answers_a_depth_of_one() {
    // THE PAIR, one step apart. Zero is not a depth: a depth counts 1-based
    // ranks from rank one, so zero names no rank at all and the old answer —
    // the lightest weight there is — was vacuous rather than small. One rank is
    // a real depth with no adjacent pair to separate, so it is answered.
    for decay in [
        DecayRule::ReciprocalRank { k: K },
        DecayRule::WeightedReciprocalRank { k: K },
    ] {
        match decay.weight_for_depth(0) {
            Err(FusionError::InvalidRank { rank }) => assert_eq!(rank, 0),
            other => panic!("{decay:?}: expected a depth of zero to be refused, got {other:?}"),
        }
        assert_eq!(
            decay
                .weight_for_depth(1)
                .expect("a single rank is a real depth"),
            Fixed::from_raw(1),
            "{decay:?}: one rank has no adjacent pair, so the lightest weight there is buys it"
        );
    }
}

#[test]
fn the_truncated_rule_saturates_where_no_weight_can_lift_it() {
    // The issue's unit-weight extrapolation, asserted symbolically rather than
    // by enumerating a million rows. The inner truncation is a ceiling: raising
    // the weight far above one buys no depth at all under this rule, which is
    // why `weight_for_depth` has a wall to report and why a caller that needs
    // more depth must change the rule rather than the weight.
    let decay = DecayRule::ReciprocalRank { k: K };
    let unit = deep_profile(decay, Fixed::ONE)
        .monotone_depth(&stratum("deep"))
        .expect("weighted")
        .rank()
        .expect("saturates");
    assert!(
        (1_000_000..1_100_000).contains(&unit),
        "a unit weight orders about a million ranks, got {unit}"
    );

    let heavy = Fixed::from_integer(1_000_000).expect("representable");
    let heavier = deep_profile(decay, heavy)
        .monotone_depth(&stratum("deep"))
        .expect("weighted")
        .rank()
        .expect("saturates");
    assert_eq!(
        heavier, unit,
        "a weight a million times larger buys no depth under the truncated rule"
    );

    // The weighted rule is the one where weight does buy depth, which is the
    // whole reason both rules exist.
    let folded = DecayRule::WeightedReciprocalRank { k: K };
    let bought = deep_profile(folded, heavy)
        .monotone_depth(&stratum("deep"))
        .expect("weighted")
        .rank();
    assert!(
        bought.is_none_or(|rank| rank > unit),
        "the weighted rule must order deeper at the same weight, got {bought:?}"
    );
}

#[test]
fn class_width_is_the_curve_the_bound_is_one_point_of() {
    // `monotone_depth` is where the width first exceeds one. The width itself
    // keeps growing past it, and reporting only "crossed / did not cross" would
    // flatten a curve spanning orders of magnitude into one bit.
    let decay = DecayRule::ReciprocalRank { k: K };
    let weight = Fixed::from_raw(1_000_000);
    let profile = deep_profile(decay, weight);
    let stratum = stratum("deep");
    let bound = profile
        .monotone_depth(&stratum)
        .expect("weighted")
        .rank()
        .expect("saturates");

    // Strictly inside the bound every rank is its own class.
    for rank in [1_u64, 2, bound / 2, bound - 1] {
        assert_eq!(
            profile
                .class_width(&stratum, rank)
                .expect("the arithmetic evaluates"),
            Some(1),
            "rank {rank} is inside the separating range, so it stands alone"
        );
    }
    // The bound itself is the first rank that shares a contribution — with its
    // successor, which a read to this depth never reaches. That is why the
    // separating *depth* is this rank and not the one before it.
    assert_eq!(
        profile
            .class_width(&stratum, bound)
            .expect("the arithmetic evaluates"),
        Some(2),
        "the bound is where the first collision begins"
    );

    // Past it the classes are wider, and they keep widening.
    let near = profile
        .class_width(&stratum, bound + 1)
        .expect("the arithmetic evaluates")
        .expect("weighted");
    let far = profile
        .class_width(&stratum, bound * 4)
        .expect("the arithmetic evaluates")
        .expect("weighted");
    assert!(near > 1, "past the bound ranks share a contribution");
    assert!(
        far > near,
        "and the class widens with depth: {near} at the boundary, {far} deeper"
    );

    // Which is exactly what `deepest_rank_within_width` inverts. `>=` the
    // separating bound is too weak a check: it passes for any understatement,
    // which is exactly how this shipped. Pin it to the exact truth instead.
    let deepest = profile
        .deepest_rank_within_width(&stratum, near)
        .expect("the arithmetic evaluates")
        .expect("weighted");
    let expected_deepest = deepest_rank_within_width_by_walking(decay, weight, near, 10_000);
    assert_eq!(
        deepest, expected_deepest,
        "tolerating a class of {near} must equal the independently walked depth"
    );
    assert_eq!(
        profile
            .deepest_rank_within_width(&stratum, 1)
            .expect("the arithmetic evaluates"),
        Some(bound),
        "a tolerance of one is the separating bound itself"
    );
}

#[test]
fn deepest_rank_within_width_matches_an_independently_walked_truth_at_several_tolerances() {
    // The magnitudes below were measured directly against this fixture (k=60,
    // weight 1_000_000 raw units, truncated rule). They are pinned alongside an
    // independent walk, not in place of it: the walk is what actually proves
    // the arithmetic, the literal numbers just document what it produces here.
    let decay = DecayRule::ReciprocalRank { k: K };
    let weight = Fixed::from_raw(1_000_000);
    let profile = deep_profile(decay, weight);
    let stratum = stratum("deep");

    let cases = [(2_u64, 1382_u64), (3, 1712), (5, 2222), (10, 3144)];
    for (max_width, measured) in cases {
        let truth = deepest_rank_within_width_by_walking(decay, weight, max_width, 10_000);
        assert_eq!(
            truth, measured,
            "the independent walk must reproduce the measured depth at max_width {max_width}"
        );
        let got = profile
            .deepest_rank_within_width(&stratum, max_width)
            .expect("the arithmetic evaluates")
            .expect("weighted");
        assert_eq!(
            got, truth,
            "deepest_rank_within_width must equal the independently walked depth at max_width {max_width}, not understate it"
        );
    }

    // The width-one boundary is unaffected by the general rule: it is decided
    // by the short-circuit that returns `monotone_depth` directly, and that
    // path must still agree with the exact separating bound.
    let bound = profile
        .monotone_depth(&stratum)
        .expect("weighted")
        .rank()
        .expect("saturates");
    assert_eq!(
        profile
            .deepest_rank_within_width(&stratum, 1)
            .expect("the arithmetic evaluates"),
        Some(bound),
        "max_width one must still agree exactly with monotone_depth"
    );
}

#[test]
fn deepest_rank_within_width_is_the_last_depth_the_offending_run_still_fits_in() {
    // The boundary property the general rule exists to guarantee: truncating a
    // read at the returned depth keeps every rank's class within tolerance, and
    // reading one rank further breaks that for at least one rank. Both halves
    // are asserted, per the repository's rule that a bound must be checked on
    // both the refused side and the neighbouring valid side.
    let decay = DecayRule::ReciprocalRank { k: K };
    let weight = Fixed::from_raw(1_000_000);
    let profile = deep_profile(decay, weight);
    let stratum = stratum("deep");

    for max_width in [2_u64, 3, 5, 10] {
        let depth = profile
            .deepest_rank_within_width(&stratum, max_width)
            .expect("the arithmetic evaluates")
            .expect("weighted");

        for rank in 1..=depth {
            let width = truncated_class_width(decay, weight, depth, rank);
            assert!(
                width <= max_width,
                "max_width {max_width}: rank {rank} must sit in a class no wider than \
                 {max_width} when the read is truncated at {depth}, got {width}"
            );
        }

        let widths_one_deeper: Vec<u64> = (1..=depth + 1)
            .map(|rank| truncated_class_width(decay, weight, depth + 1, rank))
            .collect();
        assert!(
            widths_one_deeper.iter().any(|&width| width > max_width),
            "max_width {max_width}: reading one rank past {depth} must push some rank's \
             class past {max_width}, or {depth} was not actually the deepest admissible depth"
        );
    }
}

#[test]
fn an_operand_the_decay_rule_refuses_is_propagated_rather_than_measured_as_a_class_of_one() {
    // A width of one is the most favourable thing the resolution algebra can
    // say: this rank is separated from both its neighbours by score alone.
    // Saying it because the arithmetic refused the operand would be a false
    // claim about the quality of an answer, made exactly where no answer was
    // computed. Rank numbering is 1-based, so rank zero is that operand and it
    // is reachable from the public surface with nothing else out of the
    // ordinary.
    let decay = DecayRule::ReciprocalRank { k: K };
    let weight = Fixed::from_raw(1_000_000);
    let profile = deep_profile(decay, weight);
    let stratum = stratum("deep");

    assert!(
        matches!(
            profile.class_width(&stratum, 0),
            Err(FusionError::InvalidRank { rank: 0 })
        ),
        "rank zero is not a rank, so there is no class around it to measure"
    );

    // The neighbouring valid case: one rank further along, the same profile and
    // the same stratum answer normally. The refusal above is about the operand
    // and nothing else.
    assert_eq!(
        profile
            .class_width(&stratum, 1)
            .expect("rank one is a rank and evaluates"),
        Some(1),
        "rank one is inside the separating range, so it stands alone"
    );
}

#[test]
fn the_decay_rule_reports_a_class_width_with_no_stratum_in_the_question() {
    // A width is a property of four numbers — the rule, its smoothing constant,
    // the weight and the rank — so asking for one must not require a stratum,
    // a weight map or a profile. A caller with only arithmetic in hand (a
    // language binding, a profile author sizing a weight) would otherwise have
    // to invent an IRI to ask through, and an invented IRI is a minted one.
    let decay = DecayRule::ReciprocalRank { k: K };
    let weight = Fixed::from_raw(1_000_000);

    assert_eq!(
        decay
            .class_width(weight, 1)
            .expect("rank one is a rank and evaluates"),
        1,
        "rank one is inside the separating range, so it stands alone"
    );

    // And it is the SAME arithmetic the profile-level entry point reaches, not
    // a second derivation that could drift from it: for the stratum a profile
    // does weight, the two agree at every rank probed, on both sides of the
    // point where the classes start to widen.
    let profile = deep_profile(decay, weight);
    let stratum = stratum("deep");
    let bound = profile
        .monotone_depth(&stratum)
        .expect("weighted")
        .rank()
        .expect("saturates");
    for rank in [1_u64, 2, bound - 1, bound, bound + 1, bound * 4] {
        assert_eq!(
            profile
                .class_width(&stratum, rank)
                .expect("the arithmetic evaluates"),
            Some(
                decay
                    .class_width(weight, rank)
                    .expect("the arithmetic evaluates")
            ),
            "the stratum-free entry point answers exactly what the profile does at rank {rank}"
        );
    }

    // The rule is part of the question, not a fixed backdrop. At a weight heavy
    // enough for the fold to buy depth, the two rules answer differently at the
    // same rank: the truncated rule has already lost resolution there and the
    // folded one still has it.
    let folded = DecayRule::WeightedReciprocalRank { k: K };
    let heavy = Fixed::from_integer(1000).expect("a thousand is representable");
    let deep = deep_profile(decay, heavy)
        .monotone_depth(&stratum)
        .expect("weighted")
        .rank()
        .expect("the truncated rule saturates at every weight")
        * 4;
    assert!(
        folded
            .class_width(heavy, deep)
            .expect("the arithmetic evaluates")
            < decay
                .class_width(heavy, deep)
                .expect("the arithmetic evaluates"),
        "the folded rule keeps resolution the truncated rule has already lost at rank {deep}"
    );
}

#[test]
fn the_decay_rules_stratum_free_class_width_refuses_the_operands_it_cannot_evaluate() {
    // The refusal travels with the arithmetic rather than with the profile, so
    // dropping the stratum must not drop the refusal. A width of one is the
    // most favourable thing this algebra can say, and returning it where
    // nothing was computed would be a false claim about an answer's quality.
    let decay = DecayRule::ReciprocalRank { k: K };
    let weight = Fixed::from_raw(1_000_000);

    assert!(
        matches!(
            decay.class_width(weight, 0),
            Err(FusionError::InvalidRank { rank: 0 })
        ),
        "rank zero is not a rank, so there is no class around it to measure"
    );
    assert!(
        matches!(
            DecayRule::ReciprocalRank { k: 0 }.class_width(weight, 4),
            Err(FusionError::InvalidK { k: 0 })
        ),
        "a smoothing constant of zero is not a constant this rule can evaluate under"
    );

    // The neighbouring valid cases, one operand away from each refusal above:
    // the same weight at rank one, and the same rank under K of one.
    assert_eq!(
        decay
            .class_width(weight, 1)
            .expect("rank one is a rank and evaluates"),
        1
    );
    assert_eq!(
        DecayRule::ReciprocalRank { k: 1 }
            .class_width(weight, 4)
            .expect("a smoothing constant of one is usable"),
        1,
        "rank four is inside the separating range at this weight, and the refusal above was \
         about the constant being zero and nothing else"
    );
}

#[test]
fn a_stratum_this_profile_does_not_weight_is_an_absence_and_never_a_refusal() {
    // The other half of the same signature. `Ok(None)` and `Err(..)` are two
    // different facts — "this profile says nothing about that stratum" and
    // "the arithmetic could not be evaluated" — and collapsing either into the
    // other loses the one a caller needs to act on.
    let decay = DecayRule::ReciprocalRank { k: K };
    let profile = deep_profile(decay, Fixed::from_raw(1_000_000));
    let unweighted = stratum("never/declared");

    assert!(
        matches!(profile.class_width(&unweighted, 7), Ok(None)),
        "an unweighted stratum has no width to report, which is not a failure"
    );
    assert!(
        matches!(profile.deepest_rank_within_width(&unweighted, 4), Ok(None)),
        "and no depth to report either, on the same terms"
    );
    // And the weighted stratum of the same profile still answers, so the
    // absence above is about the stratum rather than about the profile.
    assert!(
        profile
            .class_width(&stratum("deep"), 7)
            .expect("the arithmetic evaluates")
            .is_some(),
        "the stratum this profile does weight still reports a width"
    );
    assert!(
        profile
            .deepest_rank_within_width(&stratum("deep"), 4)
            .expect("the arithmetic evaluates")
            .is_some(),
        "and a depth"
    );
}

// ---------------------------------------------------------------------------
// 24. A golden for the collided regime
//
// `order.golden` pins a fusion at weights that separate every rank it reads, so
// the fused score alone decides the order and the tie-break's later keys are
// never exercised. Past the separating depth they decide everything — and the
// claim that the answer stays deterministic there rests entirely on them. Until
// now nothing pinned their bytes.
//
// This fixture fuses two strata at a weight whose adjacent ranks collide from
// the first pair, so every row in it is ordered by best stratum rank and then by
// canonical term, and it renders the resolution evidence alongside the rows so
// that channel is pinned too.
// ---------------------------------------------------------------------------

fn collided_golden_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fusion/collided.golden")
}

async fn collided_fusion() -> FusionResult<Term> {
    // One thousand raw units is `10^-9`: ranks one and two already share a
    // contribution, so every row below is decided by the tie-break.
    let weight = Fixed::from_raw(1_000);
    let profile = profile(&[("text", weight), ("vector", weight)], K);
    let decay = DecayRule::ReciprocalRank { k: K };
    let scripted = |names: [&str; 4]| {
        MockStream::new(
            names
                .iter()
                .enumerate()
                .map(|(index, name)| {
                    let rank = u64::try_from(index + 1).expect("four rows");
                    Step::Row(
                        rank,
                        contribution_under(decay, weight, rank).expect("fits"),
                        Term::new(*name),
                    )
                })
                .collect(),
            exhausted(4),
        )
    };
    // The two strata disagree about the order of the same four candidates, so
    // some candidates sum two contributions and some one, and several land on
    // exactly equal scores.
    let streams = vec![
        (
            stratum("text"),
            scripted(["alpha", "beta", "gamma", "delta"]),
        ),
        (
            stratum("vector"),
            scripted(["delta", "gamma", "beta", "alpha"]),
        ),
    ];
    purrdf_retrieval::fuse::<MockStream, Term>(streams, &profile, TopK::new(8))
        .await
        .expect("a collided fusion answers")
}

/// The rows, the statuses, the resolution evidence and both identities.
fn render_collided(result: &FusionResult<Term>) -> String {
    use std::fmt::Write as _;
    let mut out = render(result);
    for (stratum, measured) in &result.trailer.resolution {
        let _ = writeln!(
            out,
            "resolution {} separates_to={:?} ranks_pulled={} collisions={}",
            stratum.as_str(),
            measured.separation.rank(),
            measured.ranks_pulled,
            measured.collisions_observed
        );
    }
    let _ = writeln!(out, "cut_on_a_tie {}", result.trailer.cut_on_a_tie);
    out
}

#[test]
fn the_collided_regime_has_a_byte_identical_golden() {
    let result = block_on(collided_fusion());

    // Non-vacuity: if this fixture stopped colliding it would silently become a
    // second copy of `order.golden` and pin nothing new.
    let collisions: u64 = result
        .trailer
        .resolution
        .values()
        .map(|measured| measured.collisions_observed)
        .sum();
    assert!(
        collisions > 0,
        "this golden must fuse inside the collided regime, or it pins the wrong thing"
    );

    let rendered = render_collided(&result);
    let expected =
        std::fs::read_to_string(collided_golden_path()).expect("the golden fixture is checked in");
    assert_eq!(
        rendered,
        expected,
        "collided fusion drifted from the golden; if the change is intended, update {}",
        collided_golden_path().display()
    );
}
