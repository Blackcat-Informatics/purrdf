// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The request reaches the evaluator: what `compile` writes into the query text.
//!
//! The headline claim is falsifiable in one line — two different requests must
//! compile to two different queries, and a lexical request's needle must be
//! *in* the text. Everything else here is the same claim read from a different
//! side: the per-stratum depth is a real `LIMIT`, a producer that cannot be
//! invoked is refused at planning rather than emitted as a widened call, and
//! every refusal is paired with the neighbouring request that must still
//! succeed.
//!
//! The mocks record exactly what the evaluator handed them, so "the constant
//! reached the relation" is asserted against the relation's own view rather than
//! against the text alone. Fixtures are `example.org` throughout.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

use pretty_assertions::assert_eq;
use purrdf_core::{RdfDatasetBuilder, SparqlRequest, SparqlResult, TermValue};
use purrdf_retrieval::{
    AdmissionEnvironment, AdmissionError, Iri, Metric, Plan, PlanError, ProducerStatus,
    RankedStreamImpl, RejectionReason, RequestTerm, RetrievalRequest, Statistics, Term, compile,
    execute, plan,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, DepthPlacement, DuplicatePolicy, EvalError, NativeSparqlEngine,
    PfArgs, PfArity, PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry, QueryOptions,
    RankedDeclaration, RequestFacet, TermKind, TermPattern, TermPlacement, Volatility,
};

mod common;

const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

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

/// One invocation, exactly as the evaluator presented it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Call {
    /// Every flattened position's value; `None` where the position was free.
    args: Vec<Option<TermValue>>,
    /// The row ceiling the evaluator offered, verbatim.
    ceiling: Option<u64>,
}

/// A ranked producer that records what it was handed and emits `count` rows.
///
/// A bound position is echoed back (the engine filters rows against bound
/// positions, so echoing is the behaviour that emits anything at all); a free
/// position gets a distinct IRI per row, so ranks and candidates stay
/// distinguishable.
struct Recorder {
    arity: PfArity,
    modes: Vec<BindingPattern>,
    rows: u64,
    count: usize,
    prefix: String,
    log: Arc<Mutex<Vec<Call>>>,
}

impl PropertyFunction for Recorder {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        self.arity
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        self.rows
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        let bound: Vec<Option<TermValue>> =
            args.flattened().map(Option::<&TermValue>::cloned).collect();
        self.log
            .lock()
            .expect("the fixture log is never poisoned")
            .push(Call {
                args: bound.clone(),
                ceiling,
            });
        let total = self.arity.total();
        let mut rows: Vec<Vec<TermValue>> = Vec::with_capacity(self.count);
        for index in 0..self.count {
            let mut row = Vec::with_capacity(total);
            for position in 0..total {
                row.push(
                    bound
                        .get(position)
                        .and_then(Clone::clone)
                        .unwrap_or_else(|| {
                            TermValue::iri(ex(&format!("{}{index}/pos{position}", self.prefix)))
                        }),
                );
            }
            rows.push(row);
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

/// How a fixture producer is declared, in one value.
struct Spec {
    subject: usize,
    object: usize,
    modes: Vec<&'static str>,
    rows: u64,
    count: usize,
    stratum: String,
    accepted: Vec<AcceptedTerm>,
    depth: Option<DepthPlacement>,
    candidate: usize,
    mandatory: bool,
}

impl Spec {
    /// The common shape: arity (1,1), all-free mode, candidate at position 0.
    fn new(stratum: &str, accepted: Vec<AcceptedTerm>) -> Self {
        Self {
            subject: 1,
            object: 1,
            modes: vec!["ff"],
            rows: 10,
            count: 3,
            stratum: ex(&format!("stratum/{stratum}")),
            accepted,
            depth: None,
            candidate: 0,
            mandatory: false,
        }
    }

    fn arity(mut self, subject: usize, object: usize, modes: Vec<&'static str>) -> Self {
        self.subject = subject;
        self.object = object;
        self.modes = modes;
        self
    }

    const fn rows(mut self, rows: u64, count: usize) -> Self {
        self.rows = rows;
        self.count = count;
        self
    }

    fn depth(mut self, depth: DepthPlacement) -> Self {
        self.depth = Some(depth);
        self
    }

    const fn candidate(mut self, candidate: usize) -> Self {
        self.candidate = candidate;
        self
    }
}

/// One accepted alternative: `pattern`, with `facet` bound at `position`.
fn alternative(pattern: TermPattern, placements: Vec<TermPlacement>) -> AcceptedTerm {
    AcceptedTerm {
        pattern,
        placements,
    }
}

fn value_at(position: usize) -> Vec<TermPlacement> {
    vec![TermPlacement {
        facet: RequestFacet::Value,
        position,
        datatype: None,
    }]
}

/// Register `specs` (name -> declaration) and hand back the registry plus each
/// producer's own invocation log.
fn registry_of(specs: Vec<(&str, Spec)>) -> (PropertyFunctionRegistry, BTreeMap<String, Log>) {
    let mut registry = PropertyFunctionRegistry::new();
    let mut logs = BTreeMap::new();
    for (name, spec) in specs {
        let log: Log = Arc::new(Mutex::new(Vec::new()));
        let arity = PfArity::new(spec.subject, spec.object);
        let relation = Arc::new(Recorder {
            arity,
            modes: spec
                .modes
                .iter()
                .map(|code| BindingPattern::from_code(code))
                .collect(),
            rows: spec.rows,
            count: spec.count,
            prefix: format!("{name}/row"),
            log: Arc::clone(&log),
        });
        registry.register_ranked(
            ex(&format!("pf/{name}")),
            relation,
            RankedDeclaration {
                stratum: kernel_iri(&spec.stratum),
                accepted_terms: spec.accepted,
                depth_placement: spec.depth,
                candidate_position: spec.candidate,
                duplicates: DuplicatePolicy::Unique,
                mandatory: spec.mandatory,
            },
        );
        logs.insert(ex(&format!("pf/{name}")), log);
    }
    (registry, logs)
}

type Log = Arc<Mutex<Vec<Call>>>;

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

/// Statistics that bound the named strata and nothing else.
fn statistics(bounds: &[(&str, u64)]) -> MockStatistics {
    MockStatistics {
        source: "compile-request-statistics".to_owned(),
        revision: "r1".to_owned(),
        cardinalities: bounds
            .iter()
            .map(|(stratum, cardinality)| (iri(&ex(&format!("stratum/{stratum}"))), *cardinality))
            .collect(),
    }
}

fn lexical(text: &str, language: Option<&str>) -> RequestTerm {
    RequestTerm::Lexical {
        text: text.to_owned(),
        language: language.map(ToOwned::to_owned),
        predicate: None,
    }
}

fn seed(text: &str) -> RequestTerm {
    RequestTerm::EntitySeed {
        entity: Term::new(text),
    }
}

fn request(terms: Vec<RequestTerm>) -> RetrievalRequest {
    RetrievalRequest::from_terms(terms)
}

/// Plan and compile `terms` against `registry`, returning each stratum's text
/// keyed by the stratum's local name.
fn compile_units(
    registry: &PropertyFunctionRegistry,
    stats: &MockStatistics,
    terms: Vec<RequestTerm>,
) -> BTreeMap<String, String> {
    let planned = plan(&request(terms), registry, stats).expect("the fixture request plans");
    let env = AdmissionEnvironment {
        registry,
        statistics: stats,
        fusion_profile: None,
    };
    compile(&planned, &env)
        .expect("the plan is admitted")
        .units
        .into_iter()
        .map(|unit| {
            let name = unit
                .stratum
                .as_str()
                .rsplit('/')
                .next()
                .expect("a stratum IRI has a last segment")
                .to_owned();
            (name, unit.sparql)
        })
        .collect()
}

/// Plan and compile `terms` against `registry`, returning the single unit's text.
fn compile_one(
    registry: &PropertyFunctionRegistry,
    stats: &MockStatistics,
    terms: Vec<RequestTerm>,
) -> String {
    let mut units = compile_units(registry, stats, terms);
    assert_eq!(units.len(), 1, "the fixture declares one stratum");
    units.pop_first().expect("the one unit is present").1
}

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

/// Run `sparql` through the evaluator with no composition type in the path.
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
        .unwrap_or_else(|error| {
            panic!("the emitted unit must parse and prepare: {error}\n{sparql}")
        });
    match outcome {
        SparqlResult::Solutions { rows, .. } => rows,
        other => panic!("expected a SELECT solution sequence, got {other:?}"),
    }
}

/// A single text producer: `TermKind::Literal`, needle rendered at position 1.
fn text_registry() -> (PropertyFunctionRegistry, BTreeMap<String, Log>) {
    registry_of(vec![(
        "text",
        Spec::new(
            "text",
            vec![alternative(
                TermPattern::of_kind(TermKind::Literal),
                value_at(1),
            )],
        ),
    )])
}

/// A single catch-all producer, value rendered at position 1.
fn any_registry() -> (PropertyFunctionRegistry, BTreeMap<String, Log>) {
    registry_of(vec![(
        "any",
        Spec::new(
            "any",
            vec![alternative(
                TermPattern::of_kind(TermKind::Any),
                value_at(1),
            )],
        ),
    )])
}

/// The rejection reason a plan recorded for `producer`, if it rejected it.
fn rejection(planned: &Plan, producer: &str) -> Option<RejectionReason> {
    planned
        .producer_decisions
        .iter()
        .find_map(|decision| match decision {
            purrdf_retrieval::ProducerDecision::Rejected {
                producer: name,
                reason,
            } if name == producer => Some(*reason),
            _ => None,
        })
}

// ---------------------------------------------------------------------------
// 1. The headline: the request is in the text
// ---------------------------------------------------------------------------

#[test]
fn two_requests_compile_to_different_queries_and_carry_the_needle() {
    let (registry, _) = text_registry();
    let stats = statistics(&[("text", 10)]);

    let fox = compile_one(&registry, &stats, vec![lexical("quick brown fox", None)]);
    let turtle = compile_one(&registry, &stats, vec![lexical("slow green turtle", None)]);

    assert_ne!(
        fox, turtle,
        "two different requests must not compile to one query"
    );
    assert!(
        fox.contains("\"quick brown fox\""),
        "the needle is in the emitted text: {fox}"
    );
    assert!(
        !fox.contains("slow green turtle"),
        "and only that needle is: {fox}"
    );
    // The plain-string spelling is pinned: the short form, never `^^<xsd:string>`.
    assert_eq!(
        fox,
        format!(
            "SELECT ?candidate WHERE {{\n  \
             {{ SELECT (?c0 AS ?candidate) WHERE {{ ( ?c0 ) <{}> ( \"quick brown fox\" ) }} \
             LIMIT 10 }}\n}}\nLIMIT 10",
            ex("pf/text")
        )
    );
}

#[test]
fn the_rendered_constant_reaches_the_relation() {
    let (registry, logs) = text_registry();
    let stats = statistics(&[("text", 10)]);
    let sparql = compile_one(
        &registry,
        &stats,
        vec![lexical("quick brown fox", Some("en"))],
    );
    run_query(&sparql, &registry);

    let calls = logs[&ex("pf/text")]
        .lock()
        .expect("the fixture log is never poisoned")
        .clone();
    assert_eq!(calls.len(), 1, "one invocation for one branch");
    assert_eq!(
        calls[0].args[1],
        Some(TermValue::Literal {
            lexical_form: "quick brown fox".to_owned(),
            datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
            language: Some("en".to_owned()),
            direction: None,
        }),
        "the needle arrives as the language-tagged literal the request named"
    );
    assert_eq!(calls[0].args[0], None, "the candidate position stays free");
}

// ---------------------------------------------------------------------------
// 2. The depth is a real bound
// ---------------------------------------------------------------------------

#[test]
fn depth_three_emits_limit_three_and_yields_three_rows() {
    let (registry, _) = registry_of(vec![(
        "text",
        Spec::new(
            "text",
            vec![alternative(
                TermPattern::of_kind(TermKind::Literal),
                value_at(1),
            )],
        )
        .rows(10, 10),
    )]);
    let stats = statistics(&[("text", 3)]);
    let planned = plan(
        &request(vec![lexical("quick brown fox", None)]),
        &registry,
        &stats,
    )
    .expect("plans");
    assert_eq!(planned.stratum_depths[&iri(&ex("stratum/text"))], 3);

    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&planned, &env).expect("admits");
    assert!(
        compiled.units[0].sparql.contains("LIMIT 3"),
        "the stratum's depth is the emitted bound: {}",
        compiled.units[0].sparql
    );

    let execution =
        block_on(execute(&compiled, &registry, &*common::empty_dataset())).expect("the unit runs");
    let rows = drain(
        execution
            .streams
            .into_iter()
            .next()
            .expect("the stratum streamed")
            .stream,
    );
    assert_eq!(rows.len(), 3, "the depth bounds the rows the stratum emits");
}

#[test]
fn depth_zero_is_an_honest_empty_stratum_and_depth_one_emits_one_row() {
    for (cardinality, expected) in [(0_u64, 0_usize), (1, 1)] {
        let (registry, _) = registry_of(vec![(
            "text",
            Spec::new(
                "text",
                vec![alternative(
                    TermPattern::of_kind(TermKind::Literal),
                    value_at(1),
                )],
            )
            .rows(10, 10),
        )]);
        let stats = statistics(&[("text", cardinality)]);
        let planned = plan(
            &request(vec![lexical("quick brown fox", None)]),
            &registry,
            &stats,
        )
        .expect("plans");
        let env = AdmissionEnvironment {
            registry: &registry,
            statistics: &stats,
            fusion_profile: None,
        };
        let compiled = compile(&planned, &env).expect("admits");
        assert!(
            compiled.units[0]
                .sparql
                .ends_with(&format!("LIMIT {cardinality}")),
            "the unit carries its own depth: {}",
            compiled.units[0].sparql
        );

        let execution = block_on(execute(&compiled, &registry, &*common::empty_dataset()))
            .expect("the unit runs");
        assert_eq!(
            execution.statuses[&iri(&ex("stratum/text"))],
            ProducerStatus::Exhausted {
                rows_emitted: cardinality
            },
            "an empty stratum is exhausted with zero rows, never failed"
        );
        let rows = drain(
            execution
                .streams
                .into_iter()
                .next()
                .expect("the stratum streamed")
                .stream,
        );
        assert_eq!(rows.len(), expected);
    }
}

// ---------------------------------------------------------------------------
// 3. Refusals, each with the neighbour that must still succeed
// ---------------------------------------------------------------------------

/// A kNN-shaped relation: it can only run with its depth bound.
fn knn_spec(depth: Option<DepthPlacement>) -> Spec {
    let spec = Spec::new(
        "knn",
        vec![alternative(
            TermPattern::of_kind(TermKind::Any),
            value_at(1),
        )],
    )
    .arity(1, 2, vec!["fbb"]);
    match depth {
        Some(depth) => spec.depth(depth),
        None => spec,
    }
}

#[test]
fn a_knn_producer_without_a_depth_placement_is_refused_and_with_one_is_admitted() {
    let stats = statistics(&[("knn", 4)]);
    let terms = vec![lexical("quick brown fox", None)];

    let (refusing, _) = registry_of(vec![("knn", knn_spec(None))]);
    let planned = plan(&request(terms.clone()), &refusing, &stats);
    match planned {
        Err(PlanError::NoApplicableProducers) => {}
        other => panic!("expected NoApplicableProducers, got {other:?}"),
    }

    let (admitting, _) = registry_of(vec![(
        "knn",
        knn_spec(Some(DepthPlacement {
            position: 2,
            datatype: XSD_INTEGER.to_owned(),
        })),
    )]);
    let sparql = compile_one(&admitting, &stats, terms);
    assert!(
        sparql.contains(&format!("\"4\"^^<{XSD_INTEGER}>")),
        "the depth rides as an argument: {sparql}"
    );
    assert!(
        !sparql.contains("}} LIMIT 4"),
        "a producer that took the depth as an argument gets no branch LIMIT: {sparql}"
    );
    assert!(
        sparql.ends_with("LIMIT 4"),
        "the unit still carries the stratum's contract with fusion: {sparql}"
    );
}

#[test]
fn an_untagged_needle_is_refused_where_a_language_position_is_declared() {
    let (registry, _) = registry_of(vec![(
        "text",
        Spec::new(
            "text",
            vec![alternative(
                TermPattern::of_kind(TermKind::Literal),
                vec![
                    TermPlacement {
                        facet: RequestFacet::Value,
                        position: 1,
                        datatype: None,
                    },
                    TermPlacement {
                        facet: RequestFacet::Language,
                        position: 2,
                        datatype: None,
                    },
                ],
            )],
        )
        .arity(1, 2, vec!["fff"]),
    )]);
    let stats = statistics(&[("text", 5)]);

    let refused = plan(
        &request(vec![lexical("quick brown fox", None)]),
        &registry,
        &stats,
    );
    assert!(
        matches!(refused, Err(PlanError::NoApplicableProducers)),
        "got {refused:?}"
    );

    let sparql = compile_one(
        &registry,
        &stats,
        vec![lexical("quick brown fox", Some("en"))],
    );
    assert!(
        sparql.contains("( ?c0 ) <") && sparql.contains("( \"quick brown fox\" \"en\" )"),
        "the tag rides in its own position and the needle is plain: {sparql}"
    );
}

/// The datatype the fixture producer declares for a query embedding.
///
/// Named by the *host*: PurRDF mints no vocabulary, so a producer that declares
/// none has its value placement refused rather than rendered under an invented
/// one — exactly as a geometry's datatype is required.
fn embedding_datatype() -> String {
    ex("embedding")
}

/// A producer that reads a query embedding: it accepts a literal (which is what
/// an embedding is written as) and names the datatype its own space reads one
/// under.
fn embedding_registry() -> (PropertyFunctionRegistry, BTreeMap<String, Log>) {
    registry_of(vec![(
        "embedding",
        Spec::new(
            "embedding",
            vec![alternative(
                TermPattern::of_kind(TermKind::Literal),
                vec![TermPlacement {
                    facet: RequestFacet::Value,
                    position: 1,
                    datatype: Some(embedding_datatype()),
                }],
            )],
        ),
    )])
}

fn vector(embedding: Vec<f32>) -> RequestTerm {
    RequestTerm::Vector {
        embedding,
        metric: Metric::Cosine,
        index_hint: None,
    }
}

#[test]
fn an_untyped_embedding_is_refused_and_a_typed_one_carries_the_vector() {
    let embedding = vec![0.25_f32, -1.5, 3.0];
    let stats = statistics(&[("any", 5), ("embedding", 5)]);

    // Refused: the catch-all declares a value placement with no datatype, and
    // this layer mints none, so there is nothing to write the embedding under.
    let (untyped, _) = any_registry();
    let refused = plan(&request(vec![vector(embedding.clone())]), &untyped, &stats);
    assert!(
        matches!(refused, Err(PlanError::NoApplicableProducers)),
        "a producer that declares no embedding datatype cannot receive one; got {refused:?}"
    );

    // Admitted, and the embedding is IN the emitted text — the whole unit,
    // verbatim, so nothing about the constant is asserted by substring alone.
    let (typed, logs) = embedding_registry();
    let sparql = compile_one(&typed, &stats, vec![vector(embedding.clone())]);
    let lexical = purrdf_retrieval::encode_embedding(&embedding);
    assert_eq!(
        lexical, "3E800000 BFC00000 40400000",
        "the components' exact bit patterns, in order"
    );
    let constant = format!("\"{lexical}\"^^<{}>", embedding_datatype());
    assert_eq!(
        sparql,
        format!(
            "SELECT ?candidate WHERE {{\n  \
             {{ SELECT (?c0 AS ?candidate) WHERE {{ ( ?c0 ) <{}> ( {constant} ) }} \
             LIMIT 5 }}\n}}\nLIMIT 5",
            ex("pf/embedding")
        )
    );

    // The producer that declared the datatype owns the parse, and it recovers
    // the vector the caller handed in — bit for bit, which is what makes two
    // plans with one identity compile to one query.
    run_query(&sparql, &typed);
    let calls = logs[&ex("pf/embedding")]
        .lock()
        .expect("the fixture log is never poisoned")
        .clone();
    assert_eq!(calls.len(), 1, "one invocation for one branch");
    let Some(TermValue::Literal {
        lexical_form,
        datatype,
        language: None,
        direction: None,
    }) = calls[0].args[1].clone()
    else {
        panic!(
            "the embedding arrives as a typed literal, got {:?}",
            calls[0]
        );
    };
    assert_eq!(datatype, embedding_datatype());
    let decoded = purrdf_retrieval::decode_embedding(&lexical_form)
        .expect("the producer reads back what the layer wrote");
    assert_eq!(
        decoded
            .iter()
            .copied()
            .map(f32::to_bits)
            .collect::<Vec<_>>(),
        embedding
            .iter()
            .copied()
            .map(f32::to_bits)
            .collect::<Vec<_>>(),
        "the rendered lexical recovers the exact input vector"
    );
}

#[test]
fn two_different_embeddings_compile_to_different_queries() {
    // The headline claim, read at the vector arm: the request is in the text, so
    // two requests that differ compile to two queries. The signed-zero pair is
    // the case a decimal lexical would have collapsed — two distinct bit
    // patterns, one decimal spelling — which is why the form is the bits.
    let (registry, _) = embedding_registry();
    let stats = statistics(&[("embedding", 5)]);
    let first = compile_one(&registry, &stats, vec![vector(vec![0.25, -1.5])]);
    let second = compile_one(&registry, &stats, vec![vector(vec![0.25, 1.5])]);
    assert_ne!(first, second);

    let positive = compile_one(&registry, &stats, vec![vector(vec![0.0])]);
    let negative = compile_one(&registry, &stats, vec![vector(vec![-0.0])]);
    assert_ne!(
        positive, negative,
        "two request terms with distinct identities must not compile to one query"
    );
}

#[test]
fn a_seed_is_admitted_as_the_rendered_constant() {
    let (registry, _) = any_registry();
    let stats = statistics(&[("any", 5)]);
    let sparql = compile_one(&registry, &stats, vec![seed(&format!("<{}>", ex("s")))]);
    assert!(
        sparql.contains(&format!("<{}>", ex("s"))),
        "the seed is the rendered constant: {sparql}"
    );
}

#[test]
fn a_blank_seed_is_refused_and_an_iri_seed_is_admitted() {
    let (registry, _) = any_registry();
    let stats = statistics(&[("any", 5)]);

    let planned = plan(&request(vec![seed("_:b0")]), &registry, &stats);
    assert!(
        matches!(planned, Err(PlanError::NoApplicableProducers)),
        "a blank node in an argument position is free, not ground; got {planned:?}"
    );

    assert!(
        compile_one(&registry, &stats, vec![seed(&format!("<{}>", ex("s")))])
            .contains(&format!("<{}>", ex("s")))
    );
}

#[test]
fn two_terms_conflicting_at_one_position_are_refused_and_agreeing_ones_are_not() {
    // A second producer that places nothing keeps the plan itself viable, so the
    // first producer's own rejection is readable rather than collapsed into
    // `NoApplicableProducers`. It stands in its own stratum, because a stratum
    // carries one producer.
    let (registry, _) = registry_of(vec![
        (
            "any",
            Spec::new(
                "conflict",
                vec![alternative(
                    TermPattern::of_kind(TermKind::Any),
                    value_at(1),
                )],
            ),
        ),
        (
            "free",
            Spec::new(
                "spare",
                vec![alternative(TermPattern::of_kind(TermKind::Any), Vec::new())],
            ),
        ),
    ]);
    let stats = statistics(&[("conflict", 5), ("spare", 5)]);

    let planned = plan(
        &request(vec![
            seed(&format!("<{}>", ex("a"))),
            seed(&format!("<{}>", ex("b"))),
        ]),
        &registry,
        &stats,
    )
    .expect("planning still succeeds; the producer is what is rejected");
    assert_eq!(
        rejection(&planned, &ex("pf/any")),
        Some(RejectionReason::UnsatisfiedConstraint),
        "the rejection is recorded with its own reason"
    );
    assert_eq!(
        planned
            .producer_bindings
            .iter()
            .map(|binding| binding.producer.clone())
            .collect::<Vec<_>>(),
        vec![ex("pf/free")],
        "only the producer that could be invoked is bound"
    );

    let units = compile_units(
        &registry,
        &stats,
        vec![
            seed(&format!("<{}>", ex("a"))),
            seed(&format!("<{}>", ex("a"))),
        ],
    );
    let sparql = &units["conflict"];
    assert_eq!(
        sparql.matches(&format!("<{}>", ex("a"))).count(),
        1,
        "the agreeing value is written once: {sparql}"
    );
}

#[test]
fn an_edited_plan_that_revives_a_refused_producer_is_refused_at_the_waist() {
    // The planner refuses the vector, so a plan naming the producer for it can
    // only have been hand-built. Admission re-derives the same decision.
    let (registry, _) = any_registry();
    let stats = statistics(&[("any", 5)]);
    let terms = vec![seed(&format!("<{}>", ex("s")))];
    let mut planned = plan(&request(terms), &registry, &stats).expect("plans");
    planned.request_terms = vec![RequestTerm::Vector {
        embedding: vec![1.0],
        metric: Metric::Dot,
        index_hint: None,
    }];

    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let error = compile(&planned, &env).expect_err("the edited plan is refused");
    assert_eq!(error.dimension(), "unsatisfiable_placement");
    match error {
        AdmissionError::UnsatisfiablePlacement {
            producer,
            rule,
            detail,
            ..
        } => {
            assert_eq!(producer.as_str(), ex("pf/any"));
            assert_eq!(rule, "unrenderable");
            assert!(detail.contains("embedding"), "{detail}");
        }
        other => panic!("expected UnsatisfiablePlacement, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 4. Determinism
// ---------------------------------------------------------------------------

#[test]
fn reordering_a_plans_bound_indices_does_not_change_the_emitted_text() {
    let (registry, _) = registry_of(vec![(
        "pair",
        Spec::new(
            "pair",
            vec![
                alternative(TermPattern::of_kind(TermKind::Iri), value_at(1)),
                alternative(TermPattern::of_kind(TermKind::Literal), value_at(2)),
            ],
        )
        .arity(1, 2, vec!["fff"]),
    )]);
    let stats = statistics(&[("pair", 6)]);
    let terms = vec![seed(&format!("<{}>", ex("a"))), seed("\"needle\"")];
    let planned = plan(&request(terms), &registry, &stats).expect("plans");
    assert_eq!(planned.producer_bindings[0].request_terms, vec![0, 1]);

    let mut reordered = planned.clone();
    reordered.producer_bindings[0].request_terms = vec![1, 0];

    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let forward = compile(&planned, &env).expect("admits");
    let backward = compile(&reordered, &env).expect("admits");
    assert_eq!(
        forward.units, backward.units,
        "the emitted text is a pure function of the plan's content, not its order"
    );
    assert!(
        forward.units[0]
            .sparql
            .contains("( ?c0 ) <http://example.org/pf/pair> ( <http://example.org/a> \"needle\" )"),
        "each alternative renders into its own declared position: {}",
        forward.units[0].sparql
    );
}

// ---------------------------------------------------------------------------
// 5. RDF 1.2: a quoted triple round-trips through the emitted text
// ---------------------------------------------------------------------------

#[test]
fn a_quoted_triple_seed_round_trips_through_the_emitted_query() {
    let (registry, logs) = any_registry();
    let stats = statistics(&[("any", 5)]);
    let quoted = format!("<<( <{}> <{}> \"o\" )>>", ex("quoted/s"), ex("quoted/p"));
    let sparql = compile_one(&registry, &stats, vec![seed(&quoted)]);
    assert!(
        sparql.contains(&quoted),
        "the triple term is written in its value form: {sparql}"
    );

    // Round trip: the evaluator parses the emitted text and hands the relation
    // the term back. Identical in, identical out.
    run_query(&sparql, &registry);
    let calls = logs[&ex("pf/any")]
        .lock()
        .expect("the fixture log is never poisoned")
        .clone();
    assert_eq!(
        calls[0].args[1],
        Some(TermValue::Triple {
            s: Box::new(TermValue::iri(ex("quoted/s"))),
            p: Box::new(TermValue::iri(ex("quoted/p"))),
            o: Box::new(TermValue::simple_literal("o")),
        }),
        "the quoted triple reads back identically"
    );
}

#[test]
fn a_directional_literal_seed_round_trips_through_the_emitted_query() {
    let (registry, logs) = any_registry();
    let stats = statistics(&[("any", 5)]);
    let sparql = compile_one(&registry, &stats, vec![seed("\"نص\"@ar--rtl")]);
    assert!(
        sparql.contains("\"نص\"@ar--rtl"),
        "the base direction is written: {sparql}"
    );

    run_query(&sparql, &registry);
    let calls = logs[&ex("pf/any")]
        .lock()
        .expect("the fixture log is never poisoned")
        .clone();
    assert_eq!(
        calls[0].args[1],
        Some(TermValue::Literal {
            lexical_form: "نص".to_owned(),
            datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString".to_owned(),
            language: Some("ar".to_owned()),
            direction: Some(purrdf_core::RdfTextDirection::Rtl),
        }),
        "the directional literal reads back identically"
    );
}

// ---------------------------------------------------------------------------
// 6. The emitted shape the grammar actually accepts
// ---------------------------------------------------------------------------

#[test]
fn producers_with_different_candidate_positions_read_back_under_one_name() {
    // A stratum carries one producer, so each of these gets its own — which is
    // what makes the shared projection name load-bearing rather than decorative:
    // two relations that declare their candidate in DIFFERENT argument positions
    // must still be read back by the executor under the single name `?candidate`.
    let (registry, _) = registry_of(vec![
        (
            "first",
            Spec::new(
                "alpha",
                vec![alternative(
                    TermPattern::of_kind(TermKind::Any),
                    value_at(1),
                )],
            )
            .rows(10, 3),
        ),
        (
            "second",
            Spec::new(
                "beta",
                vec![alternative(
                    TermPattern::of_kind(TermKind::Any),
                    value_at(2),
                )],
            )
            .arity(1, 2, vec!["fff"])
            .candidate(1)
            .rows(10, 3),
        ),
    ]);
    let stats = statistics(&[("alpha", 4), ("beta", 4)]);
    let units = compile_units(&registry, &stats, vec![lexical("needle", None)]);
    assert_eq!(units.len(), 2, "one unit per stratum, one producer each");

    assert!(
        units["alpha"].contains("(?c0 AS ?candidate)"),
        "the first stratum projects its own declared candidate position: {}",
        units["alpha"]
    );
    assert!(
        units["beta"].contains("(?c1 AS ?candidate)"),
        "and the second projects its own: {}",
        units["beta"]
    );
    for name in ["alpha", "beta"] {
        assert!(
            !units[name].contains("UNION"),
            "one producer is one branch, with nothing to compose it with: {}",
            units[name]
        );
        assert_eq!(
            units[name].matches("LIMIT 4").count(),
            2,
            "the branch bound plus the unit's own: {}",
            units[name]
        );
    }

    // Parsed, prepared and evaluated: the sub-`SELECT` alias alongside a
    // property-function call is accepted, and the parenthesized subject argument
    // list at arity one is routed to the call.
    let alpha = run_query(&units["alpha"], &registry);
    assert_eq!(alpha.len(), 3, "the producer's three rows, under its depth");
    assert_eq!(
        alpha[0][0].as_ref().expect("candidate bound"),
        &TermValue::iri(ex("first/row0/pos0")),
        "the rows arrive in the relation's own emission order"
    );
    let beta = run_query(&units["beta"], &registry);
    assert_eq!(beta.len(), 3);
    assert_eq!(
        beta[0][0].as_ref().expect("candidate bound"),
        &TermValue::iri(ex("second/row0/pos1")),
        "the second stratum reads its candidate out of position 1 under the same name"
    );
}

#[test]
fn the_branch_limit_reaches_the_relation_as_the_observed_ceiling() {
    // Recorded, not hoped for: correctness does not depend on the ceiling being
    // offered (the branch `LIMIT` bounds the rows either way), only an
    // optimization does. This pins what the evaluator actually does today, so a
    // change to it is visible rather than silent.
    let (registry, logs) = text_registry();
    let stats = statistics(&[("text", 2)]);
    let sparql = compile_one(&registry, &stats, vec![lexical("needle", None)]);
    run_query(&sparql, &registry);
    let calls = logs[&ex("pf/text")]
        .lock()
        .expect("the fixture log is never poisoned")
        .clone();
    assert_eq!(
        calls[0].ceiling,
        Some(2),
        "the branch LIMIT is observed to reach the relation as its row ceiling"
    );
}

// The evaluator's own cap-pushdown claim — that each arm of a `UNION` is offered
// its own `LIMIT` as a row ceiling — used to be pinned from here, by a stratum
// with two producers. The registry now refuses that configuration, and the claim
// never depended on retrieval reaching it: it is a fact about the evaluator's
// descent past a `UNION`, and it is proved in `purrdf-sparql-eval` against
// hand-written two-arm queries, including the constant-bearing arm this compiler
// emits.
