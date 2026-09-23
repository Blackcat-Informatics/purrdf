// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A caller's own one-call text answers exclusion lookups exactly as the text this
//! layer renders does.
//!
//! The configuration is the one whose read the lookups exist to shorten: two
//! producers declaring one shared block, naming disjoint candidates, four hundred
//! rows each, a bound of five, both `DuplicatePolicy::Unique`, both declaring
//! [`ExclusionBasis::Membership`]. Rendered, the read stops at the sixty-sixth rank
//! of each stream; with no lookup it drains all four hundred. Here both strata run a
//! caller-supplied text instead — one property-function call, written by hand — and
//! the question is whether that text's lookup is derived and asked, at the same
//! price, to the same answer.
//!
//! The refusal is pinned beside it: a supplied text that is *not* one call (a
//! `FILTER`, a join) and declares a basis is refused by name, and the same `FILTER`
//! text declaring no basis runs, drains, and is shown to have been honoured.
//!
//! Fixtures use `example.org` throughout; every IRI is fixture configuration.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

use pretty_assertions::assert_eq;
use purrdf_core::{RdfDataset, TermValue};
use purrdf_retrieval::{
    AdmissionEnvironment, CandidateDomains, CompiledRetrieval, DecayRule, DomainTag,
    DuplicatePolicy, ExecutionResult, Fixed, FusedRow, FusionError, FusionProfile, FusionTrailer,
    Iri, ProducerStatus, ProtocolError, RECIP_K, RankFidelity, RankedStreamAdapter, ReadSchedule,
    RequestTerm, RetrievalRequest, Statistics, StratumUnit, StreamContract, Term, TopK, UnitError,
    compile, contribution_under, execute_within, fuse, plan,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, EvalError, ExclusionBasis, IndexGeneration, PfArgs, PfArity,
    PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry, RankedDeclaration, RequestFacet,
    ServiceLevel, TermKind, TermPattern, TermPlacement, Volatility,
};

mod common;

const TOP_K: TopK = TopK::new(5);
const ROWS: u64 = 400;
/// The flattened argument position both producers project their candidate from.
const CANDIDATE_POSITION: usize = 0;

fn ex(suffix: &str) -> String {
    format!("http://example.org/{suffix}")
}

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

/// A minimal single-threaded executor. The producers never pend.
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

const PREDICATES: [&str; 2] = ["title", "body"];

fn strata() -> [Iri; 2] {
    [iri(&ex("stratum/left")), iri(&ex("stratum/right"))]
}

fn producer_iri(predicate: &str) -> String {
    ex(&format!("pf/{predicate}"))
}

// ---------------------------------------------------------------------------
// The producers: lazy, counted, and answering point lookups off their own IRIs.
// ---------------------------------------------------------------------------

/// What one producer was asked: the rows of each ranked read, and the
/// candidate-bound invocations — the lookups — beside how many found their
/// candidate. Counted inside the relation, so every figure the library reports is
/// checked against the producer's own.
#[derive(Debug, Default)]
struct Reads {
    ranked: Mutex<Vec<Arc<AtomicU64>>>,
    lookups: AtomicU64,
    lookups_found: AtomicU64,
}

impl Reads {
    fn rows(&self) -> Vec<u64> {
        self.ranked
            .lock()
            .expect("the fixture counters are never poisoned")
            .iter()
            .map(|counter| counter.load(Ordering::SeqCst))
            .collect()
    }
}

/// The index generations a producer's ranked read and its lookups attest.
#[derive(Clone, Copy, Debug)]
struct Generations {
    read: &'static str,
    lookups: &'static str,
}

/// A producer minting `{prefix}entity{index:06}` for every index below `ROWS`,
/// lazily, one row per pull. A call whose candidate position arrives bound is the
/// exclusion lookup, answered from the IRI's own shape rather than by a scan.
struct CountingProducer {
    arity: PfArity,
    modes: Vec<BindingPattern>,
    prefix: &'static str,
    generations: Option<Generations>,
    reads: Arc<Reads>,
}

impl CountingProducer {
    fn generation(&self, lookup: bool) -> IndexGeneration {
        self.generations
            .map_or(IndexGeneration::Undeclared, |generations| {
                IndexGeneration::declared(if lookup {
                    generations.lookups
                } else {
                    generations.read
                })
            })
    }

    fn holds_candidate(&self, candidate: &TermValue) -> bool {
        let TermValue::Iri(iri) = candidate else {
            return false;
        };
        let prefix = format!("{}entity", ex(self.prefix));
        iri.as_str().strip_prefix(&prefix).is_some_and(|index| {
            index.len() == 6
                && index.bytes().all(|byte| byte.is_ascii_digit())
                && index.parse::<u64>().is_ok_and(|index| index < ROWS)
        })
    }
}

impl PropertyFunction for CountingProducer {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        self.arity
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
        if mode.is_bound(CANDIDATE_POSITION) {
            1
        } else {
            ROWS
        }
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        let bound: Vec<Option<TermValue>> =
            args.flattened().map(Option::<&TermValue>::cloned).collect();
        if let Some(candidate) = bound[CANDIDATE_POSITION].clone() {
            self.reads.lookups.fetch_add(1, Ordering::SeqCst);
            let held = self.holds_candidate(&candidate);
            if held {
                self.reads.lookups_found.fetch_add(1, Ordering::SeqCst);
            }
            return Ok(Box::new(LookupCursor {
                generation: self.generation(true),
                row: held.then(|| {
                    bound
                        .iter()
                        .enumerate()
                        .map(|(position, value)| {
                            value.clone().unwrap_or_else(|| {
                                TermValue::iri(format!("{}score{position:06}", ex(self.prefix)))
                            })
                        })
                        .collect()
                }),
            }));
        }
        let pulled = Arc::new(AtomicU64::new(0));
        self.reads
            .ranked
            .lock()
            .expect("the fixture counters are never poisoned")
            .push(Arc::clone(&pulled));
        Ok(Box::new(CountingCursor {
            generation: self.generation(false),
            prefix: self.prefix,
            emitted: 0,
            bound,
            pulled,
        }))
    }
}

struct LookupCursor {
    row: Option<PfRow>,
    generation: IndexGeneration,
}

impl PfCursor for LookupCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(self.row.take())
    }

    fn generation(&self) -> IndexGeneration {
        self.generation.clone()
    }

    fn service_level(&self) -> ServiceLevel {
        ServiceLevel::Undeclared
    }
}

struct CountingCursor {
    generation: IndexGeneration,
    prefix: &'static str,
    emitted: u64,
    bound: Vec<Option<TermValue>>,
    pulled: Arc<AtomicU64>,
}

impl PfCursor for CountingCursor {
    fn generation(&self) -> IndexGeneration {
        self.generation.clone()
    }

    fn service_level(&self) -> ServiceLevel {
        ServiceLevel::Undeclared
    }

    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        if self.emitted >= ROWS {
            return Ok(None);
        }
        let index = self.emitted;
        self.emitted += 1;
        self.pulled.fetch_add(1, Ordering::SeqCst);
        let row = [
            TermValue::iri(format!("{}entity{index:06}", ex(self.prefix))),
            TermValue::iri(format!("{}score{index:06}", ex(self.prefix))),
        ];
        Ok(Some(
            row.iter()
                .enumerate()
                .map(|(position, value)| {
                    self.bound
                        .get(position)
                        .and_then(Clone::clone)
                        .unwrap_or_else(|| value.clone())
                })
                .collect(),
        ))
    }
}

/// Both producers, declaring `exclusion`, one shared block and disjoint
/// candidates, each attesting `generations` where given.
fn fixture_registry(
    exclusion: ExclusionBasis,
    generations: Option<Generations>,
) -> (PropertyFunctionRegistry, [Arc<Reads>; 2]) {
    let counters = [Arc::new(Reads::default()), Arc::new(Reads::default())];
    let shared = DomainTag::parse(&ex("domain/shared")).expect("the fixture tag is an IRI");
    let mut registry = PropertyFunctionRegistry::new();
    for (index, ((predicate, stratum), prefix)) in PREDICATES
        .into_iter()
        .zip(strata())
        .zip(["left/", "right/"])
        .enumerate()
    {
        let arity = PfArity::new(1, 1);
        let mut modes = vec![arity.all_free_mode()];
        if exclusion.is_declared() {
            modes.push(BindingPattern::from_bound_positions(
                arity.total(),
                [CANDIDATE_POSITION],
            ));
        }
        registry.register_ranked(
            producer_iri(predicate),
            Arc::new(CountingProducer {
                arity,
                modes,
                prefix,
                generations,
                reads: Arc::clone(&counters[index]),
            }),
            RankedDeclaration {
                stratum: purrdf_core::parse_iri(stratum.as_str()).expect("fixture IRI"),
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
                candidate_position: CANDIDATE_POSITION,
                duplicates: DuplicatePolicy::Unique,
                fidelity: RankFidelity::EXACT,
                domains: CandidateDomains::within([shared.clone()]),
                block_position: None,
                exclusion,
                mandatory: false,
            },
        );
    }
    (registry, counters)
}

struct FixtureStatistics;

impl Statistics for FixtureStatistics {
    fn source(&self) -> &'static str {
        "example-statistics"
    }

    fn revision(&self) -> &'static str {
        "r1"
    }

    fn cardinality(&self, _predicate: &Iri) -> Option<u64> {
        Some(ROWS)
    }

    fn selectivity_ppm(&self, _subject: &Iri, _term: &RequestTerm) -> Option<u64> {
        None
    }
}

const fn decay() -> DecayRule {
    DecayRule::ReciprocalRank { k: RECIP_K as u32 }
}

fn profile() -> FusionProfile {
    FusionProfile::with_decay(
        strata()
            .into_iter()
            .map(|stratum| (stratum, Fixed::ONE))
            .collect(),
        decay(),
    )
    .expect("the fixture profile is valid")
}

fn request() -> RetrievalRequest {
    let terms = PREDICATES
        .into_iter()
        .map(|predicate| RequestTerm::Lexical {
            text: "quick brown fox".to_owned(),
            language: Some("en".to_owned()),
            predicate: Some(iri(&ex(predicate))),
        })
        .collect();
    RetrievalRequest::bounded(terms, TOP_K)
}

/// The rendered bundle for `registry`: exactly what `search` would compile.
fn compiled(registry: &PropertyFunctionRegistry) -> CompiledRetrieval {
    let statistics = FixtureStatistics;
    let profile = profile();
    let env = AdmissionEnvironment {
        registry,
        statistics: &statistics,
        fusion_profile: Some(&profile),
    };
    let planned = plan(&request(), registry, &statistics).expect("the fixture plans");
    compile(&planned, &env).expect("a fresh plan is admitted")
}

/// Replace every unit's rendered text by `text(predicate)`, keeping the unit's own
/// contract, depth and declaration — so the bundle's attribution still holds.
fn supplied(
    mut bundle: CompiledRetrieval,
    text: impl Fn(&str) -> String,
) -> Result<CompiledRetrieval, UnitError> {
    for (unit, predicate) in bundle.units.iter_mut().zip(PREDICATES) {
        *unit = StratumUnit::new(
            unit.stratum.clone(),
            text(predicate),
            unit.contract.clone(),
            unit.depth(),
            unit.declared_rows(),
        )?;
        assert!(
            unit.supplied_query().is_some(),
            "the unit runs the caller's text"
        );
    }
    Ok(bundle)
}

/// The plain one-call text a caller would write for `predicate`.
fn one_call(predicate: &str) -> String {
    format!(
        "SELECT ?candidate WHERE {{ ( ?candidate ) <{}> ( \"quick brown fox\"@en ) }}",
        producer_iri(predicate)
    )
}

/// What one run cost and answered.
#[derive(Debug)]
struct Measured {
    rows: Vec<FusedRow>,
    trailer: FusionTrailer,
    /// Rows each producer's ranked reads pulled, one entry per read.
    reads: BTreeMap<Iri, Vec<u64>>,
    /// Lookups each producer served.
    served: BTreeMap<Iri, u64>,
    /// How many of those found their candidate.
    found: BTreeMap<Iri, u64>,
}

impl Measured {
    fn per_stratum<T: Clone>(
        &self,
        read: impl Fn(&purrdf_retrieval::StratumResolution) -> T,
    ) -> BTreeMap<Iri, T> {
        self.trailer
            .resolution
            .iter()
            .map(|(stratum, resolution)| (stratum.clone(), read(resolution)))
            .collect()
    }

    fn ranks_pulled(&self) -> BTreeMap<Iri, u64> {
        self.per_stratum(|resolution| resolution.ranks_pulled)
    }

    fn fused_lookups(&self) -> BTreeMap<Iri, u64> {
        self.per_stratum(|resolution| resolution.exclusion_lookups)
    }

    fn reported_materialised(&self) -> BTreeMap<Iri, Option<u64>> {
        self.per_stratum(|resolution| resolution.rows_materialised)
    }
}

fn both<T: Clone>(value: T) -> BTreeMap<Iri, T> {
    strata()
        .into_iter()
        .map(|stratum| (stratum, value.clone()))
        .collect()
}

/// Execute `bundle` on `schedule` and fuse it as `search` does.
fn run(
    bundle: &CompiledRetrieval,
    registry: &PropertyFunctionRegistry,
    counters: &[Arc<Reads>; 2],
    dataset: &RdfDataset,
    schedule: ReadSchedule,
) -> Result<Measured, FusionError> {
    let profile = profile();
    let ExecutionResult { streams, statuses } =
        block_on(execute_within(bundle, registry, dataset, schedule)).expect("the bundle runs");
    let mut adapters = Vec::new();
    for stream in streams {
        let adapter =
            RankedStreamAdapter::new(stream.stream, stream.contract, &profile, &stream.stratum)
                .expect("the profile weights every stratum");
        adapters.push((
            stream.stratum,
            adapter
                .with_plan_id(stream.plan_id)
                .with_fused_bound(stream.fused_bound)
                .with_attestation(stream.attestation),
        ));
    }
    let fused = block_on(fuse::<RankedStreamAdapter<'_>, Term>(
        adapters,
        &profile,
        bundle.fused_bound,
    ))?;
    let per_counter = |read: &dyn Fn(&Reads) -> u64| -> BTreeMap<Iri, u64> {
        strata()
            .into_iter()
            .zip(counters)
            .map(|(stratum, reads)| (stratum, read(reads)))
            .collect()
    };
    Ok(Measured {
        rows: fused.rows,
        trailer: fused.trailer.completed_with(statuses),
        reads: strata()
            .into_iter()
            .zip(counters)
            .map(|(stratum, reads)| (stratum, reads.rows()))
            .collect(),
        served: per_counter(&|reads| reads.lookups.load(Ordering::SeqCst)),
        found: per_counter(&|reads| reads.lookups_found.load(Ordering::SeqCst)),
    })
}

/// Build a fresh registry, bundle it with `text` (or rendered, for `None`), and run.
fn measure(
    exclusion: ExclusionBasis,
    generations: Option<Generations>,
    text: Option<&dyn Fn(&str) -> String>,
    schedule: ReadSchedule,
) -> Result<Measured, FusionError> {
    let (registry, counters) = fixture_registry(exclusion, generations);
    let rendered = compiled(&registry);
    let bundle = match text {
        None => rendered,
        Some(text) => supplied(rendered, text).expect("a one-call text is admitted"),
    };
    run(
        &bundle,
        &registry,
        &counters,
        common::empty_dataset(),
        schedule,
    )
}

fn measured(
    exclusion: ExclusionBasis,
    text: Option<&dyn Fn(&str) -> String>,
    schedule: ReadSchedule,
) -> Measured {
    measure(exclusion, None, text, schedule).expect("the fixture fuses")
}

fn answer(measured: &Measured) -> Vec<(String, Fixed)> {
    measured
        .rows
        .iter()
        .map(|row| (row.entity.as_str().to_owned(), row.score))
        .collect()
}

fn ceiling_at(rank: u64) -> ProducerStatus {
    ProducerStatus::CeilingReached {
        bound: contribution_under(decay(), Fixed::ONE, rank).expect("in range"),
    }
}

// ---------------------------------------------------------------------------
// The valid case: a supplied one-call text gets its lookup.
// ---------------------------------------------------------------------------

/// **A caller's one-call text asks the same lookups, pulls the same sixty-six ranks
/// and answers byte for byte what the rendered text answers.**
///
/// Three runs of one declaration set. The rendered bundle read on demand is the
/// reference price: sixty-six ranks per stream, the lookups settling finality. The
/// supplied bundle — both strata running a hand-written one-call text — must match
/// it number for number: the same lookups asked of each producer, the same
/// sixty-six ranks, one read of exactly those rows. And the materialised full read
/// of the supplied bundle, which reads every row and so needs no lookup to answer,
/// is the observing oracle for the answer: agreeing with it means the lookups
/// narrowed the read without moving a row.
///
/// The drained control — the same supplied text declaring no basis — pulls four
/// hundred ranks, so sixty-six here is the lookup being honoured and not the
/// fixture being short.
#[test]
fn a_supplied_one_call_text_is_looked_up_exactly_as_the_rendered_unit_is() {
    let rendered = measured(ExclusionBasis::Membership, None, ReadSchedule::OnDemand);
    let text: &dyn Fn(&str) -> String = &one_call;
    let on_demand = measured(
        ExclusionBasis::Membership,
        Some(text),
        ReadSchedule::OnDemand,
    );
    let full = measured(
        ExclusionBasis::Membership,
        Some(text),
        ReadSchedule::Materialised,
    );
    let drained = measured(
        ExclusionBasis::Unavailable,
        Some(text),
        ReadSchedule::OnDemand,
    );

    assert_eq!(
        rendered.ranks_pulled(),
        both(66),
        "the rendered reference price"
    );
    assert_eq!(
        on_demand.ranks_pulled(),
        both(66),
        "the supplied text stops at the rendered text's rank"
    );
    assert_eq!(
        on_demand.reads,
        both(vec![66]),
        "one read per stratum, of exactly the ranks the fusion pulled"
    );
    assert_eq!(
        on_demand.reported_materialised(),
        both(Some(66)),
        "and the trailer bills that read and nothing else"
    );
    assert_eq!(
        on_demand.trailer.statuses, rendered.trailer.statuses,
        "the fusion, not the read, stopped both streams"
    );
    assert_eq!(
        on_demand.trailer.statuses.get(&strata()[0]),
        Some(&ceiling_at(66))
    );

    // The lookups: asked, of both producers, exactly as often as the rendered
    // text asks them, and served by the producer they were aimed at.
    let asked = on_demand.fused_lookups();
    assert!(
        asked.values().all(|&count| count > 0),
        "both strata asked: {asked:?}"
    );
    assert_eq!(
        asked,
        rendered.fused_lookups(),
        "as often as the rendered text asks"
    );
    assert_eq!(
        asked, on_demand.served,
        "and each was served by its own producer"
    );
    assert_eq!(
        on_demand.found,
        both(0),
        "every lookup of a disjoint candidate is an exclusion"
    );

    // The control: no basis, no lookup, and the corpus drained.
    assert_eq!(drained.ranks_pulled(), both(400), "the drained control");
    assert_eq!(drained.fused_lookups(), both(0));

    // The answer, byte for byte, against the rendered run and the full read.
    assert_eq!(on_demand.rows, rendered.rows, "the rendered text's answer");
    assert_eq!(full.reads, both(vec![ROWS]), "the reference reads in full");
    assert_eq!(on_demand.rows, full.rows, "the materialised read's answer");
    assert_eq!(
        answer(&on_demand),
        answer(&drained),
        "and the drained control's"
    );
}

/// **A renaming projection and a renaming `BIND` still carry the candidate back to
/// the call position it came from.**
///
/// The candidate column is `?hit` renamed, once by a projection expression in a
/// nested `SELECT` and once by a `BIND` — both row-for-row operators the shape
/// admits. The lookup is derived through the renaming, so each text asks exactly
/// what the plain one asks and stops at the same rank.
#[test]
fn a_renamed_candidate_column_still_gets_its_lookup() {
    let plain_text: &dyn Fn(&str) -> String = &one_call;
    let plain = measured(
        ExclusionBasis::Membership,
        Some(plain_text),
        ReadSchedule::OnDemand,
    );
    let projected: &dyn Fn(&str) -> String = &|predicate: &str| {
        format!(
            "SELECT ?candidate WHERE {{ {{ SELECT (?hit AS ?candidate) WHERE {{ ?hit <{}> \
             \"quick brown fox\"@en }} }} }}",
            producer_iri(predicate)
        )
    };
    let bound: &dyn Fn(&str) -> String = &|predicate: &str| {
        format!(
            "SELECT ?candidate WHERE {{ ?hit <{}> \"quick brown fox\"@en BIND(?hit AS \
             ?candidate) }}",
            producer_iri(predicate)
        )
    };
    for (name, text) in [("projection", projected), ("BIND", bound)] {
        let renamed = measured(
            ExclusionBasis::Membership,
            Some(text),
            ReadSchedule::OnDemand,
        );
        assert_eq!(
            renamed.ranks_pulled(),
            both(66),
            "{name}: the lookup settled finality"
        );
        assert_eq!(
            renamed.fused_lookups(),
            plain.fused_lookups(),
            "{name}: the same lookups the plain text asks"
        );
        assert_eq!(
            renamed.served,
            renamed.fused_lookups(),
            "{name}: served by the producer"
        );
        assert_eq!(renamed.rows, plain.rows, "{name}: the same answer");
    }
}

// ---------------------------------------------------------------------------
// The refusal: a text that is not one call cannot declare a basis.
// ---------------------------------------------------------------------------

/// A `FILTER` that removes exactly one candidate the left producer names first.
fn filtered(predicate: &str) -> String {
    format!(
        "SELECT ?candidate WHERE {{ ( ?candidate ) <{}> ( \"quick brown fox\"@en ) \
         FILTER(?candidate != <{}>) }}",
        producer_iri(predicate),
        ex("left/entity000000")
    )
}

/// A join of the call with a second call.
fn joined(predicate: &str) -> String {
    format!(
        "SELECT ?candidate WHERE {{ ( ?candidate ) <{p}> ( \"quick brown fox\"@en ) . \
         ( ?candidate ) <{p}> ( \"lazy dog\"@en ) }}",
        p = producer_iri(predicate)
    )
}

/// **A `FILTER` text and a join text declaring a basis are refused by name; the
/// same `FILTER` text declaring none runs, drains, and is shown to have run.**
///
/// The refusal names what stands where only the call may: the shape check's own
/// words for the `Filter` and the `Join` node. The neighbour is the same filtered
/// text with [`ExclusionBasis::Unavailable`] — it is admitted, read materialised
/// (it is not one call), drains the four hundred rows, and its answer is the
/// control's answer with the one filtered candidate gone: the left stream's ranks
/// all move up by one, so the fused scores are the control's scores exactly and the
/// left-hand entities are the control's shifted by one index. An answer that still
/// held `left/entity000000` would be the filter dropped; one that differed in any
/// score would be the text misread.
#[test]
fn a_supplied_text_that_is_not_one_call_cannot_declare_a_basis_and_its_neighbour_runs() {
    let (registry, _) = fixture_registry(ExclusionBasis::Membership, None);
    let rendered = compiled(&registry);
    for (name, text, node) in [
        ("FILTER", filtered as fn(&str) -> String, "Filter"),
        (
            "join",
            joined as fn(&str) -> String,
            "join of the call with another pattern",
        ),
    ] {
        let refused = supplied(rendered.clone(), text)
            .expect_err("a text that is not one call cannot declare a basis");
        let UnitError::ExclusionNotRenderable { basis, reason } = &refused else {
            panic!("{name}: expected ExclusionNotRenderable, got {refused:?}");
        };
        assert_eq!(*basis, "membership", "{name}: the refusal names the basis");
        assert!(
            reason.contains(&format!("a {node}")),
            "{name}: the refusal names the node in the way: {reason}"
        );
        assert!(
            refused.to_string().contains("one property-function call"),
            "{name}: and says what shape a lookup needs: {refused}"
        );
    }

    // The neighbour: the same FILTER text, declaring no basis.
    let (registry, counters) = fixture_registry(ExclusionBasis::Unavailable, None);
    let bundle = supplied(compiled(&registry), filtered).expect("no basis, nothing to derive");
    let neighbour = run(
        &bundle,
        &registry,
        &counters,
        common::empty_dataset(),
        ReadSchedule::OnDemand,
    )
    .expect("the filtered text fuses");
    let control = measured(ExclusionBasis::Unavailable, None, ReadSchedule::OnDemand);

    assert_eq!(
        neighbour.reads,
        both(vec![ROWS]),
        "it drains, once per stratum"
    );
    assert_eq!(neighbour.fused_lookups(), both(0), "and asks nothing");
    let removed = format!("<{}>", ex("left/entity000000"));
    assert!(
        answer(&control)
            .iter()
            .any(|(entity, _)| *entity == removed),
        "the control names the filtered candidate"
    );
    let shifted: Vec<(String, Fixed)> = answer(&control)
        .into_iter()
        .map(|(entity, score)| {
            let left = format!("<{}", ex("left/entity"));
            let entity = match entity.strip_prefix(&left) {
                Some(index) => {
                    let index: u64 = index
                        .trim_end_matches('>')
                        .parse()
                        .expect("the fixture's own index");
                    format!("{left}{:06}>", index + 1)
                }
                None => entity,
            };
            (entity, score)
        })
        .collect();
    assert_eq!(
        answer(&neighbour),
        shifted,
        "the filter ran: the left stream's candidates move up one rank, every score \
         stands"
    );
}

// ---------------------------------------------------------------------------
// The supplied lookup is held to the generation its read pinned.
// ---------------------------------------------------------------------------

/// **A supplied text's lookup answered by a moved index is refused; one answered by
/// the pinned index is admitted.**
///
/// The ranked reads open on `gen-7`. In the refused run the lookups are answered by
/// `gen-8`; every verdict they give is the verdict `gen-7` would give, so the
/// refusal is keyed on the evidence and not on the answer. In the neighbour both are
/// `gen-7`, and the run asks, stops at the sixty-sixth rank, and answers exactly
/// what the drained control answers.
#[test]
fn a_supplied_lookup_from_a_moved_index_is_refused_and_one_at_the_pinned_generation_is_admitted() {
    let text: &dyn Fn(&str) -> String = &one_call;
    let moved = measure(
        ExclusionBasis::Membership,
        Some(Generations {
            read: "gen-7",
            lookups: "gen-8",
        }),
        Some(text),
        ReadSchedule::OnDemand,
    );
    let refused = match moved {
        Err(FusionError::Protocol(error)) => *error,
        other => panic!(
            "a lookup answered by a moved index must fail the fusion, got {:?}",
            other.map(|measured| measured.rows.len())
        ),
    };
    let ProtocolError::ExclusionAttestationMoved { stratum, reason } = &refused else {
        panic!("expected the lookup's attestation to be refused, got {refused:?}");
    };
    assert!(
        strata().iter().any(|known| known.as_str() == stratum),
        "the refusal names the stratum: {refused}"
    );
    assert!(
        reason.contains("gen-7") && reason.contains("gen-8"),
        "and both generations: {reason}"
    );

    let stable = measure(
        ExclusionBasis::Membership,
        Some(Generations {
            read: "gen-7",
            lookups: "gen-7",
        }),
        Some(text),
        ReadSchedule::OnDemand,
    )
    .expect("a lookup at the pinned generation is admitted");
    let control = measured(
        ExclusionBasis::Unavailable,
        Some(text),
        ReadSchedule::OnDemand,
    );
    assert!(
        stable.fused_lookups().values().all(|&count| count > 0),
        "the neighbour asked on both strata"
    );
    assert_eq!(
        stable.ranks_pulled(),
        both(66),
        "and its answers shortened the read"
    );
    assert_eq!(
        answer(&stable),
        answer(&control),
        "without moving the answer"
    );
}

// ---------------------------------------------------------------------------
// The registry is the authority on what the call is.
// ---------------------------------------------------------------------------

/// **A text the construction-time reading admits but the registry does not make one
/// call of fails its own stratum, by name; the registered neighbour runs.**
///
/// `StratumUnit::new` has no registry, so it reads every bare predicate as a call.
/// A text calling an IRI nothing registered is one call under that reading and a
/// triple pattern under the registry's, so the lookup cannot be derived at
/// execution: the stratum is `ExecutionFailed`, naming the reason, and the other
/// stratum — whose text is the registered call — still runs and asks.
#[test]
fn a_supplied_text_whose_call_the_registry_does_not_register_fails_its_own_stratum() {
    let (registry, counters) = fixture_registry(ExclusionBasis::Membership, None);
    let bundle = supplied(compiled(&registry), |predicate| {
        if predicate == "title" {
            format!(
                "SELECT ?candidate WHERE {{ ?candidate <{}> \"quick brown fox\"@en }}",
                ex("pf/unregistered")
            )
        } else {
            one_call(predicate)
        }
    })
    .expect("under the widest reading both texts are one call");
    let ExecutionResult { streams, statuses } = block_on(execute_within(
        &bundle,
        &registry,
        common::empty_dataset(),
        ReadSchedule::OnDemand,
    ))
    .expect("the bundle runs");
    let [left, right] = strata();
    let Some(ProducerStatus::ExecutionFailed { reason }) = statuses.get(&left) else {
        panic!("the unregistered call fails its stratum: {statuses:?}");
    };
    assert!(
        reason.contains("could not be derived from its query") && reason.contains("a Bgp node"),
        "naming why: {reason}"
    );
    assert_eq!(
        streams
            .iter()
            .map(|stream| stream.stratum.clone())
            .collect::<Vec<_>>(),
        vec![right],
        "and the registered neighbour still runs"
    );
    assert_eq!(
        counters[0].rows(),
        Vec::<u64>::new(),
        "nothing read the left producer"
    );
}

/// **A supplied text whose contract claims a basis the registry does not declare
/// for its call fails its stratum; the matching declaration is admitted.**
#[test]
fn a_supplied_basis_the_registry_does_not_declare_fails_its_own_stratum() {
    // The registry declares no basis; the unit's contract claims membership.
    let (registry, counters) = fixture_registry(ExclusionBasis::Unavailable, None);
    let mut bundle = compiled(&registry);
    for (unit, predicate) in bundle.units.iter_mut().zip(PREDICATES) {
        let contract = StreamContract {
            exclusion: ExclusionBasis::Membership,
            ..unit.contract.clone()
        };
        *unit = StratumUnit::new(
            unit.stratum.clone(),
            one_call(predicate),
            contract,
            unit.depth(),
            unit.declared_rows(),
        )
        .expect("a one-call text may declare a basis");
    }
    let bundle = CompiledRetrieval::new(
        bundle.units,
        bundle.plan_id,
        bundle.registry_id,
        bundle.registry_fingerprint,
        bundle.fused_bound,
        bundle.resolution,
    );
    let ExecutionResult { streams, statuses } = block_on(execute_within(
        &bundle,
        &registry,
        common::empty_dataset(),
        ReadSchedule::OnDemand,
    ))
    .expect("the bundle runs");
    assert!(streams.is_empty(), "neither stratum can honour the claim");
    for stratum in strata() {
        let Some(ProducerStatus::ExecutionFailed { reason }) = statuses.get(&stratum) else {
            panic!("{stratum}: expected a failed stratum, got {statuses:?}");
        };
        assert!(
            reason.contains("declares an exclusion basis of membership")
                && reason.contains("the registry declares unavailable"),
            "{stratum}: naming both declarations: {reason}"
        );
    }
    assert!(
        counters.iter().all(|reads| reads.rows().is_empty()),
        "nothing was read"
    );
    // The neighbour — the registry declaring the basis the contract claims — is the
    // valid run pinned above.
    let admitted = measured(
        ExclusionBasis::Membership,
        Some(&one_call),
        ReadSchedule::OnDemand,
    );
    assert!(admitted.fused_lookups().values().all(|&count| count > 0));
}
