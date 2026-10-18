// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `sh:closed` on a property shape (SHACL 1.2 Core §7.9.1), against the
//! specification's own text:
//!
//! > then there is a validation result for each triple that has a value node as
//! > its subject and a predicate that is not in P. [...] The validation result
//! > MUST have the predicate of the triple as its sh:resultPath, and the object of
//! > the triple as its sh:value.
//!
//! The value nodes of a property shape are the nodes its path reaches from the
//! focus node, so the closed check runs over each of them, while the result's
//! `sh:focusNode` stays the property shape's focus node. Each refusal is set
//! beside a neighbour that conforms.

#[path = "support/roles.rs"]
mod roles;
#[path = "support/turtle.rs"]
mod turtle;

use roles::role_iris;

use purrdf_shapes::ShapesImports;
use purrdf_shapes::engine::ValidationOptions;
use purrdf_shapes::engine::{
    parse_shapes, parse_shapes_with_options, validate_dataset, validate_dataset_with_shapes_graph,
};
use purrdf_shapes::model::BoxRoleVocab;
use purrdf_shapes::profile::ShaclProfile;
use purrdf_shapes::report::ValidationReport;

const PREFIXES: &str = "
@prefix ex:    <http://example.org/ns#> .
@prefix rdf:   <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs:  <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh:    <http://www.w3.org/ns/shacl#> .
";

const DATA: &str = "
ex:Alice ex:address ex:Address1 .
ex:Address1 ex:city \"Berlin\" .
ex:Bob ex:address ex:Address2 .
ex:Address2 ex:city \"Paris\" ; ex:zip \"75001\" .
";

#[track_caller]
fn validate(property_shape_body: &str, shapes_extra: &str, data: &str) -> ValidationReport {
    let shapes = parse_shapes(
        &format!(
            "{PREFIXES}
             ex:S a sh:NodeShape ; sh:property ex:P ; sh:targetNode ex:Alice , ex:Bob .
             ex:P sh:path ex:address ; {property_shape_body} .
             {shapes_extra}"
        ),
        None,
    )
    .unwrap_or_else(|error| panic!("the shapes graph must load: {error}"));
    validate_dataset_with_shapes_graph(&turtle::data(PREFIXES, data), &shapes, None)
        .expect("validation runs")
}

/// `(focus node, result path, value)` of every closed result, sorted.
#[track_caller]
fn closed_results(report: &ValidationReport) -> Vec<(String, String, String)> {
    let mut out: Vec<_> = report
        .results
        .iter()
        .map(|result| {
            assert!(
                result
                    .source_constraint_component
                    .as_str()
                    .ends_with("#ClosedConstraintComponent"),
                "only closed results are expected: {result:?}"
            );
            assert_eq!(result.source_shape.to_string(), "<http://example.org/ns#P>");
            (
                result.focus_node.to_string(),
                result.result_path.as_ref().expect("a path").to_string(),
                result.value.as_ref().expect("a value").to_string(),
            )
        })
        .collect();
    out.sort();
    out
}

/// The report has no result at all.
#[track_caller]
fn assert_conforms(report: &ValidationReport) {
    assert!(report.conforms, "expected a conforming report: {report:?}");
    assert_eq!(report.results.len(), 0);
}

/// The W3C `core/property/closed-001` shape: no `sh:property` on the property
/// shape, so only the ignored properties are allowed at its value nodes.
#[test]
fn closed_property_shape_reports_each_unpermitted_triple_at_a_value_node() {
    let report = validate(
        "sh:closed true ; sh:ignoredProperties ( ex:city )",
        "",
        DATA,
    );
    assert_eq!(
        closed_results(&report),
        vec![(
            "<http://example.org/ns#Bob>".to_owned(),
            "<http://example.org/ns#zip>".to_owned(),
            "\"75001\"".to_owned(),
        )]
    );
}

/// The neighbouring valid cases: a property listed in `sh:ignoredProperties`, a
/// property a nested `sh:property` declares, and `sh:closed false` each conform.
#[test]
fn closed_property_shape_conforms_when_every_predicate_is_permitted() {
    let ignored = validate(
        "sh:closed true ; sh:ignoredProperties ( ex:city ex:zip )",
        "",
        DATA,
    );
    assert_conforms(&ignored);

    let declared = validate(
        "sh:closed true ; sh:ignoredProperties ( ex:city ) ; sh:property [ sh:path ex:zip ]",
        "",
        DATA,
    );
    assert_conforms(&declared);

    let open = validate("sh:closed false", "", DATA);
    assert_conforms(&open);
}

/// A closed property shape says nothing about the focus node itself: it has
/// outgoing triples the shape does not list, and none is reported. A literal value
/// node has no outgoing triples. No value node, no result.
#[test]
fn closed_property_shape_checks_value_nodes_only() {
    let report = validate(
        "sh:closed true ; sh:ignoredProperties ( ex:city ex:zip )",
        "",
        "ex:Alice ex:address ex:Address1 ; ex:other 1 . ex:Address1 ex:city \"Berlin\" .
         ex:Bob ex:address \"literal\" .",
    );
    assert_conforms(&report);
}

/// A closed property shape reports a violating predicate at each of several value
/// nodes, once per triple.
#[test]
fn closed_property_shape_reports_per_value_node_and_triple() {
    let report = validate(
        "sh:closed true ; sh:ignoredProperties ( ex:city )",
        "",
        "ex:Alice ex:address ex:A1 , ex:A2 .
         ex:A1 ex:zip 1 .
         ex:A2 ex:zip 2 ; ex:street \"x\" .",
    );
    assert_eq!(
        closed_results(&report),
        vec![
            (
                "<http://example.org/ns#Alice>".to_owned(),
                "<http://example.org/ns#street>".to_owned(),
                "\"x\"".to_owned(),
            ),
            (
                "<http://example.org/ns#Alice>".to_owned(),
                "<http://example.org/ns#zip>".to_owned(),
                "\"1\"^^<http://www.w3.org/2001/XMLSchema#integer>".to_owned(),
            ),
            (
                "<http://example.org/ns#Alice>".to_owned(),
                "<http://example.org/ns#zip>".to_owned(),
                "\"2\"^^<http://www.w3.org/2001/XMLSchema#integer>".to_owned(),
            ),
        ]
    );
}

/// `sh:ByTypes` on a property shape collects properties through the VALUE node's
/// `rdf:type`: the class's shape permits `zip`, `rdf:type` is always permitted, and
/// `street` is reported. Under `sh:closed true` the same value node also reports
/// `rdf:type` and `zip`, which the property shape does not declare.
#[test]
fn by_types_on_a_property_shape_collects_through_the_value_nodes_types() {
    let shapes_extra = "ex:Addr a rdfs:Class .
         ex:AddrShape a sh:NodeShape ; sh:targetClass ex:Addr ;
             sh:property [ sh:path ex:zip ] .";
    let data = "ex:Alice ex:address ex:A1 . ex:A1 a ex:Addr ; ex:zip 1 ; ex:street \"x\" .";

    let by_types = validate("sh:closed sh:ByTypes", shapes_extra, data);
    assert_eq!(
        closed_results(&by_types),
        vec![(
            "<http://example.org/ns#Alice>".to_owned(),
            "<http://example.org/ns#street>".to_owned(),
            "\"x\"".to_owned(),
        )]
    );

    let closed_true = validate("sh:closed true", shapes_extra, data);
    let paths: Vec<_> = closed_results(&closed_true)
        .into_iter()
        .map(|(_, path, _)| path)
        .collect();
    assert_eq!(
        paths,
        vec![
            "<http://example.org/ns#street>".to_owned(),
            "<http://example.org/ns#zip>".to_owned(),
            "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>".to_owned(),
        ]
    );

    let conforming = "ex:Alice ex:address ex:A1 . ex:A1 a ex:Addr ; ex:zip 1 .";
    assert_conforms(&validate("sh:closed sh:ByTypes", shapes_extra, conforming));
}

const META: &str = "http://example.org/ns/meta#";

/// The property shape's `sh:closed` result under `profile`, with a box-role
/// vocabulary configured.
#[track_caller]
fn closed_result_under(profile: ShaclProfile) -> ValidationReport {
    let shapes = parse_shapes_with_options(
        &format!(
            "{PREFIXES}
             @prefix meta: <{META}> .
             ex:S a sh:NodeShape ; sh:property ex:P ; sh:targetNode ex:Alice , ex:Bob ;
                 meta:graphBoxRole meta:boxTBox .
             ex:P sh:path ex:address ; sh:closed true ; sh:ignoredProperties ( ex:city ) ;
                 meta:graphBoxRole meta:boxABox ."
        ),
        None,
        Some(BoxRoleVocab::for_namespace(META)),
        None,
        &ShapesImports::new(),
        &ValidationOptions::default().with_profile(profile),
    )
    .unwrap_or_else(|error| panic!("the shapes graph must load: {error}"));
    let data = turtle::data(
        &format!("{PREFIXES} @prefix meta: <{META}> ."),
        &format!(
            "{DATA}
             ex:zip meta:graphBoxRole meta:boxRBox .
             ex:address meta:graphBoxRole meta:boxCBoxOther ."
        ),
    );
    validate_dataset(&data, &shapes).expect("validation runs")
}

/// A property-shape closed result's source roles are the shapes' (parent merged
/// with the property shape), and its path roles describe the OFFENDING predicate
/// (`ex:zip`), as a node-shape closed result's do, not the shape's own path
/// (`ex:address`, which carries a different role in the data).
#[test]
fn closed_result_path_roles_describe_the_offending_predicate() {
    let report = closed_result_under(ShaclProfile::WD_20260918);
    assert_eq!(report.results.len(), 1);
    let result = &report.results[0];
    assert_eq!(
        role_iris(&result.source_box_roles),
        vec![format!("{META}boxABox"), format!("{META}boxTBox")]
    );
    assert_eq!(
        role_iris(&result.path_box_roles),
        vec![format!("{META}boxRBox")]
    );
    assert_eq!(
        role_iris(&result.result_box_roles),
        vec![
            format!("{META}boxABox"),
            format!("{META}boxRBox"),
            format!("{META}boxTBox")
        ]
    );
}

/// The 2017 Recommendation defines `sh:closed` over value nodes in the same words,
/// so the working draft, which states it again, the editor's draft and the
/// Recommendation report the same result, with the same roles.
#[test]
fn closed_property_shape_reports_the_same_result_under_each_dated_law() {
    let dated = closed_result_under(ShaclProfile::WD_20260918);
    let editors = closed_result_under(ShaclProfile::ED_20261016);
    let recommendation = closed_result_under(ShaclProfile::REC_20170720);
    assert_eq!(
        role_iris(&dated.results[0].path_box_roles),
        role_iris(&editors.results[0].path_box_roles)
    );
    for report in [&dated, &editors, &recommendation] {
        assert_eq!(report.results.len(), 1);
        let result = &report.results[0];
        assert_eq!(result.focus_node.to_string(), "<http://example.org/ns#Bob>");
        assert_eq!(
            result.result_path.as_ref().unwrap().to_string(),
            "<http://example.org/ns#zip>"
        );
        assert_eq!(result.value.as_ref().unwrap().to_string(), "\"75001\"");
        assert_eq!(result.source_shape.to_string(), "<http://example.org/ns#P>");
    }
    assert_eq!(
        role_iris(&dated.results[0].path_box_roles),
        role_iris(&recommendation.results[0].path_box_roles)
    );
    assert_eq!(
        role_iris(&dated.results[0].source_box_roles),
        role_iris(&recommendation.results[0].source_box_roles)
    );
}
