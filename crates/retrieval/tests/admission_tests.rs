// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Admission and execution: the narrow waist over three plan origins.
//!
//! Every refusal dimension has its own test and its own distinct variant: fresh
//! plans are admitted, edited and deserialized plans are checked on the same
//! terms, and one stratum's failure leaves the others streaming. The fixtures are
//! `example.org` throughout; the registry is a mock of the property-function seam
//! and the statistics provider is a plain caller-owned lookup.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use pretty_assertions::assert_eq;
use purrdf_core::TermValue;
use purrdf_retrieval::{
    AdmissionEnvironment, AdmissionError, CompiledRetrieval, DecayRule, Fixed, FusionError,
    FusionProfile, Iri, Metric, MonotoneDepth, Plan, PlanOrigin, ProducerDecision, ProducerStatus,
    RankedStreamImpl, RejectionReason, RequestTerm, RetrievalRequest, Statistics, Term,
    UnservedReason, UnservedTerm, compile, contribution, execute,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, DuplicatePolicy, EvalError, PfArgs, PfArity, PfCursor, PfRow,
    PropertyFunction, PropertyFunctionRegistry, RankedDeclaration, RequestFacet, TermKind,
    TermPattern, TermPlacement, Volatility,
};

mod common;

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
/// An unconstrained `TermKind::Any` pattern is the exception: it declares **no**
/// placement at all, so it is matched by every request term and receives none of
/// them. Its argument stays free, and the plan reports every term that reached
/// only this producer — matching a shape is not the same as receiving it.
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

/// A registry whose declarations differ from the fixture's: one extra producer,
/// under a stratum of its own, so the content fingerprint moves.
///
/// The stratum is its own rather than the catch-all's because a stratum carries
/// one producer; what moves the fingerprint is the extra declaration, and it
/// moves it either way.
fn registry_with_different_declaration() -> PropertyFunctionRegistry {
    let mut registry = fixture_registry();
    registry.register_ranked(
        ex("pf/extra"),
        producer(999, "extra/", 0),
        ranked(
            &ex("stratum/extra"),
            vec![TermPattern::of_kind(TermKind::Any)],
            true,
        ),
    );
    registry
}

/// A ranked producer that declares **no access mode at all**.
///
/// A host can register a relation that reports no invocation pattern — an index
/// that is not built yet, a relation gated on configuration the host has not
/// supplied. Such a producer declares no worst-case row count either, because a
/// row count is declared per mode, and "declared nothing" is not "declared
/// zero": zero is a promise that nothing ranks there.
struct NoModeProducer {
    arity: PfArity,
}

impl PropertyFunction for NoModeProducer {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        self.arity
    }

    fn modes(&self) -> &[BindingPattern] {
        &[]
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        // Never reached: a row count is reported per declared mode, and this
        // relation declares none.
        0
    }

    fn open(
        &self,
        _args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        Ok(Box::new(RowCursor {
            rows: Vec::new().into_iter(),
        }))
    }
}

/// The fixture's mandatory catch-all producer, plus a producer that declares no
/// access mode under a stratum of its own.
fn registry_with_a_modeless_producer() -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
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
        ex("pf/modeless"),
        Arc::new(NoModeProducer {
            arity: PfArity::new(1, 1),
        }),
        ranked(
            &ex("stratum/modeless"),
            vec![TermPattern::of_kind(TermKind::Any)],
            false,
        ),
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

    fn selectivity_ppm(&self, _subject: &Iri, _term: &RequestTerm) -> Option<u64> {
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

fn fresh_plan(registry: &PropertyFunctionRegistry, stats: &MockStatistics) -> Plan {
    purrdf_retrieval::plan(&mixed_request(), registry, stats).expect("the fixture request plans")
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

fn mandatory() -> String {
    ex("pf/any")
}

// ---------------------------------------------------------------------------
// 1. Admission success
// ---------------------------------------------------------------------------

#[test]
fn admission_accepts_fresh_plan() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let plan = fresh_plan(&registry, &stats);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&plan, &env).expect("a fresh plan is admitted");
    assert_eq!(compiled.plan_id, plan.id());
    assert_eq!(compiled.registry_id, registry.instance_id());
    assert_eq!(
        compiled.registry_fingerprint,
        registry.content_fingerprint().expect("fingerprint")
    );
    assert_eq!(compiled.units.len(), 3, "one unit per declared stratum");
    for unit in &compiled.units {
        assert!(
            unit.sparql.starts_with("SELECT ?candidate WHERE"),
            "unit for {} is a SELECT over the common candidate variable: {}",
            unit.stratum,
            unit.sparql
        );
        assert!(
            unit.sparql.contains("( ?c0 ) <"),
            "the subject argument list is parenthesized even at arity one: {}",
            unit.sparql
        );
    }
    let universal = compiled
        .units
        .iter()
        .find(|unit| unit.stratum == iri(&ex("stratum/universal")))
        .expect("the mandatory producer's stratum emits a unit");
    assert!(universal.sparql.contains(&ex("pf/any")));
    // The seed producer receives the seed itself as a rendered constant, which is
    // what "the request is in the text" means.
    let graph = compiled
        .units
        .iter()
        .find(|unit| unit.stratum == iri(&ex("stratum/graph")))
        .expect("the seed stratum emits a unit");
    assert_eq!(
        graph.sparql,
        format!(
            "SELECT ?candidate WHERE {{\n  \
             {{ SELECT (?c0 AS ?candidate) WHERE {{ ( ?c0 ) <{}> ( <{}> ) }} LIMIT 50 }}\n\
             }}\nLIMIT 50",
            ex("pf/iri"),
            ex("seed")
        )
    );
}

// ---------------------------------------------------------------------------
// 2. Deleted / missing mandatory producers
// ---------------------------------------------------------------------------

#[test]
fn admission_rejects_edited_plan_dropping_mandatory() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let mut plan = fresh_plan(&registry, &stats);
    plan.producer_bindings
        .retain(|binding| binding.producer != mandatory());
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let error = compile(&plan, &env).expect_err("a deleted mandatory producer is refused");
    assert_eq!(error.dimension(), "missing_mandatory_producer");
    match error {
        AdmissionError::MissingMandatoryProducer { producer } => {
            assert_eq!(producer.as_str(), mandatory());
        }
        other => panic!("expected MissingMandatoryProducer, got {other:?}"),
    }
}

#[test]
fn admission_rejects_roundtrip_with_dropped_producer() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let plan = fresh_plan(&registry, &stats);
    let bytes = plan.canonical_bytes();
    let mut decoded = Plan::from_canonical_bytes(&bytes).expect("canonical bytes decode");
    decoded
        .producer_bindings
        .retain(|binding| binding.producer != mandatory());
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let error = compile(&decoded, &env).expect_err("the decoded plan still must admit");
    assert!(
        matches!(error, AdmissionError::MissingMandatoryProducer { .. }),
        "got {error:?}"
    );
}

#[test]
fn admission_rejects_tampered_deserialized() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let plan = fresh_plan(&registry, &stats);
    let json = serde_json::to_string(&plan).expect("a plan serializes");
    let mut decoded: Plan = serde_json::from_str(&json).expect("a plan deserializes");
    decoded
        .producer_bindings
        .retain(|binding| binding.producer != mandatory());
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let error = compile(&decoded, &env).expect_err("a tampered plan is refused");
    assert!(
        matches!(error, AdmissionError::MissingMandatoryProducer { .. }),
        "the tampered deserialized plan is refused on the same dimension, got {error:?}"
    );
}

#[test]
fn admission_rejects_missing_mandatory() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let mut plan = fresh_plan(&registry, &stats);
    // A hand-built plan that never carried the mandatory binding at all.
    plan.producer_bindings.clear();
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let error = compile(&plan, &env).expect_err("a plan without the mandatory producer is refused");
    assert!(
        matches!(error, AdmissionError::MissingMandatoryProducer { .. }),
        "got {error:?}"
    );
}

// ---------------------------------------------------------------------------
// 3. Under-bound mandatory producers
// ---------------------------------------------------------------------------

#[test]
fn admission_rejects_insufficient_bindings() {
    // The mandatory producer here accepts and places both of the request's
    // terms, so "every term it receives" is two and an edit that leaves it one
    // is the shortfall. `fixture_registry`'s own mandatory producer places
    // nothing and therefore receives nothing, which is a different fact with a
    // test of its own.
    let registry = catch_all_registry(true);
    let stats = statistics("r1");
    let mut plan = purrdf_retrieval::plan(&lexical_and_seed_request(), &registry, &stats)
        .expect("the fixture request plans");
    let binding = plan
        .producer_bindings
        .iter_mut()
        .find(|binding| binding.producer == ex("pf/catch-all"))
        .expect("the mandatory producer is bound");
    assert_eq!(binding.request_terms, vec![0, 1]);
    binding.request_terms.truncate(1);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let error = compile(&plan, &env).expect_err("an under-bound producer is refused");
    match error {
        AdmissionError::InsufficientBindings {
            producer,
            required,
            provided,
        } => {
            assert_eq!(producer.as_str(), ex("pf/catch-all"));
            assert_eq!(required, 2);
            assert_eq!(provided, 1);
        }
        other => panic!("expected InsufficientBindings, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 4. Depth bounds
// ---------------------------------------------------------------------------

#[test]
fn admission_rejects_depth_bound_violation() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let mut plan = fresh_plan(&registry, &stats);
    plan.stratum_depths
        .insert(iri(&ex("stratum/universal")), 201);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let error = compile(&plan, &env).expect_err("a raised depth is refused");
    match error {
        AdmissionError::DepthBoundViolation {
            stratum,
            declared,
            requested,
        } => {
            assert_eq!(*stratum, iri(&ex("stratum/universal")));
            assert_eq!(declared, 200);
            assert_eq!(requested, 201);
        }
        other => panic!("expected DepthBoundViolation, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 4b. Depth against the fusion profile's own arithmetic — measured, not refused
//
// The registry's row bound answers "can this many rows be produced". It says
// nothing about whether the profile can still tell them apart once they are.
// That second question is real, but its answer is not a defect: past the
// separating depth the fused score stops parting adjacent ranks and the declared
// tie-break — total — orders them by best stratum rank and then canonical term.
// The answer is correct and deterministic at a coarser resolution.
//
// So admission reports it instead of refusing it. Refusing rejected plans that
// order perfectly well for the caller's purpose, and it did so on a property of
// the *consumer's* arithmetic that no producer supplied a term of. The three
// tests below pin the measurement: a depth past the bound is admitted and
// carries its resolution, a depth exactly on the bound is admitted and fully
// separated, and a stratum the profile does not weight is reported on at all.
// ---------------------------------------------------------------------------

/// The smoothing constant the monotone-range fixtures use. One, so the bound is
/// the first colliding denominator less one and the arithmetic is legible.
const MONOTONE_K: u32 = 1;

/// A profile weighting `stratum/universal` alone, at a weight small enough that
/// the reciprocal's own resolution is exhausted well inside the registry's
/// declared row bound of two hundred.
///
/// A weight of `10^-9` puts the first colliding rank in the tens. That is a
/// perfectly legitimate weight — `FusionProfile::with_decay` asks only that a
/// weight be strictly positive — which is exactly the point: nothing about the
/// profile itself is malformed, and only the coupling with the depth is.
///
/// `Fixed::from_raw(1_000)` is deliberate here and is *not* the constructor a
/// whole-number weight wants: raw units are `10^-12` each, so this is `10^-9`,
/// which is the sub-unit weight this fixture is about. A weight meaning the
/// number one is `Fixed::ONE` or `Fixed::from_integer(1)`.
fn shallow_profile() -> FusionProfile {
    FusionProfile::with_decay(
        BTreeMap::from([(iri(&ex("stratum/universal")), Fixed::from_raw(1_000))]),
        DecayRule::ReciprocalRank { k: MONOTONE_K },
    )
    .expect("a strictly positive weight is a valid profile")
}

#[test]
fn a_depth_beyond_the_profiles_monotone_range_is_admitted_and_records_its_resolution() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let profile = shallow_profile();
    let stratum = iri(&ex("stratum/universal"));
    let monotone = profile
        .monotone_depth(&stratum)
        .expect("the profile weights this stratum")
        .rank()
        .expect("this fixture collides inside the expressible range");
    assert!(
        monotone < 200,
        "the fixture must collide inside the registry's row bound, got {monotone}"
    );

    let mut plan = fresh_plan(&registry, &stats);
    let requested = u32::try_from(monotone + 1).expect("the fixture bound is small");
    plan.stratum_depths.insert(stratum.clone(), requested);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: Some(&profile),
    };

    // The criterion. This plan reads one rank past the depth its profile still
    // separates, and that is not a defect to refuse: the answer is correct and
    // deterministic there, merely coarser. It is admitted.
    let compiled = compile(&plan, &env).expect("a depth past the monotone range is admitted");

    // And it is not admitted silently. The caller learns, before executing
    // anything, exactly what the depth costs in rank resolution.
    let recorded = compiled
        .resolution
        .get(&stratum)
        .copied()
        .expect("a weighted stratum's resolution is recorded");
    assert_eq!(
        recorded.separation,
        MonotoneDepth::SeparatesTo(monotone),
        "the evidence names the bound it was measured against"
    );
    assert_eq!(recorded.requested_depth, requested);
    assert!(
        !recorded.fully_separated(),
        "reading past the bound is reported as not fully separated"
    );
}

#[test]
fn a_depth_exactly_at_the_monotone_bound_is_admitted_and_still_orders_by_rank() {
    // The neighbouring valid case. A bound drawn one rank short would refuse
    // this plan, which orders perfectly — the over-refusal this repository
    // treats as exactly as severe as a wrong answer.
    let registry = fixture_registry();
    let stats = statistics("r1");
    let profile = shallow_profile();
    let stratum = iri(&ex("stratum/universal"));
    let monotone = profile
        .monotone_depth(&stratum)
        .expect("the profile weights this stratum")
        .rank()
        .expect("this fixture collides inside the expressible range");

    let mut plan = fresh_plan(&registry, &stats);
    plan.stratum_depths.insert(
        stratum.clone(),
        u32::try_from(monotone).expect("the fixture bound is small"),
    );
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: Some(&profile),
    };
    let compiled = compile(&plan, &env).expect("a depth exactly at the bound is admitted");
    assert!(
        compiled
            .resolution
            .get(&stratum)
            .expect("a weighted stratum's resolution is recorded")
            .fully_separated(),
        "a depth exactly on the bound is reported as fully separated"
    );

    // Admitted *and* ordered: every adjacent pair of ranks the admitted depth
    // covers produces a strictly smaller contribution than the one before it.
    let weight = profile
        .weight(&stratum)
        .expect("the profile weights this stratum");
    for rank in 1..monotone {
        let here = contribution(weight, rank, MONOTONE_K).expect("a 1-based rank contributes");
        let next = contribution(weight, rank + 1, MONOTONE_K).expect("a 1-based rank contributes");
        assert!(
            here > next,
            "ranks {rank} and {} must stay ordered inside the admitted depth",
            rank + 1
        );
    }
    // And the first pair beyond it is exactly where they stop.
    assert_eq!(
        contribution(weight, monotone, MONOTONE_K).expect("contributes"),
        contribution(weight, monotone + 1, MONOTONE_K).expect("contributes"),
        "the bound is the last ordered depth, not one short of it"
    );
}

#[test]
fn a_stratum_the_profile_does_not_weight_is_not_held_to_a_monotone_range() {
    // The other neighbouring valid case: `shallow_profile` weights only one of
    // the three strata the fixture plan reaches, and the other two carry depths
    // of a hundred and fifty. A profile that says nothing about a stratum has
    // said nothing about how deep it may be read, and refusing those depths
    // would refuse a plan over a law that does not govern it.
    let registry = fixture_registry();
    let stats = statistics("r1");
    let profile = shallow_profile();
    let plan = fresh_plan(&registry, &stats);
    assert_eq!(plan.stratum_depths[&iri(&ex("stratum/text"))], 100);
    assert!(
        profile.monotone_depth(&iri(&ex("stratum/text"))).is_none(),
        "the fixture profile must not weight this stratum"
    );

    let mut admitted = plan;
    admitted.stratum_depths.insert(
        iri(&ex("stratum/universal")),
        u32::try_from(
            profile
                .monotone_depth(&iri(&ex("stratum/universal")))
                .expect("weighted")
                .rank()
                .expect("this fixture collides inside the expressible range"),
        )
        .expect("the fixture bound is small"),
    );
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: Some(&profile),
    };
    let compiled = compile(&admitted, &env)
        .expect("an unweighted stratum's depth is not this profile's business");
    assert!(
        !compiled.resolution.contains_key(&iri(&ex("stratum/text"))),
        "a stratum the profile does not weight contributes nothing to this fusion, so there is \
         nothing to report about how deep it was planned"
    );
}

// ---------------------------------------------------------------------------
// 4c. The other direction of the same coupling: a deep stratum, admitted
//
// The registry's row bound and the profile's monotone range are independent, and
// under `DecayRule::WeightedReciprocalRank` the second one is bought with
// weight: the range runs to roughly `10^6 * sqrt(w)`, so a stratum that must be
// read fourteen million ranks deep needs a weight of about two hundred. These
// two tests are the criterion and its neighbour — the deep plan is admitted
// through the real admission path, and the depth one rank past the profile's own
// range is still refused by name.
// ---------------------------------------------------------------------------

/// The depth an operator requires of the deep stratum.
const REQUIRED_DEPTH: u32 = 14_000_000;

/// The weight that buys it. `(14e6 / 1e6)^2 = 196`, so two hundred clears the
/// requirement with room; the assertion below checks the bound rather than
/// trusting the arithmetic in this comment.
const DEEP_WEIGHT_UNITS: i64 = 200;

/// A registry whose one ranked producer declares enough rows for the deep plan.
///
/// The declared row count is a claim about the relation, not an allocation: the
/// mock emits three rows whatever it declares, so the fixture states a bound of
/// twenty million without producing one.
fn deep_registry() -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/any"),
        producer(20_000_000, "universal/", 3),
        ranked(
            &ex("stratum/universal"),
            vec![TermPattern::of_kind(TermKind::Any)],
            true,
        ),
    );
    registry
}

/// A profile on the weighted rule, heavy enough to order the deep stratum.
fn deep_profile() -> FusionProfile {
    FusionProfile::with_decay(
        BTreeMap::from([(
            iri(&ex("stratum/universal")),
            Fixed::from_integer(DEEP_WEIGHT_UNITS).expect("two hundred is representable"),
        )]),
        DecayRule::WeightedReciprocalRank { k: 1 },
    )
    .expect("a strictly positive weight is a valid profile")
}

#[test]
fn a_fourteen_million_deep_stratum_is_admitted_under_a_heavy_enough_weighted_profile() {
    let registry = deep_registry();
    let stats = statistics("r1");
    let profile = deep_profile();
    let stratum = iri(&ex("stratum/universal"));
    let monotone = profile
        .monotone_depth(&stratum)
        .expect("the profile weights this stratum")
        .rank()
        .expect("this weight collides inside the expressible range");
    assert!(
        monotone >= u64::from(REQUIRED_DEPTH),
        "a weight of {DEEP_WEIGHT_UNITS} must order {REQUIRED_DEPTH} ranks, got {monotone}"
    );

    let mut plan = fresh_plan(&registry, &stats);
    plan.stratum_depths.insert(stratum.clone(), REQUIRED_DEPTH);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: Some(&profile),
    };
    let compiled = compile(&plan, &env).expect("a depth of fourteen million is admitted");
    assert_eq!(compiled.units.len(), 1, "one unit for the one deep stratum");
    assert!(
        compiled.units[0].sparql.contains("LIMIT 14000000"),
        "the admitted depth reaches the emitted text: {}",
        compiled.units[0].sparql
    );

    // The same plan under the same weight on the *first* rule answers at a much
    // coarser resolution, which is what makes the rule the load-bearing choice
    // rather than the weight. Both are admitted; only one of them still tells
    // fourteen million ranks apart, and the evidence says which.
    let truncated = FusionProfile::with_decay(
        BTreeMap::from([(
            stratum.clone(),
            Fixed::from_integer(DEEP_WEIGHT_UNITS).expect("two hundred is representable"),
        )]),
        DecayRule::ReciprocalRank { k: 1 },
    )
    .expect("valid");
    let shallow_env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: Some(&truncated),
    };
    let coarse = compile(&plan, &shallow_env).expect("the truncated rule still answers");
    let recorded = coarse
        .resolution
        .get(&stratum)
        .copied()
        .expect("a weighted stratum's resolution is recorded");
    assert!(
        !recorded.fully_separated(),
        "the truncated rule cannot separate fourteen million ranks at any weight"
    );
    // And it stops near `sqrt(S) = 10^6` rather than anywhere the weight chose:
    // the inner truncation is a ceiling two hundred times the weight cannot lift.
    let bound = recorded
        .separation
        .rank()
        .expect("the truncated rule always collides inside the expressible range");
    assert!(
        (1_000_000..1_100_000).contains(&bound),
        "the truncated rule's bound is set by the scale, not by the weight, got {bound}"
    );
}

#[test]
fn a_depth_past_the_weighted_profiles_own_range_reports_a_coarser_resolution() {
    // Buying depth with weight does not become "any depth at all" — it becomes a
    // number the caller can read. One rank past this profile's exact range is
    // admitted and reported as not fully separated; the rank exactly on it is
    // admitted and reported as separated. The pair is the whole claim: the
    // boundary is still measured exactly, it is simply no longer a refusal.
    let registry = deep_registry();
    let stats = statistics("r1");
    let profile = deep_profile();
    let stratum = iri(&ex("stratum/universal"));
    let monotone = profile
        .monotone_depth(&stratum)
        .expect("the profile weights this stratum")
        .rank()
        .expect("this weight collides inside the expressible range");
    let requested = u32::try_from(monotone + 1).expect("the bound is inside a u32");
    assert!(
        u64::from(requested) <= 20_000_000,
        "the measurement must come from the profile, not from the registry's row bound"
    );

    let mut plan = fresh_plan(&registry, &stats);
    plan.stratum_depths.insert(stratum.clone(), requested);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: Some(&profile),
    };
    let compiled = compile(&plan, &env).expect("a depth past the range is admitted");
    let recorded = compiled
        .resolution
        .get(&stratum)
        .copied()
        .expect("a weighted stratum's resolution is recorded");
    assert_eq!(recorded.separation, MonotoneDepth::SeparatesTo(monotone));
    assert_eq!(recorded.requested_depth, requested);
    assert!(
        !recorded.fully_separated(),
        "one rank past the range is reported as coarser"
    );

    // The neighbour, one rank down, at the exact boundary.
    let mut admitted = plan;
    admitted.stratum_depths.insert(
        stratum.clone(),
        u32::try_from(monotone).expect("the bound is inside a u32"),
    );
    let compiled = compile(&admitted, &env).expect("a depth exactly at the bound is admitted");
    assert!(
        compiled
            .resolution
            .get(&stratum)
            .expect("recorded")
            .fully_separated(),
        "the bound is the last fully separated depth, not one short of it"
    );
}

#[test]
fn an_environment_that_names_no_profile_reports_no_resolution() {
    // Resolution is measured against a law, and an environment that names no law
    // has none to measure against. A caller that plans and compiles before
    // choosing how to fuse is not refused for not having chosen — and is handed
    // no fabricated evidence either.
    let registry = fixture_registry();
    let stats = statistics("r1");
    let profile = shallow_profile();
    let stratum = iri(&ex("stratum/universal"));
    let monotone = profile
        .monotone_depth(&stratum)
        .expect("the profile weights this stratum")
        .rank()
        .expect("this fixture collides inside the expressible range");
    let mut plan = fresh_plan(&registry, &stats);
    plan.stratum_depths.insert(
        stratum,
        u32::try_from(monotone + 1).expect("the fixture bound is small"),
    );
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&plan, &env).expect("no profile named, so nothing to measure against");
    assert!(
        compiled.resolution.is_empty(),
        "an unnamed law reports nothing rather than a default"
    );
}

// ---------------------------------------------------------------------------
// 5. Weights are the fusion law's, and are checked where they live
//
// A plan records no weights, so the admission waist has no weight dimension.
// Both halves of what one would have asked are answered by the value that
// actually holds the fusing weights, and these two tests pin them there — each
// with the neighbouring case that must still be admitted, because a weight
// check that over-refuses costs a caller its whole answer.
// ---------------------------------------------------------------------------

#[test]
fn a_weight_unusable_under_the_sum_is_refused_where_the_fusing_weights_live() {
    let stratum = iri(&ex("stratum/text"));

    // The criterion: a non-positive weight cannot participate in the §5 sum, so
    // no profile carrying one can be built. A plan therefore cannot present one.
    let error = FusionProfile::with_decay(
        BTreeMap::from([(stratum.clone(), Fixed::ZERO)]),
        DecayRule::ReciprocalRank { k: 60 },
    )
    .expect_err("a zero weight cannot fuse");
    match error {
        FusionError::NonPositiveWeight {
            stratum: named,
            weight,
        } => {
            assert_eq!(named, stratum.as_str());
            assert_eq!(weight, Fixed::ZERO);
        }
        other => panic!("expected NonPositiveWeight, got {other:?}"),
    }

    // THE NEIGHBOURING CASE: the smallest strictly positive weight there is —
    // one raw unit, `10^-12` — is admitted. The refusal is of a weight that
    // cannot contribute at all, never of a small one. (What such a weight cannot
    // do is order a deep stratum, and that is a different, separately named
    // dimension: see the monotone-depth tests above.)
    let thin = FusionProfile::with_decay(
        BTreeMap::from([(stratum.clone(), Fixed::from_raw(1))]),
        DecayRule::ReciprocalRank { k: 60 },
    )
    .expect("one raw unit is strictly positive");
    assert_eq!(thin.weight(&stratum), Some(Fixed::from_raw(1)));

    // And a profile a plan can actually be fused under admits that plan, with no
    // weight dimension of its own left at the waist to trip over.
    let registry = fixture_registry();
    let stats = statistics("r1");
    let plan = fresh_plan(&registry, &stats);
    let weights: BTreeMap<Iri, Fixed> = plan
        .stratum_depths
        .keys()
        .cloned()
        .map(|reached| (reached, Fixed::ONE))
        .collect();
    let profile = FusionProfile::with_decay(weights, DecayRule::ReciprocalRank { k: 60 })
        .expect("unit weights are valid");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: Some(&profile),
    };
    compile(&plan, &env).expect("a valid profile admits the plan it will fuse");
}

#[test]
fn a_profile_weighting_a_stratum_no_producer_emits_under_is_admitted() {
    // A profile is a reusable law, chosen without reference to any one plan, so
    // a weight for a stratum this request never reaches costs nothing and hides
    // nothing: no stream ever carries that stratum, so the weight is simply
    // never consulted. Refusing it would refuse the reuse a profile exists for.
    let registry = fixture_registry();
    let stats = statistics("r1");
    let plan = fresh_plan(&registry, &stats);
    let ghost = iri(&ex("stratum/ghost"));
    let weights: BTreeMap<Iri, Fixed> = plan
        .stratum_depths
        .keys()
        .cloned()
        .chain(core::iter::once(ghost.clone()))
        .map(|stratum| (stratum, Fixed::ONE))
        .collect();
    let profile = FusionProfile::with_decay(weights, DecayRule::ReciprocalRank { k: 60 })
        .expect("every weight is strictly positive");
    assert_eq!(
        profile.weight(&ghost),
        Some(Fixed::ONE),
        "the profile does declare the stratum nothing emits under"
    );

    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: Some(&profile),
    };
    let compiled = compile(&plan, &env).expect("an unreached weight is not the plan's problem");
    assert!(
        compiled.units.iter().all(|unit| unit.stratum != ghost),
        "and no unit is emitted for it, because no producer ranks there"
    );
}

// ---------------------------------------------------------------------------
// 6. Statistics staleness
// ---------------------------------------------------------------------------

#[test]
fn admission_rejects_stale_statistics() {
    let registry = fixture_registry();
    let planned = statistics("r1");
    let plan = fresh_plan(&registry, &planned);
    let moved = statistics("r2");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &moved,
        fusion_profile: None,
    };
    let error = compile(&plan, &env).expect_err("moved statistics are refused");
    match error {
        AdmissionError::StaleStatistics {
            plan_revision,
            current_revision,
        } => {
            assert_eq!(plan_revision, "r1");
            assert_eq!(current_revision, "r2");
        }
        other => panic!("expected StaleStatistics, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 7. Registry identity
// ---------------------------------------------------------------------------

#[test]
fn admission_rejects_registry_instance_mismatch() {
    let first = fixture_registry();
    let second = fixture_registry();
    assert_eq!(
        first.content_fingerprint().expect("fingerprint"),
        second.content_fingerprint().expect("fingerprint"),
        "the fixtures declare identical contents"
    );
    assert_ne!(first.instance_id(), second.instance_id());
    let stats = statistics("r1");
    let plan = fresh_plan(&first, &stats);
    let env = AdmissionEnvironment {
        registry: &second,
        statistics: &stats,
        fusion_profile: None,
    };
    let error = compile(&plan, &env).expect_err("a different live instance is refused");
    match error {
        AdmissionError::RegistryMismatch {
            expected_instance,
            got_instance,
        } => {
            assert_eq!(expected_instance, first.instance_id());
            assert_eq!(got_instance, second.instance_id());
        }
        other => panic!("expected RegistryMismatch, got {other:?}"),
    }
}

#[test]
fn admission_rejects_registry_fingerprint_mismatch() {
    let original = fixture_registry();
    let changed = registry_with_different_declaration();
    assert_ne!(
        original.content_fingerprint().expect("fingerprint"),
        changed.content_fingerprint().expect("fingerprint"),
        "the changed registry declares different contents"
    );
    let stats = statistics("r1");
    let plan = fresh_plan(&original, &stats);
    let env = AdmissionEnvironment {
        registry: &changed,
        statistics: &stats,
        fusion_profile: None,
    };
    let error = compile(&plan, &env).expect_err("a changed registry is refused");
    assert!(
        matches!(error, AdmissionError::RegistryFingerprintMismatch { .. }),
        "got {error:?}"
    );
}

#[test]
fn same_fingerprint_different_implementation_refused() {
    // Two independently built registries that declare byte-identical contents but
    // answer with different rows. The content fingerprint cannot see the
    // difference; the instance identity can, and admission refuses on it.
    let mut implementation_a = fixture_registry();
    let mut implementation_b = fixture_registry();
    implementation_a.register_ranked(
        ex("pf/impl"),
        producer(7, "impl-a/", 1),
        ranked(
            &ex("stratum/impl"),
            vec![TermPattern::of_kind(TermKind::Any)],
            true,
        ),
    );
    implementation_b.register_ranked(
        ex("pf/impl"),
        producer(7, "impl-b/", 1),
        ranked(
            &ex("stratum/impl"),
            vec![TermPattern::of_kind(TermKind::Any)],
            true,
        ),
    );
    assert_eq!(
        implementation_a.content_fingerprint().expect("fingerprint"),
        implementation_b.content_fingerprint().expect("fingerprint")
    );
    let stats = statistics("r1");
    let plan = fresh_plan(&implementation_a, &stats);
    let env = AdmissionEnvironment {
        registry: &implementation_b,
        statistics: &stats,
        fusion_profile: None,
    };
    let error = compile(&plan, &env).expect_err("same fingerprint, different implementation");
    assert!(
        matches!(error, AdmissionError::RegistryMismatch { .. }),
        "got {error:?}"
    );
}

// ---------------------------------------------------------------------------
// 7b. A plan that crossed a process boundary
//
// The instance id is a per-process counter, so a plan that was serialized and
// reloaded carries a number no live registry can ever match. Holding such a plan
// to it would refuse every portable plan there is, which is why the plan records
// its origin and the durable content fingerprint carries the claim instead.
//
// These are pairs: the plan that must be admitted, and the neighbouring one that
// must still be refused, so neither the refusal nor the relaxation can drift.
// ---------------------------------------------------------------------------

/// The canonical-bytes path and the serde path must agree, so each of the two
/// decoders is exercised against the same rebuilt registry.
fn decoded_both_ways(plan: &Plan) -> Vec<(&'static str, Plan)> {
    let canonical =
        Plan::from_canonical_bytes(&plan.canonical_bytes()).expect("canonical bytes decode");
    let json = serde_json::to_string(plan).expect("a plan serializes");
    let serde_decoded: Plan = serde_json::from_str(&json).expect("a plan deserializes");
    vec![("canonical", canonical), ("serde", serde_decoded)]
}

#[test]
fn a_deserialized_plan_is_admitted_against_a_rebuilt_registry() {
    let planned_against = fixture_registry();
    let stats = statistics("r1");
    let plan = fresh_plan(&planned_against, &stats);
    let original = compile(
        &plan,
        &AdmissionEnvironment {
            registry: &planned_against,
            statistics: &stats,
            fusion_profile: None,
        },
    )
    .expect("the fresh plan is admitted in its own process");

    // Stand in for the reload: the registry the plan was planned against is
    // gone, and an independently built one declaring the same relations takes
    // its place. Its instance id is a fresh counter value, so it cannot match.
    drop(planned_against);
    let rebuilt = fixture_registry();
    let env = AdmissionEnvironment {
        registry: &rebuilt,
        statistics: &stats,
        fusion_profile: None,
    };

    for (path, decoded) in decoded_both_ways(&plan) {
        assert_eq!(
            decoded.origin,
            PlanOrigin::Deserialized,
            "{path}: the decoder records that the plan crossed a boundary"
        );
        assert_ne!(
            decoded.registry_instance_id,
            rebuilt.instance_id(),
            "{path}: a reloaded counter can never name a freshly minted registry"
        );
        let compiled = compile(&decoded, &env)
            .unwrap_or_else(|error| panic!("{path}: a reloaded plan must admit, got {error:?}"));
        assert_eq!(
            compiled.plan_id,
            plan.id(),
            "{path}: it is still the same pinned plan"
        );
        assert_eq!(
            compiled.units, original.units,
            "{path}: and it compiles to the same text the planning process emitted"
        );
        assert_eq!(
            compiled.registry_id,
            rebuilt.instance_id(),
            "{path}: the units are pinned to the registry that admitted them"
        );
    }
}

#[test]
fn a_deserialized_plan_against_different_declarations_is_refused() {
    // The neighbour of the test above: same reload, but the registry now
    // declares something else, which the durable fingerprint sees. Relaxing the
    // instance check must not have relaxed this one.
    let planned_against = fixture_registry();
    let stats = statistics("r1");
    let plan = fresh_plan(&planned_against, &stats);
    drop(planned_against);

    let changed = registry_with_different_declaration();
    let env = AdmissionEnvironment {
        registry: &changed,
        statistics: &stats,
        fusion_profile: None,
    };
    for (path, decoded) in decoded_both_ways(&plan) {
        let error = compile(&decoded, &env).expect_err("a moved registry is refused");
        assert_eq!(
            error.dimension(),
            "registry_fingerprint_mismatch",
            "{path}: the durable dimension is the one that refuses, got {error:?}"
        );
        match error {
            AdmissionError::RegistryFingerprintMismatch { expected, got } => {
                assert_eq!(expected, plan.registry_content_fingerprint);
                assert_eq!(got, changed.content_fingerprint().expect("fingerprint"));
            }
            other => panic!("{path}: expected a fingerprint mismatch, got {other:?}"),
        }
    }
}

#[test]
fn origin_is_what_selects_the_identity_a_plan_is_held_to() {
    // The same decoded plan, the same rebuilt registry, and only the origin
    // differs: as `Deserialized` it is admitted on the fingerprint, and as
    // `SameProcess` it claims a live instance it does not name and is refused.
    // Editing origin therefore never removes a check — it chooses which of the
    // two registry identities applies, and this direction is the stricter one.
    let planned_against = fixture_registry();
    let stats = statistics("r1");
    let plan = fresh_plan(&planned_against, &stats);
    drop(planned_against);

    let rebuilt = fixture_registry();
    let env = AdmissionEnvironment {
        registry: &rebuilt,
        statistics: &stats,
        fusion_profile: None,
    };
    let mut decoded =
        Plan::from_canonical_bytes(&plan.canonical_bytes()).expect("canonical bytes decode");
    compile(&decoded, &env).expect("as a deserialized plan it is admitted");

    decoded.origin = PlanOrigin::SameProcess;
    let error =
        compile(&decoded, &env).expect_err("as a same-process plan it names no live instance");
    match error {
        AdmissionError::RegistryMismatch {
            expected_instance,
            got_instance,
        } => {
            assert_eq!(expected_instance, plan.registry_instance_id);
            assert_eq!(got_instance, rebuilt.instance_id());
        }
        other => panic!("expected RegistryMismatch, got {other:?}"),
    }
}

#[test]
fn a_same_process_plan_is_still_held_to_its_live_instance() {
    // The relaxation above must not have reached the plans that never left the
    // process: two registries that declare byte-identical contents can register
    // one IRI to two implementations that answer differently, and only the
    // instance id sees that. The fingerprints here are equal on purpose.
    let first = fixture_registry();
    let second = fixture_registry();
    assert_eq!(
        first.content_fingerprint().expect("fingerprint"),
        second.content_fingerprint().expect("fingerprint")
    );
    let stats = statistics("r1");
    let plan = fresh_plan(&first, &stats);
    assert_eq!(plan.origin, PlanOrigin::SameProcess);

    let error = compile(
        &plan,
        &AdmissionEnvironment {
            registry: &second,
            statistics: &stats,
            fusion_profile: None,
        },
    )
    .expect_err("a same-process plan names a live instance, and this is not it");
    assert_eq!(error.dimension(), "registry_instance_mismatch");

    // And its own registry still admits it, which is the valid neighbour.
    compile(
        &plan,
        &AdmissionEnvironment {
            registry: &first,
            statistics: &stats,
            fusion_profile: None,
        },
    )
    .expect("the registry it was planned against admits it");
}

// ---------------------------------------------------------------------------
// 7c. Every bound producer reaches the emitted text
//
// `compile` emits one unit per stratum **depth** entry and looks that stratum's
// bindings up from there, so a binding whose stratum carries no depth is never
// visited. Nothing about the emitted text would look wrong: it parses, it runs,
// and it answers less than the registry promised. Admission therefore enforces
// the implication in both directions.
// ---------------------------------------------------------------------------

#[test]
fn admission_refuses_a_binding_whose_stratum_has_no_depth() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let mut plan = fresh_plan(&registry, &stats);
    // The text producer stays bound; only its stratum's depth is removed — an
    // edit that is invisible in the plan's binding list.
    let removed = plan
        .stratum_depths
        .remove(&iri(&ex("stratum/text")))
        .expect("the planner recorded a depth for the text stratum");
    assert!(
        plan.producer_bindings
            .iter()
            .any(|binding| binding.producer == ex("pf/literal")),
        "the producer whose depth was removed is still bound"
    );

    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let error = compile(&plan, &env).expect_err("a producer that would never compile is refused");
    assert_eq!(error.dimension(), "malformed_plan");
    match error {
        AdmissionError::MalformedPlan { reason } => {
            assert!(
                reason.contains(&ex("pf/literal")) && reason.contains(&ex("stratum/text")),
                "the refusal names the producer and the stratum: {reason}"
            );
        }
        other => panic!("expected MalformedPlan, got {other:?}"),
    }

    // Restoring the depth restores the plan: the refusal is about that one edit
    // and nothing else.
    plan.stratum_depths
        .insert(iri(&ex("stratum/text")), removed);
    compile(&plan, &env).expect("the plan admits once every bound stratum has its depth again");
}

#[test]
fn every_bound_stratum_with_a_depth_emits_its_branch() {
    // The valid neighbour, stated positively: a plan whose every bound stratum
    // carries a depth is admitted, and every binding it records appears in the
    // emitted text. That is the property the refusal above protects.
    let registry = fixture_registry();
    let stats = statistics("r1");
    let plan = fresh_plan(&registry, &stats);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&plan, &env).expect("the plan admits");

    assert_eq!(plan.producer_bindings.len(), 3, "three producers are bound");
    for binding in &plan.producer_bindings {
        assert!(
            plan.stratum_depths.contains_key(&binding.stratum),
            "every bound stratum carries a depth"
        );
        let unit = compiled
            .units
            .iter()
            .find(|unit| unit.stratum == binding.stratum)
            .unwrap_or_else(|| panic!("stratum {} emits a unit", binding.stratum));
        assert!(
            unit.sparql.contains(&binding.producer),
            "producer {} has a branch in its stratum's unit: {}",
            binding.producer,
            unit.sparql
        );
    }
}

// ---------------------------------------------------------------------------
// 7d. An undeclared row bound is not a bound of zero
//
// A producer that declares no access mode declares no worst-case row count.
// Reading that back as zero bounds its stratum at zero rows and refuses every
// positive depth — a refusal that also misreports the cause, telling the host
// its registry bounds the stratum at nothing.
// ---------------------------------------------------------------------------

#[test]
fn a_stratum_whose_producer_declares_no_mode_bounds_no_depth() {
    let registry = registry_with_a_modeless_producer();
    let stats = statistics("r1");
    let mut plan = fresh_plan(&registry, &stats);
    // Placement refuses a producer with no declared mode, so the planner never
    // binds it; the stratum is still declared, and a plan may carry a depth for
    // it exactly as it may for any other declared stratum.
    assert!(
        plan.producer_decisions.iter().any(|decision| matches!(
            decision,
            ProducerDecision::Rejected { producer, reason }
                if producer == &ex("pf/modeless")
                    && *reason == RejectionReason::UnsatisfiedConstraint
        )),
        "the modeless producer is recorded as rejected: {:?}",
        plan.producer_decisions
    );
    plan.stratum_depths.insert(iri(&ex("stratum/modeless")), 25);

    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&plan, &env)
        .expect("a stratum whose producers declared no row count bounds no depth");
    assert!(
        compiled
            .units
            .iter()
            .all(|unit| unit.stratum != iri(&ex("stratum/modeless"))),
        "and it emits nothing, having no bound producer"
    );
}

#[test]
fn a_declared_row_bound_still_refuses_a_raised_depth() {
    // The neighbour of the relaxation above, in the same registry: a stratum
    // whose producer *did* declare a row count is still held to it exactly.
    let registry = registry_with_a_modeless_producer();
    let stats = statistics("r1");
    let mut plan = fresh_plan(&registry, &stats);
    plan.stratum_depths
        .insert(iri(&ex("stratum/universal")), 201);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let error = compile(&plan, &env).expect_err("a raised depth is still refused");
    match error {
        AdmissionError::DepthBoundViolation {
            stratum,
            declared,
            requested,
        } => {
            assert_eq!(*stratum, iri(&ex("stratum/universal")));
            assert_eq!(declared, 200);
            assert_eq!(requested, 201);
        }
        other => panic!("expected DepthBoundViolation, got {other:?}"),
    }
}

#[test]
fn a_stratum_no_producer_ranks_under_still_bounds_a_depth_at_zero() {
    // The third case, pinned so it stays distinguishable from the two above: a
    // stratum the registry ranks nothing under declared a bound — of zero —
    // because nothing can rank there. That is not the same fact as a stratum
    // whose producers declared no row count, and it is refused where that one
    // is admitted.
    let registry = fixture_registry();
    let stats = statistics("r1");
    let mut plan = fresh_plan(&registry, &stats);
    plan.stratum_depths.insert(iri(&ex("stratum/ghost")), 5);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let error = compile(&plan, &env).expect_err("a depth for a stratum nothing ranks under");
    match error {
        AdmissionError::DepthBoundViolation {
            stratum,
            declared,
            requested,
        } => {
            assert_eq!(*stratum, iri(&ex("stratum/ghost")));
            assert_eq!(declared, 0);
            assert_eq!(requested, 5);
        }
        other => panic!("expected DepthBoundViolation, got {other:?}"),
    }

    // A depth of zero for the same stratum is an honest empty stratum, not a
    // violation, which is the neighbour that keeps the bound from being read as
    // "this stratum may not appear".
    plan.stratum_depths.insert(iri(&ex("stratum/ghost")), 0);
    compile(&plan, &env).expect("a zero depth is within a zero bound");
}

// ---------------------------------------------------------------------------
// 8. Execution: failure isolation and replay
// ---------------------------------------------------------------------------

/// Read an executed stream the way a caller that stopped at `execute` reads it:
/// one row at a time through the ranked-stream protocol, to exhaustion.
fn drain(mut stream: RankedStreamImpl) -> Vec<(u64, Term)> {
    let mut rows = Vec::new();
    while let Some(row) = block_on(stream.next()).expect("a materialized stream obeys the protocol")
    {
        rows.push(row);
    }
    rows
}

fn rows_by_stratum(result: purrdf_retrieval::ExecutionResult) -> BTreeMap<Iri, Vec<(u64, Term)>> {
    result
        .streams
        .into_iter()
        .map(|stream| (stream.stratum, drain(stream.stream)))
        .collect()
}

#[test]
fn one_stratum_failure_others_continue() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let plan = fresh_plan(&registry, &stats);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let mut compiled = compile(&plan, &env).expect("the plan admits");
    let failing = compiled.units[1].stratum.clone();
    compiled.units[1].sparql = "THIS IS NOT SPARQL".to_owned();

    let result = block_on(execute(&compiled, &registry, &*common::empty_dataset()))
        .expect("execution starts");
    match result.statuses.get(&failing) {
        Some(ProducerStatus::ExecutionFailed { .. }) => {}
        other => panic!("the failing stratum is ExecutionFailed, got {other:?}"),
    }
    for unit in &compiled.units {
        if unit.stratum == failing {
            continue;
        }
        match result.statuses.get(&unit.stratum) {
            Some(ProducerStatus::Exhausted { .. }) => {}
            other => panic!(
                "stratum {} should have completed, got {other:?}",
                unit.stratum
            ),
        }
    }
    assert_eq!(
        result.streams.len(),
        compiled.units.len() - 1,
        "only the surviving strata stream"
    );
}

#[test]
fn pinned_plan_replay_reproduces_candidate_set_and_ranks() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let plan = fresh_plan(&registry, &stats);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let first = rows_by_stratum(
        block_on(execute(
            &compile(&plan, &env).expect("admits"),
            &registry,
            &*common::empty_dataset(),
        ))
        .expect("runs"),
    );
    let second = rows_by_stratum(
        block_on(execute(
            &compile(&plan, &env).expect("admits"),
            &registry,
            &*common::empty_dataset(),
        ))
        .expect("runs"),
    );
    assert_eq!(first, second, "same plan and registry replay identically");
    assert!(!first.is_empty());

    for (stratum, rows) in &first {
        let ranks: Vec<u64> = rows.iter().map(|(rank, _)| *rank).collect();
        let expected: Vec<u64> = (1..=u64::try_from(rows.len()).expect("length fits")).collect();
        assert_eq!(
            ranks, expected,
            "stratum {stratum} ranks are 1-based and contiguous"
        );
    }

    let universal = first
        .get(&iri(&ex("stratum/universal")))
        .expect("the universal stratum streamed");
    assert_eq!(universal.len(), 3, "the mandatory producer's rows");
    assert_eq!(universal[0].0, 1);
    assert_eq!(universal[2].0, 3);
}

#[test]
fn execute_refuses_a_different_registry_instance() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let plan = fresh_plan(&registry, &stats);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&plan, &env).expect("admits");
    let other = fixture_registry();
    let error = block_on(execute(&compiled, &other, &*common::empty_dataset()))
        .expect_err("a foreign registry is refused");
    assert!(matches!(
        error,
        purrdf_retrieval::ExecutionError::RegistryMismatch { .. }
    ));
}

/// Touch the `CompiledRetrieval` import so the type is exercised by name.
#[test]
fn compiled_retrieval_carries_both_identities() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let plan = fresh_plan(&registry, &stats);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled: CompiledRetrieval = compile(&plan, &env).expect("admits");
    assert_eq!(compiled.registry_id, registry.instance_id());
    assert_eq!(
        compiled.registry_fingerprint,
        plan.registry_content_fingerprint
    );
}

// ---------------------------------------------------------------------------
// 9. `mandatory` is a declaration, not an inference
//
// Admission reads the registry's own `mandatory` flag and nothing else. These
// three tests are pairs: the same plan, the same edit, the same request, run
// against two registries that differ only in that flag. Each pins the refusal
// AND its valid neighbour, because a coverage rule that is derived rather than
// declared fails silently in exactly one direction — it refuses plans that were
// always legitimate, and every test still passes.
// ---------------------------------------------------------------------------

/// A declaration with an explicit accepted-term list, so a test can control
/// what places and what does not.
fn declaration(stratum: &str, accepted: Vec<AcceptedTerm>, mandatory: bool) -> RankedDeclaration {
    RankedDeclaration {
        stratum: kernel_iri(stratum),
        accepted_terms: accepted,
        depth_placement: None,
        candidate_position: 0,
        duplicates: DuplicatePolicy::Unique,
        mandatory,
    }
}

/// One accepted alternative that renders the term's value at position 1.
fn renders_value() -> Vec<AcceptedTerm> {
    vec![AcceptedTerm {
        pattern: TermPattern::of_kind(TermKind::Any),
        placements: vec![TermPlacement {
            facet: RequestFacet::Value,
            position: 1,
            datatype: None,
        }],
    }]
}

/// One accepted alternative that accepts every shape and declares no placement:
/// it is matched by every request term and writes none of them, so its argument
/// stays free and the emitted call carries no part of the request. A producer
/// declared this way is selected and emitted, and is bound to **no** term.
fn renders_nothing() -> Vec<AcceptedTerm> {
    vec![AcceptedTerm {
        pattern: TermPattern::of_kind(TermKind::Any),
        placements: Vec::new(),
    }]
}

/// One `Value` placement at `position`, untyped.
fn value_at(position: usize) -> Vec<TermPlacement> {
    vec![TermPlacement {
        facet: RequestFacet::Value,
        position,
        datatype: None,
    }]
}

/// A mock producer of arity `(1, object)`, emitting `count` rows of that width.
///
/// Distinct from [`producer`] because that one's row shape — and the candidate
/// IRIs read out of position 0 — is asserted verbatim by the execution tests;
/// this is the wider relation a producer that receives more than one term needs.
fn producer_of_arity(
    rows: u64,
    prefix: &str,
    count: usize,
    object: usize,
) -> Arc<dyn PropertyFunction> {
    let arity = PfArity::new(1, object);
    let total = arity.total();
    let emitted = (0..count)
        .map(|index| {
            (0..total)
                .map(|position| {
                    TermValue::iri(format!("{}entity{index}/pos{position}", ex(prefix)))
                })
                .collect()
        })
        .collect();
    Arc::new(MockProducer {
        arity,
        mode: arity.all_free_mode(),
        rows,
        emitted,
    })
}

/// `pf/catch-all` — two *distinguishable* accepted shapes, each placed into its
/// own argument position — beside a literal-only producer in its own stratum, so
/// dropping the catch-all still leaves a unit to emit.
///
/// Two terms of the **same** shape cannot both be received: a placement is
/// declared per accepted shape, so a second literal would contend for the
/// first's position and the invocation would be refused. So a producer that
/// genuinely receives more than one term of a request is one that takes more
/// than one shape, which is what this declares. The requests these fixtures use
/// against it carry a literal and an IRI seed for exactly that reason.
fn catch_all_registry(mandatory: bool) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/catch-all"),
        producer_of_arity(200, "catch/", 3, 2),
        declaration(
            &ex("stratum/catch"),
            vec![
                AcceptedTerm {
                    pattern: TermPattern::of_kind(TermKind::Literal),
                    placements: value_at(1),
                },
                AcceptedTerm {
                    pattern: TermPattern::of_kind(TermKind::Iri),
                    placements: value_at(2),
                },
            ],
            mandatory,
        ),
    );
    registry.register_ranked(
        ex("pf/literal"),
        producer(100, "text/", 2),
        ranked(
            &ex("stratum/text"),
            vec![TermPattern::of_kind(TermKind::Literal)],
            false,
        ),
    );
    registry
}

/// `pf/renders` places a request term's value, so a vector term — which has no
/// SPARQL constant form — makes it unplaceable. `pf/free` sits in a stratum of
/// its own and places nothing, so the plan stays viable and the first producer's
/// own fate is readable rather than collapsed into `NoApplicableProducers`.
///
/// The two are in separate strata because a stratum carries one producer; what
/// this fixture needs from `pf/free` is only that *something* survives planning.
fn unplaceable_registry(mandatory: bool) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/renders"),
        producer(30, "renders/", 2),
        declaration(&ex("stratum/render"), renders_value(), mandatory),
    );
    registry.register_ranked(
        ex("pf/free"),
        producer(30, "free/", 2),
        declaration(&ex("stratum/free"), renders_nothing(), false),
    );
    registry
}

/// A registry whose literal-only producer is declared mandatory by the host: it
/// accepts a lexical term and nothing else, so it can serve a whole request only
/// when the whole request is lexical.
fn literal_only_registry(mandatory: bool) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/literal"),
        producer(100, "text/", 2),
        declaration(
            &ex("stratum/text"),
            vec![AcceptedTerm {
                pattern: TermPattern::of_kind(TermKind::Literal),
                placements: vec![TermPlacement {
                    facet: RequestFacet::Value,
                    position: 1,
                    datatype: None,
                }],
            }],
            mandatory,
        ),
    );
    registry
}

/// Narrow the catch-all's binding to the request's first term only, and admit.
fn admit_narrowed(registry: &PropertyFunctionRegistry) -> Result<(), AdmissionError> {
    let stats = statistics("r1");
    let mut plan = purrdf_retrieval::plan(&lexical_and_seed_request(), registry, &stats)
        .expect("the request plans");
    let binding = plan
        .producer_bindings
        .iter_mut()
        .find(|binding| binding.producer == ex("pf/catch-all"))
        .expect("the catch-all is bound to the whole request");
    assert_eq!(
        binding.request_terms,
        vec![0, 1],
        "the planner binds a producer to every term it accepts AND places"
    );
    binding.request_terms.truncate(1);
    let env = AdmissionEnvironment {
        registry,
        statistics: &stats,
        fusion_profile: None,
    };
    compile(&plan, &env).map(|_| ())
}

#[test]
fn narrowing_a_producer_is_refused_only_when_the_registry_declared_it_mandatory() {
    // The valid neighbour, and the whole point of the change: a permissive
    // producer the registry did not declare mandatory may legitimately be bound
    // to a SUBSET of the request. A rule that derived "mandatory" from the
    // producer's own patterns made this plan impossible to admit, which is a
    // refusal of something that was never wrong.
    admit_narrowed(&catch_all_registry(false))
        .expect("a producer the registry did not declare mandatory may be narrowed");

    // The same edit, against a registry that did declare it: refused, loudly,
    // naming the shortfall.
    let error = admit_narrowed(&catch_all_registry(true))
        .expect_err("a mandatory producer must receive every request term");
    assert_eq!(error.dimension(), "insufficient_bindings");
    match error {
        AdmissionError::InsufficientBindings {
            producer,
            required,
            provided,
        } => {
            assert_eq!(producer.as_str(), ex("pf/catch-all"));
            assert_eq!(required, 2, "it accepts and places both terms");
            assert_eq!(provided, 1);
        }
        other => panic!("expected InsufficientBindings, got {other:?}"),
    }
}

#[test]
fn a_producer_placement_refuses_is_dropped_unless_the_registry_declared_it_mandatory() {
    let request = RetrievalRequest::from_terms(vec![vector_term()]);

    // Not declared mandatory: the planner records why it could not be invoked
    // and drops it, and the plan is admitted on the strength of what remains.
    let registry = unplaceable_registry(false);
    let stats = statistics("r1");
    let plan = purrdf_retrieval::plan(&request, &registry, &stats).expect("the request plans");
    assert!(
        plan.producer_decisions
            .contains(&ProducerDecision::Rejected {
                producer: ex("pf/renders"),
                reason: RejectionReason::UnsatisfiedConstraint,
            }),
        "the rejection is recorded with its own reason: {:?}",
        plan.producer_decisions
    );
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&plan, &env).expect("a dropped optional producer is not a refusal");
    assert_eq!(
        compiled.units.len(),
        1,
        "the surviving producer still emits"
    );
    assert!(compiled.units[0].sparql.contains(&ex("pf/free")));
    assert!(
        !compiled.units[0].sparql.contains(&ex("pf/renders")),
        "and the dropped producer is not in the text: {}",
        compiled.units[0].sparql
    );
    // The survivor declares no placement, so it is NOT serving the vector: the
    // plan says so per term, and the text it emits carries no embedding. Naming
    // the producer is not the same as transporting the request, and the two
    // assertions below are what tells them apart.
    //
    // The reason is the narrower of the two that are true here. `pf/renders`
    // would have carried the embedding — it declares a value placement — and
    // was rejected at placement for want of a datatype, which is a recorded,
    // fixable refusal; `pf/free` merely declared nowhere to put it. The reader
    // is sent to the refusal that names a producer.
    assert_eq!(
        plan.unserved_evidence(),
        vec![UnservedTerm {
            request_term: 0,
            reason: UnservedReason::EveryAcceptingProducerRejected,
        }],
        "the one producer that could have carried the embedding was rejected"
    );
    assert!(
        !compiled.units[0]
            .sparql
            .contains(&purrdf_retrieval::encode_embedding(&[0.25, -1.5, 3.0])),
        "and the embedding is nowhere in it: {}",
        compiled.units[0].sparql
    );

    // Declared mandatory: the very same plan is refused, because the registry
    // promised that producer would serve every request and this one cannot
    // reach it.
    let registry = unplaceable_registry(true);
    let plan = purrdf_retrieval::plan(&request, &registry, &stats).expect("the request plans");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let error = compile(&plan, &env).expect_err("a mandatory producer cannot be dropped");
    assert_eq!(error.dimension(), "missing_mandatory_producer");
    match error {
        AdmissionError::MissingMandatoryProducer { producer } => {
            assert_eq!(producer.as_str(), ex("pf/renders"));
        }
        other => panic!("expected MissingMandatoryProducer, got {other:?}"),
    }
}

#[test]
fn a_plan_that_binds_two_producers_to_one_stratum_is_refused() {
    // The plan-side face of the registry's one-stratum-one-producer rule. The
    // registry refuses a second producer's *declaration*, but a plan is editable,
    // so it can still carry two bindings under one stratum — the same producer
    // twice, with two term sets — and that compiles to the same concatenation,
    // where the second branch's rank-1 row lands below the whole of the first.
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };

    // The neighbouring valid case first: one binding per stratum, admitted.
    let plan = fresh_plan(&registry, &stats);
    compile(&plan, &env).expect("one binding per stratum is the ordinary plan");

    let mut doubled = plan;
    let repeat = doubled
        .producer_bindings
        .iter()
        .find(|binding| binding.producer == ex("pf/any"))
        .expect("the catch-all is bound")
        .clone();
    doubled.producer_bindings.push(repeat);

    let error = compile(&doubled, &env).expect_err("one stratum carries one producer");
    assert_eq!(error.dimension(), "malformed_plan");
    match error {
        AdmissionError::MalformedPlan { reason } => {
            assert!(
                reason.contains(&ex("stratum/universal")),
                "the refusal names the stratum: {reason}"
            );
            assert!(
                reason.contains(&ex("pf/any")),
                "and the producers under it: {reason}"
            );
        }
        other => panic!("expected MalformedPlan, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 9b. What `mandatory` quantifies over
//
// Mandatory means "every request term this producer's own patterns accept", not
// "every request term". The difference is invisible for the one producer shape
// where it cannot bite — a catch-all `TermKind::Any`, which accepts everything,
// so both readings say the same thing — and decisive for every real one: a
// full-text producer accepts literals and cannot accept an entity seed, so under
// the wider reading a coverage-floor host had every multimodal request refused.
// Each test below is a pair, because a quantifier that is too wide fails in the
// direction where every test still passes.
// ---------------------------------------------------------------------------

/// A lexical term beside an entity seed: the multimodal request a coverage-floor
/// host asks for, and the one the wider quantifier refused.
fn lexical_and_seed_request() -> RetrievalRequest {
    RetrievalRequest::from_terms(vec![lexical_term(), seed_term()])
}

/// The declared-mandatory text producer's binding in `plan`, if it has one.
fn literal_binding(plan: &Plan) -> Option<&purrdf_retrieval::ProducerBinding> {
    plan.producer_bindings
        .iter()
        .find(|binding| binding.producer == ex("pf/literal"))
}

/// The catch-all producer's binding in `plan`, if it has one.
fn catch_all_binding(plan: &Plan) -> Option<&purrdf_retrieval::ProducerBinding> {
    plan.producer_bindings
        .iter()
        .find(|binding| binding.producer == ex("pf/catch-all"))
}

/// A request the catch-all accepts two of and the third of which it accepts
/// not at all: a literal, an IRI seed, and an RDF 1.2 quoted-triple seed.
///
/// The quoted triple is what makes the mandatory quantifier visible. Its kind is
/// `Triple`, which neither of the catch-all's alternatives names, so it is in
/// neither the required count nor the provided one.
fn literal_seed_and_triple_request() -> RetrievalRequest {
    RetrievalRequest::from_terms(vec![
        lexical_term(),
        seed_term(),
        RequestTerm::EntitySeed {
            entity: Term::new(format!("<<( <{}> <{}> \"o\" )>>", ex("s"), ex("p"))),
        },
    ])
}

#[test]
fn a_mandatory_text_producer_admits_a_request_it_can_only_partly_accept() {
    // The adopter's exact case. A coverage-floor host declares its full-text
    // producer mandatory, then issues `Lexical + EntitySeed`. The text producer's
    // patterns cannot accept a seed — no full-text producer's can — so under the
    // old quantifier this was `InsufficientBindings` and withdrawing `mandatory`
    // admitted the identical plan. The declaration was never the problem.
    let stats = statistics("r1");
    let registry = literal_only_registry(true);
    let plan = purrdf_retrieval::plan(&lexical_and_seed_request(), &registry, &stats)
        .expect("the lexical term reaches the producer, so the request plans");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    compile(&plan, &env).expect("a term the producer cannot accept is not its business");
    assert_eq!(
        literal_binding(&plan)
            .expect("the text producer is bound")
            .request_terms,
        vec![0],
        "bound to the lexical term only; the seed is no part of what it promised"
    );

    // And the seed is not quietly lost: nothing in this registry accepts it, and
    // the plan says so by index rather than by omission.
    assert_eq!(
        plan.unserved_evidence(),
        vec![UnservedTerm {
            request_term: 1,
            reason: UnservedReason::NoProducerAccepts,
        }]
    );
}

#[test]
fn a_mandatory_producer_dropped_from_a_term_it_accepts_is_refused_by_name() {
    // The protection `mandatory` exists for, on the very plan the test above
    // admits: the host promised this producer would serve what it can of every
    // request, and this edit takes the one term it can serve away from it.
    let stats = statistics("r1");
    let registry = literal_only_registry(true);
    let mut plan = purrdf_retrieval::plan(&lexical_and_seed_request(), &registry, &stats)
        .expect("the request plans");
    plan.producer_bindings
        .iter_mut()
        .find(|binding| binding.producer == ex("pf/literal"))
        .expect("the text producer is bound")
        .request_terms
        .clear();

    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let error = compile(&plan, &env).expect_err("the accepted term cannot be taken away");
    assert_eq!(error.dimension(), "insufficient_bindings");
    match error {
        AdmissionError::InsufficientBindings {
            producer,
            required,
            provided,
        } => {
            assert_eq!(producer.as_str(), ex("pf/literal"));
            assert_eq!(required, 1, "one term of the request is a literal");
            assert_eq!(provided, 0, "and the edit left it bound to none of them");
        }
        other => panic!("expected InsufficientBindings, got {other:?}"),
    }
}

#[test]
fn a_mandatory_producer_bound_to_some_of_what_it_accepts_is_refused() {
    // A literal, an IRI seed and a quoted-triple seed. The producer accepts and
    // places the first two and accepts the third not at all, so "every term it
    // receives" is two — and a plan that binds it to one of them is the partial
    // coverage the flag forbids.
    let stats = statistics("r1");
    let registry = catch_all_registry(true);
    let request = literal_seed_and_triple_request();
    let plan =
        purrdf_retrieval::plan(&request, &registry, &stats).expect("two of the three reach it");
    assert_eq!(
        catch_all_binding(&plan)
            .expect("the catch-all is bound")
            .request_terms,
        vec![0, 1],
        "the planner binds it to the two shapes it places, not to the quoted triple"
    );
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    // The neighbouring valid case first: the unedited plan, which covers every
    // literal and no seed, is admitted.
    compile(&plan, &env).expect("full coverage of what it accepts is admitted");

    let mut narrowed = plan;
    narrowed
        .producer_bindings
        .iter_mut()
        .find(|binding| binding.producer == ex("pf/catch-all"))
        .expect("the catch-all is bound")
        .request_terms
        .truncate(1);
    let error = compile(&narrowed, &env).expect_err("half the accepted terms is not all of them");
    match error {
        AdmissionError::InsufficientBindings {
            producer,
            required,
            provided,
        } => {
            assert_eq!(producer.as_str(), ex("pf/catch-all"));
            assert_eq!(
                required, 2,
                "two of the three terms are shapes it accepts and places"
            );
            assert_eq!(provided, 1);
        }
        other => panic!("expected InsufficientBindings, got {other:?}"),
    }

    // And the same edit is admitted where the host made no such claim, so the
    // refusal is the declaration's rather than the shape's.
    let optional = catch_all_registry(false);
    let mut plan = purrdf_retrieval::plan(&request, &optional, &stats).expect("plans");
    plan.producer_bindings
        .iter_mut()
        .find(|binding| binding.producer == ex("pf/catch-all"))
        .expect("the catch-all is bound")
        .request_terms
        .truncate(1);
    let env = AdmissionEnvironment {
        registry: &optional,
        statistics: &stats,
        fusion_profile: None,
    };
    compile(&plan, &env).expect("an optional producer may cover part of a request");
}

#[test]
fn a_mandatory_producer_that_places_nothing_must_be_present_and_serves_no_term() {
    // The quantifier at its other edge. `fixture_registry`'s mandatory producer
    // accepts every shape and declares no placement, so it is called with the
    // whole request absent from its arguments: it ranks within its own stratum
    // without reading the request. Two things follow, and both are checked here.
    //
    // It is bound to NO term, because a binding is the claim that the producer
    // received the term — and every term it matched without placing is named in
    // the plan's own per-term evidence instead. An empty unserved list has to
    // mean the emitted text carries every term, and for this producer the text
    // carries none.
    let stats = statistics("r1");
    let registry = fixture_registry();
    let plan = fresh_plan(&registry, &stats);
    let catch_all = plan
        .producer_bindings
        .iter()
        .find(|binding| binding.producer == mandatory())
        .expect("a mandatory producer that accepts something of the request is selected");
    assert_eq!(
        catch_all.request_terms,
        Vec::<u32>::new(),
        "it writes none of the request, so it serves none of it"
    );
    assert_eq!(
        plan.unserved_evidence(),
        vec![UnservedTerm {
            request_term: 1,
            reason: UnservedReason::AcceptedWithoutPlacement,
        }],
        "the vector term reached only a producer that declared nowhere to put it; \
         the other two are served by producers that do place them"
    );

    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled =
        compile(&plan, &env).expect("a producer required to receive nothing is admitted");
    let universal = compiled
        .units
        .iter()
        .find(|unit| unit.stratum == iri(&ex("stratum/universal")))
        .expect("its stratum still emits");
    assert!(
        universal.sparql.contains("( ?c0 ) <") && universal.sparql.contains("( ?c1 )"),
        "and it is emitted with both arguments free: {}",
        universal.sparql
    );

    // Presence is still enforced, and that is the whole of what `mandatory`
    // buys for such a producer: editing it out is refused by name.
    let mut dropped = plan;
    dropped
        .producer_bindings
        .retain(|binding| binding.producer != mandatory());
    match compile(&dropped, &env).expect_err("a mandatory producer cannot be edited out") {
        AdmissionError::MissingMandatoryProducer { producer } => {
            assert_eq!(producer.as_str(), mandatory());
        }
        other => panic!("expected MissingMandatoryProducer, got {other:?}"),
    }
}

#[test]
fn a_mandatory_producer_that_accepts_nothing_of_a_request_is_not_required_by_it() {
    // The same quantifier, read at zero. A request the producer accepts no term
    // of has no term to have dropped it from, so there is no coverage claim for
    // the plan to falsify — reporting it missing would be the wider quantifier
    // again, one layer up, refusing a request the host's own declaration says
    // nothing about.
    let stats = statistics("r1");
    let mut registry = literal_only_registry(true);
    // Something must survive planning, and it must accept the seed the text
    // producer cannot; its own stratum, because a stratum carries one producer.
    registry.register_ranked(
        ex("pf/iri"),
        producer(50, "graph/", 1),
        ranked(
            &ex("stratum/graph"),
            vec![TermPattern::of_kind(TermKind::Iri)],
            false,
        ),
    );
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };

    let seed_only = RetrievalRequest::from_terms(vec![seed_term()]);
    let plan = purrdf_retrieval::plan(&seed_only, &registry, &stats).expect("the seed plans");
    assert!(
        literal_binding(&plan).is_none(),
        "the text producer accepts no term of this request, so it is not bound"
    );
    compile(&plan, &env).expect("a request that is no part of its business is admitted");

    // The neighbouring case that must still refuse: add a term it DOES accept,
    // and drop it from the plan, and the promise bites.
    let mut plan = purrdf_retrieval::plan(&lexical_and_seed_request(), &registry, &stats)
        .expect("the request plans");
    plan.producer_bindings
        .retain(|binding| binding.producer != ex("pf/literal"));
    match compile(&plan, &env).expect_err("dropping it from a term it accepts is refused") {
        AdmissionError::MissingMandatoryProducer { producer } => {
            assert_eq!(producer.as_str(), ex("pf/literal"));
        }
        other => panic!("expected MissingMandatoryProducer, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// A plan that contradicts itself about which terms went unanswered
//
// The recorded unserved list is evidence a caller reads back as "nothing
// answered this". A plan is editable, so the list is untrusted like every other
// field, and the waist refuses the contradictions no legitimate edit produces —
// while admitting the one a legitimate edit does.
// ---------------------------------------------------------------------------

#[test]
fn a_plan_that_both_binds_a_term_and_reports_it_unanswered_is_refused() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };

    let mut plan = fresh_plan(&registry, &stats);
    assert!(
        plan.producer_bindings
            .iter()
            .any(|binding| binding.request_terms.contains(&0)),
        "term 0 is bound"
    );
    plan.unserved_terms.push(UnservedTerm {
        request_term: 0,
        reason: UnservedReason::NoProducerAccepts,
    });

    let error = compile(&plan, &env).expect_err("a plan cannot both serve a term and report it");
    assert_eq!(error.dimension(), "malformed_plan");
    match error {
        AdmissionError::MalformedPlan { reason } => assert!(
            reason.contains("unserved") && reason.contains("binds"),
            "the refusal names the contradiction: {reason}"
        ),
        other => panic!("expected MalformedPlan, got {other:?}"),
    }

    // An index that names no term of this request is the other way the value can
    // be wrong, and it is refused on the same dimension.
    let mut plan = fresh_plan(&registry, &stats);
    plan.unserved_terms.push(UnservedTerm {
        request_term: 99,
        reason: UnservedReason::NoProducerAccepts,
    });
    assert!(
        matches!(
            compile(&plan, &env),
            Err(AdmissionError::MalformedPlan { .. })
        ),
        "an out-of-range index addresses nothing"
    );

    // Recorded twice, disagreeing with itself about one term. The catch-all
    // registry declares nothing mandatory, so narrowing it is admissible and the
    // only thing left to refuse is the duplicate.
    let optional = catch_all_registry(false);
    let env = AdmissionEnvironment {
        registry: &optional,
        statistics: &stats,
        fusion_profile: None,
    };
    let mut plan =
        purrdf_retrieval::plan(&lexical_and_seed_request(), &optional, &stats).expect("plans");
    for binding in &mut plan.producer_bindings {
        binding.request_terms.retain(|index| *index == 0);
    }
    for reason in [
        UnservedReason::NoProducerAccepts,
        UnservedReason::EveryAcceptingProducerRejected,
    ] {
        plan.unserved_terms.push(UnservedTerm {
            request_term: 1,
            reason,
        });
    }
    let error = compile(&plan, &env).expect_err("one term cannot have two reasons");
    assert_eq!(error.dimension(), "malformed_plan");
    match error {
        AdmissionError::MalformedPlan { reason } => assert!(
            reason.contains("more than once"),
            "the refusal names the duplicate: {reason}"
        ),
        other => panic!("expected MalformedPlan, got {other:?}"),
    }
}

#[test]
fn narrowing_a_producer_leaves_the_plan_admissible_and_the_term_reported() {
    // The valid neighbour, and the reason the converse is NOT enforced: an edit
    // that narrows a producer the registry does not declare mandatory is
    // admitted, even though it strands terms the recorded list never mentions.
    // Refusing that would refuse the edit the waist explicitly allows — and the
    // stranded terms are not lost either, because the evidence is derived from
    // the bindings in hand.
    let registry = catch_all_registry(false);
    let stats = statistics("r1");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };

    let mut plan =
        purrdf_retrieval::plan(&lexical_and_seed_request(), &registry, &stats).expect("plans");
    assert!(
        plan.unserved_terms.is_empty(),
        "the catch-all receives both terms, so the planner recorded nothing"
    );
    for binding in &mut plan.producer_bindings {
        binding.request_terms.retain(|index| *index == 0);
    }

    compile(&plan, &env)
        .expect("narrowing a producer the registry did not require stays admissible");
    assert_eq!(
        plan.unserved_evidence(),
        vec![UnservedTerm {
            request_term: 1,
            reason: UnservedReason::Unbound,
        }],
        "and the term the edit stranded is reported from the bindings in hand"
    );
}

// ---------------------------------------------------------------------------
// A binding that names a term the producer cannot receive
//
// Every edit above removes something. These three add: an index PUSHED onto a
// binding is the one shape of tampering that makes a plan claim *more* than the
// planner did, and it is invisible downstream — `place` iterates an
// alternative's placements, so an alternative that declares none gives it
// nothing to do and it succeeds. The plan then compiles to well-formed query
// text carrying no trace of the term while `unserved_evidence`, which reads
// "served" off binding membership, reports the term answered. The waist applies
// the planner's own rule so the two cannot part company over an editable value.
// ---------------------------------------------------------------------------

#[test]
fn a_binding_pushed_onto_a_term_the_producer_places_nowhere_is_refused() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let plan = fresh_plan(&registry, &stats);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };

    // The neighbouring valid case first, and it is the plan as planned: the
    // mandatory producer accepts every shape, places none, and is bound to
    // nothing, which is admitted exactly as it stands.
    let mandatory_binding = plan
        .producer_bindings
        .iter()
        .find(|binding| binding.producer == mandatory())
        .expect("the mandatory producer is selected");
    assert_eq!(
        mandatory_binding.request_terms,
        Vec::<u32>::new(),
        "it declares nowhere to put any of them, so it receives none of them"
    );
    compile(&plan, &env).expect("the plan as planned is admitted");
    assert_eq!(
        plan.unserved_evidence(),
        vec![UnservedTerm {
            request_term: 1,
            reason: UnservedReason::AcceptedWithoutPlacement,
        }],
        "and the vector term is reported as reaching nowhere to be put"
    );

    // The upward edit: the vector term IS matched by the producer's `Any`
    // pattern, so nothing downstream objects — the alternative places nothing,
    // `place` has nothing to render, and the emitted text is byte-identical to
    // the admitted plan's. Only the plan's own claim changed.
    let mut hollow = plan.clone();
    hollow
        .producer_bindings
        .iter_mut()
        .find(|binding| binding.producer == mandatory())
        .expect("the mandatory producer is selected")
        .request_terms
        .push(1);
    assert!(
        hollow.unserved_evidence().is_empty(),
        "which is the whole danger: the edited plan now reports every term served"
    );

    let error = compile(&hollow, &env).expect_err("a binding that transports nothing is refused");
    assert_eq!(error.dimension(), "hollow_binding");
    match error {
        AdmissionError::HollowBinding {
            producer,
            request_term,
        } => {
            assert_eq!(producer.as_str(), mandatory());
            assert_eq!(request_term, 1);
        }
        other => panic!("expected HollowBinding, got {other:?}"),
    }
}

#[test]
fn a_binding_pushed_onto_a_term_the_producer_does_receive_is_admitted() {
    // The over-refusal mirror. Pushing an index is not itself the defect: a
    // plan narrowed by hand and then widened back to what the producer can
    // receive is a legitimate value, and it must still admit. What is refused is
    // an index past the received set, not an index that was added.
    let registry = catch_all_registry(false);
    let stats = statistics("r1");
    let request = literal_seed_and_triple_request();
    let plan = purrdf_retrieval::plan(&request, &registry, &stats).expect("the request plans");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    assert_eq!(
        catch_all_binding(&plan)
            .expect("the catch-all is bound")
            .request_terms,
        vec![0, 1],
        "it places the literal and the IRI seed, and accepts the quoted triple not at all"
    );
    let admitted = compile(&plan, &env).expect("the plan as planned is admitted");

    // Narrowed, then widened back to the received set: admitted, and emitting
    // the same text as the plan it was derived from.
    let mut widened = plan.clone();
    let binding = widened
        .producer_bindings
        .iter_mut()
        .find(|binding| binding.producer == ex("pf/catch-all"))
        .expect("the catch-all is bound");
    binding.request_terms.truncate(1);
    binding.request_terms.push(1);
    let rewidened = compile(&widened, &env).expect("an index the producer receives is admitted");
    assert_eq!(rewidened.units, admitted.units);

    // One index further, onto the quoted triple, and the producer's declaration
    // names no alternative that takes it at all — so nothing of it is placed
    // either, and it is refused on the same dimension.
    let mut overreaching = plan;
    overreaching
        .producer_bindings
        .iter_mut()
        .find(|binding| binding.producer == ex("pf/catch-all"))
        .expect("the catch-all is bound")
        .request_terms
        .push(2);
    let error =
        compile(&overreaching, &env).expect_err("a term no alternative accepts is not received");
    assert_eq!(error.dimension(), "hollow_binding");
    match error {
        AdmissionError::HollowBinding {
            producer,
            request_term,
        } => {
            assert_eq!(producer.as_str(), ex("pf/catch-all"));
            assert_eq!(request_term, 2);
        }
        other => panic!("expected HollowBinding, got {other:?}"),
    }
}

#[test]
fn a_hollow_binding_is_refused_on_every_decode_path() {
    // The waist exists for plans that arrived from outside this process, so the
    // rule has to hold on the paths such a plan actually takes — not only on a
    // value edited in place.
    let registry = fixture_registry();
    let stats = statistics("r1");
    let plan = fresh_plan(&registry, &stats);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };

    let mut hollow = plan.clone();
    hollow
        .producer_bindings
        .iter_mut()
        .find(|binding| binding.producer == mandatory())
        .expect("the mandatory producer is selected")
        .request_terms
        .push(1);

    // serde, which is how a plan crosses a service boundary.
    let json = serde_json::to_string(&hollow).expect("a plan serializes");
    let decoded: Plan = serde_json::from_str(&json).expect("a plan deserializes");
    match compile(&decoded, &env).expect_err("a deserialized hollow binding is refused") {
        AdmissionError::HollowBinding {
            producer,
            request_term,
        } => {
            assert_eq!(producer.as_str(), mandatory());
            assert_eq!(request_term, 1);
        }
        other => panic!("expected HollowBinding from serde, got {other:?}"),
    }

    // And the canonical bytes, which is how it is cached and replayed.
    let bytes = hollow.canonical_bytes();
    let decoded = Plan::from_canonical_bytes(&bytes).expect("canonical bytes decode");
    match compile(&decoded, &env).expect_err("a decoded hollow binding is refused") {
        AdmissionError::HollowBinding {
            producer,
            request_term,
        } => {
            assert_eq!(producer.as_str(), mandatory());
            assert_eq!(request_term, 1);
        }
        other => panic!("expected HollowBinding from canonical bytes, got {other:?}"),
    }

    // The neighbouring valid case on both paths: the untampered plan survives
    // the round trip and admits, so the refusal is the edit's and not the
    // decoder's.
    let json = serde_json::to_string(&plan).expect("a plan serializes");
    let decoded: Plan = serde_json::from_str(&json).expect("a plan deserializes");
    compile(&decoded, &env).expect("an untampered deserialized plan is admitted");
    let decoded =
        Plan::from_canonical_bytes(&plan.canonical_bytes()).expect("canonical bytes decode");
    compile(&decoded, &env).expect("an untampered decoded plan is admitted");
}
