// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Search composition identity: `search` is exactly
//! `fuse ∘ execute ∘ compile ∘ plan`.
//!
//! The differential test drives the stage functions by hand with the same mock
//! registry, statistics and profile, and requires the composed `search` answer to
//! be byte-identical to that manual composition. The error tests pin each stage's
//! typed refusal to the matching [`SearchError`] variant, and the end-to-end test
//! proves a full mock search returns the expected rows and trailer. Fixtures use
//! `example.org` throughout.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use pretty_assertions::assert_eq;
use purrdf_core::{RdfDataset, TermValue};
use purrdf_retrieval::{
    AdmissionEnvironment, AdmissionError, ExecutionError, Fixed, FusionError, FusionProfile, Iri,
    Metric, PlanError, ProducerReceipt, ProtocolError, RankedStream, RankedStreamImpl, RequestTerm,
    RetrievalRequest, SearchError, SearchResult, Statistics, Term, compile, contribution, execute,
    fuse, plan, search,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, DuplicatePolicy, EvalError, PfArgs, PfArity, PfCursor, PfRow,
    PropertyFunction, PropertyFunctionRegistry, RankOrdering, RankedDeclaration, RequestFacet,
    TermKind, TermPattern, TermPlacement, Volatility,
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
fn ranked(stratum: &str, patterns: Vec<TermPattern>, mandatory: bool) -> RankedDeclaration {
    RankedDeclaration {
        stratum: kernel_iri(stratum),
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

fn vector_term() -> RequestTerm {
    RequestTerm::Vector {
        embedding: vec![0.25, -1.5, 3.0],
        metric: Metric::Cosine,
        index_hint: None,
    }
}

fn seed_term() -> RequestTerm {
    RequestTerm::EntitySeed {
        entity: Term::new(format!("<{}>", ex("seed"))),
    }
}

fn mixed_request() -> RetrievalRequest {
    RetrievalRequest::from_terms(vec![lexical_term(), vector_term(), seed_term()])
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

fn producer(rows: u64, prefix: &str, count: usize) -> Arc<dyn PropertyFunction> {
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

/// The fixture registry: a catch-all producer the host declares mandatory, a
/// literal producer, an IRI-seed producer, and one unranked producer.
fn fixture_registry() -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    let literal_pattern = TermPattern {
        kind: TermKind::Literal,
        datatype: None,
        language: Some("en".to_owned()),
        predicate: Some(ex("body")),
    };
    registry.register_ranked(
        ex("pf/any"),
        producer(200, "universal/", 3),
        ranked(
            &ex("stratum/universal"),
            vec![TermPattern::of_kind(TermKind::Any)],
            true,
        ),
    );
    registry.register_ranked(
        ex("pf/literal"),
        producer(100, "text/", 2),
        ranked(&ex("stratum/text"), vec![literal_pattern], false),
    );
    registry.register_ranked(
        ex("pf/iri"),
        producer(50, "graph/", 1),
        ranked(
            &ex("stratum/graph"),
            vec![TermPattern::of_kind(TermKind::Iri)],
            false,
        ),
    );
    // Registered with no declaration at all: that is the whole of "does not
    // participate in ranked retrieval".
    registry.register(ex("pf/not-ranked"), producer(0, "unranked/", 0));
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

fn statistics(revision: &str) -> MockStatistics {
    let mut cardinalities = BTreeMap::new();
    cardinalities.insert(iri(&ex("stratum/universal")), 1000);
    cardinalities.insert(iri(&ex("stratum/text")), 100);
    cardinalities.insert(iri(&ex("stratum/graph")), 50);
    cardinalities.insert(iri(&ex("body")), 500);
    MockStatistics {
        source: "example-statistics".to_owned(),
        revision: revision.to_owned(),
        cardinalities,
    }
}

/// The fixture fusion profile: unit weight for each of the fixture's three
/// strata, smoothing `K`, and room for a candidate to surface in every stratum.
fn fixture_profile() -> FusionProfile {
    let mut weights = BTreeMap::new();
    weights.insert(iri(&ex("stratum/universal")), Fixed::ONE);
    weights.insert(iri(&ex("stratum/text")), Fixed::ONE);
    weights.insert(iri(&ex("stratum/graph")), Fixed::ONE);
    FusionProfile::new(weights, K, 4).expect("the fixture profile is valid")
}

fn fixture_env<'a>(
    registry: &'a PropertyFunctionRegistry,
    stats: &'a MockStatistics,
) -> AdmissionEnvironment<'a> {
    AdmissionEnvironment {
        registry,
        statistics: stats,
    }
}

// ---------------------------------------------------------------------------
// A minimal single-threaded executor (the mock never actually pends)
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

// ---------------------------------------------------------------------------
// The manual bridge, so the composition can be driven by hand
// ---------------------------------------------------------------------------

/// The test's own copy of the production bridge: the executor's materialized
/// `(rank, candidate)` rows plus the profile's reciprocal-rank contribution. The
/// differential test needs an adapter it can name, and re-deriving it here is
/// what makes the identity check honest — the composed answer must match the
/// hand-composed one even though each uses its own bridge.
struct Bridge {
    inner: RankedStreamImpl,
    weight: Fixed,
    k: u32,
}

impl RankedStream for Bridge {
    type Item = Term;

    async fn next(&mut self) -> Result<Option<(u64, Fixed, Self::Item)>, ProtocolError> {
        let Some((rank, item)) = self.inner.next().await? else {
            return Ok(None);
        };
        Ok(Some((
            rank,
            contribution(self.weight, rank, self.k).expect("valid profile and 1-based rank"),
            item,
        )))
    }

    async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError> {
        self.inner.receipt().await
    }
}

/// Drive `fuse ∘ execute ∘ compile ∘ plan` by hand and assemble the same
/// [`SearchResult`] `search` would.
// The manual composition mirrors `search`, including its single-task,
// runtime-agnostic future; see the same allow on `search` itself.
#[allow(clippy::future_not_send)]
async fn manual_composition(
    request: &RetrievalRequest,
    registry: &PropertyFunctionRegistry,
    stats: &MockStatistics,
    dataset: &RdfDataset,
    env: &AdmissionEnvironment<'_>,
    profile: &FusionProfile,
) -> SearchResult {
    let planned = plan(request, registry, stats).expect("the fixture request plans");
    let compiled = compile(&planned, env).expect("a fresh plan is admitted");
    let execution = execute(&compiled, registry, dataset)
        .await
        .expect("the fixture registry executes");

    let streams = execution
        .streams
        .into_iter()
        .map(|stream| {
            let weight = profile
                .weight(&stream.stratum)
                .expect("the fixture profile weights every stratum");
            (
                stream.stratum,
                Bridge {
                    inner: stream.stream,
                    weight,
                    k: profile.k_parameter(),
                },
            )
        })
        .collect();

    let fused = fuse::<Bridge, Term>(streams, profile)
        .await
        .expect("the surviving streams fuse");
    SearchResult {
        rows: fused.rows,
        trailer: fused.trailer,
        plan_id: planned.id(),
        profile_id: profile.id(),
    }
}

/// Render a [`SearchResult`] as deterministic bytes: rows in final order with
/// their provenance, then statuses by stratum, then the pinned identities.
/// Nothing here depends on a `HashMap`.
fn render(result: &SearchResult) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "plan {}", result.plan_id.to_hex());
    let _ = writeln!(out, "profile {}", result.profile_id.to_hex());
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
    let _ = writeln!(
        out,
        "trailer-profile {}",
        result.trailer.profile_id.to_hex()
    );
    out
}

// ---------------------------------------------------------------------------
// 1. Composition identity
// ---------------------------------------------------------------------------

#[test]
fn search_equals_manual_composition() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let profile = fixture_profile();
    let request = mixed_request();
    let dataset = common::empty_dataset();

    let direct = block_on(search(
        &request, &registry, &stats, &*dataset, &env, &profile,
    ))
    .expect("the composed search answers");
    let manual = block_on(manual_composition(
        &request, &registry, &stats, &dataset, &env, &profile,
    ));

    assert_eq!(
        render(&direct).as_bytes(),
        render(&manual).as_bytes(),
        "search must be byte-for-byte the manual composition"
    );
    assert_eq!(direct, manual, "and structurally the same answer");
}

// ---------------------------------------------------------------------------
// 2. Error propagation
// ---------------------------------------------------------------------------

#[test]
fn plan_error_propagates() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let profile = fixture_profile();

    // An empty request reaches no producer, so planning refuses it.
    let error = block_on(search(
        &RetrievalRequest::new(),
        &registry,
        &stats,
        &*common::empty_dataset(),
        &env,
        &profile,
    ))
    .expect_err("an empty request does not plan");
    assert!(
        matches!(
            error,
            SearchError::PlanError(PlanError::NoApplicableProducers)
        ),
        "expected the planner's refusal, got {error:?}"
    );
}

#[test]
fn admission_error_propagates() {
    // Two registries with identical declarations share a content fingerprint but
    // have distinct live instance identities; planning against one and admitting
    // against the other is the admission refusal.
    let planned_against = fixture_registry();
    let admitted_against = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&admitted_against, &stats);
    let profile = fixture_profile();

    let error = block_on(search(
        &mixed_request(),
        &planned_against,
        &stats,
        &*common::empty_dataset(),
        &env,
        &profile,
    ))
    .expect_err("a different live registry instance is refused at admission");
    assert!(
        matches!(
            error,
            SearchError::AdmissionError(AdmissionError::RegistryMismatch { .. })
        ),
        "expected an admission refusal, got {error:?}"
    );
}

#[test]
fn execution_error_maps_to_search_error() {
    // Admission pins both the planner's registry and the executor's registry to
    // the environment's live instance, so a whole-execution failure is not
    // reachable end-to-end through `search` — the mapping itself is what carries
    // `execute`'s error out unchanged when `execute` does refuse.
    let first = fixture_registry();
    let second = fixture_registry();
    let error: SearchError = ExecutionError::RegistryMismatch {
        expected: first.instance_id(),
        got: second.instance_id(),
    }
    .into();
    assert!(
        matches!(error, SearchError::ExecutionError(_)),
        "the executor's error propagates unchanged, got {error:?}"
    );
}

#[test]
fn fusion_error_propagates() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    // A profile that weights only one of the executor's three strata cannot
    // fuse the others.
    let narrow = FusionProfile::new(
        BTreeMap::from([(iri(&ex("stratum/text")), Fixed::ONE)]),
        K,
        4,
    )
    .expect("the narrow profile is valid");

    let error = block_on(search(
        &mixed_request(),
        &registry,
        &stats,
        &*common::empty_dataset(),
        &env,
        &narrow,
    ))
    .expect_err("a profile missing a stratum is refused at fusion");
    match error {
        SearchError::FusionError(FusionError::UnknownStratum { stratum }) => {
            // `compile` orders units by stratum IRI, so `stratum/graph` is the
            // first stream and the first stratum the profile does not weight.
            assert_eq!(
                stratum,
                ex("stratum/graph"),
                "the undeclared stratum is named exactly"
            );
        }
        other => panic!("expected a fusion refusal, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 3. End to end
// ---------------------------------------------------------------------------

#[test]
fn end_to_end_search_returns_expected_results() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let profile = fixture_profile();
    let request = mixed_request();

    let result = block_on(search(
        &request,
        &registry,
        &stats,
        &*common::empty_dataset(),
        &env,
        &profile,
    ))
    .expect("the fixture search answers");
    let expected_plan = plan(&request, &registry, &stats).expect("the request plans");

    assert_eq!(result.plan_id, expected_plan.id());
    assert_eq!(result.profile_id, profile.id());
    assert_eq!(result.trailer.profile_id, profile.id());
    assert_eq!(
        result.rows.len(),
        6,
        "3 universal + 2 text + 1 graph distinct candidates"
    );

    // Order is score-descending and each row's score is its provenance's sum.
    for pair in result.rows.windows(2) {
        assert!(
            pair[0].score >= pair[1].score,
            "rows are ordered score-descending"
        );
    }
    for row in &result.rows {
        assert!(
            !row.contributions.is_empty(),
            "every emitted row has provenance"
        );
        let sum = row
            .contributions
            .iter()
            .fold(Fixed::ZERO, |total, (_, _, value)| {
                total.checked_add(*value).expect("no fixed-point overflow")
            });
        assert_eq!(sum, row.score, "contributions sum to the fused score");
    }

    // The trailer reports each contributing stratum's own status.
    assert_eq!(result.trailer.statuses.len(), 3);
    assert!(matches!(
        result.trailer.statuses.get(&iri(&ex("stratum/universal"))),
        Some(purrdf_retrieval::ProducerStatus::Exhausted { rows_emitted: 3 })
    ));
    assert!(matches!(
        result.trailer.statuses.get(&iri(&ex("stratum/text"))),
        Some(purrdf_retrieval::ProducerStatus::Exhausted { rows_emitted: 2 })
    ));
    assert!(matches!(
        result.trailer.statuses.get(&iri(&ex("stratum/graph"))),
        Some(purrdf_retrieval::ProducerStatus::Exhausted { rows_emitted: 1 })
    ));
}
