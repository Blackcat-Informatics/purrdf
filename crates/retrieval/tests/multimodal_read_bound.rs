// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! How much of a ranked stream a bounded multimodal read actually consumes.
//!
//! Three configurations of the *same* two producers run through the whole
//! ladder — `plan` → `compile` → `execute` → `fuse`. They differ in exactly two
//! bits: whether the two producers declare the same block or different ones,
//! and whether their candidate sets intersect. Everything else — four hundred
//! rows each, unit weights, a bound of five, `DuplicatePolicy::Unique` — is
//! held fixed, so every number that moves between them moved because of those
//! two bits and nothing else.
//!
//! Five numbers are measured per configuration: the depth the planner recorded,
//! the `LIMIT` the compiler emitted, the rows the executor really pulled out of
//! the relation, the ranks fusion really pulled off the stream, and the
//! terminal status the trailer reports. They are asserted as exact literals,
//! because a range would hide the very drift this file exists to detect.
//!
//! Beside them sits a derivation with no executor in it: the rank at which a
//! candidate's lower bound overtakes the fusion threshold, computed from the
//! profile's own decay law through the crate's own `contribution_under`. It
//! predicts the two configurations that reach that gate, and that prediction is
//! asserted. The third never reaches it — a candidate one stream named is not
//! *final* while another stream sharing its block is still open — which is why
//! that configuration drains, and the drain is pinned as a permanent control.
//!
//! Fixtures use `example.org` throughout; every block tag and stratum IRI below
//! is fixture configuration, never a minted vocabulary.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll, Wake, Waker};

use pretty_assertions::assert_eq;
use purrdf_core::{RdfDataset, TermValue};
use purrdf_retrieval::{
    AdmissionEnvironment, CandidateDomains, DecayRule, DomainTag, DuplicatePolicy, Fixed, FusedRow,
    FusionProfile, FusionResult, Iri, PfAttestation, PlanId, ProducerReceipt, ProducerStatus,
    ProtocolError, RECIP_K, RankFidelity, RankedRow, RankedStream, RankedStreamAdapter,
    RequestTerm, RetrievalRequest, RowBlock, Statistics, StratumUnit, StreamContract, Term, TopK,
    compile, contribution_under, execute, fuse, plan,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, EvalError, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction,
    PropertyFunctionRegistry, RankedDeclaration, RequestFacet, TermKind, TermPattern,
    TermPlacement, Volatility,
};

mod common;

/// The smoothing constant every profile in this file fuses under, read from the
/// crate's own constant rather than written twice.
const K: u32 = RECIP_K as u32;

/// The bound every configuration searches under.
///
/// Small on purpose: the whole question is how much of a four-hundred-row
/// stream a five-row answer has to read.
const TOP_K: TopK = TopK::new(5);

/// How many rows each producer holds, and the row bound each declares.
const ROWS: u64 = 400;

/// The deepest head rank [`crossing_rank_at`] will look for a crossing at.
///
/// Every decay rule this crate carries is non-increasing and reaches zero, so a
/// non-zero lower bound always crosses; the ceiling exists so a profile that
/// somehow does not crash the search rather than spinning.
const CROSSING_SEARCH_CEILING: u64 = 10_000;

fn ex(suffix: &str) -> String {
    format!("http://example.org/{suffix}")
}

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

fn kernel_iri(text: &str) -> purrdf_core::Iri {
    purrdf_core::parse_iri(text).expect("fixture IRIs are valid")
}

fn domain_tag(suffix: &str) -> DomainTag {
    DomainTag::parse(&ex(suffix)).expect("fixture domain tags are valid IRIs")
}

/// A minimal single-threaded executor. The mocks never actually pend.
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

// ---------------------------------------------------------------------------
// 1a. The threshold-crossing derivation: one pure function, no executor.
// ---------------------------------------------------------------------------

/// The sum of `weights`' contributions at one rank, under `decay`.
///
/// **One arithmetic path.** Every term comes out of the crate's own
/// [`contribution_under`]. The engine's contributions are truncated fixed point,
/// so `w / (k + r)` recomputed in floating point disagrees with it at exactly
/// the boundary this file is about — the rank where a doubled threshold ties a
/// single lower bound instead of falling below it.
fn sum_at(decay: DecayRule, weights: &[Fixed], rank: u64) -> Fixed {
    weights.iter().fold(Fixed::ZERO, |total, weight| {
        let term =
            contribution_under(decay, *weight, rank).expect("fixture contributions are in range");
        total
            .checked_add(term)
            .expect("fixture threshold sums are in range")
    })
}

/// The smallest head rank `r` at which a candidate sitting at `at_rank` in every
/// stratum that names it is emittable.
///
/// `select_emittable` runs two gates in order. The first is `is_final`, which
/// this function says nothing about. The second is the threshold, and this is
/// it: while any head is live a candidate must have
/// `lower_bound > threshold`, **strictly**.
///
/// * `naming` are the weights of the strata that named the candidate, so
///   `L = Σ contribution(w, at_rank)` is what it has already collected;
/// * `sharing` are the weights of the open strata whose declaration admits the
///   candidate's block, so `T(r) = Σ contribution(w, r)` is what
///   `compute_threshold` maximises to for that block once every head sits at
///   rank `r`.
///
/// A stratum that names the candidate is also a stratum that shares its block,
/// so `naming ⊆ sharing` for every fixture here; the two are separate arguments
/// because the gap between them is the whole phenomenon.
fn crossing_rank_at(decay: DecayRule, naming: &[Fixed], at_rank: u64, sharing: &[Fixed]) -> u64 {
    let lower = sum_at(decay, naming, at_rank);
    (1..=CROSSING_SEARCH_CEILING)
        .find(|rank| lower > sum_at(decay, sharing, *rank))
        .expect("a non-increasing decay always falls below a positive lower bound")
}

/// [`crossing_rank_at`] for the head of a stream: the candidate at rank one.
fn crossing_rank(decay: DecayRule, naming: &[Fixed], sharing: &[Fixed]) -> u64 {
    crossing_rank_at(decay, naming, 1, sharing)
}

/// The decay rule every configuration in this file fuses under.
const fn decay() -> DecayRule {
    DecayRule::ReciprocalRank { k: K }
}

/// **The disjoint-block case.** Two strata declaring different blocks. Only the
/// stratum that named the candidate admits its block, so the threshold for that
/// block is that one stream's head alone: `c(1) > c(r)` first holds at `r = 2`.
#[test]
fn a_candidate_whose_block_only_its_namer_admits_crosses_at_the_second_rank() {
    assert_eq!(
        crossing_rank(decay(), &[Fixed::ONE], &[Fixed::ONE]),
        2,
        "one namer bounded by its own head alone is beaten by the next rank down"
    );
}

/// **The overlap case.** Two strata declaring the *same* block, both naming the
/// candidate. Both sides of the comparison double, so the doubling cancels and
/// the crossing is where it was: `2c(1) > 2c(r)` first holds at `r = 2`.
#[test]
fn a_candidate_both_sharers_named_crosses_at_the_second_rank_too() {
    assert_eq!(
        crossing_rank(
            decay(),
            &[Fixed::ONE, Fixed::ONE],
            &[Fixed::ONE, Fixed::ONE]
        ),
        2,
        "a candidate that collected from every stream admitting its block is \
         bounded by exactly the streams it already read"
    );
}

/// **The case the bound does not survive.** One namer, two open streams sharing
/// the block. The candidate collected one contribution and is measured against
/// two, so the threshold has to fall to *half* its lower bound — which for
/// reciprocal rank means the head must reach past `k` itself.
///
/// The value is exact, not approximate, and it is the value the truncated
/// fixed-point arithmetic gives: at `r = 62` the doubled threshold is
/// `2 · ⌊10¹²/122⌋ = ⌊10¹²/61⌋` **exactly**, a tie rather than a crossing, and
/// a strict comparison does not pass a tie. Re-deriving this in floating point
/// would have said 62.
#[test]
fn a_candidate_one_of_two_sharers_named_crosses_only_past_the_smoothing_constant() {
    assert_eq!(
        crossing_rank(decay(), &[Fixed::ONE], &[Fixed::ONE, Fixed::ONE]),
        63,
        "a lower bound measured against twice its own weight has to outlast the \
         smoothing constant"
    );

    // The tie at the rank below, stated as the arithmetic rather than as a
    // claim about it: this is why the answer is 63 and not 62.
    assert_eq!(
        sum_at(decay(), &[Fixed::ONE], 1),
        sum_at(decay(), &[Fixed::ONE, Fixed::ONE], 62),
        "at the rank below the crossing the two sides are equal, and a strict \
         comparison does not emit on equality"
    );
}

// ---------------------------------------------------------------------------
// 1b. The harness: three configurations of two producers through the ladder.
// ---------------------------------------------------------------------------

/// What the two producers declare about where their candidates lie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Blocks {
    /// Each producer declares its own block, and the two do not meet.
    Distinct,
    /// Both producers declare one and the same block.
    Shared,
}

/// Whether the two producers name the same candidates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Results {
    /// Every candidate is named by exactly one of them.
    Disjoint,
    /// Both name the identical four hundred candidates in the identical order.
    Intersecting,
}

/// One of the three configurations, and its name.
#[derive(Clone, Copy, Debug)]
struct Configuration {
    name: &'static str,
    blocks: Blocks,
    results: Results,
}

/// The control: different blocks, different candidates. The planner's merge
/// argument holds, so the bound is a bound on the read.
const DISTINCT_BLOCKS_DISJOINT_RESULTS: Configuration = Configuration {
    name: "distinct_blocks_disjoint_results",
    blocks: Blocks::Distinct,
    results: Results::Disjoint,
};

/// One block, and both producers naming the same candidates.
const SHARED_BLOCK_INTERSECTING_RESULTS: Configuration = Configuration {
    name: "shared_block_intersecting_results",
    blocks: Blocks::Shared,
    results: Results::Intersecting,
};

/// One block, and candidates no two producers share — structurally the control,
/// differing from it only in what was declared.
const SHARED_BLOCK_DISJOINT_RESULTS: Configuration = Configuration {
    name: "shared_block_disjoint_results",
    blocks: Blocks::Shared,
    results: Results::Disjoint,
};

/// The two strata, in the order the two producers are registered.
fn strata() -> [Iri; 2] {
    [iri(&ex("stratum/left")), iri(&ex("stratum/right"))]
}

/// The block each producer declares, under `blocks`.
fn declared_blocks(blocks: Blocks) -> [DomainTag; 2] {
    match blocks {
        Blocks::Distinct => [domain_tag("domain/left"), domain_tag("domain/right")],
        Blocks::Shared => [domain_tag("domain/shared"), domain_tag("domain/shared")],
    }
}

/// The entity-IRI prefix each producer mints its candidates under, under
/// `results`. One prefix for both is what makes the two candidate sets equal.
const fn candidate_prefixes(results: Results) -> [&'static str; 2] {
    match results {
        Results::Disjoint => ["left/", "right/"],
        Results::Intersecting => ["shared/", "shared/"],
    }
}

/// A producer that mints `ROWS` candidates lazily under `prefix`, counting every
/// row the executor actually takes from it.
///
/// Lazy on purpose. Nothing is materialised in `open`, so the counter measures
/// the read the emitted `LIMIT` licensed rather than the mock's own
/// construction.
struct CountingProducer {
    arity: PfArity,
    mode: BindingPattern,
    prefix: &'static str,
    pulled: Arc<AtomicU64>,
}

impl PropertyFunction for CountingProducer {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        self.arity
    }

    fn modes(&self) -> &[BindingPattern] {
        std::slice::from_ref(&self.mode)
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        ROWS
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        // A bound position is an input the call site supplied, and the engine
        // drops any row that disagrees with it there; echoing it back is the
        // cheapest correct behaviour, exactly as the other mock producers in
        // this crate's tests do.
        let bound: Vec<Option<TermValue>> =
            args.flattened().map(Option::<&TermValue>::cloned).collect();
        Ok(Box::new(CountingCursor {
            prefix: self.prefix,
            emitted: 0,
            bound,
            pulled: Arc::clone(&self.pulled),
        }))
    }
}

/// The lazy cursor behind [`CountingProducer`].
struct CountingCursor {
    prefix: &'static str,
    emitted: u64,
    bound: Vec<Option<TermValue>>,
    pulled: Arc<AtomicU64>,
}

impl PfCursor for CountingCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        if self.emitted >= ROWS {
            return Ok(None);
        }
        let index = self.emitted;
        self.emitted += 1;
        self.pulled.fetch_add(1, Ordering::SeqCst);
        // Zero-padded, so the lexical order of the candidate IRIs is the order
        // the rows arrive in and the declared tie-break's last key agrees with
        // the rank order instead of cutting across it.
        let row = [
            TermValue::iri(format!("{}entity{index:06}", ex(self.prefix))),
            TermValue::iri(format!("{}score{index:06}", ex(self.prefix))),
        ];
        let echoed = row
            .iter()
            .enumerate()
            .map(|(position, value)| {
                self.bound
                    .get(position)
                    .and_then(Clone::clone)
                    .unwrap_or_else(|| value.clone())
            })
            .collect();
        Ok(Some(echoed))
    }
}

/// The two request terms, one per producer. Each names its own predicate, so
/// each reaches exactly one producer and each producer is bound to one term.
fn request_terms() -> Vec<RequestTerm> {
    ["title", "body"]
        .into_iter()
        .map(|predicate| RequestTerm::Lexical {
            text: "quick brown fox".to_owned(),
            language: Some("en".to_owned()),
            predicate: Some(iri(&ex(predicate))),
        })
        .collect()
}

/// The shape one producer accepts: an English literal under its own predicate,
/// rendered into the object-side argument position.
fn accepted_terms(predicate: &str) -> Vec<AcceptedTerm> {
    vec![AcceptedTerm {
        pattern: TermPattern {
            kind: TermKind::Literal,
            datatype: None,
            language: Some("en".to_owned()),
            predicate: Some(ex(predicate)),
        },
        placements: vec![TermPlacement {
            facet: RequestFacet::Value,
            position: 1,
            datatype: None,
        }],
    }]
}

/// The registry for one configuration, with both producers' pull counters.
fn configured_registry(config: Configuration) -> (PropertyFunctionRegistry, [Arc<AtomicU64>; 2]) {
    let blocks = declared_blocks(config.blocks);
    let prefixes = candidate_prefixes(config.results);
    let counters = [Arc::new(AtomicU64::new(0)), Arc::new(AtomicU64::new(0))];
    let mut registry = PropertyFunctionRegistry::new();
    for (index, (predicate, stratum)) in ["title", "body"].into_iter().zip(strata()).enumerate() {
        let arity = PfArity::new(1, 1);
        let relation = CountingProducer {
            arity,
            mode: arity.all_free_mode(),
            prefix: prefixes[index],
            pulled: Arc::clone(&counters[index]),
        };
        registry.register_ranked(
            ex(&format!("pf/{predicate}")),
            Arc::new(relation),
            RankedDeclaration {
                stratum: kernel_iri(stratum.as_str()),
                accepted_terms: accepted_terms(predicate),
                depth_placement: None,
                candidate_position: 0,
                duplicates: DuplicatePolicy::Unique,
                fidelity: RankFidelity::EXACT,
                // One block per producer, so the declaration entails the block
                // of every row and nothing is owed per row.
                domains: CandidateDomains::within([blocks[index].clone()]),
                block_position: None,
                mandatory: false,
            },
        );
    }
    (registry, counters)
}

/// Statistics that narrow nothing: each stratum really does hold the four
/// hundred rows its producer declares, and no selectivity is measured.
struct FixtureStatistics {
    cardinalities: BTreeMap<Iri, u64>,
}

impl Statistics for FixtureStatistics {
    fn source(&self) -> &str {
        "example-statistics"
    }

    fn revision(&self) -> &str {
        "r1"
    }

    fn cardinality(&self, predicate: &Iri) -> Option<u64> {
        self.cardinalities.get(predicate).copied()
    }

    fn selectivity_ppm(&self, _subject: &Iri, _term: &RequestTerm) -> Option<u64> {
        None
    }
}

fn fixture_statistics() -> FixtureStatistics {
    let mut cardinalities = BTreeMap::new();
    for stratum in strata() {
        cardinalities.insert(stratum, ROWS);
    }
    for predicate in ["title", "body"] {
        cardinalities.insert(iri(&ex(predicate)), ROWS);
    }
    FixtureStatistics { cardinalities }
}

/// Unit weight for each stratum, under the one decay rule this file uses.
fn fixture_profile() -> FusionProfile {
    let weights = strata()
        .into_iter()
        .map(|stratum| (stratum, Fixed::ONE))
        .collect();
    FusionProfile::with_decay(weights, decay()).expect("the fixture profile is valid")
}

/// The emitted row ceiling of one compiled unit, parsed out of the text the
/// unit itself renders rather than recomputed from the depth beside it.
fn emitted_limit(unit: &StratumUnit) -> u64 {
    let text = unit.sparql();
    let (_, tail) = text
        .rsplit_once("LIMIT ")
        .expect("a rendered unit carries its own outer bound");
    tail.trim()
        .parse()
        .expect("the emitted bound is a decimal integer")
}

/// Everything one configuration's run is measured on.
struct Measured {
    /// The per-stratum depth the planner recorded.
    planned_depth: BTreeMap<Iri, u32>,
    /// The `LIMIT` the compiler emitted for each unit.
    emitted_limit: BTreeMap<Iri, u64>,
    /// Rows the executor really took out of each relation.
    rows_materialised: BTreeMap<Iri, u64>,
    /// Ranks fusion really pulled off each stream.
    ranks_pulled: BTreeMap<Iri, u64>,
    /// What the trailer says stopped each stream.
    status: BTreeMap<Iri, ProducerStatus>,
    /// The answer itself, with each row's per-stratum rank.
    rows: Vec<FusedRow>,
}

impl Measured {
    /// The value `field` carries for both strata, and a panic naming both when
    /// they disagree — every assertion in this file is about a number the two
    /// symmetric producers reach together.
    fn both<T: Clone + PartialEq + std::fmt::Debug>(
        &self,
        what: &str,
        field: &BTreeMap<Iri, T>,
    ) -> T {
        let [left, right] = strata();
        let left_value = field.get(&left).expect("the left stratum was measured");
        let right_value = field.get(&right).expect("the right stratum was measured");
        assert_eq!(
            left_value, right_value,
            "the two symmetric producers disagree on {what}"
        );
        left_value.clone()
    }

    /// The deepest per-stratum rank any emitted row carries.
    ///
    /// Read off the answer, never off the value under test: this is a property
    /// of which rows the top-k contains, and it is the rank the threshold has to
    /// fall below for the last of them to certify.
    fn deepest_emitted_rank(&self) -> u64 {
        self.rows
            .iter()
            .flat_map(|row| row.contributions.iter().map(|(_, rank, _)| *rank))
            .max()
            .expect("the fixture answer is not empty")
    }

    /// How many strata named the deepest-ranked emitted row.
    fn namers_of_deepest_row(&self) -> usize {
        let deepest = self.deepest_emitted_rank();
        self.rows
            .iter()
            .find(|row| {
                row.contributions
                    .iter()
                    .any(|(_, rank, _)| *rank == deepest)
            })
            .expect("the deepest rank belongs to some emitted row")
            .contributions
            .len()
    }
}

/// Run one configuration through `plan` → `compile` → `execute` → `fuse`.
///
/// The stages are driven by hand rather than through `search` because three of
/// the five measurements live between them: the depth is the plan's, the `LIMIT`
/// is the compiled unit's, and the materialised rows are counted inside the
/// relation while the executor is reading it.
fn measure(config: Configuration, dataset: &RdfDataset) -> Measured {
    let (registry, counters) = configured_registry(config);
    let statistics = fixture_statistics();
    let profile = fixture_profile();
    let request = RetrievalRequest::bounded(request_terms(), TOP_K);

    let planned = plan(&request, &registry, &statistics).expect("the fixture request plans");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: Some(&profile),
    };
    let compiled = compile(&planned, &env).expect("a fresh plan is admitted");

    let emitted_limit: BTreeMap<Iri, u64> = compiled
        .units
        .iter()
        .map(|unit| (unit.stratum.clone(), emitted_limit(unit)))
        .collect();

    let execution =
        block_on(execute(&compiled, &registry, dataset)).expect("the fixture registry executes");

    let mut streams = Vec::new();
    for stream in execution.streams {
        let plan_id = stream.plan_id;
        let fused_bound = stream.fused_bound;
        let attestation = stream.attestation.clone();
        let adapter =
            RankedStreamAdapter::new(stream.stream, stream.contract, &profile, &stream.stratum)
                .expect("the fixture profile weights every executed stratum");
        streams.push((
            stream.stratum,
            adapter
                .with_plan_id(plan_id)
                .with_fused_bound(fused_bound)
                .with_attestation(attestation),
        ));
    }
    let fused = block_on(fuse::<RankedStreamAdapter, Term>(
        streams,
        &profile,
        compiled.fused_bound,
    ))
    .expect("the executed streams fuse");
    let trailer = fused.trailer.completed_with(execution.statuses);

    let planned_depth = planned
        .stratum_depths
        .iter()
        .map(|(stratum, depth)| (stratum.clone(), *depth))
        .collect();
    let rows_materialised = strata()
        .into_iter()
        .zip(counters.iter())
        .map(|(stratum, counter)| (stratum, counter.load(Ordering::SeqCst)))
        .collect();
    let ranks_pulled = trailer
        .resolution
        .iter()
        .map(|(stratum, resolution)| (stratum.clone(), resolution.ranks_pulled))
        .collect();

    Measured {
        planned_depth,
        emitted_limit,
        rows_materialised,
        ranks_pulled,
        status: trailer.statuses.clone(),
        rows: fused.rows,
    }
}

/// The five measured numbers of one configuration, rendered for a failure to
/// read as a row of the table this file pins.
fn render(config: Configuration, measured: &Measured) -> String {
    format!(
        "{name}: depth={depth} limit={limit} materialised={materialised} \
         ranks_pulled={pulled} status={status:?}",
        name = config.name,
        depth = measured.both("the planned depth", &measured.planned_depth),
        limit = measured.both("the emitted LIMIT", &measured.emitted_limit),
        materialised = measured.both("the rows materialised", &measured.rows_materialised),
        pulled = measured.both("the ranks pulled", &measured.ranks_pulled),
        status = measured.both("the terminal status", &measured.status),
    )
}

/// The status a fusion that stopped a live stream writes, at the contribution
/// its head was sitting on. Computed from the profile's own law over the rank
/// the run really reached, never read back out of the trailer.
fn ceiling_at(rank: u64) -> ProducerStatus {
    ProducerStatus::CeilingReached {
        bound: contribution_under(decay(), Fixed::ONE, rank)
            .expect("the fixture contribution is in range"),
    }
}

/// **The control, and the solved case.** Different blocks, different candidates.
///
/// Every candidate lies in exactly one declared block, so the planner's merge
/// argument holds and the request's bound of five becomes the recorded depth.
/// The compiler emits one probe row past it, the executor reads no further, and
/// fusion certifies the fifth row with both heads one rank past the deepest row
/// it emitted.
#[test]
fn distinct_blocks_disjoint_results_read_only_what_the_bound_needs() {
    let dataset = common::empty_dataset();
    let measured = measure(DISTINCT_BLOCKS_DISJOINT_RESULTS, &dataset);
    let report = render(DISTINCT_BLOCKS_DISJOINT_RESULTS, &measured);

    assert_eq!(measured.rows.len(), 5, "the bound is what stopped this run");
    assert_eq!(
        measured.both("the planned depth", &measured.planned_depth),
        5,
        "the disjoint declarations license the request's own bound as the \
         depth — {report}"
    );
    assert_eq!(
        measured.both("the emitted LIMIT", &measured.emitted_limit),
        6,
        "and the emitted bound is that depth plus its probe row — {report}"
    );
    assert_eq!(
        measured.both("the ranks pulled", &measured.ranks_pulled),
        4,
        "and fusion certified five rows without reading past the fourth — {report}"
    );
    assert_eq!(
        measured.both("the terminal status", &measured.status),
        ceiling_at(4),
        "a stream fusion stopped reports the contribution it stopped at — {report}"
    );

    // The derivation, with no executor in it, predicts the measured number.
    // The deepest row this answer contains sits at rank three in its own
    // stratum, and only its own stratum admits its block, so the threshold has
    // to fall to the fourth rank for it to certify.
    let deepest = measured.deepest_emitted_rank();
    assert_eq!(
        deepest, 3,
        "the fifth row of an alternating answer — {report}"
    );
    assert_eq!(
        measured.namers_of_deepest_row(),
        1,
        "disjoint blocks mean one namer per candidate — {report}"
    );
    assert_eq!(
        crossing_rank_at(decay(), &[Fixed::ONE], deepest, &[Fixed::ONE]),
        measured.both("the ranks pulled", &measured.ranks_pulled),
        "the threshold derivation predicts the read — {report}"
    );
}

/// **One block, and both producers naming the same candidates.**
///
/// Two strata sharing a block are not pairwise disjoint, so the planner's merge
/// argument does not hold and the depth falls back to the declared four
/// hundred. Fusion still stops at the sixth rank — every candidate collected
/// from both streams that admit its block, so its lower bound is measured
/// against exactly the streams it already read — but the executor has already
/// materialised the whole relation by then.
#[test]
fn shared_block_intersecting_results_read_the_whole_relation_to_answer_at_the_sixth_rank() {
    let dataset = common::empty_dataset();
    let measured = measure(SHARED_BLOCK_INTERSECTING_RESULTS, &dataset);
    let report = render(SHARED_BLOCK_INTERSECTING_RESULTS, &measured);

    assert_eq!(measured.rows.len(), 5, "the bound is what stopped this run");
    assert_eq!(
        measured.both("the planned depth", &measured.planned_depth),
        400,
        "a shared block is not a disjoint pair, so the declaration stands as \
         the depth — {report}"
    );
    assert_eq!(
        measured.both("the emitted LIMIT", &measured.emitted_limit),
        401,
        "and the emitted bound is that depth plus its probe row — {report}"
    );
    assert_eq!(
        measured.both("the rows materialised", &measured.rows_materialised),
        400,
        "so the executor read every row the relation held — {report}"
    );
    assert_eq!(
        measured.both("the ranks pulled", &measured.ranks_pulled),
        6,
        "while fusion needed six ranks of it — {report}"
    );
    assert_eq!(
        measured.both("the terminal status", &measured.status),
        ceiling_at(6),
        "a stream fusion stopped reports the contribution it stopped at — {report}"
    );

    // The gap between the two numbers above is the measurement this
    // configuration exists to make, and it is stated as the subtraction rather
    // than as a word: the executor's read is not bounded by what fusion
    // consumed, and here it overshot by three hundred and ninety-four rows per
    // stream.
    assert_eq!(
        measured.both("the rows materialised", &measured.rows_materialised)
            - measured.both("the ranks pulled", &measured.ranks_pulled),
        394,
        "rows the executor materialised that fusion never asked for — {report}"
    );

    let deepest = measured.deepest_emitted_rank();
    assert_eq!(
        deepest, 5,
        "both streams rank the same candidates — {report}"
    );
    assert_eq!(
        measured.namers_of_deepest_row(),
        2,
        "and both of them name every candidate — {report}"
    );
    assert_eq!(
        crossing_rank_at(
            decay(),
            &[Fixed::ONE, Fixed::ONE],
            deepest,
            &[Fixed::ONE, Fixed::ONE]
        ),
        measured.both("the ranks pulled", &measured.ranks_pulled),
        "the threshold derivation predicts the read — {report}"
    );
}

/// **The permanent control: one block, and no candidate named twice.**
///
/// This configuration never reaches the threshold gate at all. A candidate the
/// left stream named is not *final* while the right stream — which declares the
/// same block, and so could still name it — is open, so nothing certifies until
/// both streams run out. Both drain, and both report exhaustion.
///
/// The four hundreds below are the evidence of what an improvement would be
/// improving. They stay asserted exactly, so work that lowers them has to
/// change this literal in the same commit that earns it.
#[test]
fn shared_block_disjoint_results_drain_both_streams_unbounded_control() {
    let dataset = common::empty_dataset();
    let measured = measure(SHARED_BLOCK_DISJOINT_RESULTS, &dataset);
    let report = render(SHARED_BLOCK_DISJOINT_RESULTS, &measured);

    assert_eq!(measured.rows.len(), 5, "the answer is still five rows");
    assert_eq!(
        measured.both("the planned depth", &measured.planned_depth),
        400,
        "a shared block is not a disjoint pair — {report}"
    );
    assert_eq!(
        measured.both("the emitted LIMIT", &measured.emitted_limit),
        401,
        "and the emitted bound is that depth plus its probe row — {report}"
    );
    assert_eq!(
        measured.both("the rows materialised", &measured.rows_materialised),
        400,
        "the executor read every row the relation held — {report}"
    );
    assert_eq!(
        measured.both("the ranks pulled", &measured.ranks_pulled),
        400,
        "and fusion read every one of them, because no candidate is final \
         while a stream sharing its block is still open — {report}"
    );
    assert_eq!(
        measured.both("the terminal status", &measured.status),
        ProducerStatus::Exhausted { rows_emitted: ROWS },
        "a stream read to its end reports exhaustion, not a ceiling — {report}"
    );

    // Why the threshold derivation does not apply here: the gate this run never
    // reached would have let it stop at the sixty-sixth rank. The candidates
    // are single-named and two streams share their block, so the crossing is
    // the expensive one — and even that is a sixth of what was read.
    let deepest = measured.deepest_emitted_rank();
    assert_eq!(
        deepest, 3,
        "the fifth row of an alternating answer — {report}"
    );
    assert_eq!(
        crossing_rank_at(decay(), &[Fixed::ONE], deepest, &[Fixed::ONE, Fixed::ONE]),
        66,
        "the threshold alone would have licensed a stop at the sixty-sixth \
         rank; finality is what forced the drain — {report}"
    );
}

/// **Anti-vacuity.** The control and the permanent control have the identical
/// result structure — four hundred rows each, no candidate named twice, the
/// same bound, the same weights. They differ in one thing only: what the two
/// producers *declared* about where their candidates lie.
///
/// If every measured number agreed, this harness would be measuring the
/// fixtures rather than the declaration, and every other assertion in this file
/// would be worth nothing.
#[test]
fn the_declaration_alone_moves_every_measured_number() {
    let dataset = common::empty_dataset();
    let distinct = measure(DISTINCT_BLOCKS_DISJOINT_RESULTS, &dataset);
    let shared = measure(SHARED_BLOCK_DISJOINT_RESULTS, &dataset);

    // First the sameness, so the difference below is known to come from the
    // declaration: the two runs return the same answer.
    let answer = |measured: &Measured| -> Vec<(String, Fixed)> {
        measured
            .rows
            .iter()
            .map(|row| (row.entity.as_str().to_owned(), row.score))
            .collect()
    };
    assert_eq!(
        answer(&distinct),
        answer(&shared),
        "the two configurations hold the identical candidates and must answer \
         identically; only the cost of the answer is in question"
    );

    // Then the difference, number by number.
    assert_ne!(
        distinct.both("the planned depth", &distinct.planned_depth),
        shared.both("the planned depth", &shared.planned_depth),
        "the declaration decides the depth"
    );
    assert_ne!(
        distinct.both("the emitted LIMIT", &distinct.emitted_limit),
        shared.both("the emitted LIMIT", &shared.emitted_limit),
        "the declaration decides the emitted bound"
    );
    assert_ne!(
        distinct.both("the rows materialised", &distinct.rows_materialised),
        shared.both("the rows materialised", &shared.rows_materialised),
        "the declaration decides how much the executor reads"
    );
    assert_ne!(
        distinct.both("the ranks pulled", &distinct.ranks_pulled),
        shared.both("the ranks pulled", &shared.ranks_pulled),
        "the declaration decides how much fusion reads"
    );
    assert_ne!(
        distinct.both("the terminal status", &distinct.status),
        shared.both("the terminal status", &shared.status),
        "and the declaration decides which ending the trailer reports"
    );
}

// ---------------------------------------------------------------------------
// 1d. Instance optimality, against a brute-forced certificate.
// ---------------------------------------------------------------------------

/// How far past the certificate the engine's total read may sit.
///
/// **What it bounds:** the ratio of the *total* ranks the engine pulled across
/// every stream of a generated instance to the total depth of that instance's
/// certificate — the shallowest per-stream prefix from which the answer is
/// entailed. It is a per-instance bound, asserted on each instance below, not an
/// average over them.
///
/// The certificate is computed by an oracle that already knows where every
/// stream ends. The engine knows neither, so it pays for two things the
/// certificate does not: one unmerged head per open stream, which is the only
/// thing that makes the threshold fall at all, and — where a candidate is named
/// by one stream but shares its block with others — a threshold summed over
/// every sharer, which its single contribution has to outlast.
///
/// Six is the pinned value because the worst instance below reaches 16 ranks
/// against a certificate of 3, which is 5⅓. The gap is concentrated in
/// `one_block_disjoint_items`, where finality — not the threshold — forbids the
/// stop: exactly the configuration the four-hundred-row control pins at full
/// scale. Every other instance sits at or below 2.
const CERTIFICATE_FACTOR: u64 = 6;

/// One stratum of a generated instance.
struct CertStratum {
    name: &'static str,
    /// The blocks this stream declares. Every item it holds lies in one of them.
    tags: Vec<&'static str>,
    weight: Fixed,
    /// Its items, in rank order.
    items: Vec<String>,
}

/// A generated instance: a handful of short streams and a bound.
struct Instance {
    name: &'static str,
    strata: Vec<CertStratum>,
    top_k: TopK,
}

const BLOCK_DOCS: &str = "http://example.org/domain/documents";
const BLOCK_PEOPLE: &str = "http://example.org/domain/people";

/// The block an item really lies in, read off the item itself — which is what
/// the partition axiom says a block is.
fn block_of(item: &str) -> &'static str {
    if item.contains("/doc/") {
        BLOCK_DOCS
    } else {
        BLOCK_PEOPLE
    }
}

/// `count` items under `prefix`, zero padded so lexical order is rank order.
fn items(prefix: &str, first: u64, count: u64) -> Vec<String> {
    (first..first + count)
        .map(|index| format!("http://example.org/{prefix}/{index:06}"))
        .collect()
}

/// A scripted stream for the certificate oracle: a prefix of one stratum's
/// items, ending in whatever receipt the caller states.
struct ScriptedStream {
    rows: VecDeque<RankedRow<Term>>,
    receipt: ProducerReceipt,
    contract: StreamContract,
}

// The trait's methods are `async`; these bodies are synchronous because the rows
// are pre-scripted. The keyword is required to implement the trait.
#[allow(clippy::unused_async_trait_impl)]
impl RankedStream for ScriptedStream {
    type Item = Term;

    async fn next(&mut self) -> Result<Option<RankedRow<Self::Item>>, ProtocolError> {
        Ok(self.rows.pop_front())
    }

    async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError> {
        Ok(self.receipt.clone())
    }

    fn contract(&self) -> StreamContract {
        self.contract.clone()
    }

    fn plan_id(&self) -> Option<PlanId> {
        None
    }

    fn attestation(&self) -> PfAttestation {
        PfAttestation::UNDECLARED
    }
}

fn cert_stratum_iri(name: &str) -> Iri {
    iri(&ex(&format!("stratum/{name}")))
}

fn cert_profile(instance: &Instance) -> FusionProfile {
    let weights = instance
        .strata
        .iter()
        .map(|entry| (cert_stratum_iri(entry.name), entry.weight))
        .collect();
    FusionProfile::with_decay(weights, decay()).expect("the generated profile is valid")
}

/// The instance's streams, each cut to `depths[index]` rows and declaring that
/// it ended there.
///
/// An oracle's prefix ends because the oracle chose to cut it, and a cut prefix
/// that claimed a deeper stream would be a different question. `Exhausted` is
/// therefore the honest receipt: the certificate asks what a fusion could
/// conclude from exactly these rows.
fn cert_streams(instance: &Instance, depths: &[usize]) -> Vec<(Iri, ScriptedStream)> {
    instance
        .strata
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let rows: VecDeque<RankedRow<Term>> = entry
                .items
                .iter()
                .take(depths[index])
                .enumerate()
                .map(|(position, item)| {
                    let rank = u64::try_from(position + 1).expect("generated ranks fit");
                    RankedRow::new(
                        rank,
                        contribution_under(decay(), entry.weight, rank)
                            .expect("generated contributions fit"),
                        Term::new(item),
                        RowBlock::Declared(
                            DomainTag::parse(block_of(item))
                                .expect("generated block tags are valid IRIs"),
                        ),
                    )
                })
                .collect();
            let emitted = u64::try_from(rows.len()).expect("generated row counts fit");
            (
                cert_stratum_iri(entry.name),
                ScriptedStream {
                    rows,
                    receipt: ProducerReceipt::Exhausted {
                        rows_emitted: emitted,
                    },
                    contract: StreamContract::new(
                        DuplicatePolicy::Unique,
                        RankFidelity::EXACT,
                        CandidateDomains::within(entry.tags.iter().map(|tag| {
                            DomainTag::parse(tag).expect("generated block tags are valid IRIs")
                        })),
                    ),
                },
            )
        })
        .collect()
}

fn cert_fuse(instance: &Instance, depths: &[usize]) -> FusionResult<Term> {
    block_on(fuse::<ScriptedStream, Term>(
        cert_streams(instance, depths),
        &cert_profile(instance),
        instance.top_k,
    ))
    .expect("the generated streams obey the protocol")
}

/// The answer a fusion returned, as the thing a certificate has to reproduce:
/// the rows, their scores, and their order.
fn answer_of(result: &FusionResult<Term>) -> Vec<(String, Fixed)> {
    result
        .rows
        .iter()
        .map(|row| (row.entity.as_str().to_owned(), row.score))
        .collect()
}

/// Every depth vector over the instance's streams, from all-zero upward.
fn depth_vectors(lengths: &[usize]) -> Vec<Vec<usize>> {
    let mut vectors = vec![Vec::new()];
    for length in lengths {
        let mut next = Vec::new();
        for prefix in &vectors {
            for depth in 0..=*length {
                let mut extended = prefix.clone();
                extended.push(depth);
                next.push(extended);
            }
        }
        vectors = next;
    }
    vectors
}

/// The certificate: the cheapest per-stream prefix from which this instance's
/// answer is entailed.
///
/// Brute force over every prefix vector, keeping those whose fusion reproduces
/// the full answer exactly — same rows, same scores, same order — and among
/// them the one with the smallest total depth. Ties are settled by the shallower
/// deepest stream, then lexically, so the answer is one vector and not an
/// arbitrary member of a set.
fn certificate(instance: &Instance) -> Vec<usize> {
    let lengths: Vec<usize> = instance
        .strata
        .iter()
        .map(|entry| entry.items.len())
        .collect();
    let full = cert_fuse(instance, &lengths);
    let target = answer_of(&full);
    let mut best: Option<Vec<usize>> = None;
    for candidate in depth_vectors(&lengths) {
        if answer_of(&cert_fuse(instance, &candidate)) != target {
            continue;
        }
        let key = |vector: &[usize]| {
            (
                vector.iter().sum::<usize>(),
                vector.iter().copied().max().unwrap_or(0),
                vector.to_vec(),
            )
        };
        match &best {
            Some(current) if key(current) <= key(&candidate) => {}
            _ => best = Some(candidate),
        }
    }
    best.expect("the full prefix always entails the answer it produced")
}

/// The instances the certificate is brute-forced over: two and three strata, at
/// most a dozen items each, across the overlap patterns a declaration can take.
fn instances() -> Vec<Instance> {
    vec![
        // Disjoint blocks, disjoint items: the shape the planner's merge
        // argument covers.
        Instance {
            name: "two_disjoint_blocks",
            strata: vec![
                CertStratum {
                    name: "docs",
                    tags: vec![BLOCK_DOCS],
                    weight: Fixed::ONE,
                    items: items("doc", 1, 8),
                },
                CertStratum {
                    name: "people",
                    tags: vec![BLOCK_PEOPLE],
                    weight: Fixed::ONE,
                    items: items("person", 1, 8),
                },
            ],
            top_k: TopK::new(3),
        },
        // One block, and the two streams rank the same items in opposite
        // orders: total overlap, maximal disagreement.
        Instance {
            name: "one_block_reversed_agreement",
            strata: vec![
                CertStratum {
                    name: "docs",
                    tags: vec![BLOCK_DOCS],
                    weight: Fixed::ONE,
                    items: items("doc", 1, 8),
                },
                CertStratum {
                    name: "titles",
                    tags: vec![BLOCK_DOCS],
                    weight: Fixed::ONE,
                    items: items("doc", 1, 8).into_iter().rev().collect(),
                },
            ],
            top_k: TopK::new(3),
        },
        // One block, partial overlap: four items in common, four in neither.
        Instance {
            name: "one_block_partial_overlap",
            strata: vec![
                CertStratum {
                    name: "docs",
                    tags: vec![BLOCK_DOCS],
                    weight: Fixed::ONE,
                    items: items("doc", 1, 8),
                },
                CertStratum {
                    name: "titles",
                    tags: vec![BLOCK_DOCS],
                    weight: Fixed::ONE,
                    items: items("doc", 5, 8),
                },
            ],
            top_k: TopK::new(4),
        },
        // Two disjoint blocks and a third stream that cross-cuts both: the
        // quality-prior shape, where no stream may be skipped by anyone.
        Instance {
            name: "cross_cutting_prior",
            strata: vec![
                CertStratum {
                    name: "docs",
                    tags: vec![BLOCK_DOCS],
                    weight: Fixed::ONE,
                    items: items("doc", 1, 6),
                },
                CertStratum {
                    name: "people",
                    tags: vec![BLOCK_PEOPLE],
                    weight: Fixed::ONE,
                    items: items("person", 1, 6),
                },
                CertStratum {
                    name: "prior",
                    tags: vec![BLOCK_DOCS, BLOCK_PEOPLE],
                    weight: Fixed::ONE,
                    items: items("doc", 1, 3)
                        .into_iter()
                        .zip(items("person", 1, 3))
                        .flat_map(|(doc, person)| [doc, person])
                        .collect(),
                },
            ],
            top_k: TopK::new(3),
        },
        // Disjoint blocks of very different lengths, under a bound that reaches
        // past the shorter one.
        Instance {
            name: "uneven_disjoint_blocks",
            strata: vec![
                CertStratum {
                    name: "docs",
                    tags: vec![BLOCK_DOCS],
                    weight: Fixed::ONE,
                    items: items("doc", 1, 4),
                },
                CertStratum {
                    name: "people",
                    tags: vec![BLOCK_PEOPLE],
                    weight: Fixed::ONE,
                    items: items("person", 1, 12),
                },
            ],
            top_k: TopK::new(5),
        },
        // One block, disjoint items: the configuration finality forbids stopping
        // early in, carried down to a size a brute force can certify.
        Instance {
            name: "one_block_disjoint_items",
            strata: vec![
                CertStratum {
                    name: "docs",
                    tags: vec![BLOCK_DOCS],
                    weight: Fixed::ONE,
                    items: items("doc", 1, 8),
                },
                CertStratum {
                    name: "titles",
                    tags: vec![BLOCK_DOCS],
                    weight: Fixed::ONE,
                    items: items("doc", 100, 8),
                },
            ],
            top_k: TopK::new(3),
        },
    ]
}

/// The engine reads within a pinned constant factor of the certificate on every
/// generated instance.
#[test]
fn the_engine_reads_within_a_pinned_factor_of_the_certificate() {
    for instance in instances() {
        let lengths: Vec<usize> = instance
            .strata
            .iter()
            .map(|entry| entry.items.len())
            .collect();
        let run = cert_fuse(&instance, &lengths);
        let engine: u64 = instance
            .strata
            .iter()
            .map(|entry| {
                run.trailer
                    .resolution
                    .get(&cert_stratum_iri(entry.name))
                    .expect("the generated profile weights every generated stratum")
                    .ranks_pulled
            })
            .sum();

        let cert = certificate(&instance);
        let cert_total: u64 = cert
            .iter()
            .map(|depth| u64::try_from(*depth).expect("generated depths fit"))
            .sum();

        // A certificate of zero would make the ratio meaningless, and an answer
        // of `k` rows cannot be entailed by no rows at all.
        assert!(
            cert_total > 0,
            "{}: the certificate reads nothing, which cannot entail an answer",
            instance.name
        );
        assert!(
            engine <= CERTIFICATE_FACTOR * cert_total,
            "{}: the engine pulled {engine} ranks against a certificate of \
             {cert_total} ({cert:?}), which is past the pinned factor of \
             {CERTIFICATE_FACTOR}",
            instance.name
        );

        // And the certificate really is a certificate: reading it reproduces the
        // answer, and every strictly shallower prefix does not.
        assert_eq!(
            answer_of(&cert_fuse(&instance, &cert)),
            answer_of(&run),
            "{}: the certificate must reproduce the answer it certifies",
            instance.name
        );
        for (index, depth) in cert.iter().enumerate() {
            if *depth == 0 {
                continue;
            }
            let mut shallower = cert.clone();
            shallower[index] -= 1;
            assert_ne!(
                answer_of(&cert_fuse(&instance, &shallower)),
                answer_of(&run),
                "{}: {shallower:?} already entails the answer, so {cert:?} is \
                 not the certificate",
                instance.name
            );
        }
    }
}

/// The three configurations' measured table, in one place, so a failure reads
/// as the row that moved rather than as five separate assertions.
///
/// Every number here is also asserted individually above; this exists because
/// the table is the evidence, and evidence spread across three tests is evidence
/// nobody reads together.
#[test]
fn the_measured_table_is_what_it_was() {
    let dataset = common::empty_dataset();
    let rendered: Vec<String> = [
        DISTINCT_BLOCKS_DISJOINT_RESULTS,
        SHARED_BLOCK_INTERSECTING_RESULTS,
        SHARED_BLOCK_DISJOINT_RESULTS,
    ]
    .into_iter()
    .map(|config| render(config, &measure(config, &dataset)))
    .collect();

    assert_eq!(
        rendered,
        vec![
            format!(
                "distinct_blocks_disjoint_results: depth=5 limit=6 materialised=6 \
                 ranks_pulled=4 status={:?}",
                ceiling_at(4)
            ),
            format!(
                "shared_block_intersecting_results: depth=400 limit=401 \
                 materialised=400 ranks_pulled=6 status={:?}",
                ceiling_at(6)
            ),
            format!(
                "shared_block_disjoint_results: depth=400 limit=401 \
                 materialised=400 ranks_pulled=400 status={:?}",
                ProducerStatus::Exhausted { rows_emitted: ROWS }
            ),
        ],
        "the measured table moved"
    );

    // Ordered set equality is not what is being claimed — the three rows are
    // three distinct measurements, and a harness that collapsed two of them
    // would be measuring less than it says.
    let distinct: BTreeSet<&String> = rendered.iter().collect();
    assert_eq!(
        distinct.len(),
        3,
        "three configurations must measure three different things"
    );
}
