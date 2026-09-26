// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Copies of algebra trees that can refuse when the stack runs low.
//!
//! The derived `Clone` of [`GraphPattern`], [`Expression`] and
//! [`PropertyPathExpression`] recurses once per level of the tree it copies, and the
//! evaluator copies whole subtrees from wherever it happens to be: an `EXISTS` preparing
//! its site copies the site's inner pattern, a correlated substitution copies a property
//! path leaf, a `SERVICE` copies its body before serializing it, a user-defined function
//! copies its body before rewriting it. The tree the parser admits is as tall as the
//! stack that parsed it holds, and one level of the derived copy costs a few hundred bytes
//! of native stack, so a copy made deep in an evaluation can need more stack than the
//! guard's margin leaves.
//!
//! These are the same copies, one level at a time, each level first asking
//! [`super::walk_is_low`]. They must run inside a [`super::walk`] scope — every caller's
//! is — which discards the placeholder a refusing level leaves. Outside a scope they never
//! refuse, and copy exactly what the derived `Clone` copies. The leaves they reach
//! (terms, triple patterns, `VALUES` cells) are copied with their own `Clone`, at a few
//! hundred bytes a triple-term level: the stack those copies take past what the margin
//! holds is reserved when the evaluation starts (see [`super::reserve_terms`]).
//!
//! Every `match` here is exhaustive and wildcard-free, so a new algebra variant is a
//! compile error here rather than a node these copies silently drop.

use purrdf_sparql_algebra::{
    AggregateExpression, Expression, GraphPattern, OrderExpression, PropertyPathExpression, Query,
};

use super::walk_is_low;

/// What a refusing level is replaced with. Never observed: the enclosing
/// [`super::walk`] scope discards the whole copy.
const fn placeholder_pattern() -> GraphPattern {
    GraphPattern::Bgp {
        patterns: Vec::new(),
    }
}

/// A copy of `query`, its pattern copied by [`pattern`].
pub(crate) fn query(query: &Query) -> Query {
    match query {
        Query::Select {
            pattern: body,
            dataset,
            base_iri,
            version,
        } => Query::Select {
            pattern: pattern(body),
            dataset: dataset.clone(),
            base_iri: base_iri.clone(),
            version: version.clone(),
        },
        Query::Construct {
            template,
            pattern: body,
            dataset,
            base_iri,
            version,
        } => Query::Construct {
            template: template.clone(),
            pattern: pattern(body),
            dataset: dataset.clone(),
            base_iri: base_iri.clone(),
            version: version.clone(),
        },
        Query::Describe {
            pattern: body,
            targets,
            dataset,
            base_iri,
            version,
        } => Query::Describe {
            pattern: pattern(body),
            targets: targets.clone(),
            dataset: dataset.clone(),
            base_iri: base_iri.clone(),
            version: version.clone(),
        },
        Query::Ask {
            pattern: body,
            dataset,
            base_iri,
            version,
        } => Query::Ask {
            pattern: pattern(body),
            dataset: dataset.clone(),
            base_iri: base_iri.clone(),
            version: version.clone(),
        },
    }
}

/// A copy of `node`, one level at a time.
pub(crate) fn pattern(node: &GraphPattern) -> GraphPattern {
    if walk_is_low("copy of a graph pattern") {
        return placeholder_pattern();
    }
    #[cfg(test)]
    crate::op_count::bump(crate::op_count::Op::Cloned);
    let boxed = |child: &GraphPattern| Box::new(pattern(child));
    match node {
        GraphPattern::Bgp { patterns } => GraphPattern::Bgp {
            patterns: patterns.clone(),
        },
        GraphPattern::Path {
            subject,
            path: steps,
            object,
        } => GraphPattern::Path {
            subject: subject.clone(),
            path: path(steps),
            object: object.clone(),
        },
        GraphPattern::Join { left, right } => GraphPattern::Join {
            left: boxed(left),
            right: boxed(right),
        },
        GraphPattern::LeftJoin {
            left,
            right,
            expression: condition,
        } => GraphPattern::LeftJoin {
            left: boxed(left),
            right: boxed(right),
            expression: condition.as_ref().map(expression),
        },
        GraphPattern::Lateral { left, right } => GraphPattern::Lateral {
            left: boxed(left),
            right: boxed(right),
        },
        GraphPattern::Filter { expr, inner } => GraphPattern::Filter {
            expr: expression(expr),
            inner: boxed(inner),
        },
        GraphPattern::Union { arms } => GraphPattern::Union {
            arms: arms.iter().map(pattern).collect(),
        },
        GraphPattern::Graph { name, inner } => GraphPattern::Graph {
            name: name.clone(),
            inner: boxed(inner),
        },
        GraphPattern::Extend {
            inner,
            variable,
            expression: value,
        } => GraphPattern::Extend {
            inner: boxed(inner),
            variable: variable.clone(),
            expression: expression(value),
        },
        GraphPattern::Minus { left, right } => GraphPattern::Minus {
            left: boxed(left),
            right: boxed(right),
        },
        GraphPattern::Service {
            name,
            inner,
            silent,
        } => GraphPattern::Service {
            name: name.clone(),
            inner: boxed(inner),
            silent: *silent,
        },
        GraphPattern::Values {
            variables,
            bindings,
        } => GraphPattern::Values {
            variables: variables.clone(),
            bindings: bindings.clone(),
        },
        GraphPattern::OrderBy {
            inner,
            expression: keys,
        } => GraphPattern::OrderBy {
            inner: boxed(inner),
            expression: keys.iter().map(order).collect(),
        },
        GraphPattern::Project { inner, variables } => GraphPattern::Project {
            inner: boxed(inner),
            variables: variables.clone(),
        },
        GraphPattern::Distinct { inner } => GraphPattern::Distinct {
            inner: boxed(inner),
        },
        GraphPattern::Reduced { inner } => GraphPattern::Reduced {
            inner: boxed(inner),
        },
        GraphPattern::Slice {
            inner,
            start,
            length,
        } => GraphPattern::Slice {
            inner: boxed(inner),
            start: *start,
            length: *length,
        },
        GraphPattern::Group {
            inner,
            variables,
            aggregates,
        } => GraphPattern::Group {
            inner: boxed(inner),
            variables: variables.clone(),
            aggregates: aggregates
                .iter()
                .map(|(variable, call)| (variable.clone(), aggregate(call)))
                .collect(),
        },
        GraphPattern::PropertyFunction(call) => GraphPattern::PropertyFunction(call.clone()),
        GraphPattern::Unfold {
            inner,
            expression: value,
            element,
            companion,
        } => GraphPattern::Unfold {
            inner: boxed(inner),
            expression: expression(value),
            element: element.clone(),
            companion: companion.clone(),
        },
    }
}

/// A copy of `node`, one level at a time.
pub(crate) fn expression(node: &Expression) -> Expression {
    if walk_is_low("copy of an expression") {
        return Expression::NamedNode(purrdf_sparql_algebra::NamedNode::new_unchecked(""));
    }
    let boxed = |child: &Expression| Box::new(expression(child));
    let list = |children: &[Expression]| children.iter().map(expression).collect();
    match node {
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Variable(_)
        | Expression::Bound(_) => node.clone(),
        Expression::Or(operands) => Expression::Or(list(operands)),
        Expression::And(operands) => Expression::And(list(operands)),
        Expression::Arithmetic(first, steps) => Expression::Arithmetic(
            boxed(first),
            steps
                .iter()
                .map(|(op, operand)| (*op, expression(operand)))
                .collect(),
        ),
        Expression::Equal(a, b) => Expression::Equal(boxed(a), boxed(b)),
        Expression::SameTerm(a, b) => Expression::SameTerm(boxed(a), boxed(b)),
        Expression::Greater(a, b) => Expression::Greater(boxed(a), boxed(b)),
        Expression::GreaterOrEqual(a, b) => Expression::GreaterOrEqual(boxed(a), boxed(b)),
        Expression::Less(a, b) => Expression::Less(boxed(a), boxed(b)),
        Expression::LessOrEqual(a, b) => Expression::LessOrEqual(boxed(a), boxed(b)),
        Expression::UnaryPlus(a) => Expression::UnaryPlus(boxed(a)),
        Expression::UnaryMinus(a) => Expression::UnaryMinus(boxed(a)),
        Expression::Not(a) => Expression::Not(boxed(a)),
        Expression::In(needle, haystack) => Expression::In(boxed(needle), list(haystack)),
        Expression::If(condition, then, otherwise) => {
            Expression::If(boxed(condition), boxed(then), boxed(otherwise))
        }
        Expression::Coalesce(items) => Expression::Coalesce(list(items)),
        Expression::FunctionCall(function, arguments) => {
            Expression::FunctionCall(function.clone(), list(arguments))
        }
        Expression::Exists(inner) => Expression::Exists(Box::new(pattern(inner))),
    }
}

/// A copy of `node`, one level at a time.
pub(crate) fn path(node: &PropertyPathExpression) -> PropertyPathExpression {
    if walk_is_low("copy of a property path") {
        return PropertyPathExpression::Wildcard { namespace: None };
    }
    let boxed = |child: &PropertyPathExpression| Box::new(path(child));
    match node {
        PropertyPathExpression::NamedNode(_)
        | PropertyPathExpression::NegatedPropertySet(_)
        | PropertyPathExpression::Wildcard { .. } => node.clone(),
        PropertyPathExpression::Reverse(inner) => PropertyPathExpression::Reverse(boxed(inner)),
        PropertyPathExpression::Sequence(elements) => {
            PropertyPathExpression::Sequence(elements.iter().map(path).collect())
        }
        PropertyPathExpression::Alternative(elements) => {
            PropertyPathExpression::Alternative(elements.iter().map(path).collect())
        }
        PropertyPathExpression::ZeroOrMore(inner) => {
            PropertyPathExpression::ZeroOrMore(boxed(inner))
        }
        PropertyPathExpression::OneOrMore(inner) => PropertyPathExpression::OneOrMore(boxed(inner)),
        PropertyPathExpression::ZeroOrOne(inner) => PropertyPathExpression::ZeroOrOne(boxed(inner)),
        PropertyPathExpression::Range { inner, min, max } => PropertyPathExpression::Range {
            inner: boxed(inner),
            min: *min,
            max: *max,
        },
    }
}

/// A copy of one sort key.
pub(crate) fn order(key: &OrderExpression) -> OrderExpression {
    match key {
        OrderExpression::Asc(value) => OrderExpression::Asc(expression(value)),
        OrderExpression::Desc(value) => OrderExpression::Desc(expression(value)),
    }
}

/// A copy of one aggregate call. Its fields are only readable, so it is rebuilt through
/// its one constructor with the same function, arguments, scalar values, sort keys and
/// `DISTINCT` flag — which that constructor accepted once already, so it accepts them
/// again.
pub(crate) fn aggregate(call: &AggregateExpression) -> AggregateExpression {
    AggregateExpression::new(
        call.function().clone(),
        call.args().iter().map(expression).collect(),
        call.scalarvals().to_vec(),
        call.order_by().iter().map(order).collect(),
        call.distinct,
    )
    .expect("a copy of an admitted aggregate call is admitted")
}

#[cfg(test)]
mod tests {
    use purrdf_sparql_algebra::SparqlParser;

    use super::*;

    /// Outside a scope the copies never refuse, and they copy exactly what the derived
    /// `Clone` copies — every algebra node kind a query can write, aggregates with sort
    /// keys and property paths of every shape included.
    #[test]
    fn copies_equal_the_derived_clone() {
        let text = "PREFIX ex: <http://example.org/>
            SELECT ?s (COUNT(DISTINCT ?o) AS ?n) (SAMPLE(?o) AS ?any) WHERE {
              { ?s ex:p ?o . ?s ex:q+/^ex:r|!(ex:a|^ex:b)?/ex:c* ?x }
              UNION { GRAPH ?g { ?s ex:p ?o } } UNION { ?s ex:t ?o }
              OPTIONAL { ?s ex:q ?w FILTER(?w > 1 && !BOUND(?z) && (?w + 2 * ?o - 1) / 3 < 9 || ?o) }
              MINUS { ?s ex:m ?o }
              BIND(IF(?o = 1, COALESCE(?w, 2), ?o IN (1, 2, 3)) AS ?b)
              FILTER NOT EXISTS { SELECT ?s WHERE { ?s ex:n ?v } ORDER BY DESC(?v) LIMIT 2 OFFSET 1 }
              VALUES ?v { 1 2 }
              LATERAL { SELECT DISTINCT ?s WHERE { ?s ex:p ?o } }
            } GROUP BY ?s ORDER BY ?n";
        let parsed = SparqlParser::new().parse_query(text).expect("parses");
        assert_eq!(query(&parsed), parsed);
    }

    /// Inside a scope, a copy that runs out of stack is the typed refusal.
    #[test]
    fn a_copy_that_runs_out_of_stack_is_refused() {
        // An operator chain is one node however long, so the depth comes from real
        // nesting: seventy bracket levels of seven operator levels each, 490 levels in
        // all, which a test thread's stack parses.
        let mut nested = String::from("?x");
        for _ in 0..70 {
            nested = format!("(?x || ?x && ?x != ?x + ?x * {nested})");
        }
        let chain = format!("SELECT * WHERE {{ BIND(1 AS ?x) FILTER({nested}) }}");
        let parsed = SparqlParser::new().parse_query(&chain).expect("parses");
        // Eat into the thread's stack until a little more than the margin is left:
        // the copy's 490 levels cannot fit in what remains above it. Measured, not
        // assumed from the requested size, which the C library may round up.
        fn descend(parsed: &Query) -> Result<Query, crate::EvalError> {
            if purrdf_stack::remaining() <= purrdf_stack::MARGIN_BYTES + 32 * 1024 {
                return super::super::walk(|| query(parsed));
            }
            let frame = core::hint::black_box([0u8; 4096]);
            let copy = descend(parsed);
            core::hint::black_box(&frame);
            copy
        }
        let refused = std::thread::Builder::new()
            .stack_size(4 * 1024 * 1024)
            .spawn(move || descend(&parsed))
            .expect("spawn")
            .join()
            .expect("join");
        assert!(
            matches!(refused, Err(crate::EvalError::StackExhausted { .. })),
            "{refused:?}"
        );
    }
}
