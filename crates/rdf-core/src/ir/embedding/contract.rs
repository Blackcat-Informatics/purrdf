// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Canonical vector-family, pipeline, and Matryoshka contracts.

use crate::ContentDigest;

use super::error::{DigestKind, EmbeddingError};
use super::identity::{
    ChunkingContractId, FamilyContractDigest, FamilyId, VectorSpaceId, derive_chunking_contract_id,
    derive_family_contract_digest, derive_family_id, derive_vector_space_id,
};

/// Maximum canonical TLV block size in PURREMB v1.
pub const MAX_TLV_BLOCK_LEN: usize = 16 * 1024 * 1024;
/// Maximum nested TLV depth in PURREMB v1.
pub const MAX_TLV_DEPTH: u8 = 8;

const TLV_CRITICAL: u8 = 1;

/// Canonical PURREMB TLV wire types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TlvWireType {
    /// Uninterpreted exact bytes.
    Bytes = 1,
    /// Well-formed UTF-8 bytes.
    Utf8 = 2,
    /// One little-endian `u32`.
    U32 = 3,
    /// One little-endian `u64`.
    U64 = 4,
    /// Exactly 32 digest bytes.
    Digest32 = 5,
    /// One canonical boolean byte.
    Bool = 6,
    /// One nested canonical TLV block.
    Block = 7,
    /// A canonical list of nested TLV blocks.
    BlockList = 8,
    /// A canonical list of little-endian `u32` values.
    U32List = 9,
}

impl TryFrom<u8> for TlvWireType {
    type Error = EmbeddingError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Bytes),
            2 => Ok(Self::Utf8),
            3 => Ok(Self::U32),
            4 => Ok(Self::U64),
            5 => Ok(Self::Digest32),
            6 => Ok(Self::Bool),
            7 => Ok(Self::Block),
            8 => Ok(Self::BlockList),
            9 => Ok(Self::U32List),
            value => Err(EmbeddingError::UnsupportedCode {
                field: "TLV wire type",
                value: u32::from(value),
            }),
        }
    }
}

/// One caller extension field retained in a canonical contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractExtension {
    /// Caller tag in `0x8000..=0xffff`.
    pub tag: u16,
    /// Canonical wire type.
    pub wire_type: TlvWireType,
    /// Exact canonical value bytes, excluding TLV framing.
    pub value: Vec<u8>,
}

impl ContractExtension {
    /// Validates and constructs a noncritical caller extension.
    pub fn new(tag: u16, wire_type: TlvWireType, value: Vec<u8>) -> Result<Self, EmbeddingError> {
        if tag < 0x8000 {
            return Err(EmbeddingError::MalformedTlv(
                "caller extension tags start at 0x8000",
            ));
        }
        validate_tlv_value(wire_type, &value, 1)?;
        Ok(Self {
            tag,
            wire_type,
            value,
        })
    }
}

/// Borrowed framing for one canonical TLV entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TlvEntryRef<'a> {
    /// Numeric field tag.
    pub tag: u16,
    /// Canonical wire type.
    pub wire_type: TlvWireType,
    /// Whether an unknown reader must reject the field.
    pub critical: bool,
    /// Exact value bytes without framing or padding.
    pub value: &'a [u8],
}

/// Allocation-free iterator over a structurally validated canonical TLV block.
#[derive(Debug, Clone)]
pub struct TlvIter<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Iterator for TlvIter<'a> {
    type Item = TlvEntryRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position == self.bytes.len() {
            return None;
        }
        let start = self.position;
        let tag = u16::from_le_bytes(self.bytes[start..start + 2].try_into().ok()?);
        let wire_type = TlvWireType::try_from(self.bytes[start + 2]).ok()?;
        let flags = self.bytes[start + 3];
        let length = u32::from_le_bytes(self.bytes[start + 4..start + 8].try_into().ok()?);
        let length = usize::try_from(length).ok()?;
        let value_start = start + 8;
        let value_end = value_start.checked_add(length)?;
        self.position = align_up_usize(value_end, 8)?;
        Some(TlvEntryRef {
            tag,
            wire_type,
            critical: flags & TLV_CRITICAL != 0,
            value: &self.bytes[value_start..value_end],
        })
    }
}

/// Validates a canonical TLV block and returns an allocation-free iterator.
pub fn canonical_tlv(bytes: &[u8]) -> Result<TlvIter<'_>, EmbeddingError> {
    validate_tlv_block(bytes, 0)?;
    Ok(TlvIter { bytes, position: 0 })
}

/// Verifies a canonical block's embedded SHA-256 field against its payload.
///
/// Kind-specific schema validation remains responsible for requiring the two
/// tags and their exact wire types. This helper supplies the semantic equality
/// check shared by contract, target, and index validation paths.
pub(crate) fn validate_sha256_field(
    bytes: &[u8],
    payload_tag: u16,
    digest_tag: u16,
    kind: DigestKind,
) -> Result<(), EmbeddingError> {
    let mut payload = None;
    let mut stored = None;
    for entry in canonical_tlv(bytes)? {
        if entry.tag == payload_tag {
            payload = Some(entry.value);
        } else if entry.tag == digest_tag {
            stored = Some(entry);
        }
    }
    let payload = payload.ok_or(EmbeddingError::Missing("self-digest payload"))?;
    let stored = stored.ok_or(EmbeddingError::Missing("self-digest field"))?;
    if stored.wire_type != TlvWireType::Digest32 || !stored.critical {
        return Err(EmbeddingError::MalformedTlv(
            "self-digest field has wrong type or criticality",
        ));
    }
    let expected: [u8; 32] = stored
        .value
        .try_into()
        .map_err(|_| EmbeddingError::MalformedTlv("self-digest length"))?;
    let actual = *ContentDigest::of(payload).as_bytes();
    if expected != actual {
        return Err(EmbeddingError::DigestMismatch {
            kind,
            expected,
            actual,
        });
    }
    Ok(())
}

/// Exact identity of a model, engine, tokenizer, or manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactIdentity {
    /// Caller-supplied stable identifier.
    pub identifier: String,
    /// Caller-supplied media type or format identifier.
    pub media_type: String,
    /// SHA-256 of exact artifact or canonical manifest bytes.
    pub digest: ContentDigest,
    /// Exact caller revision bytes.
    pub revision: Option<Vec<u8>>,
    /// Whether the digest identifies one artifact or a canonical manifest.
    pub kind: ArtifactIdentityKind,
}

impl ArtifactIdentity {
    /// Constructs a validated artifact identity.
    pub fn new(
        identifier: impl Into<String>,
        media_type: impl Into<String>,
        digest: ContentDigest,
        revision: Option<Vec<u8>>,
        kind: ArtifactIdentityKind,
    ) -> Result<Self, EmbeddingError> {
        let identity = Self {
            identifier: identifier.into(),
            media_type: media_type.into(),
            digest,
            revision,
            kind,
        };
        identity.validate()?;
        Ok(identity)
    }

    fn validate(&self) -> Result<(), EmbeddingError> {
        validate_identifier(&self.identifier, "artifact identifier")?;
        validate_identifier(&self.media_type, "artifact media type")?;
        if self.revision.as_ref().is_some_and(Vec::is_empty) {
            return Err(EmbeddingError::MalformedTlv(
                "artifact revision is present but empty",
            ));
        }
        Ok(())
    }

    fn encode(&self) -> Result<Vec<u8>, EmbeddingError> {
        self.validate()?;
        let mut block = Vec::new();
        push_tlv(
            &mut block,
            1,
            TlvWireType::Utf8,
            true,
            self.identifier.as_bytes(),
        )?;
        push_tlv(
            &mut block,
            2,
            TlvWireType::Utf8,
            true,
            self.media_type.as_bytes(),
        )?;
        push_tlv(
            &mut block,
            3,
            TlvWireType::Digest32,
            true,
            self.digest.as_bytes(),
        )?;
        if let Some(revision) = &self.revision {
            push_tlv(&mut block, 4, TlvWireType::Bytes, true, revision)?;
        }
        push_tlv(
            &mut block,
            5,
            TlvWireType::U32,
            true,
            &self.kind.code().to_le_bytes(),
        )?;
        Ok(block)
    }
}

/// Artifact cardinality carried by [`ArtifactIdentity`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactIdentityKind {
    /// One exact artifact.
    Single,
    /// A canonical manifest identifying multiple exact artifacts.
    Manifest,
}

impl ArtifactIdentityKind {
    const fn code(self) -> u32 {
        match self {
            Self::Single => 1,
            Self::Manifest => 2,
        }
    }
}

/// Complete identity and parameters for one applied pipeline stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageImplementation {
    /// Caller-supplied implementation identifier.
    pub identifier: String,
    /// Exact implementation or manifest digest.
    pub digest: ContentDigest,
    /// Caller-supplied canonical parameter encoding identifier.
    pub parameter_encoding: String,
    /// Canonical parameter bytes.
    pub parameters: Vec<u8>,
}

impl StageImplementation {
    /// Constructs and validates an applied stage implementation.
    pub fn new(
        identifier: impl Into<String>,
        digest: ContentDigest,
        parameter_encoding: impl Into<String>,
        parameters: Vec<u8>,
    ) -> Result<Self, EmbeddingError> {
        let implementation = Self {
            identifier: identifier.into(),
            digest,
            parameter_encoding: parameter_encoding.into(),
            parameters,
        };
        implementation.validate()?;
        Ok(implementation)
    }

    fn validate(&self) -> Result<(), EmbeddingError> {
        validate_identifier(&self.identifier, "stage identifier")?;
        validate_identifier(&self.parameter_encoding, "stage parameter encoding")
    }
}

/// Explicit pipeline-stage state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppliedStage {
    /// The stage is explicitly absent.
    NotApplied,
    /// The stage is applied with complete implementation identity and parameters.
    Applied(StageImplementation),
}

impl AppliedStage {
    /// Returns whether this stage is applied.
    #[must_use]
    pub const fn is_applied(&self) -> bool {
        matches!(self, Self::Applied(_))
    }

    /// Encodes the exact canonical stage block.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EmbeddingError> {
        let mut block = Vec::new();
        match self {
            Self::NotApplied => {
                push_tlv(&mut block, 1, TlvWireType::U32, true, &0u32.to_le_bytes())?;
            }
            Self::Applied(implementation) => {
                implementation.validate()?;
                push_tlv(&mut block, 1, TlvWireType::U32, true, &1u32.to_le_bytes())?;
                push_tlv(
                    &mut block,
                    2,
                    TlvWireType::Utf8,
                    true,
                    implementation.identifier.as_bytes(),
                )?;
                push_tlv(
                    &mut block,
                    3,
                    TlvWireType::Digest32,
                    true,
                    implementation.digest.as_bytes(),
                )?;
                push_tlv(
                    &mut block,
                    4,
                    TlvWireType::Utf8,
                    true,
                    implementation.parameter_encoding.as_bytes(),
                )?;
                push_tlv(
                    &mut block,
                    5,
                    TlvWireType::Bytes,
                    true,
                    &implementation.parameters,
                )?;
                let digest = ContentDigest::of(&implementation.parameters);
                push_tlv(
                    &mut block,
                    6,
                    TlvWireType::Digest32,
                    true,
                    digest.as_bytes(),
                )?;
            }
        }
        Ok(block)
    }
}

/// Authoritative dense scalar representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum VectorDtype {
    /// IEEE-754 binary32.
    F32 = 1,
    /// IEEE-754 binary64.
    F64 = 2,
}

impl VectorDtype {
    /// Width of one scalar in bytes.
    #[must_use]
    pub const fn width(self) -> u32 {
        match self {
            Self::F32 => 4,
            Self::F64 => 8,
        }
    }

    /// Stable v1 wire code.
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }
}

impl TryFrom<u32> for VectorDtype {
    type Error = EmbeddingError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::F32),
            2 => Ok(Self::F64),
            value => Err(EmbeddingError::UnsupportedCode {
                field: "vector dtype",
                value,
            }),
        }
    }
}

/// Distance semantics for compatible vectors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DistanceMetric {
    /// Cosine distance.
    Cosine,
    /// Negative dot-product ranking.
    NegativeDot,
    /// Squared Euclidean distance.
    SquaredEuclidean,
    /// Caller-supplied metric with canonical parameters.
    Extension {
        /// Stable metric identifier.
        identifier: String,
        /// Canonical parameter encoding identifier.
        parameter_encoding: String,
        /// Canonical parameter bytes.
        parameters: Vec<u8>,
    },
}

impl DistanceMetric {
    /// Stable metric code.
    #[must_use]
    pub const fn code(&self) -> u32 {
        match self {
            Self::Cosine => 1,
            Self::NegativeDot => 2,
            Self::SquaredEuclidean => 3,
            Self::Extension { .. } => 0x8000_0000,
        }
    }

    fn encode(&self) -> Result<Vec<u8>, EmbeddingError> {
        let mut block = Vec::new();
        push_tlv(
            &mut block,
            1,
            TlvWireType::U32,
            true,
            &self.code().to_le_bytes(),
        )?;
        if let Self::Extension {
            identifier,
            parameter_encoding,
            parameters,
        } = self
        {
            validate_identifier(identifier, "metric identifier")?;
            validate_identifier(parameter_encoding, "metric parameter encoding")?;
            push_tlv(
                &mut block,
                2,
                TlvWireType::Utf8,
                true,
                identifier.as_bytes(),
            )?;
            push_tlv(
                &mut block,
                3,
                TlvWireType::Utf8,
                true,
                parameter_encoding.as_bytes(),
            )?;
            push_tlv(&mut block, 4, TlvWireType::Bytes, true, parameters)?;
            push_tlv(
                &mut block,
                5,
                TlvWireType::Digest32,
                true,
                ContentDigest::of(parameters).as_bytes(),
            )?;
        }
        Ok(block)
    }
}

/// Postprocessing applied to one leading-prefix space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PrefixPostprocessing {
    /// Use exact stored leading-prefix scalar bits.
    None = 0,
    /// Apply the deterministic v1 L2 algorithm while reading.
    DeterministicL2 = 1,
}

impl PrefixPostprocessing {
    /// Stable v1 wire code.
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }
}

impl TryFrom<u32> for PrefixPostprocessing {
    type Error = EmbeddingError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::DeterministicL2),
            value => Err(EmbeddingError::UnsupportedCode {
                field: "prefix postprocessing",
                value,
            }),
        }
    }
}

/// One declared effective prefix in an embedding family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EffectivePrefix {
    /// Nonzero leading-prefix dimension.
    pub dimension: u32,
    /// Exact postprocessing for this effective space.
    pub postprocessing: PrefixPostprocessing,
}

/// Fixed or Matryoshka dimensionality contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DimensionalityPolicy {
    /// One fixed effective dimension.
    Fixed(EffectivePrefix),
    /// Two or more strictly increasing leading-prefix spaces.
    Matryoshka(Vec<EffectivePrefix>),
}

impl DimensionalityPolicy {
    /// Constructs a fixed-dimensional policy.
    pub fn fixed(
        dimension: u32,
        postprocessing: PrefixPostprocessing,
    ) -> Result<Self, EmbeddingError> {
        let policy = Self::Fixed(EffectivePrefix {
            dimension,
            postprocessing,
        });
        policy.validate()?;
        Ok(policy)
    }

    /// Constructs a Matryoshka leading-prefix policy.
    pub fn matryoshka(prefixes: Vec<EffectivePrefix>) -> Result<Self, EmbeddingError> {
        let policy = Self::Matryoshka(prefixes);
        policy.validate()?;
        Ok(policy)
    }

    /// Declared effective prefixes in strictly increasing dimension order.
    #[must_use]
    pub fn prefixes(&self) -> &[EffectivePrefix] {
        match self {
            Self::Fixed(prefix) => core::slice::from_ref(prefix),
            Self::Matryoshka(prefixes) => prefixes,
        }
    }

    /// Highest stored dimension.
    #[must_use]
    pub fn stored_dimension(&self) -> u32 {
        self.prefixes().last().map_or(0, |prefix| prefix.dimension)
    }

    /// Stable policy code.
    #[must_use]
    pub const fn code(&self) -> u32 {
        match self {
            Self::Fixed(_) => 1,
            Self::Matryoshka(_) => 2,
        }
    }

    fn validate(&self) -> Result<(), EmbeddingError> {
        let prefixes = self.prefixes();
        let valid_count = match self {
            Self::Fixed(_) => prefixes.len() == 1,
            Self::Matryoshka(_) => prefixes.len() >= 2,
        };
        if !valid_count {
            return Err(EmbeddingError::Malformed(
                "invalid fixed or Matryoshka prefix count",
            ));
        }
        let mut previous = 0;
        for prefix in prefixes {
            if prefix.dimension == 0 || prefix.dimension <= previous {
                return Err(EmbeddingError::NonCanonicalOrder(
                    "effective prefix dimensions",
                ));
            }
            previous = prefix.dimension;
        }
        Ok(())
    }

    fn encode(&self) -> Result<Vec<u8>, EmbeddingError> {
        self.validate()?;
        let mut prefix_blocks = Vec::with_capacity(self.prefixes().len());
        for prefix in self.prefixes() {
            let mut block = Vec::new();
            push_tlv(
                &mut block,
                1,
                TlvWireType::U32,
                true,
                &prefix.dimension.to_le_bytes(),
            )?;
            push_tlv(
                &mut block,
                2,
                TlvWireType::U32,
                true,
                &prefix.postprocessing.code().to_le_bytes(),
            )?;
            prefix_blocks.push(block);
        }
        let list = encode_block_list(&prefix_blocks)?;
        let mut block = Vec::new();
        push_tlv(
            &mut block,
            1,
            TlvWireType::U32,
            true,
            &self.code().to_le_bytes(),
        )?;
        push_tlv(
            &mut block,
            2,
            TlvWireType::U32,
            true,
            &self.stored_dimension().to_le_bytes(),
        )?;
        push_tlv(&mut block, 3, TlvWireType::BlockList, true, &list)?;
        Ok(block)
    }
}

/// Complete generation contract for one fixed or Matryoshka embedding family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingFamilyContract {
    /// Model identity.
    pub model: ArtifactIdentity,
    /// Inference-engine identity.
    pub engine: ArtifactIdentity,
    /// Tokenizer identity.
    pub tokenizer: ArtifactIdentity,
    /// Execution settings.
    pub execution: AppliedStage,
    /// Mapping from an embedding subject to model input; always applied.
    pub subject_projection: AppliedStage,
    /// Input preprocessing.
    pub preprocessing: AppliedStage,
    /// Chunking policy.
    pub chunking: AppliedStage,
    /// Token/output pooling.
    pub pooling: AppliedStage,
    /// Generation-time normalization.
    pub normalization: AppliedStage,
    /// Generation-time truncation.
    pub truncation: AppliedStage,
    /// Authoritative stored scalar type.
    pub dtype: VectorDtype,
    /// Distance semantics.
    pub metric: DistanceMetric,
    /// Fixed or Matryoshka dimension declarations.
    pub dimensionality: DimensionalityPolicy,
    /// Canonical caller extensions.
    pub extensions: Vec<ContractExtension>,
}

impl EmbeddingFamilyContract {
    /// Validates the complete contract and returns its canonical TLV bytes.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EmbeddingError> {
        self.validate()?;
        let artifact_blocks = [
            self.model.encode()?,
            self.engine.encode()?,
            self.tokenizer.encode()?,
        ];
        let stage_blocks = [
            self.execution.canonical_bytes()?,
            self.subject_projection.canonical_bytes()?,
            self.preprocessing.canonical_bytes()?,
            self.chunking.canonical_bytes()?,
            self.pooling.canonical_bytes()?,
            self.normalization.canonical_bytes()?,
            self.truncation.canonical_bytes()?,
        ];
        let metric = self.metric.encode()?;
        let dimensionality = self.dimensionality.encode()?;

        let mut block = Vec::new();
        for (index, value) in artifact_blocks.iter().enumerate() {
            push_tlv(
                &mut block,
                u16::try_from(index + 1).expect("small fixed tag"),
                TlvWireType::Block,
                true,
                value,
            )?;
        }
        for (index, value) in stage_blocks.iter().enumerate() {
            push_tlv(
                &mut block,
                u16::try_from(index + 4).expect("small fixed tag"),
                TlvWireType::Block,
                true,
                value,
            )?;
        }
        push_tlv(
            &mut block,
            11,
            TlvWireType::U32,
            true,
            &self.dtype.code().to_le_bytes(),
        )?;
        push_tlv(&mut block, 12, TlvWireType::U32, true, &1u32.to_le_bytes())?;
        push_tlv(&mut block, 13, TlvWireType::U32, true, &0u32.to_le_bytes())?;
        push_tlv(&mut block, 14, TlvWireType::Block, true, &metric)?;
        push_tlv(&mut block, 15, TlvWireType::Block, true, &dimensionality)?;
        for extension in &self.extensions {
            push_tlv(
                &mut block,
                extension.tag,
                extension.wire_type,
                false,
                &extension.value,
            )?;
        }
        Ok(block)
    }

    /// Derives all stable family and effective-space identities.
    pub fn derive(&self) -> Result<EmbeddingFamily, EmbeddingError> {
        let canonical_bytes = self.canonical_bytes()?;
        let contract_digest = derive_family_contract_digest(&canonical_bytes);
        let family_id = derive_family_id(contract_digest);
        let chunking_bytes = self.chunking.canonical_bytes()?;
        let chunking_id = derive_chunking_contract_id(&chunking_bytes);
        let spaces = self
            .dimensionality
            .prefixes()
            .iter()
            .enumerate()
            .map(|(ordinal, prefix)| {
                Ok(EffectiveSpace {
                    id: derive_vector_space_id(
                        family_id,
                        prefix.dimension,
                        prefix.postprocessing.code(),
                    ),
                    family_id,
                    dimension: prefix.dimension,
                    postprocessing: prefix.postprocessing,
                    ordinal: u32::try_from(ordinal).map_err(|_| EmbeddingError::CountLimit {
                        field: "effective prefix",
                        value: u64::try_from(ordinal).unwrap_or(u64::MAX),
                    })?,
                })
            })
            .collect::<Result<Vec<_>, EmbeddingError>>()?;
        Ok(EmbeddingFamily {
            id: family_id,
            contract_digest,
            chunking_id,
            dtype: self.dtype,
            metric: self.metric.clone(),
            stored_dimension: self.dimensionality.stored_dimension(),
            spaces,
            canonical_contract: canonical_bytes,
        })
    }

    fn validate(&self) -> Result<(), EmbeddingError> {
        self.model.validate()?;
        self.engine.validate()?;
        self.tokenizer.validate()?;
        if !self.subject_projection.is_applied() {
            return Err(EmbeddingError::Malformed(
                "subject projection must be applied",
            ));
        }
        self.dimensionality.validate()?;
        let mut previous = None;
        for extension in &self.extensions {
            if previous.is_some_and(|tag| tag >= extension.tag) {
                return Err(EmbeddingError::NonCanonicalOrder("contract extension tags"));
            }
            if extension.tag < 0x8000 {
                return Err(EmbeddingError::MalformedTlv(
                    "contract extension tag is outside caller range",
                ));
            }
            validate_tlv_value(extension.wire_type, &extension.value, 1)?;
            previous = Some(extension.tag);
        }
        Ok(())
    }
}

/// Derived, canonical representation of one embedding family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingFamily {
    /// Stable family identity.
    pub id: FamilyId,
    /// Digest of the exact canonical contract bytes.
    pub contract_digest: FamilyContractDigest,
    /// Stable chunking-stage identity.
    pub chunking_id: ChunkingContractId,
    /// Stored scalar type.
    pub dtype: VectorDtype,
    /// Distance semantics.
    pub metric: DistanceMetric,
    /// Highest stored dimension.
    pub stored_dimension: u32,
    /// Effective fixed or Matryoshka spaces.
    pub spaces: Vec<EffectiveSpace>,
    /// Exact canonical contract block.
    pub canonical_contract: Vec<u8>,
}

/// One effective vector space in a fixed or Matryoshka family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveSpace {
    /// Stable effective-space identity.
    pub id: VectorSpaceId,
    /// Owning family.
    pub family_id: FamilyId,
    /// Leading-prefix dimension.
    pub dimension: u32,
    /// Prefix postprocessing.
    pub postprocessing: PrefixPostprocessing,
    /// Zero-based ordinal inside the family.
    pub ordinal: u32,
}

fn validate_identifier(value: &str, context: &'static str) -> Result<(), EmbeddingError> {
    if value.is_empty() {
        return Err(EmbeddingError::Missing(context));
    }
    if value.as_bytes().contains(&0) {
        return Err(EmbeddingError::InvalidUtf8(context));
    }
    Ok(())
}

pub(crate) fn push_tlv(
    output: &mut Vec<u8>,
    tag: u16,
    wire_type: TlvWireType,
    critical: bool,
    value: &[u8],
) -> Result<(), EmbeddingError> {
    validate_tlv_value(wire_type, value, 1)?;
    let value_length = u32::try_from(value.len()).map_err(|_| EmbeddingError::CountLimit {
        field: "TLV value byte",
        value: u64::try_from(value.len()).unwrap_or(u64::MAX),
    })?;
    output.extend_from_slice(&tag.to_le_bytes());
    output.push(wire_type as u8);
    output.push(u8::from(critical));
    output.extend_from_slice(&value_length.to_le_bytes());
    output.extend_from_slice(value);
    let padded =
        align_up_usize(output.len(), 8).ok_or(EmbeddingError::ArithmeticOverflow("TLV padding"))?;
    output.resize(padded, 0);
    if output.len() > MAX_TLV_BLOCK_LEN {
        return Err(EmbeddingError::CountLimit {
            field: "TLV block byte",
            value: u64::try_from(output.len()).unwrap_or(u64::MAX),
        });
    }
    Ok(())
}

fn encode_block_list(blocks: &[Vec<u8>]) -> Result<Vec<u8>, EmbeddingError> {
    let count = u32::try_from(blocks.len()).map_err(|_| EmbeddingError::CountLimit {
        field: "TLV block-list item",
        value: u64::try_from(blocks.len()).unwrap_or(u64::MAX),
    })?;
    let mut output = Vec::new();
    output.extend_from_slice(&count.to_le_bytes());
    output.extend_from_slice(&0u32.to_le_bytes());
    for block in blocks {
        validate_tlv_block(block, 1)?;
        output.extend_from_slice(
            &u64::try_from(block.len())
                .expect("validated TLV block length fits u64")
                .to_le_bytes(),
        );
        output.extend_from_slice(block);
        let padded = align_up_usize(output.len(), 8)
            .ok_or(EmbeddingError::ArithmeticOverflow("TLV block-list padding"))?;
        output.resize(padded, 0);
    }
    Ok(output)
}

pub(crate) fn validate_tlv_block(bytes: &[u8], depth: u8) -> Result<(), EmbeddingError> {
    if depth > MAX_TLV_DEPTH {
        return Err(EmbeddingError::MalformedTlv(
            "nested TLV depth exceeds v1 limit",
        ));
    }
    if bytes.len() > MAX_TLV_BLOCK_LEN {
        return Err(EmbeddingError::CountLimit {
            field: "TLV block byte",
            value: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        });
    }
    let mut position = 0usize;
    let mut previous_tag = None;
    while position < bytes.len() {
        let header_end = position
            .checked_add(8)
            .ok_or(EmbeddingError::ArithmeticOverflow("TLV header"))?;
        let header = bytes
            .get(position..header_end)
            .ok_or(EmbeddingError::MalformedTlv("truncated entry header"))?;
        let tag = u16::from_le_bytes([header[0], header[1]]);
        if previous_tag.is_some_and(|previous| previous >= tag) {
            return Err(EmbeddingError::NonCanonicalOrder("TLV tags"));
        }
        let wire_type = TlvWireType::try_from(header[2])?;
        if header[3] & !TLV_CRITICAL != 0 {
            return Err(EmbeddingError::ReservedNonzero("TLV flags"));
        }
        let value_length = u32::from_le_bytes(header[4..8].try_into().expect("fixed slice"));
        let value_length = usize::try_from(value_length)
            .map_err(|_| EmbeddingError::ArithmeticOverflow("TLV value length"))?;
        let value_end = header_end
            .checked_add(value_length)
            .ok_or(EmbeddingError::ArithmeticOverflow("TLV value span"))?;
        let value = bytes
            .get(header_end..value_end)
            .ok_or(EmbeddingError::MalformedTlv("truncated entry value"))?;
        validate_tlv_value(wire_type, value, depth.saturating_add(1))?;
        let next = align_up_usize(value_end, 8)
            .ok_or(EmbeddingError::ArithmeticOverflow("TLV entry padding"))?;
        let padding = bytes
            .get(value_end..next)
            .ok_or(EmbeddingError::MalformedTlv("truncated entry padding"))?;
        if padding.iter().any(|byte| *byte != 0) {
            return Err(EmbeddingError::InvalidPadding {
                offset: u64::try_from(value_end).expect("slice offset fits u64"),
            });
        }
        position = next;
        previous_tag = Some(tag);
    }
    if position != bytes.len() {
        return Err(EmbeddingError::MalformedTlv("nonminimal block length"));
    }
    Ok(())
}

fn validate_tlv_value(
    wire_type: TlvWireType,
    value: &[u8],
    depth: u8,
) -> Result<(), EmbeddingError> {
    match wire_type {
        TlvWireType::Bytes => Ok(()),
        TlvWireType::Utf8 => core::str::from_utf8(value)
            .map(|_| ())
            .map_err(|_| EmbeddingError::InvalidUtf8("TLV UTF8 value")),
        TlvWireType::U32 if value.len() == 4 => Ok(()),
        TlvWireType::U64 if value.len() == 8 => Ok(()),
        TlvWireType::Digest32 if value.len() == 32 => Ok(()),
        TlvWireType::Bool if value == [0] || value == [1] => Ok(()),
        TlvWireType::Block => validate_tlv_block(value, depth),
        TlvWireType::BlockList => validate_block_list(value, depth),
        TlvWireType::U32List => validate_u32_list(value),
        _ => Err(EmbeddingError::MalformedTlv(
            "wire type has a noncanonical value length or payload",
        )),
    }
}

fn validate_block_list(value: &[u8], depth: u8) -> Result<(), EmbeddingError> {
    let header = value
        .get(..8)
        .ok_or(EmbeddingError::MalformedTlv("truncated block-list header"))?;
    let count = u32::from_le_bytes(header[..4].try_into().expect("fixed slice"));
    if header[4..8] != [0; 4] {
        return Err(EmbeddingError::ReservedNonzero("TLV block-list header"));
    }
    let mut position = 8usize;
    for _ in 0..count {
        let length_end = position
            .checked_add(8)
            .ok_or(EmbeddingError::ArithmeticOverflow("block-list length"))?;
        let length_bytes = value
            .get(position..length_end)
            .ok_or(EmbeddingError::MalformedTlv("truncated block-list length"))?;
        let length = u64::from_le_bytes(length_bytes.try_into().expect("fixed slice"));
        let length = usize::try_from(length)
            .map_err(|_| EmbeddingError::ArithmeticOverflow("block-list item length"))?;
        let block_end = length_end
            .checked_add(length)
            .ok_or(EmbeddingError::ArithmeticOverflow("block-list item span"))?;
        let block = value
            .get(length_end..block_end)
            .ok_or(EmbeddingError::MalformedTlv("truncated block-list item"))?;
        validate_tlv_block(block, depth)?;
        let next = align_up_usize(block_end, 8)
            .ok_or(EmbeddingError::ArithmeticOverflow("block-list padding"))?;
        let padding = value
            .get(block_end..next)
            .ok_or(EmbeddingError::MalformedTlv("truncated block-list padding"))?;
        if padding.iter().any(|byte| *byte != 0) {
            return Err(EmbeddingError::InvalidPadding {
                offset: u64::try_from(block_end).expect("slice offset fits u64"),
            });
        }
        position = next;
    }
    if position != value.len() {
        return Err(EmbeddingError::MalformedTlv(
            "block-list count or final length mismatch",
        ));
    }
    Ok(())
}

fn validate_u32_list(value: &[u8]) -> Result<(), EmbeddingError> {
    let header = value
        .get(..8)
        .ok_or(EmbeddingError::MalformedTlv("truncated u32-list header"))?;
    let count = u32::from_le_bytes(header[..4].try_into().expect("fixed slice"));
    if header[4..8] != [0; 4] {
        return Err(EmbeddingError::ReservedNonzero("TLV u32-list header"));
    }
    let values_len = usize::try_from(count)
        .map_err(|_| EmbeddingError::ArithmeticOverflow("u32-list count"))?
        .checked_mul(4)
        .ok_or(EmbeddingError::ArithmeticOverflow("u32-list byte length"))?;
    if value.len() != 8 + values_len {
        return Err(EmbeddingError::MalformedTlv("u32-list length mismatch"));
    }
    Ok(())
}

const fn align_up_usize(value: usize, alignment: usize) -> Option<usize> {
    match value.checked_add(alignment - 1) {
        Some(sum) => Some(sum & !(alignment - 1)),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(name: &str) -> ArtifactIdentity {
        ArtifactIdentity::new(
            name,
            "application/example",
            ContentDigest::of(name.as_bytes()),
            None,
            ArtifactIdentityKind::Single,
        )
        .expect("valid fixture artifact")
    }

    fn stage(name: &str) -> AppliedStage {
        AppliedStage::Applied(
            StageImplementation::new(
                name,
                ContentDigest::of(name.as_bytes()),
                "application/octet-stream",
                vec![1, 2, 3],
            )
            .expect("valid fixture stage"),
        )
    }

    fn contract(policy: DimensionalityPolicy) -> EmbeddingFamilyContract {
        EmbeddingFamilyContract {
            model: artifact("model"),
            engine: artifact("engine"),
            tokenizer: artifact("tokenizer"),
            execution: stage("execution"),
            subject_projection: stage("projection"),
            preprocessing: AppliedStage::NotApplied,
            chunking: stage("chunking"),
            pooling: stage("pooling"),
            normalization: AppliedStage::NotApplied,
            truncation: AppliedStage::NotApplied,
            dtype: VectorDtype::F32,
            metric: DistanceMetric::Cosine,
            dimensionality: policy,
            extensions: Vec::new(),
        }
    }

    #[test]
    fn fixed_contract_is_canonical_and_derives_one_space() {
        let policy =
            DimensionalityPolicy::fixed(384, PrefixPostprocessing::None).expect("fixed policy");
        let family = contract(policy).derive().expect("family derives");
        assert_eq!(family.stored_dimension, 384);
        assert_eq!(family.spaces.len(), 1);
        assert_eq!(family.spaces[0].dimension, 384);
        canonical_tlv(&family.canonical_contract).expect("contract TLV validates");
    }

    #[test]
    fn matryoshka_dimensions_produce_distinct_spaces() {
        let policy = DimensionalityPolicy::matryoshka(vec![
            EffectivePrefix {
                dimension: 64,
                postprocessing: PrefixPostprocessing::DeterministicL2,
            },
            EffectivePrefix {
                dimension: 256,
                postprocessing: PrefixPostprocessing::DeterministicL2,
            },
            EffectivePrefix {
                dimension: 768,
                postprocessing: PrefixPostprocessing::None,
            },
        ])
        .expect("Matryoshka policy");
        let family = contract(policy).derive().expect("family derives");
        assert_eq!(family.spaces.len(), 3);
        assert_ne!(family.spaces[0].id, family.spaces[1].id);
        assert_ne!(family.spaces[1].id, family.spaces[2].id);
    }

    #[test]
    fn matryoshka_prefix_count_has_no_arbitrary_256_limit() {
        let prefixes = (1..=257)
            .map(|dimension| EffectivePrefix {
                dimension,
                postprocessing: PrefixPostprocessing::None,
            })
            .collect();
        let family = contract(
            DimensionalityPolicy::matryoshka(prefixes).expect("257-prefix Matryoshka policy"),
        )
        .derive()
        .expect("257-prefix family derives");
        assert_eq!(family.spaces.len(), 257);
        assert_eq!(family.spaces.last().map(|space| space.ordinal), Some(256));
    }

    #[test]
    fn dimensions_must_be_strictly_increasing() {
        let error = DimensionalityPolicy::matryoshka(vec![
            EffectivePrefix {
                dimension: 256,
                postprocessing: PrefixPostprocessing::None,
            },
            EffectivePrefix {
                dimension: 64,
                postprocessing: PrefixPostprocessing::None,
            },
        ])
        .expect_err("descending dimensions reject");
        assert!(matches!(error, EmbeddingError::NonCanonicalOrder(_)));
    }

    #[test]
    fn subject_projection_is_mandatory() {
        let policy =
            DimensionalityPolicy::fixed(16, PrefixPostprocessing::None).expect("fixed policy");
        let mut input = contract(policy);
        input.subject_projection = AppliedStage::NotApplied;
        assert!(input.derive().is_err());
    }

    #[test]
    fn tlv_rejects_unsorted_tags_and_nonzero_padding() {
        let mut block = Vec::new();
        push_tlv(&mut block, 2, TlvWireType::U32, true, &1u32.to_le_bytes()).expect("first field");
        push_tlv(&mut block, 1, TlvWireType::U32, true, &2u32.to_le_bytes()).expect("second field");
        assert!(matches!(
            canonical_tlv(&block),
            Err(EmbeddingError::NonCanonicalOrder(_))
        ));

        let mut padded = Vec::new();
        push_tlv(&mut padded, 1, TlvWireType::Bool, true, &[1]).expect("field");
        *padded.last_mut().expect("padding byte") = 9;
        assert!(matches!(
            canonical_tlv(&padded),
            Err(EmbeddingError::InvalidPadding { .. })
        ));
    }
}
