// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Typed canonical encoders for PURREMB metadata sections.

use super::error::SourceAttachmentError;
use super::source::{EmbeddingSource, canonical_source};
use super::writer::CanonicalMetadataSections;
use purrdf_core::embedding::{
    DerivedIndex, EmbeddingError, EmbeddingFamilyContract, EmbeddingSourceFormat, EmbeddingTarget,
    ExtensionSection, ExternalBinding, MatrixCommitment, MetadataCatalogInput, RdfDatasetTarget,
    RdfcDigest, TargetId, TargetRelation, TargetSet, TokenSpan,
};
use purrdf_core::embedding::{put_u32, put_u64};
use purrdf_core::{ContentDigest, PackBuilder, RdfDataset, verify_pack_bytes};

const SOURCE_LENGTH: usize = 128;

/// Independently certified attachment to exact pack bytes or actual Columnar files.
///
/// The fields are private so the public construction path cannot turn an
/// arbitrary digest claim into certified source metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertifiedEmbeddingSource {
    source_format: EmbeddingSourceFormat,
    source_length: u64,
    source_exact_digest: ContentDigest,
    certified_rdf_digest: RdfcDigest,
    dataset_target_id: TargetId,
}

impl CertifiedEmbeddingSource {
    /// Independently certify the actual native source; no digest claims are accepted.
    pub fn certify(source: EmbeddingSource<'_>) -> Result<Self, SourceAttachmentError> {
        let (source_format, source_length, source_exact_digest, certified_rdf_digest) = match source
        {
            EmbeddingSource::Purrpck(bytes) => {
                let proof = verify_pack_bytes(bytes).map_err(EmbeddingError::source_pack)?;
                (
                    EmbeddingSourceFormat::PurrpckV1,
                    u64::try_from(bytes.len())
                        .map_err(|_| EmbeddingError::ArithmeticOverflow("source byte length"))?,
                    proof.exact_digest(),
                    RdfcDigest::from_raw(*proof.rdf_digest().as_bytes()),
                )
            }
            EmbeddingSource::Columnar(files) => {
                // A certificate is constructed only after every table and blob passed.
                let read = purrdf_columnar::read(files)?;
                let (rdf_digest, _) = canonical_source(&read.dataset)?;
                let manifest = purrdf_columnar::source_manifest(files)?;
                (
                    EmbeddingSourceFormat::ColumnarV1,
                    u64::try_from(manifest.len()).map_err(|_| {
                        EmbeddingError::ArithmeticOverflow("source manifest length")
                    })?,
                    ContentDigest::of(&manifest),
                    rdf_digest,
                )
            }
        };
        let dataset_target = RdfDatasetTarget {
            rdfc_digest: *certified_rdf_digest.as_bytes(),
        }
        .into_target(false)?;
        Ok(Self {
            source_format,
            source_length,
            source_exact_digest,
            certified_rdf_digest,
            dataset_target_id: dataset_target.id,
        })
    }

    /// The carrier that was independently certified.
    #[must_use]
    pub const fn source_format(self) -> EmbeddingSourceFormat {
        self.source_format
    }

    /// Builds and certifies a `.purrpck` directly from a frozen RDF dataset.
    pub fn from_dataset(dataset: &RdfDataset) -> Result<(Self, Vec<u8>), SourceAttachmentError> {
        let bytes = PackBuilder::build_bytes(dataset).map_err(EmbeddingError::source_pack)?;
        let source = Self::certify(EmbeddingSource::Purrpck(&bytes))?;
        Ok((source, bytes))
    }

    /// Exact source SHA-256 duplicated into the PURREMB header.
    #[must_use]
    pub const fn source_exact_digest(self) -> ContentDigest {
        self.source_exact_digest
    }

    /// Independently verified RDFC SHA-256.
    #[must_use]
    pub const fn certified_rdf_digest(self) -> RdfcDigest {
        self.certified_rdf_digest
    }

    /// Canonical RDF-dataset target referenced by the `SOURCE` section.
    #[must_use]
    pub const fn dataset_target_id(self) -> TargetId {
        self.dataset_target_id
    }

    /// Reconstructs the dataset target for insertion into the target catalog.
    pub fn dataset_target(self, retain_identity: bool) -> Result<EmbeddingTarget, EmbeddingError> {
        RdfDatasetTarget {
            rdfc_digest: *self.certified_rdf_digest.as_bytes(),
        }
        .into_target(retain_identity)
    }

    /// Encodes the exact fixed-size `SOURCE` section.
    #[must_use]
    pub fn encode(self) -> [u8; SOURCE_LENGTH] {
        let mut output = [0u8; SOURCE_LENGTH];
        put_u32(&mut output, 0, 1);
        put_u32(&mut output, 4, 1);
        put_u64(&mut output, 8, self.source_length);
        put_u32(&mut output, 16, self.source_format.code());
        output[24..56].copy_from_slice(self.source_exact_digest.as_bytes());
        output[56..88].copy_from_slice(self.certified_rdf_digest.as_bytes());
        output[88..120].copy_from_slice(self.dataset_target_id.as_bytes());
        output
    }
}

/// Complete typed source attachment and canonical metadata input.
#[derive(Debug, Clone)]
pub struct CanonicalMetadataInput {
    /// Exact source attachment created through independent native certification.
    pub source: CertifiedEmbeddingSource,
    /// Complete generation contracts; logical duplicates are collapsed.
    pub family_contracts: Vec<EmbeddingFamilyContract>,
    /// Canonical target records in arbitrary input order.
    pub targets: Vec<EmbeddingTarget>,
    /// Canonical target sets in arbitrary input order.
    pub target_sets: Vec<TargetSet>,
    /// Structural relations in arbitrary input order.
    pub relations: Vec<TargetRelation>,
    /// Family-scoped token spans in arbitrary input order.
    pub token_spans: Vec<TokenSpan>,
    /// Exact external-artifact bindings in arbitrary input order.
    pub external_bindings: Vec<ExternalBinding>,
    /// Opaque derived indexes in arbitrary input order.
    pub indexes: Vec<DerivedIndex>,
    /// Caller extension sections.
    pub extensions: Vec<ExtensionSection>,
}

impl CanonicalMetadataInput {
    /// Encode source-bound metadata with no matrix-dependent references.
    pub fn encode(self) -> Result<CanonicalMetadataSections, EmbeddingError> {
        self.encode_inner(None)
    }

    /// Encode source-bound metadata against the original matrix commitments.
    pub fn encode_for_matrices(
        self,
        matrices: &[MatrixCommitment],
    ) -> Result<CanonicalMetadataSections, EmbeddingError> {
        self.encode_inner(Some(matrices))
    }

    fn encode_inner(
        self,
        matrices: Option<&[MatrixCommitment]>,
    ) -> Result<CanonicalMetadataSections, EmbeddingError> {
        let catalog = MetadataCatalogInput {
            family_contracts: self.family_contracts,
            targets: self.targets,
            target_sets: self.target_sets,
            relations: self.relations,
            token_spans: self.token_spans,
            external_bindings: self.external_bindings,
            indexes: self.indexes,
            extensions: self.extensions,
        }
        .encode(
            self.source.source_exact_digest(),
            self.source.certified_rdf_digest(),
            self.source.dataset_target_id(),
            matrices,
        )?;
        Ok(CanonicalMetadataSections {
            source_exact_digest: self.source.source_exact_digest(),
            source: self.source.encode().to_vec(),
            contracts: catalog.contracts,
            targets: catalog.targets,
            target_sets: catalog.target_sets,
            relations: catalog.relations,
            token_spans: catalog.token_spans,
            external_bindings: catalog.external_bindings,
            index_guards: catalog.index_guards,
            inline_index_payloads: catalog.inline_index_payloads,
            extensions: catalog.extensions,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::{
        AppliedStage, DimensionalityPolicy, DistanceMetric, PrefixPostprocessing,
        RdfDatasetBuilder, VectorDtype,
    };
    const FX: purrdf_core::purremb_fixture::Identities = purrdf_core::purremb_fixture::Identities {
        artifact_media: "application/example",
        stage_payload: &[1, 2, 3],
        ..purrdf_core::purremb_fixture::Identities::at("")
    };

    fn family_contract() -> EmbeddingFamilyContract {
        EmbeddingFamilyContract {
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
            dtype: VectorDtype::F32,
            metric: DistanceMetric::Cosine,
            dimensionality: DimensionalityPolicy::fixed(2, PrefixPostprocessing::None).unwrap(),
            extensions: Vec::new(),
        }
    }
    fn certified_empty_pack() -> (CertifiedEmbeddingSource, Vec<u8>) {
        let dataset = RdfDatasetBuilder::new().freeze().unwrap();
        CertifiedEmbeddingSource::from_dataset(&dataset).unwrap()
    }
    #[test]
    fn pack_certification_checks_original_exact_bytes() {
        let (source, bytes) = certified_empty_pack();
        assert_eq!(source.source_exact_digest(), ContentDigest::of(&bytes));
        let encoded = source.encode();
        assert_eq!(u32::from_le_bytes(encoded[0..4].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(encoded[4..8].try_into().unwrap()), 1);
        assert_eq!(
            u64::from_le_bytes(encoded[8..16].try_into().unwrap()),
            bytes.len() as u64
        );
        assert_eq!(&encoded[24..56], source.source_exact_digest().as_bytes());
        assert!(matches!(
            CertifiedEmbeddingSource::certify(EmbeddingSource::Purrpck(b"not a pack")),
            Err(SourceAttachmentError::Embedding(
                EmbeddingError::InvalidSourcePack(_)
            ))
        ));
    }
    #[test]
    fn typed_metadata_is_deterministic_and_deduplicates_logical_input() {
        let (source, _) = certified_empty_pack();
        let dataset_target = source.dataset_target(true).unwrap();
        let set = TargetSet::new(vec![dataset_target.id]).unwrap();
        let input = CanonicalMetadataInput {
            source,
            family_contracts: vec![family_contract(), family_contract()],
            targets: vec![dataset_target.clone(), dataset_target],
            target_sets: vec![set.clone(), set],
            relations: Vec::new(),
            token_spans: Vec::new(),
            external_bindings: Vec::new(),
            indexes: Vec::new(),
            extensions: Vec::new(),
        };
        let first = input.clone().encode().unwrap();
        let second = input.encode().unwrap();
        assert_eq!(first, second);
        assert_eq!(
            u64::from_le_bytes(first.contracts[8..16].try_into().unwrap()),
            1
        );
        assert_eq!(
            u64::from_le_bytes(first.targets[8..16].try_into().unwrap()),
            1
        );
        assert_eq!(
            u64::from_le_bytes(first.target_sets[8..16].try_into().unwrap()),
            1
        );
    }
}
