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
//! `MINUS` over `VALUES`, an aggregate, a computed `GROUP BY` condition, a `GRAPH`
//! name, a data triple — and declares a basis is refused by name, a condition
//! rebinding the call's own `?candidate` does not parse, and the neighbour differing
//! in that one operator is admitted and answers its lookups. A `UNION` whose every
//! branch draws the candidate from a call is admitted and asks every branch; a call
//! whose needle the text reads out of the data — over the real text relation — keeps
//! that pattern in its lookup, as the text evaluates it: a needle a `LATERAL` picks
//! through its left operand's variables stays correlated, and one read in a `GRAPH`
//! or under a dataset clause is read there. And every admitted shape is run twice, with its
//! lookups and as a full read with none, to one answer.
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
    DuplicatePolicy, ExclusionVerdict, ExecutionError, ExecutionResult, Fixed, FusedRow,
    FusionError, FusionProfile, FusionTrailer, Iri, ProducerStatus, ProtocolError, RECIP_K,
    RankFidelity, RankedStreamAdapter, ReadSchedule, RequestTerm, RetrievalRequest, Statistics,
    StratumUnit, StreamContract, Term, TopK, UnitError, compile, contribution_under,
    execute_within, fuse, plan,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, EvalError, ExclusionBasis, IndexGeneration, PfArgs, PfArity,
    PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry, RankArithmetic, RankedDeclaration,
    RequestFacet, ServiceLevel, TermKind, TermPattern, TermPlacement, Volatility,
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
                arithmetic: RankArithmetic::FloatFree,
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
    asked_over(bundle, registry, common::empty_dataset())
}

/// [`asked`], over `dataset`.
fn asked_over(
    bundle: &CompiledRetrieval,
    registry: &PropertyFunctionRegistry,
    dataset: &RdfDataset,
) -> Asked {
    let ExecutionResult {
        mut streams,
        statuses,
    } = block_on(execute_within(
        bundle,
        registry,
        dataset,
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

// ---------------------------------------------------------------------------
// A column rebound over a call's: refused where it is written, and never read as
// the call's.
// ---------------------------------------------------------------------------

/// The call a caller writes for `predicate`, binding `variable` to its candidate.
fn call_binding(predicate: &str, variable: &str) -> String {
    format!(
        "( ?{variable} ) <{}> ( \"quick brown fox\"@en )",
        producer_iri(predicate)
    )
}

/// The first candidate the producer behind `predicate` names.
fn first_of(predicate: &str) -> String {
    if predicate == "title" {
        ex("left/entity000000")
    } else {
        ex("right/entity000000")
    }
}

/// **A `GROUP BY` condition rebinding the call's `?candidate` is a syntax error, as
/// the `BIND` over it is; the same computation under a fresh key and carried to the
/// column is refused by name; the plain key and a renaming condition get their
/// lookups and answer what the full read answers.**
///
/// The rebinding text swaps the producer's first candidate for an intruder no
/// producer names. Were it admitted and its `?candidate` read as the call's, the
/// lookups would answer for a column that holds the intruder, and the fused answer
/// would differ from the full read's — the defect pinned here. It is refused at
/// construction instead: `?candidate` is in scope where the condition binds it, and
/// the parser refuses that exactly as it refuses the `BIND` beside it.
///
/// The legal spelling of the same computation — the condition binding a fresh `?k`,
/// projected as `?candidate` — is a column no call is a source of, and declaring a
/// basis over it is refused naming the computed condition. The neighbours are the
/// plain `GROUP BY ?candidate` and the renaming `GROUP BY (?hit AS ?k)` projected as
/// `?candidate`: both admitted, both ask their lookups and stop at the sixty-sixth
/// rank, and both answer exactly what the same text answers read in full with no
/// lookup — the observing oracle, which would hold the intruder had the swap been
/// honoured.
#[test]
fn a_group_by_condition_rebinding_the_candidate_is_refused_and_its_neighbours_answer_as_the_full_read()
 {
    let (registry, _) = fixture_registry(ExclusionBasis::Membership, None);
    let rendered = compiled(&registry);
    let intruder = ex("intruder");
    let shadowed = |predicate: &str| {
        format!(
            "SELECT ?candidate WHERE {{ {} }} GROUP BY (IF(?candidate = <{}>, <{intruder}>, \
             ?candidate) AS ?candidate)",
            call_binding(predicate, "candidate"),
            first_of(predicate)
        )
    };
    let bound = |predicate: &str| {
        format!(
            "SELECT ?candidate WHERE {{ {} BIND(<{intruder}> AS ?candidate) }}",
            call_binding(predicate, "candidate")
        )
    };
    for (name, text) in [
        ("GROUP BY", &shadowed as &dyn Fn(&str) -> String),
        ("BIND", &bound),
    ] {
        let refusal = supplied(rendered.clone(), text).expect_err("a rebinding does not parse");
        let UnitError::NotAQuery { reason } = &refusal else {
            panic!("{name}: expected NotAQuery, got {refusal:?}");
        };
        assert!(
            reason.contains("?candidate is already in scope"),
            "{name}: the parser names the rebinding: {reason}"
        );
    }

    let computed = |predicate: &str| {
        format!(
            "SELECT (?k AS ?candidate) WHERE {{ {} }} GROUP BY (IF(?hit = <{}>, <{intruder}>, \
             ?hit) AS ?k)",
            call_binding(predicate, "hit"),
            first_of(predicate)
        )
    };
    let refusal = supplied(rendered, computed).expect_err("a computed key has no source");
    let UnitError::ExclusionNotRenderable { basis, reason } = &refusal else {
        panic!("expected ExclusionNotRenderable, got {refusal:?}");
    };
    assert_eq!(*basis, "membership");
    assert!(
        reason.contains("a computed BIND, SELECT expression or GROUP BY condition"),
        "the refusal names the computed column: {reason}"
    );
    // The same text with no basis declared runs, and holds the intruder: the swap is
    // real, so a lookup answering for the call would have been answering for a
    // column the call does not fill.
    let swapped = measured(
        ExclusionBasis::Unavailable,
        Some(&computed),
        ReadSchedule::Materialised,
    );
    assert!(
        answer(&swapped)
            .iter()
            .any(|(entity, _)| *entity == format!("<{intruder}>")),
        "the computed column holds the intruder: {:?}",
        answer(&swapped)
    );

    let keyed = |predicate: &str| {
        format!(
            "SELECT ?candidate WHERE {{ {} }} GROUP BY ?candidate ORDER BY ?candidate",
            call_binding(predicate, "candidate")
        )
    };
    let renamed = |predicate: &str| {
        format!(
            "SELECT (?k AS ?candidate) WHERE {{ {} }} GROUP BY (?hit AS ?k) ORDER BY ?k",
            call_binding(predicate, "hit")
        )
    };
    for (name, text) in [
        ("GROUP BY ?candidate", &keyed as &dyn Fn(&str) -> String),
        ("GROUP BY (?hit AS ?k)", &renamed),
    ] {
        let looked_up = measured(
            ExclusionBasis::Membership,
            Some(text),
            ReadSchedule::OnDemand,
        );
        let full = measured(
            ExclusionBasis::Unavailable,
            Some(text),
            ReadSchedule::Materialised,
        );
        assert_eq!(
            looked_up.fused_lookups(),
            both(65),
            "{name}: the lookups were asked"
        );
        assert_eq!(
            looked_up.served,
            both(65),
            "{name}: and served by the producer"
        );
        assert_eq!(
            looked_up.ranks_pulled(),
            both(66),
            "{name}: they settled finality"
        );
        assert_eq!(
            full.ranks_pulled(),
            both(ROWS),
            "{name}: the full read drains"
        );
        assert_eq!(
            answer(&looked_up),
            answer(&full),
            "{name}: the full read's answer"
        );
        assert_eq!(
            looked_up.trailer.exactness, full.trailer.exactness,
            "{name}: as exactly"
        );
    }
}

// ---------------------------------------------------------------------------
// A UNION of calls: one alternative per branch.
// ---------------------------------------------------------------------------

/// Each branch of the `UNION` the call under a `FILTER`, the two halves of the
/// producer's candidates between them.
fn union_of_calls(predicate: &str) -> String {
    let cut = if predicate == "title" {
        ex("left/entity000200")
    } else {
        ex("right/entity000200")
    };
    format!(
        "SELECT ?candidate WHERE {{ {{ {call} FILTER(STR(?candidate) < \"{cut}\") }} UNION \
         {{ {call} FILTER(STR(?candidate) >= \"{cut}\") }} }}",
        call = call_binding(predicate, "candidate")
    )
}

/// **A `UNION` whose branches each draw the candidate from a call asks both calls,
/// stops where the one-call text stops, and answers what the full read answers; a
/// branch that binds it from `VALUES` is refused; and an alternative that holds the
/// candidate keeps it `Possible` however the other answers.**
///
/// Every value either branch gives `?candidate` is a value that branch's call
/// emitted, so each branch is an alternative, and a candidate is out of the text's
/// reach when each branch's call excludes it. The run asks both per verdict — each
/// verdict two producer invocations, both exclusions of the other producer's
/// candidate — pulls sixty-six ranks where the same text with no basis drains four
/// hundred, and answers exactly what that full read answers.
///
/// Refused beside it: the same `UNION` with a `VALUES` branch naming the intruder,
/// whose values are no call's. And the rule is *every* alternative, not *any*: with
/// one branch asking the `"everything"` needle, whose lookup holds every candidate,
/// the other producer's candidate is `Possible` — though the first branch's call
/// excludes it — while the two-call `UNION` answers `Excluded` for the same
/// candidate.
#[test]
fn a_union_of_calls_asks_every_branch_and_answers_as_the_full_read() {
    let text: &dyn Fn(&str) -> String = &union_of_calls;
    let looked_up = measured(
        ExclusionBasis::Membership,
        Some(text),
        ReadSchedule::OnDemand,
    );
    let drained = measured(
        ExclusionBasis::Unavailable,
        Some(text),
        ReadSchedule::OnDemand,
    );
    let full = measured(
        ExclusionBasis::Unavailable,
        Some(text),
        ReadSchedule::Materialised,
    );
    let plain = measured(
        ExclusionBasis::Membership,
        Some(&one_call),
        ReadSchedule::OnDemand,
    );

    assert_eq!(looked_up.fused_lookups(), both(65), "both strata asked");
    assert_eq!(
        looked_up.served,
        both(2 * 65),
        "each verdict asked both branches' calls"
    );
    assert_eq!(looked_up.found, both(0), "and each call excluded");
    assert_eq!(
        looked_up.ranks_pulled(),
        both(66),
        "the lookups settled finality"
    );
    assert_eq!(
        drained.ranks_pulled(),
        both(ROWS),
        "with no basis it drains"
    );
    assert_eq!(drained.fused_lookups(), both(0));
    assert_eq!(answer(&looked_up), answer(&full), "the full read's answer");
    assert_eq!(
        answer(&looked_up),
        answer(&plain),
        "the one-call text's answer"
    );
    assert_eq!(looked_up.trailer.exactness, full.trailer.exactness);
    assert_eq!(looked_up.trailer.statuses, plain.trailer.statuses);

    // The refused neighbour: a branch binding the candidate from VALUES.
    let (registry, _) = fixture_registry(ExclusionBasis::Membership, None);
    let rendered = compiled(&registry);
    let valued = |predicate: &str| {
        format!(
            "SELECT ?candidate WHERE {{ {{ {} }} UNION {{ VALUES ?candidate {{ <{}> }} }} }}",
            call_binding(predicate, "candidate"),
            ex("intruder")
        )
    };
    let refusal = supplied(rendered.clone(), valued).expect_err("a VALUES branch is no call");
    let UnitError::ExclusionNotRenderable { reason, .. } = &refusal else {
        panic!("expected ExclusionNotRenderable, got {refusal:?}");
    };
    assert!(reason.contains("a UNION"), "naming the UNION: {reason}");

    // Every alternative, not any: the other producer's first candidate. The
    // `"everything"` branch names nothing — its FILTER drops every row, which keeps
    // the text inside the producer's declared bound — yet its call, asked with the
    // candidate bound, holds it, and that alone keeps the verdict `Possible`.
    let with_everything = |predicate: &str| {
        format!(
            "SELECT ?candidate WHERE {{ {{ {} }} UNION {{ ( ?candidate ) <{}> ( \
             \"everything\"@en ) FILTER(false) }} }}",
            call_binding(predicate, "candidate"),
            producer_iri(predicate)
        )
    };
    for (text, foreign) in [
        (
            &with_everything as &dyn Fn(&str) -> String,
            ExclusionVerdict::Possible,
        ),
        (&union_of_calls, ExclusionVerdict::Excluded),
    ] {
        let bundle = supplied(rendered.clone(), text).expect("admitted");
        for (stratum, answered) in asked(&bundle, &registry) {
            let (verdict, own, _) =
                answered.unwrap_or_else(|reason| panic!("{stratum} failed: {reason}"));
            assert_eq!(
                verdict, foreign,
                "{stratum}: the other producer's candidate"
            );
            assert_eq!(own, ExclusionVerdict::Possible, "{stratum}: its own");
        }
    }
}

// ---------------------------------------------------------------------------
// Every admitted shape: with lookups and without, one answer.
// ---------------------------------------------------------------------------

/// **Every supplied text this file admits answers the same with its lookups as
/// without: the answer and its exactness of the on-demand run declaring the basis
/// are those of the full, materialised read declaring none.**
///
/// The run with no lookup reads every row and needs no verdict to answer, so it is
/// the oracle for what the text says; the run with lookups stops wherever they let
/// it. A lookup that answered `Excluded` for a candidate the text names would move a
/// row or its score, or leave the fusion's exactness different, and no shape here is
/// allowed to. Each shape that asks lookups is pinned as asking them — and pulling
/// fewer ranks than the oracle — so the comparison is between a shortened read and a
/// full one, not two full reads.
/// The differential list's needles bound inside a `LATERAL` over `?y`, and the
/// uncorrelated neighbour: `(name, text, whether the shortened read asks lookups)`.
///
/// Four pick the needle through `?y` — a `FILTER` over it, one disjoining it with a
/// constant, a `FILTER EXISTS` over it — so read on its own, cut from the left
/// operand, each picks no needle at all: `?y` unbound, `?q = ?y` an error.
fn lateral_needle_shapes() -> Vec<(&'static str, Text<'static>, bool)> {
    vec![
        (
            "a LATERAL picking a VALUES needle by a FILTER disjoining the left's variable",
            Box::new(move |p: &str| {
                lateral_needle(
                    p,
                    "VALUES ?q { \"lazy dog\"@en \"quick brown fox\"@en } FILTER(?q = ?y || ?q = \
                 \"nothing\"@en)",
                )
            }),
            true,
        ),
        (
            "a LATERAL whose needle a FILTER disjoining the left's variable keeps",
            Box::new(move |p: &str| {
                lateral_needle(
                    p,
                    "BIND(\"quick brown fox\"@en AS ?q) FILTER(?q = ?y || ?q = \"lazy dog\"@en)",
                )
            }),
            true,
        ),
        (
            "a LATERAL picking its needle by a FILTER over the left's variable",
            Box::new(move |p: &str| {
                lateral_needle(
                    p,
                    "VALUES ?q { \"quick brown fox\"@en \"lazy dog\"@en } FILTER(?q = ?y)",
                )
            }),
            true,
        ),
        (
            "a LATERAL picking its needle by a FILTER EXISTS over the left's variable",
            Box::new(move |p: &str| {
                lateral_needle(
                    p,
                    "VALUES ?q { \"quick brown fox\"@en \"lazy dog\"@en } FILTER EXISTS { \
                 FILTER(?q = ?y) }",
                )
            }),
            true,
        ),
        (
            "an uncorrelated LATERAL binding the needle",
            Box::new(move |p: &str| lateral_needle(p, "VALUES ?q { \"quick brown fox\"@en }")),
            true,
        ),
        (
            "a sub-SELECT picking its needle by a FILTER over the variable the LATERAL injects",
            Box::new(move |p: &str| {
                lateral_subselect_needle(
                    p,
                    "{ VALUES ?q { \"quick brown fox\"@en \"lazy dog\"@en } FILTER(?q = ?y) }",
                )
            }),
            true,
        ),
        (
            "a sub-SELECT binding its needle from the variable the LATERAL injects",
            Box::new(move |p: &str| lateral_subselect_needle(p, "BIND(?y AS ?q)")),
            true,
        ),
        (
            "a sub-SELECT whose needle the LATERAL injects",
            Box::new(move |p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ VALUES ?q {{ \"quick brown fox\"@en }} LATERAL \
                     {{ SELECT ?candidate ?q WHERE {{ ( ?candidate ) <{}> ( ?q ) }} }} }}",
                    producer_iri(p)
                )
            }),
            true,
        ),
    ]
}

/// `VALUES ?y { "quick brown fox"@en } LATERAL { SELECT ?candidate ?y WHERE { body
/// call } }`, the call's needle `?q`: the sub-`SELECT` projects `?y`, so the `LATERAL`
/// injects it there.
fn lateral_subselect_needle(predicate: &str, body: &str) -> String {
    format!(
        "SELECT ?candidate WHERE {{ VALUES ?y {{ \"quick brown fox\"@en }} LATERAL {{ SELECT \
         ?candidate ?y WHERE {{ {body} ( ?candidate ) <{}> ( ?q ) }} }} }}",
        producer_iri(predicate)
    )
}

/// `VALUES ?y { "quick brown fox"@en } LATERAL { lateral }` and then the call, its
/// needle `?q`: a needle the `LATERAL` binds, over no data at all.
fn lateral_needle(predicate: &str, lateral: &str) -> String {
    format!(
        "SELECT ?candidate WHERE {{ VALUES ?y {{ \"quick brown fox\"@en }} LATERAL {{ {lateral} }} \
         ( ?candidate ) <{}> ( ?q ) }}",
        producer_iri(predicate)
    )
}

#[test]
fn every_admitted_supplied_shape_answers_with_its_lookups_as_the_full_read_without() {
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
    let call = |predicate: &str, variable: &str| call_binding(predicate, variable);
    let select = |body: String| format!("SELECT ?candidate WHERE {{ {body} }}");
    // (name, text, whether the shortened read asks lookups)
    let mut shapes: Vec<(&str, Text<'_>, bool)> = vec![
        ("one call", Box::new(one_call), true),
        (
            "a renaming projection",
            Box::new(move |p: &str| {
                select(format!(
                    "{{ SELECT (?hit AS ?candidate) WHERE {{ {} }} }}",
                    call(p, "hit")
                ))
            }),
            true,
        ),
        (
            "a renaming BIND",
            Box::new(move |p: &str| select(format!("{} BIND(?hit AS ?candidate)", call(p, "hit")))),
            true,
        ),
        ("a FILTER", Box::new(filtered), true),
        (
            "a FILTER removing ten",
            Box::new(move |p: &str| {
                select(format!(
                    "{} FILTER(!STRSTARTS(STR(?candidate), \"{}\"))",
                    call(p, "candidate"),
                    ex("left/entity00000")
                ))
            }),
            true,
        ),
        ("a join of calls", Box::new(joined), true),
        ("a UNION of calls", Box::new(union_of_calls), true),
        (
            "VALUES joined with the call",
            Box::new(move |p: &str| {
                select(format!(
                    "{{ VALUES ?candidate {{ <{}> <{}> <{}> }} }} {{ {} }}",
                    ex(&format!("{}entity000000", own(p))),
                    ex(&format!("{}entity000001", own(p))),
                    ex(&format!("{}entity000000", other(p))),
                    call(p, "candidate")
                ))
            }),
            false,
        ),
        (
            "the call on the required side of an OPTIONAL",
            Box::new(move |p: &str| {
                select(format!(
                    "{{ {} }} OPTIONAL {{ VALUES ?candidate {{ <{}> }} }}",
                    call(p, "candidate"),
                    ex("intruder")
                ))
            }),
            true,
        ),
        (
            "the call on the left of a MINUS",
            Box::new(move |p: &str| {
                select(format!(
                    "{{ {} }} MINUS {{ VALUES ?candidate {{ <{}> }} }}",
                    call(p, "candidate"),
                    ex(&format!("{}entity000000", own(p)))
                ))
            }),
            true,
        ),
        (
            "a GROUP BY sub-SELECT projecting its key",
            Box::new(move |p: &str| {
                select(format!(
                    "{{ SELECT (?hit AS ?candidate) WHERE {{ {} }} GROUP BY ?hit }}",
                    call(p, "hit")
                ))
            }),
            true,
        ),
        (
            "GROUP BY ?candidate",
            Box::new(move |p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ {} }} GROUP BY ?candidate",
                    call(p, "candidate")
                )
            }),
            true,
        ),
        (
            "a renaming GROUP BY condition",
            Box::new(move |p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ {} }} GROUP BY (?hit AS ?candidate)",
                    call(p, "hit")
                )
            }),
            true,
        ),
        (
            "a DISTINCT sub-SELECT",
            Box::new(move |p: &str| {
                select(format!(
                    "{{ SELECT DISTINCT ?candidate WHERE {{ {} }} }}",
                    call(p, "candidate")
                ))
            }),
            true,
        ),
        (
            "an ORDER BY and LIMIT sub-SELECT",
            Box::new(move |p: &str| {
                select(format!(
                    "{{ SELECT ?candidate WHERE {{ {} }} ORDER BY ?candidate LIMIT 300 }}",
                    call(p, "candidate")
                ))
            }),
            true,
        ),
    ];
    shapes.extend(lateral_needle_shapes());
    for (name, text, asks) in &shapes {
        let looked_up = measured(
            ExclusionBasis::Membership,
            Some(text),
            ReadSchedule::OnDemand,
        );
        let full = measured(
            ExclusionBasis::Unavailable,
            Some(text),
            ReadSchedule::Materialised,
        );
        assert_eq!(
            answer(&looked_up),
            answer(&full),
            "{name}: the answer with lookups is the full read's"
        );
        assert_eq!(
            looked_up.trailer.exactness, full.trailer.exactness,
            "{name}: and as exact"
        );
        assert_eq!(
            full.fused_lookups(),
            both(0),
            "{name}: the oracle asks nothing"
        );
        let asked = looked_up.fused_lookups();
        if *asks {
            assert!(
                asked.values().all(|&count| count > 0),
                "{name}: the lookups were asked: {asked:?}"
            );
            assert!(
                looked_up
                    .ranks_pulled()
                    .iter()
                    .all(|(stratum, ranks)| *ranks < full.ranks_pulled()[stratum]),
                "{name}: and shortened the read: {:?} against {:?}",
                looked_up.ranks_pulled(),
                full.ranks_pulled()
            );
        } else {
            assert_eq!(
                looked_up.ranks_pulled(),
                full.ranks_pulled(),
                "{name}: a stream that runs out needs no verdict"
            );
        }
    }
    // The lateral needles again, over producers naming the same candidates: there a
    // lookup excluding a candidate the other stream names is a verdict the fusion
    // reads — the certified answer and its exactness move with it — so a driving
    // pattern binding fewer needles than the text binds cannot pass unseen.
    for (name, text, _) in &lateral_needle_shapes() {
        let looked_up = measure_shaped(
            ExclusionBasis::Membership,
            None,
            Some(text),
            ReadSchedule::OnDemand,
            INTERSECTING,
        )
        .unwrap_or_else(|error| panic!("{name}, shared candidates: fuses with lookups: {error:?}"));
        let full = measure_shaped(
            ExclusionBasis::Unavailable,
            None,
            Some(text),
            ReadSchedule::Materialised,
            INTERSECTING,
        )
        .unwrap_or_else(|error| panic!("{name}, shared candidates: fuses without: {error:?}"));
        assert_eq!(
            answer(&looked_up),
            answer(&full),
            "{name}, shared candidates: the full read's answer"
        );
        assert_eq!(
            looked_up.trailer.exactness, full.trailer.exactness,
            "{name}, shared candidates: as exactly"
        );
        assert!(
            looked_up.fused_lookups().values().any(|&count| count > 0),
            "{name}, shared candidates: a lookup was asked: {:?}",
            looked_up.fused_lookups()
        );
    }
}

// ---------------------------------------------------------------------------
// A call its text drives: the real text relation, its needle read out of the data.
// ---------------------------------------------------------------------------

/// The real text relation's side of this file: two single-partition indexes, one per
/// stratum, each holding `TEXT_MATCHING` documents the needle reaches under its own
/// subject prefix — the right one also holding a document per left subject that the
/// needle does not reach — and one configuration triple naming the needle.
mod text_relation {
    use super::*;
    use pretty_assertions::assert_eq;
    use purrdf_core::{RdfDatasetBuilder, RdfLiteral};
    use purrdf_text::{
        GraphSelector, SearchObservations, TextIndex, TextIndexConfig, TextSearchRelation,
    };

    /// The needle both producers search.
    pub(super) const NEEDLE: &str = "alpha beta";
    /// Documents the needle reaches, per side.
    const TEXT_MATCHING: usize = 100;

    /// `(stratum, predicate the index is built over, producer IRI, subject prefix)`.
    pub(super) fn sides() -> [(Iri, String, String, &'static str); 2] {
        let [left, right] = strata();
        [
            (left, ex("text/left"), ex("pf/text-left"), "a"),
            (right, ex("text/right"), ex("pf/text-right"), "b"),
        ]
    }

    /// Each side's `(subject, text)` rows.
    fn rows(side: usize) -> Vec<(String, String)> {
        let prefix = sides()[side].3;
        let mut out: Vec<(String, String)> = (0..TEXT_MATCHING)
            .map(|at| {
                (
                    ex(&format!("{prefix}{at}")),
                    format!("alpha beta gamma {}", "alpha ".repeat(at % 4 + 1).trim()),
                )
            })
            .collect();
        if side == 1 {
            out.extend(
                (0..TEXT_MATCHING)
                    .map(|at| (ex(&format!("a{at}")), "zulu yankee xray whiskey".to_owned())),
            );
        }
        out
    }

    /// Both sides' documents, and `<config> ex:needle "alpha beta"`.
    pub(super) fn dataset() -> Arc<RdfDataset> {
        dataset_with(&[])
    }

    /// [`dataset`], and `(subject, predicate, literal, graph)` quads beside it.
    pub(super) fn dataset_with(extra: &[(&str, &str, &str, Option<&str>)]) -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let config = builder.intern_iri(&ex("config"));
        let needle = builder.intern_iri(&ex("needle"));
        let value = builder.intern_literal(RdfLiteral::simple(NEEDLE));
        builder.push_quad(config, needle, value, None);
        for (subject, predicate, object, graph) in extra {
            let subject = builder.intern_iri(&ex(subject));
            let predicate = builder.intern_iri(&ex(predicate));
            let object = if object.starts_with('<') {
                builder.intern_iri(&ex(object.trim_start_matches('<').trim_end_matches('>')))
            } else {
                builder.intern_literal(RdfLiteral::simple(*object))
            };
            let graph = graph.map(|graph| builder.intern_iri(&ex(graph)));
            builder.push_quad(subject, predicate, object, graph);
        }
        for (side, (_, predicate, _, _)) in sides().iter().enumerate() {
            let predicate = builder.intern_iri(predicate);
            for (subject, text) in rows(side) {
                let subject = builder.intern_iri(&subject);
                let object = builder.intern_literal(RdfLiteral::simple(&text));
                builder.push_quad(subject, predicate, object, None);
            }
        }
        builder.freeze().expect("the fixture dataset is valid")
    }

    struct TextStatistics;

    impl Statistics for TextStatistics {
        fn source(&self) -> &'static str {
            "example-statistics"
        }

        fn revision(&self) -> &'static str {
            "r1"
        }

        fn cardinality(&self, predicate: &Iri) -> Option<u64> {
            sides()
                .iter()
                .enumerate()
                .find(|(_, (stratum, text, _, _))| {
                    stratum == predicate || text == predicate.as_str()
                })
                .map(|(side, _)| rows(side).len() as u64)
        }

        fn selectivity_ppm(&self, _subject: &Iri, _term: &RequestTerm) -> Option<u64> {
            None
        }
    }

    /// Both relations registered over `dataset`, each declaring its own ranked
    /// order with `basis` written in, beside each relation's observations.
    pub(super) fn registry(
        dataset: &RdfDataset,
        basis: ExclusionBasis,
    ) -> (PropertyFunctionRegistry, [Arc<SearchObservations>; 2]) {
        let shared = DomainTag::parse(&ex("domain/shared")).expect("the fixture tag is an IRI");
        let mut registry = PropertyFunctionRegistry::new();
        let observed = sides().map(|(stratum, predicate, producer, _)| {
            let config =
                TextIndexConfig::new(vec![TermValue::iri(predicate.clone())], GraphSelector::Any)
                    .expect("the fixture configuration is well formed");
            let index =
                Arc::new(TextIndex::from_dataset(dataset, &config).expect("the fixture indexes"));
            let relation = TextSearchRelation::new(index);
            let mut declaration = relation
                .ranked_declaration(
                    purrdf_core::parse_iri(stratum.as_str()).expect("fixture IRI"),
                    Some(predicate),
                    RankFidelity::EXACT,
                    CandidateDomains::within([shared.clone()]),
                )
                .expect("a single-partition index declares a ranked order");
            assert_eq!(
                declaration.exclusion,
                ExclusionBasis::Membership,
                "the relation's own basis; the fixture varies it rather than inventing it"
            );
            declaration.exclusion = basis;
            let observations = relation.observations();
            registry.register_ranked(producer, Arc::new(relation), declaration);
            observations
        });
        (registry, observed)
    }

    /// What `search` would compile for the needle over both sides.
    pub(super) fn compiled(registry: &PropertyFunctionRegistry) -> CompiledRetrieval {
        let statistics = TextStatistics;
        let profile = profile();
        let env = AdmissionEnvironment {
            registry,
            statistics: &statistics,
            fusion_profile: Some(&profile),
        };
        let terms = sides()
            .into_iter()
            .map(|(_, predicate, _, _)| RequestTerm::Lexical {
                text: NEEDLE.to_owned(),
                language: None,
                predicate: Some(iri(&predicate)),
            })
            .collect();
        let planned = plan(
            &RetrievalRequest::bounded(terms, TOP_K),
            registry,
            &statistics,
        )
        .expect("the fixture plans");
        compile(&planned, &env).expect("a fresh plan is admitted")
    }

    /// What one run over the text relations answered and cost.
    #[derive(Debug)]
    pub(super) struct TextRun {
        pub(super) answer: Vec<(String, Fixed)>,
        pub(super) exactness: purrdf_retrieval::ScoreExactness,
        pub(super) statuses: BTreeMap<Iri, ProducerStatus>,
        pub(super) ranks_pulled: BTreeMap<Iri, u64>,
        pub(super) fused_lookups: BTreeMap<Iri, u64>,
        /// Membership lookups each relation performed, by stratum.
        pub(super) membership_lookups: BTreeMap<Iri, u64>,
    }

    /// Run `text(producer)` for both strata under `basis` and `schedule`, or the
    /// rendered bundle for `None`.
    pub(super) fn run(
        basis: ExclusionBasis,
        text: Option<&dyn Fn(&str) -> String>,
        schedule: ReadSchedule,
    ) -> Result<TextRun, FusionError> {
        run_over(&dataset(), basis, text, schedule)
    }

    /// Execute `text` for both strata over `dataset`, each declaring membership, on
    /// demand, and ask the right stratum's stream about each of `probes` before
    /// reading it to its end: the verdicts, the candidates it named, and the
    /// membership tests the right relation performed.
    pub(super) fn asked_right(
        dataset: &Arc<RdfDataset>,
        text: &dyn Fn(&str) -> String,
        probes: &[String],
    ) -> (Vec<Result<ExclusionVerdict, String>>, Vec<String>, u64) {
        let (registry, observed) = registry(dataset, ExclusionBasis::Membership);
        let mut bundle = compiled(&registry);
        for (unit, (_, _, producer, _)) in bundle.units.iter_mut().zip(sides()) {
            *unit = StratumUnit::new(
                unit.stratum.clone(),
                text(&producer),
                unit.contract.clone(),
                unit.depth(),
                unit.declared_rows(),
            )
            .expect("the text is admitted at construction");
        }
        let ExecutionResult { mut streams, .. } = block_on(execute_within(
            &bundle,
            &registry,
            &**dataset,
            ReadSchedule::OnDemand,
        ))
        .expect("the bundle runs");
        let [_, right] = strata();
        let stream = streams
            .iter_mut()
            .find(|stream| stream.stratum == right)
            .expect("the right stratum runs");
        let verdicts = probes
            .iter()
            .map(|probe| {
                block_on(stream.stream.exclusion(&Term::new(probe.clone())))
                    .map_err(|error| format!("{error:?}"))
            })
            .collect();
        let mut held = Vec::new();
        while let Some((_, term, _)) = block_on(stream.stream.next()).expect("it reads") {
            held.push(term.as_str().to_owned());
        }
        (verdicts, held, observed[1].membership_lookups())
    }

    /// [`run`], over `dataset`.
    pub(super) fn run_over(
        dataset: &Arc<RdfDataset>,
        basis: ExclusionBasis,
        text: Option<&dyn Fn(&str) -> String>,
        schedule: ReadSchedule,
    ) -> Result<TextRun, FusionError> {
        let (registry, observed) = registry(dataset, basis);
        let mut bundle = compiled(&registry);
        if let Some(text) = text {
            for (unit, (_, _, producer, _)) in bundle.units.iter_mut().zip(sides()) {
                *unit = StratumUnit::new(
                    unit.stratum.clone(),
                    text(&producer),
                    unit.contract.clone(),
                    unit.depth(),
                    unit.declared_rows(),
                )
                .expect("the text is admitted at construction");
            }
        }
        let profile = profile();
        let ExecutionResult { streams, statuses } =
            block_on(execute_within(&bundle, &registry, &**dataset, schedule))
                .expect("the bundle runs");
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
        let trailer = fused.trailer.completed_with(statuses);
        let per = |read: &dyn Fn(&purrdf_retrieval::StratumResolution) -> u64| {
            trailer
                .resolution
                .iter()
                .map(|(stratum, resolution)| (stratum.clone(), read(resolution)))
                .collect::<BTreeMap<_, _>>()
        };
        Ok(TextRun {
            answer: fused
                .rows
                .iter()
                .map(|row| (row.entity.as_str().to_owned(), row.score))
                .collect(),
            exactness: trailer.exactness.clone(),
            statuses: trailer.statuses.clone(),
            ranks_pulled: per(&|resolution| resolution.ranks_pulled),
            fused_lookups: per(&|resolution| resolution.exclusion_lookups),
            membership_lookups: sides()
                .into_iter()
                .zip(&observed)
                .map(|((stratum, _, _, _), observed)| (stratum, observed.membership_lookups()))
                .collect(),
        })
    }
}

/// **A needle the text reads out of the data drives its call, and the lookup keeps
/// the pattern that binds it: asked, answered by the relation's membership test, the
/// read stopped where the rendered text's stops, and the answer the full read's.**
///
/// `<config> ex:needle ?q . ?candidate <pf> ( ?q ?score ?rank ?lang ?matched )`
/// against the real text relation, which cannot be invoked without its needle. A
/// lookup that blanked `?q` asks a mode the relation never declared, and the stratum
/// failed to prepare it; the lookup instead keeps the configuration triple, in a
/// `DISTINCT` sub-`SELECT` exporting `?q`, and invokes the call once per needle it
/// binds — here one — with the candidate bound, as a membership test.
///
/// Pinned against three runs: the same text declaring no basis (no lookup, and it
/// drains every row the needle reaches); the rendered bundle, whose lookups are the
/// reference price; and the constant-needle neighbour, which derives the undriven
/// lookup it always did and pays exactly the rendered price. The data-needle run asks
/// lookups on both strata, each answered by a membership test and not a ranking,
/// pulls the rendered run's ranks, and answers what the drained run answers, as
/// exactly.
#[test]
fn a_needle_read_out_of_the_data_drives_its_lookup_and_answers_as_the_full_read() {
    use text_relation::{NEEDLE, run};

    let data_needle = |producer: &str| {
        format!(
            "SELECT ?candidate WHERE {{ <{}> <{}> ?q . ?candidate <{producer}> ( ?q ?score ?rank \
             ?lang ?matched ) }}",
            ex("config"),
            ex("needle")
        )
    };
    let constant_needle = |producer: &str| {
        format!(
            "SELECT ?candidate WHERE {{ ?candidate <{producer}> ( \"{NEEDLE}\" ?score ?rank \
             ?lang ?matched ) }}"
        )
    };
    let looked_up = run(
        ExclusionBasis::Membership,
        Some(&data_needle),
        ReadSchedule::OnDemand,
    )
    .expect("the data needle fuses with its lookups");
    let drained = run(
        ExclusionBasis::Unavailable,
        Some(&data_needle),
        ReadSchedule::Materialised,
    )
    .expect("the data needle fuses without lookups");
    let rendered = run(ExclusionBasis::Membership, None, ReadSchedule::OnDemand)
        .expect("the rendered bundle fuses");
    let constant = run(
        ExclusionBasis::Membership,
        Some(&constant_needle),
        ReadSchedule::OnDemand,
    )
    .expect("the constant needle fuses");

    // Nothing failed: both strata read, and the fusion stopped them.
    assert!(
        looked_up
            .statuses
            .values()
            .all(|status| !matches!(status, ProducerStatus::ExecutionFailed { .. })),
        "{:?}",
        looked_up.statuses
    );
    assert_eq!(
        looked_up.statuses, rendered.statuses,
        "stopped where the rendered run stops"
    );

    // The lookups: asked on both strata, each a membership test of the relation.
    assert!(
        looked_up.fused_lookups.values().all(|&count| count > 0),
        "both strata asked: {:?}",
        looked_up.fused_lookups
    );
    assert_eq!(
        looked_up.fused_lookups, rendered.fused_lookups,
        "as often as the rendered bundle asks"
    );
    assert_eq!(
        looked_up.membership_lookups, rendered.membership_lookups,
        "answered by the relation's membership tests, exactly as the rendered lookups are"
    );
    assert!(
        looked_up
            .membership_lookups
            .values()
            .any(|&tests| tests > 0),
        "a membership test really ran: {:?}",
        looked_up.membership_lookups
    );

    // The price and the answer.
    assert_eq!(
        looked_up.ranks_pulled, rendered.ranks_pulled,
        "the rendered price"
    );
    assert_eq!(
        looked_up.ranks_pulled,
        both(66),
        "the rank the fusion certifies at"
    );
    assert!(
        drained
            .ranks_pulled
            .iter()
            .all(|(stratum, ranks)| *ranks > looked_up.ranks_pulled[stratum]),
        "without lookups the read drains further: {:?}",
        drained.ranks_pulled
    );
    assert_eq!(
        drained.ranks_pulled,
        both(100),
        "every document the needle reaches"
    );
    assert_eq!(drained.fused_lookups, both(0), "the oracle asks nothing");
    assert_eq!(looked_up.answer, drained.answer, "the full read's answer");
    assert_eq!(
        looked_up.answer, rendered.answer,
        "and the rendered bundle's"
    );
    assert_eq!(looked_up.exactness, drained.exactness, "as exactly");

    // The constant-needle neighbour: the undriven lookup, at the rendered price.
    assert_eq!(constant.fused_lookups, rendered.fused_lookups);
    assert_eq!(constant.membership_lookups, rendered.membership_lookups);
    assert_eq!(constant.ranks_pulled, rendered.ranks_pulled);
    assert_eq!(constant.answer, rendered.answer);
}

/// **A lookup driven by several needle bindings asks the call once per distinct
/// binding, answers `Excluded` only when every one excludes, and reads a producer
/// answering each invocation with one row as the conforming producer it is.**
///
/// The data names three `(configuration, needle)` pairs — two configurations asking
/// `"quick brown fox"`, one asking `"lazy dog"` — and the text reads the needle out
/// of it, keeping one pair with a `FILTER` so its ranked read names each candidate
/// once. The `FILTER` is over the whole group, not a conjunct of the call's join, so
/// the lookup does not keep it: its driving pattern binds both needles, which only
/// widens what it asks. For the other producer's candidate both needles' invocations
/// find nothing: `Excluded`, two invocations. For the stratum's own candidate both find
/// it: `Possible`, two rows — one per distinct needle, not three, because the driving
/// pattern is read `DISTINCT`, so the duplicate needle is one invocation and two
/// identical rows never reach the point-bound check.
///
/// The neighbour asks `"everything"` in place of `"lazy dog"`: that needle's lookup
/// holds every candidate, so the other producer's candidate — which the
/// `"quick brown fox"` invocation excludes — is `Possible`. A candidate is out of
/// reach only when every needle binding puts it there.
#[test]
fn a_lookup_driven_by_several_needles_asks_each_distinct_one_and_needs_all_to_exclude() {
    use purrdf_core::{RdfDatasetBuilder, RdfLiteral};

    let dataset_with = |second: &str| {
        let mut builder = RdfDatasetBuilder::new();
        let needle = builder.intern_iri(&ex("needle"));
        for (config, text) in [
            ("config/one", "quick brown fox"),
            ("config/two", "quick brown fox"),
            ("config/one", second),
        ] {
            let config = builder.intern_iri(&ex(config));
            let value = builder.intern_literal(RdfLiteral::language_tagged(text, "en"));
            builder.push_quad(config, needle, value, None);
        }
        builder.freeze().expect("the fixture dataset is valid")
    };
    let text = |predicate: &str| {
        format!(
            "SELECT ?candidate WHERE {{ ?config <{}> ?q . ( ?candidate ) <{}> ( ?q ) \
             FILTER(?q = \"quick brown fox\"@en && ?config = <{}>) }}",
            ex("needle"),
            producer_iri(predicate),
            ex("config/one")
        )
    };
    for (second, foreign, lookups) in [
        ("lazy dog", ExclusionVerdict::Excluded, 2 + 2),
        ("everything", ExclusionVerdict::Possible, 2 + 2),
    ] {
        let dataset = dataset_with(second);
        let (registry, counters) = fixture_registry(ExclusionBasis::Membership, None);
        let bundle = supplied(compiled(&registry), text).expect("admitted");
        for ((stratum, answered), reads) in asked_over(&bundle, &registry, &dataset)
            .into_iter()
            .zip(&counters)
        {
            let (verdict, own, held) =
                answered.unwrap_or_else(|reason| panic!("{second}: {stratum} failed: {reason}"));
            assert_eq!(
                verdict, foreign,
                "{second}: the other producer's candidate — {stratum}"
            );
            assert_eq!(
                own,
                ExclusionVerdict::Possible,
                "{second}: its own — {stratum}"
            );
            assert_eq!(
                held.len(),
                usize::try_from(ROWS).expect("small"),
                "{second}: the ranked read names each candidate once — {stratum}"
            );
            assert_eq!(
                reads.lookups.load(Ordering::SeqCst),
                lookups,
                "{second}: one invocation per distinct needle, per verdict — {stratum}"
            );
        }
    }
}

/// **A needle a `VALUES` table or a `BIND` supplies drives the lookup the same way:
/// kept, asked, and answered as the rendered bundle answers.**
///
/// The two other conjuncts a caller writes a needle with. Each is kept in its
/// lookup's driving sub-`SELECT` exactly as the data triple is, so each run asks the
/// rendered bundle's lookups, pulls its ranks and answers its answer — and the same
/// text declaring no basis, read in full, answers the same.
#[test]
fn a_needle_from_values_or_bind_drives_its_lookup_as_the_data_needle_does() {
    use text_relation::{NEEDLE, run};

    let rendered = run(ExclusionBasis::Membership, None, ReadSchedule::OnDemand)
        .expect("the rendered bundle fuses");
    let values = |producer: &str| {
        format!(
            "SELECT ?candidate WHERE {{ VALUES ?q {{ \"{NEEDLE}\" }} ?candidate <{producer}> ( \
             ?q ?score ?rank ?lang ?matched ) }}"
        )
    };
    let bound = |producer: &str| {
        format!(
            "SELECT ?candidate WHERE {{ BIND(\"{NEEDLE}\" AS ?q) ?candidate <{producer}> ( ?q \
             ?score ?rank ?lang ?matched ) }}"
        )
    };
    for (name, text) in [
        ("VALUES", &values as &dyn Fn(&str) -> String),
        ("BIND", &bound),
    ] {
        let looked_up = run(
            ExclusionBasis::Membership,
            Some(text),
            ReadSchedule::OnDemand,
        )
        .unwrap_or_else(|error| panic!("{name}: fuses with its lookups: {error:?}"));
        let full = run(
            ExclusionBasis::Unavailable,
            Some(text),
            ReadSchedule::Materialised,
        )
        .unwrap_or_else(|error| panic!("{name}: fuses without lookups: {error:?}"));
        assert_eq!(
            looked_up.statuses, rendered.statuses,
            "{name}: stopped where the rendered run stops"
        );
        assert_eq!(
            looked_up.fused_lookups, rendered.fused_lookups,
            "{name}: the rendered lookups"
        );
        assert_eq!(
            looked_up.membership_lookups, rendered.membership_lookups,
            "{name}: each a membership test"
        );
        assert_eq!(
            looked_up.ranks_pulled,
            both(66),
            "{name}: the rendered price"
        );
        assert_eq!(full.ranks_pulled, both(100), "{name}: the full read drains");
        assert_eq!(
            looked_up.answer, full.answer,
            "{name}: the full read's answer"
        );
        assert_eq!(
            looked_up.answer, rendered.answer,
            "{name}: the rendered answer"
        );
        assert_eq!(looked_up.exactness, full.exactness, "{name}: as exactly");
    }
}

// ---------------------------------------------------------------------------
// A needle a LATERAL reads: the driving pattern is the text's, correlation and all.
// ---------------------------------------------------------------------------

/// The data the correlated-needle texts read, beside the text relation's documents:
/// `<x> ex:p <y>`; `<y>`'s needle `"zulu yankee"`, which reaches the right index's
/// `a…` documents and no left one; `<y0>`'s and `<y2>`'s needle the stratum needle;
/// `<config2>`'s needle the stratum needle in the default graph but `"zulu yankee"`
/// in `<g1>`.
fn correlated_dataset() -> Arc<RdfDataset> {
    text_relation::dataset_with(&[
        ("x", "p", "<y>", None),
        ("y", "needle", "zulu yankee", None),
        ("y0", "needle", text_relation::NEEDLE, None),
        ("y2", "needle", text_relation::NEEDLE, None),
        ("config2", "needle", text_relation::NEEDLE, None),
        ("config2", "needle", "zulu yankee", Some("g1")),
    ])
}

/// The right stratum's text: `<x> ex:p ?y LATERAL { lateral }` and then the call,
/// its needle `?q`. The left stratum's is always its constant needle, so the right
/// stratum's text is the only thing a run varies.
fn correlated(producer: &str, lateral: &str) -> String {
    if producer.ends_with("text-left") {
        return format!(
            "SELECT ?candidate WHERE {{ ?candidate <{producer}> ( \"{}\" ?score ?rank ?lang \
             ?matched ) }}",
            text_relation::NEEDLE
        );
    }
    format!(
        "SELECT ?candidate WHERE {{ <{}> <{}> ?y LATERAL {{ {lateral} }} ?candidate \
         <{producer}> ( ?q ?score ?rank ?lang ?matched ) }}",
        ex("x"),
        ex("p")
    )
}

/// [`correlated`], the right stratum's text written in full by `right`.
fn right_only(producer: &str, right: impl Fn(&str) -> String) -> String {
    if producer.ends_with("text-left") {
        correlated(producer, "")
    } else {
        right(producer)
    }
}

/// One row of [`correlated_needles`].
type CorrelatedNeedle = (&'static str, Text<'static>, i128, [u64; 2], [u64; 2]);

/// `(name, text, the fused score of <a1>, the lookups each stratum's fusion asks,
/// the membership tests each stratum's relation performs)` for every text whose
/// needle is read through a `LATERAL`, a `GRAPH` or a dataset clause.
///
/// `<a1>` leads every answer. Its score is the observing oracle: it carries the right
/// stratum's contribution only where the right stratum's needles include
/// `"zulu yankee"` — the text's own needle set — and a lookup driven by any other set
/// excluded it there, leaving the left stratum's `16393442622` alone. The texts
/// binding both needles score it `22566282128`, the others `32522474880`. The
/// membership tests are the second oracle: two per needle term per candidate asked, so
/// a lookup driven by a wider needle set than the text's — an injected variable read
/// free — asks twice as many as one driven by the text's own `"zulu yankee"`.
fn correlated_needles() -> Vec<CorrelatedNeedle> {
    let needle = || ex("needle");
    vec![
        (
            "a VALUES row the LATERAL picks by a FILTER over the left's variable",
            Box::new(move |producer: &str| {
                correlated(
                    producer,
                    &format!(
                        "VALUES ?z {{ <{}> <{}> }} ?z <{}> ?q FILTER(?z = ?y || ?z = <{}>)",
                        ex("y0"),
                        ex("y"),
                        needle(),
                        ex("y0")
                    ),
                )
            }),
            22_566_282_128,
            [99, 100],
            [0, 400],
        ),
        (
            "a FILTER disjoining the left's variable with a constant",
            Box::new(move |producer: &str| {
                correlated(
                    producer,
                    &format!(
                        "?z <{}> ?q FILTER(?z = ?y || ?z = <{}>)",
                        needle(),
                        ex("y2")
                    ),
                )
            }),
            22_566_282_128,
            [99, 100],
            [0, 400],
        ),
        (
            "a FILTER over the left's variable",
            Box::new(move |producer: &str| {
                correlated(producer, &format!("?z <{}> ?q FILTER(?z = ?y)", needle()))
            }),
            32_522_474_880,
            [18, 25],
            [36, 50],
        ),
        (
            "a FILTER EXISTS correlated through the left's variable",
            Box::new(move |producer: &str| {
                correlated(
                    producer,
                    &format!(
                        "?z <{0}> ?q FILTER EXISTS {{ ?z <{0}> ?q FILTER(?z = ?y) }}",
                        needle()
                    ),
                )
            }),
            32_522_474_880,
            [18, 25],
            [36, 50],
        ),
        (
            "an uncorrelated LATERAL",
            Box::new(move |producer: &str| {
                correlated(
                    producer,
                    &format!("?z <{}> ?q FILTER(?z = <{}>)", needle(), ex("y")),
                )
            }),
            32_522_474_880,
            [18, 25],
            [36, 50],
        ),
        (
            "a correlated group inside the LATERAL, with the call",
            Box::new(move |producer: &str| {
                right_only(producer, |producer| {
                    format!(
                        "SELECT ?candidate WHERE {{ <{}> <{}> ?y LATERAL {{ {{ ?z <{}> ?q \
                         FILTER(?z = ?y) }} ?candidate <{producer}> ( ?q ?score ?rank ?lang \
                         ?matched ) }} }}",
                        ex("x"),
                        ex("p"),
                        needle()
                    )
                })
            }),
            32_522_474_880,
            [18, 25],
            [36, 50],
        ),
        (
            "a needle read in a named GRAPH",
            Box::new(move |producer: &str| {
                right_only(producer, |producer| {
                    format!(
                        "SELECT ?candidate WHERE {{ GRAPH <{}> {{ <{}> <{}> ?q . ?candidate \
                         <{producer}> ( ?q ?score ?rank ?lang ?matched ) }} }}",
                        ex("g1"),
                        ex("config2"),
                        needle()
                    )
                })
            }),
            32_522_474_880,
            [18, 25],
            [36, 50],
        ),
        (
            "a sub-SELECT whose needle triple reads the variable the LATERAL injects",
            Box::new(move |producer: &str| {
                right_only(producer, |producer| {
                    format!(
                        "SELECT ?candidate WHERE {{ <{}> <{}> ?y LATERAL {{ SELECT ?candidate \
                         ?y WHERE {{ ?y <{}> ?q . ?candidate <{producer}> ( ?q ?score ?rank \
                         ?lang ?matched ) }} }} }}",
                        ex("x"),
                        ex("p"),
                        needle()
                    )
                })
            }),
            32_522_474_880,
            [18, 25],
            [36, 50],
        ),
        (
            "a sub-SELECT whose needle the LATERAL injects",
            Box::new(move |producer: &str| {
                right_only(producer, |producer| {
                    format!(
                        "SELECT ?candidate WHERE {{ <{}> <{}> ?q LATERAL {{ SELECT ?candidate \
                         ?q WHERE {{ ?candidate <{producer}> ( ?q ?score ?rank ?lang ?matched ) \
                         }} }} }}",
                        ex("y"),
                        needle()
                    )
                })
            }),
            32_522_474_880,
            [18, 25],
            [36, 50],
        ),
        (
            "a sub-SELECT whose needle group's FILTER compares against the variable the \
             LATERAL injects",
            Box::new(move |producer: &str| {
                right_only(producer, |producer| {
                    format!(
                        "SELECT ?candidate WHERE {{ <{}> <{}> ?y LATERAL {{ SELECT ?candidate \
                         ?y WHERE {{ {{ ?z <{}> ?q FILTER(?z = ?y) }} ?candidate <{producer}> \
                         ( ?q ?score ?rank ?lang ?matched ) }} }} }}",
                        ex("x"),
                        ex("p"),
                        needle()
                    )
                })
            }),
            32_522_474_880,
            [18, 25],
            [36, 50],
        ),
        // The neighbour: the FILTER written in the call's own group filters the call's
        // rows, so it drives nothing — every needle the triple reads drives the lookup,
        // two terms each, which is looser than the text's one needle and still answers
        // as the full read.
        (
            "a sub-SELECT whose FILTER over the injected variable filters the call's own group",
            Box::new(move |producer: &str| {
                right_only(producer, |producer| {
                    format!(
                        "SELECT ?candidate WHERE {{ <{}> <{}> ?y LATERAL {{ SELECT ?candidate \
                         ?y WHERE {{ ?z <{}> ?q FILTER(?z = ?y) ?candidate <{producer}> ( ?q \
                         ?score ?rank ?lang ?matched ) }} }} }}",
                        ex("x"),
                        ex("p"),
                        needle()
                    )
                })
            }),
            32_522_474_880,
            [18, 25],
            [36, 100],
        ),
        (
            "a sub-SELECT inside a sub-SELECT, the injected variable carried through both",
            Box::new(move |producer: &str| {
                right_only(producer, |producer| {
                    format!(
                        "SELECT ?candidate WHERE {{ <{}> <{}> ?y LATERAL {{ SELECT ?candidate \
                         ?y WHERE {{ {{ SELECT ?candidate ?y WHERE {{ ?y <{}> ?q . ?candidate \
                         <{producer}> ( ?q ?score ?rank ?lang ?matched ) }} }} }} }} }}",
                        ex("x"),
                        ex("p"),
                        needle()
                    )
                })
            }),
            32_522_474_880,
            [18, 25],
            [36, 50],
        ),
        (
            "a sub-SELECT inside a GRAPH variable it does not project",
            Box::new(move |producer: &str| {
                right_only(producer, |producer| {
                    format!(
                        "SELECT ?candidate WHERE {{ GRAPH ?g {{ SELECT ?candidate WHERE {{ <{}> \
                         <{}> ?q . ?candidate <{producer}> ( ?q ?score ?rank ?lang ?matched ) \
                         }} }} }}",
                        ex("config2"),
                        needle()
                    )
                })
            }),
            32_522_474_880,
            [18, 25],
            [36, 50],
        ),
        (
            "a needle read under a FROM clause",
            Box::new(move |producer: &str| {
                right_only(producer, |producer| {
                    format!(
                        "SELECT ?candidate FROM <{}> WHERE {{ <{}> <{}> ?q . ?candidate \
                         <{producer}> ( ?q ?score ?rank ?lang ?matched ) }}",
                        ex("g1"),
                        ex("config2"),
                        needle()
                    )
                })
            }),
            32_522_474_880,
            [18, 25],
            [36, 50],
        ),
    ]
}

/// **A needle read through a `LATERAL` drives its lookup with the pattern the text
/// wrote — correlated where the text is — so the lookups answer what the full read
/// answers, as exactly.**
///
/// Each text reads the right stratum's needle through a `LATERAL` over `<x> ex:p ?y`,
/// and the correlated ones pick it through `?y`: a `FILTER` comparing against it, one
/// disjoining it with a constant, a `FILTER EXISTS` correlated through it, a `VALUES`
/// row picked by a `FILTER` over it. Read on their own, cut away from the left
/// operand, those patterns answer a different relation — `?y` unbound, `?z = ?y` an
/// error — which binds only `"alpha beta"`, or nothing, where the text binds
/// `"zulu yankee"` too. A lookup driven by that relation excluded `<a1>` from the
/// right stratum, which names it: a certified answer missing the right stratum's
/// contribution, or a failed request. Here each `LATERAL` is kept whole in the
/// lookup's driving pattern, and each text is run twice — with its lookups, read on
/// demand, and declaring no basis, read in full — to one answer, as exactly.
///
/// The call may sit inside a sub-`SELECT` the `LATERAL` injects into: its needle
/// triple reading the injected `?y`, a group whose `FILTER` compares against it, the
/// needle `?q` itself injected, and `?y` carried through two nested sub-`SELECT`s. A
/// lookup that left such a piece out drove the call with no needle — a mode the text
/// relation does not declare — and the right stratum failed, leaving `<a1>` the left
/// stratum's `16393442622`; here the injection is reproduced, and the membership
/// tests are the one needle's. The neighbour whose `FILTER` sits in the call's own
/// group filters the call's rows rather than driving it: its lookup is driven by every
/// needle the triple reads, twice the tests, and answers the same. A `GRAPH ?g` around
/// a sub-`SELECT` that does not project `?g` is read under a fresh name, in every named
/// graph — here `<g1>` alone, whose `"zulu yankee"` the text reads.
///
/// The neighbours: an uncorrelated `LATERAL`, still admitted and answering the same;
/// a correlated group inside the `LATERAL`'s right operand beside the call, whose
/// frame is rebuilt inside a `LATERAL` of the frame above it; a needle read in a
/// named `GRAPH`, and one read under a `FROM` clause, each of which reads
/// `"zulu yankee"` where the default graph holds `"alpha beta"` — the lookup reads
/// them in that graph, under that clause. The answer is the observing oracle: every
/// correlated text's right stratum names `<a1>` through `"zulu yankee"`, which the
/// full read's answer carries, and a lookup driven by the default graph's needle, or
/// by none, would have excluded it.
#[test]
fn a_needle_read_through_a_lateral_drives_its_lookup_with_the_text_s_own_pattern() {
    use text_relation::run_over;

    let dataset = correlated_dataset();
    let [left, right] = strata();
    let per_stratum = |[on_left, on_right]: [u64; 2]| {
        BTreeMap::from([(left.clone(), on_left), (right.clone(), on_right)])
    };
    for (name, text, first, lookups, membership) in correlated_needles() {
        let looked_up = run_over(
            &dataset,
            ExclusionBasis::Membership,
            Some(&text),
            ReadSchedule::OnDemand,
        )
        .unwrap_or_else(|error| panic!("{name}: fuses with its lookups: {error:?}"));
        let full = run_over(
            &dataset,
            ExclusionBasis::Unavailable,
            Some(&text),
            ReadSchedule::Materialised,
        )
        .unwrap_or_else(|error| panic!("{name}: fuses without lookups: {error:?}"));
        assert!(
            looked_up
                .statuses
                .values()
                .all(|status| !matches!(status, ProducerStatus::ExecutionFailed { .. })),
            "{name}: both strata read: {:?}",
            looked_up.statuses
        );
        assert_eq!(
            looked_up.answer, full.answer,
            "{name}: the full read's answer"
        );
        assert_eq!(looked_up.exactness, full.exactness, "{name}: as exactly");
        assert_eq!(
            looked_up.answer.first(),
            Some(&(format!("<{}>", ex("a1")), Fixed::from_raw(first))),
            "{name}: <a1> leads, carrying what the right stratum's own needles name"
        );
        assert_eq!(
            full.fused_lookups,
            both(0),
            "{name}: the oracle asks nothing"
        );
        assert_eq!(
            looked_up.fused_lookups,
            per_stratum(lookups),
            "{name}: the lookups each stratum asks"
        );
        assert_eq!(
            looked_up.membership_lookups,
            per_stratum(membership),
            "{name}: the membership tests each relation performs"
        );
    }
}

/// **A driving pattern that binds no input at all excludes every candidate, having
/// invoked nothing — and the answer is the full read's.**
///
/// `<x> ex:p ?y LATERAL { ?z ex:needle ?q FILTER(?z = ?y && ?z = <y0>) }` binds no
/// needle: `?y` is `<y>`. So the text never invokes the right stratum's call, its
/// stream names nothing, and neither does the lookup's driving sub-`SELECT` bind a
/// needle to invoke the call with. The call attests nothing, because it ran not once
/// — and that is the proof, not a failure to attest: no binding of the text's inputs
/// exists, so no candidate is one the text names. Asked directly, before its stream
/// is read, the stratum answers `Excluded` for a candidate of either index and for
/// the one its correlated neighbour names, with no membership test run.
///
/// The neighbour differs in one constant, `?z = <y>`, and binds `"zulu yankee"`: its
/// stream names `<a1>`, which it answers `Possible`, and `<b1>` — a right-index
/// document the needle does not reach — `Excluded`, each by a membership test. And
/// the empty text, fused with its lookups, answers what it answers read in full with
/// none, as exactly.
#[test]
fn a_driving_pattern_binding_no_input_excludes_every_candidate_and_answers_the_full_read() {
    use text_relation::{asked_right, run_over};

    let dataset = correlated_dataset();
    let picking = |constant: &'static str| {
        move |producer: &str| {
            correlated(
                producer,
                &format!(
                    "?z <{}> ?q FILTER(?z = ?y && ?z = <{}>)",
                    ex("needle"),
                    ex(constant)
                ),
            )
        }
    };
    let probes: Vec<String> = ["a1", "b1", "a13"]
        .into_iter()
        .map(|subject| format!("<{}>", ex(subject)))
        .collect();

    let (verdicts, held, membership) = asked_right(&dataset, &picking("y0"), &probes);
    assert_eq!(
        held,
        Vec::<String>::new(),
        "the text binds no needle and names nothing"
    );
    assert_eq!(
        verdicts,
        vec![Ok(ExclusionVerdict::Excluded); 3],
        "every candidate excluded, none refused"
    );
    assert_eq!(membership, 0, "the call was invoked not once");

    let (verdicts, held, membership) = asked_right(&dataset, &picking("y"), &probes);
    assert_eq!(
        held.len(),
        100,
        "the neighbour names every document its needle reaches"
    );
    assert!(held.contains(&probes[0]) && !held.contains(&probes[1]));
    assert_eq!(
        verdicts,
        vec![
            Ok(ExclusionVerdict::Possible),
            Ok(ExclusionVerdict::Excluded),
            Ok(ExclusionVerdict::Possible)
        ],
        "the neighbour's lookups answer from its needle"
    );
    assert_eq!(
        membership, 6,
        "one membership test per needle term, per candidate asked"
    );

    let empty = picking("y0");
    let looked_up = run_over(
        &dataset,
        ExclusionBasis::Membership,
        Some(&empty),
        ReadSchedule::OnDemand,
    )
    .expect("the empty text fuses with its lookups");
    let full = run_over(
        &dataset,
        ExclusionBasis::Unavailable,
        Some(&empty),
        ReadSchedule::Materialised,
    )
    .expect("the empty text fuses without lookups");
    assert_eq!(looked_up.answer, full.answer, "the full read's answer");
    assert_eq!(looked_up.exactness, full.exactness, "as exactly");
    assert_eq!(
        looked_up.answer.first(),
        Some(&(format!("<{}>", ex("a1")), Fixed::from_raw(16_393_442_622))),
        "the left stratum's alone: the right names nothing"
    );
    assert!(
        looked_up
            .statuses
            .values()
            .all(|status| !matches!(status, ProducerStatus::ExecutionFailed { .. })),
        "{:?}",
        looked_up.statuses
    );
}

// ---------------------------------------------------------------------------
// Soundness over data: no verdict excludes a candidate the stream names.
// ---------------------------------------------------------------------------

/// Both producers' first forty candidates tagged in the default graph and in a named
/// graph, an intruder no producer names tagged too and aliased from each producer's
/// fourth candidate.
fn tagged_dataset() -> Arc<RdfDataset> {
    use purrdf_core::{RdfDatasetBuilder, RdfLiteral};
    let mut builder = RdfDatasetBuilder::new();
    let tag = builder.intern_iri(&ex("data/tag"));
    let alias = builder.intern_iri(&ex("data/alias"));
    let graph = builder.intern_iri(&ex("g1"));
    let value = builder.intern_literal(RdfLiteral::simple("t"));
    let intruder = builder.intern_iri(&ex("intruder"));
    for prefix in DISJOINT {
        for index in 0..40_u64 {
            let subject = builder.intern_iri(&ex(&format!("{prefix}entity{index:06}")));
            builder.push_quad(subject, tag, value, None);
            builder.push_quad(subject, tag, value, Some(graph));
        }
        let fourth = builder.intern_iri(&ex(&format!("{prefix}entity000003")));
        builder.push_quad(fourth, alias, intruder, None);
    }
    builder.push_quad(intruder, tag, value, None);
    builder.push_quad(intruder, tag, value, Some(graph));
    builder.freeze().expect("the fixture dataset is valid")
}

/// `<left/entity000003> data:alias ?y LATERAL { lateral }` and then the call, its
/// needle `?q`, over [`tagged_dataset`]: `?y` is the intruder.
fn aliased_needle(predicate: &str, lateral: &str) -> String {
    format!(
        "SELECT ?candidate WHERE {{ <{}> <{}> ?y LATERAL {{ {lateral} }} ( ?candidate ) <{}> ( \
         ?q ) }}",
        ex("left/entity000003"),
        ex("data/alias"),
        producer_iri(predicate)
    )
}

/// `<left/entity000003> data:alias ?y LATERAL { SELECT ?candidate ?y WHERE { body
/// call } }`, the call's needle `?q`, over [`tagged_dataset`]: `?y`, the intruder, is
/// injected into the sub-`SELECT`, which projects it.
fn aliased_subselect(predicate: &str, body: &str) -> String {
    format!(
        "SELECT ?candidate WHERE {{ <{}> <{}> ?y LATERAL {{ SELECT ?candidate ?y WHERE {{ {body} \
         ( ?candidate ) <{}> ( ?q ) }} }} }}",
        ex("left/entity000003"),
        ex("data/alias"),
        producer_iri(predicate)
    )
}

/// What a shape of the soundness sweep does when a basis is declared over it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sweep {
    /// Admitted, and both strata run and answer verdicts.
    Runs,
    /// Refused at construction, under the widest reading of its predicates.
    Refused,
    /// Admitted at construction, and each stratum fails at execution: no call the
    /// registry registers is a source of its column.
    FailsItsStratum,
    /// Admitted, and the text itself names more rows than its producer declares —
    /// a refusal of the read, reached before any verdict is asked.
    BreachesItsBound,
}

/// **Over a dataset the text's patterns really read, every shape a caller can write
/// around the call is either refused or answers no verdict the stream contradicts.**
///
/// Each text is built into a bundle declaring membership, executed over
/// [`tagged_dataset`] on demand, and every stream is asked about the intruder and
/// candidates on both sides of each producer's range *before* it is read; then it is
/// read to its end. A verdict of `Excluded` for a candidate the stream then names
/// would be a lookup answering for a column its call does not fill — the defect the
/// shape description exists to rule out. A text refused at construction, or a
/// stratum failed at execution, answers nothing and so contradicts nothing; the
/// shapes that must be admitted are pinned as admitted, so the sweep cannot pass by
/// refusing everything.
#[test]
fn no_admitted_shape_excludes_a_candidate_its_stream_names() {
    let tag = ex("data/tag");
    let alias = ex("data/alias");
    let intruder = ex("intruder");
    let call = |p: &str, v: &str| call_binding(p, v);
    let pf = |p: &str| producer_iri(p);
    let shapes: Vec<(&str, Text<'_>, Sweep)> = vec![
        (
            "join data after",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ {} . ?candidate <{tag}> ?t }}",
                    call(p, "candidate")
                )
            }),
            Sweep::Runs,
        ),
        (
            "join data before",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ ?candidate <{tag}> ?t . {} }}",
                    call(p, "candidate")
                )
            }),
            Sweep::Runs,
        ),
        (
            "alias data candidate",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ {} . ?c <{alias}> ?candidate }}",
                    call(p, "c")
                )
            }),
            Sweep::FailsItsStratum,
        ),
        (
            "alias joined with the call",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ {} . ?c <{alias}> ?candidate . {} }}",
                    call(p, "c"),
                    call(p, "candidate")
                )
            }),
            Sweep::Runs,
        ),
        (
            "call on the optional side",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ ?candidate <{tag}> ?t OPTIONAL {{ {} }} }}",
                    call(p, "candidate")
                )
            }),
            Sweep::FailsItsStratum,
        ),
        (
            "nested optional",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ {{ ?x <{tag}> ?t OPTIONAL {{ {} }} }} OPTIONAL {{ ?x <{alias}> ?candidate }} }}",
                    call(p, "candidate")
                )
            }),
            Sweep::FailsItsStratum,
        ),
        (
            "required call, data optional",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ {} OPTIONAL {{ ?candidate <{alias}> ?o }} }}",
                    call(p, "candidate")
                )
            }),
            Sweep::Runs,
        ),
        (
            "GRAPH ?g over the call and data",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ GRAPH ?g {{ {} . ?candidate <{tag}> ?t }} }}",
                    call(p, "candidate")
                )
            }),
            Sweep::Runs,
        ),
        (
            "FILTER EXISTS",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ {} FILTER EXISTS {{ ?candidate <{tag}> ?t }} }}",
                    call(p, "candidate")
                )
            }),
            Sweep::Runs,
        ),
        (
            "FILTER NOT EXISTS",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ {} FILTER NOT EXISTS {{ ?candidate <{alias}> ?t }} }}",
                    call(p, "candidate")
                )
            }),
            Sweep::Runs,
        ),
        (
            "ORDER BY and LIMIT sub-select",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ {{ SELECT ?candidate WHERE {{ {} }} ORDER BY DESC(?candidate) LIMIT 50 }} }}",
                    call(p, "candidate")
                )
            }),
            Sweep::Runs,
        ),
        (
            "DISTINCT over GROUP BY",
            Box::new(|p: &str| {
                format!(
                    "SELECT DISTINCT ?candidate WHERE {{ {} }} GROUP BY ?candidate",
                    call(p, "candidate")
                )
            }),
            Sweep::Runs,
        ),
        (
            "MINUS data",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ {} MINUS {{ ?candidate <{tag}> ?t }} }}",
                    call(p, "candidate")
                )
            }),
            Sweep::Runs,
        ),
        (
            "VALUES with the intruder joined",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ VALUES ?candidate {{ <{intruder}> <{}> }} {} }}",
                    ex("left/entity000003"),
                    call(p, "candidate")
                )
            }),
            Sweep::Runs,
        ),
        (
            "renaming BIND then data",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ {{ {} BIND(?c AS ?candidate) }} ?candidate <{tag}> ?t }}",
                    call(p, "c")
                )
            }),
            Sweep::Runs,
        ),
        (
            "UNION of data joined with the call",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ {{ ?candidate <{tag}> ?t }} UNION {{ ?z <{alias}> ?w }} {} }}",
                    call(p, "candidate")
                )
            }),
            Sweep::BreachesItsBound,
        ),
        (
            "sub-select hiding the candidate",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ {{ SELECT ?x WHERE {{ {} . ?candidate <{alias}> ?x }} }} ?x <{tag}> ?t . ?candidate <{tag}> ?t2 }}",
                    call(p, "candidate")
                )
            }),
            Sweep::FailsItsStratum,
        ),
        (
            "candidate in the needle too",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ ( ?candidate ) <{}> ( ?candidate ) }}",
                    pf(p)
                )
            }),
            Sweep::Runs,
        ),
        (
            "LATERAL call",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ ?candidate <{tag}> ?t LATERAL {{ {} }} }}",
                    call(p, "candidate")
                )
            }),
            Sweep::Runs,
        ),
        (
            "LATERAL sub-select rebinding",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ {} LATERAL {{ SELECT ?candidate WHERE {{ ?candidate <{alias}> ?o }} }} }}",
                    call(p, "candidate")
                )
            }),
            Sweep::Runs,
        ),
        (
            "VALUES after an optional call",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ {{ ?x <{tag}> ?t OPTIONAL {{ {} }} }} VALUES ?candidate {{ <{intruder}> }} }}",
                    call(p, "candidate")
                )
            }),
            Sweep::Refused,
        ),
        ("UNION of calls", Box::new(union_of_calls), Sweep::Runs),
        (
            "GROUP BY rebinding the candidate",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ {} }} GROUP BY (IF(?candidate = <{}>, <{intruder}>, ?candidate) AS ?candidate)",
                    call(p, "candidate"),
                    first_of(p)
                )
            }),
            Sweep::Refused,
        ),
        (
            "renaming GROUP BY condition",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ {} }} GROUP BY (?hit AS ?candidate)",
                    call(p, "hit")
                )
            }),
            Sweep::Runs,
        ),
        // A needle a `LATERAL` over the fourth candidate's alias picks through the
        // left's `?y` — the intruder — and the uncorrelated neighbour.
        (
            "LATERAL: a VALUES row picked by a FILTER disjoining the left's variable",
            Box::new(|p: &str| {
                aliased_needle(
                    p,
                    &format!(
                        "VALUES ?z {{ <{0}> <{intruder}> }} ?z <{tag}> ?q FILTER(?z = ?y || ?z = <{0}>)",
                        ex("left/entity000399")
                    ),
                )
            }),
            Sweep::Runs,
        ),
        (
            "LATERAL: a FILTER disjoining the left's variable",
            Box::new(|p: &str| {
                aliased_needle(
                    p,
                    &format!(
                        "?z <{tag}> ?q FILTER(?z = ?y || ?z = <{}>)",
                        ex("left/entity000399")
                    ),
                )
            }),
            Sweep::Runs,
        ),
        (
            "LATERAL: a FILTER over the left's variable",
            Box::new(|p: &str| aliased_needle(p, &format!("?z <{tag}> ?q FILTER(?z = ?y)"))),
            Sweep::Runs,
        ),
        (
            "LATERAL: a FILTER EXISTS over the left's variable",
            Box::new(|p: &str| {
                aliased_needle(
                    p,
                    &format!("?z <{tag}> ?q FILTER EXISTS {{ ?z <{tag}> ?q FILTER(?z = ?y) }}"),
                )
            }),
            Sweep::Runs,
        ),
        (
            "LATERAL: uncorrelated",
            Box::new(|p: &str| {
                aliased_needle(p, &format!("?z <{tag}> ?q FILTER(?z = <{intruder}>)"))
            }),
            Sweep::Runs,
        ),
        // The same needles picked inside a sub-`SELECT` the `LATERAL` injects `?y`
        // into, and a needle the `LATERAL` injects itself.
        (
            "LATERAL sub-SELECT: a triple reading the injected variable",
            Box::new(|p: &str| aliased_subselect(p, &format!("?y <{tag}> ?q ."))),
            Sweep::Runs,
        ),
        (
            "LATERAL sub-SELECT: a FILTER over the injected variable",
            Box::new(|p: &str| {
                aliased_subselect(p, &format!("{{ ?z <{tag}> ?q FILTER(?z = ?y) }}"))
            }),
            Sweep::Runs,
        ),
        (
            "LATERAL sub-SELECT: the needle injected",
            Box::new(|p: &str| {
                format!(
                    "SELECT ?candidate WHERE {{ <{intruder}> <{tag}> ?q LATERAL {{ SELECT \
                     ?candidate ?q WHERE {{ ( ?candidate ) <{}> ( ?q ) }} }} }}",
                    pf(p)
                )
            }),
            Sweep::Runs,
        ),
    ];
    let mut probes = vec![format!("<{intruder}>")];
    for prefix in DISJOINT {
        for index in [0_u64, 3, 7, 39, 45, 399] {
            probes.push(format!("<{}>", ex(&format!("{prefix}entity{index:06}"))));
        }
    }
    let dataset = tagged_dataset();
    for (name, text, expected) in &shapes {
        let (registry, _) = fixture_registry(ExclusionBasis::Membership, None);
        let Ok(bundle) = supplied(compiled(&registry), text) else {
            assert_eq!(*expected, Sweep::Refused, "{name}: refused at construction");
            continue;
        };
        assert_ne!(
            *expected,
            Sweep::Refused,
            "{name}: admitted at construction"
        );
        let executed = block_on(execute_within(
            &bundle,
            &registry,
            &*dataset,
            ReadSchedule::OnDemand,
        ));
        let ExecutionResult {
            mut streams,
            statuses,
        } = match executed {
            Ok(executed) => executed,
            Err(error) => {
                assert_eq!(
                    *expected,
                    Sweep::BreachesItsBound,
                    "{name}: the bundle runs: {error:?}"
                );
                assert!(
                    matches!(error, ExecutionError::RowBoundBreached { .. }),
                    "{name}: the text names more rows than its producer declares: {error:?}"
                );
                continue;
            }
        };
        match expected {
            Sweep::Runs => assert_eq!(streams.len(), 2, "{name}: both strata run: {statuses:?}"),
            Sweep::FailsItsStratum => assert!(
                streams.is_empty()
                    && statuses.values().all(|status| matches!(
                        status,
                        ProducerStatus::ExecutionFailed { reason }
                            if reason.contains("could not be derived from its query")
                    )),
                "{name}: no registered call is a source, so each stratum fails: {statuses:?}"
            ),
            Sweep::Refused | Sweep::BreachesItsBound => {
                panic!("{name}: expected {expected:?}, and it ran: {statuses:?}")
            }
        }
        for stream in &mut streams {
            let verdicts: Vec<(String, ExclusionVerdict)> = probes
                .iter()
                .map(|probe| {
                    let verdict = block_on(stream.stream.exclusion(&Term::new(probe.clone())))
                        .unwrap_or_else(|error| panic!("{name}: the lookup answers: {error:?}"));
                    (probe.clone(), verdict)
                })
                .collect();
            let mut held = Vec::new();
            while let Some((_, term, _)) = block_on(stream.stream.next()).unwrap_or_else(|error| {
                panic!("{name}: the stream reads after its verdicts: {error:?}")
            }) {
                held.push(term.as_str().to_owned());
            }
            let contradicted: Vec<&String> = verdicts
                .iter()
                .filter(|(probe, verdict)| {
                    *verdict == ExclusionVerdict::Excluded && held.contains(probe)
                })
                .map(|(probe, _)| probe)
                .collect();
            assert!(
                contradicted.is_empty(),
                "{name}: {} excluded candidates it names: {contradicted:?}",
                stream.stratum
            );
            // And the lookups really answered: the other producer's first candidate,
            // which no call of this stratum emits, is excluded.
            let foreign = if stream.stratum == strata()[0] {
                format!("<{}>", ex("right/entity000000"))
            } else {
                format!("<{}>", ex("left/entity000000"))
            };
            assert!(
                verdicts.contains(&(foreign, ExclusionVerdict::Excluded)),
                "{name}: {} excluded the other producer's candidate: {verdicts:?}",
                stream.stratum
            );
        }
    }
}
