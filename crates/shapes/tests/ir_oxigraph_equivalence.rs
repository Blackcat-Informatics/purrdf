// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Differential equivalence across the two native engine entry-paths (C4).
//!
//! Historically this proved the IR-native SHACL backend agreed with an oxigraph
//! `Store` oracle. The engine is now IR-native end-to-end (no oxigraph backend), so
//! the differential is reframed onto the two remaining INDEPENDENT entry-paths:
//!
//! - the text path — [`validate_graphs`], which natively parses N-Triples/Turtle to
//!   a frozen dataset then validates; and
//! - the dataset path — [`validate_dataset_graphs`], which validates a pre-frozen
//!   [`RdfDataset`] (the GTS-bundle path).
//!
//! For each `(shapes_ttl, data)` case both paths must produce byte-identical
//! reports (same `conforms` flag, same deterministically-sorted results compared
//! via the canonical `to_ntriples()` serialization). This keeps the cases as a
//! native conformance safety net without a second (oxigraph) engine.

use std::sync::Arc;

use ::purrdf::RdfDataset;
use ::purrdf::parse_dataset;
use purrdf_shapes::engine::{validate_dataset_graphs, validate_graphs};

const PREFIXES: &str = r"
    @prefix sh:   <http://www.w3.org/ns/shacl#> .
    @prefix ex:   <http://example.org/ns#> .
    @prefix rdf:  <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .
";

/// Parse a Turtle data document (so RDF 1.2 `<<( … )>>` reifier syntax can be
/// expressed) into a frozen dataset via the native codec.
fn load_data_turtle(ttl: &str) -> Arc<RdfDataset> {
    parse_dataset(ttl.as_bytes(), "text/turtle", None).expect("valid Turtle")
}

/// Run BOTH native entry-paths on the same data (N-Triples) and shapes, asserting
/// the reports are byte-identical.
fn assert_backends_agree(label: &str, shapes_ttl: &str, data_nt: &str) {
    let dataset = if data_nt.is_empty() {
        ::purrdf::RdfDatasetBuilder::new()
            .freeze()
            .expect("empty dataset")
    } else {
        parse_dataset(data_nt.as_bytes(), "application/n-triples", None).expect("valid N-Triples")
    };

    // Text path.
    let text = validate_graphs(data_nt, shapes_ttl)
        .unwrap_or_else(|e| panic!("[{label}] validate_graphs: {e}"));
    // Dataset path.
    let dataset_report = validate_dataset_graphs(dataset.as_ref(), shapes_ttl)
        .unwrap_or_else(|e| panic!("[{label}] validate_dataset_graphs: {e}"));

    assert_reports_agree(label, &text, &dataset_report);
}

/// As [`assert_backends_agree`], but over an already-frozen dataset (lets a case use
/// Turtle for RDF 1.2 constructs — quoted-triple reifiers — that N-Triples cannot
/// express). The text path cannot carry a reifier, so the differential here is the
/// dataset path against an owned-quad round-trip re-freeze of the same dataset
/// (which preserves reifiers/annotations): both must produce byte-identical reports.
fn assert_backends_agree_store(label: &str, shapes_ttl: &str, dataset: &Arc<RdfDataset>) {
    use ::purrdf::RdfDatasetBuilder;

    let mut builder = RdfDatasetBuilder::new();
    for quad in dataset.owned_quads() {
        builder.push_owned_quad(&quad);
    }
    for reifier in dataset.owned_reifiers() {
        builder.push_owned_reifier(&reifier);
    }
    for annotation in dataset.owned_annotations() {
        builder.push_owned_annotation(&annotation);
    }
    let round_trip = builder.freeze().expect("dataset re-freezes");

    let direct = validate_dataset_graphs(dataset.as_ref(), shapes_ttl)
        .unwrap_or_else(|e| panic!("[{label}] validate_dataset_graphs (direct): {e}"));
    let re_frozen = validate_dataset_graphs(round_trip.as_ref(), shapes_ttl)
        .unwrap_or_else(|e| panic!("[{label}] validate_dataset_graphs (round-trip): {e}"));

    assert_reports_agree(label, &direct, &re_frozen);
}

fn assert_reports_agree(
    label: &str,
    a: &purrdf_shapes::report::ValidationReport,
    b: &purrdf_shapes::report::ValidationReport,
) {
    assert_eq!(
        a.conforms, b.conforms,
        "[{label}] conforms flag must match (text={}, dataset={})",
        a.conforms, b.conforms
    );
    assert_eq!(
        a.results.len(),
        b.results.len(),
        "[{label}] result count must match (text={}, dataset={})",
        a.results.len(),
        b.results.len()
    );
    assert_eq!(
        a.to_ntriples(),
        b.to_ntriples(),
        "[{label}] the two native entry-paths must produce byte-identical reports"
    );
}

#[test]
fn empty_data_and_shapes_agree() {
    assert_backends_agree("empty", "", "");
}

#[test]
fn target_class_min_count_violation_agrees() {
    let shapes = format!(
        r"{PREFIXES}
        ex:PersonShape a sh:NodeShape ;
            sh:targetClass ex:Person ;
            sh:property [ sh:path ex:name ; sh:minCount 1 ] .
        "
    );
    let data = "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n";
    assert_backends_agree("targetClass-minCount-violate", &shapes, data);
}

#[test]
fn target_class_min_count_conforming_agrees() {
    let shapes = format!(
        r"{PREFIXES}
        ex:PersonShape a sh:NodeShape ;
            sh:targetClass ex:Person ;
            sh:property [ sh:path ex:name ; sh:minCount 1 ] .
        "
    );
    let data = concat!(
        "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#alice> <http://example.org/ns#name> \"Alice\" .\n",
    );
    assert_backends_agree("targetClass-minCount-conform", &shapes, data);
}

#[test]
fn subclass_closure_agrees() {
    // sh:targetClass honors asserted rdfs:subClassOf (SHACL §4.2.5).
    let shapes = format!(
        r"{PREFIXES}
        ex:PersonShape a sh:NodeShape ;
            sh:targetClass ex:Person ;
            sh:property [ sh:path ex:name ; sh:minCount 1 ] .
        "
    );
    let data = concat!(
        "<http://example.org/ns#bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Employee> .\n",
        "<http://example.org/ns#Employee> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/ns#Manager> .\n",
        "<http://example.org/ns#Manager> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/ns#Person> .\n",
    );
    assert_backends_agree("subclass-closure", &shapes, data);
}

#[test]
fn sh_class_target_objects_of_agrees() {
    // sh:class constraint on path values + targetObjectsOf target.
    let shapes = format!(
        r"{PREFIXES}
        ex:KnowsShape a sh:NodeShape ;
            sh:targetSubjectsOf ex:knows ;
            sh:property [ sh:path ex:knows ; sh:class ex:Person ] .
        "
    );
    let data = concat!(
        "<http://example.org/ns#alice> <http://example.org/ns#knows> <http://example.org/ns#bob> .\n",
        "<http://example.org/ns#alice> <http://example.org/ns#knows> <http://example.org/ns#carol> .\n",
        "<http://example.org/ns#bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        // carol is NOT a Person → sh:class violation
    );
    assert_backends_agree("sh-class-targetSubjectsOf", &shapes, data);
}

#[test]
fn target_subjects_objects_of_agrees() {
    let shapes = format!(
        r"{PREFIXES}
        ex:KnowerShape a sh:NodeShape ;
            sh:targetSubjectsOf ex:knows ;
            sh:property [ sh:path ex:label ; sh:minCount 1 ] .
        ex:KnownShape a sh:NodeShape ;
            sh:targetObjectsOf ex:knows ;
            sh:property [ sh:path ex:label ; sh:minCount 1 ] .
        "
    );
    let data = "<http://example.org/ns#alice> <http://example.org/ns#knows> <http://example.org/ns#bob> .\n";
    assert_backends_agree("targetSubjectsOf+ObjectsOf", &shapes, data);
}

#[test]
fn datatype_and_pattern_constraints_agree() {
    let shapes = format!(
        r#"{PREFIXES}
        ex:ThingShape a sh:NodeShape ;
            sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] ;
            sh:property [ sh:path ex:code ; sh:pattern "^[A-Z]{{3}}$" ] .
        "#
    );
    let data = concat!(
        "<http://example.org/ns#t1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Thing> .\n",
        // age has the WRONG datatype (string, not integer) → datatype violation
        "<http://example.org/ns#t1> <http://example.org/ns#age> \"forty\" .\n",
        // code does not match the pattern → pattern violation
        "<http://example.org/ns#t1> <http://example.org/ns#code> \"ab\" .\n",
        "<http://example.org/ns#t2> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Thing> .\n",
        "<http://example.org/ns#t2> <http://example.org/ns#age> \"42\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
        "<http://example.org/ns#t2> <http://example.org/ns#code> \"ABC\" .\n",
    );
    assert_backends_agree("datatype+pattern", &shapes, data);
}

#[test]
fn cardinality_min_max_count_agrees() {
    let shapes = format!(
        r"{PREFIXES}
        ex:CardShape a sh:NodeShape ;
            sh:targetClass ex:Card ;
            sh:property [ sh:path ex:tag ; sh:minCount 2 ; sh:maxCount 3 ] .
        "
    );
    let data = concat!(
        // c1 has 1 tag → minCount violation
        "<http://example.org/ns#c1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Card> .\n",
        "<http://example.org/ns#c1> <http://example.org/ns#tag> \"a\" .\n",
        // c2 has 4 tags → maxCount violation
        "<http://example.org/ns#c2> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Card> .\n",
        "<http://example.org/ns#c2> <http://example.org/ns#tag> \"a\" .\n",
        "<http://example.org/ns#c2> <http://example.org/ns#tag> \"b\" .\n",
        "<http://example.org/ns#c2> <http://example.org/ns#tag> \"c\" .\n",
        "<http://example.org/ns#c2> <http://example.org/ns#tag> \"d\" .\n",
    );
    assert_backends_agree("cardinality", &shapes, data);
}

#[test]
fn node_shape_recursion_agrees() {
    // sh:node references another node shape (recursion through the generic engine).
    let shapes = format!(
        r"{PREFIXES}
        ex:AddressShape a sh:NodeShape ;
            sh:property [ sh:path ex:city ; sh:minCount 1 ] .
        ex:PersonShape a sh:NodeShape ;
            sh:targetClass ex:Person ;
            sh:property [ sh:path ex:address ; sh:node ex:AddressShape ] .
        "
    );
    let data = concat!(
        "<http://example.org/ns#p1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        // p1's address has no city → nested node-shape violation
        "<http://example.org/ns#p1> <http://example.org/ns#address> <http://example.org/ns#addr1> .\n",
        "<http://example.org/ns#p2> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#p2> <http://example.org/ns#address> <http://example.org/ns#addr2> .\n",
        "<http://example.org/ns#addr2> <http://example.org/ns#city> \"Springfield\" .\n",
    );
    assert_backends_agree("node-shape-recursion", &shapes, data);
}

#[test]
fn inverse_path_agrees() {
    let shapes = format!(
        r"{PREFIXES}
        ex:ChildShape a sh:NodeShape ;
            sh:targetClass ex:Child ;
            sh:property [ sh:path [ sh:inversePath ex:parent ] ; sh:minCount 1 ] .
        "
    );
    let data = concat!(
        // c1 has an inverse-parent edge → conforms; c2 has none → minCount violation
        "<http://example.org/ns#c1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Child> .\n",
        "<http://example.org/ns#x> <http://example.org/ns#parent> <http://example.org/ns#c1> .\n",
        "<http://example.org/ns#c2> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Child> .\n",
    );
    assert_backends_agree("inverse-path", &shapes, data);
}

#[test]
fn target_node_explicit_agrees() {
    let shapes = format!(
        r"{PREFIXES}
        ex:AliceShape a sh:NodeShape ;
            sh:targetNode ex:alice ;
            sh:property [ sh:path ex:name ; sh:minCount 1 ] .
        "
    );
    let data = "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n";
    assert_backends_agree("targetNode", &shapes, data);
}

#[test]
fn and_or_xone_logical_constraints_agree() {
    let shapes = format!(
        r"{PREFIXES}
        ex:HasName a sh:NodeShape ; sh:property [ sh:path ex:name ; sh:minCount 1 ] .
        ex:HasEmail a sh:NodeShape ; sh:property [ sh:path ex:email ; sh:minCount 1 ] .
        ex:ContactShape a sh:NodeShape ;
            sh:targetClass ex:Contact ;
            sh:xone ( ex:HasName ex:HasEmail ) .
        "
    );
    let data = concat!(
        // ct1 has BOTH name and email → xone(exactly-one) violation
        "<http://example.org/ns#ct1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Contact> .\n",
        "<http://example.org/ns#ct1> <http://example.org/ns#name> \"X\" .\n",
        "<http://example.org/ns#ct1> <http://example.org/ns#email> \"x@e\" .\n",
        // ct2 has only a name → conforms
        "<http://example.org/ns#ct2> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Contact> .\n",
        "<http://example.org/ns#ct2> <http://example.org/ns#name> \"Y\" .\n",
    );
    assert_backends_agree("and-or-xone", &shapes, data);
}

#[test]
fn sparql_constraint_agrees() {
    // SHACL-AF `sh:sparql` constraint: the IR backend lazily materializes an
    // oxigraph Store for SPARQL evaluation; the result must still match the oracle.
    let shapes = format!(
        r#"{PREFIXES}
        ex:SelfRefShape a sh:NodeShape ;
            sh:targetClass ex:Node ;
            sh:sparql [
                sh:select "SELECT $this WHERE {{ $this <http://example.org/ns#self> $this . }}" ;
            ] .
        "#
    );
    let data = concat!(
        // n1 is self-referencing → the SPARQL constraint fires
        "<http://example.org/ns#n1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Node> .\n",
        "<http://example.org/ns#n1> <http://example.org/ns#self> <http://example.org/ns#n1> .\n",
        // n2 is not self-referencing → no result
        "<http://example.org/ns#n2> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Node> .\n",
    );
    assert_backends_agree("sparql-constraint", &shapes, data);
}

#[test]
fn sparql_target_agrees() {
    // SHACL-AF `sh:SPARQLTarget`: focus-node resolution runs through SPARQL, so the
    // IR backend must materialize its store and agree with the oracle.
    let shapes = format!(
        r#"{PREFIXES}
        ex:FooShape a sh:NodeShape ;
            sh:target [
                a sh:SPARQLTarget ;
                sh:select "SELECT ?this WHERE {{ ?this <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Foo> . }}" ;
            ] ;
            sh:property [ sh:path ex:label ; sh:minCount 1 ] .
        "#
    );
    let data = concat!(
        // f1 is a Foo lacking ex:label → minCount violation on the SPARQL-targeted node
        "<http://example.org/ns#f1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Foo> .\n",
        "<http://example.org/ns#f2> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Foo> .\n",
        "<http://example.org/ns#f2> <http://example.org/ns#label> \"F2\" .\n",
    );
    assert_backends_agree("sparql-target", &shapes, data);
}

#[test]
fn multi_focus_determinism_agrees() {
    // Several violating focus nodes — exercises the deterministic sort across both
    // backends so the serialized reports line up.
    let shapes = format!(
        r"{PREFIXES}
        ex:PersonShape a sh:NodeShape ;
            sh:targetClass ex:Person ;
            sh:property [ sh:path ex:name ; sh:minCount 1 ] .
        "
    );
    let data = concat!(
        "<http://example.org/ns#zeta> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#alpha> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#mu> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
    );
    assert_backends_agree("multi-focus-determinism", &shapes, data);
}

#[test]
fn reifier_shape_over_rdf12_triple_term_agrees() {
    // RDF 1.2 reifier-shape validation: a `rdf:reifies` binding to a quoted triple,
    // expressed in Turtle. Through the IR backend these reifier `rdf:reifies` quads
    // are materialized in the quad table; the oracle reads them as plain quads. Both
    // must reach the reifier shape's inner constraint identically.
    let shapes = format!(
        r"{PREFIXES}
        ex:S a sh:NodeShape ;
            sh:targetNode ex:a ;
            sh:property [
                sh:path ex:p ;
                sh:reifierShape [
                    sh:property [ sh:path ex:confidence ; sh:minCount 1 ] ;
                ] ;
            ] .
        "
    );
    // ex:a ex:p ex:b is reified by ex:r; ex:r has NO ex:confidence → the reifier
    // shape's inner minCount fires.
    let data_ttl = format!(
        r"{PREFIXES}
        ex:a ex:p ex:b .
        ex:r rdf:reifies <<( ex:a ex:p ex:b )>> .
        "
    );
    assert_backends_agree_store("reifier-shape", &shapes, &load_data_turtle(&data_ttl));
}
