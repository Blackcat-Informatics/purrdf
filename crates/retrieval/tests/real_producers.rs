// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
//! Geometry is deliberately absent. `purrdf-geo`'s relation computes a set — it
//! sorts and deduplicates its pairs and carries neither a score nor a rank — so
//! it composes as a constraint on candidates rather than as a stratum of a fused
//! ranking, and inventing a stratum for it would be inventing a ranking it never
//! claimed.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
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
    AdmissionEnvironment, Fixed, FusionProfile, Iri, RankedStreamAdapter, RequestTerm,
    RetrievalRequest, SearchResult, Statistics, Term, TopK, compile, contribution, execute, fuse,
    plan, search,
};
use purrdf_sparql_eval::{
    EmbeddingKnnRelation, EmbeddingSpace, KnnGuard, PropertyFunctionRegistry, TermKind,
};
use purrdf_text::{GraphSelector, TextIndex, TextIndexConfig, TextSearchRelation};

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
/// construction, so the bound is stated rather than defaulted; four documents
/// are all this corpus holds, so nothing here is decided by it.
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
    let mut builder = RdfDatasetBuilder::new();
    let note = builder.intern_iri(NOTE);
    for (local, text, _) in corpus() {
        let subject = builder.intern_iri(&ex(local));
        let literal = builder.intern_literal(RdfLiteral::simple(text));
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

/// Encode a sealed PURREMB artifact holding one `f64` row per corpus document,
/// under the squared-Euclidean metric.
fn artifact() -> (
    Vec<u8>,
    TargetSetId,
    VectorSpaceId,
    Vec<(TargetId, TermValue)>,
) {
    let rows = corpus();
    let dimension = u32::try_from(rows[0].2.len()).expect("the fixture dimension is small");
    let empty = RdfDatasetBuilder::new().freeze().expect("empty dataset");
    let (source, _) = CertifiedPurrpckSource::from_dataset(&empty).expect("source pack");

    let mut targets = Vec::with_capacity(rows.len());
    let mut bindings = Vec::with_capacity(rows.len());
    for (local, _, _) in &rows {
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
            .map(|((_, _, values), (target, _))| MatrixRow::new(*target, values.clone()))
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
    let (bytes, target_set, vector_space, bindings) = artifact();
    EmbeddingSpace::from_artifact(
        &bytes,
        target_set,
        vector_space,
        bindings,
        KnnGuard::new(10, 5).expect("the fixture guard bounds are positive"),
    )
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
        .ranked_declaration(kernel_iri(TEXT_STRATUM), Some(NOTE.to_owned()))
        .expect("a single-partition index declares a ranked order");
    registry.register_ranked(TEXT_PF, Arc::new(text), text_declaration);

    let knn = EmbeddingKnnRelation::new(Arc::new(embedding_space()));
    let knn_declaration = knn.ranked_declaration(
        kernel_iri(KNN_STRATUM),
        // The space's rows are IRIs, and an IRI seed is what this host's
        // requests name.
        TermKind::Iri,
        XSD_INTEGER.to_owned(),
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
    RetrievalRequest::from_terms(vec![
        RequestTerm::Lexical {
            text: "alpha beta".to_owned(),
            language: None,
            predicate: Some(iri(NOTE)),
        },
        RequestTerm::EntitySeed {
            entity: Term::new(format!("<{}>", ex("a"))),
        },
    ])
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
    FusionProfile::new(weights, K).expect("the fixture profile is valid")
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
        TOP_K,
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

    // Both strata ran to completion, with the row counts their own data
    // supports: two documents hold the needle's terms, and the space returns
    // the four nearest of its four rows.
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
            Some(purrdf_retrieval::ProducerStatus::Exhausted { rows_emitted: 4 })
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
        text.sparql,
        format!(
            "SELECT ?candidate WHERE {{\n  \
             {{ SELECT (?c0 AS ?candidate) WHERE {{ ( ?c0 ) <{TEXT_PF}> \
             ( \"alpha beta\" ?c2 ?c3 ?c4 ?c5 ) }} LIMIT 4 }}\n\
             }}\nLIMIT 4"
        ),
        "the needle is a rendered constant at the relation's own needle position, \
         every other position is free, and the relation takes no depth argument so the \
         branch carries the stratum's LIMIT"
    );

    let knn = compiled
        .units
        .iter()
        .find(|unit| unit.stratum == iri(KNN_STRATUM))
        .expect("the neighbour stratum emits a unit");
    assert_eq!(
        knn.sparql,
        format!(
            "SELECT ?candidate WHERE {{\n  \
             {{ SELECT (?c0 AS ?candidate) WHERE {{ ( ?c0 ) <{KNN_PF}> \
             ( <{seed}> \"4\"^^<{XSD_INTEGER}> ?c3 ) }} }}\n\
             }}\nLIMIT 4",
            seed = ex("a")
        ),
        "the seed is a rendered constant and the stratum's depth IS the neighbour \
         count, so this branch bounds itself and carries no LIMIT of its own"
    );
}

// ---------------------------------------------------------------------------
// 3. The composition identity, on the real path
// ---------------------------------------------------------------------------

/// Drive `fuse ∘ execute ∘ compile ∘ plan` by hand over the real producers.
///
/// The bridge from the executor's `(rank, candidate)` rows to the fusion
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
    let compiled = compile(&planned, env).expect("a fresh plan is admitted");
    let execution = execute(&compiled, registry, data)
        .await
        .expect("both real relations run");

    let mut streams = Vec::new();
    let mut unweighted_strata = Vec::new();
    for stream in execution.streams {
        match RankedStreamAdapter::new(stream.stream, profile, &stream.stratum) {
            // The plan the unit was compiled from travels on with the rows; the
            // trailer names it, and the answer's identity is read back from
            // there rather than asked of the plan a second time.
            Some(adapter) => streams.push((stream.stratum, adapter.with_plan_id(stream.plan_id))),
            None => unweighted_strata.push(stream.stratum),
        }
    }
    unweighted_strata.sort();

    let fused = fuse::<RankedStreamAdapter, Term>(streams, profile, TOP_K)
        .await
        .expect("the surviving streams fuse");
    SearchResult {
        rows: fused.rows,
        trailer: fused.trailer.completed_with(execution.statuses),
        unserved_terms: planned.unserved_evidence(),
        plan_id: planned.id(),
        profile_id: profile.id(),
        unweighted_strata,
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
        TOP_K,
    ))
    .expect("the composed search answers");
    let manual = block_on(manual_composition(
        &registry,
        &statistics,
        &data,
        &env,
        &profile,
    ));

    assert_eq!(
        direct, manual,
        "search is exactly fuse ∘ execute ∘ compile ∘ plan, on the real path too"
    );
}
