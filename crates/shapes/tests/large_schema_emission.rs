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

use purrdf_shapes::json_schema::{Namespaces, SchemaCompileRequest, SchemaSurfaceMode};
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
