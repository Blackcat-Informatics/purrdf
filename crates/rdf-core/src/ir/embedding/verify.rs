// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cryptographic, numerical, source-pack, and external-binding verification.

use std::collections::BTreeMap;

use sha2::{Digest as _, Sha256};

use crate::{
    ContentDigest, DatasetView, PackId, PackView, QuadIds, RdfDataset, TermId, TermRef, TermValue,
    dataset_from_view, try_canonicalize, verify_pack,
};

use super::contract::{PrefixPostprocessing, VectorDtype};
use super::error::{DigestKind, EmbeddingError};
use super::identity::{
    ExternalBindingIdentity, IndexIdentity, ProjectionContentDigest, RdfcDigest, TargetId,
    TargetSetId, derive_artifact_root, derive_external_binding_id, derive_external_contract_digest,
    derive_family_contract_digest, derive_family_id, derive_index_guard_digest, derive_index_id,
    derive_matrix_content_digest, derive_matrix_id, derive_projection_id,
    derive_relation_role_digest, derive_target_id, derive_target_identity_digest,
    derive_vector_space_id,
};
use super::target::{
    RdfAnnotationTarget, RdfDatasetTarget, RdfGraphTarget, RdfReifierTarget, RdfStatementTarget,
    RdfTermTarget, TargetKind,
};
use super::view::{EmbeddingView, ExternalBindingView, MatrixView, ProjectionView};
use super::wire::{PURREMB_DIRECTORY_ENTRY_LENGTH, PURREMB_HEADER_LENGTH};

const D_TARGET_SET: &[u8] = b"purrdf.purremb.v1.target-set\0";
const D_PROJECTION_CONTENT: &[u8] = b"purrdf.purremb.v1.projection-content\0";

/// Requested evidence level for an attached source pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceVerificationMode {
    /// Check exact bytes and structural `.purrpck` validity.
    Exact,
    /// Also independently certify RDF identity and every retained pack ordinal.
    Certified,
}

/// Evidence returned after verifying the attached source pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceVerificationReport {
    mode: SourceVerificationMode,
    exact_digest: ContentDigest,
    certified_rdf_digest: Option<RdfcDigest>,
    ordinals_checked: u64,
}

impl SourceVerificationReport {
    /// Evidence level that completed successfully.
    #[must_use]
    pub const fn mode(self) -> SourceVerificationMode {
        self.mode
    }

    /// Independently computed exact source SHA-256.
    #[must_use]
    pub const fn exact_digest(self) -> ContentDigest {
        self.exact_digest
    }

    /// Independently certified RDF digest, absent in exact-only mode.
    #[must_use]
    pub const fn certified_rdf_digest(self) -> Option<RdfcDigest> {
        self.certified_rdf_digest
    }

    /// Number of source-local ordinal hints cross-checked.
    #[must_use]
    pub const fn ordinals_checked(self) -> u64 {
        self.ordinals_checked
    }
}

/// Opaque proof that one exact resident byte range passed full verification.
///
/// The certificate cannot be constructed or deserialized by callers. It is
/// useful only while the same allocation and byte range remain immutable.
#[derive(Debug)]
pub struct ResidentEmbeddingCertificate {
    address: usize,
    length: usize,
    root: super::identity::ArtifactRoot,
}

impl ResidentEmbeddingCertificate {
    fn new(view: &EmbeddingView<'_>) -> Self {
        Self {
            address: view.bytes().as_ptr() as usize,
            length: view.bytes().len(),
            root: view.artifact_root(),
        }
    }

    fn matches(&self, bytes: &[u8], root: super::identity::ArtifactRoot) -> bool {
        self.address == bytes.as_ptr() as usize && self.length == bytes.len() && self.root == root
    }
}

/// Counts and resident proof produced by full artifact verification.
#[derive(Debug)]
pub struct EmbeddingVerificationReport {
    certificate: ResidentEmbeddingCertificate,
    section_count: usize,
    scalar_count: u64,
    projection_count: usize,
}

impl EmbeddingVerificationReport {
    /// Number of verified directory sections.
    #[must_use]
    pub const fn section_count(&self) -> usize {
        self.section_count
    }

    /// Number of finite authoritative matrix scalars scanned.
    #[must_use]
    pub const fn scalar_count(&self) -> u64 {
        self.scalar_count
    }

    /// Number of logical projection digests recomputed.
    #[must_use]
    pub const fn projection_count(&self) -> usize {
        self.projection_count
    }

    /// Borrows the resident reopen certificate.
    #[must_use]
    pub const fn certificate(&self) -> &ResidentEmbeddingCertificate {
        &self.certificate
    }

    /// Consumes the report and retains only its resident reopen certificate.
    #[must_use]
    pub fn into_certificate(self) -> ResidentEmbeddingCertificate {
        self.certificate
    }
}

/// Verifies every contained digest, identity, scalar, and logical projection.
///
/// Structural validation must already have succeeded through
/// [`EmbeddingView::from_bytes`]. On success, the supplied view is marked fully
/// verified and may expose aligned native scalar slices.
pub fn verify_embedding(
    view: &mut EmbeddingView<'_>,
) -> Result<EmbeddingVerificationReport, EmbeddingError> {
    verify_section_hashes_and_root(view)?;
    verify_family_identities(view)?;
    verify_target_identities(view)?;
    verify_target_sets(view)?;
    verify_relation_roles(view)?;
    view.verify_relation_completeness()?;
    verify_external_bindings(view)?;
    let scalar_count = verify_matrices_and_projections(view)?;
    verify_index_guards(view)?;
    verify_source_dataset_target(view)?;

    view.mark_fully_verified();
    Ok(EmbeddingVerificationReport {
        certificate: ResidentEmbeddingCertificate::new(view),
        section_count: view.section_count(),
        scalar_count,
        projection_count: view.projection_count(),
    })
}

/// Structurally reopens the same immutable resident bytes under a prior proof.
pub fn reopen_prevalidated<'a>(
    bytes: &'a [u8],
    certificate: &ResidentEmbeddingCertificate,
) -> Result<EmbeddingView<'a>, EmbeddingError> {
    let mut view = EmbeddingView::from_bytes(bytes)?;
    if !certificate.matches(bytes, view.artifact_root()) {
        return Err(EmbeddingError::CertificateMismatch);
    }
    view.mark_fully_verified();
    Ok(view)
}

/// Verifies the exact attached pack and, optionally, its certified RDF identity.
pub fn verify_embedding_source(
    embedding: &EmbeddingView<'_>,
    source_bytes: &[u8],
    mode: SourceVerificationMode,
) -> Result<SourceVerificationReport, EmbeddingError> {
    let source = embedding.source();
    let actual_length = u64::try_from(source_bytes.len())
        .map_err(|_| EmbeddingError::ArithmeticOverflow("source byte length"))?;
    if actual_length != source.source_length() {
        return Err(EmbeddingError::SourceLengthMismatch {
            expected: source.source_length(),
            actual: actual_length,
        });
    }
    let exact_digest = ContentDigest::of(source_bytes);
    compare_digest(
        DigestKind::SourceExact,
        source.source_exact_digest().as_bytes(),
        exact_digest.as_bytes(),
    )?;

    let pack = PackView::from_bytes(source_bytes)
        .map_err(|error| EmbeddingError::InvalidSourcePack(error.to_string()))?;
    if mode == SourceVerificationMode::Exact {
        return Ok(SourceVerificationReport {
            mode,
            exact_digest,
            certified_rdf_digest: None,
            ordinals_checked: 0,
        });
    }

    let certified = verify_pack(source_bytes)
        .map_err(|error| EmbeddingError::InvalidSourcePack(error.to_string()))?;
    compare_digest(
        DigestKind::CertifiedRdf,
        &source.certified_rdf_digest(),
        certified.as_bytes(),
    )?;
    verify_source_dataset_target(embedding)?;
    let ordinals_checked = verify_source_ordinals(embedding, &pack)?;
    Ok(SourceVerificationReport {
        mode,
        exact_digest,
        certified_rdf_digest: Some(RdfcDigest::from_raw(*certified.as_bytes())),
        ordinals_checked,
    })
}

/// Checks exact bytes against one generic external-artifact binding.
pub fn verify_external_artifact(
    binding: ExternalBindingView<'_>,
    bytes: &[u8],
) -> Result<ContentDigest, EmbeddingError> {
    let actual_length = u64::try_from(bytes.len())
        .map_err(|_| EmbeddingError::ArithmeticOverflow("external artifact byte length"))?;
    if actual_length != binding.artifact_length() {
        return Err(EmbeddingError::ExternalLengthMismatch {
            expected: binding.artifact_length(),
            actual: actual_length,
        });
    }
    let digest = ContentDigest::of(bytes);
    compare_digest(
        DigestKind::ExternalBinding,
        binding.artifact_sha256().as_bytes(),
        digest.as_bytes(),
    )?;
    Ok(digest)
}

/// Checks exact bytes and independently certifies a pack-backed external binding.
pub fn verify_external_pack(
    binding: ExternalBindingView<'_>,
    bytes: &[u8],
) -> Result<RdfcDigest, EmbeddingError> {
    verify_external_artifact(binding, bytes)?;
    let expected = binding
        .certified_rdf_digest()
        .ok_or(EmbeddingError::Missing("external certified RDF digest"))?;
    let certified = verify_pack(bytes)
        .map_err(|error| EmbeddingError::InvalidExternalPack(error.to_string()))?;
    compare_digest(DigestKind::CertifiedRdf, &expected, certified.as_bytes())?;
    Ok(RdfcDigest::from_raw(*certified.as_bytes()))
}

/// Rejects a comparison across distinct effective vector spaces.
pub fn require_compatible_vector_spaces(
    left: super::identity::VectorSpaceId,
    right: super::identity::VectorSpaceId,
) -> Result<(), EmbeddingError> {
    if left == right {
        Ok(())
    } else {
        Err(EmbeddingError::IncompatibleVectorSpaces)
    }
}

fn verify_section_hashes_and_root(view: &EmbeddingView<'_>) -> Result<(), EmbeddingError> {
    for section in view.sections() {
        let actual: [u8; 32] = Sha256::digest(section.bytes()).into();
        compare_digest(DigestKind::Section, &section.stored_sha256(), &actual)?;
    }

    let mut header = [0u8; PURREMB_HEADER_LENGTH as usize];
    header.copy_from_slice(
        view.bytes()
            .get(..PURREMB_HEADER_LENGTH as usize)
            .ok_or(EmbeddingError::Truncated)?,
    );
    header[64..96].fill(0);
    let directory_length = view
        .section_count()
        .checked_mul(PURREMB_DIRECTORY_ENTRY_LENGTH as usize)
        .ok_or(EmbeddingError::ArithmeticOverflow("directory byte length"))?;
    let directory_end = (PURREMB_HEADER_LENGTH as usize)
        .checked_add(directory_length)
        .ok_or(EmbeddingError::ArithmeticOverflow("directory end"))?;
    let directory = view
        .bytes()
        .get(PURREMB_HEADER_LENGTH as usize..directory_end)
        .ok_or(EmbeddingError::Truncated)?;
    let actual = derive_artifact_root(&header, directory);
    compare_digest(
        DigestKind::ArtifactRoot,
        view.artifact_root().as_bytes(),
        actual.as_bytes(),
    )
}

fn verify_family_identities(view: &EmbeddingView<'_>) -> Result<(), EmbeddingError> {
    for family in view.families() {
        let digest = derive_family_contract_digest(family.contract_bytes());
        compare_digest(
            DigestKind::Contract,
            family.contract_digest().as_bytes(),
            digest.as_bytes(),
        )?;
        let id = derive_family_id(digest);
        compare_digest(DigestKind::Contract, family.id().as_bytes(), id.as_bytes())?;
        for space in family.spaces() {
            let actual =
                derive_vector_space_id(id, space.dimension(), space.postprocessing()?.code());
            compare_digest(
                DigestKind::Contract,
                space.id().as_bytes(),
                actual.as_bytes(),
            )?;
        }
    }
    Ok(())
}

fn verify_target_identities(view: &EmbeddingView<'_>) -> Result<(), EmbeddingError> {
    for target in view.targets() {
        let kind = target.kind()?.code();
        if let Some(identity) = target.identity_bytes() {
            let digest = derive_target_identity_digest(kind, identity);
            compare_digest(
                DigestKind::Target,
                target.identity_digest().as_bytes(),
                digest.as_bytes(),
            )?;
        }
        let id = derive_target_id(kind, target.identity_digest());
        compare_digest(DigestKind::Target, target.id().as_bytes(), id.as_bytes())?;
    }
    Ok(())
}

fn verify_target_sets(view: &EmbeddingView<'_>) -> Result<(), EmbeddingError> {
    for set in view.target_sets() {
        let row_count = u64::try_from(set.row_count())
            .map_err(|_| EmbeddingError::ArithmeticOverflow("target-set row count"))?;
        let mut hasher = FramedHasher::new(D_TARGET_SET);
        hasher.field(&row_count.to_le_bytes());
        for target in set.targets() {
            hasher.field(target.as_bytes());
        }
        let actual = TargetSetId::from_raw(hasher.finish());
        compare_digest(
            DigestKind::TargetSet,
            set.id().as_bytes(),
            actual.as_bytes(),
        )?;
    }
    Ok(())
}

fn verify_relation_roles(view: &EmbeddingView<'_>) -> Result<(), EmbeddingError> {
    for relation in view.relations() {
        let actual = relation
            .role_bytes()
            .map_or([0; 32], derive_relation_role_digest);
        compare_digest(DigestKind::Contract, &relation.role_digest(), &actual)?;
    }
    Ok(())
}

fn verify_external_bindings(view: &EmbeddingView<'_>) -> Result<(), EmbeddingError> {
    for binding in view.external_bindings() {
        let contract_digest = derive_external_contract_digest(binding.contract_bytes());
        compare_digest(
            DigestKind::ExternalBinding,
            binding.contract_digest().as_bytes(),
            contract_digest.as_bytes(),
        )?;
        let certified = binding.certified_rdf_digest().unwrap_or([0; 32]);
        let scope_id = binding.scope_id();
        let actual = derive_external_binding_id(ExternalBindingIdentity {
            scope_kind: binding.scope_kind()? as u32,
            scope_id: &scope_id,
            artifact_sha256: binding.artifact_sha256(),
            artifact_length: binding.artifact_length(),
            certified_rdf_digest: certified,
            contract_digest,
        });
        compare_digest(
            DigestKind::ExternalBinding,
            binding.id().as_bytes(),
            actual.as_bytes(),
        )?;
    }
    Ok(())
}

fn verify_matrices_and_projections(view: &EmbeddingView<'_>) -> Result<u64, EmbeddingError> {
    let mut scalar_count = 0u64;
    for matrix in view.matrices() {
        scalar_count = scalar_count
            .checked_add(scan_matrix_scalars(matrix)?)
            .ok_or(EmbeddingError::ArithmeticOverflow("verified scalar count"))?;
        let content = derive_matrix_content_digest(
            matrix.dtype()?.code(),
            matrix.row_count(),
            matrix.stored_dimension(),
            matrix.data_bytes(),
        );
        compare_digest(
            DigestKind::Matrix,
            matrix.content_digest().as_bytes(),
            content.as_bytes(),
        )?;
        let id = derive_matrix_id(matrix.target_set_id(), matrix.family_id(), content);
        compare_digest(DigestKind::Matrix, matrix.id().as_bytes(), id.as_bytes())?;
    }

    for projection in view.projections() {
        verify_projection(view, projection)?;
    }
    Ok(scalar_count)
}

fn scan_matrix_scalars(matrix: MatrixView<'_>) -> Result<u64, EmbeddingError> {
    for row in 0..matrix.row_count() {
        match matrix.dtype()? {
            VectorDtype::F32 => {
                for value in matrix.f32_row(row)? {
                    value?;
                }
            }
            VectorDtype::F64 => {
                for value in matrix.f64_row(row)? {
                    value?;
                }
            }
        }
    }
    matrix
        .row_count()
        .checked_mul(u64::from(matrix.stored_dimension()))
        .ok_or(EmbeddingError::ArithmeticOverflow("matrix scalar count"))
}

fn verify_projection(
    view: &EmbeddingView<'_>,
    projection: ProjectionView<'_>,
) -> Result<(), EmbeddingError> {
    let matrix = view
        .matrix(projection.matrix_id())
        .ok_or(EmbeddingError::MissingReference("projection matrix"))?;
    let effective = view
        .effective_matrix(matrix.target_set_id(), projection.vector_space_id())?
        .ok_or(EmbeddingError::MissingReference(
            "effective matrix projection",
        ))?;
    if effective.projection().id() != projection.id() {
        return Err(EmbeddingError::Duplicate(
            "target-set/vector-space projection",
        ));
    }

    let postprocessing = projection.postprocessing()?;
    let mut hasher = projection_hasher(matrix, projection, postprocessing)?;
    for row in 0..matrix.row_count() {
        match matrix.dtype()? {
            VectorDtype::F32 => {
                for value in effective.f32_row(row)? {
                    hasher.update(&value?.to_le_bytes());
                }
            }
            VectorDtype::F64 => {
                for value in effective.f64_row(row)? {
                    hasher.update(&value?.to_le_bytes());
                }
            }
        }
    }
    let content = ProjectionContentDigest::from_raw(hasher.finish());
    compare_digest(
        DigestKind::Projection,
        projection.content_digest().as_bytes(),
        content.as_bytes(),
    )?;
    let id = derive_projection_id(matrix.id(), projection.vector_space_id(), content);
    compare_digest(
        DigestKind::Projection,
        projection.id().as_bytes(),
        id.as_bytes(),
    )
}

fn projection_hasher(
    matrix: MatrixView<'_>,
    projection: ProjectionView<'_>,
    postprocessing: PrefixPostprocessing,
) -> Result<FramedHasher, EmbeddingError> {
    let mut hasher = FramedHasher::new(D_PROJECTION_CONTENT);
    hasher.field(&matrix.dtype()?.code().to_le_bytes());
    hasher.field(&matrix.row_count().to_le_bytes());
    hasher.field(&projection.effective_dimension().to_le_bytes());
    hasher.field(&postprocessing.code().to_le_bytes());
    hasher.begin_field(projection.logical_byte_length());
    Ok(hasher)
}

fn verify_index_guards(view: &EmbeddingView<'_>) -> Result<(), EmbeddingError> {
    for index in view.index_guards() {
        if let Some(payload) = index.payload_bytes() {
            let digest = ContentDigest::of(payload);
            compare_digest(
                DigestKind::Index,
                index.payload_sha256().as_bytes(),
                digest.as_bytes(),
            )?;
        }
        let guard_digest = derive_index_guard_digest(index.guard_bytes());
        compare_digest(
            DigestKind::Index,
            index.guard_digest().as_bytes(),
            guard_digest.as_bytes(),
        )?;
        let actual = derive_index_id(IndexIdentity {
            source_exact_digest: index.source_exact_digest(),
            family_id: index.family_id(),
            vector_space_id: index.vector_space_id(),
            matrix_id: index.matrix_id(),
            projection_id: index.projection_id(),
            target_set_id: index.target_set_id(),
            prefix_dimension: index.prefix_dimension(),
            payload_sha256: index.payload_sha256(),
            payload_length: index.payload_length(),
            determinism: index.determinism()? as u32,
            guard_digest,
        });
        compare_digest(DigestKind::Index, index.id().as_bytes(), actual.as_bytes())?;
    }
    Ok(())
}

fn verify_source_dataset_target(view: &EmbeddingView<'_>) -> Result<(), EmbeddingError> {
    let source = view.source();
    let expected = RdfDatasetTarget {
        rdfc_digest: source.certified_rdf_digest(),
    }
    .into_target(false)?;
    compare_digest(
        DigestKind::Target,
        source.dataset_target_id().as_bytes(),
        expected.id.as_bytes(),
    )?;
    let target = view
        .target(source.dataset_target_id())
        .ok_or(EmbeddingError::MissingReference("SOURCE dataset target"))?;
    if target.kind()? != TargetKind::RdfDataset {
        return Err(EmbeddingError::Malformed(
            "SOURCE dataset target has the wrong kind",
        ));
    }
    Ok(())
}

fn verify_source_ordinals(
    embedding: &EmbeddingView<'_>,
    pack: &PackView<'_>,
) -> Result<u64, EmbeddingError> {
    let dataset = dataset_from_view(pack)
        .map_err(|error| EmbeddingError::InvalidSourcePack(error.to_string()))?;
    let canonical = try_canonicalize(&dataset).map_err(|_| {
        EmbeddingError::InvalidSourcePack("RDFC canonicalization budget exceeded".to_owned())
    })?;
    let dataset_target = embedding.source().dataset_target_id();
    let mut checked = 0u64;

    for target in embedding.targets() {
        let Some(ordinal) = target.source_local_ordinal() else {
            continue;
        };
        let expected = target_from_source_ordinal(
            target.kind()?,
            ordinal,
            pack,
            &dataset,
            &canonical.labels,
            dataset_target,
        )?;
        if expected != target.id() {
            return Err(EmbeddingError::OrdinalMismatch {
                target_kind: target.kind()?.code(),
                ordinal,
            });
        }
        checked = checked
            .checked_add(1)
            .ok_or(EmbeddingError::ArithmeticOverflow("verified ordinal count"))?;
    }
    Ok(checked)
}

fn target_from_source_ordinal(
    kind: TargetKind,
    ordinal: u64,
    pack: &PackView<'_>,
    dataset: &RdfDataset,
    labels: &BTreeMap<TermId, Box<str>>,
    dataset_target: TargetId,
) -> Result<TargetId, EmbeddingError> {
    let mismatch = || EmbeddingError::OrdinalMismatch {
        target_kind: kind.code(),
        ordinal,
    };
    match kind {
        TargetKind::RdfTerm => {
            if ordinal == 0 || ordinal > pack.dict().n_terms() {
                return Err(mismatch());
            }
            term_target_id(pack, ordinal, dataset, labels, dataset_target)
        }
        TargetKind::RdfStatement => {
            let index = usize::try_from(ordinal).map_err(|_| mismatch())?;
            let quad = pack.quads().nth(index).ok_or_else(mismatch)?;
            statement_target_id(pack, quad, dataset, labels, dataset_target)
        }
        TargetKind::RdfReifier => {
            let index = usize::try_from(ordinal).map_err(|_| mismatch())?;
            let quad = pack.reifier_quads().nth(index).ok_or_else(mismatch)?;
            reifier_target_id(pack, quad, dataset, labels, dataset_target)
        }
        TargetKind::RdfAnnotation => {
            let index = usize::try_from(ordinal).map_err(|_| mismatch())?;
            let quad = pack.annotation_quads().nth(index).ok_or_else(mismatch)?;
            annotation_target_id(pack, quad, dataset, labels, dataset_target)
        }
        _ => Err(mismatch()),
    }
}

fn term_target_id(
    pack: &PackView<'_>,
    id: u64,
    dataset: &RdfDataset,
    labels: &BTreeMap<TermId, Box<str>>,
    dataset_target: TargetId,
) -> Result<TargetId, EmbeddingError> {
    if id == 0 || id > pack.dict().n_terms() {
        return Err(EmbeddingError::MissingReference("source pack term ordinal"));
    }
    let value = pack.dict().term_value(id);
    term_value_target_id(&value, dataset, labels, dataset_target)
}

fn term_value_target_id(
    value: &TermValue,
    dataset: &RdfDataset,
    labels: &BTreeMap<TermId, Box<str>>,
    dataset_target: TargetId,
) -> Result<TargetId, EmbeddingError> {
    let target = match value {
        TermValue::Iri(iri) => RdfTermTarget::Iri(iri.clone()),
        TermValue::Blank { .. } => {
            let id = dataset
                .term_id_by_value(value)
                .ok_or(EmbeddingError::MissingReference("reconstructed blank term"))?;
            let label = labels
                .get(&id)
                .ok_or(EmbeddingError::MissingReference("canonical blank label"))?;
            RdfTermTarget::Blank {
                dataset_id: dataset_target,
                canonical_label: label.to_string(),
            }
        }
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => RdfTermTarget::Literal {
            lexical: lexical_form.clone(),
            datatype: datatype.clone(),
            language: language.clone(),
            direction: *direction,
        },
        TermValue::Triple { s, p, o } => RdfTermTarget::Triple {
            subject: term_value_target_id(s, dataset, labels, dataset_target)?,
            predicate: term_value_target_id(p, dataset, labels, dataset_target)?,
            object: term_value_target_id(o, dataset, labels, dataset_target)?,
        },
    };
    Ok(target.into_target(false, None)?.id)
}

fn graph_target_id(
    pack: &PackView<'_>,
    graph: Option<PackId>,
    dataset: &RdfDataset,
    labels: &BTreeMap<TermId, Box<str>>,
    dataset_target: TargetId,
) -> Result<TargetId, EmbeddingError> {
    let graph_name = graph
        .map(|id| term_target_id(pack, id.get(), dataset, labels, dataset_target))
        .transpose()?;
    Ok(RdfGraphTarget {
        dataset_id: dataset_target,
        graph_name,
    }
    .into_target(false)?
    .id)
}

fn statement_target_id(
    pack: &PackView<'_>,
    quad: QuadIds<PackId>,
    dataset: &RdfDataset,
    labels: &BTreeMap<TermId, Box<str>>,
    dataset_target: TargetId,
) -> Result<TargetId, EmbeddingError> {
    Ok(RdfStatementTarget {
        graph: graph_target_id(pack, quad.g, dataset, labels, dataset_target)?,
        subject: term_target_id(pack, quad.s.get(), dataset, labels, dataset_target)?,
        predicate: term_target_id(pack, quad.p.get(), dataset, labels, dataset_target)?,
        object: term_target_id(pack, quad.o.get(), dataset, labels, dataset_target)?,
    }
    .into_target(false, None)?
    .id)
}

fn statement_from_triple_target_id(
    pack: &PackView<'_>,
    triple: PackId,
    graph: Option<PackId>,
    dataset: &RdfDataset,
    labels: &BTreeMap<TermId, Box<str>>,
    dataset_target: TargetId,
) -> Result<TargetId, EmbeddingError> {
    let TermRef::Triple { s, p, o } = pack.resolve(triple) else {
        return Err(EmbeddingError::Malformed(
            "source reifier does not reference a triple term",
        ));
    };
    statement_target_id(
        pack,
        QuadIds { s, p, o, g: graph },
        dataset,
        labels,
        dataset_target,
    )
}

fn reifier_target_id(
    pack: &PackView<'_>,
    quad: QuadIds<PackId>,
    dataset: &RdfDataset,
    labels: &BTreeMap<TermId, Box<str>>,
    dataset_target: TargetId,
) -> Result<TargetId, EmbeddingError> {
    Ok(RdfReifierTarget {
        graph: graph_target_id(pack, quad.g, dataset, labels, dataset_target)?,
        statement: statement_from_triple_target_id(
            pack,
            quad.o,
            quad.g,
            dataset,
            labels,
            dataset_target,
        )?,
        reifier: term_target_id(pack, quad.s.get(), dataset, labels, dataset_target)?,
    }
    .into_target(false, None)?
    .id)
}

fn annotation_target_id(
    pack: &PackView<'_>,
    quad: QuadIds<PackId>,
    dataset: &RdfDataset,
    labels: &BTreeMap<TermId, Box<str>>,
    dataset_target: TargetId,
) -> Result<TargetId, EmbeddingError> {
    let mut binding = None;
    for candidate in pack.reifier_quads() {
        if candidate.s == quad.s && candidate.g == quad.g {
            if binding.is_some() {
                return Err(EmbeddingError::Malformed(
                    "annotation reifier has multiple bindings in one graph",
                ));
            }
            binding = Some(candidate);
        }
    }
    let binding = binding.ok_or(EmbeddingError::MissingReference(
        "annotation reifier binding",
    ))?;
    Ok(RdfAnnotationTarget {
        graph: graph_target_id(pack, quad.g, dataset, labels, dataset_target)?,
        reifier: reifier_target_id(pack, binding, dataset, labels, dataset_target)?,
        predicate: term_target_id(pack, quad.p.get(), dataset, labels, dataset_target)?,
        object: term_target_id(pack, quad.o.get(), dataset, labels, dataset_target)?,
    }
    .into_target(false, None)?
    .id)
}

fn compare_digest(
    kind: DigestKind,
    expected: &[u8; 32],
    actual: &[u8; 32],
) -> Result<(), EmbeddingError> {
    if expected == actual {
        Ok(())
    } else {
        Err(EmbeddingError::DigestMismatch {
            kind,
            expected: *expected,
            actual: *actual,
        })
    }
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
        self.begin_field(u64::try_from(bytes.len()).expect("an in-memory slice length fits u64"));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_space_compatibility_uses_identity_not_dimension() {
        let left = super::super::identity::VectorSpaceId::from_raw([1; 32]);
        let right = super::super::identity::VectorSpaceId::from_raw([2; 32]);
        require_compatible_vector_spaces(left, left).expect("equal identities compare");
        assert_eq!(
            require_compatible_vector_spaces(left, right),
            Err(EmbeddingError::IncompatibleVectorSpaces)
        );
    }

    #[test]
    fn resident_certificate_binds_allocation_range_and_root() {
        let bytes = [1u8, 2, 3];
        let certificate = ResidentEmbeddingCertificate {
            address: bytes.as_ptr() as usize,
            length: bytes.len(),
            root: super::super::identity::ArtifactRoot::from_raw([9; 32]),
        };
        assert!(certificate.matches(
            &bytes,
            super::super::identity::ArtifactRoot::from_raw([9; 32])
        ));
        assert!(!certificate.matches(
            &bytes[1..],
            super::super::identity::ArtifactRoot::from_raw([9; 32])
        ));
    }
}
