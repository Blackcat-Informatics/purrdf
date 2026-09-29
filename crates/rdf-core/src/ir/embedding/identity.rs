// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Strong, domain-separated identities used by the PURREMB container.
//!
//! Every persisted identity occupies 32 bytes, but distinct newtypes prevent a
//! source digest, vector-space id, target id, matrix id, or artifact root from
//! being substituted accidentally. The hash fold is the normative length-framed
//! construction from `docs/PURREMB.md`.

use core::fmt;

use purrdf_hash::Domain;
use sha2::{Digest as _, Sha256};

use crate::ContentDigest;

// The hash domains of the PURREMB identities: `docs/PURREMB.md` names each one,
// and each is the prefix `hash_fold` opens its preimage with. The writer and the
// verifier stream three of them through their own framed hashers.
const D_ARTIFACT: Domain = Domain::new(b"purrdf.purremb.v1.artifact\0");
const D_FAMILY_CONTRACT: Domain = Domain::new(b"purrdf.purremb.v1.family-contract\0");
const D_FAMILY: Domain = Domain::new(b"purrdf.purremb.v1.family\0");
const D_CHUNKING: Domain = Domain::new(b"purrdf.purremb.v1.chunking\0");
const D_SPACE: Domain = Domain::new(b"purrdf.purremb.v1.vector-space\0");
const D_TARGET_IDENTITY: Domain = Domain::new(b"purrdf.purremb.v1.target-identity\0");
const D_TARGET: Domain = Domain::new(b"purrdf.purremb.v1.target\0");
pub(super) const D_TARGET_SET: Domain = Domain::new(b"purrdf.purremb.v1.target-set\0");
const D_RELATION_ROLE: Domain = Domain::new(b"purrdf.purremb.v1.relation-role\0");
pub(super) const D_MATRIX_CONTENT: Domain = Domain::new(b"purrdf.purremb.v1.matrix-content\0");
const D_MATRIX: Domain = Domain::new(b"purrdf.purremb.v1.matrix\0");
pub(super) const D_PROJECTION_CONTENT: Domain =
    Domain::new(b"purrdf.purremb.v1.projection-content\0");
const D_PROJECTION: Domain = Domain::new(b"purrdf.purremb.v1.projection\0");
const D_EXTERNAL_CONTRACT: Domain = Domain::new(b"purrdf.purremb.v1.external-contract\0");
const D_EXTERNAL: Domain = Domain::new(b"purrdf.purremb.v1.external-binding\0");
const D_INDEX_GUARD: Domain = Domain::new(b"purrdf.purremb.v1.index-guard\0");
const D_INDEX: Domain = Domain::new(b"purrdf.purremb.v1.index\0");

macro_rules! identity_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        pub struct $name([u8; 32]);

        impl $name {
            /// Adopts already-decoded identity bytes without hashing them.
            #[must_use]
            pub const fn from_raw(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            /// Returns the exact 32 persisted bytes.
            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }

            /// Consumes the identity and returns its exact persisted bytes.
            #[must_use]
            pub const fn into_bytes(self) -> [u8; 32] {
                self.0
            }

            /// Renders lowercase hexadecimal for diagnostics and tooling.
            #[must_use]
            pub fn to_hex(self) -> String {
                crate::hex::lower(&self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple(stringify!($name)).field(&crate::hex::lower(&self.0)).finish()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&crate::hex::lower(&self.0))
            }
        }
    };
}

identity_type!(
    /// Digest of one canonical vector-family contract block.
    FamilyContractDigest
);
identity_type!(
    /// Identity of a complete embedding family.
    FamilyId
);
identity_type!(
    /// Identity of the exact chunking-stage contract.
    ChunkingContractId
);
identity_type!(
    /// Identity of one effective dimension and prefix policy in a family.
    VectorSpaceId
);
identity_type!(
    /// Digest of one target kind's canonical identity block.
    TargetIdentityDigest
);
identity_type!(
    /// Stable identity of one embedding target.
    TargetId
);
identity_type!(
    /// Identity of one sorted, duplicate-free target row set.
    TargetSetId
);
identity_type!(
    /// Domain-separated digest of one stored matrix's exact scalar bytes.
    MatrixContentDigest
);
identity_type!(
    /// Identity of one stored matrix over a family and target set.
    MatrixId
);
identity_type!(
    /// Domain-separated digest of one effective matrix projection.
    ProjectionContentDigest
);
identity_type!(
    /// Identity of one effective matrix projection.
    ProjectionId
);
identity_type!(
    /// Digest of one generic external-artifact contract.
    ExternalContractDigest
);
identity_type!(
    /// Identity of one exact external-artifact binding.
    ExternalBindingId
);
identity_type!(
    /// Digest of one opaque derived-index guard contract.
    IndexGuardDigest
);
identity_type!(
    /// Identity of one guarded opaque derived index.
    IndexId
);
identity_type!(
    /// Integrity root over the canonical PURREMB header and section directory.
    ArtifactRoot
);
identity_type!(
    /// Claimed or verified RDFC SHA-256 bytes carried by PURREMB metadata.
    RdfcDigest
);

/// Inputs to the external-binding identity fold.
#[derive(Clone, Copy, Debug)]
pub struct ExternalBindingIdentity<'a> {
    /// Numeric scope-kind code from the wire format.
    pub scope_kind: u32,
    /// Typed scope identity serialized as 32 bytes.
    pub scope_id: &'a [u8; 32],
    /// Plain SHA-256 of all external artifact bytes.
    pub artifact_sha256: ContentDigest,
    /// Exact external artifact byte length.
    pub artifact_length: u64,
    /// Independently certified RDF digest, or 32 zero bytes when absent.
    pub certified_rdf_digest: [u8; 32],
    /// Digest of the canonical binding contract.
    pub contract_digest: ExternalContractDigest,
}

/// Inputs to the opaque derived-index identity fold.
#[derive(Clone, Copy, Debug)]
pub struct IndexIdentity {
    /// Plain SHA-256 of the exact source pack.
    pub source_exact_digest: ContentDigest,
    /// Family whose vectors the index accelerates.
    pub family_id: FamilyId,
    /// Effective vector space, including prefix dimension and postprocessing.
    pub vector_space_id: VectorSpaceId,
    /// Exact stored matrix identity.
    pub matrix_id: MatrixId,
    /// Exact effective projection identity.
    pub projection_id: ProjectionId,
    /// Target row set indexed by the payload.
    pub target_set_id: TargetSetId,
    /// Effective leading-prefix dimension.
    pub prefix_dimension: u32,
    /// Plain SHA-256 of the exact opaque payload bytes.
    pub payload_sha256: ContentDigest,
    /// Exact opaque payload byte length.
    pub payload_length: u64,
    /// Declared determinism code.
    pub determinism: u32,
    /// Digest of the canonical guard contract.
    pub guard_digest: IndexGuardDigest,
}

/// Derives a digest from a canonical family-contract block.
#[must_use]
pub fn derive_family_contract_digest(contract_bytes: &[u8]) -> FamilyContractDigest {
    FamilyContractDigest(hash_fold(D_FAMILY_CONTRACT, &[contract_bytes]))
}

/// Derives the family id from its contract digest.
#[must_use]
pub fn derive_family_id(digest: FamilyContractDigest) -> FamilyId {
    FamilyId(hash_fold(D_FAMILY, &[digest.as_bytes()]))
}

/// Derives the chunking-contract id from the nested chunking-stage block.
#[must_use]
pub fn derive_chunking_contract_id(stage_bytes: &[u8]) -> ChunkingContractId {
    ChunkingContractId(hash_fold(D_CHUNKING, &[stage_bytes]))
}

/// Derives one effective vector-space id.
#[must_use]
pub fn derive_vector_space_id(
    family_id: FamilyId,
    effective_dimension: u32,
    prefix_postprocessing: u32,
) -> VectorSpaceId {
    VectorSpaceId(hash_fold(
        D_SPACE,
        &[
            family_id.as_bytes(),
            &effective_dimension.to_le_bytes(),
            &prefix_postprocessing.to_le_bytes(),
        ],
    ))
}

/// Derives a target identity digest from its kind and canonical block.
#[must_use]
pub fn derive_target_identity_digest(
    target_kind: u32,
    canonical_identity: &[u8],
) -> TargetIdentityDigest {
    TargetIdentityDigest(hash_fold(
        D_TARGET_IDENTITY,
        &[&target_kind.to_le_bytes(), canonical_identity],
    ))
}

/// Derives the target id from its kind and identity digest.
#[must_use]
pub fn derive_target_id(target_kind: u32, identity_digest: TargetIdentityDigest) -> TargetId {
    TargetId(hash_fold(
        D_TARGET,
        &[&target_kind.to_le_bytes(), identity_digest.as_bytes()],
    ))
}

/// Derives a target-set id from its strictly sorted target ids.
#[must_use]
pub fn derive_target_set_id(target_ids: &[TargetId]) -> TargetSetId {
    let row_count = u64::try_from(target_ids.len()).expect("a slice length fits u64");
    let mut fields: Vec<&[u8]> = Vec::with_capacity(target_ids.len() + 1);
    let row_count_bytes = row_count.to_le_bytes();
    fields.push(&row_count_bytes);
    fields.extend(target_ids.iter().map(|id| id.as_bytes().as_slice()));
    TargetSetId(hash_fold(D_TARGET_SET, &fields))
}

/// Derives a role digest for an extension relation.
#[must_use]
pub fn derive_relation_role_digest(role_bytes: &[u8]) -> [u8; 32] {
    hash_fold(D_RELATION_ROLE, &[role_bytes])
}

/// Derives the typed content digest of exact stored matrix bytes.
#[must_use]
pub fn derive_matrix_content_digest(
    dtype: u32,
    row_count: u64,
    stored_dimension: u32,
    matrix_bytes: &[u8],
) -> MatrixContentDigest {
    MatrixContentDigest(hash_fold(
        D_MATRIX_CONTENT,
        &[
            &dtype.to_le_bytes(),
            &row_count.to_le_bytes(),
            &stored_dimension.to_le_bytes(),
            matrix_bytes,
        ],
    ))
}

/// Derives a stored-matrix id.
#[must_use]
pub fn derive_matrix_id(
    target_set_id: TargetSetId,
    family_id: FamilyId,
    content_digest: MatrixContentDigest,
) -> MatrixId {
    MatrixId(hash_fold(
        D_MATRIX,
        &[
            target_set_id.as_bytes(),
            family_id.as_bytes(),
            content_digest.as_bytes(),
        ],
    ))
}

/// Derives the typed content digest of an effective projection byte stream.
#[must_use]
pub fn derive_projection_content_digest(
    dtype: u32,
    row_count: u64,
    effective_dimension: u32,
    prefix_postprocessing: u32,
    logical_projection_bytes: &[u8],
) -> ProjectionContentDigest {
    ProjectionContentDigest(hash_fold(
        D_PROJECTION_CONTENT,
        &[
            &dtype.to_le_bytes(),
            &row_count.to_le_bytes(),
            &effective_dimension.to_le_bytes(),
            &prefix_postprocessing.to_le_bytes(),
            logical_projection_bytes,
        ],
    ))
}

/// Derives an effective projection id.
#[must_use]
pub fn derive_projection_id(
    matrix_id: MatrixId,
    vector_space_id: VectorSpaceId,
    content_digest: ProjectionContentDigest,
) -> ProjectionId {
    ProjectionId(hash_fold(
        D_PROJECTION,
        &[
            matrix_id.as_bytes(),
            vector_space_id.as_bytes(),
            content_digest.as_bytes(),
        ],
    ))
}

/// Derives the digest of a canonical external-artifact contract.
#[must_use]
pub fn derive_external_contract_digest(contract_bytes: &[u8]) -> ExternalContractDigest {
    ExternalContractDigest(hash_fold(D_EXTERNAL_CONTRACT, &[contract_bytes]))
}

/// Derives an external-artifact binding id.
#[must_use]
pub fn derive_external_binding_id(input: ExternalBindingIdentity<'_>) -> ExternalBindingId {
    ExternalBindingId(hash_fold(
        D_EXTERNAL,
        &[
            &input.scope_kind.to_le_bytes(),
            input.scope_id,
            input.artifact_sha256.as_bytes(),
            &input.artifact_length.to_le_bytes(),
            &input.certified_rdf_digest,
            input.contract_digest.as_bytes(),
        ],
    ))
}

/// Derives the digest of a canonical opaque-index guard.
#[must_use]
pub fn derive_index_guard_digest(guard_bytes: &[u8]) -> IndexGuardDigest {
    IndexGuardDigest(hash_fold(D_INDEX_GUARD, &[guard_bytes]))
}

/// Derives an opaque derived-index id.
#[must_use]
pub fn derive_index_id(input: IndexIdentity) -> IndexId {
    IndexId(hash_fold(
        D_INDEX,
        &[
            input.source_exact_digest.as_bytes(),
            input.family_id.as_bytes(),
            input.vector_space_id.as_bytes(),
            input.matrix_id.as_bytes(),
            input.projection_id.as_bytes(),
            input.target_set_id.as_bytes(),
            &input.prefix_dimension.to_le_bytes(),
            input.payload_sha256.as_bytes(),
            &input.payload_length.to_le_bytes(),
            &input.determinism.to_le_bytes(),
            input.guard_digest.as_bytes(),
        ],
    ))
}

/// Derives the whole-artifact root from the root-zeroed header and directory.
#[must_use]
pub fn derive_artifact_root(header_zero_root: &[u8], directory: &[u8]) -> ArtifactRoot {
    ArtifactRoot(hash_fold(D_ARTIFACT, &[header_zero_root, directory]))
}

fn hash_fold(domain: Domain, fields: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    for field in fields {
        let length = u64::try_from(field.len()).expect("an in-memory slice length fits u64");
        hasher.update(length.to_le_bytes());
        hasher.update(field);
    }
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_lengths_make_the_fold_unambiguous() {
        let left = hash_fold(D_TARGET, &[b"ab", b"c"]);
        let right = hash_fold(D_TARGET, &[b"a", b"bc"]);
        assert_ne!(left, right);
    }

    /// The identities the checked-in PURREMB golden does not exercise, frozen over fixed
    /// inputs: a relation role, an external contract and binding, an index guard and an
    /// index. Each is persisted in an artifact, so a moved value is a changed identity.
    #[test]
    fn the_identities_outside_the_artifact_golden_are_frozen() {
        let hex = |bytes: &[u8]| purrdf_hash::hex::Lower(bytes).to_string();
        assert_eq!(
            hex(&derive_relation_role_digest(b"https://example.org/role")),
            "6f0a7fcae396ad75e599ecea1c4807948e480aba63594c4a51c3bfc636bdfca6"
        );
        let contract = derive_external_contract_digest(b"external contract");
        assert_eq!(
            hex(contract.as_bytes()),
            "998bc47560e841615d067c98576b34459cfe426d7a942a71bc082ab7339f99e5"
        );
        let binding = derive_external_binding_id(ExternalBindingIdentity {
            scope_kind: 1,
            scope_id: &[0x31; 32],
            artifact_sha256: ContentDigest::of(b"external artifact"),
            artifact_length: 17,
            certified_rdf_digest: [0x32; 32],
            contract_digest: contract,
        });
        assert_eq!(
            hex(binding.as_bytes()),
            "56a18d2f143ea8c61f410986fb2344c652b5687b04dfca9e301cc96afd618c73"
        );
        let guard = derive_index_guard_digest(b"index guard");
        assert_eq!(
            hex(guard.as_bytes()),
            "3eb1ad8a271b1a739b32cb3d4a47fa60064788d285275408c48ecde9106cd2df"
        );
        let index = derive_index_id(IndexIdentity {
            source_exact_digest: ContentDigest::of(b"source"),
            family_id: FamilyId::from_raw([0x41; 32]),
            vector_space_id: VectorSpaceId::from_raw([0x42; 32]),
            matrix_id: MatrixId::from_raw([0x43; 32]),
            projection_id: ProjectionId::from_raw([0x44; 32]),
            target_set_id: TargetSetId::from_raw([0x45; 32]),
            prefix_dimension: 8,
            payload_sha256: ContentDigest::of(b"payload"),
            payload_length: 7,
            determinism: 1,
            guard_digest: guard,
        });
        assert_eq!(
            hex(index.as_bytes()),
            "acf422a8377bbd072aa3ab4f033e961006a0d24c16961d9c487e6f5663f37673"
        );
    }

    #[test]
    fn domains_separate_equal_payloads() {
        let payload = [0x42; 32];
        let family = derive_family_id(FamilyContractDigest::from_raw(payload));
        let target = derive_target_id(1, TargetIdentityDigest::from_raw(payload));
        assert_ne!(family.as_bytes(), target.as_bytes());
    }

    #[test]
    fn target_set_order_is_identity_significant() {
        let a = TargetId::from_raw([1; 32]);
        let b = TargetId::from_raw([2; 32]);
        assert_ne!(derive_target_set_id(&[a, b]), derive_target_set_id(&[b, a]));
    }

    #[test]
    fn vector_space_binds_dimension_and_postprocessing() {
        let family = FamilyId::from_raw([7; 32]);
        let raw = derive_vector_space_id(family, 256, 0);
        let normalized = derive_vector_space_id(family, 256, 1);
        let full = derive_vector_space_id(family, 768, 0);
        assert_ne!(raw, normalized);
        assert_ne!(raw, full);
    }

    #[test]
    fn identity_hex_is_fixed_width_lowercase() {
        let id = MatrixId::from_raw([0xab; 32]);
        assert_eq!(id.to_hex(), "ab".repeat(32));
        assert_eq!(id.to_string(), id.to_hex());
    }

    #[test]
    fn artifact_root_binds_header_and_directory() {
        let root = derive_artifact_root(&[0; 128], &[1; 64]);
        let changed_header = derive_artifact_root(&[1; 128], &[1; 64]);
        let changed_directory = derive_artifact_root(&[0; 128], &[2; 64]);
        assert_ne!(root, changed_header);
        assert_ne!(root, changed_directory);
    }
}

#[cfg(test)]
mod hex_differential {
    use super::FamilyId;

    /// The identity rendering against the frozen table.
    #[test]
    fn identity_rendering_matches_the_frozen_table() {
        for (input, lower, _) in crate::hex_frozen_vectors::encodings() {
            if input.len() != 32 {
                continue;
            }
            let id = FamilyId::from_raw(input.clone().try_into().expect("32 bytes"));
            assert_eq!(id.to_hex(), lower);
            assert_eq!(id.to_string(), lower);
            assert_eq!(format!("{id:?}"), format!("FamilyId({lower:?})"));
        }
    }
}
