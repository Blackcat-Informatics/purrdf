// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The fusion contract: verified-protocol NRA over exact fixed point.
//!
//! Every test drives the public surface. Mock producers are scripted row by row
//! so a protocol violation can be produced deliberately; the oracle recomputes
//! the fused order from first principles and must agree with the engine.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Wake, Waker};

use pretty_assertions::assert_eq;
use purrdf_retrieval::{
    AdmissionEnvironment, CandidateDomains, ClassWidth, Completeness, DecayRule, DomainTag,
    DuplicatePolicy, EvidenceId, ExclusionVerdict, Fixed, FusedRow, FusionError, FusionProfile,
    FusionProfileId, FusionResult, FusionStream, IndexGeneration, Iri, MonotoneDepth,
    OrderFidelity, PfAttestation, PlanId, ProducerReceipt, ProducerStatus, ProtocolError, RECIP_K,
    RankFidelity, RankedRow, RankedStream, RankedStreamImpl, RequestTerm, RetrievalRequest,
    RowBlock, ScoreExactness, ScoreInterval, ServiceLevel, Statistics, StreamContract,
    StreamEnding, Term, ToleratedDepth, TopK, contribution, contribution_under,
};
use purrdf_sparql_eval::{
    AcceptedTerm, ExclusionBasis, MemoryRelation, PropertyFunctionRegistry, RankedDeclaration,
    TermKind, TermPattern,
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
    Row(RankedRow<Term>),
    Fail(ProtocolError),
}

/// The contract most fixtures here declare: no repeated item. It is the honest
/// declaration for a scripted stream of distinct items, and it is what
/// `register_ranked`'s own fixtures elsewhere in the crate state. The contiguous,
/// ascending ranks those scripts emit are not part of it -- that law holds for
/// every stream and is checked rank by rank rather than declared.
fn unique_items() -> StreamContract {
    StreamContract::new(
        DuplicatePolicy::Unique,
        RankFidelity::EXACT,
        CandidateDomains::Unrestricted,
        ExclusionBasis::Unavailable,
    )
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
    /// What this stream attests about the index behind its rows. A scripted
    /// stream descends from no index at all, so the honest default is the
    /// undeclared attestation; the evidence fixtures state their own.
    attestation: PfAttestation,
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
            contract: unique_items(),
            attestation: PfAttestation::UNDECLARED,
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

    /// The same producer, attesting `attestation` about the index behind its
    /// rows. A fixture that says nothing keeps the default every stream that
    /// descends from no index honestly reports.
    fn attesting(mut self, attestation: PfAttestation) -> Self {
        self.attestation = attestation;
        self
    }
}

// The trait's methods are `async`; the mock's bodies are synchronous because
// its rows are pre-scripted. The `async` keyword is required to implement the
// trait, not a signal that this body awaits.
#[allow(clippy::unused_async_trait_impl)]
impl RankedStream for MockStream {
    type Item = Term;

    async fn next(&mut self) -> Result<Option<RankedRow<Self::Item>>, ProtocolError> {
        match self.steps.pop_front() {
            Some(Step::Row(row)) => {
                self.rows_emitted += 1;
                self.pull_counter.fetch_add(1, Ordering::SeqCst);
                Ok(Some(row))
            }
            Some(Step::Fail(error)) => Err(error),
            None => Ok(None),
        }
    }

    async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError> {
        Ok(self.receipt.clone())
    }

    fn contract(&self) -> StreamContract {
        self.contract.clone()
    }

    /// This stream declares no exclusion basis, so being asked for a verdict is
    /// the disagreement [`ProtocolError::ExclusionUnavailable`] names rather
    /// than a question it could answer. Fusion never asks it; a hand-written
    /// caller that did would be told so.
    async fn exclusion(&mut self, _candidate: &Term) -> Result<ExclusionVerdict, ProtocolError> {
        Err(ProtocolError::ExclusionUnavailable)
    }

    fn plan_id(&self) -> Option<PlanId> {
        self.plan_id
    }

    fn attestation(&self) -> PfAttestation {
        self.attestation.clone()
    }
}

/// A well-formed row at `rank` for `weight` under `k`.
fn row(rank: u64, weight: Fixed, k: u32, item: &str) -> Step {
    Step::Row(RankedRow::new(
        rank,
        contribution(weight, rank, k).expect("fixture contribution fits"),
        Term::new(item),
        // No block, which is what the `Unrestricted` declaration most fixtures
        // here make owes. `row_in` is the same row drawn from a named block.
        RowBlock::Undeclared,
    ))
}

/// The same row, drawn from the block `block` names.
///
/// A restricted stream owes this on every row: its declaration is a promise
/// about where its candidates lie, and the block is where an individual row
/// backs it.
fn row_in(rank: u64, weight: Fixed, k: u32, item: &str, block: &str) -> Step {
    Step::Row(RankedRow::new(
        rank,
        contribution(weight, rank, k).expect("fixture contribution fits"),
        Term::new(item),
        RowBlock::Declared(domain(block)),
    ))
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
            vec![Step::Row(RankedRow::new(
                1,
                Fixed::from_raw(1),
                Term::new("a"),
                RowBlock::Undeclared,
            ))],
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

// 7a. The one rank law, and the valid streams next door to each refusal.
/// **The rank law is single, unconditional, and not over-applied.**
///
/// There is exactly one law about ranks -- 1-based, contiguous, ascending -- and
/// no registration softens it, so a stream that steps backwards and a stream
/// that skips are both refused whatever their producer declared. A refusal is a
/// claim, though, and a refusal that also swallowed the nearest conforming
/// stream would look identical from inside the error. So each violation is
/// executed beside its neighbour: the same stratum, the same weight, the same
/// item names, the same row count where the shape allows it, differing only in
/// the one rank that breaks the law. The neighbour must still fuse, and fuse to
/// every row it emitted.
#[test]
fn a_backwards_rank_and_a_skipped_rank_are_refused_while_their_valid_neighbours_fuse() {
    let profile = profile(&[("text", Fixed::ONE)], K);
    let weight = Fixed::ONE;

    // The neighbour of the backwards stream: two items at 1 then 2, which is the
    // law kept exactly.
    let ascending = vec![(
        stratum("text"),
        MockStream::new(
            vec![row(1, weight, K, "a"), row(2, weight, K, "b")],
            exhausted(2),
        ),
    )];
    let fused = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        ascending, &profile, TOP_K,
    ))
    .expect("ranks 1 then 2 keep the law and must fuse");
    assert_eq!(
        fused.rows.len(),
        2,
        "and every row the stream emitted reaches the answer"
    );
    assert_eq!(fused.rows[0].entity, Term::new("a"));
    assert_eq!(fused.rows[1].entity, Term::new("b"));

    // The violation: the second row repeats rank 1 instead of advancing to 2.
    let backwards = vec![(
        stratum("text"),
        MockStream::new(
            vec![row(1, weight, K, "a"), row(1, weight, K, "b")],
            exhausted(2),
        ),
    )];
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        backwards, &profile, TOP_K,
    ));
    assert!(
        matches!(
            &result,
            Err(FusionError::Protocol(error))
                if matches!(**error, ProtocolError::OutOfOrderRanks { expected: 2, got: 1 })
        ),
        "expected OutOfOrderRanks, got {result:?}"
    );

    // The neighbour of the skipping stream: three items at 1, 2, 3 -- the ranks
    // the skipping stream would have had to emit to reach its own rank 3.
    let contiguous = vec![(
        stratum("text"),
        MockStream::new(
            vec![
                row(1, weight, K, "a"),
                row(2, weight, K, "b"),
                row(3, weight, K, "c"),
            ],
            exhausted(3),
        ),
    )];
    let fused = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        contiguous, &profile, TOP_K,
    ))
    .expect("ranks 1, 2 then 3 keep the law and must fuse");
    assert_eq!(
        fused.rows.len(),
        3,
        "a deeper conforming stream is read to its end, not truncated at the \
         depth the refused one stopped at"
    );
    assert_eq!(fused.rows[2].entity, Term::new("c"));

    // The violation: rank 2 is never emitted, so rank 3 arrives one rank early.
    let skipping = vec![(
        stratum("text"),
        MockStream::new(
            vec![row(1, weight, K, "a"), row(3, weight, K, "c")],
            exhausted(2),
        ),
    )];
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        skipping, &profile, TOP_K,
    ));
    assert!(
        matches!(
            &result,
            Err(FusionError::Protocol(error))
                if matches!(**error, ProtocolError::NonContiguousRanks { gap: 1 })
        ),
        "expected NonContiguousRanks, got {result:?}"
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

// 11c. Decoded bytes are held to the weight law, which is what makes the
//      profile's contribution curve non-increasing for every stream fused.
//
// Fusion re-derives every row's contribution and refuses a disagreement, so a
// stream cannot present a value that rises with rank *provided the profile's own
// curve never rises*. That holds for a strictly positive weight and fails for a
// negative one: the truncated rule multiplies a shrinking reciprocal by a
// negative weight and the product climbs toward zero, so every adjacent pair
// rises. `with_decay` refuses such a weight, and this test is the other door —
// hand-built canonical bytes — held to the same law.

/// The raw magnitude the negative-weight cases below use: one whole unit, so
/// the truncated rule's reciprocal still carries it at both ranks compared.
const NEGATIVE_WEIGHT_UNITS: i128 = 1_000_000_000_000;

#[test]
fn decoded_profile_bytes_are_held_to_the_positive_weight_law() {
    // One stratum, so the weight is the last `i128` before the fixed tail:
    // tie-break tag (1), contribution maximum (4), scale (4), rounding (1),
    // item encoding (1).
    const TAIL: usize = 1 + 4 + 4 + 1 + 1;
    let smallest = Fixed::from_raw(1);
    let bytes = profile(&[("text", smallest)], K).canonical_bytes();
    let offset = bytes.len() - TAIL - 16;
    assert_eq!(
        i128::from_le_bytes(
            bytes[offset..offset + 16]
                .try_into()
                .expect("sixteen bytes are sixteen bytes")
        ),
        1,
        "the field this test rewrites is the stratum weight"
    );

    // The valid neighbour, and it is the *nearest* one: a single raw unit is the
    // smallest weight a profile admits at all. It decodes, and the curve it
    // decodes to never rises.
    let decoded =
        FusionProfile::from_canonical_bytes(&bytes).expect("the smallest positive weight decodes");
    let mut previous = contribution_under(decoded.decay(), smallest, 1).expect("rank one fits");
    for rank in 2..512 {
        let current =
            contribution_under(decoded.decay(), smallest, rank).expect("a 1-based rank fits");
        assert!(
            current <= previous,
            "a decoded profile's curve rose from {previous:?} to {current:?} at rank {rank}"
        );
        previous = current;
    }

    // The invalid case. Zero and a negative raw unit are both refused, and the
    // negative one is refused before anything can observe the rise it would
    // otherwise produce at every rank.
    for forged in [0_i128, -1, -NEGATIVE_WEIGHT_UNITS] {
        let mut tampered = bytes.clone();
        tampered[offset..offset + 16].copy_from_slice(&forged.to_le_bytes());
        assert!(
            matches!(
                FusionProfile::from_canonical_bytes(&tampered),
                Err(FusionError::NonPositiveWeight { .. })
            ),
            "canonical bytes declaring a weight of {forged} must be refused"
        );
    }

    // What the refusal above is protecting: read directly, a negative weight's
    // contribution rises with every rank, so a stream emitting exactly the value
    // the profile computes would carry a rising sequence into fusion.
    let negative = Fixed::from_raw(-NEGATIVE_WEIGHT_UNITS);
    let decay = DecayRule::ReciprocalRank { k: K };
    assert!(
        contribution_under(decay, negative, 2).expect("rank two fits")
            > contribution_under(decay, negative, 1).expect("rank one fits"),
        "a negative weight is the case the profile refuses, and this is why"
    );
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
//     (below) or a stream emitted one candidate twice — and the second of those
//     is refused as the protocol violation it is (`ProtocolError`'s
//     `DuplicateItem`, test 20) before any count can cross. `fuse` refuses a
//     repeated stratum before a row is read, so this drives `FusionStream`
//     directly — which is the honest way to reach an invariant violation that
//     no conforming input produces.
//
//     The mechanism is the repeated stratum *tag* and nothing else, which is
//     worth stating because the engine now also refuses a repeat whose earlier
//     occurrence has already been emitted. That refusal cannot pre-empt this
//     one: `a` is still an un-emitted frontier candidate when the third stream
//     names it — the other streams are open and have not all named it, so it is
//     not final and cannot have certified — so the emitted map is empty here
//     and the contribution count is what the third contribution crosses. The
//     property enumeration in test 20b asserts the converse for every
//     conforming shape: no stream set built from distinct strata provokes this
//     error at all.
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
//     exactly, at `ceiling() / 2`.
//
//     This is why the ceiling needs no runtime check and no refusal of its own:
//     the most aggressive legitimate load the arithmetic can produce still stops
//     at half of it, so a sum held to the contribution count is held below the
//     ceiling for free. The property is asserted here rather than assumed.
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

// 16. The contribution count is the refusal, at the one configuration whose raw
//     arithmetic lands exactly on the declared ceiling.
//
//     Two equal-weight, rank-1, `K = 1` contributions under a profile that
//     weights one stratum, and therefore admits one contribution: the sum is
//     exactly `ceiling()`, and it is refused as
//     `MaxContributionsExceeded` — because reaching that sum at all requires
//     contributing twice under a one-stratum profile. Every configuration with a
//     valid contribution count stops at `ceiling() / 2` (test 15), which is why
//     the score has no second, ceiling-shaped refusal behind this one: nothing
//     could reach it. This drives `FusionStream` directly (bypassing `fuse`'s
//     duplicate-stratum guard) because getting here needs two streams sharing
//     one stratum tag, which `fuse` itself refuses before fusion ever begins.
//
//     As in test 13, the mechanism is the shared stratum tag and not a repeat
//     within one stream: `a` is named by the second stream while the first has
//     already ended but `a` is still in the frontier un-emitted — it cannot
//     have certified, because the second stream is open and has not named it —
//     so nothing is in the emitted map and the contribution count is what the
//     second contribution crosses.
#[test]
fn a_sum_landing_on_the_ceiling_is_refused_by_the_contribution_count() {
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

// 15. The bound stops the reading, not only the returning — over ONE stratum.
//
// **The single-stratum case is degenerate, and the name says so.** With one
// stream open there is no rival stratum that could still contribute to a
// candidate, so the finality test every certification rests on is satisfied by
// the first row pulled and the bound stops the read whatever the engine's
// threshold arithmetic does. That is worth pinning — it is the shape a
// single-producer request really has — but it is *not* evidence that the reading
// is bounded in general, and a version of this test that claimed to be would
// have stayed green throughout the fused top-k drain defect, which lived
// entirely in the multi-stratum threshold.
//
// The load-bearing claim over several strata is
// `declared_domains_bound_the_reading_over_disjoint_strata`, which measures the
// same quantity across two disjoint strata and carries the counter-measurement:
// without a domain declaration the identical streams drain, because with two
// open streams nothing licenses an early stop.
#[test]
fn a_bounded_stop_closes_a_single_stratum_stream_instead_of_draining_it() {
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
            .declaring(unique_items()),
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
    StreamContract::new(
        DuplicatePolicy::Allowed,
        RankFidelity::EXACT,
        CandidateDomains::Unrestricted,
        ExclusionBasis::Unavailable,
    )
}

/// Two streams: `dense` as scripted, and a one-row `sparse` stream that stays
/// open long enough to keep `dense`'s first candidate in the frontier.
///
/// Without the second stratum nothing is held: a single-stratum candidate is
/// final the moment it is read, so it certifies and leaves the frontier before
/// the next row arrives. The fixture is two strata because the frontier is *one*
/// of the two places a declared-`Unique` stream's promise is checked, and this
/// shape is what reaches it; the single-stratum fixtures below reach the other.
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

/// A declared-`Unique` stream's repeat is refused while the earlier occurrence
/// is still an un-emitted frontier candidate.
///
/// This is the cheap half of the check — the frontier already holds the
/// `seen_streams` set the refusal reads — and it is no longer the whole of it.
/// The name carries no "while unemitted" qualifier because the behaviour has
/// none: the repeat is refused either way, and the sibling test below is the
/// same refusal reached after the earlier occurrence has left the frontier.
#[test]
fn a_declared_unique_streams_repeat_is_refused() {
    let profile = dense_sparse_profile();
    // `a` cannot be emitted before the duplicate is read: the sparse stream is
    // open and has not named it, so it is not final and stays in the frontier.
    // That makes this the in-frontier arm specifically, and the assertion below
    // therefore pins the arm as well as the outcome.
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
                if matches!(
                    &**error,
                    ProtocolError::DuplicateItem { item, stratum }
                        if item == "a" && stratum == self::stratum("dense").as_str()
                )
        ),
        "expected DuplicateItem naming `a` and the dense stratum, got {result:?}"
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

/// What a `Unique` declaration promises and what breaking it costs, stated as
/// one test because they are one decision.
///
/// The promise is held for the whole fusion, not merely while the earlier
/// occurrence is still in the frontier. A candidate leaves the frontier the
/// instant it certifies, so the frontier's own once-per-stream check cannot see
/// a repeat that arrives afterwards — and if nothing else did, the same entity
/// would reach the caller twice, the second time scored from one late rank
/// alone. It does not: the engine keeps a record of what it has emitted, and a
/// stream that contradicts its own declaration is told so, naming the entity and
/// the stratum.
///
/// Refusing rather than silently dropping the late repeat is the deliberate
/// half. A `Unique` declaration is a claim about the producer's index, and a
/// producer that breaks it has a stream whose ranks are no longer trustworthy —
/// returning a plausible answer computed from it would hide exactly the fact the
/// consumer needs.
///
/// A producer that cannot make the promise has a complete, supported answer one
/// line away, and the second half of this test is that answer: the same stream
/// declared `Allowed` is de-duplicated in full.
#[test]
fn a_unique_declarations_repeat_is_refused_past_the_frontier_and_allowed_is_the_remedy() {
    // One stratum, so `a` certifies the moment it is read and the repeat arrives
    // after it left the frontier — which is the arm this test exists for.
    let profile = profile(&[("dense", Fixed::ONE)], K);
    let repeated = vec![(
        stratum("dense"),
        MockStream::new(
            vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "a")],
            exhausted(2),
        ),
    )];
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        repeated, &profile, TOP_K,
    ));
    assert!(
        matches!(
            &result,
            Err(FusionError::Protocol(error))
                if matches!(
                    &**error,
                    ProtocolError::DuplicateItem { item, stratum }
                        if item == "a" && stratum == self::stratum("dense").as_str()
                )
        ),
        "a repeat past the frontier must be refused naming `a` and the dense \
         stratum, got {result:?}"
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
        rows_of(unique_items()),
        "with no repeat to remove, the policy is invisible in the answer"
    );
}

/// **The reported defect, in the shape it was reported: one stratum, `[a, b,
/// a]`.**
///
/// This is not a variation on the fixture above, it is the case a user actually
/// hit, kept separate so that a later edit to the general fixtures cannot quietly
/// stop covering it. `a` certifies at rank 1 and leaves the frontier, `b`
/// certifies at rank 2, and only then does the stream name `a` again at rank 3 —
/// the exact interleaving under which the frontier holds nothing to collide
/// with.
///
/// It asserts both halves of the actionable report. The entity, because a
/// consumer has to know which row to distrust; the stratum, because with several
/// producers fused the entity alone does not say which declaration is wrong, and
/// fixing the wrong producer is the same as fixing none.
#[test]
fn the_reported_single_stratum_repeat_is_refused_naming_the_item_and_the_stratum() {
    let profile = profile(&[("dense", Fixed::ONE)], K);
    let streams = vec![(
        stratum("dense"),
        MockStream::new(
            vec![
                row(1, Fixed::ONE, K, "a"),
                row(2, Fixed::ONE, K, "b"),
                row(3, Fixed::ONE, K, "a"),
            ],
            exhausted(3),
        ),
    )];
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        streams, &profile, TOP_K,
    ));
    let Err(FusionError::Protocol(error)) = &result else {
        panic!("expected a protocol refusal, got {result:?}");
    };
    let ProtocolError::DuplicateItem {
        item,
        stratum: named,
    } = &**error
    else {
        panic!("expected DuplicateItem, got {error:?}");
    };
    assert_eq!(item, "a", "the refusal must name the repeated entity");
    assert_eq!(
        named,
        stratum("dense").as_str(),
        "the refusal must name the stratum whose producer broke its declaration"
    );

    // THE NEIGHBOURING CASE, and it is the one over-refusal would break: the
    // same three rows with no repeat fuse into three rows and no refusal at all.
    let clean = vec![(
        stratum("dense"),
        MockStream::new(
            vec![
                row(1, Fixed::ONE, K, "a"),
                row(2, Fixed::ONE, K, "b"),
                row(3, Fixed::ONE, K, "c"),
            ],
            exhausted(3),
        ),
    )];
    let fused = block_on(run_fuse(clean, &profile));
    assert_eq!(
        fused
            .rows
            .iter()
            .map(|fused_row| fused_row.entity.clone())
            .collect::<Vec<_>>(),
        vec![Term::new("a"), Term::new("b"), Term::new("c")],
        "a stream of distinct items is not a duplicate and must answer in full"
    );
}

// ---------------------------------------------------------------------------
// 20b. The answer invariant, unconditionally: no fused answer ever contains one
//      entity twice.
//
// The tests above pin named interleavings. This one pins the *property*, over
// every shape the engine admits, because the invariant is not "we handled the
// reported case" — it is a statement about all of them, and a statement about
// all of them has to be checked over more than one.
// ---------------------------------------------------------------------------

/// The per-stratum row scripts the property enumeration draws from.
///
/// Chosen to cover the interleavings the invariant is about rather than to be
/// random: distinct items, an immediate repeat, a repeat separated by another
/// item (the reported defect's shape), a repeat at the very end of a longer
/// stream, and scripts that overlap across strata so candidates genuinely have
/// to be held in the frontier while other strata catch up.
const PROPERTY_SCRIPTS: [&[&str]; 6] = [
    &["a"],
    &["a", "b"],
    &["a", "b", "a"],
    &["b", "a", "b", "c"],
    &["c", "c"],
    &["a", "b", "c", "a"],
];

/// **A fused answer never contains the same entity twice — for every stream
/// set, every declared policy and every bound.**
///
/// Deterministic by enumeration rather than by a seeded generator, which is the
/// stronger of the two here: this crate forbids nondeterminism, and an
/// enumeration has no seed to drift, no shrinking step whose report depends on
/// the run, and a failure that reproduces from the printed configuration alone.
/// Every combination of one to four strata drawn from [`PROPERTY_SCRIPTS`], both
/// [`DuplicatePolicy`] spellings, and every bound from zero to one past the
/// longest script is executed through the shipped [`purrdf_retrieval::fuse`].
///
/// Exactly two outcomes are admissible, and the test asserts the disjunction
/// rather than either branch:
///
/// * the fusion answered, and the entities in that answer are pairwise
///   distinct; or
/// * the fusion refused with [`ProtocolError::DuplicateItem`], which is the
///   declared-`Unique` producer being told its promise is broken.
///
/// Anything else fails, and two "anything else"s are worth naming. A
/// [`FusionError::MaxContributionsExceeded`] here would mean the contribution
/// bound had become reachable from conforming input — it is an invariant
/// violation and no stream set built from distinct strata may provoke one. And
/// a [`FusionError::MalformedProfile`] would mean the post-emission check's
/// impossibility proof is wrong. Both are caught by the same `else` arm, which
/// prints the whole configuration.
#[test]
fn no_fused_answer_ever_contains_one_entity_twice() {
    /// The strata names the enumeration tags its streams with, in order. Four,
    /// because the profile's contribution maximum is the stratum count and four
    /// distinct strata is the widest shape these scripts need.
    const NAMES: [&str; 4] = ["s0", "s1", "s2", "s3"];

    let mut configurations = 0_u32;
    let mut refusals = 0_u32;
    let mut answers = 0_u32;

    for stratum_count in 1..=NAMES.len() {
        // A fixed mixed-radix odometer over the script catalogue: configuration
        // `n` gives stratum `i` the script at digit `i` of `n` in base
        // `PROPERTY_SCRIPTS.len()`. Total order, no randomness, and the index is
        // printable, so a failure names the exact stream set that produced it.
        let combinations = PROPERTY_SCRIPTS
            .len()
            .pow(u32::try_from(stratum_count).expect("at most four strata fit in a u32 exponent"));
        for combination in 0..combinations {
            let scripts: Vec<&[&str]> = (0..stratum_count)
                .map(|position| {
                    let digit = (combination
                        / PROPERTY_SCRIPTS
                            .len()
                            .pow(u32::try_from(position).expect("at most four positions")))
                        % PROPERTY_SCRIPTS.len();
                    PROPERTY_SCRIPTS[digit]
                })
                .collect();
            let longest = scripts
                .iter()
                .map(|script| script.len())
                .max()
                .expect("at least one stratum");

            for policy in [DuplicatePolicy::Unique, DuplicatePolicy::Allowed] {
                // Zero through one past the longest script: zero is the bound
                // that must certify nothing at all, and one past the longest is
                // the bound that cannot be what stopped the run.
                for bound in 0..=longest + 1 {
                    configurations += 1;
                    let weights: Vec<(&str, Fixed)> = NAMES[..stratum_count]
                        .iter()
                        .map(|name| (*name, Fixed::ONE))
                        .collect();
                    let profile = profile(&weights, K);
                    let streams: Vec<(Iri, MockStream)> = scripts
                        .iter()
                        .enumerate()
                        .map(|(position, script)| {
                            let steps: Vec<Step> = script
                                .iter()
                                .enumerate()
                                .map(|(offset, item)| {
                                    let rank = u64::try_from(offset + 1).expect("small rank");
                                    row(rank, Fixed::ONE, K, item)
                                })
                                .collect();
                            let emitted = u64::try_from(script.len()).expect("short script");
                            (
                                stratum(NAMES[position]),
                                MockStream::new(steps, exhausted(emitted)).declaring(
                                    StreamContract::new(
                                        policy,
                                        RankFidelity::EXACT,
                                        CandidateDomains::Unrestricted,
                                        ExclusionBasis::Unavailable,
                                    ),
                                ),
                            )
                        })
                        .collect();

                    let context = format!(
                        "strata {stratum_count}, combination {combination}, policy \
                         {policy:?}, bound {bound}, scripts {scripts:?}"
                    );
                    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
                        streams,
                        &profile,
                        TopK::new(bound),
                    ));
                    match result {
                        Ok(fused) => {
                            answers += 1;
                            let mut distinct = BTreeSet::new();
                            for fused_row in &fused.rows {
                                assert!(
                                    distinct.insert(fused_row.entity.clone()),
                                    "a fused answer contained {:?} twice — {context}",
                                    fused_row.entity
                                );
                            }
                            assert!(
                                fused.rows.len() <= bound,
                                "the answer exceeded its own bound — {context}"
                            );
                        }
                        Err(FusionError::Protocol(error))
                            if matches!(&*error, ProtocolError::DuplicateItem { .. }) =>
                        {
                            refusals += 1;
                            assert_eq!(
                                policy,
                                DuplicatePolicy::Unique,
                                "an `Allowed` stream is de-duplicated, never refused — {context}"
                            );
                        }
                        Err(other) => {
                            panic!(
                                "neither a distinct answer nor a duplicate refusal: {other:?} — {context}"
                            );
                        }
                    }
                }
            }
        }
    }

    // The crossing guards. Without them this test would pass over an
    // enumeration that never ran, or one in which every single configuration
    // refused and "the entities are distinct" was never actually checked.
    assert_eq!(
        configurations, 17_720,
        "the enumeration must be the fixed size it claims, or it has silently \
         changed what it covers"
    );
    assert!(
        refusals > 0,
        "no configuration was refused, so the refusal branch is untested"
    );
    assert!(
        answers > 0,
        "no configuration answered, so the distinctness branch is untested"
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
                Step::Row(RankedRow::new(
                    1,
                    first,
                    Term::new("a"),
                    RowBlock::Undeclared,
                )),
                Step::Row(RankedRow::new(
                    2,
                    second,
                    Term::new("b"),
                    RowBlock::Undeclared,
                )),
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
    // `DecayRule` (`reciprocal_rank::tests::every_decay_rule_is_non_increasing_in_the_rank`),
    // and enforced where a stream meets it, by this re-derivation. There is no
    // second, ordering-shaped refusal behind it: nothing could reach one.
    let first_rank = contribution(Fixed::ONE, 1, K).expect("fits");
    let risen = Fixed::from_raw(first_rank.into_raw() + 1);
    for contract in [unique_items(), allowed_duplicates()] {
        let heavy = crate::profile(&[("thin", Fixed::ONE)], K);
        let streams = vec![(
            stratum("thin"),
            MockStream::new(
                vec![
                    row(1, Fixed::ONE, K, "a"),
                    Step::Row(RankedRow::new(
                        2,
                        risen,
                        Term::new("b"),
                        RowBlock::Undeclared,
                    )),
                ],
                exhausted(2),
            )
            .declaring(contract.clone()),
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
            Step::Row(RankedRow::new(
                rank,
                contribution_under(decay, weight, rank).expect("fixture contribution fits"),
                Term::new(format!("d{rank:07}")),
                RowBlock::Undeclared,
            ))
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
    // The middle of the three weights this file pins: 1e6 raw units collides at
    // rank 972, well inside a 1400-row stream, and every one of those rows is
    // well formed in every respect a producer controls.
    let decay = DecayRule::ReciprocalRank { k: K };
    let weight = Fixed::from_raw(1_000_000);
    let profile = deep_profile(decay, weight);

    // Witness one, and the reason this fixture tests anything: bare arithmetic
    // on the decay rule says these ranks really do collide inside the stream.
    let collision = first_collision_by_walking(decay, weight, 1_400)
        .expect("this fixture must collide inside the stream it fuses");
    assert_eq!(
        collision, 972,
        "the measured collision rank for this weight"
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
    // The trap a small bound sets, pinned as a positive test rather than left as
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
    // The lightest of the three weights: one thousand raw units collides
    // immediately, so the shortest possible stream already carries a repeated
    // contribution. This is the exact case that was refused as
    // `RepeatedContribution` at rank two.
    let decay = DecayRule::ReciprocalRank { k: K };
    let weight = Fixed::from_raw(1_000);
    let profile = deep_profile(decay, weight);

    let first = contribution_under(decay, weight, 1).expect("fits");
    let second = contribution_under(decay, weight, 2).expect("fits");
    assert_eq!(first, second, "1000/61 and 1000/62 both truncate to 16");
    assert_eq!(first.into_raw(), 16, "the measured value at this weight");

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
fn every_weight_in_the_measurement_table_fuses() {
    // All three weights, end to end. They differ only in how soon the decay
    // stops separating ranks; none of them is a protocol violation and all three
    // answer. The two lighter ones used to be refused.
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
    // The tests above exercise only the truncated rule. The weighted rule has
    // its own boundary, near `sqrt(w_raw) - k` rather than near `sqrt(S)`, and
    // the same law applies there: a collision is quantization, not a violation.
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
                Step::Row(RankedRow::new(
                    1,
                    plateau,
                    Term::new("a"),
                    RowBlock::Undeclared,
                )),
                Step::Row(RankedRow::new(
                    2,
                    forged,
                    Term::new("b"),
                    RowBlock::Undeclared,
                )),
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
                Step::Row(RankedRow::new(
                    1,
                    plateau,
                    Term::new("a"),
                    RowBlock::Undeclared,
                )),
                Step::Row(RankedRow::new(
                    2,
                    plateau,
                    Term::new("b"),
                    RowBlock::Undeclared,
                )),
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
    // A trailer asserts a terminal status per producer, which requires pulling
    // from each stream until it yields its first row or its receipt. This stream
    // has a first row, so `ranks_pulled` is one even under a bound of zero:
    // a caller reading `== 0` as "this stream was never touched" would be wrong
    // here, and that is worth pinning.
    //
    // Zero is nonetheless reachable, and it means something narrower than
    // "untouched" — see `an_empty_stream_reports_zero_ranks_pulled` below, which
    // is the case this one does not cover.
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
fn an_empty_stream_reports_zero_ranks_pulled() {
    // The other end of the counter, and the case `ranks_pulled`'s own
    // documentation has to be true at: a producer that ran, found nothing and
    // said so. `next_ranks` advances only when a row arrives, so nothing
    // advances it here and the reported depth is zero.
    //
    // Zero therefore reports "this stream yielded no rows", not "this stream was
    // never asked" — it was asked, and it answered with its receipt.
    let decay = DecayRule::ReciprocalRank { k: K };
    let weight = Fixed::from_raw(1_000);
    let profile = deep_profile(decay, weight);

    let empty = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        vec![(stratum("deep"), MockStream::new(Vec::new(), exhausted(0)))],
        &profile,
        TopK::new(10),
    ))
    .expect("a producer with nothing to return is not a protocol violation");
    assert!(empty.rows.is_empty(), "no rows in, no rows out");
    let measured = empty.trailer.resolution[&stratum("deep")];
    assert_eq!(measured.ranks_pulled, 0, "no row arrived, so no rank did");
    assert_eq!(
        measured.collisions_observed, 0,
        "no row has no adjacent pair to collide with"
    );
    // The witness that the stream really was read rather than skipped, and it
    // does not come from the counter under test: the producer's own receipt,
    // which fusion only holds because it pulled until the stream ended.
    assert_eq!(
        empty.trailer.statuses[&stratum("deep")],
        ProducerStatus::Exhausted { rows_emitted: 0 }
    );

    // THE NEIGHBOURING CASE, so the zero above means something: the same
    // profile, the same bound, one row on the stream. Everything that differs
    // between the two is that row.
    let one_row = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        vec![(stratum("deep"), deep_stream(decay, weight, 1))],
        &profile,
        TopK::new(10),
    ))
    .expect("a one-row stream fuses");
    assert_eq!(one_row.rows.len(), 1);
    assert_eq!(
        one_row.trailer.resolution[&stratum("deep")].ranks_pulled,
        1,
        "one row pulled is one rank pulled"
    );
    assert_eq!(
        one_row.trailer.resolution[&stratum("deep")].separation,
        measured.separation,
        "separation is a property of the law and does not depend on what arrived"
    );
}

#[test]
fn the_resolution_map_keys_every_weighted_stream_including_one_that_yielded_nothing() {
    // The key set, pinned exactly. Two weighted strata, one of which returns
    // nothing: both are keyed, because `separation` is the resolution recorded
    // for a stratum whether or not rows arrived and `ranks_pulled` of zero is
    // how a caller learns none did. An absent key would have to be told apart
    // from a stratum the profile never weighted, which is a different fact
    // entirely.
    let profile = profile(&[("dense", Fixed::ONE), ("barren", Fixed::ONE)], K);
    let streams = vec![
        (
            stratum("dense"),
            MockStream::new(
                vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "b")],
                exhausted(2),
            ),
        ),
        (stratum("barren"), MockStream::new(Vec::new(), exhausted(0))),
    ];
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        streams,
        &profile,
        TopK::new(10),
    ))
    .expect("one empty stratum does not stop a fusion");

    let keys: Vec<&str> = result.trailer.resolution.keys().map(Iri::as_str).collect();
    assert_eq!(
        keys,
        vec![stratum("barren").as_str(), stratum("dense").as_str()],
        "every weighted stream is keyed, in canonical stratum order"
    );
    assert_eq!(
        result.trailer.resolution[&stratum("barren")].ranks_pulled,
        0,
        "and the one that yielded nothing says so in its own entry"
    );
    assert_eq!(
        result.trailer.resolution[&stratum("dense")].ranks_pulled,
        2,
        "while the one that yielded rows reports them"
    );

    // And the one direction the two maps really do differ in: `statuses` grows
    // by the producers that never became streams, and `resolution` does not,
    // because a producer that was never read from has no read to report.
    let completed = result
        .trailer
        .completed_with([(stratum("absent"), ProducerStatus::TermsRejected)]);
    assert_eq!(
        completed.statuses.keys().collect::<Vec<_>>(),
        vec![&stratum("absent"), &stratum("barren"), &stratum("dense")],
        "a producer that never became a stream still gets a status"
    );
    assert_eq!(
        completed.resolution.keys().collect::<Vec<_>>(),
        vec![&stratum("barren"), &stratum("dense")],
        "but not a resolution entry, so the key set stays a subset of the statuses'"
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
        !profile
            .class_width(&stratum("deep"), 200)
            .expect("the arithmetic evaluates")
            .expect("weighted")
            .fits_within(1),
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
    // THE PAIR, one step apart, on BOTH rules. The limit here is the 32-bit
    // depth field a plan carries, and the refusal past it must say so: the
    // folded rule's arithmetic has not run out of anything at `u32::MAX + 1` —
    // it answers at `u32::MAX` — so reporting this as the rule saturating would
    // be the same defect as refusing a valid stream for a property of the
    // consumer's own internal clamp.
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

    // The truncated rule saturates long before the range a plan can record runs
    // out, so at `u32::MAX` it refuses as saturation — and one step further it
    // refuses for the range instead, because that is the first fact about the
    // request that is wrong, and it is wrong whatever the rule.
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
    // refusal names the depth encoding a plan carries and never says the rule
    // stopped separating; `MonotoneDepth::SeparatesBeyondAnyPlan` documents
    // itself as "there is no bound to report", so a refusal rendering it would
    // assert saturation and its absence in one sentence.
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
    // Where a unit weight runs out, asserted symbolically rather than by
    // enumerating a million rows. The inner truncation is a ceiling: raising
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
            Some(ClassWidth::SpansTo(1)),
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
        Some(ClassWidth::SpansTo(2)),
        "the bound is where the first collision begins"
    );

    // Past it the classes are wider, and they keep widening. Both are counted
    // widths at this weight — the run ends well inside the expressible range —
    // so the comparison below is between two measurements and not between two
    // readings of the same ceiling.
    let near = profile
        .class_width(&stratum, bound + 1)
        .expect("the arithmetic evaluates")
        .expect("weighted")
        .width()
        .expect("this weight's classes end inside the expressible range");
    let far = profile
        .class_width(&stratum, bound * 4)
        .expect("the arithmetic evaluates")
        .expect("weighted")
        .width()
        .expect("this weight's classes end inside the expressible range");
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
        deepest,
        ToleratedDepth::ReadsTo(expected_deepest),
        "tolerating a class of {near} must equal the independently walked depth, and report it \
         as a measured depth rather than as a saturation point"
    );
    assert_eq!(
        profile
            .deepest_rank_within_width(&stratum, 1)
            .expect("the arithmetic evaluates"),
        Some(ToleratedDepth::ReadsTo(bound)),
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
            got,
            ToleratedDepth::ReadsTo(truth),
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
        Some(ToleratedDepth::ReadsTo(bound)),
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
            .expect("weighted")
            .rank()
            .expect("this fixture's tolerated depth is inside a plan's range");

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
fn the_class_at_the_reported_depth_is_the_run_the_bounded_read_never_finishes() {
    // What the reported depth means, asserted rather than described. It is the
    // deepest depth a READ can stop at with every rank it actually read sitting
    // in a class no wider than the tolerance -- NOT the deepest rank whose own
    // class on the unbounded curve is that narrow. The two differ by exactly
    // the rank the bounded read never reaches, so `class_width` at the reported
    // depth is never the tolerance and never one: it is at least the tolerance
    // plus one, being the whole run whose (max_width + 1)-th member ended the
    // walk.
    //
    // The magnitudes are measured against this fixture and pinned beside the
    // inequality, not in place of it. They also show why the relation is an
    // inequality: at a tolerance of fifty the run that ends the walk is two
    // ranks longer than the tolerance rather than one, so equality would be a
    // claim this arithmetic does not make.
    let decay = DecayRule::ReciprocalRank { k: K };
    let weight = Fixed::from_raw(1_000_000);
    let profile = deep_profile(decay, weight);
    let stratum = stratum("deep");

    for (max_width, measured_depth, measured_width) in [
        (1_u64, 972_u64, 2_u64),
        (2, 1382, 3),
        (3, 1712, 4),
        (5, 2222, 6),
        (10, 3144, 11),
        (50, 7132, 52),
    ] {
        let depth = profile
            .deepest_rank_within_width(&stratum, max_width)
            .expect("the arithmetic evaluates")
            .expect("weighted")
            .rank()
            .expect("this fixture's tolerated depth is inside a plan's range");
        assert_eq!(
            depth, measured_depth,
            "max_width {max_width}: the depth this fixture reports"
        );

        let width = profile
            .class_width(&stratum, depth)
            .expect("the arithmetic evaluates")
            .expect("weighted")
            .width()
            .expect("this fixture's classes end inside the expressible range");
        assert!(
            width > max_width,
            "max_width {max_width}: the unbounded class at the reported depth {depth} must be \
             at least {} ranks wide -- it contains the run whose last member is the rank the \
             bounded read never reaches -- and was {width}",
            max_width + 1
        );
        assert_eq!(
            width, measured_width,
            "max_width {max_width}: the width this fixture measures at depth {depth}"
        );
    }
}

#[test]
fn the_bound_at_the_reported_depth_holds_where_the_width_is_nowhere_near_the_tolerance() {
    // The same law as the test above, executed where it is NOT trivially true.
    // At a raw weight of 1_000_000 the run that ends the walk is one or two
    // ranks longer than the tolerance, so `max_width + 1` and the measured width
    // very nearly coincide, and a reader could mistake the inequality for an
    // equality. These weights are three and four orders of magnitude lighter.
    // Their contributions collapse within the first handful of ranks, so a
    // tolerance of one lands on depth ONE and the class there is tens of ranks
    // wide: `>= max_width + 1` is all that holds, and "two at a tolerance of
    // one" is false by a factor of twenty.
    //
    // Every magnitude here was executed against this fixture.
    for (raw, decay, measured_width) in [
        (100_i128, DecayRule::ReciprocalRank { k: K }, 40_u64),
        (100, DecayRule::WeightedReciprocalRank { k: K }, 40),
        (121, DecayRule::ReciprocalRank { k: K }, 60),
        (121, DecayRule::WeightedReciprocalRank { k: K }, 61),
        (200, DecayRule::ReciprocalRank { k: K }, 6),
        (1_000, DecayRule::ReciprocalRank { k: K }, 2),
    ] {
        let weight = Fixed::from_raw(raw);
        let profile = deep_profile(decay, weight);
        let stratum = stratum("deep");

        let depth = profile
            .deepest_rank_within_width(&stratum, 1)
            .expect("the arithmetic evaluates")
            .expect("weighted")
            .rank()
            .expect("a light weight's separating depth is inside a plan's range");
        assert_eq!(
            depth, 1,
            "{decay:?} raw {raw}: these weights stop separating immediately"
        );

        let width = profile
            .class_width(&stratum, depth)
            .expect("the arithmetic evaluates")
            .expect("weighted");
        assert_eq!(
            width,
            ClassWidth::SpansTo(measured_width),
            "{decay:?} raw {raw}: the width this fixture measures at depth {depth}"
        );
        assert!(
            !width.fits_within(1),
            "{decay:?} raw {raw}: the class at the reported depth is never the one a \
             'separated from both neighbours' reading would predict"
        );
        // And the same call at the arithmetic altitude, with no stratum in it.
        assert_eq!(
            decay
                .class_width(weight, depth)
                .expect("the arithmetic evaluates"),
            ClassWidth::SpansTo(measured_width),
            "{decay:?} raw {raw}: the rule answers the width the profile does"
        );
    }
}

#[test]
fn a_tolerance_of_zero_is_refused_at_both_altitudes_and_a_tolerance_of_one_answers() {
    // A class always contains its own rank, so a tolerance of zero describes no
    // class at all -- the same shape as the smoothing constant of zero that
    // describes no law, and refused on the same terms rather than quietly read
    // as one. Reading it as one answered the narrowest REAL tolerance in its
    // place, which is the deepest fully-separated depth this algebra reports:
    // the most favourable answer there is, returned where nothing was asked.
    //
    // Both altitudes are asserted, each beside its neighbouring valid case, per
    // the repository's rule that a refusal is proved from both sides.
    let decay = DecayRule::ReciprocalRank { k: K };
    let weight = Fixed::from_raw(1_000_000);
    let profile = deep_profile(decay, weight);
    let unweighted = stratum("never/declared");
    let stratum = stratum("deep");

    assert!(
        matches!(
            decay.deepest_rank_within_width(weight, 0),
            Err(FusionError::InvalidWidth { max_width: 0 })
        ),
        "the arithmetic-only altitude refuses a tolerance of zero as a tolerance"
    );
    assert!(
        matches!(
            profile.deepest_rank_within_width(&stratum, 0),
            Err(FusionError::InvalidWidth { max_width: 0 })
        ),
        "and so does the altitude that looks the weight up by stratum"
    );

    let bound = profile
        .monotone_depth(&stratum)
        .expect("weighted")
        .rank()
        .expect("this fixture separates inside a plan's range");
    assert_eq!(
        decay
            .deepest_rank_within_width(weight, 1)
            .expect("a tolerance of one is a tolerance"),
        ToleratedDepth::ReadsTo(bound),
        "the neighbouring valid tolerance still reads to the separating depth"
    );
    assert_eq!(
        profile
            .deepest_rank_within_width(&stratum, 1)
            .expect("a tolerance of one is a tolerance"),
        Some(ToleratedDepth::ReadsTo(bound)),
        "at both altitudes, so the refusal above is about the tolerance and nothing else"
    );

    // A stratum this profile does not weight stays an absence at either
    // tolerance, exactly as it does for `class_width`: "this profile says
    // nothing about that stratum" is true before the tolerance is read.
    assert!(
        matches!(profile.deepest_rank_within_width(&unweighted, 0), Ok(None)),
        "an unweighted stratum is an absence, not a refusal, whatever the tolerance"
    );
    assert!(
        matches!(profile.class_width(&unweighted, 0), Ok(None)),
        "and its sibling answers the same way at the operand it refuses for a weighted stratum"
    );
}

#[test]
fn a_depth_that_outruns_every_plan_is_saturation_and_not_the_ceiling_as_a_number() {
    // Two weights fifty times apart, one tolerance, one answer each. Handing
    // back `u32::MAX` from both says they reach the same depth -- a number a
    // caller can log, plot or divide by -- when the only true statement is that
    // neither has a bound inside any plan's reach. A plan records a per-stratum
    // depth as a 32-bit rank, so there is nothing in that range left to report.
    let decay = DecayRule::WeightedReciprocalRank { k: K };
    let scale = Fixed::ONE.into_raw();
    for weight in [
        Fixed::from_raw(20_000_000 * scale),
        Fixed::from_raw(1_000_000_000 * scale),
    ] {
        assert_eq!(
            decay
                .deepest_rank_within_width(weight, 1)
                .expect("the arithmetic evaluates"),
            ToleratedDepth::ReadsBeyondAnyPlan,
            "a weight that outruns every expressible depth has no bound to report"
        );
        assert_eq!(
            decay
                .deepest_rank_within_width(weight, 1)
                .expect("the arithmetic evaluates")
                .rank(),
            None,
            "and reading it as a number is declined rather than answered with the ceiling"
        );
    }

    // The neighbouring weight below the wall answers with a real depth, so the
    // saturating pair above is about the wall and not about the question.
    let below = Fixed::from_raw(18_000_000 * scale);
    let measured = decay
        .deepest_rank_within_width(below, 1)
        .expect("the arithmetic evaluates");
    assert!(
        matches!(measured, ToleratedDepth::ReadsTo(depth) if depth < u64::from(u32::MAX)),
        "a bound inside a plan's range is a measurement and is reported as one, got {measured:?}"
    );
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
        Some(ClassWidth::SpansTo(1)),
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
        ClassWidth::SpansTo(1),
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
    let folded_width = folded
        .class_width(heavy, deep)
        .expect("the arithmetic evaluates")
        .width()
        .expect("a heavy folded weight's class ends inside the expressible range");
    let truncated_width = decay
        .class_width(heavy, deep)
        .expect("the arithmetic evaluates")
        .width()
        .expect("the truncated rule's class at this rank ends inside the expressible range");
    assert!(
        folded_width < truncated_width,
        "the folded rule keeps resolution the truncated rule has already lost at rank {deep}"
    );
}

#[test]
fn a_class_with_no_end_inside_a_plans_range_is_a_saturation_point_and_not_a_width() {
    // The wall the width curve runs into, told as a wall. A plan records a
    // per-stratum depth as a 32-bit rank, so the search for the class's far end
    // stops there; a class still running at that rank has no counted end, and
    // `u32::MAX` would be the search's own ceiling handed back as a measurement.
    //
    // Raw weights of one and fifty are FIFTY TIMES APART and both saturate:
    // under either rule every contribution has truncated to the same value by
    // rank one, so the class is the whole expressible range in both cases. A
    // bare number reported them as the identical width 4_294_967_295, which a
    // caller could log, plot, or divide by.
    for raw in [1_i128, 50] {
        for decay in [
            DecayRule::ReciprocalRank { k: K },
            DecayRule::WeightedReciprocalRank { k: K },
        ] {
            let weight = Fixed::from_raw(raw);
            assert_eq!(
                decay
                    .class_width(weight, 1)
                    .expect("rank one is a rank and evaluates"),
                ClassWidth::ExceedsAnyPlan,
                "{decay:?} raw {raw}: this class has no end inside a plan's range to count to"
            );
            assert_eq!(
                deep_profile(decay, weight)
                    .class_width(&stratum("deep"), 1)
                    .expect("the arithmetic evaluates"),
                Some(ClassWidth::ExceedsAnyPlan),
                "{decay:?} raw {raw}: the profile reports the same wall the rule does"
            );
            assert!(
                decay
                    .class_width(weight, 1)
                    .expect("rank one is a rank and evaluates")
                    .width()
                    .is_none(),
                "{decay:?} raw {raw}: there is no number to hand a caller here"
            );
            assert!(
                !decay
                    .class_width(weight, 1)
                    .expect("rank one is a rank and evaluates")
                    .fits_within(u64::MAX),
                "{decay:?} raw {raw}: a class with no end is inside no tolerance, however \
                 generous -- the opposite polarity to a separating depth that never collides"
            );
        }
    }

    // The neighbouring weights whose classes DO end inside the range still
    // report counts, so the saturating case is about the measurement and not
    // about light weights in general. A raw weight of 99 is one unit below the
    // 100 whose class is forty ranks; both are far lighter than the 1_000_000
    // the rest of these tests use, and neither saturates.
    for (raw, decay, measured_width) in [
        (99_i128, DecayRule::ReciprocalRank { k: K }, 38_u64),
        (99, DecayRule::WeightedReciprocalRank { k: K }, 39),
        (100, DecayRule::ReciprocalRank { k: K }, 40),
        (100, DecayRule::WeightedReciprocalRank { k: K }, 40),
    ] {
        let weight = Fixed::from_raw(raw);
        assert_eq!(
            decay
                .class_width(weight, 1)
                .expect("rank one is a rank and evaluates"),
            ClassWidth::SpansTo(measured_width),
            "{decay:?} raw {raw}: this class ends inside the range and is counted"
        );
        assert_eq!(
            deep_profile(decay, weight)
                .class_width(&stratum("deep"), 1)
                .expect("the arithmetic evaluates"),
            Some(ClassWidth::SpansTo(measured_width)),
            "{decay:?} raw {raw}: and the profile counts it the same way"
        );
    }
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
        ClassWidth::SpansTo(1)
    );
    assert_eq!(
        DecayRule::ReciprocalRank { k: 1 }
            .class_width(weight, 4)
            .expect("a smoothing constant of one is usable"),
        ClassWidth::SpansTo(1),
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
    collided_fusion_at(TopK::new(8)).await
}

/// The collided fixture under a caller-chosen bound.
///
/// The bound is the only thing that varies between the golden below and the
/// cut-on-a-tie pair after it: same weights, same decay, same two scripts, same
/// four candidates on exactly equal scores. Every candidate here needs both
/// streams read to the end before it can be certified, so the ranks pulled do
/// not move with the bound either — which is what makes the pair one variable.
async fn collided_fusion_at(top_k: TopK) -> FusionResult<Term> {
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
                    Step::Row(RankedRow::new(
                        rank,
                        contribution_under(decay, weight, rank).expect("fits"),
                        Term::new(*name),
                        RowBlock::Undeclared,
                    ))
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
    purrdf_retrieval::fuse::<MockStream, Term>(streams, &profile, top_k)
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

// ---------------------------------------------------------------------------
// 25. The cut that fell on a tie
//
// The golden above fuses four candidates that land on exactly the same score,
// but under a bound larger than the row count — so nothing is excluded and
// `cut_on_a_tie` is false for want of a rival rather than because the last place
// was earned. That leaves the flag's meaningful state unpinned: a trailer that
// hard-coded it to `false` would satisfy every other test in this file.
//
// The pair below closes that with one variable. The same four tied candidates,
// the same law and the same two scripts are fused twice, and only the bound
// moves: once small enough to leave a tied rival outside, where the final place
// is decided by the tie-break and the flag must be `true`, and once large enough
// to exclude nothing, where it must be `false`. The `true` half pins its rows
// and their order too, because a flag announcing a coarse cut is worth nothing
// if the answer under it stopped being deterministic.
// ---------------------------------------------------------------------------

fn collided_cut_golden_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/fusion/collided_cut.golden")
}

#[test]
fn cut_on_a_tie_turns_on_the_bound_and_on_nothing_else() {
    // The bound that excludes a tied rival.
    let cut = block_on(collided_fusion_at(TopK::new(2)));

    let scores: Vec<_> = cut.rows.iter().map(|row| row.score).collect();
    assert_eq!(scores.len(), 2, "the bound holds this answer to two rows");
    assert!(
        scores.windows(2).all(|pair| pair[0] == pair[1]),
        "every candidate in this fixture scores the same, so the emitted rows tie \
         with each other and with the two left outside"
    );
    assert!(
        cut.trailer.cut_on_a_tie,
        "two settled rivals tie with the last emitted row on score, so only the \
         declared tie-break separated them from it"
    );

    // Still deterministic where the score stopped deciding: best stratum rank
    // ascending, then canonical term bytes. `alpha` and `delta` each hold a
    // rank one and `alpha` sorts first; `beta` and `gamma` hold a rank two and
    // are the rivals the cut fell on.
    let emitted: Vec<&str> = cut.rows.iter().map(|row| row.entity.as_str()).collect();
    assert_eq!(
        emitted,
        vec!["alpha", "delta"],
        "the tie-break, not the score, decides which candidates are inside the bound"
    );

    let rendered = render_collided(&cut);
    let expected = std::fs::read_to_string(collided_cut_golden_path())
        .expect("the golden fixture is checked in");
    assert_eq!(
        rendered,
        expected,
        "the cut-on-a-tie fusion drifted from the golden; if the change is intended, update {}",
        collided_cut_golden_path().display()
    );

    // The same stream under a bound that excludes nothing. One variable.
    let whole = block_on(collided_fusion_at(TopK::new(8)));
    assert_eq!(
        whole.rows.len(),
        4,
        "this bound is larger than the fixture, so nothing is excluded"
    );
    assert!(
        !whole.trailer.cut_on_a_tie,
        "no rival was left outside to tie with, so the flag reports none"
    );

    // What the flag does *not* turn on: the evidence of how deep this fusion
    // read is identical on both sides, because every candidate here needs both
    // streams read to the end before it can be certified at all.
    assert_eq!(
        cut.trailer.resolution, whole.trailer.resolution,
        "the two halves differ in their bound and in nothing else"
    );
}

// ---------------------------------------------------------------------------
// 26. The refusals `fuse` makes before a row is pulled, and the profile bounds
//     it makes before a stream exists, each executed beside the valid case next
//     door to it.
//
// A refusal is a claim, and the claim is not "this input is strange" but "this
// exact input is wrong and the one beside it is right". Every test below runs
// both halves and changes exactly one thing between them, because a refusal
// that also swallowed its neighbour looks identical from inside the error.
// ---------------------------------------------------------------------------

/// **Two streams may not share a stratum; two streams under distinct strata are
/// the ordinary case.**
///
/// `fuse` refuses a repeated stratum tag before it pulls a row, because the
/// profile weights a stratum once and two streams under one tag would let a
/// single candidate collect that weight twice. The refusal is about the *tag*,
/// not about the rows: the neighbour below is the same two producers emitting
/// the same candidate at the same rank, differing only in the stratum the second
/// one is tagged with — and it must fuse into exactly the cross-stratum sum the
/// whole engine exists to compute.
#[test]
fn two_streams_tagged_with_one_stratum_are_refused_while_two_distinct_strata_fuse() {
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    let producers = || {
        (
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
        )
    };

    // The violation: both streams claim the stratum `text`.
    let (first, second) = producers();
    let repeated = vec![(stratum("text"), first), (stratum("text"), second)];
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        repeated, &profile, TOP_K,
    ));
    assert!(
        matches!(
            &result,
            Err(FusionError::DuplicateStratum { stratum: named })
                if *named == stratum("text").as_str()
        ),
        "expected DuplicateStratum naming `text`, got {result:?}"
    );

    // THE NEIGHBOURING CASE: the same rows, with the second stream tagged with
    // the stratum it actually came from. One candidate, two contributions.
    let (first, second) = producers();
    let distinct = vec![(stratum("text"), first), (stratum("vector"), second)];
    let fused = block_on(run_fuse(distinct, &profile));
    assert_eq!(fused.rows.len(), 1, "one candidate, named by both strata");
    assert_eq!(fused.rows[0].entity, Term::new("a"));
    assert_eq!(
        fused.rows[0].contributions.len(),
        2,
        "distinct strata contribute once each rather than being refused"
    );
    let per_stratum = contribution(Fixed::ONE, 1, K).expect("fits");
    assert_eq!(
        fused.rows[0].score,
        per_stratum.checked_add(per_stratum).expect("fits"),
        "and the score is the checked sum of both"
    );
}

/// **A stratum the profile never weights is refused; the same stream under a
/// profile that weights it fuses.**
///
/// The refusal is a statement about the *profile*, not about the producer: a
/// weight is "how much does this count", and a profile that never named the
/// stratum has stated no answer, so fusing the stream would mean inventing one.
/// The two halves below therefore hold the streams fixed and vary only the
/// profile, which is the single thing the refusal is about.
#[test]
fn a_stream_whose_stratum_the_profile_never_weights_is_refused_while_the_weighted_neighbour_fuses()
{
    let streams = || {
        vec![
            (
                stratum("text"),
                MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
            ),
            (
                stratum("vector"),
                MockStream::new(vec![row(1, Fixed::ONE, K, "b")], exhausted(1)),
            ),
        ]
    };

    // The violation: the profile weights `text` and says nothing about
    // `vector`, which is the stratum the second stream is tagged with.
    let partial = profile(&[("text", Fixed::ONE)], K);
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        streams(),
        &partial,
        TOP_K,
    ));
    assert!(
        matches!(
            &result,
            Err(FusionError::UnknownStratum { stratum: named })
                if *named == stratum("vector").as_str()
        ),
        "expected UnknownStratum naming `vector`, got {result:?}"
    );

    // THE NEIGHBOURING CASE: the identical streams under a profile that does
    // declare the second stratum. Both producers reach the answer.
    let complete = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    let fused = block_on(run_fuse(streams(), &complete));
    assert_eq!(
        fused
            .rows
            .iter()
            .map(|fused_row| fused_row.entity.clone())
            .collect::<Vec<_>>(),
        vec![Term::new("a"), Term::new("b")],
        "a weighted stratum contributes rather than being refused"
    );
    assert_eq!(
        fused.trailer.statuses.len(),
        2,
        "and both producers report their own status"
    );
}

/// **A failure is a protocol violation only once rows have gone out.**
///
/// `ErrorAfterRows` is the producer's own vocabulary for "I broke mid-stream",
/// and the refusal it triggers is narrow on purpose. A producer that fails
/// *before* emitting anything has an honest terminal receipt to return and no
/// rows to contradict it, so it is not refused at all: it reports
/// `ExecutionFailed` and the trailer carries the reason verbatim. And the same
/// stream that broke, with the break removed, fuses every row it emitted. Both
/// neighbours run here, because a refusal that reached either of them would
/// convert an ordinary empty answer — or an ordinary complete one — into a
/// failed request.
#[test]
fn a_failure_after_rows_is_refused_while_a_producer_that_fails_before_any_row_is_reported() {
    let profile = profile(&[("text", Fixed::ONE)], K);

    // The violation: a row went out, and then the producer broke. A clean
    // receipt can no longer describe what this stream did.
    let broke_mid_stream = vec![(
        stratum("text"),
        MockStream::new(
            vec![
                row(1, Fixed::ONE, K, "a"),
                Step::Fail(ProtocolError::ErrorAfterRows { rows_before: 1 }),
            ],
            exhausted(0),
        ),
    )];
    let result = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        broke_mid_stream,
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

    // THE FIRST NEIGHBOUR: the same single row, with the break removed. One
    // variable, and the row must reach the answer.
    let intact = vec![(
        stratum("text"),
        MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
    )];
    let fused = block_on(run_fuse(intact, &profile));
    assert_eq!(
        fused
            .rows
            .iter()
            .map(|fused_row| fused_row.entity.clone())
            .collect::<Vec<_>>(),
        vec![Term::new("a")],
        "a stream that did not break is read to its end"
    );

    // THE SECOND NEIGHBOUR: the same failure, before any row went out. There is
    // nothing for it to contradict, so it is a receipt rather than a violation
    // and the answer is an ordinary empty one.
    let failed_before_rows = vec![(
        stratum("text"),
        MockStream::new(
            Vec::new(),
            ProducerReceipt::ExecutionFailed {
                reason: "the index was unavailable".to_owned(),
            },
        ),
    )];
    let fused = block_on(run_fuse(failed_before_rows, &profile));
    assert_eq!(
        fused.rows,
        Vec::new(),
        "a producer that could not run emits no rows"
    );
    assert_eq!(
        fused.trailer.statuses.get(&stratum("text")),
        Some(&ProducerStatus::ExecutionFailed {
            reason: "the index was unavailable".to_owned(),
        }),
        "and the reason it could not run is carried verbatim, not refused"
    );
}

/// **A stream that will not describe its own end is refused; the same rows with
/// a declared end are not.**
///
/// `NeverEndingSource` is the one refusal this crate raises about a producer's
/// *silence* rather than about a row, and it has an in-crate author:
/// [`RankedStreamImpl`] returns it when a receipt is asked for before the rows
/// are drained, because a completeness claim from a partially read stream is
/// exactly the falsifiable status the protocol forbids. Both halves are executed
/// on one stream here — the same producer, asked the same question, before and
/// after it has actually finished — so the refusal cannot be mistaken for a
/// property of the stream rather than of the moment it was asked.
#[test]
fn a_stream_that_will_not_end_is_refused_while_the_same_rows_with_a_declared_end_fuse() {
    let profile = profile(&[("text", Fixed::ONE)], K);

    // The violation, at the fusion boundary: the producer emits a row and then
    // reports that it has no end to declare.
    let never_ending = vec![(
        stratum("text"),
        MockStream::new(
            vec![
                row(1, Fixed::ONE, K, "a"),
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

    // THE NEIGHBOURING CASE: the same rows, from a producer that does declare
    // its end. It fuses, and the trailer reports the count it declared.
    let terminating = vec![(
        stratum("text"),
        MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
    )];
    let fused = block_on(run_fuse(terminating, &profile));
    assert_eq!(fused.rows.len(), 1);
    assert_eq!(
        fused.trailer.statuses.get(&stratum("text")),
        Some(&ProducerStatus::Exhausted { rows_emitted: 1 })
    );

    // And the same pair on the in-crate producer that actually mints the
    // refusal. Asked too early it refuses; asked after its rows ran out — one
    // more pull, nothing else changed — it answers with the count it emitted.
    let mut stream = RankedStreamImpl::new(
        vec![
            (1, Term::new("a"), RowBlock::Undeclared),
            (2, Term::new("b"), RowBlock::Undeclared),
        ],
        StreamEnding::Exhausted,
    );
    assert!(
        block_on(stream.next())
            .expect("the first row pulls")
            .is_some()
    );
    assert!(
        matches!(
            block_on(stream.receipt()),
            Err(ProtocolError::NeverEndingSource)
        ),
        "a partially read stream may not describe its own completeness"
    );
    assert!(
        block_on(stream.next())
            .expect("the second row pulls")
            .is_some()
    );
    assert!(
        block_on(stream.next()).expect("the stream ends").is_none(),
        "the rows ran out"
    );
    assert_eq!(
        block_on(stream.receipt()).expect("a drained stream has a receipt"),
        ProducerReceipt::Exhausted { rows_emitted: 2 },
        "the very same stream, asked once its rows had run out, answers"
    );
}

/// **The two profile bounds are refused at zero and admitted at one.**
///
/// `K >= 1` and "at least one stratum weight" are both stated as minimums, and a
/// minimum is the refusal most likely to be set one too high: the whole
/// difference between a law and an over-refusal is whether the boundary value
/// itself is admitted. So each is executed at the value it rejects and at the
/// smallest value it must accept, and the admitted profile is not merely
/// constructed — it computes a contribution, and it fuses a stream.
#[test]
fn the_smallest_k_and_the_smallest_weight_set_a_profile_admits_are_not_refused() {
    let one_weight = || BTreeMap::from([(stratum("text"), Fixed::ONE)]);

    // The violation: a smoothing constant of zero. Rank one would then carry the
    // whole weight and the reciprocal would be undefined at rank zero.
    for decay in [
        DecayRule::ReciprocalRank { k: 0 },
        DecayRule::WeightedReciprocalRank { k: 0 },
    ] {
        let result = FusionProfile::with_decay(one_weight(), decay);
        assert!(
            matches!(result, Err(FusionError::InvalidK { k: 0 })),
            "expected InvalidK for {decay:?}, got {result:?}"
        );
    }

    // THE NEIGHBOURING CASE: `K = 1`, the smallest constant the law admits,
    // under both rules. Each must construct and each must compute.
    for decay in [
        DecayRule::ReciprocalRank { k: 1 },
        DecayRule::WeightedReciprocalRank { k: 1 },
    ] {
        let smallest = FusionProfile::with_decay(one_weight(), decay)
            .expect("K = 1 is the smallest constant the law admits");
        assert_eq!(smallest.k_parameter(), 1);
        assert_eq!(smallest.decay(), decay);
        let value = contribution_under(smallest.decay(), Fixed::ONE, 1)
            .expect("the admitted constant computes a contribution at rank one");
        assert!(
            value > Fixed::ZERO,
            "a profile at the boundary produces a real contribution, got {value:?}"
        );
    }

    // The violation: no stratum weights at all. Checked under a valid `K`, so
    // the dimension under test is the only one that can fail.
    let result = FusionProfile::with_decay(BTreeMap::new(), DecayRule::ReciprocalRank { k: K });
    assert!(
        matches!(result, Err(FusionError::EmptyWeights)),
        "expected EmptyWeights, got {result:?}"
    );

    // THE NEIGHBOURING CASE: exactly one weight, which is the smallest set that
    // is not empty. It constructs, it derives a contribution maximum of one, and
    // it fuses a stream under the stratum it names.
    let single = FusionProfile::with_decay(one_weight(), DecayRule::ReciprocalRank { k: K })
        .expect("one stratum weight is a profile");
    assert_eq!(
        single.max_contributions(),
        1,
        "one stratum, one contribution maximum"
    );
    assert_eq!(single.weight(&stratum("text")), Some(Fixed::ONE));
    let fused = block_on(run_fuse(
        vec![(
            stratum("text"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
        )],
        &single,
    ));
    assert_eq!(fused.rows.len(), 1, "a one-stratum profile fuses");
    assert_eq!(
        fused.rows[0].score,
        contribution(Fixed::ONE, 1, K).expect("fits")
    );
}

// ---------------------------------------------------------------------------
// A read ending the producer authored, and the evidence it answered from
//
// Two different kinds of fact meet in the trailer here, and every test below
// exists to keep them apart. `ProducerStatus` says who stopped the read; an
// attestation says what the index behind the rows was. The first is written
// when a stream ends and can be written *over* by a bounded stop; the second is
// pinned before the first row is pulled and nothing a caller does can move it.
// ---------------------------------------------------------------------------

/// A producer that names a generation and declares nothing about wholeness.
fn attests(generation: &str) -> PfAttestation {
    PfAttestation {
        generation: IndexGeneration::declared(generation),
        service: ServiceLevel::Undeclared,
    }
}

/// A producer that names a generation and declares that index was **not** whole.
fn attests_short(generation: &str, reason: &str) -> PfAttestation {
    PfAttestation {
        generation: IndexGeneration::declared(generation),
        service: ServiceLevel::Incomplete {
            reason: reason.to_owned(),
        },
    }
}

// T3.1. A producer-authored ending for a read the planned depth stopped.
//
// The rows are returned and the ending is carried verbatim: fusion has no way
// of telling a stream that stopped at its depth from one that ran out — both
// simply stop yielding — so the producer is the only party that can say which
// happened, and this is the channel it says it on.
#[test]
fn a_depth_ending_is_carried_and_its_rank_is_still_measured_against_the_rows() {
    let profile = profile(&[("text", Fixed::ONE)], K);
    let scripted = || vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "b")];

    let stopped = block_on(run_fuse(
        vec![(
            stratum("text"),
            MockStream::new(scripted(), ProducerReceipt::DepthReached { rank: 2 }),
        )],
        &profile,
    ));
    assert_eq!(
        stopped.rows.len(),
        2,
        "a depth-stopped stream still contributed every row it emitted"
    );
    assert_eq!(
        stopped.trailer.statuses.get(&stratum("text")),
        Some(&ProducerStatus::DepthReached { rank: 2 }),
        "the producer's own ending reaches the trailer in rank space, unreworded"
    );
    assert_ne!(
        stopped.trailer.statuses.get(&stratum("text")),
        Some(&ProducerStatus::Exhausted { rows_emitted: 2 }),
        "and it is emphatically not a completeness claim: rows existed below rank 2"
    );

    // The refusal: a depth ending may stop the read, it may not miscount it.
    // Two rows were pulled and the receipt names rank one, so the producer is
    // claiming it never emitted the row fusion already holds.
    let forged = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        vec![(
            stratum("text"),
            MockStream::new(scripted(), ProducerReceipt::DepthReached { rank: 1 }),
        )],
        &profile,
        TOP_K,
    ));
    assert!(
        matches!(
            &forged,
            Err(FusionError::Protocol(error))
                if matches!(**error, ProtocolError::ForgedReceipt { declared: 1, actual: 2 })
        ),
        "expected ForgedReceipt {{ declared: 1, actual: 2 }}, got {forged:?}"
    );

    // THE NEIGHBOURING CASE that must still succeed, because the refusal above
    // is about the count and not about stopping: the same two rows with the
    // honest rank fuse, and so does a one-row stream that stopped at rank one.
    let honest = block_on(run_fuse(
        vec![(
            stratum("text"),
            MockStream::new(
                vec![row(1, Fixed::ONE, K, "a")],
                ProducerReceipt::DepthReached { rank: 1 },
            ),
        )],
        &profile,
    ));
    assert_eq!(honest.rows.len(), 1);
    assert_eq!(
        honest.trailer.statuses.get(&stratum("text")),
        Some(&ProducerStatus::DepthReached { rank: 1 }),
        "a depth of one is a legitimate depth, not a forgery"
    );
}

// T3.2. The trailer names exactly what the handed streams attested.
#[test]
fn the_trailer_carries_every_handed_streams_attestation_and_invents_none() {
    let profile = profile(
        &[
            ("text", Fixed::ONE),
            ("vector", Fixed::ONE),
            ("geo", Fixed::ONE),
        ],
        K,
    );
    let streams = vec![
        (
            stratum("text"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)).attesting(attests("a")),
        ),
        (
            stratum("vector"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "b")], exhausted(1))
                .attesting(attests_short("b", "shard 3 of 4 failed to load")),
        ),
        // The third says nothing at all, which is what a stream that descends
        // from no index honestly reports.
        (
            stratum("geo"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "c")], exhausted(1)),
        ),
    ];

    let result = block_on(run_fuse(streams, &profile));
    assert_eq!(
        result.trailer.attestations,
        BTreeMap::from([
            (stratum("text"), attests("a")),
            (
                stratum("vector"),
                attests_short("b", "shard 3 of 4 failed to load")
            ),
            (stratum("geo"), PfAttestation::UNDECLARED),
        ]),
        "every handed stream is named, with its own attestation and nobody else's"
    );

    // A producer that never became a stream gets a status and no attestation.
    // Its absence is the honest answer: no index of its was ever opened, so
    // there is nothing it attested — and `Undeclared` would be the wrong
    // answer, because that means a producer was asked and stayed silent.
    let completed = result
        .trailer
        .completed_with([(stratum("absent"), ProducerStatus::TermsRejected)]);
    assert!(
        completed.statuses.contains_key(&stratum("absent")),
        "a producer that never became a stream still gets a status"
    );
    assert!(
        !completed.attestations.contains_key(&stratum("absent")),
        "but no attestation, fabricated or otherwise"
    );
    assert_eq!(
        completed.attestations.len(),
        3,
        "so the attestation keys stay a subset of the statuses'"
    );
}

// T3.3. The load-bearing consequence of reading the attestation at `open`.
#[test]
fn a_bounded_stop_and_an_incomplete_index_both_survive_in_one_trailer() {
    // One stratum, three rows, and a bound that stops the read at the first.
    // The stream is therefore closed by fusion rather than by its own receipt —
    // which is exactly the path that would have destroyed an incompleteness
    // held as a terminal fact, because a stopped stream never returns a receipt
    // at all.
    let profile = profile(&[("text", Fixed::ONE)], K);
    let streams = vec![(
        stratum("text"),
        MockStream::new(
            vec![
                row(1, Fixed::ONE, K, "a"),
                row(2, Fixed::ONE, K, "b"),
                row(3, Fixed::ONE, K, "c"),
            ],
            exhausted(3),
        )
        .attesting(attests_short(
            "2026-09-18T00:00:00Z",
            "replica is 2 segments behind",
        )),
    )];

    let bounded = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        streams,
        &profile,
        TopK::new(1),
    ))
    .expect("a bounded fusion succeeds");

    assert_eq!(bounded.rows.len(), 1, "the bound is the bound");
    assert!(
        matches!(
            bounded.trailer.statuses.get(&stratum("text")),
            Some(&ProducerStatus::CeilingReached { .. })
        ),
        "the bound stopped this stream, so its status is fusion's own bounded stop, \
         got {:?}",
        bounded.trailer.statuses.get(&stratum("text"))
    );
    assert_eq!(
        bounded
            .trailer
            .attestations
            .get(&stratum("text"))
            .map(|attestation| &attestation.service),
        Some(&ServiceLevel::Incomplete {
            reason: "replica is 2 segments behind".to_owned()
        }),
        "and the short index it served from survives the stop that overwrote nothing \
         else about it"
    );
}

// T3.4. An incomplete stratum makes every score an ESTIMATE whose error runs in
// both directions — and the rows are still returned, because a short index
// produced real rows in a real order.
#[test]
fn an_incomplete_stratum_makes_the_scores_estimates_without_refusing_the_rows() {
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    let streams = |vector: PfAttestation| {
        vec![
            (
                stratum("text"),
                MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
            ),
            (
                stratum("vector"),
                MockStream::new(vec![row(1, Fixed::ONE, K, "b")], exhausted(1)).attesting(vector),
            ),
        ]
    };

    // Nothing declared short: the sums are exact over every contribution that
    // was due. Note what this does *not* say — most producers attest nothing,
    // and `Exact` is the narrow true claim that none of them declared itself
    // short, never a certificate that the indexes were whole.
    let exact = block_on(run_fuse(streams(PfAttestation::UNDECLARED), &profile));
    assert_eq!(
        exact.trailer.exactness,
        ScoreExactness::Exact,
        "no stratum declared itself short, so nothing makes these scores estimates"
    );

    // One stratum short: every score in the answer is an estimate, and the
    // trailer names the stratum to rebuild on BOTH sides rather than raising an
    // anonymous flag. Both sides, because scoring by rank means a missed row is
    // withheld from its own candidate and promotes every candidate behind it.
    let bounded = block_on(run_fuse(
        streams(attests_short("b", "segment rebuilding")),
        &profile,
    ));
    assert_eq!(
        bounded.trailer.exactness,
        ScoreExactness::Estimated {
            deficit: BTreeSet::from([stratum("vector")]),
            inflation: BTreeSet::from([stratum("vector")]),
            unbounded: BTreeSet::new(),
        },
        "exactly the stratum that declared itself short, and no other, named on \
         both sides: a missing shard withholds its own rows and promotes every \
         row that was behind them"
    );
    assert_eq!(
        bounded.rows.len(),
        2,
        "and the rows are returned, not refused: a short index still produced \
         real rows in a real order"
    );
    assert_eq!(
        bounded
            .rows
            .iter()
            .map(|fused| fused.score)
            .collect::<Vec<_>>(),
        exact
            .rows
            .iter()
            .map(|fused| fused.score)
            .collect::<Vec<_>>(),
        "the arithmetic did not change; what changed is what may be concluded from it"
    );

    // The reason a caller can act on it: the verbatim reason is readable under
    // the same key the exactness named.
    assert_eq!(
        bounded
            .trailer
            .attestations
            .get(&stratum("vector"))
            .map(|attestation| &attestation.service),
        Some(&ServiceLevel::Incomplete {
            reason: "segment rebuilding".to_owned()
        })
    );
}

// T3.5. The third identity: equal evidence digests equally, and a rebuilt index
// does not.
#[test]
fn the_evidence_identity_moves_exactly_when_the_evidence_does() {
    // The injectivity fixtures at the foot of this test need profiles of their
    // own, and they are built here, before the binding below shadows the
    // helper that builds them.
    let split_profile = profile(&[("ab", Fixed::ONE)], K);
    let joined_profile = profile(&[("a", Fixed::ONE)], K);
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    let streams = |text: PfAttestation| {
        vec![
            (
                stratum("text"),
                MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)).attesting(text),
            ),
            (
                stratum("vector"),
                MockStream::new(vec![row(1, Fixed::ONE, K, "b")], exhausted(1))
                    .attesting(attests_short("v-1", "one shard offline")),
            ),
        ]
    };

    let first = block_on(run_fuse(streams(attests("t-1")), &profile));
    let repeat = block_on(run_fuse(streams(attests("t-1")), &profile));
    let rebuilt = block_on(run_fuse(streams(attests("t-2")), &profile));

    assert_eq!(
        first.trailer.evidence_id, repeat.trailer.evidence_id,
        "identically-attesting streams are identical evidence"
    );
    assert_ne!(
        first.trailer.evidence_id, rebuilt.trailer.evidence_id,
        "one rebuilt generation is different evidence, and nothing else in the \
         answer would have said so"
    );
    // The gap the third identity closes, stated as the assertion it is: the
    // plan and the law are byte-identical across a rebuild.
    assert_eq!(first.trailer.plan_id, rebuilt.trailer.plan_id);
    assert_eq!(first.trailer.profile_id, rebuilt.trailer.profile_id);

    // The identity is re-derivable from bytes a holder of the answer has, which
    // is what makes it auditable rather than merely present.
    assert_eq!(
        EvidenceId::from_canonical(&first.trailer.evidence_canonical_bytes()),
        first.trailer.evidence_id,
        "the id in the trailer is the digest of the trailer's own canonical bytes"
    );
    assert_ne!(
        first.trailer.evidence_canonical_bytes(),
        rebuilt.trailer.evidence_canonical_bytes(),
        "and the bytes themselves differ, so the difference is not a digest artefact"
    );

    // Injectivity, which is what the length framing buys: a stratum suffix and
    // a generation that run together the same way must not encode alike.
    let split = block_on(run_fuse(
        vec![(
            stratum("ab"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)).attesting(attests("c")),
        )],
        &split_profile,
    ));
    let joined = block_on(run_fuse(
        vec![(
            stratum("a"),
            MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1))
                .attesting(attests("bc")),
        )],
        &joined_profile,
    ));
    assert_ne!(
        split.trailer.evidence_canonical_bytes(),
        joined.trailer.evidence_canonical_bytes(),
        "framed fields cannot run together into one another's bytes"
    );
    assert_ne!(split.trailer.evidence_id, joined.trailer.evidence_id);
}

// T3.6. A trailer is non-consuming and re-callable, and the two kinds of fact
// behave differently across two reads — which is the whole point of pinning one
// of them at `open`.
#[test]
fn a_re_read_trailer_moves_the_read_and_never_the_evidence() {
    let profile = profile(&[("text", Fixed::ONE)], K);
    let streams = vec![(
        stratum("text"),
        MockStream::new(
            vec![
                row(1, Fixed::ONE, K, "a"),
                row(2, Fixed::ONE, K, "b"),
                row(3, Fixed::ONE, K, "c"),
            ],
            exhausted(3),
        )
        .attesting(attests_short("g-7", "shard 1 offline")),
    )];
    let mut fusion = FusionStream::new(streams, profile);

    block_on(fusion.next())
        .expect("the first row certifies")
        .expect("a row");
    let early = block_on(fusion.trailer()).expect("a trailer mid-stream");
    assert!(
        matches!(
            early.statuses.get(&stratum("text")),
            Some(&ProducerStatus::CeilingReached { .. })
        ),
        "mid-stream the producer has not ended, so it is reported at the bound \
         reading had reached, got {:?}",
        early.statuses.get(&stratum("text"))
    );

    block_on(fusion.next()).expect("the second row certifies");
    block_on(fusion.next()).expect("the third row certifies");
    let late = block_on(fusion.trailer()).expect("a trailer after the stream ended");

    // What moved: how the read ended, because the read moved.
    assert_eq!(
        late.statuses.get(&stratum("text")),
        Some(&ProducerStatus::Exhausted { rows_emitted: 3 }),
        "the stream reached its own receipt between the two reads"
    );
    assert_ne!(
        early.statuses, late.statuses,
        "so the statuses are as of each read, exactly as the resolution counters are"
    );

    // What did not move, and must not: an index generation is pinned when the
    // index is opened, so how deep a caller chose to read cannot change what
    // answered, whether it was whole, or the identity built from those two.
    assert_eq!(
        early.attestations, late.attestations,
        "the attestations were read before the first row and never re-asked"
    );
    assert_eq!(
        early.evidence_id, late.evidence_id,
        "so the evidence identity is the same identity at any read depth"
    );
    assert_eq!(
        early.exactness, late.exactness,
        "and so is the exactness derived from it"
    );
    assert_eq!(
        late.exactness,
        ScoreExactness::Estimated {
            deficit: BTreeSet::from([stratum("text")]),
            inflation: BTreeSet::from([stratum("text")]),
            unbounded: BTreeSet::new(),
        },
        "which is `Estimated` throughout, because the index was short throughout \
         — how deep a caller read changes which streams are still open, never \
         whether an index was whole"
    );
}

// ---------------------------------------------------------------------------
// T6. Declared candidate domains: the reading is bounded where the producers
//     said it could be, and the answer never moves.
//
// The defect these fixtures pin is a drain. Fusion certifies a candidate only
// once every stream that could still name it has named it, and with no
// declaration "could still name it" is true of every open stream — so over
// strata whose candidate sets do not overlap, nothing ever certifies while
// another stream is open, and a top-five answer over two thousand-row strata
// reads two thousand rows. The trailer then compounds it: a stream that was
// DRAINED reports `Exhausted`, the one ending that names no stopper, which the
// caller's bound never asked anyone to earn.
//
// A producer's declaration is what removes it. `CandidateDomains` names the
// blocks of the candidate universe a producer may draw from, fusion skips
// exactly the streams that provably cannot name the candidate in hand, and the
// scores are the scores they always were. Every test below is in one of three
// families, and the third is the one that makes the other two worth having:
//
//   * the BOUND — how much was read (T6.1, T6.2, T6.4, T6.8 in
//     `fusion_frontier_alloc.rs`);
//   * the REFUSAL — a declaration held to, with its valid neighbours (T6.5);
//   * the SOUNDNESS — the answer under a declaration is the answer without one
//     (T6.3), because a bound that changes the rows is not a bound, it is a bug.
// ---------------------------------------------------------------------------

/// Two caller-named blocks of a candidate universe. Nothing here mints them:
/// they are `example.org` IRIs a host chose, exactly as strata are.
const DOMAIN_DOCS: &str = "http://example.org/domain/documents";
const DOMAIN_PEOPLE: &str = "http://example.org/domain/people";

fn domain(tag: &str) -> DomainTag {
    DomainTag::parse(tag).expect("fixture domain tags are valid IRIs")
}

fn within(tags: &[&str]) -> CandidateDomains {
    CandidateDomains::within(tags.iter().map(|tag| domain(tag)))
}

/// One stratum of a domain fixture: who it is, what it promises, what it emits.
struct StratumSpec {
    /// The stratum's short name, spelled into an IRI by [`stratum`].
    name: &'static str,
    /// The blocks this producer declares it may name. Its items are all drawn
    /// from them, so the declaration is true — which is the precondition every
    /// soundness claim below rests on.
    tags: Vec<&'static str>,
    /// This stratum's weight in the fixture profile.
    weight: Fixed,
    /// The items it emits, in rank order, ranks 1..=len.
    items: Vec<String>,
}

/// Which declaration a run is made under.
///
/// The three are a lattice, from the widest promise to the narrowest, and every
/// one of them is TRUE of the same streams. That is what makes comparing them a
/// measurement of the declaration rather than of three different fixtures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Declared {
    /// Every stream may name anything: the promise every producer made before
    /// the term existed, and the behaviour this engine had then.
    Nothing,
    /// Every stream names the union of every block in play. True, and useless:
    /// each stream's declaration meets every other's, so nothing may be
    /// skipped. It is the middle of the lattice and exists to separate "a
    /// declaration was read" from "a declaration was USED".
    TheUnion,
    /// Each stream's own blocks, as the spec states them.
    ItsOwnBlocks,
}

fn spec_profile(spec: &[StratumSpec]) -> FusionProfile {
    let weights: Vec<(&str, Fixed)> = spec
        .iter()
        .map(|stratum| (stratum.name, stratum.weight))
        .collect();
    profile(&weights, K)
}

/// Every block any stratum in `spec` declares, in canonical order.
fn spec_union(spec: &[StratumSpec]) -> Vec<&'static str> {
    let mut union: BTreeSet<&'static str> = BTreeSet::new();
    for stratum in spec {
        union.extend(stratum.tags.iter().copied());
    }
    union.into_iter().collect()
}

/// The block a fixture item really lies in, read off the item itself.
///
/// The fixtures mint `doc/…` and `person/…` items, so the block is a property of
/// the item rather than of the stream that emitted it — which is exactly what the
/// partition axiom says a block is, and what lets the cross-cutting stratum name
/// items from both blocks truthfully.
fn block_of(item: &str) -> &'static str {
    if item.contains("/doc/") {
        DOMAIN_DOCS
    } else {
        DOMAIN_PEOPLE
    }
}

/// The fixture's streams, each declaring what `declared` says it declares.
fn spec_streams(spec: &[StratumSpec], declared: Declared) -> Vec<(Iri, MockStream)> {
    let union = spec_union(spec);
    spec.iter()
        .map(|entry| {
            let steps: Vec<Step> = entry
                .items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    let rank = u64::try_from(index + 1).expect("fixture ranks fit");
                    match declared {
                        // An unrestricted stream owes no per-row block and names
                        // none: this is the fixture as it was before blocks
                        // existed, which is what the differential compares
                        // against.
                        Declared::Nothing => row(rank, entry.weight, K, item),
                        // A restricted stream backs its declaration row by row,
                        // with the block the item is really in.
                        Declared::TheUnion | Declared::ItsOwnBlocks => {
                            row_in(rank, entry.weight, K, item, block_of(item))
                        }
                    }
                })
                .collect();
            let emitted = u64::try_from(steps.len()).expect("fixture row counts fit");
            let domains = match declared {
                Declared::Nothing => CandidateDomains::Unrestricted,
                Declared::TheUnion => within(&union),
                Declared::ItsOwnBlocks => within(&entry.tags),
            };
            (
                stratum(entry.name),
                MockStream::new(steps, exhausted(emitted)).declaring(StreamContract::new(
                    DuplicatePolicy::Unique,
                    RankFidelity::EXACT,
                    domains,
                    ExclusionBasis::Unavailable,
                )),
            )
        })
        .collect()
}

/// The eager oracle: every item's total, computed from the script rather than
/// from the engine, ordered by the declared total tie-break.
fn spec_oracle(spec: &[StratumSpec]) -> Vec<(String, Fixed, u64)> {
    let mut totals: BTreeMap<String, (Fixed, u64)> = BTreeMap::new();
    for entry in spec {
        for (index, item) in entry.items.iter().enumerate() {
            let rank = u64::try_from(index + 1).expect("fixture ranks fit");
            let value = contribution(entry.weight, rank, K).expect("fixture contributions fit");
            let seen = totals
                .entry(item.clone())
                .or_insert((Fixed::ZERO, u64::MAX));
            seen.0 = seen.0.checked_add(value).expect("fixture sums fit");
            seen.1 = seen.1.min(rank);
        }
    }
    let mut ordered: Vec<(String, Fixed, u64)> = totals
        .into_iter()
        .map(|(item, (score, rank))| (item, score, rank))
        .collect();
    ordered.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then(left.2.cmp(&right.2))
            .then(left.0.cmp(&right.0))
    });
    ordered
}

fn fuse_spec(spec: &[StratumSpec], declared: Declared, top_k: TopK) -> FusionResult<Term> {
    block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        spec_streams(spec, declared),
        &spec_profile(spec),
        top_k,
    ))
    .expect("the fixture streams obey the protocol")
}

fn ranks_pulled(result: &FusionResult<Term>, name: &str) -> u64 {
    result
        .trailer
        .resolution
        .get(&stratum(name))
        .expect("the fixture profile weights every fixture stratum")
        .ranks_pulled
}

/// `count` items in one block, named so that the block's own order is the
/// lexical order the tie-break falls back on.
fn block_items(prefix: &str, count: u64) -> Vec<String> {
    (1..=count)
        .map(|index| format!("http://example.org/{prefix}/{index:06}"))
        .collect()
}

/// **Case A**, the reported reproduction: two thousand-row strata whose
/// candidate sets are disjoint, each declaring its own block.
fn case_a() -> Vec<StratumSpec> {
    vec![
        StratumSpec {
            name: "docs",
            tags: vec![DOMAIN_DOCS],
            weight: Fixed::ONE,
            items: block_items("doc", 1_000),
        },
        StratumSpec {
            name: "people",
            tags: vec![DOMAIN_PEOPLE],
            weight: Fixed::ONE,
            items: block_items("person", 1_000),
        },
    ]
}

/// **Case B**: the same disjoint shape, with both streams shorter than the
/// caller's bound. Nothing here can end at a ceiling, because the rows run out
/// first — the valid neighbour of case A's bounded stop.
fn case_b() -> Vec<StratumSpec> {
    vec![
        StratumSpec {
            name: "docs",
            tags: vec![DOMAIN_DOCS],
            weight: Fixed::ONE,
            items: block_items("doc", 2),
        },
        StratumSpec {
            name: "people",
            tags: vec![DOMAIN_PEOPLE],
            weight: Fixed::ONE,
            items: block_items("person", 2),
        },
    ]
}

/// **Case C**: one short stratum beside one long one. The short one exhausts
/// and says so; the long one is stopped at a bound and says that.
fn case_c() -> Vec<StratumSpec> {
    vec![
        StratumSpec {
            name: "docs",
            tags: vec![DOMAIN_DOCS],
            weight: Fixed::ONE,
            items: block_items("doc", 3),
        },
        StratumSpec {
            name: "people",
            tags: vec![DOMAIN_PEOPLE],
            weight: Fixed::ONE,
            items: block_items("person", 1_000),
        },
    ]
}

/// **The cross-cutting configuration**: two disjoint strata and a third that
/// names candidates from BOTH blocks — a quality prior, a popularity rank, any
/// signal that applies to the whole corpus.
///
/// It is why a declaration is a SET of blocks per producer rather than a
/// partition of the strata. Forced to name one block, such a producer would
/// have to lie in one direction or the other; allowed to name both, it stays
/// truthful, still meets both disjoint strata, and the bound survives.
fn case_cross_cutting() -> Vec<StratumSpec> {
    let docs = block_items("doc", 1_000);
    let people = block_items("person", 1_000);
    // The prior ranks the same universe, alternating blocks, so it genuinely
    // names candidates both other strata name.
    let mixed: Vec<String> = docs
        .iter()
        .zip(people.iter())
        .flat_map(|(doc, person)| [doc.clone(), person.clone()])
        .take(1_000)
        .collect();
    vec![
        StratumSpec {
            name: "docs",
            tags: vec![DOMAIN_DOCS],
            weight: Fixed::ONE,
            items: docs,
        },
        StratumSpec {
            name: "people",
            tags: vec![DOMAIN_PEOPLE],
            weight: Fixed::ONE,
            items: people,
        },
        StratumSpec {
            name: "prior",
            tags: vec![DOMAIN_DOCS, DOMAIN_PEOPLE],
            weight: Fixed::ONE,
            items: mixed,
        },
    ]
}

// T6.1. The reading is bounded by the caller's `k`, not by the streams' length.
#[test]
fn declared_domains_bound_the_reading_over_disjoint_strata() {
    // The bound each assertion is measured against, spelled as the arithmetic
    // it comes from rather than as "small": `k` rows have to be certified, and
    // certifying the last of them requires the threshold to fall below it,
    // which costs one unmerged head per stream that is still open. A ceiling of
    // `k + streams` is therefore the claim; anything at the streams' length is
    // the drain.
    let bound = TopK::new(5);

    let a = fuse_spec(&case_a(), Declared::ItsOwnBlocks, bound);
    assert_eq!(a.rows.len(), 5, "the bound is what stopped this run");
    assert!(
        ranks_pulled(&a, "docs") <= 7,
        "case A read {} ranks from the documents stratum; the bound is 5 + 2",
        ranks_pulled(&a, "docs")
    );
    assert!(
        ranks_pulled(&a, "people") <= 7,
        "case A read {} ranks from the people stratum; the bound is 5 + 2",
        ranks_pulled(&a, "people")
    );

    // And the counter-measurement, so the ceiling above is known to be doing
    // work: without the declaration the identical streams drain completely.
    let drained = fuse_spec(&case_a(), Declared::Nothing, bound);
    assert_eq!(
        ranks_pulled(&drained, "docs"),
        1_000,
        "with no declaration there is nothing to license an early stop, and the \
         drain is what this whole mechanism removes"
    );
    assert_eq!(ranks_pulled(&drained, "people"), 1_000);
    // And the status the drain produces, which is the other half of what the
    // declaration buys: a stream read to its end reports the vocabulary's one
    // completeness claim, so the undeclared run is not merely slower, it
    // answers a different question about the stratum than the bounded run does.
    for block in ["docs", "people"] {
        assert_eq!(
            drained.trailer.statuses.get(&stratum(block)),
            Some(&ProducerStatus::Exhausted {
                rows_emitted: 1_000
            }),
            "the undeclared run drained {block} and says so"
        );
    }

    // Case C: one stratum really does run out, and the bound applies to the
    // other. A ceiling that only held when both streams were long would be a
    // ceiling that never met a short stratum.
    let c = fuse_spec(&case_c(), Declared::ItsOwnBlocks, bound);
    assert_eq!(
        c.trailer.statuses.get(&stratum("docs")),
        Some(&ProducerStatus::Exhausted { rows_emitted: 3 }),
        "three rows were all it had, so it is exhausted rather than bounded"
    );
    assert_eq!(ranks_pulled(&c, "docs"), 3);
    assert!(
        ranks_pulled(&c, "people") <= 7,
        "case C read {} ranks from the long stratum; the bound is 5 + 2",
        ranks_pulled(&c, "people")
    );

    // The cross-cutting configuration: a producer that names EVERY block must
    // not collapse the bound. It cannot be skipped by anyone — it meets every
    // candidate's domain — so it is read as deeply as certification needs, and
    // that is still `k` plus a head per stream.
    let cross = fuse_spec(&case_cross_cutting(), Declared::ItsOwnBlocks, bound);
    assert_eq!(cross.rows.len(), 5);
    for name in ["docs", "people", "prior"] {
        assert!(
            ranks_pulled(&cross, name) <= 8,
            "the cross-cutting configuration read {} ranks from {name}; the bound is 5 + 3",
            ranks_pulled(&cross, name)
        );
    }
}

// T6.2. A stream the bound stopped says so, and a stream that ran out says
// that. The two are different claims and the trailer must not merge them.
#[test]
fn a_bounded_domain_run_reports_a_ceiling_and_an_exhausted_one_reports_exhaustion() {
    let bound = TopK::new(5);
    let a = fuse_spec(&case_a(), Declared::ItsOwnBlocks, bound);

    for name in ["docs", "people"] {
        let pulled = ranks_pulled(&a, name);
        // The bound is computed here, from the profile's own law over the rank
        // this run actually reached — never read back out of the value under
        // test. A head is the last row pulled and is not yet merged, so
        // everything at or above its contribution was read and nothing below
        // it was.
        let expected = contribution(Fixed::ONE, pulled, K).expect("the fixture contribution fits");
        assert_eq!(
            a.trailer.statuses.get(&stratum(name)),
            Some(&ProducerStatus::CeilingReached { bound: expected }),
            "a stratum stopped by the row bound is closed at the contribution it \
             was read down to, never drained into an `Exhausted` it did not earn"
        );
    }

    // Case B: both streams are shorter than the bound, so nothing is stopped
    // and both reach their own receipt. This is the valid neighbour — a
    // `CeilingReached` written here would be a bound claimed where the rows
    // simply ran out.
    let b = fuse_spec(&case_b(), Declared::ItsOwnBlocks, bound);
    for name in ["docs", "people"] {
        assert_eq!(
            b.trailer.statuses.get(&stratum(name)),
            Some(&ProducerStatus::Exhausted { rows_emitted: 2 }),
            "case B's streams ran out; nothing bounded them"
        );
    }

    // Case C carries one of each, in one trailer.
    let c = fuse_spec(&case_c(), Declared::ItsOwnBlocks, bound);
    assert_eq!(
        c.trailer.statuses.get(&stratum("docs")),
        Some(&ProducerStatus::Exhausted { rows_emitted: 3 })
    );
    let people_pulled = ranks_pulled(&c, "people");
    assert_eq!(
        c.trailer.statuses.get(&stratum("people")),
        Some(&ProducerStatus::CeilingReached {
            bound: contribution(Fixed::ONE, people_pulled, K).expect("the contribution fits"),
        }),
    );
}

/// The ANSWER a fused row carries: what it is, what it scored, and where that
/// score came from. Everything a caller reads as the result.
///
/// [`FusedRow::threshold_witness`] is deliberately not here, and it is the one
/// field of a row that is a statement about the READ rather than about the
/// answer: it is the threshold in force at the instant the row certified, so it
/// falls as the streams are read further and is zero once they are exhausted. A
/// declaration that lets fusion stop early certifies the identical row against
/// a higher threshold — a *stronger* witness, not a different answer — and
/// requiring the two runs to agree on it would be requiring the bounded run to
/// have read as far as the draining one, which is the whole thing being
/// removed. It is checked below as the invariant it really is: a certified
/// row's score is never beneath the threshold it was certified against.
fn answer_of(row: &FusedRow) -> (Term, Fixed, Vec<(Iri, u64, Fixed)>) {
    (row.entity.clone(), row.score, row.contributions.clone())
}

/// The trailer facts a declaration is ALLOWED to move, named once so the
/// soundness assertions below can say what they cover by saying what they do
/// not.
///
/// Four of them, and each is a statement about the READ rather than about the
/// answer: `ranks_pulled` and `collisions_observed` count rows this run
/// actually pulled, `statuses` says how each stream's read ended, and
/// `cut_on_a_tie` reports whether the last emitted row beat a rival that was
/// already settled *at the moment it was emitted* — which depends on how far
/// the other streams had been read. `domains` moves too, and is not in the same
/// category: it is the declaration under test, an input echoed back for audit,
/// not a result. Within a row, `threshold_witness` moves for the same reason —
/// see [`answer_of`].
///
/// Everything else in a trailer must be identical, and `assert_same_answer`
/// checks each one by name.
fn assert_same_answer(
    declared: &FusionResult<Term>,
    unrestricted: &FusionResult<Term>,
    oracle: &[(String, Fixed, u64)],
    top_k: TopK,
    context: &str,
) {
    // The rows as answers: entity, score and per-stratum provenance. A
    // declaration that changed any of these would be buying its bound with a
    // wrong answer.
    let declared_answers: Vec<_> = declared.rows.iter().map(answer_of).collect();
    let unrestricted_answers: Vec<_> = unrestricted.rows.iter().map(answer_of).collect();
    assert_eq!(
        declared_answers, unrestricted_answers,
        "{context}: the declared run's rows differ from the same streams fused \
         with no declaration at all"
    );

    // And every row of both runs is certified against a threshold it really
    // beat, which is what makes the differing witnesses above a difference in
    // strength rather than in kind.
    for row in declared.rows.iter().chain(&unrestricted.rows) {
        assert!(
            row.score >= row.threshold_witness,
            "{context}: {} was certified at a score beneath the threshold in \
             force, which certification forbids",
            row.entity
        );
    }

    // And against the oracle, so "both engines agree" cannot be both engines
    // being wrong the same way.
    let expected: Vec<&(String, Fixed, u64)> = oracle.iter().take(top_k.get()).collect();
    assert_eq!(
        declared.rows.len(),
        expected.len(),
        "{context}: row count against the oracle"
    );
    for (row, (item, score, rank)) in declared.rows.iter().zip(&expected) {
        assert_eq!(row.entity, Term::new(item.clone()), "{context}: order");
        assert_eq!(row.score, *score, "{context}: score of {item}");
        let best = row
            .contributions
            .iter()
            .map(|(_, rank, _)| *rank)
            .min()
            .expect("a row has at least one contribution");
        assert_eq!(best, *rank, "{context}: best rank of {item}");
    }

    // The identities and the evidence: none of them is a function of how deep
    // this run read, so none of them may move with the declaration.
    assert_eq!(
        declared.trailer.profile_id, unrestricted.trailer.profile_id,
        "{context}: the fusion law is the law, whatever the producers declared"
    );
    assert_eq!(declared.trailer.plan_id, unrestricted.trailer.plan_id);
    assert_eq!(
        declared.trailer.evidence_id,
        unrestricted.trailer.evidence_id
    );
    assert_eq!(
        declared.trailer.attestations,
        unrestricted.trailer.attestations
    );
    assert_eq!(declared.trailer.exactness, unrestricted.trailer.exactness);

    // The resolution map: the same strata, each with the same separation. Its
    // other two fields are counts of what was read and are expected to differ —
    // that difference is the whole point — so they are compared nowhere here.
    assert_eq!(
        declared.trailer.resolution.keys().collect::<Vec<_>>(),
        unrestricted.trailer.resolution.keys().collect::<Vec<_>>(),
        "{context}: the same strata are reported either way"
    );
    for (stratum_iri, measured) in &declared.trailer.resolution {
        assert_eq!(
            measured.separation, unrestricted.trailer.resolution[stratum_iri].separation,
            "{context}: separation is a property of the law and of the stratum's \
             weight, never of how deep this run read"
        );
    }
}

// T6.3. THE SOUNDNESS TEST. A declaration changes how much is read and nothing
// else: the same rows, the same scores, the same provenance, whether the
// streams declare their blocks, declare the union of every block, or declare
// nothing at all — and all three agree with an eager oracle that never ran the
// engine.
#[test]
fn a_declaration_changes_the_reading_and_never_the_answer() {
    for (name, spec, bound) in [
        ("case A", case_a(), TopK::new(5)),
        ("case B", case_b(), TopK::new(5)),
        ("case C", case_c(), TopK::new(5)),
        ("cross-cutting", case_cross_cutting(), TopK::new(5)),
    ] {
        let declared = fuse_spec(&spec, Declared::ItsOwnBlocks, bound);
        let unrestricted = fuse_spec(&spec, Declared::Nothing, bound);
        assert_same_answer(&declared, &unrestricted, &spec_oracle(&spec), bound, name);
    }
}

/// A deterministic stream of values, seeded once per configuration.
///
/// A named, fixed recurrence rather than a random-number generator: every
/// configuration below is reproduced exactly by its index, so a failure names a
/// case a reader can rebuild, and nothing in an assertion depends on a draw.
fn seeded(state: &mut u64) -> u64 {
    // A 64-bit xorshift. Its only property that matters here is that it is a
    // pure function of its state, identical on every target.
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// Configuration `index` of the differential: three strata over a twelve-item
/// universe partitioned into two blocks, with a cross-cutting third producer.
///
/// The partition is what makes every declaration TRUE — each item is in exactly
/// one block and each producer emits only items from the blocks it declares —
/// which is the precondition the whole mechanism assumes and which a host is
/// responsible for. A configuration that violated it would be testing the
/// refusal, and the refusal is T6.5's subject.
fn differential_spec(index: u64) -> Vec<StratumSpec> {
    let mut state = index.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    // Each item's block, drawn once and then respected by every producer.
    let blocks: Vec<&'static str> = (0..12)
        .map(|_: u64| {
            if seeded(&mut state).is_multiple_of(2) {
                DOMAIN_DOCS
            } else {
                DOMAIN_PEOPLE
            }
        })
        .collect();
    // The item's own name says which block it is in, so the partition is
    // readable from any row in isolation — which is what lets every producer here
    // name the same block for the same item without a second table to keep in
    // step. The draw above is still what decides it.
    let universe: Vec<String> = blocks
        .iter()
        .enumerate()
        .map(|(item, block)| {
            let space = if *block == DOMAIN_DOCS {
                "doc"
            } else {
                "person"
            };
            format!("http://example.org/{space}/{item:04}")
        })
        .collect();
    let pick = |state: &mut u64, allowed: &[&'static str]| -> Vec<String> {
        let mut chosen: Vec<String> = universe
            .iter()
            .zip(&blocks)
            .filter(|(_, block)| allowed.contains(block))
            .filter(|_| !seeded(state).is_multiple_of(3))
            .map(|(item, _)| item.clone())
            .collect();
        // A deterministic rotation, so the strata disagree about the order of
        // the items they share — which is what makes the fusion do work.
        if !chosen.is_empty() {
            let rotation = (seeded(state) % chosen.len() as u64) as usize;
            chosen.rotate_left(rotation);
        }
        chosen
    };
    let weights = [
        Fixed::ONE,
        Fixed::from_raw(700_000_000_000),
        Fixed::from_raw(300_000_000_000),
    ];
    let docs_items = pick(&mut state, &[DOMAIN_DOCS]);
    let people_items = pick(&mut state, &[DOMAIN_PEOPLE]);
    let prior_items = pick(&mut state, &[DOMAIN_DOCS, DOMAIN_PEOPLE]);
    vec![
        StratumSpec {
            name: "docs",
            tags: vec![DOMAIN_DOCS],
            weight: weights[(seeded(&mut state) % 3) as usize],
            items: docs_items,
        },
        StratumSpec {
            name: "people",
            tags: vec![DOMAIN_PEOPLE],
            weight: weights[(seeded(&mut state) % 3) as usize],
            items: people_items,
        },
        StratumSpec {
            name: "prior",
            tags: vec![DOMAIN_DOCS, DOMAIN_PEOPLE],
            weight: weights[(seeded(&mut state) % 3) as usize],
            items: prior_items,
        },
    ]
}

// T6.3, continued: the same claim over 256 deterministically seeded
// configurations, plus the lattice property the mechanism rests on.
#[test]
fn the_differential_holds_under_declared_domains_over_many_configurations() {
    let mut exercised = 0_u32;
    for index in 0..256 {
        let spec = differential_spec(index);
        // A configuration where nothing is emitted proves nothing, so the ones
        // that do work are counted and the count is asserted at the end.
        if spec.iter().all(|entry| entry.items.is_empty()) {
            continue;
        }
        for bound in [TopK::new(1), TopK::new(3), TopK::new(8)] {
            let nothing = fuse_spec(&spec, Declared::Nothing, bound);
            let union = fuse_spec(&spec, Declared::TheUnion, bound);
            let own = fuse_spec(&spec, Declared::ItsOwnBlocks, bound);
            let oracle = spec_oracle(&spec);
            let context = format!("configuration {index}, bound {bound}");
            assert_same_answer(&own, &nothing, &oracle, bound, &context);
            assert_same_answer(&union, &nothing, &oracle, bound, &context);

            // The lattice property, and the reason the middle rung exists.
            // Reading is non-increasing as the declarations get finer: a wider
            // promise licenses nothing a narrower one does not. `TheUnion` is a
            // real declaration that licenses no skipping at all, so it must
            // read exactly what no declaration reads — if it read less, the
            // engine would be skipping a stream that had promised nothing.
            for entry in &spec {
                let widest = ranks_pulled(&nothing, entry.name);
                let middle = ranks_pulled(&union, entry.name);
                let finest = ranks_pulled(&own, entry.name);
                assert_eq!(
                    widest, middle,
                    "{context}: {} — a declaration that meets every other \
                     declaration licenses no early stop",
                    entry.name
                );
                assert!(
                    finest <= middle,
                    "{context}: {} — a finer declaration read MORE ({finest} \
                     against {middle}), which is the lattice inverted",
                    entry.name
                );
            }
            exercised += 1;
        }
    }
    assert!(
        exercised >= 200,
        "the differential must exercise at least 200 configurations, ran {exercised}"
    );
}

/// The smallest weight the fixed-point carries, and the one whose decay collides
/// immediately: at this weight ranks one and two already share a contribution value.
///
/// `Fixed::ONE` collides too, but not until rank 1_000_941 — past the end of any
/// fixture here — so a collision assertion over a `case_a` run at `Fixed::ONE` is an
/// assertion that zero equals zero. It would pass with the collision counter emptied,
/// and it would pass against the very estimate its own message forbids. This is the
/// weight the two sibling tests that enter the collided regime already use.
const COLLIDED_WEIGHT: Fixed = Fixed::from_raw(1);

/// [`case_a`] at [`COLLIDED_WEIGHT`]: the same streams, the same blocks, the same
/// lengths, weighted so that adjacent ranks collide from the second one on.
fn case_a_collided() -> Vec<StratumSpec> {
    case_a()
        .into_iter()
        .map(|entry| StratumSpec {
            weight: COLLIDED_WEIGHT,
            ..entry
        })
        .collect()
}

// T6.4. The two measured fields stay measurements. `collisions_observed` counts
// what this run actually saw, and `separation` is the profile's own answer for
// the stratum — neither is estimated from the other.
//
// The fixture is weighted into the collided regime on purpose, and the guard below
// holds it there: a counter assertion over a read containing no collision is an
// assertion about nothing, which is what this test used to be.
//
// That weight is also why the reading here is the full drain rather than the bounded
// prefix T6.1 measures, and the two facts are the same fact. A declaration licenses an
// early stop only when a threshold can fall below the live heads, and a collided
// regime is precisely where it cannot: every rank carries one contribution value, so
// nothing separates and nothing may be skipped. No single weight can put a collision
// inside a read that a declaration shortened — what this test measures is the
// counters, in the one regime where the collision counter has anything to count.
#[test]
fn the_resolution_counters_stay_observations_under_a_declared_domain() {
    let bound = TopK::new(5);
    let spec = case_a_collided();
    let result = fuse_spec(&spec, Declared::ItsOwnBlocks, bound);
    let law = spec_profile(&spec);

    for entry in &spec {
        let measured = result
            .trailer
            .resolution
            .get(&stratum(entry.name))
            .expect("the fixture profile weights every fixture stratum");

        // Derived from the rows this run pulled, by walking the profile's curve
        // over exactly those ranks: two adjacent ranks collide when the
        // fixed-point decay gives them one value. Computed here rather than
        // predicted from `separation`, so a count that drifted from the rows
        // would fail even where the profile's a-priori bound was unchanged.
        let mut expected = 0_u64;
        for rank in 2..=measured.ranks_pulled {
            let previous =
                contribution(entry.weight, rank - 1, K).expect("the fixture contribution fits");
            let current =
                contribution(entry.weight, rank, K).expect("the fixture contribution fits");
            if previous == current {
                expected += 1;
            }
        }
        // Non-vacuity, the crossing guard its siblings carry: a fixture whose
        // pulled ranks hold no collision makes the equality below `0 == 0`, which
        // an emptied counter satisfies as readily as a correct one.
        assert!(
            expected > 0,
            "{}: the pulled ranks must contain a collision, or the count below \
             asserts nothing",
            entry.name
        );
        assert_eq!(
            measured.collisions_observed, expected,
            "the collision count must be what the pulled rows show, not an \
             estimate from the profile's bound"
        );
        assert_eq!(
            measured.separation,
            law.monotone_depth(&stratum(entry.name))
                .expect("the profile weights this stratum"),
            "separation is the profile's own answer for this stratum, and what the \
             read did does not move it"
        );
    }
}

// T6.5. The declaration is held to, and the refusal is narrow.
//
// Two streams whose declarations put one candidate in two disjoint blocks
// cannot both be telling the truth about it. The consumer says so — naming the
// item, the offending stratum, and the stratum whose declaration it
// contradicted — rather than picking a side, because it has already certified
// rows on the strength of those declarations.
#[test]
fn a_stream_naming_a_candidate_outside_its_declared_domains_is_refused() {
    let shared = "http://example.org/doc/000001";
    let docs = || {
        MockStream::new(
            vec![row_in(1, Fixed::ONE, K, shared, DOMAIN_DOCS)],
            ProducerReceipt::Exhausted { rows_emitted: 1 },
        )
    };

    // (i) While the candidate is still in the frontier. `people`'s first row
    // names the candidate `docs` has just put there, so the contradiction is
    // found against a live frontier entry.
    let in_frontier = vec![
        (
            stratum("docs"),
            docs().declaring(StreamContract::new(
                DuplicatePolicy::Unique,
                RankFidelity::EXACT,
                within(&[DOMAIN_DOCS]),
                ExclusionBasis::Unavailable,
            )),
        ),
        (
            stratum("people"),
            MockStream::new(
                vec![
                    // `people` says this candidate came from ITS block, which is
                    // what makes the pair of declarations contradictory rather
                    // than merely unusual.
                    row_in(1, Fixed::ONE, K, shared, DOMAIN_PEOPLE),
                    row_in(
                        2,
                        Fixed::ONE,
                        K,
                        "http://example.org/person/000001",
                        DOMAIN_PEOPLE,
                    ),
                ],
                exhausted(2),
            )
            .declaring(StreamContract::new(
                DuplicatePolicy::Unique,
                RankFidelity::EXACT,
                within(&[DOMAIN_PEOPLE]),
                ExclusionBasis::Unavailable,
            )),
        ),
    ];
    let error = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        in_frontier,
        &profile(&[("docs", Fixed::ONE), ("people", Fixed::ONE)], K),
        TopK::new(10),
    ))
    .expect_err("a declaration this row contradicts must be refused");
    match error {
        FusionError::Protocol(protocol) => match *protocol {
            ProtocolError::OutsideDeclaredDomain {
                item,
                stratum: offender,
                named_by,
            } => {
                assert_eq!(item, shared, "the refusal names the candidate");
                assert_eq!(
                    offender,
                    stratum("people").as_str(),
                    "and the stratum whose stream broke its own declaration"
                );
                assert_eq!(
                    named_by,
                    stratum("docs").as_str(),
                    "and the declaration it contradicted, without which a reader \
                     cannot find the other half of the disagreement"
                );
            }
            other => panic!("expected an outside-domain refusal, got {other:?}"),
        },
        other => panic!("expected a protocol refusal, got {other:?}"),
    }

    // (ii) After the candidate has been certified and left the frontier. A
    // promise that lapses once a row is emitted is a promise that depends on
    // how deep the caller read, which is not a promise.
    let after_emission = vec![
        (
            stratum("docs"),
            docs().declaring(StreamContract::new(
                DuplicatePolicy::Unique,
                RankFidelity::EXACT,
                within(&[DOMAIN_DOCS]),
                ExclusionBasis::Unavailable,
            )),
        ),
        (
            stratum("people"),
            MockStream::new(
                vec![
                    row_in(
                        1,
                        Fixed::ONE,
                        K,
                        "http://example.org/person/000001",
                        DOMAIN_PEOPLE,
                    ),
                    row_in(2, Fixed::ONE, K, shared, DOMAIN_PEOPLE),
                ],
                exhausted(2),
            )
            .declaring(StreamContract::new(
                DuplicatePolicy::Unique,
                RankFidelity::EXACT,
                within(&[DOMAIN_PEOPLE]),
                ExclusionBasis::Unavailable,
            )),
        ),
    ];
    let error = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        after_emission,
        &profile(&[("docs", Fixed::ONE), ("people", Fixed::ONE)], K),
        TopK::new(10),
    ))
    .expect_err("the promise is held for the whole fusion, not only for the frontier");
    assert!(
        matches!(
            &error,
            FusionError::Protocol(protocol)
                if matches!(
                    &**protocol,
                    ProtocolError::OutsideDeclaredDomain { item, stratum: offender, named_by }
                        if item == shared
                            && offender == stratum("people").as_str()
                            && named_by == stratum("docs").as_str()
                )
        ),
        "a late name must be refused exactly as an early one is, got {error:?}"
    );
}

// T6.5b. The axiom itself, held against the ROWS rather than the declarations.
//
// Two declarations can overlap — both naming `documents` — and still place one
// candidate in two different blocks, one row each. Nothing about that pair of
// promises is contradictory, so `OutsideDeclaredDomain` cannot see it; what is
// contradictory is the pair of rows, and it is exactly the case the threshold's
// per-block maximum under-bounds, because the streams that reach one block and
// the streams that reach the other are different sets.
//
// The shipped-path proof of this refusal is `search.rs`'s
// `two_streams_naming_one_candidate_from_two_blocks_are_refused`. What is pinned
// here is the engine-level case a producer reaches through `fuse` with its own
// streams, including the one row `pull` never sees: a repeat a permissive
// duplicate policy discards.
#[test]
fn a_candidate_two_rows_place_in_two_blocks_is_refused_and_one_block_fuses() {
    let shared = "http://example.org/doc/000001";
    let law = profile(&[("docs", Fixed::ONE), ("people", Fixed::ONE)], K);
    // Overlapping declarations: both admit `documents`, so a candidate both name
    // is perfectly possible and the declaration-level refusal stays silent.
    let overlapping = || {
        StreamContract::new(
            DuplicatePolicy::Unique,
            RankFidelity::EXACT,
            within(&[DOMAIN_DOCS, DOMAIN_PEOPLE]),
            ExclusionBasis::Unavailable,
        )
    };
    let pair = |left_block: &'static str, right_block: &'static str| {
        vec![
            (
                stratum("docs"),
                MockStream::new(
                    vec![row_in(1, Fixed::ONE, K, shared, left_block)],
                    exhausted(1),
                )
                .declaring(overlapping()),
            ),
            (
                stratum("people"),
                MockStream::new(
                    vec![row_in(1, Fixed::ONE, K, shared, right_block)],
                    exhausted(1),
                )
                .declaring(overlapping()),
            ),
        ]
    };

    let error = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        pair(DOMAIN_DOCS, DOMAIN_PEOPLE),
        &law,
        TopK::new(10),
    ))
    .expect_err("a candidate in two blocks falsifies the axiom the threshold rests on");
    match error {
        FusionError::Protocol(protocol) => match *protocol {
            ProtocolError::CandidateInTwoBlocks {
                item,
                stratum: offender,
                block,
                named_by,
                named_by_block,
            } => {
                assert_eq!(item, shared, "the refusal names the candidate");
                assert_eq!(offender, stratum("people").as_str());
                assert_eq!(block, DOMAIN_PEOPLE, "the block the later row claimed");
                assert_eq!(named_by, stratum("docs").as_str());
                assert_eq!(
                    named_by_block, DOMAIN_DOCS,
                    "and the placement it contradicted — either producer could be \
                     the one that tagged wrongly, so both are named"
                );
            }
            other => panic!("expected a two-block refusal, got {other:?}"),
        },
        other => panic!("expected a protocol refusal, got {other:?}"),
    }

    // The valid neighbour, one block different: both streams place the candidate
    // in the SAME block. Two producers ranking one entity is what fused
    // enumeration is for, and it fuses into one row carrying both contributions.
    let agreeing = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        pair(DOMAIN_DOCS, DOMAIN_DOCS),
        &law,
        TopK::new(10),
    ))
    .expect("agreeing placements are the ordinary overlapping case");
    assert_eq!(agreeing.rows.len(), 1, "one candidate, one row");
    assert_eq!(agreeing.rows[0].entity, Term::new(shared));
    assert_eq!(
        agreeing.rows[0].contributions.len(),
        2,
        "and both strata's contributions, which is what the agreement buys"
    );
}

// T6.5d. An unrestricted stream owes no block — and a block it volunteers is
// still evidence about the CANDIDATE, so it is honoured rather than discarded.
//
// Both halves are executed, because both are decisions. Discarding the volunteered
// block would throw away a host's own evidence that its tagging is wrong;
// *requiring* one would refuse a producer that promised nothing and therefore
// tightened nothing.
#[test]
fn a_volunteered_block_is_honoured_and_a_silent_unrestricted_stream_still_fuses() {
    let shared = "http://example.org/doc/000001";
    let law = profile(&[("wide", Fixed::ONE), ("people", Fixed::ONE)], K);
    let restricted = || {
        StreamContract::new(
            DuplicatePolicy::Unique,
            RankFidelity::EXACT,
            within(&[DOMAIN_PEOPLE]),
            ExclusionBasis::Unavailable,
        )
    };
    let pair = |volunteered: Step| {
        vec![
            (
                stratum("wide"),
                MockStream::new(vec![volunteered], exhausted(1)).declaring(unique_items()),
            ),
            (
                stratum("people"),
                MockStream::new(
                    vec![row_in(1, Fixed::ONE, K, shared, DOMAIN_PEOPLE)],
                    exhausted(1),
                )
                .declaring(restricted()),
            ),
        ]
    };

    // Volunteered and contradictory: the unrestricted stream places the candidate
    // in `documents` and the restricted one in `people`. One candidate cannot be
    // in two blocks, and an unrestricted declaration does not make the claim
    // unfalsifiable — it only means this stream was never obliged to make it.
    let error = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        pair(row_in(1, Fixed::ONE, K, shared, DOMAIN_DOCS)),
        &law,
        TopK::new(10),
    ))
    .expect_err("a volunteered block that contradicts a placement is still a contradiction");
    assert!(
        matches!(
            &error,
            FusionError::Protocol(protocol)
                if matches!(&**protocol, ProtocolError::CandidateInTwoBlocks { item, .. } if item == shared)
        ),
        "expected a two-block refusal, got {error:?}"
    );

    // Silent, which is what an unrestricted stream owes: no block, nothing to
    // contradict, and the two streams fuse the shared candidate into one row.
    let quiet = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        pair(row(1, Fixed::ONE, K, shared)),
        &law,
        TopK::new(10),
    ))
    .expect("an unrestricted stream owes no block and is not refused for naming none");
    assert_eq!(quiet.rows.len(), 1, "one candidate, one row");
    assert_eq!(
        quiet.rows[0].contributions.len(),
        2,
        "and both strata contributed, exactly as they did before blocks existed"
    );
}

// T6.5c. The row `pull` never sees. A stream that declared `Allowed` has its
// repeats discarded by the consumer, and a discarded row is still a row the
// producer made claims about: it may not smuggle a second, different placement
// for a candidate past the check by repeating it.
#[test]
fn a_dropped_duplicate_may_not_place_its_candidate_in_a_second_block() {
    let shared = "http://example.org/doc/000001";
    let law = profile(&[("docs", Fixed::ONE)], K);
    let repeating = || {
        StreamContract::new(
            DuplicatePolicy::Allowed,
            RankFidelity::EXACT,
            within(&[DOMAIN_DOCS, DOMAIN_PEOPLE]),
            ExclusionBasis::Unavailable,
        )
    };
    let stream = |second_block: &'static str| {
        vec![(
            stratum("docs"),
            MockStream::new(
                vec![
                    row_in(1, Fixed::ONE, K, shared, DOMAIN_DOCS),
                    row_in(2, Fixed::ONE, K, shared, second_block),
                ],
                exhausted(2),
            )
            .declaring(repeating()),
        )]
    };

    let error = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        stream(DOMAIN_PEOPLE),
        &law,
        TopK::new(10),
    ))
    .expect_err("a repeat may be dropped; its claim about the candidate may not be");
    assert!(
        matches!(
            &error,
            FusionError::Protocol(protocol)
                if matches!(
                    &**protocol,
                    ProtocolError::CandidateInTwoBlocks { item, block, named_by_block, .. }
                        if item == shared
                            && block == DOMAIN_PEOPLE
                            && named_by_block == DOMAIN_DOCS
                )
        ),
        "the second placement must be refused even though the row it rode in on \
         was about to be discarded, got {error:?}"
    );

    // The valid neighbour: the identical repeat, placed in the block the first
    // row already placed the candidate in. That contradicts nothing, so it is
    // de-duplicated exactly as this policy says it must be — one row, one
    // contribution, and the producer still charged for both rows it emitted.
    let consistent = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        stream(DOMAIN_DOCS),
        &law,
        TopK::new(10),
    ))
    .expect("a repeat that agrees about the candidate's block is an ordinary repeat");
    assert_eq!(
        consistent.rows.len(),
        1,
        "the repeat was dropped, not fused"
    );
    assert_eq!(
        consistent.rows[0].contributions.len(),
        1,
        "one contribution per stratum, at the best rank the stream gave it"
    );
    assert_eq!(
        consistent.trailer.statuses.get(&stratum("docs")),
        Some(&ProducerStatus::Exhausted { rows_emitted: 2 }),
        "and the dropped row is still a row the producer emitted"
    );
}

// T6.5's valid neighbours. The refusal above is about a pair of DECLARATIONS,
// not about two producers naming one entity — which is the case fused
// enumeration exists for, and which must keep working unchanged.
#[test]
fn two_producers_naming_one_entity_fuse_normally_unless_they_declared_otherwise() {
    let shared = "http://example.org/doc/000001";
    let law = profile(&[("docs", Fixed::ONE), ("people", Fixed::ONE)], K);
    // Each side is a declaration and the row that backs it. A restricted stream
    // owes a block on every row, and both sides name the block the shared
    // candidate is really in — it is one document, and it is in one block, which
    // is the axiom holding rather than being violated.
    let both_naming = |left: (StreamContract, Step), right: (StreamContract, Step)| {
        vec![
            (
                stratum("docs"),
                MockStream::new(vec![left.1], exhausted(1)).declaring(left.0),
            ),
            (
                stratum("people"),
                MockStream::new(vec![right.1], exhausted(1)).declaring(right.0),
            ),
        ]
    };
    let blockless = || row(1, Fixed::ONE, K, shared);
    let in_docs = || row_in(1, Fixed::ONE, K, shared, DOMAIN_DOCS);

    // (i) No declaration at all: one row, two contributions, the sum of both.
    let unrestricted = block_on(run_fuse(
        both_naming((unique_items(), blockless()), (unique_items(), blockless())),
        &law,
    ));
    let doubled = contribution(Fixed::ONE, 1, K)
        .expect("fits")
        .checked_add(contribution(Fixed::ONE, 1, K).expect("fits"))
        .expect("fits");
    assert_eq!(unrestricted.rows.len(), 1);
    assert_eq!(unrestricted.rows[0].entity, Term::new(shared));
    assert_eq!(unrestricted.rows[0].score, doubled);
    assert_eq!(unrestricted.rows[0].contributions.len(), 2);

    // (ii) Two producers declared over the SAME block, naming the same entity:
    // the declarations agree, so nothing is refused and the answer is
    // identical. This is the case a stratum-derived tag would have broken.
    let agreeing = block_on(run_fuse(
        both_naming(
            (
                StreamContract::new(
                    DuplicatePolicy::Unique,
                    RankFidelity::EXACT,
                    within(&[DOMAIN_DOCS]),
                    ExclusionBasis::Unavailable,
                ),
                in_docs(),
            ),
            (
                StreamContract::new(
                    DuplicatePolicy::Unique,
                    RankFidelity::EXACT,
                    within(&[DOMAIN_DOCS]),
                    ExclusionBasis::Unavailable,
                ),
                in_docs(),
            ),
        ),
        &law,
    ));
    assert_eq!(
        agreeing.rows, unrestricted.rows,
        "two producers over one block are the ordinary overlapping case, and \
         the declaration must not cost them their fused row"
    );

    // (iii) Declarations that overlap without being equal: `docs` names one
    // block, the cross-cutting producer names both. They meet, so the candidate
    // fuses.
    let overlapping = block_on(run_fuse(
        both_naming(
            (
                StreamContract::new(
                    DuplicatePolicy::Unique,
                    RankFidelity::EXACT,
                    within(&[DOMAIN_DOCS]),
                    ExclusionBasis::Unavailable,
                ),
                in_docs(),
            ),
            (
                StreamContract::new(
                    DuplicatePolicy::Unique,
                    RankFidelity::EXACT,
                    within(&[DOMAIN_DOCS, DOMAIN_PEOPLE]),
                    ExclusionBasis::Unavailable,
                ),
                // The cross-cutting producer names two blocks and says which one
                // THIS row came from, which is the whole point of a per-row
                // block: a several-block declaration is backed row by row.
                in_docs(),
            ),
        ),
        &law,
    ));
    assert_eq!(overlapping.rows, unrestricted.rows);
}

// T6.6. The declaration is part of the producer's identity, and of nothing
// else's. It reaches the registry fingerprint — pinned in `sparql-eval`'s own
// `ranked_retrieval_fingerprint.rs` and, through the plan, in `plan.rs` — and
// it must NOT reach the fusion profile, which describes the law rather than the
// producers.
#[test]
fn a_declaration_never_moves_the_fusion_profiles_identity() {
    let spec = case_a();
    let law = spec_profile(&spec);
    let bound = TopK::new(5);

    let declared = fuse_spec(&spec, Declared::ItsOwnBlocks, bound);
    let unrestricted = fuse_spec(&spec, Declared::Nothing, bound);

    assert_eq!(
        declared.trailer.profile_id,
        law.id(),
        "the trailer names the law in force"
    );
    assert_eq!(
        unrestricted.trailer.profile_id,
        law.id(),
        "and the same law under a different set of producer declarations"
    );

    // The profile is a function of its strata, weights and decay rule, and a
    // producer's promise about its own candidates is none of those. A caller
    // comparing two answers' `profile_id` is asking whether they were scored
    // by the same law; folding a producer's declaration in would make that
    // question unanswerable.
    assert_eq!(declared.trailer.profile_id, unrestricted.trailer.profile_id);
    assert_eq!(
        law.id(),
        spec_profile(&case_b()).id(),
        "the fixture profiles are the same law"
    );
}

// T6.7. The licence must not leak into the case the membership test exists for.
//
// `is_final` skips a stream that CANNOT name a candidate. A stream that can
// name it — same block, or no declaration — still blocks certification even
// when its contribution is exactly zero, because a zero contribution is still a
// rank, a provenance entry and a possible better rank. The test next door
// (`a_live_zero_contribution_stream_is_not_mistaken_for_an_exhausted_one`) pins
// that with no declarations at all; this one pins it with declarations present
// and agreeing, which is where a too-eager skip would hide.
#[test]
fn a_live_zero_contribution_stream_in_the_same_domain_still_blocks_certification() {
    let law = profile(
        &[("dense", Fixed::ONE), ("sparse", ZERO_CONTRIBUTING_WEIGHT)],
        K,
    );
    let dense_contribution = contribution(Fixed::ONE, 1, K).expect("fits");
    let target = "http://example.org/doc/000001";
    let same_block = || {
        StreamContract::new(
            DuplicatePolicy::Unique,
            RankFidelity::EXACT,
            within(&[DOMAIN_DOCS]),
            ExclusionBasis::Unavailable,
        )
    };

    let streams = vec![
        (
            stratum("dense"),
            MockStream::new(
                vec![row_in(1, Fixed::ONE, K, target, DOMAIN_DOCS)],
                exhausted(1),
            )
            .declaring(same_block()),
        ),
        (
            stratum("sparse"),
            MockStream::new(
                vec![
                    row_in(
                        1,
                        ZERO_CONTRIBUTING_WEIGHT,
                        K,
                        "http://example.org/doc/000002",
                        DOMAIN_DOCS,
                    ),
                    row_in(2, ZERO_CONTRIBUTING_WEIGHT, K, target, DOMAIN_DOCS),
                ],
                exhausted(2),
            )
            .declaring(same_block()),
        ),
    ];
    let result = block_on(run_fuse(streams, &law));

    let entities: Vec<&str> = result
        .rows
        .iter()
        .map(|fused| fused.entity.as_str())
        .collect();
    assert_eq!(
        entities,
        vec![target, "http://example.org/doc/000002"],
        "two candidates were named, so two rows are the whole answer — the \
         target emitted twice would be one candidate with two scores"
    );
    let fused = &result.rows[0];
    assert_eq!(fused.score, dense_contribution);
    assert_eq!(
        fused.contributions.len(),
        2,
        "a stream in the SAME block can still name this candidate, so it is \
         awaited — the declaration licenses skipping only what it excludes"
    );
    assert_eq!(fused.contributions[0].0, stratum("dense"));
    assert_eq!(fused.contributions[1], (stratum("sparse"), 2, Fixed::ZERO));
}

// T6.9's engine-side half: the trailer echoes the declarations the fusion ran
// under, keyed by stratum, so an answer can be audited against them. The
// registry-to-trailer route is pinned end to end in `search.rs`.
#[test]
fn the_trailer_reports_the_declarations_the_fusion_ran_under() {
    let spec = case_a();
    let bound = TopK::new(5);

    let declared = fuse_spec(&spec, Declared::ItsOwnBlocks, bound);
    assert_eq!(
        declared.trailer.domains,
        BTreeMap::from([
            (stratum("docs"), within(&[DOMAIN_DOCS])),
            (stratum("people"), within(&[DOMAIN_PEOPLE])),
        ]),
        "every handed stream's declaration is reported, verbatim and keyed by \
         its stratum"
    );

    let unrestricted = fuse_spec(&spec, Declared::Nothing, bound);
    assert_eq!(
        unrestricted.trailer.domains,
        BTreeMap::from([
            (stratum("docs"), CandidateDomains::Unrestricted),
            (stratum("people"), CandidateDomains::Unrestricted),
        ]),
        "and a fusion that was licensed to skip nothing says so, rather than \
         leaving a reader to infer it from an absent key"
    );
    assert_eq!(
        unrestricted.trailer.domains.keys().collect::<Vec<_>>(),
        unrestricted.trailer.attestations.keys().collect::<Vec<_>>(),
        "the key set is the streams this fusion was handed, exactly as the \
         attestation map's is"
    );
}

// ---------------------------------------------------------------------------
// T8. The fidelity a producer declared reaches the answer, and the answer's
//     completeness verdict accounts for it.
//
//     A stratum served by an approximate search and one served exhaustively are
//     reported identically by every OTHER field of a trailer: both say
//     `Exhausted`, both carry contiguous ranks, both look like a producer that
//     ran out. The whole of the difference is here.
// ---------------------------------------------------------------------------

/// A producer's own words about what its search does not promise. Carries the
/// characters the canonical framing must survive, so the route from declaration
/// to answer is proved on prose a real producer would publish rather than on a
/// token.
const LOSS: &str = "approximate: beam search; recall UNMEASURED above 10^6 rows, and an \
                    empty result is never a proof of absence";

/// A contract declaring an incomplete but order-faithful search -- what an HNSW
/// graph is: exact distances for the candidates it visits, and it only fails to
/// visit.
fn lossy_contract() -> StreamContract {
    StreamContract::new(
        DuplicatePolicy::Unique,
        RankFidelity {
            completeness: Completeness::Lossy {
                evidence: Arc::from(LOSS),
            },
            order: OrderFidelity::Faithful,
        },
        CandidateDomains::Unrestricted,
        ExclusionBasis::Unavailable,
    )
}

// T8.1. An approximate stratum and an exact one are distinguishable in the
// trailer alone, and the producer's own string arrives byte for byte.
#[test]
fn an_approximate_stratum_is_distinguishable_from_an_exact_one_in_the_trailer() {
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    let fused = block_on(run_fuse(
        vec![
            (
                stratum("text"),
                MockStream::new(
                    vec![(row(1, Fixed::ONE, K, "a"))],
                    ProducerReceipt::Exhausted { rows_emitted: 1 },
                ),
            ),
            (
                stratum("vector"),
                MockStream::new(
                    vec![(row(1, Fixed::ONE, K, "a"))],
                    ProducerReceipt::Exhausted { rows_emitted: 1 },
                )
                .declaring(lossy_contract()),
            ),
        ],
        &profile,
    ));

    // Read from the trailer alone. No registry is in scope here, which is the
    // point: a consumer holding only an answer can tell the two apart.
    assert_eq!(
        fused.trailer.fidelities[&stratum("text")],
        RankFidelity::EXACT,
        "the exhaustive stratum reports the top of the lattice"
    );
    let approximate = &fused.trailer.fidelities[&stratum("vector")];
    assert!(approximate.may_omit(), "and the approximate one does not");

    let evidence: Vec<&str> = approximate.evidence().map(|e| &**e).collect();
    assert_eq!(
        evidence,
        vec![LOSS],
        "the string the producer published is the string the consumer reads: \
         not a boolean derived here, and not re-worded"
    );
}

// T8.2. An exhaustive stratum is REPORTED exact, never omitted. A consumer must
// never have to read an absent key as a claim.
#[test]
fn an_exhaustive_stratum_is_reported_rather_than_left_out() {
    let profile = profile(&[("text", Fixed::ONE)], K);
    let fused = block_on(run_fuse(
        vec![(
            stratum("text"),
            MockStream::new(
                vec![(row(1, Fixed::ONE, K, "a"))],
                ProducerReceipt::Exhausted { rows_emitted: 1 },
            ),
        )],
        &profile,
    ));

    assert!(
        fused.trailer.fidelities.contains_key(&stratum("text")),
        "present, not absent -- a future change that omits exactness instead of \
         stating it fails here"
    );
    assert_eq!(fused.trailer.exactness, ScoreExactness::Exact);
}

// T8.3. The key set is the streams this fusion was handed, exactly as the two
// neighbouring maps are. One key, three facts, no disagreement about how many
// producers there were.
#[test]
fn the_fidelity_map_is_keyed_like_the_maps_beside_it() {
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    let fused = block_on(run_fuse(
        vec![
            (
                stratum("text"),
                MockStream::new(Vec::new(), ProducerReceipt::Exhausted { rows_emitted: 0 }),
            ),
            (
                stratum("vector"),
                MockStream::new(Vec::new(), ProducerReceipt::Exhausted { rows_emitted: 0 })
                    .declaring(lossy_contract()),
            ),
        ],
        &profile,
    ));

    let fidelities: BTreeSet<_> = fused.trailer.fidelities.keys().cloned().collect();
    let attestations: BTreeSet<_> = fused.trailer.attestations.keys().cloned().collect();
    let domains: BTreeSet<_> = fused.trailer.domains.keys().cloned().collect();
    assert_eq!(fidelities, attestations);
    assert_eq!(fidelities, domains);
}

// T8.4. A status is not, on its own, a completeness claim -- the defect this
// work exists to remove. `Exhausted` beside a lossy declaration is not a
// completeness claim, and the answer-level verdict says so rather than leaving
// it to prose.
#[test]
fn exhausted_on_an_approximate_stratum_does_not_make_the_answer_exact() {
    let profile = profile(&[("vector", Fixed::ONE)], K);
    let fused = block_on(run_fuse(
        vec![(
            stratum("vector"),
            MockStream::new(
                vec![(row(1, Fixed::ONE, K, "a"))],
                ProducerReceipt::Exhausted { rows_emitted: 1 },
            )
            .declaring(lossy_contract()),
        )],
        &profile,
    ));

    // The status alone is exactly what an exhaustive producer reports.
    assert_eq!(
        fused.trailer.statuses[&stratum("vector")],
        ProducerStatus::Exhausted { rows_emitted: 1 },
        "the read ending is unchanged -- which is precisely why it cannot be the \
         channel that carries the approximation"
    );
    // And the verdict beside it refuses to call the answer exact.
    assert_eq!(
        fused.trailer.exactness,
        ScoreExactness::Estimated {
            deficit: BTreeSet::from([stratum("vector")]),
            inflation: BTreeSet::from([stratum("vector")]),
            unbounded: BTreeSet::new(),
        },
        "named on BOTH sides: the rows the search missed are absent (deficit), \
         and every row behind a missed one moved up a rank and collected more \
         than it earned (inflation)"
    );
}

// T8.5. Existing statuses are unchanged for exact producers. The neighbouring
// case that must not move: a fusion of exhaustive producers reports exactly what
// it reported before this term existed.
#[test]
fn a_fusion_of_exhaustive_producers_is_still_exact() {
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    let fused = block_on(run_fuse(
        vec![
            (
                stratum("text"),
                MockStream::new(
                    vec![(row(1, Fixed::ONE, K, "a"))],
                    ProducerReceipt::Exhausted { rows_emitted: 1 },
                ),
            ),
            (
                stratum("vector"),
                MockStream::new(
                    vec![(row(1, Fixed::ONE, K, "b"))],
                    ProducerReceipt::Exhausted { rows_emitted: 1 },
                ),
            ),
        ],
        &profile,
    ));

    assert_eq!(
        fused.trailer.exactness,
        ScoreExactness::Exact,
        "an empty `Estimated` must never appear in place of `Exact`"
    );
    for stratum_iri in [stratum("text"), stratum("vector")] {
        assert_eq!(
            fused.trailer.statuses[&stratum_iri],
            ProducerStatus::Exhausted { rows_emitted: 1 },
        );
        assert_eq!(fused.trailer.fidelities[&stratum_iri], RankFidelity::EXACT);
    }
}

// T8.6. A bounded read must not be able to destroy the disclosure. This is the
// test that proves the declaration leg was the right choice: a stream a `TopK`
// stopped never returns a receipt at all, so a fidelity fetched at the END would
// go missing in exactly the runs where the caller read shallowly.
#[test]
fn a_top_k_that_stops_an_approximate_stream_still_reports_its_fidelity() {
    let profile = profile(&[("vector", Fixed::ONE)], K);
    let mut fusion = FusionStream::new(
        vec![(
            stratum("vector"),
            MockStream::new(
                vec![
                    (row(1, Fixed::ONE, K, "a")),
                    (row(2, Fixed::ONE, K, "b")),
                    (row(3, Fixed::ONE, K, "c")),
                ],
                ProducerReceipt::Exhausted { rows_emitted: 3 },
            )
            .declaring(lossy_contract()),
        )],
        profile,
    );

    // Pull one row and stop, leaving the stream open and unreceipted.
    let first = block_on(fusion.next()).expect("a row");
    assert!(first.is_some());
    let trailer = block_on(fusion.trailer()).expect("a trailer");

    assert!(
        trailer.fidelities[&stratum("vector")].may_omit(),
        "the disclosure survives a stop that destroys the receipt"
    );
    assert!(matches!(
        trailer.exactness,
        ScoreExactness::Estimated { .. }
    ));
}

// T8.7. The verdict is INVARIANT under read depth. Its inputs are pinned before
// the first row, so certifying more rows moves the statuses and never this.
#[test]
fn the_exactness_verdict_does_not_move_as_a_caller_reads_deeper() {
    let profile = profile(&[("vector", Fixed::ONE)], K);
    let mut fusion = FusionStream::new(
        vec![(
            stratum("vector"),
            MockStream::new(
                vec![(row(1, Fixed::ONE, K, "a")), (row(2, Fixed::ONE, K, "b"))],
                ProducerReceipt::Exhausted { rows_emitted: 2 },
            )
            .declaring(lossy_contract()),
        )],
        profile,
    );

    block_on(fusion.next()).expect("a row");
    let early = block_on(fusion.trailer()).expect("a trailer");
    while block_on(fusion.next()).expect("a row").is_some() {}
    let late = block_on(fusion.trailer()).expect("a trailer");

    assert_eq!(
        early.exactness, late.exactness,
        "how deep a caller read changes which streams are still open, never \
         whether a producer's search was exhaustive"
    );
    assert_eq!(
        early.fidelities, late.fidelities,
        "and the declarations behind it are equally immovable"
    );
}

// T8.8. A stratum that never became a stream is ABSENT, not filled in with the
// top of the lattice. "Was never asked" and "promised everything" are different
// facts, and fabricating the second would be the strongest possible claim put
// into the mouth of a producer that never spoke.
#[test]
fn a_stratum_added_after_the_fact_declares_nothing() {
    let profile = profile(&[("text", Fixed::ONE)], K);
    let fused = block_on(run_fuse(
        vec![(
            stratum("text"),
            MockStream::new(Vec::new(), ProducerReceipt::Exhausted { rows_emitted: 0 }),
        )],
        &profile,
    ));

    let failed = stratum("vector");
    let trailer = fused.trailer.completed_with([(
        failed.clone(),
        ProducerStatus::ExecutionFailed {
            reason: "the unit could not run".to_owned(),
        },
    )]);

    assert!(
        trailer.statuses.contains_key(&failed),
        "the status grows, because the executor really did report one"
    );
    assert!(
        !trailer.fidelities.contains_key(&failed),
        "the fidelity map does not: no producer of that stratum was ever asked"
    );
}

// ---------------------------------------------------------------------------
// T9. The per-row interval: how far a degraded stratum could have moved this
//     row, in both directions, and which prefix of the answer is certain.
// ---------------------------------------------------------------------------

/// The interval's two terms, or a panic naming what arrived instead.
fn bounds(row: &FusedRow) -> (Fixed, Fixed) {
    match &row.interval {
        ScoreInterval::Bounded { deficit, inflation } => (*deficit, *inflation),
        ScoreInterval::Unbounded { perturbed } => {
            panic!("expected a bounded interval, got unbounded over {perturbed:?}")
        }
    }
}

/// A contract declaring an order a consumer cannot bound: a producer comparing
/// approximated values, which can rank a row it found BETTER than it was due.
fn perturbed_contract() -> StreamContract {
    StreamContract::new(
        DuplicatePolicy::Unique,
        RankFidelity {
            completeness: Completeness::Lossy {
                evidence: Arc::from("approximate: beam search"),
            },
            order: OrderFidelity::Perturbed {
                evidence: Arc::from("quantized: distances compared in 8-bit space"),
            },
        },
        CandidateDomains::Unrestricted,
        ExclusionBasis::Unavailable,
    )
}

// T9.1. THE SOUNDNESS TEST. A row a lossy stratum NAMED carries a non-zero
// inflation, because a missing row ahead of it promoted it into a rank it did
// not earn. A one-sided implementation returns zero here and fails.
#[test]
fn a_row_a_lossy_stratum_named_carries_an_inflation_term() {
    let profile = profile(&[("vector", Fixed::ONE)], K);
    let fused = block_on(run_fuse(
        vec![(
            stratum("vector"),
            MockStream::new(
                vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "b")],
                ProducerReceipt::Exhausted { rows_emitted: 2 },
            )
            .declaring(lossy_contract()),
        )],
        &profile,
    ));

    for emitted in &fused.rows {
        let (_, inflation) = bounds(emitted);
        assert!(
            inflation > Fixed::ZERO,
            "reciprocal-rank fusion scores by RANK, so a row the lossy stratum \
             named may have been promoted by a row it missed -- its whole \
             contribution is therefore suspect, and calling the score a mere \
             lower bound would be wrong in the one direction the name forbids \
             looking; got {inflation:?} for {:?}",
            emitted.entity
        );
        assert_eq!(
            inflation, emitted.score,
            "one stratum served this row, and all of its contribution is the \
             part that could be spurious"
        );
    }
}

// T9.2. The neighbour: an exhaustive fusion has zero-width intervals on every
// row, so an answer over undegraded producers reads exactly as it always did.
#[test]
fn an_exhaustive_fusion_carries_zero_width_intervals() {
    let profile = profile(&[("text", Fixed::ONE)], K);
    let fused = block_on(run_fuse(
        vec![(
            stratum("text"),
            MockStream::new(
                vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "b")],
                ProducerReceipt::Exhausted { rows_emitted: 2 },
            ),
        )],
        &profile,
    ));

    for emitted in &fused.rows {
        assert_eq!(
            bounds(emitted),
            (Fixed::ZERO, Fixed::ZERO),
            "nothing was withheld and nothing was promoted"
        );
    }
    assert_eq!(
        fused.trailer.unemitted_ceiling,
        Some(Fixed::ZERO),
        "every stream ran out and none of them could have missed a row, so \
         there is nothing outside this answer worth anything at all"
    );
    assert_eq!(
        fused.trailer.certain_prefix(&fused.rows),
        fused.rows.len(),
        "so the whole answer is certain, which is what this engine always said"
    );
}

// T9.3. The `Dom(x)` narrowing. A lossy stratum that PROVABLY cannot name a
// candidate withheld nothing from it, so it must not be charged. An aggregate
// per-stratum bound has no way to express this and over-charges here.
#[test]
fn a_stratum_that_cannot_name_a_candidate_is_not_charged_for_it() {
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    let docs = within(&["documents"]);
    let people = within(&["people"]);

    let fused = block_on(run_fuse(
        vec![
            (
                stratum("text"),
                MockStream::new(
                    // The row names the block its stream declared: a restricted
                    // declaration is backed row by row, and an unbacked one is
                    // refused rather than believed.
                    vec![row_in(1, Fixed::ONE, K, "a", "documents")],
                    ProducerReceipt::Exhausted { rows_emitted: 1 },
                )
                .declaring(StreamContract::new(
                    DuplicatePolicy::Unique,
                    RankFidelity::EXACT,
                    docs,
                    ExclusionBasis::Unavailable,
                )),
            ),
            (
                // Lossy, and declared over a DISJOINT block: it could never have
                // named the text stratum's candidate, however much it missed.
                stratum("vector"),
                MockStream::new(
                    vec![row_in(1, Fixed::ONE, K, "z", "people")],
                    ProducerReceipt::Exhausted { rows_emitted: 1 },
                )
                .declaring(StreamContract::new(
                    DuplicatePolicy::Unique,
                    RankFidelity {
                        completeness: Completeness::Lossy {
                            evidence: Arc::from("approximate: beam search"),
                        },
                        order: OrderFidelity::Faithful,
                    },
                    people,
                    ExclusionBasis::Unavailable,
                )),
            ),
        ],
        &profile,
    ));

    let from_text = fused
        .rows
        .iter()
        .find(|emitted| emitted.entity == Term::new("a"))
        .expect("the text stratum's row is in the answer");
    let (deficit, inflation) = bounds(from_text);
    assert_eq!(
        deficit,
        Fixed::ZERO,
        "the lossy stratum's declaration proves it could not have named this \
         candidate, so it withheld nothing from it -- charging it anyway would \
         bound the answer by a contribution that was never possible"
    );
    assert_eq!(
        inflation,
        Fixed::ZERO,
        "and it did not name it, so it promoted nothing either"
    );
}

// T9.4. A perturbed order removes the bound entirely, and says so by naming the
// strata rather than by inventing a number.
#[test]
fn a_perturbed_order_leaves_no_finite_bound() {
    let profile = profile(&[("vector", Fixed::ONE)], K);
    let fused = block_on(run_fuse(
        vec![(
            stratum("vector"),
            MockStream::new(
                vec![row(1, Fixed::ONE, K, "a")],
                ProducerReceipt::Exhausted { rows_emitted: 1 },
            )
            .declaring(perturbed_contract()),
        )],
        &profile,
    ));

    assert_eq!(
        fused.rows[0].interval,
        ScoreInterval::Unbounded {
            perturbed: BTreeSet::from([stratum("vector")]),
        },
        "the inequality every bound rests on -- a named row's true rank is at \
         least its emitted rank -- does not hold for a producer comparing \
         approximated values, so there is no number to report and none is made up"
    );
    let ScoreExactness::Estimated { ref unbounded, .. } = fused.trailer.exactness else {
        panic!("a perturbed stratum makes the answer an estimate");
    };
    assert_eq!(unbounded, &BTreeSet::from([stratum("vector")]));
    assert_eq!(
        fused.trailer.certain_prefix(&fused.rows),
        0,
        "an unbounded row could outscore anything, so nothing is certain of \
         outranking it"
    );
}

// T9.5. `certain_prefix`, both directions. A well-separated answer has a
// non-empty certain prefix; widening the loss until the intervals overlap
// shrinks it. Asserting only the first direction would pass against a function
// that always returned the whole length.
#[test]
fn the_certain_prefix_shrinks_as_the_declared_loss_widens() {
    // Two strata. The leader is named by both; the runner-up by one. With the
    // second stratum exhaustive the gap is clean.
    let separated = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    let clean = block_on(run_fuse(
        vec![
            (
                stratum("text"),
                MockStream::new(
                    vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "b")],
                    ProducerReceipt::Exhausted { rows_emitted: 2 },
                ),
            ),
            (
                stratum("vector"),
                MockStream::new(
                    vec![row(1, Fixed::ONE, K, "a")],
                    ProducerReceipt::Exhausted { rows_emitted: 1 },
                ),
            ),
        ],
        &separated,
    ));
    assert_eq!(
        clean.trailer.certain_prefix(&clean.rows),
        clean.rows.len(),
        "every stratum exhaustive, so every row keeps its place whatever else \
         is true"
    );

    // The same shape, with the second stratum declaring a lossy search. Now the
    // runner-up could have been withheld from, and the leader could have been
    // promoted, so their intervals overlap and neither place is certain.
    let degraded = block_on(run_fuse(
        vec![
            (
                stratum("text"),
                MockStream::new(
                    vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "b")],
                    ProducerReceipt::Exhausted { rows_emitted: 2 },
                ),
            ),
            (
                stratum("vector"),
                MockStream::new(
                    vec![row(1, Fixed::ONE, K, "a")],
                    ProducerReceipt::Exhausted { rows_emitted: 1 },
                )
                .declaring(lossy_contract()),
            ),
        ],
        &separated,
    ));
    assert!(
        degraded.trailer.certain_prefix(&degraded.rows) < degraded.rows.len(),
        "a declared loss must be able to SHORTEN the certain prefix, or the \
         function is reporting a constant"
    );
    assert_eq!(
        degraded
            .rows
            .iter()
            .map(|r| r.entity.clone())
            .collect::<Vec<_>>(),
        clean
            .rows
            .iter()
            .map(|r| r.entity.clone())
            .collect::<Vec<_>>(),
        "and the ROWS are identical either way: a declaration changes what the \
         answer may be read to claim, never what the answer is"
    );
}

// T9.6. The interval is reporting, not certification. Two fusions differing
// only in a fidelity declaration emit byte-identical rows, scores and
// provenance -- the decay-tie law, the threshold and the top-k bound are read
// here and never written.
#[test]
fn a_fidelity_declaration_changes_no_certified_row() {
    let profile = profile(&[("vector", Fixed::ONE)], K);
    let script = || {
        vec![
            row(1, Fixed::ONE, K, "a"),
            row(2, Fixed::ONE, K, "b"),
            row(3, Fixed::ONE, K, "c"),
        ]
    };
    let ranking = |result: &FusionResult<Term>| {
        result
            .rows
            .iter()
            .map(|r| {
                (
                    r.entity.clone(),
                    r.score,
                    r.threshold_witness,
                    r.contributions.clone(),
                )
            })
            .collect::<Vec<_>>()
    };

    let exact = block_on(run_fuse(
        vec![(
            stratum("vector"),
            MockStream::new(script(), ProducerReceipt::Exhausted { rows_emitted: 3 }),
        )],
        &profile,
    ));
    let lossy = block_on(run_fuse(
        vec![(
            stratum("vector"),
            MockStream::new(script(), ProducerReceipt::Exhausted { rows_emitted: 3 })
                .declaring(lossy_contract()),
        )],
        &profile,
    ));

    assert_eq!(
        ranking(&exact),
        ranking(&lossy),
        "the certified order, every score and every threshold witness are \
         identical: what a producer declares about itself is read into the \
         answer's claims and never into its arithmetic"
    );
    assert_eq!(
        exact.trailer.statuses, lossy.trailer.statuses,
        "and so are the read endings"
    );
}

// T9.7. An open exhaustive stream contributes only its head to the deficit --
// what it has not yet offered -- while a lossy one contributes its rank-one
// contribution, because a row it never found could have been due at any rank.
#[test]
fn an_open_stream_is_charged_its_head_and_a_lossy_one_its_first_rank() {
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    let rank_one = contribution(Fixed::ONE, 1, K).expect("fixture contribution");

    // `b` is named only by `text`. `vector` is exhausted having named only `a`,
    // so for `b` it is a closed stream that never named it.
    let fused = block_on(run_fuse(
        vec![
            (
                stratum("text"),
                MockStream::new(
                    vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "b")],
                    ProducerReceipt::Exhausted { rows_emitted: 2 },
                ),
            ),
            (
                stratum("vector"),
                MockStream::new(
                    vec![row(1, Fixed::ONE, K, "a")],
                    ProducerReceipt::Exhausted { rows_emitted: 1 },
                )
                .declaring(lossy_contract()),
            ),
        ],
        &profile,
    ));

    let runner_up = fused
        .rows
        .iter()
        .find(|emitted| emitted.entity == Term::new("b"))
        .expect("the runner-up is in the answer");
    let (deficit, inflation) = bounds(runner_up);
    assert_eq!(
        deficit, rank_one,
        "the lossy stratum closed without naming this row, and a row it never \
         found could have been due at rank one -- so what it could have \
         withheld is the rank-one contribution, not zero and not a head it no \
         longer has"
    );
    assert_eq!(
        inflation,
        Fixed::ZERO,
        "but it did not name this row, so it promoted nothing here"
    );
}

// T9.8. Tied scores with zero uncertainty are still certain.
//
// Found by the Python suite, whose fixture ties where the Rust fixtures above
// happen not to. Fixed-point reciprocal-rank decay quantizes adjacent ranks to
// one value routinely, so a tie inside an exhaustive answer is the ordinary
// case, not a corner. A strict comparison reported a certain prefix ending at
// the tie -- claiming doubt about an answer in which nothing was degraded and
// nothing could have been different.
#[test]
fn tied_rows_in_an_exhaustive_answer_are_all_certain() {
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    // `a` is named by both strata and leads. `b` and `c` are named once each at
    // the same rank, so they carry identical scores.
    let fused = block_on(run_fuse(
        vec![
            (
                stratum("text"),
                MockStream::new(
                    vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "b")],
                    ProducerReceipt::Exhausted { rows_emitted: 2 },
                ),
            ),
            (
                stratum("vector"),
                MockStream::new(
                    vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "c")],
                    ProducerReceipt::Exhausted { rows_emitted: 2 },
                ),
            ),
        ],
        &profile,
    ));

    let scores: Vec<Fixed> = fused.rows.iter().map(|r| r.score).collect();
    assert!(
        scores.windows(2).any(|pair| pair[0] == pair[1]),
        "the fixture must actually tie, or this test proves nothing: {scores:?}"
    );
    assert_eq!(
        fused.trailer.certain_prefix(&fused.rows),
        fused.rows.len(),
        "every interval is zero-width, so nothing could have been different and \
         every row keeps its place -- a tie is decided by the engine's own total \
         tie-break, which is not uncertainty about the answer"
    );
}

// T9.9. The same tie under a declared loss is NOT certain, which is what makes
// the test above a statement about uncertainty rather than a rubber stamp.
#[test]
fn tied_rows_stop_being_certain_once_a_stratum_declares_a_loss() {
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    let fused = block_on(run_fuse(
        vec![
            (
                stratum("text"),
                MockStream::new(
                    vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "b")],
                    ProducerReceipt::Exhausted { rows_emitted: 2 },
                ),
            ),
            (
                stratum("vector"),
                MockStream::new(
                    vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "c")],
                    ProducerReceipt::Exhausted { rows_emitted: 2 },
                )
                .declaring(lossy_contract()),
            ),
        ],
        &profile,
    ));

    assert!(
        fused.trailer.certain_prefix(&fused.rows) < fused.rows.len(),
        "now the tied rows could have been ordered differently, because the \
         lossy stratum may have withheld from one and promoted the other"
    );
}

/// The prefix a comparison among the rows in hand alone would report.
///
/// The term `certain_prefix` used to be the whole of, kept here as the thing it
/// is measured against: it can only ever say that the emitted rows are ordered
/// consistently with each other, which is a statement about a list and not
/// about membership of an answer.
fn pairwise_only_prefix(rows: &[FusedRow]) -> usize {
    let mut certain = 0;
    for (position, row) in rows.iter().enumerate() {
        let ScoreInterval::Bounded { inflation, .. } = &row.interval else {
            break;
        };
        let mine = row.score.checked_sub(*inflation).expect("a floor fits");
        let settled = rows[position + 1..].iter().all(|rival| {
            matches!(
                &rival.interval,
                ScoreInterval::Bounded { deficit, .. }
                    if mine >= rival.score.checked_add(*deficit).expect("a ceiling fits")
            )
        });
        if !settled {
            break;
        }
        certain = position + 1;
    }
    certain
}

// T9.10. THE MEMBERSHIP TERM. A candidate a lossy stratum never named is
// exactly the candidate that can take an emitted row's place, and no comparison
// among the rows in hand can see it: it is not among them. This fixture is the
// smallest one in which that is provable rather than merely conceivable, and a
// prefix computed from the rows alone reports the opposite answer on it.
#[test]
fn a_candidate_the_lossy_stratum_never_named_can_unseat_an_emitted_row() {
    // `vector` outweighs `text` two to one, so the only emitted row is mostly
    // made of a contribution the declaration says could be spurious -- and a
    // candidate `vector` never found could have held its rank one, which is
    // worth more than everything `text` has to give.
    let heavy = Fixed::ONE.checked_add(Fixed::ONE).expect("two fits");
    let fusion_profile = profile(&[("text", Fixed::ONE), ("vector", heavy)], K);
    let streams = |lossy: bool| {
        let vector = MockStream::new(vec![row(1, heavy, K, "a")], exhausted(1));
        vec![
            (
                stratum("text"),
                MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
            ),
            (
                stratum("vector"),
                if lossy {
                    vector.declaring(lossy_contract())
                } else {
                    vector
                },
            ),
        ]
    };

    let exhaustive = block_on(run_fuse(streams(false), &fusion_profile));
    assert_eq!(
        exhaustive.trailer.certain_prefix(&exhaustive.rows),
        exhaustive.rows.len(),
        "with both strata exhaustive there is nothing outside the answer: a \
         stratum that found everything it had cannot have hidden a rival"
    );

    let degraded = block_on(run_fuse(streams(true), &fusion_profile));
    assert_eq!(
        degraded.rows.len(),
        exhaustive.rows.len(),
        "the same rows either way -- a declaration changes what the answer may \
         be read to claim, never what the answer is"
    );

    // The numbers the claim rests on, from the profile rather than from the
    // output: the emitted row's floor is what `text` alone gave it, and the
    // absentee's ceiling is `vector`'s rank one.
    let from_text = contribution(Fixed::ONE, 1, K).expect("fixture contribution");
    let held_back = contribution(heavy, 1, K).expect("fixture contribution");
    assert!(
        held_back > from_text,
        "the fixture must put more at stake in the lossy stratum than the \
         exhaustive one can defend, or it proves nothing: {held_back:?} vs \
         {from_text:?}"
    );
    assert_eq!(
        degraded.trailer.unemitted_ceiling,
        Some(held_back),
        "every stream is exhausted, so nothing is still on offer; what remains \
         is the rank-one contribution the lossy stratum could have awarded a \
         row it never emitted"
    );
    let (_, inflation) = bounds(&degraded.rows[0]);
    assert_eq!(
        degraded.rows[0]
            .score
            .checked_sub(inflation)
            .expect("a floor fits"),
        from_text,
        "and the emitted row's floor is what the exhaustive stratum gave it, \
         the rest of its score being the part that could be spurious"
    );

    assert_eq!(
        pairwise_only_prefix(&degraded.rows),
        degraded.rows.len(),
        "the rows in hand are consistent with each other -- there is only one \
         -- so a prefix computed from them alone certifies the whole answer"
    );
    assert_eq!(
        degraded.trailer.certain_prefix(&degraded.rows),
        0,
        "but the answer is not certain: a candidate the lossy stratum never \
         named could have scored the whole of its rank one, which beats \
         everything the emitted row can prove it is worth"
    );
}

// T9.11. The same defect from the other side. A bounded read leaves candidates
// it named and never certified, and one of those can be lifted past an emitted
// row by the loss too -- so the unemitted term is a bound over the frontier as
// well as over the candidates nobody ever named. The threshold cannot answer
// for these: it bounds what an *unseen* item could still collect, and these are
// not unseen.
#[test]
fn a_rival_left_in_the_frontier_by_the_bound_is_bounded_too() {
    // `vector` is light, so it promotes the leader by very little -- and the
    // runner-up it never named lost by less than that little.
    let light = Fixed::from_raw(20_000_000_000);
    let fusion_profile = profile(&[("text", Fixed::ONE), ("vector", light)], K);
    let leader = contribution(Fixed::ONE, 1, K).expect("fixture contribution");
    let runner_up = contribution(Fixed::ONE, 2, K).expect("fixture contribution");
    let held_back = contribution(light, 1, K).expect("fixture contribution");
    assert!(
        runner_up.checked_add(held_back).expect("a ceiling fits") > leader,
        "the fixture must let the abandoned rival overtake the emitted row, or \
         it proves nothing: {runner_up:?} + {held_back:?} vs {leader:?}"
    );

    let streams = || {
        vec![
            (
                stratum("text"),
                MockStream::new(
                    vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "b")],
                    exhausted(2),
                ),
            ),
            (
                stratum("vector"),
                MockStream::new(vec![row(1, light, K, "a")], exhausted(1))
                    .declaring(lossy_contract()),
            ),
        ]
    };
    let fused = block_on(purrdf_retrieval::fuse::<MockStream, Term>(
        streams(),
        &fusion_profile,
        TopK::new(1),
    ))
    .expect("fusion succeeds");

    assert_eq!(
        fused.rows.len(),
        1,
        "the bound stopped the answer at one row"
    );
    assert_eq!(
        fused.rows[0].entity,
        Term::new("a"),
        "and the leader is the row it kept"
    );
    assert_eq!(
        fused.trailer.unemitted_ceiling,
        Some(
            runner_up
                .checked_add(held_back)
                .expect("the fixture's ceiling fits")
        ),
        "the rival the bound abandoned had already collected the exhaustive \
         stratum's second rank, and the lossy stratum could have owed it a \
         rank one on top of that"
    );
    assert_eq!(
        pairwise_only_prefix(&fused.rows),
        fused.rows.len(),
        "nothing in the answer contradicts anything else in it"
    );
    assert_eq!(
        fused.trailer.certain_prefix(&fused.rows),
        0,
        "yet the abandoned rival could outscore the row that was kept, so the \
         place is not settled -- a bounded read is where this is decided, and \
         it is the ordinary case rather than a corner"
    );
}

// T9.12. The linear pass is the quadratic one. `certain_prefix` runs on the
// default path of every fused search, so it carries the suffix maximum
// backwards instead of walking the suffix per row -- and a rewrite of a
// published number has to be measured against the definition it replaced, over
// intervals no scripted producer would ever generate.
#[test]
fn the_backward_pass_agrees_with_the_quadratic_definition() {
    // The trailer is real -- a degraded fusion, so no exhaustive short-circuit
    // applies -- and only the two quantities under test are substituted:
    // arbitrary rows, and the ceiling on what the answer does not contain.
    let fusion_profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    let fused = block_on(run_fuse(
        vec![
            (
                stratum("text"),
                MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
            ),
            (
                stratum("vector"),
                MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1))
                    .declaring(lossy_contract()),
            ),
        ],
        &fusion_profile,
    ));
    assert!(
        matches!(fused.trailer.exactness, ScoreExactness::Estimated { .. }),
        "the fixture trailer must be an estimate, or the exhaustive \
         short-circuit answers instead of the pass under test"
    );

    /// The definition, spelled as the suffix scan it is: a row is certain when
    /// its floor reaches every later row's ceiling and the ceiling on what the
    /// answer does not contain. Quadratic, and here for exactly that reason.
    fn reference(rows: &[FusedRow], unemitted: Fixed) -> usize {
        let floor = |row: &FusedRow| match &row.interval {
            ScoreInterval::Bounded { inflation, .. } => {
                row.score.checked_sub(*inflation).unwrap_or(Fixed::ZERO)
            }
            ScoreInterval::Unbounded { .. } => Fixed::ZERO,
        };
        let ceiling = |row: &FusedRow| match &row.interval {
            ScoreInterval::Bounded { deficit, .. } => row.score.checked_add(*deficit).ok(),
            ScoreInterval::Unbounded { .. } => None,
        };
        let mut certain = 0;
        for (position, row) in rows.iter().enumerate() {
            let mine = floor(row);
            let settled = mine >= unemitted
                && rows[position + 1..]
                    .iter()
                    .all(|rival| ceiling(rival).is_some_and(|reach| mine >= reach));
            if !settled {
                break;
            }
            certain = position + 1;
        }
        certain
    }

    // A fixed-seed xorshift, because a test that cannot be replayed from its
    // failure message is not evidence. The values are small enough that no sum
    // can overflow, so every case exercises the comparison rather than the
    // arithmetic's refusal.
    let mut state: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = |bound: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state % bound
    };

    let mut saw_empty = 0_u32;
    let mut saw_partial = 0_u32;
    let mut saw_whole = 0_u32;
    for _ in 0..2_000 {
        let length = usize::try_from(next(8)).expect("a small length fits");
        let rows: Vec<FusedRow> = (0..length)
            .map(|index| {
                let score = next(40);
                let deficit = next(6);
                // An inflation is a sum of contributions that are themselves
                // summands of the score, so it never exceeds it -- the
                // invariant the floor is computed under.
                let inflation = next(score + 1);
                FusedRow {
                    entity: Term::new(format!("item-{index}")),
                    score: Fixed::from_raw(i128::from(score)),
                    contributions: Vec::new(),
                    interval: ScoreInterval::Bounded {
                        deficit: Fixed::from_raw(i128::from(deficit)),
                        inflation: Fixed::from_raw(i128::from(inflation)),
                    },
                    threshold_witness: Fixed::ZERO,
                }
            })
            .collect();
        let unemitted = Fixed::from_raw(i128::from(next(12)));

        let mut trailer = fused.trailer.clone();
        trailer.unemitted_ceiling = Some(unemitted);
        let got = trailer.certain_prefix(&rows);
        let want = reference(&rows, unemitted);
        assert_eq!(
            got, want,
            "the backward pass and the definition disagree on {rows:?} under \
             an unemitted ceiling of {unemitted:?}"
        );

        if got == 0 {
            saw_empty += 1;
        } else if got == rows.len() {
            saw_whole += 1;
        } else {
            saw_partial += 1;
        }
    }
    assert!(
        saw_empty > 0 && saw_partial > 0 && saw_whole > 0,
        "the cases must cover all three outcomes, or agreement proves only \
         that both functions return the same constant: {saw_empty} empty, \
         {saw_partial} partial, {saw_whole} whole"
    );
}

// ---------------------------------------------------------------------------
// T10. What `Exact` claims, what a refusal does instead of reporting a zero,
//      and where the emitted order and the plan identity are read from.
//
//      Each of the four below pins a law that an otherwise-green rewrite can
//      break silently, because breaking it produces a *plausible* answer: a
//      stricter exactness verdict, a zero bound where the arithmetic gave out,
//      an order that is consistently wrong, and an identity that stops moving
//      when a declaration does.
// ---------------------------------------------------------------------------

// T10.1. THE LAW: `Exact` says *no stratum declared itself degraded*, and it
// says nothing wider. A stratum the caller's depth STOPPED is not a stratum
// that failed to name a row it could see -- it stated, in its own status, the
// rank it read to and stopped. Reading any shortfall as degradation is the
// tempting-but-wrong generalisation, and it would make `Exact` unreachable for
// every bounded read: the verdict would become a paraphrase of "nothing was
// bounded", and a consumer could no longer tell a depth it chose from an
// approximation it did not.
//
// It can regress from either side. A rewrite that folds `DepthReached` into the
// `deficit` set turns every depth-bounded read into an estimate; one that
// derives exactness from the statuses rather than from the declarations does
// the same by a different route. This is the case that goes red for both.
#[test]
fn a_depth_bounded_exact_stratum_is_still_exact() {
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    let fused = block_on(run_fuse(
        vec![
            (
                stratum("text"),
                MockStream::new(
                    vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "b")],
                    ProducerReceipt::DepthReached { rank: 2 },
                ),
            ),
            (
                stratum("vector"),
                MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
            ),
        ],
        &profile,
    ));

    // The precondition: the stratum really did stop at a depth, so this is not
    // an exhaustive fusion wearing a bounded label.
    assert_eq!(
        fused.trailer.statuses.get(&stratum("text")),
        Some(&ProducerStatus::DepthReached { rank: 2 }),
        "the fixture must actually reach the depth-bounded ending"
    );
    assert_eq!(
        fused.trailer.exactness,
        ScoreExactness::Exact,
        "a stratum that was STOPPED has not declared itself degraded: what it \
         left unread is already stated in its own status, in the rank space the \
         producer was handed, and repeating it as an unnamed score error would \
         charge the answer twice for one fact"
    );
    // And the disclosure channel is not merely empty -- it is present and says
    // the strongest thing it can, which is what a consumer reads.
    assert_eq!(
        fused.trailer.fidelities[&stratum("text")],
        RankFidelity::EXACT
    );
}

// T10.2. The same law from its farthest edge: a producer that could not run at
// all is still `Exact`. It emitted nothing, so it withheld nothing it had; what
// it did not do is recorded, by name and with its own words, in
// `ProducerStatus::ExecutionFailed`. Calling the answer an *estimate* here would
// claim a score error nobody can bound and would hide a hard failure behind a
// soft word -- the fused rows are exactly the sum of what the strata that ran
// contributed, and that is the narrow true claim.
#[test]
fn a_failed_stratum_is_reported_by_name_and_leaves_the_scores_exact() {
    const REASON: &str = "index unavailable: the segment generation was reaped mid-read";
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    let fused = block_on(run_fuse(
        vec![
            (
                stratum("text"),
                MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1)),
            ),
            (
                stratum("vector"),
                MockStream::new(
                    Vec::new(),
                    ProducerReceipt::ExecutionFailed {
                        reason: REASON.to_owned(),
                    },
                ),
            ),
        ],
        &profile,
    ));

    assert_eq!(
        fused.trailer.statuses.get(&stratum("vector")),
        Some(&ProducerStatus::ExecutionFailed {
            reason: REASON.to_owned(),
        }),
        "the failure is reported where a failure belongs, verbatim"
    );
    assert_eq!(
        fused.trailer.exactness,
        ScoreExactness::Exact,
        "a stratum that never ran declared no loss: the rows are the exact sum \
         of the contributions that arrived, and the one that did not arrive is \
         named in the status rather than smeared into an unbounded estimate"
    );
    // The neighbour that must still move it, so this is not the verdict being
    // stuck at `Exact`: the same fusion with the same failure and one declared
    // loss is an estimate.
    let estimated = block_on(run_fuse(
        vec![
            (
                stratum("text"),
                MockStream::new(vec![row(1, Fixed::ONE, K, "a")], exhausted(1))
                    .declaring(lossy_contract()),
            ),
            (
                stratum("vector"),
                MockStream::new(
                    Vec::new(),
                    ProducerReceipt::ExecutionFailed {
                        reason: REASON.to_owned(),
                    },
                ),
            ),
        ],
        &profile,
    ));
    assert!(
        matches!(
            estimated.trailer.exactness,
            ScoreExactness::Estimated { .. }
        ),
        "a declared loss still moves the verdict; only the failure does not"
    );
}

// T10.3. THE HARD-FAILS PROPERTY, executed. `score_interval` documents that a
// refusal is propagated and NEVER rendered as a zero bound -- reporting
// `Fixed::ZERO` where the arithmetic gave out would say "nothing was withheld"
// at exactly the moment nothing is known, which is a bound on the read silently
// becoming a value. Nothing executed it, so a rewrite that answered
// `.unwrap_or(Fixed::ZERO)` on either summand would have passed every test in
// this file while publishing a certain-looking answer built on an overflow.
//
// # The shape, and why it is the one available
//
// The deficit charges every degraded stratum that could still have named the
// row its RANK-ONE contribution, because a row it never found could have been
// due at any rank. Three such charges at a weight whose own ceiling is the whole
// fixed-point range leave that range on the third addition.
//
// A repeated stratum is what makes three charges reachable: `fuse` refuses one
// before a row is read, so this drives `FusionStream` directly, exactly as
// `checked_addition_overflow_is_refused` above does. It has to. `contribution_under`
// at rank one cannot itself refuse under any profile `FusionProfile::with_decay`
// admits -- the reciprocal at rank one is at most one half, so a rank-one
// contribution is at most its own weight, and the derived ceiling
// (`max_weight x stratum count`) already had to fit. The refusal is therefore in
// the SUM, which is the summand `score_interval` guards, and it is reached by
// charging one stratum's weight more than once.
//
// The degraded streams are EMPTY on purpose. A degraded stream holding a head
// would have its contribution counted by `compute_threshold` first, and the
// refusal under test would be pre-empted by a different one, several frames
// earlier, in a function this test is not about.
#[test]
fn an_interval_that_cannot_be_computed_is_refused_and_never_reported_as_zero() {
    let huge = Fixed::from_raw(i128::MAX);
    let whole_range = profile(&[("only", huge)], 1);
    let only = stratum("only");
    let mut streams = vec![(
        only.clone(),
        MockStream::new(vec![row(1, huge, 1, "a")], exhausted(1)),
    )];
    for _ in 0..3 {
        streams.push((
            only.clone(),
            MockStream::new(Vec::new(), exhausted(0)).declaring(lossy_contract()),
        ));
    }
    let mut fusion = FusionStream::new(streams, whole_range);

    let refused = block_on(fusion.next());
    assert!(
        matches!(refused, Err(FusionError::Overflow)),
        "the interval's arithmetic left the fixed-point range, so there is no \
         bound to report and the refusal is the answer; got {refused:?}"
    );

    // The neighbouring case that must still SUCCEED, and must succeed with a
    // real number. Over-refusal is the mirror of the fault above: a rewrite that
    // refused every multiply-charged deficit would pass the assertion above and
    // be wrong about every fusion that fits.
    let large = Fixed::from_raw(i128::MAX / 8);
    let narrow = profile(&[("only", large)], 1);
    let mut streams = vec![(
        only.clone(),
        MockStream::new(vec![row(1, large, 1, "a")], exhausted(1)),
    )];
    for _ in 0..3 {
        streams.push((
            only.clone(),
            MockStream::new(Vec::new(), exhausted(0)).declaring(lossy_contract()),
        ));
    }
    let mut fusion = FusionStream::new(streams, narrow);
    let emitted = block_on(fusion.next())
        .expect("three rank-one charges at an eighth of the range still sum")
        .expect("the fusion certifies its one candidate");
    let rank_one = contribution_under(DecayRule::ReciprocalRank { k: 1 }, large, 1)
        .expect("the fixture rank-one contribution fits");
    let (deficit, _) = bounds(&emitted);
    assert_eq!(
        deficit,
        rank_one
            .checked_add(rank_one)
            .and_then(|sum| sum.checked_add(rank_one))
            .expect("three eighths of the range fits"),
        "each of the three lossy strata is charged its rank-one contribution, \
         and the sum is reported as the number it is"
    );
    assert!(
        deficit > Fixed::ZERO,
        "the bound the refusal above declined to invent is non-zero here, which \
         is what makes a zero in its place a lie rather than a coincidence"
    );
}

// T10.4. The emitted order against an EXPLICIT list, derived from the profile's
// own arithmetic rather than from a run.
//
// Every other ordering test in this file compares one fusion with another, or a
// fusion with an oracle assembled from the same helpers the engine uses. Both
// pass when the engine and its comparator are wrong the same way -- a decay rule
// that truncated one digit too many, a tie-break that reversed, a sum that
// dropped a stratum would move BOTH sides together. A literal is the only
// comparator that cannot move.
//
// # The arithmetic, spelled out
//
// A `Fixed` is a multiple of `10^-12`, and under `ReciprocalRank { k }` a
// unit-weighted row at rank `r` contributes exactly `trunc(10^12 / (k + r))`
// raw units. At `k = 60`:
//
// | rank | raw contribution |
// |---|---|
// | 1 | `10^12 / 61` = `16393442622` (`.95...` truncated) |
// | 2 | `10^12 / 62` = `16129032258` (`.06...` truncated) |
// | 3 | `10^12 / 63` = `15873015873` (`.01...` truncated) |
//
// The two strata below name `a b c` and `c a d`, so the fused scores are:
//
// * `a` = rank 1 in text + rank 2 in vector = `16393442622 + 16129032258` = `32522474880`
// * `c` = rank 3 in text + rank 1 in vector = `15873015873 + 16393442622` = `32266458495`
// * `b` = rank 2 in text alone = `16129032258`
// * `d` = rank 3 in vector alone = `15873015873`
//
// which is a strict descent, so the order is decided by score alone and no
// tie-break is consulted. `c` beating `b` is the whole point of fusing: `b` is
// ranked ABOVE `c` by the only stratum that named both, and the second stratum's
// agreement is what moves `c` past it.
#[test]
fn the_emitted_sequence_matches_an_explicit_expected_order() {
    let profile = profile(&[("text", Fixed::ONE), ("vector", Fixed::ONE)], K);
    let fused = block_on(run_fuse(
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
                    vec![
                        row(1, Fixed::ONE, K, "c"),
                        row(2, Fixed::ONE, K, "a"),
                        row(3, Fixed::ONE, K, "d"),
                    ],
                    exhausted(3),
                ),
            ),
        ],
        &profile,
    ));

    let emitted: Vec<(Term, Fixed)> = fused
        .rows
        .iter()
        .map(|emitted| (emitted.entity.clone(), emitted.score))
        .collect();
    let expected: Vec<(Term, Fixed)> = vec![
        (Term::new("a"), Fixed::from_raw(32_522_474_880)),
        (Term::new("c"), Fixed::from_raw(32_266_458_495)),
        (Term::new("b"), Fixed::from_raw(16_129_032_258)),
        (Term::new("d"), Fixed::from_raw(15_873_015_873)),
    ];
    assert_eq!(
        emitted, expected,
        "the answer is compared against numbers written down from the decay \
         rule, not against a second computation that could be wrong the same way"
    );
}

// T10.5. THE LAW: a producer's declared fidelity reaches the compiled bundle's
// `plan_id`. Two wirings that differ in NOTHING but one producer's fidelity must
// compile to two identities.
//
// This matters because a plan identity is what a caller caches against and what
// a host compares to decide "is this the same read?". Two registries that differ
// only here FUSE DIFFERENTLY -- one answer's scores are values and the other's
// are estimates with a reported interval -- so a shared identity would let a
// plan admitted against the exhaustive wiring be served, silently, by the
// approximate one. The route is declaration -> registry content fingerprint ->
// plan bytes -> `Plan::id` -> `CompiledRetrieval::plan_id`, and this walks all
// of it rather than asserting a link in the middle. Only the first hop is tested
// elsewhere (`canonical_description` injectivity, in `purrdf-sparql-eval`),
// which is precisely the hop that cannot show the identity moved.
//
// # The registries share an instance id, and that is what makes this sharp
//
// A plan records the registry's instance id as well as its content fingerprint,
// so two independently built registries produce two identities whatever they
// declare -- an assertion over such a pair would pass on a build that had
// dropped the fidelity from the digest entirely. Cloning one empty registry
// inherits the id rather than re-minting it, so the ONLY difference between the
// wirings below is the declaration under test.

/// A `Statistics` provider that measures nothing. The claim here is about a
/// declaration reaching an identity; a cardinality it did not report cannot be
/// what moved one.
struct SilentStatistics {
    /// Owned rather than a literal, because the trait hands back a borrow of
    /// the provider and a provider that reads its own source from configuration
    /// is the shape this stands in for.
    source: String,
    revision: String,
}

impl SilentStatistics {
    fn new() -> Self {
        Self {
            source: "fusion-fixture-statistics".to_owned(),
            revision: "r1".to_owned(),
        }
    }
}

impl Statistics for SilentStatistics {
    fn source(&self) -> &str {
        &self.source
    }

    fn revision(&self) -> &str {
        &self.revision
    }

    fn cardinality(&self, _predicate: &Iri) -> Option<u64> {
        None
    }

    fn selectivity_ppm(&self, _subject: &Iri, _term: &RequestTerm) -> Option<u64> {
        None
    }
}

/// `base`, with one ranked producer registered that declares `fidelity` and is
/// otherwise fixed. The registry is taken by value so its instance id -- cloned
/// from one empty registry by every caller -- is carried into the result.
fn registry_declaring_fidelity(
    mut base: PropertyFunctionRegistry,
    fidelity: RankFidelity,
) -> PropertyFunctionRegistry {
    base.register_ranked(
        "http://example.org/pf/ranked",
        Arc::new(MemoryRelation::new(1, 1, Vec::new()).expect("an empty table is uniform")),
        RankedDeclaration {
            stratum: purrdf_core::parse_iri("http://example.org/stratum/text")
                .expect("fixture IRI"),
            accepted_terms: vec![AcceptedTerm {
                pattern: TermPattern::of_kind(TermKind::Any),
                placements: Vec::new(),
            }],
            depth_placement: None,
            candidate_position: 0,
            duplicates: DuplicatePolicy::Unique,
            fidelity,
            domains: CandidateDomains::Unrestricted,
            block_position: None,
            exclusion: ExclusionBasis::Unavailable,
            mandatory: false,
        },
    );
    base
}

/// The identity `compile` publishes for a bundle built against `registry`.
fn compiled_plan_id(registry: &PropertyFunctionRegistry) -> PlanId {
    let statistics = SilentStatistics::new();
    let request = RetrievalRequest::bounded(
        vec![RequestTerm::Lexical {
            text: "quick brown".to_owned(),
            language: Some("en".to_owned()),
            predicate: Some(iri("http://example.org/p")),
        }],
        TopK::new(8),
    );
    let planned =
        purrdf_retrieval::plan(&request, registry, &statistics).expect("the fixture request plans");
    let compiled = purrdf_retrieval::compile(
        &planned,
        &AdmissionEnvironment {
            registry,
            statistics: &statistics,
            fusion_profile: None,
        },
    )
    .expect("a freshly planned bundle is admitted against the registry it was planned on");
    assert_eq!(
        compiled.plan_id,
        planned.id(),
        "the compiled bundle republishes the plan's own identity"
    );
    compiled.plan_id
}

#[test]
fn a_declared_fidelity_moves_the_compiled_plan_identity() {
    let base = PropertyFunctionRegistry::new();
    let exhaustive = registry_declaring_fidelity(base.clone(), RankFidelity::EXACT);
    let approximate = registry_declaring_fidelity(
        base.clone(),
        RankFidelity {
            completeness: Completeness::Lossy {
                evidence: Arc::from(LOSS),
            },
            order: OrderFidelity::Faithful,
        },
    );
    let twin = registry_declaring_fidelity(base.clone(), RankFidelity::EXACT);
    let other_reason = registry_declaring_fidelity(
        base,
        RankFidelity {
            completeness: Completeness::Lossy {
                evidence: Arc::from("approximate: sampled one segment in a thousand"),
            },
            order: OrderFidelity::Faithful,
        },
    );

    // The precondition: the wirings share an instance id, so nothing but the
    // declaration can move an identity below.
    assert_eq!(
        exhaustive.instance_id(),
        approximate.instance_id(),
        "the fixture registries must be clones of one, or every assertion here \
         passes on the instance id alone"
    );
    assert_ne!(
        exhaustive.content_fingerprint().expect("readable"),
        approximate.content_fingerprint().expect("readable"),
        "the declaration reaches the registry's durable fingerprint"
    );

    assert_ne!(
        compiled_plan_id(&exhaustive),
        compiled_plan_id(&approximate),
        "and through it the compiled bundle's identity: a plan cached against \
         the exhaustive wiring must not be served by the approximate one, whose \
         every score is an estimate"
    );

    // The valid neighbour, and the property that makes the identity usable at
    // all: two wirings declaring the SAME fidelity compile to the same
    // identity. An id that moved between identical wirings would invalidate
    // every cached plan on every rebuild.
    assert_eq!(
        compiled_plan_id(&exhaustive),
        compiled_plan_id(&twin),
        "the identity is a function of what was declared, not of when it was \
         declared"
    );

    // And the producer's own words are part of the declaration, not merely the
    // fact that a loss exists: two lossy wirings that differ only in the reason
    // they publish are two identities. A digest that recorded "lossy" and
    // dropped the evidence would pass every assertion above and fail here.
    assert_ne!(
        compiled_plan_id(&approximate),
        compiled_plan_id(&other_reason),
        "the evidence a producer publishes is part of what it declared"
    );
}
