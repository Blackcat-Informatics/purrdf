// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Exact external bindings and opaque inline/detached index guards.

use purrdf_core::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, CanonicalMetadataInput,
    CertifiedPurrpckSource, ContentDigest, DerivedIndex, DimensionalityPolicy, DistanceMetric,
    EffectivePrefix, EmbeddingBuilder, EmbeddingFamily, EmbeddingFamilyContract, EmbeddingTarget,
    EmbeddingView, ExtensionTarget, ExternalBinding, ExternalBindingContract, ExternalScope,
    IndexBuildDeterminism, IndexCoordinates, IndexGuardContract, IndexLossContract,
    IndexPayloadStorage, IndexStorage, IndexUseRole, MatrixCommitment, MatrixInput, MatrixRow,
    PURREMB_HEADER_LENGTH, PrefixPostprocessing, ProjectionCommitment, ProjectionSpec,
    RdfDatasetBuilder, SECTION_CONTRACTS, SECTION_EXTERNAL_BINDINGS, SECTION_INDEX_GUARDS,
    SECTION_INDEX_PAYLOAD, SECTION_TARGETS, StageImplementation, TargetSet, VectorDtype,
    derive_artifact_root, derive_matrix_content_digest, derive_matrix_id,
    derive_projection_content_digest, derive_projection_id, verify_embedding,
    verify_external_artifact, verify_external_pack,
};
use sha2::{Digest as _, Sha256};

const DIRECTORY_ENTRY_LENGTH: usize = 64;

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("u32 field"))
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().expect("u64 field"))
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn directory_entry(bytes: &[u8], kind: u32, instance: u32) -> usize {
    let count = usize::try_from(read_u32(bytes, 20)).expect("section count");
    (0..count)
        .map(|index| PURREMB_HEADER_LENGTH as usize + index * DIRECTORY_ENTRY_LENGTH)
        .find(|offset| read_u32(bytes, *offset) == kind && read_u32(bytes, *offset + 8) == instance)
        .expect("section directory entry")
}

fn section_span(bytes: &[u8], kind: u32, instance: u32) -> (usize, usize) {
    let entry = directory_entry(bytes, kind, instance);
    (
        usize::try_from(read_u64(bytes, entry + 16)).expect("section offset"),
        usize::try_from(read_u64(bytes, entry + 24)).expect("section length"),
    )
}

fn reseal(bytes: &mut [u8], sections: &[(u32, u32)]) {
    for &(kind, instance) in sections {
        let entry = directory_entry(bytes, kind, instance);
        let (offset, length) = section_span(bytes, kind, instance);
        let digest: [u8; 32] = Sha256::digest(&bytes[offset..offset + length]).into();
        bytes[entry + 32..entry + 64].copy_from_slice(&digest);
    }
    let count = usize::try_from(read_u32(bytes, 20)).expect("section count");
    let directory_end = PURREMB_HEADER_LENGTH as usize + count * DIRECTORY_ENTRY_LENGTH;
    let mut header = [0u8; PURREMB_HEADER_LENGTH as usize];
    header.copy_from_slice(&bytes[..PURREMB_HEADER_LENGTH as usize]);
    header[64..96].fill(0);
    let root = derive_artifact_root(
        &header,
        &bytes[PURREMB_HEADER_LENGTH as usize..directory_end],
    );
    bytes[64..96].copy_from_slice(root.as_bytes());
    let trailer = usize::try_from(read_u64(bytes, 48)).expect("trailer offset");
    bytes[trailer + 24..trailer + 56].copy_from_slice(root.as_bytes());
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
        .expect("known guard bytes")
}

fn assert_reader_or_verifier_rejects(bytes: &[u8]) {
    if let Ok(mut view) = EmbeddingView::from_bytes(bytes) {
        assert!(verify_embedding(&mut view).is_err());
    }
}

#[derive(Clone)]
struct Context {
    source: CertifiedPurrpckSource,
    source_bytes: Vec<u8>,
    contract: EmbeddingFamilyContract,
    family: EmbeddingFamily,
    target: EmbeddingTarget,
    target_set: TargetSet,
    matrix: MatrixInput<f32>,
    commitment: MatrixCommitment,
}

fn artifact(name: &str) -> ArtifactIdentity {
    ArtifactIdentity::new(
        format!("https://example.org/index/{name}"),
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
            format!("https://example.org/index/{name}"),
            ContentDigest::of(name.as_bytes()),
            "application/cbor",
            vec![1, 2],
        )
        .expect("stage"),
    )
}

fn context() -> Context {
    context_with_metric(DistanceMetric::Cosine)
}

fn context_with_metric(metric: DistanceMetric) -> Context {
    let dataset = RdfDatasetBuilder::new().freeze().expect("empty dataset");
    let (source, source_bytes) =
        CertifiedPurrpckSource::from_dataset(&dataset).expect("source pack");
    let target = source.dataset_target(true).expect("dataset target");
    let target_set = TargetSet::new(vec![target.id]).expect("target set");
    let contract = EmbeddingFamilyContract {
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
        metric,
        dimensionality: DimensionalityPolicy::matryoshka(vec![
            EffectivePrefix {
                dimension: 2,
                postprocessing: PrefixPostprocessing::None,
            },
            EffectivePrefix {
                dimension: 4,
                postprocessing: PrefixPostprocessing::None,
            },
        ])
        .expect("Matryoshka family"),
        extensions: Vec::new(),
    };
    let family = contract.derive().expect("family");
    let values = [1.0f32, 2.0, 3.0, 4.0];
    let matrix_bytes = values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect::<Vec<_>>();
    let content_digest = derive_matrix_content_digest(1, 1, 4, &matrix_bytes);
    let matrix_id = derive_matrix_id(target_set.id, family.id, content_digest);
    let projections = family
        .spaces
        .iter()
        .map(|space| {
            let logical = &matrix_bytes[..usize::try_from(space.dimension).expect("dimension") * 4];
            let projection_content = derive_projection_content_digest(
                1,
                1,
                space.dimension,
                space.postprocessing.code(),
                logical,
            );
            ProjectionCommitment {
                projection_id: derive_projection_id(matrix_id, space.id, projection_content),
                content_digest: projection_content,
                vector_space_id: space.id,
                effective_dimension: space.dimension,
                postprocessing: space.postprocessing,
            }
        })
        .collect::<Vec<_>>();
    let commitment = MatrixCommitment {
        matrix_id,
        content_digest,
        target_set_id: target_set.id,
        family_id: family.id,
        dtype: VectorDtype::F32,
        row_count: 1,
        stored_dimension: 4,
        projections,
    };
    let matrix = MatrixInput {
        family_id: family.id,
        target_set_id: target_set.id,
        stored_dimension: 4,
        rows: vec![MatrixRow::new(target.id, values.to_vec())],
        projections: family
            .spaces
            .iter()
            .map(|space| {
                ProjectionSpec::derive(space.family_id, space.dimension, space.postprocessing)
            })
            .collect(),
    };
    Context {
        source,
        source_bytes,
        contract,
        family,
        target,
        target_set,
        matrix,
        commitment,
    }
}

fn binding_contract() -> ExternalBindingContract {
    ExternalBindingContract {
        role: "https://example.org/role/index-metadata".into(),
        media_type: "application/vnd.purrdf.pack".into(),
        stable_identifier: Some(b"fixture-source".to_vec()),
        revision: Some(b"v1".to_vec()),
        policy_reference: None,
    }
}

fn guard(
    role: IndexUseRole,
    certified_metadata_binding: Option<purrdf_core::ExternalBindingId>,
) -> IndexGuardContract {
    IndexGuardContract {
        implementation: artifact("hnsw-fixture"),
        parameter_encoding: "application/cbor".into(),
        parameters: vec![0xa1, 0x61, b'm', 0x10],
        loss: IndexLossContract {
            transforms_vectors: false,
            loss_encoding: None,
            loss_parameters: None,
        },
        use_role: role,
        payload_media_type: "application/vnd.example.hnsw".into(),
        certified_metadata_binding,
    }
}

fn coordinates(context: &Context, projection: usize) -> IndexCoordinates {
    let projection = context
        .commitment
        .projections
        .get(projection)
        .expect("projection");
    IndexCoordinates {
        source_exact_digest: context.source.source_exact_digest(),
        family_id: context.family.id,
        vector_space_id: projection.vector_space_id,
        matrix_id: context.commitment.matrix_id,
        projection_id: projection.projection_id,
        target_set_id: context.target_set.id,
        prefix_dimension: projection.effective_dimension,
    }
}

fn build(
    context: &Context,
    external_bindings: Vec<ExternalBinding>,
    indexes: Vec<DerivedIndex>,
) -> Vec<u8> {
    let metadata = CanonicalMetadataInput {
        source: context.source,
        family_contracts: vec![context.contract.clone()],
        targets: vec![context.target.clone()],
        target_sets: vec![context.target_set.clone()],
        relations: Vec::new(),
        token_spans: Vec::new(),
        external_bindings,
        indexes,
        extensions: Vec::new(),
    };
    let mut builder = EmbeddingBuilder::from_typed_metadata(metadata);
    builder.add_f32_matrix(context.matrix.clone());
    builder.build().expect("indexed artifact").bytes
}

#[test]
fn inline_coarse_and_full_indexes_bind_exact_prefixes() {
    let context = context();
    let certified = ExternalBinding::from_purrpck(
        ExternalScope::Source(context.source.source_exact_digest()),
        &context.source_bytes,
        &binding_contract(),
    )
    .expect("certified metadata binding");
    let coarse = DerivedIndex::new(
        coordinates(&context, 0),
        IndexPayloadStorage::Inline(b"coarse-prefix-index".to_vec()),
        IndexBuildDeterminism::Deterministic,
        &guard(IndexUseRole::CoarsePrefixRetrieval, Some(certified.id())),
    )
    .expect("coarse index");
    let full = DerivedIndex::new(
        coordinates(&context, 1),
        IndexPayloadStorage::Inline(b"full-prefix-index".to_vec()),
        IndexBuildDeterminism::Deterministic,
        &guard(IndexUseRole::FullPrefixReranking, Some(certified.id())),
    )
    .expect("full index");
    let bytes = build(&context, vec![certified], vec![coarse, full]);
    let mut view = EmbeddingView::from_bytes(&bytes).expect("indexed view");
    verify_embedding(&mut view).expect("verified indexes");
    assert_eq!(view.index_guard_count(), 2);
    let mut dimensions = view
        .index_guards()
        .map(|index| {
            assert_eq!(index.storage().expect("storage"), IndexStorage::Inline);
            assert!(index.payload_bytes().is_some());
            index.prefix_dimension()
        })
        .collect::<Vec<_>>();
    dimensions.sort_unstable();
    assert_eq!(dimensions, vec![2, 4]);

    let binding = view.external_bindings().next().expect("metadata binding");
    verify_external_pack(binding, &context.source_bytes).expect("external pack certification");
    assert!(verify_external_artifact(binding, b"wrong").is_err());
}

#[test]
fn embedded_metric_and_extension_target_digests_survive_outer_resealing() {
    let metric_parameters = b"metric-parameters-with-embedded-digest";
    let metric_context = context_with_metric(DistanceMetric::Extension {
        identifier: "https://example.org/metric/custom".into(),
        parameter_encoding: "application/example".into(),
        parameters: metric_parameters.to_vec(),
    });
    let mut metric_bytes = build(&metric_context, Vec::new(), Vec::new());
    let (section, length) = section_span(&metric_bytes, SECTION_CONTRACTS, 0);
    let relative = find_bytes(&metric_bytes[section..section + length], metric_parameters);
    metric_bytes[section + relative] ^= 1;
    reseal(&mut metric_bytes, &[(SECTION_CONTRACTS, 0)]);
    assert!(EmbeddingView::from_bytes(&metric_bytes).is_err());

    let context = context();
    let extension_payload = b"extension-target-payload-with-embedded-digest";
    let extension = ExtensionTarget {
        kind_identifier: "https://example.org/target/custom".into(),
        payload_encoding: "application/example".into(),
        payload: extension_payload.to_vec(),
    }
    .into_target(true)
    .expect("extension target");
    let metadata = CanonicalMetadataInput {
        source: context.source,
        family_contracts: vec![context.contract.clone()],
        targets: vec![context.target.clone(), extension],
        target_sets: vec![context.target_set.clone()],
        relations: Vec::new(),
        token_spans: Vec::new(),
        external_bindings: Vec::new(),
        indexes: Vec::new(),
        extensions: Vec::new(),
    };
    let mut builder = EmbeddingBuilder::from_typed_metadata(metadata);
    builder.add_f32_matrix(context.matrix);
    let mut target_bytes = builder.build().expect("extension artifact").bytes;
    let (section, length) = section_span(&target_bytes, SECTION_TARGETS, 0);
    let relative = find_bytes(&target_bytes[section..section + length], extension_payload);
    target_bytes[section + relative] ^= 1;
    reseal(&mut target_bytes, &[(SECTION_TARGETS, 0)]);
    assert!(EmbeddingView::from_bytes(&target_bytes).is_err());
}

#[test]
fn detached_index_requires_and_verifies_exact_external_payload() {
    let context = context();
    let payload = b"detached opaque index payload";
    let index = DerivedIndex::new(
        coordinates(&context, 0),
        IndexPayloadStorage::Detached {
            payload_sha256: ContentDigest::of(payload),
            payload_length: payload.len() as u64,
        },
        IndexBuildDeterminism::Nondeterministic,
        &guard(IndexUseRole::Generic, None),
    )
    .expect("detached index");
    let binding = ExternalBinding::from_bytes(
        ExternalScope::Index(index.id()),
        payload,
        &ExternalBindingContract {
            role: "https://example.org/role/detached-index".into(),
            media_type: "application/vnd.example.hnsw".into(),
            stable_identifier: None,
            revision: None,
            policy_reference: None,
        },
    )
    .expect("detached binding");
    let bytes = build(&context, vec![binding], vec![index]);
    let mut view = EmbeddingView::from_bytes(&bytes).expect("detached-index view");
    verify_embedding(&mut view).expect("verified detached guard");
    let guard = view.index_guards().next().expect("index guard");
    assert_eq!(guard.storage().expect("storage"), IndexStorage::Detached);
    assert!(guard.payload_bytes().is_none());
    let binding = view.external_bindings().next().expect("external binding");
    verify_external_artifact(binding, payload).expect("detached exact bytes");

    let mut stale_lookup = bytes;
    let (section, _) = section_span(&stale_lookup, SECTION_EXTERNAL_BINDINGS, 0);
    let lookup = usize::try_from(read_u64(&stale_lookup, section + 64)).expect("lookup offset");
    put_u32(&mut stale_lookup, section + lookup + 76, u32::MAX);
    reseal(&mut stale_lookup, &[(SECTION_EXTERNAL_BINDINGS, 0)]);
    assert!(EmbeddingView::from_bytes(&stale_lookup).is_err());
}

#[test]
fn stale_prefix_and_nondeterministic_inline_guards_are_rejected() {
    let context = context();
    let mut stale = coordinates(&context, 0);
    stale.prefix_dimension = 3;
    let stale = DerivedIndex::new(
        stale,
        IndexPayloadStorage::Inline(vec![1]),
        IndexBuildDeterminism::Deterministic,
        &guard(IndexUseRole::Generic, None),
    )
    .expect("internally consistent stale declaration");
    let metadata = CanonicalMetadataInput {
        source: context.source,
        family_contracts: vec![context.contract.clone()],
        targets: vec![context.target.clone()],
        target_sets: vec![context.target_set.clone()],
        relations: Vec::new(),
        token_spans: Vec::new(),
        external_bindings: Vec::new(),
        indexes: vec![stale],
        extensions: Vec::new(),
    };
    let mut builder = EmbeddingBuilder::from_typed_metadata(metadata);
    builder.add_f32_matrix(context.matrix.clone());
    assert!(builder.build().is_err());

    assert!(
        DerivedIndex::new(
            coordinates(&context, 0),
            IndexPayloadStorage::Inline(vec![1]),
            IndexBuildDeterminism::Nondeterministic,
            &guard(IndexUseRole::Generic, None),
        )
        .is_err()
    );
}

#[test]
fn every_stale_index_coordinate_and_contract_guard_is_rejected() {
    let context = context();
    let certified = ExternalBinding::from_purrpck(
        ExternalScope::Source(context.source.source_exact_digest()),
        &context.source_bytes,
        &binding_contract(),
    )
    .expect("certified metadata binding");
    let mut guarded = guard(IndexUseRole::Generic, Some(certified.id()));
    guarded.loss = IndexLossContract {
        transforms_vectors: true,
        loss_encoding: Some("application/vnd.example.loss-v1".into()),
        loss_parameters: Some(b"loss-parameters-v1".to_vec()),
    };
    let index = DerivedIndex::new(
        coordinates(&context, 0),
        IndexPayloadStorage::Inline(b"guarded-inline-index-v1".to_vec()),
        IndexBuildDeterminism::Deterministic,
        &guarded,
    )
    .expect("guarded index");
    let original = build(&context, vec![certified.clone()], vec![index]);
    let (guard_section, _) = section_span(&original, SECTION_INDEX_GUARDS, 0);
    let record = guard_section + 64;

    for field in [32usize, 64, 96, 128, 160, 192] {
        let mut corrupted = original.clone();
        corrupted[record + field] ^= 0x80;
        reseal(&mut corrupted, &[(SECTION_INDEX_GUARDS, 0)]);
        assert_reader_or_verifier_rejects(&corrupted);
    }

    let mut stale_prefix = original.clone();
    put_u32(&mut stale_prefix, record + 328, 3);
    reseal(&mut stale_prefix, &[(SECTION_INDEX_GUARDS, 0)]);
    assert_reader_or_verifier_rejects(&stale_prefix);

    let guard_offset = usize::try_from(read_u64(&original, record + 296)).expect("guard offset");
    let guard_length = usize::try_from(read_u64(&original, record + 304)).expect("guard length");
    let guard_start = guard_section + guard_offset;
    let guard_end = guard_start + guard_length;
    let guard_bytes = &original[guard_start..guard_end];
    for needle in [
        b"https://example.org/index/hnsw-fixture".as_slice(),
        [0xa1, 0x61, b'm', 0x10].as_slice(),
        b"application/vnd.example.loss-v1".as_slice(),
        b"loss-parameters-v1".as_slice(),
        certified.id().as_bytes().as_slice(),
    ] {
        let mut corrupted = original.clone();
        let position = guard_start + find_bytes(guard_bytes, needle);
        corrupted[position] ^= 1;
        reseal(&mut corrupted, &[(SECTION_INDEX_GUARDS, 0)]);
        if needle == [0xa1, 0x61, b'm', 0x10] || needle == b"loss-parameters-v1" {
            assert!(EmbeddingView::from_bytes(&corrupted).is_err());
        } else {
            assert_reader_or_verifier_rejects(&corrupted);
        }
    }

    let mut payload = original;
    let (payload_offset, _) = section_span(&payload, SECTION_INDEX_PAYLOAD, 1);
    payload[payload_offset] ^= 1;
    reseal(&mut payload, &[(SECTION_INDEX_PAYLOAD, 1)]);
    assert_reader_or_verifier_rejects(&payload);
}
