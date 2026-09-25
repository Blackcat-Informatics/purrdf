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
//! load, and the SHACL 1.2 list-valued `sh:class` / `sh:datatype` / `sh:nodeKind`,
//! the list components, `sh:singleLine`, `sh:rootClass` and `sh:someValue`, and
//! the path-valued property pairs and `sh:subsetOf` answer as the specification
//! says.

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

// ── SHACL 1.2 sh:singleLine, sh:rootClass and sh:someValue ───────────────────

/// `(focus node, value)` of every result of `component`, sorted.
fn component_results(report: &ValidationReport, component: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = report
        .results
        .iter()
        .filter(|r| r.source_constraint_component.as_str() == component)
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

const SINGLE_LINE: &str = "http://www.w3.org/ns/shacl#SingleLineConstraintComponent";
const ROOT_CLASS: &str = "http://www.w3.org/ns/shacl#RootClassConstraintComponent";
const SOME_VALUE: &str = "http://www.w3.org/ns/shacl#SomeValueConstraintComponent";

/// `sh:singleLine true` reports each literal whose lexical form holds a line
/// feed, carriage return, form feed or vertical tab, with the literal as
/// `sh:value`; a literal without one conforms, and an IRI is never judged.
#[test]
fn single_line_true_reports_each_literal_with_a_line_break() {
    let shapes = "ex:S a sh:NodeShape ; sh:targetNode ex:x ;
           sh:property [ sh:path ex:label ; sh:singleLine true ] .";
    let report = validate(
        shapes,
        "ex:x ex:label \"one line\", \"two\\nlines\", \"vertical\\u000Btab\", ex:anIri .",
    );
    assert_eq!(
        component_results(&report, SINGLE_LINE),
        vec![
            (
                "<http://example.org/ns#x>".to_owned(),
                "\"two\\nlines\"".to_owned()
            ),
            (
                "<http://example.org/ns#x>".to_owned(),
                "\"vertical\\u000Btab\"".to_owned()
            ),
        ]
    );
    assert_eq!(report.results.len(), 2);
    // Neighbour: a literal with no line break conforms.
    let single = validate(shapes, "ex:x ex:label \"one line\", \"tab\\tis fine\" .");
    assert!(single.conforms, "{:?}", single.results);
}

/// `sh:singleLine false` checks nothing: the treatment row that `true` reports
/// conforms under `false`.
#[test]
fn single_line_false_admits_a_line_break() {
    let data_ttl = "ex:x ex:label \"two\\nlines\" .";
    let off = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:x ;
           sh:property [ sh:path ex:label ; sh:singleLine false ] .",
        data_ttl,
    );
    assert!(off.conforms, "{:?}", off.results);
    let on = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:x ;
           sh:property [ sh:path ex:label ; sh:singleLine true ] .",
        data_ttl,
    );
    assert_eq!(component_results(&on, SINGLE_LINE).len(), 1);
}

/// A non-boolean `sh:singleLine` is refused at load; the boolean neighbour loads
/// and is honoured.
#[test]
fn a_non_boolean_single_line_is_refused_and_a_boolean_loads() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:x ;
           sh:property [ sh:path ex:label ; sh:singleLine \"true\" ] .",
        "singleLine",
    );
    let report = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:x ;
           sh:property [ sh:path ex:label ; sh:singleLine true ] .",
        "ex:x ex:label \"a\\nb\" .",
    );
    assert_eq!(component_results(&report, SINGLE_LINE).len(), 1);
}

const HIERARCHY: &str = "
    ex:Animal a rdfs:Class .
    ex:Mammal rdfs:subClassOf ex:Animal .
    ex:Dog rdfs:subClassOf ex:Mammal .
    ex:Plant rdfs:subClassOf ex:Organism .
    ex:Loop1 rdfs:subClassOf ex:Loop2 .
    ex:Loop2 rdfs:subClassOf ex:Loop1 .
";

/// `sh:rootClass` admits the root itself (the reflexive `*`) and every transitive
/// subclass, and reports each other value node with it as `sh:value`: an IRI
/// outside the hierarchy (including one on a subclass cycle), a literal and a
/// blank node.
#[test]
fn root_class_admits_the_root_and_its_subclasses_only() {
    let shapes = "ex:S a sh:NodeShape ; sh:targetNode ex:zoo ;
           sh:property [ sh:path ex:holds ; sh:rootClass ex:Animal ] .";
    let conforming = validate(
        shapes,
        &format!("{HIERARCHY} ex:zoo ex:holds ex:Animal, ex:Mammal, ex:Dog ."),
    );
    assert!(conforming.conforms, "{:?}", conforming.results);
    let report = validate(
        shapes,
        &format!("{HIERARCHY} ex:zoo ex:holds ex:Dog, ex:Plant, ex:Loop1, \"ex:Animal\", [] ."),
    );
    let zoo = "<http://example.org/ns#zoo>".to_owned();
    let mut values: Vec<String> = component_results(&report, ROOT_CLASS)
        .into_iter()
        .map(|(focus, value)| {
            assert_eq!(focus, zoo);
            value
        })
        .collect();
    values.sort();
    assert_eq!(values.len(), 4, "{values:?}");
    assert!(values.iter().any(|v| v.starts_with("_:")), "{values:?}");
    assert!(values.contains(&"\"ex:Animal\"".to_owned()), "{values:?}");
    assert!(
        values.contains(&"<http://example.org/ns#Plant>".to_owned()),
        "{values:?}"
    );
    assert!(
        values.contains(&"<http://example.org/ns#Loop1>".to_owned()),
        "{values:?}"
    );
    assert_eq!(report.results.len(), 4);
}

/// A root the data graph never mentions but a value node names is still that
/// root: the reflexive half needs no `rdfs:subClassOf` edge.
#[test]
fn root_class_is_reflexive_without_any_hierarchy() {
    let report = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:zoo ;
           sh:property [ sh:path ex:holds ; sh:rootClass ex:Standalone ] .",
        "ex:zoo ex:holds ex:Standalone .",
    );
    assert!(report.conforms, "{:?}", report.results);
    let control = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:zoo ;
           sh:property [ sh:path ex:holds ; sh:rootClass ex:Standalone ] .",
        "ex:zoo ex:holds ex:Other .",
    );
    assert_eq!(component_results(&control, ROOT_CLASS).len(), 1);
}

/// A list value is a set of roots: a value node under ANY of them conforms.
#[test]
fn root_class_list_admits_any_root() {
    let shapes = "ex:S a sh:NodeShape ; sh:targetNode ex:zoo ;
           sh:property [ sh:path ex:holds ; sh:rootClass ( ex:Animal ex:Organism ) ] .";
    let report = validate(
        shapes,
        &format!("{HIERARCHY} ex:zoo ex:holds ex:Dog, ex:Plant, ex:Loop1 ."),
    );
    assert_eq!(
        component_results(&report, ROOT_CLASS),
        vec![(
            "<http://example.org/ns#zoo>".to_owned(),
            "<http://example.org/ns#Loop1>".to_owned()
        )]
    );
}

/// A literal `sh:rootClass`, or a list holding one, is refused at load; the IRI
/// and IRI-list neighbours load and are honoured (see the tests above).
#[test]
fn an_ill_typed_root_class_is_refused_and_iris_load() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:zoo ;
           sh:property [ sh:path ex:holds ; sh:rootClass \"ex:Animal\" ] .",
        "rootClass",
    );
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:zoo ;
           sh:property [ sh:path ex:holds ; sh:rootClass ( ex:Animal \"ex:Plant\" ) ] .",
        "rootClass",
    );
    loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:zoo ;
           sh:property [ sh:path ex:holds ; sh:rootClass ( ex:Animal ex:Plant ) ] .",
    );
}

const DUCKS: &str = "
    ex:donald a ex:Duck .
    ex:daisy a ex:Duck .
    ex:eliza a ex:Cow .
";

/// `sh:someValue`: one conforming value node among several conforms; a focus
/// node with none — including one with no value node at all — gets exactly one
/// result, naming no `sh:value`.
#[test]
fn some_value_needs_one_conforming_value_and_reports_once_without_a_value() {
    let shapes = "ex:S a sh:NodeShape ; sh:targetNode ex:alice, ex:bob, ex:carol ;
           sh:property [ sh:path ex:tends ; sh:someValue [ sh:class ex:Duck ] ] .";
    let report = validate(
        shapes,
        &format!(
            "{DUCKS} ex:alice ex:tends ex:eliza, ex:carrot .
                     ex:bob ex:tends ex:eliza, ex:donald, ex:daisy ."
        ),
    );
    assert_eq!(
        component_results(&report, SOME_VALUE),
        vec![
            ("<http://example.org/ns#alice>".to_owned(), String::new()),
            ("<http://example.org/ns#carol>".to_owned(), String::new()),
        ]
    );
    assert_eq!(report.results.len(), 2);
    assert!(report.results.iter().all(|r| r.value.is_none()));
}

/// On a node shape the one value node is the focus node, so `sh:someValue` asks
/// what `sh:node` asks.
#[test]
fn some_value_on_a_node_shape_judges_the_focus_node() {
    let shapes = "ex:S a sh:NodeShape ; sh:targetNode ex:donald, ex:eliza ;
           sh:someValue [ sh:class ex:Duck ] .";
    let report = validate(shapes, DUCKS);
    assert_eq!(
        component_results(&report, SOME_VALUE),
        vec![("<http://example.org/ns#eliza>".to_owned(), String::new())]
    );
}

/// A failure while checking a value node against `sh:someValue` is produced when
/// no value node conforms, and discarded when one does.
///
/// The `sh:someValue` shape is a disjunction whose first member admits a duck
/// and whose second member calls a function that recurses without bound, so
/// checking a non-duck FAILS. The focus node tending only a cow therefore makes
/// validation fail; the focus node tending the cow AND a duck conforms, because
/// a conforming value node overrides the failure whichever order the two are
/// checked in.
#[test]
fn a_failure_inside_some_value_propagates_unless_a_value_conforms() {
    let shapes = |targets: &str| {
        format!(
            "ex:loop a sh:ListParameterExpressionFunction ;
               sh:bodyExpression [ ex:loop ( [ shnex:arg 0 ] ) ] ;
               sh:parameter [ sh:path shnex:arg0 ] .
             ex:S a sh:NodeShape ; sh:targetNode {targets} ;
               sh:property [ sh:path ex:tends ; sh:someValue [
                 sh:or ( [ sh:class ex:Duck ] [ sh:expression [ ex:loop ( sh:this ) ] ] )
               ] ] ."
        )
    };
    let data_ttl =
        format!("{DUCKS} ex:alice ex:tends ex:eliza . ex:bob ex:tends ex:eliza, ex:donald .");
    let error =
        validate_dataset_with_shapes_graph(&data(&data_ttl), &loads(&shapes("ex:alice")), None)
            .expect_err("a failure with no conforming value node is produced");
    assert!(
        error.contains("64"),
        "the failure is the recursion bound: {error}"
    );
    let report = validate(&shapes("ex:bob"), &data_ttl);
    assert!(report.conforms, "{:?}", report.results);
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

// ── SHACL 1.2 path-valued property pairs and sh:subsetOf ─────────────────────

const EQUALS: &str = "http://www.w3.org/ns/shacl#EqualsConstraintComponent";
const DISJOINT: &str = "http://www.w3.org/ns/shacl#DisjointConstraintComponent";
const SUBSET_OF: &str = "http://www.w3.org/ns/shacl#SubsetOfConstraintComponent";
const LESS_THAN: &str = "http://www.w3.org/ns/shacl#LessThanConstraintComponent";
const LESS_THAN_OR_EQUALS: &str = "http://www.w3.org/ns/shacl#LessThanOrEqualsConstraintComponent";

fn x(local: &str) -> String {
    format!("<http://example.org/ns#{local}>")
}

/// The IRI form of every property pair answers as it always has: the IRI is the
/// one-hop predicate path, each component reports exactly the value nodes the
/// SHACL 1.0 text names, and the parsed constraint still carries that predicate.
#[test]
fn iri_valued_property_pairs_answer_as_before() {
    let shapes = "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path ex:p ; sh:equals ex:q ; sh:disjoint ex:r ] ;
           sh:property [ sh:path ex:start ; sh:lessThan ex:end ;
                         sh:lessThanOrEquals ex:stop ] .";
    let report = validate(
        shapes,
        "ex:a ex:p ex:v1, ex:v2 ; ex:q ex:v2, ex:v3 ; ex:r ex:v1 ;
              ex:start 5 ; ex:end 5 ; ex:stop 4 .",
    );
    let a = x("a");
    assert_eq!(
        component_results(&report, EQUALS),
        vec![(a.clone(), x("v1")), (a.clone(), x("v3"))]
    );
    assert_eq!(
        component_results(&report, DISJOINT),
        vec![(a.clone(), x("v1"))]
    );
    let five = "\"5\"^^<http://www.w3.org/2001/XMLSchema#integer>".to_owned();
    assert_eq!(
        component_results(&report, LESS_THAN),
        vec![(a.clone(), five.clone())]
    );
    assert_eq!(
        component_results(&report, LESS_THAN_OR_EQUALS),
        vec![(a, five)]
    );
    assert_eq!(report.results.len(), 5);
    // The neighbour row conforms under the same shapes.
    let conforming = validate(
        shapes,
        "ex:a ex:p ex:v2 ; ex:q ex:v2 ; ex:r ex:v1 ; ex:start 4 ; ex:end 5 ; ex:stop 4 .",
    );
    assert!(conforming.conforms, "{:?}", conforming.results);
    // The parsed constraint is the predicate path of the IRI.
    let parsed = loads(shapes);
    let property = &parsed.node_shapes[0].property_shapes[0];
    assert!(property.constraints.iter().any(|c| matches!(
        c,
        purrdf_shapes::shapes::Constraint::Equals(purrdf_shapes::shapes::Path::Predicate(n))
            if n.as_str() == "http://example.org/ns#q"
    )));
}

/// `sh:equals [ sh:inversePath ex:q ]` compares against the nodes that reach the
/// focus node through `ex:q`. The treatment row conforms only under the inverse
/// reading and the control row conforms only under the forward one, so a reading
/// that dropped the inversion answers both rows the other way round.
#[test]
fn an_inverse_path_equals_is_evaluated_as_the_inverse() {
    let shapes = "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path ex:p ; sh:equals [ sh:inversePath ex:q ] ] .";
    let inverse = validate(shapes, "ex:a ex:p ex:b . ex:b ex:q ex:a .");
    assert!(inverse.conforms, "{:?}", inverse.results);
    let forward = validate(shapes, "ex:a ex:p ex:b ; ex:q ex:b .");
    assert_eq!(component_results(&forward, EQUALS), vec![(x("a"), x("b"))]);
}

/// `sh:disjoint ( ex:a ex:b )` compares against the two-hop values. The control
/// row puts the value one hop away along each step alone, which the sequence
/// never reaches, so only the treatment row reports it.
#[test]
fn a_sequence_path_disjoint_is_evaluated_as_the_sequence() {
    let shapes = "ex:S a sh:NodeShape ; sh:targetNode ex:f ;
           sh:property [ sh:path ex:p ; sh:disjoint ( ex:a ex:b ) ] .";
    let treatment = validate(shapes, "ex:f ex:p \"v\" ; ex:a ex:n . ex:n ex:b \"v\" .");
    assert_eq!(
        component_results(&treatment, DISJOINT),
        vec![(x("f"), "\"v\"".to_owned())]
    );
    let control = validate(shapes, "ex:f ex:p \"v\" ; ex:a \"v\" ; ex:b \"v\" .");
    assert!(control.conforms, "{:?}", control.results);
}

/// `sh:lessThan ( ex:next ex:start )` orders each value below the start of the
/// next node — a comparand only the sequence reaches.
#[test]
fn a_sequence_path_less_than_orders_against_the_reached_nodes() {
    let shapes = "ex:S a sh:NodeShape ; sh:targetNode ex:f ;
           sh:property [ sh:path ex:start ; sh:lessThan ( ex:next ex:start ) ] .";
    let before = validate(shapes, "ex:f ex:start 1 ; ex:next ex:g . ex:g ex:start 2 .");
    assert!(before.conforms, "{:?}", before.results);
    let after = validate(shapes, "ex:f ex:start 3 ; ex:next ex:g . ex:g ex:start 2 .");
    assert_eq!(
        component_results(&after, LESS_THAN),
        vec![(
            x("f"),
            "\"3\"^^<http://www.w3.org/2001/XMLSchema#integer>".to_owned()
        )]
    );
}

/// `sh:subsetOf` reports each value node the compared path does not reach, with
/// the value node as `sh:value`. Against `ex:child` a grandchild is outside the
/// subset; against `[ sh:oneOrMorePath ex:child ]` it is inside — the same data
/// row, two different answers.
#[test]
fn subset_of_reports_each_value_outside_the_reached_nodes() {
    let data_ttl = "ex:f ex:favourite ex:c, ex:b ; ex:child ex:b . ex:b ex:child ex:c .";
    let direct = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:f ;
           sh:property [ sh:path ex:favourite ; sh:subsetOf ex:child ] .",
        data_ttl,
    );
    assert_eq!(
        component_results(&direct, SUBSET_OF),
        vec![(x("f"), x("c"))]
    );
    assert_eq!(direct.results.len(), 1);
    let descendants = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:f ;
           sh:property [ sh:path ex:favourite ; sh:subsetOf [ sh:oneOrMorePath ex:child ] ] .",
        data_ttl,
    );
    assert!(descendants.conforms, "{:?}", descendants.results);
}

/// A focus node the data graph does not intern reaches itself through a
/// zero-length path and nothing else. `sh:equals [ sh:zeroOrMorePath ex:q ]`
/// and `sh:subsetOf [ sh:zeroOrOnePath ex:q ]` therefore hold for it; the IRI
/// form of each reaches nothing, so the same focus node is reported.
#[test]
fn a_focus_node_absent_from_the_data_reaches_itself_along_a_reflexive_path() {
    let reflexive = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:absent ;
           sh:property [ sh:path [ sh:zeroOrMorePath ex:p ] ;
                         sh:equals [ sh:zeroOrMorePath ex:q ] ;
                         sh:subsetOf [ sh:zeroOrOnePath ex:q ] ] .",
        "ex:other ex:p ex:other .",
    );
    assert!(reflexive.conforms, "{:?}", reflexive.results);
    let one_hop = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:absent ;
           sh:property [ sh:path [ sh:zeroOrMorePath ex:p ] ;
                         sh:equals ex:q ; sh:subsetOf ex:q ] .",
        "ex:other ex:p ex:other .",
    );
    assert_eq!(
        component_results(&one_hop, EQUALS),
        vec![(x("absent"), x("absent"))]
    );
    assert_eq!(
        component_results(&one_hop, SUBSET_OF),
        vec![(x("absent"), x("absent"))]
    );
}

/// SHACL 1.2 Core §7.6.3 sets no node-shape restriction on `sh:subsetOf` (unlike
/// §7.6.4 and §7.6.5 for `sh:lessThan` and `sh:lessThanOrEquals`), so on a node
/// shape it loads and judges the focus node, its one value node: it holds when
/// the path leads back to the focus node and is reported when it does not.
#[test]
fn subset_of_on_a_node_shape_judges_the_focus_node() {
    let shapes = "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:subsetOf ex:self .";
    let back = validate(shapes, "ex:a ex:self ex:a .");
    assert!(back.conforms, "{:?}", back.results);
    let away = validate(shapes, "ex:a ex:self ex:b .");
    assert_eq!(component_results(&away, SUBSET_OF), vec![(x("a"), x("a"))]);
}

/// `sh:lessThan` over a sequence path on a node shape is refused ("Node shapes
/// cannot have any value for sh:lessThan"); the same value on a property shape
/// loads and is evaluated.
#[test]
fn a_path_valued_less_than_on_a_node_shape_is_refused_and_on_a_property_shape_loads() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:lessThan ( ex:next ex:start ) .",
        "lessThan",
    );
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:lessThanOrEquals ( ex:next ex:start ) .",
        "lessThanOrEquals",
    );
    let report = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path ex:start ; sh:lessThanOrEquals ( ex:next ex:start ) ] .",
        "ex:a ex:start 3 ; ex:next ex:g . ex:g ex:start 2 .",
    );
    assert_eq!(component_results(&report, LESS_THAN_OR_EQUALS).len(), 1);
}

/// A literal is no property path: as the value of `sh:subsetOf` or of any other
/// pair it is refused, and the IRI neighbour loads and is honoured.
#[test]
fn a_literal_pair_value_is_refused_and_an_iri_path_loads() {
    for parameter in [
        "subsetOf",
        "equals",
        "disjoint",
        "lessThan",
        "lessThanOrEquals",
    ] {
        refused(
            &format!(
                "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
                   sh:property [ sh:path ex:p ; sh:{parameter} \"ex:q\" ] ."
            ),
            parameter,
        );
    }
    let report = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path ex:p ; sh:subsetOf ex:q ] .",
        "ex:a ex:p ex:v .",
    );
    assert_eq!(
        component_results(&report, SUBSET_OF),
        vec![(x("a"), x("v"))]
    );
}

/// A blank node that is no path form — here one carrying an unrelated predicate —
/// is refused as the value of `sh:equals`; a well-formed blank path loads.
#[test]
fn a_malformed_blank_pair_path_is_refused_and_a_well_formed_one_loads() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path ex:p ; sh:equals [ ex:notAPath ex:q ] ] .",
        "equals",
    );
    let report = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path ex:p ; sh:equals [ sh:alternativePath ( ex:q ex:r ) ] ] .",
        "ex:a ex:p ex:v ; ex:q ex:v ; ex:r ex:w .",
    );
    assert_eq!(component_results(&report, EQUALS), vec![(x("a"), x("w"))]);
}
