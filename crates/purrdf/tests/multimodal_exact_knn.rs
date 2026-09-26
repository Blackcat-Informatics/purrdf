// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A text stratum and an **exact** nearest-neighbour stratum in one fused answer,
//! measured end to end through the umbrella alone — and what a producer that takes
//! its depth as an argument costs when the fusion stops long before that depth.
//!
//! `multimodal_exclusion_lookup.rs` measures the same mechanism with the approximate
//! graph index on the vector side. This file puts the exact scan there: a real
//! [`TextSearchRelation`] over a real inverted index and a real
//! [`EmbeddingKnnRelation`] over a real, sealed embedding artifact. They share a block
//! and hold disjoint candidates, so no candidate either stream names is final while
//! the other is open, and only an exclusion lookup — "you will never name this" — lets
//! the fusion stop before both streams drain.
//!
//! # What the depth argument costs
//!
//! The vector stratum is handed the planned depth as its `k` and opened once. The
//! fusion stops pulling it far shallower, and the rows past the stop are never built.
//! The *search* is not shortened, and cannot be: an exact top-`k` has measured every
//! row of the space before it can name its first neighbour, at any `k`. The counters
//! below pin that directly — one scan, and one distance per row of the space, in the
//! run that stopped early and in the control that drained alike — so a change that
//! opened the producer shallower would show here as saving nothing, and one that
//! continued it with a second invocation would show as a second scan.
//!
//! # The shape a consumer may write with the candidate already bound
//!
//! A call with the neighbour bound *and* the count bound asks "is this term among the
//! `k` nearest" — the ranked question, answered by ranking. It is a legitimate query,
//! not a malformed exclusion lookup, and it is answered: its answer is the full ranked
//! reading restricted to that term, and its cost is a whole search per bound term,
//! which the producers' own counters report.
//!
//! Every IRI below is this test's own, in the host's role. PurRDF mints no vocabulary.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use purrdf::hnsw::relation::{HnswRelation, HnswSpace};
use purrdf::hnsw::{HnswIndex, Params, VectorMatrix};
use purrdf::retrieval::{
    AdmissionEnvironment, CandidateDomains, DecayRule, DomainTag, ExclusionBasis, Fixed, FusedRow,
    FusionProfile, Iri, RECIP_K, RankFidelity, RequestTerm, RetrievalRequest, Statistics, Term,
    TopK, plan, search,
};
use purrdf::sparql::{
    EmbeddingKnnRelation, EmbeddingSpace, ExtensionEnv, InternedOutcome, KnnGuard, KnnObservations,
    NativeSparqlEngine, PropertyFunctionRegistry, QueryOptions, RankedDeclaration, TermKind,
};
use purrdf::text::{
    GraphSelector, SearchObservations, TextIndex, TextIndexConfig, TextSearchRelation,
};
use purrdf::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, CanonicalMetadataInput,
    CertifiedPurrpckSource, ContentDigest, DimensionalityPolicy, DistanceMetric, EmbeddingBuilder,
    EmbeddingFamilyContract, MatrixInput, MatrixRow, PrefixPostprocessing, ProjectionSpec,
    RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfTermTarget, StageImplementation, TargetSet,
    TermValue, VectorDtype,
};

// ---------------------------------------------------------------------------
// The host's vocabulary and the fixture's dimensions
// ---------------------------------------------------------------------------

/// The one predicate the text corpus is indexed over.
const NOTE: &str = "https://example.org/note";
/// The IRI this host registers the text relation under.
const TEXT_PF: &str = "https://example.org/pf/search";
/// The IRI this host registers the exact nearest-neighbour relation under.
const KNN_PF: &str = "https://example.org/pf/nearest";
/// The IRI this host registers the graph-index relation under, for the bound-candidate
/// pin alone.
const HNSW_PF: &str = "https://example.org/pf/neighbours";
/// The stratum this host ranks lexical rows within.
const TEXT_STRATUM: &str = "https://example.org/stratum/lexical";
/// The stratum this host ranks vector rows within.
const KNN_STRATUM: &str = "https://example.org/stratum/vector";
/// The one block both producers declare.
const SHARED_BLOCK: &str = "https://example.org/domain/shared";
/// The datatype the depth argument is written under — the host's, never invented here.
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// The needle the text producer is asked for.
const NEEDLE: &str = "alpha beta";

/// How many documents the needle reaches, and how many rows the vector space holds.
const CORPUS: usize = 80;

/// The vector space's dimensionality.
const DIMS: usize = 8;

/// The bound the request searches under.
const TOP_K: TopK = TopK::new(5);

/// The smoothing constant the fusion profile decays by.
const K: u32 = RECIP_K as u32;

/// The `k` the bound-candidate pin asks for: well inside the space, so most terms are
/// outside the offer and the pin has a genuine answer in both directions.
const OFFER: usize = 10;

fn text_subject(at: usize) -> String {
    format!("https://example.org/doc/text/{at}")
}

fn vector_term(at: usize) -> String {
    format!("https://example.org/doc/vec/{at}")
}

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

fn kernel_iri(text: &str) -> purrdf::iri::Iri {
    purrdf::iri::parse(text).expect("fixture IRIs are valid")
}

fn shared_block() -> DomainTag {
    DomainTag::parse(SHARED_BLOCK).expect("the fixture domain tag is a valid IRI")
}

/// A minimal single-threaded executor. Nothing in this pipeline actually pends.
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
// The corpus
// ---------------------------------------------------------------------------

/// [`CORPUS`] documents the needle reaches, and one document per vector term whose
/// text shares no term with it — so a text-side lookup about a vector candidate is a
/// real dictionary search that finds no posting.
fn text_rows() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = (0..CORPUS)
        .map(|at| {
            (
                text_subject(at),
                format!("alpha beta gamma {}", "alpha ".repeat(at % 4 + 1).trim()),
            )
        })
        .collect();
    out.extend((0..CORPUS).map(|at| (vector_term(at), "zulu yankee xray whiskey".to_owned())));
    out
}

fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri(NOTE);
    for (subject, text) in text_rows() {
        let subject = builder.intern_iri(&subject);
        let object = builder.intern_literal(RdfLiteral::simple(&text));
        builder.push_quad(subject, predicate, object, None);
    }
    builder.freeze().expect("the fixture dataset is valid")
}

fn text_index(dataset: &RdfDataset) -> Arc<TextIndex> {
    let config = TextIndexConfig::new(vec![TermValue::iri(NOTE)], GraphSelector::Any)
        .expect("the fixture configuration is well formed");
    Arc::new(TextIndex::from_dataset(dataset, &config).expect("the fixture index builds"))
}

/// [`CORPUS`] rows of [`DIMS`] components, deterministic, with no zero component.
fn vectors() -> Vec<Vec<f64>> {
    let mut state = 0x51DE_0000_1234_ABCD_u64;
    (0..CORPUS)
        .map(|_| {
            (0..DIMS)
                .map(|_| {
                    // splitmix64, spelled here so the fixture depends on no private helper.
                    state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
                    let mut z = state;
                    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
                    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
                    z ^= z >> 31;
                    let value = ((z >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0);
                    if value == 0.0 { 0.125 } else { value }
                })
                .collect()
        })
        .collect()
}

fn identity(name: &str) -> ArtifactIdentity {
    ArtifactIdentity::new(
        format!("https://example.org/artifact/{name}"),
        "application/octet-stream",
        ContentDigest::of(name.as_bytes()),
        None,
        ArtifactIdentityKind::Single,
    )
    .expect("the fixture artifact identity is well formed")
}

fn stage(name: &str) -> AppliedStage {
    AppliedStage::Applied(
        StageImplementation::new(
            format!("https://example.org/stage/{name}"),
            ContentDigest::of(name.as_bytes()),
            "application/octet-stream",
            vec![1],
        )
        .expect("the fixture stage is well formed"),
    )
}

/// The exact space: a sealed embedding artifact over [`vectors`], named by the vector
/// terms and by **no text subject**, opened and verified exactly as a host opens one.
fn knn_space() -> EmbeddingSpace {
    let rows = vectors();
    let dimension = u32::try_from(DIMS).expect("small");
    let empty = RdfDatasetBuilder::new().freeze().expect("empty dataset");
    let (source, _) = CertifiedPurrpckSource::from_dataset(&empty).expect("source pack");

    let mut targets = Vec::with_capacity(CORPUS);
    let mut bindings = Vec::with_capacity(CORPUS);
    for at in 0..CORPUS {
        let target = RdfTermTarget::Iri(vector_term(at))
            .into_target(true, None)
            .expect("the fixture term target is well formed");
        bindings.push((target.id, TermValue::iri(vector_term(at))));
        targets.push(target);
    }
    let set = TargetSet::new(targets.iter().map(|target| target.id).collect())
        .expect("the fixture target set is well formed");
    let mut declared = targets;
    declared.push(source.dataset_target(true).expect("dataset target"));
    declared.sort_unstable_by_key(|target| target.id);

    let contract = EmbeddingFamilyContract {
        model: identity("model"),
        engine: identity("engine"),
        tokenizer: identity("tokenizer"),
        execution: stage("execution"),
        subject_projection: stage("projection"),
        preprocessing: AppliedStage::NotApplied,
        chunking: AppliedStage::NotApplied,
        pooling: stage("pooling"),
        normalization: AppliedStage::NotApplied,
        truncation: AppliedStage::NotApplied,
        dtype: VectorDtype::F64,
        metric: DistanceMetric::SquaredEuclidean,
        dimensionality: DimensionalityPolicy::fixed(dimension, PrefixPostprocessing::None)
            .expect("fixed dimensions"),
        extensions: Vec::new(),
    };
    let family = contract.derive().expect("the family derives");
    let projection = ProjectionSpec::derive(family.id, dimension, PrefixPostprocessing::None);
    let vector_space = projection.vector_space_id;
    let matrix = MatrixInput {
        family_id: family.id,
        target_set_id: set.id,
        stored_dimension: dimension,
        rows: rows
            .into_iter()
            .zip(&bindings)
            .map(|(values, (target, _))| MatrixRow::new(*target, values))
            .collect(),
        projections: vec![projection],
    };
    let metadata = CanonicalMetadataInput {
        source,
        family_contracts: vec![contract],
        targets: declared,
        target_sets: vec![set.clone()],
        relations: Vec::new(),
        token_spans: Vec::new(),
        external_bindings: Vec::new(),
        indexes: Vec::new(),
        extensions: Vec::new(),
    };
    let mut builder = EmbeddingBuilder::from_typed_metadata(metadata);
    builder.add_f64_matrix(matrix);
    let encoded = builder.build().expect("the fixture artifact encodes");
    let guard = KnnGuard::new(CORPUS as u64, CORPUS as u64).expect("a valid guard");
    EmbeddingSpace::from_artifact(&encoded.bytes, set.id, vector_space, bindings, guard)
        .expect("the fixture space opens")
}

/// The graph-index space over the same vectors and terms, with a beam as wide as the
/// corpus — for the bound-candidate pin alone.
fn hnsw_space() -> Arc<HnswSpace> {
    let data: Vec<f64> = vectors().into_iter().flatten().collect();
    let matrix = VectorMatrix::new(CORPUS, DIMS, data).expect("a valid matrix");
    let params = Params::new(8, 16, 64, CORPUS).expect("valid parameters");
    let index =
        HnswIndex::build(matrix, &DistanceMetric::SquaredEuclidean, params).expect("it builds");
    let terms: Vec<TermValue> = (0..CORPUS)
        .map(|at| TermValue::iri(vector_term(at)))
        .collect();
    let guard = KnnGuard::new(CORPUS as u64, CORPUS as u64).expect("a valid guard");
    Arc::new(HnswSpace::from_index(index, terms, guard).expect("a valid space"))
}

// ---------------------------------------------------------------------------
// The wiring
// ---------------------------------------------------------------------------

fn request_terms() -> Vec<RequestTerm> {
    vec![
        RequestTerm::Lexical {
            text: NEEDLE.to_owned(),
            language: None,
            predicate: Some(iri(NOTE)),
        },
        RequestTerm::EntitySeed {
            entity: Term::new(format!("<{}>", vector_term(0))),
        },
    ]
}

/// Both producers registered, with `basis` written into the declaration each relation
/// hands out for itself — the only field this function touches.
fn registry(
    dataset: &RdfDataset,
    basis: ExclusionBasis,
) -> (
    PropertyFunctionRegistry,
    Arc<SearchObservations>,
    Arc<KnnObservations>,
) {
    let mut registry = PropertyFunctionRegistry::new();

    let text = TextSearchRelation::new(text_index(dataset));
    let text_observations = text.observations();
    let mut text_declaration: RankedDeclaration = text
        .ranked_declaration(
            kernel_iri(TEXT_STRATUM),
            Some(NOTE.to_owned()),
            RankFidelity::EXACT,
            CandidateDomains::within([shared_block()]),
        )
        .expect("a single-partition index declares a ranked order");
    assert_eq!(text_declaration.exclusion, ExclusionBasis::Membership);
    text_declaration.exclusion = basis;
    registry.register_ranked(TEXT_PF, Arc::new(text), text_declaration);

    let knn = EmbeddingKnnRelation::new(Arc::new(knn_space()));
    let knn_observations = knn.observations();
    let mut knn_declaration = knn.ranked_declaration(
        kernel_iri(KNN_STRATUM),
        TermKind::Iri,
        XSD_INTEGER.to_owned(),
        // The space holds a vector for every row this host embedded, and the scan
        // compares exact distances: the exhaustive declaration is the true one.
        RankFidelity::EXACT,
        CandidateDomains::within([shared_block()]),
    );
    assert_eq!(knn_declaration.exclusion, ExclusionBasis::Membership);
    knn_declaration.exclusion = basis;
    registry.register_ranked(KNN_PF, Arc::new(knn), knn_declaration);

    (registry, text_observations, knn_observations)
}

struct FixtureStatistics {
    cardinalities: BTreeMap<Iri, u64>,
}

impl Statistics for FixtureStatistics {
    fn source(&self) -> &'static str {
        "example-statistics"
    }

    fn revision(&self) -> &'static str {
        "r1"
    }

    fn cardinality(&self, predicate: &Iri) -> Option<u64> {
        self.cardinalities.get(predicate).copied()
    }

    fn selectivity_ppm(&self, _subject: &Iri, _term: &RequestTerm) -> Option<u64> {
        None
    }
}

fn fixture_statistics() -> FixtureStatistics {
    let mut cardinalities = BTreeMap::new();
    cardinalities.insert(iri(TEXT_STRATUM), text_rows().len() as u64);
    cardinalities.insert(iri(NOTE), text_rows().len() as u64);
    cardinalities.insert(iri(KNN_STRATUM), CORPUS as u64);
    FixtureStatistics { cardinalities }
}

fn fixture_profile() -> FusionProfile {
    let weights = [iri(TEXT_STRATUM), iri(KNN_STRATUM)]
        .into_iter()
        .map(|stratum| (stratum, Fixed::ONE))
        .collect();
    FusionProfile::with_decay(weights, DecayRule::ReciprocalRank { k: K })
        .expect("the fixture profile is valid")
}

// ---------------------------------------------------------------------------
// The measurement
// ---------------------------------------------------------------------------

/// One row of a fused answer: the candidate, its fused score, and which stratum
/// contributed what at which rank.
type AnswerRow = (Term, Fixed, Vec<(Iri, u64, Fixed)>);

/// What one run cost and what it answered, every figure read off either the answer's
/// trailer or a relation's own counter.
#[derive(Debug, PartialEq, Eq)]
struct Measured {
    answer: Vec<AnswerRow>,
    /// The depth the plan recorded for the vector stratum — the `k` it is opened at.
    knn_planned_depth: u32,
    text_ranks: u64,
    knn_ranks: u64,
    text_rows_materialised: Option<u64>,
    knn_rows_materialised: Option<u64>,
    fused_lookups: u64,
    text_lookups: u64,
    knn_lookups: u64,
    knn_membership_distances: u64,
    text_rankings: u64,
    /// Scans the exact relation ran — one per ranked invocation.
    knn_scans: u64,
    /// Distances those scans computed — its search work.
    knn_scanned_candidates: u64,
}

fn reduce(row: &FusedRow) -> (Term, Fixed, Vec<(Iri, u64, Fixed)>) {
    (
        row.entity.clone(),
        row.score,
        row.contributions
            .iter()
            .map(|(stratum, rank, contribution)| (stratum.clone(), *rank, *contribution))
            .collect(),
    )
}

fn measure(dataset: &RdfDataset, basis: ExclusionBasis) -> Measured {
    let (registry, text_observations, knn_observations) = registry(dataset, basis);
    let statistics = fixture_statistics();
    let profile = fixture_profile();
    let request = RetrievalRequest::bounded(request_terms(), TOP_K);
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: Some(&profile),
    };
    let planned = plan(&request, &registry, &statistics).expect("the fixture request plans");
    let knn_planned_depth = planned.stratum_depths[&iri(KNN_STRATUM)];
    let result = block_on(search(
        &request,
        &registry,
        &statistics,
        dataset,
        &env,
        &profile,
    ))
    .expect("the fixture request searches");
    let resolution = |stratum: &str| {
        result
            .trailer
            .resolution
            .get(&iri(stratum))
            .expect("the stratum was read")
    };
    Measured {
        answer: result.rows.iter().map(reduce).collect(),
        knn_planned_depth,
        text_ranks: resolution(TEXT_STRATUM).ranks_pulled,
        knn_ranks: resolution(KNN_STRATUM).ranks_pulled,
        text_rows_materialised: resolution(TEXT_STRATUM).rows_materialised,
        knn_rows_materialised: resolution(KNN_STRATUM).rows_materialised,
        fused_lookups: result
            .trailer
            .resolution
            .values()
            .map(|resolution| resolution.exclusion_lookups)
            .sum(),
        text_lookups: text_observations.membership_lookups(),
        knn_lookups: knn_observations.membership_lookups(),
        knn_membership_distances: knn_observations.membership_distances(),
        text_rankings: text_observations.rankings(),
        knn_scans: knn_observations.scans(),
        knn_scanned_candidates: knn_observations.scanned_candidates(),
    }
}

/// **Text and exact kNN, one shared block, disjoint candidates: the same answer out of
/// a strictly shorter read, paid for with lookups on both sides — and one scan of the
/// space either way.**
///
/// The control declares no basis, so nothing is final while either stream is open and
/// both drain: every row of each stratum, read once. The declaring run stops where the
/// fifth row crosses both heads' threshold, at rank 66 of each stratum. Every number
/// is an exact literal, so a drift in any of them is a failure rather than a range
/// quietly absorbing it.
#[test]
fn a_text_and_exact_knn_read_stops_early_on_lookups_and_scans_the_space_once() {
    let dataset = dataset();
    let control = measure(&dataset, ExclusionBasis::Unavailable);
    let asked = measure(&dataset, ExclusionBasis::Membership);
    let report = format!("control: {control:?}\nasked: {asked:?}");

    // 1. The same answer, row for row, score for score, in order — and a full one.
    assert_eq!(asked.answer, control.answer, "{report}");
    assert_eq!(asked.answer.len(), TOP_K.get(), "{report}");

    // 2. The read: strictly shorter, and exactly as deep as the fusion pulled.
    assert_eq!(
        (control.text_ranks, control.knn_ranks),
        (80, 80),
        "the control drains both strata — {report}"
    );
    assert_eq!(
        (asked.text_ranks, asked.knn_ranks),
        (66, 66),
        "the declaring run stops where the fifth row crosses — {report}"
    );
    assert!(
        asked.text_ranks + asked.knn_ranks < control.text_ranks + control.knn_ranks,
        "{report}"
    );
    assert_eq!(
        (asked.text_rows_materialised, asked.knn_rows_materialised),
        (Some(66), Some(66)),
        "each stratum's one read produced the ranks the fusion pulled — {report}"
    );
    assert_eq!(
        (
            control.text_rows_materialised,
            control.knn_rows_materialised
        ),
        (Some(80), Some(80)),
        "every row of each stratum, once — {report}"
    );

    // 3. The producers' work. The vector stratum is opened once, at its planned depth,
    //    in both runs: the planned depth is every row the space holds, the scan
    //    measures every row whatever `k` is, and the declaring run's early stop left
    //    rows unbuilt rather than distances uncomputed. One scan each, of the whole
    //    space — the stop did not shorten the search, and nothing continued it.
    assert_eq!(
        (control.knn_planned_depth, asked.knn_planned_depth),
        (80, 80),
        "{report}"
    );
    assert_eq!(
        (control.text_rankings, control.knn_scans),
        (1, 1),
        "{report}"
    );
    assert_eq!((asked.text_rankings, asked.knn_scans), (1, 1), "{report}");
    assert_eq!(
        (control.knn_scanned_candidates, asked.knn_scanned_candidates),
        (80, 80),
        "one distance per row of the space, in the run that stopped early and in the one \
         that drained — {report}"
    );

    // 4. The lookups: both strata served them, counted by the relations themselves.
    assert_eq!(
        (
            control.fused_lookups,
            control.text_lookups,
            control.knn_lookups
        ),
        (0, 0, 0),
        "the control asks nothing — {report}"
    );
    // The fusion asked 65 candidates of each stratum: it asks only about a candidate
    // whose upper bound still exceeds the threshold, and the rows at rank 66 — where
    // the threshold fell below every open bound and the fusion stopped — no longer
    // could enter the answer, so nothing was waiting on a verdict for them. The text
    // relation counts a
    // lookup per (document, needle term) pair, and the needle is two terms; the exact
    // relation counts one per candidate.
    assert_eq!(
        (asked.fused_lookups, asked.text_lookups, asked.knn_lookups),
        (130, 65 * 2, 65),
        "one lookup per candidate the other stratum named — {report}"
    );
    assert_eq!(
        asked.knn_membership_distances, 0,
        "the space holds no row for a text subject, so no lookup read a vector — {report}"
    );
}

// ---------------------------------------------------------------------------
// The bound-candidate, bound-count call
// ---------------------------------------------------------------------------

/// Run `?n <pf> ( seed OFFER ?d )` with `?n` declared a parameter and bound to `term`,
/// returning the `?d` of each row.
fn bound_candidate(
    engine: &NativeSparqlEngine,
    env: &ExtensionEnv,
    dataset: &RdfDataset,
    pf: &str,
    term: &TermValue,
) -> Vec<TermValue> {
    let options = QueryOptions::new().with_env(env);
    let text = format!(
        "SELECT ?d WHERE {{ ( ?n ) <{pf}> ( <{}> {OFFER} ?d ) }}",
        vector_term(0)
    );
    let mut execution = engine
        .prepare_execution(&text, None, &["n"], options)
        .expect("the bound-candidate call prepares");
    let slot = execution.slot("n").expect("the declared parameter");
    execution.bind(slot, term.clone()).expect("the term binds");
    engine
        .execute(&mut execution, dataset, options, |outcome| match outcome {
            InternedOutcome::Solutions(solutions) => {
                let column = solutions.column("d").expect("?d is projected");
                solutions
                    .rows()
                    .iter()
                    .map(|row| solutions.cell(row, column).expect("?d is bound"))
                    .collect()
            }
            InternedOutcome::Boolean(_) | InternedOutcome::Graph(_) => {
                panic!("a SELECT answers solutions")
            }
        })
        .expect("the bound-candidate call runs")
}

/// The free ranked reading `?n <pf> ( seed OFFER ?d )`, as `(term, distance)` pairs.
fn ranked_reading(
    engine: &NativeSparqlEngine,
    env: &ExtensionEnv,
    dataset: &RdfDataset,
    pf: &str,
) -> BTreeMap<TermValue, TermValue> {
    let options = QueryOptions::new().with_env(env);
    let text = format!(
        "SELECT ?n ?d WHERE {{ ( ?n ) <{pf}> ( <{}> {OFFER} ?d ) }}",
        vector_term(0)
    );
    let prepared = engine
        .prepare_query_with_options(&text, None, options)
        .expect("the ranked call prepares");
    match engine
        .query_prepared_governed_view(
            dataset,
            &prepared,
            &[],
            options,
            &purrdf::sparql::QueryGovernors::UNBOUNDED,
        )
        .expect("the ranked call runs")
    {
        purrdf::sparql::GovernedOutcome::Complete {
            result: purrdf::SparqlResult::Solutions { rows, .. },
            ..
        } => rows
            .into_iter()
            .map(|row| {
                (
                    row[0].clone().expect("?n is bound"),
                    row[1].clone().expect("?d is bound"),
                )
            })
            .collect(),
        other => panic!("expected solutions, got {other:?}"),
    }
}

/// **A call with the candidate and the count both bound answers the ranked question,
/// and its cost is a whole search per candidate — visible, not refused.**
///
/// "Is this term among the `k` nearest" is answered by ranking the neighbourhood and
/// keeping the term if it is in the first `k`. Asked of every term the space holds, the
/// terms that come back — each with its distance — are exactly the free ranked
/// reading's, and every other term comes back empty. What it cost is on the relations'
/// own counters: one full search per bound term, where the free reading paid one
/// search in all. Both producers, the exact scan and the graph beam, are held to it.
#[test]
fn a_bound_candidate_beside_a_bound_count_is_the_ranked_reading_at_a_search_per_candidate() {
    let dataset = RdfDatasetBuilder::new()
        .freeze()
        .expect("an empty default graph is structurally valid");
    let engine = NativeSparqlEngine::new();
    let terms: Vec<TermValue> = (0..CORPUS)
        .map(|at| TermValue::iri(vector_term(at)))
        .collect();

    // The exact scan.
    let knn = EmbeddingKnnRelation::new(Arc::new(knn_space()));
    let knn_observed = knn.observations();
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(KNN_PF.to_owned(), Arc::new(knn));
    let env = ExtensionEnv::over_relations(registry).expect("the declarations read");
    let reading = ranked_reading(&engine, &env, &dataset, KNN_PF);
    assert_eq!(reading.len(), OFFER);
    assert_eq!(
        (knn_observed.scans(), knn_observed.scanned_candidates()),
        (1, 80),
        "the free reading: one scan of the whole space"
    );
    let mut answered = BTreeMap::new();
    for term in &terms {
        let rows = bound_candidate(&engine, &env, &dataset, KNN_PF, term);
        assert!(rows.len() <= 1, "one term is named at most once");
        if let Some(distance) = rows.into_iter().next() {
            answered.insert(term.clone(), distance);
        }
    }
    assert_eq!(
        answered, reading,
        "the bound-candidate answers are the ranked reading, distance for distance"
    );
    assert_eq!(
        (
            knn_observed.scans(),
            knn_observed.scanned_candidates(),
            knn_observed.membership_lookups()
        ),
        (81, 81 * 80, 0),
        "one whole scan per bound term, and not one membership lookup: the count was bound"
    );

    // The graph beam.
    let hnsw = HnswRelation::new(hnsw_space());
    let hnsw_observed = hnsw.observations();
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(HNSW_PF.to_owned(), Arc::new(hnsw));
    let env = ExtensionEnv::over_relations(registry).expect("the declarations read");
    let reading = ranked_reading(&engine, &env, &dataset, HNSW_PF);
    assert_eq!(reading.len(), OFFER);
    let beam = hnsw_observed.graph_candidates();
    assert_eq!(hnsw_observed.searches(), 1);
    assert!(beam > 0, "the free reading walked the graph");
    let mut answered = BTreeMap::new();
    for term in &terms {
        let rows = bound_candidate(&engine, &env, &dataset, HNSW_PF, term);
        assert!(rows.len() <= 1, "one term is named at most once");
        if let Some(distance) = rows.into_iter().next() {
            answered.insert(term.clone(), distance);
        }
    }
    assert_eq!(answered, reading, "the same, out of the graph beam");
    assert_eq!(
        (
            hnsw_observed.searches(),
            hnsw_observed.graph_candidates(),
            hnsw_observed.membership_lookups()
        ),
        (81, 81 * beam, 0),
        "one whole beam per bound term, each the free reading's own beam"
    );
}
