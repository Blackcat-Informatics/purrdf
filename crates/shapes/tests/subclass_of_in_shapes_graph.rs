// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHACL 1.2 Core §6.3, `subClassOfInShapesGraph`: "SHACL processors SHOULD offer a
//! parameter subClassOfInShapesGraph that, if set to true, should alter the definition of
//! SHACL Type so that the rdfs:subClassOf triples are queried from the shapes graph in
//! addition to the data graph. The rdf:type triples are always expected to be in the data
//! graph."
//!
//! Every case validates the same data twice, with the parameter off (the specification's
//! default) and on, and each carries a control row whose verdict is the same both ways, so
//! a difference is observed against a shape that is demonstrably active.

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf_rdf::ir::ViewLimits;
use purrdf_rdf::{DatasetMut, MutableDataset, QuadValues, TermValue};
use purrdf_shapes::engine::{
    PreparedShapes, ValidationOptions, parse_shapes, validate_dataset_with_shapes_graph,
};
use purrdf_shapes::shapes::Shapes;
use purrdf_shapes::term::{NamedNode, Term};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const PREFIXES: &str = "@prefix ex: <http://example.org/ns#> .\n\
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
    @prefix sh: <http://www.w3.org/ns/shacl#> .\n";

fn shapes(turtle: &str, on: bool) -> Shapes {
    let mut shapes = parse_shapes(&format!("{PREFIXES}{turtle}"), None).expect("shapes parse");
    shapes
        .set_validation_options(ValidationOptions::default().with_subclass_of_in_shapes_graph(on));
    shapes
}

/// The focus nodes of every result, through the free function and through a prepared
/// binding — the two must agree.
fn focus_nodes(shapes_turtle: &str, data: &str, on: bool) -> BTreeSet<String> {
    let shapes = shapes(shapes_turtle, on);
    let data = parse_turtle_to_dataset(&format!("{PREFIXES}{data}"), None).expect("data parses");
    let free: BTreeSet<String> = validate_dataset_with_shapes_graph(&data, &shapes, None)
        .expect("validates")
        .results
        .iter()
        .map(|result| result.focus_node.to_string())
        .collect();
    let prepared: BTreeSet<String> = PreparedShapes::new(Arc::new(shapes))
        .bind_dataset(&data)
        .expect("binds")
        .validate()
        .expect("validates")
        .results
        .iter()
        .map(|result| result.focus_node.to_string())
        .collect();
    assert_eq!(
        free, prepared,
        "the free function and a prepared binding agree"
    );
    free
}

fn nodes(names: &[&str]) -> BTreeSet<String> {
    names
        .iter()
        .map(|name| format!("<http://example.org/ns#{name}>"))
        .collect()
}

/// A class target reached only through the shapes graph's `rdfs:subClassOf` chain — two
/// hops, through a class and to a target class the data graph never mentions — fires with
/// the parameter on and not with it off. The control, a direct instance of the target
/// class, fires both ways.
#[test]
fn a_class_target_reached_through_the_shapes_graph_fires_only_when_on() {
    let shapes = "ex:Student rdfs:subClassOf ex:Scholar .\n\
        ex:Scholar rdfs:subClassOf ex:Person .\n\
        ex:PersonShape a sh:NodeShape ; sh:targetClass ex:Person ;\n\
            sh:property [ sh:path ex:name ; sh:minCount 1 ] .\n";
    let data = "ex:alice a ex:Student .\n";
    assert_eq!(focus_nodes(shapes, data, false), nodes(&[]));
    assert_eq!(focus_nodes(shapes, data, true), nodes(&["alice"]));

    let with_control = "ex:alice a ex:Student .\nex:bob a ex:Person .\n";
    assert_eq!(focus_nodes(shapes, with_control, false), nodes(&["bob"]));
    assert_eq!(
        focus_nodes(shapes, with_control, true),
        nodes(&["alice", "bob"])
    );
}

/// `sh:class` reads the same SHACL type: a member typed only with a subclass the shapes
/// graph declares violates with the parameter off and conforms with it on. The control, a
/// member of an unrelated class, violates both ways.
#[test]
fn sh_class_follows_the_shapes_graph_only_when_on() {
    let shapes = "ex:Student rdfs:subClassOf ex:Person .\n\
        ex:TeamShape a sh:NodeShape ; sh:targetClass ex:Team ;\n\
            sh:property [ sh:path ex:member ; sh:class ex:Person ] .\n";
    let data = "ex:t1 a ex:Team ; ex:member ex:alice .\nex:alice a ex:Student .\n\
        ex:t2 a ex:Team ; ex:member ex:rock .\nex:rock a ex:Stone .\n";
    assert_eq!(focus_nodes(shapes, data, false), nodes(&["t1", "t2"]));
    assert_eq!(focus_nodes(shapes, data, true), nodes(&["t2"]));
}

/// Only SHACL type changes: the shapes graph's `rdfs:subClassOf` triple is not a triple of
/// the data graph, so a target over `rdfs:subClassOf` still reads the data graph alone. The
/// control, the same kind of triple asserted IN the data graph, is targeted both ways.
#[test]
fn a_shapes_graph_subclass_triple_is_not_data() {
    let shapes = "ex:Student rdfs:subClassOf ex:Person .\n\
        ex:SubclassShape a sh:NodeShape ; sh:targetSubjectsOf rdfs:subClassOf ;\n\
            sh:nodeKind sh:Literal .\n";
    let data = "ex:Chair rdfs:subClassOf ex:Furniture .\n";
    assert_eq!(focus_nodes(shapes, data, false), nodes(&["Chair"]));
    assert_eq!(focus_nodes(shapes, data, true), nodes(&["Chair"]));
}

/// The parameter reaches a binding over a mutation snapshot, and the change path still
/// answers for that snapshot: adding a subclass instance moves a verdict, and the
/// expansion covers the moved focus node.
#[test]
fn a_mutation_snapshot_binding_follows_the_shapes_graph_and_keeps_its_change_path() {
    let shapes = shapes(
        "ex:Student rdfs:subClassOf ex:Person .\n\
         ex:PersonShape a sh:NodeShape ; sh:targetClass ex:Person ;\n\
             sh:property [ sh:path ex:name ; sh:minCount 1 ] .\n",
        true,
    );
    let prepared = PreparedShapes::new(Arc::new(shapes));
    let base =
        parse_turtle_to_dataset(&format!("{PREFIXES}ex:bob a ex:Person .\n"), None).expect("data");
    let mut mutation = MutableDataset::new(Arc::clone(&base));
    assert!(
        mutation
            .insert(QuadValues {
                s: TermValue::iri("http://example.org/ns#alice"),
                p: TermValue::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
                o: TermValue::iri("http://example.org/ns#Student"),
                g: None,
            })
            .expect("insert")
    );
    let snapshot = Arc::new(mutation.snapshot_view().expect("snapshot"));
    let validator = prepared
        .bind_delta_with_shapes_graph(Arc::clone(&snapshot), None, ViewLimits::default())
        .expect("delta bind");
    let focus: BTreeSet<String> = validator
        .validate()
        .expect("validates")
        .results
        .iter()
        .map(|result| result.focus_node.to_string())
        .collect();
    assert_eq!(focus, nodes(&["alice", "bob"]));
    let expansion = validator
        .affected_focus_node_ids(&snapshot)
        .expect("the change path answers for the snapshot it was bound to");
    let ids = expansion
        .ids()
        .expect("a Core shapes graph has a bounded expansion");
    let alice = validator
        .term_id(&Term::NamedNode(NamedNode::from(
            "http://example.org/ns#alice",
        )))
        .expect("the snapshot interns alice");
    assert!(
        ids.contains(&alice),
        "the moved focus node is in the expansion"
    );
}

/// SHACL type follows the transitive `rdfs:subClassOf` closure whatever the kind of its
/// intermediate nodes: one blank node between two IRI classes of the shapes graph connects
/// them, both for a class target and for `sh:class`. The controls — a direct instance of
/// the target class, a member of an unrelated class — answer the same both ways.
#[test]
fn a_blank_intermediate_in_the_shapes_graph_connects_iri_classes() {
    let targets = "ex:Student rdfs:subClassOf _:b .\n_:b rdfs:subClassOf ex:Person .\n\
        ex:PersonShape a sh:NodeShape ; sh:targetClass ex:Person ;\n\
            sh:property [ sh:path ex:name ; sh:minCount 1 ] .\n";
    let data = "ex:alice a ex:Student .\nex:bob a ex:Person .\n";
    assert_eq!(focus_nodes(targets, data, false), nodes(&["bob"]));
    assert_eq!(focus_nodes(targets, data, true), nodes(&["alice", "bob"]));

    let class = "ex:Student rdfs:subClassOf _:b .\n_:b rdfs:subClassOf ex:Person .\n\
        ex:TeamShape a sh:NodeShape ; sh:targetClass ex:Team ;\n\
            sh:property [ sh:path ex:member ; sh:class ex:Person ] .\n";
    let data = "ex:t1 a ex:Team ; ex:member ex:alice .\nex:alice a ex:Student .\n\
        ex:t2 a ex:Team ; ex:member ex:rock .\nex:rock a ex:Stone .\n";
    assert_eq!(focus_nodes(class, data, false), nodes(&["t1", "t2"]));
    assert_eq!(focus_nodes(class, data, true), nodes(&["t2"]));
}

/// Several blank hops, a cycle among them (which must terminate), and an IRI edge past the
/// blank chain — in the shapes graph and in the data graph — all compose. A blank label
/// in the data graph equal to one in the shapes graph names a different node, so a data
/// node typed with it gains no membership.
#[test]
fn blank_chains_and_cycles_in_the_shapes_graph_compose_and_terminate() {
    let shapes = "ex:Student rdfs:subClassOf _:same .\n\
        _:same rdfs:subClassOf _:second .\n\
        _:second rdfs:subClassOf ex:Scholar, _:same .\n\
        ex:Scholar rdfs:subClassOf ex:Person .\n\
        ex:Loop rdfs:subClassOf _:l1 .\n_:l1 rdfs:subClassOf _:l2 .\n_:l2 rdfs:subClassOf _:l1 .\n\
        ex:PersonShape a sh:NodeShape ; sh:targetClass ex:Person ;\n\
            sh:property [ sh:path ex:name ; sh:minCount 1 ] .\n\
        ex:AgentShape a sh:NodeShape ; sh:targetClass ex:Agent ;\n\
            sh:property [ sh:path ex:id ; sh:minCount 1 ] .\n";
    let data = "ex:alice a ex:Student .\nex:bob a ex:Person .\n\
        ex:collision a _:same .\nex:looper a ex:Loop .\n\
        ex:Person rdfs:subClassOf ex:Agent .\n\
        ex:carol a ex:Student ; ex:name \"Carol\" .\n";
    assert_eq!(focus_nodes(shapes, data, false), nodes(&["bob"]));
    assert_eq!(
        focus_nodes(shapes, data, true),
        nodes(&["alice", "bob", "carol"])
    );
}

/// A blank node that lies on no path from one IRI class to another creates no membership:
/// a chain into a dead-end blank, a blank with no IRI subclass above an IRI class, and a
/// cycle of blanks under an IRI class all leave their members outside `ex:Person`. Four
/// members violate `sh:class ex:Person` with the parameter off and on; the one member whose
/// class reaches `ex:Person` through a blank conforms only with it on.
#[test]
fn a_blank_off_every_iri_path_creates_no_membership() {
    let shapes = "ex:Student rdfs:subClassOf _:b .\n_:b rdfs:subClassOf ex:Person .\n\
        ex:Orphan rdfs:subClassOf _:dead .\n\
        _:floating rdfs:subClassOf ex:Person .\n\
        ex:Spinner rdfs:subClassOf _:c1 .\n_:c1 rdfs:subClassOf _:c2 .\n_:c2 rdfs:subClassOf _:c1 .\n\
        ex:Robot rdfs:subClassOf _:r .\n_:r rdfs:subClassOf ex:Machine .\n\
        ex:TeamShape a sh:NodeShape ; sh:targetClass ex:Team ;\n\
            sh:property [ sh:path ex:member ; sh:class ex:Person ] .\n";
    let data = "ex:t0 a ex:Team ; ex:member ex:alice .\nex:alice a ex:Student .\n\
        ex:t1 a ex:Team ; ex:member ex:orphan .\nex:orphan a ex:Orphan .\n\
        ex:t2 a ex:Team ; ex:member ex:spinner .\nex:spinner a ex:Spinner .\n\
        ex:t3 a ex:Team ; ex:member ex:robot .\nex:robot a ex:Robot .\n\
        ex:t4 a ex:Team ; ex:member ex:untyped .\n";
    assert_eq!(
        focus_nodes(shapes, data, false),
        nodes(&["t0", "t1", "t2", "t3", "t4"])
    );
    assert_eq!(
        focus_nodes(shapes, data, true),
        nodes(&["t1", "t2", "t3", "t4"])
    );
}
