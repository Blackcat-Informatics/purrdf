// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The executor's two contracts with its caller: whose data it reads, and what
//! a candidate is spelled as.
//!
//! The load-bearing test here builds a real dataset and requires the answer to
//! change when the data changes — the one claim an executor that searched a
//! graph of its own could never make. The rest read the candidate side of the
//! same seam: a candidate is the canonical term lexical, so it is legible, it
//! round-trips as the seed of a follow-up request, and an unbound projection is
//! a named per-stratum failure rather than a row quietly removed from the
//! ranking. Every refusal is paired with the neighbouring case that must still
//! succeed. Fixtures are `example.org` throughout.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use pretty_assertions::assert_eq;
use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_retrieval::{
    AdmissionEnvironment, CompiledRetrieval, Fixed, FusionProfile, Iri, ProducerStatus,
    RankedStream, RankedStreamAdapter, RequestTerm, RetrievalRequest, Statistics, StreamContract,
    Term, TopK, compile, execute, plan, search,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, DuplicatePolicy, EvalError, PfArgs, PfArity, PfCursor, PfRow,
    PropertyFunction, PropertyFunctionRegistry, RankOrdering, RankedDeclaration, RequestFacet,
    TermKind, TermPattern, TermPlacement, Volatility,
};

const K: u32 = 60;

/// The row bound these fixtures search under. Fused enumeration is top-k by
/// construction, so the bound is stated; it is far above what the mocks yield.
const TOP_K: TopK = TopK::new(1024);

/// The two strata the fixture registry declares, in the IRI order `compile`
/// emits their units in.
const STRATA: [&str; 2] = ["stratum/alpha", "stratum/beta"];

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

/// A dataset holding exactly `triples`, in the default graph.
///
/// Built here rather than taken from the shared empty-dataset helper: these
/// tests are about the data, so the data has to be theirs.
fn dataset_of(triples: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for (subject, predicate, object) in triples {
        let s = builder.intern_iri(subject);
        let p = builder.intern_iri(predicate);
        let o = builder.intern_iri(object);
        builder.push_quad(s, p, o, None);
    }
    builder
        .freeze()
        .expect("the fixture dataset is structurally valid")
}

/// A ranked producer that emits `count` distinct `(entity, score)` rows.
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
        // The engine drops a row that disagrees with a bound position, so a
        // bound input is echoed back rather than overwritten.
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

fn producer(prefix: &str, count: usize) -> Arc<dyn PropertyFunction> {
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
        rows: 100,
        emitted,
    })
}

/// A ranked declaration for `stratum` that takes an IRI term and writes its
/// value into the producer's second argument.
///
/// The placement is what makes the seed visible in the emitted text, which is
/// how the round-trip test can see that a candidate reached the query as a seed
/// rather than merely being accepted by the planner.
fn ranked(stratum: &str) -> RankedDeclaration {
    declaring(
        stratum,
        RankOrdering::StrictlyDescending,
        DuplicatePolicy::Unique,
    )
}

/// The same declaration, stating `ordering` and `duplicates` explicitly.
fn declaring(
    stratum: &str,
    ordering: RankOrdering,
    duplicates: DuplicatePolicy,
) -> RankedDeclaration {
    RankedDeclaration {
        stratum: kernel_iri(stratum),
        accepted_terms: vec![AcceptedTerm {
            pattern: TermPattern::of_kind(TermKind::Iri),
            placements: vec![TermPlacement {
                facet: RequestFacet::Value,
                position: 1,
                datatype: None,
            }],
        }],
        depth_placement: None,
        candidate_position: 0,
        ordering,
        duplicates,
        mandatory: true,
    }
}

/// One producer per stratum, so a compiled bundle carries two independent units.
fn fixture_registry() -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/alpha"),
        producer("alpha/", 2),
        ranked(&ex(STRATA[0])),
    );
    registry.register_ranked(ex("pf/beta"), producer("beta/", 2), ranked(&ex(STRATA[1])));
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

fn statistics() -> MockStatistics {
    MockStatistics {
        source: "execute-dataset-statistics".to_owned(),
        revision: "r1".to_owned(),
        cardinalities: STRATA
            .iter()
            .map(|stratum| (iri(&ex(stratum)), 10))
            .collect(),
    }
}

fn fixture_profile() -> FusionProfile {
    FusionProfile::new(
        STRATA
            .iter()
            .map(|stratum| (iri(&ex(stratum)), Fixed::ONE))
            .collect(),
        K,
    )
    .expect("the fixture profile is valid")
}

fn seed_request() -> RetrievalRequest {
    RetrievalRequest::from_terms(vec![RequestTerm::EntitySeed {
        entity: Term::new(format!("<{}>", ex("seed"))),
    }])
}

/// Plan and compile [`seed_request`] against `registry`: a genuine bundle, with
/// the real plan identity and the live registry instance the executor checks.
fn compiled(registry: &PropertyFunctionRegistry, stats: &MockStatistics) -> CompiledRetrieval {
    let planned = plan(&seed_request(), registry, stats).expect("the fixture request plans");
    let env = AdmissionEnvironment {
        registry,
        statistics: stats,
        fusion_profile: None,
    };
    let bundle = compile(&planned, &env).expect("the fresh plan is admitted");
    assert_eq!(bundle.units.len(), 2, "one unit per declared stratum");
    bundle
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

/// Every candidate the named stratum streamed, in rank order.
fn candidates(execution: &mut purrdf_retrieval::ExecutionResult, stratum: &Iri) -> Vec<String> {
    let stream = execution
        .streams
        .iter_mut()
        .find(|stream| &stream.stratum == stratum)
        .unwrap_or_else(|| panic!("stratum {stratum} streamed"));
    let mut out = Vec::new();
    let mut expected_rank = 0u64;
    while let Some((rank, term)) = block_on(stream.stream.next()).expect("the stream pulls") {
        expected_rank += 1;
        assert_eq!(rank, expected_rank, "ranks are 1-based and contiguous");
        out.push(term.as_str().to_owned());
    }
    out
}

// ---------------------------------------------------------------------------
// 1. The dataset is load-bearing
// ---------------------------------------------------------------------------

/// The unit that reads the graph: whatever `<mentions>` the fox, in IRI order.
fn mentions_fox() -> String {
    format!(
        "SELECT ?candidate WHERE {{ ?candidate <{}> <{}> }} ORDER BY ?candidate",
        ex("mentions"),
        ex("topic/fox")
    )
}

#[test]
fn execute_answers_from_the_callers_dataset() {
    let registry = fixture_registry();
    let stats = statistics();
    let mut bundle = compiled(&registry, &stats);
    // One stratum's unit is replaced with a graph query. The bundle is otherwise
    // exactly what `compile` produced, so the plan identity and the registry
    // instance the executor checks are the real ones.
    bundle.units[0].sparql = mentions_fox();

    let alpha = iri(&ex(STRATA[0]));
    let mentions = ex("mentions");

    // The dataset the caller actually holds.
    let held = dataset_of(&[
        (&ex("doc/alpha"), &mentions, &ex("topic/fox")),
        (&ex("doc/beta"), &mentions, &ex("topic/fox")),
        (&ex("doc/gamma"), &mentions, &ex("topic/turtle")),
    ]);
    let mut execution = block_on(execute(&bundle, &registry, &*held)).expect("the units run");
    assert_eq!(
        candidates(&mut execution, &alpha),
        vec![
            format!("<{}>", ex("doc/alpha")),
            format!("<{}>", ex("doc/beta")),
        ],
        "the rows are the stored triples that match, and only those"
    );
    assert_eq!(
        execution.statuses[&alpha],
        ProducerStatus::Exhausted { rows_emitted: 2 }
    );

    // The same bundle, the same registry, different stored data: the answer
    // follows the data. Nothing but the dataset parameter changed.
    let other = dataset_of(&[
        (&ex("doc/delta"), &mentions, &ex("topic/fox")),
        (&ex("doc/alpha"), &mentions, &ex("topic/turtle")),
    ]);
    let mut execution = block_on(execute(&bundle, &registry, &*other)).expect("the units run");
    assert_eq!(
        candidates(&mut execution, &alpha),
        vec![format!("<{}>", ex("doc/delta"))],
        "a different dataset is a different answer"
    );

    // And the untouched stratum still runs its own compiled unit off the
    // registry, unaffected by either dataset.
    assert_eq!(
        execution.statuses[&iri(&ex(STRATA[1]))],
        ProducerStatus::Exhausted { rows_emitted: 2 }
    );
}

// ---------------------------------------------------------------------------
// 2. A candidate is the canonical term lexical
// ---------------------------------------------------------------------------

#[test]
fn candidates_are_the_canonical_lexical_and_not_an_opaque_blob() {
    let registry = fixture_registry();
    let stats = statistics();
    let bundle = compiled(&registry, &stats);
    let mut execution =
        block_on(execute(&bundle, &registry, &*dataset_of(&[]))).expect("the units run");

    let alpha = candidates(&mut execution, &iri(&ex(STRATA[0])));
    assert_eq!(
        alpha,
        vec![
            format!("<{}>", ex("alpha/entity0")),
            format!("<{}>", ex("alpha/entity1")),
        ],
        "a candidate is the term's own lexical form, readable as written"
    );

    // The negative, stated exactly: not the hex of the value's canonical bytes.
    let hexed = TermValue::iri(ex("alpha/entity0"))
        .to_canonical_bytes()
        .iter()
        .fold(String::new(), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        });
    assert_ne!(alpha[0], hexed, "a candidate is not a hex blob");
}

// ---------------------------------------------------------------------------
// 3. A candidate is a seed
// ---------------------------------------------------------------------------

#[test]
fn a_fused_candidate_round_trips_as_the_seed_of_a_follow_up_request() {
    let registry = fixture_registry();
    let stats = statistics();
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let profile = fixture_profile();
    let dataset = dataset_of(&[]);

    let first = block_on(search(
        &seed_request(),
        &registry,
        &stats,
        &*dataset,
        &env,
        &profile,
        TOP_K,
    ))
    .expect("the fixture search answers");
    let candidate = first
        .rows
        .first()
        .expect("the search returned a row")
        .entity
        .clone();

    // The consequence the executor's rendering buys: a fused row is spelled
    // exactly as a seed is, so it is a request term without any translation.
    let follow_up = RetrievalRequest::from_terms(vec![RequestTerm::EntitySeed {
        entity: candidate.clone(),
    }]);
    let planned = plan(&follow_up, &registry, &stats).expect("a candidate plans as a seed");
    let bundle = compile(&planned, &env).expect("a candidate is admitted as a seed");
    assert!(
        bundle
            .units
            .iter()
            .all(|unit| unit.sparql.contains(candidate.as_str())),
        "the candidate is written back into the emitted text verbatim: {:?}",
        bundle.units
    );

    let second = block_on(search(
        &follow_up, &registry, &stats, &*dataset, &env, &profile, TOP_K,
    ))
    .expect("the follow-up search answers");
    assert!(
        !second.rows.is_empty(),
        "the follow-up request answers rather than refusing its own seed"
    );
}

// ---------------------------------------------------------------------------
// 4. An unbound projection is a named failure, not a dropped row
// ---------------------------------------------------------------------------

/// `OPTIONAL` over a triple the dataset does not hold: one solution, with the
/// projected variable unbound.
fn optional_mentions(topic: &str) -> String {
    format!(
        "SELECT ?candidate WHERE {{ OPTIONAL {{ ?candidate <{}> <{}> }} }}",
        ex("mentions"),
        ex(topic)
    )
}

#[test]
fn an_unbound_projection_fails_its_stratum_while_a_bound_one_streams() {
    let registry = fixture_registry();
    let stats = statistics();
    let mut bundle = compiled(&registry, &stats);
    // Stratum alpha asks for a topic the dataset does not hold, so its single
    // solution leaves `?candidate` unbound. Stratum beta asks for one it does.
    bundle.units[0].sparql = optional_mentions("topic/unicorn");
    bundle.units[1].sparql = optional_mentions("topic/fox");

    let dataset = dataset_of(&[(&ex("doc/alpha"), &ex("mentions"), &ex("topic/fox"))]);
    let mut execution = block_on(execute(&bundle, &registry, &*dataset)).expect("the units run");

    // The refusal: named, typed, and attributed to its own stratum.
    let alpha = iri(&ex(STRATA[0]));
    match &execution.statuses[&alpha] {
        ProducerStatus::ExecutionFailed { reason } => {
            assert!(
                reason.contains(alpha.as_str()),
                "the failure names its stratum: {reason}"
            );
            assert!(
                reason.contains("?candidate") && reason.contains("unbound"),
                "the failure says what was wrong: {reason}"
            );
            assert!(
                reason.contains("row 1"),
                "and which row it was wrong in: {reason}"
            );
        }
        other => panic!("an unbound projection is a typed failure, got {other:?}"),
    }
    assert!(
        !execution
            .streams
            .iter()
            .any(|stream| stream.stratum == alpha),
        "a stratum that could not be read contributes no stream"
    );

    // The neighbouring valid case: a bound projection yields its row, and the
    // failure next door did not stop it.
    let beta = iri(&ex(STRATA[1]));
    assert_eq!(
        candidates(&mut execution, &beta),
        vec![format!("<{}>", ex("doc/alpha"))],
        "a bound projection is a row, not a refusal"
    );
    assert_eq!(
        execution.statuses[&beta],
        ProducerStatus::Exhausted { rows_emitted: 1 },
        "the surviving stratum streams to completion"
    );
}

// ---------------------------------------------------------------------------
// 5. Failure isolation survives, for the executor's other refusals too
// ---------------------------------------------------------------------------

#[test]
fn a_forced_failure_isolates_to_its_stratum() {
    let registry = fixture_registry();
    let stats = statistics();
    let mut bundle = compiled(&registry, &stats);
    bundle.units[0].sparql = "THIS IS NOT SPARQL".to_owned();

    let mut execution =
        block_on(execute(&bundle, &registry, &*dataset_of(&[]))).expect("execution starts");

    assert!(
        matches!(
            execution.statuses[&iri(&ex(STRATA[0]))],
            ProducerStatus::ExecutionFailed { .. }
        ),
        "the broken unit's stratum carries its own typed status"
    );
    assert_eq!(execution.streams.len(), 1, "only the survivor streams");
    assert_eq!(
        candidates(&mut execution, &iri(&ex(STRATA[1]))).len(),
        2,
        "and it streams to completion"
    );
}

/// **The producer's declaration reaches the consumer that was written for it.**
///
/// The rank ordering and duplicate policy a host supplies at `register_ranked`
/// decide what the fusion engine holds and what it refuses, and the fusion
/// engine is three stages downstream of the registry. So the declaration is
/// carried rather than re-fetched, on the same route the pinned plan identity
/// takes: admission reads it off the registry the units are compiled against,
/// the compiled unit carries it, the executed stream is tagged with it, and the
/// exported bridge reports it through the protocol the engine reads.
///
/// This walks that route for both spellings of both halves, because a carry that
/// happened to deliver one constant everywhere would look identical to one that
/// delivered nothing.
#[test]
fn a_producers_declared_contract_travels_the_pipeline_to_the_fusion_protocol() {
    for (ordering, duplicates) in [
        (RankOrdering::StrictlyDescending, DuplicatePolicy::Unique),
        (RankOrdering::NonIncreasing, DuplicatePolicy::Allowed),
    ] {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register_ranked(
            ex("pf/alpha"),
            producer("alpha/", 2),
            declaring(&ex(STRATA[0]), ordering, duplicates),
        );
        // The second stratum keeps the other spelling throughout, so a carry
        // that overwrote every stratum with one contract would be caught here
        // rather than agreeing with itself.
        registry.register_ranked(ex("pf/beta"), producer("beta/", 2), ranked(&ex(STRATA[1])));

        let stats = statistics();
        let bundle = compiled(&registry, &stats);
        let expected = StreamContract::new(duplicates);
        let beta = StreamContract::new(DuplicatePolicy::Unique);

        let unit = bundle
            .units
            .iter()
            .find(|unit| unit.stratum == iri(&ex(STRATA[0])))
            .expect("the alpha stratum compiled a unit");
        assert_eq!(
            unit.contract, expected,
            "the compiled unit carries the declaration the registry it was admitted against made"
        );

        let execution = block_on(execute(&bundle, &registry, &*dataset_of(&[])))
            .expect("the fixture registry executes");
        let profile = fixture_profile();
        for stream in execution.streams {
            let wanted = if stream.stratum == iri(&ex(STRATA[0])) {
                expected
            } else {
                beta
            };
            assert_eq!(
                stream.contract, wanted,
                "the executed stream is tagged with its own producer's declaration"
            );
            let adapter =
                RankedStreamAdapter::new(stream.stream, stream.contract, &profile, &stream.stratum)
                    .expect("the fixture profile weights every stratum the plan reached");
            assert_eq!(
                adapter.contract(),
                wanted,
                "and the bridge reports it through the protocol the fusion engine reads"
            );
        }
    }
}
