// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cross-emitter proofs for ontology-complete developer-schema surfaces.

use std::collections::BTreeMap;

use purrdf_shapes::json_schema::{
    Namespaces, SchemaCompileRequest, SchemaSurfaceMode, compile_schema,
};
use purrdf_shapes::shapes::from_dataset;
use purrdf_shapes::{
    GRAPHQL_SCHEMA_PATH, GraphqlConfig, LinkmlConfig, PydanticConfig, TYPESCRIPT_DECLARATION_PATH,
    TypeScriptConfig, emit_graphql, emit_linkml, emit_pydantic, emit_typescript,
};

const PREFIXES: &str = r"
    @prefix ex: <https://example.org/schema/> .
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
";

fn compiled_surface() -> purrdf_shapes::SchemaCompilation {
    let shapes_dataset = purrdf_shapes::text_ingest::parse_turtle_to_dataset(&format!(
        "{PREFIXES}
        ex:PersonShape a sh:NodeShape ; sh:targetClass ex:Person ;
            sh:property [ sh:path ex:name ; sh:minCount 1 ; sh:datatype xsd:string ] ."
    ))
    .expect("shapes Turtle");
    let shapes = from_dataset(&shapes_dataset).expect("shapes graph");
    let ontology = purrdf_shapes::text_ingest::parse_turtle_to_dataset(&format!(
        "{PREFIXES}
        ex:Person a owl:Class .
        ex:EmailMessage a owl:Class .
        ex:name a owl:DatatypeProperty ; rdfs:domain ex:Person ; rdfs:range rdfs:Literal .
        ex:resentDate a owl:DatatypeProperty ;
            rdfs:domain ex:EmailMessage ; rdfs:range xsd:dateTime .
        ex:resentMessageId a owl:DatatypeProperty ;
            rdfs:domain ex:EmailMessage ; rdfs:range rdfs:Literal .
        ex:latestMessage a owl:ObjectProperty ;
            rdfs:domain ex:Person ; rdfs:range ex:EmailMessage ."
    ))
    .expect("ontology Turtle");
    let namespaces = Namespaces::new(
        "ex",
        &[("ex".to_owned(), "https://example.org/schema/".to_owned())],
    )
    .expect("namespace config");
    compile_schema(&SchemaCompileRequest::new(
        &shapes,
        &namespaces,
        ontology.as_ref(),
        SchemaSurfaceMode::OntologyComplete,
    ))
    .expect("ontology-complete compilation")
}

fn linkml_config() -> LinkmlConfig {
    LinkmlConfig::new(
        "https://example.org/schema/generated",
        "ExampleSchema",
        "Example ontology-complete schema.",
        "ex",
        BTreeMap::from([
            ("ex".to_owned(), "https://example.org/schema/".to_owned()),
            ("linkml".to_owned(), "https://w3id.org/linkml/".to_owned()),
        ]),
    )
    .expect("LinkML config")
}

fn typescript_config() -> TypeScriptConfig {
    TypeScriptConfig::new(
        "example-ontology-types",
        "Example ontology package.",
        "Example ontology declarations.",
    )
    .expect("TypeScript config")
}

fn graphql_config() -> GraphqlConfig {
    GraphqlConfig::new(
        "ExampleOntology",
        "Example GraphQL package.",
        "Example GraphQL module.",
        "RdfValue",
    )
    .expect("GraphQL config")
}

fn pydantic_config() -> PydanticConfig {
    PydanticConfig::new(
        "example_ontology",
        "Example Pydantic package.",
        "Example Pydantic models.",
    )
    .expect("Pydantic config")
}

#[test]
fn ontology_property_surface_reaches_every_language_emitter() {
    let compilation = compiled_surface();
    let compiled = &compilation.compiled;

    let linkml = emit_linkml(compiled, &linkml_config()).expect("LinkML emission");
    let linkml_classes = &linkml.document.as_value()["classes"];
    assert!(linkml_classes["EmailMessage"].is_object());
    assert!(
        linkml_classes["EmailMessage"]["attributes"]["ex:resentDate"].is_object(),
        "LinkML must retain the ontology-only resentDate attribute"
    );
    assert!(
        linkml_classes["EmailMessage"]["attributes"]["ex:resentMessageId"].is_object(),
        "LinkML must retain the ontology-only resentMessageId attribute"
    );

    let typescript = emit_typescript(compiled, &typescript_config()).expect("TypeScript emission");
    let declarations = std::str::from_utf8(&typescript.artifacts[TYPESCRIPT_DECLARATION_PATH])
        .expect("UTF-8 TypeScript");
    assert!(declarations.contains("ex:resentDate"));
    assert!(declarations.contains("ex:resentMessageId"));

    let graphql = emit_graphql(compiled, &graphql_config()).expect("GraphQL emission");
    assert!(graphql.artifacts[GRAPHQL_SCHEMA_PATH].is_ascii());
    assert!(graphql.names.fields.values().any(|fields| {
        fields.contains_key("ex:resentDate") && fields.contains_key("ex:resentMessageId")
    }));

    let pydantic = emit_pydantic(compiled, &pydantic_config()).expect("Pydantic emission");
    let python = pydantic
        .artifacts
        .values()
        .filter_map(|artifact| std::str::from_utf8(artifact).ok())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(python.contains("ex:resentDate"));
    assert!(python.contains("ex:resentMessageId"));

    assert_eq!(
        compilation.coverage.properties.len(),
        compilation
            .coverage
            .properties
            .iter()
            .map(|property| property.property_iri.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        "coverage catalog has one aggregate row per property"
    );
}

#[test]
fn all_language_emitter_outputs_are_byte_deterministic() {
    let first = compiled_surface();
    let second = compiled_surface();
    assert_eq!(first.compiled.schema_json, second.compiled.schema_json);
    assert_eq!(first.compiled.openapi_json, second.compiled.openapi_json);
    assert_eq!(first.coverage.to_json(), second.coverage.to_json());
    assert_eq!(first.key, second.key);
    assert_eq!(
        emit_linkml(&first.compiled, &linkml_config()).expect("first LinkML"),
        emit_linkml(&second.compiled, &linkml_config()).expect("second LinkML")
    );
    assert_eq!(
        emit_typescript(&first.compiled, &typescript_config()).expect("first TypeScript"),
        emit_typescript(&second.compiled, &typescript_config()).expect("second TypeScript")
    );
    assert_eq!(
        emit_graphql(&first.compiled, &graphql_config()).expect("first GraphQL"),
        emit_graphql(&second.compiled, &graphql_config()).expect("second GraphQL")
    );
    assert_eq!(
        emit_pydantic(&first.compiled, &pydantic_config()).expect("first Pydantic"),
        emit_pydantic(&second.compiled, &pydantic_config()).expect("second Pydantic")
    );
}
