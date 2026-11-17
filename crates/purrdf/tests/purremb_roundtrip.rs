// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! End-to-end PURREMB construction, verification, and Matryoshka access.

use purrdf::{
    AppliedStage, CanonicalMetadataInput, CertifiedEmbeddingSource, ContentDigest, CorpusTarget,
    DimensionalityPolicy, DistanceMetric, DocumentTarget, EffectivePrefix, EmbeddingBuilder,
    EmbeddingFamilyContract, EmbeddingSource, EmbeddingView, MatrixInput, MatrixRow, PackView,
    PrefixPostprocessing, ProjectionSpec, RdfDatasetBuilder, RdfTermTarget, RelationKind,
    TargetRelation, TargetSet, TermValue, TextChunkTarget, TokenSpan, VectorDtype,
    derive_vector_space_id, require_compatible_vector_spaces, verify_embedding,
    verify_embedding_source,
};
use purrdf_core::distance::{Arithmetic as _, Exact};
use purrdf_core::purremb_fixture::Identities;

/// The canonical artifact the writer emits: format version 2.
fn golden_path() -> std::path::PathBuf {
    purrdf_testkit::paths::workspace_root!().join("crates/rdf-core/tests/goldens/purremb_v2.bin")
}

/// The artifact PurRDF 4.x wrote for the same fixture, frozen as the evidence that a
/// valid 4.x embedding keeps verifying. It is never regenerated: format version 1.
fn version_one_golden() -> Vec<u8> {
    std::fs::read(
        purrdf_testkit::paths::workspace_root!()
            .join("crates/rdf-core/tests/goldens/purremb_v1.bin"),
    )
    .expect("checked-in 4.x PURREMB golden")
}

struct Fixture {
    source_bytes: Vec<u8>,
    artifact_bytes: Vec<u8>,
}

const FX: Identities = Identities {
    artifact_base: "https://example.org/artifact/",
    stage_base: "https://example.org/stage/",
    artifact_salt: Some(b"fixture-v1"),
    stage_media: "application/cbor",
    stage_payload: &[0xa1, 0x61, b'v', 0x01],
    ..Identities::at("")
};

fn family_contract() -> EmbeddingFamilyContract {
    EmbeddingFamilyContract {
        model: FX.artifact("model"),
        engine: FX.artifact("engine"),
        tokenizer: FX.artifact("tokenizer"),
        execution: FX.stage("execution"),
        subject_projection: FX.stage("subject-projection"),
        preprocessing: FX.stage("unicode-nfc"),
        chunking: FX.stage("overlapping-utf8-chunks"),
        pooling: FX.stage("mean-pooling"),
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

fn source_dataset() -> std::sync::Arc<purrdf_core::RdfDataset> {
    let mut rdf = RdfDatasetBuilder::new();
    let subject = rdf.intern_iri("https://example.org/rdf/subject").unwrap();
    let predicate = rdf.intern_iri("https://example.org/rdf/predicate").unwrap();
    let object = rdf.intern_iri("https://example.org/rdf/object").unwrap();
    rdf.push_quad(subject, predicate, object, None).unwrap();
    rdf.freeze().expect("valid RDF 1.2 dataset")
}

fn build_fixture(reverse_input: bool) -> Fixture {
    let rdf = source_dataset();
    let (source, source_bytes) = CertifiedEmbeddingSource::from_dataset(
        &rdf,
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::CANON_CORPUS_POLICY, None),
    )
    .expect("source pack");

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
        EmbeddingSource::Purrpck(&fixture.source_bytes),
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::CANON_CORPUS_POLICY, None),
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
        .f32_row(
            0,
            Exact::resolve().expect("the test thread runs the default float environment"),
        )
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

/// The canonical artifact, byte for byte; `PURREMB_REGENERATE_GOLDEN=1` rewrites the
/// golden from what the build produced instead.
#[test]
fn canonical_artifact_matches_checked_in_golden() {
    if let Err(error) = purrdf_testkit::golden::check_bytes(
        &golden_path(),
        &build_fixture(false).artifact_bytes,
        "PURREMB_REGENERATE_GOLDEN",
    ) {
        panic!("{error}");
    }
}

/// The header's `version` field and the trailer's, four little-endian bytes each.
const HEADER_VERSION_OFFSET: usize = 8;
const PACK_VERSION_OFFSET: usize = 8;
/// The pack header's `rdfc_digest` field: 32 bytes at offset 32.
const PACK_RDFC_DIGEST_OFFSET: usize = 32;

fn control() -> purrdf_core::CanonicalizationControl<'static> {
    purrdf_core::CanonicalizationControl::new(purrdf_core::CANON_CORPUS_POLICY, None)
}

/// The pack PurRDF 4.x wrote for the fixture dataset: the current pack with its
/// header version set to 1 and its digest replaced by the RDFC-1.0 SHA-256. Its
/// exact SHA-256 must equal the one the frozen 4.x artifact recorded.
fn version_one_pack() -> Vec<u8> {
    let rdf = source_dataset();
    let mut pack = purrdf_core::PackBuilder::build_bytes(&rdf, &mut control()).expect("pack");
    let legacy = purrdf_core::rdfc12::try_canonical_sha256_view(
        &rdf,
        purrdf_core::rdfc12::CanonHash::Sha256,
        &mut control(),
    )
    .expect("RDFC-1.0 digest");
    pack[PACK_VERSION_OFFSET..PACK_VERSION_OFFSET + 4].copy_from_slice(&1u32.to_le_bytes());
    pack[PACK_RDFC_DIGEST_OFFSET..PACK_RDFC_DIGEST_OFFSET + 32].copy_from_slice(legacy.as_bytes());
    pack
}

/// Format version 2 is what the writer emits, and a 4.x (version 1) artifact keeps
/// verifying: its source digest is recomputed with the RDFC-1.0 run that wrote it.
/// The same 4.x bytes relabelled version 2 are refused, because the version selects
/// the digest law rather than being ignored.
#[test]
fn version_one_embedding_verifies_by_its_rdfc12_digest_and_version_selects_the_law() {
    let current = build_fixture(false);
    let current_view = EmbeddingView::from_bytes(&current.artifact_bytes).expect("v2 opens");
    assert_eq!(current_view.format_version(), 2);

    let v1_bytes = version_one_golden();
    let mut v1 = EmbeddingView::from_bytes(&v1_bytes).expect("a 4.x artifact opens");
    assert_eq!(v1.format_version(), 1);
    verify_embedding(&mut v1).expect("a 4.x artifact verifies whole");
    let pack = version_one_pack();
    let report = verify_embedding_source(&v1, EmbeddingSource::Purrpck(&pack), &mut control())
        .expect("a 4.x source digest is recomputed by RDFC-1.0");
    assert_eq!(report.ordinals_checked(), 1);

    let mut relabelled = v1_bytes;
    let trailer = usize::try_from(purrdf_core::purremb_fixture::read_u64(&relabelled, 48))
        .expect("trailer offset");
    purrdf_core::purremb_fixture::put_u32(&mut relabelled, HEADER_VERSION_OFFSET, 2);
    purrdf_core::purremb_fixture::put_u32(&mut relabelled, trailer + 8, 2);
    purrdf_core::purremb_fixture::reseal(&mut relabelled, &[]);
    let mut v2 = EmbeddingView::from_bytes(&relabelled).expect("relabelled framing is valid");
    verify_embedding(&mut v2).expect("sections are untouched");
    let refused = verify_embedding_source(&v2, EmbeddingSource::Purrpck(&pack), &mut control())
        .expect_err("a RDFC-1.0 digest is not a version-2 digest");
    assert!(
        matches!(
            refused,
            purrdf::SourceAttachmentError::Embedding(purrdf_core::EmbeddingError::DigestMismatch {
                kind: purrdf_core::embedding::DigestKind::CertifiedRdf,
                ..
            })
        ),
        "{refused:?}"
    );
}
