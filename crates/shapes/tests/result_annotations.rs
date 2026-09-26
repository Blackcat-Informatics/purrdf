// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHACL-SPARQL RESULT ANNOTATIONS, end to end, and the SPARQL-executable terms
//! the loader refuses beside their valid neighbours.
//!
//! SHACL 1.2 SPARQL Extensions, "Annotation Properties": "For each solution of a
//! SELECT result set, a SHACL processor that supports annotations walks through the
//! declared result annotations. … If a variable name could be determined, then the
//! SHACL processor copies the binding for the given variable as a value for the
//! property specified using sh:annotationProperty into the validation result that is
//! being produced for the current solution. If the variable has no binding in the
//! result set solution, then the values of sh:annotationValue are used, if present."
//!
//! Each evaluation test observes the annotation on the produced result AND in the
//! report graph, beside a control row (the same constraint without the annotation)
//! that carries none — so an annotation silently dropped cannot pass.

use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_shapes::engine::{PreparedShapes, parse_shapes, validate_dataset_with_shapes_graph};
use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};
use purrdf_shapes::report::ValidationReport;
use purrdf_shapes::shapes::Shapes;
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const PREFIXES: &str = "
@prefix ex:    <http://example.org/ns#> .
@prefix rdf:   <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix sh:    <http://www.w3.org/ns/shacl#> .
@prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .
";

const DATA: &str = "ex:a ex:p ex:b . ex:b ex:tag \"urgent\" .";

fn load(shapes_ttl: &str) -> Result<Shapes, String> {
    parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None).map_err(String::from)
}

#[track_caller]
fn refused(shapes_ttl: &str, needle: &str) {
    let error = load(shapes_ttl).expect_err("the shapes graph must be refused at load");
    assert!(
        error.contains(needle),
        "refusal must mention {needle:?}: {error}"
    );
}

#[track_caller]
fn loads(shapes_ttl: &str) -> Shapes {
    load(shapes_ttl).unwrap_or_else(|error| panic!("the valid neighbour must load: {error}"))
}

fn data() -> Arc<RdfDataset> {
    parse_turtle_to_dataset(&format!("{PREFIXES}{DATA}"), None).expect("data parses")
}

#[track_caller]
fn validate(shapes_ttl: &str) -> ValidationReport {
    validate_dataset_with_shapes_graph(&data(), &loads(shapes_ttl), None).expect("validation runs")
}

/// Every result's annotations as `(property, value)` N-Triples strings.
fn annotations(report: &ValidationReport) -> Vec<Vec<(String, String)>> {
    report
        .results
        .iter()
        .map(|result| {
            result
                .annotations
                .iter()
                .map(|(property, value)| (format!("<{}>", property.as_str()), value.to_string()))
                .collect()
        })
        .collect()
}

fn ex(local: &str) -> String {
    format!("<http://example.org/ns#{local}>")
}

/// A SPARQL-based constraint whose `?tag` is copied into each result under
/// `ex:tag`, plus its control without the annotation.
fn sparql_constraint(annotation: &str) -> String {
    format!(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:sparql [ {annotation}
             sh:select \"SELECT $this ?value ?tag WHERE {{ $this <http://example.org/ns#p> ?value . OPTIONAL {{ ?value <http://example.org/ns#tag> ?tag }} }}\" ] ."
    )
}

#[test]
fn a_sparql_constraint_copies_the_bound_variable_and_the_control_carries_nothing() {
    let annotated = validate(&sparql_constraint(
        "sh:resultAnnotation [ a sh:ResultAnnotation ; sh:annotationProperty ex:label ; \
         sh:annotationVarName \"tag\" ] ;",
    ));
    let control = validate(&sparql_constraint(""));
    assert_eq!(
        annotations(&annotated),
        vec![vec![(ex("label"), "\"urgent\"".to_owned())]]
    );
    assert_eq!(annotations(&control), vec![Vec::<(String, String)>::new()]);
    // The report graph carries the triple; the control's does not.
    let nt = annotated.to_ntriples();
    assert!(
        nt.contains(&format!("{} \"urgent\"", ex("label"))),
        "the annotation reaches the report graph: {nt}"
    );
    assert!(!control.to_ntriples().contains(&ex("label")));
}

/// Without `sh:annotationVarName` the local name of the property is the variable:
/// `ex:tag` reads `?tag`.
#[test]
fn the_property_local_name_is_the_default_variable() {
    let report = validate(&sparql_constraint(
        "sh:resultAnnotation [ sh:annotationProperty ex:tag ] ;",
    ));
    assert_eq!(
        annotations(&report),
        vec![vec![(ex("tag"), "\"urgent\"".to_owned())]]
    );
}

/// An unbound variable takes the `sh:annotationValue` defaults; a bound one does
/// not. Two data rows, one of each, observe both branches in one report.
#[test]
fn an_unbound_variable_takes_the_defaults() {
    let shapes = "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:sparql [ sh:resultAnnotation [ sh:annotationProperty ex:label ;
                         sh:annotationVarName \"tag\" ; sh:annotationValue \"none\", ex:Unknown ] ;
             sh:select \"SELECT $this ?value ?tag WHERE { { $this <http://example.org/ns#p> ?value . ?value <http://example.org/ns#tag> ?tag } UNION { BIND (<http://example.org/ns#x> AS ?value) } }\" ] .";
    let report = validate(shapes);
    let mut rows = annotations(&report);
    rows.sort();
    assert_eq!(
        rows,
        vec![
            vec![
                (ex("label"), "\"none\"".to_owned()),
                (ex("label"), ex("Unknown")),
            ],
            vec![(ex("label"), "\"urgent\"".to_owned())],
        ]
    );
}

/// A SELECT validator of a SPARQL-based constraint component: the annotation is
/// declared on the validator node, and a pre-bound parameter is readable.
#[test]
fn a_select_validator_copies_bindings_including_a_parameter() {
    let component = |annotation: &str| {
        format!(
            "ex:TagComponent a sh:ConstraintComponent ;
               sh:parameter [ sh:path ex:marker ] ;
               sh:nodeValidator ex:TagValidator .
             ex:TagValidator a sh:SPARQLSelectValidator ; {annotation}
               sh:select \"SELECT $this ?value WHERE {{ $this <http://example.org/ns#p> ?value }}\" .
             ex:S a sh:NodeShape ; sh:targetNode ex:a ; ex:marker ex:M ."
        )
    };
    let annotated = validate(&component(
        "sh:resultAnnotation [ sh:annotationProperty ex:seenWith ; sh:annotationVarName \"marker\" ], \
         [ sh:annotationProperty ex:offender ; sh:annotationVarName \"value\" ] ;",
    ));
    let control = validate(&component(""));
    assert_eq!(
        annotations(&annotated),
        vec![vec![(ex("offender"), ex("b")), (ex("seenWith"), ex("M"))]]
    );
    assert_eq!(annotations(&control), vec![Vec::<(String, String)>::new()]);
}

/// An ASK validator's solution for a failing value node is `($this, focus)` and
/// `($value, v)`, so an annotation reading `value` carries the value node.
#[test]
fn an_ask_validator_annotation_reads_this_and_value() {
    let component = |annotation: &str| {
        format!(
            "ex:NoTagComponent a sh:ConstraintComponent ;
               sh:parameter [ sh:path ex:forbidden ] ;
               sh:validator ex:NoTagValidator .
             ex:NoTagValidator a sh:SPARQLAskValidator ; {annotation}
               sh:ask \"ASK {{ FILTER NOT EXISTS {{ $value <http://example.org/ns#tag> $forbidden }} }}\" .
             ex:S a sh:NodeShape ; sh:targetNode ex:a ;
               sh:property [ sh:path ex:p ; ex:forbidden \"urgent\" ] ."
        )
    };
    let annotated = validate(&component(
        "sh:resultAnnotation [ sh:annotationProperty ex:culprit ; sh:annotationVarName \"value\" ], \
         [ sh:annotationProperty ex:owner ; sh:annotationVarName \"this\" ] ;",
    ));
    let control = validate(&component(""));
    assert_eq!(
        annotations(&annotated),
        vec![vec![(ex("culprit"), ex("b")), (ex("owner"), ex("a"))]]
    );
    assert_eq!(annotations(&control), vec![Vec::<(String, String)>::new()]);
}

/// A prepared product carries the annotations: the restored preparation reports
/// the same bytes as the freshly parsed one, annotation triple included.
#[test]
fn a_prepared_product_keeps_the_annotations() {
    let shapes_ttl = format!(
        "{PREFIXES}{}",
        sparql_constraint(
            "sh:resultAnnotation [ sh:annotationProperty ex:label ; sh:annotationVarName \"tag\" ; \
             sh:annotationValue \"none\" ] ;",
        )
    );
    let fresh = PreparedShapes::new(Arc::new(parse_shapes(&shapes_ttl, None).expect("parses")));
    let bytes = fresh
        .to_product(&ShapesProfile::CORE)
        .expect("the shapes are representable as a product");
    let restored = ShapesProduct::open(&bytes)
        .expect("the product opens")
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("the product admits");
    let report = |prepared: &PreparedShapes| {
        prepared
            .bind_shared_dataset(data())
            .expect("binds")
            .validate()
            .expect("validates")
            .to_ntriples()
    };
    let restored_nt = report(&restored);
    assert_eq!(report(&fresh), restored_nt);
    assert!(restored_nt.contains(&format!("{} \"urgent\"", ex("label"))));
}

// ── Refusals, each beside its valid neighbour ─────────────────────────────────

#[test]
fn a_var_name_that_is_no_sparql_variable_is_refused_and_a_bare_name_loads() {
    refused(
        &sparql_constraint(
            "sh:resultAnnotation [ sh:annotationProperty ex:label ; sh:annotationVarName \"?tag\" ] ;",
        ),
        "SPARQL variable name",
    );
    loads(&sparql_constraint(
        "sh:resultAnnotation [ sh:annotationProperty ex:label ; sh:annotationVarName \"tag\" ] ;",
    ));
}

#[test]
fn an_annotation_needs_exactly_one_iri_property() {
    refused(
        &sparql_constraint(
            "sh:resultAnnotation [ sh:annotationProperty ex:a1, ex:a2 ; sh:annotationVarName \"tag\" ] ;",
        ),
        "exactly one value for the property sh:annotationProperty",
    );
    refused(
        &sparql_constraint("sh:resultAnnotation [ sh:annotationVarName \"tag\" ] ;"),
        "exactly one value for the property sh:annotationProperty",
    );
    refused(
        &sparql_constraint("sh:resultAnnotation \"tag\" ;"),
        "IRIs or blank nodes",
    );
    loads(&sparql_constraint(
        "sh:resultAnnotation [ sh:annotationProperty ex:a1 ; sh:annotationVarName \"tag\" ] ;",
    ));
}

#[test]
fn a_report_property_as_annotation_is_refused_and_an_application_property_loads() {
    refused(
        &sparql_constraint(
            "sh:resultAnnotation [ sh:annotationProperty sh:focusNode ; sh:annotationVarName \"tag\" ] ;",
        ),
        "validation-report vocabulary",
    );
    loads(&sparql_constraint(
        "sh:resultAnnotation [ sh:annotationProperty ex:focus ; sh:annotationVarName \"tag\" ] ;",
    ));
}

#[test]
fn an_unknown_term_on_an_annotation_node_is_refused() {
    refused(
        &sparql_constraint(
            "sh:resultAnnotation [ sh:annotationProperty ex:label ; sh:annotationVarNmae \"tag\" ] ;",
        ),
        "annotationVarNmae",
    );
}

/// `sh:update` and `sh:describe` have no processing semantics in any SHACL
/// specification: beside a constraint's query, on a shape or on a validator they
/// would be silently ignored, so each is refused; the same constraint without
/// them loads (and is evaluated by the tests above), and a stand-alone
/// `sh:SPARQLUpdateExecutable` resource no shape reads loads.
#[test]
fn describe_and_update_are_refused_wherever_the_loader_reads_and_load_elsewhere() {
    refused(
        &sparql_constraint("sh:update \"DELETE WHERE { ?s ?p ?o }\" ;"),
        "no SHACL specification executes",
    );
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:describe \"DESCRIBE ?x\" .",
        "no SHACL specification executes",
    );
    refused(
        "ex:C a sh:ConstraintComponent ; sh:parameter [ sh:path ex:q ] ; sh:validator ex:V .
         ex:V a sh:SPARQLAskValidator ; sh:ask \"ASK {}\" ; sh:update \"CLEAR ALL\" .",
        "no SHACL specification executes",
    );
    loads(&sparql_constraint(""));
    loads(
        "ex:Cleanup a sh:SPARQLUpdateExecutable ; sh:update \"CLEAR ALL\" .
         ex:Peek a sh:SPARQLDescribeExecutable ; sh:describe \"DESCRIBE <http://example.org/ns#a>\" .
         ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:class ex:Thing .",
    );
}

/// A SPARQL-based constraint runs only `sh:select`: an `sh:ask` beside it, or a
/// misspelled `sh:mesage`, is refused rather than ignored; an ASK validator
/// carrying an `sh:select` is refused; the SELECT-only neighbours load.
#[test]
fn a_query_a_node_never_runs_is_refused() {
    refused(&sparql_constraint("sh:ask \"ASK {}\" ;"), "shacl#ask");
    refused(&sparql_constraint("sh:mesage \"typo\" ;"), "shacl#mesage");
    refused(
        "ex:C a sh:ConstraintComponent ; sh:parameter [ sh:path ex:q ] ; sh:validator ex:V .
         ex:V a sh:SPARQLAskValidator ; sh:ask \"ASK {}\" ; sh:select \"SELECT $this WHERE {}\" .",
        "never runs",
    );
    loads(&sparql_constraint("sh:message \"fine\" ;"));
    loads(
        "ex:C a sh:ConstraintComponent ; sh:parameter [ sh:path ex:q ] ; sh:validator ex:V .
         ex:V a sh:SPARQLAskValidator ; sh:ask \"ASK {}\" .",
    );
}

/// SHACL JavaScript Extensions are refused with the reason: not SHACL 1.2, and no
/// JavaScript engine.
#[test]
fn shacl_js_is_refused_as_not_shacl_1_2() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:js [ sh:jsFunctionName \"f\" ] .",
        "not part of SHACL 1.2",
    );
}
