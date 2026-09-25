// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `sh:closed sh:ByTypes` (SHACL 1.2 Core §7.9.1), against the specification's
//! own text:
//!
//! > If $closed is sh:ByTypes, then P is the set of IRI properties that can be
//! > reached from the value node via the following algorithm, plus rdf:type:
//! >
//! > ```text
//! > function collectProperties(S)
//! >     add all IRI properties that can be reached from S via the SPARQL path
//! >             sh:property/sh:path
//! >     if S is a SHACL instance of rdfs:Class in the shapes graph {
//! >         for each triple in the shapes graph matching (S rdfs:subClassOf ?o)
//! >             collectProperties(?o)
//! >         for each triple in the shapes graph matching (?s sh:targetClass S)
//! >             collectProperties(?s)
//! >     }
//! >     if S is a SHACL instance of sh:NodeShape in the shapes graph
//! >         for each triple in the shapes graph matching (S sh:node ?o)
//! >             collectProperties(?o)
//! > for each rdf:type T of the value node in the data graph
//! >     collectProperties(T)
//! > ```
//!
//! Every test validates a DATA graph separate from the shapes graph, so a read of
//! the wrong graph cannot pass by accident, and every treatment is set beside a
//! control whose reported set differs from it.

use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_shapes::engine::{parse_shapes, validate_dataset_with_shapes_graph};
use purrdf_shapes::report::ValidationReport;
use purrdf_shapes::shapes::{ClosedMode, Constraint};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const PREFIXES: &str = "
@prefix ex:    <http://example.org/ns#> .
@prefix rdf:   <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs:  <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh:    <http://www.w3.org/ns/shacl#> .
";

fn data(data_ttl: &str) -> Arc<RdfDataset> {
    parse_turtle_to_dataset(&format!("{PREFIXES}{data_ttl}"), None).expect("data parses")
}

#[track_caller]
fn validate(shapes_ttl: &str, data_ttl: &str) -> ValidationReport {
    let shapes = parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None)
        .unwrap_or_else(|error| panic!("the shapes graph must load: {error}"));
    validate_dataset_with_shapes_graph(&data(data_ttl), &shapes, None).expect("validation runs")
}

/// The local name of the `sh:resultPath` of every `sh:ClosedConstraintComponent`
/// result, sorted: the properties the closed shape did NOT permit.
fn reported(report: &ValidationReport) -> Vec<String> {
    let mut out: Vec<String> = report
        .results
        .iter()
        .filter(|result| {
            result
                .source_constraint_component
                .as_str()
                .ends_with("#ClosedConstraintComponent")
        })
        .map(|result| {
            let path = result
                .result_path
                .as_ref()
                .expect("a closed result names its predicate")
                .to_string();
            path.trim_start_matches('<')
                .trim_end_matches('>')
                .rsplit(['#', '/'])
                .next()
                .expect("an IRI has a local name")
                .to_owned()
        })
        .collect();
    out.sort();
    out
}

/// A class hierarchy declared in the shapes graph, a shape targeting the
/// superclass, a shape targeting an unrelated class, and the closed shape
/// targeting the subclass, whose mode is the parameter.
fn hierarchy(mode: &str, extra: &str) -> String {
    format!(
        "ex:Base a rdfs:Class .
         ex:Sub a rdfs:Class ; rdfs:subClassOf ex:Base .
         ex:Other a rdfs:Class .
         ex:BaseShape a sh:NodeShape ; sh:targetClass ex:Base ;
             sh:property [ sh:path ex:inherited ] .
         ex:OtherShape a sh:NodeShape ; sh:targetClass ex:Other ;
             sh:property [ sh:path ex:otherProp ] .
         ex:SubShape a sh:NodeShape ; sh:targetClass ex:Sub ; sh:closed {mode} ;
             sh:property [ sh:path ex:own ] {extra} ."
    )
}

const HIERARCHY_DATA: &str =
    "ex:a a ex:Sub ; ex:own 1 ; ex:inherited 2 ; ex:otherProp 3 ; ex:stray 4 .";

/// A property only a shape targeting the focus node's SUPERCLASS declares is
/// permitted under `sh:ByTypes` and reported under `true`; `rdf:type` is permitted
/// under `sh:ByTypes` and reported under `true`; a property declared only by a
/// shape for an unrelated class, and one no shape declares, are reported under
/// both.
#[test]
fn superclass_shapes_permit_under_by_types_and_not_under_true() {
    assert_eq!(
        reported(&validate(&hierarchy("sh:ByTypes", ""), HIERARCHY_DATA)),
        vec!["otherProp", "stray"]
    );
    assert_eq!(
        reported(&validate(&hierarchy("true", ""), HIERARCHY_DATA)),
        vec!["inherited", "otherProp", "stray", "type"]
    );
}

/// A focus node with a second type collects that type's properties too.
#[test]
fn every_type_of_the_value_node_contributes() {
    let data = "ex:a a ex:Sub, ex:Other ; ex:own 1 ; ex:inherited 2 ; ex:otherProp 3 ; \
                ex:stray 4 .";
    assert_eq!(
        reported(&validate(&hierarchy("sh:ByTypes", ""), data)),
        vec!["stray"]
    );
    assert_eq!(
        reported(&validate(&hierarchy("sh:ByTypes", ""), HIERARCHY_DATA)),
        vec!["otherProp", "stray"]
    );
}

/// `sh:ignoredProperties` is permitted under `sh:ByTypes` as under `true`.
#[test]
fn ignored_properties_are_honoured_under_by_types() {
    assert_eq!(
        reported(&validate(
            &hierarchy("sh:ByTypes", "; sh:ignoredProperties ( ex:stray )"),
            HIERARCHY_DATA
        )),
        vec!["otherProp"]
    );
    assert_eq!(
        reported(&validate(&hierarchy("sh:ByTypes", ""), HIERARCHY_DATA)),
        vec!["otherProp", "stray"]
    );
}

/// `P` is what the value node's TYPES reach, "plus rdf:type" — the closed shape's
/// own `sh:property` paths are not in it unless its types reach the shape. A
/// focus node reached by `sh:targetNode` with no type is therefore reported for
/// the shape's own property under `sh:ByTypes`, where `true` permits it.
#[test]
fn the_shapes_own_properties_count_only_when_a_type_reaches_the_shape() {
    let shapes = |mode: &str| {
        format!(
            "ex:S a sh:NodeShape ; sh:targetNode ex:b ; sh:closed {mode} ;
                 sh:property [ sh:path ex:p ] ."
        )
    };
    assert_eq!(
        reported(&validate(&shapes("sh:ByTypes"), "ex:b ex:p 1 .")),
        vec!["p"]
    );
    assert_eq!(
        reported(&validate(&shapes("true"), "ex:b ex:p 1 .")),
        Vec::<String>::new()
    );
}

/// `rdfs:subClassOf` and `sh:targetClass` are followed only from a node that is a
/// SHACL instance of `rdfs:Class` IN THE SHAPES GRAPH — directly, or through a
/// class declared a subclass of `rdfs:Class` there.
#[test]
fn target_class_is_followed_only_from_a_declared_class() {
    let shapes = |declaration: &str| {
        format!(
            "{declaration}
             ex:PlainShape a sh:NodeShape ; sh:targetClass ex:Plain ; sh:closed sh:ByTypes ;
                 sh:property [ sh:path ex:q ] ."
        )
    };
    let data = "ex:c a ex:Plain ; ex:q 1 .";
    assert_eq!(reported(&validate(&shapes(""), data)), vec!["q"]);
    assert_eq!(
        reported(&validate(&shapes("ex:Plain a rdfs:Class ."), data)),
        Vec::<String>::new()
    );
    assert_eq!(
        reported(&validate(
            &shapes("ex:Meta rdfs:subClassOf rdfs:Class . ex:Plain a ex:Meta ."),
            data
        )),
        Vec::<String>::new()
    );
}

/// The superclass step reads the SHAPES graph: an `rdfs:subClassOf` asserted only
/// in the data graph does not reach the superclass's shapes.
#[test]
fn subclass_links_are_read_from_the_shapes_graph() {
    let shapes = |link: &str| {
        format!(
            "ex:Base a rdfs:Class .
             ex:Sub a rdfs:Class {link} .
             ex:BaseShape a sh:NodeShape ; sh:targetClass ex:Base ;
                 sh:property [ sh:path ex:inherited ] .
             ex:Closed a sh:NodeShape ; sh:targetNode ex:d ; sh:closed sh:ByTypes ."
        )
    };
    let data = "ex:Sub rdfs:subClassOf ex:Base . ex:d a ex:Sub ; ex:inherited 1 .";
    assert_eq!(reported(&validate(&shapes(""), data)), vec!["inherited"]);
    assert_eq!(
        reported(&validate(&shapes("; rdfs:subClassOf ex:Base"), data)),
        Vec::<String>::new()
    );
}

/// `sh:node` is followed from a node shape, and a non-IRI path is not an "IRI
/// property": an inverse path permits no outgoing predicate.
#[test]
fn node_references_are_followed_and_complex_paths_permit_nothing() {
    let shapes = |reference: &str| {
        format!(
            "ex:K a rdfs:Class, sh:NodeShape ; sh:closed sh:ByTypes {reference} ;
                 sh:property [ sh:path [ sh:inversePath ex:inv ] ] .
             ex:Referenced a sh:NodeShape ; sh:property [ sh:path ex:viaNode ] ."
        )
    };
    let data = "ex:e a ex:K ; ex:viaNode 1 ; ex:inv 2 .";
    assert_eq!(
        reported(&validate(&shapes("; sh:node ex:Referenced"), data)),
        vec!["inv"]
    );
    assert_eq!(
        reported(&validate(&shapes(""), data)),
        vec!["inv", "viaNode"]
    );
}

/// Every `sh:closed sh:ByTypes` constraint of a shapes graph shares ONE index,
/// which holds exactly what `collectProperties` collects for each type.
#[test]
fn one_index_is_shared_and_holds_the_collected_properties() {
    let shapes = parse_shapes(
        &format!(
            "{PREFIXES}{}
             ex:Second a sh:NodeShape ; sh:targetNode ex:z ; sh:closed sh:ByTypes .",
            hierarchy("sh:ByTypes", "")
        ),
        None,
    )
    .expect("loads");
    let indexes: Vec<_> = shapes
        .node_shapes
        .iter()
        .flat_map(|shape| &shape.constraints)
        .filter_map(|constraint| match constraint {
            Constraint::Closed {
                mode: ClosedMode::ByTypes(index),
                ..
            } => Some(index),
            _ => None,
        })
        .collect();
    assert_eq!(indexes.len(), 2);
    assert!(Arc::ptr_eq(indexes[0], indexes[1]));
    let collected = |local: &str| -> Vec<String> {
        indexes[0]
            .properties(&purrdf_shapes::term::Term::NamedNode(
                purrdf_shapes::term::NamedNode::new_unchecked(format!(
                    "http://example.org/ns#{local}"
                )),
            ))
            .iter()
            .map(|property| property.as_str().to_owned())
            .collect()
    };
    assert_eq!(
        collected("Sub"),
        vec![
            "http://example.org/ns#inherited".to_owned(),
            "http://example.org/ns#own".to_owned()
        ]
    );
    assert_eq!(collected("Base"), vec!["http://example.org/ns#inherited"]);
    assert_eq!(collected("Unrelated"), Vec::<String>::new());
}
