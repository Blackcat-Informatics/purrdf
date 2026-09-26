// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! THE SHACL 1.2 TARGETS THAT ARE EVALUATED, EACH BESIDE A CONTROL ROW.
//!
//! Implicit class targets decided on SHACL instances, `sh:ShapeClass`, explicit
//! shape targets (`sh:shape` in the data graph), where targets (`sh:targetWhere`)
//! and node-expression `sh:targetNode` values. A target is observed through the
//! RESULTS it produces: every fixture shape carries a constraint every targeted
//! node fails, so the result rows are the focus set, and each control row differs
//! from its treatment row — a target silently dropped and a target honoured can
//! never produce the same answer here.
//!
//! Fixture IRIs are under `example.org`: PurRDF mints no vocabulary IRIs.

use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_shapes::engine::{parse_shapes, validate_dataset_with_shapes_graph};
use purrdf_shapes::report::ValidationReport;
use purrdf_shapes::shapes::Shapes;
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const PREFIXES: &str = "
@prefix ex:    <http://example.org/ns#> .
@prefix rdf:   <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs:  <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh:    <http://www.w3.org/ns/shacl#> .
@prefix sparql: <http://www.w3.org/ns/sparql#> .
@prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .
";

fn shapes(shapes_ttl: &str) -> Shapes {
    parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None)
        .unwrap_or_else(|error| panic!("the shapes graph must load: {error}"))
}

fn data(data_ttl: &str) -> Arc<RdfDataset> {
    parse_turtle_to_dataset(&format!("{PREFIXES}{data_ttl}"), None).expect("data parses")
}

fn validate(shapes_ttl: &str, data_ttl: &str) -> ValidationReport {
    validate_dataset_with_shapes_graph(&data(data_ttl), &shapes(shapes_ttl), None)
        .expect("validation runs")
}

/// The focus node of every result, sorted and deduplicated — the focus set, for a
/// shape every targeted node fails.
fn focus_nodes(report: &ValidationReport) -> Vec<String> {
    let mut out: Vec<String> = report
        .results
        .iter()
        .map(|result| result.focus_node.to_string())
        .collect();
    out.sort();
    out.dedup();
    out
}

fn ex(local: &str) -> String {
    format!("<http://example.org/ns#{local}>")
}

// ── Implicit class targets ─────────────────────────────────────────────────────

/// "If s is a SHACL instance of sh:NodeShape or sh:PropertyShape in a shapes graph
/// SG and s is also a SHACL instance of rdfs:Class in SG then the set of SHACL
/// instances of s in a data graph DG is a target from DG for s in SG." SHACL
/// INSTANCE: a type that is a SHACL subclass of `rdfs:Class` (or of `sh:NodeShape`)
/// counts exactly as the class itself does.
#[test]
fn an_implicit_class_target_follows_shacl_instances_in_the_shapes_graph() {
    let instances = "ex:a a ex:C . ex:b a ex:Other .";
    let via_meta_class = validate(
        "ex:MetaClass rdfs:subClassOf rdfs:Class .
         ex:C a ex:MetaClass, sh:NodeShape ; sh:nodeKind sh:Literal .",
        instances,
    );
    assert_eq!(focus_nodes(&via_meta_class), vec![ex("a")]);
    let via_shape_kind = validate(
        "ex:ShapeKind rdfs:subClassOf sh:NodeShape .
         ex:C a ex:ShapeKind, rdfs:Class ; sh:nodeKind sh:Literal .",
        instances,
    );
    assert_eq!(focus_nodes(&via_shape_kind), vec![ex("a")]);
    // Control: the same shape without the class typing targets nothing.
    let not_a_class = validate(
        "ex:ShapeKind rdfs:subClassOf sh:NodeShape .
         ex:C a ex:ShapeKind ; sh:nodeKind sh:Literal .",
        instances,
    );
    assert!(not_a_class.conforms && not_a_class.results.is_empty());
}

/// A property shape qualifies as a node shape does: "a SHACL instance of
/// sh:NodeShape or sh:PropertyShape".
#[test]
fn a_property_shape_that_is_a_class_targets_its_instances() {
    let report = validate(
        "ex:C a sh:PropertyShape, rdfs:Class ; sh:path ex:p ; sh:minCount 1 .",
        "ex:a a ex:C . ex:b a ex:C ; ex:p 1 .",
    );
    assert_eq!(focus_nodes(&report), vec![ex("a")]);
}

/// A shape that is NOT a class gets no implicit target, however its instances are
/// typed in the data: the control is the same shape typed `rdfs:Class`.
#[test]
fn a_shape_that_is_not_a_class_gets_no_implicit_target() {
    let instances = "ex:a a ex:C .";
    let plain = validate("ex:C a sh:NodeShape ; sh:nodeKind sh:Literal .", instances);
    assert!(plain.results.is_empty());
    let class = validate(
        "ex:C a sh:NodeShape, rdfs:Class ; sh:nodeKind sh:Literal .",
        instances,
    );
    assert_eq!(focus_nodes(&class), vec![ex("a")]);
}

/// The class must be one IN THE SHAPES GRAPH ("a SHACL instance of rdfs:Class in
/// SG"): a data graph that declares the shape's node a class does not give the
/// shape a target. The control declares it in the shapes graph.
#[test]
fn a_class_declared_only_in_the_data_graph_triggers_no_implicit_target() {
    let data_says_class = validate(
        "ex:C a sh:NodeShape ; sh:nodeKind sh:Literal .",
        "ex:C a rdfs:Class . ex:a a ex:C .",
    );
    assert!(data_says_class.results.is_empty());
    let shapes_say_class = validate(
        "ex:C a sh:NodeShape, rdfs:Class ; sh:nodeKind sh:Literal .",
        "ex:a a ex:C .",
    );
    assert_eq!(focus_nodes(&shapes_say_class), vec![ex("a")]);
}

/// A shape that is a class but is not a SHACL instance of `sh:NodeShape` or
/// `sh:PropertyShape` — a shape only because it carries a parameter — gets no
/// implicit target: the textual definition requires both.
#[test]
fn a_class_that_is_a_shape_only_by_its_parameters_gets_no_implicit_target() {
    let untyped = validate(
        "ex:C a rdfs:Class ; sh:nodeKind sh:Literal .",
        "ex:a a ex:C .",
    );
    assert!(untyped.results.is_empty());
    let typed = validate(
        "ex:C a rdfs:Class, sh:NodeShape ; sh:nodeKind sh:Literal .",
        "ex:a a ex:C .",
    );
    assert_eq!(focus_nodes(&typed), vec![ex("a")]);
}

/// `sh:ShapeClass` needs no vocabulary merge: the shapes graph states only
/// `ex:C a sh:ShapeClass`, and its subclasses' instances are targets too — the
/// data graph's `rdfs:subClassOf` is followed, as `sh:targetClass` follows it.
#[test]
fn a_shape_class_targets_the_instances_of_its_subclasses() {
    let report = validate(
        "ex:C a sh:ShapeClass ; sh:nodeKind sh:Literal .",
        "ex:Sub rdfs:subClassOf ex:C . ex:a a ex:Sub . ex:b a ex:Unrelated .",
    );
    assert_eq!(focus_nodes(&report), vec![ex("a")]);
}

// ── Explicit shape targets (sh:shape) ──────────────────────────────────────────

/// "If s is a shape in a shapes graph and n is a node in the data graph. If n has
/// value s for sh:shape in the data graph, then n is a target for s." The control
/// data graph carries no `sh:shape` statement.
#[test]
fn a_data_graph_sh_shape_statement_makes_its_subject_a_focus_node() {
    let shape = "ex:S a sh:NodeShape ; sh:nodeKind sh:Literal .";
    let declared = validate(shape, "ex:a sh:shape ex:S . ex:b ex:p 1 .");
    assert_eq!(focus_nodes(&declared), vec![ex("a")]);
    let undeclared = validate(shape, "ex:a ex:p 1 . ex:b ex:p 1 .");
    assert!(undeclared.results.is_empty());
}

/// ANY shape can be named — "If s is a shape in a shapes graph" — including a
/// property shape reached only through `sh:property` and a shape reached only
/// through `sh:node`, neither of which declares a target. Each result names the
/// shape that was named. The control gives the named node the value the property
/// shape requires, and conforms.
#[test]
fn a_sh_shape_statement_can_name_a_shape_reached_only_through_another() {
    let shapes_ttl = "ex:Parent a sh:NodeShape ; sh:property ex:NameShape ; sh:node ex:Inner .
         ex:NameShape sh:path ex:name ; sh:minCount 1 .
         ex:Inner sh:nodeKind sh:Literal .";
    let named = validate(
        shapes_ttl,
        "ex:a sh:shape ex:NameShape . ex:b sh:shape ex:Inner .",
    );
    let mut rows: Vec<(String, String)> = named
        .results
        .iter()
        .map(|result| {
            (
                result.focus_node.to_string(),
                result.source_shape.to_string(),
            )
        })
        .collect();
    rows.sort();
    assert_eq!(
        rows,
        vec![(ex("a"), ex("NameShape")), (ex("b"), ex("Inner"))]
    );
    let satisfied = validate(shapes_ttl, "ex:a sh:shape ex:NameShape ; ex:name \"a\" .");
    assert!(satisfied.conforms && satisfied.results.is_empty());
}

/// A data graph can name a BLANK shape only when it is the shapes graph itself —
/// and then the `sh:shape` statement is in the shapes graph too, which is what
/// makes that blank shape top-level. The control is the same graph without the
/// statement: the blank shape is then not top-level at all.
#[test]
fn a_sh_shape_statement_in_a_shared_graph_can_name_a_blank_shape() {
    let named = data(
        "ex:Outer a sh:NodeShape ; sh:node _:inner .
         _:inner sh:nodeKind sh:Literal .
         ex:a sh:shape _:inner .",
    );
    let parsed = purrdf_shapes::shapes::from_dataset(&named).expect("the shared graph loads");
    let report =
        validate_dataset_with_shapes_graph(&named, &parsed, None).expect("validation runs");
    assert_eq!(focus_nodes(&report), vec![ex("a")]);
    let unnamed = data(
        "ex:Outer a sh:NodeShape ; sh:node _:inner .
         _:inner sh:nodeKind sh:Literal .",
    );
    let parsed = purrdf_shapes::shapes::from_dataset(&unnamed).expect("the shared graph loads");
    assert_eq!(
        parsed
            .node_shapes
            .iter()
            .map(|shape| shape.id.to_string())
            .collect::<Vec<_>>(),
        vec![ex("Outer")]
    );
}

/// The explicit shape targets UNION with the shape's own declarations ("The target
/// of a shape is the union of all RDF terms produced by the individual targets").
#[test]
fn explicit_shape_targets_union_with_declared_targets() {
    let report = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:b ; sh:nodeKind sh:Literal .",
        "ex:a sh:shape ex:S .",
    );
    assert_eq!(focus_nodes(&report), vec![ex("a"), ex("b")]);
}

/// A `sh:shape` statement naming a term the shapes graph does not hold as a shape
/// targets nothing — the rule applies only "If s is a shape in a shapes graph" —
/// and is not an error. The control names the shape.
#[test]
fn a_sh_shape_statement_naming_no_shape_targets_nothing() {
    let shape = "ex:S a sh:NodeShape ; sh:nodeKind sh:Literal .";
    let unknown = validate(shape, "ex:a sh:shape ex:NotAShape .");
    assert!(unknown.conforms && unknown.results.is_empty());
    let known = validate(shape, "ex:a sh:shape ex:S .");
    assert_eq!(focus_nodes(&known), vec![ex("a")]);
}

/// `$targetNodes` of `sh:uniqueValuesFor` is "the target nodes of S", so a shape
/// with no target declarations still compares the nodes the data graph names it
/// for. The control gives the two nodes different values.
#[test]
fn unique_values_for_compares_the_explicit_shape_targets() {
    let shape = "ex:S a sh:NodeShape ; sh:uniqueValuesFor ex:id .";
    let colliding = validate(
        shape,
        "ex:a sh:shape ex:S ; ex:id 1 . ex:b sh:shape ex:S ; ex:id 1 .",
    );
    assert_eq!(focus_nodes(&colliding), vec![ex("a"), ex("b")]);
    let distinct = validate(
        shape,
        "ex:a sh:shape ex:S ; ex:id 1 . ex:b sh:shape ex:S ; ex:id 2 .",
    );
    assert!(distinct.results.is_empty());
}

/// A rule's focus nodes are its shape's targets, so an explicit shape target fires
/// the rule. The control data graph carries no `sh:shape` statement.
#[test]
fn a_rule_fires_at_an_explicit_shape_target() {
    let shapes = shapes(
        "ex:S a sh:NodeShape ;
           sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:q ;
                     sh:object ex:v ] .",
    );
    let entail = |data_ttl: &str| {
        let entailed =
            purrdf_shapes::entail_dataset(data(data_ttl).as_ref(), &shapes).expect("rules run");
        purrdf::canonicalize(entailed.as_ref()).nquads
    };
    let fired = "<http://example.org/ns#a> <http://example.org/ns#q> <http://example.org/ns#v>";
    assert!(entail("ex:a sh:shape ex:S .").contains(fired));
    assert!(!entail("ex:a ex:p ex:S .").contains(fired));
}

// ── Where targets (sh:targetWhere) ─────────────────────────────────────────────

/// "the set of nodes in a data graph DG that conform to w" — the NODES of the
/// data graph, which RDF 1.2 Concepts defines as "the set of subjects and objects
/// of the asserted triples of the graph". A triple term that is an object is a
/// node, and is targeted when it conforms; a term that occurs only INSIDE a triple
/// term is not a node and is never a candidate, however well it would conform; a
/// predicate is not a node either.
#[test]
fn a_where_target_ranges_over_the_nodes_of_the_data_graph_triple_terms_included() {
    let data_ttl = "ex:r ex:says <<( ex:x ex:p ex:y )>> .";
    // Every triple-term node conforms, and the shape rejects triple terms.
    let triple_terms = validate(
        "ex:S a sh:NodeShape ; sh:targetWhere [ sh:nodeKind sh:TripleTerm ] ;
           sh:nodeKind sh:IRI .",
        data_ttl,
    );
    assert_eq!(
        focus_nodes(&triple_terms),
        vec![
            "<<( <http://example.org/ns#x> <http://example.org/ns#p> \
             <http://example.org/ns#y> )>>"
                .to_owned()
        ]
    );
    // Every IRI node conforms: only `ex:r` is one. `ex:x`, `ex:p`, `ex:y` occur only
    // inside the triple term, and `ex:says` only as a predicate.
    let iris = validate(
        "ex:S a sh:NodeShape ; sh:targetWhere [ sh:nodeKind sh:IRI ] ;
           sh:nodeKind sh:Literal .",
        data_ttl,
    );
    assert_eq!(focus_nodes(&iris), vec![ex("r")]);
}

/// A where shape whose constraint cannot make a node non-conforming — its severity
/// is `sh:Debug`, outside the default conformance-disallow set — narrows nothing:
/// every node conforms and is targeted. The control grades the same constraint
/// `sh:Violation`, and only the class's instances conform.
#[test]
fn a_where_constraint_that_does_not_count_against_conformance_narrows_nothing() {
    let data_ttl = "ex:a a ex:C . ex:b ex:p ex:c .";
    let debug = validate(
        "ex:S a sh:NodeShape ;
           sh:targetWhere [ sh:class ex:C ; sh:severity sh:Debug ] ;
           sh:nodeKind sh:Literal .",
        data_ttl,
    );
    assert_eq!(
        focus_nodes(&debug),
        vec![ex("C"), ex("a"), ex("b"), ex("c")]
    );
    let violation = validate(
        "ex:S a sh:NodeShape ;
           sh:targetWhere [ sh:class ex:C ; sh:severity sh:Violation ] ;
           sh:nodeKind sh:Literal .",
        data_ttl,
    );
    assert_eq!(focus_nodes(&violation), vec![ex("a")]);
}

/// A where shape with a required property narrows to that property's subjects,
/// and the answer is the full scan's: the candidates are re-checked against the
/// whole shape, so `ex:b`, which has the property but fails its bound, is not
/// targeted. The control wraps the same property shape in a one-member `sh:or`,
/// which means the same thing and which the narrowing does not look inside, so it
/// scans every node; both select `ex:a` alone.
#[test]
fn a_narrowed_where_target_agrees_with_the_full_scan() {
    let data_ttl = "ex:a ex:age 20 . ex:b ex:age 10 . ex:c ex:name \"c\" .";
    let narrowed = validate(
        "ex:S a sh:NodeShape ;
           sh:targetWhere [ sh:property [ sh:path ex:age ; sh:minCount 1 ;
                                          sh:minInclusive 18 ] ] ;
           sh:nodeKind sh:Literal .",
        data_ttl,
    );
    let scanned = validate(
        "ex:S a sh:NodeShape ;
           sh:targetWhere [ sh:or ( [ sh:property [ sh:path ex:age ; sh:minCount 1 ;
                                                    sh:minInclusive 18 ] ] ) ] ;
           sh:nodeKind sh:Literal .",
        data_ttl,
    );
    assert_eq!(focus_nodes(&narrowed), vec![ex("a")]);
    assert_eq!(focus_nodes(&scanned), vec![ex("a")]);
}

// ── Node-expression sh:targetNode ──────────────────────────────────────────────

/// A SPARQL node expression as `sh:targetNode`: its output nodes are the targets.
/// The control is the same shape with the constant `sh:targetNode` it would
/// otherwise have to be written with.
#[test]
fn a_select_node_expression_target_selects_its_output_nodes() {
    let data_ttl = "ex:a ex:age 12 . ex:b ex:age 40 .";
    let selected = validate(
        "ex:S a sh:NodeShape ;
           sh:targetNode [ sh:select \"\"\"
               SELECT ?n WHERE { ?n <http://example.org/ns#age> ?age FILTER (?age < 18) }
           \"\"\" ] ;
           sh:nodeKind sh:Literal .",
        data_ttl,
    );
    assert_eq!(focus_nodes(&selected), vec![ex("a")]);
    let constant = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:b ; sh:nodeKind sh:Literal .",
        data_ttl,
    );
    assert_eq!(focus_nodes(&constant), vec![ex("b")]);
}

/// A node-expression target is parsed as a node expression: an unparsable SPARQL
/// query is refused at load rather than read as a target set. The control is the
/// same query well-formed, which loads and targets its output.
#[test]
fn an_ill_formed_node_expression_target_is_refused_and_the_well_formed_one_targets() {
    let error = parse_shapes(
        &format!(
            "{PREFIXES}ex:S a sh:NodeShape ;
               sh:targetNode [ sh:select \"SELECT ?n WHERE {{ ?n ?p }}\" ] ;
               sh:nodeKind sh:Literal ."
        ),
        None,
    )
    .expect_err("an unparsable SPARQL target expression is refused")
    .to_string();
    assert!(error.contains("sh:targetNode"), "{error}");
    let report = validate(
        "ex:S a sh:NodeShape ;
           sh:targetNode [ sh:select \"SELECT ?n WHERE { ?n ?p ?o }\" ] ;
           sh:nodeKind sh:Literal .",
        "ex:a ex:p 1 .",
    );
    assert_eq!(focus_nodes(&report), vec![ex("a")]);
}

/// A function-call node expression as `sh:targetNode` evaluates like any other:
/// `sparql:iri("…")` outputs the IRI, which is the target.
#[test]
fn a_function_call_target_node_targets_its_output() {
    let report = validate(
        "ex:S a sh:NodeShape ;
           sh:targetNode [ sparql:iri ( \"http://example.org/ns#a\" ) ] ;
           sh:nodeKind sh:Literal .",
        "ex:b ex:p 1 .",
    );
    assert_eq!(focus_nodes(&report), vec![ex("a")]);
}

// ── The shapes-graph reports see inside targets ────────────────────────────────

/// `Shapes::function_resolution` reports the call sites a target carries: the
/// call inside a node-expression `sh:targetNode`, and the call inside a
/// `sh:targetWhere` shape's `sh:expression`. The control declares the same shape
/// with constant targets and reports neither.
#[test]
fn function_resolution_reports_the_calls_inside_targets() {
    let evaluated = shapes(
        "ex:S a sh:NodeShape ;
           sh:targetNode [ sparql:iri ( \"http://example.org/ns#a\" ) ] ;
           sh:targetWhere [ sh:expression [ sparql:isIRI ( sh:this ) ] ] ;
           sh:nodeKind sh:Literal .",
    );
    let report = evaluated.function_resolution();
    let owners = |function: &str| -> Vec<String> {
        report
            .sites()
            .filter(|site| site.function == function)
            .map(|site| site.owner.clone())
            .collect()
    };
    assert_eq!(
        owners("http://www.w3.org/ns/sparql#iri"),
        vec![format!("sh:targetNode on {}", ex("S"))]
    );
    assert_eq!(owners("http://www.w3.org/ns/sparql#isIRI").len(), 1);
    let constant = shapes("ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal .");
    assert_eq!(constant.function_resolution().sites().count(), 0);
}

/// `Shapes::extension_usage` reads the SPARQL a target carries: a `sh:select`
/// node expression as `sh:targetNode` names its predicate as a data edge under an
/// empty environment. The control, with a constant target, names nothing.
#[test]
fn extension_usage_reads_the_sparql_inside_a_target_node_expression() {
    let relation = "http://example.org/ns#related";
    let evaluated = shapes(
        "ex:S a sh:NodeShape ;
           sh:targetNode [ sh:select \"SELECT ?n WHERE { ?n <http://example.org/ns#related> ?o }\" ] ;
           sh:nodeKind sh:Literal .",
    );
    let usage = evaluated.extension_usage(purrdf_sparql_eval::ExtensionEnv::empty());
    assert!(usage.data().contains(relation), "{usage:?}");
    let constant = shapes("ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal .");
    let usage = constant.extension_usage(purrdf_sparql_eval::ExtensionEnv::empty());
    assert!(!usage.data().contains(relation), "{usage:?}");
}
