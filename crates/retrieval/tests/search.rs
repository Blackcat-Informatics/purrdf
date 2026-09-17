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
    Metric, PlanError, ProducerStatus, RankedStreamAdapter, RequestTerm, RetrievalRequest,
    SearchError, SearchResult, Statistics, Term, TopK, UnservedReason, UnservedTerm, compile,
    execute, fuse, plan, search,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, DuplicatePolicy, EvalError, PfArgs, PfArity, PfCursor, PfRow,
    PropertyFunction, PropertyFunctionRegistry, RankOrdering, RankedDeclaration, RequestFacet,
    TermKind, TermPattern, TermPlacement, Volatility,
};

mod common;

const K: u32 = 60;

/// The row bound these fixtures search under.
///
/// Fused enumeration is top-k by construction, so every search states a bound.
/// This one is well above what the mocks can yield, so nothing below is decided
/// by the bound; `the_bound_is_the_bound_and_a_larger_one_refuses_nothing` is
/// where the bound's own behaviour is pinned.
const TOP_K: TopK = TopK::new(1024);

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
        fusion_profile: None,
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
// The hand-composed pipeline, assembled from the exported seam
// ---------------------------------------------------------------------------

/// Drive `fuse ∘ execute ∘ compile ∘ plan` by hand and assemble the same
/// [`SearchResult`] `search` would.
///
/// Everything a caller cannot reasonably re-derive is taken from the crate's own
/// exports: [`RankedStreamAdapter`] bridges each executed stream to the fusion
/// protocol, and [`purrdf_retrieval::FusionTrailer::completed_with`] carries the
/// executor's statuses into the terminal report. Re-deriving either here would
/// make this test compare `search` against a copy of itself — the drift a seam
/// exists to prevent — instead of against the pipeline a caller can write.
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
    top_k: TopK,
) -> SearchResult {
    let planned = plan(request, registry, stats).expect("the fixture request plans");
    let compiled = compile(&planned, env).expect("a fresh plan is admitted");
    let execution = execute(&compiled, registry, dataset)
        .await
        .expect("the fixture registry executes");

    let mut streams = Vec::new();
    let mut unweighted_strata = Vec::new();
    for stream in execution.streams {
        match RankedStreamAdapter::new(stream.stream, profile, &stream.stratum) {
            Some(adapter) => streams.push((stream.stratum, adapter)),
            None => unweighted_strata.push(stream.stratum),
        }
    }
    unweighted_strata.sort();

    let fused = fuse::<RankedStreamAdapter, Term>(streams, profile, top_k)
        .await
        .expect("the surviving streams fuse");
    SearchResult {
        rows: fused.rows,
        trailer: fused.trailer.completed_with(execution.statuses),
        unserved_terms: planned.unserved_evidence(),
        plan_id: planned.id(),
        profile_id: profile.id(),
        unweighted_strata,
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
    for stratum in &result.unweighted_strata {
        let _ = writeln!(out, "unweighted {}", stratum.as_str());
    }
    for unserved in &result.unserved_terms {
        let _ = writeln!(
            out,
            "unserved term={} {:?}",
            unserved.request_term, unserved.reason
        );
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
        &request, &registry, &stats, &*dataset, &env, &profile, TOP_K,
    ))
    .expect("the composed search answers");
    let manual = block_on(manual_composition(
        &request, &registry, &stats, &dataset, &env, &profile, TOP_K,
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
        TOP_K,
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
        TOP_K,
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
    // A profile whose only weight names a stratum this plan never reaches. Every
    // stratum that ran is unweighted, so nothing that ran can contribute and
    // there is no partial answer to return.
    let disjoint = FusionProfile::new(
        BTreeMap::from([(iri(&ex("stratum/elsewhere")), Fixed::ONE)]),
        K,
        4,
    )
    .expect("the disjoint profile is valid");

    let error = block_on(search(
        &mixed_request(),
        &registry,
        &stats,
        &*common::empty_dataset(),
        &env,
        &disjoint,
        TOP_K,
    ))
    .expect_err("a profile that weights none of the executed strata is refused");
    match error {
        SearchError::FusionError(FusionError::UnknownStratum { stratum }) => {
            // Reported in ascending stratum order, so `stratum/graph` is named.
            assert_eq!(
                stratum,
                ex("stratum/graph"),
                "the unweighted stratum is named exactly"
            );
        }
        other => panic!("expected a fusion refusal, got {other:?}"),
    }
}

#[test]
fn search_holds_the_plan_to_the_profile_it_is_about_to_fuse_under() {
    // `search` is the one place the plan and the law meet before a row is read,
    // so it re-forms the admission environment around that law. The caller's own
    // environment names no profile here, and the refusal still arrives: a
    // per-stratum depth of a hundred outruns what a weight of `10^-9` can order.
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let shallow = FusionProfile::new(
        BTreeMap::from([(iri(&ex("stratum/text")), Fixed::from_raw(1_000))]),
        1,
        4,
    )
    .expect("a strictly positive weight is a valid profile");

    let error = block_on(search(
        &mixed_request(),
        &registry,
        &stats,
        &*common::empty_dataset(),
        &env,
        &shallow,
        TOP_K,
    ))
    .expect_err("the depth outruns the profile's own arithmetic");
    match error {
        SearchError::AdmissionError(AdmissionError::DepthBeyondMonotoneRange {
            stratum,
            monotone,
            requested,
        }) => {
            assert_eq!(*stratum, iri(&ex("stratum/text")));
            assert_eq!(
                monotone,
                shallow
                    .monotone_depth(&iri(&ex("stratum/text")))
                    .expect("weighted"),
            );
            assert_eq!(requested, 100);
        }
        other => panic!("expected DepthBeyondMonotoneRange, got {other:?}"),
    }

    // The neighbouring valid case: the fixture profile's unit weights order far
    // deeper than any depth this plan records, and the same search answers.
    assert!(
        block_on(search(
            &mixed_request(),
            &registry,
            &stats,
            &*common::empty_dataset(),
            &env,
            &fixture_profile(),
            TOP_K,
        ))
        .is_ok(),
        "a profile whose arithmetic covers every recorded depth still answers"
    );
}

// ---------------------------------------------------------------------------
// 2b. A profile that weights only some of the plan's strata
//
// The profile is deliberately not a planning input, so a plan's strata and a
// profile's weights are two independent lists that can legitimately disagree.
// The pair below is the whole policy: a partial overlap answers from the strata
// the profile weights and names the rest, and only a disjoint profile refuses.
// Refusing the partial case too would throw away every stratum the profile did
// weight in order to report one it did not.
// ---------------------------------------------------------------------------

#[test]
fn a_profile_that_weights_some_strata_answers_from_those_and_names_the_rest() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    // Weights one of the three strata the plan reaches. The other two ran and
    // produced rows; they have no weight, so they have no contribution.
    let narrow = FusionProfile::new(
        BTreeMap::from([(iri(&ex("stratum/text")), Fixed::ONE)]),
        K,
        4,
    )
    .expect("the narrow profile is valid");

    let result = block_on(search(
        &mixed_request(),
        &registry,
        &stats,
        &*common::empty_dataset(),
        &env,
        &narrow,
        TOP_K,
    ))
    .expect("the weighted stratum still answers");

    assert_eq!(
        result.unweighted_strata,
        vec![iri(&ex("stratum/graph")), iri(&ex("stratum/universal"))],
        "the strata the profile does not weight are reported, in stratum order"
    );
    assert_eq!(
        result.rows.len(),
        2,
        "the text producer's two rows are the whole answer"
    );
    for row in &result.rows {
        for (stratum, _, _) in &row.contributions {
            assert_eq!(
                *stratum,
                iri(&ex("stratum/text")),
                "only the weighted stratum contributes"
            );
        }
    }
    // Every stratum that ran has a status, including the two the profile did not
    // weight: they answered, and "the profile did not score this" is not the
    // same fact as "this could not answer". The fusion verified one of them; the
    // executor's own report carries the other two into the trailer.
    assert_eq!(
        result.trailer.statuses.len(),
        3,
        "an unweighted stratum still ran, and the trailer says how it ended"
    );
    assert!(matches!(
        result.trailer.statuses.get(&iri(&ex("stratum/universal"))),
        Some(ProducerStatus::Exhausted { rows_emitted: 3 })
    ));
}

#[test]
fn a_profile_that_weights_every_stratum_sets_nothing_aside() {
    // The neighbouring valid case of the refusal above and of the report above:
    // when the profile covers the plan, the report is empty and every stratum
    // answers.
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let profile = fixture_profile();

    let result = block_on(search(
        &mixed_request(),
        &registry,
        &stats,
        &*common::empty_dataset(),
        &env,
        &profile,
        TOP_K,
    ))
    .expect("the fixture search answers");
    assert!(
        result.unweighted_strata.is_empty(),
        "nothing is set aside when the profile weights every stratum, got {:?}",
        result.unweighted_strata
    );
    assert_eq!(result.trailer.statuses.len(), 3);
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
        TOP_K,
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
        Some(ProducerStatus::Exhausted { rows_emitted: 3 })
    ));
    assert!(matches!(
        result.trailer.statuses.get(&iri(&ex("stratum/text"))),
        Some(ProducerStatus::Exhausted { rows_emitted: 2 })
    ));
    assert!(matches!(
        result.trailer.statuses.get(&iri(&ex("stratum/graph"))),
        Some(ProducerStatus::Exhausted { rows_emitted: 1 })
    ));
}

// ---------------------------------------------------------------------------
// 4. A stratum that could not answer is named beside the ones that did
//
// §6 of the design record: a fused answer carries the status of every producer
// that contributed AND of every applicable producer that could not, never
// reduced to one aggregate flag. Fusion is where that information can die,
// because a failed stratum has no stream to fuse — so the answer is where it
// must be found.
// ---------------------------------------------------------------------------

/// A ranked producer that cannot run at all: it refuses to open a cursor.
///
/// This is the stratum-level failure the executor isolates — every other
/// stratum still runs — and the status a caller must not have to go back to
/// `execute` to discover.
struct FailingProducer {
    arity: PfArity,
    mode: BindingPattern,
}

impl PropertyFunction for FailingProducer {
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
        10
    }

    fn open(
        &self,
        _args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        Err(EvalError::data("the fixture index is unavailable"))
    }
}

/// The fixture registry plus one producer whose stratum cannot run.
fn registry_with_a_failing_stratum() -> PropertyFunctionRegistry {
    let mut registry = fixture_registry();
    let arity = PfArity::new(1, 1);
    registry.register_ranked(
        ex("pf/broken"),
        Arc::new(FailingProducer {
            arity,
            mode: arity.all_free_mode(),
        }),
        ranked(
            &ex("stratum/broken"),
            vec![TermPattern {
                kind: TermKind::Literal,
                datatype: None,
                language: Some("en".to_owned()),
                predicate: Some(ex("body")),
            }],
            false,
        ),
    );
    registry
}

/// The fixture profile, plus a weight for the stratum that cannot run: the
/// profile declares it applicable, which is what makes its silence a fact about
/// the producer rather than about the law.
fn profile_with_the_failing_stratum() -> FusionProfile {
    let mut weights = BTreeMap::new();
    weights.insert(iri(&ex("stratum/universal")), Fixed::ONE);
    weights.insert(iri(&ex("stratum/text")), Fixed::ONE);
    weights.insert(iri(&ex("stratum/graph")), Fixed::ONE);
    weights.insert(iri(&ex("stratum/broken")), Fixed::ONE);
    FusionProfile::new(weights, K, 4).expect("the fixture profile is valid")
}

fn statistics_with_the_failing_stratum() -> MockStatistics {
    let mut stats = statistics("r1");
    stats.cardinalities.insert(iri(&ex("stratum/broken")), 10);
    stats
}

#[test]
fn a_stratum_that_could_not_answer_is_named_in_the_trailer_beside_those_that_did() {
    let registry = registry_with_a_failing_stratum();
    let stats = statistics_with_the_failing_stratum();
    let env = fixture_env(&registry, &stats);
    let profile = profile_with_the_failing_stratum();

    let result = block_on(search(
        &mixed_request(),
        &registry,
        &stats,
        &*common::empty_dataset(),
        &env,
        &profile,
        TOP_K,
    ))
    .expect("the strata that can answer still answer");

    // Per producer, typed, never a flag: three succeeded and one could not, and
    // the answer says which is which.
    assert_eq!(
        result.trailer.statuses.len(),
        4,
        "every stratum the plan reached has its own status: {:?}",
        result.trailer.statuses
    );
    match result.trailer.statuses.get(&iri(&ex("stratum/broken"))) {
        Some(ProducerStatus::ExecutionFailed { reason }) => assert!(
            reason.contains("unavailable"),
            "the producer's own reason is carried verbatim: {reason}"
        ),
        other => panic!("expected the failed stratum's own status, got {other:?}"),
    }
    for stratum in ["stratum/universal", "stratum/text", "stratum/graph"] {
        assert!(
            matches!(
                result.trailer.statuses.get(&iri(&ex(stratum))),
                Some(ProducerStatus::Exhausted { .. })
            ),
            "{stratum} answered and says so: {:?}",
            result.trailer.statuses.get(&iri(&ex(stratum)))
        );
    }
    assert!(
        !result.rows.is_empty(),
        "the strata that could answer still produced the answer"
    );
    assert!(
        result
            .rows
            .iter()
            .flat_map(|row| &row.contributions)
            .all(|(stratum, _, _)| *stratum != iri(&ex("stratum/broken"))),
        "and the stratum that could not answer contributed no rows"
    );
}

#[test]
fn every_contributor_is_named_when_none_fail() {
    // The neighbouring valid case: with nothing failing, the same report names
    // every stratum as having answered. "Three answered" and "three answered and
    // a fourth could not" are different answers, read from the same place.
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let profile = fixture_profile();

    let result = block_on(search(
        &mixed_request(),
        &registry,
        &stats,
        &*common::empty_dataset(),
        &env,
        &profile,
        TOP_K,
    ))
    .expect("the fixture search answers");

    assert_eq!(result.trailer.statuses.len(), 3);
    assert!(
        result
            .trailer
            .statuses
            .values()
            .all(|status| matches!(status, ProducerStatus::Exhausted { .. })),
        "every contributor answered: {:?}",
        result.trailer.statuses
    );
}

// ---------------------------------------------------------------------------
// 5. A request term that reached no producer
// ---------------------------------------------------------------------------

/// A registry with one literal producer and nothing else, so a vector term in
/// the same request reaches no declaration at all.
fn literal_only_registry() -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/literal"),
        producer(100, "text/", 2),
        ranked(
            &ex("stratum/text"),
            vec![TermPattern {
                kind: TermKind::Literal,
                datatype: None,
                language: Some("en".to_owned()),
                predicate: Some(ex("body")),
            }],
            false,
        ),
    );
    registry
}

fn text_only_profile() -> FusionProfile {
    FusionProfile::new(
        BTreeMap::from([(iri(&ex("stratum/text")), Fixed::ONE)]),
        K,
        4,
    )
    .expect("the fixture profile is valid")
}

#[test]
fn a_term_that_reached_no_producer_is_visible_in_the_plan_and_in_the_answer() {
    let registry = literal_only_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let profile = text_only_profile();
    // The lexical term reaches the one producer; the vector term reaches
    // nothing, because no declaration in this registry accepts that shape.
    let request = RetrievalRequest::from_terms(vec![lexical_term(), vector_term()]);

    let planned = plan(&request, &registry, &stats).expect("the lexical term plans");
    assert_eq!(
        planned.unserved_terms,
        vec![UnservedTerm {
            request_term: 1,
            reason: UnservedReason::NoProducerAccepts,
        }],
        "the plan records the term nothing accepted"
    );

    let result = block_on(search(
        &request,
        &registry,
        &stats,
        &*common::empty_dataset(),
        &env,
        &profile,
        TOP_K,
    ))
    .expect("the served term still answers");

    assert_eq!(
        result.unserved_terms,
        vec![UnservedTerm {
            request_term: 1,
            reason: UnservedReason::NoProducerAccepts,
        }],
        "and the answer carries it, typed, rather than omitting it quietly"
    );
    // The valid neighbour, in the same answer: the term that DID reach a
    // producer is not reported, and its rows are the answer.
    assert!(
        result
            .unserved_terms
            .iter()
            .all(|entry| entry.request_term != 0),
        "a term that reached a producer is never reported unserved"
    );
    assert_eq!(
        result.rows.len(),
        2,
        "the literal producer's two rows are the answer"
    );
}

#[test]
fn a_request_every_term_of_which_reached_a_producer_reports_nothing_unserved() {
    // The neighbouring valid case at the answer: the mixed request reaches a
    // producer for each of its three terms, so the evidence is empty. An empty
    // list is a claim too — it says every term was answered by something.
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let profile = fixture_profile();

    let result = block_on(search(
        &mixed_request(),
        &registry,
        &stats,
        &*common::empty_dataset(),
        &env,
        &profile,
        TOP_K,
    ))
    .expect("the fixture search answers");
    assert!(
        result.unserved_terms.is_empty(),
        "every term reached a producer, got {:?}",
        result.unserved_terms
    );
}

// ---------------------------------------------------------------------------
// 6. The bound
// ---------------------------------------------------------------------------

#[test]
fn the_bound_is_the_bound_and_a_larger_one_refuses_nothing() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let profile = fixture_profile();
    let request = mixed_request();

    let run = |top_k| {
        block_on(search(
            &request,
            &registry,
            &stats,
            &*common::empty_dataset(),
            &env,
            &profile,
            top_k,
        ))
        .expect("the fixture search answers")
    };

    let whole = run(TOP_K);
    assert_eq!(whole.rows.len(), 6, "six distinct candidates rank");

    let bounded = run(TopK::new(3));
    assert_eq!(bounded.rows.len(), 3, "at most k rows");
    assert_eq!(
        bounded.rows,
        whole.rows[..3].to_vec(),
        "and they are the top of the same declared order"
    );
    for pair in bounded.rows.windows(2) {
        assert!(pair[0].score >= pair[1].score, "score descending");
    }

    // Over-refusal is as severe as a wrong answer: a bound above what ranked is
    // answered with everything that ranked, not with an error and not with a
    // truncation.
    let generous = run(TopK::new(1_000));
    assert_eq!(generous.rows, whole.rows);

    // And the bound moves the rows, never the report: every stratum still
    // reaches its own terminal status under the smallest bound there is.
    let none = run(TopK::new(0));
    assert!(
        none.rows.is_empty(),
        "a bound of zero certifies no rows, got {:?}",
        none.rows
    );
    assert_eq!(
        none.trailer, whole.trailer,
        "the trailer is the completeness claim, and the rows never were"
    );
}
