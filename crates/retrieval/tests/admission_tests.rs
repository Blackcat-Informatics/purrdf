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
    AdmissionEnvironment, AdmissionError, CompiledRetrieval, Fixed, Iri, Metric, Plan,
    ProducerDecision, ProducerStatus, RejectionReason, RequestTerm, RetrievalRequest, Statistics,
    Term, Weight, compile, execute,
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
    };
    let error = compile(&plan, &env).expect_err("same fingerprint, different implementation");
    assert!(
        matches!(error, AdmissionError::RegistryMismatch { .. }),
        "got {error:?}"
    );
}

// ---------------------------------------------------------------------------
// 8. Execution: failure isolation and replay
// ---------------------------------------------------------------------------

fn rows_by_stratum(result: purrdf_retrieval::ExecutionResult) -> BTreeMap<Iri, Vec<(u64, Term)>> {
    result
        .streams
        .into_iter()
        .map(|stream| (stream.stratum, stream.stream.into_rows()))
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
    };
    compile(&plan, &env).expect("an optional producer may cover part of a request");
}
