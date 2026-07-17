// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Borrowed, fail-closed views over deterministic PURREMB v1 artifacts.

use core::mem::size_of;

use crate::ContentDigest;

use super::contract::{PrefixPostprocessing, TlvEntryRef, TlvWireType, VectorDtype, canonical_tlv};
use super::error::EmbeddingError;
use super::identity::{
    ArtifactRoot, ChunkingContractId, ExternalBindingId, FamilyContractDigest, FamilyId,
    IndexGuardDigest, IndexId, MatrixContentDigest, MatrixId, ProjectionContentDigest,
    ProjectionId, TargetId, TargetIdentityDigest, TargetSetId, VectorSpaceId,
    derive_chunking_contract_id,
};
use super::target::TargetKind;
use super::wire::{
    PURREMB_DIRECTORY_ENTRY_LENGTH, PURREMB_FILE_ALIGNMENT, PURREMB_HEADER_LENGTH, PURREMB_MAGIC,
    PURREMB_MAX_SECTION_COUNT, PURREMB_TRAILER_LENGTH, PURREMB_TRAILER_MAGIC, PURREMB_VERSION,
    SECTION_CONTRACTS, SECTION_CRITICAL, SECTION_DERIVED, SECTION_EXTENSION_MIN,
    SECTION_EXTERNAL_BINDINGS, SECTION_INDEX_GUARDS, SECTION_INDEX_PAYLOAD, SECTION_MATRICES,
    SECTION_MATRIX_DATA, SECTION_RELATIONS, SECTION_SOURCE, SECTION_TARGET_SETS, SECTION_TARGETS,
    SECTION_TOKEN_SPANS, SectionKey, checked_align_up,
};

const SOURCE_LENGTH: usize = 128;
const FAMILY_HEADER_LENGTH: usize = 96;
const FAMILY_RECORD_LENGTH: usize = 96;
const SPACE_RECORD_LENGTH: usize = 80;
const TARGET_HEADER_LENGTH: usize = 64;
const TARGET_RECORD_LENGTH: usize = 96;
const TARGET_SET_RECORD_LENGTH: usize = 64;
const RELATION_RECORD_LENGTH: usize = 120;
const TOKEN_SPAN_RECORD_LENGTH: usize = 96;
const MATRIX_HEADER_LENGTH: usize = 96;
const MATRIX_RECORD_LENGTH: usize = 160;
const PROJECTION_RECORD_LENGTH: usize = 152;
const EXTERNAL_RECORD_LENGTH: usize = 192;
const INDEX_RECORD_LENGTH: usize = 336;

const IDENTITY_BYTES_PRESENT: u32 = 1;
const SOURCE_ORDINAL_PRESENT: u32 = 1 << 1;
const ROLE_BYTES_PRESENT: u32 = 1;
const CERTIFIED_RDF_PRESENT: u32 = 1;
const INDEX_REBUILDABLE: u32 = 1;

/// Integrity evidence currently associated with a borrowed view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingIntegrity {
    /// Bounds, canonical layout, record relationships, and metadata were checked.
    Structural,
    /// Cryptographic identities, scalar values, and projections were also checked.
    FullyVerified,
}

/// One borrowed PURREMB directory entry and its exact section bytes.
#[derive(Debug, Clone, Copy)]
pub struct SectionView<'a> {
    key: SectionKey,
    flags: u32,
    offset: u64,
    sha256: [u8; 32],
    bytes: &'a [u8],
}

impl<'a> SectionView<'a> {
    /// Section kind and instance.
    #[must_use]
    pub const fn key(self) -> SectionKey {
        self.key
    }

    /// Exact directory flags.
    #[must_use]
    pub const fn flags(self) -> u32 {
        self.flags
    }

    /// Absolute file offset.
    #[must_use]
    pub const fn offset(self) -> u64 {
        self.offset
    }

    /// Plain SHA-256 claimed by the directory.
    #[must_use]
    pub const fn stored_sha256(self) -> [u8; 32] {
        self.sha256
    }

    /// Exact borrowed section body.
    #[must_use]
    pub const fn bytes(self) -> &'a [u8] {
        self.bytes
    }
}

/// Exact source-pack attachment carried by the `SOURCE` section.
#[derive(Debug, Clone, Copy)]
pub struct SourceView {
    source_length: u64,
    source_exact_digest: ContentDigest,
    certified_rdf_digest: [u8; 32],
    dataset_target_id: TargetId,
}

impl SourceView {
    /// Required exact source byte length.
    #[must_use]
    pub const fn source_length(self) -> u64 {
        self.source_length
    }

    /// Plain SHA-256 of the exact source pack.
    #[must_use]
    pub const fn source_exact_digest(self) -> ContentDigest {
        self.source_exact_digest
    }

    /// Independently claimed RDFC SHA-256.
    #[must_use]
    pub const fn certified_rdf_digest(self) -> [u8; 32] {
        self.certified_rdf_digest
    }

    /// Dataset target bound to the certified digest.
    #[must_use]
    pub const fn dataset_target_id(self) -> TargetId {
        self.dataset_target_id
    }
}

/// Borrowed view of one vector-family record.
#[derive(Debug, Clone, Copy)]
pub struct FamilyView<'a> {
    record: &'a [u8],
    contract: &'a [u8],
    spaces: &'a [u8],
    space_count: usize,
}

impl<'a> FamilyView<'a> {
    /// Stable family ID carried by the record.
    #[must_use]
    pub fn id(self) -> FamilyId {
        FamilyId::from_raw(array32(self.record, 0))
    }

    /// Stored digest of the canonical contract block.
    #[must_use]
    pub fn contract_digest(self) -> FamilyContractDigest {
        FamilyContractDigest::from_raw(array32(self.record, 32))
    }

    /// Exact canonical family contract bytes.
    #[must_use]
    pub const fn contract_bytes(self) -> &'a [u8] {
        self.contract
    }

    /// Fixed (`1`) or Matryoshka (`2`) policy code.
    #[must_use]
    pub fn dimensionality_policy(self) -> u32 {
        infallible_u32(self.record, 80)
    }

    /// Authoritative stored dimension.
    #[must_use]
    pub fn stored_dimension(self) -> u32 {
        infallible_u32(self.record, 84)
    }

    /// Stored scalar type read from the canonical contract.
    pub fn dtype(self) -> Result<VectorDtype, EmbeddingError> {
        let entry = required_tlv(self.contract, 11, TlvWireType::U32, "contract dtype")?;
        VectorDtype::try_from(tlv_u32(entry)?)
    }

    /// Exact canonical chunking-stage block used for `ChunkingContractId`.
    pub fn chunking_stage_bytes(self) -> Result<&'a [u8], EmbeddingError> {
        Ok(required_tlv(self.contract, 7, TlvWireType::Block, "chunking stage")?.value)
    }

    /// Stable identity of the exact chunking-stage contract.
    pub fn chunking_contract_id(self) -> Result<ChunkingContractId, EmbeddingError> {
        Ok(derive_chunking_contract_id(self.chunking_stage_bytes()?))
    }

    /// Whether the family applies an explicit chunking stage.
    pub fn chunking_applied(self) -> Result<bool, EmbeddingError> {
        stage_is_applied(self.chunking_stage_bytes()?)
    }

    /// Exact canonical truncation-stage block.
    pub fn truncation_stage_bytes(self) -> Result<&'a [u8], EmbeddingError> {
        Ok(required_tlv(self.contract, 10, TlvWireType::Block, "truncation stage")?.value)
    }

    /// Whether the family applies explicit truncation.
    pub fn truncation_applied(self) -> Result<bool, EmbeddingError> {
        stage_is_applied(self.truncation_stage_bytes()?)
    }

    /// Number of effective fixed or Matryoshka spaces.
    #[must_use]
    pub const fn space_count(self) -> usize {
        self.space_count
    }

    /// Effective space at one family-local ordinal.
    #[must_use]
    pub fn space(self, ordinal: usize) -> Option<EffectiveSpaceView<'a>> {
        let start = ordinal.checked_mul(SPACE_RECORD_LENGTH)?;
        let record = self.spaces.get(start..start + SPACE_RECORD_LENGTH)?;
        Some(EffectiveSpaceView { record })
    }

    /// Effective spaces in dimension order.
    pub fn spaces(self) -> impl ExactSizeIterator<Item = EffectiveSpaceView<'a>> {
        (0..self.space_count).map(move |ordinal| {
            self.space(ordinal)
                .expect("structural validation fixed the family space range")
        })
    }
}

/// Borrowed view of one effective vector-space record.
#[derive(Debug, Clone, Copy)]
pub struct EffectiveSpaceView<'a> {
    record: &'a [u8],
}

impl EffectiveSpaceView<'_> {
    /// Stable vector-space ID.
    #[must_use]
    pub fn id(self) -> VectorSpaceId {
        VectorSpaceId::from_raw(array32(self.record, 0))
    }

    /// Owning family ID.
    #[must_use]
    pub fn family_id(self) -> FamilyId {
        FamilyId::from_raw(array32(self.record, 32))
    }

    /// Leading-prefix dimension.
    #[must_use]
    pub fn dimension(self) -> u32 {
        infallible_u32(self.record, 64)
    }

    /// Prefix postprocessing policy.
    pub fn postprocessing(self) -> Result<PrefixPostprocessing, EmbeddingError> {
        PrefixPostprocessing::try_from(infallible_u32(self.record, 68))
    }

    /// Zero-based ordinal within the family.
    #[must_use]
    pub fn ordinal(self) -> u32 {
        infallible_u32(self.record, 72)
    }
}

/// Borrowed canonical target record.
#[derive(Debug, Clone, Copy)]
pub struct TargetView<'a> {
    record: &'a [u8],
    identity: Option<&'a [u8]>,
}

impl<'a> TargetView<'a> {
    /// Stable target ID.
    #[must_use]
    pub fn id(self) -> TargetId {
        TargetId::from_raw(array32(self.record, 0))
    }

    /// Digest of the canonical target identity block.
    #[must_use]
    pub fn identity_digest(self) -> TargetIdentityDigest {
        TargetIdentityDigest::from_raw(array32(self.record, 32))
    }

    /// Stable target kind.
    pub fn kind(self) -> Result<TargetKind, EmbeddingError> {
        TargetKind::try_from(infallible_u32(self.record, 64))
    }

    /// Retained canonical identity bytes, if disclosed.
    #[must_use]
    pub const fn identity_bytes(self) -> Option<&'a [u8]> {
        self.identity
    }

    /// Source-pack-local acceleration ordinal, if present.
    #[must_use]
    pub fn source_local_ordinal(self) -> Option<u64> {
        (infallible_u32(self.record, 68) & SOURCE_ORDINAL_PRESENT != 0)
            .then(|| infallible_u64(self.record, 88))
    }
}

/// Borrowed, nonempty target row set.
#[derive(Debug, Clone, Copy)]
pub struct TargetSetView<'a> {
    record: &'a [u8],
    rows: &'a [u8],
}

impl<'a> TargetSetView<'a> {
    /// Stable target-set ID.
    #[must_use]
    pub fn id(self) -> TargetSetId {
        TargetSetId::from_raw(array32(self.record, 0))
    }

    /// Number of matrix rows.
    #[must_use]
    pub const fn row_count(self) -> usize {
        self.rows.len() / 32
    }

    /// O(1) row-to-target lookup.
    #[must_use]
    pub fn target(self, row: usize) -> Option<TargetId> {
        let start = row.checked_mul(32)?;
        Some(TargetId::from_raw(array32(
            self.rows.get(start..start + 32)?,
            0,
        )))
    }

    /// Allocation-free binary search from target ID to row.
    #[must_use]
    pub fn row_for_target(self, target: TargetId) -> Option<usize> {
        binary_search_ids(self.rows, target.as_bytes())
    }

    /// Target IDs in authoritative matrix row order.
    pub fn targets(self) -> impl ExactSizeIterator<Item = TargetId> + 'a {
        self.rows
            .chunks_exact(32)
            .map(|bytes| TargetId::from_raw(array32(bytes, 0)))
    }
}

/// Borrowed structural relation.
#[derive(Debug, Clone, Copy)]
pub struct RelationView<'a> {
    record: &'a [u8],
    role: Option<&'a [u8]>,
}

impl<'a> RelationView<'a> {
    /// Subject endpoint.
    #[must_use]
    pub fn subject(self) -> TargetId {
        TargetId::from_raw(array32(self.record, 0))
    }

    /// Object endpoint.
    #[must_use]
    pub fn object(self) -> TargetId {
        TargetId::from_raw(array32(self.record, 32))
    }

    /// Stable relation-kind code.
    #[must_use]
    pub fn kind_code(self) -> u32 {
        infallible_u32(self.record, 64)
    }

    /// Stored role digest, zero for built-in relations.
    #[must_use]
    pub fn role_digest(self) -> [u8; 32] {
        array32(self.record, 72)
    }

    /// Exact caller extension role, if this is an extension relation.
    #[must_use]
    pub const fn role_bytes(self) -> Option<&'a [u8]> {
        self.role
    }
}

/// Borrowed family-scoped token span.
#[derive(Debug, Clone, Copy)]
pub struct TokenSpanView<'a> {
    record: &'a [u8],
}

impl TokenSpanView<'_> {
    /// Tokenizer family.
    #[must_use]
    pub fn family_id(self) -> FamilyId {
        FamilyId::from_raw(array32(self.record, 0))
    }

    /// Document or chunk target.
    #[must_use]
    pub fn target_id(self) -> TargetId {
        TargetId::from_raw(array32(self.record, 32))
    }

    /// Inclusive token start.
    #[must_use]
    pub fn token_start(self) -> u64 {
        infallible_u64(self.record, 64)
    }

    /// Exclusive token end.
    #[must_use]
    pub fn token_end(self) -> u64 {
        infallible_u64(self.record, 72)
    }

    /// Actual model-input token count.
    #[must_use]
    pub fn model_input_token_count(self) -> u64 {
        infallible_u64(self.record, 80)
    }

    /// Stable truncation and special-token flags.
    #[must_use]
    pub fn flags(self) -> u32 {
        infallible_u32(self.record, 88)
    }
}

/// Borrowed authoritative dense matrix.
#[derive(Debug, Clone, Copy)]
pub struct MatrixView<'a> {
    record: &'a [u8],
    data: &'a [u8],
    fully_verified: bool,
}

impl<'a> MatrixView<'a> {
    /// Stable matrix ID.
    #[must_use]
    pub fn id(self) -> MatrixId {
        MatrixId::from_raw(array32(self.record, 0))
    }

    /// Typed digest of exact scalar bytes.
    #[must_use]
    pub fn content_digest(self) -> MatrixContentDigest {
        MatrixContentDigest::from_raw(array32(self.record, 32))
    }

    /// Target row set.
    #[must_use]
    pub fn target_set_id(self) -> TargetSetId {
        TargetSetId::from_raw(array32(self.record, 64))
    }

    /// Embedding family.
    #[must_use]
    pub fn family_id(self) -> FamilyId {
        FamilyId::from_raw(array32(self.record, 96))
    }

    /// Scalar representation.
    pub fn dtype(self) -> Result<VectorDtype, EmbeddingError> {
        VectorDtype::try_from(infallible_u32(self.record, 132))
    }

    /// Number of rows.
    #[must_use]
    pub fn row_count(self) -> u64 {
        infallible_u64(self.record, 136)
    }

    /// Scalars stored in each row.
    #[must_use]
    pub fn stored_dimension(self) -> u32 {
        infallible_u32(self.record, 144)
    }

    /// Exact borrowed matrix section.
    #[must_use]
    pub const fn data_bytes(self) -> &'a [u8] {
        self.data
    }

    /// Zero-copy exact little-endian bytes for one row.
    pub fn row_bytes(self, row: u64) -> Result<&'a [u8], EmbeddingError> {
        let width = u64::from(self.dtype()?.width());
        let row_length = u64::from(self.stored_dimension())
            .checked_mul(width)
            .ok_or(EmbeddingError::ArithmeticOverflow("matrix row length"))?;
        let offset = row
            .checked_mul(row_length)
            .ok_or(EmbeddingError::ArithmeticOverflow("matrix row offset"))?;
        borrowed_span(self.data, offset, row_length, "matrix row")
    }

    /// Portable finite-checking `f32` row iterator.
    pub fn f32_row(self, row: u64) -> Result<F32Scalars<'a>, EmbeddingError> {
        if self.dtype()? != VectorDtype::F32 {
            return Err(EmbeddingError::UnsupportedCode {
                field: "matrix dtype for f32 row",
                value: self.dtype()?.code(),
            });
        }
        Ok(F32Scalars::new(self.row_bytes(row)?, row, 0))
    }

    /// Portable finite-checking `f64` row iterator.
    pub fn f64_row(self, row: u64) -> Result<F64Scalars<'a>, EmbeddingError> {
        if self.dtype()? != VectorDtype::F64 {
            return Err(EmbeddingError::UnsupportedCode {
                field: "matrix dtype for f64 row",
                value: self.dtype()?.code(),
            });
        }
        Ok(F64Scalars::new(self.row_bytes(row)?, row, 0))
    }

    /// Native aligned `f32` row after full verification, when the host permits it.
    #[must_use]
    pub fn native_f32_row(self, row: u64) -> Option<&'a [f32]> {
        if !self.fully_verified
            || !cfg!(target_endian = "little")
            || self.dtype().ok()? != VectorDtype::F32
        {
            return None;
        }
        native_f32(self.row_bytes(row).ok()?)
    }

    /// Native aligned `f64` row after full verification, when the host permits it.
    #[must_use]
    pub fn native_f64_row(self, row: u64) -> Option<&'a [f64]> {
        if !self.fully_verified
            || !cfg!(target_endian = "little")
            || self.dtype().ok()? != VectorDtype::F64
        {
            return None;
        }
        native_f64(self.row_bytes(row).ok()?)
    }
}

/// Borrowed effective projection record.
#[derive(Debug, Clone, Copy)]
pub struct ProjectionView<'a> {
    record: &'a [u8],
}

impl ProjectionView<'_> {
    /// Stable projection ID.
    #[must_use]
    pub fn id(self) -> ProjectionId {
        ProjectionId::from_raw(array32(self.record, 0))
    }

    /// Typed digest of the logical row-prefix byte stream.
    #[must_use]
    pub fn content_digest(self) -> ProjectionContentDigest {
        ProjectionContentDigest::from_raw(array32(self.record, 32))
    }

    /// Authoritative stored matrix.
    #[must_use]
    pub fn matrix_id(self) -> MatrixId {
        MatrixId::from_raw(array32(self.record, 64))
    }

    /// Effective vector space.
    #[must_use]
    pub fn vector_space_id(self) -> VectorSpaceId {
        VectorSpaceId::from_raw(array32(self.record, 96))
    }

    /// Leading-prefix dimension.
    #[must_use]
    pub fn effective_dimension(self) -> u32 {
        infallible_u32(self.record, 128)
    }

    /// Prefix postprocessing policy.
    pub fn postprocessing(self) -> Result<PrefixPostprocessing, EmbeddingError> {
        PrefixPostprocessing::try_from(infallible_u32(self.record, 132))
    }

    /// Number of projected rows.
    #[must_use]
    pub fn row_count(self) -> u64 {
        infallible_u64(self.record, 136)
    }

    /// Length of the canonical logical projection stream.
    #[must_use]
    pub fn logical_byte_length(self) -> u64 {
        infallible_u64(self.record, 144)
    }
}

/// A matrix paired with one compatible fixed or Matryoshka projection.
#[derive(Debug, Clone, Copy)]
pub struct EffectiveMatrixView<'a> {
    matrix: MatrixView<'a>,
    projection: ProjectionView<'a>,
}

impl<'a> EffectiveMatrixView<'a> {
    /// Projection metadata.
    #[must_use]
    pub const fn projection(self) -> ProjectionView<'a> {
        self.projection
    }

    /// Authoritative stored matrix.
    #[must_use]
    pub const fn matrix(self) -> MatrixView<'a> {
        self.matrix
    }

    /// Zero-copy exact stored prefix bytes for one row.
    pub fn raw_prefix_bytes(self, row: u64) -> Result<&'a [u8], EmbeddingError> {
        let width = usize::try_from(self.matrix.dtype()?.width())
            .map_err(|_| EmbeddingError::ArithmeticOverflow("scalar width"))?;
        let length = usize::try_from(self.projection.effective_dimension())
            .map_err(|_| EmbeddingError::ArithmeticOverflow("prefix dimension"))?
            .checked_mul(width)
            .ok_or(EmbeddingError::ArithmeticOverflow("prefix byte length"))?;
        self.matrix
            .row_bytes(row)?
            .get(..length)
            .ok_or_else(|| EmbeddingError::UnavailablePrefix(self.projection.effective_dimension()))
    }

    /// Allocation-free logical `f32` projection row.
    pub fn f32_row(self, row: u64) -> Result<EffectiveF32Row<'a>, EmbeddingError> {
        if self.matrix.dtype()? != VectorDtype::F32 {
            return Err(EmbeddingError::UnsupportedCode {
                field: "projection dtype for f32 row",
                value: self.matrix.dtype()?.code(),
            });
        }
        let raw = F32Scalars::new(self.raw_prefix_bytes(row)?, row, 0);
        match self.projection.postprocessing()? {
            PrefixPostprocessing::None => Ok(EffectiveF32Row::Raw(raw)),
            PrefixPostprocessing::DeterministicL2 => {
                Ok(EffectiveF32Row::Normalized(L2F32Scalars::new(
                    self.raw_prefix_bytes(row)?,
                    row,
                    self.projection.effective_dimension(),
                )?))
            }
        }
    }

    /// Allocation-free logical `f64` projection row.
    pub fn f64_row(self, row: u64) -> Result<EffectiveF64Row<'a>, EmbeddingError> {
        if self.matrix.dtype()? != VectorDtype::F64 {
            return Err(EmbeddingError::UnsupportedCode {
                field: "projection dtype for f64 row",
                value: self.matrix.dtype()?.code(),
            });
        }
        let raw = F64Scalars::new(self.raw_prefix_bytes(row)?, row, 0);
        match self.projection.postprocessing()? {
            PrefixPostprocessing::None => Ok(EffectiveF64Row::Raw(raw)),
            PrefixPostprocessing::DeterministicL2 => {
                Ok(EffectiveF64Row::Normalized(L2F64Scalars::new(
                    self.raw_prefix_bytes(row)?,
                    row,
                    self.projection.effective_dimension(),
                )?))
            }
        }
    }

    /// Native stored `f32` prefix after full verification when no postprocessing applies.
    #[must_use]
    pub fn native_f32_row(self, row: u64) -> Option<&'a [f32]> {
        (self.matrix.fully_verified
            && cfg!(target_endian = "little")
            && self.matrix.dtype().ok()? == VectorDtype::F32
            && self.projection.postprocessing().ok()? == PrefixPostprocessing::None)
            .then(|| native_f32(self.raw_prefix_bytes(row).ok()?))?
    }

    /// Native stored `f64` prefix after full verification when no postprocessing applies.
    #[must_use]
    pub fn native_f64_row(self, row: u64) -> Option<&'a [f64]> {
        (self.matrix.fully_verified
            && cfg!(target_endian = "little")
            && self.matrix.dtype().ok()? == VectorDtype::F64
            && self.projection.postprocessing().ok()? == PrefixPostprocessing::None)
            .then(|| native_f64(self.raw_prefix_bytes(row).ok()?))?
    }
}

/// Portable little-endian `f32` decoder that rejects non-finite values lazily.
#[derive(Debug, Clone)]
pub struct F32Scalars<'a> {
    bytes: &'a [u8],
    position: usize,
    row: u64,
    first_column: u32,
}

impl<'a> F32Scalars<'a> {
    const fn new(bytes: &'a [u8], row: u64, first_column: u32) -> Self {
        Self {
            bytes,
            position: 0,
            row,
            first_column,
        }
    }
}

impl Iterator for F32Scalars<'_> {
    type Item = Result<f32, EmbeddingError>;

    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.bytes.get(self.position..self.position + 4)?;
        let bits = u32::from_le_bytes(chunk.try_into().ok()?);
        let value = f32::from_bits(bits);
        let column = self
            .first_column
            .saturating_add(u32::try_from(self.position / 4).unwrap_or(u32::MAX));
        self.position += 4;
        Some(if value.is_finite() {
            Ok(value)
        } else {
            Err(EmbeddingError::NonFiniteScalar {
                row: self.row,
                column,
            })
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.bytes.len() - self.position) / 4;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for F32Scalars<'_> {}

/// Portable little-endian `f64` decoder that rejects non-finite values lazily.
#[derive(Debug, Clone)]
pub struct F64Scalars<'a> {
    bytes: &'a [u8],
    position: usize,
    row: u64,
    first_column: u32,
}

impl<'a> F64Scalars<'a> {
    const fn new(bytes: &'a [u8], row: u64, first_column: u32) -> Self {
        Self {
            bytes,
            position: 0,
            row,
            first_column,
        }
    }
}

impl Iterator for F64Scalars<'_> {
    type Item = Result<f64, EmbeddingError>;

    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.bytes.get(self.position..self.position + 8)?;
        let bits = u64::from_le_bytes(chunk.try_into().ok()?);
        let value = f64::from_bits(bits);
        let column = self
            .first_column
            .saturating_add(u32::try_from(self.position / 8).unwrap_or(u32::MAX));
        self.position += 8;
        Some(if value.is_finite() {
            Ok(value)
        } else {
            Err(EmbeddingError::NonFiniteScalar {
                row: self.row,
                column,
            })
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.bytes.len() - self.position) / 8;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for F64Scalars<'_> {}

/// Logical `f32` prefix values, raw or deterministically L2-normalized.
#[derive(Debug, Clone)]
pub enum EffectiveF32Row<'a> {
    /// Exact stored leading-prefix values.
    Raw(F32Scalars<'a>),
    /// Values normalized by the normative binary64 fold.
    Normalized(L2F32Scalars<'a>),
}

impl Iterator for EffectiveF32Row<'_> {
    type Item = Result<f32, EmbeddingError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Raw(values) => values.next(),
            Self::Normalized(values) => values.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Raw(values) => values.size_hint(),
            Self::Normalized(values) => values.size_hint(),
        }
    }
}

impl ExactSizeIterator for EffectiveF32Row<'_> {}

/// Logical `f64` prefix values, raw or deterministically L2-normalized.
#[derive(Debug, Clone)]
pub enum EffectiveF64Row<'a> {
    /// Exact stored leading-prefix values.
    Raw(F64Scalars<'a>),
    /// Values normalized by the normative binary64 fold.
    Normalized(L2F64Scalars<'a>),
}

impl Iterator for EffectiveF64Row<'_> {
    type Item = Result<f64, EmbeddingError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Raw(values) => values.next(),
            Self::Normalized(values) => values.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Raw(values) => values.size_hint(),
            Self::Normalized(values) => values.size_hint(),
        }
    }
}

impl ExactSizeIterator for EffectiveF64Row<'_> {}

/// Allocation-free deterministic-L2 `f32` projection iterator.
#[derive(Debug, Clone)]
pub struct L2F32Scalars<'a> {
    raw: F32Scalars<'a>,
    norm: f64,
}

impl<'a> L2F32Scalars<'a> {
    fn new(bytes: &'a [u8], row: u64, dimension: u32) -> Result<Self, EmbeddingError> {
        let norm = deterministic_norm_f32(bytes, row, dimension)?;
        Ok(Self {
            raw: F32Scalars::new(bytes, row, 0),
            norm,
        })
    }
}

impl Iterator for L2F32Scalars<'_> {
    type Item = Result<f32, EmbeddingError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.raw
            .next()
            .map(|value| value.map(|value| (f64::from(value) / self.norm) as f32))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.raw.size_hint()
    }
}

impl ExactSizeIterator for L2F32Scalars<'_> {}

/// Allocation-free deterministic-L2 `f64` projection iterator.
#[derive(Debug, Clone)]
pub struct L2F64Scalars<'a> {
    raw: F64Scalars<'a>,
    norm: f64,
}

impl<'a> L2F64Scalars<'a> {
    fn new(bytes: &'a [u8], row: u64, dimension: u32) -> Result<Self, EmbeddingError> {
        let norm = deterministic_norm_f64(bytes, row, dimension)?;
        Ok(Self {
            raw: F64Scalars::new(bytes, row, 0),
            norm,
        })
    }
}

impl Iterator for L2F64Scalars<'_> {
    type Item = Result<f64, EmbeddingError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.raw
            .next()
            .map(|value| value.map(|value| value / self.norm))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.raw.size_hint()
    }
}

impl ExactSizeIterator for L2F64Scalars<'_> {}

/// Semantic type of an external-artifact binding scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ExternalScopeKind {
    /// Exact source SHA-256.
    ExactSource = 1,
    /// One target.
    Target = 2,
    /// One target row set.
    TargetSet = 3,
    /// One embedding family.
    Family = 4,
    /// One effective vector space.
    VectorSpace = 5,
    /// One authoritative matrix.
    Matrix = 6,
    /// One effective projection.
    Projection = 7,
    /// One opaque derived index.
    Index = 8,
}

impl TryFrom<u32> for ExternalScopeKind {
    type Error = EmbeddingError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::ExactSource),
            2 => Ok(Self::Target),
            3 => Ok(Self::TargetSet),
            4 => Ok(Self::Family),
            5 => Ok(Self::VectorSpace),
            6 => Ok(Self::Matrix),
            7 => Ok(Self::Projection),
            8 => Ok(Self::Index),
            value => Err(EmbeddingError::UnsupportedCode {
                field: "external binding scope",
                value,
            }),
        }
    }
}

/// Borrowed external-artifact binding record.
#[derive(Debug, Clone, Copy)]
pub struct ExternalBindingView<'a> {
    record: &'a [u8],
    contract: &'a [u8],
}

impl<'a> ExternalBindingView<'a> {
    /// Stable binding ID.
    #[must_use]
    pub fn id(self) -> ExternalBindingId {
        ExternalBindingId::from_raw(array32(self.record, 0))
    }

    /// Typed scope category.
    pub fn scope_kind(self) -> Result<ExternalScopeKind, EmbeddingError> {
        ExternalScopeKind::try_from(infallible_u32(self.record, 32))
    }

    /// Exact 32-byte scope ID.
    #[must_use]
    pub fn scope_id(self) -> [u8; 32] {
        array32(self.record, 40)
    }

    /// Plain SHA-256 of exact external bytes.
    #[must_use]
    pub fn artifact_sha256(self) -> ContentDigest {
        ContentDigest::from_raw(array32(self.record, 72))
    }

    /// Exact external byte length.
    #[must_use]
    pub fn artifact_length(self) -> u64 {
        infallible_u64(self.record, 104)
    }

    /// Independently claimed RDF digest, when present.
    #[must_use]
    pub fn certified_rdf_digest(self) -> Option<[u8; 32]> {
        (infallible_u32(self.record, 36) & CERTIFIED_RDF_PRESENT != 0)
            .then(|| array32(self.record, 112))
    }

    /// Stored digest of the canonical binding contract.
    #[must_use]
    pub fn contract_digest(self) -> super::identity::ExternalContractDigest {
        super::identity::ExternalContractDigest::from_raw(array32(self.record, 144))
    }

    /// Exact canonical binding contract.
    #[must_use]
    pub const fn contract_bytes(self) -> &'a [u8] {
        self.contract
    }
}

/// Opaque index storage mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum IndexStorage {
    /// Payload is a raw in-file section.
    Inline = 1,
    /// Payload is attached through an external binding.
    Detached = 2,
}

impl TryFrom<u32> for IndexStorage {
    type Error = EmbeddingError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Inline),
            2 => Ok(Self::Detached),
            value => Err(EmbeddingError::UnsupportedCode {
                field: "index storage",
                value,
            }),
        }
    }
}

/// Declared index-build determinism.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum IndexDeterminism {
    /// Equal guarded input produces equal payload bytes.
    Deterministic = 1,
    /// Payload identity remains exact but builds may differ.
    Nondeterministic = 2,
}

impl TryFrom<u32> for IndexDeterminism {
    type Error = EmbeddingError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Deterministic),
            2 => Ok(Self::Nondeterministic),
            value => Err(EmbeddingError::UnsupportedCode {
                field: "index determinism",
                value,
            }),
        }
    }
}

/// Borrowed opaque derived-index guard and optional inline payload.
#[derive(Debug, Clone, Copy)]
pub struct IndexGuardView<'a> {
    record: &'a [u8],
    guard: &'a [u8],
    payload: Option<&'a [u8]>,
}

impl<'a> IndexGuardView<'a> {
    /// Stable guarded-index ID.
    #[must_use]
    pub fn id(self) -> IndexId {
        IndexId::from_raw(array32(self.record, 0))
    }

    /// Exact source digest.
    #[must_use]
    pub fn source_exact_digest(self) -> ContentDigest {
        ContentDigest::from_raw(array32(self.record, 32))
    }

    /// Family ID.
    #[must_use]
    pub fn family_id(self) -> FamilyId {
        FamilyId::from_raw(array32(self.record, 64))
    }

    /// Effective vector-space ID.
    #[must_use]
    pub fn vector_space_id(self) -> VectorSpaceId {
        VectorSpaceId::from_raw(array32(self.record, 96))
    }

    /// Authoritative matrix ID.
    #[must_use]
    pub fn matrix_id(self) -> MatrixId {
        MatrixId::from_raw(array32(self.record, 128))
    }

    /// Effective projection ID.
    #[must_use]
    pub fn projection_id(self) -> ProjectionId {
        ProjectionId::from_raw(array32(self.record, 160))
    }

    /// Target row-set ID.
    #[must_use]
    pub fn target_set_id(self) -> TargetSetId {
        TargetSetId::from_raw(array32(self.record, 192))
    }

    /// Plain payload SHA-256.
    #[must_use]
    pub fn payload_sha256(self) -> ContentDigest {
        ContentDigest::from_raw(array32(self.record, 224))
    }

    /// Stored guard-contract digest.
    #[must_use]
    pub fn guard_digest(self) -> IndexGuardDigest {
        IndexGuardDigest::from_raw(array32(self.record, 256))
    }

    /// Exact payload length.
    #[must_use]
    pub fn payload_length(self) -> u64 {
        infallible_u64(self.record, 288)
    }

    /// Inline or detached storage.
    pub fn storage(self) -> Result<IndexStorage, EmbeddingError> {
        IndexStorage::try_from(infallible_u32(self.record, 316))
    }

    /// Declared build determinism.
    pub fn determinism(self) -> Result<IndexDeterminism, EmbeddingError> {
        IndexDeterminism::try_from(infallible_u32(self.record, 320))
    }

    /// Effective leading-prefix dimension.
    #[must_use]
    pub fn prefix_dimension(self) -> u32 {
        infallible_u32(self.record, 328)
    }

    /// Exact canonical guard contract.
    #[must_use]
    pub const fn guard_bytes(self) -> &'a [u8] {
        self.guard
    }

    /// Exact inline payload, absent for detached storage.
    #[must_use]
    pub const fn payload_bytes(self) -> Option<&'a [u8]> {
        self.payload
    }
}

#[derive(Debug, Clone, Copy)]
struct DirectoryView<'a> {
    file: &'a [u8],
    entries: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
struct ContractsView<'a> {
    bytes: &'a [u8],
    family_count: usize,
    space_count: usize,
}

#[derive(Debug, Clone, Copy)]
struct TargetsView<'a> {
    bytes: &'a [u8],
    count: usize,
}

#[derive(Debug, Clone, Copy)]
struct TargetSetsView<'a> {
    bytes: &'a [u8],
    count: usize,
    row_pool_offset: usize,
}

#[derive(Debug, Clone, Copy)]
struct RelationsView<'a> {
    bytes: &'a [u8],
    count: usize,
}

#[derive(Debug, Clone, Copy)]
struct TokenSpansView<'a> {
    bytes: &'a [u8],
    count: usize,
}

#[derive(Debug, Clone, Copy)]
struct MatricesView<'a> {
    bytes: &'a [u8],
    matrix_count: usize,
    projection_count: usize,
    projection_offset: usize,
}

#[derive(Debug, Clone, Copy)]
struct ExternalBindingsView<'a> {
    bytes: &'a [u8],
    count: usize,
}

#[derive(Debug, Clone, Copy)]
struct IndexGuardsView<'a> {
    bytes: &'a [u8],
    count: usize,
}

/// Bounds-safe borrowed view over one structurally canonical PURREMB artifact.
#[derive(Debug)]
pub struct EmbeddingView<'a> {
    bytes: &'a [u8],
    root: ArtifactRoot,
    source: SourceView,
    directory: DirectoryView<'a>,
    contracts: ContractsView<'a>,
    targets: TargetsView<'a>,
    target_sets: TargetSetsView<'a>,
    relations: RelationsView<'a>,
    token_spans: TokenSpansView<'a>,
    matrices: MatricesView<'a>,
    external_bindings: ExternalBindingsView<'a>,
    index_guards: IndexGuardsView<'a>,
    integrity: EmbeddingIntegrity,
}

impl<'a> EmbeddingView<'a> {
    /// Performs a structural, allocation-free open over immutable bytes.
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, EmbeddingError> {
        let (directory, root) = open_framing(bytes)?;
        let source_section = required_section(directory, SECTION_SOURCE, 0)?;
        let source = parse_source(source_section.bytes, bytes)?;
        let contracts = parse_contracts(required_section(directory, SECTION_CONTRACTS, 0)?.bytes)?;
        let targets = parse_targets(required_section(directory, SECTION_TARGETS, 0)?.bytes)?;
        let target_sets =
            parse_target_sets(required_section(directory, SECTION_TARGET_SETS, 0)?.bytes)?;
        let relations = parse_relations(required_section(directory, SECTION_RELATIONS, 0)?.bytes)?;
        let token_spans =
            parse_token_spans(required_section(directory, SECTION_TOKEN_SPANS, 0)?.bytes)?;
        let matrices = parse_matrices(required_section(directory, SECTION_MATRICES, 0)?.bytes)?;
        let external_bindings = parse_external_bindings(
            required_section(directory, SECTION_EXTERNAL_BINDINGS, 0)?.bytes,
        )?;
        let index_guards =
            parse_index_guards(required_section(directory, SECTION_INDEX_GUARDS, 0)?.bytes)?;

        let view = Self {
            bytes,
            root,
            source,
            directory,
            contracts,
            targets,
            target_sets,
            relations,
            token_spans,
            matrices,
            external_bindings,
            index_guards,
            integrity: EmbeddingIntegrity::Structural,
        };
        view.validate_cross_references()?;
        Ok(view)
    }

    /// Exact immutable bytes backing this view.
    #[must_use]
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// Stored whole-artifact integrity root.
    #[must_use]
    pub const fn artifact_root(&self) -> ArtifactRoot {
        self.root
    }

    /// Current resident verification state.
    #[must_use]
    pub const fn integrity(&self) -> EmbeddingIntegrity {
        self.integrity
    }

    /// Exact source-pack attachment.
    #[must_use]
    pub const fn source(&self) -> SourceView {
        self.source
    }

    /// Number of directory entries.
    #[must_use]
    pub const fn section_count(&self) -> usize {
        self.directory.entries.len() / 64
    }

    /// Directory entry by canonical index.
    #[must_use]
    pub fn section_at(&self, index: usize) -> Option<SectionView<'a>> {
        self.directory.at(index)
    }

    /// Directory entries in canonical order.
    pub fn sections(&self) -> impl ExactSizeIterator<Item = SectionView<'a>> + '_ {
        (0..self.section_count()).map(|index| {
            self.section_at(index)
                .expect("structural validation fixed the section directory")
        })
    }

    /// Looks up one section kind and instance.
    #[must_use]
    pub fn section(&self, kind: u32, instance: u32) -> Option<SectionView<'a>> {
        self.directory.find(SectionKey::new(kind, instance))
    }

    /// Number of embedding families.
    #[must_use]
    pub const fn family_count(&self) -> usize {
        self.contracts.family_count
    }

    /// Family by canonical record index.
    #[must_use]
    pub fn family_at(&self, index: usize) -> Option<FamilyView<'a>> {
        self.contracts.family_at(index)
    }

    /// Embedding families in stable ID order.
    pub fn families(&self) -> impl ExactSizeIterator<Item = FamilyView<'a>> + '_ {
        (0..self.family_count()).map(|index| {
            self.family_at(index)
                .expect("structural validation fixed the family table")
        })
    }

    /// Finds a family by ID.
    #[must_use]
    pub fn family(&self, id: FamilyId) -> Option<FamilyView<'a>> {
        binary_search_records(
            self.contracts.bytes,
            FAMILY_HEADER_LENGTH,
            FAMILY_RECORD_LENGTH,
            self.contracts.family_count,
            id.as_bytes(),
        )
        .and_then(|index| self.family_at(index))
    }

    /// Number of effective vector spaces.
    #[must_use]
    pub const fn vector_space_count(&self) -> usize {
        self.contracts.space_count
    }

    /// Finds one effective vector space by stable ID.
    #[must_use]
    pub fn vector_space(&self, id: VectorSpaceId) -> Option<EffectiveSpaceView<'a>> {
        (0..self.contracts.space_count)
            .filter_map(|index| self.contracts.space_at(index))
            .find(|space| space.id() == id)
    }

    /// Number of canonical targets.
    #[must_use]
    pub const fn target_count(&self) -> usize {
        self.targets.count
    }

    /// Target by canonical index.
    #[must_use]
    pub fn target_at(&self, index: usize) -> Option<TargetView<'a>> {
        self.targets.target_at(index)
    }

    /// Targets in stable ID order.
    pub fn targets(&self) -> impl ExactSizeIterator<Item = TargetView<'a>> + '_ {
        (0..self.target_count()).map(|index| {
            self.target_at(index)
                .expect("structural validation fixed the target table")
        })
    }

    /// Finds a target by ID with allocation-free binary search.
    #[must_use]
    pub fn target(&self, id: TargetId) -> Option<TargetView<'a>> {
        binary_search_records(
            self.targets.bytes,
            TARGET_HEADER_LENGTH,
            TARGET_RECORD_LENGTH,
            self.targets.count,
            id.as_bytes(),
        )
        .and_then(|index| self.target_at(index))
    }

    /// Number of target row sets.
    #[must_use]
    pub const fn target_set_count(&self) -> usize {
        self.target_sets.count
    }

    /// Target row set by canonical index.
    #[must_use]
    pub fn target_set_at(&self, index: usize) -> Option<TargetSetView<'a>> {
        self.target_sets.set_at(index)
    }

    /// Target row sets in stable ID order.
    pub fn target_sets(&self) -> impl ExactSizeIterator<Item = TargetSetView<'a>> + '_ {
        (0..self.target_set_count()).map(|index| {
            self.target_set_at(index)
                .expect("structural validation fixed the target-set table")
        })
    }

    /// Finds a target row set by ID.
    #[must_use]
    pub fn target_set(&self, id: TargetSetId) -> Option<TargetSetView<'a>> {
        binary_search_records(
            self.target_sets.bytes,
            TARGET_HEADER_LENGTH,
            TARGET_SET_RECORD_LENGTH,
            self.target_sets.count,
            id.as_bytes(),
        )
        .and_then(|index| self.target_set_at(index))
    }

    /// Number of structural relations.
    #[must_use]
    pub const fn relation_count(&self) -> usize {
        self.relations.count
    }

    /// Relation by canonical index.
    #[must_use]
    pub fn relation_at(&self, index: usize) -> Option<RelationView<'a>> {
        self.relations.relation_at(index)
    }

    /// Structural relations in canonical key order.
    pub fn relations(&self) -> impl ExactSizeIterator<Item = RelationView<'a>> + '_ {
        (0..self.relation_count()).map(|index| {
            self.relation_at(index)
                .expect("structural validation fixed the relation table")
        })
    }

    /// Contiguous relation range for one subject target.
    #[must_use]
    pub fn relations_for(&self, subject: TargetId) -> RelationRange<'_, 'a> {
        let mut start = 0;
        let mut end = self.relation_count();
        while start < end {
            let middle = start + (end - start) / 2;
            let relation = self.relation_at(middle).expect("validated relation index");
            if relation.subject() < subject {
                start = middle + 1;
            } else {
                end = middle;
            }
        }
        let first = start;
        while start < self.relation_count()
            && self
                .relation_at(start)
                .is_some_and(|relation| relation.subject() == subject)
        {
            start += 1;
        }
        RelationRange {
            view: self,
            next: first,
            end: start,
        }
    }

    /// Number of family-scoped token spans.
    #[must_use]
    pub const fn token_span_count(&self) -> usize {
        self.token_spans.count
    }

    /// Token span by canonical index.
    #[must_use]
    pub fn token_span_at(&self, index: usize) -> Option<TokenSpanView<'a>> {
        self.token_spans.span_at(index)
    }

    /// Token spans in `(family, target)` order.
    pub fn token_spans(&self) -> impl ExactSizeIterator<Item = TokenSpanView<'a>> + '_ {
        (0..self.token_span_count()).map(|index| {
            self.token_span_at(index)
                .expect("structural validation fixed the token-span table")
        })
    }

    /// Finds the unique span for a family and target.
    #[must_use]
    pub fn token_span(&self, family: FamilyId, target: TargetId) -> Option<TokenSpanView<'a>> {
        let key = (family, target);
        let mut low = 0;
        let mut high = self.token_span_count();
        while low < high {
            let middle = low + (high - low) / 2;
            let span = self.token_span_at(middle)?;
            match (span.family_id(), span.target_id()).cmp(&key) {
                core::cmp::Ordering::Less => low = middle + 1,
                core::cmp::Ordering::Greater => high = middle,
                core::cmp::Ordering::Equal => return Some(span),
            }
        }
        None
    }

    /// Number of authoritative matrices.
    #[must_use]
    pub const fn matrix_count(&self) -> usize {
        self.matrices.matrix_count
    }

    /// Matrix by canonical index.
    #[must_use]
    pub fn matrix_at(&self, index: usize) -> Option<MatrixView<'a>> {
        self.matrices.matrix_at(
            index,
            self.directory,
            self.integrity == EmbeddingIntegrity::FullyVerified,
        )
    }

    /// Authoritative matrices in stable ID order.
    pub fn matrices(&self) -> impl ExactSizeIterator<Item = MatrixView<'a>> + '_ {
        (0..self.matrix_count()).map(|index| {
            self.matrix_at(index)
                .expect("structural validation fixed the matrix table")
        })
    }

    /// Finds a matrix by ID.
    #[must_use]
    pub fn matrix(&self, id: MatrixId) -> Option<MatrixView<'a>> {
        binary_search_records(
            self.matrices.bytes,
            MATRIX_HEADER_LENGTH,
            MATRIX_RECORD_LENGTH,
            self.matrices.matrix_count,
            id.as_bytes(),
        )
        .and_then(|index| self.matrix_at(index))
    }

    /// Number of effective projection records.
    #[must_use]
    pub const fn projection_count(&self) -> usize {
        self.matrices.projection_count
    }

    /// Projection by physical table index.
    #[must_use]
    pub fn projection_at(&self, index: usize) -> Option<ProjectionView<'a>> {
        self.matrices.projection_at(index)
    }

    /// Effective projections in matrix grouping order.
    pub fn projections(&self) -> impl ExactSizeIterator<Item = ProjectionView<'a>> + '_ {
        (0..self.projection_count()).map(|index| {
            self.projection_at(index)
                .expect("structural validation fixed the projection table")
        })
    }

    /// Finds a projection by stable ID.
    #[must_use]
    pub fn projection(&self, id: ProjectionId) -> Option<ProjectionView<'a>> {
        self.projections().find(|projection| projection.id() == id)
    }

    /// Finds the unique effective matrix for a target set and vector space.
    pub fn effective_matrix(
        &self,
        target_set: TargetSetId,
        vector_space: VectorSpaceId,
    ) -> Result<Option<EffectiveMatrixView<'a>>, EmbeddingError> {
        let mut found = None;
        for projection in self.projections() {
            if projection.vector_space_id() != vector_space {
                continue;
            }
            let matrix = self
                .matrix(projection.matrix_id())
                .ok_or(EmbeddingError::MissingReference("projection matrix"))?;
            if matrix.target_set_id() == target_set {
                if found.is_some() {
                    return Err(EmbeddingError::Duplicate(
                        "target-set/vector-space projection",
                    ));
                }
                found = Some(EffectiveMatrixView { matrix, projection });
            }
        }
        Ok(found)
    }

    /// Number of external-artifact bindings.
    #[must_use]
    pub const fn external_binding_count(&self) -> usize {
        self.external_bindings.count
    }

    /// External binding by canonical index.
    #[must_use]
    pub fn external_binding_at(&self, index: usize) -> Option<ExternalBindingView<'a>> {
        self.external_bindings.binding_at(index)
    }

    /// External bindings in stable ID order.
    pub fn external_bindings(&self) -> impl ExactSizeIterator<Item = ExternalBindingView<'a>> + '_ {
        (0..self.external_binding_count()).map(|index| {
            self.external_binding_at(index)
                .expect("structural validation fixed the external-binding table")
        })
    }

    /// Finds an external binding by ID.
    #[must_use]
    pub fn external_binding(&self, id: ExternalBindingId) -> Option<ExternalBindingView<'a>> {
        binary_search_records(
            self.external_bindings.bytes,
            TARGET_HEADER_LENGTH,
            EXTERNAL_RECORD_LENGTH,
            self.external_bindings.count,
            id.as_bytes(),
        )
        .and_then(|index| self.external_binding_at(index))
    }

    /// Number of opaque derived-index guards.
    #[must_use]
    pub const fn index_guard_count(&self) -> usize {
        self.index_guards.count
    }

    /// Index guard by canonical index.
    #[must_use]
    pub fn index_guard_at(&self, index: usize) -> Option<IndexGuardView<'a>> {
        self.index_guards.guard_at(index, self.directory)
    }

    /// Index guards in stable ID order.
    pub fn index_guards(&self) -> impl ExactSizeIterator<Item = IndexGuardView<'a>> + '_ {
        (0..self.index_guard_count()).map(|index| {
            self.index_guard_at(index)
                .expect("structural validation fixed the index-guard table")
        })
    }

    /// Finds an index guard by ID.
    #[must_use]
    pub fn index_guard(&self, id: IndexId) -> Option<IndexGuardView<'a>> {
        binary_search_records(
            self.index_guards.bytes,
            TARGET_HEADER_LENGTH,
            INDEX_RECORD_LENGTH,
            self.index_guards.count,
            id.as_bytes(),
        )
        .and_then(|index| self.index_guard_at(index))
    }

    /// Verifies complete built-in hierarchy and RDF component relations.
    ///
    /// Structural open remains allocation-free. Full verification calls this
    /// method and uses two linear relation catalogs so digest-only targets can
    /// also prove their required incoming edges without quadratic scans.
    pub(crate) fn verify_relation_completeness(&self) -> Result<(), EmbeddingError> {
        validate_complete_relations(self)
    }

    /// Marks this already-validated borrowed view for native typed access.
    pub(crate) const fn mark_fully_verified(&mut self) {
        self.integrity = EmbeddingIntegrity::FullyVerified;
    }
}

type BorrowedBuiltinEdge = (TargetId, u32, TargetId);

fn validate_complete_relations(view: &EmbeddingView<'_>) -> Result<(), EmbeddingError> {
    let edges = view
        .relations()
        .filter(|relation| relation.kind_code() != 0x8000_0000)
        .map(|relation| (relation.subject(), relation.kind_code(), relation.object()))
        .collect::<Vec<_>>();
    let mut incoming = edges
        .iter()
        .map(|(subject, kind, object)| (*object, *kind, *subject))
        .collect::<Vec<_>>();
    incoming.sort_unstable();

    for target in view.targets() {
        match target.kind()? {
            TargetKind::Corpus | TargetKind::RdfDataset | TargetKind::Extension => {}
            TargetKind::Document => {
                require_borrowed_edge_count(&incoming, target.id(), 1, 1)?;
                if let Some(parent) = borrowed_target_id(target, 1)? {
                    require_borrowed_edge(&edges, parent, 1, target.id())?;
                }
            }
            TargetKind::Chunk => {
                require_borrowed_edge_count(&incoming, target.id(), 2, 1)?;
                if let Some(parent) = borrowed_target_id(target, 1)? {
                    require_borrowed_edge(&edges, parent, 2, target.id())?;
                }
            }
            TargetKind::RdfGraph => {
                require_borrowed_edge_count(&incoming, target.id(), 16, 1)?;
                if let Some(dataset) = borrowed_target_id(target, 1)? {
                    require_borrowed_edge(&edges, dataset, 16, target.id())?;
                    match borrowed_u32(target, 2)?
                        .ok_or(EmbeddingError::Missing("retained RDF graph form"))?
                    {
                        0 => require_borrowed_edge_count(&edges, target.id(), 26, 0)?,
                        1 => {
                            let name =
                                borrowed_required_target_id(target, 3, "retained RDF graph name")?;
                            require_borrowed_edge_count(&edges, target.id(), 26, 1)?;
                            require_borrowed_edge(&edges, target.id(), 26, name)?;
                            validate_borrowed_term_position(view, name, &[1, 2], "RDF graph name")?;
                        }
                        value => {
                            return Err(EmbeddingError::UnsupportedCode {
                                field: "RDF graph form",
                                value,
                            });
                        }
                    }
                } else if borrowed_edge_count(&edges, target.id(), 26) > 1 {
                    return Err(EmbeddingError::Duplicate("RDF graph name relation"));
                }
            }
            TargetKind::RdfStatement => {
                require_borrowed_edge_count(&incoming, target.id(), 17, 1)?;
                for kind in 18..=20 {
                    require_borrowed_edge_count(&edges, target.id(), kind, 1)?;
                }
                if let Some(graph) = borrowed_target_id(target, 1)? {
                    require_borrowed_edge(&edges, graph, 17, target.id())?;
                    let subject = borrowed_required_target_id(target, 2, "statement subject")?;
                    let predicate = borrowed_required_target_id(target, 3, "statement predicate")?;
                    let object = borrowed_required_target_id(target, 4, "statement object")?;
                    require_borrowed_edge(&edges, target.id(), 18, subject)?;
                    require_borrowed_edge(&edges, target.id(), 19, predicate)?;
                    require_borrowed_edge(&edges, target.id(), 20, object)?;
                    validate_borrowed_term_position(view, subject, &[1, 2, 4], "RDF subject")?;
                    validate_borrowed_term_position(view, predicate, &[1], "RDF predicate")?;
                }
            }
            TargetKind::RdfReifier => {
                require_borrowed_edge_count(&incoming, target.id(), 21, 1)?;
                require_borrowed_edge_count(&edges, target.id(), 22, 1)?;
                if borrowed_target_id(target, 1)?.is_some() {
                    let graph = borrowed_required_target_id(target, 1, "reifier graph")?;
                    let statement = borrowed_required_target_id(target, 2, "reified statement")?;
                    let term = borrowed_required_target_id(target, 3, "reifier term")?;
                    require_borrowed_edge(&edges, statement, 21, target.id())?;
                    require_borrowed_edge(&edges, target.id(), 22, term)?;
                    validate_borrowed_term_position(view, term, &[1, 2], "RDF reifier term")?;
                    require_borrowed_composite_graph(
                        view,
                        statement,
                        graph,
                        "reifier statement graph",
                    )?;
                }
            }
            TargetKind::RdfAnnotation => {
                require_borrowed_edge_count(&incoming, target.id(), 23, 1)?;
                require_borrowed_edge_count(&edges, target.id(), 24, 1)?;
                require_borrowed_edge_count(&edges, target.id(), 25, 1)?;
                if borrowed_target_id(target, 1)?.is_some() {
                    let graph = borrowed_required_target_id(target, 1, "annotation graph")?;
                    let reifier = borrowed_required_target_id(target, 2, "annotation reifier")?;
                    let predicate = borrowed_required_target_id(target, 3, "annotation predicate")?;
                    let object = borrowed_required_target_id(target, 4, "annotation object")?;
                    require_borrowed_edge(&edges, reifier, 23, target.id())?;
                    require_borrowed_edge(&edges, target.id(), 24, predicate)?;
                    require_borrowed_edge(&edges, target.id(), 25, object)?;
                    validate_borrowed_term_position(view, predicate, &[1], "annotation predicate")?;
                    require_borrowed_composite_graph(
                        view,
                        reifier,
                        graph,
                        "annotation reifier graph",
                    )?;
                }
            }
            TargetKind::RdfTerm => validate_borrowed_triple_relations(view, &edges, target)?,
        }
    }
    Ok(())
}

fn validate_borrowed_triple_relations(
    view: &EmbeddingView<'_>,
    edges: &[BorrowedBuiltinEdge],
    target: TargetView<'_>,
) -> Result<(), EmbeddingError> {
    let counts = [32, 33, 34].map(|kind| borrowed_edge_count(edges, target.id(), kind));
    match borrowed_u32(target, 1)? {
        Some(4) => {
            for (tag, kind) in (2..=4).zip(32..=34) {
                require_borrowed_edge_count(edges, target.id(), kind, 1)?;
                let component = borrowed_required_target_id(target, tag, "triple-term component")?;
                require_borrowed_edge(edges, target.id(), kind, component)?;
                if tag == 2 {
                    validate_borrowed_term_position(view, component, &[1, 2, 4], "triple subject")?;
                } else if tag == 3 {
                    validate_borrowed_term_position(view, component, &[1], "triple predicate")?;
                }
            }
        }
        Some(_) => {
            if counts != [0, 0, 0] {
                return Err(EmbeddingError::Malformed(
                    "non-triple RDF term has triple-component relations",
                ));
            }
        }
        None => {
            if counts.iter().any(|count| *count > 1)
                || !(counts == [0, 0, 0] || counts == [1, 1, 1])
            {
                return Err(EmbeddingError::Malformed(
                    "digest-only triple-term relation coverage",
                ));
            }
        }
    }
    Ok(())
}

fn borrowed_required_target_id(
    target: TargetView<'_>,
    tag: u16,
    context: &'static str,
) -> Result<TargetId, EmbeddingError> {
    borrowed_target_id(target, tag)?.ok_or(EmbeddingError::Missing(context))
}

fn borrowed_target_id(
    target: TargetView<'_>,
    tag: u16,
) -> Result<Option<TargetId>, EmbeddingError> {
    target
        .identity_bytes()
        .map(|identity| required_digest(identity, tag, "retained target reference"))
        .transpose()
        .map(|value| value.map(TargetId::from_raw))
}

fn borrowed_u32(target: TargetView<'_>, tag: u16) -> Result<Option<u32>, EmbeddingError> {
    target
        .identity_bytes()
        .map(|identity| {
            tlv_u32(required_tlv(
                identity,
                tag,
                TlvWireType::U32,
                "retained target u32",
            )?)
        })
        .transpose()
}

fn validate_borrowed_term_position(
    view: &EmbeddingView<'_>,
    id: TargetId,
    allowed_forms: &[u32],
    context: &'static str,
) -> Result<(), EmbeddingError> {
    let term = view
        .target(id)
        .ok_or(EmbeddingError::MissingReference(context))?;
    if term.kind()? != TargetKind::RdfTerm {
        return Err(EmbeddingError::Malformed(context));
    }
    if let Some(form) = borrowed_u32(term, 1)?
        && !allowed_forms.contains(&form)
    {
        return Err(EmbeddingError::Malformed(context));
    }
    Ok(())
}

fn require_borrowed_composite_graph(
    view: &EmbeddingView<'_>,
    composite: TargetId,
    expected_graph: TargetId,
    context: &'static str,
) -> Result<(), EmbeddingError> {
    let target = view
        .target(composite)
        .ok_or(EmbeddingError::MissingReference(context))?;
    if let Some(actual_graph) = borrowed_target_id(target, 1)?
        && actual_graph != expected_graph
    {
        return Err(EmbeddingError::Malformed(context));
    }
    Ok(())
}

fn require_borrowed_edge(
    edges: &[BorrowedBuiltinEdge],
    subject: TargetId,
    kind: u32,
    object: TargetId,
) -> Result<(), EmbeddingError> {
    if edges.binary_search(&(subject, kind, object)).is_err() {
        return Err(EmbeddingError::MissingReference(
            "required built-in target relation",
        ));
    }
    Ok(())
}

fn require_borrowed_edge_count(
    edges: &[BorrowedBuiltinEdge],
    target: TargetId,
    kind: u32,
    expected: usize,
) -> Result<(), EmbeddingError> {
    if borrowed_edge_count(edges, target, kind) != expected {
        return Err(EmbeddingError::Malformed(
            "built-in target relation cardinality",
        ));
    }
    Ok(())
}

fn borrowed_edge_count(edges: &[BorrowedBuiltinEdge], target: TargetId, kind: u32) -> usize {
    let start = edges.partition_point(|edge| (edge.0, edge.1) < (target, kind));
    let end = edges.partition_point(|edge| (edge.0, edge.1) <= (target, kind));
    end - start
}

/// Iterator over one target's contiguous relation range.
#[derive(Debug)]
pub struct RelationRange<'v, 'a> {
    view: &'v EmbeddingView<'a>,
    next: usize,
    end: usize,
}

impl<'a> Iterator for RelationRange<'_, 'a> {
    type Item = RelationView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next == self.end {
            return None;
        }
        let relation = self.view.relation_at(self.next);
        self.next += 1;
        relation
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.end - self.next;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for RelationRange<'_, '_> {}

impl<'a> DirectoryView<'a> {
    fn at(self, index: usize) -> Option<SectionView<'a>> {
        let start = index.checked_mul(64)?;
        let entry = self.entries.get(start..start + 64)?;
        let offset = infallible_u64(entry, 16);
        let length = infallible_u64(entry, 24);
        Some(SectionView {
            key: SectionKey::new(infallible_u32(entry, 0), infallible_u32(entry, 8)),
            flags: infallible_u32(entry, 4),
            offset,
            sha256: array32(entry, 32),
            bytes: borrowed_span(self.file, offset, length, "directory section").ok()?,
        })
    }

    fn find(self, key: SectionKey) -> Option<SectionView<'a>> {
        let mut low = 0;
        let mut high = self.entries.len() / 64;
        while low < high {
            let middle = low + (high - low) / 2;
            let section = self.at(middle)?;
            match section.key.cmp(&key) {
                core::cmp::Ordering::Less => low = middle + 1,
                core::cmp::Ordering::Greater => high = middle,
                core::cmp::Ordering::Equal => return Some(section),
            }
        }
        None
    }
}

impl<'a> ContractsView<'a> {
    fn family_at(self, index: usize) -> Option<FamilyView<'a>> {
        let start = FAMILY_HEADER_LENGTH.checked_add(index.checked_mul(FAMILY_RECORD_LENGTH)?)?;
        let record = self.bytes.get(start..start + FAMILY_RECORD_LENGTH)?;
        let contract = borrowed_span(
            self.bytes,
            infallible_u64(record, 64),
            infallible_u64(record, 72),
            "family contract",
        )
        .ok()?;
        let space_start = usize::try_from(infallible_u32(record, 88)).ok()?;
        let space_count = usize::try_from(infallible_u32(record, 92)).ok()?;
        let table_offset = usize::try_from(infallible_u64(self.bytes, 40)).ok()?;
        let byte_start = table_offset.checked_add(space_start.checked_mul(SPACE_RECORD_LENGTH)?)?;
        let byte_length = space_count.checked_mul(SPACE_RECORD_LENGTH)?;
        let spaces = self.bytes.get(byte_start..byte_start + byte_length)?;
        Some(FamilyView {
            record,
            contract,
            spaces,
            space_count,
        })
    }

    fn space_at(self, index: usize) -> Option<EffectiveSpaceView<'a>> {
        let table_offset = usize::try_from(infallible_u64(self.bytes, 40)).ok()?;
        let start = table_offset.checked_add(index.checked_mul(SPACE_RECORD_LENGTH)?)?;
        Some(EffectiveSpaceView {
            record: self.bytes.get(start..start + SPACE_RECORD_LENGTH)?,
        })
    }
}

impl<'a> TargetsView<'a> {
    fn target_at(self, index: usize) -> Option<TargetView<'a>> {
        let start = TARGET_HEADER_LENGTH.checked_add(index.checked_mul(TARGET_RECORD_LENGTH)?)?;
        let record = self.bytes.get(start..start + TARGET_RECORD_LENGTH)?;
        let identity = (infallible_u32(record, 68) & IDENTITY_BYTES_PRESENT != 0)
            .then(|| {
                borrowed_span(
                    self.bytes,
                    infallible_u64(record, 72),
                    infallible_u64(record, 80),
                    "target identity",
                )
                .ok()
            })
            .flatten();
        Some(TargetView { record, identity })
    }
}

impl<'a> TargetSetsView<'a> {
    fn set_at(self, index: usize) -> Option<TargetSetView<'a>> {
        let start =
            TARGET_HEADER_LENGTH.checked_add(index.checked_mul(TARGET_SET_RECORD_LENGTH)?)?;
        let record = self.bytes.get(start..start + TARGET_SET_RECORD_LENGTH)?;
        let row_start = usize::try_from(infallible_u64(record, 32)).ok()?;
        let row_count = usize::try_from(infallible_u64(record, 40)).ok()?;
        let byte_start = self
            .row_pool_offset
            .checked_add(row_start.checked_mul(32)?)?;
        let byte_length = row_count.checked_mul(32)?;
        let rows = self.bytes.get(byte_start..byte_start + byte_length)?;
        Some(TargetSetView { record, rows })
    }
}

impl<'a> RelationsView<'a> {
    fn relation_at(self, index: usize) -> Option<RelationView<'a>> {
        let start = TARGET_HEADER_LENGTH.checked_add(index.checked_mul(RELATION_RECORD_LENGTH)?)?;
        let record = self.bytes.get(start..start + RELATION_RECORD_LENGTH)?;
        let role = (infallible_u32(record, 68) & ROLE_BYTES_PRESENT != 0)
            .then(|| {
                borrowed_span(
                    self.bytes,
                    infallible_u64(record, 104),
                    infallible_u64(record, 112),
                    "relation role",
                )
                .ok()
            })
            .flatten();
        Some(RelationView { record, role })
    }
}

impl<'a> TokenSpansView<'a> {
    fn span_at(self, index: usize) -> Option<TokenSpanView<'a>> {
        let start =
            TARGET_HEADER_LENGTH.checked_add(index.checked_mul(TOKEN_SPAN_RECORD_LENGTH)?)?;
        Some(TokenSpanView {
            record: self.bytes.get(start..start + TOKEN_SPAN_RECORD_LENGTH)?,
        })
    }
}

impl<'a> MatricesView<'a> {
    fn matrix_at(
        self,
        index: usize,
        directory: DirectoryView<'a>,
        fully_verified: bool,
    ) -> Option<MatrixView<'a>> {
        let start = MATRIX_HEADER_LENGTH.checked_add(index.checked_mul(MATRIX_RECORD_LENGTH)?)?;
        let record = self.bytes.get(start..start + MATRIX_RECORD_LENGTH)?;
        let instance = infallible_u32(record, 128);
        let data = directory
            .find(SectionKey::new(SECTION_MATRIX_DATA, instance))?
            .bytes;
        Some(MatrixView {
            record,
            data,
            fully_verified,
        })
    }

    fn projection_at(self, index: usize) -> Option<ProjectionView<'a>> {
        let start = self
            .projection_offset
            .checked_add(index.checked_mul(PROJECTION_RECORD_LENGTH)?)?;
        Some(ProjectionView {
            record: self.bytes.get(start..start + PROJECTION_RECORD_LENGTH)?,
        })
    }
}

impl<'a> ExternalBindingsView<'a> {
    fn binding_at(self, index: usize) -> Option<ExternalBindingView<'a>> {
        let start = TARGET_HEADER_LENGTH.checked_add(index.checked_mul(EXTERNAL_RECORD_LENGTH)?)?;
        let record = self.bytes.get(start..start + EXTERNAL_RECORD_LENGTH)?;
        let contract = borrowed_span(
            self.bytes,
            infallible_u64(record, 176),
            infallible_u64(record, 184),
            "external binding contract",
        )
        .ok()?;
        Some(ExternalBindingView { record, contract })
    }
}

impl<'a> IndexGuardsView<'a> {
    fn guard_at(self, index: usize, directory: DirectoryView<'a>) -> Option<IndexGuardView<'a>> {
        let start = TARGET_HEADER_LENGTH.checked_add(index.checked_mul(INDEX_RECORD_LENGTH)?)?;
        let record = self.bytes.get(start..start + INDEX_RECORD_LENGTH)?;
        let guard = borrowed_span(
            self.bytes,
            infallible_u64(record, 296),
            infallible_u64(record, 304),
            "index guard contract",
        )
        .ok()?;
        let payload = match infallible_u32(record, 316) {
            1 => Some(
                directory
                    .find(SectionKey::new(
                        SECTION_INDEX_PAYLOAD,
                        infallible_u32(record, 312),
                    ))?
                    .bytes,
            ),
            2 => None,
            _ => return None,
        };
        Some(IndexGuardView {
            record,
            guard,
            payload,
        })
    }
}

fn open_framing(bytes: &[u8]) -> Result<(DirectoryView<'_>, ArtifactRoot), EmbeddingError> {
    if bytes.len() < PURREMB_HEADER_LENGTH as usize {
        return Err(EmbeddingError::Truncated);
    }
    if bytes.get(..8) != Some(PURREMB_MAGIC.as_slice()) {
        return Err(EmbeddingError::BadMagic);
    }
    let version = read_u32(bytes, 8)?;
    if version != PURREMB_VERSION {
        return Err(EmbeddingError::UnsupportedVersion(version));
    }
    require_u32(bytes, 12, PURREMB_HEADER_LENGTH, "header length")?;
    require_u32(bytes, 16, 0, "header flags")?;
    let section_count = read_u32(bytes, 20)?;
    if !(10..=PURREMB_MAX_SECTION_COUNT).contains(&section_count) {
        return Err(EmbeddingError::CountLimit {
            field: "section",
            value: u64::from(section_count),
        });
    }
    require_u64(
        bytes,
        24,
        u64::from(PURREMB_HEADER_LENGTH),
        "directory offset",
    )?;
    let directory_length = u64::from(section_count)
        .checked_mul(PURREMB_DIRECTORY_ENTRY_LENGTH)
        .ok_or(EmbeddingError::ArithmeticOverflow("directory length"))?;
    require_u64(bytes, 32, directory_length, "directory length")?;
    let first_section_offset = checked_align_up(
        u64::from(PURREMB_HEADER_LENGTH)
            .checked_add(directory_length)
            .ok_or(EmbeddingError::ArithmeticOverflow("directory end"))?,
        PURREMB_FILE_ALIGNMENT,
    )?;
    require_u64(bytes, 40, first_section_offset, "first section offset")?;
    let trailer_offset = read_u64(bytes, 48)?;
    let file_length = read_u64(bytes, 56)?;
    let expected_file_length = trailer_offset
        .checked_add(u64::from(PURREMB_TRAILER_LENGTH))
        .ok_or(EmbeddingError::ArithmeticOverflow("file length"))?;
    if file_length != expected_file_length
        || usize::try_from(file_length).ok() != Some(bytes.len())
        || file_length % PURREMB_FILE_ALIGNMENT != 0
    {
        return Err(EmbeddingError::InvalidSpan {
            context: "PURREMB file",
            offset: 0,
            length: file_length,
        });
    }
    let directory_start = PURREMB_HEADER_LENGTH as usize;
    let directory_end = usize::try_from(
        u64::from(PURREMB_HEADER_LENGTH)
            .checked_add(directory_length)
            .ok_or(EmbeddingError::ArithmeticOverflow("directory end"))?,
    )
    .map_err(|_| EmbeddingError::ArithmeticOverflow("directory end host conversion"))?;
    let entries = bytes
        .get(directory_start..directory_end)
        .ok_or(EmbeddingError::Truncated)?;
    validate_zero_padding(
        bytes,
        u64::try_from(directory_end).unwrap_or(u64::MAX),
        first_section_offset,
    )?;

    let directory = DirectoryView {
        file: bytes,
        entries,
    };
    validate_directory(directory, first_section_offset, trailer_offset)?;
    validate_trailer(bytes, trailer_offset, file_length)?;
    let root = ArtifactRoot::from_raw(array32(bytes, 64));
    if array32(
        bytes,
        usize::try_from(trailer_offset)
            .map_err(|_| EmbeddingError::ArithmeticOverflow("trailer offset host conversion"))?
            + 24,
    ) != *root.as_bytes()
    {
        return Err(EmbeddingError::Malformed("header and trailer roots differ"));
    }
    Ok((directory, root))
}

fn validate_directory(
    directory: DirectoryView<'_>,
    first_section_offset: u64,
    trailer_offset: u64,
) -> Result<(), EmbeddingError> {
    let count = directory.entries.len() / 64;
    let mut previous_key = None;
    let mut expected_offset = first_section_offset;
    let mut singleton_seen = [false; 9];
    let mut next_matrix = 1u32;
    let mut next_payload = 1u32;
    for index in 0..count {
        let start = index * 64;
        let entry = &directory.entries[start..start + 64];
        let key = SectionKey::new(infallible_u32(entry, 0), infallible_u32(entry, 8));
        let flags = infallible_u32(entry, 4);
        if infallible_u32(entry, 12) != 0 {
            return Err(EmbeddingError::ReservedNonzero("directory entry"));
        }
        if previous_key.is_some_and(|previous| key <= previous) {
            return Err(EmbeddingError::NonCanonicalOrder("section directory"));
        }
        previous_key = Some(key);
        let offset = infallible_u64(entry, 16);
        let length = infallible_u64(entry, 24);
        if length == 0
            || offset != expected_offset
            || !offset.is_multiple_of(PURREMB_FILE_ALIGNMENT)
        {
            return Err(EmbeddingError::InvalidSpan {
                context: "canonical section",
                offset,
                length,
            });
        }
        borrowed_span(directory.file, offset, length, "section")?;
        expected_offset = checked_align_up(
            offset
                .checked_add(length)
                .ok_or(EmbeddingError::ArithmeticOverflow("section end"))?,
            PURREMB_FILE_ALIGNMENT,
        )?;
        validate_zero_padding(directory.file, offset + length, expected_offset)?;

        match key.kind {
            SECTION_SOURCE..=SECTION_INDEX_GUARDS => {
                if key.instance != 0 {
                    return Err(EmbeddingError::Malformed("singleton section instance"));
                }
                let singleton = usize::try_from(key.kind - 1)
                    .map_err(|_| EmbeddingError::ArithmeticOverflow("singleton index"))?;
                if singleton_seen[singleton] {
                    return Err(EmbeddingError::Duplicate("singleton section"));
                }
                singleton_seen[singleton] = true;
                let expected = if key.kind == SECTION_INDEX_GUARDS {
                    SECTION_CRITICAL | SECTION_DERIVED
                } else {
                    SECTION_CRITICAL
                };
                if flags != expected {
                    return Err(EmbeddingError::ReservedNonzero("singleton section flags"));
                }
            }
            SECTION_MATRIX_DATA => {
                if flags != SECTION_CRITICAL || key.instance != next_matrix {
                    return Err(EmbeddingError::NonCanonicalOrder("matrix data sections"));
                }
                next_matrix = next_matrix
                    .checked_add(1)
                    .ok_or(EmbeddingError::ArithmeticOverflow("matrix data instances"))?;
            }
            SECTION_INDEX_PAYLOAD => {
                if flags != SECTION_CRITICAL | SECTION_DERIVED || key.instance != next_payload {
                    return Err(EmbeddingError::NonCanonicalOrder("index payload sections"));
                }
                next_payload =
                    next_payload
                        .checked_add(1)
                        .ok_or(EmbeddingError::ArithmeticOverflow(
                            "index payload instances",
                        ))?;
            }
            kind if kind >= SECTION_EXTENSION_MIN => {
                if flags & !(SECTION_CRITICAL | SECTION_DERIVED) != 0 {
                    return Err(EmbeddingError::ReservedNonzero("extension section flags"));
                }
                if flags & SECTION_CRITICAL != 0 {
                    return Err(EmbeddingError::UnsupportedCode {
                        field: "critical extension section",
                        value: kind,
                    });
                }
            }
            kind => {
                return Err(EmbeddingError::UnsupportedCode {
                    field: "section kind",
                    value: kind,
                });
            }
        }
    }
    if singleton_seen.iter().any(|seen| !seen) {
        return Err(EmbeddingError::Missing("singleton section"));
    }
    if next_matrix == 1 {
        return Err(EmbeddingError::Missing("MATRIX_DATA section"));
    }
    if expected_offset != trailer_offset {
        return Err(EmbeddingError::InvalidSpan {
            context: "trailer",
            offset: trailer_offset,
            length: u64::from(PURREMB_TRAILER_LENGTH),
        });
    }
    Ok(())
}

fn validate_trailer(bytes: &[u8], offset: u64, file_length: u64) -> Result<(), EmbeddingError> {
    let trailer = borrowed_span(bytes, offset, u64::from(PURREMB_TRAILER_LENGTH), "trailer")?;
    if trailer.get(..8) != Some(PURREMB_TRAILER_MAGIC.as_slice()) {
        return Err(EmbeddingError::BadTrailerMagic);
    }
    let version = read_u32(trailer, 8)?;
    if version != PURREMB_VERSION {
        return Err(EmbeddingError::UnsupportedVersion(version));
    }
    require_u32(trailer, 12, PURREMB_TRAILER_LENGTH, "trailer length")?;
    require_u64(trailer, 16, file_length, "trailer file length")?;
    require_zero(&trailer[56..64], "trailer reserved")
}

fn required_section(
    directory: DirectoryView<'_>,
    kind: u32,
    instance: u32,
) -> Result<SectionView<'_>, EmbeddingError> {
    directory
        .find(SectionKey::new(kind, instance))
        .ok_or(EmbeddingError::Missing("required PURREMB section"))
}

fn parse_source(section: &[u8], file: &[u8]) -> Result<SourceView, EmbeddingError> {
    if section.len() != SOURCE_LENGTH {
        return Err(EmbeddingError::InvalidSpan {
            context: "SOURCE section",
            offset: 0,
            length: u64::try_from(section.len()).unwrap_or(u64::MAX),
        });
    }
    require_u32(section, 0, 1, "SOURCE schema version")?;
    require_u32(section, 4, 1, "SOURCE flags")?;
    require_u32(section, 16, 1, "SOURCE format")?;
    require_u32(section, 20, 0, "SOURCE reserved")?;
    require_zero(&section[120..128], "SOURCE reserved")?;
    let source_exact_digest = ContentDigest::from_raw(array32(section, 24));
    if file[96..128] != source_exact_digest.as_bytes()[..] {
        return Err(EmbeddingError::Malformed(
            "SOURCE and header exact digests differ",
        ));
    }
    Ok(SourceView {
        source_length: infallible_u64(section, 8),
        source_exact_digest,
        certified_rdf_digest: array32(section, 56),
        dataset_target_id: TargetId::from_raw(array32(section, 88)),
    })
}

fn parse_contracts(bytes: &[u8]) -> Result<ContractsView<'_>, EmbeddingError> {
    require_minimum(bytes, FAMILY_HEADER_LENGTH, "CONTRACTS header")?;
    require_u32(bytes, 0, 1, "CONTRACTS schema version")?;
    require_u32(bytes, 4, 0, "CONTRACTS flags")?;
    let family_count = bounded_count(read_u64(bytes, 8)?, "family", true)?;
    require_u64(
        bytes,
        16,
        FAMILY_HEADER_LENGTH as u64,
        "family records offset",
    )?;
    require_u32(bytes, 24, FAMILY_RECORD_LENGTH as u32, "family record size")?;
    require_u32(bytes, 28, SPACE_RECORD_LENGTH as u32, "space record size")?;
    let space_count = bounded_count(read_u64(bytes, 32)?, "effective space", true)?;
    if space_count < family_count {
        return Err(EmbeddingError::Malformed("fewer spaces than families"));
    }
    let family_end = checked_table_end(FAMILY_HEADER_LENGTH, family_count, FAMILY_RECORD_LENGTH)?;
    let expected_space_offset = align8(family_end)?;
    require_u64(
        bytes,
        40,
        u64::try_from(expected_space_offset)
            .map_err(|_| EmbeddingError::ArithmeticOverflow("space offset"))?,
        "space records offset",
    )?;
    validate_zero_padding_usize(bytes, family_end, expected_space_offset)?;
    let space_end = checked_table_end(expected_space_offset, space_count, SPACE_RECORD_LENGTH)?;
    let expected_pool_offset = align8(space_end)?;
    require_u64(
        bytes,
        48,
        u64::try_from(expected_pool_offset)
            .map_err(|_| EmbeddingError::ArithmeticOverflow("contract pool offset"))?,
        "contract pool offset",
    )?;
    validate_zero_padding_usize(bytes, space_end, expected_pool_offset)?;
    let pool_length = host_usize(read_u64(bytes, 56)?, "contract pool length")?;
    let pool_end = expected_pool_offset
        .checked_add(pool_length)
        .ok_or(EmbeddingError::ArithmeticOverflow("contract pool end"))?;
    if pool_end != bytes.len() {
        return Err(EmbeddingError::InvalidSpan {
            context: "contract pool",
            offset: expected_pool_offset as u64,
            length: pool_length as u64,
        });
    }
    require_zero(&bytes[64..96], "CONTRACTS reserved")?;

    let mut previous_family = None;
    let mut next_space = 0usize;
    let mut pool_cursor = expected_pool_offset;
    for index in 0..family_count {
        let record = fixed_record(bytes, FAMILY_HEADER_LENGTH, FAMILY_RECORD_LENGTH, index)?;
        let family_id = array32(record, 0);
        if previous_family.is_some_and(|previous| family_id <= previous) {
            return Err(EmbeddingError::NonCanonicalOrder("family records"));
        }
        previous_family = Some(family_id);
        let contract_offset = host_usize(infallible_u64(record, 64), "contract offset")?;
        let contract_length = host_usize(infallible_u64(record, 72), "contract length")?;
        if contract_length == 0 || contract_offset != pool_cursor || contract_offset % 8 != 0 {
            return Err(EmbeddingError::InvalidSpan {
                context: "canonical family contract",
                offset: contract_offset as u64,
                length: contract_length as u64,
            });
        }
        let contract_end = contract_offset
            .checked_add(contract_length)
            .ok_or(EmbeddingError::ArithmeticOverflow("family contract end"))?;
        let contract = bytes
            .get(contract_offset..contract_end)
            .ok_or(EmbeddingError::Truncated)?;
        let shape = validate_family_contract(contract)?;
        if infallible_u32(record, 80) != shape.policy
            || infallible_u32(record, 84) != shape.stored_dimension
        {
            return Err(EmbeddingError::Malformed(
                "family record dimensionality disagrees with contract",
            ));
        }
        let space_start = usize::try_from(infallible_u32(record, 88))
            .map_err(|_| EmbeddingError::ArithmeticOverflow("family space start"))?;
        let family_space_count = usize::try_from(infallible_u32(record, 92))
            .map_err(|_| EmbeddingError::ArithmeticOverflow("family space count"))?;
        if space_start != next_space || family_space_count != shape.prefix_count {
            return Err(EmbeddingError::Malformed("family space range"));
        }
        for ordinal in 0..family_space_count {
            let global = space_start
                .checked_add(ordinal)
                .ok_or(EmbeddingError::ArithmeticOverflow("space index"))?;
            let space = fixed_record(bytes, expected_space_offset, SPACE_RECORD_LENGTH, global)?;
            if array32(space, 32) != family_id
                || infallible_u32(space, 72) != u32::try_from(ordinal).unwrap_or(u32::MAX)
                || infallible_u32(space, 76) != 0
            {
                return Err(EmbeddingError::Malformed("effective space record"));
            }
            let (dimension, postprocessing) = contract_prefix(contract, ordinal)?;
            if infallible_u32(space, 64) != dimension || infallible_u32(space, 68) != postprocessing
            {
                return Err(EmbeddingError::Malformed(
                    "effective space disagrees with contract",
                ));
            }
        }
        next_space = next_space
            .checked_add(family_space_count)
            .ok_or(EmbeddingError::ArithmeticOverflow("space coverage"))?;
        pool_cursor = if index + 1 == family_count {
            contract_end
        } else {
            let next = align8(contract_end)?;
            validate_zero_padding_usize(bytes, contract_end, next)?;
            next
        };
    }
    if next_space != space_count || pool_cursor != pool_end {
        return Err(EmbeddingError::Malformed("contract table coverage"));
    }
    Ok(ContractsView {
        bytes,
        family_count,
        space_count,
    })
}

fn parse_targets(bytes: &[u8]) -> Result<TargetsView<'_>, EmbeddingError> {
    require_minimum(bytes, TARGET_HEADER_LENGTH, "TARGETS header")?;
    require_u32(bytes, 0, 1, "TARGETS schema version")?;
    require_u32(bytes, 4, TARGET_RECORD_LENGTH as u32, "target record size")?;
    let count = bounded_count(read_u64(bytes, 8)?, "target", true)?;
    require_u64(
        bytes,
        16,
        TARGET_HEADER_LENGTH as u64,
        "target records offset",
    )?;
    let records_length = checked_product_u64(count, TARGET_RECORD_LENGTH, "target records")?;
    require_u64(bytes, 24, records_length, "target records length")?;
    let records_end = checked_table_end(TARGET_HEADER_LENGTH, count, TARGET_RECORD_LENGTH)?;
    let pool_offset = align8(records_end)?;
    require_u64(bytes, 32, pool_offset as u64, "target identity pool offset")?;
    let pool_length = host_usize(read_u64(bytes, 40)?, "target identity pool length")?;
    if pool_offset.checked_add(pool_length) != Some(bytes.len()) {
        return Err(EmbeddingError::Malformed("target identity pool extent"));
    }
    require_zero(&bytes[48..64], "TARGETS reserved")?;
    validate_zero_padding_usize(bytes, records_end, pool_offset)?;
    let mut previous_id = None;
    let mut pool_cursor = pool_offset;
    let mut retained_seen = 0usize;
    let retained_count = (0..count)
        .filter(|&index| {
            fixed_record(bytes, TARGET_HEADER_LENGTH, TARGET_RECORD_LENGTH, index)
                .is_ok_and(|record| infallible_u32(record, 68) & IDENTITY_BYTES_PRESENT != 0)
        })
        .count();
    for index in 0..count {
        let record = fixed_record(bytes, TARGET_HEADER_LENGTH, TARGET_RECORD_LENGTH, index)?;
        let id = array32(record, 0);
        if previous_id.is_some_and(|previous| id <= previous) {
            return Err(EmbeddingError::NonCanonicalOrder("target records"));
        }
        previous_id = Some(id);
        let kind = TargetKind::try_from(infallible_u32(record, 64))?;
        let flags = infallible_u32(record, 68);
        if flags & !(IDENTITY_BYTES_PRESENT | SOURCE_ORDINAL_PRESENT) != 0 {
            return Err(EmbeddingError::ReservedNonzero("target flags"));
        }
        let identity_offset = host_usize(infallible_u64(record, 72), "target identity offset")?;
        let identity_length = host_usize(infallible_u64(record, 80), "target identity length")?;
        if flags & IDENTITY_BYTES_PRESENT == 0 {
            if identity_offset != 0 || identity_length != 0 {
                return Err(EmbeddingError::Malformed("absent target identity span"));
            }
        } else {
            if identity_length == 0 || identity_offset != pool_cursor || identity_offset % 8 != 0 {
                return Err(EmbeddingError::InvalidSpan {
                    context: "canonical target identity",
                    offset: identity_offset as u64,
                    length: identity_length as u64,
                });
            }
            let end = identity_offset
                .checked_add(identity_length)
                .ok_or(EmbeddingError::ArithmeticOverflow("target identity end"))?;
            let identity = bytes
                .get(identity_offset..end)
                .ok_or(EmbeddingError::Truncated)?;
            validate_target_identity(kind, identity)?;
            retained_seen += 1;
            pool_cursor = if retained_seen == retained_count {
                end
            } else {
                let next = align8(end)?;
                validate_zero_padding_usize(bytes, end, next)?;
                next
            };
        }
        validate_target_ordinal(kind, flags, infallible_u64(record, 88))?;
    }
    if pool_cursor != bytes.len() {
        return Err(EmbeddingError::Malformed("target identity pool coverage"));
    }
    Ok(TargetsView { bytes, count })
}

fn parse_target_sets(bytes: &[u8]) -> Result<TargetSetsView<'_>, EmbeddingError> {
    require_minimum(bytes, TARGET_HEADER_LENGTH, "TARGET_SETS header")?;
    require_u32(bytes, 0, 1, "TARGET_SETS schema version")?;
    require_u32(
        bytes,
        4,
        TARGET_SET_RECORD_LENGTH as u32,
        "target-set record size",
    )?;
    let count = bounded_count(read_u64(bytes, 8)?, "target set", true)?;
    require_u64(
        bytes,
        16,
        TARGET_HEADER_LENGTH as u64,
        "target-set records offset",
    )?;
    let records_length =
        checked_product_u64(count, TARGET_SET_RECORD_LENGTH, "target-set records")?;
    require_u64(bytes, 24, records_length, "target-set records length")?;
    let row_count = host_usize(read_u64(bytes, 32)?, "target-set row reference count")?;
    let records_end = checked_table_end(TARGET_HEADER_LENGTH, count, TARGET_SET_RECORD_LENGTH)?;
    let row_pool_offset = align8(records_end)?;
    require_u64(bytes, 40, row_pool_offset as u64, "target-set rows offset")?;
    let row_bytes = row_count
        .checked_mul(32)
        .ok_or(EmbeddingError::ArithmeticOverflow("target-set row bytes"))?;
    require_u64(bytes, 48, row_bytes as u64, "target-set rows length")?;
    if row_pool_offset.checked_add(row_bytes) != Some(bytes.len()) {
        return Err(EmbeddingError::Malformed("target-set row table extent"));
    }
    require_zero(&bytes[56..64], "TARGET_SETS reserved")?;
    validate_zero_padding_usize(bytes, records_end, row_pool_offset)?;
    let mut previous = None;
    let mut next_row = 0usize;
    for index in 0..count {
        let record = fixed_record(bytes, TARGET_HEADER_LENGTH, TARGET_SET_RECORD_LENGTH, index)?;
        let id = array32(record, 0);
        if previous.is_some_and(|value| id <= value) {
            return Err(EmbeddingError::NonCanonicalOrder("target-set records"));
        }
        previous = Some(id);
        let start = host_usize(infallible_u64(record, 32), "target-set row start")?;
        let rows = host_usize(infallible_u64(record, 40), "target-set row count")?;
        if start != next_row || rows == 0 {
            return Err(EmbeddingError::Malformed("target-set row range"));
        }
        require_zero(&record[48..64], "target-set record reserved")?;
        let byte_start = row_pool_offset
            .checked_add(
                start
                    .checked_mul(32)
                    .ok_or(EmbeddingError::ArithmeticOverflow("target-set row offset"))?,
            )
            .ok_or(EmbeddingError::ArithmeticOverflow("target-set row offset"))?;
        let byte_end = byte_start
            .checked_add(
                rows.checked_mul(32)
                    .ok_or(EmbeddingError::ArithmeticOverflow("target-set row length"))?,
            )
            .ok_or(EmbeddingError::ArithmeticOverflow("target-set row end"))?;
        let row_ids = bytes
            .get(byte_start..byte_end)
            .ok_or(EmbeddingError::Truncated)?;
        validate_sorted_ids(row_ids, "target-set targets")?;
        next_row = next_row
            .checked_add(rows)
            .ok_or(EmbeddingError::ArithmeticOverflow(
                "target-set row coverage",
            ))?;
    }
    if next_row != row_count {
        return Err(EmbeddingError::Malformed("target-set row coverage"));
    }
    Ok(TargetSetsView {
        bytes,
        count,
        row_pool_offset,
    })
}

fn parse_relations(bytes: &[u8]) -> Result<RelationsView<'_>, EmbeddingError> {
    require_minimum(bytes, TARGET_HEADER_LENGTH, "RELATIONS header")?;
    require_u32(bytes, 0, 1, "RELATIONS schema version")?;
    require_u32(
        bytes,
        4,
        RELATION_RECORD_LENGTH as u32,
        "relation record size",
    )?;
    let count = bounded_count(read_u64(bytes, 8)?, "relation", false)?;
    require_u64(
        bytes,
        16,
        TARGET_HEADER_LENGTH as u64,
        "relation records offset",
    )?;
    let records_length = checked_product_u64(count, RELATION_RECORD_LENGTH, "relation records")?;
    require_u64(bytes, 24, records_length, "relation records length")?;
    let records_end = checked_table_end(TARGET_HEADER_LENGTH, count, RELATION_RECORD_LENGTH)?;
    let pool_offset = align8(records_end)?;
    require_u64(bytes, 32, pool_offset as u64, "relation role pool offset")?;
    let pool_length = host_usize(read_u64(bytes, 40)?, "relation role pool length")?;
    if pool_offset.checked_add(pool_length) != Some(bytes.len()) {
        return Err(EmbeddingError::Malformed("relation role pool extent"));
    }
    require_zero(&bytes[48..64], "RELATIONS reserved")?;
    validate_zero_padding_usize(bytes, records_end, pool_offset)?;
    let extension_count = (0..count)
        .filter(|&index| {
            fixed_record(bytes, TARGET_HEADER_LENGTH, RELATION_RECORD_LENGTH, index)
                .is_ok_and(|record| infallible_u32(record, 64) == 0x8000_0000)
        })
        .count();
    let mut extension_seen = 0usize;
    let mut cursor = pool_offset;
    let mut previous_key = None;
    for index in 0..count {
        let record = fixed_record(bytes, TARGET_HEADER_LENGTH, RELATION_RECORD_LENGTH, index)?;
        let kind = infallible_u32(record, 64);
        validate_relation_kind(kind)?;
        let key = (
            array32(record, 0),
            kind,
            array32(record, 32),
            array32(record, 72),
        );
        if previous_key.is_some_and(|previous| key <= previous) {
            return Err(EmbeddingError::NonCanonicalOrder("relation records"));
        }
        previous_key = Some(key);
        let flags = infallible_u32(record, 68);
        if flags & !ROLE_BYTES_PRESENT != 0 {
            return Err(EmbeddingError::ReservedNonzero("relation flags"));
        }
        let offset = host_usize(infallible_u64(record, 104), "relation role offset")?;
        let length = host_usize(infallible_u64(record, 112), "relation role length")?;
        if kind == 0x8000_0000 {
            if flags != ROLE_BYTES_PRESENT || length == 0 || offset != cursor || offset % 8 != 0 {
                return Err(EmbeddingError::Malformed("extension relation role"));
            }
            let end = offset
                .checked_add(length)
                .ok_or(EmbeddingError::ArithmeticOverflow("relation role end"))?;
            let role = bytes.get(offset..end).ok_or(EmbeddingError::Truncated)?;
            core::str::from_utf8(role)
                .map_err(|_| EmbeddingError::InvalidUtf8("extension relation role"))?;
            extension_seen += 1;
            cursor = if extension_seen == extension_count {
                end
            } else {
                let next = align8(end)?;
                validate_zero_padding_usize(bytes, end, next)?;
                next
            };
        } else if flags != 0 || offset != 0 || length != 0 || array32(record, 72) != [0; 32] {
            return Err(EmbeddingError::Malformed("built-in relation role"));
        }
    }
    if cursor != bytes.len() {
        return Err(EmbeddingError::Malformed("relation role pool coverage"));
    }
    Ok(RelationsView { bytes, count })
}

fn parse_token_spans(bytes: &[u8]) -> Result<TokenSpansView<'_>, EmbeddingError> {
    require_minimum(bytes, TARGET_HEADER_LENGTH, "TOKEN_SPANS header")?;
    require_u32(bytes, 0, 1, "TOKEN_SPANS schema version")?;
    require_u32(
        bytes,
        4,
        TOKEN_SPAN_RECORD_LENGTH as u32,
        "token-span record size",
    )?;
    let count = bounded_count(read_u64(bytes, 8)?, "token span", false)?;
    require_u64(
        bytes,
        16,
        TARGET_HEADER_LENGTH as u64,
        "token-span records offset",
    )?;
    let records_length =
        checked_product_u64(count, TOKEN_SPAN_RECORD_LENGTH, "token-span records")?;
    require_u64(bytes, 24, records_length, "token-span records length")?;
    require_zero(&bytes[32..64], "TOKEN_SPANS reserved")?;
    if checked_table_end(TARGET_HEADER_LENGTH, count, TOKEN_SPAN_RECORD_LENGTH)? != bytes.len() {
        return Err(EmbeddingError::Malformed("token-span section extent"));
    }
    let mut previous = None;
    for index in 0..count {
        let record = fixed_record(bytes, TARGET_HEADER_LENGTH, TOKEN_SPAN_RECORD_LENGTH, index)?;
        let key = (array32(record, 0), array32(record, 32));
        if previous.is_some_and(|value| key <= value) {
            return Err(EmbeddingError::NonCanonicalOrder("token-span records"));
        }
        previous = Some(key);
        let start = infallible_u64(record, 64);
        let end = infallible_u64(record, 72);
        if start > end || infallible_u64(record, 80) == 0 {
            return Err(EmbeddingError::InvalidSpan {
                context: "token span",
                offset: start,
                length: end.saturating_sub(start),
            });
        }
        if infallible_u32(record, 88) & !0b111 != 0 {
            return Err(EmbeddingError::ReservedNonzero("token-span flags"));
        }
        require_u32(record, 92, 0, "token-span reserved")?;
    }
    Ok(TokenSpansView { bytes, count })
}

fn parse_matrices(bytes: &[u8]) -> Result<MatricesView<'_>, EmbeddingError> {
    require_minimum(bytes, MATRIX_HEADER_LENGTH, "MATRICES header")?;
    require_u32(bytes, 0, 1, "MATRICES schema version")?;
    require_u32(bytes, 4, MATRIX_RECORD_LENGTH as u32, "matrix record size")?;
    let matrix_count = bounded_count(read_u64(bytes, 8)?, "matrix", true)?;
    require_u64(
        bytes,
        16,
        MATRIX_HEADER_LENGTH as u64,
        "matrix records offset",
    )?;
    let matrix_length = checked_product_u64(matrix_count, MATRIX_RECORD_LENGTH, "matrix records")?;
    require_u64(bytes, 24, matrix_length, "matrix records length")?;
    require_u32(
        bytes,
        32,
        PROJECTION_RECORD_LENGTH as u32,
        "projection record size",
    )?;
    require_u32(bytes, 36, 0, "MATRICES flags")?;
    let projection_count = bounded_count(read_u64(bytes, 40)?, "projection", true)?;
    let matrix_end = checked_table_end(MATRIX_HEADER_LENGTH, matrix_count, MATRIX_RECORD_LENGTH)?;
    let projection_offset = align8(matrix_end)?;
    require_u64(
        bytes,
        48,
        projection_offset as u64,
        "projection records offset",
    )?;
    let projection_length = checked_product_u64(
        projection_count,
        PROJECTION_RECORD_LENGTH,
        "projection records",
    )?;
    require_u64(bytes, 56, projection_length, "projection records length")?;
    require_zero(&bytes[64..96], "MATRICES reserved")?;
    validate_zero_padding_usize(bytes, matrix_end, projection_offset)?;
    if checked_table_end(
        projection_offset,
        projection_count,
        PROJECTION_RECORD_LENGTH,
    )? != bytes.len()
    {
        return Err(EmbeddingError::Malformed("projection table extent"));
    }
    let mut previous = None;
    for index in 0..matrix_count {
        let record = fixed_record(bytes, MATRIX_HEADER_LENGTH, MATRIX_RECORD_LENGTH, index)?;
        let id = array32(record, 0);
        if previous.is_some_and(|value| id <= value) {
            return Err(EmbeddingError::NonCanonicalOrder("matrix records"));
        }
        previous = Some(id);
        require_u32(
            record,
            128,
            u32::try_from(index + 1)
                .map_err(|_| EmbeddingError::ArithmeticOverflow("matrix data section instance"))?,
            "matrix data section instance",
        )?;
        let dtype = VectorDtype::try_from(infallible_u32(record, 132))?;
        let rows = infallible_u64(record, 136);
        let dimension = infallible_u32(record, 144);
        if rows == 0 || dimension == 0 {
            return Err(EmbeddingError::Malformed("zero matrix shape"));
        }
        require_u32(record, 148, 0, "matrix reserved")?;
        let expected_length = rows
            .checked_mul(u64::from(dimension))
            .and_then(|value| value.checked_mul(u64::from(dtype.width())))
            .ok_or(EmbeddingError::ArithmeticOverflow("matrix byte length"))?;
        require_u64(record, 152, expected_length, "matrix data length")?;
        for previous_index in 0..index {
            let earlier = fixed_record(
                bytes,
                MATRIX_HEADER_LENGTH,
                MATRIX_RECORD_LENGTH,
                previous_index,
            )?;
            if earlier[64..128] == record[64..128] {
                return Err(EmbeddingError::Duplicate("family/target-set matrix"));
            }
        }
    }
    Ok(MatricesView {
        bytes,
        matrix_count,
        projection_count,
        projection_offset,
    })
}

fn parse_external_bindings(bytes: &[u8]) -> Result<ExternalBindingsView<'_>, EmbeddingError> {
    require_minimum(bytes, TARGET_HEADER_LENGTH, "EXTERNAL_BINDINGS header")?;
    require_u32(bytes, 0, 1, "EXTERNAL_BINDINGS schema version")?;
    require_u32(
        bytes,
        4,
        EXTERNAL_RECORD_LENGTH as u32,
        "external binding record size",
    )?;
    let count = bounded_count(read_u64(bytes, 8)?, "external binding", false)?;
    require_u64(
        bytes,
        16,
        TARGET_HEADER_LENGTH as u64,
        "external binding records offset",
    )?;
    let records_length =
        checked_product_u64(count, EXTERNAL_RECORD_LENGTH, "external binding records")?;
    require_u64(bytes, 24, records_length, "external binding records length")?;
    let records_end = checked_table_end(TARGET_HEADER_LENGTH, count, EXTERNAL_RECORD_LENGTH)?;
    let pool_offset = align8(records_end)?;
    require_u64(
        bytes,
        32,
        pool_offset as u64,
        "binding contract pool offset",
    )?;
    let pool_length = host_usize(read_u64(bytes, 40)?, "binding contract pool length")?;
    if pool_offset.checked_add(pool_length) != Some(bytes.len()) {
        return Err(EmbeddingError::Malformed("binding contract pool extent"));
    }
    require_zero(&bytes[48..64], "EXTERNAL_BINDINGS reserved")?;
    validate_zero_padding_usize(bytes, records_end, pool_offset)?;
    let mut previous = None;
    let mut cursor = pool_offset;
    for index in 0..count {
        let record = fixed_record(bytes, TARGET_HEADER_LENGTH, EXTERNAL_RECORD_LENGTH, index)?;
        let id = array32(record, 0);
        if previous.is_some_and(|value| id <= value) {
            return Err(EmbeddingError::NonCanonicalOrder(
                "external binding records",
            ));
        }
        previous = Some(id);
        ExternalScopeKind::try_from(infallible_u32(record, 32))?;
        let flags = infallible_u32(record, 36);
        if flags & !CERTIFIED_RDF_PRESENT != 0 {
            return Err(EmbeddingError::ReservedNonzero("external binding flags"));
        }
        let certified = array32(record, 112);
        if (flags & CERTIFIED_RDF_PRESENT == 0) != (certified == [0; 32]) {
            return Err(EmbeddingError::Malformed("external certified RDF presence"));
        }
        let offset = host_usize(infallible_u64(record, 176), "binding contract offset")?;
        let length = host_usize(infallible_u64(record, 184), "binding contract length")?;
        if length == 0 || offset != cursor || offset % 8 != 0 {
            return Err(EmbeddingError::InvalidSpan {
                context: "binding contract",
                offset: offset as u64,
                length: length as u64,
            });
        }
        let end = offset
            .checked_add(length)
            .ok_or(EmbeddingError::ArithmeticOverflow("binding contract end"))?;
        validate_binding_contract(bytes.get(offset..end).ok_or(EmbeddingError::Truncated)?)?;
        cursor = if index + 1 == count {
            end
        } else {
            let next = align8(end)?;
            validate_zero_padding_usize(bytes, end, next)?;
            next
        };
    }
    if cursor != bytes.len() {
        return Err(EmbeddingError::Malformed("binding contract pool coverage"));
    }
    Ok(ExternalBindingsView { bytes, count })
}

fn parse_index_guards(bytes: &[u8]) -> Result<IndexGuardsView<'_>, EmbeddingError> {
    require_minimum(bytes, TARGET_HEADER_LENGTH, "INDEX_GUARDS header")?;
    require_u32(bytes, 0, 1, "INDEX_GUARDS schema version")?;
    require_u32(bytes, 4, INDEX_RECORD_LENGTH as u32, "index record size")?;
    let count = bounded_count(read_u64(bytes, 8)?, "index guard", false)?;
    require_u64(
        bytes,
        16,
        TARGET_HEADER_LENGTH as u64,
        "index records offset",
    )?;
    let records_length = checked_product_u64(count, INDEX_RECORD_LENGTH, "index records")?;
    require_u64(bytes, 24, records_length, "index records length")?;
    let records_end = checked_table_end(TARGET_HEADER_LENGTH, count, INDEX_RECORD_LENGTH)?;
    let pool_offset = align8(records_end)?;
    require_u64(bytes, 32, pool_offset as u64, "index guard pool offset")?;
    let pool_length = host_usize(read_u64(bytes, 40)?, "index guard pool length")?;
    if pool_offset.checked_add(pool_length) != Some(bytes.len()) {
        return Err(EmbeddingError::Malformed("index guard pool extent"));
    }
    require_zero(&bytes[48..64], "INDEX_GUARDS reserved")?;
    validate_zero_padding_usize(bytes, records_end, pool_offset)?;
    let mut previous = None;
    let mut cursor = pool_offset;
    let mut inline_instance = 1u32;
    for index in 0..count {
        let record = fixed_record(bytes, TARGET_HEADER_LENGTH, INDEX_RECORD_LENGTH, index)?;
        let id = array32(record, 0);
        if previous.is_some_and(|value| id <= value) {
            return Err(EmbeddingError::NonCanonicalOrder("index records"));
        }
        previous = Some(id);
        let storage = IndexStorage::try_from(infallible_u32(record, 316))?;
        let determinism = IndexDeterminism::try_from(infallible_u32(record, 320))?;
        if infallible_u32(record, 324) != INDEX_REBUILDABLE {
            return Err(EmbeddingError::ReservedNonzero("index flags"));
        }
        if infallible_u32(record, 328) == 0 || infallible_u32(record, 332) != 0 {
            return Err(EmbeddingError::Malformed(
                "index dimension or reserved field",
            ));
        }
        let payload_length = infallible_u64(record, 288);
        match storage {
            IndexStorage::Inline => {
                if determinism != IndexDeterminism::Deterministic
                    || payload_length == 0
                    || infallible_u32(record, 312) != inline_instance
                {
                    return Err(EmbeddingError::Malformed("inline index storage"));
                }
                inline_instance = inline_instance
                    .checked_add(1)
                    .ok_or(EmbeddingError::ArithmeticOverflow("inline index instances"))?;
            }
            IndexStorage::Detached => {
                if infallible_u32(record, 312) != 0 {
                    return Err(EmbeddingError::Malformed("detached index section instance"));
                }
            }
        }
        let offset = host_usize(infallible_u64(record, 296), "index guard offset")?;
        let length = host_usize(infallible_u64(record, 304), "index guard length")?;
        if length == 0 || offset != cursor || offset % 8 != 0 {
            return Err(EmbeddingError::InvalidSpan {
                context: "index guard contract",
                offset: offset as u64,
                length: length as u64,
            });
        }
        let end = offset
            .checked_add(length)
            .ok_or(EmbeddingError::ArithmeticOverflow("index guard end"))?;
        validate_index_guard(bytes.get(offset..end).ok_or(EmbeddingError::Truncated)?)?;
        cursor = if index + 1 == count {
            end
        } else {
            let next = align8(end)?;
            validate_zero_padding_usize(bytes, end, next)?;
            next
        };
    }
    if cursor != bytes.len() {
        return Err(EmbeddingError::Malformed("index guard pool coverage"));
    }
    Ok(IndexGuardsView { bytes, count })
}

impl EmbeddingView<'_> {
    fn validate_cross_references(&self) -> Result<(), EmbeddingError> {
        let dataset = self
            .target(self.source.dataset_target_id())
            .ok_or(EmbeddingError::MissingReference("SOURCE dataset target"))?;
        if dataset.kind()? != TargetKind::RdfDataset {
            return Err(EmbeddingError::Malformed(
                "SOURCE target is not an RDF dataset",
            ));
        }
        if let Some(identity) = dataset.identity_bytes()
            && required_digest(identity, 1, "dataset RDFC digest")?
                != self.source.certified_rdf_digest()
        {
            return Err(EmbeddingError::Malformed(
                "dataset target disagrees with SOURCE RDFC digest",
            ));
        }

        for target in self.targets() {
            self.validate_target_references(target)?;
        }
        for set in self.target_sets() {
            for target in set.targets() {
                if self.target(target).is_none() {
                    return Err(EmbeddingError::MissingReference("target-set target"));
                }
            }
        }
        for relation in self.relations() {
            let subject = self
                .target(relation.subject())
                .ok_or(EmbeddingError::MissingReference("relation subject"))?;
            let object = self
                .target(relation.object())
                .ok_or(EmbeddingError::MissingReference("relation object"))?;
            validate_relation_endpoints(relation.kind_code(), subject.kind()?, object.kind()?)?;
        }
        for span in self.token_spans() {
            let family = self
                .family(span.family_id())
                .ok_or(EmbeddingError::MissingReference("token-span family"))?;
            let target = self
                .target(span.target_id())
                .ok_or(EmbeddingError::MissingReference("token-span target"))?;
            if !matches!(target.kind()?, TargetKind::Document | TargetKind::Chunk) {
                return Err(EmbeddingError::Malformed("token span target kind"));
            }
            if !family.truncation_applied()? && span.flags() & 0b11 != 0 {
                return Err(EmbeddingError::Malformed(
                    "token-span truncation flags disagree with family contract",
                ));
            }
            if target.kind()? == TargetKind::Chunk {
                if !family.chunking_applied()? {
                    return Err(EmbeddingError::Malformed(
                        "chunk token span uses a family without applied chunking",
                    ));
                }
                if let Some(identity) = target.identity_bytes()
                    && required_digest(identity, 2, "chunking contract")?
                        != *family.chunking_contract_id()?.as_bytes()
                {
                    return Err(EmbeddingError::Malformed(
                        "chunk target uses a different family chunking contract",
                    ));
                }
            }
        }
        self.validate_matrix_references()?;
        self.validate_external_references()?;
        self.validate_index_references()?;
        Ok(())
    }

    fn validate_target_references(&self, target: TargetView<'_>) -> Result<(), EmbeddingError> {
        let Some(identity) = target.identity_bytes() else {
            return Ok(());
        };
        match target.kind()? {
            TargetKind::Corpus | TargetKind::RdfDataset | TargetKind::Extension => {}
            TargetKind::Document => {
                self.require_target_kind(
                    TargetId::from_raw(required_digest(identity, 1, "document corpus")?),
                    &[TargetKind::Corpus],
                    "document corpus",
                )?;
            }
            TargetKind::Chunk => {
                self.require_target_kind(
                    TargetId::from_raw(required_digest(identity, 1, "chunk document")?),
                    &[TargetKind::Document],
                    "chunk document",
                )?;
            }
            TargetKind::RdfGraph => {
                self.require_target_kind(
                    TargetId::from_raw(required_digest(identity, 1, "graph dataset")?),
                    &[TargetKind::RdfDataset],
                    "graph dataset",
                )?;
                if tlv_u32(required_tlv(identity, 2, TlvWireType::U32, "graph form")?)? == 1 {
                    self.require_target_kind(
                        TargetId::from_raw(required_digest(identity, 3, "graph name")?),
                        &[TargetKind::RdfTerm],
                        "graph name",
                    )?;
                }
            }
            TargetKind::RdfStatement => {
                self.require_target_kind(
                    TargetId::from_raw(required_digest(identity, 1, "statement graph")?),
                    &[TargetKind::RdfGraph],
                    "statement graph",
                )?;
                for tag in 2..=4 {
                    self.require_target_kind(
                        TargetId::from_raw(required_digest(identity, tag, "statement term")?),
                        &[TargetKind::RdfTerm],
                        "statement term",
                    )?;
                }
            }
            TargetKind::RdfReifier => {
                self.require_target_kind(
                    TargetId::from_raw(required_digest(identity, 1, "reifier graph")?),
                    &[TargetKind::RdfGraph],
                    "reifier graph",
                )?;
                self.require_target_kind(
                    TargetId::from_raw(required_digest(identity, 2, "reified statement")?),
                    &[TargetKind::RdfStatement],
                    "reified statement",
                )?;
                self.require_target_kind(
                    TargetId::from_raw(required_digest(identity, 3, "reifier term")?),
                    &[TargetKind::RdfTerm],
                    "reifier term",
                )?;
            }
            TargetKind::RdfAnnotation => {
                self.require_target_kind(
                    TargetId::from_raw(required_digest(identity, 1, "annotation graph")?),
                    &[TargetKind::RdfGraph],
                    "annotation graph",
                )?;
                self.require_target_kind(
                    TargetId::from_raw(required_digest(identity, 2, "annotation reifier")?),
                    &[TargetKind::RdfReifier],
                    "annotation reifier",
                )?;
                for tag in 3..=4 {
                    self.require_target_kind(
                        TargetId::from_raw(required_digest(identity, tag, "annotation term")?),
                        &[TargetKind::RdfTerm],
                        "annotation term",
                    )?;
                }
            }
            TargetKind::RdfTerm => {
                match tlv_u32(required_tlv(identity, 1, TlvWireType::U32, "term form")?)? {
                    2 => self.require_target_kind(
                        TargetId::from_raw(required_digest(identity, 2, "blank dataset")?),
                        &[TargetKind::RdfDataset],
                        "blank dataset",
                    )?,
                    4 => {
                        for tag in 2..=4 {
                            self.require_target_kind(
                                TargetId::from_raw(required_digest(identity, tag, "triple term")?),
                                &[TargetKind::RdfTerm],
                                "triple-term component",
                            )?;
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn require_target_kind(
        &self,
        id: TargetId,
        allowed: &[TargetKind],
        context: &'static str,
    ) -> Result<(), EmbeddingError> {
        let target = self
            .target(id)
            .ok_or(EmbeddingError::MissingReference(context))?;
        if !allowed.contains(&target.kind()?) {
            return Err(EmbeddingError::Malformed(context));
        }
        Ok(())
    }

    fn validate_matrix_references(&self) -> Result<(), EmbeddingError> {
        if self
            .section(
                SECTION_MATRIX_DATA,
                u32::try_from(self.matrix_count()).unwrap_or(u32::MAX) + 1,
            )
            .is_some()
        {
            return Err(EmbeddingError::Malformed("extra MATRIX_DATA section"));
        }
        let mut projection_index = 0usize;
        for matrix in self.matrices() {
            let family = self
                .family(matrix.family_id())
                .ok_or(EmbeddingError::MissingReference("matrix family"))?;
            let set = self
                .target_set(matrix.target_set_id())
                .ok_or(EmbeddingError::MissingReference("matrix target set"))?;
            if matrix.dtype()? != family.dtype()?
                || matrix.stored_dimension() != family.stored_dimension()
                || matrix.row_count() != u64::try_from(set.row_count()).unwrap_or(u64::MAX)
                || matrix.data_bytes().len()
                    != usize::try_from(infallible_u64(matrix.record, 152))
                        .map_err(|_| EmbeddingError::ArithmeticOverflow("matrix data length"))?
            {
                return Err(EmbeddingError::Malformed(
                    "matrix family or target-set shape",
                ));
            }
            for target_id in set.targets() {
                let target = self
                    .target(target_id)
                    .ok_or(EmbeddingError::MissingReference("matrix target"))?;
                if matches!(target.kind()?, TargetKind::Document | TargetKind::Chunk)
                    && self.token_span(family.id(), target_id).is_none()
                {
                    return Err(EmbeddingError::MissingReference(
                        "matrix text-subject token span",
                    ));
                }
                if target.kind()? == TargetKind::Chunk {
                    if !family.chunking_applied()? {
                        return Err(EmbeddingError::Malformed(
                            "chunk matrix uses a family without applied chunking",
                        ));
                    }
                    if let Some(identity) = target.identity_bytes()
                        && required_digest(identity, 2, "chunking contract")?
                            != *family.chunking_contract_id()?.as_bytes()
                    {
                        return Err(EmbeddingError::Malformed(
                            "chunk target uses a different family chunking contract",
                        ));
                    }
                }
            }
            let mut previous_dimension = 0;
            for space in family.spaces() {
                let projection = self
                    .projection_at(projection_index)
                    .ok_or(EmbeddingError::Missing("matrix projection"))?;
                projection_index += 1;
                if projection.matrix_id() != matrix.id()
                    || projection.vector_space_id() != space.id()
                    || projection.effective_dimension() != space.dimension()
                    || projection.postprocessing()? != space.postprocessing()?
                    || projection.row_count() != matrix.row_count()
                    || projection.effective_dimension() <= previous_dimension
                {
                    return Err(EmbeddingError::Malformed("matrix projection relationship"));
                }
                previous_dimension = projection.effective_dimension();
                let expected_length = projection
                    .row_count()
                    .checked_mul(u64::from(projection.effective_dimension()))
                    .and_then(|value| value.checked_mul(u64::from(matrix.dtype().ok()?.width())))
                    .ok_or(EmbeddingError::ArithmeticOverflow(
                        "projection logical length",
                    ))?;
                if projection.logical_byte_length() != expected_length {
                    return Err(EmbeddingError::Malformed("projection logical byte length"));
                }
            }
        }
        if projection_index != self.projection_count() {
            return Err(EmbeddingError::Malformed("projection table coverage"));
        }
        Ok(())
    }

    fn validate_external_references(&self) -> Result<(), EmbeddingError> {
        for binding in self.external_bindings() {
            let exists = match binding.scope_kind()? {
                ExternalScopeKind::ExactSource => {
                    binding.scope_id() == *self.source.source_exact_digest().as_bytes()
                }
                ExternalScopeKind::Target => self
                    .target(TargetId::from_raw(binding.scope_id()))
                    .is_some(),
                ExternalScopeKind::TargetSet => self
                    .target_set(TargetSetId::from_raw(binding.scope_id()))
                    .is_some(),
                ExternalScopeKind::Family => self
                    .family(FamilyId::from_raw(binding.scope_id()))
                    .is_some(),
                ExternalScopeKind::VectorSpace => self
                    .vector_space(VectorSpaceId::from_raw(binding.scope_id()))
                    .is_some(),
                ExternalScopeKind::Matrix => self
                    .matrix(MatrixId::from_raw(binding.scope_id()))
                    .is_some(),
                ExternalScopeKind::Projection => self
                    .projection(ProjectionId::from_raw(binding.scope_id()))
                    .is_some(),
                ExternalScopeKind::Index => self
                    .index_guard(IndexId::from_raw(binding.scope_id()))
                    .is_some(),
            };
            if !exists {
                return Err(EmbeddingError::MissingReference("external binding scope"));
            }
        }
        Ok(())
    }

    fn validate_index_references(&self) -> Result<(), EmbeddingError> {
        let mut inline_count = 0u32;
        for index in self.index_guards() {
            if index.source_exact_digest() != self.source.source_exact_digest() {
                return Err(EmbeddingError::Malformed("index source digest"));
            }
            let family = self
                .family(index.family_id())
                .ok_or(EmbeddingError::MissingReference("index family"))?;
            let space = self
                .vector_space(index.vector_space_id())
                .ok_or(EmbeddingError::MissingReference("index vector space"))?;
            let matrix = self
                .matrix(index.matrix_id())
                .ok_or(EmbeddingError::MissingReference("index matrix"))?;
            let projection = self
                .projection(index.projection_id())
                .ok_or(EmbeddingError::MissingReference("index projection"))?;
            if space.family_id() != family.id()
                || matrix.family_id() != family.id()
                || matrix.target_set_id() != index.target_set_id()
                || projection.matrix_id() != matrix.id()
                || projection.vector_space_id() != space.id()
                || projection.effective_dimension() != index.prefix_dimension()
                || index.prefix_dimension() != space.dimension()
            {
                return Err(EmbeddingError::Malformed("stale index guard relationship"));
            }
            match index.storage()? {
                IndexStorage::Inline => {
                    inline_count += 1;
                    if index.payload_bytes().map(<[u8]>::len)
                        != Some(usize::try_from(index.payload_length()).map_err(|_| {
                            EmbeddingError::ArithmeticOverflow("index payload length")
                        })?)
                    {
                        return Err(EmbeddingError::Malformed("inline index payload length"));
                    }
                }
                IndexStorage::Detached => {
                    let found = self.external_bindings().any(|binding| {
                        binding.scope_kind().ok() == Some(ExternalScopeKind::Index)
                            && binding.scope_id() == *index.id().as_bytes()
                            && binding.artifact_sha256() == index.payload_sha256()
                            && binding.artifact_length() == index.payload_length()
                    });
                    if !found {
                        return Err(EmbeddingError::MissingReference("detached index binding"));
                    }
                }
            }
        }
        if self
            .section(SECTION_INDEX_PAYLOAD, inline_count.saturating_add(1))
            .is_some()
        {
            return Err(EmbeddingError::Malformed("extra INDEX_PAYLOAD section"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct FieldRule {
    tag: u16,
    wire: TlvWireType,
    required: bool,
}

#[derive(Clone, Copy)]
struct ContractShape {
    policy: u32,
    stored_dimension: u32,
    prefix_count: usize,
}

const fn required_rule(tag: u16, wire: TlvWireType) -> FieldRule {
    FieldRule {
        tag,
        wire,
        required: true,
    }
}

const fn optional_rule(tag: u16, wire: TlvWireType) -> FieldRule {
    FieldRule {
        tag,
        wire,
        required: false,
    }
}

fn validate_schema(bytes: &[u8], rules: &[FieldRule]) -> Result<(), EmbeddingError> {
    let mut seen = 0u32;
    for entry in canonical_tlv(bytes)? {
        if let Some((index, rule)) = rules
            .iter()
            .enumerate()
            .find(|(_, rule)| rule.tag == entry.tag)
        {
            if !entry.critical || entry.wire_type != rule.wire {
                return Err(EmbeddingError::MalformedTlv(
                    "known field has wrong criticality or wire type",
                ));
            }
            seen |= 1u32
                .checked_shl(u32::try_from(index).unwrap_or(u32::MAX))
                .unwrap_or(0);
        } else if entry.tag < 0x8000 || entry.critical {
            return Err(EmbeddingError::UnsupportedCode {
                field: "critical or reserved TLV tag",
                value: u32::from(entry.tag),
            });
        }
    }
    for (index, rule) in rules.iter().enumerate() {
        if rule.required
            && seen
                & 1u32
                    .checked_shl(u32::try_from(index).unwrap_or(u32::MAX))
                    .unwrap_or(0)
                == 0
        {
            return Err(EmbeddingError::Missing("required TLV field"));
        }
    }
    Ok(())
}

fn validate_family_contract(bytes: &[u8]) -> Result<ContractShape, EmbeddingError> {
    const RULES: [FieldRule; 15] = [
        required_rule(1, TlvWireType::Block),
        required_rule(2, TlvWireType::Block),
        required_rule(3, TlvWireType::Block),
        required_rule(4, TlvWireType::Block),
        required_rule(5, TlvWireType::Block),
        required_rule(6, TlvWireType::Block),
        required_rule(7, TlvWireType::Block),
        required_rule(8, TlvWireType::Block),
        required_rule(9, TlvWireType::Block),
        required_rule(10, TlvWireType::Block),
        required_rule(11, TlvWireType::U32),
        required_rule(12, TlvWireType::U32),
        required_rule(13, TlvWireType::U32),
        required_rule(14, TlvWireType::Block),
        required_rule(15, TlvWireType::Block),
    ];
    validate_schema(bytes, &RULES)?;
    for tag in 1..=3 {
        validate_artifact_identity(
            required_tlv(bytes, tag, TlvWireType::Block, "artifact")?.value,
        )?;
    }
    for tag in 4..=10 {
        let applied = validate_stage(required_tlv(bytes, tag, TlvWireType::Block, "stage")?.value)?;
        if tag == 5 && !applied {
            return Err(EmbeddingError::Malformed(
                "subject projection is not applied",
            ));
        }
    }
    VectorDtype::try_from(tlv_u32(required_tlv(
        bytes,
        11,
        TlvWireType::U32,
        "dtype",
    )?)?)?;
    if tlv_u32(required_tlv(bytes, 12, TlvWireType::U32, "byte order")?)? != 1
        || tlv_u32(required_tlv(bytes, 13, TlvWireType::U32, "quantization")?)? != 0
    {
        return Err(EmbeddingError::Malformed("unsupported numerical contract"));
    }
    validate_metric(required_tlv(bytes, 14, TlvWireType::Block, "metric")?.value)?;
    validate_dimensionality(required_tlv(bytes, 15, TlvWireType::Block, "dimensionality")?.value)
}

fn validate_artifact_identity(bytes: &[u8]) -> Result<(), EmbeddingError> {
    const RULES: [FieldRule; 5] = [
        required_rule(1, TlvWireType::Utf8),
        required_rule(2, TlvWireType::Utf8),
        required_rule(3, TlvWireType::Digest32),
        optional_rule(4, TlvWireType::Bytes),
        required_rule(5, TlvWireType::U32),
    ];
    validate_schema(bytes, &RULES)?;
    require_nonempty_tlv(bytes, 1, TlvWireType::Utf8, "artifact identifier")?;
    require_nonempty_tlv(bytes, 2, TlvWireType::Utf8, "artifact media type")?;
    if let Some(revision) = optional_tlv(bytes, 4)?
        && revision.value.is_empty()
    {
        return Err(EmbeddingError::Missing("artifact revision bytes"));
    }
    match tlv_u32(required_tlv(bytes, 5, TlvWireType::U32, "artifact kind")?)? {
        1 | 2 => Ok(()),
        value => Err(EmbeddingError::UnsupportedCode {
            field: "artifact identity kind",
            value,
        }),
    }
}

fn validate_stage(bytes: &[u8]) -> Result<bool, EmbeddingError> {
    let state = tlv_u32(required_tlv(bytes, 1, TlvWireType::U32, "stage state")?)?;
    match state {
        0 => {
            validate_schema(bytes, &[required_rule(1, TlvWireType::U32)])?;
            Ok(false)
        }
        1 => {
            const RULES: [FieldRule; 6] = [
                required_rule(1, TlvWireType::U32),
                required_rule(2, TlvWireType::Utf8),
                required_rule(3, TlvWireType::Digest32),
                required_rule(4, TlvWireType::Utf8),
                required_rule(5, TlvWireType::Bytes),
                required_rule(6, TlvWireType::Digest32),
            ];
            validate_schema(bytes, &RULES)?;
            require_nonempty_tlv(bytes, 2, TlvWireType::Utf8, "stage identifier")?;
            require_nonempty_tlv(bytes, 4, TlvWireType::Utf8, "stage parameter encoding")?;
            Ok(true)
        }
        value => Err(EmbeddingError::UnsupportedCode {
            field: "stage state",
            value,
        }),
    }
}

fn validate_metric(bytes: &[u8]) -> Result<(), EmbeddingError> {
    let code = tlv_u32(required_tlv(bytes, 1, TlvWireType::U32, "metric code")?)?;
    if matches!(code, 1..=3) {
        return validate_schema(bytes, &[required_rule(1, TlvWireType::U32)]);
    }
    if code != 0x8000_0000 {
        return Err(EmbeddingError::UnsupportedCode {
            field: "distance metric",
            value: code,
        });
    }
    const RULES: [FieldRule; 5] = [
        required_rule(1, TlvWireType::U32),
        required_rule(2, TlvWireType::Utf8),
        required_rule(3, TlvWireType::Utf8),
        required_rule(4, TlvWireType::Bytes),
        required_rule(5, TlvWireType::Digest32),
    ];
    validate_schema(bytes, &RULES)?;
    require_nonempty_tlv(bytes, 2, TlvWireType::Utf8, "metric identifier")?;
    require_nonempty_tlv(bytes, 3, TlvWireType::Utf8, "metric parameter encoding")?;
    Ok(())
}

fn validate_dimensionality(bytes: &[u8]) -> Result<ContractShape, EmbeddingError> {
    const RULES: [FieldRule; 3] = [
        required_rule(1, TlvWireType::U32),
        required_rule(2, TlvWireType::U32),
        required_rule(3, TlvWireType::BlockList),
    ];
    validate_schema(bytes, &RULES)?;
    let policy = tlv_u32(required_tlv(
        bytes,
        1,
        TlvWireType::U32,
        "dimension policy",
    )?)?;
    let stored_dimension = tlv_u32(required_tlv(
        bytes,
        2,
        TlvWireType::U32,
        "stored dimension",
    )?)?;
    if stored_dimension == 0 || !matches!(policy, 1 | 2) {
        return Err(EmbeddingError::Malformed(
            "dimensionality policy or dimension",
        ));
    }
    let list = required_tlv(bytes, 3, TlvWireType::BlockList, "effective prefixes")?.value;
    let prefix_count = block_list_count(list)?;
    if (policy == 1 && prefix_count != 1) || (policy == 2 && prefix_count < 2) {
        return Err(EmbeddingError::Malformed("effective prefix count"));
    }
    let mut previous = 0;
    for index in 0..prefix_count {
        let block = block_list_item(list, index)?;
        const PREFIX_RULES: [FieldRule; 2] = [
            required_rule(1, TlvWireType::U32),
            required_rule(2, TlvWireType::U32),
        ];
        validate_schema(block, &PREFIX_RULES)?;
        let dimension = tlv_u32(required_tlv(
            block,
            1,
            TlvWireType::U32,
            "prefix dimension",
        )?)?;
        let post = tlv_u32(required_tlv(
            block,
            2,
            TlvWireType::U32,
            "prefix postprocessing",
        )?)?;
        PrefixPostprocessing::try_from(post)?;
        if dimension == 0 || dimension <= previous || dimension > stored_dimension {
            return Err(EmbeddingError::NonCanonicalOrder(
                "effective prefix dimensions",
            ));
        }
        previous = dimension;
    }
    if previous != stored_dimension {
        return Err(EmbeddingError::Malformed(
            "final prefix is not the stored dimension",
        ));
    }
    Ok(ContractShape {
        policy,
        stored_dimension,
        prefix_count,
    })
}

fn contract_prefix(contract: &[u8], index: usize) -> Result<(u32, u32), EmbeddingError> {
    let dimensionality = required_tlv(contract, 15, TlvWireType::Block, "dimensionality")?.value;
    let list = required_tlv(
        dimensionality,
        3,
        TlvWireType::BlockList,
        "effective prefixes",
    )?
    .value;
    let prefix = block_list_item(list, index)?;
    Ok((
        tlv_u32(required_tlv(
            prefix,
            1,
            TlvWireType::U32,
            "prefix dimension",
        )?)?,
        tlv_u32(required_tlv(
            prefix,
            2,
            TlvWireType::U32,
            "prefix postprocessing",
        )?)?,
    ))
}

fn validate_target_identity(kind: TargetKind, bytes: &[u8]) -> Result<(), EmbeddingError> {
    match kind {
        TargetKind::Corpus => {
            validate_schema(
                bytes,
                &[
                    required_rule(1, TlvWireType::Digest32),
                    required_rule(2, TlvWireType::Utf8),
                    required_rule(3, TlvWireType::Digest32),
                ],
            )?;
            require_nonempty_tlv(bytes, 2, TlvWireType::Utf8, "corpus media type")?;
        }
        TargetKind::Document => {
            validate_schema(
                bytes,
                &[
                    required_rule(1, TlvWireType::Digest32),
                    required_rule(2, TlvWireType::Digest32),
                    required_rule(3, TlvWireType::Digest32),
                    required_rule(4, TlvWireType::Utf8),
                    required_rule(5, TlvWireType::U64),
                    required_rule(6, TlvWireType::U64),
                ],
            )?;
            require_nonempty_tlv(bytes, 4, TlvWireType::Utf8, "document media type")?;
        }
        TargetKind::Chunk => {
            validate_schema(
                bytes,
                &[
                    required_rule(1, TlvWireType::Digest32),
                    required_rule(2, TlvWireType::Digest32),
                    required_rule(3, TlvWireType::Digest32),
                    required_rule(4, TlvWireType::U64),
                    required_rule(5, TlvWireType::U64),
                    required_rule(6, TlvWireType::U64),
                    required_rule(7, TlvWireType::U64),
                ],
            )?;
            let byte_start = tlv_u64(required_tlv(
                bytes,
                4,
                TlvWireType::U64,
                "chunk byte start",
            )?)?;
            let byte_end = tlv_u64(required_tlv(bytes, 5, TlvWireType::U64, "chunk byte end")?)?;
            let scalar_start = tlv_u64(required_tlv(
                bytes,
                6,
                TlvWireType::U64,
                "chunk scalar start",
            )?)?;
            let scalar_end = tlv_u64(required_tlv(
                bytes,
                7,
                TlvWireType::U64,
                "chunk scalar end",
            )?)?;
            if byte_start >= byte_end || scalar_start >= scalar_end {
                return Err(EmbeddingError::InvalidSpan {
                    context: "chunk identity",
                    offset: byte_start,
                    length: byte_end.saturating_sub(byte_start),
                });
            }
        }
        TargetKind::RdfDataset => {
            validate_schema(bytes, &[required_rule(1, TlvWireType::Digest32)])?;
        }
        TargetKind::RdfGraph => {
            let form = tlv_u32(required_tlv(bytes, 2, TlvWireType::U32, "graph form")?)?;
            let rules: &[FieldRule] = if form == 0 {
                &[
                    required_rule(1, TlvWireType::Digest32),
                    required_rule(2, TlvWireType::U32),
                ]
            } else if form == 1 {
                &[
                    required_rule(1, TlvWireType::Digest32),
                    required_rule(2, TlvWireType::U32),
                    required_rule(3, TlvWireType::Digest32),
                ]
            } else {
                return Err(EmbeddingError::UnsupportedCode {
                    field: "RDF graph form",
                    value: form,
                });
            };
            validate_schema(bytes, rules)?;
        }
        TargetKind::RdfStatement | TargetKind::RdfAnnotation => {
            validate_schema(bytes, &four_digest_rules())?;
        }
        TargetKind::RdfReifier => {
            validate_schema(
                bytes,
                &[
                    required_rule(1, TlvWireType::Digest32),
                    required_rule(2, TlvWireType::Digest32),
                    required_rule(3, TlvWireType::Digest32),
                ],
            )?;
        }
        TargetKind::RdfTerm => validate_rdf_term(bytes)?,
        TargetKind::Extension => {
            validate_schema(
                bytes,
                &[
                    required_rule(1, TlvWireType::Utf8),
                    required_rule(2, TlvWireType::Utf8),
                    required_rule(3, TlvWireType::Bytes),
                    required_rule(4, TlvWireType::Digest32),
                ],
            )?;
            require_nonempty_tlv(bytes, 1, TlvWireType::Utf8, "extension target kind")?;
            require_nonempty_tlv(bytes, 2, TlvWireType::Utf8, "extension payload encoding")?;
        }
    }
    Ok(())
}

const fn four_digest_rules() -> [FieldRule; 4] {
    [
        required_rule(1, TlvWireType::Digest32),
        required_rule(2, TlvWireType::Digest32),
        required_rule(3, TlvWireType::Digest32),
        required_rule(4, TlvWireType::Digest32),
    ]
}

fn validate_rdf_term(bytes: &[u8]) -> Result<(), EmbeddingError> {
    let form = tlv_u32(required_tlv(bytes, 1, TlvWireType::U32, "RDF term form")?)?;
    match form {
        1 => {
            validate_schema(
                bytes,
                &[
                    required_rule(1, TlvWireType::U32),
                    required_rule(2, TlvWireType::Utf8),
                ],
            )?;
            let iri = require_nonempty_tlv(bytes, 2, TlvWireType::Utf8, "RDF IRI")?;
            validate_absolute_iri(iri)?;
        }
        2 => {
            validate_schema(
                bytes,
                &[
                    required_rule(1, TlvWireType::U32),
                    required_rule(2, TlvWireType::Digest32),
                    required_rule(3, TlvWireType::Utf8),
                ],
            )?;
            let label = require_nonempty_tlv(bytes, 3, TlvWireType::Utf8, "blank label")?;
            if label.starts_with("_:") {
                return Err(EmbeddingError::Malformed("blank label includes prefix"));
            }
        }
        3 => {
            validate_schema(
                bytes,
                &[
                    required_rule(1, TlvWireType::U32),
                    required_rule(2, TlvWireType::Utf8),
                    required_rule(3, TlvWireType::Utf8),
                    optional_rule(4, TlvWireType::Utf8),
                    required_rule(5, TlvWireType::U32),
                ],
            )?;
            let datatype = require_nonempty_tlv(bytes, 3, TlvWireType::Utf8, "literal datatype")?;
            validate_absolute_iri(datatype)?;
            let language = optional_tlv(bytes, 4)?;
            if let Some(language) = language {
                validate_language(
                    core::str::from_utf8(language.value)
                        .map_err(|_| EmbeddingError::InvalidUtf8("language tag"))?,
                )?;
            }
            let direction = tlv_u32(required_tlv(
                bytes,
                5,
                TlvWireType::U32,
                "literal direction",
            )?)?;
            if direction > 2 || (direction != 0 && language.is_none()) {
                return Err(EmbeddingError::Malformed("literal direction"));
            }
        }
        4 => validate_schema(
            bytes,
            &[
                required_rule(1, TlvWireType::U32),
                required_rule(2, TlvWireType::Digest32),
                required_rule(3, TlvWireType::Digest32),
                required_rule(4, TlvWireType::Digest32),
            ],
        )?,
        value => {
            return Err(EmbeddingError::UnsupportedCode {
                field: "RDF term form",
                value,
            });
        }
    }
    Ok(())
}

fn validate_binding_contract(bytes: &[u8]) -> Result<(), EmbeddingError> {
    validate_schema(
        bytes,
        &[
            required_rule(1, TlvWireType::Utf8),
            required_rule(2, TlvWireType::Utf8),
            optional_rule(3, TlvWireType::Bytes),
            optional_rule(4, TlvWireType::Bytes),
            optional_rule(5, TlvWireType::Bytes),
        ],
    )?;
    require_nonempty_tlv(bytes, 1, TlvWireType::Utf8, "binding role")?;
    require_nonempty_tlv(bytes, 2, TlvWireType::Utf8, "binding media type")?;
    for tag in 3..=5 {
        if optional_tlv(bytes, tag)?.is_some_and(|entry| entry.value.is_empty()) {
            return Err(EmbeddingError::Missing("binding optional bytes"));
        }
    }
    Ok(())
}

fn validate_index_guard(bytes: &[u8]) -> Result<(), EmbeddingError> {
    const RULES: [FieldRule; 9] = [
        required_rule(1, TlvWireType::Block),
        required_rule(2, TlvWireType::Utf8),
        required_rule(3, TlvWireType::Bytes),
        required_rule(4, TlvWireType::Digest32),
        required_rule(5, TlvWireType::Block),
        required_rule(6, TlvWireType::U32),
        required_rule(7, TlvWireType::Utf8),
        optional_rule(8, TlvWireType::Digest32),
        required_rule(9, TlvWireType::Bool),
    ];
    validate_schema(bytes, &RULES)?;
    validate_artifact_identity(
        required_tlv(bytes, 1, TlvWireType::Block, "index artifact")?.value,
    )?;
    require_nonempty_tlv(bytes, 2, TlvWireType::Utf8, "index parameter encoding")?;
    validate_loss_contract(required_tlv(bytes, 5, TlvWireType::Block, "loss contract")?.value)?;
    if !matches!(
        tlv_u32(required_tlv(bytes, 6, TlvWireType::U32, "index use role")?)?,
        1..=3
    ) {
        return Err(EmbeddingError::Malformed("index use role"));
    }
    require_nonempty_tlv(bytes, 7, TlvWireType::Utf8, "index payload media type")?;
    if required_tlv(bytes, 9, TlvWireType::Bool, "index rebuildable")?.value != [1] {
        return Err(EmbeddingError::Malformed("index guard is not rebuildable"));
    }
    Ok(())
}

fn validate_loss_contract(bytes: &[u8]) -> Result<(), EmbeddingError> {
    let transformed = required_tlv(bytes, 2, TlvWireType::Bool, "loss transform")?.value;
    let rules: &[FieldRule] = if transformed == [0] {
        &[
            required_rule(1, TlvWireType::Bool),
            required_rule(2, TlvWireType::Bool),
        ]
    } else if transformed == [1] {
        &[
            required_rule(1, TlvWireType::Bool),
            required_rule(2, TlvWireType::Bool),
            required_rule(3, TlvWireType::Utf8),
            required_rule(4, TlvWireType::Bytes),
            required_rule(5, TlvWireType::Digest32),
        ]
    } else {
        return Err(EmbeddingError::Malformed("loss transformation boolean"));
    };
    validate_schema(bytes, rules)?;
    if required_tlv(bytes, 1, TlvWireType::Bool, "approximate search")?.value != [1] {
        return Err(EmbeddingError::Malformed(
            "index loss contract is not approximate",
        ));
    }
    if transformed == [1] {
        require_nonempty_tlv(bytes, 3, TlvWireType::Utf8, "loss encoding")?;
    }
    Ok(())
}

fn required_tlv<'a>(
    bytes: &'a [u8],
    tag: u16,
    wire: TlvWireType,
    context: &'static str,
) -> Result<TlvEntryRef<'a>, EmbeddingError> {
    let entry = optional_tlv(bytes, tag)?.ok_or(EmbeddingError::Missing(context))?;
    if entry.wire_type != wire || !entry.critical {
        return Err(EmbeddingError::MalformedTlv(
            "required field has wrong type or criticality",
        ));
    }
    Ok(entry)
}

fn optional_tlv(bytes: &[u8], tag: u16) -> Result<Option<TlvEntryRef<'_>>, EmbeddingError> {
    Ok(canonical_tlv(bytes)?.find(|entry| entry.tag == tag))
}

fn require_nonempty_tlv<'a>(
    bytes: &'a [u8],
    tag: u16,
    wire: TlvWireType,
    context: &'static str,
) -> Result<&'a str, EmbeddingError> {
    let entry = required_tlv(bytes, tag, wire, context)?;
    if entry.value.is_empty() {
        return Err(EmbeddingError::Missing(context));
    }
    core::str::from_utf8(entry.value).map_err(|_| EmbeddingError::InvalidUtf8(context))
}

fn required_digest(
    bytes: &[u8],
    tag: u16,
    context: &'static str,
) -> Result<[u8; 32], EmbeddingError> {
    let entry = required_tlv(bytes, tag, TlvWireType::Digest32, context)?;
    Ok(array32(entry.value, 0))
}

fn tlv_u32(entry: TlvEntryRef<'_>) -> Result<u32, EmbeddingError> {
    read_u32(entry.value, 0).map_err(|_| EmbeddingError::MalformedTlv("invalid u32 value"))
}

fn stage_is_applied(bytes: &[u8]) -> Result<bool, EmbeddingError> {
    match tlv_u32(required_tlv(
        bytes,
        1,
        TlvWireType::U32,
        "stage applied marker",
    )?)? {
        0 => Ok(false),
        1 => Ok(true),
        value => Err(EmbeddingError::UnsupportedCode {
            field: "stage applied marker",
            value,
        }),
    }
}

fn tlv_u64(entry: TlvEntryRef<'_>) -> Result<u64, EmbeddingError> {
    read_u64(entry.value, 0).map_err(|_| EmbeddingError::MalformedTlv("invalid u64 value"))
}

fn block_list_count(bytes: &[u8]) -> Result<usize, EmbeddingError> {
    require_minimum(bytes, 8, "TLV block list")?;
    require_u32(bytes, 4, 0, "TLV block-list reserved")?;
    usize::try_from(read_u32(bytes, 0)?)
        .map_err(|_| EmbeddingError::ArithmeticOverflow("TLV block-list count"))
}

fn block_list_item(bytes: &[u8], wanted: usize) -> Result<&[u8], EmbeddingError> {
    let count = block_list_count(bytes)?;
    if wanted >= count {
        return Err(EmbeddingError::Missing("TLV block-list item"));
    }
    let mut position = 8usize;
    for index in 0..count {
        let length = host_usize(read_u64(bytes, position)?, "TLV block-list item length")?;
        position = position
            .checked_add(8)
            .ok_or(EmbeddingError::ArithmeticOverflow("TLV block-list item"))?;
        let end = position
            .checked_add(length)
            .ok_or(EmbeddingError::ArithmeticOverflow(
                "TLV block-list item end",
            ))?;
        let item = bytes.get(position..end).ok_or(EmbeddingError::Truncated)?;
        if index == wanted {
            return Ok(item);
        }
        position = align8(end)?;
    }
    Err(EmbeddingError::Missing("TLV block-list item"))
}

fn validate_absolute_iri(value: &str) -> Result<(), EmbeddingError> {
    let Some(colon) = value.find(':') else {
        return Err(EmbeddingError::Malformed("RDF IRI is not absolute"));
    };
    let scheme = &value[..colon];
    if scheme.is_empty()
        || !scheme.as_bytes()[0].is_ascii_alphabetic()
        || !scheme
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
        || value
            .bytes()
            .any(|byte| byte <= b' ' || byte == b'<' || byte == b'>')
    {
        return Err(EmbeddingError::Malformed("invalid absolute RDF IRI"));
    }
    Ok(())
}

fn validate_language(value: &str) -> Result<(), EmbeddingError> {
    if value.is_empty()
        || value.starts_with('-')
        || value.ends_with('-')
        || value.contains("--")
        || value.bytes().any(|byte| {
            byte.is_ascii_uppercase()
                || !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
    {
        return Err(EmbeddingError::Malformed("invalid lowercase language tag"));
    }
    Ok(())
}

fn validate_target_ordinal(
    kind: TargetKind,
    flags: u32,
    ordinal: u64,
) -> Result<(), EmbeddingError> {
    let present = flags & SOURCE_ORDINAL_PRESENT != 0;
    if present == (ordinal == u64::MAX) {
        return Err(EmbeddingError::Malformed("source ordinal presence"));
    }
    if present
        && !matches!(
            kind,
            TargetKind::RdfTerm
                | TargetKind::RdfStatement
                | TargetKind::RdfReifier
                | TargetKind::RdfAnnotation
        )
    {
        return Err(EmbeddingError::Malformed(
            "ordinal on unsupported target kind",
        ));
    }
    Ok(())
}

fn validate_relation_kind(kind: u32) -> Result<(), EmbeddingError> {
    if matches!(kind, 1 | 2 | 16..=26 | 32..=34 | 0x8000_0000) {
        Ok(())
    } else {
        Err(EmbeddingError::UnsupportedCode {
            field: "relation kind",
            value: kind,
        })
    }
}

fn validate_relation_endpoints(
    kind: u32,
    subject: TargetKind,
    object: TargetKind,
) -> Result<(), EmbeddingError> {
    let valid = match kind {
        1 => (TargetKind::Corpus, TargetKind::Document),
        2 => (TargetKind::Document, TargetKind::Chunk),
        16 => (TargetKind::RdfDataset, TargetKind::RdfGraph),
        17 => (TargetKind::RdfGraph, TargetKind::RdfStatement),
        18..=20 => (TargetKind::RdfStatement, TargetKind::RdfTerm),
        21 => (TargetKind::RdfStatement, TargetKind::RdfReifier),
        22 => (TargetKind::RdfReifier, TargetKind::RdfTerm),
        23 => (TargetKind::RdfReifier, TargetKind::RdfAnnotation),
        24 | 25 => (TargetKind::RdfAnnotation, TargetKind::RdfTerm),
        26 => (TargetKind::RdfGraph, TargetKind::RdfTerm),
        32..=34 => (TargetKind::RdfTerm, TargetKind::RdfTerm),
        0x8000_0000 => return Ok(()),
        value => {
            return Err(EmbeddingError::UnsupportedCode {
                field: "relation kind",
                value,
            });
        }
    };
    if (subject, object) != valid {
        return Err(EmbeddingError::Malformed("relation endpoint kinds"));
    }
    Ok(())
}

fn deterministic_norm_f32(bytes: &[u8], row: u64, dimension: u32) -> Result<f64, EmbeddingError> {
    let mut scale = 0.0f64;
    let mut ssq = 1.0f64;
    for (column, chunk) in bytes.chunks_exact(4).enumerate() {
        let value = f32::from_bits(u32::from_le_bytes(
            chunk.try_into().map_err(|_| EmbeddingError::Truncated)?,
        ));
        if !value.is_finite() {
            return Err(EmbeddingError::NonFiniteScalar {
                row,
                column: u32::try_from(column).unwrap_or(u32::MAX),
            });
        }
        norm_fold(f64::from(value).abs(), &mut scale, &mut ssq);
    }
    finish_norm(scale, ssq, row, dimension)
}

fn deterministic_norm_f64(bytes: &[u8], row: u64, dimension: u32) -> Result<f64, EmbeddingError> {
    let mut scale = 0.0f64;
    let mut ssq = 1.0f64;
    for (column, chunk) in bytes.chunks_exact(8).enumerate() {
        let value = f64::from_bits(u64::from_le_bytes(
            chunk.try_into().map_err(|_| EmbeddingError::Truncated)?,
        ));
        if !value.is_finite() {
            return Err(EmbeddingError::NonFiniteScalar {
                row,
                column: u32::try_from(column).unwrap_or(u32::MAX),
            });
        }
        norm_fold(value.abs(), &mut scale, &mut ssq);
    }
    finish_norm(scale, ssq, row, dimension)
}

// PURREMB v1 prescribes separate rounded multiply and add operations; fusing
// them would change portable projection bytes.
#[allow(clippy::suboptimal_flops)]
fn norm_fold(value: f64, scale: &mut f64, ssq: &mut f64) {
    if value == 0.0 {
        return;
    }
    if *scale < value {
        let ratio = *scale / value;
        let square = ratio * ratio;
        *ssq = 1.0 + *ssq * square;
        *scale = value;
    } else {
        let ratio = value / *scale;
        let square = ratio * ratio;
        *ssq += square;
    }
}

fn finish_norm(scale: f64, ssq: f64, row: u64, dimension: u32) -> Result<f64, EmbeddingError> {
    if scale == 0.0 {
        return Err(EmbeddingError::ZeroNorm { row, dimension });
    }
    let norm = scale * ssq.sqrt();
    if !norm.is_finite() || norm == 0.0 {
        return Err(EmbeddingError::ContentMismatch(
            "invalid deterministic L2 norm",
        ));
    }
    Ok(norm)
}

fn native_f32(bytes: &[u8]) -> Option<&[f32]> {
    if !bytes.len().is_multiple_of(size_of::<f32>()) {
        return None;
    }
    // SAFETY: every bit pattern is a valid `f32`; `align_to` reports any
    // unaligned prefix or incomplete suffix instead of constructing a bad view.
    let (prefix, native, suffix) = unsafe { bytes.align_to::<f32>() };
    (prefix.is_empty() && suffix.is_empty()).then_some(native)
}

fn native_f64(bytes: &[u8]) -> Option<&[f64]> {
    if !bytes.len().is_multiple_of(size_of::<f64>()) {
        return None;
    }
    // SAFETY: every bit pattern is a valid `f64`; `align_to` reports any
    // unaligned prefix or incomplete suffix instead of constructing a bad view.
    let (prefix, native, suffix) = unsafe { bytes.align_to::<f64>() };
    (prefix.is_empty() && suffix.is_empty()).then_some(native)
}

fn borrowed_span<'a>(
    bytes: &'a [u8],
    offset: u64,
    length: u64,
    context: &'static str,
) -> Result<&'a [u8], EmbeddingError> {
    let start = host_usize(offset, context)?;
    let length = host_usize(length, context)?;
    let end = start
        .checked_add(length)
        .ok_or(EmbeddingError::ArithmeticOverflow(context))?;
    bytes
        .get(start..end)
        .ok_or_else(|| EmbeddingError::InvalidSpan {
            context,
            offset,
            length: u64::try_from(length).unwrap_or(u64::MAX),
        })
}

fn fixed_record(
    bytes: &[u8],
    table_offset: usize,
    record_size: usize,
    index: usize,
) -> Result<&[u8], EmbeddingError> {
    let start = table_offset
        .checked_add(
            index
                .checked_mul(record_size)
                .ok_or(EmbeddingError::ArithmeticOverflow("record offset"))?,
        )
        .ok_or(EmbeddingError::ArithmeticOverflow("record offset"))?;
    bytes
        .get(start..start + record_size)
        .ok_or(EmbeddingError::Truncated)
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, EmbeddingError> {
    let end = offset
        .checked_add(4)
        .ok_or(EmbeddingError::ArithmeticOverflow("u32 read"))?;
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..end)
            .ok_or(EmbeddingError::Truncated)?
            .try_into()
            .map_err(|_| EmbeddingError::Truncated)?,
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, EmbeddingError> {
    let end = offset
        .checked_add(8)
        .ok_or(EmbeddingError::ArithmeticOverflow("u64 read"))?;
    Ok(u64::from_le_bytes(
        bytes
            .get(offset..end)
            .ok_or(EmbeddingError::Truncated)?
            .try_into()
            .map_err(|_| EmbeddingError::Truncated)?,
    ))
}

fn infallible_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("structurally validated fixed u32 field"),
    )
}

fn infallible_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("structurally validated fixed u64 field"),
    )
}

fn array32(bytes: &[u8], offset: usize) -> [u8; 32] {
    bytes[offset..offset + 32]
        .try_into()
        .expect("structurally validated 32-byte field")
}

fn require_u32(
    bytes: &[u8],
    offset: usize,
    expected: u32,
    context: &'static str,
) -> Result<(), EmbeddingError> {
    if read_u32(bytes, offset)? != expected {
        return Err(EmbeddingError::Malformed(context));
    }
    Ok(())
}

fn require_u64(
    bytes: &[u8],
    offset: usize,
    expected: u64,
    context: &'static str,
) -> Result<(), EmbeddingError> {
    if read_u64(bytes, offset)? != expected {
        return Err(EmbeddingError::Malformed(context));
    }
    Ok(())
}

fn require_zero(bytes: &[u8], context: &'static str) -> Result<(), EmbeddingError> {
    if bytes.iter().any(|byte| *byte != 0) {
        return Err(EmbeddingError::ReservedNonzero(context));
    }
    Ok(())
}

fn require_minimum(
    bytes: &[u8],
    minimum: usize,
    context: &'static str,
) -> Result<(), EmbeddingError> {
    if bytes.len() < minimum {
        return Err(EmbeddingError::InvalidSpan {
            context,
            offset: 0,
            length: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        });
    }
    Ok(())
}

fn host_usize(value: u64, context: &'static str) -> Result<usize, EmbeddingError> {
    usize::try_from(value).map_err(|_| EmbeddingError::ArithmeticOverflow(context))
}

fn bounded_count(value: u64, field: &'static str, nonzero: bool) -> Result<usize, EmbeddingError> {
    if nonzero && value == 0 {
        return Err(EmbeddingError::Missing(field));
    }
    if value > u64::from(u32::MAX) {
        return Err(EmbeddingError::CountLimit { field, value });
    }
    host_usize(value, field)
}

fn checked_product_u64(
    count: usize,
    width: usize,
    context: &'static str,
) -> Result<u64, EmbeddingError> {
    let value = count
        .checked_mul(width)
        .ok_or(EmbeddingError::ArithmeticOverflow(context))?;
    u64::try_from(value).map_err(|_| EmbeddingError::ArithmeticOverflow(context))
}

fn checked_table_end(offset: usize, count: usize, width: usize) -> Result<usize, EmbeddingError> {
    offset
        .checked_add(
            count
                .checked_mul(width)
                .ok_or(EmbeddingError::ArithmeticOverflow("table length"))?,
        )
        .ok_or(EmbeddingError::ArithmeticOverflow("table end"))
}

fn align8(value: usize) -> Result<usize, EmbeddingError> {
    value
        .checked_add(7)
        .map(|biased| biased & !7)
        .ok_or(EmbeddingError::ArithmeticOverflow("8-byte alignment"))
}

fn validate_zero_padding(bytes: &[u8], start: u64, end: u64) -> Result<(), EmbeddingError> {
    let start = host_usize(start, "padding start")?;
    let end = host_usize(end, "padding end")?;
    validate_zero_padding_usize(bytes, start, end)
}

fn validate_zero_padding_usize(
    bytes: &[u8],
    start: usize,
    end: usize,
) -> Result<(), EmbeddingError> {
    let padding = bytes.get(start..end).ok_or(EmbeddingError::Truncated)?;
    if let Some(relative) = padding.iter().position(|byte| *byte != 0) {
        return Err(EmbeddingError::InvalidPadding {
            offset: u64::try_from(start + relative).unwrap_or(u64::MAX),
        });
    }
    Ok(())
}

fn validate_sorted_ids(bytes: &[u8], context: &'static str) -> Result<(), EmbeddingError> {
    if bytes
        .chunks_exact(32)
        .zip(bytes.chunks_exact(32).skip(1))
        .any(|(left, right)| left >= right)
    {
        return Err(EmbeddingError::NonCanonicalOrder(context));
    }
    Ok(())
}

fn binary_search_ids(bytes: &[u8], wanted: &[u8; 32]) -> Option<usize> {
    let mut low = 0;
    let mut high = bytes.len() / 32;
    while low < high {
        let middle = low + (high - low) / 2;
        let value = bytes.get(middle * 32..middle * 32 + 32)?;
        match value.cmp(wanted) {
            core::cmp::Ordering::Less => low = middle + 1,
            core::cmp::Ordering::Greater => high = middle,
            core::cmp::Ordering::Equal => return Some(middle),
        }
    }
    None
}

fn binary_search_records(
    bytes: &[u8],
    table_offset: usize,
    record_size: usize,
    count: usize,
    wanted: &[u8; 32],
) -> Option<usize> {
    let mut low = 0;
    let mut high = count;
    while low < high {
        let middle = low + (high - low) / 2;
        let record = fixed_record(bytes, table_offset, record_size, middle).ok()?;
        match record[..32].cmp(wanted) {
            core::cmp::Ordering::Less => low = middle + 1,
            core::cmp::Ordering::Greater => high = middle,
            core::cmp::Ordering::Equal => return Some(middle),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_f32_iteration_preserves_signed_zero() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0.0f32.to_bits().to_le_bytes());
        bytes.extend_from_slice(&(-0.0f32).to_bits().to_le_bytes());
        bytes.extend_from_slice(&1.5f32.to_bits().to_le_bytes());
        let values = F32Scalars::new(&bytes, 4, 0)
            .collect::<Result<Vec<_>, _>>()
            .expect("finite row");
        assert_eq!(values[0].to_bits(), 0.0f32.to_bits());
        assert_eq!(values[1].to_bits(), (-0.0f32).to_bits());
        assert_eq!(values[2], 1.5);
    }

    #[test]
    fn portable_scalar_iteration_rejects_nonfinite_values() {
        let bytes = f64::INFINITY.to_bits().to_le_bytes();
        let error = F64Scalars::new(&bytes, 9, 7)
            .next()
            .expect("one scalar")
            .expect_err("infinity is invalid");
        assert_eq!(error, EmbeddingError::NonFiniteScalar { row: 9, column: 7 });
    }

    #[test]
    fn deterministic_l2_is_allocation_free_and_normative() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&3.0f32.to_bits().to_le_bytes());
        bytes.extend_from_slice(&4.0f32.to_bits().to_le_bytes());
        let values = L2F32Scalars::new(&bytes, 0, 2)
            .expect("nonzero row")
            .collect::<Result<Vec<_>, _>>()
            .expect("finite normalized row");
        assert_eq!(values, vec![0.6, 0.8]);
    }

    #[test]
    fn deterministic_l2_rejects_zero_norm() {
        let bytes = [0u8; 16];
        assert_eq!(
            L2F64Scalars::new(&bytes, 3, 2).expect_err("zero row"),
            EmbeddingError::ZeroNorm {
                row: 3,
                dimension: 2,
            }
        );
    }

    #[test]
    fn framing_rejects_truncation_before_reading_fields() {
        assert_eq!(
            EmbeddingView::from_bytes(&[0; 127]).expect_err("short header"),
            EmbeddingError::Truncated
        );
    }

    #[test]
    fn framing_rejects_bad_magic() {
        let bytes = [0u8; PURREMB_HEADER_LENGTH as usize];
        assert_eq!(
            EmbeddingView::from_bytes(&bytes).expect_err("bad magic"),
            EmbeddingError::BadMagic
        );
    }

    #[test]
    fn language_validation_requires_lowercase_canonical_form() {
        assert!(validate_language("en-ca").is_ok());
        assert!(validate_language("EN-ca").is_err());
        assert!(validate_language("en--ca").is_err());
    }

    #[test]
    fn absolute_iri_validation_requires_a_scheme() {
        assert!(validate_absolute_iri("https://example.org/value").is_ok());
        assert!(validate_absolute_iri("relative/path").is_err());
        assert!(validate_absolute_iri("1bad:value").is_err());
    }
}
