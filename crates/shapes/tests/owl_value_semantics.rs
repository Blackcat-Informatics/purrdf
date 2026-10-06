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

#[path = "support/turtle.rs"]
mod turtle;

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
    @prefix qudt: <http://qudt.org/schema/qudt/> .
    @prefix dtype: <http://www.linkedmodel.org/schema/dtype#> .
";
const EX: &str = "https://example.org/schema/";

fn namespaces(prefix: &str, iri: &str) -> Namespaces {
    Namespaces::new(prefix, &[(prefix.to_owned(), iri.to_owned())]).expect("namespace config")
}

fn example() -> Namespaces {
    namespaces("ex", EX)
}

/// The schema and the class-expression manifest of an ontology with no shapes.
fn compile(
    ontology: &str,
    namespaces: &Namespaces,
) -> (
    purrdf_shapes::SchemaCompilation,
    SchemaClassExpressionReport,
) {
    let shapes = from_dataset(&turtle::data(PREFIXES, "")).expect("shapes graph");
    let ontology = turtle::data(PREFIXES, ontology);
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
    let data = turtle::data(PREFIXES, data);
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
            ("ex:w \"a b\"^^xsd:token", true),
            ("ex:w \"a b\"^^xsd:normalizedString", true),
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
    let shapes = from_dataset(&turtle::data(
        PREFIXES,
        "ex:S a sh:NodeShape ; sh:targetClass ex:A ;
             sh:property [ sh:path ex:amount ; sh:datatype xsd:decimal ] .",
    ))
    .expect("shapes");
    let ontology = turtle::data(PREFIXES, "ex:A a owl:Class .");
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

#[test]
fn qudt_datatype_properties_over_classes_are_read_by_owl_2_full() {
    // QUDT (http://qudt.org/schema/qudt/): `qudt:numericValue` is an
    // owl:DatatypeProperty whose range is the owl:Class `qudt:NumericUnion`.
    // Read by the OWL 2 Full Semantics, its values are literals whose class
    // membership is not judged.
    let ns = namespaces("qudt", "http://qudt.org/schema/qudt/");
    let (compilation, report) = compile(
        "qudt:Concept a owl:Class .
         qudt:QuantityValue a owl:Class .
         qudt:NumericUnion a owl:Class ;
             rdfs:subClassOf qudt:Concept , dtype:numericUnion .
         qudt:numericValue a owl:DatatypeProperty ;
             rdfs:domain qudt:QuantityValue ;
             rdfs:range qudt:NumericUnion .
         qudt:Measured a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty qudt:numericValue ;
               owl:allValuesFrom qudt:NumericUnion ] .",
        &ns,
    );
    let schema = &compilation.compiled.schema_json;
    let value = |class: &str, data: &str| {
        accepts(
            schema,
            &ns,
            class,
            &format!("ex:v a qudt:{class} ; qudt:numericValue {data} ."),
            &format!("{EX}v"),
        )
    };
    assert!(value("QuantityValue", "5"), "a literal value");
    assert!(value("QuantityValue", "\"5.5\"^^xsd:decimal"));
    assert!(
        !value("QuantityValue", "ex:node"),
        "a datatype property takes no node"
    );
    assert!(value("Measured", "5"));
    assert!(!value("Measured", "ex:node"));
    // The class range and filler are approximations.
    let row = compilation
        .coverage
        .properties
        .iter()
        .find(|row| row.property_iri == "http://qudt.org/schema/qudt/numericValue")
        .expect("the qudt:numericValue row");
    assert!(row.classes.iter().any(|cell| cell.class_iri
        == "http://qudt.org/schema/qudt/QuantityValue"
        && cell.precision
            == purrdf_shapes::json_schema::SchemaCoveragePrecision::RepresentationApproximation));
    let filler: Vec<SchemaExpressionOutcome> = report
        .axioms
        .iter()
        .flat_map(|axiom| &axiom.classes)
        .filter(|row| row.class_iri == "http://qudt.org/schema/qudt/Measured")
        .flat_map(|row| &row.components)
        .map(|component| component.outcome)
        .collect();
    assert_eq!(filler, vec![SchemaExpressionOutcome::Approximated]);
}

#[test]
fn a_dl_valid_datatype_range_is_unchanged_beside_a_class_range() {
    // Neighbour: a datatype property over a datatype keeps its value-space
    // schema and its precision.
    let (compilation, _) = compile(
        "ex:A a owl:Class .
         ex:label a owl:DatatypeProperty ; rdfs:domain ex:A ; rdfs:range xsd:string .",
        &example(),
    );
    judge(
        &compilation.compiled.schema_json,
        "A",
        &[
            ("ex:label \"x\"", true),
            ("ex:label 5", false),
            ("ex:label ex:node", false),
        ],
    );
    let row = compilation
        .coverage
        .properties
        .iter()
        .find(|row| row.property_iri == format!("{EX}label"))
        .expect("the ex:label row");
    assert!(
        row.classes
            .iter()
            .any(|cell| cell.class_iri == format!("{EX}A")
                && cell.precision == purrdf_shapes::json_schema::SchemaCoveragePrecision::Exact)
    );
}

#[test]
fn string_facets_hold_of_every_string_datatype_literal() {
    // A string literal's lexical form is its value (XSD 1.1 Part 2 §3.4), so a
    // length or pattern facet over xsd:string holds of an xsd:token or
    // xsd:normalizedString literal's lexical form too.
    let (compilation, report) = compile(
        "ex:A a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:code ; owl:allValuesFrom
                 [ a rdfs:Datatype ; owl:onDatatype xsd:string ;
                   owl:withRestrictions ( [ xsd:maxLength 4 ] ) ] ] ,
             [ a owl:Restriction ; owl:onProperty ex:tag ; owl:allValuesFrom
                 [ a rdfs:Datatype ; owl:onDatatype xsd:token ;
                   owl:withRestrictions ( [ xsd:pattern \"[A-Z]+\" ] ) ] ] .
         ex:code a owl:DatatypeProperty .
         ex:tag a owl:DatatypeProperty .",
        &example(),
    );
    judge(
        &compilation.compiled.schema_json,
        "A",
        &[
            ("ex:code \"ABC\"", true),
            ("ex:code \"ABCDEFG\"", false),
            ("ex:code \"ABC\"^^xsd:token", true),
            ("ex:code \"ABCDEFG\"^^xsd:token", false),
            ("ex:code \"ABCDEFG\"^^xsd:normalizedString", false),
            ("ex:tag \"ABC\"^^xsd:token", true),
            ("ex:tag \"ABC\"", true),
            ("ex:tag \"abc\"^^xsd:language", false),
        ],
    );
    for property in ["code", "tag"] {
        assert_eq!(
            outcomes(&report, "A", property),
            vec![SchemaExpressionOutcome::Projected],
            "{property}"
        );
    }
}

#[test]
fn a_subclass_of_the_empty_union_admits_no_instance() {
    // `A ⊑ ⊔()` makes A a subclass of owl:Nothing: its definition is `false`.
    let (compilation, _) = compile(
        "ex:A a owl:Class ; rdfs:subClassOf [ a owl:Class ; owl:unionOf () ] .
         ex:B a owl:Class .",
        &example(),
    );
    let schema = &compilation.compiled.schema_json;
    let parsed: Value = purrdf_lex::json::read(schema).expect("schema JSON");
    assert_eq!(parsed["$defs"]["A"], Value::Bool(false));
    assert!(!accepts(
        schema,
        &example(),
        "A",
        "ex:x a ex:A .",
        &format!("{EX}x")
    ));
    // Neighbour: a class with no such axiom admits its instances.
    assert!(accepts(
        schema,
        &example(),
        "B",
        "ex:x a ex:B .",
        &format!("{EX}x")
    ));
}

#[test]
fn an_object_property_over_a_datatype_takes_its_literals() {
    // OWL 2 RDF-Based Semantics §5.3: every property is an owl:ObjectProperty,
    // so an object property over xsd:string takes string literals.
    let (compilation, _) = compile(
        "ex:A a owl:Class .
         ex:p a owl:ObjectProperty ; rdfs:domain ex:A ; rdfs:range xsd:string .",
        &example(),
    );
    judge(
        &compilation.compiled.schema_json,
        "A",
        &[("ex:p \"text\"", true), ("ex:p 5", false)],
    );
    let row = compilation
        .coverage
        .properties
        .iter()
        .find(|row| row.property_iri == format!("{EX}p"))
        .expect("the ex:p row");
    assert!(row.classes.iter().any(|cell| cell.precision
        == purrdf_shapes::json_schema::SchemaCoveragePrecision::RepresentationApproximation));
}

#[test]
fn an_object_property_restricted_to_a_data_range_takes_its_literals() {
    // OWL 2 Full (RDF-Based Semantics §5.3): an object property restricted to
    // a data range takes that range's literals, so its unranged values admit
    // literals and the restriction narrows them.
    let (compilation, _) = compile(
        "ex:A a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:p ; owl:someValuesFrom xsd:string ] .
         ex:B a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:q ; owl:allValuesFrom xsd:integer ] .
         ex:C a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:q ; owl:allValuesFrom
                 [ a rdfs:Datatype ; owl:onDatatype xsd:integer ;
                   owl:withRestrictions ( [ xsd:maxInclusive 10 ] ) ] ] .
         ex:p a owl:ObjectProperty .
         ex:q a owl:ObjectProperty .",
        &example(),
    );
    let schema = &compilation.compiled.schema_json;
    judge(
        schema,
        "A",
        &[
            ("ex:p \"hello\"", true),
            // An IRI may denote a data value (OWL 2 RDF-Based Semantics), so
            // a node meets an object property's data range, unjudged.
            ("ex:p ex:node", true),
            ("ex:p 3", false),
        ],
    );
    judge(
        schema,
        "B",
        &[
            ("ex:q 3", true),
            ("ex:q ex:node", true),
            ("ex:q \"three\"", false),
        ],
    );
    judge(
        schema,
        "C",
        &[("ex:q 3", true), ("ex:q 11", false), ("ex:q ex:node", true)],
    );
    let row = compilation
        .coverage
        .properties
        .iter()
        .find(|row| row.property_iri == format!("{EX}p"))
        .expect("the ex:p row");
    assert!(row.classes.iter().any(|cell| cell.precision
        == purrdf_shapes::json_schema::SchemaCoveragePrecision::RepresentationApproximation));
}

#[test]
fn literals_an_object_property_takes_are_admitted_on_every_class_carrying_it() {
    // Taking literals is a fact about the property, not about the class the
    // restriction is asserted of: an instance of A ⊑ F ⊓ ∃p.xsd:string with
    // a string value is valid for F, p's domain, and for a sibling of A.
    let (compilation, _) = compile(
        "ex:F a owl:Class .
         ex:p a owl:ObjectProperty ; rdfs:domain ex:F .
         ex:A a owl:Class ; rdfs:subClassOf ex:F ,
             [ a owl:Restriction ; owl:onProperty ex:p ; owl:someValuesFrom xsd:string ] .
         ex:E a owl:Class ; rdfs:subClassOf ex:F .",
        &example(),
    );
    let schema = &compilation.compiled.schema_json;
    let x = format!("{EX}x");
    for (class, data, expected) in [
        ("A", "ex:x a ex:A ; ex:p \"hello\" .", true),
        ("F", "ex:x a ex:A ; ex:p \"hello\" .", true),
        ("F", "ex:x a ex:A , ex:F ; ex:p \"hello\" .", true),
        ("E", "ex:x a ex:E ; ex:p \"hello\" .", true),
        // The restriction still narrows the class it is asserted of.
        ("A", "ex:x a ex:A ; ex:p ex:node .", true),
        ("A", "ex:x a ex:A ; ex:p 3 .", false),
        ("A", "ex:x a ex:A .", false),
        // Neighbours: F and E keep their nodes, and need no value.
        ("F", "ex:x a ex:F ; ex:p ex:node .", true),
        ("E", "ex:x a ex:E .", true),
    ] {
        assert_eq!(
            accepts(schema, &example(), class, data, &x),
            expected,
            "{class}: {data}"
        );
    }
}

#[test]
fn a_cross_kind_has_value_or_has_self_is_read_by_owl_2_full() {
    // OWL 2 Full (RDF-Based Semantics §5.3): a literal owl:hasValue on an
    // object property is ∃r.{"fixed"}, an individual one on a datatype
    // property is ∃d.{ex:v}, and owl:hasSelf on a datatype property is the
    // self restriction, each read rather than refused.
    let (compilation, report) = compile(
        "ex:G a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:r ; owl:hasValue \"fixed\" ] .
         ex:r a owl:ObjectProperty .
         ex:V a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:d ; owl:hasValue ex:v ] .
         ex:d a owl:DatatypeProperty .
         ex:S a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:e ; owl:hasSelf true ] .
         ex:e a owl:DatatypeProperty .",
        &example(),
    );
    let schema = &compilation.compiled.schema_json;
    judge(
        schema,
        "G",
        &[
            ("ex:r \"fixed\"", true),
            ("ex:r \"other\"", false),
            ("ex:r ex:fixed", false),
        ],
    );
    judge(
        schema,
        "V",
        &[
            ("ex:d ex:v", true),
            ("ex:d ex:w", false),
            ("ex:d \"v\"", false),
        ],
    );
    // No schema keyword at the value location states that the value is the
    // focus node itself, so S's self restriction is reported unrepresented,
    // as on an object property, and its self value is admitted.
    judge(schema, "S", &[("ex:e ex:x", true)]);
    assert_eq!(
        outcomes(&report, "S", "e"),
        [SchemaExpressionOutcome::Unrepresented]
    );
    assert_eq!(
        outcomes(&report, "G", "r"),
        [SchemaExpressionOutcome::Approximated]
    );
}

/// The coverage precision of `property` on `class`.
fn cell_precision(
    compilation: &purrdf_shapes::SchemaCompilation,
    property: &str,
    class: &str,
) -> purrdf_shapes::json_schema::SchemaCoveragePrecision {
    compilation
        .coverage
        .properties
        .iter()
        .find(|row| row.property_iri == format!("{EX}{property}"))
        .and_then(|row| {
            row.classes
                .iter()
                .find(|cell| cell.class_iri == format!("{EX}{class}"))
        })
        .unwrap_or_else(|| panic!("the {property} cell on {class}"))
        .precision
}

#[test]
fn class_fillers_of_a_property_that_takes_literals_admit_literals() {
    // Under OWL 2 Full a class extension may hold literals, owl:Thing's
    // included, so once p takes literals (A ⊑ ∃p.xsd:string) a class filler
    // of p admits them on every class, as a class range does.
    let (compilation, report) = compile(
        "ex:K a owl:Class . ex:F a owl:Class .
         ex:p a owl:ObjectProperty ; rdfs:domain ex:F .
         ex:A a owl:Class ; rdfs:subClassOf ex:F ,
             [ a owl:Restriction ; owl:onProperty ex:p ; owl:someValuesFrom xsd:string ] .
         ex:T a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:p ; owl:someValuesFrom owl:Thing ] .
         ex:AT a owl:Class ; rdfs:subClassOf ex:A , ex:T .
         ex:N a owl:Class ; rdfs:subClassOf ex:F ,
             [ a owl:Restriction ; owl:onProperty ex:p ; owl:allValuesFrom ex:K ] .
         ex:B a owl:Class ; rdfs:subClassOf ex:F ,
             [ a owl:Restriction ; owl:onProperty ex:p ; owl:allValuesFrom xsd:integer ] .
         ex:F2 a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:q2 ; owl:allValuesFrom ex:K ] .
         ex:A2 a owl:Class ; rdfs:subClassOf ex:F2 ,
             [ a owl:Restriction ; owl:onProperty ex:q2 ; owl:someValuesFrom xsd:string ] .
         ex:q2 a owl:ObjectProperty .
         ex:dq a owl:DatatypeProperty .
         ex:DB a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:dq ; owl:allValuesFrom xsd:string ] .",
        &example(),
    );
    let schema = &compilation.compiled.schema_json;
    judge(schema, "T", &[("ex:p \"s\"", true), ("ex:p ex:n", true)]);
    assert!(!accepts(
        schema,
        &example(),
        "T",
        "ex:x a ex:T .",
        &format!("{EX}x")
    ));
    judge(
        schema,
        "AT",
        &[("ex:p \"s\"", true), ("ex:p ex:n", true), ("ex:p 3", false)],
    );
    judge(schema, "N", &[("ex:p \"lit\"", true), ("ex:p ex:k", true)]);
    // A data-range filler keeps its own literals, and admits nodes, which an
    // IRI may denote a data value for.
    judge(
        schema,
        "B",
        &[("ex:p 3", true), ("ex:p ex:n", true), ("ex:p \"s\"", false)],
    );
    // The node is admitted unjudged, so B's ∀p.xsd:integer and its cell are
    // approximations.
    assert_eq!(
        outcomes(&report, "B", "p"),
        [SchemaExpressionOutcome::Approximated]
    );
    assert_eq!(
        cell_precision(&compilation, "p", "B"),
        purrdf_shapes::json_schema::SchemaCoveragePrecision::RepresentationApproximation
    );
    // Neighbour: a datatype property's exact data range still rejects a
    // node, and is projected.
    judge(
        schema,
        "DB",
        &[("ex:dq \"s\"", true), ("ex:dq ex:n", false)],
    );
    assert_eq!(
        outcomes(&report, "DB", "dq"),
        [SchemaExpressionOutcome::Projected]
    );
    judge(
        schema,
        "A2",
        &[
            ("ex:q2 \"s\"", true),
            ("ex:q2 ex:k", true),
            ("ex:q2 3", false),
        ],
    );
    for class in ["F2", "A2"] {
        assert!(
            accepts(
                schema,
                &example(),
                class,
                "ex:x a ex:A2 ; ex:q2 \"s\" .",
                &format!("{EX}x")
            ),
            "{class}"
        );
    }
}

#[test]
fn cross_kind_and_self_restricted_cells_are_approximations() {
    use purrdf_shapes::json_schema::SchemaCoveragePrecision::{Exact, RepresentationApproximation};
    let (compilation, _) = compile(
        "ex:F a owl:Class . ex:W a owl:Class .
         ex:p a owl:ObjectProperty ; rdfs:domain ex:F .
         ex:A a owl:Class ; rdfs:subClassOf ex:F ,
             [ a owl:Restriction ; owl:onProperty ex:p ; owl:someValuesFrom xsd:string ] .
         ex:u a owl:ObjectProperty ; rdfs:domain ex:F .
         ex:U a owl:Class ; rdfs:subClassOf ex:F ,
             [ a owl:Restriction ; owl:onProperty ex:u ; owl:hasValue \"fixed\" ] .
         ex:d a owl:DatatypeProperty ; rdfs:domain ex:W .
         ex:V a owl:Class ; rdfs:subClassOf ex:W ,
             [ a owl:Restriction ; owl:onProperty ex:d ; owl:hasValue ex:v ] .
         ex:o a owl:ObjectProperty .
         ex:SO a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:o ; owl:hasSelf true ] .
         ex:plain a owl:ObjectProperty ; rdfs:domain ex:F .",
        &example(),
    );
    // Widened by another class's restriction, so approximate on F too.
    assert_eq!(
        cell_precision(&compilation, "p", "F"),
        RepresentationApproximation
    );
    assert_eq!(
        cell_precision(&compilation, "u", "F"),
        RepresentationApproximation
    );
    assert_eq!(
        cell_precision(&compilation, "d", "W"),
        RepresentationApproximation
    );
    // A self restriction, reported unrepresented, is no exact cell.
    assert_eq!(
        cell_precision(&compilation, "o", "SO"),
        RepresentationApproximation
    );
    // Neighbours: a plain object property, and o on a class without the
    // self restriction, stay exact.
    assert_eq!(cell_precision(&compilation, "plain", "F"), Exact);
    assert_eq!(cell_precision(&compilation, "o", "F"), Exact);
}

#[test]
fn a_datatype_range_admits_the_node_a_has_value_gives_it() {
    // OWL 2 Full: ds ranges over xsd:string, and VS ⊑ ∃ds.{ex:v}; an IRI may
    // denote a string, so ex:v is admitted, as an approximation.
    let (compilation, _) = compile(
        "ex:W a owl:Class .
         ex:ds a owl:DatatypeProperty ; rdfs:domain ex:W ; rdfs:range xsd:string .
         ex:VS a owl:Class ; rdfs:subClassOf ex:W ,
             [ a owl:Restriction ; owl:onProperty ex:ds ; owl:hasValue ex:v ] .",
        &example(),
    );
    let schema = &compilation.compiled.schema_json;
    judge(
        schema,
        "VS",
        &[
            ("ex:ds ex:v", true),
            ("ex:ds ex:w", false),
            ("ex:ds \"v\"", false),
        ],
    );
    for class in ["VS", "W"] {
        assert!(
            accepts(
                schema,
                &example(),
                class,
                "ex:x a ex:VS ; ex:ds ex:v .",
                &format!("{EX}x")
            ),
            "{class}"
        );
    }
    // Neighbour: W's own string values are still literals of the range.
    judge(schema, "W", &[("ex:ds \"s\"", true), ("ex:ds 3", false)]);
    assert_eq!(
        cell_precision(&compilation, "ds", "W"),
        purrdf_shapes::json_schema::SchemaCoveragePrecision::RepresentationApproximation
    );
}

#[test]
fn a_node_value_meets_an_object_property_data_range() {
    // OWL 2 RDF-Based Semantics: an IRI may denote a data value, so the node
    // ex:v meets ∀p.xsd:integer and an object property's xsd:integer range.
    let (compilation, _) = compile(
        "ex:p a owl:ObjectProperty .
         ex:X a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:p ; owl:hasValue ex:v ] ,
             [ a owl:Restriction ; owl:onProperty ex:p ; owl:allValuesFrom xsd:integer ] .
         ex:p3 a owl:ObjectProperty ; rdfs:range xsd:integer .
         ex:XR a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:p3 ; owl:hasValue ex:v ] .",
        &example(),
    );
    let schema = &compilation.compiled.schema_json;
    judge(schema, "X", &[("ex:p ex:v", true), ("ex:p ex:w", false)]);
    judge(
        schema,
        "XR",
        &[
            ("ex:p3 ex:v", true),
            ("ex:p3 ex:w", false),
            ("ex:p3 3", false),
        ],
    );
}

#[test]
fn empty_fillers_and_ranges_admit_no_value() {
    // owl:Nothing, and what normalises to it (¬owl:Thing), admits no value in
    // any filler or range position, stated exactly.
    let (compilation, report) = compile(
        "ex:p a owl:ObjectProperty .
         ex:SN a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:p ; owl:someValuesFrom owl:Nothing ] .
         ex:AN a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:p ; owl:allValuesFrom owl:Nothing ] .
         ex:AC a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:p ;
               owl:allValuesFrom [ owl:complementOf owl:Thing ] ] .
         ex:MQ a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:p ;
               owl:minQualifiedCardinality \"1\"^^xsd:nonNegativeInteger ; owl:onClass owl:Nothing ] .
         ex:T a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:p ; owl:someValuesFrom owl:Thing ] .
         ex:pn a owl:ObjectProperty ; rdfs:range owl:Nothing .
         ex:PN a owl:Class ; rdfs:subClassOf
             [ a owl:Restriction ; owl:onProperty ex:pn ; owl:someValuesFrom xsd:string ] .
         ex:PO a owl:Class .
         ex:pn rdfs:domain ex:PO .
         ex:pz a owl:ObjectProperty ; rdfs:domain ex:PO ; rdfs:range owl:Nothing .",
        &example(),
    );
    let schema = &compilation.compiled.schema_json;
    let x = format!("{EX}x");
    let instance = |class: &str, data: &str| {
        accepts(
            schema,
            &example(),
            class,
            &format!("ex:x a ex:{class} {data} ."),
            &x,
        )
    };
    // An existential over the empty class admits no instance.
    for class in ["SN", "MQ"] {
        assert!(!instance(class, ""), "{class}");
        assert!(!instance(class, "; ex:p ex:n"), "{class}");
        assert!(!instance(class, "; ex:p \"x\""), "{class}");
    }
    // A universal over it admits no value; with none the instance is valid.
    for class in ["AN", "AC"] {
        assert!(instance(class, ""), "{class}");
        assert!(!instance(class, "; ex:p ex:n"), "{class}");
        assert!(!instance(class, "; ex:p \"x\""), "{class}");
    }
    // A range of owl:Nothing admits no value, so PN, which needs one, admits
    // no instance; PO admits one without a value.
    assert!(!instance("PN", "; ex:pn \"x\""));
    assert!(!instance("PN", ""));
    assert!(instance("PO", ""));
    assert!(!instance("PO", "; ex:pn ex:n"));
    assert!(!instance("PO", "; ex:pz ex:n"));
    // Neighbour: owl:Thing admits a value.
    assert!(instance("T", "; ex:p ex:n"));
    for (class, property) in [("SN", "p"), ("AN", "p"), ("AC", "p"), ("MQ", "p")] {
        assert_eq!(
            outcomes(&report, class, property),
            [SchemaExpressionOutcome::Projected],
            "{class}"
        );
    }
    assert_eq!(
        cell_precision(&compilation, "pz", "PO"),
        purrdf_shapes::json_schema::SchemaCoveragePrecision::Exact
    );
}
