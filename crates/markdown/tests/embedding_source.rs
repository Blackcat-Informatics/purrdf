// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A projected citation with several anchors remains a certified embedding source.

use purrdf_core::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, CanonicalMetadataInput,
    CertifiedPurrpckSource, ContentDigest, DimensionalityPolicy, DistanceMetric, EmbeddingBuilder,
    EmbeddingFamilyContract, EmbeddingView, MatrixInput, MatrixRow, PackView, PrefixPostprocessing,
    ProjectionSpec, RdfTermTarget, SourceVerificationMode, StageImplementation, TargetSet,
    TermValue, VectorDtype, verify_embedding, verify_embedding_source,
};
use purrdf_markdown::{Profile, SourceDocument, Vocabulary, slice_markdown};
use purrdf_rdf::parse_dataset;

const DOCUMENT: &str = "# Field notes\n\n1. Two observations.\n\n## Concordance\n\n\
    | Verses | Canon source | Anchors |\n| --- | --- | --- |\n\
    | 1 | `observations.ttl` | `first` `second` |\n";

fn artifact(name: &str) -> ArtifactIdentity {
    ArtifactIdentity::new(
        format!("https://example.org/embedding/{name}"),
        "application/octet-stream",
        ContentDigest::of(name.as_bytes()),
        None,
        ArtifactIdentityKind::Single,
    )
    .expect("artifact identity")
}

fn stage(name: &str) -> AppliedStage {
    AppliedStage::Applied(
        StageImplementation::new(
            format!("https://example.org/embedding/{name}"),
            ContentDigest::of(name.as_bytes()),
            "application/octet-stream",
            vec![1],
        )
        .expect("stage implementation"),
    )
}

fn contract() -> EmbeddingFamilyContract {
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
        dtype: VectorDtype::F32,
        metric: DistanceMetric::Cosine,
        dimensionality: DimensionalityPolicy::fixed(1, PrefixPostprocessing::None)
            .expect("fixed dimensionality"),
        extensions: Vec::new(),
    }
}

#[test]
fn a_two_anchor_citation_certifies_with_a_source_ordinal_embedding_target() {
    let profile = Profile::new(
        "embedding-source-md-v1",
        1,
        Vocabulary::under("https://example.org/markdown/").expect("vocabulary"),
    );
    let claims = slice_markdown(
        &SourceDocument {
            id: "https://example.org/doc/field-notes",
            bytes: DOCUMENT.as_bytes(),
        },
        &profile,
    )
    .expect("Markdown projection");
    let turtle: String = claims.iter().map(|claim| claim.turtle.as_str()).collect();
    let dataset = parse_dataset(turtle.as_bytes(), "text/turtle", None).expect("projected RDF 1.2");
    let bindings: Vec<_> = dataset.reifiers_with_graph().collect();
    assert_eq!(bindings.len(), 2, "the row must lift both anchors");
    assert_eq!(bindings[0].0, bindings[1].0, "one citation reifier");
    assert_eq!(bindings[0].2, bindings[1].2, "bindings share one graph");
    assert_ne!(bindings[0].1, bindings[1].1, "distinct reified triples");
    let TermValue::Iri(citation) = dataset.term_value(bindings[0].0) else {
        panic!("the projected citation must have an IRI");
    };

    let (source, source_bytes) =
        CertifiedPurrpckSource::from_dataset(&dataset).expect("certified source pack");
    let dataset_target = source.dataset_target(true).expect("dataset target");
    let pack = PackView::from_bytes(&source_bytes).expect("source pack view");
    let ordinal = pack
        .dict()
        .id_by_value(&TermValue::Iri(citation.clone()))
        .expect("citation source ordinal");
    let target = RdfTermTarget::Iri(citation)
        .into_target(true, Some(ordinal))
        .expect("citation target with its source ordinal");
    let set = TargetSet::new(vec![target.id]).expect("target set");
    let contract = contract();
    let family = contract.derive().expect("embedding family");
    let matrix = MatrixInput {
        family_id: family.id,
        target_set_id: set.id,
        stored_dimension: 1,
        rows: vec![MatrixRow::new(target.id, vec![1.0])],
        projections: vec![ProjectionSpec::derive(
            family.id,
            1,
            PrefixPostprocessing::None,
        )],
    };
    let mut builder = EmbeddingBuilder::from_typed_metadata(CanonicalMetadataInput {
        source,
        family_contracts: vec![contract],
        targets: vec![dataset_target, target],
        target_sets: vec![set],
        relations: Vec::new(),
        token_spans: Vec::new(),
        external_bindings: Vec::new(),
        indexes: Vec::new(),
        extensions: Vec::new(),
    });
    builder.add_f32_matrix(matrix);
    let artifact = builder.build().expect("embedding artifact");
    let mut embedding = EmbeddingView::from_bytes(&artifact.bytes).expect("embedding view");
    verify_embedding(&mut embedding).expect("embedding integrity");
    let report =
        verify_embedding_source(&embedding, &source_bytes, SourceVerificationMode::Certified)
            .expect("a multi-anchor citation is a valid certified RDF 1.2 source");
    assert_eq!(report.mode(), SourceVerificationMode::Certified);
    assert!(report.certified_rdf_digest().is_some());
    assert_eq!(
        report.ordinals_checked(),
        1,
        "the citation ordinal was bound"
    );
}
