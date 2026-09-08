// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Property constraints retain the identity of their declaring RDF shape.

use std::sync::Arc;

use purrdf_shapes::data::{GraphFilter, native_quads};
use purrdf_shapes::engine::{
    PreparedValidator, parse_shapes, validate_dataset, validate_graphs_with_config,
};
use purrdf_shapes::json_schema::Namespaces;
use purrdf_shapes::model::{BoxRoleVocab, sh};
use purrdf_shapes::report::Severity;
use purrdf_shapes::schema_import::{SchemaDatatypeMap, SchemaImportConfig, import_json_schema};
use purrdf_shapes::shapes::from_dataset;
use purrdf_shapes::term::{NamedNode, Term};

const PREFIXES: &str = r"
    @prefix ex: <http://example.org/> .
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
    @prefix meta: <http://example.org/meta/> .
";

fn ex(local: &str) -> Term {
    Term::NamedNode(NamedNode::new_unchecked(format!(
        "http://example.org/{local}"
    )))
}

fn dataset(body: &str) -> Arc<::purrdf::RdfDataset> {
    ::purrdf::parse_dataset(format!("{PREFIXES}{body}").as_bytes(), "text/turtle", None)
        .expect("valid Turtle")
}

/// The source of a property constraint is the property shape for both named
/// and anonymous declarations, including an anonymous node's original identity.
#[test]
fn property_sources_match_the_declaring_graph_nodes() {
    let data = dataset("");
    for property in ["ex:Property", "_:property"] {
        let source = dataset(&format!(
            "ex:Shape a sh:NodeShape; sh:targetNode ex:n; sh:property {property} .
             {property} sh:path ex:p; sh:minCount 1 ."
        ));
        let predicate = Term::NamedNode(NamedNode::from(sh::PROPERTY));
        let property_id = native_quads(
            &source,
            Some(&ex("Shape")),
            Some(&predicate),
            None,
            GraphFilter::AnyGraph,
        )
        .pop()
        .expect("declared property shape")
        .2;
        let shapes = from_dataset(&source).expect("valid shapes");
        assert_eq!(shapes.node_shapes[0].property_shapes[0].id, property_id);
        let report = validate_dataset(&data, &shapes).expect("validation");
        assert_eq!(report.results.len(), 1);
        assert_eq!(report.results[0].source_shape, property_id);
        assert_ne!(report.results[0].source_shape, ex("Shape"));
    }
}

/// Source identity changes at each nested property, while box-role provenance
/// continues to include the ancestor shapes and the actual property path.
#[test]
fn nested_property_source_preserves_ancestor_box_roles() {
    let shapes = format!(
        "{PREFIXES}
         ex:Shape a sh:NodeShape; sh:targetNode ex:n; sh:property ex:Outer;
             meta:graphBoxRole meta:boxTBox .
         ex:Outer sh:path ex:outer; sh:property ex:Inner;
             meta:graphBoxRole meta:boxConfigBox .
         ex:Inner sh:path ex:required; sh:minCount 1;
             meta:graphBoxRole meta:boxCBox ."
    );
    let report = validate_graphs_with_config(
        "<http://example.org/n> <http://example.org/outer> <http://example.org/child> .\n\
         <http://example.org/required> <http://example.org/meta/graphBoxRole> <http://example.org/meta/boxRBox> .",
        &shapes,
        None,
        Some(BoxRoleVocab::for_namespace("http://example.org/meta/")),
    )
    .expect("nested validation");
    assert_eq!(report.results.len(), 1);
    let result = &report.results[0];
    assert_eq!(result.source_shape, ex("Inner"));
    assert_eq!(result.focus_node, ex("child"));
    assert_eq!(result.result_path, Some(ex("required")));
    for role in ["boxTBox", "boxConfigBox", "boxCBox"] {
        assert!(
            result
                .source_box_roles
                .iter()
                .any(|value| { value.as_str() == format!("http://example.org/meta/{role}") })
        );
    }
    assert!(
        result
            .path_box_roles
            .iter()
            .any(|value| { value.as_str() == "http://example.org/meta/boxRBox" })
    );
}

/// Direct and prepared validation pre-bind `currentShape` to the property
/// declaration, including bounded validation by native terms and interned IDs.
#[test]
fn sparql_current_shape_is_the_property_in_every_validation_entry() {
    let shapes = Arc::new(
        parse_shapes(
            &format!(
                "{PREFIXES}
                 ex:Shape a sh:NodeShape; sh:targetNode ex:n; sh:property ex:Property .
                 ex:Property sh:path ex:p; sh:sparql [
                     sh:select \"SELECT $this WHERE {{ FILTER ($currentShape = <http://example.org/Property>) }}\"
                 ] ."
            ),
            None,
        )
        .expect("SPARQL property shape"),
    );
    let data = dataset("ex:n ex:p ex:value .");
    let direct = validate_dataset(&data, &shapes).expect("direct validation");
    assert_eq!(
        direct.results.len(),
        1,
        "currentShape must bind the property"
    );
    assert_eq!(direct.results[0].source_shape, ex("Property"));
    let prepared =
        PreparedValidator::from_dataset(&data, Arc::clone(&shapes)).expect("prepared validator");
    let focus_id = data
        .term_id_by_iri("http://example.org/n")
        .expect("focus ID");
    for report in [
        prepared.validate().expect("all targets"),
        prepared
            .validate_focus_nodes(&[ex("n")])
            .expect("native focus"),
        prepared
            .validate_focus_node_ids(&[focus_id])
            .expect("interned focus"),
    ] {
        assert_eq!(report.to_ntriples(), direct.to_ntriples());
    }
}

/// Both missing reification and a failing reifier shape are constraints declared
/// by the property shape; their reported source is that declaration.
#[test]
fn reifier_constraint_sources_use_the_property_declaration() {
    let shapes = parse_shapes(
        &format!(
            "{PREFIXES}
             ex:Shape a sh:NodeShape; sh:targetNode ex:n; sh:property ex:Property .
             ex:Property sh:path ex:p; sh:reificationRequired true; sh:severity sh:Warning;
                 sh:reifierShape ex:ReifierShape .
             ex:ReifierShape sh:class ex:Record; sh:severity sh:Info ."
        ),
        None,
    )
    .expect("reifier shapes");
    for data in [
        dataset("ex:n ex:p ex:value ."),
        dataset("ex:n ex:p ex:value . ex:statement rdf:reifies <<(ex:n ex:p ex:value)>> ."),
    ] {
        let report = validate_dataset(&data, &shapes).expect("reifier validation");
        assert_eq!(report.results.len(), 1);
        assert_eq!(report.results[0].source_shape, ex("Property"));
        assert_eq!(report.results[0].severity, Severity::Warning);
        assert_eq!(
            report.results[0].source_constraint_component.as_str(),
            sh::REIFIER_SHAPE_CONSTRAINT_COMPONENT
        );
    }
}

/// Imported schemas give each property a stable blank-node identity derived
/// from its source shape and JSON Pointer, including required-only properties.
#[test]
fn imported_properties_have_distinct_deterministic_source_identities() {
    const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
    let config = SchemaImportConfig::new(
        Namespaces::new("ex", &[("ex".to_owned(), "http://example.org/".to_owned())])
            .expect("namespace configuration"),
        SchemaDatatypeMap::new(
            format!("{XSD}string"),
            format!("{XSD}boolean"),
            format!("{XSD}integer"),
            format!("{XSD}decimal"),
            format!("{XSD}dateTime"),
            format!("{XSD}date"),
            format!("{XSD}time"),
            format!("{XSD}anyURI"),
        )
        .expect("datatype configuration"),
    );
    let document = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$defs": {
            "Record": {
                "type": "object",
                "properties": {"ex:name": {"type": "string"}},
                "required": ["ex:name", "ex:missing"]
            }
        }
    })
    .to_string();
    let imported = import_json_schema(&document, &config).expect("imported schema");
    let repeated = import_json_schema(&document, &config).expect("repeated import");
    let properties = &imported.shapes.node_shapes[0].property_shapes;
    assert_eq!(properties.len(), 2);
    assert_ne!(properties[0].id, properties[1].id);
    for (property, repeated) in properties
        .iter()
        .zip(&repeated.shapes.node_shapes[0].property_shapes)
    {
        assert!(matches!(property.id, Term::BlankNode(_)));
        assert_eq!(property.id, repeated.id);
    }
    let report = validate_dataset(&dataset("ex:n a ex:Record ."), &imported.shapes)
        .expect("imported schema validation");
    assert_eq!(report.results.len(), 2);
    for result in report.results {
        assert!(
            properties
                .iter()
                .any(|property| property.id == result.source_shape)
        );
    }
}
