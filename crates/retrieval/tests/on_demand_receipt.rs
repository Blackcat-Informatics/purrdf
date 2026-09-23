// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The receipt of a read produced on demand: what it attests, when it is taken,
//! and what it refuses.
//!
//! [`search`] reads every stratum it rendered as one invocation held open and read
//! a row per pull, and the fusion stops pulling long before a deep plan's depth.
//! Its evidence is taken when the fusion stops: each stream announced, before its
//! first row, what its invocation attested the instant it opened, and each stream
//! then settles to the witness its invocation stands behind at the stop, read under
//! the same sole-witness rule a finished run is read under and held to that
//! announcement.
//!
//! Every refusal below is executed beside a neighbour that is admitted, and each
//! neighbour's oracle distinguishes "honoured" from "silently dropped": the
//! admitted run's answer is compared, row for row, with the answer the same
//! producers give read materialised, and its trailer is shown to carry the very
//! attestation the producer declared.
//!
//! Fixtures use `example.org` throughout; every block tag and stratum IRI below is
//! fixture configuration, never a minted vocabulary.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll, Wake, Waker};

use pretty_assertions::assert_eq;
use purrdf_core::{RdfDataset, TermValue};
use purrdf_retrieval::{
    AdmissionEnvironment, CandidateDomains, DecayRule, DomainTag, DuplicatePolicy, Fixed,
    FusionError, FusionProfile, Iri, PfAttestation, ProtocolError, RECIP_K, RankFidelity,
    RankedStreamAdapter, ReadSchedule, RequestTerm, RetrievalRequest, SearchError, SearchResult,
    Statistics, Term, TopK, compile, execute, execute_within, fuse, plan, search,
};
use purrdf_sparql_eval::{
    AcceptedTerm, EvalError, ExclusionBasis, IndexGeneration, PfArgs, PfArity, PfCursor, PfRow,
    PropertyFunction, PropertyFunctionRegistry, RankedDeclaration, RequestFacet, ServiceLevel,
    TermKind, TermPattern, TermPlacement, Volatility,
};

mod common;

/// The bound every run searches under.
const TOP_K: TopK = TopK::new(5);

/// How many candidates each producer holds, and declares.
const ROWS: u64 = 400;

/// The generation every producer here opens on.
const OPENED_ON: &str = "gen-7";

/// The generation a moving index has moved to.
const MOVED_TO: &str = "gen-8";

/// How many rows a moving producer hands out before it moves.
///
/// Well inside the six ranks the fusion pulls from each stratum in this shape, so a
/// move keyed to it happens during the read the fusion takes.
const MOVES_AFTER: u64 = 3;

fn ex(suffix: &str) -> String {
    format!("http://example.org/{suffix}")
}

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

/// A minimal single-threaded executor. The producers never actually pend.
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

/// What a producer's index does while it is being read.
#[derive(Clone, Debug)]
enum Index {
    /// Pinned at open, whole throughout: the ordinary index.
    Stable,
    /// Pinned at open, short throughout, and saying so from the first instant.
    ShortFromTheStart,
    /// Rebuilt under the read: after [`MOVES_AFTER`] rows the cursor reports a
    /// different generation from the one it opened on.
    MovesUnderTheRead,
    /// Found short under the read: whole at open, short after [`MOVES_AFTER`] rows.
    FoundShortUnderTheRead,
}

/// The reason a short index gives.
const SHORT_REASON: &str = "shard 3 rebuilding";

/// A ranked producer over one shared candidate list, whose index behaves as
/// [`Index`] says.
struct IndexedProducer {
    index: Index,
    /// Every row any of this producer's cursors has minted: the producer's own count
    /// of the work it did, read independently of anything the executor reports.
    minted: Arc<AtomicU64>,
}

impl PropertyFunction for IndexedProducer {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 1)
    }

    fn modes(&self) -> &[purrdf_sparql_eval::BindingPattern] {
        static MODES: std::sync::OnceLock<Vec<purrdf_sparql_eval::BindingPattern>> =
            std::sync::OnceLock::new();
        MODES.get_or_init(|| vec![PfArity::new(1, 1).all_free_mode()])
    }

    fn rows_per_invocation(&self, _mode: purrdf_sparql_eval::BindingPattern) -> u64 {
        ROWS
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        Ok(Box::new(IndexedCursor {
            index: self.index.clone(),
            minted: Arc::clone(&self.minted),
            needle: args
                .flattened()
                .nth(1)
                .flatten()
                .cloned()
                .expect("the needle is placed"),
            emitted: 0,
        }))
    }
}

/// The cursor behind [`IndexedProducer`].
struct IndexedCursor {
    index: Index,
    minted: Arc<AtomicU64>,
    needle: TermValue,
    emitted: u64,
}

impl PfCursor for IndexedCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        if self.emitted >= ROWS {
            return Ok(None);
        }
        let row = vec![
            TermValue::iri(ex(&format!("shared/entity{:06}", self.emitted))),
            self.needle.clone(),
        ];
        self.emitted += 1;
        self.minted.fetch_add(1, Ordering::SeqCst);
        Ok(Some(row))
    }

    fn generation(&self) -> IndexGeneration {
        match self.index {
            Index::MovesUnderTheRead if self.emitted >= MOVES_AFTER => {
                IndexGeneration::declared(MOVED_TO)
            }
            _ => IndexGeneration::declared(OPENED_ON),
        }
    }

    fn service_level(&self) -> ServiceLevel {
        match self.index {
            Index::ShortFromTheStart => ServiceLevel::Incomplete {
                reason: SHORT_REASON.to_owned(),
            },
            Index::FoundShortUnderTheRead if self.emitted >= MOVES_AFTER => {
                ServiceLevel::Incomplete {
                    reason: SHORT_REASON.to_owned(),
                }
            }
            _ => ServiceLevel::Undeclared,
        }
    }
}

/// The two strata: one block, the same candidates, so the fusion certifies at its
/// sixth rank and stops both streams far inside the planned four hundred.
fn strata() -> [Iri; 2] {
    [iri(&ex("stratum/left")), iri(&ex("stratum/right"))]
}

/// The registry, with the left producer's index behaving as `left` says and the
/// right one's stable.
fn registry(left: Index) -> PropertyFunctionRegistry {
    counted_registry(left).0
}

/// [`registry`], with each producer's count of the rows it minted, in stratum order.
fn counted_registry(left: Index) -> (PropertyFunctionRegistry, [Arc<AtomicU64>; 2]) {
    let minted = [Arc::new(AtomicU64::new(0)), Arc::new(AtomicU64::new(0))];
    let mut registry = PropertyFunctionRegistry::new();
    let block = DomainTag::parse(&ex("domain/shared")).expect("a valid tag");
    for (((predicate, stratum), index), minted) in ["title", "body"]
        .into_iter()
        .zip(strata())
        .zip([left, Index::Stable])
        .zip(&minted)
    {
        registry.register_ranked(
            ex(&format!("pf/{predicate}")),
            Arc::new(IndexedProducer {
                index,
                minted: Arc::clone(minted),
            }),
            RankedDeclaration {
                stratum: purrdf_core::parse_iri(stratum.as_str()).expect("a valid IRI"),
                accepted_terms: vec![AcceptedTerm {
                    pattern: TermPattern {
                        kind: TermKind::Literal,
                        datatype: None,
                        language: Some("en".to_owned()),
                        predicate: Some(ex(predicate)),
                    },
                    placements: vec![TermPlacement {
                        facet: RequestFacet::Value,
                        position: 1,
                        datatype: None,
                    }],
                }],
                depth_placement: None,
                candidate_position: 0,
                duplicates: DuplicatePolicy::Unique,
                fidelity: RankFidelity::EXACT,
                domains: CandidateDomains::within([block.clone()]),
                block_position: None,
                exclusion: ExclusionBasis::Unavailable,
                mandatory: false,
            },
        );
    }
    (registry, minted)
}

fn request() -> RetrievalRequest {
    let terms = ["title", "body"]
        .into_iter()
        .map(|predicate| RequestTerm::Lexical {
            text: "quick brown fox".to_owned(),
            language: Some("en".to_owned()),
            predicate: Some(iri(&ex(predicate))),
        })
        .collect();
    RetrievalRequest::bounded(terms, TOP_K)
}

/// Statistics that narrow nothing.
struct Cardinalities(BTreeMap<Iri, u64>);

impl Statistics for Cardinalities {
    fn source(&self) -> &str {
        "example-statistics"
    }

    fn revision(&self) -> &str {
        "r1"
    }

    fn cardinality(&self, predicate: &Iri) -> Option<u64> {
        self.0.get(predicate).copied()
    }

    fn selectivity_ppm(&self, _subject: &Iri, _term: &RequestTerm) -> Option<u64> {
        None
    }
}

fn statistics() -> Cardinalities {
    let mut cardinalities = BTreeMap::new();
    for stratum in strata() {
        cardinalities.insert(stratum, ROWS);
    }
    for predicate in ["title", "body"] {
        cardinalities.insert(iri(&ex(predicate)), ROWS);
    }
    Cardinalities(cardinalities)
}

fn profile() -> FusionProfile {
    FusionProfile::with_decay(
        strata()
            .into_iter()
            .map(|stratum| (stratum, Fixed::ONE))
            .collect(),
        DecayRule::ReciprocalRank {
            k: u32::try_from(RECIP_K).expect("the smoothing constant fits"),
        },
    )
    .expect("the fixture profile is valid")
}

/// Run `search` over a registry whose left producer's index behaves as `left` says.
fn searched(left: Index, dataset: &RdfDataset) -> Result<SearchResult, SearchError> {
    let registry = registry(left);
    let statistics = statistics();
    let profile = profile();
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: Some(&profile),
    };
    block_on(search(
        &request(),
        &registry,
        &statistics,
        dataset,
        &env,
        &profile,
    ))
}

/// The same registry, read **materialised** — `execute`, then `fuse` — so the
/// on-demand answer has a reference that read every row the plan admitted.
fn materialised(left: Index, dataset: &RdfDataset) -> purrdf_retrieval::FusionResult<Term> {
    let registry = registry(left);
    let statistics = statistics();
    let profile = profile();
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: Some(&profile),
    };
    let planned = plan(&request(), &registry, &statistics).expect("the fixture plans");
    let compiled = compile(&planned, &env).expect("the fixture compiles");
    let execution = block_on(execute(&compiled, &registry, dataset)).expect("it executes");
    let streams = execution
        .streams
        .into_iter()
        .map(|stream| {
            let adapter =
                RankedStreamAdapter::new(stream.stream, stream.contract, &profile, &stream.stratum)
                    .expect("both strata are weighted")
                    .with_plan_id(stream.plan_id)
                    .with_fused_bound(stream.fused_bound)
                    .with_attestation(stream.attestation);
            (stream.stratum, adapter)
        })
        .collect();
    let mut fused = block_on(fuse::<RankedStreamAdapter<'_>, Term>(
        streams,
        &profile,
        compiled.fused_bound,
    ))
    .expect("the materialised streams fuse");
    fused.trailer = fused.trailer.completed_with(execution.statuses);
    fused
}

/// The attestation a stable producer opens on.
fn pinned() -> PfAttestation {
    PfAttestation {
        generation: IndexGeneration::declared(OPENED_ON),
        service: ServiceLevel::Undeclared,
    }
}

/// The protocol error a failed search carries, or a panic naming what came back.
fn protocol_error(result: Result<SearchResult, SearchError>) -> ProtocolError {
    match result {
        Err(SearchError::FusionError(FusionError::Protocol(error))) => *error,
        other => panic!("expected a protocol refusal, got {other:?}"),
    }
}

/// **The admitted neighbour, and the oracle for every refusal below.**
///
/// A stable index read on demand: the fusion stops at the sixth rank of a
/// four-hundred-row plan, so each stream is stopped mid-read — the case a receipt
/// taken only at exhaustion could not describe. Its evidence is read at the stop,
/// verifies under the sole-witness rule, and is the very attestation the producer
/// declared; the evidence identity, the rows and their order are the ones the same
/// producers give read materialised to the planned depth.
#[test]
fn a_read_stopped_mid_invocation_carries_a_verified_receipt_identical_to_the_materialised_one() {
    let dataset = common::empty_dataset();
    let result = searched(Index::Stable, dataset).expect("a stable index answers");
    let reference = materialised(Index::Stable, dataset);

    // Stopped mid-read: six rows of four hundred, one read, no probe row.
    for stratum in strata() {
        let resolution = &result.trailer.resolution[&stratum];
        assert_eq!(
            resolution.ranks_pulled, 6,
            "the fusion stopped at the sixth rank"
        );
        assert_eq!(
            resolution.rows_materialised,
            Some(6),
            "and the read produced exactly those six rows"
        );
        assert_eq!(
            reference.trailer.resolution[&stratum].rows_materialised,
            Some(ROWS),
            "where the materialised reference read every row it was admitted"
        );
    }

    // The receipt: the attestation the producer declared, per stratum, verified at
    // the stop — and the evidence the answer carries is exactly the materialised
    // read's.
    for stratum in strata() {
        assert_eq!(result.trailer.attestations[&stratum], pinned());
    }
    assert_eq!(
        result.trailer.attestations, reference.trailer.attestations,
        "the on-demand receipt attests what the materialised receipt attests"
    );
    assert_eq!(
        result.evidence_id, reference.trailer.evidence_id,
        "and the answer names the same evidence"
    );
    assert_eq!(
        result.trailer.exactness, reference.trailer.exactness,
        "so it makes the same exactness claim"
    );

    // And the answer, byte for byte: entity, score, contributions, interval and
    // threshold witness.
    assert_eq!(result.rows, reference.rows);
}

/// **An index rebuilt under the read is refused, by the witness rule.**
///
/// The left producer's cursor reports the generation it opened on for three rows
/// and a different one after. A materialised read asks the generation once, at
/// open, and never sees the move; an on-demand read settles when the fusion stops,
/// and its witness then holds the two generations one invocation served from —
/// which is exactly what the sole-witness rule refuses a finished run for. The
/// refusal is typed, names the stratum, and names the count the rule refused.
///
/// The neighbour is the stable index above, admitted with the identical answer.
#[test]
fn an_index_that_moves_under_the_read_is_refused_when_the_read_settles() {
    let dataset = common::empty_dataset();
    let error = protocol_error(searched(Index::MovesUnderTheRead, dataset));
    let ProtocolError::AttestationMoved { stratum, reason } = &error else {
        panic!("expected the read's attestation to be refused, got {error:?}");
    };
    assert_eq!(stratum, strata()[0].as_str(), "the moving stratum is named");
    assert!(
        reason.contains("2 distinct index generations"),
        "the refusal is the sole-witness rule's own: {reason}"
    );

    // The neighbour, executed here too so the pair is one test's claim: the same
    // configuration over a stable index is admitted.
    searched(Index::Stable, dataset).expect("a stable index is admitted");
}

/// **A shortfall found after the announcement is refused; one announced is not.**
///
/// The fusion certifies every row under the attestation a stream announced before
/// its first row — an attested-short index widens the intervals it certifies
/// against. A producer that opened whole and reports itself short at the stop had
/// its rows certified under a claim it no longer stands behind, so the answer is
/// refused rather than relabelled.
///
/// The neighbour is the producer short from the start: announced, so the fusion
/// certified under it, and admitted — with the shortfall on the trailer and the
/// answer's exactness downgraded, which is what tells "honoured" from "dropped".
#[test]
fn a_shortfall_found_after_the_announcement_is_refused_and_one_announced_is_carried() {
    let dataset = common::empty_dataset();

    let error = protocol_error(searched(Index::FoundShortUnderTheRead, dataset));
    let ProtocolError::AttestationMoved { stratum, reason } = &error else {
        panic!("expected the read's attestation to be refused, got {error:?}");
    };
    assert_eq!(stratum, strata()[0].as_str());
    assert!(
        reason.contains(SHORT_REASON) && reason.contains("announced"),
        "the refusal names both sides: {reason}"
    );

    let announced = searched(Index::ShortFromTheStart, dataset)
        .expect("a shortfall announced before the first row is admitted");
    let reference = materialised(Index::ShortFromTheStart, dataset);
    assert_eq!(
        announced.trailer.attestations[&strata()[0]],
        PfAttestation {
            generation: IndexGeneration::declared(OPENED_ON),
            service: ServiceLevel::Incomplete {
                reason: SHORT_REASON.to_owned(),
            },
        },
        "the shortfall is carried, not dropped"
    );
    assert_ne!(
        announced.trailer.exactness,
        searched(Index::Stable, dataset)
            .expect("a stable index answers")
            .trailer
            .exactness,
        "and it changes the claim the answer makes"
    );
    assert_eq!(
        (announced.rows, announced.trailer.exactness),
        (reference.rows, reference.trailer.exactness),
        "exactly as the materialised read of the same short index does"
    );
}

/// **A tampered announcement is refused; the true one is admitted.**
///
/// A caller composing the ladder by hand reads on demand through
/// [`execute_within`] and attaches each stream's attestation itself. One that
/// attaches an attestation the read does not end under — here, a generation the
/// producer never served — has certified every row under a forged claim, and the
/// fusion refuses the answer when the read settles.
///
/// The neighbour attaches the attestation `execute_within` handed back, and gets
/// exactly `search`'s answer.
#[test]
fn a_forged_announcement_is_refused_at_the_settlement_and_the_true_one_is_admitted() {
    let dataset = common::empty_dataset();
    let compose = |forge: bool| {
        let registry = registry(Index::Stable);
        let statistics = statistics();
        let profile = profile();
        let env = AdmissionEnvironment {
            registry: &registry,
            statistics: &statistics,
            fusion_profile: Some(&profile),
        };
        let planned = plan(&request(), &registry, &statistics).expect("the fixture plans");
        let compiled = compile(&planned, &env).expect("the fixture compiles");
        let execution = block_on(execute_within(
            &compiled,
            &registry,
            dataset,
            ReadSchedule::OnDemand,
        ))
        .expect("it executes");
        let streams = execution
            .streams
            .into_iter()
            .map(|stream| {
                let attestation = if forge && stream.stratum == strata()[0] {
                    PfAttestation {
                        generation: IndexGeneration::declared(MOVED_TO),
                        service: ServiceLevel::Undeclared,
                    }
                } else {
                    stream.attestation
                };
                let adapter = RankedStreamAdapter::new(
                    stream.stream,
                    stream.contract,
                    &profile,
                    &stream.stratum,
                )
                .expect("both strata are weighted")
                .with_plan_id(stream.plan_id)
                .with_fused_bound(stream.fused_bound)
                .with_attestation(attestation);
                (stream.stratum, adapter)
            })
            .collect();
        block_on(fuse::<RankedStreamAdapter<'_>, Term>(
            streams,
            &profile,
            compiled.fused_bound,
        ))
        .map(|fused| fused.rows)
    };

    let forged = compose(true);
    let Err(FusionError::Protocol(error)) = &forged else {
        panic!("a forged announcement must be refused, got {forged:?}");
    };
    let ProtocolError::AttestationMoved { stratum, reason } = &**error else {
        panic!("expected the announcement to be refused, got {error:?}");
    };
    assert_eq!(stratum, strata()[0].as_str());
    assert!(
        reason.contains(MOVED_TO) && reason.contains(OPENED_ON),
        "the refusal names the forged announcement and the settled truth: {reason}"
    );

    let honest = compose(false).expect("the true announcement is admitted");
    assert_eq!(
        honest,
        searched(Index::Stable, dataset)
            .expect("a stable index answers")
            .rows,
        "and the hand-composed on-demand read is `search`'s answer"
    );
}

// ---------------------------------------------------------------------------
// A caller's own text: read on demand by its shape, not by its origin
// ---------------------------------------------------------------------------

/// A caller's hand-written text for the producer at `predicate`: one call, projected,
/// with the needle every rendered unit places — the shape the on-demand read admits.
fn one_call(predicate: &str) -> String {
    format!(
        "SELECT ?candidate WHERE {{ ( ?candidate ) <{}> ( \"quick brown fox\"@en ) }}",
        ex(&format!("pf/{predicate}"))
    )
}

/// The same call under a `FILTER` every one of its rows passes: the same rows in the
/// same order, in a shape that is not one call under row-for-row operators.
fn filtered_call(predicate: &str) -> String {
    format!(
        "SELECT ?candidate WHERE {{ ( ?candidate ) <{}> ( \"quick brown fox\"@en ) \
         FILTER(isIRI(?candidate)) }}",
        ex(&format!("pf/{predicate}"))
    )
}

/// The same call joined with an empty-bindings `VALUES` row: again the same rows in
/// the same order, and again not the shape the on-demand read admits.
fn joined_call(predicate: &str) -> String {
    format!(
        "SELECT ?candidate WHERE {{ VALUES ?unused {{ UNDEF }} ( ?candidate ) <{}> \
         ( \"quick brown fox\"@en ) }}",
        ex(&format!("pf/{predicate}"))
    )
}

/// What one hand-composed read of caller-supplied units came to.
struct Supplied {
    rows: Vec<purrdf_retrieval::FusedRow>,
    /// Per stratum: `(ranks the fusion pulled, rows the stream reports it produced)`.
    resolution: Vec<(u64, Option<u64>)>,
    /// Per stratum, the rows the producer itself minted.
    minted: Vec<u64>,
    /// Per stratum, the status the executor recorded before any row was pulled.
    statuses: Vec<Option<purrdf_retrieval::ProducerStatus>>,
}

/// Compile the fixture, replace every unit with the caller text `text` writes for its
/// producer, run it under [`ReadSchedule::OnDemand`] and fuse it.
fn supplied(text: fn(&str) -> String) -> Supplied {
    let dataset = common::empty_dataset();
    let (registry, minted) = counted_registry(Index::Stable);
    let statistics = statistics();
    let profile = profile();
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: Some(&profile),
    };
    let planned = plan(&request(), &registry, &statistics).expect("the fixture plans");
    let mut compiled = compile(&planned, &env).expect("the fixture compiles");
    for (unit, predicate) in compiled.units.iter_mut().zip(["title", "body"]) {
        *unit = purrdf_retrieval::StratumUnit::new(
            unit.stratum.clone(),
            text(predicate),
            unit.contract.clone(),
            unit.depth(),
            unit.declared_rows(),
        )
        .expect("the compiler's own depth and declaration are admitted");
        assert!(
            unit.supplied_query().is_some(),
            "the unit runs the caller's text"
        );
    }
    let execution = block_on(execute_within(
        &compiled,
        &registry,
        dataset,
        ReadSchedule::OnDemand,
    ))
    .expect("it executes");
    let statuses = strata()
        .iter()
        .map(|stratum| execution.statuses.get(stratum).cloned())
        .collect();
    let streams = execution
        .streams
        .into_iter()
        .map(|stream| {
            let adapter =
                RankedStreamAdapter::new(stream.stream, stream.contract, &profile, &stream.stratum)
                    .expect("both strata are weighted")
                    .with_plan_id(stream.plan_id)
                    .with_fused_bound(stream.fused_bound)
                    .with_attestation(stream.attestation);
            (stream.stratum, adapter)
        })
        .collect();
    let fused = block_on(fuse::<RankedStreamAdapter<'_>, Term>(
        streams,
        &profile,
        compiled.fused_bound,
    ))
    .expect("the caller's units fuse");
    Supplied {
        resolution: strata()
            .iter()
            .map(|stratum| {
                let resolution = &fused.trailer.resolution[stratum];
                (resolution.ranks_pulled, resolution.rows_materialised)
            })
            .collect(),
        rows: fused.rows,
        minted: minted
            .iter()
            .map(|minted| minted.load(Ordering::SeqCst))
            .collect(),
        statuses,
    }
}

/// **A caller's own text that is one call is read on demand; one that is not is
/// materialised; both answer what the rendered units answer.**
///
/// Whether a stratum is read a row per pull is decided by its prepared text's shape.
/// The single-call text is exactly the invocation a rendered unit makes, so the fusion
/// stops it at the sixth rank having caused six rows to be minted — no probe row,
/// because the fusion never read past the depth. The `FILTER` and join texts are
/// executed too, not merely refused a cursor: each is read materialised, all four
/// hundred rows of each producer before the first is fused, and each ends naming the
/// caller's text as its stopper. All three give the rendered bundle's answer, row
/// for row.
#[test]
fn a_callers_single_call_is_read_on_demand_and_a_callers_filter_or_join_is_materialised() {
    let dataset = common::empty_dataset();
    let reference = materialised(Index::Stable, dataset);

    let on_demand = supplied(one_call);
    assert_eq!(
        on_demand.rows, reference.rows,
        "the caller's single call answers what the rendered unit answers"
    );
    assert_eq!(
        on_demand.resolution,
        vec![(6, Some(6)), (6, Some(6))],
        "read on demand: each stream produced the ranks the fusion pulled, and no probe \
         row because it never read past the depth"
    );
    assert_eq!(
        on_demand.minted,
        vec![6, 6],
        "and the producers themselves minted exactly those rows"
    );
    assert_eq!(
        on_demand.statuses,
        vec![None, None],
        "an on-demand stratum has no status until its stream is read"
    );

    for (name, text) in [
        ("FILTER", filtered_call as fn(&str) -> String),
        ("join", joined_call),
    ] {
        let read = supplied(text);
        assert_eq!(
            read.rows, reference.rows,
            "the caller's {name} answers what the rendered unit answers"
        );
        assert_eq!(
            read.minted,
            vec![ROWS, ROWS],
            "the {name} text is materialised: every row each producer holds was minted"
        );
        assert_eq!(
            read.resolution,
            vec![(6, Some(ROWS)), (6, Some(ROWS))],
            "the fusion still pulls six ranks, out of a read of every row — {name}"
        );
        assert_eq!(
            read.statuses,
            vec![
                Some(purrdf_retrieval::ProducerStatus::SuppliedQueryEnded { rank: ROWS }),
                Some(purrdf_retrieval::ProducerStatus::SuppliedQueryEnded { rank: ROWS }),
            ],
            "and it has its status before a row is pulled, naming the caller's text — {name}"
        );
    }
}

/// **A caller's single call that runs out is not certified exhausted.**
///
/// Read on demand to its end, a caller's text bounded by a `LIMIT` of its own stops
/// at three rows with the probe slot never reached; the ending names the caller's
/// text, exactly as the same text read materialised does. The neighbour is the
/// rendered unit read on demand to its end: this layer wrote its bound, the probe
/// slot existed and nothing filled it, and it is certified `Exhausted`.
#[test]
fn a_callers_single_call_read_on_demand_to_its_end_names_the_callers_text() {
    let dataset = common::empty_dataset();
    let registry = registry(Index::Stable);
    let statistics = statistics();
    let profile = profile();
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: Some(&profile),
    };
    let planned = plan(&request(), &registry, &statistics).expect("the fixture plans");
    let rendered = compile(&planned, &env).expect("the fixture compiles");
    let mut bounded = rendered.clone();
    bounded.units[0] = purrdf_retrieval::StratumUnit::new(
        bounded.units[0].stratum.clone(),
        format!("{} LIMIT 3", one_call("title")),
        bounded.units[0].contract.clone(),
        bounded.units[0].depth(),
        bounded.units[0].declared_rows(),
    )
    .expect("admitted");

    let drained = |compiled: &purrdf_retrieval::CompiledRetrieval, schedule| {
        let mut execution =
            block_on(execute_within(compiled, &registry, dataset, schedule)).expect("it runs");
        let stratum = strata()[0].clone();
        let status = execution.statuses.remove(&stratum);
        let stream = execution
            .streams
            .iter_mut()
            .find(|stream| stream.stratum == stratum)
            .expect("the stratum streamed");
        let mut rows = 0_u64;
        while block_on(stream.stream.next()).expect("it reads").is_some() {
            rows += 1;
        }
        let receipt = block_on(stream.stream.receipt()).expect("it settles");
        (
            rows,
            purrdf_retrieval::ProducerStatus::from(receipt),
            status,
        )
    };

    let (rows, ending, status) = drained(&bounded, ReadSchedule::OnDemand);
    assert_eq!(
        (rows, ending, status),
        (
            3,
            purrdf_retrieval::ProducerStatus::SuppliedQueryEnded { rank: 3 },
            None
        ),
        "read on demand, the caller's bound stopped it, and the ending says so"
    );
    let (rows, ending, status) = drained(&bounded, ReadSchedule::Materialised);
    assert_eq!(
        (rows, ending, status),
        (
            3,
            purrdf_retrieval::ProducerStatus::SuppliedQueryEnded { rank: 3 },
            Some(purrdf_retrieval::ProducerStatus::SuppliedQueryEnded { rank: 3 })
        ),
        "exactly as its materialised read ends"
    );

    let (rows, ending, _) = drained(&rendered, ReadSchedule::OnDemand);
    assert_eq!(
        (rows, ending),
        (
            ROWS,
            purrdf_retrieval::ProducerStatus::Exhausted { rows_emitted: ROWS }
        ),
        "the rendered neighbour, whose probe slot this layer wrote, is certified exhausted"
    );
}
