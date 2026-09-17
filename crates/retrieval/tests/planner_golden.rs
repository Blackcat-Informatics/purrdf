// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The pure planner's contract: declaration-only term matching, deterministic
//! golden output, statistics consumption, and purity.
//!
//! The fixtures are `example.org` throughout. The registry is a mock of the
//! property-function seam; the statistics provider is a plain caller-owned
//! lookup. Neither touches a store, a file, a network or a clock.

use std::collections::BTreeMap;
use std::sync::Arc;

use pretty_assertions::assert_eq;
use purrdf_retrieval::{
    Iri, Metric, Plan, PlanError, RejectionReason, RequestTerm, RetrievalRequest, Statistics, Term,
    plan,
};
use purrdf_sparql_eval::{
    BindingPattern, DuplicatePolicy, EvalError, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction,
    PropertyFunctionRegistry, RankOrdering, RetrievalCapability, TermKind, TermPattern, Volatility,
};

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
        index_hint: Some("hint".to_owned()),
    }
}

fn seed_term() -> RequestTerm {
    RequestTerm::EntitySeed {
        entity: Term::new("<http://example.org/seed>"),
    }
}

fn mixed_request() -> RetrievalRequest {
    RetrievalRequest::from_terms(vec![lexical_term(), vector_term(), seed_term()])
}

fn lexical_request() -> RetrievalRequest {
    RetrievalRequest::from_terms(vec![lexical_term()])
}

fn vector_request() -> RetrievalRequest {
    RetrievalRequest::from_terms(vec![vector_term()])
}

fn ranked(stratum: &str, accepted_terms: Vec<TermPattern>) -> RetrievalCapability {
    RetrievalCapability::Ranked {
        stratum: kernel_iri(stratum),
        accepted_terms,
        ordering: RankOrdering::StrictlyDescending,
        duplicates: DuplicatePolicy::Unique,
    }
}

/// The mixed registry: one always-applicable (`Any`) producer, one
/// literal-only producer, one IRI-only producer, one quoted-triple producer
/// that no request term can reach, and one unranked producer.
fn mixed_registry() -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    let literal_pattern = TermPattern {
        kind: TermKind::Literal,
        datatype: None,
        language: Some("en".to_owned()),
        predicate: Some(ex("body")),
    };
    for (name, capability, rows) in [
        (
            ex("pf/any"),
            ranked(
                &ex("stratum/universal"),
                vec![TermPattern::of_kind(TermKind::Any)],
            ),
            200,
        ),
        (
            ex("pf/literal"),
            ranked(&ex("stratum/text"), vec![literal_pattern]),
            100,
        ),
        (
            ex("pf/iri"),
            ranked(
                &ex("stratum/graph"),
                vec![TermPattern::of_kind(TermKind::Iri)],
            ),
            50,
        ),
        (
            ex("pf/quoted"),
            ranked(
                &ex("stratum/quoted"),
                vec![TermPattern::of_kind(TermKind::Triple)],
            ),
            25,
        ),
        (ex("pf/not-ranked"), RetrievalCapability::NotRanked, 0),
    ] {
        let arity = PfArity::new(1, 1);
        registry.register(
            name,
            Arc::new(MockProducer {
                arity,
                mode: arity.all_free_mode(),
                rows,
                capability,
            }),
        );
    }
    registry
}

/// The mock statistics: cardinalities for the strata and the request predicate,
/// and one reported selectivity.
fn fixture_statistics() -> MockStatistics {
    let mut cardinalities = BTreeMap::new();
    cardinalities.insert(iri(&ex("stratum/universal")), 1000);
    cardinalities.insert(iri(&ex("stratum/text")), 100);
    cardinalities.insert(iri(&ex("stratum/graph")), 50);
    cardinalities.insert(iri(&ex("body")), 500);
    MockStatistics {
        source: "example-statistics".to_owned(),
        revision: "r1".to_owned(),
        cardinalities,
        selectivities: vec![(iri(&ex("body")), lexical_term(), 0.5)],
    }
}

/// A statistics provider that reports only the request predicate, so a stratum
/// with a genuinely unbounded declared row count has no cardinality to bound it.
fn no_stratum_cardinality() -> MockStatistics {
    let mut cardinalities = BTreeMap::new();
    cardinalities.insert(iri(&ex("body")), 500);
    MockStatistics {
        source: "example-statistics".to_owned(),
        revision: "r1".to_owned(),
        cardinalities,
        selectivities: vec![],
    }
}

// ---------------------------------------------------------------------------
// Mock property-function seam
// ---------------------------------------------------------------------------

struct MockProducer {
    arity: PfArity,
    mode: BindingPattern,
    rows: u64,
    capability: RetrievalCapability,
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

    fn retrieval_capability(&self) -> RetrievalCapability {
        self.capability.clone()
    }

    fn open(
        &self,
        _args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        // Planning never opens a cursor; the mock still satisfies the trait.
        Ok(Box::new(EmptyCursor))
    }
}

struct EmptyCursor;

impl PfCursor for EmptyCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(None)
    }
}

struct MockStatistics {
    source: String,
    revision: String,
    cardinalities: BTreeMap<Iri, u64>,
    selectivities: Vec<(Iri, RequestTerm, f64)>,
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

    fn selectivity(&self, predicate: &Iri, term: &RequestTerm) -> Option<f64> {
        self.selectivities
            .iter()
            .find(|(subject, candidate, _)| subject == predicate && candidate == term)
            .map(|(_, _, value)| *value)
    }
}

// ---------------------------------------------------------------------------
// Golden support
// ---------------------------------------------------------------------------

const GOLDEN_DIR: &str = "tests/fixtures/planner";

fn golden_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(GOLDEN_DIR)
        .join(name)
}

fn read_golden(name: &str) -> String {
    std::fs::read_to_string(golden_path(name))
        .unwrap_or_else(|error| panic!("golden {name} is checked in: {error}"))
}

/// A deterministic rendering of a plan as pretty JSON.
///
/// `serde_json`'s default (non-`preserve_order`) `Value` is sorted-key, so the
/// `HashMap` stratum maps serialize in canonical order rather than in
/// iteration order. The registry instance id is a per-process counter
/// (`registry_id.rs`), so it is pinned here and asserted separately; every
/// durable field is captured verbatim.
fn canonical_json(plan: &Plan) -> String {
    let mut value = serde_json::to_value(plan).expect("a plan serializes");
    value
        .as_object_mut()
        .expect("a plan serializes to an object")
        .insert(
            "registry_instance_id".to_owned(),
            serde_json::Value::from(0_u64),
        );
    let mut rendered = serde_json::to_string_pretty(&value).expect("the value renders");
    rendered.push('\n');
    rendered
}

// ---------------------------------------------------------------------------
// 1. Golden tests
// ---------------------------------------------------------------------------

#[test]
fn golden_mixed_request_matches() {
    let registry = mixed_registry();
    let plan = plan(&mixed_request(), &registry, &fixture_statistics()).expect("plans");
    assert_eq!(
        plan.registry_instance_id,
        registry.instance_id(),
        "the plan records the live registry instance identity"
    );
    assert_eq!(
        canonical_json(&plan),
        read_golden("mixed_request.json"),
        "planner output drifted from the golden; update {} if intended",
        golden_path("mixed_request.json").display()
    );
}

#[test]
fn golden_lexical_request_matches() {
    let registry = mixed_registry();
    let plan = plan(&lexical_request(), &registry, &fixture_statistics()).expect("plans");
    assert_eq!(
        canonical_json(&plan),
        read_golden("lexical_request.json"),
        "planner output drifted from the golden; update {} if intended",
        golden_path("lexical_request.json").display()
    );
}

#[test]
fn planning_is_a_pure_replay() {
    let request = mixed_request();
    let registry = mixed_registry();
    let statistics = fixture_statistics();
    let first = plan(&request, &registry, &statistics).expect("plans");
    let second = plan(&request, &registry, &statistics).expect("plans");
    assert_eq!(first, second, "same inputs, same plan");
    assert_eq!(first.canonical_bytes(), second.canonical_bytes());
    assert_eq!(first.id(), second.id());
    assert_eq!(canonical_json(&first), canonical_json(&second));
}

// ---------------------------------------------------------------------------
// 2. Term matching
// ---------------------------------------------------------------------------

fn binding<'a>(plan: &'a Plan, producer: &str) -> Option<&'a purrdf_retrieval::ProducerBinding> {
    plan.producer_bindings
        .iter()
        .find(|binding| binding.producer == producer)
}

fn reason(plan: &Plan, producer: &str) -> Option<RejectionReason> {
    plan.producer_decisions
        .iter()
        .find_map(|decision| match decision {
            purrdf_retrieval::ProducerDecision::Rejected {
                producer: name,
                reason,
            } if name == producer => Some(*reason),
            _ => None,
        })
}

#[test]
fn lexical_term_matches_a_literal_producer() {
    let plan = plan(&mixed_request(), &mixed_registry(), &fixture_statistics()).expect("plans");
    let literal = binding(&plan, &ex("pf/literal")).expect("literal producer selected");
    assert_eq!(literal.request_terms, vec![0]);
    assert_eq!(literal.stratum, iri(&ex("stratum/text")));
}

#[test]
fn vector_term_matches_only_an_any_producer() {
    let plan = plan(&vector_request(), &mixed_registry(), &fixture_statistics()).expect("plans");
    let any = binding(&plan, &ex("pf/any")).expect("the Any producer accepts the vector");
    assert_eq!(any.request_terms, vec![0]);
    assert_eq!(
        reason(&plan, &ex("pf/literal")),
        Some(RejectionReason::NoAcceptedTerm)
    );
    assert_eq!(
        reason(&plan, &ex("pf/iri")),
        Some(RejectionReason::NoAcceptedTerm)
    );
}

#[test]
fn entity_seed_matches_an_iri_producer() {
    let plan = plan(&mixed_request(), &mixed_registry(), &fixture_statistics()).expect("plans");
    let iri_producer = binding(&plan, &ex("pf/iri")).expect("IRI producer selected");
    assert_eq!(iri_producer.request_terms, vec![2]);
}

#[test]
fn unmatched_and_unranked_producers_are_rejected() {
    let plan = plan(&mixed_request(), &mixed_registry(), &fixture_statistics()).expect("plans");
    assert_eq!(
        reason(&plan, &ex("pf/quoted")),
        Some(RejectionReason::NoAcceptedTerm)
    );
    assert_eq!(
        reason(&plan, &ex("pf/not-ranked")),
        Some(RejectionReason::NotRanked)
    );
}

#[test]
fn language_and_predicate_constraints_are_enforced() {
    let request = RetrievalRequest::from_terms(vec![RequestTerm::Lexical {
        text: "different".to_owned(),
        language: Some("fr".to_owned()),
        predicate: Some(iri(&ex("other"))),
    }]);
    let plan = plan(&request, &mixed_registry(), &fixture_statistics()).expect("plans");
    // The literal producer constrains language `en` and predicate `body`, so it
    // rejects; only the unconstrained `Any` producer takes the term.
    assert_eq!(
        reason(&plan, &ex("pf/literal")),
        Some(RejectionReason::NoAcceptedTerm)
    );
    assert_eq!(
        binding(&plan, &ex("pf/any"))
            .expect("Any selected")
            .request_terms,
        vec![0]
    );
}

// ---------------------------------------------------------------------------
// 3. Producer selection
// ---------------------------------------------------------------------------

#[test]
fn always_applicable_producer_receives_every_term() {
    let plan = plan(&mixed_request(), &mixed_registry(), &fixture_statistics()).expect("plans");
    assert_eq!(
        binding(&plan, &ex("pf/any"))
            .expect("Any producer selected")
            .request_terms,
        vec![0, 1, 2]
    );
}

#[test]
fn a_request_that_reaches_nothing_is_refused() {
    let mut registry = PropertyFunctionRegistry::new();
    let arity = PfArity::new(1, 1);
    registry.register(
        ex("pf/quoted"),
        Arc::new(MockProducer {
            arity,
            mode: arity.all_free_mode(),
            rows: 25,
            capability: ranked(
                &ex("stratum/quoted"),
                vec![TermPattern::of_kind(TermKind::Triple)],
            ),
        }),
    );
    let error = plan(&lexical_request(), &registry, &fixture_statistics())
        .expect_err("a request reaching nothing must be refused");
    assert!(
        matches!(error, PlanError::NoApplicableProducers),
        "got {error:?}"
    );
}

#[test]
fn malformed_terms_are_refused_with_a_name() {
    let request = RetrievalRequest::from_terms(vec![RequestTerm::Lexical {
        text: "   ".to_owned(),
        language: None,
        predicate: None,
    }]);
    let error = plan(&request, &mixed_registry(), &fixture_statistics())
        .expect_err("an empty needle is malformed");
    match error {
        PlanError::InvalidRequestTerm { term, reason } => {
            assert!(matches!(*term, RequestTerm::Lexical { .. }));
            assert!(reason.contains("empty"));
        }
        other => panic!("expected InvalidRequestTerm, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 4. Statistics consumption
// ---------------------------------------------------------------------------

#[test]
fn depths_come_from_the_registry_row_bound_capped_by_statistics() {
    let plan = plan(&mixed_request(), &mixed_registry(), &fixture_statistics()).expect("plans");
    // Declared 200, stat 1000 -> 200; declared 100, stat 100 -> 100; declared 50, stat 50 -> 50.
    assert_eq!(plan.stratum_depths[&iri(&ex("stratum/universal"))], 200);
    assert_eq!(plan.stratum_depths[&iri(&ex("stratum/text"))], 100);
    assert_eq!(plan.stratum_depths[&iri(&ex("stratum/graph"))], 50);
    for (stratum, cardinality) in [
        (ex("stratum/universal"), 1000_u64),
        (ex("stratum/text"), 100),
        (ex("stratum/graph"), 50),
    ] {
        assert_eq!(
            plan.statistics_snapshot
                .entries
                .iter()
                .find(|entry| entry.subject == stratum)
                .map(|entry| entry.cardinality),
            Some(cardinality),
            "the recorded cardinality is the statistic planning consulted"
        );
    }
}

#[test]
fn the_planner_reads_the_registry_row_bound_not_a_parallel_field() {
    let mut registry = PropertyFunctionRegistry::new();
    let arity = PfArity::new(1, 1);
    registry.register(
        ex("pf/any"),
        Arc::new(MockProducer {
            arity,
            mode: arity.all_free_mode(),
            // 40, not 200: only a planner that reads the seam's declaration can
            // see this change.
            rows: 40,
            capability: ranked(
                &ex("stratum/universal"),
                vec![TermPattern::of_kind(TermKind::Any)],
            ),
        }),
    );
    let plan = plan(&mixed_request(), &registry, &fixture_statistics()).expect("plans");
    assert_eq!(plan.stratum_depths[&iri(&ex("stratum/universal"))], 40);
}

#[test]
fn statistics_lower_but_never_raise_the_declared_bound() {
    let mut registry = PropertyFunctionRegistry::new();
    let arity = PfArity::new(1, 1);
    registry.register(
        ex("pf/any"),
        Arc::new(MockProducer {
            arity,
            mode: arity.all_free_mode(),
            rows: 5_000,
            capability: ranked(
                &ex("stratum/universal"),
                vec![TermPattern::of_kind(TermKind::Any)],
            ),
        }),
    );
    let plan = plan(&mixed_request(), &registry, &fixture_statistics()).expect("plans");
    // Declared 5_000, stat 1_000 -> 1_000, never the other direction.
    assert_eq!(plan.stratum_depths[&iri(&ex("stratum/universal"))], 1_000);
}

#[test]
fn an_unbounded_stratum_without_statistics_is_refused() {
    let mut registry = PropertyFunctionRegistry::new();
    let arity = PfArity::new(1, 1);
    registry.register(
        ex("pf/unbounded"),
        Arc::new(MockProducer {
            arity,
            mode: arity.all_free_mode(),
            rows: u64::MAX,
            capability: ranked(
                &ex("stratum/endless"),
                vec![TermPattern::of_kind(TermKind::Any)],
            ),
        }),
    );
    let error = plan(&lexical_request(), &registry, &no_stratum_cardinality())
        .expect_err("an unbounded stratum with no statistic has no finite depth");
    match error {
        PlanError::StatisticsUnavailable { predicate } => {
            assert_eq!(*predicate, iri(&ex("stratum/endless")));
        }
        other => panic!("expected StatisticsUnavailable, got {other:?}"),
    }
}

#[test]
fn the_snapshot_records_the_consulted_selectivity() {
    let plan = plan(&mixed_request(), &mixed_registry(), &fixture_statistics()).expect("plans");
    let body = plan
        .statistics_snapshot
        .entries
        .iter()
        .find(|entry| entry.subject == ex("body"))
        .expect("the request predicate is recorded");
    assert_eq!(body.cardinality, 500);
    assert_eq!(body.selectivity_ppm, Some(500_000));
    assert_eq!(plan.statistics_snapshot.source, "example-statistics");
    assert_eq!(plan.statistics_snapshot.revision, "r1");
}

#[test]
fn the_planner_redeclares_no_seam_declaration_and_reads_no_ambient_state() {
    let source = include_str!("../src/planner.rs");
    // No parallel copy of the seam's declaration surface.
    for field in ["volatility:", "arity:", "modes:", "rows_per_invocation:"] {
        assert!(
            !source.contains(field),
            "planner.rs declares a parallel `{field}`; the seam is the single source"
        );
    }
    // Consumed through the registry's own describe machinery.
    assert!(source.contains("describe()"));
    assert!(source.contains("rows_per_invocation"));
    // Pure: no store, filesystem, network, clock or RNG.
    for ambient in [
        "std::fs",
        "std::net",
        "SystemTime",
        "Instant",
        "rand",
        "File::",
    ] {
        assert!(
            !source.contains(ambient),
            "planner.rs reaches for ambient state (`{ambient}`); planning must be pure"
        );
    }
}
