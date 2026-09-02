// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The two SEP-0009 composite-datatype **grammar** additions: the `FOLD`
//! aggregate and the `UNFOLD` graph pattern.
//!
//! ```text
//! [127+] Aggregate ::= … | 'FOLD' '(' 'DISTINCT'? Expression ( ',' Expression )?
//!                             ( 'ORDER' 'BY' OrderCondition+ )? ')'
//! [174]  Unfold    ::= 'UNFOLD' '(' Expression 'AS' Var ( ',' Var )? ')'
//! ```
//!
//! Two properties, for every attested surface form:
//!
//! * it **parses**, to the algebra shape it names
//!   ([`AggregateFunction::Fold`] on a `Group`'s aggregate list, or a
//!   [`GraphPattern::Unfold`] node stacked above the pattern before it); and
//! * serialization is a **fixpoint**: parse → serialize → re-parse → serialize
//!   yields the identical text AND the identical algebra. That is the property
//!   `SERVICE` federation depends on — a forwarded body is serialized text — so a
//!   form that parses but re-emits as something else is a wire-format bug, not a
//!   cosmetic one.
//!
//! The forms exercised are the ones the vendored corpus actually writes
//! (`vectors/sparql-cdt/fold/`, `vectors/sparql-cdt/unfold/`), plus the shapes the
//! corpus leaves unwritten but the grammar admits.

use purrdf_sparql_algebra::{
    AggregateExpression, AggregateExpressionError, AggregateFunction, Expression, GraphPattern,
    OrderExpression, Query, SparqlParser, Variable, pattern_to_select_query,
};

/// The SEP-0009 prologue every query below is written under. The namespace is the
/// spec's own fixed string — recognized, never minted.
const PREFIX: &str = "PREFIX cdt: <http://w3id.org/awslabs/neptune/SPARQL-CDTs/>\n\
                      PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>\n";

/// Parse a `SELECT` and return its root pattern.
fn select_pattern(query: &str) -> GraphPattern {
    match SparqlParser::new()
        .parse_query(&format!("{PREFIX}{query}"))
        .unwrap_or_else(|error| panic!("parse `{query}`: {error:?}"))
    {
        Query::Select { pattern, .. } => pattern,
        other => panic!("expected a SELECT, got {other:?}"),
    }
}

/// Strip exactly one outer `Project` — the `SELECT` scaffold — to recover the body
/// [`pattern_to_select_query`] consumes and re-produces.
fn where_body(pattern: &GraphPattern) -> GraphPattern {
    match pattern {
        GraphPattern::Project { inner, .. } => (**inner).clone(),
        other => other.clone(),
    }
}

/// Assert that serialization is a FIXPOINT for `query`: the round trip preserves
/// both the rendered text and the algebra, so a second pass changes nothing.
/// Returns the serialized text for a caller that also wants to read it.
fn assert_roundtrip_fixpoint(query: &str) -> String {
    let body = where_body(&select_pattern(query));
    let text = pattern_to_select_query(&body);

    let reparsed = match SparqlParser::new()
        .parse_query(&text)
        .unwrap_or_else(|error| panic!("re-parse `{text}`: {error:?}"))
    {
        Query::Select { pattern, .. } => pattern,
        other => panic!("expected a SELECT, got {other:?}"),
    };
    let reparsed_body = where_body(&reparsed);
    assert_eq!(
        reparsed_body, body,
        "round trip changed the algebra for `{query}`\n  serialized: {text}"
    );
    let text_again = pattern_to_select_query(&reparsed_body);
    assert_eq!(
        text_again, text,
        "round trip is not a fixpoint on the TEXT for `{query}`"
    );
    text
}

/// The `Group` node's aggregate list, wherever it sits under `pattern`'s
/// projection/extend scaffold.
fn aggregates_of(pattern: &GraphPattern) -> Vec<AggregateExpression> {
    fn walk(pattern: &GraphPattern, out: &mut Vec<AggregateExpression>) {
        match pattern {
            GraphPattern::Group { aggregates, .. } => {
                out.extend(aggregates.iter().map(|(_, agg)| agg.clone()));
            }
            GraphPattern::Project { inner, .. }
            | GraphPattern::Extend { inner, .. }
            | GraphPattern::Filter { inner, .. }
            | GraphPattern::OrderBy { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Slice { inner, .. } => walk(inner, out),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(pattern, &mut out);
    out
}

// ── FOLD parses to the aggregate it names ───────────────────────────────────

/// One expression is the `cdt:List` form, two the `cdt:Map` form, and the FIRST
/// of the two is the key (`fold-map-02.rq`'s reading).
#[test]
fn fold_parses_to_one_or_two_exprlist_entries() {
    let list = aggregates_of(&select_pattern(
        "SELECT (FOLD(?v) AS ?l) WHERE { VALUES ?v { 1 } }",
    ));
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].function(), &AggregateFunction::Fold);
    assert_eq!(list[0].args().len(), 1, "the cdt:List form is unary");
    assert!(
        list[0].order_by().is_empty(),
        "a FOLD written without an ORDER BY clause must carry none; got {:?}",
        list[0].order_by()
    );
    assert!(!list[0].distinct);

    let map = aggregates_of(&select_pattern(
        "SELECT (FOLD(?k, ?v) AS ?m) WHERE { VALUES (?k ?v) { (1 2) } }",
    ));
    assert_eq!(map[0].function(), &AggregateFunction::Fold);
    assert_eq!(map[0].args().len(), 2, "the cdt:Map form is binary");
    assert_eq!(map[0].args()[0], Expression::Variable(Variable::new("k")));
    assert_eq!(map[0].args()[1], Expression::Variable(Variable::new("v")));
}

/// `DISTINCT` sits immediately after `(`, before the first expression, in both
/// forms.
#[test]
fn fold_parses_distinct_in_both_forms() {
    for (query, arity) in [
        (
            "SELECT (FOLD(DISTINCT ?v) AS ?l) WHERE { VALUES ?v { 1 } }",
            1,
        ),
        (
            "SELECT (FOLD(DISTINCT ?k, ?v) AS ?m) WHERE { VALUES (?k ?v) { (1 2) } }",
            2,
        ),
    ] {
        let aggregates = aggregates_of(&select_pattern(query));
        assert!(aggregates[0].distinct, "`{query}` must carry DISTINCT");
        assert_eq!(aggregates[0].args().len(), arity);
    }
}

/// `ORDER BY` follows the LAST expression with no separating comma, and its
/// conditions land on the aggregation node rather than on the query's own
/// solution modifier.
#[test]
fn fold_parses_its_own_order_by_onto_the_aggregation() {
    let aggregates = aggregates_of(&select_pattern(
        "SELECT (FOLD(?v ORDER BY DESC(?sort)) AS ?l) WHERE { VALUES (?sort ?v) { (1 2) } }",
    ));
    assert_eq!(
        aggregates[0].order_by(),
        &[OrderExpression::Desc(Expression::Variable(Variable::new(
            "sort"
        )))]
    );

    // Multiple conditions, left to right.
    let aggregates = aggregates_of(&select_pattern(
        "SELECT (FOLD(?v ORDER BY ASC(?a) ASC(?b)) AS ?l) \
         WHERE { VALUES (?a ?b ?v) { (1 2 3) } }",
    ));
    assert_eq!(
        aggregates[0].order_by(),
        &[
            OrderExpression::Asc(Expression::Variable(Variable::new("a"))),
            OrderExpression::Asc(Expression::Variable(Variable::new("b"))),
        ]
    );

    // A BARE sort key is `ASC`, exactly as it is in a query's own ORDER BY.
    let aggregates = aggregates_of(&select_pattern(
        "SELECT (FOLD(?v ORDER BY ?v) AS ?l) WHERE { VALUES ?v { 1 } }",
    ));
    assert_eq!(
        aggregates[0].order_by(),
        &[OrderExpression::Asc(Expression::Variable(Variable::new(
            "v"
        )))]
    );
}

/// The `ORDER BY` belongs to the aggregation, so an OUTER `ORDER BY` in the same
/// query is a separate, independent thing — the two must not be conflated.
#[test]
fn a_folds_order_by_is_not_the_querys_own() {
    let pattern = select_pattern(
        "SELECT ?g (FOLD(?v ORDER BY ?v) AS ?l) WHERE { VALUES (?g ?v) { (1 2) } } \
         GROUP BY ?g ORDER BY DESC(?g)",
    );
    let aggregates = aggregates_of(&pattern);
    assert_eq!(
        aggregates[0].order_by(),
        &[OrderExpression::Asc(Expression::Variable(Variable::new(
            "v"
        )))],
        "the aggregate keeps its OWN ascending key"
    );
    // …and the query's own descending key is still on an `OrderBy` node.
    fn has_outer_order_by(pattern: &GraphPattern) -> bool {
        match pattern {
            GraphPattern::OrderBy { expression, .. } => {
                expression
                    == &[OrderExpression::Desc(Expression::Variable(Variable::new(
                        "g",
                    )))]
            }
            GraphPattern::Project { inner, .. }
            | GraphPattern::Slice { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner } => has_outer_order_by(inner),
            _ => false,
        }
    }
    assert!(has_outer_order_by(&pattern));
}

// ── FOLD's grammar has edges, and they are hard errors ──────────────────────

/// `FOLD(*)` is not grammar: `'*'` is the empty exprlist, defined only for
/// `COUNT`.
#[test]
fn fold_refuses_the_star_exprlist() {
    let error = SparqlParser::new()
        .parse_query(&format!(
            "{PREFIX}SELECT (FOLD(*) AS ?l) WHERE {{ ?s ?p ?o }}"
        ))
        .expect_err("FOLD(*) is not SPARQL");
    assert!(
        format!("{error:?}").contains('*'),
        "the diagnostic must name the offending `*`: {error:?}"
    );
}

/// A third expression is refused: `FOLD` builds a list or a map, and there is no
/// three-argument composite.
#[test]
fn fold_refuses_a_third_expression() {
    SparqlParser::new()
        .parse_query(&format!(
            "{PREFIX}SELECT (FOLD(?a, ?b, ?c) AS ?l) WHERE {{ ?s ?p ?o }}"
        ))
        .expect_err("FOLD takes one or two expressions");
}

/// A nested aggregate inside a `FOLD` sort key is a hard error, not a silently
/// lifted second aggregate.
#[test]
fn fold_refuses_a_nested_aggregate_in_a_sort_key() {
    SparqlParser::new()
        .parse_query(&format!(
            "{PREFIX}SELECT (FOLD(?v ORDER BY SUM(?w)) AS ?l) WHERE {{ ?s ?p ?o }}"
        ))
        .expect_err("aggregates do not nest");
}

/// The type-level half of the same rule: sort keys are representable only on
/// `FOLD`. A `SUM` carrying them would render as `SUM(?v ORDER BY ?k)`, which is
/// grammar for nothing.
#[test]
fn only_fold_admits_order_by_on_the_aggregation_node() {
    let keys = vec![OrderExpression::Asc(Expression::Variable(Variable::new(
        "k",
    )))];
    for function in [
        AggregateFunction::Count,
        AggregateFunction::Sum,
        AggregateFunction::Avg,
        AggregateFunction::Min,
        AggregateFunction::Max,
        AggregateFunction::Sample,
        AggregateFunction::GroupConcat,
    ] {
        let error = AggregateExpression::new(
            function.clone(),
            vec![Expression::Variable(Variable::new("v"))],
            Vec::new(),
            keys.clone(),
            false,
        )
        .expect_err("only FOLD accepts sort keys");
        assert!(matches!(error, AggregateExpressionError::OrderBy(_)));
        assert_eq!(error.function(), &function);
    }
    assert!(
        AggregateExpression::new(
            AggregateFunction::Fold,
            vec![Expression::Variable(Variable::new("v"))],
            Vec::new(),
            keys,
            false,
        )
        .is_ok()
    );
}

/// `FOLD` is the one built-in whose exprlist may hold two expressions, and it
/// stops there — the constructor's arity rule, checked from both ends.
#[test]
fn fold_admits_exactly_one_or_two_arguments() {
    let var = || Expression::Variable(Variable::new("v"));
    for count in [0usize, 3, 4] {
        let error = AggregateExpression::new(
            AggregateFunction::Fold,
            std::iter::repeat_with(var).take(count).collect(),
            Vec::new(),
            Vec::new(),
            false,
        )
        .expect_err("FOLD takes one or two arguments");
        let AggregateExpressionError::Arity(arity) = error else {
            panic!("expected the Arity arm");
        };
        assert_eq!(arity.supplied(), count);
    }
    for count in [1usize, 2] {
        assert!(
            AggregateExpression::new(
                AggregateFunction::Fold,
                std::iter::repeat_with(var).take(count).collect(),
                Vec::new(),
                Vec::new(),
                false,
            )
            .is_ok()
        );
    }
}

/// `FOLD` admits no `; NAME=value` scalarval — `GROUP_CONCAT`'s `SEPARATOR` is
/// the only built-in scalarval SPARQL has.
#[test]
fn fold_admits_no_scalarval() {
    let error = AggregateExpression::new(
        AggregateFunction::Fold,
        vec![Expression::Variable(Variable::new("v"))],
        vec![(
            "separator".to_owned(),
            purrdf_sparql_algebra::Literal::new_simple("|"),
        )],
        Vec::new(),
        false,
    )
    .expect_err("FOLD accepts no scalarval");
    assert!(matches!(error, AggregateExpressionError::Scalarval(_)));
}

// ── UNFOLD parses to its own graph-pattern node ─────────────────────────────

/// The one-variable form binds `element` and nothing else, stacked ABOVE the
/// `BIND` that supplied its operand.
#[test]
fn unfold_parses_to_its_own_node_above_the_binding_before_it() {
    let pattern = select_pattern(
        "SELECT ?elmt WHERE { BIND(\"[1,2]\"^^cdt:List AS ?list) UNFOLD(?list AS ?elmt) }",
    );
    let GraphPattern::Project { inner, variables } = &pattern else {
        panic!("expected a projection");
    };
    assert_eq!(variables, &[Variable::new("elmt")]);
    let GraphPattern::Unfold {
        inner,
        expression,
        element,
        companion,
    } = &**inner
    else {
        panic!("expected an Unfold node, got {inner:?}");
    };
    assert_eq!(expression, &Expression::Variable(Variable::new("list")));
    assert_eq!(element, &Variable::new("elmt"));
    assert_eq!(companion, &None);
    assert!(
        matches!(**inner, GraphPattern::Extend { .. }),
        "UNFOLD sits above the BIND that supplied it, got {inner:?}"
    );
}

/// The two-variable form's second target is the `companion`.
#[test]
fn unfold_parses_the_two_variable_form() {
    let pattern = select_pattern(
        "SELECT ?elmt ?idx WHERE { BIND(\"[1,2]\"^^cdt:List AS ?list) \
         UNFOLD(?list AS ?elmt, ?idx) }",
    );
    let GraphPattern::Project { inner, .. } = &pattern else {
        panic!("expected a projection");
    };
    let GraphPattern::Unfold {
        element, companion, ..
    } = &**inner
    else {
        panic!("expected an Unfold node");
    };
    assert_eq!(element, &Variable::new("elmt"));
    assert_eq!(companion.as_ref(), Some(&Variable::new("idx")));
}

/// The operand is an arbitrary EXPRESSION, not merely a variable — which is
/// exactly why `UNFOLD` is a graph-pattern node rather than a property function
/// (a property function's arguments are plain terms).
#[test]
fn unfold_accepts_an_arbitrary_expression_operand() {
    let pattern = select_pattern("SELECT ?e WHERE { UNFOLD(cdt:List(1, 2) AS ?e) }");
    let GraphPattern::Project { inner, .. } = &pattern else {
        panic!("expected a projection");
    };
    let GraphPattern::Unfold { expression, .. } = &**inner else {
        panic!("expected an Unfold node");
    };
    assert!(
        matches!(expression, Expression::FunctionCall(..)),
        "the operand is a call expression, got {expression:?}"
    );
}

/// `SELECT *` sees both targets: they are ordinary in-scope bindings of the
/// enclosing group.
#[test]
fn unfold_targets_are_visible_to_select_star() {
    let pattern =
        select_pattern("SELECT * WHERE { BIND(\"{1 : 2}\"^^cdt:Map AS ?m) UNFOLD(?m AS ?k, ?v) }");
    let GraphPattern::Project { variables, .. } = &pattern else {
        panic!("expected a projection");
    };
    for name in ["m", "k", "v"] {
        assert!(
            variables.contains(&Variable::new(name)),
            "SELECT * must project ?{name}, got {variables:?}"
        );
    }
}

/// `BIND`'s §19.6 scope rule, applied to both `UNFOLD` targets: re-binding a
/// variable already in scope is a hard syntax error, not a silent shadow.
#[test]
fn unfold_refuses_a_target_already_in_scope() {
    for query in [
        "SELECT * WHERE { ?elmt <http://example.org/p> ?o UNFOLD(?o AS ?elmt) }",
        "SELECT * WHERE { ?idx <http://example.org/p> ?o UNFOLD(?o AS ?elmt, ?idx) }",
        "SELECT * WHERE { BIND(1 AS ?e) UNFOLD(?e AS ?e) }",
    ] {
        let error = SparqlParser::new()
            .parse_query(&format!("{PREFIX}{query}"))
            .expect_err("re-binding an in-scope variable is refused");
        assert!(
            format!("{error:?}").contains("UNFOLD"),
            "the diagnostic must name UNFOLD: {error:?}"
        );
    }
}

/// The two targets bind two DIFFERENT positions of one element, so one variable
/// in both slots is refused rather than resolved by a precedence rule.
#[test]
fn unfold_refuses_binding_one_variable_twice() {
    let error = SparqlParser::new()
        .parse_query(&format!(
            "{PREFIX}SELECT * WHERE {{ BIND(\"[1]\"^^cdt:List AS ?l) UNFOLD(?l AS ?e, ?e) }}"
        ))
        .expect_err("one variable cannot fill both targets");
    assert!(
        format!("{error:?}").contains("twice"),
        "the diagnostic must say the variable is bound twice: {error:?}"
    );
}

// ── serialization is a fixpoint ─────────────────────────────────────────────

/// Every `FOLD` surface form the corpus attests, plus the two the grammar admits
/// and the corpus leaves unwritten (`FOLD(DISTINCT ?k, ?v)` and a `FOLD` beside
/// another aggregate).
#[test]
fn every_attested_fold_form_round_trips() {
    for query in [
        "SELECT (FOLD(?v) AS ?l) WHERE { VALUES ?v { 1 2 } }",
        "SELECT (FOLD(DISTINCT ?v) AS ?l) WHERE { VALUES ?v { 1 1 } }",
        "SELECT (FOLD(?v ORDER BY ?v) AS ?l) WHERE { VALUES ?v { 2 1 } }",
        "SELECT (FOLD(?v ORDER BY DESC(?sort)) AS ?l) WHERE { VALUES (?sort ?v) { (1 2) } }",
        "SELECT (FOLD(?v ORDER BY ASC(?a) ASC(?b)) AS ?l) WHERE { VALUES (?a ?b ?v) { (1 2 3) } }",
        "SELECT (FOLD(DISTINCT ?v ORDER BY ASC(?sort)) AS ?l) \
         WHERE { VALUES (?sort ?v) { (1 2) } }",
        "SELECT (FOLD(?k, ?v) AS ?m) WHERE { VALUES (?k ?v) { (1 2) } }",
        "SELECT (FOLD(?k, ?v ORDER BY ?sort) AS ?m) WHERE { VALUES (?k ?v ?sort) { (1 2 3) } }",
        "SELECT (FOLD(?k, ?v ORDER BY DESC(?sort)) AS ?m) \
         WHERE { VALUES (?k ?v ?sort) { (1 2 3) } }",
        // Not in the corpus: DISTINCT over the map form, and a FOLD beside
        // another aggregate in one SELECT.
        "SELECT (FOLD(DISTINCT ?k, ?v) AS ?m) WHERE { VALUES (?k ?v) { (1 2) } }",
        "SELECT (FOLD(?v) AS ?l) (COUNT(?v) AS ?c) WHERE { VALUES ?v { 1 2 } }",
        // A FOLD grouped, and a FOLD whose expression is a call rather than a bare
        // variable.
        "SELECT ?g (FOLD(?v) AS ?l) WHERE { VALUES (?g ?v) { (1 2) } } GROUP BY ?g",
        "SELECT (FOLD(STR(?v)) AS ?l) WHERE { VALUES ?v { 1 } }",
    ] {
        let text = assert_roundtrip_fixpoint(query);
        assert!(
            text.contains("FOLD("),
            "the rendered text must still spell FOLD: {text}"
        );
    }
}

/// A `FOLD`'s `ORDER BY` renders in the explicit `ASC(…)`/`DESC(…)` form so two
/// bare keys cannot run together into one expression on re-parse — the reason the
/// serializer does not simply echo what was written.
#[test]
fn fold_order_by_renders_explicitly_directed() {
    let text = assert_roundtrip_fixpoint(
        "SELECT (FOLD(?v ORDER BY ?a ?b) AS ?l) WHERE { VALUES (?a ?b ?v) { (1 2 3) } }",
    );
    assert!(
        text.contains("ORDER BY ASC(?a) ASC(?b)"),
        "both keys must be rendered with an explicit direction: {text}"
    );
    assert_eq!(
        text.matches("ORDER BY").count(),
        1,
        "the clause is written ONCE, before the first condition: {text}"
    );
}

/// Every `UNFOLD` surface form the corpus attests, plus its composition with the
/// other group elements the corpus leaves unwritten.
#[test]
fn every_attested_unfold_form_round_trips() {
    for query in [
        "SELECT ?elmt WHERE { BIND(\"[1,2]\"^^cdt:List AS ?list) UNFOLD(?list AS ?elmt) }",
        "SELECT ?elmt ?idx WHERE { BIND(\"[1,2]\"^^cdt:List AS ?list) \
         UNFOLD(?list AS ?elmt, ?idx) }",
        "SELECT ?k ?v WHERE { BIND(\"{1 : 2}\"^^cdt:Map AS ?map) UNFOLD(?map AS ?k, ?v) }",
        // The corpus's `unfold-get-*` shape: a FILTER in the same group reads the
        // bound targets.
        "SELECT * WHERE { BIND(\"[1]\"^^cdt:List AS ?list) UNFOLD(?list AS ?elmt) \
         FILTER(SAMETERM(?elmt, cdt:get(?list, 1))) }",
        // Not in the corpus: UNFOLD over a constructed value, first in its group,
        // twice in one group, inside OPTIONAL/UNION/MINUS/GRAPH, and beside VALUES.
        "SELECT * WHERE { UNFOLD(cdt:List(1, 2) AS ?e) }",
        "SELECT * WHERE { BIND(\"[[1],[2]]\"^^cdt:List AS ?l) UNFOLD(?l AS ?outer) \
         UNFOLD(?outer AS ?inner) }",
        "SELECT * WHERE { ?s <http://example.org/p> ?o \
         OPTIONAL { UNFOLD(?o AS ?e) } }",
        "SELECT * WHERE { { UNFOLD(cdt:List(1) AS ?e) } UNION { UNFOLD(cdt:List(2) AS ?e) } }",
        "SELECT * WHERE { ?s <http://example.org/p> ?o UNFOLD(?o AS ?e) \
         MINUS { ?s <http://example.org/q> ?z } }",
        "SELECT * WHERE { GRAPH ?g { ?s <http://example.org/p> ?o UNFOLD(?o AS ?e) } }",
        "SELECT * WHERE { VALUES ?o { 1 } UNFOLD(?o AS ?e) }",
        "SELECT * WHERE { ?s <http://example.org/p> ?o UNFOLD(?o AS ?e) FILTER(?e > 1) }",
    ] {
        let text = assert_roundtrip_fixpoint(query);
        assert!(
            text.contains("UNFOLD("),
            "the rendered text must still spell UNFOLD: {text}"
        );
    }
}

/// The two nouns compose: a `FOLD` whose group was produced by an `UNFOLD` round
/// trips as one query.
#[test]
fn fold_over_an_unfold_round_trips() {
    let text = assert_roundtrip_fixpoint(
        "SELECT (FOLD(?e ORDER BY ?i) AS ?l) WHERE { \
         BIND(\"[3,1,2]\"^^cdt:List AS ?list) UNFOLD(?list AS ?e, ?i) }",
    );
    assert!(text.contains("UNFOLD("), "{text}");
    assert!(text.contains("FOLD("), "{text}");
}
