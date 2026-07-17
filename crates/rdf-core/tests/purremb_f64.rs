// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Lossless binary64 PURREMB round trip.

use purrdf_core::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, CanonicalMetadataInput,
    CertifiedPurrpckSource, ContentDigest, DimensionalityPolicy, DistanceMetric, EmbeddingBuilder,
    EmbeddingFamilyContract, EmbeddingView, MatrixInput, MatrixRow, PrefixPostprocessing,
    ProjectionSpec, RdfDatasetBuilder, StageImplementation, TargetSet, VectorDtype,
    verify_embedding,
};

fn artifact(name: &str) -> ArtifactIdentity {
    ArtifactIdentity::new(
        format!("https://example.org/{name}"),
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
            format!("https://example.org/{name}"),
            ContentDigest::of(name.as_bytes()),
            "application/octet-stream",
            vec![1],
        )
        .expect("stage"),
    )
}

#[test]
fn binary64_bits_and_signed_zero_round_trip() {
    let dataset = RdfDatasetBuilder::new().freeze().expect("empty dataset");
    let (source, _) = CertifiedPurrpckSource::from_dataset(&dataset).expect("source pack");
    let target = source.dataset_target(true).expect("dataset target");
    let target_id = target.id;
    let set = TargetSet::new(vec![target_id]).expect("target set");
    let contract = EmbeddingFamilyContract {
        model: artifact("model-f64"),
        engine: artifact("engine-f64"),
        tokenizer: artifact("tokenizer-f64"),
        execution: stage("execution-f64"),
        subject_projection: stage("projection-f64"),
        preprocessing: AppliedStage::NotApplied,
        chunking: AppliedStage::NotApplied,
        pooling: stage("pooling-f64"),
        normalization: AppliedStage::NotApplied,
        truncation: AppliedStage::NotApplied,
        dtype: VectorDtype::F64,
        metric: DistanceMetric::SquaredEuclidean,
        dimensionality: DimensionalityPolicy::fixed(3, PrefixPostprocessing::None)
            .expect("fixed dimensions"),
        extensions: Vec::new(),
    };
    let family = contract.derive().expect("family");
    let values = vec![-0.0f64, f64::MIN_POSITIVE, f64::MAX];
    let metadata = CanonicalMetadataInput {
        source,
        family_contracts: vec![contract],
        targets: vec![target],
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
        stored_dimension: 3,
        rows: vec![MatrixRow::new(target_id, values.clone())],
        projections: vec![ProjectionSpec::derive(
            family.id,
            3,
            PrefixPostprocessing::None,
        )],
    };
    let mut builder = EmbeddingBuilder::from_typed_metadata(metadata);
    builder.add_f64_matrix(matrix);
    let encoded = builder.build().expect("binary64 artifact");
    let mut view = EmbeddingView::from_bytes(&encoded.bytes).expect("structural view");
    verify_embedding(&mut view).expect("verified view");
    let actual = view
        .matrices()
        .next()
        .expect("matrix")
        .f64_row(0)
        .expect("f64 row")
        .map(|value| value.expect("finite f64").to_bits())
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        values.into_iter().map(f64::to_bits).collect::<Vec<_>>()
    );
}
