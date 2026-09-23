// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::task::{Context, Poll, Wake, Waker};

use pretty_assertions::assert_eq;
use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, ResourceDimension, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_retrieval::{
    AdmissionEnvironment, BoundMode, CandidateDomains, CompiledRetrieval, DecayRule,
    ExecutionError, Fixed, FusionProfile, IndexGeneration, Iri, ProducerStatus, RankFidelity,
    RankedStream, RankedStreamAdapter, RequestTerm, RetrievalRequest, ScoreExactness, SearchResult,
    ServiceLevel, Statistics, StratumUnit, StreamContract, Term, TopK, UnitError, compile, execute,
    plan, search,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, DuplicatePolicy, EvalError, ExtensionEnv, NativeSparqlEngine,
    PfArgs, PfArity, PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry, QueryGovernors,
    QueryOptions, RankedDeclaration, RequestFacet, TermKind, TermPattern, TermPlacement,
    Volatility,
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

/// What a fixture producer's cursor attests about the index behind it.
///
/// The first three cases are the three a host can actually be in: a relation
/// written before the channel existed and overriding neither method, one that
/// pins the generation it served from, and one that pins a generation **and**
/// says that generation was not whole. The fourth is the defect: an index that
/// moved while one run was still reading it.
#[derive(Clone, Copy)]
enum Attests {
    /// Overrides neither method; the trait defaults answer for it.
    Nothing,
    /// Pins the generation that answered, and declares nothing short.
    Generation(&'static str),
    /// Pins the generation, and declares it was not whole, in its own words.
    Incomplete(&'static str, &'static str),
    /// Pins a **different** generation on every invocation, which is what an
    /// index rebuilt under a running query looks like from the cursor's side:
    /// the relation is the same relation, and the version answering is not the
    /// version that answered a moment ago.
    Moving,
}

/// A ranked producer that emits `count` distinct `(entity, score)` rows.
struct MockProducer {
    arity: PfArity,
    mode: BindingPattern,
    rows: u64,
    emitted: Vec<Vec<TermValue>>,
    attests: Attests,
    /// How many times this producer has been opened — one count per invocation
    /// that entered host code.
    ///
    /// Shared with the test rather than private, because a test about what two
    /// invocations attest is vacuous unless the fixture really drove two. It is
    /// also what [`Attests::Moving`] counts off to name a fresh generation each
    /// time.
    opens: Arc<AtomicU64>,
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
        // The generation is fixed HERE, at the instant the cursor opens, because
        // that is the instant an index-backed relation pins its snapshot — and
        // it is why a moving index is visible at all: a second invocation opens
        // a second cursor and pins whatever is current then.
        let opened = self.opens.fetch_add(1, AtomicOrdering::SeqCst) + 1;
        let generation = match self.attests {
            Attests::Nothing => IndexGeneration::Undeclared,
            Attests::Generation(value) | Attests::Incomplete(value, _) => {
                IndexGeneration::declared(value)
            }
            Attests::Moving => IndexGeneration::declared(format!("gen-{opened}")),
        };
        Ok(Box::new(RowCursor {
            rows: rows.into_iter(),
            generation,
            attests: self.attests,
        }))
    }
}

struct RowCursor {
    rows: std::vec::IntoIter<Vec<TermValue>>,
    /// Read once at `open` and held, so every row this cursor emits is
    /// attributed to the version that was current when it opened.
    generation: IndexGeneration,
    attests: Attests,
}

impl PfCursor for RowCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(self.rows.next())
    }

    fn generation(&self) -> IndexGeneration {
        self.generation.clone()
    }

    fn service_level(&self) -> ServiceLevel {
        match self.attests {
            Attests::Nothing | Attests::Generation(_) | Attests::Moving => ServiceLevel::Undeclared,
            Attests::Incomplete(_, reason) => ServiceLevel::Incomplete {
                reason: reason.to_owned(),
            },
        }
    }
}

fn producer(prefix: &str, count: usize) -> Arc<dyn PropertyFunction> {
    producer_declaring(prefix, count, 100, Attests::Nothing)
}

/// A producer holding `count` rows, declaring `rows` per invocation, attesting
/// `attests`.
///
/// `rows` is the registry's worst-case declaration and `count` is what the mock
/// actually holds; they are separate parameters because the whole depth-probe
/// question is what happens when the two disagree.
fn producer_declaring(
    prefix: &str,
    count: usize,
    rows: u64,
    attests: Attests,
) -> Arc<dyn PropertyFunction> {
    producer_counting(prefix, count, rows, attests, Arc::new(AtomicU64::new(0)))
}

/// The same producer, counting its invocations into `opens` so a test can see
/// how many times the relation was actually entered.
fn producer_counting(
    prefix: &str,
    count: usize,
    rows: u64,
    attests: Attests,
    opens: Arc<AtomicU64>,
) -> Arc<dyn PropertyFunction> {
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
        attests,
        opens,
    })
}

/// One producer per stratum, each holding two rows and attesting what its entry
/// of `attests` says.
fn registry_attesting(attests: [Attests; 2]) -> PropertyFunctionRegistry {
    registry_holding([(2, attests[0]), (2, attests[1])])
}

/// One producer per stratum, each holding the rows its entry names.
fn registry_holding(specs: [(usize, Attests); 2]) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    for (index, (count, attests)) in specs.into_iter().enumerate() {
        let name = ["alpha", "beta"][index];
        registry.register_ranked(
            ex(&format!("pf/{name}")),
            producer_declaring(&format!("{name}/"), count, 100, attests),
            ranked(&ex(STRATA[index])),
        );
    }
    registry
}

/// A registry with ONE stratum, for the depth-probe arms: a producer holding
/// `count` rows and declaring `rows` per invocation.
fn one_stratum_registry(count: usize, rows: u64) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/alpha"),
        producer_declaring("alpha/", count, rows, Attests::Nothing),
        ranked(&ex(STRATA[0])),
    );
    registry
}

/// A ranked declaration for `stratum` that takes an IRI term and writes its
/// value into the producer's second argument.
///
/// The placement is what makes the seed visible in the emitted text, which is
/// how the round-trip test can see that a candidate reached the query as a seed
/// rather than merely being accepted by the planner.
fn ranked(stratum: &str) -> RankedDeclaration {
    declaring(stratum, DuplicatePolicy::Unique)
}

/// The same declaration, stating `duplicates` explicitly.
fn declaring(stratum: &str, duplicates: DuplicatePolicy) -> RankedDeclaration {
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
        duplicates,
        // These fixtures fuse strata that rank the same dataset, so the widest
        // promise is the honest one on both terms; each is exercised where it is
        // the subject, in `fusion.rs`.
        fidelity: RankFidelity::EXACT,
        arithmetic: None,
        domains: CandidateDomains::Unrestricted,
        block_position: None,
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
    statistics_bounding(10)
}

/// Statistics reporting `cardinality` for every fixture stratum.
///
/// The measured cardinality is what narrows a stratum's depth below the
/// registry's declared row bound, which is how these tests choose a depth
/// without ever editing a plan.
fn statistics_bounding(cardinality: u64) -> MockStatistics {
    MockStatistics {
        source: "execute-dataset-statistics".to_owned(),
        revision: "r1".to_owned(),
        cardinalities: STRATA
            .iter()
            .map(|stratum| (iri(&ex(stratum)), cardinality))
            .collect(),
    }
}

fn fixture_profile() -> FusionProfile {
    FusionProfile::with_decay(
        STRATA
            .iter()
            .map(|stratum| (iri(&ex(stratum)), Fixed::ONE))
            .collect(),
        DecayRule::ReciprocalRank { k: K },
    )
    .expect("the fixture profile is valid")
}

fn seed_request() -> RetrievalRequest {
    RetrievalRequest::bounded(
        vec![RequestTerm::EntitySeed {
            entity: Term::new(format!("<{}>", ex("seed"))),
        }],
        TOP_K,
    )
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
    while let Some((rank, term, _block)) = block_on(stream.stream.next()).expect("the stream pulls")
    {
        expected_rank += 1;
        assert_eq!(rank, expected_rank, "ranks are 1-based and contiguous");
        out.push(term.as_str().to_owned());
    }
    out
}

// ---------------------------------------------------------------------------
// 1. The dataset is load-bearing
// ---------------------------------------------------------------------------

/// A call on `producer`, projected under a name nothing else reads and bounded
/// at one row.
///
/// Every hand-written unit below carries one of these. A compiled unit calls
/// exactly one registered relation, and `execute` holds the run's relation
/// witness to that — a unit that returned rows while attesting nothing did not
/// run the text this layer emits, and reporting on it would be reporting about
/// some other query. So a substituted unit keeps the one structural property of
/// a real unit and varies only what this file is actually about: which graph the
/// rest of the pattern reads.
///
/// It contributes exactly one row and binds no variable the unit projects, so it
/// changes neither the candidates nor their order.
fn calling(producer: &str) -> String {
    format!("{{ SELECT (?r0 AS ?probe) WHERE {{ ( ?r0 ) <{producer}> ( ?r1 ) }} LIMIT 1 }}")
}

/// Replace the unit at `index` with one running `query`, a text of this test's own.
///
/// [`StratumUnit::new`] is the only way to hand `execute` a query nobody compiled, and
/// it is what every substitution below goes through. Nothing else about the unit
/// moves: the stratum, the contract, the depth and the declared row bound are the
/// compiler's own, so each test varies exactly the thing it is about.
///
/// The cost of the seam is stated once here rather than at each site. A text this
/// layer did not write is bounded by it only on the outside, so such a read is never
/// certified `ProducerStatus::Exhausted`: it ends
/// `ProducerStatus::SuppliedQueryEnded`, naming the caller's text as the stopper. The
/// rows, the ranks, the refusals and the attestations are unaffected — they are
/// observations rather than completeness claims.
fn running(bundle: &mut CompiledRetrieval, index: usize, query: String) {
    let unit = &bundle.units[index];
    let replacement = StratumUnit::new(
        unit.stratum.clone(),
        query,
        unit.contract.clone(),
        unit.depth(),
        unit.declared_rows(),
    )
    .expect("the compiler's own depth and declaration are admitted");
    bundle.units[index] = replacement;
}

/// The unit that reads the graph: whatever `<mentions>` the fox, in IRI order.
fn mentions_fox() -> String {
    format!(
        "SELECT ?candidate WHERE {{ {} ?candidate <{}> <{}> }} ORDER BY ?candidate",
        calling(&ex("pf/alpha")),
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
    running(&mut bundle, 0, mentions_fox());

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
        ProducerStatus::SuppliedQueryEnded { rank: 2 },
        "the two rows are the caller's query's, and so is whatever bounded it, so the \
         ending names that query rather than certifying an exhaustion"
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
    let follow_up = RetrievalRequest::bounded(
        vec![RequestTerm::EntitySeed {
            entity: candidate.clone(),
        }],
        TOP_K,
    );
    let planned = plan(&follow_up, &registry, &stats).expect("a candidate plans as a seed");
    let bundle = compile(&planned, &env).expect("a candidate is admitted as a seed");
    assert!(
        bundle
            .units
            .iter()
            .all(|unit| unit.sparql().contains(candidate.as_str())),
        "the candidate is written back into the emitted text verbatim: {:?}",
        bundle.units
    );

    let second = block_on(search(
        &follow_up, &registry, &stats, &*dataset, &env, &profile,
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
///
/// Carries `producer`'s call for the reason [`calling`] gives.
fn optional_mentions(producer: &str, topic: &str) -> String {
    format!(
        "SELECT ?candidate WHERE {{ {} OPTIONAL {{ ?candidate <{}> <{}> }} }}",
        calling(producer),
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
    running(
        &mut bundle,
        0,
        optional_mentions(&ex("pf/alpha"), "topic/unicorn"),
    );
    running(
        &mut bundle,
        1,
        optional_mentions(&ex("pf/beta"), "topic/fox"),
    );

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
        ProducerStatus::SuppliedQueryEnded { rank: 1 },
        "the surviving stratum streams its row to the end of the caller's own query, \
         which is the ending this layer can honestly report of a text it did not write"
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
    // A text that is not SPARQL never gets as far as a bundle: the constructor reads
    // what it is handed, so this is refused there rather than isolated here.
    let template = &bundle.units[0];
    match StratumUnit::new(
        template.stratum.clone(),
        "THIS IS NOT SPARQL".to_owned(),
        template.contract.clone(),
        template.depth(),
        template.declared_rows(),
    )
    .expect_err("a text that is not a query is not a unit")
    {
        UnitError::NotAQuery { reason } => assert!(
            reason.contains("expected SELECT, CONSTRUCT, ASK or DESCRIBE"),
            "the parser's own diagnostic reaches the caller: {reason}"
        ),
        other => panic!("expected NotAQuery, got {other:?}"),
    }

    // What is isolated here is a failure of a text that IS a query: it reaches for a
    // user-defined function nothing registered, so the read cannot be performed at all.
    running(
        &mut bundle,
        0,
        format!(
            "SELECT ?candidate WHERE {{ BIND(<{}>(1) AS ?candidate) }}",
            ex("fn/absent")
        ),
    );

    let mut execution =
        block_on(execute(&bundle, &registry, &*dataset_of(&[]))).expect("execution starts");

    match &execution.statuses[&iri(&ex(STRATA[0]))] {
        ProducerStatus::ExecutionFailed { reason } => assert!(
            reason.contains(&ex("fn/absent")),
            "the broken unit's stratum carries its own typed status, naming what it \
             could not reach: {reason}"
        ),
        other => panic!("expected the failing stratum's own status, got {other:?}"),
    }
    assert_eq!(execution.streams.len(), 1, "only the survivor streams");
    assert_eq!(
        candidates(&mut execution, &iri(&ex(STRATA[1]))).len(),
        2,
        "and it streams to completion"
    );
}

/// **The producer's declaration reaches the consumer that was written for it.**
///
/// The duplicate policy a host supplies at `register_ranked` decides what the
/// fusion engine holds and what it refuses, and the fusion engine is three
/// stages downstream of the registry. So the declaration is carried rather than
/// re-fetched, on the same route the pinned plan identity takes: admission reads
/// it off the registry the units are compiled against, the compiled unit carries
/// it, the executed stream is tagged with it, and the exported bridge reports it
/// through the protocol the engine reads.
///
/// This walks that route for both spellings, because a carry that happened to
/// deliver one constant everywhere would look identical to one that delivered
/// nothing.
#[test]
fn a_producers_declared_contract_travels_the_pipeline_to_the_fusion_protocol() {
    for duplicates in [DuplicatePolicy::Unique, DuplicatePolicy::Allowed] {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register_ranked(
            ex("pf/alpha"),
            producer("alpha/", 2),
            declaring(&ex(STRATA[0]), duplicates),
        );
        // The second stratum keeps the other spelling throughout, so a carry
        // that overwrote every stratum with one contract would be caught here
        // rather than agreeing with itself.
        registry.register_ranked(ex("pf/beta"), producer("beta/", 2), ranked(&ex(STRATA[1])));

        let stats = statistics();
        let bundle = compiled(&registry, &stats);
        let expected = StreamContract::new(
            duplicates,
            RankFidelity::EXACT,
            CandidateDomains::Unrestricted,
        );
        let beta = StreamContract::new(
            DuplicatePolicy::Unique,
            RankFidelity::EXACT,
            CandidateDomains::Unrestricted,
        );

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
                expected.clone()
            } else {
                beta.clone()
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

// ---------------------------------------------------------------------------
// 6. What the index attested travels with the rows
// ---------------------------------------------------------------------------

/// Plan, compile and execute `registry` against `stats` over an empty dataset.
fn run(
    registry: &PropertyFunctionRegistry,
    stats: &MockStatistics,
) -> purrdf_retrieval::ExecutionResult {
    let planned = plan(&seed_request(), registry, stats).expect("the fixture request plans");
    let env = AdmissionEnvironment {
        registry,
        statistics: stats,
        fusion_profile: None,
    };
    let bundle = compile(&planned, &env).expect("the fresh plan is admitted");
    block_on(execute(&bundle, registry, &*dataset_of(&[]))).expect("the units run")
}

/// The one-call path over `registry` and `stats`, under the fixture profile.
fn searched(registry: &PropertyFunctionRegistry, stats: &MockStatistics) -> SearchResult {
    let env = AdmissionEnvironment {
        registry,
        statistics: stats,
        fusion_profile: None,
    };
    block_on(search(
        &seed_request(),
        registry,
        stats,
        &*dataset_of(&[]),
        &env,
        &fixture_profile(),
    ))
    .expect("the fixture search answers")
}

/// What one stratum's executed stream attested.
fn attested(
    execution: &purrdf_retrieval::ExecutionResult,
    stratum: &Iri,
) -> purrdf_retrieval::PfAttestation {
    execution
        .streams
        .iter()
        .find(|stream| &stream.stratum == stratum)
        .unwrap_or_else(|| panic!("stratum {stratum} streamed"))
        .attestation
        .clone()
}

/// **T4.1 — the generation a relation pinned reaches both paths.**
///
/// A generation is the one fact that can change a query's answer while every
/// input this layer can see stays identical: the dataset snapshot, the query
/// text and the registry fingerprint are all unchanged by an index rebuild. It
/// is known only to the relation's cursor, so it has to travel — off the
/// governed receipt of the run that produced the rows, onto the stream, through
/// the bridge, into the trailer.
#[test]
fn a_pinned_generation_reaches_the_stream_and_the_fused_trailer() {
    let registry = registry_attesting([Attests::Generation("gen-7"), Attests::Nothing]);
    let stats = statistics();
    let alpha = iri(&ex(STRATA[0]));
    let beta = iri(&ex(STRATA[1]));

    let execution = run(&registry, &stats);
    assert_eq!(
        attested(&execution, &alpha).generation,
        IndexGeneration::declared("gen-7"),
        "the executed stream carries the generation its own run attested"
    );
    assert_eq!(
        attested(&execution, &beta).generation,
        IndexGeneration::Undeclared,
        "and the relation that declared nothing is recorded as having declared \
         nothing, never as having certified its index"
    );

    let result = searched(&registry, &stats);
    assert_eq!(
        result.trailer.attestations[&alpha].generation,
        IndexGeneration::declared("gen-7"),
        "and the one-call path reports it under the stratum's own key"
    );
    assert_eq!(
        result.trailer.attestations[&beta].generation,
        IndexGeneration::Undeclared
    );
    assert_eq!(
        result.evidence_id, result.trailer.evidence_id,
        "the answer's evidence identity is the one its trailer carries, read \
         back rather than recomputed beside it"
    );
    assert_eq!(
        result.trailer.exactness,
        ScoreExactness::Exact,
        "nothing declared itself short, so the scores are exact — which is the \
         narrower true claim, not a certificate that either index was whole"
    );
}

/// **T4.2 — a short index is reported on a different axis from the read's end.**
///
/// "My index was missing a shard" and "my rows ran out" answer different
/// questions, and this stratum says both at once: the read ended by exhaustion
/// and the thing it exhausted was not whole. Collapsing them would lose whichever
/// one the other overwrote.
#[test]
fn an_incomplete_index_is_carried_beside_an_exhausted_read() {
    let registry = registry_holding([
        (1, Attests::Incomplete("gen-7", "rebuilding")),
        (2, Attests::Nothing),
    ]);
    let stats = statistics();
    let alpha = iri(&ex(STRATA[0]));
    let beta = iri(&ex(STRATA[1]));

    let execution = run(&registry, &stats);
    assert_eq!(
        attested(&execution, &alpha).service,
        ServiceLevel::Incomplete {
            reason: "rebuilding".to_owned(),
        },
        "the relation's own words, verbatim: that is what tells an operator \
         which index to rebuild"
    );
    assert_eq!(
        execution.statuses[&alpha],
        ProducerStatus::Exhausted { rows_emitted: 1 },
        "the index was short; the READ still ended by running out of rows, and \
         the two facts are on different axes"
    );

    let result = searched(&registry, &stats);
    assert_eq!(
        result.rows.len(),
        3,
        "the rows the short index DID serve are fused beside the other \
         stratum's, not discarded"
    );
    assert_eq!(
        result.trailer.attestations[&alpha].service,
        ServiceLevel::Incomplete {
            reason: "rebuilding".to_owned(),
        }
    );
    assert_eq!(
        result.trailer.exactness,
        ScoreExactness::Estimated {
            deficit: BTreeSet::from([alpha.clone()]),
            inflation: BTreeSet::from([alpha]),
            unbounded: BTreeSet::new(),
        },
        "one stratum served from a short index, so every fused score is an \
         estimate — and the answer names exactly which stratum made it one, on \
         both sides: the rows that shard held are missing (deficit) AND every row \
         behind them moved up a rank and over-contributed (inflation)"
    );

    // The neighbour that must still be reported plainly: a stratum that attested
    // nothing is not swept into the shortfall its sibling declared.
    assert_eq!(
        result.trailer.attestations[&beta].service,
        ServiceLevel::Undeclared
    );
    assert_eq!(
        result.trailer.statuses[&beta],
        ProducerStatus::Exhausted { rows_emitted: 2 },
        "the silent stratum reports its own ordinary exhaustion"
    );
    let ScoreExactness::Estimated {
        ref deficit,
        ref inflation,
        ref unbounded,
    } = result.trailer.exactness
    else {
        panic!("asserted above");
    };
    assert!(
        !deficit.contains(&beta) && !inflation.contains(&beta),
        "and it is not named on either side of the shortfall its sibling declared"
    );
    assert!(
        unbounded.is_empty(),
        "a short index still ranks truly among the rows it kept, so the error \
         stays bounded; only a perturbed ORDER removes the bound"
    );
}

/// **T4.3 — a short index that served nothing is still a short index.**
///
/// Zero rows is the case where the incompleteness is easiest to lose and worst
/// to lose: with nothing on the stream there is no row to hang the fact on, and
/// a layer that reported the stratum as a plain failure would turn "I have a
/// hole and it may be why you got nothing" into "I could not run".
#[test]
fn a_short_index_that_served_no_row_still_reports_its_shortfall() {
    let registry = registry_holding([
        (0, Attests::Incomplete("gen-7", "rebuilding")),
        (2, Attests::Nothing),
    ]);
    let stats = statistics();
    let alpha = iri(&ex(STRATA[0]));

    let execution = run(&registry, &stats);
    assert_eq!(
        execution.statuses[&alpha],
        ProducerStatus::Exhausted { rows_emitted: 0 },
        "the producer was asked and had nothing, which is what exhausted means — \
         never ExecutionFailed, which would claim it could not run at all"
    );
    let attestation = attested(&execution, &alpha);
    assert_eq!(
        attestation.service,
        ServiceLevel::Incomplete {
            reason: "rebuilding".to_owned(),
        },
        "and the hole beneath that emptiness survives having no row to ride on"
    );
    assert_eq!(
        attestation.generation,
        IndexGeneration::declared("gen-7"),
        "including which generation of the index was the short one"
    );
}

// ---------------------------------------------------------------------------
// 7. The depth probe: an exhausted stratum is one that really ran out
// ---------------------------------------------------------------------------

/// **T4.4 — the three arms of the probe.**
///
/// A stratum read at depth three over a producer holding ten rows is not
/// exhausted, and before the probe there was no way for this layer to know that:
/// ten rows cut to three and three rows that were all there ever was arrive
/// identically. The emitted bound is therefore one row deeper, and the arrival of
/// that row — and nothing else about it — decides the ending.
///
/// The third arm is the depth the `min` used to erase: a producer declaring three
/// rows, planned at three. The probe row is emitted there too, because that is the
/// depth at which `Exhausted` would otherwise be a guess. This arm is the honest
/// half of that pair — a producer that declared three and holds three — and it must
/// stay exactly as it was: three rows, no refusal, `Exhausted`. Its lying neighbour
/// is [`an_under_declared_row_bound_is_refused_and_an_honest_one_is_not`].
#[test]
fn the_probe_separates_a_cut_read_from_an_exhausted_one() {
    let alpha = iri(&ex(STRATA[0]));

    // (a) Ten rows held, one hundred declared, three planned: the fourth row
    //     arrives, is dropped, and turns the ending into DepthReached.
    let registry = one_stratum_registry(10, 100);
    let stats = statistics_bounding(3);
    let planned = plan(&seed_request(), &registry, &stats).expect("plans");
    assert_eq!(planned.stratum_depths[&alpha], 3);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let bundle = compile(&planned, &env).expect("admits");
    assert!(
        bundle.units[0].sparql().ends_with("LIMIT 4"),
        "the emitted bound is the depth plus one probe row: {}",
        bundle.units[0].sparql()
    );
    assert_eq!(
        bundle.units[0].depth(),
        3,
        "and the unit records the depth, not the bound it was emitted under"
    );
    let mut execution =
        block_on(execute(&bundle, &registry, &*dataset_of(&[]))).expect("the unit runs");
    assert_eq!(
        candidates(&mut execution, &alpha).len(),
        3,
        "exactly the depth reaches the stream; the probe row is a read, never a value"
    );
    assert_eq!(
        execution.statuses[&alpha],
        ProducerStatus::DepthReached { rank: 3 },
        "the rows did not run out — the depth did"
    );
    let result = searched(&registry, &stats);
    assert_eq!(
        result.trailer.statuses[&alpha],
        ProducerStatus::DepthReached { rank: 3 },
        "and fusion, having checked the rank against the rows it pulled, says so too"
    );
    assert_eq!(
        result.planned_resolution[&alpha].requested_depth, 3,
        "the recorded depth is three; the probe row moves no plan field"
    );

    // (b) The same declaration and the same depth over a producer holding two
    //     rows: the probe never arrives, so the stratum really is exhausted.
    let registry = one_stratum_registry(2, 100);
    let planned = plan(&seed_request(), &registry, &stats).expect("plans");
    assert_eq!(planned.stratum_depths[&alpha], 3);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let bundle = compile(&planned, &env).expect("admits");
    let execution =
        block_on(execute(&bundle, &registry, &*dataset_of(&[]))).expect("the unit runs");
    assert_eq!(
        execution.statuses[&alpha],
        ProducerStatus::Exhausted { rows_emitted: 2 },
        "fewer rows than the depth is the one case where exhaustion was never in \
         doubt, and it must keep saying so"
    );
    assert_eq!(
        searched(&registry, &stats).planned_resolution[&alpha].requested_depth,
        3
    );

    // (c) A producer declaring exactly three rows and holding exactly three,
    //     planned at three: the depth IS the declared bound, and the probe slot
    //     is emitted there too. It comes back empty, which is what turns this
    //     stratum's exhaustion from a claim about the bound into a claim about
    //     the data.
    let registry = one_stratum_registry(3, 3);
    let stats = statistics_bounding(10);
    let planned = plan(&seed_request(), &registry, &stats).expect("plans");
    assert_eq!(
        planned.stratum_depths[&alpha], 3,
        "the declared bound narrows the measured cardinality, not the other way round"
    );
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let bundle = compile(&planned, &env).expect("admits");
    assert!(
        bundle.units[0].sparql().ends_with("LIMIT 4"),
        "the probe slot exists at the declared bound too — erasing it there is the \
         one depth where `Exhausted` would be a guess: {}",
        bundle.units[0].sparql()
    );
    assert_eq!(
        bundle.units[0].depth(),
        3,
        "and only the emitted bound moved: the recorded depth is still three"
    );
    assert_eq!(
        bundle.units[0].declared_rows(),
        Some(3),
        "the unit carries the declaration the probe row will be read against"
    );
    let execution =
        block_on(execute(&bundle, &registry, &*dataset_of(&[]))).expect("the unit runs");
    assert_eq!(
        execution.statuses[&alpha],
        ProducerStatus::Exhausted { rows_emitted: 3 },
        "an honest producer loses nothing to the probe: three rows, no refusal, and \
         an exhaustion now verified by the empty slot"
    );
    assert_eq!(
        searched(&registry, &stats).planned_resolution[&alpha].requested_depth,
        3
    );
}

/// The hazard arm of case (c): a producer that **under-declares** its row bound.
///
/// Ten rows held, three declared. The planner takes the declaration for the
/// stratum's depth, so the plan lands at exactly three — the depth at which the
/// probe row used to be erased, and therefore the depth at which this read was
/// reported [`ProducerStatus::Exhausted`] for three of its ten rows, with nothing
/// anywhere saying so. That is the one ending that names no stopper, minted for
/// a read a wrong declaration truncated.
///
/// With the probe slot present the fourth row arrives, and it cannot be the
/// ordinary `DepthReached`: the producer promised there is no fourth. It is the
/// producer contradicting the registry, so the whole run is refused by name.
///
/// The neighbouring valid case — the same depth over a producer whose declaration
/// is true — is arm (c) of
/// [`the_probe_separates_a_cut_read_from_an_exhausted_one`], and it is asserted
/// again here so the refusal and its non-refusal live in one place.
#[test]
fn an_under_declared_row_bound_is_refused_and_an_honest_one_is_not() {
    let alpha = iri(&ex(STRATA[0]));
    let stats = statistics_bounding(3);

    // The liar: ten rows behind a declaration of three.
    let registry = one_stratum_registry(10, 3);
    let planned = plan(&seed_request(), &registry, &stats).expect("plans");
    assert_eq!(
        planned.stratum_depths[&alpha], 3,
        "the plan sits at the declared bound, which is the hazard depth"
    );
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let bundle = compile(&planned, &env).expect("a depth at the bound is admitted, not refused");
    assert!(
        bundle.units[0].sparql().ends_with("LIMIT 4"),
        "the read reaches for the row the declaration ruled out: {}",
        bundle.units[0].sparql()
    );
    assert_eq!(
        bundle.units[0].depth(),
        3,
        "the recorded depth did not move with the emitted bound"
    );
    let error = block_on(execute(&bundle, &registry, &*dataset_of(&[])))
        .expect_err("a fourth row from a producer that declared three is refused");
    match &error {
        ExecutionError::RowBoundBreached {
            stratum,
            declared,
            pulled,
            mode,
        } => {
            assert_eq!(
                stratum.as_str(),
                alpha.as_str(),
                "the refusal names the stratum"
            );
            assert_eq!(*declared, 3, "the promise the host has to go and fix");
            assert_eq!(*pulled, 4, "and the evidence that it is false");
            assert_eq!(
                *mode,
                BoundMode::Subsuming {
                    declared: BindingPattern::from_code("ff"),
                    invoked: BindingPattern::from_code("fb"),
                },
                "this fixture declares the all-free mode alone and serves the needle \
                 bound through it, which is the shape the reference relation has — so \
                 the promise was read at `ff` and the refusal says which call it \
                 served: {mode:?}"
            );
        }
        other => panic!("the breach is refused by name, not reported as a status or as {other:?}"),
    }
    let message = error.to_string();
    assert!(
        message.contains(
            "at most 3 rows per invocation under mode `ff`, which serves this call \
             under mode `fb`, and the read returned 4"
        ),
        "both numbers and the declared mode they were read at reach a host that only \
         reads the message: {message}"
    );

    // The neighbour that must stay green: the same depth, the same declaration,
    // over a producer whose declaration is true.
    let honest = one_stratum_registry(3, 3);
    let planned = plan(&seed_request(), &honest, &stats).expect("plans");
    assert_eq!(planned.stratum_depths[&alpha], 3);
    let env = AdmissionEnvironment {
        registry: &honest,
        statistics: &stats,
        fusion_profile: None,
    };
    let bundle = compile(&planned, &env).expect("admits");
    let execution =
        block_on(execute(&bundle, &honest, &*dataset_of(&[]))).expect("an honest producer runs");
    assert_eq!(
        execution.statuses[&alpha],
        ProducerStatus::Exhausted { rows_emitted: 3 },
        "the refusal costs the honest producer nothing: no extra row, no refusal, \
         and the same exhaustion it always reported"
    );
}

// ---------------------------------------------------------------------------
// 8. The witness rule, through the real `execute` path
// ---------------------------------------------------------------------------

/// A unit whose relation is driven by the **data**: one invocation per matching
/// triple, all inside one run of one unit.
///
/// A compiled unit passes its producer constants, so it enters the relation once;
/// that is exactly why a conforming run attests exactly one generation. This unit
/// is the same shape with a variable in the candidate position, so the evaluator
/// drives the relation once per driving row — the ordinary lane an under-bound
/// producer is entered through, and the only lane on which a snapshot can move
/// between two of one run's invocations.
///
/// The data pattern is written first and is scheduled first regardless: the
/// feasibility pass orders data atoms ahead of calls, because a data atom can
/// only ever add bindings.
fn driven_by_the_data(producer: &str) -> String {
    format!(
        "SELECT ?candidate WHERE {{ ?candidate <{}> <{}> . ( ?candidate ) <{producer}> ( ?score ) }} \
         ORDER BY ?candidate",
        ex("mentions"),
        ex("topic/fox")
    )
}

/// The two documents [`driven_by_the_data`] matches, so the relation is entered
/// twice.
fn two_driving_rows() -> Arc<RdfDataset> {
    dataset_of(&[
        (&ex("doc/alpha"), &ex("mentions"), &ex("topic/fox")),
        (&ex("doc/beta"), &ex("mentions"), &ex("topic/fox")),
    ])
}

/// A two-stratum registry whose alpha producer holds ONE row, attests `attests`,
/// and counts every invocation it serves.
fn registry_counting_opens(attests: Attests) -> (PropertyFunctionRegistry, Arc<AtomicU64>) {
    let opens = Arc::new(AtomicU64::new(0));
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/alpha"),
        producer_counting("alpha/", 1, 100, attests, Arc::clone(&opens)),
        ranked(&ex(STRATA[0])),
    );
    registry.register_ranked(ex("pf/beta"), producer("beta/", 1), ranked(&ex(STRATA[1])));
    (registry, opens)
}

/// **T4.5 — an index that moved under one run refuses the RUN, not a stratum.**
///
/// The helper-level tests beside `sole_attestation` show what the rule says about
/// a hand-built witness. This drives the wiring: a real registry, a real prepared
/// query, the real governed receipt, and the `map_err` in `execute` that turns a
/// witness no compiled unit could have produced into
/// [`ExecutionError::InconsistentWitness`]. Without this the rule could have held
/// perfectly while nothing ever consulted it.
///
/// The valid neighbour is the same unit, driven the same number of times, over a
/// relation whose index did **not** move — which must answer, or the refusal
/// above would be a refusal of the multi-invocation shape rather than of the
/// moving snapshot.
#[test]
fn a_relation_whose_index_moved_mid_run_refuses_the_whole_run() {
    let stats = statistics();
    let alpha = iri(&ex(STRATA[0]));
    let dataset = two_driving_rows();

    // The defect: two invocations of one relation, two generations.
    let (registry, opens) = registry_counting_opens(Attests::Moving);
    let mut bundle = compiled(&registry, &stats);
    running(&mut bundle, 0, driven_by_the_data(&ex("pf/alpha")));
    let error = block_on(execute(&bundle, &registry, &*dataset))
        .expect_err("a snapshot that moved mid-run invalidates the run");
    match error {
        ExecutionError::InconsistentWitness { stratum, reason } => {
            assert_eq!(
                *stratum, alpha,
                "the refusal names the stratum whose unit exposed it"
            );
            assert!(
                reason.contains("2 distinct index generations"),
                "the refusal names the count that was wrong: {reason}"
            );
            assert!(
                reason.contains(&ex("pf/alpha")),
                "and the relation whose index moved: {reason}"
            );
        }
        other => panic!("a moved index is a whole-run refusal, got {other:?}"),
    }
    assert_eq!(
        opens.load(AtomicOrdering::SeqCst),
        2,
        "the fixture really did enter the relation twice; one invocation could \
         only ever pin one generation, so a rule tested against it would be vacuous"
    );

    // The neighbour that must still answer: the same unit, the same two
    // invocations, one unchanged index.
    let (registry, opens) = registry_counting_opens(Attests::Generation("gen-7"));
    let mut bundle = compiled(&registry, &stats);
    running(&mut bundle, 0, driven_by_the_data(&ex("pf/alpha")));
    let mut execution =
        block_on(execute(&bundle, &registry, &*dataset)).expect("one index is one generation");
    assert_eq!(
        opens.load(AtomicOrdering::SeqCst),
        2,
        "the valid case is driven exactly as hard as the refused one"
    );
    assert_eq!(
        attested(&execution, &alpha).generation,
        IndexGeneration::declared("gen-7"),
        "seven invocations or two, an index that did not move attests once"
    );
    assert_eq!(
        candidates(&mut execution, &alpha),
        vec![
            format!("<{}>", ex("doc/alpha")),
            format!("<{}>", ex("doc/beta")),
        ],
        "and the rows both invocations produced are on the stream, in rank order"
    );
    assert_eq!(
        execution.statuses[&alpha],
        ProducerStatus::SuppliedQueryEnded { rank: 2 },
        "an attestation is an observation and survives a caller's own query; the \
         completeness claim is what does not"
    );
}

/// **T4.6 — the one ceiling the unbounded lane keeps has no charge site a unit
/// can reach.**
///
/// `execute` runs every unit under [`QueryGovernors::UNBOUNDED`] for the
/// receipt's sake, and then handles `GovernedOutcome::BudgetExhausted` by
/// refusing the run. That arm is not removable — the evaluator's outcome enum is
/// deliberately exhaustive over exactly "complete" and "budget exhausted", so the
/// compiler requires the case to be handled and the only question is what it does
/// — but it cannot fire on this lane today, and a claim like that is worth
/// executing rather than asserting in prose.
///
/// It rests on two facts, one measured here directly and one demonstrated:
///
/// * `UNBOUNDED` engages no caller-settable ceiling and carries no stop signal.
///   Exactly one dimension keeps a ceiling — the evaluator's own recursion guard
///   on user-defined function depth, which is a build constant no caller can
///   raise or lower.
/// * A compiled unit cannot charge that dimension, because reaching it means
///   entering a user-defined function, and `execute` injects no user-function
///   registry: the call resolves to nothing and the unit fails as its own
///   stratum, which is the per-stratum report a malformed unit has always had.
#[test]
fn the_unbounded_lane_keeps_one_ceiling_and_a_unit_cannot_charge_it() {
    let engaged: Vec<ResourceDimension> = ResourceDimension::ALL
        .into_iter()
        .filter(|dimension| QueryGovernors::UNBOUNDED.is_engaged_in(*dimension))
        .collect();
    assert_eq!(
        engaged,
        vec![ResourceDimension::UdfDepth],
        "one dimension keeps a ceiling under the unbounded lane, and it is the \
         recursion guard"
    );
    assert!(
        !QueryGovernors::UNBOUNDED.is_engaged(),
        "no caller-settable governor is engaged, so no other charge site enforces \
         anything"
    );
    assert!(
        QueryGovernors::UNBOUNDED.stop_signal().is_none(),
        "and there is no stop signal to latch a trip of its own"
    );

    // The demonstration: a unit that tries to reach a user-defined function.
    // There is no registry for it to resolve against, so the unit fails — as its
    // own stratum, with the whole run still answering for the other one.
    let registry = fixture_registry();
    let stats = statistics();
    let mut bundle = compiled(&registry, &stats);
    running(
        &mut bundle,
        0,
        format!(
            "SELECT ?candidate WHERE {{ {} BIND(<{}>(1) AS ?candidate) }}",
            calling(&ex("pf/alpha")),
            ex("fn/deepen")
        ),
    );
    let mut execution = block_on(execute(&bundle, &registry, &*dataset_of(&[])))
        .expect("an unreachable function is a stratum's failure, never a budget trip");
    match &execution.statuses[&iri(&ex(STRATA[0]))] {
        ProducerStatus::ExecutionFailed { reason } => assert!(
            reason.contains(&ex("fn/deepen")),
            "the failure names the function nothing registered, which is what makes \
             this the function path rather than a malformed unit: {reason}"
        ),
        other => panic!(
            "a unit that reached for a function it could not have carries its own \
             typed status, got {other:?}"
        ),
    }
    assert_eq!(
        candidates(&mut execution, &iri(&ex(STRATA[1]))).len(),
        2,
        "and the sibling stratum answered, because nothing whole-run happened"
    );
}

// ---------------------------------------------------------------------------
// 10. The bundle above the unit decides whose read the evidence describes
// ---------------------------------------------------------------------------

/// Re-tagging, swapping or dropping a unit AFTER the bundle was assembled is refused
/// by name, and a bundle a caller assembles for itself still runs.
///
/// A unit's numbers and its query were sealed one at a time, each because a writable
/// value there decided what a run could claim. The tag *above* them was the one left
/// open: `CompiledRetrieval::units` is a public `Vec` and a unit's stratum is a public
/// field, and every later step of a run is keyed by that stratum — the status map, the
/// tag on the stream, the per-stratum weight a fusion profile applies. So three edits
/// each produced a served answer carrying the real plan identity:
///
/// * renaming one unit's stratum onto its neighbour's ran two units and recorded ONE
///   status, so a producer's evidence was simply gone;
/// * swapping the two strata attached each status to the other producer's read, and the
///   fused answer came back `Exact` over six rows;
/// * removing a unit answered from one stratum fewer, still `Exact`, still under the
///   plan identity of a plan that had two.
///
/// The waist already enforces the same implication one stage earlier — a stratum missing
/// from the compiled set is a producer missing from the emitted text — and it is
/// enforced from plan into `compile` and not from `compile` into `execute`. This is that
/// second half.
///
/// The seam it must not close is executed first and last: a bundle assembled by a
/// caller, and a unit whose *query* a caller substituted, both run. Only a bundle that
/// changed after assembly is refused.
#[test]
fn a_bundle_retagged_after_assembly_is_refused_and_one_assembled_by_hand_is_not() {
    let registry = fixture_registry();
    let stats = statistics();
    let alpha = iri(&ex(STRATA[0]));
    let beta = iri(&ex(STRATA[1]));

    // The seam, first: a bundle assembled out of the compiler's own units runs exactly
    // as the compiler's own bundle does. Recording the attribution is not a lock on the
    // door.
    let source = compiled(&registry, &stats);
    let by_hand = CompiledRetrieval::new(
        source.units.clone(),
        source.plan_id,
        source.registry_id,
        source.registry_fingerprint.clone(),
        source.fused_bound,
        source.resolution,
    );
    let execution = block_on(execute(&by_hand, &registry, &*dataset_of(&[])))
        .expect("a bundle a caller assembled runs");
    assert_eq!(
        execution.statuses.len(),
        2,
        "both producers answered, each under its own stratum"
    );

    // (1) One unit's stratum renamed onto its neighbour's. Two units run, and without
    //     the check they wrote one status between them.
    let mut renamed = compiled(&registry, &stats);
    renamed.units[1].stratum = alpha.clone();
    let error = block_on(execute(&renamed, &registry, &*dataset_of(&[])))
        .expect_err("a unit reporting under another producer's stratum");
    match error {
        ExecutionError::UnitsNotAsAssembled { plan, reason } => {
            assert_eq!(plan, renamed.plan_id, "the refusal names the bundle's plan");
            assert!(
                reason.contains(alpha.as_str()) && reason.contains(beta.as_str()),
                "and says which tag moved where: {reason}"
            );
        }
        other => panic!("expected UnitsNotAsAssembled, got {other:?}"),
    }

    // (2) The two strata swapped. Both units still run and both statuses are still
    //     written — each onto the other producer's read, which is why a count of
    //     statuses could never have caught this.
    let mut swapped = compiled(&registry, &stats);
    swapped.units[0].stratum = beta;
    swapped.units[1].stratum = alpha;
    assert!(
        matches!(
            block_on(execute(&swapped, &registry, &*dataset_of(&[]))),
            Err(ExecutionError::UnitsNotAsAssembled { .. })
        ),
        "a swap crosses two producers' evidence and is refused"
    );

    // (3) A unit removed. The narrowing the waist exists to prevent, one stage later.
    let mut dropped = compiled(&registry, &stats);
    dropped.units.remove(1);
    match block_on(execute(&dropped, &registry, &*dataset_of(&[]))) {
        Err(ExecutionError::UnitsNotAsAssembled { reason, .. }) => assert!(
            reason.contains("assembled with 2 unit(s) and holds 1"),
            "the count is reported before any position, because a removal shifts every \
             position after it: {reason}"
        ),
        other => panic!("expected UnitsNotAsAssembled, got {other:?}"),
    }

    // (4) The promise a stream is held to is part of the attribution: a duplicate
    //     policy rewritten after assembly is the same class of edit, and fusion reads
    //     that policy before it pulls a row.
    let mut relaxed = compiled(&registry, &stats);
    relaxed.units[0].contract = StreamContract {
        duplicates: DuplicatePolicy::Allowed,
        // Carried over rather than restated: this test rewrites the DUPLICATE
        // policy and nothing else, so a fidelity typed in here would be a second
        // edit the assertion below could not tell apart from the first.
        fidelity: relaxed.units[0].contract.fidelity.clone(),
        domains: relaxed.units[0].contract.domains.clone(),
    };
    assert!(
        matches!(
            block_on(execute(&relaxed, &registry, &*dataset_of(&[]))),
            Err(ExecutionError::UnitsNotAsAssembled { .. })
        ),
        "a rewritten declaration is not the declaration the producer made"
    );

    // And the seam once more, at the end: substituting a unit's QUERY is what
    // `StratumUnit::new` is for, and it still runs. The attribution deliberately
    // records neither the text nor the numbers, so the one thing refused above is the
    // one thing that was wrong.
    let mut supplied = compiled(&registry, &stats);
    running(&mut supplied, 0, mentions_fox());
    block_on(execute(&supplied, &registry, &*dataset_of(&[])))
        .expect("a caller's own text in a compiled unit still runs");
}

// ---------------------------------------------------------------------------
// 11. A caller's dataset clause decides which graphs the caller's query reads
// ---------------------------------------------------------------------------

/// The named graph the cases below address, holding rows the store's default graph
/// does not.
fn graph() -> String {
    ex("g")
}

/// A dataset holding one matching triple in the default graph and two more in
/// `<http://example.org/g>`.
///
/// The shape is what makes a dataset clause **observable**: every `FROM` spelling
/// below selects a different set of graphs, and each set has a different answer, so a
/// clause that was quietly dropped cannot pass as a clause that was honoured. Two rows
/// in the named graph rather than one, because a case further down writes a `LIMIT` of
/// its own and a bound of one is invisible against a single row.
fn two_graphs() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let mentions = builder.intern_iri(&ex("mentions"));
    let fox = builder.intern_iri(&ex("topic/fox"));
    let plain = builder.intern_iri(&ex("doc/plain"));
    let first = builder.intern_iri(&ex("doc/graphed-1"));
    let second = builder.intern_iri(&ex("doc/graphed-2"));
    let named = builder.intern_iri(&graph());
    builder.push_quad(plain, mentions, fox, None);
    builder.push_quad(first, mentions, fox, Some(named));
    builder.push_quad(second, mentions, fox, Some(named));
    builder
        .freeze()
        .expect("the fixture dataset is structurally valid")
}

/// A caller's query: whatever `<mentions>` the fox, in IRI order, read under the
/// dataset `clause` names and matched by `pattern`.
///
/// `clause` is written where a WHOLE query writes one — between the projection and the
/// `WHERE` — because that is the only place the grammar has for it, and the whole point
/// of the cases below is that this text is a whole query when the caller writes it and
/// a sub-`SELECT` after the layer wraps it.
fn reading(clause: &str, pattern: &str) -> String {
    format!(
        "SELECT ?candidate {clause}WHERE {{ {} {pattern} }} ORDER BY ?candidate",
        calling(&ex("pf/alpha"))
    )
}

/// The pattern that reads the active default graph, whatever the clause made that.
fn in_the_default_graph() -> String {
    format!("?candidate <{}> <{}>", ex("mentions"), ex("topic/fox"))
}

/// The same pattern, addressed through `GRAPH ?g`, which only a `FROM NAMED` graph
/// answers.
fn in_an_addressable_graph() -> String {
    format!("GRAPH ?g {{ {} }}", in_the_default_graph())
}

/// One of [`two_graphs`]'s documents, spelled as a candidate column reads it back.
fn doc(name: &str) -> String {
    format!("<{}>", ex(&format!("doc/{name}")))
}

/// The candidate column of a solution sequence, spelled as [`candidates`] spells it.
///
/// Every fixture here binds that column to an IRI, and anything else is this test's own
/// mistake rather than an answer to interpret — so it panics instead of coercing.
fn candidate_column(rows: &[Vec<Option<TermValue>>]) -> Vec<String> {
    rows.iter()
        .map(|row| match row.first() {
            Some(Some(TermValue::Iri(value))) => format!("<{value}>"),
            other => panic!("the candidate column of these fixtures is an IRI: {other:?}"),
        })
        .collect()
}

/// The caller's text run by the evaluator as the WHOLE query it is, over `held`.
///
/// This is the oracle the wrapped read is held to. It is the answer the caller asked
/// for — the text as written, parsed as a whole query, with its dataset clause in the
/// position the grammar puts one — and the whole claim the wrap makes is that putting
/// that text inside a bounded sub-`SELECT` does not change what it means.
fn whole_query(
    sparql: &str,
    registry: &PropertyFunctionRegistry,
    held: &RdfDataset,
) -> Vec<String> {
    let engine = NativeSparqlEngine::new();
    let outcome = engine
        .query_with_options_view(
            held,
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
        .expect("the caller's own text evaluates as a whole query");
    match outcome {
        SparqlResult::Solutions { rows, .. } => candidate_column(&rows),
        other => panic!("expected a SELECT solution sequence, got {other:?}"),
    }
}

/// The same text run the way a unit runs it: wrapped, bounded, through `execute`.
fn wrapped_query(
    sparql: &str,
    registry: &PropertyFunctionRegistry,
    stats: &MockStatistics,
    held: &RdfDataset,
) -> (Vec<String>, ProducerStatus) {
    let mut bundle = compiled(registry, stats);
    running(&mut bundle, 0, sparql.to_owned());
    let alpha = iri(&ex(STRATA[0]));
    let mut execution = block_on(execute(&bundle, registry, held)).expect("the units run");
    let rows = candidates(&mut execution, &alpha);
    (rows, execution.statuses[&alpha].clone())
}

/// **A caller's `FROM` / `FROM NAMED` decides the wrapped read's dataset, and decides
/// it the same way it decides the whole query's.**
///
/// The wrap is a sub-`SELECT`, and `SubSelect ::= SelectClause WhereClause
/// SolutionModifier ValuesClause` has no `DatasetClause` in it. The clause used to be
/// carried inside that wrapper, where the parser read it and then dropped it, and this
/// is the defect that made that unacceptable rather than merely untidy: a caller writing
/// `FROM <a-graph-that-does-not-exist>` got **rows** — read out of the very default
/// graph the clause excluded — under an ordinary ending, with no refusal and no
/// diagnostic anywhere. A wrong answer is worse than a broken one.
///
/// Every spelling is executed twice over the same data: once as the whole query the
/// caller wrote, and once wrapped. The pair is the assertion. Asserting only the wrapped
/// row counts would have passed against the defect, because the defect returned rows;
/// only the comparison with the text's own meaning can see it. And the fixture is built
/// so the spellings disagree with each other — a graph with rows the default graph does
/// not have, and a graph with none at all — because over a dataset where every clause
/// selects the same triples, a dropped clause and an honoured one are the same answer.
#[test]
fn a_supplied_dataset_clause_reads_the_graphs_it_names_wrapped_or_whole() {
    let registry = fixture_registry();
    let stats = statistics();
    let held = two_graphs();

    for (shape, text, expected) in [
        // THE CONTROL: no dataset clause at all. The store's own default dataset
        // answers, the emitted text is byte for byte what it always was, and this is
        // the answer every clause below has to differ from for the rest of the case to
        // mean anything.
        (
            "no dataset clause",
            reading("", &in_the_default_graph()),
            vec![doc("plain")],
        ),
        // (1) A graph the dataset does not hold. The active default graph is the merge
        //     of nothing, so the pattern matches nothing. This is the case the defect
        //     answered with the control's row.
        (
            "FROM a graph that does not exist",
            reading(
                &format!("FROM <{}> ", ex("no-such-graph")),
                &in_the_default_graph(),
            ),
            Vec::new(),
        ),
        // (2) A graph the dataset does hold, whose rows are NOT the default graph's.
        //     Both directions are therefore observable at once: the named graph's rows
        //     arrive and the default graph's row does not.
        (
            "FROM a named graph",
            reading(&format!("FROM <{}> ", graph()), &in_the_default_graph()),
            vec![doc("graphed-1"), doc("graphed-2")],
        ),
        // (3) `FROM NAMED` addresses rather than merges, so it takes a `GRAPH` block to
        //     reach and leaves the active default graph empty.
        (
            "FROM NAMED with a GRAPH block",
            reading(
                &format!("FROM NAMED <{}> ", graph()),
                &in_an_addressable_graph(),
            ),
            vec![doc("graphed-1"), doc("graphed-2")],
        ),
        // (4) A prologue AND a dataset clause: both moves in one text, to two different
        //     positions. The prefixed names must still resolve against the caller's own
        //     declarations while the clause still selects the caller's own graphs.
        (
            "PREFIX and FROM together",
            format!(
                "PREFIX p: <{}>\nPREFIX t: <{}>\nSELECT ?candidate FROM <{}> \
                 WHERE {{ {} ?candidate p:mentions t:fox }} ORDER BY ?candidate",
                ex(""),
                ex("topic/"),
                graph(),
                calling(&ex("pf/alpha"))
            ),
            vec![doc("graphed-1"), doc("graphed-2")],
        ),
        // (5) The caller's own bound AND a dataset clause. The bound is why the wrap
        //     exists; the clause is what the wrap was destroying. Both stand, and the
        //     bound is the one that cuts, so the row it keeps says the clause chose the
        //     graph first.
        (
            "FROM with the caller's own LIMIT",
            format!(
                "{} LIMIT 1",
                reading(&format!("FROM <{}> ", graph()), &in_the_default_graph())
            ),
            vec![doc("graphed-1")],
        ),
    ] {
        assert_eq!(
            whole_query(&text, &registry, &held),
            expected,
            "the whole query's own answer, for {shape}"
        );
        let (rows, status) = wrapped_query(&text, &registry, &stats, &held);
        assert_eq!(
            rows, expected,
            "and the wrapped read agrees with it, for {shape}"
        );
        assert_eq!(
            status,
            ProducerStatus::SuppliedQueryEnded {
                rank: u64::try_from(expected.len()).expect("a fixture row count fits"),
            },
            "the ending names the caller's text as the stopper and reports the rank it \
             reached, which for a clause selecting nothing is zero, for {shape}"
        );
    }

    // What the emitted text actually is, for the one shape it matters in: the clause is
    // written on the WRAPPER's own `SELECT`, where it scopes the whole query and
    // therefore the body inside it, and it is cut out of the body — the one edit the
    // wrap makes to a caller's bytes, and the only one.
    let text = reading(&format!("FROM <{}> ", graph()), &in_the_default_graph());
    let mut bundle = compiled(&registry, &stats);
    running(&mut bundle, 0, text.clone());
    let unit = &bundle.units[0];
    assert_eq!(
        unit.sparql(),
        format!(
            "SELECT * FROM <{}> WHERE {{\n  {{ SELECT ?candidate WHERE {{ {} {} }} \
             ORDER BY ?candidate }}\n}}\nLIMIT {}",
            graph(),
            calling(&ex("pf/alpha")),
            in_the_default_graph(),
            unit.depth() + 1
        ),
        "the clause moves onto the wrapper and out of the body; nothing else moves"
    );
    assert_eq!(
        unit.supplied_query(),
        Some(text.as_str()),
        "and the text read back is the whole of what the caller handed over, dataset \
         clause included and in the caller's own position"
    );
}
