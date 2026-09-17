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
    UnservedReason, UnservedTerm, plan,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, DuplicatePolicy, EvalError, PfArgs, PfArity, PfCursor, PfRow,
    PropertyFunction, PropertyFunctionRegistry, RankOrdering, RankedDeclaration, RequestFacet,
    TermKind, TermPattern, TermPlacement, Volatility,
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

/// A mock relation of arity (1,1) declaring `rows` rows per invocation.
fn relation(rows: u64) -> Arc<dyn PropertyFunction> {
    let arity = PfArity::new(1, 1);
    Arc::new(MockProducer {
        arity,
        mode: arity.all_free_mode(),
        rows,
    })
}

/// The mixed registry: one catch-all (`Any`) producer, one
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
    for (name, declaration, rows) in [
        (
            ex("pf/any"),
            Some(ranked(
                &ex("stratum/universal"),
                vec![TermPattern::of_kind(TermKind::Any)],
                true,
            )),
            200,
        ),
        (
            ex("pf/literal"),
            Some(ranked(&ex("stratum/text"), vec![literal_pattern], false)),
            100,
        ),
        (
            ex("pf/iri"),
            Some(ranked(
                &ex("stratum/graph"),
                vec![TermPattern::of_kind(TermKind::Iri)],
                false,
            )),
            50,
        ),
        (
            ex("pf/quoted"),
            Some(ranked(
                &ex("stratum/quoted"),
                vec![TermPattern::of_kind(TermKind::Triple)],
                false,
            )),
            25,
        ),
        // Registered with no declaration at all: that is the whole of "does not
        // participate in ranked retrieval".
        (ex("pf/not-ranked"), None, 0),
    ] {
        match declaration {
            Some(declaration) => registry.register_ranked(name, relation(rows), declaration),
            None => registry.register(name, relation(rows)),
        }
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
        selectivities: vec![(iri(&ex("body")), lexical_term(), 500_000)],
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
    /// Reported selectivities as `(subject, term, parts per million)`. A `Vec`
    /// rather than a map because the key carries a [`RequestTerm`], which is
    /// deliberately not `Ord`.
    selectivities: Vec<(Iri, RequestTerm, u64)>,
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

    fn selectivity_ppm(&self, subject: &Iri, term: &RequestTerm) -> Option<u64> {
        self.selectivities
            .iter()
            .find(|(declared, candidate, _)| declared == subject && candidate == term)
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
// 2b. Per-term evidence: a term that reached nothing says so
//
// The decisions above are per producer. A caller asks per term, and the request
// lattice deliberately carries modalities ahead of the producers that answer
// them, so "this registry has nobody for that" has to be visible as exactly that
// rather than as a term that quietly produced no rows.
// ---------------------------------------------------------------------------

/// A registry that accepts a lexical `en`/`body` term and nothing else, so a
/// vector term in the same request reaches no declaration at all.
fn literal_only_registry() -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/literal"),
        relation(100),
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

/// The same registry plus a catch-all whose declaration demands that every term
/// it accepts be rendered into an argument position. A query embedding has no
/// SPARQL constant form, so the catch-all cannot be invoked for this request and
/// is rejected at placement — leaving the vector term accepted by something and
/// served by nothing.
fn accepting_but_uninvocable_registry() -> PropertyFunctionRegistry {
    let mut registry = literal_only_registry();
    registry.register_ranked(
        ex("pf/any"),
        relation(200),
        RankedDeclaration {
            stratum: kernel_iri(&ex("stratum/universal")),
            accepted_terms: vec![AcceptedTerm {
                pattern: TermPattern::of_kind(TermKind::Any),
                placements: vec![TermPlacement {
                    facet: RequestFacet::Value,
                    position: 1,
                    datatype: None,
                }],
            }],
            depth_placement: None,
            candidate_position: 0,
            ordering: RankOrdering::StrictlyDescending,
            duplicates: DuplicatePolicy::Unique,
            mandatory: false,
        },
    );
    registry
}

fn lexical_and_vector_request() -> RetrievalRequest {
    RetrievalRequest::from_terms(vec![lexical_term(), vector_term()])
}

#[test]
fn a_term_no_declaration_accepts_is_recorded_as_unserved() {
    let plan = plan(
        &lexical_and_vector_request(),
        &literal_only_registry(),
        &fixture_statistics(),
    )
    .expect("the lexical term still reaches a producer, so the request plans");

    assert_eq!(
        plan.unserved_terms,
        vec![UnservedTerm {
            request_term: 1,
            reason: UnservedReason::NoProducerAccepts,
        }],
        "the vector term reached no declaration, and the plan says so by index"
    );
    assert_eq!(
        plan.unserved_evidence(),
        plan.unserved_terms,
        "the evidence this plan supports is the record it made"
    );
    // The valid neighbour, in the same plan: the term that DID reach a producer
    // is served and is not reported.
    assert_eq!(
        binding(&plan, &ex("pf/literal"))
            .expect("the literal producer takes the lexical term")
            .request_terms,
        vec![0],
        "term 0 reached a producer"
    );
    assert!(
        plan.unserved_terms
            .iter()
            .all(|entry| entry.request_term != 0),
        "a term that reached a producer is never reported unserved"
    );
}

#[test]
fn a_term_every_acceptor_of_which_was_rejected_is_recorded_as_such() {
    let plan = plan(
        &lexical_and_vector_request(),
        &accepting_but_uninvocable_registry(),
        &fixture_statistics(),
    )
    .expect("the lexical term still reaches an invocable producer");

    assert_eq!(
        reason(&plan, &ex("pf/any")),
        Some(RejectionReason::UnsatisfiedConstraint),
        "the catch-all accepted the vector term but cannot be invoked for it"
    );
    assert_eq!(
        plan.unserved_terms,
        vec![UnservedTerm {
            request_term: 1,
            reason: UnservedReason::EveryAcceptingProducerRejected,
        }],
        "something accepted the shape and was then rejected, which is not the \
         same fact as nothing accepting it"
    );
}

#[test]
fn a_request_every_term_of_which_is_served_records_nothing() {
    // The neighbouring valid case: the mixed request reaches a producer for each
    // of its three terms, so there is no per-term evidence to report at all.
    let plan = plan(&mixed_request(), &mixed_registry(), &fixture_statistics()).expect("plans");
    assert!(
        plan.unserved_terms.is_empty(),
        "every term reached a producer, got {:?}",
        plan.unserved_terms
    );
    assert!(
        plan.unserved_evidence().is_empty(),
        "and the evidence it supports is empty too, got {:?}",
        plan.unserved_evidence()
    );
}

// ---------------------------------------------------------------------------
// 2c. The interval modalities: carried ahead of their producers, and reported
// as unserved rather than quietly dropped.
// ---------------------------------------------------------------------------

fn temporal_term() -> RequestTerm {
    RequestTerm::Temporal {
        predicate: iri(&ex("observed")),
        lower: Some("2026-01-01T00:00:00Z".to_owned()),
        upper: Some("2026-02-01T00:00:00Z".to_owned()),
    }
}

fn numeric_range_term() -> RequestTerm {
    RequestTerm::NumericRange {
        predicate: iri(&ex("price")),
        lower: Some(purrdf_retrieval::Fixed::ONE),
        upper: None,
    }
}

#[test]
fn an_interval_term_no_registry_producer_takes_is_reported_per_term() {
    // The registry's literal producer constrains predicate `body`, so neither
    // interval reaches it; nothing else in this registry accepts a literal. The
    // lexical term still reaches it, so the request plans and the answer is
    // honest about what it could not serve.
    let request =
        RetrievalRequest::from_terms(vec![lexical_term(), temporal_term(), numeric_range_term()]);
    let plan = plan(&request, &literal_only_registry(), &fixture_statistics())
        .expect("the lexical term still reaches a producer");

    assert_eq!(
        plan.unserved_terms,
        vec![
            UnservedTerm {
                request_term: 1,
                reason: UnservedReason::NoProducerAccepts,
            },
            UnservedTerm {
                request_term: 2,
                reason: UnservedReason::NoProducerAccepts,
            },
        ],
        "a modality the lattice carries ahead of its producers says so by index"
    );
    assert_eq!(plan.unserved_evidence(), plan.unserved_terms);
    // The neighbouring valid case, in the same plan: the term that did reach a
    // producer is served.
    assert_eq!(
        binding(&plan, &ex("pf/literal"))
            .expect("the literal producer takes the lexical term")
            .request_terms,
        vec![0]
    );
}

#[test]
fn an_interval_term_reaches_a_producer_that_declares_its_predicate() {
    // The other side of the same fact: nothing about the arm makes it
    // unreachable. A producer declaring a literal pattern on the interval's own
    // predicate, with a placement for each endpoint, is bound to it.
    let mut registry = PropertyFunctionRegistry::new();
    let date_time = "http://www.w3.org/2001/XMLSchema#dateTime";
    registry.register_ranked(
        ex("pf/temporal"),
        relation(100),
        RankedDeclaration {
            stratum: kernel_iri(&ex("stratum/time")),
            accepted_terms: vec![AcceptedTerm {
                pattern: TermPattern {
                    kind: TermKind::Literal,
                    datatype: None,
                    language: None,
                    predicate: Some(ex("observed")),
                },
                // The mock relation is arity (1,1), so it has exactly one
                // argument position beside its candidate and can take exactly
                // one endpoint; the request below is the half-open interval that
                // matches what this producer declared it can receive.
                placements: vec![TermPlacement {
                    facet: RequestFacet::LowerBound,
                    position: 1,
                    datatype: Some(date_time.to_owned()),
                }],
            }],
            depth_placement: None,
            candidate_position: 0,
            ordering: RankOrdering::StrictlyDescending,
            duplicates: DuplicatePolicy::Unique,
            mandatory: false,
        },
    );
    let request = RetrievalRequest::from_terms(vec![RequestTerm::Temporal {
        predicate: iri(&ex("observed")),
        lower: Some("2026-01-01T00:00:00Z".to_owned()),
        upper: None,
    }]);
    let plan = plan(&request, &registry, &fixture_statistics()).expect("the interval term plans");
    assert_eq!(
        binding(&plan, &ex("pf/temporal"))
            .expect("the temporal producer takes the interval")
            .request_terms,
        vec![0]
    );
    assert_eq!(
        plan.unserved_terms,
        Vec::new(),
        "the interval reached a producer, so nothing is reported unserved"
    );
}

#[test]
fn an_interval_that_constrains_nothing_is_refused_but_a_half_open_one_plans() {
    for empty in [
        RequestTerm::Temporal {
            predicate: iri(&ex("observed")),
            lower: None,
            upper: None,
        },
        RequestTerm::NumericRange {
            predicate: iri(&ex("price")),
            lower: None,
            upper: None,
        },
    ] {
        let request = RetrievalRequest::from_terms(vec![empty]);
        let error = plan(&request, &mixed_registry(), &fixture_statistics())
            .expect_err("an interval with neither endpoint constrains nothing");
        match error {
            PlanError::InvalidRequestTerm { reason, .. } => {
                assert!(reason.contains("neither endpoint"), "{reason}");
            }
            other => panic!("expected InvalidRequestTerm, got {other:?}"),
        }
    }
    // The neighbouring valid cases: one endpoint is a half-open interval, and it
    // plans.
    for half_open in [temporal_term(), numeric_range_term()] {
        let request = RetrievalRequest::from_terms(vec![half_open]);
        assert!(
            plan(&request, &mixed_registry(), &fixture_statistics()).is_ok(),
            "an interval carrying an endpoint is a question"
        );
    }
}

#[test]
fn an_inverted_numeric_range_is_refused_but_a_degenerate_one_plans() {
    let inverted = RetrievalRequest::from_terms(vec![RequestTerm::NumericRange {
        predicate: iri(&ex("price")),
        lower: Some(purrdf_retrieval::Fixed::from_raw(2)),
        upper: Some(purrdf_retrieval::Fixed::from_raw(1)),
    }]);
    let error = plan(&inverted, &mixed_registry(), &fixture_statistics())
        .expect_err("a range whose lower endpoint is above its upper matches nothing");
    match error {
        PlanError::InvalidRequestTerm { reason, .. } => {
            assert!(reason.contains("exceeds"), "{reason}");
        }
        other => panic!("expected InvalidRequestTerm, got {other:?}"),
    }
    // The neighbouring valid case, one raw unit away: equal endpoints are a
    // single point, which is a perfectly good question.
    let degenerate = RetrievalRequest::from_terms(vec![RequestTerm::NumericRange {
        predicate: iri(&ex("price")),
        lower: Some(purrdf_retrieval::Fixed::from_raw(1)),
        upper: Some(purrdf_retrieval::Fixed::from_raw(1)),
    }]);
    assert!(
        plan(&degenerate, &mixed_registry(), &fixture_statistics()).is_ok(),
        "a degenerate range is a point, not an empty set"
    );
}

#[test]
fn an_interval_predicate_reaches_the_statistics_snapshot() {
    // The interval names a predicate, so planning consults the statistics for
    // it exactly as it does for a needle's predicate; an arm the planner did not
    // teach `term_predicate` about would silently consult nothing.
    let request = RetrievalRequest::from_terms(vec![RequestTerm::NumericRange {
        predicate: iri(&ex("body")),
        lower: Some(purrdf_retrieval::Fixed::ONE),
        upper: None,
    }]);
    let plan = plan(&request, &mixed_registry(), &fixture_statistics()).expect("plans");
    assert_eq!(
        plan.statistics_snapshot
            .entries
            .iter()
            .find(|entry| entry.subject == ex("body"))
            .map(|entry| entry.cardinality),
        Some(500)
    );
}

// ---------------------------------------------------------------------------
// 3. Producer selection
// ---------------------------------------------------------------------------

/// Breadth of a pattern, not mandatory-ness: an unconstrained pattern matches
/// every term, so the planner binds every term to it. Whether the producer may
/// be dropped is a separate fact the registry declares, and admission reads it
/// from there rather than deriving it from this breadth.
#[test]
fn an_unconstrained_producer_is_bound_to_every_term() {
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
    registry.register_ranked(
        ex("pf/quoted"),
        relation(25),
        ranked(
            &ex("stratum/quoted"),
            vec![TermPattern::of_kind(TermKind::Triple)],
            false,
        ),
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
    registry.register_ranked(
        ex("pf/any"),
        // 40, not 200: only a planner that reads the seam's declaration can
        // see this change.
        relation(40),
        ranked(
            &ex("stratum/universal"),
            vec![TermPattern::of_kind(TermKind::Any)],
            true,
        ),
    );
    let plan = plan(&mixed_request(), &registry, &fixture_statistics()).expect("plans");
    assert_eq!(plan.stratum_depths[&iri(&ex("stratum/universal"))], 40);
}

#[test]
fn statistics_lower_but_never_raise_the_declared_bound() {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/any"),
        relation(5_000),
        ranked(
            &ex("stratum/universal"),
            vec![TermPattern::of_kind(TermKind::Any)],
            true,
        ),
    );
    let plan = plan(&mixed_request(), &registry, &fixture_statistics()).expect("plans");
    // Declared 5_000, stat 1_000 -> 1_000, never the other direction.
    assert_eq!(plan.stratum_depths[&iri(&ex("stratum/universal"))], 1_000);
}

#[test]
fn an_unbounded_stratum_without_statistics_is_refused() {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/unbounded"),
        relation(u64::MAX),
        ranked(
            &ex("stratum/endless"),
            vec![TermPattern::of_kind(TermKind::Any)],
            true,
        ),
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

/// The fixture statistics with exactly the reported selectivities given, so a
/// test states the whole of what the provider knows about selectivity.
fn selectivity_statistics(reported: Vec<(Iri, RequestTerm, u64)>) -> MockStatistics {
    MockStatistics {
        selectivities: reported,
        ..fixture_statistics()
    }
}

/// A selectivity is a statement about the rows a term can match, so it bounds
/// the depth the way a cardinality does. Both halves are executed: the stratum
/// the provider spoke about is narrowed, and — the case that would be silently
/// over-refused if the rule leaked — the stratum it said nothing about, and the
/// identical plan under a provider that reports no selectivity at all, keep
/// exactly the depth the registry declared.
#[test]
fn a_reported_stratum_selectivity_bounds_that_stratum_and_only_that_stratum() {
    let reported = selectivity_statistics(vec![(
        iri(&ex("stratum/text")),
        lexical_term(),
        250_000, // a quarter of the stratum's rows can match
    )]);
    let narrowed = plan(&lexical_request(), &mixed_registry(), &reported).expect("plans");
    assert_eq!(
        narrowed.stratum_depths[&iri(&ex("stratum/text"))],
        25,
        "a quarter of 100 rows is 25, so a depth of 100 would license reading rows no term fills"
    );
    assert_eq!(
        narrowed.stratum_depths[&iri(&ex("stratum/universal"))],
        200,
        "the stratum the provider said nothing about keeps its declared depth"
    );

    let silent = selectivity_statistics(Vec::new());
    let untouched = plan(&lexical_request(), &mixed_registry(), &silent).expect("plans");
    assert_eq!(
        untouched.stratum_depths[&iri(&ex("stratum/text"))],
        100,
        "a provider that measured no selectivity narrows nothing"
    );

    // The snapshot explains the depth rather than reporting a second number:
    // the value that narrowed the stratum is the value recorded beside it.
    assert_eq!(
        narrowed
            .statistics_snapshot
            .entries
            .iter()
            .find(|entry| entry.subject == ex("stratum/text"))
            .and_then(|entry| entry.selectivity_ppm),
        Some(250_000)
    );
}

/// The bound is rounded up and clamped at unity, because the failure mode of
/// getting either wrong is a ranked list that is quietly shorter than the
/// stratum's real answer.
#[test]
fn a_selectivity_rounds_up_and_never_raises_a_declared_depth() {
    // One part per million of 100 rows is a ten-thousandth of a row. Rounding
    // down would record a depth of zero for a stratum that can still answer.
    let sparse = selectivity_statistics(vec![(iri(&ex("stratum/text")), lexical_term(), 1)]);
    assert_eq!(
        plan(&lexical_request(), &mixed_registry(), &sparse)
            .expect("plans")
            .stratum_depths[&iri(&ex("stratum/text"))],
        1
    );

    // Unity, and the two provider faults above it, all leave the bound exactly
    // where the registry's declaration and the cardinality put it. A ratio
    // cannot widen a depth.
    for ppm in [1_000_000_u64, 5_000_000, u64::MAX] {
        let full = selectivity_statistics(vec![(iri(&ex("stratum/text")), lexical_term(), ppm)]);
        assert_eq!(
            plan(&lexical_request(), &mixed_registry(), &full)
                .expect("plans")
                .stratum_depths[&iri(&ex("stratum/text"))],
            100,
            "{ppm} ppm must not raise the declared bound"
        );
    }

    // Zero is a measurement, not an absence: the provider says no row under
    // this stratum matches, and the plan records the depth that follows.
    let none = selectivity_statistics(vec![(iri(&ex("stratum/text")), lexical_term(), 0)]);
    assert_eq!(
        plan(&lexical_request(), &mixed_registry(), &none)
            .expect("plans")
            .stratum_depths[&iri(&ex("stratum/text"))],
        0
    );
}

/// Several terms reaching one stratum are summed, not minimised. A producer
/// that answers the union of the terms it was handed returns more rows than the
/// most selective of them describes, and a bound below that would truncate its
/// ranked list with nothing saying so.
#[test]
fn selectivities_across_the_terms_one_stratum_receives_are_summed() {
    let reported = selectivity_statistics(vec![
        (iri(&ex("stratum/universal")), lexical_term(), 300_000),
        (iri(&ex("stratum/universal")), vector_term(), 400_000),
    ]);
    let planned = plan(&mixed_request(), &mixed_registry(), &reported).expect("plans");
    assert_eq!(
        planned.stratum_depths[&iri(&ex("stratum/universal"))],
        140,
        "seven tenths of the 200-row bound, not the three tenths the most selective term names"
    );
    assert_eq!(
        planned
            .statistics_snapshot
            .entries
            .iter()
            .find(|entry| entry.subject == ex("stratum/universal"))
            .and_then(|entry| entry.selectivity_ppm),
        Some(700_000),
        "the recorded aggregate is the one the depth was derived from"
    );
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
