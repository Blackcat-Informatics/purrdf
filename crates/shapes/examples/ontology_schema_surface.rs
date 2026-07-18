// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Compare shaped-only and ontology-complete schema surfaces and emit every carrier.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use purrdf_shapes::{
    GraphqlConfig, LinkmlConfig, Namespaces, PydanticConfig, SchemaCompileRequest,
    SchemaSurfaceMode, TypeScriptConfig, compile_schema, emit_graphql, emit_linkml, emit_pydantic,
    emit_typescript,
};

const EX: &str = "https://example.org/schema/";
const LINKML: &str = "https://w3id.org/linkml/";
const PREFIXES: &str = r"
@prefix ex: <https://example.org/schema/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
";

fn artifact_contains(artifacts: &BTreeMap<String, Vec<u8>>, needle: &[u8]) -> bool {
    artifacts.values().any(|artifact| {
        artifact
            .windows(needle.len())
            .any(|window| window == needle)
    })
}

fn definition_keys(schema: &str) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let document: serde_json::Value = serde_json::from_str(schema)?;
    let definitions = document["$defs"]
        .as_object()
        .ok_or("compiled schema has no $defs object")?;
    Ok(definitions.keys().cloned().collect())
}

fn main() -> Result<(), Box<dyn Error>> {
    let shapes_dataset = purrdf_shapes::text_ingest::parse_turtle_to_dataset(&format!(
        "{PREFIXES}
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ; sh:datatype xsd:string ] ."
    ))
    .map_err(|errors| std::io::Error::other(errors.join("\n")))?;
    let shapes = purrdf_shapes::shapes::from_dataset(&shapes_dataset)?;
    let ontology = purrdf_shapes::text_ingest::parse_turtle_to_dataset(&format!(
        "{PREFIXES}
ex:Person a owl:Class .
ex:EmailMessage a owl:Class .
ex:name a owl:DatatypeProperty ; rdfs:domain ex:Person ; rdfs:range xsd:string .
ex:latestMessage a owl:ObjectProperty, owl:FunctionalProperty ;
    rdfs:domain ex:Person ; rdfs:range ex:EmailMessage .
ex:resentDate a owl:DatatypeProperty ;
    rdfs:domain ex:EmailMessage ; rdfs:range xsd:dateTime .
ex:resentMessageId a owl:DatatypeProperty ;
    rdfs:domain ex:EmailMessage ; rdfs:range rdfs:Literal ."
    ))
    .map_err(|errors| std::io::Error::other(errors.join("\n")))?;
    let namespaces = Namespaces::new("ex", &[("ex".to_owned(), EX.to_owned())])?;

    let shaped = compile_schema(&SchemaCompileRequest::new(
        &shapes,
        &namespaces,
        ontology.as_ref(),
        SchemaSurfaceMode::ShapedOnly,
    ))?;
    let complete = compile_schema(&SchemaCompileRequest::new(
        &shapes,
        &namespaces,
        ontology.as_ref(),
        SchemaSurfaceMode::OntologyComplete,
    ))?;
    let shaped_defs = definition_keys(&shaped.compiled.schema_json)?;
    let complete_defs = definition_keys(&complete.compiled.schema_json)?;
    println!("shaped-only definitions: {shaped_defs:?}");
    println!("ontology-complete definitions: {complete_defs:?}");
    println!("ontology-complete cache key: {}", complete.key);
    println!("coverage manifest:\n{}", complete.coverage.to_json());
    if shaped_defs.contains("EmailMessage") || !complete_defs.contains("EmailMessage") {
        return Err("surface-mode comparison did not expose the ontology-only class".into());
    }

    let linkml = emit_linkml(
        &complete.compiled,
        &LinkmlConfig::new(
            "https://example.org/schema/generated",
            "ExampleSchema",
            "Ontology-complete schema owned by the example.org caller.",
            "ex",
            BTreeMap::from([
                ("ex".to_owned(), EX.to_owned()),
                ("linkml".to_owned(), LINKML.to_owned()),
            ]),
        )?,
    )?;
    let typescript = emit_typescript(
        &complete.compiled,
        &TypeScriptConfig::new(
            "example-ontology-types",
            "Types owned by the example.org caller.",
            "Ontology-complete TypeScript declarations.",
        )?,
    )?;
    let graphql = emit_graphql(
        &complete.compiled,
        &GraphqlConfig::new(
            "ExampleOntology",
            "GraphQL package owned by the example.org caller.",
            "Ontology-complete GraphQL types.",
            "RdfValue",
        )?,
    )?;
    let pydantic = emit_pydantic(
        &complete.compiled,
        &PydanticConfig::new(
            "example_ontology",
            "Models owned by the example.org caller.",
            "Ontology-complete Pydantic models.",
        )?,
    )?;

    let property = b"ex:resentMessageId";
    let reaches_every_carrier = linkml
        .yaml
        .as_bytes()
        .windows(property.len())
        .any(|w| w == property)
        && artifact_contains(&typescript.artifacts, property)
        && graphql
            .names
            .fields
            .values()
            .any(|fields| fields.contains_key("ex:resentMessageId"))
        && artifact_contains(&pydantic.artifacts, property);
    if !reaches_every_carrier {
        return Err("ontology-only property did not reach every language carrier".into());
    }
    println!(
        "emitted LinkML YAML plus {} TypeScript, {} GraphQL, and {} Pydantic artifact(s)",
        typescript.artifacts.len(),
        graphql.artifacts.len(),
        pydantic.artifacts.len()
    );
    Ok(())
}
