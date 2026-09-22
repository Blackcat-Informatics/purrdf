// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The headline, against the real producers: one request, two shipped
//! relations, one fused answer.
//!
//! Every other test in this crate drives a mock, which proves the layer is
//! self-consistent and proves nothing about whether a real producer can be
//! wired into it. This one builds a real [`TextIndex`] over real documents and
//! a real [`EmbeddingSpace`] over a real, sealed, fully verified PURREMB
//! artifact; registers `TextSearchRelation` and `EmbeddingKnnRelation` through
//! [`PropertyFunctionRegistry::register_ranked`] with declarations the relations
//! hand out themselves; and runs the public [`search`] entry point over a
//! non-empty dataset. The rows it asserts on could not have come from anywhere
//! but that data.
//!
//! Nothing is minted here. The predicate, the two producer IRIs, the two strata
//! and the depth datatype are all `example.org` fixtures supplied by this test
//! in the host's role, exactly as a caller would supply its own.
//!
//! The last section is about a different kind of claim. Both shipped relations
//! declare `DuplicatePolicy::Unique`, which the fusion layer spends rather than
//! checks, so it is a promise about the producer's own index that only the
//! producer can keep. Those tests drive each one over data shaped to break it.
//!
//! Geometry is deliberately absent. `purrdf-geo`'s relation computes a set — it
//! sorts and deduplicates its pairs and carries neither a score nor a rank — so
//! it composes as a constraint on candidates rather than as a stratum of a fused
//! ranking, and inventing a stratum for it would be inventing a ranking it never
//! claimed.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll, Wake, Waker};

use pretty_assertions::assert_eq;
use purrdf_core::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, CanonicalMetadataInput,
    CertifiedPurrpckSource, ContentDigest, DimensionalityPolicy, DistanceMetric, EmbeddingBuilder,
    EmbeddingFamilyContract, MatrixInput, MatrixRow, PrefixPostprocessing, ProjectionSpec,
    RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfTermTarget, StageImplementation, TargetId,
    TargetSet, TargetSetId, TermValue, VectorDtype, VectorSpaceId,
};
use purrdf_retrieval::{
    AdmissionEnvironment, Completeness, DecayRule, Fixed, FusionError, FusionProfile, Iri,
    OrderFidelity, ProtocolError, RankFidelity, RankedStreamAdapter, ReadAttempts, RequestTerm,
    RetrievalRequest, ScoreExactness, SearchError, SearchResult, Statistics, Term, TopK, compile,
    contribution, execute, fuse, plan, search,
};
use purrdf_sparql_eval::{
    BindingPattern, CandidateDomains, DuplicatePolicy, EmbeddingKnnRelation, EmbeddingSpace,
    EvalError, KnnGuard, PfArgs, PfArity, PfCursor, PropertyFunction, PropertyFunctionRegistry,
    TermKind, Volatility,
};
use purrdf_text::{GraphSelector, TextError, TextIndex, TextIndexConfig, TextSearchRelation};

// ---------------------------------------------------------------------------
// The host's vocabulary. Every IRI below is the caller's; PurRDF mints none.
// ---------------------------------------------------------------------------

/// The one predicate the fixture corpus is indexed over.
const NOTE: &str = "https://example.org/note";
/// The IRI this host registers the text relation under.
const TEXT_PF: &str = "https://example.org/pf/search";
/// The IRI this host registers the kNN relation under.
const KNN_PF: &str = "https://example.org/pf/neighbours";
/// The stratum this host ranks lexical rows within.
const TEXT_STRATUM: &str = "https://example.org/stratum/lexical";
/// The stratum this host ranks nearest-neighbour rows within.
const KNN_STRATUM: &str = "https://example.org/stratum/neighbour";
/// The datatype this host renders a neighbour count with.
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
/// The reciprocal-rank smoothing constant this host fuses under.
const K: u32 = 60;
/// The row bound this host asks for. Fused enumeration is top-k by
/// construction, so the bound is stated rather than defaulted; every fixture in
/// this file holds a handful of rows, all well below this, so nothing here is
/// decided by it.
const TOP_K: TopK = TopK::new(16);

fn ex(local: &str) -> String {
    format!("https://example.org/{local}")
}

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

fn kernel_iri(text: &str) -> purrdf_core::Iri {
    purrdf_core::parse_iri(text).expect("fixture IRIs are valid")
}

// ---------------------------------------------------------------------------
// The data: four documents, four vectors, one dataset
// ---------------------------------------------------------------------------

/// The corpus, as `(subject local name, text, vector)`.
///
/// The text half is this workspace's hand-computed BM25 golden: four untagged
/// documents of four tokens each, so `avgdl` is exactly four. The needle
/// `"alpha beta"` matches `ex:a` (rank 1) and `ex:b` (rank 2) and nothing else.
///
/// The vector half is squared-Euclidean and equally hand-checkable from the
/// seed `ex:a` at the origin: `b` at 25, `d` at 100, `c` at 2500.
///
/// The two halves deliberately disagree about `ex:c` and `ex:d`, which is what
/// makes "both producers contributed" an observation rather than a coincidence.
fn corpus() -> Vec<(&'static str, &'static str, Vec<f64>)> {
    vec![
        ("a", "alpha alpha beta gamma", vec![0.0, 0.0]),
        ("b", "alpha beta gamma delta", vec![3.0, 4.0]),
        ("c", "epsilon zeta eta theta", vec![30.0, 40.0]),
        ("d", "iota kappa lambda mu", vec![6.0, 8.0]),
    ]
}

/// The dataset every stage runs against: one `note` triple per document, in the
/// default graph. Non-empty, and the very rows the index is built from.
fn dataset() -> Arc<RdfDataset> {
    let rows: Vec<(&str, &str, Option<&str>)> = corpus()
        .into_iter()
        .map(|(local, text, _)| (local, text, None))
        .collect();
    dataset_of(&rows)
}

/// The same shape over an arbitrary `(subject local name, text, language)`
/// list, in the default graph.
///
/// The language is a parameter because a partition of a [`TextIndex`] is keyed
/// by `(graph, language)`, so a language is the cheapest way for a fixture to
/// ask for more than one partition — which is the condition the text producer
/// refuses to declare a ranked order over.
fn dataset_of(rows: &[(&str, &str, Option<&str>)]) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let note = builder.intern_iri(NOTE);
    for &(local, text, language) in rows {
        let subject = builder.intern_iri(&ex(local));
        let literal = builder.intern_literal(language.map_or_else(
            || RdfLiteral::simple(text),
            |tag| RdfLiteral::language_tagged(text, tag),
        ));
        builder.push_quad(subject, note, literal, None);
    }
    builder.freeze().expect("the fixture dataset is valid")
}

/// The index configuration: one predicate, every graph. The corpus is untagged
/// and lives in the default graph, so this yields exactly one partition — which
/// is the condition `TextSearchRelation::ranked_declaration` requires before it
/// will claim a ranked order.
fn text_config() -> TextIndexConfig {
    TextIndexConfig::new(vec![TermValue::iri(NOTE)], GraphSelector::Any)
        .expect("the fixture configuration is well formed")
}

/// A real index over the real dataset.
fn text_index() -> TextIndex {
    TextIndex::from_dataset(&*dataset(), &text_config()).expect("the fixture index builds")
}

// ---------------------------------------------------------------------------
// The PURREMB artifact, encoded and sealed by the kernel's own writer
// ---------------------------------------------------------------------------

/// A fixture artifact identity, distinct per `name`.
fn identity(name: &str) -> ArtifactIdentity {
    ArtifactIdentity::new(
        ex(name),
        "application/octet-stream",
        ContentDigest::of(name.as_bytes()),
        None,
        ArtifactIdentityKind::Single,
    )
    .expect("the fixture artifact identity is well formed")
}

/// A fixture applied stage, distinct per `name`.
fn stage(name: &str) -> AppliedStage {
    AppliedStage::Applied(
        StageImplementation::new(
            ex(name),
            ContentDigest::of(name.as_bytes()),
            "application/octet-stream",
            vec![1],
        )
        .expect("the fixture stage is well formed"),
    )
}

/// The corpus's vector half alone, as `(subject local name, vector)`.
fn vector_rows() -> Vec<(&'static str, Vec<f64>)> {
    corpus()
        .into_iter()
        .map(|(local, _, vector)| (local, vector))
        .collect()
}

/// Encode a sealed PURREMB artifact holding one `f64` row per given row, under
/// the squared-Euclidean metric.
///
/// The rows are a parameter rather than [`vector_rows`] directly, so a fixture
/// can choose the vectors as well as the terms that name them.
fn artifact_over(
    rows: &[(&str, Vec<f64>)],
) -> (
    Vec<u8>,
    TargetSetId,
    VectorSpaceId,
    Vec<(TargetId, TermValue)>,
) {
    let dimension = u32::try_from(rows[0].1.len()).expect("the fixture dimension is small");
    let empty = RdfDatasetBuilder::new().freeze().expect("empty dataset");
    let (source, _) = CertifiedPurrpckSource::from_dataset(&empty).expect("source pack");

    let mut targets = Vec::with_capacity(rows.len());
    let mut bindings = Vec::with_capacity(rows.len());
    for (local, _) in rows {
        let term = TermValue::iri(ex(local));
        let TermValue::Iri(text) = &term else {
            unreachable!("the fixture terms are IRIs")
        };
        let target = RdfTermTarget::Iri(text.clone())
            .into_target(true, None)
            .expect("the fixture term target is well formed");
        bindings.push((target.id, term));
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
            .iter()
            .zip(&bindings)
            .map(|((_, values), (target, _))| MatrixRow::new(*target, values.clone()))
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
    (encoded.bytes, set.id, vector_space, bindings)
}

/// A real, fully verified embedding space over the artifact above.
fn embedding_space() -> EmbeddingSpace {
    space_over(
        &vector_rows(),
        KnnGuard::new(10, 5).expect("the fixture guard bounds are positive"),
    )
}

/// [`embedding_space`] over an arbitrary row list and guard.
///
/// The guard is a parameter because it is what bounds `k`, and `k` is the
/// per-stratum depth a plan derives from the producer's own declared row bound:
/// a fixture that wants the space read to its last row needs a guard that
/// admits as many neighbours as the space holds.
fn space_over(rows: &[(&str, Vec<f64>)], guard: KnnGuard) -> EmbeddingSpace {
    let (bytes, target_set, vector_space, bindings) = artifact_over(rows);
    EmbeddingSpace::from_artifact(&bytes, target_set, vector_space, bindings, guard)
        .expect("the fixture space opens")
}

// ---------------------------------------------------------------------------
// The wiring: what a host actually writes
// ---------------------------------------------------------------------------

/// Register both real relations as ranked producers, using the declaration each
/// relation hands out for a caller-supplied stratum.
fn registry() -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();

    let text = TextSearchRelation::new(Arc::new(text_index()));
    let text_declaration = text
        .ranked_declaration(
            kernel_iri(TEXT_STRATUM),
            Some(NOTE.to_owned()),
            RankFidelity::EXACT,
            // This fixture's notes and its embedded entities are the SAME
            // entities — the whole point of the file is a candidate both real
            // producers name — so neither restricts its domain, and the fusion
            // below certifies with no licence to skip anything.
            CandidateDomains::Unrestricted,
        )
        .expect("a single-partition index declares a ranked order");
    registry.register_ranked(TEXT_PF, Arc::new(text), text_declaration);

    let knn = EmbeddingKnnRelation::new(Arc::new(embedding_space()));
    let knn_declaration = knn.ranked_declaration(
        kernel_iri(KNN_STRATUM),
        // The space's rows are IRIs, and an IRI seed is what this host's
        // requests name.
        TermKind::Iri,
        XSD_INTEGER.to_owned(),
        // This space holds a vector for every document the fixture corpus has,
        // so the exhaustive declaration is the true one — and it is stated here
        // rather than asserted by the relation.
        RankFidelity::EXACT,
        // As above: one entity space, ranked twice under two laws.
        CandidateDomains::Unrestricted,
    );
    registry.register_ranked(KNN_PF, Arc::new(knn), knn_declaration);

    registry
}

/// The request: one lexical term for the text producer and one entity seed for
/// the nearest-neighbour producer.
///
/// The seed is what kNN accepts — it searches *from* a term whose vector the
/// space already holds — and no producer here accepts a raw embedding.
fn request() -> RetrievalRequest {
    RetrievalRequest::bounded(
        vec![
            RequestTerm::Lexical {
                text: "alpha beta".to_owned(),
                language: None,
                predicate: Some(iri(NOTE)),
            },
            RequestTerm::EntitySeed {
                entity: Term::new(format!("<{}>", ex("a"))),
            },
        ],
        TOP_K,
    )
}

/// A statistics provider that reports nothing.
///
/// Both producers declare a finite row bound measured from their own frozen
/// data, so there is no unbounded declaration for a cardinality to bound. A
/// provider that invented one would be putting a number into the plan that no
/// measurement supports.
struct NoStatistics;

impl Statistics for NoStatistics {
    fn source(&self) -> &'static str {
        "example-statistics"
    }

    fn revision(&self) -> &'static str {
        "r1"
    }

    fn cardinality(&self, _predicate: &Iri) -> Option<u64> {
        None
    }

    fn selectivity_ppm(&self, _subject: &Iri, _term: &RequestTerm) -> Option<u64> {
        None
    }
}

/// Unit weights for both strata and `K` smoothing. Two weighted strata is what
/// gives a candidate room to surface in both: the contribution maximum is the
/// stratum count, not a separate parameter.
fn profile() -> FusionProfile {
    let mut weights = BTreeMap::new();
    weights.insert(iri(TEXT_STRATUM), Fixed::ONE);
    weights.insert(iri(KNN_STRATUM), Fixed::ONE);
    FusionProfile::with_decay(weights, DecayRule::ReciprocalRank { k: K })
        .expect("the fixture profile is valid")
}

// ---------------------------------------------------------------------------
// A minimal single-threaded executor
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
// Reading the answer
// ---------------------------------------------------------------------------

/// The fused rows as `(candidate, [(stratum, rank)])`, in final order.
fn ranking(result: &SearchResult) -> Vec<(String, Vec<(String, u64)>)> {
    result
        .rows
        .iter()
        .map(|row| {
            (
                row.entity.as_str().to_owned(),
                row.contributions
                    .iter()
                    .map(|(stratum, rank, _)| (stratum.as_str().to_owned(), *rank))
                    .collect(),
            )
        })
        .collect()
}

/// The strata one candidate drew a contribution from.
fn strata_of(result: &SearchResult, candidate: &str) -> Vec<String> {
    result
        .rows
        .iter()
        .find(|row| row.entity.as_str() == candidate)
        .map(|row| {
            row.contributions
                .iter()
                .map(|(stratum, _, _)| stratum.as_str().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// 1. The demonstration
// ---------------------------------------------------------------------------

#[test]
fn two_real_producers_fuse_into_one_ranking_over_real_data() {
    let registry = registry();
    let statistics = NoStatistics;
    let data = dataset();
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: None,
    };
    let profile = profile();
    let request = request();

    let result = block_on(search(
        &request,
        &registry,
        &statistics,
        &*data,
        &env,
        &profile,
    ))
    .expect("the real producers answer");

    // Every row names a subject of the fixture corpus and nothing else: these
    // terms exist only because the index and the space hold them.
    assert_eq!(
        ranking(&result),
        vec![
            (
                format!("<{}>", ex("a")),
                vec![(TEXT_STRATUM.to_owned(), 1), (KNN_STRATUM.to_owned(), 1)],
            ),
            (
                format!("<{}>", ex("b")),
                vec![(TEXT_STRATUM.to_owned(), 2), (KNN_STRATUM.to_owned(), 2)],
            ),
            (format!("<{}>", ex("d")), vec![(KNN_STRATUM.to_owned(), 3)]),
            (format!("<{}>", ex("c")), vec![(KNN_STRATUM.to_owned(), 4)]),
        ],
        "ex:a and ex:b are ranked by BOTH producers; ex:d and ex:c are near ex:a in the \
         embedding space but hold none of the needle's terms, so only kNN reaches them"
    );

    // Both producers contributed, and the claim is checked per producer rather
    // than by counting rows.
    assert_eq!(
        strata_of(&result, &format!("<{}>", ex("a"))),
        vec![TEXT_STRATUM.to_owned(), KNN_STRATUM.to_owned()],
        "the top candidate carries per-stratum provenance from both producers"
    );
    assert!(
        result.rows.iter().any(|row| row.contributions.len() == 1),
        "and a candidate only one producer reached carries only that producer"
    );

    // The ordering is the profile's, and every score is exactly its provenance.
    for pair in result.rows.windows(2) {
        assert!(
            pair[0].score >= pair[1].score,
            "rows are ordered score-descending"
        );
    }
    for row in &result.rows {
        let sum = row
            .contributions
            .iter()
            .fold(Fixed::ZERO, |total, (_, _, value)| {
                total.checked_add(*value).expect("no fixed-point overflow")
            });
        assert_eq!(sum, row.score, "contributions sum to the fused score");
    }

    // The exact reciprocal-rank arithmetic, so the numbers are pinned and not
    // merely ordered: unit weight over `K + rank`, summed.
    let one_at_rank_one =
        contribution(Fixed::ONE, 1, K).expect("a unit weight at rank one is in range");
    let expected_top = one_at_rank_one
        .checked_add(one_at_rank_one)
        .expect("two rank-one contributions sum within the fixed-point range");
    assert_eq!(
        result.rows[0].score, expected_top,
        "ex:a's score is 1/(K+1) from each of the two producers"
    );

    // Both strata reported an ending, and the two endings are deliberately
    // different because the two producers are bounded differently.
    //
    // The text relation is bounded by the unit's own `LIMIT`, which is emitted one
    // row past the planned depth, so the empty probe slot VERIFIES that the two
    // documents holding the needle's terms were all there were: `Exhausted`.
    //
    // The nearest-neighbour relation takes its depth as an argument and this
    // fixture's guard puts the declared bound (four rows) exactly at the depth, so
    // the relation was asked for four, returned four, and a fifth could not have been
    // requested — asking for it would ask the relation to breach the guard it
    // registered. This fixture's space does hold exactly four rows, but the read
    // could not see that, and reporting `Exhausted` here would be a completeness
    // claim minted from the declaration rather than from the read. So it names the
    // stopper it had: the row bound the producer itself declared.
    assert_eq!(result.trailer.statuses.len(), 2);
    assert!(
        matches!(
            result.trailer.statuses.get(&iri(TEXT_STRATUM)),
            Some(purrdf_retrieval::ProducerStatus::Exhausted { rows_emitted: 2 })
        ),
        "got {:?}",
        result.trailer.statuses.get(&iri(TEXT_STRATUM))
    );
    assert!(
        matches!(
            result.trailer.statuses.get(&iri(KNN_STRATUM)),
            Some(purrdf_retrieval::ProducerStatus::RowBoundReached { rank: 4 })
        ),
        "got {:?}",
        result.trailer.statuses.get(&iri(KNN_STRATUM))
    );

    // The identity in the answer came up the pipeline with the rows: both real
    // streams were tagged with the plan their unit was compiled from, and the
    // trailer names it because they still agreed at the fusion.
    let planned = plan(&request, &registry, &statistics).expect("plans");
    assert_eq!(result.trailer.plan_id, Some(planned.id()));
    assert_eq!(result.plan_id, planned.id());
    assert_eq!(result.profile_id, profile.id());
}

// ---------------------------------------------------------------------------
// 2. The request is in the text, and the text is what runs
// ---------------------------------------------------------------------------

#[test]
fn each_real_producer_is_compiled_with_the_facet_it_declared() {
    let registry = registry();
    let statistics = NoStatistics;
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: None,
    };
    let planned = plan(&request(), &registry, &statistics).expect("the request plans");
    let compiled = compile(&planned, &env).expect("a fresh plan is admitted");

    let text = compiled
        .units
        .iter()
        .find(|unit| unit.stratum == iri(TEXT_STRATUM))
        .expect("the lexical stratum emits a unit");
    assert_eq!(
        text.sparql(),
        format!(
            "SELECT ?candidate WHERE {{\n  \
             {{ SELECT (?c0 AS ?candidate) WHERE {{ ( ?c0 ) <{TEXT_PF}> \
             ( \"alpha beta\" ?c2 ?c3 ?c4 ?c5 ) }} LIMIT 5 }}\n\
             }}\nLIMIT 5"
        ),
        "the needle is a rendered constant at the relation's own needle position, \
         every other position is free, and the relation takes no depth argument so the \
         branch carries the stratum's LIMIT — five over a depth of four, because the \
         probe row is emitted at every depth including one that sits on the declaration"
    );
    assert_eq!(
        text.depth(),
        4,
        "and the recorded depth is four: only the emitted bound carries the probe"
    );

    let knn = compiled
        .units
        .iter()
        .find(|unit| unit.stratum == iri(KNN_STRATUM))
        .expect("the neighbour stratum emits a unit");
    assert_eq!(
        knn.sparql(),
        format!(
            "SELECT ?candidate WHERE {{\n  \
             {{ SELECT (?c0 AS ?candidate) WHERE {{ ( ?c0 ) <{KNN_PF}> \
             ( <{seed}> \"4\"^^<{XSD_INTEGER}> ?c3 ) }} }}\n\
             }}\nLIMIT 5",
            seed = ex("a")
        ),
        "the seed is a rendered constant and the stratum's depth IS the neighbour \
         count, so this branch bounds itself and carries no LIMIT of its own"
    );
    assert_eq!(
        knn.declared_rows(),
        Some(4),
        "this producer's declaration is its guard, and the depth sits on it"
    );
}

/// The depth **argument** stops at the declaration, the emitted `LIMIT` does not,
/// and the read's own ending says which of the two stopped it.
///
/// The two numbers differ for this producer and only at this depth, and the
/// difference is the point: a `LIMIT` is a ceiling the evaluator applies to a
/// cursor, while the neighbour count is a request the relation reads and checks
/// against its own configured guard. Asking for five neighbours from a guard that
/// admits four is refused by the relation — correctly, since serving it would be a
/// short answer returned as a complete one — so the probe row is bought on the
/// `LIMIT`, where it costs the producer nothing, and never on the argument.
///
/// Without this split the probe would have turned a valid query into a refused
/// one, which is the mirror of the silent truncation it exists to prevent.
///
/// The consequence is then executed rather than described, because the split leaves
/// a read this layer cannot see the end of. The relation is asked for exactly the
/// four rows it declared, returns four, and no fifth can be requested — so the
/// stratum reports the bound that stopped it and not an exhaustion nobody verified.
/// This is not a property of THIS fixture's guard: `rows_per_invocation` is
/// `min(max_neighbours, rows)` and the planner takes that declaration for the depth,
/// so for the shipped nearest-neighbour relation under a statistics provider that
/// measures nothing, the depth always lands on the declaration and this is always
/// the ending. The wrong-declaration case a mock CAN reach — a self-bounding
/// producer that returns more rows than it registered, still caught by the unit's
/// own bound — is
/// `a_self_bounding_producer_reports_the_bound_that_stopped_it_and_still_catches_a_wrong_one`
/// in `tests/compile_request.rs`.
#[test]
fn the_neighbour_count_stays_inside_the_guard_and_the_ending_says_which_bound_stopped_it() {
    let registry = registry();
    let statistics = NoStatistics;
    let data = dataset();
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: None,
    };
    let planned = plan(&request(), &registry, &statistics).expect("the request plans");
    let compiled = compile(&planned, &env).expect("a fresh plan is admitted");
    let knn = compiled
        .units
        .iter()
        .find(|unit| unit.stratum == iri(KNN_STRATUM))
        .expect("the neighbour stratum emits a unit");

    assert!(
        knn.sparql().contains(&format!("\"4\"^^<{XSD_INTEGER}>")),
        "the relation is asked for the four neighbours it declared it can serve, \
         never the five that would breach its guard: {}",
        knn.sparql()
    );
    assert!(
        knn.sparql().ends_with("LIMIT 5"),
        "while the unit's own bound still reaches one row past the declaration, so a \
         relation that returned five would still be caught: {}",
        knn.sparql()
    );
    assert_eq!(
        (knn.depth(), knn.declared_rows()),
        (4, Some(4)),
        "the depth sits ON the declaration, which is the state that has no probe"
    );

    // And the ending it actually has. The four rows the relation returned are every
    // row it was allowed to return, so `Exhausted` would be a completeness claim
    // minted from the guard rather than read off the data, and `DepthReached` would
    // blame a planned depth that cut nothing.
    let execution = block_on(execute(&compiled, &registry, &data)).expect("both relations run");
    assert_eq!(
        execution.statuses.get(&iri(KNN_STRATUM)),
        Some(&purrdf_retrieval::ProducerStatus::RowBoundReached { rank: 4 }),
        "the producer's own declared bound is what stopped this read"
    );
    // The neighbour in the same bundle, so the ending is not simply what this
    // executor writes for everything: the text relation is bounded by the unit's
    // `LIMIT`, its probe slot came back empty, and its exhaustion is verified.
    assert_eq!(
        execution.statuses.get(&iri(TEXT_STRATUM)),
        Some(&purrdf_retrieval::ProducerStatus::Exhausted { rows_emitted: 2 }),
        "a producer the evaluator bounds still reports a verified exhaustion"
    );
}

// ---------------------------------------------------------------------------
// 3. The composition identity, on the real path
// ---------------------------------------------------------------------------

/// Drive `fuse ∘ execute ∘ compile ∘ plan` by hand over the real producers.
///
/// The bridge from the executor's `(rank, candidate, block)` rows to the fusion
/// protocol is the crate's own exported [`RankedStreamAdapter`], and the
/// executor's statuses reach the trailer through the exported
/// `FusionTrailer::completed_with`. That is what makes the identity below worth
/// asserting: the pipeline this assembles is the one a caller can write, not a
/// second copy of `search`'s insides that would have to be kept in step by hand.
// The manual composition mirrors `search`, including its single-task,
// runtime-agnostic future; see the same allow on `search` itself.
#[allow(clippy::future_not_send)]
async fn manual_composition(
    registry: &PropertyFunctionRegistry,
    statistics: &NoStatistics,
    data: &RdfDataset,
    env: &AdmissionEnvironment<'_>,
    profile: &FusionProfile,
) -> SearchResult {
    let planned = plan(&request(), registry, statistics).expect("the request plans");
    // The waist is re-formed around the law this composition will fuse under,
    // exactly as `search` re-forms the environment it is handed: the planner
    // still never sees a profile, and admission gains the one thing it can only
    // know here — what each planned depth costs in rank resolution.
    let env = AdmissionEnvironment {
        registry: env.registry,
        statistics: env.statistics,
        fusion_profile: Some(profile),
    };
    let compiled = compile(&planned, &env).expect("a fresh plan is admitted");
    let execution = execute(&compiled, registry, data)
        .await
        .expect("both real relations run");

    let mut streams = Vec::new();
    let mut unweighted_strata = Vec::new();
    for stream in execution.streams {
        let plan_id = stream.plan_id;
        let fused_bound = stream.fused_bound;
        let attestation = stream.attestation.clone();
        match RankedStreamAdapter::new(stream.stream, stream.contract, profile, &stream.stratum) {
            // The plan the unit was compiled from travels on with the rows; the
            // trailer names it, and the answer's identity is read back from
            // there rather than asked of the plan a second time. What the index
            // behind those rows attested rides the same way, and it is what the
            // trailer's exactness and evidence identity are derived from — a
            // composition that dropped it would publish a lower bound as an
            // exact score.
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

    let fused = fuse::<RankedStreamAdapter, Term>(streams, profile, compiled.fused_bound)
        .await
        .expect("the surviving streams fuse");
    let trailer = fused.trailer.completed_with(execution.statuses);
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
        // One read, at the depths the plan recorded, because that is the read a
        // caller composing `execute` by hand takes. `search` attempts a narrower
        // one first and keeps it only when it certified, so a run that agrees with
        // this composition on everything else must agree here too: either the
        // narrowed read was the whole answer, or it was discarded and this very
        // read replaced it.
        read_attempts: ReadAttempts::Once,
    }
}

#[test]
fn search_equals_the_hand_composed_pipeline_over_the_real_producers() {
    let registry = registry();
    let statistics = NoStatistics;
    let data = dataset();
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: None,
    };
    let profile = profile();

    let direct = block_on(search(
        &request(),
        &registry,
        &statistics,
        &*data,
        &env,
        &profile,
    ))
    .expect("the composed search answers");
    let manual = block_on(manual_composition(
        &registry,
        &statistics,
        &data,
        &env,
        &profile,
    ));

    // The evidence a caller acts on, named field by field before the whole-value
    // comparison. `assert_eq!` on two `SearchResult`s already covers these, but
    // a regression that dropped the attestation on one path would show up as an
    // opaque struct diff; naming them says which claim broke — and these three
    // are the ones that decide whether a score may be read as a number.
    assert_eq!(
        direct.trailer.attestations, manual.trailer.attestations,
        "what each real index attested must reach both paths identically"
    );
    assert_eq!(
        direct.trailer.exactness, manual.trailer.exactness,
        "and therefore so must whether the fused scores are exact"
    );
    assert_eq!(
        direct.evidence_id, manual.evidence_id,
        "and the digest of that evidence, which is what makes two answers \
         comparable at all"
    );
    assert_eq!(
        direct.evidence_id, direct.trailer.evidence_id,
        "the answer's evidence identity is the trailer's own, never a second \
         derivation of it"
    );

    assert_eq!(
        direct, manual,
        "search is exactly fuse ∘ execute ∘ compile ∘ plan, on the real path too"
    );
}

// ---------------------------------------------------------------------------
// 4. The evidence: what the two real indexes attested
// ---------------------------------------------------------------------------

/// Run the fixture request over a freshly built registry and return the answer.
///
/// Fresh each time on purpose. Two runs that shared one registry would share
/// one `TextIndex` and one `EmbeddingSpace` object, and an equal generation
/// across them could be explained by object identity rather than by content.
/// Rebuilding both from the same rows is what makes the comparison below a
/// statement about the data.
fn answer_of_a_fresh_build() -> SearchResult {
    let registry = registry();
    let statistics = NoStatistics;
    let data = dataset();
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: None,
    };
    block_on(search(
        &request(),
        &registry,
        &statistics,
        &*data,
        &env,
        &profile(),
    ))
    .expect("the real producers answer")
}

/// The generation attested for `stratum`, or a panic naming what was attested
/// instead.
fn attested_generation(result: &SearchResult, stratum: &str) -> String {
    let attestation = result
        .trailer
        .attestations
        .get(&iri(stratum))
        .unwrap_or_else(|| panic!("{stratum} ran, so it must have an attestation"));
    match &attestation.generation {
        purrdf_retrieval::IndexGeneration::Declared(value) => value.to_string(),
        purrdf_retrieval::IndexGeneration::Undeclared => panic!(
            "{stratum} is served by a shipped producer over a content-addressable index, \
             so it must declare the generation that answered rather than stay silent"
        ),
    }
}

/// T8.3 — the generation both shipped producers attest reaches the fused
/// answer, and is the digest of the index that actually answered.
///
/// The whole chain is under test here and nowhere else: a cursor declares, the
/// evaluator reads the declaration immediately after `open`, the executor
/// carries it out of the per-stratum run, the fusion pins it into the trailer
/// before pulling a row, and the `EvidenceId` is its content identity. A break
/// anywhere along it shows up as an `Undeclared` in the trailer or as a
/// generation that does not equal the index's own fingerprint.
#[test]
fn both_real_producers_attest_the_generation_of_the_index_that_answered() {
    let result = answer_of_a_fresh_build();

    assert_eq!(
        result.trailer.attestations.len(),
        2,
        "both strata opened an index, so both are keyed"
    );

    let lexical = attested_generation(&result, TEXT_STRATUM);
    let neighbour = attested_generation(&result, KNN_STRATUM);

    for (stratum, generation) in [(TEXT_STRATUM, &lexical), (KNN_STRATUM, &neighbour)] {
        assert_eq!(generation.len(), 64, "{stratum}: a 32-byte digest in hex");
        assert!(
            generation
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "{stratum}: rendered in lowercase hex, got {generation}"
        );
    }
    assert_ne!(
        lexical, neighbour,
        "two different indexes answered, and the attestations distinguish them"
    );

    // Not merely non-empty: each one is the digest of the index it came from,
    // recomputed here from the same fixture data through the producers' own
    // public surfaces. A generation that were a constant, a counter or a hash of
    // the wrong thing would pass every assertion above and fail these two.
    assert_eq!(
        lexical,
        purrdf_core::hex::lower(&text_index().fingerprint()),
        "the lexical stratum attests the text index's own content fingerprint"
    );
    assert_eq!(
        neighbour,
        embedding_space().generation(),
        "the neighbour stratum attests the embedding space's own generation"
    );

    // No producer said its index was short, and `None` there is silence rather
    // than a certificate — so the claim tested is only that neither declared
    // incompleteness.
    for (stratum, attestation) in &result.trailer.attestations {
        assert_eq!(
            attestation.service,
            purrdf_retrieval::ServiceLevel::Undeclared,
            "{stratum} served from whole fixture data and declared no shortfall"
        );
    }
}

/// T8.3 — and the evidence id built from those attestations is stable across
/// two independent runs over the same data.
///
/// This is the property a host actually consumes: `plan_id` pins the question
/// and `profile_id` pins the law, and neither of them moves when an index is
/// rebuilt. `evidence_id` is the only one that can, so it is worth nothing
/// unless two answers produced against the same index state agree on it.
#[test]
fn two_runs_over_the_same_index_state_carry_one_evidence_id() {
    let first = answer_of_a_fresh_build();
    let second = answer_of_a_fresh_build();

    assert_eq!(
        first.trailer.attestations, second.trailer.attestations,
        "the same indexes rebuilt from the same rows attest the same thing twice"
    );
    assert_eq!(
        first.evidence_id, second.evidence_id,
        "so the two answers were produced against the same evidence and say so"
    );
    assert_eq!(
        first.evidence_id, first.trailer.evidence_id,
        "the answer's evidence id is read off the trailer, never recomputed beside it"
    );

    // And it is its own identity rather than a restatement of the other two: a
    // reader compares the triple.
    assert_ne!(first.evidence_id.to_hex(), first.plan_id.to_hex());
    assert_ne!(first.evidence_id.to_hex(), first.profile_id.to_hex());
}

// ---------------------------------------------------------------------------
// 5. The uniqueness promise both shipped producers declare
// ---------------------------------------------------------------------------
//
// Both shipped ranked producers register with `DuplicatePolicy::Unique`. That
// declaration is not a description of the answer, it is a promise the fusion
// layer spends: a stream that declares it is not charged for a per-row identity
// set, and the whole saving the declaration buys is that the layer believes it
// without paying to check every row. So the promise is only as good as the
// producer, and until something drives the real producers over data built to
// break it, "Unique" is a comment.
//
// The tests below drive them over exactly that data.

/// One weighted stratum and `K` smoothing.
///
/// The fixtures below each exercise a single producer, so the profile names a
/// single stratum: a weightless stratum is dropped before fusion, and a fusion
/// with no weighted stream would assert nothing about the producer's rows.
fn one_stratum_profile(stratum: &str) -> FusionProfile {
    let mut weights = BTreeMap::new();
    weights.insert(iri(stratum), Fixed::ONE);
    FusionProfile::with_decay(weights, DecayRule::ReciprocalRank { k: K })
        .expect("the fixture profile is valid")
}

/// Register the text relation alone, under the declaration it hands out itself.
///
/// The declaration is the relation's own rather than one written here, which is
/// what makes the assertions below statements about the shipped producer: a
/// hand-written `Unique` would only prove that this test can write the word.
fn text_only_registry(index: TextIndex) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    let text = TextSearchRelation::new(Arc::new(index));
    let declaration = text
        .ranked_declaration(
            kernel_iri(TEXT_STRATUM),
            Some(NOTE.to_owned()),
            RankFidelity::EXACT,
            CandidateDomains::Unrestricted,
        )
        .expect("a single-partition index declares a ranked order");
    assert_eq!(
        declaration.duplicates,
        DuplicatePolicy::Unique,
        "the promise under test is the producer's own, read back before it is relied on"
    );
    registry.register_ranked(TEXT_PF, Arc::new(text), declaration);
    registry
}

/// Register the nearest-neighbour relation alone, under its own declaration.
fn knn_only_registry(space: EmbeddingSpace) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    let knn = EmbeddingKnnRelation::new(Arc::new(space));
    let declaration = knn.ranked_declaration(
        kernel_iri(KNN_STRATUM),
        TermKind::Iri,
        XSD_INTEGER.to_owned(),
        RankFidelity::EXACT,
        CandidateDomains::Unrestricted,
    );
    assert_eq!(
        declaration.duplicates,
        DuplicatePolicy::Unique,
        "the promise under test is the producer's own, read back before it is relied on"
    );
    registry.register_ranked(KNN_PF, Arc::new(knn), declaration);
    registry
}

/// Run `search` and return the answer, distinguishing the one refusal these
/// tests exist to rule out from every other way a fixture can be wrong.
///
/// A `DuplicateItem` refusal and a clean answer both yield "no duplicate reached
/// the caller", so a test that only unwrapped would pass either way. This panics
/// on the refusal with its own message, so a producer that broke its promise is
/// reported as having broken its promise rather than as an opaque error.
fn answer_under_the_declared_contract(
    request: &RetrievalRequest,
    registry: &PropertyFunctionRegistry,
    data: &RdfDataset,
    profile: &FusionProfile,
) -> SearchResult {
    let statistics = NoStatistics;
    let env = AdmissionEnvironment {
        registry,
        statistics: &statistics,
        fusion_profile: None,
    };
    match block_on(search(request, registry, &statistics, data, &env, profile)) {
        Ok(result) => result,
        Err(SearchError::FusionError(FusionError::Protocol(error)))
            if matches!(&*error, ProtocolError::DuplicateItem { .. }) =>
        {
            panic!(
                "a shipped producer named one entity twice under its own `Unique` \
                 declaration and the fusion refused the whole query: {error:?}"
            )
        }
        Err(other) => panic!("the fixture request must be answerable; got {other:?}"),
    }
}

/// The candidates of an answer, in final order.
fn candidates(result: &SearchResult) -> Vec<String> {
    result
        .rows
        .iter()
        .map(|row| row.entity.as_str().to_owned())
        .collect()
}

/// Every candidate of `result` is distinct, compared against a set rather than
/// by eye.
fn assert_candidates_are_distinct(result: &SearchResult) {
    let emitted = candidates(result);
    let distinct: BTreeSet<&str> = emitted.iter().map(String::as_str).collect();
    assert_eq!(
        distinct.len(),
        emitted.len(),
        "a `Unique` producer's candidates must all differ, got {emitted:?}"
    );
}

/// How many rows the named stratum reported emitting, or a panic naming what it
/// reported instead.
fn rows_emitted(result: &SearchResult, stratum: &str) -> u64 {
    match result.trailer.statuses.get(&iri(stratum)) {
        Some(purrdf_retrieval::ProducerStatus::Exhausted { rows_emitted }) => *rows_emitted,
        other => panic!("{stratum} must run to exhaustion here, got {other:?}"),
    }
}

// ── the text producer ───────────────────────────────────────────────────────

/// The needle every document below is measured against. Three terms, because
/// one term cannot tempt a term-at-a-time repeat.
const HUB_NEEDLE: &str = "alpha beta gamma";

/// A corpus built to tempt a repeat out of the text producer.
///
/// `ex:hub` holds **every** term of [`HUB_NEEDLE`], twice over, so an
/// implementation that walked the needle term by term and emitted the postings
/// of each would name `ex:hub` three times — once per matching term — and an
/// implementation that walked occurrences rather than documents would name it
/// six times. The three single-term documents beside it are the control: each is
/// reachable by exactly one needle term, so an answer that named `ex:hub` once
/// and the others once cannot be explained by the producer having simply
/// collapsed everything.
///
/// Untagged, and in the default graph, so the index holds exactly one partition
/// — which is the condition the producer requires before it claims `Unique` at
/// all.
fn hub_corpus() -> Vec<(&'static str, &'static str, Option<&'static str>)> {
    vec![
        ("hub", "alpha beta gamma alpha beta gamma", None),
        ("only-alpha", "alpha delta delta delta", None),
        ("only-beta", "beta epsilon epsilon epsilon", None),
        ("only-gamma", "gamma zeta zeta zeta", None),
    ]
}

/// A lexical request for `needle` over the fixture predicate.
fn lexical_request(needle: &str) -> RetrievalRequest {
    RetrievalRequest::bounded(
        vec![RequestTerm::Lexical {
            text: needle.to_owned(),
            language: None,
            predicate: Some(iri(NOTE)),
        }],
        TOP_K,
    )
}

/// T10.1 — the shipped text producer keeps the `Unique` promise it declares,
/// over a corpus built so that a naive implementation would not.
///
/// `TextSearchRelation::ranked_declaration` declares
/// [`DuplicatePolicy::Unique`], and the fusion layer spends that declaration:
/// it does not keep a per-row identity set for a stream that promises not to
/// need one. The promise is therefore load-bearing, and
/// [`hub_corpus`] is shaped to catch it if it is false — `ex:hub` matches every
/// term of the needle, so any term-at-a-time or occurrence-at-a-time emission
/// names it more than once.
///
/// Both halves of the assertion matter and neither is sufficient alone. A
/// producer that repeated `ex:hub` would today be refused with
/// `ProtocolError::DuplicateItem` — which also yields "no duplicate in the
/// answer" — so the test must show the producer was not merely caught. It shows
/// that by requiring the call to succeed *and* the candidates to be distinct,
/// and by pinning the row count the stratum reported so the distinctness is not
/// the distinctness of a truncated prefix.
///
/// Its pair on the other side of the contract is
/// `a_declared_unique_streams_repeat_is_refused` in `fusion.rs`, which drives a
/// hand-built stream that *does* declare `Unique` and repeat, and pins the
/// refusal. Between them: a producer that breaks the promise is refused, and the
/// shipped producer does not break it.
#[test]
fn the_text_producer_names_each_document_once_over_a_corpus_that_tempts_a_repeat() {
    let data = dataset_of(&hub_corpus());
    let index = TextIndex::from_dataset(&*data, &text_config()).expect("the fixture index builds");
    assert_eq!(
        index.partition_count(),
        1,
        "the corpus is untagged and single-graph, so the producer may claim a ranked order"
    );
    let registry = text_only_registry(index);
    let profile = one_stratum_profile(TEXT_STRATUM);

    let result = answer_under_the_declared_contract(
        &lexical_request(HUB_NEEDLE),
        &registry,
        &data,
        &profile,
    );

    // Nothing was truncated: the stratum ran to exhaustion and emitted one row
    // per document of the corpus, so what follows is a statement about every
    // row the producer can return rather than about a short prefix of them.
    assert_eq!(
        rows_emitted(&result, TEXT_STRATUM),
        4,
        "all four documents hold at least one needle term"
    );
    assert_candidates_are_distinct(&result);
    assert_eq!(
        candidates(&result).len(),
        4,
        "four documents, four candidates"
    );
    assert_eq!(
        candidates(&result)
            .iter()
            .filter(|candidate| *candidate == &format!("<{}>", ex("hub")))
            .count(),
        1,
        "the document holding every needle term is named exactly once"
    );

    // And the temptation is real rather than asserted: each needle term on its
    // own reaches `ex:hub`, so the three-term needle gave the producer three
    // independent routes to it.
    for term in ["alpha", "beta", "gamma"] {
        let single =
            answer_under_the_declared_contract(&lexical_request(term), &registry, &data, &profile);
        assert!(
            candidates(&single).contains(&format!("<{}>", ex("hub"))),
            "{term} alone reaches ex:hub, so the full needle matched it through {term} too"
        );
        assert_candidates_are_distinct(&single);
    }
}

/// T10.3 — the text producer's own guard against the case where its `Unique`
/// declaration would be false, and the neighbouring case that must still work.
///
/// The producer's ranks are computed *within* a partition, and a partition is
/// keyed by `(graph, language)`. Over more than one partition the rows are
/// emitted partition-major, so one subject may appear in several of them and
/// `Unique` would be a lie. The producer does not declare it anyway: it declines
/// to hand out a ranked declaration at all, which is the refusal that makes the
/// declaration in [`text_only_registry`] trustworthy.
///
/// A refusal is a claim too, so both sides are executed here. The multi-language
/// index is refused; the single-partition index next to it still declares, and
/// the declaration it hands out still carries [`DuplicatePolicy::Unique`]. A
/// tightening that refused both would pass a test asserting only the first.
#[test]
fn the_text_producer_declares_unique_only_where_one_subject_can_appear_once() {
    let stratum = kernel_iri(TEXT_STRATUM);

    // The refused case: one subject, two languages, two partitions. `ex:shared`
    // is deliberately in both, so this is not a hypothetical overlap.
    let multilingual = dataset_of(&[
        ("shared", "alpha beta", Some("en")),
        ("shared", "alpha gamma", Some("fr")),
        ("other", "alpha delta", Some("en")),
    ]);
    let spread = TextIndex::from_dataset(&*multilingual, &text_config())
        .expect("the multilingual fixture index builds");
    assert_eq!(
        spread.partition_count(),
        2,
        "two language tags are two partitions"
    );
    let error = TextSearchRelation::new(Arc::new(spread))
        .ranked_declaration(
            stratum.clone(),
            Some(NOTE.to_owned()),
            RankFidelity::EXACT,
            CandidateDomains::Unrestricted,
        )
        .expect_err("a multi-partition index has no one ranked order to declare");
    match error {
        TextError::Config(message) => assert!(
            message.contains("rank is computed within one"),
            "the refusal must say why a multi-partition rank is not a ranking, got {message}"
        ),
        other => panic!("expected a configuration refusal, got {other:?}"),
    }

    // THE NEIGHBOURING VALID CASE: the file's own single-partition index. It
    // still declares, and what it declares is the promise the fusion spends.
    let single = text_index();
    assert_eq!(single.partition_count(), 1);
    let declaration = TextSearchRelation::new(Arc::new(single))
        .ranked_declaration(
            stratum,
            Some(NOTE.to_owned()),
            RankFidelity::EXACT,
            CandidateDomains::Unrestricted,
        )
        .expect("a single-partition index declares a ranked order");
    assert_eq!(
        declaration.duplicates,
        DuplicatePolicy::Unique,
        "within one partition a subject occurs at most once, and the producer says so"
    );
}

// ── the nearest-neighbour producer ──────────────────────────────────────────

/// A space whose rows are distinct terms carrying *indistinguishable* vectors.
///
/// `ex:twin-one` and `ex:twin-two` are byte-identical points, and `ex:near-twin`
/// is a point a hair further out. Every one of them is the same squared distance
/// from `ex:seed` to within rounding, so a search from the seed returns all
/// three at adjacent ranks. That is the shape that tempts a repeat: an
/// implementation keying its emitted set on the *vector*, or on the distance,
/// rather than on the target the row stands for, would either collapse the twins
/// into one row or name one of them twice.
///
/// It is also exactly the shape the space's construction permits: a PURREMB
/// target is derived from the RDF term, so two distinct IRIs are two distinct
/// targets and two distinct rows no matter what vectors they carry. Identical
/// vectors are legal; identical terms are not, and `bind_terms` refuses them.
fn twinned_vectors() -> Vec<(&'static str, Vec<f64>)> {
    vec![
        ("seed", vec![0.0, 0.0]),
        ("twin-one", vec![3.0, 4.0]),
        ("twin-two", vec![3.0, 4.0]),
        ("near-twin", vec![3.0, 4.000_000_1]),
        ("far", vec![30.0, 40.0]),
    ]
}

/// T10.2 — the shipped nearest-neighbour producer keeps the `Unique` promise it
/// declares, over a space whose vectors do not distinguish its rows.
///
/// What makes `EmbeddingKnnRelation`'s declaration true is not the geometry: it
/// is that a row of the space stands for exactly one target and each target is
/// bound to exactly one distinct RDF term — `bind_terms` refuses a space with an
/// unnamed row, a row bound twice, or one term claimed by two rows — and a
/// nearest-neighbour search returns distinct rows. So the property under test is
/// the one the producer actually relies on, and [`twinned_vectors`] removes the
/// thing it does *not* rely on: two of its rows are the same point, and a third
/// is a point too close to tell apart by eye.
///
/// As in the text case, both halves are asserted. A repeat would be refused with
/// `ProtocolError::DuplicateItem` and refusal also yields a duplicate-free
/// answer, so the call must succeed *and* the candidates must be distinct, with
/// the emitted row count pinned so the distinctness is not a truncated prefix's.
#[test]
fn the_knn_producer_names_each_target_once_when_two_rows_share_one_vector() {
    let rows = twinned_vectors();
    let row_count = u64::try_from(rows.len()).expect("the fixture is small");
    // The guard admits the whole space and a `k` as large as it: the planned
    // depth is the producer's declared row bound, which is
    // `min(max_neighbours, rows)`, so a tighter guard would read only a prefix
    // and the twins might never both be reached.
    let space = space_over(
        &rows,
        KnnGuard::new(row_count, row_count).expect("the fixture guard bounds are positive"),
    );
    let registry = knn_only_registry(space);
    let profile = one_stratum_profile(KNN_STRATUM);
    let data = dataset();
    let request = RetrievalRequest::bounded(
        vec![RequestTerm::EntitySeed {
            entity: Term::new(format!("<{}>", ex("seed"))),
        }],
        TOP_K,
    );

    let result = answer_under_the_declared_contract(&request, &registry, &data, &profile);

    // Every row of the space was reached, and the ending names the bound that
    // stopped the read rather than claiming the rows ran out: the depth sits on this
    // producer's declared row bound, so the row past it could not be asked for.
    assert_eq!(
        result.trailer.statuses.get(&iri(KNN_STRATUM)),
        Some(&purrdf_retrieval::ProducerStatus::RowBoundReached { rank: row_count }),
        "the guard admits the whole space, so the read went to the declared bound"
    );
    assert_candidates_are_distinct(&result);

    // The twins are both present, at adjacent ranks, which is what says the
    // fixture tempted the collapse rather than merely avoiding it: a producer
    // keyed on the vector would have emitted one of them, not two.
    assert_eq!(
        ranking(&result),
        vec![
            (
                format!("<{}>", ex("seed")),
                vec![(KNN_STRATUM.to_owned(), 1)],
            ),
            (
                format!("<{}>", ex("twin-two")),
                vec![(KNN_STRATUM.to_owned(), 2)],
            ),
            (
                format!("<{}>", ex("twin-one")),
                vec![(KNN_STRATUM.to_owned(), 3)],
            ),
            (
                format!("<{}>", ex("near-twin")),
                vec![(KNN_STRATUM.to_owned(), 4)],
            ),
            (
                format!("<{}>", ex("far")),
                vec![(KNN_STRATUM.to_owned(), 5)]
            ),
        ],
        "the two identical points are two candidates at consecutive ranks; their \
         distances tie exactly, so the order between them is ascending row number, \
         and a row number is a position in the target set's canonical target-id \
         order rather than in the order this fixture lists its rows"
    );
}

// ---------------------------------------------------------------------------
// 6. The sole producer over an empty corpus
// ---------------------------------------------------------------------------
//
// An index built before its documents land, or built over a predicate no triple
// carries yet, holds no documents. Its declared `rows_per_invocation` is
// therefore zero — an honest measurement of the data, not a refusal of the
// relation — and a host with one text index has exactly one registered producer.
//
// That pairing is the whole of this section, and it is the shape every other
// test of a zero-row declaration left out: each of those registered a second,
// surviving producer, so the plan always had something else to bind and the
// sole-producer registry was never driven. The claim here is that the relation
// is INVOKED at the floored depth of one and reports its own
// `Exhausted { rows_emitted: 0 }` — an ending it earned by reading, rather than
// one a bound asserted for it — rather than the request being refused before
// anything ran.

/// A real index over a corpus holding no documents at all.
///
/// The configuration is [`text_config`], unchanged, so nothing about the index's
/// shape differs from the populated fixtures; only the data does. The dataset is
/// returned alongside it because the same rows have to reach `search`.
fn empty_text_index() -> (Arc<RdfDataset>, TextIndex) {
    let data = dataset_of(&[]);
    let index = TextIndex::from_dataset(&*data, &text_config())
        .expect("an empty corpus is a valid corpus to index");
    assert_eq!(
        index.document_count(),
        0,
        "the fixture's premise is that the index holds nothing"
    );
    (data, index)
}

/// The shipped text relation, wrapped so that its invocation is observable.
///
/// Every method delegates to the real [`TextSearchRelation`], which does all of
/// the work; the only thing the wrapper adds is a counter bumped in `open`. The
/// point is that "the producer reports the emptiness" is a claim about a call, and
/// a status of `Exhausted { rows_emitted: 0 }` does not evidence one: a plan that
/// bound the producer nowhere and refused outright made no call at all, and there
/// was no receipt to read. The counter checks the call happened rather than
/// inferring it.
///
/// It is deliberately *not* the check that the emitted bound is not `LIMIT 0`. The
/// evaluator opens a relation and applies the bound afterwards, so the counter
/// rises either way — measured, not assumed. That bound is asserted separately,
/// against the compiled text, in [`sole_producer_answer`].
struct CountingTextRelation {
    inner: TextSearchRelation,
    opened: Arc<AtomicU64>,
}

impl PropertyFunction for CountingTextRelation {
    fn volatility(&self) -> Volatility {
        self.inner.volatility()
    }

    fn arity(&self) -> PfArity {
        self.inner.arity()
    }

    fn modes(&self) -> &[BindingPattern] {
        self.inner.modes()
    }

    fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
        self.inner.rows_per_invocation(mode)
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        self.opened.fetch_add(1, Ordering::Relaxed);
        self.inner.open(args, ceiling)
    }
}

/// Register the shipped text relation, alone, behind [`CountingTextRelation`].
///
/// The declaration is the shipped relation's own — read off the relation before it
/// is wrapped — so the registry describes the real producer and not the wrapper.
fn counting_text_registry(index: TextIndex) -> (PropertyFunctionRegistry, Arc<AtomicU64>) {
    let mut registry = PropertyFunctionRegistry::new();
    let inner = TextSearchRelation::new(Arc::new(index));
    let declaration = inner
        .ranked_declaration(
            kernel_iri(TEXT_STRATUM),
            Some(NOTE.to_owned()),
            RankFidelity::EXACT,
            CandidateDomains::Unrestricted,
        )
        .expect("a single-partition index declares a ranked order");
    let opened = Arc::new(AtomicU64::new(0));
    registry.register_ranked(
        TEXT_PF,
        Arc::new(CountingTextRelation {
            inner,
            opened: Arc::clone(&opened),
        }),
        declaration,
    );
    (registry, opened)
}

/// Plan, admit and answer `request` against a registry holding one producer.
///
/// Returns the plan beside the answer, because the two claims this section makes
/// live in different places: the depth comes from the plan, and the receipt from
/// the answer. The compiled unit is checked on the way through, because the emitted
/// bound is the one thing neither of those two can report.
fn sole_producer_answer(
    request: &RetrievalRequest,
    registry: &PropertyFunctionRegistry,
    data: &RdfDataset,
) -> (purrdf_retrieval::Plan, SearchResult) {
    let statistics = NoStatistics;
    let planned = plan(request, registry, &statistics)
        .expect("a registry holding one accepting producer plans the request");
    let env = AdmissionEnvironment {
        registry,
        statistics: &statistics,
        fusion_profile: None,
    };
    let compiled = compile(&planned, &env).expect("the sole-producer plan is admitted");
    let unit = compiled
        .units
        .iter()
        .find(|unit| unit.stratum == iri(TEXT_STRATUM))
        .expect("the one stratum emits a unit");
    assert!(
        !unit.sparql().contains("LIMIT 0"),
        "a unit bounded at nothing hands back no row whatever the index holds, so its \
         stratum's exhaustion would be the bound's claim and not the producer's — and \
         it reads identically to an honest empty answer in every field of the trailer: \
         {}",
        unit.sparql()
    );
    let profile = one_stratum_profile(TEXT_STRATUM);
    let result = block_on(search(request, registry, &statistics, data, &env, &profile))
        .expect("the sole producer answers rather than the request being refused");
    (planned, result)
}

/// Every stratum status of `result`, so a claim about one of them can be made
/// against the whole report rather than against a lookup that might miss.
fn statuses(result: &SearchResult) -> Vec<(String, purrdf_retrieval::ProducerStatus)> {
    result
        .trailer
        .statuses
        .iter()
        .map(|(stratum, status)| (stratum.as_str().to_owned(), status.clone()))
        .collect()
}

#[test]
fn the_sole_text_producer_over_an_empty_corpus_reports_its_own_emptiness() {
    let (data, index) = empty_text_index();
    let (registry, opened) = counting_text_registry(index);
    let request = lexical_request("alpha beta");
    let (planned, result) = sole_producer_answer(&request, &registry, &data);

    // The plan. The producer accepts the lexical term and is bound to it — the
    // declaration promising zero rows describes the data, so it is no ground for
    // dropping the producer — and its stratum reads the one floored row.
    assert!(
        planned
            .producer_bindings
            .iter()
            .any(|binding| binding.producer == TEXT_PF && binding.request_terms == vec![0]),
        "the sole producer receives the needle: {:?}",
        planned.producer_bindings
    );
    assert_eq!(
        planned.stratum_depths[&iri(TEXT_STRATUM)],
        1,
        "the stratum records the floored depth of one, which is the probing read"
    );
    assert!(
        planned.unserved_terms.is_empty(),
        "and the needle is served: a producer that reads and finds nothing has \
         answered the term, {:?}",
        planned.unserved_terms
    );

    // The answer. No rows, and the reason there are none is the producer's own
    // receipt rather than a planner verdict: `Exhausted` is the one ending that
    // names no stopper, and here it says all it can say, because the relation
    // really did run and really found nothing.
    assert!(
        result.rows.is_empty(),
        "an empty corpus ranks nothing: {:?}",
        ranking(&result)
    );
    assert!(
        opened.load(Ordering::Relaxed) > 0,
        "the relation was called at all — which a refused plan, the defect this \
         section pins, never managed, leaving no receipt for the exhaustion below \
         to be read off"
    );
    assert_eq!(
        statuses(&result),
        vec![(
            TEXT_STRATUM.to_owned(),
            purrdf_retrieval::ProducerStatus::Exhausted { rows_emitted: 0 },
        )],
        "the one stratum reports the producer's own exhaustion at zero rows"
    );
    // Stated separately, because it is the one status that would mean the read
    // was cut rather than complete — and a `LIMIT 0` unit would have produced
    // `Exhausted` here too, so the count above is only half the claim.
    assert!(
        !result.trailer.statuses.values().any(|status| matches!(
            status,
            purrdf_retrieval::ProducerStatus::DepthReached { .. }
        )),
        "nothing was cut by the depth: {:?}",
        statuses(&result)
    );
}

#[test]
fn the_sole_text_producer_over_one_document_still_returns_that_document() {
    // The neighbour that must still work, and the arm that proves the producer
    // accepts the term the empty-corpus arm asserts it accepts. Nothing changes
    // but the number of documents the index holds.
    let data = dataset_of(&[("only", "alpha beta gamma delta", None)]);
    let index =
        TextIndex::from_dataset(&*data, &text_config()).expect("the one-document corpus indexes");
    assert_eq!(index.document_count(), 1);
    let (registry, opened) = counting_text_registry(index);
    let request = lexical_request("alpha beta");
    let (planned, result) = sole_producer_answer(&request, &registry, &data);

    assert!(
        opened.load(Ordering::Relaxed) > 0,
        "the same registry over one document invokes the same relation"
    );
    assert_eq!(
        planned.stratum_depths[&iri(TEXT_STRATUM)],
        1,
        "one document is one row of declared depth"
    );
    assert_eq!(
        candidates(&result),
        vec![format!("<{}>", ex("only"))],
        "the document the index holds is the answer"
    );
    assert_eq!(
        statuses(&result),
        vec![(
            TEXT_STRATUM.to_owned(),
            purrdf_retrieval::ProducerStatus::Exhausted { rows_emitted: 1 },
        )],
        "and the receipt counts the row it emitted"
    );
}

// ---------------------------------------------------------------------------
// 6. The vector stratum that covers half a corpus
// ---------------------------------------------------------------------------

/// The IRI of the disclosure a host makes when its vector space is a sample.
///
/// Prose a reader can act on, naming the measurement, its limit, and what an
/// absence does not prove — which is what the producer contract asks a declared
/// loss to carry.
const SAMPLE_EVIDENCE: &str = "this space was embedded over the first half of \
     the corpus only; a document the lexical stratum names and this one does \
     not may be an unembedded document rather than a distant one";

/// The whole text index, and a nearest-neighbour space over **half** the
/// vectors, under whatever fidelity the host states about it.
///
/// The space really is short — it is built from a truncated row list, not
/// labelled as though it were — so the declaration beside it is a statement a
/// reader can falsify by deleting the truncation.
fn registry_over_a_partial_vector_space(fidelity: RankFidelity) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();

    let text = TextSearchRelation::new(Arc::new(text_index()));
    let text_declaration = text
        .ranked_declaration(
            kernel_iri(TEXT_STRATUM),
            Some(NOTE.to_owned()),
            // The control beside the producer under test: this index holds every
            // document the corpus has, so its exhaustive declaration is true and
            // any deficit the answer reports is the other stratum's.
            RankFidelity::EXACT,
            CandidateDomains::Unrestricted,
        )
        .expect("a single-partition index declares a ranked order");
    registry.register_ranked(TEXT_PF, Arc::new(text), text_declaration);

    let rows = vector_rows();
    let half = &rows[..rows.len() / 2];
    let knn = EmbeddingKnnRelation::new(Arc::new(space_over(
        half,
        KnnGuard::new(10, 5).expect("the fixture guard bounds are positive"),
    )));
    let knn_declaration = knn.ranked_declaration(
        kernel_iri(KNN_STRATUM),
        TermKind::Iri,
        XSD_INTEGER.to_owned(),
        fidelity,
        CandidateDomains::Unrestricted,
    );
    registry.register_ranked(KNN_PF, Arc::new(knn), knn_declaration);

    registry
}

/// Run the standard request against `registry` over the whole dataset.
fn answer_from(registry: &PropertyFunctionRegistry) -> SearchResult {
    let data = dataset();
    let statistics = NoStatistics;
    let environment = AdmissionEnvironment {
        registry,
        statistics: &statistics,
        fusion_profile: None,
    };
    block_on(search(
        &request(),
        registry,
        &statistics,
        &data,
        &environment,
        &profile(),
    ))
    .expect("the partial-space fixture answers rather than being refused")
}

/// A vector space holding half the corpus, with the host declaring the
/// shortfall, stops the answer certifying that every stratum was whole.
///
/// This is the failure the exhaustive declaration used to make unreportable.
/// The kNN relation's own search is exact — it scans every row it holds, prunes
/// nothing, exits early nowhere — so the relation had every reason to believe
/// `EXACT` of itself, and asserting it put the top of the lattice into the mouth
/// of the one party that knew the space was a sample. Downstream nothing could
/// recover it: a stream that ran out of rows and a stream whose space never held
/// them both stop yielding, both leave contiguous ranks, both report exhaustion.
/// The answer read as complete while a document the whole space would have
/// ranked was silently absent.
#[test]
fn a_vector_space_over_half_the_corpus_stops_the_answer_claiming_wholeness() {
    let evidence: Arc<str> = Arc::from(SAMPLE_EVIDENCE);
    let declared = answer_from(&registry_over_a_partial_vector_space(RankFidelity {
        completeness: Completeness::Lossy {
            evidence: Arc::clone(&evidence),
        },
        // The rows it DOES hold are compared at exact distances and ordered
        // truly, so an emitted rank still bounds a true rank and the answer's
        // error stays finite. The two axes fail independently and only one of
        // them failed here.
        order: OrderFidelity::Faithful,
    }));

    // The shortfall is real, not a label. `ex:d` is the document the whole
    // vector space ranks third from this seed, and it is absent from the
    // truncated one — so it reaches the answer through no stratum at all,
    // because the needle does not match its text either.
    assert!(
        !candidates(&declared).contains(&format!("<{}>", ex("d"))),
        "the fixture is genuinely short: {:?}",
        candidates(&declared)
    );

    // And the answer says so, where a consumer looks.
    let ScoreExactness::Estimated {
        deficit,
        inflation,
        unbounded,
    } = &declared.trailer.exactness
    else {
        panic!("a lossy stratum must not leave the answer certifying exact scores");
    };
    assert_eq!(
        deficit.iter().map(Iri::as_str).collect::<Vec<_>>(),
        vec![KNN_STRATUM],
        "the stratum that may have withheld a row is the short one, and only it"
    );
    assert_eq!(
        inflation.iter().map(Iri::as_str).collect::<Vec<_>>(),
        vec![KNN_STRATUM],
        "a withheld row promotes every row behind it, so loss runs both ways"
    );
    assert!(
        unbounded.is_empty(),
        "order is still faithful, so a finite bound on the error survives"
    );

    // The host's words arrive byte for byte. A consumer renders these; nothing
    // in the workspace parses them, and nothing re-words them.
    let carried = declared
        .trailer
        .fidelities
        .get(&iri(KNN_STRATUM))
        .expect("the short stratum declared a fidelity");
    assert_eq!(
        carried.evidence().map(|e| &**e).collect::<Vec<_>>(),
        vec![SAMPLE_EVIDENCE]
    );

    // Nothing was routed through the attestation channel to get here. That
    // channel answers "was the index version that served this invocation
    // whole", and an `Incomplete` reading there is a REFUSAL on every lane that
    // carries no witness — so a host stating a permanent fact about its corpus
    // there would take out its ordinary SPARQL queries as well.
    assert_eq!(
        declared
            .trailer
            .attestations
            .get(&iri(KNN_STRATUM))
            .map(|attestation| attestation.service.clone()),
        Some(purrdf_sparql_eval::ServiceLevel::Undeclared),
        "the index really was whole; it is the corpus behind it that was not"
    );
}

/// The neighbour that must keep working: a host with nothing to disclose gets
/// the exact answer it always did.
///
/// The failure mode of a fidelity term is not only that it can be omitted — it
/// is also that it can degrade every answer into an estimate and make the
/// certified score unreachable. This is the case that proves it did not: the
/// whole corpus, both producers exhaustive, and a trailer that still certifies
/// its scores as values.
#[test]
fn a_whole_corpus_with_nothing_to_disclose_still_certifies_exact_scores() {
    let whole = answer_from(&registry());
    assert_eq!(
        whole.trailer.exactness,
        ScoreExactness::Exact,
        "every stratum was exhaustive and whole, and the answer may say so"
    );
    assert!(
        candidates(&whole).contains(&format!("<{}>", ex("d"))),
        "including the document the truncated space above could not name"
    );
    for stratum in [TEXT_STRATUM, KNN_STRATUM] {
        assert_eq!(
            whole
                .trailer
                .fidelities
                .get(&iri(stratum))
                .expect("both producers declared a fidelity"),
            &RankFidelity::EXACT
        );
    }
}
