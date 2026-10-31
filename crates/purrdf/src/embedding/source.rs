// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Full native source attachments and external pack verification.

use super::error::SourceAttachmentError;
use purrdf_columnar::{ColumnarOrdinal, ColumnarOrdinalRead, ColumnarOrdinalValue, ParquetFiles};
use purrdf_core::embedding::{
    DigestKind, EmbeddingError, EmbeddingSourceFormat, EmbeddingView, ExternalBinding,
    ExternalBindingContract, ExternalBindingView, ExternalScope, RdfAnnotationTarget,
    RdfGraphTarget, RdfReifierTarget, RdfStatementTarget, RdfcDigest, TargetId, TargetKind,
    TargetView, term_value_target_id, verify_embedding_dataset_target, verify_external_artifact,
};
use purrdf_core::{
    Canonicalized, ContentDigest, DatasetView, PackId, PackView, QuadIds, RdfDataset, TermId,
    TermRef, dataset_from_view, try_canonicalize, verify_pack_bytes,
};
use std::collections::BTreeMap;

/// Actual untrusted bytes supplied for one of the two closed native carriers.
#[derive(Clone, Copy)]
pub enum EmbeddingSource<'a> {
    /// One complete exact native pack.
    Purrpck(&'a [u8]),
    /// All five actual Columnar files, including valid zero-row files.
    Columnar(&'a ParquetFiles),
}

purrdf_hash::debug_non_exhaustive!(EmbeddingSource<'_> {});

impl EmbeddingSource<'_> {
    /// The actual supplied carrier.
    #[must_use]
    pub const fn source_format(self) -> EmbeddingSourceFormat {
        match self {
            Self::Purrpck(_) => EmbeddingSourceFormat::PurrpckV1,
            Self::Columnar(_) => EmbeddingSourceFormat::ColumnarV1,
        }
    }
}

/// Evidence returned after complete exact-source, RDF and ordinal verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceVerificationReport {
    source_format: EmbeddingSourceFormat,
    exact_digest: ContentDigest,
    certified_rdf_digest: RdfcDigest,
    ordinals_checked: u64,
}
impl SourceVerificationReport {
    /// The independently verified native carrier.
    #[must_use]
    pub const fn source_format(self) -> EmbeddingSourceFormat {
        self.source_format
    }
    /// Independently computed exact source SHA-256.
    #[must_use]
    pub const fn exact_digest(self) -> ContentDigest {
        self.exact_digest
    }
    /// Independently certified RDF identity.
    #[must_use]
    pub const fn certified_rdf_digest(self) -> RdfcDigest {
        self.certified_rdf_digest
    }
    /// Number of source-local ordinal hints checked against canonical targets.
    #[must_use]
    pub const fn ordinals_checked(self) -> u64 {
        self.ordinals_checked
    }
}

/// Verify exact native source bytes, independent RDF identity and every ordinal.
/// Columnar manifests are derived from actual files; callers supply no claims.
pub fn verify_embedding_source(
    embedding: &EmbeddingView<'_>,
    input: EmbeddingSource<'_>,
    control: &mut purrdf_core::CanonicalizationControl<'_>,
) -> Result<SourceVerificationReport, SourceAttachmentError> {
    let source_format = input.source_format();
    let expected = embedding.source().source_format();
    if source_format != expected {
        return Err(SourceAttachmentError::FormatMismatch {
            expected,
            actual: source_format,
        });
    }
    let (exact_digest, certified_rdf_digest, ordinals_checked) = match input {
        EmbeddingSource::Purrpck(bytes) => {
            let exact = verify_exact_source(embedding, bytes)?;
            let proof = verify_pack_bytes(bytes, control).map_err(EmbeddingError::source_pack)?;
            let rdf = RdfcDigest::from_raw(*proof.rdf_digest().as_bytes());
            verify_rdf_source(embedding, rdf)?;
            let checked = verify_pack_ordinals(embedding, bytes, control)?;
            (exact, rdf, checked)
        }
        EmbeddingSource::Columnar(files) => {
            let manifest = purrdf_columnar::source_manifest(files)?;
            let exact = verify_exact_source(embedding, &manifest)?;
            let requested = embedding
                .targets()
                .filter_map(|target| {
                    target
                        .source_local_ordinal()
                        .map(|ordinal| columnar_ordinal(target, ordinal))
                })
                .collect::<Result<Vec<_>, _>>()?;
            // This shared strict path validates every table and blob, retaining
            // only requested original row evidence bound to its actual dataset.
            let read = purrdf_columnar::read_with_ordinals(files, &requested)?;
            let dataset = &read.read().dataset;
            let (rdf, canonical) = canonical_source(dataset, control)?;
            verify_rdf_source(embedding, rdf)?;
            let checked = verify_ordinals(embedding, |target, ordinal| {
                columnar_target_id(
                    target,
                    ordinal,
                    &read,
                    &canonical.labels,
                    embedding.source().dataset_target_id(),
                )
            })?;
            (exact, rdf, checked)
        }
    };
    Ok(SourceVerificationReport {
        source_format,
        exact_digest,
        certified_rdf_digest,
        ordinals_checked,
    })
}

fn verify_exact_source(
    embedding: &EmbeddingView<'_>,
    bytes: &[u8],
) -> Result<ContentDigest, EmbeddingError> {
    let source = embedding.source();
    let actual = u64::try_from(bytes.len())
        .map_err(|_| EmbeddingError::ArithmeticOverflow("source byte length"))?;
    if actual != source.source_length() {
        return Err(EmbeddingError::SourceLengthMismatch {
            expected: source.source_length(),
            actual,
        });
    }
    let digest = ContentDigest::of(bytes);
    DigestKind::SourceExact.verify(source.source_exact_digest().as_bytes(), digest.as_bytes())?;
    Ok(digest)
}

fn verify_rdf_source(
    embedding: &EmbeddingView<'_>,
    digest: RdfcDigest,
) -> Result<(), EmbeddingError> {
    DigestKind::CertifiedRdf.verify(
        &embedding.source().certified_rdf_digest(),
        digest.as_bytes(),
    )?;
    verify_embedding_dataset_target(embedding)
}

pub(super) fn canonical_source(
    dataset: &RdfDataset,
    control: &mut purrdf_core::CanonicalizationControl<'_>,
) -> Result<(RdfcDigest, Canonicalized), SourceAttachmentError> {
    let (canonical, digest) = purrdf_core::try_canonicalize_sha256_view(
        dataset,
        purrdf_core::CanonHash::Sha256,
        control,
    )?;
    let digest = RdfcDigest::from_raw(*digest.as_bytes());
    Ok((digest, canonical))
}

fn verify_pack_ordinals(
    embedding: &EmbeddingView<'_>,
    bytes: &[u8],
    control: &mut purrdf_core::CanonicalizationControl<'_>,
) -> Result<u64, EmbeddingError> {
    if embedding
        .targets()
        .all(|target| target.source_local_ordinal().is_none())
    {
        return Ok(0);
    }
    let pack = PackView::from_bytes(bytes).map_err(EmbeddingError::source_pack)?;
    let dataset = dataset_from_view(&pack).map_err(EmbeddingError::source_pack)?;
    let canonical = try_canonicalize(&dataset, control).map_err(EmbeddingError::source_pack)?;
    verify_ordinals(embedding, |target, ordinal| {
        pack_target_id(
            target,
            ordinal,
            &pack,
            &dataset,
            &canonical.labels,
            embedding.source().dataset_target_id(),
        )
    })
}

fn verify_ordinals<'a>(
    embedding: &EmbeddingView<'a>,
    mut resolve: impl FnMut(TargetView<'a>, u64) -> Result<TargetId, EmbeddingError>,
) -> Result<u64, EmbeddingError> {
    let mut checked = 0_u64;
    for target in embedding.targets() {
        let Some(ordinal) = target.source_local_ordinal() else {
            continue;
        };
        if resolve(target, ordinal)? != target.id() {
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

fn columnar_ordinal(
    target: TargetView<'_>,
    ordinal: u64,
) -> Result<ColumnarOrdinal, EmbeddingError> {
    Ok(match target.kind()? {
        TargetKind::RdfTerm => ColumnarOrdinal::Term(ordinal),
        TargetKind::RdfStatement => ColumnarOrdinal::Quad(ordinal),
        TargetKind::RdfReifier => ColumnarOrdinal::Reifier(ordinal),
        TargetKind::RdfAnnotation => ColumnarOrdinal::Annotation(ordinal),
        kind => {
            return Err(EmbeddingError::OrdinalMismatch {
                target_kind: kind.code(),
                ordinal,
            });
        }
    })
}

fn columnar_target_id(
    target: TargetView<'_>,
    ordinal: u64,
    read: &ColumnarOrdinalRead,
    labels: &BTreeMap<TermId, Box<str>>,
    dataset_target: TargetId,
) -> Result<TargetId, EmbeddingError> {
    let dataset = &read.read().dataset;
    let value = read
        .get(columnar_ordinal(target, ordinal)?)
        .ok_or(EmbeddingError::MissingReference("captured source ordinal"))?;
    match value {
        ColumnarOrdinalValue::Term(id) => term_target_id(id, dataset, labels, dataset_target),
        ColumnarOrdinalValue::Quad(quad) => {
            statement_target_id(quad, dataset, labels, dataset_target)
        }
        ColumnarOrdinalValue::Reifier {
            reifier,
            triple,
            graph,
        } => reifier_target_id(reifier, triple, graph, dataset, labels, dataset_target),
        ColumnarOrdinalValue::Annotation(quad) => {
            annotation_target_id(quad, dataset, labels, dataset_target, target.id(), ordinal)
        }
    }
}

fn pack_target_id(
    target: TargetView<'_>,
    ordinal: u64,
    pack: &PackView<'_>,
    dataset: &RdfDataset,
    labels: &BTreeMap<TermId, Box<str>>,
    dataset_target: TargetId,
) -> Result<TargetId, EmbeddingError> {
    let kind = target.kind()?;
    let mismatch = || EmbeddingError::OrdinalMismatch {
        target_kind: kind.code(),
        ordinal,
    };
    match kind {
        TargetKind::RdfTerm => {
            if ordinal == 0 || ordinal > pack.dict().n_terms() {
                return Err(mismatch());
            }
            // A pack dictionary may retain an unreferenced scalar term. Its
            // original value is authoritative even when reconstruction omits it.
            term_value_target_id(
                &pack.dict().term_value(ordinal),
                dataset,
                labels,
                dataset_target,
            )
        }
        TargetKind::RdfStatement => {
            let row = usize::try_from(ordinal).map_err(|_| mismatch())?;
            let quad = pack.quads().nth(row).ok_or_else(mismatch)?;
            statement_target_id(
                pack_quad(pack, quad, dataset)?,
                dataset,
                labels,
                dataset_target,
            )
        }
        TargetKind::RdfReifier => {
            let row = usize::try_from(ordinal).map_err(|_| mismatch())?;
            let quad = pack.reifier_quads().nth(row).ok_or_else(mismatch)?;
            reifier_target_id(
                pack_term(pack, quad.s, dataset)?,
                pack_term(pack, quad.o, dataset)?,
                quad.g.map(|id| pack_term(pack, id, dataset)).transpose()?,
                dataset,
                labels,
                dataset_target,
            )
        }
        TargetKind::RdfAnnotation => {
            let row = usize::try_from(ordinal).map_err(|_| mismatch())?;
            let quad = pack.annotation_quads().nth(row).ok_or_else(mismatch)?;
            annotation_target_id(
                pack_quad(pack, quad, dataset)?,
                dataset,
                labels,
                dataset_target,
                target.id(),
                ordinal,
            )
        }
        _ => Err(mismatch()),
    }
}

fn pack_term(
    pack: &PackView<'_>,
    id: PackId,
    dataset: &RdfDataset,
) -> Result<TermId, EmbeddingError> {
    dataset
        .term_id_by_value(&pack.dict().term_value(id.get()))
        .ok_or(EmbeddingError::MissingReference(
            "reconstructed source term",
        ))
}

fn pack_quad(
    pack: &PackView<'_>,
    quad: QuadIds<PackId>,
    dataset: &RdfDataset,
) -> Result<QuadIds<TermId>, EmbeddingError> {
    Ok(QuadIds {
        s: pack_term(pack, quad.s, dataset)?,
        p: pack_term(pack, quad.p, dataset)?,
        o: pack_term(pack, quad.o, dataset)?,
        g: quad.g.map(|id| pack_term(pack, id, dataset)).transpose()?,
    })
}

fn term_target_id(
    id: TermId,
    dataset: &RdfDataset,
    labels: &BTreeMap<TermId, Box<str>>,
    dataset_target: TargetId,
) -> Result<TargetId, EmbeddingError> {
    term_value_target_id(&dataset.term_value(id), dataset, labels, dataset_target)
}

fn graph_target_id(
    graph: Option<TermId>,
    dataset: &RdfDataset,
    labels: &BTreeMap<TermId, Box<str>>,
    dataset_target: TargetId,
) -> Result<TargetId, EmbeddingError> {
    let graph_name = graph
        .map(|id| term_target_id(id, dataset, labels, dataset_target))
        .transpose()?;
    Ok(RdfGraphTarget {
        dataset_id: dataset_target,
        graph_name,
    }
    .into_target(false)?
    .id)
}

fn statement_target_id(
    quad: QuadIds<TermId>,
    dataset: &RdfDataset,
    labels: &BTreeMap<TermId, Box<str>>,
    dataset_target: TargetId,
) -> Result<TargetId, EmbeddingError> {
    Ok(RdfStatementTarget {
        graph: graph_target_id(quad.g, dataset, labels, dataset_target)?,
        subject: term_target_id(quad.s, dataset, labels, dataset_target)?,
        predicate: term_target_id(quad.p, dataset, labels, dataset_target)?,
        object: term_target_id(quad.o, dataset, labels, dataset_target)?,
    }
    .into_target(false, None)?
    .id)
}

fn reifier_target_id(
    reifier: TermId,
    triple: TermId,
    graph: Option<TermId>,
    dataset: &RdfDataset,
    labels: &BTreeMap<TermId, Box<str>>,
    dataset_target: TargetId,
) -> Result<TargetId, EmbeddingError> {
    let TermRef::Triple { s, p, o } = dataset.resolve(triple) else {
        return Err(EmbeddingError::Malformed(
            "source reifier does not reference a triple term",
        ));
    };
    Ok(RdfReifierTarget {
        graph: graph_target_id(graph, dataset, labels, dataset_target)?,
        statement: statement_target_id(
            QuadIds { s, p, o, g: graph },
            dataset,
            labels,
            dataset_target,
        )?,
        reifier: term_target_id(reifier, dataset, labels, dataset_target)?,
    }
    .into_target(false, None)?
    .id)
}

fn annotation_target_id(
    quad: QuadIds<TermId>,
    dataset: &RdfDataset,
    labels: &BTreeMap<TermId, Box<str>>,
    dataset_target: TargetId,
    expected: TargetId,
    ordinal: u64,
) -> Result<TargetId, EmbeddingError> {
    // Bindings are semantic values in the same actual source dataset. Their
    // iteration order supplies no source ordinal: the captured annotation row
    // already selected the original resource, predicate, value and graph.
    let mut bindings = dataset
        .reifier_quads_of(quad.s)
        .filter(|binding| binding.g == quad.g)
        .peekable();
    if bindings.peek().is_none() {
        return Err(EmbeddingError::MissingReference(
            "annotation reifier binding",
        ));
    }
    let graph = graph_target_id(quad.g, dataset, labels, dataset_target)?;
    let predicate = term_target_id(quad.p, dataset, labels, dataset_target)?;
    let object = term_target_id(quad.o, dataset, labels, dataset_target)?;
    for binding in bindings {
        let target = RdfAnnotationTarget {
            graph,
            reifier: reifier_target_id(
                binding.s,
                binding.o,
                binding.g,
                dataset,
                labels,
                dataset_target,
            )?,
            predicate,
            object,
        }
        .into_target(false, None)?;
        if target.id == expected {
            return Ok(expected);
        }
    }
    Err(EmbeddingError::OrdinalMismatch {
        target_kind: TargetKind::RdfAnnotation.code(),
        ordinal,
    })
}

/// Independently certify one external pack and derive its exact binding.
pub fn bind_external_pack(
    scope: ExternalScope,
    bytes: &[u8],
    contract: &ExternalBindingContract,
    control: &mut purrdf_core::CanonicalizationControl<'_>,
) -> Result<ExternalBinding, EmbeddingError> {
    let proof = verify_pack_bytes(bytes, control).map_err(EmbeddingError::external_pack)?;
    ExternalBinding::from_verified_pack(scope, proof, contract)
}

/// Verify an external pack attachment's exact bytes and independent RDF identity.
pub fn verify_external_pack(
    binding: ExternalBindingView<'_>,
    bytes: &[u8],
    control: &mut purrdf_core::CanonicalizationControl<'_>,
) -> Result<RdfcDigest, EmbeddingError> {
    verify_external_artifact(binding, bytes)?;
    let expected = binding
        .certified_rdf_digest()
        .ok_or(EmbeddingError::Missing("external certified RDF digest"))?;
    let proof = verify_pack_bytes(bytes, control).map_err(EmbeddingError::external_pack)?;
    let certified = proof.rdf_digest();
    DigestKind::CertifiedRdf.verify(&expected, certified.as_bytes())?;
    Ok(RdfcDigest::from_raw(*certified.as_bytes()))
}
