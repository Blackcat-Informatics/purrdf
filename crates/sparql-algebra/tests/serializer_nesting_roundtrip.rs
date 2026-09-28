// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Admitted in, admitted out: the text [`pattern_to_select_query`] renders for any
//! algebra the parser admits is admitted again, and re-parses to the same tree.
//!
//! The forwarded text of a `SERVICE` is exactly such a rendering. These tests pin the
//! property three ways: a proptest over generated expression and property-path trees,
//! the named precedence cases (with the brackets they must and must not carry), and,
//! for every construct family that nests, a body written twenty thousand levels deep
//! inside a `SERVICE` (and for every family that chains, one [`CHAIN`] elements long),
//! forwarded and re-parsed on the test thread's own stack — nothing but memory bounds
//! how deep either the parse or the rendering goes.

use proptest::prelude::*;
use purrdf_sparql_algebra::{
    ArithmeticOperator, Child, Expression, Function, GraphPattern, Literal, NamedNode,
    NamedNodePattern, NegatedPathElement, PropertyPathExpression, Query, SparqlParser, TermPattern,
    TriplePattern, Variable, pattern_to_select_query,
};

const EX: &str = "http://example.org/";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// Parse `query`, returning the `SELECT`'s pattern, or the parser's refusal text.
fn try_select(query: &str) -> Result<GraphPattern, String> {
    match SparqlParser::new().parse_query(query) {
        Ok(Query::Select { pattern, .. }) => Ok(pattern),
        Ok(other) => panic!("expected a SELECT, got {other:?}"),
        Err(e) => Err(e.to_string()),
    }
}

/// The WHERE body under the `SELECT` scaffold — the shape a `SERVICE` forwards.
fn unproject(p: GraphPattern) -> GraphPattern {
    match p {
        GraphPattern::Project { inner, .. } => inner.into_inner(),
        other => other,
    }
}

/// Render `body` as the forwarded query, re-parse it, and assert the parser admits
/// it and rebuilds exactly `body`. Returns the rendered text.
fn assert_forwarded_roundtrip(body: &GraphPattern) -> String {
    let text = pattern_to_select_query(body);
    let reparsed = try_select(&text)
        .unwrap_or_else(|e| panic!("the forwarded text is refused: {e}\n text: {text}"));
    assert_eq!(
        &unproject(reparsed),
        body,
        "the forwarded text rebuilt a different tree\n text: {text}"
    );
    text
}

/// `parse(serialize(parse(q)))` for a whole query: returns the forwarded text.
fn assert_query_roundtrip(query: &str) -> String {
    let body = unproject(try_select(query).unwrap_or_else(|e| panic!("`{query}`: {e}")));
    assert_forwarded_roundtrip(&body)
}

/// The FILTER expression of `SELECT * WHERE { ?a <p> ?b FILTER(expr) }`'s rendering.
fn rendered_filter(expr: &str) -> String {
    let text = assert_query_roundtrip(&format!(
        "SELECT * WHERE {{ ?a <{EX}p> ?b FILTER({expr}) }}"
    ));
    let start = text.find("FILTER(").expect("a FILTER is rendered") + "FILTER(".len();
    text[start..text.len() - ") }".len()].to_owned()
}

/// The path of `SELECT * WHERE { ?s PATH ?o }`'s rendering.
fn rendered_path(path: &str) -> String {
    let text = assert_query_roundtrip(&format!("SELECT * WHERE {{ ?s {path} ?o }}"));
    let start = text.find("?s ").expect("the subject is rendered") + "?s ".len();
    let end = text.rfind(" ?o").expect("the object is rendered");
    text[start..end].to_owned()
}

// ── the named precedence cases ──────────────────────────────────────────────

#[test]
fn a_left_associative_chain_is_written_without_brackets() {
    assert_eq!(rendered_filter("?a + ?b + ?c"), "?a + ?b + ?c");
    assert_eq!(rendered_filter("?a - ?b - ?c"), "?a - ?b - ?c");
    assert_eq!(rendered_filter("?a || ?b || ?c"), "?a || ?b || ?c");
    assert_eq!(rendered_filter("?a * ?b / ?c"), "?a * ?b / ?c");
}

#[test]
fn a_tighter_operand_needs_no_bracket_and_a_looser_one_keeps_it() {
    assert_eq!(rendered_filter("?a + ?b * ?c"), "?a + ?b * ?c");
    assert_eq!(rendered_filter("(?a + ?b) * ?c"), "(?a + ?b) * ?c");
    assert_eq!(rendered_filter("?a || ?b && ?c"), "?a || ?b && ?c");
    assert_eq!(rendered_filter("(?a || ?b) && ?c"), "(?a || ?b) && ?c");
    assert_eq!(rendered_filter("?a + ?b < ?c * ?a"), "?a + ?b < ?c * ?a");
}

#[test]
fn a_right_nested_operand_of_the_same_level_keeps_its_bracket() {
    assert_eq!(rendered_filter("?a - (?b - ?c)"), "?a - (?b - ?c)");
    assert_eq!(rendered_filter("?a + (?b + ?c)"), "?a + (?b + ?c)");
    assert_eq!(rendered_filter("?a / (?b * ?c)"), "?a / (?b * ?c)");
    assert_eq!(rendered_filter("?a && (?b && ?c)"), "?a && (?b && ?c)");
}

#[test]
fn a_relational_operand_is_bracketted_on_either_side() {
    // Relational operators do not associate: neither side may be another one.
    assert_eq!(rendered_filter("(?a = ?b) = ?c"), "(?a = ?b) = ?c");
    assert_eq!(rendered_filter("?a = (?b < ?c)"), "?a = (?b < ?c)");
    assert_eq!(rendered_filter("(?a && ?b) = ?c"), "(?a && ?b) = ?c");
}

#[test]
fn unary_operators_bracket_only_a_looser_operand() {
    assert_eq!(rendered_filter("-(?a + ?b)"), "-(?a + ?b)");
    assert_eq!(rendered_filter("!(?a && ?b)"), "!(?a && ?b)");
    assert_eq!(rendered_filter("-?a * ?b"), "-?a * ?b");
    assert_eq!(rendered_filter("-(?a * ?b)"), "-(?a * ?b)");
    assert_eq!(rendered_filter("- - ?a"), "--?a");
    assert_eq!(rendered_filter("!!?a"), "!!?a");
    assert_eq!(rendered_filter("?a - -?b"), "?a - -?b");
}

#[test]
fn negated_forms_are_written_the_way_the_parser_builds_them() {
    // `!=`, `NOT IN` and `NOT EXISTS` are the spellings of `Not(Equal)`,
    // `Not(In)` and `Not(Exists)`; the bracketted `!(…)` forms build the same trees.
    assert_eq!(rendered_filter("?a != ?b"), "?a != ?b");
    assert_eq!(rendered_filter("!(?a = ?b)"), "?a != ?b");
    assert_eq!(rendered_filter("!(?a != ?b)"), "!(?a != ?b)");
    assert_eq!(rendered_filter("?a NOT IN (?b, ?c)"), "?a NOT IN (?b, ?c)");
    assert_eq!(rendered_filter("!(?a IN (?b))"), "?a NOT IN (?b)");
    assert_eq!(rendered_filter("(?a != ?b) * ?c"), "(?a != ?b) * ?c");
}

#[test]
fn in_lists_hold_whole_expressions() {
    assert_eq!(
        rendered_filter("?a + ?b IN (?c || ?a, ?b * ?c, (?a = ?b))"),
        "?a + ?b IN (?c || ?a, ?b * ?c, ?a = ?b)"
    );
    assert_eq!(rendered_filter("(?a IN (?b)) IN ()"), "(?a IN (?b)) IN ()");
}

#[test]
fn exists_nests_inside_expressions() {
    let text = rendered_filter(&format!(
        "?a && EXISTS {{ ?a <{EX}q> ?c FILTER(NOT EXISTS {{ ?c <{EX}r> ?a }} || ?c > ?a) }}"
    ));
    assert!(text.starts_with("?a && EXISTS { "), "{text}");
    assert!(text.contains("NOT EXISTS { "), "{text}");
    assert!(text.contains("FILTER(NOT EXISTS"), "{text}");
    assert_eq!(
        rendered_filter(&format!("-(?a) * !EXISTS {{ ?a <{EX}q> ?b }}"))
            .matches('(')
            .count(),
        0
    );
}

#[test]
fn property_paths_bracket_only_where_the_grammar_needs_it() {
    let (a, b, c) = (format!("<{EX}a>"), format!("<{EX}b>"), format!("<{EX}c>"));
    assert_eq!(
        rendered_path(&format!("({a}/{b})|{c}")),
        format!("{a}/{b}|{c}")
    );
    assert_eq!(
        rendered_path(&format!("{a}/({b}|{c})")),
        format!("{a}/({b}|{c})")
    );
    assert_eq!(rendered_path(&format!("^({a}/{b})")), format!("^({a}/{b})"));
    assert_eq!(rendered_path(&format!("({a}|{b})*")), format!("({a}|{b})*"));
    assert_eq!(
        rendered_path(&format!("{a}/({b}/{c})")),
        format!("{a}/({b}/{c})")
    );
    assert_eq!(
        rendered_path(&format!("{a}/{b}/{c}")),
        format!("{a}/{b}/{c}")
    );
}

#[test]
fn a_left_spine_across_precedence_levels_brackets_only_what_the_source_did() {
    assert_eq!(
        rendered_filter("((?a + ?b) * ?c + ?a) * ?b"),
        "((?a + ?b) * ?c + ?a) * ?b"
    );
    assert_eq!(rendered_filter("(?a * ?b) + ?c"), "?a * ?b + ?c");
    assert_eq!(rendered_filter("((?a || ?b)) || ?c"), "?a || ?b || ?c");
}

// ── operator chains of any length ───────────────────────────────────────────

/// `SELECT * WHERE { BIND(?a OP ?a OP … AS ?x) }` with `operators` operators.
fn bind_chain(op: &str, operators: usize) -> String {
    let mut expr = String::from("?a");
    for _ in 0..operators {
        expr.push_str(op);
        expr.push_str("?a");
    }
    format!("SELECT * WHERE {{ BIND({expr} AS ?x) }}")
}

/// A chain is one node however long, so a 10 000-operator chain — twenty times
/// what the old per-operator height charge refused — forwards as the same flat
/// text with no bracket but `BIND`'s own, and re-parses to the same node.
#[test]
fn a_ten_thousand_operator_chain_round_trips_flat() {
    for op in [" + ", " - ", " * ", " / ", " || ", " && "] {
        let text = assert_query_roundtrip(&bind_chain(op, 10_000));
        assert_eq!(
            text.matches('(').count(),
            1,
            "a chain needs no bracket beyond BIND's own: {op}"
        );
    }
    // Alternating precedence levels: every `* ?a` is its own chain, one operand of
    // the one additive chain, so the text stays flat too.
    let mixed = format!(
        "SELECT * WHERE {{ BIND(?a{} AS ?x) }}",
        " + ?a * ?a - ?a / ?a".repeat(5_000)
    );
    let text = assert_query_roundtrip(&mixed);
    assert_eq!(text.matches('(').count(), 1);
}

/// A `UNION` chain is one node with one arm per branch, and forwards as the flat
/// chain it was written as: a 10 000-arm chain re-parses to the same node.
#[test]
fn a_ten_thousand_arm_union_round_trips_flat() {
    let arms = (0..10_000)
        .map(|i| format!("{{ ?s <{EX}p> {i} }}"))
        .collect::<Vec<_>>()
        .join(" UNION ");
    let body = unproject(
        try_select(&format!("SELECT * WHERE {{ {arms} }}")).expect("a flat UNION chain parses"),
    );
    let GraphPattern::Union { arms: parsed } = &body else {
        panic!("expected one Union node");
    };
    assert_eq!(parsed.len(), 10_000);
    let text = assert_forwarded_roundtrip(&body);
    assert_eq!(text.matches(" UNION ").count(), 9_999);
}

/// A property-path chain is one node with one element per step, and forwards as the
/// flat chain it was written as: 10 000-step sequences and alternatives, and a mixed
/// chain of both with inverse, modified and negated steps, re-parse to the same node.
#[test]
fn a_ten_thousand_step_path_round_trips_flat() {
    for op in ["/", "|"] {
        let steps = (0..10_000)
            .map(|i| format!("<{EX}p{i}>"))
            .collect::<Vec<_>>()
            .join(op);
        let text = assert_query_roundtrip(&format!("SELECT * WHERE {{ ?s {steps} ?o }}"));
        assert_eq!(
            text.matches('(').count(),
            0,
            "a flat {op} chain needs no bracket"
        );
        assert_eq!(text.matches(&format!(">{op}<")).count(), 9_999);
    }
    let mixed = (0..2_000)
        .map(|i| match i % 5 {
            0 => format!("^<{EX}p{i}>"),
            1 => format!("<{EX}p{i}>*"),
            2 => format!("<{EX}p{i}>+|<{EX}q{i}>?"),
            3 => format!("!(<{EX}p{i}>|^<{EX}q{i}>)"),
            _ => format!("(<{EX}p{i}>|<{EX}q{i}>)"),
        })
        .collect::<Vec<_>>()
        .join("/");
    assert_query_roundtrip(&format!("SELECT * WHERE {{ ?s {mixed} ?o }}"));
}

/// Mixed precedence forwards with only the brackets the grammar needs, and re-parses
/// to the same algebra: `(a/b)|c/d` is an alternative of two sequences.
#[test]
fn a_mixed_precedence_path_round_trips() {
    let (a, b, c, d) = (
        format!("<{EX}a>"),
        format!("<{EX}b>"),
        format!("<{EX}c>"),
        format!("<{EX}d>"),
    );
    assert_eq!(
        rendered_path(&format!("({a}/{b})|{c}/{d}")),
        format!("{a}/{b}|{c}/{d}")
    );
    assert_eq!(
        rendered_path(&format!("({a}|{b})/({c}|{d})")),
        format!("({a}|{b})/({c}|{d})")
    );
    assert_eq!(
        rendered_path(&format!("{a}|({b}|{c})|{d}")),
        format!("{a}|({b}|{c})|{d}")
    );
    assert_eq!(
        rendered_path(&format!("(({a}/{b})/{c})/{d}")),
        format!("{a}/{b}/{c}/{d}")
    );
}

// ── every nesting and chaining family, written deep or long in a SERVICE ────────

/// `SELECT * WHERE { SERVICE <ep> { BODY } }`.
fn in_service(body: &str) -> String {
    format!("SELECT * WHERE {{ SERVICE <{EX}ep> {{ {body} }} }}")
}

/// The inner pattern of the query's one `SERVICE`.
fn service_body(pattern: GraphPattern) -> GraphPattern {
    match unproject(pattern) {
        GraphPattern::Service { inner, .. } => inner.into_inner(),
        other => panic!("expected the SERVICE at the root, got {other:?}"),
    }
}

/// Forward a family's `SERVICE` body written `n` levels deep and assert the text is
/// admitted and rebuilds the body.
fn assert_family_forwards(name: &str, body: &dyn Fn(usize) -> String, n: usize) {
    let query = in_service(&body(n));
    let inner = service_body(
        try_select(&query).unwrap_or_else(|e| panic!("{name}: n = {n} is admitted: {e}")),
    );
    let text = pattern_to_select_query(&inner);
    let reparsed = try_select(&text).unwrap_or_else(|e| {
        panic!("{name}: the body admitted at n = {n} forwards as refused text: {e}")
    });
    assert_eq!(
        unproject(reparsed),
        inner,
        "{name}: the forwarded text of the n = {n} body rebuilt a different tree"
    );
}

/// `f(f(…f(base)…))`, `n` applications.
fn nest(n: usize, base: &str, f: impl Fn(&str) -> String) -> String {
    let mut s = base.to_owned();
    for _ in 0..n {
        s = f(&s);
    }
    s
}

/// `item` repeated `n` times, each instance given its index.
fn chain(n: usize, item: impl Fn(usize) -> String) -> String {
    (0..n).map(item).collect::<Vec<_>>().join(" ")
}

const TRIPLE: &str = "?s <http://example.org/p> ?o .";

/// How many sibling elements a chained-clause family is written with. A chain builds a
/// spine, not nesting; each `BIND` in one is checked, under `debug_assertions`, against
/// a fresh walk of the whole group before it, so a test build's parse of a chain grows
/// with its square.
const CHAIN: usize = 4_000;

#[test]
fn expression_families_forward_twenty_thousand_levels_deep() {
    let filter = |e: String| format!("{TRIPLE} FILTER({e})");
    assert_family_forwards(
        "right-nested subtraction",
        &|n| filter(nest(n, "?o", |e| format!("?o - ({e})"))),
        20_000,
    );
    assert_family_forwards(
        "unary minus",
        &|n| filter(nest(n, "?o", |e| format!("-{e}"))),
        20_000,
    );
    assert_family_forwards(
        "logical not",
        &|n| filter(nest(n, "?o", |e| format!("!{e}"))),
        20_000,
    );
    assert_family_forwards(
        "built-in call",
        &|n| filter(nest(n, "?o", |e| format!("STR({e})"))),
        20_000,
    );
    assert_family_forwards(
        "IN list",
        &|n| filter(nest(n, "?o", |e| format!("?o IN ({e})"))),
        20_000,
    );
    assert_family_forwards(
        "not-equal operand",
        &|n| filter(nest(n, "?o", |e| format!("({e} != ?o) * ?o"))),
        20_000,
    );
    assert_family_forwards(
        "EXISTS in an expression",
        &|n| {
            filter(nest(n, "?o", |e| {
                format!("?o && EXISTS {{ {TRIPLE} FILTER({e}) }}")
            }))
        },
        20_000,
    );
}

#[test]
fn graph_pattern_families_forward_twenty_thousand_levels_deep() {
    assert_family_forwards(
        "nested OPTIONAL",
        &|n| nest(n, TRIPLE, |b| format!("{TRIPLE} OPTIONAL {{ {b} }}")),
        20_000,
    );
    assert_family_forwards(
        "nested OPTIONAL sub-SELECT",
        &|n| {
            nest(n, TRIPLE, |b| {
                format!("{TRIPLE} OPTIONAL {{ SELECT * WHERE {{ {b} }} }}")
            })
        },
        20_000,
    );
    assert_family_forwards(
        "nested sub-SELECT",
        &|n| {
            nest(n, TRIPLE, |b| {
                format!("{TRIPLE} {{ SELECT * WHERE {{ {b} }} }}")
            })
        },
        20_000,
    );
    assert_family_forwards(
        "right-nested UNION",
        &|n| nest(n, TRIPLE, |b| format!("{{ {TRIPLE} }} UNION {{ {b} }}")),
        20_000,
    );
    assert_family_forwards(
        "braced FILTER group before an OPTIONAL",
        &|n| {
            nest(n, TRIPLE, |b| {
                format!("{{ {b} FILTER(?o) }} OPTIONAL {{ {TRIPLE} }}")
            })
        },
        20_000,
    );
    assert_family_forwards(
        "nested NOT EXISTS",
        &|n| {
            nest(n, TRIPLE, |b| {
                format!("{TRIPLE} FILTER NOT EXISTS {{ {b} }}")
            })
        },
        20_000,
    );
    assert_family_forwards(
        "nested GRAPH",
        &|n| nest(n, TRIPLE, |b| format!("GRAPH <{EX}g> {{ {b} }}")),
        20_000,
    );
    assert_family_forwards(
        "nested SERVICE",
        &|n| nest(n, TRIPLE, |b| format!("SERVICE <{EX}ep> {{ {b} }}")),
        20_000,
    );
    assert_family_forwards(
        "nested MINUS",
        &|n| nest(n, TRIPLE, |b| format!("{TRIPLE} MINUS {{ {b} }}")),
        20_000,
    );
}

#[test]
fn chained_clause_families_forward_four_thousand_elements_long() {
    // A `UNION` chain of any length is one node (see
    // `a_ten_thousand_arm_union_round_trips_flat`); what bounds it is the spine
    // inside its tallest arm.
    assert_family_forwards(
        "UNION arm holding an OPTIONAL chain",
        &|n| {
            format!(
                "{{ {TRIPLE} }} UNION {{ {TRIPLE} {} }} UNION {{ {TRIPLE} }}",
                chain(n, |_| format!("OPTIONAL {{ {TRIPLE} }}"))
            )
        },
        CHAIN,
    );
    assert_family_forwards(
        "OPTIONAL chain",
        &|n| {
            format!(
                "{TRIPLE} {}",
                chain(n, |_| format!("OPTIONAL {{ {TRIPLE} }}"))
            )
        },
        CHAIN,
    );
    assert_family_forwards(
        "MINUS chain",
        &|n| format!("{TRIPLE} {}", chain(n, |_| format!("MINUS {{ {TRIPLE} }}"))),
        CHAIN,
    );
    assert_family_forwards(
        "BIND chain",
        &|n| format!("{TRIPLE} {}", chain(n, |i| format!("BIND(?o AS ?v{i})"))),
        CHAIN,
    );
    assert_family_forwards(
        "mixed OPTIONAL / BIND / MINUS chain",
        &|n| {
            format!(
                "{TRIPLE} {}",
                chain(n, |i| match i % 3 {
                    0 => format!("OPTIONAL {{ {TRIPLE} }}"),
                    1 => format!("BIND(?o AS ?v{i})"),
                    _ => format!("MINUS {{ {TRIPLE} }}"),
                })
            )
        },
        CHAIN,
    );
    assert_family_forwards(
        "FILTER chain",
        &|n| format!("{TRIPLE} {}", chain(n, |_| "FILTER(?o)".to_owned())),
        CHAIN,
    );
}

#[test]
fn property_path_families_forward_twenty_thousand_levels_deep() {
    let a = format!("<{EX}a>");
    assert_family_forwards(
        "right-nested sequence",
        &|n| format!("?s {} ?o", nest(n, &a, |p| format!("{a}/({p})"))),
        20_000,
    );
    assert_family_forwards(
        "right-nested alternative",
        &|n| format!("?s {} ?o", nest(n, &a, |p| format!("{a}|({p})"))),
        20_000,
    );
    assert_family_forwards(
        "nested inverse",
        &|n| format!("?s {} ?o", nest(n, &a, |p| format!("^({p})"))),
        20_000,
    );
    assert_family_forwards(
        "nested quantifier",
        &|n| format!("?s {} ?o", nest(n, &a, |p| format!("({p})*"))),
        20_000,
    );
}

// ── generated trees ─────────────────────────────────────────────────────────

fn var(name: &str) -> Variable {
    Variable::new(name)
}

fn iri(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{EX}{local}"))
}

fn leaf_expression() -> impl Strategy<Value = Expression> {
    prop_oneof![
        prop::sample::select(vec!["a", "b", "c"]).prop_map(|v| Expression::Variable(var(v))),
        prop::sample::select(vec!["x", "y"]).prop_map(|l| Expression::NamedNode(iri(l))),
        (0_u8..3).prop_map(|n| Expression::Literal(Literal::new_simple(format!("s{n}")))),
        (0_u8..3).prop_map(|n| Expression::Literal(Literal::new_typed(
            n.to_string(),
            NamedNode::new_unchecked(XSD_INTEGER)
        ))),
        prop::sample::select(vec!["a", "b"]).prop_map(|v| Expression::Bound(var(v))),
    ]
}

fn exists_body() -> GraphPattern {
    GraphPattern::Bgp {
        patterns: vec![TriplePattern {
            subject: TermPattern::Variable(var("a")),
            predicate: NamedNodePattern::NamedNode(iri("q")),
            object: TermPattern::Variable(var("b")),
        }],
    }
}

/// A constructor of a two-operand expression node, built the way the parser builds
/// it (a chain operator extends a chain on its left).
type Binary = fn(Expression, Expression) -> Expression;

fn expression_tree() -> impl Strategy<Value = Expression> {
    let binaries: Vec<Binary> = vec![
        Expression::or,
        Expression::and,
        |a, b| Expression::Equal(Child::new(a), Child::new(b)),
        |a, b| Expression::Less(Child::new(a), Child::new(b)),
        |a, b| Expression::GreaterOrEqual(Child::new(a), Child::new(b)),
        |a, b| Expression::arithmetic(a, ArithmeticOperator::Add, b),
        |a, b| Expression::arithmetic(a, ArithmeticOperator::Subtract, b),
        |a, b| Expression::arithmetic(a, ArithmeticOperator::Multiply, b),
        |a, b| Expression::arithmetic(a, ArithmeticOperator::Divide, b),
        |a, b| Expression::SameTerm(Child::new(a), Child::new(b)),
    ];
    leaf_expression().prop_recursive(6, 64, 3, move |inner| {
        // Two-operand nodes carry most of the weight: operator precedence and
        // associativity are what the rendering has to get right.
        prop_oneof![
            10 => (
                prop::sample::select(binaries.clone()),
                inner.clone(),
                inner.clone()
            )
                .prop_map(|(node, a, b)| node(a, b)),
            1 => inner.clone().prop_map(|a| Expression::Not(Child::new(a))),
            1 => inner.clone().prop_map(|a| Expression::UnaryMinus(Child::new(a))),
            1 => inner.clone().prop_map(|a| Expression::UnaryPlus(Child::new(a))),
            1 => (inner.clone(), prop::collection::vec(inner.clone(), 0..3))
                .prop_map(|(a, list)| Expression::In(Child::new(a), list.into())),
            1 => inner
                .clone()
                .prop_map(|a| Expression::FunctionCall(Function::Str, vec![a].into())),
            1 => (inner.clone(), inner.clone(), inner.clone())
                .prop_map(|(c, t, e)| Expression::If(Child::new(c), Child::new(t), Child::new(e))),
            1 => prop::collection::vec(inner.clone(), 1..3).prop_map(|list| Expression::Coalesce(list.into())),
            1 => Just(Expression::Exists(Child::new(exists_body()))),
            1 => inner.prop_map(|a| Expression::and(
                a,
                Expression::Exists(Child::new(exists_body()))
            )),
        ]
    })
}

fn leaf_path() -> impl Strategy<Value = PropertyPathExpression> {
    prop_oneof![
        prop::sample::select(vec!["a", "b", "c"])
            .prop_map(|l| PropertyPathExpression::NamedNode(iri(l))),
        prop::collection::vec((prop::sample::select(vec!["a", "b"]), any::<bool>()), 1..3)
            .prop_map(|elements| {
                PropertyPathExpression::NegatedPropertySet(
                    elements
                        .into_iter()
                        .map(|(l, inverse)| NegatedPathElement {
                            predicate: iri(l),
                            inverse,
                        })
                        .collect(),
                )
            }),
    ]
}

fn path_tree() -> impl Strategy<Value = PropertyPathExpression> {
    leaf_path().prop_recursive(6, 48, 2, |inner| {
        prop_oneof![
            // Chains are built the way the parser builds them: a left element of the
            // same operator extends the chain, a later one stays one element.
            (inner.clone(), prop::collection::vec(inner.clone(), 1..4)).prop_map(|(a, rest)| {
                rest.into_iter().fold(a, PropertyPathExpression::sequence)
            }),
            (inner.clone(), prop::collection::vec(inner.clone(), 1..4)).prop_map(|(a, rest)| {
                rest.into_iter()
                    .fold(a, PropertyPathExpression::alternative)
            }),
            inner
                .clone()
                .prop_map(|a| PropertyPathExpression::Reverse(Child::new(a))),
            inner
                .clone()
                .prop_map(|a| PropertyPathExpression::ZeroOrMore(Child::new(a))),
            inner
                .clone()
                .prop_map(|a| PropertyPathExpression::OneOrMore(Child::new(a))),
            inner
                .clone()
                .prop_map(|a| PropertyPathExpression::ZeroOrOne(Child::new(a))),
            (inner, 0_u32..3, prop::option::of(3_u32..5)).prop_map(|(a, min, max)| {
                PropertyPathExpression::Range {
                    inner: Child::new(a),
                    min,
                    max,
                }
            }),
        ]
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// Every generated expression tree, filtered over a triple, is rendered into
    /// text the parser admits and re-parses to the identical tree.
    #[test]
    fn a_generated_expression_round_trips(expr in expression_tree()) {
        let body = GraphPattern::Filter {
            expr,
            inner: Child::new(exists_body()),
        };
        assert_forwarded_roundtrip(&body);
    }

    /// Every generated property path (a bare predicate is a triple, not a path, so it
    /// is wrapped in `+`) is rendered into text that re-parses to the identical tree.
    #[test]
    fn a_generated_property_path_round_trips(path in path_tree()) {
        let path = match path {
            PropertyPathExpression::NamedNode(_) => {
                PropertyPathExpression::OneOrMore(Child::new(path))
            }
            other => other,
        };
        let body = GraphPattern::Path {
            subject: TermPattern::Variable(var("s")),
            path,
            object: TermPattern::Variable(var("o")),
        };
        assert_forwarded_roundtrip(&body);
    }
}
