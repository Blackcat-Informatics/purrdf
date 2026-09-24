// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Search composition identity: `search` is exactly
//! `fuse ∘ execute ∘ compile ∘ plan`.
//!
//! The differential test drives the stage functions by hand with the same mock
//! registry, statistics and profile, and requires the composed `search` answer to
//! be byte-identical to that manual composition. The error tests pin each stage's
//! typed refusal to the matching [`SearchError`] variant, and the end-to-end test
//! proves a full mock search returns the expected rows and trailer. Fixtures use
//! `example.org` throughout.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use pretty_assertions::assert_eq;
use purrdf_core::{RdfDataset, TermValue};
use purrdf_retrieval::{
    AdmissionEnvironment, AdmissionError, DecayRule, ExecutionError, Fixed, FusionError,
    FusionProfile, Iri, Metric, PlanError, ProducerStatus, ProtocolError, RankFidelity,
    RankedStreamAdapter, ReadSchedule, RequestTerm, RetrievalRequest, SearchError, SearchResult,
    Statistics, StratumUnit, Term, TopK, UnservedReason, UnservedTerm, compile, execute_within,
    fuse, plan, search,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, CandidateDomains, DomainTag, DuplicatePolicy, EvalError,
    ExclusionBasis, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry,
    RankArithmetic, RankedDeclaration, RequestFacet, TermKind, TermPattern, TermPlacement,
    Volatility,
};

mod common;

const K: u32 = 60;

/// The row bound these fixtures search under.
///
/// Fused enumeration is top-k by construction, so every search states a bound.
/// This one is well above what the mocks can yield, so nothing below is decided
/// by the bound; `the_bound_is_the_bound_and_a_larger_one_refuses_nothing` is
/// where the bound's own behaviour is pinned.
const TOP_K: TopK = TopK::new(1024);

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
        fidelity: RankFidelity::EXACT,
        arithmetic: RankArithmetic::FloatFree,
        domains: CandidateDomains::Unrestricted,
        block_position: None,
        exclusion: ExclusionBasis::Unavailable,
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
    mixed_request_at(TOP_K)
}

/// The mixed request, bounded at `top_k`.
///
/// The bound is part of the request rather than an argument of `search`, so a
/// fixture that varies it varies the request — and, by construction, the plan.
fn mixed_request_at(top_k: TopK) -> RetrievalRequest {
    RetrievalRequest::bounded(vec![lexical_term(), vector_term(), seed_term()], top_k)
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

/// A mock producer of arity (1,2) that names, in a third argument position, the
/// block each of its rows was drawn from.
///
/// One row per entry in `blocks`, in that order. The extra position is what a
/// declaration's `block_position` points at, and it is the only way a producer
/// restricted to **several** blocks can back that restriction per row: one block
/// per row is entailed by a one-block declaration, and several blocks are not.
fn producer_naming_blocks(rows: u64, prefix: &str, blocks: &[&str]) -> Arc<dyn PropertyFunction> {
    let arity = PfArity::new(1, 2);
    let emitted = blocks
        .iter()
        .enumerate()
        .map(|(index, block)| {
            vec![
                TermValue::iri(format!("{}entity{index}", ex(prefix))),
                TermValue::iri(format!("{}score{index}", ex(prefix))),
                TermValue::iri((*block).to_owned()),
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

/// The fixture fusion profile: unit weight for each of the fixture's three
/// strata and smoothing `K`. Three weights is also what gives a candidate room
/// to surface in every stratum — the contribution maximum is the stratum count.
fn fixture_profile() -> FusionProfile {
    let mut weights = BTreeMap::new();
    weights.insert(iri(&ex("stratum/universal")), Fixed::ONE);
    weights.insert(iri(&ex("stratum/text")), Fixed::ONE);
    weights.insert(iri(&ex("stratum/graph")), Fixed::ONE);
    FusionProfile::with_decay(weights, DecayRule::ReciprocalRank { k: K })
        .expect("the fixture profile is valid")
}

fn fixture_env<'a>(
    registry: &'a PropertyFunctionRegistry,
    stats: &'a MockStatistics,
) -> AdmissionEnvironment<'a> {
    AdmissionEnvironment {
        registry,
        statistics: stats,
        fusion_profile: None,
    }
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

// ---------------------------------------------------------------------------
// The hand-composed pipeline, assembled from the exported seam
// ---------------------------------------------------------------------------

/// Drive `fuse ∘ execute ∘ compile ∘ plan` by hand and assemble the same
/// [`SearchResult`] `search` would.
///
/// Everything a caller cannot reasonably re-derive is taken from the crate's own
/// exports: [`RankedStreamAdapter`] bridges each executed stream to the fusion
/// protocol, and [`purrdf_retrieval::FusionTrailer::completed_with`] carries the
/// executor's statuses into the terminal report. Re-deriving either here would
/// make this test compare `search` against a copy of itself — the drift a seam
/// exists to prevent — instead of against the pipeline a caller can write.
// The manual composition mirrors `search`, including its single-task,
// runtime-agnostic future; see the same allow on `search` itself.
#[allow(clippy::future_not_send)]
async fn manual_composition(
    request: &RetrievalRequest,
    registry: &PropertyFunctionRegistry,
    stats: &MockStatistics,
    dataset: &RdfDataset,
    env: &AdmissionEnvironment<'_>,
    profile: &FusionProfile,
) -> SearchResult {
    let planned = plan(request, registry, stats).expect("the fixture request plans");
    // The waist is held to the law this composition is about to fuse under, which
    // is exactly what `search` does with the environment it is handed: the plan is
    // unchanged and the planner still never sees the profile, but admission can
    // now say what each planned depth costs in rank resolution. A caller composing
    // by hand re-forms the environment here for the same reason.
    let env = AdmissionEnvironment {
        registry: env.registry,
        statistics: env.statistics,
        fusion_profile: Some(profile),
    };
    let compiled = compile(&planned, &env).expect("a fresh plan is admitted");
    // The schedule `search` reads under, which a caller composing by hand names
    // as well: each stratum is one invocation held open and read as far as the
    // fusion pulls it.
    let execution = execute_within(&compiled, registry, dataset, ReadSchedule::OnDemand)
        .await
        .expect("the fixture registry executes");
    let mut statuses = execution.statuses;

    let mut streams = Vec::new();
    let mut unweighted_strata = Vec::new();
    for mut stream in execution.streams {
        if profile.weight(&stream.stratum).is_none() {
            // Not fused: its status is how its read ended, and a read produced on
            // demand has ended only once it is read, so it is read to its end.
            while stream
                .stream
                .next()
                .await
                .expect("an unweighted stratum reads cleanly")
                .is_some()
            {}
            let receipt = stream
                .stream
                .receipt()
                .await
                .expect("a stream read to its end has a receipt");
            // Its ending is a claim about the read its witness stands behind, so it
            // settles exactly as a fused stream does, and is held to what it
            // announced before its first row.
            let settled = stream
                .stream
                .settle()
                .await
                .expect("an unweighted stratum's witness is one attestation");
            assert_eq!(
                settled.as_ref(),
                Some(&stream.attestation),
                "an unweighted stratum settles to what it announced"
            );
            statuses
                .entry(stream.stratum.clone())
                .or_insert_with(|| ProducerStatus::from(receipt));
            unweighted_strata.push(stream.stratum);
            continue;
        }
        let plan_id = stream.plan_id;
        let fused_bound = stream.fused_bound;
        let attestation = stream.attestation.clone();
        match RankedStreamAdapter::new(stream.stream, stream.contract, profile, &stream.stratum) {
            // The plan the unit was compiled from rides on with the rows, which
            // is how the trailer comes to name it — and so does what the index
            // behind those rows attested, which is what the trailer's exactness
            // and evidence identity are derived from.
            Some(adapter) => streams.push((
                stream.stratum,
                adapter
                    .with_plan_id(plan_id)
                    .with_fused_bound(fused_bound)
                    .with_attestation(attestation),
            )),
            None => unweighted_strata.push(stream.stratum),
        }
    }
    unweighted_strata.sort();

    let fused = fuse::<RankedStreamAdapter<'_>, Term>(streams, profile, compiled.fused_bound)
        .await
        .expect("the surviving streams fuse");
    let trailer = fused.trailer.completed_with(statuses);
    let evidence_id = trailer.evidence_id;
    SearchResult {
        rows: fused.rows,
        trailer,
        unserved_terms: planned.unserved_evidence(),
        plan_id: planned.id(),
        evidence_id,
        planned_resolution: compiled.resolution,
        profile_id: profile.id(),
        unweighted_strata,
    }
}

/// Render a [`SearchResult`] as deterministic bytes: rows in final order with
/// their provenance, then statuses by stratum, then the pinned identities.
/// Nothing here depends on a `HashMap`.
fn render(result: &SearchResult) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "plan {}", result.plan_id.to_hex());
    let _ = writeln!(out, "profile {}", result.profile_id.to_hex());
    for row in &result.rows {
        let _ = writeln!(
            out,
            "row {} score={} witness={}",
            row.entity.as_str(),
            row.score.to_decimal_lexical(),
            row.threshold_witness.to_decimal_lexical()
        );
        for (stratum, rank, score) in &row.contributions {
            let _ = writeln!(
                out,
                "  {} rank={rank} score={}",
                stratum.as_str(),
                score.to_decimal_lexical()
            );
        }
    }
    for (stratum, status) in &result.trailer.statuses {
        let _ = writeln!(out, "status {} {status:?}", stratum.as_str());
    }
    for stratum in &result.unweighted_strata {
        let _ = writeln!(out, "unweighted {}", stratum.as_str());
    }
    for unserved in &result.unserved_terms {
        let _ = writeln!(
            out,
            "unserved term={} {:?}",
            unserved.request_term, unserved.reason
        );
    }
    for (stratum, planned) in &result.planned_resolution {
        let _ = writeln!(
            out,
            "planned-resolution {} separation={:?} depth={} separated={}",
            stratum.as_str(),
            planned.separation,
            planned.requested_depth,
            planned.fully_separated()
        );
    }
    let _ = writeln!(
        out,
        "trailer-profile {}",
        result.trailer.profile_id.to_hex()
    );
    // The evidence the answer carries about the indexes that served it. Rendered
    // rather than only structurally compared, because this is the part of the
    // answer that decides whether a score may be read as a number, and a path
    // that silently lost it would otherwise differ only inside an opaque digest.
    for (stratum, attestation) in &result.trailer.attestations {
        let _ = writeln!(
            out,
            "attestation {} generation={:?} service={:?}",
            stratum.as_str(),
            attestation.generation,
            attestation.service
        );
    }
    let _ = writeln!(out, "exactness {:?}", result.trailer.exactness);
    let _ = writeln!(out, "evidence {}", result.evidence_id.to_hex());
    out
}

// ---------------------------------------------------------------------------
// 1. Composition identity
// ---------------------------------------------------------------------------

#[test]
fn search_equals_manual_composition() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let profile = fixture_profile();
    let request = mixed_request();
    let dataset = common::empty_dataset();

    let direct = block_on(search(&request, &registry, &stats, dataset, &env, &profile))
        .expect("the composed search answers");
    let manual = block_on(manual_composition(
        &request, &registry, &stats, dataset, &env, &profile,
    ));

    assert_eq!(
        render(&direct).as_bytes(),
        render(&manual).as_bytes(),
        "search must be byte-for-byte the manual composition"
    );

    // The evidence about the indexes, named claim by claim. The rendering above
    // already covers it, but a failure there reads as one long diff; these say
    // which of the three facts moved — and all three are the difference between
    // a score a caller may compare and one that is only a lower bound.
    assert_eq!(
        direct.trailer.attestations, manual.trailer.attestations,
        "what each stratum's index attested must reach both paths identically"
    );
    assert_eq!(
        direct.trailer.exactness, manual.trailer.exactness,
        "and therefore so must whether the fused scores are exact"
    );
    assert_eq!(
        direct.evidence_id, manual.evidence_id,
        "and so must the digest of that evidence"
    );
    assert_eq!(
        direct.evidence_id, direct.trailer.evidence_id,
        "the answer's evidence identity is read off its own trailer, never \
         recomputed beside it"
    );

    assert_eq!(direct, manual, "and structurally the same answer");
}

// ---------------------------------------------------------------------------
// 2. Error propagation
// ---------------------------------------------------------------------------

#[test]
fn plan_error_propagates() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let profile = fixture_profile();

    // An empty request reaches no producer, so planning refuses it.
    let error = block_on(search(
        &RetrievalRequest::bounded(Vec::new(), TOP_K),
        &registry,
        &stats,
        common::empty_dataset(),
        &env,
        &profile,
    ))
    .expect_err("an empty request does not plan");
    assert!(
        matches!(
            error,
            SearchError::PlanError(PlanError::NoApplicableProducers)
        ),
        "expected the planner's refusal, got {error:?}"
    );
}

#[test]
fn admission_error_propagates() {
    // Two registries with identical declarations share a content fingerprint but
    // have distinct live instance identities; planning against one and admitting
    // against the other is the admission refusal.
    let planned_against = fixture_registry();
    let admitted_against = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&admitted_against, &stats);
    let profile = fixture_profile();

    let error = block_on(search(
        &mixed_request(),
        &planned_against,
        &stats,
        common::empty_dataset(),
        &env,
        &profile,
    ))
    .expect_err("a different live registry instance is refused at admission");
    assert!(
        matches!(
            error,
            SearchError::AdmissionError(AdmissionError::RegistryMismatch { .. })
        ),
        "expected an admission refusal, got {error:?}"
    );
}

#[test]
fn execution_error_maps_to_search_error() {
    // Admission pins both the planner's registry and the executor's registry to
    // the environment's live instance, so a whole-execution failure is not
    // reachable end-to-end through `search` — the mapping itself is what carries
    // `execute`'s error out unchanged when `execute` does refuse.
    let first = fixture_registry();
    let second = fixture_registry();
    let error: SearchError = ExecutionError::RegistryMismatch {
        expected: first.instance_id(),
        got: second.instance_id(),
    }
    .into();
    assert!(
        matches!(error, SearchError::ExecutionError(_)),
        "the executor's error propagates unchanged, got {error:?}"
    );
}

#[test]
fn fusion_error_propagates() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    // A profile whose only weight names a stratum this plan never reaches. Every
    // stratum that ran is unweighted, so nothing that ran can contribute and
    // there is no partial answer to return.
    let disjoint = FusionProfile::with_decay(
        BTreeMap::from([(iri(&ex("stratum/elsewhere")), Fixed::ONE)]),
        DecayRule::ReciprocalRank { k: K },
    )
    .expect("the disjoint profile is valid");

    let error = block_on(search(
        &mixed_request(),
        &registry,
        &stats,
        common::empty_dataset(),
        &env,
        &disjoint,
    ))
    .expect_err("a profile that weights none of the executed strata is refused");
    match error {
        SearchError::FusionError(FusionError::UnknownStratum { stratum }) => {
            // Reported in ascending stratum order, so `stratum/graph` is named.
            assert_eq!(
                stratum,
                ex("stratum/graph"),
                "the unweighted stratum is named exactly"
            );
        }
        other => panic!("expected a fusion refusal, got {other:?}"),
    }
}

#[test]
fn search_answers_under_a_profile_that_cannot_separate_every_planned_rank() {
    // `search` is the one place the plan and the law meet before a row is read,
    // so it re-forms the admission environment around that law. This fixture is
    // the case that used to be refused: a per-stratum depth of a hundred outruns
    // what a weight of `10^-9` can still order, so the deepest ranks fuse at a
    // coarser resolution.
    //
    // Coarser is not wrong. The declared tie-break is total, so the answer is a
    // pure function of its inputs at every depth, and refusing it rejected a
    // perfectly usable answer over a property of the consumer's own arithmetic.
    // `Fixed::from_raw(1_000)` is one thousand raw units of `10^-12`, which is
    // the sub-unit weight this fixture needs; a weight of *one* is `Fixed::ONE`.
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let shallow = FusionProfile::with_decay(
        BTreeMap::from([(iri(&ex("stratum/text")), Fixed::from_raw(1_000))]),
        DecayRule::ReciprocalRank { k: 1 },
    )
    .expect("a strictly positive weight is a valid profile");

    // The bound is real and this fixture is genuinely past it — otherwise the
    // test would pass without exercising anything.
    let separation = shallow
        .monotone_depth(&iri(&ex("stratum/text")))
        .expect("the profile weights this stratum");
    assert!(
        !separation.covers(100),
        "this fixture must plan past the profile's separating depth, got {separation:?}"
    );

    block_on(search(
        &mixed_request(),
        &registry,
        &stats,
        common::empty_dataset(),
        &env,
        &shallow,
    ))
    .expect("a depth past the profile's separating range still answers");

    // The neighbouring case: the fixture profile's unit weights order far deeper
    // than any depth this plan records, and the same search answers.
    assert!(
        block_on(search(
            &mixed_request(),
            &registry,
            &stats,
            common::empty_dataset(),
            &env,
            &fixture_profile(),
        ))
        .is_ok(),
        "a profile whose arithmetic covers every recorded depth still answers"
    );
}

// ---------------------------------------------------------------------------
// 2a-bis. The waist's resolution evidence reaches the answer
//
// `search` is the only entry point that plans and executes in one call, so it is
// the only one where a caller cannot stop at the compiled bundle to read what the
// plan's depths were going to cost. The pair below holds it to carrying that
// evidence forward: the criterion is a profile whose plan is NOT fully separated,
// and the neighbour is one that is — which must report itself as separated rather
// than as an absence.
// ---------------------------------------------------------------------------

/// The compiled bundle a `search` under `profile` would produce, for comparing
/// the answer's planned resolution against the plan that produced it.
///
/// It re-forms the environment exactly as `search` does, because the resolution
/// is a property of the plan *and* the law: an environment naming no profile has
/// nothing to measure against and would report nothing at all.
fn compiled_under(
    registry: &PropertyFunctionRegistry,
    stats: &MockStatistics,
    profile: &FusionProfile,
) -> purrdf_retrieval::CompiledRetrieval {
    let planned = plan(&mixed_request(), registry, stats).expect("the fixture request plans");
    let env = AdmissionEnvironment {
        registry,
        statistics: stats,
        fusion_profile: Some(profile),
    };
    compile(&planned, &env).expect("a fresh plan is admitted")
}

#[test]
fn search_reports_the_planned_resolution_the_compiled_plan_recorded() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    // The same sub-unit weight as the fixture above: `10^-9` cannot separate a
    // hundred ranks under the truncated rule, so this plan is admitted and
    // reported as coarse rather than refused.
    let shallow = FusionProfile::with_decay(
        BTreeMap::from([(iri(&ex("stratum/text")), Fixed::from_raw(1_000))]),
        DecayRule::ReciprocalRank { k: 1 },
    )
    .expect("a strictly positive weight is a valid profile");

    let result = block_on(search(
        &mixed_request(),
        &registry,
        &stats,
        common::empty_dataset(),
        &env,
        &shallow,
    ))
    .expect("a depth past the profile's separating range still answers");

    // The criterion: what the answer reports is what the admission waist
    // recorded, not a number `search` derived a second time.
    let compiled = compiled_under(&registry, &stats, &shallow);
    assert_eq!(
        result.planned_resolution, compiled.resolution,
        "the answer carries the compiled plan's own resolution map"
    );
    let planned = result
        .planned_resolution
        .get(&iri(&ex("stratum/text")))
        .cloned()
        .expect("the weighted stratum's planned resolution is on the answer");
    assert_eq!(
        planned.requested_depth, 100,
        "the depth reported is the one the PLAN recorded for the stratum"
    );
    assert!(
        !planned.fully_separated(),
        "a hundred ranks at a weight of 10^-9 is not fully separated"
    );

    // …and it is emphatically not the trailer's observed number wearing another
    // name. The mock yields two rows, so fusion pulled two ranks and observed no
    // collision at all: the plan's estimate and the run's outcome disagree here
    // precisely because a depth nothing reached cost nothing.
    let observed = result
        .trailer
        .resolution
        .get(&iri(&ex("stratum/text")))
        .copied()
        .expect("the fused stratum's observed resolution is in the trailer");
    assert_eq!(observed.ranks_pulled, 2, "the mock holds two rows");
    assert_eq!(
        observed.collisions_observed, 0,
        "two ranks is far short of where this law stops separating"
    );
    assert_ne!(
        u64::from(planned.requested_depth),
        observed.ranks_pulled,
        "the planned depth and the ranks actually pulled are two different facts"
    );

    // A stratum with no weight has no contribution and so no resolution; it is
    // named as unweighted instead, never reported at a fabricated depth.
    assert_eq!(
        result.planned_resolution.keys().collect::<Vec<_>>(),
        vec![&iri(&ex("stratum/text"))],
        "only the weighted stratum has a resolution to report"
    );
    assert_eq!(
        result.unweighted_strata,
        vec![iri(&ex("stratum/graph")), iri(&ex("stratum/universal"))],
        "the strata this profile does not weight are named, not silently resolved"
    );
}

#[test]
fn a_fully_separated_plan_reports_itself_as_separated_rather_than_as_an_absence() {
    // The neighbouring valid case. The fixture profile's unit weights order far
    // deeper than any depth this plan records, and the honest report for that is
    // an entry saying so — not a missing entry, which is what a stratum with no
    // weight looks like, and not a refusal.
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let profile = fixture_profile();

    let result = block_on(search(
        &mixed_request(),
        &registry,
        &stats,
        common::empty_dataset(),
        &env,
        &profile,
    ))
    .expect("a profile whose arithmetic covers every recorded depth answers");

    let compiled = compiled_under(&registry, &stats, &profile);
    assert_eq!(result.planned_resolution, compiled.resolution);
    assert_eq!(
        result.planned_resolution.keys().collect::<Vec<_>>(),
        vec![
            &iri(&ex("stratum/graph")),
            &iri(&ex("stratum/text")),
            &iri(&ex("stratum/universal")),
        ],
        "every stratum the plan reached is weighted, so every one has an entry"
    );
    for (stratum, planned) in &result.planned_resolution {
        assert!(
            planned.fully_separated(),
            "a unit weight separates every rank {stratum} plans to read ({planned:?})"
        );
        assert!(
            planned.requested_depth > 0,
            "the entry names the depth it separates, not merely that it does"
        );
    }
    assert!(
        result.unweighted_strata.is_empty(),
        "nothing was set aside, so nothing is missing from the resolution map"
    );
}

// ---------------------------------------------------------------------------
// 2b. A profile that weights only some of the plan's strata
//
// The profile is deliberately not a planning input, so a plan's strata and a
// profile's weights are two independent lists that can legitimately disagree.
// The pair below is the whole policy: a partial overlap answers from the strata
// the profile weights and names the rest, and only a disjoint profile refuses.
// Refusing the partial case too would throw away every stratum the profile did
// weight in order to report one it did not.
// ---------------------------------------------------------------------------

#[test]
fn a_profile_that_weights_some_strata_answers_from_those_and_names_the_rest() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    // Weights one of the three strata the plan reaches. The other two ran and
    // produced rows; they have no weight, so they have no contribution.
    let narrow = FusionProfile::with_decay(
        BTreeMap::from([(iri(&ex("stratum/text")), Fixed::ONE)]),
        DecayRule::ReciprocalRank { k: K },
    )
    .expect("the narrow profile is valid");

    let result = block_on(search(
        &mixed_request(),
        &registry,
        &stats,
        common::empty_dataset(),
        &env,
        &narrow,
    ))
    .expect("the weighted stratum still answers");

    assert_eq!(
        result.unweighted_strata,
        vec![iri(&ex("stratum/graph")), iri(&ex("stratum/universal"))],
        "the strata the profile does not weight are reported, in stratum order"
    );
    assert_eq!(
        result.rows.len(),
        2,
        "the text producer's two rows are the whole answer"
    );
    for row in &result.rows {
        for (stratum, _, _) in &row.contributions {
            assert_eq!(
                *stratum,
                iri(&ex("stratum/text")),
                "only the weighted stratum contributes"
            );
        }
    }
    // Every stratum that ran has a status, including the two the profile did not
    // weight: they answered, and "the profile did not score this" is not the
    // same fact as "this could not answer". The fusion verified one of them; the
    // executor's own report carries the other two into the trailer.
    assert_eq!(
        result.trailer.statuses.len(),
        3,
        "an unweighted stratum still ran, and the trailer says how it ended"
    );
    assert!(matches!(
        result.trailer.statuses.get(&iri(&ex("stratum/universal"))),
        Some(ProducerStatus::Exhausted { rows_emitted: 3 })
    ));
}

#[test]
fn a_profile_that_weights_every_stratum_sets_nothing_aside() {
    // The neighbouring valid case of the refusal above and of the report above:
    // when the profile covers the plan, the report is empty and every stratum
    // answers.
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let profile = fixture_profile();

    let result = block_on(search(
        &mixed_request(),
        &registry,
        &stats,
        common::empty_dataset(),
        &env,
        &profile,
    ))
    .expect("the fixture search answers");
    assert!(
        result.unweighted_strata.is_empty(),
        "nothing is set aside when the profile weights every stratum, got {:?}",
        result.unweighted_strata
    );
    assert_eq!(result.trailer.statuses.len(), 3);
}

// ---------------------------------------------------------------------------
// 3. End to end
// ---------------------------------------------------------------------------

#[test]
fn end_to_end_search_returns_expected_results() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let profile = fixture_profile();
    let request = mixed_request();

    let result = block_on(search(
        &request,
        &registry,
        &stats,
        common::empty_dataset(),
        &env,
        &profile,
    ))
    .expect("the fixture search answers");
    let expected_plan = plan(&request, &registry, &stats).expect("the request plans");

    assert_eq!(result.plan_id, expected_plan.id());
    assert_eq!(result.profile_id, profile.id());
    assert_eq!(result.trailer.profile_id, profile.id());
    assert_eq!(
        result.rows.len(),
        6,
        "3 universal + 2 text + 1 graph distinct candidates"
    );

    // Order is score-descending and each row's score is its provenance's sum.
    for pair in result.rows.windows(2) {
        assert!(
            pair[0].score >= pair[1].score,
            "rows are ordered score-descending"
        );
    }
    for row in &result.rows {
        assert!(
            !row.contributions.is_empty(),
            "every emitted row has provenance"
        );
        let sum = row
            .contributions
            .iter()
            .fold(Fixed::ZERO, |total, (_, _, value)| {
                total.checked_add(*value).expect("no fixed-point overflow")
            });
        assert_eq!(sum, row.score, "contributions sum to the fused score");
    }

    // The trailer reports each contributing stratum's own status.
    assert_eq!(result.trailer.statuses.len(), 3);
    assert!(matches!(
        result.trailer.statuses.get(&iri(&ex("stratum/universal"))),
        Some(ProducerStatus::Exhausted { rows_emitted: 3 })
    ));
    assert!(matches!(
        result.trailer.statuses.get(&iri(&ex("stratum/text"))),
        Some(ProducerStatus::Exhausted { rows_emitted: 2 })
    ));
    assert!(matches!(
        result.trailer.statuses.get(&iri(&ex("stratum/graph"))),
        Some(ProducerStatus::Exhausted { rows_emitted: 1 })
    ));
}

// ---------------------------------------------------------------------------
// 4. A stratum that could not answer is named beside the ones that did
//
// §6 of the design record: a fused answer carries the status of every producer
// that contributed AND of every applicable producer that could not, never
// reduced to one aggregate flag. Fusion is where that information can die,
// because a failed stratum has no stream to fuse — so the answer is where it
// must be found.
// ---------------------------------------------------------------------------

/// A ranked producer that cannot run at all: it refuses to open a cursor.
///
/// This is the stratum-level failure the executor isolates — every other
/// stratum still runs — and the status a caller must not have to go back to
/// `execute` to discover.
struct FailingProducer {
    arity: PfArity,
    mode: BindingPattern,
}

impl PropertyFunction for FailingProducer {
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
        10
    }

    fn open(
        &self,
        _args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        Err(EvalError::data("the fixture index is unavailable"))
    }
}

/// The fixture registry plus one producer whose stratum cannot run.
fn registry_with_a_failing_stratum() -> PropertyFunctionRegistry {
    let mut registry = fixture_registry();
    let arity = PfArity::new(1, 1);
    registry.register_ranked(
        ex("pf/broken"),
        Arc::new(FailingProducer {
            arity,
            mode: arity.all_free_mode(),
        }),
        ranked(
            &ex("stratum/broken"),
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

/// The fixture profile, plus a weight for the stratum that cannot run: the
/// profile declares it applicable, which is what makes its silence a fact about
/// the producer rather than about the law.
fn profile_with_the_failing_stratum() -> FusionProfile {
    let mut weights = BTreeMap::new();
    weights.insert(iri(&ex("stratum/universal")), Fixed::ONE);
    weights.insert(iri(&ex("stratum/text")), Fixed::ONE);
    weights.insert(iri(&ex("stratum/graph")), Fixed::ONE);
    weights.insert(iri(&ex("stratum/broken")), Fixed::ONE);
    FusionProfile::with_decay(weights, DecayRule::ReciprocalRank { k: K })
        .expect("the fixture profile is valid")
}

fn statistics_with_the_failing_stratum() -> MockStatistics {
    let mut stats = statistics("r1");
    stats.cardinalities.insert(iri(&ex("stratum/broken")), 10);
    stats
}

#[test]
fn a_stratum_that_could_not_answer_is_named_in_the_trailer_beside_those_that_did() {
    let registry = registry_with_a_failing_stratum();
    let stats = statistics_with_the_failing_stratum();
    let env = fixture_env(&registry, &stats);
    let profile = profile_with_the_failing_stratum();

    let result = block_on(search(
        &mixed_request(),
        &registry,
        &stats,
        common::empty_dataset(),
        &env,
        &profile,
    ))
    .expect("the strata that can answer still answer");

    // Per producer, typed, never a flag: three succeeded and one could not, and
    // the answer says which is which.
    assert_eq!(
        result.trailer.statuses.len(),
        4,
        "every stratum the plan reached has its own status: {:?}",
        result.trailer.statuses
    );
    match result.trailer.statuses.get(&iri(&ex("stratum/broken"))) {
        Some(ProducerStatus::ExecutionFailed { reason }) => assert!(
            reason.contains("unavailable"),
            "the producer's own reason is carried verbatim: {reason}"
        ),
        other => panic!("expected the failed stratum's own status, got {other:?}"),
    }
    for stratum in ["stratum/universal", "stratum/text", "stratum/graph"] {
        assert!(
            matches!(
                result.trailer.statuses.get(&iri(&ex(stratum))),
                Some(ProducerStatus::Exhausted { .. })
            ),
            "{stratum} answered and says so: {:?}",
            result.trailer.statuses.get(&iri(&ex(stratum)))
        );
    }
    assert!(
        !result.rows.is_empty(),
        "the strata that could answer still produced the answer"
    );
    assert!(
        result
            .rows
            .iter()
            .flat_map(|row| &row.contributions)
            .all(|(stratum, _, _)| *stratum != iri(&ex("stratum/broken"))),
        "and the stratum that could not answer contributed no rows"
    );
}

#[test]
fn every_contributor_is_named_when_none_fail() {
    // The neighbouring valid case: with nothing failing, the same report names
    // every stratum as having answered. "Three answered" and "three answered and
    // a fourth could not" are different answers, read from the same place.
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let profile = fixture_profile();

    let result = block_on(search(
        &mixed_request(),
        &registry,
        &stats,
        common::empty_dataset(),
        &env,
        &profile,
    ))
    .expect("the fixture search answers");

    assert_eq!(result.trailer.statuses.len(), 3);
    assert!(
        result
            .trailer
            .statuses
            .values()
            .all(|status| matches!(status, ProducerStatus::Exhausted { .. })),
        "every contributor answered: {:?}",
        result.trailer.statuses
    );
}

// ---------------------------------------------------------------------------
// 5. A request term that reached no producer
// ---------------------------------------------------------------------------

/// A registry with one literal producer and nothing else, so a vector term in
/// the same request reaches no declaration at all.
fn literal_only_registry() -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/literal"),
        producer(100, "text/", 2),
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

fn text_only_profile() -> FusionProfile {
    FusionProfile::with_decay(
        BTreeMap::from([(iri(&ex("stratum/text")), Fixed::ONE)]),
        DecayRule::ReciprocalRank { k: K },
    )
    .expect("the fixture profile is valid")
}

#[test]
fn a_term_that_reached_no_producer_is_visible_in_the_plan_and_in_the_answer() {
    let registry = literal_only_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let profile = text_only_profile();
    // The lexical term reaches the one producer; the vector term reaches
    // nothing, because no declaration in this registry accepts that shape.
    let request = RetrievalRequest::bounded(vec![lexical_term(), vector_term()], TOP_K);

    let planned = plan(&request, &registry, &stats).expect("the lexical term plans");
    assert_eq!(
        planned.unserved_terms,
        vec![UnservedTerm {
            request_term: 1,
            reason: UnservedReason::NoProducerAccepts,
        }],
        "the plan records the term nothing accepted"
    );

    let result = block_on(search(
        &request,
        &registry,
        &stats,
        common::empty_dataset(),
        &env,
        &profile,
    ))
    .expect("the served term still answers");

    assert_eq!(
        result.unserved_terms,
        vec![UnservedTerm {
            request_term: 1,
            reason: UnservedReason::NoProducerAccepts,
        }],
        "and the answer carries it, typed, rather than omitting it quietly"
    );
    // The valid neighbour, in the same answer: the term that DID reach a
    // producer is not reported, and its rows are the answer.
    assert!(
        result
            .unserved_terms
            .iter()
            .all(|entry| entry.request_term != 0),
        "a term that reached a producer is never reported unserved"
    );
    assert_eq!(
        result.rows.len(),
        2,
        "the literal producer's two rows are the answer"
    );
}

#[test]
fn a_request_every_term_of_which_reached_a_producer_reports_nothing_unserved() {
    // The neighbouring valid case at the answer. An empty list is a claim too,
    // and the claim is strong: every term reached a producer *with its content*,
    // so the emitted text carries all of it. The needle reaches `pf/literal`,
    // which places it, and the seed reaches `pf/iri`, which places it — and the
    // catch-all's placement-free acceptance of both adds nothing either way.
    //
    // The mixed request is deliberately NOT used here: its vector term reaches
    // only the placement-free catch-all, so the honest evidence for it is not
    // empty. `a_request_carrying_a_term_only_a_placement_free_producer_accepts_
    // reports_it` is that case.
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let profile = fixture_profile();
    let request = RetrievalRequest::bounded(vec![lexical_term(), seed_term()], TOP_K);

    let result = block_on(search(
        &request,
        &registry,
        &stats,
        common::empty_dataset(),
        &env,
        &profile,
    ))
    .expect("the fixture search answers");
    assert!(
        result.unserved_terms.is_empty(),
        "every term reached a producer that places it, got {:?}",
        result.unserved_terms
    );
    // And that claim is checkable against the text: both constants are in it.
    let compiled = compile(
        &plan(&request, &registry, &stats).expect("plans"),
        &fixture_env(&registry, &stats),
    )
    .expect("admits");
    let all: String = compiled.units.iter().map(StratumUnit::sparql).collect();
    assert!(
        all.contains("\"quick brown fox\"@en") && all.contains(&format!("<{}>", ex("seed"))),
        "an empty unserved list means the text carries every term: {all}"
    );
}

#[test]
fn a_request_carrying_a_term_only_a_placement_free_producer_accepts_reports_it() {
    // The other side of the same claim, and the one an empty list would have
    // been wrong about. The mixed request's vector term is matched by the
    // catch-all's unconstrained pattern, which declares no placement — so the
    // catch-all is called with the embedding absent from its arguments, and the
    // answer reports the term rather than counting it served.
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let profile = fixture_profile();

    let result = block_on(search(
        &mixed_request(),
        &registry,
        &stats,
        common::empty_dataset(),
        &env,
        &profile,
    ))
    .expect("the two served terms still answer");
    assert_eq!(
        result.unserved_terms,
        vec![UnservedTerm {
            request_term: 1,
            reason: UnservedReason::AcceptedWithoutPlacement,
        }],
        "the embedding reached a producer that declared nowhere to put it"
    );
    assert!(
        !result.rows.is_empty(),
        "and the terms that WERE placed still answer; this is not a refusal"
    );
}

// ---------------------------------------------------------------------------
// 6. The bound
// ---------------------------------------------------------------------------

#[test]
fn the_bound_is_the_bound_and_a_larger_one_refuses_nothing() {
    let registry = fixture_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let profile = fixture_profile();

    // The bound travels in the request, so varying it varies the request — and a
    // request that asks for a different number of rows is planned as a different
    // plan. That is the point of the identity, not an accident of it.
    let run = |top_k| {
        block_on(search(
            &mixed_request_at(top_k),
            &registry,
            &stats,
            common::empty_dataset(),
            &env,
            &profile,
        ))
        .expect("the fixture search answers")
    };

    let whole = run(TOP_K);
    assert_eq!(whole.rows.len(), 6, "six distinct candidates rank");

    let bounded = run(TopK::new(3));
    assert_eq!(bounded.rows.len(), 3, "at most k rows");
    assert_eq!(
        bounded.rows,
        whole.rows[..3].to_vec(),
        "and they are the top of the same declared order"
    );
    for pair in bounded.rows.windows(2) {
        assert!(pair[0].score >= pair[1].score, "score descending");
    }

    // Over-refusal is as severe as a wrong answer: a bound above what ranked is
    // answered with everything that ranked, not with an error and not with a
    // truncation.
    let generous = run(TopK::new(1_000));
    assert_eq!(generous.rows, whole.rows);

    // And under the smallest bound there is, every stratum still has a status —
    // a status that tells the truth about the bound rather than one copied from
    // the unbounded run. A search that answered "every producer was exhausted"
    // after reading one row from each would have had to read every row of every
    // stream to say it, and it would have been wrong anyway.
    let none = run(TopK::new(0));
    assert!(
        none.rows.is_empty(),
        "a bound of zero certifies no rows, got {:?}",
        none.rows
    );
    assert_eq!(
        none.trailer.statuses.keys().collect::<Vec<_>>(),
        whole.trailer.statuses.keys().collect::<Vec<_>>(),
        "the same producers are named whatever the bound"
    );
    assert_ne!(
        none.trailer.plan_id, whole.trailer.plan_id,
        "a bound is a planning input, so the two runs descend from DIFFERENT plans \
         — which is exactly what a plan identity sensitive to the bound is for"
    );
    assert_eq!(
        plan(&mixed_request_at(TopK::new(0)), &registry, &stats)
            .expect("the fixture request plans")
            .id(),
        none.trailer
            .plan_id
            .expect("the streams carried their plan"),
        "and each answer names the plan its own bound was planned into"
    );
    for (stratum, status) in &none.trailer.statuses {
        let whole_status = whole
            .trailer
            .statuses
            .get(stratum)
            .expect("the same producers are named");
        match (status, whole_status) {
            // A stratum that ranked rows was read down to one of them and no
            // further, so it is reported at that contribution. The unbounded run
            // read the same stratum to its end.
            (
                ProducerStatus::CeilingReached { bound },
                ProducerStatus::Exhausted { rows_emitted },
            ) => {
                assert!(*bound > Fixed::ZERO, "{stratum} was closed at no row");
                assert!(*rows_emitted > 0, "{stratum} had rows to stop short of");
            }
            // A stratum with nothing to read, or one that could not run, is the
            // same either way: there was never anything for a bound to stop.
            (bounded, unbounded) => assert_eq!(
                bounded, unbounded,
                "{stratum} held no rows, so the bound cannot have changed its status"
            ),
        }
    }

    // The completeness claim is still the trailer's, and it was never in the
    // rows: the unbounded run is where every producer reports exhaustion.
    for status in whole.trailer.statuses.values() {
        assert!(
            !matches!(status, ProducerStatus::CeilingReached { .. }),
            "nothing stopped the unbounded run, so nothing in it is bounded"
        );
    }
}

/// A registry whose *only* producer cannot run, so nothing reaches the fusion.
fn registry_of_only_the_failing_stratum() -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    let arity = PfArity::new(1, 1);
    registry.register_ranked(
        ex("pf/broken"),
        Arc::new(FailingProducer {
            arity,
            mode: arity.all_free_mode(),
        }),
        ranked(
            &ex("stratum/broken"),
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

#[test]
fn an_answer_names_its_plan_even_when_no_stream_reached_the_fusion() {
    // Every producer the plan reached failed to run, so no stream existed to
    // carry the plan's identity into the trailer. That is the one case where the
    // identity cannot be read back off the answer, and it is not a refusal: the
    // plan ran, the failure is reported per producer, and the answer still names
    // the plan it descends from.
    let registry = registry_of_only_the_failing_stratum();
    let stats = statistics_with_the_failing_stratum();
    let env = fixture_env(&registry, &stats);
    let profile = FusionProfile::with_decay(
        BTreeMap::from([(iri(&ex("stratum/broken")), Fixed::ONE)]),
        DecayRule::ReciprocalRank { k: K },
    )
    .expect("the fixture profile is valid");
    let request = mixed_request();

    let result = block_on(search(
        &request,
        &registry,
        &stats,
        common::empty_dataset(),
        &env,
        &profile,
    ))
    .expect("a producer that could not run is a status, not a whole-request refusal");

    assert_eq!(
        result.rows,
        Vec::new(),
        "the only producer could not run, so nothing ranked"
    );
    assert_eq!(
        result.trailer.plan_id, None,
        "no stream reached the fusion, so no stream carried the identity"
    );
    assert_eq!(
        result.plan_id,
        plan(&request, &registry, &stats)
            .expect("the request plans")
            .id(),
        "and the answer still names the plan that ran"
    );
    assert!(
        matches!(
            result.trailer.statuses.get(&iri(&ex("stratum/broken"))),
            Some(ProducerStatus::ExecutionFailed { .. })
        ),
        "an empty answer is not 'nothing matched': {:?}",
        result.trailer.statuses
    );
}

// ---------------------------------------------------------------------------
// 9. The answer invariant, proven at the shipped ladder
//
// `a fused answer never contains the same entity twice` is a claim about what
// `search` returns, so it is checked over `search` and not only over a
// hand-built stream set. The defect it closes was reachable here, not merely in
// principle: `rank_candidates` ranks the projected `?candidate` column row by
// row and de-duplicates nothing, and the compiled unit selects that column with
// no `DISTINCT` — so a producer whose rows repeat an entity produced a ranked
// stream that repeated it, and a `Unique` declaration over such a stream used to
// put the same entity in the answer twice.
//
// Both halves are here because both are the contract. A `Unique` producer that
// repeats is *refused*, naming the entity and the stratum, because the
// declaration is a claim about its index and breaking it makes that stratum's
// ranks untrustworthy. An `Allowed` producer with the identical rows answers,
// once, because that declaration predicted the repeat and made de-duplication
// the consumer's job.
// ---------------------------------------------------------------------------

/// A ranked declaration whose duplicate policy is the caller's to choose.
///
/// [`ranked`] hard-codes [`DuplicatePolicy::Unique`], which is the right default
/// for every other fixture in this file. The pair of tests below is *about* the
/// policy, so it must be able to state both.
fn ranked_declaring(
    stratum: &str,
    patterns: Vec<TermPattern>,
    mandatory: bool,
    duplicates: DuplicatePolicy,
) -> RankedDeclaration {
    RankedDeclaration {
        duplicates,
        ..ranked(stratum, patterns, mandatory)
    }
}

/// A producer whose rows name `entity0`, `entity1` and then `entity0` again.
///
/// Three rows rather than two, and in that order, because the repeat has to
/// arrive *after* the first occurrence has already certified and left the
/// frontier — which is exactly the interleaving the frontier's own
/// once-per-stream check cannot see. Two rows would be caught in the frontier by
/// the older check and would prove nothing about this one.
fn repeating_producer() -> Arc<dyn PropertyFunction> {
    let arity = PfArity::new(1, 1);
    let entity = |index: usize| {
        vec![
            TermValue::iri(format!("{}entity{index}", ex("repeat/"))),
            TermValue::iri(format!("{}score{index}", ex("repeat/"))),
        ]
    };
    Arc::new(MockProducer {
        arity,
        mode: arity.all_free_mode(),
        rows: 200,
        emitted: vec![entity(0), entity(1), entity(0)],
    })
}

/// A one-producer registry over the repeating rows, declaring `duplicates`.
///
/// One stratum, deliberately. A candidate in a single-stratum fusion is final
/// the moment it is read, so it certifies and leaves the frontier before the
/// next row arrives — which is what puts the repeat past the frontier and on the
/// path under test.
fn repeating_registry(duplicates: DuplicatePolicy) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/repeat"),
        repeating_producer(),
        ranked_declaring(
            &ex("stratum/repeat"),
            vec![TermPattern::of_kind(TermKind::Any)],
            true,
            duplicates,
        ),
    );
    registry
}

/// The statistics and profile the repeating fixture searches under.
fn repeating_statistics() -> MockStatistics {
    let mut cardinalities = BTreeMap::new();
    cardinalities.insert(iri(&ex("stratum/repeat")), 1000);
    cardinalities.insert(iri(&ex("body")), 500);
    MockStatistics {
        source: "example-statistics".to_owned(),
        revision: "r1".to_owned(),
        cardinalities,
    }
}

fn repeating_profile() -> FusionProfile {
    FusionProfile::with_decay(
        BTreeMap::from([(iri(&ex("stratum/repeat")), Fixed::ONE)]),
        DecayRule::ReciprocalRank { k: K },
    )
    .expect("the fixture profile is valid")
}

#[test]
fn a_repeating_unique_relation_is_refused_through_search_naming_the_item_and_stratum() {
    let registry = repeating_registry(DuplicatePolicy::Unique);
    let stats = repeating_statistics();
    let env = fixture_env(&registry, &stats);
    let profile = repeating_profile();

    // First, evidence that this fixture really does exercise the *past the
    // frontier* arm rather than the frontier's own check. Bounded at two rows
    // the identical search answers, with both candidates in it: the first
    // candidate has been certified and handed to the caller, and the repeat at
    // rank three is simply never merged. So when the unbounded search below
    // refuses, what it refused is a row naming a candidate that had already
    // left the frontier — which is the arm under test and the one the defect
    // lived in.
    let bounded = block_on(search(
        &RetrievalRequest::bounded(vec![lexical_term()], TopK::new(2)),
        &registry,
        &stats,
        common::empty_dataset(),
        &env,
        &profile,
    ))
    .expect("the bound stops the reading before the repeat is merged");
    assert_eq!(
        bounded
            .rows
            .iter()
            .map(|row| row.entity.as_str().to_owned())
            .collect::<Vec<_>>(),
        vec![
            format!("<{}entity0>", ex("repeat/")),
            format!("<{}entity1>", ex("repeat/")),
        ],
        "the repeated candidate must have been certified and emitted first"
    );

    let error = block_on(search(
        &RetrievalRequest::bounded(vec![lexical_term()], TOP_K),
        &registry,
        &stats,
        common::empty_dataset(),
        &env,
        &profile,
    ))
    .expect_err("a `Unique` relation whose rows repeat a candidate must be refused");

    let SearchError::FusionError(FusionError::Protocol(protocol)) = &error else {
        panic!("expected a fused protocol refusal, got {error:?}");
    };
    let ProtocolError::DuplicateItem {
        item,
        stratum: named,
    } = &**protocol
    else {
        panic!("expected DuplicateItem, got {protocol:?}");
    };
    assert_eq!(
        item,
        &format!("<{}entity0>", ex("repeat/")),
        "the refusal must name the repeated candidate, spelled as a caller would seed it"
    );
    assert_eq!(
        named,
        &ex("stratum/repeat"),
        "the refusal must name the stratum whose producer broke its declaration"
    );
}

#[test]
fn the_same_repeating_relation_declaring_allowed_answers_once_through_search() {
    // THE NEIGHBOURING CASE the refusal above must not swallow: identical rows,
    // identical everything, and the one declaration that predicted the repeat.
    // It answers — and answers with the candidate exactly once.
    let registry = repeating_registry(DuplicatePolicy::Allowed);
    let stats = repeating_statistics();
    let env = fixture_env(&registry, &stats);
    let profile = repeating_profile();

    let result = block_on(search(
        &RetrievalRequest::bounded(vec![lexical_term()], TOP_K),
        &registry,
        &stats,
        common::empty_dataset(),
        &env,
        &profile,
    ))
    .expect("an `Allowed` relation's repeats are de-duplicated, not refused");

    let entities: Vec<String> = result
        .rows
        .iter()
        .map(|row| row.entity.as_str().to_owned())
        .collect();
    assert_eq!(
        entities,
        vec![
            format!("<{}entity0>", ex("repeat/")),
            format!("<{}entity1>", ex("repeat/")),
        ],
        "the repeated candidate must appear once, at its best rank, ahead of the other"
    );

    // And the general statement, asserted over the answer rather than inferred
    // from its length: no entity appears twice.
    let mut distinct = BTreeMap::new();
    for row in &result.rows {
        assert!(
            distinct.insert(row.entity.clone(), ()).is_none(),
            "a search answer contained {:?} twice",
            row.entity
        );
    }

    // The producer is still charged for the row it emitted: the discarded
    // duplicate was pulled, and the terminal receipt is measured against it.
    assert_eq!(
        result.trailer.statuses[&iri(&ex("stratum/repeat"))],
        ProducerStatus::Exhausted { rows_emitted: 3 },
        "a de-duplicated row is still a row the producer emitted"
    );
}

// ---------------------------------------------------------------------------
// T6.9. The candidate-domain declaration travels the whole route, and the
//       answer says which declarations it was certified under.
//
// The declaration is host configuration supplied at `register_ranked`. Fusion
// is the consumer it was written for, and it is four stages downstream: the
// admission waist reads it into `StratumUnit::contract`, `execute` tags every
// stream with it, `RankedStreamAdapter` reports it through the protocol, and
// `FusionStream::new` reads it before pulling a row. A break anywhere on that
// route is invisible from either end — the registry still declares, the answer
// still has rows — so the trailer echoes what actually reached the engine, and
// this test compares that echo against the registry itself rather than against
// a literal written twice.
// ---------------------------------------------------------------------------

/// What a fused row says as an ANSWER: the entity, its score and where that
/// score came from.
///
/// [`FusedRow::threshold_witness`] is deliberately absent: it is the threshold
/// in force at the instant the row certified, so it is a fact about how far the
/// streams had been read rather than about the answer, and a declaration that
/// licenses an earlier stop certifies the identical row against a higher one.
type FusedAnswer = (Term, Fixed, Vec<(Iri, u64, Fixed)>);

/// Two caller-named blocks. Nothing here mints them: they are `example.org`
/// IRIs a host chose for its own partition, exactly as the strata are.
fn domain_tag(suffix: &str) -> DomainTag {
    DomainTag::parse(&ex(suffix)).expect("fixture domain tags are valid IRIs")
}

/// The fixture registry, with each producer declaring where its candidates lie.
///
/// The three mock producers really do emit disjoint candidate sets — each mints
/// its entities under its own prefix — so `universal` and `text` are declared
/// over their own blocks and `graph` over both, which is the truthful
/// declaration for a producer whose rows a host knows span the corpus. Every
/// declaration below is a fact about these fixtures rather than a convenient
/// label, which is what makes the answer comparable with the undeclared one.
///
/// The two one-block producers name no block per row and need not: their
/// declaration entails it, because a producer that promised its candidates lie in
/// one block has already said where every row came from. `graph` declares two
/// blocks, so its declaration entails nothing per row, and it backs the promise
/// the only way a several-block producer can — an argument position its rows name
/// their own block from, declared as `block_position`. A several-block
/// declaration with no such position is refused at the first row, which
/// `a_several_block_declaration_no_row_backs_is_refused` executes beside this.
fn domain_declaring_registry() -> PropertyFunctionRegistry {
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
        RankedDeclaration {
            domains: CandidateDomains::within([domain_tag("domain/universal")]),
            ..ranked(
                &ex("stratum/universal"),
                vec![TermPattern::of_kind(TermKind::Any)],
                true,
            )
        },
    );
    registry.register_ranked(
        ex("pf/literal"),
        producer(100, "text/", 2),
        RankedDeclaration {
            domains: CandidateDomains::within([domain_tag("domain/text")]),
            ..ranked(&ex("stratum/text"), vec![literal_pattern], false)
        },
    );
    registry.register_ranked(
        ex("pf/iri"),
        producer_naming_blocks(50, "graph/", &[&ex("domain/universal")]),
        RankedDeclaration {
            domains: CandidateDomains::within([
                domain_tag("domain/universal"),
                domain_tag("domain/text"),
            ]),
            block_position: Some(2),
            ..ranked(
                &ex("stratum/graph"),
                vec![TermPattern::of_kind(TermKind::Iri)],
                false,
            )
        },
    );
    registry.register(ex("pf/not-ranked"), producer(0, "unranked/", 0));
    registry
}

#[test]
fn the_trailer_reports_the_candidate_domains_the_registry_declared() {
    let registry = domain_declaring_registry();
    let stats = statistics("r1");
    let env = fixture_env(&registry, &stats);
    let profile = fixture_profile();
    let result = block_on(search(
        &mixed_request(),
        &registry,
        &stats,
        common::empty_dataset(),
        &env,
        &profile,
    ))
    .expect("the fixture request searches");

    // The expectation is read back OUT OF THE REGISTRY, by producer IRI, so
    // this compares the answer against the configuration rather than against a
    // second copy of the same literal. A route that dropped the declaration
    // somewhere between the two would pass a literal-to-literal comparison.
    let declared = |producer: &str, stratum: &str| {
        (
            iri(&ex(stratum)),
            registry
                .ranked_declaration(&ex(producer))
                .expect("the fixture producer is registered as ranked")
                .domains
                .clone(),
        )
    };
    assert_eq!(
        result.trailer.domains,
        BTreeMap::from([
            declared("pf/any", "stratum/universal"),
            declared("pf/literal", "stratum/text"),
            declared("pf/iri", "stratum/graph"),
        ]),
        "every stream this search fused reports the declaration its producer \
         was registered with, keyed by its stratum"
    );

    // And the declaration is an input, not a result: the same request over the
    // same producers declaring nothing returns the same rows. A declaration
    // that moved the answer would be a bound bought with a wrong result.
    let undeclared = fixture_registry();
    let undeclared_env = fixture_env(&undeclared, &stats);
    let plain = block_on(search(
        &mixed_request(),
        &undeclared,
        &stats,
        common::empty_dataset(),
        &undeclared_env,
        &profile,
    ))
    .expect("the fixture request searches");
    let answers = |result: &SearchResult| -> Vec<FusedAnswer> {
        result
            .rows
            .iter()
            .map(|row| (row.entity.clone(), row.score, row.contributions.clone()))
            .collect()
    };
    assert_eq!(
        answers(&result),
        answers(&plain),
        "the declared search and the undeclared one answer identically"
    );
    assert_eq!(
        plain.trailer.domains,
        BTreeMap::from([
            (
                iri(&ex("stratum/universal")),
                CandidateDomains::Unrestricted
            ),
            (iri(&ex("stratum/text")), CandidateDomains::Unrestricted),
            (iri(&ex("stratum/graph")), CandidateDomains::Unrestricted),
        ]),
        "a producer that declared nothing is reported as promising everything, \
         which is what it did promise"
    );
}

// ---------------------------------------------------------------------------
// T6.10. The per-row block, through the shipped path.
//
// A restricted declaration is the premise fusion certifies early on: it skips
// streams that provably cannot name a candidate, and it bounds every item nobody
// has seen yet by the best single block rather than by every stream at once. Both
// are sound only under one axiom — the host's tags partition the candidate
// universe, so a candidate lies in exactly one block — which is a fact about the
// host's corpus that no consumer can derive.
//
// So each row says which block it came from, and the three ways that can go wrong
// are refused by name rather than folded into an order. Each is executed here
// through `search`, the whole ladder, because the route from a registry
// declaration to a checked row runs through all four stages: the block column is
// projected by `compile`, read by `execute`, carried by `RankedStreamAdapter` and
// held to the declaration by `FusionStream`. A break anywhere on it is invisible
// from either end.
//
// Every refusal is paired with its valid neighbour, executed in the same test.
// Over-refusal is as severe a defect as a silent wrong answer: a declaration a
// host can back must keep working, or this mechanism has cost the answers it was
// supposed to protect.
// ---------------------------------------------------------------------------

/// A one-producer registry over the universal stratum: `relation` declaring
/// `domains`, reading each row's block from `block_position`.
fn block_declaring_registry(
    relation: Arc<dyn PropertyFunction>,
    domains: CandidateDomains,
    block_position: Option<usize>,
) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/any"),
        relation,
        RankedDeclaration {
            domains,
            block_position,
            ..ranked(
                &ex("stratum/universal"),
                vec![TermPattern::of_kind(TermKind::Any)],
                true,
            )
        },
    );
    registry
}

/// Run the fixture request through the whole ladder against `registry`.
fn search_against(registry: &PropertyFunctionRegistry) -> Result<SearchResult, SearchError> {
    let stats = statistics("r1");
    let env = fixture_env(registry, &stats);
    block_on(search(
        &mixed_request(),
        registry,
        &stats,
        common::empty_dataset(),
        &env,
        &fixture_profile(),
    ))
}

/// The protocol refusal `outcome` carries, or a panic naming what it carried
/// instead.
fn protocol_refusal(outcome: Result<SearchResult, SearchError>) -> ProtocolError {
    match outcome {
        Err(SearchError::FusionError(FusionError::Protocol(protocol))) => *protocol,
        Err(other) => panic!("expected a protocol refusal, got {other:?}"),
        Ok(_) => panic!("expected a refusal, and the search answered"),
    }
}

/// The entities `result` returned, in order.
fn entities(result: &SearchResult) -> Vec<String> {
    result
        .rows
        .iter()
        .map(|row| row.entity.as_str().to_owned())
        .collect()
}

/// **Refusal one.** A producer restricted to several blocks, whose rows name
/// none, has declared something nothing backs.
///
/// A one-block declaration entails every row's block, so it needs no column; a
/// several-block declaration entails nothing about any one row, and a consumer
/// may not choose on the host's behalf. Fusion has already *used* the
/// restriction by the time the row arrives, so the alternative to refusing is to
/// keep certifying against an axiom nothing checked.
#[test]
fn a_several_block_declaration_no_row_backs_is_refused() {
    let unbacked = block_declaring_registry(
        producer(200, "universal/", 3),
        CandidateDomains::within([domain_tag("domain/universal"), domain_tag("domain/text")]),
        None,
    );
    match protocol_refusal(search_against(&unbacked)) {
        ProtocolError::UnbackedDomainDeclaration {
            stratum,
            declared,
            rank,
        } => {
            assert_eq!(
                stratum,
                ex("stratum/universal"),
                "the refusal names the stratum whose declaration nothing backs"
            );
            assert_eq!(
                declared,
                vec![ex("domain/text"), ex("domain/universal")],
                "and the promise itself, which is what decides between the two exits"
            );
            assert_eq!(rank, 1, "and the row measured against it");
        }
        other => panic!("expected an unbacked-declaration refusal, got {other:?}"),
    }

    // Valid neighbour one: the same producer, the same rows, restricted to ONE
    // block. The declaration entails where every row came from, so nothing is
    // owed per row and the search answers.
    let entailed = block_declaring_registry(
        producer(200, "universal/", 3),
        CandidateDomains::within([domain_tag("domain/universal")]),
        None,
    );
    let answered = search_against(&entailed).expect("a one-block declaration backs itself");
    assert_eq!(
        entities(&answered),
        vec![
            format!("<{}entity0>", ex("universal/")),
            format!("<{}entity1>", ex("universal/")),
            format!("<{}entity2>", ex("universal/")),
        ],
        "a producer whose declaration entails its rows' block answers in full"
    );

    // Valid neighbour two: the several-block declaration, backed. The host names
    // the position its rows carry their own block in, and the identical
    // restriction is now a promise every row supports — including one row from
    // each of the two declared blocks.
    let backed = block_declaring_registry(
        producer_naming_blocks(
            200,
            "universal/",
            &[
                &ex("domain/universal"),
                &ex("domain/text"),
                &ex("domain/universal"),
            ],
        ),
        CandidateDomains::within([domain_tag("domain/universal"), domain_tag("domain/text")]),
        Some(2),
    );
    let backed = search_against(&backed).expect("a several-block declaration its rows back");
    assert_eq!(
        entities(&backed),
        entities(&answered),
        "the same rows in the same order: a backed declaration costs the answer nothing"
    );
}

/// **Refusal two.** A row naming a block its own declaration excludes is a
/// stream contradicting itself, and needs no second stream to witness it.
#[test]
fn a_row_naming_a_block_outside_its_own_declaration_is_refused() {
    let declared =
        || CandidateDomains::within([domain_tag("domain/universal"), domain_tag("domain/text")]);
    let outside = block_declaring_registry(
        producer_naming_blocks(
            200,
            "universal/",
            &[&ex("domain/universal"), &ex("domain/people")],
        ),
        declared(),
        Some(2),
    );
    match protocol_refusal(search_against(&outside)) {
        ProtocolError::BlockOutsideDeclaredDomain {
            item,
            stratum,
            block,
            declared,
        } => {
            assert_eq!(
                item,
                format!("<{}entity1>", ex("universal/")),
                "the refusal names the row's candidate"
            );
            assert_eq!(stratum, ex("stratum/universal"), "and who named it");
            assert_eq!(
                block,
                ex("domain/people"),
                "and the block the row claimed, without which the reader sees no contradiction"
            );
            assert_eq!(
                declared,
                vec![ex("domain/text"), ex("domain/universal")],
                "and the set it fell outside"
            );
        }
        other => panic!("expected an outside-declaration refusal, got {other:?}"),
    }

    // The valid neighbour, one block different: the second row names the other
    // block the SAME declaration includes. Naming a block is not what was
    // refused — naming one the producer never promised was.
    let inside = block_declaring_registry(
        producer_naming_blocks(
            200,
            "universal/",
            &[&ex("domain/universal"), &ex("domain/text")],
        ),
        declared(),
        Some(2),
    );
    let answered = search_against(&inside).expect("both blocks are declared, so both are honest");
    assert_eq!(
        entities(&answered),
        vec![
            format!("<{}entity0>", ex("universal/")),
            format!("<{}entity1>", ex("universal/")),
        ],
        "a row drawn from either declared block is an ordinary row"
    );
}

/// **Refusal three — the one that closes the gap.** Two streams naming one
/// candidate from two different blocks have proven the axiom false for that
/// candidate.
///
/// Their declarations are not contradictory: both name `domain/universal`, so
/// `OutsideDeclaredDomain` cannot see this and does not fire. What is
/// contradictory is the pair of *rows*, and that is exactly the case the
/// threshold's per-block maximum under-bounds — the streams that reach one block
/// and the streams that reach the other are different sets, so an item in two
/// blocks can collect more than any single block's sum. Refused here rather than
/// silently reordered.
#[test]
fn two_streams_naming_one_candidate_from_two_blocks_are_refused() {
    // Both producers emit the same entity, which is the ordinary overlapping
    // case fused enumeration exists for. What varies below is only the block
    // each one says that entity came from.
    let pair = |left_block: &str, right_block: &str| {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register_ranked(
            ex("pf/any"),
            producer_naming_blocks(200, "shared/", &[left_block]),
            RankedDeclaration {
                domains: CandidateDomains::within([
                    domain_tag("domain/universal"),
                    domain_tag("domain/text"),
                ]),
                block_position: Some(2),
                ..ranked(
                    &ex("stratum/universal"),
                    vec![TermPattern::of_kind(TermKind::Any)],
                    true,
                )
            },
        );
        registry.register_ranked(
            ex("pf/literal"),
            producer_naming_blocks(100, "shared/", &[right_block]),
            RankedDeclaration {
                // Overlapping, not disjoint: both declarations admit
                // `domain/universal`, so nothing about the pair of promises is
                // contradictory and the declaration-level refusal stays silent.
                domains: CandidateDomains::within([
                    domain_tag("domain/universal"),
                    domain_tag("domain/people"),
                ]),
                block_position: Some(2),
                ..ranked(
                    &ex("stratum/text"),
                    vec![TermPattern {
                        kind: TermKind::Literal,
                        datatype: None,
                        language: Some("en".to_owned()),
                        predicate: Some(ex("body")),
                    }],
                    false,
                )
            },
        );
        registry
    };

    let contradicting = pair(&ex("domain/universal"), &ex("domain/people"));
    match protocol_refusal(search_against(&contradicting)) {
        ProtocolError::CandidateInTwoBlocks {
            item,
            stratum,
            block,
            named_by,
            named_by_block,
        } => {
            assert_eq!(
                item,
                format!("<{}entity0>", ex("shared/")),
                "the refusal names the candidate whose tagging cannot be true"
            );
            assert_eq!(
                (stratum, block),
                (ex("stratum/universal"), ex("domain/universal")),
                "the stream whose row arrived last, and the block it claimed"
            );
            assert_eq!(
                (named_by, named_by_block),
                (ex("stratum/text"), ex("domain/people")),
                "and the stream that had already placed it, with the block it placed it in — \
                 either producer could be the one that tagged wrongly, so both are named"
            );
        }
        other => panic!("expected a two-block refusal, got {other:?}"),
    }

    // The valid neighbour, one block different: the second stream says the
    // candidate came from the block the first one placed it in. Two producers
    // ranking one entity is the case fused enumeration is FOR, and it still
    // fuses into a single row carrying both contributions.
    let agreeing = pair(&ex("domain/universal"), &ex("domain/universal"));
    let answered =
        search_against(&agreeing).expect("two streams agreeing about a candidate's block fuse");
    assert_eq!(
        entities(&answered),
        vec![format!("<{}entity0>", ex("shared/"))],
        "one candidate, one row"
    );
    assert_eq!(
        answered.rows[0].contributions.len(),
        2,
        "and both strata's contributions, which is what the agreement buys"
    );
}

// ---------------------------------------------------------------------------
// T6.11. The bounded read rests on TWO declarations, and a stream that may
//        repeat supplies only one of them.
//
// The planner narrows a bounded request's per-stratum depth to `k` when every
// surviving stratum declares a block set and no two of those sets meet. The merge
// argument behind it counts ranks: a candidate its stratum ranks at `r > k` is
// beaten by the `r-1 >= k` candidates above it in that same stratum. That step
// reads a count of ranks as a count of candidates, which is true of a
// `DuplicatePolicy::Unique` stream and false of a `DuplicatePolicy::Allowed` one —
// a repeat there is validated, charged to the producer and then DISCARDED, which
// is exactly what that declaration asks a consumer to do. A depth-`k` prefix of
// such a stream can therefore carry fewer than `k` candidates, and a prefix short
// of `k` candidates is no longer a superset of the top `k`.
//
// The assertion below is a differential rather than a depth check, because a
// depth check alone would pass on an answer that was wrong for some other reason:
// the `Allowed` request must return the very rows, scores and provenance the same
// request returns from a stratum declaring nothing at all, which is the read
// nothing has ever narrowed. Its neighbour is here for the mirror defect — the
// `Unique` case must KEEP the bounded read it already had, or the correction has
// cost the answers it was meant to protect.
// ---------------------------------------------------------------------------

/// The stratum the fixtures below rank within, and the one block they draw from.
const PREFIX: &str = "docs/";

/// The rows a producer of this section emits, by candidate index.
///
/// Six rows naming five candidates, with `entity0` named again at rank five: the
/// repeat lands INSIDE a bound of five, which is what makes a depth of five a
/// prefix of four candidates. `entity4`, the fifth candidate, is then at rank six
/// and a read of five rows never reaches it. Three rows would not do — the repeat
/// has to arrive after the first occurrence has certified and left the frontier,
/// which is the interleaving a single-stratum fusion always produces.
const REPEATING_ROWS: [usize; 6] = [0, 1, 2, 3, 0, 4];

/// The same length with no repeat in it: six candidates, one per rank. This is
/// the honest `Unique` stream, and the neighbour whose bound must survive.
const DISTINCT_ROWS: [usize; 6] = [0, 1, 2, 3, 4, 5];

/// A producer emitting one row per entry of `rows`, naming `entity<index>`.
///
/// The declared row bound is two hundred — far above the rows it emits — so the
/// depth this stratum is planned at is decided by the request's bound and by the
/// declarations, never by the script running out.
fn producer_over(rows: &[usize]) -> Arc<dyn PropertyFunction> {
    let arity = PfArity::new(1, 1);
    let emitted = rows
        .iter()
        .map(|index| {
            vec![
                TermValue::iri(format!("{}entity{index}", ex(PREFIX))),
                TermValue::iri(format!("{}score{index}", ex(PREFIX))),
            ]
        })
        .collect();
    Arc::new(MockProducer {
        arity,
        mode: arity.all_free_mode(),
        rows: 200,
        emitted,
    })
}

/// A one-producer registry over `rows`, declaring `domains` and `duplicates`.
///
/// One stratum, because the narrowing is a per-stratum depth and one stratum
/// declaring a block set is already the disjoint case — there is no second
/// declaration for it to meet.
fn docs_registry(
    rows: &[usize],
    domains: CandidateDomains,
    duplicates: DuplicatePolicy,
) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/docs"),
        producer_over(rows),
        RankedDeclaration {
            domains,
            ..ranked_declaring(
                &ex("stratum/docs"),
                vec![TermPattern::of_kind(TermKind::Any)],
                true,
                duplicates,
            )
        },
    );
    registry
}

fn docs_statistics() -> MockStatistics {
    let mut cardinalities = BTreeMap::new();
    cardinalities.insert(iri(&ex("stratum/docs")), 1000);
    cardinalities.insert(iri(&ex("body")), 500);
    MockStatistics {
        source: "example-statistics".to_owned(),
        revision: "r1".to_owned(),
        cardinalities,
    }
}

fn docs_profile() -> FusionProfile {
    FusionProfile::with_decay(
        BTreeMap::from([(iri(&ex("stratum/docs")), Fixed::ONE)]),
        DecayRule::ReciprocalRank { k: K },
    )
    .expect("the fixture profile is valid")
}

/// The request both halves of each differential are run under: a top five, which
/// is the bound the narrowing would apply.
fn top_five() -> RetrievalRequest {
    RetrievalRequest::bounded(vec![lexical_term()], TopK::new(5))
}

/// The depth the planner records for the fixture stratum under `registry`.
fn docs_depth(registry: &PropertyFunctionRegistry) -> u32 {
    *plan(&top_five(), registry, &docs_statistics())
        .expect("the fixture request plans")
        .stratum_depths
        .get(&iri(&ex("stratum/docs")))
        .expect("the fixture stratum is planned")
}

/// The same request with no bound on it: everything the stratum holds.
///
/// This is the un-narrowed neighbour every differential below is measured against,
/// and it is the same registry rather than a differently-declared one — a
/// `ReadBound::Complete` request licenses no prefix whatever the producers declare,
/// so the only thing that moves between the two runs is the depth.
fn everything() -> RetrievalRequest {
    RetrievalRequest::complete(vec![lexical_term()])
}

/// The depth the planner records for the fixture stratum under `registry` when the
/// request states no bound.
fn docs_complete_depth(registry: &PropertyFunctionRegistry) -> u32 {
    *plan(&everything(), registry, &docs_statistics())
        .expect("the fixture request plans")
        .stratum_depths
        .get(&iri(&ex("stratum/docs")))
        .expect("the fixture stratum is planned")
}

/// The fixture request, searched through the whole ladder against `registry`.
fn docs_search(registry: &PropertyFunctionRegistry) -> SearchResult {
    docs_search_for(&top_five(), registry)
}

/// The un-narrowed search: the whole stratum, fused at the bound its depths sum to.
fn docs_complete_search(registry: &PropertyFunctionRegistry) -> SearchResult {
    docs_search_for(&everything(), registry)
}

fn docs_search_for(
    request: &RetrievalRequest,
    registry: &PropertyFunctionRegistry,
) -> SearchResult {
    let stats = docs_statistics();
    let env = fixture_env(registry, &stats);
    block_on(search(
        request,
        registry,
        &stats,
        common::empty_dataset(),
        &env,
        &docs_profile(),
    ))
    .expect("the fixture request searches")
}

/// What an answer says, as an answer: the rows in order with their scores and
/// the ranks those scores came from.
fn fused_answers(result: &SearchResult) -> Vec<FusedAnswer> {
    result
        .rows
        .iter()
        .map(|row| (row.entity.clone(), row.score, row.contributions.clone()))
        .collect()
}

/// The fixture stratum's rank behind each row of `result`, in row order.
fn docs_ranks(result: &SearchResult) -> Vec<u64> {
    result
        .rows
        .iter()
        .map(|row| {
            row.contributions
                .iter()
                .find(|(stratum, _, _)| stratum == &iri(&ex("stratum/docs")))
                .map(|(_, rank, _)| *rank)
                .expect("every row of a one-stratum fusion is contributed to by that stratum")
        })
        .collect()
}

/// The entity `index` names, spelled as a fused row carries it.
fn docs_entity(index: usize) -> String {
    format!("<{}entity{index}>", ex(PREFIX))
}

#[test]
fn an_allowed_stratum_keeps_the_read_its_repeats_need_and_answers_as_the_undeclared_one_does() {
    let declared = docs_registry(
        &REPEATING_ROWS,
        CandidateDomains::within([domain_tag("domain/docs")]),
        DuplicatePolicy::Allowed,
    );
    let undeclared = docs_registry(
        &REPEATING_ROWS,
        CandidateDomains::Unrestricted,
        DuplicatePolicy::Allowed,
    );

    // The depth first: a stream that may repeat supplies no count of candidates,
    // so neither the block declaration nor the fact that this is the only stratum
    // licenses anything, and the registry's own bound stands. This is the shape that
    // still does not narrow at one stratum: uniqueness is what makes a count of
    // ranks a count of candidates, and it is absent here, so the vacuous
    // disjointness a single stratum enjoys has no second premise to work with.
    assert_eq!(
        (docs_depth(&declared), docs_depth(&undeclared)),
        (200, 200),
        "a declared-`Allowed` stratum reads exactly as deep as an undeclared one, \
         because the merge argument a narrower depth rests on has no premise here"
    );

    // And the answer, which is the real claim: the same rows, the same scores and
    // the same provenance either way. A depth of five would have returned four
    // rows from the declared registry and five from the undeclared one, with
    // nothing in either answer saying which had been cut.
    let answered = docs_search(&declared);
    assert_eq!(
        fused_answers(&answered),
        fused_answers(&docs_search(&undeclared)),
        "the declared search and the undeclared one answer identically, row for \
         row and score for score"
    );
    assert_eq!(
        entities(&answered),
        (0..5).map(docs_entity).collect::<Vec<_>>(),
        "five candidates were asked for and five distinct candidates are returned"
    );

    // The repeat is shown to have happened rather than assumed: the fifth row
    // carries stratum rank SIX. Rank five was pulled, validated, charged to the
    // producer and discarded as the declared repeat it was, so the fifth
    // candidate lies one row past a bound of five — which is the whole reason the
    // narrowed read could not have served this answer.
    assert_eq!(
        docs_ranks(&answered),
        vec![1, 2, 3, 4, 6],
        "rank five is absent from the answer because the row carrying it repeated \
         a candidate already certified"
    );
    assert_eq!(
        answered.trailer.statuses[&iri(&ex("stratum/docs"))],
        ProducerStatus::Exhausted { rows_emitted: 6 },
        "and the producer is charged for the discarded row, which is a row it emitted"
    );
}

#[test]
fn the_unique_neighbour_keeps_its_bounded_read_and_the_answer_it_already_gave() {
    // THE OVER-REFUSAL MIRROR. The same shape, the same six rows, and the promise
    // that no candidate is named twice, which is what makes a count of ranks a count
    // of candidates. The bound must survive.
    //
    // Both declarations narrow, and that pair is the second claim here. With one
    // surviving stratum there is no pair for a disjointness premise to be about, so
    // the blocks a producer declares cannot add anything to what uniqueness alone
    // already gives: its rank order IS the fused order. Withholding the narrowing
    // from the undeclared stratum refused exactly the ordinary configuration —
    // neither shipped ranked relation declares a domain — and the refusal was
    // invisible, because the answer was identical and only the read was larger.
    let declared = docs_registry(
        &DISTINCT_ROWS,
        CandidateDomains::within([domain_tag("domain/docs")]),
        DuplicatePolicy::Unique,
    );
    let undeclared = docs_registry(
        &DISTINCT_ROWS,
        CandidateDomains::Unrestricted,
        DuplicatePolicy::Unique,
    );

    assert_eq!(
        (docs_depth(&declared), docs_depth(&undeclared)),
        (5, 5),
        "a `Unique` stratum reads a five-row prefix for a top five whether or not it \
         declares a block, because one stratum has no second one to overlap with"
    );

    // The un-narrowed neighbour, and the only honest one available here: the SAME
    // registry asked for everything it holds. That is what makes the comparison
    // below a measurement of the depth rather than of two different fixtures.
    assert_eq!(
        docs_complete_depth(&undeclared),
        200,
        "a request that states no bound licenses no prefix, so the registry's own \
         declaration still stands — which is what makes the depth above a narrowing"
    );

    let answered = docs_search(&declared);
    assert_eq!(
        fused_answers(&answered),
        fused_answers(&docs_search(&undeclared)),
        "the declared search and the undeclared one answer identically, row for row \
         and score for score"
    );
    assert_eq!(
        entities(&answered),
        (0..5).map(docs_entity).collect::<Vec<_>>(),
        "the top five candidates, in rank order"
    );
    assert_eq!(
        docs_ranks(&answered),
        vec![1, 2, 3, 4, 5],
        "and no rank is missing, because no row repeated a candidate"
    );

    // THE DIFFERENTIAL, which is what makes the narrowing exact rather than merely
    // cheaper: the narrowed read answers with the leading rows of the two-hundred-row
    // read, byte for byte, score for score and rank for rank. A depth check alone
    // would pass for a narrowing that dropped the right answer.
    //
    // Three bounds, because the claim is about every `k` and not about five: one row,
    // a bound inside the corpus, and a bound above everything the stratum holds —
    // where the read runs out before the depth does and the narrowing must still not
    // change the answer.
    //
    // How many rows the narrowed answer owes is derived from the request and from the
    // WHOLE answer, never from the narrowed answer itself. Reading that length off the
    // value under test is what made an earlier version of this differential vacuous: it
    // compared the narrowed answer against its own prefix of the whole one, so every
    // truncation satisfied it — including the empty one, and including the three-row
    // loss the narrowing is here to rule out. A prefix comparison whose length comes
    // from the shorter side cannot see a short side.
    let whole = docs_complete_search(&undeclared);
    let whole_rows = whole.rows.len();
    assert_eq!(
        whole_rows, 6,
        "the un-narrowed answer holds every candidate the stratum names, which is what \
         the bounded answers below are measured against"
    );
    for requested in [1_usize, 3, 20] {
        let request = RetrievalRequest::bounded(vec![lexical_term()], TopK::new(requested));
        let narrowed = docs_search_for(&request, &undeclared);
        let depth = *plan(&request, &undeclared, &docs_statistics())
            .expect("the fixture request plans")
            .stratum_depths
            .get(&iri(&ex("stratum/docs")))
            .expect("the fixture stratum is planned");
        assert_eq!(
            depth,
            u32::try_from(requested).expect("the fixture bounds fit a rank"),
            "the bound IS the depth for a top {requested} over one unique stratum"
        );
        // A top `k` owes `k` rows, or every row there is where the stratum holds fewer
        // than `k` — the bound above the corpus is the third case in this loop, and it
        // must return six rather than refuse or truncate.
        let owed = requested.min(whole_rows);
        assert_eq!(
            narrowed.rows.len(),
            owed,
            "a top {requested} over a six-candidate stratum answers with {owed} rows"
        );
        assert_eq!(
            fused_answers(&narrowed),
            fused_answers(&whole)[..owed].to_vec(),
            "a top {requested} returns the prefix of the un-narrowed answer, unchanged"
        );
        assert_eq!(
            docs_ranks(&narrowed),
            docs_ranks(&whole)[..owed].to_vec(),
            "and each row arrives from the same stratum rank it did in the full read"
        );
    }
}
