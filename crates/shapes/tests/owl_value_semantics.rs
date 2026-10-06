// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! OWL ranges, fillers and enumerations judged by value, not by tag; boolean
//! constructs and cardinalities read by their meaning rather than refused;
//! and the punned properties, hash namespaces and colliding names real
//! ontologies use. Every rejected case runs beside a valid neighbour that is
//! accepted.
//!
//! The excerpts below are the triples of published W3C and GoodRelations
//! vocabularies that exercise each case, with the vocabulary's own IRIs.

use purrdf_lex::json::Value;
use purrdf_shapes::json_schema::{
    Namespaces, SchemaCompileRequest, SchemaSurfaceMode, compile_schema,
};
use purrdf_shapes::shapes::from_dataset;
use purrdf_shapes::{
    GraphqlConfig, SchemaClassExpressionReport, SchemaExpressionOutcome,
    compile_schema_with_class_expressions, emit_graphql,
};

const PREFIXES: &str = r"
    @prefix ex: <https://example.org/schema/> .
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
    @prefix time: <http://www.w3.org/2006/time#> .
    @prefix prov: <http://www.w3.org/ns/prov#> .
    @prefix gr: <http://purl.org/goodrelations/v1#> .
";
const EX: &str = "https://example.org/schema/";

fn namespaces(prefix: &str, iri: &str) -> Namespaces {
    Namespaces::new(prefix, &[(prefix.to_owned(), iri.to_owned())]).expect("namespace config")
}

fn example() -> Namespaces {
    namespaces("ex", EX)
}

fn parse(turtle: &str) -> std::sync::Arc<purrdf_rdf::RdfDataset> {
    purrdf_shapes::text_ingest::parse_turtle_to_dataset(&format!("{PREFIXES}{turtle}"), None)
        .expect("Turtle")
}

/// The schema and the class-expression manifest of an ontology with no shapes.
fn compile(
    ontology: &str,
    namespaces: &Namespaces,
) -> (
    purrdf_shapes::SchemaCompilation,
    SchemaClassExpressionReport,
) {
    let shapes = from_dataset(&parse("")).expect("shapes graph");
    let ontology = parse(ontology);
    compile_schema_with_class_expressions(&SchemaCompileRequest::new(
        &shapes,
        namespaces,
        ontology.as_ref(),
        SchemaSurfaceMode::OntologyComplete,
    ))
    .expect("the ontology compiles")
}

/// Whether the projected node `subject` of `data` validates against
/// `#/$defs/{class}`.
fn accepts(
    schema_json: &str,
    namespaces: &Namespaces,
    class: &str,
    data: &str,
    subject: &str,
) -> bool {
    let data = parse(data);
    let projected = purrdf_shapes::instance::project_graph(&data, namespaces);
    let node = projected["@graph"]
        .as_array()
        .expect("@graph")
        .iter()
        .find(|node| node["@id"] == subject)
        .unwrap_or_else(|| panic!("{subject} is projected"))
        .clone();
    let metaschemas = purrdf_jsonschema::Metaschemas::new(
        purrdf_testkit::jsonschema_metaschemas::DRAFT_2020_12
            .iter()
            .map(|&(uri, text)| {
                let document: Value = purrdf_lex::json::read(text).expect("meta-schema JSON");
                (uri, document)
            }),
    )
    .expect("the draft 2020-12 meta-schemas");
    let schema: Value = purrdf_lex::json::read(schema_json).expect("schema JSON");
    let location = "mem:///ontology.schema.json";
    let mut registry = purrdf_jsonschema::Registry::with_metaschemas(&metaschemas);
    registry
        .add_resource(location, schema)
        .expect("schema registers");
    registry
        .compile(&format!("{location}#/$defs/{class}"))
        .expect("schema compiles under draft 2020-12")
        .is_valid(&node)
        .expect("schema evaluation completes")
}

/// The outcomes the manifest reports on `class` for the restrictions on
/// `property`.
fn outcomes(
    report: &SchemaClassExpressionReport,
    class: &str,
    property: &str,
) -> Vec<SchemaExpressionOutcome> {
    report
        .axioms
        .iter()
        .flat_map(|axiom| &axiom.classes)
        .filter(|row| row.class_iri == format!("{EX}{class}"))
        .flat_map(|row| &row.components)
        .filter(|component| {
            component.property_iri.as_deref() == Some(format!("{EX}{property}").as_str())
        })
        .map(|component| component.outcome)
        .collect()
}

/// Assert which of `cases` (data, whether accepted) the schema of `class`
/// accepts for the subject `ex:x`.
fn judge(schema: &str, class: &str, cases: &[(&str, bool)]) {
    let ns = example();
    for (data, expected) in cases {
        assert_eq!(
            accepts(
                schema,
                &ns,
                class,
                &format!("ex:x a ex:{class} ; {data} ."),
                &format!("{EX}x")
            ),
            *expected,
            "{class}: {data}"
        );
    }
}

#[test]
fn a_decimal_range_or_filler_admits_every_decimal_value_whatever_its_tag() {
    let (compilation, report) = compile(
        "ex:A a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:amount ; owl:allValuesFrom xsd:decimal ] .
         ex:amount a owl:DatatypeProperty .
         ex:B a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:size ; owl:someValuesFrom xsd:decimal ] .
         ex:size a owl:DatatypeProperty .
         ex:C a owl:Class .
         ex:weight a owl:DatatypeProperty ; rdfs:domain ex:C ; rdfs:range xsd:decimal .",
        &example(),
    );
    let schema = &compilation.compiled.schema_json;
    judge(
        schema,
        "A",
        &[
            ("ex:amount 1.5", true),
            ("ex:amount 1", true),
            ("ex:amount \"7\"^^xsd:nonNegativeInteger", true),
            ("ex:amount \"-3\"^^xsd:byte", true),
            ("ex:amount \"x\"", false),
            ("ex:amount \"1.5E0\"^^xsd:double", false),
        ],
    );
    judge(
        schema,
        "B",
        &[("ex:size 2", true), ("ex:size \"2\"", false)],
    );
    judge(
        schema,
        "C",
        &[
            ("ex:weight 2", true),
            ("ex:weight 2.5", true),
            ("ex:weight true", false),
        ],
    );
    // A literal typed owl:rational is admitted unjudged, so the universal is an
    // approximation, not an exact projection.
    assert_eq!(
        outcomes(&report, "A", "amount"),
        vec![SchemaExpressionOutcome::Approximated]
    );
}

#[test]
fn a_string_filler_admits_the_string_datatypes_derived_from_it() {
    let (compilation, report) = compile(
        "ex:A a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:s ; owl:allValuesFrom xsd:string ] ,
             [ a owl:Restriction ; owl:onProperty ex:t ; owl:allValuesFrom xsd:token ] .
         ex:s a owl:DatatypeProperty .
         ex:t a owl:DatatypeProperty .",
        &example(),
    );
    let schema = &compilation.compiled.schema_json;
    judge(
        schema,
        "A",
        &[
            ("ex:s \"a\"", true),
            ("ex:s \"a b\"^^xsd:normalizedString", true),
            ("ex:s \"a\"^^xsd:token", true),
            ("ex:s \"en\"^^xsd:language", true),
            ("ex:s 5", false),
            ("ex:s \"a\"@en", false),
            // A string without stray spaces is a token; one with two spaces
            // in a row is not.
            ("ex:t \"a b\"", true),
            ("ex:t \"a  b\"", false),
            ("ex:t \" a \"^^xsd:token", true),
        ],
    );
    assert_eq!(
        outcomes(&report, "A", "s"),
        vec![SchemaExpressionOutcome::Projected]
    );
}

#[test]
fn a_facet_over_an_integer_admits_every_integer_datatype_within_it() {
    let (compilation, _) = compile(
        "ex:A a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:i ; owl:allValuesFrom
                 [ a rdfs:Datatype ; owl:onDatatype xsd:integer ;
                   owl:withRestrictions ( [ xsd:minInclusive 0 ] ) ] ] .
         ex:i a owl:DatatypeProperty .",
        &example(),
    );
    judge(
        &compilation.compiled.schema_json,
        "A",
        &[
            ("ex:i \"5\"^^xsd:nonNegativeInteger", true),
            ("ex:i \"5\"^^xsd:int", true),
            ("ex:i 5", true),
            ("ex:i \"5.0\"^^xsd:decimal", true),
            ("ex:i -1", false),
            ("ex:i \"-3\"^^xsd:int", false),
            ("ex:i 1.5", false),
        ],
    );
}

#[test]
fn an_enumeration_and_a_has_value_match_every_literal_of_an_equal_value() {
    let (compilation, report) = compile(
        "ex:A a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:n ;
               owl:allValuesFrom [ a rdfs:Datatype ; owl:oneOf ( 1 ) ] ] ,
             [ a owl:Restriction ; owl:onProperty ex:w ;
               owl:allValuesFrom [ a rdfs:Datatype ; owl:oneOf ( \"a b\" ) ] ] ,
             [ a owl:Restriction ; owl:onProperty ex:f ;
               owl:allValuesFrom [ a rdfs:Datatype ; owl:oneOf ( true ) ] ] .
         ex:n a owl:DatatypeProperty .
         ex:w a owl:DatatypeProperty .
         ex:f a owl:DatatypeProperty .
         ex:H a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:n ; owl:hasValue 1 ] .",
        &example(),
    );
    let schema = &compilation.compiled.schema_json;
    judge(
        schema,
        "A",
        &[
            ("ex:n 1", true),
            ("ex:n \"01\"^^xsd:integer", true),
            ("ex:n \"1.0\"^^xsd:decimal", true),
            ("ex:n \"+1\"^^xsd:unsignedByte", true),
            ("ex:n 2", false),
            ("ex:w \"a b\"", true),
            ("ex:w \" a  b \"^^xsd:token", true),
            ("ex:w \"a  b\"", false),
            ("ex:f true", true),
            ("ex:f \"1\"^^xsd:boolean", true),
            ("ex:f false", false),
        ],
    );
    judge(
        schema,
        "H",
        &[
            ("ex:n \"01\"^^xsd:integer", true),
            ("ex:n 1 , 3", true),
            ("ex:n 3", false),
        ],
    );
    // A string or boolean enumeration is matched exactly; a numeric one also
    // admits owl:rational literals unjudged.
    assert_eq!(
        outcomes(&report, "A", "w"),
        vec![SchemaExpressionOutcome::Projected]
    );
    assert_eq!(
        outcomes(&report, "A", "f"),
        vec![SchemaExpressionOutcome::Projected]
    );
    assert_eq!(
        outcomes(&report, "A", "n"),
        vec![SchemaExpressionOutcome::Approximated]
    );
}

#[test]
fn owl_time_durations_take_bare_integer_years() {
    // OWL-Time (W3C Recommendation, http://www.w3.org/2006/time):
    // `time:years` ranges over xsd:decimal.
    let ns = namespaces("time", "http://www.w3.org/2006/time#");
    let (compilation, _) = compile(
        "time:GeneralDurationDescription a owl:Class .
         time:years a owl:DatatypeProperty ;
             rdfs:domain time:GeneralDurationDescription ;
             rdfs:range xsd:decimal .",
        &ns,
    );
    let schema = &compilation.compiled.schema_json;
    let duration = |value: &str| {
        accepts(
            schema,
            &ns,
            "GeneralDurationDescription",
            &format!("ex:d a time:GeneralDurationDescription ; time:years {value} ."),
            &format!("{EX}d"),
        )
    };
    assert!(duration("1"), "one year is a decimal number of years");
    assert!(duration("1.5"));
    assert!(!duration("\"one\""));
}

#[test]
fn empty_single_and_repeated_boolean_members_are_read_by_their_meaning() {
    let (compilation, report) = compile(
        "ex:A a owl:Class .
         ex:p a owl:DatatypeProperty ; rdfs:domain ex:A .
         ex:B a owl:Class ; rdfs:subClassOf [ a owl:Class ; owl:unionOf ( ex:A ) ] .
         ex:C a owl:Class ; rdfs:subClassOf [ a owl:Class ; owl:intersectionOf ( ex:A ex:A ) ] .
         ex:N a owl:Class ; rdfs:subClassOf [ a owl:Class ; owl:unionOf () ] .
         ex:T a owl:Class ; rdfs:subClassOf [ a owl:Class ; owl:intersectionOf () ] .
         ex:E a owl:Class ; rdfs:subClassOf [ a owl:Class ; owl:oneOf () ] .
         ex:D a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:none ;
               owl:allValuesFrom [ a rdfs:Datatype ; owl:unionOf () ] ] ,
             [ a owl:Restriction ; owl:onProperty ex:any ;
               owl:allValuesFrom [ a rdfs:Datatype ; owl:intersectionOf () ] ] .
         ex:none a owl:DatatypeProperty .
         ex:any a owl:DatatypeProperty .",
        &example(),
    );
    let schema = &compilation.compiled.schema_json;
    // The union or intersection of one class is that class, so B and C are
    // in A and carry A's property.
    for class in ["B", "C"] {
        assert!(
            compilation
                .coverage
                .properties
                .iter()
                .find(|row| row.property_iri == format!("{EX}p"))
                .is_some_and(|row| row
                    .classes
                    .iter()
                    .any(|cell| cell.class_iri == format!("{EX}{class}")
                        && cell.status
                            == purrdf_shapes::json_schema::SchemaCoverageStatus::IncludedUnshaped)),
            "{class} ⊑ A"
        );
    }
    // The empty data union admits no value, the empty data intersection any.
    judge(
        schema,
        "D",
        &[
            ("ex:none 1", false),
            ("ex:any 1", true),
            ("ex:any \"x\"", true),
        ],
    );
    assert!(accepts(
        schema,
        &example(),
        "D",
        "ex:x a ex:D .",
        &format!("{EX}x")
    ));
    // Every axiom is reported; none is refused.
    let objects: Vec<&str> = report
        .axioms
        .iter()
        .map(|axiom| axiom.provenance.object.as_str())
        .collect();
    for expected in ["owl#Nothing", "owl#Thing"] {
        assert!(
            objects.iter().any(|object| object.contains(expected)),
            "{expected} in {objects:?}"
        );
    }
}

#[test]
fn cardinalities_beyond_64_bits_are_read_not_refused() {
    let (compilation, report) = compile(
        "ex:p a owl:ObjectProperty .
         ex:Many a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:p ; owl:maxCardinality 18446744073709551616 ] .
         ex:Never a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:p ; owl:minCardinality 99999999999999999999999 ] .",
        &example(),
    );
    let schema = &compilation.compiled.schema_json;
    // A maximum that large holds of any finite set of values; a minimum that
    // large of none.
    judge(
        schema,
        "Many",
        &[("ex:p ex:a , ex:b , ex:c", true), ("ex:p ex:a", true)],
    );
    judge(schema, "Never", &[("ex:p ex:a , ex:b , ex:c", false)]);
    assert!(!accepts(
        schema,
        &example(),
        "Never",
        "ex:x a ex:Never .",
        &format!("{EX}x")
    ));
    for class in ["Many", "Never"] {
        assert_eq!(
            outcomes(&report, class, "p"),
            vec![SchemaExpressionOutcome::Approximated],
            "{class}"
        );
    }
}

#[test]
fn prov_o_annotation_and_object_property_punning_compiles() {
    // PROV-O (W3C Recommendation, http://www.w3.org/ns/prov-o): an annotation
    // property punned as an object property.
    let ns = namespaces("prov", "http://www.w3.org/ns/prov#");
    let (compilation, _) = compile(
        "prov:Entity a owl:Class .
         prov:alternateOf a owl:ObjectProperty ;
             rdfs:domain prov:Entity ; rdfs:range prov:Entity .
         prov:specializationOf a owl:AnnotationProperty , owl:ObjectProperty ;
             rdfs:domain prov:Entity ; rdfs:range prov:Entity ;
             rdfs:subPropertyOf prov:alternateOf .",
        &ns,
    );
    let schema = &compilation.compiled.schema_json;
    let entity = |data: &str| {
        accepts(
            schema,
            &ns,
            "Entity",
            &format!("ex:e a prov:Entity ; {data} ."),
            &format!("{EX}e"),
        )
    };
    assert!(
        entity("prov:specializationOf ex:f"),
        "an entity value is a node"
    );
    assert!(
        !entity("prov:specializationOf \"text\""),
        "an object property takes no literal"
    );
    // A datatype declaration beside an object one decides the value kind.
    let (both, _) = compile(
        "ex:A a owl:Class .
         ex:q a owl:ObjectProperty , owl:DatatypeProperty ; rdfs:domain ex:A .",
        &example(),
    );
    judge(
        &both.compiled.schema_json,
        "A",
        &[("ex:q \"text\"", true), ("ex:q ex:node", false)],
    );
}

#[test]
fn a_hash_namespace_gives_the_schema_a_fragment_free_id() {
    // GoodRelations (http://purl.org/goodrelations/v1#) is a hash namespace.
    let ns = namespaces("gr", "http://purl.org/goodrelations/v1#");
    let (compilation, _) = compile("gr:Offering a owl:Class .", &ns);
    let schema: Value =
        purrdf_lex::json::read(&compilation.compiled.schema_json).expect("schema JSON");
    assert_eq!(
        schema["$id"],
        "http://purl.org/goodrelations/v1/schema/instance.schema.json"
    );
    // The schema registers and compiles under its own $id.
    let metaschemas = purrdf_jsonschema::Metaschemas::new(
        purrdf_testkit::jsonschema_metaschemas::DRAFT_2020_12
            .iter()
            .map(|&(uri, text)| {
                let document: Value = purrdf_lex::json::read(text).expect("meta-schema JSON");
                (uri, document)
            }),
    )
    .expect("meta-schemas");
    let id = schema["$id"].as_str().expect("$id").to_owned();
    let mut registry = purrdf_jsonschema::Registry::with_metaschemas(&metaschemas);
    registry
        .add_resource(&id, schema)
        .expect("the schema registers under its $id");
    registry
        .compile(&format!("{id}#/$defs/Offering"))
        .expect("the schema compiles");
    // Neighbour: a slash namespace keeps its $id.
    let (slash, _) = compile("ex:A a owl:Class .", &example());
    let slash: Value = purrdf_lex::json::read(&slash.compiled.schema_json).expect("schema JSON");
    assert_eq!(
        slash["$id"],
        "https://example.org/schema/schema/instance.schema.json"
    );
}

#[test]
fn graphql_names_a_nested_type_apart_from_a_colliding_class() {
    // GoodRelations declares gr:BusinessEntity and gr:BusinessEntityType; the
    // former's `@type` field would take the latter's GraphQL name.
    let ns = namespaces("gr", "http://purl.org/goodrelations/v1#");
    let (compilation, _) = compile(
        "gr:BusinessEntity a owl:Class .
         gr:BusinessEntityType a owl:Class .",
        &ns,
    );
    let config = GraphqlConfig::new("GoodRelations", "x", "y", "RdfValue").expect("config");
    let first = emit_graphql(&compilation.compiled, &config).expect("GraphQL emits");
    let second = emit_graphql(&compilation.compiled, &config).expect("GraphQL emits again");
    assert_eq!(first.artifacts, second.artifacts, "deterministic");
    let schema = first
        .artifacts
        .iter()
        .find(|(path, _)| path.ends_with(".graphql"))
        .map(|(_, text)| String::from_utf8(text.clone()).expect("UTF-8"))
        .expect("a GraphQL schema");
    assert!(schema.contains("type BusinessEntityType "), "{schema}");
    // Neighbour: without the colliding class the nested name is unchanged.
    let (alone, _) = compile("gr:BusinessEntity a owl:Class .", &ns);
    emit_graphql(&alone.compiled, &config).expect("GraphQL emits");
}

#[test]
fn shacl_datatypes_stay_judged_by_tag() {
    // `sh:datatype xsd:decimal` is SHACL's tag check, whatever OWL reads.
    let shapes = from_dataset(&parse(
        "ex:S a sh:NodeShape ; sh:targetClass ex:A ;
             sh:property [ sh:path ex:amount ; sh:datatype xsd:decimal ] .",
    ))
    .expect("shapes");
    let ontology = parse("ex:A a owl:Class .");
    let ns = example();
    let compilation = compile_schema(&SchemaCompileRequest::new(
        &shapes,
        &ns,
        ontology.as_ref(),
        SchemaSurfaceMode::OntologyComplete,
    ))
    .expect("compiles");
    judge(
        &compilation.compiled.schema_json,
        "A",
        &[("ex:amount 1.5", true), ("ex:amount 1", false)],
    );
}
