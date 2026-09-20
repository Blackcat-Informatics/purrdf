// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Stage boundaries are seams, read in both directions.
//!
//! The ladder is `plan -> compile -> execute -> fuse`. Every boundary is a
//! supported place to **stop**: a caller that wants only the plan calls `plan`
//! and stops; one that wants the query text calls through `compile` and runs it
//! through the evaluator directly; one that wants per-producer ranked streams
//! calls through `execute` and enumerates them with no threshold in the path.
//! Every boundary is
//! equally a place to **start**: a hand-built [`Plan`] is admitted on exactly the
//! terms the planner's own output is, hand-written SPARQL runs through the
//! evaluator with no composition type in the path, and hand-built ranked streams
//! fuse with no planning, compilation or execution involved.
//!
//! The two rungs differ in kind. Fusion is top-k by construction — it certifies
//! a row only when a threshold over the live stream heads proves nothing can
//! overtake it — so its frontier holds the candidates the strata still disagree
//! about, and not the input. The unfused rung carries no cross-stratum
//! accounting: N producers emit in their own rank order, nothing applies a
//! threshold, and each stratum's rows are materialized by the evaluator before
//! the first one is read — so that rung costs what a stratum's own result costs,
//! not what the consumer reads. A completeness claim is available only from the
//! terminal trailer; a prefix reader holds evidence the answer is incomplete.
//!
//! These are tests of the seams, not of the stages' semantics (those live
//! elsewhere): every fixture is `example.org`, every mock is a plain
//! caller-owned value, and no composition layer is constructed where the test
//! says a caller stopped below it.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Wake, Waker};

use pretty_assertions::assert_eq;
use purrdf_core::{RdfDatasetBuilder, SparqlRequest, SparqlResult, TermValue};
use purrdf_retrieval::{
    AdmissionEnvironment, AdmissionError, CandidateDomains, CompiledRetrieval, DecayRule,
    ExecutionError, ExecutionResult, Fixed, FusionError, FusionProfile, FusionResult, FusionStream,
    Iri, PfAttestation, Plan, PlanError, PlanId, PlanOrigin, ProducerBinding, ProducerReceipt,
    ProducerStatus, ProtocolError, RankedRow, RankedStream, RankedStreamAdapter, RankedStreamImpl,
    ReadBound, RequestTerm, RetrievalRequest, RowBlock, ScoreExactness, SearchError, SearchResult,
    Statistics, StatisticsSnapshot, StratumUnit, StreamContract, StreamEnding, Term, TopK,
    UnitError, UnservedReason, UnservedTerm, compile, contribution, execute, fuse, plan, search,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, DomainTag, DuplicatePolicy, EvalError, NativeSparqlEngine,
    PfArgs, PfArity, PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry, QueryOptions,
    RankedDeclaration, RequestFacet, TermKind, TermPattern, TermPlacement, Volatility,
};

mod common;

const K: u32 = 60;

/// The row bound these fixtures fuse under.
///
/// Fused enumeration is top-k by construction, so every fusion states a bound.
/// It is far above what any fixture here yields, so nothing below is decided by
/// the bound — `fused_top_k_holds_a_bounded_frontier_across_disagreeing_strata`
/// states a bound of its own, which is where the bound's own behaviour belongs.
const TOP_K: TopK = TopK::new(1024);

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn ex(suffix: &str) -> String {
    format!("http://example.org/{suffix}")
}

/// A bundle of `units` under the identities `compiled` carries.
///
/// Assembled through [`CompiledRetrieval::new`] rather than written as a literal,
/// because the attribution a bundle is checked against is recorded when it is
/// assembled — a caller handing `execute` a bundle of its own is the seam, and a
/// bundle re-tagged after assembly is the refusal. Starting at `execute` therefore
/// goes through the same constructor `compile` does.
fn bundle_of(units: Vec<StratumUnit>, compiled: &CompiledRetrieval) -> CompiledRetrieval {
    CompiledRetrieval::new(
        units,
        compiled.plan_id,
        compiled.registry_id,
        compiled.registry_fingerprint.clone(),
        compiled.fused_bound,
        compiled.resolution.clone(),
    )
}

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

fn stratum(suffix: &str) -> Iri {
    iri(&ex(&format!("stratum/{suffix}")))
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
fn ranked(stratum_iri: &str, patterns: Vec<TermPattern>, mandatory: bool) -> RankedDeclaration {
    RankedDeclaration {
        stratum: kernel_iri(stratum_iri),
        accepted_terms: accepted(patterns),
        depth_placement: None,
        candidate_position: 0,
        duplicates: DuplicatePolicy::Unique,
        domains: CandidateDomains::Unrestricted,
        block_position: None,
        mandatory,
    }
}

/// The contract every fixture producer here declares, and the one a hand-built
/// stream in this file states: no repeats. It is spelled once so the registered
/// declaration above and the streams below cannot drift into describing two
/// different promises. The contiguous, ascending ranks these streams emit are
/// not part of it — that law holds for every stream and is checked rank by rank
/// rather than declared.
fn unique_items() -> StreamContract {
    StreamContract::new(DuplicatePolicy::Unique, CandidateDomains::Unrestricted)
}

fn lexical_term() -> RequestTerm {
    RequestTerm::Lexical {
        text: "quick brown fox".to_owned(),
        language: Some("en".to_owned()),
        predicate: Some(iri(&ex("body"))),
    }
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

/// A ranked producer emitting `count` distinct `(entity, score)` row pairs.
fn make_producer(rows: u64, prefix: &str, count: usize) -> Arc<dyn PropertyFunction> {
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

/// A single catch-all producer under its own stratum.
fn single_registry(
    stratum_iri: &str,
    producer_iri: &str,
    rows: u64,
    count: usize,
) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        producer_iri,
        make_producer(rows, "hand/", count),
        ranked(stratum_iri, vec![TermPattern::of_kind(TermKind::Any)], true),
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

/// Statistics that bound exactly one stratum at `cardinality`.
fn single_statistics(stratum_iri: &str, cardinality: u64) -> MockStatistics {
    MockStatistics {
        source: "boundary-statistics".to_owned(),
        revision: "r1".to_owned(),
        cardinalities: BTreeMap::from([(iri(stratum_iri), cardinality)]),
    }
}

/// A profile weighting exactly one stratum.
fn single_profile(stratum_iri: &str) -> FusionProfile {
    FusionProfile::with_decay(
        BTreeMap::from([(iri(stratum_iri), Fixed::ONE)]),
        DecayRule::ReciprocalRank { k: K },
    )
    .expect("the fixture profile is valid")
}

/// A profile weighting the named strata, which is also what leaves room for
/// every one of them to contribute to a candidate: the contribution maximum is
/// the stratum count.
fn profile(weights: &[(&str, Fixed)], k: u32) -> FusionProfile {
    let map: BTreeMap<Iri, Fixed> = weights
        .iter()
        .map(|(name, weight)| (stratum(name), *weight))
        .collect();
    FusionProfile::with_decay(map, DecayRule::ReciprocalRank { k })
        .expect("fixture profile is valid")
}

// ---------------------------------------------------------------------------
// A minimal single-threaded executor (the mocks never actually pend)
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

/// Run `sparql` through the evaluator itself, with no composition type in the
/// path: this is exactly what a caller does when it stops at `compile` (or
/// starts at `execute`).
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
        .expect("the fixture query evaluates");
    match outcome {
        SparqlResult::Solutions { rows, .. } => rows,
        other => panic!("expected a SELECT solution sequence, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Hand-built ranked streams, so `fuse` can be entered with no earlier stage
// ---------------------------------------------------------------------------

/// A producer whose rows and terminal receipt are pre-scripted.
struct ScriptedStream {
    steps: VecDeque<RankedRow<Term>>,
    receipt: ProducerReceipt,
    emitted: u64,
}

impl ScriptedStream {
    fn new(steps: Vec<RankedRow<Term>>, receipt: ProducerReceipt) -> Self {
        Self {
            steps: steps.into(),
            receipt,
            emitted: 0,
        }
    }
}

// The trait's methods are `async`; this mock's body is synchronous because its
// rows are pre-scripted. The keyword is required by the trait, not a signal
// that the body awaits.
#[allow(clippy::unused_async_trait_impl)]
impl RankedStream for ScriptedStream {
    type Item = Term;

    async fn next(&mut self) -> Result<Option<RankedRow<Self::Item>>, ProtocolError> {
        match self.steps.pop_front() {
            Some(row) => {
                self.emitted += 1;
                Ok(Some(row))
            }
            None => Ok(None),
        }
    }

    async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError> {
        Ok(self.receipt.clone())
    }

    fn contract(&self) -> StreamContract {
        unique_items()
    }
}

/// A well-formed row at `rank` for `weight` under `k`.
///
/// It names no block, which is exactly what these fixtures' `Unrestricted`
/// declaration owes: an unrestricted producer has made no promise for a row to
/// back. The domain fixtures in `tests/fusion.rs` are where a named block is
/// exercised.
fn row(rank: u64, weight: Fixed, k: u32, item: &str) -> RankedRow<Term> {
    RankedRow::new(
        rank,
        contribution(weight, rank, k).expect("fixture contribution fits"),
        Term::new(item),
        RowBlock::Undeclared,
    )
}

fn exhausted(rows: u64) -> ProducerReceipt {
    ProducerReceipt::Exhausted { rows_emitted: rows }
}

// ---------------------------------------------------------------------------
// 1. Stop at plan: inspect the value
// ---------------------------------------------------------------------------

#[test]
fn stop_at_plan_inspect_value() {
    let registry = single_registry(&ex("stratum/hand"), &ex("pf/hand"), 10, 2);
    let stats = single_statistics(&ex("stratum/hand"), 10);
    let request = RetrievalRequest::complete(vec![lexical_term()]);

    let planned = plan(&request, &registry, &stats).expect("the fixture request plans");

    // The plan is a value: every decision is inspectable and owned. A caller
    // that stops here never reaches compilation.
    assert_eq!(planned.producer_bindings.len(), 1);
    assert_eq!(planned.producer_bindings[0].producer, ex("pf/hand"));
    assert_eq!(
        planned.producer_bindings[0].stratum,
        iri(&ex("stratum/hand"))
    );
    assert_eq!(planned.request_terms.len(), 1);
    assert!(!planned.stratum_depths.is_empty());
    assert_eq!(planned.registry_instance_id, registry.instance_id());
    assert_eq!(
        planned.registry_content_fingerprint,
        registry.content_fingerprint().expect("fingerprint")
    );
    // Deliberately no `compile`, `execute` or `fuse` call.
}

// ---------------------------------------------------------------------------
// 2. Stop at compile: run the emitted SPARQL directly
// ---------------------------------------------------------------------------

#[test]
fn stop_at_compile_run_directly() {
    let registry = single_registry(&ex("stratum/hand"), &ex("pf/hand"), 12, 3);
    let stats = single_statistics(&ex("stratum/hand"), 12);
    let request = RetrievalRequest::complete(vec![lexical_term()]);
    let planned = plan(&request, &registry, &stats).expect("plans");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };

    let compiled = compile(&planned, &env).expect("the plan is admitted");
    assert_eq!(compiled.units.len(), 1, "one unit for one stratum");

    // The compiled form is text a caller can hand to the evaluator directly,
    // with no `execute` (and so no composition layer) in the path.
    let unit = &compiled.units[0];
    assert!(
        unit.sparql().contains(&ex("pf/hand")),
        "the unit names its producer: {}",
        unit.sparql()
    );
    let rows = run_query(&unit.sparql(), &registry);
    assert_eq!(
        rows.len(),
        3,
        "the emitted text is independently executable"
    );
    for (index, row) in rows.iter().enumerate() {
        assert_eq!(
            row[0].as_ref().expect("candidate is bound"),
            &TermValue::iri(format!("{}entity{index}", ex("hand/")))
        );
    }
}

// ---------------------------------------------------------------------------
// 3. Stop at execute: consume unfused streams with no fusion in the path
// ---------------------------------------------------------------------------

#[test]
fn stop_at_execute_consumes_unfused_streams_with_no_fusion_in_the_path() {
    let registry = single_registry(&ex("stratum/hand"), &ex("pf/hand"), 9, 5);
    let stats = single_statistics(&ex("stratum/hand"), 9);
    let request = RetrievalRequest::complete(vec![lexical_term()]);
    let planned = plan(&request, &registry, &stats).expect("plans");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&planned, &env).expect("admits");

    let execution =
        block_on(execute(&compiled, &registry, &*common::empty_dataset())).expect("the units run");
    assert_eq!(execution.streams.len(), 1);

    let mut stream = execution
        .streams
        .into_iter()
        .next()
        .expect("the stratum streamed");

    // Consume the stream to its own end: no fusion, so no threshold and no
    // top-k certification can block an emission.
    let mut pulled = 0usize;
    let mut last_rank = 0u64;
    while let Some((rank, _term, _block)) =
        block_on(stream.stream.next()).expect("the stream pulls")
    {
        assert_eq!(rank, last_rank + 1, "ranks are 1-based and contiguous");
        last_rank = rank;
        pulled += 1;
    }
    assert_eq!(pulled, 5, "every emitted row is consumable");
    let receipt = block_on(stream.stream.receipt()).expect("the stream ended cleanly");
    assert_eq!(receipt, exhausted(5));
}

// ---------------------------------------------------------------------------
// 4. Start at compile: a hand-built plan, admitted on the same terms
// ---------------------------------------------------------------------------

#[test]
fn start_at_compile_hand_built_plan() {
    let registry = single_registry(&ex("stratum/hand"), &ex("pf/hand"), 10, 2);
    let stats = single_statistics(&ex("stratum/hand"), 10);
    let request = RetrievalRequest::complete(vec![lexical_term()]);
    let stratum = iri(&ex("stratum/hand"));

    let mut stratum_depths = HashMap::new();
    stratum_depths.insert(stratum.clone(), 10);

    // Built by hand, never through the planner. It is still a plan, so admission
    // covers it on exactly the same terms as planner origin.
    let hand_built = Plan {
        version: Plan::VERSION,
        request_terms: vec![lexical_term()],
        read_bound: ReadBound::Complete,
        producer_bindings: vec![ProducerBinding {
            producer: ex("pf/hand"),
            stratum,
            // Bound to no term, and that is what this fixture declares: its one
            // accepted pattern is the unconstrained `Any`, which places nothing,
            // so the producer is matched by the request and receives none of it.
            // Naming term 0 here would be the claim that the emitted call
            // carries the needle, and the call this plan compiles to leaves both
            // arguments free.
            request_terms: Vec::new(),
        }],
        producer_decisions: Vec::new(),
        // So the one request term reached a producer that declared nowhere to
        // put it, which is per-term evidence rather than an empty list.
        unserved_terms: vec![UnservedTerm {
            request_term: 0,
            reason: UnservedReason::AcceptedWithoutPlacement,
        }],
        stratum_depths,
        statistics_snapshot: StatisticsSnapshot {
            source: stats.source().to_owned(),
            revision: stats.revision().to_owned(),
            entries: Vec::new(),
        },
        registry_instance_id: registry.instance_id(),
        registry_content_fingerprint: registry.content_fingerprint().expect("fingerprint"),
        // Built here, against the live registry whose instance id it just read,
        // so it is held to that instance exactly as the planner's own output is.
        origin: PlanOrigin::SameProcess,
    };

    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&hand_built, &env).expect("a hand-built plan is admitted");
    assert_eq!(compiled.plan_id, hand_built.id());
    assert_eq!(compiled.units.len(), 1);

    // The planner's own output for the same request compiles to the same units,
    // which is what "covered on the same terms" means.
    let planned = plan(&request, &registry, &stats).expect("planner origin");
    let planned_compiled = compile(&planned, &env).expect("planner origin admits");
    assert_eq!(planned_compiled.units, compiled.units);
    // Same units is the weaker half. The plans agree about what was *served*
    // too, which is the half a caller reads back: binding the term by hand would
    // have compiled to this same text while reporting the needle answered.
    assert_eq!(planned.producer_bindings, hand_built.producer_bindings);
    assert_eq!(planned.unserved_evidence(), hand_built.unserved_evidence());
}

// ---------------------------------------------------------------------------
// 4b. Start at execute: a hand-built bundle, checked on the same dimensions
// ---------------------------------------------------------------------------

/// A query of this test's own over the fixture producer: every candidate it names,
/// projected under the column the executor reads, carrying no bound of any kind.
///
/// It calls the registered relation exactly once because that is what `execute`'s
/// witness rule requires of any unit it is handed — a unit that returned rows while
/// attesting nothing did not run a query over this registry at all — and it carries no
/// bound because the unit's own bound is the layer's to render.
fn hand_written(producer: &str) -> String {
    hand_written_over(&format!("<{producer}>"))
}

/// The same text with the relation named by `predicate` exactly as written — an
/// absolute IRI in angle brackets, a prefixed name, or a relative reference — so a
/// caller's prologue has something in the body to resolve.
fn hand_written_over(predicate: &str) -> String {
    format!(
        "SELECT ?candidate WHERE {{ {{ SELECT (?c0 AS ?candidate) WHERE {{ ( ?c0 ) \
         {predicate} ( ?c1 ) }} }} }}"
    )
}

/// A hand-built [`StratumUnit`] is admitted on the same terms the compiler's own
/// output is, refused on the same terms a hand-edited *plan* is, and certified on
/// weaker terms than either — because its query is a caller's.
///
/// A bundle is what [`execute`] is handed, and **four** things about a unit decide what
/// the run may claim about how its read ended: its depth, its declared row bound, the
/// reach derived from those, and the query the read was actually taken under. The
/// first two were plain public fields, the reach was derived from a flag a caller
/// supplied beside them, and the query was a public `String` — which stayed writable
/// through two attempted fixes.
///
/// While the numbers were plain public fields, a bundle could say anything: a depth
/// raised past the range the emitted bound can probe made `Exhausted` reportable for a
/// read the `LIMIT` cut, and a declared bound lowered below the depth bypassed the
/// admission waist's `DepthBoundViolation` dimension. While the query was a writable
/// string, a bound inside it was a fourth input nobody compared against the other
/// three: writing the natural `LIMIT <depth>` — the depth being the number this layer
/// reasons about everywhere — left no slot for the probe row, and a depth of three over
/// a producer holding nine rows was then reported `Exhausted { rows_emitted: 3 }` with
/// `rows_emitted` equal to the rows pulled, so nothing downstream could catch it
/// either. Rendering only the *outer* bound left the branch's bound in that same
/// string, and the inner bound is the one that decides the read, so the identical
/// false claim came back.
///
/// So: the depth and the declaration are reachable only through [`StratumUnit::new`],
/// the reach is derived from them, and the query is not a string this constructor can
/// be handed as the compiler's. A unit built here runs a text this layer did not write,
/// and therefore never reports `Exhausted` — its ending names that text
/// ([`ProducerStatus::SuppliedQueryEnded`]), and the compiled unit beside it, over the
/// very same producer, still earns the certified exhaustion. Every refusal below is
/// executed beside the neighbouring value that must still build, because a checked
/// surface that refused the honest cases would have closed the seam instead of holding
/// it.
#[test]
fn start_at_execute_a_hand_built_unit_is_checked_on_the_numbers_it_claims() {
    let registry = single_registry(&ex("stratum/hand"), &ex("pf/hand"), 12, 3);
    let stats = single_statistics(&ex("stratum/hand"), 12);
    let request = RetrievalRequest::complete(vec![lexical_term()]);
    let planned = plan(&request, &registry, &stats).expect("plans");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&planned, &env).expect("the plan is admitted");
    let emitted = &compiled.units[0];
    let query = hand_written(&ex("pf/hand"));
    let rebuild = |depth: u32, declared: Option<u64>| {
        StratumUnit::new(
            emitted.stratum.clone(),
            query.clone(),
            emitted.contract.clone(),
            depth,
            declared,
        )
    };

    // The seam is open: the compiler's own numbers, handed to the constructor with a
    // query of this test's own, build a unit — and it is *not* the compiler's unit,
    // which is the fourth dimension stated as a fact about the type. The compiled
    // query is no longer a string anything can hand back.
    let by_hand = rebuild(emitted.depth(), emitted.declared_rows())
        .expect("the compiler's own numbers are admitted");
    assert_ne!(
        &by_hand, emitted,
        "a unit running a caller's text is not the unit the compiler rendered, however \
         much the numbers on it agree"
    );
    assert_eq!(
        by_hand.supplied_query(),
        Some(query.as_str()),
        "the caller's text is carried verbatim and read back as the caller's"
    );
    assert_eq!(
        emitted.supplied_query(),
        None,
        "while the compiled unit supplies none: its query is parts, not text"
    );

    // THE FOURTH DIMENSION, on the numbers it still varies: one query, three depths,
    // three bounds — and the bound is the depth's every time, because the unit renders
    // it rather than carrying it. Every case above and below varies a number while
    // reusing this text, so none of them could observe a bound that disagreed with the
    // depth beside it.
    //
    // The caller's text is a sub-`SELECT` of the bounded query rather than the front of
    // it, because a text carrying a top-level bound of its own can hold no second one:
    // appended, the two are two `LIMIT` clauses of one solution modifier, which the
    // grammar admits one of, and the caller's is the one a last-clause-wins parser
    // drops. `a_supplied_bound_is_not_destroyed_by_the_layers_own` executes that case.
    for depth in [1_u32, 3, 12] {
        let unit = rebuild(depth, Some(12)).expect("a depth inside the declaration builds");
        assert_eq!(
            unit.sparql(),
            format!(
                "SELECT * WHERE {{\n  {{ {query} }}\n}}\nLIMIT {}",
                depth + 1
            ),
            "the text a unit runs is its query wrapped in a read bounded one row past \
             its own depth"
        );
    }

    // A depth of zero reads nothing and proves nothing, and one row is the
    // neighbouring read that must still build.
    assert_eq!(rebuild(0, Some(12)), Err(UnitError::ZeroDepth));
    assert!(rebuild(1, Some(12)).is_ok(), "one row is a real read");

    // A depth whose probe row is not expressible could report no ending at all, so
    // `Exhausted` would be reportable for a read the emitted bound cut. One rank
    // shallower is the deepest readable depth and must still build.
    assert_eq!(
        rebuild(u32::MAX, Some(u64::MAX)),
        Err(UnitError::DepthWithoutProbe {
            depth: u32::MAX,
            ceiling: u32::MAX - 1,
        })
    );
    assert!(
        rebuild(u32::MAX - 1, Some(u64::MAX)).is_ok(),
        "the deepest depth whose ending can be observed still builds"
    );

    // A depth above the declared row bound: the read it describes cannot be taken,
    // and the rows that do come back are not that depth's. This is the waist's
    // `DepthBoundViolation` dimension, held at the stage that was trusting it.
    assert_eq!(
        rebuild(4, Some(1)),
        Err(UnitError::DepthBeyondDeclaration {
            depth: 4,
            declared: 1,
        })
    );
    assert!(
        rebuild(1, Some(1)).is_ok(),
        "a depth at the declaration is the ordinary case, not a violation"
    );
    // A declared zero is read as the floor of one here too, exactly as the planner
    // and the waist read it — and only the floor.
    assert!(
        rebuild(1, Some(0)).is_ok(),
        "the floored probing row is admitted against a declared zero"
    );
    assert_eq!(
        rebuild(2, Some(0)),
        Err(UnitError::DepthBeyondDeclaration {
            depth: 2,
            declared: 0,
        })
    );
    // And a producer that declared no access mode declared no bound, so there is no
    // number here for a depth to exceed.
    assert!(
        rebuild(9, None).is_ok(),
        "silence bounds no read, at any depth a read can reach"
    );

    // The COMPILED unit over this very producer, run first so the pair below is a
    // measurement of the query's provenance and of nothing else: three rows behind a
    // twelve-row declaration, read to their end, certified.
    let rendered = block_on(execute(&compiled, &registry, &*common::empty_dataset()))
        .expect("the compiled unit runs");
    assert_eq!(
        rendered.statuses[&stratum("hand")],
        ProducerStatus::Exhausted { rows_emitted: 3 },
        "a read this layer bounded, three rows deep into a bound of thirteen, is an \
         exhaustion it can verify"
    );

    // The hand-built bundle runs, which is the point of the seam: the checks are a
    // gate on dishonest numbers and not a lock on the door. What it does not get is the
    // certificate above — same producer, same three rows, same depth, and a text this
    // layer cannot see the bounds of, so the ending names that text.
    let bundle = bundle_of(vec![by_hand], &compiled);
    let execution =
        block_on(execute(&bundle, &registry, &*common::empty_dataset())).expect("the unit runs");
    assert_eq!(
        execution.statuses[&stratum("hand")],
        ProducerStatus::SuppliedQueryEnded { rank: 3 },
        "three rows came back inside the bound, and whether the caller's own text cut \
         the read is not a fact this layer holds"
    );

    // And the run the fourth dimension was worth catching for: a hand-built unit at
    // depth three over a producer holding NINE rows. The read is cut by the depth and
    // says so. This is the observation a caller-written `LIMIT 3` destroyed — the
    // probe slot vanished, no row could arrive past the depth, and the layer's
    // strongest completeness claim was reported for a read with six rows behind it.
    let nine = single_registry(&ex("stratum/hand"), &ex("pf/hand"), 1_000, 9);
    let nine_stats = single_statistics(&ex("stratum/hand"), 1_000);
    let nine_env = AdmissionEnvironment {
        registry: &nine,
        statistics: &nine_stats,
        fusion_profile: None,
    };
    let nine_compiled = compile(
        &plan(&request, &nine, &nine_stats).expect("plans"),
        &nine_env,
    )
    .expect("the plan is admitted");
    let shallow = StratumUnit::new(
        nine_compiled.units[0].stratum.clone(),
        hand_written(&ex("pf/hand")),
        nine_compiled.units[0].contract.clone(),
        3,
        Some(1_000),
    )
    .expect("depth three inside a thousand-row declaration is admitted");
    let cut = block_on(execute(
        &bundle_of(vec![shallow], &nine_compiled),
        &nine,
        &*common::empty_dataset(),
    ))
    .expect("the unit runs");
    assert_eq!(
        cut.statuses[&stratum("hand")],
        ProducerStatus::DepthReached { rank: 3 },
        "nine rows read at depth three ended at the depth, and a caller's text keeps \
         the ending it CAN observe: the fourth row arrived, so something is down there"
    );
}

/// A supplied text's own top-level bound survives this layer's, and the ending is
/// the caller's text rather than a read that text did not take.
///
/// The layer's bound used to FOLLOW the caller's text, which made two `LIMIT`
/// clauses of one solution modifier — a shape the grammar admits exactly one of
/// (`LimitOffsetClauses ::= LimitClause OffsetClause? | OffsetClause LimitClause?`).
/// A parser that keeps the last clause therefore dropped the caller's: a text asking
/// for two rows over a producer holding nine returned THREE, and the stream's ending
/// named that text as the stopper for a read it had not stopped. Wrapping the text
/// instead leaves the caller's bound applying to the pattern it was written against
/// and this layer's applying to the whole of it.
///
/// Both halves are executed, because a wrap that bounded the caller's text away would
/// be the mirror fault: the unbounded neighbour reads to the *depth*, which is the
/// case that proves the wrap did not become a second ceiling of its own.
#[test]
fn a_supplied_bound_is_not_destroyed_by_the_layers_own() {
    let registry = single_registry(&ex("stratum/hand"), &ex("pf/hand"), 1_000, 9);
    let stats = single_statistics(&ex("stratum/hand"), 1_000);
    let request = RetrievalRequest::complete(vec![lexical_term()]);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&plan(&request, &registry, &stats).expect("plans"), &env)
        .expect("the plan is admitted");

    // The caller's own text, bounded at two by the caller. The depth is four, so this
    // layer's own bound is five: whichever bound is dropped, the row count says which.
    let bounded = format!("{}\nLIMIT 2", hand_written(&ex("pf/hand")));
    let unit = StratumUnit::new(
        compiled.units[0].stratum.clone(),
        bounded,
        compiled.units[0].contract.clone(),
        4,
        Some(1_000),
    )
    .expect("depth four inside a thousand-row declaration is admitted");
    let bundle = bundle_of(vec![unit], &compiled);
    let execution =
        block_on(execute(&bundle, &registry, &*common::empty_dataset())).expect("the unit runs");
    assert_eq!(
        execution.statuses[&stratum("hand")],
        ProducerStatus::SuppliedQueryEnded { rank: 2 },
        "the caller asked for two rows and got two: its bound is inside this layer's \
         rather than beside it, so neither clause replaced the other"
    );

    // The neighbouring valid case: the same text with no bound of its own reads to the
    // depth over the same nine-row producer. The wrap bounds the whole read at
    // `depth + 1`, so the fifth row arrives and the depth is named as the stopper —
    // which is what proves the wrap did not narrow the read to something else.
    let unbounded = StratumUnit::new(
        compiled.units[0].stratum.clone(),
        hand_written(&ex("pf/hand")),
        compiled.units[0].contract.clone(),
        4,
        Some(1_000),
    )
    .expect("depth four inside a thousand-row declaration is admitted");
    let open = bundle_of(vec![unbounded], &compiled);
    let execution =
        block_on(execute(&open, &registry, &*common::empty_dataset())).expect("the unit runs");
    assert_eq!(
        execution.statuses[&stratum("hand")],
        ProducerStatus::DepthReached { rank: 4 },
        "with nothing of the caller's bounding it, the same text reads to the depth and \
         the probe row past it arrives"
    );
}

/// **A caller's prologue survives the wrap, and a text that is not a query never
/// becomes a unit.**
///
/// The layer's bound wraps a supplied text in a sub-`SELECT`, and the grammar puts no
/// prologue inside one: `PREFIX` and `BASE` are `Prologue`, which appears once, at the
/// front of a whole query. So wrapping a prefixed text *as it stands* produced a query
/// that does not parse, and the caller was handed a syntax error at a byte offset of a
/// query this layer wrote — for a text that was perfectly good and that the seam had
/// run before the wrap existed. One narrow shape was fixed (a text carrying its own
/// top-level bound, which cannot hold a second one) and a strictly larger valid class
/// was broken, including that same shape whenever it was prefixed.
///
/// Every shape a supplied text comes in is executed here, because the fault was
/// invisible to a suite whose every fixture spelled its IRIs absolutely: a prologue the
/// body uses, one it does not, a `BASE`, no prologue at all, a bound of the caller's
/// own, a bound inside the caller's own sub-`SELECT`, and a prologue **and** a bound
/// together — the case that failed both before the wrap and after it.
///
/// The refusals are executed too, and they are the reason the parse is affordable: the
/// hoist needs to know where the prologue ends, which only a parse can say, so a text
/// that is not a query at all is refused at the constructor rather than carried to one
/// stratum's failure after a bundle was assembled — and, since the same parse names the
/// form, so is a query the wrapper could never hold: an `ASK`, `CONSTRUCT` or
/// `DESCRIBE`, refused by that name, beside every `SELECT` shape that must still run.
#[test]
fn a_supplied_prologue_is_hoisted_over_the_wrap_and_a_non_query_is_refused() {
    // Three rows behind a depth of four, so every read below that the caller did not
    // bound returns all three and ends on the caller's text — and the two that ARE
    // bounded at two return two. The row count is therefore what says which bound
    // applied.
    let registry = single_registry(&ex("stratum/hand"), &ex("pf/hand"), 1_000, 3);
    let stats = single_statistics(&ex("stratum/hand"), 1_000);
    let request = RetrievalRequest::complete(vec![lexical_term()]);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&plan(&request, &registry, &stats).expect("plans"), &env)
        .expect("the plan is admitted");
    let unit = |query: String| {
        StratumUnit::new(
            compiled.units[0].stratum.clone(),
            query,
            compiled.units[0].contract.clone(),
            4,
            Some(1_000),
        )
    };
    let ending = |query: String| {
        let built = unit(query).expect("a query with a prologue is a query");
        let bundle = bundle_of(vec![built], &compiled);
        block_on(execute(&bundle, &registry, &*common::empty_dataset()))
            .expect("the unit runs")
            .statuses[&stratum("hand")]
            .clone()
    };

    // The relation's IRI, spelled three ways: absolutely, through a prefix the body
    // uses, and relatively against a `BASE`. All three name the same producer.
    let namespace = ex("pf/");
    let prefix = format!("PREFIX rel: <{namespace}>\n");
    let base = format!("BASE <{namespace}>\n");
    let absolute = hand_written(&ex("pf/hand"));
    let prefixed = hand_written_over("rel:hand");
    let relative = hand_written_over("<hand>");

    // (1) THE CONTROL: no prologue, absolute IRI. Unchanged by the hoist, byte for
    //     byte — the split is at zero, so there is nothing in front of the wrapper.
    assert_eq!(
        ending(absolute.clone()),
        ProducerStatus::SuppliedQueryEnded { rank: 3 },
        "the shape the seam always ran: three rows inside a depth of four, ending on \
         the caller's own text"
    );

    // (2) A PREFIX the body uses. This is the case the wrap broke outright: the
    //     directives are lifted above the wrapping `SELECT`, so `rel:hand` resolves
    //     against the caller's own declaration and the same three rows come back.
    assert_eq!(
        ending(format!("{prefix}{prefixed}")),
        ProducerStatus::SuppliedQueryEnded { rank: 3 },
        "a prefixed text names the same producer and reads the same rows"
    );

    // (3) A PREFIX the body does not use. Nothing resolves through it, and it must
    //     still not break the query it is written in front of.
    assert_eq!(
        ending(format!("PREFIX unused: <{}>\n{absolute}", ex("unused#"))),
        ProducerStatus::SuppliedQueryEnded { rank: 3 },
        "a declaration the body never spells is carried, not tripped over"
    );

    // (4) A BASE, with the relation named relatively against it.
    assert_eq!(
        ending(format!("{base}{relative}")),
        ProducerStatus::SuppliedQueryEnded { rank: 3 },
        "a relative reference resolves against the caller's own base, which is in \
         scope for the whole wrapped query"
    );

    // (5) The caller's own top-level bound, which is why the wrap exists: appended,
    //     this layer's bound and the caller's would be two `LIMIT` clauses of one
    //     solution modifier and the caller's would vanish.
    assert_eq!(
        ending(format!("{absolute}\nLIMIT 2")),
        ProducerStatus::SuppliedQueryEnded { rank: 2 },
        "the caller asked for two rows and got two"
    );

    // (6) A bound inside the caller's own sub-`SELECT`, which this layer cannot see
    //     from outside and does not need to: the rows say what it did.
    assert_eq!(
        ending(format!(
            "SELECT ?candidate WHERE {{ {{ SELECT (?c0 AS ?candidate) WHERE {{ ( ?c0 ) \
             <{}> ( ?c1 ) }} LIMIT 2 }} }}",
            ex("pf/hand")
        )),
        ProducerStatus::SuppliedQueryEnded { rank: 2 },
        "a bound the caller wrote inside its own sub-SELECT still decides the read"
    );

    // (7) A PREFIX **and** the caller's own bound. This shape failed before the wrap
    //     (the two bounds collided) and after it (the prologue was wrapped into a
    //     sub-SELECT), so it is the one case neither arrangement served.
    assert_eq!(
        ending(format!("{prefix}{prefixed}\nLIMIT 2")),
        ProducerStatus::SuppliedQueryEnded { rank: 2 },
        "a prefixed text carrying its own bound keeps both: the prologue is hoisted \
         and the bound is wrapped"
    );

    // What the emitted text actually looks like, for the one case the shape matters
    // in: the directives in front, the wrapper after them, and the caller's body
    // inside it unchanged.
    let hoisted = unit(format!("{prefix}{prefixed}")).expect("a prefixed text is a query");
    assert_eq!(
        hoisted.sparql(),
        format!("{prefix}SELECT * WHERE {{\n  {{ {prefixed} }}\n}}\nLIMIT 5"),
        "the prologue is hoisted rather than re-rendered, so the body inside the wrap \
         is the caller's own bytes"
    );
    assert_eq!(
        hoisted.supplied_query(),
        Some(format!("{prefix}{prefixed}").as_str()),
        "and the text read back is the whole of what the caller handed over, prologue \
         included"
    );

    // (8) THE REFUSAL. A text that is not a query is refused where the caller can
    //     still act on it, carrying the parser's own diagnostic.
    match unit("THIS IS NOT SPARQL".to_owned()).expect_err("not a query, so not a unit") {
        UnitError::NotAQuery { reason } => assert!(
            reason.contains("expected SELECT, CONSTRUCT, ASK or DESCRIBE"),
            "the parser's own words, not this layer's summary of them: {reason}"
        ),
        other => panic!("expected NotAQuery, got {other:?}"),
    }
    // And its neighbour that must still build: the same text with a query in it.
    assert!(
        unit(absolute).is_ok(),
        "the refusal is about the text being a query, and nothing else"
    );

    // (9) THE OTHER REFUSAL. A query that is not a SELECT is refused by its form's
    //     name. Before this refusal each of these built a unit and failed at execution
    //     with "syntax error at byte 21: expected an RDF term, found ASK" — the same
    //     diagnostic-about-a-query-the-caller-never-wrote that the prologue produced,
    //     and for the same reason: the grammar admits only a SELECT inside the wrapper.
    //     None of these forms yields a row to rank, so no valid text is lost.
    let producer = ex("pf/hand");
    let pattern = format!("( ?c0 ) <{producer}> ( ?c1 )");
    for (form, text) in [
        ("ASK", format!("ASK WHERE {{ {pattern} }}")),
        (
            "CONSTRUCT",
            format!("CONSTRUCT {{ ?c0 <{producer}> ?c1 }} WHERE {{ {pattern} }}"),
        ),
        ("DESCRIBE", format!("DESCRIBE ?c0 WHERE {{ {pattern} }}")),
    ] {
        match unit(text).expect_err("not a SELECT, so not a unit") {
            UnitError::NotASelect { form: named } => {
                assert_eq!(named, form, "the refusal names the form the caller wrote");
            }
            other => panic!("expected NotASelect for {form}, got {other:?}"),
        }
    }
    // Its neighbours that must still run: a trailing VALUES and a VERSION directive are
    // shapes the grammar lets a whole query carry and a bare sub-SELECT does not spell,
    // and each is still a SELECT, so the refusal must not reach either. Both are
    // genuinely honoured through the wrap, and the rows say so.
    //
    // The third such shape — a dataset clause — is not asserted here, and deliberately
    // not: this fixture's relation is registry-driven over an empty dataset, so which
    // graphs a clause selects cannot change a single row of it, and an assertion here
    // would be an oracle blind to the one thing that can go wrong with that clause. It
    // is executed where it is observable instead, over a dataset holding different rows
    // in different graphs, in `execute_dataset.rs`.
    let select = format!("SELECT ?candidate WHERE {{ {pattern} BIND(?c0 AS ?candidate) }}");
    for (shape, text) in [
        ("VALUES", format!("{select} VALUES ?x {{ 1 }}")),
        ("VERSION", format!("VERSION \"1.2\"\n{select}")),
    ] {
        assert_eq!(
            ending(text),
            ProducerStatus::SuppliedQueryEnded { rank: 3 },
            "a SELECT carrying {shape} is a SELECT, and reads its three rows"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. Start at execute: hand-written SPARQL, no composition types in the path
// ---------------------------------------------------------------------------

#[test]
fn start_at_execute_evaluator_only() {
    // No `Plan`, `CompiledRetrieval`, `ExecutionResult` or `FusionProfile` is
    // constructed: the relation is reached by writing the query by hand.
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/match"),
        make_producer(10, "direct/", 3),
        ranked(
            &ex("stratum/direct"),
            vec![TermPattern::of_kind(TermKind::Any)],
            true,
        ),
    );

    let sparql = "SELECT ?s WHERE { ?s <http://example.org/pf/match> ?o }";
    let rows = run_query(sparql, &registry);
    let expected: Vec<Vec<Option<TermValue>>> = (0..3)
        .map(|index| {
            vec![Some(TermValue::iri(format!(
                "{}entity{index}",
                ex("direct/")
            )))]
        })
        .collect();
    assert_eq!(
        rows, expected,
        "the evaluator answers the hand-written query"
    );
}

// ---------------------------------------------------------------------------
// 6. Start at fuse: hand-built streams, no earlier stage
// ---------------------------------------------------------------------------

#[test]
fn start_at_fuse_hand_built_streams() {
    let profile = profile(&[("s1", Fixed::ONE), ("s2", Fixed::ONE)], K);
    let streams = vec![
        (
            stratum("s1"),
            ScriptedStream::new(vec![row(1, Fixed::ONE, K, "alpha")], exhausted(1)),
        ),
        (
            stratum("s2"),
            ScriptedStream::new(vec![row(1, Fixed::ONE, K, "beta")], exhausted(1)),
        ),
    ];

    let result = block_on(fuse::<ScriptedStream, Term>(streams, &profile, TOP_K))
        .expect("fusion works with no planning, compilation or execution");

    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.trailer.statuses.len(), 2);
    assert_eq!(result.trailer.profile_id, profile.id());

    // These streams descend from no index, so they attest nothing — and the
    // seam records that as the absence it is rather than inventing evidence
    // for a caller that assembled its rows by hand. `Exact` here is the narrow
    // claim it always is: no stratum declared itself short.
    assert_eq!(
        result.trailer.attestations,
        BTreeMap::from([
            (stratum("s1"), PfAttestation::UNDECLARED),
            (stratum("s2"), PfAttestation::UNDECLARED),
        ]),
        "a hand-built stream is asked and honestly says nothing"
    );
    assert_eq!(result.trailer.exactness, ScoreExactness::Exact);
}

// ---------------------------------------------------------------------------
// 6b. Start at fuse: a hand-built stream may author the ending its own depth
//     gave it
//
// The seam is a place to start, and a caller starting here is the party that
// read its own index to some depth and stopped. Fusion cannot tell that apart
// from a stream that ran out — both simply stop yielding rows — so the ending
// has to be declarable at the seam, in the rank space the caller actually
// stopped in. What the seam *can* check, it does: the rank against the rows it
// pulled.
// ---------------------------------------------------------------------------

#[test]
fn a_hand_built_stream_can_declare_the_depth_that_stopped_it() {
    let profile = profile(&[("s1", Fixed::ONE)], K);
    let scripted = || {
        vec![
            row(1, Fixed::ONE, K, "alpha"),
            row(2, Fixed::ONE, K, "beta"),
        ]
    };

    let stopped = block_on(fuse::<ScriptedStream, Term>(
        vec![(
            stratum("s1"),
            ScriptedStream::new(scripted(), ProducerReceipt::DepthReached { rank: 2 }),
        )],
        &profile,
        TOP_K,
    ))
    .expect("a depth-stopped stream fuses");

    assert_eq!(
        stopped.rows.len(),
        2,
        "every row the caller did read is in the answer"
    );
    assert_eq!(
        stopped.trailer.statuses.get(&stratum("s1")),
        Some(&ProducerStatus::DepthReached { rank: 2 }),
        "and the trailer says the depth stopped the read, which no other party could say"
    );

    // The count is still measured, because a licence to stop reading is not a
    // licence to misreport what was read.
    let forged = block_on(fuse::<ScriptedStream, Term>(
        vec![(
            stratum("s1"),
            ScriptedStream::new(scripted(), ProducerReceipt::DepthReached { rank: 1 }),
        )],
        &profile,
        TOP_K,
    ));
    assert!(
        matches!(
            &forged,
            Err(FusionError::Protocol(error))
                if matches!(**error, ProtocolError::ForgedReceipt { declared: 1, actual: 2 })
        ),
        "expected ForgedReceipt {{ declared: 1, actual: 2 }}, got {forged:?}"
    );
}

// ---------------------------------------------------------------------------
// 7. Fused is top-k with memory proportional to the frontier
// ---------------------------------------------------------------------------

/// How many candidates the strata may disagree about at once.
///
/// Each stratum below emits the same universe of candidates permuted **within**
/// blocks of this size, so a candidate's ranks differ across strata by less than
/// one block and the frontier holds the candidates of at most a couple of blocks
/// at any moment. It is the disagreement window, and it is what the frontier's
/// size is proportional to — not the stream length, which is the claim.
const DISAGREEMENT_BLOCK: u64 = 4;

/// The three strata the frontier fixtures fuse, in their permutation order.
const FRONTIER_STRATA: [&str; 3] = ["nra/a", "nra/b", "nra/c"];

/// The candidate one stratum emits at 1-based `rank`.
///
/// Stratum 0 emits the universe in order; stratum 1 reverses each block; stratum
/// 2 rotates each block by half its width. Each is a permutation of the same
/// universe, so every stream emits distinct items (the protocol's uniqueness
/// rule) and every candidate is eventually seen by every stratum — which is what
/// makes the frontier hold candidates awaiting confirmation rather than sit
/// empty. A single-stratum fixture proves nothing here: with one stream there is
/// nothing to await, and the frontier never holds anything at all.
fn permuted_index(stream: usize, rank: u64) -> u64 {
    let index = rank - 1;
    let block = index / DISAGREEMENT_BLOCK;
    let offset = index % DISAGREEMENT_BLOCK;
    let permuted = match stream {
        0 => offset,
        1 => DISAGREEMENT_BLOCK - 1 - offset,
        _ => (offset + DISAGREEMENT_BLOCK / 2) % DISAGREEMENT_BLOCK,
    };
    block * DISAGREEMENT_BLOCK + permuted
}

/// What a [`LazyStream`] emits at a 1-based rank: the candidate a given stream
/// carries there. The rows themselves — rank, contribution, receipt — are the
/// same whichever universe is being enumerated, so the universe is the one thing
/// a fixture supplies.
type ItemAt = fn(usize, u64) -> Term;

/// The block-permuted universe [`permuted_index`] describes.
fn permuted_item(stream_index: usize, rank: u64) -> Term {
    let index = permuted_index(stream_index, rank);
    Term::new(format!("candidate-{index:08}"))
}

/// An effectively unbounded producer that mints rows lazily and counts pulls.
///
/// Nothing is materialized up front: if fusion needed the whole input to emit
/// its first rows, the pull count would approach `total`.
struct LazyStream {
    /// Which permutation of the universe this stream emits.
    stream_index: usize,
    /// How many rows it has emitted so far.
    emitted: u64,
    /// How many rows it could emit in total.
    total: u64,
    /// The stratum weight its contributions are computed under.
    weight: Fixed,
    /// The profile's smoothing constant.
    k: u32,
    /// A shared count of every row pulled from every such stream.
    pulls: Arc<AtomicUsize>,
    /// The candidate universe this stream enumerates.
    item_at: ItemAt,
}

#[allow(clippy::unused_async_trait_impl)]
impl RankedStream for LazyStream {
    type Item = Term;

    async fn next(&mut self) -> Result<Option<RankedRow<Self::Item>>, ProtocolError> {
        if self.emitted >= self.total {
            return Ok(None);
        }
        self.emitted += 1;
        self.pulls.fetch_add(1, Ordering::SeqCst);
        let rank = self.emitted;
        let value = contribution(self.weight, rank, self.k).expect("contribution fits");
        Ok(Some(RankedRow::new(
            rank,
            value,
            (self.item_at)(self.stream_index, rank),
            RowBlock::Undeclared,
        )))
    }

    async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError> {
        Ok(exhausted(self.emitted))
    }

    fn contract(&self) -> StreamContract {
        unique_items()
    }
}

/// The three-stratum profile and streams the frontier fixtures share.
///
/// The contribution maximum is three, one per stratum, because every candidate
/// is eventually seen by all three.
fn frontier_fixture(
    total: u64,
    pulls: &Arc<AtomicUsize>,
) -> (FusionProfile, Vec<(Iri, LazyStream)>) {
    let weights: BTreeMap<Iri, Fixed> = FRONTIER_STRATA
        .iter()
        .map(|name| (stratum(name), Fixed::ONE))
        .collect();
    let profile = FusionProfile::with_decay(weights, DecayRule::ReciprocalRank { k: K })
        .expect("the fixture profile is valid");
    let streams = FRONTIER_STRATA
        .iter()
        .enumerate()
        .map(|(stream_index, name)| {
            (
                stratum(name),
                LazyStream {
                    stream_index,
                    emitted: 0,
                    total,
                    weight: Fixed::ONE,
                    k: K,
                    pulls: Arc::clone(pulls),
                    item_at: permuted_item,
                },
            )
        })
        .collect();
    (profile, streams)
}

#[test]
fn fused_top_k_holds_a_bounded_frontier_across_disagreeing_strata() {
    // Three million-row streams that disagree about the order of every block;
    // only ten fused rows are asked for.
    //
    // This drives `fuse` — the shipped entry point, trailer and all — rather
    // than the engine underneath it, because the bound is a property of what a
    // caller can actually call. A `fuse` whose terminal report read each stream
    // to its end would return the same ten rows while pulling three million, and
    // a test that stopped at `FusionStream` would report that as a success.
    const ROWS: usize = 10;
    let total = 1_000_000u64;
    let pulls = Arc::new(AtomicUsize::new(0));
    let (profile, streams) = frontier_fixture(total, &pulls);

    let fused = block_on(fuse::<LazyStream, Term>(streams, &profile, TopK::new(ROWS)))
        .expect("fusion pulls");
    let top = fused.rows;
    let pulled = pulls.load(Ordering::SeqCst);

    // The frontier was genuinely populated, not bypassed: every certified row
    // carries a contribution from all three strata, which means it was held
    // while the other strata caught up to it. That is the state a single-stratum
    // fixture has none of.
    assert_eq!(top.len(), ROWS);
    for row in &top {
        assert_eq!(
            row.contributions.len(),
            FRONTIER_STRATA.len(),
            "{} was certified without every stratum confirming it",
            row.entity.as_str()
        );
        assert!(
            row.threshold_witness > Fixed::ZERO,
            "{} was certified with no stream still live, so nothing was awaited",
            row.entity.as_str()
        );
    }

    // The frontier can never be larger than the rows pulled less the rows
    // emitted, so bounding the pulls bounds the frontier. Ten rows out of three
    // million cost a few dozen pulls: certification is a threshold argument over
    // the live heads, not a scan.
    assert!(
        pulled < 64,
        "fused emission pulled {pulled} rows for {ROWS}, which is not a frontier bound"
    );
    assert!(
        pulled - ROWS < 32,
        "{} candidates were still un-emitted after {ROWS} rows",
        pulled - ROWS
    );

    // And the trailer earned its statuses without reading further. Every
    // stratum still held rows when the bound was reached, so every one of them
    // is closed at the contribution it was read down to — the honest claim, and
    // the only one available for the price of the pulls asserted above.
    assert_eq!(fused.trailer.statuses.len(), FRONTIER_STRATA.len());
    for (stratum, status) in &fused.trailer.statuses {
        let ProducerStatus::CeilingReached { bound } = status else {
            panic!("{stratum} still held rows, so its status cannot be {status:?}");
        };
        assert!(
            *bound > Fixed::ZERO,
            "{stratum} was closed at a bound of zero, which names no row"
        );
    }
}

// ---------------------------------------------------------------------------
// 7b. An exact tie is ordered, not awaited
//
// Reciprocal-rank fusion ties exactly whenever two strata disagree
// symmetrically, which is an ordinary outcome and not a corner case. Two
// candidates with the same final score can each be the other's equal-valued
// rival; if "could still be ordered ahead of me" were read as a score
// comparison alone, each would block the other and fusion would read both
// streams to the end before ordering them. The frontier bound this rung
// promises is exactly what that costs, so it is asserted here in pulls.
// ---------------------------------------------------------------------------

/// The raw contribution of a unit-weighted rank-1 row under `K`.
///
/// `trunc(10^12 / (60 + 1))`, written out rather than asked for: the tie below
/// has to be arithmetic this file states and the engine must agree with, not
/// arithmetic the engine states and this file reads back.
const RANK_1_RAW: i128 = 16_393_442_622;

/// The same at rank two: `trunc(10^12 / (60 + 2))`.
const RANK_2_RAW: i128 = 16_129_032_258;

/// How many rows sit behind the tied pair in each stream.
///
/// Far more than a bounded frontier can touch, and few enough that a fusion
/// which wrongly drained them would still finish and fail an assertion rather
/// than hang.
const TIE_TAIL_ROWS: u64 = 10_000;

/// The two strata the tie fixture fuses.
const TIE_STRATA: [&str; 2] = ["tie/left", "tie/right"];

/// Two strata that disagree symmetrically about their top two candidates: `a`
/// leads the left one with `b` immediately behind it, and the right one is the
/// mirror image. Everything from rank three on is stream-local filler, present
/// only so that draining is observably different from not draining.
fn symmetric_tie_item(stream_index: usize, rank: u64) -> Term {
    match (stream_index, rank) {
        (0, 1) | (1, 2) => Term::new("a"),
        (0, 2) | (1, 1) => Term::new("b"),
        _ => Term::new(format!("filler-{stream_index}-{rank:08}")),
    }
}

#[test]
fn exactly_tied_candidates_are_ordered_rather_than_awaited() {
    // The tie, by hand. A unit-weighted row at 1-based rank `r` contributes
    // `trunc(10^12 / (K + r))`, so the two ranks in play are:
    assert_eq!(RANK_1_RAW, 1_000_000_000_000 / (i128::from(K) + 1));
    assert_eq!(RANK_2_RAW, 1_000_000_000_000 / (i128::from(K) + 2));

    // `a` is rank 1 on the left and rank 2 on the right; `b` is rank 2 on the
    // left and rank 1 on the right. Each sum is written in the order its strata
    // contribute, and the two are the same number — which is the whole fixture.
    let a_score = RANK_1_RAW + RANK_2_RAW;
    let b_score = RANK_2_RAW + RANK_1_RAW;
    assert_eq!(a_score, 32_522_474_880);
    assert_eq!(b_score, 32_522_474_880);
    assert_eq!(a_score, b_score, "this is a fixture only if the scores tie");

    // The engine's contribution is that same arithmetic, so the rows the streams
    // emit below really do carry these two values.
    assert_eq!(
        contribution(Fixed::ONE, 1, K).expect("fits"),
        Fixed::from_raw(RANK_1_RAW)
    );
    assert_eq!(
        contribution(Fixed::ONE, 2, K).expect("fits"),
        Fixed::from_raw(RANK_2_RAW)
    );

    let pulls = Arc::new(AtomicUsize::new(0));
    let weights: BTreeMap<Iri, Fixed> = TIE_STRATA
        .iter()
        .map(|name| (stratum(name), Fixed::ONE))
        .collect();
    // Two strata, so a candidate takes at most two contributions.
    let tie_profile = FusionProfile::with_decay(weights, DecayRule::ReciprocalRank { k: K })
        .expect("the fixture profile is valid");
    let streams: Vec<(Iri, LazyStream)> = TIE_STRATA
        .iter()
        .enumerate()
        .map(|(stream_index, name)| {
            (
                stratum(name),
                LazyStream {
                    stream_index,
                    emitted: 0,
                    total: TIE_TAIL_ROWS,
                    weight: Fixed::ONE,
                    k: K,
                    pulls: Arc::clone(&pulls),
                    item_at: symmetric_tie_item,
                },
            )
        })
        .collect();

    let fused = block_on(fuse::<LazyStream, Term>(
        streams,
        &tie_profile,
        TopK::new(2),
    ))
    .expect("fusion pulls");
    let pulled = pulls.load(Ordering::SeqCst);
    let mut rows = fused.rows.into_iter();
    let first = rows.next().expect("the first tied candidate is certified");
    let second = rows.next().expect("the second tied candidate is certified");

    // Both are emitted, and in the order the declared total tie-break dictates:
    // the scores are equal, and so are the best ranks — each candidate is rank 1
    // in exactly one stratum — so the third key decides, and canonical term
    // bytes put `a` before `b`.
    assert_eq!(first.entity, Term::new("a"));
    assert_eq!(second.entity, Term::new("b"));
    for row in [&first, &second] {
        assert_eq!(
            row.score,
            Fixed::from_raw(a_score),
            "{} must carry the hand-computed tied score",
            row.entity.as_str()
        );
        assert_eq!(
            row.contributions.len(),
            TIE_STRATA.len(),
            "{} must be certified with both strata counted",
            row.entity.as_str()
        );
        assert!(
            row.threshold_witness > Fixed::ZERO,
            "{} was certified with no stream still live, so the tie was settled by \
             exhaustion rather than by the tie-break",
            row.entity.as_str()
        );
    }

    // Six pulls, and that is the bound the algorithm justifies rather than a
    // number observed and pinned: each stream gives up rank 1 and rank 2 — the
    // tied pair itself — plus exactly one lookahead row, which is what drops the
    // threshold below the tied score and proves both scores final. No row from
    // rank three on can alter either score, so no row from rank three on is
    // read. Twenty thousand rows were available behind them.
    const PER_STREAM: usize = 3;
    assert!(
        pulled <= TIE_STRATA.len() * PER_STREAM,
        "an exact tie pulled {pulled} rows of {} to emit two candidates, which is not a \
         frontier bound",
        u64::try_from(TIE_STRATA.len()).expect("two") * TIE_TAIL_ROWS
    );

    // The count above is the whole `fuse` call, trailer included: the rows
    // behind the tie were never touched, not even to write a report about them.
    assert_eq!(pulls.load(Ordering::SeqCst), pulled);
    assert_eq!(fused.trailer.statuses.len(), TIE_STRATA.len());
}

// ---------------------------------------------------------------------------
// 8. The unfused rung applies no threshold to its materialized rows
// ---------------------------------------------------------------------------

/// What this proves is the *absence of a threshold*, not unboundedness. The
/// stratum's rows are materialized by the evaluator before the first one is
/// read, so pulling all of them measures the rung's lack of cross-stratum
/// accounting — every row is consumable in contiguous rank order, down to a
/// clean receipt — and says nothing about how large a result could be.
#[test]
fn unfused_rung_applies_no_threshold_to_its_rows() {
    const ROWS: usize = 4096;
    let registry = single_registry(&ex("stratum/deep"), &ex("pf/deep"), ROWS as u64, ROWS);
    let stats = single_statistics(&ex("stratum/deep"), ROWS as u64);
    let request = RetrievalRequest::complete(vec![lexical_term()]);
    let planned = plan(&request, &registry, &stats).expect("plans");
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&planned, &env).expect("admits");
    let execution =
        block_on(execute(&compiled, &registry, &*common::empty_dataset())).expect("the units run");

    let mut stream = execution
        .streams
        .into_iter()
        .next()
        .expect("the stratum streamed");

    // The unfused rung has no threshold: every row the producer emitted is
    // immediately consumable, in contiguous rank order, to the end of the
    // stratum's own materialized result.
    let mut pulled = 0usize;
    let mut last_rank = 0u64;
    while let Some((rank, _term, _block)) =
        block_on(stream.stream.next()).expect("the stream pulls")
    {
        assert_eq!(rank, last_rank + 1, "ranks stay contiguous to the end");
        last_rank = rank;
        pulled += 1;
    }
    assert_eq!(pulled, ROWS, "the unfused rung emits the whole stream");
    let receipt = block_on(stream.stream.receipt()).expect("clean end");
    assert_eq!(receipt, exhausted(ROWS as u64));
}

// ---------------------------------------------------------------------------
// 9. Trailer semantics: a prefix reader holds incomplete evidence
// ---------------------------------------------------------------------------

#[test]
fn prefix_reader_incomplete_evidence() {
    let profile = profile(&[("s1", Fixed::ONE), ("s2", Fixed::ONE)], K);
    let streams = vec![
        (
            stratum("s1"),
            ScriptedStream::new(
                vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "x")],
                exhausted(2),
            ),
        ),
        (
            stratum("s2"),
            ScriptedStream::new(
                vec![row(1, Fixed::ONE, K, "a"), row(2, Fixed::ONE, K, "y")],
                exhausted(2),
            ),
        ),
    ];

    let mut fusion = FusionStream::new(streams, profile);
    let first = block_on(fusion.next())
        .expect("fusion pulls")
        .expect("a certified row");
    assert_eq!(first.entity, Term::new("a"));
    // The witness is strictly positive, so `a` was certified while both streams
    // still held rows: the prefix is provably incomplete and only the trailer
    // may assert otherwise.
    assert!(
        first.threshold_witness > Fixed::ZERO,
        "certification happened with rows still unread"
    );

    // A partially-read stream refuses to describe its own completeness.
    let mut partial = RankedStreamImpl::new(
        vec![
            (1, Term::new("a"), RowBlock::Undeclared),
            (2, Term::new("b"), RowBlock::Undeclared),
        ],
        StreamEnding::Exhausted,
    );
    assert!(block_on(partial.next()).expect("pulls").is_some());
    assert!(matches!(
        block_on(partial.receipt()),
        Err(ProtocolError::NeverEndingSource)
    ));

    // Only after the fusion is fully drained is the terminal report available.
    while block_on(fusion.next()).expect("fusion pulls").is_some() {}
    let trailer = block_on(fusion.trailer()).expect("the trailer is produced");
    assert_eq!(trailer.statuses.len(), 2);
}

// ---------------------------------------------------------------------------
// 10. Reporting names both the plan and the profile
// ---------------------------------------------------------------------------

#[test]
fn reporting_names_plan_and_profile() {
    let registry = single_registry(&ex("stratum/report"), &ex("pf/report"), 12, 3);
    let stats = single_statistics(&ex("stratum/report"), 12);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let profile = single_profile(&ex("stratum/report"));
    let request = RetrievalRequest::complete(vec![lexical_term()]);

    let expected_plan = plan(&request, &registry, &stats).expect("plans");
    let result = block_on(search(
        &request,
        &registry,
        &stats,
        &*common::empty_dataset(),
        &env,
        &profile,
    ))
    .expect("the composed search answers");

    // A reproducible answer is a pair of identities: the pinned plan and the
    // fusion law in force, both named by the answer.
    assert_eq!(result.plan_id, expected_plan.id());
    assert_eq!(result.profile_id, profile.id());
    assert_eq!(result.trailer.profile_id, profile.id());
}

// ---------------------------------------------------------------------------
// 11. No stage takes a selector
// ---------------------------------------------------------------------------

/// `execute`'s shape, re-declared and delegated to.
///
/// An `async fn`'s return type is opaque, so there is no `fn` pointer type to
/// coerce it to. A delegation pins the same thing by a different mechanism: the
/// parameter list is written out here, and the call below supplies exactly those
/// arguments, so a stage that grew one more would stop compiling at this line.
// `future_not_send`: the crate's own `execute` carries this allow for the same
// reason — the dataset is a caller-chosen type parameter, so the future's
// `Send`-ness is the caller's to establish and is not required here.
#[allow(clippy::future_not_send)]
async fn execute_shape<D: purrdf_core::DatasetView + Sync>(
    compiled: &CompiledRetrieval,
    registry: &PropertyFunctionRegistry,
    dataset: &D,
) -> Result<ExecutionResult, ExecutionError> {
    execute(compiled, registry, dataset).await
}

/// `fuse`'s shape, re-declared and delegated to; see [`execute_shape`].
#[allow(clippy::future_not_send)]
async fn fuse_shape<S, T>(
    streams: Vec<(Iri, S)>,
    profile: &FusionProfile,
    top_k: TopK,
) -> Result<FusionResult<T>, FusionError>
where
    S: RankedStream<Item = T>,
    T: Ord + Clone + Into<Term>,
{
    fuse(streams, profile, top_k).await
}

/// `search`'s shape, re-declared and delegated to; see [`execute_shape`].
#[allow(clippy::future_not_send)]
async fn search_shape<S, D>(
    request: &RetrievalRequest,
    registry: &PropertyFunctionRegistry,
    statistics: &S,
    dataset: &D,
    env: &AdmissionEnvironment<'_>,
    profile: &FusionProfile,
) -> Result<SearchResult, SearchError>
where
    S: Statistics,
    D: purrdf_core::DatasetView + Sync,
{
    search(request, registry, statistics, dataset, env, profile).await
}

/// The seam is the same behaviour observed earlier; a mode flag would be a
/// second behaviour the full pipeline must be kept in agreement with. A caller
/// that wants less calls an earlier stage, and that is pinned here by **shape**
/// rather than by spelling: the two synchronous stages are coerced to an
/// explicit `fn` type and the three asynchronous ones are delegated to through
/// shims whose parameter lists are written out above.
///
/// # What this catches, and what it does not
///
/// It catches any change to a stage entry point's parameter list or result
/// type — a `skip_fuse: bool`, a `dry_run: bool`, an `Option<Mode>`, a reordered
/// argument, a widened return — as a **compile** error in this file, whatever
/// the parameter is named.
///
/// It does not catch a selector hidden inside a type a stage already takes: a
/// `skip_fuse` field added to [`AdmissionEnvironment`], [`FusionProfile`] or
/// [`RetrievalRequest`] would pass, as would an entirely new entry point added
/// alongside these five. It pins the shape of the boundary, not the whole
/// surface behind it.
#[test]
fn no_stage_takes_a_selector() {
    // Coercion to a `fn` type checks the whole signature at once. `plan` is
    // generic over the statistics it reads, so it is pinned at one concrete
    // instantiation; the other two parameters and the result are exact.
    let _: fn(
        &RetrievalRequest,
        &PropertyFunctionRegistry,
        &MockStatistics,
    ) -> Result<Plan, PlanError> = plan;
    let _: fn(&Plan, &AdmissionEnvironment<'_>) -> Result<CompiledRetrieval, AdmissionError> =
        compile;

    // The shims are exercised rather than merely declared, so the delegation is
    // live code a refactor must keep compiling.
    let registry = single_registry(&ex("stratum/seam"), &ex("pf/seam"), 10, 2);
    let stats = single_statistics(&ex("stratum/seam"), 10);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let request = RetrievalRequest::complete(vec![lexical_term()]);
    let planned = plan(&request, &registry, &stats).expect("plans");
    let compiled = compile(&planned, &env).expect("admits");
    let dataset = common::empty_dataset();

    let execution = block_on(execute_shape(&compiled, &registry, &*dataset)).expect("units run");
    assert_eq!(execution.streams.len(), 1);

    let seam_profile = single_profile(&ex("stratum/seam"));
    let fused = block_on(fuse_shape::<ScriptedStream, Term>(
        vec![(
            stratum("seam"),
            ScriptedStream::new(vec![row(1, Fixed::ONE, K, "alpha")], exhausted(1)),
        )],
        &seam_profile,
        TOP_K,
    ))
    .expect("hand-built streams fuse");
    assert_eq!(fused.rows.len(), 1);

    let searched = block_on(search_shape(
        &request,
        &registry,
        &stats,
        &*dataset,
        &env,
        &seam_profile,
    ))
    .expect("the composed search answers");
    assert_eq!(searched.plan_id, planned.id());
}

// ---------------------------------------------------------------------------
// 12. The canonical encoding is a pure function of the value
// ---------------------------------------------------------------------------

/// Render a fusion as deterministic text: rows in final order with their
/// provenance, then statuses by stratum, then the profile identity.
fn render_fusion(result: &FusionResult<Term>) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for fused in &result.rows {
        let _ = writeln!(
            out,
            "row {} score={} witness={}",
            fused.entity.as_str(),
            fused.score.to_decimal_lexical(),
            fused.threshold_witness.to_decimal_lexical()
        );
        for (stratum, rank, value) in &fused.contributions {
            let _ = writeln!(
                out,
                "  {} rank={rank} score={}",
                stratum.as_str(),
                value.to_decimal_lexical()
            );
        }
    }
    for (stratum, status) in &result.trailer.statuses {
        let _ = writeln!(out, "status {} {status:?}", stratum.as_str());
    }
    let _ = writeln!(out, "profile {}", result.trailer.profile_id.to_hex());
    out
}

async fn equality_fusion(profile: &FusionProfile) -> FusionResult<Term> {
    let streams = vec![
        (
            stratum("eq1"),
            ScriptedStream::new(
                vec![
                    row(1, Fixed::ONE, K, "alpha"),
                    row(2, Fixed::ONE, K, "beta"),
                ],
                exhausted(2),
            ),
        ),
        (
            stratum("eq2"),
            ScriptedStream::new(vec![row(1, Fixed::ONE, K, "beta")], exhausted(1)),
        ),
    ];
    fuse::<ScriptedStream, Term>(streams, profile, TOP_K)
        .await
        .expect("the fixture streams fuse")
}

/// Encoding and fusion are deterministic across repeated execution **in one
/// process**: that is exactly as much as a native-only test can observe, and the
/// name says so. A plan encodes to the same bytes twice, a decode reproduces
/// them, and two independent runs of one fusion law render identically.
///
/// The cross-target claim — that a `wasm32` execution agrees with this one — is
/// a different claim that this test cannot make, because it never leaves this
/// target. It is made where it can be executed, in `wasm_determinism`, which
/// runs the same law on `wasm32-unknown-unknown` against pinned expectations.
#[test]
fn encoding_and_fusion_are_deterministic_in_one_process() {
    let registry = single_registry(&ex("stratum/eq"), &ex("pf/eq"), 12, 3);
    let stats = single_statistics(&ex("stratum/eq"), 12);
    let request = RetrievalRequest::complete(vec![lexical_term()]);
    let planned = plan(&request, &registry, &stats).expect("plans");

    // A plan's canonical encoding is a pure function of its fields: repeated
    // encodings agree and a decode reproduces them exactly.
    let plan_bytes = planned.canonical_bytes();
    assert_eq!(plan_bytes, planned.canonical_bytes());
    assert_eq!(
        Plan::from_canonical_bytes(&plan_bytes)
            .expect("canonical bytes decode")
            .canonical_bytes(),
        plan_bytes
    );

    // The fusion profile's canonical encoding is a pure function of its fields
    // the same way.
    let profile = profile(&[("eq1", Fixed::ONE), ("eq2", Fixed::ONE)], K);
    let profile_bytes = profile.canonical_bytes();
    assert_eq!(profile_bytes, profile.canonical_bytes());
    assert_eq!(
        FusionProfile::from_canonical_bytes(&profile_bytes)
            .expect("canonical bytes decode")
            .canonical_bytes(),
        profile_bytes
    );

    // Fused output is byte-identical across two independent runs of the same
    // law within this process.
    let first = block_on(equality_fusion(&profile));
    let second = block_on(equality_fusion(&profile));
    assert_eq!(render_fusion(&first), render_fusion(&second));
}

// ---------------------------------------------------------------------------
// 11. Resuming at `execute` uses the exported bridge, not a hand-written copy
//
// A seam is a place to stop AND a place to start. The one thing a caller cannot
// reasonably re-derive when it resumes is the reciprocal-rank contribution, so
// the bridge that attaches it is exported rather than left to be rewritten —
// and being public, it must answer a malformed stream rather than panic on one.
// ---------------------------------------------------------------------------

#[test]
fn the_exported_bridge_carries_an_executed_stream_into_fusion() {
    let registry = single_registry(&ex("stratum/resume"), &ex("pf/resume"), 10, 3);
    let stats = single_statistics(&ex("stratum/resume"), 10);
    let request = RetrievalRequest::complete(vec![lexical_term()]);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let planned = plan(&request, &registry, &stats).expect("plans");
    let compiled = compile(&planned, &env).expect("admits");
    let execution =
        block_on(execute(&compiled, &registry, &*common::empty_dataset())).expect("the units run");

    // A caller that stopped at `execute` resumes here, with no arithmetic of its
    // own: the weight and the smoothing constant are read from the profile by
    // the adapter, so it cannot attach a contribution the profile did not
    // authorize.
    let profile = profile(&[("resume", Fixed::ONE)], K);
    let mut streams = Vec::new();
    for stream in execution.streams {
        let adapter =
            RankedStreamAdapter::new(stream.stream, stream.contract, &profile, &stream.stratum)
                .expect("the profile weights the stratum the plan reached")
                // The stream knows the plan its unit was compiled from, and the
                // bridge carries that on rather than making the caller re-state it.
                .with_plan_id(stream.plan_id);
        streams.push((stream.stratum, adapter));
    }
    let fused = block_on(fuse::<RankedStreamAdapter, Term>(streams, &profile, TOP_K))
        .expect("the bridged streams fuse");

    assert_eq!(fused.rows.len(), 3, "every executed row reached the fusion");
    assert_eq!(
        fused.trailer.plan_id,
        Some(planned.id()),
        "the pinned plan travelled with the rows into the trailer"
    );
    for (index, row) in fused.rows.iter().enumerate() {
        let rank = u64::try_from(index + 1).expect("the fixture is small");
        assert_eq!(
            row.score,
            contribution(Fixed::ONE, rank, K).expect("fits"),
            "the bridge attached exactly the profile's contribution for rank {rank}"
        );
    }
    assert_eq!(
        fused.trailer.statuses.len(),
        1,
        "and the producer's own status survived the resumption"
    );

    // A stratum the profile does not weight has no contribution to make, and the
    // bridge says so by refusing to be built rather than by inventing one.
    let elsewhere = crate::profile(&[("elsewhere", Fixed::ONE)], K);
    assert!(
        RankedStreamAdapter::new(
            RankedStreamImpl::new(
                vec![(1, Term::new("a"), RowBlock::Undeclared)],
                StreamEnding::Exhausted,
            ),
            unique_items(),
            &elsewhere,
            &stratum("resume"),
        )
        .is_none(),
        "no weight, no contribution, no adapter"
    );
}

// ---------------------------------------------------------------------------
// 11b. The pinned plan travels with the rows, and a stream that changed plans
//      on the way is caught by the fusion rather than filed under the wrong one
// ---------------------------------------------------------------------------

/// Plan, compile and execute one single-stratum fixture, returning the plan it
/// was pinned to, its stratum, and the bridged stream — **untagged**, so each
/// test below states for itself which plan the stream claims to descend from.
fn executed_stream(suffix: &str, profile: &FusionProfile) -> (PlanId, Iri, RankedStreamAdapter) {
    let stratum_iri = ex(&format!("stratum/{suffix}"));
    let registry = single_registry(&stratum_iri, &ex(&format!("pf/{suffix}")), 10, 3);
    let stats = single_statistics(&stratum_iri, 10);
    let request = RetrievalRequest::complete(vec![lexical_term()]);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let planned = plan(&request, &registry, &stats).expect("plans");
    let compiled = compile(&planned, &env).expect("admits");
    let execution =
        block_on(execute(&compiled, &registry, &*common::empty_dataset())).expect("the units run");
    let stream = execution
        .streams
        .into_iter()
        .next()
        .expect("the stratum streamed");
    let adapter =
        RankedStreamAdapter::new(stream.stream, stream.contract, profile, &stream.stratum)
            .expect("the profile weights the stratum the plan reached");
    (stream.plan_id, stream.stratum, adapter)
}

#[test]
fn a_stream_that_names_another_plan_is_refused_rather_than_fused() {
    let profile = profile(
        &[("splice/left", Fixed::ONE), ("splice/right", Fixed::ONE)],
        K,
    );
    let (left_plan, left_stratum, left) = executed_stream("splice/left", &profile);
    let (right_plan, right_stratum, right) = executed_stream("splice/right", &profile);
    assert_ne!(
        left_plan, right_plan,
        "two distinct plans, or this fixture tests nothing"
    );

    // A stream spliced in from another plan. Its rows are perfectly well formed
    // — it is a real executed stream — so nothing about the rows can catch it.
    // The identity it carries is what does: one answer cannot descend from two
    // plans, and naming either one would be right about half the rows.
    let error = block_on(fuse::<RankedStreamAdapter, Term>(
        vec![
            (left_stratum, left.with_plan_id(left_plan)),
            (right_stratum, right.with_plan_id(right_plan)),
        ],
        &profile,
        TOP_K,
    ))
    .expect_err("two plans in one fusion is a refusal");
    assert!(
        matches!(
            &error,
            FusionError::PlanIdMismatch { expected, got: Some(got) }
                if *expected == left_plan && *got == right_plan
        ),
        "expected PlanIdMismatch, got {error:?}"
    );

    // The neighbour that must still succeed: the same two strata, agreeing on
    // one plan — which is what the two strata of a single plan actually do.
    // The answer names it, and the identity in the answer is the one the
    // streams carried.
    let (_, left_stratum, left) = executed_stream("splice/left", &profile);
    let (_, right_stratum, right) = executed_stream("splice/right", &profile);
    let agreed = block_on(fuse::<RankedStreamAdapter, Term>(
        vec![
            (left_stratum, left.with_plan_id(left_plan)),
            (right_stratum, right.with_plan_id(left_plan)),
        ],
        &profile,
        TOP_K,
    ))
    .expect("streams that agree on their plan fuse");
    assert!(!agreed.rows.is_empty(), "the fixture strata rank rows");
    assert_eq!(agreed.trailer.plan_id, Some(left_plan));

    // And the second neighbour, because the refusal must stay narrow: a stream
    // built outside the ladder names no plan at all, and fusing it with one that
    // does is not a disagreement. It fuses, and the answer honestly names no
    // pinned plan rather than borrowing its neighbour's.
    let (_, left_stratum, left) = executed_stream("splice/left", &profile);
    let (_, right_stratum, right) = executed_stream("splice/right", &profile);
    let mixed = block_on(fuse::<RankedStreamAdapter, Term>(
        vec![
            (left_stratum, left.with_plan_id(left_plan)),
            (right_stratum, right),
        ],
        &profile,
        TOP_K,
    ))
    .expect("a stream that descends from no plan is not a disagreement");
    assert_eq!(mixed.rows, agreed.rows, "and the rows are the same rows");
    assert_eq!(mixed.trailer.plan_id, None);
}

// ---------------------------------------------------------------------------
// 11c. The bound the streams were planned for travels with them too, and a
//      fusion run at another bound is refused rather than answered short
//
// This is the sibling of 11b, one field over. A depth derived for a bound of two
// rows is honest for two rows: the rows past it were never read, and nothing
// about the rows that WERE read says so. So the bound rides with them and `fuse`
// checks its own argument against it, exactly as it checks that the streams agree
// on a plan.
//
// The read really is cut to the bound here, which is what makes the refusal
// load-bearing rather than pedantic, and what cuts it is the producer's
// `DuplicatePolicy::Unique` declaration over a single surviving stratum: with no
// second stratum there is no pair for a disjointness premise to be about, so
// uniqueness alone makes a `k`-row prefix a top `k`. The test below measures that
// rather than asserting it — the same fixture declaring `Unrestricted` narrows
// identically, and the same fixture declaring `Allowed` does not narrow at all.
// ---------------------------------------------------------------------------

/// A registered producer declaring `domains` and `duplicates`, holding `count` of the
/// `rows` it registers.
///
/// The two declarations are parameters because which of them the narrowing rests on is
/// exactly what the section below measures; a fixture that fixed both could only ever
/// assert the answer.
fn registry_declaring(
    stratum_iri: &str,
    producer_iri: &str,
    rows: u64,
    count: usize,
    domains: CandidateDomains,
    duplicates: DuplicatePolicy,
) -> PropertyFunctionRegistry {
    let mut declaration = ranked(stratum_iri, vec![TermPattern::of_kind(TermKind::Any)], true);
    declaration.domains = domains;
    declaration.duplicates = duplicates;
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        producer_iri,
        make_producer(rows, "hand/", count),
        declaration,
    );
    registry
}

/// The section's own fixture: a producer restricted to one block of the candidate
/// universe, promising no repeats.
///
/// One tag rather than a set, because a single-block declaration entails the block of
/// every row and so needs no per-row block column. It is **not** what lets the planner
/// narrow this stratum's depth to the request's bound — at one surviving stratum the
/// disjointness premise is vacuous and uniqueness carries the prefix alone, which the
/// test below measures both ways. What the tag buys is that every row this fixture
/// streams carries an entailed block rather than an undeclared one, which is the shape
/// a restricted producer's stream really has.
fn registry_within_one_block(
    stratum_iri: &str,
    producer_iri: &str,
    rows: u64,
    count: usize,
) -> PropertyFunctionRegistry {
    registry_declaring(
        stratum_iri,
        producer_iri,
        rows,
        count,
        CandidateDomains::within([
            DomainTag::parse(&ex("block/only")).expect("the fixture domain tag is a valid IRI")
        ]),
        DuplicatePolicy::Unique,
    )
}

/// Plan, compile and execute one single-block fixture for a request bounded at
/// `top_k`, returning the recorded depth, the bound the bundle was compiled for,
/// its stratum, and the bridged stream tagged with that bound.
fn executed_stream_bounded(
    suffix: &str,
    profile: &FusionProfile,
    top_k: TopK,
) -> (u32, TopK, Iri, RankedStreamAdapter) {
    let stratum_iri = ex(&format!("stratum/{suffix}"));
    let registry = registry_within_one_block(&stratum_iri, &ex(&format!("pf/{suffix}")), 10, 3);
    let stats = single_statistics(&stratum_iri, 10);
    let request = RetrievalRequest::bounded(vec![lexical_term()], top_k);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let planned = plan(&request, &registry, &stats).expect("plans");
    let depth = planned.stratum_depths[&iri(&stratum_iri)];
    let compiled = compile(&planned, &env).expect("admits");
    let execution =
        block_on(execute(&compiled, &registry, &*common::empty_dataset())).expect("the units run");
    let stream = execution
        .streams
        .into_iter()
        .next()
        .expect("the stratum streamed");
    let adapter =
        RankedStreamAdapter::new(stream.stream, stream.contract, profile, &stream.stratum)
            .expect("the profile weights the stratum the plan reached")
            .with_fused_bound(stream.fused_bound);
    (depth, stream.fused_bound, stream.stratum, adapter)
}

#[test]
fn fusing_at_a_bound_the_streams_were_not_planned_for_is_refused_by_name() {
    let profile = profile(&[("bound", Fixed::ONE)], K);

    // The fixture must really have been cut by the bound, or the refusal below
    // would be protecting nothing: the producer holds three rows and declares ten,
    // and a bound of two is what the depth comes out as.
    let (depth, planned_bound, deeper_stratum, stream) =
        executed_stream_bounded("bound", &profile, TopK::new(2));
    assert_eq!(
        depth, 2,
        "the request's bound is what the depth was derived from"
    );
    assert_eq!(planned_bound, TopK::new(2));

    // WHICH DECLARATION BUYS THE CUT, measured rather than implied. The guard above
    // passes with the fixture's block declaration deleted, so on its own it says
    // nothing about what the narrowing rests on.
    let depth_declaring = |domains: CandidateDomains, duplicates: DuplicatePolicy| {
        let stratum_iri = ex("stratum/measured");
        let registry =
            registry_declaring(&stratum_iri, &ex("pf/measured"), 10, 3, domains, duplicates);
        let stats = single_statistics(&stratum_iri, 10);
        let request = RetrievalRequest::bounded(vec![lexical_term()], TopK::new(2));
        plan(&request, &registry, &stats)
            .expect("plans")
            .stratum_depths[&iri(&stratum_iri)]
    };
    assert_eq!(
        depth_declaring(CandidateDomains::Unrestricted, DuplicatePolicy::Unique),
        2,
        "a producer that declares NO block narrows to the bound exactly as the fixture \
         does, because one surviving stratum has no second one to overlap with"
    );
    assert_eq!(
        depth_declaring(
            CandidateDomains::within([
                DomainTag::parse(&ex("block/only")).expect("the fixture domain tag is valid")
            ]),
            DuplicatePolicy::Allowed
        ),
        10,
        "and a producer that may repeat does not narrow however it declares its blocks, \
         because a count of ranks is then not a count of candidates — so uniqueness is \
         what the cut above rests on"
    );

    let deeper = block_on(fuse::<RankedStreamAdapter, Term>(
        vec![(deeper_stratum, stream)],
        &profile,
        TopK::new(5),
    ))
    .expect_err("a read taken for two rows cannot answer a fusion for five");
    assert!(
        matches!(
            &deeper,
            FusionError::ReadBoundMismatch { planned, requested }
                if *planned == TopK::new(2) && *requested == TopK::new(5)
        ),
        "expected ReadBoundMismatch {{ planned: 2, requested: 5 }}, got {deeper:?}"
    );

    // A SHALLOWER bound is refused too, and the message is the same one. It would
    // answer correctly, out of a read deeper than the question needed — and then
    // the depth the bundle records, the resolution it reports and the identity it
    // carries would all be describing a different request.
    let (_, _, shallower_stratum, stream) =
        executed_stream_bounded("bound", &profile, TopK::new(2));
    let shallower = block_on(fuse::<RankedStreamAdapter, Term>(
        vec![(shallower_stratum, stream)],
        &profile,
        TopK::new(1),
    ))
    .expect_err("a bound below the planned one is still not the planned one");
    assert!(
        matches!(
            &shallower,
            FusionError::ReadBoundMismatch { planned, requested }
                if *planned == TopK::new(2) && *requested == TopK::new(1)
        ),
        "expected ReadBoundMismatch {{ planned: 2, requested: 1 }}, got {shallower:?}"
    );

    // THE NEIGHBOUR THAT MUST SUCCEED: the same streams, fused at the bound they
    // were planned for. A refusal that also swept this up would have made the
    // resumable seam unusable, which is the mirror failure of not checking at all.
    let (_, planned_bound, matched_stratum, stream) =
        executed_stream_bounded("bound", &profile, TopK::new(2));
    let matched = block_on(fuse::<RankedStreamAdapter, Term>(
        vec![(matched_stratum, stream)],
        &profile,
        planned_bound,
    ))
    .expect("streams fused at the bound they were planned for answer");
    assert_eq!(
        matched.rows.len(),
        2,
        "and they answer with the rows the bound asked for"
    );

    // THE SECOND NEIGHBOUR, because the refusal must stay narrow: a stream built
    // outside the ladder names no bound at all, so there is nothing for the
    // caller's argument to disagree with and it fuses at whatever was named.
    let unbounded = block_on(fuse::<ScriptedStream, Term>(
        vec![(
            stratum("bound"),
            ScriptedStream::new(vec![row(1, Fixed::ONE, K, "alpha")], exhausted(1)),
        )],
        &profile,
        TopK::new(9),
    ))
    .expect("a stream that no plan bounded is not a disagreement");
    assert_eq!(unbounded.rows.len(), 1);
}

#[test]
fn the_exported_bridge_reports_a_malformed_rank_rather_than_panicking() {
    // Ranks are 1-based. `execute` never emits a zero, but the bridge is public
    // and a caller can hand it a stream it built itself, so the violation must
    // come back as the protocol error fusion would raise for the same row.
    let profile = profile(&[("resume", Fixed::ONE)], K);
    let mut adapter = RankedStreamAdapter::new(
        RankedStreamImpl::new(
            vec![(0, Term::new("a"), RowBlock::Undeclared)],
            StreamEnding::Exhausted,
        ),
        unique_items(),
        &profile,
        &stratum("resume"),
    )
    .expect("the profile weights the stratum");
    assert!(
        matches!(
            block_on(adapter.next()),
            Err(ProtocolError::OutOfOrderRanks {
                expected: 1,
                got: 0
            })
        ),
        "a rank of zero is a protocol violation, reported rather than panicked on"
    );

    // The valid neighbour: a 1-based rank through the same adapter is an
    // ordinary row carrying the profile's own contribution.
    let mut adapter = RankedStreamAdapter::new(
        RankedStreamImpl::new(
            vec![(1, Term::new("a"), RowBlock::Undeclared)],
            StreamEnding::Exhausted,
        ),
        unique_items(),
        &profile,
        &stratum("resume"),
    )
    .expect("the profile weights the stratum");
    assert_eq!(
        block_on(adapter.next()).expect("a 1-based rank is well formed"),
        Some(RankedRow::new(
            1,
            contribution(Fixed::ONE, 1, K).expect("fits"),
            Term::new("a"),
            RowBlock::Undeclared
        ))
    );
}
