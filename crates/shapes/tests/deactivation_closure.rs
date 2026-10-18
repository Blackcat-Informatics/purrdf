// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The node-expression `sh:deactivated` rules that do not depend on one shape alone:
//! the set of expressions a validation evaluates is independent of member order and of
//! short-circuiting; a reifier-deactivated `sh:property` edge keeps its path in
//! `sh:closed`'s permitted set; and at most one reifier states `sh:deactivated` for a
//! statement.
//!
//! SHACL 1.2 Core, "Deactivating Shapes and Constraints": the value of `sh:deactivated`
//! "must have either true or false as the (only) output node", and a processor SHOULD
//! produce a failure for an ill-formed shapes graph. An expression that yields no node
//! at a focus node is ill-formed there, so WHETHER the validation fails must not depend
//! on whether the walk happened to reach that expression. Each position of the
//! expression is checked against the verdict of validating the member shape directly
//! (`sh:targetNode`), which walks every constraint and so evaluates every reachable
//! expression.

use purrdf_shapes::engine::validate_graphs;

const PREFIXES: &str = "
@prefix ex:    <http://example.org/> .
@prefix rdf:   <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix sh:    <http://www.w3.org/ns/shacl#> .
@prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .
@prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .
";

const TYPE: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
const BOOLEAN: &str = "<http://www.w3.org/2001/XMLSchema#boolean>";

/// `ex:a` has no `ex:muted`, so `[ sh:path ex:muted ]` yields no node there and the
/// expression is ill-formed; `ex:b` states `ex:muted false`, so the expression is well
/// formed and does not deactivate.
fn data() -> String {
    format!(
        "<http://example.org/a> {TYPE} <http://example.org/T> .\n\
         <http://example.org/b> {TYPE} <http://example.org/T> .\n\
         <http://example.org/b> <http://example.org/muted> \"false\"^^{BOOLEAN} .\n"
    )
}

/// The expression every position states.
const EXPR: &str = "[ sh:path ex:muted ]";

/// The outcome of a validation: `Ok(conforms)` or the failure text.
fn outcome(data: &str, shapes: &str) -> Result<bool, String> {
    validate_graphs(data, shapes, None)
        .map(|report| report.conforms)
        .map_err(|error| error.to_string())
}

/// A member shape `ex:B` that violates at both focus nodes and holds a deactivation
/// expression at one position of its closure. `{EXPR}` is replaced by `expr`.
///
/// Every position lists a violating constraint BEFORE the expression, so a probe that
/// stops at the first violation would never reach it.
fn member(position: &str, expr: &str) -> String {
    let text = match position {
        // (i) on the member shape itself.
        "shape" => "ex:B a sh:NodeShape ; sh:nodeKind sh:BlankNode ; sh:deactivated EXPR .",
        // (ii) on a shape reached through the member's sh:node.
        "node" => {
            "ex:B a sh:NodeShape ; sh:nodeKind sh:BlankNode ; sh:node ex:C .
             ex:C a sh:NodeShape ; sh:datatype xsd:integer ; sh:deactivated EXPR ."
        }
        // (iii) on a property shape of the member.
        "property" => {
            "ex:B a sh:NodeShape ; sh:nodeKind sh:BlankNode ;
               sh:property [ sh:path rdf:type ; sh:datatype xsd:integer ;
                             sh:deactivated EXPR ] ."
        }
        // (iv) on a constraint reifier of the member, listed after violating
        // constraints (constraints are walked in predicate order, and `sh:pattern`
        // follows `sh:class`, `sh:datatype` and `sh:nodeKind`).
        "reifier" => {
            "ex:B a sh:NodeShape ; sh:nodeKind sh:BlankNode ;
               sh:class ex:Nothing ; sh:datatype xsd:integer ;
               sh:pattern \"^zzz\" {| sh:deactivated EXPR |} ."
        }
        // (iv') on the reifier of the member's sh:property statement.
        "edge" => {
            "ex:B a sh:NodeShape ; sh:nodeKind sh:BlankNode ; sh:property ex:P {| sh:deactivated EXPR |} .
             ex:P sh:path rdf:type ; sh:datatype xsd:integer ."
        }
        other => panic!("unknown position {other}"),
    };
    text.replace("EXPR", expr)
}

const POSITIONS: [&str; 5] = ["shape", "node", "property", "reifier", "edge"];

/// `ex:A` is the member that settles each combinator without `ex:B`: it conforms (so
/// `sh:or` stops at it), and for `sh:and` the first member must FAIL to stop it, so
/// `ex:Fail` is used there.
const MEMBERS: &str = "
ex:A a sh:NodeShape ; sh:nodeKind sh:IRI .
ex:Fail a sh:NodeShape ; sh:nodeKind sh:Literal .
";

/// `ex:S` applies `combinator` to the members `first` then `second`.
fn combined(combinator: &str, members: [&str; 2], position: &str, expr: &str) -> String {
    let [first, second] = members;
    // `sh:not` takes one shape: the member under test, whatever the order.
    let constraint = match combinator {
        "not" => "sh:not ex:B".to_owned(),
        other => format!("sh:{other} ( {first} {second} )"),
    };
    format!(
        "{PREFIXES}{MEMBERS}{}
         ex:S a sh:NodeShape ; sh:targetClass ex:T ; {constraint} .",
        member(position, expr)
    )
}

/// `ex:B` validated directly: every constraint is walked, so every expression in its
/// closure is evaluated at the focus node.
fn direct(position: &str, expr: &str) -> String {
    format!(
        "{PREFIXES}{}
         ex:B sh:targetClass ex:T .",
        member(position, expr)
    )
}

/// For every combinator, member order and expression position: the verdict is the
/// same in both orders and equals the verdict of validating the member directly.
/// `ex:a` is the focus node where the expression is ill-formed, so the direct
/// validation is a failure and every order must fail; `ex:b` is well formed and every
/// order must agree on a verdict.
#[test]
fn a_deactivation_expression_fails_the_validation_whatever_the_member_order() {
    let a_only = format!("<http://example.org/a> {TYPE} <http://example.org/T> .\n");
    let all = data();
    for position in POSITIONS {
        // The ill-formed focus node: the direct verdict is a failure.
        let direct_failure = outcome(&a_only, &direct(position, EXPR))
            .expect_err("validating the member directly evaluates its expression and fails");
        assert!(
            direct_failure.contains("sh:deactivated"),
            "{position}: {direct_failure}"
        );
        for combinator in ["or", "and", "xone", "not"] {
            let orders: [[&str; 2]; 2] = if combinator == "and" {
                [["ex:Fail", "ex:B"], ["ex:B", "ex:Fail"]]
            } else {
                [["ex:A", "ex:B"], ["ex:B", "ex:A"]]
            };
            let mut well_formed = Vec::new();
            for members in orders {
                let shapes = combined(combinator, members, position, EXPR);
                let failure = outcome(&a_only, &shapes).expect_err(&format!(
                    "{combinator} {members:?} at {position}: the member's expression is \
                     ill-formed at ex:a and the validation must fail"
                ));
                assert!(
                    failure.contains("sh:deactivated"),
                    "{combinator} {members:?} at {position}: {failure}"
                );
                // A focus node where the expression is well formed validates.
                let b_only = all
                    .lines()
                    .filter(|line| line.starts_with("<http://example.org/b>"))
                    .collect::<Vec<_>>()
                    .join("\n");
                well_formed.push(outcome(&b_only, &shapes).unwrap_or_else(|error| {
                    panic!("{combinator} {members:?} at {position}: {error}")
                }));
            }
            assert_eq!(
                well_formed[0], well_formed[1],
                "{combinator} at {position}: the verdict at a well-formed focus node does not \
                 depend on member order"
            );
        }
        // The same structure with a well-formed focus node validates directly too.
        let b_only = format!(
            "<http://example.org/b> {TYPE} <http://example.org/T> .\n\
             <http://example.org/b> <http://example.org/muted> \"false\"^^{BOOLEAN} .\n"
        );
        assert_eq!(outcome(&b_only, &direct(position, EXPR)), Ok(false));
    }
}

/// The neighbour: the same structures with the constants `false` and `true` for
/// `sh:deactivated` never fail, in either order, because no expression is evaluated.
#[test]
fn constant_deactivations_never_fail_and_agree_across_member_orders() {
    for position in POSITIONS {
        for constant in ["false", "true"] {
            for combinator in ["or", "and", "xone", "not"] {
                let orders: [[&str; 2]; 2] = if combinator == "and" {
                    [["ex:Fail", "ex:B"], ["ex:B", "ex:Fail"]]
                } else {
                    [["ex:A", "ex:B"], ["ex:B", "ex:A"]]
                };
                let verdicts: Vec<bool> = orders
                    .iter()
                    .map(|members| {
                        outcome(&data(), &combined(combinator, *members, position, constant))
                            .unwrap_or_else(|error| {
                                panic!(
                                    "{combinator} {members:?} at {position} = {constant}: {error}"
                                )
                            })
                    })
                    .collect();
                assert_eq!(
                    verdicts[0], verdicts[1],
                    "{combinator} at {position} = {constant}"
                );
            }
        }
    }
}

// ── sh:closed keeps the path of a reifier-deactivated sh:property edge ─────────

/// The outcome of validating `shapes` against `data_ttl`: the `(sh:resultPath,
/// component)` of every result, sorted.
fn closed_results(data_ttl: &str, shapes: &str) -> Vec<(String, String)> {
    let data =
        purrdf_shapes::text_ingest::parse_turtle_to_dataset(&format!("{PREFIXES}{data_ttl}"), None)
            .expect("data parses");
    let shapes = purrdf_shapes::engine::parse_shapes(&format!("{PREFIXES}{shapes}"), None)
        .unwrap_or_else(|error| panic!("the shapes graph must load: {error}"));
    let report = purrdf_shapes::engine::validate_dataset_with_shapes_graph(&data, &shapes, None)
        .expect("validation runs");
    let mut rows: Vec<(String, String)> = report
        .results
        .iter()
        .map(|result| {
            (
                result
                    .result_path
                    .as_ref()
                    .map_or_else(String::new, ToString::to_string),
                result.source_constraint_component.to_string(),
            )
        })
        .collect();
    rows.sort();
    rows
}

/// The three spellings of "this reach of `ex:P` is deactivated" that all give
/// `true` at every focus node.
const ALWAYS_TRUE: [&str; 2] = [
    "true",
    "[ shnex:if true ; shnex:then true ; shnex:else false ]",
];

/// Core §7.9.1: P is "the set of IRI properties that can be reached from the current
/// shape via the SPARQL path sh:property/sh:path". The path is syntactic, so
/// deactivating the `sh:property` statement keeps `ex:p` permitted, under the constant
/// and under an always-true expression alike; `ex:q` stays a violation; and the
/// property shape's own `sh:deactivated true` gives the same results.
#[test]
fn a_reifier_deactivated_property_edge_keeps_its_path_in_the_closed_set() {
    let closed_path = "<http://example.org/q>";
    let closed_component = "<http://www.w3.org/ns/shacl#ClosedConstraintComponent>";
    let node_shape = |deactivation: &str| {
        format!(
            "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:closed true ;
               sh:ignoredProperties ( rdf:type ) ; sh:property ex:P {deactivation} .
             ex:P sh:path ex:p ; sh:minCount 5 ."
        )
    };
    let own = "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:closed true ;
                 sh:ignoredProperties ( rdf:type ) ; sh:property ex:P .
               ex:P sh:path ex:p ; sh:minCount 5 ; sh:deactivated true .";
    for (data, expected) in [
        ("ex:a ex:p 1 .", Vec::new()),
        (
            "ex:a ex:p 1 ; ex:q 2 .",
            vec![(closed_path.to_owned(), closed_component.to_owned())],
        ),
    ] {
        for value in ALWAYS_TRUE {
            assert_eq!(
                closed_results(
                    data,
                    &node_shape(&format!("{{| sh:deactivated {value} |}}"))
                ),
                expected,
                "node shape closed, reifier value {value}, data {data}"
            );
        }
        assert_eq!(
            closed_results(data, own),
            expected,
            "the property shape's own sh:deactivated true, data {data}"
        );
        // Control: an active edge whose minCount is violated reports it, so the
        // empty results above are not a vacuous pass.
    }
    let active = closed_results("ex:a ex:p 1 .", &node_shape(""));
    assert_eq!(active.len(), 1, "{active:?}");
    assert!(active[0].1.ends_with("MinCountConstraintComponent>"));
    // A reifier that states false leaves the edge active.
    assert_eq!(
        closed_results("ex:a ex:p 1 .", &node_shape("{| sh:deactivated false |}")),
        active
    );
}

/// The same on a property shape: its value nodes are checked against the paths of its
/// own `sh:property` children, and a deactivated child edge still permits its path.
#[test]
fn a_reifier_deactivated_property_edge_keeps_its_path_in_a_property_shapes_closed_set() {
    let closed_component = "<http://www.w3.org/ns/shacl#ClosedConstraintComponent>";
    let shapes = |deactivation: &str| {
        format!(
            "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:property ex:Q .
             ex:Q sh:path ex:address ; sh:closed true ; sh:ignoredProperties ( rdf:type ) ;
               sh:property ex:R {deactivation} .
             ex:R sh:path ex:city ; sh:minCount 5 ."
        )
    };
    let own = "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:property ex:Q .
               ex:Q sh:path ex:address ; sh:closed true ; sh:ignoredProperties ( rdf:type ) ;
                 sh:property ex:R .
               ex:R sh:path ex:city ; sh:minCount 5 ; sh:deactivated true .";
    let data = "ex:a ex:address ex:ad . ex:ad ex:city \"x\" ; ex:zip \"1\" .";
    let expected = vec![(
        "<http://example.org/zip>".to_owned(),
        closed_component.to_owned(),
    )];
    for value in ALWAYS_TRUE {
        assert_eq!(
            closed_results(data, &shapes(&format!("{{| sh:deactivated {value} |}}"))),
            expected,
            "property shape closed, reifier value {value}"
        );
    }
    assert_eq!(
        closed_results(data, own),
        expected,
        "own sh:deactivated true"
    );
    // Control: with the edge active the unmet minCount is reported too.
    let active = closed_results(data, &shapes(""));
    assert_eq!(active.len(), 2, "{active:?}");
}

// ── At most one reifier states sh:deactivated ──────────────────────────────────

fn two_reifiers(first: &str, second: &str) -> String {
    format!(
        "{PREFIXES}
         ex:S a sh:NodeShape ; sh:targetClass ex:T ; sh:datatype xsd:integer .
         ex:r1 rdf:reifies <<( ex:S sh:datatype xsd:integer )>> ; sh:deactivated {first} .
         ex:r2 rdf:reifies <<( ex:S sh:datatype xsd:integer )>> ; sh:deactivated {second} ."
    )
}

/// Core: a triple "can have at most one reifier with a value for the property
/// sh:deactivated". A second reifier is refused whenever either value is a node
/// expression; two reifiers with agreeing booleans keep loading, and disagreeing ones
/// keep their own refusal.
#[test]
fn a_second_reifier_stating_sh_deactivated_is_refused_when_either_value_is_an_expression() {
    let expr1 = "[ sh:path ex:m1 ]";
    let expr2 = "[ sh:path ex:m2 ]";
    for (first, second) in [
        (expr1, expr2),
        (expr1, "false"),
        ("true", expr2),
        (expr1, expr1),
    ] {
        let error = validate_graphs(&data(), &two_reifiers(first, second), None)
            .expect_err(&format!("{first} and {second} on two reifiers are refused"))
            .to_string();
        assert!(
            error.contains("at most one reifier"),
            "{first}, {second}: {error}"
        );
    }
    // The single-reifier neighbour is accepted, with either spelling.
    for single in [expr1, "false"] {
        let shapes = format!(
            "{PREFIXES}
             ex:S a sh:NodeShape ; sh:targetClass ex:T ; sh:datatype xsd:integer .
             ex:r1 rdf:reifies <<( ex:S sh:datatype xsd:integer )>> ; sh:deactivated {single} ."
        );
        // ex:m1 is absent so the expression fails; the load itself must succeed.
        let loaded = purrdf_shapes::engine::parse_shapes(&shapes, None);
        assert!(loaded.is_ok(), "{single}: {:?}", loaded.err());
    }
    // Booleans keep their own check: agreeing values load, disagreeing ones are refused.
    assert!(validate_graphs(&data(), &two_reifiers("true", "true"), None).is_ok());
    let disagreement = validate_graphs(&data(), &two_reifiers("true", "false"), None)
        .expect_err("true and false disagree")
        .to_string();
    assert!(disagreement.contains("both sh:deactivated true and false"));
    // A second reifier that states only another annotation is not a second value.
    let shapes = format!(
        "{PREFIXES}
         ex:S a sh:NodeShape ; sh:targetClass ex:T ; sh:datatype xsd:integer .
         ex:r1 rdf:reifies <<( ex:S sh:datatype xsd:integer )>> ; sh:deactivated {expr1} .
         ex:r2 rdf:reifies <<( ex:S sh:datatype xsd:integer )>> ; sh:severity sh:Warning ."
    );
    assert!(purrdf_shapes::engine::parse_shapes(&shapes, None).is_ok());
}
