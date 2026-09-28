// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The ONE definition of the PURREMB test fixtures: the artifact identities, applied
//! stages and family contracts every `.purremb` test and bench seals an artifact
//! under.
//!
//! This module exists for THIS crate's own tests and benches, exactly as the
//! `purrdf-rdf` crate's `gts_fixtures` does: a `tests/` integration target and a
//! `benches/` target are separate crates that can only share code through the
//! library, and a bench cannot `include!` a test file. Some twenty copies of the same
//! three helpers — an `artifact`, a `stage`, and a contract literal over them — each
//! differing in a base IRI, a media type or a parameter byte, were twenty definitions
//! of what "a fixture contract" means, and only one of them got updated when a field
//! was added. Here the fields that differed are the arguments.
//!
//! It is `#[doc(hidden)]` at the crate root: shipped, because the targets that
//! consume it are built from the same crate, but not public API. Nothing here is
//! part of any stability promise, and no shipping code path calls it.

use crate::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, ContentDigest, DimensionalityPolicy,
    DistanceMetric, EmbeddingFamilyContract, StageImplementation, VectorDtype,
};

/// The media type every fixture uses unless it says otherwise.
pub const OCTET_STREAM: &str = "application/octet-stream";

/// A single artifact identity at `{prefix}{name}` with the octet-stream media type,
/// no parameters, and a digest of `name` itself. `prefix` is everything before the
/// name, trailing slash included: `artifact("https://example.org/index/", "model")`.
#[must_use]
pub fn artifact(prefix: &str, name: &str) -> ArtifactIdentity {
    artifact_with(prefix, name, OCTET_STREAM, None)
}

/// [`artifact`] with an explicit media type and optional parameter bytes.
#[must_use]
pub fn artifact_with(
    prefix: &str,
    name: &str,
    media_type: &str,
    parameters: Option<&[u8]>,
) -> ArtifactIdentity {
    ArtifactIdentity::new(
        format!("{prefix}{name}"),
        media_type,
        ContentDigest::of(name.as_bytes()),
        parameters.map(<[u8]>::to_vec),
        ArtifactIdentityKind::Single,
    )
    .expect("a fixture artifact identity is well-formed")
}

/// An applied stage at `{prefix}{name}` with a digest of `name` itself, the given
/// parameter media type and parameter bytes.
#[must_use]
pub fn stage(prefix: &str, name: &str, media_type: &str, parameters: Vec<u8>) -> AppliedStage {
    AppliedStage::Applied(
        StageImplementation::new(
            format!("{prefix}{name}"),
            ContentDigest::of(name.as_bytes()),
            media_type,
            parameters,
        )
        .expect("a fixture stage is well-formed"),
    )
}

/// Everything that differed between the fixture contracts: the base IRI, the name
/// of each artifact and stage, which optional stages are applied, the media types
/// and parameters, and the three typed fields. [`ContractSpec::new`] fills the
/// names and media types the plain fixtures use; a test overrides what it needs
/// with struct-update syntax.
#[derive(Clone, Debug)]
pub struct ContractSpec<'a> {
    /// Everything before each artifact's and stage's name in its IRI.
    pub prefix: &'a str,
    /// The model artifact's name.
    pub model: &'a str,
    /// The engine artifact's name.
    pub engine: &'a str,
    /// The tokenizer artifact's name.
    pub tokenizer: &'a str,
    /// The execution stage's name.
    pub execution: &'a str,
    /// The subject-projection stage's name.
    pub subject_projection: &'a str,
    /// The preprocessing stage's name, or `None` for a stage not applied.
    pub preprocessing: Option<&'a str>,
    /// The chunking stage's name, or `None` for a stage not applied.
    pub chunking: Option<&'a str>,
    /// The pooling stage's name.
    pub pooling: &'a str,
    /// The normalization stage's name, or `None` for a stage not applied.
    pub normalization: Option<&'a str>,
    /// The truncation stage's name, or `None` for a stage not applied.
    pub truncation: Option<&'a str>,
    /// The media type of every artifact.
    pub artifact_media_type: &'a str,
    /// The parameter bytes of every artifact, if any.
    pub artifact_parameters: Option<&'a [u8]>,
    /// The parameter media type of every applied stage.
    pub stage_media_type: &'a str,
    /// The parameter bytes of every applied stage.
    pub stage_parameters: &'a [u8],
    /// The stored scalar type.
    pub dtype: VectorDtype,
    /// The distance semantics.
    pub metric: DistanceMetric,
    /// The dimension declarations.
    pub dimensionality: DimensionalityPolicy,
}

impl<'a> ContractSpec<'a> {
    /// The plain fixture: artifacts `model`, `engine` and `tokenizer`, stages
    /// `execution`, `projection` and `pooling`, no optional stage applied, every
    /// media type octet-stream, no artifact parameters, and the one stage parameter
    /// byte `1`.
    #[must_use]
    pub const fn new(
        prefix: &'a str,
        dtype: VectorDtype,
        metric: DistanceMetric,
        dimensionality: DimensionalityPolicy,
    ) -> Self {
        Self {
            prefix,
            model: "model",
            engine: "engine",
            tokenizer: "tokenizer",
            execution: "execution",
            subject_projection: "projection",
            preprocessing: None,
            chunking: None,
            pooling: "pooling",
            normalization: None,
            truncation: None,
            artifact_media_type: OCTET_STREAM,
            artifact_parameters: None,
            stage_media_type: OCTET_STREAM,
            stage_parameters: &[1],
            dtype,
            metric,
            dimensionality,
        }
    }

    fn artifact(&self, name: &str) -> ArtifactIdentity {
        artifact_with(
            self.prefix,
            name,
            self.artifact_media_type,
            self.artifact_parameters,
        )
    }

    fn stage(&self, name: &str) -> AppliedStage {
        stage(
            self.prefix,
            name,
            self.stage_media_type,
            self.stage_parameters.to_vec(),
        )
    }

    fn optional_stage(&self, name: Option<&str>) -> AppliedStage {
        name.map_or(AppliedStage::NotApplied, |name| self.stage(name))
    }
}

/// The family contract `spec` describes, with no extensions.
#[must_use]
pub fn contract(spec: &ContractSpec<'_>) -> EmbeddingFamilyContract {
    EmbeddingFamilyContract {
        model: spec.artifact(spec.model),
        engine: spec.artifact(spec.engine),
        tokenizer: spec.artifact(spec.tokenizer),
        execution: spec.stage(spec.execution),
        subject_projection: spec.stage(spec.subject_projection),
        preprocessing: spec.optional_stage(spec.preprocessing),
        chunking: spec.optional_stage(spec.chunking),
        pooling: spec.stage(spec.pooling),
        normalization: spec.optional_stage(spec.normalization),
        truncation: spec.optional_stage(spec.truncation),
        dtype: spec.dtype,
        metric: spec.metric.clone(),
        dimensionality: spec.dimensionality.clone(),
        extensions: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PrefixPostprocessing;

    fn fixed(dimension: u32) -> DimensionalityPolicy {
        DimensionalityPolicy::fixed(dimension, PrefixPostprocessing::None)
            .expect("a fixed dimensionality")
    }

    #[test]
    fn the_plain_contract_derives_a_family_and_the_prefix_reaches_every_iri() {
        let spec = ContractSpec::new(
            "https://example.org/fixture/",
            VectorDtype::F32,
            DistanceMetric::Cosine,
            fixed(2),
        );
        let contract = contract(&spec);
        contract.derive().expect("the plain fixture derives");
        assert_eq!(
            contract.model,
            artifact("https://example.org/fixture/", "model")
        );
        assert_eq!(
            contract.pooling,
            stage(
                "https://example.org/fixture/",
                "pooling",
                OCTET_STREAM,
                vec![1]
            )
        );
        assert_eq!(contract.preprocessing, AppliedStage::NotApplied);
        assert_eq!(contract.chunking, AppliedStage::NotApplied);
        assert_eq!(contract.normalization, AppliedStage::NotApplied);
        assert_eq!(contract.truncation, AppliedStage::NotApplied);
        assert_eq!(contract.dtype, VectorDtype::F32);
        assert_eq!(contract.metric, DistanceMetric::Cosine);
        assert_eq!(contract.extensions, Vec::new());
    }

    #[test]
    fn every_overridable_field_reaches_the_contract() {
        let spec = ContractSpec {
            model: "model-x",
            engine: "engine-x",
            tokenizer: "tokenizer-x",
            execution: "execution-x",
            subject_projection: "chunk-text",
            preprocessing: Some("unicode-nfc"),
            chunking: Some("overlapping-utf8-chunks"),
            pooling: "mean-pooling",
            normalization: Some("l2"),
            truncation: Some("cut"),
            artifact_media_type: "application/example",
            artifact_parameters: Some(b"fixture-v1"),
            stage_media_type: "application/cbor",
            stage_parameters: &[0xa1, 0x61, b'v', 0x01],
            ..ContractSpec::new(
                "https://example.org/corpus/",
                VectorDtype::F64,
                DistanceMetric::SquaredEuclidean,
                fixed(3),
            )
        };
        let contract = contract(&spec);
        contract.derive().expect("the overridden fixture derives");
        assert_eq!(
            contract.model,
            artifact_with(
                "https://example.org/corpus/",
                "model-x",
                "application/example",
                Some(b"fixture-v1"),
            )
        );
        assert_eq!(
            contract.chunking,
            stage(
                "https://example.org/corpus/",
                "overlapping-utf8-chunks",
                "application/cbor",
                vec![0xa1, 0x61, b'v', 0x01],
            )
        );
        assert_ne!(contract.normalization, AppliedStage::NotApplied);
        assert_ne!(contract.truncation, AppliedStage::NotApplied);
        assert_eq!(contract.dtype, VectorDtype::F64);
        assert_eq!(contract.metric, DistanceMetric::SquaredEuclidean);
    }

    #[test]
    fn distinct_prefixes_and_names_give_distinct_identities() {
        let a = artifact("https://example.org/", "model");
        let b = artifact("https://example.org/index/", "model");
        let c = artifact("https://example.org/", "engine");
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(
            stage("https://example.org/", "s", OCTET_STREAM, vec![1]),
            stage("https://example.org/", "s", OCTET_STREAM, vec![2])
        );
    }
}
