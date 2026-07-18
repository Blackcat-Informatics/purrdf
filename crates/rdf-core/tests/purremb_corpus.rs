// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Sharded external corpora, overlapping chunks, and extension behavior.

use purrdf_core::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, CanonicalMetadataInput,
    CertifiedPurrpckSource, ContentDigest, CorpusTarget, DimensionalityPolicy, DistanceMetric,
    DocumentTarget, EmbeddingBuilder, EmbeddingFamilyContract, EmbeddingView, ExtensionSection,
    MatrixInput, MatrixRow, PrefixPostprocessing, ProjectionSpec, RdfDatasetBuilder, RelationKind,
    SECTION_CRITICAL, SECTION_DERIVED, SECTION_EXTENSION_MIN, StageImplementation, TargetId,
    TargetRelation, TargetSet, TextChunkTarget, TokenSpan, VectorDtype, verify_embedding,
};

const EXTENSION_KIND: u32 = SECTION_EXTENSION_MIN + 0x43;
const EXTENSION_BYTES: &[u8] = b"opaque corpus partition evidence";

fn artifact(name: &str) -> ArtifactIdentity {
    ArtifactIdentity::new(
        format!("https://example.org/corpus/{name}"),
        "application/octet-stream",
        ContentDigest::of(name.as_bytes()),
        None,
        ArtifactIdentityKind::Single,
    )
    .expect("artifact")
}

fn stage(name: &str) -> AppliedStage {
    AppliedStage::Applied(
        StageImplementation::new(
            format!("https://example.org/corpus/{name}"),
            ContentDigest::of(name.as_bytes()),
            "application/cbor",
            vec![0xa1, 0x01, 0x01],
        )
        .expect("stage"),
    )
}

fn family(name: &str, chunking: &str) -> EmbeddingFamilyContract {
    EmbeddingFamilyContract {
        model: artifact(&format!("model-{name}")),
        engine: artifact("engine"),
        tokenizer: artifact(&format!("tokenizer-{name}")),
        execution: stage("execution"),
        subject_projection: stage("chunk-text"),
        preprocessing: stage("unicode-nfc"),
        chunking: stage(chunking),
        pooling: stage("mean-pooling"),
        normalization: AppliedStage::NotApplied,
        truncation: AppliedStage::NotApplied,
        dtype: VectorDtype::F32,
        metric: DistanceMetric::Cosine,
        dimensionality: DimensionalityPolicy::fixed(2, PrefixPostprocessing::None)
            .expect("fixed dimension"),
        extensions: Vec::new(),
    }
}

struct CorpusFixture {
    bytes: Vec<u8>,
    first_document: TargetId,
    family_a_chunking: purrdf_core::ChunkingContractId,
    family_b_chunking: purrdf_core::ChunkingContractId,
    overlapping_scalar_spans: [(u64, u64); 2],
}

fn build_fixture(extension_flags: u32) -> CorpusFixture {
    let dataset = RdfDatasetBuilder::new().freeze().expect("empty RDF source");
    let (source, _) = CertifiedPurrpckSource::from_dataset(&dataset).expect("source pack");
    let dataset_target = source.dataset_target(true).expect("dataset target");

    let corpus = CorpusTarget {
        manifest_digest: ContentDigest::of(b"two-shard manifest"),
        manifest_media_type: "application/vnd.example.corpus-manifest+cbor".into(),
        logical_id_digest: ContentDigest::of(b"corpus:sharded-fixture"),
    }
    .into_target(true)
    .expect("corpus target");
    let shard_a_bytes = "alpha βeta gamma delta".as_bytes();
    let shard_b_bytes = "epsilon ζeta eta".as_bytes();
    let shard_a_model = DocumentTarget::from_content(
        corpus.id,
        ContentDigest::of(b"shard:000001"),
        "text/plain;charset=utf-8",
        shard_a_bytes,
    )
    .expect("first document shard");
    let shard_b_model = DocumentTarget::from_content(
        corpus.id,
        ContentDigest::of(b"shard:000002"),
        "text/plain;charset=utf-8",
        shard_b_bytes,
    )
    .expect("second document shard");
    shard_a_model
        .verify_content(shard_a_bytes)
        .expect("first shard content");
    shard_b_model
        .verify_content(shard_b_bytes)
        .expect("second shard content");
    assert!(shard_a_model.verify_content(b"alpha beta").is_err());
    let shard_a = shard_a_model
        .clone()
        .into_target(true)
        .expect("first document target");
    let shard_b = shard_b_model
        .clone()
        .into_target(true)
        .expect("second document target");

    let contract_a = family("a", "overlap-11-bytes-stride-6");
    let contract_b = family("b", "whole-document-v2");
    let family_a = contract_a.derive().expect("family A");
    let family_b = contract_b.derive().expect("family B");
    assert_ne!(family_a.chunking_id, family_b.chunking_id);

    let overlap_a_model =
        TextChunkTarget::from_document(shard_a.id, family_a.chunking_id, shard_a_bytes, 0, 11)
            .expect("first UTF-8 chunk");
    let overlap_b_model =
        TextChunkTarget::from_document(shard_a.id, family_a.chunking_id, shard_a_bytes, 6, 17)
            .expect("overlapping UTF-8 chunk");
    let shard_b_chunk_model = TextChunkTarget::from_document(
        shard_b.id,
        family_a.chunking_id,
        shard_b_bytes,
        0,
        shard_b_model.byte_length,
    )
    .expect("second-shard chunk");
    let alternate_chunk_model = TextChunkTarget::from_document(
        shard_a.id,
        family_b.chunking_id,
        shard_a_bytes,
        0,
        shard_a_model.byte_length,
    )
    .expect("alternate chunking contract");
    for chunk in [
        &overlap_a_model,
        &overlap_b_model,
        &shard_b_chunk_model,
        &alternate_chunk_model,
    ] {
        let document = if chunk.document_id == shard_a.id {
            shard_a_bytes
        } else {
            shard_b_bytes
        };
        chunk.verify_document(document).expect("chunk content");
    }
    let overlapping_scalar_spans = [
        (overlap_a_model.scalar_start, overlap_a_model.scalar_end),
        (overlap_b_model.scalar_start, overlap_b_model.scalar_end),
    ];
    assert!(overlapping_scalar_spans[1].0 < overlapping_scalar_spans[0].1);

    let overlap_a = overlap_a_model
        .into_target(true)
        .expect("first chunk target");
    let overlap_b = overlap_b_model
        .into_target(true)
        .expect("overlapping chunk target");
    let shard_b_chunk = shard_b_chunk_model
        .into_target(true)
        .expect("second-shard chunk target");
    let alternate_chunk = alternate_chunk_model
        .into_target(true)
        .expect("alternate chunk target");

    let set_a = TargetSet::new(vec![overlap_a.id, overlap_b.id, shard_b_chunk.id])
        .expect("family A target set");
    let set_b = TargetSet::new(vec![alternate_chunk.id]).expect("family B target set");
    let relations = vec![
        TargetRelation::builtin(corpus.id, RelationKind::CorpusDocument, shard_a.id),
        TargetRelation::builtin(corpus.id, RelationKind::CorpusDocument, shard_b.id),
        TargetRelation::builtin(shard_a.id, RelationKind::DocumentChunk, overlap_a.id),
        TargetRelation::builtin(shard_a.id, RelationKind::DocumentChunk, overlap_b.id),
        TargetRelation::builtin(shard_a.id, RelationKind::DocumentChunk, alternate_chunk.id),
        TargetRelation::builtin(shard_b.id, RelationKind::DocumentChunk, shard_b_chunk.id),
    ];
    let token_spans = vec![
        TokenSpan {
            family_id: family_a.id,
            target_id: overlap_a.id,
            token_start: 0,
            token_end: 2,
            model_input_token_count: 2,
            left_truncated: false,
            right_truncated: false,
            includes_special_tokens: false,
        },
        TokenSpan {
            family_id: family_a.id,
            target_id: overlap_b.id,
            token_start: 1,
            token_end: 3,
            model_input_token_count: 2,
            left_truncated: false,
            right_truncated: false,
            includes_special_tokens: false,
        },
        TokenSpan {
            family_id: family_a.id,
            target_id: shard_b_chunk.id,
            token_start: 0,
            token_end: 3,
            model_input_token_count: 3,
            left_truncated: false,
            right_truncated: false,
            includes_special_tokens: false,
        },
        TokenSpan {
            family_id: family_b.id,
            target_id: alternate_chunk.id,
            token_start: 0,
            token_end: 4,
            model_input_token_count: 4,
            left_truncated: false,
            right_truncated: false,
            includes_special_tokens: false,
        },
    ];
    let metadata = CanonicalMetadataInput {
        source,
        family_contracts: vec![contract_b, contract_a],
        targets: vec![
            alternate_chunk.clone(),
            shard_b_chunk.clone(),
            overlap_b.clone(),
            overlap_a.clone(),
            shard_b,
            shard_a.clone(),
            corpus,
            dataset_target,
        ],
        target_sets: vec![set_b.clone(), set_a.clone()],
        relations,
        token_spans,
        external_bindings: Vec::new(),
        indexes: Vec::new(),
        extensions: vec![
            ExtensionSection::new(EXTENSION_KIND, 7, extension_flags, EXTENSION_BYTES.to_vec())
                .expect("extension"),
        ],
    };
    let matrix_a = MatrixInput {
        family_id: family_a.id,
        target_set_id: set_a.id,
        stored_dimension: 2,
        rows: vec![
            MatrixRow::new(overlap_b.id, vec![2.0, 1.0]),
            MatrixRow::new(shard_b_chunk.id, vec![3.0, 1.0]),
            MatrixRow::new(overlap_a.id, vec![1.0, 1.0]),
        ],
        projections: vec![ProjectionSpec::derive(
            family_a.id,
            2,
            PrefixPostprocessing::None,
        )],
    };
    let matrix_b = MatrixInput {
        family_id: family_b.id,
        target_set_id: set_b.id,
        stored_dimension: 2,
        rows: vec![MatrixRow::new(alternate_chunk.id, vec![4.0, 1.0])],
        projections: vec![ProjectionSpec::derive(
            family_b.id,
            2,
            PrefixPostprocessing::None,
        )],
    };
    let mut builder = EmbeddingBuilder::from_typed_metadata(metadata);
    builder.add_f32_matrix(matrix_b);
    builder.add_f32_matrix(matrix_a);
    CorpusFixture {
        bytes: builder.build().expect("corpus PURREMB").bytes,
        first_document: shard_a.id,
        family_a_chunking: family_a.chunking_id,
        family_b_chunking: family_b.chunking_id,
        overlapping_scalar_spans,
    }
}

#[test]
fn sharded_corpus_with_overlapping_multi_contract_chunks_round_trips() {
    let fixture = build_fixture(SECTION_DERIVED);
    let mut view = EmbeddingView::from_bytes(&fixture.bytes).expect("structural corpus view");
    verify_embedding(&mut view).expect("verified corpus view");
    assert_eq!(view.family_count(), 2);
    assert_eq!(view.matrix_count(), 2);
    assert_eq!(view.target_count(), 8);
    assert_eq!(view.relation_count(), 6);
    assert_eq!(view.token_span_count(), 4);
    assert_eq!(view.relations_for(fixture.first_document).count(), 3);
    assert_ne!(fixture.family_a_chunking, fixture.family_b_chunking);
    assert!(fixture.overlapping_scalar_spans[1].0 < fixture.overlapping_scalar_spans[0].1);
    assert_eq!(
        view.section(EXTENSION_KIND, 7)
            .expect("retained extension")
            .bytes(),
        EXTENSION_BYTES
    );
    for text in ["alpha βeta gamma delta", "epsilon ζeta eta"] {
        assert!(
            !fixture
                .bytes
                .windows(text.len())
                .any(|window| window == text.as_bytes()),
            "external source text leaked into PURREMB"
        );
    }
}

#[test]
fn unknown_critical_extension_is_rejected_by_a_reader_without_its_contract() {
    let fixture = build_fixture(SECTION_CRITICAL);
    assert!(EmbeddingView::from_bytes(&fixture.bytes).is_err());
}
