// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The declared distance metric survives the round trip through a sealed artifact.
//!
//! The metric defines *comparison semantics* — which of two vectors ranks first — and
//! nothing in the stored scalars implies it, so a reader that could not recover it would
//! have to guess. A consumer guessing cosine for an artifact that declared squared
//! Euclidean returns a confidently wrong ordering, which is exactly what the declaration
//! exists to prevent; this asserts the declaration comes back verbatim, including the
//! opaque parameter bytes of a caller-defined extension metric.

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

/// Seal a one-row artifact declaring `metric`, and read the metric back off its family.
fn round_trip(metric: &DistanceMetric) -> DistanceMetric {
    let dataset = RdfDatasetBuilder::new().freeze().expect("empty dataset");
    let (source, _) = CertifiedPurrpckSource::from_dataset(&dataset).expect("source pack");
    let target = source.dataset_target(true).expect("dataset target");
    let target_id = target.id;
    let set = TargetSet::new(vec![target_id]).expect("target set");
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
        dtype: VectorDtype::F64,
        metric: metric.clone(),
        dimensionality: DimensionalityPolicy::fixed(2, PrefixPostprocessing::None)
            .expect("fixed dimensions"),
        extensions: Vec::new(),
    };
    let family = contract.derive().expect("family");
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
        stored_dimension: 2,
        rows: vec![MatrixRow::new(target_id, vec![1.0_f64, 2.0])],
        projections: vec![ProjectionSpec::derive(
            family.id,
            2,
            PrefixPostprocessing::None,
        )],
    };
    let mut builder = EmbeddingBuilder::from_typed_metadata(metadata);
    builder.add_f64_matrix(matrix);
    let encoded = builder.build().expect("sealed artifact");

    let mut view = EmbeddingView::from_bytes(&encoded.bytes).expect("structural view");
    verify_embedding(&mut view).expect("verified view");
    view.family(family.id)
        .expect("the artifact holds its own family")
        .metric()
        .expect("the declared metric decodes")
}

#[test]
fn every_declared_metric_reads_back_verbatim() {
    for metric in [
        DistanceMetric::Cosine,
        DistanceMetric::NegativeDot,
        DistanceMetric::SquaredEuclidean,
        DistanceMetric::Extension {
            identifier: "https://example.org/metric/mahalanobis".to_owned(),
            parameter_encoding: "application/cbor".to_owned(),
            parameters: vec![0xA1, 0x01, 0x02],
        },
    ] {
        assert_eq!(
            round_trip(&metric),
            metric,
            "the metric a family declares must come back byte for byte, including an \
             extension's opaque parameter bytes"
        );
    }
}

#[test]
fn two_metrics_are_two_families() {
    // The metric is identity-significant, so it cannot be read off the fixed-width family
    // record by accident: two contracts differing only in their metric derive different
    // `FamilyId`s. This is what makes the accessor a read of the CONTRACT rather than a
    // lookup that could silently answer for the wrong family.
    let base = |metric: DistanceMetric| EmbeddingFamilyContract {
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
        dtype: VectorDtype::F64,
        metric,
        dimensionality: DimensionalityPolicy::fixed(2, PrefixPostprocessing::None)
            .expect("fixed dimensions"),
        extensions: Vec::new(),
    };
    let cosine = base(DistanceMetric::Cosine).derive().expect("family");
    let euclidean = base(DistanceMetric::SquaredEuclidean)
        .derive()
        .expect("family");
    assert_ne!(cosine.id, euclidean.id);
}
