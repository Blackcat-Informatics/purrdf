// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A compiled schema larger than any fixed byte ceiling (a large ontology's,
//! as QUDT's 55 MB is) emits through every language emitter, deterministically
//! and in time linear in it, and reads back through every importer; a
//! pathological input is still bounded by its own size.

#[path = "support/turtle.rs"]
mod turtle;

use std::collections::BTreeMap;
use std::fmt::Write as _;

use purrdf_shapes::json_schema::{
    CompiledSchema, Namespaces, SchemaCompileRequest, SchemaSurfaceMode,
};
use purrdf_shapes::shapes::from_dataset;
use purrdf_shapes::{
    GraphqlConfig, LinkmlConfig, PydanticConfig, SchemaDatatypeMap, SchemaImportConfig,
    TypeScriptConfig, compile_schema, emit_graphql, emit_linkml, emit_pydantic, emit_typescript,
    import_compiled_schema, import_graphql_package, import_linkml_package, import_pydantic_package,
    import_typescript_package, parse_linkml,
};

const PREFIXES: &str = r"
    @prefix ex: <https://example.org/schema/> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
";
const EX: &str = "https://example.org/schema/";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

fn namespaces() -> Namespaces {
    Namespaces::new("ex", &[("ex".to_owned(), EX.to_owned())]).expect("namespaces")
}

/// `classes` classes, each with an existential on each of four shared object
/// properties over the next class: a schema whose size grows with the classes,
/// and whose definitions reference each other in one long chain (the shape on
/// which the Pydantic emitter's negation audit was exponential).
fn ontology(classes: usize) -> String {
    let mut ontology = String::new();
    for property in 0..4 {
        let _ = writeln!(ontology, "ex:link{property} a owl:ObjectProperty .");
    }
    for class in 0..classes {
        let _ = writeln!(ontology, "ex:Class{class} a owl:Class .");
        for property in 0..4 {
            let target = (class + 1) % classes;
            let _ = writeln!(
                ontology,
                "ex:Class{class} rdfs:subClassOf [ a owl:Restriction ;
                     owl:onProperty ex:link{property} ; owl:someValuesFrom ex:Class{target} ] ."
            );
        }
    }
    ontology
}

fn import_config() -> SchemaImportConfig {
    let datatypes = SchemaDatatypeMap::new(
        format!("{XSD}string"),
        format!("{XSD}boolean"),
        format!("{XSD}integer"),
        format!("{XSD}decimal"),
        format!("{XSD}dateTime"),
        format!("{XSD}date"),
        format!("{XSD}time"),
        format!("{XSD}anyURI"),
    )
    .expect("datatypes");
    SchemaImportConfig::new(namespaces(), datatypes)
}

#[test]
fn a_schema_past_sixteen_mebibytes_emits_in_every_language_deterministically() {
    let shapes = from_dataset(&turtle::data(PREFIXES, "")).expect("shapes");
    let ontology = turtle::data(PREFIXES, &ontology(1_100));
    let ns = namespaces();
    let compiled = compile_schema(&SchemaCompileRequest::new(
        &shapes,
        &ns,
        ontology.as_ref(),
        SchemaSurfaceMode::OntologyComplete,
    ))
    .expect("the ontology compiles")
    .compiled;
    assert!(
        compiled.schema_json.len() > 16 * 1024 * 1024,
        "the schema ({} bytes) is past the old fixed ceiling",
        compiled.schema_json.len()
    );
    let import = import_config();
    import_compiled_schema(&compiled, &import).expect("the JSON Schema reads back");

    let graphql = GraphqlConfig::new("Large", "x", "y", "RdfValue").expect("config");
    let first = emit_graphql(&compiled, &graphql).expect("GraphQL emits");
    let second = emit_graphql(&compiled, &graphql).expect("GraphQL emits again");
    assert_eq!(
        first.artifacts, second.artifacts,
        "GraphQL is deterministic"
    );
    import_graphql_package(&first, &import).expect("GraphQL reads back");

    let typescript = TypeScriptConfig::new("large-types", "x", "y").expect("config");
    let first = emit_typescript(&compiled, &typescript).expect("TypeScript emits");
    let second = emit_typescript(&compiled, &typescript).expect("TypeScript emits again");
    assert_eq!(
        first.artifacts, second.artifacts,
        "TypeScript is deterministic"
    );
    import_typescript_package(&first, &import).expect("TypeScript reads back");

    let pydantic = PydanticConfig::new("large_models", "x", "y").expect("config");
    let first = emit_pydantic(&compiled, &pydantic).expect("Pydantic emits");
    let second = emit_pydantic(&compiled, &pydantic).expect("Pydantic emits again");
    assert_eq!(
        first.artifacts, second.artifacts,
        "Pydantic is deterministic"
    );
    import_pydantic_package(&first, &import).expect("Pydantic reads back");

    let linkml = LinkmlConfig::new(
        "https://example.org/schema/generated",
        "Large",
        "x",
        "ex",
        BTreeMap::from([
            ("ex".to_owned(), EX.to_owned()),
            ("linkml".to_owned(), "https://w3id.org/linkml/".to_owned()),
        ]),
    )
    .expect("config");
    let first = emit_linkml(&compiled, &linkml).expect("LinkML emits");
    let second = emit_linkml(&compiled, &linkml).expect("LinkML emits again");
    assert_eq!(first.yaml, second.yaml, "LinkML is deterministic");
    parse_linkml(&first.yaml).expect("the large LinkML YAML parses");
    import_linkml_package(&first, &import).expect("LinkML reads back");
}

#[test]
fn a_yaml_alias_expansion_is_bounded_by_the_input() {
    // Each level doubles the nodes its aliases expand to: 2^24 from a few
    // hundred bytes, far past what the input's size allows.
    let mut yaml = String::from("a0: &a0 [x, x]\n");
    for level in 1..=24 {
        let _ = writeln!(
            yaml,
            "a{level}: &a{level} [*a{}, *a{}]",
            level - 1,
            level - 1
        );
    }
    let error = parse_linkml(&yaml).expect_err("an alias expansion past the bound is refused");
    assert!(error.to_string().contains("invalid LinkML YAML"), "{error}");
    // Neighbour: a few levels expand within the bound and parse as YAML (the
    // document is then refused only for lacking LinkML's envelope).
    let mut small = String::from("a0: &a0 [x, x]\n");
    for level in 1..=4 {
        let _ = writeln!(
            small,
            "a{level}: &a{level} [*a{}, *a{}]",
            level - 1,
            level - 1
        );
    }
    let error = parse_linkml(&small).expect_err("not a LinkML document");
    assert!(
        !error.to_string().contains("invalid LinkML YAML"),
        "{error}"
    );
}

/// A JSON Schema of `definitions` object definitions, the first with
/// `properties` properties, each property's schema nested `depth` levels deep
/// under `allOf`.
fn many_definitions(definitions: usize, properties: usize, depth: usize) -> String {
    let mut value = String::from(r#"{"type":"string"}"#);
    for _ in 0..depth {
        value = format!(r#"{{"allOf":[{value}]}}"#);
    }
    let mut schema = String::from(r#"{"$defs":{"#);
    for definition in 0..definitions {
        if definition > 0 {
            schema.push(',');
        }
        let _ = write!(
            schema,
            r#""Class{definition}":{{"type":"object","properties":{{"#
        );
        let count = if definition == 0 { properties } else { 1 };
        for property in 0..count {
            if property > 0 {
                schema.push(',');
            }
            let _ = write!(schema, r#""ex:p{property}":{value}"#);
        }
        schema.push_str("}}");
    }
    schema.push_str("}}");
    schema
}

#[test]
fn a_schema_past_65536_definitions_and_properties_imports() {
    // QUDT's LinkML document reads to a JSON Schema of 90,765 definitions;
    // neither definitions nor properties have a fixed ceiling, only the
    // input's own size.
    let imported =
        purrdf_shapes::import_json_schema(&many_definitions(70_000, 70_000, 0), &import_config())
            .expect("70,000 definitions and 70,000 properties import");
    assert_eq!(imported.shapes.node_shapes.len(), 70_000);
    let first = imported
        .shapes
        .node_shapes
        .iter()
        .find(|shape| shape.id.to_string() == format!("<{EX}Class0>"))
        .expect("Class0's shape");
    assert_eq!(first.property_shapes.len(), 70_000);
    // Neighbour: a pathological schema, nested past the shared depth ceiling,
    // is still refused, whatever its size.
    let error = purrdf_shapes::import_json_schema(
        &many_definitions(2, 1, purrdf_shapes::limits::MAX_SCHEMA_DEPTH),
        &import_config(),
    )
    .expect_err("a schema nested past the depth ceiling is refused");
    assert!(
        error.to_string().contains("nested arrays and objects"),
        "{error}"
    );
    purrdf_shapes::import_json_schema(&many_definitions(2, 1, 8), &import_config())
        .expect("a shallow neighbour imports");
}

#[test]
fn ontology_width_and_coverage_past_the_old_caps_emit_in_all_three_languages() {
    const CLASSES: usize = 65_537;
    const PROPERTIES: usize = 17;
    let mut source = String::new();
    for class in 0..CLASSES {
        writeln!(source, "ex:Wide{class:05} a owl:Class .").expect("fixture text");
    }
    // Each property belongs only to one class. This crosses the coverage-cell
    // ceiling without making every emitted class own every property.
    for property in 0..PROPERTIES {
        writeln!(source, "ex:wide{property} a owl:DatatypeProperty ; rdfs:domain ex:Wide{property:05} ; rdfs:range xsd:string .").expect("fixture text");
    }
    let shape_data = turtle::data(PREFIXES, "");
    let shapes = from_dataset(&shape_data).expect("empty shapes");
    let ontology = turtle::data(PREFIXES, &source);
    let ns = namespaces();
    let compilation = compile_schema(&SchemaCompileRequest::new(
        &shapes,
        &ns,
        ontology.as_ref(),
        SchemaSurfaceMode::OntologyComplete,
    ))
    .expect("65,537 actual ontology classes compile");
    let cells: usize = compilation
        .coverage
        .properties
        .iter()
        .map(|property| property.classes.len())
        .sum();
    assert_eq!(compilation.coverage.properties.len(), PROPERTIES);
    assert_eq!(cells, CLASSES * PROPERTIES);
    assert!(
        cells > 1_048_576,
        "the actual complete coverage exceeds the old cap"
    );
    let compiled = &compilation.compiled;
    let schema = purrdf_lex::json::read_with(
        &compiled.schema_json,
        purrdf_lex::json::Limits {
            max_depth: purrdf_shapes::limits::MAX_SCHEMA_DEPTH,
            max_values: u64::try_from(compiled.schema_json.len()).expect("source length fits"),
            max_string_bytes: compiled.schema_json.len(),
            unique_members: true,
        },
    )
    .expect("the actual compiled schema reads under its input-derived limits");
    let definitions = schema["$defs"].as_object().expect("definitions");
    assert_eq!(
        definitions
            .keys()
            .filter(|key| key.starts_with("Wide"))
            .count(),
        CLASSES
    );
    assert!(definitions.len() > 65_536);
    drop(schema);
    // Keep each emitted artifact's peak independent of the other emitters.
    let typescript = emit_typescript(
        compiled,
        &TypeScriptConfig::new("wide-types", "x", "y").expect("config"),
    )
    .expect("TypeScript emits every actual definition beyond 65,536");
    assert_eq!(
        typescript
            .type_names
            .keys()
            .filter(|key| key.starts_with("Wide"))
            .count(),
        CLASSES
    );
    assert!(typescript.type_names.contains_key("Wide65536"));
    drop(typescript);
    let graphql = emit_graphql(
        compiled,
        &GraphqlConfig::new("Wide", "x", "y", "RdfValue").expect("config"),
    )
    .expect("GraphQL emits every actual definition beyond 65,536");
    assert_eq!(
        graphql
            .names
            .definitions
            .keys()
            .filter(|key| key.starts_with("Wide"))
            .count(),
        CLASSES
    );
    assert!(graphql.names.definitions.contains_key("Wide65536"));
    drop(graphql);
    let pydantic = emit_pydantic(
        compiled,
        &PydanticConfig::new("wide_models", "x", "y").expect("config"),
    )
    .expect("Pydantic emits every actual definition beyond 65,536");
    assert_eq!(
        pydantic
            .model_paths
            .keys()
            .filter(|key| key.starts_with("Wide"))
            .count(),
        CLASSES
    );
    assert!(pydantic.model_paths.contains_key("Wide65536"));
}

#[test]
fn all_three_emitters_keep_the_shared_depth_refusal_and_accept_its_neighbor() {
    let compiled = |depth| CompiledSchema {
        schema_json: many_definitions(2, 1, depth),
        openapi_json: String::new(),
        losses: purrdf_rdf::loss::LossLedger::default(),
    };
    let deep = compiled(purrdf_shapes::limits::MAX_SCHEMA_DEPTH);
    let shallow = compiled(8);
    let typescript = TypeScriptConfig::new("depth-types", "x", "y").expect("config");
    let graphql = GraphqlConfig::new("Depth", "x", "y", "RdfValue").expect("config");
    let pydantic = PydanticConfig::new("depth_models", "x", "y").expect("config");
    for error in [
        emit_typescript(&deep, &typescript)
            .expect_err("TypeScript preserves depth refusal")
            .to_string(),
        emit_graphql(&deep, &graphql)
            .expect_err("GraphQL preserves depth refusal")
            .to_string(),
        emit_pydantic(&deep, &pydantic)
            .expect_err("Pydantic preserves depth refusal")
            .to_string(),
    ] {
        assert!(
            error.contains("depth") || error.contains("nesting"),
            "{error}"
        );
    }
    emit_typescript(&shallow, &typescript).expect("shallow TypeScript neighbor");
    emit_graphql(&shallow, &graphql).expect("shallow GraphQL neighbor");
    emit_pydantic(&shallow, &pydantic).expect("shallow Pydantic neighbor");
}
