// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL-AF targets identify shapes without an explicit shape type.

use purrdf_shapes::engine::{parse_shapes, validate_graphs};
use purrdf_shapes::model::sh;
use purrdf_shapes::report::Severity;
use purrdf_shapes::term::{NamedNode, Term};

const PREFIXES: &str = r"
    @prefix ex: <http://example.org/> .
    @prefix sh: <http://www.w3.org/ns/shacl#> .
";

const INVALID_DATA: &str = concat!(
    "<http://example.org/alice> <http://example.org/kind> <http://example.org/Kind> .\n",
    "<http://example.org/bob> <http://example.org/kind> <http://example.org/Other> .\n",
);

const CONFORMING_FACTS: &str = concat!(
    "<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ",
    "<http://example.org/Allowed> .\n",
    "<http://example.org/alice> <http://example.org/value> <http://example.org/Resource> .\n",
);

const PARAMETERIZED_TARGET: &str = r#"
    ex:ByKind a sh:SPARQLTargetType ;
        sh:parameter [ sh:path ex:kind ] ;
        sh:select "SELECT ?this WHERE { ?this <http://example.org/kind> $kind . }" .
    ex:Target a ex:ByKind ; ex:kind ex:Kind .
"#;

fn ex(local: &str) -> Term {
    Term::NamedNode(NamedNode::new_unchecked(format!(
        "http://example.org/{local}"
    )))
}

fn assert_target_discovery(target: &str) {
    for (constraints, shape_type, component, path, value) in [
        (
            "sh:class ex:Allowed",
            "sh:NodeShape",
            sh::CLASS_CONSTRAINT_COMPONENT,
            None,
            Some(ex("alice")),
        ),
        (
            "sh:path ex:value ; sh:minCount 1",
            "sh:PropertyShape",
            sh::MIN_COUNT_CONSTRAINT_COMPONENT,
            Some(ex("value")),
            None,
        ),
    ] {
        let control = format!("{PREFIXES} ex:Shape sh:targetNode ex:alice ; {constraints} .");
        let expected = validate_graphs(INVALID_DATA, &control, None).expect("Core target control");
        assert!(!expected.conforms);
        assert_eq!(expected.results.len(), 1);
        let result = &expected.results[0];
        assert_eq!(result.focus_node, ex("alice"));
        assert_eq!(result.source_shape, ex("Shape"));
        assert_eq!(result.source_constraint_component.as_str(), component);
        assert_eq!(result.result_path, path);
        assert_eq!(result.value, value);
        assert_eq!(result.severity, Severity::Violation);

        let untyped = format!("{PREFIXES} {target} ex:Shape sh:target ex:Target ; {constraints} .");
        let typed = format!("{untyped} ex:Shape a {shape_type} .");
        for document in [&untyped, &typed] {
            let parsed = parse_shapes(document, None).expect("valid SHACL-AF shape");
            assert_eq!(
                parsed.node_shapes.len(),
                1,
                "sh:target must discover its subject, including {shape_type}"
            );
            assert_eq!(parsed.node_shapes[0].id, ex("Shape"));
            let report =
                validate_graphs(INVALID_DATA, document, None).expect("SHACL-AF target validation");
            assert_eq!(
                report.to_ntriples(),
                expected.to_ntriples(),
                "custom and Core targets must produce the same exact violation"
            );

            let valid_data = format!("{INVALID_DATA}{CONFORMING_FACTS}");
            let valid = validate_graphs(&valid_data, document, None)
                .expect("conforming SHACL-AF target validation");
            assert!(valid.conforms);
            assert!(valid.results.is_empty());
            assert_eq!(
                valid.to_ntriples(),
                validate_graphs(&valid_data, &control, None)
                    .expect("conforming Core target control")
                    .to_ntriples(),
            );
        }
    }
}

#[test]
fn plain_targets_discover_untyped_node_and_standalone_property_shapes() {
    assert_target_discovery(
        r#"
        ex:Target a sh:SPARQLTarget ;
            sh:select """SELECT ?this WHERE {
                ?this <http://example.org/kind> <http://example.org/Kind> .
            }""" .
        "#,
    );
}

#[test]
fn parameterized_targets_discover_untyped_node_and_standalone_property_shapes() {
    assert_target_discovery(PARAMETERIZED_TARGET);
}

#[test]
fn missing_mandatory_targets_preserve_discovered_shapes_without_focus_nodes() {
    let target = PARAMETERIZED_TARGET.replace(" ; ex:kind ex:Kind", "");
    for constraints in ["sh:class ex:Allowed", "sh:path ex:value ; sh:minCount 1"] {
        let document =
            format!("{PREFIXES} {target} ex:Shape sh:target ex:Target ; {constraints} .");
        let parsed = parse_shapes(&document, None).expect("inactive custom target");
        assert_eq!(parsed.node_shapes.len(), 1);
        assert_eq!(parsed.node_shapes[0].id, ex("Shape"));
        assert!(parsed.node_shapes[0].targets.is_empty());
        let report =
            validate_graphs(INVALID_DATA, &document, None).expect("inactive target validation");
        assert!(report.conforms);
        assert!(report.results.is_empty());
    }
}
