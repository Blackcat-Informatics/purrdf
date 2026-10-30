// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source-free metadata extension and matrix commitment records.
use super::contract::{PrefixPostprocessing, VectorDtype};
use super::error::EmbeddingError;
use super::identity::{
    FamilyId, MatrixContentDigest, MatrixId, ProjectionContentDigest, ProjectionId, TargetSetId,
    VectorSpaceId,
};
use super::wire::{SECTION_CRITICAL, SECTION_DERIVED, SECTION_EXTENSION_MIN};
/// One caller extension section retained byte-for-byte by the writer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionSection {
    /// Caller section kind in the extension range.
    pub kind: u32,
    /// Instance number within the caller section kind.
    pub instance: u32,
    /// Directory flags; only `CRITICAL` and `DERIVED` bits are defined in v1.
    pub flags: u32,
    /// Exact section bytes.
    pub bytes: Vec<u8>,
}

impl ExtensionSection {
    /// Constructs and validates a caller extension section.
    pub fn new(
        kind: u32,
        instance: u32,
        flags: u32,
        bytes: Vec<u8>,
    ) -> Result<Self, EmbeddingError> {
        if kind < SECTION_EXTENSION_MIN {
            return Err(EmbeddingError::UnsupportedCode {
                field: "extension section kind",
                value: kind,
            });
        }
        if flags & !(SECTION_CRITICAL | SECTION_DERIVED) != 0 {
            return Err(EmbeddingError::ReservedNonzero("extension section flags"));
        }
        if bytes.is_empty() {
            return Err(EmbeddingError::InvalidSpan {
                context: "empty extension section",
                offset: 0,
                length: 0,
            });
        }
        Ok(Self {
            kind,
            instance,
            flags,
            bytes,
        })
    }
}

/// A fully derived effective-projection commitment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectionCommitment {
    /// Identity of the effective projection.
    pub projection_id: ProjectionId,
    /// Digest of the logical row-prefix byte stream.
    pub content_digest: ProjectionContentDigest,
    /// Effective vector-space identity.
    pub vector_space_id: VectorSpaceId,
    /// Nonzero leading-prefix dimension.
    pub effective_dimension: u32,
    /// Raw or deterministic-L2 prefix behavior.
    pub postprocessing: PrefixPostprocessing,
}

/// A fully derived stored-matrix commitment used by the streaming writer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixCommitment {
    /// Identity of the stored matrix.
    pub matrix_id: MatrixId,
    /// Domain-separated digest of the exact scalar bytes.
    pub content_digest: MatrixContentDigest,
    /// Identity of the canonical sorted target row set.
    pub target_set_id: TargetSetId,
    /// Family whose generation contract produced the matrix.
    pub family_id: FamilyId,
    /// Authoritative scalar type.
    pub dtype: VectorDtype,
    /// Nonzero matrix row count.
    pub row_count: u64,
    /// Nonzero dense stored row width.
    pub stored_dimension: u32,
    /// One commitment for every effective family space.
    pub projections: Vec<ProjectionCommitment>,
}

impl MatrixCommitment {
    /// Exact `MATRIX_DATA` section length.
    pub fn data_length(&self) -> Result<u64, EmbeddingError> {
        checked_matrix_byte_length(self.row_count, self.stored_dimension, self.dtype)
    }
}

/// Checked physical byte length of one matrix body.
pub fn checked_matrix_byte_length(
    row_count: u64,
    stored_dimension: u32,
    dtype: VectorDtype,
) -> Result<u64, EmbeddingError> {
    row_count
        .checked_mul(u64::from(stored_dimension))
        .and_then(|scalars| scalars.checked_mul(u64::from(dtype.width())))
        .ok_or(EmbeddingError::ArithmeticOverflow("matrix byte length"))
}
