// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Exact-source, certified-RDF, and source-ordinal mismatch separation.

use purrdf_core::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, CanonicalMetadataInput,
    CertifiedPurrpckSource, ContentDigest, CorpusTarget, DimensionalityPolicy, DistanceMetric,
    EmbeddingBuilder, EmbeddingError, EmbeddingFamilyContract, EmbeddingTarget, EmbeddingView,
    MatrixInput, MatrixRow, PackView, PrefixPostprocessing, ProjectionSpec, RdfDatasetBuilder,
    RdfDatasetTarget, RdfTermTarget, SourceVerificationMode, StageImplementation, TargetSet,
    TermValue, VectorDtype, derive_artifact_root, verify_embedding, verify_embedding_source,
};
use sha2::{Digest as _, Sha256};

const HEADER_LENGTH: usize = 128;
const DIRECTORY_ENTRY_LENGTH: usize = 64;
const SECTION_SOURCE: u32 = 1;
const SECTION_TARGETS: u32 = 3;

struct Fixture {
    source_bytes: Vec<u8>,
    artifact_bytes: Vec<u8>,
    dataset_target: EmbeddingTarget,
    corpus_target: EmbeddingTarget,
}

fn artifact(name: &str) -> ArtifactIdentity {
    ArtifactIdentity::new(
        format!("https://example.org/source/{name}"),
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
            format!("https://example.org/source/{name}"),
            ContentDigest::of(name.as_bytes()),
            "application/octet-stream",
            vec![1],
        )
        .expect("stage"),
    )
}

fn contract() -> EmbeddingFamilyContract {
    EmbeddingFamilyContract {
        model: artifact("model"),
        engine: artifact("engine"),
        tokenizer: artifact("tokenizer"),
        execution: stage("execution"),
        subject_projection: stage("projection"),
        preprocessing: AppliedStage::NotApplied,
        chunking: AppliedStage::NotApplied,
        pooling: stage("pooling"),
        normalization: AppliedStage::NotApplied,
        truncation: AppliedStage::NotApplied,
        dtype: VectorDtype::F32,
        metric: DistanceMetric::Cosine,
        dimensionality: DimensionalityPolicy::fixed(1, PrefixPostprocessing::None)
            .expect("dimension"),
        extensions: Vec::new(),
    }
}

fn fixture(extra_targets: Vec<EmbeddingTarget>) -> Fixture {
    let mut dataset = RdfDatasetBuilder::new();
    let subject = dataset.intern_iri("https://example.org/source/s");
    let predicate = dataset.intern_iri("https://example.org/source/p");
    let object = dataset.intern_iri("https://example.org/source/o");
    dataset.push_quad(subject, predicate, object, None);
    let dataset = dataset.freeze().expect("dataset");
    let (source, source_bytes) =
        CertifiedPurrpckSource::from_dataset(&dataset).expect("source pack");
    let dataset_target = source.dataset_target(true).expect("dataset target");
    let corpus_target = CorpusTarget {
        manifest_digest: ContentDigest::of(b"manifest"),
        manifest_media_type: "application/example".into(),
        logical_id_digest: ContentDigest::of(b"corpus"),
    }
    .into_target(true)
    .expect("corpus target");
    let set = TargetSet::new(vec![corpus_target.id]).expect("target set");
    let contract = contract();
    let family = contract.derive().expect("family");
    let mut targets = vec![dataset_target.clone(), corpus_target.clone()];
    targets.extend(extra_targets);
    let metadata = CanonicalMetadataInput {
        source,
        family_contracts: vec![contract],
        targets,
        target_sets: vec![set.clone()],
        relations: Vec::new(),
        token_spans: Vec::new(),
        external_bindings: Vec::new(),
        indexes: Vec::new(),
        extensions: Vec::new(),
    };
    let matrix = MatrixInput {
        family_id: family.id,
        target_set_id: set.id,
        stored_dimension: 1,
        rows: vec![MatrixRow::new(corpus_target.id, vec![1.0])],
        projections: vec![ProjectionSpec::derive(
            family.id,
            1,
            PrefixPostprocessing::None,
        )],
    };
    let mut builder = EmbeddingBuilder::from_typed_metadata(metadata);
    builder.add_f32_matrix(matrix);
    Fixture {
        source_bytes,
        artifact_bytes: builder.build().expect("artifact").bytes,
        dataset_target,
        corpus_target,
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("u32"))
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().expect("u64"))
}

fn directory_entry(bytes: &[u8], kind: u32) -> usize {
    let count = read_u32(bytes, 20) as usize;
    (0..count)
        .map(|index| HEADER_LENGTH + index * DIRECTORY_ENTRY_LENGTH)
        .find(|offset| read_u32(bytes, *offset) == kind && read_u32(bytes, *offset + 8) == 0)
        .expect("directory entry")
}

fn section_span(bytes: &[u8], kind: u32) -> (usize, usize) {
    let entry = directory_entry(bytes, kind);
    (
        read_u64(bytes, entry + 16) as usize,
        read_u64(bytes, entry + 24) as usize,
    )
}

fn reseal(bytes: &mut [u8], kinds: &[u32]) {
    for &kind in kinds {
        let entry = directory_entry(bytes, kind);
        let (offset, length) = section_span(bytes, kind);
        let digest: [u8; 32] = Sha256::digest(&bytes[offset..offset + length]).into();
        bytes[entry + 32..entry + 64].copy_from_slice(&digest);
    }
    let directory_end = HEADER_LENGTH + read_u32(bytes, 20) as usize * DIRECTORY_ENTRY_LENGTH;
    let mut header = [0u8; HEADER_LENGTH];
    header.copy_from_slice(&bytes[..HEADER_LENGTH]);
    header[64..96].fill(0);
    let root = derive_artifact_root(&header, &bytes[HEADER_LENGTH..directory_end]);
    bytes[64..96].copy_from_slice(root.as_bytes());
    let trailer = read_u64(bytes, 48) as usize;
    bytes[trailer + 24..trailer + 56].copy_from_slice(root.as_bytes());
}

#[test]
fn exact_and_certified_source_mismatches_are_distinct() {
    let fixture = fixture(Vec::new());
    let mut view = EmbeddingView::from_bytes(&fixture.artifact_bytes).expect("view");
    verify_embedding(&mut view).expect("artifact integrity");
    verify_embedding_source(&view, &fixture.source_bytes, SourceVerificationMode::Exact)
        .expect("exact source");
    verify_embedding_source(
        &view,
        &fixture.source_bytes,
        SourceVerificationMode::Certified,
    )
    .expect("certified source");

    let short = &fixture.source_bytes[..fixture.source_bytes.len() - 1];
    assert!(matches!(
        verify_embedding_source(&view, short, SourceVerificationMode::Exact),
        Err(EmbeddingError::SourceLengthMismatch { .. })
    ));
    let mut changed = fixture.source_bytes.clone();
    changed[0] ^= 1;
    assert!(matches!(
        verify_embedding_source(&view, &changed, SourceVerificationMode::Exact),
        Err(EmbeddingError::DigestMismatch { .. })
    ));
}

#[test]
fn certified_mode_detects_a_wrong_rdf_claim_after_exact_mode_passes() {
    let fixture = fixture(Vec::new());
    let old_order = fixture.dataset_target.id < fixture.corpus_target.id;
    let (wrong_digest, replacement) = (1u32..=1024)
        .map(|counter| {
            let wrong = ContentDigest::of(&counter.to_le_bytes());
            let target = RdfDatasetTarget {
                rdfc_digest: *wrong.as_bytes(),
            }
            .into_target(true)
            .expect("replacement dataset target");
            (wrong, target)
        })
        .find(|(_, target)| (target.id < fixture.corpus_target.id) == old_order)
        .expect("ordering-preserving replacement");

    let mut artifact = fixture.artifact_bytes.clone();
    let (source_offset, _) = section_span(&artifact, SECTION_SOURCE);
    artifact[source_offset + 56..source_offset + 88].copy_from_slice(wrong_digest.as_bytes());
    artifact[source_offset + 88..source_offset + 120].copy_from_slice(replacement.id.as_bytes());

    let (targets_offset, _) = section_span(&artifact, SECTION_TARGETS);
    let target_count = read_u64(&artifact, targets_offset + 8) as usize;
    let record = (0..target_count)
        .map(|index| targets_offset + 64 + index * 96)
        .find(|offset| artifact[*offset..*offset + 32] == fixture.dataset_target.id.as_bytes()[..])
        .expect("dataset target record");
    artifact[record..record + 32].copy_from_slice(replacement.id.as_bytes());
    artifact[record + 32..record + 64].copy_from_slice(replacement.identity_digest.as_bytes());
    let identity_offset = read_u64(&artifact, record + 72) as usize;
    let identity_length = read_u64(&artifact, record + 80) as usize;
    let replacement_identity = replacement
        .canonical_identity
        .as_deref()
        .expect("retained replacement identity");
    assert_eq!(replacement_identity.len(), identity_length);
    artifact[targets_offset + identity_offset..targets_offset + identity_offset + identity_length]
        .copy_from_slice(replacement_identity);
    reseal(&mut artifact, &[SECTION_SOURCE, SECTION_TARGETS]);

    let mut view = EmbeddingView::from_bytes(&artifact).expect("self-consistent forged claim");
    verify_embedding(&mut view).expect("artifact is internally self-consistent");
    verify_embedding_source(&view, &fixture.source_bytes, SourceVerificationMode::Exact)
        .expect("exact bytes still match");
    assert!(matches!(
        verify_embedding_source(
            &view,
            &fixture.source_bytes,
            SourceVerificationMode::Certified,
        ),
        Err(EmbeddingError::DigestMismatch { .. })
    ));
}

#[test]
fn certified_mode_rejects_a_stale_source_ordinal() {
    let base = fixture(Vec::new());
    let pack = PackView::from_bytes(&base.source_bytes).expect("pack");
    let subject = pack
        .dict()
        .id_by_value(&TermValue::Iri("https://example.org/source/s".into()))
        .expect("subject ordinal");
    let object = pack
        .dict()
        .id_by_value(&TermValue::Iri("https://example.org/source/o".into()))
        .expect("object ordinal");
    assert_ne!(subject, object);
    let stale = RdfTermTarget::Iri("https://example.org/source/s".into())
        .into_target(true, Some(object))
        .expect("stale ordinal target");
    let fixture = fixture(vec![stale]);
    let mut view = EmbeddingView::from_bytes(&fixture.artifact_bytes).expect("view");
    verify_embedding(&mut view).expect("artifact integrity");
    assert!(matches!(
        verify_embedding_source(
            &view,
            &fixture.source_bytes,
            SourceVerificationMode::Certified,
        ),
        Err(EmbeddingError::OrdinalMismatch { .. })
    ));
}
