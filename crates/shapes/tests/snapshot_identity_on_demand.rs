// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A SHACL bind hashes the data graph's content for its identity only when
//! something reads it.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral};
use purrdf_shapes::engine::{parse_shapes, project_dataset, validate_projected_dataset};

const EX: &str = "http://example.org/";

const SHAPES: &str = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
ex:Targeted a sh:NodeShape ;
    sh:target [ a sh:SPARQLTarget ; sh:select "SELECT ?this WHERE { ?this a <http://example.org/Person> }" ] ;
    sh:property [ sh:path ex:clearance ; sh:minCount 1 ] .
ex:Constrained a sh:NodeShape ; sh:targetClass ex:Person ;
    sh:sparql [ sh:select "SELECT $this WHERE { $this <http://example.org/status> \"banned\" }" ] .
"#;

fn people() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder
        .intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
        .unwrap();
    let person = builder.intern_iri(&format!("{EX}Person")).unwrap();
    let status = builder.intern_iri(&format!("{EX}status")).unwrap();
    let banned = builder
        .intern_literal(RdfLiteral::simple("banned"))
        .unwrap();
    for name in ["alice", "bob"] {
        let node = builder.intern_iri(&format!("{EX}{name}")).unwrap();
        builder.push_quad(node, rdf_type, person, None).unwrap();
        builder.push_quad(node, status, banned, None).unwrap();
    }
    builder.freeze().unwrap()
}

/// `subClassOfInShapesGraph` naming classes the data graph never mentions
/// composes the data graph with a term supplement for their identities (the
/// class-membership route). Building that composite, binding and validating
/// hash nothing.
#[test]
fn a_class_supplement_composite_hashes_nothing() {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder
        .intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
        .unwrap();
    let carol = builder.intern_iri(&format!("{EX}carol")).unwrap();
    let student = builder.intern_iri(&format!("{EX}Student")).unwrap();
    builder.push_quad(carol, rdf_type, student, None).unwrap();
    let data = project_dataset(&builder.freeze().unwrap()).unwrap();
    let unset = "snapshot_identity: OnceLock(<uninit>)";
    let mut shapes = parse_shapes(
        r"@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex: <http://example.org/> .
ex:Student rdfs:subClassOf ex:Scholar .
ex:ScholarShape a sh:NodeShape ; sh:targetClass ex:Scholar ;
    sh:property [ sh:path ex:clearance ; sh:minCount 1 ] .
",
        None,
    )
    .unwrap();
    shapes.set_validation_options(
        purrdf_shapes::engine::ValidationOptions::default().with_subclass_of_in_shapes_graph(true),
    );
    let report = validate_projected_dataset(Arc::clone(&data), &shapes).unwrap();
    // carol is a Scholar only through the shapes graph, and she has no clearance.
    assert_eq!(report.results.len(), 1);
    assert!(format!("{data:?}").contains(unset));
}

/// No cross-run cache and no receipt: a bind with SPARQL targets and SPARQL
/// constraints over many focus nodes never hashes the data graph.
#[test]
fn a_bind_with_sparql_targets_and_constraints_hashes_nothing() {
    let data = project_dataset(&people()).unwrap();
    // The identity is hashed once per dataset behind a once-lock, so an unset lock
    // is exactly zero hashes.
    let unset = "snapshot_identity: OnceLock(<uninit>)";
    assert!(
        format!("{data:?}").contains(unset),
        "a fresh projection is unsealed"
    );
    let shapes = parse_shapes(SHAPES, None).unwrap();
    let report = validate_projected_dataset(Arc::clone(&data), &shapes).unwrap();
    // The target and constraint queries really ran: both people violate both shapes.
    assert_eq!(report.results.len(), 4);
    assert!(format!("{data:?}").contains(unset));
}
