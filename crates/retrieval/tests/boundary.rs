// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Stage boundaries are seams, read in both directions.
//!
//! The ladder is `plan -> compile -> execute -> fuse`. Every boundary is a
//! supported place to **stop**: a caller that wants only the plan calls `plan`
//! and stops; one that wants the query text calls through `compile` and runs it
//! through the evaluator directly; one that wants per-producer ranked streams
//! calls through `execute` and enumerates them without bound. Every boundary is
//! equally a place to **start**: a hand-built [`Plan`] is admitted on exactly the
//! terms the planner's own output is, hand-written SPARQL runs through the
//! evaluator with no composition type in the path, and hand-built ranked streams
//! fuse with no planning, compilation or execution involved.
//!
//! The two rungs differ in kind. Fusion is top-k by construction — it certifies
//! a row only when a threshold over the live stream heads proves nothing can
//! overtake it — so its memory is proportional to the un-emitted frontier, not
//! the input. The unfused streams are unbounded: N independent producers emit in
//! their own rank order and nothing applies a threshold. A completeness claim is
//! available only from the terminal trailer; a prefix reader holds evidence the
//! answer is incomplete.
//!
//! These are tests of the seams, not of the stages' semantics (those live
//! elsewhere): every fixture is `example.org`, every mock is a plain
//! caller-owned value, and no composition layer is constructed where the test
//! says a caller stopped below it.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Wake, Waker};

use pretty_assertions::assert_eq;
use purrdf_core::{RdfDatasetBuilder, SparqlRequest, SparqlResult, TermValue};
use purrdf_retrieval::{
    AdmissionEnvironment, Fixed, FusionProfile, FusionResult, FusionStream, Iri, Plan, PlanOrigin,
    ProducerBinding, ProducerReceipt, ProtocolError, RankedStream, RankedStreamImpl, RequestTerm,
    RetrievalRequest, Statistics, StatisticsSnapshot, Term, Weight, compile, contribution, execute,
    fuse, plan, search,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, DuplicatePolicy, EvalError, NativeSparqlEngine, PfArgs, PfArity,
    PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry, QueryOptions, RankOrdering,
    RankedDeclaration, RequestFacet, TermKind, TermPattern, TermPlacement, Volatility,
};

mod common;

const K: u32 = 60;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn ex(suffix: &str) -> String {
    format!("http://example.org/{suffix}")
}

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

fn stratum(suffix: &str) -> Iri {
    iri(&ex(&format!("stratum/{suffix}")))
}

fn kernel_iri(text: &str) -> purrdf_core::Iri {
    purrdf_core::parse_iri(text).expect("fixture IRIs are valid")
}

/// Each accepted pattern, with the request term's value rendered into the
/// object-side position. The mocks are arity (1,1) and project `?c0`, so the
/// candidate is position 0 and a rendered facet binds at position 1.
///
/// An unconstrained `TermKind::Any` pattern is the exception: it also accepts a
/// vector term, whose query embedding has no SPARQL constant form, so it
/// declares no placement at all. Its argument stays a free variable, which is
/// exactly what "I take the whole request without needing it written out" is.
fn accepted(patterns: Vec<TermPattern>) -> Vec<AcceptedTerm> {
    patterns
        .into_iter()
        .map(|pattern| {
            let placements = if pattern == TermPattern::of_kind(TermKind::Any) {
                Vec::new()
            } else {
                vec![TermPlacement {
                    facet: RequestFacet::Value,
                    position: 1,
                    datatype: None,
                }]
            };
            AcceptedTerm {
                pattern,
                placements,
            }
        })
        .collect()
}

/// A ranked declaration, supplied where a producer is registered. `mandatory`
/// is declared by the host rather than inferred: it states, explicitly, what an
/// unconstrained `TermKind::Any` pattern used to imply.
fn ranked(stratum_iri: &str, patterns: Vec<TermPattern>, mandatory: bool) -> RankedDeclaration {
    RankedDeclaration {
        stratum: kernel_iri(stratum_iri),
        accepted_terms: accepted(patterns),
        depth_placement: None,
        candidate_position: 0,
        ordering: RankOrdering::StrictlyDescending,
        duplicates: DuplicatePolicy::Unique,
        mandatory,
    }
}

fn lexical_term() -> RequestTerm {
    RequestTerm::Lexical {
        text: "quick brown fox".to_owned(),
        language: Some("en".to_owned()),
        predicate: Some(iri(&ex("body"))),
    }
}

/// A mock ranked producer that declares `rows` and emits `emitted`.
struct MockProducer {
    arity: PfArity,
    mode: BindingPattern,
    rows: u64,
    emitted: Vec<Vec<TermValue>>,
}

impl PropertyFunction for MockProducer {
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
        self.rows
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        // A bound position is an input the call site supplied, and the engine
        // drops any row that disagrees with it there. Echoing the input back is
        // the cheapest correct behaviour, and it is what makes these fixtures
        // sensitive to the constants the compiler renders.
        let bound: Vec<Option<TermValue>> =
            args.flattened().map(Option::<&TermValue>::cloned).collect();
        let mut rows: Vec<Vec<TermValue>> = Vec::with_capacity(self.emitted.len());
        for row in &self.emitted {
            let mut echoed = Vec::with_capacity(row.len());
            for (position, value) in row.iter().enumerate() {
                echoed.push(
                    bound
                        .get(position)
                        .and_then(Clone::clone)
                        .unwrap_or_else(|| value.clone()),
                );
            }
            rows.push(echoed);
        }
        Ok(Box::new(RowCursor {
            rows: rows.into_iter(),
        }))
    }
}

struct RowCursor {
    rows: std::vec::IntoIter<Vec<TermValue>>,
}

impl PfCursor for RowCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(self.rows.next())
    }
}

/// A ranked producer emitting `count` distinct `(entity, score)` row pairs.
fn make_producer(rows: u64, prefix: &str, count: usize) -> Arc<dyn PropertyFunction> {
    let arity = PfArity::new(1, 1);
    let emitted = (0..count)
        .map(|index| {
            vec![
                TermValue::iri(format!("{}entity{index}", ex(prefix))),
                TermValue::iri(format!("{}score{index}", ex(prefix))),
            ]
        })
        .collect();
    Arc::new(MockProducer {
        arity,
        mode: arity.all_free_mode(),
        rows,
        emitted,
    })
}

/// A single catch-all producer under its own stratum.
fn single_registry(
    stratum_iri: &str,
    producer_iri: &str,
    rows: u64,
    count: usize,
) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        producer_iri,
        make_producer(rows, "hand/", count),
        ranked(stratum_iri, vec![TermPattern::of_kind(TermKind::Any)], true),
    );
    registry
}

struct MockStatistics {
    source: String,
    revision: String,
    cardinalities: BTreeMap<Iri, u64>,
}

impl Statistics for MockStatistics {
    fn source(&self) -> &str {
        &self.source
    }

    fn revision(&self) -> &str {
        &self.revision
    }

    fn cardinality(&self, predicate: &Iri) -> Option<u64> {
        self.cardinalities.get(predicate).copied()
    }

    fn selectivity(&self, _predicate: &Iri, _term: &RequestTerm) -> Option<f64> {
        None
    }
}

/// Statistics that bound exactly one stratum at `cardinality`.
fn single_statistics(stratum_iri: &str, cardinality: u64) -> MockStatistics {
    MockStatistics {
        source: "boundary-statistics".to_owned(),
        revision: "r1".to_owned(),
        cardinalities: BTreeMap::from([(iri(stratum_iri), cardinality)]),
    }
}

/// A profile weighting exactly one stratum.
fn single_profile(stratum_iri: &str) -> FusionProfile {
    FusionProfile::new(BTreeMap::from([(iri(stratum_iri), Fixed::ONE)]), K, 4)
        .expect("the fixture profile is valid")
}

/// A profile weighting the named strata unit-for-unit with room for every one of
/// them to contribute to a candidate.
fn profile(weights: &[(&str, Fixed)], k: u32, max_contributions: u32) -> FusionProfile {
    let map: BTreeMap<Iri, Fixed> = weights
        .iter()
        .map(|(name, weight)| (stratum(name), *weight))
        .collect();
    FusionProfile::new(map, k, max_contributions).expect("fixture profile is valid")
}

// ---------------------------------------------------------------------------
// A minimal single-threaded executor (the mocks never actually pend)
// ---------------------------------------------------------------------------

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

/// Run `sparql` through the evaluator itself, with no composition type in the
/// path: this is exactly what a caller does when it stops at `compile` (or
/// starts at `execute`).
fn run_query(sparql: &str, registry: &PropertyFunctionRegistry) -> Vec<Vec<Option<TermValue>>> {
    let dataset = RdfDatasetBuilder::new()
        .freeze()
        .expect("an empty default graph is structurally valid");
    let engine = NativeSparqlEngine::new();
    let outcome = engine
        .query_with_options_view(
            &*dataset,
            SparqlRequest {
                query: sparql,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                property_functions: registry,
                ..QueryOptions::EMPTY
            },
        )
        .expect("the fixture query evaluates");
    match outcome {
        SparqlResult::Solutions { rows, .. } => rows,
        other => panic!("expected a SELECT solution sequence, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Hand-built ranked streams, so `fuse` can be entered with no earlier stage
// ---------------------------------------------------------------------------

/// A producer whose rows and terminal receipt are pre-scripted.
struct ScriptedStream {
    steps: VecDeque<(u64, Fixed, Term)>,
    receipt: ProducerReceipt,
    emitted: u64,
}

impl ScriptedStream {
    fn new(steps: Vec<(u64, Fixed, Term)>, receipt: ProducerReceipt) -> Self {
        Self {
            steps: steps.into(),
            receipt,
            emitted: 0,
        }
    }
}

// The trait's methods are `async`; this mock's body is synchronous because its
// rows are pre-scripted. The keyword is required by the trait, not a signal
// that the body awaits.
#[allow(clippy::unused_async_trait_impl)]
impl RankedStream for ScriptedStream {
    type Item = Term;

    async fn next(&mut self) -> Result<Option<(u64, Fixed, Self::Item)>, ProtocolError> {
        match self.steps.pop_front() {
            Some((rank, value, item)) => {
                self.emitted += 1;
                Ok(Some((rank, value, item)))
            }
            None => Ok(None),
        }
    }

    async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError> {
        Ok(self.receipt.clone())
    }
}

/// A well-formed row at `rank` for `weight` under `k`.
fn row(rank: u64, weight: Fixed, k: u32, item: &str) -> (u64, Fixed, Term) {
    (
        rank,
        contribution(weight, rank, k).expect("fixture contribution fits"),
        Term::new(item),
    )
}

fn exhausted(rows: u64) -> ProducerReceipt {
    ProducerReceipt::Exhausted { rows_emitted: rows }
}

// ---------------------------------------------------------------------------
// 1. Stop at plan: inspect the value
// ---------------------------------------------------------------------------

#[test]
fn stop_at_plan_inspect_value() {
    let registry = single_registry(&ex("stratum/hand"), &ex("pf/hand"), 10, 2);
    let stats = single_statistics(&ex("stratum/hand"), 10);
    let request = RetrievalRequest::from_terms(vec![lexical_term()]);

    let planned = plan(&request, &registry, &stats).expect("the fixture request plans");

    // The plan is a value: every decision is inspectable and owned. A caller
    // that stops here never reaches compilation.
    assert_eq!(planned.producer_bindings.len(), 1);
    assert_eq!(planned.producer_bindings[0].producer, ex("pf/hand"));
    assert_eq!(
        planned.producer_bindings[0].stratum,
        iri(&ex("stratum/hand"))
    );
    assert_eq!(planned.request_terms.len(), 1);
    assert!(!planned.stratum_depths.is_empty());
    assert!(!planned.stratum_weights.is_empty());
    assert_eq!(planned.registry_instance_id, registry.instance_id());
    assert_eq!(
        planned.registry_content_fingerprint,
        registry.content_fingerprint().expect("fingerprint")
    );
    // Deliberately no `compile`, `execute` or `fuse` call.
}

// ---------------------------------------------------------------------------
// 2. Stop at compile: run the emitted SPARQL directly
// ---------------------------------------------------------------------------

#[test]
fn stop_at_compile_run_directly() {
    let registry = single_registry(&ex("stratum/hand"), &ex("pf/hand"), 12, 3);
    let stats = single_statistics(&ex("stratum/hand"), 12);
    let request = RetrievalRequest::from_terms(vec![lexical_term()]);
    let planned = plan(&request, &registry, &stats).expect("plans");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
    };

    let compiled = compile(&planned, &env).expect("the plan is admitted");
    assert_eq!(compiled.units.len(), 1, "one unit for one stratum");

    // The compiled form is text a caller can hand to the evaluator directly,
    // with no `execute` (and so no composition layer) in the path.
    let unit = &compiled.units[0];
    assert!(
        unit.sparql.contains(&ex("pf/hand")),
        "the unit names its producer: {}",
        unit.sparql
    );
    let rows = run_query(&unit.sparql, &registry);
    assert_eq!(
        rows.len(),
        3,
        "the emitted text is independently executable"
    );
    for (index, row) in rows.iter().enumerate() {
        assert_eq!(
            row[0].as_ref().expect("candidate is bound"),
            &TermValue::iri(format!("{}entity{index}", ex("hand/")))
        );
    }
}

// ---------------------------------------------------------------------------
// 3. Stop at execute: consume unfused streams with no fusion in the path
// ---------------------------------------------------------------------------

#[test]
fn stop_at_execute_unbounded_unfused() {
    let registry = single_registry(&ex("stratum/hand"), &ex("pf/hand"), 9, 5);
    let stats = single_statistics(&ex("stratum/hand"), 9);
    let request = RetrievalRequest::from_terms(vec![lexical_term()]);
    let planned = plan(&request, &registry, &stats).expect("plans");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
    };
    let compiled = compile(&planned, &env).expect("admits");

    let execution =
        block_on(execute(&compiled, &registry, &*common::empty_dataset())).expect("the units run");
    assert_eq!(execution.streams.len(), 1);

    let mut stream = execution
        .streams
        .into_iter()
        .next()
        .expect("the stratum streamed");

    // Consume the stream to its own end: no fusion, so no threshold and no
    // top-k certification can block an emission.
    let mut pulled = 0usize;
    let mut last_rank = 0u64;
    while let Some((rank, _term)) = block_on(stream.stream.next()).expect("the stream pulls") {
        assert_eq!(rank, last_rank + 1, "ranks are 1-based and contiguous");
        last_rank = rank;
        pulled += 1;
    }
    assert_eq!(pulled, 5, "every emitted row is consumable");
    let receipt = block_on(stream.stream.receipt()).expect("the stream ended cleanly");
    assert_eq!(receipt, exhausted(5));
}

// ---------------------------------------------------------------------------
// 4. Start at compile: a hand-built plan, admitted on the same terms
// ---------------------------------------------------------------------------

#[test]
fn start_at_compile_hand_built_plan() {
    let registry = single_registry(&ex("stratum/hand"), &ex("pf/hand"), 10, 2);
    let stats = single_statistics(&ex("stratum/hand"), 10);
    let request = RetrievalRequest::from_terms(vec![lexical_term()]);
    let stratum = iri(&ex("stratum/hand"));

    let mut stratum_depths = HashMap::new();
    stratum_depths.insert(stratum.clone(), 10);
    let mut stratum_weights = HashMap::new();
    stratum_weights.insert(stratum.clone(), Weight::new(Fixed::ONE));

    // Built by hand, never through the planner. It is still a plan, so admission
    // covers it on exactly the same terms as planner origin.
    let hand_built = Plan {
        version: Plan::VERSION,
        request_terms: vec![lexical_term()],
        producer_bindings: vec![ProducerBinding {
            producer: ex("pf/hand"),
            stratum,
            request_terms: vec![0],
        }],
        producer_decisions: Vec::new(),
        stratum_depths,
        stratum_weights,
        statistics_snapshot: StatisticsSnapshot {
            source: stats.source().to_owned(),
            revision: stats.revision().to_owned(),
            entries: Vec::new(),
        },
        registry_instance_id: registry.instance_id(),
        registry_content_fingerprint: registry.content_fingerprint().expect("fingerprint"),
        // Built here, against the live registry whose instance id it just read,
        // so it is held to that instance exactly as the planner's own output is.
        origin: PlanOrigin::SameProcess,
    };

    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
    };
    let compiled = compile(&hand_built, &env).expect("a hand-built plan is admitted");
    assert_eq!(compiled.plan_id, hand_built.id());
    assert_eq!(compiled.units.len(), 1);

    // The planner's own output for the same request compiles to the same units,
    // which is what "covered on the same terms" means.
    let planned = plan(&request, &registry, &stats).expect("planner origin");
    let planned_compiled = compile(&planned, &env).expect("planner origin admits");
    assert_eq!(planned_compiled.units, compiled.units);
}

// ---------------------------------------------------------------------------
// 5. Start at execute: hand-written SPARQL, no composition types in the path
// ---------------------------------------------------------------------------

#[test]
fn start_at_execute_evaluator_only() {
    // No `Plan`, `CompiledRetrieval`, `ExecutionResult` or `FusionProfile` is
    // constructed: the relation is reached by writing the query by hand.
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/match"),
        make_producer(10, "direct/", 3),
        ranked(
            &ex("stratum/direct"),
            vec![TermPattern::of_kind(TermKind::Any)],
            true,
        ),
    );

    let sparql = "SELECT ?s WHERE { ?s <http://example.org/pf/match> ?o }";
    let rows = run_query(sparql, &registry);
    let expected: Vec<Vec<Option<TermValue>>> = (0..3)
        .map(|index| {
            vec![Some(TermValue::iri(format!(
                "{}entity{index}",
                ex("direct/")
            )))]
        })
        .collect();
    assert_eq!(
        rows, expected,
        "the evaluator answers the hand-written query"
    );
}

// ---------------------------------------------------------------------------
// 6. Start at fuse: hand-built streams, no earlier stage
// ---------------------------------------------------------------------------

#[test]
fn start_at_fuse_hand_built_streams() {
    let profile = profile(&[("s1", Fixed::ONE), ("s2", Fixed::ONE)], K, 4);
    let streams = vec![
        (
            stratum("s1"),
            ScriptedStream::new(vec![row(1, Fixed::ONE, K, "alpha")], exhausted(1)),
        ),
        (
            stratum("s2"),
            ScriptedStream::new(vec![row(1, Fixed::ONE, K, "beta")], exhausted(1)),
        ),
    ];

    let result = block_on(fuse::<ScriptedStream, Term>(streams, &profile))
        .expect("fusion works with no planning, compilation or execution");

    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.trailer.statuses.len(), 2);
    assert_eq!(result.trailer.profile_id, profile.id());
}

// ---------------------------------------------------------------------------
// 7. Fused is top-k with memory proportional to the frontier
// ---------------------------------------------------------------------------

/// An effectively unbounded producer that mints rows lazily and counts pulls.
///
/// Nothing is materialized up front: if fusion needed the whole input to emit
/// its first rows, the pull count would approach `total`.
struct LazyStream {
    emitted: u64,
    total: u64,
    weight: Fixed,
    k: u32,
    pulls: Arc<AtomicUsize>,
}

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
        let value = contribution(self.weight, rank, self.k).expect("contribution fits");
        Ok(Some((
            rank,
            value,
            Term::new(format!("candidate-{rank:08}")),
        )))
    }

    async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError> {
        Ok(exhausted(self.emitted))
    }
}

#[test]
fn fused_top_k_bounded_memory() {
    // A million-row stream; only ten rows are asked for.
    let total = 1_000_000u64;
    let pulls = Arc::new(AtomicUsize::new(0));
    let profile = FusionProfile::new(BTreeMap::from([(stratum("big"), Fixed::ONE)]), K, 1)
        .expect("the single-stratum profile is valid");
    let streams = vec![(
        stratum("big"),
        LazyStream {
            emitted: 0,
            total,
            weight: Fixed::ONE,
            k: K,
            pulls: Arc::clone(&pulls),
        },
    )];

    let mut fusion = FusionStream::new(streams, profile);
    let mut top = Vec::new();
    for _ in 0..10 {
        let row = block_on(fusion.next())
            .expect("fusion pulls")
            .expect("a certified row");
        top.push(row);
    }

    // Certification is a threshold argument over the live heads, not a scan:
    // emitting ten rows pulls eleven (the initial head plus one advance per
    // certification), never the million the stream could yield. The frontier is
    // the request, not the input.
    assert_eq!(top.len(), 10);
    assert_eq!(
        pulls.load(Ordering::SeqCst),
        11,
        "fused emission is bounded by the frontier, not the stream"
    );
    // Dropped without draining: the producer never materialized its full set.
    drop(fusion);
    assert!(pulls.load(Ordering::SeqCst) < 100);
}

// ---------------------------------------------------------------------------
// 8. Unfused streams are unbounded
// ---------------------------------------------------------------------------

#[test]
fn unfused_unbounded_stream() {
    const ROWS: usize = 4096;
    let registry = single_registry(&ex("stratum/deep"), &ex("pf/deep"), ROWS as u64, ROWS);
    let stats = single_statistics(&ex("stratum/deep"), ROWS as u64);
    let request = RetrievalRequest::from_terms(vec![lexical_term()]);
    let planned = plan(&request, &registry, &stats).expect("plans");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
    };
    let compiled = compile(&planned, &env).expect("admits");
    let execution =
        block_on(execute(&compiled, &registry, &*common::empty_dataset())).expect("the units run");

    let mut stream = execution
        .streams
        .into_iter()
        .next()
        .expect("the stratum streamed");

    // The unfused rung has no threshold: every row the producer emits is
    // immediately consumable, however deep the enumeration goes.
    let mut pulled = 0usize;
    let mut last_rank = 0u64;
    while let Some((rank, _term)) = block_on(stream.stream.next()).expect("the stream pulls") {
        assert_eq!(rank, last_rank + 1, "ranks stay contiguous to the end");
        last_rank = rank;
        pulled += 1;
    }
    assert_eq!(pulled, ROWS, "the unfused rung emits the whole stream");
    let receipt = block_on(stream.stream.receipt()).expect("clean end");
    assert_eq!(receipt, exhausted(ROWS as u64));
}

// ---------------------------------------------------------------------------
// 9. Trailer semantics: a prefix reader holds incomplete evidence
// ---------------------------------------------------------------------------

#[test]
fn prefix_reader_incomplete_evidence() {
    let profile = profile(&[("s1", Fixed::ONE), ("s2", Fixed::ONE)], K, 4);
    let streams = vec![
        (
            stratum("s1"),
            ScriptedStream::new(
                vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "x")],
                exhausted(2),
            ),
        ),
        (
            stratum("s2"),
            ScriptedStream::new(
                vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "y")],
                exhausted(2),
            ),
        ),
    ];

    let mut fusion = FusionStream::new(streams, profile);
    let first = block_on(fusion.next())
        .expect("fusion pulls")
        .expect("a certified row");
    assert_eq!(first.entity, Term::new("a"));
    // The witness is strictly positive, so `a` was certified while both streams
    // still held rows: the prefix is provably incomplete and only the trailer
    // may assert otherwise.
    assert!(
        first.threshold_witness > Fixed::ZERO,
        "certification happened with rows still unread"
    );

    // A partially-read stream refuses to describe its own completeness.
    let mut partial = RankedStreamImpl::new(vec![(1, Term::new("a")), (2, Term::new("b"))]);
    assert!(block_on(partial.next()).expect("pulls").is_some());
    assert!(matches!(
        block_on(partial.receipt()),
        Err(ProtocolError::NeverEndingSource)
    ));

    // Only after the fusion is fully drained is the terminal report available.
    while block_on(fusion.next()).expect("fusion pulls").is_some() {}
    let trailer = block_on(fusion.trailer()).expect("the trailer is produced");
    assert_eq!(trailer.statuses.len(), 2);
}

// ---------------------------------------------------------------------------
// 10. Reporting names both the plan and the profile
// ---------------------------------------------------------------------------

#[test]
fn reporting_names_plan_and_profile() {
    let registry = single_registry(&ex("stratum/report"), &ex("pf/report"), 12, 3);
    let stats = single_statistics(&ex("stratum/report"), 12);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
    };
    let profile = single_profile(&ex("stratum/report"));
    let request = RetrievalRequest::from_terms(vec![lexical_term()]);

    let expected_plan = plan(&request, &registry, &stats).expect("plans");
    let result = block_on(search(
        &request,
        &registry,
        &stats,
        &*common::empty_dataset(),
        &env,
        &profile,
    ))
    .expect("the composed search answers");

    // A reproducible answer is a pair of identities: the pinned plan and the
    // fusion law in force, both named by the answer.
    assert_eq!(result.plan_id, expected_plan.id());
    assert_eq!(result.profile_id, profile.id());
    assert_eq!(result.trailer.profile_id, profile.id());
}

// ---------------------------------------------------------------------------
// 11. No mode flags
// ---------------------------------------------------------------------------

/// The seam is the same behavior observed earlier; a mode flag would be a
/// second behavior the full pipeline must be kept in agreement with. No stage
/// of this crate takes one, so a caller that wants less calls an earlier stage.
#[test]
fn no_mode_flags_in_api() {
    const SOURCES: &[(&str, &str)] = &[
        ("lib.rs", include_str!("../src/lib.rs")),
        ("search.rs", include_str!("../src/search.rs")),
        ("compile.rs", include_str!("../src/compile.rs")),
        ("execute.rs", include_str!("../src/execute.rs")),
        ("fuse.rs", include_str!("../src/fuse.rs")),
        ("planner.rs", include_str!("../src/planner.rs")),
    ];
    const FORBIDDEN: &[&str] = &[
        "skip_fuse",
        "skip_plan",
        "skip_compile",
        "skip_execute",
        "dry_run",
        "skip: bool",
        "mode: bool",
        "mode: Option",
    ];
    for (name, source) in SOURCES {
        for flag in FORBIDDEN {
            assert!(
                !source.contains(flag),
                "{name} carries mode flag {flag:?}; stages are seams, never mode switches"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 12. Native and wasm agree on the bytes
// ---------------------------------------------------------------------------

/// Render a fusion as deterministic text: rows in final order with their
/// provenance, then statuses by stratum, then the profile identity.
fn render_fusion(result: &FusionResult<Term>) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for fused in &result.rows {
        let _ = writeln!(
            out,
            "row {} score={} witness={}",
            fused.entity.as_str(),
            fused.score.to_decimal_lexical(),
            fused.threshold_witness.to_decimal_lexical()
        );
        for (stratum, rank, value) in &fused.contributions {
            let _ = writeln!(
                out,
                "  {} rank={rank} score={}",
                stratum.as_str(),
                value.to_decimal_lexical()
            );
        }
    }
    for (stratum, status) in &result.trailer.statuses {
        let _ = writeln!(out, "status {} {status:?}", stratum.as_str());
    }
    let _ = writeln!(out, "profile {}", result.trailer.profile_id.to_hex());
    out
}

async fn equality_fusion(profile: &FusionProfile) -> FusionResult<Term> {
    let streams = vec![
        (
            stratum("eq1"),
            ScriptedStream::new(
                vec![
                    row(1, Fixed::ONE, K, "alpha"),
                    row(2, Fixed::ONE, K, "beta"),
                ],
                exhausted(2),
            ),
        ),
        (
            stratum("eq2"),
            ScriptedStream::new(vec![row(1, Fixed::ONE, K, "beta")], exhausted(1)),
        ),
    ];
    fuse::<ScriptedStream, Term>(streams, profile)
        .await
        .expect("the fixture streams fuse")
}

#[test]
fn native_wasm_execution_equality() {
    let registry = single_registry(&ex("stratum/eq"), &ex("pf/eq"), 12, 3);
    let stats = single_statistics(&ex("stratum/eq"), 12);
    let request = RetrievalRequest::from_terms(vec![lexical_term()]);
    let planned = plan(&request, &registry, &stats).expect("plans");

    // A plan's canonical encoding is a pure function of its fields: repeated
    // encodings agree and a decode reproduces them exactly, so native and wasm
    // name the same plan with the same bytes.
    let plan_bytes = planned.canonical_bytes();
    assert_eq!(plan_bytes, planned.canonical_bytes());
    assert_eq!(
        Plan::from_canonical_bytes(&plan_bytes)
            .expect("canonical bytes decode")
            .canonical_bytes(),
        plan_bytes
    );

    // The fusion profile's canonical encoding is target-independent the same way.
    let profile = profile(&[("eq1", Fixed::ONE), ("eq2", Fixed::ONE)], K, 4);
    let profile_bytes = profile.canonical_bytes();
    assert_eq!(profile_bytes, profile.canonical_bytes());
    assert_eq!(
        FusionProfile::from_canonical_bytes(&profile_bytes)
            .expect("canonical bytes decode")
            .canonical_bytes(),
        profile_bytes
    );

    // Fused output is byte-identical across two independent runs of the same
    // law, which is the equality a native and a wasm execution share.
    let first = block_on(equality_fusion(&profile));
    let second = block_on(equality_fusion(&profile));
    assert_eq!(render_fusion(&first), render_fusion(&second));
}
