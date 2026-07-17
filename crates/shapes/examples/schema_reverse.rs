// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Exercise every schema-language → SHACL production entry point.

use std::collections::BTreeMap;
use std::error::Error;

use purrdf_shapes::{
    GraphqlConfig, LinkmlConfig, Namespaces, PydanticConfig, SchemaDatatypeMap, SchemaImportConfig,
    TypeScriptConfig, emit_graphql, emit_linkml, emit_pydantic, emit_typescript,
    import_graphql_package, import_json_schema, import_linkml, import_linkml_package,
    import_pydantic_package, import_typescript_package, parse_linkml, write_linkml,
};

const EX: &str = "https://example.org/";
const LINKML: &str = "https://w3id.org/linkml/";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const SOURCE_SHAPES: &str = r#"
@prefix ex:  <https://example.org/> .
@prefix sh:  <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:closed true ;
    sh:property [
        sh:path ex:name ;
        sh:datatype xsd:string ;
        sh:minCount 1 ;
        sh:maxCount 1
    ] ;
    sh:property [
        sh:path ex:status ;
        sh:maxCount 1 ;
        sh:in ( "active" "paused" )
    ] .
"#;

fn import_config() -> Result<SchemaImportConfig, Box<dyn Error>> {
    let namespaces = Namespaces::new("ex", &[("ex".to_owned(), EX.to_owned())])?;
    let datatypes = SchemaDatatypeMap::new(
        format!("{XSD}string"),
        format!("{XSD}boolean"),
        format!("{XSD}integer"),
        format!("{XSD}decimal"),
        format!("{XSD}dateTime"),
        format!("{XSD}date"),
        format!("{XSD}time"),
        format!("{XSD}anyURI"),
    )?;
    Ok(SchemaImportConfig::new(namespaces, datatypes))
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = import_config()?;
    let dataset = purrdf_shapes::text_ingest::parse_turtle_to_dataset(SOURCE_SHAPES)
        .map_err(|errors| std::io::Error::other(errors.join("\n")))?;
    let source_shapes = purrdf_shapes::shapes::from_dataset(&dataset)?;
    let compiled = purrdf_shapes::json_schema::compile(&source_shapes, config.namespaces());
    if !compiled.losses.is_empty() {
        return Err("example SHACL source must compile without forward losses".into());
    }

    let json_schema = import_json_schema(&compiled.schema_json, &config)?;

    let linkml_config = LinkmlConfig::new(
        "https://example.org/schema/linkml",
        "ExampleSchema",
        "Schema package owned by the example.org caller.",
        "ex",
        BTreeMap::from([
            ("ex".to_owned(), EX.to_owned()),
            ("linkml".to_owned(), LINKML.to_owned()),
        ]),
    )?;
    let linkml_package = emit_linkml(&compiled, &linkml_config)?;
    let linkml_document = parse_linkml(&linkml_package.yaml)?;
    if write_linkml(&linkml_document)? != linkml_package.yaml {
        return Err("native LinkML read/write path drifted".into());
    }
    let native_linkml = import_linkml(&linkml_document, &config)?;
    purrdf::loss::check_ledger_sound(&native_linkml.losses, "linkml-1.11", "shacl")?;
    println!(
        "linkml-1.11/native: canonical read/write; {} located reverse loss(es)",
        native_linkml.losses.entries().len()
    );
    let verified_linkml = import_linkml_package(&linkml_package, &config)?;

    let pydantic_package = emit_pydantic(
        &compiled,
        &PydanticConfig::new(
            "example_models",
            "Models owned by the example.org caller.",
            "Validation models generated from the caller's shapes.",
        )?,
    )?;
    let pydantic = import_pydantic_package(&pydantic_package, &config)?;

    let typescript_package = emit_typescript(
        &compiled,
        &TypeScriptConfig::new(
            "@example/schema-types",
            "Types owned by the example.org caller.",
            "Declarations generated from the caller's shapes.",
        )?,
    )?;
    let typescript = import_typescript_package(&typescript_package, &config)?;

    let graphql_package = emit_graphql(
        &compiled,
        &GraphqlConfig::new(
            "ExampleSchema",
            "GraphQL package owned by the example.org caller.",
            "Types generated from the caller's shapes.",
            "JsonCarrier",
        )?,
    )?;
    let graphql = import_graphql_package(&graphql_package, &config)?;

    for (label, profile, imported) in [
        ("json-schema", "json-schema", json_schema),
        ("linkml-1.11/package", "linkml-1.11", verified_linkml),
        ("pydantic-v2", "pydantic-v2", pydantic),
        ("typescript-7.0", "typescript-7.0", typescript),
        ("graphql-september-2025", "graphql-september-2025", graphql),
    ] {
        purrdf::loss::check_ledger_sound(&imported.losses, profile, "shacl")?;
        let round_trip = purrdf_shapes::json_schema::compile(&imported.shapes, config.namespaces());
        if round_trip.schema_json != compiled.schema_json {
            return Err(format!("{label} reverse path did not round-trip byte-exactly").into());
        }
        println!(
            "{label}: {} SHACL node shape(s), byte-exact; {} located reverse loss(es)",
            imported.shapes.node_shapes.len(),
            imported.losses.entries().len()
        );
    }
    Ok(())
}
