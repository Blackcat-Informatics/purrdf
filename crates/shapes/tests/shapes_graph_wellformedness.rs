// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! THE SHAPES-GRAPH WELL-FORMEDNESS REFUSALS, EACH BESIDE ITS VALID NEIGHBOUR.
//!
//! A refusal is a claim that the input is invalid, and a refusal that also fires
//! on valid input is the mirror of the silent drop it replaced. So every refusal
//! the census and the parser hardening added is executed here twice: the input it
//! must refuse, and the nearest input it must accept — and where the neighbour's
//! acceptance could be vacuous (a shape that loads but checks nothing), the
//! neighbour is VALIDATED and its answer observed, with a control row that differs
//! from the treatment row.
//!
//! Also here: SHACL Advanced Features 1.0 graphs (`sh:SPARQLFunction`, `sh:rule`,
//! the AF node-expression spellings) and every non-validating shape property still
//! load, and the SHACL 1.2 list-valued `sh:class` / `sh:datatype` / `sh:nodeKind`
//! and list components answer as the specification says.

use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_shapes::engine::{
    parse_shapes, parse_shapes_with_config, validate_dataset_with_shapes_graph,
};
use purrdf_shapes::model::BoxRoleVocab;
use purrdf_shapes::report::ValidationReport;
use purrdf_shapes::shapes::Shapes;
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const PREFIXES: &str = "
@prefix ex:    <http://example.org/ns#> .
@prefix meta:  <https://example.org/meta/> .
@prefix rdf:   <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs:  <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh:    <http://www.w3.org/ns/shacl#> .
@prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .
@prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .
";

fn load(shapes_ttl: &str) -> Result<Shapes, String> {
    parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None)
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

fn data(data_ttl: &str) -> Arc<RdfDataset> {
    parse_turtle_to_dataset(&format!("{PREFIXES}{data_ttl}"), None).expect("data parses")
}

#[track_caller]
fn validate(shapes_ttl: &str, data_ttl: &str) -> ValidationReport {
    let shapes = loads(shapes_ttl);
    validate_dataset_with_shapes_graph(&data(data_ttl), &shapes, None).expect("validation runs")
}

/// `(focus node, value)` of every result, sorted.
fn results(report: &ValidationReport) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = report
        .results
        .iter()
        .map(|r| {
            (
                r.focus_node.to_string(),
                r.value
                    .as_ref()
                    .map_or_else(String::new, ToString::to_string),
            )
        })
        .collect();
    out.sort();
    out
}

// ── Unknown and unimplemented terms ──────────────────────────────────────────

#[test]
fn a_misspelled_parameter_is_refused_and_the_real_one_loads() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:property [ sh:path ex:p ; sh:minCont 1 ] .",
        "shacl#minCont",
    );
    let report = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:property [ sh:path ex:p ; sh:minCount 1 ] .",
        "ex:b ex:p 1 .",
    );
    assert_eq!(
        results(&report),
        vec![("<http://example.org/ns#a>".to_owned(), String::new())]
    );
}

#[test]
fn an_unknown_term_on_a_node_expression_is_refused_and_an_annotation_loads() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:expression [ sh:count [ sh:path ex:p ] ; sh:countt 1 ] .",
        "shacl#countt",
    );
    loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:expression [ sh:exists [ sh:path ex:p ] ; sh:message \"needs ex:p\" ] .",
    );
}

#[test]
fn shacl_js_is_refused_and_shacl_sparql_loads() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:js [ sh:jsFunctionName \"f\" ] .",
        "SHACL JavaScript Extensions",
    );
    loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:sparql [ sh:select \"SELECT $this WHERE { $this ?p ?o }\" ] .",
    );
}

#[test]
fn the_af_minus_expression_is_refused_and_shnex_remove_loads() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:nodeByExpression [ sh:minus ( ex:S ) ] .",
        "sh:minus",
    );
    loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:expression [ shnex:exists [ shnex:remove ( ex:x ) ; shnex:nodes ( ex:x ex:y ) ] ] .",
    );
}

#[test]
fn values_is_refused_and_the_same_property_shape_without_it_loads() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path ex:p ; sh:values [ sh:path ex:q ] ; sh:minCount 1 ] .",
        "sh:values computes",
    );
    loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:property [ sh:path ex:p ; sh:minCount 1 ] .",
    );
}

#[test]
fn a_parameter_declarations_default_value_loads_and_a_shapes_is_refused() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path ex:p ; sh:defaultValue 1 ] .",
        "sh:defaultValue computes",
    );
    loads(
        "ex:f a sh:SPARQLFunction ;
           sh:parameter [ sh:path ex:x ; sh:datatype xsd:integer ; sh:optional true ;
                          sh:defaultValue 1 ] ;
           sh:returnType xsd:integer ;
           sh:select \"SELECT ((COALESCE($x, 1) * 2) AS ?result) WHERE {}\" .",
    );
}

#[test]
fn target_where_is_refused_and_target_class_loads() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetWhere [ sh:class ex:C ] ; sh:nodeKind sh:IRI .",
        "sh:targetWhere",
    );
    loads("ex:S a sh:NodeShape ; sh:targetClass ex:C ; sh:nodeKind sh:IRI .");
}

#[test]
fn shape_class_is_refused_and_an_rdfs_class_node_shape_loads() {
    refused(
        "ex:C a sh:ShapeClass ; sh:property [ sh:path ex:p ; sh:minCount 1 ] .",
        "sh:ShapeClass",
    );
    let report = validate(
        "ex:C a rdfs:Class, sh:NodeShape ; sh:property [ sh:path ex:p ; sh:minCount 1 ] .",
        "ex:a a ex:C . ex:b a ex:C ; ex:p 1 .",
    );
    assert_eq!(
        results(&report),
        vec![("<http://example.org/ns#a>".to_owned(), String::new())]
    );
}

#[test]
fn debug_severity_is_refused_and_warning_loads() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:severity sh:Debug ; sh:nodeKind sh:Literal .",
        "sh:Debug",
    );
    let report = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:severity sh:Warning ; sh:nodeKind sh:Literal .",
        "",
    );
    assert_eq!(report.results.len(), 1);
}

#[test]
fn a_reifier_annotation_is_refused_and_a_non_validating_one_loads() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal {| sh:deactivated true |} .",
        "reifier annotation",
    );
    loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:IRI ;
           sh:intent \"a is an IRI\"@en {| sh:formalized true |} .",
    );
}

#[test]
fn an_entailment_regime_is_refused_and_the_graph_without_it_loads() {
    refused(
        "ex:graph sh:entailment <http://www.w3.org/ns/entailment/RDFS> .
         ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:IRI .",
        "entailment regime",
    );
    loads("ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:IRI .");
}

#[test]
fn by_types_is_refused_and_closed_true_loads() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:closed sh:ByTypes .",
        "sh:ByTypes",
    );
    let report = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:closed true ; sh:property [ sh:path ex:p ] .",
        "ex:a ex:p 1 ; ex:q 2 .",
    );
    assert_eq!(
        results(&report),
        vec![(
            "<http://example.org/ns#a>".to_owned(),
            "\"2\"^^<http://www.w3.org/2001/XMLSchema#integer>".to_owned()
        )]
    );
}

// ── Ill-typed parameter values ───────────────────────────────────────────────

#[test]
fn a_non_integer_count_is_refused_and_an_integer_one_loads() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:property [ sh:path ex:p ; sh:minCount \"x\" ] .",
        "shacl#minCount",
    );
    // A plain string "1" is as ill-typed as "x": SHACL counts are xsd:integer.
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:property [ sh:path ex:p ; sh:minCount \"1\" ] .",
        "shacl#minCount",
    );
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:property [ sh:path ex:p ; sh:minCount 1.0 ] .",
        "shacl#minCount",
    );
    loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:property [ sh:path ex:p ; sh:minCount 1 ] .",
    );
}

#[test]
fn a_min_count_on_a_node_shape_is_refused_and_on_a_property_shape_loads() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:minCount 1 .",
        "node shapes cannot have any value",
    );
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:uniqueLang true .",
        "node shapes cannot have any value",
    );
    loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:property [ sh:path ex:p ; sh:minCount 1 ; sh:uniqueLang true ] .",
    );
}

#[test]
fn a_string_closed_flag_is_refused_and_boolean_true_closes() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:closed \"true\" .",
        "shacl#closed",
    );
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:closed \"yes\" .",
        "shacl#closed",
    );
    let closed = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:closed true .",
        "ex:a ex:q 2 .",
    );
    assert_eq!(closed.results.len(), 1, "sh:closed true closes the shape");
}

/// SHACL compares its boolean parameters as the RDF term `true`: the W3C suites'
/// `core/property/uniqueLang-002` gives `"1"^^xsd:boolean` and expects it to be
/// inactive. Well-typed, so accepted — and observed inactive against the `true`
/// control, on the same data.
#[test]
fn a_one_valued_boolean_is_well_typed_and_is_not_true() {
    let data = "ex:a ex:p \"x\"@en, \"y\"@en .";
    let one = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path ex:p ; sh:uniqueLang \"1\"^^xsd:boolean ] .",
        data,
    );
    let true_ = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:property [ sh:path ex:p ; sh:uniqueLang true ] .",
        data,
    );
    assert!(one.conforms);
    assert!(!true_.conforms);
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path ex:p ; sh:uniqueLang \"true\" ] .",
        "shacl#uniqueLang",
    );
}

#[test]
fn a_string_deactivation_is_refused_and_boolean_true_deactivates() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal ; sh:deactivated \"true\" .",
        "sh:deactivated",
    );
    let deactivated = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal ; sh:deactivated true .",
        "",
    );
    let active = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal ; sh:deactivated false .",
        "",
    );
    assert!(deactivated.conforms);
    assert!(!active.conforms);
}

#[test]
fn two_flags_and_a_non_literal_pattern_are_refused_and_one_string_pattern_loads() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path ex:p ; sh:pattern \"^a\" ; sh:flags \"i\", \"x\" ] .",
        "shacl#flags",
    );
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:property [ sh:path ex:p ; sh:pattern ex:regex ] .",
        "shacl#pattern",
    );
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:property [ sh:path ex:p ; sh:pattern \"^a\"@en ] .",
        "shacl#pattern",
    );
    let report = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path ex:p ; sh:pattern \"^a\" ; sh:flags \"i\" ] .",
        "ex:a ex:p \"Apple\", \"banana\" .",
    );
    assert_eq!(
        results(&report),
        vec![(
            "<http://example.org/ns#a>".to_owned(),
            "\"banana\"".to_owned()
        )]
    );
}

#[test]
fn a_literal_node_kind_is_refused_and_the_iri_loads() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind \"IRI\" .",
        "shacl#nodeKind",
    );
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind ex:Unknown .",
        "nodeKind",
    );
    loads("ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:IRI .");
}

#[test]
fn a_literal_class_or_target_is_refused_and_the_iri_loads() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:class \"ex:C\" .",
        "shacl#class",
    );
    refused(
        "ex:S a sh:NodeShape ; sh:targetClass \"ex:C\" ; sh:nodeKind sh:IRI .",
        "shacl#targetClass",
    );
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:severity \"warning\" .",
        "sh:severity",
    );
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:message ex:note .",
        "sh:message",
    );
    loads(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ; sh:class ex:C ; sh:severity sh:Warning ;
           sh:message \"one\"@en, \"eins\"@de .",
    );
}

#[test]
fn an_ill_formed_list_is_refused_and_a_well_formed_one_loads() {
    // The cell `_:c` has no rdf:rest, so it is no SHACL list.
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:in _:c . _:c rdf:first ex:x .",
        "well-formed SHACL list",
    );
    loads("ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:in ( ex:x ) .");
}

#[test]
fn a_path_node_with_a_second_form_is_refused_and_one_form_loads() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path [ sh:inversePath ex:p ; sh:zeroOrMorePath ex:q ] ; sh:minCount 1 ] .",
        "not a well-formed SHACL path",
    );
    let report = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path [ sh:inversePath ex:p ] ; sh:minCount 1 ] .",
        "ex:b ex:p ex:a .",
    );
    assert!(report.conforms, "the inverse path reaches ex:b");
}

#[test]
fn a_member_shape_with_a_path_is_refused_and_a_node_shape_loads() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path ex:items ; sh:memberShape [ sh:path ex:p ; sh:minCount 1 ] ] .",
        "must be node shapes",
    );
    loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path ex:items ; sh:memberShape [ sh:nodeKind sh:IRI ] ] .",
    );
}

#[test]
fn a_literal_box_role_is_refused_and_an_iri_role_loads() {
    let vocab = || Some(BoxRoleVocab::for_namespace("https://example.org/meta/"));
    let refused_role = parse_shapes_with_config(
        &format!(
            "{PREFIXES} ex:S a sh:NodeShape ; sh:targetNode ex:a ; meta:graphBoxRole \"tbox\" ."
        ),
        None,
        vocab(),
    )
    .expect_err("a literal role is refused");
    assert!(refused_role.contains("graphBoxRole"), "{refused_role}");
    parse_shapes_with_config(
        &format!(
            "{PREFIXES} ex:S a sh:NodeShape ; sh:targetNode ex:a ; meta:graphBoxRole meta:tbox ."
        ),
        None,
        vocab(),
    )
    .expect("an IRI role loads");
}

#[test]
fn a_blank_sparql_function_is_refused_and_a_named_one_loads() {
    refused(
        "[] a sh:SPARQLFunction ; sh:returnType xsd:integer ;
            sh:select \"SELECT (1 AS ?result) WHERE {}\" .",
        "is not an IRI",
    );
    loads(
        "ex:one a sh:SPARQLFunction ; sh:returnType xsd:integer ;
            sh:select \"SELECT (1 AS ?result) WHERE {}\" .",
    );
}

#[test]
fn a_string_rule_deactivation_is_refused_and_boolean_true_deactivates_the_rule() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
           sh:rule [ a sh:TripleRule ; sh:deactivated \"yes\" ;
                     sh:subject sh:this ; sh:predicate ex:q ; sh:object ex:v ] .",
        "sh:deactivated",
    );
    let entail = |flag: &str| {
        let shapes = loads(&format!(
            "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
               sh:rule [ a sh:TripleRule ; sh:deactivated {flag} ;
                         sh:subject sh:this ; sh:predicate ex:q ; sh:object ex:v ] ."
        ));
        let input = data("ex:a a ex:C .");
        let entailed = purrdf_shapes::entail_dataset(input.as_ref(), &shapes).expect("rules run");
        purrdf::canonicalize(entailed.as_ref()).nquads
    };
    assert!(!entail("true").contains("<http://example.org/ns#q>"));
    assert!(entail("false").contains("<http://example.org/ns#q>"));
}

// ── sh:targetNode as a node expression ───────────────────────────────────────

#[test]
fn a_structured_target_node_is_refused_the_empty_one_targets_nothing_and_an_iri_targets() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode [ sh:path ex:p ] ; sh:nodeKind sh:Literal .",
        "sh:targetNode",
    );
    // `[]` is the empty node expression: no targets, so nothing is validated.
    let empty = validate(
        "ex:S a sh:NodeShape ; sh:targetNode [] ; sh:nodeKind sh:Literal .",
        "",
    );
    assert!(empty.conforms);
    let iri = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal .",
        "",
    );
    assert_eq!(
        results(&iri),
        vec![(
            "<http://example.org/ns#a>".to_owned(),
            "<http://example.org/ns#a>".to_owned()
        )]
    );
}

// ── SHACL 1.2 list-valued sh:class / sh:datatype / sh:nodeKind ──────────────

#[test]
fn a_class_list_is_a_disjunction_and_separate_values_a_conjunction() {
    let data = "ex:cat a ex:Cat . ex:dog a ex:Dog . ex:rock a ex:Rock .
                ex:x ex:pet ex:cat, ex:dog, ex:rock .";
    let list = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:x ;
           sh:property [ sh:path ex:pet ; sh:class ( ex:Cat ex:Dog ) ] .",
        data,
    );
    assert_eq!(
        results(&list),
        vec![(
            "<http://example.org/ns#x>".to_owned(),
            "<http://example.org/ns#rock>".to_owned()
        )]
    );
    // Control: two separate values are two constraints, and every pet fails one.
    let conjunction = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:x ;
           sh:property [ sh:path ex:pet ; sh:class ex:Cat, ex:Dog ] .",
        data,
    );
    assert_eq!(conjunction.results.len(), 4);
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:x ; sh:class ( ex:Cat \"Dog\" ) .",
        "non-IRI member",
    );
}

#[test]
fn a_datatype_list_is_a_disjunction() {
    let report = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:x ;
           sh:property [ sh:path ex:label ; sh:datatype ( xsd:string rdf:langString ) ] .",
        "ex:x ex:label \"plain\", \"tagged\"@en, 3 .",
    );
    assert_eq!(
        results(&report),
        vec![(
            "<http://example.org/ns#x>".to_owned(),
            "\"3\"^^<http://www.w3.org/2001/XMLSchema#integer>".to_owned()
        )]
    );
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:x ; sh:datatype ( xsd:string 3 ) .",
        "non-IRI member",
    );
}

#[test]
fn a_node_kind_list_is_a_disjunction_of_basic_kinds() {
    let report = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:x ;
           sh:property [ sh:path ex:p ; sh:nodeKind ( sh:BlankNode sh:IRI ) ] .",
        "ex:x ex:p ex:y, [], \"lit\" .",
    );
    assert_eq!(
        results(&report),
        vec![("<http://example.org/ns#x>".to_owned(), "\"lit\"".to_owned())]
    );
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:x ; sh:nodeKind ( sh:BlankNodeOrIRI sh:Literal ) .",
        "nodeKind> list",
    );
}

#[test]
fn triple_term_is_a_node_kind() {
    let report = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:x ;
           sh:property [ sh:path ex:p ; sh:nodeKind sh:TripleTerm ] .",
        "ex:x ex:p <<( ex:a ex:b ex:c )>>, ex:y .",
    );
    assert_eq!(
        results(&report),
        vec![(
            "<http://example.org/ns#x>".to_owned(),
            "<http://example.org/ns#y>".to_owned()
        )]
    );
}

#[test]
fn a_datatype_list_projects_to_json_schema_any_of() {
    let shapes = loads(
        "ex:Thing a sh:NodeShape ; sh:targetClass ex:Thing ;
           sh:property [ sh:path ex:label ; sh:datatype ( xsd:string xsd:integer ) ] .",
    );
    let ns = purrdf_shapes::json_schema::Namespaces::new(
        "ex",
        &[("ex".to_owned(), "http://example.org/ns#".to_owned())],
    )
    .expect("namespaces");
    let compiled = purrdf_shapes::json_schema::compile(&shapes, &ns).expect("the schema compiles");
    let schema: serde_json::Value =
        serde_json::from_str(&compiled.schema_json).expect("the schema is JSON");
    let text = schema.to_string();
    assert!(
        text.contains("\"anyOf\""),
        "a datatype list projects as anyOf: {text}"
    );
}

// ── SHACL 1.2 list components ────────────────────────────────────────────────

#[test]
fn list_lengths_are_checked_and_a_non_list_value_gets_a_result() {
    let shapes = "ex:S a sh:NodeShape ; sh:targetNode ex:x ;
           sh:property [ sh:path ex:items ; sh:minListLength 1 ; sh:maxListLength 2 ] .";
    let report = validate(
        shapes,
        "ex:x ex:items ( ex:a ), rdf:nil, ( ex:a ex:b ex:c ), ex:notAList .",
    );
    // rdf:nil is too short, the three-member list too long, and ex:notAList is
    // no list: each of min and max reports it.
    assert_eq!(report.results.len(), 4);
    // Neighbour: rdf:nil is a well-formed list of length 0.
    let nil = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:x ; sh:property [ sh:path ex:items ; sh:minListLength 0 ] .",
        "ex:x ex:items rdf:nil .",
    );
    assert!(nil.conforms);
}

#[test]
fn unique_members_and_member_shape_report_their_details() {
    let unique = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:x ; sh:property [ sh:path ex:items ; sh:uniqueMembers true ] .",
        "ex:x ex:items ( ex:a ex:b ex:a ) .",
    );
    assert_eq!(unique.results.len(), 1);
    assert_eq!(
        unique.results[0]
            .details
            .iter()
            .map(|d| d.value.as_ref().map(ToString::to_string))
            .collect::<Vec<_>>(),
        vec![Some("<http://example.org/ns#a>".to_owned())]
    );
    let control = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:x ; sh:property [ sh:path ex:items ; sh:uniqueMembers true ] .",
        "ex:x ex:items ( ex:a ex:b ) .",
    );
    assert!(control.conforms);

    let member = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:x ;
           sh:property [ sh:path ex:items ; sh:memberShape [ sh:nodeKind sh:IRI ] ] .",
        "ex:x ex:items ( ex:a \"b\" ) .",
    );
    assert_eq!(member.results.len(), 1);
    assert_eq!(member.results[0].details.len(), 1);
    assert_eq!(
        member.results[0].details[0].focus_node.to_string(),
        "\"b\"",
        "the detail is the member's own result against the member shape"
    );
    let report_text = member.to_ntriples();
    assert!(
        report_text.contains("<http://www.w3.org/ns/shacl#detail>"),
        "{report_text}"
    );
}

// ── SHACL Advanced Features 1.0 and non-validating properties still load ─────

#[test]
fn an_af_graph_with_functions_rules_and_node_expressions_loads_and_runs() {
    let shapes_ttl = "
        ex:tripled a sh:SPARQLFunction ;
            sh:parameter [ sh:path ex:x ; sh:datatype xsd:integer ; sh:order 0 ; sh:name \"x\" ] ;
            sh:returnType xsd:integer ;
            sh:select \"SELECT ((?x * 3) AS ?result) WHERE {}\" .
        ex:S a sh:NodeShape ; sh:targetClass ex:C ;
            sh:rule [ a sh:TripleRule ; sh:order 1 ; sh:deactivated false ;
                      sh:subject sh:this ; sh:predicate ex:childCount ;
                      sh:object [ sh:count [ sh:path ex:child ] ] ] ;
            sh:rule [ a sh:SPARQLRule ; sh:construct \"CONSTRUCT { $this ex:seen true } WHERE {}\" ] ;
            sh:expression [ sh:if [ sh:path ex:flag ] ; sh:then true ; sh:else false ] ;
            sh:sparql [ sh:select \"SELECT $this WHERE { $this ex:amount ?a . FILTER(<http://example.org/ns#tripled>(?a) > 100) }\" ] .
    ";
    let shapes = loads(shapes_ttl);
    let input = data(
        "ex:ok a ex:C ; ex:flag true ; ex:amount 10 ; ex:child ex:k1, ex:k2 .
                      ex:bad a ex:C ; ex:flag true ; ex:amount 50 .",
    );
    let report = validate_dataset_with_shapes_graph(&input, &shapes, None).expect("validates");
    assert_eq!(
        report
            .results
            .iter()
            .map(|r| r.focus_node.to_string())
            .collect::<Vec<_>>(),
        vec!["<http://example.org/ns#bad>".to_owned()],
        "the SPARQL function ran: only ex:bad's tripled amount exceeds 100"
    );
    let entailed = purrdf_shapes::entail_dataset(input.as_ref(), &shapes).expect("the rules run");
    let nquads = purrdf::canonicalize(entailed.as_ref()).nquads;
    assert!(
        nquads.contains(
            "<http://example.org/ns#childCount> \"2\"^^<http://www.w3.org/2001/XMLSchema#integer>"
        ),
        "{nquads}"
    );
    assert!(nquads.contains("<http://example.org/ns#seen>"), "{nquads}");
}

#[test]
fn a_shape_with_every_non_validating_property_loads_and_validates_unchanged() {
    let annotated = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal ;
           sh:name \"S\"@en ; sh:description \"the <b>S</b>\"^^rdf:HTML ; sh:order 1 ;
           sh:group ex:G ; sh:intent \"a is literal\"@en {| sh:formalized true |} ;
           sh:agentInstruction \"check a\"@en ; sh:codeIdentifier \"s_shape\" ;
           sh:unit \"cm\" ; sh:labelTemplate \"{$this}\" .
         ex:G a sh:PropertyGroup .",
        "",
    );
    let bare = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal .",
        "",
    );
    assert_eq!(annotated.to_ntriples(), bare.to_ntriples());
    assert!(
        !bare.conforms,
        "the shape checks something, so equality is not vacuous"
    );
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:name ex:NotText .",
        "shacl#name",
    );
}
