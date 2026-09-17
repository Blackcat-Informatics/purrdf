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
    AdmissionEnvironment, AdmissionError, CompiledRetrieval, DecayRule, Fixed, FusionProfile, Iri,
    Metric, Plan, PlanOrigin, ProducerDecision, ProducerStatus, RankedStreamImpl, RejectionReason,
    RequestTerm, RetrievalRequest, SCALE_DIGITS, Statistics, Term, UnservedReason, UnservedTerm,
    Weight, compile, contribution, execute,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, DuplicatePolicy, EvalError, PfArgs, PfArity, PfCursor, PfRow,
    PropertyFunction, PropertyFunctionRegistry, RankOrdering, RankedDeclaration, RequestFacet,
    TermKind, TermPattern, TermPlacement, Volatility,
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

/// A registry whose declarations differ from the fixture's: the catch-all
/// producer declares a different row bound, so the content fingerprint moves.
fn registry_with_different_declaration() -> PropertyFunctionRegistry {
    let mut registry = fixture_registry();
    registry.register_ranked(
        ex("pf/extra"),
        producer(999, "extra/", 0),
        ranked(
            &ex("stratum/universal"),
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
    let registry = fixture_registry();
    let stats = statistics("r1");
    let mut plan = fresh_plan(&registry, &stats);
    let binding = plan
        .producer_bindings
        .iter_mut()
        .find(|binding| binding.producer == mandatory())
        .expect("the mandatory producer is bound");
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
            assert_eq!(producer.as_str(), mandatory());
            assert_eq!(required, 3);
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
// 4b. Depth against the fusion profile's own arithmetic
//
// The registry's row bound answers "can this many rows be produced". It says
// nothing about whether the profile can still tell them apart once they are, and
// the failure to tell them apart is silent: no error, no nondeterminism, just
// rows that stop being ordered by the ranks their producers assigned. The three
// tests below pin the refusal, the exact bound it is drawn at, and — because a
// bound off by one in the strict direction would refuse legitimate profiles —
// that a depth sitting exactly on the bound is admitted and still orders.
// ---------------------------------------------------------------------------

/// The smoothing constant the monotone-range fixtures use. One, so the bound is
/// the first colliding denominator less one and the arithmetic is legible.
const MONOTONE_K: u32 = 1;

/// A profile weighting `stratum/universal` alone, at a weight small enough that
/// the reciprocal's own resolution is exhausted well inside the registry's
/// declared row bound of two hundred.
///
/// A weight of `10^-9` puts the first colliding rank in the tens. That is a
/// perfectly legitimate weight — `FusionProfile::new` asks only that a weight be
/// strictly positive — which is exactly the point: nothing about the profile
/// itself is malformed, and only the coupling with the depth is.
fn shallow_profile() -> FusionProfile {
    FusionProfile::new(
        BTreeMap::from([(iri(&ex("stratum/universal")), Fixed::from_raw(1_000))]),
        MONOTONE_K,
        1,
    )
    .expect("a strictly positive weight is a valid profile")
}

#[test]
fn admission_rejects_a_depth_beyond_the_profiles_monotone_range() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let profile = shallow_profile();
    let stratum = iri(&ex("stratum/universal"));
    let monotone = profile
        .monotone_depth(&stratum)
        .expect("the profile weights this stratum");
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
    let error = compile(&plan, &env).expect_err("a depth past the monotone range is refused");
    match error {
        AdmissionError::DepthBeyondMonotoneRange {
            stratum: named,
            monotone: reported,
            requested: asked,
        } => {
            assert_eq!(*named, stratum, "the refusal names the stratum");
            assert_eq!(reported, monotone, "and the bound it was drawn at");
            assert_eq!(asked, requested);
        }
        other => panic!("expected DepthBeyondMonotoneRange, got {other:?}"),
    }
    assert_eq!(
        compile(&plan, &env).expect_err("still refused").dimension(),
        "depth_beyond_monotone_range"
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
        .expect("the profile weights this stratum");

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
    compile(&plan, &env).expect("a depth exactly at the bound is admitted");

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
                .expect("weighted"),
        )
        .expect("the fixture bound is small"),
    );
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: Some(&profile),
    };
    compile(&admitted, &env).expect("an unweighted stratum's depth is not this profile's business");
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
const DEEP_WEIGHT_UNITS: i128 = 200;

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
            Fixed::from_raw(DEEP_WEIGHT_UNITS * 10_i128.pow(SCALE_DIGITS)),
        )]),
        DecayRule::WeightedReciprocalRank { k: 1 },
        1,
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
        .expect("the profile weights this stratum");
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

    // The same plan under the same weight on the *first* rule is refused, which
    // is what makes the rule the load-bearing choice rather than the weight.
    let truncated = FusionProfile::with_decay(
        BTreeMap::from([(
            stratum,
            Fixed::from_raw(DEEP_WEIGHT_UNITS * 10_i128.pow(SCALE_DIGITS)),
        )]),
        DecayRule::ReciprocalRank { k: 1 },
        1,
    )
    .expect("valid");
    let shallow_env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: Some(&truncated),
    };
    assert_eq!(
        compile(&plan, &shallow_env)
            .expect_err("the truncated rule cannot order fourteen million ranks")
            .dimension(),
        "depth_beyond_monotone_range"
    );
}

#[test]
fn a_depth_past_the_weighted_profiles_own_range_is_still_refused() {
    // The neighbouring refusal. Buying depth with weight must not become "any
    // depth at all": one rank past this profile's exact range is refused, by
    // name, reporting the stratum and the bound it was drawn at.
    let registry = deep_registry();
    let stats = statistics("r1");
    let profile = deep_profile();
    let stratum = iri(&ex("stratum/universal"));
    let monotone = profile
        .monotone_depth(&stratum)
        .expect("the profile weights this stratum");
    let requested = u32::try_from(monotone + 1).expect("the bound is inside a u32");
    assert!(
        u64::from(requested) <= 20_000_000,
        "the refusal must come from the profile, not from the registry's row bound"
    );

    let mut plan = fresh_plan(&registry, &stats);
    plan.stratum_depths.insert(stratum.clone(), requested);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: Some(&profile),
    };
    match compile(&plan, &env).expect_err("a depth past the range is refused") {
        AdmissionError::DepthBeyondMonotoneRange {
            stratum: named,
            monotone: reported,
            requested: asked,
        } => {
            assert_eq!(*named, stratum, "the refusal names the stratum");
            assert_eq!(reported, monotone, "and the bound it was drawn at");
            assert_eq!(asked, requested);
        }
        other => panic!("expected DepthBeyondMonotoneRange, got {other:?}"),
    }

    // And the neighbour that must still pass: the depth exactly on the bound.
    let mut admitted = plan;
    admitted.stratum_depths.insert(
        stratum,
        u32::try_from(monotone).expect("the bound is inside a u32"),
    );
    compile(&admitted, &env).expect("a depth exactly at the bound is admitted");
}

#[test]
fn an_environment_that_names_no_profile_checks_no_monotone_range() {
    // The dimension is checked against a law, and an environment that names no
    // law has none to check against. A caller that plans and compiles before
    // choosing how to fuse is not refused for not having chosen.
    let registry = fixture_registry();
    let stats = statistics("r1");
    let profile = shallow_profile();
    let stratum = iri(&ex("stratum/universal"));
    let monotone = profile
        .monotone_depth(&stratum)
        .expect("the profile weights this stratum");
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
    compile(&plan, &env).expect("no profile named, so no profile arithmetic to violate");
}

// ---------------------------------------------------------------------------
// 5. Weights
// ---------------------------------------------------------------------------

#[test]
fn admission_rejects_undeclared_stratum_weight() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let mut plan = fresh_plan(&registry, &stats);
    plan.stratum_weights
        .insert(iri(&ex("stratum/ghost")), Weight::new(Fixed::ONE));
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let error = compile(&plan, &env).expect_err("a weight for an undeclared stratum is refused");
    match error {
        AdmissionError::UndeclaredStratumWeight { stratum } => {
            assert_eq!(*stratum, iri(&ex("stratum/ghost")));
        }
        other => panic!("expected UndeclaredStratumWeight, got {other:?}"),
    }
}

#[test]
fn admission_rejects_invalid_weight() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let mut plan = fresh_plan(&registry, &stats);
    plan.stratum_weights
        .insert(iri(&ex("stratum/text")), Weight::from_raw(0));
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let error = compile(&plan, &env).expect_err("a non-positive weight is refused");
    match error {
        AdmissionError::InvalidWeight {
            stratum, weight, ..
        } => {
            assert_eq!(*stratum, iri(&ex("stratum/text")));
            assert_eq!(weight, Fixed::ZERO);
        }
        other => panic!("expected InvalidWeight, got {other:?}"),
    }
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
        ordering: RankOrdering::StrictlyDescending,
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

/// One accepted alternative that accepts every shape and renders nothing: the
/// permissive catch-all. Its argument stays a free variable, which is what "I
/// take the whole request without needing it written out" is.
fn renders_nothing() -> Vec<AcceptedTerm> {
    vec![AcceptedTerm {
        pattern: TermPattern::of_kind(TermKind::Any),
        placements: Vec::new(),
    }]
}

/// `pf/catch-all` — permissive, always placeable — beside a literal-only
/// producer in its own stratum, so dropping the catch-all still leaves a unit
/// to emit.
fn catch_all_registry(mandatory: bool) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/catch-all"),
        producer(200, "catch/", 3),
        declaration(&ex("stratum/catch"), renders_nothing(), mandatory),
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
/// SPARQL constant form — makes it unplaceable. `pf/free` shares its stratum and
/// places nothing, so the plan stays viable and the first producer's own fate is
/// readable rather than collapsed into `NoApplicableProducers`.
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
        declaration(&ex("stratum/render"), renders_nothing(), false),
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
    let mut plan =
        purrdf_retrieval::plan(&mixed_request(), registry, &stats).expect("the request plans");
    let binding = plan
        .producer_bindings
        .iter_mut()
        .find(|binding| binding.producer == ex("pf/catch-all"))
        .expect("the catch-all is bound to the whole request");
    assert_eq!(
        binding.request_terms.len(),
        3,
        "the planner binds a permissive producer to every term it accepts"
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
            assert_eq!(required, 3);
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
fn a_mandatory_producer_is_refused_for_a_request_shape_it_does_not_accept() {
    let stats = statistics("r1");

    // A host may declare a narrow producer mandatory. That is a claim about
    // every request, so a request carrying shapes the producer does not accept
    // falsifies it — and the falsification is a named refusal at the waist, not
    // an answer that quietly covers one term of three.
    let registry = literal_only_registry(true);
    let plan = purrdf_retrieval::plan(&mixed_request(), &registry, &stats)
        .expect("the lexical term still reaches the producer, so the request plans");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let error =
        compile(&plan, &env).expect_err("a mandatory producer must serve the whole request");
    match error {
        AdmissionError::InsufficientBindings {
            producer,
            required,
            provided,
        } => {
            assert_eq!(producer.as_str(), ex("pf/literal"));
            assert_eq!(required, 3, "the request carries three terms");
            assert_eq!(provided, 1, "only the lexical one matches a declared shape");
        }
        other => panic!("expected InsufficientBindings, got {other:?}"),
    }

    // The neighbouring valid case: a request the producer does accept in full is
    // admitted by the same mandatory registry.
    let lexical_only = RetrievalRequest::from_terms(vec![lexical_term()]);
    let plan =
        purrdf_retrieval::plan(&lexical_only, &registry, &stats).expect("a lexical request plans");
    compile(&plan, &env).expect("a request the mandatory producer serves in full is admitted");

    // And the same three-term request is admitted where the host did not make
    // that claim, so the refusal above is the declaration's, not the shape's.
    let optional = literal_only_registry(false);
    let plan = purrdf_retrieval::plan(&mixed_request(), &optional, &stats).expect("plans");
    let env = AdmissionEnvironment {
        registry: &optional,
        statistics: &stats,
        fusion_profile: None,
    };
    compile(&plan, &env).expect("an optional producer may cover part of a request");
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
        plan.unserved_terms.is_empty(),
        "the fixture request reaches a producer for every term"
    );
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
    let mut plan = purrdf_retrieval::plan(&mixed_request(), &optional, &stats).expect("plans");
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

    let mut plan = purrdf_retrieval::plan(&mixed_request(), &registry, &stats).expect("plans");
    assert!(
        plan.unserved_terms.is_empty(),
        "the catch-all reaches every term, so the planner recorded nothing"
    );
    for binding in &mut plan.producer_bindings {
        binding.request_terms.retain(|index| *index == 0);
    }

    compile(&plan, &env)
        .expect("narrowing a producer the registry did not require stays admissible");
    assert_eq!(
        plan.unserved_evidence(),
        vec![
            UnservedTerm {
                request_term: 1,
                reason: UnservedReason::Unbound,
            },
            UnservedTerm {
                request_term: 2,
                reason: UnservedReason::Unbound,
            },
        ],
        "and the terms the edit stranded are reported from the bindings in hand"
    );
}
