// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Cross-emitter proofs for ontology-complete developer-schema surfaces, and
//! for the anonymous OWL class expressions in them: the relation and its
//! provenance, the class-expression manifest, the JSON Schema projection
//! judged against projected instances, every language emitter, and
//! determinism.

use std::collections::BTreeMap;
use std::sync::Arc;

use purrdf_lex::json::Value;
use purrdf_rdf::RdfDataset;
use purrdf_shapes::json_schema::{
    Namespaces, SchemaCompileRequest, SchemaCoveragePrecision, SchemaCoverageStatus,
    SchemaSurfaceMode, compile_schema,
};
use purrdf_shapes::shapes::{Shapes, from_dataset};
use purrdf_shapes::{
    GRAPHQL_SCHEMA_PATH, GraphqlConfig, LinkmlConfig, PydanticConfig, SchemaClassExpressionReport,
    SchemaExpressionOutcome, TYPESCRIPT_DECLARATION_PATH, TypeScriptConfig,
    compile_schema_with_class_expressions, emit_graphql, emit_linkml, emit_pydantic,
    emit_typescript,
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
    let shapes_dataset = purrdf_shapes::text_ingest::parse_turtle_to_dataset(
        &format!(
            "{PREFIXES}
        ex:PersonShape a sh:NodeShape ; sh:targetClass ex:Person ;
            sh:property [ sh:path ex:name ; sh:minCount 1 ; sh:datatype xsd:string ] ."
        ),
        None,
    )
    .expect("shapes Turtle");
    let shapes = from_dataset(&shapes_dataset).expect("shapes graph");
    let ontology = purrdf_shapes::text_ingest::parse_turtle_to_dataset(
        &format!(
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
        ),
        None,
    )
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

fn generated_definition_block<'a>(source: &'a str, start: &str, next: &str) -> &'a str {
    let offset = source.find(start).expect("generated definition start");
    let tail = &source[offset..];
    let end = tail[start.len()..]
        .find(next)
        .map_or(tail.len(), |next_offset| start.len() + next_offset);
    &tail[..end]
}

#[test]
fn ontology_property_surface_reaches_every_language_emitter() {
    let compilation = compiled_surface();
    let compiled = &compilation.compiled;

    let linkml = emit_linkml(compiled, &linkml_config()).expect("LinkML emission");
    let linkml_classes = &linkml.document.as_value()["classes"];
    assert!(linkml_classes["EmailMessage"].as_object().is_some());
    assert!(
        linkml_classes["EmailMessage"]["attributes"]["ex:resentDate"]
            .as_object()
            .is_some(),
        "LinkML must retain the ontology-only resentDate attribute"
    );
    assert!(
        linkml_classes["EmailMessage"]["attributes"]["ex:resentMessageId"]
            .as_object()
            .is_some(),
        "LinkML must retain the ontology-only resentMessageId attribute"
    );

    let typescript = emit_typescript(compiled, &typescript_config()).expect("TypeScript emission");
    let declarations = std::str::from_utf8(&typescript.artifacts[TYPESCRIPT_DECLARATION_PATH])
        .expect("UTF-8 TypeScript");
    let email_type = &typescript.type_names["EmailMessage"];
    let email_declaration = generated_definition_block(
        declarations,
        &format!("export type {email_type} = "),
        "\nexport type ",
    );
    assert!(email_declaration.contains("ex:resentDate"));
    assert!(email_declaration.contains("ex:resentMessageId"));

    let graphql = emit_graphql(compiled, &graphql_config()).expect("GraphQL emission");
    assert!(graphql.artifacts[GRAPHQL_SCHEMA_PATH].is_ascii());
    let email_fields = &graphql.names.fields["#/$defs/EmailMessage"];
    assert!(email_fields.contains_key("ex:resentDate"));
    assert!(email_fields.contains_key("ex:resentMessageId"));

    let pydantic = emit_pydantic(compiled, &pydantic_config()).expect("Pydantic emission");
    let models = std::str::from_utf8(&pydantic.artifacts["example_ontology/models.py"])
        .expect("UTF-8 Pydantic models");
    let email_model = pydantic.model_paths["EmailMessage"]
        .rsplit('.')
        .next()
        .expect("generated model name");
    let email_model =
        generated_definition_block(models, &format!("class {email_model}("), "\nclass ");
    assert!(email_model.contains("alias=\"ex:resentDate\""));
    assert!(email_model.contains("alias=\"ex:resentMessageId\""));

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

// ── Anonymous OWL class expressions ─────────────────────────────────────────

const EX: &str = "https://example.org/schema/";

/// Every restriction form (`owl:someValuesFrom`, `owl:allValuesFrom`,
/// `owl:hasValue`, `owl:hasSelf`, the three unqualified and three qualified
/// cardinalities over `owl:onClass` and `owl:onDataRange`), every boolean form
/// (`owl:unionOf`, `owl:intersectionOf`, `owl:complementOf`, `owl:oneOf`), the
/// data ranges (`owl:onDatatype`/`owl:withRestrictions`,
/// `owl:datatypeComplementOf`, a literal `owl:oneOf`), an inverse property, a
/// general class inclusion, and equivalences in both directions.
const ONTOLOGY: &str = r#"
    ex:Agent a owl:Class .
    ex:Robot a owl:Class ;
        rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:name ; owl:maxCardinality 0 ] .
    ex:Organization a owl:Class .
    ex:Person a owl:Class ;
        rdfs:subClassOf ex:Agent ,
            [ a owl:Restriction ; owl:onProperty ex:name ; owl:someValuesFrom xsd:string ] ,
            [ a owl:Restriction ; owl:onProperty ex:knows ; owl:allValuesFrom ex:Person ] ,
            [ a owl:Restriction ; owl:onProperty ex:status ; owl:hasValue ex:active ] ,
            [ a owl:Restriction ; owl:onProperty ex:nickname ;
                owl:maxCardinality "3"^^xsd:nonNegativeInteger ] ,
            [ a owl:Restriction ; owl:onProperty ex:nickname ; owl:maxQualifiedCardinality 1 ;
                owl:onDataRange [ a rdfs:Datatype ; owl:oneOf ( "Bob" "Rob" ) ] ] ,
            [ a owl:Restriction ; owl:onProperty ex:email ; owl:minCardinality 1 ] ,
            [ a owl:Restriction ; owl:onProperty ex:email ; owl:allValuesFrom
                [ a rdfs:Datatype ; owl:onDatatype xsd:string ;
                  owl:withRestrictions ( [ xsd:pattern "[^@]+@[^@]+" ] ) ] ] ,
            [ a owl:Restriction ; owl:onProperty ex:birthDate ; owl:cardinality 1 ] ,
            [ a owl:Restriction ; owl:onProperty ex:parent ;
                owl:maxQualifiedCardinality 2 ; owl:onClass ex:Person ] ,
            [ a owl:Restriction ; owl:onProperty ex:phone ;
                owl:minQualifiedCardinality 1 ; owl:onDataRange xsd:string ] ,
            [ a owl:Restriction ; owl:onProperty ex:score ; owl:qualifiedCardinality 1 ;
                owl:onDataRange [ a rdfs:Datatype ; owl:onDatatype xsd:integer ;
                    owl:withRestrictions ( [ xsd:minInclusive 0 ] [ xsd:maxInclusive 100 ] ) ] ] ,
            [ a owl:Restriction ; owl:onProperty ex:code ;
                owl:allValuesFrom [ a rdfs:Datatype ; owl:datatypeComplementOf xsd:integer ] ] ,
            [ a owl:Class ; owl:complementOf ex:Robot ] ,
            [ a owl:Restriction ; owl:onProperty ex:self ; owl:hasSelf true ] ,
            [ a owl:Restriction ; owl:onProperty [ owl:inverseOf ex:knows ] ;
                owl:someValuesFrom ex:Person ] .
    ex:Contact a owl:Class ;
        rdfs:subClassOf [ a owl:Class ; owl:unionOf (
            [ a owl:Restriction ; owl:onProperty ex:email ; owl:minCardinality 1 ]
            [ a owl:Restriction ; owl:onProperty ex:phone ; owl:minCardinality 1 ]
        ) ] .
    ex:Status a owl:Class ;
        owl:equivalentClass [ a owl:Class ; owl:oneOf ( ex:active ex:inactive ) ] .
    ex:Parent a owl:Class ;
        owl:equivalentClass [ a owl:Class ; owl:intersectionOf ( ex:Person
            [ a owl:Restriction ; owl:onProperty ex:child ; owl:someValuesFrom ex:Person ] ) ] .
    ex:Party a owl:Class ;
        owl:equivalentClass [ a owl:Class ; owl:unionOf ( ex:Person ex:Organization ) ] .
    [ a owl:Class ; owl:unionOf ( ex:Robot ex:Organization ) ]
        rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:serial ; owl:minCardinality 1 ] .
    [ a owl:Restriction ; owl:onProperty ex:serial ; owl:someValuesFrom xsd:string ]
        rdfs:subClassOf ex:Robot .

    ex:name a owl:DatatypeProperty ; rdfs:range xsd:string .
    ex:knows a owl:ObjectProperty .
    ex:status a owl:ObjectProperty ; rdfs:range ex:Status .
    ex:nickname a owl:DatatypeProperty ; rdfs:range xsd:string .
    ex:email a owl:DatatypeProperty ; rdfs:range xsd:string .
    ex:birthDate a owl:DatatypeProperty ; rdfs:range xsd:date .
    ex:parent a owl:ObjectProperty .
    ex:hasChild a owl:ObjectProperty ; owl:equivalentProperty [ owl:inverseOf ex:parent ] .
    ex:child a owl:ObjectProperty .
    ex:phone a owl:DatatypeProperty .
    ex:score a owl:DatatypeProperty .
    ex:code a owl:DatatypeProperty .
    ex:self a owl:ObjectProperty .
    ex:serial a owl:DatatypeProperty ; rdfs:range xsd:string .
    ex:member a owl:ObjectProperty ; rdfs:domain ex:Party .
    ex:Percent owl:equivalentClass [ a rdfs:Datatype ; owl:onDatatype xsd:integer ;
        owl:withRestrictions ( [ xsd:minInclusive 0 ] [ xsd:maxInclusive 100 ] ) ] .
    ex:rating a owl:DatatypeProperty ; rdfs:range ex:Percent .
"#;

/// An ontology with only IRI objects — the fragment the surface accepted
/// before anonymous expressions were modelled.
const IRI_ONLY: &str = r"
    ex:Agent a owl:Class .
    ex:Person a owl:Class ; rdfs:subClassOf ex:Agent .
    ex:name a owl:DatatypeProperty ; rdfs:domain ex:Person ; rdfs:range xsd:string .
    ex:knows a owl:ObjectProperty ; rdfs:domain ex:Agent ; rdfs:range ex:Person .
";

fn iri(local: &str) -> String {
    format!("{EX}{local}")
}

fn namespaces() -> Namespaces {
    Namespaces::new("ex", &[("ex".to_owned(), EX.to_owned())]).expect("namespace config")
}

fn parse(turtle: &str) -> Arc<RdfDataset> {
    purrdf_shapes::text_ingest::parse_turtle_to_dataset(&format!("{PREFIXES}{turtle}"), None)
        .expect("Turtle")
}

fn shapes(turtle: &str) -> Shapes {
    from_dataset(&parse(turtle)).expect("shapes graph")
}

fn compile_both(
    shapes_turtle: &str,
    ontology: &str,
    mode: SchemaSurfaceMode,
) -> (
    purrdf_shapes::SchemaCompilation,
    SchemaClassExpressionReport,
) {
    let shapes = shapes(shapes_turtle);
    let ontology = parse(ontology);
    let namespaces = namespaces();
    compile_schema_with_class_expressions(&SchemaCompileRequest::new(
        &shapes,
        &namespaces,
        ontology.as_ref(),
        mode,
    ))
    .expect("anonymous class expressions are modelled, not refused")
}

fn complete() -> (
    purrdf_shapes::SchemaCompilation,
    SchemaClassExpressionReport,
) {
    compile_both("", ONTOLOGY, SchemaSurfaceMode::OntologyComplete)
}

/// The outcomes the manifest reports for one expression on one class.
fn outcomes(
    report: &SchemaClassExpressionReport,
    class: &str,
    expression: &str,
) -> Vec<SchemaExpressionOutcome> {
    report
        .axioms
        .iter()
        .flat_map(|axiom| &axiom.classes)
        .filter(|row| row.class_iri == iri(class))
        .flat_map(|row| &row.components)
        .filter(|component| component.expression == expression)
        .map(|component| component.outcome)
        .collect()
}

#[test]
fn ontology_with_every_anonymous_class_expression_compiles() {
    let shapes = shapes("");
    let ontology = parse(ONTOLOGY);
    let namespaces = namespaces();
    let compilation = compile_schema(&SchemaCompileRequest::new(
        &shapes,
        &namespaces,
        ontology.as_ref(),
        SchemaSurfaceMode::OntologyComplete,
    ))
    .expect("anonymous class expressions are modelled, not refused");
    assert!(compilation.compiled.schema_json.contains("\"Person\""));
}

#[test]
fn manifest_reports_every_form_with_its_outcome_and_provenance() {
    use SchemaExpressionOutcome::{Approximated, Projected, Unrepresented};
    let (_, report) = complete();
    let xsd = |local: &str| format!("<http://www.w3.org/2001/XMLSchema#{local}>");
    let p = |local: &str| format!("<{EX}{local}>");
    let cases: Vec<(&str, String, Vec<SchemaExpressionOutcome>)> = vec![
        (
            "Person",
            format!("some({},{})", p("name"), xsd("string")),
            vec![Approximated],
        ),
        (
            "Person",
            format!("all({},{})", p("knows"), p("Person")),
            vec![Approximated],
        ),
        (
            "Person",
            format!("has_value({},{})", p("status"), p("active")),
            vec![Approximated],
        ),
        (
            "Person",
            format!("max(3,{})", p("nickname")),
            vec![Approximated],
        ),
        (
            "Person",
            format!("max(1,{},one_of(\"Bob\",\"Rob\"))", p("nickname")),
            vec![Approximated],
        ),
        (
            "Person",
            format!("min(1,{})", p("email")),
            vec![Approximated],
        ),
        (
            "Person",
            format!(
                "all({},datatype_restriction({},{}=\"[^@]+@[^@]+\"))",
                p("email"),
                xsd("string"),
                xsd("pattern")
            ),
            vec![Projected],
        ),
        (
            "Person",
            format!("exact(1,{})", p("birthDate")),
            vec![Approximated],
        ),
        (
            "Person",
            format!("max(2,{},{})", p("parent"), p("Person")),
            vec![Unrepresented],
        ),
        (
            "Person",
            format!("min(1,{},{})", p("phone"), xsd("string")),
            vec![Approximated],
        ),
        (
            "Person",
            format!("all({},datatype_complement({}))", p("code"), xsd("integer")),
            vec![Approximated],
        ),
        (
            "Person",
            format!("complement({})", p("Robot")),
            vec![Approximated],
        ),
        (
            "Person",
            format!("has_self({})", p("self")),
            vec![Unrepresented],
        ),
        (
            "Person",
            format!("some(inverse({}),{})", p("knows"), p("Person")),
            vec![Unrepresented],
        ),
        (
            "Person",
            format!("union({},{})", p("Organization"), p("Person")),
            vec![Projected],
        ),
        (
            "Party",
            format!("union({},{})", p("Organization"), p("Person")),
            vec![Unrepresented],
        ),
        (
            "Contact",
            format!("union(min(1,{}),min(1,{}))", p("email"), p("phone")),
            vec![Approximated],
        ),
        (
            "Status",
            format!("one_of({},{})", p("active"), p("inactive")),
            vec![Approximated],
        ),
        ("Parent", p("Person"), vec![Projected]),
        (
            "Parent",
            format!("some({},{})", p("child"), p("Person")),
            vec![Approximated],
        ),
        // A subclass inheriting a component with its owner's outcome has no
        // row of its own: Person's row is Parent's, through the hierarchy.
        ("Parent", format!("has_self({})", p("self")), vec![]),
        ("Robot", format!("max(0,{})", p("name")), vec![Approximated]),
        (
            "Robot",
            format!("min(1,{})", p("serial")),
            vec![Approximated],
        ),
        (
            "Organization",
            format!("min(1,{})", p("serial")),
            vec![Approximated],
        ),
    ];
    for (class, expression, expected) in cases {
        assert_eq!(
            outcomes(&report, class, &expression),
            expected,
            "{class} / {expression}"
        );
    }
    let score = report
        .axioms
        .iter()
        .find(|axiom| {
            axiom
                .provenance
                .object
                .starts_with(&format!("exact(1,{}", p("score")))
        })
        .expect("score axiom");
    assert_eq!(score.provenance.subject, iri("Person"));
    assert_eq!(
        score.provenance.predicate,
        "http://www.w3.org/2000/01/rdf-schema#subClassOf"
    );
    assert_eq!(
        outcomes(&report, "Person", &score.provenance.object),
        vec![Approximated],
        "a qualifier whose facets the value schema states is counted"
    );

    // A datatype definition is an axiom-level component, and the defined
    // datatype is no class.
    let percent = report
        .axioms
        .iter()
        .find(|axiom| axiom.provenance.subject == iri("Percent"))
        .expect("datatype definition axiom");
    assert_eq!(percent.classes.len(), 0);
    assert_eq!(percent.components.len(), 1);
    assert_eq!(percent.components[0].outcome, Approximated);
    assert!(
        percent.components[0]
            .reason
            .contains("a datatype definition")
    );

    // Components no named class carries are reported on the axiom itself.
    let uncarried = |subject: &str| {
        report
            .axioms
            .iter()
            .find(|axiom| axiom.provenance.subject == subject)
            .map(|axiom| {
                axiom
                    .components
                    .iter()
                    .map(|component| (component.outcome, component.reason.clone()))
                    .collect::<Vec<_>>()
            })
            .expect("axiom present")
    };
    let gci = uncarried(&format!("some({},{})", p("serial"), xsd("string")));
    assert_eq!(gci.len(), 1);
    assert_eq!(gci[0].0, Unrepresented);
    assert!(gci[0].1.contains("general class inclusion"));
    let parent = uncarried(&iri("Parent"));
    assert_eq!(parent.len(), 1);
    assert!(parent[0].1.contains("sufficient-condition direction"));
    let union_gci = report
        .axioms
        .iter()
        .find(|axiom| axiom.provenance.subject.starts_with("union("))
        .expect("union general class inclusion");
    assert!(
        union_gci.components.is_empty(),
        "both union members carry it"
    );
    assert_eq!(
        union_gci
            .classes
            .iter()
            .map(|row| row.class_iri.clone())
            .collect::<Vec<_>>(),
        vec![iri("Organization"), iri("Robot")]
    );

    // Never silently dropped: every axiom reports at least one component.
    for axiom in &report.axioms {
        assert!(
            !axiom.components.is_empty()
                || axiom.classes.iter().any(|row| !row.components.is_empty()),
            "{:?}",
            axiom.provenance
        );
    }
}

#[test]
fn relation_carries_restriction_provenance_and_precision() {
    let (compilation, _) = complete();
    let property = |local: &str| {
        compilation
            .coverage
            .properties
            .iter()
            .find(|property| property.property_iri == iri(local))
            .expect("catalogued property")
            .clone()
    };
    let row = |local: &str, class: &str| {
        property(local)
            .classes
            .into_iter()
            .find(|row| row.class_iri == iri(class))
            .expect("class row")
    };
    // A property named only by a restriction is catalogued from it.
    assert!(
        property("self")
            .declarations
            .iter()
            .any(|declaration| declaration == "http://www.w3.org/2002/07/owl#onProperty")
    );
    let name = row("name", "Person");
    assert_eq!(name.status, SchemaCoverageStatus::IncludedUnshaped);
    assert_eq!(
        name.precision,
        SchemaCoveragePrecision::RepresentationApproximation
    );
    assert!(name.provenance.iter().any(|provenance| {
        provenance.subject == iri("Person")
            && provenance.object
                == format!("some(<{EX}name>,<http://www.w3.org/2001/XMLSchema#string>)")
    }));
    assert_eq!(
        row("name", "Agent").precision,
        SchemaCoveragePrecision::Exact,
        "the superclass carries no restriction"
    );
    // A domain reached through a union equivalence: each member is a subclass.
    assert_eq!(
        row("member", "Person").status,
        SchemaCoverageStatus::IncludedUnshaped
    );
    assert_eq!(
        row("member", "Agent").status,
        SchemaCoverageStatus::ExcludedDomain
    );
}

/// Whether the projected node `subject` of `data` validates against
/// `#/$defs/{class}` of the compiled schema.
fn accepts(schema_json: &str, class: &str, data: &str, subject: &str) -> bool {
    let data = parse(data);
    let projected = purrdf_shapes::instance::project_graph(&data, &namespaces());
    let node = projected["@graph"]
        .as_array()
        .expect("@graph")
        .iter()
        .find(|node| node["@id"] == iri(subject).as_str())
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

const ALICE: &str = r#"
    ex:alice a ex:Person ; ex:name "Alice" ; ex:email "alice@example.org" ; ex:phone "555" ;
        ex:birthDate "2000-01-01"^^xsd:date ; ex:score 50 ; ex:status ex:active ;
        ex:nickname "Al" , "Bob" ; ex:code "c-1" ; ex:rating 70 .
"#;

#[test]
fn json_schema_projection_judges_projected_instances() {
    let (compilation, _) = complete();
    let schema = &compilation.compiled.schema_json;
    assert!(
        accepts(schema, "Person", ALICE, "alice"),
        "the conforming person"
    );
    let variant = |from: &str, to: &str| {
        assert!(ALICE.contains(from), "{from}");
        ALICE.replace(from, to)
    };
    for (data, why) in [
        (
            variant("ex:name \"Alice\" ;", ""),
            "someValuesFrom requires a name",
        ),
        (
            variant("ex:status ex:active", "ex:status ex:inactive"),
            "hasValue requires ex:active among the values",
        ),
        (
            variant("\"Al\" , \"Bob\"", "\"Al\" , \"Bob\" , \"Cy\" , \"Di\""),
            "maxCardinality 3 bounds the nickname count",
        ),
        (
            variant("\"Al\" , \"Bob\"", "\"Rob\" , \"Bob\""),
            "maxQualifiedCardinality 1 over the literal enumeration",
        ),
        (
            variant("alice@example.org", "not-an-address"),
            "the pattern facet of an allValuesFrom data range",
        ),
        (
            variant("alice@example.org", "x@y@z"),
            "the pattern facet is anchored, as XSD patterns are",
        ),
        (
            variant(
                "\"2000-01-01\"^^xsd:date",
                "\"2000-01-01\"^^xsd:date , \"2001-01-01\"^^xsd:date",
            ),
            "owl:cardinality 1 admits one value",
        ),
        (
            variant("ex:score 50", "ex:score 150"),
            "the facet-bounded qualifier",
        ),
        (
            variant("ex:code \"c-1\"", "ex:code 7"),
            "datatypeComplementOf xsd:integer",
        ),
        (
            variant("ex:rating 70", "ex:rating 170"),
            "a range named by a datatype definition is held to the definition",
        ),
        (
            variant("a ex:Person ;", "a ex:Person , ex:Robot ;"),
            "complementOf ex:Robot excludes the type",
        ),
    ] {
        assert!(!accepts(schema, "Person", &data, "alice"), "{why}");
    }
    assert!(
        accepts(
            schema,
            "Person",
            &variant("ex:status ex:active", "ex:status ex:active , ex:inactive"),
            "alice"
        ),
        "hasValue is existential: other values may accompany it"
    );
    assert!(
        accepts(
            schema,
            "Person",
            &variant("\"Al\" , \"Bob\"", "\"Al\" , \"Cy\" , \"Bob\""),
            "alice"
        ),
        "three nicknames, one of them enumerated"
    );

    // The disjunction of two minimums.
    assert!(!accepts(schema, "Contact", "ex:c a ex:Contact .", "c"));
    assert!(accepts(
        schema,
        "Contact",
        "ex:c a ex:Contact ; ex:phone \"1\" .",
        "c"
    ));
    assert!(accepts(
        schema,
        "Contact",
        "ex:c a ex:Contact ; ex:email \"c@d\" .",
        "c"
    ));

    // The enumeration of a class's individuals.
    assert!(accepts(
        schema,
        "Status",
        "ex:active a ex:Status .",
        "active"
    ));
    assert!(!accepts(
        schema,
        "Status",
        "ex:paused a ex:Status .",
        "paused"
    ));

    // A maximum of zero, and a general inclusion carried by a union member.
    assert!(accepts(
        schema,
        "Robot",
        "ex:r a ex:Robot ; ex:serial \"R2\" .",
        "r"
    ));
    assert!(!accepts(
        schema,
        "Robot",
        "ex:r a ex:Robot ; ex:serial \"R2\" ; ex:name \"Artoo\" .",
        "r"
    ));
    assert!(!accepts(schema, "Robot", "ex:r a ex:Robot .", "r"));
}

#[test]
fn unrepresented_components_are_named_on_the_class_definition() {
    let (compilation, _) = complete();
    let schema: Value = purrdf_lex::json::read(&compilation.compiled.schema_json).expect("JSON");
    let comment = schema["$defs"]["Person"]["$comment"]
        .as_str()
        .expect("Person carries the unrepresented components");
    for expression in [
        format!("has_self(<{EX}self>)"),
        format!("max(2,<{EX}parent>,<{EX}Person>)"),
        format!("some(inverse(<{EX}knows>),<{EX}Person>)"),
    ] {
        assert!(comment.contains(&expression), "{expression} in {comment}");
    }
    assert!(schema["$defs"]["Agent"].get("$comment").is_none());
}

#[test]
fn every_language_emitter_carries_the_projection_or_records_its_loss() {
    let (compilation, _) = complete();
    let compiled = &compilation.compiled;

    // LinkML: an existential or minimum is a required attribute; an
    // unrestricted field stays optional.
    let linkml = emit_linkml(compiled, &linkml_config()).expect("LinkML emission");
    let classes = &linkml.document.as_value()["classes"];
    assert_eq!(
        classes["Robot"]["attributes"]["ex:serial"]["required"],
        true
    );
    assert_eq!(
        classes["Person"]["attributes"]["ex:email"]["required"],
        true
    );
    assert_eq!(
        classes["Person"]["attributes"]["ex:birthDate"]["required"],
        true
    );
    assert_ne!(classes["Agent"]["attributes"]["ex:email"]["required"], true);

    // TypeScript: required fields lose their optional marker.
    let typescript = emit_typescript(compiled, &typescript_config()).expect("TypeScript emission");
    let declarations = std::str::from_utf8(&typescript.artifacts[TYPESCRIPT_DECLARATION_PATH])
        .expect("UTF-8 TypeScript");
    let person_type = &typescript.type_names["Person"];
    let person = generated_definition_block(
        declarations,
        &format!("export type {person_type} = "),
        "\nexport type ",
    );
    for required in ["ex:name", "ex:email", "ex:birthDate", "ex:status"] {
        assert!(
            person.contains(&format!("readonly \"{required}\":")),
            "{required} is required in TypeScript"
        );
    }
    assert!(person.contains("readonly \"ex:child\"?:"));
    // What TypeScript cannot type (an existential `contains`) is recorded.
    assert!(typescript.losses.entries().iter().any(|entry| {
        entry.code == "array-contains-validation-dropped"
            && format!("{:?}", entry.location).contains("#/$defs/Person")
    }));

    // GraphQL: the enumerated class stays typed; a disjunction over several
    // properties is delegated to the custom scalar, and that is recorded.
    let graphql = emit_graphql(compiled, &graphql_config()).expect("GraphQL emission");
    assert!(graphql.names.fields.contains_key("#/$defs/Status"));
    assert!(graphql.losses.entries().iter().any(|entry| {
        entry.code == "intersection-validation-delegated"
            && format!("{:?}", entry.location).contains("#/$defs/Contact")
    }));

    // Pydantic: a required field has no default; an optional one does.
    let pydantic = emit_pydantic(compiled, &pydantic_config()).expect("Pydantic emission");
    let models = std::str::from_utf8(&pydantic.artifacts["example_ontology/models.py"])
        .expect("UTF-8 Pydantic models");
    let field = |class: &str, alias: &str| {
        let model = pydantic.model_paths[class]
            .rsplit('.')
            .next()
            .expect("generated model name");
        generated_definition_block(models, &format!("class {model}("), "\nclass ")
            .lines()
            .find(|line| line.contains(&format!("alias=\"{alias}\"")))
            .unwrap_or_else(|| panic!("{class}.{alias} field"))
            .to_owned()
    };
    let required = field("Robot", "ex:serial");
    assert!(!required.contains("default="), "{required}");
    let optional = field("Agent", "ex:serial");
    assert!(optional.contains("default="), "{optional}");
}

#[test]
fn shaped_only_mode_excludes_every_component() {
    let (compilation, report) = compile_both("", ONTOLOGY, SchemaSurfaceMode::ShapedOnly);
    assert_ne!(report.axioms.len(), 0);
    for axiom in &report.axioms {
        for row in &axiom.classes {
            for component in &row.components {
                assert_eq!(component.outcome, SchemaExpressionOutcome::Excluded);
            }
        }
    }
    assert!(!compilation.compiled.schema_json.contains("ex:Robot"));
}

#[test]
fn direct_shacl_shapes_stay_authoritative_over_restrictions() {
    let (compilation, report) = compile_both(
        "ex:PersonShape a sh:NodeShape ; sh:targetClass ex:Person ;
            sh:property [ sh:path ex:name ; sh:datatype xsd:string ] .",
        ONTOLOGY,
        SchemaSurfaceMode::OntologyComplete,
    );
    let name = format!("some(<{EX}name>,<http://www.w3.org/2001/XMLSchema#string>)");
    assert_eq!(
        outcomes(&report, "Person", &name),
        vec![SchemaExpressionOutcome::Excluded]
    );
    let schema: Value = purrdf_lex::json::read(&compilation.compiled.schema_json).expect("JSON");
    let required = schema["$defs"]["Person"]["required"]
        .as_array()
        .expect("required list")
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert!(
        !required.iter().any(|key| key == "ex:name"),
        "SHACL decides ex:name"
    );
    assert!(
        required.iter().any(|key| key == "ex:email"),
        "the ontology still decides ex:email"
    );
}

/// `A ⊑ ∃p.xsd:integer ⊔ ∃q.xsd:string`, with subclasses `B` and `C`.
const INHERITED_DISJUNCTION: &str = "
    ex:A a owl:Class ; rdfs:subClassOf [ owl:unionOf (
        [ a owl:Restriction ; owl:onProperty ex:p ; owl:someValuesFrom xsd:integer ]
        [ a owl:Restriction ; owl:onProperty ex:q ; owl:someValuesFrom xsd:string ] ) ] .
    ex:B a owl:Class ; rdfs:subClassOf ex:A .
    ex:C a owl:Class ; rdfs:subClassOf ex:A .
    ex:p a owl:DatatypeProperty .
    ex:q a owl:DatatypeProperty .
";

#[test]
fn a_subclass_shape_is_not_overridden_by_an_inherited_disjunction() {
    let disjunction = format!(
        "union(some(<{EX}p>,<http://www.w3.org/2001/XMLSchema#integer>),\
         some(<{EX}q>,<http://www.w3.org/2001/XMLSchema#string>))"
    );
    for shape in [
        "ex:BShape a sh:NodeShape ; sh:targetClass ex:B ;
            sh:property [ sh:path ex:p ; sh:datatype xsd:string ] .",
        "ex:BShape a sh:NodeShape ; sh:targetClass ex:B ; sh:closed true ;
            sh:ignoredProperties ( rdf:type ) ;
            sh:property [ sh:path ex:p ; sh:datatype xsd:string ] .",
    ] {
        let (compilation, report) = compile_both(
            shape,
            INHERITED_DISJUNCTION,
            SchemaSurfaceMode::OntologyComplete,
        );
        let schema = &compilation.compiled.schema_json;
        let on_b = outcomes(&report, "B", &disjunction);
        assert!(
            !on_b.is_empty()
                && on_b
                    .iter()
                    .all(|outcome| *outcome == SchemaExpressionOutcome::Unrepresented),
            "B's shape owns ex:p, so the disjunction is not represented on B: {on_b:?}\n{}",
            report.to_json()
        );
        // B data that conforms to B's shape, and so to B's schema, though it
        // meets neither disjunct.
        assert!(
            accepts(schema, "B", "ex:b a ex:B ; ex:p \"text\" .", "b"),
            "the SHACL shape decides B's ex:p: {schema}"
        );
        assert!(
            !accepts(schema, "B", "ex:b a ex:B ; ex:p 5 .", "b"),
            "B's shape still holds"
        );
        // Neighbours: A, and C, which has no shape, still state the
        // disjunction.
        for class in ["A", "C"] {
            let subject = class.to_lowercase();
            assert!(
                !accepts(
                    schema,
                    class,
                    &format!("ex:{subject} a ex:{class} ; ex:p \"text\" ."),
                    &subject
                ),
                "{class} meets neither disjunct"
            );
            assert!(
                accepts(
                    schema,
                    class,
                    &format!("ex:{subject} a ex:{class} ; ex:p 5 ."),
                    &subject
                ),
                "{class} meets the first disjunct"
            );
        }
    }
}

#[test]
fn a_subclass_projects_an_inherited_disjunction_its_owner_cannot() {
    // A's shape owns ex:p, so A cannot state the disjunction; B, which has no
    // shape, can, and reports it so: its schema must state it.
    let (compilation, report) = compile_both(
        "ex:AShape a sh:NodeShape ; sh:targetClass ex:A ;
            sh:property [ sh:path ex:p ; sh:datatype xsd:integer ] .",
        INHERITED_DISJUNCTION,
        SchemaSurfaceMode::OntologyComplete,
    );
    let disjunction = format!(
        "union(some(<{EX}p>,<http://www.w3.org/2001/XMLSchema#integer>),\
         some(<{EX}q>,<http://www.w3.org/2001/XMLSchema#string>))"
    );
    let schema = &compilation.compiled.schema_json;
    let on_b = outcomes(&report, "B", &disjunction);
    assert_eq!(
        outcomes(&report, "A", &disjunction),
        vec![SchemaExpressionOutcome::Unrepresented],
        "A's shape owns ex:p"
    );
    assert_eq!(
        on_b,
        vec![SchemaExpressionOutcome::Approximated],
        "B states the disjunction"
    );
    assert!(
        !accepts(schema, "B", "ex:b a ex:B ; ex:q 5 .", "b"),
        "B reports the disjunction approximated, so its schema states it: {schema}"
    );
    assert!(accepts(schema, "B", "ex:b a ex:B ; ex:q \"x\" .", "b"));
    assert!(accepts(schema, "A", "ex:a a ex:A ; ex:q 5 .", "a"));
}

#[test]
fn iri_only_ontologies_report_no_class_expressions() {
    let (compilation, report) = compile_both("", IRI_ONLY, SchemaSurfaceMode::OntologyComplete);
    assert_eq!(report.axioms.len(), 0);
    let shapes = shapes("");
    let ontology = parse(IRI_ONLY);
    let namespaces = namespaces();
    let plain = compile_schema(&SchemaCompileRequest::new(
        &shapes,
        &namespaces,
        ontology.as_ref(),
        SchemaSurfaceMode::OntologyComplete,
    ))
    .expect("IRI-only compilation");
    assert_eq!(plain.compiled.schema_json, compilation.compiled.schema_json);
    assert_eq!(plain.coverage.to_json(), compilation.coverage.to_json());
    assert!(
        !compilation
            .compiled
            .schema_json
            .contains("OWL restrictions")
    );
}

#[test]
fn compilation_manifest_and_emitters_are_deterministic() {
    let (first, first_report) = complete();
    let (second, second_report) = complete();
    assert_eq!(first.compiled.schema_json, second.compiled.schema_json);
    assert_eq!(first.compiled.openapi_json, second.compiled.openapi_json);
    assert_eq!(first.coverage.to_json(), second.coverage.to_json());
    assert_eq!(first_report.to_json(), second_report.to_json());
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

    // Reversing the statement order changes every blank-node label and the
    // order the store holds triples in, and nothing emitted.
    let mut statements: Vec<&str> = ONTOLOGY
        .split(" .\n")
        .filter(|statement| !statement.trim().is_empty())
        .collect();
    statements.reverse();
    let reversed = statements.join(" .\n") + " .\n";
    let (permuted, permuted_report) =
        compile_both("", &reversed, SchemaSurfaceMode::OntologyComplete);
    assert_eq!(first.compiled.schema_json, permuted.compiled.schema_json);
    assert_eq!(first.coverage.to_json(), permuted.coverage.to_json());
    assert_eq!(first_report.to_json(), permuted_report.to_json());
}

#[test]
fn universal_restriction_to_owl_nothing_forbids_the_property() {
    let ontology = "ex:A a owl:Class ; rdfs:subClassOf
            [ a owl:Restriction ; owl:onProperty ex:p ; owl:allValuesFrom owl:Nothing ] .
        ex:p a owl:ObjectProperty .";
    let (compilation, report) = compile_both("", ontology, SchemaSurfaceMode::OntologyComplete);
    let schema = &compilation.compiled.schema_json;
    assert!(!accepts(schema, "A", "ex:a a ex:A ; ex:p ex:b .", "a"));
    assert!(accepts(schema, "A", "ex:a a ex:A .", "a"));
    assert_eq!(
        outcomes(
            &report,
            "A",
            &format!("all(<{EX}p>,<http://www.w3.org/2002/07/owl#Nothing>)")
        ),
        vec![SchemaExpressionOutcome::Projected]
    );
}

#[test]
fn a_restricted_property_is_judged_on_its_class_outside_its_domain() {
    let ontology = "ex:Elsewhere a owl:Class .
        ex:Z a owl:Class ; rdfs:subClassOf
            [ a owl:Restriction ; owl:onProperty ex:q ; owl:allValuesFrom xsd:integer ] ,
            [ a owl:Restriction ; owl:onProperty ex:q ; owl:maxCardinality 1 ] .
        ex:q a owl:DatatypeProperty ; rdfs:domain ex:Elsewhere .";
    let (compilation, _) = compile_both("", ontology, SchemaSurfaceMode::OntologyComplete);
    let schema = &compilation.compiled.schema_json;
    assert!(!accepts(
        schema,
        "Z",
        "ex:z a ex:Z ; ex:q \"abc\" , \"def\" .",
        "z"
    ));
    assert!(!accepts(schema, "Z", "ex:z a ex:Z ; ex:q \"abc\" .", "z"));
    assert!(accepts(schema, "Z", "ex:z a ex:Z ; ex:q 5 .", "z"));
}

/// `classes` classes, each restricted by an existential on each of four
/// shared, domainless object properties.
fn shared_restriction_ontology(classes: usize) -> String {
    use std::fmt::Write as _;
    let mut ontology = String::from("ex:Target a owl:Class .\n");
    for property in 0..4 {
        let _ = writeln!(ontology, "ex:p{property} a owl:ObjectProperty .");
    }
    for class in 0..classes {
        let _ = write!(
            ontology,
            "ex:C{class} a owl:Class ; rdfs:subClassOf ex:Target"
        );
        for property in 0..4 {
            let _ = write!(
                ontology,
                " , [ a owl:Restriction ; owl:onProperty ex:p{property} ; owl:someValuesFrom ex:Target ]"
            );
        }
        ontology.push_str(" .\n");
    }
    ontology
}

#[test]
fn thousands_of_restricted_classes_on_shared_properties_scale_linearly() {
    let small = compile_both(
        "",
        &shared_restriction_ontology(250),
        SchemaSurfaceMode::OntologyComplete,
    );
    let large = compile_both(
        "",
        &shared_restriction_ontology(2_000),
        SchemaSurfaceMode::OntologyComplete,
    );
    let ratio = |measure: fn(
        &(
            purrdf_shapes::SchemaCompilation,
            SchemaClassExpressionReport,
        ),
    ) -> usize| { measure(&large) as f64 / measure(&small) as f64 };
    // Eight times the classes: linear growth is about 8, quadratic 64.
    for (name, growth) in [
        ("schema", ratio(|run| run.0.compiled.schema_json.len())),
        ("coverage", ratio(|run| run.0.coverage.to_json().len())),
        ("manifest", ratio(|run| run.1.to_json().len())),
    ] {
        assert!(growth < 10.0, "{name} grew {growth:.1}x for 8x the classes");
    }
}

/// A slice shaped like FOAF and PROV-O, in the example.org namespace:
/// existentials, cardinalities, universals, a named inverse, a union domain,
/// a qualified cardinality, functional properties and disjointness.
const AGENT_PROVENANCE_SLICE: &str = r"
    ex:Agent a owl:Class .
    ex:Person a owl:Class ; rdfs:subClassOf ex:Agent ,
        [ a owl:Restriction ; owl:onProperty ex:name ; owl:someValuesFrom xsd:string ] ,
        [ a owl:Restriction ; owl:onProperty ex:mbox ; owl:maxCardinality 1 ] ;
        owl:disjointWith ex:Organization , [ owl:complementOf ex:Agent ] .
    ex:Organization a owl:Class ; rdfs:subClassOf ex:Agent .
    ex:Group a owl:Class ; rdfs:subClassOf ex:Agent ,
        [ a owl:Restriction ; owl:onProperty ex:member ; owl:allValuesFrom ex:Agent ] .
    ex:Document a owl:Class ;
        rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:maker ; owl:minCardinality 1 ] .
    ex:made a owl:ObjectProperty ; owl:inverseOf ex:maker ; rdfs:domain ex:Agent .
    ex:maker a owl:ObjectProperty ; rdfs:range ex:Agent .
    ex:name a owl:DatatypeProperty ; rdfs:domain ex:Agent ; rdfs:range xsd:string .
    ex:mbox a owl:ObjectProperty ; rdfs:domain ex:Agent .
    ex:member a owl:ObjectProperty ; rdfs:domain ex:Group .

    ex:Entity a owl:Class ; owl:disjointWith ex:Activity .
    ex:Activity a owl:Class ;
        rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:startedAtTime ;
            owl:maxCardinality 1 ] .
    ex:wasGeneratedBy a owl:ObjectProperty ; rdfs:domain ex:Entity ; rdfs:range ex:Activity .
    ex:generated a owl:ObjectProperty ; owl:inverseOf ex:wasGeneratedBy .
    ex:startedAtTime a owl:DatatypeProperty , owl:FunctionalProperty ;
        rdfs:domain ex:Activity ; rdfs:range xsd:dateTime .
    ex:atTime a owl:DatatypeProperty ; rdfs:range xsd:dateTime ;
        rdfs:domain [ owl:unionOf ( ex:Entity ex:Activity ) ] .
    ex:Bundle a owl:Class ; rdfs:subClassOf ex:Entity .
    ex:Plan a owl:Class ; rdfs:subClassOf ex:Entity ,
        [ a owl:Restriction ; owl:onProperty [ owl:inverseOf ex:generated ] ;
          owl:someValuesFrom ex:Activity ] .
    ex:Derivation a owl:Class ; rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:entity ;
        owl:qualifiedCardinality 1 ; owl:onClass ex:Entity ] .
    ex:entity a owl:ObjectProperty .
    ex:Person rdfs:subClassOf [ a owl:Restriction ;
        owl:onProperty <https://external.example/vocab/nick> ; owl:minCardinality 1 ] .
    owl:Thing rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:homepage ;
        owl:allValuesFrom xsd:anyURI ] .
    ex:homepage a owl:DatatypeProperty .
";

/// A schema's compact text followed by that of every definition it reaches
/// through `$ref`, so an inherited restriction a class references is read
/// where it lives.
fn with_references(schema: &Value, document: &Value) -> String {
    let marker = "\"$ref\":\"#/$defs/";
    let mut text = String::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut pending = vec![schema.clone()];
    while let Some(next) = pending.pop() {
        let compact = purrdf_lex::json::write_compact(&next);
        for reference in compact.split(marker).skip(1) {
            let key = reference
                .split('"')
                .next()
                .expect("reference key")
                .to_owned();
            if seen.insert(key.clone()) {
                pending.push(document["$defs"][key.as_str()].clone());
            }
        }
        text.push_str(&compact);
    }
    text
}

/// Whether a manifest expression requires a value of its property.
fn requires_value(expression: &str) -> bool {
    expression.starts_with("some(")
        || expression.starts_with("has_value(")
        || ((expression.starts_with("min(") || expression.starts_with("exact("))
            && !expression.starts_with("min(0,")
            && !expression.starts_with("exact(0,"))
}

#[test]
fn reported_outcomes_match_every_emitter_on_an_agent_provenance_slice() {
    let (compilation, report) = compile_both(
        "",
        AGENT_PROVENANCE_SLICE,
        SchemaSurfaceMode::OntologyComplete,
    );
    let compiled = &compilation.compiled;
    let schema: Value = purrdf_lex::json::read(&compiled.schema_json).expect("schema JSON");
    let linkml = emit_linkml(compiled, &linkml_config()).expect("LinkML emission");
    let linkml_classes = &linkml.document.as_value()["classes"];
    let typescript = emit_typescript(compiled, &typescript_config()).expect("TypeScript emission");
    let declarations = std::str::from_utf8(&typescript.artifacts[TYPESCRIPT_DECLARATION_PATH])
        .expect("UTF-8 TypeScript");
    let graphql = emit_graphql(compiled, &graphql_config()).expect("GraphQL emission");
    let pydantic = emit_pydantic(compiled, &pydantic_config()).expect("Pydantic emission");
    let models = std::str::from_utf8(&pydantic.artifacts["example_ontology/models.py"])
        .expect("UTF-8 Pydantic models");

    let mut checked = 0_usize;
    for axiom in &report.axioms {
        for row in &axiom.classes {
            let class = row.class_iri.strip_prefix(EX).expect("example.org class");
            let definition = &schema["$defs"][class];
            assert!(definition.as_object().is_some(), "{class} has a definition");
            // GraphQL types the class, or delegates it to its scalar and says so.
            assert!(
                graphql
                    .names
                    .fields
                    .contains_key(&format!("#/$defs/{class}"))
                    || graphql.losses.entries().iter().any(|entry| {
                        entry.code == "custom-scalar-validation-delegated"
                            && format!("{:?}", entry.location).contains(&format!("#/$defs/{class}"))
                    }),
                "GraphQL neither types nor reports {class}"
            );
            for component in &row.components {
                let Some(property) = &component.property_iri else {
                    continue;
                };
                let key = property
                    .strip_prefix(EX)
                    .map_or_else(|| property.clone(), |local| format!("ex:{local}"));
                let emitted = definition["properties"].get(&key).is_some();
                let required = definition["required"]
                    .as_array()
                    .is_some_and(|keys| keys.iter().any(|item| item.as_str() == Some(&key)));
                match component.outcome {
                    SchemaExpressionOutcome::Projected | SchemaExpressionOutcome::Approximated => {
                        assert!(emitted, "{class}.{key}: {}", component.expression);
                        let property_schema =
                            with_references(&definition["properties"][key.as_str()], &schema);
                        if component.expression.starts_with("max(")
                            && !component.expression.contains(",<")
                        {
                            assert!(
                                property_schema.contains("\"maxItems\"")
                                    || !property_schema.contains("\"array\""),
                                "{class}.{key} bounds its count: {property_schema}"
                            );
                        }
                        if requires_value(&component.expression) {
                            assert!(required, "{class}.{key} is required in JSON Schema");
                            assert_eq!(
                                linkml_classes[class]["attributes"][key.as_str()]["required"],
                                true,
                                "{class}.{key} is required in LinkML"
                            );
                            let type_name = &typescript.type_names[class];
                            let declaration = generated_definition_block(
                                declarations,
                                &format!("export type {type_name} = "),
                                "\nexport type ",
                            );
                            assert!(
                                declaration.contains(&format!("readonly \"{key}\":")),
                                "{class}.{key} is required in TypeScript"
                            );
                            let model = pydantic.model_paths[class]
                                .rsplit('.')
                                .next()
                                .expect("model name");
                            let field = generated_definition_block(
                                models,
                                &format!("class {model}("),
                                "\nclass ",
                            )
                            .lines()
                            .find(|line| line.contains(&format!("alias=\"{key}\"")))
                            .unwrap_or_else(|| panic!("{class}.{key} Pydantic field"))
                            .to_owned();
                            assert!(!field.contains("default="), "{class}.{key}: {field}");
                        }
                        checked += 1;
                    }
                    SchemaExpressionOutcome::Excluded => {
                        if !component.reason.contains("SHACL") {
                            assert!(!emitted, "{class}.{key} is excluded but emitted");
                        }
                        checked += 1;
                    }
                    SchemaExpressionOutcome::Unrepresented => {
                        let other_requires = row.components.iter().any(|other| {
                            other.property_iri.as_deref() == Some(property.as_str())
                                && other.outcome != SchemaExpressionOutcome::Unrepresented
                                && requires_value(&other.expression)
                        });
                        if !other_requires && !requires_value(&component.expression) {
                            assert!(!required, "{class}.{key}: nothing projected requires it");
                        }
                        checked += 1;
                    }
                    _ => {}
                }
            }
        }
    }
    assert!(checked >= 9, "the slice exercises the outcomes ({checked})");
    // The global range from owl:Thing reaches every class's homepage field.
    assert!(
        purrdf_lex::json::write_compact(&schema["$defs"]["Document"]["properties"]["ex:homepage"])
            .contains("xsd:anyURI")
    );
    // The slice's named inverse resolves: Plan's restriction is on
    // wasGeneratedBy, which the class then requires.
    assert!(
        schema["$defs"]["Plan"]["required"]
            .as_array()
            .expect("Plan requires")
            .iter()
            .any(|key| key.as_str() == Some("ex:wasGeneratedBy"))
    );
}

/// A class tree shaped like the Gene Ontology: `classes` classes in a binary
/// heap (depth about log2 of the count), each restricted by two existentials
/// on shared properties.
fn ontology_tree(classes: usize) -> String {
    use std::fmt::Write as _;
    let mut ontology =
        String::from("ex:partOf a owl:ObjectProperty .\nex:regulates a owl:ObjectProperty .\n");
    for class in 0..classes {
        let _ = write!(ontology, "ex:G{class} a owl:Class");
        if class > 0 {
            let parent = (class - 1) / 2;
            let _ = write!(ontology, " ; rdfs:subClassOf ex:G{parent}");
        }
        let target = class / 3;
        let _ = writeln!(
            ontology,
            " ; rdfs:subClassOf \
             [ a owl:Restriction ; owl:onProperty ex:partOf ; owl:someValuesFrom ex:G{target} ] , \
             [ a owl:Restriction ; owl:onProperty ex:regulates ; owl:someValuesFrom ex:G{target} ] ."
        );
    }
    ontology
}

#[test]
fn a_gene_ontology_sized_tree_references_inherited_restrictions() {
    let measure = |classes: usize| {
        let (compilation, report) = compile_both(
            "",
            &ontology_tree(classes),
            SchemaSurfaceMode::OntologyComplete,
        );
        let bytes = compilation.compiled.schema_json.len()
            + compilation.coverage.to_json().len()
            + report.to_json().len();
        bytes as f64 / classes as f64
    };
    // Depth 9 against depth 15: copying inherited restrictions inline grows
    // the bytes per class with the depth; referencing them does not.
    let shallow = measure(1_023);
    let deep = measure(50_000);
    assert!(
        deep < shallow * 1.25,
        "bytes per class grew from {shallow:.0} to {deep:.0} with the depth"
    );
}

#[test]
fn graphql_names_literal_enumerations_beside_their_array_form() {
    for ontology in [
        "ex:A a owl:Class . ex:e a owl:DatatypeProperty ;
            rdfs:range [ a rdfs:Datatype ; owl:oneOf ( \"x\" \"y\" ) ] .",
        "ex:A a owl:Class ; rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:e ;
            owl:allValuesFrom [ a rdfs:Datatype ; owl:oneOf ( \"x\" \"y\" ) ] ] .
         ex:e a owl:DatatypeProperty .",
    ] {
        let (compilation, _) = compile_both("", ontology, SchemaSurfaceMode::OntologyComplete);
        let graphql = emit_graphql(&compilation.compiled, &graphql_config())
            .expect("a literal enumeration is a GraphQL enum");
        let sdl = std::str::from_utf8(&graphql.artifacts[GRAPHQL_SCHEMA_PATH]).expect("UTF-8");
        assert!(sdl.contains("enum "), "{sdl}");
    }
    // The same enumeration through SHACL sh:in.
    let (compilation, _) = compile_both(
        "ex:AShape a sh:NodeShape ; sh:targetClass ex:A ;
            sh:property [ sh:path ex:e ; sh:in ( \"x\" \"y\" ) ] .",
        "",
        SchemaSurfaceMode::ShapedOnly,
    );
    let graphql = emit_graphql(&compilation.compiled, &graphql_config())
        .expect("an sh:in enumeration is a GraphQL enum");
    let sdl = std::str::from_utf8(&graphql.artifacts[GRAPHQL_SCHEMA_PATH]).expect("UTF-8");
    assert!(sdl.contains("enum "), "{sdl}");
}
