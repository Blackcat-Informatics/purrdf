// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A node expression as the value of `sh:deactivated`, against SHACL 1.2 Core,
//! "Deactivating Shapes and Constraints":
//!
//! > The value of `sh:deactivated` is a node expression that must have either `true`
//! > or `false` as the (only) output node. Let `expr` be the value of `sh:deactivated`
//! > in a shape. If `evalExpr(expr, data graph, focus node, {})` produces `true` as its
//! > only output node, the shape is called deactivated. Deactivated shapes are ignored
//! > during validation.
//!
//! The expression is evaluated at the focus node, so one shape is deactivated at some
//! focus nodes and active at others. Every fixture validates three instances of
//! `ex:T` against a shape whose `sh:datatype xsd:integer` constraint every instance
//! violates, so the OBSERVABLE is which instances are reported: the active ones.
//! Each treatment sits beside a control whose outcome differs from it.

#[path = "support/turtle.rs"]
mod turtle;

use purrdf_shapes::engine::{parse_shapes, validate_graphs};

const PREFIXES: &str = "
@prefix ex:    <http://example.org/> .
@prefix rdf:   <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix sh:    <http://www.w3.org/ns/shacl#> .
@prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .
@prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .
";

/// `ex:muted` is `true` at `ex:a`, `false` at `ex:b`, `\"yes\"` at `ex:c`, two values at
/// `ex:d` and absent at `ex:e`. Every one is an `ex:T`.
const DATA: &str = r#"
<http://example.org/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/T> .
<http://example.org/a> <http://example.org/muted> "true"^^<http://www.w3.org/2001/XMLSchema#boolean> .
<http://example.org/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/T> .
<http://example.org/b> <http://example.org/muted> "false"^^<http://www.w3.org/2001/XMLSchema#boolean> .
<http://example.org/c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/T> .
<http://example.org/c> <http://example.org/muted> "yes" .
<http://example.org/d> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/T> .
<http://example.org/d> <http://example.org/muted> "true"^^<http://www.w3.org/2001/XMLSchema#boolean> .
<http://example.org/d> <http://example.org/muted> "false"^^<http://www.w3.org/2001/XMLSchema#boolean> .
<http://example.org/e> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/T> .
"#;

/// The instances of `ex:T` named by `focus`, as a data graph.
fn data_of(focus: &[&str]) -> String {
    DATA.lines()
        .filter(|line| {
            focus
                .iter()
                .any(|name| line.starts_with(&format!("<http://example.org/{name}>")))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A node shape over `ex:T` with the datatype constraint every instance violates and
/// `deactivated` as the value of `sh:deactivated`.
fn node_shape(deactivated: &str) -> String {
    format!(
        "{PREFIXES}
         ex:S a sh:NodeShape ; sh:targetClass ex:T ; sh:datatype xsd:integer ;
              sh:deactivated {deactivated} ."
    )
}

/// A property shape over `ex:T` whose `sh:datatype xsd:integer` constraint `ex:T`
/// instances violate, with `deactivated` as the value of its `sh:deactivated`.
fn property_shape(deactivated: &str) -> String {
    format!(
        "{PREFIXES}
         ex:S a sh:NodeShape ; sh:targetClass ex:T ;
              sh:property [ sh:path rdf:type ; sh:datatype xsd:integer ;
                            sh:deactivated {deactivated} ] ."
    )
}

/// The focus nodes of the results of validating `data` against `shapes`, sorted.
fn reported(data: &str, shapes: &str) -> Vec<String> {
    let report = validate_graphs(data, shapes, None).expect("the validation runs");
    let mut focus: Vec<String> = report
        .results
        .iter()
        .map(|result| result.focus_node.to_string())
        .collect();
    focus.sort();
    assert_eq!(report.conforms, focus.is_empty());
    focus
}

fn failure(data: &str, shapes: &str) -> String {
    validate_graphs(data, shapes, None)
        .expect_err("the validation must fail")
        .to_string()
}

fn iri(name: &str) -> String {
    format!("<http://example.org/{name}>")
}

/// A way to write a shapes graph with the given value of `sh:deactivated`.
type Writer = fn(&str) -> String;

const EXPRESSIONS: [&str; 2] = [
    "[ sh:path ex:muted ]",
    "[ sh:select \"SELECT ?m WHERE { $this <http://example.org/muted> ?m }\" ]",
];

/// The expression is evaluated at each focus node: `ex:a` is deactivated and `ex:b`
/// is not, under a path expression and under a SPARQL-based one alike. The neighbouring
/// constant spellings give the two outcomes at every focus node.
#[test]
fn a_node_expression_deactivates_the_shape_at_the_focus_nodes_where_it_yields_true() {
    let data = data_of(&["a", "b"]);
    for expression in EXPRESSIONS {
        assert_eq!(
            reported(&data, &node_shape(expression)),
            [iri("b")],
            "{expression}: deactivated at ex:a only"
        );
    }
    assert_eq!(
        reported(&data, &node_shape("true")),
        Vec::<String>::new(),
        "constant true deactivates the shape everywhere"
    );
    assert_eq!(
        reported(&data, &node_shape("false")),
        [iri("a"), iri("b")],
        "constant false deactivates it nowhere"
    );
    assert_eq!(
        reported(
            &data,
            &node_shape("[ shnex:if true ; shnex:then true ; shnex:else false ]")
        ),
        Vec::<String>::new(),
        "a focus-independent expression yielding true deactivates the shape everywhere"
    );
}

/// The same holds for a property shape, which is evaluated at the focus node of the
/// shape it belongs to.
#[test]
fn a_node_expression_deactivates_a_property_shape_at_the_focus_nodes_where_it_yields_true() {
    let data = data_of(&["a", "b"]);
    for expression in EXPRESSIONS {
        assert_eq!(
            reported(&data, &property_shape(expression)),
            [iri("b")],
            "{expression}: deactivated at ex:a only"
        );
    }
    assert_eq!(
        reported(&data, &property_shape("false")),
        [iri("a"), iri("b")]
    );
}

/// A deactivated shape reached through `sh:node` is ignored there too: the value node
/// at which the expression yields `true` conforms, the other does not.
#[test]
fn a_shape_deactivated_by_an_expression_is_ignored_where_another_shape_refers_to_it() {
    let shapes = format!(
        "{PREFIXES}
         ex:Outer a sh:NodeShape ; sh:targetClass ex:T ; sh:node ex:Inner .
         ex:Inner a sh:NodeShape ; sh:datatype xsd:integer ;
              sh:deactivated [ sh:path ex:muted ] ."
    );
    let reported = reported(&data_of(&["a", "b"]), &shapes);
    assert!(
        reported.iter().all(|focus| *focus == iri("b")) && !reported.is_empty(),
        "only ex:b is reported: {reported:?}"
    );
}

/// "Must have either true or false as the (only) output node": an expression that
/// yields anything else at a focus node makes the shapes graph ill-formed there, which
/// is reported as a failure, never read as an active shape. The neighbours yield one
/// boolean at their focus nodes and validate.
#[test]
fn an_expression_that_does_not_yield_one_boolean_is_a_failure() {
    let shapes = node_shape("[ sh:path ex:muted ]");
    assert!(failure(&data_of(&["c"]), &shapes).contains("is no xsd:boolean literal"));
    assert!(failure(&data_of(&["d"]), &shapes).contains("produced 2 output nodes"));
    assert!(failure(&data_of(&["e"]), &shapes).contains("produced 0 output nodes"));
    // One boolean at every focus node: no failure.
    assert_eq!(reported(&data_of(&["a", "b"]), &shapes), [iri("b")]);
    // The same failure is reached through a property shape.
    let property = property_shape("[ sh:path ex:muted ]");
    assert!(failure(&data_of(&["c"]), &property).contains("is no xsd:boolean literal"));
    assert_eq!(reported(&data_of(&["a", "b"]), &property), [iri("b")]);
}

/// A constant node expression produces itself, so only a boolean literal can satisfy
/// the rule; an IRI, another literal and the empty expression cannot, and are refused
/// where the shapes graph is read. The neighbours, `true` and `false`, load.
#[test]
fn a_constant_value_that_is_no_boolean_is_refused_at_load() {
    for value in ["ex:Other", "\"yes\"", "[]"] {
        let error = turtle::refusal_text("", &node_shape(value));
        assert!(error.contains("sh:deactivated"), "{value}: {error}");
        let error = turtle::refusal_text("", &property_shape(value));
        assert!(error.contains("sh:deactivated"), "{value}: {error}");
    }
    for value in ["true", "false"] {
        parse_shapes(&node_shape(value), None).expect("a boolean literal loads");
        parse_shapes(&property_shape(value), None).expect("a boolean literal loads");
    }
}

/// A JSON Schema cannot state a condition on the focus node, so a shape whose
/// `sh:deactivated` is a node expression is projected as active, and the loss is
/// recorded on the shape. The neighbours state their deactivation as a constant: `true`
/// drops the shape's `$def` and `false` keeps it, and neither records anything.
#[test]
fn a_json_schema_records_a_node_expression_deactivation_as_a_loss() {
    use purrdf_rdf::loss::{assert_ledger_complete, assert_ledger_sound};
    use purrdf_shapes::json_schema::{Namespaces, compile};
    use purrdf_shapes::shapes::from_dataset;
    use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

    let compiled = |deactivated: &str| {
        let dataset = parse_turtle_to_dataset(&node_shape(deactivated), None).expect("parse");
        let shapes = from_dataset(&dataset).expect("shapes");
        let ns = Namespaces::new("ex", &[("ex".to_owned(), "http://example.org/".to_owned())])
            .expect("namespaces");
        compile(&shapes, &ns).expect("compile")
    };
    let projected = compiled("[ sh:path ex:muted ]");
    assert_ledger_complete(&projected.losses, &["sh:deactivated"]);
    assert_ledger_sound(&projected.losses, "shacl", "json-schema");
    assert!(
        projected.schema_json.contains("\"T\""),
        "the shape is projected as active, so its class has a $def: {}",
        projected.schema_json
    );
    let constant_true = compiled("true");
    assert!(constant_true.losses.is_empty());
    assert!(!constant_true.schema_json.contains("\"T\""));
    let constant_false = compiled("false");
    assert!(constant_false.losses.is_empty());
    assert!(constant_false.schema_json.contains("\"T\""));
}

// ── The same expression on a constraint ───────────────────────────────────────

/// A node shape over `ex:T` whose one SPARQL-based constraint reports every focus
/// node, with `deactivated` as the `sh:deactivated` of the constraint node.
fn sparql_constraint(deactivated: &str) -> String {
    format!(
        "{PREFIXES}
         ex:S a sh:NodeShape ; sh:targetClass ex:T ; sh:sparql [
             sh:deactivated {deactivated} ;
             sh:select \"SELECT $this WHERE {{ }}\" ] ."
    )
}

/// SHACL 1.2 SPARQL Extensions, "Syntax of SPARQL-based Constraints": the value of a
/// SPARQL-based constraint's `sh:deactivated` "is a node expression that must have
/// either `true` or `false` as its (only) output node, as for shapes", and "Validation
/// with SPARQL-based Constraints": the constraint is deactivated, with no validation
/// results, when `evalExpr(expr, data graph, focus node, {})` "produces `true` as its
/// only output node". So the constraint reports `ex:b` and not `ex:a`; the constants
/// give every focus node or none, and an expression that is no one boolean fails.
#[test]
fn a_node_expression_deactivates_a_sparql_based_constraint_where_it_yields_true() {
    let data = data_of(&["a", "b"]);
    for expression in EXPRESSIONS {
        assert_eq!(
            reported(&data, &sparql_constraint(expression)),
            [iri("b")],
            "{expression}"
        );
    }
    assert_eq!(
        reported(&data, &sparql_constraint("true")),
        Vec::<String>::new()
    );
    assert_eq!(
        reported(&data, &sparql_constraint("false")),
        [iri("a"), iri("b")]
    );
    let shapes = sparql_constraint("[ sh:path ex:muted ]");
    assert!(failure(&data_of(&["c"]), &shapes).contains("is no xsd:boolean literal"));
    assert!(failure(&data_of(&["d"]), &shapes).contains("produced 2 output nodes"));
    assert!(failure(&data_of(&["e"]), &shapes).contains("produced 0 output nodes"));
}

/// The same holds for an `sh:expression` constraint node, which SHACL 1.2 treats as a
/// SPARQL-based constraint does (its `sh:deactivated` is read from the node).
#[test]
fn a_node_expression_deactivates_an_expression_constraint_where_it_yields_true() {
    let shapes = |deactivated: &str| {
        format!(
            "{PREFIXES}
             ex:S a sh:NodeShape ; sh:targetClass ex:T ; sh:expression [
                 sh:deactivated {deactivated} ; sh:path ex:never ] ."
        )
    };
    let data = data_of(&["a", "b"]);
    assert_eq!(reported(&data, &shapes("[ sh:path ex:muted ]")), [iri("b")]);
    assert_eq!(reported(&data, &shapes("true")), Vec::<String>::new());
    assert_eq!(reported(&data, &shapes("false")), [iri("a"), iri("b")]);
}

/// A reifier's `sh:deactivated` is a node expression too. Core, "Deactivating Shapes
/// and Constraints": "Let `expr` be the value of `sh:deactivated` in a reifier on a
/// triple that has shape subject and a parameter as predicate. If `evalExpr(expr, data
/// graph, focus node, {})` produces `true` as its only output node, the constraints
/// that use the triple are called deactivated constraints." It is read on a node
/// shape's constraint, a property shape's, the `sh:property` statement that reaches a
/// property shape, a SPARQL-based constraint, and `sh:reificationRequired`.
#[test]
fn a_reifier_node_expression_deactivates_the_constraint_where_it_yields_true() {
    let muted = "{| sh:deactivated [ sh:path ex:muted ] |}";
    let constants = |reifier: &str| reifier.replace("[ sh:path ex:muted ]", "true");
    let data = data_of(&["a", "b"]);
    let both = [iri("a"), iri("b")];
    let cases: [(&str, String); 5] = [
        (
            "a node shape's constraint",
            format!(
                "{PREFIXES} ex:S a sh:NodeShape ; sh:targetClass ex:T ;
                 sh:datatype xsd:integer MUTED ."
            ),
        ),
        (
            "a property shape's constraint",
            format!(
                "{PREFIXES} ex:S a sh:NodeShape ; sh:targetClass ex:T ;
                 sh:property [ sh:path rdf:type ; sh:datatype xsd:integer MUTED ] ."
            ),
        ),
        (
            "the sh:property statement",
            format!(
                "{PREFIXES} ex:S a sh:NodeShape ; sh:targetClass ex:T ; sh:property ex:P MUTED .
                 ex:P sh:path rdf:type ; sh:datatype xsd:integer ."
            ),
        ),
        (
            "a SPARQL-based constraint",
            format!(
                "{PREFIXES} ex:S a sh:NodeShape ; sh:targetClass ex:T ; sh:sparql ex:C MUTED .
                 ex:C sh:select \"SELECT $this WHERE {{ }}\" ."
            ),
        ),
        (
            "sh:reificationRequired",
            format!(
                "{PREFIXES} ex:S a sh:NodeShape ; sh:targetClass ex:T ;
                 sh:property [ sh:path ex:muted ; sh:reificationRequired true MUTED ] ."
            ),
        ),
    ];
    for (what, template) in cases {
        let with = |reifier: &str| template.replace("MUTED", reifier);
        assert_eq!(
            reported(&data, &with(muted)),
            [iri("b")],
            "{what}: deactivated at ex:a only"
        );
        assert_eq!(
            reported(&data, &with(&constants(muted))),
            Vec::<String>::new(),
            "{what}: constant true"
        );
        assert_eq!(
            reported(
                &data,
                &with(&muted.replace("[ sh:path ex:muted ]", "false"))
            ),
            both,
            "{what}: constant false"
        );
        assert!(
            failure(&data_of(&["c"]), &with(muted)).contains("is no xsd:boolean literal"),
            "{what}"
        );
    }
}

// ── The 2017 Recommendation ───────────────────────────────────────────────────

/// SHACL REC 2017-07-20, "Deactivating Shapes": "The value of `sh:deactivated` in a
/// shape must be either `true` or `false`." Under that law a node expression is
/// refused at load wherever it is written (a shape, a SPARQL-based constraint, a
/// reifier), and the boolean spelling is accepted. The dated 1.2 profile accepts the
/// same expression, which is the neighbour that shows the refusal is the law's.
#[test]
fn the_2017_recommendation_allows_only_a_boolean_sh_deactivated() {
    use purrdf_shapes::engine::{ValidationOptions, parse_shapes_with_options};
    use purrdf_shapes::{ShaclProfile, ShapesImports};

    let load = |shapes: &str, profile: ShaclProfile| {
        parse_shapes_with_options(
            shapes,
            None,
            None,
            None,
            &ShapesImports::new(),
            &ValidationOptions::default().with_profile(profile),
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
    };
    let written_at: [(&str, Writer); 3] = [
        ("a shape", node_shape),
        ("a SPARQL-based constraint", sparql_constraint),
        ("a reifier", |value| {
            format!(
                "{PREFIXES} ex:S a sh:NodeShape ; sh:targetClass ex:T ;
                 sh:datatype xsd:integer {{| sh:deactivated {value} |}} ."
            )
        }),
    ];
    for (what, shapes) in written_at {
        let expression = shapes("[ sh:path ex:muted ]");
        let error = load(&expression, ShaclProfile::REC_20170720)
            .expect_err("the 2017 law allows only true and false");
        assert!(error.contains("SHACL REC 2017-07-20"), "{what}: {error}");
        load(&expression, ShaclProfile::WD_20260918)
            .unwrap_or_else(|error| panic!("{what}: the 1.2 law accepts it: {error}"));
        for constant in ["true", "false"] {
            load(&shapes(constant), ShaclProfile::REC_20170720)
                .unwrap_or_else(|error| panic!("{what}: {constant} loads under 2017: {error}"));
        }
    }
}

/// A node expression on a constraint (the constraint node, or a reifier of its
/// statement) is the same loss, recorded on the shape; the constants record nothing.
#[test]
fn a_json_schema_records_a_constraint_s_node_expression_deactivation_as_a_loss() {
    use purrdf_rdf::loss::assert_ledger_complete;
    use purrdf_shapes::json_schema::{Namespaces, compile};
    use purrdf_shapes::shapes::from_dataset;
    use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

    let compiled = |shapes: &str| {
        let dataset = parse_turtle_to_dataset(shapes, None).expect("parse");
        let shapes = from_dataset(&dataset).expect("shapes");
        let ns = Namespaces::new("ex", &[("ex".to_owned(), "http://example.org/".to_owned())])
            .expect("namespaces");
        compile(&shapes, &ns).expect("compile")
    };
    let reifier = |value: &str| {
        format!(
            "{PREFIXES} ex:S a sh:NodeShape ; sh:targetClass ex:T ;
             sh:property [ sh:path ex:muted ; sh:datatype xsd:boolean
                           {{| sh:deactivated {value} |}} ] ."
        )
    };
    assert_ledger_complete(
        &compiled(&reifier("[ sh:path ex:muted ]")).losses,
        &["sh:deactivated"],
    );
    assert!(compiled(&reifier("false")).losses.is_empty());
}
