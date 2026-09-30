// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The sealed PURREMB embedding space the k-NN determinism tests query.
//!
//! One construction, shared by `knn_wasm_determinism.rs` and
//! `knn_wasm_reassociated.rs`: one `f64` row per `(local name, vector)` under [`EX`],
//! a family contract declared with the [`FX`] fixture identities, and the space opened
//! over the encoded artifact.

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use std::fmt::Display;

use purrdf_core::purremb_fixture::Identities;
use purrdf_core::{
    AppliedStage, CanonicalMetadataInput, CertifiedPurrpckSource, DimensionalityPolicy,
    DistanceMetric, EmbeddingBuilder, EmbeddingFamilyContract, MatrixInput, MatrixRow,
    PrefixPostprocessing, ProjectionSpec, RdfDatasetBuilder, RdfTermTarget, TargetSet, TermValue,
    VectorDtype,
};
use purrdf_sparql_eval::{EmbeddingSpace, KnnGuard};

/// The fixture's data namespace.
pub const EX: &str = "https://example.org/d/";

/// The fixture identities the family contract is declared with.
pub const FX: Identities = Identities::at("https://example.org/");

/// Encode `rows` as a sealed PURREMB artifact under `metric` and open it as a queryable
/// space.
pub fn space<L: Display>(rows: &[(L, Vec<f64>)], metric: DistanceMetric) -> EmbeddingSpace {
    let dimension = u32::try_from(rows[0].1.len()).expect("small");
    let dataset = RdfDatasetBuilder::new().freeze().expect("empty dataset");
    let (source, _) = CertifiedPurrpckSource::from_dataset(&dataset).expect("source pack");

    let mut targets = Vec::with_capacity(rows.len());
    let mut bindings = Vec::with_capacity(rows.len());
    for (local, _) in rows {
        let text = format!("{EX}{local}");
        let target = RdfTermTarget::Iri(text.clone())
            .into_target(true, None)
            .expect("term target");
        bindings.push((target.id, TermValue::iri(text)));
        targets.push(target);
    }
    let set = TargetSet::new(targets.iter().map(|t| t.id).collect()).expect("target set");
    let mut declared = targets;
    declared.push(source.dataset_target(true).expect("dataset target"));
    declared.sort_unstable_by_key(|target| target.id);

    let contract = EmbeddingFamilyContract {
        model: FX.artifact("model"),
        engine: FX.artifact("engine"),
        tokenizer: FX.artifact("tokenizer"),
        execution: FX.stage("execution"),
        subject_projection: FX.stage("projection"),
        preprocessing: AppliedStage::NotApplied,
        chunking: AppliedStage::NotApplied,
        pooling: FX.stage("pooling"),
        normalization: AppliedStage::NotApplied,
        truncation: AppliedStage::NotApplied,
        dtype: VectorDtype::F64,
        metric,
        dimensionality: DimensionalityPolicy::fixed(dimension, PrefixPostprocessing::None)
            .expect("fixed dimensions"),
        extensions: Vec::new(),
    };
    let family = contract.derive().expect("family");
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
    let encoded = builder.build().expect("encoded artifact");

    EmbeddingSpace::from_artifact(
        &encoded.bytes,
        set.id,
        vector_space,
        bindings,
        KnnGuard::new(64, 8).expect("positive bounds"),
    )
    .expect("the space opens")
}
