// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
    AdmissionEnvironment, AdmissionError, CompiledRetrieval, ExecutionError, Iri, Metric, Plan,
    PlanError, ProducerStatus, RankFidelity, RankedStreamImpl, ReadBound, RejectionReason,
    RequestTerm, RetrievalRequest, Statistics, StratumUnit, Term, TopK, UnservedReason,
    UnservedTerm, compile, execute, plan,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, CandidateDomains, DepthPlacement, DomainTag, DuplicatePolicy,
    EvalError, ExtensionEnv, NativeSparqlEngine, PfArgs, PfArity, PfCursor, PfRow,
    PropertyFunction, PropertyFunctionRegistry, QueryOptions, RankedDeclaration, RequestFacet,
    TermKind, TermPattern, TermPlacement, Volatility,
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
    /// The argument position this relation reads as the most rows it may return,
    /// for a fixture playing a producer that **obeys** the depth it is handed.
    ///
    /// `None` is the fixture default, and it is the right default for every claim
    /// about what the text says: a relation that ignores the argument still records
    /// it, which is what those tests read. It is `Some` only where a claim is about
    /// the argument's *effect* — a lowered depth argument cannot be observed at all
    /// through a relation that never looked at it, and a conforming self-bounding
    /// relation does look.
    obeys_depth: Option<usize>,
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
        // A relation that obeys its depth argument returns at most that many rows,
        // which is what a self-bounding producer's declaration promises. The value is
        // read off the argument the text rendered, so lowering that number in the text
        // really does shorten the read.
        let count = match self
            .obeys_depth
            .and_then(|position| bound.get(position).cloned())
        {
            Some(Some(TermValue::Literal { lexical_form, .. })) => lexical_form
                .parse::<usize>()
                .map_or(self.count, |allowed| self.count.min(allowed)),
            Some(_) | None => self.count,
        };
        let mut rows: Vec<Vec<TermValue>> = Vec::with_capacity(count);
        for index in 0..count {
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
    /// Which blocks of the candidate universe the producer may name.
    ///
    /// [`CandidateDomains::Unrestricted`] is the fixture default because it is
    /// the widest promise and the one every producer effectively made before the
    /// term existed, so a spec that says nothing about domains keeps exactly the
    /// reading it had.
    domains: CandidateDomains,
    /// The argument position the relation reads as its own row ceiling, for a
    /// fixture that obeys the depth it is handed rather than merely recording it.
    obeys_depth: Option<usize>,
    /// Whether the producer promises each candidate appears at most once in its
    /// own stream.
    ///
    /// [`DuplicatePolicy::Unique`] is the fixture default because it is the
    /// promise every fixture here keeps. `Allowed` is the honest declaration of a
    /// producer whose rows are not its candidates, and it is one of the two shapes
    /// that cannot narrow a depth to a request's bound.
    duplicates: DuplicatePolicy,
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
            obeys_depth: None,
            candidate: 0,
            mandatory: false,
            domains: CandidateDomains::Unrestricted,
            duplicates: DuplicatePolicy::Unique,
        }
    }

    /// Declare that the producer's stream may name one candidate more than once.
    const fn repeats(mut self) -> Self {
        self.duplicates = DuplicatePolicy::Allowed;
        self
    }

    /// Restrict the producer to the single block `tag`.
    ///
    /// One tag rather than a set, because a single-block declaration entails the
    /// block of every row and so needs no per-row block column; a several-block
    /// declaration would oblige this mock to project one.
    fn within(mut self, tag: &str) -> Self {
        self.domains = CandidateDomains::within([
            DomainTag::parse(&ex(tag)).expect("fixture domain tags are valid IRIs")
        ]);
        self
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

    /// Declare the depth placement `depth` **and** obey the number rendered into it.
    ///
    /// The pair travels together because neither half means anything alone: a
    /// relation that obeys a position nothing renders a depth into reads whatever the
    /// request placed there, and a depth placement no relation obeys cannot show what
    /// the argument does to a read.
    fn obeying(mut self, depth: DepthPlacement) -> Self {
        self.obeys_depth = Some(depth.position);
        self.depth = Some(depth);
        self
    }

    const fn candidate(mut self, candidate: usize) -> Self {
        self.candidate = candidate;
        self
    }

    /// Declare the producer mandatory, which is the registry promising that it
    /// serves every request it accepts anything of.
    const fn mandatory(mut self) -> Self {
        self.mandatory = true;
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
            obeys_depth: spec.obeys_depth,
        });
        registry.register_ranked(
            ex(&format!("pf/{name}")),
            relation,
            RankedDeclaration {
                stratum: kernel_iri(&spec.stratum),
                accepted_terms: spec.accepted,
                depth_placement: spec.depth,
                candidate_position: spec.candidate,
                duplicates: spec.duplicates,
                fidelity: RankFidelity::EXACT,
                arithmetic: None,
                domains: spec.domains,
                block_position: None,
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
    /// Reported for every `(subject, term)` pair the planner asks about, or
    /// `None` for a provider that measured no selectivity at all.
    ///
    /// Blanket rather than per-pair because the claims here are about what the
    /// planner does with a reported ratio, not about which pair it was reported
    /// under; a provider that answers every question with one number is the
    /// smallest fixture that exercises the arithmetic.
    selectivity: Option<u64>,
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
        self.selectivity
    }
}

/// Statistics that bound the named strata and nothing else.
fn statistics(bounds: &[(&str, u64)]) -> MockStatistics {
    statistics_reporting(bounds, None)
}

/// Statistics that bound the named strata and report `selectivity` parts per
/// million for every term they are asked about.
fn statistics_reporting(bounds: &[(&str, u64)], selectivity: Option<u64>) -> MockStatistics {
    MockStatistics {
        source: "compile-request-statistics".to_owned(),
        revision: "r1".to_owned(),
        cardinalities: bounds
            .iter()
            .map(|(stratum, cardinality)| (iri(&ex(&format!("stratum/{stratum}"))), *cardinality))
            .collect(),
        selectivity,
    }
}

fn lexical(text: &str, language: Option<&str>) -> RequestTerm {
    RequestTerm::Lexical {
        text: text.to_owned(),
        language: language.map(ToOwned::to_owned),
        predicate: None,
    }
}

/// A needle carrying the predicate it is to be matched under.
///
/// A predicate is how one producer's declaration is narrowed to a term another
/// producer's declaration cannot take, which is what a claim about a term only
/// one producer accepts needs.
fn lexical_under(text: &str, predicate: &str) -> RequestTerm {
    RequestTerm::Lexical {
        text: text.to_owned(),
        language: None,
        predicate: Some(iri(&ex(predicate))),
    }
}

fn seed(text: &str) -> RequestTerm {
    RequestTerm::EntitySeed {
        entity: Term::new(text),
    }
}

fn request(terms: Vec<RequestTerm>) -> RetrievalRequest {
    RetrievalRequest::complete(terms)
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
            (name, unit.sparql())
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
    // The block each row names is not what these assertions are about — every
    // producer here declares `Unrestricted` and so names none — so it is dropped
    // by name rather than compared.
    while let Some((rank, candidate, _block)) =
        block_on(stream.next()).expect("a materialized stream obeys the protocol")
    {
        rows.push((rank, candidate));
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
                env: &ExtensionEnv::over_relations(registry.clone())
                    .expect("the fixture declarations read cleanly"),
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
    //
    // The bound is eleven over a depth of ten: this fixture's producer declares
    // ten rows and the statistics measure ten, so the depth sits exactly at the
    // declaration and the probe row is emitted one past it, as it is at every
    // other depth. The recorded depth is still ten — that the emitted bound moves
    // and the depth does not is pinned in `admission_accepts_fresh_plan`, which
    // holds the same pair of numbers and asserts both.
    assert_eq!(
        fox,
        format!(
            "SELECT ?candidate WHERE {{\n  \
             {{ SELECT (?c0 AS ?candidate) WHERE {{ ( ?c0 ) <{}> ( \"quick brown fox\" ) }} \
             LIMIT 11 }}\n}}\nLIMIT 11",
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
    // The stratum reads three rows; the emitted bound is four. The producer
    // declares ten rows per invocation, so the fourth row is the probe that
    // tells a read the depth cut from a read that ran out — and it is a read,
    // never a value: the unit still records a depth of three and exactly three
    // rows reach the stream.
    assert!(
        compiled.units[0].sparql().contains("LIMIT 4"),
        "the stratum's depth plus its probe row is the emitted bound: {}",
        compiled.units[0].sparql()
    );
    assert_eq!(
        compiled.units[0].depth(),
        3,
        "the recorded depth is the plan's, not the emitted bound"
    );

    let execution =
        block_on(execute(&compiled, &registry, &*common::empty_dataset())).expect("the unit runs");
    assert_eq!(
        execution.statuses[&iri(&ex("stratum/text"))],
        ProducerStatus::DepthReached { rank: 3 },
        "the producer held ten rows and the plan read three of them, so the depth \
         is what stopped the read"
    );
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

/// The governing invariant, executed end to end: a bound on the read never
/// becomes a value.
///
/// A provider that measured an empty stratum is reporting honestly, and the
/// plan still asks. The old behaviour scaled the declared bound to zero,
/// compiled `LIMIT 0`, took no row from the relation whatever its index held, and
/// reported the stratum exhausted having emitted nothing — the one ending that
/// names no stopper, minted from an estimate. Both halves are
/// asserted here: the depth is one, and the emptiness that comes back is the
/// *producer's*, because the producer was allowed to answer.
#[test]
fn a_zero_statistic_still_plans_one_row_and_the_producer_reports_the_emptiness() {
    for cardinality in [0_u64, 1] {
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
        assert_eq!(
            planned.stratum_depths[&iri(&ex("stratum/text"))],
            1,
            "a measured cardinality of {cardinality} narrows the read to one row, never to none"
        );
        let env = AdmissionEnvironment {
            registry: &registry,
            statistics: &stats,
            fusion_profile: None,
        };
        let compiled = compile(&planned, &env).expect("admits");
        // One row read, plus the probe row the producer's ten-row declaration
        // leaves room for. The floor is on the *depth*, which is what the unit
        // records; the emitted bound is one deeper so the executor can tell an
        // emptied stratum from one the floored depth cut.
        assert!(
            compiled.units[0].sparql().ends_with("LIMIT 2"),
            "the unit carries the floored depth, probed one deeper: {}",
            compiled.units[0].sparql()
        );
        assert_eq!(
            compiled.units[0].depth(),
            1,
            "the floored depth is one, and the probe row does not raise it"
        );
    }

    // The producer that has nothing: it declares ten rows and holds none. The
    // emptiness in the trailer is therefore its own report, not a bound.
    let (registry, logs) = registry_of(vec![(
        "text",
        Spec::new(
            "text",
            vec![alternative(
                TermPattern::of_kind(TermKind::Literal),
                value_at(1),
            )],
        )
        .rows(10, 0),
    )]);
    let stats = statistics(&[("text", 0)]);
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
    let execution =
        block_on(execute(&compiled, &registry, &*common::empty_dataset())).expect("the unit runs");
    assert_eq!(
        execution.statuses[&iri(&ex("stratum/text"))],
        ProducerStatus::Exhausted { rows_emitted: 0 },
        "the producer was asked and had nothing, which is what exhausted means"
    );
    assert_eq!(
        invocations(&logs, "text"),
        1,
        "the relation was opened: the emptiness is the producer's report, not a LIMIT 0"
    );
}

/// How many times a fixture producer's relation was actually opened.
///
/// The claim "the producer reported the emptiness" is only true if the producer
/// ran, and an emitted `LIMIT 0` produces the identical trailer without ever
/// reaching the relation. The log the `Recorder` keeps is the only witness that
/// distinguishes them.
fn invocations(logs: &BTreeMap<String, Log>, name: &str) -> usize {
    logs.get(&ex(&format!("pf/{name}")))
        .expect("the fixture producer is registered")
        .lock()
        .expect("the fixture log is never poisoned")
        .len()
}

/// A selectivity of zero is the provider saying no row under this stratum
/// matches. It narrows the read as far as a statistic may narrow anything — to
/// one row — and the producer then answers.
#[test]
fn a_zero_selectivity_narrows_to_one_row_and_the_relation_is_still_invoked() {
    // A producer holding rows: the floored depth is a real read, and exactly one
    // row comes back.
    let (holding, holding_logs) = registry_of(vec![(
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
    let stats = statistics_reporting(&[("text", 10)], Some(0));
    let terms = vec![lexical("quick brown fox", None)];
    let planned = plan(&request(terms.clone()), &holding, &stats).expect("plans");
    assert_eq!(
        planned.stratum_depths[&iri(&ex("stratum/text"))],
        1,
        "zero parts per million narrows the ten-row bound to one row, never to none"
    );
    let env = AdmissionEnvironment {
        registry: &holding,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&planned, &env).expect("admits");
    assert!(
        compiled.units[0].sparql().ends_with("LIMIT 2"),
        "the floored depth, plus its probe row, is the emitted bound: {}",
        compiled.units[0].sparql()
    );
    let execution =
        block_on(execute(&compiled, &holding, &*common::empty_dataset())).expect("the unit runs");
    // The read the provider would have eliminated returns a row — and says the
    // right thing about it. This producer holds ten rows and the floored depth
    // read one, so the stratum is emphatically NOT exhausted; reporting it that
    // way would be the same fault the floor exists to prevent, one rung higher:
    // an estimate that narrowed a read being published as a fact about the
    // answer.
    assert_eq!(
        execution.statuses[&iri(&ex("stratum/text"))],
        ProducerStatus::DepthReached { rank: 1 },
        "the read the provider would have eliminated returns a row, and names \
         the depth that stopped it"
    );
    assert_eq!(invocations(&holding_logs, "text"), 1);
    let rows = drain(
        execution
            .streams
            .into_iter()
            .next()
            .expect("the stratum streamed")
            .stream,
    );
    assert_eq!(rows.len(), 1, "one row on the stream, not none");

    // The same plan over a producer that holds nothing: the trailer says
    // exhausted-with-nothing, and the relation was opened to find that out.
    let (empty, empty_logs) = registry_of(vec![(
        "text",
        Spec::new(
            "text",
            vec![alternative(
                TermPattern::of_kind(TermKind::Literal),
                value_at(1),
            )],
        )
        .rows(10, 0),
    )]);
    let planned = plan(&request(terms), &empty, &stats).expect("plans");
    assert_eq!(planned.stratum_depths[&iri(&ex("stratum/text"))], 1);
    let env = AdmissionEnvironment {
        registry: &empty,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&planned, &env).expect("admits");
    let execution =
        block_on(execute(&compiled, &empty, &*common::empty_dataset())).expect("the unit runs");
    assert_eq!(
        execution.statuses[&iri(&ex("stratum/text"))],
        ProducerStatus::Exhausted { rows_emitted: 0 }
    );
    assert_eq!(
        invocations(&empty_logs, "text"),
        1,
        "the producer was asked and had nothing; a LIMIT 0 would have faked this trailer"
    );
}

/// Both roads to a bound of zero are floored, including the one that reaches it
/// before the ratio is applied.
#[test]
fn a_cardinality_of_zero_is_floored_with_or_without_a_selectivity() {
    for selectivity in [None, Some(0)] {
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
        let stats = statistics_reporting(&[("text", 0)], selectivity);
        let planned = plan(
            &request(vec![lexical("quick brown fox", None)]),
            &registry,
            &stats,
        )
        .expect("plans");
        assert_eq!(
            planned.stratum_depths[&iri(&ex("stratum/text"))],
            1,
            "a cardinality of zero with selectivity {selectivity:?} still reads one row"
        );
    }
}

/// The over-refusal guard for the floor: every neighbouring statistic that was
/// already correct still derives exactly the depth it derived before.
///
/// A floor is a refusal of one input, and the failure mode of getting it wrong
/// is invisible — a depth that is quietly one instead of five looks like a
/// working plan until a caller counts the rows. So the arithmetic either side of
/// zero is executed rather than reasoned about.
#[test]
fn the_floor_leaves_every_other_selectivity_exactly_where_it_was() {
    for (ppm, expected) in [(1_u64, 1_u32), (500_000, 5), (1_000_000, 10)] {
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
        let stats = statistics_reporting(&[("text", 10)], Some(ppm));
        let planned = plan(
            &request(vec![lexical("quick brown fox", None)]),
            &registry,
            &stats,
        )
        .expect("plans");
        assert_eq!(
            planned.stratum_depths[&iri(&ex("stratum/text"))],
            expected,
            "{ppm} ppm of a ten-row bound is {expected}"
        );
    }

    // A producer that genuinely declares one row, with nothing measured about
    // it: the depth is one because the registry said so, not because a floor
    // rescued it.
    let (registry, _) = registry_of(vec![(
        "text",
        Spec::new(
            "text",
            vec![alternative(
                TermPattern::of_kind(TermKind::Literal),
                value_at(1),
            )],
        )
        .rows(1, 1),
    )]);
    let silent = statistics(&[]);
    let planned = plan(
        &request(vec![lexical("quick brown fox", None)]),
        &registry,
        &silent,
    )
    .expect("plans");
    assert_eq!(planned.stratum_depths[&iri(&ex("stratum/text"))], 1);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &silent,
        fusion_profile: None,
    };
    compile(&planned, &env).expect("a one-row stratum admits");
}

/// The mandatory case, which is where an over-refusal would be loudest: the
/// registry promises this producer answers every request it accepts anything of,
/// and a provider reporting zero must not be what takes it out of the plan.
#[test]
fn a_mandatory_producer_under_a_zero_selectivity_plans_admits_and_runs() {
    let (registry, logs) = registry_of(vec![(
        "text",
        Spec::new(
            "text",
            vec![alternative(
                TermPattern::of_kind(TermKind::Literal),
                value_at(1),
            )],
        )
        .rows(10, 10)
        .mandatory(),
    )]);
    let stats = statistics_reporting(&[("text", 10)], Some(0));
    let planned = plan(
        &request(vec![lexical("quick brown fox", None)]),
        &registry,
        &stats,
    )
    .expect("a mandatory producer is planned, not estimated away");
    assert_eq!(planned.stratum_depths[&iri(&ex("stratum/text"))], 1);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&planned, &env).expect("the mandatory producer is bound, so it admits");
    let execution =
        block_on(execute(&compiled, &registry, &*common::empty_dataset())).expect("the unit runs");
    // Ten rows held, one row read: the mandatory producer answered, and the
    // status names the floored depth as what stopped the read rather than
    // claiming the stratum was exhausted at one row.
    assert_eq!(
        execution.statuses[&iri(&ex("stratum/text"))],
        ProducerStatus::DepthReached { rank: 1 }
    );
    assert_eq!(invocations(&logs, "text"), 1);
}

// ---------------------------------------------------------------------------
// 2b. A producer that declares no rows is planned at one row and answers
// ---------------------------------------------------------------------------

/// The registry side of the same invariant: a producer whose every declared
/// access mode promises zero rows has described its data, not forbidden its own
/// invocation, and the honest answer is to read the one floored row from it so
/// the emptiness is reported in its own receipt.
///
/// `rows` is the row count each declared mode reports, so the same two
/// producers can be built promising nothing and promising one row.
fn split_registry(rows: u64, mandatory: bool) -> (PropertyFunctionRegistry, BTreeMap<String, Log>) {
    let under = |predicate: &str| TermPattern {
        kind: TermKind::Literal,
        datatype: None,
        language: None,
        predicate: Some(ex(predicate)),
    };
    let narrow = Spec::new("narrow", vec![alternative(under("secret"), value_at(1))]).rows(rows, 3);
    registry_of(vec![
        (
            "text",
            Spec::new("text", vec![alternative(under("body"), value_at(1))]).rows(10, 3),
        ),
        (
            "narrow",
            if mandatory {
                narrow.mandatory()
            } else {
                narrow
            },
        ),
    ])
}

/// The request the split registry divides: one term each producer's declaration
/// is the only acceptor of.
fn split_request() -> Vec<RequestTerm> {
    vec![
        lexical_under("quick brown fox", "body"),
        lexical_under("hidden", "secret"),
    ]
}

#[test]
fn a_producer_declaring_no_rows_is_planned_at_one_row_and_still_serves_its_term() {
    let (registry, _) = split_registry(0, false);
    let stats = statistics(&[("text", 10), ("narrow", 10)]);
    let planned = plan(&request(split_request()), &registry, &stats).expect("both producers plan");

    assert_eq!(
        rejection(&planned, &ex("pf/narrow")),
        None,
        "a declaration of zero rows describes the data and is not a rejection: {:?}",
        planned.producer_decisions
    );
    assert_eq!(
        planned.stratum_depths[&iri(&ex("stratum/narrow"))],
        1,
        "its stratum records the floored row — the probe that lets it report its own emptiness"
    );
    assert_eq!(
        planned.stratum_depths[&iri(&ex("stratum/text"))],
        10,
        "and the other producer plans exactly as it would have"
    );

    // The term only this producer accepts is bound to it, so nothing goes
    // unserved: a producer that reads and finds nothing has served the term, and
    // reporting it unserved would be reporting that the request never reached a
    // producer at all.
    assert!(
        planned.producer_bindings.iter().any(|binding| {
            binding.producer == ex("pf/narrow") && binding.request_terms == vec![1]
        }),
        "the zero-declaring producer receives the term only it accepts: {:?}",
        planned.producer_bindings
    );
    assert_eq!(
        planned.unserved_terms,
        Vec::<UnservedTerm>::new(),
        "nothing is unserved, and in particular nothing reports \
         {:?}",
        UnservedReason::EveryAcceptingProducerRejected
    );

    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&planned, &env).expect("both strata admit");
    assert_eq!(compiled.units.len(), 2, "both strata emit: {compiled:?}");
    let narrow = compiled
        .units
        .iter()
        .find(|unit| unit.stratum == iri(&ex("stratum/narrow")))
        .expect("the zero-declaring producer's stratum emits a unit");
    assert_eq!(narrow.depth(), 1, "the unit reads one row");
    // The emitted bound is one row past the depth, not `min(depth, 0) + 1`. A
    // `LIMIT 0` here would hand back no row whatever the relation holds, and the
    // stratum would then be reported exhausted with nothing — a completeness claim
    // about the bound, which is the exact defect this arm exists to keep out of the
    // tree. `LIMIT 1` is the same defect one size smaller: a bound EQUAL to the depth
    // leaves no slot for a row to arrive in, so the exhaustion would again be the
    // bound's claim rather than the producer's. A zero declaration is read rather than
    // obeyed, and reading it means probing past it.
    assert!(
        narrow.sparql().ends_with("LIMIT 2"),
        "a declared zero is read one row past its floored depth: {}",
        narrow.sparql()
    );
    assert!(
        !narrow.sparql().contains("LIMIT 0"),
        "no part of the unit is bounded at nothing: {}",
        narrow.sparql()
    );
}

/// A declared zero is READ rather than obeyed, all the way through execution: the
/// emptiness it reports is verified, and a wrong zero is as loud as a wrong bound of
/// any other size.
///
/// The floored depth of one used to be emitted at `LIMIT 1` — a bound *equal* to its
/// own depth, which is the one state `ProbedDepth` exists to make unwritable,
/// reached through the declaration instead of through the depth. Nothing could
/// arrive past it, so the stratum was certified `Exhausted` whatever the index turned
/// out to hold: an index that really was empty and one holding nine rows produced
/// byte-identical trailers. One row past the depth is what tells them apart.
///
/// The probe alone was not enough, and this is the one declaration where it could not
/// be. Every other declaration the waist admits a depth *inside*, so the only row that
/// can exceed it is the probe past the depth; a zero is read at the floor of one, so
/// the first row back is already past the declaration while still inside the depth. A
/// breach compared only about the probe therefore missed it, and one row behind a
/// declaration of none was certified exhausted. All three sizes below — none, one, nine
/// — are executed, so the floor still buys the honest producer its verified emptiness.
#[test]
fn a_declared_zero_is_read_past_rather_than_obeyed_and_a_wrong_one_is_refused() {
    let empty_stratum = iri(&ex("stratum/empty"));
    let terms = || vec![lexical("quick brown fox", None)];
    let holding = |count: usize| {
        registry_of(vec![(
            "empty",
            Spec::new(
                "empty",
                vec![alternative(
                    TermPattern::of_kind(TermKind::Literal),
                    value_at(1),
                )],
            )
            .rows(0, count),
        )])
    };
    let run = |registry: &PropertyFunctionRegistry| {
        let stats = statistics(&[]);
        let planned = plan(&request(terms()), registry, &stats).expect("a declared zero plans");
        assert_eq!(
            planned.stratum_depths[&empty_stratum], 1,
            "the planner floors a declared zero at the one probing row"
        );
        let env = AdmissionEnvironment {
            registry,
            statistics: &stats,
            fusion_profile: None,
        };
        let compiled = compile(&planned, &env).expect("and the floored depth admits");
        assert!(
            compiled.units[0].sparql().ends_with("LIMIT 2"),
            "the emitted bound reads one row PAST the floored depth: {}",
            compiled.units[0].sparql()
        );
        assert_eq!(compiled.units[0].declared_rows(), Some(0));
        block_on(execute(&compiled, registry, &*common::empty_dataset()))
    };

    // The index that really is empty: nothing came back, and the read was allowed a
    // second row, so the exhaustion is the producer's verified report.
    let (really_empty, logs) = holding(0);
    assert_eq!(
        run(&really_empty).expect("the unit runs").statuses[&empty_stratum],
        ProducerStatus::Exhausted { rows_emitted: 0 },
        "the producer was asked, had nothing, and could have said otherwise"
    );
    assert_eq!(
        invocations(&logs, "empty"),
        1,
        "and it really was opened: a bound equal to the depth would have faked this"
    );

    // ONE row behind a declaration of none. The floor of one is what lets an honestly
    // empty producer report its own emptiness; it is not a licence to hold a row. This
    // read came back inside its floored depth, so the breach was invisible to a
    // comparison made only about the row PAST the depth, and the answer was the
    // one ending that names no stopper —
    // `Exhausted { rows_emitted: 1 }` — over a producer that had already contradicted
    // its own registration. The declaration is now compared against the rows pulled
    // whether or not the depth was reached.
    let (holds_one, _) = holding(1);
    let first = run(&holds_one).expect_err("one row from a producer that declared none");
    assert!(
        matches!(
            first,
            ExecutionError::RowBoundBreached {
                declared: 0,
                pulled: 1,
                ..
            }
        ),
        "the FIRST row already breaches a declared zero, and says so: {first:?}"
    );

    // Nine rows behind a declaration of none. The second row arrives too, and the
    // refusal is the same one at the same dimension — the count is the only thing that
    // differs, which is what makes the row above a breach rather than a special case.
    let (holds_nine, _) = holding(9);
    let error = run(&holds_nine).expect_err("a second row from a producer that declared none");
    assert!(
        matches!(
            error,
            ExecutionError::RowBoundBreached {
                declared: 0,
                pulled: 2,
                ..
            }
        ),
        "the breach names both numbers, got {error:?}"
    );
}

#[test]
fn the_same_producer_declaring_one_row_plans_and_serves_its_term() {
    // The neighbour that must still succeed. Nothing about the shape changed;
    // only the row count the declaration promises did.
    let (registry, _) = split_registry(1, false);
    let stats = statistics(&[("text", 10), ("narrow", 10)]);
    let planned = plan(&request(split_request()), &registry, &stats).expect("both producers plan");

    assert_eq!(
        rejection(&planned, &ex("pf/narrow")),
        None,
        "a producer promising one row is not rejected: {:?}",
        planned.producer_decisions
    );
    assert_eq!(
        planned.stratum_depths[&iri(&ex("stratum/narrow"))],
        1,
        "its stratum records the one row it declared"
    );
    assert!(
        planned.producer_bindings.iter().any(|binding| {
            binding.producer == ex("pf/narrow") && binding.request_terms == vec![1]
        }),
        "and it receives the term only it accepts: {:?}",
        planned.producer_bindings
    );
    assert!(
        planned.unserved_terms.is_empty(),
        "so nothing goes unserved: {:?}",
        planned.unserved_terms
    );
}

#[test]
fn a_mandatory_producer_that_declares_no_rows_is_served_rather_than_missing() {
    // A producer the registry insists must answer, whose own declaration says it
    // currently holds nothing. There is no contradiction to refuse: it is bound,
    // planned at one row, and the waist — where every coverage claim is enforced
    // — finds the mandatory producer present rather than missing, because it is.
    let (registry, _) = split_registry(0, true);
    let stats = statistics(&[("text", 10), ("narrow", 10)]);
    let planned = plan(&request(split_request()), &registry, &stats).expect("both producers plan");
    assert_eq!(
        rejection(&planned, &ex("pf/narrow")),
        None,
        "a mandatory producer is not dropped for declaring its data empty: {:?}",
        planned.producer_decisions
    );

    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled =
        compile(&planned, &env).expect("the mandatory producer is served, so this admits");
    assert!(
        compiled
            .units
            .iter()
            .any(|unit| unit.stratum == iri(&ex("stratum/narrow"))),
        "the mandatory producer's stratum emits a unit: {compiled:?}"
    );

    // And where it is the registry's only producer — the shape a host with one
    // index actually has — the request is planned rather than refused. Dropping
    // the producer here left `bindings` empty and produced
    // `PlanError::NoApplicableProducers`, whose message ("no registered producer
    // accepts any term of the request") denies the acceptance the matching pass
    // had already recorded.
    let (alone, _) = registry_of(vec![(
        "narrow",
        Spec::new(
            "narrow",
            vec![alternative(
                TermPattern::of_kind(TermKind::Literal),
                value_at(1),
            )],
        )
        .rows(0, 3)
        .mandatory(),
    )]);
    let solo_stats = statistics(&[("narrow", 10)]);
    let solo = plan(
        &request(vec![lexical("quick brown fox", None)]),
        &alone,
        &solo_stats,
    )
    .expect("a lone zero-declaring producer is planned, not refused");
    assert_eq!(
        solo.stratum_depths[&iri(&ex("stratum/narrow"))],
        1,
        "at the floored depth of one"
    );
    assert!(
        solo.unserved_terms.is_empty(),
        "and the term it accepts is served: {:?}",
        solo.unserved_terms
    );
    let solo_env = AdmissionEnvironment {
        registry: &alone,
        statistics: &solo_stats,
        fusion_profile: None,
    };
    compile(&solo, &solo_env).expect("and the sole-producer plan admits");
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
    // The argument carries the *emitted* bound — the depth of four plus the one
    // probe row the producer's ten-row declaration leaves room for. For a
    // producer that bounds itself from this argument there is no other bound in
    // the text at all, so a probe it was never told about would not be a probe:
    // the relation would stop at four and the executor could never distinguish
    // a fifth row from none.
    assert!(
        sparql.contains(&format!("\"5\"^^<{XSD_INTEGER}>")),
        "the depth, probed one deeper, rides as an argument: {sparql}"
    );
    assert!(
        !sparql.contains("}} LIMIT 5"),
        "a producer that took the depth as an argument gets no branch LIMIT: {sparql}"
    );
    assert!(
        sparql.ends_with("LIMIT 5"),
        "the unit still carries the stratum's contract with fusion: {sparql}"
    );
}

/// A producer that bounds itself at its own declaration reports that bound as the
/// stopper, and a **wrong** declaration there is still caught by the unit's own
/// `LIMIT`.
///
/// This is the shipped nearest-neighbour relation's shape, driven end to end: the
/// depth is handed over as an argument rather than applied as a ceiling, and the
/// argument is never raised past the row count the producer registered. So at a
/// depth already on that declaration the read is asked for exactly `depth` rows and
/// the slot past the depth can never be filled, however many rows the index holds.
///
/// Four arms, because the ending has to be exactly as narrow as the observation:
/// the unobservable one, the wrong declaration that is still loud, the short answer
/// that really is exhaustion, and the same producer read below its declaration,
/// where the argument carries the probe and the ordinary ending returns.
#[test]
fn a_self_bounding_producer_reports_the_bound_that_stopped_it_and_still_catches_a_wrong_one() {
    let knn = iri(&ex("stratum/knn"));
    let terms = || vec![lexical("quick brown fox", None)];
    let placement = || DepthPlacement {
        position: 2,
        datatype: XSD_INTEGER.to_owned(),
    };
    // `rows` is what the producer declares per invocation and `count` is what this
    // mock really holds; the mock ignores both the depth argument and the engine's
    // ceiling, which is what lets one fixture play an honest producer and a liar.
    let spec = |rows: u64, count: usize| {
        registry_of(vec![("knn", knn_spec(Some(placement())).rows(rows, count))]).0
    };
    let run = |registry: &PropertyFunctionRegistry, stats: &MockStatistics| {
        let planned = plan(&request(terms()), registry, stats).expect("the fixture request plans");
        let env = AdmissionEnvironment {
            registry,
            statistics: stats,
            fusion_profile: None,
        };
        let compiled = compile(&planned, &env).expect("a fresh plan is admitted");
        let depth = compiled.units[0].depth();
        let sparql = compiled.units[0].sparql();
        (
            depth,
            sparql,
            block_on(execute(&compiled, registry, &*common::empty_dataset())),
        )
    };

    // (a) Declared three, holds three, planned at three. The argument is three —
    //     never four, which would ask the producer to breach its registration — so
    //     the unit's `LIMIT 4` can never be filled and NOTHING here can say whether
    //     a fourth row exists. `Exhausted` would be a completeness claim minted from
    //     the declaration; the ending names the declaration instead.
    let honest = spec(3, 3);
    let (depth, sparql, execution) = run(&honest, &statistics(&[]));
    assert_eq!(depth, 3, "the declared bound is the depth");
    assert!(
        sparql.contains(&format!("\"3\"^^<{XSD_INTEGER}>")),
        "the argument stops at the declaration: {sparql}"
    );
    assert!(
        sparql.ends_with("LIMIT 4"),
        "while the unit's own bound still reaches one row past it: {sparql}"
    );
    assert_eq!(
        execution.expect("the unit runs").statuses[&knn],
        ProducerStatus::RowBoundReached { rank: 3 },
        "the read went to the producer's own declared bound, and whether anything \
         lies below it was not observable"
    );

    // (b) The same declaration over a producer holding nine rows. The unit's bound
    //     reaches one row past the declaration, so the fourth row arrives and the
    //     run is refused by name — a wrong declaration is still loud here, which is
    //     what the emitted `LIMIT` buys that the argument cannot.
    let liar = spec(3, 9);
    let (depth, _, execution) = run(&liar, &statistics(&[]));
    assert_eq!(depth, 3);
    let error = execution.expect_err("a fourth row from a producer that declared three");
    assert!(
        matches!(
            error,
            ExecutionError::RowBoundBreached {
                declared: 3,
                pulled: 4,
                ..
            }
        ),
        "the breach is refused by name, got {error:?}"
    );

    // (c) The same declaration over a producer holding one row. It returned fewer
    //     rows than it was allowed, so nothing stopped it and the exhaustion is
    //     verified rather than assumed — this ending must not spread to the case
    //     where the read really did run out.
    let short = spec(3, 1);
    let (_, _, execution) = run(&short, &statistics(&[]));
    assert_eq!(
        execution.expect("the unit runs").statuses[&knn],
        ProducerStatus::Exhausted { rows_emitted: 1 },
        "a short answer to a request for three is exhaustion, verified"
    );

    // (d) The neighbour where the ending IS observable: the same producer declaring
    //     ten rows with a measured cardinality of four, so the depth sits BELOW the
    //     declaration. The argument then carries the probe row, the ninth row the
    //     mock holds arrives in it, and the ordinary `DepthReached` returns.
    let deep = spec(10, 9);
    let (depth, sparql, execution) = run(&deep, &statistics(&[("knn", 4)]));
    assert_eq!(depth, 4, "the measured cardinality narrows the depth");
    assert!(
        sparql.contains(&format!("\"5\"^^<{XSD_INTEGER}>")),
        "and the argument carries the probe row, because the declaration leaves room: {sparql}"
    );
    assert_eq!(
        execution.expect("the unit runs").statuses[&knn],
        ProducerStatus::DepthReached { rank: 4 },
        "a depth below the declaration is probed like any other read"
    );
}

/// The same unit, running `query` — a text this test wrote — with every number the
/// compiler put on it unchanged.
///
/// [`StratumUnit::new`] is the only way to hand `execute` a query nobody compiled, and
/// it is the seam the attack below has to go through, because it is the only one a
/// caller has: the compiled query is no longer a string on the unit for anything to
/// assign to.
fn supplying(unit: &StratumUnit, query: String) -> StratumUnit {
    StratumUnit::new(
        unit.stratum.clone(),
        query,
        unit.contract.clone(),
        unit.depth(),
        unit.declared_rows(),
    )
    .expect("the compiler's own depth and declaration are admitted")
}

/// A unit's text with this layer's outer bound removed, so a caller can hand it back.
///
/// [`StratumUnit::new`] renders that bound itself; a text carrying a second one is not
/// a query the grammar accepts, which is a loud parse failure rather than a quiet
/// wrong answer, and not what is being measured here.
fn without_the_outer_bound(text: &str, probe: u32) -> String {
    let (body, outer) = text
        .rsplit_once("\nLIMIT ")
        .expect("a unit's text ends with the bound this layer renders");
    assert_eq!(
        outer,
        probe.to_string(),
        "the outer bound is the probe row: {text}"
    );
    body.to_owned()
}

/// The same text with the **branch's** own copy of the probe row lowered to `depth`.
///
/// This is the mutation the defect needed, and it is one of exactly two shapes: the
/// inner `LIMIT` of an evaluator-bounded branch, or the depth argument of a
/// self-bounding producer. Either way it erases the slot the probe row would have
/// arrived in while every number on the unit still reads as it did.
///
/// Done by substitution rather than as a fixed string on purpose: the
/// single-occurrence assertion fails if the emitted text ever stops carrying exactly
/// one copy of that number, instead of silently lowering something else.
fn with_the_branch_bound_lowered(text: &str, probe: u32, depth: u32) -> String {
    let body = without_the_outer_bound(text, probe);
    let probe = probe.to_string();
    assert_eq!(
        body.matches(&probe).count(),
        1,
        "the branch carries exactly one copy of the probe row, and it is the number \
         this lowers: {body}"
    );
    body.replace(&probe, &depth.to_string())
}

/// **A bound inside a caller's own text is never reported as an exhaustion, in either
/// shape the branch's bound is written in.**
///
/// The read's bound has had three homes on this type, and the first two shared one
/// property: a caller could write it. While the whole query text was a writable field,
/// `LIMIT <depth>` in place of `LIMIT <depth + 1>` left no slot for the probe row and
/// the read was certified `Exhausted` over a producer with six more rows behind it.
/// Rendering the *outer* bound from the depth moved that hole inward rather than
/// closing it: the branch's own bound was still inside the same writable string, and
/// where an inner bound and an outer one disagree the inner one decides the read. So
/// `Exhausted { rows_emitted: 3 }` over nine rows came back a second time, from the
/// same numbers.
///
/// Both shapes are executed here, because they write that inner bound differently — a
/// `LIMIT` on the branch for a producer the evaluator bounds, and a rendered *argument*
/// for one that bounds itself — and a fix that closed one and left the other is
/// precisely what happened twice.
///
/// Each shape is paired with the neighbour that must still answer: the same
/// caller-supplied text, **not** lowered, which still reports the ending it can
/// observe. The refusal here is narrow by design, and the last arm measures the edge
/// of it — a caller's query still runs, still yields its rows in rank order, still
/// reports `DepthReached` when a row past the depth really did arrive, and still trips
/// [`ExecutionError::RowBoundBreached`](purrdf_retrieval::ExecutionError) when the
/// producer beats its own declaration. The one thing it cannot do is report the
/// one ending that names no stopper, because that ending rests on a bound this
/// layer wrote and can see.
#[test]
fn a_bound_lowered_in_a_caller_supplied_text_reports_that_text_and_never_an_exhaustion() {
    let terms = || vec![lexical("quick brown fox", None)];
    let compiled_for = |registry: &PropertyFunctionRegistry, stats: &MockStatistics| {
        let planned = plan(&request(terms()), registry, stats).expect("the fixture request plans");
        let env = AdmissionEnvironment {
            registry,
            statistics: stats,
            fusion_profile: None,
        };
        compile(&planned, &env).expect("a fresh plan is admitted")
    };
    let status =
        |compiled: &CompiledRetrieval, registry: &PropertyFunctionRegistry, stratum: &Iri| {
            block_on(execute(compiled, registry, &*common::empty_dataset()))
                .expect("the unit runs")
                .statuses[stratum]
                .clone()
        };
    let running = |compiled: &CompiledRetrieval, query: String| {
        CompiledRetrieval::new(
            vec![supplying(&compiled.units[0], query)],
            compiled.plan_id,
            compiled.registry_id,
            compiled.registry_fingerprint.clone(),
            compiled.fused_bound,
            compiled.resolution.clone(),
        )
    };

    // (1) THE EVALUATOR-BOUNDED SHAPE. Nine rows behind a depth of three, bounded by
    //     the branch's own `LIMIT 4`.
    let cut = iri(&ex("stratum/cut"));
    let (registry, _) = registry_of(vec![(
        "cut",
        Spec::new(
            "cut",
            vec![alternative(
                TermPattern::of_kind(TermKind::Any),
                value_at(1),
            )],
        )
        .rows(1_000, 9),
    )]);
    let stats = statistics(&[("cut", 3)]);
    let rendered = compiled_for(&registry, &stats);
    let text = rendered.units[0].sparql();
    assert_eq!(
        rendered.units[0].depth(),
        3,
        "the measured cardinality is the depth"
    );
    assert!(
        text.contains(" LIMIT 4 }"),
        "the branch carries the row ceiling the evaluator pushes down: {text}"
    );
    assert_eq!(
        status(&rendered, &registry, &cut),
        ProducerStatus::DepthReached { rank: 3 },
        "the rendered read is cut by the depth and says so"
    );

    let attacked = running(&rendered, with_the_branch_bound_lowered(&text, 4, 3));
    assert_eq!(
        status(&attacked, &registry, &cut),
        ProducerStatus::SuppliedQueryEnded { rank: 3 },
        "a text this layer did not write cannot certify an exhaustion, whatever bound \
         it carries: the ending names that text"
    );
    let unlowered = running(&rendered, without_the_outer_bound(&text, 4));
    assert_eq!(
        status(&unlowered, &registry, &cut),
        ProducerStatus::DepthReached { rank: 3 },
        "and a caller's text that did NOT cut the read still reports the ending that \
         WAS observed, because a row past the depth is an observation"
    );

    // (2) THE SELF-BOUNDING SHAPE. The depth rides as an argument, so the number to
    //     lower is that argument — and the fixture relation obeys it, which is what
    //     makes lowering it observable at all.
    let knn = iri(&ex("stratum/knn"));
    let (registry, _) = registry_of(vec![(
        "knn",
        knn_spec(None)
            .obeying(DepthPlacement {
                position: 2,
                datatype: XSD_INTEGER.to_owned(),
            })
            .rows(10, 9),
    )]);
    let stats = statistics(&[("knn", 4)]);
    let rendered = compiled_for(&registry, &stats);
    let text = rendered.units[0].sparql();
    assert_eq!(
        rendered.units[0].depth(),
        4,
        "the measured cardinality is the depth"
    );
    assert!(
        text.contains(&format!("\"5\"^^<{XSD_INTEGER}>")),
        "the argument carries the probe row, because the declaration leaves room: {text}"
    );
    assert_eq!(
        status(&rendered, &registry, &knn),
        ProducerStatus::DepthReached { rank: 4 },
        "the rendered read reaches past the depth and is cut by it"
    );

    let attacked = running(&rendered, with_the_branch_bound_lowered(&text, 5, 4));
    assert_eq!(
        status(&attacked, &registry, &knn),
        ProducerStatus::SuppliedQueryEnded { rank: 4 },
        "the argument is a bound like any other, and lowering it in a caller's text \
         buys the same refusal to certify"
    );
    let unlowered = running(&rendered, without_the_outer_bound(&text, 5));
    assert_eq!(
        status(&unlowered, &registry, &knn),
        ProducerStatus::DepthReached { rank: 4 },
        "while the unlowered text still reports what it could observe"
    );

    // (3) AND THE REFUSAL A CALLER'S TEXT DOES NOT ESCAPE. A row past the *declaration*
    //     is an observation like the row past the depth, so a producer that returns
    //     more rows than it registered is still refused by name — the weaker ending
    //     above withholds a completeness claim, and withholds nothing else.
    let (registry, _) = registry_of(vec![(
        "cut",
        Spec::new(
            "cut",
            vec![alternative(
                TermPattern::of_kind(TermKind::Any),
                value_at(1),
            )],
        )
        .rows(3, 9),
    )]);
    let rendered = compiled_for(&registry, &statistics(&[]));
    assert_eq!(rendered.units[0].depth(), 3, "the declaration is the depth");
    let supplied = running(
        &rendered,
        without_the_outer_bound(&rendered.units[0].sparql(), 4),
    );
    let error = block_on(execute(&supplied, &registry, &*common::empty_dataset()))
        .expect_err("a fourth row from a producer that declared three");
    assert!(
        matches!(
            error,
            ExecutionError::RowBoundBreached {
                declared: 3,
                pulled: 4,
                ..
            }
        ),
        "the breach is refused by name whoever wrote the query, got {error:?}"
    );
}

/// **A prologue survives the wrap over the self-bounding shape too, where the number
/// the layer rendered rides inside the caller's body.**
///
/// The evaluator-bounded shape's version of this is in `tests/boundary.rs`. This is the
/// other one, and it is not the same text: a producer that declares a
/// [`DepthPlacement`] takes its row request as an *argument*, so the number the layer
/// computed is inside the body being wrapped rather than outside it, and the body also
/// carries a typed literal whose datatype a prologue can name. Both the relation and
/// that datatype are spelled through prefixes here, so a hoist that moved the
/// directives but disturbed the body would resolve one of them against nothing.
///
/// The valid neighbour is the same text with its IRIs absolute, executed beside it:
/// both must report the same ending over the same producer, because the prologue is a
/// spelling and not a change of read.
#[test]
fn a_prefixed_supplied_text_over_a_self_bounding_producer_reads_what_the_absolute_one_does() {
    let knn = iri(&ex("stratum/knn"));
    let (registry, _) = registry_of(vec![(
        "knn",
        knn_spec(None)
            .obeying(DepthPlacement {
                position: 2,
                datatype: XSD_INTEGER.to_owned(),
            })
            .rows(10, 9),
    )]);
    let stats = statistics(&[("knn", 4)]);
    let planned = plan(
        &request(vec![lexical("quick brown fox", None)]),
        &registry,
        &stats,
    )
    .expect("the fixture request plans");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&planned, &env).expect("the plan is admitted");
    assert_eq!(
        compiled.units[0].depth(),
        4,
        "the measured cardinality is the depth"
    );
    let status = |unit: StratumUnit| {
        let bundle = CompiledRetrieval::new(
            vec![unit],
            compiled.plan_id,
            compiled.registry_id,
            compiled.registry_fingerprint.clone(),
            compiled.fused_bound,
            compiled.resolution.clone(),
        );
        block_on(execute(&bundle, &registry, &*common::empty_dataset()))
            .expect("the unit runs")
            .statuses[&knn]
            .clone()
    };

    // The compiler's own text with this layer's outer bound taken off, so a caller can
    // hand it back: the depth argument the layer rendered is still inside it.
    let absolute = without_the_outer_bound(&compiled.units[0].sparql(), 5);
    assert!(
        absolute.contains(&format!("\"5\"^^<{XSD_INTEGER}>")),
        "the row request rides inside the body, so the wrap has to leave it alone: \
         {absolute}"
    );
    assert_eq!(
        status(supplying(&compiled.units[0], absolute.clone())),
        ProducerStatus::DepthReached { rank: 4 },
        "the producer was asked for five rows, returned five, and the depth is the \
         stopper"
    );

    // The same text with both the relation and the datatype spelled through prefixes
    // the caller declares. Hoisted, the directives scope the whole wrapped query, so
    // both resolve exactly as their absolute spellings did.
    let prefixed = format!(
        "PREFIX rel: <{}>\nPREFIX xsd: <{}>\n{}",
        ex("pf/"),
        "http://www.w3.org/2001/XMLSchema#",
        absolute
            .replace(&format!("<{}>", ex("pf/knn")), "rel:knn")
            .replace(&format!("^^<{XSD_INTEGER}>"), "^^xsd:integer")
    );
    assert!(
        prefixed.contains("rel:knn") && prefixed.contains("\"5\"^^xsd:integer"),
        "the body under test really is prefixed: {prefixed}"
    );
    assert_eq!(
        status(supplying(&compiled.units[0], prefixed)),
        ProducerStatus::DepthReached { rank: 4 },
        "a prefixed text reads what the absolute one read: the prologue moved and the \
         body did not"
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
             LIMIT 6 }}\n}}\nLIMIT 6",
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
            .sparql()
            .contains("( ?c0 ) <http://example.org/pf/pair> ( <http://example.org/a> \"needle\" )"),
        "each alternative renders into its own declared position: {}",
        forward.units[0].sparql()
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
            units[name].matches("LIMIT 5").count(),
            2,
            "the branch bound plus the unit's own, each the depth of four probed \
             one row deeper: {}",
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
        Some(3),
        "the branch LIMIT — the depth of two, probed one row deeper — is observed \
         to reach the relation as its row ceiling"
    );
}

// The evaluator's own cap-pushdown claim — that each arm of a `UNION` is offered
// its own `LIMIT` as a row ceiling — used to be pinned from here, by a stratum
// with two producers. The registry now refuses that configuration, and the claim
// never depended on retrieval reaching it: it is a fact about the evaluator's
// descent past a `UNION`, and it is proved in `purrdf-sparql-eval` against
// hand-written two-arm queries, including the constant-bearing arm this compiler
// emits.

// ---------------------------------------------------------------------------
// 13. The request's own bound is what the depth is derived from
//
// The defect this section exists for: a fused top-k over strata whose declared
// candidate blocks do not overlap read its whole corpus. The declarations bought
// a shorter walk of a stream that had already been MATERIALIZED in full, because
// the compiled unit was still `LIMIT <corpus>` — the bound arrived at `fuse`,
// four stages after the depth was chosen. The measurement moved; the work did
// not.
//
// The claim below is the one that fixes it, and it is about the *materialized*
// read: the same `top_k` over two corpora of different sizes compiles to the same
// per-stratum `LIMIT` and records the same depth. Its neighbours are here too,
// because a rule that fired where it must not would be the mirror defect — a
// truncated ranked list with nothing anywhere saying so.
// ---------------------------------------------------------------------------

/// Two producers over one needle, each ranking its own stratum, with the blocks
/// each may name declared as `domains` says.
///
/// `domains` is a pair of tag local names, or `None` for the unrestricted promise
/// both producers made before the term existed. `rows` is the corpus size each
/// producer declares it can yield per invocation, which is what stands in here
/// for a corpus: it is the number the planner's depth comes from when the request
/// licenses no narrowing.
fn two_stratum_registry(rows: u64, domains: Option<(&str, &str)>) -> PropertyFunctionRegistry {
    let corpus = usize::try_from(rows).expect("the fixture corpus fits in a usize");
    two_stratum_corpus(rows, corpus, domains)
}

/// The same pair, declaring `rows` over a corpus of two.
///
/// The declaration and the corpus part company here on purpose: these cases are
/// about the depth a declaration licenses, and a fixture that really held four
/// billion rows would be a test of the allocator rather than of the planner.
fn two_stratum_registry_declaring(
    rows: u64,
    domains: Option<(&str, &str)>,
) -> PropertyFunctionRegistry {
    two_stratum_corpus(rows, 2, domains)
}

/// Both producers declaring `rows` per invocation over a corpus of `corpus` rows.
fn two_stratum_corpus(
    rows: u64,
    corpus: usize,
    domains: Option<(&str, &str)>,
) -> PropertyFunctionRegistry {
    let accepted = || {
        vec![alternative(
            TermPattern::of_kind(TermKind::Literal),
            value_at(1),
        )]
    };
    let notes = Spec::new("notes", accepted()).rows(rows, corpus);
    let titles = Spec::new("titles", accepted()).rows(rows, corpus);
    let (notes, titles) = match domains {
        None => (notes, titles),
        Some((left, right)) => (notes.within(left), titles.within(right)),
    };
    registry_of(vec![("notes", notes), ("titles", titles)]).0
}

/// Each stratum's recorded depth and the `LIMIT` its unit was emitted at, keyed
/// by the stratum's local name.
///
/// Both numbers, together, because either alone would leave the defect
/// expressible. A narrowed `LIMIT` beside an unnarrowed depth is the plan
/// recording a read nobody took; an unnarrowed `LIMIT` beside a narrowed depth is
/// the corpus still being materialized.
fn depths_and_limits(
    registry: &PropertyFunctionRegistry,
    stats: &MockStatistics,
    request: &RetrievalRequest,
) -> BTreeMap<String, (u32, u32)> {
    let planned = plan(request, registry, stats).expect("the fixture request plans");
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
            (name, (unit.depth(), emitted_limit(&unit.sparql())))
        })
        .collect()
}

/// The `LIMIT` a unit's outer `SELECT` carries.
fn emitted_limit(sparql: &str) -> u32 {
    sparql
        .rsplit("LIMIT ")
        .next()
        .expect("an emitted unit carries a LIMIT")
        .trim()
        .parse()
        .expect("an emitted LIMIT is a number")
}

#[test]
fn a_bounded_request_over_disjoint_strata_reads_a_flat_prefix_of_each() {
    let stats = statistics(&[]);
    let bounded = RetrievalRequest::bounded(vec![lexical("quick brown fox", None)], TopK::new(5));

    // The same request over a corpus and over a corpus four times its size. The
    // read is identical, which is the whole claim: a top-five answer over
    // disjoint strata costs five rows per stratum however long the streams are.
    let small = depths_and_limits(
        &two_stratum_registry(400, Some(("block/notes", "block/titles"))),
        &stats,
        &bounded,
    );
    let large = depths_and_limits(
        &two_stratum_registry(1_600, Some(("block/notes", "block/titles"))),
        &stats,
        &bounded,
    );
    assert_eq!(
        small, large,
        "the materialized read is flat in corpus size, so neither the recorded \
         depth nor the emitted LIMIT may move with it"
    );
    assert_eq!(
        small,
        BTreeMap::from([("notes".to_owned(), (5, 6)), ("titles".to_owned(), (5, 6)),]),
        "five rows per stratum is the exact depth, and the probe row sits one \
         past it exactly as it does at any other depth"
    );
}

#[test]
fn the_unbounded_and_overlapping_neighbours_keep_the_declared_depth() {
    let stats = statistics(&[]);
    let needle = || vec![lexical("quick brown fox", None)];
    let disjoint = || Some(("block/notes", "block/titles"));
    // What the declared bound alone says, at each size: this is the depth every
    // neighbour below must still carry.
    let declared_small = BTreeMap::from([
        ("notes".to_owned(), (400, 401)),
        ("titles".to_owned(), (400, 401)),
    ]);
    let declared_large = BTreeMap::from([
        ("notes".to_owned(), (1_600, 1_601)),
        ("titles".to_owned(), (1_600, 1_601)),
    ]);

    // 1. A complete request. It asks for everything the strata hold, so the
    //    declaration is the depth even though the blocks are disjoint — the
    //    narrowing is licensed by the BOUND, and this request states none.
    for (rows, expected) in [(400, &declared_small), (1_600, &declared_large)] {
        assert_eq!(
            &depths_and_limits(
                &two_stratum_registry(rows, disjoint()),
                &stats,
                &RetrievalRequest::complete(needle()),
            ),
            expected,
            "a complete request licenses no prefix, at any corpus size"
        );
    }

    // 2. Overlapping declarations. Both producers say they draw from the same
    //    block, so a candidate can be named by both and its fused score is a SUM
    //    across strata; the merge argument the prefix rests on has no premise and
    //    the declared bound stands.
    let overlapping = Some(("block/shared", "block/shared"));
    for (rows, expected) in [(400, &declared_small), (1_600, &declared_large)] {
        assert_eq!(
            &depths_and_limits(
                &two_stratum_registry(rows, overlapping),
                &stats,
                &RetrievalRequest::bounded(needle(), TopK::new(5)),
            ),
            expected,
            "declarations that meet license no prefix"
        );
    }

    // 3. No declaration at all — the widest promise, and the one every producer
    //    made before candidate domains existed. It says nothing about which
    //    candidates it will NOT name, so it supplies no premise either.
    for (rows, expected) in [(400, &declared_small), (1_600, &declared_large)] {
        assert_eq!(
            &depths_and_limits(
                &two_stratum_registry(rows, None),
                &stats,
                &RetrievalRequest::bounded(needle(), TopK::new(5)),
            ),
            expected,
            "an unrestricted stratum licenses no prefix"
        );
    }
}

#[test]
fn a_prefix_never_raises_a_depth_and_never_narrows_one_to_nothing() {
    let needle = || vec![lexical("quick brown fox", None)];
    let disjoint = Some(("block/notes", "block/titles"));

    // A bound above what the producers declared is not a licence to read deeper:
    // every step of the derivation is a `min`, so the declaration still wins.
    assert_eq!(
        depths_and_limits(
            &two_stratum_registry(7, disjoint),
            &statistics(&[]),
            &RetrievalRequest::bounded(needle(), TopK::new(5_000)),
        ),
        BTreeMap::from([("notes".to_owned(), (7, 8)), ("titles".to_owned(), (7, 8)),]),
        "a bound narrows a depth the registry already set, or it changes nothing"
    );

    // A measured cardinality below the bound still binds, for the same reason.
    assert_eq!(
        depths_and_limits(
            &two_stratum_registry(400, disjoint),
            &statistics(&[("notes", 3)]),
            &RetrievalRequest::bounded(needle(), TopK::new(5)),
        ),
        BTreeMap::from([("notes".to_owned(), (3, 4)), ("titles".to_owned(), (5, 6)),]),
        "the depth is the smallest of the declaration, the measurement and the bound"
    );

    // And a bound of zero is floored at one row, exactly as a measured zero is:
    // a bound may narrow a read and may never eliminate one, because emptiness is
    // the producer's to report and a `LIMIT 0` would report it for them.
    assert_eq!(
        depths_and_limits(
            &two_stratum_registry(400, disjoint),
            &statistics(&[]),
            &RetrievalRequest::bounded(needle(), TopK::new(0)),
        ),
        BTreeMap::from([("notes".to_owned(), (1, 2)), ("titles".to_owned(), (1, 2)),]),
        "the floor of one survives the bound, and the probe row survives the floor"
    );
}

#[test]
fn a_bound_gives_an_unbounded_declaration_a_finite_depth_and_its_absence_reads_to_the_ceiling() {
    let stats = statistics(&[]);
    let needle = || vec![lexical("quick brown fox", None)];
    let disjoint = Some(("block/notes", "block/titles"));
    let unbounded = || two_stratum_registry(u64::MAX, disjoint);
    let deepest = u32::MAX - 1;

    // A producer that declares unboundedly many rows, read for an answer that
    // provably cannot use more than five of them, is a read of five rows.
    assert_eq!(
        depths_and_limits(
            &unbounded(),
            &stats,
            &RetrievalRequest::bounded(needle(), TopK::new(5)),
        ),
        BTreeMap::from([("notes".to_owned(), (5, 6)), ("titles".to_owned(), (5, 6)),]),
        "the request's bound is a real bound on a read that otherwise has none"
    );

    // The neighbour that used to be refused: the same registry, asked for
    // everything. Nothing narrows the read, so the declaration stands — and a
    // declaration of `u64::MAX` says the same thing about a *read* that a
    // declaration of ten billion does, which is that it holds more rows than a read
    // can reach. That is recorded at the ceiling with the probe row one past it, so
    // the ending names the planned depth as the stopper, exactly as it does for
    // every other oversized declaration. `PlanError::StatisticsUnavailable` refused
    // this while serving the neighbour one row below it, which separated a
    // declaration from itself and bought nothing the ending does not report.
    for declared in [u64::MAX, u64::MAX - 1, u64::from(u32::MAX) + 100] {
        assert_eq!(
            depths_and_limits(
                &two_stratum_registry_declaring(declared, disjoint),
                &stats,
                &RetrievalRequest::complete(needle()),
            ),
            BTreeMap::from([
                ("notes".to_owned(), (deepest, u32::MAX)),
                ("titles".to_owned(), (deepest, u32::MAX)),
            ]),
            "a declaration of {declared} rows reads to the ceiling, not to a refusal"
        );
    }

    // And the valid neighbour at the other end: an ordinary declaration, bounded by
    // nothing but itself, is derived exactly as it always was.
    assert_eq!(
        depths_and_limits(
            &two_stratum_registry(400, disjoint),
            &stats,
            &RetrievalRequest::complete(needle()),
        ),
        BTreeMap::from([
            ("notes".to_owned(), (400, 401)),
            ("titles".to_owned(), (400, 401)),
        ]),
        "a declaration a read can reach is untouched by any of it"
    );
}

/// One stratum whose producer *declares* `rows` rows per invocation and emits two,
/// optionally restricted to a single block of the candidate universe.
///
/// The declared number and the corpus behind it are deliberately unrelated: these
/// cases are about the depth a declaration licenses, and a fixture that really held
/// four billion rows would be a test of the allocator. The block is a parameter
/// because a declared block set is one of the two premises that let a request's own
/// bound become the depth — without it the declaration stands and the bound narrows
/// nothing at all.
fn deep_registry(rows: u64, block: Option<&str>) -> PropertyFunctionRegistry {
    let accepted = vec![alternative(
        TermPattern::of_kind(TermKind::Literal),
        value_at(1),
    )];
    let spec = Spec::new("deep", accepted).rows(rows, 2);
    let spec = match block {
        None => spec,
        Some(tag) => spec.within(tag),
    };
    registry_of(vec![("deep", spec)]).0
}

/// [`deep_registry`]'s shape at two strata, each declaring `rows` and neither
/// declaring a block.
///
/// Two, because the disjointness premise is about *pairs*: one stratum has no pair
/// and needs no declaration, while a second one makes `Unrestricted` a promise that
/// is actually missing. This is the smallest registry where that difference shows,
/// and its corpus stays at two rows per producer so the declaration can be any size.
fn two_deep_registry(rows: u64) -> PropertyFunctionRegistry {
    let accepted = || {
        vec![alternative(
            TermPattern::of_kind(TermKind::Literal),
            value_at(1),
        )]
    };
    registry_of(vec![
        ("deep", Spec::new("deep", accepted()).rows(rows, 2)),
        ("wide", Spec::new("wide", accepted()).rows(rows, 2)),
    ])
    .0
}

/// [`deep_registry`]'s declaration, one field over: the producer says its stream
/// may repeat a candidate.
///
/// That is the other shape a request's bound cannot narrow — a stream whose rows
/// are not its candidates holds fewer than `k` candidates in `k` rows, so no
/// arrangement of declared blocks makes the prefix argument hold.
fn repeating_deep_registry(rows: u64) -> PropertyFunctionRegistry {
    let accepted = vec![alternative(
        TermPattern::of_kind(TermKind::Literal),
        value_at(1),
    )];
    registry_of(vec![(
        "deep",
        Spec::new("deep", accepted).rows(rows, 2).repeats(),
    )])
    .0
}

#[test]
fn a_declared_row_bound_past_the_read_range_is_planned_at_the_ceiling() {
    let stats = statistics(&[]);
    let needle = || vec![lexical("quick brown fox", None)];
    // The deepest depth a read can be taken to. It is one shallower than the
    // deepest a plan can express, because the read is emitted one row past the
    // depth: that probe row is how the executor tells a read the bound cut from a
    // read that ran out, and at `u32::MAX` it is not a number a `LIMIT` can hold.
    let deepest = u32::MAX - 1;

    // A declaration of exactly that many rows plans, records the depth, and is
    // emitted with the probe row one past it.
    assert_eq!(
        depths_and_limits(
            &deep_registry(u64::from(deepest), None),
            &stats,
            &RetrievalRequest::complete(needle()),
        ),
        BTreeMap::from([("deep".to_owned(), (deepest, u32::MAX))]),
        "the deepest readable depth is read, recorded and probed like any other"
    );

    // One row deeper, and a declaration well past the range: both are honest
    // descriptions of an index larger than a read can be taken to, so both are
    // recorded AT the ceiling rather than refused. The old truncation to `u32::MAX`
    // was wrong twice over — it recorded a depth below the bound it claimed to serve
    // with nothing comparing the two, and at that exact value it left the compiler
    // no room for the probe row, so the read was reported exhausted whatever the
    // relation held. At the ceiling the probe row fits, so a read this bound cuts
    // ends `DepthReached`, which names the planned depth as the stopper.
    for declared in [u64::from(u32::MAX), u64::from(u32::MAX) + 100] {
        assert_eq!(
            depths_and_limits(
                &deep_registry(declared, None),
                &stats,
                &RetrievalRequest::complete(needle()),
            ),
            BTreeMap::from([("deep".to_owned(), (deepest, u32::MAX))]),
            "a declaration of {declared} rows is read to the ceiling, not refused"
        );
    }

    // The unbounded declaration lands here too, and the pair below is why it must.
    // `u64::MAX` and `u64::MAX - 1` are two spellings of "more rows than a read can
    // reach": the same depth, the same emitted bound, the same ending. Recording one
    // and refusing the other would have been a refusal one declared row wide.
    for declared in [u64::MAX, u64::MAX - 1] {
        assert_eq!(
            depths_and_limits(
                &deep_registry(declared, None),
                &stats,
                &RetrievalRequest::complete(needle()),
            ),
            BTreeMap::from([("deep".to_owned(), (deepest, u32::MAX))]),
            "an unbounded declaration reads to the ceiling like any other oversized \
             one: {declared}"
        );
    }

    // And an ordinary declaration is untouched by any of it.
    assert_eq!(
        depths_and_limits(
            &deep_registry(7, None),
            &stats,
            &RetrievalRequest::complete(needle()),
        ),
        BTreeMap::from([("deep".to_owned(), (7, 8))]),
        "a depth nowhere near the ceiling is derived exactly as it was"
    );
}

/// A small `top_k` over an honestly oversized declaration is served, whatever the
/// producer's shape.
///
/// The shapes below are the whole of what decides whether a request's bound can
/// become the depth. A `Unique` stratum narrows — with or without a declared block,
/// because one stratum has no pair for a disjointness premise to be about — while
/// `Allowed` cannot, and neither can an undeclared stratum once a second one exists
/// to overlap with it.
///
/// The non-narrowing shapes are the ordinary configuration rather than an exotic one
/// — `Allowed` says the producer's rows are not its candidates — so a refusal here
/// would have left a host two dishonest ways out: under-declare
/// `rows_per_invocation`, or fabricate a cardinality statistic. What they pay instead
/// is a depth at the ceiling, which is a cost the read's own ending can report rather
/// than a number nobody compares.
#[test]
fn a_small_top_k_over_an_oversized_declaration_plans_admits_and_runs() {
    let stats = statistics(&[]);
    let needle = || vec![lexical("quick brown fox", None)];
    let oversized = u64::from(u32::MAX) + 100;
    let deepest = u32::MAX - 1;
    let top_ten = || RetrievalRequest::bounded(needle(), TopK::new(10));

    // The narrowing shapes: unique candidates over one stratum, so the request's own
    // bound IS the depth and the oversized declaration never binds. The declared
    // block changes nothing here, and that is the claim — with no second stratum
    // there is no pair for the blocks to be disjoint from, so the premise they
    // supply is one the shape already has.
    for spec in [
        deep_registry(oversized, Some("block/deep")),
        deep_registry(oversized, None),
    ] {
        assert_eq!(
            depths_and_limits(&spec, &stats, &top_ten()),
            BTreeMap::from([("deep".to_owned(), (10, 11))]),
            "a licensed prefix narrows the read to the ten rows the answer is for"
        );
    }

    // The two that cannot narrow: a stream whose rows are not its candidates, and
    // two undeclared strata, where `Unrestricted` really is a missing premise
    // because there is a pair for it to be missing about. Both plan, both admit,
    // both compile, and both record the ceiling — with the probe row one past it,
    // which is what keeps the ending observable.
    assert_eq!(
        depths_and_limits(&repeating_deep_registry(oversized), &stats, &top_ten()),
        BTreeMap::from([("deep".to_owned(), (deepest, u32::MAX))]),
        "a repeating stratum reads to the ceiling rather than being refused"
    );
    assert_eq!(
        depths_and_limits(&two_deep_registry(oversized), &stats, &top_ten()),
        BTreeMap::from([
            ("deep".to_owned(), (deepest, u32::MAX)),
            ("wide".to_owned(), (deepest, u32::MAX)),
        ]),
        "and two strata that declare no block keep the declaration, at the ceiling"
    );

    // And it executes. The relation behind these fixtures holds two rows, so the
    // read returns two into a bound of `u32::MAX`, the probe slot comes back empty,
    // and the exhaustion is VERIFIED rather than claimed on the strength of the
    // declaration — which is the whole point of recording the ceiling with its probe
    // instead of refusing the plan.
    let registry = repeating_deep_registry(oversized);
    let planned = plan(&top_ten(), &registry, &stats).expect("the oversized declaration plans");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&planned, &env).expect("and it is admitted");
    let execution = block_on(execute(&compiled, &registry, &*common::empty_dataset()))
        .expect("and the unit runs");
    assert_eq!(
        execution.statuses.get(&iri(&ex("stratum/deep"))),
        Some(&ProducerStatus::Exhausted { rows_emitted: 2 }),
        "the read the ceiling allowed was longer than the index, so it ran out — \
         and the empty probe slot is what verifies that"
    );
}

#[test]
fn a_read_bound_no_rank_can_address_is_refused_and_the_addressable_ones_plan() {
    let stats = statistics(&[]);
    let needle = || vec![lexical("quick brown fox", None)];
    let disjoint = Some(("block/notes", "block/titles"));
    // The largest bound a read can be PLANNED for, which is one shallower than the
    // largest a rank can express: where the declarations license it the bound is a
    // stratum's depth, and a read is emitted one row deeper than its depth. The
    // refusal's message names this number, and the neighbours below execute both
    // halves of that claim — this bound is served, and the next one up is not.
    let addressable =
        usize::try_from(u64::from(u32::MAX - 1)).expect("a 64-bit host addresses a 32-bit rank");

    // Refused: a fused row count no read can be planned for. This is the
    // configuration the truncation hid in — disjoint blocks and unique candidates,
    // where the bound IS each stratum's depth — so the plan recorded `Bounded(k)`
    // beside a depth below `k`, and nothing anywhere compared the two.
    for requested in [addressable + 1, usize::MAX / 2] {
        let refused = plan(
            &RetrievalRequest::bounded(needle(), TopK::new(requested)),
            &two_stratum_registry(400, disjoint),
            &stats,
        )
        .expect_err("a bound no depth can reach is refused");
        match refused {
            PlanError::ReadBoundBeyondDepthRange {
                requested: named,
                ceiling,
            } => {
                assert_eq!(named, requested, "the refusal names the number asked for");
                assert_eq!(
                    ceiling,
                    u64::from(u32::MAX - 1),
                    "and the ceiling it names is one a read really can be planned for"
                );
            }
            ref other => panic!("expected ReadBoundBeyondDepthRange, got {other:?}"),
        }
    }

    // Both valid neighbours: the largest bound a rank addresses, and an ordinary
    // small one. Neither is refused, and neither moves a depth — the declaration is
    // below the first and the second narrows to itself, exactly as before.
    for (bound, expected) in [
        (
            TopK::new(addressable),
            BTreeMap::from([
                ("notes".to_owned(), (400, 401)),
                ("titles".to_owned(), (400, 401)),
            ]),
        ),
        (
            TopK::new(9),
            BTreeMap::from([
                ("notes".to_owned(), (9, 10)),
                ("titles".to_owned(), (9, 10)),
            ]),
        ),
    ] {
        assert_eq!(
            depths_and_limits(
                &two_stratum_registry(400, disjoint),
                &stats,
                &RetrievalRequest::bounded(needle(), bound),
            ),
            expected,
            "an addressable bound plans: {bound}"
        );
    }

    // The case the message has to be true for: a bound that really BECOMES the depth
    // — an unbounded declaration whose one stratum declares its block, so the
    // request's bound is the whole of what bounds the read. At the named ceiling it
    // is served, end to end, with the probe row one past it. The message would be a
    // lie if this were refused anywhere, so it is executed here rather than reasoned
    // about.
    assert_eq!(
        depths_and_limits(
            &deep_registry(u64::MAX, Some("block/deep")),
            &stats,
            &RetrievalRequest::bounded(needle(), TopK::new(addressable)),
        ),
        BTreeMap::from([("deep".to_owned(), (u32::MAX - 1, u32::MAX))]),
        "the ceiling the refusal names is a bound this layer really plans a read for"
    );

    // And one row past it, in that same configuration, is the refusal above rather
    // than a silently truncated depth — the request's own number, refused where the
    // request is read.
    let refused = plan(
        &RetrievalRequest::bounded(needle(), TopK::new(addressable + 1)),
        &deep_registry(u64::MAX, Some("block/deep")),
        &stats,
    )
    .expect_err("a read one rank past the ceiling leaves no room for the probe row");
    assert!(
        matches!(
            refused,
            PlanError::ReadBoundBeyondDepthRange {
                ceiling,
                ..
            } if ceiling == u64::from(u32::MAX - 1)
        ),
        "expected ReadBoundBeyondDepthRange, got {refused:?}"
    );
}

#[test]
fn the_bound_is_in_the_identity_so_two_bounds_are_two_plans() {
    let stats = statistics(&[]);
    let registry = two_stratum_registry(400, Some(("block/notes", "block/titles")));
    let needle = || vec![lexical("quick brown fox", None)];

    let five = plan(
        &RetrievalRequest::bounded(needle(), TopK::new(5)),
        &registry,
        &stats,
    )
    .expect("plans");
    let five_hundred = plan(
        &RetrievalRequest::bounded(needle(), TopK::new(500)),
        &registry,
        &stats,
    )
    .expect("plans");
    let complete = plan(&RetrievalRequest::complete(needle()), &registry, &stats).expect("plans");

    assert_ne!(
        five.id(),
        five_hundred.id(),
        "two bounds read different numbers of rows, so they are two plans"
    );
    assert_ne!(
        five_hundred.id(),
        complete.id(),
        "and a complete request is a third, even where its depths coincide"
    );

    // The bound survives a canonical round trip, because a depth whose derivation
    // cannot be read back is a number nobody can check.
    let decoded = Plan::from_canonical_bytes(&five.canonical_bytes()).expect("round trips");
    assert_eq!(decoded.read_bound, ReadBound::Bounded(TopK::new(5)));
    assert_eq!(decoded.id(), five.id());
    let decoded = Plan::from_canonical_bytes(&complete.canonical_bytes()).expect("round trips");
    assert_eq!(decoded.read_bound, ReadBound::Complete);
    assert_eq!(decoded.id(), complete.id());
}
