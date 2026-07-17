// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Typed canonical encoders for PURREMB metadata sections.

use core::cmp::Ordering;

use crate::{ContentDigest, PackBuilder, RdfDataset, verify_pack};

use super::contract::{
    ArtifactIdentity, ArtifactIdentityKind, EmbeddingFamily, EmbeddingFamilyContract, TlvWireType,
    push_tlv,
};
use super::error::{DigestKind, EmbeddingError};
use super::identity::{
    ExternalBindingId, ExternalBindingIdentity, ExternalContractDigest, FamilyId, IndexGuardDigest,
    IndexId, IndexIdentity, MatrixId, ProjectionId, RdfcDigest, TargetId, TargetSetId,
    VectorSpaceId, derive_external_binding_id, derive_external_contract_digest,
    derive_index_guard_digest, derive_index_id,
};
use super::target::{
    EmbeddingTarget, RdfDatasetTarget, RelationKind, TargetKind, TargetRelation, TargetSet,
    TokenSpan,
};
use super::wire::checked_align_up;
use super::writer::{
    CanonicalMetadataSections, ExtensionSection, MatrixCommitment, ProjectionCommitment,
};

const SOURCE_LENGTH: usize = 128;
const CONTRACTS_HEADER_LENGTH: u64 = 96;
const FAMILY_RECORD_LENGTH: u64 = 96;
const SPACE_RECORD_LENGTH: u64 = 80;
const TARGETS_HEADER_LENGTH: u64 = 64;
const TARGET_RECORD_LENGTH: u64 = 96;
const TARGET_SETS_HEADER_LENGTH: u64 = 64;
const TARGET_SET_RECORD_LENGTH: u64 = 64;
const RELATIONS_HEADER_LENGTH: u64 = 64;
const RELATION_RECORD_LENGTH: u64 = 120;
const TOKEN_SPANS_HEADER_LENGTH: u64 = 64;
const TOKEN_SPAN_RECORD_LENGTH: u64 = 96;
const EXTERNAL_BINDINGS_HEADER_LENGTH: u64 = 64;
const EXTERNAL_BINDING_RECORD_LENGTH: u64 = 192;
const INDEX_GUARDS_HEADER_LENGTH: u64 = 64;
const INDEX_GUARD_RECORD_LENGTH: u64 = 336;

const IDENTITY_BYTES_PRESENT: u32 = 1;
const SOURCE_ORDINAL_PRESENT: u32 = 1 << 1;
const RELATION_ROLE_BYTES_PRESENT: u32 = 1;
const CERTIFIED_RDF_PRESENT: u32 = 1;
const INDEX_REBUILDABLE: u32 = 1;

/// Independently certified attachment to one exact `.purrpck` byte string.
///
/// The fields are private so the public construction path cannot turn an
/// arbitrary digest claim into certified source metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertifiedPurrpckSource {
    source_length: u64,
    source_exact_digest: ContentDigest,
    certified_rdf_digest: RdfcDigest,
    dataset_target_id: TargetId,
}

impl CertifiedPurrpckSource {
    /// Structurally opens and independently RDFC-certifies exact `.purrpck`
    /// bytes, then derives their RDF-dataset target.
    pub fn certify(source_bytes: &[u8]) -> Result<Self, EmbeddingError> {
        let certified = verify_pack(source_bytes)
            .map_err(|error| EmbeddingError::InvalidSourcePack(error.to_string()))?;
        let certified_rdf_digest = RdfcDigest::from_raw(*certified.as_bytes());
        let dataset_target = RdfDatasetTarget {
            rdfc_digest: *certified.as_bytes(),
        }
        .into_target(false)?;
        Ok(Self {
            source_length: u64::try_from(source_bytes.len())
                .map_err(|_| EmbeddingError::ArithmeticOverflow("source byte length"))?,
            source_exact_digest: ContentDigest::of(source_bytes),
            certified_rdf_digest,
            dataset_target_id: dataset_target.id,
        })
    }

    /// Builds and certifies a `.purrpck` directly from a frozen RDF dataset.
    pub fn from_dataset(dataset: &RdfDataset) -> Result<(Self, Vec<u8>), EmbeddingError> {
        let bytes = PackBuilder::build_bytes(dataset)
            .map_err(|error| EmbeddingError::InvalidSourcePack(error.to_string()))?;
        let source = Self::certify(&bytes)?;
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
        put_u32(&mut output, 16, 1);
        output[24..56].copy_from_slice(self.source_exact_digest.as_bytes());
        output[56..88].copy_from_slice(self.certified_rdf_digest.as_bytes());
        output[88..120].copy_from_slice(self.dataset_target_id.as_bytes());
        output
    }
}

/// Complete typed input for the eight non-matrix PURREMB metadata sections.
#[derive(Debug, Clone)]
pub struct CanonicalMetadataInput {
    /// Exact source attachment created through independent `.purrpck` certification.
    pub source: CertifiedPurrpckSource,
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
    /// Encodes metadata that carries no matrix- or index-dependent reference.
    ///
    /// Use [`Self::encode_for_matrices`] when indexes or external bindings name
    /// a matrix, projection, or index.
    pub fn encode(self) -> Result<CanonicalMetadataSections, EmbeddingError> {
        self.encode_inner(None)
    }

    /// Encodes metadata and validates every matrix, projection, binding, and
    /// index cross-reference against precommitted matrices.
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
        let EncodedContracts {
            bytes: contracts,
            families,
        } = encode_contracts(self.family_contracts)?;
        let canonical_targets = canonical_targets(self.targets)?;
        validate_source_target(self.source, &canonical_targets)?;
        let targets = encode_targets_canonical(&canonical_targets)?;
        let canonical_sets = canonical_target_sets(self.target_sets)?;
        validate_target_set_references(&canonical_sets, &canonical_targets)?;
        let target_sets = encode_target_sets_canonical(&canonical_sets)?;
        let canonical_relations = canonical_relations(self.relations)?;
        validate_relation_references(&canonical_relations, &canonical_targets)?;
        let relations = encode_relations_canonical(&canonical_relations)?;
        let canonical_spans = canonical_token_spans(self.token_spans)?;
        validate_token_span_references(&canonical_spans, &families, &canonical_targets)?;
        let token_spans = encode_token_spans_canonical(&canonical_spans)?;

        let canonical_bindings = canonical_external_bindings(self.external_bindings)?;
        let canonical_indexes = canonical_indexes(self.indexes)?;
        validate_matrix_and_index_references(
            self.source,
            &families,
            &canonical_targets,
            &canonical_sets,
            &canonical_bindings,
            &canonical_indexes,
            matrices,
        )?;
        let external_bindings = encode_external_bindings_canonical(&canonical_bindings)?;
        let (index_guards, inline_index_payloads) =
            encode_index_guards_canonical(&canonical_indexes)?;

        Ok(CanonicalMetadataSections {
            source_exact_digest: self.source.source_exact_digest(),
            source: self.source.encode().to_vec(),
            contracts,
            targets,
            target_sets,
            relations,
            token_spans,
            external_bindings,
            index_guards,
            inline_index_payloads,
            extensions: self.extensions,
        })
    }
}

struct EncodedContracts {
    bytes: Vec<u8>,
    families: Vec<EmbeddingFamily>,
}

/// Canonicalizes generation contracts and encodes the `CONTRACTS` section.
pub fn encode_family_contracts(
    contracts: Vec<EmbeddingFamilyContract>,
) -> Result<Vec<u8>, EmbeddingError> {
    Ok(encode_contracts(contracts)?.bytes)
}

fn encode_contracts(
    contracts: Vec<EmbeddingFamilyContract>,
) -> Result<EncodedContracts, EmbeddingError> {
    let mut families = contracts
        .into_iter()
        .map(|contract| contract.derive())
        .collect::<Result<Vec<_>, _>>()?;
    families.sort_unstable_by_key(|family| family.id);
    let mut canonical: Vec<EmbeddingFamily> = Vec::with_capacity(families.len());
    for family in families {
        if let Some(previous) = canonical.last()
            && previous.id == family.id
        {
            if previous == &family {
                continue;
            }
            return Err(EmbeddingError::Duplicate("contradictory family identity"));
        }
        canonical.push(family);
    }
    if canonical.is_empty() {
        return Err(EmbeddingError::Missing("embedding family"));
    }

    let family_count = u64_len(canonical.len(), "family count")?;
    let space_count = canonical.iter().try_fold(0u64, |count, family| {
        count
            .checked_add(u64_len(family.spaces.len(), "effective space count")?)
            .ok_or(EmbeddingError::ArithmeticOverflow("effective space count"))
    })?;
    let family_records_length = checked_mul(family_count, FAMILY_RECORD_LENGTH, "family table")?;
    let space_records_offset = align8(checked_add(
        CONTRACTS_HEADER_LENGTH,
        family_records_length,
        "space table offset",
    )?)?;
    let space_records_length = checked_mul(space_count, SPACE_RECORD_LENGTH, "space table")?;
    let pool_offset = align8(checked_add(
        space_records_offset,
        space_records_length,
        "contract pool offset",
    )?)?;
    let mut records = zero_vec(family_records_length, "family table allocation")?;
    let mut spaces = zero_vec(space_records_length, "space table allocation")?;
    let mut pool = Vec::new();
    let mut space_start = 0u64;

    for (family_index, family) in canonical.iter().enumerate() {
        let record = record_offset(family_index, FAMILY_RECORD_LENGTH, "family record")?;
        records[record..record + 32].copy_from_slice(family.id.as_bytes());
        records[record + 32..record + 64].copy_from_slice(family.contract_digest.as_bytes());
        let (contract_offset, contract_length) =
            append_pool_block(&mut pool, pool_offset, &family.canonical_contract)?;
        put_u64(&mut records, record + 64, contract_offset);
        put_u64(&mut records, record + 72, contract_length);
        let dimensionality_policy = if family.spaces.len() == 1 { 1 } else { 2 };
        put_u32(&mut records, record + 80, dimensionality_policy);
        put_u32(&mut records, record + 84, family.stored_dimension);
        put_u32(
            &mut records,
            record + 88,
            u32::try_from(space_start).map_err(|_| EmbeddingError::CountLimit {
                field: "effective space",
                value: space_start,
            })?,
        );
        put_u32(
            &mut records,
            record + 92,
            u32::try_from(family.spaces.len()).map_err(|_| EmbeddingError::CountLimit {
                field: "family effective space",
                value: u64_len(family.spaces.len(), "family effective space count")
                    .unwrap_or(u64::MAX),
            })?,
        );

        for space in &family.spaces {
            let space_index = usize::try_from(space_start)
                .map_err(|_| EmbeddingError::ArithmeticOverflow("space record index"))?;
            let offset = record_offset(space_index, SPACE_RECORD_LENGTH, "space record")?;
            spaces[offset..offset + 32].copy_from_slice(space.id.as_bytes());
            spaces[offset + 32..offset + 64].copy_from_slice(family.id.as_bytes());
            put_u32(&mut spaces, offset + 64, space.dimension);
            put_u32(&mut spaces, offset + 68, space.postprocessing.code());
            put_u32(&mut spaces, offset + 72, space.ordinal);
            space_start = space_start
                .checked_add(1)
                .ok_or(EmbeddingError::ArithmeticOverflow("space record index"))?;
        }
    }

    let pool_length = u64_len(pool.len(), "contract pool length")?;
    let section_length = checked_add(pool_offset, pool_length, "CONTRACTS section length")?;
    let mut output = zero_vec(section_length, "CONTRACTS allocation")?;
    put_u32(&mut output, 0, 1);
    put_u32(&mut output, 4, 0);
    put_u64(&mut output, 8, family_count);
    put_u64(&mut output, 16, CONTRACTS_HEADER_LENGTH);
    put_u32(&mut output, 24, FAMILY_RECORD_LENGTH as u32);
    put_u32(&mut output, 28, SPACE_RECORD_LENGTH as u32);
    put_u64(&mut output, 32, space_count);
    put_u64(&mut output, 40, space_records_offset);
    put_u64(&mut output, 48, pool_offset);
    put_u64(&mut output, 56, pool_length);
    copy_at(&mut output, CONTRACTS_HEADER_LENGTH, &records)?;
    copy_at(&mut output, space_records_offset, &spaces)?;
    copy_at(&mut output, pool_offset, &pool)?;
    Ok(EncodedContracts {
        bytes: output,
        families: canonical,
    })
}

/// Canonicalizes targets and encodes the `TARGETS` section.
pub fn encode_targets(targets: Vec<EmbeddingTarget>) -> Result<Vec<u8>, EmbeddingError> {
    let targets = canonical_targets(targets)?;
    encode_targets_canonical(&targets)
}

fn canonical_targets(
    mut targets: Vec<EmbeddingTarget>,
) -> Result<Vec<EmbeddingTarget>, EmbeddingError> {
    for target in &targets {
        let reconstructed = if let Some(identity) = &target.canonical_identity {
            EmbeddingTarget::from_canonical_identity(
                target.kind,
                identity.clone(),
                true,
                target.source_local_ordinal,
            )?
        } else {
            EmbeddingTarget::from_digest(
                target.kind,
                target.identity_digest,
                target.source_local_ordinal,
            )?
        };
        if &reconstructed != target {
            return Err(EmbeddingError::DigestMismatch {
                kind: DigestKind::Target,
                expected: target.id.into_bytes(),
                actual: reconstructed.id.into_bytes(),
            });
        }
    }
    targets.sort_unstable_by_key(|target| target.id);
    deduplicate_by_key(targets, |target| target.id, "target identity")
}

fn encode_targets_canonical(targets: &[EmbeddingTarget]) -> Result<Vec<u8>, EmbeddingError> {
    if targets.is_empty() {
        return Err(EmbeddingError::Missing("target"));
    }
    let target_count = u64_len(targets.len(), "target count")?;
    let records_length = checked_mul(target_count, TARGET_RECORD_LENGTH, "target table")?;
    let pool_offset = align8(checked_add(
        TARGETS_HEADER_LENGTH,
        records_length,
        "target identity pool offset",
    )?)?;
    let mut records = zero_vec(records_length, "target table allocation")?;
    let mut pool = Vec::new();
    for (index, target) in targets.iter().enumerate() {
        let offset = record_offset(index, TARGET_RECORD_LENGTH, "target record")?;
        records[offset..offset + 32].copy_from_slice(target.id.as_bytes());
        records[offset + 32..offset + 64].copy_from_slice(target.identity_digest.as_bytes());
        put_u32(&mut records, offset + 64, target.kind.code());
        let mut flags = 0;
        let (identity_offset, identity_length) = if let Some(identity) = &target.canonical_identity
        {
            flags |= IDENTITY_BYTES_PRESENT;
            append_pool_block(&mut pool, pool_offset, identity)?
        } else {
            (0, 0)
        };
        if target.source_local_ordinal.is_some() {
            flags |= SOURCE_ORDINAL_PRESENT;
        }
        put_u32(&mut records, offset + 68, flags);
        put_u64(&mut records, offset + 72, identity_offset);
        put_u64(&mut records, offset + 80, identity_length);
        put_u64(
            &mut records,
            offset + 88,
            target.source_local_ordinal.unwrap_or(u64::MAX),
        );
    }
    let pool_length = u64_len(pool.len(), "target identity pool length")?;
    let section_length = checked_add(pool_offset, pool_length, "TARGETS section length")?;
    let mut output = zero_vec(section_length, "TARGETS allocation")?;
    put_u32(&mut output, 0, 1);
    put_u32(&mut output, 4, TARGET_RECORD_LENGTH as u32);
    put_u64(&mut output, 8, target_count);
    put_u64(&mut output, 16, TARGETS_HEADER_LENGTH);
    put_u64(&mut output, 24, records_length);
    put_u64(&mut output, 32, pool_offset);
    put_u64(&mut output, 40, pool_length);
    copy_at(&mut output, TARGETS_HEADER_LENGTH, &records)?;
    copy_at(&mut output, pool_offset, &pool)?;
    Ok(output)
}

/// Canonicalizes target sets and encodes the `TARGET_SETS` section.
pub fn encode_target_sets(target_sets: Vec<TargetSet>) -> Result<Vec<u8>, EmbeddingError> {
    let sets = canonical_target_sets(target_sets)?;
    encode_target_sets_canonical(&sets)
}

fn canonical_target_sets(mut sets: Vec<TargetSet>) -> Result<Vec<TargetSet>, EmbeddingError> {
    for set in &sets {
        let reconstructed = TargetSet::new(set.targets.clone())?;
        if &reconstructed != set {
            return Err(EmbeddingError::DigestMismatch {
                kind: DigestKind::TargetSet,
                expected: set.id.into_bytes(),
                actual: reconstructed.id.into_bytes(),
            });
        }
    }
    sets.sort_unstable_by_key(|set| set.id);
    deduplicate_by_key(sets, |set| set.id, "target-set identity")
}

fn encode_target_sets_canonical(sets: &[TargetSet]) -> Result<Vec<u8>, EmbeddingError> {
    if sets.is_empty() {
        return Err(EmbeddingError::Missing("target set"));
    }
    let set_count = u64_len(sets.len(), "target-set count")?;
    let record_length = checked_mul(set_count, TARGET_SET_RECORD_LENGTH, "target-set table")?;
    let row_count = sets.iter().try_fold(0u64, |count, set| {
        count
            .checked_add(u64_len(set.targets.len(), "target-set row count")?)
            .ok_or(EmbeddingError::ArithmeticOverflow("target-set row count"))
    })?;
    let rows_offset = align8(checked_add(
        TARGET_SETS_HEADER_LENGTH,
        record_length,
        "target-set row table offset",
    )?)?;
    let rows_length = checked_mul(row_count, 32, "target-set row table")?;
    let section_length = checked_add(rows_offset, rows_length, "TARGET_SETS section length")?;
    let mut output = zero_vec(section_length, "TARGET_SETS allocation")?;
    put_u32(&mut output, 0, 1);
    put_u32(&mut output, 4, TARGET_SET_RECORD_LENGTH as u32);
    put_u64(&mut output, 8, set_count);
    put_u64(&mut output, 16, TARGET_SETS_HEADER_LENGTH);
    put_u64(&mut output, 24, record_length);
    put_u64(&mut output, 32, row_count);
    put_u64(&mut output, 40, rows_offset);
    put_u64(&mut output, 48, rows_length);
    let mut row_start = 0u64;
    for (index, set) in sets.iter().enumerate() {
        let offset = absolute_record_offset(
            TARGET_SETS_HEADER_LENGTH,
            index,
            TARGET_SET_RECORD_LENGTH,
            "target-set record",
        )?;
        output[offset..offset + 32].copy_from_slice(set.id.as_bytes());
        put_u64(&mut output, offset + 32, row_start);
        put_u64(
            &mut output,
            offset + 40,
            u64_len(set.targets.len(), "target-set row count")?,
        );
        for target in &set.targets {
            let row_byte_offset = checked_add(
                rows_offset,
                checked_mul(row_start, 32, "target-set row offset")?,
                "target-set row offset",
            )?;
            copy_at(&mut output, row_byte_offset, target.as_bytes())?;
            row_start = row_start
                .checked_add(1)
                .ok_or(EmbeddingError::ArithmeticOverflow("target-set row index"))?;
        }
    }
    Ok(output)
}

/// Canonicalizes structural relations and encodes the `RELATIONS` section.
pub fn encode_relations(relations: Vec<TargetRelation>) -> Result<Vec<u8>, EmbeddingError> {
    let relations = canonical_relations(relations)?;
    encode_relations_canonical(&relations)
}

fn canonical_relations(
    mut relations: Vec<TargetRelation>,
) -> Result<Vec<TargetRelation>, EmbeddingError> {
    for relation in &relations {
        match (relation.kind, relation.extension_role.as_deref()) {
            (RelationKind::Extension, Some(role)) => {
                let role = core::str::from_utf8(role)
                    .map_err(|_| EmbeddingError::InvalidUtf8("extension relation role"))?;
                if role.is_empty() {
                    return Err(EmbeddingError::Missing("extension relation role"));
                }
            }
            (RelationKind::Extension, None) => {
                return Err(EmbeddingError::Missing("extension relation role"));
            }
            (_, Some(_)) => {
                return Err(EmbeddingError::Malformed(
                    "built-in relation carries extension role bytes",
                ));
            }
            (_, None) => {}
        }
    }
    relations.sort_unstable();
    let mut canonical: Vec<TargetRelation> = Vec::with_capacity(relations.len());
    for relation in relations {
        if let Some(previous) = canonical.last()
            && previous.cmp(&relation) == Ordering::Equal
        {
            if previous == &relation {
                continue;
            }
            return Err(EmbeddingError::Duplicate("contradictory relation identity"));
        }
        canonical.push(relation);
    }
    Ok(canonical)
}

fn encode_relations_canonical(relations: &[TargetRelation]) -> Result<Vec<u8>, EmbeddingError> {
    let relation_count = u64_len(relations.len(), "relation count")?;
    let records_length = checked_mul(relation_count, RELATION_RECORD_LENGTH, "relation table")?;
    let pool_offset = align8(checked_add(
        RELATIONS_HEADER_LENGTH,
        records_length,
        "relation role pool offset",
    )?)?;
    let mut records = zero_vec(records_length, "relation table allocation")?;
    let mut pool = Vec::new();
    for (index, relation) in relations.iter().enumerate() {
        let offset = record_offset(index, RELATION_RECORD_LENGTH, "relation record")?;
        records[offset..offset + 32].copy_from_slice(relation.subject.as_bytes());
        records[offset + 32..offset + 64].copy_from_slice(relation.object.as_bytes());
        put_u32(&mut records, offset + 64, relation.kind.code());
        let (flags, role_offset, role_length) =
            if let Some(role) = relation.extension_role.as_deref() {
                let (role_offset, role_length) = append_pool_block(&mut pool, pool_offset, role)?;
                (RELATION_ROLE_BYTES_PRESENT, role_offset, role_length)
            } else {
                (0, 0, 0)
            };
        put_u32(&mut records, offset + 68, flags);
        records[offset + 72..offset + 104].copy_from_slice(&relation.role_digest());
        put_u64(&mut records, offset + 104, role_offset);
        put_u64(&mut records, offset + 112, role_length);
    }
    let pool_length = u64_len(pool.len(), "relation role pool length")?;
    let section_length = checked_add(pool_offset, pool_length, "RELATIONS section length")?;
    let mut output = zero_vec(section_length, "RELATIONS allocation")?;
    put_u32(&mut output, 0, 1);
    put_u32(&mut output, 4, RELATION_RECORD_LENGTH as u32);
    put_u64(&mut output, 8, relation_count);
    put_u64(&mut output, 16, RELATIONS_HEADER_LENGTH);
    put_u64(&mut output, 24, records_length);
    put_u64(&mut output, 32, pool_offset);
    put_u64(&mut output, 40, pool_length);
    copy_at(&mut output, RELATIONS_HEADER_LENGTH, &records)?;
    copy_at(&mut output, pool_offset, &pool)?;
    Ok(output)
}

/// Canonicalizes family-scoped spans and encodes the `TOKEN_SPANS` section.
pub fn encode_token_spans(spans: Vec<TokenSpan>) -> Result<Vec<u8>, EmbeddingError> {
    let spans = canonical_token_spans(spans)?;
    encode_token_spans_canonical(&spans)
}

fn canonical_token_spans(mut spans: Vec<TokenSpan>) -> Result<Vec<TokenSpan>, EmbeddingError> {
    for span in &spans {
        span.validate()?;
    }
    spans.sort_unstable_by_key(|span| (span.family_id, span.target_id));
    let mut canonical: Vec<TokenSpan> = Vec::with_capacity(spans.len());
    for span in spans {
        if let Some(previous) = canonical.last()
            && (previous.family_id, previous.target_id) == (span.family_id, span.target_id)
        {
            if previous == &span {
                continue;
            }
            return Err(EmbeddingError::Duplicate("contradictory family token span"));
        }
        canonical.push(span);
    }
    Ok(canonical)
}

fn encode_token_spans_canonical(spans: &[TokenSpan]) -> Result<Vec<u8>, EmbeddingError> {
    let span_count = u64_len(spans.len(), "token-span count")?;
    let records_length = checked_mul(span_count, TOKEN_SPAN_RECORD_LENGTH, "token-span table")?;
    let section_length = checked_add(
        TOKEN_SPANS_HEADER_LENGTH,
        records_length,
        "TOKEN_SPANS section length",
    )?;
    let mut output = zero_vec(section_length, "TOKEN_SPANS allocation")?;
    put_u32(&mut output, 0, 1);
    put_u32(&mut output, 4, TOKEN_SPAN_RECORD_LENGTH as u32);
    put_u64(&mut output, 8, span_count);
    put_u64(&mut output, 16, TOKEN_SPANS_HEADER_LENGTH);
    put_u64(&mut output, 24, records_length);
    for (index, span) in spans.iter().enumerate() {
        let offset = absolute_record_offset(
            TOKEN_SPANS_HEADER_LENGTH,
            index,
            TOKEN_SPAN_RECORD_LENGTH,
            "token-span record",
        )?;
        output[offset..offset + 32].copy_from_slice(span.family_id.as_bytes());
        output[offset + 32..offset + 64].copy_from_slice(span.target_id.as_bytes());
        put_u64(&mut output, offset + 64, span.token_start);
        put_u64(&mut output, offset + 72, span.token_end);
        put_u64(&mut output, offset + 80, span.model_input_token_count);
        put_u32(&mut output, offset + 88, span.flags());
    }
    Ok(output)
}

/// Typed scope of one exact external-artifact binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExternalScope {
    /// Exact source `.purrpck` SHA-256.
    Source(ContentDigest),
    /// One canonical target.
    Target(TargetId),
    /// One canonical target set.
    TargetSet(TargetSetId),
    /// One embedding family.
    Family(FamilyId),
    /// One effective vector space.
    VectorSpace(VectorSpaceId),
    /// One stored matrix.
    Matrix(MatrixId),
    /// One effective projection.
    Projection(ProjectionId),
    /// One opaque derived index.
    Index(IndexId),
}

impl ExternalScope {
    const fn code(self) -> u32 {
        match self {
            Self::Source(_) => 1,
            Self::Target(_) => 2,
            Self::TargetSet(_) => 3,
            Self::Family(_) => 4,
            Self::VectorSpace(_) => 5,
            Self::Matrix(_) => 6,
            Self::Projection(_) => 7,
            Self::Index(_) => 8,
        }
    }

    fn id(self) -> [u8; 32] {
        match self {
            Self::Source(id) => *id.as_bytes(),
            Self::Target(id) => id.into_bytes(),
            Self::TargetSet(id) => id.into_bytes(),
            Self::Family(id) => id.into_bytes(),
            Self::VectorSpace(id) => id.into_bytes(),
            Self::Matrix(id) => id.into_bytes(),
            Self::Projection(id) => id.into_bytes(),
            Self::Index(id) => id.into_bytes(),
        }
    }
}

/// Caller-supplied semantics for an exact external artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalBindingContract {
    /// Stable caller role identifier.
    pub role: String,
    /// Artifact media type or canonical format identifier.
    pub media_type: String,
    /// Optional stable caller identifier, never a retrieval path.
    pub stable_identifier: Option<Vec<u8>>,
    /// Optional exact artifact-format revision.
    pub revision: Option<Vec<u8>>,
    /// Optional opaque policy or provenance reference.
    pub policy_reference: Option<Vec<u8>>,
}

impl ExternalBindingContract {
    /// Encodes the canonical binding-contract TLV block.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EmbeddingError> {
        validate_nonempty_text(&self.role, "external binding role")?;
        validate_nonempty_text(&self.media_type, "external binding media type")?;
        let mut output = Vec::new();
        push_tlv(
            &mut output,
            1,
            TlvWireType::Utf8,
            true,
            self.role.as_bytes(),
        )?;
        push_tlv(
            &mut output,
            2,
            TlvWireType::Utf8,
            true,
            self.media_type.as_bytes(),
        )?;
        push_optional_nonempty_bytes(&mut output, 3, self.stable_identifier.as_deref())?;
        push_optional_nonempty_bytes(&mut output, 4, self.revision.as_deref())?;
        push_optional_nonempty_bytes(&mut output, 5, self.policy_reference.as_deref())?;
        Ok(output)
    }
}

/// One exact external-artifact binding with its derived identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalBinding {
    /// Stable binding identity.
    id: ExternalBindingId,
    /// Typed attachment scope.
    scope: ExternalScope,
    /// Plain SHA-256 of the exact external bytes.
    artifact_sha256: ContentDigest,
    /// Exact external byte length.
    artifact_length: u64,
    /// Independently computed external RDFC digest, when present.
    certified_rdf_digest: Option<RdfcDigest>,
    /// Digest of the canonical contract block.
    contract_digest: ExternalContractDigest,
    /// Exact canonical contract block.
    canonical_contract: Vec<u8>,
}

impl ExternalBinding {
    /// Derives a binding from exact external bytes.
    pub fn from_bytes(
        scope: ExternalScope,
        artifact_bytes: &[u8],
        contract: &ExternalBindingContract,
    ) -> Result<Self, EmbeddingError> {
        let artifact_length = u64_len(artifact_bytes.len(), "external artifact length")?;
        Self::from_digest(
            scope,
            ContentDigest::of(artifact_bytes),
            artifact_length,
            contract,
        )
    }

    /// Derives a binding from a trusted exact digest and byte length.
    pub fn from_digest(
        scope: ExternalScope,
        artifact_sha256: ContentDigest,
        artifact_length: u64,
        contract: &ExternalBindingContract,
    ) -> Result<Self, EmbeddingError> {
        Self::from_parts(scope, artifact_sha256, artifact_length, None, contract)
    }

    /// Independently verifies exact `.purrpck` v1 bytes and mints a binding
    /// carrying `CERTIFIED_RDF_PRESENT`.
    pub fn from_purrpck(
        scope: ExternalScope,
        artifact_bytes: &[u8],
        contract: &ExternalBindingContract,
    ) -> Result<Self, EmbeddingError> {
        let certified = verify_pack(artifact_bytes)
            .map_err(|error| EmbeddingError::InvalidExternalPack(error.to_string()))?;
        Self::from_parts(
            scope,
            ContentDigest::of(artifact_bytes),
            u64_len(artifact_bytes.len(), "external .purrpck length")?,
            Some(RdfcDigest::from_raw(*certified.as_bytes())),
            contract,
        )
    }

    fn from_parts(
        scope: ExternalScope,
        artifact_sha256: ContentDigest,
        artifact_length: u64,
        certified_rdf_digest: Option<RdfcDigest>,
        contract: &ExternalBindingContract,
    ) -> Result<Self, EmbeddingError> {
        let canonical_contract = contract.canonical_bytes()?;
        let contract_digest = derive_external_contract_digest(&canonical_contract);
        let certified = certified_rdf_digest.map_or([0; 32], RdfcDigest::into_bytes);
        let scope_id = scope.id();
        let id = derive_external_binding_id(ExternalBindingIdentity {
            scope_kind: scope.code(),
            scope_id: &scope_id,
            artifact_sha256,
            artifact_length,
            certified_rdf_digest: certified,
            contract_digest,
        });
        Ok(Self {
            id,
            scope,
            artifact_sha256,
            artifact_length,
            certified_rdf_digest,
            contract_digest,
            canonical_contract,
        })
    }

    /// Stable binding identity.
    #[must_use]
    pub const fn id(&self) -> ExternalBindingId {
        self.id
    }

    /// Typed attachment scope.
    #[must_use]
    pub const fn scope(&self) -> ExternalScope {
        self.scope
    }

    /// Plain SHA-256 of the exact external bytes.
    #[must_use]
    pub const fn artifact_sha256(&self) -> ContentDigest {
        self.artifact_sha256
    }

    /// Exact external byte length.
    #[must_use]
    pub const fn artifact_length(&self) -> u64 {
        self.artifact_length
    }

    /// Independently verified external RDFC digest, when present.
    #[must_use]
    pub const fn certified_rdf_digest(&self) -> Option<RdfcDigest> {
        self.certified_rdf_digest
    }

    /// Digest of the canonical binding contract.
    #[must_use]
    pub const fn contract_digest(&self) -> ExternalContractDigest {
        self.contract_digest
    }

    /// Exact canonical binding-contract bytes.
    #[must_use]
    pub fn canonical_contract(&self) -> &[u8] {
        &self.canonical_contract
    }
}

/// Canonicalizes exact external bindings and encodes `EXTERNAL_BINDINGS`.
pub fn encode_external_bindings(bindings: Vec<ExternalBinding>) -> Result<Vec<u8>, EmbeddingError> {
    let bindings = canonical_external_bindings(bindings)?;
    encode_external_bindings_canonical(&bindings)
}

fn canonical_external_bindings(
    mut bindings: Vec<ExternalBinding>,
) -> Result<Vec<ExternalBinding>, EmbeddingError> {
    for binding in &bindings {
        validate_external_binding(binding)?;
    }
    bindings.sort_unstable_by_key(|binding| binding.id);
    deduplicate_by_key(bindings, |binding| binding.id, "external binding identity")
}

fn validate_external_binding(binding: &ExternalBinding) -> Result<(), EmbeddingError> {
    let contract_digest = derive_external_contract_digest(&binding.canonical_contract);
    check_identity(
        DigestKind::ExternalBinding,
        binding.contract_digest.as_bytes(),
        contract_digest.as_bytes(),
    )?;
    let scope_id = binding.scope.id();
    let certified = binding
        .certified_rdf_digest
        .map_or([0; 32], RdfcDigest::into_bytes);
    let id = derive_external_binding_id(ExternalBindingIdentity {
        scope_kind: binding.scope.code(),
        scope_id: &scope_id,
        artifact_sha256: binding.artifact_sha256,
        artifact_length: binding.artifact_length,
        certified_rdf_digest: certified,
        contract_digest,
    });
    check_identity(
        DigestKind::ExternalBinding,
        binding.id.as_bytes(),
        id.as_bytes(),
    )
}

fn encode_external_bindings_canonical(
    bindings: &[ExternalBinding],
) -> Result<Vec<u8>, EmbeddingError> {
    let binding_count = u64_len(bindings.len(), "external binding count")?;
    let records_length = checked_mul(
        binding_count,
        EXTERNAL_BINDING_RECORD_LENGTH,
        "external binding table",
    )?;
    let pool_offset = align8(checked_add(
        EXTERNAL_BINDINGS_HEADER_LENGTH,
        records_length,
        "external contract pool offset",
    )?)?;
    let mut records = zero_vec(records_length, "external binding table allocation")?;
    let mut pool = Vec::new();
    for (index, binding) in bindings.iter().enumerate() {
        let offset = record_offset(
            index,
            EXTERNAL_BINDING_RECORD_LENGTH,
            "external binding record",
        )?;
        records[offset..offset + 32].copy_from_slice(binding.id.as_bytes());
        put_u32(&mut records, offset + 32, binding.scope.code());
        put_u32(
            &mut records,
            offset + 36,
            if binding.certified_rdf_digest.is_some() {
                CERTIFIED_RDF_PRESENT
            } else {
                0
            },
        );
        records[offset + 40..offset + 72].copy_from_slice(&binding.scope.id());
        records[offset + 72..offset + 104].copy_from_slice(binding.artifact_sha256.as_bytes());
        put_u64(&mut records, offset + 104, binding.artifact_length);
        if let Some(certified) = binding.certified_rdf_digest {
            records[offset + 112..offset + 144].copy_from_slice(certified.as_bytes());
        }
        records[offset + 144..offset + 176].copy_from_slice(binding.contract_digest.as_bytes());
        let (contract_offset, contract_length) =
            append_pool_block(&mut pool, pool_offset, &binding.canonical_contract)?;
        put_u64(&mut records, offset + 176, contract_offset);
        put_u64(&mut records, offset + 184, contract_length);
    }
    let pool_length = u64_len(pool.len(), "external contract pool length")?;
    let section_length = checked_add(pool_offset, pool_length, "EXTERNAL_BINDINGS section length")?;
    let mut output = zero_vec(section_length, "EXTERNAL_BINDINGS allocation")?;
    put_u32(&mut output, 0, 1);
    put_u32(&mut output, 4, EXTERNAL_BINDING_RECORD_LENGTH as u32);
    put_u64(&mut output, 8, binding_count);
    put_u64(&mut output, 16, EXTERNAL_BINDINGS_HEADER_LENGTH);
    put_u64(&mut output, 24, records_length);
    put_u64(&mut output, 32, pool_offset);
    put_u64(&mut output, 40, pool_length);
    copy_at(&mut output, EXTERNAL_BINDINGS_HEADER_LENGTH, &records)?;
    copy_at(&mut output, pool_offset, &pool)?;
    Ok(output)
}

/// Declared determinism of an opaque index payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum IndexBuildDeterminism {
    /// Equal logical input produces byte-identical payload bytes.
    Deterministic = 1,
    /// The detached payload may differ across equivalent builds.
    Nondeterministic = 2,
}

impl IndexBuildDeterminism {
    const fn code(self) -> u32 {
        self as u32
    }
}

/// Intended query-stage role of one index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum IndexUseRole {
    /// General-purpose index over its effective space.
    Generic = 1,
    /// Coarse retrieval over a shorter effective prefix.
    CoarsePrefixRetrieval = 2,
    /// Full-prefix reranking after candidate retrieval.
    FullPrefixReranking = 3,
}

impl IndexUseRole {
    const fn code(self) -> u32 {
        self as u32
    }
}

/// Explicit approximation and vector-loss contract for an opaque index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexLossContract {
    /// Whether payload construction quantizes or otherwise transforms vectors.
    pub transforms_vectors: bool,
    /// Stable loss-encoding identifier, required for transformed vectors.
    pub loss_encoding: Option<String>,
    /// Canonical loss parameters, required for transformed vectors.
    pub loss_parameters: Option<Vec<u8>>,
}

impl IndexLossContract {
    fn canonical_bytes(&self) -> Result<Vec<u8>, EmbeddingError> {
        match (
            self.transforms_vectors,
            self.loss_encoding.as_deref(),
            self.loss_parameters.as_deref(),
        ) {
            (false, None, None) => {}
            (true, Some(encoding), Some(_)) => {
                validate_nonempty_text(encoding, "index loss encoding")?;
            }
            (false, _, _) => {
                return Err(EmbeddingError::Malformed(
                    "loss details require transformed vectors",
                ));
            }
            (true, _, _) => {
                return Err(EmbeddingError::Missing("transformed-vector loss contract"));
            }
        }
        let mut output = Vec::new();
        push_tlv(&mut output, 1, TlvWireType::Bool, true, &[1])?;
        push_tlv(
            &mut output,
            2,
            TlvWireType::Bool,
            true,
            &[u8::from(self.transforms_vectors)],
        )?;
        if let (Some(encoding), Some(parameters)) = (&self.loss_encoding, &self.loss_parameters) {
            push_tlv(&mut output, 3, TlvWireType::Utf8, true, encoding.as_bytes())?;
            push_tlv(&mut output, 4, TlvWireType::Bytes, true, parameters)?;
            push_tlv(
                &mut output,
                5,
                TlvWireType::Digest32,
                true,
                ContentDigest::of(parameters).as_bytes(),
            )?;
        }
        Ok(output)
    }
}

/// Complete canonical contract guarding one opaque derived index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexGuardContract {
    /// Exact implementation or manifest identity.
    pub implementation: ArtifactIdentity,
    /// Stable canonical parameter-encoding identifier.
    pub parameter_encoding: String,
    /// Canonical build parameters, possibly empty.
    pub parameters: Vec<u8>,
    /// Explicit approximate/loss behavior.
    pub loss: IndexLossContract,
    /// Query-stage role.
    pub use_role: IndexUseRole,
    /// Payload media type or canonical format identifier.
    pub payload_media_type: String,
    /// Optional certified RDF metadata binding.
    pub certified_metadata_binding: Option<ExternalBindingId>,
}

impl IndexGuardContract {
    /// Encodes the canonical index-guard TLV block.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EmbeddingError> {
        validate_nonempty_text(&self.parameter_encoding, "index parameter encoding")?;
        validate_nonempty_text(&self.payload_media_type, "index payload media type")?;
        let implementation = encode_artifact_identity(&self.implementation)?;
        let loss = self.loss.canonical_bytes()?;
        let mut output = Vec::new();
        push_tlv(&mut output, 1, TlvWireType::Block, true, &implementation)?;
        push_tlv(
            &mut output,
            2,
            TlvWireType::Utf8,
            true,
            self.parameter_encoding.as_bytes(),
        )?;
        push_tlv(&mut output, 3, TlvWireType::Bytes, true, &self.parameters)?;
        push_tlv(
            &mut output,
            4,
            TlvWireType::Digest32,
            true,
            ContentDigest::of(&self.parameters).as_bytes(),
        )?;
        push_tlv(&mut output, 5, TlvWireType::Block, true, &loss)?;
        push_tlv(
            &mut output,
            6,
            TlvWireType::U32,
            true,
            &self.use_role.code().to_le_bytes(),
        )?;
        push_tlv(
            &mut output,
            7,
            TlvWireType::Utf8,
            true,
            self.payload_media_type.as_bytes(),
        )?;
        if let Some(binding) = self.certified_metadata_binding {
            push_tlv(
                &mut output,
                8,
                TlvWireType::Digest32,
                true,
                binding.as_bytes(),
            )?;
        }
        push_tlv(&mut output, 9, TlvWireType::Bool, true, &[1])?;
        Ok(output)
    }
}

/// Exact typed coordinates guarded by one opaque index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexCoordinates {
    /// Exact source `.purrpck` digest.
    pub source_exact_digest: ContentDigest,
    /// Embedding family.
    pub family_id: FamilyId,
    /// Effective vector space.
    pub vector_space_id: VectorSpaceId,
    /// Exact stored matrix.
    pub matrix_id: MatrixId,
    /// Exact effective projection.
    pub projection_id: ProjectionId,
    /// Canonical target row set.
    pub target_set_id: TargetSetId,
    /// Effective leading-prefix dimension.
    pub prefix_dimension: u32,
}

/// Inline or detached storage for exact opaque index bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexPayloadStorage {
    /// Exact deterministic bytes carried in an `INDEX_PAYLOAD` section.
    Inline(Vec<u8>),
    /// Exact digest and length of bytes supplied through an external binding.
    Detached {
        /// Plain SHA-256 of the detached bytes.
        payload_sha256: ContentDigest,
        /// Exact detached byte length.
        payload_length: u64,
    },
}

impl IndexPayloadStorage {
    const fn code(&self) -> u32 {
        match self {
            Self::Inline(_) => 1,
            Self::Detached { .. } => 2,
        }
    }

    fn digest_and_length(&self) -> Result<(ContentDigest, u64), EmbeddingError> {
        match self {
            Self::Inline(bytes) => {
                if bytes.is_empty() {
                    return Err(EmbeddingError::Missing("inline index payload"));
                }
                Ok((
                    ContentDigest::of(bytes),
                    u64_len(bytes.len(), "inline index payload length")?,
                ))
            }
            Self::Detached {
                payload_sha256,
                payload_length,
            } => {
                if *payload_length == 0 {
                    return Err(EmbeddingError::Missing("detached index payload"));
                }
                Ok((*payload_sha256, *payload_length))
            }
        }
    }
}

/// One opaque, rebuildable derived index and its exact guard commitment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedIndex {
    /// Stable index identity.
    id: IndexId,
    /// Exact guarded coordinates.
    coordinates: IndexCoordinates,
    /// Plain payload SHA-256.
    payload_sha256: ContentDigest,
    /// Exact payload length.
    payload_length: u64,
    /// Digest of the canonical guard contract.
    guard_digest: IndexGuardDigest,
    /// Exact canonical guard contract.
    canonical_guard: Vec<u8>,
    /// Inline or detached payload location.
    storage: IndexPayloadStorage,
    /// Declared payload determinism.
    determinism: IndexBuildDeterminism,
    /// Optional certified RDF metadata binding copied from the guard.
    certified_metadata_binding: Option<ExternalBindingId>,
}

impl DerivedIndex {
    /// Derives an index identity and guard from typed coordinates and storage.
    pub fn new(
        coordinates: IndexCoordinates,
        storage: IndexPayloadStorage,
        determinism: IndexBuildDeterminism,
        guard: &IndexGuardContract,
    ) -> Result<Self, EmbeddingError> {
        if coordinates.prefix_dimension == 0 {
            return Err(EmbeddingError::UnavailablePrefix(0));
        }
        if matches!(storage, IndexPayloadStorage::Inline(_))
            && determinism != IndexBuildDeterminism::Deterministic
        {
            return Err(EmbeddingError::Malformed(
                "inline index payload must be deterministic",
            ));
        }
        let (payload_sha256, payload_length) = storage.digest_and_length()?;
        let canonical_guard = guard.canonical_bytes()?;
        let guard_digest = derive_index_guard_digest(&canonical_guard);
        let id = derive_index_id(IndexIdentity {
            source_exact_digest: coordinates.source_exact_digest,
            family_id: coordinates.family_id,
            vector_space_id: coordinates.vector_space_id,
            matrix_id: coordinates.matrix_id,
            projection_id: coordinates.projection_id,
            target_set_id: coordinates.target_set_id,
            prefix_dimension: coordinates.prefix_dimension,
            payload_sha256,
            payload_length,
            determinism: determinism.code(),
            guard_digest,
        });
        Ok(Self {
            id,
            coordinates,
            payload_sha256,
            payload_length,
            guard_digest,
            canonical_guard,
            storage,
            determinism,
            certified_metadata_binding: guard.certified_metadata_binding,
        })
    }

    /// Stable index identity.
    #[must_use]
    pub const fn id(&self) -> IndexId {
        self.id
    }

    /// Exact guarded coordinates.
    #[must_use]
    pub const fn coordinates(&self) -> IndexCoordinates {
        self.coordinates
    }

    /// Plain payload SHA-256.
    #[must_use]
    pub const fn payload_sha256(&self) -> ContentDigest {
        self.payload_sha256
    }

    /// Exact payload byte length.
    #[must_use]
    pub const fn payload_length(&self) -> u64 {
        self.payload_length
    }

    /// Digest of the canonical guard contract.
    #[must_use]
    pub const fn guard_digest(&self) -> IndexGuardDigest {
        self.guard_digest
    }

    /// Exact canonical guard bytes.
    #[must_use]
    pub fn canonical_guard(&self) -> &[u8] {
        &self.canonical_guard
    }

    /// Inline or detached payload storage.
    #[must_use]
    pub const fn storage(&self) -> &IndexPayloadStorage {
        &self.storage
    }

    /// Declared payload determinism.
    #[must_use]
    pub const fn determinism(&self) -> IndexBuildDeterminism {
        self.determinism
    }

    /// Optional certified RDF metadata binding copied from the guard.
    #[must_use]
    pub const fn certified_metadata_binding(&self) -> Option<ExternalBindingId> {
        self.certified_metadata_binding
    }
}

/// Canonicalizes derived indexes and encodes `INDEX_GUARDS` plus inline
/// payload bodies in assigned instance order.
pub fn encode_index_guards(
    indexes: Vec<DerivedIndex>,
) -> Result<(Vec<u8>, Vec<Vec<u8>>), EmbeddingError> {
    let indexes = canonical_indexes(indexes)?;
    encode_index_guards_canonical(&indexes)
}

fn canonical_indexes(mut indexes: Vec<DerivedIndex>) -> Result<Vec<DerivedIndex>, EmbeddingError> {
    for index in &indexes {
        validate_derived_index(index)?;
    }
    indexes.sort_unstable_by_key(|index| index.id);
    deduplicate_by_key(indexes, |index| index.id, "derived index identity")
}

fn validate_derived_index(index: &DerivedIndex) -> Result<(), EmbeddingError> {
    if index.coordinates.prefix_dimension == 0 {
        return Err(EmbeddingError::UnavailablePrefix(0));
    }
    if matches!(index.storage, IndexPayloadStorage::Inline(_))
        && index.determinism != IndexBuildDeterminism::Deterministic
    {
        return Err(EmbeddingError::Malformed(
            "inline index payload must be deterministic",
        ));
    }
    let (payload_digest, payload_length) = index.storage.digest_and_length()?;
    check_identity(
        DigestKind::Index,
        index.payload_sha256.as_bytes(),
        payload_digest.as_bytes(),
    )?;
    if payload_length != index.payload_length {
        return Err(EmbeddingError::ContentMismatch("index payload length"));
    }
    let guard_digest = derive_index_guard_digest(&index.canonical_guard);
    check_identity(
        DigestKind::Index,
        index.guard_digest.as_bytes(),
        guard_digest.as_bytes(),
    )?;
    let actual_id = derive_index_id(IndexIdentity {
        source_exact_digest: index.coordinates.source_exact_digest,
        family_id: index.coordinates.family_id,
        vector_space_id: index.coordinates.vector_space_id,
        matrix_id: index.coordinates.matrix_id,
        projection_id: index.coordinates.projection_id,
        target_set_id: index.coordinates.target_set_id,
        prefix_dimension: index.coordinates.prefix_dimension,
        payload_sha256: index.payload_sha256,
        payload_length: index.payload_length,
        determinism: index.determinism.code(),
        guard_digest,
    });
    check_identity(DigestKind::Index, index.id.as_bytes(), actual_id.as_bytes())
}

fn encode_index_guards_canonical(
    indexes: &[DerivedIndex],
) -> Result<(Vec<u8>, Vec<Vec<u8>>), EmbeddingError> {
    let index_count = u64_len(indexes.len(), "derived index count")?;
    let records_length = checked_mul(index_count, INDEX_GUARD_RECORD_LENGTH, "index guard table")?;
    let pool_offset = align8(checked_add(
        INDEX_GUARDS_HEADER_LENGTH,
        records_length,
        "index guard pool offset",
    )?)?;
    let mut records = zero_vec(records_length, "index guard table allocation")?;
    let mut pool = Vec::new();
    let mut payloads = Vec::new();
    for (index_number, index) in indexes.iter().enumerate() {
        let offset = record_offset(
            index_number,
            INDEX_GUARD_RECORD_LENGTH,
            "index guard record",
        )?;
        records[offset..offset + 32].copy_from_slice(index.id.as_bytes());
        records[offset + 32..offset + 64]
            .copy_from_slice(index.coordinates.source_exact_digest.as_bytes());
        records[offset + 64..offset + 96].copy_from_slice(index.coordinates.family_id.as_bytes());
        records[offset + 96..offset + 128]
            .copy_from_slice(index.coordinates.vector_space_id.as_bytes());
        records[offset + 128..offset + 160].copy_from_slice(index.coordinates.matrix_id.as_bytes());
        records[offset + 160..offset + 192]
            .copy_from_slice(index.coordinates.projection_id.as_bytes());
        records[offset + 192..offset + 224]
            .copy_from_slice(index.coordinates.target_set_id.as_bytes());
        records[offset + 224..offset + 256].copy_from_slice(index.payload_sha256.as_bytes());
        records[offset + 256..offset + 288].copy_from_slice(index.guard_digest.as_bytes());
        put_u64(&mut records, offset + 288, index.payload_length);
        let (guard_offset, guard_length) =
            append_pool_block(&mut pool, pool_offset, &index.canonical_guard)?;
        put_u64(&mut records, offset + 296, guard_offset);
        put_u64(&mut records, offset + 304, guard_length);
        let payload_instance = if let IndexPayloadStorage::Inline(bytes) = &index.storage {
            payloads.push(bytes.clone());
            u32::try_from(payloads.len()).map_err(|_| EmbeddingError::CountLimit {
                field: "inline index",
                value: u64_len(payloads.len(), "inline index count").unwrap_or(u64::MAX),
            })?
        } else {
            0
        };
        put_u32(&mut records, offset + 312, payload_instance);
        put_u32(&mut records, offset + 316, index.storage.code());
        put_u32(&mut records, offset + 320, index.determinism.code());
        put_u32(&mut records, offset + 324, INDEX_REBUILDABLE);
        put_u32(
            &mut records,
            offset + 328,
            index.coordinates.prefix_dimension,
        );
    }
    let pool_length = u64_len(pool.len(), "index guard pool length")?;
    let section_length = checked_add(pool_offset, pool_length, "INDEX_GUARDS section length")?;
    let mut output = zero_vec(section_length, "INDEX_GUARDS allocation")?;
    put_u32(&mut output, 0, 1);
    put_u32(&mut output, 4, INDEX_GUARD_RECORD_LENGTH as u32);
    put_u64(&mut output, 8, index_count);
    put_u64(&mut output, 16, INDEX_GUARDS_HEADER_LENGTH);
    put_u64(&mut output, 24, records_length);
    put_u64(&mut output, 32, pool_offset);
    put_u64(&mut output, 40, pool_length);
    copy_at(&mut output, INDEX_GUARDS_HEADER_LENGTH, &records)?;
    copy_at(&mut output, pool_offset, &pool)?;
    Ok((output, payloads))
}

fn validate_source_target(
    source: CertifiedPurrpckSource,
    targets: &[EmbeddingTarget],
) -> Result<(), EmbeddingError> {
    let target = find_target(targets, source.dataset_target_id())?;
    if target.kind != TargetKind::RdfDataset {
        return Err(EmbeddingError::Malformed(
            "SOURCE dataset target is not RDF_DATASET",
        ));
    }
    let expected = source.dataset_target(target.canonical_identity.is_some())?;
    if target.id != expected.id || target.identity_digest != expected.identity_digest {
        return Err(EmbeddingError::DigestMismatch {
            kind: DigestKind::CertifiedRdf,
            expected: expected.id.into_bytes(),
            actual: target.id.into_bytes(),
        });
    }
    Ok(())
}

fn validate_target_set_references(
    sets: &[TargetSet],
    targets: &[EmbeddingTarget],
) -> Result<(), EmbeddingError> {
    for set in sets {
        for target in &set.targets {
            find_target(targets, *target)?;
        }
    }
    Ok(())
}

fn validate_relation_references(
    relations: &[TargetRelation],
    targets: &[EmbeddingTarget],
) -> Result<(), EmbeddingError> {
    for relation in relations {
        let subject = find_target(targets, relation.subject)?.kind;
        let object = find_target(targets, relation.object)?.kind;
        let valid = match relation.kind {
            RelationKind::CorpusDocument => {
                (subject, object) == (TargetKind::Corpus, TargetKind::Document)
            }
            RelationKind::DocumentChunk => {
                (subject, object) == (TargetKind::Document, TargetKind::Chunk)
            }
            RelationKind::DatasetGraph => {
                (subject, object) == (TargetKind::RdfDataset, TargetKind::RdfGraph)
            }
            RelationKind::GraphStatement => {
                (subject, object) == (TargetKind::RdfGraph, TargetKind::RdfStatement)
            }
            RelationKind::StatementSubject
            | RelationKind::StatementPredicate
            | RelationKind::StatementObject => {
                (subject, object) == (TargetKind::RdfStatement, TargetKind::RdfTerm)
            }
            RelationKind::StatementReifier => {
                (subject, object) == (TargetKind::RdfStatement, TargetKind::RdfReifier)
            }
            RelationKind::ReifierTerm => {
                (subject, object) == (TargetKind::RdfReifier, TargetKind::RdfTerm)
            }
            RelationKind::ReifierAnnotation => {
                (subject, object) == (TargetKind::RdfReifier, TargetKind::RdfAnnotation)
            }
            RelationKind::AnnotationPredicate | RelationKind::AnnotationObject => {
                (subject, object) == (TargetKind::RdfAnnotation, TargetKind::RdfTerm)
            }
            RelationKind::GraphName => {
                (subject, object) == (TargetKind::RdfGraph, TargetKind::RdfTerm)
            }
            RelationKind::TripleTermSubject
            | RelationKind::TripleTermPredicate
            | RelationKind::TripleTermObject => {
                (subject, object) == (TargetKind::RdfTerm, TargetKind::RdfTerm)
            }
            RelationKind::Extension => true,
        };
        if !valid {
            return Err(EmbeddingError::Malformed(
                "relation endpoint kinds disagree with relation kind",
            ));
        }
    }
    Ok(())
}

fn validate_token_span_references(
    spans: &[TokenSpan],
    families: &[EmbeddingFamily],
    targets: &[EmbeddingTarget],
) -> Result<(), EmbeddingError> {
    for span in spans {
        find_family(families, span.family_id)?;
        let kind = find_target(targets, span.target_id)?.kind;
        if !matches!(kind, TargetKind::Document | TargetKind::Chunk) {
            return Err(EmbeddingError::Malformed(
                "token span target is not a document or chunk",
            ));
        }
    }
    Ok(())
}

fn validate_matrix_and_index_references(
    source: CertifiedPurrpckSource,
    families: &[EmbeddingFamily],
    targets: &[EmbeddingTarget],
    target_sets: &[TargetSet],
    bindings: &[ExternalBinding],
    indexes: &[DerivedIndex],
    matrices: Option<&[MatrixCommitment]>,
) -> Result<(), EmbeddingError> {
    let needs_matrices = !indexes.is_empty()
        || bindings.iter().any(|binding| {
            matches!(
                binding.scope,
                ExternalScope::Matrix(_) | ExternalScope::Projection(_) | ExternalScope::Index(_)
            )
        });
    if needs_matrices && matrices.is_none() {
        return Err(EmbeddingError::MissingReference(
            "precommitted matrices for typed metadata",
        ));
    }
    let matrices = matrices.unwrap_or(&[]);
    validate_matrix_catalog(families, target_sets, matrices)?;
    for binding in bindings {
        validate_external_scope(
            binding.scope,
            source,
            families,
            targets,
            target_sets,
            matrices,
            indexes,
        )?;
    }
    for index in indexes {
        validate_index_references(source, families, target_sets, matrices, bindings, index)?;
    }
    Ok(())
}

fn validate_matrix_catalog(
    families: &[EmbeddingFamily],
    target_sets: &[TargetSet],
    matrices: &[MatrixCommitment],
) -> Result<(), EmbeddingError> {
    let mut ids = matrices
        .iter()
        .map(|matrix| matrix.matrix_id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    if ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(EmbeddingError::Duplicate("matrix identity"));
    }
    let mut pairs = matrices
        .iter()
        .map(|matrix| (matrix.family_id, matrix.target_set_id))
        .collect::<Vec<_>>();
    pairs.sort_unstable();
    if pairs.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(EmbeddingError::Duplicate("family and target-set matrix"));
    }
    for matrix in matrices {
        let family = find_family(families, matrix.family_id)?;
        let target_set = find_target_set(target_sets, matrix.target_set_id)?;
        if matrix.dtype != family.dtype
            || matrix.stored_dimension != family.stored_dimension
            || matrix.row_count != u64_len(target_set.targets.len(), "target-set row count")?
        {
            return Err(EmbeddingError::Malformed(
                "matrix shape or dtype disagrees with family and target set",
            ));
        }
        if matrix.projections.len() != family.spaces.len() {
            return Err(EmbeddingError::Malformed(
                "matrix projection count disagrees with family",
            ));
        }
        for (projection, space) in matrix.projections.iter().zip(&family.spaces) {
            if projection.vector_space_id != space.id
                || projection.effective_dimension != space.dimension
                || projection.postprocessing != space.postprocessing
            {
                return Err(EmbeddingError::Malformed(
                    "matrix projection disagrees with effective space",
                ));
            }
        }
    }
    Ok(())
}

fn validate_external_scope(
    scope: ExternalScope,
    source: CertifiedPurrpckSource,
    families: &[EmbeddingFamily],
    targets: &[EmbeddingTarget],
    target_sets: &[TargetSet],
    matrices: &[MatrixCommitment],
    indexes: &[DerivedIndex],
) -> Result<(), EmbeddingError> {
    match scope {
        ExternalScope::Source(digest) if digest == source.source_exact_digest() => Ok(()),
        ExternalScope::Source(_) => Err(EmbeddingError::DigestMismatch {
            kind: DigestKind::SourceExact,
            expected: *source.source_exact_digest().as_bytes(),
            actual: scope.id(),
        }),
        ExternalScope::Target(id) => find_target(targets, id).map(|_| ()),
        ExternalScope::TargetSet(id) => find_target_set(target_sets, id).map(|_| ()),
        ExternalScope::Family(id) => find_family(families, id).map(|_| ()),
        ExternalScope::VectorSpace(id) => find_space(families, id).map(|_| ()),
        ExternalScope::Matrix(id) => find_matrix(matrices, id).map(|_| ()),
        ExternalScope::Projection(id) => find_projection(matrices, id).map(|_| ()),
        ExternalScope::Index(id) => find_index(indexes, id).map(|_| ()),
    }
}

fn validate_index_references(
    source: CertifiedPurrpckSource,
    families: &[EmbeddingFamily],
    target_sets: &[TargetSet],
    matrices: &[MatrixCommitment],
    bindings: &[ExternalBinding],
    index: &DerivedIndex,
) -> Result<(), EmbeddingError> {
    if index.coordinates.source_exact_digest != source.source_exact_digest() {
        return Err(EmbeddingError::DigestMismatch {
            kind: DigestKind::SourceExact,
            expected: *source.source_exact_digest().as_bytes(),
            actual: *index.coordinates.source_exact_digest.as_bytes(),
        });
    }
    let family = find_family(families, index.coordinates.family_id)?;
    let space = find_space(families, index.coordinates.vector_space_id)?;
    let matrix = find_matrix(matrices, index.coordinates.matrix_id)?;
    let projection = find_projection(matrices, index.coordinates.projection_id)?;
    find_target_set(target_sets, index.coordinates.target_set_id)?;
    if space.family_id != family.id
        || space.dimension != index.coordinates.prefix_dimension
        || matrix.family_id != family.id
        || matrix.target_set_id != index.coordinates.target_set_id
        || projection.vector_space_id != space.id
        || projection.effective_dimension != index.coordinates.prefix_dimension
        || !matrix
            .projections
            .iter()
            .any(|candidate| candidate.projection_id == projection.projection_id)
    {
        return Err(EmbeddingError::MissingReference(
            "coherent derived-index guard coordinates",
        ));
    }
    if let Some(binding_id) = index.certified_metadata_binding {
        let binding = find_binding(bindings, binding_id)?;
        if binding.certified_rdf_digest.is_none()
            || !scope_matches_index_metadata(binding.scope, source, index)
        {
            return Err(EmbeddingError::MissingReference(
                "certified index metadata binding",
            ));
        }
    }
    if matches!(index.storage, IndexPayloadStorage::Detached { .. })
        && !bindings.iter().any(|binding| {
            binding.scope == ExternalScope::Index(index.id)
                && binding.artifact_sha256 == index.payload_sha256
                && binding.artifact_length == index.payload_length
        })
    {
        return Err(EmbeddingError::MissingReference(
            "detached index external binding",
        ));
    }
    Ok(())
}

fn scope_matches_index_metadata(
    scope: ExternalScope,
    source: CertifiedPurrpckSource,
    index: &DerivedIndex,
) -> bool {
    match scope {
        ExternalScope::Source(digest) => digest == source.source_exact_digest(),
        ExternalScope::Family(id) => id == index.coordinates.family_id,
        ExternalScope::VectorSpace(id) => id == index.coordinates.vector_space_id,
        ExternalScope::Matrix(id) => id == index.coordinates.matrix_id,
        ExternalScope::Projection(id) => id == index.coordinates.projection_id,
        ExternalScope::Target(_) | ExternalScope::TargetSet(_) | ExternalScope::Index(_) => false,
    }
}

fn find_target(
    targets: &[EmbeddingTarget],
    id: TargetId,
) -> Result<&EmbeddingTarget, EmbeddingError> {
    targets
        .binary_search_by_key(&id, |target| target.id)
        .ok()
        .map(|index| &targets[index])
        .ok_or(EmbeddingError::MissingReference("target"))
}

fn find_family(
    families: &[EmbeddingFamily],
    id: FamilyId,
) -> Result<&EmbeddingFamily, EmbeddingError> {
    families
        .binary_search_by_key(&id, |family| family.id)
        .ok()
        .map(|index| &families[index])
        .ok_or(EmbeddingError::MissingReference("embedding family"))
}

fn find_space(
    families: &[EmbeddingFamily],
    id: VectorSpaceId,
) -> Result<&super::contract::EffectiveSpace, EmbeddingError> {
    families
        .iter()
        .flat_map(|family| &family.spaces)
        .find(|space| space.id == id)
        .ok_or(EmbeddingError::MissingReference("effective vector space"))
}

fn find_target_set(sets: &[TargetSet], id: TargetSetId) -> Result<&TargetSet, EmbeddingError> {
    sets.binary_search_by_key(&id, |set| set.id)
        .ok()
        .map(|index| &sets[index])
        .ok_or(EmbeddingError::MissingReference("target set"))
}

fn find_matrix(
    matrices: &[MatrixCommitment],
    id: MatrixId,
) -> Result<&MatrixCommitment, EmbeddingError> {
    matrices
        .iter()
        .find(|matrix| matrix.matrix_id == id)
        .ok_or(EmbeddingError::MissingReference("matrix"))
}

fn find_projection(
    matrices: &[MatrixCommitment],
    id: ProjectionId,
) -> Result<&ProjectionCommitment, EmbeddingError> {
    matrices
        .iter()
        .flat_map(|matrix| &matrix.projections)
        .find(|projection| projection.projection_id == id)
        .ok_or(EmbeddingError::MissingReference("matrix projection"))
}

fn find_binding(
    bindings: &[ExternalBinding],
    id: ExternalBindingId,
) -> Result<&ExternalBinding, EmbeddingError> {
    bindings
        .binary_search_by_key(&id, |binding| binding.id)
        .ok()
        .map(|index| &bindings[index])
        .ok_or(EmbeddingError::MissingReference("external binding"))
}

fn find_index(indexes: &[DerivedIndex], id: IndexId) -> Result<&DerivedIndex, EmbeddingError> {
    indexes
        .binary_search_by_key(&id, |index| index.id)
        .ok()
        .map(|index| &indexes[index])
        .ok_or(EmbeddingError::MissingReference("derived index"))
}

fn encode_artifact_identity(identity: &ArtifactIdentity) -> Result<Vec<u8>, EmbeddingError> {
    validate_nonempty_text(&identity.identifier, "artifact identifier")?;
    validate_nonempty_text(&identity.media_type, "artifact media type")?;
    if identity.revision.as_ref().is_some_and(Vec::is_empty) {
        return Err(EmbeddingError::Missing("artifact revision bytes"));
    }
    let mut output = Vec::new();
    push_tlv(
        &mut output,
        1,
        TlvWireType::Utf8,
        true,
        identity.identifier.as_bytes(),
    )?;
    push_tlv(
        &mut output,
        2,
        TlvWireType::Utf8,
        true,
        identity.media_type.as_bytes(),
    )?;
    push_tlv(
        &mut output,
        3,
        TlvWireType::Digest32,
        true,
        identity.digest.as_bytes(),
    )?;
    if let Some(revision) = &identity.revision {
        push_tlv(&mut output, 4, TlvWireType::Bytes, true, revision)?;
    }
    let kind = match identity.kind {
        ArtifactIdentityKind::Single => 1u32,
        ArtifactIdentityKind::Manifest => 2u32,
    };
    push_tlv(&mut output, 5, TlvWireType::U32, true, &kind.to_le_bytes())?;
    Ok(output)
}

fn push_optional_nonempty_bytes(
    output: &mut Vec<u8>,
    tag: u16,
    value: Option<&[u8]>,
) -> Result<(), EmbeddingError> {
    if let Some(value) = value {
        if value.is_empty() {
            return Err(EmbeddingError::Missing("optional external contract bytes"));
        }
        push_tlv(output, tag, TlvWireType::Bytes, true, value)?;
    }
    Ok(())
}

fn validate_nonempty_text(value: &str, context: &'static str) -> Result<(), EmbeddingError> {
    if value.is_empty() {
        return Err(EmbeddingError::Missing(context));
    }
    if value.as_bytes().contains(&0) {
        return Err(EmbeddingError::InvalidUtf8(context));
    }
    Ok(())
}

fn deduplicate_by_key<T, K, F>(
    values: Vec<T>,
    mut key: F,
    context: &'static str,
) -> Result<Vec<T>, EmbeddingError>
where
    T: PartialEq,
    K: Copy + Eq,
    F: FnMut(&T) -> K,
{
    let mut canonical = Vec::with_capacity(values.len());
    for value in values {
        if let Some(previous) = canonical.last()
            && key(previous) == key(&value)
        {
            if previous == &value {
                continue;
            }
            return Err(EmbeddingError::Duplicate(context));
        }
        canonical.push(value);
    }
    Ok(canonical)
}

fn append_pool_block(
    pool: &mut Vec<u8>,
    pool_offset: u64,
    bytes: &[u8],
) -> Result<(u64, u64), EmbeddingError> {
    if bytes.is_empty() {
        return Err(EmbeddingError::Missing("metadata pool block"));
    }
    let current = checked_add(
        pool_offset,
        u64_len(pool.len(), "metadata pool length")?,
        "metadata pool offset",
    )?;
    let aligned = align8(current)?;
    let padding = aligned
        .checked_sub(current)
        .ok_or(EmbeddingError::ArithmeticOverflow("metadata pool padding"))?;
    let new_len = u64_len(pool.len(), "metadata pool length")?
        .checked_add(padding)
        .ok_or(EmbeddingError::ArithmeticOverflow("metadata pool padding"))?;
    pool.resize(
        usize::try_from(new_len)
            .map_err(|_| EmbeddingError::ArithmeticOverflow("metadata pool allocation"))?,
        0,
    );
    pool.extend_from_slice(bytes);
    Ok((aligned, u64_len(bytes.len(), "metadata block length")?))
}

fn align8(value: u64) -> Result<u64, EmbeddingError> {
    checked_align_up(value, 8)
}

fn checked_add(left: u64, right: u64, context: &'static str) -> Result<u64, EmbeddingError> {
    left.checked_add(right)
        .ok_or(EmbeddingError::ArithmeticOverflow(context))
}

fn checked_mul(left: u64, right: u64, context: &'static str) -> Result<u64, EmbeddingError> {
    left.checked_mul(right)
        .ok_or(EmbeddingError::ArithmeticOverflow(context))
}

fn u64_len(length: usize, context: &'static str) -> Result<u64, EmbeddingError> {
    u64::try_from(length).map_err(|_| EmbeddingError::ArithmeticOverflow(context))
}

fn zero_vec(length: u64, context: &'static str) -> Result<Vec<u8>, EmbeddingError> {
    let length =
        usize::try_from(length).map_err(|_| EmbeddingError::ArithmeticOverflow(context))?;
    Ok(vec![0; length])
}

fn record_offset(
    index: usize,
    record_length: u64,
    context: &'static str,
) -> Result<usize, EmbeddingError> {
    let index = u64_len(index, context)?;
    usize::try_from(checked_mul(index, record_length, context)?)
        .map_err(|_| EmbeddingError::ArithmeticOverflow(context))
}

fn absolute_record_offset(
    table_offset: u64,
    index: usize,
    record_length: u64,
    context: &'static str,
) -> Result<usize, EmbeddingError> {
    let relative = checked_mul(u64_len(index, context)?, record_length, context)?;
    usize::try_from(checked_add(table_offset, relative, context)?)
        .map_err(|_| EmbeddingError::ArithmeticOverflow(context))
}

fn copy_at(output: &mut [u8], offset: u64, bytes: &[u8]) -> Result<(), EmbeddingError> {
    let start = usize::try_from(offset)
        .map_err(|_| EmbeddingError::ArithmeticOverflow("section copy offset"))?;
    let end = start
        .checked_add(bytes.len())
        .ok_or(EmbeddingError::ArithmeticOverflow("section copy span"))?;
    output
        .get_mut(start..end)
        .ok_or(EmbeddingError::InvalidSpan {
            context: "section copy",
            offset,
            length: u64_len(bytes.len(), "section copy length")?,
        })?
        .copy_from_slice(bytes);
    Ok(())
}

fn check_identity(
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
    use crate::RdfDatasetBuilder;

    use super::*;
    use crate::ir::embedding::{
        AppliedStage, ArtifactIdentity, DimensionalityPolicy, DistanceMetric, RdfTermTarget,
        StageImplementation,
    };

    fn artifact(name: &str) -> ArtifactIdentity {
        ArtifactIdentity::new(
            name,
            "application/example",
            ContentDigest::of(name.as_bytes()),
            None,
            ArtifactIdentityKind::Single,
        )
        .unwrap()
    }

    fn stage(name: &str) -> AppliedStage {
        AppliedStage::Applied(
            StageImplementation::new(
                name,
                ContentDigest::of(name.as_bytes()),
                "application/octet-stream",
                vec![1, 2, 3],
            )
            .unwrap(),
        )
    }

    fn family_contract() -> EmbeddingFamilyContract {
        EmbeddingFamilyContract {
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
            dtype: super::super::contract::VectorDtype::F32,
            metric: DistanceMetric::Cosine,
            dimensionality: DimensionalityPolicy::fixed(
                2,
                super::super::contract::PrefixPostprocessing::None,
            )
            .unwrap(),
            extensions: Vec::new(),
        }
    }

    fn certified_empty_pack() -> (CertifiedPurrpckSource, Vec<u8>) {
        let dataset = RdfDatasetBuilder::new().freeze().unwrap();
        CertifiedPurrpckSource::from_dataset(&dataset).unwrap()
    }

    #[test]
    fn source_certification_is_exact_purrpck_only() {
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
            CertifiedPurrpckSource::certify(b"not a pack"),
            Err(EmbeddingError::InvalidSourcePack(_))
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

    #[test]
    fn target_encoding_ignores_insertion_order() {
        let first = RdfTermTarget::Iri("https://example.org/a".into())
            .into_target(true, Some(1))
            .unwrap();
        let second = RdfTermTarget::Iri("https://example.org/b".into())
            .into_target(true, Some(2))
            .unwrap();
        let forward = encode_targets(vec![first.clone(), second.clone()]).unwrap();
        let reverse = encode_targets(vec![second, first]).unwrap();
        assert_eq!(forward, reverse);
    }

    #[test]
    fn empty_relation_section_has_the_exact_canonical_header() {
        let actual = encode_relations(Vec::new()).unwrap();
        let mut expected = [0u8; 64];
        expected[0..4].copy_from_slice(&1u32.to_le_bytes());
        expected[4..8].copy_from_slice(&120u32.to_le_bytes());
        expected[16..24].copy_from_slice(&64u64.to_le_bytes());
        expected[32..40].copy_from_slice(&64u64.to_le_bytes());
        assert_eq!(actual, expected);
    }

    #[test]
    fn only_verified_external_purrpck_mints_certified_rdf() {
        let (_, pack) = certified_empty_pack();
        let contract = ExternalBindingContract {
            role: "https://example.org/role/source".into(),
            media_type: "application/vnd.purrdf.pack".into(),
            stable_identifier: None,
            revision: None,
            policy_reference: None,
        };
        let scope = ExternalScope::Source(ContentDigest::of(b"owner"));
        let exact = ExternalBinding::from_bytes(scope, b"ordinary bytes", &contract).unwrap();
        assert_eq!(exact.certified_rdf_digest(), None);
        let certified = ExternalBinding::from_purrpck(scope, &pack, &contract).unwrap();
        assert!(certified.certified_rdf_digest().is_some());
        assert!(matches!(
            ExternalBinding::from_purrpck(scope, b"ordinary bytes", &contract),
            Err(EmbeddingError::InvalidExternalPack(_))
        ));
    }

    #[test]
    fn inline_indexes_assign_instances_in_canonical_id_order() {
        let coordinates = IndexCoordinates {
            source_exact_digest: ContentDigest::of(b"source"),
            family_id: FamilyId::from_raw([1; 32]),
            vector_space_id: VectorSpaceId::from_raw([2; 32]),
            matrix_id: MatrixId::from_raw([3; 32]),
            projection_id: ProjectionId::from_raw([4; 32]),
            target_set_id: TargetSetId::from_raw([5; 32]),
            prefix_dimension: 2,
        };
        let guard = IndexGuardContract {
            implementation: artifact("index"),
            parameter_encoding: "application/example".into(),
            parameters: vec![8, 9],
            loss: IndexLossContract {
                transforms_vectors: false,
                loss_encoding: None,
                loss_parameters: None,
            },
            use_role: IndexUseRole::Generic,
            payload_media_type: "application/example-index".into(),
            certified_metadata_binding: None,
        };
        let index = DerivedIndex::new(
            coordinates,
            IndexPayloadStorage::Inline(vec![1, 2, 3]),
            IndexBuildDeterminism::Deterministic,
            &guard,
        )
        .unwrap();
        let (section, payloads) = encode_index_guards(vec![index]).unwrap();
        assert_eq!(payloads, vec![vec![1, 2, 3]]);
        assert_eq!(
            u32::from_le_bytes(section[64 + 312..64 + 316].try_into().unwrap()),
            1
        );
    }
}
