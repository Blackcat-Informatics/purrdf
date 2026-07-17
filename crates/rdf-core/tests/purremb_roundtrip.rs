// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end PURREMB construction, verification, and Matryoshka access.

use purrdf_core::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, CanonicalMetadataInput,
    CertifiedPurrpckSource, ContentDigest, CorpusTarget, DimensionalityPolicy, DistanceMetric,
    DocumentTarget, EffectivePrefix, EmbeddingBuilder, EmbeddingFamilyContract, EmbeddingView,
    MatrixInput, MatrixRow, PackView, PrefixPostprocessing, ProjectionSpec, RdfDatasetBuilder,
    RdfTermTarget, RelationKind, SourceVerificationMode, StageImplementation, TargetRelation,
    TargetSet, TermValue, TextChunkTarget, TokenSpan, VectorDtype, derive_vector_space_id,
    require_compatible_vector_spaces, verify_embedding, verify_embedding_source,
};

fn golden_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/goldens/purremb_v1.bin")
}

struct Fixture {
    source_bytes: Vec<u8>,
    artifact_bytes: Vec<u8>,
}

fn artifact(name: &str) -> ArtifactIdentity {
    ArtifactIdentity::new(
        format!("https://example.org/artifact/{name}"),
        "application/octet-stream",
        ContentDigest::of(name.as_bytes()),
        Some(b"fixture-v1".to_vec()),
        ArtifactIdentityKind::Single,
    )
    .expect("valid artifact identity")
}

fn stage(name: &str) -> AppliedStage {
    AppliedStage::Applied(
        StageImplementation::new(
            format!("https://example.org/stage/{name}"),
            ContentDigest::of(name.as_bytes()),
            "application/cbor",
            vec![0xa1, 0x61, b'v', 0x01],
        )
        .expect("valid stage contract"),
    )
}

fn family_contract() -> EmbeddingFamilyContract {
    EmbeddingFamilyContract {
        model: artifact("model"),
        engine: artifact("engine"),
        tokenizer: artifact("tokenizer"),
        execution: stage("execution"),
        subject_projection: stage("subject-projection"),
        preprocessing: stage("unicode-nfc"),
        chunking: stage("overlapping-utf8-chunks"),
        pooling: stage("mean-pooling"),
        normalization: AppliedStage::NotApplied,
        truncation: AppliedStage::NotApplied,
        dtype: VectorDtype::F32,
        metric: DistanceMetric::Cosine,
        dimensionality: DimensionalityPolicy::matryoshka(vec![
            EffectivePrefix {
                dimension: 2,
                postprocessing: PrefixPostprocessing::None,
            },
            EffectivePrefix {
                dimension: 4,
                postprocessing: PrefixPostprocessing::DeterministicL2,
            },
        ])
        .expect("valid Matryoshka policy"),
        extensions: Vec::new(),
    }
}

fn build_fixture(reverse_input: bool) -> Fixture {
    let mut rdf = RdfDatasetBuilder::new();
    let subject = rdf.intern_iri("https://example.org/rdf/subject");
    let predicate = rdf.intern_iri("https://example.org/rdf/predicate");
    let object = rdf.intern_iri("https://example.org/rdf/object");
    rdf.push_quad(subject, predicate, object, None);
    let rdf = rdf.freeze().expect("valid RDF 1.2 dataset");
    let (source, source_bytes) = CertifiedPurrpckSource::from_dataset(&rdf).expect("source pack");

    let pack = PackView::from_bytes(&source_bytes).expect("source pack view");
    let subject_ordinal = pack
        .dict()
        .id_by_value(&TermValue::Iri("https://example.org/rdf/subject".into()))
        .expect("subject in pack");

    let family_contract = family_contract();
    let family = family_contract.derive().expect("derived family");
    let dataset_target = source.dataset_target(true).expect("dataset target");
    let rdf_term = RdfTermTarget::Iri("https://example.org/rdf/subject".into())
        .into_target(true, Some(subject_ordinal))
        .expect("RDF term target");
    let dataset_target_id = dataset_target.id;
    let rdf_term_id = rdf_term.id;

    let corpus = CorpusTarget {
        manifest_digest: ContentDigest::of(b"fixture corpus manifest"),
        manifest_media_type: "application/vnd.example.corpus-manifest+cbor".into(),
        logical_id_digest: ContentDigest::of(b"corpus:fixture"),
    }
    .into_target(true)
    .expect("corpus target");
    let document_bytes = "alpha βeta gamma delta".as_bytes();
    let document_model = DocumentTarget::from_content(
        corpus.id,
        ContentDigest::of(b"document:fixture"),
        "text/plain;charset=utf-8",
        document_bytes,
    )
    .expect("document metadata");
    let document = document_model
        .clone()
        .into_target(true)
        .expect("document target");
    let chunk_model =
        TextChunkTarget::from_document(document.id, family.chunking_id, document_bytes, 6, 17)
            .expect("UTF-8 chunk");
    let chunk = chunk_model.clone().into_target(true).expect("chunk target");

    document_model
        .verify_content(document_bytes)
        .expect("document content binding");
    chunk_model
        .verify_document(document_bytes)
        .expect("chunk content binding");

    let target_set = TargetSet::new(vec![chunk.id, rdf_term_id, dataset_target_id])
        .expect("canonical target set");
    let mut targets = vec![
        dataset_target,
        rdf_term,
        corpus.clone(),
        document.clone(),
        chunk.clone(),
    ];
    let mut relations = vec![
        TargetRelation::builtin(corpus.id, RelationKind::CorpusDocument, document.id),
        TargetRelation::builtin(document.id, RelationKind::DocumentChunk, chunk.id),
    ];
    let token_span = TokenSpan {
        family_id: family.id,
        target_id: chunk.id,
        token_start: 1,
        token_end: 4,
        model_input_token_count: 5,
        left_truncated: false,
        right_truncated: false,
        includes_special_tokens: true,
    };

    let mut rows = vec![
        MatrixRow::new(chunk.id, vec![3.0, 4.0, 5.0, 12.0]),
        MatrixRow::new(rdf_term_id, vec![-0.0, 2.0, 0.5, -0.5]),
        MatrixRow::new(dataset_target_id, vec![1.0, -1.0, 2.0, -2.0]),
    ];
    if reverse_input {
        targets.reverse();
        relations.reverse();
        rows.reverse();
    }

    let metadata = CanonicalMetadataInput {
        source,
        family_contracts: vec![family_contract],
        targets,
        target_sets: vec![target_set.clone()],
        relations,
        token_spans: vec![token_span],
        external_bindings: Vec::new(),
        indexes: Vec::new(),
        extensions: Vec::new(),
    };
    let matrix = MatrixInput {
        family_id: family.id,
        target_set_id: target_set.id,
        stored_dimension: family.stored_dimension,
        rows,
        projections: family
            .spaces
            .iter()
            .map(|space| {
                ProjectionSpec::derive(space.family_id, space.dimension, space.postprocessing)
            })
            .collect(),
    };
    let mut builder = EmbeddingBuilder::from_typed_metadata(metadata);
    builder.add_f32_matrix(matrix);
    let encoded = builder.build().expect("canonical PURREMB artifact");
    Fixture {
        source_bytes,
        artifact_bytes: encoded.bytes,
    }
}

#[test]
fn typed_round_trip_verifies_source_and_matryoshka_views() {
    let fixture = build_fixture(false);
    let mut view = EmbeddingView::from_bytes(&fixture.artifact_bytes).expect("structural open");
    assert_eq!(view.family_count(), 1);
    assert_eq!(view.target_count(), 5);
    assert_eq!(view.matrix_count(), 1);
    assert_eq!(view.projection_count(), 2);

    let report = verify_embedding(&mut view).expect("full verification");
    assert_eq!(report.scalar_count(), 12);
    let source_report = verify_embedding_source(
        &view,
        &fixture.source_bytes,
        SourceVerificationMode::Certified,
    )
    .expect("certified source verification");
    assert_eq!(source_report.ordinals_checked(), 1);

    let matrix = view.matrices().next().expect("one matrix");
    let target_set = view.target_sets().next().expect("one target set");
    let family = view.families().next().expect("one family");
    let spaces = family.spaces().collect::<Vec<_>>();
    for space in &spaces {
        let effective = view
            .effective_matrix(target_set.id(), space.id())
            .expect("unique projection")
            .expect("effective matrix");
        assert_eq!(
            effective.raw_prefix_bytes(0).expect("raw prefix").len(),
            usize::try_from(space.dimension()).expect("small dimension") * size_of::<f32>()
        );
        assert_eq!(effective.matrix().id(), matrix.id());
    }
    assert_ne!(spaces[0].id(), spaces[1].id());
    assert_ne!(
        view.effective_matrix(target_set.id(), spaces[0].id())
            .expect("short lookup")
            .expect("short matrix")
            .projection()
            .id(),
        view.effective_matrix(target_set.id(), spaces[1].id())
            .expect("full lookup")
            .expect("full matrix")
            .projection()
            .id()
    );
    assert!(require_compatible_vector_spaces(spaces[0].id(), spaces[0].id()).is_ok());
    assert!(require_compatible_vector_spaces(spaces[0].id(), spaces[1].id()).is_err());
    let unavailable = derive_vector_space_id(family.id(), 3, PrefixPostprocessing::None.code());
    assert!(
        view.effective_matrix(target_set.id(), unavailable)
            .expect("unavailable lookup")
            .is_none()
    );
    let normalized = view
        .effective_matrix(target_set.id(), spaces[1].id())
        .expect("full lookup")
        .expect("full matrix")
        .f32_row(0)
        .expect("normalized row")
        .collect::<Result<Vec<_>, _>>()
        .expect("finite normalized values");
    let norm = normalized
        .iter()
        .map(|value| f64::from(*value).powi(2))
        .sum::<f64>()
        .sqrt();
    assert!((norm - 1.0).abs() < 1.0e-6);

    assert!(
        !fixture
            .artifact_bytes
            .windows("alpha βeta gamma delta".len())
            .any(|window| window == "alpha βeta gamma delta".as_bytes()),
        "external corpus text leaked into PURREMB bytes"
    );
}

#[test]
fn unordered_typed_inputs_produce_identical_bytes() {
    let forward = build_fixture(false);
    let reverse = build_fixture(true);
    assert_eq!(forward.source_bytes, reverse.source_bytes);
    assert_eq!(forward.artifact_bytes, reverse.artifact_bytes);
}

#[test]
fn canonical_artifact_matches_checked_in_golden() {
    let expected = std::fs::read(golden_path()).expect("checked-in PURREMB golden");
    assert_eq!(build_fixture(false).artifact_bytes, expected);
}

#[test]
#[ignore = "explicit golden regeneration only"]
fn regenerate_canonical_artifact_golden() {
    assert_eq!(
        std::env::var_os("PURREMB_REGENERATE_GOLDEN").as_deref(),
        Some(std::ffi::OsStr::new("1")),
        "set PURREMB_REGENERATE_GOLDEN=1 to replace the golden"
    );
    std::fs::write(golden_path(), build_fixture(false).artifact_bytes)
        .expect("write PURREMB golden");
}
