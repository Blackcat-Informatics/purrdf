// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Constraint instantiation, vocabulary coexistence, and parameter integrity.

use std::fmt::Write as _;
use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_shapes::engine::validate_dataset;
use purrdf_shapes::model::sh;
use purrdf_shapes::shapes::{Constraint, Shapes, from_dataset};

const PREFIXES: &str = r"
@prefix ex: <http://example.org/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
";

fn dataset(body: &str) -> Arc<RdfDataset> {
    purrdf::parse_dataset(format!("{PREFIXES}{body}").as_bytes(), "text/turtle", None)
        .expect("RDF fixture parses")
}

fn shapes(body: &str) -> Result<Shapes, String> {
    from_dataset(&dataset(body))
}

#[test]
fn vocabulary_declarations_preserve_all_fifteen_property_constraints() {
    let mut body = String::from("ex:Shape a sh:NodeShape ; sh:targetNode ex:focus .\n");
    let mut valid = String::new();
    for index in 0..15 {
        writeln!(body, "ex:Shape sh:property ex:Property{index} .").unwrap();
        writeln!(
            body,
            "ex:Property{index} sh:path ex:p{index} ; sh:minCount 1 ."
        )
        .unwrap();
        writeln!(valid, "ex:focus ex:p{index} ex:value .").unwrap();
    }
    let native = shapes(&body).expect("native shape loads");
    // The standard vocabulary declares the component without a SPARQL validator.
    body.push_str("sh:PropertyConstraintComponent a sh:ConstraintComponent ; sh:parameter [ sh:path sh:property ] .\n");
    let imported = shapes(&body).expect("vocabulary does not restrict property multiplicity");
    let shape = imported
        .node_shapes
        .iter()
        .find(|s| s.id.to_string() == "<http://example.org/Shape>")
        .unwrap();
    assert_eq!(shape.property_shapes.len(), 15);
    let empty = dataset("");
    let report = validate_dataset(&empty, &imported).unwrap();
    assert!(!report.conforms);
    assert_eq!(report.results.len(), 15);
    assert_eq!(
        format!("{report:?}"),
        format!("{:?}", validate_dataset(&empty, &native).unwrap())
    );
    for result in &report.results {
        assert_eq!(
            result.source_constraint_component.as_str(),
            sh::MIN_COUNT_CONSTRAINT_COMPONENT
        );
        assert!(
            result
                .source_shape
                .to_string()
                .starts_with("<http://example.org/Property")
        );
        assert!(
            result
                .result_path
                .as_ref()
                .unwrap()
                .to_string()
                .starts_with("<http://example.org/p")
        );
    }
    assert!(
        validate_dataset(&dataset(&valid), &imported)
            .unwrap()
            .conforms
    );
}

fn custom_component(validator: &str, values: &str, property: bool) -> String {
    let scope = if property { "sh:path ex:p ;" } else { "" };
    format!(
        r"
ex:Different a sh:ConstraintComponent ;
    sh:parameter [ sh:path ex:forbidden ] ;
    {validator} .
ex:Shape a sh:NodeShape ; sh:targetNode ex:focus ;
    sh:property ex:Property .
ex:Property {scope} ex:forbidden {values} .
"
    )
}

#[test]
fn repeated_single_parameter_validators_apply_conjunctively_in_every_scope() {
    for (property, validator) in [
        (
            true,
            r#"sh:validator [ a sh:SPARQLAskValidator ; sh:ask "ASK { FILTER (?value != ?forbidden) }" ]"#,
        ),
        (
            true,
            r#"sh:propertyValidator [ a sh:SPARQLSelectValidator ; sh:select "SELECT ?this ?value WHERE { ?this $PATH ?value . FILTER (?value = ?forbidden) }" ]"#,
        ),
    ] {
        let mut prior = None;
        for values in ["ex:a, ex:b", "ex:b, ex:a"] {
            let parsed = shapes(&custom_component(validator, values, property)).unwrap();
            let report = validate_dataset(&dataset("ex:focus ex:p ex:a, ex:b ."), &parsed).unwrap();
            assert!(!report.conforms);
            assert_eq!(report.results.len(), 2);
            for result in &report.results {
                assert_eq!(
                    result.source_constraint_component.as_str(),
                    "http://example.org/Different"
                );
                assert_eq!(
                    result.source_shape.to_string(),
                    "<http://example.org/Property>"
                );
            }
            let rendered = format!("{report:?}");
            if let Some(previous) = prior.replace(rendered.clone()) {
                assert_eq!(rendered, previous);
            }
            assert!(
                validate_dataset(&dataset("ex:focus ex:p ex:c ."), &parsed)
                    .unwrap()
                    .conforms
            );
        }
    }
    for validator in [
        r#"sh:validator [ a sh:SPARQLAskValidator ; sh:ask "ASK { FILTER (?value != ?forbidden) }" ]"#,
        r#"sh:nodeValidator [ a sh:SPARQLSelectValidator ; sh:select "SELECT ?this WHERE { FILTER (?this = ?forbidden) }" ]"#,
    ] {
        let body = format!(
            r"ex:Different a sh:ConstraintComponent ; sh:parameter [ sh:path ex:forbidden ] ; {validator} .
            ex:Shape a sh:NodeShape ; sh:targetNode ex:a, ex:b, ex:c ; ex:forbidden ex:a, ex:b ."
        );
        let parsed = shapes(&body).unwrap();
        assert_eq!(parsed.node_shapes[0].constraints.len(), 2);
        assert_eq!(
            validate_dataset(&dataset(""), &parsed)
                .unwrap()
                .results
                .len(),
            2
        );
    }
}

#[test]
fn multiple_parameters_reject_duplicates_even_when_another_required_value_is_absent() {
    for optional in ["true", "false"] {
        for (first, second) in [("a", "z"), ("z", "a")] {
            let body = format!(
                r"
ex:Component a sh:ConstraintComponent ;
    sh:parameter [ sh:path ex:{first} ], [ sh:path ex:{second} ; sh:optional {optional} ] .
ex:Shape a sh:NodeShape ; ex:{second} 1, 2 .
"
            );
            let error = shapes(&body).expect_err("multi-parameter duplicates must fail");
            assert!(error.contains("only one is allowed"), "{error}");
        }
    }
}

#[test]
fn optional_and_required_parameters_control_instantiation() {
    let declaration = r#"ex:Component a sh:ConstraintComponent ;
        sh:parameter [ sh:path ex:required ], [ sh:path ex:optional ; sh:optional true ] ;
        sh:validator [ a sh:SPARQLAskValidator ; sh:ask "ASK { FILTER (false) }" ] ."#;
    for (params, count) in [
        ("ex:optional 1", 0),
        ("ex:required 1", 1),
        ("ex:required 1 ; ex:optional 2", 1),
    ] {
        let parsed = shapes(&format!(
            "{declaration} ex:Shape a sh:NodeShape ; sh:targetNode ex:focus ; {params} ."
        ))
        .unwrap();
        assert_eq!(
            validate_dataset(&dataset(""), &parsed)
                .unwrap()
                .results
                .len(),
            count
        );
    }
}

#[test]
fn malformed_parameter_declarations_fail_at_load() {
    for declaration in [
        "sh:parameter [ sh:path ex:p, 1 ]",
        "sh:parameter [ sh:path ex:p ; sh:optional true, false ]",
        "sh:parameter [ sh:path ex:p ; sh:optional \"true\" ]",
        "sh:parameter [ sh:path ex:p ; sh:optional ex:true ]",
        "sh:parameter [ sh:path ex:p ; sh:optional \"invalid\"^^xsd:boolean ]",
        "sh:parameter [ sh:path ex:p ; sh:optional true ]",
        "sh:parameter [ sh:path ex:p ], [ sh:path ex:p ]",
        "sh:parameter [ sh:path ex:p ], [ sh:path <http://example.org/other/p> ]",
    ] {
        let body = format!(
            "ex:Component a sh:ConstraintComponent ; {declaration} . ex:Shape a sh:NodeShape ."
        );
        assert!(shapes(&body).is_err(), "accepted {declaration}");
    }
    assert!(shapes("ex:Component a sh:ConstraintComponent .").is_err());
}

#[test]
fn repeated_parameter_values_preserve_rdf12_term_identity() {
    let parsed = shapes(
        r#"ex:Component a sh:ConstraintComponent ;
        sh:parameter [ sh:path ex:arg ] ;
        sh:validator [ a sh:SPARQLAskValidator ; sh:ask "ASK { FILTER (false) }" ] .
        ex:Shape a sh:NodeShape ; sh:targetNode ex:focus ;
            ex:arg "text"@en, "text"@en--ltr, "text"@en--rtl, <<( ex:s ex:p ex:o )>> ."#,
    )
    .unwrap();
    let constraints = &parsed.node_shapes[0].constraints;
    assert_eq!(constraints.len(), 4);
    let mut bindings = constraints
        .iter()
        .map(|c| match c {
            Constraint::Component { bindings, .. } => {
                assert_eq!(bindings.len(), 1);
                bindings[0].1.to_string()
            }
            other => panic!("unexpected {other:?}"),
        })
        .collect::<Vec<_>>();
    bindings.sort();
    bindings.dedup();
    assert_eq!(bindings.len(), 4);
    assert_eq!(
        validate_dataset(&dataset(""), &parsed)
            .unwrap()
            .results
            .len(),
        4
    );
}

#[test]
fn imported_native_validator_does_not_duplicate_native_execution() {
    let body = r#"sh:ClassConstraintComponent a sh:ConstraintComponent ;
        sh:parameter [ sh:path sh:class ] ;
        sh:validator [ a sh:SPARQLAskValidator ; sh:ask "ASK { FILTER (false) }" ] .
        ex:Shape a sh:NodeShape ; sh:targetNode ex:focus ; sh:class ex:Class ."#;
    let parsed = shapes(body).unwrap();
    assert_eq!(parsed.node_shapes[0].constraints.len(), 1);
    assert!(
        validate_dataset(&dataset("ex:focus a ex:Class ."), &parsed)
            .unwrap()
            .conforms
    );
}

#[test]
fn repeated_statements_across_graphs_do_not_duplicate_parameters_or_constraints() {
    let graph = r#"ex:Component a sh:ConstraintComponent ;
        sh:parameter ex:Param ;
        sh:validator ex:Validator .
        ex:Param sh:path ex:arg .
        ex:Validator a sh:SPARQLAskValidator ; sh:ask "ASK { FILTER (false) }" .
        ex:Shape a sh:NodeShape ; sh:targetNode ex:focus ; ex:arg ex:a, ex:b ."#;
    let trig = format!("{PREFIXES} ex:g1 {{ {graph} }} ex:g2 {{ {graph} }}");
    let ds = purrdf::parse_dataset(trig.as_bytes(), "application/trig", None).unwrap();
    let parsed = from_dataset(&ds).expect("named graphs contribute distinct object values");
    assert_eq!(parsed.node_shapes[0].constraints.len(), 2);
    assert_eq!(
        validate_dataset(&dataset(""), &parsed)
            .unwrap()
            .results
            .len(),
        2
    );
}

#[test]
fn applicable_scope_validator_takes_precedence_over_generic_fallback() {
    for (scope, kind, path) in [
        ("nodeValidator", "NodeShape", ""),
        ("propertyValidator", "PropertyShape", "sh:path ex:p ;"),
    ] {
        let body = format!(
            r#"ex:Component a sh:ConstraintComponent ;
            sh:parameter [ sh:path ex:arg ] ;
            sh:{scope} [ a sh:SPARQLSelectValidator ; sh:select "SELECT ?this WHERE {{ FILTER(false) }}" ] ;
            sh:validator [ a sh:SPARQLAskValidator ; sh:ask "ASK {{ FILTER(false) }}" ] .
            ex:Shape a sh:{kind} ; {path} sh:targetNode ex:focus ; ex:arg ex:a, ex:b ."#
        );
        let parsed = shapes(&body).unwrap();
        assert!(
            validate_dataset(&dataset("ex:focus ex:p ex:value ."), &parsed)
                .unwrap()
                .conforms
        );
    }
}

#[test]
fn one_validator_is_selected_for_each_parameter_instance() {
    let body = r#"ex:Component a sh:ConstraintComponent ;
        sh:parameter [ sh:path ex:arg ] ;
        sh:validator ex:ValidatorA, ex:ValidatorB .
        ex:ValidatorA a sh:SPARQLAskValidator ; sh:ask "ASK { FILTER (false) }" .
        ex:ValidatorB a sh:SPARQLAskValidator ; sh:ask "ASK { FILTER (false) }" .
        ex:Shape a sh:NodeShape ; sh:targetNode ex:focus ; ex:arg ex:a, ex:b ."#;
    let parsed = shapes(body).unwrap();
    assert_eq!(
        validate_dataset(&dataset(""), &parsed)
            .unwrap()
            .results
            .len(),
        2
    );
}

#[test]
fn native_component_declarations_preserve_every_repeatable_constraint_family() {
    for (parameter, component, values) in [
        ("class", "Class", "ex:A, ex:B"),
        ("node", "Node", "ex:A, ex:B"),
        ("not", "Not", "ex:A, ex:B"),
        ("and", "And", "(ex:A), (ex:B)"),
        ("or", "Or", "(ex:A), (ex:B)"),
        ("xone", "Xone", "(ex:A), (ex:B)"),
        ("hasValue", "HasValue", "ex:a, ex:b"),
        ("equals", "Equals", "ex:a, ex:b"),
        ("disjoint", "Disjoint", "ex:a, ex:b"),
        ("lessThan", "LessThan", "ex:a, ex:b"),
        ("lessThanOrEquals", "LessThanOrEquals", "ex:a, ex:b"),
    ] {
        let body = format!(
            "ex:Shape a sh:PropertyShape ; sh:path ex:p ; sh:targetNode ex:focus ; sh:{parameter} {values} ."
        );
        let native = shapes(&body).unwrap();
        let imported = shapes(&format!("{body} sh:{component}ConstraintComponent a sh:ConstraintComponent ; sh:parameter [ sh:path sh:{parameter} ] .")).unwrap();
        let before = &native.node_shapes[0].property_shapes[0].constraints;
        let after = &imported.node_shapes[0].property_shapes[0].constraints;
        assert_eq!(after.len(), 2, "sh:{parameter}");
        assert_eq!(
            format!("{before:?}"),
            format!("{after:?}"),
            "sh:{parameter}"
        );
        let data = dataset("ex:focus ex:p ex:value .");
        assert_eq!(
            format!("{:?}", validate_dataset(&data, &native).unwrap()),
            format!("{:?}", validate_dataset(&data, &imported).unwrap()),
            "sh:{parameter}",
        );
    }
}

#[test]
fn validator_declarations_enforce_attachment_kind_and_query_datatype() {
    for validator in [
        "sh:nodeValidator [ a sh:SPARQLAskValidator ; sh:ask \"ASK {}\" ]",
        "sh:propertyValidator [ a sh:SPARQLAskValidator ; sh:ask \"ASK {}\" ]",
        "sh:validator [ a sh:SPARQLSelectValidator ; sh:select \"SELECT ?this WHERE {}\" ]",
        "sh:validator [ a sh:SPARQLAskValidator ; sh:ask \"ASK {}\"@en ]",
        "sh:validator [ a sh:SPARQLAskValidator ; sh:ask \"ASK {}\"^^ex:Query ]",
        "sh:nodeValidator [ a sh:SPARQLSelectValidator ; sh:select \"SELECT ?this WHERE {}\"@en ]",
    ] {
        let body = format!(
            "ex:Component a sh:ConstraintComponent ; sh:parameter [ sh:path ex:arg ] ; {validator} ."
        );
        assert!(shapes(&body).is_err(), "accepted {validator}");
    }
}
