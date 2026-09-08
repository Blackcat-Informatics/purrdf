// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL-AF target parameter cardinality, binding, and activation semantics.

use purrdf_shapes::engine::{parse_shapes, validate_graphs};
use purrdf_shapes::report::ValidationReport;
use purrdf_shapes::shapes::{Target, TargetTypeParam};

const DATA: &str = concat!(
    "<http://example.org/alice> <http://example.org/kind> <http://example.org/Kind> .\n",
    "<http://example.org/alice> <http://example.org/label> \"A\" .\n",
    "<http://example.org/bob> <http://example.org/kind> <http://example.org/Kind> .\n",
    "<http://example.org/bob> <http://example.org/label> \"B\" .\n",
);

fn target_shapes(optional: &str, values: &str, reverse_parameter_order: bool) -> String {
    let (kind_order, label_order) = if reverse_parameter_order {
        (1, 0)
    } else {
        (0, 1)
    };
    format!(
        r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:ByKind a sh:SPARQLTargetType ;
            sh:parameter [ sh:path ex:kind ; sh:order {kind_order} ] ;
            sh:parameter [ sh:path ex:label ; sh:order {label_order} ; {optional} ] ;
            sh:select """
                SELECT ?this WHERE {{
                    ?this <http://example.org/kind> $kind .
                    ?this <http://example.org/label> ?name .
                    FILTER (!BOUND($label) || ?name = $label)
                }}
            """ .
        ex:Shape a sh:NodeShape ; sh:target ex:instance ; sh:class ex:Allowed .
        ex:instance a ex:ByKind ; {values} .
        "#
    )
}

fn focus_nodes(report: &ValidationReport) -> Vec<String> {
    assert_eq!(report.conforms, report.results.is_empty());
    let mut nodes = Vec::new();
    for result in &report.results {
        assert_eq!(
            result.source_constraint_component.as_str(),
            "http://www.w3.org/ns/shacl#ClassConstraintComponent",
        );
        assert_eq!(
            result.source_shape.to_string(),
            "<http://example.org/Shape>"
        );
        nodes.push(result.focus_node.to_string());
    }
    nodes.sort();
    nodes
}

#[test]
fn missing_mandatory_target_parameter_contributes_no_focus_nodes() {
    for values in ["", "ex:label \"A\""] {
        for reverse in [false, true] {
            let shapes = target_shapes("sh:optional true ;", values, reverse);
            let parsed = parse_shapes(&shapes, None).expect("inactive targets are legal");
            let shape = parsed
                .node_shapes
                .iter()
                .find(|shape| shape.id.to_string() == "<http://example.org/Shape>")
                .expect("targeted shape");
            assert!(shape.targets.is_empty());
            let report = validate_graphs(DATA, &shapes, None).expect("inactive target validates");
            assert_eq!(focus_nodes(&report), Vec::<String>::new());

            let with_direct_target = format!("{shapes}\nex:Shape sh:targetNode ex:manual .");
            let report = validate_graphs(DATA, &with_direct_target, None)
                .expect("other targets remain active");
            assert_eq!(focus_nodes(&report), ["<http://example.org/manual>"]);
        }
    }
}

#[test]
fn omitted_optional_target_parameter_stays_unbound() {
    for optional in ["sh:optional true ;", "sh:optional \"1\"^^xsd:boolean ;"] {
        let shapes = target_shapes(optional, "ex:kind ex:Kind", false);
        let parsed = parse_shapes(&shapes, None).expect("optional parameter may be absent");
        let declaration = &parsed.target_types["http://example.org/ByKind"];
        let TargetTypeParam { predicate, var } = &declaration.params[0];
        assert_eq!(predicate.as_str(), "http://example.org/kind");
        assert_eq!(var, "kind");
        assert_eq!(declaration.params[1].var, "label");
        let shape = parsed
            .node_shapes
            .iter()
            .find(|shape| shape.id.to_string() == "<http://example.org/Shape>")
            .expect("targeted shape");
        let [Target::Sparql { substitutions, .. }] = shape.targets.as_slice() else {
            panic!("one parameterized target expected");
        };
        assert_eq!(substitutions.len(), 1);
        assert_eq!(substitutions[0].0, "kind");
        assert_eq!(substitutions[0].1.to_string(), "<http://example.org/Kind>");
        let report = validate_graphs(DATA, &shapes, None).expect("unbound optional evaluates");
        assert_eq!(
            focus_nodes(&report),
            ["<http://example.org/alice>", "<http://example.org/bob>"],
        );
    }
}

#[test]
fn supplied_optional_target_parameter_filters_the_focus_set() {
    for (label, focus) in [("A", "alice"), ("B", "bob")] {
        let values = format!("ex:kind ex:Kind ; ex:label \"{label}\"");
        let shapes = target_shapes("sh:optional true ;", &values, false);
        let report = validate_graphs(DATA, &shapes, None).expect("optional value is prebound");
        assert_eq!(
            focus_nodes(&report),
            [format!("<http://example.org/{focus}>")]
        );
        let reordered = target_shapes("sh:optional true ;", &values, true);
        let reordered_report =
            validate_graphs(DATA, &reordered, None).expect("parameter ordering is immaterial");
        assert_eq!(report.to_ntriples(), reordered_report.to_ntriples());
    }
}

#[test]
fn false_optional_markers_and_absent_metadata_keep_target_parameters_mandatory() {
    for optional in [
        "",
        "sh:optional false ;",
        "sh:optional \"0\"^^xsd:boolean ;",
    ] {
        let shapes = target_shapes(optional, "ex:kind ex:Kind", false);
        let report = validate_graphs(DATA, &shapes, None).expect("missing mandatory is inactive");
        assert_eq!(focus_nodes(&report), Vec::<String>::new());
        let bound = target_shapes(optional, "ex:kind ex:Kind ; ex:label \"A\"", false);
        let report = validate_graphs(DATA, &bound, None).expect("all mandatory values supplied");
        assert_eq!(focus_nodes(&report), ["<http://example.org/alice>"]);
    }
}

#[test]
fn repeated_target_parameter_values_are_errors_even_on_inactive_instances() {
    for (values, predicate) in [
        ("ex:kind ex:Kind, ex:Other", "kind"),
        ("ex:kind ex:Kind ; ex:label \"A\", \"B\"", "label"),
        ("ex:label \"A\", \"B\"", "label"),
    ] {
        for reverse in [false, true] {
            let shapes = target_shapes("sh:optional true ;", values, reverse);
            let error = parse_shapes(&shapes, None).expect_err("target parameters remain max-one");
            assert!(error.contains("only one"), "{error}");
            assert!(
                error.contains(&format!("http://example.org/{predicate}")),
                "{error}"
            );
        }
    }
}

#[test]
fn target_optional_metadata_requires_one_boolean() {
    for optional in [
        "sh:optional true, false ;",
        "sh:optional \"true\" ;",
        "sh:optional 1 ;",
        "sh:optional ex:true ;",
    ] {
        let shapes = target_shapes(optional, "ex:kind ex:Kind", false);
        let error = parse_shapes(&shapes, None).expect_err("malformed optional metadata");
        assert!(error.contains("optional"), "{error}");
    }
}

#[test]
fn target_parameter_values_cannot_be_blank_nodes() {
    for (values, predicate) in [
        ("ex:kind _:kind", "kind"),
        ("ex:kind ex:Kind ; ex:label _:label", "label"),
        ("ex:label _:label", "label"),
    ] {
        let shapes = target_shapes("sh:optional true ;", values, false);
        let error = parse_shapes(&shapes, None).expect_err("blank target values are malformed");
        assert!(error.contains("blank node"), "{error}");
        assert!(
            error.contains(&format!("http://example.org/{predicate}")),
            "{error}"
        );
    }
}
