// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Canonical in-memory and bounded-memory PURREMB writers.
//!
//! The in-memory builder accepts matrix rows in arbitrary order and sorts them
//! by [`TargetId`]. The streaming writer accepts precommitted matrices, orders
//! them by [`MatrixId`], and verifies strictly increasing target rows while
//! writing each `MATRIX_DATA` body directly to a caller-owned `Write + Seek`.

use std::io::{Seek, SeekFrom, Write};

use sha2::{Digest as _, Sha256};

use crate::ContentDigest;

use super::contract::{PrefixPostprocessing, VectorDtype};
use super::error::{DigestKind, EmbeddingError, EmbeddingWriteError};
use super::identity::{
    ArtifactRoot, FamilyId, MatrixContentDigest, MatrixId, ProjectionContentDigest, ProjectionId,
    TargetId, TargetSetId, VectorSpaceId, derive_matrix_content_digest, derive_matrix_id,
    derive_projection_id, derive_target_set_id, derive_vector_space_id,
};
use super::metadata::CanonicalMetadataInput;
use super::wire::{
    EncodedArtifact, FileLayout, PURREMB_DIRECTORY_ENTRY_LENGTH, PURREMB_HEADER_LENGTH,
    PURREMB_TRAILER_LENGTH, SECTION_CONTRACTS, SECTION_CRITICAL, SECTION_DERIVED,
    SECTION_EXTENSION_MIN, SECTION_EXTERNAL_BINDINGS, SECTION_INDEX_GUARDS, SECTION_INDEX_PAYLOAD,
    SECTION_MATRICES, SECTION_MATRIX_DATA, SECTION_RELATIONS, SECTION_SOURCE, SECTION_TARGET_SETS,
    SECTION_TARGETS, SECTION_TOKEN_SPANS, SectionDescriptor, SectionKey, SectionPayload,
    encode_artifact,
};

const MATRICES_HEADER_LENGTH: u64 = 96;
const MATRIX_RECORD_LENGTH: u64 = 160;
const PROJECTION_RECORD_LENGTH: u64 = 152;

const D_TARGET_SET: &[u8] = b"purrdf.purremb.v1.target-set\0";
const D_MATRIX_CONTENT: &[u8] = b"purrdf.purremb.v1.matrix-content\0";
const D_PROJECTION_CONTENT: &[u8] = b"purrdf.purremb.v1.projection-content\0";

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

/// Canonically encoded non-matrix sections supplied to both writer paths.
///
/// These bytes are the output of the typed metadata encoders. The writer owns
/// them, validates the duplicate source digest, and supplies the generated
/// `MATRICES` and `MATRIX_DATA` sections itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalMetadataSections {
    /// Exact SHA-256 of the source `.purrpck` bytes.
    pub(super) source_exact_digest: ContentDigest,
    /// Exact 128-byte `SOURCE` section.
    pub(super) source: Vec<u8>,
    /// Canonical `CONTRACTS` section.
    pub(super) contracts: Vec<u8>,
    /// Canonical `TARGETS` section.
    pub(super) targets: Vec<u8>,
    /// Canonical `TARGET_SETS` section.
    pub(super) target_sets: Vec<u8>,
    /// Canonical `RELATIONS` section.
    pub(super) relations: Vec<u8>,
    /// Canonical `TOKEN_SPANS` section.
    pub(super) token_spans: Vec<u8>,
    /// Canonical `EXTERNAL_BINDINGS` section.
    pub(super) external_bindings: Vec<u8>,
    /// Canonical `INDEX_GUARDS` section.
    pub(super) index_guards: Vec<u8>,
    /// Exact inline index payloads; vector order assigns instances from one.
    pub(super) inline_index_payloads: Vec<Vec<u8>>,
    /// Caller extension sections.
    pub(super) extensions: Vec<ExtensionSection>,
}

impl CanonicalMetadataSections {
    fn validate(&self) -> Result<(), EmbeddingError> {
        if self.source.len() != 128 {
            return Err(EmbeddingError::InvalidSpan {
                context: "SOURCE section",
                offset: 0,
                length: u64::try_from(self.source.len()).unwrap_or(u64::MAX),
            });
        }
        let mut source_digest = [0u8; 32];
        source_digest.copy_from_slice(&self.source[24..56]);
        if source_digest != *self.source_exact_digest.as_bytes() {
            return Err(EmbeddingError::DigestMismatch {
                kind: DigestKind::SourceExact,
                expected: source_digest,
                actual: *self.source_exact_digest.as_bytes(),
            });
        }
        for (context, bytes) in [
            ("CONTRACTS section", self.contracts.as_slice()),
            ("TARGETS section", self.targets.as_slice()),
            ("TARGET_SETS section", self.target_sets.as_slice()),
            ("RELATIONS section", self.relations.as_slice()),
            ("TOKEN_SPANS section", self.token_spans.as_slice()),
            (
                "EXTERNAL_BINDINGS section",
                self.external_bindings.as_slice(),
            ),
            ("INDEX_GUARDS section", self.index_guards.as_slice()),
        ] {
            if bytes.is_empty() {
                return Err(EmbeddingError::InvalidSpan {
                    context,
                    offset: 0,
                    length: 0,
                });
            }
        }
        for payload in &self.inline_index_payloads {
            if payload.is_empty() {
                return Err(EmbeddingError::InvalidSpan {
                    context: "empty INDEX_PAYLOAD section",
                    offset: 0,
                    length: 0,
                });
            }
        }
        let mut extension_keys = self
            .extensions
            .iter()
            .map(|extension| SectionKey::new(extension.kind, extension.instance))
            .collect::<Vec<_>>();
        extension_keys.sort_unstable();
        if extension_keys.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(EmbeddingError::Duplicate("extension section"));
        }
        for extension in &self.extensions {
            if extension.kind < SECTION_EXTENSION_MIN {
                return Err(EmbeddingError::UnsupportedCode {
                    field: "extension section kind",
                    value: extension.kind,
                });
            }
            if extension.flags & !(SECTION_CRITICAL | SECTION_DERIVED) != 0 {
                return Err(EmbeddingError::ReservedNonzero("extension section flags"));
            }
            if extension.bytes.is_empty() {
                return Err(EmbeddingError::InvalidSpan {
                    context: "empty extension section",
                    offset: 0,
                    length: 0,
                });
            }
        }
        Ok(())
    }

    fn into_payloads(self, matrices: Vec<u8>) -> Result<Vec<SectionPayload>, EmbeddingError> {
        self.validate()?;
        let mut sections = vec![
            SectionPayload::new(SECTION_SOURCE, 0, SECTION_CRITICAL, self.source),
            SectionPayload::new(SECTION_CONTRACTS, 0, SECTION_CRITICAL, self.contracts),
            SectionPayload::new(SECTION_TARGETS, 0, SECTION_CRITICAL, self.targets),
            SectionPayload::new(SECTION_TARGET_SETS, 0, SECTION_CRITICAL, self.target_sets),
            SectionPayload::new(SECTION_RELATIONS, 0, SECTION_CRITICAL, self.relations),
            SectionPayload::new(SECTION_TOKEN_SPANS, 0, SECTION_CRITICAL, self.token_spans),
            SectionPayload::new(SECTION_MATRICES, 0, SECTION_CRITICAL, matrices),
            SectionPayload::new(
                SECTION_EXTERNAL_BINDINGS,
                0,
                SECTION_CRITICAL,
                self.external_bindings,
            ),
            SectionPayload::new(
                SECTION_INDEX_GUARDS,
                0,
                SECTION_CRITICAL | SECTION_DERIVED,
                self.index_guards,
            ),
        ];
        for (index, bytes) in self.inline_index_payloads.into_iter().enumerate() {
            let instance = u32::try_from(index + 1).map_err(|_| EmbeddingError::CountLimit {
                field: "inline index",
                value: u64::try_from(index + 1).unwrap_or(u64::MAX),
            })?;
            sections.push(SectionPayload::new(
                SECTION_INDEX_PAYLOAD,
                instance,
                SECTION_CRITICAL | SECTION_DERIVED,
                bytes,
            ));
        }
        sections.extend(self.extensions.into_iter().map(|extension| {
            SectionPayload::new(
                extension.kind,
                extension.instance,
                extension.flags,
                extension.bytes,
            )
        }));
        Ok(sections)
    }
}

/// One target-associated row accepted by the unordered in-memory builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixRow<T> {
    /// Target identity that determines canonical row position.
    pub target_id: TargetId,
    /// Dense row values in stored-dimension order.
    pub values: Vec<T>,
}

impl<T> MatrixRow<T> {
    /// Constructs one target-associated matrix row.
    #[must_use]
    pub const fn new(target_id: TargetId, values: Vec<T>) -> Self {
        Self { target_id, values }
    }
}

/// One effective leading-prefix projection declared for a matrix family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectionSpec {
    /// Derived identity of the family, dimension, and postprocessing tuple.
    pub vector_space_id: VectorSpaceId,
    /// Nonzero leading-prefix dimension.
    pub effective_dimension: u32,
    /// Exact raw or deterministic-L2 postprocessing.
    pub postprocessing: PrefixPostprocessing,
}

impl ProjectionSpec {
    /// Derives a projection declaration from its owning family.
    #[must_use]
    pub fn derive(
        family_id: FamilyId,
        effective_dimension: u32,
        postprocessing: PrefixPostprocessing,
    ) -> Self {
        Self {
            vector_space_id: derive_vector_space_id(
                family_id,
                effective_dimension,
                postprocessing.code(),
            ),
            effective_dimension,
            postprocessing,
        }
    }
}

/// One unordered matrix accepted by [`EmbeddingBuilder`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixInput<T> {
    /// Family whose generation contract produced the values.
    pub family_id: FamilyId,
    /// Expected identity of the canonical sorted target rows.
    pub target_set_id: TargetSetId,
    /// Dense stored row width.
    pub stored_dimension: u32,
    /// Target-associated rows in arbitrary order.
    pub rows: Vec<MatrixRow<T>>,
    /// Effective spaces in arbitrary order; the builder sorts by dimension.
    pub projections: Vec<ProjectionSpec>,
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

#[derive(Debug)]
enum MatrixInputKind {
    F32(MatrixInput<f32>),
    F64(MatrixInput<f64>),
}

#[derive(Debug)]
struct PreparedMatrix {
    commitment: MatrixCommitment,
    bytes: Vec<u8>,
}

/// Canonical in-memory PURREMB builder.
///
/// Matrix rows and matrix inputs may arrive in arbitrary order. Target rows are
/// sorted by `TargetId`; matrices are sorted by their derived `MatrixId` before
/// data-section instances are assigned.
#[derive(Debug)]
pub struct EmbeddingBuilder {
    metadata: BuilderMetadata,
    matrices: Vec<MatrixInputKind>,
}

#[derive(Debug)]
enum BuilderMetadata {
    Encoded(CanonicalMetadataSections),
    Typed(CanonicalMetadataInput),
}

impl EmbeddingBuilder {
    /// Creates an empty matrix builder around canonical non-matrix metadata.
    #[must_use]
    pub const fn new(metadata: CanonicalMetadataSections) -> Self {
        Self {
            metadata: BuilderMetadata::Encoded(metadata),
            matrices: Vec::new(),
        }
    }

    /// Creates an empty matrix builder from typed metadata records. Section
    /// encoding and all matrix-dependent cross-checks occur after matrix
    /// commitments have been derived.
    #[must_use]
    pub const fn from_typed_metadata(metadata: CanonicalMetadataInput) -> Self {
        Self {
            metadata: BuilderMetadata::Typed(metadata),
            matrices: Vec::new(),
        }
    }

    /// Adds an unordered binary32 matrix.
    pub fn add_f32_matrix(&mut self, matrix: MatrixInput<f32>) -> &mut Self {
        self.matrices.push(MatrixInputKind::F32(matrix));
        self
    }

    /// Adds an unordered binary64 matrix.
    pub fn add_f64_matrix(&mut self, matrix: MatrixInput<f64>) -> &mut Self {
        self.matrices.push(MatrixInputKind::F64(matrix));
        self
    }

    /// Canonicalizes all rows and emits one complete PURREMB artifact.
    pub fn build(self) -> Result<EncodedArtifact, EmbeddingError> {
        if self.matrices.is_empty() {
            return Err(EmbeddingError::Missing("matrix"));
        }
        let mut prepared = self
            .matrices
            .into_iter()
            .map(|matrix| match matrix {
                MatrixInputKind::F32(matrix) => prepare_matrix(matrix),
                MatrixInputKind::F64(matrix) => prepare_matrix(matrix),
            })
            .collect::<Result<Vec<_>, _>>()?;
        prepared.sort_unstable_by_key(|matrix| matrix.commitment.matrix_id);
        validate_prepared_matrix_order(&prepared)?;

        let commitments = prepared
            .iter()
            .map(|matrix| matrix.commitment.clone())
            .collect::<Vec<_>>();
        let matrices_section = encode_matrices_section(&commitments)?;
        let metadata = match self.metadata {
            BuilderMetadata::Encoded(metadata) => metadata,
            BuilderMetadata::Typed(metadata) => metadata.encode_for_matrices(&commitments)?,
        };
        metadata.validate()?;
        let source_exact_digest = metadata.source_exact_digest;
        let mut sections = metadata.into_payloads(matrices_section)?;
        for (index, matrix) in prepared.into_iter().enumerate() {
            let instance = matrix_instance(index)?;
            sections.push(SectionPayload::new(
                SECTION_MATRIX_DATA,
                instance,
                SECTION_CRITICAL,
                matrix.bytes,
            ));
        }
        encode_artifact(source_exact_digest, sections)
    }
}

/// Bounded-memory canonical writer over an initially empty seekable output.
///
/// Matrix commitments are sorted by `MatrixId` during construction. Call
/// [`Self::next_matrix`] to discover the next physical matrix, then supply its
/// rows in strictly increasing `TargetId` order through the matching dtype
/// method. Only one row buffer and one digest state per projection are retained.
#[derive(Debug)]
pub struct EmbeddingStreamWriter<W> {
    output: W,
    layout: FileLayout,
    matrices: Vec<MatrixCommitment>,
    next_matrix: usize,
}

impl<W: Write + Seek> EmbeddingStreamWriter<W> {
    /// Encodes typed metadata against the supplied commitments and creates a
    /// bounded-memory streaming writer.
    pub fn from_typed_metadata(
        output: W,
        metadata: CanonicalMetadataInput,
        matrices: Vec<MatrixCommitment>,
    ) -> Result<Self, EmbeddingWriteError> {
        let encoded = metadata.encode_for_matrices(&matrices)?;
        Self::new(output, encoded, matrices)
    }

    /// Plans the complete artifact and writes all canonical metadata and
    /// padding. Matrix bodies remain reserved until their rows are supplied.
    pub fn new(
        mut output: W,
        metadata: CanonicalMetadataSections,
        mut matrices: Vec<MatrixCommitment>,
    ) -> Result<Self, EmbeddingWriteError> {
        metadata.validate()?;
        if matrices.is_empty() {
            return Err(EmbeddingError::Missing("matrix").into());
        }
        if output.seek(SeekFrom::End(0))? != 0 {
            return Err(EmbeddingError::Malformed("stream output must be initially empty").into());
        }
        output.seek(SeekFrom::Start(0))?;

        matrices.sort_unstable_by_key(|matrix| matrix.matrix_id);
        validate_commitments(&matrices)?;
        let matrices_section = encode_matrices_section(&matrices)?;
        let source_exact_digest = metadata.source_exact_digest;
        let mut payloads = metadata.into_payloads(matrices_section)?;
        payloads.sort_unstable_by_key(|payload| payload.key);

        let mut descriptors = payloads
            .iter()
            .map(|payload| {
                Ok(SectionDescriptor::new(
                    payload.key.kind,
                    payload.key.instance,
                    payload.flags,
                    u64::try_from(payload.bytes.len())
                        .map_err(|_| EmbeddingError::ArithmeticOverflow("section length"))?,
                ))
            })
            .collect::<Result<Vec<_>, EmbeddingError>>()?;
        for (index, matrix) in matrices.iter().enumerate() {
            descriptors.push(SectionDescriptor::new(
                SECTION_MATRIX_DATA,
                matrix_instance(index)?,
                SECTION_CRITICAL,
                matrix.data_length()?,
            ));
        }
        let mut layout = FileLayout::plan(source_exact_digest, descriptors)?;
        for payload in &payloads {
            layout.set_section_digest(payload.key, Sha256::digest(&payload.bytes).into())?;
        }
        write_stream_skeleton(&mut output, &layout, &payloads)?;

        Ok(Self {
            output,
            layout,
            matrices,
            next_matrix: 0,
        })
    }

    /// Commitment for the matrix whose rows must be supplied next.
    #[must_use]
    pub fn next_matrix(&self) -> Option<&MatrixCommitment> {
        self.matrices.get(self.next_matrix)
    }

    /// Streams the next committed binary32 matrix in canonical target order.
    pub fn write_f32_matrix<I, R>(&mut self, rows: I) -> Result<(), EmbeddingWriteError>
    where
        I: IntoIterator<Item = (TargetId, R)>,
        R: AsRef<[f32]>,
    {
        self.write_matrix::<f32, _, _>(rows)
    }

    /// Streams the next committed binary64 matrix in canonical target order.
    pub fn write_f64_matrix<I, R>(&mut self, rows: I) -> Result<(), EmbeddingWriteError>
    where
        I: IntoIterator<Item = (TargetId, R)>,
        R: AsRef<[f64]>,
    {
        self.write_matrix::<f64, _, _>(rows)
    }

    /// Backpatches the populated directory, artifact root, and trailer.
    pub fn finish(mut self) -> Result<(W, ArtifactRoot), EmbeddingWriteError> {
        if self.next_matrix != self.matrices.len() {
            return Err(EmbeddingError::Missing("streamed MATRIX_DATA section").into());
        }
        let root = self.layout.artifact_root()?;
        write_at(&mut self.output, 0, &self.layout.header(root))?;
        write_at(
            &mut self.output,
            u64::from(PURREMB_HEADER_LENGTH),
            &self.layout.directory_bytes()?,
        )?;
        write_at(
            &mut self.output,
            self.layout.trailer_offset(),
            &self.layout.trailer(root),
        )?;
        self.output
            .seek(SeekFrom::Start(self.layout.file_length()))?;
        Ok((self.output, root))
    }

    fn write_matrix<T, I, R>(&mut self, rows: I) -> Result<(), EmbeddingWriteError>
    where
        T: MatrixScalar,
        I: IntoIterator<Item = (TargetId, R)>,
        R: AsRef<[T]>,
    {
        let commitment = self
            .matrices
            .get(self.next_matrix)
            .ok_or(EmbeddingError::Duplicate("streamed matrix"))?
            .clone();
        if commitment.dtype != T::DTYPE {
            return Err(EmbeddingError::UnsupportedCode {
                field: "streamed matrix dtype",
                value: T::DTYPE.code(),
            }
            .into());
        }
        let instance = matrix_instance(self.next_matrix)?;
        let section_key = SectionKey::new(SECTION_MATRIX_DATA, instance);
        let entry = self
            .layout
            .entry(section_key)
            .ok_or(EmbeddingError::MissingReference("MATRIX_DATA layout"))?;
        self.output.seek(SeekFrom::Start(entry.offset()))?;

        let mut section_hasher = Sha256::new();
        let mut matrix_hasher = matrix_content_hasher(&commitment)?;
        let mut target_set_hasher = target_set_hasher(commitment.row_count);
        let mut projections = projection_hashers(&commitment)?;
        let row_capacity = usize::try_from(
            u64::from(commitment.stored_dimension)
                .checked_mul(u64::from(T::WIDTH))
                .ok_or(EmbeddingError::ArithmeticOverflow("matrix row byte length"))?,
        )
        .map_err(|_| EmbeddingError::ArithmeticOverflow("matrix row allocation"))?;
        let mut row_bytes = Vec::with_capacity(row_capacity);
        let mut previous_target = None;
        let mut observed_rows = 0u64;

        for (target_id, values) in rows {
            if observed_rows >= commitment.row_count {
                return Err(EmbeddingError::CountLimit {
                    field: "matrix row",
                    value: observed_rows.saturating_add(1),
                }
                .into());
            }
            if previous_target.is_some_and(|previous| target_id <= previous) {
                return Err(EmbeddingError::NonCanonicalOrder("matrix target rows").into());
            }
            previous_target = Some(target_id);
            target_set_hasher.field(target_id.as_bytes());

            let values = values.as_ref();
            validate_row(values, commitment.stored_dimension, observed_rows)?;
            row_bytes.clear();
            append_raw_row(values, &mut row_bytes);
            debug_assert_eq!(row_bytes.len(), row_capacity);
            self.output.write_all(&row_bytes)?;
            section_hasher.update(&row_bytes);
            matrix_hasher.update(&row_bytes);
            update_projection_hashers(&mut projections, values, observed_rows)?;
            observed_rows += 1;
        }
        if observed_rows != commitment.row_count {
            return Err(EmbeddingError::Malformed("matrix row count mismatch").into());
        }

        let actual_target_set = TargetSetId::from_raw(target_set_hasher.finish());
        check_digest(
            DigestKind::TargetSet,
            commitment.target_set_id.as_bytes(),
            actual_target_set.as_bytes(),
        )?;
        let actual_content = MatrixContentDigest::from_raw(matrix_hasher.finish());
        check_digest(
            DigestKind::Matrix,
            commitment.content_digest.as_bytes(),
            actual_content.as_bytes(),
        )?;
        let actual_matrix_id =
            derive_matrix_id(actual_target_set, commitment.family_id, actual_content);
        check_digest(
            DigestKind::Matrix,
            commitment.matrix_id.as_bytes(),
            actual_matrix_id.as_bytes(),
        )?;
        verify_projection_hashers(&commitment, projections, actual_matrix_id)?;
        self.layout
            .set_section_digest(section_key, section_hasher.finalize().into())?;
        self.next_matrix += 1;
        Ok(())
    }
}

fn prepare_matrix<T: MatrixScalar>(
    mut input: MatrixInput<T>,
) -> Result<PreparedMatrix, EmbeddingError> {
    if input.stored_dimension == 0 {
        return Err(EmbeddingError::Malformed("zero stored matrix dimension"));
    }
    if input.rows.is_empty() {
        return Err(EmbeddingError::Malformed("zero matrix row count"));
    }
    input.rows.sort_unstable_by_key(|row| row.target_id);
    if input
        .rows
        .windows(2)
        .any(|pair| pair[0].target_id == pair[1].target_id)
    {
        return Err(EmbeddingError::Duplicate("matrix target row"));
    }
    let target_ids = input
        .rows
        .iter()
        .map(|row| row.target_id)
        .collect::<Vec<_>>();
    let target_set_id = derive_target_set_id(&target_ids);
    check_digest(
        DigestKind::TargetSet,
        input.target_set_id.as_bytes(),
        target_set_id.as_bytes(),
    )?;

    let projections =
        canonical_projection_specs(input.family_id, input.stored_dimension, input.projections)?;
    let row_count = u64::try_from(input.rows.len())
        .map_err(|_| EmbeddingError::ArithmeticOverflow("matrix row count"))?;
    let data_length = checked_matrix_byte_length(row_count, input.stored_dimension, T::DTYPE)?;
    let capacity = usize::try_from(data_length)
        .map_err(|_| EmbeddingError::ArithmeticOverflow("matrix allocation"))?;
    let mut matrix_bytes = Vec::with_capacity(capacity);
    let mut projection_states =
        projection_hashers_from_specs::<T>(row_count, input.stored_dimension, &projections)?;

    for (row_index, row) in input.rows.iter().enumerate() {
        let row_index = u64::try_from(row_index)
            .map_err(|_| EmbeddingError::ArithmeticOverflow("matrix row index"))?;
        validate_row(&row.values, input.stored_dimension, row_index)?;
        append_raw_row(&row.values, &mut matrix_bytes);
        update_projection_hashers(&mut projection_states, &row.values, row_index)?;
    }
    debug_assert_eq!(matrix_bytes.len(), capacity);

    let content_digest = derive_matrix_content_digest(
        T::DTYPE.code(),
        row_count,
        input.stored_dimension,
        &matrix_bytes,
    );
    let matrix_id = derive_matrix_id(target_set_id, input.family_id, content_digest);
    let projection_commitments = projection_states
        .into_iter()
        .map(|state| {
            let content_digest = ProjectionContentDigest::from_raw(state.hasher.finish());
            ProjectionCommitment {
                projection_id: derive_projection_id(
                    matrix_id,
                    state.spec.vector_space_id,
                    content_digest,
                ),
                content_digest,
                vector_space_id: state.spec.vector_space_id,
                effective_dimension: state.spec.effective_dimension,
                postprocessing: state.spec.postprocessing,
            }
        })
        .collect();
    Ok(PreparedMatrix {
        commitment: MatrixCommitment {
            matrix_id,
            content_digest,
            target_set_id,
            family_id: input.family_id,
            dtype: T::DTYPE,
            row_count,
            stored_dimension: input.stored_dimension,
            projections: projection_commitments,
        },
        bytes: matrix_bytes,
    })
}

fn canonical_projection_specs(
    family_id: FamilyId,
    stored_dimension: u32,
    mut projections: Vec<ProjectionSpec>,
) -> Result<Vec<ProjectionSpec>, EmbeddingError> {
    if projections.is_empty() {
        return Err(EmbeddingError::Missing("matrix projection"));
    }
    projections.sort_unstable_by_key(|projection| projection.effective_dimension);
    let mut previous = 0;
    for projection in &projections {
        if projection.effective_dimension == 0
            || projection.effective_dimension <= previous
            || projection.effective_dimension > stored_dimension
        {
            return Err(EmbeddingError::NonCanonicalOrder(
                "matrix projection dimensions",
            ));
        }
        let expected = derive_vector_space_id(
            family_id,
            projection.effective_dimension,
            projection.postprocessing.code(),
        );
        check_digest(
            DigestKind::Contract,
            projection.vector_space_id.as_bytes(),
            expected.as_bytes(),
        )?;
        previous = projection.effective_dimension;
    }
    if previous != stored_dimension {
        return Err(EmbeddingError::Missing("stored-dimension projection"));
    }
    Ok(projections)
}

fn validate_prepared_matrix_order(matrices: &[PreparedMatrix]) -> Result<(), EmbeddingError> {
    for pair in matrices.windows(2) {
        let left = &pair[0].commitment;
        let right = &pair[1].commitment;
        if left.matrix_id == right.matrix_id {
            return Err(EmbeddingError::Duplicate("matrix identity"));
        }
        if (left.family_id, left.target_set_id) == (right.family_id, right.target_set_id) {
            return Err(EmbeddingError::Duplicate("family and target-set matrix"));
        }
    }
    let mut pairs = matrices
        .iter()
        .map(|matrix| (matrix.commitment.family_id, matrix.commitment.target_set_id))
        .collect::<Vec<_>>();
    pairs.sort_unstable();
    if pairs.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(EmbeddingError::Duplicate("family and target-set matrix"));
    }
    Ok(())
}

fn validate_commitments(matrices: &[MatrixCommitment]) -> Result<(), EmbeddingError> {
    if matrices.is_empty() {
        return Err(EmbeddingError::Missing("matrix"));
    }
    let mut matrix_pairs = Vec::with_capacity(matrices.len());
    let mut previous_matrix = None;
    for matrix in matrices {
        if previous_matrix.is_some_and(|previous| matrix.matrix_id <= previous) {
            return Err(EmbeddingError::NonCanonicalOrder("matrix identities"));
        }
        previous_matrix = Some(matrix.matrix_id);
        matrix_pairs.push((matrix.family_id, matrix.target_set_id));
        if matrix.row_count == 0 {
            return Err(EmbeddingError::Malformed("zero matrix row count"));
        }
        if matrix.stored_dimension == 0 {
            return Err(EmbeddingError::Malformed("zero stored matrix dimension"));
        }
        matrix.data_length()?;
        let expected_matrix_id = derive_matrix_id(
            matrix.target_set_id,
            matrix.family_id,
            matrix.content_digest,
        );
        check_digest(
            DigestKind::Matrix,
            matrix.matrix_id.as_bytes(),
            expected_matrix_id.as_bytes(),
        )?;
        if matrix.projections.is_empty() {
            return Err(EmbeddingError::Missing("matrix projection"));
        }
        let mut previous_dimension = 0;
        for projection in &matrix.projections {
            if projection.effective_dimension == 0
                || projection.effective_dimension <= previous_dimension
                || projection.effective_dimension > matrix.stored_dimension
            {
                return Err(EmbeddingError::NonCanonicalOrder(
                    "matrix projection dimensions",
                ));
            }
            let expected_space = derive_vector_space_id(
                matrix.family_id,
                projection.effective_dimension,
                projection.postprocessing.code(),
            );
            check_digest(
                DigestKind::Contract,
                projection.vector_space_id.as_bytes(),
                expected_space.as_bytes(),
            )?;
            let expected_projection = derive_projection_id(
                matrix.matrix_id,
                projection.vector_space_id,
                projection.content_digest,
            );
            check_digest(
                DigestKind::Projection,
                projection.projection_id.as_bytes(),
                expected_projection.as_bytes(),
            )?;
            checked_projection_byte_length(
                matrix.row_count,
                projection.effective_dimension,
                matrix.dtype,
            )?;
            previous_dimension = projection.effective_dimension;
        }
        if previous_dimension != matrix.stored_dimension {
            return Err(EmbeddingError::Missing("stored-dimension projection"));
        }
    }
    matrix_pairs.sort_unstable();
    if matrix_pairs.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(EmbeddingError::Duplicate("family and target-set matrix"));
    }
    Ok(())
}

fn encode_matrices_section(matrices: &[MatrixCommitment]) -> Result<Vec<u8>, EmbeddingError> {
    validate_commitments(matrices)?;
    let matrix_count = u64::try_from(matrices.len())
        .map_err(|_| EmbeddingError::ArithmeticOverflow("matrix count"))?;
    let projection_count = matrices.iter().try_fold(0u64, |count, matrix| {
        count
            .checked_add(
                u64::try_from(matrix.projections.len())
                    .map_err(|_| EmbeddingError::ArithmeticOverflow("projection count"))?,
            )
            .ok_or(EmbeddingError::ArithmeticOverflow("projection count"))
    })?;
    let matrix_records_length = matrix_count
        .checked_mul(MATRIX_RECORD_LENGTH)
        .ok_or(EmbeddingError::ArithmeticOverflow("matrix records length"))?;
    let projection_records_offset = MATRICES_HEADER_LENGTH
        .checked_add(matrix_records_length)
        .ok_or(EmbeddingError::ArithmeticOverflow(
            "projection records offset",
        ))?;
    let projection_records_length = projection_count
        .checked_mul(PROJECTION_RECORD_LENGTH)
        .ok_or(EmbeddingError::ArithmeticOverflow(
            "projection records length",
        ))?;
    let section_length = projection_records_offset
        .checked_add(projection_records_length)
        .ok_or(EmbeddingError::ArithmeticOverflow(
            "MATRICES section length",
        ))?;
    let section_length = usize::try_from(section_length)
        .map_err(|_| EmbeddingError::ArithmeticOverflow("MATRICES allocation"))?;
    let mut output = vec![0u8; section_length];

    put_u32(&mut output, 0, 1);
    put_u32(
        &mut output,
        4,
        u32::try_from(MATRIX_RECORD_LENGTH).expect("fixed record length fits u32"),
    );
    put_u64(&mut output, 8, matrix_count);
    put_u64(&mut output, 16, MATRICES_HEADER_LENGTH);
    put_u64(&mut output, 24, matrix_records_length);
    put_u32(
        &mut output,
        32,
        u32::try_from(PROJECTION_RECORD_LENGTH).expect("fixed record length fits u32"),
    );
    put_u32(&mut output, 36, 0);
    put_u64(&mut output, 40, projection_count);
    put_u64(&mut output, 48, projection_records_offset);
    put_u64(&mut output, 56, projection_records_length);

    let mut projection_index = 0u64;
    for (matrix_index, matrix) in matrices.iter().enumerate() {
        let matrix_index = u64::try_from(matrix_index)
            .map_err(|_| EmbeddingError::ArithmeticOverflow("matrix record index"))?;
        let record_offset = MATRICES_HEADER_LENGTH
            .checked_add(
                matrix_index
                    .checked_mul(MATRIX_RECORD_LENGTH)
                    .ok_or(EmbeddingError::ArithmeticOverflow("matrix record offset"))?,
            )
            .ok_or(EmbeddingError::ArithmeticOverflow("matrix record offset"))?;
        let record_offset = usize::try_from(record_offset)
            .map_err(|_| EmbeddingError::ArithmeticOverflow("matrix record offset"))?;
        output[record_offset..record_offset + 32].copy_from_slice(matrix.matrix_id.as_bytes());
        output[record_offset + 32..record_offset + 64]
            .copy_from_slice(matrix.content_digest.as_bytes());
        output[record_offset + 64..record_offset + 96]
            .copy_from_slice(matrix.target_set_id.as_bytes());
        output[record_offset + 96..record_offset + 128]
            .copy_from_slice(matrix.family_id.as_bytes());
        put_u32(
            &mut output,
            record_offset + 128,
            matrix_instance(usize::try_from(matrix_index).expect("source index was usize"))?,
        );
        put_u32(&mut output, record_offset + 132, matrix.dtype.code());
        put_u64(&mut output, record_offset + 136, matrix.row_count);
        put_u32(&mut output, record_offset + 144, matrix.stored_dimension);
        put_u64(&mut output, record_offset + 152, matrix.data_length()?);

        for projection in &matrix.projections {
            let projection_offset = projection_records_offset
                .checked_add(
                    projection_index
                        .checked_mul(PROJECTION_RECORD_LENGTH)
                        .ok_or(EmbeddingError::ArithmeticOverflow(
                            "projection record offset",
                        ))?,
                )
                .ok_or(EmbeddingError::ArithmeticOverflow(
                    "projection record offset",
                ))?;
            let projection_offset = usize::try_from(projection_offset)
                .map_err(|_| EmbeddingError::ArithmeticOverflow("projection record offset"))?;
            output[projection_offset..projection_offset + 32]
                .copy_from_slice(projection.projection_id.as_bytes());
            output[projection_offset + 32..projection_offset + 64]
                .copy_from_slice(projection.content_digest.as_bytes());
            output[projection_offset + 64..projection_offset + 96]
                .copy_from_slice(matrix.matrix_id.as_bytes());
            output[projection_offset + 96..projection_offset + 128]
                .copy_from_slice(projection.vector_space_id.as_bytes());
            put_u32(
                &mut output,
                projection_offset + 128,
                projection.effective_dimension,
            );
            put_u32(
                &mut output,
                projection_offset + 132,
                projection.postprocessing.code(),
            );
            put_u64(&mut output, projection_offset + 136, matrix.row_count);
            put_u64(
                &mut output,
                projection_offset + 144,
                checked_projection_byte_length(
                    matrix.row_count,
                    projection.effective_dimension,
                    matrix.dtype,
                )?,
            );
            projection_index += 1;
        }
    }
    debug_assert_eq!(projection_index, projection_count);
    Ok(output)
}

fn write_stream_skeleton<W: Write + Seek>(
    output: &mut W,
    layout: &FileLayout,
    payloads: &[SectionPayload],
) -> Result<(), EmbeddingWriteError> {
    output.seek(SeekFrom::Start(0))?;
    output.write_all(&layout.header_zero_root())?;
    let directory_length = u64::try_from(layout.entries().len())
        .map_err(|_| EmbeddingError::ArithmeticOverflow("directory count"))?
        .checked_mul(PURREMB_DIRECTORY_ENTRY_LENGTH)
        .ok_or(EmbeddingError::ArithmeticOverflow("directory length"))?;
    write_zeros(output, directory_length)?;
    let after_directory = u64::from(PURREMB_HEADER_LENGTH)
        .checked_add(directory_length)
        .ok_or(EmbeddingError::ArithmeticOverflow("directory end"))?;
    write_zeros(
        output,
        layout
            .first_section_offset()
            .checked_sub(after_directory)
            .ok_or(EmbeddingError::ArithmeticOverflow("directory padding"))?,
    )?;

    for (index, entry) in layout.entries().iter().enumerate() {
        let current = output.stream_position()?;
        if current != entry.offset() {
            return Err(EmbeddingError::InvalidSpan {
                context: "stream skeleton section",
                offset: current,
                length: entry.length(),
            }
            .into());
        }
        if entry.key().kind == SECTION_MATRIX_DATA {
            let section_end = entry
                .offset()
                .checked_add(entry.length())
                .ok_or(EmbeddingError::ArithmeticOverflow("matrix section end"))?;
            output.seek(SeekFrom::Start(section_end))?;
        } else {
            let payload = payloads
                .binary_search_by_key(&entry.key(), |payload| payload.key)
                .ok()
                .map(|payload_index| &payloads[payload_index])
                .ok_or(EmbeddingError::MissingReference("stream section payload"))?;
            if u64::try_from(payload.bytes.len())
                .map_err(|_| EmbeddingError::ArithmeticOverflow("section length"))?
                != entry.length()
            {
                return Err(EmbeddingError::Malformed(
                    "stream section length disagrees with layout",
                )
                .into());
            }
            output.write_all(&payload.bytes)?;
        }
        let next_offset = layout
            .entries()
            .get(index + 1)
            .map_or_else(|| layout.trailer_offset(), |next| next.offset());
        let section_end = entry
            .offset()
            .checked_add(entry.length())
            .ok_or(EmbeddingError::ArithmeticOverflow("section end"))?;
        write_zeros(
            output,
            next_offset
                .checked_sub(section_end)
                .ok_or(EmbeddingError::ArithmeticOverflow("section padding"))?,
        )?;
    }
    output.write_all(&[0u8; PURREMB_TRAILER_LENGTH as usize])?;
    if output.stream_position()? != layout.file_length() {
        return Err(EmbeddingError::Malformed("stream skeleton file length mismatch").into());
    }
    Ok(())
}

fn write_zeros<W: Write>(output: &mut W, mut length: u64) -> std::io::Result<()> {
    const ZEROS: [u8; 4096] = [0; 4096];
    while length != 0 {
        let count = usize::try_from(length.min(ZEROS.len() as u64)).expect("bounded by 4096");
        output.write_all(&ZEROS[..count])?;
        length -= u64::try_from(count).expect("a small buffer length fits u64");
    }
    Ok(())
}

fn write_at<W: Write + Seek>(output: &mut W, offset: u64, bytes: &[u8]) -> std::io::Result<()> {
    output.seek(SeekFrom::Start(offset))?;
    output.write_all(bytes)
}

fn checked_matrix_byte_length(
    row_count: u64,
    stored_dimension: u32,
    dtype: VectorDtype,
) -> Result<u64, EmbeddingError> {
    checked_projection_byte_length(row_count, stored_dimension, dtype)
}

fn checked_projection_byte_length(
    row_count: u64,
    dimension: u32,
    dtype: VectorDtype,
) -> Result<u64, EmbeddingError> {
    row_count
        .checked_mul(u64::from(dimension))
        .and_then(|scalars| scalars.checked_mul(u64::from(dtype.width())))
        .ok_or(EmbeddingError::ArithmeticOverflow("matrix byte length"))
}

fn matrix_instance(index: usize) -> Result<u32, EmbeddingError> {
    u32::try_from(index + 1).map_err(|_| EmbeddingError::CountLimit {
        field: "matrix",
        value: u64::try_from(index + 1).unwrap_or(u64::MAX),
    })
}

struct FramedHasher {
    hasher: Sha256,
}

impl FramedHasher {
    fn new(domain: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(domain);
        Self { hasher }
    }

    fn field(&mut self, bytes: &[u8]) {
        let length = u64::try_from(bytes.len()).expect("an in-memory slice length fits u64");
        self.begin_field(length);
        self.update(bytes);
    }

    fn begin_field(&mut self, length: u64) {
        self.hasher.update(length.to_le_bytes());
    }

    fn update(&mut self, bytes: &[u8]) {
        self.hasher.update(bytes);
    }

    fn finish(self) -> [u8; 32] {
        self.hasher.finalize().into()
    }
}

fn target_set_hasher(row_count: u64) -> FramedHasher {
    let mut hasher = FramedHasher::new(D_TARGET_SET);
    hasher.field(&row_count.to_le_bytes());
    hasher
}

fn matrix_content_hasher(commitment: &MatrixCommitment) -> Result<FramedHasher, EmbeddingError> {
    let mut hasher = FramedHasher::new(D_MATRIX_CONTENT);
    hasher.field(&commitment.dtype.code().to_le_bytes());
    hasher.field(&commitment.row_count.to_le_bytes());
    hasher.field(&commitment.stored_dimension.to_le_bytes());
    hasher.begin_field(commitment.data_length()?);
    Ok(hasher)
}

struct ProjectionHashState {
    spec: ProjectionSpec,
    hasher: FramedHasher,
}

fn projection_hashers(
    commitment: &MatrixCommitment,
) -> Result<Vec<ProjectionHashState>, EmbeddingError> {
    let specs = commitment
        .projections
        .iter()
        .map(|projection| ProjectionSpec {
            vector_space_id: projection.vector_space_id,
            effective_dimension: projection.effective_dimension,
            postprocessing: projection.postprocessing,
        })
        .collect::<Vec<_>>();
    match commitment.dtype {
        VectorDtype::F32 => projection_hashers_from_specs::<f32>(
            commitment.row_count,
            commitment.stored_dimension,
            &specs,
        ),
        VectorDtype::F64 => projection_hashers_from_specs::<f64>(
            commitment.row_count,
            commitment.stored_dimension,
            &specs,
        ),
    }
}

fn projection_hashers_from_specs<T: MatrixScalar>(
    row_count: u64,
    stored_dimension: u32,
    specs: &[ProjectionSpec],
) -> Result<Vec<ProjectionHashState>, EmbeddingError> {
    let mut states = Vec::with_capacity(specs.len());
    for spec in specs {
        let mut hasher = FramedHasher::new(D_PROJECTION_CONTENT);
        hasher.field(&T::DTYPE.code().to_le_bytes());
        hasher.field(&row_count.to_le_bytes());
        hasher.field(&spec.effective_dimension.to_le_bytes());
        hasher.field(&spec.postprocessing.code().to_le_bytes());
        hasher.begin_field(checked_projection_byte_length(
            row_count,
            spec.effective_dimension,
            T::DTYPE,
        )?);
        states.push(ProjectionHashState {
            spec: *spec,
            hasher,
        });
    }
    if specs.last().map(|spec| spec.effective_dimension) != Some(stored_dimension) {
        return Err(EmbeddingError::Missing("stored-dimension projection"));
    }
    Ok(states)
}

fn update_projection_hashers<T: MatrixScalar>(
    states: &mut [ProjectionHashState],
    values: &[T],
    row: u64,
) -> Result<(), EmbeddingError> {
    for state in states {
        let dimension = usize::try_from(state.spec.effective_dimension)
            .map_err(|_| EmbeddingError::ArithmeticOverflow("projection dimension"))?;
        let prefix = values
            .get(..dimension)
            .ok_or_else(|| EmbeddingError::InvalidSpan {
                context: "matrix projection row",
                offset: 0,
                length: u64::from(state.spec.effective_dimension),
            })?;
        match state.spec.postprocessing {
            PrefixPostprocessing::None => {
                for &value in prefix {
                    let bytes = value.raw_bytes();
                    state.hasher.update(bytes.as_slice());
                }
            }
            PrefixPostprocessing::DeterministicL2 => {
                let norm = deterministic_l2_norm(prefix, row, state.spec.effective_dimension)?;
                for &value in prefix {
                    let bytes = T::rounded_bytes(value.to_f64() / norm);
                    state.hasher.update(bytes.as_slice());
                }
            }
        }
    }
    Ok(())
}

fn verify_projection_hashers(
    commitment: &MatrixCommitment,
    states: Vec<ProjectionHashState>,
    matrix_id: MatrixId,
) -> Result<(), EmbeddingError> {
    if states.len() != commitment.projections.len() {
        return Err(EmbeddingError::Malformed(
            "projection digest state count mismatch",
        ));
    }
    for (state, expected) in states.into_iter().zip(&commitment.projections) {
        let actual_content = ProjectionContentDigest::from_raw(state.hasher.finish());
        check_digest(
            DigestKind::Projection,
            expected.content_digest.as_bytes(),
            actual_content.as_bytes(),
        )?;
        let actual_id = derive_projection_id(matrix_id, expected.vector_space_id, actual_content);
        check_digest(
            DigestKind::Projection,
            expected.projection_id.as_bytes(),
            actual_id.as_bytes(),
        )?;
    }
    Ok(())
}

fn deterministic_l2_norm<T: MatrixScalar>(
    values: &[T],
    row: u64,
    dimension: u32,
) -> Result<f64, EmbeddingError> {
    let mut scale = 0.0f64;
    let mut ssq = 1.0f64;
    for &value in values {
        let absolute = value.to_f64().abs();
        if absolute != 0.0 {
            if scale < absolute {
                let ratio = scale / absolute;
                let square = ratio * ratio;
                let product = ssq * square;
                ssq = 1.0 + product;
                scale = absolute;
            } else {
                let ratio = absolute / scale;
                let square = ratio * ratio;
                ssq += square;
            }
        }
    }
    if scale == 0.0 {
        return Err(EmbeddingError::ZeroNorm { row, dimension });
    }
    let norm = scale * ssq.sqrt();
    if !norm.is_finite() || norm == 0.0 {
        return Err(EmbeddingError::Malformed("invalid deterministic L2 norm"));
    }
    Ok(norm)
}

fn validate_row<T: MatrixScalar>(
    values: &[T],
    stored_dimension: u32,
    row: u64,
) -> Result<(), EmbeddingError> {
    if values.len()
        != usize::try_from(stored_dimension)
            .map_err(|_| EmbeddingError::ArithmeticOverflow("stored dimension"))?
    {
        return Err(EmbeddingError::Malformed("matrix row width mismatch"));
    }
    for (column, value) in values.iter().enumerate() {
        if !value.is_finite() {
            return Err(EmbeddingError::NonFiniteScalar {
                row,
                column: u32::try_from(column)
                    .map_err(|_| EmbeddingError::ArithmeticOverflow("matrix column"))?,
            });
        }
    }
    Ok(())
}

fn append_raw_row<T: MatrixScalar>(values: &[T], output: &mut Vec<u8>) {
    for &value in values {
        let bytes = value.raw_bytes();
        output.extend_from_slice(bytes.as_slice());
    }
}

struct ScalarBytes {
    bytes: [u8; 8],
    length: usize,
}

impl ScalarBytes {
    fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.length]
    }
}

trait MatrixScalar: Copy {
    const DTYPE: VectorDtype;
    const WIDTH: u32;

    fn is_finite(self) -> bool;
    fn to_f64(self) -> f64;
    fn raw_bytes(self) -> ScalarBytes;
    fn rounded_bytes(value: f64) -> ScalarBytes;
}

impl MatrixScalar for f32 {
    const DTYPE: VectorDtype = VectorDtype::F32;
    const WIDTH: u32 = 4;

    fn is_finite(self) -> bool {
        self.is_finite()
    }

    fn to_f64(self) -> f64 {
        f64::from(self)
    }

    fn raw_bytes(self) -> ScalarBytes {
        let mut bytes = [0u8; 8];
        bytes[..4].copy_from_slice(&self.to_le_bytes());
        ScalarBytes { bytes, length: 4 }
    }

    fn rounded_bytes(value: f64) -> ScalarBytes {
        (value as Self).raw_bytes()
    }
}

impl MatrixScalar for f64 {
    const DTYPE: VectorDtype = VectorDtype::F64;
    const WIDTH: u32 = 8;

    fn is_finite(self) -> bool {
        self.is_finite()
    }

    fn to_f64(self) -> f64 {
        self
    }

    fn raw_bytes(self) -> ScalarBytes {
        ScalarBytes {
            bytes: self.to_le_bytes(),
            length: 8,
        }
    }

    fn rounded_bytes(value: f64) -> ScalarBytes {
        value.raw_bytes()
    }
}

fn check_digest(
    kind: DigestKind,
    expected: &[u8; 32],
    actual: &[u8; 32],
) -> Result<(), EmbeddingError> {
    if expected != actual {
        return Err(EmbeddingError::DigestMismatch {
            kind,
            expected: *expected,
            actual: *actual,
        });
    }
    Ok(())
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn metadata() -> CanonicalMetadataSections {
        let source_exact_digest = ContentDigest::of(b"source-pack");
        let mut source = vec![0u8; 128];
        source[24..56].copy_from_slice(source_exact_digest.as_bytes());
        CanonicalMetadataSections {
            source_exact_digest,
            source,
            contracts: vec![2],
            targets: vec![3],
            target_sets: vec![4],
            relations: vec![5],
            token_spans: vec![6],
            external_bindings: vec![8],
            index_guards: vec![9],
            inline_index_payloads: Vec::new(),
            extensions: Vec::new(),
        }
    }

    fn f32_input() -> MatrixInput<f32> {
        let family_id = FamilyId::from_raw([7; 32]);
        let first = TargetId::from_raw([1; 32]);
        let second = TargetId::from_raw([2; 32]);
        MatrixInput {
            family_id,
            target_set_id: derive_target_set_id(&[first, second]),
            stored_dimension: 3,
            rows: vec![
                MatrixRow::new(second, vec![3.0, 4.0, -0.0]),
                MatrixRow::new(first, vec![1.0, 2.0, 2.0]),
            ],
            projections: vec![
                ProjectionSpec::derive(family_id, 3, PrefixPostprocessing::DeterministicL2),
                ProjectionSpec::derive(family_id, 2, PrefixPostprocessing::None),
            ],
        }
    }

    #[test]
    fn unordered_and_streaming_writers_emit_identical_bytes() {
        let prepared = prepare_matrix(f32_input()).unwrap();
        let commitment = prepared.commitment;

        let mut builder = EmbeddingBuilder::new(metadata());
        builder.add_f32_matrix(f32_input());
        let in_memory = builder.build().unwrap();

        let cursor = Cursor::new(Vec::new());
        let mut stream = EmbeddingStreamWriter::new(cursor, metadata(), vec![commitment]).unwrap();
        let first = TargetId::from_raw([1; 32]);
        let second = TargetId::from_raw([2; 32]);
        stream
            .write_f32_matrix([(first, [1.0, 2.0, 2.0]), (second, [3.0, 4.0, -0.0])])
            .unwrap();
        let (cursor, root) = stream.finish().unwrap();
        assert_eq!(root, in_memory.root);
        assert_eq!(cursor.into_inner(), in_memory.bytes);
    }

    #[test]
    fn matrix_content_identity_matches_the_normative_helper() {
        let prepared = prepare_matrix(f32_input()).unwrap();
        let expected = derive_matrix_content_digest(1, 2, 3, &prepared.bytes);
        assert_eq!(prepared.commitment.content_digest, expected);
        assert_eq!(
            prepared.commitment.matrix_id,
            derive_matrix_id(
                prepared.commitment.target_set_id,
                prepared.commitment.family_id,
                expected,
            )
        );
    }

    #[test]
    fn streaming_rejects_out_of_order_targets() {
        let prepared = prepare_matrix(f32_input()).unwrap();
        let cursor = Cursor::new(Vec::new());
        let mut stream =
            EmbeddingStreamWriter::new(cursor, metadata(), vec![prepared.commitment]).unwrap();
        let first = TargetId::from_raw([1; 32]);
        let second = TargetId::from_raw([2; 32]);
        let error = stream
            .write_f32_matrix([(second, [3.0, 4.0, -0.0]), (first, [1.0, 2.0, 2.0])])
            .unwrap_err();
        assert!(matches!(
            error,
            EmbeddingWriteError::Format(EmbeddingError::NonCanonicalOrder(_))
        ));
    }

    #[test]
    fn deterministic_l2_rejects_a_zero_prefix() {
        let error = deterministic_l2_norm(&[0.0f64, -0.0], 4, 2).unwrap_err();
        assert_eq!(
            error,
            EmbeddingError::ZeroNorm {
                row: 4,
                dimension: 2
            }
        );
    }

    #[test]
    fn f64_matrix_bits_are_preserved() {
        let family_id = FamilyId::from_raw([4; 32]);
        let target = TargetId::from_raw([8; 32]);
        let input = MatrixInput {
            family_id,
            target_set_id: derive_target_set_id(&[target]),
            stored_dimension: 2,
            rows: vec![MatrixRow::new(target, vec![-0.0f64, f64::MIN_POSITIVE])],
            projections: vec![ProjectionSpec::derive(
                family_id,
                2,
                PrefixPostprocessing::None,
            )],
        };
        let prepared = prepare_matrix(input).unwrap();
        assert_eq!(&prepared.bytes[..8], &(-0.0f64).to_le_bytes());
        assert_eq!(&prepared.bytes[8..], &f64::MIN_POSITIVE.to_le_bytes());
    }

    #[test]
    fn nonfinite_scalars_are_rejected_with_coordinates() {
        let mut input = f32_input();
        input.rows[0].values[1] = f32::INFINITY;
        let error = prepare_matrix(input).unwrap_err();
        assert!(matches!(error, EmbeddingError::NonFiniteScalar { .. }));
    }

    #[test]
    fn projection_digest_matches_normative_raw_helper() {
        let family_id = FamilyId::from_raw([5; 32]);
        let target = TargetId::from_raw([6; 32]);
        let input = MatrixInput {
            family_id,
            target_set_id: derive_target_set_id(&[target]),
            stored_dimension: 2,
            rows: vec![MatrixRow::new(target, vec![1.5f32, -0.0])],
            projections: vec![ProjectionSpec::derive(
                family_id,
                2,
                PrefixPostprocessing::None,
            )],
        };
        let prepared = prepare_matrix(input).unwrap();
        let expected =
            super::super::identity::derive_projection_content_digest(1, 1, 2, 0, &prepared.bytes);
        assert_eq!(prepared.commitment.projections[0].content_digest, expected);
    }
}
