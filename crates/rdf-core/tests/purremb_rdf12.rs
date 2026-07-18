// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RDF 1.2 graph, reifier, annotation, directional-literal, and triple-term targets.

use purrdf_core::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, BlankScope, CanonicalMetadataInput,
    CertifiedPurrpckSource, ContentDigest, DimensionalityPolicy, DistanceMetric, EmbeddingBuilder,
    EmbeddingFamilyContract, EmbeddingTarget, EmbeddingView, MatrixInput, MatrixRow,
    PrefixPostprocessing, ProjectionSpec, RdfAnnotationTarget, RdfDatasetBuilder, RdfGraphTarget,
    RdfLiteral, RdfReifierTarget, RdfStatementTarget, RdfTermTarget, RdfTextDirection,
    RelationKind, SourceVerificationMode, StageImplementation, TargetRelation, TargetSet,
    VectorDtype, try_canonicalize, verify_embedding, verify_embedding_source,
};

fn artifact(name: &str) -> ArtifactIdentity {
    ArtifactIdentity::new(
        format!("https://example.org/rdf12/{name}"),
        "application/octet-stream",
        ContentDigest::of(name.as_bytes()),
        None,
        ArtifactIdentityKind::Single,
    )
    .expect("artifact")
}

fn stage(name: &str) -> AppliedStage {
    AppliedStage::Applied(
        StageImplementation::new(
            format!("https://example.org/rdf12/{name}"),
            ContentDigest::of(name.as_bytes()),
            "application/cbor",
            vec![0xa1, 1],
        )
        .expect("stage"),
    )
}

fn contract() -> EmbeddingFamilyContract {
    EmbeddingFamilyContract {
        model: artifact("model"),
        engine: artifact("engine"),
        tokenizer: artifact("tokenizer"),
        execution: stage("execution"),
        subject_projection: stage("rdf12-projection"),
        preprocessing: AppliedStage::NotApplied,
        chunking: AppliedStage::NotApplied,
        pooling: stage("pooling"),
        normalization: AppliedStage::NotApplied,
        truncation: AppliedStage::NotApplied,
        dtype: VectorDtype::F32,
        metric: DistanceMetric::Cosine,
        dimensionality: DimensionalityPolicy::fixed(2, PrefixPostprocessing::None)
            .expect("dimension"),
        extensions: Vec::new(),
    }
}

struct RdfFixture {
    source: CertifiedPurrpckSource,
    source_bytes: Vec<u8>,
    targets: Vec<EmbeddingTarget>,
    relations: Vec<TargetRelation>,
    matrix_targets: Vec<EmbeddingTarget>,
    family_contract: EmbeddingFamilyContract,
}

fn rdf_fixture() -> RdfFixture {
    let mut dataset = RdfDatasetBuilder::new();
    let subject = dataset.intern_blank("source-label", BlankScope::DEFAULT);
    let predicate = dataset.intern_iri("https://example.org/rdf12/predicate");
    let graph_name = dataset.intern_iri("https://example.org/rdf12/graph");
    let reifier_term = dataset.intern_iri("https://example.org/rdf12/reifier");
    let annotation_predicate = dataset.intern_iri("https://example.org/rdf12/confidence");
    let literal = dataset.intern_literal(RdfLiteral {
        lexical_form: "bonjour".into(),
        datatype: None,
        language: Some("fr".into()),
        direction: Some(RdfTextDirection::Ltr),
    });
    let annotation_object = dataset.intern_literal(RdfLiteral {
        lexical_form: "0.9".into(),
        datatype: Some("http://www.w3.org/2001/XMLSchema#decimal".into()),
        language: None,
        direction: None,
    });
    let inner_triple = dataset.intern_triple(subject, predicate, literal);
    let outer_triple = dataset.intern_triple(inner_triple, predicate, annotation_object);
    dataset.push_quad(subject, predicate, literal, None);
    dataset.push_quad(subject, predicate, literal, Some(graph_name));
    dataset.push_quad(subject, predicate, outer_triple, Some(graph_name));
    dataset.push_reifier_in_graph(reifier_term, inner_triple, Some(graph_name));
    dataset.push_annotation_in_graph(
        reifier_term,
        annotation_predicate,
        annotation_object,
        Some(graph_name),
    );
    let dataset = dataset.freeze().expect("valid RDF 1.2 dataset");
    let canonical = try_canonicalize(&dataset).expect("canonical labels");
    let (source, source_bytes) =
        CertifiedPurrpckSource::from_dataset(&dataset).expect("source pack");
    let dataset_target = source.dataset_target(true).expect("dataset target");

    let subject_target = RdfTermTarget::Blank {
        dataset_id: dataset_target.id,
        canonical_label: canonical
            .labels
            .get(&subject)
            .expect("canonical blank label")
            .to_string(),
    }
    .into_target(true, None)
    .expect("blank target");
    let predicate_target = RdfTermTarget::Iri("https://example.org/rdf12/predicate".into())
        .into_target(true, None)
        .expect("predicate target");
    let graph_name_target = RdfTermTarget::Iri("https://example.org/rdf12/graph".into())
        .into_target(true, None)
        .expect("graph-name target");
    let reifier_term_target = RdfTermTarget::Iri("https://example.org/rdf12/reifier".into())
        .into_target(true, None)
        .expect("reifier term target");
    let annotation_predicate_target =
        RdfTermTarget::Iri("https://example.org/rdf12/confidence".into())
            .into_target(true, None)
            .expect("annotation predicate target");
    let literal_target = RdfTermTarget::Literal {
        lexical: "bonjour".into(),
        datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".into(),
        language: Some("fr".into()),
        direction: Some(RdfTextDirection::Ltr),
    }
    .into_target(true, None)
    .expect("directional literal target");
    let annotation_object_target = RdfTermTarget::Literal {
        lexical: "0.9".into(),
        datatype: "http://www.w3.org/2001/XMLSchema#decimal".into(),
        language: None,
        direction: None,
    }
    .into_target(true, None)
    .expect("annotation object target");
    let inner_target = RdfTermTarget::Triple {
        subject: subject_target.id,
        predicate: predicate_target.id,
        object: literal_target.id,
    }
    .into_target(true, None)
    .expect("inner triple target");
    let outer_target = RdfTermTarget::Triple {
        subject: inner_target.id,
        predicate: predicate_target.id,
        object: annotation_object_target.id,
    }
    .into_target(true, None)
    .expect("recursive triple target");

    let default_graph = RdfGraphTarget {
        dataset_id: dataset_target.id,
        graph_name: None,
    }
    .into_target(true)
    .expect("default graph target");
    let named_graph = RdfGraphTarget {
        dataset_id: dataset_target.id,
        graph_name: Some(graph_name_target.id),
    }
    .into_target(true)
    .expect("named graph target");
    let default_statement = RdfStatementTarget {
        graph: default_graph.id,
        subject: subject_target.id,
        predicate: predicate_target.id,
        object: literal_target.id,
    }
    .into_target(true, None)
    .expect("default statement");
    let named_statement = RdfStatementTarget {
        graph: named_graph.id,
        subject: subject_target.id,
        predicate: predicate_target.id,
        object: literal_target.id,
    }
    .into_target(true, None)
    .expect("named statement");
    let recursive_statement = RdfStatementTarget {
        graph: named_graph.id,
        subject: subject_target.id,
        predicate: predicate_target.id,
        object: outer_target.id,
    }
    .into_target(true, None)
    .expect("recursive statement");
    let reifier = RdfReifierTarget {
        graph: named_graph.id,
        statement: named_statement.id,
        reifier: reifier_term_target.id,
    }
    .into_target(true, None)
    .expect("reifier target");
    let annotation = RdfAnnotationTarget {
        graph: named_graph.id,
        reifier: reifier.id,
        predicate: annotation_predicate_target.id,
        object: annotation_object_target.id,
    }
    .into_target(true, None)
    .expect("annotation target");

    let relations = vec![
        TargetRelation::builtin(
            dataset_target.id,
            RelationKind::DatasetGraph,
            default_graph.id,
        ),
        TargetRelation::builtin(
            dataset_target.id,
            RelationKind::DatasetGraph,
            named_graph.id,
        ),
        TargetRelation::builtin(
            named_graph.id,
            RelationKind::GraphName,
            graph_name_target.id,
        ),
        TargetRelation::builtin(
            default_graph.id,
            RelationKind::GraphStatement,
            default_statement.id,
        ),
        TargetRelation::builtin(
            named_graph.id,
            RelationKind::GraphStatement,
            named_statement.id,
        ),
        TargetRelation::builtin(
            named_graph.id,
            RelationKind::GraphStatement,
            recursive_statement.id,
        ),
        TargetRelation::builtin(
            default_statement.id,
            RelationKind::StatementSubject,
            subject_target.id,
        ),
        TargetRelation::builtin(
            default_statement.id,
            RelationKind::StatementPredicate,
            predicate_target.id,
        ),
        TargetRelation::builtin(
            default_statement.id,
            RelationKind::StatementObject,
            literal_target.id,
        ),
        TargetRelation::builtin(
            named_statement.id,
            RelationKind::StatementSubject,
            subject_target.id,
        ),
        TargetRelation::builtin(
            named_statement.id,
            RelationKind::StatementPredicate,
            predicate_target.id,
        ),
        TargetRelation::builtin(
            named_statement.id,
            RelationKind::StatementObject,
            literal_target.id,
        ),
        TargetRelation::builtin(
            recursive_statement.id,
            RelationKind::StatementSubject,
            subject_target.id,
        ),
        TargetRelation::builtin(
            recursive_statement.id,
            RelationKind::StatementPredicate,
            predicate_target.id,
        ),
        TargetRelation::builtin(
            recursive_statement.id,
            RelationKind::StatementObject,
            outer_target.id,
        ),
        TargetRelation::builtin(
            named_statement.id,
            RelationKind::StatementReifier,
            reifier.id,
        ),
        TargetRelation::builtin(
            reifier.id,
            RelationKind::ReifierTerm,
            reifier_term_target.id,
        ),
        TargetRelation::builtin(reifier.id, RelationKind::ReifierAnnotation, annotation.id),
        TargetRelation::builtin(
            annotation.id,
            RelationKind::AnnotationPredicate,
            annotation_predicate_target.id,
        ),
        TargetRelation::builtin(
            annotation.id,
            RelationKind::AnnotationObject,
            annotation_object_target.id,
        ),
        TargetRelation::builtin(
            inner_target.id,
            RelationKind::TripleTermSubject,
            subject_target.id,
        ),
        TargetRelation::builtin(
            inner_target.id,
            RelationKind::TripleTermPredicate,
            predicate_target.id,
        ),
        TargetRelation::builtin(
            inner_target.id,
            RelationKind::TripleTermObject,
            literal_target.id,
        ),
        TargetRelation::builtin(
            outer_target.id,
            RelationKind::TripleTermSubject,
            inner_target.id,
        ),
        TargetRelation::builtin(
            outer_target.id,
            RelationKind::TripleTermPredicate,
            predicate_target.id,
        ),
        TargetRelation::builtin(
            outer_target.id,
            RelationKind::TripleTermObject,
            annotation_object_target.id,
        ),
    ];
    let targets = vec![
        dataset_target,
        subject_target,
        predicate_target,
        graph_name_target,
        reifier_term_target,
        annotation_predicate_target,
        literal_target,
        annotation_object_target,
        inner_target,
        outer_target.clone(),
        default_graph,
        named_graph,
        default_statement,
        named_statement,
        recursive_statement,
        reifier.clone(),
        annotation.clone(),
    ];
    RdfFixture {
        source,
        source_bytes,
        targets,
        relations,
        matrix_targets: vec![outer_target, reifier, annotation],
        family_contract: contract(),
    }
}

fn build(
    fixture: &RdfFixture,
    relations: Vec<TargetRelation>,
) -> Result<Vec<u8>, purrdf_core::EmbeddingError> {
    let family = fixture.family_contract.derive()?;
    let set = TargetSet::new(
        fixture
            .matrix_targets
            .iter()
            .map(|target| target.id)
            .collect(),
    )?;
    let metadata = CanonicalMetadataInput {
        source: fixture.source,
        family_contracts: vec![fixture.family_contract.clone()],
        targets: fixture.targets.clone(),
        target_sets: vec![set.clone()],
        relations,
        token_spans: Vec::new(),
        external_bindings: Vec::new(),
        indexes: Vec::new(),
        extensions: Vec::new(),
    };
    let rows = set
        .targets
        .iter()
        .enumerate()
        .map(|(index, target)| MatrixRow::new(*target, vec![index as f32 + 1.0, 1.0]))
        .collect();
    let matrix = MatrixInput {
        family_id: family.id,
        target_set_id: set.id,
        stored_dimension: 2,
        rows,
        projections: vec![ProjectionSpec::derive(
            family.id,
            2,
            PrefixPostprocessing::None,
        )],
    };
    let mut builder = EmbeddingBuilder::from_typed_metadata(metadata);
    builder.add_f32_matrix(matrix);
    Ok(builder.build()?.bytes)
}

#[test]
fn rdf12_composites_round_trip_through_one_identity_model() {
    let fixture = rdf_fixture();
    let bytes = build(&fixture, fixture.relations.clone()).expect("RDF 1.2 PURREMB");
    let mut view = EmbeddingView::from_bytes(&bytes).expect("structural view");
    verify_embedding(&mut view).expect("full RDF target verification");
    verify_embedding_source(
        &view,
        &fixture.source_bytes,
        SourceVerificationMode::Certified,
    )
    .expect("certified RDF source");
    assert_eq!(view.target_count(), fixture.targets.len());
    assert_eq!(view.relation_count(), fixture.relations.len());
    assert_eq!(
        view.targets()
            .filter(|target| target.kind().expect("kind") == purrdf_core::TargetKind::RdfGraph)
            .count(),
        2
    );
}

#[test]
fn missing_composite_relation_is_rejected_before_writing() {
    let fixture = rdf_fixture();
    let mut incomplete = fixture.relations.clone();
    incomplete.pop();
    assert!(build(&fixture, incomplete).is_err());
}
