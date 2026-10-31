// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Cryptographic, numerical, source-pack, and external-binding verification.

use purrdf_core::embedding::{
    DigestKind, EmbeddingError, EmbeddingView, ExternalBinding, ExternalBindingContract,
    ExternalBindingView, ExternalScope, RdfAnnotationTarget, RdfGraphTarget, RdfReifierTarget,
    RdfStatementTarget, RdfcDigest, TargetId, TargetKind, TargetView, term_value_target_id,
    verify_embedding_dataset_target, verify_external_artifact,
};
use purrdf_core::{
    ContentDigest, DatasetView, PackId, PackView, QuadIds, RdfDataset, TermId, TermRef,
    dataset_from_view, try_canonicalize, verify_pack_bytes,
};
use std::collections::BTreeMap;
/// Evidence returned after complete exact-source, RDF and ordinal verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceVerificationReport {
    exact_digest: ContentDigest,
    certified_rdf_digest: RdfcDigest,
    ordinals_checked: u64,
}
impl SourceVerificationReport {
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
/// Verify the complete pack attachment, including independent RDF identity and all source-local ordinals.
pub fn verify_embedding_source(
    embedding: &EmbeddingView<'_>,
    source_bytes: &[u8],
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
    DigestKind::SourceExact.verify(
        source.source_exact_digest().as_bytes(),
        exact_digest.as_bytes(),
    )?;

    let pack = PackView::from_bytes(source_bytes).map_err(EmbeddingError::source_pack)?;

    let proof = verify_pack_bytes(source_bytes).map_err(EmbeddingError::source_pack)?;
    let certified = proof.rdf_digest();
    DigestKind::CertifiedRdf.verify(&source.certified_rdf_digest(), certified.as_bytes())?;
    verify_embedding_dataset_target(embedding)?;
    let ordinals_checked = verify_source_ordinals(embedding, &pack)?;
    Ok(SourceVerificationReport {
        exact_digest,
        certified_rdf_digest: RdfcDigest::from_raw(*certified.as_bytes()),
        ordinals_checked,
    })
}

fn verify_source_ordinals(
    embedding: &EmbeddingView<'_>,
    pack: &PackView<'_>,
) -> Result<u64, EmbeddingError> {
    if embedding
        .targets()
        .all(|target| target.source_local_ordinal().is_none())
    {
        return Ok(0);
    }
    let dataset = dataset_from_view(pack).map_err(EmbeddingError::source_pack)?;
    // The refusal REASON is carried through rather than replaced by a fixed string:
    // canonicalization now refuses for two different reasons, and a pack rejected for
    // carrying reserved vocabulary must not be reported as one that merely ran out of
    // budget — the first is an attempt to forge an identity, the second is not.
    let canonical = try_canonicalize(&dataset).map_err(EmbeddingError::source_pack)?;
    let dataset_target = embedding.source().dataset_target_id();
    let mut checked = 0u64;

    for target in embedding.targets() {
        let Some(ordinal) = target.source_local_ordinal() else {
            continue;
        };
        let expected = target_from_source_ordinal(
            target,
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
            if annotation_target_matches(pack, quad, dataset, labels, dataset_target, target.id())?
            {
                Ok(target.id())
            } else {
                Err(mismatch())
            }
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
    let guard = pack
        .resolve(triple)
        .expect("validated resident pack read is infallible");
    let TermRef::Triple { s, p, o } = guard else {
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

fn annotation_target_matches(
    pack: &PackView<'_>,
    quad: QuadIds<PackId>,
    dataset: &RdfDataset,
    labels: &BTreeMap<TermId, Box<str>>,
    dataset_target: TargetId,
    expected: TargetId,
) -> Result<bool, EmbeddingError> {
    // rdf:reifies is not functional. One annotation row can describe several
    // statement bindings; the target identity selects the binding it names.
    // The pack brackets this subject's run by binary search, without a second
    // lookup table or a scan over unrelated reifiers.
    let mut bindings = pack
        .reifier_quads_of(quad.s)
        .filter(|binding| binding.g == quad.g)
        .peekable();
    if bindings.peek().is_none() {
        return Err(EmbeddingError::MissingReference(
            "annotation reifier binding",
        ));
    }
    let graph = graph_target_id(pack, quad.g, dataset, labels, dataset_target)?;
    let predicate = term_target_id(pack, quad.p.get(), dataset, labels, dataset_target)?;
    let object = term_target_id(pack, quad.o.get(), dataset, labels, dataset_target)?;
    for binding in bindings {
        let target = RdfAnnotationTarget {
            graph,
            reifier: reifier_target_id(pack, binding, dataset, labels, dataset_target)?,
            predicate,
            object,
        }
        .into_target(false, None)?;
        if target.id == expected {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Independently certify one external pack and derive its exact binding.
pub fn bind_external_pack(
    scope: ExternalScope,
    bytes: &[u8],
    contract: &ExternalBindingContract,
) -> Result<ExternalBinding, EmbeddingError> {
    let proof = verify_pack_bytes(bytes).map_err(EmbeddingError::external_pack)?;
    ExternalBinding::from_verified_pack(scope, proof, contract)
}

/// Verify an external pack attachment's exact bytes and independent RDF identity.
pub fn verify_external_pack(
    binding: ExternalBindingView<'_>,
    bytes: &[u8],
) -> Result<RdfcDigest, EmbeddingError> {
    verify_external_artifact(binding, bytes)?;
    let expected = binding
        .certified_rdf_digest()
        .ok_or(EmbeddingError::Missing("external certified RDF digest"))?;
    let proof = verify_pack_bytes(bytes).map_err(EmbeddingError::external_pack)?;
    let certified = proof.rdf_digest();
    DigestKind::CertifiedRdf.verify(&expected, certified.as_bytes())?;
    Ok(RdfcDigest::from_raw(*certified.as_bytes()))
}
