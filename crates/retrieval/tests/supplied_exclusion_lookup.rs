// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A caller's own text answers exclusion lookups exactly as the text this layer
//! renders does, whenever its `?candidate` column is drawn from calls.
//!
//! The configuration is the one whose read the lookups exist to shorten: two
//! producers declaring one shared block, naming disjoint candidates, four hundred
//! rows each, a bound of five, both `DuplicatePolicy::Unique`, both declaring
//! [`ExclusionBasis::Membership`]. Rendered, the read stops at the sixty-sixth rank
//! of each stream; with no lookup it drains all four hundred. Here both strata run a
//! caller-supplied text instead — one property-function call written by hand, the
//! call under a `FILTER`, a join of two calls — and the question is whether that
//! text's lookups are derived and asked, at the same price, to the same answer, and
//! whether a `FILTER` over one call is read on demand with its dropped rows taking no
//! rank. The configuration whose candidates the two producers share is run through
//! the `FILTER` text too.
//!
//! The refusals are pinned beside admitted neighbours: a text whose `?candidate`
//! column can take a value no call emitted — through a `UNION`, an `OPTIONAL` or a
//! `MINUS` over `VALUES`, an aggregate, a `GRAPH` name, a data triple — and declares
//! a basis is refused by name, and the neighbour differing in that one operator is
//! admitted and answers its lookups.
//!
//! Fixtures use `example.org` throughout; every IRI is fixture configuration.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

use pretty_assertions::assert_eq;
use purrdf_core::{RdfDataset, TermValue};
use purrdf_retrieval::{
    AdmissionEnvironment, CandidateDomains, CompiledRetrieval, DecayRule, DomainTag,
    DuplicatePolicy, ExclusionVerdict, ExecutionResult, Fixed, FusedRow, FusionError,
    FusionProfile, FusionTrailer, Iri, ProducerStatus, ProtocolError, RECIP_K, RankFidelity,
    RankedStreamAdapter, ReadSchedule, RequestTerm, RetrievalRequest, Statistics, StratumUnit,
    StreamContract, Term, TopK, UnitError, compile, contribution_under, execute_within, fuse, plan,
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
/// exclusion lookup, answered from the IRI's own shape rather than by a scan — except
/// that a call asked for `"everything"@en` holds every candidate.
///
/// Where `moves` is given and set, a bound call asked for `"lazy dog"@en` answers
/// from `gen-8`: the index behind that one call moved after the ranked read.
struct CountingProducer {
    arity: PfArity,
    modes: Vec<BindingPattern>,
    prefix: &'static str,
    generations: Option<Generations>,
    moves: Option<Arc<AtomicBool>>,
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
            let asked = bound.get(1).cloned().flatten();
            let held = asked == Some(TermValue::lang_literal("everything", "en"))
                || self.holds_candidate(&candidate);
            if held {
                self.reads.lookups_found.fetch_add(1, Ordering::SeqCst);
            }
            let moved = self
                .moves
                .as_ref()
                .is_some_and(|moves| moves.load(Ordering::SeqCst))
                && asked == Some(TermValue::lang_literal("lazy dog", "en"));
            return Ok(Box::new(LookupCursor {
                generation: if moved {
                    IndexGeneration::declared("gen-8")
                } else {
                    self.generation(true)
                },
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

/// The producers' candidate prefixes when no candidate is named twice.
const DISJOINT: [&str; 2] = ["left/", "right/"];
/// The producers' candidate prefixes when both name the same candidates in the same
/// order.
const INTERSECTING: [&str; 2] = ["shared/", "shared/"];

/// Both producers, declaring `exclusion`, one shared block and disjoint
/// candidates, each attesting `generations` where given.
fn fixture_registry(
    exclusion: ExclusionBasis,
    generations: Option<Generations>,
) -> (PropertyFunctionRegistry, [Arc<Reads>; 2]) {
    shaped_registry(exclusion, generations, DISJOINT, None)
}

/// Both producers, declaring `exclusion` and one shared block, naming candidates
/// under `prefixes`, each attesting `generations` where given and moving its
/// `"lazy dog"` index when `moves` is set.
fn shaped_registry(
    exclusion: ExclusionBasis,
    generations: Option<Generations>,
    prefixes: [&'static str; 2],
    moves: Option<&Arc<AtomicBool>>,
) -> (PropertyFunctionRegistry, [Arc<Reads>; 2]) {
    let counters = [Arc::new(Reads::default()), Arc::new(Reads::default())];
    let shared = DomainTag::parse(&ex("domain/shared")).expect("the fixture tag is an IRI");
    let mut registry = PropertyFunctionRegistry::new();
    for (index, ((predicate, stratum), prefix)) in PREDICATES
        .into_iter()
        .zip(strata())
        .zip(prefixes)
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
                moves: moves.cloned(),
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
    run_then(bundle, registry, counters, dataset, schedule, &|| {})
}

/// [`run`], calling `between` once every stratum has been executed and before the
/// fusion asks anything — the instant an index can move under a read that pinned it.
fn run_then(
    bundle: &CompiledRetrieval,
    registry: &PropertyFunctionRegistry,
    counters: &[Arc<Reads>; 2],
    dataset: &RdfDataset,
    schedule: ReadSchedule,
    between: &dyn Fn(),
) -> Result<Measured, FusionError> {
    let profile = profile();
    let ExecutionResult { streams, statuses } =
        block_on(execute_within(bundle, registry, dataset, schedule)).expect("the bundle runs");
    between();
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
    measure_shaped(exclusion, generations, text, schedule, DISJOINT)
}

/// [`measure`], over producers naming candidates under `prefixes`.
fn measure_shaped(
    exclusion: ExclusionBasis,
    generations: Option<Generations>,
    text: Option<&dyn Fn(&str) -> String>,
    schedule: ReadSchedule,
    prefixes: [&'static str; 2],
) -> Result<Measured, FusionError> {
    let (registry, counters) = shaped_registry(exclusion, generations, prefixes, None);
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
// A FILTER over the call: looked up, and read on demand.
// ---------------------------------------------------------------------------

/// A `FILTER` that removes exactly one candidate: `removed`, the first the producer
/// naming it names.
fn filtering(removed: &str) -> impl Fn(&str) -> String + '_ {
    move |predicate: &str| {
        format!(
            "SELECT ?candidate WHERE {{ ( ?candidate ) <{}> ( \"quick brown fox\"@en ) \
             FILTER(?candidate != <{}>) }}",
            producer_iri(predicate),
            ex(removed)
        )
    }
}

/// A `FILTER` that removes exactly one candidate the left producer names first.
fn filtered(predicate: &str) -> String {
    filtering("left/entity000000")(predicate)
}

/// A join of the call with a second call on the same candidate.
fn joined(predicate: &str) -> String {
    format!(
        "SELECT ?candidate WHERE {{ ( ?candidate ) <{p}> ( \"quick brown fox\"@en ) . \
         ( ?candidate ) <{p}> ( \"lazy dog\"@en ) }}",
        p = producer_iri(predicate)
    )
}

/// `answer`, with every left-hand entity moved `by` indices down its stream: what the
/// control answers once a `FILTER` removed the left stream's first `by` candidates.
/// Every rank after them moves up by `by`, so every fused score stands and only the
/// left entities change.
fn shifted_left(answer: Vec<(String, Fixed)>, by: u64) -> Vec<(String, Fixed)> {
    let left = format!("<{}", ex("left/entity"));
    answer
        .into_iter()
        .map(|(entity, score)| {
            let entity = match entity.strip_prefix(&left) {
                Some(index) => {
                    let index: u64 = index
                        .trim_end_matches('>')
                        .parse()
                        .expect("the fixture's own index");
                    format!("{left}{:06}>", index + by)
                }
                None => entity,
            };
            (entity, score)
        })
        .collect()
}

/// **A `FILTER` over the call is looked up and read on demand: the lookups are asked,
/// the ranks are the unfiltered text's, the removed candidate is gone, and one row
/// more is read on the stream it was removed from — not four hundred.**
///
/// The configuration the lookups exist for, through a text removing
/// `left/entity000000`. Read on demand, each stratum is one invocation the `FILTER`
/// is applied to as rows are pulled: the removed row is read and takes no rank, so
/// the left stream's ranks are the unfiltered stream's shifted by one candidate and
/// the fusion stops both streams at the sixty-sixth rank, as it stops the unfiltered
/// text — the left producer having minted sixty-seven rows, the right sixty-six. The
/// lookups settle finality as the unfiltered text's do, asked of both producers and
/// served by them.
///
/// The observing oracle is the materialised full read of the same filtered text —
/// four hundred rows each, needing no lookup to answer — and the unfiltered control's
/// answer with every left entity moved one index down: an answer still holding
/// `left/entity000000` would be the `FILTER` dropped; one differing in any score, the
/// text misread.
///
/// The neighbour is the same filtered text declaring no basis: nothing to ask, so
/// nothing is final while the other stream is open, and it drains — both producers
/// minting all four hundred rows — to the same answer.
#[test]
fn a_filter_over_the_call_is_looked_up_and_read_on_demand_and_its_unavailable_neighbour_drains() {
    let text: &dyn Fn(&str) -> String = &filtered;
    let plain: &dyn Fn(&str) -> String = &one_call;
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
    let unfiltered = measured(
        ExclusionBasis::Membership,
        Some(plain),
        ReadSchedule::OnDemand,
    );
    let drained = measured(
        ExclusionBasis::Unavailable,
        Some(text),
        ReadSchedule::OnDemand,
    );
    let control = measured(ExclusionBasis::Unavailable, None, ReadSchedule::OnDemand);
    let [left, right] = strata();

    // The price: the unfiltered text's ranks, read on demand, the removed row read
    // and never ranked.
    assert_eq!(
        on_demand.ranks_pulled(),
        both(66),
        "the unfiltered text's ranks"
    );
    assert_eq!(on_demand.ranks_pulled(), unfiltered.ranks_pulled());
    assert_eq!(
        on_demand.reads,
        BTreeMap::from([(left.clone(), vec![67]), (right.clone(), vec![66])]),
        "one read per stratum, on demand: the left read one row the FILTER dropped"
    );
    assert_eq!(
        on_demand.reported_materialised(),
        both(Some(66)),
        "and the trailer bills the rows the read yielded, as a materialised read of the \
         text bills its answer's rows"
    );
    assert_eq!(full.reads, both(vec![ROWS]), "the reference reads in full");

    // The lookups: asked of both producers, exactly as often as the unfiltered text
    // asks them, and every one an exclusion of a disjoint candidate.
    let asked = on_demand.fused_lookups();
    assert_eq!(
        asked,
        both(65),
        "both strata asked, one verdict for each of the other stream's first sixty-five \
         candidates"
    );
    assert_eq!(
        asked,
        unfiltered.fused_lookups(),
        "as the unfiltered text asks"
    );
    assert_eq!(asked, on_demand.served, "each served by its own producer");
    assert_eq!(on_demand.found, both(0));

    // The answer.
    let removed = format!("<{}>", ex("left/entity000000"));
    assert!(
        answer(&control)
            .iter()
            .any(|(entity, _)| *entity == removed),
        "the control names the filtered candidate"
    );
    assert!(
        answer(&on_demand)
            .iter()
            .all(|(entity, _)| *entity != removed),
        "the filtered candidate is gone"
    );
    assert_eq!(on_demand.rows, full.rows, "the materialised read's answer");
    assert_eq!(
        answer(&on_demand),
        shifted_left(answer(&control), 1),
        "the left stream's candidates move up one rank, every score stands"
    );

    // The neighbour: no basis, nothing asked, drained — and the same answer.
    assert_eq!(drained.fused_lookups(), both(0), "it asks nothing");
    assert_eq!(
        drained.reads,
        both(vec![ROWS]),
        "it drains, once per stratum"
    );
    assert_eq!(
        drained.ranks_pulled(),
        BTreeMap::from([(left, ROWS - 1), (right, ROWS)]),
        "every row ranked but the one the FILTER dropped"
    );
    assert_eq!(answer(&drained), answer(&on_demand), "to the same answer");
}

/// **The configuration whose producers name the same candidates, through a `FILTER`
/// text, reads seven rows of each producer on demand, not four hundred.**
///
/// Both producers name `shared/entity000000` first, and the text removes it. Read on
/// demand the fusion certifies at the sixth rank, as it does over the unfiltered text
/// — each stream's ranks are the unfiltered stream's shifted by the one removed
/// candidate — and each producer mints seven rows: the removed one and the six
/// ranked. No basis is declared: this configuration is final at its sixth rank
/// without asking anything. Materialised, the same text reads all four hundred of
/// each, to the same answer.
#[test]
fn the_shared_candidate_configuration_through_a_filter_reads_on_demand() {
    let text = filtering("shared/entity000000");
    let text: &dyn Fn(&str) -> String = &text;
    let plain: &dyn Fn(&str) -> String = &one_call;
    let run = |text, schedule| {
        measure_shaped(
            ExclusionBasis::Unavailable,
            None,
            Some(text),
            schedule,
            INTERSECTING,
        )
        .expect("the fixture fuses")
    };
    let on_demand = run(text, ReadSchedule::OnDemand);
    let full = run(text, ReadSchedule::Materialised);
    let unfiltered = run(plain, ReadSchedule::OnDemand);

    assert_eq!(unfiltered.reads, both(vec![6]), "the unfiltered price");
    assert_eq!(unfiltered.ranks_pulled(), both(6));
    assert_eq!(
        on_demand.reads,
        both(vec![7]),
        "one row more per producer: the removed candidate, read and dropped"
    );
    assert_eq!(
        on_demand.ranks_pulled(),
        both(6),
        "and six ranks, as unfiltered"
    );
    assert_eq!(on_demand.reported_materialised(), both(Some(6)));
    assert_eq!(on_demand.fused_lookups(), both(0), "nothing was asked");
    assert_eq!(
        full.reads,
        both(vec![ROWS]),
        "materialised, it reads every row"
    );
    assert_eq!(on_demand.rows, full.rows, "to the same answer");
    let removed = format!("<{}>", ex("shared/entity000000"));
    assert!(
        answer(&unfiltered)
            .iter()
            .any(|(entity, _)| *entity == removed)
            && answer(&on_demand)
                .iter()
                .all(|(entity, _)| *entity != removed),
        "the removed candidate leads the unfiltered answer and is gone from this one"
    );
}

/// **A `FILTER` removing the ten candidates the left producer names first is read on
/// demand to the materialised answer, and counts its ranks after the `FILTER`.**
///
/// The left producer mints its first ten rows only to have them dropped, so its read
/// is ten rows longer than the ranks the fusion pulled off it; the right stream, which
/// the `FILTER` removes nothing from, reads exactly its ranks. The answer is the
/// materialised full read's, and the unfiltered control's with every left entity
/// moved ten indices down.
#[test]
fn a_filter_removing_many_leading_rows_counts_ranks_after_the_filter() {
    let text = |predicate: &str| {
        format!(
            "SELECT ?candidate WHERE {{ ( ?candidate ) <{}> ( \"quick brown fox\"@en ) \
             FILTER(!STRSTARTS(STR(?candidate), \"{}\")) }}",
            producer_iri(predicate),
            ex("left/entity00000")
        )
    };
    let text: &dyn Fn(&str) -> String = &text;
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
    let control = measured(ExclusionBasis::Unavailable, None, ReadSchedule::OnDemand);
    let [left, right] = strata();

    assert_eq!(on_demand.ranks_pulled(), both(66));
    assert_eq!(
        on_demand.reads,
        BTreeMap::from([(left, vec![76]), (right, vec![66])]),
        "the left read ten rows the FILTER dropped before its first rank"
    );
    assert_eq!(
        on_demand.fused_lookups(),
        both(65),
        "the lookups were asked"
    );
    assert_eq!(
        on_demand.served,
        both(65),
        "and served, one invocation each"
    );
    assert_eq!(on_demand.rows, full.rows, "the materialised read's answer");
    assert_eq!(
        answer(&on_demand),
        shifted_left(answer(&control), 10),
        "the left stream's candidates move up ten ranks, every score stands"
    );
}

// ---------------------------------------------------------------------------
// A join of calls: every call a source, every call asked.
// ---------------------------------------------------------------------------

/// **A join of two calls on the candidate asks its lookups and stops at the
/// sixty-sixth rank; with no basis it drains to the same answer.**
///
/// The text joins `"quick brown fox"` and `"lazy dog"` on `?candidate`. A join of two
/// ranked reads is not one ranked read, so it is materialised under either schedule;
/// but every value its candidate column takes is a value each call emitted, so each
/// call's lookup is a proof of absence and both are derived. The fusion asks and
/// stops at the sixty-sixth rank of each stream, where the same text declaring no
/// basis — the materialised read with no lookups — pulls all four hundred ranks, and
/// answers exactly what that run answers.
///
/// The producer's own counters are the oracle for what was asked: the ranked read of
/// the join invokes the second call once per row of the first, with the candidate
/// bound, so each producer served four hundred bound invocations for the read itself,
/// all found; every lookup beyond those is the fusion's, and each is answered by the
/// first call it asks — it excludes a candidate the other producer named — so each
/// verdict costs exactly one invocation and finds nothing.
#[test]
fn a_join_of_calls_is_looked_up_through_each_call_and_answers_as_its_drained_neighbour() {
    let text: &dyn Fn(&str) -> String = &joined;
    let looked_up = measured(
        ExclusionBasis::Membership,
        Some(text),
        ReadSchedule::OnDemand,
    );
    let materialised = measured(
        ExclusionBasis::Membership,
        Some(text),
        ReadSchedule::Materialised,
    );
    let drained = measured(
        ExclusionBasis::Unavailable,
        Some(text),
        ReadSchedule::Materialised,
    );

    assert_eq!(
        looked_up.reads,
        both(vec![ROWS]),
        "materialised: one read of every row"
    );
    assert_eq!(
        looked_up.ranks_pulled(),
        both(66),
        "the lookups settled finality"
    );
    assert_eq!(
        drained.ranks_pulled(),
        both(ROWS),
        "no lookup, and it drains"
    );
    let asked = looked_up.fused_lookups();
    assert_eq!(asked, both(65), "both asked, as the one-call text asks");
    let join_invocations = both(ROWS);
    assert_eq!(
        looked_up.served,
        asked
            .iter()
            .map(|(stratum, count)| (stratum.clone(), count + join_invocations[stratum]))
            .collect::<BTreeMap<_, _>>(),
        "one invocation per verdict, beside the join's own"
    );
    assert_eq!(
        looked_up.served,
        both(465),
        "four hundred for the join, sixty-five asked"
    );
    assert_eq!(
        looked_up.found, join_invocations,
        "and every verdict an exclusion"
    );
    assert_eq!(
        drained.served, join_invocations,
        "the drained run asks nothing"
    );
    assert_eq!(
        materialised.rows, looked_up.rows,
        "the join is materialised under either schedule"
    );
    assert_eq!(
        answer(&looked_up),
        answer(&drained),
        "the answer the run with no lookups gives, entity for entity and score for score"
    );
}

/// **A lookup served by a moved index is refused, whichever call it was asked of; a
/// lookup at the pinned generation is admitted.**
///
/// The text joins `"everything"` — a call holding every candidate — with
/// `"lazy dog"`, so every verdict reaches the `"lazy dog"` lookup, whichever the
/// stratum asks first. The ranked reads pin `gen-7`, and every invocation of the read
/// answers from it. In the refused run the index behind the `"lazy dog"` call moves
/// once execution is done, so its lookups answer from `gen-8`: the fusion fails naming
/// the stratum and both generations, although every verdict `gen-8` gives is the
/// verdict `gen-7` would. In the neighbour nothing moves; the run asks, stops at the
/// sixty-sixth rank, and answers what the drained control answers.
#[test]
fn a_second_calls_lookup_from_a_moved_index_is_refused_and_one_at_the_pinned_generation_is_admitted()
 {
    let text = |predicate: &str| {
        format!(
            "SELECT ?candidate WHERE {{ ( ?candidate ) <{p}> ( \"everything\"@en ) . \
             ( ?candidate ) <{p}> ( \"lazy dog\"@en ) }}",
            p = producer_iri(predicate)
        )
    };
    let pinned = Some(Generations {
        read: "gen-7",
        lookups: "gen-7",
    });
    let run_moving = |move_index: bool| {
        let moves = Arc::new(AtomicBool::new(false));
        let (registry, counters) =
            shaped_registry(ExclusionBasis::Membership, pinned, DISJOINT, Some(&moves));
        let bundle = supplied(compiled(&registry), text).expect("a join of calls is admitted");
        run_then(
            &bundle,
            &registry,
            &counters,
            common::empty_dataset(),
            ReadSchedule::OnDemand,
            &|| moves.store(move_index, Ordering::SeqCst),
        )
    };

    let refused = match run_moving(true) {
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

    let stable = run_moving(false).expect("lookups at the pinned generation are admitted");
    let control = measured(ExclusionBasis::Unavailable, None, ReadSchedule::OnDemand);
    assert_eq!(
        stable.fused_lookups(),
        both(65),
        "the neighbour asked on both strata"
    );
    // Four hundred bound invocations for the join's own read, then two per verdict:
    // `"everything"` finds every candidate, so each verdict goes on to `"lazy dog"`,
    // which excludes it. Every lookup of the second call was asked.
    assert_eq!(
        stable.served,
        both(ROWS + 2 * 65),
        "both calls asked per verdict"
    );
    assert_eq!(
        stable.found,
        both(ROWS + 65),
        "the join's own rows, and each verdict's first call"
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
// Refusals: a column that can take a value no call emitted.
// ---------------------------------------------------------------------------

/// What executing a bundle and asking each stratum's stream directly came to: the
/// verdict for a candidate the stratum's producer never names (the other producer's
/// first), the verdict for one it names (its own first), and the candidates the
/// stream holds — or the stratum's failure.
type Asked = BTreeMap<Iri, Result<(ExclusionVerdict, ExclusionVerdict, Vec<String>), String>>;

/// A text per predicate, boxed so texts of different shapes sit in one table.
type Text<'a> = Box<dyn Fn(&str) -> String + 'a>;
/// The candidates a stream over a predicate's producer holds, boxed likewise.
type Holds<'a> = Box<dyn Fn(&str) -> Vec<String> + 'a>;

/// Execute `bundle` on demand and ask every stream about a foreign and an own
/// candidate, then read it to its end.
fn asked(bundle: &CompiledRetrieval, registry: &PropertyFunctionRegistry) -> Asked {
    let ExecutionResult {
        mut streams,
        statuses,
    } = block_on(execute_within(
        bundle,
        registry,
        common::empty_dataset(),
        ReadSchedule::OnDemand,
    ))
    .expect("the bundle runs");
    let [left, right] = strata();
    let first = |prefix: &str| Term::new(format!("<{}>", ex(&format!("{prefix}entity000000"))));
    strata()
        .into_iter()
        .map(|stratum| {
            let Some(stream) = streams.iter_mut().find(|stream| stream.stratum == stratum) else {
                return (
                    stratum.clone(),
                    Err(match statuses.get(&stratum) {
                        Some(ProducerStatus::ExecutionFailed { reason }) => reason.clone(),
                        other => format!("no stream and status {other:?}"),
                    }),
                );
            };
            let (own, foreign) = if stratum == left {
                (first("left/"), first("right/"))
            } else {
                assert_eq!(stratum, right);
                (first("right/"), first("left/"))
            };
            let excluded = block_on(stream.stream.exclusion(&foreign)).expect("the lookup answers");
            let possible = block_on(stream.stream.exclusion(&own)).expect("the lookup answers");
            let mut held = Vec::new();
            while let Some((_, term, _)) = block_on(stream.stream.next()).expect("it reads") {
                held.push(term.as_str().to_owned());
            }
            (stratum, Ok((excluded, possible, held)))
        })
        .collect()
}

/// **Each operator that lets the candidate take a value no call emitted is refused by
/// name, beside the neighbour differing only in that operator, which is admitted and
/// answers its lookups.**
///
/// The refused texts, and why each is unsound: the `?candidate` column of
///
/// * `{ VALUES } UNION { call }` holds the `VALUES` terms whatever the call emits;
/// * `{ VALUES } OPTIONAL { call }` holds the `VALUES` terms where the call matches
///   nothing;
/// * `{ VALUES } MINUS { call }` holds only `VALUES` terms — the call's rows only
///   subtract;
/// * `SELECT (COUNT(?hit) AS ?candidate) … GROUP BY ?hit` holds counts, which no call
///   emitted;
/// * `GRAPH ?candidate { call }` holds graph names;
///
/// so a call excluding a candidate proves nothing about the column, and a lookup of
/// it would answer `Excluded` for a term the stream names. The first five are refused
/// at construction, under any registry. The neighbours are the `VALUES` block *joined*
/// with the call, the `GROUP BY` projecting its key, and the `GRAPH` block whose call
/// binds the candidate — and, admitted too because each is sound, the call on the
/// required side of an `OPTIONAL` and on the left of a `MINUS`: every solution of
/// either is a solution of the call. Each neighbour is executed, and each stream is
/// asked directly: the other producer's first candidate is `Excluded`, its own is
/// `Possible` — a verdict only an attached lookup gives — and it holds exactly the
/// candidates its operator leaves.
///
/// The `VALUES` block names the stratum's own first two candidates and the other
/// producer's first, so the joined neighbour holds the own two and not the third.
#[test]
fn an_operator_that_lets_the_candidate_escape_its_calls_is_refused_beside_its_neighbour() {
    let (registry, counters) = fixture_registry(ExclusionBasis::Membership, None);
    let rendered = compiled(&registry);
    let own = |predicate: &str| {
        if predicate == "title" {
            "left/"
        } else {
            "right/"
        }
    };
    let other = |predicate: &str| {
        if predicate == "title" {
            "right/"
        } else {
            "left/"
        }
    };
    let values = move |predicate: &str| {
        format!(
            "VALUES ?candidate {{ <{}> <{}> <{}> }}",
            ex(&format!("{}entity000000", own(predicate))),
            ex(&format!("{}entity000001", own(predicate))),
            ex(&format!("{}entity000000", other(predicate)))
        )
    };
    let call = |predicate: &str, variable: &str| {
        format!(
            "( ?{variable} ) <{}> ( \"quick brown fox\"@en )",
            producer_iri(predicate)
        )
    };
    let select = |body: String| format!("SELECT ?candidate WHERE {{ {body} }}");
    let refused: [(&str, Text<'_>, &str); 5] = [
        (
            "UNION",
            Box::new(move |p: &str| {
                select(format!(
                    "{{ {} }} UNION {{ {} }}",
                    values(p),
                    call(p, "candidate")
                ))
            }),
            "a UNION",
        ),
        (
            "OPTIONAL",
            Box::new(move |p: &str| {
                select({
                    format!(
                        "{{ {} }} OPTIONAL {{ {} }}",
                        values(p),
                        call(p, "candidate")
                    )
                })
            }),
            "an OPTIONAL",
        ),
        (
            "MINUS",
            Box::new(move |p: &str| {
                select(format!(
                    "{{ {} }} MINUS {{ {} }}",
                    values(p),
                    call(p, "candidate")
                ))
            }),
            "a MINUS",
        ),
        (
            "GROUP BY",
            Box::new(move |p: &str| {
                select({
                    format!(
                        "{{ SELECT (COUNT(?hit) AS ?candidate) WHERE {{ {} }} GROUP BY ?hit }}",
                        call(p, "hit")
                    )
                })
            }),
            "an aggregate",
        ),
        (
            "GRAPH",
            Box::new(move |p: &str| select(format!("GRAPH ?candidate {{ {} }}", call(p, "hit")))),
            "a GRAPH, which binds it to the name of a graph",
        ),
    ];
    for (name, text, named) in &refused {
        let refusal = supplied(rendered.clone(), text)
            .expect_err("a column that can escape its calls cannot declare a basis");
        let UnitError::ExclusionNotRenderable { basis, reason } = &refusal else {
            panic!("{name}: expected ExclusionNotRenderable, got {refusal:?}");
        };
        assert_eq!(*basis, "membership", "{name}");
        assert!(
            reason.contains(named),
            "{name}: the refusal names why: {reason}"
        );
    }

    let own_first_two = |predicate: &str| {
        vec![
            format!("<{}>", ex(&format!("{}entity000000", own(predicate)))),
            format!("<{}>", ex(&format!("{}entity000001", own(predicate)))),
        ]
    };
    let every = |predicate: &str| {
        (0..ROWS)
            .map(|index| format!("<{}>", ex(&format!("{}entity{index:06}", own(predicate)))))
            .collect::<Vec<_>>()
    };
    let admitted: [(&str, Text<'_>, Holds<'_>); 6] = [
        (
            "VALUES joined with the call",
            Box::new(move |p: &str| {
                select(format!(
                    "{{ {} }} {{ {} }}",
                    values(p),
                    call(p, "candidate")
                ))
            }),
            Box::new(own_first_two),
        ),
        (
            "the call on the required side of an OPTIONAL",
            Box::new(move |p: &str| {
                select({
                    format!(
                        "{{ {} }} OPTIONAL {{ {} }}",
                        call(p, "candidate"),
                        values(p)
                    )
                })
            }),
            Box::new(every),
        ),
        (
            "the call on the left of a MINUS",
            Box::new(move |p: &str| {
                select({
                    format!(
                        "{{ {} }} MINUS {{ VALUES ?candidate {{ <{}> }} }}",
                        call(p, "candidate"),
                        ex(&format!("{}entity000000", own(p)))
                    )
                })
            }),
            Box::new(move |p: &str| every(p).split_off(1)),
        ),
        (
            "a GROUP BY projecting its key",
            Box::new(move |p: &str| {
                select({
                    format!(
                        "{{ SELECT (?hit AS ?candidate) WHERE {{ {} }} GROUP BY ?hit }}",
                        call(p, "hit")
                    )
                })
            }),
            Box::new(every),
        ),
        (
            "a GRAPH block whose call binds the candidate",
            Box::new(move |p: &str| select(format!("GRAPH ?g {{ {} }}", call(p, "candidate")))),
            Box::new(|_: &str| Vec::new()),
        ),
        (
            "a DISTINCT sub-SELECT",
            Box::new(move |p: &str| {
                select({
                    format!(
                        "{{ SELECT DISTINCT ?candidate WHERE {{ {} }} }}",
                        call(p, "candidate")
                    )
                })
            }),
            Box::new(every),
        ),
    ];
    for (name, text, holds) in &admitted {
        let bundle = supplied(rendered.clone(), text).expect("the neighbour is admitted");
        let before: Vec<u64> = counters
            .iter()
            .map(|reads| reads.lookups.load(Ordering::SeqCst))
            .collect();
        let answered = asked(&bundle, &registry);
        for (stratum, predicate) in strata().into_iter().zip(PREDICATES) {
            let Ok((foreign, own, held)) = &answered[&stratum] else {
                panic!("{name}: {stratum} failed: {:?}", answered[&stratum]);
            };
            assert_eq!(
                (*foreign, *own),
                (ExclusionVerdict::Excluded, ExclusionVerdict::Possible),
                "{name}: the lookup is attached and asks the call — {stratum}"
            );
            let mut held = held.clone();
            held.sort();
            let mut expected = holds(predicate);
            expected.sort();
            assert_eq!(
                held, expected,
                "{name}: the operator was honoured — {stratum}"
            );
        }
        let after: Vec<u64> = counters
            .iter()
            .map(|reads| reads.lookups.load(Ordering::SeqCst))
            .collect();
        assert!(
            after
                .iter()
                .zip(&before)
                .all(|(after, before)| after >= &(before + 2)),
            "{name}: both verdicts were the producer's own answers: {before:?} -> {after:?}"
        );
    }
}

/// **A `?candidate` bound only by a data triple fails its stratum at execution; the
/// neighbour whose call binds it is admitted and answers its lookups.**
///
/// Construction reads every bare predicate as a call, so both texts pass it; the
/// registry, which does not register the data predicate, reads it as a triple
/// pattern, and a triple pattern emits nothing a lookup could ask. The refused text's
/// call binds `?hit`, never the candidate, so the stratum is `ExecutionFailed` naming
/// that — before any row is read. The neighbour, where the call binds `?candidate`,
/// runs, and its stream answers `Excluded` for the other producer's candidate and
/// `Possible` for its own. The dataset is empty, so the neighbour's join holds no
/// row: the verdicts are the lookups', not a read's.
#[test]
fn a_candidate_bound_only_by_a_data_triple_fails_its_stratum_beside_its_neighbour() {
    let (registry, counters) = fixture_registry(ExclusionBasis::Membership, None);
    let rendered = compiled(&registry);
    let text = |variable: &'static str| {
        move |predicate: &str| {
            format!(
                "SELECT ?candidate WHERE {{ ?candidate <{}> ?o . ( ?{variable} ) <{}> ( \"quick \
                 brown fox\"@en ) }}",
                ex("data/p"),
                producer_iri(predicate)
            )
        }
    };
    let refused =
        supplied(rendered.clone(), text("hit")).expect("under the widest reading, two calls");
    for (stratum, answered) in asked(&refused, &registry) {
        let Err(reason) = answered else {
            panic!("{stratum}: the data triple cannot be asked: {answered:?}");
        };
        assert!(
            reason.contains("could not be derived from its query")
                && reason.contains("no property-function call binds its ?candidate column"),
            "{stratum}: naming why: {reason}"
        );
    }
    assert!(
        counters.iter().all(|reads| reads.rows().is_empty()),
        "nothing was read"
    );

    let admitted = supplied(rendered, text("candidate")).expect("admitted");
    for (stratum, answered) in asked(&admitted, &registry) {
        assert_eq!(
            answered,
            Ok((
                ExclusionVerdict::Excluded,
                ExclusionVerdict::Possible,
                Vec::new()
            )),
            "{stratum}: the call's lookup is attached and asked"
        );
    }
}

// ---------------------------------------------------------------------------
// The registry is the authority on what the call is.
// ---------------------------------------------------------------------------

/// **A text the construction-time reading admits but the registry does not make a
/// call of fails its own stratum, by name; the registered neighbour runs.**
///
/// `StratumUnit::new` has no registry, so it reads every bare predicate as a call.
/// A text calling an IRI nothing registered is one call under that reading and a
/// triple pattern under the registry's, so no lookup can be derived at execution: the
/// stratum is `ExecutionFailed`, naming the reason, and the other stratum — whose
/// text is the registered call — still runs and asks.
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
        reason.contains("could not be derived from its query")
            && reason.contains("no property-function call binds its ?candidate column"),
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
