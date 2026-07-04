// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Engine-side variable **pre-binding** (purrdf S6,  GAP-A).
//!
//! Bridges the engine's egress term model ([`TermValue`]) to the algebra's
//! [`Query::substitute_variable`] rewrite. Each `(name, value)` of a
//! [`SparqlRequest::substitutions`](purrdf_core::SparqlRequest) pre-binds the
//! query variable `name` to `value` before evaluation, exactly mirroring oxigraph's
//! `PreparedSparqlQuery::substitute_variable` (the SHACL `$this` focus-node path).
//!
//! The substitution is applied to a **clone** of the cached (un-substituted) parse,
//! so the plan cache is never poisoned by a focus-node-specific binding.

use std::collections::HashMap;

use purrdf_core::{RdfDiagnostic, RdfTextDirection, TermValue};
use purrdf_sparql_algebra::{
    AggregateExpression, BaseDirection, BlankNode, Expression, GraphPattern, GroundTerm,
    GroundTriple, Literal, NamedNode, NamedNodePattern, OrderExpression, Query, Variable,
};

/// Apply every `(name, value)` substitution to `query` as a pre-binding rewrite,
/// returning the rewritten query. Each value is mapped to the algebra's
/// [`GroundTerm`] (blank-node focus nodes ride the injection-only
/// [`GroundTerm::BlankNode`]) and injected as a single-row `VALUES` join at the core
/// `WHERE` pattern, beneath the solution-modifier stack but visible to the projected
/// variable list.
///
/// # Errors
///
/// Returns a [`RdfDiagnostic`] if a literal substitution carries a datatype IRI that
/// is not a syntactically valid IRI (the only way a [`TermValue`] cannot become a
/// [`GroundTerm`]).
pub(crate) fn apply_substitutions(
    mut query: Query,
    substitutions: &[(String, TermValue)],
) -> Result<Query, RdfDiagnostic> {
    for (name, value) in substitutions {
        let var = Variable::new(name.clone());
        let ground = ground_term_from_value(value)?;
        query = query.substitute_variable(&var, ground);
    }
    Ok(query)
}

/// Apply SHACL-SPARQL pre-binding to `query`.
///
/// First performs the ordinary VALUES-join rewrite via [`apply_substitutions`]
/// (so triple-pattern positions and projectable variables work exactly like the
/// generic pre-binding path). Then walks the algebra and, for every pre-bound
/// variable whose value is an IRI or literal:
///
/// * replaces `Expression::Variable(v)` with the constant IRI/literal,
/// * replaces `Expression::Bound(v)` with the `true` boolean literal,
///
/// recursing into nested graph patterns (`EXISTS`, `GRAPH`, sub-queries, etc.).
/// Blank-node and quoted-triple values are deliberately left unsubstituted in
/// expression positions; the VALUES-join binds them.
///
/// Returns a diagnostic on the same error conditions as [`apply_substitutions`].
pub(crate) fn apply_shacl_prebinding(
    query: Query,
    substitutions: &[(String, TermValue)],
) -> Result<Query, RdfDiagnostic> {
    let query = apply_substitutions(query, substitutions)?;

    let mut expr_subs: HashMap<String, Option<Expression>> = HashMap::new();
    for (name, value) in substitutions {
        expr_subs.insert(name.clone(), expression_from_term_value(value)?);
    }

    Ok(map_patterns_in_query(query, |pattern| {
        substitute_in_graph_pattern(pattern, &expr_subs)
    }))
}

/// Convert a dataset-independent [`TermValue`] to an [`Expression`] when it is an
/// IRI or literal; return `None` for blank nodes or quoted triples, which must be
/// handled via the VALUES-join path.
fn expression_from_term_value(value: &TermValue) -> Result<Option<Expression>, RdfDiagnostic> {
    match ground_term_from_value(value)? {
        GroundTerm::NamedNode(node) => Ok(Some(Expression::NamedNode(node))),
        GroundTerm::Literal(lit) => Ok(Some(Expression::Literal(lit))),
        GroundTerm::BlankNode(_) | GroundTerm::Triple(_) => Ok(None),
    }
}

/// Walk and rebuild the whole [`Query`], applying `f` to every [`GraphPattern`]
/// contained in it (including sub-queries).
fn map_patterns_in_query(query: Query, mut f: impl FnMut(GraphPattern) -> GraphPattern) -> Query {
    match query {
        Query::Select {
            pattern,
            dataset,
            base_iri,
        } => Query::Select {
            pattern: f(pattern),
            dataset,
            base_iri,
        },
        Query::Construct {
            template,
            pattern,
            dataset,
            base_iri,
        } => Query::Construct {
            template,
            pattern: f(pattern),
            dataset,
            base_iri,
        },
        Query::Describe {
            pattern,
            targets,
            dataset,
            base_iri,
        } => Query::Describe {
            pattern: f(pattern),
            targets,
            dataset,
            base_iri,
        },
        Query::Ask {
            pattern,
            dataset,
            base_iri,
        } => Query::Ask {
            pattern: f(pattern),
            dataset,
            base_iri,
        },
    }
}

/// Recursively substitute pre-bound variables into a [`GraphPattern`].
fn substitute_in_graph_pattern(
    pattern: GraphPattern,
    expr_subs: &HashMap<String, Option<Expression>>,
) -> GraphPattern {
    match pattern {
        GraphPattern::Bgp { patterns } => GraphPattern::Bgp { patterns },
        GraphPattern::Path {
            subject,
            path,
            object,
        } => GraphPattern::Path {
            subject,
            path,
            object,
        },
        GraphPattern::Join { left, right } => GraphPattern::Join {
            left: Box::new(substitute_in_graph_pattern(*left, expr_subs)),
            right: Box::new(substitute_in_graph_pattern(*right, expr_subs)),
        },
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => GraphPattern::LeftJoin {
            left: Box::new(substitute_in_graph_pattern(*left, expr_subs)),
            right: Box::new(substitute_in_graph_pattern(*right, expr_subs)),
            expression: expression.map(|e| substitute_in_expression(e, expr_subs)),
        },
        GraphPattern::Lateral { left, right } => GraphPattern::Lateral {
            left: Box::new(substitute_in_graph_pattern(*left, expr_subs)),
            right: Box::new(substitute_in_graph_pattern(*right, expr_subs)),
        },
        GraphPattern::Filter { expr, inner } => GraphPattern::Filter {
            expr: substitute_in_expression(expr, expr_subs),
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
        },
        GraphPattern::Union { left, right } => GraphPattern::Union {
            left: Box::new(substitute_in_graph_pattern(*left, expr_subs)),
            right: Box::new(substitute_in_graph_pattern(*right, expr_subs)),
        },
        GraphPattern::Graph { name, inner } => GraphPattern::Graph {
            name: substitute_in_named_node_pattern(name, expr_subs),
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
        },
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => GraphPattern::Extend {
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
            variable,
            expression: substitute_in_expression(expression, expr_subs),
        },
        GraphPattern::Minus { left, right } => GraphPattern::Minus {
            left: Box::new(substitute_in_graph_pattern(*left, expr_subs)),
            right: Box::new(substitute_in_graph_pattern(*right, expr_subs)),
        },
        GraphPattern::Service {
            name,
            inner,
            silent,
        } => GraphPattern::Service {
            name: substitute_in_named_node_pattern(name, expr_subs),
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
            silent,
        },
        GraphPattern::Values {
            variables,
            bindings,
        } => GraphPattern::Values {
            variables,
            bindings,
        },
        GraphPattern::OrderBy { inner, expression } => GraphPattern::OrderBy {
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
            expression: expression
                .into_iter()
                .map(|e| substitute_in_order_expression(e, expr_subs))
                .collect(),
        },
        GraphPattern::Project { inner, variables } => GraphPattern::Project {
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
            variables,
        },
        GraphPattern::Distinct { inner } => GraphPattern::Distinct {
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
        },
        GraphPattern::Reduced { inner } => GraphPattern::Reduced {
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
        },
        GraphPattern::Slice {
            inner,
            start,
            length,
        } => GraphPattern::Slice {
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
            start,
            length,
        },
        GraphPattern::Group {
            inner,
            variables,
            aggregates,
        } => GraphPattern::Group {
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
            variables,
            aggregates: aggregates
                .into_iter()
                .map(|(var, agg)| (var, substitute_in_aggregate(agg, expr_subs)))
                .collect(),
        },
    }
}

/// Replace a pre-bound variable in a `GRAPH`/`SERVICE` name with its IRI constant.
fn substitute_in_named_node_pattern(
    pattern: NamedNodePattern,
    expr_subs: &HashMap<String, Option<Expression>>,
) -> NamedNodePattern {
    match pattern {
        NamedNodePattern::Variable(var) => {
            let name = var.as_str();
            if expr_subs.contains_key(name) {
                if let Some(Some(Expression::NamedNode(node))) = expr_subs.get(name) {
                    return NamedNodePattern::NamedNode(node.clone());
                }
            }
            NamedNodePattern::Variable(var)
        }
        named @ NamedNodePattern::NamedNode(_) => named,
    }
}

/// Recursively substitute pre-bound variables into an [`Expression`].
fn substitute_in_expression(
    expr: Expression,
    expr_subs: &HashMap<String, Option<Expression>>,
) -> Expression {
    match expr {
        Expression::Variable(var) => {
            let name = var.as_str();
            if let Some(Some(subst)) = expr_subs.get(name) {
                subst.clone()
            } else {
                Expression::Variable(var)
            }
        }
        Expression::Bound(var) => {
            let name = var.as_str();
            if expr_subs.contains_key(name) {
                true_literal()
            } else {
                Expression::Bound(var)
            }
        }
        Expression::NamedNode(node) => Expression::NamedNode(node),
        Expression::Literal(lit) => Expression::Literal(lit),
        Expression::Or(left, right) => Expression::Or(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::And(left, right) => Expression::And(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::Equal(left, right) => Expression::Equal(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::SameTerm(left, right) => Expression::SameTerm(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::Greater(left, right) => Expression::Greater(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::GreaterOrEqual(left, right) => Expression::GreaterOrEqual(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::Less(left, right) => Expression::Less(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::LessOrEqual(left, right) => Expression::LessOrEqual(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::Add(left, right) => Expression::Add(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::Subtract(left, right) => Expression::Subtract(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::Multiply(left, right) => Expression::Multiply(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::Divide(left, right) => Expression::Divide(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::UnaryPlus(inner) => {
            Expression::UnaryPlus(Box::new(substitute_in_expression(*inner, expr_subs)))
        }
        Expression::UnaryMinus(inner) => {
            Expression::UnaryMinus(Box::new(substitute_in_expression(*inner, expr_subs)))
        }
        Expression::Not(inner) => {
            Expression::Not(Box::new(substitute_in_expression(*inner, expr_subs)))
        }
        Expression::In(target, list) => Expression::In(
            Box::new(substitute_in_expression(*target, expr_subs)),
            list.into_iter()
                .map(|e| substitute_in_expression(e, expr_subs))
                .collect(),
        ),
        Expression::If(cond, then_expr, else_expr) => Expression::If(
            Box::new(substitute_in_expression(*cond, expr_subs)),
            Box::new(substitute_in_expression(*then_expr, expr_subs)),
            Box::new(substitute_in_expression(*else_expr, expr_subs)),
        ),
        Expression::Coalesce(list) => Expression::Coalesce(
            list.into_iter()
                .map(|e| substitute_in_expression(e, expr_subs))
                .collect(),
        ),
        Expression::FunctionCall(function, args) => Expression::FunctionCall(
            function,
            args.into_iter()
                .map(|e| substitute_in_expression(e, expr_subs))
                .collect(),
        ),
        Expression::Exists(inner) => {
            Expression::Exists(Box::new(substitute_in_graph_pattern(*inner, expr_subs)))
        }
    }
}

/// Substitute inside an [`OrderExpression`] sort key.
fn substitute_in_order_expression(
    order: OrderExpression,
    expr_subs: &HashMap<String, Option<Expression>>,
) -> OrderExpression {
    match order {
        OrderExpression::Asc(expr) => {
            OrderExpression::Asc(substitute_in_expression(expr, expr_subs))
        }
        OrderExpression::Desc(expr) => {
            OrderExpression::Desc(substitute_in_expression(expr, expr_subs))
        }
    }
}

/// Substitute inside a [`GROUP BY`][`AggregateExpression`] aggregate.
fn substitute_in_aggregate(
    agg: AggregateExpression,
    expr_subs: &HashMap<String, Option<Expression>>,
) -> AggregateExpression {
    match agg {
        AggregateExpression::CountStar { distinct } => AggregateExpression::CountStar { distinct },
        AggregateExpression::FunctionCall {
            function,
            expression,
            distinct,
        } => AggregateExpression::FunctionCall {
            function,
            expression: Box::new(substitute_in_expression(*expression, expr_subs)),
            distinct,
        },
    }
}

/// The SPARQL `true` boolean literal (`"true"^^xsd:boolean`).
fn true_literal() -> Expression {
    Expression::Literal(Literal::new_typed(
        "true",
        NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#boolean"),
    ))
}

/// Convert a dataset-independent [`TermValue`] to the algebra's [`GroundTerm`].
fn ground_term_from_value(value: &TermValue) -> Result<GroundTerm, RdfDiagnostic> {
    match value {
        TermValue::Iri(iri) => Ok(GroundTerm::NamedNode(node(iri)?)),
        TermValue::Blank { label, .. } => Ok(GroundTerm::BlankNode(BlankNode::new(label.clone()))),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => Ok(GroundTerm::Literal(literal_from_value(
            lexical_form,
            datatype,
            language.as_deref(),
            *direction,
        )?)),
        TermValue::Triple { s, p, o } => {
            let subject = ground_term_from_value(s)?;
            let GroundTerm::NamedNode(predicate) = ground_term_from_value(p)? else {
                return Err(RdfDiagnostic::error(
                    "native-sparql-subst-triple-predicate",
                    "a quoted-triple predicate must be an IRI".to_owned(),
                ));
            };
            let object = ground_term_from_value(o)?;
            Ok(GroundTerm::Triple(Box::new(GroundTriple {
                subject,
                predicate,
                object,
            })))
        }
    }
}

/// Build an algebra [`Literal`] from a value's components, choosing the plain /
/// typed / lang / dir-lang constructor that matches its shape.
fn literal_from_value(
    lexical_form: &str,
    datatype: &str,
    language: Option<&str>,
    direction: Option<RdfTextDirection>,
) -> Result<Literal, RdfDiagnostic> {
    match (language, direction) {
        (Some(lang), dir) => Ok(Literal::new_lang(
            lexical_form,
            lang,
            dir.map(|d| match d {
                RdfTextDirection::Ltr => BaseDirection::Ltr,
                RdfTextDirection::Rtl => BaseDirection::Rtl,
            }),
        )),
        (None, _) => Ok(Literal::new_typed(lexical_form, node(datatype)?)),
    }
}

/// Validate-and-wrap an IRI, surfacing a malformed IRI as a diagnostic.
fn node(iri: &str) -> Result<NamedNode, RdfDiagnostic> {
    NamedNode::new(iri).map_err(|e| RdfDiagnostic::error("native-sparql-subst-iri", e.to_string()))
}
